//! Panics of a game played in this process: the hook records each panic's message, location and
//! the frames of the workspace crates, and [`catch`] returns them as a [`Crash`].

use std::backtrace::Backtrace;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

/// A caught panic.
#[derive(Clone, Debug)]
pub struct Crash {
    pub message: String,
    /// `file:line` (from `crates/`).
    pub location: String,
    /// Frames of the workspace crates (`fn (file:line)`), innermost first.
    pub frames: Vec<String>,
    /// The game turn reached ([`set_turn`]).
    pub turn: i32,
}

/// The game turn the runner has reached, read when a panic is recorded.
static TURN: AtomicI32 = AtomicI32::new(-1);
static PANIC: Mutex<Option<Crash>> = Mutex::new(None);

/// Most frames kept per crash.
const MAX_FRAMES: usize = 16;

/// Tells the hook which game turn is being played.
pub fn set_turn(turn: i32) {
    TURN.store(turn, Ordering::Relaxed);
}

/// The game turn last set with [`set_turn`].
pub fn turn() -> i32 {
    TURN.load(Ordering::Relaxed)
}

/// A source path without the directories above `crates/` when the compiler recorded it
/// absolute (`crates/ffh-core/src/unit/combat.rs` stays as it is).
fn from_crates(path: &str) -> String {
    path.rsplit_once("/crates/").map_or(path.to_string(), |(_, r)| r.to_string())
}

/// The frames of the workspace crates in `bt` (`fn (file:line)`), down to the first frame whose
/// symbol starts with `stop_at` (the runner's own entry point: the frames below it are its
/// callers).
pub fn ffh_frames(bt: &Backtrace, stop_at: &str) -> Vec<String> {
    let text = bt.to_string();
    let mut frames = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(l) = lines.next() {
        let Some((_, sym)) = l.trim().split_once(": ") else { continue };
        let at = match lines.peek() {
            Some(n) if n.trim().starts_with("at ") => lines.next().unwrap_or_default().trim().trim_start_matches("at ").to_string(),
            _ => String::new(),
        };
        if sym.starts_with(stop_at) {
            break;
        }
        let ours = sym.starts_with("ffh_") || sym.starts_with("<ffh");
        if ours && !sym.starts_with("ffh_harness") && !at.starts_with("/rustc/") {
            let at = from_crates(&at);
            let sym = sym.rsplit_once("::h").filter(|(_, h)| h.len() == 16).map_or(sym, |(s, _)| s);
            frames.push(format!("{sym} ({at})"));
        }
    }
    frames.truncate(MAX_FRAMES);
    frames
}

/// Installs the hook that records every panic (and prints a one-line note on stderr).
/// `stop_at`: see [`ffh_frames`].
pub fn install_hook(stop_at: &'static str) {
    panic::set_hook(Box::new(move |info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic>".to_string()
        };
        let location = info.location().map_or("?".to_string(), |l| {
            format!("{}:{}", from_crates(l.file()), l.line())
        });
        let turn = turn();
        eprintln!("panic at turn {turn}: {message} at {location}");
        let frames = ffh_frames(&Backtrace::force_capture(), stop_at);
        *PANIC.lock().unwrap_or_else(|e| e.into_inner()) = Some(Crash { message, location, frames, turn });
    }));
}

/// Runs `f` with the hook installed (see [`install_hook`]); a panic comes back as its [`Crash`].
pub fn catch<R>(stop_at: &'static str, f: impl FnOnce() -> R) -> Result<R, Crash> {
    install_hook(stop_at);
    set_turn(-1);
    panic::catch_unwind(AssertUnwindSafe(f)).map_err(|_| {
        PANIC.lock().unwrap_or_else(|e| e.into_inner()).take().unwrap_or(Crash {
            message: "?".into(),
            location: "?".into(),
            frames: Vec::new(),
            turn: turn(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_caught_panic_has_its_message_location_and_turn() {
        let crash = catch("ffh_harness::panic::tests", || {
            set_turn(7);
            panic!("boom {}", 1 + 1)
        })
        .unwrap_err();
        assert_eq!(crash.message, "boom 2");
        assert!(crash.location.contains("ffh-harness/src/panic.rs:"), "{}", crash.location);
        assert_eq!(crash.turn, 7);
        assert_eq!(catch("x", || 5).unwrap(), 5);
    }
}
