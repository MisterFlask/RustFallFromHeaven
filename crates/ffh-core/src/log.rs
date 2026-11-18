//! Game message log (replaces C++ `addMessage` / `addReplayMessage` / debug logs).
//!
//! Disabled by default (benchmarks); enable for debugging and UIs. Use the
//! closure form so formatting costs nothing when disabled:
//! `g.log.msg(turn, Some(p), || format!("..."))`.
//!
//! Each entry says who receives it: one player (`player: Some`), or, untagged, an [`Audience`]
//! computed where it is logged (C++ tests each player as it sends a message, so the audience is
//! the players the condition admits at that moment). `msg(turn, None, ..)` is for the tools and
//! the observer only; a message for several players goes through [`GameLog::msg_to`]. It also
//! says what it is: a [`LogMark`] for a located message's map marker and a [`LogTag`] for one a
//! UI links (a battle report, a ritual's news), so that no reader parses the wording.
//!
//! [`CombatNote`]s are a separate, structured record of the fights (`Game::combat_notes`),
//! for tools that look for a kind of fight (ffh-play's war start).

use crate::ids::PlayerId;

/// Who receives an untagged entry ([`LogEntry::player`] `None`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Audience {
    /// The tools and the observer only.
    #[default]
    Tools,
    /// Every player.
    Everyone,
    /// These players (those alive when it was logged that the C++ condition admits).
    Players(Vec<PlayerId>),
}

/// What a message is, for its marker on the map.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LogMark {
    #[default]
    Event,
    /// A city captured or razed.
    City,
    /// A unit of the recipient's died, or a battle the recipient lost.
    Loss,
    /// The recipient's unit destroyed another, or won a battle.
    Kill,
    /// A unit fled or withdrew.
    Retreat,
    /// About a person of note or a House (docs/PERSONS.md): the client shows it as an alert.
    Person,
}

/// What became of a world ritual, in its news.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RitualNews {
    Began,
    Abandoned,
    Completed,
}

impl RitualNews {
    /// The verb of the news (`began`, `abandoned`, `completed`).
    pub fn verb(self) -> &'static str {
        match self {
            RitualNews::Began => "began",
            RitualNews::Abandoned => "abandoned",
            RitualNews::Completed => "completed",
        }
    }
}

/// What a message links to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LogTag {
    #[default]
    None,
    /// An engagement's message: its battle report's id.
    Battle(u32),
    /// A world ritual's news.
    Ritual(RitualNews),
    /// About this person of note (docs/PERSONS.md).
    Person(crate::ids::PersonId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    pub turn: i32,
    pub player: Option<u8>,
    pub text: String,
    /// The plot `(x, y)` the message is about (combat, a city captured or razed), for a UI that
    /// shows it on the map; `None` for the others. Presentation only: not saved (a saved log
    /// reads back without it), and nothing in the rules reads it.
    pub at: Option<(i32, i32)>,
    /// Who receives it when `player` is `None` (not read otherwise).
    pub audience: Audience,
    pub mark: LogMark,
    pub tag: LogTag,
}

impl LogEntry {
    /// An entry with no plot, mark or tag (an untagged one for the tools only).
    pub fn new(turn: i32, player: Option<u8>, text: String) -> LogEntry {
        LogEntry { turn, player, text, at: None, audience: Audience::Tools, mark: LogMark::Event, tag: LogTag::None }
    }

    /// Player `p` receives this entry: it is `p`'s, or untagged with `p` in its audience.
    pub fn reaches(&self, p: PlayerId) -> bool {
        match self.player {
            Some(q) => q == p.0,
            None => match &self.audience {
                Audience::Tools => false,
                Audience::Everyone => true,
                Audience::Players(ps) => ps.contains(&p),
            },
        }
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
    fn push(&mut self, e: LogEntry) {
        if self.cap > 0 && self.entries.len() >= self.cap {
            self.entries.remove(0);
        }
        self.entries.push(e);
    }

    /// A message to player `player`; with `None`, to the tools and the observer only (a message
    /// for several players is [`GameLog::msg_to`]).
    pub fn msg(&mut self, turn: i32, player: Option<PlayerId>, f: impl FnOnce() -> String) {
        if !self.enabled {
            return;
        }
        self.push(LogEntry::new(turn, player.map(|p| p.0), f()));
    }

    /// A diplomacy screen `from` opens with the human `to` ([`DiploEntry`]).
    pub fn diplo(&mut self, turn: i32, from: crate::ids::PlayerId, to: crate::ids::PlayerId, comment: ffh_data::DiploCommentType) {
        if self.enabled {
            self.diplo.push(DiploEntry { turn, from, to, comment });
        }
    }

    /// [`GameLog::msg`] about plot `at` (`(x, y)`).
    pub fn msg_at(&mut self, turn: i32, player: Option<PlayerId>, at: (i32, i32), f: impl FnOnce() -> String) {
        self.msg_marked(turn, player, at, LogMark::Event, LogTag::None, f);
    }

    /// [`GameLog::msg_at`] with a mark and a tag.
    pub fn msg_marked(&mut self, turn: i32, player: Option<PlayerId>, at: (i32, i32), mark: LogMark, tag: LogTag, f: impl FnOnce() -> String) {
        if !self.enabled {
            return;
        }
        self.push(LogEntry { at: Some(at), mark, tag, ..LogEntry::new(turn, player.map(|p| p.0), f()) });
    }

    /// An untagged message to `audience` (the `audience_*` methods of `Game` compute one).
    pub fn msg_to(&mut self, turn: i32, audience: Audience, f: impl FnOnce() -> String) {
        if !self.enabled {
            return;
        }
        self.push(LogEntry { audience, ..LogEntry::new(turn, None, f()) });
    }

    /// [`GameLog::msg_to`] about plot `at` (`(x, y)`), with a mark.
    pub fn msg_to_at(&mut self, turn: i32, audience: Audience, at: (i32, i32), mark: LogMark, f: impl FnOnce() -> String) {
        if !self.enabled {
            return;
        }
        self.push(LogEntry { audience, at: Some(at), mark, ..LogEntry::new(turn, None, f()) });
    }
}

/// Who receives an untagged message, computed where it is logged (an empty audience when the log
/// is off: nothing reads it then).
impl crate::game::Game {
    /// The alive players `admit` admits.
    pub fn audience_where(&self, admit: impl Fn(PlayerId) -> bool) -> Audience {
        if !self.log.enabled {
            return Audience::Players(Vec::new());
        }
        Audience::Players(self.player_ids().filter(|&p| self.players[p].alive && admit(p)).collect())
    }

    /// Player `p`'s team is `t` or has met it.
    pub(crate) fn player_knows_team(&self, p: PlayerId, t: crate::ids::TeamId) -> bool {
        let team = self.players[p].team;
        team == t || self.teams[team].has_met[t]
    }

    /// The alive players whose team is `t` or has met it.
    pub fn audience_met(&self, t: crate::ids::TeamId) -> Audience {
        self.audience_where(|p| self.player_knows_team(p, t))
    }

    /// The alive players whose team is one of `a` and `b` or has met both (C++ `CvTeam::declareWar`,
    /// `makePeace`, `setVassal`).
    pub fn audience_met_both(&self, a: crate::ids::TeamId, b: crate::ids::TeamId) -> Audience {
        let team = |p: PlayerId| self.players[p].team;
        self.audience_where(|p| team(p) == a || team(p) == b || (self.player_knows_team(p, a) && self.player_knows_team(p, b)))
    }

    /// The alive players whose team is or has met the team of `p`, and that of `other` when there
    /// is one (C++ event world news: `p` and those who have met it, and the other player).
    pub fn audience_event_news(&self, p: PlayerId, other: Option<PlayerId>) -> Audience {
        let (t, o) = (self.players[p].team, other.map(|o| self.players[o].team));
        self.audience_where(|q| self.player_knows_team(q, t) && o.is_none_or(|o| self.player_knows_team(q, o)))
    }

    /// The alive players whose team has revealed city `c` (C++ `isRevealed(team, false)`).
    pub fn audience_revealed_city(&self, c: crate::ids::CityId) -> Audience {
        self.audience_where(|p| self.city_is_revealed(c, self.players[p].team, false))
    }

    /// The alive players whose team has revealed plot `plot`.
    pub fn audience_revealed_plot(&self, plot: crate::ids::PlotIdx) -> Audience {
        self.audience_where(|p| self.map.plots[plot].revealed[self.players[p].team])
    }

    /// The alive voting members of vote source `vs`.
    pub fn audience_voting_members(&self, vs: ffh_data::VoteSourceType) -> Audience {
        self.audience_where(|p| self.player_is_voting_member(p, vs))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The calls `log.msg(` / `log.msg_at(` in `text` whose player argument is a literal `None`,
    /// as line numbers (1-based).
    fn literal_none_calls(text: &str) -> Vec<usize> {
        let mut found = Vec::new();
        let bytes = text.as_bytes();
        for name in [concat!("log", ".msg("), concat!("log", ".msg_at(")] {
            let mut from = 0;
            while let Some(k) = text[from..].find(name) {
                let start = from + k;
                from = start + name.len();
                // Skip the first argument (the turn) to its comma at depth 0.
                let mut i = from;
                let mut depth = 0i32;
                while i < bytes.len() {
                    match bytes[i] {
                        b'(' | b'[' | b'{' => depth += 1,
                        b')' | b']' | b'}' => {
                            if depth == 0 {
                                break;
                            }
                            depth -= 1;
                        }
                        b',' if depth == 0 => break,
                        _ => {}
                    }
                    i += 1;
                }
                if i >= bytes.len() || bytes[i] != b',' {
                    continue;
                }
                let rest = text[i + 1..].trim_start();
                let is_none = rest.strip_prefix("None").is_some_and(|r| !r.starts_with(|c: char| c.is_alphanumeric() || c == '_'));
                if is_none {
                    found.push(text[..start].matches('\n').count() + 1);
                }
            }
        }
        found
    }

    fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                rust_files(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }

    #[test]
    fn untagged_messages_state_their_audience() {
        assert_eq!(literal_none_calls("g.log.msg(turn,\n  None, || x);\ng.log.msg_at(t(1, 2), Some(p), at, || y);\ng.log.msg(t, Nonesuch, f)"), vec![1]);
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&src, &mut files);
        files.sort();
        let mut bad = Vec::new();
        for f in &files {
            let text = std::fs::read_to_string(f).unwrap();
            for line in literal_none_calls(&text) {
                bad.push(format!("{}:{line}", f.strip_prefix(&src).unwrap().display()));
            }
        }
        assert!(
            bad.is_empty(),
            "a log message passes None as its player: state who receives it with `GameLog::msg_to` (an `Audience`: \
             `Audience::Tools` for the tools and the observer only, `Everyone`, or the players a `Game::audience_*` \
             method admits): {bad:?}"
        );
    }

    #[test]
    fn who_an_entry_reaches() {
        let p = PlayerId(1);
        assert!(LogEntry::new(1, Some(1), "x".into()).reaches(p));
        assert!(!LogEntry::new(1, Some(2), "x".into()).reaches(p));
        assert!(!LogEntry::new(1, None, "x".into()).reaches(p));
        assert!(LogEntry { audience: Audience::Everyone, ..LogEntry::new(1, None, "x".into()) }.reaches(p));
        let some = LogEntry { audience: Audience::Players(vec![PlayerId(0), p]), ..LogEntry::new(1, None, "x".into()) };
        assert!(some.reaches(p) && !some.reaches(PlayerId(2)));
    }
}
