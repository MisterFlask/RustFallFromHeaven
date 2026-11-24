//! Why a city cannot train a unit, construct a building or create a project: the first check of
//! the rules' gate that fails (`city_train_refusal`, `city_construct_refusal`,
//! `city_create_refusal` and the player-, plot- and Python-level gates they call). The boolean
//! gates (`city_can_train`, `city_can_construct`, `city_can_create`, ...) are these with
//! `.is_none()`, so a reason cannot drift from the rule.
//!
//! Every enum is `Copy` and holds only ids and small counts: the AI calls the gates in hot loops,
//! and a payload is computed only at the failing check. The sentences are ffh-play's
//! (`view::refusal`).

use crate::ids::CityId;
use ffh_data::*;

/// Who a limit on instances counts: the world, the team or the player.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LimitScope {
    /// A world unit, wonder or project (`max_global_instances`; the created count, ever).
    World,
    /// A team unit, wonder or project (`max_team_instances`).
    Team,
    /// A national unit or wonder (`max_player_instances`, plus `extra_player_instances` for a
    /// building class).
    Player,
}

/// A maxed-out limit on instances: `count` (with those in production when `in_production`) has
/// reached `max`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Limit {
    pub scope: LimitScope,
    /// The count includes those in production (the check outside a visibility test).
    pub in_production: bool,
    /// The count the rule compares (instances, plus those in production, less the one continued).
    pub count: i32,
    pub max: i32,
}

/// Why player-level rules refuse a unit: [`crate::game::Game::player_train_refusal`]
/// (C++ `CvPlayer::canTrain`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlayerTrainRefusal {
    /// `production_cost == -1`: the unit is never trained.
    NoProductionCost,
    /// One City Challenge: a human player trains no unit that founds cities.
    OneCityChallenge,
    /// Spies and units with espionage points (none in this game's data).
    Espionage,
    /// A tech the team lacks.
    Tech(TechType),
    /// The unit's state religion is not the player's.
    StateReligion(ReligionType),
    /// A limit on instances is reached.
    Limit(Limit),
    /// Nuclear units under no-nukes (none in this game's data).
    Nukes,
    /// The unit's special unit type is not enabled yet.
    SpecialUnit(SpecialUnitType),
    /// The player does not run the civic.
    Civic(CivicType),
    /// The Armageddon Counter is below the unit's.
    GlobalCounter { have: i32, need: i32 },
    /// The player's alignment is not this one.
    Alignment(AlignmentType),
}

/// Why the city's plot refuses a unit: [`crate::game::Game::plot_train_refusal`]
/// (C++ `CvPlot::canTrain`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlotTrainRefusal {
    /// `is_prereq_religion`: only a city with no religion (or no city).
    ReligionPresent,
    /// The religion is not in the city.
    Religion(ReligionType),
    /// The corporation is not active in the city (none in this game's data).
    Corporation(CorporationType),
    /// A sea unit with `prereq_bonuses`: no adjacent water area with resources.
    NoWaterResources,
    /// A land unit with `prereq_bonuses`: its area has resources (C++ quirk).
    AreaHasResources,
    /// A sea unit in a city that is not on the coast (of `min_area` water tiles or more).
    NotCoastal { min_area: i32 },
    /// The area is smaller than the unit's `min_area_size`.
    AreaTooSmall { have: i32, need: i32 },
    /// Outside a city: a sea unit on land.
    NotWater,
    /// Outside a city: a land unit on water.
    NotLand,
    /// Outside a city: a unit of another domain.
    NotInCity,
    /// Not the religion's holy city.
    HolyCity(ReligionType),
    /// The building is not in the city.
    Building(BuildingType),
    /// No building of the class in the city.
    BuildingClass(BuildingClassType),
    /// The resource is not in the city (or connected to the plot).
    Bonus(BonusType),
    /// None of the unit's `prereq_or_bonuses` is in the city.
    AnyBonus,
}

/// Why FfH's Python refuses a unit: [`crate::game::Game::py_train_refusal`] (`cannotTrain`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PyTrainRefusal {
    /// The Beast of Agares in a city of population 5 or less.
    Population { have: i32, need: i32 },
    /// A Settler under the civic.
    UnderCivic(CivicType),
    /// The unit is disabled by the game option.
    GameOption(GameOptionType),
}

/// Why a city cannot train a unit: [`crate::game::Game::city_train_refusal`] (C++
/// `CvCity::canTrain` and `canUpgrade`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TrainRefusal {
    /// `min_level > 0`: the unit comes only from an upgrade of a unit of that level.
    MinLevel(i32),
    Outpost,
    /// A unit with `upgrade_civ` that is neither the city's nor its owner's civilization's unit.
    UpgradeCivUnit,
    /// The card building that trains the unit needs the resource in the city.
    Bonus(BonusType),
    Player(PlayerTrainRefusal),
    /// Every unit it upgrades to is available: the city trains that one (its unit).
    Upgrade(UnitType),
    Plot(PlotTrainRefusal),
    /// The unit is another civilization's (`prereq_civ`).
    PrereqCiv(CivilizationType),
    /// A unit that founds cities under the game option.
    GameOption(GameOptionType),
    /// A world unit that is not the owner's civilization's unit for its class.
    WorldUnitNotOwnCiv,
    /// Neither the city's nor its owner's civilization's unit for its class.
    NotCivUnit,
    /// A unit with `upgrade_civ` that is not that civilization's unit for its class.
    NotUpgradeCivUnit,
    Py(PyTrainRefusal),
}

/// Why player-level rules refuse a building: [`crate::game::Game::player_construct_refusal`]
/// (C++ `CvPlayer::canConstruct`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlayerConstructRefusal {
    /// `production_cost == -1`: the building is never constructed.
    NoProductionCost,
    /// A tech the team lacks (`prereq_and_tech`, `prereq_and_techs`).
    Tech(TechType),
    /// The building is obsolete for the team.
    Obsolete,
    /// The tech its special building type needs.
    SpecialBuildingTech(TechType),
    /// The building's state religion is not the player's.
    StateReligion(ReligionType),
    /// Fewer civilizations have been alive than `num_teams_prereq`.
    NumTeams { have: i32, need: i32 },
    /// Its victory is off in this game.
    VictoryInvalid(VictoryType),
    /// A minor civilization does not build for a victory.
    MinorCiv,
    /// The team's countdown to its victory has started.
    VictoryCountdown(VictoryType),
    /// A limit on instances is reached.
    Limit(Limit),
    /// A prerequisite building that is obsolete is short of the count required.
    ObsoletePrereq { class: BuildingClassType, have: i32, need: i32 },
    /// Nukes under no-nukes (none in this game's data).
    Nukes,
    /// Its special building type is not enabled yet.
    SpecialBuilding(SpecialBuildingType),
    /// Fewer cities than `num_cities_prereq` (`max` = -1), or a player whose most cities
    /// (`max_cities`, below the requirement) is not reached.
    Cities { have: i32, need: i32, max: i32 },
    /// The player's highest unit level is below `unit_level_prereq`.
    UnitLevel { have: i32, need: i32 },
    /// Too few buildings of a class in the player's cities (`prereq_num_of_building_class`).
    BuildingCount { class: BuildingClassType, have: i32, need: i32 },
}

/// Why a building's location refuses it: [`crate::game::Game::city_building_location_refusal`]
/// (C++ `CvCity::isValidBuildingLocation`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LocationRefusal {
    /// `water` (and `river` when `river`): neither on the coast of `min_area` water tiles nor,
    /// when `river`, on a river.
    Coast { river: bool, min_area: i32 },
    /// The area is smaller than `min_area_size`.
    AreaTooSmall { have: i32, need: i32 },
    /// Not on a river.
    River,
}

/// Why FfH's Python refuses a building: [`crate::game::Game::py_construct_refusal`]
/// (`cannotConstruct`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PyConstructRefusal {
    /// A religion's temple, for a player with the trait.
    Trait(TraitType),
    /// The building, under the civic.
    UnderCivic(CivicType),
    /// The Shrine of the Champion, for a civilization with no hero.
    NoHero,
    /// The Shrine of the Champion before the hero of that class was created.
    HeroNotCreated(UnitClassType),
    /// The Shrine of the Champion while the hero lives.
    HeroAlive(UnitClassType),
    /// The building is disabled by the game option.
    GameOption(GameOptionType),
    /// The building, with that state religion.
    StateReligion(ReligionType),
    /// The building, in the capital.
    Capital,
    /// The building, for a player of the alignment.
    Alignment(AlignmentType),
    /// An Altar stage once a later stage exists.
    LaterStage(BuildingType),
    /// The Smugglers' Port without the Smuggling Ring.
    SmugglingRing,
}

/// Why a city cannot construct a building: [`crate::game::Game::city_construct_refusal`]
/// (C++ `CvCity::canConstruct`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ConstructRefusal {
    Outpost,
    /// Not the variant of the city's tier (docs/SUITABILITY.md).
    OtherTier,
    Player(PlayerConstructRefusal),
    /// The city has `CITY_MAX_NUM_BUILDINGS` of it.
    Built { have: i32, max: i32 },
    /// `is_prereq_religion`: the city has religions.
    ReligionPresent { have: i32 },
    /// The owner's state religion (none: `None`) is not in the city.
    StateReligion(Option<ReligionType>),
    /// The religion is not in the city.
    Religion(ReligionType),
    /// A corporation check (none in this game's data).
    Corporation(CorporationType),
    Location(LocationRefusal),
    /// A building of an exclusive class is in the city.
    Exclusive(BuildingClassType),
    /// A government center in a city that is one.
    GovernmentCenter,
    /// The building is in the city's production queue.
    InQueue,
    WorldWonders { have: i32, max: i32 },
    TeamWonders { have: i32, max: i32 },
    NationalWonders { have: i32, max: i32 },
    Buildings { have: i32, max: i32 },
    /// Not the religion's holy city.
    HolyCity(ReligionType),
    /// The resource is not in the city (`prereq_and_bonus`).
    Bonus(BonusType),
    /// The city's latitude is above the building's maximum.
    MaxLatitude { have: i32, max: i32 },
    /// The city's latitude is below the building's minimum.
    MinLatitude { have: i32, min: i32 },
    /// A resource of `prereq_or_bonuses` is not in the city (they are all required).
    OrBonus(BonusType),
    /// The building required in the city is not there.
    Building(BuildingType),
    /// Another civilization's building (`prereq_civ`).
    PrereqCiv(CivilizationType),
    /// The owner lacks the trait.
    Trait(TraitType),
    /// The city's radius already is the building's.
    PlotRadius(i32),
    /// Not the city's civilization's building for its class.
    NotCivBuilding,
    Py(PyConstructRefusal),
}

/// Why player-level rules refuse a project: [`crate::game::Game::player_create_refusal`]
/// (C++ `CvPlayer::canCreate`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlayerCreateRefusal {
    Barbarian,
    /// `production_cost == -1`.
    NoProductionCost,
    Tech(TechType),
    VictoryInvalid(VictoryType),
    MinorCiv,
    VictoryCountdown(VictoryType),
    Limit(Limit),
    /// Nukes under no-nukes (none in this game's data).
    Nukes,
    /// No civilization has created the project.
    AnyoneProject(ProjectType),
    /// The team has fewer of the project than needed.
    ProjectsNeeded { project: ProjectType, have: i32, need: i32 },
}

/// Why FfH's Python and this game's rituals refuse a project: [`crate::game::Game::py_create_refusal`]
/// (`cannotCreate`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PyCreateRefusal {
    /// Purge the Unfaithful without a state religion.
    NoStateReligion,
    /// Stir From Slumber before the player has destroyed a civilization.
    NoPlayersKilled,
    /// The project is not for the civilization.
    Civilization(CivilizationType),
    /// A card ritual whose charge the city holds.
    ChargeHeld,
    /// The Tidecall outside a coastal city (of `min_area` water tiles or more).
    NotCoastal { min_area: i32 },
    /// Birthright Regained before a world ritual has been completed (net of the doublings undone).
    NoRitualCompleted,
    /// A world ritual under the option without world spells.
    GameOption(GameOptionType),
    /// The world ritual's spell is pending (cast it first).
    CastPending,
    /// The Sanctuary's ritual while the Sanctuary holds.
    SanctuaryHolds,
    /// The world ritual is in another city's queue.
    InQueueElsewhere(CityId),
    /// The Tithe without a living named rival.
    NoTitheRival,
}

/// Why a city cannot create a project: [`crate::game::Game::city_create_refusal`] (C++
/// `CvCity::canCreate`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CreateRefusal {
    Outpost,
    /// Another civilization's project.
    PrereqCiv(CivilizationType),
    /// The Armageddon Counter is below the project's.
    GlobalCounter { have: i32, need: i32 },
    Player(PlayerCreateRefusal),
    Py(PyCreateRefusal),
}
