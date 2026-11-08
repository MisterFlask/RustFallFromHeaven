//! Batch B4, new civilizations born mid-game (docs/CARDS.md, section 5): G12 The Patrian
//! Awakening, P18 Founding of the Dawn Knights, P19 The Fey Court Awakens, M07 The Condottieri
//! Free State and the Free City Charter, each a world spell giving birth to a civilization
//! ([`world::birth`]), and Mirror Colony (two cities of the caster's own, joined to its capital by
//! the Mirror Gate). The specs are in docs/DECKS_ROUND8.md (amended by its C.4) and docs/DECKS.md.
//!
//! The rules the new civilizations bring that are not callbacks are hooked where their rule
//! lives: [`on_unit_built`] (`ev_unit_built`), [`glamour_first_strikes`] (`unit_first_strikes`),
//! [`dawn_vow_heal`] (the engagement's result), the civilizations' `unit_promotion` (`unit_init`),
//! the Condottieri's `unit_rush_gold` (the hurry rules) and the Free City Charter's
//! `defender_defense` (the defence terms of an engagement).

use std::sync::OnceLock;

use crate::ffh::cards::world::{self, Birth, NewCity, NewUnit};
use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::SpellTarget;
use ffh_data::*;

/// How far from the caster's borders the Knights of the Dawn found their cities.
pub const DAWN_RANGE: i32 = 6;
/// How far from the caster's borders the Free Companies found their cities.
pub const CONDOTTIERI_RANGE: i32 = 8;
/// How far from the Remnants of Patria the Patrians' second city stands.
pub const PATRIA_RANGE: i32 = 4;
/// The fewest forest plots of the region the Fey Court awakens in.
pub const FEY_FOREST_PLOTS: usize = 12;
/// The Fey Glamour's first strikes where it gives its feature defence (in forest).
pub const GLAMOUR_FIRST_STRIKES: i32 = 2;
/// The level the casters the Patrians train start at.
pub const PATRIAN_CASTER_LEVEL: i32 = 3;
/// A new civilization's attitude modifier toward its founder or its alignment (FFH's scale).
pub const ATTITUDE: i32 = 4;
/// The Free Companies' starting gold.
pub const CONDOTTIERI_GOLD: i32 = 1000;

/// The forest features (the Fey Court's region).
const FORESTS: [&str; 3] = ["FEATURE_FOREST", "FEATURE_FOREST_ANCIENT", "FEATURE_FOREST_NEW"];

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellPatrianAwakening" => spell_patrian_awakening,
        "spellDawnKnights" => spell_dawn_knights,
        "spellFeyCourt" => spell_fey_court,
        "spellCondottieri" => spell_condottieri,
        "spellFreeCityCharter" => spell_free_city_charter,
        "spellMirrorColony" => spell_mirror_colony,
        "spellMirrorGate" => spell_mirror_gate,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        "reqPatrianAwakening" => req_patrian_awakening,
        "reqDawnKnights" => req_dawn_knights,
        "reqFeyCourt" => req_fey_court,
        "reqCondottieri" => req_condottieri,
        "reqFreeCityCharter" => req_free_city_charter,
        "reqMirrorColony" => req_mirror_colony,
        "reqMirrorGate" => req_mirror_gate,
        _ => return None,
    };
    Some(f)
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        "targetsFreeCityCharter" => targets_free_city_charter,
        "targetsMirrorGate" => targets_mirror_gate,
        _ => return None,
    };
    Some(f)
}

// ---------------------------------------------------------------------------------------------
// Where the new cities go.

/// The land plots within `range` plots (`plot_distance`) of (x, y).
fn within(g: &Game, x: i32, y: i32, range: i32) -> Vec<PlotIdx> {
    let mut out = Vec::new();
    for dx in -range..=range {
        for dy in -range..=range {
            if let Some(q) = g.plot_xy(x, y, dx, dy) {
                let (qx, qy) = g.map.xy(q);
                if g.plot_distance(x, y, qx, qy) <= range && !g.plot_is_water(q) {
                    out.push(q);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Two free sites among `candidates` (unowned), or none.
fn two_sites(g: &Game, candidates: &[PlotIdx]) -> Option<Vec<PlotIdx>> {
    let sites = world::best_sites(g, candidates, 2, None, &[]);
    (sites.len() == 2).then_some(sites)
}

/// G12: the Patrians' two sites for `p`. Next to the Remnants of Patria where they lie outside
/// every other civilization's borders (a city cannot stand on them), the second within
/// [`PATRIA_RANGE`] of them (both unowned or `p`'s); else two on the largest unowned region.
pub fn patrian_sites(g: &Game, p: PlayerId) -> Option<Vec<PlotIdx>> {
    if let Some(remnants) = g.d.ty::<ImprovementType>("IMPROVEMENT_REMNANTS_OF_PATRIA") {
        for q in g.map.plot_ids() {
            let plot = &g.map.plots[q];
            if plot.improvement_type != Some(remnants) || plot.owner.is_some_and(|o| o != p) {
                continue;
            }
            let (x, y) = g.map.xy(q);
            let next: Vec<PlotIdx> = within(g, x, y, 1).into_iter().filter(|&t| t != q).collect();
            let Some(&first) = world::best_sites(g, &next, 1, Some(p), &[]).first() else { continue };
            if let Some(&second) = world::best_sites(g, &within(g, x, y, PATRIA_RANGE), 1, Some(p), &[first]).first() {
                return Some(vec![first, second]);
            }
        }
    }
    two_sites(g, &world::largest_region(g, |_, _| true)?)
}

/// P18 and M07: two free sites within `range` plots of `p`'s borders.
pub fn border_sites(g: &Game, p: PlayerId, range: i32) -> Option<Vec<PlotIdx>> {
    two_sites(g, &world::land_near_borders(g, p, range))
}

/// P19: the largest unclaimed forest region, when it has [`FEY_FOREST_PLOTS`] plots or more.
pub fn fey_region(g: &Game) -> Option<Vec<PlotIdx>> {
    let forests: Vec<FeatureType> = FORESTS.iter().filter_map(|n| g.d.ty::<FeatureType>(n)).collect();
    let region = world::largest_region(g, |g, q| g.map.plots[q].feature_type.is_some_and(|f| forests.contains(&f)))?;
    (region.len() >= FEY_FOREST_PLOTS).then_some(region)
}

/// P19: the Fey Court's two sites, in its forest region.
pub fn fey_sites(g: &Game) -> Option<Vec<PlotIdx>> {
    two_sites(g, &fey_region(g)?)
}

/// Mirror Colony: two sites on the largest unclaimed region.
pub fn mirror_sites(g: &Game) -> Option<Vec<PlotIdx>> {
    two_sites(g, &world::largest_region(g, |_, _| true)?)
}

// ---------------------------------------------------------------------------------------------
// The births.

fn unit(d: &GameData, name: &str) -> NewUnit {
    NewUnit::of(d.t::<UnitType>(name))
}

/// A new city at `plot` with `buildings` and `units`.
fn city(d: &GameData, plot: PlotIdx, buildings: &[&str], units: &[&str]) -> NewCity {
    NewCity { plot, buildings: buildings.iter().map(|&b| d.t::<BuildingType>(b)).collect(), units: units.iter().map(|&u| unit(d, u)).collect() }
}

/// `reqPatrianAwakening(caster)`: the Patrians have their two sites.
fn req_patrian_awakening(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    patrian_sites(g, g.units[caster].owner).is_some()
}

/// `spellPatrianAwakening(caster)`: the Patrians, the caster's vassal (+4 attitude toward it), in
/// two cities (their palace gives every city of theirs a Mage Guild), with the Archon.
fn spell_patrian_awakening(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let d = g.d;
    let p = g.units[caster].owner;
    let Some(sites) = patrian_sites(g, p) else { return };
    let mut b = Birth::new(d.t("CIVILIZATION_PATRIANS"), d.t("LEADER_AURELIAN"), p);
    b.found = vec![
        city(d, sites[0], &[], &["UNIT_MAGE", "UNIT_ADEPT", "UNIT_LONGBOWMAN", "UNIT_WORKER"]),
        city(d, sites[1], &[], &["UNIT_ADEPT", "UNIT_LONGBOWMAN"]),
    ];
    b.hero = Some(unit(d, "UNIT_ARCHON"));
    b.vassal = true;
    b.founder_attitude = ATTITUDE;
    let np = world::birth(g, &b);
    for u in g.players[np].units.clone() {
        patrian_caster(g, u);
    }
}

/// `reqDawnKnights(caster)`: there are two free sites within [`DAWN_RANGE`] of the caster's
/// borders.
fn req_dawn_knights(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    border_sites(g, g.units[caster].owner, DAWN_RANGE).is_some()
}

/// `spellDawnKnights(caster)`: the Knights of the Dawn, the caster's vassal (+4 attitude toward
/// good civilizations), of the Order, in two cities (the first with a Wall of Stone and a Temple
/// of the Order), with four Dawn Knights and the Grand Master.
fn spell_dawn_knights(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let d = g.d;
    let p = g.units[caster].owner;
    let Some(sites) = border_sites(g, p, DAWN_RANGE) else { return };
    let mut b = Birth::new(d.t("CIVILIZATION_DAWN_KNIGHTS"), d.t("LEADER_HALRIC"), p);
    b.found = vec![
        city(d, sites[0], &["BUILDING_WALL_OF_STONE", "BUILDING_TEMPLE_OF_THE_ORDER"], &["UNIT_DAWN_KNIGHT", "UNIT_DAWN_KNIGHT"]),
        city(d, sites[1], &[], &["UNIT_DAWN_KNIGHT", "UNIT_DAWN_KNIGHT"]),
    ];
    b.hero = Some(unit(d, "UNIT_GRAND_MASTER"));
    b.state_religion = Some(d.t("RELIGION_THE_ORDER"));
    b.vassal = true;
    b.alignment_attitude = Some((AlignmentType::ALIGNMENT_GOOD, ATTITUDE));
    world::birth(g, &b);
}

/// `reqFeyCourt(caster)`: an unclaimed forest region of [`FEY_FOREST_PLOTS`] plots or more has
/// two free sites.
fn req_fey_court(g: &Game, _caster: UnitId, _call: &PyCall) -> bool {
    fey_sites(g).is_some()
}

/// `spellFeyCourt(caster)`: the Fey Court, the caster's vassal, of the Fellowship of Leaves, in
/// two cities of the forest region, with Satyrs, Fawns, Treants and the Erlking.
fn spell_fey_court(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let d = g.d;
    let p = g.units[caster].owner;
    let Some(sites) = fey_sites(g) else { return };
    let mut b = Birth::new(d.t("CIVILIZATION_FEY_COURT"), d.t("LEADER_AINE"), p);
    b.found = vec![
        city(d, sites[0], &[], &["UNIT_SATYR", "UNIT_FAWN", "UNIT_TREANT"]),
        city(d, sites[1], &[], &["UNIT_SATYR", "UNIT_FAWN", "UNIT_TREANT"]),
    ];
    b.hero = Some(unit(d, "UNIT_ERLKING"));
    b.state_religion = Some(d.t("RELIGION_FELLOWSHIP_OF_LEAVES"));
    b.vassal = true;
    world::birth(g, &b);
}

/// `reqCondottieri(caster)`: there are two free sites within [`CONDOTTIERI_RANGE`] of the
/// caster's borders.
fn req_condottieri(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    border_sites(g, g.units[caster].owner, CONDOTTIERI_RANGE).is_some()
}

/// `spellCondottieri(caster)`: the Free Companies, the caster's vassal, in two cities, with six
/// Mercenaries, the Captain-General and 1000 gold.
fn spell_condottieri(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let d = g.d;
    let p = g.units[caster].owner;
    let Some(sites) = border_sites(g, p, CONDOTTIERI_RANGE) else { return };
    let mut b = Birth::new(d.t("CIVILIZATION_FREE_COMPANIES"), d.t("LEADER_VESK"), p);
    let mercenaries = ["UNIT_MERCENARY"; 3];
    b.found = vec![city(d, sites[0], &[], &mercenaries), city(d, sites[1], &[], &mercenaries)];
    b.hero = Some(unit(d, "UNIT_CAPTAIN_GENERAL"));
    b.gold = CONDOTTIERI_GOLD;
    b.vassal = true;
    world::birth(g, &b);
}

/// `targetsFreeCityCharter(caster)`: the caster's cities but its capital.
fn targets_free_city_charter(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let p = g.units[caster].owner;
    g.players[p].cities.iter().copied().filter(|&c| !g.city_is_capital(c)).map(SpellTarget::City).collect()
}

/// `reqFreeCityCharter(caster)`: always (the target is its own test).
fn req_free_city_charter(_g: &Game, _caster: UnitId, _call: &PyCall) -> bool {
    true
}

/// `spellFreeCityCharter(caster)` at a city: it becomes the Free City, a civilization of its own
/// (+4 attitude toward the caster) that keeps its buildings and garrison, with a Wall of Stone and
/// the Charter (three Sentinels, +50% defence for the units fighting in it).
fn spell_free_city_charter(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let d = g.d;
    let Some(SpellTarget::City(c)) = g.spell_target else { return };
    let p = g.units[caster].owner;
    if !g.cities.contains(c) || g.cities[c].owner != p || g.city_is_capital(c) {
        return;
    }
    let name = d.text.name(&g.cities[c].name).to_string();
    let mut b = Birth::new(d.t("CIVILIZATION_FREE_CITY"), d.t("LEADER_BURGOMASTER"), p);
    b.release = vec![c];
    b.keep = Some(caster);
    b.founder_attitude = ATTITUDE;
    let np = world::birth(g, &b);
    g.setup.players[np.idx()].civ_description = format!("Free City of {name}");
    let Some(&city) = g.players[np].cities.first() else { return };
    for building in ["BUILDING_WALL_OF_STONE", "BUILDING_FREE_CITY_CHARTER"] {
        let building = g.city_building_variant(city, d.t::<BuildingType>(building));
        g.city_set_num_real_building(city, building, 1);
    }
}

// ---------------------------------------------------------------------------------------------
// Mirror Colony.

fn mirror_gate(d: &GameData) -> BuildingType {
    d.t::<BuildingType>("BUILDING_MIRROR_GATE")
}

/// `reqMirrorColony(caster)`: the caster's owner has a capital and the largest unclaimed region
/// two free sites.
fn req_mirror_colony(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    g.player_get_capital_city(g.units[caster].owner).is_some() && mirror_sites(g).is_some()
}

/// `spellMirrorColony(caster)`: two cities of the caster's own on the largest unclaimed region; the
/// first and the capital get the Mirror Gate.
fn spell_mirror_colony(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let d = g.d;
    let p = g.units[caster].owner;
    let (Some(capital), Some(sites)) = (g.player_get_capital_city(p), mirror_sites(g)) else { return };
    let mut gates = vec![capital];
    for (i, site) in sites.into_iter().enumerate() {
        let (x, y) = g.map.xy(site);
        let c = g.player_init_city(p, x, y, true, true);
        if i == 0 {
            gates.push(c);
        }
    }
    for c in gates {
        let gate = g.city_building_variant(c, mirror_gate(d));
        g.city_set_num_real_building(c, gate, 1);
    }
}

/// The city with a Mirror Gate of `u`'s owner that `u`, a land unit, stands in.
fn gate_city(g: &Game, u: UnitId) -> Option<CityId> {
    if g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND {
        return None;
    }
    let c = g.map.plots[g.unit_plot_opt(u)?].plot_city?;
    (g.cities[c].owner == g.units[u].owner && g.city_num_building(c, mirror_gate(g.d)) > 0).then_some(c)
}

/// `reqMirrorGate(caster)`: a land unit in a city of its owner's with a Mirror Gate.
fn req_mirror_gate(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    gate_city(g, caster).is_some()
}

/// `targetsMirrorGate(caster)`: its owner's other cities with a Mirror Gate and no enemy unit.
fn targets_mirror_gate(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let Some(from) = gate_city(g, caster) else { return Vec::new() };
    let p = g.units[caster].owner;
    let gate = mirror_gate(g.d);
    g.players[p]
        .cities
        .iter()
        .copied()
        .filter(|&c| c != from && g.city_num_building(c, gate) > 0 && !g.plot_is_visible_enemy_unit(g.city_plot(c), p))
        .map(SpellTarget::City)
        .collect()
}

/// `spellMirrorGate(caster)` at a city: the caster steps through to it, its moves spent.
fn spell_mirror_gate(g: &mut Game, caster: UnitId, call: &PyCall) {
    let Some(SpellTarget::City(c)) = g.spell_target else { return };
    if !g.cities.contains(c) || !targets_mirror_gate(g, caster, call).contains(&SpellTarget::City(c)) {
        return;
    }
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    g.unit_set_xy(caster, x, y, false);
    if g.units.contains(caster) {
        g.unit_finish_moves(caster);
    }
}

// ---------------------------------------------------------------------------------------------
// The new civilizations' rules.

/// G12: a caster (an adept-combat unit, not a hero) of the Patrians' starts at
/// [`PATRIAN_CASTER_LEVEL`] (with the experience of that level) with Channeling II.
fn patrian_caster(g: &mut Game, u: UnitId) {
    let d = g.d;
    if g.unit_get_unit_combat_type(u) != d.ty::<UnitCombatType>("UNITCOMBAT_ADEPT") || g.unit_is_hero(u) {
        return;
    }
    for name in ["PROMOTION_CHANNELING1", "PROMOTION_CHANNELING2"] {
        g.unit_set_has_promotion(u, d.t::<PromotionType>(name), true);
    }
    if g.units[u].level < PATRIAN_CASTER_LEVEL {
        g.unit_set_level(u, PATRIAN_CASTER_LEVEL);
        let xp = g.py_get_experience_needed(PATRIAN_CASTER_LEVEL - 1, g.units[u].owner);
        if g.units[u].experience < xp {
            g.unit_set_experience(u, xp, -1);
        }
    }
}

/// `ev_unit_built`: a caster the Patrians train starts at level 3 with Channeling II.
pub fn on_unit_built(g: &mut Game, _c: CityId, u: UnitId) {
    if !g.units.contains(u) {
        return;
    }
    let civ = g.player_get_civilization_type(g.units[u].owner);
    if civ.is_some() && civ == g.d.ty::<CivilizationType>("CIVILIZATION_PATRIANS") {
        patrian_caster(g, u);
    }
}

/// P19: the first strikes unit `u` gets from the Fey Glamour where it stands: [`GLAMOUR_FIRST_STRIKES`]
/// on a plot whose feature the Glamour gives defence in (forest), else 0.
pub fn glamour_first_strikes(g: &Game, u: UnitId) -> i32 {
    static GLAMOUR: OnceLock<Option<PromotionType>> = OnceLock::new();
    let Some(glamour) = *GLAMOUR.get_or_init(|| g.d.ty::<PromotionType>("PROMOTION_FEY_GLAMOUR")) else { return 0 };
    if !g.units[u].has_promotion[glamour] {
        return 0;
    }
    let Some(plot) = g.unit_plot_opt(u) else { return 0 };
    match g.map.plots[plot].feature_type {
        Some(f) if g.d.promotions[glamour].feature_defense_percent[f] > 0 => GLAMOUR_FIRST_STRIKES,
        _ => 0,
    }
}

/// P18: after a battle `winners` (the winning side's survivors) won against units of `losers`
/// (the other side's owners), each winner with the Dawn Vow heals fully when a loser is of an
/// evil civilization.
pub fn dawn_vow_heal(g: &mut Game, winners: &[UnitId], losers: &[PlayerId]) {
    static VOW: OnceLock<Option<PromotionType>> = OnceLock::new();
    let Some(vow) = *VOW.get_or_init(|| g.d.ty::<PromotionType>("PROMOTION_DAWN_VOW")) else { return };
    if !losers.iter().any(|&o| g.players[o].alignment == Some(AlignmentType::ALIGNMENT_EVIL)) {
        return;
    }
    for &u in winners {
        if g.units.contains(u) && g.units[u].has_promotion[vow] && g.units[u].damage > 0 {
            g.unit_set_damage(u, 0, None);
        }
    }
}
