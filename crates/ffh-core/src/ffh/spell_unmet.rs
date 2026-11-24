//! Why a spell's own requirement (its `PyRequirement`, a [`crate::ffh::dispatch::ReqFn`]) refuses
//! a caster: the check that failed, as a value. `None` from a requirement means it may cast.
//! ffh-play writes each as a sentence (`view/reasons_spell.rs`).

use ffh_data::*;

/// The check of a spell's requirement that refuses the caster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellUnmet {
    /// A check the rules make for AI players only.
    AiOnly,

    // The caster.
    /// The caster has this promotion (or race), which the spell excludes.
    CasterHas(PromotionType),
    /// The caster needs one of these promotions.
    CasterLacksAny(PromotionType, PromotionType, PromotionType),
    /// The caster is immune to magic.
    ImmuneToMagic,
    /// The caster is immortal.
    Immortal,
    /// The caster is damaged.
    CasterDamaged,
    /// The caster is not damaged.
    CasterUndamaged,
    /// The caster has no movement.
    CannotMove,
    /// The caster cannot fight.
    CannotFight,
    /// The caster can only defend.
    OnlyDefensive,
    /// The caster has attacked this turn.
    HasAttacked,
    /// The caster sees units a player hides already.
    SeesHidden,
    /// The caster's nationality is hidden already.
    HiddenNationality,
    /// The caster carries units.
    HasCargo,
    /// The caster is carried by another unit.
    IsCargo,
    /// The caster's group holds other units.
    InGroup,
    /// Other units stand on the caster's plot.
    NotAloneOnPlot,
    /// The caster has no free cargo space for land units.
    NoCargoSpace,
    /// The caster has a duration (a summon).
    Summoned,
    /// The caster is a barbarian.
    Barbarian,
    /// The caster is a unit of this combat type.
    UnitCombat(UnitCombatType),
    /// The caster is a unit of this class.
    UnitClass(UnitClassType),
    /// The caster is a unit of this type.
    UnitType(UnitType),
    /// The caster is a special unit of this kind.
    SpecialUnit(SpecialUnitType),
    /// The caster is a world unit (a class with a global limit).
    WorldUnit,
    /// Only a unit of one of these two combat types or of this class casts it.
    OnlyCombatOrClass(UnitCombatType, UnitCombatType, UnitClassType),
    /// Only a familiar casts it.
    NotFamiliar,
    /// The caster has no summoner, or its summoner is gone.
    NoSummoner,
    /// The caster's summoner cannot move onto the caster's plot.
    SummonerCannotEnter,
    /// The player has this many units of this class, the limit.
    ClassLimit(UnitClassType, i32),

    // The plot and its city.
    /// Cast in a city.
    NotInCity,
    /// Not cast in a city.
    InCity,
    /// Not cast in a city of the caster's owner.
    InOwnCity,
    /// Not cast in a city of the caster's team.
    InTeamCity,
    /// Cast in a city of the caster's owner.
    NotInOwnCity,
    /// Cast in a barbarian city.
    NotBarbarianCity,
    /// Not cast in a city of a human player.
    CityOwnerHuman,
    /// A barbarian unit on the plot has a higher rating than the caster.
    StrongerBarbarianOnPlot,
    /// The city's population is below this.
    CityPopulationBelow(i32),
    /// The city has this religion already.
    CityHasReligion(ReligionType),
    /// Cast in a city of this civilization.
    CityCivilization(CivilizationType),
    /// Cast in a settlement.
    NotInSettlement,
    /// The city may not pack up (a capital, a holy city or an outpost).
    CannotPackUp,
    /// Not cast on water.
    OnWater,
    /// Not cast in any civilization's borders.
    PlotOwned,
    /// Not cast in another civilization's borders.
    PlotOwnedByOther,
    /// Cast in the caster's owner's borders.
    NotInOwnBorders,
    /// Not cast on this feature.
    FeatureHere(FeatureType),
    /// Not cast on this improvement.
    ImprovementHere(ImprovementType),
    /// Cast on one of these terrains.
    Terrain(TerrainType, Option<TerrainType>, Option<TerrainType>),
    /// Not cast on this terrain.
    TerrainHere(TerrainType),
    /// This improvement cannot be placed on the plot.
    CannotHaveImprovement(ImprovementType),
    /// This improvement lies within this many plots.
    ImprovementWithin(ImprovementType, i32),
    /// Requires this feature within this many plots.
    NoFeatureWithin(FeatureType, i32),
    /// An enemy unit is visible on the plot.
    VisibleEnemyOnPlot,
    /// Cast on this plot already (its one use is spent).
    PlotSpent,

    // The player.
    /// Only this civilization casts it.
    Civilization(CivilizationType),
    /// Requires this civic.
    Civic(CivicType),
    /// Requires a state religion.
    NoStateReligion,
    /// In another player's city, requires a state religion.
    NoStateReligionAbroad,
    /// The city's owner has a state religion other than the caster's owner's.
    StateReligionDiffers,
    /// Requires a city.
    NoCities,
    /// Requires a capital.
    NoCapital,
    /// The player's full cities are at their limit (this many).
    CityLimit(i32),
    /// Not cast during a golden age.
    GoldenAge,
    /// Not cast while at war with the barbarians.
    AtWarWithBarbarians,
    /// Requires war with the barbarians.
    NotAtWarWithBarbarians,
    /// Requires a war with a civilization.
    NotAtWar,
    /// The barbarians have no units.
    NoBarbarianUnits,
    /// The player has had no great person.
    NoGreatPersonYet,
    /// Cast once a game, and cast already.
    CastOnce,
    /// Requires a city with this building, and cast once a turn.
    BuildingOncePerTurn(BuildingType),
    /// This game option is on.
    GameOption(GameOptionType),
    /// Not in a network game.
    NetworkGame,
    /// The Armageddon Counter is 0.
    ArmageddonCounterZero,
    /// The player's storm is raging already.
    StormActive,
    /// The player has no storm.
    NoStorm,
    /// The storm has moved this turn.
    StormMoved,
    /// A city of the player has production in The Tithe.
    TitheInProgress,

    // Other units.
    /// The commander has cast this turn.
    CommanderHasCast,
    /// The equipment's holder has cast this turn.
    HolderHasCast,
    /// This unit keeps this promotion (its equipment is its own).
    UnitKeeps(UnitType, PromotionType),

    // The great person's card.
    /// The caster holds no card.
    NoCard,
    /// The caster's team knows the card already.
    CardKnown(TechType),
    /// The caster's team lacks the card's needs.
    CardNeeds(TechType),

    /// What the requirement looked for and did not find.
    Missing(Sought),
}

/// What a spell's requirement looks for (a target, a site, a unit) and found none of
/// ([`SpellUnmet::Missing`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sought {
    /// A unit of this type of the caster's owner on the plot without this promotion.
    UnitWithout(UnitType, PromotionType),
    /// This many units of this type of the caster's owner on the plot.
    Units(UnitType, i32),
    /// A unit of this type of the caster's owner on the plot.
    Unit(UnitType),
    /// A unit of this class of the caster's owner on the plot.
    UnitClass(UnitClassType),
    /// Within this many plots: a unit at war with the caster, a damaged unit of this race, or a
    /// unit on this improvement.
    GraveTarget(i32, PromotionType, ImprovementType),
    /// A plot within this many plots whose units are all at war with the caster.
    EnemyOnlyPlot(i32),
    /// A unit of this race within this many plots.
    Race(PromotionType, i32),
    /// A living unit at war with the caster within this many plots, with a resist chance of at
    /// most this percent.
    Resistible(i32, i32),
    /// A damaged unit within this many plots.
    Damaged(i32),
    /// Something to dispel within this many plots (`unit_can_dispel`).
    Dispellable(i32),
    /// A damaged living unit, or a unit with this promotion, on the plot.
    DamagedOrPromotion(PromotionType),
    /// A living unit of the caster's owner on the plot.
    LivingUnit,
    /// A living unit of the caster's owner on the plot without this promotion, of level 6 or
    /// more, or of this type and level 4 or more.
    VampirismCandidate(PromotionType, UnitType),
    /// A religion in the city to remove: not the caster's state religion, not the city's holy one.
    ReligionToRemove,
    /// A great person settled in the city.
    SettledGreatPerson,
    /// A unit of this combat type at war with the caster within this many plots.
    EnemyOfCombat(UnitCombatType, i32),
    /// A unit of another team within this many plots with one of these promotions.
    HiddenUnit(i32, [PromotionType; 4]),
    /// A unit at war with the caster within this many plots, not immune to fear.
    FearableEnemy(i32),
    /// One of these two improvements on the plot, or a plot within 1 touched by Hell: one of
    /// these terrains (without plot counters), else a plot counter above 0.
    Sanctify(ImprovementType, ImprovementType, Option<[TerrainType; 4]>),
    /// This feature or improvement within 1 plot, or else this terrain on the plot.
    FlamesOrTerrain(FeatureType, ImprovementType, TerrainType),
    /// Equipment of another team on the plot (a unit's or the city's).
    Equipment,
    /// A unit of the caster's owner with this promotion on the plot.
    Holder(Option<PromotionType>),
    /// A unit at war with the caster within this many plots that can attack, in its domain.
    TauntTarget(i32),
    /// A level-1 sphere promotion of the caster's to teach.
    SphereToTeach,
    /// A living unit of the caster's owner on the plot, not an animal or a bird, without one of
    /// the caster's spheres.
    Learner,
    /// A damaged siege or naval unit, or a unit with this promotion, on the plot.
    RepairTarget(UnitCombatType, UnitCombatType, PromotionType),
    /// This many cities with this religion, not capitals, of civilizations other than this one.
    ReligionCities(i32, ReligionType, CivilizationType),
    /// A caster of the familiar's owner on the plot to bind to.
    FamiliarCaster,
    /// A city of the caster's owner it stands in whose Gilded Legion has not chosen.
    LegionCity,
    /// A civilization the Gilded Legion may choose.
    LegionCivilization,
    /// An enemy land unit next to the caster to buy out.
    BuyOutTarget,
    /// A walled enemy city next to the caster.
    WalledCity,
    /// Another unit of the caster's owner on the plot that can attack, without false colours.
    FalseColoursUnit,
    /// A packed city, and a city site within this many plots of where it packed up.
    UnpackSite(i32),
    /// Two free city sites for the Patrians.
    PatrianSites,
    /// Two free city sites within this many plots of the player's borders.
    BorderSites(i32),
    /// Two free city sites in an unclaimed forest region of at least this many plots.
    FeySites(i32),
    /// Two free city sites on unclaimed land.
    UnclaimedSites,
    /// A land caster in a city of its owner's with this building.
    GateCity(BuildingType),
    /// This many City Ruins a city can stand on (and a capital).
    NecropolisSites(i32),
    /// Two free hill sites with this many unowned hills or peaks within this many plots.
    HillSites(i32, i32),
    /// Two free coastal city sites on unowned land.
    CoastalSites,
    /// A state religion, a hero of it not in play, and a city.
    FaithfulHero,
    /// A city of the player whose build Grand Order can advance.
    GrandOrderCity,
    /// City Ruins the player can refound.
    RuinsToRefound,
    /// A capital, and no Sylvarith alive.
    DragonsOathCapital,
    /// A plot of the player's for the decree to improve.
    DecreePlot,
    /// A land caster on a hill of its owner's that is no tunnel's end.
    DelveHill,
    /// A land caster on a tunnel's end with all its moves, the other end open to it.
    DelveEnd,
}

/// `None` when `ok`, else `Some(r)`: a requirement's last check.
#[inline]
pub fn need(ok: bool, r: SpellUnmet) -> Option<SpellUnmet> {
    if ok { None } else { Some(r) }
}
