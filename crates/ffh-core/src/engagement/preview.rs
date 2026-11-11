//! The attack preview (spec section 2; the plan screen): what the player sees before an attack.
//! Pure (`&self`): it plays the battle many times on a private RNG and never touches the game's.
//!
//! **The plan** ([`Game::engage_preview`]): the attacker's pool on the plot it attacks from, its
//! slots and lanes, the formation (the one given, checked, or the rules' auto formation), the auto
//! formation itself and the pool's units not in the formation; the defenders the attacker's team
//! sees and their formation (the rules' auto formation of them, as the real fight forms them),
//! the ground and its rules.
//!
//! **The forecast**: `ENGAGEMENT_PREVIEW_RUNS` battles of the real fight's own code on that
//! set-up ([`Game::engage_setup`]): the morale check and the opening's defensive strikes as pure
//! rolls ([`Game::engage_morale_rolls`], [`Game::engage_opening_rolls`]), then the formation
//! resolver, all on an [`Rng`] seeded from the plan (both formations, the plots, the turn), so
//! the same plan always shows the same numbers. A move that declares war previews the battle after
//! the declaration: the opening's strikes test war as that fight does. It gives each side's
//! expected units fallen and escaped, the chance the defender's line is wiped out and the chance
//! the plot falls (its line wiped out with no defender left on the plot: none in reserve, none
//! that sat out), the chance the attacker's line is wiped out, and per committed fighter its
//! chance to fall and to escape.
//!
//! **Frozen** ([`Game::engage_forecast_frozen`]): when a human attacker's attack whose battle gets
//! a log is made, its plan is recomputed (the same seed: the numbers it saw) into the log
//! (`report::FrozenForecast`).

use super::arrange::Place;
use super::engage::{damage_from_hp, hp_from_damage, BattleEffect, Setup};
use super::formation::{self, FighterIdx, Fate, Ground, GroundRule, Side};
use super::pick::WarTest;
use super::report::{ForecastFighter, FrozenForecast};
use super::slots::Slots;
use super::stats::HitCtx;
use super::summons::{Called, SummonCall};
use ffh_data::{SpellType, UnitType};
use crate::game::Game;
use crate::ids::*;
use crate::rng::Rng;
use crate::unit::{Declared, UnitOverlay};

/// What a preview is of.
#[derive(Copy, Clone, Debug)]
pub struct PreviewInput<'a> {
    pub player: PlayerId,
    pub from: PlotIdx,
    pub to: PlotIdx,
    /// The attacker's formation (`None`: the rules' auto formation).
    pub formation: Option<&'a [Place]>,
    /// The war the attack assumes ([`WarTest::DeclareWar`] for a move that declares it).
    pub war: WarTest,
}

impl<'a> PreviewInput<'a> {
    /// `player`'s attack from `from` on `to` at war, in the auto formation.
    pub fn new(player: PlayerId, from: PlotIdx, to: PlotIdx) -> PreviewInput<'a> {
        PreviewInput { player, from, to, formation: None, war: WarTest::AtWar }
    }
}

/// A fighter of the forecast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewFighter {
    pub unit: UnitId,
    pub side: Side,
    /// The chance it falls, and escapes (percent).
    pub fall_pct: i32,
    pub escape_pct: i32,
}

/// A Sentinel of the defended city in a plan (docs/SIEGES.md): its cell, HP and max HP (centi-HP
/// and HP), A and Dv (x100), and its chance to be breached (percent).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewSentinel {
    /// The city's, or (card) the field's.
    pub src: super::more_cards::SentinelSource,
    pub lane: u8,
    pub row: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub a: i32,
    pub dv: i32,
    pub fall_pct: i32,
}

/// The sampled forecast of a plan (the battles of the real resolver on the preview's RNG).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forecast {
    pub runs: i32,
    /// The chance (per mille) the defender's line is wiped out, the plot falls, and the
    /// attacker's line is wiped out.
    pub wiped_permille: i32,
    pub falls_permille: i32,
    pub repulsed_permille: i32,
    /// Expected units fallen and escaped per side (attacker, defender), x100.
    pub lost_x100: [i32; 2],
    pub escaped_x100: [i32; 2],
    /// Each committed fighter, in the engagement's order (the attacker's formation by lane then
    /// row, then the defender's).
    pub per: Vec<PreviewFighter>,
    /// Each Sentinel's chance to be breached (percent), in the plan's order.
    pub sentinel_fall_pct: Vec<i32>,
}

/// The attack preview (module comment).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngagePreview {
    pub slots: Slots,
    /// Lanes per side (attacker, defender).
    pub lanes: [u8; 2],
    /// The defended plot's ground and its rules that apply.
    pub ground: Ground,
    pub rules: Vec<GroundRule>,
    /// The attacker's pool, the plan's formation, the rules' auto formation of the pool, and the
    /// pool's units not in the plan's formation.
    pub pool: Vec<UnitId>,
    pub formation: Vec<Place>,
    pub auto: Vec<Place>,
    pub reserve: Vec<UnitId>,
    /// The defender's candidates the attacker's team sees, their formation, and those not in it.
    pub defenders: Vec<UnitId>,
    pub def_formation: Vec<Place>,
    pub def_reserve: Vec<UnitId>,
    /// The defended city's Sentinels that fight (docs/SIEGES.md).
    pub sentinels: Vec<PreviewSentinel>,
    /// The summons the battle calls, each side's (summons.rs); not in `forecast.per`.
    pub summons: Vec<PreviewSummon>,
    pub forecast: Forecast,
    /// The team an attack would declare war on.
    pub declare_war: Option<TeamId>,
    /// Each side's battle tactic (attacker, defender; docs/TACTICS.md): what arranges `auto` and
    /// `def_formation`.
    pub tactics: [ffh_data::BattleTacticType; 2],
    /// The committed units (either side) that belong to a starving army (docs/SUPPLY.md), each
    /// with the percent change of the damage it deals (`STARVATION_DAMAGE_DEALT_PERCENT`); the
    /// forecast counts it.
    pub starving: Vec<(UnitId, i32)>,
}

/// What the attacker knows of the defense: the defender's candidates, the units on the plot its
/// opening reads (strikers), and the war its opening assumes (a declare-war plan's: once declared;
/// `None`: the wars now).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DefView {
    pub candidates: Vec<UnitId>,
    pub on_plot: Vec<UnitId>,
    pub declared: Option<Declared>,
}

/// A fighter's fate in one sampled battle.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SampleFate {
    /// Not on the field (sat out, or a unit a kill may create that none did).
    Off,
    Standing,
    Fell,
    Withdrew,
}

/// One sampled battle (`Game::engage_sample_battle`), by set-up index.
struct SampledBattle {
    fate: Vec<SampleFate>,
    end_hp: Vec<i32>,
    sat: Vec<bool>,
    down: [bool; 2],
}

/// FNV-1a over the words: the preview RNG's seed.
fn seed_of(words: &[u32]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for w in words {
        for b in w.to_le_bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    h
}

/// The preview's set-up: the pool, the plan's formation (as given, else the auto formation), the
/// auto formation, the defender's formation, the view of the defense and the battle's set-up;
/// with summons (summons.rs), the copy of the game they were made in, whose set-up it is.
struct Plan {
    pool: Vec<UnitId>,
    formation: Vec<Place>,
    auto: Vec<Place>,
    def_formation: Vec<Place>,
    view: DefView,
    setup: Setup,
    scratch: Option<Box<Game>>,
    summons: Vec<SummonCall>,
}

/// A summon of a plan (summons.rs): who calls it, its unit type, side and cell, its HP and max
/// HP (centi-HP and HP) and A against nobody (x100), and its chance to fall (percent). It is made
/// for the battle, so it has no unit in the game yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewSummon {
    /// Its caster (a unit a kill may create: its creator, or the caster of the summon that is).
    pub caster: UnitId,
    /// The summon's spell; `None`: a unit a kill may create, off the field until one does.
    pub spell: Option<SpellType>,
    pub unit_type: UnitType,
    pub side: Side,
    pub lane: u8,
    pub row: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub a: i32,
    pub fall_pct: i32,
}

impl Forecast {
    /// This forecast frozen for a battle log, each fighter named by its log index (`index_of`).
    pub(crate) fn frozen(&self, index_of: impl Fn(UnitId) -> Option<FighterIdx>) -> FrozenForecast {
        FrozenForecast {
            runs: self.runs,
            wiped_permille: self.wiped_permille,
            falls_permille: self.falls_permille,
            repulsed_permille: self.repulsed_permille,
            lost_x100: self.lost_x100,
            per: self.per.iter().filter_map(|x| Some(ForecastFighter { b: index_of(x.unit)?, fall_pct: x.fall_pct, escape_pct: x.escape_pct })).collect(),
        }
    }
}

impl Game {
    /// The attack preview of `inp` (module comment). `Err`: the reason the attack is refused, as
    /// the real fight gives it (an invalid formation, no unit that can attack, nobody there), or
    /// the plot is not visible to the attacker's team.
    pub fn engage_preview(&self, inp: &PreviewInput) -> Result<EngagePreview, &'static str> {
        let plan = self.engage_preview_plan(inp)?;
        let runs = self.d.defines.engagement_preview_runs.max(1);
        let g = plan.scratch.as_deref().unwrap_or(self);
        let mut forecast = g.engage_forecast_sampled(&plan.setup, &plan.view, runs);
        let s = &plan.setup;
        // The summons, which have no unit in this game, apart from the units.
        let summons: Vec<PreviewSummon> = (0..s.units.len())
            .filter(|&i| g.units[s.units[i]].battle_summon)
            .filter_map(|i| {
                let u = s.units[i];
                let call = plan.summons.iter().find(|c| c.side == s.sides[i] && (c.lane, c.row) == s.places[i])?;
                let f = &s.fighters[i];
                let fall_pct = forecast.per.iter().find(|x| x.unit == u).map_or(0, |x| x.fall_pct);
                let spell = match call.what {
                    Called::Summon(sp) => Some(sp),
                    Called::OnKill(_) => None,
                };
                Some(PreviewSummon { caster: call.caster, spell, unit_type: call.unit_type, side: call.side, lane: call.lane, row: call.row, hp: f.hp, max_hp: f.max_hp, a: f.a, fall_pct })
            })
            .collect();
        forecast.per.retain(|x| !g.units[x.unit].battle_summon);
        let formation = plan.formation.clone();
        let reserve = plan.pool.iter().copied().filter(|u| !formation.iter().any(|p| p.unit == *u)).collect();
        let declare_war = match inp.war {
            WarTest::DeclareWar(t) => Some(t),
            WarTest::AtWar => None,
        };
        let sentinels = s
            .sentinel_range()
            .enumerate()
            .map(|(j, i)| {
                let f = &s.fighters[i];
                let src = s.sentinel_source(i).expect("a Sentinel has a source");
                let dv = match src {
                    super::more_cards::SentinelSource::City(c) => self.city_sentinel_dv_x100(c),
                    super::more_cards::SentinelSource::Field { grow, .. } => self.field_sentinel_def_x100(grow, false),
                };
                PreviewSentinel { src, lane: f.lane, row: f.row, hp: f.hp, max_hp: f.max_hp, a: f.a, dv, fall_pct: forecast.sentinel_fall_pct.get(j).copied().unwrap_or(0) }
            })
            .collect();
        Ok(EngagePreview {
            slots: s.slots.clone(),
            lanes: s.lanes,
            ground: s.ground,
            rules: s.ground.rules(&s.params),
            pool: plan.pool.clone(),
            formation,
            auto: plan.auto.clone(),
            reserve,
            defenders: plan.view.candidates.clone(),
            def_formation: plan.def_formation.clone(),
            def_reserve: s.def_reserve(),
            sentinels,
            summons,
            forecast,
            declare_war,
            tactics: [self.engage_pool_tactic(&plan.pool, Side::Attacker, &HitCtx::new(inp.from, inp.to)), self.engage_pool_tactic(&plan.view.candidates, Side::Defender, &HitCtx::new(inp.from, inp.to))],
            starving: g.engage_dealt_mods(s),
        })
    }

    /// What `p`'s team knows of the defense of `to` against `force` (module comment): the
    /// defender's candidates (as the real fight finds them: invisible units are never
    /// candidates), the units on the plot it can see and the war `war` assumes.
    pub(crate) fn engage_def_view(&self, p: PlayerId, to: PlotIdx, force: &[UnitId], war: WarTest) -> DefView {
        let team = self.players[p].team;
        let candidates = self.engage_defenders(to, p, force, war);
        let on_plot = self.map.plots[to].units.iter().copied().filter(|&u| !self.unit_is_invisible(u, team, true)).collect();
        DefView { candidates, on_plot, declared: self.engage_declaration(team, war) }
    }

    fn engage_preview_plan(&self, inp: &PreviewInput) -> Result<Plan, &'static str> {
        let (p, from, to) = (inp.player, inp.from, inp.to);
        let ctx = HitCtx::new(from, to);
        let slots = self.engage_slots(&ctx);
        let pool = self.engage_attackers(p, from, to, inp.war);
        let auto = self.engage_auto_formation(&pool, Side::Attacker, &slots, &ctx);
        let formation = match inp.formation {
            Some(f) => {
                self.engage_validate_formation(&pool, f, &slots)?;
                f.to_vec()
            }
            None => auto.clone(),
        };
        if formation.is_empty() {
            return Err("no unit can attack there");
        }
        if !self.plot_is_visible(to, self.players[p].team) {
            return Err("you cannot see that plot");
        }
        let force: Vec<UnitId> = formation.iter().map(|q| q.unit).collect();
        let view = self.engage_def_view(p, to, &force, inp.war);
        self.engage_plan_finish(ctx, slots, pool, formation, auto, view)
    }

    /// The rest of a plan once the attacker's pool and formation and the view of the defense are
    /// known: the defender's formation, the summons (in a copy of the game) and the battle's
    /// set-up. Shared by the preview and the hypothetical forecast ([`Game::engage_forecast_if`]).
    fn engage_plan_finish(&self, ctx: HitCtx, slots: Slots, pool: Vec<UnitId>, formation: Vec<Place>, auto: Vec<Place>, view: DefView) -> Result<Plan, &'static str> {
        // Card: the field's Sentinels take cells of the defender's grid (more_cards.rs).
        let mut slots = slots;
        self.engage_add_field_sentinels(&mut slots, &view.candidates, ctx.to);
        let def_formation = self.engage_defense_formation(&view.candidates, &slots, &ctx);
        // The summons (summons.rs) are made for the battle: the plan makes them in a copy of the
        // game, as the real fight will (the same units, the same ids), and is that copy's.
        let summons = self.engage_summon_calls(&ctx, &slots, &formation, &view.candidates, true);
        if summons.is_empty() {
            let setup = self.engage_setup(ctx, slots, formation.clone(), view.candidates.clone(), true, &[])?;
            return Ok(Plan { pool, formation, auto, def_formation, view, setup, scratch: None, summons });
        }
        let mut scratch = Box::new(Game::load(self.d, &self.save()).expect("a game loads its own save"));
        let made = scratch.engage_call_summons(&summons);
        let setup = scratch.engage_setup(ctx, slots, formation.clone(), view.candidates.clone(), true, &made)?;
        Ok(Plan { pool, formation, auto, def_formation, view, setup, scratch: Some(scratch), summons })
    }

    /// The seed of a set-up's forecast: both formations (sorted), the plots, the turn.
    fn engage_forecast_seed(&self, setup: &Setup) -> u32 {
        let mut words: Vec<u32> = Vec::with_capacity(3 * setup.units.len() + 6);
        words.push(0x464f_524d); // "FORM"
        for s in [Side::Attacker, Side::Defender] {
            let mut f = setup.formation(s);
            f.sort();
            for p in f {
                words.extend([p.unit.0, p.lane as u32, p.row as u32]);
            }
            words.push(u32::MAX);
        }
        words.push(setup.ctx.from.map_or(u32::MAX, |p| p.0));
        words.push(setup.ctx.to.0);
        words.push(self.game_turn() as u32);
        seed_of(&words)
    }

    /// One battle of `setup` sampled on `rng` (the real fight's morale check, opening and battle):
    /// the opening over the units `view.on_plot`, reading and updating `ov` (the units' damage);
    /// the Sentinels' HP from `sentinel_hp` (by set-up index) when given, else the set-up's.
    /// Each fighter's fate (set-up index), HP at the end, who sat out and which lines went down.
    fn engage_sample_battle(&self, setup: &Setup, view: &DefView, pre: &[BattleEffect], ov: &mut UnitOverlay, sentinel_hp: Option<&dyn Fn(usize) -> i32>, rng: &mut Rng) -> SampledBattle {
        let n = setup.units.len();
        let n_all = setup.fighters.len();
        let n_att = setup.n_att;
        let mut sat = setup.initial_sat_out();
        self.engage_morale_rolls(&setup.units, n_att, &mut sat, rng);
        self.engage_opening_rolls(&setup.units, n_att, &sat, &view.on_plot, ov, rng);
        let mut hp: Vec<i32> = (0..n_all)
            .map(|i| match (i < n, sentinel_hp) {
                (true, _) => hp_from_damage(ov.damage(self, setup.units[i]), setup.fighters[i].max_hp),
                (false, Some(f)) => f(i),
                (false, None) => setup.fighters[i].hp,
            })
            .collect();
        let mut effects = pre.to_vec();
        effects.extend(self.engage_opening_spells(setup, &hp));
        // Card: Thunderhead Tower (docs/CARDS.md), as the real fight.
        self.engage_arms_opening(setup, &sat, &mut hp, None);
        let (f, map) = if effects.is_empty() {
            setup.battle(&sat, &hp)
        } else {
            let mut s = setup.clone();
            s.apply_effects(&effects, &mut hp);
            s.battle(&sat, &hp)
        };
        let out = formation::resolve_quiet(&f, &mut *rng);
        let mut fate = vec![SampleFate::Off; n_all];
        let mut end_hp = hp;
        // Card: an illusion (past the set-up's fighters) is not counted.
        for (k, e) in out.fighters.iter().enumerate().filter(|&(k, _)| map[k] < n_all) {
            fate[map[k]] = match e.fate {
                Fate::Fell { .. } => SampleFate::Fell,
                Fate::Withdrew { .. } => SampleFate::Withdrew,
                Fate::Standing => SampleFate::Standing,
            };
            end_hp[map[k]] = e.hp;
        }
        SampledBattle { fate, end_hp, sat, down: out.down }
    }

    /// `runs` battles of `setup` (the real fight's morale check, opening and battle) on a private
    /// RNG seeded from the set-up, the opening over the units `view.on_plot` (what the attacker
    /// sees there) at war as `view` has it.
    pub(crate) fn engage_forecast_sampled(&self, setup: &Setup, view: &DefView, runs: i32) -> Forecast {
        let n = setup.units.len();
        let n_all = setup.fighters.len();
        let mut rng = Rng::new(self.engage_forecast_seed(setup));
        let pre = self.engage_pre_battle(setup);
        let fresh = UnitOverlay::with_declared(view.declared.as_ref());
        // The plot falls when the defender's line is wiped out and no defender is left: none in
        // reserve and none that sat out (withdrawn defenders flee).
        let reserve = !setup.def_reserve().is_empty();
        let (mut wiped, mut falls, mut repulsed) = (0i64, 0i64, 0i64);
        let mut fell = vec![0i64; n_all];
        let mut escaped = vec![0i64; n_all];
        for _ in 0..runs {
            let mut ov = fresh.clone();
            let b = self.engage_sample_battle(setup, view, &pre, &mut ov, None, &mut rng);
            for i in 0..n_all {
                match b.fate[i] {
                    SampleFate::Fell => fell[i] += 1,
                    SampleFate::Withdrew => escaped[i] += 1,
                    _ => {}
                }
            }
            if b.down[1] {
                wiped += 1;
                let sat_def = setup.of(Side::Defender).any(|i| b.sat[i]);
                if !reserve && !sat_def {
                    falls += 1;
                }
            }
            if b.down[0] {
                repulsed += 1;
            }
        }
        let runs64 = runs as i64;
        let per_mille = |k: i64| (k * 1000 / runs64) as i32;
        let side_sum = |v: &[i64], s: Side| -> i32 { (setup.of(s).map(|i| v[i]).sum::<i64>() * 100 / runs64) as i32 };
        Forecast {
            runs,
            wiped_permille: per_mille(wiped),
            falls_permille: per_mille(falls),
            repulsed_permille: per_mille(repulsed),
            lost_x100: [side_sum(&fell, Side::Attacker), side_sum(&fell, Side::Defender)],
            escaped_x100: [side_sum(&escaped, Side::Attacker), side_sum(&escaped, Side::Defender)],
            per: (0..n)
                .map(|i| PreviewFighter {
                    unit: setup.units[i],
                    side: setup.sides[i],
                    fall_pct: (fell[i] * 100 / runs64) as i32,
                    escape_pct: (escaped[i] * 100 / runs64) as i32,
                })
                .collect(),
            sentinel_fall_pct: setup.sentinel_range().map(|i| (fell[i] * 100 / runs64) as i32).collect(),
        }
    }

    /// The plan of `p`'s attack from `from` on `to` with `formation`, frozen for its battle log
    /// (module comment): a human attacker's preview; `None` for an AI attacker or when the plan
    /// cannot be made.
    /// Its summons are in `per` with the ids the real fight gives them, when it is made before
    /// them ([`Forecast::frozen`] names them by log index).
    pub(crate) fn engage_forecast_frozen(&self, p: PlayerId, from: PlotIdx, to: PlotIdx, formation: &[Place]) -> Option<Forecast> {
        if !self.players[p].human {
            return None;
        }
        let inp = PreviewInput { player: p, from, to, formation: Some(formation), war: WarTest::AtWar };
        let plan = self.engage_preview_plan(&inp).ok()?;
        let runs = self.d.defines.engagement_preview_runs.max(1);
        let g = plan.scratch.as_deref().unwrap_or(self);
        Some(g.engage_forecast_sampled(&plan.setup, &plan.view, runs))
    }

    /// The forecast of a battle that is not on the board yet (the AI's estimator,
    /// docs/CALIBRATION.md, "The battle forecast"): `attackers` (one player's, wherever they
    /// stand now) attacking `target` as if they stood on `from` (a land plot next to `target`: its
    /// approach's slots, river crossing and ground count, as for an attack from there), against
    /// the defenders now on `target` plus `extra_defenders` (units elsewhere now, as if they stood
    /// on `target`), in `runs` battles. Pure and deterministic as the preview: the same set-up,
    /// the same seed (both formations, the plots, the turn), so for a real attack (the attacker's
    /// pool on `from`, no extras) it is the preview's forecast at the same `runs`.
    ///
    /// The pool is the attackers that could make the attack: one standing on `from` already as the
    /// real fight picks it ([`Game::unit_can_engage`]: moves and attacks left), any other one if
    /// it is a land unit able to attack (not marching, not a missile, not a battle summon) that
    /// could stand on `from` (land, or water it walks on; a peak it can climb) within its combat
    /// limit; its moves are not counted (it is assumed to arrive). The defenders are those the
    /// attack would meet there (the real fight's candidates: invisible units are never one), the
    /// extras tested the same way; an extra defender keeps its own state (damage, fortification).
    ///
    /// `sight`: [`ForecastSight::Attacker`] refuses, as the preview does, a target the
    /// attacker's team does not see (the forecast of an attack must not read a garrison its
    /// planner cannot see); [`ForecastSight::Defender`] does not test it (the defender's own
    /// forecast of an attack on its plot: it knows its garrison). `Err`: the attackers are not
    /// one player's, `from` is not next to `target`, `target` is water, no attacker could attack,
    /// nobody to fight, or (`Attacker`) the target unseen.
    pub fn engage_forecast_if(&self, attackers: &[UnitId], from: PlotIdx, target: PlotIdx, extra_defenders: &[UnitId], runs: i32, sight: ForecastSight, when: ForecastTurn) -> Result<Forecast, &'static str> {
        let p = self.engage_forecast_check(attackers.iter().copied(), &[from], target, sight)?;
        let pool = self.engage_forecast_pool(attackers, from, target, when, WarTest::AtWar);
        let ctx = HitCtx::new(from, target);
        let slots = self.engage_slots(&ctx);
        let auto = self.engage_auto_formation(&pool, Side::Attacker, &slots, &ctx);
        if auto.is_empty() {
            return Err("no unit can attack there");
        }
        let force: Vec<UnitId> = auto.iter().map(|q| q.unit).collect();
        let view = self.engage_forecast_view(p, target, &force, extra_defenders, WarTest::AtWar);
        let plan = self.engage_plan_finish(ctx, slots, pool, auto.clone(), auto, view)?;
        let g = plan.scratch.as_deref().unwrap_or(self);
        let mut forecast = g.engage_forecast_sampled(&plan.setup, &plan.view, runs.max(1));
        forecast.per.retain(|x| !g.units[x.unit].battle_summon);
        Ok(forecast)
    }

    /// The checks of a hypothetical forecast (`engage_forecast_if`'s `Err`s): the attacking
    /// player.
    fn engage_forecast_check(&self, mut attackers: impl Iterator<Item = UnitId>, froms: &[PlotIdx], target: PlotIdx, sight: ForecastSight) -> Result<PlayerId, &'static str> {
        let Some(first) = attackers.next() else { return Err("no attackers") };
        if !self.units.contains(first) {
            return Err("an attacker is gone");
        }
        let p = self.units[first].owner;
        for u in attackers {
            if !self.units.contains(u) {
                return Err("an attacker is gone");
            }
            if self.units[u].owner != p {
                return Err("attackers of several players");
            }
        }
        let (tx, ty) = self.map.xy(target);
        for &from in froms {
            let (fx, fy) = self.map.xy(from);
            if self.step_distance(fx, fy, tx, ty) != 1 {
                return Err("the attack does not come from a plot next to the target");
            }
        }
        if self.plot_is_water(target) {
            return Err("no engagement on water");
        }
        if sight == ForecastSight::Attacker && !self.plot_is_visible(target, self.players[p].team) {
            return Err("you cannot see that plot");
        }
        Ok(p)
    }

    /// The pool of a hypothetical attack (`engage_forecast_if`): those of `attackers` on `from` in
    /// the plot's order that can attack (now: as the real fight picks them; on their next turn:
    /// as [`Game::engage_could_attack_from_war`], moves and attacks restored), then the others that
    /// could.
    fn engage_forecast_pool(&self, attackers: &[UnitId], from: PlotIdx, target: PlotIdx, when: ForecastTurn, war: WarTest) -> Vec<UnitId> {
        let can = |u: UnitId| match when {
            ForecastTurn::Now => self.unit_can_engage(u, target, war),
            ForecastTurn::Next => self.engage_could_attack_from_war(u, from, target, war),
        };
        let mut pool: Vec<UnitId> = self.map.plots[from].units.iter().copied().filter(|&u| attackers.contains(&u) && can(u)).collect();
        for &u in attackers {
            if !pool.contains(&u) && self.unit_plot_opt(u) != Some(from) && self.engage_could_attack_from_war(u, from, target, war) {
                pool.push(u);
            }
        }
        pool
    }

    /// The defense a hypothetical attack meets (`engage_forecast_if`): the real fight's view, and
    /// the extra defenders it would meet as if they stood on `target`.
    fn engage_forecast_view(&self, p: PlayerId, target: PlotIdx, force: &[UnitId], extra_defenders: &[UnitId], war: WarTest) -> DefView {
        let team = self.players[p].team;
        let mut view = self.engage_def_view(p, target, force, war);
        let declared = self.engage_declared(team, war);
        for &d in extra_defenders {
            if !self.units.contains(d) || view.candidates.contains(&d) || self.unit_plot_opt(d) == Some(target) {
                continue;
            }
            if self.engage_meets(d, target, team, force, declared.as_ref()) && self.engage_can_defend(d, target) {
                view.candidates.push(d);
                view.on_plot.push(d);
            }
        }
        view
    }

    /// The forecast of the battles that follow one another on `target` in a turn
    /// ([`Game::engage_forecast_if`] for each, the state carried from one to the next): each
    /// entry of `attacks` is units attacking as if from a plot next to `target`; its pool attacks
    /// in its auto formation, then the units of the pool not in it (the reserve, which has not
    /// attacked) in theirs, and so on, every unit once, at most [`FORECAST_MAX_WAVES`] battles in
    /// all; then the next entry. In each run the battles are fought in that order until the plot
    /// falls: the defenders that fell or fled are gone, the survivors keep their damage (and the
    /// strikers used up stay used up), the city's Sentinels their HP; each battle meets the
    /// defenders left (the defender's formation of them as the rules form it from the game's
    /// state: the damage of earlier battles of the run does not reorder it).
    ///
    /// The result: `falls_permille` the chance the plot falls by the last battle, `wiped_permille`
    /// that it falls or the last battle fought wiped out the defender's line, `repulsed_permille`
    /// that it does not fall and the last battle fought wiped out the attacker's; losses over all
    /// the battles; `per` every unit committed in some run (attackers by battle, then the
    /// defenders), `sentinel_fall_pct` empty. With one battle (a pool that fits one formation)
    /// the battle is `engage_forecast_if`'s, sampled on a seed of its own. `Err`: as
    /// `engage_forecast_if` (for the first battle).
    pub fn engage_forecast_waves(&self, attacks: &[(Vec<UnitId>, PlotIdx)], target: PlotIdx, extra_defenders: &[UnitId], runs: i32, sight: ForecastSight, when: ForecastTurn) -> Result<Forecast, &'static str> {
        self.engage_forecast_waves_war(attacks, target, extra_defenders, runs, sight, when, WarTest::AtWar)
    }

    /// [`Game::engage_forecast_waves`] under `war`: with [`WarTest::DeclareWar`], the battles as
    /// they would be fought once that war is declared (the AI's estimate of an attack it has not
    /// declared yet: an invasion it weighs before choosing the war).
    pub fn engage_forecast_waves_war(&self, attacks: &[(Vec<UnitId>, PlotIdx)], target: PlotIdx, extra_defenders: &[UnitId], runs: i32, sight: ForecastSight, when: ForecastTurn, war: WarTest) -> Result<Forecast, &'static str> {
        let froms: Vec<PlotIdx> = attacks.iter().map(|a| a.1).collect();
        let p = self.engage_forecast_check(attacks.iter().flat_map(|a| a.0.iter().copied()), &froms, target, sight)?;
        // The battles: each attack's pool in formations, every unit once.
        struct Wave {
            ctx: HitCtx,
            slots: Slots,
            pool: Vec<UnitId>,
            formation: Vec<Place>,
        }
        let mut waves: Vec<Wave> = Vec::new();
        let mut used: Vec<UnitId> = Vec::new();
        for (units, from) in attacks {
            let ctx = HitCtx::new(*from, target);
            let slots = self.engage_slots(&ctx);
            let mut left: Vec<UnitId> = self.engage_forecast_pool(units, *from, target, when, war).into_iter().filter(|u| !used.contains(u)).collect();
            while !left.is_empty() && waves.len() < FORECAST_MAX_WAVES {
                let formation = self.engage_auto_formation(&left, Side::Attacker, &slots, &ctx);
                if formation.is_empty() {
                    break;
                }
                let pool = left.clone();
                left.retain(|u| !formation.iter().any(|q| q.unit == *u));
                used.extend(formation.iter().map(|q| q.unit));
                waves.push(Wave { ctx, slots: slots.clone(), pool, formation });
            }
        }
        if waves.is_empty() {
            return Err("no unit can attack there");
        }
        let force: Vec<UnitId> = waves[0].formation.iter().map(|q| q.unit).collect();
        let view0 = self.engage_forecast_view(p, target, &force, extra_defenders, war);
        // The plan of battle `w` against the defenders `alive` (in `view0`'s order); `None`:
        // nobody to fight.
        let plan_of = |w: usize, alive: &[UnitId], gone: &[UnitId]| -> Option<(Plan, Vec<BattleEffect>)> {
            let wave = &waves[w];
            let view = DefView { candidates: alive.to_vec(), on_plot: view0.on_plot.iter().copied().filter(|u| !gone.contains(u)).collect(), declared: view0.declared.clone() };
            let plan = self.engage_plan_finish(wave.ctx, wave.slots.clone(), wave.pool.clone(), wave.formation.clone(), wave.formation.clone(), view).ok()?;
            let pre = plan.scratch.as_deref().unwrap_or(self).engage_pre_battle(&plan.setup);
            Some((plan, pre))
        };
        let first = plan_of(0, &view0.candidates, &[]).ok_or("there is nobody to fight there")?;
        let seed = seed_of(&[self.engage_forecast_seed(&first.0.setup), waves.len() as u32, 0x5741_5645]);
        let mut plans: std::collections::BTreeMap<(usize, Vec<UnitId>), Option<(Plan, Vec<BattleEffect>)>> = std::collections::BTreeMap::new();
        plans.insert((0, view0.candidates.clone()), Some(first));
        let mut rng = Rng::new(seed);
        let runs = runs.max(1);
        // Per unit: (side, fell, escaped, committed in some run).
        let mut stat: std::collections::BTreeMap<UnitId, (Side, i64, i64)> = std::collections::BTreeMap::new();
        let (mut wiped, mut falls, mut repulsed) = (0i64, 0i64, 0i64);
        for _ in 0..runs {
            let mut ov = UnitOverlay::with_declared(view0.declared.as_ref());
            let mut alive: Vec<UnitId> = view0.candidates.clone();
            let mut gone: Vec<UnitId> = Vec::new();
            let mut walls: std::collections::BTreeMap<usize, i32> = std::collections::BTreeMap::new();
            let mut last = [false, false];
            let mut fell_plot = false;
            for w in 0..waves.len() {
                let key = (w, alive.clone());
                if !plans.contains_key(&key) {
                    let made = plan_of(w, &alive, &gone);
                    plans.insert(key.clone(), made);
                }
                let Some((plan, pre)) = plans[&key].as_ref() else { break };
                let g = plan.scratch.as_deref().unwrap_or(self);
                let s = &plan.setup;
                let n = s.units.len();
                let wall_hp = |i: usize| -> i32 {
                    let j = i - n;
                    match s.sentinels.get(j) {
                        Some(k) => walls.get(k).copied().unwrap_or(s.fighters[i].hp),
                        None => s.fighters[i].hp,
                    }
                };
                let b = g.engage_sample_battle(s, &plan.view, pre, &mut ov, Some(&wall_hp), &mut rng);
                for i in 0..n {
                    let u = s.units[i];
                    if g.units[u].battle_summon || b.fate[i] == SampleFate::Off {
                        continue;
                    }
                    let side = s.sides[i];
                    let e = stat.entry(u).or_insert((side, 0, 0));
                    let leaves = match b.fate[i] {
                        SampleFate::Fell => {
                            e.1 += 1;
                            true
                        }
                        SampleFate::Withdrew => {
                            e.2 += 1;
                            !s.in_city
                        }
                        _ => false,
                    };
                    if side == Side::Defender {
                        if leaves {
                            alive.retain(|&x| x != u);
                            gone.push(u);
                        } else {
                            ov.set_damage(u, damage_from_hp(b.end_hp[i], s.fighters[i].max_hp));
                        }
                    }
                }
                for (j, &k) in s.sentinels.iter().enumerate() {
                    walls.insert(k, b.end_hp[n + j]);
                }
                last = b.down;
                if alive.is_empty() && b.down[1] {
                    fell_plot = true;
                    break;
                }
            }
            if fell_plot {
                falls += 1;
            }
            if fell_plot || last[1] {
                wiped += 1;
            }
            if !fell_plot && last[0] {
                repulsed += 1;
            }
        }
        let r = runs as i64;
        let per_mille = |k: i64| (k * 1000 / r) as i32;
        let side_sum = |side: Side, k: usize| -> i32 { (stat.values().filter(|e| e.0 == side).map(|e| if k == 1 { e.1 } else { e.2 }).sum::<i64>() * 100 / r) as i32 };
        let order: Vec<UnitId> = waves.iter().flat_map(|w| w.formation.iter().map(|q| q.unit)).chain(view0.candidates.iter().copied()).collect();
        let per = order
            .into_iter()
            .filter_map(|u| stat.get(&u).map(|e| PreviewFighter { unit: u, side: e.0, fall_pct: (e.1 * 100 / r) as i32, escape_pct: (e.2 * 100 / r) as i32 }))
            .collect();
        Ok(Forecast {
            runs,
            wiped_permille: per_mille(wiped),
            falls_permille: per_mille(falls),
            repulsed_permille: per_mille(repulsed),
            lost_x100: [side_sum(Side::Attacker, 1), side_sum(Side::Defender, 1)],
            escaped_x100: [side_sum(Side::Attacker, 2), side_sum(Side::Defender, 2)],
            per,
            sentinel_fall_pct: Vec::new(),
        })
    }
}

/// At most this many battles in a forecast of battles that follow one another
/// ([`Game::engage_forecast_waves`]).
pub const FORECAST_MAX_WAVES: usize = 8;

/// When a hypothetical attack ([`Game::engage_forecast_if`]) is made: the attackers that stand on
/// the plot it comes from attack with what they have left this turn, or on their next turn.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ForecastTurn {
    /// Now: a unit on `from` attacks only if it can now (moves and an attack left), as the real
    /// fight picks it.
    Now,
    /// On the attacker's next turn: its moves and attacks restored (the defender's forecast of
    /// an enemy stack, made during the defender's turn).
    Next,
}

/// Whose sight a hypothetical forecast ([`Game::engage_forecast_if`]) respects.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ForecastSight {
    /// The attacker's, as the preview: a target its team does not see is refused.
    Attacker,
    /// The defender's own forecast of an attack on its plot: no sight test.
    Defender,
}
