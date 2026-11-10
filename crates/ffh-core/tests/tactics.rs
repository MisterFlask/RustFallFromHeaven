//! Battle tactics (docs/TACTICS.md): the data, whose tactic a battle uses, the tactic's formation, the
//! change at the start of the owner's next turn, new, split, captured bodies, and save/load. The
//! tactics' selection and placement on hand-built pools are unit tests
//! (`engagement/formation/tactic_tests.rs`).

use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::engagement::{HitCtx, Side};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map, players 0 and 1 at war, `from` and `to` two unowned flat grassland plots side by
/// side with no feature, improvement or river crossing; the starting units gone.
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
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    for p in [p0, p1] {
        for u in g.players[p].units.clone() {
            g.unit_kill(u, false, None);
        }
    }
    let flat = |g: &Game, p: PlotIdx| {
        let plot = &g.map.plots[p];
        !g.plot_is_water(p)
            && !g.plot_is_peak(p)
            && !g.plot_is_hills(p)
            && plot.units.is_empty()
            && plot.owner.is_none()
            && plot.feature_type.is_none()
            && plot.improvement_type.is_none()
            && g.plot_defense_modifier(p, None, false) == 0
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
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_declare_war(t0, t1, false, None);
    World { g, p0, p1, from, to }
}

impl World {
    fn units(&mut self, owner: PlayerId, name: &str, at: PlotIdx, n: usize) -> Vec<UnitId> {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        (0..n).map(|_| self.g.player_init_unit(owner, ty, x, y, None, None)).collect()
    }

    fn army(&mut self, owner: PlayerId, units: &[UnitId]) -> ArmyId {
        self.g.army_form(owner, units).expect("an army")
    }

    fn tactic(&self, name: &str) -> BattleTacticType {
        self.g.d.t::<BattleTacticType>(name)
    }

    fn ctx(&self) -> HitCtx {
        HitCtx::new(self.from, self.to)
    }
}

#[test]
fn the_four_tactics() {
    let d = ffh_data::global();
    assert_eq!(d.battle_tactics.len(), 4);
    let k = |name: &str| &d.battle_tactics[d.t::<BattleTacticType>(name)].knobs;
    assert_eq!(d.t::<BattleTacticType>("TACTIC_LINE"), BattleTacticType(0), "Line is the first entry");
    assert_eq!(*k("TACTIC_LINE"), TacticKnobs::line());
    assert_eq!(k("TACTIC_CHARGE").pick_order.first(), Some(&TacticKind::Charge));
    assert!(!k("TACTIC_CHARGE").one_of_each);
    assert_eq!(k("TACTIC_CHARGE").place_first, [TacticKind::Charge]);
    assert_eq!(k("TACTIC_VOLLEY").screen_per_lane, 1);
    assert!(!k("TACTIC_VOLLEY").one_of_each);
    assert!(k("TACTIC_VOLLEY").pick_order[..5].iter().all(|k| !k.guard()), "support before the other guards");
    assert_eq!(k("TACTIC_VANGUARD").placement, TacticPlacement::Vanguard);
    for a in d.battle_tactics.iter() {
        assert_ne!(d.text.get(&a.base.description), a.base.description.as_str(), "{}: a name", a.base.type_);
        assert_ne!(d.text.get(&a.base.help), a.base.help.as_str(), "{}: a help", a.base.type_);
    }
    assert_eq!(d.defines.tactic_veteran_level, 4);
}

#[test]
fn whose_tactic_a_side_fights_in() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let line = w.tactic("TACTIC_LINE");
    let charge = w.tactic("TACTIC_CHARGE");
    let volley = w.tactic("TACTIC_VOLLEY");
    let axes = w.units(p0, "UNIT_AXEMAN", from, 3);
    let bows = w.units(p0, "UNIT_ARCHER", from, 2);
    let a = w.army(p0, &axes);
    let b = w.army(p0, &bows);
    assert_eq!(w.g.armies[a].tactic, line, "a new army starts in Line");
    w.g.armies[a].tactic = charge;
    w.g.armies[b].tactic = volley;
    let mut pool = axes.clone();
    pool.extend(&bows);
    assert_eq!(w.g.engage_side_tactic(&pool, from), charge, "the army most of the units belong to");
    assert_eq!(w.g.engage_side_tactic(&bows, from), volley);
    // A tie between armies: the lower id.
    let two: Vec<UnitId> = vec![axes[0], axes[1], bows[0], bows[1]];
    assert_eq!(w.g.engage_side_tactic(&two, from), w.g.armies[a.min(b)].tactic);
    // Units of no army (a hero, a mage) count for nobody.
    let adepts = w.units(p0, "UNIT_ADEPT", from, 4);
    w.g.player_army_sync(p0);
    assert!(adepts.iter().all(|&u| w.g.units[u].army.is_none()));
    let mut with_mages = bows.clone();
    with_mages.extend(&adepts);
    assert_eq!(w.g.engage_side_tactic(&with_mages, from), volley);
    assert_eq!(w.g.engage_side_tactic(&adepts, from), line, "nobody's units: Line");
    // A city's garrison counts for the city; an army before the garrison on a tie.
    let (x, y) = w.g.map.xy(to);
    let c = w.g.player_init_city(p1, x, y, true, true);
    w.g.cities[c].garrison_tactic = volley;
    let garrison = w.units(p1, "UNIT_AXEMAN", to, 2);
    w.g.player_army_sync(p1);
    assert!(garrison.iter().all(|&u| w.g.unit_garrison_city(u) == Some(c)));
    assert_eq!(w.g.engage_side_tactic(&garrison, to), volley);
    let guests = w.units(p1, "UNIT_WARRIOR", to, 2);
    let army = w.army(p1, &guests);
    w.g.armies[army].tactic = charge;
    let mut all = garrison.clone();
    all.extend(&guests);
    assert_eq!(w.g.engage_side_tactic(&all, to), charge, "a tie: the army");
    let more = w.units(p1, "UNIT_AXEMAN", to, 1);
    w.g.player_army_sync(p1);
    all.extend(&more);
    assert_eq!(w.g.engage_side_tactic(&all, to), volley, "the garrison has more");
    // The defender's formation is its tactic's.
    let ctx = w.ctx();
    let slots = w.g.engage_slots(&ctx);
    assert_eq!(w.g.engage_defense_formation(&all, &slots, &ctx), w.g.engage_tactic_formation(&all, Side::Defender, &slots, &ctx, volley));
}

#[test]
fn barbarians_fight_in_line() {
    let mut w = world();
    let barb = w.g.barbarian_player();
    let line = w.tactic("TACTIC_LINE");
    let units = w.units(barb, "UNIT_AXEMAN", w.from, 3);
    assert_eq!(w.g.engage_side_tactic(&units, w.from), line);
}

#[test]
fn the_tactic_arranges_the_battle() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    // A pool where Vanguard and Line differ: 4 axemen (2 of them veterans) and 4 archers.
    let axes = w.units(p0, "UNIT_AXEMAN", from, 4);
    let mut pool = axes.clone();
    pool.extend(w.units(p0, "UNIT_ARCHER", from, 4));
    let vets = [axes[0], axes[1]];
    let veteran_level = w.g.d.defines.tactic_veteran_level;
    for &u in &vets {
        w.g.units[u].level = veteran_level;
    }
    let a = w.army(p0, &pool);
    w.units(p1, "UNIT_WARRIOR", to, 2);
    w.g.player_army_sync(p1);
    let ctx = w.ctx();
    let slots = w.g.engage_slots(&ctx);
    let vanguard = w.tactic("TACTIC_VANGUARD");
    let line_f = w.g.engage_auto_formation(&pool, Side::Attacker, &slots, &ctx);
    w.g.armies[a].tactic = vanguard;
    let vanguard_f = w.g.engage_auto_formation(&pool, Side::Attacker, &slots, &ctx);
    assert_ne!(line_f, vanguard_f);
    assert_eq!(vanguard_f, w.g.engage_tactic_formation(&pool, Side::Attacker, &slots, &ctx, vanguard));
    // Vanguard: in the centre lane a veteran axeman stands behind one that is no veteran.
    let lanes = w.g.engage_lanes(&slots)[0];
    let centre = (lanes - 1) / 2;
    let at = |l: u8, r: u8| vanguard_f.iter().find(|p| p.lane == l && p.row == r).map(|p| p.unit);
    let (front, middle) = (at(centre, 0).expect("a front unit"), at(centre, 1).expect("a middle unit"));
    assert!(axes.contains(&front) && !vets.contains(&front));
    assert!(vets.contains(&middle));
    // The battle uses it, and its log names both sides' tactics.
    w.g.players[p1].human = true;
    let grp = w.g.units[pool[0]].group.expect("a group");
    let (tx, ty) = w.g.map.xy(to);
    let before = w.g.battle_report_next;
    w.g.group_attack_force(grp, tx, ty, MOVE_DIRECT_ATTACK, None).expect("the attack");
    let r = w.g.battle_reports.iter().find(|r| r.id >= before).expect("a battle");
    let log = r.log.as_ref().expect("a log");
    assert_eq!(log.tactics, [vanguard, w.tactic("TACTIC_LINE")]);
    let placed: Vec<(UnitId, (u8, u8))> = log.fighters.iter().filter(|f| f.side == Side::Attacker).filter_map(|f| f.place.map(|p| (f.unit, p))).collect();
    for p in &vanguard_f {
        assert!(placed.contains(&(p.unit, (p.lane, p.row))), "the battle stood as the tactic put it");
    }
}

#[test]
fn a_change_takes_effect_at_the_owners_next_turn() {
    let mut w = world();
    let (p0, p1, from) = (w.p0, w.p1, w.from);
    let line = w.tactic("TACTIC_LINE");
    let charge = w.tactic("TACTIC_CHARGE");
    let axes = w.units(p0, "UNIT_AXEMAN", from, 2);
    let a = w.army(p0, &axes);
    assert_eq!(w.g.army_set_tactic(p1, a, charge), Err("no such army"));
    assert_eq!(w.g.army_set_tactic(p0, a, BattleTacticType(99)), Err("no such tactic"));
    w.g.army_set_tactic(p0, a, charge).unwrap();
    assert_eq!((w.g.armies[a].tactic, w.g.armies[a].tactic_next), (line, charge));
    assert_eq!(w.g.engage_side_tactic(&axes, from), line, "this turn: as it was set");
    let (x, y) = w.g.map.xy(w.to);
    let c = w.g.player_init_city(p0, x, y, true, true);
    assert_eq!(w.g.city_set_garrison_tactic(p1, c, charge), Err("not your city"));
    w.g.city_set_garrison_tactic(p0, c, charge).unwrap();
    assert_eq!(w.g.cities[c].garrison_tactic, line);
    // Another player's turn changes nothing; the owner's does.
    w.g.player_apply_tactics(p1);
    assert_eq!(w.g.armies[a].tactic, line);
    w.g.player_apply_tactics(p0);
    assert_eq!(w.g.armies[a].tactic, charge);
    assert_eq!(w.g.cities[c].garrison_tactic, charge);
    assert_eq!(w.g.engage_side_tactic(&axes, from), charge);
}

#[test]
fn new_split_and_captured_bodies() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let line = w.tactic("TACTIC_LINE");
    let volley = w.tactic("TACTIC_VOLLEY");
    let charge = w.tactic("TACTIC_CHARGE");
    let axes = w.units(p0, "UNIT_AXEMAN", from, 4);
    let a = w.army(p0, &axes);
    w.g.armies[a].tactic = volley;
    w.g.armies[a].tactic_next = charge;
    // A new army formed from its units takes its tactics (a pending change too).
    let b = w.g.army_split(p0, &axes[..3]).unwrap();
    assert_eq!((w.g.armies[b].tactic, w.g.armies[b].tactic_next), (volley, charge));
    // A merged army keeps the receiving army's.
    let lone = w.units(p0, "UNIT_WARRIOR", from, 1);
    let c = w.army(p0, &lone);
    w.g.army_merge(p0, b, c).unwrap();
    assert_eq!(w.g.armies[c].tactic, line);
    // An army formed from a garrison takes the garrison's tactic.
    let (x, y) = w.g.map.xy(to);
    let city = w.g.player_init_city(p1, x, y, true, true);
    w.g.cities[city].garrison_tactic = charge;
    w.g.cities[city].garrison_tactic_next = charge;
    let garrison = w.units(p1, "UNIT_AXEMAN", to, 2);
    w.g.player_army_sync(p1);
    let g2 = w.army(p1, &garrison);
    assert_eq!(w.g.armies[g2].tactic, charge);
    // A city that changes hands (here by trade) goes back to Line.
    for u in w.g.map.plots[to].units.clone() {
        w.g.unit_kill(u, false, None);
    }
    w.g.player_acquire_city(p0, city, false, true, true);
    let taken = w.g.map.plots[to].plot_city.expect("the city");
    assert_eq!(w.g.cities[taken].owner, p0);
    assert_eq!((w.g.cities[taken].garrison_tactic, w.g.cities[taken].garrison_tactic_next), (line, line));
}

#[test]
fn save_and_load_keep_the_tactics() {
    let mut w = world();
    let (p0, from, to) = (w.p0, w.from, w.to);
    let charge = w.tactic("TACTIC_CHARGE");
    let vanguard = w.tactic("TACTIC_VANGUARD");
    let axes = w.units(p0, "UNIT_AXEMAN", from, 2);
    let a = w.army(p0, &axes);
    w.g.armies[a].tactic = charge;
    w.g.armies[a].tactic_next = vanguard;
    let (x, y) = w.g.map.xy(to);
    let c = w.g.player_init_city(p0, x, y, true, true);
    w.g.cities[c].garrison_tactic = vanguard;
    w.g.cities[c].garrison_tactic_next = charge;
    let g = Game::load(w.g.d, &w.g.save()).expect("a save of this build");
    assert_eq!((g.armies[a].tactic, g.armies[a].tactic_next), (charge, vanguard));
    assert_eq!((g.cities[c].garrison_tactic, g.cities[c].garrison_tactic_next), (vanguard, charge));
}
