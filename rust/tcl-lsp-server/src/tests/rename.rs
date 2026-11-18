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

//! Rename tiers, workspace edits and file renames.

use super::*;

#[tokio::test]
async fn namespace_coordinates_without_original_variable_symbols_do_not_start_workspace_edits() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///conditional-variable-coordinate.tcl").unwrap();
    let source = "namespace eval ::N {set value 1}";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(source.to_owned(), "tcl".to_owned()),
    );
    let mut analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    analysis.original_variable_symbols.clear();
    analysis.qualified_var_refs.clear();
    let offset = u32::try_from(source.find("value").unwrap()).unwrap();
    let position = tcl_lexer::LineIndex::new(source).position_at_utf16(offset, source);
    let position = Position::new(position.line, position.character.get());
    assert!(matches!(
        tcl_lsp_core::variable_symbol::select(source, &analysis, position.line, position.character),
        std::ops::ControlFlow::Continue(())
    ));
    assert_eq!(
        core_definition::qualified_variable_cell_at(
            source,
            analysis.resolved_profile().unwrap(),
            &analysis,
            position.line,
            position.character
        )
        .as_deref(),
        Some("::N::value")
    );
    let doc = backend.read_document(&uri).await.unwrap().unwrap();
    assert!(
        backend
            .gated_rename_tiers(&uri, &doc, &analysis, position, "changed")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn original_workspace_procedure_rename_requires_expression_edit_coverage() {
    // Implementation contract: naming.core.original-math-rename-coverage
    // docs/design/analysis/name-resolution-proofs/core-original-math-rename-coverage.md
    let backend = test_backend();
    let provider = Uri::from_str("file:///original-math-rename-provider.tcl").unwrap();
    let consumer = Uri::from_str("file:///original-math-rename-consumer.tcl").unwrap();
    let source =
        "proc ::tcl::mathfunc::local {argument} {return $argument}\n::tcl::mathfunc::local 2\n";
    register(&backend, &provider, source).await;
    let document = backend.read_document(&provider).await.unwrap().unwrap();
    let analysis = backend
        .analysis_for(&provider, document.text.clone(), document.dialect.clone())
        .await
        .unwrap();
    let position = Position::new(0, u32::try_from(source.find("local").unwrap()).unwrap());
    assert!(
        backend
            .original_procedure_expression_coverage_is_editable(
                &provider, source, &analysis, position,
            )
            .await
            .unwrap()
    );
    register(&backend, &consumer, "expr {local(1)}\n").await;
    assert!(
        !backend
            .original_procedure_expression_coverage_is_editable(
                &provider, source, &analysis, position,
            )
            .await
            .unwrap()
    );
    let parameters = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": provider.as_str()},
        "position": {"line": 0, "character": position.character},
        "newName": "changed"
    }))
    .unwrap();
    assert!(backend.rename(parameters).await.unwrap().is_none());
}

#[tokio::test]
async fn original_workspace_command_rename_uses_atomic_current_owners() {
    // Implementation contract: naming.editor.original-command-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-command-rename-plans.md
    use std::ops::ControlFlow;
    let backend = test_backend();
    let first = Uri::from_str("file:///original-command-rename.tcl").unwrap();
    let peer = Uri::from_str("file:///original-command-peer.tcl").unwrap();
    let source = r"proc p\uD800 {} {}
proc p\uD801 {} {}
p\uD800";
    register(&backend, &first, source).await;
    register(&backend, &peer, "proc unrelated {} {}").await;
    let doc = backend.read_document(&first).await.unwrap().unwrap();
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.all_procs.clear();
    let pos = Position::new(0, 7);
    assert!(matches!(
        backend
            .original_command_prepare_tier(&first, &doc, &analysis, pos,)
            .await
            .unwrap(),
        ControlFlow::Break(Some(_))
    ));
    let ControlFlow::Break(Ok(Some(edit))) = backend
        .original_command_rename_tier(&first, &doc, &analysis, pos, "renamed")
        .await
        .unwrap()
    else {
        panic!("closed current original command owners must permit the edit");
    };
    let mut changes = edit.changes.unwrap();
    assert_eq!(changes.len(), 1, "unrelated owners do not acquire edits");
    let mut edits = changes.remove(&first).unwrap();
    assert_eq!(edits.len(), 2);
    edits
        .sort_by_key(|edit| std::cmp::Reverse((edit.range.start.line, edit.range.start.character)));
    let mut after = source.to_owned();
    for edit in edits {
        let start =
            line_col_to_byte_offset(&after, edit.range.start.line, edit.range.start.character)
                .unwrap();
        let end =
            line_col_to_byte_offset(&after, edit.range.end.line, edit.range.end.character).unwrap();
        after.replace_range(start..end, &edit.new_text);
    }
    assert_eq!(after, "proc renamed {} {}\nproc p\\uD801 {} {}\nrenamed");

    register(&backend, &peer, "proc renamed {} {}").await;
    assert!(
        matches!(
            backend
                .original_command_prepare_tier(&first, &doc, &analysis, pos,)
                .await
                .unwrap(),
            ControlFlow::Break(Some(_))
        ),
        "prepare validates the current unchanged tail"
    );
    let ControlFlow::Break(Err(error)) = backend
        .original_command_rename_tier(&first, &doc, &analysis, pos, "renamed")
        .await
        .unwrap()
    else {
        panic!("a current independently owned publication must block all edits");
    };
    assert_eq!(error.data.unwrap()["code"], "original-command-collision");
    backend.documents.lock("test").await.insert(
        first.clone(),
        DocumentState::new(source.replace("D800", "D802"), "tcl8.6".to_owned()),
    );
    assert!(matches!(
        backend
            .original_command_prepare_tier(&first, &doc, &analysis, pos,)
            .await
            .unwrap(),
        ControlFlow::Break(None)
    ));
    let ControlFlow::Break(Err(error)) = backend
        .original_command_rename_tier(&first, &doc, &analysis, pos, "other")
        .await
        .unwrap()
    else {
        panic!("a stale complete source owner must withdraw its edit");
    };
    assert_eq!(error.data.unwrap()["code"], "stale-original-source");
}

#[tokio::test]
async fn original_workspace_member_rename_uses_shared_roster_and_atomic_owner_guards() {
    // Implementation contract: naming.editor.original-instance-member-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-instance-member-rename-plans.md
    use std::ops::ControlFlow;
    let backend = test_backend();
    let owner = Uri::from_str("file:///original-member-owner.tcl").unwrap();
    let peer = Uri::from_str("file:///original-member-peer.tcl").unwrap();
    let source = r"oo::class create C {method m\uD800 {} {return body}}
C create object
object m\uD800";
    register(&backend, &owner, source).await;
    let doc = backend.read_document(&owner).await.unwrap().unwrap();
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.all_classes.clear();
    analysis.global_scope.classes.clear();
    analysis.instance_classes.clear();
    let pos = Position::new(0, u32::try_from(source.find(r"m\uD800").unwrap()).unwrap());
    assert!(matches!(
        backend
            .original_command_prepare_tier(&owner, &doc, &analysis, pos)
            .await
            .unwrap(),
        ControlFlow::Break(Some(_))
    ));
    let ControlFlow::Break(Ok(Some(edit))) = backend
        .original_command_rename_tier(&owner, &doc, &analysis, pos, "changed")
        .await
        .unwrap()
    else {
        panic!("the actual current bounded instance roster must permit both edits");
    };
    let mut changes = edit.changes.unwrap();
    assert_eq!(changes.len(), 1);
    let mut edits = changes.remove(&owner).unwrap();
    assert_eq!(edits.len(), 2);
    edits
        .sort_by_key(|edit| std::cmp::Reverse((edit.range.start.line, edit.range.start.character)));
    let mut after = source.to_owned();
    for edit in edits {
        let start =
            line_col_to_byte_offset(&after, edit.range.start.line, edit.range.start.character)
                .unwrap();
        let end =
            line_col_to_byte_offset(&after, edit.range.end.line, edit.range.end.character).unwrap();
        after.replace_range(start..end, &edit.new_text);
    }
    assert_eq!(
        after,
        "oo::class create C {method changed {} {return body}}\nC create object\nobject changed"
    );
    for (tail, code) in [
        ("destroy", "original-member-collision"),
        ("Changed", "changed-original-member-visibility"),
    ] {
        let ControlFlow::Break(Err(error)) = backend
            .original_command_rename_tier(&owner, &doc, &analysis, pos, tail)
            .await
            .unwrap()
        else {
            panic!("the shared member planner must return its typed refusal");
        };
        assert_eq!(error.data.unwrap()["code"], code);
    }
    register(&backend, &peer, "proc unrelated {} {}").await;
    assert!(matches!(
        backend
            .original_command_prepare_tier(&owner, &doc, &analysis, pos)
            .await
            .unwrap(),
        ControlFlow::Break(None)
    ));
    let ControlFlow::Break(Err(error)) = backend
        .original_command_rename_tier(&owner, &doc, &analysis, pos, "changed")
        .await
        .unwrap()
    else {
        panic!("an unclosed workspace family must refuse the whole edit set");
    };
    assert_eq!(
        error.data.unwrap()["code"],
        "unclosed-original-member-family"
    );
    backend.documents.lock("test").await.insert(
        owner.clone(),
        DocumentState::new(source.replace("D800", "D802"), "tcl8.6".to_owned()),
    );
    let ControlFlow::Break(Err(error)) = backend
        .original_command_rename_tier(&owner, &doc, &analysis, pos, "changed")
        .await
        .unwrap()
    else {
        panic!("a stale owner cannot issue member edits");
    };
    assert_eq!(error.data.unwrap()["code"], "stale-original-member-source");
}

#[tokio::test]
async fn original_namespace_rename_uses_typed_workspace_collisions_and_source_plans() {
    // Implementation contract: naming.server.original-namespace-rename-plans
    // docs/design/analysis/name-resolution-proofs/server-original-namespace-rename-plans.md
    let backend = test_backend();
    let first = Uri::from_str("file:///opaque-namespace-rename-first.tcl").unwrap();
    let second = Uri::from_str("file:///opaque-namespace-rename-second.tcl").unwrap();
    let source = r"namespace eval n\uD800 {}
namespace eval n\uD801 {}
namespace exists n\uD800";
    register(&backend, &first, source).await;
    register(&backend, &second, source).await;
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let symbol =
        Backend::original_namespace_symbol(source, &analysis, Position::new(0, 16)).unwrap();
    for reference in &mut analysis.namespace_refs {
        reference.qualified_name.clear();
        reference.original_name.clear();
    }
    backend
        .workspace_index
        .write()
        .await
        .add_document(first.as_str(), &analysis);
    backend
        .workspace_index
        .write()
        .await
        .add_document(second.as_str(), &analysis);
    let doc = backend.read_document(&first).await.unwrap().unwrap();
    let workspace_edits = backend
        .cross_document_original_namespace_rename(&first, &doc, &analysis, &symbol, "renamed")
        .await
        .unwrap();
    assert_eq!(workspace_edits.len(), 2);
    for mut edits in workspace_edits.into_values() {
        assert_eq!(edits.len(), 2);
        edits.sort_by_key(|edit| {
            std::cmp::Reverse((edit.range.start.line, edit.range.start.character))
        });
        let mut changed = source.to_owned();
        for edit in edits {
            let start = line_col_to_byte_offset(
                &changed,
                edit.range.start.line,
                edit.range.start.character,
            )
            .unwrap();
            let end =
                line_col_to_byte_offset(&changed, edit.range.end.line, edit.range.end.character)
                    .unwrap();
            changed.replace_range(start..end, &edit.new_text);
        }
        assert_eq!(
            changed,
            "namespace eval renamed {}\nnamespace eval n\\uD801 {}\nnamespace exists renamed"
        );
    }
    let collision = Uri::from_str("file:///opaque-namespace-rename-collision.tcl").unwrap();
    register(&backend, &collision, "namespace eval renamed::child {}").await;
    assert!(
        backend
            .cross_document_original_namespace_rename(&first, &doc, &analysis, &symbol, "renamed",)
            .await
            .is_err(),
        "an implicit parent is an occupied proposed namespace"
    );
}

#[tokio::test]
async fn original_variable_rename_shares_source_plans_and_typed_workspace_collisions() {
    // Implementation contract: naming.server.original-variable-rename-plans
    // docs/design/analysis/name-resolution-proofs/server-original-variable-rename-plans.md
    let backend = test_backend();
    let first = Uri::from_str("file:///original-variable-rename-first.tcl").unwrap();
    let second = Uri::from_str("file:///original-variable-rename-second.tcl").unwrap();
    let source = r"set ::N::v\uD800(k) 1; set ::N::v\uD801 2; info exists ::N::v\uD800(k)";
    register(&backend, &first, source).await;
    register(&backend, &second, source).await;
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let symbol = analysis
        .original_variable_symbols
        .iter()
        .find(|row| row.is_declaration())
        .unwrap()
        .symbol()
        .clone();
    backend
        .workspace_index
        .write()
        .await
        .add_document(first.as_str(), &analysis);
    backend
        .workspace_index
        .write()
        .await
        .add_document(second.as_str(), &analysis);
    let doc = backend.read_document(&first).await.unwrap().unwrap();
    let changes = backend
        .cross_document_original_variable_rename(&first, &doc, &analysis, &symbol, "changed")
        .await
        .unwrap();
    assert_eq!(changes.len(), 2);
    for owner in [&first, &second] {
        let mut edits = changes[owner].clone();
        edits.sort_by_key(|edit| std::cmp::Reverse(edit.range.start.character));
        let mut after = source.to_owned();
        for edit in edits {
            after.replace_range(
                edit.range.start.character as usize..edit.range.end.character as usize,
                &edit.new_text,
            );
        }
        assert_eq!(
            after,
            r"set ::N::changed(k) 1; set ::N::v\uD801 2; info exists ::N::changed(k)"
        );
    }
    let collision_uri = Uri::from_str("file:///original-variable-rename-collision.tcl").unwrap();
    let collision_source = "set ::N::changed 1";
    register(&backend, &collision_uri, collision_source).await;
    let collision_analysis = Analyser::new().analyse(collision_source, "tcl8.6");
    backend
        .workspace_index
        .write()
        .await
        .add_document(collision_uri.as_str(), &collision_analysis);
    assert!(
        backend
            .cross_document_original_variable_rename(&first, &doc, &analysis, &symbol, "changed")
            .await
            .is_err()
    );
}

/// The collision refusal names the documents, and says nothing at all when
/// there is no collision.
///
/// Three at most, then a count: the reason renders as one unwrapped line in
/// some hosts, and the gate refuses on the first collision regardless, so
/// the list is there to be checkable rather than exhaustive.
#[test]
fn a_collision_refusal_names_where_the_cell_already_lives() {
    assert!(describe_variable_collision("::a", "::b", &[]).is_none());

    let one = describe_variable_collision("::a", "::b", &["file:///x.tcl".to_owned()])
        .expect("a collision refuses");
    assert!(one.contains("cannot rename `::a`"), "{one}");
    assert!(
        one.contains("`::b` is already declared in file:///x.tcl"),
        "{one}"
    );
    assert!(!one.contains("more"), "one document needs no tail: {one}");

    let many: Vec<String> = (0..5).map(|i| format!("file:///f{i}.tcl")).collect();
    let listed = describe_variable_collision("::a", "::b", &many).expect("a collision refuses");
    assert!(
        listed.contains("file:///f0.tcl, file:///f1.tcl, file:///f2.tcl"),
        "{listed}"
    );
    assert!(listed.contains("and 2 more"), "{listed}");
    assert!(
        !listed.contains("file:///f3.tcl"),
        "the tail is a count, not a list: {listed}"
    );
}

/// The same shape as
/// `cross_document_references_reach_caller_from_shadowed_duplicate_decl`,
/// but for rename — the more severe half: an LSP-presented "complete"
/// rename issued from the shadowed declaration must still rewrite the
/// cross-file caller, or accepting the edit silently resurrects the dead
/// first definition for that caller and changes real program output with
/// no error.
#[tokio::test]
async fn cross_document_rename_reaches_caller_from_shadowed_duplicate_decl() {
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib31_rename.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer31_rename.tcl").unwrap();
    let lib_src = "proc List2array {lst} { return ONE }\nproc List2array {lst} { return TWO }\n";
    register_in_dialect(&backend, &lib, lib_src, "tcl").await;
    register_in_dialect(&backend, &consumer, "List2array x\n", "tcl").await;
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(lib_src, "tcl").clone()
    };
    let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> =
        std::collections::HashMap::new();
    // Cursor on the SHADOWED (first, non-winning) declaration.
    backend
        .add_cross_document_rename_edits(
            &lib,
            lib_src,
            &analysis,
            Position::new(0, 6),
            "ListToArray",
            &mut changes,
        )
        .await
        .unwrap();
    let consumer_edits = changes.get(&consumer).cloned().unwrap_or_default();
    assert_eq!(
        consumer_edits.len(),
        1,
        "the cross-file caller must be rewritten, or it stays bound \
             to the old name while the dead shadowed definition (also \
             renamed away) silently resurrects for it: {changes:?}"
    );
    assert_eq!(consumer_edits[0].new_text, "ListToArray");
}

/// The rename half of the same shape — the severe one. An edit set that
/// drops the cross-file callers leaves them bound to the old name while
/// the *other* declaration of the very same command is still lying
/// around under it.
#[tokio::test]
async fn cross_document_rename_reaches_caller_from_nested_self_redefinition() {
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib45_rename.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer45_rename.tcl").unwrap();
    let lib_src = "namespace eval ticklecharts {}\n\
                       proc ticklecharts::activate {bool} {\n\
                       \x20if {$bool} {\n\
                       \x20 proc activate {bool} {puts \"already activated\"}\n\
                       \x20}\n\
                       }\n";
    register_in_dialect(&backend, &lib, lib_src, "tcl").await;
    register_in_dialect(
        &backend,
        &consumer,
        "ticklecharts::activate 1\nticklecharts::activate 1\n",
        "tcl",
    )
    .await;
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(lib_src, "tcl").clone()
    };
    for (line, character, which) in [(1u32, 20u32, "outer"), (3, 7, "nested")] {
        let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> =
            std::collections::HashMap::new();
        backend
            .add_cross_document_rename_edits(
                &lib,
                lib_src,
                &analysis,
                Position::new(line, character),
                "enable",
                &mut changes,
            )
            .await
            .unwrap();
        let consumer_edits = changes.get(&consumer).cloned().unwrap_or_default();
        assert_eq!(
            consumer_edits.len(),
            2,
            "both cross-file callers must be rewritten from the {which} \
                 declaration: {changes:?}"
        );
        // The caller spells the command with its namespace qualifier, and
        // rename preserves that qualifier rather than flattening the call
        // to a bare name.
        assert!(
            consumer_edits
                .iter()
                .all(|e| e.new_text == "ticklecharts::enable"),
            "the qualifier must be preserved: {consumer_edits:?}"
        );
    }
}

#[tokio::test]
async fn rename_edits_span_multiple_documents() {
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
    register(&backend, &lib, "proc helper {} {}\nhelper\n").await;
    register(&backend, &consumer, "helper\nhelper\n").await;
    let params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: lib.clone() },
            position: Position::new(0, 5),
        },
        new_name: "do_it".to_owned(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let edit = backend.rename(params).await.expect("ok").expect("some");
    let changes = edit.changes.expect("changes");
    // Both documents get edits.
    assert!(changes.contains_key(&lib), "lib edits missing: {changes:?}");
    assert!(
        changes.contains_key(&consumer),
        "consumer edits missing: {changes:?}",
    );
    // consumer.tcl: two call sites rewritten to `do_it`.
    let consumer_edits = &changes[&consumer];
    assert_eq!(consumer_edits.len(), 2, "{consumer_edits:?}");
    assert!(consumer_edits.iter().all(|e| e.new_text == "do_it"));
}

/// A rename triggered from a **consumer** document — the command's
/// definition lives only in a sibling — resolves through the workspace
/// oracle and rewrites the sibling declaration plus every call site,
/// including the consumer's own.
#[tokio::test]
async fn rename_from_a_consumer_document_rewrites_the_defining_sibling() {
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
    register(&backend, &lib, "proc helper {} {}\nhelper\n").await;
    register(&backend, &consumer, "helper\nhelper\n").await;
    let params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: consumer.clone(),
            },
            position: Position::new(0, 2),
        },
        new_name: "do_it".to_owned(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let edit = backend.rename(params).await.expect("ok").expect("some");
    let changes = edit.changes.expect("changes");
    // lib.tcl: declaration + its own call site; consumer.tcl: both calls.
    assert_eq!(
        changes.get(&lib).map(Vec::len),
        Some(2),
        "lib decl + call: {changes:?}"
    );
    assert_eq!(
        changes.get(&consumer).map(Vec::len),
        Some(2),
        "consumer call sites: {changes:?}"
    );
}

/// Collision discipline holds on the consumer-document path: renaming onto
/// a name the workspace already defines is refused wholesale — no partial
/// edit set leaks out.
#[tokio::test]
async fn rename_from_a_consumer_document_refuses_a_workspace_collision() {
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
    register(&backend, &lib, "proc helper {} {}\nproc do_it {} {}\n").await;
    register(&backend, &consumer, "helper\n").await;
    let params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: consumer.clone(),
            },
            position: Position::new(0, 2),
        },
        new_name: "do_it".to_owned(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let edit = backend.rename(params).await.expect("ok");
    assert!(
        edit.is_none(),
        "a collision with an existing workspace command must refuse: {edit:?}"
    );
}

#[tokio::test]
async fn rename_to_builtin_blocked_cross_document() {
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
    register(&backend, &lib, "proc helper {} {}\n").await;
    register(&backend, &consumer, "helper\n").await;
    let params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: lib.clone() },
            position: Position::new(0, 5),
        },
        // `if` is a built-in command — renaming to it must be
        // refused (no edits in any document).
        new_name: "if".to_owned(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let result = backend.rename(params).await.expect("ok");
    assert!(result.is_none(), "rename to a built-in should be refused");
}

#[tokio::test]
async fn will_rename_rewrites_dependent_source_literal() {
    let backend = test_backend();
    let main = Uri::from_str("file:///proj/main.tcl").unwrap();
    // main.tcl sources lib/old.tcl via a relative literal.
    register(&backend, &main, "source lib/old.tcl\nputs hi\n").await;
    let params = RenameFilesParams {
        files: vec![tower_lsp_server::ls_types::FileRename {
            old_uri: "file:///proj/lib/old.tcl".to_owned(),
            new_uri: "file:///proj/lib/new.tcl".to_owned(),
        }],
    };
    let edit = backend
        .will_rename_files(params)
        .await
        .expect("ok")
        .expect("some");
    let DocumentChanges::Edits(doc_edits) = edit.document_changes.expect("document_changes") else {
        panic!("expected Edits");
    };
    assert_eq!(doc_edits.len(), 1);
    assert_eq!(doc_edits[0].text_document.uri, main);
    assert_eq!(doc_edits[0].edits.len(), 1);
    let OneOf::Left(text_edit) = &doc_edits[0].edits[0] else {
        panic!("expected plain TextEdit");
    };
    assert_eq!(text_edit.new_text, "lib/new.tcl");
    // The edit range covers the original `lib/old.tcl` literal on
    // the first line.
    assert_eq!(text_edit.range.start.line, 0);
}

#[tokio::test]
async fn will_rename_retains_client_version_and_literal_source_value() {
    let backend = test_backend();
    let uri: Uri = "file:///project/main.tcl".parse().unwrap();
    let source = "source \"lib/old.tcl\"\n";
    Box::pin(backend.did_open(DidOpenTextDocumentParams {
        text_document: tower_lsp_server::ls_types::TextDocumentItem {
            uri: uri.clone(),
            language_id: "tcl".to_owned(),
            version: 17,
            text: source.to_owned(),
        },
    }))
    .await;
    let result = backend
        .will_rename_files(RenameFilesParams {
            files: vec![tower_lsp_server::ls_types::FileRename {
                old_uri: "file:///project/lib/old.tcl".to_owned(),
                new_uri: "file:///project/lib/new%20%24v%5Bunknown%5D%3B.tcl".to_owned(),
            }],
        })
        .await
        .unwrap()
        .expect("actual source action");
    let DocumentChanges::Edits(edits) = result.document_changes.unwrap() else {
        panic!("document edits");
    };
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].text_document.version, Some(17));
    let OneOf::Left(edit) = &edits[0].edits[0] else {
        panic!("plain edit");
    };
    let start = tcl_lexer::LineIndex::new(source).offset_at_utf16(
        edit.range.start.line,
        tcl_lexer::Utf16Col::new(edit.range.start.character),
        source,
    );
    let end = tcl_lexer::LineIndex::new(source).offset_at_utf16(
        edit.range.end.line,
        tcl_lexer::Utf16Col::new(edit.range.end.character),
        source,
    );
    let mut changed = source.to_owned();
    changed.replace_range(start as usize..end as usize, &edit.new_text);
    let document = backend.read_document(&uri).await.unwrap().unwrap();
    let analysis = backend
        .analysis_for(&uri, document.text, document.dialect)
        .await
        .unwrap();
    let config = analysis.body_lexer_config.unwrap();
    let image = tcl_lexer::SourceImage::document(&changed);
    let plan = tcl_lexer::native_script_words_in(
        image,
        tcl_lexer::Span::new(0, u32::try_from(changed.len()).unwrap()),
        config,
    )
    .unwrap();
    assert!(plan.fatal_tail.is_none());
    assert_eq!(plan.commands.len(), 1);
    assert_eq!(plan.commands[0].words.len(), 2);
    assert_eq!(
        tcl_syntax::word_rules::original_static_word_unicode_value(&plan.commands[0].words[1])
            .as_deref(),
        Some("lib/new $v[unknown];.tcl".as_bytes())
    );
}

#[tokio::test]
async fn will_rename_returns_none_without_dependents() {
    let backend = test_backend();
    let main = Uri::from_str("file:///proj/main.tcl").unwrap();
    register(&backend, &main, "source other.tcl\n").await;
    let params = RenameFilesParams {
        files: vec![tower_lsp_server::ls_types::FileRename {
            old_uri: "file:///proj/lib/old.tcl".to_owned(),
            new_uri: "file:///proj/lib/new.tcl".to_owned(),
        }],
    };
    // Nothing sources lib/old.tcl → no edit.
    assert!(
        backend
            .will_rename_files(params)
            .await
            .expect("ok")
            .is_none()
    );
}

#[tokio::test]
async fn did_rename_reindexes_moved_document() {
    let backend = test_backend();
    let old = Uri::from_str("file:///gone.tcl").unwrap();
    let src = "proc helper {} {}\n";
    register(&backend, &old, src).await;
    // A renamed-away path is not an open buffer — the client moved the file
    // on disk — so drop the live document `register` seeded and stand the
    // rest of the old URI's per-URI state up as a real session would have.
    backend.documents.lock("test").await.remove(&old);
    backend
        .db_set_source(&old, src, "tcl8.6".to_owned())
        .await
        .unwrap();
    let old_file = *backend
        .db_files
        .lock()
        .await
        .get(&old)
        .expect("precondition: the old URI has a salsa source");
    backend
        .last_semantic_tokens
        .lock()
        .await
        .insert(old.clone(), ("st-1".to_owned(), Vec::new()));
    backend.workspace_class_analyses.lock().await.insert(
        old.clone(),
        WorkspaceClassAnalysis {
            fingerprint: (0, 0, 0, 0),
            source_analysis: None,
            resource_inputs: None,
            analysis: Arc::default(),
        },
    );
    backend
        .diag_slots
        .lock()
        .await
        .insert(old.clone(), DiagSlot::default());
    backend.pull_diag_cache.lock().await.insert(
        old.clone(),
        PullDiagEntry {
            currency: None,
            result_id: next_pull_diag_result_id(),
            revision: 0,
            diagnostics: Vec::new(),
        },
    );
    backend.closed_diag_gen.lock().await.insert(old.clone(), 3);
    assert_eq!(backend.workspace_index.read().await.procs().count(), 1);

    let params = RenameFilesParams {
        files: vec![tower_lsp_server::ls_types::FileRename {
            old_uri: old.to_string(),
            // New path doesn't exist on disk (test env) — reindex
            // drops the stale old entry and finds nothing to add.
            new_uri: "file:///moved.tcl".to_owned(),
        }],
    };
    backend.did_rename_files(params).await;

    // The old document's proc is gone from the index.
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .procs()
            .all(|p| p.uri != old.as_str()),
    );
    // …and out of the salsa db, so the renamed file's procedures are
    // not counted twice in cross-file resolution (and the old URI's memo
    // chain is not retained for the process's life).
    assert!(
        !backend.db_files.lock().await.contains_key(&old),
        "the old URI's salsa SourceFile must be dropped",
    );
    let project_holds_old = {
        let db = backend.db.lock().await;
        backend
            .db_project
            .lock()
            .await
            .is_some_and(|project| project.files(&*db).contains(&old_file))
    };
    assert!(
        !project_holds_old,
        "the old URI must leave the salsa Project file set",
    );
    // …along with every per-URI cache keyed on the dead path, and its badge.
    assert!(!backend.pull_diag_cache.lock().await.contains_key(&old));
    assert!(!backend.closed_diag_gen.lock().await.contains_key(&old));
    assert!(!backend.diag_slots.lock().await.contains_key(&old));
    assert!(!backend.last_semantic_tokens.lock().await.contains_key(&old));
    assert!(
        !backend
            .workspace_class_analyses
            .lock()
            .await
            .contains_key(&old)
    );
}

#[tokio::test]
async fn logical_namespace_rename_collision_uses_constructed_colon_holder() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    // Genuine current Logical documents; reporting keys grant no Native path.
    let backend = test_backend();
    let uri = Uri::from_str("file:///logical-colon-namespace.tcl").unwrap();
    let collision = Uri::from_str("file:///logical-colon-namespace-collision.tcl").unwrap();
    let source = "namespace eval 茶 {namespace eval : {}}";
    register_logical_hierarchy(&backend, &uri, source).await;
    register_logical_hierarchy(
        &backend,
        &collision,
        "namespace eval 茶 {namespace eval box {}}",
    )
    .await;
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    let at = source.find("eval :").unwrap() + "eval ".len();
    let point =
        tcl_lexer::LineIndex::new(source).position_at_utf16(u32::try_from(at).unwrap(), source);
    let position = Position::new(point.line, point.character.get());
    assert!(Backend::original_namespace_symbol(source, &analysis, position).is_none());
    assert_eq!(
        Backend::namespace_cell(source, &analysis, position).as_deref(),
        Some("::茶:::")
    );
    let document = backend.read_document(&uri).await.unwrap().unwrap();
    let refusal = backend
        .cross_document_namespace_rename(&uri, &document, &analysis, position, "box")
        .await
        .unwrap_err();
    assert!(refusal.reason.contains("::茶::box"));
    assert!(refusal.reason.contains("already exists"));
}
