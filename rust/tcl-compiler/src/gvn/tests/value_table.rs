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

//! The scoped value table, expression keys and message text.

use super::*;

#[test]
fn scoped_table_root_scope_always_present() {
    let t = ScopedValueTable::new();
    assert_eq!(t.scope_depth(), 1);
    assert!(t.lookup(&vec!["call".into(), "foo".into()]).is_none());
}

#[test]
fn insert_and_lookup_in_root() {
    let mut t = ScopedValueTable::new();
    let e = entry(&["call", "llength", "$x@1"], "entry", 2);
    t.insert(e.clone());
    let key: ExprKey = vec!["call".into(), "llength".into(), "$x@1".into()];
    assert_eq!(t.lookup(&key), Some(&e));
}

#[test]
fn pop_scope_discards_inner_entries_only() {
    let mut t = ScopedValueTable::new();
    t.insert(entry(&["call", "root"], "b", 0));
    t.push_scope();
    t.insert(entry(&["call", "inner"], "b", 1));
    assert_eq!(t.total_entries(), 2);
    t.pop_scope();
    let root_key: ExprKey = vec!["call".into(), "root".into()];
    let inner_key: ExprKey = vec!["call".into(), "inner".into()];
    assert!(t.lookup(&root_key).is_some());
    assert!(t.lookup(&inner_key).is_none());
}

#[test]
fn pop_scope_preserves_root_scope() {
    let mut t = ScopedValueTable::new();
    t.pop_scope(); // Should be a no-op; root always survives.
    t.pop_scope();
    assert_eq!(t.scope_depth(), 1);
}

#[test]
fn lookup_shadows_outer_scope_first() {
    let mut t = ScopedValueTable::new();
    let key: ExprKey = vec!["call".into(), "f".into()];
    t.insert(entry(&["call", "f"], "outer", 0));
    t.push_scope();
    t.insert(entry(&["call", "f"], "inner", 5));
    assert_eq!(t.lookup(&key).unwrap().block, "inner");
    t.pop_scope();
    assert_eq!(t.lookup(&key).unwrap().block, "outer");
}

#[test]
fn kill_all_drops_every_scope() {
    let mut t = ScopedValueTable::new();
    t.insert(entry(&["call", "a"], "b", 0));
    t.push_scope();
    t.insert(entry(&["call", "b"], "b", 1));
    t.kill_all();
    // `kill_all` drops every entry but preserves the scope-stack depth, so
    // the push/pop discipline survives. Collapsing the
    // stack would strand later inserts in a never-popped root scope.
    assert_eq!(t.scope_depth(), 2);
    assert_eq!(t.total_entries(), 0);
}

#[test]
fn kill_all_mid_branch_does_not_leak_to_sibling() {
    // replicated at the table level exactly as the
    // dominator walk drives it: enter block, enter then-arm, a state write
    // kills, record an occurrence, leave the then-arm, enter the else-arm.
    // The then-arm's occurrence must not be visible in the sibling.
    let key: ExprKey = vec!["call".into(), "llength".into(), "$lst".into()];
    let mut t = ScopedValueTable::new(); // root, depth 1
    t.push_scope(); // enter condition block, depth 2
    t.push_scope(); // enter then-arm, depth 3
    t.kill_all(); // a state write invalidates everything
    t.insert(entry(&["call", "llength", "$lst"], "then", 0));
    t.pop_scope(); // leave then-arm → depth 2, entry dropped
    t.push_scope(); // enter else-arm, depth 3
    assert!(
        t.lookup(&key).is_none(),
        "then-arm occurrence leaked into the else-arm after kill_all",
    );
}

#[test]
fn redundant_computation_constructor() {
    let r = RedundantComputation::new(
        Span::new(10, 20),
        Span::new(0, 5),
        "llength $x",
        DiagCode::O105,
        "message",
    );
    assert_eq!(r.expression_text, "llength $x");
    assert_eq!(r.code, DiagCode::O105);
    assert_eq!(r.span.start(), 10);
    assert_eq!(r.first_span.end(), 5);
}

#[test]
fn canonicalise_empty_uses_returns_input() {
    let ssa = bare_ssa();
    let uses = HashMap::new();
    assert_eq!(canonicalise_word("foo", &uses, &ssa, STYLE), "foo");
}

#[test]
fn canonicalise_replaces_bare_and_braced() {
    let mut ssa = bare_ssa();
    let mut uses = HashMap::new();
    uses.insert(ssa.intern_var("x"), 3);
    assert_eq!(canonicalise_word("$x", &uses, &ssa, STYLE), "$x@3");
    assert_eq!(canonicalise_word("${x}", &uses, &ssa, STYLE), "$x@3");
    assert_eq!(
        canonicalise_word("é$x:tail", &uses, &ssa, STYLE),
        "é$x@3:tail"
    );
    assert_eq!(canonicalise_word(r"\$x", &uses, &ssa, STYLE), r"\$x");
}

/// The `${…}` closer follows the release rule, so a
/// nested-brace name is *versioned* rather than copied through verbatim.
///
/// The verbatim copy is the unsound half: two occurrences of `${a{b}c}` at
/// different SSA versions both render to the same literal text, so their
/// `ExprKey`s match and GVN reports the second as redundant — a CSE that
/// reuses a stale value: `set {a{b}c} 7; subst {${a{b}c}}` is `7`
/// on tclsh 9.0.4 and `can't read "a{b"` on 8.6.16.
#[test]
fn canonicalise_braced_var_follows_the_release_rule() {
    use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
    let mut ssa = bare_ssa();
    let mut uses = HashMap::new();
    let nested = ssa.intern_var("a{b}c");
    let first = ssa.intern_var("a{b");
    uses.insert(nested, 4);
    uses.insert(first, 9);

    // 9.x: the whole name is `a{b}c`, so the reference carries its version.
    assert_eq!(
        canonicalise_word("${a{b}c}", &uses, &ssa, Tcl9Nesting),
        "$a{b}c@4"
    );
    // 8.x: the name ends at the first `}` and `c}` is ordinary word text.
    assert_eq!(
        canonicalise_word("${a{b}c}", &uses, &ssa, FirstClose),
        "$a{b@9c}"
    );

    // Two versions of the same nested name must not canonicalise alike —
    // the property the unsound CSE turned on.
    let mut other = HashMap::new();
    other.insert(nested, 5);
    assert_ne!(
        canonicalise_word("${a{b}c}", &uses, &ssa, Tcl9Nesting),
        canonicalise_word("${a{b}c}", &other, &ssa, Tcl9Nesting),
    );
}

#[test]
fn canonicalise_sorts_by_name_length_desc() {
    // `$longname` must be replaced before `$long` so the
    // longer name is not partially matched.
    let mut ssa = bare_ssa();
    let mut uses = HashMap::new();
    uses.insert(ssa.intern_var("long"), 1);
    uses.insert(ssa.intern_var("longname"), 2);
    let out = canonicalise_word("$longname$long", &uses, &ssa, STYLE);
    assert_eq!(out, "$longname@2$long@1");
}

#[test]
fn canonicalise_ignores_unmentioned_variables() {
    let ssa = bare_ssa();
    let uses = HashMap::new();
    assert_eq!(canonicalise_word("$x", &uses, &ssa, STYLE), "$x");
}

#[test]
fn build_call_key_for_pure_command() {
    let mut ssa = bare_ssa();
    let mut uses = HashMap::new();
    uses.insert(ssa.intern_var("x"), 3);
    let args = vec!["$x".into(), "literal".into()];
    let key = build_call_key("llength", &args, &uses, &ssa, STYLE);
    assert_eq!(
        key,
        vec![
            "call".to_string(),
            "llength".into(),
            "$x@3".into(),
            "literal".into()
        ]
    );
}

#[test]
fn format_expression_text_no_args() {
    let args: Vec<String> = Vec::new();
    assert_eq!(format_expression_text("clock", &args), "clock");
}

#[test]
fn format_expression_text_with_args() {
    let args: Vec<String> = vec!["$x".into(), "literal".into()];
    assert_eq!(
        format_expression_text("llength", &args),
        "llength $x literal"
    );
}

#[test]
fn message_builders_include_expression_text() {
    assert!(full_redundancy_message("llength $x").contains("llength $x"));
    assert!(full_redundancy_message("llength $x").contains("local variable"));
    assert!(partial_redundancy_message("dict get $d k").contains("partially redundant"));
    assert!(loop_invariant_message("expr {$x + 1}").contains("loop-invariant"));
}
