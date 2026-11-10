//! Engagement combat's state: the dispatch rule, the new
//! unit fields, the battle reports and what is kept of them, and saves.

use ffh_core::engagement::report::*;
use ffh_core::engagement::formation::{Event, Hit};
use ffh_core::engagement::{EngageResult, Ground, GroundRule, Side};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map after `game_init` with armies: player 0's city on its first unit's plot, the starting
/// units gone. Both players are the Bannor (no race promotion: a unit's numbers are its sheet's).
fn game() -> (Game, CityId) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    for p in 0..2 {
        setup.players[p].civ = Some(d.t::<CivilizationType>("CIVILIZATION_BANNOR"));
        setup.players[p].leader = Some(d.t::<LeaderHeadType>("LEADER_CAPRIA")); // no combat trait
    }
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

/// Land attacks on land are engagements (the game's land combat, with or without armies); attacks
/// on water and by ships are duels.
#[test]
fn land_attacks_on_land_are_engagements() {
    let (mut g, c) = game();
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
}

#[test]
fn promotions_keep_the_defence_percentage() {
    let (mut g, c) = game();
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let u = warrior(&mut g, x, y);
    let (faith, combat1) = (g.d.t::<PromotionType>("PROMOTION_SHIELD_OF_FAITH"), g.d.t::<PromotionType>("PROMOTION_COMBAT1"));
    g.unit_set_has_promotion(u, faith, true);
    assert_eq!((g.units[u].extra_combat_percent, g.units[u].stat_pct[StatType::Defense.idx()]), (10, 30));
    g.unit_set_has_promotion(u, combat1, true);
    assert_eq!((g.units[u].extra_combat_percent, g.units[u].stat_pct[StatType::Defense.idx()]), (30, 50));
    g.unit_set_has_promotion(u, faith, false);
    assert_eq!((g.units[u].extra_combat_percent, g.units[u].stat_pct[StatType::Defense.idx()]), (20, 20));
    // Homeland adds to both where the unit fights: tests/engagement_stats.rs,
    // `homeland_counts_where_the_unit_fights`.
}

#[test]
fn attacks_made_resets_with_made_attack() {
    let (mut g, c) = game();
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

#[test]
fn a_bombard_is_an_attack_made() {
    let (mut g, c) = game();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let walls = g.d.t::<BuildingType>("BUILDING_WALLS");
    g.city_set_num_real_building(c, walls, 1);
    let (x, y) = next_to(&g, c, true).expect("a land plot next to the city");
    let cat = g.player_init_unit(p1, g.d.t::<UnitType>("UNIT_CATAPULT"), x, y, None, None);
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_declare_war(t1, t0, false, None);
    assert!(g.unit_bombard(cat), "the catapult bombards the city");
    assert!(g.cities[c].sentinel_damage.iter().all(|&d| d > 0), "it hits the walls' Sentinels (docs/SIEGES.md)");
    assert!(g.units[cat].made_attack);
    assert_eq!(g.units[cat].attacks_made, 1);
}

/// A report of `p1` attacking `p2` at the city's plot, with a small log when `logged`.
fn report(g: &Game, c: CityId, logged: bool) -> BattleReport {
    let plot = g.city_plot(c);
    let side = |p: u8| ReportSide {
        player: PlayerId(p),
        shown_player: PlayerId(p),
        players: vec![PlayerId(p)],
        slots: 10,
        lanes: 4,
        used: 1,
        committed: 1,
        sat_out: 0,
        fallen: 0,
        escaped: 0,
        standing: 1,
        lost: Vec::new(),
        summons: 0,
    };
    let stat = |x| Stat { base_x100: x, terms: vec![Term { src: TermSource::Fortify, pct: 25 }], total_x100: x * 125 / 100 };
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
    let strike = |count| RowActions { kind: ActionKind::Strike, count };
    let fighter = |side: Side, unit: UnitId, fate: LogFate| LogFighter {
        unit,
        sentinel: false,
        unit_type: warrior,
        name: String::new(),
        owner: PlayerId(side.index() as u8 + 1),
        shown_owner: PlayerId(side.index() as u8 + 1),
        side,
        place: Some((1, 0)),
        large: false,
        level: 1,
        xp: 0,
        hero: false,
        max_hp: 120,
        hp_start: 12000,
        hp_battle: 12000,
        speed: 4,
        first_strikes: 0,
        chance_first_strikes: 0,
        rolled_first_strikes: 0,
        actions: [strike(2), strike(1), strike(1)],
        spell: None,
        accuracy: 90,
        evasion: 0,
        crit: 10,
        anti_flank: false,
        flanker: false,
        withdraw_pct: 0,
        combat_limit: 100,
        collateral_limit: None,
        def_strike: None,
        immune: Immunities::default(),
        fear: false,
        a: stat(300),
        dv: stat(300),
        mag: stat(0),
        mdef: stat(300),
        fate: Some(fate),
        dealt: 0,
        taken: 0,
        healed: 0,
        summoner: None,
        on_kill: false,
    };
    let hit = Hit { target: 1, dmg: 1800, expected: 1777, r: 101, crit: false, miss: false, killed: true, withdrew: false, splash: false };
    let log = BattleLog {
        slots: [0, 1].map(|_| SideSlots { base: 8, changes: vec![SlotChange { src: SlotSource::Hills, delta: -2 }], total: 6, river_only: 0 }),
        lanes: [3, 3],
        ground: Ground { city: false, forest: false, hills: true },
        rules: vec![GroundRule::HillsAttacker { pct: 75 }, GroundRule::HillsDefenderRanged { pct: 125 }, GroundRule::Charge { pct: 200 }],
        ordered: [false, false],
        tactics: [ffh_data::BattleTacticType(0); 2],
        fighters: vec![fighter(Side::Attacker, UnitId(1000), LogFate::Standing), fighter(Side::Defender, UnitId(1001), LogFate::Fell { by: 0, round: 2 })],
        pairs: vec![LogPair { h: 0, t: 1, a_x100: 300, dv_x100: 375, mean: 1777, ma_x100: 300, md_x100: 375, spell_pct: 100, magic_mean: 1777, a_extra: vec![], dv_extra: vec![] }],
        events: vec![
            LogEvent::Stage { stage: Stage::Battle },
            LogEvent::Action(Event { round: 2, pass: 1, actor: 0, kind: ActionKind::Strike, spell: None, target: Some(1), hits: vec![hit], heal: None, knocked: None, note: None, entered: vec![] }),
            LogEvent::End { rounds: 2, down: [false, true] },
        ],
        forecast: Some(FrozenForecast {
            runs: 200,
            wiped_permille: 640,
            falls_permille: 600,
            repulsed_permille: 10,
            lost_x100: [30, 64],
            per: vec![ForecastFighter { b: 0, fall_pct: 30, escape_pct: 0 }],
        }),
    };
    BattleReport {
        id: 0,
        turn: 0,
        plot,
        from: plot,
        place: PlotSnap { x: g.cities[c].x, y: g.cities[c].y, terrain: g.map.plots[plot].terrain_type, feature: None, hills: false, river_crossing: false, city: None },
        sides: [side(1), side(2)],
        outcome: Outcome { result: EngageResult::Taken, rounds: 2, plot_taken: false, city_captured: None, moved_in: 0, defenders_left: 0, sentinels: 0, sentinels_breached: 0 },
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
    let (mut g, c) = game();
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
    // A crowded turn: more than ENGAGEMENT_REPORTS_KEPT battles (other players' wars count too)
    // push out the older reports only; every report of this turn and the previous one stays, with
    // its log.
    let prev: Vec<u32> = g.battle_reports.iter().filter(|r| r.turn == recent).map(|r| r.id).collect();
    assert!(!prev.is_empty());
    assert!(g.battle_reports.iter().any(|r| r.turn < recent));
    for _ in 0..kept {
        g.battle_report_add(report(&g, c, false));
    }
    assert!(g.battle_reports.len() > kept);
    assert!(g.battle_reports.iter().all(|r| r.turn >= recent), "the older reports went first");
    assert!(prev.iter().all(|&id| g.battle_report(id).is_some_and(|r| r.log.is_some())));
    // Turns on, what is now older is held to the cap again, the oldest dropped; ids are never
    // reused.
    next_turn(&mut g);
    assert!(prev.iter().all(|&id| g.battle_report(id).is_none()));
    assert!(g.battle_reports.len() > kept, "the previous turn's crowd stays one more turn");
    next_turn(&mut g);
    assert_eq!(g.battle_reports.len(), kept);
    let next = g.battle_report_add(report(&g, c, false));
    assert_eq!(g.battle_reports.len(), kept);
    assert!(g.battle_report(0).is_none());
    assert_eq!(g.battle_report(next).map(|r| r.id), Some(next));
    assert_eq!(g.battle_report_next, next + 1);
}

#[test]
fn reports_and_unit_fields_survive_a_save() {
    let (mut g, c) = game();
    let p0 = PlayerId(0);
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let units: Vec<UnitId> = (0..3).map(|_| warrior(&mut g, x, y)).collect();
    g.army_form(p0, &units).unwrap();
    let u = units[0];
    g.unit_set_has_promotion(u, g.d.t::<PromotionType>("PROMOTION_SHIELD_OF_FAITH"), true);
    g.units[u].attacks_made = 1;
    g.units[u].battle_heal_turn = 7;
    g.battle_report_add(report(&g, c, true));
    g.battle_report_add(report(&g, c, false));
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).unwrap();
    assert_eq!((h.units[u].stat_pct[StatType::Defense.idx()], h.units[u].attacks_made, h.units[u].battle_heal_turn), (30, 1, 7));
    assert_eq!(h.battle_reports, g.battle_reports);
    assert!(h.battle_reports[0].log.is_some() && h.battle_reports[1].log.is_none());
    assert_eq!(h.battle_report_next, 2);
    assert_eq!(h.save(), bytes);
    assert_eq!(h.calculate_sync_checksum(), g.calculate_sync_checksum());
}
