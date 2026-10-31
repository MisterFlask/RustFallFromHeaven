//! The attack preview (plan 15 item 2; the battle UI design's plan screen, DESIGN.md sections 2
//! and 10.6): what the player sees before an attack. Pure (`&self`): it plays the battle many
//! times on a private RNG and never touches the game's.
//!
//! **The plan** ([`Game::engage_preview`]): the attacker's pool on the plot it attacks from, its
//! slots, the force (the one given, checked, or the stance's proposal), the stance's proposal
//! (`auto_pick`) and the reserve; the defenders the attacker's team sees, their stance if known
//! (its own team's; anyone else's is unseen until phase 7's recon reveal) and the pick it assumes
//! (the known stance's, else the default's); when the candidates exceed the slots and the stance
//! is unseen, which units every stance commits ("sure") and which only some ("maybe").
//!
//! **The forecast**: `ENGAGEMENT_PREVIEW_RUNS` battles of the real fight's own code on that set-up
//! ([`Game::engage_setup`]): the morale check and the opening's defensive strikes and collateral
//! as pure rolls ([`Game::engage_morale_rolls`], [`Game::engage_opening_rolls`]), then the round
//! model ([`super::resolve_sampled_traced`]), all on an [`Rng`] seeded from the plan (the force,
//! the plots, the turn, the defender's assumed stance and pick), so the same plan always shows the
//! same numbers. A move that declares war previews the battle after the declaration: the
//! opening's rolls test war as that fight does (`DefView::declared`), so the plan is the one the
//! attack freezes once the war is declared. It gives the win and draw chances, the rounds'
//! p10/p50/p90, the cost of a win,
//! each committed fighter's chance to fall and to withdraw and the round by which it has fallen in
//! half the battles, and each side's mean HP after the opening and each round.
//!
//! **What if** ([`Game::engage_preview_alternatives`], a second call so the plan never waits):
//! the stance's own pick (its quotas before the swap search), the best single swaps of the force
//! (all of them ranked by the moment estimator, the best few of a different make-up sampled) and
//! the other stances' proposals (what choosing that stance proposes: its pick and the swap
//! search), each forecast like the plan (the same force: the same forecast).
//!
//! **Frozen** ([`Game::engage_forecast_frozen`]): when an attack whose battle gets a log is made,
//! its plan is frozen into the log (`report::FrozenForecast`): a human attacker's preview with its
//! What-if list (the numbers it saw, recomputed: the same seed), an AI attacker's moment estimate
//! (`ForecastSource::Expected`).

use ffh_data::*;

use super::battle::{FighterIdx, Side};
use super::engage::Setup;
use super::estimate::{DefView, Estimate, PairCache};
use super::expect::ONE;
use super::pick::WarTest;
use super::report::{AltKind, ForecastAlt, ForecastFighter, ForecastSource, FrozenForecast};
use super::resolve::{resolve_sampled_traced, EngageResult, Fate, Phase};
use super::slots::Slots;
use super::stats::HitCtx;
use crate::game::Game;
use crate::ids::*;
use crate::rng::Rng;
use crate::unit::UnitOverlay;

/// The best single swaps the What-if list samples (of a different make-up).
const ALT_SWAPS: usize = 3;

/// What a preview is of.
#[derive(Copy, Clone, Debug)]
pub struct PreviewInput<'a> {
    pub player: PlayerId,
    pub from: PlotIdx,
    pub to: PlotIdx,
    /// The group that would attack (its army's stance is the default stance).
    pub launcher: Option<GroupId>,
    /// The force (`None`: the stance's proposal).
    pub force: Option<&'a [UnitId]>,
    /// The stance that proposes the force (`None`: the attack's, [`Game::engage_attack_stance`]).
    pub stance: Option<StanceType>,
    /// The war the attack assumes ([`WarTest::DeclareWar`] for a move that declares it).
    pub war: WarTest,
}

impl<'a> PreviewInput<'a> {
    /// `player`'s attack from `from` on `to` at war, the proposal under the attack's stance.
    pub fn new(player: PlayerId, from: PlotIdx, to: PlotIdx) -> PreviewInput<'a> {
        PreviewInput { player, from, to, launcher: None, force: None, stance: None, war: WarTest::AtWar }
    }
}

/// The defender's pick as the attacker knows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefPick {
    /// Every unit that fights: the stance is known, or every candidate fits the slots.
    Known(Vec<UnitId>),
    /// The stance is unseen and the candidates exceed the slots: `sure` are committed under every
    /// stance, `maybe` under some.
    Likely { sure: Vec<UnitId>, maybe: Vec<UnitId> },
}

/// A fighter of the forecast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewFighter {
    pub unit: UnitId,
    pub side: Side,
    /// The chance it falls, and withdraws (percent).
    pub fall_pct: i32,
    pub withdraw_pct: i32,
    /// The round by which it has fallen in half the battles (0: the opening volleys; `None`: it
    /// falls in fewer than half).
    pub death_round: Option<i32>,
}

/// The sampled forecast of a plan (the battles of the real resolver on the preview's RNG).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forecast {
    pub runs: i32,
    /// The attacker's chance to win and to draw (per mille).
    pub win_permille: i32,
    pub draw_permille: i32,
    /// The rounds fought (volley rounds not counted): p10, p50, p90.
    pub rounds: (i32, i32, i32),
    /// The attacker's committed units lost in the battles it won, x10 (`None`: it won none).
    pub losses_if_win_x10: Option<i32>,
    /// Each committed fighter, in the round model's order (the force, then the defender's pick).
    pub per: Vec<PreviewFighter>,
    /// Each side's mean engaged HP (centi-HP) after the opening (volleys included) and each
    /// round; a battle that ended keeps its last value.
    pub curve: [Vec<i32>; 2],
}

/// A What-if plan: its kind, the stance whose pick it is, its force and its sampled numbers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alternative {
    pub kind: AltKind,
    pub stance: StanceType,
    pub force: Vec<UnitId>,
    pub win_permille: i32,
    pub losses_if_win_x10: Option<i32>,
}

/// The attack preview (module comment).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngagePreview {
    pub slots: Slots,
    /// The attacker's pool, the force, the stance's proposal and the pool's units not in the force.
    pub pool: Vec<UnitId>,
    pub force: Vec<UnitId>,
    pub auto_pick: Vec<UnitId>,
    pub reserve: Vec<UnitId>,
    /// The stance that proposed it.
    pub stance: StanceType,
    /// The defender's candidates the attacker's team sees.
    pub defenders: Vec<UnitId>,
    /// The defender's stance when the attacker knows it (`None`: unseen), and the one assumed.
    pub def_stance: Option<StanceType>,
    pub def_stance_assumed: StanceType,
    /// The defender's pick: the forecast's (`assumed_pick`) and as the attacker knows it.
    pub assumed_pick: Vec<UnitId>,
    pub defender_pick: DefPick,
    pub forecast: Forecast,
    /// The team an attack would declare war on.
    pub declare_war: Option<TeamId>,
}

/// The preview's set-up (shared by the plan and its What-if list).
struct Plan {
    ctx: HitCtx,
    slots: Slots,
    pool: Vec<UnitId>,
    stance: StanceType,
    auto_pick: Vec<UnitId>,
    view: DefView,
    setup: Setup,
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

/// The nearest-rank `pct` percentile of sorted `v`.
fn percentile(v: &[i32], pct: usize) -> i32 {
    if v.is_empty() {
        return 0;
    }
    let rank = (v.len() * pct).div_ceil(100).max(1);
    v[rank - 1]
}

impl Game {
    /// The attack preview of `inp` (module comment). `Err`: the reason the attack is refused, as
    /// the real fight gives it (an invalid force, no unit that can attack, nobody there), or the
    /// plot is not visible to the attacker's team.
    pub fn engage_preview(&self, inp: &PreviewInput) -> Result<EngagePreview, &'static str> {
        let plan = self.engage_preview_plan(inp)?;
        let runs = self.d.defines.engagement_preview_runs.max(1);
        let forecast = self.engage_forecast_sampled(&plan.setup, &plan.view, runs);
        let team = self.players[inp.player].team;
        let known = self.engage_stance_known(team, &plan.view.candidates);
        let def_stance = known.then_some(plan.view.stance);
        let defender_pick = self.engage_defender_pick_known(&plan, known);
        let reserve = plan.pool.iter().copied().filter(|u| !plan.setup.force.contains(u)).collect();
        let declare_war = match inp.war {
            WarTest::DeclareWar(t) => Some(t),
            WarTest::AtWar => None,
        };
        Ok(EngagePreview {
            slots: plan.slots,
            pool: plan.pool,
            force: plan.setup.force.clone(),
            auto_pick: plan.auto_pick,
            reserve,
            stance: plan.stance,
            defenders: plan.view.candidates.clone(),
            def_stance,
            def_stance_assumed: plan.view.stance,
            assumed_pick: plan.setup.def_pick.clone(),
            defender_pick,
            forecast,
            declare_war,
        })
    }

    /// The What-if list of `inp`'s plan (module comment): the stance's own pick, the best single
    /// swaps of the plan's force, the other stances' proposals, each with its sampled win chance
    /// and cost (as many runs as the plan's).
    pub fn engage_preview_alternatives(&self, inp: &PreviewInput) -> Result<Vec<Alternative>, &'static str> {
        let plan = self.engage_preview_plan(inp)?;
        Ok(self.engage_alternatives(&plan))
    }

    /// Whether `team` knows the stance of the defenders `candidates` (phase 6: its own team's
    /// only; phase 7's recon reveal widens it).
    pub fn engage_stance_known(&self, team: TeamId, candidates: &[UnitId]) -> bool {
        candidates.first().is_some_and(|&u| self.unit_get_team(u) == team)
    }

    fn engage_preview_plan(&self, inp: &PreviewInput) -> Result<Plan, &'static str> {
        let (p, from, to) = (inp.player, inp.from, inp.to);
        let ctx = HitCtx::new(from, to);
        let slots = self.engage_slots(&ctx);
        let pool = self.engage_attackers(p, from, to, inp.war);
        let stance = inp.stance.unwrap_or_else(|| self.engage_attack_stance(p, from, inp.launcher).stance);
        let auto_pick = self.engage_propose_force_for(&pool, stance, &slots, &ctx, inp.war);
        let force = match inp.force {
            Some(f) => {
                self.engage_validate_force(&pool, f, &slots)?;
                f.to_vec()
            }
            None => auto_pick.clone(),
        };
        if force.is_empty() {
            return Err("no unit can attack there");
        }
        if !self.plot_is_visible(to, self.players[p].team) {
            return Err("you cannot see that plot");
        }
        let view = self.engage_def_view(p, to, &force, inp.war);
        let setup = self.engage_setup(ctx, slots.clone(), force, view.candidates.clone(), view.stance, &[])?;
        Ok(Plan { ctx, slots, pool, stance, auto_pick, view, setup })
    }

    /// The defender's pick as the attacker knows it (module comment).
    fn engage_defender_pick_known(&self, plan: &Plan, known: bool) -> DefPick {
        let cands = &plan.view.candidates;
        if known || cands.len() as i32 <= plan.slots.def() {
            return DefPick::Known(plan.setup.def_pick.clone());
        }
        let picks: Vec<Vec<UnitId>> = self.d.stances.keys().map(|s| self.engage_auto_pick(cands, s, &plan.slots, Side::Defender, &plan.ctx)).collect();
        let sure: Vec<UnitId> = plan.setup.def_pick.iter().copied().filter(|u| picks.iter().all(|p| p.contains(u))).collect();
        let maybe: Vec<UnitId> = cands.iter().copied().filter(|u| !sure.contains(u) && picks.iter().any(|p| p.contains(u))).collect();
        DefPick::Likely { sure, maybe }
    }

    /// The seed of a set-up's forecast: its force (sorted), the plots, the turn, the defender's
    /// assumed stance and pick (sorted). The attacker's stance is not in it: the same force is
    /// the same battle.
    fn engage_forecast_seed(&self, setup: &Setup, def_stance: StanceType) -> u32 {
        let mut words: Vec<u32> = Vec::with_capacity(setup.units.len() + 6);
        let mut force: Vec<u32> = setup.force.iter().map(|u| u.0).collect();
        force.sort_unstable();
        let mut defs: Vec<u32> = setup.def_pick.iter().map(|u| u.0).collect();
        defs.sort_unstable();
        words.push(0x5052_4556); // "PREV"
        words.extend(force);
        words.push(u32::MAX);
        words.extend(defs);
        words.push(setup.ctx.from.map_or(u32::MAX, |p| p.0));
        words.push(setup.ctx.to.0);
        words.push(self.game_turn() as u32);
        words.push(def_stance.idx() as u32);
        seed_of(&words)
    }

    /// `runs` battles of `setup` (the real fight's morale check, opening and rounds) on a private
    /// RNG seeded from the set-up, the opening over the units `view.on_plot` (what the attacker
    /// sees there) at war as `view` has it (a declare-war plan's: once declared, so its forecast is
    /// the one the attack after the declaration freezes).
    pub(crate) fn engage_forecast_sampled(&self, setup: &Setup, view: &DefView, runs: i32) -> Forecast {
        let n = setup.units.len();
        let n_att = setup.n_att;
        let on_plot = &view.on_plot[..];
        let mut rng = Rng::new(self.engage_forecast_seed(setup, setup.def_stance));
        let pre = self.engage_pre_battle(&setup.battle);
        let fresh = UnitOverlay::with_declared(view.declared.as_ref());
        let mut battle = setup.battle.clone();
        let (mut won, mut drawn) = (0i32, 0i32);
        let mut win_losses = 0i64;
        let mut rounds: Vec<i32> = Vec::with_capacity(runs as usize);
        let mut fell = vec![0i32; n];
        let mut withdrew = vec![0i32; n];
        let mut death: Vec<Vec<i32>> = vec![Vec::new(); n];
        let mut curves: Vec<Vec<[i32; 2]>> = Vec::with_capacity(runs as usize);
        for _ in 0..runs {
            let mut sat = vec![false; n];
            self.engage_morale_rolls(&setup.units, n_att, &mut sat, &mut rng);
            let mut ov = fresh.clone();
            self.engage_opening_rolls(&setup.units, n_att, &sat, setup.ctx.to, on_plot, &mut ov, &mut rng);
            for (i, &u) in setup.units.iter().enumerate() {
                battle.set_start(i, ov.damage(self, u), sat[i]);
            }
            let mut effects = pre.clone();
            effects.extend(self.engage_opening_spells(&battle));
            let res = if effects.is_empty() {
                resolve_sampled_traced(&battle, &mut rng)
            } else {
                let mut b = battle.clone();
                b.add_effects(effects);
                resolve_sampled_traced(&b, &mut rng)
            };
            match res.result {
                EngageResult::AttackerWon => {
                    won += 1;
                    win_losses += res.fighters[..n_att].iter().filter(|f| matches!(f.fate, Fate::Killed { .. })).count() as i64;
                }
                EngageResult::Draw => drawn += 1,
                _ => {}
            }
            rounds.push(res.rounds);
            for (i, f) in res.fighters.iter().enumerate() {
                match f.fate {
                    Fate::Killed { phase, .. } => {
                        fell[i] += 1;
                        death[i].push(match phase {
                            Phase::Round(r) => r,
                            _ => 0,
                        });
                    }
                    Fate::Withdrew { .. } => withdrew[i] += 1,
                    _ => {}
                }
            }
            curves.push(res.curve);
        }
        rounds.sort_unstable();
        let pct = |k: i32| (k as i64 * 100 / runs as i64) as i32;
        let per = (0..n)
            .map(|i| {
                let mut d = std::mem::take(&mut death[i]);
                d.sort_unstable();
                // The round by which half the battles have it dead.
                let half = (runs as usize).div_ceil(2);
                PreviewFighter {
                    unit: setup.units[i],
                    side: setup.sides[i],
                    fall_pct: pct(fell[i]),
                    withdraw_pct: pct(withdrew[i]),
                    death_round: (d.len() >= half && half > 0).then(|| d[half - 1]),
                }
            })
            .collect();
        let len = curves.iter().map(|c| c.len()).max().unwrap_or(0);
        let curve = [0usize, 1].map(|s| {
            (0..len)
                .map(|k| {
                    let sum: i64 = curves.iter().filter(|c| !c.is_empty()).map(|c| c[k.min(c.len() - 1)][s] as i64).sum();
                    (sum / runs as i64) as i32
                })
                .collect()
        });
        Forecast {
            runs,
            win_permille: (won as i64 * 1000 / runs as i64) as i32,
            draw_permille: (drawn as i64 * 1000 / runs as i64) as i32,
            rounds: (percentile(&rounds, 10), percentile(&rounds, 50), percentile(&rounds, 90)),
            losses_if_win_x10: (won > 0).then(|| (win_losses * 10 / won as i64) as i32),
            per,
            curve,
        }
    }

    /// The What-if list of `plan` (module comment).
    fn engage_alternatives(&self, plan: &Plan) -> Vec<Alternative> {
        let runs = self.d.defines.engagement_preview_runs.max(1);
        let in_city = plan.setup.in_city;
        let cache = PairCache::new(self, &plan.pool, &plan.view.candidates, &plan.ctx, in_city);
        let mut done: Vec<(Vec<UnitId>, i32, Option<i32>)> = Vec::new();
        let mut sampled = |force: &[UnitId]| -> Option<(i32, Option<i32>)> {
            let mut key = force.to_vec();
            key.sort_unstable();
            if let Some(d) = done.iter().find(|d| d.0 == key) {
                return Some((d.1, d.2));
            }
            let setup = self.engage_setup_cached(plan.ctx, plan.slots.clone(), force.to_vec(), plan.view.candidates.clone(), plan.view.stance, &[], Some(&cache)).ok()?;
            let f = self.engage_forecast_sampled(&setup, &plan.view, runs);
            done.push((key, f.win_permille, f.losses_if_win_x10));
            Some((f.win_permille, f.losses_if_win_x10))
        };
        let mut out = Vec::new();
        let mut add = |out: &mut Vec<Alternative>, kind: AltKind, stance: StanceType, force: Vec<UnitId>| {
            if let Some((win, losses)) = sampled(&force) {
                out.push(Alternative { kind, stance, force, win_permille: win, losses_if_win_x10: losses });
            }
        };
        // The stance's own pick.
        let own = self.engage_auto_pick(&plan.pool, plan.stance, &plan.slots, Side::Attacker, &plan.ctx);
        add(&mut out, AltKind::StancePick, plan.stance, own);
        // The best single swaps of the plan's force, by the moment estimator; of a different
        // make-up from the force and from each other.
        let force = &plan.setup.force;
        let kind = self.engage_swap_kinds(&plan.pool, &cache, &[]);
        let kind_of = |u: UnitId| plan.pool.iter().position(|&x| x == u).map_or(usize::MAX, |i| kind[i]);
        let mut tried: Vec<(usize, usize)> = Vec::new();
        let mut swaps: Vec<(Vec<UnitId>, Estimate)> = self
            .engage_single_swaps(&plan.pool, force, plan.stance, &plan.slots)
            .into_iter()
            .filter(|&(o, a)| {
                let k = (kind_of(o), kind_of(a));
                let new = k.0 != k.1 && !tried.contains(&k);
                tried.push(k);
                new
            })
            .filter_map(|(o, a)| {
                let f: Vec<UnitId> = force.iter().map(|&u| if u == o { a } else { u }).collect();
                let e = self.engage_estimate_with(&plan.ctx, &plan.slots, &f, &plan.view, &[], &[], Some(&cache)).ok()?;
                Some((f, e))
            })
            .collect();
        // Best first: a swap that beats another comes before it (win chance, then losses).
        swaps.sort_by(|a, b| b.1.win_permille.cmp(&a.1.win_permille).then(a.1.att_loss_x1000.cmp(&b.1.att_loss_x1000)));
        let mut kinds = vec![self.engage_make_up(force)];
        for (f, _) in swaps {
            if kinds.len() > ALT_SWAPS {
                break;
            }
            let k = self.engage_make_up(&f);
            if kinds.contains(&k) {
                continue;
            }
            kinds.push(k);
            add(&mut out, AltKind::Swap, plan.stance, f);
        }
        // The other stances' proposals: the plan choosing that stance gives (its quota pick and
        // the swap search, against the same defense).
        for s in self.d.stances.keys() {
            if s != plan.stance {
                let pick = self.engage_auto_pick(&plan.pool, s, &plan.slots, Side::Attacker, &plan.ctx);
                let proposal = self.engage_swap_search_in(&plan.pool, s, &plan.slots, &plan.ctx, pick, &plan.view, &[], Some(&cache));
                add(&mut out, AltKind::OtherStance, s, proposal);
            }
        }
        out
    }

    /// A force's make-up (the client's composition key): each unit's type, level and
    /// promotions, sorted.
    fn engage_make_up(&self, force: &[UnitId]) -> Vec<(UnitType, i32, Vec<PromotionType>)> {
        let mut v: Vec<(UnitType, i32, Vec<PromotionType>)> = force
            .iter()
            .map(|&u| {
                let unit = &self.units[u];
                (unit.unit_type, unit.level, self.d.promotions.keys().filter(|&p| unit.has_promotion[p]).collect())
            })
            .collect();
        v.sort();
        v
    }

    /// The plan of `p`'s attack from `from` on `to` with `force` under `stance`, frozen for its
    /// battle log (module comment; plan 15 items 1-2): a human's preview with its What-if list,
    /// an AI's moment estimate. Each unit is named by its log index (`index_of`); `None` when the
    /// plan cannot be made.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn engage_forecast_frozen(
        &self,
        p: PlayerId,
        from: PlotIdx,
        to: PlotIdx,
        launcher: Option<GroupId>,
        force: &[UnitId],
        stance: StanceType,
        index_of: impl Fn(UnitId) -> Option<FighterIdx>,
    ) -> Option<FrozenForecast> {
        let idx = |units: &[UnitId]| -> Vec<FighterIdx> { units.iter().filter_map(|&u| index_of(u)).collect() };
        if self.players[p].human {
            let inp = PreviewInput { player: p, from, to, launcher, force: Some(force), stance: Some(stance), war: WarTest::AtWar };
            let plan = self.engage_preview_plan(&inp).ok()?;
            let runs = self.d.defines.engagement_preview_runs.max(1);
            let f = self.engage_forecast_sampled(&plan.setup, &plan.view, runs);
            let alternatives = self
                .engage_alternatives(&plan)
                .into_iter()
                .map(|a| ForecastAlt { kind: a.kind, stance: a.stance, force: idx(&a.force), win_permille: a.win_permille, losses_if_win_x10: a.losses_if_win_x10 })
                .collect();
            Some(FrozenForecast {
                source: ForecastSource::Sampled { runs },
                win_permille: f.win_permille,
                draw_permille: f.draw_permille,
                rounds: f.rounds,
                losses_if_win_x10: f.losses_if_win_x10,
                per: f.per.iter().filter_map(|x| Some(ForecastFighter { b: index_of(x.unit)?, fall_pct: x.fall_pct, withdraw_pct: x.withdraw_pct, death_round: x.death_round })).collect(),
                curve: f.curve,
                alternatives,
            })
        } else {
            let ctx = HitCtx::new(from, to);
            let slots = self.engage_slots(&ctx);
            let view = self.engage_def_view(p, to, force, WarTest::AtWar);
            let e = self.engage_estimate_with(&ctx, &slots, force, &view, &[], &[], None).ok()?;
            let pct = |w: i32| (w as i64 * 100 / ONE) as i32;
            Some(FrozenForecast {
                source: ForecastSource::Expected,
                win_permille: e.win_permille,
                draw_permille: 0,
                rounds: (e.rounds, e.rounds, e.rounds),
                losses_if_win_x10: None,
                per: e.units().zip(&e.exp.fighters).filter_map(|(u, x)| Some(ForecastFighter { b: index_of(u)?, fall_pct: pct(x.lost), withdraw_pct: pct(x.withdrawn), death_round: None })).collect(),
                curve: [Vec::new(), Vec::new()],
                alternatives: Vec::new(),
            })
        }
    }
}
