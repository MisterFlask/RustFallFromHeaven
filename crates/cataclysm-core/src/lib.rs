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
}
