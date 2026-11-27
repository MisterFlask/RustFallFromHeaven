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
        // Push AC to 100 via BlightStrike rituals
        g.players[0].mana = 1000;
        for _ in 0..20 {
            if g.apocalypse.carcosa_risen {
                break;
            }
            let _ = g.apply(p0, Command::CastRitual { ritual: RitualKind::BlightStrike });
        }
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
}
