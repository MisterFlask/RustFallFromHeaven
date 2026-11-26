//! Pure command-in / event-out state machine API shared 100% by human players and `cataclysm-ai`.
//!
//! No blocking callbacks, no re-entrant Answerers, no Civ4 turn-slice loop: every player decision
//! is a value of [`Command`], applied atomically by [`crate::game::Game::apply`].

use crate::city::{BuildOrder, SallyPosture};
use crate::data::{Doctrine, Religion, RitualKind, UnitClassId};
use crate::great_people::GreatPersonAction;
use crate::ids::{BattleId, CityId, CrusadeId, GreatPersonId, HouseId, MemberId, PlayerId, PlotIdx, SquadId};
use crate::world::Resource;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    // --- Squad commands (atomic map unit) ---
    MoveSquad { squad: SquadId, to: PlotIdx },
    AttackPlot { squad: SquadId, to: PlotIdx },
    SetSquadCell { squad: SquadId, member: MemberId, row: u8, lane: u8 },
    AutoArrangeSquad { squad: SquadId },
    ReinforceSquad { squad: SquadId, class: UnitClassId },
    UpgradeMember { squad: SquadId, member: MemberId },
    SetSquadRaid { squad: SquadId, active: bool },
    SetSquadFortify { squad: SquadId },
    SquadBuildRoad { squad: SquadId },
    SquadBuildFort { squad: SquadId },
    SquadExploreLair { squad: SquadId },
    SquadFoundCity { squad: SquadId, name: Option<String> },
    DisbandSquad { squad: SquadId },

    // --- Macro City commands ---
    CityBuild { city: CityId, order: BuildOrder },
    CitySallyPosture { city: CityId, posture: SallyPosture },
    CityWaypoint { city: CityId, plot: Option<PlotIdx> },
    RazeCity { city: CityId },
    LaunchCaravan { origin: CityId, destination: CityId, resource: Resource },

    // --- Doctrine, Tech, Religion & Strategic Rituals ---
    ChooseResearch { tech: crate::data::TechId },
    AdoptDoctrine { doctrine: Doctrine, secondary: bool },
    FoundOrAdoptReligion { religion: Religion },
    CastRitual { ritual: RitualKind },

    // --- Great People Court (3-way choice) ---
    UseGreatPerson { gp: GreatPersonId, action: GreatPersonAction },

    // --- Houses, Diplomacy & Crusades ---
    DeclareWar { target: PlayerId },
    MakePeace { target: PlayerId },
    BreakVassalage,
    CallCrusade { target: PlayerId },
    LendSquadToCrusade { crusade: CrusadeId, squad: SquadId },
    PayOffRaiders { raider: PlayerId },

    // --- Turn control ---
    EndTurn,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    TurnStarted { turn: i32, active_player: PlayerId },
    SquadMoved { squad: SquadId, from: PlotIdx, to: PlotIdx },
    BattleFought { battle: BattleId, attacker: PlayerId, defender: PlayerId, plot: PlotIdx, plot_taken: bool },
    CityFounded { city: CityId, owner: PlayerId, plot: PlotIdx, name: String },
    CityCaptured { city: CityId, new_owner: PlayerId, old_owner: PlayerId, name: String },
    CityRazed { plot: PlotIdx, name: String, by: PlayerId },
    TechCompleted { player: PlayerId, tech: crate::data::TechId },
    ReligionFounded { player: PlayerId, religion: Religion },
    GreatPersonBorn { player: PlayerId, gp: GreatPersonId, name: String },
    HouseFormed { player: PlayerId, house: HouseId, name: String },
    CrisisTriggered { threshold: i32, name: String, summary: String },
    HyboremArrived { player: PlayerId, city: CityId },
    CarcosaRisen { city: CityId, plot: PlotIdx },
    CrusadeCalled { crusade: CrusadeId, caller: PlayerId, target: PlayerId },
    LairExplored { player: PlayerId, plot: PlotIdx, summary: String },
    Log { player: Option<PlayerId>, plot: Option<PlotIdx>, text: String },
    GameEnded { summary: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CmdErr {
    pub code: &'static str,
    pub detail: String,
}

impl CmdErr {
    pub fn invalid(detail: impl Into<String>) -> Self {
        Self { code: "invalid", detail: detail.into() }
    }

    pub fn cannot(detail: impl Into<String>) -> Self {
        Self { code: "cannot", detail: detail.into() }
    }
}

impl std::fmt::Display for CmdErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}
