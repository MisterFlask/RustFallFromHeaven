//! Who can fight an engagement (plan 5.2).
//!
//! **Who attacks** ([`Game::unit_can_engage`]): a land unit next to the land plot it attacks,
//! able to attack, with moves left, not marching, not a missile, that may still attack this turn
//! (one attack a turn, Blitz one more: [`Game::unit_has_attack_left`], counting engagements,
//! bombards and missile strikes) and that the move rules let attack the plot
//! (`unit_can_move_into` with its attack gates under the option: the attack count, and for a
//! unit with a combat limit some defender below the limit and no amphibious attack). The
//! attacker's pool ([`Game::engage_attackers`]) is every such unit of the player on the plot it
//! attacks from: its armies, its garrison, its heroes, mages and recon units.
//!
//! **Who defends** ([`Game::engage_defenders`]): every unit on the plot that can defend there,
//! is an enemy of the attacking side as `plot_get_best_defender` tests it (both ways, so hidden
//! nationality works as in the duel) and is not invisible to the attacking team (G5). Ships never
//! defend land; marching units and immobile units with strength do. A unit defends any number of
//! times a turn. Under the option `plot_get_best_defender` (and its "better defender" ratings)
//! no longer picks the land defender; it stays for its other callers.

use crate::game::Game;
use crate::ids::*;

/// Which war an engagement test assumes (plan 5.2).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WarTest {
    /// The real fight: the defenders are the attacking side's enemies now (at war, or hostile:
    /// hidden nationality).
    AtWar,
    /// The preview of a move that declares war on `team`: its units count as enemies too.
    DeclareWar(TeamId),
}

impl Game {
    /// Whether unit `u` may make another attack this turn under engagements (plan 5.2): one a
    /// turn, Blitz one more. Engagements, bombards and missile strikes count
    /// (`Unit::attacks_made`); any other attack this turn (a duel: boarding a ship) counts as one.
    pub fn unit_has_attack_left(&self, u: UnitId) -> bool {
        let unit = &self.units[u];
        let spent = unit.attacks_made.max(unit.made_attack as i32);
        spent < 1 + self.unit_is_blitz(u) as i32
    }

    /// Whether unit `u` can attack plot `to` in an engagement (module comment; plan 5.2). Cheap:
    /// no best-defender search (a plot scan only for a unit with a combat limit).
    pub fn unit_can_engage(&self, u: UnitId, to: PlotIdx, war: WarTest) -> bool {
        if !self.engage_applies(u, to) {
            return false;
        }
        let Some(at) = self.unit_plot_opt(u) else { return false };
        let ((fx, fy), (tx, ty)) = (self.map.xy(at), self.map.xy(to));
        if self.step_distance(fx, fy, tx, ty) != 1 {
            return false;
        }
        let unit = &self.units[u];
        if unit.march.is_some() || self.unit_is_suicide(u) || !self.unit_can_attack(u) || !self.unit_can_move(u) {
            return false;
        }
        let declare_war = matches!(war, WarTest::DeclareWar(_));
        // `unit_can_move_into`'s attack branch tests the attack count and the combat limit
        // against the defenders at war (`unit_engage_limit_ok`); a declare-war preview also tests
        // the limit against the team it would fight.
        if declare_war && !self.unit_engage_limit_ok(u, to, war) {
            return false;
        }
        self.unit_can_move_into(u, to, true, declare_war, false)
    }

    /// The combat-limit gate of an engagement (plan 5.2; C++ `canAttack(defender)`'s rules, on
    /// every defender instead of the best one): when `to` has defenders, a unit with a combat
    /// limit (siege, Illusion) needs one below its limit and does not attack from the water (no
    /// amphibious artillery). Units without a limit pass at once.
    pub(crate) fn unit_engage_limit_ok(&self, u: UnitId, to: PlotIdx, war: WarTest) -> bool {
        let limit = self.units[u].combat_limit;
        if limit >= 100 {
            return true;
        }
        let from_water = self.unit_plot_opt(u).is_some_and(|at| self.plot_is_water(at));
        let owner = self.units[u].owner;
        let mut any = false;
        for &d in &self.map.plots[to].units {
            if self.engage_is_defender(d, to, owner, std::slice::from_ref(&u), war) {
                if !from_water && self.units[d].damage < limit {
                    return true;
                }
                any = true;
            }
        }
        !any
    }

    /// The attacker's pool (plan 5.2): every unit of `p` on `from` that can attack `to` in an
    /// engagement, in the plot's order.
    pub fn engage_attackers(&self, p: PlayerId, from: PlotIdx, to: PlotIdx, war: WarTest) -> Vec<UnitId> {
        self.map.plots[from].units.iter().copied().filter(|&u| self.units[u].owner == p && self.unit_can_engage(u, to, war)).collect()
    }

    /// The defender's candidates on `to` against `p`'s attack with `force` (plan 5.2; the module
    /// comment), in the plot's order. `force` decides hidden nationality: a force all of whose
    /// units are hostile to everyone fights any team's units, as such an attacker does in the
    /// duel; an empty force fights only `p`'s enemies.
    pub fn engage_defenders(&self, to: PlotIdx, p: PlayerId, force: &[UnitId], war: WarTest) -> Vec<UnitId> {
        self.map.plots[to].units.iter().copied().filter(|&d| self.engage_is_defender(d, to, p, force, war)).collect()
    }

    /// Whether unit `d` on `to` is a defender against `p`'s attack with `force`
    /// ([`Game::engage_defenders`]).
    fn engage_is_defender(&self, d: UnitId, to: PlotIdx, p: PlayerId, force: &[UnitId], war: WarTest) -> bool {
        let team = self.players[p].team;
        let d_team = self.unit_get_team(d);
        if d_team == team || self.unit_is_dead(d) || !self.unit_can_defend(d, Some(to)) || self.unit_combat_role(d).is_none() {
            return false;
        }
        if self.unit_is_invisible(d, team, true) {
            return false;
        }
        self.unit_is_enemy(d, Some(team), Some(to))
            || (!force.is_empty() && force.iter().all(|&a| self.unit_is_enemy(a, Some(d_team), Some(to))))
            || war == WarTest::DeclareWar(d_team)
    }
}
