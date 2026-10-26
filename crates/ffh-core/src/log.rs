//! Game message log (replaces C++ `addMessage` / `addReplayMessage` / debug logs).
//!
//! Disabled by default (benchmarks); enable for debugging and UIs. Use the
//! closure form so formatting costs nothing when disabled:
//! `g.log.msg(turn, Some(p), || format!("..."))`.

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

#[derive(Clone, Debug, Default)]
pub struct GameLog {
    pub enabled: bool,
    pub entries: Vec<LogEntry>,
    /// Keep at most this many entries (0 = unbounded).
    pub cap: usize,
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
