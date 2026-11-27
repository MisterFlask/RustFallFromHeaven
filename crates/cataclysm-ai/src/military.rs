//! Squads: who stays home, where the others go, and when they attack.
//!
//! Every turn the roles are worked out again from the game (the AI keeps no memory):
//! - **Guards** stand in our cities: a city an enemy civilization's squad could reach soon, the
//!   capital while we are at war or Strain is high, and a Lifeboat we hold (two guards).
//! - After the Blowout, a civilization without a Lifeboat sends every other squad at **one**
//!   target Lifeboat, gathers them next to it and attacks with all of them in the same turn.
//! - The rest go for the nearest enemy city, an enemy squad nearby, or a lair; with nothing to do
//!   they build roads between our cities.
//!
//! An attack is made only on the rules' own forecast (`Game::preview_attack`): when the plot is
//! likely to fall, or when the defender is expected to lose clearly more fighters than we do (a
//! city's Sentinels keep their damage while it is besieged, so several attacks wear it down), or
//! when the city is empty. Otherwise the squad holds next to its target: it raids when the rules
//! allow it, else it fortifies.

use cataclysm_core::combat::BuiltFormation;
use cataclysm_core::game::RAID_STREAK_TO_STRIP;
use cataclysm_core::{
    CityId, Command, Event, FactionId, Forecast, Game, PlayerId, PlotIdx, SallyPosture, SquadId, SquadOrder, UnitClassId,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{best_squad_recipe, try_cmd};

/// An enemy civilization's squad this close to one of our cities makes the city need a guard.
pub(crate) const THREAT_RANGE: i32 = 4;
/// Attack when the forecast gives the plot at least this chance (permille) of falling. Of the
/// attacks this AI made at a 60-70% forecast in 120 games, about half took the plot, and they
/// cost the attacker 0.7 fighters for 1.3-1.5 of the defender's: worth making either way.
const FALLS_TO_ATTACK: i32 = 600;
/// A trade attack (the plot holds, but the defender loses more) risks the attacking squad's
/// wipe-out at most this often (permille).
const TRADE_MAX_WIPED: i32 = 200;
/// ... and the defender must be expected to lose this share (percent) of what we lose, plus
/// `TRADE_MARGIN` Industry.
const TRADE_RATIO_PCT: i32 = 150;
const TRADE_MARGIN: i32 = 20;
/// What a Sentinel's fall is worth in a trade, in Industry.
const SENTINEL_VALUE: i32 = 20;
/// Free squads march on an enemy city only when together they have this share (percent) of
/// what defends it.
const OFFENSE_EDGE_PCT: i32 = 130;
/// A guard leaves its city for an adjacent enemy squad only on a near-certain win.
const SORTIE_FALLS: i32 = 800;
const SORTIE_MAX_WIPED: i32 = 100;
/// A threatened city takes guards until they and its Sentinels have this share (percent) of the
/// power of the enemy squads that could reach it, up to `MAX_GUARDS`.
const GUARD_MATCH_PCT: i32 = 100;
const MAX_GUARDS: usize = 3;
/// Guards a held Lifeboat keeps.
const LIFEBOAT_GUARDS: usize = 2;
/// Strain from which King Cinder's Host and the Wild Clans' raiders are in play: the capital is
/// always guarded from here on.
const CAPITAL_GUARD_STRAIN: i32 = 50;
/// When this few turns of the Blowout countdown are left, a civilization without a Lifeboat attacks
/// its target on any good trade, gathered or not.
const LAST_TURNS: i32 = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    Guard(CityId),
    Assault(PlotIdx),
    Free,
}

/// A squad of a civilization at war with `p` (the Wild Clans excluded: their squads never move).
fn enemy_civ(g: &Game, p: PlayerId, owner: PlayerId) -> bool {
    owner != p && !g.players[owner.idx()].is_neutral && g.at_war(p, owner)
}

/// Something at war with `p` stands on `plot`: a squad or a city.
fn hostile_on(g: &Game, p: PlayerId, plot: PlotIdx) -> bool {
    g.squads.values().any(|s| s.plot == plot && s.owner != p && g.at_war(p, s.owner)) || hostile_city(g, p, plot).is_some()
}

fn hostile_city(g: &Game, p: PlayerId, plot: PlotIdx) -> Option<CityId> {
    let cid = g.world.plot(plot).city?;
    let c = g.cities.get(&cid)?;
    (c.owner != p && g.at_war(p, c.owner)).then_some(cid)
}

/// An enemy city with no squad and no standing Sentinel: an attack walks in and takes it.
fn walk_in(g: &Game, p: PlayerId, plot: PlotIdx) -> bool {
    let Some(cid) = hostile_city(g, p, plot) else { return false };
    g.cities[&cid].standing_sentinels() == 0 && !g.squads.values().any(|s| s.plot == plot && s.owner != p)
}

/// Expected losses of a battle in Industry (each fighter's chance to fall times its cost), ours
/// and the defender's. A Sentinel counts `SENTINEL_VALUE`: it is rebuilt by repair, not bought.
fn expected_losses(built: &BuiltFormation, fc: &Forecast) -> (i32, i32) {
    let mut lost = [0i32; 2];
    for (snap, &pct) in built.snaps.iter().zip(&fc.per_fall_pct) {
        let value = if snap.sentinel_idx.is_some() { SENTINEL_VALUE } else { snap.class.info().cost_industry };
        lost[snap.side.index()] += value * pct;
    }
    (lost[0] / 100, lost[1] / 100)
}

/// The plot holds, but the defender loses clearly more than we do.
fn good_trade(built: &BuiltFormation, fc: &Forecast) -> bool {
    let (ours, theirs) = expected_losses(built, fc);
    fc.attackers_wiped_permille <= TRADE_MAX_WIPED && theirs * 100 >= ours * TRADE_RATIO_PCT + TRADE_MARGIN
}

/// Why an attack is worth making, or `None` when the squad should hold.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Reason {
    WalkIn,
    Falls,
    Trade,
}

fn attack_reason(g: &Game, p: PlayerId, sid: SquadId, to: PlotIdx, allow_trade: bool) -> Option<Reason> {
    let s = g.squads.get(&sid)?;
    if s.moves_left <= 0 || g.world.distance(s.plot, to) != 1 || !hostile_on(g, p, to) {
        return None;
    }
    if walk_in(g, p, to) {
        return Some(Reason::WalkIn);
    }
    let (built, fc) = g.preview_attack(p, sid, to).ok()?;
    // The forecast is for one defending squad (and the Sentinels): a city held by two squads does
    // not fall to one battle. On open ground the losing side's squads are all pushed off.
    let in_city = g.world.plot(to).city.is_some();
    let defenders = g.squads.values().filter(|o| o.plot == to && o.owner != p && g.at_war(p, o.owner)).count();
    if fc.plot_falls_permille >= FALLS_TO_ATTACK && (!in_city || defenders <= 1) {
        return Some(Reason::Falls);
    }
    if allow_trade && good_trade(&built, &fc) {
        return Some(Reason::Trade);
    }
    None
}

fn attack(g: &mut Game, p: PlayerId, sid: SquadId, to: PlotIdx, events: &mut Vec<Event>) -> bool {
    try_cmd(g, p, Command::AttackPlot { squad: sid, to }, events)
}

/// Distances (in steps) to `goal` over the plots a squad of `p` may step on, from every plot.
/// A plot holding a squad or city at war with us is a battle, not a step, so it is closed (the
/// goal itself excepted); so is a city of a civilization we are at peace with.
fn distance_field(g: &Game, p: PlayerId, goal: PlotIdx) -> Vec<i32> {
    let n = g.world.len();
    let mut blocked = vec![false; n];
    for s in g.squads.values() {
        if s.owner != p && g.at_war(p, s.owner) {
            blocked[s.plot.idx()] = true;
        }
    }
    for c in g.cities.values() {
        if c.owner != p {
            blocked[c.plot.idx()] = true;
        }
    }
    blocked[goal.idx()] = false;
    let mut dist = vec![i32::MAX; n];
    let mut q = VecDeque::new();
    dist[goal.idx()] = 0;
    q.push_back(goal);
    while let Some(u) = q.pop_front() {
        // Do not expand through a closed plot (the goal is open only as an end).
        if u != goal && blocked[u.idx()] {
            continue;
        }
        let du = dist[u.idx()];
        for nb in g.world.neighbors(u) {
            if dist[nb.idx()] != i32::MAX || !g.world.plot(nb).kind.is_passable_land() {
                continue;
            }
            dist[nb.idx()] = du + 1;
            if !blocked[nb.idx()] {
                q.push_back(nb);
            }
        }
    }
    dist
}

/// Fields cached for one call of `manage_squads`; a battle can change what blocks, so it clears.
#[derive(Default)]
struct Paths {
    fields: BTreeMap<PlotIdx, Vec<i32>>,
}

impl Paths {
    fn dist(&mut self, g: &Game, p: PlayerId, from: PlotIdx, goal: PlotIdx) -> i32 {
        self.fields.entry(goal).or_insert_with(|| distance_field(g, p, goal))[from.idx()]
    }

    /// The next step from `from` toward `goal` (never onto a plot with something hostile on it).
    fn step(&mut self, g: &Game, p: PlayerId, from: PlotIdx, goal: PlotIdx) -> Option<PlotIdx> {
        let field = self.fields.entry(goal).or_insert_with(|| distance_field(g, p, goal));
        let here = field[from.idx()];
        g.world
            .neighbors(from)
            .into_iter()
            .filter(|&nb| field[nb.idx()] < here && !hostile_on(g, p, nb) && g.world.plot(nb).kind.is_passable_land())
            .filter(|&nb| g.world.plot(nb).city.and_then(|c| g.cities.get(&c)).is_none_or(|c| c.owner == p))
            .min_by_key(|&nb| (field[nb.idx()], !g.world.plot(nb).road, nb))
    }

    fn clear(&mut self) {
        self.fields.clear();
    }
}

/// Moves `sid` toward `goal`, at most until it stands next to it (or on it, when `enter`).
fn march(g: &mut Game, p: PlayerId, sid: SquadId, goal: PlotIdx, enter: bool, paths: &mut Paths, events: &mut Vec<Event>) {
    for _ in 0..6 {
        let Some(s) = g.squads.get(&sid) else { return };
        if s.moves_left <= 0 || s.plot == goal || (!enter && g.world.distance(s.plot, goal) <= 1) {
            return;
        }
        let from = s.plot;
        let Some(step) = paths.step(g, p, from, goal) else { return };
        if !try_cmd(g, p, Command::MoveSquad { squad: sid, to: step }, events) {
            return;
        }
    }
}

/// Ends a squad's turn next to a target it does not attack: raid when the rules allow it, else
/// fortify.
fn hold(g: &mut Game, p: PlayerId, sid: SquadId, events: &mut Vec<Event>) {
    let Some(s) = g.squads.get(&sid) else { return };
    if s.moves_left <= 0 || matches!(s.order, SquadOrder::Raid | SquadOrder::Fortify) {
        return;
    }
    if g.check_raid(p, sid).is_ok() {
        try_cmd(g, p, Command::SetSquadRaid { squad: sid, active: true }, events);
    } else {
        try_cmd(g, p, Command::SetSquadFortify { squad: sid }, events);
    }
}

fn fortify(g: &mut Game, p: PlayerId, sid: SquadId, events: &mut Vec<Event>) {
    if g.squads.get(&sid).is_some_and(|s| s.moves_left > 0 && s.order != SquadOrder::Fortify) {
        try_cmd(g, p, Command::SetSquadFortify { squad: sid }, events);
    }
}

pub(crate) fn sentinel_power(g: &Game, cid: CityId) -> i32 {
    let si = UnitClassId::Sentinel.info();
    g.cities[&cid].sentinel_damage.iter().filter(|&&d| d < 100).map(|&d| (si.atk + si.def) * (100 - d) / 100).sum()
}

/// The cities that need a guard, most urgent first: (city, guards it always gets, the power of
/// the enemy squads that could reach it soon).
fn guard_needs(g: &Game, p: PlayerId) -> Vec<(CityId, usize, i32)> {
    let mut needs: Vec<(CityId, usize, i32)> = Vec::new();
    let threat = |cid: CityId| -> (i32, i32) {
        let plot = g.cities[&cid].plot;
        let near: Vec<(i32, i32)> = g
            .squads
            .values()
            .filter(|s| enemy_civ(g, p, s.owner))
            .map(|s| (g.world.distance(s.plot, plot), s.power_rating()))
            .filter(|&(d, _)| d <= THREAT_RANGE)
            .collect();
        (near.iter().map(|x| x.0).min().unwrap_or(i32::MAX), near.iter().map(|x| x.1).sum())
    };
    let add = |needs: &mut Vec<(CityId, usize, i32)>, cid: CityId, n: usize| match needs.iter_mut().find(|(c, _, _)| *c == cid) {
        Some((_, m, _)) => *m = (*m).max(n),
        None => needs.push((cid, n, threat(cid).1)),
    };
    let cities = g.player_cities(p);
    // 1. The Lifeboats we hold.
    if g.apocalypse.carcosa_risen {
        for &cid in &cities {
            if g.apocalypse.is_lifeboat(cid).is_some() {
                add(&mut needs, cid, LIFEBOAT_GUARDS);
            }
        }
    }
    // 2. The capital, while we are at war with a civilization or Strain is high.
    let at_war = g.players.iter().any(|o| o.alive && enemy_civ(g, p, o.id));
    if at_war || g.apocalypse.counter >= CAPITAL_GUARD_STRAIN || g.apocalypse.carcosa_risen {
        if let Some(&cap) = cities.iter().find(|c| g.cities[c].capital) {
            add(&mut needs, cap, 1);
        }
    }
    // 3. Every city an enemy civilization's squad could reach soon, the nearest threat first.
    let mut threatened: Vec<(i32, CityId)> = cities.iter().map(|&cid| (threat(cid).0, cid)).filter(|&(d, _)| d <= THREAT_RANGE).collect();
    threatened.sort();
    for (_, cid) in threatened {
        add(&mut needs, cid, 1);
    }
    needs
}

/// The Lifeboat this civilization's free squads go for after the Blowout, if any: the standing
/// Lifeboat held by someone we are at war with, or may declare war on, that has the best prospects.
fn pick_lifeboat_target(g: &Game, p: PlayerId, free: &[SquadId], paths: &mut Paths) -> Option<PlotIdx> {
    let our_power: i32 = free.iter().filter_map(|s| g.squads.get(s)).map(|s| s.power_rating()).sum();
    if free.is_empty() {
        return None;
    }
    let mut best: Option<(i64, PlotIdx)> = None;
    for lb in g.apocalypse.lifeboats.iter().filter(|l| !l.razed) {
        let Some(c) = g.cities.get(&lb.city_id) else { continue };
        // Held by someone we are at war with, or may go to war with (diplomacy declares on the pick).
        if c.owner == p || (!g.at_war(p, c.owner) && g.check_war(p, c.owner).is_err()) {
            continue;
        }
        // How far our squads have to go (the nearest one), over the plots they can step on.
        let dist = free.iter().filter_map(|s| g.squads.get(s)).map(|s| paths.dist(g, p, s.plot, c.plot)).min().unwrap_or(i32::MAX);
        if dist == i32::MAX {
            continue;
        }
        let def_power: i32 = g.squads.values().filter(|s| s.plot == c.plot && s.owner != p).map(|s| s.power_rating()).sum::<i32>() + sentinel_power(g, c.id);
        let mut score = our_power as i64 * 100 / (def_power as i64 + 1) - dist as i64 * 4;
        // Where a squad already stands next to it, the forecast says more than the ratings do.
        if let Some(fc) = free
            .iter()
            .filter(|s| g.squads.get(s).is_some_and(|q| g.world.distance(q.plot, c.plot) == 1))
            .filter_map(|&s| g.preview_attack(p, s, c.plot).ok().map(|(_, fc)| fc.plot_falls_permille))
            .max()
        {
            score += fc as i64 / 10;
        }
        if walk_in(g, p, c.plot) {
            score += 1000;
        }
        if best.is_none_or(|(b, bp)| score > b || (score == b && c.plot < bp)) {
            best = Some((score, c.plot));
        }
    }
    best.map(|(_, plot)| plot)
}

/// What defends an enemy city: the squads on it and next to it, and its Sentinels.
fn city_defense(g: &Game, p: PlayerId, c: &cataclysm_core::City) -> i32 {
    g.squads.values().filter(|s| s.owner != p && g.at_war(p, s.owner) && g.world.distance(s.plot, c.plot) <= 1).map(|s| s.power_rating()).sum::<i32>() + sentinel_power(g, c.id)
}

/// Where a free squad goes: the nearest enemy city our free squads together outmatch, an enemy
/// civilization's squad near it (in peace, also a Wild Clans squad or a lair); `None` when there
/// is nothing to do.
fn free_target(g: &Game, p: PlayerId, from: PlotIdx, guard_of: Option<PlotIdx>, free_power: i32) -> Option<PlotIdx> {
    // A civilization holding a Lifeboat attacks only what threatens it.
    if let Some(home) = guard_of {
        return g
            .squads
            .values()
            .filter(|s| enemy_civ(g, p, s.owner) && g.world.distance(s.plot, home) <= THREAT_RANGE + 1)
            .min_by_key(|s| (g.world.distance(from, s.plot), s.id))
            .map(|s| s.plot);
    }
    if let Some(c) = g
        .cities
        .values()
        .filter(|c| c.owner != p && g.at_war(p, c.owner) && !g.players[c.owner.idx()].is_neutral)
        .filter(|c| free_power * 100 >= city_defense(g, p, c) * OFFENSE_EDGE_PCT)
        .min_by_key(|c| (g.world.distance(from, c.plot), c.id))
    {
        return Some(c.plot);
    }
    // At war with a civilization, its squads; in peace, the Wild Clans' too (they guard lairs).
    let at_war = g.players.iter().any(|o| o.alive && enemy_civ(g, p, o.id));
    if let Some(s) = g
        .squads
        .values()
        .filter(|s| s.owner != p && g.at_war(p, s.owner) && (!at_war || enemy_civ(g, p, s.owner)) && g.world.distance(from, s.plot) <= 6)
        .min_by_key(|s| (g.world.distance(from, s.plot), s.id))
    {
        return Some(s.plot);
    }
    if at_war {
        return None;
    }
    (0..g.world.len())
        .map(|i| PlotIdx(i as u32))
        .filter(|&pt| g.world.plot(pt).lair_unexplored)
        .min_by_key(|&pt| (g.world.distance(from, pt), pt))
}

/// A plot on our land without a road, on the way from the capital to another of our cities.
fn road_work(g: &Game, p: PlayerId, from: PlotIdx, paths: &mut Paths) -> Option<PlotIdx> {
    if !g.players[p.idx()].has_tech(cataclysm_core::data::TECH_FIELDCRAFT) {
        return None;
    }
    let cities = g.player_cities(p);
    let cap = cities.iter().map(|c| &g.cities[c]).find(|c| c.capital)?.plot;
    let mut wanted: BTreeSet<PlotIdx> = BTreeSet::new();
    for c in cities.iter().map(|c| &g.cities[c]).filter(|c| c.plot != cap) {
        // Walk the shortest path from the other city to the capital.
        let mut at = c.plot;
        for _ in 0..20 {
            let Some(next) = paths.step(g, p, at, cap) else { break };
            if next == cap {
                break;
            }
            let pl = g.world.plot(next);
            if !pl.road && pl.owner == Some(p) && pl.city.is_none() {
                wanted.insert(next);
            }
            at = next;
        }
    }
    wanted.into_iter().min_by_key(|&pt| (g.world.distance(from, pt), pt))
}

pub(crate) fn manage_squads(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    let cap = g.players[p.idx()].squad_place_cap();
    for sid in g.player_squads(p) {
        // Upgrade or reinforce inside own borders when Gold allows.
        // Gold is kept for a raiders' tribute while a city of ours is being raided.
        let reserve = if g.player_cities(p).iter().any(|c| g.cities[c].raid_streak > 0) { cataclysm_core::game::RAID_TRIBUTE_GOLD } else { 0 };
        let in_home = g.world.plot(g.squads[&sid].plot).owner == Some(p);
        if in_home && g.players[p.idx()].gold >= 80 + reserve {
            // As many upgrades and recruits as the Gold allows (above the reserve), not one a turn.
            for _ in 0..8 {
                if g.players[p.idx()].gold < 80 + reserve {
                    break;
                }
                let upgrades: Vec<_> = g.squads[&sid]
                    .members
                    .iter()
                    .filter(|m| m.class.info().upgrades_to.is_some_and(|n| g.can_recruit_class(p, n).is_ok()))
                    .map(|m| m.id)
                    .collect();
                if upgrades.into_iter().any(|mid| try_cmd(g, p, Command::UpgradeMember { squad: sid, member: mid }, events)) {
                    continue;
                }
                if g.squads[&sid].places_used() >= cap {
                    break;
                }
                let rec = best_squad_recipe(g, p);
                let start = g.squads[&sid].members.len();
                let recruited = (0..rec.len()).any(|i| try_cmd(g, p, Command::ReinforceSquad { squad: sid, class: rec[(start + i) % rec.len()] }, events));
                if !recruited {
                    break;
                }
            }
            try_cmd(g, p, Command::AutoArrangeSquad { squad: sid }, events);
        }
    }

    let mut paths = Paths::default();
    let roles = assign_roles(g, p, &mut paths);

    // 1. Guards go to their cities and stay there.
    for (&sid, &role) in &roles {
        if let Role::Guard(cid) = role {
            run_guard(g, p, sid, cid, &mut paths, events);
        }
    }

    // 2. Everyone else: found a city or explore a lair where it stands, then march.
    let my_cities_count = g.player_cities(p).len();
    let holder_home = g.apocalypse.carcosa_risen.then(|| held_lifeboat(g, p)).flatten();
    let free_power: i32 = roles.iter().filter(|(_, r)| **r == Role::Free).filter_map(|(s, _)| g.squads.get(s)).map(|s| s.power_rating()).sum();
    let mut goals: BTreeMap<SquadId, PlotIdx> = BTreeMap::new();
    for (&sid, &role) in &roles {
        if matches!(role, Role::Guard(_)) {
            continue;
        }
        let Some(s) = g.squads.get(&sid) else { continue };
        if s.moves_left <= 0 {
            continue;
        }
        if g.world.plot(s.plot).lair_unexplored && try_cmd(g, p, Command::SquadExploreLair { squad: sid }, events) {
            continue;
        }
        let s = &g.squads[&sid];
        if my_cities_count < 4 && g.players[p.idx()].gold >= 130 && g.world.plot(s.plot).city.is_none() && g.cities.values().all(|c| g.world.distance(c.plot, s.plot) >= 4) {
            if try_cmd(g, p, Command::SquadFoundCity { squad: sid, name: None }, events) {
                continue;
            }
        }
        let from = g.squads[&sid].plot;
        let goal = match role {
            Role::Assault(t) => Some(t),
            _ => free_target(g, p, from, holder_home, free_power),
        };
        let Some(goal) = goal else {
            run_idle(g, p, sid, holder_home, &mut paths, events);
            continue;
        };
        let enter = !hostile_on(g, p, goal);
        march(g, p, sid, goal, enter, &mut paths, events);
        if g.squads.get(&sid).is_some_and(|s| s.plot == goal && g.world.plot(goal).lair_unexplored) {
            try_cmd(g, p, Command::SquadExploreLair { squad: sid }, events);
            continue;
        }
        goals.insert(sid, goal);
    }

    // 3. At each target, the squads next to it attack together, strongest first, when the
    //    forecast says so; the others hold.
    let mut by_goal: BTreeMap<PlotIdx, Vec<SquadId>> = BTreeMap::new();
    for (&sid, &goal) in &goals {
        by_goal.entry(goal).or_default().push(sid);
    }
    for (goal, sids) in by_goal {
        let assault = sids.iter().any(|s| roles.get(s) == Some(&Role::Assault(goal)));
        resolve_target(g, p, goal, &sids, assault, &mut paths, events);
    }
}

/// The power of our squads that are not home guards this turn (the roles `manage_squads` gives).
pub(crate) fn free_power(g: &Game, p: PlayerId) -> i32 {
    let roles = assign_roles(g, p, &mut Paths::default());
    roles.iter().filter(|(_, r)| !matches!(r, Role::Guard(_))).filter_map(|(s, _)| g.squads.get(s)).map(|s| s.power_rating()).sum()
}

/// The Lifeboat our squads are sent at this turn, if any (the same pick `manage_squads` makes).
pub(crate) fn lifeboat_assault_target(g: &Game, p: PlayerId) -> Option<PlotIdx> {
    assign_roles(g, p, &mut Paths::default()).values().find_map(|r| match r {
        Role::Assault(t) => Some(*t),
        _ => None,
    })
}

/// One of our squads stands next to a city of `e` and would take it now on the forecast.
pub(crate) fn can_take_city_of(g: &Game, p: PlayerId, e: PlayerId) -> bool {
    g.cities.values().filter(|c| c.owner == e).any(|c| {
        g.squads
            .values()
            .filter(|s| s.owner == p && g.world.distance(s.plot, c.plot) == 1)
            .any(|s| matches!(attack_reason(g, p, s.id, c.plot, false), Some(Reason::WalkIn | Reason::Falls)))
    })
}

/// The Lifeboat city we hold, if any (the plot).
pub(crate) fn held_lifeboat(g: &Game, p: PlayerId) -> Option<PlotIdx> {
    g.apocalypse.lifeboats.iter().filter(|l| !l.razed).filter_map(|l| g.cities.get(&l.city_id)).filter(|c| c.owner == p).map(|c| c.plot).min()
}

fn assign_roles(g: &Game, p: PlayerId, paths: &mut Paths) -> BTreeMap<SquadId, Role> {
    let mut roles: BTreeMap<SquadId, Role> = g.player_squads(p).into_iter().map(|s| (s, Role::Free)).collect();
    let needs = guard_needs(g, p);
    let mut pick_guard = |roles: &mut BTreeMap<SquadId, Role>, cid: CityId| -> bool {
        let plot = g.cities[&cid].plot;
        let pick = roles.iter().filter(|(_, r)| **r == Role::Free).map(|(&s, _)| s).min_by_key(|s| (paths.dist(g, p, g.squads[s].plot, plot), *s));
        match pick {
            Some(s) if paths.dist(g, p, g.squads[&s].plot, plot) != i32::MAX => {
                roles.insert(s, Role::Guard(cid));
                true
            }
            _ => false,
        }
    };
    // First every city gets the guards it always has; then, most urgent first, more guards
    // until they and the Sentinels match the enemy squads that could reach the city.
    for &(cid, n, _) in &needs {
        for _ in 0..n {
            if !pick_guard(&mut roles, cid) {
                break;
            }
        }
    }
    for &(cid, _, threat) in &needs {
        loop {
            let guards: Vec<SquadId> = roles.iter().filter(|(_, r)| **r == Role::Guard(cid)).map(|(&s, _)| s).collect();
            let power: i32 = guards.iter().map(|s| g.squads[s].power_rating()).sum::<i32>() + sentinel_power(g, cid);
            if guards.len() >= MAX_GUARDS || power * 100 >= threat * GUARD_MATCH_PCT || !pick_guard(&mut roles, cid) {
                break;
            }
        }
    }
    // After the Blowout, a civilization without a Lifeboat (and King Cinder's Host, which razes
    // them) sends every free squad at one Lifeboat.
    let infernal = g.players[p.idx()].faction == FactionId::Infernals;
    if g.apocalypse.carcosa_risen && (infernal || held_lifeboat(g, p).is_none()) {
        let free: Vec<SquadId> = roles.iter().filter(|(_, r)| **r == Role::Free).map(|(&s, _)| s).collect();
        if let Some(target) = pick_lifeboat_target(g, p, &free, paths) {
            for s in free {
                roles.insert(s, Role::Assault(target));
            }
        }
    }
    roles
}

fn run_guard(g: &mut Game, p: PlayerId, sid: SquadId, cid: CityId, paths: &mut Paths, events: &mut Vec<Event>) {
    let Some(city_plot) = g.cities.get(&cid).filter(|c| c.owner == p).map(|c| c.plot) else { return };
    march(g, p, sid, city_plot, true, paths, events);
    let Some(s) = g.squads.get(&sid) else { return };
    if s.plot != city_plot {
        return;
    }
    // A sortie against an adjacent enemy squad, only on a near-certain win and when the city is not
    // left open to another enemy.
    let others_home = g.squads.values().filter(|o| o.owner == p && o.plot == city_plot && o.id != sid).count();
    let targets: Vec<PlotIdx> = g
        .world
        .neighbors(city_plot)
        .into_iter()
        .filter(|&nb| g.world.plot(nb).city.is_none() && g.squads.values().any(|o| o.plot == nb && o.owner != p && g.at_war(p, o.owner)))
        .collect();
    for to in targets {
        let other_threat = g.squads.values().any(|o| enemy_civ(g, p, o.owner) && o.plot != to && g.world.distance(o.plot, city_plot) <= THREAT_RANGE);
        if others_home == 0 && other_threat {
            continue;
        }
        if let Ok((_, fc)) = g.preview_attack(p, sid, to) {
            if fc.plot_falls_permille >= SORTIE_FALLS && fc.attackers_wiped_permille <= SORTIE_MAX_WIPED && g.squads[&sid].moves_left > 0 {
                attack(g, p, sid, to, events);
                paths.clear();
                return;
            }
        }
    }
    fortify(g, p, sid, events);
}

/// A squad with no target: improve the land between our cities, else wait in the capital (or the
/// Lifeboat we hold).
fn run_idle(g: &mut Game, p: PlayerId, sid: SquadId, home: Option<PlotIdx>, paths: &mut Paths, events: &mut Vec<Event>) {
    let from = g.squads[&sid].plot;
    let at_war = g.players.iter().any(|o| o.alive && enemy_civ(g, p, o.id));
    if home.is_none() && !at_war {
        if let Some(work) = road_work(g, p, from, paths) {
            march(g, p, sid, work, true, paths, events);
            if g.squads.get(&sid).is_some_and(|s| s.plot == work) && g.check_field_work(p, sid, true).is_ok() {
                try_cmd(g, p, Command::SquadBuildRoad { squad: sid }, events);
            }
            return;
        }
    }
    // At war, the spare squads wait in the city the most enemy power could reach soon.
    let threatened = at_war
        .then(|| {
            g.cities
                .values()
                .filter(|c| c.owner == p)
                .map(|c| {
                    let power: i32 = g.squads.values().filter(|s| enemy_civ(g, p, s.owner) && g.world.distance(s.plot, c.plot) <= THREAT_RANGE + 1).map(|s| s.power_rating()).sum();
                    (power, std::cmp::Reverse(c.id), c.plot)
                })
                .filter(|x| x.0 > 0)
                .max()
                .map(|x| x.2)
        })
        .flatten();
    let rest = home.or(threatened).or_else(|| g.cities.values().find(|c| c.owner == p && c.capital).map(|c| c.plot));
    if let Some(rest) = rest {
        march(g, p, sid, rest, true, paths, events);
        if g.squads.get(&sid).is_some_and(|s| s.plot == rest) {
            fortify(g, p, sid, events);
        }
    }
}

fn resolve_target(g: &mut Game, p: PlayerId, goal: PlotIdx, sids: &[SquadId], assault: bool, paths: &mut Paths, events: &mut Vec<Event>) {
    let adjacent = |g: &Game| -> Vec<SquadId> {
        let mut v: Vec<SquadId> = sids.iter().copied().filter(|s| g.squads.get(s).is_some_and(|q| q.moves_left > 0 && g.world.distance(q.plot, goal) == 1)).collect();
        v.sort_by_key(|s| (std::cmp::Reverse(g.squads[s].power_rating()), *s));
        v
    };
    let ready = adjacent(g);
    if ready.is_empty() || !hostile_on(g, p, goal) {
        return;
    }
    // Start only when it is worth it: the strongest squad alone is likely to take the plot, or a
    // trade is good and the squads are gathered (two or more here, or all of them, or time is short).
    let gathered = ready.len() >= 2 || ready.len() == sids.len();
    let late = assault && g.apocalypse.countdown.is_some_and(|t| t <= LAST_TURNS);
    let start = attack_reason(g, p, ready[0], goal, gathered || late || !assault).is_some();
    if start {
        for sid in ready.iter().copied() {
            if !hostile_on(g, p, goal) {
                // Taken: the others move in (it is ours now) or stop.
                if g.world.plot(goal).city.is_some() {
                    try_cmd(g, p, Command::MoveSquad { squad: sid, to: goal }, events);
                }
                continue;
            }
            if attack_reason(g, p, sid, goal, true).is_some() {
                attack(g, p, sid, goal, events);
                paths.clear();
            }
        }
    }
    for sid in ready {
        if g.squads.get(&sid).is_some_and(|s| s.moves_left > 0 && g.world.distance(s.plot, goal) == 1) {
            hold(g, p, sid, events);
        }
    }
}

/// Sally posture, once per turn: a lone guard with more than one enemy squad near stays inside (a
/// lost sally leaves the city empty); otherwise it sallies when it is the stronger (the rules'
/// "Favoured").
pub(crate) fn manage_sallies(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    for cid in g.player_cities(p) {
        let c = &g.cities[&cid];
        let home = g.squads.values().filter(|s| s.owner == p && s.plot == c.plot).count();
        let near = g.squads.values().filter(|s| enemy_civ(g, p, s.owner) && g.world.distance(s.plot, c.plot) <= 3).count();
        let want = if home <= 1 && near >= 2 { SallyPosture::Never } else { SallyPosture::Favoured };
        if c.sally != want {
            try_cmd(g, p, Command::CitySallyPosture { city: cid, posture: want }, events);
        }
    }
}

/// Raids are settled at the end of our turn. The tribute ends the raids of this turn only (the
/// raider can raid again next turn), so it is paid only to stop a city's countryside being
/// stripped: when the next raid would strip it and our squads did not drive the raiders off.
pub(crate) fn manage_raiders(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    let mut raiders: BTreeSet<PlayerId> = BTreeSet::new();
    for cid in g.player_cities(p) {
        let c = &g.cities[&cid];
        if c.raid_streak + 1 < RAID_STREAK_TO_STRIP {
            continue;
        }
        for s in g.squads.values() {
            if s.order == SquadOrder::Raid && g.at_war(p, s.owner) && g.world.plot(s.plot).owner == Some(p) && g.world.distance(s.plot, c.plot) <= cataclysm_core::game::RAID_RANGE {
                raiders.insert(s.owner);
            }
        }
    }
    for r in raiders {
        if g.players[p.idx()].gold >= cataclysm_core::game::RAID_TRIBUTE_GOLD {
            try_cmd(g, p, Command::PayOffRaiders { raider: r }, events);
        }
    }
}
