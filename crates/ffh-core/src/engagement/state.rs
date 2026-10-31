//! Engagement combat on `Game`: the option, the dispatch rule (plan section 1) and the stances
//! (plan 5.3).
//!
//! **Which attacks are engagements.** [`Game::engage_applies`]: the option is on (it requires
//! armies: stances live on armies and garrisons), the attacker is a land unit (one carried on a
//! transport counts: an amphibious attack) and the target plot is land. Ships against ships, the
//! sea-pillage interceptor, air units and every attack with the option off keep today's duel.
//!
//! **Stances.** An army's stance ([`Army::stance`](crate::army::Army::stance)) is its standing
//! order: what it defends with, and the force the rules propose when it attacks; a city's
//! [`garrison_stance`](crate::city::City::garrison_stance) is its garrison's. A new army takes
//! the stance of the army its units came from (a split), or of the garrison they came from
//! (`army_inherited_stance`, army/mod.rs); a merge keeps the receiving army's. With the option off
//! every stance stays the default and the commands are refused.

use ffh_data::*;

use crate::army::ArmyResult;
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

    /// Sets the stance of `p`'s army `a`.
    pub fn army_set_stance(&mut self, p: PlayerId, a: ArmyId, stance: StanceType) -> ArmyResult {
        if !self.engagements_on() {
            return Err("engagements are off in this game");
        }
        if !self.armies.contains(a) || self.armies[a].owner != p {
            return Err("no such army");
        }
        if stance.idx() >= self.d.stances.len() {
            return Err("no such stance");
        }
        self.armies[a].stance = stance;
        Ok(())
    }

    /// Sets the stance of the garrison of `p`'s city `c`.
    pub fn city_set_garrison_stance(&mut self, p: PlayerId, c: CityId, stance: StanceType) -> ArmyResult {
        if !self.engagements_on() {
            return Err("engagements are off in this game");
        }
        if !self.cities.contains(c) || self.cities[c].owner != p {
            return Err("not your city");
        }
        if stance.idx() >= self.d.stances.len() {
            return Err("no such stance");
        }
        self.cities[c].garrison_stance = stance;
        Ok(())
    }
}
