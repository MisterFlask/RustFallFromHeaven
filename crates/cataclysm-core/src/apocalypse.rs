//! The Apocalypse Clock, Armageddon Crisis Deck, Hyborem Emergence, Carcosa, and Lifeboat Victory.
//!
//! Act I (Turns 1..50): Exploration, Houses, early skirmishes & Raids.
//! Act II (Turns 50..110): Doctrines, Crusades, Great Person Secret Deck, escalating Crises.
//! Act III (AC 100, 40-turn timer): Carcosa rises from the sea with land bridges; alliances &
//! vassal oaths can shatter; mortal empires fight to hold a Lifeboat while demonic sides try to
//! raze every Lifeboat on the map!

use crate::data::{ARMAGEDDON_CRISES, FactionId};
use crate::ids::{CityId, PlayerId};

pub const CARCOSA_TIMER_TURNS: i32 = 40;
pub const CARCOSA_PROTECTED_TURNS: i32 = 12;
pub const MAX_GAME_TURNS: i32 = 160;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LifeboatKind {
    Carcosa,
    TowerOfMastery,
    AltarOfLuonnotar,
    MostLegendary,
}

impl LifeboatKind {
    pub const fn name(self) -> &'static str {
        match self {
            LifeboatKind::Carcosa => "Caldera Keep",
            LifeboatKind::TowerOfMastery => "Siphon Spire",
            LifeboatKind::AltarOfLuonnotar => "Great Cistern",
            LifeboatKind::MostLegendary => "Last Metropolis",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Lifeboat {
    pub kind: LifeboatKind,
    pub city_id: CityId,
    pub razed: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VictoryKind {
    MortalLifeboat,
    DemonicRuin,
    ElohimVigil,
    Conquest,
    ScoreAtTurnLimit,
    MutualExtinction,
}

impl VictoryKind {
    pub const fn name(self) -> &'static str {
        match self {
            VictoryKind::MortalLifeboat => "Surface Lifeboat Survival",
            VictoryKind::DemonicRuin => "Furnace Triumph (All Lifeboats Razed)",
            VictoryKind::ElohimVigil => "Worldseal Held (Blowout Averted)",
            VictoryKind::Conquest => "Total Conquest",
            VictoryKind::ScoreAtTurnLimit => "Age Survived (Highest Score)",
            VictoryKind::MutualExtinction => "The Surface Burned (No Survivors)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameOutcome {
    pub kind: VictoryKind,
    pub winners: Vec<PlayerId>,
    pub summary: String,
}

#[derive(Clone, Debug)]
pub struct ApocalypseState {
    pub counter: i32,
    /// Highest Strain threshold crossed (so each crisis in `ARMAGEDDON_CRISES` fires once).
    pub triggered_crises: Vec<i32>,
    pub hyborem_spawned: bool,
    pub carcosa_risen: bool,
    pub carcosa_city: Option<CityId>,
    /// Remaining turns once Worldseal Strain reaches 100 (`None` before 100).
    pub countdown: Option<i32>,
    pub carcosa_protected_turns: i32,
    pub lifeboats: Vec<Lifeboat>,
    /// Transparent reasons the counter moved recently.
    pub recent_changes: Vec<String>,
    pub outcome: Option<GameOutcome>,
}

impl Default for ApocalypseState {
    fn default() -> Self {
        Self {
            counter: 0,
            triggered_crises: Vec::new(),
            hyborem_spawned: false,
            carcosa_risen: false,
            carcosa_city: None,
            countdown: None,
            carcosa_protected_turns: 0,
            lifeboats: Vec::new(),
            recent_changes: Vec::new(),
            outcome: None,
        }
    }
}

impl ApocalypseState {
    pub fn shift(&mut self, delta: i32, reason: &str) -> Vec<usize> {
        if delta == 0 || self.carcosa_risen {
            return Vec::new();
        }
        let prev = self.counter;
        self.counter = (self.counter + delta).clamp(0, 100);
        if self.counter != prev {
            self.recent_changes.push(format!("{:+} Strain ({})", self.counter - prev, reason));
            if self.recent_changes.len() > 8 {
                self.recent_changes.remove(0);
            }
        }
        let mut newly_fired = Vec::new();
        for (i, c) in ARMAGEDDON_CRISES.iter().enumerate() {
            if self.counter >= c.threshold && !self.triggered_crises.contains(&c.threshold) {
                self.triggered_crises.push(c.threshold);
                newly_fired.push(i);
            }
        }
        newly_fired
    }

    pub fn act_number(&self, turn: i32) -> u8 {
        if self.carcosa_risen || self.counter >= 100 {
            3
        } else if turn >= 50 || self.counter >= 40 {
            2
        } else {
            1
        }
    }

    pub fn act_label(&self, turn: i32) -> &'static str {
        match self.act_number(turn) {
            1 => "Act I · The Creaking Seal",
            2 => "Act II · Rising Strain",
            _ => "Act III · The Blowout",
        }
    }

    pub fn standing_lifeboats(&self) -> usize {
        self.lifeboats.iter().filter(|l| !l.razed).count()
    }

    pub fn is_lifeboat(&self, city: CityId) -> Option<LifeboatKind> {
        self.lifeboats.iter().find(|l| !l.razed && l.city_id == city).map(|l| l.kind)
    }

    pub fn can_raze_city(&self, city: CityId) -> bool {
        if Some(city) == self.carcosa_city && self.carcosa_protected_turns > 0 {
            return false;
        }
        true
    }

    pub fn demonic_victory_eligible(faction: FactionId) -> bool {
        faction.info().can_demonic_win
    }

    pub fn mortal_victory_eligible(faction: FactionId) -> bool {
        faction.info().can_mortal_win
    }
}
