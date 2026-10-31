//! Engagement combat's state (GAMEOPTION_ENGAGEMENTS, phase 6 step 2): the option and the dispatch
//! rule, stances and their commands with the split and merge rules, the new unit fields, the
//! battle reports and what is kept of them, and saves.

use ffh_core::engagement::report::*;
use ffh_core::engagement::{EngageResult, Fate, Phase, Side};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map after `game_init` with armies (and engagements when `engagements`): player 0's city
/// on its first unit's plot, the starting units gone.
fn game(engagements: bool) -> (Game, CityId) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
    setup.options[GameOptionType::GAMEOPTION_ENGAGEMENTS] = engagements;
    let mut g = Game::new(d, setup);
    g.game_init();
    let p0 = PlayerId(0);
    let u = g.players[p0].units[0];
    let (x, y) = (g.units[u].x, g.units[u].y);
    for u in g.players[p0].units.clone() {
        g.unit_kill(u, false, None);
    }
    let c = g.player_init_city(p0, x, y, true, true);
    (g, c)
}

fn warrior(g: &mut Game, x: i32, y: i32) -> UnitId {
    let w = g.d.t::<UnitType>("UNIT_WARRIOR");
    g.player_init_unit(PlayerId(0), w, x, y, None, None)
}

fn stance(g: &Game, name: &str) -> StanceType {
    g.d.t::<StanceType>(name)
}

/// A plot next to city `c` that is land (`land`) or water, outside the city.
fn next_to(g: &Game, c: CityId, land: bool) -> Option<(i32, i32)> {
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    for r in 1..=4 {
        for dy in -r..=r {
            for dx in -r..=r {
                let Some(q) = g.map.plot_xy(x + dx, y + dy) else { continue };
                if g.plot_distance(x, y, x + dx, y + dy) == r && g.plot_is_water(q) != land && !g.plot_is_impassable(q) && g.map.plots[q].plot_city.is_none() {
                    return Some(g.map.xy(q));
                }
            }
        }
    }
    None
}

#[test]
fn the_option_requires_armies_and_applies_to_land_attacks_on_land() {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_ENGAGEMENTS] = true;
    let g = Game::new(d, setup);
    assert!(!g.engagements_on(), "engagements need armies");

    let (mut g, c) = game(true);
    assert!(g.engagements_on());
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let u = warrior(&mut g, x, y);
    let (lx, ly) = next_to(&g, c, true).expect("land");
    let land = g.map.plot_xy(lx, ly).unwrap();
    assert!(g.engage_applies(u, land));
    let grp = g.units[u].group.unwrap();
    assert!(g.group_engage_applies(grp, land));
    if let Some((wx, wy)) = next_to(&g, c, false) {
        let water = g.map.plot_xy(wx, wy).unwrap();
        assert!(!g.engage_applies(u, water), "an attack on water is a duel");
        // A ship attacking land is not an engagement.
        let ship = g.d.units.keys().find(|&t| g.d.units[t].domain_type == Some(DomainType::DOMAIN_SEA) && g.d.units[t].combat > 0).unwrap();
        let s = g.player_init_unit(PlayerId(0), ship, wx, wy, None, None);
        assert!(!g.engage_applies(s, land));
    }

    let (mut off, c) = game(false);
    let (x, y) = (off.cities[c].x, off.cities[c].y);
    let u = warrior(&mut off, x, y);
    assert!(!off.engagements_on());
    assert!(!off.engage_applies(u, land));
}

#[test]
fn stance_commands_are_refused_with_the_option_off() {
    let (mut g, c) = game(false);
    let p0 = PlayerId(0);
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let units: Vec<UnitId> = (0..2).map(|_| warrior(&mut g, x, y)).collect();
    let a = g.army_form(p0, &units).unwrap();
    let mobile = stance(&g, "STANCE_MOBILE");
    assert!(g.army_set_stance(p0, a, mobile).is_err());
    assert!(g.city_set_garrison_stance(p0, c, mobile).is_err());
    assert_eq!(g.armies[a].stance, g.d.default_stance);
    assert_eq!(g.cities[c].garrison_stance, g.d.default_stance);
}

#[test]
fn new_armies_take_the_stance_they_came_from_and_a_merge_keeps_the_receivers() {
    let (mut g, c) = game(true);
    let p0 = PlayerId(0);
    let d = g.d;
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let balanced = stance(&g, "STANCE_BALANCED");
    let (wall, mobile, ranged) = (stance(&g, "STANCE_SHIELD_WALL"), stance(&g, "STANCE_MOBILE"), stance(&g, "STANCE_RANGED_LINE"));
    assert_eq!(d.default_stance, balanced);
    assert_eq!(g.cities[c].garrison_stance, balanced);

    // Commands: an army or city of another player, a stance that does not exist.
    assert!(g.city_set_garrison_stance(PlayerId(1), c, wall).is_err());
    assert!(g.city_set_garrison_stance(p0, c, StanceType(d.stances.len() as u16)).is_err());
    g.city_set_garrison_stance(p0, c, wall).unwrap();

    // An army formed from the garrison takes the garrison's stance.
    let units: Vec<UnitId> = (0..4).map(|_| warrior(&mut g, x, y)).collect();
    g.player_army_sync(p0);
    assert!(units.iter().all(|&u| g.unit_garrison_city(u) == Some(c)));
    let a = g.army_form(p0, &units).unwrap();
    assert_eq!(g.armies[a].stance, wall);
    assert!(g.army_set_stance(PlayerId(1), a, mobile).is_err());
    g.army_set_stance(p0, a, mobile).unwrap();

    // A split takes its army's stance.
    let b = g.army_split(p0, &units[..1]).unwrap();
    assert_ne!(a, b);
    assert_eq!(g.armies[b].stance, mobile);
    g.army_set_stance(p0, b, ranged).unwrap();

    // A merge keeps the receiving army's.
    g.army_merge(p0, b, a).unwrap();
    assert!(!g.armies.contains(b));
    assert_eq!(g.armies[a].stance, mobile);

    // A split made by the groups (Civ4's split) takes its army's stance too.
    g.unit_join_group(units[1], None, true);
    g.player_army_sync(p0);
    let c2 = g.units[units[1]].army.unwrap();
    assert_ne!(c2, a);
    assert_eq!(g.armies[c2].stance, mobile);

    // Garrison units that march out of the city: the garrison's stance.
    let (ox, oy) = next_to(&g, c, true).expect("land");
    let out: Vec<UnitId> = (0..2).map(|_| warrior(&mut g, x, y)).collect();
    g.player_army_sync(p0);
    assert!(out.iter().all(|&u| g.unit_garrison_city(u) == Some(c)));
    g.unit_join_group(out[0], None, true);
    let grp = g.units[out[0]].group;
    g.unit_join_group(out[1], grp, false);
    for &u in &out {
        g.unit_set_xy(u, ox, oy, true);
    }
    g.player_army_sync(p0);
    let e = g.units[out[0]].army.expect("an army outside the city");
    assert_eq!(g.units[out[1]].army, Some(e));
    assert_eq!(g.armies[e].stance, wall);
}

#[test]
fn promotions_keep_the_defence_percentage() {
    let (mut g, c) = game(true);
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let u = warrior(&mut g, x, y);
    let (faith, combat1) = (g.d.t::<PromotionType>("PROMOTION_SHIELD_OF_FAITH"), g.d.t::<PromotionType>("PROMOTION_COMBAT1"));
    g.unit_set_has_promotion(u, faith, true);
    assert_eq!((g.units[u].extra_combat_percent, g.units[u].extra_combat_percent_defense), (10, 30));
    g.unit_set_has_promotion(u, combat1, true);
    assert_eq!((g.units[u].extra_combat_percent, g.units[u].extra_combat_percent_defense), (30, 50));
    g.unit_set_has_promotion(u, faith, false);
    assert_eq!((g.units[u].extra_combat_percent, g.units[u].extra_combat_percent_defense), (20, 20));
    // In its owner's borders the Homeland bonus adds to both.
    g.units[u].combat_percent_in_borders = 25;
    assert_eq!((g.unit_get_extra_combat_percent(u), g.unit_get_extra_combat_percent_defense(u)), (45, 45));
}

#[test]
fn attacks_made_resets_with_made_attack() {
    let (mut g, c) = game(true);
    let p0 = PlayerId(0);
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let u = warrior(&mut g, x, y);
    assert_eq!((g.units[u].attacks_made, g.units[u].battle_heal_turn), (0, -1));
    g.units[u].made_attack = true;
    g.units[u].attacks_made = 2;
    g.units[u].battle_heal_turn = 3;
    g.player_do_turn_units(p0);
    assert!(!g.units[u].made_attack);
    assert_eq!(g.units[u].attacks_made, 0);
    assert_eq!(g.units[u].battle_heal_turn, 3, "the heal turn is a turn number, not a counter");
}

/// A report of `p1` attacking `p2` at the city's plot, with a small log when `logged`.
fn report(g: &Game, c: CityId, logged: bool) -> BattleReport {
    let plot = g.city_plot(c);
    let side = |p: u8, stance: StanceType| ReportSide {
        player: PlayerId(p),
        shown_player: PlayerId(p),
        players: vec![PlayerId(p)],
        army: None,
        garrison_of: None,
        stance,
        committed: 1,
        withdrew: 0,
        sat_out: 0,
        fled: 0,
        lost: Vec::new(),
    };
    let stat = |x| Stat { base_x100: x, role_pct: 100, terms: vec![Term { src: TermSource::Fortify, pct: 25 }], total_x100: x * 125 / 100 };
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
    let fighter = |side: Side, unit: UnitId, fate: Fate| LogFighter {
        unit,
        unit_type: warrior,
        name: String::new(),
        owner: PlayerId(side.index() as u8 + 1),
        shown_owner: PlayerId(side.index() as u8 + 1),
        side,
        slot: Some(0),
        level: 1,
        xp: 0,
        hero: false,
        role: CombatRoleType(0),
        pos: Position::Front,
        screen: false,
        ignores_shield: false,
        assassin_profile: false,
        guardsman: false,
        max_hp: 120,
        hp_start: 12000,
        volleys: 0,
        volley_chance: 0,
        withdraw_pct: 0,
        def_strike: None,
        collateral: None,
        combat_limit: 100,
        immune: Immunities::default(),
        fear: false,
        a: stat(300),
        dv: stat(300),
        fate: Some(fate),
    };
    let log = BattleLog {
        slots: [0, 1].map(|_| SideSlots { base: 8, changes: vec![SlotChange { src: SlotSource::Hills, delta: -2 }], total: 6, river_only: 0 }),
        picks: [0, 1].map(|_| SidePick { auto_pick: vec![0], picked_by: Pick::Auto }),
        fighters: vec![fighter(Side::Attacker, UnitId(1000), Fate::Survived), fighter(Side::Defender, UnitId(1001), Fate::Killed { by: Some(0), phase: Phase::Round(2) })],
        pairs: vec![LogPair { h: 0, t: 1, a_x100: 300, dv_x100: 375, mean: 888, a_extra: vec![], dv_extra: vec![] }],
        events: vec![
            LogEvent::Stage { stage: Stage::Round(1) },
            LogEvent::Hit { h: 0, t: 1, rule: TargetRule::FrontFirst, step: 0, blocked: vec![], eligible: vec![1], r: 100, dmg: 888, capped: false },
            LogEvent::RoundEnd { deaths: vec![LogDeath { b: 1, killer: Some(0), assists: vec![], overkill: 12 }] },
            LogEvent::End { result: EngageResult::AttackerWon, volley_rounds: 0, rounds: 2 },
        ],
        forecast: Some(FrozenForecast {
            source: ForecastSource::Expected,
            win_permille: 640,
            draw_permille: 0,
            rounds: (1, 2, 4),
            losses_if_win_x10: 3,
            per: vec![ForecastFighter { b: 0, fall_pct: 30, withdraw_pct: 0, death_round: None }],
            curve: [vec![12000, 9000], vec![12000, 0]],
            alternatives: vec![(AltKind::ThisPlan, 640)],
        }),
    };
    BattleReport {
        id: 0,
        turn: 0,
        plot,
        from: plot,
        place: PlotSnap { x: g.cities[c].x, y: g.cities[c].y, terrain: g.map.plots[plot].terrain_type, feature: None, hills: false, river_crossing: false, city: None },
        sides: [side(1, g.d.default_stance), side(2, g.d.default_stance)],
        outcome: Outcome { result: EngageResult::AttackerWon, volley_rounds: 0, rounds: 2, plot_taken: false, city_captured: None, moved_in: 0, defenders_left: 0 },
        log: logged.then(|| Box::new(log)),
    }
}

/// The next game turn, as `do_turn` starts it (the reports are trimmed there).
fn next_turn(g: &mut Game) {
    g.increment_game_turn();
    g.battle_reports_trim();
}

#[test]
fn reports_keep_logs_for_two_turns_and_the_latest_battles() {
    let (mut g, c) = game(true);
    let logs_kept = g.d.defines.engagement_report_logs_kept as usize;
    let kept = g.d.defines.engagement_reports_kept as usize;
    let t0 = g.game_turn();
    let first: Vec<u32> = (0..5).map(|_| g.battle_report_add(report(&g, c, true))).collect();
    assert_eq!(first, vec![0, 1, 2, 3, 4]);
    assert_eq!(g.battle_report(2).map(|r| r.turn), Some(t0));
    assert_eq!(g.battle_reports_of(PlayerId(1)).count(), 5);
    assert_eq!(g.battle_reports_of(PlayerId(3)).count(), 0);
    // Two turns on, the five are still among the latest: kept.
    next_turn(&mut g);
    next_turn(&mut g);
    assert!(g.battle_reports.iter().all(|r| r.log.is_some()));
    // The latest battles push the older logs out; the summaries stay.
    for _ in 0..logs_kept - 2 {
        g.battle_report_add(report(&g, c, true));
    }
    let logged: Vec<bool> = g.battle_reports.iter().take(5).map(|r| r.log.is_some()).collect();
    assert_eq!(logged, vec![false, false, false, true, true]);
    assert_eq!(g.battle_reports.len(), logs_kept + 3);
    // A turn's battles keep their logs through the next turn, however many there are.
    next_turn(&mut g);
    for _ in 0..logs_kept + 5 {
        g.battle_report_add(report(&g, c, true));
    }
    let recent = g.game_turn() - 1;
    assert!(g.battle_reports.iter().filter(|r| r.turn >= recent).all(|r| r.log.is_some()));
    assert!(g.battle_reports.iter().filter(|r| r.turn < recent).all(|r| r.log.is_none()));
    // Unlogged reports (battles nobody saw) are summaries from the start.
    let id = g.battle_report_add(report(&g, c, false));
    assert!(g.battle_report(id).unwrap().log.is_none());
    // At most ENGAGEMENT_REPORTS_KEPT, the oldest dropped; ids are never reused.
    while g.battle_reports.len() < kept {
        g.battle_report_add(report(&g, c, false));
    }
    let next = g.battle_report_add(report(&g, c, false));
    assert_eq!(g.battle_reports.len(), kept);
    assert!(g.battle_report(0).is_none());
    assert_eq!(g.battle_report(next).map(|r| r.id), Some(next));
    assert_eq!(g.battle_report_next, next + 1);
}

#[test]
fn stances_reports_and_unit_fields_survive_a_save() {
    let (mut g, c) = game(true);
    let p0 = PlayerId(0);
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let units: Vec<UnitId> = (0..3).map(|_| warrior(&mut g, x, y)).collect();
    let a = g.army_form(p0, &units).unwrap();
    let (wall, mobile) = (stance(&g, "STANCE_SHIELD_WALL"), stance(&g, "STANCE_MOBILE"));
    g.army_set_stance(p0, a, mobile).unwrap();
    g.city_set_garrison_stance(p0, c, wall).unwrap();
    let u = units[0];
    g.unit_set_has_promotion(u, g.d.t::<PromotionType>("PROMOTION_SHIELD_OF_FAITH"), true);
    g.units[u].attacks_made = 1;
    g.units[u].battle_heal_turn = 7;
    g.battle_report_add(report(&g, c, true));
    g.battle_report_add(report(&g, c, false));
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).unwrap();
    assert_eq!(h.armies[a].stance, mobile);
    assert_eq!(h.cities[c].garrison_stance, wall);
    assert_eq!((h.units[u].extra_combat_percent_defense, h.units[u].attacks_made, h.units[u].battle_heal_turn), (30, 1, 7));
    assert_eq!(h.battle_reports, g.battle_reports);
    assert!(h.battle_reports[0].log.is_some() && h.battle_reports[1].log.is_none());
    assert_eq!(h.battle_report_next, 2);
    assert_eq!(h.save(), bytes);
    assert_eq!(h.calculate_sync_checksum(), g.calculate_sync_checksum());
}
