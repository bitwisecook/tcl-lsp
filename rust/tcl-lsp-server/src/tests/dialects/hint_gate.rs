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

//! The edit gate for in-source dialect hints.

use super::*;

/// Pure unit coverage of the dialect-hint edit gate — no async backend
/// needed, since [`change_could_affect_dialect_hint`] is a pure function
/// of the edit and the pre-edit buffer. A plain-file edit deep in the
/// document does not look hint-touching; the same edit landing on line 0
/// (or spelling out a directive/content-signature marker) does.
#[test]
fn dialect_hint_gate_skips_a_deep_plain_edit_but_catches_a_directive() {
    use std::fmt::Write as _;
    let mut body = String::from("# an ordinary Tcl script\n");
    for i in 0..500 {
        writeln!(body, "puts {i}").unwrap();
    }
    let index = tcl_lexer::LineIndex::new_lsp(&body);
    // An edit at line 500 that only touches ordinary text: no hint marker
    // anywhere near it, nowhere near the directive/shebang lines.
    let deep_edit = Range {
        start: pos(500, 5),
        end: pos(500, 6),
    };
    assert!(
        !change_could_affect_dialect_hint(&body, Some(deep_edit), "9", &index),
        "an edit to a bare digit deep in a plain file must not re-trigger detection"
    );
    // The same line, but the inserted text spells out a directive form —
    // caught by the marker-text check even though it's nowhere near the
    // top of the file.
    let deep_directive_edit = Range {
        start: pos(500, 0),
        end: pos(500, 0),
    };
    assert!(
        change_could_affect_dialect_hint(
            &body,
            Some(deep_directive_edit),
            "# tcl-dialect: tcl8.4\n",
            &index
        ),
        "inserting a tcl-dialect directive text must re-trigger detection"
    );
    // An edit on line 0 (where a directive or shebang could land) always
    // re-triggers, regardless of what the edit itself contains.
    let top_edit = Range {
        start: pos(0, 0),
        end: pos(0, 0),
    };
    assert!(
        change_could_affect_dialect_hint(&body, Some(top_edit), "x", &index),
        "an edit on the leading lines must always re-trigger detection"
    );
    // A full-document replacement always re-triggers.
    assert!(change_could_affect_dialect_hint(
        &body,
        None,
        "puts hi\n",
        &index
    ));
}

/// The marker test must widen the edit to the **lines** it touches, not
/// just the changed slice.
///
/// `detect_dialect` scans the whole source for the version-guard and
/// content-signature tiers, so a `package require Tcl 8.6` at line 500 is
/// live — and retyping its version digit changes what the detector
/// returns while the removed and inserted slices are both a bare digit
/// holding no marker at all.  The gate must re-trigger on that, and still
/// not on an edit to an ordinary line at the same depth.
#[test]
fn dialect_hint_gate_catches_an_edit_inside_a_deep_marker_line() {
    use std::fmt::Write as _;
    let mut body = String::from("# an ordinary Tcl script\n");
    for i in 0..500 {
        writeln!(body, "puts {i}").unwrap();
    }
    // Line 501 (0-based) carries a live version guard, far past the
    // leading-lines margin.
    let guard_line = u32::try_from(body.lines().count()).unwrap();
    body.push_str("package require Tcl 8.6\n");
    writeln!(body, "puts done").unwrap();
    let index = tcl_lexer::LineIndex::new_lsp(&body);
    // Retype the minor version: `8.6` → `8.4`. The slice on both sides of
    // the splice is one digit.
    let digit = u32::try_from("package require Tcl 8.".len()).unwrap();
    let version_edit = Range {
        start: pos(guard_line, digit),
        end: pos(guard_line, digit + 1),
    };
    assert!(
        change_could_affect_dialect_hint(&body, Some(version_edit), "4", &index),
        "a version-digit edit inside a deep `package require` line must re-trigger detection"
    );
    // Deleting the digit (no inserted text at all) is the same case.
    assert!(
        change_could_affect_dialect_hint(&body, Some(version_edit), "", &index),
        "deleting inside the marker line must re-trigger detection too"
    );
    // The plain line right after it, at the same depth, still does not.
    let plain_edit = Range {
        start: pos(guard_line + 1, 5),
        end: pos(guard_line + 1, 6),
    };
    assert!(
        !change_could_affect_dialect_hint(&body, Some(plain_edit), "x", &index),
        "an edit to an ordinary deep line must still skip the rescan"
    );
    // A marker typed one character at a time is caught the moment the
    // spliced line first spells one out, even though no single keystroke
    // ever carries the whole marker.
    let partial_line = u32::try_from(body.lines().count()).unwrap();
    body.push_str("packag\n");
    let index = tcl_lexer::LineIndex::new_lsp(&body);
    let finish_marker = Range {
        start: pos(partial_line, 6),
        end: pos(partial_line, 6),
    };
    assert!(
        change_could_affect_dialect_hint(&body, Some(finish_marker), "e", &index),
        "the keystroke that completes a marker on the line must re-trigger detection"
    );
}

/// The pre-edit half of the widened test: **removing** a marker matters as
/// The marker filter must be case-insensitive,
/// because the detector is.  The `tcl-dialect:` directive key is matched
/// with `eq_ignore_ascii_case` and the content-signature tiers lowercase
/// their words, so `# TCL-DIALECT: irules` typed deep in a document is
/// live to `detect_dialect` — a case-sensitive `contains` over the
/// lowercase marker list would skip the re-detect and strand the buffer
/// on its stale dialect.
#[test]
fn dialect_hint_gate_is_case_insensitive_like_the_detector() {
    use std::fmt::Write as _;
    let mut body = String::from("# an ordinary Tcl script\n");
    for i in 0..500 {
        writeln!(body, "puts {i}").unwrap();
    }
    let index = tcl_lexer::LineIndex::new_lsp(&body);
    let deep_insert = Range {
        start: pos(500, 0),
        end: pos(500, 0),
    };
    assert!(
        change_could_affect_dialect_hint(
            &body,
            Some(deep_insert),
            "# TCL-DIALECT: irules\n",
            &index
        ),
        "an upper-case directive is live to the detector and must re-trigger"
    );
    assert!(
        change_could_affect_dialect_hint(
            &body,
            Some(deep_insert),
            "PACKAGE REQUIRE Tcl 8.6\n",
            &index
        ),
        "upper-case marker words must re-trigger like their lower-case forms"
    );
}

/// much as adding one.  Deleting the `package require Tcl 8.4` line from
/// deep in a document changes the detected dialect back to the default,
/// and the deleted slice is the only place that marker still appears.
#[test]
fn dialect_hint_gate_catches_deleting_a_deep_marker_line() {
    use std::fmt::Write as _;
    let mut body = String::from("# an ordinary Tcl script\n");
    for i in 0..500 {
        writeln!(body, "puts {i}").unwrap();
    }
    let guard_line = u32::try_from(body.lines().count()).unwrap();
    body.push_str("package require Tcl 8.4\n");
    let index = tcl_lexer::LineIndex::new_lsp(&body);
    let delete_line = Range {
        start: pos(guard_line, 0),
        end: pos(guard_line + 1, 0),
    };
    assert!(
        change_could_affect_dialect_hint(&body, Some(delete_line), "", &index),
        "deleting the whole marker line must re-trigger detection"
    );
}
