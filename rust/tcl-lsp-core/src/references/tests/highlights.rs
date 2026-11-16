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

//! Document highlights and their read and write kinds.

use super::*;

// read/write distinction

#[test]
fn document_highlights_var_records_write_at_definition() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    // Cursor inside `$x`.
    let highlights = document_highlights(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        &analysis,
    );
    // The defining `set x` span should be tagged Write.
    let writes: Vec<_> = highlights
        .iter()
        .filter(|(_, k)| *k == HighlightKind::Write)
        .collect();
    assert!(
        !writes.is_empty(),
        "expected at least one Write for `set x 1`; got {highlights:?}",
    );
    // The Write should be on line 0 (the `set` line).
    assert!(
        writes.iter().any(|(r, _)| r.start_line == 0),
        "expected Write on line 0; got {highlights:?}",
    );
}

#[test]
fn document_highlights_var_read_kind_is_correctly_tagged() {
    // Pure report conversion contract: selection and current-source
    // authority belong to the public caller; this helper only maps the
    // supplied declaration/read spans to Write/Read LSP ranges.
    use tcl_lexer::Span;
    let src = "set x 1\nputs $x\n";
    let line_index = LineIndex::new(src);
    let highlights = variable_occurrence_highlights(
        src,
        &line_index,
        [(Span::new(4, 5), true), (Span::new(13, 14), false)],
    );
    // Write at definition.
    assert!(
        highlights
            .iter()
            .any(|(r, k)| r.start_line == 0 && *k == HighlightKind::Write),
        "expected Write at line 0; got {highlights:?}",
    );
    // Read at the supplied reference.
    assert!(
        highlights
            .iter()
            .any(|(r, k)| r.start_line == 1 && *k == HighlightKind::Read),
        "expected Read at line 1; got {highlights:?}",
    );
}

#[test]
fn document_highlights_proc_decl_is_text() {
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let highlights = document_highlights(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
    );
    // Declaration on line 0 should be Text — procs carry no Write/Read
    // distinction (only variables do).
    let line0 = highlights
        .iter()
        .find(|(r, k)| r.start_line == 0 && *k == HighlightKind::Text);
    assert!(
        line0.is_some(),
        "expected Text on line 0 (declaration); got {highlights:?}",
    );
    // Call site on line 1 should be Text (no read/write
    // semantics on command-invocation heads).
    assert!(
        highlights
            .iter()
            .any(|(r, k)| r.start_line == 1 && *k == HighlightKind::Text),
        "expected Text on line 1 (call site); got {highlights:?}",
    );
}

#[test]
fn document_highlights_empty_for_unknown_symbol() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert_eq!(
        document_highlights(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis
        ),
        [] as [(
            crate::definition::LspRange,
            crate::references::HighlightKind
        ); 0]
    );
}

#[test]
fn document_highlights_surfaces_var_reads_from_arg_positions() {
    // With `record_arg_var_reads`, `$x`
    // reads in command arguments populate
    // `VarDef.references` and surface as `Read` spans in
    // the document-highlight provider.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    let highlights = document_highlights(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
    );
    let reads: Vec<_> = highlights
        .iter()
        .filter(|(_, k)| *k == HighlightKind::Read)
        .collect();
    assert!(
        reads.len() >= 2,
        "expected >= 2 Read entries (for two `$x` sites); got {highlights:?}",
    );
    // The defining `set x` span is Write.
    assert!(
        highlights
            .iter()
            .any(|(r, k)| r.start_line == 0 && *k == HighlightKind::Write),
        "expected Write on line 0; got {highlights:?}",
    );
}

#[test]
fn document_highlights_for_method_marks_decl_and_calls_text() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    let h = document_highlights(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
    );
    // Methods carry no Write/Read distinction — declaration + both call
    // sites are all Text (only variables are Write/Read).
    let writes = h.iter().filter(|(_, k)| *k == HighlightKind::Write).count();
    let texts = h.iter().filter(|(_, k)| *k == HighlightKind::Text).count();
    assert_eq!(writes, 0, "{h:?}");
    assert_eq!(texts, 3, "{h:?}");
}
