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
use super::engage::{hp_from_damage, Setup};
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
        // Card: the field's Sentinels take cells of the defender's grid (more_cards.rs).
        let mut slots = slots;
        self.engage_add_field_sentinels(&mut slots, &view.candidates, to);
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

    /// `runs` battles of `setup` (the real fight's morale check, opening and battle) on a private
    /// RNG seeded from the set-up, the opening over the units `view.on_plot` (what the attacker
    /// sees there) at war as `view` has it.
    pub(crate) fn engage_forecast_sampled(&self, setup: &Setup, view: &DefView, runs: i32) -> Forecast {
        let n = setup.units.len();
        let n_all = setup.fighters.len();
        let n_att = setup.n_att;
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
            let mut sat = setup.initial_sat_out();
            self.engage_morale_rolls(&setup.units, n_att, &mut sat, &mut rng);
            let mut ov = fresh.clone();
            self.engage_opening_rolls(&setup.units, n_att, &sat, &view.on_plot, &mut ov, &mut rng);
            let mut hp: Vec<i32> = (0..n_all).map(|i| if i < n { hp_from_damage(ov.damage(self, setup.units[i]), setup.fighters[i].max_hp) } else { setup.fighters[i].hp }).collect();
            let mut effects = pre.clone();
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
            let out = formation::resolve_quiet(&f, &mut rng);
            // Card: an illusion (past the set-up's fighters) is not counted.
            for (k, e) in out.fighters.iter().enumerate().filter(|&(k, _)| map[k] < n_all) {
                match e.fate {
                    Fate::Fell { .. } => fell[map[k]] += 1,
                    Fate::Withdrew { .. } => escaped[map[k]] += 1,
                    Fate::Standing => {}
                }
            }
            if out.down[1] {
                wiped += 1;
                let sat_def = setup.of(Side::Defender).any(|i| sat[i]);
                if !reserve && !sat_def {
                    falls += 1;
                }
            }
            if out.down[0] {
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
}
