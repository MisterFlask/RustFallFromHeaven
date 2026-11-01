//! One child process with a timeout: a game played in its own process, so aborts, stack
//! overflows and hangs are caught as well as panics.

use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// What a child left.
#[derive(Clone, Debug)]
pub struct ChildRun {
    pub stdout: String,
    pub stderr: String,
    /// `None` when waiting for it failed.
    pub status: Option<ExitStatus>,
    /// Killed after the timeout.
    pub timed_out: bool,
    pub secs: f64,
}

impl ChildRun {
    /// Exited by itself with status 0.
    pub fn success(&self) -> bool {
        !self.timed_out && self.status.is_some_and(|s| s.success())
    }

    /// The `KEY\tvalue` lines of its stdout, in order.
    pub fn lines(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.stdout.lines().map(|l| l.split_once('\t').unwrap_or((l, "")))
    }

    /// The exit status and the last lines of stderr, for a child that died.
    pub fn crash_detail(&self) -> String {
        let tail: Vec<&str> = self.stderr.lines().rev().take(6).collect();
        let status = self.status.map_or("?".to_string(), |s| s.to_string());
        format!("{status}: {}", tail.into_iter().rev().collect::<Vec<_>>().join(" | "))
    }
}

/// Runs `cmd` (stdout and stderr piped, `RUST_BACKTRACE=0`) and kills it after `timeout`.
pub fn run(cmd: &mut Command, timeout: Duration) -> std::io::Result<ChildRun> {
    let t = Instant::now();
    let mut child = cmd.env("RUST_BACKTRACE", "0").stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let out_thread = std::thread::spawn(move || {
        let mut v = String::new();
        let _ = stdout.read_to_string(&mut v);
        v
    });
    let err_thread = std::thread::spawn(move || {
        let mut v = String::new();
        let _ = stderr.read_to_string(&mut v);
        v
    });
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Some(st),
            Ok(None) => {}
            Err(_) => break None,
        }
        if t.elapsed() > timeout {
            let _ = child.kill();
            timed_out = true;
            break child.wait().ok();
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    Ok(ChildRun {
        stdout: out_thread.join().unwrap_or_default(),
        stderr: err_thread.join().unwrap_or_default(),
        status,
        timed_out,
        secs: t.elapsed().as_secs_f64(),
    })
}
