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

//! Type hierarchy and call hierarchy across documents.

use super::*;

#[tokio::test]
async fn original_type_hierarchy_wire_preserves_owner_and_rejects_labels_and_stale_source() {
    // Contract: naming.consumer.original-type-hierarchy
    // docs/design/analysis/name-resolution-proofs/original-type-hierarchy.md
    let backend = test_backend();
    let base = Uri::from_str("file:///original-hierarchy-base.tcl").unwrap();
    let child = Uri::from_str("file:///original-hierarchy-child.tcl").unwrap();
    let base_source = r"oo::class create B\uD800 {}";
    let child_source = r"oo::class create Child {superclass B\uD800}";
    register(&backend, &base, base_source).await;
    register(&backend, &child, child_source).await;
    let params = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": child.as_str()}, "position": {"line": 0, "character": 18}
    }))
    .unwrap();
    let mut items = backend
        .prepare_type_hierarchy(params)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(items.len(), 1);
    let mut item = items.remove(0);
    assert!(
        item.data
            .as_ref()
            .is_some_and(|data| data.get("originalDeclaration").is_some())
    );
    // Presentation is deliberately counterfactual; only the retained
    // receipt can select the original declaration for this request.
    item.name = "unrelated display".to_owned();
    item.range = Range::default();
    let parents = backend
        .type_hierarchy_walk(item.clone(), false)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(parents.len(), 1);
    assert_eq!(parents[0].uri, base);
    assert!(parents[0].data.is_some());
    let mut unissued = item.clone();
    unissued.data = None;
    assert_eq!(
        backend
            .type_hierarchy_walk(unissued, false)
            .await
            .unwrap()
            .unwrap(),
        [] as [tower_lsp_server::ls_types::TypeHierarchyItem; 0]
    );
    let mut foreign = item.clone();
    foreign.uri = base.clone();
    assert_eq!(
        backend
            .type_hierarchy_walk(foreign, false)
            .await
            .unwrap()
            .unwrap(),
        [] as [tower_lsp_server::ls_types::TypeHierarchyItem; 0]
    );
    register(&backend, &child, &format!("# changed\n{child_source}")).await;
    assert_eq!(
        backend
            .type_hierarchy_walk(item, false)
            .await
            .unwrap()
            .unwrap(),
        [] as [tower_lsp_server::ls_types::TypeHierarchyItem; 0]
    );
}

#[tokio::test]
async fn incoming_calls_span_multiple_documents() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
    register_logical_hierarchy(&backend, &lib, "proc helper {} {}\n").await;
    register_logical_hierarchy(&backend, &consumer, "proc caller {} { helper }\n").await;
    // prepare the `helper` item from lib.tcl.
    let prep = {
        let mut a = Analyser::new();
        let analysis = a.analyse("proc helper {} {}\n", "tcl").clone();
        core_call_hierarchy::prepare("proc helper {} {}\n", 0, 5, &analysis)
    };
    let core_item = prep.into_iter().next().expect("prepared item");
    // Source-advice procedure items carry the short display name.
    assert_eq!(core_item.name, "helper");
    let analysis = Analyser::new().analyse("proc helper {} {}\n", "tcl");
    let target = core_call_hierarchy::LogicalCallTarget::capture(
        "proc helper {} {}\n",
        &analysis,
        &core_item,
    )
    .expect("current source-advice declaration");
    let cross = backend
        .cross_document_incoming_calls(&lib, &target)
        .await
        .unwrap();
    // The only caller is `caller` in consumer.tcl.
    assert_eq!(cross.len(), 1, "{cross:?}");
    assert_eq!(cross[0].0, consumer);
    assert_eq!(cross[0].1.from.name, "caller");
}

#[tokio::test]
async fn outgoing_calls_resolve_sibling_document_callee() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib.tcl").unwrap();
    let main = Uri::from_str("file:///main.tcl").unwrap();
    register_logical_hierarchy(&backend, &lib, "proc helper {} {}\n").await;
    // `caller` calls a builtin (`puts`) and the sibling-file
    // proc `helper`; only `helper` resolves cross-document.
    let main_src = "proc caller {} { puts hi\n helper }\n";
    register_logical_hierarchy(&backend, &main, main_src).await;

    let item = core_call_hierarchy::CallHierarchyItem {
        identity: None,
        name: "::caller".to_owned(),
        detail: None,
        range: CoreLspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 0,
        },
        selection_range: CoreLspRange {
            start_line: 0,
            start_character: 5,
            end_line: 0,
            end_character: 11,
        },
    };
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(main_src, "tcl").clone()
    };
    let cross = backend
        .cross_document_outgoing_calls(
            &main,
            main_src,
            tcl_lsp_core::profile_for_dialect("tcl"),
            &item,
            &analysis,
        )
        .await
        .unwrap();
    assert_eq!(cross.len(), 1, "{cross:?}");
    assert_eq!(cross[0].to.name, "::helper");
    assert_eq!(cross[0].to.uri, lib);
    assert_eq!(cross[0].from_ranges.len(), 1, "{cross:?}");
}

fn logical_member_item(
    source: &str,
    analysis: &AnalysisResult,
    class: &str,
    method: &str,
    side: tcl_compiler::analyser::MemberSide,
) -> core_call_hierarchy::CallHierarchyItem {
    let class = &analysis.all_classes[class];
    let members = match side {
        tcl_compiler::analyser::MemberSide::Instance => &class.methods,
        tcl_compiler::analyser::MemberSide::ClassObject => &class.class_methods,
    };
    let span = members[method].name_span;
    let lines = tcl_lexer::LineIndex::new(source);
    let position = lines.position_at_utf16(span.start(), source);
    core_call_hierarchy::prepare(source, position.line, position.character.get(), analysis)
        .into_iter()
        .find(|item| {
            item.selection_range.start_line == position.line
                && item.selection_range.start_character == position.character.get()
        })
        .expect("actual member declaration")
}

#[tokio::test]
async fn logical_incoming_target_keeps_procedure_member_side_and_colon_name_separate() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let backend = test_backend();
    let provider_uri = Uri::from_str("file:///logical-provider.tcl").unwrap();
    let caller_uri = Uri::from_str("file:///logical-callers.tcl").unwrap();
    let provider = "namespace eval C {}; proc ::C::m {} {}; oo::class create C {method m {} {}; self method m {} {}; method a::b {} {}}";
    let caller = "oo::class create C {method instanceCaller {} {my m}; self method classCaller {} {my m}; method colonCaller {} {my a::b}}; proc procedureCaller {} {::C::m}";
    register_logical_hierarchy(&backend, &provider_uri, provider).await;
    register_logical_hierarchy(&backend, &caller_uri, caller).await;
    let analysis = Analyser::new().analyse(provider, "tcl");
    for (side, method, expected) in [
        (
            tcl_compiler::analyser::MemberSide::Instance,
            "m",
            "::C::instanceCaller",
        ),
        (
            tcl_compiler::analyser::MemberSide::ClassObject,
            "m",
            "::C::classCaller",
        ),
        (
            tcl_compiler::analyser::MemberSide::Instance,
            "a::b",
            "::C::colonCaller",
        ),
    ] {
        let mut item = logical_member_item(provider, &analysis, "::C", method, side);
        item.name = "copied display label".into();
        item.detail = Some("copied detail".into());
        let target =
            core_call_hierarchy::LogicalCallTarget::capture(provider, &analysis, &item).unwrap();
        let cross = backend
            .cross_document_incoming_calls(&provider_uri, &target)
            .await
            .unwrap();
        assert_eq!(cross.len(), 1, "{side:?}/{method}: {cross:?}");
        assert_eq!(cross[0].0, caller_uri);
        assert_eq!(cross[0].1.from.name, expected);
        assert_eq!(cross[0].1.from_ranges.len(), 1);
    }
    let span = analysis.all_procs["::C::m"].name_span;
    let position = tcl_lexer::LineIndex::new(provider).position_at_utf16(span.start(), provider);
    let item =
        core_call_hierarchy::prepare(provider, position.line, position.character.get(), &analysis)
            .pop()
            .unwrap();
    let target =
        core_call_hierarchy::LogicalCallTarget::capture(provider, &analysis, &item).unwrap();
    let cross = backend
        .cross_document_incoming_calls(&provider_uri, &target)
        .await
        .unwrap();
    assert_eq!(cross.len(), 1, "{cross:?}");
    assert_eq!(cross[0].1.from.name, "procedureCaller");
}

#[tokio::test]
async fn logical_incoming_source_owner_withdrawal_cannot_be_recovered_from_a_title() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let backend = test_backend();
    let provider_uri = Uri::from_str("file:///logical-owner.tcl").unwrap();
    let caller_uri = Uri::from_str("file:///logical-caller.tcl").unwrap();
    let provider = "proc selected {} {}";
    let caller = "proc caller {} {selected}";
    register_logical_hierarchy(&backend, &provider_uri, provider).await;
    register_logical_hierarchy(&backend, &caller_uri, caller).await;
    let mut analysis = Analyser::new().analyse(provider, "tcl");
    let mut item = core_call_hierarchy::prepare(provider, 0, 6, &analysis)
        .pop()
        .unwrap();
    let target =
        core_call_hierarchy::LogicalCallTarget::capture(provider, &analysis, &item).unwrap();
    assert_eq!(
        backend
            .cross_document_incoming_calls(&provider_uri, &target)
            .await
            .unwrap()
            .len(),
        1
    );
    drop(target);
    item.selection_range.start_character += 1;
    assert!(core_call_hierarchy::LogicalCallTarget::capture(provider, &analysis, &item).is_none());
    item.selection_range.start_character -= 1;
    assert!(
        core_call_hierarchy::LogicalCallTarget::capture("proc changed {} {}", &analysis, &item)
            .is_none()
    );
    analysis.resolved_input = None;
    assert!(core_call_hierarchy::LogicalCallTarget::capture(provider, &analysis, &item).is_none());
    let native = Analyser::new().analyse(provider, "tcl8.6");
    assert!(core_call_hierarchy::LogicalCallTarget::capture(provider, &native, &item).is_none());
}
