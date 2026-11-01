//! Mana nodes as an economy (`MANA_NODES`; More Naval AI's `CvUnitAI::AI_upgrademanaMove` and
//! its mana valuation in `CvPlayerAI::AI_bonusVal`). Raw mana (`BONUS_MANA`) in our lands is
//! worth nothing until a caster builds a node on it (`BUILD_MANA_FIRE` ... : the improvement
//! converts the bonus, for good: a node cannot be changed into another type). Before this, only the
//! Tower of Mastery goal built nodes (`victory::mana_node`), and that goal is never ours, so no
//! civilization ever converted its raw mana.
//!
//! What a node of a type is worth ([`node_value`]), on `production::building_value`'s scale for
//! the whole empire:
//! - the bonus's own effects in every city, read from the bonus data (`CvCity::processBonus`):
//!   Mind +3% research, Law -5% maintenance, Spirit +5% great people, Enchantment +1 happiness,
//!   Life +1 health (each node again where the bonus is `bModifierPerBonus`), Body and Entropy
//!   healing, Earth resource discovery, Chaos mutation;
//! - the first node of a type opens its sphere to our casters: the spells the sphere's promotions
//!   open, for the casters we have and one we could train ([`magic::sphere_value`]);
//! - units with an affinity for the mana grow stronger with every node;
//! - the first node of a type opens the buildings that need the mana (the Luchuirp's Blasting
//!   Workshop needs Fire, Pallens Engine Sun, the Adularia Chamber Shadow): each worth what
//!   `production::building_value` gives it in our best city for it ([`buildings_value`], switch
//!   `MANA_BUILDINGS`).
//!
//! A caster with nothing better to do (no strike to join, no city spell to keep) goes to build the
//! node worth most for the turns it takes ([`build_node`]); production trains one caster for it
//! when none of ours can ([`production`]), and the Mage Guild that trains it where no city can;
//! research values the techs that open node types by the nodes our raw mana would become
//! ([`tech_value`], More Naval AI's `AI_techValue` mana term).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::units::{alive, move_to, push, skip};

/// Raw mana and the node builds (`BUILD_MANA_X` -> `BONUS_MANA_X`).
fn raw_mana(d: &GameData) -> Option<BonusType> {
    crate::victory::statics(d).raw_mana
}

fn mana_builds(d: &GameData) -> &'static [(BuildType, BonusType)] {
    &crate::victory::statics(d).mana_builds
}

/// A typed mana bonus (`BONUSCLASS_MANA`).
pub fn is_mana(d: &GameData, b: BonusType) -> bool {
    d.bonuses[b].bonus_class_type.map_or(-1, |c| c.i()) == d.defines.bonusclass_mana
}

/// The plots in our cities' radius, ours, where a node can go: raw mana, and typed mana whose node
/// is gone (pillaged: the bonus stays, its node can be rebuilt). In plot order.
pub fn node_plots(g: &Game, p: PlayerId) -> Vec<PlotIdx> {
    let d = g.d;
    let Some(raw) = raw_mana(d) else { return Vec::new() };
    let mut out: Vec<PlotIdx> = Vec::new();
    for &c in &g.players[p].cities {
        for i in 0..ffh_core::defs::NUM_CITY_PLOTS {
            let Some(q) = g.city_index_plot(c, i) else { continue };
            let plot = &g.map.plots[q];
            if plot.owner != Some(p) || out.contains(&q) {
                continue;
            }
            let Some(b) = plot.bonus_type else { continue };
            let open = b == raw || is_mana(d, b) && plot.improvement_type.map_or(true, |imp| d.improvements[imp].bonus_convert.is_none());
            if open {
                out.push(q);
            }
        }
    }
    out.sort_unstable();
    out
}

/// One more node of type `bonus` for player `p`, on `production::building_value`'s scale summed
/// over our cities (see the module doc).
pub fn node_value(g: &Game, p: PlayerId, bonus: BonusType) -> i32 {
    let d = g.d;
    let info = &d.bonuses[bonus];
    let pl = &g.players[p];
    let have = g.player_has_bonus(p, bonus);
    let mut v = 0;
    // The bonus's effects: in full for the first node, again for each further one where the
    // modifiers count every copy.
    if !have || info.modifier_per_bonus {
        let gold_value = if crate::production::financial_trouble(g, p) { 160 } else { 80 };
        for &c in &pl.cities {
            let research = g.city_commerce_rate(c, CommerceType::COMMERCE_RESEARCH).max(0);
            v += info.research_modifier * research * 9 / 10;
            v += -info.maintenance_modifier * g.cities[c].maintenance.max(0) / 100 * gold_value / 100;
            v += info.great_people_rate_modifier * g.cities[c].population / 4;
            if info.happiness != 0 {
                let room = g.city_happy_level(c) - g.city_unhappy_level(c, 0);
                v += info.happiness * if room <= 0 { 180 } else if room <= 1 { 100 } else { 35 };
            }
            if info.health != 0 {
                let room = g.city_good_health(c) - g.city_bad_health(c, false, 0);
                v += info.health * if room <= 0 { 120 } else if room <= 2 { 60 } else { 20 };
            }
        }
        let n = pl.cities.len() as i32;
        v += info.heal_change * 2 * n;
        v += -info.heal_change_enemy * n * if at_civ_war(g, p) { 4 } else { 1 };
        v += info.discover_rand_modifier;
        v += info.mutate_chance * 5;
    }
    // The first node opens the sphere to our casters, and the buildings that need the mana.
    if !have {
        v += SPELL_SCALE * spells_value(g, p, bonus);
        if crate::switches::MANA_BUILDINGS.on(g, p) {
            v += buildings_value(g, p, bonus);
        }
    }
    // Affinities (a unit's strength grows with the mana of its type: Fire Elementals, ...).
    let affinity: i32 = pl.units.iter().map(|&u| g.units[u].bonus_affinity[bonus].max(0)).sum();
    v += affinity * 40;
    crate::traits::node_value(g, p, bonus, v) // trait-play
}

/// The buildings of our civilization that need mana `bonus` (`<Bonus>` or one of `<PrereqBonuses>`)
/// and whose tech we know: each worth its value in the city of ours where it is worth most and
/// not yet built (`production::building_value`, effects in the rules' code included).
fn buildings_value(g: &Game, p: PlayerId, bonus: BonusType) -> i32 {
    let d = g.d;
    let Some(civ) = g.player_get_civilization_type(p) else { return 0 };
    let team = g.players[p].team;
    let mut v = 0;
    for b in d.buildings.keys() {
        let info = &d.buildings[b];
        let needs = info.prereq_and_bonus == Some(bonus) || info.prereq_or_bonuses.contains(&Some(bonus));
        let ours = info.building_class_type.is_some_and(|k| d.civ_building(civ, k) == Some(b));
        if !needs || !ours || !info.prereq_and_tech.map_or(true, |t| g.team_is_has_tech(team, Some(t))) {
            continue;
        }
        let best = g.players[p].cities.iter().filter(|&&c| g.city_num_building(c, b) == 0).map(|&c| crate::production::building_value(g, c, b, 0)).max().unwrap_or(0);
        v += best.max(0);
    }
    v
}

/// At war with another civilization.
fn at_civ_war(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    let barb = g.barbarian_team();
    g.teams.keys().any(|t| t != team && t != barb && g.teams[t].alive_count > 0 && g.at_war(team, t))
}

/// `spell_worth`-scale points (Combat I about 40) to `building_value` points.
const SPELL_SCALE: i32 = 3;

/// The spells the sphere of `bonus` opens for our casters, on `magic::promotion_value`'s scale:
/// half its first promotion's worth to each caster of ours that lacks it (it may have no pick
/// left), and in full to one more caster, when we could train one.
fn spells_value(g: &Game, p: PlayerId, bonus: BonusType) -> i32 {
    let d = g.d;
    // The sphere's first promotion is the one that needs the mana (Fire I; Fire II needs Fire I).
    let Some(sphere) = d.promotions.keys().find(|&pr| d.promotions[pr].bonus_prereq == Some(bonus)) else {
        return 0;
    };
    let mut v = 0;
    for &u in &g.players[p].units {
        if is_caster(g, u) && !g.units[u].has_promotion[sphere] {
            v += crate::magic::promotion_value(g, u, sphere) / 2;
        }
    }
    if can_train_builder(g, p) {
        v += crate::magic::sphere_value(g, p, sphere);
    }
    v
}

/// A spellcaster of ours by trade (an adept or better, not a hero): what the caster roles use.
pub fn is_caster(g: &Game, u: UnitId) -> bool {
    matches!(
        g.units[u].ai_type,
        Some(UnitAIType::UNITAI_MAGE) | Some(UnitAIType::UNITAI_WARWIZARD) | Some(UnitAIType::UNITAI_MANA_UPGRADE) | Some(UnitAIType::UNITAI_TERRAFORMER)
    ) && can_build_nodes(g.d, g.units[u].unit_type)
}

/// Can units of type `t` build mana nodes?
pub fn can_build_nodes(d: &GameData, t: UnitType) -> bool {
    mana_builds(d).iter().any(|&(b, _)| d.units[t].builds[b])
}

/// Could we train a unit that builds nodes (its techs; the Mage Guild it may need aside)?
fn can_train_builder(g: &Game, p: PlayerId) -> bool {
    let d = g.d;
    let Some(civ) = g.player_get_civilization_type(p) else { return false };
    d.unit_classes.keys().filter_map(|cl| d.civ_unit(civ, cl)).any(|t| {
        let info = &d.units[t];
        can_build_nodes(d, t) && info.production_cost > 0 && info.min_level == 0 && info.default_unit_ai_type == Some(UnitAIType::UNITAI_MAGE) && g.player_can_train(p, t, false, false, false)
    })
}

/// The node jobs: for every node plot ([`node_plots`]), the build worth most ([`node_value`])
/// among those our techs allow there, with its value; best first.
pub fn jobs(g: &Game, p: PlayerId) -> Vec<(PlotIdx, BuildType, i32)> {
    let d = g.d;
    let team = g.players[p].team;
    let mut values: Vec<(BonusType, i32)> = Vec::new();
    let mut out: Vec<(PlotIdx, BuildType, i32)> = Vec::new();
    for q in node_plots(g, p) {
        let mut best: Option<(BuildType, i32)> = None;
        for &(b, bonus) in mana_builds(d) {
            let binfo = &d.builds[b];
            if !binfo.tech_prereq.map_or(true, |t| g.team_is_has_tech(team, Some(t))) {
                continue;
            }
            let Some(imp) = binfo.improvement else { continue };
            if !g.plot_can_have_improvement(q, imp, Some(team), false) {
                continue;
            }
            let value = match values.iter().find(|&&(o, _)| o == bonus) {
                Some(&(_, v)) => v,
                None => {
                    let v = node_value(g, p, bonus);
                    values.push((bonus, v));
                    v
                }
            };
            if best.map_or(true, |(_, bv)| value > bv) {
                best = Some((b, value));
            }
        }
        if let Some((b, value)) = best {
            out.push((q, b, value));
        }
    }
    out.sort_by_key(|&(q, _, v)| (std::cmp::Reverse(v), q));
    out
}

/// A node is worth a caster's trip below this (`building_value` scale).
pub const MIN_NODE_VALUE: i32 = 60;
/// A caster trained for nodes needs this much in jobs (the guild and the caster are about 210
/// hammers: two libraries).
const BUILDER_JOBS_VALUE: i32 = 300;

/// Caster `u` goes to build the node worth most for the turns it takes (within 12 turns, on a plot
/// no enemy we see can reach next turn), and builds it. True if it got orders.
pub fn build_node(g: &mut Game, v: &mut crate::view::View, grp: GroupId, u: UnitId) -> bool {
    let p = v.p;
    if !crate::switches::MANA_NODES.on(g, p) || !can_build_nodes(g.d, g.units[u].unit_type) {
        return false;
    }
    let jobs: Vec<(PlotIdx, BuildType, i32)> =
        jobs(g, p).into_iter().filter(|&(q, b, value)| value >= MIN_NODE_VALUE && !v.is_claimed(q) && v.danger_at(q) == 0 && g.unit_can_build(u, q, b, false)).collect();
    if jobs.is_empty() {
        return false;
    }
    let here = g.unit_plot(u);
    let reach = v.flood.run(g, u, 12, false);
    let turns = |q: PlotIdx| if q == here { Some(0) } else { reach.iter().find(|r| r.plot == q).map(|r| r.turns) };
    let Some((q, b, value, _)) = jobs
        .iter()
        .filter_map(|&(q, b, value)| turns(q).map(|t| (q, b, value, value * 10 / (t + 3))))
        .max_by_key(|&(q, _, _, score)| (score, std::cmp::Reverse(q)))
    else {
        return false;
    };
    v.claimed.push(q);
    let name = g.d.builds[b].base.type_.clone();
    crate::trace!(g, p, "caster {:?}: {} at {:?} (value {})", u, name, g.map.xy(q), value);
    g.tel_decision(p, "mana_node", &name, "value", value as i64, String::new);
    if q != here {
        move_to(g, grp, q, ffh_core::defs::MOVE_SAFE_TERRITORY);
        if !alive(g, grp) {
            return true;
        }
        if g.unit_plot(u) == here && g.group_ready_to_move(grp, false) {
            // No path the rules accept: let the caster do something else.
            g.group_clear_mission_queue(grp);
            return false;
        }
        push(g, grp, MissionType::MISSION_BUILD, b.i(), -1, 0, true);
    } else {
        push(g, grp, MissionType::MISSION_BUILD, b.i(), -1, 0, false);
    }
    if alive(g, grp) && g.group_ready_to_move(grp, false) {
        skip(g, grp);
    }
    true
}

/// Units of ours that can build nodes and do (casters; heroes have their own business).
pub fn builders(g: &Game, p: PlayerId) -> i32 {
    g.players[p].units.iter().filter(|&&u| is_caster(g, u)).count() as i32 + g.players[p].ai_num_train_ai_units[UnitAIType::UNITAI_MANA_UPGRADE]
}

/// Production (`MANA_NODES`): a caster to build our nodes when none of ours can and the jobs are
/// worth it ([`BUILDER_JOBS_VALUE`]), or the building that trains one (the Mage Guild) in our most
/// productive city when no city of ours can.
pub fn production(g: &Game, c: CityId) -> Option<crate::production::Choice> {
    use crate::production::Choice;
    let p = g.cities[c].owner;
    if !crate::switches::MANA_NODES.on(g, p) || builders(g, p) > 0 {
        return None;
    }
    let jobs = jobs(g, p);
    let total: i32 = jobs.iter().map(|&(_, _, v)| v).filter(|&v| v >= MIN_NODE_VALUE).sum();
    if crate::trace_on(p) && !jobs.is_empty() {
        let list: Vec<String> = jobs.iter().map(|&(q, b, v)| format!("{:?} {} {}", g.map.xy(q), g.d.builds[b].base.type_, v)).collect();
        crate::trace!(g, p, "{}: mana jobs {} [{}]", g.cities[c].name, total, list.join(", "));
    }
    if total < BUILDER_JOBS_VALUE {
        return None;
    }
    if let Some(u) = builder_unit(g, c) {
        crate::trace!(g, p, "{}: a caster for mana nodes (jobs {})", g.cities[c].name, total);
        return Some(Choice::Train(u, UnitAIType::UNITAI_MANA_UPGRADE));
    }
    crate::casters::caster_building(g, c).map(Choice::Construct)
}

/// The unit city `c` trains to build nodes: the best caster it can train that builds them.
pub fn builder_unit(g: &Game, c: CityId) -> Option<UnitType> {
    crate::production::best_unit(g, c, crate::production::Role::Mage).filter(|&u| can_build_nodes(g.d, u))
}

/// What tech `t` is worth to `p` for the node types it opens (`research::builds_value`, More
/// Naval AI's mana term in `AI_techValue`): the best of them for each node plot of ours still
/// open (raw mana, or a typed mana without its node) that our known techs give nothing better
/// for, at half a node's value; a quarter of that while we could not train a caster to build
/// them.
pub fn tech_value(g: &Game, p: PlayerId, t: TechType) -> i32 {
    let d = g.d;
    let team = g.players[p].team;
    let opened: Vec<BonusType> = mana_builds(d).iter().filter(|&&(b, _)| d.builds[b].tech_prereq == Some(t)).map(|&(_, bonus)| bonus).collect();
    if opened.is_empty() {
        return 0;
    }
    let raw = raw_mana(d);
    let plots: Vec<PlotIdx> = node_plots(g, p).into_iter().filter(|&q| g.map.plots[q].bonus_type == raw).collect();
    if plots.is_empty() {
        return 0;
    }
    // What the node types we can already build are worth (the plots go to the best).
    let known_best = mana_builds(d)
        .iter()
        .filter(|&&(b, _)| d.builds[b].tech_prereq.map_or(true, |k| g.team_is_has_tech(team, Some(k))))
        .map(|&(_, bonus)| node_value(g, p, bonus))
        .max()
        .unwrap_or(0);
    let mut gains: Vec<i32> = opened.iter().map(|&bonus| (node_value(g, p, bonus) - known_best).max(0)).collect();
    gains.sort_unstable_by(|a, b| b.cmp(a));
    let value: i32 = gains.iter().take(plots.len()).sum::<i32>() / 2;
    let builder = builders(g, p) > 0 || can_train_builder(g, p);
    if builder {
        value
    } else {
        value / 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_builds_and_typed_mana() {
        let d = ffh_data::global();
        assert!(mana_builds(d).len() >= 17, "a build for every typed mana");
        let fire = d.t::<BonusType>("BONUS_MANA_FIRE");
        assert!(is_mana(d, fire));
        assert!(!is_mana(d, d.t::<BonusType>("BONUS_MANA")), "raw mana is its own class");
        assert!(can_build_nodes(d, d.t::<UnitType>("UNIT_ADEPT")));
        assert!(!can_build_nodes(d, d.t::<UnitType>("UNIT_WORKER")));
    }
}
