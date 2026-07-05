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

/// The WASI preview-1 host: the [`BrowserHost`] stubs (clock) plus real
/// `stdout`/`stderr` via WASI `fd_write`, a writable [`WasiEnv`], and — under the
/// `wasm_stdlib` feature — an in-memory [`MemFs`](crate::mem_fs::MemFs) seeded
/// with the embedded Tcl 9 standard library. With the stdlib mounted, the
/// filesystem-backed startup path (`init_library()` → `source init.tcl` →
/// `package require tcltest`) runs on a target with no host filesystem.
#[cfg(target_os = "wasi")]
pub struct WasiHost {
    clock: BrowserClock,
    stdio: WasiStdIo,
    env: WasiEnv,
    #[cfg(feature = "wasm_stdlib")]
    fs: crate::mem_fs::MemFs,
}

#[cfg(target_os = "wasi")]
impl WasiHost {
    /// Create the host. With `wasm_stdlib`, seed the VFS with the embedded
    /// standard library and report its mount point as `$TCL_LIBRARY`.
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "wasm_stdlib")]
        let fs = {
            let fs = crate::mem_fs::MemFs::new();
            crate::embedded_stdlib::seed(&fs);
            fs
        };
        Self {
            clock: BrowserClock,
            stdio: WasiStdIo,
            env: WasiEnv::new(),
            #[cfg(feature = "wasm_stdlib")]
            fs,
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
        // stdio reaches WASI; the VFS provides filesystem under `wasm_stdlib`.
        // Sockets/process stay absent under preview 1.
        #[cfg(feature = "wasm_stdlib")]
        {
            Capabilities::FILESYSTEM
        }
        #[cfg(not(feature = "wasm_stdlib"))]
        {
            Capabilities::empty()
        }
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
    #[cfg(feature = "wasm_stdlib")]
    fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
        Some(&self.fs)
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

/// The WASI environment: a writable variable map plus a tracked working
/// directory. Seeded with `TCL_LIBRARY` (the embedded-stdlib mount) under the
/// `wasm_stdlib` feature so `init_library()` locates the library on a host with
/// no real environment.
#[cfg(target_os = "wasi")]
struct WasiEnv {
    vars: core::cell::RefCell<std::collections::BTreeMap<String, String>>,
    cwd: core::cell::RefCell<String>,
}

#[cfg(target_os = "wasi")]
impl WasiEnv {
    fn new() -> Self {
        #[allow(unused_mut)]
        let mut vars = std::collections::BTreeMap::new();
        #[cfg(feature = "wasm_stdlib")]
        vars.insert(
            "TCL_LIBRARY".to_string(),
            crate::embedded_stdlib::TCL_LIBRARY_MOUNT.to_string(),
        );
        Self {
            vars: core::cell::RefCell::new(vars),
            cwd: core::cell::RefCell::new("/".to_string()),
        }
    }
}

#[cfg(target_os = "wasi")]
impl Env for WasiEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.vars.borrow().get(key).cloned()
    }
    fn set(&self, key: &str, val: &str) {
        self.vars
            .borrow_mut()
            .insert(key.to_string(), val.to_string());
    }
    fn vars(&self) -> Vec<(String, String)> {
        self.vars
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
    fn cwd(&self) -> Result<String, HostError> {
        Ok(self.cwd.borrow().clone())
    }
    fn chdir(&self, path: &str) -> Result<(), HostError> {
        *self.cwd.borrow_mut() = path.to_string();
        Ok(())
    }
}
