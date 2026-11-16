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

//! A brace-quoted name that contains `$` is its own variable cell for every provider.

use super::*;

// A brace-quoted `$`-bearing name is its own variable.
//
// tclsh 9.0.4 and 8.6.14, byte-identical:
//
//   set {$n} v ; set {$n}             -> v
//   info exists {$n} / info exists n  -> 1 / 0     (two distinct variables)
//   set n other ; set {$n}            -> v         (`n` never affects it)
//   set ${$n}                         -> can't read "v"  (`${$n}` read `$n`)
//
// So `{$n}` and `n` are two cells, and an edit to one must never touch the
// other.  Normalising `{$n}` down to `n` merges the two into one `VarDef`, so
// renaming the *plain* `n` emits an edit over the `{$n` span — turning
// `set {$n} 1` into `set q} 1`, which does not parse.

/// The mixed script: line 1 writes the literal `$n`, line 2 the plain `n`,
/// line 3 reads the plain one, line 4 reads the literal one.
const BRACE_LITERAL_MIX: &str = "proc f {} {\n    \
set {$n} 1\n    \
set n 2\n    \
puts $n\n    \
puts [set {$n}]\n\
}\n";

#[test]
fn references_of_the_plain_name_exclude_the_brace_literal_cell() {
    let analysis = analyse(BRACE_LITERAL_MIX);
    // Cursor on the `n` of `set n 2` (line 2).
    let refs = references(
        BRACE_LITERAL_MIX,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        8,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs),
        vec![2, 3],
        "the plain `n`'s sites are its own write and its own `$n` read — the \
`{{$n}}` word on line 1 is a different variable; got {refs:?}"
    );
}

#[test]
fn rename_of_the_plain_name_never_rewrites_a_brace_literal_word() {
    let analysis = analyse(BRACE_LITERAL_MIX);
    let edits = rename(
        BRACE_LITERAL_MIX,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        2,
        8,
        "q",
        &analysis,
        None,
    );
    assert_eq!(
        edit_lines(&edits),
        vec![2, 3],
        "renaming `n` must leave both `{{$n}}` words alone; got {edits:?}"
    );
    // And the applied result is still the same program for the literal cell.
    assert!(
        edits
            .iter()
            .all(|e| e.range.start_line != 1 && e.range.start_line != 4),
        "no edit may land on a `{{$n}}` word; got {edits:?}"
    );
}

#[test]
fn rename_of_a_brace_literal_name_is_refused_with_a_reason() {
    use tcl_lsp_core::rename::rename_with_diagnosis;
    let analysis = analyse(BRACE_LITERAL_MIX);
    // Cursor inside the `{$n}` word on line 1.
    let refusal = rename_with_diagnosis(
        BRACE_LITERAL_MIX,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        10,
        "q",
        &analysis,
        None,
    )
    .expect_err("a quoted-only name cannot be renamed by span substitution");
    assert!(
        refusal.reason.contains("$n") && refusal.reason.contains("quoted"),
        "the refusal must name the cell and say why; got {:?}",
        refusal.reason
    );
    // The refusal is typed, not an empty edit set: an empty set would fall
    // through to the server's cross-document rename branches.
    assert!(refusal.range.is_some(), "{refusal:?}");
}

#[test]
fn rename_of_an_ordinary_name_is_still_allowed() {
    // TN control for the refusal gate: it must key on the *target's* spelling,
    // not merely on a brace-quoted word being present in the document.
    use tcl_lsp_core::rename::rename_with_diagnosis;
    let analysis = analyse(BRACE_LITERAL_MIX);
    let edits = rename_with_diagnosis(
        BRACE_LITERAL_MIX,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        2,
        8,
        "q",
        &analysis,
        None,
    )
    .expect("an ordinary local is renameable");
    assert_eq!(edit_lines(&edits), vec![2, 3], "{edits:?}");
}

// Every variable provider resolves a brace-literal cursor to the literal
// cell.
//
// The character scan behind `find_var_at_position` reports `n` for a cursor on
// the `n` of `set {$n} 1`.  That word is brace-quoted, so Tcl substitutes
// nothing in it: the `$n` there is not a reference to `n`, it is part of the
// literal *name* of a different variable.  Every variable provider started
// with that scan and short-circuited on it, so a cursor one column right of
// the `{` stopped resolving the cell whose declaration it is sitting inside —
// answering the unrelated plain cell where the scope gate allowed it, and
// nothing at all where it did not.  Rename was the only one guarded.
//
// All five now resolve through `substituting_var_at_position`, so the `{`,
// `$` and `n` columns of one word answer alike.

/// Plain `n` declared *before* the literal cell — the ordering in which a
/// scope-chain lookup for `n` would succeed and hand back the wrong cell.
const BRACE_LITERAL_PLAIN_FIRST: &str = "proc f {} {\n    \
set n 2\n    \
puts $n\n    \
set {$n} 1\n    \
puts [set {$n}]\n\
}\n";

/// The literal cell alone — no `n` in scope for the mis-resolution to land on.
const BRACE_LITERAL_ONLY: &str = "proc f {} {\n    \
set {$n} 1\n    \
puts [set {$n}]\n\
}\n";

#[test]
fn every_variable_provider_resolves_a_brace_literal_cursor_to_the_literal_cell() {
    // (source, line of `set {$n} 1`, line of its `[set {$n}]` read)
    for (label, src, decl_line, read_line) in [
        (
            "plain `n` declared first",
            BRACE_LITERAL_PLAIN_FIRST,
            3u32,
            4u32,
        ),
        ("plain `n` declared after", BRACE_LITERAL_MIX, 1, 4),
        ("no plain `n` at all", BRACE_LITERAL_ONLY, 1, 2),
    ] {
        let analysis = analyse(src);
        // All three columns of the one word: the `{`, the `$`, the `n`.  The
        // `{` column already worked (the character scan finds no `$` there);
        // the other two are the regression.
        for col in [8u32, 9, 10] {
            let want = LspRange {
                start_line: decl_line,
                start_character: 8,
                end_line: decl_line,
                end_character: 11,
            };

            let defs = tcl_lsp_core::definition::definition(src, decl_line, col, &analysis);
            assert_eq!(
                defs,
                vec![want],
                "{label}: go-to-definition at column {col} must reach the `{{$n}}` \
declaration"
            );

            let hover = hover_text(src, decl_line, col, &analysis)
                .unwrap_or_else(|| panic!("{label}: hover at column {col} must resolve"));
            assert!(
                hover.contains("**Variable** `$n`"),
                "{label}: hover at column {col} must name the literal cell; got {hover:?}"
            );

            // Both of the literal cell's own sites: its declaration and the
            // `[set {$n}]` read.  The read is a registry `VarRead`-role name
            // word, recorded generically; without that, the cell's reference
            // list holds only the declaration and Find References
            // under-reports every `[set X]`-shaped read.
            let read = LspRange {
                start_line: read_line,
                start_character: 14,
                end_line: read_line,
                end_character: 17,
            };
            let refs = references(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                decl_line,
                col,
                &analysis,
                true,
            );
            assert_eq!(
                refs,
                vec![want, read],
                "{label}: find-references at column {col} must answer the literal cell's \
own declaration and read"
            );

            // Document-highlight resolves `$ref` cursors only — it has no
            // declaration-offset search, so it answers nothing for a *plain*
            // bareword declaration cursor either (`set n 2` with the cursor on
            // `n` → `[]`).  What matters here is that the braced cursor is no
            // longer answered from the wrong cell: it now abstains exactly as
            // the plain declaration cursor does.  Teaching that provider
            // declaration cursors is an independent gap, not this finding.
            let highlights = tcl_lsp_core::references::document_highlights(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                decl_line,
                col,
                &analysis,
            );
            assert!(
                highlights.is_empty(),
                "{label}: document-highlight at column {col} must abstain like a \
plain declaration cursor, not answer another cell; got {highlights:?}"
            );
        }
    }
}

#[test]
fn a_brace_literal_cursor_never_answers_the_plain_cells_sites() {
    // FP guard for the pair above, stated the other way round: whatever the
    // providers answer from inside `{$n}`, none of it may be a site of the
    // unrelated plain `n` — line 1 (`set n 2`) or line 2 (`puts $n`) here.
    let src = BRACE_LITERAL_PLAIN_FIRST;
    let analysis = analyse(src);
    let plain_lines = [1u32, 2];
    for col in [8u32, 9, 10] {
        for r in tcl_lsp_core::definition::definition(src, 3, col, &analysis) {
            assert!(
                !plain_lines.contains(&r.start_line),
                "definition at column {col} leaked a plain-`n` site: {r:?}"
            );
        }
        for r in references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            3,
            col,
            &analysis,
            true,
        ) {
            assert!(
                !plain_lines.contains(&r.start_line),
                "references at column {col} leaked a plain-`n` site: {r:?}"
            );
        }
        let hover = hover_text(src, 3, col, &analysis).unwrap_or_default();
        assert!(
            !hover.contains("**Variable** `n`\n"),
            "hover at column {col} rendered the plain cell: {hover:?}"
        );
    }
}

#[test]
fn a_plain_cursor_is_unaffected_by_the_shared_gate() {
    // TN control: the ordinary `$ref` and bareword-declaration cursors keep
    // resolving exactly as before, in a document that also holds a literal
    // cell.  `set n 2` is line 1, its `$n` read line 2.
    let src = BRACE_LITERAL_PLAIN_FIRST;
    let analysis = analyse(src);
    let decl = LspRange {
        start_line: 1,
        start_character: 8,
        end_line: 1,
        end_character: 9,
    };
    for (line, col) in [(1u32, 8u32), (2, 10)] {
        assert_eq!(
            tcl_lsp_core::definition::definition(src, line, col, &analysis),
            vec![decl],
            "the plain cell still resolves from ({line},{col})"
        );
        let hover = hover_text(src, line, col, &analysis).expect("plain hover");
        assert!(hover.contains("**Variable** `n`"), "{hover:?}");
        assert_eq!(
            ref_lines(&references(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                line,
                col,
                &analysis,
                true
            )),
            vec![1, 2],
            "the plain cell's own sites, and only those"
        );
    }
}

#[test]
fn rename_still_refuses_from_every_column_of_the_brace_literal_word() {
    // The refusal gate resolves the same cell the other providers now do, so
    // it fires from all three columns and names `$n` — not `n`.
    use tcl_lsp_core::rename::rename_with_diagnosis;
    let src = BRACE_LITERAL_PLAIN_FIRST;
    let analysis = analyse(src);
    for col in [8u32, 9, 10] {
        let refusal = rename_with_diagnosis(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            3,
            col,
            "q",
            &analysis,
            None,
        )
        .expect_err("a quoted-only name is refused from every column");
        assert!(
            refusal.reason.contains("`$n`"),
            "column {col} must refuse for the literal cell: {:?}",
            refusal.reason
        );
    }
    // TN control: the plain cell in the same document still renames, and the
    // edits stay off the `{$n}` words (lines 3 and 4).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        8,
        "q",
        &analysis,
        None,
    );
    assert_eq!(edit_lines(&edits), vec![1, 2], "{edits:?}");
}

#[test]
fn genuinely_inert_dollar_shapes_still_resolve_to_nothing() {
    // TN control for the inert-text proof in the shared gate: a `$v`-shaped
    // substring Tcl never substitutes is still not a reference, in a data
    // brace or a comment.
    for (src, line, col) in [
        ("set v 1\nputs {plain $v here}\n", 1u32, 14u32),
        ("set v 1\nputs hi ;# note $v\n", 1, 18),
    ] {
        let analysis = analyse(src);
        assert!(
            tcl_lsp_core::definition::definition(src, line, col, &analysis).is_empty(),
            "{src:?} at ({line},{col}) must stay suppressed"
        );
        assert!(
            hover_text(src, line, col, &analysis).is_none(),
            "{src:?} at ({line},{col}) must stay suppressed"
        );
        assert!(
            references(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                line,
                col,
                &analysis,
                true
            )
            .is_empty(),
            "{src:?} at ({line},{col}) must stay suppressed"
        );
    }
}
