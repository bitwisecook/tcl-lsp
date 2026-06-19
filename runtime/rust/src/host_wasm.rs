//! A minimal `wasm32-unknown-unknown` host (no WASI, no JS imports yet): the
//! mandatory capabilities ([`Clock`]/[`StdIo`]/[`Env`]) are stubbed and the
//! optional ones ([`filesystem`](Host::filesystem)/sockets/process) are absent.
//!
//! Its job is to make `runtime/rust` **build and link** for
//! `wasm32-unknown-unknown` — the prerequisite for running emitted modules and
//! for a real browser host. That real host (JS imports for the console + wall
//! clock, an in-memory VFS) plugs into this same [`Host`] trait later; nothing
//! else in the runtime needs to change when it does.

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
