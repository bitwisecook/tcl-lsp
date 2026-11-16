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
use tcl_compiler::analyser::Analyser;

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

#[test]
fn native_unresolved_source_heads_keep_actual_body_and_ignore_report_mutation() {
    let source = "proc owner {} {remote V; remote W; set data {inert COMMAND}}";
    let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let item = prepare(source, 0, 5, &analysis)
        .pop()
        .expect("original owner header");
    analysis.command_invocations.clear();
    analysis.all_procs.clear();
    let calls = unresolved_outgoing_calls(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &item,
        &analysis,
    );
    let remote = calls.iter().find(|call| call.name == "remote").unwrap();
    assert_eq!(remote.from_ranges.len(), 2);
    assert!(calls.iter().all(|call| call.name != "inert"));
    assert!(remote.resolved_qualified_name.is_none());
    assert!(
        remote
            .logical_occurrences
            .iter()
            .all(|call| call.candidates.is_none())
    );
    let independent = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let borrowed =
        tcl_compiler::registry_invocation::source_structure::source_native_procedure_body_commands(
            source, &analysis,
        )
        .unwrap();
    assert!(
        borrowed
            .iter()
            .all(|command| command.caller_declaration(&independent).is_none())
    );
    assert_eq!(
        unresolved_outgoing_calls(
            &source.replace("COMMAND", "CHANGED"),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &item,
            &analysis
        ),
        [] as [crate::call_hierarchy::UnresolvedOutgoingCall; 0]
    );
}

#[test]
fn original_hierarchy_preparation_keeps_variable_cursors_and_method_declarations_separate() {
    // Implementation contract: naming.consumer.original-call-hierarchy
    // docs/design/analysis/name-resolution-proofs/original-call-hierarchy.md
    let source = "oo::class create C {method x {} {set x 1; puts $x}}";
    let mut analysis = analyse(source);
    let variable = u32::try_from(source.find("$x").unwrap() + 1).unwrap();
    assert!(!analysis.allows_lexical_declaration_advice());
    assert!(crate::definition::original_variable_cursor_retained(
        source, &analysis, 0, variable
    ));
    assert!(
        analysis
            .all_classes
            .values()
            .any(|class| class.methods.contains_key("x"))
    );
    assert_eq!(
        prepare(source, 0, variable, &analysis),
        [] as [crate::call_hierarchy::CallHierarchyItem; 0]
    );
    let declaration = u32::try_from(source.find("method x").unwrap() + "method ".len()).unwrap();
    analysis.all_classes.clear();
    analysis.global_scope.classes.clear();
    let items = prepare(source, 0, declaration, &analysis);
    assert_eq!(items.len(), 1);
    let identity = items[0]
        .identity
        .as_ref()
        .expect("genuine source declaration identity");
    assert_eq!(identity.span().start(), declaration);
    assert!(matches!(
        identity.role(),
        crate::original_declaration::OriginalDeclarationRole::Method(_)
    ));
    assert_eq!(
        prepare(&format!("#{source}"), 0, declaration, &analysis),
        [] as [crate::call_hierarchy::CallHierarchyItem; 0]
    );
}

#[test]
fn prepare_resolves_proc_at_cursor() {
    let src = "proc greet {} {}\n";
    let analysis = analyse(src);
    let items = prepare(src, 0, 6, &analysis);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "greet");
}

#[test]
fn prepare_returns_empty_for_unknown_word() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert_eq!(
        prepare(src, 0, 6, &analysis),
        [] as [crate::call_hierarchy::CallHierarchyItem; 0]
    );
}

#[test]
fn same_named_procs_in_different_namespaces_disambiguate_by_span() {
    // `::a::helper` and `::b::helper` share the short display name
    // `helper`; the call-hierarchy item must resolve to the definition at
    // its own `selectionRange`, not whichever `all_procs` entry hashes
    // first. Each helper calls a distinct callee so the wrong resolution
    // is observable. tclsh rejects `proc` into a missing namespace, so
    // line 0 creates both.
    let src = "namespace eval a {}; namespace eval b {}; proc ::a::helper {} { aCallee }\n\
                   proc ::b::helper {} { bCallee }\n\
                   proc aCallee {} {}\n\
                   proc bCallee {} {}\n";
    let analysis = analyse(src);
    // Cursor on the `helper` of the second (`::b::helper`) definition —
    // line 1, inside `helper` (after the `proc ::b::` prefix, 10 chars).
    let items = prepare(src, 1, 12, &analysis);
    assert_eq!(items.len(), 1, "{items:?}");
    let outgoing = outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &items[0],
        &analysis,
    );
    let callees: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
    assert!(
        callees.contains(&"bCallee"),
        "b::helper must resolve to its own body (bCallee); got {callees:?}"
    );
    assert!(
        !callees.contains(&"aCallee"),
        "b::helper must not pick up a::helper's callee; got {callees:?}"
    );
}

// incoming + outgoing calls

#[test]
fn incoming_calls_from_other_procs() {
    // `caller` calls `target`; `target` is being asked
    // for its incoming calls.
    let src = "proc target {} {}\nproc caller {} { target }\n";
    let analysis = analyse(src);
    let items = prepare(src, 0, 6, &analysis);
    let target = &items[0];
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        target,
        &analysis,
    );
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from.name, "caller");
    assert_eq!(incoming[0].from_ranges.len(), 1);
}

#[test]
fn consumed_command_names_remain_references_without_becoming_calls() {
    let source = "proc target {} {return yes}\nproc caller {} {info body target; target}\ncaller\n";
    let analysis = analyse(source);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let target = prepare(source, 0, 6, &analysis).remove(0);
    let caller = prepare(source, 1, 6, &analysis).remove(0);
    let references = analysis
        .command_invocations
        .iter()
        .filter(|invocation| invocation.name == "target")
        .collect::<Vec<_>>();
    assert_eq!(references.len(), 2);
    assert!(references.iter().any(|invocation| matches!(
        invocation.lookup,
        tcl_compiler::signature_scan::types::SignatureCommandLookup::ConsumedName { .. }
    )));
    let incoming = incoming_calls(source, dialect, &target, &analysis);
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].from_ranges.len(), 1);
    let outgoing = outgoing_calls(source, dialect, &caller, &analysis);
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].to.name, "target");
    assert_eq!(outgoing[0].from_ranges.len(), 1);
}

#[test]
fn deferred_command_prefixes_do_not_create_incoming_call_edges() {
    let source = "proc target args {}\nproc registrar {} {trace add command target delete target}\nregistrar\n";
    let analysis = analyse(source);
    let target = prepare(source, 0, 6, &analysis).remove(0);
    assert!(analysis.command_invocations.iter().any(|invocation| {
        invocation.name == "target"
            && invocation.lookup
                == tcl_compiler::signature_scan::types::SignatureCommandLookup::DeferredReference
    }));
    let incoming = incoming_calls(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &target,
        &analysis,
    );
    assert!(incoming.is_empty(), "{incoming:?}");
}

#[test]
fn incoming_calls_group_by_caller() {
    // Two call sites from the same caller bucket together
    // into a single IncomingCall entry.
    let src = "proc target {} {}\nproc caller {} { target\n target }\n";
    let analysis = analyse(src);
    let items = prepare(src, 0, 6, &analysis);
    let target = &items[0];
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        target,
        &analysis,
    );
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from_ranges.len(), 2);
}

#[test]
fn incoming_calls_top_level_bucketed_as_top_level() {
    let src = "proc target {} {}\ntarget\n";
    let analysis = analyse(src);
    let items = prepare(src, 0, 6, &analysis);
    let target = &items[0];
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        target,
        &analysis,
    );
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from.name, "<top-level>");
}

#[test]
fn outgoing_calls_from_proc_body() {
    // `caller` calls `target` and `other`; outgoing-calls
    // for `caller` should include both.
    let src = "proc target {} {}\nproc other {} {}\nproc caller {} { target\n other }\n";
    let analysis = analyse(src);
    let items = prepare(src, 2, 6, &analysis);
    assert_eq!(items[0].name, "caller");
    let outgoing = outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    let target_names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
    assert!(target_names.contains(&"target"), "{outgoing:?}");
    assert!(target_names.contains(&"other"), "{outgoing:?}");
}

#[test]
fn outgoing_calls_skip_builtins() {
    // `caller` calls `puts` (built-in) and `target`
    // (user proc).  Only `target` should appear in the
    // outgoing list.
    let src = "proc target {} {}\nproc caller {} { puts hi\n target }\n";
    let analysis = analyse(src);
    let items = prepare(src, 1, 6, &analysis);
    assert_eq!(items[0].name, "caller");
    let outgoing = outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
    assert_eq!(names, vec!["target"], "{outgoing:?}");
}

#[test]
fn outgoing_calls_empty_for_unknown_proc() {
    let src = "proc greet {} {}\n";
    let analysis = analyse(src);
    let bogus = CallHierarchyItem {
        identity: None,
        name: "::not_a_real_proc".to_string(),
        detail: None,
        range: LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 0,
        },
        selection_range: LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 0,
        },
    };
    assert_eq!(
        outgoing_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &bogus,
            &analysis
        ),
        [] as [crate::call_hierarchy::OutgoingCall; 0]
    );
}

#[test]
fn unresolved_outgoing_calls_lists_non_local_heads() {
    // `caller` calls `local` (defined here), `sibling` (would
    // live in another file) and `puts` (builtin).  Only the
    // heads not resolvable in this document come back; the
    // server filters those against the workspace index.
    let src = "proc local {} {}\nproc caller {} { local\n sibling\n puts hi }\n";
    let analysis = analyse(src);
    let items = prepare(src, 1, 6, &analysis);
    assert_eq!(items[0].name, "caller");
    let unresolved = unresolved_outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    let names: Vec<&str> = unresolved.iter().map(|u| u.name.as_str()).collect();
    assert!(names.contains(&"sibling"), "{unresolved:?}");
    assert!(names.contains(&"puts"), "{unresolved:?}");
    assert!(
        !names.contains(&"local"),
        "local resolves in-document: {unresolved:?}"
    );
}

#[test]
fn unresolved_outgoing_calls_groups_repeated_heads() {
    let src = "proc caller {} { sibling\n sibling }\n";
    let analysis = analyse(src);
    let items = prepare(src, 0, 6, &analysis);
    assert_eq!(items[0].name, "caller");
    let unresolved = unresolved_outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    let sibling = unresolved
        .iter()
        .find(|u| u.name == "sibling")
        .expect("sibling head present");
    assert_eq!(sibling.from_ranges.len(), 2, "{unresolved:?}");
}

// class methods

#[test]
fn prepare_resolves_method_at_cursor() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1, col 11).
    let items = prepare(src, 1, 11, &analysis);
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0].name, "::C::greet");
}

/// Regression: a bare `greet` call (no `my`) is not valid `TclOO`
/// dispatch — it errors "invalid command name" at runtime (confirmed
/// against tclsh 9.0.4 elsewhere in this crate, e.g.
/// `references::tests::fp_bare_head_is_not_a_call`) — so intra-class
/// incoming/outgoing method calls must match `my <method>`, never a
/// bare head.
#[test]
fn incoming_calls_for_method_grouped_by_caller_method() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    let items = prepare(src, 1, 11, &analysis);
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    // One caller method (`twice`) with two call ranges.
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from.name, "::C::twice");
    assert_eq!(incoming[0].from_ranges.len(), 2, "{incoming:?}");
}

#[test]
fn hierarchy_follows_registry_wrapped_my_callback() {
    let src = "oo::class create C {\n    method read {} {}\n    unexport read\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my read]]\n    }\n}\n";
    let analysis = analyse(src);
    let items = prepare(src, 1, 11, &analysis);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let incoming = incoming_calls(src, dialect, &items[0], &analysis);
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from.name, "::C::wire", "{incoming:?}");
    assert_eq!(incoming[0].from_ranges.len(), 1, "{incoming:?}");

    let wire = prepare(src, 3, 11, &analysis);
    let outgoing = outgoing_calls(src, dialect, &wire[0], &analysis);
    assert_eq!(outgoing.len(), 1, "{outgoing:?}");
    assert_eq!(outgoing[0].to.name, "::C::read", "{outgoing:?}");
    assert_eq!(outgoing[0].from_ranges.len(), 1, "{outgoing:?}");
}

#[test]
fn hierarchy_follows_one_hop_stored_my_callback() {
    let src = "oo::class create C {\n    method tick {} {}\n    method wire {} {\n        set cb [list my tick]\n        after 0 $cb\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let tick = prepare(src, 1, 11, &analysis);
    let incoming = incoming_calls(src, dialect, &tick[0], &analysis);
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from.name, "::C::wire", "{incoming:?}");
    assert_eq!(incoming[0].from_ranges.len(), 1, "{incoming:?}");

    let wire = prepare(src, 2, 11, &analysis);
    let outgoing = outgoing_calls(src, dialect, &wire[0], &analysis);
    assert_eq!(outgoing.len(), 1, "{outgoing:?}");
    assert_eq!(outgoing[0].to.name, "::C::tick", "{outgoing:?}");
    assert_eq!(outgoing[0].from_ranges.len(), 1, "{outgoing:?}");
}

#[test]
fn cross_document_instance_hierarchy_includes_constructor_and_destructor_frames() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let src =
        "oo::class create Child {\n    constructor {} { my read }\n    destructor { my read }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl");
    let mut incoming = incoming_instance_method_calls_in_class(
        src,
        analysis.resolved_profile().unwrap(),
        &analysis,
        "::Child",
        "read",
        true,
    );
    incoming.sort_by(|a, b| a.from.name.cmp(&b.from.name));
    let callers: Vec<_> = incoming
        .iter()
        .map(|call| (call.from.name.as_str(), call.from_ranges.len()))
        .collect();
    assert_eq!(
        callers,
        vec![("::Child::<constructor>", 1), ("::Child::<destructor>", 1)],
        "cross-document callers must cover every instance execution frame"
    );
}

/// FP guard: a bare `greet` call inside a sibling method is *not* a
/// `TclOO` dispatch (unlike a plain proc, a method is never a
/// bare-callable command), so it must not be counted as an incoming
/// call — the previous bare-head matcher would have (wrongly) found
/// this, while missing the one real (`my`-prefixed) shape entirely.
#[test]
fn incoming_calls_excludes_bare_head_that_is_not_valid_tcl() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet }\n}\n";
    let analysis = analyse(src);
    let items = prepare(src, 1, 11, &analysis);
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    assert!(incoming.is_empty(), "{incoming:?}");
}

/// Regression: `prepare()` must resolve the *classmethod* when the
/// cursor sits on its own declaration, even though a same-named
/// instance method exists in the same class — the previous
/// methods-first lookup ignored where the cursor actually was and would
/// have silently returned the instance method's item instead.
/// `classmethod` is Tcl 9.0+, so this analyses at 9.0.
#[test]
fn prepare_resolves_classmethod_over_same_named_method_by_cursor() {
    let src = "oo::class create C {\n    method greet {} {}\n    classmethod greet {} {}\n}\n";
    let analysis = analyse_tcl9(src);
    // Cursor on the *classmethod* declaration (line 2, col 16).
    let items = prepare(src, 2, 16, &analysis);
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(
        items[0].detail.as_deref(),
        Some("classmethod of ::C (0 params)"),
        "{items:?}"
    );
}

/// Regression: a `method` and a `classmethod` sharing a name must not
/// collide. In Tcl 9 `classmethod greet` also installs the instance forward
/// `greet` -> `myclass greet`; the later `method greet` replaces it, so
/// `my greet` from an instance method enters only the instance method
/// (tclsh 9.0.4/9.1.0). Matching both by name alone double-counts the
/// single real call site under one item key. `classmethod` is Tcl 9.0+,
/// so this analyses at 9.0 and declares the classmethod first.
#[test]
fn outgoing_calls_does_not_conflate_method_and_classmethod_sharing_a_name() {
    let src = "oo::class create C {classmethod greet {} {}\n    method greet {} {}\n\n    method twice {} { my greet }\n}\n";
    let analysis = analyse_tcl9(src);
    let items = prepare(src, 3, 11, &analysis);
    assert_eq!(items[0].name, "::C::twice");
    let outgoing = outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    assert_eq!(outgoing.len(), 1, "{outgoing:?}");
    assert_eq!(
        outgoing[0].from_ranges.len(),
        1,
        "must not double-count the single `my greet` site: {outgoing:?}"
    );
    // The resolved item is the instance method (line 1), not the
    // classmethod (line 0).
    assert_eq!(outgoing[0].to.selection_range.start_line, 1, "{outgoing:?}");
}

#[test]
fn outgoing_calls_from_method_to_sibling_method() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    // Resolve the `twice` method (line 2, col 11).
    let items = prepare(src, 2, 11, &analysis);
    assert_eq!(items[0].name, "::C::twice");
    let outgoing = outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
    assert_eq!(names, vec!["::C::greet"], "{outgoing:?}");
    // Two call sites collapse into one target entry.
    assert_eq!(outgoing[0].from_ranges.len(), 2, "{outgoing:?}");
}

/// FN→TP: a `my method` dispatch nested
/// inside `if` / `foreach` / `switch` control flow is an outgoing call
/// too — `scan_my_method_sites` recurses generically via the
/// registry's `Plain`-`BodyKind` body roles, so call hierarchy inherits
/// the same coverage Find-References / rename / the code lens gained.
#[test]
fn outgoing_calls_from_method_nested_in_control_flow() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} {\n        if {1} {\n            switch -- 1 {\n                default {\n                    my greet\n                }\n            }\n        }\n    }\n}\n";
    let analysis = analyse(src);
    let items = prepare(src, 2, 11, &analysis);
    assert_eq!(items[0].name, "::C::twice");
    let outgoing = outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
    assert_eq!(names, vec!["::C::greet"], "{outgoing:?}");
}

/// FN→TP: a `my method` dispatch nested
/// inside control flow is an *incoming* call edge too, mirroring
/// `outgoing_calls_from_method_nested_in_control_flow`.
#[test]
fn incoming_calls_for_method_nested_in_control_flow() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} {\n        if {1} {\n            switch -- 1 {\n                default {\n                    my greet\n                }\n            }\n        }\n    }\n}\n";
    let analysis = analyse(src);
    let items = prepare(src, 1, 11, &analysis);
    assert_eq!(items[0].name, "::C::greet");
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from.name, "::C::twice", "{incoming:?}");
}

#[test]
fn outgoing_calls_from_method_to_top_level_proc() {
    let src = "proc helper {} {}\noo::class create C {\n    method use {} { helper }\n}\n";
    let analysis = analyse(src);
    // Resolve the `use` method (line 2, col 11).
    let items = prepare(src, 2, 11, &analysis);
    assert_eq!(items[0].name, "::C::use");
    let outgoing = outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    let names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
    assert_eq!(names, vec!["helper"], "{outgoing:?}");
}

/// Regression: a `my <method>` dispatch inside a method body must not
/// leak "my" itself into the unresolved-outgoing-calls list — `my` is
/// a `TclOO` dispatch keyword, not an unresolved command reference.
#[test]
fn unresolved_method_outgoing_calls_excludes_my_keyword() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet }\n}\n";
    let analysis = analyse(src);
    let items = prepare(src, 2, 11, &analysis);
    assert_eq!(items[0].name, "::C::twice");
    let unresolved = unresolved_outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    assert!(unresolved.iter().all(|u| u.name != "my"), "{unresolved:?}");
}

// workspace-index: cross-document incoming calls

#[test]
fn incoming_calls_for_target_finds_callers_in_other_doc() {
    // A consumer document that *doesn't* define `helper`
    // but calls it from inside `caller` and at top level.
    let src = "proc caller {} { helper }\nhelper\n";
    let analysis = analyse(src);
    let calls = incoming_calls_for_target(src, &analysis, "helper", "helper", None);
    // Callers: `caller` (one call) + `<top-level>` (one).
    let from: Vec<&str> = calls.iter().map(|c| c.from.name.as_str()).collect();
    assert!(from.contains(&"caller"), "{calls:?}");
    assert!(from.contains(&"<top-level>"), "{calls:?}");
}

#[test]
fn incoming_calls_for_target_no_self_skip_without_span() {
    // With target_name_span = None nothing is skipped as a
    // declaration, so a doc that calls the proc once yields
    // exactly one caller bucket.
    let src = "proc c {} { helper }\n";
    let analysis = analyse(src);
    let calls = incoming_calls_for_target(src, &analysis, "helper", "helper", None);
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].from.name, "c");
}

// Nested namespaces: `invocation_targets` delegates to
// `references::invocation_references_proc`, so a bareword call inside a
// namespace nested 2+ levels deep must reach Call Hierarchy just as it
// reaches find-references.

#[test]
fn incoming_calls_finds_bare_call_from_two_level_nested_namespace() {
    let src = concat!(
        "namespace eval a {\n",
        "    namespace eval b {\n",
        "        proc target {} {}\n",
        "        proc caller {} { target }\n",
        "    }\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Cursor on `target`'s declaration (line 2).
    let items = prepare(src, 2, 14, &analysis);
    assert_eq!(items.len(), 1, "{items:?}");
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items[0],
        &analysis,
    );
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from.name, "caller");
}

#[test]
fn incoming_calls_two_level_nested_namespace_does_not_leak_across_namespaces() {
    let src = concat!(
        "namespace eval a {\n",
        "    namespace eval b {\n",
        "        proc helper {} {}\n",
        "    }\n",
        "}\n",
        "namespace eval c {\n",
        "    namespace eval d {\n",
        "        proc helper {} {}\n",
        "        proc caller {} { helper }\n",
        "    }\n",
        "}\n",
    );
    let analysis = analyse(src);
    // `::a::b::helper` has no callers.
    let items_ab = prepare(src, 2, 14, &analysis);
    assert_eq!(items_ab.len(), 1, "{items_ab:?}");
    assert!(
        incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &items_ab[0],
            &analysis
        )
        .is_empty(),
        "::a::b::helper must not pick up ::c::d's caller"
    );
    // `::c::d::helper` has exactly one (its own).
    let items_cd = prepare(src, 7, 14, &analysis);
    assert_eq!(items_cd.len(), 1, "{items_cd:?}");
    let incoming_cd = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &items_cd[0],
        &analysis,
    );
    assert_eq!(incoming_cd.len(), 1, "{incoming_cd:?}");
    assert_eq!(incoming_cd[0].from.name, "caller");
}

// Bare `ClassName <classmethod>` dispatch.  `classmethod`
// is Tcl 9.0+, so these analyse at 9.0.

fn analyse_tcl9(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl9.0").clone()
}

/// FN→TP: a `classmethod` dispatches on the
/// class's own command (`Factory make`), so its callers are the
/// top-level statement *and* the sibling classmethod's body — neither
/// of which has the member's own name as its head word, which is why
/// the head-word comparison found nothing.  tclsh9.0-verified: both
/// `Factory build` and the bare `Factory make` really do enter `make`.
#[test]
fn incoming_calls_for_classmethod_find_bare_class_dispatch() {
    let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n    classmethod build {} { Factory make }\n}\nFactory make\n";
    let analysis = analyse_tcl9(src);
    // Cursor on `make`'s declaration name (line 1, col 16).
    let items = prepare(src, 1, 16, &analysis);
    assert_eq!(items[0].name, "::Factory::make", "{items:?}");
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &items[0],
        &analysis,
    );
    let callers: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
    assert_eq!(
        callers,
        vec!["::Factory::build", "<top-level>"],
        "{incoming:?}"
    );
    assert!(
        incoming.iter().all(|c| c.from_ranges.len() == 1),
        "one call site each: {incoming:?}"
    );
}

/// FN→TP: the outgoing direction of the same repro — `build`'s body
/// dispatches `Factory make`, so `make` is its callee.
#[test]
fn outgoing_calls_from_classmethod_reach_bare_class_dispatch() {
    let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n    classmethod build {} { Factory make }\n}\nFactory make\n";
    let analysis = analyse_tcl9(src);
    // Cursor on `build`'s declaration name (line 2, col 16).
    let items = prepare(src, 2, 16, &analysis);
    assert_eq!(items[0].name, "::Factory::build", "{items:?}");
    let outgoing = outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &items[0],
        &analysis,
    );
    let callees: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
    assert_eq!(callees, vec!["::Factory::make"], "{outgoing:?}");
    assert_eq!(outgoing[0].from_ranges.len(), 1, "{outgoing:?}");
}

/// FN→TP: [incr Tcl]'s class-scoped `proc` dispatches as a
/// single `::`-qualified word, so the two-word scan that finds a
/// `classmethod`'s edges never saw it and itcl call hierarchies were
/// empty.  Oracle (tclsh 8.6.14 + Itcl 3.4): a bare `make` inside a
/// sibling class `proc`, `Factory::make` inside a method body, and a
/// top-level `Factory::make` all enter `make`.
#[test]
fn incoming_calls_for_an_itcl_class_proc_find_colon_qualified_dispatch() {
    let src = "itcl::class Factory {\n    proc make {} { return 1 }\n    proc build {} { return [make] }\n    method viaInstance {} { return [Factory::make] }\n}\nFactory::make\n";
    let analysis = analyse(src);
    // Cursor on `make`'s declaration name (line 1, col 10).
    let items = prepare(src, 1, 10, &analysis);
    assert_eq!(items[0].name, "::Factory::make", "{items:?}");
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &items[0],
        &analysis,
    );
    let callers: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
    assert_eq!(
        callers,
        vec!["::Factory::build", "::Factory::viaInstance", "<top-level>"],
        "{incoming:?}"
    );
}

/// FN→TP: the outgoing direction of the same shape — `build`'s body
/// dispatches `make`, so `make` is its callee.
#[test]
fn outgoing_calls_from_an_itcl_class_proc_reach_the_sibling_class_proc() {
    let src = "itcl::class Factory {\n    proc make {} { return 1 }\n    proc build {} { return [make] }\n}\nFactory::make\n";
    let analysis = analyse(src);
    let items = prepare(src, 2, 10, &analysis);
    assert_eq!(items[0].name, "::Factory::build", "{items:?}");
    let outgoing = outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &items[0],
        &analysis,
    );
    let callees: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
    assert_eq!(callees, vec!["::Factory::make"], "{outgoing:?}");
}

/// TN: itcl's two-word `Factory make` is object *creation*
/// (`ClassName instanceName`), never a class-proc dispatch, so it adds
/// no call-hierarchy edge.
#[test]
fn itcl_two_word_object_creation_is_not_a_call_hierarchy_edge() {
    let src = "itcl::class Factory {\n    proc make {} { return 1 }\n}\nFactory make\n";
    let analysis = analyse(src);
    let items = prepare(src, 1, 10, &analysis);
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &items[0],
        &analysis,
    );
    assert!(incoming.is_empty(), "{incoming:?}");
}

/// TP: a class command is an ordinary global command, so an *instance*
/// method's body dispatching `Factory make` is an incoming edge too
/// (tclsh9.0-verified) — the `my`-scope rule that keeps instance and
/// class method tables apart does not apply to this shape.
#[test]
fn incoming_calls_for_classmethod_include_an_instance_method_caller() {
    let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n    method viaInstance {} { Factory make }\n}\n";
    let analysis = analyse_tcl9(src);
    let items = prepare(src, 1, 16, &analysis);
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &items[0],
        &analysis,
    );
    let callers: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
    assert_eq!(callers, vec!["::Factory::viaInstance"], "{incoming:?}");
}

/// FN→TP: a **proc** body dispatching a bare class command must list the
/// classmethod under its Outgoing Calls, matching the proc that the
/// classmethod's Incoming Calls lists — the edge must exist in both
/// directions.
#[test]
fn outgoing_calls_from_proc_reach_bare_class_dispatch() {
    let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n}\nproc build {} { Factory make }\n";
    let analysis = analyse_tcl9(src);
    // Cursor on `build`'s declaration name (line 3, col 6).
    let items = prepare(src, 3, 6, &analysis);
    assert_eq!(items[0].name, "build", "{items:?}");
    let outgoing = outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &items[0],
        &analysis,
    );
    let callees: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
    assert_eq!(callees, vec!["::Factory::make"], "{outgoing:?}");
    assert_eq!(outgoing[0].from_ranges.len(), 1, "{outgoing:?}");
    // Symmetric with the incoming direction from the same fixture.
    let make = prepare(src, 1, 16, &analysis);
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &make[0],
        &analysis,
    );
    let calling_procs: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
    assert_eq!(calling_procs, vec!["::build"], "{incoming:?}");
}

/// An instance `method make` and a `classmethod
/// make` on the same class are two distinct declarations (tclsh 9.0.4:
/// `my make` returns `inst-make`, `C make` returns `class-make`), so a
/// caller that dispatches both must show two callee items with their own
/// declaration ranges — not one merged item keyed on the shared display
/// name `::C::make`.
#[test]
fn outgoing_calls_separate_same_named_instance_method_and_classmethod() {
    let src = "oo::class create C {\n\
                   \x20   classmethod make {} { return 1 }\n\
                   \x20   method make {} { return 2 }\n\
                   \x20   method caller {} { my make ; C make }\n\
                   }\n";
    let analysis = analyse_tcl9(src);
    // Cursor on `caller`'s declaration name (line 3, col 11).
    let items = prepare(src, 3, 11, &analysis);
    assert_eq!(items[0].name, "::C::caller", "{items:?}");
    let outgoing = outgoing_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &items[0],
        &analysis,
    );
    assert_eq!(outgoing.len(), 2, "{outgoing:?}");
    for call in &outgoing {
        assert_eq!(call.to.name, "::C::make", "{outgoing:?}");
        assert_eq!(call.from_ranges.len(), 1, "{outgoing:?}");
    }
    // One item points at the classmethod's declaration (line 1), the
    // other at the instance method's (line 2).
    let mut decl_lines: Vec<u32> = outgoing
        .iter()
        .map(|c| c.to.selection_range.start_line)
        .collect();
    decl_lines.sort_unstable();
    assert_eq!(decl_lines, vec![1, 2], "{outgoing:?}");
}

/// A proc caller listed beside a
/// method caller is named the same way — qualified — rather than by its
/// short name.
#[test]
fn classmethod_incoming_names_proc_callers_qualified() {
    let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n    method viaInstance {} { Factory make }\n}\nnamespace eval ::util {\n    proc helper {} { Factory make }\n}\n";
    let analysis = analyse_tcl9(src);
    let items = prepare(src, 1, 16, &analysis);
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &items[0],
        &analysis,
    );
    let callers: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
    assert_eq!(
        callers,
        vec!["::Factory::viaInstance", "::util::helper"],
        "{incoming:?}"
    );
}

/// FN→TP: a bare class dispatch
/// inside an `apply` lambda body or a `namespace eval` body is a real
/// call site, so the call hierarchy must attribute it to the body it
/// sits in.  tclsh 9.0.4 runs all three of these dispatches.
#[test]
fn classmethod_incoming_covers_lambda_and_namespace_eval_bodies() {
    let src = "oo::class create Factory {\n\
                   \x20   classmethod make {} { return 1 }\n\
                   \x20   method inst {} { apply {{} { Factory make }} }\n\
                   \x20   method nsev {} { namespace eval ::zz { Factory make } }\n\
                   }\n\
                   namespace eval ::top2 { Factory make }\n";
    let analysis = analyse_tcl9(src);
    let items = prepare(src, 1, 16, &analysis);
    let incoming = incoming_calls(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &items[0],
        &analysis,
    );
    let callers: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
    assert_eq!(
        callers,
        vec!["::Factory::inst", "::Factory::nsev", "<top-level>"],
        "{incoming:?}"
    );
}

/// FP guard: `Factory make` where `Factory` is an ordinary proc calls
/// *that proc* with the literal argument `make` (tclsh8.6/9.0-verified:
/// it prints `proc Factory: make`), so it is no edge at all to an
/// unrelated class's same-named classmethod.
#[test]
fn bare_dispatch_on_a_same_named_proc_is_not_a_classmethod_edge() {
    let src = "oo::class create Widget {\n    classmethod make {} { return 1 }\n}\nproc Factory {args} { return $args }\nFactory make\n";
    let analysis = analyse_tcl9(src);
    let items = prepare(src, 1, 16, &analysis);
    assert_eq!(items[0].name, "::Widget::make", "{items:?}");
    assert!(
        incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &items[0],
            &analysis
        )
        .is_empty(),
        "a proc call spelled `Factory make` is not a dispatch of Widget's classmethod"
    );
}

/// FP guard: an instance `method` is *not* bare-dispatchable on the
/// class command (`Factory make` reaches only the class object's own
/// method table), so the classmethod leg must not fire for one.
#[test]
fn bare_class_dispatch_is_not_an_edge_to_a_same_named_instance_method() {
    let src = "oo::class create Factory {\n    method make {} { return 1 }\n}\nFactory make\n";
    let analysis = analyse_tcl9(src);
    let items = prepare(src, 1, 11, &analysis);
    assert_eq!(items[0].name, "::Factory::make", "{items:?}");
    assert!(
        incoming_calls(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &items[0],
            &analysis
        )
        .is_empty(),
        "an instance method has no bare class-command dispatch"
    );
}
