//! Bounded reachability flood for target selection.
//!
//! The C++ AI runs a full A* (`generatePath`) for every candidate target of every unit,
//! which dominates late-game turn time. Here a unit floods outward once (Dijkstra on
//! movement points, bounded by a turn budget) and scores every reached plot; only the
//! chosen target gets a real path, through the rules' `MISSION_MOVE_TO`.
//!
//! The flood is an estimate: it uses the unit's own passability (`canMoveInto` without
//! attacking) and plot movement costs, treats unrevealed plots as passable at one move,
//! and does not model zones of control or stacking limits. The rules' path finder has the
//! final word when the mission starts.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// One reached plot: `turns` is the estimated number of turns to arrive (0 = this turn).
#[derive(Copy, Clone, Debug)]
pub struct Reached {
    pub plot: PlotIdx,
    pub turns: i32,
}

/// Reusable flood buffers (one per AI turn; sized to the map lazily).
#[derive(Default)]
pub struct Flood {
    cost: Vec<i32>,
    touched: Vec<PlotIdx>,
    heap: BinaryHeap<Reverse<(i32, u32)>>,
    /// Per-run memo of the predicates that depend only on the entered plot (and the unit and
    /// the game, both fixed during a run): `memo[q]` is valid when `memo_run[q] == run`.
    /// Bits: [`ENEMY_KNOWN`], [`ENEMY`], [`ENTER_KNOWN`], [`ENTER`].
    memo: Vec<u8>,
    memo_run: Vec<u32>,
    run: u32,
}

const ENEMY_KNOWN: u8 = 1;
const ENEMY: u8 = 2;
const ENTER_KNOWN: u8 = 4;
const ENTER: u8 = 8;

impl Flood {
    fn reset(&mut self, num_plots: usize) {
        if self.cost.len() != num_plots {
            self.cost = vec![i32::MAX; num_plots];
            self.touched.clear();
        } else {
            for &p in &self.touched {
                self.cost[p.idx()] = i32::MAX;
            }
            self.touched.clear();
        }
        self.heap.clear();
        if self.memo.len() != num_plots || self.run == u32::MAX {
            self.memo = vec![0; num_plots];
            self.memo_run = vec![0; num_plots];
            self.run = 0;
        }
        self.run += 1;
    }

    /// The memo bits of `q` for this run (0 if none computed yet).
    #[inline]
    fn memo_get(&self, q: PlotIdx) -> u8 {
        if self.memo_run[q.idx()] == self.run { self.memo[q.idx()] } else { 0 }
    }

    #[inline]
    fn memo_set(&mut self, q: PlotIdx, bits: u8) {
        self.memo_run[q.idx()] = self.run;
        self.memo[q.idx()] = bits;
    }

    /// `g.plot_is_visible_enemy_unit_for_unit(q, u)`, computed once per plot and run.
    #[inline]
    fn visible_enemy(&mut self, g: &Game, u: UnitId, q: PlotIdx) -> bool {
        let m = self.memo_get(q);
        if m & ENEMY_KNOWN != 0 {
            return m & ENEMY != 0;
        }
        let enemy = g.plot_is_visible_enemy_unit_for_unit(q, u);
        self.memo_set(q, m | ENEMY_KNOWN | if enemy { ENEMY } else { 0 });
        enemy
    }

    /// `g.unit_can_move_into(u, q, false, false, false)`, computed once per plot and run.
    #[inline]
    fn can_enter(&mut self, g: &Game, u: UnitId, q: PlotIdx) -> bool {
        let m = self.memo_get(q);
        if m & ENTER_KNOWN != 0 {
            return m & ENTER != 0;
        }
        let enter = g.unit_can_move_into(u, q, false, false, false);
        self.memo_set(q, m | ENTER_KNOWN | if enter { ENTER } else { 0 });
        enter
    }

    /// Flood from unit `u`'s plot for at most `max_turns` turns. `allow_attack_last`
    /// lets the last step enter a plot holding enemies (for attack targets).
    /// Returns reached plots in nondecreasing cost order (deterministic tie-break on index).
    pub fn run(&mut self, g: &Game, u: UnitId, max_turns: i32, allow_enemy_target: bool) -> Vec<Reached> {
        let max_moves = g.unit_max_moves(u).max(1);
        // Moves already spent this turn count against the first turn.
        let spent = (max_moves - g.unit_moves_left(u)).max(0);
        self.run_moves(g, u, max_moves, spent, max_turns, allow_enemy_target)
    }

    /// [`Flood::run`] that stops once `stop` returns true for a reached plot (that plot is the
    /// last one returned): the result is that prefix of what `run` returns.
    pub fn run_until(&mut self, g: &Game, u: UnitId, max_turns: i32, allow_enemy_target: bool, stop: impl FnMut(Reached) -> bool) -> Vec<Reached> {
        let max_moves = g.unit_max_moves(u).max(1);
        let spent = (max_moves - g.unit_moves_left(u)).max(0);
        self.flood(g, u, max_moves, spent, max_turns, allow_enemy_target, stop)
    }

    /// [`Flood::run`] that stops after the plots arrived at on the first turn any plot `pred`
    /// accepts is arrived at: everything a minimum over the accepted plots keyed by `turns` first
    /// can choose (the prefix of `run` holding all accepted plots of the fewest turns).
    pub fn run_nearest(&mut self, g: &Game, u: UnitId, max_turns: i32, allow_enemy_target: bool, mut pred: impl FnMut(Reached) -> bool) -> Vec<Reached> {
        // Plots come out in nondecreasing turns when no step costs less than 0: `plot_movement_cost`
        // returns the move denominator, the unit's own moves or at least 1. Else no early stop.
        let monotone = g.d.defines.move_denominator > 0 && g.unit_max_moves(u) >= 0;
        let mut first: Option<i32> = None;
        self.run_until(g, u, max_turns, allow_enemy_target, |r| match first {
            Some(t) => monotone && r.turns > t,
            None => {
                if pred(r) {
                    first = Some(r.turns);
                }
                false
            }
        })
    }

    /// [`Flood::run`] for a unit of `max_moves` moves (in `MOVE_DENOMINATOR` units), `spent` of
    /// them used this turn, that moves as `u` does (its passability and movement costs): a unit
    /// `u` is about to create on its plot, such as a summon.
    pub fn run_moves(&mut self, g: &Game, u: UnitId, max_moves: i32, spent: i32, max_turns: i32, allow_enemy_target: bool) -> Vec<Reached> {
        self.flood(g, u, max_moves, spent, max_turns, allow_enemy_target, |_| false)
    }

    #[inline]
    fn flood(&mut self, g: &Game, u: UnitId, max_moves: i32, spent: i32, max_turns: i32, allow_enemy_target: bool, mut stop: impl FnMut(Reached) -> bool) -> Vec<Reached> {
        let num_plots = g.map.num_plots();
        self.reset(num_plots);
        let start = g.unit_plot(u);
        let team = g.unit_get_team(u);
        let land = g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND;
        let max_moves = max_moves.max(1);
        let budget = max_moves * (max_turns + 1);
        // Every step costs at least 1 when the unit's own moves and the move denominator are
        // positive (`plot_movement_cost` returns one of them or at least 1; a step past the end
        // of a turn costs the moves left, at least 1): a plot at the budget then reaches nothing.
        let steps_positive = g.d.defines.move_denominator > 0 && g.unit_max_moves(u) > 0;
        // With no step below 0, plots come out in nondecreasing cost (`run_nearest` relies on it).
        // Both rest on `plot_movement_cost`'s return values: checked in debug builds.
        let steps_nonnegative = g.d.defines.move_denominator > 0 && g.unit_max_moves(u) >= 0;
        let mut last_cost = i32::MIN;
        let mut out = Vec::new();

        self.cost[start.idx()] = spent;
        self.touched.push(start);
        self.heap.push(Reverse((spent, start.idx() as u32)));

        while let Some(Reverse((c, pi))) = self.heap.pop() {
            let p = PlotIdx::from_usize(pi as usize);
            if c > self.cost[p.idx()] {
                continue;
            }
            debug_assert!(!steps_nonnegative || c >= last_cost, "flood: plot popped out of cost order");
            last_cost = c;
            let r = Reached { plot: p, turns: c / max_moves };
            out.push(r);
            if stop(r) {
                break;
            }
            if c >= budget && steps_positive {
                continue;
            }
            // Do not expand through plots with visible enemies (they are targets, not paths).
            if p != start && self.visible_enemy(g, u, p) {
                continue;
            }
            let (x, y) = g.map.xy(p);
            for dir in 0..DirectionType::NUM {
                let Some(q) = g.plot_direction(x, y, DirectionType::from_usize(dir)) else { continue };
                if self.cost[q.idx()] != i32::MAX && self.cost[q.idx()] <= c {
                    continue;
                }
                let revealed = g.map.plots[q].revealed[team];
                let step = if !revealed {
                    if land && g.map.plots[q].plot_type == PlotType::PLOT_OCEAN {
                        continue;
                    }
                    g.d.defines.move_denominator
                } else {
                    let enemy = self.visible_enemy(g, u, q);
                    if enemy {
                        if !allow_enemy_target {
                            continue;
                        }
                    } else if !self.can_enter(g, u, q) {
                        continue;
                    }
                    g.plot_movement_cost(q, u, p)
                };
                // A unit can always enter a plot with any moves left this turn: moves past the
                // end of a turn are rounded up to the next turn boundary.
                let moves_left_this_turn = max_moves - (c % max_moves);
                let nc = if step >= moves_left_this_turn { c + moves_left_this_turn } else { c + step };
                debug_assert!(!steps_positive || nc > c, "flood: a step cost less than 1");
                if nc > budget {
                    continue;
                }
                if nc < self.cost[q.idx()] {
                    if self.cost[q.idx()] == i32::MAX {
                        self.touched.push(q);
                    }
                    self.cost[q.idx()] = nc;
                    self.heap.push(Reverse((nc, q.idx() as u32)));
                }
            }
        }
        out
    }
}
