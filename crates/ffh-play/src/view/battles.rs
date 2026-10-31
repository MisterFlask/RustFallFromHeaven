//! Engagement combat as the human sees it (docs/ENGAGEMENTS.md, formation battles): the battle
//! reports (`battle_reports`, the view's `battles`, the turn report's `engagements`, the reply of
//! the command that launched one), a battle's log as the battle screens read it (`battle_log`),
//! and the plan of an attack (`engage_preview`).
//!
//! **One shape for the screens.** The engine keeps a [`BattleReport`] (the summary, which
//! outlives the log) and its [`BattleLog`]; the view joins them and words the enums as their
//! names. What the saved report does not hold is derived here, never stored: the attacking plot's
//! terrain (the map's, now), each side's civilization and label, each fighter's display name and
//! promotions (the unit's while it lives, else the promotions among its A and Dv parts).
//!
//! **The log's events** are the engine's in order: the stages, the morale rolls and the units
//! that sat out, the defensive strikes, then every action of the battle as
//! `{kind: "Action", round, pass, actor, action, spell, target, hits: [{target, dmg, expected, r,
//! crit, miss, killed, withdrew, splash}], heal: {target, amount, r}, knocked: {unit, from, to},
//! note}` (fighters by their index in `fighters`), the battle's end, and the post-battle steps (XP,
//! kill effects, heals, flight, the move in).
//!
//! **Per viewer (G5).** A viewer sees the reports of the battles it fought on a side (the
//! engine's `BattleReport::involves`), or every one under debug reveal. Its own side shows its real
//! owners, the other side the owners it saw (`BattleReport::owner_for`: a hidden-nationality
//! attacker stays the barbarian it looked like).
//!
//! **The plan** (`engage_preview`, the engine's `Game::engage_preview`) is the attack before it is
//! made: the pool on the attacking plot, the plan's formation (the one asked for, checked, else
//! the rules' auto formation) and the auto formation, the slots and lanes, the ground and its
//! rules, the defenders the attacker sees and their formation, every pair's A and Dv both ways,
//! and the forecast (the real resolver's battles on a private RNG). A plot the viewer does not
//! see, or one not next to the attackers, has no plan: the reply is its `errors` (and
//! `declare_war`) alone.

use ffh_core::engagement::formation::{Event, Note};
use ffh_core::engagement::report::*;
use ffh_core::engagement::{EngageResult, FighterIdx, GroundRule, HitCtx, Place, PreviewInput, Side, WarTest};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::command::{formation_json, plot_json, EngageBy};
use crate::json::Json;
use crate::names;
use crate::query::{invalid, QueryErr};
use crate::view::{city_name, Viewer};

/// The log schema's version (formation battles: 2).
pub const LOG_VERSION: i32 = 2;

// ---------------------------------------------------------------------------------------------
// Names of the enums

fn side_ix(s: Side) -> usize {
    s.index()
}

pub fn result_name(r: EngageResult) -> &'static str {
    match r {
        EngageResult::Taken => "Taken",
        EngageResult::Cleared => "Cleared",
        EngageResult::Repulsed => "Repulsed",
        EngageResult::Held => "Held",
    }
}

fn fate_json(f: &LogFate) -> Json {
    match f {
        LogFate::Standing => crate::jobj! { "kind" => "Standing" },
        LogFate::Fell { by, round } => crate::jobj! { "kind" => "Fell", "by" => *by, "round" => *round },
        LogFate::Withdrew { round } => crate::jobj! { "kind" => "Withdrew", "round" => *round },
        LogFate::SatOut => crate::jobj! { "kind" => "SatOut" },
    }
}

fn plot_idx_json(g: &Game, p: PlotIdx) -> Json {
    let (x, y) = g.map.xy(p);
    plot_json(x, y)
}

fn rule_json(r: &GroundRule) -> Json {
    let j = |kind: &str| crate::jobj! { "kind" => kind };
    match *r {
        GroundRule::CityShots { pct } => j("CityShots").with("pct", pct),
        GroundRule::ForestRanged { pct } => j("ForestRanged").with("pct", pct),
        GroundRule::HillsAttacker { pct } => j("HillsAttacker").with("pct", pct),
        GroundRule::HillsDefenderRanged { pct } => j("HillsDefenderRanged").with("pct", pct),
        GroundRule::Charge { pct } => j("Charge").with("pct", pct),
        GroundRule::NoFlanking => j("NoFlanking"),
    }
}

fn rules_json(rules: &[GroundRule]) -> Json {
    Json::Arr(rules.iter().map(rule_json).collect())
}

fn ground_json(g: &ffh_core::engagement::Ground, rules: &[GroundRule]) -> Json {
    crate::jobj! { "city" => g.city, "forest" => g.forest, "hills" => g.hills, "flanking" => g.flanking(), "rules" => rules_json(rules) }
}

fn actions_json(rows: &[RowActions; ROWS]) -> Json {
    Json::Arr(rows.iter().map(|r| crate::jobj! { "kind" => r.kind.token(), "count" => r.count }).collect())
}

// ---------------------------------------------------------------------------------------------
// Parts: terms, stats, slots

/// A term of A or Dv: `{src, pct}` and the source's payload (`promotion`, `terrain`, ...).
pub fn term_json(t: &Term) -> Json {
    let j = |src: &str| crate::jobj! { "src" => src, "pct" => t.pct };
    match &t.src {
        TermSource::Own => j("Own"),
        TermSource::Promotion(p) => j("Promotion").with("promotion", Json::idx(*p)),
        TermSource::Spell(s) => j("Spell").with("spell", Json::idx(*s)),
        TermSource::BattleEffect(s) => j("BattleEffect").with("spell", Json::idx(*s)),
        TermSource::Hills => j("Hills"),
        TermSource::Terrain(x) => j("Terrain").with("terrain", Json::idx(*x)),
        TermSource::Feature(x) => j("Feature").with("feature", Json::idx(*x)),
        TermSource::Fortify => j("Fortify"),
        TermSource::CityDefense => j("CityDefense"),
        TermSource::UnitCityDefense => j("UnitCityDefense"),
        TermSource::UnitHillsDefense => j("UnitHillsDefense"),
        TermSource::UnitTerrainDefense(x) => j("UnitTerrainDefense").with("terrain", Json::idx(*x)),
        TermSource::UnitFeatureDefense(x) => j("UnitFeatureDefense").with("feature", Json::idx(*x)),
        TermSource::CityAttack => j("CityAttack"),
        TermSource::UnitTerrainAttack(x) => j("UnitTerrainAttack").with("terrain", Json::idx(*x)),
        TermSource::UnitFeatureAttack(x) => j("UnitFeatureAttack").with("feature", Json::idx(*x)),
        TermSource::Amphibious => j("Amphibious"),
        TermSource::VsUnitCombat(x) => j("VsUnitCombat").with("unit_combat", Json::idx(*x)),
        TermSource::VsUnitClass(x) => j("VsUnitClass").with("unit_class", Json::idx(*x)),
        TermSource::VsPromotion(p) => j("VsPromotion").with("promotion", Json::idx(*p)),
        TermSource::VsAnimal => j("VsAnimal"),
        TermSource::VsBarbarian => j("VsBarbarian"),
        TermSource::VsDomain(x) => j("VsDomain").with("domain", Json::idx(*x)),
        TermSource::InBorders => j("InBorders"),
        TermSource::GlobalCounter => j("GlobalCounter"),
        TermSource::Handicap => j("Handicap"),
        TermSource::Aura(u) => j("Aura").with("unit", *u),
        TermSource::Typed(x) => j("Typed").with("damage", Json::idx(*x)),
        TermSource::UnitHillsAttack => j("UnitHillsAttack"),
        TermSource::Improvement(x) => j("Improvement").with("improvement", Json::idx(*x)),
        TermSource::RangeDefense => j("RangeDefense"),
    }
}

fn terms_json(ts: &[Term]) -> Json {
    Json::Arr(ts.iter().map(term_json).collect())
}

/// A fighter's A (`attack`) or Dv as the screens read it (DESIGN `Stat`): `total = base x role x
/// (1 + terms)`, so `base_x100` is the strength before its role's percent. The engine's
/// [`Stat::base_x100`] has the role in it (`engage_base_x100`), so the view takes it out; a unit
/// type with a fixed strength (`iAttack` / `iDefense`) takes no role percent on it, and shows its
/// role's percent as 100.
pub fn stat_json(d: &GameData, unit_type: UnitType, attack: bool, s: &Stat) -> Json {
    let info = &d.units[unit_type];
    let fixed = if attack { info.attack_override } else { info.defense_override } > 0;
    let (base, role) = if fixed || s.role_pct <= 0 { (s.base_x100, if fixed { 100 } else { s.role_pct }) } else { (s.base_x100 * 100 / s.role_pct, s.role_pct) };
    crate::jobj! { "base_x100" => base, "role_pct" => role, "terms" => terms_json(&s.terms), "total_x100" => s.total_x100 }
}

fn slot_change_json(c: &SlotChange) -> Json {
    let j = |src: &str| crate::jobj! { "src" => src, "delta" => c.delta };
    match &c.src {
        SlotSource::Terrain(t) => j("Terrain").with("terrain", Json::idx(*t)),
        SlotSource::Feature(f) => j("Feature").with("feature", Json::idx(*f)),
        SlotSource::Hills => j("Hills"),
        SlotSource::River => j("River"),
        SlotSource::CityDefense { pct, next_at } => j("CityDefense").with("pct", *pct).with("next_at", *next_at),
        SlotSource::Bombarded { pct } => j("Bombarded").with("pct", *pct),
        SlotSource::Minimum => j("Minimum"),
        SlotSource::Summons(n) => j("Summons").with("n", *n),
    }
}

pub fn side_slots_json(s: &SideSlots) -> Json {
    crate::jobj! {
        "base" => s.base,
        "changes" => Json::Arr(s.changes.iter().map(slot_change_json).collect()),
        "total" => s.total,
        "river_only" => s.river_only,
    }
}

/// The defended city as it was (its name resolved: the engine keeps its text key).
fn city_snap_json(d: &GameData, c: &CitySnap) -> Json {
    crate::jobj! { "id" => c.id, "name" => names::resolve_keys(d, &c.name).into_owned(), "owner" => c.owner, "size" => c.size, "defense_pct" => c.defense_pct }
}

fn plot_snap_json(d: &GameData, p: &PlotSnap) -> Json {
    crate::jobj! {
        "x" => p.x,
        "y" => p.y,
        "terrain" => Json::opt_idx(p.terrain),
        "feature" => Json::opt_idx(p.feature),
        "hills" => p.hills,
        "river_crossing" => p.river_crossing,
        "city" => p.city.as_ref().map(|c| city_snap_json(d, c)),
    }
}

/// A plot as it is now, in the log's `PlotSnap` shape (the attacking plot; the plan's plots), as
/// the viewer may know it (the fog rules, `view::fog`): its terrain, feature and hills once
/// revealed (else null and false), a city the viewer has revealed, its size and defence while
/// the viewer sees its plot.
fn plot_now(v: &Viewer, p: PlotIdx, river_crossing: bool) -> Json {
    let g = v.g;
    let plot = &g.map.plots[p];
    let (x, y) = g.map.xy(p);
    let revealed = v.plot_revealed(p);
    let city = plot.plot_city.filter(|&c| g.cities.contains(c) && v.city_revealed(c)).map(|c| {
        let city = &g.cities[c];
        let seen = v.city_visible(c);
        crate::jobj! {
            "id" => c,
            "name" => city_name(g, c),
            "owner" => city.owner,
            "size" => seen.then_some(city.population),
            "defense_pct" => seen.then(|| g.city_defense_modifier(c, false)),
        }
    });
    crate::jobj! {
        "x" => x,
        "y" => y,
        "terrain" => Json::opt_idx(plot.terrain_type.filter(|_| revealed)),
        "feature" => Json::opt_idx(plot.feature_type.filter(|_| revealed)),
        "hills" => revealed && g.plot_is_hills(p),
        "river_crossing" => river_crossing,
        "city" => city,
    }
}

// ---------------------------------------------------------------------------------------------
// Fighters and pairs

/// A unit as a fighter of an attack from `ctx.from` on `ctx.to` (the plan's): the battle log's
/// fighter as the engagement writes it (`engagement::sink`), at the unit's HP now, in cell
/// `place`.
fn plan_fighter(g: &Game, u: UnitId, side: Side, place: Option<(u8, u8)>, in_city: bool, viewer_team: TeamId, ctx: &HitCtx) -> LogFighter {
    let d = g.d;
    let unit = &g.units[u];
    let role = g.unit_combat_role(u);
    let max_hp = g.unit_engage_max_hp(u);
    let withdraw_pct = match side {
        Side::Attacker => g.unit_withdrawal_probability(u),
        Side::Defender if in_city => 0,
        Side::Defender => g.unit_get_withdrawl_prob_defensive(u),
    };
    let immune = Immunities::default().with(Immunities::DEF_STRIKE, g.unit_is_immune_to_defensive_strike(u)).with(Immunities::FEAR, g.unit_is_immune_to_fear(u));
    let hp = ffh_core::engagement::hp_from_damage(unit.damage, max_hp);
    let first_strikes = g.unit_first_strikes(u);
    LogFighter {
        unit: u,
        unit_type: unit.unit_type,
        name: unit.name.clone(),
        owner: unit.owner,
        shown_owner: g.unit_get_visual_owner(u, Some(viewer_team)),
        side,
        place,
        large: g.unit_engage_large(u),
        level: unit.level,
        xp: unit.experience,
        hero: g.unit_is_hero(u),
        role: role.unwrap_or(CombatRoleType::from_usize(0)),
        max_hp,
        hp_start: hp,
        hp_battle: hp,
        speed: g.unit_engage_speed(u) + d.defines.engagement_first_strike_speed * first_strikes.max(0),
        first_strikes,
        chance_first_strikes: g.unit_chance_first_strikes(u),
        rolled_first_strikes: 0,
        actions: g.unit_engage_actions(u),
        spell: g.unit_engage_spell(u),
        dodge: g.unit_engage_dodge(u),
        crit_bonus: g.unit_engage_crit_bonus(u),
        anti_flank: role.is_some_and(|r| d.combat_roles[r].anti_flank),
        flanker: g.unit_engage_flanks(u),
        withdraw_pct,
        combat_limit: if side == Side::Attacker { unit.combat_limit } else { 100 },
        collateral_limit: (g.unit_collateral_damage(u) > 0).then(|| g.unit_collateral_damage_limit(u)),
        def_strike: (unit.defensive_strike_chance > 0).then_some((unit.defensive_strike_chance, unit.defensive_strike_damage)),
        immune,
        fear: g.unit_is_fear(u),
        a: g.engage_stat(u, None, side, true, ctx),
        dv: g.engage_stat(u, None, side, false, ctx),
        fate: None,
        dealt: 0,
        taken: 0,
        healed: 0,
    }
}

/// A fighter's promotions: the unit's while it lives (with the type it fought as), else those
/// among its A and Dv parts (a fallen unit's are not kept).
fn fighter_promotions(g: &Game, f: &LogFighter) -> Vec<PromotionType> {
    match g.units.get(f.unit).filter(|u| u.unit_type == f.unit_type && u.owner == f.owner) {
        Some(u) => g.d.promotions.keys().filter(|&p| u.has_promotion[p]).collect(),
        None => {
            let mut out: Vec<PromotionType> = Vec::new();
            for t in f.a.terms.iter().chain(&f.dv.terms) {
                if let TermSource::Promotion(p) = t.src {
                    if !out.contains(&p) {
                        out.push(p);
                    }
                }
            }
            out
        }
    }
}

/// A fighter as the screens read it (`b` its index); `owner` the owner to show the viewer.
fn fighter_json(g: &Game, b: usize, f: &LogFighter, owner: PlayerId) -> Json {
    let d = g.d;
    let name = if f.name.is_empty() { names::display(d, f.unit_type).into_owned() } else { names::text(d, &f.name).to_string() };
    let immune: Vec<Json> = [(Immunities::DEF_STRIKE, "DefStrike"), (Immunities::FEAR, "Fear")].into_iter().filter(|&(bit, _)| f.immune.has(bit)).map(|(_, n)| Json::from(n)).collect();
    crate::jobj! {
        "b" => b,
        "unit" => f.unit,
        "unit_type" => Json::idx(f.unit_type),
        "name" => name,
        "owner" => owner,
        "side" => side_ix(f.side),
        "lane" => f.place.map(|p| p.0),
        "row" => f.place.map(|p| p.1),
        "large" => f.large,
        "level" => f.level,
        "xp" => f.xp,
        "hero" => f.hero,
        "role" => Json::idx(f.role),
        "promotions" => Json::Arr(fighter_promotions(g, f).into_iter().map(Json::idx).collect()),
        "max_hp" => f.max_hp,
        "hp_start" => f.hp_start,
        "hp_battle" => f.hp_battle,
        "speed" => f.speed,
        "first_strikes" => f.first_strikes,
        "chance_first_strikes" => f.chance_first_strikes,
        "rolled_first_strikes" => f.rolled_first_strikes,
        "actions" => actions_json(&f.actions),
        "spell" => Json::opt_idx(f.spell),
        "dodge" => f.dodge,
        "crit_bonus" => f.crit_bonus,
        "anti_flank" => f.anti_flank,
        "flanker" => f.flanker,
        "withdraw_pct" => f.withdraw_pct,
        "combat_limit" => f.combat_limit,
        "collateral_limit" => f.collateral_limit,
        "def_strike" => f.def_strike.map(|(c, dmg)| crate::jarr![c, dmg]),
        "immune" => Json::Arr(immune),
        "fear" => f.fear,
        "a" => stat_json(d, f.unit_type, true, &f.a),
        "dv" => stat_json(d, f.unit_type, false, &f.dv),
        "fate" => f.fate.as_ref().map(fate_json),
        "dealt" => f.dealt,
        "taken" => f.taken,
        "healed" => f.healed,
    }
}

fn pair_json(p: &LogPair) -> Json {
    crate::jobj! {
        "h" => p.h,
        "t" => p.t,
        "a_x100" => p.a_x100,
        "dv_x100" => p.dv_x100,
        "mean" => p.mean,
        "a_extra" => terms_json(&p.a_extra),
        "dv_extra" => terms_json(&p.dv_extra),
    }
}

/// `terms` less the terms of `headline` (a multiset difference): the pair-only terms (as the
/// engine's log has them).
fn extra_terms(terms: Vec<Term>, headline: &[Term]) -> Vec<Term> {
    let mut left: Vec<&Term> = headline.iter().collect();
    terms
        .into_iter()
        .filter(|t| match left.iter().position(|h| *h == t) {
            Some(i) => {
                left.swap_remove(i);
                false
            }
            None => t.pct != 0,
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Events

fn heal_source_json(s: &HealSource) -> Json {
    match s {
        HealSource::Medic { b, unit_type, adjacent } => crate::jobj! { "kind" => "Medic", "b" => *b, "unit_type" => Json::idx(*unit_type), "adjacent" => *adjacent },
        HealSource::OnKill => crate::jobj! { "kind" => "OnKill" },
        HealSource::Spell(s) => crate::jobj! { "kind" => "Spell", "spell" => Json::idx(*s) },
    }
}

fn xp_source_json(s: &XpSource) -> Json {
    match s {
        XpSource::Kill { victim, share_permille } => crate::jobj! { "kind" => "Kill", "victim" => *victim, "share_permille" => *share_permille },
        XpSource::Withdrew => crate::jobj! { "kind" => "Withdrew" },
        XpSource::DefStrike => crate::jobj! { "kind" => "DefStrike" },
    }
}

fn kill_kind_json(k: &KillKind) -> Json {
    match k {
        KillKind::Capture(t) => crate::jobj! { "kind" => "Capture", "unit_type" => Json::idx(*t) },
        KillKind::Enslave(t) => crate::jobj! { "kind" => "Enslave", "unit_type" => Json::idx(*t) },
        KillKind::Convert(t) => crate::jobj! { "kind" => "Convert", "unit_type" => Json::idx(*t) },
        KillKind::Create(t) => crate::jobj! { "kind" => "Create", "unit_type" => Json::idx(*t) },
        KillKind::Gold(n) => crate::jobj! { "kind" => "Gold", "amount" => *n },
        KillKind::Afflict(p) => crate::jobj! { "kind" => "Afflict", "promotion" => Json::idx(*p) },
        KillKind::Promotion(p) => crate::jobj! { "kind" => "Promotion", "promotion" => Json::idx(*p) },
    }
}

fn cast_effect_json(e: &CastEffect) -> Json {
    match e {
        CastEffect::Mod { a_pct, dv_pct } => crate::jobj! { "kind" => "Mod", "a_pct" => *a_pct, "dv_pct" => *dv_pct },
        CastEffect::Damage(hits) => crate::jobj! { "kind" => "Damage", "hits" => Json::Arr(hits.iter().map(|&(b, dmg)| crate::jarr![b, dmg]).collect()) },
        CastEffect::Summon(b) => crate::jobj! { "kind" => "Summon", "b" => *b },
    }
}

fn idxs(v: &[FighterIdx]) -> Json {
    Json::Arr(v.iter().map(|&b| Json::from(b)).collect())
}

/// An action of the battle (module comment).
fn action_json(e: &Event) -> Json {
    let hits = e.hits.iter().map(|h| {
        crate::jobj! {
            "target" => h.target,
            "dmg" => h.dmg,
            "expected" => h.expected,
            "r" => h.r,
            "crit" => h.crit,
            "miss" => h.miss,
            "killed" => h.killed,
            "withdrew" => h.withdrew,
            "splash" => h.splash,
        }
    });
    crate::jobj! {
        "kind" => "Action",
        "round" => e.round,
        "pass" => e.pass,
        "actor" => e.actor,
        "action" => e.kind.token(),
        "spell" => e.spell.map(|s| Json::idx(SpellType::from_usize(s as usize))),
        "target" => e.target,
        "hits" => Json::Arr(hits.collect()),
        "heal" => e.heal.map(|h| crate::jobj! { "target" => h.target, "amount" => h.amount, "r" => h.r }),
        "knocked" => e.knocked.map(|k| crate::jobj! { "unit" => k.unit, "from" => k.from, "to" => k.to }),
        "note" => e.note.map(|n| match n {
            Note::NoTarget => "NoTarget",
            Note::NoOneHurt => "NoOneHurt",
        }),
    }
}

/// An event of the log, `{kind, ...}` (module comment).
fn event_json(g: &Game, e: &LogEvent) -> Json {
    let k = |kind: &str| crate::jobj! { "kind" => kind };
    match e {
        LogEvent::Stage { stage } => k("Phase").with(
            "phase",
            match stage {
                Stage::PreBattle => "PreBattle",
                Stage::Morale => "Morale",
                Stage::Opening => "Opening",
                Stage::Battle => "Battle",
                Stage::Post => "Post",
            },
        ),
        LogEvent::Cast { caster, spell, targets, effect } => k("Cast").with("caster", *caster).with("spell", Json::idx(*spell)).with("targets", idxs(targets)).with("effect", cast_effect_json(effect)),
        LogEvent::Morale { unit, source, chance, roll, passed } => k("Morale").with("unit", *unit).with("source", *source).with("chance", *chance).with("roll", *roll).with("passed", *passed),
        LogEvent::SatOut { unit, why } => k("SatOut").with("unit", *unit).with(
            "why",
            match why {
                SatOutWhy::Afraid => "Afraid",
            },
        ),
        LogEvent::DefStrike { striker, target, rolled, chance, roll, pct, dmg } => {
            k("DefStrike").with("striker", *striker).with("target", *target).with("rolled", *rolled).with("chance", *chance).with("roll", *roll).with("pct", *pct).with("dmg", *dmg)
        }
        LogEvent::Action(a) => action_json(a),
        LogEvent::End { rounds, down } => k("End").with("rounds", *rounds).with("down", crate::jarr![down[0], down[1]]),
        LogEvent::Heal { unit, amount, source } => k("Heal").with("unit", *unit).with("amount", *amount).with("source", heal_source_json(source)),
        LogEvent::Xp { unit, amount, from } => k("Xp").with("unit", *unit).with("amount", *amount).with("from", Json::Arr(from.iter().map(xp_source_json).collect())),
        LogEvent::KillEffect { killer, victim, kind } => k("KillEffect").with("killer", *killer).with("victim", *victim).with("effect", kill_kind_json(kind)),
        LogEvent::Flee { unit, to } => k("Flee").with("unit", *unit).with("to", plot_idx_json(g, *to)),
        LogEvent::MoveIn { units, city } => k("MoveIn").with("units", idxs(units)).with("city", *city),
    }
}

fn forecast_json(f: &FrozenForecast) -> Json {
    crate::jobj! {
        "runs" => f.runs,
        "wiped_permille" => f.wiped_permille,
        "falls_permille" => f.falls_permille,
        "repulsed_permille" => f.repulsed_permille,
        "lost_x100" => crate::jarr![f.lost_x100[0], f.lost_x100[1]],
        "per" => Json::Arr(f.per.iter().map(|p| crate::jobj! { "b" => p.b, "fall_pct" => p.fall_pct, "escape_pct" => p.escape_pct }).collect()),
    }
}

// ---------------------------------------------------------------------------------------------
// Sides and summaries

/// Whether `v` may see report `r`: it fought on a side (or debug reveal).
fn visible_to(v: &Viewer, r: &BattleReport) -> bool {
    v.reveal || r.involves(v.h)
}

/// The side the viewer fought on (`None`: neither, a reveal view).
fn viewer_side(v: &Viewer, r: &BattleReport) -> Option<Side> {
    r.side_of(v.h)
}

/// Side `s`'s player as the viewer may see it: the real one on its own side (or under reveal),
/// the one it saw on the other.
fn shown_player(v: &Viewer, r: &BattleReport, s: Side) -> PlayerId {
    let side = &r.sides[side_ix(s)];
    if v.reveal || viewer_side(v, r) == Some(s) {
        side.player
    } else {
        side.shown_player
    }
}

/// A side's name for the viewer: the defended city's (the defender of a city), "Your force",
/// else "Calabim force".
fn side_label(v: &Viewer, r: &BattleReport, s: Side, player: PlayerId) -> String {
    let g = v.g;
    if s == Side::Defender {
        if let Some(c) = r.place.city.as_ref().filter(|c| c.owner == player) {
            return names::resolve_keys(g.d, &c.name).into_owned();
        }
    }
    let own = v.reveal || viewer_side(v, r) == Some(s);
    if own && player == v.h {
        "Your force".into()
    } else {
        format!("{} force", g.player_get_civilization_adjective(player))
    }
}

/// Report `r`'s summary as the viewer may see it (the battle lists, the toasts, the command's
/// reply): where, when, the sides (player, civilization, label, slots, lanes, committed, sat out,
/// fallen by type, escaped, standing), the outcome, and whether its log is kept.
pub fn summary_json(v: &Viewer, r: &BattleReport) -> Json {
    let g = v.g;
    let me = viewer_side(v, r);
    let sides = [Side::Attacker, Side::Defender].map(|s| {
        let side = &r.sides[side_ix(s)];
        let player = shown_player(v, r, s);
        crate::jobj! {
            "player" => player,
            "civ" => Json::opt_idx(g.player_get_civilization_type(player)),
            "label" => side_label(v, r, s, player),
            "slots" => side.slots,
            "lanes" => side.lanes,
            "used" => side.used,
            "committed" => side.committed,
            "sat_out" => side.sat_out,
            "fallen" => side.fallen,
            "lost_types" => Json::Arr(side.lost.iter().map(|&(t, n)| crate::jarr![Json::idx(t), n]).collect()),
            "escaped" => side.escaped,
            "standing" => side.standing,
        }
    });
    let (fx, fy) = g.map.xy(r.from);
    crate::jobj! {
        "id" => r.id,
        "turn" => r.turn,
        "x" => r.place.x,
        "y" => r.place.y,
        "from" => plot_json(fx, fy),
        "place" => r.place.city.as_ref().map(|c| names::resolve_keys(g.d, &c.name).into_owned()),
        "city" => r.place.city.as_ref().map(|c| c.id),
        "viewer_side" => me.map(side_ix),
        "result" => result_name(r.outcome.result),
        "rounds" => r.outcome.rounds,
        "plot_taken" => r.outcome.plot_taken,
        "city_captured" => r.outcome.city_captured,
        "defenders_left" => r.outcome.defenders_left,
        "has_log" => r.log.is_some(),
        "sides" => Json::Arr(sides.into_iter().collect()),
    }
}

/// [`summary_json`] of report `id`, or null when it is not kept or not the viewer's.
pub fn summary_by_id(v: &Viewer, id: u32) -> Json {
    v.g.battle_report(id).filter(|r| visible_to(v, r)).map_or(Json::Null, |r| summary_json(v, r))
}

/// The viewer's kept reports from game turn `from` on, newest first.
pub fn reports_since<'g>(v: &Viewer<'g>, from: i32) -> Vec<&'g BattleReport> {
    let g: &'g Game = v.g;
    g.battle_reports.iter().rev().filter(|r| r.turn >= from && visible_to(v, r)).collect()
}

/// The viewer's kept reports from game turn `from` on, newest first (summaries).
pub fn summaries_since(v: &Viewer, from: i32) -> Vec<Json> {
    reports_since(v, from).into_iter().map(|r| summary_json(v, r)).collect()
}

/// One line for people (the turn report's, `text`): "battle #12 at Strages: held; 2 fell, 5 of
/// theirs fell".
pub fn report_line(v: &Viewer, r: &BattleReport) -> String {
    let g = v.g;
    let place = r.place.city.as_ref().map_or_else(|| format!("({}, {})", r.place.x, r.place.y), |c| names::resolve_keys(g.d, &c.name).into_owned());
    let me = viewer_side(v, r).unwrap_or(Side::Attacker);
    let outcome = match (me, r.outcome.result) {
        (Side::Attacker, EngageResult::Taken) if r.outcome.city_captured.is_some() => "the city taken",
        (Side::Attacker, EngageResult::Taken) => "the plot taken",
        (Side::Attacker, EngageResult::Cleared) => "their line broken, the plot holds",
        (Side::Attacker, EngageResult::Repulsed) => "repulsed",
        (Side::Attacker, EngageResult::Held) => "their line held",
        (Side::Defender, EngageResult::Taken) => "overrun",
        (Side::Defender, EngageResult::Cleared) => "our line broken, the plot held",
        (Side::Defender, EngageResult::Repulsed) => "they were driven off",
        (Side::Defender, EngageResult::Held) => "held",
    };
    let (mine, theirs) = (&r.sides[side_ix(me)], &r.sides[side_ix(me.other())]);
    format!("battle #{} at {place}: {outcome}; {} fell, {} of theirs fell", r.id, mine.fallen, theirs.fallen)
}

/// `battle_reports`: the viewer's kept reports, newest first (`[]` with engagements off).
pub fn battle_reports(v: &Viewer) -> Json {
    crate::jobj! { "on" => v.g.engagements_on(), "reports" => Json::Arr(summaries_since(v, i32::MIN)) }
}

/// The view's `battles`: the viewer's reports of this game turn and the previous one (null with
/// engagements off): what the map's markers, the toasts and the combat flash read.
pub fn recent(v: &Viewer) -> Json {
    if !v.g.engagements_on() {
        return Json::Null;
    }
    Json::Arr(summaries_since(v, v.g.game_turn() - 1))
}

// ---------------------------------------------------------------------------------------------
// The battle log

/// `battle_log {id}`: report `id` with its log as the battle screens read it (module comment).
/// `gone` when the report is not kept, is not the viewer's, or has lost its log (older reports
/// keep their summary only: `battle_reports` says `has_log`).
pub fn battle_log(v: &Viewer, id: u32) -> Result<Json, QueryErr> {
    let g = v.g;
    let r = g.battle_report(id).filter(|r| visible_to(v, r)).ok_or(QueryErr::Gone)?;
    let log = r.log.as_deref().ok_or(QueryErr::Gone)?;
    let me = viewer_side(v, r);
    let sides = [Side::Attacker, Side::Defender].map(|s| {
        let side = &r.sides[side_ix(s)];
        let player = shown_player(v, r, s);
        crate::jobj! {
            "player" => player,
            "shown_player" => side.shown_player,
            "civ" => Json::opt_idx(g.player_get_civilization_type(player)),
            "label" => side_label(v, r, s, player),
            "lanes" => log.lanes[side_ix(s)],
            "ordered" => log.ordered[side_ix(s)],
        }
    });
    let fighters: Vec<Json> = log.fighters.iter().enumerate().map(|(b, f)| fighter_json(g, b, f, if v.reveal { f.owner } else { r.owner_for(v.h, f.side, f.owner, f.shown_owner) })).collect();
    let moved_in = log.events.iter().find_map(|e| match e {
        LogEvent::MoveIn { units, .. } => Some(idxs(units)),
        _ => None,
    });
    let o = &r.outcome;
    Ok(crate::jobj! {
        "v" => LOG_VERSION,
        "id" => r.id,
        "turn" => r.turn,
        "viewer_side" => me.map(side_ix),
        "plot" => plot_snap_json(g.d, &r.place),
        "from" => plot_now(v, r.from, false),
        "ground" => ground_json(&log.ground, &log.rules),
        "slots" => Json::Arr(log.slots.iter().map(side_slots_json).collect()),
        "lanes" => crate::jarr![log.lanes[0], log.lanes[1]],
        "sides" => Json::Arr(sides.into_iter().collect()),
        "fighters" => Json::Arr(fighters),
        "pairs" => Json::Arr(log.pairs.iter().map(pair_json).collect()),
        "events" => Json::Arr(log.events.iter().map(|e| event_json(g, e)).collect()),
        "forecast" => log.forecast.as_ref().map(forecast_json),
        "outcome" => crate::jobj! {
            "result" => result_name(o.result),
            "rounds" => o.rounds,
            "plot_taken" => o.plot_taken,
            "city_captured" => o.city_captured,
            "moved_in" => moved_in.unwrap_or_else(Json::arr),
            "defenders_left" => o.defenders_left,
        },
        "summary" => summary_json(v, r),
    })
}

// ---------------------------------------------------------------------------------------------
// The plan of an attack

/// The attacking plot of `by`.
fn attack_origin(v: &Viewer, by: &EngageBy) -> Result<PlotIdx, QueryErr> {
    let g = v.g;
    match by {
        EngageBy::Army(a) => match g.armies.get(*a) {
            None => Err(QueryErr::Gone),
            Some(army) if army.owner != v.h => Err(QueryErr::NotOwner),
            Some(_) => g.army_plot(*a).ok_or_else(|| invalid("the army is not on the map")),
        },
        EngageBy::Units(units) => {
            crate::view::units::check_selection(v, units)?;
            Ok(g.unit_plot(units[0]))
        }
        EngageBy::From(x, y) => crate::view::map::plot_at(v, *x, *y),
    }
}

/// `engage_preview`: the plan of an attack on `to` by `by` in `formation` (else the rules' auto
/// formation; module comment). Pure. A plot the viewer does not see, or one not next to the
/// attackers, has no plan: the reply is its `errors` (and `declare_war`) alone, so the query tells
/// no more of the fog than the `combat` query does (G5). A refused plan (an invalid formation,
/// nobody to fight) says why in `errors`.
pub fn engage_preview(v: &Viewer, by: &EngageBy, to: (i32, i32), formation: Option<&[Place]>) -> Result<Json, QueryErr> {
    let g = v.g;
    let h = v.h;
    if !g.engagements_on() {
        return Err(invalid("engagements are off in this game"));
    }
    let from = attack_origin(v, by)?;
    let dest = crate::view::map::plot_at(v, to.0, to.1)?;
    let mut errors: Vec<String> = Vec::new();
    let ((fx, fy), (tx, ty)) = (g.map.xy(from), g.map.xy(dest));
    if g.step_distance(fx, fy, tx, ty) != 1 {
        errors.push("an attack is on a plot next to the attackers".into());
    }
    if !v.plot_visible(dest) {
        errors.push("you cannot see that plot".into());
    }
    let rival = g.map.plots[from].units.iter().filter(|&&u| g.units[u].owner == h).find_map(|&u| g.unit_get_declare_war_move(u, dest));
    let err_reply = |errors: Vec<String>| crate::jobj! { "declare_war" => rival, "errors" => Json::Arr(errors.into_iter().map(Json::from).collect()) };
    if !errors.is_empty() {
        return Ok(err_reply(errors));
    }
    let war = rival.map_or(WarTest::AtWar, WarTest::DeclareWar);
    let inp = PreviewInput { player: h, from, to: dest, formation, war };
    let pv = match g.engage_preview(&inp) {
        Ok(pv) => pv,
        Err(e) => return Ok(err_reply(vec![e.to_string()])),
    };
    let ctx = HitCtx::new(from, dest);
    let in_city = g.plot_is_city(dest, false, None);

    // The fighters: the pool (side 0), then the defenders seen (side 1), each in its cell when
    // its formation places it.
    let cell = |f: &[Place], u: UnitId| f.iter().find(|p| p.unit == u).map(|p| (p.lane, p.row));
    let mut units: Vec<(UnitId, Side)> = pv.pool.iter().map(|&u| (u, Side::Attacker)).collect();
    units.extend(pv.defenders.iter().map(|&u| (u, Side::Defender)));
    let fighters: Vec<LogFighter> = units
        .iter()
        .map(|&(u, side)| {
            let place = if side == Side::Attacker { cell(&pv.formation, u) } else { cell(&pv.def_formation, u) };
            plan_fighter(g, u, side, place, in_city, v.team, &ctx)
        })
        .collect();
    let b_of = |u: UnitId, side: Side| units.iter().position(|&x| x == (u, side));
    let bs = |list: &[UnitId], side: Side| Json::Arr(list.iter().filter_map(|&u| b_of(u, side)).map(Json::from).collect());
    // Every pair both ways (the cards' damage per hit).
    let base = g.d.defines.engagement_base_damage.max(0) as i64 * 100;
    let mut pairs = Vec::new();
    for (ai, &(a, _)) in units.iter().enumerate().filter(|(_, x)| x.1 == Side::Attacker) {
        for (di, &(dfd, _)) in units.iter().enumerate().filter(|(_, x)| x.1 == Side::Defender) {
            for (h_i, hu, t_i, tu, side) in [(ai, a, di, dfd, Side::Attacker), (di, dfd, ai, a, Side::Defender)] {
                let pr = g.engage_hit(hu, tu, side, &ctx);
                let sa = g.engage_stat(hu, Some(tu), side, true, &ctx);
                let sd = g.engage_stat(tu, Some(hu), side.other(), false, &ctx);
                let mean = (base * pr.a.max(1) as i64 / (pr.a.max(1) as i64 + pr.dv.max(1) as i64)) as i32;
                pairs.push(pair_json(&LogPair {
                    h: h_i as FighterIdx,
                    t: t_i as FighterIdx,
                    a_x100: pr.a,
                    dv_x100: pr.dv,
                    mean,
                    a_extra: extra_terms(sa.terms, &fighters[h_i].a.terms),
                    dv_extra: extra_terms(sd.terms, &fighters[t_i].dv.terms),
                }));
            }
        }
    }
    let fighters_json: Vec<Json> = fighters
        .iter()
        .enumerate()
        .map(|(b, f)| fighter_json(g, b, f, if f.side == Side::Attacker || v.reveal { f.owner } else { v.unit_owner(f.unit) }))
        .collect();
    let def_player = match g.map.plots[dest].plot_city.filter(|&c| g.cities.contains(c) && v.city_revealed(c)) {
        Some(c) => g.cities[c].owner,
        None => pv.defenders.first().map_or(h, |&u| v.unit_owner(u)),
    };
    let def_label = match g.map.plots[dest].plot_city.filter(|&c| g.cities.contains(c) && v.city_revealed(c)) {
        Some(c) => city_name(g, c),
        None => format!("{} force", g.player_get_civilization_adjective(def_player)),
    };
    let sides = crate::jarr![
        crate::jobj! { "player" => h, "civ" => Json::opt_idx(g.player_get_civilization_type(h)), "label" => "Your force", "lanes" => pv.lanes[0] },
        crate::jobj! { "player" => def_player, "civ" => Json::opt_idx(g.player_get_civilization_type(def_player)), "label" => def_label, "lanes" => pv.lanes[1] },
    ];
    // The forecast, its fighters by index.
    let fc = &pv.forecast;
    let per: Vec<Json> = fc
        .per
        .iter()
        .filter_map(|p| Some(crate::jobj! { "b" => b_of(p.unit, p.side)?, "fall_pct" => p.fall_pct, "escape_pct" => p.escape_pct }))
        .collect();
    let forecast = crate::jobj! {
        "runs" => fc.runs,
        "wiped_permille" => fc.wiped_permille,
        "falls_permille" => fc.falls_permille,
        "repulsed_permille" => fc.repulsed_permille,
        "lost_x100" => crate::jarr![fc.lost_x100[0], fc.lost_x100[1]],
        "escaped_x100" => crate::jarr![fc.escaped_x100[0], fc.escaped_x100[1]],
        "per" => Json::Arr(per),
    };
    let crossing = g.plot_is_river_crossing(from, g.direction_xy_plots(from, dest));
    Ok(crate::jobj! {
        "plot" => plot_now(v, dest, crossing),
        "from" => plot_now(v, from, false),
        "ground" => ground_json(&pv.ground, &pv.rules),
        "slots" => Json::Arr(pv.slots.sides.iter().map(side_slots_json).collect()),
        "lanes" => crate::jarr![pv.lanes[0], pv.lanes[1]],
        "sides" => sides,
        "fighters" => Json::Arr(fighters_json),
        "pairs" => Json::Arr(pairs),
        "formation" => formation_json(&pv.formation),
        "auto" => formation_json(&pv.auto),
        "reserve" => bs(&pv.reserve, Side::Attacker),
        "def_formation" => formation_json(&pv.def_formation),
        "def_reserve" => bs(&pv.def_reserve, Side::Defender),
        "forecast" => forecast,
        "declare_war" => rival,
        "errors" => Json::arr(),
    })
}

// ---------------------------------------------------------------------------------------------
// The `combat` query and the view's fields under engagements

/// The `combat` query under engagements, for an attack of `units` (land units) on `plot` (land),
/// given the duel's reply `j`: when the plot holds a defender the attack would meet (a
/// declare-war attack tests the teams it would fight), `engagement: true`, `pool` and
/// `defenders` (their sizes), `declare_war_team` and `on_arrival`:
/// - an attack the selection can launch now (the plot is next to it and some unit on its plot
///   can attack the plot now: [`Game::engage_attackers`]): `on_arrival: false`, `valid: true`,
///   `pool` the pool on the selection's plot (its other units there count: the garrison, other
///   armies);
/// - else (a go-to that ends in an attack, units that have used their moves or their attack):
///   `on_arrival: true`, the duel's `valid` and odds kept, `pool` the selected units that could
///   fight there once they can attack it ([`could_engage`]); the battle is fought from the plot
///   the attack is launched from, with whatever stands there then.
///
/// The duel's odds and strengths stay beside them (the plan's forecast is `engage_preview`'s).
/// With no defender there the attack is today's (it captures non-combatants): the duel's reply
/// unchanged.
pub fn combat_fields(v: &Viewer, units: &[UnitId], plot: PlotIdx, mut j: Json) -> Json {
    let g = v.g;
    if !v.plot_visible(plot) {
        return j;
    }
    let from = g.unit_plot(units[0]);
    let ((fx, fy), (tx, ty)) = (g.map.xy(from), g.map.xy(plot));
    let rival = units.iter().find_map(|&u| g.unit_get_declare_war_move(u, plot));
    let war = rival.map_or(WarTest::AtWar, WarTest::DeclareWar);
    let pool = if g.step_distance(fx, fy, tx, ty) == 1 { g.engage_attackers(v.h, from, plot, war) } else { Vec::new() };
    let now = !pool.is_empty();
    let force: Vec<UnitId> = if now { pool } else { units.iter().copied().filter(|&u| could_engage(g, u, plot)).collect() };
    let defenders = g.engage_defenders(plot, v.h, if force.is_empty() { units } else { &force }, war);
    if defenders.is_empty() {
        return j;
    }
    j.set("engagement", true);
    if now {
        j.set("valid", true);
    }
    j.set("on_arrival", !now);
    j.set("pool", force.len());
    j.set("defenders", defenders.len());
    j.set("declare_war_team", rival);
    j
}

/// Whether unit `u` could fight in an attack on `plot` once it is next to it and can attack:
/// [`Game::unit_can_engage`]'s tests but the place, the moves and the attacks left (a land unit
/// on a land plot, able to attack, not marching, not a missile).
fn could_engage(g: &Game, u: UnitId, plot: PlotIdx) -> bool {
    g.engage_applies(u, plot) && g.units[u].march.is_none() && !g.unit_is_suicide(u) && g.unit_can_attack(u)
}

/// A unit's engagement fields (the view's units, under engagements): its role, its A before the
/// bonuses of a plot it attacks (its role and own percentages: as it hits back), its Dv where it
/// stands, both x100, its max HP, its speed before first strikes, what it does from each row, its
/// dodge and crit bonus, and whether it is large.
pub fn unit_fields(g: &Game, u: UnitId, j: &mut Json) {
    let Some(role) = g.unit_combat_role(u) else { return };
    let Some(at) = g.unit_plot_opt(u) else { return };
    let ctx = HitCtx { to: at, from: None, city_defense: None };
    j.set("engage_role", Json::idx(role));
    j.set("engage_attack", g.engage_stat(u, None, Side::Defender, true, &ctx).total_x100);
    j.set("engage_defense", g.engage_job_defense(u, &ctx));
    j.set("engage_max_hp", g.unit_engage_max_hp(u));
    j.set("engage_speed", g.unit_engage_speed(u));
    j.set("engage_actions", actions_json(&g.unit_engage_actions(u)));
    j.set("engage_dodge", g.unit_engage_dodge(u));
    j.set("engage_crit_bonus", g.unit_engage_crit_bonus(u));
    j.set("engage_large", g.unit_engage_large(u));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_term_source_has_a_name() {
        let srcs = [
            TermSource::Own,
            TermSource::Promotion(PromotionType::from_usize(0)),
            TermSource::Hills,
            TermSource::VsUnitCombat(UnitCombatType::from_usize(0)),
            TermSource::Improvement(ImprovementType::from_usize(0)),
            TermSource::RangeDefense,
        ];
        for s in srcs {
            let j = term_json(&Term { src: s, pct: 5 });
            assert!(j.get("src").and_then(Json::as_str).is_some_and(|n| !n.is_empty()));
            assert_eq!(j.get("pct").and_then(Json::as_i64), Some(5));
        }
    }

    /// The screens' `base_x100` is the strength before the role's percent (the engine's has it in).
    #[test]
    fn a_stat_shows_the_strength_before_its_role() {
        let d = ffh_data::global();
        let horseman = d.t::<UnitType>("UNIT_HORSEMAN");
        let j = stat_json(d, horseman, true, &Stat { base_x100: 440, role_pct: 110, terms: Vec::new(), total_x100: 440 });
        assert_eq!(j.get("base_x100").and_then(Json::as_i64), Some(400));
        assert_eq!(j.get("role_pct").and_then(Json::as_i64), Some(110));
        assert_eq!(j.get("total_x100").and_then(Json::as_i64), Some(440));
    }

    /// A plot the viewer has not revealed tells nothing of itself (the plan's and the log's
    /// plots): no terrain, feature or hills; a revealed one shows its terrain.
    #[test]
    fn an_unrevealed_plot_shows_no_terrain() {
        let d = ffh_data::global();
        let mut g = Game::new(d, ffh_core::setup::GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5));
        g.game_init();
        let h = PlayerId(0);
        let team = g.players[h].team;
        let hidden = g.map.plot_ids().find(|&p| !g.map.plots[p].revealed[team] && g.map.plots[p].terrain_type.is_some()).expect("an unrevealed plot");
        let v = Viewer::new(&g, h, false);
        let j = plot_now(&v, hidden, false);
        assert!(j.get("terrain").is_some_and(Json::is_null), "{j:?}");
        assert!(j.get("feature").is_some_and(Json::is_null), "{j:?}");
        assert_eq!(j.get("hills").and_then(Json::as_bool), Some(false));
        assert!(plot_now(&Viewer::new(&g, h, true), hidden, false).get("terrain").and_then(Json::as_i64).is_some(), "debug reveal shows it");
        g.map.plots[hidden].revealed[team] = true;
        let v = Viewer::new(&g, h, false);
        assert!(plot_now(&v, hidden, false).get("terrain").and_then(Json::as_i64).is_some(), "once revealed");
    }

    /// The help's engagement wording goes to the units and promotions with the abilities it
    /// explains, and only to them.
    #[test]
    fn the_combat_abilities_get_their_engagement_wording() {
        let d = ffh_data::global();
        let unit = |n: &str| crate::view::help::engagement_notes(d, "UnitType", d.t::<UnitType>(n).idx());
        let promo = |n: &str| crate::view::help::engagement_notes(d, "PromotionType", d.t::<PromotionType>(n).idx());
        assert!(unit("UNIT_HORSEMAN").is_some_and(|t| t.contains("Withdrawal is rolled when a hit would kill it")), "{:?}", unit("UNIT_HORSEMAN"));
        assert_eq!(unit("UNIT_WARRIOR"), None);
        assert!(promo("PROMOTION_DRILL1").is_some_and(|t| t.contains("First strikes are speed")));
        assert!(promo("PROMOTION_BLITZ").is_some_and(|t| t.contains("one more battle")));
        assert_eq!(crate::view::help::engagement_notes(d, "TechType", 0), None);
    }
}
