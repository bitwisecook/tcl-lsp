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

//! `foreach` iteration values: literal lists, nested braces and lists resolved through the lattice.

use super::*;

// Foreach constset extraction.

fn foreach_stmt(ssa: &mut SsaFunction, var: &str, list: &str, new_ver: u32) -> SsaStatement {
    let mut defs = HashMap::new();
    defs.insert(ssa.intern_var(var), new_ver);
    SsaStatement {
        statement: Statement::Call {
            span: Span::new(0, 0),
            command: "foreach".into(),
            canonical_command: None,
            args: vec![list.into()],
            defs: vec![var.into()],
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            // The CFG builder marks the synthetic loop header it emits
            // with its iterator-group sizes; the driver keys the header
            // layout on that typed fact, never on the command's name.
            foreach_groups: Some(vec![1]),
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

#[test]
fn extract_foreach_elements_literal_list() {
    // `list_text` is already delimiter-stripped by the segmenter, matching
    // the shape `foreach v {a b c}`'s `list_arg` is built in: the word's
    // own `{…}` is gone, leaving plain whitespace-separated text.
    assert_eq!(
        extract_foreach_elements("a b c", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a".into(), "b".into(), "c".into()])
    );
}

#[test]
fn extract_foreach_elements_rejects_substitutions() {
    assert_eq!(
        extract_foreach_elements("$lst", tcl_syntax::word_rules::WordValueRules::TCL),
        None
    );
    assert_eq!(
        extract_foreach_elements("[list a b c]", tcl_syntax::word_rules::WordValueRules::TCL),
        None
    );
}

#[test]
fn extract_foreach_elements_splits_nested_braces_as_tcl_list() {
    // `a {b c} d` is a three-element Tcl list — `a`, `b c`, `d` — not the
    // four whitespace runs `a`, `{b`, `c}`, `d`. A naive `split_ascii_whitespace`
    // corrupted the CONSTSET; the list-aware split fixes it.
    assert_eq!(
        extract_foreach_elements("a {b c} d", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a".into(), "b c".into(), "d".into()])
    );
    // Backslash-escaped whitespace groups an element too: `a\ b c` is two
    // elements `a b` and `c`.
    assert_eq!(
        extract_foreach_elements("a\\ b c", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a b".into(), "c".into()])
    );
}

#[test]
fn extract_foreach_elements_empty_list_returns_empty() {
    assert_eq!(
        extract_foreach_elements("", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(Vec::new())
    );
}

// `list_text` already has its outer
// `foreach`-word delimiter removed by the segmenter (see
// `Statement::Foreach::list_arg`'s construction in
// `lowering::structured`), so a value that itself starts with `{` / `"`
// is a *nested* list element, not a delimiter to peel a second time.
#[test]
fn extract_foreach_elements_single_element_braced_list_not_double_stripped() {
    // Source `foreach v {{a b c}}`: the segmenter strips the word's own
    // outer `{…}`, leaving the single-element list `{a b c}` as
    // `list_text`. Splitting it as a Tcl list yields the one element
    // `a b c`, not the three whitespace-separated words `a`, `b`, `c`
    // a second brace-strip-then-split would wrongly produce.
    assert_eq!(
        extract_foreach_elements("{a b c}", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a b c".into()])
    );
}

#[test]
fn extract_foreach_elements_two_braced_elements_not_double_stripped() {
    // Source `foreach v {{a b} {c d}}`: `list_text` is `{a b} {c d}`.
    // The pre-fix double-strip peeled the text's own outer `{`/`}` too,
    // yielding the corrupted split `a`, `b}`, `{c`, `d` (lenient split of
    // `a b} {c d`). The correct split is the two nested-list elements.
    assert_eq!(
        extract_foreach_elements("{a b} {c d}", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a b".into(), "c d".into()])
    );
}

#[test]
fn extract_foreach_elements_quoted_word_already_delimiter_stripped() {
    // Source `foreach v "a b c"`: the segmenter strips the word's own
    // `"…"` delimiters the same way it strips `{…}`, so `list_text` is
    // plain `a b c` — no quote handling is needed (or wanted) here.
    assert_eq!(
        extract_foreach_elements("a b c", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a".into(), "b".into(), "c".into()])
    );
}

#[test]
fn evaluate_def_foreach_literal_list_folds_constset() {
    // `list` mirrors `list_arg` as the segmenter hands it: source
    // `foreach v {1 2 3}`'s outer `{…}` is already stripped.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "1 2 3", 1);
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    match result {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 3);
            assert!(vs.contains(&ConstValue::int(1)));
            assert!(vs.contains(&ConstValue::int(3)));
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_list_cmd_subst_folds_constset() {
    // `foreach v [list a b c]` folds through the declared `list` route to the
    // same element CONSTSET as the braced-literal form.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "[list a b c]", 1);
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    match result {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 3);
            assert!(vs.contains(&ConstValue::String("a".into())));
            assert!(vs.contains(&ConstValue::String("c".into())));
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_single_element_folds_const() {
    // `list` mirrors `list_arg` for source `foreach v {only}`: the
    // segmenter's delimiter strip leaves the bare word `only`.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "only", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::String("only".into()))
    );
}

#[test]
fn evaluate_def_foreach_nested_braced_elements_folds_constset() {
    // Source `foreach v {{a b} {c d}}` hands `list_arg` = `{a b} {c d}`
    // (only the word's own outer `{…}` is stripped by the segmenter).
    // Peeling a *second* level of bracing off this already
    // delimiter-stripped text would fold `v` to the corrupted CONSTSET
    // {"a", "b}"} instead of the two list elements.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "{a b} {c d}", 1);
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    match result {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 2);
            assert!(vs.contains(&ConstValue::String("a b".into())));
            assert!(vs.contains(&ConstValue::String("c d".into())));
            assert!(!vs.contains(&ConstValue::String("b}".into())));
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_via_lattice_var() {
    let mut ssa = bare_ssa();
    let mut stmt = foreach_stmt(&mut ssa, "v", "$lst", 1);
    let lst = ssa.intern_var("lst");
    stmt.uses.insert(lst, 1);
    let mut values = HashMap::new();
    values.insert(
        (lst, 1),
        LatticeValue::Const(ConstValue::String("a b c".into())),
    );
    let result = evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default());
    match result {
        LatticeValue::ConstSet(ref vs) => assert_eq!(vs.len(), 3),
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_unbound_var_widens() {
    let mut ssa = bare_ssa();
    let mut stmt = foreach_stmt(&mut ssa, "v", "$lst", 1);
    let lst = ssa.intern_var("lst");
    stmt.uses.insert(lst, 1);
    // Empty lattice — var not bound.
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    assert_eq!(result, LatticeValue::Overdefined);
}

#[test]
fn evaluate_def_foreach_multi_var_binds_each_its_elements() {
    // Two binders over one list: each takes the elements it is
    // assigned, so over `a b` the first binder, `v`, holds `a`.
    assert_eq!(
        settled("tcl8.6", "proc p {} {foreach {v w} {a b} {puts $v$w}}", "v"),
        LatticeValue::Const(ConstValue::String("a".into()))
    );
}
