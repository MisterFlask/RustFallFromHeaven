//! The helpers the cards share (docs/CARDS.md, section 3), and their lookups.

use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::SpellTarget;
use ffh_data::*;

/// `py_target_label` functions.
pub fn find_target_label(name: &str) -> Option<TargetLabelFn> {
    let f: TargetLabelFn = match name {
        "labelInfo" => label_info,
        _ => return None,
    };
    Some(f)
}

// ---------------------------------------------------------------------------------------------
// Choice targets whose options are entries of an info table (docs/CARDS.md, section 3).

/// The kinds `labelInfo` names entries of (the info tables a choice spell may offer).
pub const LABEL_KINDS: [&str; 15] = [
    "TraitType",
    "BuildingType",
    "BuildingClassType",
    "UnitType",
    "UnitClassType",
    "PromotionType",
    "BonusType",
    "BonusClassType",
    "DamageType",
    "ReligionType",
    "CivicType",
    "TechType",
    "ImprovementType",
    "ProjectType",
    "SpecialistType",
];

/// The description key of entry `i` of the info table of kind `kind` (one of [`LABEL_KINDS`]).
pub fn info_description<'d>(d: &'d GameData, kind: &str, i: i32) -> Option<&'d str> {
    let i = usize::try_from(i).ok()?;
    fn key<'d, I: Idx, T>(v: &'d IdxVec<I, T>, i: usize, f: impl Fn(&'d T) -> &'d InfoBase) -> Option<&'d str> {
        (i < v.len()).then(|| f(&v[I::from_usize(i)]).description.as_str())
    }
    match kind {
        "TraitType" => key(&d.traits, i, |x| &x.base),
        "BuildingType" => key(&d.buildings, i, |x| &x.base),
        "BuildingClassType" => key(&d.building_classes, i, |x| &x.base),
        "UnitType" => key(&d.units, i, |x| &x.base),
        "UnitClassType" => key(&d.unit_classes, i, |x| &x.base),
        "PromotionType" => key(&d.promotions, i, |x| &x.base),
        "BonusType" => key(&d.bonuses, i, |x| &x.base),
        "BonusClassType" => key(&d.bonus_classes, i, |x| &x.base),
        "DamageType" => key(&d.damages, i, |x| &x.base),
        "ReligionType" => key(&d.religions, i, |x| &x.base),
        "CivicType" => key(&d.civics, i, |x| &x.base),
        "TechType" => key(&d.techs, i, |x| &x.base),
        "ImprovementType" => key(&d.improvements, i, |x| &x.base),
        "ProjectType" => key(&d.projects, i, |x| &x.base),
        "SpecialistType" => key(&d.specialists, i, |x| &x.base),
        _ => None,
    }
}

/// `labelInfo(pCaster, iChoice, 'TraitType')`: a choice spell whose options are entries of an info
/// table (its third argument, one of [`LABEL_KINDS`]) names option `choice` by the entry's name.
/// A UI also shows the entry's help (ffh-play reads the kind from the call).
fn label_info(g: &Game, _caster: UnitId, target: SpellTarget, call: &PyCall) -> String {
    let SpellTarget::Choice(choice) = target else { return String::new() };
    let d = g.d;
    match call.str_arg(2).and_then(|kind| info_description(d, kind, choice)) {
        Some(key) => d.text.name(key).to_string(),
        None => format!("Option {choice}"),
    }
}

// ---------------------------------------------------------------------------------------------
// Distances and levels.

/// The plots within `range` plots (the 8 neighbours, the map wrapping) of a plot player `p` owns,
/// its own plots included, as a mask by plot index: "within N plots of your borders".
pub fn near_borders(g: &Game, p: PlayerId, range: i32) -> Vec<bool> {
    use crate::utils::{PLOT_DIRECTION_X, PLOT_DIRECTION_Y};
    let n = g.map.plots.len();
    let mut dist = vec![i32::MAX; n];
    let mut queue: Vec<PlotIdx> = Vec::new();
    for t in g.map.plot_ids() {
        if g.map.plots[t].owner == Some(p) {
            dist[t.idx()] = 0;
            queue.push(t);
        }
    }
    let mut i = 0;
    while i < queue.len() {
        let t = queue[i];
        i += 1;
        let next = dist[t.idx()] + 1;
        if next > range {
            continue;
        }
        let (x, y) = g.map.xy(t);
        for k in 0..PLOT_DIRECTION_X.len() {
            if let Some(q) = g.map.plot_xy(x + PLOT_DIRECTION_X[k], y + PLOT_DIRECTION_Y[k]) {
                if dist[q.idx()] > next {
                    dist[q.idx()] = next;
                    queue.push(q);
                }
            }
        }
    }
    dist.into_iter().map(|x| x <= range).collect()
}

/// Unit `u` gains `n` levels (docs/DECKS_ROUND8.md C.3, rule 10: a level is the XP for its next
/// level): its experience rises to what it needs for `n` more promotions than it can take now.
pub fn unit_gain_levels(g: &mut Game, u: UnitId, n: i32) {
    if n <= 0 {
        return;
    }
    let owner = g.units[u].owner;
    let level = g.units[u].level;
    let xp = g.units[u].experience;
    // The promotions it can take now: the levels whose threshold its experience meets.
    let mut ready = 0;
    while g.py_get_experience_needed(level + ready, owner) <= xp {
        ready += 1;
    }
    let target = g.py_get_experience_needed(level + ready + n - 1, owner);
    if target > xp {
        g.unit_set_experience(u, target, -1);
    }
    g.unit_test_promotion_ready(u);
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
// Borders and the Armageddon counter.

/// Does `p` own a plot within `range` plots (plot distance) of `plot`? A card's "within N plots of
/// your borders".
pub fn within_range_of_borders(g: &Game, p: PlayerId, plot: PlotIdx, range: i32) -> bool {
    let (x, y) = g.map.xy(plot);
    for dx in -range..=range {
        for dy in -range..=range {
            if g.plot_distance(x, y, x + dx, y + dy) > range {
                continue;
            }
            if g.plot_xy(x, y, dx, dy).is_some_and(|q| g.map.plots[q].owner == Some(p)) {
                return true;
            }
        }
    }
    false
}

/// The raw change (`change_global_counter`) that raises the Armageddon counter's percentage
/// (`get_global_counter`, the scale of the Armageddon deck's slots) by `points`, whatever the
/// counter's limit: a card's "+N to the Armageddon counter".
pub fn counter_change_for_points(g: &Game, points: i32) -> i32 {
    let limit = g.global_counter_limit.max(1) as i64;
    let target = (g.get_global_counter() + points) as i64;
    // The smallest raw value whose percentage reaches the target.
    let raw = (target * limit + 99) / 100;
    (raw - g.global_counter as i64).max(0) as i32
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
/// order (a site list; [`near_borders`] is the mask of every plot within N steps).
pub fn land_near_borders(g: &Game, p: PlayerId, dist: i32) -> Vec<PlotIdx> {
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

/// A new civilization: who it is, what it starts with, and its relation to its founder and its
/// parent.
#[derive(Clone, Debug)]
pub struct Birth {
    pub civ: CivilizationType,
    pub leader: LeaderHeadType,
    /// A name of its own in place of its civilization's (`GameSetup` `civ_description`: the
    /// Usurper's Crown's "Bannor Usurpers").
    pub description: Option<String>,
    /// The player whose card makes it: its controller plays the new civilization, it has met it,
    /// and it is a vassal birth's master. `None`: an Armageddon card's civilization
    /// ([`Birth::wild`]), played by its new slot's own controller (the game's AI:
    /// `Game::add_player_slot`).
    pub founder: Option<PlayerId>,
    /// Its parent (`Player::parent`), whose team's techs it knows (never the cards) and which it
    /// has met: the founder unless set otherwise (the Usurper's Crown: the civilization it secedes
    /// from). `None`: it knows the world's techs ([`world_techs`]).
    pub parent: Option<PlayerId>,
    /// The cities it founds, its capital first.
    pub found: Vec<NewCity>,
    /// The cities it takes over from their owners (a release, as a trade), with their buildings
    /// and garrisons (the owner's land combat units in them but `keep` and heroes), after the
    /// cities it founds.
    pub release: Vec<CityId>,
    /// A unit that stays its owner's in a released city (the caster).
    pub keep: Option<UnitId>,
    /// Units that arrive on a plot of their own, without a city (the Court of Mammon: its settlers
    /// found its cities), after the cities' units.
    pub camp: Option<(PlotIdx, Vec<NewUnit>)>,
    /// Its hero, in its capital (without a city, in its [`Birth::camp`]).
    pub hero: Option<NewUnit>,
    /// Techs beyond its parent's (or the world's).
    pub techs: Vec<TechType>,
    /// Its state religion, spread to its cities.
    pub state_religion: Option<ReligionType>,
    pub gold: i32,
    /// The founder's vassal ([`birth`]: with No Vassal States, open borders and a defensive pact).
    pub vassal: bool,
    /// Its attitude modifier toward the founder (`ai_attitude_extra`: FFH's scale, 4 a step).
    pub founder_attitude: i32,
    /// Its attitude modifier toward its parent, when the parent is not the founder (the Usurper's
    /// grudge).
    pub parent_attitude: i32,
    /// Its attitude modifier toward civilizations of an alignment (`Player::alignment_attitude`).
    pub alignment_attitude: Option<(AlignmentType, i32)>,
}

impl Birth {
    /// A birth of civilization `civ` led by `leader`, founded by `founder` (its parent too), with
    /// nothing yet.
    pub fn new(civ: CivilizationType, leader: LeaderHeadType, founder: PlayerId) -> Birth {
        Birth { founder: Some(founder), parent: Some(founder), ..Birth::wild(civ, leader) }
    }

    /// An Armageddon card's civilization, with nothing yet: no founder and no parent (its new
    /// slot's own controller, the world's techs, nobody's vassal).
    pub fn wild(civ: CivilizationType, leader: LeaderHeadType) -> Birth {
        Birth {
            civ,
            leader,
            description: None,
            founder: None,
            parent: None,
            found: Vec::new(),
            release: Vec::new(),
            keep: None,
            camp: None,
            hero: None,
            techs: Vec::new(),
            state_religion: None,
            gold: 0,
            vassal: false,
            founder_attitude: 0,
            parent_attitude: 0,
            alignment_attitude: None,
        }
    }
}

/// The id the next civilization born gets: a new player slot, after every existing one
/// (`Game::add_player_slot`). A birth never fails for lack of a slot.
pub fn next_player(g: &Game) -> PlayerId {
    PlayerId(g.num_players() as u8)
}

/// The living civilizations: not the barbarians, a minor civilization or a crusade Host.
pub fn civ_players(g: &Game) -> Vec<PlayerId> {
    g.player_ids().filter(|&p| g.players[p].alive && !g.player_is_barbarian(p) && !g.player_is_minor_civ(p) && !g.player_is_crusade_host(p)).collect()
}

/// The world's techs, which a civilization without a parent starts with: every
/// tech (not a card) that at least half of the living civilizations' teams know.
pub fn world_techs(g: &Game) -> Vec<TechType> {
    let d = g.d;
    let mut teams: Vec<TeamId> = civ_players(g).into_iter().map(|p| g.players[p].team).collect();
    teams.sort();
    teams.dedup();
    let half = |t: TechType| 2 * teams.iter().filter(|&&team| g.team_is_has_tech(team, Some(t))).count() >= teams.len();
    d.techs.keys().filter(|&t| d.techs[t].card.is_none() && !teams.is_empty() && half(t)).collect()
}

/// Gives birth to `b` as a new player, which it returns: the player is added
/// (`add_player_advanced`: a new slot, on a team of its own) and played by the founder's
/// controller (as the Infernals are), or by its own (the game's AI) without a founder; its parent is `b.parent`, and it knows its
/// parent's team's techs (never the cards), or without a parent the world's ([`world_techs`]), and
/// `b.techs`. It founds its cities (the first its capital) with their buildings and takes over the
/// released cities, its state religion in every one; its units stand in their cities (or in its
/// camp), its hero in the capital (or the camp). It has met its founder and its parent. As a
/// vassal it is the founder's (ordinary vassal rules: it joins its master's wars); where vassals
/// cannot be (No Vassal States, or the founder a vassal itself) it signs open borders and a
/// defensive pact with the founder instead, a deal like any other. No other war or peace is forced (docs/DECKS.md, principle 27): its
/// leanings are its attitude modifiers.
pub fn birth(g: &mut Game, b: &Birth) -> PlayerId {
    let d = g.d;
    let techs: Vec<TechType> = match b.parent {
        Some(parent) => {
            let pt = g.players[parent].team;
            d.techs.keys().filter(|&t| g.team_is_has_tech(pt, Some(t)) && d.techs[t].card.is_none()).collect()
        }
        None => world_techs(g),
    };
    let p = g.add_player_advanced(None, b.leader, b.civ);
    if let Some(description) = &b.description {
        g.setup.players[p.idx()].civ_description = description.clone();
    }
    if let Some(founder) = b.founder {
        g.controllers[p] = g.controllers[founder].clone();
    }
    g.players[p].parent = b.parent;
    let nt = g.players[p].team;
    // At war with the barbarians on both sides, as every civilization is (`add_player_advanced`
    // sets only the new team's side, FFH's quirk; the other side left at peace would also deny
    // the vassal trade, `ai_vassal_trade`, and end a born vassal's vassalage).
    let barbarians = g.players[g.barbarian_player()].team;
    g.teams[barbarians].at_war[nt] = true;
    for tech in d.techs.keys() {
        if (techs.contains(&tech) || b.techs.contains(&tech)) && !g.team_is_has_tech(nt, Some(tech)) {
            g.team_set_has_tech(nt, tech, true, Some(p), false, false);
        }
    }
    g.players[p].gold += b.gold;
    for other in [b.founder, b.parent].into_iter().flatten() {
        let ot = g.players[other].team;
        if !g.teams[nt].has_met[ot] {
            g.team_meet(nt, ot, true);
        }
    }

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
    if let Some((plot, units)) = &b.camp {
        let (x, y) = g.map.xy(*plot);
        for nu in units {
            new_unit(g, p, nu, x, y);
        }
    }
    let hero_at = cities.first().map(|&capital| g.city_plot(capital)).or(b.camp.as_ref().map(|(plot, _)| *plot));
    if let (Some(hero), Some(at)) = (&b.hero, hero_at) {
        let (x, y) = g.map.xy(at);
        new_unit(g, p, hero, x, y);
    }

    if let (Some(founder), true) = (b.founder, b.vassal) {
        let ft = g.players[founder].team;
        if !g.is_option(GameOptionType::GAMEOPTION_NO_VASSAL_STATES) && !g.team_is_a_vassal(ft) {
            g.team_assign_vassal(ft, nt, false);
        } else {
            // Open borders and a defensive pact, each held by its own standing deal (`data` 1, as
            // `CvDeal::startTeamTrade` records an agreement), so that the ordinary rules end them:
            // a deal that starts them (`data` 0) records them in a checked deal of its own, which
            // the checks can refuse here (the pact needs its tech and both teams at peace), and
            // a pact no deal records is never cancelled (the new civilization, made a vassal of
            // its founder's enemy, then declared war on itself).
            for item_type in [TradeableItem::TRADE_OPEN_BORDERS, TradeableItem::TRADE_DEFENSIVE_PACT] {
                let items = [crate::structs::TradeData { item_type, data: 1, offering: false, hidden: false }];
                g.implement_deal(founder, p, &items, &items, true);
            }
            for (a, b) in [(ft, nt), (nt, ft)] {
                g.team_set_open_borders(a, b, true);
                g.team_set_defensive_pact(a, b, true);
            }
        }
    }
    if let Some(founder) = b.founder {
        g.players[p].ai_attitude_extra[founder] += b.founder_attitude;
    }
    if let Some(parent) = b.parent.filter(|&parent| Some(parent) != b.founder) {
        g.players[p].ai_attitude_extra[parent] += b.parent_attitude;
    }
    if let Some((alignment, change)) = b.alignment_attitude {
        g.players[p].alignment_attitude[alignment] += change;
    }
    let turn = g.game_turn();
    let name = g.player_get_civilization_description(p);
    match b.parent.or(b.founder) {
        Some(by) => {
            let by = g.player_get_civilization_description(by);
            g.log.msg(turn, b.founder, || format!("{name} is born of {by}"));
        }
        None => g.log.msg(turn, None, || format!("{name} rises")),
    }
    p
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
