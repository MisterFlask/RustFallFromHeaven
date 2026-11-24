//! Why an event option cannot be chosen, the parts that come from the checks the option's gate
//! calls: the city's, the plot's and the unit's `canApplyEvent`, the option's own script
//! (`PythonCanDo`) and the war it would declare. Called from `reasons::event_refusal_text`; bare
//! sentences (CLAUDE.md, "Rules text and tooltips").

use crate::names::display;
use crate::view::reasons::{team_name, war_refusal_text};
use ffh_core::city::turn::CityEventRefusal;
use ffh_core::ffh::event_unmet::EventUnmet;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::plot::units::PlotEventRefusal;
use ffh_core::team::diplomacy::WarRefusal;
use ffh_core::unit::UnitEventRefusal;
use ffh_data::*;

/// Why the event's city (`other`: the other player's city) fails `canApplyEvent`.
pub fn city_event_text(g: &Game, other: bool, r: CityEventRefusal) -> String {
    let d = g.d;
    let (subj, lower) = if other { ("The other player's city", "the other player's city") } else { ("The city", "the city") };
    match r {
        CityEventRefusal::NoCity => format!("{subj} is lost."),
        CityEventRefusal::Food { stored, after } => format!("{subj} has too little stored food: {stored} now, {after} after."),
        CityEventRefusal::Population(n) => format!("{subj} must have a population of more than {n}."),
        CityEventRefusal::Culture(n) => format!("{subj} must have at least {n} culture."),
        CityEventRefusal::NoCivBuilding(c) => format!("The civilization of {lower} has no {}.", display(d, c)),
        CityEventRefusal::LacksBuilding(b) => format!("{subj} must have {}.", display(d, b)),
        CityEventRefusal::HasBuilding(b) => format!("{subj} has {} already.", display(d, b)),
        CityEventRefusal::Religions(0) => format!("{subj} must have no religion."),
        CityEventRefusal::Religions(1) => format!("{subj} must have at most 1 religion."),
        CityEventRefusal::Religions(n) => format!("{subj} must have at most {n} religions."),
        CityEventRefusal::Pillage { have, need } => format!("Improvements to pillage around {lower}: {have} of {need}."),
    }
}

/// Why the event's plot fails `canApplyEvent`.
pub fn plot_event_text(g: &Game, r: PlotEventRefusal) -> String {
    let d = g.d;
    match r {
        PlotEventRefusal::HasImprovement => "The plot has an improvement already.".to_string(),
        PlotEventRefusal::Feature(f) => format!("The plot cannot have {}.", display(d, f)),
        PlotEventRefusal::NoFeature => "The plot has no feature.".to_string(),
        PlotEventRefusal::Improvement(i) => format!("The plot cannot have {}.", display(d, i)),
        PlotEventRefusal::NoImprovement => "The plot has no improvement.".to_string(),
        PlotEventRefusal::NoRoute => "The plot has no route.".to_string(),
        PlotEventRefusal::CityRoute => "A city's route is not removed.".to_string(),
    }
}

/// Why the event's unit fails `canApplyEvent`.
pub fn unit_event_text(g: &Game, r: UnitEventRefusal) -> String {
    let d = g.d;
    match r {
        UnitEventRefusal::NoPromotion => "The unit can take no promotion.".to_string(),
        UnitEventRefusal::HasPromotion(p) => format!("The unit has {} already.", display(d, p)),
        UnitEventRefusal::CannotAttack => "The unit cannot attack.".to_string(),
    }
}

/// The check of the option's own script (`PythonCanDo`) that player `p` fails.
pub fn event_unmet_text(g: &Game, p: PlayerId, r: EventUnmet) -> String {
    let d = g.d;
    match r {
        EventUnmet::Civic(c) => format!("Requires {}.", display(d, c)),
        EventUnmet::NotStateReligion(r) => format!("Not for a civilization with {} as state religion.", display(d, r)),
        EventUnmet::CityLacks(b) => format!("The city must have {}.", display(d, b)),
        EventUnmet::PermanentTrait(t) => format!("The leader has {} permanently.", display(d, t)),
        EventUnmet::Trophy => "Requires an outcome of an earlier scenario.".to_string(),
        EventUnmet::Gold(n) => format!("Gold: {} of {}.", g.players[p].gold.max(0), n),
        EventUnmet::NoOtherPlayer => "It needs another player.".to_string(),
        EventUnmet::NoTrade => "No trade network with the other player.".to_string(),
        EventUnmet::NoBuildingToLose { other, max_cost } => {
            let subj = if other { "The other player's city" } else { "The city" };
            match max_cost {
                Some(m) => format!("{subj} has no building, other than a wonder, that costs {m} or less."),
                None => format!("{subj} has no building other than wonders."),
            }
        }
        EventUnmet::BuildingToLose => "Only for a city with no building other than wonders.".to_string(),
        EventUnmet::NoSpareStrategic => {
            let names: Vec<String> = ["BONUS_COPPER", "BONUS_IRON", "BONUS_HORSE", "BONUS_IVORY"]
                .iter()
                .filter_map(|n| d.ty::<BonusType>(n))
                .map(|b| display(d, b).into_owned())
                .collect();
            format!("Requires a second {} that the other player lacks.", or_list(&names))
        }
        EventUnmet::NoThirdPartyWar => "Requires a civilization at war with the other player and at peace with you.".to_string(),
        EventUnmet::NoAdjacentImprovement => "No plot next to the event's plot has an improvement.".to_string(),
    }
}

/// Why the war the option declares cannot be: `declarer` cannot declare war on `target`. From
/// player `p`'s team, the war gate's own sentence; from the other player's, the same check said
/// of that player.
pub fn event_war_text(g: &Game, p: PlayerId, declarer: TeamId, target: TeamId, r: WarRefusal) -> String {
    let us = g.players[p].team;
    if declarer == us {
        return war_refusal_text(g, declarer, target, r);
    }
    let them = team_name(g, declarer);
    let whom = |t: TeamId| if t == us { "you".to_string() } else { team_name(g, t) };
    match r {
        WarRefusal::Own => "The other player is on your team.".to_string(),
        WarRefusal::CrusadeHost => "A crusade's Host is at war with its crusade's enemies only.".to_string(),
        WarRefusal::Dead => format!("{them} is not alive."),
        WarRefusal::AtWar => format!("At war with {them} already."),
        WarRefusal::NotMet => format!("{them} has not met you."),
        WarRefusal::Peace(t) => format!("A peace treaty between {them} and {} holds.", whom(t)),
        WarRefusal::Truce(t) => format!("A truce between {them} and {} holds until turn {}.", whom(t), g.teams[declarer].truce_until[t]),
        WarRefusal::NoChangingWarPeace => "War and peace do not change in this game (No Changing War or Peace).".to_string(),
        WarRefusal::Permanent(t) => format!("Permanent war or peace between {them} and {}.", whom(t)),
        WarRefusal::WeAreVassal => format!("{them} is a vassal: its master decides its wars."),
        WarRefusal::OurVassal => format!("You are a vassal of {them}."),
        WarRefusal::TheyAreVassal => "You are a vassal: war is declared on your master.".to_string(),
        WarRefusal::AlwaysPeace => "Always Peace is on in this game.".to_string(),
        WarRefusal::Script => "Refused by the rules' war script.".to_string(),
    }
}

/// `A, B or C`.
fn or_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {}", rest.join(", "), last),
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

    /// Every sentence of this file is shown by the client (`plainDetail`) and ends a sentence.
    #[test]
    fn every_event_reason_is_plain() {
        let g = game();
        let d = g.d;
        let p = g.player_ids().next().unwrap();
        let q = g.player_ids().nth(1).unwrap();
        let (us, them) = (g.players[p].team, g.players[q].team);
        let building = d.buildings.keys().next().unwrap();
        let mut texts = Vec::new();
        for other in [false, true] {
            for r in [
                CityEventRefusal::NoCity,
                CityEventRefusal::Food { stored: 4, after: -2 },
                CityEventRefusal::Population(3),
                CityEventRefusal::Culture(50),
                CityEventRefusal::NoCivBuilding(d.building_classes.keys().next().unwrap()),
                CityEventRefusal::LacksBuilding(building),
                CityEventRefusal::HasBuilding(building),
                CityEventRefusal::Religions(0),
                CityEventRefusal::Religions(1),
                CityEventRefusal::Religions(2),
                CityEventRefusal::Pillage { have: 1, need: 3 },
            ] {
                texts.push(city_event_text(&g, other, r));
            }
        }
        for r in [
            PlotEventRefusal::HasImprovement,
            PlotEventRefusal::Feature(d.features.keys().next().unwrap()),
            PlotEventRefusal::NoFeature,
            PlotEventRefusal::Improvement(d.improvements.keys().next().unwrap()),
            PlotEventRefusal::NoImprovement,
            PlotEventRefusal::NoRoute,
            PlotEventRefusal::CityRoute,
        ] {
            texts.push(plot_event_text(&g, r));
        }
        for r in [UnitEventRefusal::NoPromotion, UnitEventRefusal::HasPromotion(d.promotions.keys().next().unwrap()), UnitEventRefusal::CannotAttack] {
            texts.push(unit_event_text(&g, r));
        }
        for r in [
            EventUnmet::Civic(d.t::<CivicType>("CIVIC_SOCIAL_ORDER")),
            EventUnmet::NotStateReligion(d.t::<ReligionType>("RELIGION_THE_ORDER")),
            EventUnmet::CityLacks(d.t::<BuildingType>("BUILDING_DUNGEON")),
            EventUnmet::PermanentTrait(d.t::<TraitType>("TRAIT_AGGRESSIVE")),
            EventUnmet::Trophy,
            EventUnmet::Gold(40),
            EventUnmet::NoOtherPlayer,
            EventUnmet::NoTrade,
            EventUnmet::NoBuildingToLose { other: true, max_cost: Some(100) },
            EventUnmet::NoBuildingToLose { other: false, max_cost: None },
            EventUnmet::BuildingToLose,
            EventUnmet::NoSpareStrategic,
            EventUnmet::NoThirdPartyWar,
            EventUnmet::NoAdjacentImprovement,
        ] {
            texts.push(event_unmet_text(&g, p, r));
        }
        let wars = [
            WarRefusal::Own,
            WarRefusal::CrusadeHost,
            WarRefusal::Dead,
            WarRefusal::AtWar,
            WarRefusal::NotMet,
            WarRefusal::Peace(us),
            WarRefusal::Truce(us),
            WarRefusal::NoChangingWarPeace,
            WarRefusal::Permanent(us),
            WarRefusal::WeAreVassal,
            WarRefusal::OurVassal,
            WarRefusal::TheyAreVassal,
            WarRefusal::AlwaysPeace,
            WarRefusal::Script,
        ];
        for r in wars {
            texts.push(event_war_text(&g, p, them, us, r));
            texts.push(event_war_text(&g, p, us, them, r));
        }
        for t in &texts {
            assert!(plain(t), "{t}");
            assert!(t.ends_with('.'), "{t}");
            assert!(t.starts_with(|c: char| c.is_ascii_uppercase()), "{t}");
        }
        assert!(texts.contains(&"The city must have a population of more than 3.".to_string()));
        assert!(event_unmet_text(&g, p, EventUnmet::NoSpareStrategic).contains(" or "));
    }
}
