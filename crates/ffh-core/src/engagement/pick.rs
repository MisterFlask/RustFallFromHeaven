//! Who can fight an engagement (plan 5.2).
//!
//! **Who attacks** ([`Game::unit_can_engage`]): a land unit next to the land plot it attacks,
//! able to attack, with moves left, not marching, not a missile, that may still attack this turn
//! (one attack a turn, Blitz one more: [`Game::unit_has_attack_left`], counting engagements,
//! bombards and missile strikes) and that the move rules let attack the plot
//! (`unit_can_move_into` with its attack gates under the option: the attack count, and for a
//! unit with a combat limit some defender below the limit and no amphibious attack:
//! `unit_engage_limit_ok`). The
//! attacker's pool ([`Game::engage_attackers`]) is every such unit of the player on the plot it
//! attacks from: its armies, its garrison, its heroes, mages and recon units.
//!
//! **Who defends** ([`Game::engage_defenders`]): every unit on the plot that can defend there,
//! is an enemy of the attacking side as `plot_get_best_defender` tests it (both ways, so hidden
//! nationality works as in the duel) and is not invisible to the attacking team (G5); a
//! declare-war preview also counts every team the declaration would bring into the war
//! ([`WarTest::DeclareWar`]). Ships never
//! defend land; marching units and immobile units with strength do. A unit defends any number of
//! times a turn. Under the option `plot_get_best_defender` (and its "better defender" ratings)
//! no longer picks the land defender; it stays for its other callers.
//!
//! Where the attacker's units and the defenders stand is `arrange.rs`'s (the auto formation, an
//! ordered formation's check).

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;
use crate::unit::Declared;

/// Which war an engagement test assumes (plan 5.2).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WarTest {
    /// The real fight: the defenders are the attacking side's enemies now (at war, or hostile:
    /// hidden nationality).
    AtWar,
    /// The preview of a move that declares war on `team`: the units of every team the
    /// declaration would bring into the war count as enemies too (`team`, its vassals and master,
    /// its defensive-pact partners and theirs: [`Game::team_wars_after_declaring`]), as the real
    /// fight after the declaration counts them.
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
        // A summon called into a battle is in that battle only (summons.rs).
        if !self.engage_applies(u, to) || self.units[u].battle_summon {
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
        // the limit against the teams it would fight.
        if declare_war && !self.unit_engage_limit_ok(u, to, war) {
            return false;
        }
        self.unit_can_move_into(u, to, true, declare_war, false)
    }

    /// The combat-limit gate of an engagement (plan 5.2; C++ `canAttack(defender)`'s rules, on
    /// every defender instead of the best one), for a unit with a combat limit (siege): from the
    /// water it attacks no plot holding a unit it would meet, a non-combatant included (no
    /// amphibious artillery, as today: the duel's best defender may be a lone Worker); otherwise
    /// one of the units it would meet must be below its limit: a defender when the plot has any,
    /// else a non-combatant (as today). Units without a limit pass at once.
    pub(crate) fn unit_engage_limit_ok(&self, u: UnitId, to: PlotIdx, war: WarTest) -> bool {
        if self.units[u].combat_limit >= 100 {
            return true;
        }
        // (Card: Causeway, Ferries (docs/CARDS.md): water walked on as land is not water here.)
        let from_water = self.unit_plot_opt(u).is_some_and(|at| self.plot_is_water(at) && !self.unit_walks_on_water(u, at));
        self.unit_engage_limit_ok_from(u, from_water, to, war)
    }

    /// [`Game::unit_engage_limit_ok`] for an attack made from water (`from_water`) or not, wherever
    /// the unit stands now (a hypothetical attack: `engage_could_attack_from_war`).
    pub(crate) fn unit_engage_limit_ok_from(&self, u: UnitId, from_water: bool, to: PlotIdx, war: WarTest) -> bool {
        let limit = self.units[u].combat_limit;
        if limit >= 100 {
            return true;
        }
        let team = self.players[self.units[u].owner].team;
        let declared = self.engage_declared(team, war);
        let (mut defenders, mut others, mut other_below) = (false, false, false);
        for &d in &self.map.plots[to].units {
            if !self.engage_meets(d, to, team, std::slice::from_ref(&u), declared.as_ref()) {
                continue;
            }
            if from_water {
                return false;
            }
            let below = self.units[d].damage < limit;
            if self.engage_can_defend(d, to) {
                if below {
                    return true;
                }
                defenders = true;
            } else {
                others = true;
                other_below |= below;
            }
        }
        !defenders && (!others || other_below)
    }

    /// Whether unit `u`, wherever it stands now, could attack `to` from `from` in an engagement
    /// were it there with moves left (the hypothetical forecast's pool, `Game::engage_forecast_if`):
    /// [`Game::unit_can_engage`]'s tests that do not depend on where it is or on what it did this
    /// turn -- a land unit able to attack, not marching, not a missile, not a battle summon, within
    /// its combat limit -- and it could stand on `from` (land, or water it walks on; a peak only
    /// if it can climb it). `war`: the war its combat limit is tested under (the defenders it
    /// meets).
    pub(crate) fn engage_could_attack_from_war(&self, u: UnitId, from: PlotIdx, to: PlotIdx, war: WarTest) -> bool {
        if !self.engage_applies(u, to) || self.units[u].battle_summon || self.unit_is_dead(u) {
            return false;
        }
        let unit = &self.units[u];
        if unit.march.is_some() || self.unit_is_suicide(u) || !self.unit_can_attack(u) {
            return false;
        }
        if self.plot_is_water(from) && !self.unit_can_move_all_terrain(u) && !self.unit_walks_on_water(u, from) {
            return false;
        }
        if self.plot_is_impassable(from) && !self.unit_can_move_impassable(u) && !self.unit_can_climb(u, from) {
            return false;
        }
        // (The attack comes from `from`: not from where it stands now, aboard a ship say.)
        let from_water = self.plot_is_water(from) && !self.unit_walks_on_water(u, from);
        self.unit_engage_limit_ok_from(u, from_water, to, war)
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
        let team = self.players[p].team;
        let declared = self.engage_declared(team, war);
        self.map.plots[to]
            .units
            .iter()
            .copied()
            .filter(|&d| self.engage_meets(d, to, team, force, declared.as_ref()) && self.engage_can_defend(d, to))
            .collect()
    }

    /// The teams a `war` test counts as enemies of `team` beyond its wars now: none for the real
    /// fight; for a declare-war preview every team the declaration brings in.
    pub(crate) fn engage_declared(&self, team: TeamId, war: WarTest) -> Option<IdxVec<TeamId, bool>> {
        match war {
            WarTest::AtWar => None,
            WarTest::DeclareWar(t) => Some(self.team_wars_after_declaring(team, t)),
        }
    }

    /// The declaration `team`'s plan under `war` assumes made, for its opening's pure rolls
    /// (`UnitOverlay`: the defensive strikes and the collateral test war as the real fight after
    /// the declaration does): `None` for the real fight.
    pub(crate) fn engage_declaration(&self, team: TeamId, war: WarTest) -> Option<Declared> {
        self.engage_declared(team, war).map(|at_war| Declared { team, at_war })
    }

    /// Whether `team`'s attack with `force` on `to` meets unit `d` there, combatant or not: not
    /// its own, alive, not invisible to it (G5), and an enemy as `plot_get_best_defender` tests
    /// it (both ways: hidden nationality), or of a team in `declared` ([`Game::engage_declared`]).
    pub(crate) fn engage_meets(&self, d: UnitId, to: PlotIdx, team: TeamId, force: &[UnitId], declared: Option<&IdxVec<TeamId, bool>>) -> bool {
        let d_team = self.unit_get_team(d);
        if d_team == team || self.unit_is_dead(d) || self.unit_is_invisible(d, team, true) {
            return false;
        }
        self.unit_is_enemy(d, Some(team), Some(to))
            || (!force.is_empty() && force.iter().all(|&a| self.unit_is_enemy(a, Some(d_team), Some(to))))
            || declared.is_some_and(|w| w[d_team])
    }

    /// Whether a unit the attack meets on `to` is one of its defenders ([`Game::engage_defenders`]):
    /// it can defend there and fights land battles (a land or immobile type).
    pub(crate) fn engage_can_defend(&self, d: UnitId, to: PlotIdx) -> bool {
        !self.units[d].battle_summon && self.unit_can_defend(d, Some(to)) && self.unit_engages_on_land(d)
    }
}
