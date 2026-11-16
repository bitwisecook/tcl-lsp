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

use super::*;
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
use tcl_dialect::model::{Family, SurfaceLayer};

mod commands;
mod member_hazards;
mod member_targets;
mod members;
mod namespace_variables;
mod override_families;
mod variables;

use member_hazards::IDX79_HAZARD;

#[test]
fn rename_edit_plan_refuses_conflicts_before_utf16_publication() {
    use tcl_lexer::Span;
    let source = "αbc def";
    let index = LineIndex::new(source);
    let edit = |start, end, text: &str| PendingRenameEdit {
        span: Span::new(start, end),
        new_text: text.to_owned(),
    };
    let same = edit(0, 3, "x");
    let accepted = finish_rename_edits(
        source,
        &index,
        vec![same.clone(), edit(5, 8, "y"), same.clone()],
    )
    .unwrap();
    assert_eq!(accepted.len(), 2);
    assert_eq!(accepted[0].range.end_character, 2);
    assert_eq!(accepted[0].new_text, "x");
    assert!(finish_rename_edits(source, &index, vec![same.clone(), edit(0, 3, "z")]).is_err());
    assert!(finish_rename_edits(source, &index, vec![same.clone(), edit(2, 4, "x")]).is_err());
    assert!(finish_rename_edits(source, &index, vec![edit(0, 8, "x"), same]).is_err());
    assert!(finish_rename_edits(source, &index, vec![edit(1, 3, "x")]).is_err());
    assert!(finish_rename_edits(source, &index, vec![edit(5, 9, "x")]).is_err());
    assert!(finish_rename_edits(source, &index, vec![edit(2, 2, "x")]).is_err());
    assert_eq!(
        finish_rename_edits(source, &index, vec![edit(2, 3, "x"), edit(3, 4, "y")])
            .unwrap()
            .len(),
        2
    );
}

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

/// Apply every edit's `(range, new_text)` back onto `source` — the
/// same thing an editor does — sorted back-to-front so earlier byte
/// offsets stay valid as later edits are spliced in. Some tests below
/// use this to check the *result*, not just that `new_text` looks
/// right in isolation: a rename edit range short by one byte is
/// invisible to a test that only asserts `new_text` and never applies
/// it.
fn apply_edits(source: &str, edits: &[TextEdit]) -> String {
    let line_index = LineIndex::new(source);
    let mut spans: Vec<(usize, usize, &str)> = edits
        .iter()
        .map(|e| {
            let start = crate::definition::byte_offset_at(
                &line_index,
                source,
                e.range.start_line,
                e.range.start_character,
            ) as usize;
            let end = crate::definition::byte_offset_at(
                &line_index,
                source,
                e.range.end_line,
                e.range.end_character,
            ) as usize;
            (start, end, e.new_text.as_str())
        })
        .collect();
    spans.sort_by_key(|s| std::cmp::Reverse(s.0));
    let mut result = source.to_string();
    for (start, end, new_text) in spans {
        result.replace_range(start..end, new_text);
    }
    result
}

#[test]
fn prepare_rename_returns_range_for_proc() {
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 0, 6, &analysis).expect("expected prepare_rename on proc name");
    assert_eq!(p.placeholder, "greet");
    // Anchored at the proc name span.
    assert_eq!(p.range.start_line, 0);
    assert_eq!(p.range.start_character, 5);
}

#[test]
fn prepare_rename_returns_range_for_var() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 1, 7, &analysis).expect("expected prepare_rename on var");
    assert_eq!(p.placeholder, "x");
}

#[test]
fn original_prepare_variable_rename_selects_the_tail_and_preserves_index_geometry() {
    let source = "namespace eval N {}\nset ::N::a(k) 1\nlist $::N::a(k) ${::N::a(k)}\n";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut analysis = Analyser::new().analyse(source, dialect);
        analysis.global_scope.variables.clear();
        for reference in &mut analysis.qualified_var_refs {
            reference.qualified_name.clear();
        }
        for prefix in ["set ::N::", "list $::N::", "${::N::"] {
            let offset = source.find(prefix).unwrap() + prefix.len();
            let position =
                LineIndex::new(source).position_at_utf16(u32::try_from(offset).unwrap(), source);
            let prepared =
                prepare_rename(source, position.line, position.character.get(), &analysis)
                    .unwrap_or_else(|| panic!("{dialect}: {prefix}"));
            assert_eq!(prepared.placeholder, "a");
            assert_eq!(prepared.range.start_line, prepared.range.end_line);
            assert_eq!(
                prepared.range.end_character - prepared.range.start_character,
                1
            );
        }
        assert!(prepare_rename("# changed\n", 0, 2, &analysis).is_none());
    }
}

#[test]
fn prepare_rename_returns_none_for_unknown_word() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    // `hello` isn't a proc or var declaration.
    assert!(prepare_rename(src, 0, 6, &analysis).is_none());
}

// safety gating

#[test]
fn is_safe_symbol_name_accepts_canonical_identifiers() {
    assert!(is_safe_symbol_name("foo"));
    assert!(is_safe_symbol_name("Foo"));
    assert!(is_safe_symbol_name("_underscore"));
    assert!(is_safe_symbol_name("a1"));
    assert!(is_safe_symbol_name("snake_case_42"));
}

#[test]
fn is_safe_symbol_name_rejects_invalid_shapes() {
    assert!(!is_safe_symbol_name(""), "empty name should be invalid");
    assert!(!is_safe_symbol_name("1leading_digit"));
    assert!(!is_safe_symbol_name("has space"));
    assert!(!is_safe_symbol_name("has-dash"));
    assert!(!is_safe_symbol_name("has::colon"));
    assert!(!is_safe_symbol_name("with$dollar"));
    // Tcl-valid but rename-rejects (forces editor to refuse
    // partially-edited symbols).
    assert!(!is_safe_symbol_name("dotted.name"));
}

#[test]
fn rename_returns_empty_for_unsafe_new_name() {
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    // Whitespace in the new name fails the shape gate.
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            "bad name",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
    // Leading digit fails.
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            "1lead",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
    // Dash fails.
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            "with-dash",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
}

#[test]
fn rename_var_also_rejects_unsafe_new_name() {
    // The shape gate applies to variable renames too.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            7,
            "bad name",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
}

#[test]
fn rename_proc_to_builtin_command_name_blocked() {
    // Renaming `greet` to `puts` would shadow the built-in
    // `puts` command — the safety gate must reject the
    // rename when a registry is provided.
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "puts",
        &analysis,
        Some(&registry),
    );
    assert!(
        edits.is_empty(),
        "rename to built-in `puts` must produce no edits, got {edits:?}",
    );
}

#[test]
fn rename_proc_to_non_builtin_succeeds_with_registry() {
    // Renaming `greet` to a non-built-in name should still
    // succeed when the registry is provided.
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "salut",
        &analysis,
        Some(&registry),
    );
    assert!(
        !edits.is_empty(),
        "rename to non-built-in `salut` should produce edits",
    );
    assert!(edits.iter().all(|e| e.new_text == "salut"));
}

#[test]
fn rename_var_to_builtin_name_allowed() {
    // The built-in-shadow gate is proc-only.  Variable
    // names live in a separate namespace and can use any
    // shape-valid identifier — including `puts`.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "puts",
        &analysis,
        Some(&registry),
    );
    assert!(
        !edits.is_empty(),
        "variable rename to `puts` should succeed (different namespace)",
    );
}
