//! The WASM capability hosts.
//!
//! Two flavours, selected by target in [`crate::interp`]:
//!
//! - **[`BrowserHost`]** (`wasm32-unknown-unknown`, no WASI, no JS imports yet):
//!   the mandatory capabilities ([`Clock`]/[`StdIo`]/[`Env`]) are stubbed and the
//!   optional ones ([`filesystem`](Host::filesystem)/sockets/process) are absent.
//!   Its job is to make `runtime/rust` **build and link** for
//!   `wasm32-unknown-unknown` — the prerequisite for a real browser host (JS
//!   imports for the console + wall clock, an in-memory VFS) that plugs into the
//!   same [`Host`] trait later, with nothing else in the runtime changing.
//!
//! - **[`WasiHost`]** (`wasm32-wasip1`, `target_os = "wasi"`): identical except
//!   `stdout`/`stderr` reach the real WASI `fd_write` (via `std::io`), so `puts`
//!   is **visible** under `wasmtime` / any WASI host — what the AOT-script path
//!   runs against. Clock + env stay stubbed (the AOT path needs neither yet; a
//!   fuller WASI host can wire `std::time`/`std::env`/preopens later).

use tcl_platform::{Capabilities, Clock, Env, Host, HostError, StdIo};

/// The placeholder browser host (see the module docs).
pub struct BrowserHost {
    clock: BrowserClock,
    stdio: BrowserStdIo,
    env: BrowserEnv,
}

impl BrowserHost {
    /// Create the host.
    #[must_use]
    pub fn new() -> Self {
        Self {
            clock: BrowserClock,
            stdio: BrowserStdIo,
            env: BrowserEnv,
        }
    }
}

impl Default for BrowserHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host for BrowserHost {
    fn capabilities(&self) -> Capabilities {
        // No filesystem, sockets, process, or threads in this placeholder.
        Capabilities::empty()
    }

    fn clock(&self) -> &dyn Clock {
        &self.clock
    }

    fn stdio(&self) -> &dyn StdIo {
        &self.stdio
    }

    fn env(&self) -> &dyn Env {
        &self.env
    }
    // filesystem()/sockets()/process() keep the trait's `None` defaults.
}

/// No wall clock without a host import — reports the epoch.
struct BrowserClock;

impl Clock for BrowserClock {
    fn now_secs(&self) -> i64 {
        0
    }
    fn now_millis(&self) -> i128 {
        0
    }
}

/// Standard output/error are discarded until a console host import is wired.
struct BrowserStdIo;

impl StdIo for BrowserStdIo {
    fn write_stdout(&self, _bytes: &[u8]) {}
    fn write_stderr(&self, _bytes: &[u8]) {}
}

/// The WASI preview-1 host: the [`BrowserHost`] stubs (clock/env) plus real
/// `stdout`/`stderr` via WASI `fd_write`.
#[cfg(target_os = "wasi")]
pub struct WasiHost {
    clock: BrowserClock,
    stdio: WasiStdIo,
    env: BrowserEnv,
}

#[cfg(target_os = "wasi")]
impl WasiHost {
    /// Create the host.
    #[must_use]
    pub fn new() -> Self {
        Self {
            clock: BrowserClock,
            stdio: WasiStdIo,
            env: BrowserEnv,
        }
    }
}

#[cfg(target_os = "wasi")]
impl Default for WasiHost {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "wasi")]
impl Host for WasiHost {
    fn capabilities(&self) -> Capabilities {
        // stdio reaches WASI; fs/sockets/process stay absent under preview 1.
        Capabilities::empty()
    }
    fn clock(&self) -> &dyn Clock {
        &self.clock
    }
    fn stdio(&self) -> &dyn StdIo {
        &self.stdio
    }
    fn env(&self) -> &dyn Env {
        &self.env
    }
}

/// `stdout`/`stderr` to the real WASI `fd_write` (Rust `std` maps stdio to it on
/// `wasm32-wasip1`). Each write is flushed so output is visible immediately
/// under `wasmtime`, whose pipe is block-buffered (`puts` callers append the
/// newline before handing the buffer here).
#[cfg(target_os = "wasi")]
struct WasiStdIo;

#[cfg(target_os = "wasi")]
impl StdIo for WasiStdIo {
    fn write_stdout(&self, bytes: &[u8]) {
        use std::io::Write;
        let mut out = std::io::stdout();
        let _ = out.write_all(bytes);
        let _ = out.flush();
    }
    fn write_stderr(&self, bytes: &[u8]) {
        use std::io::Write;
        let mut err = std::io::stderr();
        let _ = err.write_all(bytes);
        let _ = err.flush();
    }
    fn flush_stdout(&self) {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    fn flush_stderr(&self) {
        use std::io::Write;
        let _ = std::io::stderr().flush();
    }
}

/// No environment and a fixed root working directory.
struct BrowserEnv;

impl Env for BrowserEnv {
    fn get(&self, _key: &str) -> Option<String> {
        None
    }
    fn set(&self, _key: &str, _val: &str) {}
    fn vars(&self) -> Vec<(String, String)> {
        Vec::new()
    }
    fn cwd(&self) -> Result<String, HostError> {
        Ok("/".to_string())
    }
    fn chdir(&self, _path: &str) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
}
