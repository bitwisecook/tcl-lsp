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

//! Exact UTF-16 column correctness across providers — the byte-vs-UTF-16 bug
//! class. Each test places the token of interest AFTER multibyte (and astral —
//! emoji, 2 UTF-16 units) characters on its line, so a byte/scalar confusion
//! shifts the column. Both directions are checked: INPUT (a UTF-16 request
//! column resolves to the intended token) and OUTPUT (returned ranges carry
//! UTF-16 columns, not byte columns).

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use std::collections::BTreeSet;
use std::time::Duration;

/// UTF-16 code-unit column where `token` first starts in `line`.
fn utf16_col(line: &str, token: &str) -> u32 {
    let idx = line.find(token).expect("token in line");
    // `len_utf16` is 1 or 2, so the conversion is always exact.
    line[..idx]
        .chars()
        .map(|c| u32::try_from(c.len_utf16()).unwrap())
        .sum()
}

/// The (wrong) UTF-8 byte column — what a byte-column regression would use.
fn byte_col(line: &str, token: &str) -> u32 {
    u32::try_from(line.find(token).expect("token in line")).expect("byte column fits u32")
}

/// The scalar (code-point) index of `token` in `line` — what a scalar-count
/// confusion would use.
fn scalar_col(line: &str, token: &str) -> u32 {
    let idx = line.find(token).expect("token in line");
    u32::try_from(line[..idx].chars().count()).expect("scalar column fits u32")
}

// -- TestUtf16InputColumnMapping -----------------------------------------
// A UTF-16 request column on a multibyte line resolves to the right token.

#[test]
fn definition_resolves_through_multibyte_prefix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set x 1\nputs \"café $x\"\n";
    lsp.open_ready(&uri, src);
    let line1 = "puts \"café $x\"";
    // Land on the `x` of `$x` (one UTF-16 unit past the `$`).
    let col = utf16_col(line1, "$x") + 1;
    let locs = starts(&lsp.definition(&uri, 1, col));
    assert!(
        locs.contains(&(0, 4)),
        "$x on a café-prefixed line must resolve to its definition at (0,4); \
         requested UTF-16 col {col}, got {locs:?}"
    );
}

#[test]
fn references_resolve_through_astral_prefix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // 🚀 is a single code point but TWO UTF-16 units — the column past it
    // differs between a UTF-16 and a scalar-count interpretation.
    let src = "set y 2\nputs \"🚀 $y here\"\nputs \"$y again\"\n";
    lsp.open_ready(&uri, src);
    let line1 = "puts \"🚀 $y here\"";
    let col = utf16_col(line1, "$y") + 1;
    let ref_lines: BTreeSet<i64> = starts(&lsp.references(&uri, 1, col, true))
        .into_iter()
        .map(|(line, _)| line)
        .collect();
    assert!(
        ref_lines.contains(&1) && ref_lines.contains(&2),
        "references from an astral-prefixed line must find both uses (lines 1,2); \
         requested UTF-16 col {col}, got lines {ref_lines:?}"
    );
}

// -- TestUtf16OutputColumns ----------------------------------------------
// Ranges the server returns carry UTF-16 columns, not byte columns.

#[test]
fn highlight_columns_are_utf16_not_bytes() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set z 3\nputs \"café résumé $z\"\n";
    lsp.open_ready(&uri, src);
    let line1 = "puts \"café résumé $z\"";
    let dollar = utf16_col(line1, "$z");
    let name = dollar + 1;
    let bcol = byte_col(line1, "$z");
    assert!(
        bcol > dollar,
        "test setup: multibyte prefix must shift the byte column"
    );
    // Request on the name; the returned highlight on line 1 must sit at the
    // UTF-16 column of either the `$` or the name — never the byte column.
    let cols: BTreeSet<i64> = starts(&lsp.document_highlight(&uri, 1, name))
        .into_iter()
        .filter(|(line, _)| *line == 1)
        .map(|(_, c)| c)
        .collect();
    let (dollar, name) = (i64::from(dollar), i64::from(name));
    assert!(
        !cols.is_empty(),
        "expected a highlight on line 1 for $z; got none"
    );
    assert!(
        cols.iter().all(|c| *c == dollar || *c == name),
        "highlight column must be UTF-16 ({dollar} or {name}), not the byte \
         column ({bcol}); got {cols:?}"
    );
}

#[test]
fn highlight_columns_are_utf16_through_astral_prefix() {
    // The astral-output counterpart of the BMP test above: 🚀 is a single code
    // point but TWO UTF-16 units, so a scalar (code-point) count yields a column
    // one short.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set z 3\nputs \"🚀 résumé $z\"\n";
    lsp.open_ready(&uri, src);
    let line1 = "puts \"🚀 résumé $z\"";
    let dollar = utf16_col(line1, "$z");
    let name = dollar + 1;
    let scalar = scalar_col(line1, "$z");
    // The 🚀 makes UTF-16 and scalar columns differ — that gap is what a
    // scalar→UTF-16 confusion would expose.
    assert!(
        dollar > scalar,
        "test setup: astral prefix must widen the UTF-16 column"
    );
    let cols: BTreeSet<i64> = starts(&lsp.document_highlight(&uri, 1, name))
        .into_iter()
        .filter(|(line, _)| *line == 1)
        .map(|(_, c)| c)
        .collect();
    let (dollar, name) = (i64::from(dollar), i64::from(name));
    assert!(
        !cols.is_empty(),
        "expected a highlight on line 1 for $z; got none"
    );
    assert!(
        cols.iter().all(|c| *c == dollar || *c == name),
        "highlight column must be UTF-16 ({dollar} or {name}), not the scalar \
         column ({scalar} / {}); got {cols:?}",
        scalar + 1
    );
}

// -- TestRapidEditConsistency --------------------------------------------
// Rapid full-document replaces leave the server's view consistent with the
// final content (no stale analysis surviving an out-of-order/queued change).

#[test]
fn final_state_wins_after_rapid_replaces() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc alpha {} { return 1 }\nalpha\n");
    // A burst of full replaces, each a distinct proc name, without waiting.
    let names = ["beta", "gamma", "delta", "epsilon"];
    let mut version = 2;
    for n in names {
        lsp.replace_document(
            &uri,
            version,
            &format!("proc {n} {{}} {{ return 1 }}\n{n}\n"),
        );
        version += 1;
    }
    // The final content defines `epsilon`; its call on line 1 must resolve to
    // the definition on line 0 — proving the latest change won. We settle on the
    // final version first, so the analysis snapshot is up to date before the
    // definition request.
    let final_version = version - 1;
    lsp.await_diagnostics_version(&uri, Some(final_version), Duration::from_secs(30));
    let defs = starts(&lsp.definition(&uri, 1, 0));
    assert!(
        defs.contains(&(0, 5)),
        "after rapid replaces, `epsilon` call must resolve to the final \
         definition at (0,5); got {defs:?}"
    );
    // And a stale name must NOT resolve anywhere (or, if it resolves, only to
    // the final definition).
    let line0 = starts(&lsp.definition(&uri, 0, 0));
    assert!(
        line0.is_empty() || line0.contains(&(0, 5)),
        "stale resolution: {line0:?}"
    );
}
