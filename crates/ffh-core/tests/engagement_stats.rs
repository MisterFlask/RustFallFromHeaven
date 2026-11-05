//! Engagement combat's unit stats (phase 6 step 4; `engagement/stats.rs`):
//! roles and HP, and each term of A and Dv per hit on the side the plan's table puts it (4.2: G1,
//! Q1, Q2, Q9), against numbers worked out by hand from the unit data, on concrete units and plots.

use ffh_core::engagement::report::TermSource;
use ffh_core::engagement::{HitCtx, Pair, Side};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map with players 0 and 1 at war (both AI), `from` and `to` two unowned flat land plots
/// side by side with no feature, improvement or river crossing between them and no plot defense.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    from: PlotIdx,
    to: PlotIdx,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
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
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_declare_war(t0, t1, false, None);
    World { g, p0, p1, from, to }
}

impl World {
    fn unit(&mut self, owner: PlayerId, name: &str, at: PlotIdx) -> UnitId {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    fn promote(&mut self, u: UnitId, name: &str) {
        let p = self.g.d.t::<PromotionType>(name);
        self.g.unit_set_has_promotion(u, p, true);
    }

    fn ctx(&self) -> HitCtx {
        HitCtx::new(self.from, self.to)
    }

    /// The pair `hitter` (an attacker) -> `target` (a defender), and back.
    fn both_ways(&self, attacker: UnitId, defender: UnitId) -> (Pair, Pair) {
        let ctx = self.ctx();
        (self.g.engage_hit(attacker, defender, Side::Attacker, &ctx), self.g.engage_hit(defender, attacker, Side::Defender, &ctx))
    }
}

/// `apply_combat_modifier` (positive multiplies, negative divides), at least 1.
fn apply(base_x100: i32, modifier: i32) -> i32 {
    1.max(if modifier > 0 { base_x100 * (modifier + 100) / 100 } else { base_x100 * 100 / (100 - modifier) })
}


/// A pair's A and Dv (its magic is checked apart).
fn ad(p: Pair) -> (i32, i32) {
    (p.a, p.dv)
}

fn ad2((p, q): (Pair, Pair)) -> ((i32, i32), (i32, i32)) {
    (ad(p), ad(q))
}

#[test]
fn roles_and_hit_points() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let role = |g: &Game, u: UnitId| g.unit_combat_role(u).map(|r| g.d.combat_roles[r].base.type_.clone());
    for (name, expect, hp) in [
        ("UNIT_AXEMAN", "COMBATROLE_FRONTLINE", 120),
        ("UNIT_ARCHER", "COMBATROLE_ARCHER", 80),
        ("UNIT_KNIGHT", "COMBATROLE_FLANKER", 100),
        ("UNIT_ASSASSIN", "COMBATROLE_ASSASSIN", 60),
        ("UNIT_ADEPT", "COMBATROLE_ARCANE", 70),
        ("UNIT_FIRE_ELEMENTAL", "COMBATROLE_FRONTLINE", 120),
    ] {
        let u = w.unit(p0, name, from);
        assert_eq!(role(&w.g, u).as_deref(), Some(expect), "{name}");
        assert_eq!(w.g.unit_engage_max_hp(u), hp, "{name}");
    }
    // Formation makes a frontline unit a screen (Q7); an archer keeps its role.
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    let archer = w.unit(p0, "UNIT_ARCHER", from);
    w.promote(axeman, "PROMOTION_FORMATION");
    w.promote(archer, "PROMOTION_FORMATION");
    assert_eq!(role(&w.g, axeman).as_deref(), Some("COMBATROLE_SCREEN"));
    assert_eq!(w.g.unit_engage_max_hp(axeman), 120);
    assert_eq!(role(&w.g, archer).as_deref(), Some("COMBATROLE_ARCHER"));
}

/// Formation battles' per-unit values (`Game::unit_engage_*`): with no unit type setting its own
/// (the game's data; crates/ffh-data/tests/engagement_overrides.rs loads a fixture that does),
/// each is the unit's role's, by the unit's own role, with a tier 4 unit's or a hero's extra
/// action in its best row.
#[test]
fn formation_values_by_unit() {
    use ActionKind::*;
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let r = |kind, count| RowActions { kind, count };
    let values = |g: &Game, u: UnitId| {
        (g.unit_engage_speed(u), g.unit_engage_evasion(u), g.unit_engage_crit(u), g.unit_engage_best_row(u), g.unit_engage_flanks(u))
    };
    // A tier 4 flanker: the flanker's values, one more charge from the front.
    let knight = w.unit(p0, "UNIT_KNIGHT", from);
    assert_eq!(values(&w.g, knight), (6, 10, 10, 0, true));
    assert_eq!(w.g.unit_engage_rows(knight), [r(Charge, 2), r(Strike, 1), r(Strike, 1)]);
    assert_eq!(w.g.unit_engage_actions(knight), [r(Charge, 3), r(Strike, 1), r(Strike, 1)]);
    // An assassin (tier 3): its sheet's evasion and crit, best from the middle, no extra action.
    let assassin = w.unit(p0, "UNIT_ASSASSIN", from);
    assert_eq!(values(&w.g, assassin), (7, 15, 25, 1, false));
    assert_eq!(w.g.unit_engage_actions(assassin), [r(Stab, 1), r(Stab, 2), r(Stab, 1)]);
    // An axeman: the frontline's; with Formation the screen's (its own role); a hero acts once
    // more from its best row.
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    assert_eq!(values(&w.g, axeman), (4, 0, 10, 0, false));
    assert_eq!(w.g.unit_engage_actions(axeman), [r(Strike, 2), r(Strike, 1), r(Strike, 1)]);
    w.promote(axeman, "PROMOTION_FORMATION");
    assert_eq!(w.g.unit_engage_speed(axeman), 3);
    w.promote(axeman, "PROMOTION_HERO");
    assert_eq!(w.g.unit_engage_actions(axeman), [r(Strike, 3), r(Strike, 1), r(Strike, 1)]);
    // A siege engine's front row stays without an action.
    let catapult = w.unit(p0, "UNIT_CATAPULT", from);
    assert_eq!(values(&w.g, catapult), (1, 0, 10, 2, false));
    assert_eq!(w.g.unit_engage_actions(catapult), [r(None, 0), r(Bombard, 1), r(Bombard, 2)]);
}

#[test]
fn archer_in_a_walled_hill_city_fortified() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let d = w.g.d;
    w.g.map.plots[to].plot_type = PlotType::PLOT_HILLS;
    w.g.map.plots[from].plot_type = PlotType::PLOT_HILLS;
    let (tx, ty) = w.g.map.xy(to);
    let city = w.g.player_init_city(p1, tx, ty, true, true);
    w.g.city_set_num_real_building(city, d.t::<BuildingType>("BUILDING_WALLS"), 1);
    let archer = w.unit(p1, "UNIT_ARCHER", to);
    w.g.units[archer].fortify_turns = 5;
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    w.g.units[axeman].fortify_turns = 5;

    let team1 = w.g.players[p1].team;
    let plot_defense = w.g.plot_defense_modifier(to, Some(team1), false);
    let city_defense = w.g.city_defense_modifier(city, false);
    assert!(city_defense > 0, "walls");
    assert_eq!(plot_defense, d.defines.hills_extra_defense + city_defense);
    let (attack, back) = w.both_ways(axeman, archer);

    // The archer's Dv: 5 x 60% (Archer) under its own terms only (revision 4: the plot's own
    // defense, the hills and the city's walls, gives Dv nothing): fortification (5 turns x 5), its
    // own city defense (+25) and hills defense (+25).
    assert_eq!(attack.dv, apply(5 * 60, 25 + 25 + 25));
    // The axeman's A: 4 x 100% and its city attack (+10).
    assert_eq!(attack.a, apply(400, 10));
    // Hitting back (G1): the archer has no city or terrain attack; the axeman, the attacker, has
    // no plot defense, fortification or hills: its Dv is its bare 4.
    assert_eq!(ad(back), (300, 400));

    // The duel puts all of it on the archer's one strength.
    assert_eq!(w.g.unit_max_combat_str(archer, Some(to), Some(axeman), None), apply(500, plot_defense + 25 + 25 + 25 - 10));

    // The breakdown lists the parts and they sum to the modifier.
    let stat = w.g.engage_stat(archer, Some(axeman), Side::Defender, false, &w.ctx());
    assert_eq!(stat.total_x100, attack.dv);
    assert_eq!(stat.base_x100, 300); // its sheet's defence, 30
    let srcs: Vec<&TermSource> = stat.terms.iter().map(|t| &t.src).collect();
    for src in [TermSource::Fortify, TermSource::UnitCityDefense, TermSource::UnitHillsDefense] {
        assert!(srcs.contains(&&src), "{src:?} in {srcs:?}");
    }
    for src in [TermSource::Hills, TermSource::CityDefense] {
        assert!(!srcs.contains(&&src), "{src:?} in {srcs:?}: the plot's own defense");
    }
    assert_eq!(stat.terms.iter().map(|t| t.pct).sum::<i32>(), 75);

    // Against nobody in particular: the archer's Dv against a hitter not known yet is the same
    // here; the axeman's A against the city plot keeps its city attack.
    assert_eq!(w.g.engage_job_defense(archer, &w.ctx()), attack.dv);
    assert_eq!(w.g.engage_a_alone(axeman, Side::Attacker, &w.ctx()), apply(400, 10));

    // The walls' Sentinels breached (docs/SIEGES.md) change the slots, not Dv.
    let walled = w.g.engage_slots(&w.ctx()).att();
    for k in 0..w.g.city_sentinel_count(city) {
        w.g.city_set_sentinel_damage(city, k, 100);
    }
    assert_eq!(w.g.engage_hit(axeman, archer, Side::Attacker, &w.ctx()).dv, attack.dv);
    assert!(w.g.engage_slots(&w.ctx()).att() > walled);
}

/// A city's defense (its walls, its culture) gives the defenders' Dv nothing (revision 4): a unit
/// that ignores building defense, the defense half or wholly knocked down: the same Dv. It costs
/// no slots either (docs/SIEGES.md): its walls' two Sentinels cost each side one slot each, the
/// defence knocked down or not.
#[test]
fn a_city_s_defense_costs_slots_and_gives_dv_nothing() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let d = w.g.d;
    let (tx, ty) = w.g.map.xy(to);
    let city = w.g.player_init_city(p1, tx, ty, true, true);
    w.g.city_set_num_real_building(city, d.t::<BuildingType>("BUILDING_WALLS"), 1);
    w.g.players[p1].city_defense_modifier = 20;
    let defender = w.unit(p1, "UNIT_AXEMAN", to);
    let cannon = w.unit(p0, "UNIT_CANNON", from);
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    assert!(w.g.unit_ignore_building_defense(cannon));
    let full = w.g.city_defense_modifier(city, false);
    assert!(full >= 25, "{full}");
    let max_damage = d.defines.max_city_defense_damage;
    let mut slots = Vec::new();
    for defense_damage in [0, max_damage / 2, max_damage] {
        w.g.cities[city].defense_damage = defense_damage;
        let ctx = w.ctx();
        for hitter in [cannon, axeman] {
            assert_eq!(w.g.engage_hit(hitter, defender, Side::Attacker, &ctx).dv, 400, "{defense_damage}");
            let stat = w.g.engage_stat(defender, Some(hitter), Side::Defender, false, &ctx);
            assert!(!stat.terms.iter().any(|t| t.src == TermSource::CityDefense && t.pct != 0), "{:?}", stat.terms);
        }
        slots.push(w.g.engage_slots(&ctx).att());
    }
    // The slots: the two Sentinels', whatever the defence.
    assert_eq!(slots, vec![8, 8, 8]);
}

#[test]
fn knight_against_a_formation_axeman() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let knight = w.unit(p0, "UNIT_KNIGHT", from);
    let axeman = w.unit(p1, "UNIT_AXEMAN", to);
    w.promote(axeman, "PROMOTION_FORMATION");
    let (attack, back) = w.both_ways(knight, axeman);
    // Knight 121 (its sheet: 11 x 110% as a Flanker); the screen axeman 40 with Formation's +10%
    // defence and +40% against mounted units, on the Dv it shows the knight.
    assert_eq!(ad(attack), (1210, apply(400, 50)));
    // Back: the axeman's A with +40% against mounted, the knight's Dv 7 x 80%.
    assert_eq!(ad(back), (apply(400, 40), 560));
}

#[test]
fn typed_strength_is_scaled_by_the_opponent_resistance() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let elemental = w.unit(p0, "UNIT_FIRE_ELEMENTAL", from);
    let plain = w.unit(p1, "UNIT_AXEMAN", to);
    let immune = w.unit(p1, "UNIT_AXEMAN", to);
    w.promote(immune, "PROMOTION_IMMUNE_FIRE");
    let ctx = w.ctx();
    // A Fire Elemental is 6 + 3 fire (Frontline, 100%): 9 against an axeman, 6 against a
    // fire-immune one (Q2).
    assert_eq!(w.g.engage_hit(elemental, plain, Side::Attacker, &ctx).a, 900);
    assert_eq!(w.g.engage_hit(elemental, immune, Side::Attacker, &ctx).a, 600);
    // Its Dv against them: its fire points scaled by the hitter's resistance.
    assert_eq!(w.g.engage_hit(plain, elemental, Side::Defender, &ctx).dv, 900);
    assert_eq!(w.g.engage_hit(immune, elemental, Side::Defender, &ctx).dv, 600);
    // A promotion's points add to the sheet's (docs/UNIT_STATS.md): an archer's defence 30 and
    // Bronze Weapons' +10.
    let archer = w.unit(p1, "UNIT_ARCHER", to);
    w.promote(archer, "PROMOTION_BRONZE_WEAPONS");
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    assert_eq!(w.g.engage_hit(axeman, archer, Side::Attacker, &ctx).dv, 400);
}

#[test]
fn the_defense_percentage_goes_to_dv_and_the_attack_percentage_to_a() {
    // Q1: Enraged +20 A / -20 Dv; Shield of Faith +10 A / its defense percentage on Dv.
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let d = w.g.d;
    let shield = &d.promotions[d.t::<PromotionType>("PROMOTION_SHIELD_OF_FAITH")];
    assert!(shield.combat_percent_defense() > shield.combat_percent());
    for (name, a_pct, dv_pct) in [("PROMOTION_ENRAGED", 20, -20), ("PROMOTION_SHIELD_OF_FAITH", shield.combat_percent(), shield.combat_percent_defense())] {
        let attacker = w.unit(p0, "UNIT_AXEMAN", from);
        let defender = w.unit(p1, "UNIT_AXEMAN", to);
        w.promote(attacker, name);
        w.promote(defender, name);
        let (attack, back) = w.both_ways(attacker, defender);
        assert_eq!(ad(attack), (apply(400, a_pct), apply(400, dv_pct)), "{name}");
        assert_eq!(ad(back), (apply(400, a_pct), apply(400, dv_pct)), "{name}");
        // The duel applies the attack percentage to both.
        assert_eq!(w.g.unit_max_combat_str(defender, Some(to), Some(attacker), None), apply(400, a_pct), "{name}");
    }
}

#[test]
fn homeland_counts_where_the_unit_fights() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let attacker = w.unit(p0, "UNIT_AXEMAN", from);
    let defender = w.unit(p1, "UNIT_AXEMAN", to);
    w.promote(attacker, "PROMOTION_HOMELAND");
    w.promote(defender, "PROMOTION_HOMELAND");
    // Outside their borders: nothing.
    assert_eq!(ad2(w.both_ways(attacker, defender)), ((400, 400), (400, 400)));
    // Each in its own borders: +10% on A and Dv.
    w.g.map.plots[from].owner = Some(p0);
    w.g.map.plots[to].owner = Some(p1);
    assert_eq!(ad2(w.both_ways(attacker, defender)), ((440, 440), (440, 440)));
    // An estimate from an approach plot outside the attacker's borders: no Homeland for it.
    let elsewhere = w.g.map.plot_ids().find(|&p| !w.g.plot_is_water(p) && w.g.map.plots[p].owner.is_none()).unwrap();
    let ctx = HitCtx::new(elsewhere, to);
    assert_eq!(ad(w.g.engage_hit(attacker, defender, Side::Attacker, &ctx)), (400, 440));
}

#[test]
fn stigmata_follows_the_armageddon_counter() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let d = w.g.d;
    let attacker = w.unit(p0, "UNIT_AXEMAN", from);
    let defender = w.unit(p1, "UNIT_AXEMAN", to);
    w.promote(attacker, "PROMOTION_STIGMATA");
    w.promote(defender, "PROMOTION_STIGMATA");
    w.g.global_counter = w.g.global_counter_limit / 2;
    let per = d.promotions[d.t::<PromotionType>("PROMOTION_STIGMATA")].combat_percent_global_counter;
    let pct = w.g.get_global_counter() * per / 100;
    assert!(pct > 0);
    assert_eq!(ad2(w.both_ways(attacker, defender)), ((apply(400, pct), apply(400, pct)), (apply(400, pct), apply(400, pct))));
    // The job strengths have it too (it does not depend on the opponent).
    assert_eq!(w.g.engage_a_alone(attacker, Side::Attacker, &w.ctx()), apply(400, pct));
    assert_eq!(w.g.engage_job_defense(defender, &w.ctx()), apply(400, pct));
}

#[test]
fn an_attack_from_water_costs_a_unless_amphibious() {
    let mut w = world();
    let (p0, p1, to) = (w.p0, w.p1, w.to);
    let d = w.g.d;
    let water = w.g.map.plot_ids().find(|&p| w.g.plot_is_water(p)).expect("water");
    let attacker = w.unit(p0, "UNIT_AXEMAN", w.from);
    let defender = w.unit(p1, "UNIT_AXEMAN", to);
    let ctx = HitCtx::new(water, to);
    assert_eq!(d.defines.amphib_attack_modifier, -50);
    assert_eq!(ad(w.g.engage_hit(attacker, defender, Side::Attacker, &ctx)), (apply(400, -50), 400));
    // The defender hitting back has no such term (it does not attack from water).
    assert_eq!(ad(w.g.engage_hit(defender, attacker, Side::Defender, &ctx)), (400, 400));
    w.promote(attacker, "PROMOTION_AMPHIBIOUS");
    assert_eq!(w.g.engage_hit(attacker, defender, Side::Attacker, &ctx).a, 400);
    // An approach not known is by land.
    let unknown = HitCtx { from: None, ..ctx };
    w.g.unit_set_has_promotion(attacker, d.t::<PromotionType>("PROMOTION_AMPHIBIOUS"), false);
    assert_eq!(w.g.engage_hit(attacker, defender, Side::Attacker, &unknown).a, 400);
}

#[test]
fn a_river_gives_the_defender_no_dv() {
    // Q9: the river costs the attacker slots instead of the duel's defender bonus.
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    let g = &w.g;
    let (from, to) = g
        .map
        .plot_ids()
        .filter(|&p| !g.plot_is_water(p) && !g.plot_is_peak(p))
        .find_map(|p| {
            let (x, y) = g.map.xy(p);
            [(1, 0), (0, 1), (1, 1), (-1, 1)].into_iter().find_map(|(dx, dy)| {
                let q = g.map.plot_xy(x + dx, y + dy)?;
                (!g.plot_is_water(q) && !g.plot_is_peak(q) && g.plot_is_river_crossing(p, g.direction_xy_plots(p, q))).then_some((p, q))
            })
        })
        .expect("a river crossing between two land plots");
    let attacker = w.unit(p0, "UNIT_AXEMAN", from);
    let defender = w.unit(p1, "UNIT_AXEMAN", to);
    let ctx = HitCtx::new(from, to);
    let team1 = w.g.players[p1].team;
    let plot_defense = w.g.plot_defense_modifier(to, Some(team1), false);
    let terrain_defense = 0; // no axeman terrain or feature defense
    let expected_dv = apply(400, plot_defense + terrain_defense);
    assert_eq!(w.g.engage_hit(attacker, defender, Side::Attacker, &ctx).dv, expected_dv);
    // The duel's defender has the river bonus on top.
    let mut det = Default::default();
    w.g.unit_max_combat_str(defender, Some(to), Some(attacker), Some(&mut det));
    assert_eq!(det.river_attack_modifier, -w.g.d.defines.river_attack_modifier);
    assert!(det.river_attack_modifier > 0);
}

#[test]
fn the_handicap_keeps_the_duel_ratio_against_barbarians() {
    // An AI unit and a barbarian: today's duel puts the AI's barbarian bonus on the defender's
    // strength alone, so each pair carries it once, on the target's Dv.
    let mut w = world();
    let (p0, from, to) = (w.p0, w.from, w.to);
    let barbarian = w.g.barbarian_player();
    let (t0, tb) = (w.g.players[p0].team, w.g.players[barbarian].team);
    if !w.g.teams[t0].at_war[tb] {
        w.g.team_declare_war(t0, tb, false, None);
    }
    let bonus = w.g.d.handicaps[w.g.handicap].ai_barbarian_combat_modifier;
    assert_eq!(bonus, -25);
    let ai = w.unit(p0, "UNIT_AXEMAN", from);
    let barb = w.unit(barbarian, "UNIT_AXEMAN", to);
    let (attack, back) = w.both_ways(ai, barb);
    // The AI hits the barbarian: the barbarian's Dv / 1.25, as its duel strength defending.
    assert_eq!(ad(attack), (400, apply(400, bonus)));
    let duel_ratio = (w.g.unit_max_combat_str(ai, None, Some(barb), None), w.g.unit_max_combat_str(barb, Some(to), Some(ai), None));
    assert_eq!(duel_ratio, (attack.a, attack.dv));
    // The barbarian hits back: the AI's Dv x 1.25.
    assert_eq!(ad(back), (400, apply(400, -bonus)));
    // A barbarian attacking an AI city: the handicap's bonus still; the city's defense and its
    // barbarian defense give Dv nothing (revision 4: defended-plot terms).
    let (fx, fy) = w.g.map.xy(from);
    w.g.unit_kill(ai, false, None);
    let city = w.g.player_init_city(p0, fx, fy, true, true);
    let defender = w.unit(p0, "UNIT_AXEMAN", from);
    let ctx = HitCtx::new(to, from);
    let pair = w.g.engage_hit(barb, defender, Side::Attacker, &ctx);
    assert!(w.g.cities.contains(city));
    assert!(w.g.d.defines.city_barbarian_defense_modifier != 0);
    assert_eq!(pair.dv, apply(400, -bonus));
}

#[test]
fn each_side_has_only_its_own_terms() {
    // G1 on a defended plot with every kind of term: the defending side has its own defense
    // there (revision 4: not the plot's); the attacking side has its attack bonuses against the
    // plot and no defense there.
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let d = w.g.d;
    let forest = d.t::<FeatureType>("FEATURE_FOREST");
    w.g.map.plots[to].feature_type = Some(forest);
    w.g.map.plots[from].feature_type = Some(forest);
    let attacker = w.unit(p0, "UNIT_AXEMAN", from);
    let defender = w.unit(p1, "UNIT_AXEMAN", to);
    for u in [attacker, defender] {
        w.promote(u, "PROMOTION_WOODSMAN1");
        w.g.units[u].fortify_turns = 2;
    }
    let woodsman = &d.promotions[d.t::<PromotionType>("PROMOTION_WOODSMAN1")];
    let (attack_bonus, defense_bonus) = (woodsman.feature_attack_percent[forest], woodsman.feature_defense_percent[forest]);
    assert!(attack_bonus > 0 && defense_bonus > 0);
    assert!(d.features[forest].defense_modifier > 0);
    let (attack, back) = w.both_ways(attacker, defender);
    // The attacker's A: its forest attack; the defender's Dv: its forest defense and its
    // fortification (the forest's own cover is the ground's rules now, not Dv).
    assert_eq!(ad(attack), (apply(400, attack_bonus), apply(400, defense_bonus + 10)));
    // The defender hitting back: no forest attack; the attacker being hit: no forest, no
    // fortification, though it stands fortified in a forest.
    assert_eq!(ad(back), (400, 400));
}
