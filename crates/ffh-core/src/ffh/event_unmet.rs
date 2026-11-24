//! Why an event option's own script (`PythonCanDo`, `Game::py_event_unmet`) refuses it: the check
//! of the script that failed, with what the text needs to say it.

use ffh_data::*;

/// The check of an event option's `PythonCanDo` script that failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventUnmet {
    /// The player must run this civic.
    Civic(CivicType),
    /// Not for a player with this state religion.
    NotStateReligion(ReligionType),
    /// The event's city must have this building.
    CityLacks(BuildingType),
    /// The leader has this trait permanently (the option gives it).
    PermanentTrait(TraitType),
    /// Requires an outcome of an earlier scenario (a world-builder trophy).
    Trophy,
    /// The player must have this much gold.
    Gold(i32),
    /// The event has no other player.
    NoOtherPlayer,
    /// No trade network with the other player.
    NoTrade,
    /// The city has no building that can be destroyed: a real building with a positive cost
    /// that is not a limited wonder, of at most `max_cost` if set. `other`: the other player's
    /// city.
    NoBuildingToLose { other: bool, max_cost: Option<i32> },
    /// The city has a building that can be destroyed (the option is for a city with none).
    BuildingToLose,
    /// The player has no second Copper, Iron, Horse or Ivory that the other player lacks.
    NoSpareStrategic,
    /// No third team is at war with the other player and not with the player.
    NoThirdPartyWar,
    /// No plot next to the event's plot has an improvement.
    NoAdjacentImprovement,
}
