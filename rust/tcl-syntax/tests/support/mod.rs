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

//! The real-tclsh matrix the conformance suites pin their vectors against.
//!
//! One interpreter per modelled release, keyed the way
//! `tcl-vm/tests/cross_version_info_surface_e2e.rs` keys it:
//! `TCL_LSP_TCLSH84` … `TCL_LSP_TCLSH91` name a binary explicitly, and each
//! release otherwise falls back to its conventional PATH name.  A release
//! with no binary is skipped **loudly** — the suite says which column went
//! unchecked rather than passing in silence.

use std::path::{Path, PathBuf};

use tcl_dialect::TclVersion;

/// Every release of the matrix that has a usable interpreter on this
/// machine, oldest first.
///
/// Prints one `skipping` line per release it could not find, so a run that
/// only covers part of the ladder says so in its output.
pub fn available_releases() -> Vec<(TclVersion, PathBuf)> {
    tcl_test_support::available_tclshs()
        .into_iter()
        .map(|interpreter| (interpreter.version, interpreter.path))
        .collect()
}

/// Run `script` on `tclsh`, returning its trimmed standard output.
///
/// # Errors
/// Returns the interpreter's standard error whenever the script exits
/// non-zero **or** writes anything to standard error.  Both halves matter:
/// `tclsh` reading a script from standard input reports a failed command
/// and then carries on with the next one, still exiting 0, so a row whose
/// setup a release rejects would otherwise report whatever the *rest* of
/// the script happened to do.
pub fn run_script(tclsh: &Path, script: &str) -> Result<String, String> {
    tcl_test_support::run_script(tclsh, script.as_bytes())
        .map_err(|error| error.to_string())?
        .strict_text()
}
