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

//! `my method` and self-dispatch call sites inside member bodies.

use super::*;

// class-member references

#[test]
fn idx63_two_block_class_my_dispatch_already_fixed_by_idx52() {
    // Go-to-definition and find-references must both answer for a `my
    // methodName` call when the class is created via `oo::class create`
    // with no body and every method (including the call site itself) is
    // added via a *separate*, later `oo::define ClassName { ... }` block —
    // the real corpus's `ticklecharts::chart` shape.  The load-bearing
    // machinery is `class_body_spans` / `enclosing_class_at`, pinned here
    // from the `my`-dispatch direction.
    let src = "oo::class create foo::widget {\n    variable _x\n    constructor {} { set _x 0 }\n}\noo::define foo::widget {\n    method bar {} { return \"bar-value\" }\n    method baz {} { return [my bar] }\n}\nputs [[foo::widget new] baz]\n";
    let analysis = analyse(src);
    // `definition` at the `my bar` call site (line 6, col 31).
    let locs = crate::definition::definition(src, 6, 31, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 5,
        "must resolve to method bar's declaration"
    );
    // `references` from `bar`'s declaration (line 5, col 11).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&5), "decl missing: {refs:?}");
    assert!(
        lines.contains(&6),
        "the my bar call site must be reachable from the declaration: {refs:?}"
    );
}

#[test]
fn references_for_method_includes_decl_and_call_sites() {
    // Intra-class dispatch uses `my <method>` (the dispatch form tclsh
    // accepts; a bare `greet` head errors with "invalid command name", so it
    // is not a reference).
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1, col 11).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    assert!(refs.len() >= 3, "expected ≥3 refs; got {refs:?}");
}

#[test]
fn references_for_method_reach_a_my_dispatch_call_inside_a_switch_arm() {
    // A `my methodName` call written inside a `switch` arm body is a
    // genuine, statically-known call site (tclsh9.0/8.6-verified) —
    // the real corpus shape (`ticklecharts::chart`'s `Add` dispatcher:
    // `switch ... { barSeries { my AddBarSeries {*}$args } ... }`).
    // `scan_my_method_region`'s `[...]`-substitution recursion alone never
    // reaches a switch arm's braced body (it isn't a command
    // substitution), which would leave this invisible to find-references even
    // though go-to-definition (an independent cursor-token walk)
    // already resolved it.
    let src = "oo::class create widget {\n    method bar {} { return \"bar-value\" }\n    method dispatch {args} {\n        switch -exact -- [lindex $args 0] {\n            bar { my bar {*}[lrange $args 1 end] }\n        }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `bar` declaration (line 1, col 11).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(
        lines.contains(&4),
        "the my bar call site inside the switch arm is missing: {refs:?}"
    );
}

#[test]
fn references_for_method_excludes_decl_when_requested() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        false,
    );
    // Only the two call sites — the declaration is
    // excluded when include_declaration=false.
    assert_eq!(refs.len(), 2, "{refs:?}");
}

#[test]
fn references_from_decl_reach_my_dispatch_when_class_extended_via_separate_oo_define() {
    // `Gadget` is created via `oo::class create` with no body; every method
    // (including the `my Helper` call site) is added via a *separate*,
    // later `oo::define Gadget { ... }` block — the real corpus shape
    // (`ticklecharts::chart`). References from the `Helper` declaration
    // must reach the `my Helper` call site living in that separate
    // block, not silently return nothing.
    let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `Helper` declaration (line 4, col 11).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&4), "decl missing: {refs:?}");
    assert!(
        lines.contains(&5),
        "the my Helper call site inside the separate oo::define block is missing: {refs:?}"
    );
}

#[test]
fn references_from_decl_reach_a_self_bracket_dispatch_call_site() {
    // `[self] m` is TclOO's own same-object dispatch
    // idiom — reaches the enclosing class exactly like `my m`, but
    // through a bracketed command substitution. References from the
    // declaration must reach it, not silently return only the
    // declaration itself.
    let src = "oo::class create C {\n    method animTick {} { return 1 }\n    method anim {} { [self] animTick }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `animTick` declaration (line 1, col 11).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(
        lines.contains(&2),
        "the [self] animTick call site is missing: {refs:?}"
    );
}

#[test]
fn method_dispatch_scan_uses_retained_custom_schema_and_script_purpose() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    fn reference_only(
        _args: tcl_registry::InvocationArguments<'_>,
    ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
        vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
    }
    let source = "oo::class create C {method m {} {}; method run {} {custom-body {my m}; reference-script {my m}}}";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "custom-body",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, tcl_registry::ArgRole::Body)],
        ..tcl_registry::CommandSpec::DEFAULT
    });
    registry.insert(tcl_registry::CommandSpec {
        name: "reference-script",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, tcl_registry::ArgRole::Body)],
        script_timing_resolver: Some(reference_only),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let context = tcl_registry::model::context_for_profile(profile)
        .with_command_store(std::sync::Arc::new(registry));
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let mut analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    assert!(analysis.allows_retained_logical_declaration_advice());
    let body = analysis.all_classes["::C"].methods["run"].body_span;
    let sites = scan_my_method_sites(source, &analysis, &[body], "m", None);
    assert_eq!(
        sites.len(),
        1,
        "custom schema is retained; reference-only script is excluded"
    );
    assert_eq!(sites[0].start() as usize, source.find("my m").unwrap() + 3);
    let stale = source.replace("custom-body", "custom-bodz");
    assert_eq!(
        scan_my_method_sites(&stale, &analysis, &[body], "m", None),
        [] as [tcl_lexer::Span; 0]
    );
    analysis.resolved_input = None;
    assert_eq!(
        scan_my_method_sites(source, &analysis, &[body], "m", None),
        [] as [tcl_lexer::Span; 0]
    );
}
