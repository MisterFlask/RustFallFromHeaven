//! The session (design §2.2): owns the `Game` and drives it the way the Civ4 exe does for one
//! human player. `advance` runs `update()` until the human has control (or the game ends);
//! `execute` applies one command between `update()` calls, then lets the game settle. Inputs
//! are journaled, the log is drained at deterministic points, turn boundaries are autosaved.
//!
//! An observed session (`NewGame::observe`, or a loaded AI-only save) has no human: the AI plays
//! every slot (`ffh autoplay`'s game), and the session stops at turn boundaries instead of at a
//! human's control points; `end_turn` / `autoplay {turns}` play one or more turns, nothing else is
//! accepted, and `human` is only the player the view follows (`Session::set_viewer`).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ffh_ai::Ai;
use ffh_core::controller::ControllerRef;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::log::LogEntry;
use ffh_data::*;

use crate::command::{self, Action, CmdErr, CmdOk, Command, Plan};
use crate::decision::{self, Pending};
use crate::guard::{self, QueryGuard};
use crate::human::{Answerer, HumanController, SessionShared, Stats};
use crate::journal::{self, Cursor, Journal, JournalAnswerer, Line, Stamp};
use crate::json::Json;
use crate::query::{self, Query, QueryErr, UiState};
use crate::report::{self, MessageStore, Report, Snapshot};
use crate::save::{self, SavePaths, Sidecar};
use crate::setup::{self, NewGame, HUMAN_SLOT};
use crate::view::fog::LogFog;
use crate::view::Viewer;

/// Who has control.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The human can act (a control point).
    Human(PlayerId),
    /// The game is running (AI turns, end of turn, the human's missions).
    Running,
    Over,
}

/// Why `advance` returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stop {
    Human,
    Over,
    /// A turn did not finish (the watchdog): diagnostics.
    Stuck(String),
}

/// Session settings (design §3).
#[derive(Clone, Debug)]
pub struct SessionOptions {
    /// The game's directory: autosaves in `auto/`, the journal, manual saves. `None`: no files
    /// (the journal is kept in memory).
    pub saves_dir: Option<PathBuf>,
    /// Autosaves kept (0 = none).
    pub autosave_keep: usize,
    /// Port of `updateTestEndTurn`'s AutoMoves: when nothing is ready but groups have queued
    /// orders, send `auto_moves` (journaled), once per control episode.
    pub auto_moves_when_idle: bool,
    /// Most `update()` calls a command's settle runs.
    pub settle_limit: u32,
    /// Write a journal.
    pub journal: bool,
}

impl Default for SessionOptions {
    fn default() -> SessionOptions {
        SessionOptions { saves_dir: None, autosave_keep: 20, auto_moves_when_idle: true, settle_limit: 10_000, journal: true }
    }
}

/// Session-level failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayError {
    /// Bad new-game settings.
    Setup(String),
    /// A save that cannot be loaded (or taken over).
    Load(String),
    Io(String),
    /// Not now (a decision is open, not at a control point, no saves directory).
    Refused(String),
}

impl std::fmt::Display for PlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayError::Setup(s) => write!(f, "setup: {s}"),
            PlayError::Load(s) => write!(f, "load: {s}"),
            PlayError::Io(s) => write!(f, "io: {s}"),
            PlayError::Refused(s) => write!(f, "refused: {s}"),
        }
    }
}

impl std::error::Error for PlayError {}

/// Turn slices `advance` runs in one game turn before it gives up (as ffh-cli's run loop).
pub const WATCHDOG_SLICES: u64 = 200_000;

/// Where [`Session::advance`] spends its time while the game runs (T13, `bench.rs`): the rules'
/// `update()` slices, and the session's own work around them in the same slices (the slot-move
/// check, the log drain and its fog, the turn-boundary checkpoint and its checksum, the per-slice
/// callback), the autosave counted apart. Both are wall time read around each part of each
/// slice, so the two shares see the same host load: their ratio holds on a loaded host where
/// two separate runs differ by a quarter (P6 review). Off unless `Session::profile` is set.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AdvanceProfile {
    pub update: Duration,
    pub own: Duration,
    pub autosave: Duration,
    pub slices: u64,
}

/// A game played by one human (design §3), or watched by an observer.
pub struct Session {
    pub g: Game,
    /// The human player; in an observed session, the player the view follows.
    pub human: PlayerId,
    /// An AI-only game watched by an observer (see the module docs).
    observing: bool,
    /// Observed: the turns to play before the session stops again (0: stopped at a turn boundary,
    /// or at the start of a new game).
    to_play: i32,
    /// The game's settings, when known (a new game; a save whose sidecar has them): flags quote
    /// them, so that a spectator's bug report replays from the seed.
    setup: Option<NewGame>,
    pub shared: Arc<SessionShared>,
    /// The client state queries use (debug reveal, the interface path finder), shared with the
    /// human's controller for the queries answered during decisions (and with transports).
    pub ui: Arc<Mutex<UiState>>,
    advisor: Arc<Ai>,
    messages: MessageStore,
    /// Which log entries the human receives (design §4.4).
    fog: LogFog,
    /// The turn report's snapshot, taken when the human ends its turn.
    snapshot: Option<Snapshot>,
    /// The last turn report (built at the control point after the snapshot).
    report: Option<Report>,
    opts: SessionOptions,
    /// The AutoMoves rule fired in this control episode (since the human's last applied
    /// command); saved in the sidecar, so a resumed session does not send it again.
    auto_moves_tried: bool,
    last_autosave: Option<SavePaths>,
    /// A new game not yet at its first control point (see `advance`).
    start_pending: bool,
    /// When set, `advance` adds up where its time goes ([`AdvanceProfile`]; the bench).
    pub profile: Option<AdvanceProfile>,
}

impl Session {
    /// Attaches controllers and the human-play flags to `g` (`observe`: the AI on every slot and
    /// the observer's flags, `human` being the player the view follows, with debug reveal on);
    /// `journal` gets everything.
    fn assemble(mut g: Game, human: PlayerId, answerer: Arc<dyn Answerer>, opts: SessionOptions, journal: Journal, observe: bool) -> Session {
        let advisor = Arc::new(Ai::new());
        let shared = Arc::new(SessionShared::new(journal));
        let ui: Arc<Mutex<UiState>> = Arc::default();
        if observe {
            let ai = ffh_ai::ai_controller();
            for p in g.player_ids() {
                g.controllers[p] = ai.clone();
            }
            setup::configure_observer(&mut g);
            UiState::lock(&ui).reveal = true;
        } else {
            let controller: ControllerRef = Arc::new(HumanController::with_ui(advisor.clone(), answerer, shared.clone(), ui.clone()));
            setup::attach_controllers(&mut g, human, ffh_ai::ai_controller(), controller);
            setup::configure_human(&mut g, human);
        }
        let fog = LogFog::new(&g, human);
        Session {
            g,
            human,
            observing: observe,
            to_play: 0,
            setup: None,
            shared,
            ui,
            advisor,
            messages: MessageStore::default(),
            fog,
            snapshot: None,
            report: None,
            opts,
            auto_moves_tried: false,
            last_autosave: None,
            start_pending: false,
            profile: None,
        }
    }

    fn make_journal(opts: &SessionOptions) -> Result<Journal, PlayError> {
        Ok(match (opts.journal, &opts.saves_dir) {
            (false, _) => Journal::off(),
            (true, None) => Journal::memory(),
            (true, Some(dir)) => Journal::create_in(dir).map_err(|e| PlayError::Io(format!("{}: {e}", dir.display())))?,
        })
    }

    /// A new game (design §2.6 "New game"); returns before the first `advance`.
    pub fn new_game(ng: &NewGame, answerer: Arc<dyn Answerer>, opts: SessionOptions) -> Result<Session, PlayError> {
        let journal = Self::make_journal(&opts)?;
        Self::new_game_with(ng, answerer, opts, journal)
    }

    fn new_game_with(ng: &NewGame, answerer: Arc<dyn Answerer>, opts: SessionOptions, journal: Journal) -> Result<Session, PlayError> {
        let d = ffh_data::global();
        let g = if ng.observe { setup::build_observed_game(d, ng) } else { setup::build_game(d, ng) }.map_err(PlayError::Setup)?;
        let mut s = Session::assemble(g, HUMAN_SLOT, answerer, opts, journal, ng.observe);
        s.setup = Some(ng.clone());
        s.journal(&Line::New { setup: ng.to_json() });
        s.g.game_init();
        if !ng.observe {
            setup::set_human_options(&mut s.g, HUMAN_SLOT);
        }
        s.start_pending = true;
        Ok(s)
    }

    /// A saved game (a turn boundary or a human control point). The human is the sidecar's, else
    /// the first human slot. Without a sidecar no event popup is pending.
    pub fn load(save: &[u8], sidecar: Option<&Sidecar>, answerer: Arc<dyn Answerer>, opts: SessionOptions) -> Result<Session, PlayError> {
        let journal = Self::make_journal(&opts)?;
        Self::load_with(save, sidecar, answerer, opts, journal)
    }

    fn load_with(save: &[u8], sidecar: Option<&Sidecar>, answerer: Arc<dyn Answerer>, opts: SessionOptions, journal: Journal) -> Result<Session, PlayError> {
        let g = Game::load(ffh_data::global(), save).map_err(|e| PlayError::Load(e.to_string()))?;
        // An AI-only save (an observed game's, ...) is observed; take over plays one of its slots
        // instead.
        let observe = sidecar.is_some_and(|sc| sc.observe) || g.num_human_players() == 0;
        if observe && g.num_human_players() > 0 {
            return Err(PlayError::Load("the save has a human player but its sidecar says observed".into()));
        }
        if observe && !g.at_turn_boundary() && sidecar.is_none() {
            return Err(PlayError::Load("an AI-only save is observed from a turn boundary (an autosave)".into()));
        }
        let human = match sidecar {
            Some(sc) => sc.human,
            None if observe => g.civ_player_ids().find(|&p| g.players[p].alive).unwrap_or(PlayerId(0)),
            None => g
                .civ_player_ids()
                .find(|&p| g.setup.players[p.idx()].human)
                .ok_or_else(|| PlayError::Load("the save has no human player (use take over)".into()))?,
        };
        if human.idx() >= g.num_civ_players() || (!observe && !g.setup.players[human.idx()].human) {
            return Err(PlayError::Load(format!("player {} is not a human player of this save", human.0)));
        }
        let mut s = Session::assemble(g, human, answerer, opts, journal, observe);
        s.journal(&Line::Load { fnv: journal::fnv_hex(save), human });
        if let Some(sc) = sidecar {
            s.setup = sc.setup.as_ref().and_then(|j| NewGame::from_json(s.g.d, j).ok());
            if observe {
                s.to_play = sc.to_play.max(0);
            }
            *s.shared.pending_events.lock().unwrap() = sc.pending_events.clone();
            s.messages = MessageStore::from_tail(sc.messages_tail.clone());
            s.auto_moves_tried = sc.auto_moves_tried;
            // The report is presentation only: one that does not read (another data version),
            // or whose ids do not fit this game (a mismatched or corrupted sidecar), is dropped.
            // The snapshot's log mark counts the messages of the saving session.
            s.snapshot = sc.report_snapshot.as_ref().and_then(|j| Snapshot::from_json(s.g.d, j).ok()).filter(|snap| snap.fits(&s.g)).map(|mut snap| {
                snap.log_mark = snap.log_mark.min(s.messages.total());
                snap
            });
            s.report = sc.report.as_ref().and_then(Report::from_json);
        }
        Ok(s)
    }

    /// Takes over slot `p` of an AI-only save at a turn boundary (a debugging aid, design §2.6).
    pub fn take_over(save: &[u8], p: PlayerId, answerer: Arc<dyn Answerer>, opts: SessionOptions) -> Result<Session, PlayError> {
        let journal = Self::make_journal(&opts)?;
        Self::take_over_with(save, p, answerer, opts, journal)
    }

    fn take_over_with(save: &[u8], p: PlayerId, answerer: Arc<dyn Answerer>, opts: SessionOptions, journal: Journal) -> Result<Session, PlayError> {
        let mut g = Game::load(ffh_data::global(), save).map_err(|e| PlayError::Load(e.to_string()))?;
        if !g.at_turn_boundary() {
            return Err(PlayError::Load("take over needs a save at a turn boundary".into()));
        }
        if p.idx() >= g.num_civ_players() || !g.players[p].alive {
            return Err(PlayError::Load(format!("player {} is not an alive civilization", p.0)));
        }
        if g.num_human_players() > 0 {
            return Err(PlayError::Load("the save already has a human player (load it instead)".into()));
        }
        g.setup.players[p.idx()].human = true;
        g.setup.players[p.idx()].slot_status = ffh_core::setup::SlotStatus::Taken;
        g.player_update_human(p);
        let mut s = Session::assemble(g, p, answerer, opts, journal, false);
        setup::set_human_options(&mut s.g, p);
        s.journal(&Line::TakeOver { fnv: journal::fnv_hex(save), human: p });
        Ok(s)
    }

    fn journal(&self, line: &Line) {
        self.shared.journal.lock().unwrap().write(line);
    }

    // ---- phase, advance, execute --------------------------------------------------------------

    /// The phase predicate (pure, design §2.2). Observed: `Human(viewer)` while stopped.
    pub fn phase(&self) -> Phase {
        match self.observe_state() {
            Some(to_play) => observed_phase(&self.g, self.human, to_play),
            None => phase(&self.g, self.human),
        }
    }

    /// `Some(turns left to play)` in an observed session.
    pub fn observe_state(&self) -> Option<i32> {
        self.observing.then_some(self.to_play)
    }

    /// The AI autoplay's turns left (`ai_auto_play`), or an observed session's turns to play.
    pub fn autoplay_left(&self) -> i32 {
        if self.observing {
            self.to_play
        } else {
            self.g.ai_auto_play
        }
    }

    /// Observed: stops at the next stop whatever turns were left to play (a save taken during a
    /// multi-turn autoplay loads with them; `ffh-play observe --load` replays from the save).
    pub fn halt(&mut self) {
        if self.observing {
            self.to_play = 0;
        }
    }

    /// Observed: the view follows player `p` (its empire in the top bar, its messages first);
    /// the game is not changed.
    pub fn set_viewer(&mut self, p: PlayerId) -> Result<(), PlayError> {
        if !self.observing {
            return Err(PlayError::Refused("only an observed game follows another player".into()));
        }
        if p.idx() >= self.g.num_civ_players() || !self.g.players[p].ever_alive {
            return Err(PlayError::Refused(format!("player {} is not a civilization of this game", p.0)));
        }
        self.human = p;
        self.fog = LogFog::new(&self.g, p);
        UiState::lock(&self.ui).view_version += 1;
        Ok(())
    }

    /// Runs the game until the human has control or the game ends (design §2.2). `on_slice` is
    /// called after every `update()` while running, with the session (the HTTP transport
    /// publishes status there and answers the queries queued meanwhile: `Session::query` reads
    /// under a `QueryGuard`; plain fields of `s.g` may be read directly).
    pub fn advance(&mut self, on_slice: &mut dyn FnMut(&Session)) -> Stop {
        if self.observing {
            return self.advance_observed(on_slice);
        }
        let mut slices: u64 = 0;
        // A stop of the AI autoplay is asked while this `advance` runs (`on_slice`).
        self.shared.stop_autoplay.store(false, std::sync::atomic::Ordering::SeqCst);
        loop {
            // The phase predicate is the session's own work too (`AdvanceProfile`).
            let tp = self.profile.is_some().then(Instant::now);
            match self.phase() {
                Phase::Over => {
                    self.drain_log();
                    // The turn report of the turn that ended the game (the end-turn snapshot
                    // against the final state), as at a control point.
                    self.build_report(self.human);
                    return Stop::Over;
                }
                Phase::Human(h) => {
                    self.drain_log();
                    self.launch_events(h);
                    // A new game is saved at its first control point (the human's first turn;
                    // `game_init` leaves the first player's turn active, so the game starts
                    // without a turn boundary).
                    if std::mem::take(&mut self.start_pending) {
                        self.checkpoint(Some("start"));
                    }
                    // updateTestEndTurn: nothing ready (and nothing busy) but queued orders ->
                    // CvNetAutoMoves. At most once per control episode (until the human's next
                    // applied command), so a mission that leaves its group ready to auto again
                    // cannot spin.
                    if self.auto_moves_rule(h) {
                        self.auto_moves_tried = true;
                        self.apply_journaled(h, &Command::AutoMoves, &Action::AutoMoves);
                        continue;
                    }
                    self.build_report(h);
                    UiState::lock(&self.ui).view_version += 1;
                    return Stop::Human;
                }
                Phase::Running => {
                    if self.stop_autoplay_due() {
                        // `setAIAutoPlay(0)`: the human's slot is human again at once (its turn,
                        // if it is under way, goes on as the human's).
                        self.journal(&Line::Cmd { stamp: Stamp::of(&self.g), cmd: Command::AutoPlay { turns: 0 }.to_json() });
                        self.g.set_ai_auto_play(0);
                        continue;
                    }
                    let t0 = tp.map(|_| Instant::now());
                    let log_mark = self.g.log.entries.len();
                    self.g.update();
                    let t1 = t0.map(|_| Instant::now());
                    let saved = self.profile.map_or(Duration::ZERO, |p| p.autosave);
                    self.follow_human(MovedIn::Update { log_mark });
                    if self.g.at_turn_boundary() {
                        self.boundary();
                        slices = 0;
                    }
                    on_slice(self);
                    if let (Some(tp), Some(t0), Some(t1), Some(p)) = (tp, t0, t1, self.profile.as_mut()) {
                        p.update += t1 - t0;
                        p.own += (t0 - tp) + t1.elapsed().saturating_sub(p.autosave - saved);
                        p.slices += 1;
                    }
                    slices += 1;
                    if slices > WATCHDOG_SLICES {
                        return Stop::Stuck(diagnose(&self.g));
                    }
                }
            }
        }
    }

    /// [`Session::advance`] of an observed session: plays `to_play` turns (to the turn boundary
    /// after the last), then stops; a new game stops first at its start. A stop asked while it
    /// runs (`autoplay {turns: 0}`) ends the turn under way (journaled where it takes effect, so
    /// a replay stops at the same boundary).
    fn advance_observed(&mut self, on_slice: &mut dyn FnMut(&Session)) -> Stop {
        let mut slices: u64 = 0;
        self.shared.stop_autoplay.store(false, std::sync::atomic::Ordering::SeqCst);
        loop {
            // A victory under autorun leaves the game EXTENDED ("just one more turn"), as in
            // `ffh autoplay`, whose games end there too: the observed game is over.
            if self.g.game_state != GameStateType::GAMESTATE_ON {
                self.drain_log();
                return Stop::Over;
            }
            if self.to_play <= 0 {
                self.drain_log();
                if std::mem::take(&mut self.start_pending) {
                    self.checkpoint(Some("start"));
                }
                UiState::lock(&self.ui).view_version += 1;
                return Stop::Human;
            }
            if self.stop_autoplay_due() {
                self.journal(&Line::Cmd { stamp: Stamp::of(&self.g), cmd: Command::AutoPlay { turns: 0 }.to_json() });
                self.to_play = if self.g.at_turn_boundary() { 0 } else { 1 };
                continue;
            }
            self.g.update();
            if self.g.at_turn_boundary() {
                // Before the autosave, so that its sidecar has the turns still to play (a replay
                // from it goes on as the uninterrupted session did).
                self.to_play -= 1;
                self.boundary();
                slices = 0;
            }
            on_slice(self);
            slices += 1;
            if slices > WATCHDOG_SLICES {
                return Stop::Stuck(diagnose(&self.g));
            }
        }
    }

    /// Before a turn slice: whether the AI autoplay stops now. Live, a transport asked for it
    /// while the game ran (`SessionShared::request_stop_autoplay`; dropped when the autoplay
    /// already ended); replaying, the journal has the stop at this slice's stamp.
    fn stop_autoplay_due(&mut self) -> bool {
        let asked = self.shared.stop_autoplay.swap(false, std::sync::atomic::Ordering::SeqCst);
        let replayed = self.shared.journal.lock().unwrap().take_running_cmd(Stamp::of(&self.g)).is_some();
        (asked || replayed) && self.autoplay_left() > 0
    }

    /// The AutoMoves rule of `updateTestEndTurn` for human `h` at a control point: nothing is
    /// ready (and nothing busy, which the control point implies) but some group has orders to
    /// continue, and the rule has not fired in this control episode.
    fn auto_moves_rule(&self, h: PlayerId) -> bool {
        self.opts.auto_moves_when_idle && !self.auto_moves_tried && decision::ready_groups(&self.g, h).is_empty() && self.g.player_has_auto_unit(h)
    }

    /// At a control point, the AutoMoves rule would fire now: the exe checks it every frame, so
    /// after a command that leaves nothing ready while orders are queued (the last ready unit
    /// skipped, fortified or given a multi-turn order) its next frame sends AutoMoves. A client
    /// that sends commands without calling `advance` after each one (the transports) calls
    /// `advance` when this holds; `advance` applies the rule.
    pub fn auto_moves_due(&self) -> bool {
        if self.observing {
            return false;
        }
        match self.phase() {
            Phase::Human(h) => !self.shared.decision_open() && self.auto_moves_rule(h),
            _ => false,
        }
    }

    /// Applies `cmd` at a control point (design §2.2): validate (pure), journal, the C++ port,
    /// then settle. Confirmations and errors change nothing.
    pub fn execute(&mut self, cmd: &Command) -> Result<CmdOk, CmdErr> {
        if self.observing {
            return self.execute_observed(cmd);
        }
        if self.shared.decision_open() {
            return Err(CmdErr::DecisionPending);
        }
        let Phase::Human(h) = self.phase() else { return Err(CmdErr::NotYourTurn) };
        let events = self.shared.open_events_of(h);
        let plan = {
            let g = QueryGuard::new(&self.g);
            command::validate(&g, h, cmd, &command::Ctx { events: &events })?
        };
        match plan {
            Plan::Noop => Ok(CmdOk::Applied { log: Vec::new(), accepted: None }),
            // An offer the AI declines: nothing changes (C++ sends no message).
            Plan::Declined => Ok(CmdOk::Applied { log: Vec::new(), accepted: Some(false) }),
            Plan::Confirm(c) => Ok(CmdOk::NeedsConfirm(c)),
            Plan::Apply(action) => {
                // The human acted: a new control episode, in which the AutoMoves rule may fire
                // again. Reset before applying, so a turn boundary autosaved during the settle
                // records the episode's state as the uninterrupted run has it.
                self.auto_moves_tried = false;
                let mut ok = self.apply_journaled(h, cmd, &action);
                if let (Action::ImplementDeal { .. }, CmdOk::Applied { accepted, .. }) = (&action, &mut ok) {
                    *accepted = Some(true);
                }
                Ok(ok)
            }
        }
    }

    /// [`Session::execute`] of an observed session: while stopped, `end_turn` plays one turn and
    /// `autoplay {turns: N}` plays N; nothing else changes an AI-only game.
    fn execute_observed(&mut self, cmd: &Command) -> Result<CmdOk, CmdErr> {
        let Phase::Human(_) = self.phase() else { return Err(CmdErr::NotYourTurn) };
        let turns = match cmd {
            Command::EndTurn { .. } => 1,
            Command::AutoPlay { turns } if (1..=10_000).contains(turns) => *turns,
            Command::AutoPlay { .. } => return Err(CmdErr::Invalid("autoplay: 1 to 10000 turns".into())),
            _ => return Err(CmdErr::Cannot(format!("observing an AI-only game: the AI plays every civilization (`{}` is not available; end_turn and autoplay play turns)", cmd.name()))),
        };
        self.journal(&Line::Cmd { stamp: Stamp::of(&self.g), cmd: cmd.to_json() });
        self.to_play = turns;
        UiState::lock(&self.ui).view_version += 1;
        Ok(CmdOk::Applied { log: Vec::new(), accepted: None })
    }

    fn apply_journaled(&mut self, h: PlayerId, cmd: &Command, action: &Action) -> CmdOk {
        self.journal(&Line::Cmd { stamp: Stamp::of(&self.g), cmd: cmd.to_json() });
        self.drain_log();
        let mark = self.messages.total();
        match action {
            Action::Event { triggered, .. } => self.shared.pending_events.lock().unwrap().retain(|e| (e.player, e.triggered) != (h, *triggered)),
            // C++: closing an event popup without a choice does nothing; the popup is gone.
            Action::EndTurn { drop_events: true, .. } => self.shared.pending_events.lock().unwrap().retain(|e| e.player != h),
            _ => {}
        }
        if let Action::EndTurn { .. } = action {
            // The turn report diffs what the human knows now against its next control point.
            self.snapshot = Some(report::snapshot(&self.g, h, mark));
        }
        command::apply(&mut self.g, h, action);
        self.follow_human(MovedIn::Command);
        self.settle();
        // Back at a control point: popups the command queued are shown now (as the exe shows
        // them in its next frames).
        if let Phase::Human(h) = self.phase() {
            self.launch_events(h);
        }
        // The human's messages since the command started (turn boundaries during the settle
        // drained some of them already).
        self.drain_log();
        let log = self.messages.since(mark).to_vec();
        UiState::lock(&self.ui).view_version += 1;
        CmdOk::Applied { log, accepted: None }
    }

    /// At a control point after the human ended its turn: the turn report (under a guard).
    fn build_report(&mut self, h: PlayerId) {
        if let Some(snap) = self.snapshot.take() {
            let messages = self.messages.since(snap.log_mark).to_vec();
            self.report = Some(guard::read(&self.g, |g| Report::build(&snap, &Viewer::new(g, h, false), messages)));
        }
    }

    /// The exe's next frames after a command: at least one `update()` (the governor, war and
    /// timers react), then while a human group is busy (missions in progress, combat) or one the
    /// AI moves waits for orders.
    fn settle(&mut self) {
        let mut n = 0;
        loop {
            let log_mark = self.g.log.entries.len();
            self.g.update();
            self.follow_human(MovedIn::Update { log_mark });
            n += 1;
            if self.g.at_turn_boundary() {
                self.boundary();
            }
            if self.phase() == Phase::Over || n >= self.opts.settle_limit || !(human_busy(&self.g, self.human) || ai_groups_waiting(&self.g, self.human)) {
                break;
            }
        }
    }

    /// Follows the human to another slot. FfH moves the human player with
    /// `CvInitCore::reassignPlayerAdvanced` (the Impersonate Leader spell and the end of its
    /// timer, the "switch civilizations" event, the High to Low challenge): the slot's human flag,
    /// options, controller and the active player move (ffh-core `reassign_player_advanced`,
    /// CvInitCore.cpp:474-482); the session moves its own `human`, fog and report snapshot.
    ///
    /// The log entries since the last drain are split at the move in C++: those before go to
    /// the old slot's player, those after to the new one, and the old slot's later messages
    /// reach nobody (the active player has moved). Without a log marker at the move (P7), the
    /// session splits them by the rules call that moved the human (`at`).
    fn follow_human(&mut self, at: MovedIn) {
        if self.observing || self.g.players[self.human].human {
            return;
        }
        let Some(new) = self.g.civ_player_ids().find(|&p| self.g.players[p].human) else { return };
        match at {
            MovedIn::Command => self.drain_log(),
            MovedIn::Update { log_mark } => {
                // The entries of earlier slices are the old slot's; the update's wait for the
                // next drain, for the new slot.
                let update = self.g.log.entries.split_off(log_mark.min(self.g.log.entries.len()));
                self.drain_log();
                self.g.log.entries = update;
            }
        }
        self.human = new;
        debug_assert_eq!(self.g.active_player, Some(new), "ffh-core moves the active player with the human");
        self.fog = LogFog::new(&self.g, new);
        // The report's snapshot was of the old slot.
        self.snapshot = None;
        self.shared.stats.lock().unwrap().reassigned += 1;
    }

    /// At a turn boundary: drain the log, journal a checkpoint, autosave.
    fn boundary(&mut self) {
        self.checkpoint(None);
    }

    /// Drains the log, journals a checkpoint and autosaves: the ring autosave of a turn
    /// boundary, or (`start`) the new game's `auto/start` at its first control point.
    fn checkpoint(&mut self, start: Option<&str>) {
        self.drain_log();
        let turn = self.g.game_turn();
        self.journal(&Line::Checkpoint { turn, checksum: self.g.calculate_sync_checksum() });
        if let (Some(dir), true) = (&self.opts.saves_dir, self.opts.autosave_keep > 0) {
            let t0 = Instant::now();
            let sidecar = self.sidecar();
            let r = match start {
                None => save::autosave(dir, turn, &self.g.save(), &sidecar, self.opts.autosave_keep),
                Some(name) => save::write_bundle(&dir.join("auto"), name, &self.g.save(), &sidecar),
            };
            match r {
                Ok(p) => self.last_autosave = Some(p),
                Err(e) => eprintln!("ffh-play: autosave failed: {e}"),
            }
            if let Some(p) = self.profile.as_mut() {
                p.autosave += t0.elapsed();
            }
        }
    }

    /// Moves the game log into the message store: the entries C++ would send to the human
    /// ([`LogFog`]).
    fn drain_log(&mut self) {
        let h = self.human;
        let d = self.g.d;
        let entries = std::mem::take(&mut self.g.log.entries);
        // Only untagged entries read the game (under a guard). The text keys ffh-core logs
        // (city names, civilization descriptions, unit names) are resolved for the human.
        let resolve = |mut e: LogEntry| {
            if let std::borrow::Cow::Owned(t) = crate::names::resolve_keys(d, &e.text) {
                e.text = t;
            }
            e
        };
        if self.observing {
            // The observer sees every player's messages (each tagged with its player).
            for e in entries {
                self.messages.push(resolve(e));
            }
            return;
        }
        if entries.iter().any(|e| e.player.is_none()) {
            let g = QueryGuard::new(&self.g);
            for e in entries.into_iter().filter_map(|e| self.fog.filter(&g, h, e)) {
                self.messages.push(resolve(e));
            }
        } else {
            for e in entries.into_iter().filter_map(|e| self.fog.filter(&self.g, h, e)) {
                self.messages.push(resolve(e));
            }
        }
        self.fog.update(&self.g, h);
    }

    /// Shows the queued event popups of `h` at a control point: `launchEventPopup` checks once,
    /// when the popup is shown, that its data exists and some event can be done, else drops it.
    /// A shown popup is modal in C++ with its buttons fixed, so it stays open (its options
    /// enabled as `canDoEvent` says at each moment) until answered or closed by a forced end
    /// turn; it is dropped only when its data is gone (`applyEvent` would do nothing).
    fn launch_events(&mut self, h: PlayerId) {
        let g = QueryGuard::new(&self.g);
        self.shared.pending_events.lock().unwrap().retain_mut(|e| {
            if e.player != h {
                true
            } else if e.launched {
                g.player_get_event_triggered(h, e.triggered).is_some()
            } else {
                e.launched = decision::event_available(&g, h, e.triggered);
                e.launched
            }
        });
    }

    // ---- reads ---------------------------------------------------------------------------------

    /// Reads the game under a `QueryGuard` (in-process readers such as the bot).
    pub fn read<R>(&self, f: impl FnOnce(&Game) -> R) -> R {
        guard::read(&self.g, f)
    }

    /// What waits for the human.
    pub fn pending(&self) -> Pending {
        if self.observing {
            return Pending::default();
        }
        let events = self.shared.open_events_of(self.human);
        self.read(|g| decision::pending(g, self.human, &events))
    }

    /// The human's messages (oldest first; the last `report::MESSAGES_KEPT`).
    pub fn messages(&self) -> &[LogEntry] {
        self.messages.entries()
    }

    /// The message store (indices for the `log` query).
    pub fn message_store(&self) -> &MessageStore {
        &self.messages
    }

    /// The last turn report.
    pub fn report(&self) -> Option<&Report> {
        self.report.as_ref()
    }

    /// Debug reveal: queries answer with the true state (marked `"reveal": true`). Switching it
    /// changes the view: `view_version` is bumped, so a client refetches it.
    pub fn set_reveal(&self, on: bool) {
        let mut ui = UiState::lock(&self.ui);
        if ui.reveal != on {
            ui.reveal = on;
            ui.view_version += 1;
        }
    }

    /// Answers a protocol query (design §4.6) from the human's side, under a `QueryGuard`.
    pub fn query(&self, q: &Query) -> Result<Json, QueryErr> {
        let events = self.shared.open_events_of(self.human);
        let decision = self.shared.open_decision.lock().unwrap().clone();
        let ctx = query::Ctx {
            h: self.human,
            events: &events,
            decision: decision.as_ref(),
            messages: Some(&self.messages),
            report: self.report.as_ref(),
            in_decision: false,
            saves_dir: self.opts.saves_dir.as_deref(),
            ui: &self.ui,
            observe: self.observe_state(),
        };
        query::run(&self.g, &ctx, q)
    }

    /// [`Session::query`] of a request object (`{"q": ..}`).
    pub fn query_json(&self, j: &Json) -> Result<Json, QueryErr> {
        self.query(&Query::from_json(j)?)
    }

    pub fn stats(&self) -> Stats {
        self.shared.stats.lock().unwrap().clone()
    }

    /// The private advisor (`ffh_ai::Ai`), e.g. for suggestions.
    pub fn advisor(&self) -> &Arc<Ai> {
        &self.advisor
    }

    /// The journal's lines when it is kept in memory.
    pub fn journal_lines(&self) -> Option<Vec<String>> {
        self.shared.journal.lock().unwrap().lines().map(<[String]>::to_vec)
    }

    pub fn journal_path(&self) -> Option<PathBuf> {
        self.shared.journal.lock().unwrap().path().map(PathBuf::from)
    }

    pub fn last_autosave(&self) -> Option<&SavePaths> {
        self.last_autosave.as_ref()
    }

    // ---- saves ---------------------------------------------------------------------------------

    fn sidecar(&self) -> Sidecar {
        let journal = self.shared.journal.lock().unwrap();
        let tail = self.messages.tail(50);
        // The snapshot's log mark in the loaded store's numbering (the tail is 0..len).
        let report_snapshot = self.snapshot.as_ref().map(|s| {
            let mut s = s.clone();
            let since = self.messages.total() - s.log_mark;
            s.log_mark = tail.len().saturating_sub(since);
            s.to_json()
        });
        Sidecar {
            human: self.human,
            observe: self.observing,
            to_play: self.to_play,
            setup: self.setup.as_ref().map(|ng| ng.to_json_names(self.g.d)),
            pending_events: self.shared.pending_events.lock().unwrap().clone(),
            journal: journal.path().map(|p| p.display().to_string()),
            journal_line: journal.count(),
            messages_tail: tail,
            auto_moves_tried: self.auto_moves_tried,
            report_snapshot,
            report: self.report.as_ref().map(Report::to_json),
        }
    }

    /// The save and its sidecar, if saving is allowed now: at a control point or a turn
    /// boundary, never while a decision is open.
    pub fn save_bytes(&self) -> Result<(Vec<u8>, Sidecar), PlayError> {
        if self.shared.decision_open() {
            return Err(PlayError::Refused("a decision is open".into()));
        }
        if !matches!(self.phase(), Phase::Human(_) | Phase::Over) && !self.g.at_turn_boundary() {
            return Err(PlayError::Refused("saving is possible at a control point or a turn boundary".into()));
        }
        Ok((self.g.save(), self.sidecar()))
    }

    /// Flags plot `(x, y)` with `note` (`crate::flag`): the record, appended to the game
    /// directory's `flags.jsonl` (with the replay's start save pinned) when there is one. Taken
    /// while the game is stopped (a control point, an observed stop, the game over).
    pub fn flag(&self, x: i32, y: i32, note: &str) -> Result<Json, PlayError> {
        if !matches!(self.phase(), Phase::Human(_) | Phase::Over) || self.shared.decision_open() {
            return Err(PlayError::Refused("flags are taken while the game is stopped".into()));
        }
        let input = crate::flag::FlagInput {
            g: &self.g,
            observed: self.observing,
            x,
            y,
            note,
            setup: self.setup.as_ref(),
            dir: self.opts.saves_dir.as_deref(),
            journal: self.journal_path(),
            messages: self.messages.entries(),
        };
        guard::read(&self.g, |_| crate::flag::record(&input)).map_err(PlayError::Refused)
    }

    /// The game's settings, when known.
    pub fn setup(&self) -> Option<&NewGame> {
        self.setup.as_ref()
    }

    /// Saves `<saves_dir>/<name>.ffhsave` + `.ffhsess`.
    pub fn save(&self, name: &str) -> Result<SavePaths, PlayError> {
        let Some(dir) = &self.opts.saves_dir else { return Err(PlayError::Refused("no saves directory".into())) };
        save::sanitize_name(name).map_err(PlayError::Refused)?;
        let (bytes, sidecar) = self.save_bytes()?;
        save::write_bundle(dir, name, &bytes, &sidecar).map_err(|e| PlayError::Io(e.to_string()))
    }
}

/// Where the rules moved the human to another slot (`Session::follow_human`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum MovedIn {
    /// The human's own command (casting Impersonate Leader, answering the "switch civilizations"
    /// event): the move is the command's last act, so its entries are drained for the old slot.
    Command,
    /// An `update()`, which started when the log had `log_mark` entries: High to Low reassigns
    /// slot 0 as its turn starts, and slot 0 then plays that turn as an AI in the same update;
    /// the Impersonate Leader timer runs out at the end of the impersonated slot's turn, and the
    /// AI turns that follow run in the same update. The entries of earlier slices are drained
    /// for the old slot, the update's for the new slot: the old slot's messages from after the
    /// move reach nobody, as in C++; the old slot's messages logged earlier in the same update
    /// are lost (the start of turn 50 for High to Low, the impersonated civ's end-of-turn
    /// production and research for the timer), and the new slot's from before the move are
    /// shown (C++ keeps them in that player's message list, which is now the human's).
    Update { log_mark: usize },
}

/// A human group is busy (a mission step under way, or combat).
fn human_busy(g: &Game, h: PlayerId) -> bool {
    g.players[h].groups.iter().any(|&grp| g.group_is_busy(grp) || g.group_is_cargo_busy(grp))
}

/// During the human's turn, a group the AI moves (automated, FfH AI-controlled) waits for its
/// next orders: C++ `AI_unitUpdate` gives them in the exe's next frames, while the human plays;
/// the session runs those frames first (`update_automated_group` never leaves a group ready, so
/// this ends).
fn ai_groups_waiting(g: &Game, h: PlayerId) -> bool {
    let pl = &g.players[h];
    pl.human && pl.turn_active && !pl.auto_moves && !pl.end_turn && !decision::ai_groups_waiting(g, h).is_empty()
}

/// The phase predicate (design §2.2): pure, `&self` functions only.
pub fn phase(g: &Game, h: PlayerId) -> Phase {
    if g.game_state == GameStateType::GAMESTATE_OVER {
        return Phase::Over;
    }
    let pl = &g.players[h];
    if pl.alive && pl.human && pl.turn_active && !pl.auto_moves && !pl.end_turn && !human_busy(g, h) && !ai_groups_waiting(g, h) {
        return Phase::Human(h);
    }
    Phase::Running
}

/// The phase of an observed session (viewer `h`, `to_play` turns to play): `Human(h)` while it is
/// stopped, else `Running`; `Over` once the game is decided (a victory leaves an autorun game
/// EXTENDED, not OVER).
pub fn observed_phase(g: &Game, h: PlayerId, to_play: i32) -> Phase {
    if g.game_state != GameStateType::GAMESTATE_ON {
        Phase::Over
    } else if to_play <= 0 {
        Phase::Human(h)
    } else {
        Phase::Running
    }
}

/// Why a turn does not finish: the active players' ready or busy groups (as ffh-cli's run loop).
pub fn diagnose(g: &Game) -> String {
    let mut out = format!("turn {} did not finish after {WATCHDOG_SLICES} slices;", g.game_turn());
    for p in g.player_ids() {
        let pl = &g.players[p];
        if !pl.alive || !pl.turn_active {
            continue;
        }
        out += &format!(" player {} (human {}, auto moves {}, end turn {}):", p.0, pl.human, pl.auto_moves, pl.end_turn);
        for &grp in &pl.groups {
            if g.group_ready_to_move(grp, true) || g.group_is_busy(grp) {
                let head = g.group_get_head_unit(grp);
                out += &format!(
                    " group {} head {:?} ({}) missions {:?} activity {:?} busy {};",
                    grp.0,
                    head,
                    head.map_or("-", |u| g.d.type_name(g.units[u].unit_type)),
                    g.groups[grp].mission_queue.iter().map(|m| m.mission_type).collect::<Vec<_>>(),
                    g.groups[grp].activity_type,
                    g.group_is_busy(grp)
                );
            }
        }
    }
    out
}

// ---- replay -------------------------------------------------------------------------------------

/// Where a replay starts: the save a `load` / `take_over` journal names in its header, or a save
/// (with its sidecar) taken at one of the journal's checkpoints.
pub struct ReplayFrom<'a> {
    pub save: &'a [u8],
    pub sidecar: Option<&'a Sidecar>,
}

/// How the replaying session is built.
enum ReplayStart {
    New(crate::json::Json),
    /// `Session::load` of the `--from` save (with its sidecar).
    Load,
    /// `Session::take_over` of the `--from` AI save.
    TakeOver(PlayerId),
}

/// Replays a journal (design §2.6): from its header (a new game), from the save its `load` or
/// `take_over` header names (by FNV hash), or from a save at one of its checkpoints. Every
/// journaled command is re-applied at the same `(turn, slice)` and every decision answered from
/// the journal; checkpoints are compared. Ends at the control point after the last input (or
/// game over), returning the session; an error names the first divergence.
pub fn replay(lines: Vec<Line>, from: Option<ReplayFrom>, opts: SessionOptions) -> Result<Session, String> {
    replay_to(lines, from, opts, None)
}

/// [`replay`], stopping early at the first stop (a control point, an observed stop) after the
/// moves of game turn `to_turn` (`ffh-play replay --to-turn`, a flag's repro: with
/// `FFH_AI_TRACE` on, the trace ends with that turn).
pub fn replay_to(lines: Vec<Line>, from: Option<ReplayFrom>, opts: SessionOptions, to_turn: Option<i32>) -> Result<Session, String> {
    let d = ffh_data::global();
    let opts = SessionOptions { saves_dir: None, autosave_keep: 0, auto_moves_when_idle: false, journal: false, ..opts };
    let (pos, start) = match (&from, lines.first()) {
        (_, None) => return Err("empty journal".into()),
        (None, Some(Line::New { setup })) => (1, ReplayStart::New(setup.clone())),
        (None, Some(Line::Load { fnv, .. } | Line::TakeOver { fnv, .. })) => {
            return Err(format!("journal starts with a {} line: replay it with --from <the save it names, FNV {fnv}>", lines[0].kind()))
        }
        (None, Some(other)) => return Err(format!("journal starts with a {} line, not a header", other.kind())),
        // The save the journal's session started from.
        (Some(f), Some(Line::Load { fnv, .. })) if *fnv == journal::fnv_hex(f.save) => (1, ReplayStart::Load),
        (Some(f), Some(Line::TakeOver { fnv, human })) if *fnv == journal::fnv_hex(f.save) => (1, ReplayStart::TakeOver(*human)),
        // A save at one of its checkpoints (an autosave or a save at a turn boundary).
        (Some(f), Some(_)) => {
            let g = Game::load(d, f.save).map_err(|e| e.to_string())?;
            let (turn, checksum) = (g.game_turn(), g.calculate_sync_checksum());
            let pos = match f.sidecar.map(|s| s.journal_line) {
                Some(n) if n > 0 && matches!(lines.get(n - 1), Some(Line::Checkpoint { turn: t, checksum: c }) if (*t, *c) == (turn, checksum)) => n,
                _ => lines
                    .iter()
                    .position(|l| matches!(l, Line::Checkpoint { turn: t, checksum: c } if (*t, *c) == (turn, checksum)))
                    .map(|i| i + 1)
                    .ok_or_else(|| format!("the save is neither the one the journal's header names nor at one of its checkpoints (no checkpoint for turn {turn} with checksum {checksum})"))?,
            };
            (pos, ReplayStart::Load)
        }
    };
    let cursor = Arc::new(Mutex::new(Cursor::new(lines, pos)));
    let answerer = Arc::new(JournalAnswerer { cursor: cursor.clone() });
    let jr = Journal::replay(cursor.clone());
    let mut s = match (start, &from) {
        (ReplayStart::New(setup), _) => {
            let ng = NewGame::from_json(d, &setup).map_err(|e| e.to_string())?;
            Session::new_game_with(&ng, answerer, opts, jr).map_err(|e| e.to_string())?
        }
        (ReplayStart::Load, Some(f)) => Session::load_with(f.save, f.sidecar, answerer, opts, jr).map_err(|e| e.to_string())?,
        (ReplayStart::TakeOver(p), Some(f)) => Session::take_over_with(f.save, p, answerer, opts, jr).map_err(|e| e.to_string())?,
        (_, None) => unreachable!("load and take-over starts need a save"),
    };
    let desync = |c: &Arc<Mutex<Cursor>>| c.lock().unwrap().desync.clone();
    loop {
        let stop = s.advance(&mut |_| {});
        if let Some(e) = desync(&cursor) {
            return Err(e);
        }
        if to_turn.is_some_and(|t| s.g.game_turn() > t || (s.g.at_turn_boundary() && s.g.game_turn() >= t)) {
            return Ok(s);
        }
        let (next, pos) = {
            let c = cursor.lock().unwrap();
            (c.peek().cloned(), c.pos)
        };
        match next {
            None => return Ok(s),
            Some(Line::Cmd { stamp, cmd }) => {
                if stop != Stop::Human {
                    return Err(format!("journal line {}: a command, but the replay stopped: {stop:?}", pos + 1));
                }
                let here = Stamp::of(&s.g);
                if stamp != here {
                    return Err(format!(
                        "journal line {}: command stamped turn {} slice {}, replay at turn {} slice {}",
                        pos + 1,
                        stamp.turn,
                        stamp.slice,
                        here.turn,
                        here.slice
                    ));
                }
                cursor.lock().unwrap().pos += 1;
                let c = Command::from_json(d, &cmd).map_err(|e| format!("journal line {}: {e}", pos + 1))?;
                match s.execute(&c) {
                    Ok(CmdOk::Applied { .. }) => {}
                    Ok(CmdOk::NeedsConfirm(c)) => return Err(format!("journal line {}: the command now needs confirmation: {c:?}", pos + 1)),
                    Err(e) => return Err(format!("journal line {}: the command is refused: {e}", pos + 1)),
                }
                if let Some(e) = desync(&cursor) {
                    return Err(e);
                }
            }
            Some(other) => {
                return Err(format!("journal line {}: {} line, but the replay is at a control point ({stop:?})", pos + 1, other.kind()));
            }
        }
    }
}
