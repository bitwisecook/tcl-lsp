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

//! BPF-Tcl CLI binary.
#![forbid(unsafe_code)]

use std::process::ExitCode;

/// Stack budget for the dedicated worker thread the CLI runs on.
///
/// Matches `WORKER_STACK_SIZE` in `f5-cli`/`tcl-lsp-server`/`tcl-mcp`/
/// `tcl-cli`'s entry points — see `f5-cli/src/lib.rs`'s doc comment for the
/// full rationale. Short version: `check`/`compile`/`run` all call into
/// `tcl_compiler::lowering::lower_to_ir`/`tcl_syntax::expr::parse_expr` on
/// caller-supplied `.bpftcl` source — the same depth-capped-but-stack-hungry
/// recursion chain that crashed `tcl-lsp-server` in issue #996. The
/// OS-provided main-thread stack this binary would otherwise inherit is
/// outside this crate's control (8 MiB by default on Linux, far less
/// guaranteed elsewhere), so the CLI runs on an explicitly-sized thread
/// instead.
const WORKER_STACK_SIZE: usize = 64 * 1024 * 1024;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    std::thread::Builder::new()
        .stack_size(WORKER_STACK_SIZE)
        .spawn(move || bpf_tcl::run(args.into_iter()))
        .expect("failed to spawn the bpf-tcl CLI worker thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}
