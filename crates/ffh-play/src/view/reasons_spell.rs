//! Why a spell refuses a caster, for the checks [`super::reasons::spell_refusal_text`] hands on:
//! the spell's own requirement ([`SpellUnmet`]), a spell none of whose effects applies
//! ([`NoEffect`]), and the creation of a unit or a building ([`CreateUnitRefusal`],
//! [`CreateBuildingRefusal`]). One bare sentence each (CLAUDE.md, "Rules text and tooltips").

use std::borrow::Cow;

use crate::names::display;
use ffh_core::ffh::spell_unmet::{Sought, SpellUnmet};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::unit::{CreateBuildingRefusal, CreateUnitRefusal, NoEffect};
use ffh_data::*;

/// `on the plot`, `within 1 plot`, `within 3 plots`.
fn within(n: i32) -> String {
    match n {
        n if n <= 0 => "on the plot".to_string(),
        1 => "within 1 plot".to_string(),
        n => format!("within {n} plots"),
    }
}

/// `A`, `A or B`, `A, B or C`.
fn or_names(items: &[Cow<'_, str>]) -> String {
    match items {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} or {}", rest.join(", "), last),
    }
}

/// `a`/`an` for a name.
fn article(s: &str) -> &'static str {
    if s.starts_with(['A', 'E', 'I', 'O', 'U', 'a', 'e', 'i', 'o', 'u']) { "an" } else { "a" }
}

/// `a Wolf`, `an Adept`.
fn a(s: &str) -> String {
    format!("{} {s}", article(s))
}

/// The first letter in upper case.
fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Why the spell's own requirement refuses unit `u`, as a sentence.
pub fn spell_unmet_text(g: &Game, u: UnitId, _spell: SpellType, r: SpellUnmet) -> String {
    let d = g.d;
    let name = display(d, g.units[u].unit_type);
    match r {
        SpellUnmet::AiOnly => "Not cast by an AI player here.".to_string(),
        SpellUnmet::CasterHas(p) => format!("Not for a unit with {}.", display(d, p)),
        SpellUnmet::CasterLacksAny(a1, a2, a3) => format!("Requires {}.", or_names(&[display(d, a1), display(d, a2), display(d, a3)])),
        SpellUnmet::ImmuneToMagic => format!("{name} is immune to magic."),
        SpellUnmet::Immortal => format!("{name} is immortal."),
        SpellUnmet::CasterDamaged => format!("{name} is damaged."),
        SpellUnmet::CasterUndamaged => format!("{name} is not damaged."),
        SpellUnmet::CannotMove => format!("{name} has no movement."),
        SpellUnmet::CannotFight => format!("{name} cannot fight."),
        SpellUnmet::OnlyDefensive => format!("{name} can only defend."),
        SpellUnmet::HasAttacked => format!("{name} has attacked this turn."),
        SpellUnmet::SeesHidden => format!("{name} sees hidden units already."),
        SpellUnmet::HiddenNationality => format!("{name} has hidden nationality already."),
        SpellUnmet::HasCargo => format!("{name} carries units."),
        SpellUnmet::IsCargo => format!("{name} is carried by another unit."),
        SpellUnmet::InGroup => format!("{name} must be alone in its group."),
        SpellUnmet::NotAloneOnPlot => format!("{name} must be the only unit on the plot."),
        SpellUnmet::NoCargoSpace => format!("{name} has no free cargo space for land units."),
        SpellUnmet::Summoned => "Not cast by a unit with a duration.".to_string(),
        SpellUnmet::Barbarian => "Not cast by barbarians.".to_string(),
        SpellUnmet::UnitCombat(c) => format!("Not cast by {} unit.", a(&display(d, c))),
        SpellUnmet::UnitClass(c) => format!("Not cast by {}.", a(&display(d, c))),
        SpellUnmet::UnitType(t) => format!("Not cast by {}.", display(d, t)),
        SpellUnmet::SpecialUnit(s) => format!("Not cast by {} unit.", a(&display(d, s))),
        SpellUnmet::WorldUnit => "Not cast by a world unit.".to_string(),
        SpellUnmet::OnlyCombatOrClass(c1, c2, k) => format!("Only {} or {} unit or {} casts it.", a(&display(d, c1)), display(d, c2), a(&display(d, k))),
        SpellUnmet::NotFamiliar => "Only a familiar casts it.".to_string(),
        SpellUnmet::NoSummoner => format!("{name} has no living summoner."),
        SpellUnmet::SummonerCannotEnter => "The summoner cannot move onto this plot.".to_string(),
        SpellUnmet::ClassLimit(c, n) => format!("Limit reached: {n} {}.", display(d, c)),

        SpellUnmet::NotInCity => "Cast in a city.".to_string(),
        SpellUnmet::InCity => "Not cast in a city.".to_string(),
        SpellUnmet::InOwnCity => "Not cast in a city of yours.".to_string(),
        SpellUnmet::InTeamCity => "Not cast in a city of your team.".to_string(),
        SpellUnmet::NotInOwnCity => "Cast in a city of yours.".to_string(),
        SpellUnmet::NotBarbarianCity => "Cast in a barbarian city.".to_string(),
        SpellUnmet::CityOwnerHuman => "Not cast in a city of a human player.".to_string(),
        SpellUnmet::StrongerBarbarianOnPlot => format!("A barbarian unit on the plot is stronger than {name}."),
        SpellUnmet::CityPopulationBelow(n) => format!("The city's population must be at least {n}."),
        SpellUnmet::CityHasReligion(r) => format!("The city has {} already.", display(d, r)),
        SpellUnmet::CityCivilization(c) => format!("Cast in a city of the {}.", display(d, c)),
        SpellUnmet::NotInSettlement => "Cast in a settlement.".to_string(),
        SpellUnmet::CannotPackUp => "Not for a capital, a holy city or an outpost.".to_string(),
        SpellUnmet::OnWater => "Not cast on water.".to_string(),
        SpellUnmet::PlotOwned => "Cast outside every civilization's borders.".to_string(),
        SpellUnmet::PlotOwnedByOther => "Not cast in another civilization's borders.".to_string(),
        SpellUnmet::NotInOwnBorders => "Cast in your borders.".to_string(),
        SpellUnmet::FeatureHere(f) => format!("Not cast on {}.", display(d, f)),
        SpellUnmet::ImprovementHere(i) => format!("Not cast on {}.", display(d, i)),
        SpellUnmet::Terrain(t1, t2, t3) => {
            let names: Vec<Cow<'_, str>> = [Some(t1), t2, t3].into_iter().flatten().map(|t| display(d, t)).collect();
            format!("Cast on {}.", or_names(&names))
        }
        SpellUnmet::TerrainHere(t) => format!("Not cast on {}.", display(d, t)),
        SpellUnmet::CannotHaveImprovement(i) => format!("{} cannot be placed on this plot.", display(d, i)),
        SpellUnmet::ImprovementWithin(i, n) => format!("Not cast {} of {}.", within(n), display(d, i)),
        SpellUnmet::NoFeatureWithin(f, n) => format!("Requires {} {}.", display(d, f), within(n)),
        SpellUnmet::VisibleEnemyOnPlot => "An enemy unit is in view on the plot.".to_string(),
        SpellUnmet::PlotSpent => "Cast on this plot already.".to_string(),

        SpellUnmet::Civilization(c) => format!("Only the {} cast it.", display(d, c)),
        SpellUnmet::Civic(c) => format!("Requires {}.", display(d, c)),
        SpellUnmet::NoStateReligion => "Requires a state religion.".to_string(),
        SpellUnmet::NoStateReligionAbroad => "Requires a state religion in a city not yours.".to_string(),
        SpellUnmet::StateReligionDiffers => "The city's owner must have your state religion.".to_string(),
        SpellUnmet::NoCities => "Requires a city.".to_string(),
        SpellUnmet::NoCapital => "Requires a capital.".to_string(),
        SpellUnmet::CityLimit(n) => format!("Limit reached: {n} full cities."),
        SpellUnmet::GoldenAge => "Not cast during a golden age.".to_string(),
        SpellUnmet::AtWarWithBarbarians => "Not cast while at war with the barbarians.".to_string(),
        SpellUnmet::NotAtWarWithBarbarians => "Requires war with the barbarians.".to_string(),
        SpellUnmet::NotAtWar => "Requires a war with a civilization.".to_string(),
        SpellUnmet::NoBarbarianUnits => "The barbarians have no units.".to_string(),
        SpellUnmet::NoGreatPersonYet => "Requires a great person born to your civilization.".to_string(),
        SpellUnmet::CastOnce => "Cast already: once a game.".to_string(),
        SpellUnmet::BuildingOncePerTurn(b) => format!("Requires {}: once a turn.", display(d, b)),
        SpellUnmet::GameOption(o) => format!("Not with the {} option.", display(d, o)),
        SpellUnmet::NetworkGame => "Not in a network game.".to_string(),
        SpellUnmet::ArmageddonCounterZero => "The Armageddon Counter is 0.".to_string(),
        SpellUnmet::StormActive => "Your storm is raging already.".to_string(),
        SpellUnmet::NoStorm => "Requires your storm.".to_string(),
        SpellUnmet::StormMoved => "The storm has moved this turn.".to_string(),
        SpellUnmet::TitheInProgress => "A city of yours is building The Tithe.".to_string(),

        SpellUnmet::CommanderHasCast => "The commander has cast this turn.".to_string(),
        SpellUnmet::HolderHasCast => "The holder of the equipment has cast this turn.".to_string(),
        SpellUnmet::UnitKeeps(t, p) => format!("{} keeps {}.", display(d, t), display(d, p)),

        SpellUnmet::NoCard => format!("{name} holds no card."),
        SpellUnmet::CardKnown(t) => format!("You know {} already.", display(d, t)),
        SpellUnmet::CardNeeds(t) => format!("Requires the needs of {}.", display(d, t)),

        SpellUnmet::Missing(s) => sought_text(g, s),
    }
}

/// What a requirement looked for and did not find, as a sentence.
fn sought_text(g: &Game, s: Sought) -> String {
    let d = g.d;
    match s {
        Sought::UnitWithout(t, p) => format!("Requires {} of yours without {} on the plot.", a(&display(d, t)), display(d, p)),
        Sought::Units(t, n) => format!("Requires {n} units of type {} of yours on the plot.", display(d, t)),
        Sought::Unit(t) => format!("Requires {} of yours on the plot.", a(&display(d, t))),
        Sought::UnitClass(c) => format!("Requires {} of yours on the plot.", a(&display(d, c))),
        Sought::GraveTarget(n, race, imp) => {
            format!("Requires {}: a unit at war with you, a damaged {} unit, or a unit on {}.", within(n), display(d, race), display(d, imp))
        }
        Sought::EnemyOnlyPlot(n) => format!("Requires a plot {} whose units are all at war with you.", within(n)),
        Sought::Race(p, n) => format!("Requires {} unit {}.", a(&display(d, p)), within(n)),
        Sought::Resistible(n, max) => format!("Requires a living unit at war with you {} with a resist chance of at most {max}%.", within(n)),
        Sought::Damaged(n) => format!("Requires a damaged unit {}.", within(n)),
        Sought::Dispellable(n) => format!("Nothing {} to dispel.", within(n)),
        Sought::DamagedOrPromotion(p) => format!("Requires a damaged living unit or a unit with {} on the plot.", display(d, p)),
        Sought::LivingUnit => "Requires a living unit of yours on the plot.".to_string(),
        Sought::VampirismCandidate(p, t) => {
            format!("Requires a living unit of yours on the plot without {}: level 6 or more, or {} of level 4 or more.", display(d, p), a(&display(d, t)))
        }
        Sought::ReligionToRemove => "The city has no religion to remove: none besides your state religion that is not its holy city's.".to_string(),
        Sought::SettledGreatPerson => "The city has no settled great person.".to_string(),
        Sought::EnemyOfCombat(c, n) => format!("Requires {} unit at war with you {}.", a(&display(d, c)), within(n)),
        Sought::HiddenUnit(n, ps) => {
            let names: Vec<Cow<'_, str>> = ps.iter().map(|&p| display(d, p)).collect();
            format!("Requires a unit of another team {} with {}.", within(n), or_names(&names))
        }
        Sought::FearableEnemy(n) => format!("Requires a unit at war with you {} that is not immune to fear.", within(n)),
        Sought::Sanctify(i1, i2, Some(ts)) => {
            let names: Vec<Cow<'_, str>> = ts.iter().map(|&t| display(d, t)).collect();
            format!("Requires {} or {} on the plot, or {} within 1 plot.", display(d, i1), display(d, i2), or_names(&names))
        }
        Sought::Sanctify(i1, i2, None) => format!("Requires {} or {} on the plot, or a plot within 1 touched by Hell.", display(d, i1), display(d, i2)),
        Sought::FlamesOrTerrain(f, i, t) => format!("Requires {} or {} within 1 plot, or {} on the plot.", display(d, f), display(d, i), display(d, t)),
        Sought::Equipment => "Requires equipment of another team on the plot.".to_string(),
        Sought::Holder(Some(p)) => format!("Requires a unit of yours with {} on the plot.", display(d, p)),
        Sought::Holder(None) => "Requires a unit of yours with the equipment on the plot.".to_string(),
        Sought::TauntTarget(n) => format!("Requires a unit at war with you {} that can attack, of the same domain.", within(n)),
        Sought::SphereToTeach => "Requires a sphere of the first rank to teach.".to_string(),
        Sought::Learner => "Requires a living unit of yours on the plot, not an animal or a bird, that lacks one of your spheres.".to_string(),
        Sought::RepairTarget(c1, c2, p) => format!("Requires a damaged {} or {} unit, or a unit with {}, on the plot.", display(d, c1), display(d, c2), display(d, p)),
        Sought::ReligionCities(n, r, c) => format!("Requires {n} cities with {} that are not capitals, outside the {}.", display(d, r), display(d, c)),
        Sought::FamiliarCaster => "Requires a caster of yours with magic on the plot that no familiar is bound to.".to_string(),
        Sought::LegionCity => "Cast in a city of yours whose Gilded Legion has not chosen.".to_string(),
        Sought::LegionCivilization => "Requires a living civilization you have met, not barbarian, other than yours.".to_string(),
        Sought::BuyOutTarget => "Requires an enemy land unit next to the caster that you see, at a price you can pay.".to_string(),
        Sought::WalledCity => "Requires a city at war with you next to the caster, with Sentinels standing.".to_string(),
        Sought::FalseColoursUnit => "Requires another unit of yours on the plot that can attack, without false colours.".to_string(),
        Sought::UnpackSite(n) => format!("Requires a city site within {n} plots of where the city packed up."),
        Sought::PatrianSites => "Requires two free city sites for the Patrians.".to_string(),
        Sought::BorderSites(n) => format!("Requires two free city sites within {n} plots of your borders."),
        Sought::FeySites(n) => format!("Requires an unclaimed forest region of {n} plots or more with two free city sites."),
        Sought::UnclaimedSites => "Requires two free city sites on unclaimed land.".to_string(),
        Sought::GateCity(b) => format!("Requires a land unit in a city of yours with {}.", display(d, b)),
        Sought::NecropolisSites(n) => format!("Requires a capital and {n} City Ruins where a city can stand."),
        Sought::HillSites(n, r) => format!("Requires two free hill sites with {n} unowned hills or peaks within {r} plots."),
        Sought::CoastalSites => "Requires two free coastal city sites on unowned land.".to_string(),
        Sought::FaithfulHero => "Requires a state religion, a hero of it not in play, and a city.".to_string(),
        Sought::GrandOrderCity => "Requires a city of yours with a build that production can advance.".to_string(),
        Sought::RuinsToRefound => "Requires City Ruins that you can refound.".to_string(),
        Sought::DragonsOathCapital => "Requires a capital, and no Sylvarith alive.".to_string(),
        Sought::DecreePlot => "Requires a plot of yours for the decree to improve.".to_string(),
        Sought::DelveHill => "Requires a land unit on a hill of yours that is no tunnel's end.".to_string(),
        Sought::DelveEnd => "Requires a land unit with all its moves on a tunnel's end, the other end open to it.".to_string(),
    }
}

/// One effect's part of a [`NoEffect`] sentence, without its full stop (lower case first where
/// it starts with a word of this text).
fn no_effect_part(g: &Game, u: UnitId, spell: SpellType, e: NoEffect) -> String {
    let d = g.d;
    let info = &d.spells[spell];
    let name = display(d, g.units[u].unit_type);
    let range = info.range;
    match e {
        NoEffect::Nothing => format!("{} has no effect", display(d, spell)),
        NoEffect::AddPromotion(_) => {
            let names: Vec<Cow<'_, str>> = [info.add_promotion_type1, info.add_promotion_type2, info.add_promotion_type3].into_iter().flatten().map(|p| display(d, p)).collect();
            if info.buff_caster_only {
                format!("{name} has {} already", names.join(" and "))
            } else {
                format!("no unit {} can gain {}", within(range), or_names(&names))
            }
        }
        NoEffect::RemovePromotion(_) => {
            let promotions = [info.remove_promotion_type1, info.remove_promotion_type2, info.remove_promotion_type3];
            let hidden = d.defines.hidden_nationality_promotion;
            if g.unit_plot_opt(u).is_some_and(|q| g.plot_is_visible_enemy_unit(q, g.units[u].owner))
                && promotions.iter().any(|p| p.map_or(-1, |p| p.idx() as i32) == hidden)
            {
                return "an enemy unit is in view on the plot".to_string();
            }
            let names: Vec<Cow<'_, str>> = promotions.into_iter().flatten().map(|p| display(d, p)).collect();
            if info.buff_caster_only {
                format!("{name} has no {}", or_names(&names))
            } else {
                format!("no unit {} has {}", within(range), or_names(&names))
            }
        }
        NoEffect::Feature(f) => format!("{} cannot be placed on this plot", display(d, f)),
        NoEffect::Improvement(i) => format!("{} cannot be placed on this plot", display(d, i)),
        NoEffect::SpreadReligion(r) => format!("{} needs a city here without it", display(d, r)),
        NoEffect::Dispel => format!("nothing {} to dispel", within(range)),
        NoEffect::Immobile => format!("no unit {} to hold in place", within(range)),
        NoEffect::Push => format!("no unit outside a city {} to push", within(range)),
        NoEffect::Several => String::new(),
    }
}

/// Why none of the spell's effects applies, as a sentence.
pub fn no_effect_text(g: &Game, u: UnitId, spell: SpellType, e: NoEffect) -> String {
    if e != NoEffect::Several {
        return format!("{}.", capital(&no_effect_part(g, u, spell, e)));
    }
    let info = &g.d.spells[spell];
    let parts: Vec<String> = [
        info.add_promotion_type1.map(NoEffect::AddPromotion),
        info.remove_promotion_type1.map(NoEffect::RemovePromotion),
        info.create_feature_type.map(NoEffect::Feature),
        info.create_improvement_type.map(NoEffect::Improvement),
        info.spread_religion.map(NoEffect::SpreadReligion),
        info.dispel.then_some(NoEffect::Dispel),
        (info.immobile_turns > 0).then_some(NoEffect::Immobile),
        info.push.then_some(NoEffect::Push),
    ]
    .into_iter()
    .flatten()
    .map(|e| no_effect_part(g, u, spell, e))
    .collect();
    format!("No effect here: {}.", parts.join("; "))
}

/// Why the spell's unit cannot be created, as a sentence.
pub fn create_unit_text(r: CreateUnitRefusal) -> String {
    match r {
        CreateUnitRefusal::OwnKind => "A summon does not summon its own kind.".to_string(),
        CreateUnitRefusal::VisibleEnemy => "An enemy unit is in view on the plot.".to_string(),
        CreateUnitRefusal::PermanentLimit => "The limit of permanent summons is reached.".to_string(),
    }
}

/// Why the spell's building cannot be created, as a sentence.
pub fn create_building_text(g: &Game, r: CreateBuildingRefusal) -> String {
    match r {
        CreateBuildingRefusal::NotInCity => "Cast in a city.".to_string(),
        CreateBuildingRefusal::HasBuilding(b) => format!("The city has {} already.", display(g.d, b)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::reasons::tests::plain;
    use ffh_core::setup::GameSetup;

    fn game() -> Game {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 3));
        g.game_init();
        g
    }

    /// Every variant once, with sample ids. The match below lists them all (no catch-all), so a
    /// new variant breaks the build until it is added here.
    fn every_unmet(d: &GameData) -> Vec<SpellUnmet> {
        let pr = |n: &str| d.t::<PromotionType>(n);
        let (hidden, undead) = (pr("PROMOTION_HIDDEN"), pr("PROMOTION_UNDEAD"));
        let melee = d.t::<UnitCombatType>("UNITCOMBAT_MELEE");
        let recon = d.t::<UnitCombatType>("UNITCOMBAT_RECON");
        let lich = d.t::<UnitClassType>("UNITCLASS_LICH");
        let wolf = d.t::<UnitType>("UNIT_WOLF");
        let infernal = d.t::<CivilizationType>("CIVILIZATION_INFERNAL");
        let esus = d.t::<ReligionType>("RELIGION_COUNCIL_OF_ESUS");
        let blizzard = d.t::<FeatureType>("FEATURE_BLIZZARD");
        let graveyard = d.t::<ImprovementType>("IMPROVEMENT_GRAVEYARD");
        let ruins = d.t::<ImprovementType>("IMPROVEMENT_CITY_RUINS");
        let snow = d.t::<TerrainType>("TERRAIN_SNOW");
        let desert = d.t::<TerrainType>("TERRAIN_DESERT");
        let shrine = d.t::<BuildingType>("BUILDING_SHRINE_OF_SIRONA");
        let tech = d.t::<TechType>("TECH_MYSTICISM");
        let sought = [
            Sought::UnitWithout(wolf, hidden),
            Sought::Units(wolf, 2),
            Sought::Unit(wolf),
            Sought::UnitClass(lich),
            Sought::GraveTarget(2, undead, graveyard),
            Sought::EnemyOnlyPlot(2),
            Sought::Race(undead, 1),
            Sought::Resistible(1, 95),
            Sought::Damaged(1),
            Sought::Dispellable(0),
            Sought::DamagedOrPromotion(hidden),
            Sought::LivingUnit,
            Sought::VampirismCandidate(hidden, wolf),
            Sought::ReligionToRemove,
            Sought::SettledGreatPerson,
            Sought::EnemyOfCombat(melee, 1),
            Sought::HiddenUnit(3, [hidden, undead, hidden, undead]),
            Sought::FearableEnemy(1),
            Sought::Sanctify(ruins, graveyard, Some([snow, desert, snow, desert])),
            Sought::Sanctify(ruins, graveyard, None),
            Sought::FlamesOrTerrain(blizzard, graveyard, desert),
            Sought::Equipment,
            Sought::Holder(Some(hidden)),
            Sought::Holder(None),
            Sought::TauntTarget(1),
            Sought::SphereToTeach,
            Sought::Learner,
            Sought::RepairTarget(melee, recon, hidden),
            Sought::ReligionCities(3, esus, infernal),
            Sought::FamiliarCaster,
            Sought::LegionCity,
            Sought::LegionCivilization,
            Sought::BuyOutTarget,
            Sought::WalledCity,
            Sought::FalseColoursUnit,
            Sought::UnpackSite(12),
            Sought::PatrianSites,
            Sought::BorderSites(6),
            Sought::FeySites(12),
            Sought::UnclaimedSites,
            Sought::GateCity(shrine),
            Sought::NecropolisSites(3),
            Sought::HillSites(6, 3),
            Sought::CoastalSites,
            Sought::FaithfulHero,
            Sought::GrandOrderCity,
            Sought::RuinsToRefound,
            Sought::DragonsOathCapital,
            Sought::DecreePlot,
            Sought::DelveHill,
            Sought::DelveEnd,
        ];
        let mut v = vec![
            SpellUnmet::AiOnly,
            SpellUnmet::CasterHas(hidden),
            SpellUnmet::CasterLacksAny(hidden, undead, hidden),
            SpellUnmet::ImmuneToMagic,
            SpellUnmet::Immortal,
            SpellUnmet::CasterDamaged,
            SpellUnmet::CasterUndamaged,
            SpellUnmet::CannotMove,
            SpellUnmet::CannotFight,
            SpellUnmet::OnlyDefensive,
            SpellUnmet::HasAttacked,
            SpellUnmet::SeesHidden,
            SpellUnmet::HiddenNationality,
            SpellUnmet::HasCargo,
            SpellUnmet::IsCargo,
            SpellUnmet::InGroup,
            SpellUnmet::NotAloneOnPlot,
            SpellUnmet::NoCargoSpace,
            SpellUnmet::Summoned,
            SpellUnmet::Barbarian,
            SpellUnmet::UnitCombat(melee),
            SpellUnmet::UnitClass(lich),
            SpellUnmet::UnitType(wolf),
            SpellUnmet::SpecialUnit(d.t::<SpecialUnitType>("SPECIALUNIT_BIRD")),
            SpellUnmet::WorldUnit,
            SpellUnmet::OnlyCombatOrClass(melee, recon, lich),
            SpellUnmet::NotFamiliar,
            SpellUnmet::NoSummoner,
            SpellUnmet::SummonerCannotEnter,
            SpellUnmet::ClassLimit(lich, 4),
            SpellUnmet::NotInCity,
            SpellUnmet::InCity,
            SpellUnmet::InOwnCity,
            SpellUnmet::InTeamCity,
            SpellUnmet::NotInOwnCity,
            SpellUnmet::NotBarbarianCity,
            SpellUnmet::CityOwnerHuman,
            SpellUnmet::StrongerBarbarianOnPlot,
            SpellUnmet::CityPopulationBelow(3),
            SpellUnmet::CityHasReligion(esus),
            SpellUnmet::CityCivilization(infernal),
            SpellUnmet::NotInSettlement,
            SpellUnmet::CannotPackUp,
            SpellUnmet::OnWater,
            SpellUnmet::PlotOwned,
            SpellUnmet::PlotOwnedByOther,
            SpellUnmet::NotInOwnBorders,
            SpellUnmet::FeatureHere(blizzard),
            SpellUnmet::ImprovementHere(graveyard),
            SpellUnmet::Terrain(snow, None, None),
            SpellUnmet::Terrain(snow, Some(desert), Some(snow)),
            SpellUnmet::TerrainHere(snow),
            SpellUnmet::CannotHaveImprovement(graveyard),
            SpellUnmet::ImprovementWithin(graveyard, 4),
            SpellUnmet::NoFeatureWithin(blizzard, 1),
            SpellUnmet::VisibleEnemyOnPlot,
            SpellUnmet::PlotSpent,
            SpellUnmet::Civilization(infernal),
            SpellUnmet::Civic(d.t::<CivicType>("CIVIC_CRUSADE")),
            SpellUnmet::NoStateReligion,
            SpellUnmet::NoStateReligionAbroad,
            SpellUnmet::StateReligionDiffers,
            SpellUnmet::NoCities,
            SpellUnmet::NoCapital,
            SpellUnmet::CityLimit(5),
            SpellUnmet::GoldenAge,
            SpellUnmet::AtWarWithBarbarians,
            SpellUnmet::NotAtWarWithBarbarians,
            SpellUnmet::NotAtWar,
            SpellUnmet::NoBarbarianUnits,
            SpellUnmet::NoGreatPersonYet,
            SpellUnmet::CastOnce,
            SpellUnmet::BuildingOncePerTurn(shrine),
            SpellUnmet::GameOption(GameOptionType::GAMEOPTION_NO_LAIRS),
            SpellUnmet::GameOption(GameOptionType::GAMEOPTION_NO_CHANGING_WAR_PEACE),
            SpellUnmet::NetworkGame,
            SpellUnmet::ArmageddonCounterZero,
            SpellUnmet::StormActive,
            SpellUnmet::NoStorm,
            SpellUnmet::StormMoved,
            SpellUnmet::TitheInProgress,
            SpellUnmet::CommanderHasCast,
            SpellUnmet::HolderHasCast,
            SpellUnmet::UnitKeeps(wolf, hidden),
            SpellUnmet::NoCard,
            SpellUnmet::CardKnown(tech),
            SpellUnmet::CardNeeds(tech),
        ];
        v.extend(sought.map(SpellUnmet::Missing));
        v
    }

    /// Exhaustive: adding a variant without a sample above fails to compile here.
    #[allow(dead_code)]
    fn listed(r: SpellUnmet) {
        match r {
            SpellUnmet::AiOnly
            | SpellUnmet::CasterHas(_)
            | SpellUnmet::CasterLacksAny(..)
            | SpellUnmet::ImmuneToMagic
            | SpellUnmet::Immortal
            | SpellUnmet::CasterDamaged
            | SpellUnmet::CasterUndamaged
            | SpellUnmet::CannotMove
            | SpellUnmet::CannotFight
            | SpellUnmet::OnlyDefensive
            | SpellUnmet::HasAttacked
            | SpellUnmet::SeesHidden
            | SpellUnmet::HiddenNationality
            | SpellUnmet::HasCargo
            | SpellUnmet::IsCargo
            | SpellUnmet::InGroup
            | SpellUnmet::NotAloneOnPlot
            | SpellUnmet::NoCargoSpace
            | SpellUnmet::Summoned
            | SpellUnmet::Barbarian
            | SpellUnmet::UnitCombat(_)
            | SpellUnmet::UnitClass(_)
            | SpellUnmet::UnitType(_)
            | SpellUnmet::SpecialUnit(_)
            | SpellUnmet::WorldUnit
            | SpellUnmet::OnlyCombatOrClass(..)
            | SpellUnmet::NotFamiliar
            | SpellUnmet::NoSummoner
            | SpellUnmet::SummonerCannotEnter
            | SpellUnmet::ClassLimit(..)
            | SpellUnmet::NotInCity
            | SpellUnmet::InCity
            | SpellUnmet::InOwnCity
            | SpellUnmet::InTeamCity
            | SpellUnmet::NotInOwnCity
            | SpellUnmet::NotBarbarianCity
            | SpellUnmet::CityOwnerHuman
            | SpellUnmet::StrongerBarbarianOnPlot
            | SpellUnmet::CityPopulationBelow(_)
            | SpellUnmet::CityHasReligion(_)
            | SpellUnmet::CityCivilization(_)
            | SpellUnmet::NotInSettlement
            | SpellUnmet::CannotPackUp
            | SpellUnmet::OnWater
            | SpellUnmet::PlotOwned
            | SpellUnmet::PlotOwnedByOther
            | SpellUnmet::NotInOwnBorders
            | SpellUnmet::FeatureHere(_)
            | SpellUnmet::ImprovementHere(_)
            | SpellUnmet::Terrain(..)
            | SpellUnmet::TerrainHere(_)
            | SpellUnmet::CannotHaveImprovement(_)
            | SpellUnmet::ImprovementWithin(..)
            | SpellUnmet::NoFeatureWithin(..)
            | SpellUnmet::VisibleEnemyOnPlot
            | SpellUnmet::PlotSpent
            | SpellUnmet::Civilization(_)
            | SpellUnmet::Civic(_)
            | SpellUnmet::NoStateReligion
            | SpellUnmet::NoStateReligionAbroad
            | SpellUnmet::StateReligionDiffers
            | SpellUnmet::NoCities
            | SpellUnmet::NoCapital
            | SpellUnmet::CityLimit(_)
            | SpellUnmet::GoldenAge
            | SpellUnmet::AtWarWithBarbarians
            | SpellUnmet::NotAtWarWithBarbarians
            | SpellUnmet::NotAtWar
            | SpellUnmet::NoBarbarianUnits
            | SpellUnmet::NoGreatPersonYet
            | SpellUnmet::CastOnce
            | SpellUnmet::BuildingOncePerTurn(_)
            | SpellUnmet::GameOption(_)
            | SpellUnmet::NetworkGame
            | SpellUnmet::ArmageddonCounterZero
            | SpellUnmet::StormActive
            | SpellUnmet::NoStorm
            | SpellUnmet::StormMoved
            | SpellUnmet::TitheInProgress
            | SpellUnmet::CommanderHasCast
            | SpellUnmet::HolderHasCast
            | SpellUnmet::UnitKeeps(..)
            | SpellUnmet::NoCard
            | SpellUnmet::CardKnown(_)
            | SpellUnmet::CardNeeds(_) => {}
            SpellUnmet::Missing(s) => match s {
                Sought::UnitWithout(..)
                | Sought::Units(..)
                | Sought::Unit(_)
                | Sought::UnitClass(_)
                | Sought::GraveTarget(..)
                | Sought::EnemyOnlyPlot(_)
                | Sought::Race(..)
                | Sought::Resistible(..)
                | Sought::Damaged(_)
                | Sought::Dispellable(_)
                | Sought::DamagedOrPromotion(_)
                | Sought::LivingUnit
                | Sought::VampirismCandidate(..)
                | Sought::ReligionToRemove
                | Sought::SettledGreatPerson
                | Sought::EnemyOfCombat(..)
                | Sought::HiddenUnit(..)
                | Sought::FearableEnemy(_)
                | Sought::Sanctify(..)
                | Sought::FlamesOrTerrain(..)
                | Sought::Equipment
                | Sought::Holder(_)
                | Sought::TauntTarget(_)
                | Sought::SphereToTeach
                | Sought::Learner
                | Sought::RepairTarget(..)
                | Sought::ReligionCities(..)
                | Sought::FamiliarCaster
                | Sought::LegionCity
                | Sought::LegionCivilization
                | Sought::BuyOutTarget
                | Sought::WalledCity
                | Sought::FalseColoursUnit
                | Sought::UnpackSite(_)
                | Sought::PatrianSites
                | Sought::BorderSites(_)
                | Sought::FeySites(_)
                | Sought::UnclaimedSites
                | Sought::GateCity(_)
                | Sought::NecropolisSites(_)
                | Sought::HillSites(..)
                | Sought::CoastalSites
                | Sought::FaithfulHero
                | Sought::GrandOrderCity
                | Sought::RuinsToRefound
                | Sought::DragonsOathCapital
                | Sought::DecreePlot
                | Sought::DelveHill
                | Sought::DelveEnd => {}
            },
        }
    }

    /// A one-sentence, plain text: what the client shows (`plainDetail`), ending with a full stop.
    fn check(t: &str) {
        assert!(plain(t), "not plain: {t}");
        assert!(t.ends_with('.'), "no full stop: {t}");
        assert!(t.starts_with(|c: char| c.is_uppercase() || c.is_ascii_digit()), "lower case: {t}");
    }

    #[test]
    fn every_spell_refusal_sentence_is_plain() {
        let g = game();
        let d = g.d;
        let u = g.players.iter().flat_map(|p| p.units.iter().copied()).next().expect("a unit");
        let spell = d.spells.keys().next().unwrap();
        for r in every_unmet(d) {
            check(&spell_unmet_text(&g, u, spell, r));
        }
        check(&create_unit_text(CreateUnitRefusal::OwnKind));
        check(&create_unit_text(CreateUnitRefusal::VisibleEnemy));
        check(&create_unit_text(CreateUnitRefusal::PermanentLimit));
        check(&create_building_text(&g, CreateBuildingRefusal::NotInCity));
        check(&create_building_text(&g, CreateBuildingRefusal::HasBuilding(d.t::<BuildingType>("BUILDING_SHRINE_OF_SIRONA"))));
        // Every spell's effects as the cast gate names them, alone and together.
        for s in d.spells.keys() {
            let info = &d.spells[s];
            let gated: Vec<NoEffect> = [
                info.add_promotion_type1.map(NoEffect::AddPromotion),
                info.remove_promotion_type1.map(NoEffect::RemovePromotion),
                info.create_feature_type.map(NoEffect::Feature),
                info.create_improvement_type.map(NoEffect::Improvement),
                info.spread_religion.map(NoEffect::SpreadReligion),
                info.dispel.then_some(NoEffect::Dispel),
                (info.immobile_turns > 0).then_some(NoEffect::Immobile),
                info.push.then_some(NoEffect::Push),
            ]
            .into_iter()
            .flatten()
            .collect();
            for &e in &gated {
                check(&no_effect_text(&g, u, s, e));
            }
            if gated.len() > 1 {
                check(&no_effect_text(&g, u, s, NoEffect::Several));
            }
        }
        check(&no_effect_text(&g, u, spell, NoEffect::Nothing));
        // What the cast gate says for every unit and spell of a new game.
        for p in g.player_ids() {
            for &u in &g.players[p].units {
                for s in d.spells.keys() {
                    if let Some(r) = g.unit_cast_refusal(u, s, false) {
                        check(&crate::view::reasons::spell_refusal_text(&g, u, s, r));
                    }
                }
            }
        }
    }
}
