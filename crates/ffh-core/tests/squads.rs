//! Squads (docs/ENGAGEMENTS.md, "Squads"; `army/squad.rs`, `engagement/squads.rs`): the arrival
//! rule, hand-set cells kept, moves and swaps, auto-sort, a whole squad split off keeping its id,
//! the defender's rotation (the squads that fought the fewest battles this turn), saves.

use ffh_core::army::{SquadBody, SquadDest};
use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::engagement::Side;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map, players 0 and 1 at war (both AI), `from` and `to` two flat grassland plots side by
/// side with no feature, improvement or river crossing.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    from: PlotIdx,
    to: PlotIdx,
}

fn world() -> World {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    let mut g = Game::new(d, setup);
    g.game_init();
    let flat = |g: &Game, p: PlotIdx| {
        let plot = &g.map.plots[p];
        !g.plot_is_water(p) && !g.plot_is_peak(p) && !g.plot_is_hills(p) && plot.units.is_empty() && plot.owner.is_none() && plot.feature_type.is_none() && plot.improvement_type.is_none()
    };
    let (from, to) = g
        .map
        .plot_ids()
        .filter(|&p| flat(&g, p))
        .find_map(|p| {
            let (x, y) = g.map.xy(p);
            let q = g.map.plot_xy(x + 1, y)?;
            (flat(&g, q) && !g.plot_is_river_crossing(p, Some(DirectionType::DIRECTION_EAST))).then_some((p, q))
        })
        .expect("two flat plots side by side");
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    g.map.plots[from].terrain_type = Some(grass);
    g.map.plots[to].terrain_type = Some(grass);
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_declare_war(t0, t1, false, None);
    World { g, p0, p1, from, to }
}

impl World {
    fn units(&mut self, owner: PlayerId, name: &str, at: PlotIdx, n: usize) -> Vec<UnitId> {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        let us = (0..n).map(|_| self.g.player_init_unit(owner, ty, x, y, None, None)).collect();
        self.g.player_army_sync(owner);
        us
    }

    /// The squads of `p` on `at`, each as `(squad, [(unit, cell)])`.
    fn squads(&self, p: PlayerId, at: PlotIdx) -> Vec<(SquadId, Vec<(UnitId, u8)>)> {
        self.g.squad_layouts_at(p, at).into_iter().map(|l| (l.squad.expect("synced"), l.units())).collect()
    }
}

/// A new unit joins the first squad with room and a free cell in its best row, else forms a
/// squad of one: the fourth archer of an army finds the middle row full.
#[test]
fn arrivals_take_their_best_row_or_form_a_squad() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 1);
    let archers = w.units(p0, "UNIT_ARCHER", from, 4);
    let all: Vec<UnitId> = axemen.iter().chain(&archers).copied().collect();
    let army = w.g.army_form(p0, &all).expect("an army");
    let sq = w.squads(p0, from);
    assert_eq!(sq.len(), 2, "{sq:?}");
    let (first, second) = (&sq[0].1, &sq[1].1);
    assert_eq!(first.len(), 4);
    assert!(axemen.iter().all(|a| first.iter().any(|&(u, c)| u == *a && c / 3 == 0)), "the axeman in front: {first:?}");
    assert_eq!(first.iter().filter(|&&(u, c)| archers.contains(&u) && c / 3 == 1).count(), 3, "three archers in the middle");
    assert_eq!(second.len(), 1, "the fourth archer is a squad of one");
    for (s, _) in &sq {
        assert_eq!(w.g.squads[*s].body, SquadBody::Army(army));
        assert!(!w.g.squads[*s].hand_set);
    }
    // 5 places fit one squad: the AI's rule masses them.
    assert_eq!(w.g.squad_bodies_to_mass(p0), vec![(from, SquadBody::Army(army))]);
}

/// A moved unit keeps its cell, both squads become hand-set, a unit there swaps with it, a full
/// squad refuses a unit, and Auto gives a squad back to the auto formation.
#[test]
fn moves_swaps_and_auto() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 3);
    let archers = w.units(p0, "UNIT_ARCHER", from, 3);
    let all: Vec<UnitId> = axemen.iter().chain(&archers).copied().collect();
    w.g.army_form(p0, &all).expect("an army");
    let sq = w.squads(p0, from);
    let (one, two) = (sq[0].0, sq[1].0);
    let lone = sq[1].1[0].0;
    // The squad of five is full.
    assert!(w.g.squad_move(p0, lone, SquadDest::Squad(one), Some(7)).is_err(), "squad I has no free place");
    // A swap: the lone archer takes an axeman's front cell; the axeman goes to squad II.
    let ax = axemen[0];
    let cell = w.g.units[ax].squad_cell;
    w.g.squad_move(p0, lone, SquadDest::Squad(one), Some(cell)).expect("a swap");
    assert_eq!((w.g.units[lone].squad, w.g.units[lone].squad_cell), (Some(one), cell));
    assert_eq!(w.g.units[ax].squad, Some(two));
    assert!(w.g.squads[one].hand_set && w.g.squads[two].hand_set);
    // Kept through syncs and a save.
    w.g.player_army_sync(p0);
    let g2 = Game::load(w.g.d, &w.g.save()).expect("a save of this build");
    assert_eq!((g2.units[lone].squad, g2.units[lone].squad_cell), (Some(one), cell));
    assert!(g2.squads[one].hand_set);
    // Auto: the archer goes back to a middle or rear cell.
    w.g.squad_set_auto(p0, one).expect("auto");
    assert!(!w.g.squads[one].hand_set);
    assert!(w.g.units[lone].squad_cell / 3 > 0, "the auto formation puts the archer behind");
    // A new squad of its own.
    w.g.squad_move(p0, axemen[1], SquadDest::New, Some(1)).expect("a new squad");
    let s = w.g.units[axemen[1]].squad.unwrap();
    assert!(s != one && s != two);
    assert_eq!(w.g.units[axemen[1]].squad_cell, 1);
    // No cell given: the first free cell of its best row (an archer: the middle row).
    w.g.squad_move(p0, archers[0], SquadDest::Squad(s), None).expect("a move");
    assert_eq!((w.g.units[archers[0]].squad, w.g.units[archers[0]].squad_cell / 3), (Some(s), 1));
}

/// Auto-sort deals the chosen squads' units into as few squads as hold them, guards first in
/// snake order: each squad has a front.
#[test]
fn auto_sort_evens_the_squads() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 2);
    let archers = w.units(p0, "UNIT_ARCHER", from, 7);
    let all: Vec<UnitId> = axemen.iter().chain(&archers).copied().collect();
    w.g.army_form(p0, &all).expect("an army");
    // Arrivals: [2 axemen + 3 archers], [3 archers], [1 archer]: three squads for 9 places.
    let sq = w.squads(p0, from);
    assert_eq!(sq.len(), 3, "{sq:?}");
    assert_eq!(w.g.squad_bodies_to_mass(p0).len(), 1);
    let ids: Vec<SquadId> = sq.iter().map(|s| s.0).collect();
    w.g.squad_auto_sort(p0, &ids).expect("sorted");
    let sq = w.squads(p0, from);
    assert_eq!(sq.len(), 2, "{sq:?}");
    for (s, units) in &sq {
        assert!(!w.g.squads[*s].hand_set);
        assert!(units.iter().any(|&(u, c)| axemen.contains(&u) && c / 3 == 0), "each squad has an axeman in front: {units:?}");
    }
    // The snake splits the axemen; the AI's massed deal keeps them together: still to mass.
    assert_eq!(w.g.squad_bodies_to_mass(p0).len(), 1);
}

/// A whole squad of two or more split off into a new army keeps its squad; a lone unit arrives.
#[test]
fn a_squad_split_off_keeps_its_id() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 3);
    let archers = w.units(p0, "UNIT_ARCHER", from, 4);
    let all: Vec<UnitId> = axemen.iter().chain(&archers).copied().collect();
    w.g.army_form(p0, &all).expect("an army");
    let sq = w.squads(p0, from);
    let two = sq[1].0;
    let pair: Vec<UnitId> = sq[1].1.iter().map(|&(u, _)| u).collect();
    assert_eq!(pair.len(), 2);
    let army = w.g.army_split(p0, &pair).expect("a new army");
    assert!(pair.iter().all(|&u| w.g.units[u].squad == Some(two)));
    assert_eq!(w.g.squads[two].body, SquadBody::Army(army));
    // A lone unit split off arrives in its new army: a squad of its own there.
    let lone = sq[0].1[0].0;
    let solo = w.g.army_split(p0, &[lone]).expect("a new army");
    let s = w.g.units[lone].squad.expect("a squad");
    assert!(s != sq[0].0 && w.g.squads[s].body == SquadBody::Army(solo));
}

/// The defenders field a squad that fought the fewest battles this turn: two attacks in a turn on
/// a plot held by two squads meet both.
#[test]
fn the_defenders_rotate_their_squads() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p1].human = true;
    // Two defending squads (two armies of warriors).
    let a = w.units(p1, "UNIT_WARRIOR", to, 3);
    let b = w.units(p1, "UNIT_WARRIOR", to, 3);
    let army_a = w.g.army_form(p1, &a).expect("an army");
    w.g.army_form(p1, &b).expect("an army");
    assert_eq!(w.squads(p1, to).len(), 2);
    // Two attacking armies of one axeman each, weak enough not to take the plot.
    let x1 = w.units(p0, "UNIT_WARRIOR", from, 1);
    let x2 = w.units(p0, "UNIT_WARRIOR", from, 1);
    let mut defenders: Vec<Vec<UnitId>> = Vec::new();
    for x in [x1[0], x2[0]] {
        let before = w.g.battle_report_next;
        let grp = w.g.units[x].group.expect("a group");
        let (tx, ty) = w.g.map.xy(to);
        w.g.group_attack_force(grp, tx, ty, MOVE_DIRECT_ATTACK, None).expect("an attack");
        let r = w.g.battle_reports.iter().find(|r| r.id >= before).expect("a battle");
        let log = r.log.as_ref().expect("a human's battle is logged");
        defenders.push(log.fighters.iter().filter(|f| f.side == Side::Defender && f.place.is_some()).map(|f| f.unit).collect());
    }
    let in_a = |d: &Vec<UnitId>| d.iter().all(|u| a.contains(u));
    let in_b = |d: &Vec<UnitId>| d.iter().all(|u| b.contains(u));
    assert!((in_a(&defenders[0]) && in_b(&defenders[1])) || (in_b(&defenders[0]) && in_a(&defenders[1])), "{defenders:?}");
    let _ = army_a;
}

/// A drop on another body's squad moves the unit to that body (army to army; army to garrison
/// is the same path); a refusal changes nothing.
#[test]
fn a_move_to_another_bodys_squad_changes_its_body() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let first = w.units(p0, "UNIT_AXEMAN", from, 2);
    let second = w.units(p0, "UNIT_ARCHER", from, 3);
    let third = w.units(p0, "UNIT_AXEMAN", from, 1);
    let a = w.g.army_form(p0, &first).expect("an army");
    let b = w.g.army_form(p0, &second).expect("another army");
    let c = w.g.army_form(p0, &third).expect("a third army");
    let target = w.g.units[second[0]].squad.expect("a squad");
    assert_eq!(w.g.unit_squad_body(first[0]), SquadBody::Army(a));
    // Room in the squad (3 of 5 places): the unit changes army and stands in the squad.
    w.g.squad_move(p0, first[0], SquadDest::Squad(target), None).expect("a move into another army");
    assert_eq!(w.g.unit_squad_body(first[0]), SquadBody::Army(b));
    assert_eq!(w.g.units[first[0]].squad, Some(target));
    // The last place; the first army is left without units and is gone.
    w.g.squad_move(p0, first[1], SquadDest::Squad(target), None).expect("the last place");
    assert_eq!(w.g.units[first[1]].squad, Some(target));
    assert!(!w.g.armies.contains(a));
    // The squad is full: a refusal changes nothing.
    assert!(w.g.squad_move(p0, third[0], SquadDest::Squad(target), None).is_err(), "no place left");
    assert_eq!(w.g.unit_squad_body(third[0]), SquadBody::Army(c));
    assert_ne!(w.g.units[third[0]].squad, Some(target));
}
