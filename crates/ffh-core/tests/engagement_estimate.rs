//! The engagement estimator (phase 6 step 7, plan 8.2; docs/ENGAGEMENTS.md, "The estimator"):
//! no random number drawn and nothing changed, hypothetical eligibility, the caller's defender
//! list (fog), damage and city-defense overrides, the assault, and the estimate against the real
//! engagement on the same battle.

use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::engagement::report::BattleReport;
use ffh_core::engagement::{swap_better, Attackers, Eligible, EngageResult, EstimateInput, HitCtx, Side, WarTest};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::rng::Rng;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map, players 0 and 1 at war, `from` and `to` two unowned flat grassland plots side by
/// side with no feature, improvement or river crossing (as `tests/engagements.rs`'s world).
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
    setup.options[GameOptionType::GAMEOPTION_ENGAGEMENTS] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
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
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_declare_war(t0, t1, false, None);
    World { g, p0, p1, from, to }
}

impl World {
    fn unit(&mut self, owner: PlayerId, name: &str, at: PlotIdx) -> UnitId {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        let u = self.g.player_init_unit(owner, ty, x, y, None, None);
        self.g.player_army_sync(owner);
        u
    }

    fn units(&mut self, owner: PlayerId, name: &str, at: PlotIdx, n: usize) -> Vec<UnitId> {
        (0..n).map(|_| self.unit(owner, name, at)).collect()
    }

    fn city(&mut self, owner: PlayerId, at: PlotIdx, pct: i32) -> CityId {
        let (x, y) = self.g.map.xy(at);
        let c = self.g.player_init_city(owner, x, y, true, true);
        let total = self.g.city_total_defense(c, false);
        self.g.cities[c].building_defense += pct - total;
        c
    }

    fn fork(&self, seed: u32) -> Game {
        let mut g = Game::load(self.g.d, &self.g.save()).expect("a save of this build");
        g.soren_rand = Rng::new(seed);
        g
    }

    fn on(&self, p: PlotIdx) -> Vec<UnitId> {
        self.g.map.plots[p].units.clone()
    }

    fn input<'a>(&self, attackers: Attackers<'a>, defenders: &'a [UnitId]) -> EstimateInput<'a> {
        EstimateInput::new(&self.g, self.p0, self.to, Some(self.from), attackers, defenders)
    }
}

/// `g`'s RNGs and its whole saved state.
fn state(g: &Game) -> (Rng, Rng, Vec<u8>) {
    (g.soren_rand.clone(), g.map_rand.clone(), g.save())
}

/// A mixed attack (with a catapult's collateral) on a mixed defense (archers' defensive strikes,
/// a reserve on a forest): every estimate draws no random number and changes nothing.
#[test]
fn estimates_draw_nothing_and_change_nothing() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.map.plots[to].feature_type = Some(w.g.d.t::<FeatureType>("FEATURE_FOREST"));
    let cats = w.units(p0, "UNIT_CATAPULT", from, 2);
    w.units(p0, "UNIT_AXEMAN", from, 6);
    w.units(p0, "UNIT_HORSEMAN", from, 2);
    w.units(p0, "UNIT_ARCHER", from, 2);
    w.units(p1, "UNIT_ARCHER", to, 4);
    w.units(p1, "UNIT_AXEMAN", to, 4);
    w.units(p1, "UNIT_WARRIOR", to, 2);
    let (pool, defs) = (w.on(from), w.on(to));
    let before = state(&w.g);
    let balanced = w.g.d.default_stance;
    let est = w.g.engage_estimate(&w.input(Attackers::Pick(&pool, balanced), &defs)).expect("an estimate");
    assert!(est.force.len() <= est.slots.max(Side::Attacker) as usize);
    assert!(!est.def_reserve.is_empty(), "a reserve on the forest: {:?}", est.slots);
    // A force with the catapults: their collateral and the archers' strikes are in the estimate.
    let s = est.slots.att() as usize;
    let force: Vec<UnitId> = cats.iter().copied().chain(pool.iter().copied().filter(|u| !cats.contains(u))).take(s).collect();
    let with_cats = w.g.engage_estimate(&w.input(Attackers::Force(&force), &defs)).unwrap();
    assert_eq!(with_cats.force, force);
    assert!(with_cats.opening.iter().any(|(u, x)| defs.contains(u) && *x > 0), "collateral: {:?}", with_cats.opening);
    assert!(with_cats.opening.iter().any(|(u, x)| force.contains(u) && *x > 0), "defensive strikes: {:?}", with_cats.opening);
    let assault = w.g.engage_estimate_assault(&w.input(Attackers::Pick(&pool, balanced), &defs));
    assert!(assault.steps.len() >= 2, "{} engagements", assault.steps.len());
    w.g.engage_estimate_plot(p0, from, to, None, None, WarTest::AtWar).unwrap();
    w.g.engage_estimate_plot(p0, from, to, Some(&force), Some(balanced), WarTest::AtWar).unwrap();
    assert!(state(&w.g) == before, "an estimate changed the game");
    // And it is a function of its inputs.
    let again = w.g.engage_estimate(&w.input(Attackers::Pick(&pool, balanced), &defs)).unwrap();
    assert_eq!(again, est);
}

/// `Eligible::Now` counts the units that can attack now; `Hypothetical` also those that could
/// once there: a unit far away, one that has attacked this turn. A catapult whose every target
/// is at its combat limit counts in neither, unless an override puts one below it.
#[test]
fn hypothetical_eligibility() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let near = w.unit(p0, "UNIT_AXEMAN", from);
    let tired = w.unit(p0, "UNIT_AXEMAN", from);
    w.g.units[tired].attacks_made = 1;
    w.g.units[tired].made_attack = true;
    let far_plot = w
        .g
        .map
        .plot_ids()
        .find(|&p| {
            let ((x, y), (tx, ty)) = (w.g.map.xy(p), w.g.map.xy(to));
            !w.g.plot_is_water(p) && !w.g.plot_is_peak(p) && w.g.map.plots[p].units.is_empty() && w.g.plot_distance(x, y, tx, ty) >= 4
        })
        .unwrap();
    let far = w.unit(p0, "UNIT_AXEMAN", far_plot);
    let cat = w.unit(p0, "UNIT_CATAPULT", from);
    let def = w.unit(p1, "UNIT_WARRIOR", to);
    let limit = w.g.units[cat].combat_limit;
    assert!(limit < 100);
    w.g.units[def].damage = limit;
    let defs = w.on(to);
    let all = [near, tired, far, cat];
    let mut inp = w.input(Attackers::Force(&all), &defs);
    let now = w.g.engage_estimate(&inp).unwrap();
    assert_eq!(now.force, vec![near]);
    inp.eligible = Eligible::Hypothetical;
    let hyp = w.g.engage_estimate(&inp).unwrap();
    assert_eq!(hyp.force, vec![near, tired, far], "the catapult has no target below its limit");
    let lower = [(def, 10)];
    inp.damage = &lower;
    let hyp = w.g.engage_estimate(&inp).unwrap();
    assert_eq!(hyp.force, vec![near, tired, far, cat]);
    // Nothing eligible: refused.
    let only_far = [far];
    let inp = w.input(Attackers::Force(&only_far), &defs);
    assert!(w.g.engage_estimate(&inp).is_err());
    let _ = p1;
}

/// The defense is the caller's list (what it sees): a unit left out of it never fights in the
/// estimate, and fewer defenders make a better estimate.
#[test]
fn the_defense_is_the_callers_list() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let pool = w.units(p0, "UNIT_AXEMAN", from, 4);
    let defs = w.units(p1, "UNIT_AXEMAN", to, 4);
    let balanced = w.g.d.default_stance;
    let all = w.g.engage_estimate(&w.input(Attackers::Pick(&pool, balanced), &defs)).unwrap();
    let seen = &defs[..2];
    let fog = w.g.engage_estimate(&w.input(Attackers::Pick(&pool, balanced), seen)).unwrap();
    assert_eq!(all.def_force.len(), 4);
    assert_eq!(fog.def_force.len(), 2);
    assert!(fog.def_force.iter().all(|u| seen.contains(u)));
    assert!(fog.win_permille > all.win_permille + 200, "{} against {}", fog.win_permille, all.win_permille);
    // A unit of the attacker's own team in the list is no defender.
    let mut mixed = seen.to_vec();
    mixed.push(pool[0]);
    let mixed_est = w.g.engage_estimate(&w.input(Attackers::Pick(&pool, balanced), &mixed)).unwrap();
    assert_eq!(mixed_est.def_force, fog.def_force);
    // No defender in the list: refused.
    assert!(w.g.engage_estimate(&w.input(Attackers::Pick(&pool, balanced), &[])).is_err());
    let _ = to;
}

/// Damage overrides and a hypothetical city defense change the battle the estimate integrates:
/// HP, the defender's pick, the slots and the Dv.
#[test]
fn overrides_change_the_battle() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.city(p1, to, 50);
    let pool = w.units(p0, "UNIT_AXEMAN", from, 10);
    let defs = w.units(p1, "UNIT_ARCHER", to, 3);
    let force = pool.clone();
    let base = w.g.engage_estimate(&w.input(Attackers::Force(&force), &defs)).unwrap();
    assert_eq!(base.slots.att(), 8, "a 50% city costs two slots: {:?}", base.slots);
    assert_eq!(base.force.len(), 8, "a force too big for the slots: what the default stance commits of it");
    // Planned softening: the defenders at 60% damage, against three axemen.
    let few = &pool[..3];
    let unsoftened = w.g.engage_estimate(&w.input(Attackers::Force(few), &defs)).unwrap();
    let soft: Vec<(UnitId, i32)> = defs.iter().map(|&u| (u, 60)).collect();
    let mut inp = w.input(Attackers::Force(few), &defs);
    inp.damage = &soft;
    let softened = w.g.engage_estimate(&inp).unwrap();
    let max_hp = w.g.unit_engage_max_hp(defs[0]);
    assert!(softened.start_hp[3..].iter().all(|&hp| hp <= 40 * max_hp), "{:?}", &softened.start_hp[3..]);
    assert!(unsoftened.start_hp[3..].iter().all(|&hp| hp == 100 * max_hp));
    assert!(softened.win_permille > unsoftened.win_permille, "{} against {}", softened.win_permille, unsoftened.win_permille);
    assert!(softened.att_loss_x1000 < unsoftened.att_loss_x1000);
    // A planned bombard to 0%: all ten slots, and the archers lose the walls' Dv.
    let mut inp = w.input(Attackers::Force(&force), &defs);
    inp.city_defense = Some(0);
    let bombarded = w.g.engage_estimate(&inp).unwrap();
    assert_eq!(bombarded.slots.att(), 10);
    assert_eq!(bombarded.force.len(), 10);
    let ctx = HitCtx { to, from: Some(from), city_defense: Some(0) };
    let walls = HitCtx::new(from, to);
    assert!(w.g.engage_job_defense(defs[0], &ctx) < w.g.engage_job_defense(defs[0], &walls));
    assert!(bombarded.win_permille >= base.win_permille);
    // The plot as the rules see it: the same as the estimate of the true lists.
    let plot = w.g.engage_estimate_plot(p0, from, to, Some(&force[..8]), None, WarTest::AtWar).unwrap();
    let listed = w.g.engage_estimate(&w.input(Attackers::Force(&force[..8]), &defs)).unwrap();
    assert_eq!(plot, listed);
}

/// The assault: more defenders than slots on a forest. Engagements follow until the plot falls,
/// each with fresh attackers, the defenders carrying their damage and alive weight.
#[test]
fn the_assault_carries_the_defense_from_one_engagement_to_the_next() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.map.plots[to].feature_type = Some(w.g.d.t::<FeatureType>("FEATURE_FOREST"));
    let slots = w.g.engage_slots(&HitCtx::new(from, to));
    let s = slots.def() as usize;
    let pool = w.units(p0, "UNIT_AXEMAN", from, 3 * s);
    let defs = w.units(p1, "UNIT_WARRIOR", to, s + 3);
    let balanced = w.g.d.default_stance;
    let inp = w.input(Attackers::Pick(&pool, balanced), &defs);
    let a = w.g.engage_estimate_assault(&inp);
    let first = w.g.engage_estimate(&inp).unwrap();
    assert_eq!(a.first, first.win_permille);
    assert_eq!(a.steps[0].force, first.force);
    assert!(a.steps.len() >= 2 && a.falls, "{} engagements, falls {}", a.steps.len(), a.falls);
    assert_eq!(a.used, Some(a.steps.len()));
    assert_eq!((a.n_att, a.n_def), (3 * s, s + 3));
    assert!(a.kills_x1000 > (s as i32 + 2) * 1000, "{}", a.kills_x1000);
    assert!(a.def_left_x1000 < 500);
    // Each engagement commits attackers not used before.
    let mut used: Vec<UnitId> = Vec::new();
    for st in &a.steps {
        assert!(st.force.iter().all(|u| !used.contains(u)));
        used.extend(&st.force);
    }
    // The second engagement's defenders include the first one's reserve, and a carried
    // defender starts with the damage of the first.
    let reserve = &a.steps[0].def_reserve;
    assert!(reserve.iter().all(|u| a.steps[1].def_force.contains(u)), "{reserve:?} {:?}", a.steps[1].def_force);
    let carried = a.steps[1].def_force.iter().position(|u| a.steps[0].def_force.contains(u));
    if let Some(j) = carried {
        let n = a.steps[1].force.len();
        assert!(a.steps[1].exp.fighters[n + j].mu > 0, "a carried defender's damage");
    }
    // Too few attackers: the plot holds.
    let few = &pool[..2];
    let short = w.g.engage_estimate_assault(&w.input(Attackers::Pick(few, balanced), &defs));
    assert!(!short.falls && short.used.is_none());
    assert_eq!(short.steps.len(), 1);
}

/// The estimate of an attack is of the battle the real engagement fights: the same force, the
/// same defender's pick, the same HP; and its win chance is near the real one's rate over seeds.
#[test]
fn the_estimate_is_of_the_real_battle() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p1].human = true; // the battle log is written
    let pool = w.units(p0, "UNIT_AXEMAN", from, 5);
    w.g.units[pool[1]].damage = 30;
    let defs = w.units(p1, "UNIT_AXEMAN", to, 4);
    w.g.units[defs[2]].damage = 20;
    let stance = w.g.engage_attack_stance(p0, from, w.g.units[pool[0]].group).stance;
    let candidates = w.g.engage_defenders(to, p0, &pool, WarTest::AtWar);
    let mut inp = w.input(Attackers::Pick(&pool, stance), &candidates);
    inp.def_stance = w.g.engage_defense_stance(to, &candidates).stance;
    let est = w.g.engage_estimate(&inp).unwrap();
    let (mut won, runs) = (0, 60);
    for seed in 0..runs {
        let mut g = w.fork(seed);
        let grp = g.units[pool[0]].group.unwrap();
        let before = g.battle_report_next;
        let (x, y) = g.map.xy(to);
        g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
        let r: &BattleReport = g.battle_reports.iter().find(|r| r.id >= before).expect("an engagement");
        if seed == 0 {
            let log = r.log.as_ref().unwrap();
            let committed = |s: Side| log.fighters.iter().filter(|f| f.side == s && f.slot.is_some()).map(|f| (f.unit, f.hp_start)).collect::<Vec<_>>();
            let att: Vec<(UnitId, i32)> = est.force.iter().zip(&est.start_hp).map(|(&u, &hp)| (u, hp)).collect();
            let def: Vec<(UnitId, i32)> = est.def_force.iter().zip(&est.start_hp[est.force.len()..]).map(|(&u, &hp)| (u, hp)).collect();
            assert_eq!(committed(Side::Attacker), att);
            assert_eq!(committed(Side::Defender), def);
        }
        won += (r.outcome.result == EngageResult::AttackerWon) as i32;
    }
    let rate = won * 1000 / runs as i32;
    assert!((rate - est.win_permille).abs() <= 200, "the estimate {} against {rate} realized", est.win_permille);
}

/// The swap search (plan 15 item 3): the attacker's proposal starts from the stance's quota
/// pick and keeps a better plan of single swaps by the estimate: here Balanced's quota fills the
/// back half of the slots with archers, and an axeman in reserve does better in front. The
/// proposal is a valid force, at least as likely to win, and the defender's pick does not change
/// with it. Preserve Veterans' held units stay out; with no reserve there is nothing to swap.
#[test]
fn the_proposal_keeps_the_better_single_swaps() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.map.plots[to].feature_type = Some(w.g.d.t::<FeatureType>("FEATURE_FOREST"));
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 6);
    let archers = w.units(p0, "UNIT_ARCHER", from, 4);
    let defs = w.units(p1, "UNIT_AXEMAN", to, 5);
    let ctx = HitCtx::new(from, to);
    let slots = w.g.engage_slots(&ctx);
    let pool: Vec<UnitId> = axemen.iter().chain(&archers).copied().collect();
    assert!(pool.len() as i32 > slots.att(), "a reserve: {:?}", slots);
    let balanced = w.g.d.default_stance;
    let quota = w.g.engage_auto_pick(&pool, balanced, &slots, Side::Attacker, &ctx);
    let proposal = w.g.engage_propose_force(&pool, balanced, &slots, &ctx);
    assert!(w.g.engage_validate_force(&pool, &proposal, &slots).is_ok());
    assert_ne!(proposal, quota, "an axeman in reserve beats an archer in the force");
    let count = |f: &[UnitId]| f.iter().filter(|u| axemen.contains(u)).count();
    assert!(count(&proposal) > count(&quota));
    let est = |f: &[UnitId]| w.g.engage_estimate(&w.input(Attackers::Force(f), &defs)).unwrap();
    let (q, p) = (est(&quota), est(&proposal));
    assert!(p.win_permille >= q.win_permille - 2 && (p.win_permille > q.win_permille || p.att_loss_x1000 < q.att_loss_x1000), "{} / {} against {} / {}", p.win_permille, p.att_loss_x1000, q.win_permille, q.att_loss_x1000);
    assert_eq!(p.def_force, q.def_force, "the defender's pick is blind to the attacker's force");
    // A real attack with no force given fights the proposal.
    let mut g = Game::load(w.g.d, &w.g.save()).unwrap();
    let before = g.battle_report_next;
    let grp = g.units[axemen[0]].group.unwrap();
    let (x, y) = g.map.xy(to);
    g.players[p1].human = true;
    g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
    let r = g.battle_reports.iter().find(|r| r.id >= before).unwrap();
    let log = r.log.as_ref().unwrap();
    let fought: Vec<UnitId> = log.fighters.iter().filter(|f| f.side == Side::Attacker && f.slot.is_some()).map(|f| f.unit).collect();
    let stance = g.engage_attack_stance(p0, from, Some(grp)).stance;
    assert_eq!(fought, w.g.engage_propose_force(&pool, stance, &slots, &ctx));
    assert_eq!(log.picks[0].picked_by, ffh_core::engagement::report::Pick::Auto, "the proposal is the auto-pick");
    // Preserve Veterans: a veteran in reserve is never swapped in.
    let vet = axemen[5];
    w.g.units[vet].level = 6;
    let preserve = w.g.d.t::<StanceType>("STANCE_PRESERVE_VETERANS");
    let quota = w.g.engage_auto_pick(&pool, preserve, &slots, Side::Attacker, &ctx);
    assert!(!quota.contains(&vet));
    let proposal = w.g.engage_propose_force(&pool, preserve, &slots, &ctx);
    assert!(!proposal.contains(&vet), "{proposal:?}");
    // No reserve: the quota pick.
    let few: Vec<UnitId> = pool[..slots.att() as usize].to_vec();
    let quota = w.g.engage_auto_pick(&few, balanced, &slots, Side::Attacker, &ctx);
    assert_eq!(w.g.engage_propose_force(&few, balanced, &slots, &ctx), quota);
}

/// The swap search never worsens the plan (plan 15 item 3), on constructed battles: mixed pools
/// with a reserve (some units wounded, some veterans), against mixed defenses on open land, a
/// forest and a walled city, under every stance. The proposal is a valid force, a unit the stance
/// holds is never swapped in, the defender's pick is the quota pick's, and the proposal's
/// estimate is the quota pick's or beats it (`swap_better`); on some battles it beats it.
#[test]
fn the_swap_search_never_worsens_the_plan() {
    let att_types = ["UNIT_AXEMAN", "UNIT_ARCHER", "UNIT_HORSEMAN", "UNIT_WARRIOR", "UNIT_CATAPULT", "UNIT_SWORDSMAN", "UNIT_CHARIOT", "UNIT_HUNTER"];
    let def_types = ["UNIT_AXEMAN", "UNIT_ARCHER", "UNIT_WARRIOR", "UNIT_PHALANX", "UNIT_HORSEMAN"];
    let stances: Vec<StanceType> = ffh_data::global().stances.keys().collect();
    let mut rng = Rng::new(0x5357_4150);
    let (mut cases, mut improved, mut swapped_in) = (0, 0, 0);
    for case in 0..30usize {
        let mut w = world();
        let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
        match case % 3 {
            1 => w.g.map.plots[to].feature_type = Some(w.g.d.t::<FeatureType>("FEATURE_FOREST")),
            2 => {
                w.city(p1, to, 40);
            }
            _ => {}
        }
        let ctx = HitCtx::new(from, to);
        let slots = w.g.engage_slots(&ctx);
        let n_att = slots.att() as usize + 2 + rng.get(4, "test") as usize;
        let pool: Vec<UnitId> = (0..n_att).map(|_| w.unit(p0, att_types[rng.get(att_types.len() as u16, "test") as usize], from)).collect();
        for &u in &pool {
            match rng.get(6, "test") {
                0 => w.g.units[u].damage = 45,
                1 => w.g.units[u].damage = 20,
                2 => w.g.units[u].level = 6,
                _ => {}
            }
        }
        let n_def = 3 + rng.get(8, "test") as usize;
        for _ in 0..n_def {
            let u = w.unit(p1, def_types[rng.get(def_types.len() as u16, "test") as usize], to);
            if rng.get(4, "test") == 0 {
                w.g.units[u].damage = 30;
            }
        }
        let defs = w.on(to);
        let stance = stances[case % stances.len()];
        let quota = w.g.engage_auto_pick(&pool, stance, &slots, Side::Attacker, &ctx);
        let proposal = w.g.engage_propose_force(&pool, stance, &slots, &ctx);
        assert!(w.g.engage_validate_force(&pool, &proposal, &slots).is_ok(), "case {case}: {proposal:?}");
        assert_eq!(proposal.len(), quota.len());
        for &u in proposal.iter().filter(|u| !quota.contains(u)) {
            assert!(!w.g.engage_is_held(u, stance), "case {case}: held unit {u:?} swapped in");
            swapped_in += 1;
        }
        let est = |f: &[UnitId]| w.g.engage_estimate(&w.input(Attackers::Force(f), &defs)).unwrap();
        let (q, p) = (est(&quota), est(&proposal));
        assert_eq!(p.def_force, q.def_force, "case {case}: the defender's pick is blind to the attacker's force");
        if proposal != quota {
            assert!(swap_better(&p, &q), "case {case}: {} / {} against the quota pick's {} / {}", p.win_permille, p.att_loss_x1000, q.win_permille, q.att_loss_x1000);
            improved += 1;
        }
        cases += 1;
    }
    println!("the swap search changed {improved} of {cases} plans ({swapped_in} units swapped in)");
    assert!(improved >= 3, "the search changed only {improved} of {cases} plans");
}
