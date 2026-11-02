//! Game message log (replaces C++ `addMessage` / `addReplayMessage` / debug logs).
//!
//! Disabled by default (benchmarks); enable for debugging and UIs. Use the
//! closure form so formatting costs nothing when disabled:
//! `g.log.msg(turn, Some(p), || format!("..."))`.
//!
//! [`CombatNote`]s are a separate, structured record of the fights (`Game::combat_notes`),
//! for tools that look for a kind of fight (ffh-play's war start).

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    pub turn: i32,
    pub player: Option<u8>,
    pub text: String,
    /// The plot `(x, y)` the message is about (combat, a city captured or razed), for a UI that
    /// shows it on the map; `None` for the others. Presentation only: not saved (a saved log
    /// reads back without it), and nothing in the rules reads it.
    pub at: Option<(i32, i32)>,
}

impl LogEntry {
    /// An entry with no plot.
    pub fn new(turn: i32, player: Option<u8>, text: String) -> LogEntry {
        LogEntry { turn, player, text, at: None }
    }
}

/// A leader opening the diplomacy screen with a human (C++ `gDLL->beginDiplomacy` with
/// `setAIContact(true)`): `from`'s leader says `comment` (`AI_DIPLOCOMMENT_FIRST_CONTACT`,
/// `AI_DIPLOCOMMENT_DECLARE_WAR`) to the human `to`. Presentation only, like the messages: not
/// saved, and nothing in the rules reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiploEntry {
    pub turn: i32,
    pub from: crate::ids::PlayerId,
    pub to: crate::ids::PlayerId,
    pub comment: ffh_data::DiploCommentType,
}

#[derive(Clone, Debug, Default)]
pub struct GameLog {
    pub enabled: bool,
    pub entries: Vec<LogEntry>,
    /// Keep at most this many entries (0 = unbounded).
    pub cap: usize,
    /// The diplomacy screens AI leaders opened with humans since a UI last took them (not
    /// saved; not capped: a UI drains them with the messages).
    pub diplo: Vec<DiploEntry>,
}

impl GameLog {
    pub fn msg(&mut self, turn: i32, player: Option<crate::ids::PlayerId>, f: impl FnOnce() -> String) {
        if !self.enabled {
            return;
        }
        if self.cap > 0 && self.entries.len() >= self.cap {
            self.entries.remove(0);
        }
        self.entries.push(LogEntry { turn, player: player.map(|p| p.0), text: f(), at: None });
    }

    /// A diplomacy screen `from` opens with the human `to` ([`DiploEntry`]).
    pub fn diplo(&mut self, turn: i32, from: crate::ids::PlayerId, to: crate::ids::PlayerId, comment: ffh_data::DiploCommentType) {
        if self.enabled {
            self.diplo.push(DiploEntry { turn, from, to, comment });
        }
    }

    /// [`GameLog::msg`] about plot `at` (`(x, y)`).
    pub fn msg_at(&mut self, turn: i32, player: Option<crate::ids::PlayerId>, at: (i32, i32), f: impl FnOnce() -> String) {
        if !self.enabled {
            return;
        }
        self.msg(turn, player, f);
        if let Some(e) = self.entries.last_mut() {
            e.at = Some(at);
        }
    }
}

/// One fight as it starts, recorded while `Game::combat_notes` is `Some` (off by default; not game
/// state, not saved). A duel (`unit_update_combat`, once the attack is set up and before it is
/// resolved): the stacks are the units that can fight (`unit_can_fight`) of the attacker's team on
/// the attacker's plot and of the defender's team on the attacked plot, the two units included. An
/// engagement (`engage`, once both sides are committed): the stacks are the committed units, and
/// `attacker` / `defender` the first of each side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatNote {
    pub turn: i32,
    pub attacker: crate::ids::UnitId,
    pub defender: crate::ids::UnitId,
    pub attacker_owner: crate::ids::PlayerId,
    pub defender_owner: crate::ids::PlayerId,
    /// The attacked plot.
    pub plot: crate::ids::PlotIdx,
    pub attacker_stack: i32,
    pub defender_stack: i32,
}
