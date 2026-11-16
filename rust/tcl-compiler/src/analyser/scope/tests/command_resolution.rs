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

//! The namespace a command resolves in and the settlement of each call against local, global and deleted definitions.

use super::*;

#[test]
fn command_resolution_namespace_global_returns_root() {
    let a = Analyser::new();
    assert_eq!(a.command_resolution_namespace(&[]), "::");
}

#[test]
fn command_resolution_namespace_single_relative() {
    let a = make_analyser_with_namespace("ns1");
    assert_eq!(a.command_resolution_namespace(&[0]), "::ns1");
}

#[test]
fn command_resolution_namespace_absolute_rebases() {
    let mut a = make_analyser_with_namespace("outer");
    // Add an absolute child namespace.
    a.result.global_scope.children[0]
        .children
        .push(Scope::new(ScopeKind::Namespace, "::abs"));
    // Path [0, 0] should rebase at ::abs, not nest under outer.
    assert_eq!(a.command_resolution_namespace(&[0, 0]), "::abs");
}

/// A proc scope resolves in its own **defining** namespace, so a
/// qualified-name proc declared at the top level still answers `::ns1`
/// (a purely lexical walk would answer `::`).
#[test]
fn command_resolution_namespace_uses_a_procs_defining_namespace() {
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Proc, "::ns1::myproc"));
    assert_eq!(a.command_resolution_namespace(&[0]), "::ns1");
}

#[test]
fn command_resolution_namespace_nested_proc_keeps_the_enclosing_namespace() {
    let mut a = Analyser::new();
    let mut ns = Scope::new(ScopeKind::Namespace, "ns1");
    ns.children
        .push(Scope::new(ScopeKind::Proc, "::ns1::myproc"));
    a.result.global_scope.children.push(ns);
    assert_eq!(a.command_resolution_namespace(&[0, 0]), "::ns1");
}

/// The Method-scope namespace rule: a `TclOO` method (flagged) resolves
/// globally; a snit/itcl method (unflagged) resolves in the class's
/// defining namespace, like a proc.
#[test]
fn method_scope_namespace_honours_oo_global_resolution() {
    let flagged = {
        let mut m = Scope::new(ScopeKind::Method, "::pkg::C::m");
        m.oo_global_resolution = true;
        m
    };
    assert_eq!(
        advance_command_resolution_namespace("::pkg", &flagged),
        "::"
    );
    let unflagged = Scope::new(ScopeKind::Method, "::pkg::C::m");
    assert_eq!(
        advance_command_resolution_namespace("::pkg", &unflagged),
        "::pkg::C",
    );
}

/// tclsh 8.6.16 / 9.0.4-pinned (G1 shape): a bare call in a `TclOO`
/// method body dispatches the GLOBAL helper, not the class's
/// defining-namespace helper — method bodies run in the object's
/// namespace and never search the class's namespace.
#[test]
fn tcloo_method_call_settles_to_the_global_helper() {
    let src = "namespace eval ::pkg {\n\
                       proc helper {} { return PKGHELPER }\n\
                       oo::class create C { method m {} { helper } }\n\
                   }\n\
                   proc ::helper {} { return GLOBALHELPER }\n";
    let mut a = Analyser::new();
    let analysis = a.analyse(src, "tcl8.6");
    let inv = analysis
        .command_invocations
        .iter()
        .find(|i| i.name == "helper")
        .expect("method-body helper invocation recorded");
    assert_eq!(
        inv.resolved_qualified_name.as_deref(),
        Some("::helper"),
        "TclOO method bodies resolve bare calls globally (tclsh-pinned)",
    );
}

/// A builtin renamed away is not callable under its original name
/// (tclsh-pinned: C raises `invalid command name`), so settlement must
/// not demote a namespace-local guess to the dead builtin name.
#[test]
fn renamed_away_builtin_no_longer_settles_calls() {
    let src = "rename puts ::a::p\n\
                   namespace eval ::ns { puts x }\n";
    let mut a = Analyser::new();
    let analysis = a.analyse(src, "tcl8.6");
    // `rename`'s own OLD argument (`puts`, line 0) is now itself a
    // recorded, correctly-resolved-to-the-still-live-builtin reference
    // — an earlier "puts" entry this test must not
    // mistake for the *call* site (`puts x`, line 1) it actually means
    // to check. The call site is always the *last* "puts"-named
    // invocation in source order.
    let inv = analysis
        .command_invocations
        .iter()
        .rev()
        .find(|i| i.name == "puts")
        .expect("namespaced puts invocation recorded");
    assert_eq!(
        inv.resolved_qualified_name.as_deref(),
        Some("::ns::puts"),
        "the dead builtin name must not win; the local-first guess stays",
    );
}

// finalise_invocation_resolutions — Tcl's existence-checked two-step
// resolution rule, applied post-walk.  Every case below is pinned
// against tclsh 8.6: the local-first candidate wins
// only when that command exists by the end of the file; otherwise the
// call falls back to the global candidate.

/// The `resolved_qualified_name` recorded for the invocation whose raw
/// call text is `name`.
fn resolved_for(source: &str, name: &str) -> Option<String> {
    let mut a = Analyser::new();
    let analysis = a.analyse(source, "tcl8.6");
    analysis
        .command_invocations
        .iter()
        .find(|inv| inv.name == name)
        .unwrap_or_else(|| panic!("no `{name}` invocation recorded"))
        .resolved_qualified_name
        .clone()
}

#[test]
fn relative_qualified_call_falls_back_to_global_when_local_absent() {
    // tclsh8.6: `inner::p` inside `outer` dispatches ::inner::p when
    // ::outer::inner::p does not exist.
    let src = "namespace eval ::inner {}\nproc ::inner::p {} {}\nnamespace eval outer { proc caller {} { inner::p } }\n";
    assert_eq!(resolved_for(src, "inner::p").as_deref(), Some("::inner::p"));
}

#[test]
fn relative_qualified_call_prefers_local_defined_later_in_file() {
    // tclsh8.6: a proc body's calls resolve at call time, so the local
    // candidate wins even when its `proc` runs after the caller was
    // defined — the walk order must not leak into the resolution.
    let src = concat!(
        "namespace eval ::inner {}\n",
        "proc ::inner::p {} {}\n",
        "namespace eval outer { proc caller {} { inner::p } }\n",
        "namespace eval outer { namespace eval inner {} ; proc inner::p {} {} }\n",
    );
    assert_eq!(
        resolved_for(src, "inner::p").as_deref(),
        Some("::outer::inner::p")
    );
}

#[test]
fn bare_call_falls_back_to_global_when_local_absent() {
    let src = "proc ::g {} {}\nnamespace eval ns2 { proc caller {} { g } }\n";
    assert_eq!(resolved_for(src, "g").as_deref(), Some("::g"));
}

#[test]
fn bare_call_prefers_local_defined_later_in_file() {
    let src = concat!(
        "proc ::g {} {}\n",
        "namespace eval ns2 { proc caller {} { g } }\n",
        "namespace eval ns2 { proc g {} {} }\n",
    );
    assert_eq!(resolved_for(src, "g").as_deref(), Some("::ns2::g"));
}

#[test]
fn builtin_call_inside_namespace_resolves_global() {
    // tclsh8.6: `puts` inside a namespace is the global builtin — the
    // local-first guess `::ns3::puts` names nothing, so the settled
    // resolution is `::puts` (as it was before the scope-aware walk).
    let src = "namespace eval ns3 { proc caller {} { puts hi } }\n";
    assert_eq!(resolved_for(src, "puts").as_deref(), Some("::puts"));
}

#[test]
fn builtin_shadowed_by_local_proc_stays_local() {
    // tclsh8.6: a namespace-local `puts` shadows the builtin for
    // unqualified calls from inside that namespace.
    let src = "namespace eval ns3 { proc puts {msg} {} ; proc caller {} { puts hi } }\n";
    assert_eq!(resolved_for(src, "puts").as_deref(), Some("::ns3::puts"));
}

#[test]
fn unknown_in_both_namespaces_keeps_local_first_guess() {
    // Neither ::ns4::mystery::call nor ::mystery::call is defined in
    // this file (a cross-file proc, or a typo W123 reports) — keep the
    // local-first candidate so cross-file consumers still see the
    // Tcl-priority guess.
    let src = "namespace eval ns4 { proc caller {} { mystery::call } }\n";
    assert_eq!(
        resolved_for(src, "mystery::call").as_deref(),
        Some("::ns4::mystery::call")
    );
}

// `finalise_invocation_resolutions`'s local-vs-global candidate choice.
// A "known" predicate that compared only the final deletion offset against
// the candidate's own establishing offset, never the call site, would make
// a namespaced local call textually *before* a later unconditional
// deletion wrongly lose to the global candidate. Confirmed against
// tclsh 8.6.14 throughout.

#[test]
fn local_call_before_later_deletion_resolves_local_not_global_codex_1009() {
    // TP: `foo::caller`'s own top-level
    // invocation runs before `rename foo::bar {}`, so its `bar` call
    // must still resolve to the local `::foo::bar`, not the global
    // `::bar` — confirmed against tclsh 8.6.14 (prints "local").
    let mut a = Analyser::new();
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n}\nfoo::caller\nrename foo::bar {}\n";
    let r = a.analyse(src, "tcl8.6");
    let bar_call = r
        .command_invocations
        .iter()
        .find(|i| i.name == "bar" && !i.indirect)
        .expect("bar call recorded");
    assert_eq!(
        bar_call.resolved_qualified_name.as_deref(),
        Some("::foo::bar"),
        "a call before the deletion must still resolve to the local candidate"
    );
    assert!(
        bar_call.resolved_user_definition,
        "the position-sensitive winner is a live user definition at this call"
    );
}

#[test]
fn local_call_after_deletion_still_falls_back_to_global_issue_973() {
    // FN guard: unlike the case above, `foo::caller` is
    // only ever invoked (at the top level) *after* `rename foo::bar
    // {}` runs, so the local `bar` is genuinely gone by the time the
    // call executes — it must still fall back to the global `::bar`.
    let mut a = Analyser::new();
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    rename foo::bar {}\n    proc caller {} { return [bar] }\n}\nfoo::caller\n";
    let r = a.analyse(src, "tcl8.6");
    let bar_call = r
        .command_invocations
        .iter()
        .find(|i| i.name == "bar" && !i.indirect)
        .expect("bar call recorded");
    assert_eq!(
        bar_call.resolved_qualified_name.as_deref(),
        Some("::bar"),
        "a call genuinely after the deletion must fall back to the global candidate"
    );
    assert!(
        bar_call.resolved_user_definition,
        "the global fallback is itself a live user definition"
    );
}

#[test]
fn escape_hatch_requires_the_specific_enclosing_definitions_own_top_level_call() {
    // FN guard: an unrelated proc's top-level invocation elsewhere in
    // the file must not "lend" liveness to a different enclosing
    // definition that is itself never invoked — the escape hatch must
    // key off the *specific* body's own top-level call, not just
    // "some call happened somewhere before the deletion". `foo::bar`
    // is deleted, and `foo::caller` (the only body that calls it) is
    // never invoked anywhere — only the unrelated `baz::unrelated` is
    // — so the `bar` call must fall back to the global `::bar`.
    let mut a = Analyser::new();
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n}\nnamespace eval baz {\n    proc unrelated {} { return 1 }\n}\nbaz::unrelated\nrename foo::bar {}\n";
    let r = a.analyse(src, "tcl8.6");
    let bar_call = r
        .command_invocations
        .iter()
        .find(|i| i.name == "bar" && !i.indirect)
        .expect("bar call recorded");
    assert_eq!(
        bar_call.resolved_qualified_name.as_deref(),
        Some("::bar"),
        "foo::caller is never invoked, so the escape hatch must not apply"
    );
}

#[test]
fn escape_hatch_follows_a_chain_of_enclosing_definitions_issue_1015() {
    // FP guard: `foo::caller` is never invoked at the top
    // level — only `foo::entry` is — but `entry` calls `caller`, which
    // calls `bar`, all before the rename, so the local `::foo::bar`
    // stays the resolution. tclsh8.6/9.0 confirm the chain runs clean.
    let mut a = Analyser::new();
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n    proc entry {} { return [caller] }\n}\nfoo::entry\nrename foo::bar {}\n";
    let r = a.analyse(src, "tcl8.6");
    let bar_call = r
        .command_invocations
        .iter()
        .find(|i| i.name == "bar" && !i.indirect)
        .expect("bar call recorded");
    assert_eq!(
        bar_call.resolved_qualified_name.as_deref(),
        Some("::foo::bar"),
        "foo::entry reaches foo::caller, which reaches bar, before the rename"
    );
}

#[test]
fn escape_hatch_terminates_on_a_never_entered_mutual_recursion_cycle_issue_1015() {
    // TP guard: `foo::ping` and `foo::pong` call each
    // other and nothing calls either, so neither is ever reached — the
    // reachability fixpoint must terminate and leave the escape hatch
    // shut, falling the `bar` call back to the global `::bar`.
    let mut a = Analyser::new();
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc ping {} { pong\n        return [bar] }\n    proc pong {} { return [ping] }\n}\nrename foo::bar {}\n";
    let r = a.analyse(src, "tcl8.6");
    let bar_call = r
        .command_invocations
        .iter()
        .find(|i| i.name == "bar" && !i.indirect)
        .expect("bar call recorded");
    assert_eq!(
        bar_call.resolved_qualified_name.as_deref(),
        Some("::bar"),
        "neither cycle member is ever reached, so the escape hatch must not apply"
    );
}

#[test]
fn navigation_uses_reached_body_definition_before_later_redefinition() {
    let source = "proc p {} {return FIRST}; proc run {} {p}; run; proc p {x} {return SECOND}";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let result = Analyser::new().analyse(source, dialect);
        let call = result
            .command_invocations
            .iter()
            .find(|invocation| invocation.name == "p")
            .unwrap();
        let definition = call.resolved_definition.as_ref().expect(dialect);
        let allocation = definition.allocation();
        assert_eq!(allocation.site.offset, 0, "{dialect}");
        let declaration = result
            .proc_for_definition(definition, source)
            .unwrap_or_else(|| {
                panic!("{dialect}: retained definition {definition:?} has no authored declaration")
            });
        assert!(declaration.params.is_empty(), "{dialect}");
        assert_eq!(
            result
                .proc_def_in_effect_at("::p", call.range.start())
                .unwrap()
                .name_span,
            declaration.name_span,
            "{dialect}"
        );
        assert!(
            result
                .proc_for_definition(definition, "proc p {x} {return WRONG}")
                .is_none()
        );
    }
}

#[test]
fn definition_navigation_uses_retained_grammar_instead_of_display_dialect() {
    let source = "proc p {} {return FIRST}; p";
    let mut result = Analyser::new().analyse(source, "tcl8.4");
    let definition = result
        .command_invocations
        .iter()
        .find(|invocation| invocation.name == "p")
        .unwrap()
        .resolved_definition
        .clone()
        .unwrap();
    assert!(!result.body_lexer_config.unwrap().expand_syntax);
    result.dialect = "custom-display-name".to_owned();
    assert!(result.proc_for_definition(&definition, source).is_some());
    result.body_lexer_config = None;
    assert!(result.proc_for_definition(&definition, source).is_none());
}

#[test]
fn class_navigation_retains_selected_declaration_after_name_reuse() {
    let source = "oo::class create C {}; C create nav_before; rename C {}; oo::class create C {}; C create nav_after";
    let call_offsets = ["C create nav_before", "C create nav_after"]
        .map(|call| u32::try_from(source.find(call).unwrap()).unwrap());
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = Analyser::new().analyse(source, dialect);
        let calls = result
            .command_invocations
            .iter()
            .filter(|invocation| {
                invocation.name == "C" && call_offsets.contains(&invocation.range.start())
            })
            .collect::<Vec<_>>();
        assert_eq!(calls.len(), 2, "{dialect}");
        let declarations = calls
            .iter()
            .map(|invocation| {
                result
                    .class_for_definition(invocation.resolved_definition.as_ref().unwrap(), source)
                    .unwrap()
                    .name_span
            })
            .collect::<Vec<_>>();
        assert_ne!(declarations[0], declarations[1], "{dialect}");
        assert_eq!(result.superseded_classes["::C"].len(), 1);
        assert_eq!(result.all_classes["::C"].name_span, declarations[1]);
    }
}
