//! The engine of the static site (docs/WEB.md): `ffh-play`'s web host
//! ([`ffh_play::transport::web`]) compiled to bare-metal WebAssembly (`wasm32-unknown-unknown`,
//! zero external crates, zero WASI) for a Web Worker, with no server: the page
//! (`crates/ffh-web/site`) routes the client's `/api` requests to it.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p ffh-web` (`tools/web-site.sh`
//! builds the whole site). Natively the crate is empty: everything is `cfg(target_arch = "wasm32")`.
//!
//! Exports: `ffh_alloc(len) -> ptr`, `ffh_free(ptr, len)`, `ffh_init(ptr, len) -> 0 | 1`,
//! `ffh_request(ptr, len)`. Imports (all in module `ffh`, zero WASI imports): engine protocol
//! (`reply`, `publish`, `pull`, `pull_copy`, `crash`), clocks/log (`now_ms`, `unix_ms`, `log`),
//! and in-memory filesystem (`fs_stat`, `fs_mtime_secs`, `fs_read`, `fs_list`, `fs_take`,
//! `fs_write`, `fs_mkdir`, `fs_rename`, `fs_remove`). Buffers: the page owns what it allocates
//! with `ffh_alloc` (it frees them with `ffh_free` once the call returns); the engine owns what
//! it passes to an import (valid during that call only: copy it).

#[cfg(target_arch = "wasm32")]
mod wasm;
