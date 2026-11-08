//! Info class structs, one module per XML area. See lib.rs `game_data!` for the table list.

pub mod basic;
pub mod terrain;
pub mod civics;
pub mod game;
pub mod building;
pub mod civ;
pub mod unit;
pub mod event;
pub mod culture;
pub mod governor;
pub mod engagement;
pub mod battle_cards;
pub mod battle_magic;
pub mod battle_kit;

pub use basic::*;
pub use terrain::*;
pub use civics::*;
pub use game::*;
pub use building::*;
pub use civ::*;
pub use unit::*;
pub use event::*;
pub use culture::*;
pub use governor::*;
pub use engagement::*;
pub use battle_cards::*;
pub use battle_magic::*;
pub use battle_kit::*;

use crate::registry::Rd;

/// C++ `CvInfoBase`: common fields of every info class.
#[derive(Debug, Clone, Default)]
pub struct InfoBase {
    pub graphical_only: bool,
    pub type_: String,
    /// Text key of the name (C++ `m_szTextKey`, read from `<Description>`).
    pub description: String,
    pub civilopedia: String,
    pub help: String,
    pub strategy: String,
    pub button: String,
}

impl InfoBase {
    /// C++ `CvInfoBase::read`.
    pub fn read(rd: &Rd) -> InfoBase {
        InfoBase {
            graphical_only: rd.bool("graphical_only"),
            type_: rd.string("type"),
            description: rd.string("description"),
            civilopedia: rd.string("civilopedia"),
            help: rd.string("help"),
            strategy: rd.string("strategy"),
            button: rd.string("button"),
        }
    }
}
