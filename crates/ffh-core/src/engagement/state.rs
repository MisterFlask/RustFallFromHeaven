//! Engagement combat on `Game`: the dispatch rule (plan section 1).
//!
//! **Which attacks are engagements.** [`Game::engage_applies`]: the attacker is a land unit (one
//! carried on a transport counts: an amphibious attack) and the target plot is land. Ships against
//! ships, the sea-pillage interceptor and air units keep the duel. Engagement combat is the game's
//! land combat, not an option (the owner's decision, 2026-11-03: `GAMEOPTION_ENGAGEMENTS` is gone).

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;

impl Game {
    /// The unit half of [`Game::engage_applies`]: whether unit `u` fights engagements when it
    /// attacks a land plot (its domain is land; one carried on a ship too). Not
    /// [`Game::unit_engages_on_land`], which is whether it can stand in a land battle at all. The
    /// host serves it to the client per unit.
    pub fn unit_attack_engages(&self, u: UnitId) -> bool {
        self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
    }

    /// Whether unit `u` attacking plot `to` is an engagement (module comment); otherwise it is
    /// the duel.
    pub fn engage_applies(&self, u: UnitId, to: PlotIdx) -> bool {
        self.unit_attack_engages(u) && !self.plot_is_water(to)
    }

    /// [`Game::engage_applies`] for a group: on the group's domain (its head unit's).
    pub fn group_engage_applies(&self, g: GroupId, to: PlotIdx) -> bool {
        self.group_get_head_unit(g).is_some_and(|head| self.unit_attack_engages(head))
            && !self.plot_is_water(to)
    }
}
