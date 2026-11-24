//! Why the rules refuse a unit order: the reason a gate (`plot_can_build`, `unit_can_build`,
//! `unit_can_bombard`, `unit_can_promote`, ...) returns from its refusal twin
//! (`plot_build_refusal`, `unit_build_refusal`, `unit_bombard_refusal`, `unit_promote_refusal`,
//! ...), one variant per check, in the gate's order. The gate itself is the twin's `.is_none()`.
//! The sentences are written by the client (ffh-play, `view/reasons_orders.rs`).

use crate::city::refusal::ConstructRefusal;
use ffh_data::*;

/// Why an improvement cannot be on a plot: a check of `plot_can_have_improvement` (C++
/// `CvPlot::canHaveImprovement`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImprovementRefusal {
    /// A city is on the plot.
    City,
    /// The terrain takes no improvement (Crater, Sea Ice).
    Unimprovable,
    /// The plot is impassable (a peak takes only an improvement that requires one).
    Impassable,
    /// The improvement requires a peak.
    Peak,
    /// The improvement goes on water.
    OnWater,
    /// The improvement goes on land.
    OnLand,
    /// The plot's feature takes no improvement.
    Feature(FeatureType),
    /// The improvement is not built next to fresh water.
    FreshWater,
    /// The improvement requires flatlands.
    Flatlands,
    /// The improvement requires a feature.
    NeedsFeature,
    /// Nothing on the plot makes the improvement valid (its terrains, features, hills, fresh
    /// water, river side, bonuses).
    Site,
    /// The improvement requires a river side not shared with the same improvement.
    RiverSide,
    /// The plot's natural yield of this kind is below the improvement's need.
    NatureYield(YieldType, i32),
    /// The improvement requires irrigation.
    Irrigation,
}

/// Why a build cannot be done on a plot: a check of `plot_can_build` (C++ `CvPlot::canBuild`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlotBuildRefusal {
    /// Building is disabled on the plot.
    Disabled,
    /// Nothing is built on Sea Ice.
    SeaIce,
    /// The rules' `canBuild` script refuses it.
    Script,
    /// The build's improvement cannot be on the plot.
    Improvement(ImprovementRefusal),
    /// The improvement goes next to a city of a civilization at war with the builder.
    EnemyCityAdjacent,
    /// The improvement goes on a route.
    NeedsRoute,
    /// A canal goes next to water or another canal.
    CanalNotConnected,
    /// A causeway goes next to land.
    CausewayNeedsLand,
    /// A Pirate Cove goes on an empty coast plot of the builder, 2 plots from another cove.
    PirateCoveSite,
    /// The improvement goes outside every border.
    ClaimOwned,
    /// A city is within this many plots.
    NearCity(i32),
    /// A permanent improvement is on the plot.
    Permanent(ImprovementType),
    /// The improvement is on the plot already.
    Already(ImprovementType),
    /// The improvement on the plot upgrades to the same improvement.
    SameUpgrade(ImprovementType),
    /// The improvement is not built in another civilization's borders.
    ForeignBorders,
    /// The improvement is built in the builder's own borders only.
    OwnBorders,
    /// A route as good is on the plot.
    Route(RouteType),
    /// The route requires this bonus connected next to the plot.
    RouteBonus(BonusType),
    /// The route requires one of its bonuses connected next to the plot.
    RouteOrBonus(RouteType),
    /// The feature is in another civilization's borders, at peace.
    FeatureOwner(FeatureType),
    /// The build does nothing on this plot (no improvement or route of it, no feature it removes).
    Nothing,
}

/// Why a unit or a player cannot do a build: a check of `unit_can_build` (C++ `CvUnit::canBuild`)
/// or `player_can_build` (C++ `CvPlayer::canBuild`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BuildRefusal {
    /// The unit does not do this build.
    NotBuilder,
    /// The plot's own check.
    Plot(PlotBuildRefusal),
    /// The build is this civilization's own.
    Civilization(CivilizationType),
    /// The build requires this tech.
    Tech(TechType),
    /// Removing this feature requires this tech.
    FeatureTech(FeatureType, TechType),
    /// Gold to start the build.
    Gold { have: i32, need: i32 },
    /// The unit cannot act from this plot.
    Domain,
    /// Automated units build no forts.
    AutomatedFort,
}

/// Why a unit cannot start a mission: a check of the mission's gate (`unit_can_sleep`,
/// `unit_can_bombard`, `unit_can_spread`, ...), in its order.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MissionRefusal {
    /// The unit's group is fortified or asleep already.
    Waiting,
    /// The unit is dying.
    Dying,
    /// The unit has cast this turn.
    HasCast,
    /// No city on the plot.
    NoCity,
    /// The city is not of the unit's team.
    ForeignCity,
    /// The city is not the unit owner's.
    NotOwnCity,
    /// The unit cannot enter the plot's territory.
    CannotEnter,
    /// The unit has no moves left.
    NoMoves,
    /// The unit fortifies instead (sleep).
    Fortifies,
    /// The unit does not fortify.
    NotFortifyable,
    /// Not in boats.
    InBoats,
    /// The unit is not hurt.
    NotHurt,
    /// The unit heals nothing on the plot.
    NoHeal,
    /// The unit cannot defend.
    CannotDefend,
    /// The unit has no bombard rate.
    NoBombard,
    /// The unit has no attack left this turn.
    NoAttackLeft,
    /// The unit has attacked this turn.
    Attacked,
    /// The unit is loaded on a transport.
    Cargo,
    /// No bombardable city within reach.
    NoBombardTarget,
    /// An air unit (range strike).
    Air,
    /// The unit has no range.
    NoRange,
    /// The unit has no ranged strength.
    NoStrength,
    /// The unit cannot fight.
    CannotFight,
    /// The target is off the map.
    OffMap,
    /// The plot the strike is made from is not visible.
    NotVisible,
    /// The target is beyond the unit's range.
    OutOfRange(i32),
    /// No unit to strike on the target.
    NoTarget,
    /// The target is out of sight.
    NoLineOfSight,
    /// The unit does not pillage.
    NotPillager,
    /// No pillaging a city.
    PillageCity,
    /// No improvement and no route to pillage.
    NothingToPillage,
    /// The improvement is permanent.
    Permanent(ImprovementType),
    /// Pillage in your own land or the land of a civilization you are at war with.
    PillageNotAtWar,
    /// The unit cannot act from this plot.
    Domain,
    /// Barbarians do not pillage a lair.
    Lair(ImprovementType),
    /// Fields are raided, not pillaged.
    Fields,
    /// Not a warship (Patrol).
    NotWarship,
    /// Barbarians do not patrol.
    Barbarian,
    /// Patrol on a sea.
    NotSea,
    /// No Patrol in a city.
    PatrolCity,
    /// No religion given.
    NoReligion,
    /// The unit does not spread this religion.
    NoSpread(ReligionType),
    /// The city has this religion already.
    HasReligion(ReligionType),
    /// The city's owner lets in only its state religion.
    StateReligionOnly,
    /// The rules' `cannotSpreadReligion` script refuses it.
    SpreadScript,
    /// No corporation given.
    NoCorporation,
    /// The owner has not this corporation active.
    InactiveCorporation(CorporationType),
    /// The unit does not spread this corporation.
    NoCorporationSpread(CorporationType),
    /// The city has this corporation already.
    HasCorporation(CorporationType),
    /// The city's owner has not this corporation active.
    ForeignInactiveCorporation(CorporationType),
    /// The city is the headquarters of a competing corporation.
    CompetingHeadquarters(CorporationType),
    /// The city has none of the corporation's bonuses.
    CorporationBonus(CorporationType),
    /// Gold to spread the corporation.
    Gold { have: i32, need: i32 },
    /// No specialist given.
    NoSpecialist,
    /// The unit does not join a city as this specialist.
    NotGreatPerson(SpecialistType),
    /// The city takes no one.
    CityCannotJoin,
    /// No building given.
    NoBuilding,
    /// The city has the building already.
    HasBuilding(BuildingType),
    /// The unit does not build this building.
    NotConstructor(BuildingType),
    /// The city cannot construct the building.
    Construct(BuildingType, ConstructRefusal),
    /// No tech for the unit to discover.
    NoDiscoveryTech,
    /// The unit adds no research.
    NoResearch,
    /// The unit adds no production here.
    NoHurry,
    /// The build finishes next turn.
    OneTurnLeft,
    /// Only a building or a ritual is hurried.
    HurryOnly,
    /// The trade mission pays no gold here.
    NoTradeGold,
    /// Trade missions go to another team's city.
    OwnTeamCity,
    /// The unit makes no culture.
    NoCulture,
    /// The unit does not start golden ages.
    NotGoldenAge,
    /// Great people of different kinds needed for a golden age.
    GoldenAgeUnits { need: i32, have: i32 },
    /// No unit of yours on the plot to lead.
    NoneToLead,
    /// The unit to lead cannot take the leader's promotion.
    Lead(PromoteRefusal),
}

/// Why a unit cannot acquire a promotion: a check of `unit_can_acquire_promotion` (C++
/// `CvUnit::canAcquirePromotion` and `isPromotionValid`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AcquireRefusal {
    /// The unit has it already.
    Has,
    /// It requires this promotion.
    Prereq(PromotionType),
    /// It is never picked (given by the rules).
    NotPickable,
    /// A race.
    Race,
    /// Equipment.
    Equipment,
    /// It requires this level.
    Level(i32),
    /// It requires this promotion (the AND prerequisite).
    PrereqAnd(PromotionType),
    /// It requires one of these promotions.
    PrereqOr(PromotionType, Option<PromotionType>, Option<PromotionType>, Option<PromotionType>),
    /// It requires this bonus.
    Bonus(BonusType),
    /// The unit's promotion makes it immune to this one.
    Immune(PromotionType),
    /// The unit must be alive.
    Alive,
    /// It requires this tech.
    Tech(TechType),
    /// It requires this state religion.
    StateReligion(ReligionType),
    /// Not for this unit (its unit combat, class or abilities).
    NotForUnit,
    /// The withdrawal chance would pass this maximum.
    Withdrawal(i32),
    /// The interception chance would pass this maximum.
    Interception(i32),
    /// The evasion chance would pass this maximum.
    Evasion(i32),
    /// The unit learnt a spell by a promotion already.
    SpellClash,
}

/// Why a unit cannot take a promotion now: a check of `unit_can_promote` (C++
/// `CvUnit::canPromote`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PromoteRefusal {
    /// A leader does not promote itself.
    LeaderSelf,
    /// No promotion given.
    NoPromotion,
    /// The promotion's own checks.
    Acquire(PromotionType, AcquireRefusal),
    /// A leader's promotion, given by a leader of it.
    Leader(PromotionType),
    /// The unit has not the experience for a promotion.
    NotReady,
}

/// Why a unit cannot upgrade: a check of `unit_can_upgrade` (C++ `CvUnit::canUpgrade`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UpgradeRefusal {
    /// No unit type given.
    NoUnit,
    /// The unit has no moves left.
    NoMoves,
    /// Units upgrade in their team's territory.
    Territory,
    /// Gold for the upgrade.
    Gold { have: i32, need: i32 },
    /// The new type requires this level.
    Level(i32),
    /// No unit upgrades to the new type.
    Disabled,
    /// The new type's class is at its limit.
    ClassMaxed,
    /// AI only: a missionary below level 2.
    AiMissionary,
    /// AI only: a permanent defender to a unit without defensive bonuses.
    AiDefender,
    /// The unit has no upgrade to the new type here (its upgrade path, its cargo, the nearest
    /// city).
    NoUpgrade,
}

/// Why a unit cannot be automated: a check of `unit_can_automate` (C++ `CvUnit::canAutomate`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AutomateRefusal {
    /// No automation given.
    NoAutomate,
    /// Only the head of its group is automated.
    NotGroupHead,
    /// This automation is not in this game.
    Disabled,
    /// The unit cannot explore.
    Explore,
    /// An air unit that cannot recon.
    AirRecon,
    /// Only a missionary spreads its religion by itself.
    NotMissionary,
    /// Not a terraformer.
    NotTerraformer,
}

/// Why a unit cannot be gifted: a check of `unit_can_gift` (C++ `CvUnit::canGift`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GiftRefusal {
    /// Gifting is disabled (FfH 0.41).
    Disabled,
    /// The plot has no owner.
    NotOwned,
    /// The plot is the unit owner's.
    OwnPlot,
    /// An enemy unit is in view.
    Enemy,
    /// The unit cannot stand on the plot.
    Domain,
    /// A unit that spreads a corporation.
    Corporation,
    /// The transport is not of the receiver's team.
    Transport,
    /// The receiver's class limit.
    ClassMaxed,
    /// The receiver does not accept it.
    NotAccepted,
    /// A temporary player.
    TempPlayer,
    /// At war with the receiver.
    War,
}

/// Why a unit cannot load onto a transport: a check of `unit_can_load_unit` (C++
/// `CvUnit::canLoadUnit`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoadRefusal {
    /// A unit does not load onto itself.
    Itself,
    /// A caravan of a trade route.
    InRoute,
    /// The transport is of another team.
    OtherTeam,
    /// The unit is loaded already.
    Cargo,
    /// The unit carries units.
    HasCargo,
    /// The transport is itself loaded.
    TransportCargo,
    /// The transport has no room for the unit.
    NoRoom,
    /// The transport is not on the plot.
    NotHere,
    /// One has hidden nationality and the other not.
    HiddenNationality,
    /// The unit loads in a city of its team only.
    CityLoad,
}

/// Why a unit cannot unload: a check of `unit_can_unload` (C++ `CvUnit::canUnload`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnloadRefusal {
    /// The unit is not loaded.
    NotLoaded,
    /// The unit cannot stand on the plot.
    Domain,
    /// The city's air capacity is full.
    AirCapacity,
}

/// Why a unit cannot be disbanded: a check of `unit_can_scrap` (C++ `CvUnit::canScrap`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ScrapRefusal {
    /// A battle is under way on the plot.
    Fighting,
    /// A temporary player.
    TempPlayer,
    /// Equipment is not disbanded.
    Equipment,
}
