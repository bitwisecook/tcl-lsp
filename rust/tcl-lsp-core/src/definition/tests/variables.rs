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

//! Variable definitions: scope extents, loop variables, qualified names and parameters.

use super::*;

/// Scope extent and source-declaration membership must handle nested
/// namespace scopes without overflowing. 80 nested `namespace eval`
/// levels is past the point (confirmed empirically: 100+) where
/// unguarded namespace-scope recursion overflows `cargo test`'s bare
/// ~2 MiB per-test default. The assertion is that both return at all,
/// not what they return.
#[test]
fn deeply_nested_namespaces_survive_scope_extent_and_alias_scans() {
    const DEPTH: usize = 80;
    let mut source = String::new();
    for i in 0..DEPTH {
        let _ = writeln!(source, "namespace eval ns{i} {{");
    }
    source.push_str("set x 1\nputs $x\n");
    for _ in 0..DEPTH {
        source.push_str("}\n");
    }
    // The analyser runs on the server's worker stack; the scans under
    // test keep the default test stack.
    let analysed = source.clone();
    let analysis = crate::on_worker_stack(move || analyse(&analysed));
    let _ = variable_scope_extent(&analysis, "x", 0);
    let mut scopes = vec![&analysis.global_scope];
    while let Some(scope) = scopes.pop() {
        if let Some(var_def) = scope.variables.values().next() {
            let _ = linked_var_reference_spans(
                &source,
                &analysis,
                var_def,
                var_def.definition_span.start(),
            );
            break;
        }
        scopes.extend(&scope.children);
    }
    let _ = cross_namespace_qualified_vars(&analysis.global_scope, &FxHashSet::default());
}

// foreach multi-list lock-step form: `foreach
// varList1 list1 varList2 list2 ... body` binds every varList, not
// just the first.

#[test]
fn multi_list_foreach_name_resolves_to_its_own_loop_not_an_unrelated_later_one() {
    // The failure mode on the real pix corpus file (docs/pixdoc.tcl) — a
    // *second* `foreach` reusing the bare name `name` (a wholly
    // unrelated, later loop) wins whenever it is the only `VarDef` named
    // `name` in the flat top-level scope, which happens if the first
    // loop's own `name` (the second varList of a `foreach dirName
    // {...} name {...} {...}` multi-list form) is not bound at
    // all. tclsh9.0/8.6 both prove `name` inside the first loop's body
    // is that loop's own variable, never reaching the second loop.
    let src = "foreach dirName {src src {src core}} name {alpha beta gamma} {\n    puts \"$dirName $name\"\n    if {$name eq \"pixutils\"} { puts skip }\n}\nforeach name {examples color changes} {\n    puts $name.ruff\n}\n";
    let analysis = analyse(src);
    // Cursor on the first loop's own `$name` read (line 1, col 21).
    let locs = definition(src, 1, 21, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 0,
        "must resolve to the first loop's own `name` clause (line 0), not the unrelated second loop (line 4): {locs:?}"
    );
    assert_eq!(locs[0].start_character, 37);

    // Same wrong-location bug reproduced from a second usage inside
    // the same loop body (line 2, `if {$name eq ...}`).
    let locs2 = definition(src, 2, 10, &analysis);
    assert_eq!(locs2.len(), 1, "{locs2:?}");
    assert_eq!(locs2[0].start_line, 0, "{locs2:?}");
}

#[test]
fn jump_to_var_definition() {
    let src = "set greeting hi\nputs $greeting\n";
    let analysis = analyse(src);
    // Cursor inside `$greeting` on line 1.
    let locs = definition(src, 1, 8, &analysis);
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
}

#[test]
fn jump_to_proc_param_bareword_declaration_resolves_to_itself() {
    // TP — a cursor placed directly on a proc parameter's own bareword
    // name (not a `$`-prefixed read) must resolve, even though
    // `scope_chain_at` never reaches the proc's own scope for a byte
    // offset inside the parameter list, which sits textually *before*
    // that scope's `body_span` even starts.
    let src = "proc greet {name} { return $name }\n";
    let analysis = analyse(src);
    // Cursor on `name` inside the parameter list (col 12-16).
    let locs = definition(src, 0, 13, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
    assert_eq!(locs[0].start_character, 12, "{:?}", locs[0]);
    assert_eq!(locs[0].end_character, 16, "{:?}", locs[0]);
}

#[test]
fn jump_to_catch_resultvar_bareword_resolves_to_the_reused_variable() {
    // TP — the finding's other confirmed shape: `catch script name`
    // reuses (writes back into) an *existing* variable — its own
    // bareword token is recorded in `VarDef.references`, not
    // `definition_span` (that stays pinned to the variable's original
    // declaration) — so a cursor placed directly on it must still
    // resolve, to the *original* declaration site.
    let src = "proc resolveSwitch {name def} {\n    catch {foo} name\n    return $name\n}\n";
    let analysis = analyse(src);
    // Cursor on the catch result-var `name` (line 1, col 16-20).
    let locs = definition(src, 1, 17, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 0,
        "must resolve to the original param declaration: {:?}",
        locs[0]
    );
    assert_eq!(locs[0].start_character, 20, "{:?}", locs[0]);
    assert_eq!(locs[0].end_character, 24, "{:?}", locs[0]);
}

#[test]
fn bareword_that_is_not_any_variable_occurrence_does_not_resolve_as_a_variable() {
    // FP guard — an ordinary bareword that never appears as any
    // variable's declaration or reference span (an unrelated command's
    // plain argument) must not be mistaken for a variable declaration
    // site, even in a file containing unrelated variables.
    let src = "proc greet {name} { return $name }\nputs literal\n";
    let analysis = analyse(src);
    // Cursor on `literal` (line 1, col 5-12) — not a variable anywhere.
    let locs = definition(src, 1, 7, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn jump_to_absolutely_qualified_var_definition() {
    // TP — the shape defer.tcl's `$::defer::idVar` and uri.tcl's
    // namespace-current pattern both reduce to: a `variable` declared at
    // namespace-eval scope, referenced elsewhere via its fully
    // `::`-qualified name.
    let src = "namespace eval ::simple {\n    variable v \"hello\"\n}\nputs $::simple::v\n";
    let analysis = analyse(src);
    // Cursor on `v` inside `$::simple::v` (line 3, col 16).
    let locs = definition(src, 3, 16, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1, "{:?}", locs[0]);
    assert_eq!(locs[0].start_character, 13, "{:?}", locs[0]);
    assert_eq!(locs[0].end_character, 14, "name-sized span: {:?}", locs[0]);
}

#[test]
fn jump_to_relatively_qualified_var_definition_from_sibling_namespace() {
    // TP — a relative qualifier (`B::v`, no leading `::`) resolves under
    // the *cursor's* command-resolution namespace (tclsh 9.0.4/8.6.16
    // -verified: identical rule to what an unqualified command there
    // would resolve against), not the file's top level. `show` is
    // defined in `::A`, so `B::v` must reach `::A::B::v`, never a
    // top-level `::B::v`.
    let src = "namespace eval ::A {\n    namespace eval ::A::B {\n        variable v \"nested\"\n    }\n    proc show {} {\n        puts $B::v\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on `v` inside `$B::v` (line 5, col 17).
    let locs = definition(src, 5, 17, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 2, "{:?}", locs[0]);
    assert_eq!(locs[0].start_character, 17, "{:?}", locs[0]);
}

#[test]
fn qualified_var_definition_for_var_declared_in_a_reopened_namespace() {
    // TP — a namespace `namespace eval`-reopened in two unrelated
    // places: the qualified reference must find the declaration
    // regardless of which block declares it (no single lexical block
    // may be assumed to hold every variable a qualified reference
    // might mean).
    let src = "namespace eval ::A {\n    proc noop {} {}\n}\nnamespace eval ::A {\n    variable v 1\n}\nputs $::A::v\n";
    let analysis = analyse(src);
    let locs = definition(src, 6, 11, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 4, "{:?}", locs[0]);
}

#[test]
fn namespace_write_and_read_navigate_to_the_release_selected_storage() {
    let source =
        "set x global\nnamespace eval ::n { set x nsevalset }\nnamespace eval ::n { set y $x }\n";
    for (dialect, reaches_root) in [
        ("tcl8.4", true),
        ("tcl8.5", true),
        ("tcl8.6", true),
        ("tcl9.0", false),
        ("tcl9.1", false),
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
        let lines: Vec<_> = definition(source, 2, 28, &analysis)
            .into_iter()
            .map(|range| range.start_line)
            .collect();
        assert_eq!(
            lines,
            if reaches_root { &[0, 1][..] } else { &[1] },
            "{dialect}"
        );

        let locations = crate::references::references(
            source,
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            2,
            28,
            &analysis,
            true,
        );
        let lines: Vec<u32> = locations
            .into_iter()
            .map(|range| range.start_line)
            .collect();
        assert_eq!(
            lines,
            if reaches_root {
                &[0, 1, 2][..]
            } else {
                &[1, 2]
            },
            "{dialect}"
        );
    }
}

#[test]
fn qualified_var_definition_abstains_for_unknown_namespace() {
    // TN — a qualified reference into a namespace that's never declared
    // anywhere in the file must abstain (empty), not guess at an
    // unrelated variable.
    let src = "namespace eval ::real {\n    variable v 1\n}\nputs $::nonexistent::v\n";
    let analysis = analyse(src);
    let locs = definition(src, 3, 20, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn qualified_var_definition_does_not_match_a_same_named_proc_local() {
    // FP guard — a proc's local variable must never satisfy a
    // namespace-qualified lookup even though the proc's own
    // command-resolution namespace coincides with the target: `f`'s
    // local `v` is unrelated to `::A::v`, which is never declared here.
    let src = "namespace eval ::A {\n    proc f {} {\n        set v 1\n    }\n}\nputs $::A::v\n";
    let analysis = analyse(src);
    let locs = definition(src, 5, 11, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

/// Go-to-definition of a formal-parameter use must resolve to
/// the parameter *name* in the declaration, not the proc name (proc) or the
/// whole method/constructor body (`TclOO`). The returned range must be a
/// single-line, name-sized span over `arg1`.
#[test]
fn param_definition_points_at_the_name_not_the_body() {
    let cases = [
        // (source, usage line, usage char, expected decl line, decl start col)
        (
            "proc greet {arg1 arg2} {\n    puts $arg1\n}\n",
            1,
            11,
            0,
            12,
        ),
        (
            "oo::class create C {\n    method m {arg1 arg2} {\n        puts $arg1\n    }\n}\n",
            2,
            15,
            1,
            14,
        ),
        (
            "oo::class create C {\n    constructor {arg1 arg2} {\n        puts $arg1\n    }\n}\n",
            2,
            15,
            1,
            17,
        ),
    ];
    for (src, ul, uc, dl, dc) in cases {
        let analysis = analyse(src);
        let locs = definition(src, ul, uc, &analysis);
        assert_eq!(locs.len(), 1, "one def for {src:?}: {locs:?}");
        let r = locs[0];
        assert_eq!(r.start_line, dl, "decl line for {src:?}: {r:?}");
        assert_eq!(r.start_character, dc, "decl col for {src:?}: {r:?}");
        // Name-sized: same line, spans the 4-char `arg1`.
        assert_eq!(r.end_line, dl, "decl is single-line for {src:?}: {r:?}");
        assert_eq!(
            r.end_character - r.start_character,
            4,
            "decl spans `arg1` (4 chars) for {src:?}: {r:?}",
        );
    }
}

// scope-chain $var descent

#[test]
fn proc_local_var_jumps_to_proc_scope_definition() {
    // `local` is defined inside `proc f`.  Cursor on
    // `$local` inside `f`'s body must jump to the
    // proc-local definition, not the global scope
    // (which has none).
    let src = "proc f {} {\n    set local 1\n    puts $local\n}\n";
    let analysis = analyse(src);
    // Cursor inside `$local` on line 2.
    let locs = definition(src, 2, 12, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The proc-local `set local 1` is on line 1.
    assert_eq!(locs[0].start_line, 1, "{:?}", locs[0]);
}
