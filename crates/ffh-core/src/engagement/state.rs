//! Engagement combat on `Game`: the option and the dispatch rule (plan section 1).
//!
//! **Which attacks are engagements.** [`Game::engage_applies`]: the option is on (it requires
//! armies), the attacker is a land unit (one carried on a transport counts: an amphibious attack)
//! and the target plot is land. Ships against ships, the sea-pillage interceptor, air units and
//! every attack with the option off keep today's duel.

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;

impl Game {
    /// Whether this game fights engagements (`GAMEOPTION_ENGAGEMENTS`, which requires armies).
    #[inline]
    pub fn engagements_on(&self) -> bool {
        self.is_option(GameOptionType::GAMEOPTION_ENGAGEMENTS) && self.armies_on()
    }

    /// Whether unit `u` attacking plot `to` is an engagement (module comment); otherwise it is
    /// today's duel.
    pub fn engage_applies(&self, u: UnitId, to: PlotIdx) -> bool {
        self.engagements_on() && self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && !self.plot_is_water(to)
    }

    /// [`Game::engage_applies`] for a group: on the group's domain (its head unit's).
    pub fn group_engage_applies(&self, g: GroupId, to: PlotIdx) -> bool {
        self.engagements_on() && self.group_get_domain_type(g) == Some(DomainType::DOMAIN_LAND) && !self.plot_is_water(to)
    }
}
