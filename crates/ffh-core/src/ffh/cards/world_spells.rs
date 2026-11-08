//! Batch B3, the card world spells (docs/CARDS.md, section 5; the specs are docs/DECKS_ROUND8.md
//! with the C.4 changes): ten great-person cards whose content is one world spell each, cast once a
//! game by any unit of their owner (`SpellInfo::once`).
//!
//! Seven are cast at a distance (`target`, `py_targets`; docs/CARDS.md, section 3). Each of those
//! has a predicate for one target (`*_ok`): its `targets*` callback lists what passes it, and its
//! result checks the target it is given again, so a target that stopped being valid does nothing.
//!
//! | Card | Target | Effect |
//! |---|---|---|
//! | G07 Astral Leap | plot | the units in the caster's city move to a visible land plot |
//! | G10 Seal of Binding | unit | an enemy world unit or hero cannot move for 20 turns |
//! | B08 Masque of the Red Death | city | an enemy city loses half its population |
//! | M08 Purchase Charter | city | a small city of a civilization at peace is bought |
//! | B06 Cultural Annexation | city | a foreign city where our culture is strong joins us |
//! | M09 Transmutation Charter | choice | a resource of ours becomes another of its group |
//! | M03 Bazaar of Wonders | city | a city of ours completes the world wonder it is building |
//! | P16 Call of the Faithful | | our state religion's missing heroes join us |
//! | M19 Grand Order | | every city of ours completes (or advances) its current build |
//! | M18 Mammon's Bargain | | 1500 gold, and +10 to the Armageddon counter |

use super::world::{counter_change_for_points, within_range_of_borders};
use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::SpellTarget;
use crate::utils::{is_limited_wonder_class, is_world_unit_class, is_world_wonder_class};
use ffh_data::*;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellAstralLeap" => spell_astral_leap,
        "spellSealOfBinding" => spell_seal_of_binding,
        "spellMasqueOfTheRedDeath" => spell_masque,
        "spellPurchaseCharter" => spell_purchase_charter,
        "spellCulturalAnnexation" => spell_cultural_annexation,
        "spellTransmutationCharter" => spell_transmutation_charter,
        "spellBazaarOfWonders" => spell_bazaar_of_wonders,
        "spellCallOfTheFaithful" => spell_call_of_the_faithful,
        "spellGrandOrder" => spell_grand_order,
        "spellMammonsBargain" => spell_mammons_bargain,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        "reqCallOfTheFaithful" => req_call_of_the_faithful,
        "reqGrandOrder" => req_grand_order,
        _ => return None,
    };
    Some(f)
}

/// `py_targets` functions.
/// The batch's `py_target_label` callbacks: a Transmutation Charter option's name, a city's price
/// (docs/CARDS.md, section 3).
pub fn find_target_label(name: &str) -> Option<TargetLabelFn> {
    let f: TargetLabelFn = match name {
        "labelTransmutationCharter" => label_transmutation_charter,
        "labelPurchaseCharter" => label_purchase_charter,
        "labelBazaarOfWonders" => label_bazaar,
        _ => return None,
    };
    Some(f)
}

fn label_transmutation_charter(g: &Game, _caster: UnitId, target: SpellTarget, _: &PyCall) -> String {
    match target {
        SpellTarget::Choice(choice) => transmutation_charter_choice_label(g, choice),
        _ => String::new(),
    }
}

fn label_purchase_charter(g: &Game, _caster: UnitId, target: SpellTarget, _: &PyCall) -> String {
    match target {
        SpellTarget::City(c) if g.cities.contains(c) => format!("Price: {} gold", purchase_charter_price(g, c)),
        _ => String::new(),
    }
}

fn label_bazaar(g: &Game, _caster: UnitId, target: SpellTarget, _: &PyCall) -> String {
    match target {
        SpellTarget::City(c) if g.cities.contains(c) => format!("Price: {} gold", bazaar_price(g, c)),
        _ => String::new(),
    }
}

pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        "targetsAstralLeap" => targets_astral_leap,
        "targetsSealOfBinding" => targets_seal_of_binding,
        "targetsMasqueOfTheRedDeath" => targets_masque,
        "targetsPurchaseCharter" => targets_purchase_charter,
        "targetsCulturalAnnexation" => targets_cultural_annexation,
        "targetsTransmutationCharter" => targets_transmutation_charter,
        "targetsBazaarOfWonders" => targets_bazaar_of_wonders,
        _ => return None,
    };
    Some(f)
}

// ---------------------------------------------------------------------------------------------
// Shared

/// Seal of Binding: the turns its target cannot move.
pub const SEAL_TURNS: i32 = 20;
/// Purchase Charter: the largest city it buys.
pub const PURCHASE_MAX_POPULATION: i32 = 5;
/// Purchase Charter: how far from our borders the city may be.
pub const PURCHASE_RANGE: i32 = 6;
/// Purchase Charter: gold per citizen.
pub const PURCHASE_GOLD_PER_POPULATION: i32 = 100;
/// Purchase Charter: gold per hammer of the city's buildings (docs/DECKS_ROUND8.md C.3, rule 3).
pub const PURCHASE_GOLD_PER_HAMMER: i32 = 2;
/// Purchase Charter and Cultural Annexation: what the city's owner thinks of the taker.
pub const CITY_TAKEN_ATTITUDE: i32 = -4;
/// Cultural Annexation: how far from our borders the city may be.
pub const ANNEX_RANGE: i32 = 3;
/// Cultural Annexation: our share of the culture on the city's plot, in percent.
pub const ANNEX_CULTURE_PERCENT: i64 = 30;
/// Bazaar of Wonders: gold per hammer the wonder still needs.
pub const BAZAAR_GOLD_PER_HAMMER: i32 = 3;
/// Grand Order: the turns of a city's own production it completes at most.
pub const GRAND_ORDER_TURNS: i32 = 10;
/// Mammon's Bargain: the gold.
pub const MAMMON_GOLD: i32 = 1500;
/// Mammon's Bargain: the rise of the Armageddon counter, in the points of its percentage
/// (`get_global_counter`, which the Armageddon deck's slots use).
pub const MAMMON_COUNTER: i32 = 10;

/// The city of its owner that unit `u` stands in.
pub fn own_city_at(g: &Game, u: UnitId) -> Option<CityId> {
    let plot = g.unit_plot_opt(u)?;
    let c = g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c))?;
    (g.cities[c].owner == g.units[u].owner).then_some(c)
}

/// Is city `c` the holy city of a religion?
pub fn is_holy_city(g: &Game, c: CityId) -> bool {
    g.d.religions.keys().any(|r| g.city_is_holy_city(c, r))
}

/// The cities of every player that pass `ok`, in player and city-list order.
fn city_targets(g: &Game, ok: impl Fn(CityId) -> bool) -> Vec<SpellTarget> {
    let mut out = Vec::new();
    for p in g.player_ids() {
        for &c in &g.players[p].cities {
            if g.cities.contains(c) && ok(c) {
                out.push(SpellTarget::City(c));
            }
        }
    }
    out
}

/// The city target being cast at, if it is still a city.
fn target_city(g: &Game) -> Option<CityId> {
    match g.spell_target {
        Some(SpellTarget::City(c)) if g.cities.contains(c) => Some(c),
        _ => None,
    }
}

/// `p` loses `CITY_TAKEN_ATTITUDE` toward `taker` (a city taken without war). The AI reads it
/// as one of its relation's terms (`ai_attitude_extra`).
fn remember_city_taken(g: &mut Game, p: PlayerId, taker: PlayerId) {
    if !g.player_is_barbarian(p) {
        g.players[p].ai_attitude_extra[taker] += CITY_TAKEN_ATTITUDE;
    }
}

// ---------------------------------------------------------------------------------------------
// G07 Astral Leap: cast by a unit in one of its owner's cities, at a land plot its team sees with
// no unit or city of another team. The units of the caster's owner on the city's plot that can
// enter that plot (not as an attack) and are not held move there, with no moves left.

/// Does unit `u` of `owner` leap to `to`: not held, and able to enter it without attacking?
fn leaps(g: &Game, u: UnitId, owner: PlayerId, to: PlotIdx) -> bool {
    let unit = &g.units[u];
    unit.owner == owner && !unit.death_delay && g.unit_get_immobile_timer(u) == 0 && g.unit_can_move_into(u, to, false, false, true)
}

/// The units Astral Leap cast by `caster` moves to `to`, in plot order: those of its owner in its
/// city that leap there.
pub fn astral_leap_movers(g: &Game, caster: UnitId, to: PlotIdx) -> Vec<UnitId> {
    let Some(c) = own_city_at(g, caster) else { return Vec::new() };
    let owner = g.units[caster].owner;
    g.map.plots[g.city_plot(c)].units.iter().copied().filter(|&u| leaps(g, u, owner, to)).collect()
}

/// May `caster` cast Astral Leap at `to`?
pub fn astral_leap_ok(g: &Game, caster: UnitId, to: PlotIdx) -> bool {
    let team = g.unit_get_team(caster);
    if g.plot_is_water(to) || !g.plot_is_visible(to, team) {
        return false;
    }
    let plot = &g.map.plots[to];
    if plot.plot_city.filter(|&c| g.cities.contains(c)).is_some_and(|c| g.city_team(c) != team) {
        return false;
    }
    if plot.units.iter().any(|&o| g.unit_get_team(o) != team) {
        return false;
    }
    let Some(c) = own_city_at(g, caster) else { return false };
    let owner = g.units[caster].owner;
    g.map.plots[g.city_plot(c)].units.iter().any(|&u| leaps(g, u, owner, to))
}

/// `targetsAstralLeap(caster)`: the plots it may leap to, in plot order.
fn targets_astral_leap(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    if own_city_at(g, caster).is_none() {
        return Vec::new();
    }
    g.map.plot_ids().filter(|&q| astral_leap_ok(g, caster, q)).map(SpellTarget::Plot).collect()
}

/// `spellAstralLeap(caster)`: the city's units move to the target plot, in one group, with no moves
/// left.
fn spell_astral_leap(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Plot(to)) = g.spell_target else { return };
    if !astral_leap_ok(g, caster, to) {
        return;
    }
    let movers = astral_leap_movers(g, caster, to);
    let (x, y) = g.map.xy(to);
    let mut group = None;
    for &u in &movers {
        if !g.units.contains(u) {
            continue;
        }
        g.unit_set_xy(u, x, y, false);
        if !g.units.contains(u) || g.unit_plot_opt(u) != Some(to) {
            continue;
        }
        g.unit_finish_moves(u);
        match group {
            None => group = g.units[u].group,
            Some(grp) => g.unit_join_group(u, Some(grp), true),
        }
    }
    let owner = g.units.get(caster).map(|u| u.owner);
    let turn = g.game_turn();
    g.log.msg(turn, owner, || format!("Astral Leap: {} units to ({x}, {y})", movers.len()));
}

// ---------------------------------------------------------------------------------------------
// G10 Seal of Binding: a world unit or a hero of a team at war with the caster's (the barbarians
// included), which its team sees, cannot move for 20 turns: its immobile timer, which also keeps
// it from attacking and casting but not from defending.

/// May `caster` cast Seal of Binding at unit `u`?
pub fn seal_ok(g: &Game, caster: UnitId, u: UnitId) -> bool {
    if !g.units.contains(u) {
        return false;
    }
    let Some(plot) = g.unit_plot_opt(u) else { return false };
    let team = g.unit_get_team(caster);
    let their = g.unit_get_team(u);
    their != team
        && g.at_war(team, their)
        && g.plot_is_visible(plot, team)
        && !g.unit_is_invisible(u, team, false)
        && (is_world_unit_class(g.d, g.unit_get_unit_class_type(u)) || g.unit_is_hero(u))
        && g.unit_get_immobile_timer(u) < SEAL_TURNS
}

/// `targetsSealOfBinding(caster)`: the units it may hold, in player and unit-list order.
fn targets_seal_of_binding(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let mut out = Vec::new();
    for p in g.player_ids() {
        for &u in &g.players[p].units {
            if seal_ok(g, caster, u) {
                out.push(SpellTarget::Unit(u));
            }
        }
    }
    out
}

/// `spellSealOfBinding(caster)`: the target cannot move for `SEAL_TURNS` turns.
fn spell_seal_of_binding(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Unit(u)) = g.spell_target else { return };
    if !seal_ok(g, caster, u) {
        return;
    }
    g.unit_set_immobile_timer(u, SEAL_TURNS);
    let turn = g.game_turn();
    let owner = g.units[caster].owner;
    let name = g.d.text.name(&g.d.units[g.units[u].unit_type].base.description).to_string();
    g.log.msg(turn, Some(owner), || format!("Seal of Binding: {name} cannot move for {SEAL_TURNS} turns"));
}

// ---------------------------------------------------------------------------------------------
// B08 Masque of the Red Death: a city its team sees, of a civilization at war with it (not the
// barbarians), with population 2 or more, loses half its population (rounded down).

/// May `caster` cast Masque of the Red Death at city `c`?
pub fn masque_ok(g: &Game, caster: UnitId, c: CityId) -> bool {
    let team = g.unit_get_team(caster);
    let owner = g.cities[c].owner;
    let their = g.players[owner].team;
    !g.player_is_barbarian(owner) && their != team && g.at_war(team, their) && g.plot_is_visible(g.city_plot(c), team) && g.cities[c].population >= 2
}

/// `targetsMasqueOfTheRedDeath(caster)`.
fn targets_masque(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    city_targets(g, |c| masque_ok(g, caster, c))
}

/// `spellMasqueOfTheRedDeath(caster)`: the city loses half its population.
fn spell_masque(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(c) = target_city(g) else { return };
    if !masque_ok(g, caster, c) {
        return;
    }
    let loss = g.cities[c].population / 2;
    g.city_change_population(c, -loss);
    let turn = g.game_turn();
    let owner = g.units[caster].owner;
    let name = g.d.text.name(&g.cities[c].name).to_string();
    g.log.msg(turn, Some(owner), || format!("Masque of the Red Death: {name} loses {loss} population"));
}

// ---------------------------------------------------------------------------------------------
// M08 Purchase Charter: a city of a civilization the caster's team has met and is at peace with --
// not its capital, a holy city or a city with a world wonder; population 5 or less; within 6 plots
// of the caster's owner's borders; its plot revealed to the caster's team -- is bought for 100 gold
// per citizen plus 2 gold per hammer of its buildings' cost. It changes hands as a traded city,
// with its buildings; its owner gets the gold and -4 attitude toward the buyer.

/// What Purchase Charter costs for city `c`: 100 gold per citizen, and 2 gold per hammer of the
/// cost of each of its buildings (its owner's cost; buildings that cannot be built count nothing).
pub fn purchase_charter_price(g: &Game, c: CityId) -> i32 {
    let d = g.d;
    let owner = g.cities[c].owner;
    let mut price = PURCHASE_GOLD_PER_POPULATION * g.cities[c].population;
    for b in d.buildings.keys() {
        let n = g.cities[c].num_real_building[b];
        if n > 0 && d.buildings[b].production_cost > 0 {
            price += PURCHASE_GOLD_PER_HAMMER * n * g.player_get_production_needed_building(owner, b);
        }
    }
    price
}

/// May city `c` be bought with Purchase Charter by `caster`'s owner, the price aside?
pub fn purchase_charter_eligible(g: &Game, caster: UnitId, c: CityId) -> bool {
    let p = g.units[caster].owner;
    let team = g.players[p].team;
    let owner = g.cities[c].owner;
    let their = g.players[owner].team;
    their != team
        && !g.player_is_barbarian(owner)
        && g.teams[team].has_met[their]
        && !g.at_war(team, their)
        && g.map.plots[g.city_plot(c)].revealed[team]
        && !g.city_is_capital(c)
        && !is_holy_city(g, c)
        && g.cities[c].num_world_wonders == 0
        && g.cities[c].population <= PURCHASE_MAX_POPULATION
        && within_range_of_borders(g, p, g.city_plot(c), PURCHASE_RANGE)
}

/// May `caster` cast Purchase Charter at city `c` (eligible, and its owner has the gold)?
pub fn purchase_charter_ok(g: &Game, caster: UnitId, c: CityId) -> bool {
    purchase_charter_eligible(g, caster, c) && g.players[g.units[caster].owner].gold >= purchase_charter_price(g, c)
}

/// `targetsPurchaseCharter(caster)`.
fn targets_purchase_charter(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    city_targets(g, |c| purchase_charter_ok(g, caster, c))
}

/// `spellPurchaseCharter(caster)`: the gold goes to the city's owner, the city to the buyer.
fn spell_purchase_charter(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(c) = target_city(g) else { return };
    if !purchase_charter_ok(g, caster, c) {
        return;
    }
    let p = g.units[caster].owner;
    let owner = g.cities[c].owner;
    let price = purchase_charter_price(g, c);
    let name = g.d.text.name(&g.cities[c].name).to_string();
    g.players[p].gold -= price;
    g.players[owner].gold += price;
    remember_city_taken(g, owner, p);
    g.player_acquire_city(p, c, false, true, true);
    let turn = g.game_turn();
    g.log.msg(turn, Some(p), || format!("Purchase Charter: {name} bought for {price} gold"));
    g.log.msg(turn, Some(owner), || format!("Purchase Charter: {name} sold for {price} gold"));
}

// ---------------------------------------------------------------------------------------------
// B06 Cultural Annexation: a city of another team (the barbarians included), not a capital, whose
// plot the caster's team has revealed, within 3 plots of the caster's owner's borders, where the
// owner's culture is at least 30% of the culture on the city's plot, joins the caster's owner as
// a culture flip does (with its buildings). It declares no war; its owner gets -4 attitude toward
// the taker.

/// May `caster` cast Cultural Annexation at city `c`?
pub fn cultural_annexation_ok(g: &Game, caster: UnitId, c: CityId) -> bool {
    let p = g.units[caster].owner;
    let team = g.players[p].team;
    let plot = g.city_plot(c);
    if g.city_team(c) == team || g.city_is_capital(c) || !g.map.plots[plot].revealed[team] {
        return false;
    }
    if !within_range_of_borders(g, p, plot, ANNEX_RANGE) {
        return false;
    }
    let ours = g.map.plots[plot].culture[p] as i64;
    let total = g.plot_count_total_culture(plot) as i64;
    ours > 0 && ours * 100 >= ANNEX_CULTURE_PERCENT * total
}

/// `targetsCulturalAnnexation(caster)`.
fn targets_cultural_annexation(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    city_targets(g, |c| cultural_annexation_ok(g, caster, c))
}

/// `spellCulturalAnnexation(caster)`: the city flips to the caster's owner.
fn spell_cultural_annexation(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(c) = target_city(g) else { return };
    if !cultural_annexation_ok(g, caster, c) {
        return;
    }
    let p = g.units[caster].owner;
    let owner = g.cities[c].owner;
    let name = g.d.text.name(&g.cities[c].name).to_string();
    remember_city_taken(g, owner, p);
    // A culture flip's transfer (`plot_set_owner` and FfH's Convert City spells): neither a
    // conquest nor a trade.
    g.player_acquire_city(p, c, false, false, true);
    let turn = g.game_turn();
    g.log.msg(turn, Some(p), || format!("Cultural Annexation: {name} joins you"));
    g.log.msg(turn, Some(owner), || format!("Cultural Annexation: {name} is lost"));
}

// ---------------------------------------------------------------------------------------------
// M09 Transmutation Charter: a resource on a plot of the caster's owner becomes another resource
// of its group. The groups come from the data, among the resources the map places
// (`const_appearance` > 0): strategic (`BONUSCLASS_RUSH`), luxury (happiness), food (health); mana,
// Marble and the resources buildings make belong to none. A water resource becomes a water
// resource and a land one a land one; the new resource is one the owner's team can see (its
// reveal tech). The target is a choice: the plot and the new resource, `transmutation_choice`.

/// The groups of Transmutation Charter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceGroup {
    /// `BONUSCLASS_RUSH`: Copper, Gunpowder, Horse, Iron, Mithril, Reagents.
    Strategic,
    /// Happiness: Cotton, Dye, Fur, Gems, Gold, Ivory, Pearls, Silk, Sugar, Whale, Wine.
    Luxury,
    /// Health: Banana, Clam, Corn, Cow, Crab, Deer, Fish, Pig, Rice, Sheep, Wheat.
    Food,
}

/// The Transmutation Charter group of resource `b` (`None`: it has none).
pub fn resource_group(d: &GameData, b: BonusType) -> Option<ResourceGroup> {
    let info = &d.bonuses[b];
    if info.const_appearance <= 0 {
        return None;
    }
    match info.bonus_class_type.map(|c| d.bonus_classes[c].base.type_.as_str()) {
        Some("BONUSCLASS_MANA" | "BONUSCLASS_RAWMANA") => None,
        Some("BONUSCLASS_RUSH") => Some(ResourceGroup::Strategic),
        _ if info.happiness > 0 => Some(ResourceGroup::Luxury),
        _ if info.health > 0 => Some(ResourceGroup::Food),
        _ => None,
    }
}

/// Is `b` a water resource (one of its terrains is water)?
fn water_resource(d: &GameData, b: BonusType) -> bool {
    d.terrains.keys().any(|t| d.bonuses[b].terrain[t] && d.terrains[t].water)
}

/// The choice that turns the resource on `plot` into `bonus`.
pub fn transmutation_choice(d: &GameData, plot: PlotIdx, bonus: BonusType) -> i32 {
    plot.idx() as i32 * d.bonuses.len() as i32 + bonus.idx() as i32
}

/// The plot and the new resource of a Transmutation Charter choice.
pub fn transmutation_decode(g: &Game, choice: i32) -> Option<(PlotIdx, BonusType)> {
    let n = g.d.bonuses.len() as i32;
    if choice < 0 || (choice / n) as usize >= g.map.plots.len() {
        return None;
    }
    Some((PlotIdx::from_usize((choice / n) as usize), BonusType::from_usize((choice % n) as usize)))
}

/// May `caster` turn the resource on `plot` into `new` with Transmutation Charter?
pub fn transmutation_ok(g: &Game, caster: UnitId, plot: PlotIdx, new: BonusType) -> bool {
    let d = g.d;
    let p = g.units[caster].owner;
    let team = g.players[p].team;
    if g.map.plots[plot].owner != Some(p) {
        return false;
    }
    let Some(old) = g.plot_get_bonus_type(plot, Some(team)) else { return false };
    let Some(group) = resource_group(d, old) else { return false };
    new != old
        && resource_group(d, new) == Some(group)
        && water_resource(d, new) == g.plot_is_water(plot)
        && g.team_is_has_tech(team, d.bonuses[new].tech_reveal)
}

/// `targetsTransmutationCharter(caster)`: the choices, by plot, then by resource.
fn targets_transmutation_charter(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let d = g.d;
    let p = g.units[caster].owner;
    let mut out = Vec::new();
    for q in g.map.plot_ids() {
        if g.map.plots[q].owner != Some(p) || g.map.plots[q].bonus_type.is_none() {
            continue;
        }
        for b in d.bonuses.keys() {
            if transmutation_ok(g, caster, q, b) {
                out.push(SpellTarget::Choice(transmutation_choice(d, q, b)));
            }
        }
    }
    out
}

/// The label of a Transmutation Charter choice: `Copper (4, 5) into Iron`.
pub fn transmutation_charter_choice_label(g: &Game, choice: i32) -> String {
    let d = g.d;
    let Some((plot, new)) = transmutation_decode(g, choice) else { return String::new() };
    let (x, y) = g.map.xy(plot);
    let old = g.map.plots[plot].bonus_type.map_or("", |b| d.text.name(&d.bonuses[b].base.description));
    format!("{old} ({x}, {y}) into {}", d.text.name(&d.bonuses[new].base.description))
}

/// `spellTransmutationCharter(caster)`: the plot's resource becomes the chosen one.
fn spell_transmutation_charter(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Choice(choice)) = g.spell_target else { return };
    let Some((plot, new)) = transmutation_decode(g, choice) else { return };
    if !transmutation_ok(g, caster, plot, new) {
        return;
    }
    let label = transmutation_charter_choice_label(g, choice);
    g.plot_set_bonus_type(plot, Some(new));
    let turn = g.game_turn();
    let owner = g.units[caster].owner;
    g.log.msg(turn, Some(owner), || format!("Transmutation Charter: {label}"));
}

// ---------------------------------------------------------------------------------------------
// M03 Bazaar of Wonders: a city of the caster's owner completes the world wonder at the head of its
// queue at once, for 3 gold per hammer the wonder still needs there. The wonder costs hammers, the
// city can still build it (its prerequisites), and it is no victory wonder: no building class with
// a victory threshold (the Tower of Mastery and the four towers it needs, the Altar of the
// Luonnotar's final stage) nor an earlier stage of the Altar.

/// The Altar of the Luonnotar's stages (the last one has the victory threshold).
const ALTAR_STAGES: [&str; 7] = [
    "BUILDINGCLASS_ALTAR_OF_THE_LUONNOTAR",
    "BUILDINGCLASS_ALTAR_OF_THE_LUONNOTAR_ANOINTED",
    "BUILDINGCLASS_ALTAR_OF_THE_LUONNOTAR_BLESSED",
    "BUILDINGCLASS_ALTAR_OF_THE_LUONNOTAR_CONSECRATED",
    "BUILDINGCLASS_ALTAR_OF_THE_LUONNOTAR_DIVINE",
    "BUILDINGCLASS_ALTAR_OF_THE_LUONNOTAR_EXALTED",
    "BUILDINGCLASS_ALTAR_OF_THE_LUONNOTAR_FINAL",
];

/// Is `class` a victory wonder (a victory threshold, or a stage of the Altar of the Luonnotar)?
pub fn is_victory_wonder(d: &GameData, class: BuildingClassType) -> bool {
    let info = &d.building_classes[class];
    info.victory_threshold.iter().any(|&t| t > 0) || ALTAR_STAGES.contains(&info.base.type_.as_str())
}

/// The world wonder Bazaar of Wonders would complete in city `c`: the head of its queue, if it is
/// one it may buy.
pub fn bazaar_wonder(g: &Game, c: CityId) -> Option<BuildingType> {
    let d = g.d;
    let order = g.cities[c].head_order()?;
    if order.order_type != OrderType::ORDER_CONSTRUCT || order.data1 < 0 {
        return None;
    }
    let b = BuildingType::from_usize(order.data1 as usize);
    let class = d.buildings[b].building_class_type?;
    let ok = is_world_wonder_class(d, class) && d.buildings[b].production_cost > 0 && !is_victory_wonder(d, class) && g.city_can_continue_production(c, order);
    ok.then_some(b)
}

/// The price of the wonder at the head of city `c`'s queue: 3 gold per hammer it still needs.
pub fn bazaar_price(g: &Game, c: CityId) -> i32 {
    BAZAAR_GOLD_PER_HAMMER * (g.city_production_needed(c) - g.city_production(c)).max(0)
}

/// May `caster` cast Bazaar of Wonders at city `c` (a city of its owner, building a wonder it
/// may buy, and the owner has the gold)?
pub fn bazaar_ok(g: &Game, caster: UnitId, c: CityId) -> bool {
    let p = g.units[caster].owner;
    g.cities[c].owner == p && bazaar_wonder(g, c).is_some() && g.players[p].gold >= bazaar_price(g, c)
}

/// `targetsBazaarOfWonders(caster)`.
fn targets_bazaar_of_wonders(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let p = g.units[caster].owner;
    g.players[p].cities.iter().copied().filter(|&c| bazaar_ok(g, caster, c)).map(SpellTarget::City).collect()
}

/// `spellBazaarOfWonders(caster)`: the gold is paid and the wonder completes, as a build does.
fn spell_bazaar_of_wonders(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(c) = target_city(g) else { return };
    if !bazaar_ok(g, caster, c) {
        return;
    }
    let p = g.units[caster].owner;
    let price = bazaar_price(g, c);
    g.players[p].gold -= price;
    let rest = (g.city_production_needed(c) - g.city_production(c)).max(0);
    g.city_change_production(c, rest);
    g.city_pop_order(c, 0, true, true);
    let turn = g.game_turn();
    g.log.msg(turn, Some(p), || format!("Bazaar of Wonders: a wonder bought for {price} gold"));
}

// ---------------------------------------------------------------------------------------------
// P16 Call of the Faithful: the heroes of the caster's owner's state religion (`religion_hero1`,
// `religion_hero2`: one or two) that no player has alive join it -- dead ones too, whatever the
// world-unit limit and their techs -- in the religion's holy city if the owner holds it, else in
// its capital.

/// The heroes Call of the Faithful brings `p` and the city they join it in (`None`: no state
/// religion, no hero missing, or no city). A hero is the civilization's unit of its class, else
/// the class's own.
pub fn call_of_the_faithful(g: &Game, p: PlayerId) -> Option<(CityId, Vec<UnitType>)> {
    let d = g.d;
    let religion = g.player_get_state_religion(p)?;
    let info = &d.religions[religion];
    let civ = g.player_get_civilization_type(p);
    let heroes: Vec<UnitType> = [info.religion_hero1, info.religion_hero2]
        .into_iter()
        .flatten()
        .filter(|&class| !g.player_ids().any(|o| g.players[o].unit_class_count[class] > 0))
        .filter_map(|class| civ.and_then(|civ| d.civ_unit(civ, class)).or(d.unit_classes[class].default_unit_index))
        .collect();
    if heroes.is_empty() {
        return None;
    }
    let city = g.get_holy_city(religion).filter(|&c| g.cities[c].owner == p).or_else(|| g.player_get_capital_city(p))?;
    Some((city, heroes))
}

/// `reqCallOfTheFaithful(caster)`.
fn req_call_of_the_faithful(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    call_of_the_faithful(g, g.units[caster].owner).is_some()
}

/// `spellCallOfTheFaithful(caster)`: the missing heroes appear in the city.
fn spell_call_of_the_faithful(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let p = g.units[caster].owner;
    let Some((c, heroes)) = call_of_the_faithful(g, p) else { return };
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    for &unit in &heroes {
        g.player_init_unit(p, unit, x, y, None, None);
    }
    let turn = g.game_turn();
    let names: Vec<String> = heroes.iter().map(|&u| g.d.text.name(&g.d.units[u].base.description).to_string()).collect();
    g.log.msg(turn, Some(p), || format!("Call of the Faithful: {}", names.join(", ")));
}

// ---------------------------------------------------------------------------------------------
// M19 Grand Order: every city of the caster's owner completes its current build at once -- a unit
// or a building that is no wonder (world, team or national); rituals (projects) and processes are
// left alone -- but at most 10 turns of its own production toward it: a build that needs more
// gets those 10 turns of production.

/// What Grand Order does in city `c`: the production its current build gets, and whether that
/// completes it (`None`: no eligible build, or no production toward it).
pub fn grand_order_gain(g: &Game, c: CityId) -> Option<(i32, bool)> {
    let d = g.d;
    let order = g.cities[c].head_order()?;
    match order.order_type {
        OrderType::ORDER_TRAIN => {}
        OrderType::ORDER_CONSTRUCT => {
            let class = d.buildings[BuildingType::from_usize(order.data1.max(0) as usize)].building_class_type?;
            if is_limited_wonder_class(d, class) {
                return None;
            }
        }
        _ => return None,
    }
    if !g.city_can_continue_production(c, order) {
        return None;
    }
    let rest = (g.city_production_needed(c) - g.city_production(c)).max(0);
    let most = GRAND_ORDER_TURNS * g.city_current_production_difference(c, false, false).max(0);
    if rest <= most {
        Some((rest, true))
    } else if most > 0 {
        Some((most, false))
    } else {
        None
    }
}

/// `reqGrandOrder(caster)`: some city of the owner has a build it advances.
fn req_grand_order(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    let p = g.units[caster].owner;
    g.players[p].cities.iter().any(|&c| grand_order_gain(g, c).is_some())
}

/// `spellGrandOrder(caster)`.
fn spell_grand_order(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let p = g.units[caster].owner;
    let (mut done, mut advanced) = (0, 0);
    for c in g.players[p].cities.clone() {
        if !g.cities.contains(c) {
            continue;
        }
        let Some((gain, complete)) = grand_order_gain(g, c) else { continue };
        g.city_change_production(c, gain);
        if complete {
            g.city_pop_order(c, 0, true, true);
            done += 1;
        } else {
            advanced += 1;
        }
    }
    let turn = g.game_turn();
    g.log.msg(turn, Some(p), || format!("Grand Order: {done} builds completed, {advanced} advanced"));
}

// ---------------------------------------------------------------------------------------------
// M18 Mammon's Bargain: 1500 gold, and the Armageddon counter rises by 10 points of its
// percentage (the slots' scale), whatever its limit.

/// `spellMammonsBargain(caster)`.
fn spell_mammons_bargain(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let p = g.units[caster].owner;
    g.players[p].gold += MAMMON_GOLD;
    let change = counter_change_for_points(g, MAMMON_COUNTER);
    g.change_global_counter(change);
    let turn = g.game_turn();
    g.log.msg(turn, Some(p), || format!("Mammon's Bargain: +{MAMMON_GOLD} gold, +{MAMMON_COUNTER} to the Armageddon counter"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Map, MapInitData};
    use crate::setup::GameSetup;

    const US: PlayerId = PlayerId(0);
    const THEM: PlayerId = PlayerId(1);

    /// Every name the batch's data uses resolves.
    #[test]
    fn registered_names_resolve() {
        for name in [
            "spellAstralLeap",
            "spellSealOfBinding",
            "spellMasqueOfTheRedDeath",
            "spellPurchaseCharter",
            "spellCulturalAnnexation",
            "spellTransmutationCharter",
            "spellBazaarOfWonders",
            "spellCallOfTheFaithful",
            "spellGrandOrder",
            "spellMammonsBargain",
        ] {
            assert!(find_spell(name).is_some(), "{name}");
        }
        assert!(find_req("reqCallOfTheFaithful").is_some() && find_req("reqGrandOrder").is_some());
        for name in [
            "targetsAstralLeap",
            "targetsSealOfBinding",
            "targetsMasqueOfTheRedDeath",
            "targetsPurchaseCharter",
            "targetsCulturalAnnexation",
            "targetsTransmutationCharter",
            "targetsBazaarOfWonders",
        ] {
            assert!(find_targets(name).is_some(), "{name}");
        }
    }

    /// A 20x12 map, grassland at x 2..=17, y 3..=8 and coast around it, for two players with no
    /// units or cities who have met; every plot revealed to both teams; no barbarians.
    fn game() -> Game {
        let d = ffh_data::global();
        let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 7);
        for (slot, name) in ["CIVILIZATION_BANNOR", "CIVILIZATION_LJOSALFAR"].iter().enumerate() {
            let civ = d.t::<CivilizationType>(name);
            setup.players[slot].civ = Some(civ);
            setup.players[slot].leader = d.leader_heads.keys().find(|&l| d.civilizations[civ].leaders[l]);
        }
        let mut g = Game::new(d, setup);
        g.setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
        g.game_init();
        for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
            if g.units.contains(u) {
                g.unit_kill(u, false, None);
            }
        }
        let (np, nt) = (g.num_players(), g.num_teams());
        g.map = Map::new(d, &MapInitData { grid_w: 20, grid_h: 12, ..Default::default() }, np, nt);
        let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
        for p in g.map.plot_ids() {
            let (x, y) = g.map.xy(p);
            let land = (3..=8).contains(&y) && (2..=17).contains(&x);
            g.plot_set_plot_type(p, if land { PlotType::PLOT_LAND } else { PlotType::PLOT_OCEAN }, false);
            g.plot_set_terrain_type(p, if land { grass } else { coast }, false);
        }
        g.map_recalculate_areas();
        g.update_plot_groups();
        for t in g.team_ids() {
            for p in g.map.plot_ids() {
                g.plot_set_revealed(p, t, true, false, None, false);
            }
        }
        let (t0, t1) = (g.players[US].team, g.players[THEM].team);
        g.team_meet(t0, t1, false);
        g
    }

    /// `p`'s team learns the card of `spell`.
    fn learn(g: &mut Game, p: PlayerId, spell: &str) -> SpellType {
        let s = g.d.t::<SpellType>(spell);
        let card = g.d.spells[s].tech_prereq.expect("a card spell needs its card");
        assert!(g.d.techs[card].card.is_some(), "{spell}: its tech is a card");
        let team = g.players[p].team;
        g.team_set_has_tech(team, card, true, Some(p), false, false);
        s
    }

    fn unit(g: &mut Game, p: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let t = g.d.t::<UnitType>(name);
        g.player_init_unit(p, t, x, y, None, None)
    }

    fn plot(g: &Game, x: i32, y: i32) -> PlotIdx {
        g.map.plot_xy(x, y).expect("on the map")
    }

    fn war(g: &mut Game) {
        let (t0, t1) = (g.players[US].team, g.players[THEM].team);
        g.team_declare_war(t0, t1, false, None);
        assert!(g.at_war(t0, t1));
    }

    fn city_at(g: &Game, x: i32, y: i32) -> CityId {
        g.map.plots[plot(g, x, y)].plot_city.expect("a city")
    }

    /// Does city `c` have a building of `b`'s class (its owner's building of the class, at the
    /// city's tier)?
    fn has_class(g: &Game, c: CityId, b: BuildingType) -> bool {
        let class = g.d.buildings[b].building_class_type;
        g.d.buildings.keys().any(|x| g.d.buildings[x].building_class_type == class && g.cities[c].num_real_building[x] > 0)
    }

    #[test]
    fn astral_leap_moves_the_citys_units_to_a_free_visible_land_plot() {
        let mut g = game();
        let spell = learn(&mut g, US, "SPELL_ASTRAL_LEAP");
        let c = g.player_init_city(US, 3, 5, true, true);
        let a = unit(&mut g, US, "UNIT_AXEMAN", 3, 5);
        let b = unit(&mut g, US, "UNIT_WARRIOR", 3, 5);
        // A scout sees the plots around (14, 6); another civilization's unit stands on (15, 5).
        let scout = unit(&mut g, US, "UNIT_SCOUT", 14, 6);
        unit(&mut g, THEM, "UNIT_WARRIOR", 15, 5);
        let team = g.players[US].team;
        let to = plot(&g, 14, 5);
        assert!(!g.unit_can_cast(scout, spell, false), "the caster stands in no city of ours");
        let targets = g.unit_spell_targets(a, spell);
        assert!(targets.contains(&SpellTarget::Plot(to)));
        assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 15, 5))), "another team's unit");
        assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 3, 2))), "water");
        let unseen = plot(&g, 9, 8);
        assert!(!g.plot_is_visible(unseen, team) && !targets.contains(&SpellTarget::Plot(unseen)), "a plot we do not see");
        assert!(!targets.contains(&SpellTarget::Plot(g.city_plot(c))), "where the units stand");
        assert!(g.unit_can_cast_at(a, spell, SpellTarget::Plot(to)));
        g.unit_cast_at(a, spell, SpellTarget::Plot(to));
        for u in [a, b] {
            assert_eq!(g.unit_plot(u), to);
            assert!(!g.unit_can_move(u), "no moves left");
        }
        assert_eq!(g.units[a].group, g.units[b].group, "one stack");
        assert!(g.map.plots[g.city_plot(c)].units.is_empty());
        assert!(!g.unit_can_cast(b, spell, false), "once a game");
    }

    #[test]
    fn seal_of_binding_holds_an_enemy_hero_for_twenty_turns() {
        let mut g = game();
        let spell = learn(&mut g, US, "SPELL_SEAL_OF_BINDING");
        let caster = unit(&mut g, US, "UNIT_WARRIOR", 5, 5);
        let hero = unit(&mut g, THEM, "UNIT_AXEMAN", 6, 5);
        unit(&mut g, THEM, "UNIT_AXEMAN", 6, 6);
        let hero_promotion = g.d.t::<PromotionType>("PROMOTION_HERO");
        g.unit_set_has_promotion(hero, hero_promotion, true);
        let far = unit(&mut g, THEM, "UNIT_AXEMAN", 16, 8);
        g.unit_set_has_promotion(far, hero_promotion, true);
        assert!(g.unit_spell_targets(caster, spell).is_empty(), "at peace: no target");
        war(&mut g);
        assert_eq!(g.unit_spell_targets(caster, spell), vec![SpellTarget::Unit(hero)], "the hero we see, not a plain unit or one out of sight");
        g.unit_cast_at(caster, spell, SpellTarget::Unit(hero));
        assert_eq!(g.unit_get_immobile_timer(hero), SEAL_TURNS);
        assert!(!g.unit_can_move(hero), "it cannot move");
        assert!(g.unit_can_defend(hero, None), "it still defends");
    }

    #[test]
    fn masque_of_the_red_death_halves_a_visible_enemy_city() {
        let mut g = game();
        let spell = learn(&mut g, US, "SPELL_MASQUE_OF_THE_RED_DEATH");
        g.player_init_city(US, 3, 5, true, true);
        let caster = unit(&mut g, US, "UNIT_WARRIOR", 3, 5);
        let theirs = g.player_init_city(THEM, 10, 5, true, true);
        g.city_set_population(theirs, 7);
        let small = g.player_init_city(THEM, 14, 7, true, true);
        g.city_set_population(small, 1);
        war(&mut g);
        assert!(g.unit_spell_targets(caster, spell).is_empty(), "no enemy city in sight");
        unit(&mut g, US, "UNIT_SCOUT", 9, 5);
        unit(&mut g, US, "UNIT_SCOUT", 13, 7);
        assert_eq!(g.unit_spell_targets(caster, spell), vec![SpellTarget::City(theirs)], "the one of 2 or more");
        g.unit_cast_at(caster, spell, SpellTarget::City(theirs));
        assert_eq!(g.cities[theirs].population, 4);
    }

    #[test]
    fn purchase_charter_buys_a_small_city_at_peace_with_its_buildings() {
        let mut g = game();
        let d = g.d;
        let spell = learn(&mut g, US, "SPELL_PURCHASE_CHARTER");
        g.player_init_city(US, 3, 5, true, true);
        let caster = unit(&mut g, US, "UNIT_WARRIOR", 3, 5);
        let capital = g.player_init_city(THEM, 15, 5, true, true);
        assert!(g.city_is_capital(capital));
        let small = g.player_init_city(THEM, 8, 5, true, true);
        g.city_set_population(small, 3);
        let granary = d.t::<BuildingType>("BUILDING_GRANARY");
        g.city_set_num_real_building(small, granary, 1);
        let big = g.player_init_city(THEM, 8, 8, true, true);
        g.city_set_population(big, 6);
        let price = purchase_charter_price(&g, small);
        assert_eq!(price, 300 + 2 * g.player_get_production_needed_building(THEM, granary));
        g.players[US].gold = price - 1;
        assert!(g.unit_spell_targets(caster, spell).is_empty(), "short of gold");
        g.players[US].gold = price + 1000;
        assert_eq!(g.unit_spell_targets(caster, spell), vec![SpellTarget::City(small)], "not the capital, not a city of 6");
        // The target, named for people, says its price (`py_target_label`).
        let label = g.spell_target_label(caster, spell, SpellTarget::City(small));
        assert!(label.ends_with(&format!(": Price: {price} gold")), "{label}");
        let their_gold = g.players[THEM].gold;
        g.unit_cast_at(caster, spell, SpellTarget::City(small));
        let bought = city_at(&g, 8, 5);
        assert_eq!(g.cities[bought].owner, US);
        assert!(has_class(&g, bought, granary), "it keeps its buildings");
        assert_eq!(g.players[US].gold, 1000);
        assert_eq!(g.players[THEM].gold, their_gold + price);
        assert_eq!(g.players[THEM].ai_attitude_extra[US], CITY_TAKEN_ATTITUDE);
        let (t0, t1) = (g.players[US].team, g.players[THEM].team);
        assert!(!g.at_war(t0, t1));
    }

    #[test]
    fn purchase_charter_wants_peace() {
        let mut g = game();
        let spell = learn(&mut g, US, "SPELL_PURCHASE_CHARTER");
        g.player_init_city(US, 3, 5, true, true);
        let caster = unit(&mut g, US, "UNIT_WARRIOR", 3, 5);
        g.player_init_city(THEM, 15, 5, true, true);
        let small = g.player_init_city(THEM, 8, 5, true, true);
        g.players[US].gold = 10_000;
        assert_eq!(g.unit_spell_targets(caster, spell), vec![SpellTarget::City(small)]);
        war(&mut g);
        assert!(g.unit_spell_targets(caster, spell).is_empty(), "at war");
    }

    #[test]
    fn cultural_annexation_takes_a_city_where_our_culture_is_strong() {
        let mut g = game();
        let spell = learn(&mut g, US, "SPELL_CULTURAL_ANNEXATION");
        g.player_init_city(US, 3, 5, true, true);
        let caster = unit(&mut g, US, "UNIT_WARRIOR", 3, 5);
        let capital = g.player_init_city(THEM, 15, 5, true, true);
        let near = g.player_init_city(THEM, 7, 5, true, true);
        let monument = g.d.t::<BuildingType>("BUILDING_MONUMENT");
        g.city_set_num_real_building(near, monument, 1);
        let q = g.city_plot(near);
        assert!(within_range_of_borders(&g, US, q, ANNEX_RANGE));
        for p in g.player_ids() {
            g.map.plots[q].culture[p] = 0;
        }
        g.map.plots[q].culture[THEM] = 100;
        g.map.plots[q].culture[US] = 40;
        assert!(g.unit_spell_targets(caster, spell).is_empty(), "28% of the culture");
        g.map.plots[q].culture[US] = 60;
        let cq = g.city_plot(capital);
        g.map.plots[cq].culture[US] = 1000;
        assert_eq!(g.unit_spell_targets(caster, spell), vec![SpellTarget::City(near)], "37%; not the capital");
        g.unit_cast_at(caster, spell, SpellTarget::City(near));
        let annexed = city_at(&g, 7, 5);
        assert_eq!(g.cities[annexed].owner, US);
        assert!(has_class(&g, annexed, monument), "it keeps its buildings");
        assert_eq!(g.players[THEM].ai_attitude_extra[US], CITY_TAKEN_ATTITUDE);
        let (t0, t1) = (g.players[US].team, g.players[THEM].team);
        assert!(!g.at_war(t0, t1), "the owner keeps peace");
    }

    /// The groups the help names are the data's.
    #[test]
    fn transmutation_groups_are_the_ones_the_help_names() {
        let d = ffh_data::global();
        let names = |group: ResourceGroup| -> Vec<String> {
            let mut v: Vec<String> = d.bonuses.keys().filter(|&b| resource_group(d, b) == Some(group)).map(|b| d.text.name(&d.bonuses[b].base.description).to_string()).collect();
            v.sort();
            v
        };
        assert_eq!(names(ResourceGroup::Strategic), ["Copper", "Gunpowder", "Horse", "Iron", "Mithril", "Reagents"]);
        assert_eq!(names(ResourceGroup::Luxury), ["Cotton", "Dye", "Fur", "Gems", "Gold", "Ivory", "Pearls", "Silk", "Sugar", "Whale", "Wine"]);
        assert_eq!(names(ResourceGroup::Food), ["Banana", "Clam", "Corn", "Cow", "Crab", "Deer", "Fish", "Pig", "Rice", "Sheep", "Wheat"]);
        let help = d.text.get("TXT_KEY_TECH_CARD_TRANSMUTATION_CHARTER_HELP");
        for group in [ResourceGroup::Strategic, ResourceGroup::Luxury, ResourceGroup::Food] {
            assert!(help.contains(&names(group).join(", ")), "{group:?} in the help");
        }
    }

    #[test]
    fn transmutation_charter_turns_a_resource_into_another_of_its_group() {
        let mut g = game();
        let d = g.d;
        let spell = learn(&mut g, US, "SPELL_TRANSMUTATION_CHARTER");
        g.player_init_city(US, 2, 5, true, true);
        let caster = unit(&mut g, US, "UNIT_WARRIOR", 2, 5);
        let team = g.players[US].team;
        for tech in ["TECH_MINING", "TECH_SMELTING"] {
            g.team_set_has_tech(team, d.t::<TechType>(tech), true, Some(US), false, false);
        }
        let b = |name: &str| d.t::<BonusType>(name);
        let land = plot(&g, 3, 5);
        let sea = plot(&g, 1, 5);
        assert_eq!(g.map.plots[sea].owner, Some(US));
        g.plot_set_bonus_type(land, Some(b("BONUS_COPPER")));
        g.plot_set_bonus_type(sea, Some(b("BONUS_FISH")));
        let targets = g.unit_spell_targets(caster, spell);
        let choice = |q: PlotIdx, name: &str| SpellTarget::Choice(transmutation_choice(d, q, b(name)));
        assert!(targets.contains(&choice(land, "BONUS_IRON")));
        assert!(!targets.contains(&choice(land, "BONUS_COPPER")), "itself");
        assert!(!targets.contains(&choice(land, "BONUS_WHEAT")), "another group");
        assert!(!targets.contains(&choice(land, "BONUS_MITHRIL")), "a resource we cannot see");
        assert!(targets.contains(&choice(sea, "BONUS_CLAM")));
        assert!(!targets.contains(&choice(sea, "BONUS_WHEAT")), "water stays water");
        let SpellTarget::Choice(iron) = choice(land, "BONUS_IRON") else { unreachable!() };
        assert_eq!(transmutation_charter_choice_label(&g, iron), "Copper (3, 5) into Iron");
        g.unit_cast_at(caster, spell, choice(land, "BONUS_IRON"));
        assert_eq!(g.map.plots[land].bonus_type, Some(b("BONUS_IRON")));
        assert_eq!(g.map.plots[sea].bonus_type, Some(b("BONUS_FISH")), "one plot only");
    }

    #[test]
    fn bazaar_of_wonders_completes_the_wonder_a_city_builds() {
        let mut g = game();
        let d = g.d;
        let spell = learn(&mut g, US, "SPELL_BAZAAR_OF_WONDERS");
        let c = g.player_init_city(US, 4, 5, true, true);
        let caster = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        let team = g.players[US].team;
        // A world wonder the city can build once its tech is known.
        let mut wonder = None;
        for b in d.buildings.keys() {
            let Some(class) = d.buildings[b].building_class_type else { continue };
            if !is_world_wonder_class(d, class) || d.buildings[b].production_cost <= 0 || is_victory_wonder(d, class) {
                continue;
            }
            let tech = d.buildings[b].prereq_and_tech.filter(|&t| !g.teams[team].has_tech[t]);
            if let Some(t) = tech {
                g.team_set_has_tech(team, t, true, Some(US), false, false);
            }
            if g.city_can_construct(c, b, false, false, false) {
                wonder = Some(b);
                break;
            }
            if let Some(t) = tech {
                g.team_set_has_tech(team, t, false, Some(US), false, false);
            }
        }
        let wonder = wonder.expect("a world wonder to build");
        let granary = d.t::<BuildingType>("BUILDING_GRANARY");
        g.city_push_order(c, OrderType::ORDER_CONSTRUCT, granary.i(), -1, false, false, false, true);
        g.players[US].gold = 100_000;
        assert!(g.unit_spell_targets(caster, spell).is_empty(), "no wonder in the queue");
        g.city_push_order(c, OrderType::ORDER_CONSTRUCT, wonder.i(), -1, false, false, false, false);
        assert_eq!(bazaar_wonder(&g, c), Some(wonder));
        g.cities[c].building_production[wonder] = 30;
        let price = bazaar_price(&g, c);
        assert_eq!(price, 3 * (g.city_production_needed_building(c, wonder) - 30));
        g.players[US].gold = price - 1;
        assert!(g.unit_spell_targets(caster, spell).is_empty(), "short of gold");
        g.players[US].gold = price;
        assert_eq!(g.unit_spell_targets(caster, spell), vec![SpellTarget::City(c)]);
        g.unit_cast_at(caster, spell, SpellTarget::City(c));
        assert_eq!(g.cities[c].num_real_building[wonder], 1);
        assert_eq!(g.players[US].gold, 0);
        assert_eq!(g.city_production_building(c), Some(granary), "the queue goes on");
    }

    #[test]
    fn call_of_the_faithful_brings_the_missing_heroes_dead_ones_too() {
        let mut g = game();
        let d = g.d;
        let spell = learn(&mut g, US, "SPELL_CALL_OF_THE_FAITHFUL");
        let c = g.player_init_city(US, 4, 5, true, true);
        let caster = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        assert!(!g.unit_can_cast(caster, spell, false), "no state religion");
        let order = d.t::<ReligionType>("RELIGION_THE_ORDER");
        g.player_change_state_religion_count(US, 1);
        g.player_set_last_state_religion(US, Some(order));
        let (valin, sphener) = (d.t::<UnitType>("UNIT_VALIN"), d.t::<UnitType>("UNIT_SPHENER"));
        // Valin lives with the other player; Sphener lived and died.
        unit(&mut g, THEM, "UNIT_VALIN", 15, 5);
        let dead = unit(&mut g, THEM, "UNIT_SPHENER", 15, 6);
        g.unit_kill(dead, false, None);
        let sphener_class = d.units[sphener].unit_class_type.unwrap();
        assert!(g.unit_class_created_count[sphener_class] > 0, "created once already");
        assert_eq!(call_of_the_faithful(&g, US), Some((c, vec![sphener])), "only the missing hero");
        assert!(g.unit_can_cast(caster, spell, false));
        g.unit_cast(caster, spell);
        let ours: Vec<UnitType> = g.players[US].units.iter().map(|&u| g.units[u].unit_type).collect();
        assert!(ours.contains(&sphener) && !ours.contains(&valin));
        let s = g.players[US].units.iter().copied().find(|&u| g.units[u].unit_type == sphener).unwrap();
        assert_eq!(g.unit_plot(s), g.city_plot(c), "in the capital (the holy city is not ours)");
        assert!(!g.unit_can_cast(caster, spell, false), "once a game");
    }

    #[test]
    fn grand_order_completes_builds_up_to_ten_turns_of_production() {
        let mut g = game();
        let d = g.d;
        let spell = learn(&mut g, US, "SPELL_GRAND_ORDER");
        let a = g.player_init_city(US, 4, 5, true, true);
        let b = g.player_init_city(US, 10, 5, true, true);
        let w = g.player_init_city(US, 15, 5, true, true);
        let caster = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        // City a: a warrior one hammer short.
        let warrior = d.t::<UnitType>("UNIT_WARRIOR");
        g.city_push_order(a, OrderType::ORDER_TRAIN, warrior.i(), -1, false, false, false, true);
        let needed = g.city_production_needed(a);
        g.cities[a].unit_production[warrior] = needed - 1;
        g.cities[a].base_yield_rate[YieldType::YIELD_PRODUCTION.idx()] = 2;
        // City b: its most expensive unit that is no food build, at 1 hammer a turn.
        let costly = d
            .units
            .keys()
            .filter(|&t| g.city_can_train(b, t, false, false, false, false) && !g.city_is_food_production_unit(b, t))
            .max_by_key(|&t| (g.city_production_needed_unit(b, t), std::cmp::Reverse(t)))
            .expect("a unit to train");
        g.city_push_order(b, OrderType::ORDER_TRAIN, costly.i(), -1, false, false, false, false);
        g.cities[b].base_yield_rate[YieldType::YIELD_PRODUCTION.idx()] = 1;
        let rate = g.city_current_production_difference(b, false, false);
        assert!(rate > 0 && g.city_production_needed(b) > GRAND_ORDER_TURNS * rate, "a build of more than 10 turns");
        // City w: a world wonder (left alone).
        let wonder = d.buildings.keys().find(|&x| d.buildings[x].building_class_type.is_some_and(|k| is_world_wonder_class(d, k)) && d.buildings[x].production_cost > 0).unwrap();
        g.city_push_order(w, OrderType::ORDER_CONSTRUCT, wonder.i(), -1, false, false, false, true);
        assert_eq!(grand_order_gain(&g, w), None);
        let before = g.players[US].units.len();
        assert!(g.unit_can_cast(caster, spell, false));
        g.unit_cast(caster, spell);
        let warriors = g.map.plots[g.city_plot(a)].units.iter().filter(|&&u| g.units[u].unit_type == warrior && u != caster).count();
        assert_eq!(warriors, 1, "the warrior is trained");
        assert_eq!(g.players[US].units.len(), before + 1);
        assert_eq!(g.cities[b].unit_production[costly], GRAND_ORDER_TURNS * rate, "10 turns of production");
        assert_eq!(g.city_production_unit(b), Some(costly), "still building it");
        assert_eq!(g.cities[w].building_production[wonder], 0, "no wonders");
    }

    #[test]
    fn mammons_bargain_pays_and_raises_the_counter_ten_points() {
        let mut g = game();
        let spell = learn(&mut g, US, "SPELL_MAMMONS_BARGAIN");
        let caster = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        g.global_counter_limit = 133;
        g.global_counter = 14;
        assert_eq!(g.get_global_counter(), 10);
        let gold = g.players[US].gold;
        assert!(g.unit_can_cast(caster, spell, false));
        g.unit_cast(caster, spell);
        assert_eq!(g.players[US].gold, gold + MAMMON_GOLD);
        assert_eq!(g.get_global_counter(), 20);
        for (limit, counter, raw) in [(100, 0, 10), (150, 0, 15), (133, 0, 14), (133, 14, 13), (250, 25, 25)] {
            g.global_counter_limit = limit;
            g.global_counter = counter;
            let change = counter_change_for_points(&g, MAMMON_COUNTER);
            assert_eq!(change, raw, "limit {limit}, counter {counter}");
            let before = g.get_global_counter();
            g.global_counter += change;
            assert_eq!(g.get_global_counter(), before + MAMMON_COUNTER, "limit {limit}, counter {counter}");
        }
    }
}
