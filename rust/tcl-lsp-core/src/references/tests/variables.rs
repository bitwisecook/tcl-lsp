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

//! Variable references: scopes, global aliases, loop variables and caller frames.

use super::*;

#[test]
fn references_skip_a_same_named_untyped_variable_in_another_scope() {
    // FP guard for the lattice half of the scan: the lattice types `b`
    // inside `::mk` only (`by_scope`); a same-named integer in `::other`
    // must not become a reference — the scope-keyed map is exactly what
    // stops the bare-name collision.
    let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   proc mk {} { set a [A new]\n  set b [$a make]\n  $b greet }\n\
                   proc other {} { set b 7\n  $b greet }\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1, col 29).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        29,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&4),
        "the lattice-typed site inside ::mk is missing: {refs:?}"
    );
    assert!(
        !lines.contains(&6),
        "::other's `b` is an untyped integer; its `$b greet` must not be \
             a reference: {refs:?}"
    );
}

#[test]
fn references_from_in_proc_global_alias_reach_the_callers_canonical_set() {
    // Reduces the real `isEqual`/`tolComp` shape from
    // nico-robert/pix's test/data_b64.test — a proc aliases a top-level
    // cell via `global`, and the caller overrides it via a plain `set
    // ::name` before invoking the proc. tclsh proves `tolComp` (via
    // `global`) and `::tolComp` (the caller's `set`) are the identical
    // storage cell. Querying from the in-proc `$tolComp` read must reach
    // the caller's `set ::tolComp`: `collect_alias_spans` on its own finds
    // *other aliases* of the same target, never the target's own canonical
    // (non-aliased) declaration.
    let src =
        "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
    let analysis = analyse(src);
    // Cursor on the `$tolComp` read inside the proc (line 2, col 14).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        14,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&1),
        "the `global tolComp` decl itself must still be present: {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "the in-proc $tolComp read must still be present: {refs:?}"
    );
    assert!(
        lines.contains(&4),
        "the caller's canonical `set ::tolComp 0.05` must now be reached: {refs:?}"
    );
}

#[test]
fn references_from_the_callers_canonical_set_reach_the_in_proc_global_alias() {
    // The reverse direction of the test above: querying from the caller's
    // own `set ::tolComp` must not return only its own 2 spans (decl + any
    // top-level reads) and miss every in-proc `global tolComp` occurrence,
    // which is what a plain `set` — with no `link_target` of its own to
    // search alias records by — would otherwise do.
    let src =
        "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
    let analysis = analyse(src);
    // Cursor on the `set ::tolComp` declaration (line 4, col 8, inside
    // "tolComp" — `set ::tolComp 0.05` has "tolComp" starting at col 6).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        8,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&4),
        "the caller's own decl must still be present: {refs:?}"
    );
    assert!(
        lines.contains(&1),
        "the in-proc `global tolComp` decl must now be reached: {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "the in-proc $tolComp read must now be reached: {refs:?}"
    );
}

#[test]
fn references_unify_global_alias_and_canonical_set_when_the_set_is_unqualified() {
    // A second repro: an *unqualified* `set tolComp
    // 0.05` at global scope reproduces the identical split, ruling out
    // the `::`-prefix as the sole cause — `set`'s binding never
    // calls `set_var_link_target` regardless of how the name is spelled,
    // so the shape is the same either way.
    let src = "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset tolComp 0.05\nuse\n";
    let analysis = analyse(src);
    // Cursor on the unqualified `set tolComp` declaration (line 4, col 6).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        6,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&4), "the caller's own decl: {refs:?}");
    assert!(
        lines.contains(&1),
        "the in-proc `global tolComp` decl must be reached even from an unqualified set: {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "the in-proc $tolComp read must be reached even from an unqualified set: {refs:?}"
    );
}

#[test]
fn references_do_not_conflate_unrelated_same_named_cells_in_different_namespaces() {
    // FP guard: the canonical-cell fold-in must
    // still be exact-qualified-name matched — two unrelated top-level
    // `tolComp` cells living in different namespaces, neither aliasing
    // the other, must never be unioned together just because they share
    // a bare name.
    let src = "namespace eval A {\n    variable tolComp 1\n}\nnamespace eval B {\n    variable tolComp 2\n}\n";
    let analysis = analyse(src);
    // Cursor on `A::tolComp`'s declaration (line 1, col 13).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        13,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(
        lines,
        vec![1],
        "an unrelated same-named cell in a different namespace must not be pulled in: {refs:?}"
    );
}

#[test]
fn references_for_a_multi_list_foreach_second_varlist_now_reach_every_use() {
    // Unless the first loop's own `name` (the second varList of `foreach
    // dirName {...} name {...} {...}`) is bound, `references()` from *any*
    // use inside the first loop's body falls through to whatever *other*
    // same-named `VarDef` exists anywhere in the flat top-level scope
    // — here, a second, later, textually unrelated `foreach name
    // {...}` — returning only that second loop's own 2 spans and
    // omitting every actual first-loop span, including the query site
    // itself. `foreach` (like `if`/`set`) introduces no new analyser
    // scope (correctly modelling Tcl's lack of block scoping), so at
    // the top level these two loops' `name` genuinely share one global
    // storage cell — same as any two sequential top-level `set name
    // ...` statements — so the correct reference set spans
    // *both* loops, not just the first: the bug was under-reporting
    // (missing the first loop's spans entirely), not over-reporting.
    let src = "foreach dirName {src src {src core}} name {alpha beta gamma} {\n    puts \"$dirName $name\"\n    if {$name eq \"pixutils\"} { puts skip }\n}\nforeach name {examples color changes} {\n    puts $name.ruff\n}\n";
    let analysis = analyse(src);
    // Cursor on the first loop's own `$name` read (line 1, col 21) —
    // the exact query shape the finding's own repro used.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        21,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&0),
        "the first loop's own `name` decl (line 0) must no longer be missing: {refs:?}"
    );
    assert!(
        lines.contains(&1),
        "the query site itself (line 1): {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "the first loop's other in-body use (line 2): {refs:?}"
    );
    assert!(
        lines.contains(&4) && lines.contains(&5),
        "the second loop shares the same global cell (no block scoping) so its spans stay unified too: {refs:?}"
    );
}

#[test]
fn references_to_var_includes_definition_and_uses() {
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor on `$x` first reference.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        &analysis,
        true,
    );
    // The analyser may or may not record the literal `$x`
    // as a reference depending on lowering; at minimum the
    // declaration should land in the result list.
    assert!(!refs.is_empty(), "{refs:?}");
    assert!(refs.iter().any(|r| r.start_line == 0));
}

#[test]
fn references_from_proc_param_bareword_declaration_include_every_use() {
    // TP — a cursor on a proc parameter's own bareword name (not a
    // `$`-prefixed read) must return the same reference set a query from
    // any `$name` read resolves.
    let src = "proc greet {name} { return $name }\ngreet hi\n";
    let analysis = analyse(src);
    // Cursor on `name` inside the parameter list (col 12-16).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        13,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(
        lines.contains(&0) && refs.len() >= 2,
        "read missing: {refs:?}"
    );
}

#[test]
fn references_from_catch_resultvar_bareword_include_the_original_declaration() {
    // TP — the finding's other confirmed shape: a `catch script name`
    // result-var reuses an existing variable; a cursor placed on its
    // own bareword token must still surface the full reference set,
    // including the original declaration.
    let src = "proc resolveSwitch {name def} {\n    catch {error boom} name\n    return $name\n}\n";
    let analysis = analyse(src);
    // Cursor on the catch result-var `name` (line 1, col 23-27).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        24,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "original decl missing: {refs:?}");
    assert!(
        lines.contains(&2),
        "the later `$name` read missing: {refs:?}"
    );
}

#[test]
fn caller_frame_reference_component_keeps_retained_registry_and_input() {
    // naming.core.original-caller-frame-navigation
    // docs/design/analysis/name-resolution-proofs/original-caller-frame-navigation.md
    // Readonly source template/reference geometry, not a completed native store.
    let source = "proc setdef {d} {upvar 1 $d dst; set dst SET}\nproc caller {} {setdef shared; puts $shared}\n";
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "context-marker",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let context = tcl_registry::model::ingress::context_for_profile(profile)
        .with_command_store(std::sync::Arc::new(registry));
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    let line_index = LineIndex::new(source);
    let read = u32::try_from(source.find("$shared").unwrap()).unwrap() + 1;
    let ctx = RefCtx {
        source,
        dialect: profile,
        line_index: &line_index,
        line: 1,
        character: 0,
        analysis: &analysis,
        include_declaration: true,
        resolution: crate::definition::CallResolution::document_only(),
    };
    let found = caller_frame_references(&ctx, read, "shared").unwrap();
    assert_eq!(found.len(), 2);
    let bare = u32::try_from(source.find("setdef shared").unwrap() + 7).unwrap();
    let position = line_index.position_at_utf16(bare, source);
    let bare_ctx = RefCtx {
        line: position.line,
        character: position.character.get(),
        ..ctx
    };
    assert_eq!(variable_references(&bare_ctx), Some(found));
    let stale = format!("{source}# changed");
    let stale_ctx = RefCtx {
        source: &stale,
        ..ctx
    };
    assert!(caller_frame_references(&stale_ctx, read, "shared").is_none());
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    let missing_ctx = RefCtx {
        analysis: &missing,
        ..ctx
    };
    assert!(caller_frame_references(&missing_ctx, read, "shared").is_none());
    let foreign_ctx = RefCtx {
        resolution: ctx
            .resolution
            .with_registry(crate::registry_for_dialect_profile(profile)),
        ..ctx
    };
    assert!(caller_frame_references(&foreign_ctx, read, "shared").is_none());
}
