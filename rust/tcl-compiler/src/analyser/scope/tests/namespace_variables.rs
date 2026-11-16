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

//! Namespace-qualified variables: recorded occurrences, `lookup_var_in_namespace`, `qualified_name_for_var_decl` and `lookup_var_by_qualified_name`.

use super::*;

/// TP — a namespace-qualified occurrence is
/// recorded with the `::`-rooted cell it names, whether it is written
/// absolutely or relative to the enclosing namespace, so the workspace
/// index can match it against a declaration in another document.
#[test]
fn qualified_var_occurrences_are_recorded_with_their_rooted_cell() {
    let src = "namespace eval app::colors {\n    variable palette red\n}\n\
                   puts $app::colors::palette\n\
                   puts $::app::colors::palette\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let cells: Vec<&str> = r
        .qualified_var_refs
        .iter()
        .map(|v| v.qualified_name.as_str())
        .collect();
    assert_eq!(
        cells,
        vec!["::app::colors::palette", "::app::colors::palette"],
        "both spellings name one cell",
    );
    for reference in &r.qualified_var_refs {
        assert!(matches!(
            reference.original_name_input,
            Some(crate::signature_scan::scope::SignatureSourceNameInput::OriginalVariableRoot(_)),
        ));
        assert!(reference.original_namespace.is_some());
    }
}

/// TN — an **unqualified** occurrence is never recorded: it names whatever
/// the local scope chain supplies, which no other document can resolve.
#[test]
fn unqualified_var_occurrences_are_not_recorded_as_qualified_refs() {
    let src = "set counter 0\nincr counter\nputs $counter\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(
        r.qualified_var_refs.is_empty(),
        "bare names carry no cross-document identity: {:?}",
        r.qualified_var_refs,
    );
}

/// TP — the post-walk attach pass links a *relative*-qualified read to the
/// declaring namespace cell in the same document, including when the read
/// is written **before** the `namespace eval` that declares it.
///
/// Oracle (tclsh 9.0.4 / 8.6.16): `proc p {} { return $::n::v }; namespace
/// eval n { variable v 1 }; puts [p]` prints `1`.
#[test]
fn qualified_reads_attach_to_the_namespace_cell_regardless_of_order() {
    let src = "proc p {} { return $::n::v }\n\
                   namespace eval n {\n    variable v 1\n}\n\
                   puts $n::v\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let (_, var) = namespace_variables(&r.global_scope)
        .into_iter()
        .find(|(q, _)| q == "::n::v")
        .expect("::n::v declared");
    assert_eq!(
        var.references.len(),
        2,
        "both the earlier `$::n::v` and the later `$n::v` are references: {:?}",
        var.references,
    );
}

/// TP — `namespace_variables` enumerates exactly the cells
/// `lookup_var_in_namespace` can find, and never a proc local.
#[test]
fn namespace_variables_enumerates_namespace_cells_only() {
    let src = "namespace eval a::b {\n    variable v 1\n}\n\
                   proc a::b::f {} {\n    set localOnly 2\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let names: Vec<String> = namespace_variables(&r.global_scope)
        .into_iter()
        .map(|(q, _)| q)
        .collect();
    assert!(names.contains(&"::a::b::v".to_owned()), "{names:?}");
    assert!(
        !names.iter().any(|n| n.ends_with("::localOnly")),
        "a proc local is not a namespace cell: {names:?}",
    );
}

#[test]
fn lookup_var_in_namespace_finds_var_in_matching_top_level_namespace() {
    // TP — the base case both mined findings (idx=107, idx=115) reduce
    // to: `namespace eval ::simple { variable v ... }`, then `$::simple::v`
    // referenced elsewhere.
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
    ns_a.variables
        .insert("v".to_string(), var("v", span(10, 11)));
    root.children.push(ns_a);

    let found = lookup_var_in_namespace(&root, "::A", "v");
    assert_eq!(found.map(|v| v.definition_span), Some(span(10, 11)));
    assert!(lookup_var_in_namespace(&root, "::A", "missing").is_none());
}

#[test]
fn lookup_var_in_namespace_accumulates_through_nested_namespaces() {
    // TP — a namespace written 2+ levels deep must accumulate through
    // every enclosing `namespace eval`, exactly like command resolution
    // (`advance_command_resolution_namespace`, reused verbatim here).
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "A");
    let mut ns_b = Scope::new(ScopeKind::Namespace, "B");
    ns_b.variables
        .insert("v".to_string(), var("v", span(20, 21)));
    ns_a.children.push(ns_b);
    root.children.push(ns_a);

    assert_eq!(
        lookup_var_in_namespace(&root, "::A::B", "v").map(|v| v.definition_span),
        Some(span(20, 21))
    );
    // The intermediate namespace's own (empty) table must not be
    // mistaken for the leaf's.
    assert!(lookup_var_in_namespace(&root, "::A", "v").is_none());
}

#[test]
fn lookup_var_in_namespace_finds_var_defined_in_a_reopened_block() {
    // TP — the generalisation the mined findings' fix direction calls
    // for: a namespace can be `namespace eval`-reopened any number of
    // times (even lexically unrelated ones), so a single Scope tree
    // node must never be assumed to hold every variable a qualified
    // reference might mean. Two independent `::A` blocks here; only the
    // second declares `v`.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.children.push(Scope::new(ScopeKind::Namespace, "::A"));
    let mut ns_a_reopened = Scope::new(ScopeKind::Namespace, "::A");
    ns_a_reopened
        .variables
        .insert("v".to_string(), var("v", span(30, 31)));
    root.children.push(ns_a_reopened);

    assert_eq!(
        lookup_var_in_namespace(&root, "::A", "v").map(|v| v.definition_span),
        Some(span(30, 31))
    );
}

#[test]
fn lookup_var_in_namespace_skips_proc_locals_even_when_namespace_matches() {
    // FP guard — a proc's own defining ("command-resolution") namespace
    // can coincide with a target namespace path (any proc's does, by
    // construction), but its *local* variable table is never the
    // namespace's cells: a plain `set v 1` inside `proc ::A::f` has no
    // relation to a same-named `::A::v` unless linked via `variable`/
    // `global` (which resolves through the ordinary scope-chain walk,
    // not this namespace-table lookup). Only `f`'s *own* locals exist
    // here — `::A` itself declares nothing — so a hit would prove the
    // guard is missing.
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
    let mut proc_f = Scope::new(ScopeKind::Proc, "f");
    proc_f
        .variables
        .insert("v".to_string(), var("v", span(40, 41)));
    ns_a.children.push(proc_f);
    root.children.push(ns_a);

    assert!(
        lookup_var_in_namespace(&root, "::A", "v").is_none(),
        "a proc-local variable must never satisfy a namespace-qualified lookup"
    );
}

#[test]
fn lookup_var_in_namespace_returns_none_for_unknown_namespace() {
    // TN — a target namespace that appears nowhere in the tree.
    let root = Scope::new(ScopeKind::Global, "::");
    assert!(lookup_var_in_namespace(&root, "::Nope", "v").is_none());
}

#[test]
fn lookup_var_in_namespace_finds_global_scope_variable() {
    // TP — the degenerate `target_ns == "::"` case: the root scope
    // itself is `::`, so an absolute `$::v` reference must resolve
    // directly against the root's own variable table.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables.insert("v".to_string(), var("v", span(0, 1)));
    assert_eq!(
        lookup_var_in_namespace(&root, "::", "v").map(|v| v.definition_span),
        Some(span(0, 1))
    );
}

#[test]
fn qualified_name_for_var_decl_finds_var_in_matching_top_level_namespace() {
    // TP — the reverse of `lookup_var_in_namespace_finds_var_in_matching_top_level_namespace`:
    // given the declaration's own span, recover the
    // qualified name an alias elsewhere would name it by.
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
    ns_a.variables
        .insert("v".to_string(), var("v", span(10, 11)));
    root.children.push(ns_a);

    assert_eq!(
        qualified_name_for_var_decl(&root, span(10, 11)),
        Some("::A::v".to_string())
    );
}

#[test]
fn qualified_name_for_var_decl_accumulates_through_nested_namespaces() {
    // TP — mirrors `lookup_var_in_namespace_accumulates_through_nested_namespaces`.
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "A");
    let mut ns_b = Scope::new(ScopeKind::Namespace, "B");
    ns_b.variables
        .insert("v".to_string(), var("v", span(20, 21)));
    ns_a.children.push(ns_b);
    root.children.push(ns_a);

    assert_eq!(
        qualified_name_for_var_decl(&root, span(20, 21)),
        Some("::A::B::v".to_string())
    );
}

#[test]
fn qualified_name_for_var_decl_skips_proc_locals() {
    // FP guard — a proc-local variable's declaration span must never
    // produce a qualified name: `ScopeKind::Proc` is excluded from the
    // walk exactly like `lookup_var_in_namespace_skips_proc_locals_even_when_namespace_matches`,
    // since a bare `set v 1` inside a proc has no relation to any
    // namespace cell unless linked via `variable`/`global` (a separate
    // `VarDef` with its own `link_target`, not this one).
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
    let mut proc_f = Scope::new(ScopeKind::Proc, "f");
    proc_f
        .variables
        .insert("v".to_string(), var("v", span(40, 41)));
    ns_a.children.push(proc_f);
    root.children.push(ns_a);

    assert!(
        qualified_name_for_var_decl(&root, span(40, 41)).is_none(),
        "a proc-local variable's span must never resolve to a qualified name"
    );
}

#[test]
fn qualified_name_for_var_decl_returns_none_for_unknown_span() {
    // TN — a span that names no declaration anywhere in the tree.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables.insert("v".to_string(), var("v", span(0, 1)));
    assert!(qualified_name_for_var_decl(&root, span(99, 100)).is_none());
}

#[test]
fn qualified_name_for_var_decl_finds_global_scope_variable() {
    // TP — the degenerate root/`::` case: a variable declared directly
    // in the global scope qualifies as `::name`, not `::::name`.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables.insert("v".to_string(), var("v", span(0, 1)));
    assert_eq!(
        qualified_name_for_var_decl(&root, span(0, 1)),
        Some("::v".to_string())
    );
}

#[test]
fn qualified_name_for_var_decl_does_not_double_prefix_a_literal_qualified_name() {
    // TP: `bind_value_word_assignment`/`define_var` never
    // re-qualify a name they're given (`normalise_var_name` only strips
    // a `$`/`${…}` wrapper and an array index), so a literal `set
    // ::tolComp val` stores `VarDef::name == "::tolComp"` verbatim, not
    // the bare tail `"tolComp"`. Prefixing that with `ns` again would
    // produce `"::::tolComp"`, matching no alias's `link_target`.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables
        .insert("::tolComp".to_string(), var("::tolComp", span(50, 60)));
    assert_eq!(
        qualified_name_for_var_decl(&root, span(50, 60)),
        Some("::tolComp".to_string())
    );
}

#[test]
fn lookup_var_by_qualified_name_finds_a_bare_tail_namespace_variable() {
    // TP — the ordinary `variable`-declared namespace cell shape,
    // delegated straight through to `lookup_var_in_namespace`.
    let mut root = Scope::new(ScopeKind::Global, "::");
    let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
    ns_a.variables
        .insert("v".to_string(), var("v", span(10, 11)));
    root.children.push(ns_a);

    assert_eq!(
        lookup_var_by_qualified_name(&root, "::A::v").map(|v| v.definition_span),
        Some(span(10, 11))
    );
}

#[test]
fn lookup_var_by_qualified_name_finds_a_literal_qualified_top_level_set() {
    // TP — a plain `set ::tolComp val` at global scope stores its key
    // verbatim (`"::tolComp"`), which the bare-tail lookup alone
    // (`base_name == "tolComp"`) can never match; the literal-name
    // fallback must.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables
        .insert("::tolComp".to_string(), var("::tolComp", span(50, 60)));

    assert_eq!(
        lookup_var_by_qualified_name(&root, "::tolComp").map(|v| v.definition_span),
        Some(span(50, 60))
    );
}

#[test]
fn lookup_var_by_qualified_name_finds_an_unqualified_top_level_set() {
    // TP — the other half: an *unqualified* `set tolComp val` at global
    // scope stores the bare key `"tolComp"`, found by the tail-based
    // lookup with no fallback needed.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables
        .insert("tolComp".to_string(), var("tolComp", span(50, 60)));

    assert_eq!(
        lookup_var_by_qualified_name(&root, "::tolComp").map(|v| v.definition_span),
        Some(span(50, 60))
    );
}

#[test]
fn lookup_var_by_qualified_name_does_not_conflate_unrelated_literal_cells() {
    // FP guard — two literal-qualified cells in different namespaces
    // must never cross-match just because both are reachable via the
    // literal-name fallback.
    let mut root = Scope::new(ScopeKind::Global, "::");
    root.variables
        .insert("::A::x".to_string(), var("::A::x", span(1, 2)));
    root.variables
        .insert("::B::x".to_string(), var("::B::x", span(3, 4)));

    assert_eq!(
        lookup_var_by_qualified_name(&root, "::A::x").map(|v| v.definition_span),
        Some(span(1, 2))
    );
    assert_eq!(
        lookup_var_by_qualified_name(&root, "::B::x").map(|v| v.definition_span),
        Some(span(3, 4))
    );
}

#[test]
fn lookup_var_by_qualified_name_returns_none_for_unknown_target() {
    // TN
    let root = Scope::new(ScopeKind::Global, "::");
    assert!(lookup_var_by_qualified_name(&root, "::Nope::v").is_none());
}
