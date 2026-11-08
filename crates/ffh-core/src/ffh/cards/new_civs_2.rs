//! Batch B8, new civilizations 2 (docs/CARDS.md, sections 5 and 6): five great-person cards whose
//! world spell gives birth to a civilization or changes one's allegiance, and three Armageddon cards
//! that bring a civilization into the game. The specs are in docs/DECKS.md (sections 12-14) and
//! docs/DECKS_ROUND8.md (section B, amended by its C.5).
//!
//! - **Rite of the Necropolis** (Prophet, III, not drawn by good civilizations): the Necropolis
//!   rises on the three City Ruins nearest the caster's capital ([`necropolis_sites`]); its dead
//!   rise ([`necropolis_risers`], [`necropolis_rise`]).
//! - **Wake the Hill Clans** (Engineer, I): the Hill Clans, giants, the caster's vassal on hills
//!   ([`hill_clan_sites`]); +1 production on their hills (the civilization's `hills_yield_change`).
//! - **The Usurper's Crown** (Bard, III): a third of a large civilization's cities secede
//!   ([`usurper_cities`]) under the Usurper ([`usurper_unit`]).
//! - **Oath of Fealty** (Merchant, II): an enemy a third as strong becomes the caster's vassal.
//! - **Call the Aifon** (Merchant, II): the Aifon, the caster's vassal on an unowned coast
//!   ([`aifon_sites`]); their land units walk on water (the civilization's `unit_promotion`).
//! - **A04 The Scaled Kingdom** (Armageddon, 30), **A06 The Brood** (30, 50) and **A22 The Infernal
//!   Gate Opens** (60): their `valid` and `fire` functions, listed in `deck::doom_cards::CARDS`. Their
//!   civilizations have no founder ([`Birth::wild`]): the AI plays them, and they know the world's
//!   techs. The Infernal Gate brings the Infernals that exist (`Game::infernals_arrive`, shared with
//!   the Infernal Pact), once a game.

use std::cmp::Reverse;
use std::sync::OnceLock;

use crate::ffh::cards::world::{self, Birth, NewCity, NewUnit};
use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::SpellTarget;
use ffh_data::*;

/// A new civilization's attitude modifier toward the caster (FFH's scale, 4 a step).
pub const ATTITUDE: i32 = 4;
/// How many City Ruins the Necropolis rises on.
pub const NECROPOLIS_RUINS: usize = 3;
/// The size of the Necropolis's cities.
pub const NECROPOLIS_POPULATION: i32 = 2;
/// The Necropolis's attitude modifier toward good civilizations.
pub const NECROPOLIS_GOOD_ATTITUDE: i32 = -4;
/// The fewest unowned hill or peak plots within [`HILL_RANGE`] of the Hill Clans' first city.
pub const HILL_PLOTS: usize = 6;
/// How far the Hill Clans' hills reach from their first city.
pub const HILL_RANGE: i32 = 3;
/// The fewest cities of a civilization the Usurper's Crown splits.
pub const USURPER_CITIES: usize = 8;
/// The fewest cities that secede.
pub const USURPER_SECEDE_MIN: usize = 2;
/// The Usurper's attitude modifier toward the civilization it secedes from.
pub const USURPER_GRUDGE: i32 = -6;
/// The Oath of Fealty's target has at most a third of the caster's power.
pub const OATH_POWER_RATIO: i32 = 3;
/// The Scaled Kingdom's cities.
pub const SCALED_CITIES: usize = 3;
/// How near the Scaled Kingdom's cities a civilization's land makes it a neighbour.
pub const SCALED_NEIGHBOUR_RANGE: i32 = 6;
/// The Scaled Kingdom's attitude modifier toward its neighbours.
pub const SCALED_NEIGHBOUR_ATTITUDE: i32 = -2;
/// The Brood's nests: one in each of the largest unowned forest regions.
pub const BROOD_NESTS: usize = 3;
/// The fewest forest plots of a region a nest appears in.
pub const BROOD_FOREST_PLOTS: usize = 8;
/// The Giant Spiders the Brood starts with.
pub const BROOD_SPIDERS: usize = 6;
/// How far from the evil civilization's borders the Infernal Gate opens.
pub const INFERNAL_GATE_RANGE: i32 = 4;

/// The forest features (the Brood's regions).
const FORESTS: [&str; 3] = ["FEATURE_FOREST", "FEATURE_FOREST_ANCIENT", "FEATURE_FOREST_NEW"];

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellRiteOfTheNecropolis" => spell_rite_of_the_necropolis,
        "spellWakeTheHillClans" => spell_wake_the_hill_clans,
        "spellUsurpersCrown" => spell_usurpers_crown,
        "spellOathOfFealty" => spell_oath_of_fealty,
        "spellCallTheAifon" => spell_call_the_aifon,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        "reqRiteOfTheNecropolis" => req_rite_of_the_necropolis,
        "reqWakeTheHillClans" => req_wake_the_hill_clans,
        "reqUsurpersCrown" => req_usurpers_crown,
        "reqCallTheAifon" => req_call_the_aifon,
        _ => return None,
    };
    Some(f)
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        "targetsUsurpersCrown" => targets_usurpers_crown,
        "targetsOathOfFealty" => targets_oath_of_fealty,
        _ => return None,
    };
    Some(f)
}

/// `py_target_label` functions.
pub fn find_target_label(name: &str) -> Option<TargetLabelFn> {
    let f: TargetLabelFn = match name {
        "labelUsurpersCrown" => label_usurpers_crown,
        "labelOathOfFealty" => label_oath_of_fealty,
        _ => return None,
    };
    Some(f)
}

// ---------------------------------------------------------------------------------------------
// Shared.

fn unit(d: &GameData, name: &str) -> NewUnit {
    NewUnit::of(d.t::<UnitType>(name))
}

/// A new city at `plot` with `buildings` and `units`.
fn city(d: &GameData, plot: PlotIdx, buildings: &[&str], units: &[&str]) -> NewCity {
    NewCity { plot, buildings: buildings.iter().map(|&b| d.t::<BuildingType>(b)).collect(), units: units.iter().map(|&u| unit(d, u)).collect() }
}

fn dist(g: &Game, a: PlotIdx, b: PlotIdx) -> i32 {
    let ((ax, ay), (bx, by)) = (g.map.xy(a), g.map.xy(b));
    g.plot_distance(ax, ay, bx, by)
}

/// The plots within `range` plots (`plot_distance`) of `at`, in plot order.
fn plots_within(g: &Game, at: PlotIdx, range: i32) -> Vec<PlotIdx> {
    let (x, y) = g.map.xy(at);
    let mut out = Vec::new();
    for dx in -range..=range {
        for dy in -range..=range {
            if let Some(q) = g.plot_xy(x, y, dx, dy) {
                if dist(g, at, q) <= range {
                    out.push(q);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Is `b` within `MIN_CITY_RANGE` of `a` (the window `player_can_found` tests; the map wraps)?
fn too_near(g: &Game, a: PlotIdx, b: PlotIdx) -> bool {
    let range = g.d.defines.min_city_range;
    let (x, y) = g.map.xy(a);
    (-range..=range).any(|dx| (-range..=range).any(|dy| g.plot_xy(x, y, dx, dy) == Some(b)))
}

/// `tech` and the techs it needs: every and-prerequisite and the first or-prerequisite, down to
/// the first techs (what a civilization needs to know to train the units `tech` opens).
pub fn tech_with_prereqs(d: &GameData, tech: TechType) -> Vec<TechType> {
    let mut out = vec![tech];
    let mut i = 0;
    while i < out.len() {
        let info = &d.techs[out[i]];
        i += 1;
        for &t in info.prereq_and_techs.iter().chain(info.prereq_or_techs.first()) {
            if !out.contains(&t) {
                out.push(t);
            }
        }
    }
    out
}

/// The type `name` of civilization, cached (`None`: no such civilization in the data).
fn civ_named(d: &GameData, cell: &'static OnceLock<Option<CivilizationType>>, name: &str) -> Option<CivilizationType> {
    *cell.get_or_init(|| d.ty::<CivilizationType>(name))
}

// ---------------------------------------------------------------------------------------------
// Rite of the Necropolis.

/// The Necropolis's civilization.
pub fn necropolis_civ(d: &GameData) -> Option<CivilizationType> {
    static C: OnceLock<Option<CivilizationType>> = OnceLock::new();
    civ_named(d, &C, "CIVILIZATION_NECROPOLIS")
}

/// The City Ruins the Necropolis rises on for `p`: the [`NECROPOLIS_RUINS`] nearest `p`'s capital
/// (plot distance; ties: the lower plot index) among those a city can stand on
/// ([`world::free_site`]: unowned or `p`'s, no unit), none within `MIN_CITY_RANGE` of another.
/// `None` with fewer, or without a capital.
pub fn necropolis_sites(g: &Game, p: PlayerId) -> Option<Vec<PlotIdx>> {
    let ruins = g.d.ty::<ImprovementType>("IMPROVEMENT_CITY_RUINS")?;
    let capital = g.city_plot(g.player_get_capital_city(p)?);
    let mut found: Vec<(i32, PlotIdx)> =
        g.map.plot_ids().filter(|&q| g.map.plots[q].improvement_type == Some(ruins) && world::free_site(g, q, Some(p))).map(|q| (dist(g, q, capital), q)).collect();
    found.sort();
    let mut out: Vec<PlotIdx> = Vec::new();
    for (_, q) in found {
        if out.len() < NECROPOLIS_RUINS && !out.iter().any(|&o| too_near(g, o, q)) {
            out.push(q);
        }
    }
    (out.len() == NECROPOLIS_RUINS).then_some(out)
}

/// `reqRiteOfTheNecropolis(caster)`: the Necropolis has its ruins.
fn req_rite_of_the_necropolis(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    necropolis_sites(g, g.units[caster].owner).is_some()
}

/// `spellRiteOfTheNecropolis(caster)`: the Necropolis (+4 attitude toward the caster, -4 toward
/// good civilizations) in three cities of size 2 on the ruins, each with Walls and two Pyre
/// Zombies, with the Lich-Prince.
fn spell_rite_of_the_necropolis(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let d = g.d;
    let p = g.units[caster].owner;
    let Some(sites) = necropolis_sites(g, p) else { return };
    let mut b = Birth::new(d.t("CIVILIZATION_NECROPOLIS"), d.t("LEADER_LICH_PRINCE"), p);
    b.found = sites.iter().map(|&q| city(d, q, &["BUILDING_WALLS"], &["UNIT_PYRE_ZOMBIE", "UNIT_PYRE_ZOMBIE"])).collect();
    b.hero = Some(unit(d, "UNIT_LICH_PRINCE"));
    b.founder_attitude = ATTITUDE;
    b.alignment_attitude = Some((AlignmentType::ALIGNMENT_GOOD, NECROPOLIS_GOOD_ATTITUDE));
    let np = world::birth(g, &b);
    for c in g.players[np].cities.clone() {
        if g.cities[c].population < NECROPOLIS_POPULATION {
            g.city_set_population(c, NECROPOLIS_POPULATION);
        }
    }
}

/// Card: Rite of the Necropolis. When the battle at `to` is fought on the Necropolis's land: the
/// Necropolis, and how many of the battle's `victims` rise as its Pyre Zombies (the living ones:
/// not undead or otherwise not alive, not summoned, not immortal, who are reborn). Counted before
/// the victims die (`engagement/apply.rs`).
pub fn necropolis_risers(g: &Game, to: PlotIdx, victims: impl Iterator<Item = UnitId>) -> Option<(PlayerId, usize)> {
    let necropolis = necropolis_civ(g.d)?;
    let owner = g.map.plots[to].owner?;
    if g.player_get_civilization_type(owner) != Some(necropolis) {
        return None;
    }
    let n = victims.filter(|&u| g.units.contains(u) && g.unit_is_alive(u) && !g.unit_is_immortal(u) && g.units[u].duration == 0).count();
    (n > 0).then_some((owner, n))
}

/// Card: Rite of the Necropolis. After the battle at `to`, `count` Pyre Zombies of `necropolis`
/// rise there when it may stand there (no unit at war with it, no city of another civilization),
/// else in its city nearest `to`; they have no moves left.
pub fn necropolis_rise(g: &mut Game, necropolis: PlayerId, to: PlotIdx, count: usize) {
    let Some(zombie) = g.d.ty::<UnitType>("UNIT_PYRE_ZOMBIE") else { return };
    if !g.players[necropolis].alive {
        return;
    }
    let team = g.players[necropolis].team;
    let pl = &g.map.plots[to];
    let held = !g.plot_is_water(to)
        && pl.plot_city.map_or(true, |c| g.cities[c].owner == necropolis)
        && !pl.units.iter().any(|&u| !g.units[u].death_delay && g.at_war(team, g.unit_get_team(u)));
    let at = if held {
        Some(to)
    } else {
        g.players[necropolis].cities.iter().copied().filter(|&c| g.cities.contains(c)).min_by_key(|&c| (dist(g, g.city_plot(c), to), c)).map(|c| g.city_plot(c))
    };
    let Some(at) = at else { return };
    let (x, y) = g.map.xy(at);
    for _ in 0..count {
        let u = g.player_init_unit(necropolis, zombie, x, y, None, Some(DirectionType::DIRECTION_SOUTH));
        if g.units.contains(u) {
            g.unit_finish_moves(u);
        }
    }
    let turn = g.game_turn();
    g.log.msg(turn, Some(necropolis), || format!("{count} dead rise as Pyre Zombies at ({x}, {y})"));
}

// ---------------------------------------------------------------------------------------------
// Wake the Hill Clans.

/// Is `q` a hill or a peak no civilization owns?
fn clan_hill(g: &Game, q: PlotIdx) -> bool {
    (g.plot_is_hills(q) || g.plot_is_peak(q)) && g.map.plots[q].owner.is_none()
}

/// The Hill Clans' two sites for `p`: a free hill site ([`world::free_site`], unowned) with at
/// least [`HILL_PLOTS`] unowned hill or peak plots within [`HILL_RANGE`] (itself included), and the
/// best other free hill site among those plots. Of the hill sites that have both, the nearest
/// `p`'s capital (ties: the most hills, then the lower plot index; without a capital, the most
/// hills).
pub fn hill_clan_sites(g: &Game, p: PlayerId) -> Option<Vec<PlotIdx>> {
    let capital = g.player_get_capital_city(p).map(|c| g.city_plot(c));
    let hill_site = |q: PlotIdx| g.plot_is_hills(q) && world::free_site(g, q, None);
    let mut firsts: Vec<(i32, Reverse<usize>, PlotIdx)> = Vec::new();
    for q in g.map.plot_ids() {
        if !hill_site(q) {
            continue;
        }
        let hills = plots_within(g, q, HILL_RANGE).into_iter().filter(|&t| clan_hill(g, t)).count();
        if hills >= HILL_PLOTS {
            firsts.push((capital.map_or(0, |c| dist(g, q, c)), Reverse(hills), q));
        }
    }
    firsts.sort();
    for (_, _, first) in firsts {
        let others: Vec<PlotIdx> = plots_within(g, first, HILL_RANGE).into_iter().filter(|&t| t != first && hill_site(t)).collect();
        if let Some(&second) = world::best_sites(g, &others, 1, None, &[first]).first() {
            return Some(vec![first, second]);
        }
    }
    None
}

/// `reqWakeTheHillClans(caster)`: the Hill Clans have their hills.
fn req_wake_the_hill_clans(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    hill_clan_sites(g, g.units[caster].owner).is_some()
}

/// `spellWakeTheHillClans(caster)`: the Hill Clans, the caster's vassal, in two cities on the
/// hills, each with a Palisade and two Hill Giants, with Thrumm the Mountain-King.
fn spell_wake_the_hill_clans(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let d = g.d;
    let p = g.units[caster].owner;
    let Some(sites) = hill_clan_sites(g, p) else { return };
    let mut b = Birth::new(d.t("CIVILIZATION_HILL_CLANS"), d.t("LEADER_THRUMM"), p);
    b.found = sites.iter().map(|&q| city(d, q, &["BUILDING_PALISADE"], &["UNIT_CLAN_GIANT", "UNIT_CLAN_GIANT"])).collect();
    b.hero = Some(unit(d, "UNIT_THRUMM"));
    b.vassal = true;
    world::birth(g, &b);
}

// ---------------------------------------------------------------------------------------------
// The Usurper's Crown.

/// The civilizations the Usurper's Crown may split for `p`: those `p`'s team has met (not its own
/// team) with [`USURPER_CITIES`] cities or more and a capital.
pub fn usurper_targets(g: &Game, p: PlayerId) -> Vec<PlayerId> {
    let team = g.players[p].team;
    world::civ_players(g)
        .into_iter()
        .filter(|&q| {
            let qt = g.players[q].team;
            qt != team && g.teams[team].has_met[qt] && g.players[q].cities.len() >= USURPER_CITIES && g.player_get_capital_city(q).is_some()
        })
        .collect()
}

/// The cities of `q` that secede: a third of them (at least [`USURPER_SECEDE_MIN`]), those
/// farthest from its capital (plot distance; ties: the lower city id), the most populous first
/// (ties: the lower city id): the first is the Usurper's capital.
pub fn usurper_cities(g: &Game, q: PlayerId) -> Vec<CityId> {
    let Some(capital) = g.player_get_capital_city(q) else { return Vec::new() };
    let at = g.city_plot(capital);
    let n = (g.players[q].cities.len() / 3).max(USURPER_SECEDE_MIN);
    let mut far: Vec<(Reverse<i32>, CityId)> =
        g.players[q].cities.iter().copied().filter(|&c| c != capital && g.cities.contains(c)).map(|c| (Reverse(dist(g, g.city_plot(c), at)), c)).collect();
    far.sort();
    let mut out: Vec<CityId> = far.into_iter().take(n).map(|(_, c)| c).collect();
    out.sort_by_key(|&c| (Reverse(g.cities[c].population), c));
    out
}

/// The Usurper's unit: the best melee unit of `q`'s civilization that `q` could train (the highest
/// tier, then strength; ties: the lower id), not a world unit; else its civilization's Warrior.
pub fn usurper_unit(g: &Game, q: PlayerId) -> Option<UnitType> {
    let d = g.d;
    let civ = g.player_get_civilization_type(q)?;
    let melee = d.ty::<UnitCombatType>("UNITCOMBAT_MELEE");
    let best = d
        .unit_classes
        .keys()
        .filter(|&class| !crate::utils::is_world_unit_class(d, class))
        .filter_map(|class| d.civ_unit(civ, class))
        .filter(|&u| d.units[u].unit_combat_type == melee && g.player_can_train(q, u, false, true, false))
        .max_by_key(|&u| (d.units[u].tier, d.units[u].combat, Reverse(u)));
    best.or_else(|| d.ty::<UnitClassType>("UNITCLASS_WARRIOR").and_then(|class| d.civ_unit(civ, class)))
}

/// `reqUsurpersCrown(caster)`: always (the target is its own test).
fn req_usurpers_crown(_g: &Game, _caster: UnitId, _call: &PyCall) -> bool {
    true
}

/// `targetsUsurpersCrown(caster)`: [`usurper_targets`] of the caster's owner.
fn targets_usurpers_crown(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    usurper_targets(g, g.units[caster].owner).into_iter().map(SpellTarget::Player).collect()
}

/// `labelUsurpersCrown(pCaster, pTarget)`: how many of the civilization's cities secede.
fn label_usurpers_crown(g: &Game, _caster: UnitId, target: SpellTarget, _call: &PyCall) -> String {
    let SpellTarget::Player(q) = target else { return String::new() };
    format!("{} cities secede", usurper_cities(g, q).len())
}

/// `spellUsurpersCrown(caster)` at a civilization: its farthest third of cities secede with their
/// buildings and garrisons as a civilization of its own civilization, its parent, led by the
/// Usurper (a hero of its best melee unit), with its parent's alignment; +4 attitude toward the
/// caster, -6 toward its parent.
fn spell_usurpers_crown(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let d = g.d;
    let Some(SpellTarget::Player(q)) = g.spell_target else { return };
    let p = g.units[caster].owner;
    if !usurper_targets(g, p).contains(&q) {
        return;
    }
    let Some(civ) = g.player_get_civilization_type(q) else { return };
    let mut b = Birth::new(civ, d.t("LEADER_USURPER"), p);
    b.parent = Some(q);
    b.release = usurper_cities(g, q);
    b.hero = usurper_unit(g, q).map(|u| NewUnit { unit: u, promotions: vec![d.t::<PromotionType>("PROMOTION_HERO")] });
    b.founder_attitude = ATTITUDE;
    b.parent_attitude = USURPER_GRUDGE;
    b.description = Some(format!("{} Usurpers", g.player_get_civilization_adjective(q)));
    let np = world::birth(g, &b);
    g.players[np].alignment = g.players[q].alignment;
}

// ---------------------------------------------------------------------------------------------
// Oath of Fealty.

/// The civilizations the Oath of Fealty may take for `p`, one per team: at war with `p`'s team,
/// not a vassal, with at most a third of its power (`team_get_power`, without vassals). None under
/// No Vassal States, or while `p`'s team is a vassal.
pub fn oath_targets(g: &Game, p: PlayerId) -> Vec<PlayerId> {
    let team = g.players[p].team;
    if g.is_option(GameOptionType::GAMEOPTION_NO_VASSAL_STATES) || g.team_is_a_vassal(team) {
        return Vec::new();
    }
    let power = g.team_get_power(team, false);
    let mut teams: Vec<TeamId> = Vec::new();
    let mut out = Vec::new();
    for q in world::civ_players(g) {
        let qt = g.players[q].team;
        if qt == team || teams.contains(&qt) || !g.at_war(team, qt) || g.team_is_a_vassal(qt) || OATH_POWER_RATIO * g.team_get_power(qt, false) > power {
            continue;
        }
        teams.push(qt);
        out.push(q);
    }
    out
}

/// `targetsOathOfFealty(caster)`: [`oath_targets`] of the caster's owner.
fn targets_oath_of_fealty(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    oath_targets(g, g.units[caster].owner).into_iter().map(SpellTarget::Player).collect()
}

/// `labelOathOfFealty(pCaster, pTarget)`: its power and the caster's.
fn label_oath_of_fealty(g: &Game, caster: UnitId, target: SpellTarget, _call: &PyCall) -> String {
    let SpellTarget::Player(q) = target else { return String::new() };
    let (team, qt) = (g.players[g.units[caster].owner].team, g.players[q].team);
    format!("Power: {} (yours: {})", g.team_get_power(qt, false), g.team_get_power(team, false))
}

/// `spellOathOfFealty(caster)` at a civilization: the war ends and its team becomes the caster's
/// team's vassal (it capitulates), under the ordinary vassal rules.
fn spell_oath_of_fealty(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Player(q)) = g.spell_target else { return };
    let p = g.units[caster].owner;
    if !oath_targets(g, p).contains(&q) {
        return;
    }
    let (team, qt) = (g.players[p].team, g.players[q].team);
    g.team_make_peace(team, qt, true);
    g.team_assign_vassal(team, qt, true);
    let turn = g.game_turn();
    let (name, by) = (g.player_get_civilization_description(q), g.player_get_civilization_description(p));
    g.log.msg(turn, Some(p), || format!("{name} swears fealty to {by}"));
}

// ---------------------------------------------------------------------------------------------
// Call the Aifon.

/// The Aifon's two sites: in the largest unowned land region ([`world::regions`]) with two free
/// sites on the coast of an ocean, its two best.
pub fn aifon_sites(g: &Game) -> Option<Vec<PlotIdx>> {
    let ocean = g.d.defines.min_water_size_for_ocean;
    for region in world::regions(g, |_, _| true) {
        let coast: Vec<PlotIdx> = region.into_iter().filter(|&q| g.plot_is_coastal_land(q, ocean)).collect();
        if coast.len() < 2 {
            continue;
        }
        let sites = world::best_sites(g, &coast, 2, None, &[]);
        if sites.len() == 2 {
            return Some(sites);
        }
    }
    None
}

/// `reqCallTheAifon(caster)`: the Aifon have their coast.
fn req_call_the_aifon(g: &Game, _caster: UnitId, _call: &PyCall) -> bool {
    aifon_sites(g).is_some()
}

/// `spellCallTheAifon(caster)`: the Aifon, the caster's vassal, in two coastal cities, each with two
/// Axemen and an Archer (their land units walk on water), with the Deep Mother.
fn spell_call_the_aifon(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let d = g.d;
    let p = g.units[caster].owner;
    let Some(sites) = aifon_sites(g) else { return };
    let mut b = Birth::new(d.t("CIVILIZATION_AIFON"), d.t("LEADER_DEEP_MOTHER"), p);
    b.found = sites.iter().map(|&q| city(d, q, &[], &["UNIT_AXEMAN", "UNIT_AXEMAN", "UNIT_ARCHER"])).collect();
    b.hero = Some(unit(d, "UNIT_DEEP_MOTHER"));
    b.vassal = true;
    world::birth(g, &b);
}

// ---------------------------------------------------------------------------------------------
// A04 The Scaled Kingdom (Armageddon, 30).

/// The Scaled Kingdom's sites: up to [`SCALED_CITIES`] free sites of the largest unowned region of
/// jungle and marsh (the hell marsh, Shallows, included).
pub fn scaled_sites(g: &Game) -> Vec<PlotIdx> {
    let d = g.d;
    let jungle = d.ty::<FeatureType>("FEATURE_JUNGLE");
    let marsh: Vec<TerrainType> = ["TERRAIN_MARSH", "TERRAIN_SHALLOWS"].iter().filter_map(|n| d.ty::<TerrainType>(n)).collect();
    let swamp = |g: &Game, q: PlotIdx| {
        let pl = &g.map.plots[q];
        (jungle.is_some() && pl.feature_type == jungle) || pl.terrain_type.is_some_and(|t| marsh.contains(&t))
    };
    match world::largest_region(g, swamp) {
        Some(region) => world::best_sites(g, &region, SCALED_CITIES, None, &[]),
        None => Vec::new(),
    }
}

/// The Scaled Kingdom is played where its region has room for its three cities.
pub fn valid_scaled_kingdom(g: &Game) -> bool {
    scaled_sites(g).len() == SCALED_CITIES
}

/// The civilizations whose land lies within `range` plots of a city of `p`'s.
fn neighbours(g: &Game, p: PlayerId, range: i32) -> Vec<PlayerId> {
    let cities: Vec<PlotIdx> = g.players[p].cities.iter().map(|&c| g.city_plot(c)).collect();
    world::civ_players(g).into_iter().filter(|&q| q != p && cities.iter().any(|&c| world::within_range_of_borders(g, q, c, range))).collect()
}

/// The Scaled Kingdom rises in its region: up to three cities, each with a Palisade and two
/// Lizardman Rangers, with the Slitherking; it knows the world's techs and those its Rangers need,
/// and has -2 attitude toward the civilizations whose land is within 6 plots of its cities.
pub fn fire_scaled_kingdom(g: &mut Game, _: usize) {
    let d = g.d;
    let sites = scaled_sites(g);
    if sites.is_empty() {
        return;
    }
    let mut b = Birth::wild(d.t("CIVILIZATION_SCALED_KINGDOM"), d.t("LEADER_SLITHERKING"));
    b.found = sites.iter().map(|&q| city(d, q, &["BUILDING_PALISADE"], &["UNIT_LIZARDMAN_RANGER", "UNIT_LIZARDMAN_RANGER"])).collect();
    b.hero = Some(unit(d, "UNIT_SLITHERKING"));
    if let Some(t) = d.units[d.t::<UnitType>("UNIT_LIZARDMAN_RANGER")].prereq_and_tech {
        b.techs = tech_with_prereqs(d, t);
    }
    let np = world::birth(g, &b);
    for q in neighbours(g, np, SCALED_NEIGHBOUR_RANGE) {
        g.players[np].ai_attitude_extra[q] += SCALED_NEIGHBOUR_ATTITUDE;
    }
}

// ---------------------------------------------------------------------------------------------
// A06 The Brood (Armageddon, 30 then 50).

/// The Spider Nest, the Brood's lair.
pub fn spider_nest(d: &GameData) -> Option<ImprovementType> {
    d.ty::<ImprovementType>("IMPROVEMENT_SPIDER_NEST")
}

/// Where the nests appear: in each of the [`BROOD_NESTS`] largest unowned forest regions of
/// [`BROOD_FOREST_PLOTS`] forest plots or more, its best free site (none near another nest's).
pub fn nest_sites(g: &Game) -> Vec<PlotIdx> {
    let forests: Vec<FeatureType> = FORESTS.iter().filter_map(|n| g.d.ty::<FeatureType>(n)).collect();
    let mut out: Vec<PlotIdx> = Vec::new();
    for region in world::regions(g, |g, q| g.map.plots[q].feature_type.is_some_and(|f| forests.contains(&f))) {
        if out.len() >= BROOD_NESTS || region.len() < BROOD_FOREST_PLOTS {
            break;
        }
        if let Some(&q) = world::best_sites(g, &region, 1, None, &out).first() {
            out.push(q);
        }
    }
    out
}

/// The Brood brings barbarians (its nests' spiders): played only with barbarians and a forest
/// region for a nest.
pub fn valid_brood(g: &Game) -> bool {
    !g.is_option(GameOptionType::GAMEOPTION_NO_BARBARIANS) && spider_nest(g.d).is_some() && !nest_sites(g).is_empty()
}

/// The nests the Brood takes: every Spider Nest still standing on land, none within
/// `MIN_CITY_RANGE` of a city or of another taken nest (plot order).
pub fn brood_sites(g: &Game) -> Vec<PlotIdx> {
    let Some(nest) = spider_nest(g.d) else { return Vec::new() };
    let range = g.d.defines.min_city_range;
    let mut out: Vec<PlotIdx> = Vec::new();
    for q in g.map.plot_ids() {
        if g.map.plots[q].improvement_type != Some(nest) || g.plot_is_water(q) || out.iter().any(|&o| too_near(g, o, q)) {
            continue;
        }
        let (x, y) = g.map.xy(q);
        let city_near = (-range..=range).any(|dx| (-range..=range).any(|dy| g.plot_xy(x, y, dx, dy).is_some_and(|t| g.map.plots[t].plot_city.is_some())));
        if !city_near {
            out.push(q);
        }
    }
    out
}

/// Stage 0 (30): a Spider Nest appears at each [`nest_sites`] plot. Stage 1 (50): each nest still
/// standing becomes a city of the Brood, with six Giant Spiders shared among them and the Spider
/// Queen; nothing when no nest stands.
pub fn fire_brood(g: &mut Game, stage: usize) {
    let d = g.d;
    let Some(nest) = spider_nest(d) else { return };
    let turn = g.game_turn();
    if stage == 0 {
        for q in nest_sites(g) {
            g.plot_set_improvement_type(q, Some(nest));
            let (x, y) = g.map.xy(q);
            g.log.msg(turn, None, || format!("The Brood: a Spider Nest appears at ({x}, {y})."));
        }
        return;
    }
    let sites = brood_sites(g);
    if sites.is_empty() {
        g.log.msg(turn, None, || "The Brood: no nest stands.".to_string());
        return;
    }
    let mut b = Birth::wild(d.t("CIVILIZATION_BROOD"), d.t("LEADER_SPIDER_QUEEN"));
    b.found = sites.iter().map(|&q| city(d, q, &[], &[])).collect();
    let spider = d.t::<UnitType>("UNIT_BROOD_SPIDER");
    let n = b.found.len();
    for i in 0..BROOD_SPIDERS {
        b.found[i % n].units.push(NewUnit::of(spider));
    }
    b.hero = Some(unit(d, "UNIT_SPIDER_QUEEN"));
    world::birth(g, &b);
}

// ---------------------------------------------------------------------------------------------
// A22 The Infernal Gate Opens (Armageddon, 60).

/// The civilization the Infernal Gate opens beside: the evil civilization with the most cities
/// (ties: the lower player id).
pub fn infernal_patron(g: &Game) -> Option<PlayerId> {
    world::civ_players(g)
        .into_iter()
        .filter(|&p| g.players[p].alignment == Some(AlignmentType::ALIGNMENT_EVIL) && !g.players[p].cities.is_empty())
        .max_by_key(|&p| (g.players[p].cities.len(), Reverse(p)))
}

/// Where the Infernals arrive beside `patron`: the best free site (unowned) within
/// [`INFERNAL_GATE_RANGE`] plots of its borders; else the empty passable land plot of its own
/// nearest its capital (ties: the lower plot index).
pub fn infernal_gate_plot(g: &Game, patron: PlayerId) -> Option<PlotIdx> {
    let near = world::land_near_borders(g, patron, INFERNAL_GATE_RANGE);
    if let Some(&q) = world::best_sites(g, &near, 1, None, &[]).first() {
        return Some(q);
    }
    let capital = g.city_plot(g.player_get_capital_city(patron)?);
    g.map
        .plot_ids()
        .filter(|&q| {
            let pl = &g.map.plots[q];
            pl.owner == Some(patron) && !g.plot_is_water(q) && !g.plot_is_impassable(q) && pl.plot_city.is_none() && pl.units.is_empty()
        })
        .min_by_key(|&q| (dist(g, q, capital), q))
}

/// The Infernals come once a game: played if they have never been in play, an evil civilization
/// has a city and room beside it, and Hyborem may come (not No Hyborem or Basium).
pub fn valid_infernal_gate(g: &Game) -> bool {
    !g.is_option(GameOptionType::GAMEOPTION_NO_HYBOREM_OR_BASIUM)
        && !g.infernals_ever_in_play()
        && infernal_patron(g).is_some_and(|p| infernal_gate_plot(g, p).is_some())
}

/// The Infernals arrive as with the Infernal Pact beside the evil civilization with the most cities
/// (`Game::infernals_arrive`; the AI plays them). Nothing if they came meanwhile.
pub fn fire_infernal_gate(g: &mut Game, _: usize) {
    if !valid_infernal_gate(g) {
        return;
    }
    let Some(patron) = infernal_patron(g) else { return };
    let Some(plot) = infernal_gate_plot(g, patron) else { return };
    g.infernals_arrive(patron, plot, false);
    let turn = g.game_turn();
    let by = g.player_get_civilization_description(patron);
    g.log.msg(turn, None, || format!("The Infernal Gate opens beside {by}."));
}
