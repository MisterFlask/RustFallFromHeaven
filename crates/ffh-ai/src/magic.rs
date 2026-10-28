//! Spellcasting (item `magic`): what a spell does to the units it reaches, read from the spell
//! data, and what that is worth where the caster stands. FfH's own AI (`casting.rs`, ported term
//! by term) casts damage spells and summons only at the end of the turn when an enemy happens to
//! be in reach, and otherwise the buff with the highest weight: its casters spent their turns on
//! Haste and Regeneration (docs/AI_LOG.md, "Casters through a Mage Guild"). Here a spell is valued
//! by its effect on the units it reaches:
//!
//! - **Before an attack** ([`before_attack`], from `campaign::strike` and `military::assault`):
//!   every caster of ours within two plots of the target casts what helps the attack most, one
//!   cast at a time, the best of all casters first and every value recomputed after each cast:
//!   what weakens the defenders (damage, curses, stolen weapons and blessings), then summons,
//!   which attack the target at once, then blessings on the units about to attack. FfH's AI does
//!   the same in `CvSelectionGroup::groupAttack` ("BETTER AI (Groupattack Spellcasting)", left to
//!   the AI by the rules port).
//! - **At the end of the turn** ([`choose_turn_spell`], from `casting::end_turn`): damage and
//!   curses on the enemies in reach, summons that attack what stands next to us (as FfH's do,
//!   `casting::summons_attack`), lasting blessings on our units that lack them (and short ones by
//!   the defense they add through the enemies' turn), healing, the buildings city spells create;
//!   when nothing is worth casting, FfH's own choice (`casting::end_turn`).
//! - **Movement** ([`caster_target`], from `units::caster`): a caster that knows a spell for a
//!   fight goes with the stack of our operation nearest its target; the others go where their
//!   lasting blessings, city spells or terraforming are worth most, and one that keeps a city
//!   spell's building (`bRequiresCaster`) stays.
//! - **Promotions** ([`promotion_value`], from `units::promotion_value`): a sphere is worth the
//!   spells it opens, and half of those the next promotion of the sphere opens (Fire I for
//!   Fire II's Fireball).
//!
//! Casters are trained by role (`casters.rs`, the caster corps) and to build mana nodes
//! (`mana.rs`); see "Casters to train" below.
//!
//! Values are hit points: those a spell takes from enemy units (a change of strength counts as
//! that share of the unit's health), less those it costs ours. FfH's own terms stay for what is
//! not modelled here ([`understood`]: world spells, abilities such as equipment, and spells that
//! live in Python). `FFH_AI_MAGIC_OFF` turns all of this off for some civilizations
//! ([`enabled`]), the measurement switch.

use std::sync::OnceLock;

use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::utils::{combat_odds, CombatOddsInput};
use ffh_data::*;

use crate::view::{attack_power, View};

// ----------------------------------------------------------------------------
// Switch and trace
// ----------------------------------------------------------------------------

fn off_table(d: &GameData) -> &'static [bool] {
    static TABLE: OnceLock<Vec<bool>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let off: Vec<String> = std::env::var("FFH_AI_MAGIC_OFF")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().trim_start_matches("CIVILIZATION_").to_ascii_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
        d.civilizations
            .keys()
            .map(|c| {
                let name = d.civilizations[c].base.type_.trim_start_matches("CIVILIZATION_");
                off.iter().any(|o| o == "ALL" || o == name)
            })
            .collect()
    })
}

/// False when the player casts as FfH's AI did (`casting.rs` as ported, nothing before attacks,
/// casters moved and promoted as before): the barbarians, and the civilizations listed in
/// `FFH_AI_MAGIC_OFF` (`CIVILIZATION_AMURITES` or `AMURITES`, comma-separated, or `all`). A
/// measurement switch like `FFH_AI_CIV_OFF`: read once, so play stays deterministic, and the run
/// cache keys runs by every `FFH_*` variable.
pub fn enabled(g: &Game, p: PlayerId) -> bool {
    !g.player_is_barbarian(p) && g.player_get_civilization_type(p).is_some_and(|c| !off_table(g.d)[c.idx()])
}

/// Does player `p` raise its arcane casters as the rules intend ([`apprentice`], [`safe_step`]):
/// its spellcasting is this AI's ([`enabled`]) and the switch `CASTER_SCHOOL` is on for it.
pub fn school(g: &Game, p: PlayerId) -> bool {
    enabled(g, p) && crate::switches::CASTER_SCHOOL.on(g, p)
}

/// Every AI cast goes through here: `FFH_AI_TRACE_CAST=1` prints one line per cast (turn,
/// player, civilization, unit, spell, where, value) to stderr, and telemetry records `ctx` and
/// `value` as the cast's reason. Neither changes play.
pub fn cast(g: &mut Game, u: UnitId, spell: SpellType, ctx: &str, value: i32) {
    static ON: OnceLock<bool> = OnceLock::new();
    if *ON.get_or_init(|| std::env::var("FFH_AI_TRACE_CAST").is_ok()) {
        let p = g.units[u].owner;
        let civ = g.player_get_civilization_type(p).map_or("NONE", |c| g.d.civilizations[c].base.type_.as_str());
        eprintln!(
            "[cast t{} p{}] civ={} unit={} spell={} ctx={} value={}",
            g.game_turn(),
            p.0,
            civ.trim_start_matches("CIVILIZATION_"),
            g.d.units[g.units[u].unit_type].base.type_.trim_start_matches("UNIT_"),
            g.d.spells[spell].base.type_.trim_start_matches("SPELL_"),
            ctx,
            value
        );
    }
    g.tel_cast_reason(u, ctx, value as i64);
    g.unit_cast(u, spell);
}

/// `FFH_AI_TRACE_CAST` is set (read once).
fn trace_cast_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("FFH_AI_TRACE_CAST").is_ok())
}

/// With `FFH_AI_TRACE_CAST=1`, a caster joining (`join`) or leaving (`leave`) a stack. It only
/// prints.
fn trace_stack(g: &Game, u: UnitId, what: &str) {
    if !trace_cast_on() {
        return;
    }
    let p = g.units[u].owner;
    let civ = g.player_get_civilization_type(p).map_or("NONE", |c| g.d.civilizations[c].base.type_.as_str());
    eprintln!("[{} t{} p{}] civ={} unit={} at={:?}", what, g.game_turn(), p.0, civ.trim_start_matches("CIVILIZATION_"), g.d.units[g.units[u].unit_type].base.type_.trim_start_matches("UNIT_"), g.map.xy(g.unit_plot(u)));
}

/// With `FFH_AI_TRACE_CAST=1`, one line per strike of player `p` on city plot `tp` (before its
/// casts): our arcane casters within two plots of it (where they cast before the strike) and
/// those with Channeling II anywhere. It only prints.
pub fn trace_strike(g: &Game, p: PlayerId, tp: PlotIdx) {
    if !trace_cast_on() {
        return;
    }
    let channeling2 = g.d.ty::<PromotionType>("PROMOTION_CHANNELING2");
    let (tx, ty) = g.map.xy(tp);
    let ours: Vec<UnitId> = g.players[p].units.iter().copied().filter(|&u| g.units.contains(u) && arcane(g, u)).collect();
    let near = ours.iter().filter(|&&u| {
        let (x, y) = g.map.xy(g.unit_plot(u));
        g.plot_distance(x, y, tx, ty) <= 2
    });
    let civ = g.player_get_civilization_type(p).map_or("NONE", |c| g.d.civilizations[c].base.type_.as_str());
    eprintln!(
        "[strike t{} p{}] civ={} near={} near_mages={} mages={}",
        g.game_turn(),
        p.0,
        civ.trim_start_matches("CIVILIZATION_"),
        near.clone().count(),
        near.filter(|&&u| channeling2.is_some_and(|c| g.units[u].has_promotion[c])).count(),
        ours.iter().filter(|&&u| channeling2.is_some_and(|c| g.units[u].has_promotion[c])).count()
    );
}

// ----------------------------------------------------------------------------
// What a spell does
// ----------------------------------------------------------------------------

/// Spells whose effect is all in Python, valued here by what that Python does.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Py {
    None,
    /// `spellCrush` / `spellPillarofFire`: 50 damage (limit 75) on every unit of the plot within
    /// two that holds most enemies and no one else.
    Crush,
    PillarOfFire,
    /// `spellDestroyUndead`: 30 holy damage (no limit) on every undead unit within one.
    DestroyUndead,
    /// `spellDomination`: the best enemy unit within one changes sides unless it resists (the AI
    /// may only try units resisting 20% or less).
    Domination,
    /// `spellHeal(caster, 20)`: 20 hit points to every living unit on the plot.
    Heal,
    /// `spellEarthquake`: a quarter of the buildings of every city within one, fortification lost.
    Earthquake,
    /// `spellSpring`, `spellScorch`, `spellVitalize`: the caster's plot gets a better terrain
    /// ([`terraform`]).
    Spring,
    Scorch,
    Vitalize,
    /// `spellRecruit` (Donal, once): units in the caster's city ([`recruit_value`]).
    Recruit,
}

fn py_kind(d: &GameData, s: SpellType) -> Py {
    static TABLE: OnceLock<Vec<Py>> = OnceLock::new();
    TABLE.get_or_init(|| {
        d.spells
            .keys()
            .map(|s| match d.spells[s].base.type_.as_str() {
                "SPELL_CRUSH" => Py::Crush,
                "SPELL_PILLAR_OF_FIRE" => Py::PillarOfFire,
                "SPELL_DESTROY_UNDEAD" => Py::DestroyUndead,
                "SPELL_DOMINATION" => Py::Domination,
                "SPELL_HEAL" => Py::Heal,
                "SPELL_EARTHQUAKE" => Py::Earthquake,
                "SPELL_SPRING" => Py::Spring,
                "SPELL_SCORCH" => Py::Scorch,
                "SPELL_VITALIZE" => Py::Vitalize,
                "SPELL_RECRUIT" => Py::Recruit,
                _ => Py::None,
            })
            .collect()
    })[s.idx()]
}

/// The spells this module values; the others keep FfH's terms (`casting::ffh_value`): world
/// spells (their timing is `world_spells.rs`), abilities (equipment, crews, cages, upgrades...),
/// and spells whose effect lives only in Python and is not one of [`Py`].
/// A Python spell that damages enemy units ([`Py`]: Crush, Pillar of Fire, Destroy Undead).
pub fn damage_spell(d: &GameData, s: SpellType) -> bool {
    matches!(py_kind(d, s), Py::Crush | Py::PillarOfFire | Py::DestroyUndead)
}

pub fn understood(d: &GameData, s: SpellType) -> bool {
    let i = &d.spells[s];
    if i.global || i.ability {
        return false;
    }
    i.damage != 0
        || i.create_unit_type.is_some()
        || i.add_promotion_type1.is_some()
        || i.remove_promotion_type1.is_some()
        || i.immobile_turns != 0
        || i.dispel
        || i.create_building_type.is_some()
        || py_kind(d, s) != Py::None
}

/// Units of type `t` are summoned for good by some spell (`bPermanentUnitCreate`: Raise
/// Skeleton's skeletons, Battering Rams, Flesh Golems, Tigers, Treants, the Kraken), not trained:
/// a strike spends them right after its collateral attackers (`campaign::strike`).
pub fn raised(d: &GameData, t: UnitType) -> bool {
    static TABLE: OnceLock<Vec<bool>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = vec![false; d.units.len()];
        for s in d.spells.iter() {
            if let (true, Some(ut)) = (s.permanent_unit_create, s.create_unit_type) {
                table[ut.idx()] = true;
            }
        }
        table
    })[t.idx()]
}

fn adds(info: &SpellInfo) -> impl Iterator<Item = PromotionType> {
    [info.add_promotion_type1, info.add_promotion_type2, info.add_promotion_type3].into_iter().flatten()
}

fn removes(info: &SpellInfo) -> impl Iterator<Item = PromotionType> {
    [info.remove_promotion_type1, info.remove_promotion_type2, info.remove_promotion_type3].into_iter().flatten()
}

fn square(g: &Game, x: i32, y: i32, r: i32) -> Vec<PlotIdx> {
    let mut out = Vec::new();
    for i in -r..=r {
        for j in -r..=r {
            if let Some(q) = g.plot_xy(x, y, i, j) {
                out.push(q);
            }
        }
    }
    out
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Side {
    Ours,
    Enemy,
    Neutral,
}

fn side(g: &Game, t: UnitId, team: TeamId) -> Side {
    if g.unit_get_team(t) == team {
        Side::Ours
    } else if g.unit_is_enemy(t, Some(team), None) {
        Side::Enemy
    } else {
        Side::Neutral
    }
}

/// Mean damage a hit of `dmg` (limit `limit`, type `dt`) by `caster` deals `t`. C++ `castDamage`
/// rolls `dmg / 2 + rand(dmg)` and `doDamage` then `rand(n) + rand(n)` with the caster's spell
/// damage bonus in `n` (both about `n - 1` on average), less the target's resistance -- twice
/// its defense strength, twice its level, a quarter of its city's defense, its resistance to
/// the damage type -- and never past the limit.
fn expected_damage(g: &Game, caster: UnitId, t: UnitId, dmg: i32, limit: i32, dt: Option<DamageType>) -> i32 {
    expected_damage_at(g, caster, t, dmg, limit, dt, g.units[t].damage)
}

/// [`expected_damage`] on `t` hurt by `damage` hit points.
fn expected_damage_at(g: &Game, caster: UnitId, t: UnitId, dmg: i32, limit: i32, dt: Option<DamageType>, damage: i32) -> i32 {
    let unit = &g.units[t];
    let mut resist = g.unit_base_combat_str_defense(t) * 2 + unit.level * 2;
    if let Some(c) = g.map.plots[g.unit_plot(t)].plot_city {
        resist += g.city_defense_modifier(c, false) / 4;
    }
    if let Some(dt) = dt {
        resist += g.unit_get_damage_type_resist(t, dt);
    }
    if resist >= 100 {
        return 0;
    }
    let mut n = dmg;
    if dt != Some(DamageType::DAMAGE_PHYSICAL) {
        n += g.units[caster].spell_damage_modify;
    }
    ((n - 1).max(0) * (100 - resist) / 100).min(limit - damage).max(0)
}

/// Strength change, in percent, that promotion `pr` makes to `t` attacking (`attack`) or
/// defending; `city`: the fight is for a city (its attack or defense bonus counts, and a unit
/// ignoring building defense gains those walls). Extra and damage-type strength count against
/// the unit's base strength, a first strike about 10%; a unit that may only defend loses its
/// whole attack.
fn promo_pct(g: &Game, t: UnitId, pr: PromotionType, attack: bool, city: Option<CityId>) -> i32 {
    let info = &g.d.promotions[pr];
    if attack && info.only_defensive {
        return -100;
    }
    let base = if attack { g.unit_base_combat_str(t) } else { g.unit_base_combat_str_defense(t) }.max(1);
    let mut pct = if attack { info.combat_percent } else { info.combat_percent_defense };
    let extra = if attack { info.extra_combat_str } else { info.extra_combat_defense };
    pct += (extra + info.damage_type_combat.iter().sum::<i32>()) * 100 / base;
    if let Some(c) = city {
        pct += if attack { info.city_attack_percent } else { info.city_defense_percent };
        if attack && info.ignore_building_defense {
            pct += g.city_defense_modifier(c, false) - g.city_defense_modifier(c, true);
        }
    }
    pct + info.first_strikes_change * 10 + info.chance_first_strikes_change * 5
}

/// Does the spell's promotion `pr` go to `t` (C++ `castAddPromotion`: units of a unit combat it
/// lists, that lack it)?
fn takes(g: &Game, t: UnitId, pr: PromotionType) -> bool {
    !g.units[t].has_promotion[pr] && g.unit_get_unit_combat_type(t).is_some_and(|uc| g.d.promotions[pr].unit_combat[uc])
}

fn hp(g: &Game, t: UnitId) -> i32 {
    g.unit_curr_hit_points(t).max(0)
}

/// Where a spell is cast, for its value.
#[derive(Copy, Clone)]
enum Ctx<'a> {
    /// Our `attackers` attack plot `tp` this turn.
    Attack { tp: PlotIdx, attackers: &'a [UnitId] },
    /// The end of our turn.
    Turn { v: &'a View },
}

/// Hit points an attacker's strength change of `pct` percent is worth in the strike: its share
/// of a typical fight (about half the defender's health changes hands).
const FIGHT_HP: i32 = 50;

/// What gaining `pr` is worth to our unit `t` at the end of the turn: a lasting promotion its
/// strength gain (defense for a garrison, the better of attack and defense for a field unit, a
/// half for one that combat removes, plus a tenth of its AI weight for what strength does not
/// show), one that expires soon its defense through the enemies' turn (Dance of Blades' first
/// strike; Haste's move is worth nothing then), healing on a hurt unit.
fn ours_worth(g: &Game, v: &View, t: UnitId, pr: PromotionType) -> i32 {
    let info = &g.d.promotions[pr];
    let here = g.unit_plot(t);
    let city = g.map.plots[here].plot_city;
    let garrison = matches!(g.units[t].ai_type, Some(UnitAIType::UNITAI_CITY_DEFENSE) | Some(UnitAIType::UNITAI_CITY_COUNTER));
    let def = promo_pct(g, t, pr, false, city);
    let damage = g.units[t].damage;
    if info.removed_when_healed && damage == 0 {
        return 0;
    }
    let heal = info.friendly_heal_change.max(info.neutral_heal_change).max(info.enemy_heal_change).max(info.same_tile_heal_change);
    let mut w = if heal > 0 { heal.min(damage) } else { 0 };
    if info.expire_chance > 0 {
        // Gone at the start of our next turn with this chance: what counts is the enemies' turn,
        // in full where an enemy we see can strike, a quarter elsewhere (those we do not see).
        let share = if v.danger_at(here) > 0 { 200 } else { 800 };
        w += def * hp(g, t) / share;
        return w;
    }
    let pct = if garrison { def } else { promo_pct(g, t, pr, true, None).max(def) };
    let mut lasting = pct + info.ai_weight / 10;
    if info.removed_by_combat {
        lasting /= 2;
    }
    w + lasting
}

/// The effects of `spell` cast by `caster` on the units it reaches -- damage, promotions given
/// and taken, immobility, dispel -- in hit points for us. `None`: it would hurt a third party (a
/// spell's damage on a neutral's land declares war, C++ `doDamage`).
fn reached_value(g: &Game, caster: UnitId, spell: SpellType, ctx: Ctx) -> Option<i32> {
    let d = g.d;
    let info = &d.spells[spell];
    let team = g.unit_get_team(caster);
    let here = g.unit_plot(caster);
    let (x, y) = g.map.xy(here);
    let mut value = 0;
    let city_at = |q: PlotIdx| g.map.plots[q].plot_city;
    if info.buff_caster_only {
        // The caster alone gains these (C++ `castAddPromotion`), whatever its unit combat.
        for pr in adds(info).filter(|&pr| !g.units[caster].has_promotion[pr]) {
            value += match ctx {
                Ctx::Turn { v } => ours_worth(g, v, caster, pr),
                Ctx::Attack { tp, attackers } if attackers.contains(&caster) => promo_pct(g, caster, pr, true, city_at(tp)) * FIGHT_HP / 100,
                Ctx::Attack { .. } => 0,
            };
        }
    }
    for q in square(g, x, y, info.range) {
        let own_plot = q == here;
        for &t in &g.map.plots[q].units {
            if g.units[t].death_delay || g.unit_is_immune_to_spell(t, caster, spell) {
                continue;
            }
            let s = side(g, t, team);
            // Enemy health counts in full on the target, else at a share: in a strike the other
            // enemies in reach matter less; at the end of the turn, those that can attack us more.
            let weight = match (ctx, s) {
                (Ctx::Attack { tp, .. }, Side::Enemy) => {
                    if q == tp {
                        100
                    } else {
                        40
                    }
                }
                (Ctx::Turn { .. }, Side::Enemy) => {
                    if attack_power(g, t) > 0 {
                        60
                    } else {
                        20
                    }
                }
                _ => 100,
            };
            // The chance the spell lands on an enemy (C++ `isResisted`: rand(100) <= chance).
            let land = if info.resistable && s == Side::Enemy { 100 - g.unit_get_resist_chance(t, caster, spell) } else { 100 };
            let mut tv = 0;
            if info.damage != 0 && !own_plot {
                let dmg = expected_damage(g, caster, t, info.damage, info.damage_limit, info.damage_type);
                match s {
                    Side::Ours => tv -= 2 * dmg,
                    Side::Enemy => tv += dmg * weight / 100,
                    Side::Neutral if dmg > 0 => return None,
                    Side::Neutral => {}
                }
            }
            for pr in adds(info) {
                if info.buff_caster_only || !takes(g, t, pr) {
                    continue;
                }
                let pinfo = &d.promotions[pr];
                match (s, ctx) {
                    (Side::Enemy, Ctx::Attack { tp, .. }) => {
                        // The defenders of the target fight with it.
                        if q == tp {
                            tv -= promo_pct(g, t, pr, false, city_at(q)) * hp(g, t) / 100;
                        }
                    }
                    (Side::Enemy, Ctx::Turn { .. }) => {
                        // An enemy in reach attacks with it (or, made to defend only, not at all).
                        if attack_power(g, t) > 0 {
                            tv -= promo_pct(g, t, pr, true, None) * hp(g, t) * weight / 10000;
                            let moves = g.d.units[g.units[t].unit_type].moves;
                            if pinfo.moves_change < 0 && moves + pinfo.moves_change <= 0 {
                                // Slowed to a standstill until it wears off.
                                tv += 30;
                            }
                        }
                    }
                    (Side::Ours, Ctx::Attack { attackers, tp }) => {
                        if attackers.contains(&t) {
                            tv += promo_pct(g, t, pr, true, city_at(tp)) * FIGHT_HP / 100;
                        }
                    }
                    (Side::Ours, Ctx::Turn { v }) => tv += ours_worth(g, v, t, pr),
                    _ => {}
                }
            }
            for pr in removes(info) {
                if info.buff_caster_only || !g.units[t].has_promotion[pr] {
                    continue;
                }
                match (s, ctx) {
                    (Side::Enemy, Ctx::Attack { tp, .. }) => {
                        if q == tp {
                            tv += promo_pct(g, t, pr, false, city_at(q)) * hp(g, t) / 100;
                        }
                    }
                    (Side::Enemy, Ctx::Turn { .. }) => {
                        if attack_power(g, t) > 0 {
                            tv += promo_pct(g, t, pr, true, None) * hp(g, t) * weight / 10000;
                        }
                    }
                    (Side::Ours, _) => {
                        // A curse lifted (Cure Disease, Haste's end of Slow), or a blessing lost.
                        let pct = promo_pct(g, t, pr, true, None).max(promo_pct(g, t, pr, false, None));
                        tv -= pct * hp(g, t) / 100;
                        if d.promotions[pr].moves_change < 0 {
                            tv += 10;
                        }
                    }
                    _ => {}
                }
            }
            if info.immobile_turns > 0 && !own_plot && g.units[t].immobile_timer == 0 {
                match (s, ctx) {
                    (Side::Enemy, Ctx::Turn { .. }) if attack_power(g, t) > 0 => tv += 25 * info.immobile_turns.min(2),
                    (Side::Enemy, Ctx::Attack { tp, .. }) if q != tp && attack_power(g, t) > 0 => tv += 10,
                    (Side::Ours, _) => tv -= 30,
                    _ => {}
                }
            }
            if info.dispel {
                for pr in d.promotions.keys() {
                    let pinfo = &d.promotions[pr];
                    if !g.units[t].has_promotion[pr] || !pinfo.dispellable {
                        continue;
                    }
                    match s {
                        Side::Enemy if pinfo.ai_weight > 0 => {
                            let attack = matches!(ctx, Ctx::Turn { .. });
                            tv += promo_pct(g, t, pr, attack, None).max(0) * hp(g, t) * weight / 10000 + 5;
                        }
                        Side::Ours if pinfo.ai_weight < 0 => tv -= promo_pct(g, t, pr, true, None).min(0) * hp(g, t) / 100,
                        _ => {}
                    }
                }
            }
            value += tv * land / 100;
        }
    }
    Some(value)
}

/// The value of a Python-only spell ([`Py`]), as [`reached_value`].
fn py_value(g: &Game, caster: UnitId, spell: SpellType, ctx: Ctx) -> Option<i32> {
    let d = g.d;
    let team = g.unit_get_team(caster);
    let here = g.unit_plot(caster);
    let (x, y) = g.map.xy(here);
    let target = match ctx {
        Ctx::Attack { tp, .. } => Some(tp),
        Ctx::Turn { .. } => None,
    };
    match py_kind(d, spell) {
        Py::None => Some(0),
        Py::Crush | Py::PillarOfFire => {
            // The plot within two with most enemies and no one else takes 50 (limit 75).
            let dt = if py_kind(d, spell) == Py::Crush { Some(DamageType::DAMAGE_PHYSICAL) } else { d.ty::<DamageType>("DAMAGE_FIRE") };
            let mut best = 0;
            for q in square(g, x, y, 2) {
                let units = &g.map.plots[q].units;
                if units.is_empty() || units.iter().any(|&t| side(g, t, team) != Side::Enemy) {
                    continue;
                }
                let w = if Some(q) == target || target.is_none() { 100 } else { 40 };
                let v: i32 = units.iter().map(|&t| expected_damage(g, caster, t, 50, 75, dt) * w / 100).sum();
                best = best.max(v);
            }
            Some(best)
        }
        Py::DestroyUndead => {
            let undead = d.t::<PromotionType>("PROMOTION_UNDEAD");
            let holy = d.ty::<DamageType>("DAMAGE_HOLY");
            let mut v = 0;
            for q in square(g, x, y, 1) {
                for &t in &g.map.plots[q].units {
                    if g.units[t].race != Some(undead) {
                        continue;
                    }
                    let dmg = expected_damage(g, caster, t, 30, 100, holy);
                    match side(g, t, team) {
                        Side::Ours => v -= 2 * dmg,
                        Side::Enemy => v += dmg,
                        Side::Neutral if dmg > 0 => return None,
                        Side::Neutral => {}
                    }
                }
            }
            Some(v)
        }
        Py::Domination => {
            // The unit it would take (C++ `spellDomination`'s choice), if it fails to resist.
            let mut best: Option<(i32, UnitId)> = None;
            for q in square(g, x, y, 1) {
                for &t in &g.map.plots[q].units {
                    if !g.unit_is_alive(t) || g.units[t].death_delay || side(g, t, team) != Side::Enemy {
                        continue;
                    }
                    let resist = g.unit_get_resist_chance(t, caster, spell);
                    if resist > 20 {
                        continue;
                    }
                    let value = g.unit_base_combat_str(t) * 10 + (100 - resist);
                    if best.map_or(true, |(b, _)| value > b) {
                        best = Some((value, t));
                    }
                }
            }
            Some(best.map_or(0, |(_, t)| {
                let resist = g.unit_get_resist_chance(t, caster, spell);
                // Theirs lost and ours gained (the caster loses Mind III when it is resisted).
                (hp(g, t) + attack_power(g, t) / 10) * (100 - resist) / 100
            }))
        }
        Py::Heal => {
            let v: i32 = g.map.plots[here]
                .units
                .iter()
                .filter(|&&t| side(g, t, team) == Side::Ours && g.unit_is_alive(t))
                .map(|&t| g.units[t].damage.min(20))
                .sum();
            Some(match ctx {
                Ctx::Attack { attackers, .. } => {
                    g.map.plots[here].units.iter().filter(|&&t| attackers.contains(&t) && g.unit_is_alive(t)).map(|&t| g.units[t].damage.min(20)).sum::<i32>()
                        / 2
                }
                Ctx::Turn { .. } => v / 2,
            })
        }
        Py::Spring | Py::Scorch | Py::Vitalize => Some(match target {
            Some(_) => 0,
            None => terraform_value(g, g.units[caster].owner, spell, here),
        }),
        Py::Recruit => Some(match target {
            Some(_) => 0,
            None => recruit_value(g, caster),
        }),
        Py::Earthquake => match target {
            // Walls and fortification of the city we strike.
            Some(tp) if g.map.plots[tp].plot_city.is_some() => {
                let n = g.map.plots[tp].units.iter().filter(|&&t| side(g, t, team) == Side::Enemy).count() as i32;
                Some(15 * n)
            }
            _ => Some(0),
        },
    }
}

/// `spellRecruit`, Donal's once: population / 3 + 1 units of classes our techs open (scouts to
/// champions) in the caster's city, twice that with a Temple of the Order. A unit's hit points
/// for each, in the first city of ours that gives two or more (or as many as our best): Donal
/// marches with the army, and waiting for a city half as good as our best, 5 of 9 Donals left
/// their city and never recruited (by FfH's own terms it was worth nothing: none of 20 Donals
/// recruited, docs/AI.md "Unique units").
pub(crate) fn recruit_value(g: &Game, caster: UnitId) -> i32 {
    let temple = g.d.ty::<BuildingType>("BUILDING_TEMPLE_OF_THE_ORDER");
    let recruits = |c: CityId| {
        let n = g.cities[c].population / 3 + 1;
        if temple.is_some_and(|b| g.city_num_building(c, b) == 1) {
            n * 2
        } else {
            n
        }
    };
    let p = g.units[caster].owner;
    let Some(c) = g.map.plots[g.unit_plot(caster)].plot_city.filter(|&c| g.cities[c].owner == p) else { return 0 };
    let best = g.players[p].cities.iter().map(|&o| recruits(o)).max().unwrap_or(0);
    let here = recruits(c);
    if here >= 2 || here >= best {
        here * 100
    } else {
        0
    }
}

/// The terrain a terraforming spell makes of our plot `q` when a unit of `p` casts it there (the
/// Python's change, under its AI requirement: our own land): Spring a desert (not flood plains)
/// into plains, Scorch snow into tundra (not for the Doviello and the Illians, who like snow),
/// Vitalize snow into tundra, tundra and desert into plains, plains into grassland.
fn terraform(g: &Game, p: PlayerId, s: SpellType, q: PlotIdx) -> Option<TerrainType> {
    let d = g.d;
    let plot = &g.map.plots[q];
    if plot.owner != Some(p) || g.plot_is_water(q) {
        return None;
    }
    let t = |name: &str| d.ty::<TerrainType>(name);
    let terrain = plot.terrain_type;
    match py_kind(d, s) {
        Py::Spring if terrain == t("TERRAIN_DESERT") && plot.feature_type != d.ty::<FeatureType>("FEATURE_FLOOD_PLAINS") => t("TERRAIN_PLAINS"),
        Py::Scorch if terrain == t("TERRAIN_SNOW") => {
            let civ = g.player_get_civilization_type(p).map(|c| d.civilizations[c].base.type_.as_str());
            if matches!(civ, Some("CIVILIZATION_DOVIELLO") | Some("CIVILIZATION_ILLIANS")) {
                None
            } else {
                t("TERRAIN_TUNDRA")
            }
        }
        Py::Vitalize => {
            if terrain == t("TERRAIN_SNOW") {
                t("TERRAIN_TUNDRA")
            } else if terrain == t("TERRAIN_TUNDRA") || terrain == t("TERRAIN_DESERT") {
                t("TERRAIN_PLAINS")
            } else if terrain == t("TERRAIN_PLAINS") {
                t("TERRAIN_GRASS")
            } else {
                None
            }
        }
        _ => None,
    }
}

/// What terraforming our plot `q` with `s` is worth: the terrain's yield gain, a turn's worth of
/// it on the scale of `production::building_value` (a quarter of it, as for a city spell), for a
/// plot one of our cities can work (in full if it is worked).
fn terraform_value(g: &Game, p: PlayerId, s: SpellType, q: PlotIdx) -> i32 {
    let Some(to) = terraform(g, p, s, q) else { return 0 };
    let plot = &g.map.plots[q];
    let Some(from) = plot.terrain_type else { return 0 };
    if plot.city_radius_count == 0 {
        return 0;
    }
    let d = g.d;
    let weights = [200, 150, 70];
    let gain: i32 = (0..YieldType::NUM).map(|i| (d.terrains[to].yield_[i] - d.terrains[from].yield_[i]) * weights[i]).sum();
    let gain = gain / 4;
    if g.plot_is_being_worked(q) {
        gain
    } else {
        gain * 3 / 5
    }
}

// ----------------------------------------------------------------------------
// Summons
// ----------------------------------------------------------------------------

/// Hit points a fresh summon of type `ut` of player `p` takes from the defenders of `tp` when it
/// attacks at once (summons cast by the AI attack, C++ `AI_summonAttackMove`): the fight with
/// the defender the rules put up -- its health if the summon wins, else the rounds the summon wins
/// before it falls -- and its collateral damage on the others (C++ `collateralCombat`).
fn summon_strike(g: &Game, p: PlayerId, ut: UnitType, tp: PlotIdx) -> i32 {
    summon_fight(g, p, ut, tp).map_or(0, |(_, v)| v)
}

/// [`summon_strike`] with the summon's odds of winning (of 1000) by the same estimate; `None`
/// when it cannot fight there.
pub fn summon_fight(g: &Game, p: PlayerId, ut: UnitType, tp: PlotIdx) -> Option<(i32, i32)> {
    summon_fight_at(g, p, ut, tp, &|e| hp(g, e)).map(|(odds, taken)| (odds, taken.iter().map(|&(_, v)| v).sum()))
}

/// [`summon_fight`] against defenders whose health is `health` (hit points), with what the fight
/// takes from each: the defender the rules put up first, then those its collateral damage hurts.
fn summon_fight_at(g: &Game, p: PlayerId, ut: UnitType, tp: PlotIdx, health: &dyn Fn(UnitId) -> i32) -> Option<(i32, Vec<(UnitId, i32)>)> {
    let d = g.d;
    let hp = |_: &Game, e: UnitId| health(e);
    let info = &d.units[ut];
    if info.combat + info.damage_type_combat.iter().sum::<i32>() <= 0 || info.only_defensive {
        return None;
    }
    let team = g.players[p].team;
    let defs: Vec<UnitId> = g.map.plots[tp]
        .units
        .iter()
        .copied()
        .filter(|&e| g.unit_is_enemy(e, Some(team), None) && g.unit_can_defend(e, Some(tp)) && !g.unit_is_invisible(e, team, false) && health(e) > 0)
        .collect();
    if defs.is_empty() {
        return None;
    }
    let max_hp = d.defines.max_hit_points.max(1);
    let strength = |e: UnitId| g.unit_max_combat_str(e, Some(tp), None, None);
    let best = *defs.iter().max_by_key(|&&e| (strength(e) as i64 * hp(g, e) as i64, std::cmp::Reverse(e))).unwrap();
    // The summon's damage types count as far as the defender does not resist them (C++
    // `maxCombatStr`; switch `BARRAGE`): a Fireball, all fire, does nothing to a Fire Elemental.
    let resist = |dt: DamageType| if crate::switches::BARRAGE.on(g, p) { g.unit_get_damage_type_resist(best, dt) } else { 0 };
    let a = info.combat * 100 + d.damages.keys().map(|dt| info.damage_type_combat[dt] * (100 - resist(dt))).sum::<i32>();
    if a <= 0 {
        return None;
    }
    let d_max = strength(best).max(1);
    let d_hp = hp(g, best).max(1);
    let d_str = (d_max * d_hp / max_hp).max(1);
    let d_fp = (d_max + d_str + 1) / 2;
    let k = CombatOddsInput {
        attacker_strength: a,
        attacker_firepower: a,
        defender_strength: d_str,
        defender_firepower: d_fp,
        attacker_hit_points: max_hp,
        defender_hit_points: d_hp,
        defender_max_hit_points: max_hp,
        attacker_combat_limit: info.combat_limit,
        attacker_first_strikes: info.first_strikes,
        attacker_chance_first_strikes: info.chance_first_strikes,
        attacker_immune_to_first_strikes: info.first_strike_immune,
        defender_first_strikes: g.unit_first_strikes(best),
        defender_chance_first_strikes: g.unit_chance_first_strikes(best),
        defender_immune_to_first_strikes: g.unit_immune_to_first_strikes(best),
    };
    let defines = &d.defines;
    let odds = combat_odds(&k, defines.combat_die_sides, defines.combat_damage) as i64;
    // A lost fight: the rounds won before the summon's hit points are gone (as `campaign::fight`).
    let sf = (a + d_fp + 1) / 2;
    let to_att = (defines.combat_damage * (d_fp + sf) / (a + sf)).max(1);
    let to_def = (defines.combat_damage * (a + sf) / (d_fp + sf)).max(1);
    let losses = (max_hp + to_att - 1) / to_att;
    let q = a as i64 * 1000 / (a + d_str) as i64;
    let won = losses as i64 * q / (1000 - q).max(50);
    let mut wound = (won * to_def as i64).min(d_hp as i64);
    if info.combat_limit < max_hp {
        wound = wound.min((d_hp - (max_hp - info.combat_limit)).max(0) as i64);
    }
    let mut taken = vec![(best, ((odds * d_hp as i64 + (1000 - odds) * wound) / 1000) as i32)];
    if info.collateral_damage > 0 && defs.len() > 1 {
        let cs = a / 100 * info.collateral_damage / 100;
        let limit = info.collateral_damage_limit * max_hp / 100;
        let mut others: Vec<UnitId> = defs.iter().copied().filter(|&o| o != best).collect();
        others.sort_by_key(|&o| (std::cmp::Reverse(hp(g, o)), o));
        for &o in others.iter().take(info.collateral_damage_max_units.max(0) as usize) {
            let ts = g.unit_base_combat_str(o);
            let s2 = (cs + ts + 1) / 2;
            if ts + s2 <= 0 {
                continue;
            }
            let dmg = defines.collateral_combat_damage * (cs + s2) / (ts + s2);
            let cap = limit.min(limit * (cs + s2) / (ts + s2));
            taken.push((o, dmg.min(cap - (max_hp - hp(g, o))).max(0)));
        }
    }
    Some((odds as i32, taken))
}

/// What the casts of our `casters` before a strike on `tp` should take from its defenders `defs`
/// (switch `CASTER_FORCE`; hit points, in `defs`' order), for the force estimates that decide when
/// our stacks gather, close and strike (`campaign::covers`): an expert counts his Mages' Fireballs
/// in the force he needs. Each caster that is no apprentice casts once, the spell taking most hit
/// points of those it knows (damage on every defender, a temporary summon's fight with the
/// defender the rules put up and its collateral damage), each cast on the defenders as the casts
/// before it left them. The strike itself casts first and weighs what is left (`soften`).
pub fn planned_soften(g: &Game, tp: PlotIdx, defs: &[UnitId], casters: &[UnitId]) -> Vec<i32> {
    let d = g.d;
    let max_hp = d.defines.max_hit_points.max(1);
    let start: Vec<i32> = defs.iter().map(|&e| hp(g, e)).collect();
    let mut cur = start.clone();
    let mut casters: Vec<UnitId> = casters.iter().copied().filter(|&c| g.units.contains(c) && combat_caster(g, c) && !apprentice(g, c)).collect();
    casters.sort();
    casters.dedup();
    for c in casters {
        let p = g.units[c].owner;
        let twin = if g.unit_is_twincast(c) { 2 } else { 1 };
        let mut best: Option<(i32, Vec<i32>)> = None;
        for s in known(g, c) {
            let info = &d.spells[s];
            if !info.allow_ai || info.global {
                continue;
            }
            let mut after = cur.clone();
            if info.damage > 0 && info.range >= 1 {
                for (i, &e) in defs.iter().enumerate() {
                    if g.unit_is_immune_to_spell(e, c, s) {
                        continue;
                    }
                    let land = if info.resistable { 100 - g.unit_get_resist_chance(e, c, s) } else { 100 };
                    let dmg = expected_damage_at(g, c, e, info.damage, info.damage_limit, info.damage_type, max_hp - after[i]) * land / 100;
                    after[i] -= dmg;
                }
            } else if let Some(ut) = info.create_unit_type.filter(|_| !info.permanent_unit_create) {
                for _ in 0..info.create_unit_num.max(1) * twin {
                    let health = |e: UnitId| defs.iter().position(|&x| x == e).map_or_else(|| hp(g, e), |i| after[i]);
                    let Some((_, taken)) = summon_fight_at(g, p, ut, tp, &health) else { break };
                    for (e, v) in taken {
                        if let Some(i) = defs.iter().position(|&x| x == e) {
                            after[i] = (after[i] - v).max(0);
                        }
                    }
                }
            } else {
                continue;
            }
            let gain: i32 = cur.iter().zip(&after).map(|(a, b)| a - b).sum();
            if gain > 0 && best.as_ref().map_or(true, |(b, _)| gain > *b) {
                best = Some((gain, after));
            }
        }
        if let Some((_, after)) = best {
            cur = after;
        }
    }
    start.iter().zip(&cur).map(|(a, b)| a - b).collect()
}

/// Value of the summons of `spell` cast by `caster` (temporary summons attack at once; a lasting
/// summon is an army unit, worth having while there is fighting to do).
fn summon_value(g: &Game, caster: UnitId, spell: SpellType, ctx: Ctx) -> i32 {
    let d = g.d;
    let info = &d.spells[spell];
    let Some(ut) = info.create_unit_type else { return 0 };
    let p = g.units[caster].owner;
    let n = info.create_unit_num * if g.unit_is_twincast(caster) { 2 } else { 1 };
    let uinfo = &d.units[ut];
    let strength = uinfo.combat + uinfo.damage_type_combat.iter().sum::<i32>();
    let here = g.unit_plot(caster);
    let (x, y) = g.map.xy(here);
    match ctx {
        Ctx::Attack { tp, .. } => {
            // Only next to the target (it appears on the caster's plot), and a lasting summon
            // cannot move for two turns.
            let (tx, ty) = g.map.xy(tp);
            if info.permanent_unit_create || g.plot_distance(x, y, tx, ty) != 1 {
                return 0;
            }
            let (num, den) = crate::traits::summon_turns(g, p); // trait-play
            summon_strike(g, p, ut, tp) * n * num / den
        }
        Ctx::Turn { v } => {
            if info.permanent_unit_create {
                // A unit for good (it pays support like any other): while at war or with enemies
                // about.
                let near = v.danger_at(here) > 0 || v.at_war();
                return if near { strength * 10 * n } else { crate::traits::peace_summon(g, p, strength, n) }; // trait-play
            }
            // It attacks what `casting::summons_attack` picks: of the plots with enemies next to a
            // plot it reaches with moves left (its own moves, the caster's way of moving), the one
            // where its odds are best, at 5% or better; nothing when none is. A summon gone before
            // it could act again that is not the stack's cover takes worse odds too, but not on a
            // city, whose defenders heal before we strike (`SUMMON_SPEND`, `casting::summon_target`).
            let moves = (uinfo.moves + g.unit_get_extra_spell_move(caster)) * d.defines.move_denominator;
            let last_turn = uinfo.special_unit_type.map_or(-1, |s| s.i()) == d.defines.specialunit_spell || g.players[p].summon_duration <= 0;
            let spend = crate::switches::SUMMON_SPEND.on(g, p)
                && last_turn
                && !crate::casting::is_cover(g, v, here, crate::view::type_defense_power(d, ut), None);
            let reach = crate::reach::Flood::default().run_moves(g, caster, moves, 0, 0, false);
            let mut seen: Vec<PlotIdx> = Vec::new();
            let mut best: Option<(i64, i32)> = None;
            for r in reach.iter().filter(|r| r.turns == 0) {
                let (rx, ry) = g.map.xy(r.plot);
                for dir in 0..DirectionType::NUM {
                    let Some(q) = g.plot_direction(rx, ry, DirectionType::from_usize(dir)) else { continue };
                    if seen.contains(&q) {
                        continue;
                    }
                    seen.push(q);
                    if g.plot_is_water(q) != (uinfo.domain_type == Some(DomainType::DOMAIN_SEA)) {
                        continue;
                    }
                    let units = &g.map.plots[q].units;
                    if !units.iter().any(|&t| side(g, t, v.team) == Side::Enemy) || units.iter().all(|&t| g.units[t].duration > 0) {
                        continue;
                    }
                    let Some((odds, value)) = summon_fight(g, p, ut, q) else { continue };
                    let city = g.map.plots[q].plot_city.is_some();
                    let Some(score) = crate::casting::summon_score(odds, city, || if spend && !city { value } else { 0 }) else { continue };
                    if best.map_or(true, |(s, _)| score > s) {
                        best = Some((score, value));
                    }
                }
            }
            let (num, den) = crate::traits::summon_turns(g, p); // trait-play
            best.map_or(0, |(_, value)| value) * 4 / 5 * n * num / den
        }
    }
}

// ----------------------------------------------------------------------------
// Values
// ----------------------------------------------------------------------------

/// What `spell` cast by `caster` is worth just before our `attackers` attack `tp`, in hit points
/// (`i32::MIN / 2` when it would hurt a third party).
pub fn attack_value(g: &Game, caster: UnitId, spell: SpellType, tp: PlotIdx, attackers: &[UnitId]) -> i32 {
    let ctx = Ctx::Attack { tp, attackers };
    let (Some(a), Some(b)) = (reached_value(g, caster, spell, ctx), py_value(g, caster, spell, ctx)) else { return i32::MIN / 2 };
    a + b + summon_value(g, caster, spell, ctx)
}

/// What `spell` cast by `u` at the end of the turn is worth (see the module doc), `None` for a
/// spell FfH's terms value ([`understood`]). Besides the effects: FfH's own terms for gold,
/// conversion, population, sacrifice, features, and the spell's AI weight (FfH's explicit
/// preferences, such as the Balseraphs' puppets).
pub fn turn_value(g: &Game, v: &View, u: UnitId, spell: SpellType) -> Option<i32> {
    let d = g.d;
    if !understood(d, spell) {
        return None;
    }
    let info = &d.spells[spell];
    let ctx = Ctx::Turn { v };
    let (Some(a), Some(b)) = (reached_value(g, u, spell, ctx), py_value(g, u, spell, ctx)) else { return Some(i32::MIN / 2) };
    let mut value = a + b + summon_value(g, u, spell, ctx);
    let here = g.unit_plot(u);
    if let Some(b) = info.create_building_type {
        if let Some(c) = g.map.plots[here].plot_city {
            value += crate::production::building_value(g, c, b, 0) / 4;
        }
    }
    value += crate::casting::ffh_misc_value(g, u, spell);
    Some(value + info.ai_weight)
}

/// The spell `u` casts at the end of the turn: the best by [`turn_value`] (FfH's terms for the
/// spells it leaves alone), after the world spells' timing and the civilization's say.
pub fn choose_turn_spell(g: &Game, v: &View, u: UnitId) -> Option<(SpellType, i32)> {
    choose_turn_spell_from(g, v, u, &crate::casting::castable_spells(g, u))
}

/// [`choose_turn_spell`] among `castable`, the spells `u` can cast now
/// (`casting::castable_spells`).
pub fn choose_turn_spell_from(g: &Game, v: &View, u: UnitId, castable: &[SpellType]) -> Option<(SpellType, i32)> {
    let mut best: Option<(SpellType, i32)> = None;
    for &s in castable {
        if !crate::world_spells::ready(g, g.units[u].owner, s) {
            continue;
        }
        let value = turn_value(g, v, u, s).unwrap_or_else(|| crate::casting::ffh_value(g, u, s));
        let value = crate::civs::spell_value(g, u, s, value);
        if value > best.map_or(0, |(_, b)| b) {
            best = Some((s, value));
        }
    }
    best
}

// ----------------------------------------------------------------------------
// Casting before an attack
// ----------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Phase {
    /// Damage, curses, dispels: the defenders get weaker.
    Soften,
    /// Summons attack the weakened defenders.
    Summon,
    /// Blessings on the units about to attack.
    Bless,
}

fn phase(d: &GameData, s: SpellType) -> Phase {
    let i = &d.spells[s];
    if i.create_unit_type.is_some() {
        Phase::Summon
    } else if i.damage == 0 && py_kind(d, s) == Py::None && i.add_promotion_type1.is_some_and(|p| d.promotions[p].ai_weight > 0) {
        Phase::Bless
    } else {
        Phase::Soften
    }
}

/// A cast before an attack must be worth this many hit points.
const MIN_ATTACK_VALUE: i32 = 5;

/// Casting before player `v.p`'s units attack `tp` (see the module doc): [`soften`], then
/// [`bless`]. For an attack that is going ahead.
pub fn before_attack(g: &mut Game, v: &View, tp: PlotIdx, attackers: &[UnitId]) {
    soften(g, v, tp, attackers);
    bless(g, v, tp, attackers);
}

/// The first casts before an attack on `tp`: what weakens its defenders, then summons, which
/// attack it at once (the player's summon-suicide mode is on meanwhile, as for FfH's end-of-turn
/// casting: they do not join the caster's stack). Worth casting before deciding whether to
/// attack: the decision weighs the defenders as they are left. `attackers`: our units that may
/// attack.
pub fn soften(g: &mut Game, v: &View, tp: PlotIdx, attackers: &[UnitId]) {
    cast_phases(g, v, tp, attackers, &[Phase::Soften, Phase::Summon]);
}

/// The blessings on `attackers` before they attack `tp` (only once the attack goes ahead: a
/// blessing that wears off by our next turn is lost otherwise).
pub fn bless(g: &mut Game, v: &View, tp: PlotIdx, attackers: &[UnitId]) {
    cast_phases(g, v, tp, attackers, &[Phase::Bless]);
}

/// The casts of `phases` (see [`before_attack`]). A unit casts once a turn, so a caster casts
/// here only what is worth more than what it would cast at the end of the turn (its best value
/// now by [`choose_turn_spell`]: the Balseraphs' Loki keeps his cast for the puppet FfH's weight
/// asks for every turn).
fn cast_phases(g: &mut Game, v: &View, tp: PlotIdx, attackers: &[UnitId], phases: &[Phase]) {
    let p = v.p;
    if !enabled(g, p) || !g.map.plots[tp].units.iter().any(|&t| g.unit_is_enemy(t, Some(g.players[p].team), None)) {
        return;
    }
    let d = g.d;
    let (tx, ty) = g.map.xy(tp);
    let mut casters: Vec<UnitId> = square(g, tx, ty, 2)
        .into_iter()
        .flat_map(|q| g.map.plots[q].units.iter().copied().filter(|&u| g.units[u].owner == p).collect::<Vec<_>>())
        .collect();
    casters.sort();
    casters.dedup();
    // The spells each could cast at all (re-checked before every cast), and what it would cast
    // at the end of the turn instead.
    let mut book: Vec<(UnitId, Vec<SpellType>, i32)> = casters
        .into_iter()
        .filter(|&u| !g.units[u].has_casted && !g.units[u].death_delay && !g.unit_is_cargo(u))
        .filter_map(|u| {
            let castable = crate::casting::castable_spells(g, u);
            let ss: Vec<SpellType> = castable.iter().copied().filter(|&s| d.spells[s].allow_ai && !d.spells[s].global).collect();
            if ss.is_empty() {
                return None;
            }
            let reserve = choose_turn_spell_from(g, v, u, &castable).map_or(0, |(_, val)| val);
            Some((u, ss, reserve))
        })
        .collect();
    if book.is_empty() {
        return;
    }
    let was = g.players[p].sum_sui_mode;
    g.players[p].sum_sui_mode = true;
    for &ph in phases {
        for _ in 0..book.len() {
            let mut best: Option<(i32, UnitId, SpellType)> = None;
            for (u, spells, reserve) in &book {
                let u = *u;
                if !g.units.contains(u) || g.units[u].has_casted {
                    continue;
                }
                for &s in spells {
                    if phase(d, s) != ph || !g.unit_can_cast(u, s, false) {
                        continue;
                    }
                    let value = attack_value(g, u, s, tp, attackers);
                    if value >= MIN_ATTACK_VALUE.max(*reserve) && best.map_or(true, |(b, bu, _)| value > b || value == b && u < bu) {
                        best = Some((value, u, s));
                    }
                }
            }
            let Some((value, u, s)) = best else { break };
            let newest = g.players[p].units.iter().copied().max();
            cast(g, u, s, "attack", value);
            if ph == Phase::Summon {
                summons_attack(g, p, newest, tp);
            }
            book.retain(|(c, _, _)| g.units.contains(*c));
        }
        if !g.map.plots[tp].units.iter().any(|&t| g.unit_is_enemy(t, Some(g.players[p].team), None)) {
            break;
        }
    }
    g.players[p].sum_sui_mode = was;
}

/// The summons player `p` just made (unit ids are never reused: those above `newest`) attack
/// `tp` while they can, each out of its group (a summon joins its caster's group).
fn summons_attack(g: &mut Game, p: PlayerId, newest: Option<UnitId>, tp: PlotIdx) {
    let (tx, ty) = g.map.xy(tp);
    let summons: Vec<UnitId> = g.players[p].units.iter().copied().filter(|&n| newest.map_or(true, |m| n > m) && g.units[n].ai_suicide_summon).collect();
    for n in summons {
        for _ in 0..4 {
            if !g.units.contains(n) || g.unit_moves_left(n) <= 0 || !g.plot_is_visible_enemy_unit_for_unit(tp, n) {
                break;
            }
            let (x, y) = (g.units[n].x, g.units[n].y);
            if g.plot_distance(x, y, tx, ty) != 1 || !g.unit_can_move_or_attack_into(n, tp, false) {
                break;
            }
            if g.units[n].group.is_some_and(|grp| g.groups[grp].units.len() > 1) {
                g.unit_join_group(n, None, true);
            }
            let Some(grp) = g.units[n].group else { break };
            let moves = g.unit_moves_left(n);
            crate::units::move_to(g, grp, tp, MOVE_DIRECT_ATTACK);
            if g.units.contains(n) && g.unit_moves_left(n) == moves && !g.units[n].made_attack {
                break;
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Where a caster goes
// ----------------------------------------------------------------------------

/// The spells `u` holds the promotions for and may cast at all (its unit, class, unit combat,
/// civilization and religion fit), world spells and abilities aside.
fn known(g: &Game, u: UnitId) -> Vec<SpellType> {
    let d = g.d;
    let unit = &g.units[u];
    d.spells
        .keys()
        .filter(|&s| {
            let i = &d.spells[s];
            !i.global && !i.ability && [i.promotion_prereq1, i.promotion_prereq2].into_iter().flatten().all(|q| unit.has_promotion[q]) && could_cast(g, u, s)
        })
        .collect()
}

/// Where caster `u` of player `v.p` should go this turn (`units::caster`), and why: with the
/// stack of our operation nearest its target when it knows a spell for a fight and the operation
/// is under way (in the rally city while the force for a war gathers), so it stands next to the
/// city when the strike comes; else where its lasting blessings (on the units there that lack
/// them) or its city spells (a building the city lacks) are worth most for the turns it takes to
/// get there, within 8 turns. `None`: nothing to do there (it guards the most threatened city).
pub fn caster_target(g: &Game, v: &mut View, u: UnitId) -> Option<(PlotIdx, &'static str)> {
    strike_target(g, v, u).or_else(|| support_target(g, v, u))
}

/// The strike part of [`caster_target`]: with the stack of our operation nearest its target (in
/// the rally city while the force for a war gathers), for a caster that knows a spell for a fight.
pub fn strike_target(g: &Game, v: &View, u: UnitId) -> Option<(PlotIdx, &'static str)> {
    let p = v.p;
    // An apprentice stays away from the front ([`apprentice`]).
    if combat_caster(g, u) && !apprentice(g, u) {
        for op in &v.campaign.ops {
            if !g.cities.contains(op.city) || g.cities[op.city].owner == p {
                continue;
            }
            if op.phase == crate::campaign::Phase::Gather {
                if op.civ {
                    if let Some(r) = op.rally {
                        return Some((r, "rally"));
                    }
                }
                continue;
            }
            let (tx, ty) = g.map.xy(op.plot);
            let front = op
                .groups
                .iter()
                .copied()
                .filter(|&grp| crate::units::alive(g, grp))
                .filter_map(|grp| g.group_get_head_unit(grp).map(|h| (grp, g.unit_plot(h))))
                .min_by_key(|&(grp, q)| {
                    let (x, y) = g.map.xy(q);
                    (g.plot_distance(x, y, tx, ty), std::cmp::Reverse(g.groups[grp].units.len()), grp)
                });
            if let Some((_, q)) = front {
                return Some((q, "strike"));
            }
        }
    }
    None
}

/// The rest of [`caster_target`]: where the caster's lasting blessings, city spells or
/// terraforming are worth most for the turns it takes.
pub fn support_target(g: &Game, v: &mut View, u: UnitId) -> Option<(PlotIdx, &'static str)> {
    let d = g.d;
    let p = v.p;
    let spells: Vec<SpellType> = known(g, u)
        .into_iter()
        .filter(|&s| {
            let i = &d.spells[s];
            i.create_building_type.is_some()
                || matches!(py_kind(d, s), Py::Spring | Py::Scorch | Py::Vitalize)
                || !i.buff_caster_only && adds(i).any(|pr| d.promotions[pr].ai_weight > 0 && d.promotions[pr].expire_chance == 0 && !d.promotions[pr].removed_when_healed)
        })
        .collect();
    if spells.is_empty() {
        return None;
    }
    let here = g.unit_plot(u);
    // An apprentice works at home: its city spells and blessings in our cities, terraforming
    // where no enemy can come ([`apprentice`]).
    let apprentice = apprentice(g, u);
    // A building only a caster keeps (Inspiration, Wall of Stone, ...) goes when it leaves: the
    // caster that keeps one stays unless twice as much waits elsewhere.
    let keep = upkeep_value(g, u, &spells);
    let mut best: Option<(i32, PlotIdx, &'static str)> = (keep > 0).then_some((keep * 10, here, "keep"));
    let reach = v.flood.run(g, u, 8, false);
    for r in &reach {
        let q = r.plot;
        let units = &g.map.plots[q].units;
        if apprentice && (v.danger_at(q) > 0 || g.map.plots[q].owner != Some(p)) {
            continue;
        }
        // Our land to terraform (no one else heading there).
        if !v.is_claimed(q) {
            for &s in spells.iter().filter(|&&s| matches!(py_kind(d, s), Py::Spring | Py::Scorch | Py::Vitalize)) {
                let value = terraform_value(g, p, s, q);
                let score = value * 10 / (r.turns + 2);
                if value >= 15 && best.map_or(true, |(b, bq, _)| score > b || score == b && q < bq) {
                    best = Some((score, q, "terraform"));
                }
            }
        }
        if !units.iter().any(|&t| g.units[t].owner == p) {
            continue;
        }
        for &s in &spells {
            let info = &d.spells[s];
            if matches!(py_kind(d, s), Py::Spring | Py::Scorch | Py::Vitalize) {
                continue;
            }
            let mut value = 0;
            let mut why = "bless";
            if let Some(b) = info.create_building_type {
                if let Some(c) = g.map.plots[q].plot_city.filter(|&c| g.cities[c].owner == p && g.city_num_building(c, b) == 0 && !claimed_city_spell(g, u, c, s)) {
                    value += crate::production::building_value(g, c, b, 0) / 4;
                    why = "city spell";
                }
            }
            for pr in adds(info) {
                let pinfo = &d.promotions[pr];
                if pinfo.ai_weight <= 0 || pinfo.expire_chance != 0 || pinfo.removed_when_healed {
                    continue;
                }
                for &t in units {
                    if t != u && g.units[t].owner == p && takes(g, t, pr) && !g.unit_is_immune_to_spell(t, u, s) {
                        value += ours_worth(g, v, t, pr);
                    }
                }
            }
            if value < 15 {
                continue;
            }
            let score = value * 10 / (r.turns + 2);
            if best.map_or(true, |(b, bq, _)| score > b || score == b && q < bq) {
                best = Some((score, q, why));
            }
        }
    }
    if let Some((_, q, "terraform")) = best {
        v.claimed.push(q);
    }
    best.map(|(_, q, why)| (q, why))
}

/// Does caster `u` alone keep a building in our city it stands in ([`upkeep_value`])? It stays.
pub fn keeps_building(g: &Game, u: UnitId) -> bool {
    let d = g.d;
    let spells: Vec<SpellType> = known(g, u).into_iter().filter(|&s| d.spells[s].create_building_type.is_some()).collect();
    !spells.is_empty() && upkeep_value(g, u, &spells) > 0
}

/// What the buildings `u` alone keeps in our city it stands in are worth: those that need a
/// caster (C++ `CvCity::doTurn` removes a `bRequiresCaster` building when no unit in the city can
/// cast the spell that makes it) made by one of `spells` and that no other unit there could cast.
fn upkeep_value(g: &Game, u: UnitId, spells: &[SpellType]) -> i32 {
    let d = g.d;
    let here = g.unit_plot(u);
    let Some(c) = g.map.plots[here].plot_city.filter(|&c| g.cities[c].owner == g.units[u].owner) else { return 0 };
    let mut value = 0;
    for &s in spells {
        let Some(b) = d.spells[s].create_building_type.filter(|&b| d.buildings[b].requires_caster && g.city_num_building(c, b) > 0) else { continue };
        if !g.map.plots[here].units.iter().any(|&o| o != u && g.unit_can_cast(o, s, true)) {
            value += crate::production::building_value(g, c, b, 0) / 4;
        }
    }
    value
}

/// Is another caster of ours already in city `c` able to cast `s` (it will make the building,
/// or keeps it)?
fn claimed_city_spell(g: &Game, u: UnitId, c: CityId, s: SpellType) -> bool {
    let cp = g.city_plot(c);
    g.map.plots[cp].units.iter().any(|&o| o != u && g.units[o].owner == g.units[u].owner && g.unit_can_cast(o, s, true))
}

// ----------------------------------------------------------------------------
// The caster school
// ----------------------------------------------------------------------------
//
// How the rules raise an arcane caster: an Adept gains experience by itself, each turn with
// chance (spellcaster XP - experience)% (`CvUnit::doTurn`, bFreeXP: 20 with Channeling I, more
// with Channeling II and III and with the Arcane trait's Potency), not by casting or by where it
// stands; at level 4, with Sorcery, it becomes a Mage (Channeling II: Fireball, Maelstrom, the
// Spectre, Pit Beast and Ice Elemental), at level 6 with Strength of Will an Archmage
// (Channeling III: the elementals). No Channeling I spell summons a fighter or does damage: an
// Adept's spells are curses, blessings, city spells and terraforming. So an Adept adds little at
// the front and its worth is its age: lost there (strength 3, most often alone on the way), it
// takes with it the 30 to 70 turns it studied -- also once it has the level, until the tech and
// the gold make it a Mage. The school: an apprentice works at home and waits in our city
// (`caster_target`, [`study_city`]); every caster crosses dangerous ground only
// where our units stand ([`safe_step`]); a player with an apprentice near its level researches
// the tech that makes it a Mage ([`school_tech_value`]) and pays for the upgrade before a soldier's
// (`units::upgrade`).

/// Is `u` an apprentice of the caster school ([`school`]): an arcane caster of ours (unit combat
/// Adept, not a hero or a summon) without Channeling II that can still become a Mage -- at any
/// level: an Adept of level 4 casts an Adept's spells until the upgrade.
pub fn apprentice(g: &Game, u: UnitId) -> bool {
    apprentice_need(g, u).is_some()
}

/// The level and unit type of an [`apprentice`]'s upgrade: the lowest `iMinLevel` among them (a
/// Mage's 4).
fn apprentice_need(g: &Game, u: UnitId) -> Option<(i32, UnitType)> {
    static CHANNELING2: OnceLock<Option<PromotionType>> = OnceLock::new();
    let d = g.d;
    let channeling2 = *CHANNELING2.get_or_init(|| d.ty::<PromotionType>("PROMOTION_CHANNELING2"));
    let unit = &g.units[u];
    let info = &d.units[unit.unit_type];
    if !arcane(g, u) || !school(g, unit.owner) || channeling2.is_some_and(|pr| unit.has_promotion[pr]) {
        return None;
    }
    let civ = g.player_get_civilization_type(unit.owner)?;
    info.upgrade_unit_class
        .iter_enumerated()
        .filter(|&(_, &ok)| ok)
        .filter_map(|(class, _)| d.civ_unit(civ, class))
        .filter(|&t| d.units[t].min_level > 0)
        .map(|t| (d.units[t].min_level, t))
        .min()
}

/// An arcane caster: unit combat Adept (Adepts, Mages, Archmages and their variants), not a hero
/// or a summon.
fn arcane(g: &Game, u: UnitId) -> bool {
    static IDS: OnceLock<(Option<UnitCombatType>, Option<PromotionType>)> = OnceLock::new();
    let d = g.d;
    let &(adept, hero) = IDS.get_or_init(|| (d.ty::<UnitCombatType>("UNITCOMBAT_ADEPT"), d.ty::<PromotionType>("PROMOTION_HERO")));
    let unit = &g.units[u];
    adept.is_some() && d.units[unit.unit_type].unit_combat_type == adept && unit.duration == 0 && !hero.is_some_and(|h| unit.has_promotion[h])
}

/// The odds (per mille) from which a caster that marches with a stack attacks ([`spared_caster`]):
/// FfH's own bar for its war wizards (`AI_getBestGroupAttacker`, Sephi's block, which the rules'
/// group attack keeps).
pub const SPARED_CASTER_ODDS: i32 = 950;

/// Does caster `u` go to war in a stack without fighting for it (`CASTER_STACKS`): an arcane
/// caster of a school player. Its worth is its spells and the levels it took to learn them; an
/// attack at strength 4 or 5 risks them for little. Below [`SPARED_CASTER_ODDS`] the strike leaves
/// it out, a stack's assault holds rather than send it, and the campaign's estimates of the force
/// do not count it.
pub fn spared_caster(g: &Game, u: UnitId) -> bool {
    let p = g.units[u].owner;
    arcane(g, u) && school(g, p) && crate::switches::CASTER_STACKS.on(g, p)
}

/// Fighters a stack needs for a caster to join it ([`stack_to_join`]); below [`STACK_KEEP`] the
/// casters leave it ([`leave_stack`]). A group of one to three units is a straggler or a
/// reinforcement on its way, not the stack that strikes: a third of the joins went to such groups,
/// and a caster shared their fate.
const STACK_JOIN: usize = 4;
const STACK_KEEP: usize = 2;

/// Units of group `grp` that fight (`campaign::fights`: not the casters marching with it).
fn stack_fighters(g: &Game, grp: GroupId) -> usize {
    g.groups[grp].units.iter().filter(|&&o| crate::campaign::fights(g, o)).count()
}

/// The stack caster `u` joins where it stands (`CASTER_STACKS`; `units::caster`, for a caster
/// [`strike_target`] sends to an operation): a committed group of ours there with
/// [`STACK_JOIN`] fighters or more, headed by one, the largest. It then moves with that stack,
/// which guards it on the way and carries it to the city at the stack's pace -- marching on its
/// own from one sheltered plot to the next, it fell behind any stack that moved faster or crossed
/// open ground.
pub fn stack_to_join(g: &Game, v: &View, u: UnitId) -> Option<GroupId> {
    if !spared_caster(g, u) {
        return None;
    }
    let here = g.unit_plot(u);
    v.campaign
        .ops
        .iter()
        .flat_map(|op| op.groups.iter().copied())
        .filter(|&grp| crate::units::alive(g, grp) && g.units[u].group != Some(grp))
        .filter(|&grp| g.group_get_head_unit(grp).is_some_and(|h| g.unit_plot(h) == here && !spared_caster(g, h) && attack_power(g, h) > 0))
        .filter(|&grp| stack_fighters(g, grp) >= STACK_JOIN)
        .max_by_key(|&grp| (stack_fighters(g, grp), std::cmp::Reverse(grp)))
}

/// The spared casters of group `from` join stack `grp` on their plot, as war wizards (the AI type
/// FfH's group attack keeps from attacking below [`SPARED_CASTER_ODDS`]). True when they joined.
pub fn join_stack(g: &mut Game, from: GroupId, grp: GroupId) -> bool {
    let Some(head) = g.group_get_head_unit(grp) else { return false };
    let at = g.unit_plot(head);
    let mut joined = false;
    for u in g.groups[from].units.clone() {
        if !spared_caster(g, u) || g.unit_plot(u) != at {
            continue;
        }
        if g.units[u].ai_type != Some(UnitAIType::UNITAI_WARWIZARD) {
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_WARWIZARD);
        }
        g.unit_join_group(u, Some(grp), true);
        if g.units[u].group == Some(grp) {
            trace_stack(g, u, "join");
            joined = true;
        }
    }
    joined
}

/// Spared casters leave stack `grp` (each into a group of its own) once it is out of our
/// operations, or fewer than [`STACK_KEEP`] units of ours that fight are left in it -- a strike
/// takes its attackers out of their stacks, a battle kills them (`CASTER_STACKS`; after the stack's orders, `units::play`, which
/// gives the casters their own orders at once: left where the strike left them, with no one
/// beside them, they were lost alone). Left in, a caster would follow a stack home to heal or to
/// guard, or head a group of casters by the army's orders. The casters released, with their
/// groups.
pub fn leave_stack(g: &mut Game, v: &View, grp: GroupId) -> Vec<(UnitId, GroupId)> {
    if !crate::units::alive(g, grp) {
        return Vec::new();
    }
    let casters: Vec<UnitId> = g.groups[grp].units.iter().copied().filter(|&u| spared_caster(g, u)).collect();
    if casters.is_empty() {
        return Vec::new();
    }
    if stack_fighters(g, grp) >= STACK_KEEP && v.campaign.op_of(grp).is_some() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for u in casters {
        trace_stack(g, u, "leave");
        if g.groups[grp].units.len() > 1 {
            g.unit_join_group(u, None, true);
        }
        if let Some(ng) = g.units[u].group {
            out.push((u, ng));
        }
    }
    out
}

/// At the start of player `p`'s turn: a group a spared caster heads (its fighters fell or left
/// it) splits, each caster into a group of its own, so that no caster leads others by its own
/// orders (`CASTER_STACKS`).
pub fn split_caster_heads(g: &mut Game, p: PlayerId) {
    for grp in g.players[p].groups.clone() {
        if !crate::units::alive(g, grp) || g.groups[grp].units.len() < 2 || !g.group_get_head_unit(grp).is_some_and(|h| spared_caster(g, h)) {
            continue;
        }
        for u in g.groups[grp].units.clone() {
            if spared_caster(g, u) && g.groups[grp].units.len() > 1 {
                g.unit_join_group(u, None, true);
            }
        }
    }
}

/// Can caster `u` of player `v.p` end its turn on `q`: in our city, where no enemy we see can
/// reach (not in an enemy's land), or beside a unit of ours at least as strong (that defends the
/// plot before it does; not another caster, and a Scout is no escort).
pub fn sheltered(g: &Game, v: &View, u: UnitId, q: PlotIdx) -> bool {
    let d = g.d;
    let p = v.p;
    let plot = &g.map.plots[q];
    if plot.plot_city.is_some_and(|c| g.cities[c].owner == p) {
        return true;
    }
    if v.danger_at(q) == 0 && !plot.owner.is_some_and(|o| v.enemies.contains(&g.players[o].team)) {
        return true;
    }
    let strength = |t: UnitId| {
        let info = &d.units[g.units[t].unit_type];
        info.combat.max(info.combat_defense)
    };
    let caster_combat = d.units[g.units[u].unit_type].unit_combat_type;
    plot.units.iter().any(|&o| {
        o != u
            && g.units[o].owner == p
            && g.units[o].duration == 0
            && (caster_combat.is_none() || d.units[g.units[o].unit_type].unit_combat_type != caster_combat)
            && strength(o) >= strength(u)
    })
}

/// Where caster `u` heading for `q` goes this turn when it is a school caster ([`school`]): the
/// [`sheltered`] plot it can reach this turn that is nearest `q` (here when none is nearer);
/// `None` when it can reach none (it falls back to [`study_city`]).
pub fn safe_step(g: &Game, v: &mut View, u: UnitId, q: PlotIdx) -> Option<PlotIdx> {
    let (qx, qy) = g.map.xy(q);
    let here = g.unit_plot(u);
    // A budget of 0 turns: the plots it can enter with the moves it has left this turn.
    let reach = v.flood.run(g, u, 0, false);
    let mut best: Option<(i32, i32, PlotIdx)> = None;
    for r in &reach {
        if !sheltered(g, v, u, r.plot) {
            continue;
        }
        let (x, y) = g.map.xy(r.plot);
        let key = (g.plot_distance(x, y, qx, qy), if r.plot == here { 0 } else { 1 }, r.plot);
        if best.map_or(true, |b| key < b) {
            best = Some(key);
        }
    }
    best.map(|(_, _, s)| s)
}

/// Where an idle apprentice of player `v.p` waits: the city of ours it stands in (behind its walls
/// and defenders; leaving one in danger for another meant crossing open ground, where the
/// apprentices that shuttled between two cities died), else the nearest of ours it reaches within
/// 10 turns where no enemy we see can reach, else the nearest.
pub fn study_city(g: &Game, v: &mut View, u: UnitId) -> Option<PlotIdx> {
    let p = v.p;
    let here = g.unit_plot(u);
    if g.map.plots[here].plot_city.is_some_and(|c| g.cities[c].owner == p) {
        return Some(here);
    }
    let reach = v.flood.run(g, u, 10, false);
    reach
        .iter()
        .filter(|r| g.map.plots[r.plot].plot_city.is_some_and(|c| g.cities[c].owner == p))
        .min_by_key(|r| (v.danger_at(r.plot) > 0, r.turns, r.plot))
        .map(|r| r.plot)
}

/// Tech `t`'s worth to player `p` with what its school adds: for each apprentice at most a level
/// short of its upgrade (up to 5), 300 on the tech the upgrade needs and 100 on the techs on the
/// way to it.
pub fn school_tech_value(g: &Game, p: PlayerId, t: TechType, value: i32) -> i32 {
    let team = g.players[p].team;
    if !school(g, p) || g.team_is_has_tech(team, Some(t)) {
        return value;
    }
    let mut needed: Vec<(TechType, i32)> = Vec::new();
    for &u in &g.players[p].units {
        let Some((need, mage)) = g.units.contains(u).then(|| apprentice_need(g, u)).flatten() else { continue };
        if g.units[u].level + 1 < need {
            continue;
        }
        let info = &g.d.units[mage];
        for x in info.prereq_and_tech.into_iter().chain(info.prereq_and_techs.iter().copied()) {
            if g.team_is_has_tech(team, Some(x)) {
                continue;
            }
            match needed.iter_mut().find(|(y, _)| *y == x) {
                Some((_, n)) => *n += 1,
                None => needed.push((x, 1)),
            }
        }
    }
    let mut bonus = 0;
    for (x, n) in needed {
        let n = n.min(5);
        if x == t {
            bonus += 300 * n;
        } else if crate::victory::tech_path(g, team, &[x]).contains(&t) {
            bonus += 100 * n;
        }
    }
    value + bonus
}

// ----------------------------------------------------------------------------
// What a caster can do (promotions)
// ----------------------------------------------------------------------------

/// Rough worth of spell `s` to a caster, on the scale of `units::promotion_value` (Combat I is
/// about 40): a summon by its strength (lasting ones less a bit, more with collateral damage),
/// damage by its size and reach, curses and blessings by the strength they change on the units
/// they reach (blessings on a stack, lasting ones more), Python spells by what they do, a city
/// spell like a good building.
pub fn spell_worth(d: &GameData, s: SpellType) -> i32 {
    let info = &d.spells[s];
    if !info.allow_ai || info.global || info.ability {
        return 0;
    }
    let reach = info.range + 1;
    let mut w = match py_kind(d, s) {
        Py::Crush | Py::PillarOfFire => 100,
        Py::Domination => 150,
        Py::DestroyUndead | Py::Heal => 30,
        Py::Earthquake | Py::Vitalize => 40,
        Py::Spring | Py::Scorch => 20,
        Py::None | Py::Recruit => 0,
    };
    if let Some(ut) = info.create_unit_type {
        let u = &d.units[ut];
        let strength = u.combat + u.damage_type_combat.iter().sum::<i32>();
        let mut sw = strength * if info.permanent_unit_create { 15 } else { 20 };
        if u.collateral_damage > 0 {
            sw = sw * 3 / 2;
        }
        w += (sw.max(5) * info.create_unit_num.max(1)).min(200);
    }
    w += info.damage * reach;
    for pr in adds(info) {
        let p = &d.promotions[pr];
        if p.ai_weight < 0 {
            let curse = (-p.combat_percent_defense).max(0)
                + (-p.extra_combat_defense).max(0) * 20
                + if p.only_defensive { 40 } else { 0 }
                + if p.moves_change < 0 { 30 } else { 0 };
            w += curse * reach;
        } else {
            let gain = p.combat_percent.max(p.combat_percent_defense)
                + (p.extra_combat_str.max(p.extra_combat_defense) + p.damage_type_combat.iter().sum::<i32>()) * 20
                + p.first_strikes_change * 10;
            let stack = if info.buff_caster_only { 1 } else { 3 };
            let lasting = if p.expire_chance == 0 && !p.removed_by_combat { 3 } else { 2 };
            w += gain * stack * lasting / 2 + p.ai_weight / 10;
        }
    }
    if info.immune_team {
        // Weapons and blessings taken from the enemy (Rust): a unit carries one tier of weapons,
        // so the best of them, at half (not every defender has them).
        let taken = removes(info).map(|pr| (d.promotions[pr].extra_combat_defense * 20 + d.promotions[pr].combat_percent_defense).max(0)).max().unwrap_or(0);
        w += taken * reach / 2;
    }
    w += info.immobile_turns * 20 * reach;
    if info.create_building_type.is_some() {
        w += 40;
    }
    if info.dispel {
        w += 30;
    }
    w
}

/// Can `u` itself ever cast `s` once it has the spell's promotions (its unit, class, unit
/// combat, civilization and religion fit)?
fn could_cast(g: &Game, u: UnitId, s: SpellType) -> bool {
    let info = &g.d.spells[s];
    let unit = &g.units[u];
    info.allow_ai
        && info.unit_prereq.map_or(true, |ut| ut == unit.unit_type)
        && info.unit_class_prereq.map_or(true, |c| c == g.unit_get_unit_class_type(u))
        && info.unit_combat_prereq.map_or(true, |c| Some(c) == g.unit_get_unit_combat_type(u))
        && info.civilization_prereq.map_or(true, |c| c == g.unit_get_civilization_type(u))
        && info.religion_prereq.map_or(true, |r| unit.religion == Some(r))
}

/// The best [`spell_worth`] promotion `pr` opens for `u` (its other promotion held or `pr`).
fn opens(g: &Game, u: UnitId, pr: PromotionType, has: &dyn Fn(PromotionType) -> bool) -> i32 {
    let d = g.d;
    d.spells
        .keys()
        .filter(|&s| {
            let i = &d.spells[s];
            (i.promotion_prereq1 == Some(pr) || i.promotion_prereq2 == Some(pr))
                && [i.promotion_prereq1, i.promotion_prereq2].into_iter().flatten().all(|q| q == pr || has(q))
                && could_cast(g, u, s)
        })
        .map(|s| spell_worth(d, s))
        .max()
        .unwrap_or(0)
}

/// What promotion `pr` is worth to `u` for its spells: the best spell it opens, and half the best
/// one a next promotion of the same line opens (Fire II after Fire I) when `u` has the other
/// promotion that one needs (Channeling II) -- a quarter when it has the channeling below it,
/// since an adept becomes a mage only through an upgrade. 0 for a unit that casts nothing.
pub fn promotion_value(g: &Game, u: UnitId, pr: PromotionType) -> i32 {
    let d = g.d;
    let unit = &g.units[u];
    let has = |q: PromotionType| unit.has_promotion[q];
    let now = opens(g, u, pr, &has);
    let mut later = 0;
    for next in d.promotions.keys() {
        let n = &d.promotions[next];
        if n.prereq_promotion != Some(pr) {
            continue;
        }
        let Some(and) = n.promotion_prereq_and else { continue };
        let factor = if has(and) {
            2
        } else if d.promotions[and].prereq_promotion.map_or(false, |below| has(below)) || channeling_below(d, and).is_some_and(|b| has(b)) {
            4
        } else {
            continue;
        };
        let with = |q: PromotionType| q == pr || has(q);
        later = later.max(opens(g, u, next, &with) / factor);
    }
    now + later
}

/// The channeling promotion below `and` (Channeling I below Channeling II), by name.
fn channeling_below(d: &GameData, and: PromotionType) -> Option<PromotionType> {
    match d.promotions[and].base.type_.as_str() {
        "PROMOTION_CHANNELING2" => d.ty::<PromotionType>("PROMOTION_CHANNELING1"),
        "PROMOTION_CHANNELING3" => d.ty::<PromotionType>("PROMOTION_CHANNELING2"),
        _ => None,
    }
}

// ----------------------------------------------------------------------------
// Casters to train
// ----------------------------------------------------------------------------
//
// The corps by role is `casters.rs` (`CASTER_ROLES`), the node builder `mana.rs` (`MANA_NODES`).
// Tried before, not kept (docs/AI_LOG.md, "Spellcasting"): training casters by what their spells
// are worth in a fight alone -- one per five field units when an adept could cast such a spell
// with our mana (Enchanted Blade, Rust), and the Mage Guild that opens them in our most productive
// safe city. The Enchantment civilizations that trained them lost wins (3 against 9 over 80
// games) for no share.

/// Does `u` know a spell that helps in a fight (a summon that attacks, damage, a curse, a
/// blessing for a stack)? Such casters march with the army.
pub fn combat_caster(g: &Game, u: UnitId) -> bool {
    let d = g.d;
    let unit = &g.units[u];
    d.spells.keys().any(|s| {
        let i = &d.spells[s];
        !i.global
            && !i.ability
            && (i.damage > 0
                || (i.create_unit_type.is_some() && !i.permanent_unit_create)
                || matches!(py_kind(d, s), Py::Crush | Py::PillarOfFire | Py::Domination | Py::DestroyUndead)
                || adds(i).any(|p| d.promotions[p].ai_weight != 0 && !i.buff_caster_only))
            && [i.promotion_prereq1, i.promotion_prereq2].into_iter().flatten().all(|q| unit.has_promotion[q])
            && could_cast(g, u, s)
    })
}

/// Does `u` know a spell that hurts enemy units: damage, a temporary summon, Crush, Pillar of Fire
/// or Destroy Undead (a Mage's Fireball, not an Adept's Haste)?
pub fn damage_caster(g: &Game, u: UnitId) -> bool {
    let d = g.d;
    let fights = |t: UnitType| d.units[t].combat + d.units[t].damage_type_combat.iter().sum::<i32>() > 0;
    known(g, u).into_iter().any(|s| {
        let i = &d.spells[s];
        i.damage > 0 || i.create_unit_type.is_some_and(fights) && !i.permanent_unit_create || damage_spell(d, s)
    })
}

/// What sphere promotion `pr` would be worth to a fresh caster of player `p` (Channeling I and no
/// other sphere: an adept just trained), on [`promotion_value`]'s scale: the best spell it opens,
/// and a quarter of the best a next promotion of the line opens (an adept becomes a mage only
/// through an upgrade). For the mana a node would bring (`mana.rs`) and a caster worth training
/// (`casters.rs`).
pub fn sphere_value(g: &Game, p: PlayerId, pr: PromotionType) -> i32 {
    let d = g.d;
    let Some(ch1) = d.ty::<PromotionType>("PROMOTION_CHANNELING1") else { return 0 };
    let ch2 = d.ty::<PromotionType>("PROMOTION_CHANNELING2");
    let civ = g.player_get_civilization_type(p);
    let adept = d.ty::<UnitCombatType>("UNITCOMBAT_ADEPT");
    let castable = |s: SpellType, has: &dyn Fn(PromotionType) -> bool| {
        let i = &d.spells[s];
        i.allow_ai
            && i.unit_prereq.is_none()
            && i.unit_class_prereq.is_none()
            && i.religion_prereq.is_none()
            && i.unit_combat_prereq.map_or(true, |c| Some(c) == adept)
            && i.civilization_prereq.map_or(true, |c| Some(c) == civ)
            && [i.promotion_prereq1, i.promotion_prereq2].into_iter().flatten().all(|q| has(q))
    };
    let best = |has: &dyn Fn(PromotionType) -> bool, need: PromotionType| {
        d.spells
            .keys()
            .filter(|&s| [d.spells[s].promotion_prereq1, d.spells[s].promotion_prereq2].contains(&Some(need)) && castable(s, has))
            .map(|s| spell_worth(d, s))
            .max()
            .unwrap_or(0)
    };
    let now = best(&|q| q == pr || q == ch1, pr);
    let mut later = 0;
    for next in d.promotions.keys().filter(|&n| d.promotions[n].prereq_promotion == Some(pr)) {
        later = later.max(best(&|q| q == next || q == pr || q == ch1 || Some(q) == ch2, next) / 4);
    }
    now + later
}

/// What Teach Spellcasting (Govannon: every unit of ours on his plot, but animals and birds, gains
/// his first-level spheres it lacks, and Channeling I) is worth on plot `q`, on
/// [`promotion_value`]'s scale: for each unit, each sphere it would gain by the spells that
/// opens for it.
pub fn teach_value(g: &Game, caster: UnitId, q: PlotIdx) -> i32 {
    let d = g.d;
    let Some(ch1) = d.ty::<PromotionType>("PROMOTION_CHANNELING1") else { return 0 };
    let p = g.units[caster].owner;
    let animal = d.ty::<UnitCombatType>("UNITCOMBAT_ANIMAL");
    let bird = d.ty::<SpecialUnitType>("SPECIALUNIT_BIRD");
    let spheres: Vec<PromotionType> = d
        .promotions
        .keys()
        .filter(|&pr| g.units[caster].has_promotion[pr] && d.promotions[pr].bonus_prereq.is_some_and(|b| crate::mana::is_mana(d, b)))
        .collect();
    if spheres.is_empty() {
        return 0;
    }
    let mut v = 0;
    for &t in &g.map.plots[q].units {
        if t == caster || g.units[t].owner != p || !g.unit_is_alive(t) || g.unit_get_unit_combat_type(t) == animal || g.unit_get_special_unit_type(t) == bird {
            continue;
        }
        let unit = &g.units[t];
        for &pr in &spheres {
            if unit.has_promotion[pr] {
                continue;
            }
            let has = |o: PromotionType| o == pr || o == ch1 || unit.has_promotion[o];
            v += opens(g, t, pr, &has);
        }
    }
    v
}

/// Our plots where caster `u` could terraform with a spell it knows, and the best value there
/// (`terraform_value`): a terraformer's work (`casters.rs`).
pub fn terraform_work(g: &Game, p: PlayerId, u: UnitId) -> Vec<(PlotIdx, i32)> {
    let d = g.d;
    let spells: Vec<SpellType> = known(g, u).into_iter().filter(|&s| matches!(py_kind(d, s), Py::Spring | Py::Scorch | Py::Vitalize)).collect();
    if spells.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for &c in &g.players[p].cities {
        for i in 0..ffh_core::defs::NUM_CITY_PLOTS {
            let Some(q) = g.city_index_plot(c, i) else { continue };
            if out.iter().any(|&(o, _)| o == q) {
                continue;
            }
            let best = spells.iter().map(|&s| terraform_value(g, p, s, q)).max().unwrap_or(0);
            if best >= 15 {
                out.push((q, best));
            }
        }
    }
    out
}

/// Where terraformer `u` goes: the plot of [`terraform_work`] worth most for the turns it takes
/// (within 8), no one else heading there.
pub fn terraform_target(g: &Game, v: &mut View, u: UnitId) -> Option<PlotIdx> {
    let work = terraform_work(g, v.p, u);
    if work.is_empty() {
        return None;
    }
    let here = g.unit_plot(u);
    let reach = v.flood.run(g, u, 8, false);
    let mut best: Option<(i32, PlotIdx)> = None;
    for (q, value) in work {
        if v.is_claimed(q) || v.danger_at(q) > 0 {
            continue;
        }
        let Some(turns) = (if q == here { Some(0) } else { reach.iter().find(|r| r.plot == q).map(|r| r.turns) }) else { continue };
        let score = value * 10 / (turns + 2);
        if best.map_or(true, |(b, bq)| score > b || score == b && q < bq) {
            best = Some((score, q));
        }
    }
    let q = best?.1;
    v.claimed.push(q);
    Some(q)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worth(d: &GameData, name: &str) -> i32 {
        spell_worth(d, d.t::<SpellType>(name))
    }

    #[test]
    fn spell_worth_ranks_combat_spells() {
        let d = ffh_data::global();
        if std::env::var("FFH_PRINT_SPELL_WORTH").is_ok() {
            let mut all: Vec<(i32, &str)> = d.spells.keys().map(|s| (spell_worth(d, s), d.spells[s].base.type_.as_str())).filter(|&(w, _)| w > 0).collect();
            all.sort();
            for (w, n) in all {
                eprintln!("{w:5} {n}");
            }
        }
        // A summon that attacks with collateral damage beats a one-turn move bonus; a stack
        // blessing that lasts beats one that does not; a curse that takes weapons is worth a
        // caster.
        assert!(worth(d, "SPELL_FIREBALL") > worth(d, "SPELL_HASTE"));
        assert!(worth(d, "SPELL_ENCHANTED_BLADE") > worth(d, "SPELL_DANCE_OF_BLADES"));
        assert!(worth(d, "SPELL_RUST") >= 60);
        assert!(worth(d, "SPELL_ENCHANTED_BLADE") >= 60);
        // World spells and abilities are not valued here.
        assert_eq!(worth(d, "SPELL_RALLY"), 0);
        assert_eq!(worth(d, "SPELL_TAKE_GOLDEN_HAMMER_UNIT"), 0);
    }
}
