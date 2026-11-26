//! Minimal host platform abstraction for native targets and bare-metal `wasm32-unknown-unknown`.
//!
//! On native targets every operation delegates directly to `std::fs`, `std::time`, and `eprintln!`.
//! On `wasm32` (`crates/ffh-web`, zero WASI) it calls the `"ffh"` host imports backed by the
//! worker's in-memory file system (`crates/ffh-web/site/memfs.js`) and browser clocks
//! (`performance.now` / `Date.now`).

use std::io;
use std::ops::{Add, Sub};
use std::path::{Path, PathBuf};
use std::time::Duration;

// ---------------------------------------------------------------------------------------------
// Monotonic & wall-clock time
// ---------------------------------------------------------------------------------------------

/// A monotonic timestamp: `std::time::Instant` natively, `performance.now()` (ms) on `wasm32`.
#[derive(Copy, Clone, Debug)]
pub struct Monotonic {
    #[cfg(not(target_arch = "wasm32"))]
    inner: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    ms: f64,
}

impl PartialEq for Monotonic {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner == other.inner
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.ms == other.ms
        }
    }
}

impl PartialOrd for Monotonic {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.partial_cmp(&other.inner)
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.ms.partial_cmp(&other.ms)
        }
    }
}

impl Monotonic {
    #[inline]
    pub fn now() -> Monotonic {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Monotonic { inner: std::time::Instant::now() }
        }
        #[cfg(target_arch = "wasm32")]
        {
            // SAFETY: pure host clock query.
            Monotonic { ms: unsafe { wasm::now_ms() } }
        }
    }

    #[inline]
    pub fn elapsed(&self) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.elapsed()
        }
        #[cfg(target_arch = "wasm32")]
        {
            Monotonic::now().saturating_duration_since(*self)
        }
    }

    #[inline]
    pub fn saturating_duration_since(&self, earlier: Monotonic) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.saturating_duration_since(earlier.inner)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let diff_ms = self.ms - earlier.ms;
            if diff_ms.is_finite() && diff_ms > 0.0 {
                Duration::from_secs_f64(diff_ms / 1000.0)
            } else {
                Duration::ZERO
            }
        }
    }
}

impl Sub<Monotonic> for Monotonic {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: Monotonic) -> Duration {
        self.saturating_duration_since(rhs)
    }
}

impl Add<Duration> for Monotonic {
    type Output = Monotonic;

    #[inline]
    fn add(self, rhs: Duration) -> Monotonic {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Monotonic { inner: self.inner + rhs }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Monotonic { ms: self.ms + rhs.as_secs_f64() * 1000.0 }
        }
    }
}

/// Wall-clock nanoseconds since the Unix epoch (0 if unavailable). Presentation / epoch ids only,
/// never simulation rules.
pub fn unix_time_nanos() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64)
    }
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: pure host clock query (`Date.now()`).
        let ms = unsafe { wasm::unix_ms() };
        if ms.is_finite() && ms > 0.0 {
            (ms * 1_000_000.0) as u64
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Diagnostic logging
// ---------------------------------------------------------------------------------------------

/// Logs an informational message (`println!` natively, worker console on `wasm32`).
pub fn log(msg: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("{msg}");
    }
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: the host copies the UTF-8 bytes during the call.
        unsafe { wasm::log(0, msg.as_ptr(), msg.len() as u32) }
    }
}

/// Logs a warning/error message (`eprintln!` natively, worker console warn on `wasm32`).
pub fn warn(msg: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        eprintln!("{msg}");
    }
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: the host copies the UTF-8 bytes during the call.
        unsafe { wasm::log(1, msg.as_ptr(), msg.len() as u32) }
    }
}

// ---------------------------------------------------------------------------------------------
// File system
// ---------------------------------------------------------------------------------------------

pub mod fs {
    use super::*;

    pub fn exists(path: &Path) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            path.exists()
        }
        #[cfg(target_arch = "wasm32")]
        {
            stat_kind(path) != 0
        }
    }

    pub fn is_file(path: &Path) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            path.is_file()
        }
        #[cfg(target_arch = "wasm32")]
        {
            stat_kind(path) == 1
        }
    }

    pub fn is_dir(path: &Path) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            path.is_dir()
        }
        #[cfg(target_arch = "wasm32")]
        {
            stat_kind(path) == 2
        }
    }

    /// Modification time as seconds since the Unix epoch, or `None` when absent/unavailable.
    pub fn modified_secs(path: &Path) -> Option<i64> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::metadata(path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let p = path.to_str()?;
            // SAFETY: valid slice during the call.
            let secs = unsafe { wasm::fs_mtime_secs(p.as_ptr(), p.len() as u32) };
            (secs.is_finite() && secs >= 0.0).then_some(secs as i64)
        }
    }

    pub fn read(path: &Path) -> io::Result<Vec<u8>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::read(path)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let p = path_str(path)?;
            // SAFETY: valid slice during the call; host stages `n` bytes if `n >= 0`.
            let n = unsafe { wasm::fs_read(p.as_ptr(), p.len() as u32) };
            if n < 0 {
                return Err(wasm::err(n));
            }
            let mut buf = vec![0u8; n as usize];
            if n > 0 {
                // SAFETY: `buf` has exact length `n` announced by `fs_read`.
                unsafe { wasm::fs_take(buf.as_mut_ptr()) };
            }
            Ok(buf)
        }
    }

    pub fn read_to_string(path: &Path) -> io::Result<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::read_to_string(path)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let bytes = read(path)?;
            String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
    }

    pub fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::write(path, bytes)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let p = path_str(path)?;
            // SAFETY: valid slices during the call.
            let rc = unsafe { wasm::fs_write(p.as_ptr(), p.len() as u32, bytes.as_ptr(), bytes.len() as u32, 0) };
            wasm::check(rc)
        }
    }

    pub fn append(path: &Path, bytes: &[u8]) -> io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
            f.write_all(bytes)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let p = path_str(path)?;
            // SAFETY: valid slices during the call.
            let rc = unsafe { wasm::fs_write(p.as_ptr(), p.len() as u32, bytes.as_ptr(), bytes.len() as u32, 1) };
            wasm::check(rc)
        }
    }

    pub fn copy(from: &Path, to: &Path) -> io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::copy(from, to).map(|_| ())
        }
        #[cfg(target_arch = "wasm32")]
        {
            let bytes = read(from)?;
            write(to, &bytes)
        }
    }

    pub fn create_dir(path: &Path) -> io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::create_dir(path)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let p = path_str(path)?;
            // SAFETY: valid slice during the call.
            let rc = unsafe { wasm::fs_mkdir(p.as_ptr(), p.len() as u32, 0) };
            wasm::check(rc)
        }
    }

    pub fn create_dir_all(path: &Path) -> io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::create_dir_all(path)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let p = path_str(path)?;
            // SAFETY: valid slice during the call.
            let rc = unsafe { wasm::fs_mkdir(p.as_ptr(), p.len() as u32, 1) };
            wasm::check(rc)
        }
    }

    pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::rename(from, to)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let p1 = path_str(from)?;
            let p2 = path_str(to)?;
            // SAFETY: valid slices during the call.
            let rc = unsafe { wasm::fs_rename(p1.as_ptr(), p1.len() as u32, p2.as_ptr(), p2.len() as u32) };
            wasm::check(rc)
        }
    }

    pub fn remove_file(path: &Path) -> io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::remove_file(path)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let p = path_str(path)?;
            // SAFETY: valid slice during the call.
            let rc = unsafe { wasm::fs_remove(p.as_ptr(), p.len() as u32, 0) };
            wasm::check(rc)
        }
    }

    pub fn remove_dir_all(path: &Path) -> io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::remove_dir_all(path)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let p = path_str(path)?;
            // SAFETY: valid slice during the call.
            let rc = unsafe { wasm::fs_remove(p.as_ptr(), p.len() as u32, 1) };
            wasm::check(rc)
        }
    }

    /// Names of entries directly inside directory `path`.
    pub fn read_dir_names(path: &Path) -> io::Result<Vec<String>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut out = Vec::new();
            for entry in std::fs::read_dir(path)? {
                if let Ok(e) = entry {
                    if let Some(name) = e.file_name().to_str() {
                        out.push(name.to_string());
                    }
                }
            }
            Ok(out)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let p = path_str(path)?;
            // SAFETY: valid slice during the call; host stages NUL-separated UTF-8 names if `n > 0`.
            let n = unsafe { wasm::fs_list(p.as_ptr(), p.len() as u32) };
            if n < 0 {
                return Err(wasm::err(n));
            }
            if n == 0 {
                return Ok(Vec::new());
            }
            let mut buf = vec![0u8; n as usize];
            // SAFETY: `buf` has exact length `n` announced by `fs_list`.
            unsafe { wasm::fs_take(buf.as_mut_ptr()) };
            let text = String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Ok(text.split('\0').map(str::to_string).collect())
        }
    }

    /// Full paths (`path.join(name)`) of entries directly inside directory `path`.
    pub fn read_dir_paths(path: &Path) -> io::Result<Vec<PathBuf>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(std::fs::read_dir(path)?.filter_map(|e| e.ok().map(|e| e.path())).collect())
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(read_dir_names(path)?.into_iter().map(|n| path.join(n)).collect())
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn path_str(path: &Path) -> io::Result<&str> {
        path.to_str().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non-UTF-8 path"))
    }

    #[cfg(target_arch = "wasm32")]
    fn stat_kind(path: &Path) -> i32 {
        let Some(p) = path.to_str() else { return 0 };
        // SAFETY: valid slice during the call.
        unsafe { wasm::fs_stat(p.as_ptr(), p.len() as u32) }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::io;

    #[link(wasm_import_module = "ffh")]
    extern "C" {
        pub fn now_ms() -> f64;
        pub fn unix_ms() -> f64;
        pub fn log(level: u32, ptr: *const u8, len: u32);
        pub fn fs_stat(p: *const u8, pl: u32) -> i32;
        pub fn fs_mtime_secs(p: *const u8, pl: u32) -> f64;
        pub fn fs_read(p: *const u8, pl: u32) -> i32;
        pub fn fs_list(p: *const u8, pl: u32) -> i32;
        pub fn fs_take(dst: *mut u8);
        pub fn fs_write(p: *const u8, pl: u32, d: *const u8, dl: u32, append: u32) -> i32;
        pub fn fs_mkdir(p: *const u8, pl: u32, recursive: u32) -> i32;
        pub fn fs_rename(p1: *const u8, pl1: u32, p2: *const u8, pl2: u32) -> i32;
        pub fn fs_remove(p: *const u8, pl: u32, recursive: u32) -> i32;
    }

    pub fn err(code: i32) -> io::Error {
        let (kind, msg) = match code {
            -1 => (io::ErrorKind::NotFound, "not found"),
            -2 => (io::ErrorKind::AlreadyExists, "already exists"),
            -3 => (io::ErrorKind::InvalidInput, "invalid path or file type"),
            -4 => (io::ErrorKind::Other, "directory not empty"),
            _ => (io::ErrorKind::Other, "host I/O error"),
        };
        io::Error::new(kind, msg)
    }

    pub fn check(rc: i32) -> io::Result<()> {
        if rc < 0 {
            Err(err(rc))
        } else {
            Ok(())
        }
    }
}
