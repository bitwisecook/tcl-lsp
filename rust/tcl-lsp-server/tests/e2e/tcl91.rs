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

//! Native port of `tests/lsp_e2e/test_tcl91_e2e.py`.
//!
//! Tcl 9.1 dialect support, end-to-end over LSP. Oracle: C Tcl 9.1b0 source. The
//! dialect is pinned with a `# tcl-dialect: tcl9.1` directive (server-side source
//! detection), and behaviour is observed through completion + diagnostics.

use crate::common::codes;
use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

use std::collections::BTreeSet;

/// The set of completion labels from a completion result.
fn labels(result: &Value) -> BTreeSet<String> {
    completion_labels(result).into_iter().collect()
}

/// Open a buffer pinned to `dialect` and complete a command-position word.
fn complete_cmd(lsp: &mut Lsp, dialect: &str, partial: &str) -> BTreeSet<String> {
    let uri = unique_uri("tcl");
    let src = format!("# tcl-dialect: {dialect}\n{partial}\n");
    lsp.open_ready(&uri, &src);
    labels(&lsp.completion(&uri, 1, u32::try_from(partial.len()).unwrap()))
}

// -- TestTcl91Completion -------------------------------------------------

#[test]
fn unicode_and_timer_offered_in_91() {
    // doc/unicode.n, doc/timer.n — both are new commands in 9.1.
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "unic").contains("unicode"));
    assert!(complete_cmd(&mut lsp, "tcl9.1", "time").contains("timer"));
}

#[test]
fn unicode_and_timer_absent_in_90() {
    let mut lsp = Lsp::tcl();
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "unic").contains("unicode"));
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "time").contains("timer"));
}

#[test]
fn math_and_lfilter_offered_in_91() {
    // doc/divmod.n, doc/lfilter.n — new commands in 9.1 (C tclBasic.c).
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "divm").contains("divmod"));
    assert!(complete_cmd(&mut lsp, "tcl9.1", "lfil").contains("lfilter"));
}

#[test]
fn math_absent_in_90() {
    let mut lsp = Lsp::tcl();
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "divm").contains("divmod"));
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "lfil").contains("lfilter"));
}

#[test]
fn commands_90_still_offered_in_91() {
    // A `.1` release is additive: `lseq` (9.0) persists in 9.1.
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "lseq").contains("lseq"));
}

// -- TestTcl91Operators --------------------------------------------------
// doc/expr.n — the `lt`/`le`/`gt`/`ge` string operators (TIP 461) are 9.0+.

#[test]
fn lt_operator_no_w003_in_91() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl9.1\nexpr {$a lt $b}\n");
    assert!(!codes(&diags).contains("W003"));
}

#[test]
fn lt_operator_flags_w003_in_86() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.6\nexpr {$a lt $b}\n");
    assert!(codes(&diags).contains("W003"));
}

#[test]
fn ge_operator_no_w003_in_91() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl9.1\nexpr {$a ge $b}\n");
    assert!(!codes(&diags).contains("W003"));
}

#[test]
fn ge_operator_flags_w003_in_86() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.6\nexpr {$a ge $b}\n");
    assert!(codes(&diags).contains("W003"));
}
