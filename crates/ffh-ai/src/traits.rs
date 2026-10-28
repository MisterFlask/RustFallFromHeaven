//! Leader traits (item `trait-play`): the decisions a leader's traits call for. The rules apply
//! every trait's effect by themselves (free promotions, cheaper civics, more health, ...), and the
//! generic AI already profits from some of them where it reads numbers the trait changes (wonder
//! turns, civic upkeep, upgrade prices). What it did not do is *play* to them; here a leader does:
//!
//! - **Raiders** (pillaged gold +100%, Commando): an army stack at war pillages the improved enemy
//!   plot it stands on (one unit, with moves to spare or while the stack holds), and a stack that
//!   no operation needs raids the enemy improvements it can reach safely ([`after_army`],
//!   [`raid`]). The generic AI never pillages.
//! - **Spiritual** (no anarchy): civics are reconsidered every 2 turns at a gain of 40 instead of
//!   every 10 at 100, and a change of state religion carries no anarchy weight ([`civic_cadence`],
//!   [`religion_stickiness`]).
//! - **Arcane** (adepts gain experience 20% faster) and **Summoner** (summons last a turn longer
//!   and fight with Combat I): a corps of casters, (cities + 2) / 3 (a Summoner's (cities + 2) / 2),
//!   from the most productive safe city, its Mage Guild first ([`production`]); a Summoner's casters favour the spheres that
//!   summon ([`promotion_value`]). A Summoner researches toward its Adepts (Knowledge of the
//!   Ether, [`tech_value`]), values a temporary summon for the extra turn it lasts
//!   ([`summon_turns`]) and raises lasting summons (Raise Skeleton) at peace too, as army units
//!   while the army is short and a unit is affordable ([`peace_summon`]). A Summoner without a
//!   mana whose Mage-level spell summons a creature (Death, Entropy, Law, Ice: Keelyn's Balseraphs
//!   start with Mind, Chaos and Air) goes for one first: its node on our raw mana is worth
//!   [`SUMMON_NODE`] more (`mana::node_value`, which research and the builder's training follow),
//!   it pays more for it in trade, and its casters take the sphere's first promotion toward the
//!   summon ([`summon_mana`], switch `SUMMON_MANA`).
//! - **Philosophical** (great people +100%): the great-people emphasis in its best great-people
//!   city, one more per eight cities ([`turn`]).
//! - **Financial** (+1 commerce on plots with 2 or more): workers count the extra commerce an
//!   improvement brings over the threshold on an unimproved plot a city works ([`threshold_value`]).
//! - **Industrious** (limited wonders +50% production): wonders are worth half again as much
//!   ([`building_value`]).
//! - **Aggressive** (Combat I for its fighters): once its home land is full, the war bar counts
//!   that edge, which the power rating does not ([`attack_bonus_pct`]).
//! - **Ingenuity** (upgrades at half price): upgrades keep half the gold reserve ([`upgrade_reserve`]).
//! - **Tolerant** (a conquered city keeps its civilization's buildings and units): never razes
//!   a conquered city ([`keeps_conquests`]).
//! - **Expansive** (+3 health a city): one more settler at a time ([`extra_settlers`]).
//!
//! Passive, nothing to decide: Charismatic, Creative, Defender, Organized, Magic Resistant
//! (their effects reach every decision that reads the game). Played elsewhere: Adaptive
//! (`events.rs`), Agnostic and Barbarian (`tech_plan.rs`), Fallow (`workers.rs`), Sprawling
//! (`civs/kuriotates.rs`).
//!
//! Measurement switches (read once; the run cache keys runs by every `FFH_*` variable):
//! `FFH_AI_TRAIT_OFF=TRAIT_X,..` (or `X`, or `all`) turns those traits' play off;
//! `FFH_AI_TRAIT_CIVS=CIVILIZATION_X,..` (or `X`) keeps it only for players of those
//! civilizations, so one civilization's leaders can be compared against identical opponents
//! (`civ_ab.py --new-env FFH_AI_TRAIT_CIVS={civ} --base-env FFH_AI_TRAIT_OFF=all`). With every
//! trait off the AI plays exactly as without this module (same checksums).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::production::Choice;
use crate::units::{alive, move_to, push};
use crate::view::{defense_power, View};

/// A trait this module plays.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Trait {
    Raiders,
    Spiritual,
    Arcane,
    Summoner,
    Philosophical,
    Financial,
    Industrious,
    Aggressive,
    Ingenuity,
    Tolerant,
    Expansive,
}

const NAMES: [(&str, Trait); 11] = [
    ("RAIDERS", Trait::Raiders),
    ("SPIRITUAL", Trait::Spiritual),
    ("ARCANE", Trait::Arcane),
    ("SUMMONER", Trait::Summoner),
    ("PHILOSOPHICAL", Trait::Philosophical),
    ("FINANCIAL", Trait::Financial),
    ("INDUSTRIOUS", Trait::Industrious),
    ("AGGRESSIVE", Trait::Aggressive),
    ("INGENUITY", Trait::Ingenuity),
    ("TOLERANT", Trait::Tolerant),
    ("EXPANSIVE", Trait::Expansive),
];

/// Per [`Trait`] (in `NAMES` order): its trait type, `None` when the data lacks it or
/// `FFH_AI_TRAIT_OFF` turns it off ([`crate::switches::env_list`]).
fn types(d: &GameData) -> &'static [Option<TraitType>] {
    static S: std::sync::OnceLock<Vec<Option<TraitType>>> = std::sync::OnceLock::new();
    S.get_or_init(|| {
        let off = crate::switches::env_list("FFH_AI_TRAIT_OFF", "TRAIT_").unwrap_or_default();
        NAMES
            .iter()
            .map(|(name, _)| if crate::switches::listed(&off, name) { None } else { d.ty::<TraitType>(&format!("TRAIT_{name}")) })
            .collect()
    })
}

/// The trait type of `t` if its play is on for player `p` (switches), whether or not `p` has it.
fn enabled_type(g: &Game, p: PlayerId, t: Trait) -> Option<TraitType> {
    let i = NAMES.iter().position(|&(_, x)| x == t)?;
    let tr = types(g.d)[i]?;
    if !crate::switches::TRAIT_CIVS.on(g, p) || g.player_is_barbarian(p) {
        return None;
    }
    Some(tr)
}

/// Does player `p` play trait `t`: it has the trait now (Adaptive and Insane leaders change
/// theirs) and its play is on.
pub fn has(g: &Game, p: PlayerId, t: Trait) -> bool {
    enabled_type(g, p, t).is_some_and(|tr| g.players[p].traits[tr])
}

// ----------------------------------------------------------------------------
// Raiders: pillage
// ----------------------------------------------------------------------------

/// Can our units pillage plot `q` for gold: an improvement (not a permanent one, not a city) on
/// land owned by a civilization we are at war with -- but not the land of the city our war
/// operation is taking (within two plots of it), which is ours to work once it falls.
fn pillage_target(g: &Game, v: &View, q: PlotIdx) -> bool {
    let pl = &g.map.plots[q];
    let Some(imp) = pl.improvement_type else { return false };
    if g.d.improvements[imp].permanent || pl.plot_city.is_some() {
        return false;
    }
    if !pl.owner.is_some_and(|o| v.civ_enemies.contains(&g.players[o].team)) {
        return false;
    }
    let (x, y) = g.map.xy(q);
    !v.campaign.ops.iter().any(|op| op.civ && {
        let (tx, ty) = g.map.xy(op.plot);
        g.plot_distance(x, y, tx, ty) <= 2
    })
}

/// The unit of `grp` that pillages its plot: one that can, with the most moves left.
fn pillager(g: &Game, grp: GroupId) -> Option<UnitId> {
    let units = &g.groups[grp].units;
    let q = g.unit_plot(*units.first()?);
    units.iter().copied().filter(|&u| g.unit_moves_left(u) > 0 && g.unit_can_pillage(u, q)).max_by_key(|&u| (g.unit_moves_left(u), std::cmp::Reverse(u)))
}

/// One unit of the group pillages the plot (taken out of the group, as a human selects a single
/// unit, and back in it after), so the rest of the stack keeps its moves. True if it pillaged.
fn pillage_with(g: &mut Game, grp: GroupId, u: UnitId) -> bool {
    let p = g.units[u].owner;
    let q = g.unit_plot(u);
    let imp = g.map.plots[q].improvement_type;
    let gold = g.players[p].gold;
    let alone = g.groups[grp].units.len() == 1;
    if !alone {
        g.unit_join_group(u, None, true);
    }
    let Some(ng) = g.units.contains(u).then(|| g.units[u].group).flatten() else { return false };
    push(g, ng, MissionType::MISSION_PILLAGE, -1, -1, 0, false);
    let done = g.map.plots[q].improvement_type != imp;
    if !alone && g.units.contains(u) && alive(g, grp) && g.units[u].group != Some(grp) {
        g.unit_join_group(u, Some(grp), false);
    }
    if done {
        crate::trace!(g, p, "raiders: pillaged {} at {:?} (+{} gold)", imp.map_or("?", |i| g.d.improvements[i].base.type_.as_str()), g.map.xy(q), g.players[p].gold - gold);
    }
    done
}

/// Before a Raiders stack's orders: on an enemy improvement, pillage it when that costs the
/// stack no march -- it belongs to no operation (and no city of ours is in peril), or every unit
/// has a move to spare.
pub fn before_army(g: &mut Game, v: &mut View, grp: GroupId) {
    if !has(g, v.p, Trait::Raiders) || v.civ_enemies.is_empty() {
        return;
    }
    crate::campaign::plan(g, v);
    let Some(head) = g.group_get_head_unit(grp) else { return };
    if !pillage_target(g, v, g.unit_plot(head)) {
        return;
    }
    // (A stack that may have to march home to a city in peril keeps its moves too.)
    let free = v.campaign.op_of(grp).is_none() && v.peril.is_empty();
    let spare = g.groups[grp].units.iter().all(|&u| g.unit_moves_left(u) > g.d.defines.move_denominator);
    if !free && !spare {
        return;
    }
    if let Some(u) = pillager(g, grp) {
        pillage_with(g, grp, u);
    }
}

/// After a Raiders stack's orders: a stack that stays on an enemy improvement this turn (it
/// holds, or arrived with moves left) pillages it with a unit that still has moves.
pub fn after_army(g: &mut Game, v: &View, grp: GroupId) {
    if !alive(g, grp) || !has(g, v.p, Trait::Raiders) || v.civ_enemies.is_empty() {
        return;
    }
    let Some(head) = g.group_get_head_unit(grp) else { return };
    if !pillage_target(g, v, g.unit_plot(head)) {
        return;
    }
    if let Some(u) = pillager(g, grp) {
        pillage_with(g, grp, u);
    }
}

/// A Raiders stack that no operation needs, at war: the enemy improvement within two turns that
/// is worth most, where the danger is well under the stack's defense and no enemy city is next
/// door (garrisons are not in the danger map). True if the stack set out.
pub fn raid(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    if !has(g, v.p, Trait::Raiders) || v.civ_enemies.is_empty() {
        return false;
    }
    crate::campaign::plan(g, v);
    if v.campaign.op_of(grp).is_some() || !g.groups[grp].units.iter().any(|&o| g.d.units[g.units[o].unit_type].pillage) {
        return false;
    }
    let here = g.unit_plot(u);
    let defense: i32 = g.groups[grp].units.iter().map(|&o| defense_power(g, o)).sum::<i32>().max(1);
    let reach = v.flood.run(g, u, 2, false);
    let near_city = |q: PlotIdx| {
        let (x, y) = g.map.xy(q);
        (0..DirectionType::NUM).any(|dir| g.plot_direction(x, y, DirectionType::from_usize(dir)).is_some_and(|n| g.map.plots[n].plot_city.is_some_and(|c| v.civ_enemies.contains(&g.players[g.cities[c].owner].team))))
    };
    let best = reach
        .iter()
        .filter(|r| r.plot != here && pillage_target(g, v, r.plot) && v.danger_at(r.plot) * 3 < defense * 2 && !near_city(r.plot))
        .map(|r| {
            let imp = g.map.plots[r.plot].improvement_type.expect("pillage target");
            let worth: i32 = g.d.improvements[imp].yield_change.iter().sum::<i32>() + 1;
            (worth * 10 - r.turns * 4, r.plot)
        })
        .max_by_key(|&(s, q)| (s, std::cmp::Reverse(q)));
    let Some((_, q)) = best else { return false };
    crate::trace!(g, v.p, "raiders: stack {:?} raids {:?}", grp, g.map.xy(q));
    move_to(g, grp, q, 0);
    after_army(g, v, grp);
    if alive(g, grp) && g.group_ready_to_move(grp, true) {
        crate::units::skip(g, grp);
    }
    true
}

// ----------------------------------------------------------------------------
// Spiritual: civics and religion without anarchy
// ----------------------------------------------------------------------------

/// How often player `p` reconsiders its civics (turns) and the summed civic value a change must
/// gain: every 10 turns at 100 (a revolution costs anarchy), a Spiritual leader every 2 at 40.
pub fn civic_cadence(g: &Game, p: PlayerId) -> (i32, i32) {
    if has(g, p, Trait::Spiritual) {
        (2, 40)
    } else {
        (10, 100)
    }
}

/// The weight of keeping the current state religion (a conversion costs anarchy): 15, or 5 for
/// a Spiritual leader (no anarchy; only against flip-flopping).
pub fn religion_stickiness(g: &Game, p: PlayerId) -> i32 {
    if has(g, p, Trait::Spiritual) {
        5
    } else {
        15
    }
}

// ----------------------------------------------------------------------------
// Arcane and Summoner: casters
// ----------------------------------------------------------------------------

/// Does `p` take the arcane path (switch `ARCANE_PATH`): a magic leader (Arcane or Summoner)
/// heads for Sorcery once the economy's tier 2 is in (`tech_plan`), with the sphere tech its mana
/// makes worth most on the way; the Amurites' own plan did so, the other magic leaders reached
/// Sorcery by the plain scores. (Earlier versions put Knowledge of the Ether right after the first
/// essentials and Sorcery right after Writing: Adepts a century before their Mages, and an economy
/// behind when the games are decided -- docs/AI_LOG.md, "Magic civilizations".)
pub fn arcane_path(g: &Game, p: PlayerId) -> bool {
    (has(g, p, Trait::Arcane) || has(g, p, Trait::Summoner)) && crate::switches::ARCANE_PATH.on(g, p)
}

/// Does `p` field a magic leader's army (switch `MAGE_ARMY`; Arcane or Summoner): casters in
/// proportion to its field army ([`mage_army_casters`]), on top of it. An expert who plays a magic
/// civilization fights with his Mages: a Fireball softens what his swords then take, and a Wizard
/// is never lost in the attack. Without it the corps stayed at (cities + 2) / 2 or less while the
/// army grew: 4 casters to 45 field units at turn 400. (First version: one for every three field
/// units, its war wizards counting toward the army target -- the casters replaced fighters and
/// defenders, and the magic civilizations lost more cities: docs/AI_LOG.md, "Magic civilizations".)
pub fn mage_army(g: &Game, p: PlayerId) -> bool {
    (has(g, p, Trait::Arcane) || has(g, p, Trait::Summoner)) && crate::switches::MAGE_ARMY.on(g, p)
}

/// The casters a [`mage_army`] wants: one for every four field units (attackers, siege), once our
/// Adepts can become Mages (we know the tech of a unit an Adept upgrades to: Sorcery); 0 before.
/// Adepts trained for the army before then studied for a hundred turns at the economy's cost
/// (docs/AI_LOG.md, "Magic civilizations").
pub fn mage_army_casters(g: &Game, p: PlayerId) -> i32 {
    use crate::production::count_ai;
    if !mages_open(g, p) {
        return 0;
    }
    (count_ai(g, p, UnitAIType::UNITAI_ATTACK) + count_ai(g, p, UnitAIType::UNITAI_ATTACK_CITY) + count_ai(g, p, UnitAIType::UNITAI_COLLATERAL)) / 4
}

/// Can `p`'s Adepts become Mages: its team knows the tech of a unit its Adept upgrades to.
fn mages_open(g: &Game, p: PlayerId) -> bool {
    let d = g.d;
    let Some(civ) = g.player_get_civilization_type(p) else { return false };
    let Some(adept) = d.ty::<UnitClassType>("UNITCLASS_ADEPT").and_then(|cl| d.civ_unit(civ, cl)) else { return false };
    let team = g.players[p].team;
    d.unit_classes
        .keys()
        .filter(|&cl| d.units[adept].upgrade_unit_class[cl])
        .filter_map(|cl| d.civ_unit(civ, cl))
        .any(|m| d.units[m].prereq_and_tech.map_or(true, |t| g.team_is_has_tech(team, Some(t))))
}

/// A production choice for city `c` (after the civilization's own): an Arcane leader keeps
/// (cities + 2) / 3 casters, a Summoner (cities + 2) / 2 (its summons need casters, and they die
/// in the fights where they summon), trained in its most productive safe city.
pub fn production(g: &Game, c: CityId, threat: i32) -> Option<Choice> {
    let p = g.cities[c].owner;
    let n = g.players[p].cities.len() as i32;
    let target = if has(g, p, Trait::Summoner) {
        (n + 2) / 2
    } else if has(g, p, Trait::Arcane) {
        (n + 2) / 3
    } else {
        return None;
    };
    crate::civs::casters_up_to(g, c, threat, target.max(1))
}

/// What promotion `promo` is worth to unit `u` (`value`: the generic estimate and the
/// civilization's): a Summoner's caster values a sphere that opens a summoning spell half again
/// as much, plus 60.
pub fn promotion_value(g: &Game, u: UnitId, promo: PromotionType, value: i32) -> i32 {
    let unit = &g.units[u];
    if !matches!(unit.ai_type, Some(UnitAIType::UNITAI_MAGE) | Some(UnitAIType::UNITAI_WARWIZARD)) || !has(g, unit.owner, Trait::Summoner) {
        return value;
    }
    let d = g.d;
    // A sphere's first promotion that leads to a summon (Entropy I before Entropy II's Pit Beast)
    // counts as one (switch `SUMMON_MANA`).
    let toward = || crate::switches::SUMMON_MANA.on(g, unit.owner) && d.promotions.keys().any(|next| d.promotions[next].prereq_promotion == Some(promo) && summoning_spheres(d).contains(&next));
    if summoning_spheres(d).contains(&promo) || toward() {
        value + value / 2 + 60
    } else {
        value
    }
}

/// What a node of mana `bonus` is worth over [`summon_mana`]'s competitors to a Summoner without
/// one (`mana::node_value` scale; nodes run from about 1000 to 3000): the first node goes to the
/// summon.
pub const SUMMON_NODE: i32 = 3000;

/// The mana whose sphere's second promotion (a Mage's, with Channeling II) opens a spell summoning
/// a creature that stays to fight: Death (Spectre), Entropy (Pit Beast), Law (Einherjar), Ice (Ice
/// Elemental). Fire's, Fireball, summons a projectile (`bSuicide`), not a creature.
pub fn summon_mana(d: &GameData) -> &'static [BonusType] {
    static S: std::sync::OnceLock<Vec<BonusType>> = std::sync::OnceLock::new();
    S.get_or_init(|| {
        let creature = |u: UnitType| !d.units[u].suicide && d.units[u].combat + d.units[u].damage_type_combat.iter().sum::<i32>() > 0;
        let mut v: Vec<BonusType> = d
            .spells
            .iter()
            .filter(|s| s.civilization_prereq.is_none() && s.create_unit_type.is_some_and(creature))
            .filter_map(|s| s.promotion_prereq1)
            .filter_map(|second| d.promotions[second].prereq_promotion)
            .filter_map(|first| d.promotions[first].bonus_prereq)
            .collect();
        v.sort();
        v.dedup();
        v
    })
}

/// Does Summoner `p` still go for one of the [`summon_mana`] (switch `SUMMON_MANA`): it has none.
pub fn wants_summon_mana(g: &Game, p: PlayerId) -> bool {
    has(g, p, Trait::Summoner) && crate::switches::SUMMON_MANA.on(g, p) && !summon_mana(g.d).iter().any(|&b| g.player_has_bonus(p, b))
}

/// What one more node of mana `bonus` is worth to `p` (`mana::node_value`; `value`: its generic
/// worth): [`SUMMON_NODE`] more for a Summoner's first [`summon_mana`].
pub fn node_value(g: &Game, p: PlayerId, bonus: BonusType, value: i32) -> i32 {
    if summon_mana(g.d).contains(&bonus) && wants_summon_mana(g, p) {
        value + SUMMON_NODE
    } else {
        value
    }
}

/// The base value (C++ `AI_bonusVal` scale) of resource `bonus` to `p` gaining it in a trade
/// (`trade::bonus_trade_val`; `value`: the generic one): a Summoner's first [`summon_mana`] is
/// worth 20 more, two happy resources (`AI_baseBonusVal` gives 10 a point of happiness) -- not
/// more: `trade::buy_resource_offer` pays our own valuation in gold.
pub fn bonus_trade_value(g: &Game, p: PlayerId, bonus: BonusType, change: i32, value: i32) -> i32 {
    if change > 0 && summon_mana(g.d).contains(&bonus) && wants_summon_mana(g, p) {
        value + 20
    } else {
        value
    }
}

/// The summoning spheres: sphere promotions (a chain with a promotion that needs a mana
/// resource, so not Channeling) that are the first prerequisite of a spell any civilization may
/// cast creating a unit that fights (Raise Skeleton, Fireball, the elementals, ...).
fn summoning_spheres(d: &GameData) -> &'static [PromotionType] {
    static S: std::sync::OnceLock<Vec<PromotionType>> = std::sync::OnceLock::new();
    S.get_or_init(|| {
        let sphere = |mut pr: PromotionType| {
            for _ in 0..8 {
                if d.promotions[pr].bonus_prereq.is_some() {
                    return true;
                }
                match d.promotions[pr].prereq_promotion {
                    Some(below) => pr = below,
                    None => break,
                }
            }
            false
        };
        let fights = |u: UnitType| d.units[u].combat + d.units[u].damage_type_combat.iter().sum::<i32>() > 0;
        let mut v: Vec<PromotionType> = d
            .spells
            .iter()
            .filter(|s| s.civilization_prereq.is_none() && s.create_unit_type.is_some_and(fights))
            .filter_map(|s| s.promotion_prereq1)
            .filter(|&pr| sphere(pr))
            .collect();
        v.sort();
        v.dedup();
        v
    })
}

/// What researching `t` is worth to player `p` (`value`: the generic estimate and the
/// civilization's): a Summoner without Adepts values the tech that opens them (the civilization's
/// Adept and the building it needs, Knowledge of the Ether) at +1500 once it can research it, and
/// the techs on the way at +500 -- summons need casters, and casters came only around turn 300.
pub fn tech_value(g: &Game, p: PlayerId, t: TechType, value: i32) -> i32 {
    if !has(g, p, Trait::Summoner) {
        return value;
    }
    let team = g.players[p].team;
    let needed = adept_techs(g, p);
    if needed.is_empty() || g.team_is_has_tech(team, Some(t)) {
        return value;
    }
    let path = crate::victory::tech_path(g, team, &needed);
    if !path.contains(&t) {
        return value;
    }
    let info = &g.d.techs[t];
    let ready = info.prereq_and_techs.iter().all(|&x| g.team_is_has_tech(team, Some(x)))
        && (info.prereq_or_techs.is_empty() || info.prereq_or_techs.iter().any(|&x| g.team_is_has_tech(team, Some(x))));
    value + if needed.contains(&t) && ready { 1500 } else { 500 }
}

/// The techs player `p`'s Adept still needs (its own and its building's); empty when it has them
/// or has no Adept.
fn adept_techs(g: &Game, p: PlayerId) -> Vec<TechType> {
    let d = g.d;
    let team = g.players[p].team;
    let Some(civ) = g.player_get_civilization_type(p) else { return Vec::new() };
    let Some(adept) = d.ty::<UnitClassType>("UNITCLASS_ADEPT").and_then(|cl| d.civ_unit(civ, cl)) else { return Vec::new() };
    let info = &d.units[adept];
    let mut techs: Vec<TechType> = info.prereq_and_tech.into_iter().chain(info.prereq_and_techs.iter().copied()).collect();
    if let Some(b) = crate::production::unit_building(d, civ, info) {
        techs.extend(d.buildings[b].prereq_and_tech);
        techs.extend(d.buildings[b].prereq_and_techs.iter().flatten().copied());
    }
    techs.sort();
    techs.dedup();
    techs.retain(|&x| !g.team_is_has_tech(team, Some(x)));
    techs
}

/// The factor (numerator, denominator) on what a temporary summon of player `p` is worth: a
/// Summoner's last a turn longer (`iSummonDuration`), so one that fights this turn stands to fight
/// again the next: 3/2. 1/1 for others.
pub fn summon_turns(g: &Game, p: PlayerId) -> (i32, i32) {
    if has(g, p, Trait::Summoner) {
        (3, 2)
    } else {
        (1, 1)
    }
}

/// What a lasting summon (Raise Skeleton) of `strength` cast `n` at a time is worth to player `p`
/// at peace with no enemy about (the generic value is 0 then): for a Summoner, the army unit it
/// is, while the army is short of its peacetime target (`production::army_target`) and the
/// treasury can carry another unit. 0 for others.
pub fn peace_summon(g: &Game, p: PlayerId, strength: i32, n: i32) -> i32 {
    if !has(g, p, Trait::Summoner) {
        return 0;
    }
    let attackers: i32 = [UnitAIType::UNITAI_ATTACK, UnitAIType::UNITAI_ATTACK_CITY, UnitAIType::UNITAI_COLLATERAL].into_iter().map(|a| crate::production::count_ai(g, p, a)).sum();
    if attackers >= crate::production::army_target(g, p) || !crate::economy::can_afford_unit(g, p, true, false) {
        return 0;
    }
    strength * 10 * n
}

// ----------------------------------------------------------------------------
// Philosophical: great people
// ----------------------------------------------------------------------------

/// Once per turn, before units move: a Philosophical leader keeps the great-people emphasis on
/// its best great-people cities, 1 + cities / 8 of them (size 4 and up, with a slot for a
/// specialist that makes great people), changing one only for a city clearly better (half as
/// good again) so the specialists stay put. A player whose trait play is on but who lost the
/// trait (Adaptive, Insane) drops the emphasis.
pub fn turn(g: &mut Game, v: &View) {
    let p = v.p;
    if enabled_type(g, p, Trait::Philosophical).is_none() {
        return;
    }
    let Some(e) = g.d.emphasizes.keys().find(|&e| g.d.emphasizes[e].great_people) else { return };
    let cities: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| g.cities.contains(c)).collect();
    let marked: Vec<CityId> = cities.iter().copied().filter(|&c| g.cities[c].ai_emphasize[e]).collect();
    if !has(g, p, Trait::Philosophical) {
        for c in marked {
            g.city_ai_set_emphasize(c, e, false);
        }
        return;
    }
    let want = 1 + cities.len() / 8;
    let mut ranked: Vec<(i32, CityId)> = cities.iter().copied().filter(|&c| g.cities[c].population >= 4).map(|c| (gp_city_score(g, c), c)).filter(|&(s, _)| s > 0).collect();
    ranked.sort_by_key(|&(s, c)| (std::cmp::Reverse(s), c));
    // Keep the marked cities that still qualify, then fill or swap from the ranking.
    let mut keep: Vec<(i32, CityId)> = ranked.iter().copied().filter(|&(_, c)| marked.contains(&c)).take(want).collect();
    for &(s, c) in &ranked {
        if keep.iter().any(|&(_, k)| k == c) {
            continue;
        }
        if keep.len() < want {
            keep.push((s, c));
        } else if let Some(i) = keep.iter().enumerate().min_by_key(|(_, &(ks, kc))| (ks, std::cmp::Reverse(kc))).map(|(i, _)| i) {
            if s * 2 > keep[i].0 * 3 {
                keep[i] = (s, c);
            }
        }
    }
    if crate::trace_on(p) && (keep.len() != marked.len() || keep.iter().any(|&(_, k)| !marked.contains(&k))) {
        let show = |c: CityId| format!("{}(pop {}, gp {}, score {})", g.cities[c].name, g.cities[c].population, g.city_great_people_rate(c), gp_city_score(g, c));
        crate::trace!(g, p, "philosophical: marked {:?} -> {:?}; ranked {:?}", marked.iter().map(|&c| show(c)).collect::<Vec<_>>(), keep.iter().map(|&(_, c)| show(c)).collect::<Vec<_>>(), ranked.iter().take(4).map(|&(_, c)| show(c)).collect::<Vec<_>>());
    }
    for c in marked {
        if !keep.iter().any(|&(_, k)| k == c) {
            g.city_ai_set_emphasize(c, e, false);
        }
    }
    for (_, c) in keep {
        if !g.cities[c].ai_emphasize[e] {
            crate::trace!(g, p, "philosophical: great people in {}", g.cities[c].name);
            g.city_ai_set_emphasize(c, e, true);
        }
    }
}

/// How well city `c` makes great people: the great-people points of the specialists it can
/// employ (slots its buildings give, or any number the civics allow; at most half its people),
/// plus its great-people rate now (which the emphasis itself raises: the choice stays put).
/// 0 for a city that cannot employ any such specialist.
fn gp_city_score(g: &Game, c: CityId) -> i32 {
    let d = g.d;
    let city = &g.cities[c];
    let staff = (city.population / 2).max(1);
    let mut points = 0;
    for s in d.specialists.keys() {
        let gpp = d.specialists[s].great_people_rate_change;
        if gpp <= 0 {
            continue;
        }
        let slots = if g.player_is_specialist_valid(city.owner, s) { staff } else { city.max_specialist_count[s] };
        points += slots.min(staff) * gpp;
    }
    if points == 0 {
        return 0;
    }
    points.min(staff * 3) * 4 + g.city_great_people_rate(c) * 2 + city.population
}

// ----------------------------------------------------------------------------
// Financial: the extra commerce at the threshold
// ----------------------------------------------------------------------------

/// The value of the extra yield a Financial leader gets at the threshold (+1 commerce on plots
/// with 2 or more) that an improvement brings to plot `q`, counted as the worker counts the
/// yields: `now` is the improvement's yield change when built and `grown` its grown form's (a
/// cottage's town), weighted 1 and 2 as the yields are; `old` the current improvement's
/// (`plot_calculate_improvement_yield_change` leaves the threshold out). `w` is the worker's weight
/// of a point of that yield. Only on an unimproved plot a city of ours works now: an unworked
/// cottage never grows and its extra commerce is never had (counting it on every plot drew the
/// workers from mines, farms and roads to cottages nobody worked), and on an improved plot it made
/// workers replace farms with cottages that start over (fewer improved plots worked around turn
/// 250). docs/AI_LOG.md, "Leader traits".
pub fn threshold_value(g: &Game, p: PlayerId, q: PlotIdx, y: usize, old: i32, now: i32, grown: Option<i32>, w: i32) -> i32 {
    if !has(g, p, Trait::Financial) {
        return 0;
    }
    let th = g.players[p].extra_yield_threshold[y];
    let pl = &g.map.plots[q];
    if th <= 0 || pl.improvement_type.is_some() || !g.plot_is_being_worked(q) || !pl.working_city.is_some_and(|c| g.cities[c].owner == p) {
        return 0;
    }
    let extra = g.d.defines.extra_yield;
    let cur = g.map.plots[q].yields[y];
    // The plot's yield for its owner includes the extra yield already when it is ours.
    let raw = if g.map.plots[q].owner == Some(p) && cur >= th + extra { cur - extra } else { cur };
    let gain = |change: i32| i32::from(raw - old + change >= th) - i32::from(raw >= th);
    let thirds = match grown {
        Some(gy) => gain(now) + 2 * gain(gy),
        None => 3 * gain(now),
    };
    thirds * extra * w / 3
}

// ----------------------------------------------------------------------------
// Industrious: wonders
// ----------------------------------------------------------------------------

/// What building `b` is worth in city `c` (`value`: the generic estimate and the
/// civilization's): an Industrious leader values limited wonders half again as much.
pub fn building_value(g: &Game, c: CityId, b: BuildingType, value: i32) -> i32 {
    if value <= 0 || !has(g, g.cities[c].owner, Trait::Industrious) {
        return value;
    }
    match g.d.buildings[b].building_class_type {
        Some(bc) if ffh_core::utils::is_limited_wonder_class(g.d, bc) => value + value / 2,
        _ => value,
    }
}

// ----------------------------------------------------------------------------
// Aggressive, Ingenuity, Tolerant, Expansive
// ----------------------------------------------------------------------------

/// The strength (percent) an Aggressive leader's fighters get from their free Combat I, which
/// neither the power rating nor a unit type's strength counts, once its home landmass is full
/// (`naval::home_is_full`); 0 for others. Counted from the start, the lower war bar had Aggressive
/// leaders preparing wars while land was still free: preparing halves the settlers and turns
/// production to the army, and at turn 200 they had 0.7 fewer cities (t -4.0) and from turn 250 a
/// fifth less research (seeds 7401-7496, every trait on; docs/AI_LOG.md, "Leader traits").
pub fn attack_bonus_pct(g: &Game, p: PlayerId) -> i32 {
    let Some(tr) = enabled_type(g, p, Trait::Aggressive).filter(|&tr| g.players[p].traits[tr]) else { return 0 };
    if !crate::naval::home_is_full(g, p) {
        return 0;
    }
    let info = &g.d.traits[tr];
    g.d.promotions.keys().filter(|&pr| info.free_promotion[pr]).map(|pr| g.d.promotions[pr].combat_percent).sum::<i32>().max(0)
}

/// The gold player `p` keeps after an upgrade: `reserve`, or half of it for an Ingenuity leader
/// (its upgrades cost half).
pub fn upgrade_reserve(g: &Game, p: PlayerId, reserve: i32) -> i32 {
    if has(g, p, Trait::Ingenuity) {
        reserve / 2
    } else {
        reserve
    }
}

/// A Tolerant leader keeps every city it conquers (Assimilation: its civilization's buildings
/// and units become ours to build).
pub fn keeps_conquests(g: &Game, p: PlayerId) -> bool {
    has(g, p, Trait::Tolerant)
}

/// Settlers an Expansive leader builds at a time beyond the generic number: 1.
pub fn extra_settlers(g: &Game, p: PlayerId) -> i32 {
    i32::from(has(g, p, Trait::Expansive))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trait_names_resolve() {
        let d = ffh_data::global();
        for (name, _) in NAMES {
            assert!(d.ty::<TraitType>(&format!("TRAIT_{name}")).is_some(), "TRAIT_{name}");
        }
    }

    #[test]
    fn summon_mana_is_death_entropy_law_ice() {
        let d = ffh_data::global();
        let mut names: Vec<&str> = super::summon_mana(d).iter().map(|&b| d.bonuses[b].base.type_.as_str()).collect();
        names.sort();
        assert_eq!(names, ["BONUS_MANA_DEATH", "BONUS_MANA_ENTROPY", "BONUS_MANA_ICE", "BONUS_MANA_LAW"]);
    }

    #[test]
    fn summoning_spheres() {
        let d = ffh_data::global();
        let s = super::summoning_spheres(d);
        let has = |name: &str| s.contains(&d.t::<PromotionType>(name));
        for name in ["PROMOTION_DEATH1", "PROMOTION_DEATH2", "PROMOTION_FIRE2", "PROMOTION_AIR3", "PROMOTION_EARTH3"] {
            assert!(has(name), "{name} summons");
        }
        // Channeling opens civilization summons only; the Floating Eye does not fight.
        for name in ["PROMOTION_CHANNELING2", "PROMOTION_CHANNELING3", "PROMOTION_METAMAGIC1", "PROMOTION_FIRE1"] {
            assert!(!has(name), "{name} is no summoning sphere");
        }
    }
}
