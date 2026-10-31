//! The estimator (plan 8.2): the expected outcome of an engagement, for the AI's attack decisions
//! and for the attacker's auto-pick (the swap search, plan 15 item 3). `&self`, no random number
//! drawn, nothing changed; the battle it integrates is built by the real fight's own set-up
//! ([`Game::engage_setup`]), so its fighters, pairs and HP are the fight's.
//!
//! **Inputs are explicit** ([`EstimateInput`]): the AI estimates fights it cannot start yet
//! (attackers not adjacent, out of moves), against the defenders it sees through its fog, after
//! softening and bombards it has only planned. So the estimator decides nothing from the true
//! plot beyond what anyone sees (the terrain, a city and its defense): the defenders are the
//! caller's list (the defender's candidates are its units that can defend there; the opening's
//! strikers and collateral targets are taken from it too), the defender's stance is the caller's
//! assumption, damage fields and the city's defense can be overridden, and the attackers are a
//! pool to pick from or a force, eligible now ([`Eligible::Now`]: `unit_can_engage`) or
//! hypothetically ([`Eligible::Hypothetical`]: able to attack the plot at all).
//!
//! **What it integrates.** The set-up's battle, then in expectation what the real fight rolls:
//! the morale check (each fighter's chance to sit out, `Fighter::fear_fail`), the opening's
//! defensive strikes (per attacker, the strikers sorted by damage: `sum P(i is the best success) x
//! E[damage_i]`, each striker available with the chance it was not used up by an earlier
//! attacker) and collateral (on the healthiest targets, today's formula on the expected damage),
//! both folded into the fighters' HP, then the rounds by the moment integrator
//! ([`super::resolve_expected_from`]) and its calibrated win table.
//!
//! **The assault** ([`Game::engage_estimate_assault`]): successive engagements, each committing
//! the next force from the attackers not used yet, the defenders carrying their expected damage
//! and alive weight from one to the next ([`MomentStart`]), the reserve its expected collateral,
//! until the expected defenders left are below one half or no attacker is left.
//!
//! **Wrappers.** [`Game::engage_estimate_plot`]: a player's real pool on the plot it attacks
//! from, the defenders its team sees there, the defender's stance as its team knows it (its own
//! team's: the real one; anyone else's: the default, until phase 7's recon reveal).

use ffh_data::*;

use super::battle::{Pair, Side};
use super::engage::Setup;
use super::expect::{resolve_expected_from, Expectation, MomentStart, ONE};
use super::pick::WarTest;
use super::slots::Slots;
use super::stats::HitCtx;
use crate::game::Game;
use crate::ids::*;
use crate::unit::UnitOverlay;

/// The attackers of an estimate.
#[derive(Copy, Clone, Debug)]
pub enum Attackers<'a> {
    /// A pool to pick from under a stance: the force the rules would propose
    /// ([`Game::engage_propose_force`]: the stance's pick, improved by the swap search), as an
    /// attack with no force given fights.
    Pick(&'a [UnitId], StanceType),
    /// The force as given (its units that are not eligible left out).
    Force(&'a [UnitId]),
}

/// Which attackers an estimate counts.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Eligible {
    /// Those that can attack the plot now ([`Game::unit_can_engage`]).
    Now,
    /// Those that could attack it once there: a land unit with a role that can attack, not a
    /// missile, the plot passable to it, and for a unit with a combat limit some defender of the
    /// caller's list below it (not from the water). Moves, attacks left and adjacency are not
    /// asked (planning).
    Hypothetical,
}

/// What an estimate is of (plan 8.2): everything explicit, nothing read from the true plot but
/// what anyone sees.
#[derive(Copy, Clone, Debug)]
pub struct EstimateInput<'a> {
    /// The attacking player.
    pub player: PlayerId,
    /// The defended plot.
    pub to: PlotIdx,
    /// The approach (river slots, amphibious A, where the attackers fight for Homeland); `None`:
    /// no river, by land, each attacker where it stands.
    pub from: Option<PlotIdx>,
    pub attackers: Attackers<'a>,
    pub eligible: Eligible,
    /// The war the test assumes ([`WarTest::DeclareWar`] for an attack that would declare it).
    pub war: WarTest,
    /// The caller's list of the units on `to` (the AI: what it sees through its fog): the
    /// defender's candidates are those that can defend there; strikers and collateral targets are
    /// taken from it too.
    pub defenders: &'a [UnitId],
    /// The stance the caller assumes for the defender.
    pub def_stance: StanceType,
    /// Damage fields in place of the units' own (planned softening, damage carried between
    /// engagements; the last entry of a unit counts).
    pub damage: &'a [(UnitId, i32)],
    /// A hypothetical defense modifier for the defended city (after a planned bombard); `None`:
    /// its current one.
    pub city_defense: Option<i32>,
}

impl<'a> EstimateInput<'a> {
    /// `player`'s attack on `to` (from `from`) with `attackers` against `defenders`, decided now
    /// and at war, the defender's stance the default, no override.
    pub fn new(g: &Game, player: PlayerId, to: PlotIdx, from: Option<PlotIdx>, attackers: Attackers<'a>, defenders: &'a [UnitId]) -> EstimateInput<'a> {
        EstimateInput { player, to, from, attackers, eligible: Eligible::Now, war: WarTest::AtWar, defenders, def_stance: g.d.default_stance, damage: &[], city_defense: None }
    }

    /// The hit context (the approach, the defended plot, the city's defense override).
    pub fn ctx(&self) -> HitCtx {
        HitCtx { to: self.to, from: self.from, city_defense: self.city_defense }
    }
}

/// The expected outcome of one engagement (plan 8.2, "Outputs").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Estimate {
    /// The attacker's chance to win the engagement (per mille, the calibrated table; a draw or a
    /// mutual end is not a win).
    pub win_permille: i32,
    /// Expected units lost and withdrawn per side, x1000 (the loser's residual counted as lost).
    pub att_loss_x1000: i32,
    pub def_loss_x1000: i32,
    pub att_withdrawn_x1000: i32,
    pub def_withdrawn_x1000: i32,
    /// Expected HP left per side (centi-HP), every outcome but death.
    pub att_hp_left: i64,
    pub def_hp_left: i64,
    /// Volley rounds and rounds integrated.
    pub volley_rounds: i32,
    pub rounds: i32,
    /// Winning takes the plot: the defender commits every candidate.
    pub plot_falls: bool,
    /// The chance the plot falls in this engagement (per mille): the win chance when
    /// `plot_falls`, else 0.
    pub falls_permille: i32,
    /// The force estimated, the defender's pick, its candidates left out.
    pub force: Vec<UnitId>,
    pub def_force: Vec<UnitId>,
    pub def_reserve: Vec<UnitId>,
    pub slots: Slots,
    /// Each fighter's HP when the rounds start (centi-HP, after the expected opening; a carried
    /// fighter's before the damage it carries), in the round model's order.
    pub start_hp: Vec<i32>,
    /// The expected damage the opening adds (percent x4096), by unit: committed attackers struck
    /// and the defenders' collateral, the reserve's included.
    pub opening: Vec<(UnitId, i64)>,
    /// The integrator's result, per fighter in the round model's order (the force, then the
    /// defender's pick).
    pub exp: Expectation,
}

impl Estimate {
    /// The committed units in the round model's order.
    pub fn units(&self) -> impl Iterator<Item = UnitId> + '_ {
        self.force.iter().chain(&self.def_force).copied()
    }

    /// Unit `u`'s expected loss (x1000; 0 when it does not fight).
    pub fn lost_x1000(&self, u: UnitId) -> i32 {
        self.units().position(|x| x == u).map_or(0, |i| ((self.exp.fighters[i].lost as i64 * 1000 + ONE / 2) / ONE) as i32)
    }
}

/// The expected outcome of an assault (plan 8.2, "Assaults"): successive engagements until the
/// plot falls or the attack stops. Shaped like the AI's `combat_sim::Sim`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssaultEstimate {
    /// Engagements until the expected defenders left fell below one half (`None`: they hold).
    pub used: Option<usize>,
    /// The first engagement's win chance (per mille; 0 when there is none).
    pub first: i32,
    /// The plot falls: the expected defenders left are below one half at the end.
    pub falls: bool,
    /// Expected defenders killed and attackers lost, x1000.
    pub kills_x1000: i32,
    pub losses_x1000: i32,
    /// Expected defenders left on the plot at the end (x1000; withdrawn ones have fled).
    pub def_left_x1000: i32,
    /// Expected HP left (centi-HP): the committed attackers' and the defenders' on the plot.
    pub att_hp_left: i64,
    pub def_hp_left: i64,
    /// Attackers able to attack, defenders at the start.
    pub n_att: usize,
    pub n_def: usize,
    /// Each unit's expected loss over the assault (x1000), committed units only, in the order
    /// first committed: the AI weighs them by what they are worth.
    pub att_lost: Vec<(UnitId, i32)>,
    pub def_lost: Vec<(UnitId, i32)>,
    /// The engagements, in order.
    pub steps: Vec<Estimate>,
}

/// Rounds of the swap search at most (each keeps the best single swap of the plan so far).
const SWAP_ROUNDS: usize = 3;
/// A plan beats another by a higher win chance by more than this (per mille), or by a win chance
/// at most `SWAP_TIE` lower and `SWAP_LOSS` (x1000 units) fewer expected losses: the estimator is
/// a model, so smaller differences are not worth a different force.
const SWAP_WIN: i32 = 5;
const SWAP_TIE: i32 = 2;
const SWAP_LOSS: i32 = 50;

/// Whether plan `b` beats plan `a` in the swap search (the constants above): the attacker's
/// proposal is the force most likely to win, and among those about as likely, the cheapest. The
/// search's result beats the stance's quota pick by this test, or is that pick.
pub fn swap_better(b: &Estimate, a: &Estimate) -> bool {
    b.win_permille > a.win_permille + SWAP_WIN || (b.win_permille >= a.win_permille - SWAP_TIE && b.att_loss_x1000 + SWAP_LOSS < a.att_loss_x1000)
}

/// The engagements an assault estimate runs at most (the AI's `military::assault` tries 12).
const MAX_ASSAULT: usize = 12;
/// A defender carried into the next engagement with less alive weight than this (1/4096) is gone.
const GONE: i64 = 41;

/// What the attacker knows of the defense: the defender's candidates, the stance it assumes for
/// them, and the units on the plot its opening reads (strikers, collateral targets).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DefView {
    pub candidates: Vec<UnitId>,
    pub stance: StanceType,
    pub on_plot: Vec<UnitId>,
}

/// The pair table and fighters of a pool against a defender list, computed once: an estimate
/// over many forces from the same pool (the swap search, the preview's alternatives) builds
/// each battle from it. Exactly [`Game::engage_hit`]'s and [`Game::engage_fighter`]'s values.
pub(crate) struct PairCache {
    att: Vec<UnitId>,
    def: Vec<UnitId>,
    /// `[a * nd + d]`: the attacker hitting, then the defender hitting.
    pairs: Vec<(Pair, Pair)>,
    fighters: Vec<super::Fighter>,
}

impl PairCache {
    pub fn new(g: &Game, att: &[UnitId], def: &[UnitId], ctx: &HitCtx, in_city: bool) -> PairCache {
        let mut pairs = Vec::with_capacity(att.len() * def.len());
        for &a in att {
            for &d in def {
                pairs.push((g.engage_hit(a, d, Side::Attacker, ctx), g.engage_hit(d, a, Side::Defender, ctx)));
            }
        }
        let fighters = att.iter().map(|&u| g.engage_fighter(u, Side::Attacker, in_city)).chain(def.iter().map(|&u| g.engage_fighter(u, Side::Defender, in_city))).collect();
        PairCache { att: att.to_vec(), def: def.to_vec(), pairs, fighters }
    }

    fn att_idx(&self, u: UnitId) -> Option<usize> {
        self.att.iter().position(|&x| x == u)
    }

    fn def_idx(&self, u: UnitId) -> Option<usize> {
        self.def.iter().position(|&x| x == u)
    }

    /// The pair of hitter `h` (on side `side`) and target `t`, if both are in the cache.
    pub fn pair(&self, h: UnitId, t: UnitId, side: Side) -> Option<Pair> {
        let nd = self.def.len();
        match side {
            Side::Attacker => Some(self.pairs[self.att_idx(h)? * nd + self.def_idx(t)?].0),
            Side::Defender => Some(self.pairs[self.att_idx(t)? * nd + self.def_idx(h)?].1),
        }
    }

    /// Attacker `u`'s pairs against every defender of the cache (empty when it is not in it).
    pub fn att_row(&self, u: UnitId) -> &[(Pair, Pair)] {
        let nd = self.def.len();
        match self.att_idx(u) {
            Some(i) => &self.pairs[i * nd..(i + 1) * nd],
            None => &[],
        }
    }

    /// Unit `u`'s fighter on side `side`, if it is in the cache.
    pub fn fighter(&self, u: UnitId, side: Side) -> Option<&super::Fighter> {
        match side {
            Side::Attacker => self.att_idx(u).map(|i| &self.fighters[i]),
            Side::Defender => self.def_idx(u).map(|i| &self.fighters[self.att.len() + i]),
        }
    }
}

/// Drops the later copies of a unit listed twice.
fn dedup_keep_order(v: &mut Vec<UnitId>) {
    let mut seen: Vec<UnitId> = Vec::with_capacity(v.len());
    v.retain(|&u| {
        if seen.contains(&u) {
            false
        } else {
            seen.push(u);
            true
        }
    });
}

/// The triangular sum of two rolls `0..n` each: how many of the `n^2` pairs sum to `s`.
fn two_roll_count(n: i64, s: i64) -> i64 {
    if s < 0 || s > 2 * (n - 1) {
        0
    } else {
        s.min(2 * (n - 1) - s) + 1
    }
}

impl Game {
    /// The estimate of engagement `inp` (module comment). `Err`: no eligible attacker, or no
    /// defender among the caller's list.
    pub fn engage_estimate(&self, inp: &EstimateInput) -> Result<Estimate, &'static str> {
        let ctx = inp.ctx();
        let slots = self.engage_slots(&ctx);
        let view = self.engage_estimate_view(inp);
        let force = self.engage_estimate_force(inp, &slots, &ctx, &view)?;
        self.engage_estimate_with(&ctx, &slots, &force, &view, inp.damage, &[], None)
    }

    /// The estimate of `p`'s attack from `from` on `to` as the rules see it now (plan 8.2; the
    /// preview's and the `combat` query's): `p`'s pool there ([`Eligible::Now`] under `war`), the
    /// force given (checked) or proposed under `stance` (default: the stance the attack would
    /// propose with), the defenders `p`'s team sees on `to`, the defender's stance as that team
    /// knows it. `Err`: the reason the attack is refused (as the real fight's), or the plot is
    /// not visible to `p`'s team.
    pub fn engage_estimate_plot(&self, p: PlayerId, from: PlotIdx, to: PlotIdx, force: Option<&[UnitId]>, stance: Option<StanceType>, war: WarTest) -> Result<Estimate, &'static str> {
        let ctx = HitCtx::new(from, to);
        let slots = self.engage_slots(&ctx);
        let pool = self.engage_attackers(p, from, to, war);
        let stance = stance.unwrap_or_else(|| self.engage_attack_stance(p, from, None).stance);
        let force = match force {
            Some(f) => {
                self.engage_validate_force(&pool, f, &slots)?;
                f.to_vec()
            }
            None => self.engage_propose_force_for(&pool, stance, &slots, &ctx, war),
        };
        if force.is_empty() {
            return Err("no unit can attack there");
        }
        if !self.plot_is_visible(to, self.players[p].team) {
            return Err("you cannot see that plot");
        }
        let view = self.engage_def_view(p, to, &force, war);
        self.engage_estimate_with(&ctx, &slots, &force, &view, &[], &[], None)
    }

    /// What `p`'s team knows of the defense of `to` against `force` (module comment): the
    /// defender's candidates (as the real fight finds them: invisible units are never
    /// candidates), the stance it assumes (the real one when it is its own team's, else the
    /// default) and the units on the plot it can see.
    pub(crate) fn engage_def_view(&self, p: PlayerId, to: PlotIdx, force: &[UnitId], war: WarTest) -> DefView {
        let team = self.players[p].team;
        let candidates = self.engage_defenders(to, p, force, war);
        let real = self.engage_defense_stance(to, &candidates);
        let known = candidates.first().is_some_and(|&u| self.unit_get_team(u) == team);
        let stance = if known { real.stance } else { self.d.default_stance };
        let on_plot = self.map.plots[to].units.iter().copied().filter(|&u| !self.unit_is_invisible(u, team, true)).collect();
        DefView { candidates, stance, on_plot }
    }

    /// An estimate's view of the defense: the caller's list, its units that can defend there as
    /// the candidates, the caller's stance.
    fn engage_estimate_view(&self, inp: &EstimateInput) -> DefView {
        let candidates = inp
            .defenders
            .iter()
            .copied()
            .filter(|&d| self.units.contains(d) && !self.unit_is_dead(d) && self.unit_get_team(d) != self.players[inp.player].team && self.engage_can_defend(d, inp.to))
            .collect();
        DefView { candidates, stance: inp.def_stance, on_plot: inp.defenders.iter().copied().filter(|&d| self.units.contains(d)).collect() }
    }

    /// Whether unit `u` counts as an attacker of `inp` ([`Eligible`]).
    pub fn engage_estimate_eligible(&self, inp: &EstimateInput, u: UnitId) -> bool {
        if !self.units.contains(u) || self.unit_is_dead(u) || self.units[u].owner != inp.player {
            return false;
        }
        match inp.eligible {
            Eligible::Now => self.unit_can_engage(u, inp.to, inp.war),
            Eligible::Hypothetical => {
                if !self.engage_applies(u, inp.to) || !self.unit_can_attack(u) || self.unit_is_suicide(u) || self.unit_combat_role(u).is_none() {
                    return false;
                }
                if self.plot_is_impassable(inp.to) && !self.unit_can_move_impassable(u) {
                    return false;
                }
                let limit = self.units[u].combat_limit;
                if limit < 100 {
                    if inp.from.is_some_and(|f| self.plot_is_water(f)) {
                        return false;
                    }
                    let below = inp.defenders.iter().any(|&d| self.units.contains(d) && self.engage_can_defend(d, inp.to) && super::damage_with(self, d, inp.damage) < limit);
                    if !below {
                        return false;
                    }
                }
                true
            }
        }
    }

    /// The force `inp` fights with: its pool's proposal, or its force with the ineligible left out
    /// (a force the slots cannot hold: what the default stance would commit of it).
    fn engage_estimate_force(&self, inp: &EstimateInput, slots: &Slots, ctx: &HitCtx, view: &DefView) -> Result<Vec<UnitId>, &'static str> {
        let force = match inp.attackers {
            Attackers::Force(f) => {
                let mut f: Vec<UnitId> = f.iter().copied().filter(|&u| self.engage_estimate_eligible(inp, u)).collect();
                dedup_keep_order(&mut f);
                if f.is_empty() || self.engage_validate_force(&f, &f, slots).is_ok() {
                    f
                } else {
                    self.engage_auto_pick_with(&f, self.d.default_stance, slots, Side::Attacker, ctx, inp.damage)
                }
            }
            Attackers::Pick(pool, stance) => {
                let pool: Vec<UnitId> = pool.iter().copied().filter(|&u| self.engage_estimate_eligible(inp, u)).collect();
                let pick = self.engage_auto_pick_with(&pool, stance, slots, Side::Attacker, ctx, inp.damage);
                self.engage_swap_search_in(&pool, stance, slots, ctx, pick, view, inp.damage, None)
            }
        };
        if force.is_empty() {
            Err("no unit can attack there")
        } else {
            Ok(force)
        }
    }

    /// The estimate of `force` against `view` on `ctx` with `slots` (the core of every estimate):
    /// the set-up's battle, the expected morale check and opening, carried states `start` (by
    /// unit; an assault's defenders), the moment integrator. `cache`: the pairs and fighters to
    /// build the battle from.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn engage_estimate_with(
        &self,
        ctx: &HitCtx,
        slots: &Slots,
        force: &[UnitId],
        view: &DefView,
        damage: &[(UnitId, i32)],
        start: &[(UnitId, MomentStart, i32)],
        cache: Option<&PairCache>,
    ) -> Result<Estimate, &'static str> {
        let setup = self.engage_setup_cached(*ctx, slots.clone(), force.to_vec(), view.candidates.clone(), view.stance, damage, cache)?;
        Ok(self.engage_estimate_setup(setup, view, damage, start))
    }

    /// [`Game::engage_estimate_with`] from a set-up. `start`: carried fighters, by unit, with the
    /// state they carry and the HP (centi-HP) it is on top of.
    pub(crate) fn engage_estimate_setup(&self, setup: Setup, view: &DefView, damage: &[(UnitId, i32)], start: &[(UnitId, MomentStart, i32)]) -> Estimate {
        let def_reserve = setup.def_reserve();
        let mut battle = setup.battle;
        let fear = self.engage_fear_fail(&setup.units, setup.n_att);
        for (f, &x) in battle.fighters.iter_mut().zip(&fear) {
            f.fear_fail = x;
        }
        let mut effects = self.engage_pre_battle(&battle);
        let mut starts = vec![MomentStart::default(); setup.units.len()];
        for (i, &u) in setup.units.iter().enumerate() {
            if let Some(&(_, ms, hp)) = start.iter().find(|(x, _, _)| *x == u) {
                let f = &mut battle.fighters[i];
                f.hp = hp.clamp(1, f.max_hp * 100);
                starts[i] = ms;
            }
        }
        // The opening, in expectation: a fresh fighter's HP is lowered by it, a carried one's
        // damage moment raised.
        let fight: Vec<i64> = fear.iter().map(|&x| ONE - x as i64).collect();
        let ov = UnitOverlay::with_damage(damage);
        let opening = self.engage_opening_expected(&setup.units, setup.n_att, &fight, setup.ctx.to, &view.on_plot, &ov);
        for (i, &u) in setup.units.iter().enumerate() {
            let Some(&(_, x)) = opening.iter().find(|(v, _)| *v == u) else { continue };
            let f = &mut battle.fighters[i];
            let centi = x * f.max_hp as i64 / ONE;
            if start.iter().any(|(v, _, _)| *v == u) {
                starts[i].mu += centi;
            } else {
                f.hp = (f.hp as i64 - centi).clamp(1, f.max_hp as i64 * 100) as i32;
            }
        }
        effects.extend(self.engage_opening_spells(&battle));
        if !effects.is_empty() {
            battle.add_effects(effects);
        }
        let exp = resolve_expected_from(&battle, &starts);
        let plot_falls = def_reserve.is_empty();
        Estimate {
            win_permille: exp.win_permille,
            att_loss_x1000: exp.att_loss_x1000,
            def_loss_x1000: exp.def_loss_x1000,
            att_withdrawn_x1000: exp.att_withdrawn_x1000,
            def_withdrawn_x1000: exp.def_withdrawn_x1000,
            att_hp_left: exp.att_hp_left,
            def_hp_left: exp.def_hp_left,
            volley_rounds: exp.volley_rounds,
            rounds: exp.rounds,
            plot_falls,
            falls_permille: if plot_falls { exp.win_permille } else { 0 },
            force: setup.force,
            def_force: setup.def_pick,
            def_reserve,
            slots: setup.slots,
            start_hp: battle.fighters.iter().map(|f| f.hp).collect(),
            opening,
            exp,
        }
    }

    /// The swap search (plan 15, item 3; [`Game::engage_swap_search`]) against `view`, with the
    /// damage fields `damage`: from `pick`, each round estimates every single swap
    /// ([`Game::engage_single_swaps`]: a reserve unit in, a committed unit out, held units never
    /// in; one swap per pair of kinds the estimate cannot tell apart) and keeps the best if it
    /// beats the plan so far and the start ([`swap_better`]: so tolerated small drops of the win
    /// chance never add up), at most `SWAP_ROUNDS` rounds. The result is `pick` or beats it.
    /// `cache`: the pool's pairs, when the caller has them.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn engage_swap_search_in(
        &self,
        pool: &[UnitId],
        stance: StanceType,
        slots: &Slots,
        ctx: &HitCtx,
        pick: Vec<UnitId>,
        view: &DefView,
        damage: &[(UnitId, i32)],
        cache: Option<&PairCache>,
    ) -> Vec<UnitId> {
        if pick.is_empty() || view.candidates.is_empty() || pool.len() <= pick.len() {
            return pick;
        }
        if self.engage_single_swaps(pool, &pick, stance, slots).is_empty() {
            return pick;
        }
        let own;
        let cache = match cache {
            Some(c) => c,
            None => {
                own = PairCache::new(self, pool, &view.candidates, ctx, self.plot_is_city(ctx.to, false, None));
                &own
            }
        };
        let est = |f: &[UnitId]| self.engage_estimate_with(ctx, slots, f, view, damage, &[], Some(cache)).ok();
        let kind = self.engage_swap_kinds(pool, cache, damage);
        let kind_of = |u: UnitId| pool.iter().position(|&x| x == u).map_or(usize::MAX, |i| kind[i]);
        let mut best = pick;
        let Some(mut best_est) = est(&best) else { return best };
        let start = best_est.clone();
        for _ in 0..SWAP_ROUNDS {
            let mut found: Option<(Vec<UnitId>, Estimate)> = None;
            let mut tried: Vec<(usize, usize)> = Vec::new();
            for (out, add) in self.engage_single_swaps(pool, &best, stance, slots) {
                // A swap of two units the battle cannot tell apart changes nothing, and one like a
                // swap already tried gives its estimate.
                let k = (kind_of(out), kind_of(add));
                if k.0 == k.1 || tried.contains(&k) {
                    continue;
                }
                tried.push(k);
                let f: Vec<UnitId> = best.iter().map(|&u| if u == out { add } else { u }).collect();
                let Some(e) = est(&f) else { continue };
                if swap_better(&e, found.as_ref().map_or(&best_est, |x| &x.1)) && swap_better(&e, &start) {
                    found = Some((f, e));
                }
            }
            match found {
                Some((f, e)) => {
                    best = f;
                    best_est = e;
                }
                None => break,
            }
        }
        best
    }

    /// For each unit of `pool` (in `cache`), the index of the first unit of the pool the
    /// estimate cannot tell from it: the same fighter (but its tag), the same pairs against every
    /// defender, the same damage (or override) and the same stats the expected morale check and
    /// opening read. Units outside the cache are their own kind.
    pub(crate) fn engage_swap_kinds(&self, pool: &[UnitId], cache: &PairCache, damage: &[(UnitId, i32)]) -> Vec<usize> {
        type Sig = (Option<super::Fighter>, Vec<(Pair, Pair)>, [i32; 9], [bool; 3]);
        let sig = |u: UnitId| -> Sig {
            let mut f = cache.fighter(u, Side::Attacker).cloned();
            if let Some(f) = f.as_mut() {
                f.tag = 0;
            }
            let unit = &self.units[u];
            let stats = [
                super::damage_with(self, u, damage),
                unit.level,
                self.unit_base_combat_str(u),
                self.unit_base_combat_str_defense(u),
                self.unit_collateral_damage(u),
                self.unit_collateral_damage_limit(u),
                self.unit_collateral_damage_max_units(u),
                unit.extra_collateral_damage,
                unit.unit_type.idx() as i32,
            ];
            (f, cache.att_row(u).to_vec(), stats, [self.unit_is_fear(u), self.unit_is_immune_to_fear(u), self.unit_is_immune_to_defensive_strike(u)])
        };
        let sigs: Vec<Sig> = pool.iter().map(|&u| sig(u)).collect();
        (0..pool.len()).map(|i| if sigs[i].0.is_none() { i } else { (0..=i).find(|&j| sigs[j] == sigs[i]).unwrap_or(i) }).collect()
    }

    /// Each committed fighter's chance to fail the morale check (1/4096; plan 6.3 in
    /// expectation): against the other side's strongest Fear unit, `P(rand(40) > chance)`; a
    /// defender's scaled by the chance the attacking source fights itself.
    pub(crate) fn engage_fear_fail(&self, units: &[UnitId], n_att: usize) -> Vec<i32> {
        let n = units.len();
        let mut out = vec![0i32; n];
        let none = vec![false; n];
        for (src_side, rollers) in [(n_att..n, 0..n_att), (0..n_att, n_att..n)] {
            let Some(src) = self.engage_fear_source(units, src_side, &none) else { continue };
            let su = units[src];
            let (src_str, src_level) = (self.unit_base_combat_str(su), self.units[su].level);
            let src_fights = ONE - out[src] as i64;
            for i in rollers {
                let u = units[i];
                if self.unit_is_immune_to_fear(u) {
                    continue;
                }
                let chance = (self.unit_base_combat_str(u) + 20 + self.units[u].level - src_str - src_level).max(4);
                let fail = (39 - chance).clamp(0, 40) as i64 * ONE / 40;
                out[i] = (fail * src_fights / ONE) as i32;
            }
        }
        out
    }

    /// The opening's expected damage (plan 8.2; the module comment) for the committed `units`
    /// (the first `n_att` attack; `fight[i]` the chance fighter `i` fights, 1/4096) on `to`, over
    /// the units `on_plot`, from the damage fields of `ov`: each unit's expected added damage, in
    /// percent x4096. No random number is drawn.
    pub(crate) fn engage_opening_expected(&self, units: &[UnitId], n_att: usize, fight: &[i64], to: PlotIdx, on_plot: &[UnitId], ov: &UnitOverlay) -> Vec<(UnitId, i64)> {
        let mut added: Vec<(UnitId, i64)> = Vec::new();
        let add = |added: &mut Vec<(UnitId, i64)>, u: UnitId, x: i64| match added.iter_mut().find(|(v, _)| *v == u) {
            Some(e) => e.1 += x,
            None => added.push((u, x)),
        };
        let now = |added: &Vec<(UnitId, i64)>, u: UnitId| ov.damage(self, u) as i64 * ONE + added.iter().find(|(v, _)| *v == u).map_or(0, |e| e.1);
        // Defensive strikes: per attacker in slot order, the best success among the strikers.
        let mut used: Vec<(UnitId, i64)> = Vec::new();
        for i in 0..n_att {
            let a = units[i];
            if fight[i] <= 0 || self.unit_is_immune_to_defensive_strike(a) {
                continue;
            }
            let team = self.unit_get_team(a);
            let mut strikers: Vec<(i32, usize, UnitId)> = on_plot
                .iter()
                .enumerate()
                .filter(|&(_, &s)| {
                    let su = &self.units[s];
                    su.defensive_strike_chance > 0 && su.defensive_strike_damage > 0 && self.at_war(self.unit_get_team(s), team) && (self.unit_is_blitz(s) || !ov.made_attack(self, s))
                })
                .map(|(k, &s)| (self.units[s].defensive_strike_damage, k, s))
                .collect();
            strikers.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));
            let k = (self.unit_base_combat_str_defense(a) * 2 + self.units[a].level) as i64;
            let cap = 95 - (now(&added, a) / ONE);
            let mut none_before = ONE;
            let mut expected = 0i64;
            for &(dmg, _, s) in &strikers {
                let avail = if self.unit_is_blitz(s) { ONE } else { ONE - used.iter().find(|(v, _)| *v == s).map_or(0, |e| e.1) };
                let c = self.units[s].defensive_strike_chance.clamp(0, 100) as i64 * ONE / 100;
                let p_best = fight[i] * none_before / ONE * avail / ONE * c / ONE;
                none_before = none_before * (ONE - avail * c / ONE) / ONE;
                if p_best <= 0 {
                    continue;
                }
                // E[clamp(r1 + r2 - k, 0, cap)], r1, r2 uniform in 0..dmg.
                let n = dmg as i64;
                let mut e = 0i64;
                if cap > 0 {
                    for sum in 0..=2 * (n - 1) {
                        let v = (sum - k).clamp(0, cap);
                        if v > 0 {
                            e += two_roll_count(n, sum) * v;
                        }
                    }
                }
                expected += p_best * e * ONE / (n * n) / ONE;
                if !self.unit_is_blitz(s) {
                    match used.iter_mut().find(|(v, _)| *v == s) {
                        Some(x) => x.1 = (x.1 + p_best).min(ONE),
                        None => used.push((s, p_best)),
                    }
                }
            }
            if expected > 0 {
                add(&mut added, a, expected);
            }
        }
        // Collateral: per attacker in slot order, today's formula on the healthiest targets.
        for i in 0..n_att {
            let a = units[i];
            if fight[i] <= 0 || self.unit_collateral_damage(a) <= 0 {
                continue;
            }
            let strength = self.unit_collateral_strength(a);
            if strength == 0 {
                continue;
            }
            let count = self.unit_collateral_target_count(a, to, on_plot, true).max(0) as usize;
            let mut targets: Vec<(i64, usize, UnitId)> = on_plot
                .iter()
                .enumerate()
                .filter(|&(_, &t)| self.unit_collateral_can_hit(a, t, to))
                .map(|(k, &t)| (now(&added, t), k, t))
                .collect();
            targets.sort();
            let city = self.unit_collateral_city(a, to);
            for &(cur, _, t) in targets.iter().take(count) {
                let cur_pct = ((cur + ONE / 2) / ONE) as i32;
                let new = self.unit_collateral_new_damage(a, t, cur_pct, strength, city);
                if new > cur_pct {
                    add(&mut added, t, (new - cur_pct) as i64 * fight[i]);
                }
            }
        }
        added
    }

    /// The expected assault on `inp`'s plot (module comment; plan 8.2, "Assaults"): engagement
    /// after engagement, each committing the next force from the attackers not used yet (a
    /// `Pick`: the stance's proposal; a `Force`: what the default stance commits of what is
    /// left), the defenders committed carrying their alive weight and expected damage, the reserve
    /// its expected collateral, until the expected defenders left are below one half, no attacker
    /// is left, or `MAX_ASSAULT` engagements.
    pub fn engage_estimate_assault(&self, inp: &EstimateInput) -> AssaultEstimate {
        let ctx = inp.ctx();
        let slots = self.engage_slots(&ctx);
        let view0 = self.engage_estimate_view(inp);
        let (pool, stance): (&[UnitId], Option<StanceType>) = match inp.attackers {
            Attackers::Force(f) => (f, None),
            Attackers::Pick(pool, s) => (pool, Some(s)),
        };
        let mut left: Vec<UnitId> = pool.iter().copied().filter(|&u| self.engage_estimate_eligible(inp, u)).collect();
        dedup_keep_order(&mut left);
        let mut out = AssaultEstimate { n_att: left.len(), n_def: view0.candidates.len(), ..AssaultEstimate::default() };
        let mut att_hp = 0i64;
        if left.is_empty() || view0.candidates.is_empty() {
            out.def_left_x1000 = out.n_def as i32 * 1000;
            out.falls = view0.candidates.is_empty() && !left.is_empty();
            return out;
        }
        let in_city = self.plot_is_city(ctx.to, false, None);
        let cache = PairCache::new(self, &left, &view0.candidates, &ctx, in_city);
        // Per defender: its alive weight (1/4096). A committed one carries its state and the HP
        // it is on top of; the next pick sorts every defender by `damage`.
        let mut alive: Vec<(UnitId, i64)> = view0.candidates.iter().map(|&u| (u, ONE)).collect();
        let mut carried: Vec<(UnitId, MomentStart, i32)> = Vec::new();
        let mut damage: Vec<(UnitId, i32)> = inp.damage.to_vec();
        let add_lost = |list: &mut Vec<(UnitId, i32)>, u: UnitId, x: i32| match list.iter_mut().find(|(v, _)| *v == u) {
            Some(e) => e.1 += x,
            None => list.push((u, x)),
        };
        let x1000 = |w: i64| ((w * 1000 + ONE / 2) / ONE) as i32;
        while out.steps.len() < MAX_ASSAULT && !left.is_empty() {
            let view = DefView { candidates: alive.iter().filter(|e| e.1 >= GONE).map(|e| e.0).collect(), stance: view0.stance, on_plot: view0.on_plot.clone() };
            let def_left: i64 = alive.iter().map(|e| e.1).sum();
            if def_left * 2 < ONE || view.candidates.is_empty() {
                break;
            }
            let force = match stance {
                Some(s) => {
                    let pick = self.engage_auto_pick_with(&left, s, &slots, Side::Attacker, &ctx, &damage);
                    self.engage_swap_search_in(&left, s, &slots, &ctx, pick, &view, &damage, Some(&cache))
                }
                None => self.engage_auto_pick_with(&left, self.d.default_stance, &slots, Side::Attacker, &ctx, &damage),
            };
            if force.is_empty() {
                break;
            }
            let starts: Vec<(UnitId, MomentStart, i32)> = carried.clone();
            let Ok(est) = self.engage_estimate_with(&ctx, &slots, &force, &view, &damage, &starts, Some(&cache)) else { break };
            if out.steps.is_empty() {
                out.first = est.win_permille;
            }
            left.retain(|u| !force.contains(u));
            let na = est.force.len();
            for (i, &u) in est.force.iter().enumerate() {
                let fe = &est.exp.fighters[i];
                add_lost(&mut out.att_lost, u, x1000(fe.lost as i64));
                att_hp += fe.hp_left as i64;
            }
            // The defenders committed: alive weight and damage carried (withdrawn ones have fled).
            for (j, &u) in est.def_force.iter().enumerate() {
                let fe = &est.exp.fighters[na + j];
                add_lost(&mut out.def_lost, u, x1000(fe.lost as i64));
                if let Some(e) = alive.iter_mut().find(|e| e.0 == u) {
                    e.1 = fe.alive as i64;
                }
                let ms = MomentStart { alive: fe.alive, mu: fe.mu, var: fe.var };
                match carried.iter_mut().find(|c| c.0 == u) {
                    Some(c) => c.1 = ms,
                    None => carried.push((u, ms, est.start_hp[na + j])),
                }
                let max = self.unit_engage_max_hp(u).max(1);
                damage.push((u, (100 - (fe.hp_alive + max - 1) / max).clamp(0, 99)));
            }
            // The reserve: its expected collateral.
            for &(u, x) in &est.opening {
                if est.def_force.contains(&u) || est.force.contains(&u) || carried.iter().any(|c| c.0 == u) {
                    continue;
                }
                let now = super::damage_with(self, u, &damage);
                damage.push((u, (now + ((x + ONE / 2) / ONE) as i32).clamp(0, 99)));
            }
            out.steps.push(est);
        }
        out.losses_x1000 = out.att_lost.iter().map(|e| e.1).sum();
        out.kills_x1000 = out.def_lost.iter().map(|e| e.1).sum();
        let def_left: i64 = alive.iter().map(|e| e.1).sum();
        out.def_left_x1000 = x1000(def_left);
        out.falls = def_left * 2 < ONE;
        out.used = out.falls.then_some(out.steps.len());
        out.att_hp_left = att_hp;
        out.def_hp_left = alive
            .iter()
            .map(|&(u, a)| {
                let hp = match carried.iter().find(|c| c.0 == u) {
                    Some(c) => (c.2 as i64 - c.1.mu).max(0),
                    None => super::hp_from_damage(super::damage_with(self, u, &damage), self.unit_engage_max_hp(u)) as i64,
                };
                a * hp / ONE
            })
            .sum();
        out
    }
}
#[cfg(test)]
mod tests {
    use super::two_roll_count;

    #[test]
    fn two_rolls_sum_to_the_square() {
        for n in 1..40i64 {
            let total: i64 = (0..=2 * (n - 1)).map(|s| two_roll_count(n, s)).sum();
            assert_eq!(total, n * n);
            let brute = |s: i64| (0..n).flat_map(|a| (0..n).map(move |b| a + b)).filter(|&x| x == s).count() as i64;
            for s in 0..=2 * (n - 1) {
                assert_eq!(two_roll_count(n, s), brute(s), "n {n} s {s}");
            }
        }
    }
}
