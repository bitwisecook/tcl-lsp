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

fn analysed(source: &str) -> AnalysisResult {
    Analyser::new().analyse(source, "tcl")
}

fn proc_item(source: &str, analysis: &AnalysisResult, qualified: &str) -> CallHierarchyItem {
    item_for_proc(
        source,
        &analysis.all_procs[qualified],
        qualified,
        &LineIndex::new(source),
    )
}

fn member_item(
    source: &str,
    analysis: &AnalysisResult,
    class: &str,
    method: &str,
    side: MemberSide,
) -> CallHierarchyItem {
    let class = &analysis.all_classes[class];
    let members = match side {
        MemberSide::Instance => &class.methods,
        MemberSide::ClassObject => &class.class_methods,
    };
    item_for_method(source, class, &members[method], &LineIndex::new(source))
}

#[test]
fn exact_declaration_owner_separates_procedure_member_and_member_side() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let source = "namespace eval C {}; proc ::C::m {} {}; oo::class create C {method m {} {}; self method m {} {}}";
    let analysis = analysed(source);
    let mut proc = proc_item(source, &analysis, "::C::m");
    proc.name = "::C::m".into();
    let instance = member_item(source, &analysis, "::C", "m", MemberSide::Instance);
    let class = member_item(source, &analysis, "::C", "m", MemberSide::ClassObject);
    assert_eq!(proc.name, instance.name);
    assert_eq!(instance.name, class.name);
    for item in [&proc, &instance, &class] {
        let prepared = prepare(
            source,
            item.selection_range.start_line,
            item.selection_range.start_character,
            &analysis,
        );
        assert_eq!(prepared.len(), 1);
        assert_eq!(prepared[0].selection_range, item.selection_range);
    }
    assert!(matches!(
        LogicalCallTarget::capture(source, &analysis, &proc)
            .unwrap()
            .declaration(),
        LogicalCallDeclaration::Procedure {
            qualified: "::C::m",
            ..
        }
    ));
    for (item, expected) in [
        (instance, MemberSide::Instance),
        (class, MemberSide::ClassObject),
    ] {
        let target = LogicalCallTarget::capture(source, &analysis, &item).unwrap();
        let LogicalCallDeclaration::Member {
            class,
            definition,
            side,
        } = target.declaration()
        else {
            panic!("member declaration");
        };
        assert_eq!(class.qualified_name, "::C");
        assert_eq!(definition.name, "m");
        assert_eq!(side, expected);
    }
}

#[test]
fn colon_bearing_members_and_equal_joined_titles_keep_actual_boundaries() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let source = "namespace eval A {}; oo::class create A {method b::c {} {}}; oo::class create ::A::b {method c {} {}}";
    let analysis = analysed(source);
    let a = member_item(source, &analysis, "::A", "b::c", MemberSide::Instance);
    let b = member_item(source, &analysis, "::A::b", "c", MemberSide::Instance);
    assert_eq!(a.name, b.name);
    for (item, class_name, member_name) in [(a, "::A", "b::c"), (b, "::A::b", "c")] {
        let LogicalCallDeclaration::Member {
            class, definition, ..
        } = LogicalCallTarget::capture(source, &analysis, &item)
            .unwrap()
            .declaration()
        else {
            panic!("member declaration");
        };
        assert_eq!(class.qualified_name, class_name);
        assert_eq!(definition.name, member_name);
    }
}

#[test]
fn all_local_call_consumers_ignore_copied_reporting_labels() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let source = "proc callee {} {}; proc p {} {callee; absent}; p; oo::class create C {method a::b {} {callee; absent}; method caller {} {my a::b}}";
    let analysis = analysed(source);
    let profile = analysis.resolved_profile().unwrap();
    for original in [
        proc_item(source, &analysis, "::p"),
        member_item(source, &analysis, "::C", "a::b", MemberSide::Instance),
    ] {
        let mut relabelled = original.clone();
        relabelled.name = "::unrelated::display".into();
        relabelled.detail = Some("copied detail".into());
        let incoming = incoming_calls(source, profile, &original, &analysis);
        let outgoing = outgoing_calls(source, profile, &original, &analysis);
        let unresolved = unresolved_outgoing_calls(source, profile, &original, &analysis);
        assert!(
            !outgoing.is_empty(),
            "actual selected body must reach callee"
        );
        assert!(unresolved.iter().any(|call| call.name == "absent"));
        assert_eq!(
            incoming,
            incoming_calls(source, profile, &relabelled, &analysis)
        );
        assert_eq!(
            outgoing,
            outgoing_calls(source, profile, &relabelled, &analysis)
        );
        assert_eq!(
            unresolved,
            unresolved_outgoing_calls(source, profile, &relabelled, &analysis)
        );
    }
}

#[test]
fn target_withdraws_missing_foreign_stale_ambiguous_and_non_declaration_owners() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let source = "proc p {} {}; p";
    let analysis = analysed(source);
    let item = proc_item(source, &analysis, "::p");
    assert!(LogicalCallTarget::capture(source, &analysis, &item).is_some());
    assert!(LogicalCallTarget::capture("proc q {} {}; q", &analysis, &item).is_none());
    let mut stale = item.clone();
    stale.selection_range.start_character += 1;
    assert!(LogicalCallTarget::capture(source, &analysis, &stale).is_none());
    let mut non_decl = item.clone();
    non_decl.selection_range = span_to_range(
        source,
        &LineIndex::new(source),
        analysis.command_invocations.last().unwrap().range,
    );
    assert!(LogicalCallTarget::capture(source, &analysis, &non_decl).is_none());
    let mut ambiguous = analysis.clone();
    ambiguous
        .all_procs
        .insert("::copied".into(), analysis.all_procs["::p"].clone());
    assert!(LogicalCallTarget::capture(source, &ambiguous, &item).is_none());
    for axis in 0..3 {
        let mut changed = analysis.clone();
        match axis {
            0 => changed.resolved_input = None,
            1 => changed.body_lexer_config.as_mut().unwrap().strict_quoting ^= true,
            _ => {
                let input = analysis.resolved_input.as_ref().unwrap();
                changed.resolved_input = Some(ResolvedAnalysisInput::new(
                    input.analyser_profile(),
                    input.unit_profile(),
                    tcl_registry::model::ingress::resolve_environment("tcl8.4")
                        .default_context_registry(),
                    input.lexer_config(),
                ));
            }
        }
        assert!(LogicalCallTarget::capture(source, &changed, &item).is_none());
    }
}

#[test]
fn native_and_hosted_inputs_cannot_receive_a_logical_target_from_ranges() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let source = "proc p {} {}; p";
    for dialect in ["tcl8.6", "f5-irules"] {
        let analysis = Analyser::new().analyse(source, dialect);
        let item = proc_item(source, &analysis, "::p");
        assert!(!analysis.allows_retained_logical_declaration_advice());
        if dialect == "tcl8.6" {
            assert!(
                crate::original_context::CurrentSourceContext::capture(source, &analysis).is_some()
            );
        }
        assert!(
            LogicalCallTarget::capture(source, &analysis, &item).is_none(),
            "{dialect}"
        );
    }
}

#[test]
fn class_selector_geometry_and_external_member_visibility_remain_independent() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let source = "oo::class create C {self method visible {} {}; self method hidden {} {}; self unexport hidden}; proc caller {} {C visible; C hidden}";
    let analysis = analysed(source);
    let class = &analysis.all_classes["::C"];
    assert_eq!(class.class_methods["hidden"].visibility, "unexported");
    for method in ["visible", "hidden"] {
        assert_eq!(
            crate::references::logical_class_command_method_sites(source, &analysis, class, method)
                .len(),
            1
        );
    }
    let caller = proc_item(source, &analysis, "::caller");
    let outgoing = outgoing_calls(
        source,
        analysis.resolved_profile().unwrap(),
        &caller,
        &analysis,
    );
    assert_eq!(outgoing.len(), 1, "{outgoing:?}");
    assert_eq!(outgoing[0].to.name, "::C::visible");
    assert_eq!(outgoing[0].from_ranges.len(), 1);
}

#[test]
fn logical_workspace_member_provider_uses_current_source_owners_instead_of_labels() {
    use crate::workspace_index::{MethodAccess, WorkspaceIndex};
    let source = "oo::class create Base {method m {} {}; self method m {} {}}";
    let analysis = analysed(source);
    let uri = "file:///provider.tcl";
    for side in [MemberSide::Instance, MemberSide::ClassObject] {
        let item = member_item(source, &analysis, "::Base", "m", side);
        let target = LogicalCallTarget::capture(source, &analysis, &item).unwrap();
        let mut index = WorkspaceIndex::new();
        index.add_document(uri, &analysis);
        let chain = match side {
            MemberSide::Instance => {
                index.method_dispatch_chain("::Base", "m", MethodAccess::Internal)
            }
            MemberSide::ClassObject => {
                index.class_method_dispatch_chain("::Base", "m", MethodAccess::Internal)
            }
        };
        let provider = chain.first().unwrap();
        assert!(index.logical_member_provider_matches_target(uri, provider, &target));
        assert!(!index.logical_member_provider_matches_target(
            "file:///other.tcl",
            provider,
            &target
        ));
        let mut label_analysis = analysis.clone();
        label_analysis
            .all_classes
            .get_mut("::Base")
            .unwrap()
            .qualified_name = "an unrelated display".to_owned();
        let mut label_index = WorkspaceIndex::new();
        label_index.add_document(uri, &label_analysis);
        let label_provider = label_index.classes().next().unwrap();
        assert!(label_index.logical_member_provider_matches_target(uri, label_provider, &target));
        let mut detached = (**provider).clone();
        detached.qualified_name = "an unrelated display".to_owned();
        assert!(!index.logical_member_provider_matches_target(uri, &detached, &target));
        index.replace_document(uri, &analysed(&format!("# changed\n{source}")));
        let provider = match side {
            MemberSide::Instance => {
                index.method_dispatch_chain("::Base", "m", MethodAccess::Internal)
            }
            MemberSide::ClassObject => {
                index.class_method_dispatch_chain("::Base", "m", MethodAccess::Internal)
            }
        };
        assert!(!index.logical_member_provider_matches_target(
            uri,
            provider.first().unwrap(),
            &target
        ));
    }
}

#[test]
fn cross_document_member_advice_selects_only_the_actual_side_frames() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let provider = "oo::class create Base {method m {} {}; self method m {} {}}";
    let provider_analysis = analysed(provider);
    let consumer = "oo::class create Child {method instanceCaller {} {my m}; self method classCaller {} {my m}}";
    let consumer_analysis = analysed(consumer);
    for (side, expected) in [
        (MemberSide::Instance, "::Child::instanceCaller"),
        (MemberSide::ClassObject, "::Child::classCaller"),
    ] {
        let item = member_item(provider, &provider_analysis, "::Base", "m", side);
        let target = LogicalCallTarget::capture(provider, &provider_analysis, &item).unwrap();
        let calls = incoming_calls_for_logical_member_in_class(
            consumer,
            &consumer_analysis,
            "::Child",
            &target,
            true,
        );
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].from.name, expected);
        assert_eq!(calls[0].from_ranges.len(), 1);
    }
}

/// A method body's absolute call reaches the procedure in the call
/// hierarchy, from both ends. tclsh 8.6.18/9.1.0: [C new] m runs ::helper.
#[test]
fn a_method_body_call_of_an_absolute_procedure_joins_both_hierarchy_ends() {
    let source = "proc helper {} {}\noo::class create C {method m {} {::helper}}\n[C new] m\n";
    let analysis = analysed(source);
    let profile = analysis.resolved_profile().unwrap();
    let helper = proc_item(source, &analysis, "::helper");
    let method = member_item(source, &analysis, "::C", "m", MemberSide::Instance);
    let incoming = incoming_calls(source, profile, &helper, &analysis);
    assert!(
        incoming
            .iter()
            .any(|call| call.from.selection_range == method.selection_range),
        "::helper's callers include method m: {incoming:?}"
    );
    let outgoing = outgoing_calls(source, profile, &method, &analysis);
    assert!(
        outgoing
            .iter()
            .any(|call| call.to.selection_range == helper.selection_range),
        "method m calls ::helper: {outgoing:?}"
    );
}
