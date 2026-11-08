//! The helpers the cards share (docs/CARDS.md, section 3), and their lookups.

// (Remove `unused_variables, unreachable_code` once every lookup has a card in it.)
#![allow(unused_imports, unused_variables, unreachable_code)]

use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::SpellTarget;
use ffh_data::*;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        _ => return None,
    };
    #[allow(unreachable_code)]
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        _ => return None,
    };
    #[allow(unreachable_code)]
    Some(f)
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        _ => return None,
    };
    #[allow(unreachable_code)]
    Some(f)
}

/// Unit callbacks.
pub fn find_unit_fn(name: &str) -> Option<UnitFn> {
    let f: UnitFn = match name {
        _ => return None,
    };
    #[allow(unreachable_code)]
    Some(f)
}

// ---------------------------------------------------------------------------------------------
// Regions (docs/DECKS_ROUND8.md C.3, rule 9).

/// The regions of plots that pass `member`: each a connected group (the 8 neighbours) of unowned
/// land plots without a city, largest first (ties: the region with the lowest plot index first).
pub fn regions(g: &Game, member: impl Fn(&Game, PlotIdx) -> bool) -> Vec<Vec<PlotIdx>> {
    use crate::utils::{PLOT_DIRECTION_X, PLOT_DIRECTION_Y};
    let n = g.map.plots.len();
    let ok = |t: PlotIdx| {
        let plot = &g.map.plots[t];
        plot.owner.is_none() && plot.plot_city.is_none() && !g.plot_is_water(t) && member(g, t)
    };
    let mut seen = vec![false; n];
    let mut out: Vec<Vec<PlotIdx>> = Vec::new();
    for start in g.map.plot_ids() {
        if seen[start.idx()] || !ok(start) {
            continue;
        }
        seen[start.idx()] = true;
        let mut region = vec![start];
        let mut i = 0;
        while i < region.len() {
            let (x, y) = g.map.xy(region[i]);
            i += 1;
            for k in 0..PLOT_DIRECTION_X.len() {
                if let Some(t) = g.map.plot_xy(x + PLOT_DIRECTION_X[k], y + PLOT_DIRECTION_Y[k]) {
                    if !seen[t.idx()] && ok(t) {
                        seen[t.idx()] = true;
                        region.push(t);
                    }
                }
            }
        }
        region.sort();
        out.push(region);
    }
    out.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].cmp(&b[0])));
    out
}

/// The largest region of plots that pass `member` (see [`regions`]).
pub fn largest_region(g: &Game, member: impl Fn(&Game, PlotIdx) -> bool) -> Option<Vec<PlotIdx>> {
    regions(g, member).into_iter().next()
}

// ---------------------------------------------------------------------------------------------
// City sites (the new civilizations' and the cards' own cities).

/// Can a card's city be founded on `plot`: land a city can stand on (`player_can_found`'s
/// terrain, feature, peak, permanent improvement and mana rules), no unit and no city on it, no
/// city within `MIN_CITY_RANGE`, and unowned (or owned by `owner`).
pub fn free_site(g: &Game, plot: PlotIdx, owner: Option<PlayerId>) -> bool {
    let d = g.d;
    let pl = &g.map.plots[plot];
    if g.plot_is_water(plot) || g.plot_is_impassable(plot) || pl.plot_city.is_some() || !pl.units.is_empty() {
        return false;
    }
    if pl.owner.is_some() && pl.owner != owner {
        return false;
    }
    if pl.feature_type.is_some_and(|f| d.features[f].no_city) {
        return false;
    }
    if pl.improvement_type.is_some_and(|i| d.improvements[i].permanent) {
        return false;
    }
    if let Some(bonus) = pl.bonus_type {
        let class = opt_to_i32(d.bonuses[bonus].bonus_class_type);
        if class != -1 && (class == d.defines.bonusclass_mana || class == d.defines.bonusclass_mana_raw) {
            return false;
        }
    }
    let Some(terrain) = pl.terrain_type else { return false };
    let t = &d.terrains[terrain];
    if !(t.found || (t.found_coast && g.plot_is_coastal_land(plot, -1)) || (t.found_fresh_water && g.plot_is_fresh_water(plot))) {
        return false;
    }
    let (x, y) = g.map.xy(plot);
    let range = d.defines.min_city_range;
    for dx in -range..=range {
        for dy in -range..=range {
            if g.plot_xy(x, y, dx, dy).is_some_and(|q| g.map.plots[q].plot_city.is_some()) {
                return false;
            }
        }
    }
    true
}

/// How good a city on `plot` would be, to rank free sites: its land within the city radius
/// (more with a bonus), its water, fresh water and the coast.
pub fn site_value(g: &Game, plot: PlotIdx) -> i32 {
    let (x, y) = g.map.xy(plot);
    let mut value = 0;
    for dx in -2..=2i32 {
        for dy in -2..=2i32 {
            if dx.abs() == 2 && dy.abs() == 2 {
                continue;
            }
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            value += if g.plot_is_water(q) {
                1
            } else if g.plot_is_impassable(q) {
                0
            } else if g.map.plots[q].bonus_type.is_some() {
                5
            } else {
                3
            };
        }
    }
    if g.plot_is_fresh_water(plot) {
        value += 4;
    }
    if g.plot_is_coastal_land(plot, -1) {
        value += 2;
    }
    value
}

/// The best `n` free sites among `candidates` ([`free_site`] for `owner`), best first by
/// [`site_value`] (ties: the lowest plot index), each farther than `MIN_CITY_RANGE` from the
/// others and from `taken`. Fewer when there are not enough.
pub fn best_sites(g: &Game, candidates: &[PlotIdx], n: usize, owner: Option<PlayerId>, taken: &[PlotIdx]) -> Vec<PlotIdx> {
    let range = g.d.defines.min_city_range;
    let mut ranked: Vec<(i32, PlotIdx)> = candidates.iter().filter(|&&q| free_site(g, q, owner)).map(|&q| (-site_value(g, q), q)).collect();
    ranked.sort();
    ranked.dedup();
    let mut out: Vec<PlotIdx> = Vec::new();
    for (_, q) in ranked {
        if out.len() >= n {
            break;
        }
        let (x, y) = g.map.xy(q);
        // The window `player_can_found` tests for cities (it wraps with the map).
        let near = |o: &PlotIdx| (-range..=range).any(|dx| (-range..=range).any(|dy| g.plot_xy(x, y, dx, dy) == Some(*o)));
        if out.iter().any(near) || taken.iter().any(near) {
            continue;
        }
        out.push(q);
    }
    out
}

/// The land plots outside `p`'s borders within `dist` plots (`plot_distance`) of them, in plot
/// order.
pub fn near_borders(g: &Game, p: PlayerId, dist: i32) -> Vec<PlotIdx> {
    let mut near = vec![false; g.map.plots.len()];
    for q in g.map.plot_ids() {
        if g.map.plots[q].owner != Some(p) {
            continue;
        }
        let (x, y) = g.map.xy(q);
        for dx in -dist..=dist {
            for dy in -dist..=dist {
                if let Some(t) = g.plot_xy(x, y, dx, dy) {
                    let (tx, ty) = g.map.xy(t);
                    if g.plot_distance(x, y, tx, ty) <= dist {
                        near[t.idx()] = true;
                    }
                }
            }
        }
    }
    g.map.plot_ids().filter(|&q| near[q.idx()] && g.map.plots[q].owner != Some(p) && !g.plot_is_water(q)).collect()
}

// ---------------------------------------------------------------------------------------------
// A new civilization born mid-game (docs/CARDS.md, "New civilizations").

/// A unit a new civilization starts with: its type and promotions beyond its own.
#[derive(Clone, Debug)]
pub struct NewUnit {
    pub unit: UnitType,
    pub promotions: Vec<PromotionType>,
}

impl NewUnit {
    pub fn of(unit: UnitType) -> NewUnit {
        NewUnit { unit, promotions: Vec::new() }
    }
}

/// A city a new civilization founds: where, the buildings it starts with and the units in it.
#[derive(Clone, Debug)]
pub struct NewCity {
    pub plot: PlotIdx,
    pub buildings: Vec<BuildingType>,
    pub units: Vec<NewUnit>,
}

/// A new civilization: who it is, what it starts with, and its relation to its founder.
#[derive(Clone, Debug)]
pub struct Birth {
    pub civ: CivilizationType,
    pub leader: LeaderHeadType,
    /// The player whose card makes it: its techs and its controller are the new civilization's.
    pub founder: PlayerId,
    /// The cities it founds, its capital first.
    pub found: Vec<NewCity>,
    /// The founder's cities it takes over (a release), with their buildings and garrisons (the
    /// founder's land combat units in them but `keep` and heroes).
    pub release: Vec<CityId>,
    /// A unit of the founder's that stays the founder's in a released city (the caster).
    pub keep: Option<UnitId>,
    /// Its hero, in its capital.
    pub hero: Option<NewUnit>,
    /// Techs beyond the founder's (it knows every tech the founder's team knows but the cards).
    pub techs: Vec<TechType>,
    /// Its state religion, spread to its cities.
    pub state_religion: Option<ReligionType>,
    pub gold: i32,
    /// The founder's vassal ([`birth`]: with No Vassal States, open borders and a defensive pact).
    pub vassal: bool,
    /// Its attitude modifier toward the founder (`ai_attitude_extra`: FFH's scale, 4 a step).
    pub founder_attitude: i32,
    /// Its attitude modifier toward civilizations of an alignment (`Player::alignment_attitude`).
    pub alignment_attitude: Option<(AlignmentType, i32)>,
}

impl Birth {
    /// A birth of civilization `civ` led by `leader`, founded by `founder`, with nothing yet.
    pub fn new(civ: CivilizationType, leader: LeaderHeadType, founder: PlayerId) -> Birth {
        Birth {
            civ,
            leader,
            founder,
            found: Vec::new(),
            release: Vec::new(),
            keep: None,
            hero: None,
            techs: Vec::new(),
            state_religion: None,
            gold: 0,
            vassal: false,
            founder_attitude: 0,
            alignment_attitude: None,
        }
    }
}

/// The player slot a new civilization would take: a spare slot never alive (shared with Hyborem
/// and Basium; never a crusade Host's). `None`: a card that makes a civilization cannot be cast.
pub fn open_slot(g: &Game) -> Option<PlayerId> {
    g.cf_get_open_player()
}

/// Gives birth to `b` in an [`open_slot`] (`None` when there is none). The slot is added
/// (`add_player_advanced`, on a team of its own) and played by the founder's controller (as the
/// Infernals are); its parent is the founder, and it knows the founder's team's techs (never the
/// cards) and `b.techs`. It founds its cities (the first its capital) with their buildings and
/// takes over the released cities, its state religion in every one; its units stand in their
/// cities, its hero in the capital. It has met the founder. As a vassal it is the founder's
/// (ordinary vassal rules: it joins its master's wars); where vassals cannot be (No Vassal States,
/// or the founder a vassal itself) it signs open borders and a defensive pact with the founder
/// instead, a deal like any other. No other war or peace is forced (docs/DECKS.md, principle 27):
/// its leanings are its attitude modifiers.
pub fn birth(g: &mut Game, b: &Birth) -> Option<PlayerId> {
    let d = g.d;
    let p = open_slot(g)?;
    let founder = b.founder;
    g.add_player_advanced(p, None, b.leader, b.civ);
    g.controllers[p] = g.controllers[founder].clone();
    g.players[p].parent = Some(founder);
    let (ft, nt) = (g.players[founder].team, g.players[p].team);
    // At war with the barbarians on both sides, as every civilization is (`add_player_advanced`
    // sets only the new team's side, FFH's quirk; the other side left at peace would also deny
    // the vassal trade, `ai_vassal_trade`, and end a born vassal's vassalage).
    let barbarians = g.players[g.barbarian_player()].team;
    g.teams[barbarians].at_war[nt] = true;
    for tech in d.techs.keys() {
        let known = g.team_is_has_tech(ft, Some(tech)) && d.techs[tech].card.is_none();
        if (known || b.techs.contains(&tech)) && !g.team_is_has_tech(nt, Some(tech)) {
            g.team_set_has_tech(nt, tech, true, Some(p), false, false);
        }
    }
    g.players[p].gold += b.gold;
    g.team_meet(nt, ft, true);

    let mut cities = Vec::new();
    for nc in &b.found {
        let (x, y) = g.map.xy(nc.plot);
        cities.push(g.player_init_city(p, x, y, true, true));
    }
    for &c in &b.release {
        if let Some(c) = release_city(g, p, c, b.keep) {
            cities.push(c);
        }
    }
    if let Some(r) = b.state_religion {
        for &c in &cities {
            g.city_set_has_religion(c, r, true, false, false);
        }
        g.player_set_last_state_religion(p, Some(r));
    }
    for (nc, &c) in b.found.iter().zip(&cities) {
        for &building in &nc.buildings {
            // At the city's tier (docs/SUITABILITY.md).
            let building = g.city_building_variant(c, building);
            g.city_set_num_real_building(c, building, 1);
        }
        let (x, y) = g.map.xy(nc.plot);
        for nu in &nc.units {
            new_unit(g, p, nu, x, y);
        }
    }
    if let (Some(hero), Some(&capital)) = (&b.hero, cities.first()) {
        let (x, y) = g.map.xy(g.city_plot(capital));
        new_unit(g, p, hero, x, y);
    }

    let vassal_states = !g.is_option(GameOptionType::GAMEOPTION_NO_VASSAL_STATES) && !g.team_is_a_vassal(ft);
    if b.vassal && vassal_states {
        g.team_assign_vassal(ft, nt, false);
    } else if b.vassal {
        let item = |item_type| crate::structs::TradeData { item_type, data: 0, offering: false, hidden: false };
        let items = [item(TradeableItem::TRADE_OPEN_BORDERS), item(TradeableItem::TRADE_DEFENSIVE_PACT)];
        g.implement_deal(founder, p, &items, &items, true);
    }
    g.players[p].ai_attitude_extra[founder] += b.founder_attitude;
    if let Some((alignment, change)) = b.alignment_attitude {
        g.players[p].alignment_attitude[alignment] += change;
    }
    let turn = g.game_turn();
    let (name, by) = (g.player_get_civilization_description(p), g.player_get_civilization_description(founder));
    g.log.msg(turn, Some(founder), || format!("{name} is born of {by}"));
    Some(p)
}

/// A unit of a new civilization's at (x, y), with its promotions.
fn new_unit(g: &mut Game, p: PlayerId, nu: &NewUnit, x: i32, y: i32) -> UnitId {
    let u = g.player_init_unit(p, nu.unit, x, y, None, Some(DirectionType::DIRECTION_SOUTH));
    for &promotion in &nu.promotions {
        g.unit_set_has_promotion(u, promotion, true);
    }
    u
}

/// City `c` of its owner goes to `p` as a trade would (its buildings stay), with its culture and
/// its garrison: the owner's land units in it that can fight, but `keep` and heroes. The city it
/// becomes (a city changing hands has a new id).
fn release_city(g: &mut Game, p: PlayerId, c: CityId, keep: Option<UnitId>) -> Option<CityId> {
    let d = g.d;
    let plot = g.city_plot(c);
    let owner = g.cities[c].owner;
    for u in g.map.plots[plot].units.clone() {
        if !g.units.contains(u) || Some(u) == keep || g.units[u].owner != owner {
            continue;
        }
        let t = g.units[u].unit_type;
        if g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND || d.units[t].combat <= 0 || g.unit_is_hero(u) {
            continue;
        }
        let (x, y, ai) = (g.units[u].x, g.units[u].y, g.units[u].ai_type);
        let nu = g.player_init_unit(p, t, x, y, ai, None);
        g.unit_convert(nu, u);
    }
    let culture = g.cities[c].culture[owner];
    g.player_acquire_city(p, c, false, true, true);
    let new = g.map.plots[plot].plot_city?;
    g.city_set_culture_times100(new, p, culture, true, false);
    g.city_set_culture_times100(new, owner, 0, true, false);
    Some(new)
}
