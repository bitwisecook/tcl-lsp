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

//! The WASI stdio transport for the Tcl language server.
//!
//! `tcl_lsp_server::Backend` wrapped in `LspService` is the entire protocol
//! core, and it is transport-free: the native binary drives it over real stdio,
//! `tcl-lsp-server-wasm` drives it over `postMessage`, and this binary drives
//! it over Content-Length-framed stdio inside a wasm32-wasip1 sandbox. Nothing
//! here decides anything about Tcl — every handler, diagnostic, and provider is
//! the same code the native binary runs, including the two protocol shims in
//! [`tcl_lsp_server::service`], applied for the same reasons `main.rs` applies
//! them.
//!
//! # The protocol on the wire
//!
//! Standard LSP base protocol: `Content-Length: N\r\n\r\n` followed by N bytes
//! of JSON-RPC, on stdin and stdout. Not the browser transport's bare
//! `postMessage` framing — a byte stream has to delimit its messages, and every
//! stdio client, `@vscode/wasm-wasi-lsp` included, expects exactly this.
//! Diagnostics go to stderr, which is the only channel left over.
//!
//! # Files
//!
//! [`vfs::NativeStore`], not the browser's `MemoryStore`. WASI preopens make
//! `std::fs` real: a directory the host grants with `--dir` (wasmtime) or
//! `MapDir` (`@vscode/wasm-wasi`) is a directory the server can walk, so the
//! whole-workspace paths — folder scan, `source` resolution, package database,
//! spec-pack discovery — work as they do natively, within the sandbox. A path
//! outside every preopen is simply `NotFound`, which is the store's documented
//! contract for a missing file.
//!
//! # What the host owes this binary
//!
//! 1. **Preopen the workspace.** Everything outside a preopened directory does
//!    not exist as far as the server is concerned.
//! 2. **Drain stdout.** There is one thread; a host that stops reading stdout
//!    eventually blocks the server inside `write`. The native transport solves
//!    this with a second task (`stdio_pump`) precisely because it *can*; wasip1
//!    cannot, so the obligation moves to the host.
//! 3. **Expect a monotonic clock.** The driver's wait and the server's
//!    deadlines both need `poll_oneoff`'s `CLOCKID_MONOTONIC`.
//!
//! # What a panic means here
//!
//! wasip1 cannot unwind, so `panic = "abort"`: a panic anywhere in the server
//! graph kills the session outright. There is no `catch_unwind` recovery to add
//! — the process is already gone by the time one could run — and the same is
//! true of the salsa cancellation the native server relies on. See the caveats
//! on [`tcl_lsp_server::rt`]'s wasi arm; they are the reason the analysis
//! offload runs inline rather than concurrently.

#![deny(unsafe_op_in_unsafe_fn)]

use std::sync::Arc;

use tcl_lsp_server::{Backend, vfs};
use tower_lsp_server::LspService;

mod driver;
mod framing;
mod wasi_poll;

/// The analysis stack's stack budget — the wasip1 twin of `main.rs`'s
/// `WORKER_STACK_SIZE` and of `build-wasm.sh`'s `STACK_SIZE`, and load-bearing
/// for the same reason (issue #996).
///
/// The analyser's `analyse_body` recursion and the CFG builder's `lower_script`
/// recursion each cap their nesting depth, but a cap on the *number* of frames
/// says nothing about how much stack those frames need. The native server gives
/// its Tokio workers 64 MiB because 2 MiB overflows around nesting depth
/// 130-140, well inside the cap. rust-lld's default wasm stack is 1 MiB —
/// smaller still — and a wasm stack overflow is not a clean panic: it silently
/// corrupts the shadow stack or traps with `unreachable`, so the failure mode
/// is worse than native's. Match the native budget.
///
/// This constant is documentation; the value is applied at link time by
/// `build-wasi.sh`'s `-C link-arg=-zstack-size=…`, because a linker argument is
/// not something a source file can set. Keep the two in step.
#[allow(dead_code)]
const STACK_SIZE: usize = 64 * 1024 * 1024;

fn main() {
    // A current-thread runtime with the timer wheel, and nothing else. wasip1
    // has no thread-spawn syscall, so `new_multi_thread` builds and then fails
    // at run time with `os error 58`; `enable_io` would want a reactor there is
    // no driver for. This is the runtime shape `tcl_lsp_server::rt`'s wasi arm
    // documents as its host contract, and `crate::driver` is written to the
    // other half of it — never letting the runtime park with nothing pending.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("a current-thread Tokio runtime should build on wasip1");

    let stop = runtime.block_on(async {
        // WASI preopens make `std::fs` real, so the server reads closed files
        // from the host filesystem exactly as the native binary does.
        let store: Arc<dyn vfs::SourceStore> = Arc::new(vfs::NativeStore);
        let (service, socket) = LspService::new(move |client| Backend::with_store(client, store));
        driver::Driver::new(service, socket).run().await
    });

    // Drop the runtime before exiting so nothing is mid-write, then report the
    // protocol's exit status: 0 when `exit` followed `shutdown`, 1 when it did
    // not.
    drop(runtime);
    std::process::exit(stop.exit_code());
}
