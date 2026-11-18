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

use std::collections::HashSet;
use std::sync::Arc;

use tower_lsp_server::ls_types::Uri;

use super::OriginalSourceViewSettlement;
use crate::Backend;

use crate::{DocumentPublication, DocumentState};

struct Fixture {
    root: crate::tests::ScratchDir,
    main: Uri,
    host: Uri,
    source: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

async fn open(backend: &Backend, uri: &Uri, source: &str) {
    backend
        .db_set_source(uri, source, "tcl8.6".to_owned())
        .await
        .unwrap();
    let mut document = DocumentState::with_version(source.to_owned(), "tcl8.6".to_owned(), 1);
    document.path_admission = Some(backend.document_path_admission(uri).await.unwrap());
    document.publication = DocumentPublication::Indexed;
    backend
        .documents
        .lock("completed world owner fixture")
        .await
        .insert(uri.clone(), document);
}

async fn fixture(opaque: bool) -> (Fixture, Backend, Vec<crate::OriginalDocumentSnapshot>) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = crate::tests::ScratchDir::at(std::env::temp_dir().join(format!(
        "tcl-lsp-completed-owner-{}-{nonce}",
        std::process::id()
    )));
    std::fs::create_dir(&root).unwrap();
    let child = root.join("child.tcl");
    std::fs::write(&child, "proc helper {value} {return $value}\n").unwrap();
    let source = if opaque {
        let source =
            "proc host_old {value} { return $value }\nputs [nop \"🧪\"]; puts [host_old 7]\n";
        assert_eq!(source.len(), 77);
        source.to_owned()
    } else {
        format!(
            "source {{{}}}\nproc independent {{}} {{return [helper 21]}}\n",
            child.display()
        )
    };
    let host_source = "proc selected {} {return 42}\n";
    std::fs::write(root.join("main.tcl"), &source).unwrap();
    std::fs::write(root.join("host.tcl"), host_source).unwrap();
    let main = Uri::from_file_path(root.join("main.tcl")).unwrap();
    let host = Uri::from_file_path(root.join("host.tcl")).unwrap();
    let backend = crate::tests::test_backend();
    // Production admits closed workspace sources through the folder scan.
    *backend.workspace_folders.lock().await = vec![Uri::from_file_path(&root).unwrap()];
    backend.scan_workspace_folders().await.unwrap();
    open(&backend, &main, &source).await;
    open(&backend, &host, host_source).await;
    let documents = backend.original_declaration_documents(&host).await.unwrap();
    (
        Fixture {
            root,
            main,
            host,
            source,
        },
        backend,
        documents,
    )
}

#[tokio::test]
async fn actual_prefix_owner_is_selected_without_renewing_the_old_analysis() {
    let (fixture, backend, documents) = fixture(false).await;
    let original = documents
        .iter()
        .find(|row| row.0 == fixture.main.as_str())
        .unwrap()
        .2
        .clone();
    assert!(original.original_completed_command_world().is_none());
    let (owned, capture) = backend
        .command_source_documents(false, documents, fixture.host.as_str())
        .await
        .unwrap();
    let capture = capture.unwrap();
    let analysis = &owned
        .iter()
        .find(|row| row.0 == fixture.main.as_str())
        .unwrap()
        .2;
    let worlds = capture.worlds();
    let outcome = worlds
        .iter()
        .find(|row| row.uri == fixture.main.as_str())
        .unwrap()
        .outcome;
    assert!(outcome.original_completed_command_world(analysis).is_some());
    assert!(
        outcome
            .original_completed_command_world(&original)
            .is_none()
    );
    assert!(!Arc::ptr_eq(&original, analysis));
    assert!(original.original_completed_command_world().is_none());
    assert!(
        backend
            .original_command_source_worlds_are_current(&capture)
            .await
            .unwrap()
    );
    std::fs::write(
        fixture.root.join("child.tcl"),
        "proc changed {} {return OTHER}\n",
    )
    .unwrap();
    assert!(
        !backend
            .original_command_source_worlds_are_current(&capture)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn foreign_input_source_uri_and_selected_owner_keep_their_refusals() {
    let (fixture, backend, documents) = fixture(false).await;
    let index = documents
        .iter()
        .position(|row| row.0 == fixture.main.as_str())
        .unwrap();
    let original = Arc::clone(&documents[index].2);
    let (selected, _) = backend
        .command_source_documents(false, documents.clone(), fixture.main.as_str())
        .await
        .unwrap();
    assert!(Arc::ptr_eq(&selected[index].2, &original));
    let (members, capture) = backend
        .command_source_documents(true, documents.clone(), fixture.host.as_str())
        .await
        .unwrap();
    assert!(capture.is_none() && Arc::ptr_eq(&members[index].2, &original));
    let mut foreign = documents.clone();
    foreign[index].2 =
        Arc::new(tcl_compiler::analyser::Analyser::new().analyse(&fixture.source, "tcl9.0"));
    let retained = Arc::clone(&foreign[index].2);
    let (refused, capture) = backend
        .command_source_documents(false, foreign, fixture.host.as_str())
        .await
        .unwrap();
    assert!(capture.is_none() && Arc::ptr_eq(&refused[index].2, &retained));
    let mut wrong_source = documents.clone();
    wrong_source[index].1 = Arc::from("different source");
    let (refused, capture) = backend
        .command_source_documents(false, wrong_source, fixture.host.as_str())
        .await
        .unwrap();
    assert!(capture.is_none() && Arc::ptr_eq(&refused[index].2, &original));
    let mut wrong_uri = documents;
    wrong_uri[index].0 = Uri::from_file_path(fixture.root.join("other.tcl"))
        .unwrap()
        .as_str()
        .to_owned();
    let (refused, _) = backend
        .command_source_documents(false, wrong_uri, fixture.host.as_str())
        .await
        .unwrap();
    assert!(Arc::ptr_eq(&refused[index].2, &original));
}

#[tokio::test]
async fn exact_original_opaque_source_never_borrows_a_peer_world() {
    let (fixture, backend, documents) = fixture(true).await;
    let original = documents
        .iter()
        .find(|row| row.0 == fixture.main.as_str())
        .unwrap()
        .2
        .clone();
    let (owned, capture) = backend
        .command_source_documents(false, documents, fixture.host.as_str())
        .await
        .unwrap();
    let analysis = &owned
        .iter()
        .find(|row| row.0 == fixture.main.as_str())
        .unwrap()
        .2;
    assert!(Arc::ptr_eq(analysis, &original));
    assert!(analysis.original_completed_command_world().is_none());
    if let Some(capture) = capture {
        assert!(
            capture
                .worlds()
                .iter()
                .all(|row| row.uri != fixture.main.as_str())
        );
    }
}

const REFERENCE_SOURCE: &str = "proc host_old {value} { return $value }\nset host_marker \"🧪 café\"; proc host_caller {} {return [host_old 7]}\n";

async fn reference_fixture(source_host: bool, tail: &str, host_source: &str) -> (Fixture, Backend) {
    let (mut fixture, backend, documents) = fixture(false).await;
    drop(documents);
    let sourced = fixture
        .root
        .join(if source_host { "host.tcl" } else { "child.tcl" });
    fixture.source = format!("source {{{}}}\n{tail}", sourced.display());
    std::fs::write(fixture.root.join("main.tcl"), &fixture.source).unwrap();
    std::fs::write(fixture.root.join("host.tcl"), host_source).unwrap();
    open(&backend, &fixture.main, &fixture.source).await;
    open(&backend, &fixture.host, host_source).await;
    // Match the live navigation fixture: Indexed alone supplies no root.
    // Publish each actual current source/input analysis to the real index.
    for (uri, source) in [
        (&fixture.main, fixture.source.as_str()),
        (&fixture.host, host_source),
    ] {
        let analysis = backend
            .analysis_for(uri, Arc::from(source), "tcl8.6".to_owned())
            .await
            .unwrap();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
    }
    (fixture, backend)
}

type ReferenceScopeCensusRow = (bool, bool, bool, bool, bool, usize, bool, usize, usize);
fn reference_scope_census(
    settlement: &OriginalSourceViewSettlement,
    fixture: &Fixture,
) -> Vec<ReferenceScopeCensusRow> {
    settlement
        .scopes
        .iter()
        .take(8)
        .map(|scope| {
            (
                scope.root_uri == fixture.main,
                scope.root_uri == fixture.host,
                scope.root.is_some(),
                scope.complete,
                scope.conditional_obligations,
                scope.ids.len(),
                scope.required.contains(&fixture.host),
                scope.required.len(),
                scope.association_owners.len(),
            )
        })
        .collect()
}

fn assert_reference_locations(host: &Uri, locations: &[tower_lsp_server::ls_types::Location]) {
    assert_eq!(locations.len(), 2);
    for (line, start, end) in [(0, 5, 13), (1, 56, 64)] {
        assert!(locations.iter().any(|location| &location.uri == host
            && location.range.start.line == line
            && location.range.end.line == line
            && location.range.start.character == start
            && location.range.end.character == end));
    }
}

#[tokio::test]
async fn unfinished_suffix_definition_uses_only_its_owned_region() {
    let (fixture, backend) = reference_fixture(
        true,
        "proc main {} {helper}\nmain\n",
        "proc helper {} {return 8}\n",
    )
    .await;
    let settlement = backend
        .refresh_original_source_view_graph()
        .await
        .unwrap()
        .unwrap();
    let scope = settlement
        .scopes
        .iter()
        .find(|scope| scope.root_uri == fixture.main)
        .unwrap();
    assert!(!scope.complete);
    assert!(scope.partial_definition.is_some());
    assert!(settlement.projection(&fixture.main, false).is_none());
    assert!(
        settlement
            .positive_reference_projection(&fixture.main)
            .is_none()
    );
    let partial_projection = settlement.definition_projection(&fixture.main).unwrap();
    let rejected_prefix = backend
        .with_settled_original_source_scope(&partial_projection, |views| {
            Backend::calculate_partial_original_procedure_hover(
                views,
                &fixture.main,
                tower_lsp_server::ls_types::Position {
                    line: 0,
                    character: 0,
                },
                &partial_projection.partial_definition_groups,
            )
        })
        .await
        .unwrap();
    assert!(rejected_prefix.is_none());
    let position = tower_lsp_server::ls_types::Position {
        line: 1,
        character: 15,
    };
    let result = backend
        .original_source_view_navigation_route(&fixture.main, position, None)
        .await
        .unwrap();
    let std::ops::ControlFlow::Break(Some(locations)) = result else {
        panic!("definition unavailable")
    };
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].uri, fixture.host);
    assert_eq!(locations[0].range.start.line, 0);
    assert_eq!(locations[0].range.start.character, 5);
    let hover = backend
        .original_source_view_hover_route(&fixture.main, position)
        .await
        .unwrap();
    let std::ops::ControlFlow::Break(Some(hover)) = hover else {
        panic!("partial procedure hover unavailable")
    };
    assert!(hover.value.contains("proc ::helper {}"));
    assert!(hover.value.contains("Conditional source declaration"));
    assert!(matches!(
        backend
            .original_source_view_navigation_route(&fixture.main, position, Some(true))
            .await
            .unwrap(),
        std::ops::ControlFlow::Break(None)
    ));
    // A cursor on the source prefix is not owned by the issued suffix.
    let prefix = tower_lsp_server::ls_types::Position {
        line: 0,
        character: 0,
    };
    assert!(matches!(
        backend
            .original_source_view_navigation_route(&fixture.main, prefix, None)
            .await
            .unwrap(),
        std::ops::ControlFlow::Break(None)
    ));
}

#[tokio::test]
async fn unfinished_suffix_hover_rejoins_current_source_inputs() {
    let (fixture, backend) = reference_fixture(
        true,
        "proc main {} {helper}\nmain\n",
        "proc helper {} {return 8}\n",
    )
    .await;
    let settlement = backend
        .refresh_original_source_view_graph()
        .await
        .unwrap()
        .unwrap();
    let projection = settlement.definition_projection(&fixture.main).unwrap();
    let uri = fixture.main.clone();
    let groups = projection.partial_definition_groups.clone();
    let (owners, output) = backend
        .compute_original_source_views(&projection, move |views| {
            Some(Backend::calculate_partial_original_procedure_hover(
                views,
                &uri,
                tower_lsp_server::ls_types::Position {
                    line: 1,
                    character: 15,
                },
                &groups,
            ))
        })
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        output,
        Some(std::ops::ControlFlow::Break(Some(_)))
    ));
    backend.invalidate_diag_inputs();
    assert!(
        backend
            .with_settled_original_source_scope(&projection, |views| {
                Backend::same_original_source_view_inventory(&owners, views).then_some(())
            })
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn unfinished_suffix_definition_rejects_unknown_body_mutation() {
    let (fixture, backend) = reference_fixture(
        true,
        "proc main {} {opaque; helper}\nmain\n",
        "proc helper {} {return 8}\n",
    )
    .await;
    let position = tower_lsp_server::ls_types::Position {
        line: 1,
        character: 23,
    };
    assert!(matches!(
        backend
            .original_source_view_hover_route(&fixture.main, position)
            .await
            .unwrap(),
        std::ops::ControlFlow::Break(None)
    ));
    assert!(matches!(
        backend
            .original_source_view_navigation_route(&fixture.main, position, None)
            .await
            .unwrap(),
        std::ops::ControlFlow::Break(None)
    ));
}

#[tokio::test]
async fn positive_references_from_root_follow_its_source_view() {
    let (fixture, backend) = reference_fixture(true, "host_old 9\n", REFERENCE_SOURCE).await;
    let route = backend
        .original_source_view_navigation_route(
            &fixture.main,
            tower_lsp_server::ls_types::Position {
                line: 1,
                character: 2,
            },
            Some(true),
        )
        .await
        .unwrap();
    let locations = match route {
        std::ops::ControlFlow::Break(Some(locations)) => locations,
        other => panic!("source-view references unavailable: {other:?}"),
    };
    assert!(
        locations
            .iter()
            .any(|location| location.uri == fixture.main && location.range.start.line == 1)
    );
    assert_reference_locations(
        &fixture.host,
        &locations
            .iter()
            .filter(|location| location.uri == fixture.host)
            .cloned()
            .collect::<Vec<_>>(),
    );
}

#[tokio::test]
async fn positive_references_keep_all_live_views_without_unrelated_completion() {
    assert_eq!(REFERENCE_SOURCE.len(), 112);
    let (fixture, backend) = reference_fixture(false, "puts opaque\n", REFERENCE_SOURCE).await;
    let settlement = backend
        .refresh_original_source_view_graph()
        .await
        .unwrap()
        .unwrap();
    assert!(
        settlement
            .scopes
            .iter()
            .any(|scope| scope.root_uri == fixture.main && !scope.complete && scope.ids.len() > 1),
        "scope_count={} census(main,host,published,complete,conditional,ids,host_required,required,associations)={:?}",
        settlement.scopes.len(),
        reference_scope_census(&settlement, &fixture),
    );
    assert!(settlement.projection(&fixture.host, true).is_none());
    let projection = settlement
        .positive_reference_projection(&fixture.host)
        .unwrap();
    let expected = settlement
        .scopes
        .iter()
        .flat_map(|scope| scope.ids.iter().copied())
        .collect::<HashSet<_>>();
    assert_eq!(
        projection.ids.iter().copied().collect::<HashSet<_>>(),
        expected
    );
    assert_eq!(projection.revision, settlement.revision);
    let route = backend
        .original_source_view_navigation_route(
            &fixture.host,
            tower_lsp_server::ls_types::Position {
                line: 0,
                character: 6,
            },
            Some(true),
        )
        .await
        .unwrap();
    let std::ops::ControlFlow::Break(Some(locations)) = route else {
        panic!("positive reference inventory unavailable")
    };
    assert_reference_locations(&fixture.host, &locations);
}

#[tokio::test]
async fn positive_references_refuse_selected_and_dependent_unknown_mutation() {
    for source_host in [false, true] {
        let host = if source_host {
            REFERENCE_SOURCE
        } else {
            "proc host_old {value} {return $value}\neval [gets stdin]\nhost_old 7\n"
        };
        let (fixture, backend) =
            reference_fixture(source_host, "eval [gets stdin]\nhost_old 9\n", host).await;
        let settlement = backend
            .refresh_original_source_view_graph()
            .await
            .unwrap()
            .unwrap();
        assert!(
            settlement
                .scopes
                .iter()
                .any(|scope| scope.required.contains(&fixture.host)
                    && (!scope.complete || scope.root_world_uncertain)),
            "source_host={source_host} scope_count={} census(main,host,published,complete,conditional,ids,host_required,required,associations)={:?}",
            settlement.scopes.len(),
            reference_scope_census(&settlement, &fixture),
        );
        assert!(
            settlement
                .positive_reference_projection(&fixture.host)
                .is_none()
        );
        assert!(matches!(
            backend
                .original_source_view_navigation_route(
                    &fixture.host,
                    tower_lsp_server::ls_types::Position {
                        line: 0,
                        character: 6
                    },
                    Some(true)
                )
                .await
                .unwrap(),
            std::ops::ControlFlow::Break(None)
        ));
    }
}

#[tokio::test]
async fn positive_references_refuse_foreign_publication_and_changed_input() {
    let (fixture, backend) = reference_fixture(false, "puts opaque\n", REFERENCE_SOURCE).await;
    let settlement = backend
        .refresh_original_source_view_graph()
        .await
        .unwrap()
        .unwrap();
    let projection = settlement
        .positive_reference_projection(&fixture.host)
        .unwrap();
    let checked = backend
        .with_settled_original_source_scope(&projection, |views| {
            let (publications, ancestors) = Backend::original_source_view_labels(views)?;
            let inventory =
                Backend::original_source_view_inventory(views, &publications, &ancestors);
            let cursor = tcl_lsp_core::original_source_views::cursor_publications(
                fixture.host.as_str(),
                6,
                &inventory,
            )?;
            let selected =
                tcl_lsp_core::original_source_views::prepare(cursor.first()?, 6, &inventory)?;
            assert_eq!(
                tcl_lsp_core::original_source_views::references(&selected, &inventory).len(),
                1
            );
            let foreign = tcl_lsp_core::original_source_views::OriginalSourceViewDeclaration {
                publication:
                    tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(
                        "foreign publication",
                    ),
                identity: selected.identity.clone(),
            };
            assert!(
                tcl_lsp_core::original_source_views::references(&foreign, &inventory).is_empty()
            );
            Some(())
        })
        .await
        .unwrap();
    assert!(checked.is_some());
    backend
        .db_set_source(&fixture.host, REFERENCE_SOURCE, "tcl9.0".to_owned())
        .await
        .unwrap();
    let mut document =
        DocumentState::with_version(REFERENCE_SOURCE.to_owned(), "tcl9.0".to_owned(), 1);
    document.path_admission = Some(
        backend
            .document_path_admission(&fixture.host)
            .await
            .unwrap(),
    );
    document.publication = DocumentPublication::Indexed;
    backend
        .documents
        .lock("foreign reference input")
        .await
        .insert(fixture.host.clone(), document);
    assert!(
        backend
            .with_settled_original_source_scope(&projection, |_| Some(()))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn positive_references_keep_unrelated_child_physical_lease() {
    let (fixture, backend) = reference_fixture(false, "puts opaque\n", REFERENCE_SOURCE).await;
    let settlement = backend
        .refresh_original_source_view_graph()
        .await
        .unwrap()
        .unwrap();
    let projection = settlement
        .positive_reference_projection(&fixture.host)
        .unwrap();
    assert!(
        backend
            .with_settled_original_source_scope(&projection, |_| Some(()))
            .await
            .unwrap()
            .is_some()
    );
    std::fs::write(
        fixture.root.join("child.tcl"),
        "proc changed {} {return OTHER}\n",
    )
    .unwrap();
    assert!(
        backend
            .with_settled_original_source_scope(&projection, |_| Some(()))
            .await
            .unwrap()
            .is_none(),
        "projection_ids={} scope_count={} census(main,host,published,complete,conditional,ids,host_required,required,associations)={:?}",
        projection.ids.len(),
        settlement.scopes.len(),
        reference_scope_census(&settlement, &fixture),
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("host.tcl")).unwrap(),
        REFERENCE_SOURCE
    );
}
