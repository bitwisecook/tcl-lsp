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

//! Native `tcl` CLI binary entry point.
//!
//! Mirrors the thin-main-over-lib pattern used by `tcl-lsp-server`: all
//! argument parsing and dispatch logic lives in [`tcl_cli`]; this binary just
//! forwards the process exit code.

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    tcl_cli::run(std::env::args_os())
}
