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

//! References through ensemble subcommand dispatch.

use super::*;

#[test]
fn ensemble_subcommand_references_include_decl_and_both_call_sites() {
    // TP — references on an ensemble subcommand call
    // site must return the target proc's declaration plus every call
    // site — proves the automatic pickup via
    // `proc_reference_spans`/`invocation_references_named`'s
    // `resolved_qualified_name` matching, not just a single hardcoded
    // case.
    let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e foo bar]\nputs [e foo baz]\n";
    let analysis = analyse(src);
    // Cursor on "foo" in the first call site (0-based line 7, col 8).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        7,
        8,
        &analysis,
        true,
    );
    // decl + the `-map`'s own target-text reference (line 2, pre-existing
    // — needed so renaming the proc also updates the map entry) + both
    // nested-`[...]` call sites.
    assert_eq!(
        refs.len(),
        4,
        "expected decl + map entry + 2 call sites: {refs:?}"
    );
    // References come back in source order, so the `-map` entry precedes
    // the declaration.
    assert!(
        refs.iter().any(|r| r.start_line == 5),
        "declaration: {refs:?}"
    );
    assert!(
        refs.iter().any(|r| r.start_line == 2),
        "expected the -map target-text reference: {refs:?}",
    );
    assert!(
        refs.iter().any(|r| r.start_line == 7) && refs.iter().any(|r| r.start_line == 8),
        "expected both call sites: {refs:?}",
    );
}

/// The analyser's own dispatch recording resolves the
/// subcommand the way the ensemble does, so an **abbreviated** call site
/// is a reference too. Oracle (tclsh 8.6.16 / 9.0.4): with `-map {foo
/// ::e::Foo}` and the default `-prefixes 1`, `e fo` returns `foo: bar`.
///
/// The abbreviated site sits inside a proc body so the deferred replay
/// path (`flush_pending_ensemble_subcommand_invocations`) resolves it,
/// not only the direct one.
#[test]
fn ensemble_abbreviated_dispatch_is_a_reference_to_the_target() {
    let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\nproc caller {} {\n    return [e fo bar]\n}\n";
    let analysis = analyse(src);
    // Cursor on the declaration of ::e::Foo (0-based line 5).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        11,
        &analysis,
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 7),
        "expected the abbreviated call site inside the proc body: {refs:?}",
    );
}

/// The `-prefixes 0` counterpart: prefix matching is off, so the
/// abbreviated word is a plain unknown subcommand and must NOT be
/// recorded as a dispatch reference. Oracle: `g fo` errors
/// `unknown subcommand "fo": must be foo`.
#[test]
fn prefixless_ensemble_abbreviation_is_not_a_reference() {
    let src = "namespace eval ::g {\n    namespace ensemble create -map {\n        foo ::g::Foo\n    } -prefixes 0\n}\nproc ::g::Foo {args} { return G }\nproc caller {} {\n    return [g fo bar]\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        11,
        &analysis,
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 7),
        "a -prefixes 0 ensemble must not resolve the abbreviation: {refs:?}",
    );
}

/// The ensemble is created by
/// `namespace ensemble create -map` inside a proc declared with a
/// fully-qualified name at top level, with no enclosing `namespace eval`,
/// so it homes to `::app::widget`. Both reference directions must reach
/// the dispatch call site: from the mapped target's declaration, and from
/// the call site itself.
///
/// Oracle (tclsh 8.6.16 and 9.0.4, identical): the script prints `shown`,
/// so `::app::widget show` really does dispatch to `::app::widget::Show`.
///
/// Go-to-definition already answered this (it resolves on demand against
/// the finished analysis); references enumerate recorded invocations, and
/// the invocation was missing. The analyser fix is what supplies it — this
/// pins that the reference provider consumes it from both cursor
/// positions and returns the *same* answer for each.
#[test]
fn ensemble_subcommand_references_reach_the_call_site_through_a_qualified_proc_923_idx85() {
    let src = "namespace eval ::app::widget {\n    variable state 0\n}\n\
                   proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map {\n        \
                   show      ::app::widget::Show\n    }\n}\n\
                   proc ::app::widget::Show {} { puts \"shown\" }\n\
                   ::app::widget::Setup\n\
                   puts [::app::widget show]\n";
    let analysis = analyse(src);
    // Line 8 is `proc ::app::widget::Show …`; `Show` starts at column 20.
    let from_decl = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        8,
        21,
        &analysis,
        true,
    );
    // Line 10 is `puts [::app::widget show]`; `show` starts at column 20.
    let from_call = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        10,
        21,
        &analysis,
        true,
    );
    let lines = |refs: &[LspRange]| -> Vec<u32> {
        let mut l: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        l.sort_unstable();
        l
    };
    assert!(
        lines(&from_decl).contains(&10),
        "the `::app::widget show` dispatch is missing from the declaration's \
             reference set: {from_decl:?}",
    );
    assert!(
        lines(&from_decl).contains(&8),
        "the declaration itself is missing: {from_decl:?}",
    );
    assert_eq!(
        lines(&from_call),
        lines(&from_decl),
        "both query directions must agree: {from_call:?} vs {from_decl:?}",
    );
}

/// TN for the above — a `-map` built by `[list …]` is not a literal
/// mapping, so no `show -> ::app::widget::Show` fact exists and the
/// dispatch site must stay unattributed rather than guessed at.
#[test]
fn a_dynamic_ensemble_map_yields_no_call_site_reference_923_idx85() {
    let src = "proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map [list show ::app::widget::Show]\n}\n\
                   proc ::app::widget::Show {} {}\n\
                   ::app::widget::Setup\n\
                   ::app::widget show\n";
    let analysis = analyse(src);
    // Cursor on `Show` in its declaration (line 3, column 20).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        21,
        &analysis,
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 5),
        "a dynamic -map must not manufacture a navigation edge: {refs:?}",
    );
}
