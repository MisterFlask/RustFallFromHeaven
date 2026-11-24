//! Why the city screen's production list shows an entry disabled: the sentence of each refusal
//! of the rules' gates (ffh-core's `city::refusal`, the first check of `canTrain`,
//! `canConstruct` or `canCreate` that fails). The reasons come from the gates themselves, so
//! they cannot drift from the rules; every sentence the page shows for them is written here,
//! bare (labels and the current numbers), one per refusal.

use ffh_core::city::refusal::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::names::{display, Named};
use crate::view::city_name;

/// A type's display name.
fn n<T: Named>(g: &Game, v: T) -> String {
    display(g.d, v).into_owned()
}

/// The name of city `c`'s civilization's building of class `k` (the class's own name without one).
fn class_building(g: &Game, c: CityId, k: BuildingClassType) -> String {
    match g.d.civ_building(g.city_civilization_type(c), k) {
        Some(b) => n(g, b),
        None => n(g, k),
    }
}

/// The name of city `c`'s owner's unit of class `k` (the class's own name without one).
fn class_unit(g: &Game, c: CityId, k: UnitClassType) -> String {
    let unit = g.player_get_civilization_type(g.cities[c].owner).and_then(|civ| g.d.civ_unit(civ, k));
    match unit {
        Some(u) => n(g, u),
        None => n(g, k),
    }
}

/// The owner's state religion, `none` without one.
fn state_religion(g: &Game, c: CityId) -> String {
    match g.player_get_state_religion(g.cities[c].owner) {
        Some(r) => n(g, r),
        None => "none".to_string(),
    }
}

/// What a limit counts.
#[derive(Copy, Clone)]
enum Item {
    Unit,
    Building,
    Project,
}

/// `Built in the world: 1 of 1.`, `Units held or in production: 2 of 2.`, ...
fn limit_text(item: Item, l: Limit) -> String {
    let what = match (item, l.scope, l.in_production) {
        (Item::Unit, LimitScope::World, false) => "Created in the world",
        (Item::Unit, LimitScope::World, true) => "Created or in production in the world",
        (Item::Unit, LimitScope::Team, false) => "Units held by the team",
        (Item::Unit, LimitScope::Team, true) => "Units held or in production in the team",
        (Item::Unit, LimitScope::Player, false) => "Units held",
        (Item::Unit, LimitScope::Player, true) => "Units held or in production",
        (Item::Building | Item::Project, LimitScope::World, false) => "Built in the world",
        (Item::Building | Item::Project, LimitScope::World, true) => "Built or in production in the world",
        (Item::Building | Item::Project, LimitScope::Team, false) => "Built by the team",
        (Item::Building | Item::Project, LimitScope::Team, true) => "Built or in production in the team",
        (Item::Building | Item::Project, LimitScope::Player, false) => "Built already",
        (Item::Building | Item::Project, LimitScope::Player, true) => "Built or in production",
    };
    format!("{what}: {} of {}.", l.count, l.max)
}

/// `Requires state religion The Order; current: none.`
fn state_religion_text(g: &Game, c: CityId, r: ReligionType) -> String {
    format!("Requires state religion {}; current: {}.", n(g, r), state_religion(g, c))
}

/// `Requires a coast on water of 20 tiles or more.`
fn coast_text(min_area: i32) -> String {
    format!("Requires a coast on water of {min_area} tiles or more.")
}

/// Why city `c` cannot train `unit` ([`Game::city_train_refusal`]).
pub fn train_refusal_text(g: &Game, c: CityId, unit: UnitType, r: TrainRefusal) -> String {
    match r {
        TrainRefusal::MinLevel(level) => format!("Only by upgrade, at level {level}."),
        TrainRefusal::Outpost => "The city is an outpost.".to_string(),
        TrainRefusal::UpgradeCivUnit | TrainRefusal::WorldUnitNotOwnCiv | TrainRefusal::NotCivUnit | TrainRefusal::NotUpgradeCivUnit => {
            "Not a unit of this civilization.".to_string()
        }
        TrainRefusal::Bonus(b) => format!("Requires {} in this city.", n(g, b)),
        TrainRefusal::Player(r) => player_train_text(g, c, r),
        TrainRefusal::Upgrade(u) => format!("Replaced by {}.", n(g, u)),
        TrainRefusal::Plot(r) => plot_train_text(g, c, unit, r),
        TrainRefusal::PrereqCiv(civ) => format!("Only for the {}.", n(g, civ)),
        TrainRefusal::GameOption(o) => format!("Game option: {}.", n(g, o)),
        TrainRefusal::Py(r) => match r {
            PyTrainRefusal::Population { have, need } => format!("Population: {have} of {need}."),
            PyTrainRefusal::UnderCivic(civic) => format!("Not trained under {}.", n(g, civic)),
            PyTrainRefusal::GameOption(o) => format!("Game option: {}.", n(g, o)),
        },
    }
}

fn player_train_text(g: &Game, c: CityId, r: PlayerTrainRefusal) -> String {
    match r {
        PlayerTrainRefusal::NoProductionCost => "No production cost: not trained.".to_string(),
        PlayerTrainRefusal::OneCityChallenge => "One City Challenge: no units that found cities.".to_string(),
        PlayerTrainRefusal::Espionage => "Spy units are not trained in this game.".to_string(),
        PlayerTrainRefusal::Tech(t) => format!("Requires {}.", n(g, t)),
        PlayerTrainRefusal::StateReligion(r) => state_religion_text(g, c, r),
        PlayerTrainRefusal::Limit(l) => limit_text(Item::Unit, l),
        PlayerTrainRefusal::Nukes => "Nuclear units are off in this game.".to_string(),
        PlayerTrainRefusal::SpecialUnit(s) => format!("Not enabled yet: {}.", n(g, s)),
        PlayerTrainRefusal::Civic(civic) => format!("Requires the civic {}.", n(g, civic)),
        PlayerTrainRefusal::GlobalCounter { have, need } => format!("Armageddon Counter: {have} of {need}."),
        PlayerTrainRefusal::Alignment(a) => {
            let current = g.players[g.cities[c].owner].alignment.map_or_else(|| "none".to_string(), |x| n(g, x));
            format!("Requires alignment {}; current: {current}.", n(g, a))
        }
    }
}

fn plot_train_text(g: &Game, c: CityId, unit: UnitType, r: PlotTrainRefusal) -> String {
    match r {
        PlotTrainRefusal::ReligionPresent => format!("Requires no religion in this city: {} present.", g.city_religion_count(c)),
        PlotTrainRefusal::Religion(r) => format!("Requires {} in this city.", n(g, r)),
        PlotTrainRefusal::Corporation(k) => format!("Requires {} in this city.", n(g, k)),
        PlotTrainRefusal::NoWaterResources => "Requires water with resources next to the city.".to_string(),
        PlotTrainRefusal::AreaHasResources => "Only in an area without resources.".to_string(),
        PlotTrainRefusal::NotCoastal { min_area } => coast_text(min_area),
        PlotTrainRefusal::AreaTooSmall { have, need } => format!("Area: {have} of {need} tiles."),
        PlotTrainRefusal::NotWater => "Only on water.".to_string(),
        PlotTrainRefusal::NotLand => "Only on land.".to_string(),
        PlotTrainRefusal::NotInCity => "Only in a city.".to_string(),
        PlotTrainRefusal::HolyCity(r) => format!("Only in the holy city of {}.", n(g, r)),
        PlotTrainRefusal::Building(b) => format!("Requires {} in this city.", n(g, b)),
        PlotTrainRefusal::BuildingClass(k) => format!("Requires {} in this city.", class_building(g, c, k)),
        PlotTrainRefusal::Bonus(b) => format!("Requires {} in this city.", n(g, b)),
        PlotTrainRefusal::AnyBonus => {
            let names: Vec<String> = g.d.units[unit].prereq_or_bonuses.iter().map(|&b| n(g, b)).collect();
            format!("Requires {} in this city.", names.join(" or "))
        }
    }
}

/// Why city `c` cannot construct `building` ([`Game::city_construct_refusal`]).
pub fn construct_refusal_text(g: &Game, c: CityId, building: BuildingType, r: ConstructRefusal) -> String {
    let _ = building;
    match r {
        ConstructRefusal::Outpost => "The city is an outpost.".to_string(),
        ConstructRefusal::OtherTier => "Not this city's tier.".to_string(),
        ConstructRefusal::Player(r) => player_construct_text(g, c, r),
        ConstructRefusal::Built { have, max } => format!("Built already: {have} of {max}."),
        ConstructRefusal::ReligionPresent { have } => format!("Requires no religion in this city: {have} present."),
        ConstructRefusal::StateReligion(None) => "Requires a state religion.".to_string(),
        ConstructRefusal::StateReligion(Some(r)) => format!("Requires the state religion {} in this city.", n(g, r)),
        ConstructRefusal::Religion(r) => format!("Requires {} in this city.", n(g, r)),
        ConstructRefusal::Corporation(k) => format!("Requires {}.", n(g, k)),
        ConstructRefusal::Location(l) => match l {
            LocationRefusal::Coast { river: false, min_area } => coast_text(min_area),
            LocationRefusal::Coast { river: true, min_area } => format!("Requires a river or a coast on water of {min_area} tiles or more."),
            LocationRefusal::AreaTooSmall { have, need } => format!("Area: {have} of {need} tiles."),
            LocationRefusal::River => "Requires a river.".to_string(),
        },
        ConstructRefusal::Exclusive(k) => format!("Excluded by {} in this city.", class_building(g, c, k)),
        ConstructRefusal::GovernmentCenter => "The city is a government center already.".to_string(),
        ConstructRefusal::InQueue => "Already in the production queue.".to_string(),
        ConstructRefusal::WorldWonders { have, max } => format!("World wonders in this city: {have} of {max}."),
        ConstructRefusal::TeamWonders { have, max } => format!("Team wonders in this city: {have} of {max}."),
        ConstructRefusal::NationalWonders { have, max } => format!("National wonders in this city: {have} of {max}."),
        ConstructRefusal::Buildings { have, max } => format!("Buildings in this city: {have} of {max}."),
        ConstructRefusal::HolyCity(r) => format!("Only in the holy city of {}.", n(g, r)),
        ConstructRefusal::Bonus(b) | ConstructRefusal::OrBonus(b) => format!("Requires {} in this city.", n(g, b)),
        ConstructRefusal::MaxLatitude { have, max } => format!("Latitude: {have}; at most {max}."),
        ConstructRefusal::MinLatitude { have, min } => format!("Latitude: {have}; at least {min}."),
        ConstructRefusal::Building(b) => format!("Requires {} in this city.", n(g, b)),
        ConstructRefusal::PrereqCiv(civ) => format!("Only for the {}.", n(g, civ)),
        ConstructRefusal::Trait(t) => format!("Requires the trait {}.", n(g, t)),
        ConstructRefusal::PlotRadius(r) => format!("City radius: {r} already."),
        ConstructRefusal::NotCivBuilding => "Not a building of this civilization.".to_string(),
        ConstructRefusal::Py(r) => match r {
            PyConstructRefusal::Trait(t) => format!("Not with the trait {}.", n(g, t)),
            PyConstructRefusal::UnderCivic(civic) => format!("Not built under {}.", n(g, civic)),
            PyConstructRefusal::NoHero => "Requires a civilization with a hero.".to_string(),
            PyConstructRefusal::HeroNotCreated(k) => format!("{} not trained yet.", class_unit(g, c, k)),
            PyConstructRefusal::HeroAlive(k) => format!("{} is alive.", class_unit(g, c, k)),
            PyConstructRefusal::GameOption(o) => format!("Game option: {}.", n(g, o)),
            PyConstructRefusal::StateReligion(r) => format!("Not with the state religion {}.", n(g, r)),
            PyConstructRefusal::Capital => "Not in the capital.".to_string(),
            PyConstructRefusal::Alignment(a) => format!("Not with alignment {}.", n(g, a)),
            PyConstructRefusal::LaterStage(b) => format!("{} is built.", n(g, b)),
            PyConstructRefusal::SmugglingRing => "Requires the Undercouncil resolution Smuggling Ring.".to_string(),
        },
    }
}

fn player_construct_text(g: &Game, c: CityId, r: PlayerConstructRefusal) -> String {
    match r {
        PlayerConstructRefusal::NoProductionCost => "No production cost: not built.".to_string(),
        PlayerConstructRefusal::Tech(t) | PlayerConstructRefusal::SpecialBuildingTech(t) => format!("Requires {}.", n(g, t)),
        PlayerConstructRefusal::Obsolete => "Obsolete.".to_string(),
        PlayerConstructRefusal::StateReligion(r) => state_religion_text(g, c, r),
        PlayerConstructRefusal::NumTeams { have, need } => format!("Civilizations in the game: {have} of {need}."),
        PlayerConstructRefusal::VictoryInvalid(v) => format!("Victory off in this game: {}.", n(g, v)),
        PlayerConstructRefusal::MinorCiv => "Not for a minor civilization.".to_string(),
        PlayerConstructRefusal::VictoryCountdown(v) => format!("Countdown to {} started.", n(g, v)),
        PlayerConstructRefusal::Limit(l) => limit_text(Item::Building, l),
        PlayerConstructRefusal::ObsoletePrereq { class, have, need } | PlayerConstructRefusal::BuildingCount { class, have, need } => {
            format!("Requires {}: {have} of {need}.", class_building(g, c, class))
        }
        PlayerConstructRefusal::Nukes => "Nuclear weapons are off in this game.".to_string(),
        PlayerConstructRefusal::SpecialBuilding(s) => format!("Not enabled yet: {}.", n(g, s)),
        PlayerConstructRefusal::Cities { have, need, max } => {
            if max == -1 {
                format!("Cities: {have} of {need}.")
            } else {
                format!("Cities: {have} of {max}.")
            }
        }
        PlayerConstructRefusal::UnitLevel { have, need } => format!("Requires a unit of level {need}: highest is {have}."),
    }
}

/// Why city `c` cannot create `project` ([`Game::city_create_refusal`]).
pub fn create_refusal_text(g: &Game, c: CityId, project: ProjectType, r: CreateRefusal) -> String {
    let d = g.d;
    let owner = g.cities[c].owner;
    match r {
        CreateRefusal::Outpost => "The city is an outpost.".to_string(),
        CreateRefusal::PrereqCiv(civ) => format!("Only for the {}.", n(g, civ)),
        CreateRefusal::GlobalCounter { have, need } => format!("Armageddon Counter: {have} of {need}."),
        CreateRefusal::Player(r) => match r {
            PlayerCreateRefusal::Barbarian => "Not for the barbarians.".to_string(),
            PlayerCreateRefusal::NoProductionCost => "No production cost: not created.".to_string(),
            PlayerCreateRefusal::Tech(t) => format!("Requires {}.", n(g, t)),
            PlayerCreateRefusal::VictoryInvalid(v) => format!("Victory off in this game: {}.", n(g, v)),
            PlayerCreateRefusal::MinorCiv => "Not for a minor civilization.".to_string(),
            PlayerCreateRefusal::VictoryCountdown(v) => format!("Countdown to {} started.", n(g, v)),
            PlayerCreateRefusal::Limit(l) => limit_text(Item::Project, l),
            PlayerCreateRefusal::Nukes => "Nuclear weapons are off in this game.".to_string(),
            PlayerCreateRefusal::AnyoneProject(p) => format!("Requires {}, by any civilization.", n(g, p)),
            PlayerCreateRefusal::ProjectsNeeded { project, have, need } => format!("Requires {}: {have} of {need}.", n(g, project)),
        },
        CreateRefusal::Py(r) => match r {
            PyCreateRefusal::NoStateReligion => "Requires a state religion.".to_string(),
            PyCreateRefusal::NoPlayersKilled => format!("Civilizations' last cities taken: {} of 1.", g.players[owner].players_killed),
            PyCreateRefusal::Civilization(civ) => format!("Not for the {}.", n(g, civ)),
            PyCreateRefusal::ChargeHeld => "This city holds its charge.".to_string(),
            PyCreateRefusal::NotCoastal { min_area } => coast_text(min_area),
            PyCreateRefusal::NoRitualCompleted => format!("World rituals completed: {} of 1.", g.player_world_ritual_net_completions(owner)),
            PyCreateRefusal::GameOption(o) => format!("Game option: {}.", n(g, o)),
            PyCreateRefusal::CastPending => match g.world_ritual_spell(project) {
                Some(s) => format!("Cast pending: cast {} first.", n(g, s)),
                None => "Cast pending.".to_string(),
            },
            PyCreateRefusal::SanctuaryHolds => format!("Sanctuary holds: {}.", crate::view::rituals::turns_text(g.players[owner].sanctuary_timer)),
            PyCreateRefusal::InQueueElsewhere(o) => format!("In progress in {}.", city_name(g, o)),
            PyCreateRefusal::NoTitheRival => match d.ty::<SpellType>("SPELL_NAME_THE_TITHE") {
                Some(s) => format!("No rival named: use {}.", n(g, s)),
                None => "No rival named.".to_string(),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits() {
        let l = |scope, in_production| Limit { scope, in_production, count: 1, max: 1 };
        assert_eq!(limit_text(Item::Building, l(LimitScope::Player, false)), "Built already: 1 of 1.");
        assert_eq!(limit_text(Item::Building, l(LimitScope::World, true)), "Built or in production in the world: 1 of 1.");
        assert_eq!(limit_text(Item::Unit, l(LimitScope::Player, true)), "Units held or in production: 1 of 1.");
        assert_eq!(coast_text(20), "Requires a coast on water of 20 tiles or more.");
    }
}
