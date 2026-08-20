// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! [`BrowserHost`] — the capability [`Host`] for `wasm32-unknown-unknown`.
//!
//! The other half of the seam [`host_native`](crate::host_native) owns, and the
//! [`Vm`](crate::Vm)'s default host wherever there is no operating
//! system underneath: a browser tab, a Web Worker, a bare `WebAssembly`
//! instantiation under node. `runtime/rust` has had this split since it learnt
//! to link for wasm (`runtime/rust/src/host_wasm.rs`); this is the bytecode
//! VM's copy of the same decision, and it exists because the VM *runs* on that
//! target now — the Tcl language server compiled to wasm evaluates `SpecTcl`
//! pack hook bodies through it (`tcl_spectcl::hooks::ensure_thread_host`).
//!
//! Reaching [`NativeHost`](tcl_host_native::NativeHost) here is not a
//! degradation, it is a **crash**: `std::time::SystemTime::now` on
//! `wasm32-unknown-unknown` is `unsupported.rs`'s `panic!("time not implemented
//! on this platform")`, and a wasm panic aborts (the module traps on
//! `unreachable`) rather than unwinding, so every lock and refcount the aborted
//! call held stays held. Issue #1661 is exactly that: the first bundled
//! `.tclspec` to declare hook bodies (`specs/upf.tclspec`) made the language
//! server build a hook host in the browser, `Vm::set_wall_clock_budget` read
//! the native clock to arm its 250 ms budget, and the trap that followed
//! stranded a salsa database handle — so the next input mutation blocked in
//! `Storage::cancel_others` and panicked a second time inside parking_lot's
//! `wasm` thread parker.
//!
//! ## What each capability does here
//!
//! - **Clock** — real under the default `js-clock` feature, and the reason this
//!   module is not a stub: the browser has a wall clock, so `BrowserClock`
//!   reads JavaScript's `Date.now()`. Millisecond resolution (what `Date.now()`
//!   reports, and what browsers clamp to anyway), so `now_micros` keeps the
//!   trait's `now_millis() * 1000` default rather than pretending to a
//!   precision it does not have. With `js-clock` off — the import-free build
//!   `tcl-vm-wasm` needs — it reports the epoch, which is a wrong answer rather
//!   than a crash, and the posture `runtime/rust`'s `BrowserHost` has always
//!   had.
//! - **`StdIo`** — discarded. There is no terminal on the other end of a
//!   worker's `puts`; a host that wants the output owns the decision of where
//!   it goes and can install its own [`Host`] with [`Vm::set_host`].
//! - **Env** — empty, with `/` as the working directory. `std::env::vars` is
//!   another `unsupported.rs` panic on this target, which is why the VM's
//!   `write_platform_globals` already skips it here.
//! - **Filesystem / process / sockets** — absent, so `tcl-cmd-core`'s portable
//!   bodies take their documented "the platform cannot do this" path instead of
//!   failing at a syscall.
//!
//! [`Vm::set_host`]: crate::Vm::set_host

use tcl_platform::{Capabilities, Clock, Env, Host, HostError, StdIo};

/// The browser/bare-wasm host: a real JS wall clock, and nothing else.
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
        // No filesystem, sockets, or subprocess in a browser tab.
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

/// The wall clock.
///
/// Under the default `js-clock` feature this is JavaScript's `Date.now()` —
/// milliseconds since the Unix epoch, as an `f64`. `Date.now()` is on the
/// global object in a window, a worker, and node alike, so it needs no host
/// import beyond `js-sys` and works in every context the language server's wasm
/// build is loaded into.
///
/// With `js-clock` off there is no JavaScript to ask (that build has no imports
/// at all, by design), so it reports the epoch.
///
/// `now_micros` keeps the trait default (`now_millis() * 1000`) either way:
/// `Date.now()` has no sub-millisecond resolution to report. So does
/// `local_offset_secs` (0 = UTC), matching the std host — the browser knows its
/// timezone, but wiring that is a behaviour change for `clock format`, not a
/// crash fix.
struct BrowserClock;

#[cfg(feature = "js-clock")]
impl Clock for BrowserClock {
    fn now_secs(&self) -> i64 {
        // `Date.now()` is an integral number of milliseconds; dividing before
        // the cast keeps the seconds exact for every representable instant.
        (js_sys::Date::now() / 1000.0) as i64
    }

    fn now_millis(&self) -> i128 {
        js_sys::Date::now() as i128
    }
}

#[cfg(not(feature = "js-clock"))]
impl Clock for BrowserClock {
    fn now_secs(&self) -> i64 {
        0
    }

    fn now_millis(&self) -> i128 {
        0
    }
}

/// `puts` output is discarded: a worker has no terminal, and an embedder that
/// wants it installs its own host.
struct BrowserStdIo;

impl StdIo for BrowserStdIo {
    fn write_stdout(&self, _bytes: &[u8]) {}
    fn write_stderr(&self, _bytes: &[u8]) {}
}

/// No environment variables, and a fixed root working directory.
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
        Ok("/".to_owned())
    }

    fn chdir(&self, _path: &str) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
}
