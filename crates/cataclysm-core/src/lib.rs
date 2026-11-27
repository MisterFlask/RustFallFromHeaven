//! `cataclysm-core`: Squad-and-Apocalypse-native 4X rules engine built cleanly from scratch.
//!
//! Shares `ffh-battle` for 3x3 Ogre Battle formation combat and maps its factions, unit classes,
//! and character portraits onto existing embedded art assets. Zero Civ4 / `ffh-core` dependency.

pub mod apocalypse;
pub mod city;
pub mod combat;
pub mod command;
pub mod crusade;
pub mod data;
pub mod economy;
pub mod game;
pub mod great_people;
pub mod house;
pub mod ids;
pub mod rng;
pub mod squad;
pub mod supply;
pub mod world;

pub use apocalypse::{ApocalypseState, GameOutcome, Lifeboat, LifeboatKind, VictoryKind};
pub use city::{BuildOrder, City, SallyPosture, WorkCrewTask};
pub use combat::{BattleRecord, FighterSnap, Forecast, SideMods};
pub use command::{CmdErr, Command, Event};
pub use crusade::Crusade;
pub use data::{
    Alignment, CaptainAura, CrisisInfo, DistrictInfo, DistrictKind, Doctrine, FactionId, FactionInfo, GreatSuit, HousePerk, PortraitCharacter,
    Religion, RitualKind, SecretEffect, SecretTechId, SecretTechInfo, TechId, TechInfo, UnitClassId, UnitClassInfo, WonderKind,
    ARMAGEDDON_CRISES, CHARACTER_POOL, DISTRICTS, FACTIONS, SECRET_TECHS, TECHS, UNIT_CLASSES,
};
pub use economy::{CaravanRoute, RateBreakdown};
pub use game::{Game, GameSetup, LogLine, Player};
pub use great_people::{GreatPerson, GreatPersonAction, SecretDeck};
pub use house::{House, HouseMood};
pub use ids::{BattleId, CaravanId, CityId, CrusadeId, GreatPersonId, HouseId, MemberId, PlayerId, PlotIdx, SquadId};
pub use rng::Rng;
pub use squad::{RecoveringHero, Squad, SquadMember, SquadOrder};
pub use supply::SupplyLine;
pub use world::{Feature, Improvement, Plot, PlotKind, Resource, Terrain, Visibility, World};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_satisfies_invariants_and_is_deterministic() {
        let g1 = Game::new(GameSetup { seed: 123, width: 24, height: 16, human_faction: FactionId::Bannor, ai_count: 4 });
        let g2 = Game::new(GameSetup { seed: 123, width: 24, height: 16, human_faction: FactionId::Bannor, ai_count: 4 });
        g1.check_invariants().unwrap();
        assert_eq!(g1.cities.len(), 5);
        assert_eq!(g1.cities.len(), g2.cities.len());
        assert_eq!(g1.squads.len(), g2.squads.len());
        assert_eq!(g1.world.carcosa_plot, g2.world.carcosa_plot);
    }

    #[test]
    fn squad_3x3_grid_and_reinforce_work() {
        let mut g = Game::new(GameSetup::default());
        let p0 = PlayerId(0);
        let sid = g.player_squads(p0)[0];
        let m0 = g.squads[&sid].members[0].id;
        g.apply(p0, Command::SetSquadCell { squad: sid, member: m0, row: 1, lane: 0 }).unwrap();
        assert_eq!((g.squads[&sid].members[0].row, g.squads[&sid].members[0].lane), (1, 0));
        g.players[0].gold = 500;
        g.apply(p0, Command::ReinforceSquad { squad: sid, class: UnitClassId::Guard }).unwrap();
        assert_eq!(g.squads[&sid].members.len(), 4);
        g.check_invariants().unwrap();
    }

    #[test]
    fn carcosa_rises_at_ac_100_and_lifeboat_victory_resolves() {
        let mut g = Game::new(GameSetup { seed: 7, width: 22, height: 16, human_faction: FactionId::Sheaim, ai_count: 3 });
        let p0 = PlayerId(0);
        // Strain 94, then one Crack the Crust: the Strain path fires every crisis up to the
        // Blowout (King Cinder's Host at 50 included).
        g.apocalypse.counter = 100 - RitualKind::BlightStrike.ac_delta();
        g.players[0].mana = 1000;
        g.apply(p0, Command::CastRitual { ritual: RitualKind::BlightStrike }).unwrap();
        assert!(g.apocalypse.carcosa_risen, "Carcosa must rise at AC 100");
        assert!(g.apocalypse.hyborem_spawned, "Hyborem must have risen at AC 50");
        assert!(g.apocalypse.carcosa_city.is_some());
        assert!(!g.world.carcosa_bridges.is_empty(), "Carcosa must build land bridges");
        assert!(g.apocalypse.standing_lifeboats() >= 1);
        g.check_invariants().unwrap();
    }

    #[test]
    fn a_player_eliminated_at_the_turn_wrap_is_skipped() {
        let mut g = Game::new(GameSetup { seed: 5, width: 22, height: 16, human_faction: FactionId::Bannor, ai_count: 2 });
        // Everyone ends their turn until the last player of the round is the active one.
        let last = (0..g.players.len()).rev().find(|&i| g.players[i].alive).unwrap();
        while g.active_player.idx() != last {
            let p = g.active_player;
            g.apply(p, Command::EndTurn).unwrap();
        }
        // Seat 0 loses every city and squad; the world turn at the wrap marks it eliminated.
        let p0 = PlayerId(0);
        for cid in g.player_cities(p0) {
            let c = g.cities.get_mut(&cid).unwrap();
            c.owner = PlayerId(1);
            c.capital = false;
            c.house = None;
        }
        g.squads.retain(|_, s| s.owner != p0);
        g.apply(PlayerId(last as u16), Command::EndTurn).unwrap();
        assert!(!g.players[0].alive);
        let next = g.active_player;
        assert!(g.players[next.idx()].alive, "the turn went to an eliminated player");
        g.apply(next, Command::EndTurn).unwrap();
    }

    /// Gives player 0 every tech its doctrines can open, so none is left to research.
    fn exhaust_research(g: &mut Game) {
        g.players[0].primary_doctrine = Some(Doctrine::Steel);
        g.players[0].secondary_doctrine = Some(Doctrine::Cavalry);
        while let Some(t) = TECHS.iter().find(|t| g.can_research(PlayerId(0), t.id).is_ok()) {
            g.players[0].techs.insert(t.id);
        }
    }

    #[test]
    fn idle_lore_becomes_sage_renown_with_the_remainder_kept() {
        let mut g = Game::new(GameSetup::default());
        exhaust_research(&mut g);
        assert!(!g.can_research_any(PlayerId(0)));
        g.players[0].lore_progress = 23;
        let sage = g.players[0].renown_meters[GreatSuit::Sage.id()];
        g.convert_idle_lore(PlayerId(0));
        assert_eq!(g.players[0].lore_progress, 23 % game::LORE_PER_SAGE_RENOWN);
        assert_eq!(g.players[0].renown_meters[GreatSuit::Sage.id()], sage + 23 / game::LORE_PER_SAGE_RENOWN);
    }

    #[test]
    fn lore_is_kept_while_a_tech_can_be_researched() {
        let mut g = Game::new(GameSetup::default());
        g.players[0].current_research = None;
        assert!(g.can_research_any(PlayerId(0)));
        g.players[0].lore_progress = 500;
        let sage = g.players[0].renown_meters[GreatSuit::Sage.id()];
        g.convert_idle_lore(PlayerId(0));
        assert_eq!(g.players[0].lore_progress, 500);
        assert_eq!(g.players[0].renown_meters[GreatSuit::Sage.id()], sage);
    }

    #[test]
    fn doctrine_techs_cost_the_same_by_tier() {
        let tier_cost = [0, data::TECH_COST_TIER_1, data::TECH_COST_TIER_2, data::TECH_COST_TIER_3, data::TECH_COST_TIER_4];
        for t in TECHS.iter().filter(|t| t.doctrine.is_some()) {
            assert_eq!(t.cost, tier_cost[t.tier as usize], "{}", t.name);
        }
    }

    /// A game where players 0 and 1 are at war.
    fn two_at_war() -> Game {
        let mut g = Game::new(GameSetup::default());
        g.set_war(PlayerId(0), PlayerId(1), true);
        g
    }

    #[test]
    fn a_peace_offer_alone_does_not_end_the_war() {
        let mut g = two_at_war();
        let (a, b) = (PlayerId(0), PlayerId(1));
        assert_eq!(g.war_turns(a, b), Some(0));
        g.apply(a, Command::MakePeace { target: b }).unwrap();
        assert!(g.at_war(a, b));
        assert_eq!(g.peace_offer_from(a, b), Some(game::PEACE_OFFER_TURNS));
        assert_eq!(g.peace_offer_from(b, a), None);
        assert!(g.apply(a, Command::MakePeace { target: b }).is_err(), "a second offer while one stands is refused");
        g.check_invariants().unwrap();
    }

    #[test]
    fn an_answered_offer_signs_peace_and_starts_the_truce() {
        let mut g = two_at_war();
        let (a, b) = (PlayerId(0), PlayerId(1));
        g.apply(a, Command::MakePeace { target: b }).unwrap();
        g.apply(b, Command::MakePeace { target: a }).unwrap();
        assert!(!g.at_war(a, b));
        assert_eq!(g.war_turns(a, b), None);
        assert_eq!(g.peace_offer_from(a, b), None);
        assert_eq!(g.truce_turns_left(a, b), game::TRUCE_TURNS);
        assert_eq!(g.truce_turns_left(b, a), game::TRUCE_TURNS);
        g.check_invariants().unwrap();
    }

    #[test]
    fn a_crusade_ends_with_the_peace_of_its_caller_and_target() {
        let mut g = two_at_war();
        let (a, b, c) = (PlayerId(0), PlayerId(1), PlayerId(2));
        g.players[0].religion = Some(Religion::Order);
        g.apply(a, Command::CallCrusade { target: b }).unwrap();
        let crusade = *g.crusades.keys().next().unwrap();
        let lent = g.player_squads(c)[0];
        g.apply(c, Command::LendSquadToCrusade { crusade, squad: lent }).unwrap();
        assert!(g.squads[&lent].lent_by.is_some());
        g.apply(a, Command::MakePeace { target: b }).unwrap();
        g.apply(b, Command::MakePeace { target: a }).unwrap();
        assert!(!g.crusades[&crusade].active);
        assert!(g.squads[&lent].lent_by.is_none());
        g.check_invariants().unwrap();
    }

    #[test]
    fn furnace_levy_without_a_city_spends_nothing() {
        let mut g = Game::new(GameSetup::default());
        let p0 = PlayerId(0);
        for cid in g.player_cities(p0) {
            let c = g.cities.get_mut(&cid).unwrap();
            c.owner = PlayerId(1);
            c.capital = false;
            c.house = None;
        }
        g.players[0].mana = 100;
        assert!(g.apply(p0, Command::CastRitual { ritual: RitualKind::SummonHost }).is_err());
        assert_eq!(g.players[0].mana, 100);
    }

    #[test]
    fn a_finished_squad_waits_for_room_under_the_squad_cap() {
        let mut g = Game::new(GameSetup::default());
        let p0 = PlayerId(0);
        let cid = g.player_cities(p0)[0];
        let plot = g.cities[&cid].plot;
        while g.player_squads(p0).len() < g.squad_cap(p0) {
            g.spawn_squad_internal(p0, plot, &[UnitClassId::Guard]);
        }
        let at_cap = g.player_squads(p0).len();
        let order = BuildOrder::Squad(vec![UnitClassId::Guard]);
        let c = g.cities.get_mut(&cid).unwrap();
        c.build_progress = order.cost();
        c.building = Some(order.clone());
        let mut events = Vec::new();
        g.try_complete_city_build(cid, &mut events);
        assert_eq!(g.player_squads(p0).len(), at_cap, "no squad past the cap");
        assert_eq!(g.cities[&cid].building, Some(order), "the order waits");
        let gone = g.player_squads(p0)[0];
        g.squads.remove(&gone);
        g.try_complete_city_build(cid, &mut events);
        assert_eq!(g.player_squads(p0).len(), at_cap);
        assert_eq!(g.cities[&cid].building, None);
    }

    #[test]
    fn no_war_starts_during_a_truce() {
        let mut g = two_at_war();
        let (a, b) = (PlayerId(0), PlayerId(1));
        g.apply(a, Command::MakePeace { target: b }).unwrap();
        g.apply(b, Command::MakePeace { target: a }).unwrap();

        // Declaring war.
        let err = g.apply(a, Command::DeclareWar { target: b }).unwrap_err();
        assert_eq!(err.code, "cannot");
        assert!(err.detail.contains(&format!("{} turns left", game::TRUCE_TURNS)), "{}", err.detail);

        // Attacking a city of the other side.
        let capital = g.player_cities(b)[0];
        let cplot = g.cities[&capital].plot;
        let next = g.world.neighbors(cplot).into_iter().find(|&p| g.world.plot(p).kind.is_passable_land() && g.world.plot(p).city.is_none()).unwrap();
        let sid = g.player_squads(a)[0];
        g.squads.get_mut(&sid).unwrap().plot = next;
        g.squads.get_mut(&sid).unwrap().moves_left = 2;
        assert!(g.apply(a, Command::AttackPlot { squad: sid, to: cplot }).unwrap_err().detail.starts_with("Truce"));
        assert!(!g.at_war(a, b));

        // Calling a Crusade.
        g.players[0].techs.insert(data::TECH_FANATICISM);
        assert!(g.apply(a, Command::CallCrusade { target: b }).unwrap_err().detail.starts_with("Truce"));

        // Lending a squad to another civilization's Crusade.
        g.players[2].techs.insert(data::TECH_FANATICISM);
        g.apply(PlayerId(2), Command::CallCrusade { target: b }).unwrap();
        let cr = *g.crusades.keys().next().unwrap();
        assert!(g.apply(a, Command::LendSquadToCrusade { crusade: cr, squad: sid }).unwrap_err().detail.starts_with("Truce"));
        assert!(!g.at_war(a, b));
        g.check_invariants().unwrap();

        // Once the truce has run out, war may be declared.
        g.turn += game::TRUCE_TURNS;
        assert_eq!(g.truce_turns_left(a, b), 0);
        g.apply(a, Command::DeclareWar { target: b }).unwrap();
        assert!(g.at_war(a, b));
        g.check_invariants().unwrap();
    }

    #[test]
    fn a_peace_offer_lapses() {
        let mut g = two_at_war();
        let (a, b) = (PlayerId(0), PlayerId(1));
        g.apply(a, Command::MakePeace { target: b }).unwrap();
        g.turn += game::PEACE_OFFER_TURNS - 1;
        assert_eq!(g.peace_offer_from(a, b), Some(1));
        g.turn += 1;
        assert_eq!(g.peace_offer_from(a, b), None);
        // The answer now is a new offer, not peace.
        g.apply(b, Command::MakePeace { target: a }).unwrap();
        assert!(g.at_war(a, b));
        assert_eq!(g.peace_offer_from(b, a), Some(game::PEACE_OFFER_TURNS));
        g.check_invariants().unwrap();
    }

    #[test]
    fn only_the_first_researcher_of_a_tech_moves_strain() {
        let mut g = Game::new(GameSetup::default());
        let tech = data::TECH_METALLURGY;
        assert!(TECHS[tech.0 as usize].ac_delta != 0);
        assert!(!g.discovered_techs.contains(&tech));
        for i in [1, 2] {
            g.players[i].techs.remove(&tech);
            g.players[i].current_research = Some(tech);
            g.players[i].lore_progress = 1000;
        }
        let before = g.apocalypse.counter;
        // Player 1's turn starts: it completes the tech first.
        g.apply(PlayerId(0), Command::EndTurn).unwrap();
        assert!(g.players[1].has_tech(tech));
        assert_eq!(g.apocalypse.counter, before + TECHS[tech.0 as usize].ac_delta);
        // Player 2's turn starts: the same tech moves nothing.
        g.apply(PlayerId(1), Command::EndTurn).unwrap();
        assert!(g.players[2].has_tech(tech));
        assert_eq!(g.apocalypse.counter, before + TECHS[tech.0 as usize].ac_delta);
    }

    #[test]
    fn starting_techs_count_as_discovered() {
        let g = Game::new(GameSetup { human_faction: FactionId::Amurites, ..GameSetup::default() });
        assert!(g.players[0].has_tech(data::TECH_ARCANA));
        assert!(g.discovered_techs.contains(&data::TECH_ARCANA));
    }

    #[test]
    fn a_strain_ritual_waits_for_its_recast_time() {
        let mut g = Game::new(GameSetup::default());
        let p0 = PlayerId(0);
        g.players[0].mana = 1000;
        let r = RitualKind::BlightStrike;
        assert!(r.recast_turns() > 0);
        g.apply(p0, Command::CastRitual { ritual: r }).unwrap();
        let err = g.apply(p0, Command::CastRitual { ritual: r }).unwrap_err();
        assert_eq!(err.detail, format!("Ready in {} turns.", r.recast_turns()));
        g.turn += r.recast_turns() - 1;
        assert_eq!(g.ritual_turns_left(p0, r), 1);
        assert!(g.check_ritual(p0, r).is_err());
        g.turn += 1;
        assert!(g.check_ritual(p0, r).is_ok());
        g.apply(p0, Command::CastRitual { ritual: r }).unwrap();
    }

    #[test]
    fn only_vent_the_seal_and_crack_the_crust_move_strain() {
        let mut g = Game::new(GameSetup::default());
        let p0 = PlayerId(0);
        g.apocalypse.counter = 5;
        for r in [RitualKind::HasteMarch, RitualKind::ArcaneSurge, RitualKind::SummonHost, RitualKind::WardWalls] {
            assert_eq!(r.ac_delta(), 0);
            assert_eq!(r.recast_turns(), 0);
            g.players[0].mana = 1000;
            g.apply(p0, Command::CastRitual { ritual: r }).unwrap();
            g.apply(p0, Command::CastRitual { ritual: r }).unwrap();
            assert_eq!(g.apocalypse.counter, 5, "{}", r.name());
        }
        g.apply(p0, Command::CastRitual { ritual: RitualKind::Sanctify }).unwrap();
        assert_eq!(g.apocalypse.counter, 5 + RitualKind::Sanctify.ac_delta());
    }

    #[test]
    fn every_third_bloody_battle_moves_strain_through_its_crisis() {
        let mut g = Game::new(GameSetup::default());
        let threshold = ARMAGEDDON_CRISES[0].threshold;
        g.apocalypse.counter = threshold - game::BLOODY_BATTLE_STRAIN;
        let mut events = Vec::new();
        for _ in 0..game::BLOODY_BATTLES_PER_STRAIN - 1 {
            g.note_bloody_battle(&mut events);
        }
        assert_eq!(g.apocalypse.counter, threshold - game::BLOODY_BATTLE_STRAIN);
        assert!(events.is_empty());
        g.note_bloody_battle(&mut events);
        assert_eq!(g.apocalypse.counter, threshold);
        assert!(events.iter().any(|e| matches!(e, Event::CrisisTriggered { threshold: t, .. } if *t == threshold)));
        assert!(g.apocalypse.triggered_crises.contains(&threshold));
        g.check_invariants().unwrap();
    }

    #[test]
    fn a_wonder_completed_elsewhere_refunds_the_rivals_industry() {
        let mut g = Game::new(GameSetup::default());
        let w = WonderKind::TowerOfMastery;
        let a = g.player_cities(PlayerId(0))[0];
        let b = g.player_cities(PlayerId(1))[0];
        let cost = BuildOrder::Wonder(w).cost();
        for (cid, progress) in [(a, cost), (b, 37)] {
            let c = g.cities.get_mut(&cid).unwrap();
            c.building = Some(BuildOrder::Wonder(w));
            c.build_progress = progress;
        }
        let gold = g.players[1].gold;
        let mut events = Vec::new();
        g.try_complete_city_build(a, &mut events);
        assert_eq!(g.built_wonders.get(&w), Some(&a));
        assert_eq!(g.cities[&b].building, None);
        assert_eq!(g.cities[&b].build_progress, 0);
        assert_eq!(g.players[1].gold, gold + 37);
        assert!(g.log.iter().any(|l| l.player == Some(PlayerId(1)) && l.text.contains("37 Industry refunded as 37 Gold")));
        g.check_invariants().unwrap();
    }
}
