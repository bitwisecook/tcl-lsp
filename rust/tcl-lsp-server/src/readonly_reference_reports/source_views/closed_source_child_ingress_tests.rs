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
use crate::{DocumentPublication, DocumentState, RehomingDocumentSnapshot};
use futures_util::{SinkExt, StreamExt};
use tower::{Service, ServiceExt};
use tower_lsp_server::{
    ExitedError, LspService,
    jsonrpc::{Request, Response},
    ls_types::Position,
};

const CALLER: &str = "source child.tcl\nproc caller {} {return [helper 7]}\n";
const CHILD: &str = "proc helper {value} {return $value}\n";
const EXACT_APP: &str = include_str!("../../../../tcl-lsp-server-wasi/test/fixture/app.tcl");
const EXACT_LIB: &str = include_str!("../../../../tcl-lsp-server-wasi/test/fixture/lib.tcl");

struct Fixture {
    root: std::path::PathBuf,
    main: Uri,
    child: Uri,
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
    let admission = backend.document_path_admission(uri).await.unwrap();
    let mut document = DocumentState::with_version(source.to_owned(), "tcl8.6".to_owned(), 1);
    document.path_admission = Some(admission);
    document.publication = DocumentPublication::Indexed;
    backend
        .documents
        .lock("closed source child fixture")
        .await
        .insert(uri.clone(), document);
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

async fn fixture() -> (Fixture, Backend) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-source-child-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("main.tcl"), CALLER).unwrap();
    std::fs::write(root.join("child.tcl"), CHILD).unwrap();
    let main = Uri::from_file_path(root.join("main.tcl")).unwrap();
    let child = Uri::from_file_path(root.join("child.tcl")).unwrap();
    let backend = crate::tests::test_backend();
    open(&backend, &main, CALLER).await;
    (Fixture { root, main, child }, backend)
}

async fn child_view(fixture: &Fixture, backend: &Backend) -> Arc<PublishedOriginalSourceView> {
    let route = backend
        .original_source_view_navigation_route(&fixture.main, Position::new(1, 24), None)
        .await
        .unwrap();
    let std::ops::ControlFlow::Break(Some(locations)) = route else {
        panic!("expected inherited source navigation")
    };
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].uri, fixture.child);
    let table = backend.original_source_views.lock().unwrap();
    let ids = table.by_uri.get(&fixture.child).unwrap();
    let mut views = ids
        .iter()
        .filter_map(|id| table.by_id.get(id))
        .filter(|view| view.draft.inherited.is_some());
    let view = Arc::clone(views.next().unwrap());
    assert!(views.next().is_none());
    view
}

async fn publication_parent(
    fixture: &Fixture,
    backend: &Backend,
) -> Arc<PublishedOriginalSourceView> {
    let document = backend.read_document(&fixture.main).await.unwrap().unwrap();
    let draft = backend
        .capture_initial_original_source_root(&fixture.main, document)
        .await
        .unwrap()
        .unwrap();
    let blocked = backend.rehoming_gate.lock().await;
    assert!(matches!(
        backend
            .publish_original_source_views(&fixture.main, vec![draft.clone()])
            .await
            .unwrap(),
        OriginalSourceViewAttempt::RetryCapture
    ));
    drop(blocked);
    let OriginalSourceViewAttempt::Ready(mut views) = backend
        .publish_original_source_views(&fixture.main, vec![draft])
        .await
        .unwrap()
    else {
        panic!("current root publication after contention")
    };
    assert_eq!(views.len(), 1);
    views.remove(0)
}

async fn inherited_draft(
    fixture: &Fixture,
    backend: &Backend,
    parent: &Arc<PublishedOriginalSourceView>,
) -> OriginalSourceViewDraft {
    let edges =
        tcl_lsp_core::workspace_index::WorkspaceOriginalSourceView::source_edges_for_analysis(
            fixture.main.as_str(),
            &parent.draft.physical.source,
            &parent.draft.producer.seed.analysis,
            None,
        )
        .unwrap();
    let edge = edges
        .iter()
        .find(|edge| {
            matches!(
                edge.original_source_frontier.as_deref(),
                Some(SourceFrontier::CurrentCut(_))
            )
        })
        .unwrap();
    let Some(SourceFrontier::CurrentCut(cut)) = edge.original_source_frontier.as_deref() else {
        unreachable!()
    };
    let RehomingDocumentSnapshot::Ready(document) = backend
        .source_child_document_snapshot(&fixture.child)
        .await
        .unwrap()
    else {
        panic!("actual closed child publication")
    };
    let OriginalSourceViewAttempt::Ready(draft) = backend
        .capture_inherited_original_source_view(
            &fixture.child,
            document.text,
            Arc::clone(parent),
            Arc::new(cut.clone()),
        )
        .await
        .unwrap()
    else {
        panic!("genuine inherited draft")
    };
    draft
}

#[tokio::test]
async fn source_publication_contention_retries_root_child_and_region_leases() {
    let (fixture, backend) = fixture().await;
    let parent = publication_parent(&fixture, &backend).await;
    let draft = inherited_draft(&fixture, &backend, &parent).await;
    let blocked = backend.rehoming_gate.lock().await;
    assert!(matches!(
        backend
            .append_inherited_original_source_view(&fixture.child, draft)
            .await
            .unwrap(),
        OriginalSourceViewAttempt::RetryCapture
    ));
    drop(blocked);
    let child = child_view(&fixture, &backend).await;
    assert!(child.draft.inherited.is_some());
    let request = {
        let table = backend.original_source_views.lock().unwrap();
        table
            .by_id
            .values()
            .find_map(|view| view.draft.region.clone())
            .expect("issued actual source region")
    };
    let OriginalSourceViewAttempt::Ready(draft) = backend
        .capture_conditional_region_draft(request)
        .await
        .unwrap()
    else {
        panic!("actual issued region draft")
    };
    let blocked = backend.rehoming_gate.lock().await;
    assert!(matches!(
        backend
            .append_original_source_regions(&fixture.main, vec![draft])
            .await
            .unwrap(),
        OriginalSourceViewAttempt::RetryCapture
    ));
    drop(blocked);
    let route = backend
        .original_source_view_navigation_route(&fixture.main, Position::new(1, 24), None)
        .await
        .unwrap();
    assert!(
        matches!(route, std::ops::ControlFlow::Break(Some(ref locations)) if locations.iter().any(|location| location.uri == fixture.child))
    );
}

#[tokio::test]
async fn child_publication_retries_lost_parent_index_membership_without_relaxing_it() {
    let (fixture, backend) = fixture().await;
    let parent = publication_parent(&fixture, &backend).await;
    let draft = inherited_draft(&fixture, &backend, &parent).await;
    // The ordinary diagnostic publisher replaces this presentation through
    // the genuine index API. The source table still holds its old row.
    backend
        .workspace_index
        .write()
        .await
        .replace_document(fixture.main.as_str(), &parent.draft.producer.seed.analysis);
    assert!(
        backend
            .original_source_views
            .lock()
            .unwrap()
            .contains_actual(&parent)
    );
    assert!(!parent.is_indexed(&*backend.workspace_index.read().await));
    assert!(matches!(
        backend
            .append_inherited_original_source_view(&fixture.child, draft)
            .await
            .unwrap(),
        OriginalSourceViewAttempt::RetryCapture
    ));
    assert!(
        !backend
            .original_source_views
            .lock()
            .unwrap()
            .by_uri
            .contains_key(&fixture.child)
    );
    // Recapture through the graph's actual producers must restore the
    // parent index row before admitting a fresh child and its owned suffix.
    let child = child_view(&fixture, &backend).await;
    let current_parent = &child.draft.inherited.as_ref().unwrap().parent;
    assert!(current_parent.is_indexed(&*backend.workspace_index.read().await));
    assert_ne!(current_parent.id, parent.id);
}

async fn unrelated_shared_source_root(
    fixture: &Fixture,
    backend: &Backend,
) -> (Fixture, Arc<PublishedOriginalSourceView>) {
    let other_root = fixture.root.join("unrelated");
    std::fs::create_dir(&other_root).unwrap();
    let other_source = format!(
        "source {{{}}}\nproc other_caller {{}} {{return [helper 7]}}\n",
        fixture.root.join("child.tcl").display()
    );
    std::fs::write(other_root.join("main.tcl"), &other_source).unwrap();
    let other = Fixture {
        main: Uri::from_file_path(other_root.join("main.tcl")).unwrap(),
        child: fixture.child.clone(),
        root: other_root,
    };
    // Admit the closed physical input before issuing its independent root.
    assert!(matches!(
        backend
            .source_child_document_snapshot(&other.child)
            .await
            .unwrap(),
        RehomingDocumentSnapshot::Ready(_)
    ));
    open(backend, &other.main, &other_source).await;
    let other_parent = publication_parent(&other, backend).await;
    (other, other_parent)
}

fn assert_ordinary_scope(
    settlement: &OriginalSourceViewSettlement,
    selected: &Fixture,
    unrelated: &Fixture,
) {
    let selected_uri = &selected.main;
    let selected_definition = settlement.definition_projection(selected_uri).unwrap();
    assert!(settlement.ordinary_definition_scope_is_complete(selected_uri, &selected_definition));
    assert!(
        !settlement.ordinary_definition_scope_is_complete(&unrelated.main, &selected_definition)
    );
    assert!(
        !settlement.ordinary_definition_scope_is_complete(&selected.child, &selected_definition)
    );
    let mut partial_definition = settlement.definition_projection(selected_uri).unwrap();
    partial_definition
        .partial_definition_groups
        .push(Vec::new());
    assert!(!settlement.ordinary_definition_scope_is_complete(selected_uri, &partial_definition));
}

#[tokio::test]
async fn unrelated_persistent_publication_refusal_preserves_selected_scope() {
    let (fixture, backend) = fixture().await;
    let selected_child = child_view(&fixture, &backend).await;
    let selected_parent = Arc::clone(&selected_child.draft.inherited.as_ref().unwrap().parent);
    let (other, other_parent) = unrelated_shared_source_root(&fixture, &backend).await;
    let work = backend.original_source_root_work().await.unwrap().unwrap();
    let bootstrap = work
        .candidates
        .iter()
        .map(|candidate| candidate.draft.clone())
        .collect::<Vec<_>>();
    let excluded = work
        .candidates
        .iter()
        .filter(|candidate| work.associated.contains(&candidate.draft.physical.uri))
        .cloned()
        .collect::<Vec<_>>();
    let incoming = work.incoming;
    backend.workspace_index.write().await.replace_document(
        other.main.as_str(),
        &other_parent.draft.producer.seed.analysis,
    );
    for _ in 0..2 {
        // No publisher restores this row between attempts: this is a
        // persistent refusal in a different, actual root interpretation.
        assert!(!other_parent.is_indexed(&*backend.workspace_index.read().await));
        let selected_scope = backend
            .run_original_source_view_work(Arc::clone(&selected_parent))
            .await
            .unwrap();
        assert!(selected_scope.complete);
        let mut other_scope = backend
            .run_original_source_view_work(Arc::clone(&other_parent))
            .await
            .unwrap();
        assert!(other_scope.recapture.is_some());
        other_scope.require_known_targets(&incoming);
        assert!(other_scope.required.contains(&other.child));
        assert!(
            other_scope.ids.is_empty()
                && !other_scope.complete
                && other_scope.partial_definition.is_none()
        );
        let settlement = backend
            .finish_original_root_reconciliation(OriginalSourceRootReconciliation {
                bootstrap: bootstrap.clone(),
                scopes: vec![selected_scope, other_scope],
                excluded: excluded.clone(),
                incoming: incoming.clone(),
            })
            .await
            .unwrap()
            .expect("unrelated retry debt must not discard the selected settlement");
        assert!(!settlement.requires_recapture(&fixture.main));
        assert!(settlement.requires_recapture(&other.main));
        assert!(settlement.requires_recapture(&other.child));
        assert!(!settlement.aggregate_complete());
        assert_ordinary_scope(&settlement, &fixture, &other);
        assert!(!backend.original_source_view_settlement_views_are_indexed(&settlement));
        assert!(settlement.projection(&fixture.main, true).is_none());
        assert!(settlement.definition_projection(&other.main).is_none());
        // Both real roots source the same physical helper. We make no
        // claim that their interpretation-specific target identities are
        // equivalent; the missing retained inventory alone blocks refs.
        assert!(settlement.definition_projection(&fixture.child).is_none());
        assert!(
            settlement
                .positive_reference_projection(&fixture.main)
                .is_none()
        );
        let projection = settlement
            .definition_projection(&fixture.main)
            .expect("current selected definition remains available");
        let uri = fixture.main.clone();
        let groups = projection.partial_definition_groups.clone();
        let (owners, route) = backend
            .compute_original_source_views(&projection, move |views| {
                Some(Backend::calculate_original_source_navigation(
                    views,
                    &uri,
                    Position::new(1, 24),
                    None,
                    false,
                    &groups,
                ))
            })
            .await
            .unwrap()
            .unwrap();
        assert!(
            matches!(route, Some(std::ops::ControlFlow::Break(Some(ref locations))) if locations.iter().any(|location| location.uri == fixture.child))
        );
        assert!(
            backend
                .with_settled_original_source_scope(&projection, |views| {
                    Backend::same_original_source_view_inventory(&owners, views).then_some(())
                })
                .await
                .unwrap()
                .is_some()
        );
    }
}

#[tokio::test]
async fn source_publication_never_renews_a_changed_or_deleted_saved_draft() {
    for deleted in [false, true] {
        let (fixture, backend) = fixture().await;
        let parent = publication_parent(&fixture, &backend).await;
        let draft = inherited_draft(&fixture, &backend, &parent).await;
        if deleted {
            std::fs::remove_file(fixture.root.join("child.tcl")).unwrap();
        } else {
            std::fs::write(fixture.root.join("child.tcl"), "proc replacement {} {}\n").unwrap();
        }
        assert!(matches!(
            backend
                .append_inherited_original_source_view(&fixture.child, draft)
                .await
                .unwrap(),
            OriginalSourceViewAttempt::Unavailable
        ));
        assert!(
            !backend
                .original_source_views
                .lock()
                .unwrap()
                .by_uri
                .contains_key(&fixture.child)
        );
    }
}

#[tokio::test]
async fn unknown_source_frontier_remains_terminal_unavailable() {
    let (fixture, backend) = fixture().await;
    open(
        &backend,
        &fixture.main,
        "source $unresolved\nproc caller {} {return [helper 7]}\n",
    )
    .await;
    let settlement = backend
        .refresh_original_source_view_graph_once()
        .await
        .unwrap()
        .expect("semantic uncertainty is a pending scope, not publication contention");
    assert!(settlement.definition_projection(&fixture.main).is_none());
    let attempt = backend
        .original_source_view_navigation_attempt(&fixture.main, Position::new(1, 24), None)
        .await
        .unwrap();
    assert!(matches!(attempt, Some(std::ops::ControlFlow::Break(None))));
}

type SourceProtocolService = crate::transport_liveness::DeferredConcurrency<
    tower::util::MapRequest<LspService<Backend>, fn(Request) -> Request>,
>;
type SourceProtocolTask = tokio::task::JoinHandle<Result<Option<Response>, ExitedError>>;

struct SourceProtocol {
    service: SourceProtocolService,
    backend: Backend,
    responder: tokio::task::JoinHandle<()>,
    startup: Vec<SourceProtocolTask>,
    serial: i64,
}

impl SourceProtocol {
    fn new() -> Self {
        let (service, socket) = LspService::new(Backend::new);
        let backend = service.inner().clone();
        let (mut outgoing, mut responses) = socket.split();
        let responder = tokio::spawn(async move {
            while let Some(request) = outgoing.next().await {
                let (method, id, params) = request.into_parts();
                if let Some(id) = id {
                    let result = if method == "workspace/configuration" {
                        serde_json::Value::Array(
                            params.unwrap()["items"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|item| {
                                    if item["section"] == "tclLsp.dialect" {
                                        serde_json::json!("tcl8.6")
                                    } else {
                                        serde_json::json!({"dialect":"tcl8.6","libraryPaths":[]})
                                    }
                                })
                                .collect(),
                        )
                    } else {
                        serde_json::Value::Null
                    };
                    responses.send(Response::from_ok(id, result)).await.unwrap();
                }
            }
        });
        let service =
            service.map_request(crate::service::normalise_request_uris as fn(Request) -> Request);
        Self {
            service: crate::transport_liveness::DeferredConcurrency::new(
                service,
                crate::transport_liveness::DEFAULT_HANDLER_CONCURRENCY,
            ),
            backend,
            responder,
            startup: Vec::new(),
            serial: 0,
        }
    }

    async fn request(
        &mut self,
        method: &'static str,
        params: serde_json::Value,
    ) -> serde_json::Value {
        self.serial += 1;
        let request = Request::build(method)
            .id(self.serial)
            .params(params)
            .finish();
        self.service
            .ready()
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap()
            .unwrap()
            .into_parts()
            .1
            .unwrap()
    }

    async fn notify(&mut self, method: &'static str, params: serde_json::Value) {
        let future = self
            .service
            .ready()
            .await
            .unwrap()
            .call(Request::build(method).params(params).finish());
        self.startup.push(tokio::spawn(future));
        tokio::task::yield_now().await;
    }

    async fn start(&mut self, fixture: &Fixture) {
        let root_uri = Uri::from_file_path(&fixture.root).unwrap();
        let _ = self.request("initialize", serde_json::json!({
                "processId":std::process::id(), "rootUri":null, "workspaceFolders":[],
                "initializationOptions":{"folderDialects":{(root_uri.as_str()):"tcl8.6"}},
                "capabilities":{"general":{"positionEncodings":["utf-16"]},"workspace":{"configuration":true,"workspaceFolders":true},"textDocument":{"hover":{"contentFormat":["markdown","plaintext"]},"definition":{"linkSupport":false},"publishDiagnostics":{}}}
            })).await;
        // Preserve the real concurrent notification/callback order.
        self.notify("initialized", serde_json::json!({})).await;
        self.notify(
            "workspace/didChangeConfiguration",
            serde_json::json!({"settings":{"tclLsp":{"dialect":"tcl8.6","libraryPaths":[]}}}),
        )
        .await;
        self.notify("textDocument/didOpen", serde_json::json!({"textDocument":{"uri":fixture.main,"languageId":"tcl","version":1,"text":EXACT_APP}})).await;
        let config = self.request("workspace/executeCommand", serde_json::json!({"command":"tcl-lsp.getEffectiveConfig","arguments":[fixture.main]})).await;
        assert_eq!(config["dialect"], "tcl8.6", "{config}");
    }

    async fn wait_current(&mut self, fixture: &Fixture) {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
                loop {
                    let report = self.request("workspace/executeCommand", serde_json::json!({"command":"tcl-lsp.currentDiagnostics","arguments":[{"uri":fixture.main,"version":1,"source":EXACT_APP}]})).await;
                    if report["status"] == "complete" {
                        assert_eq!(report["dialectId"], "tcl8.6");
                        assert_eq!(report["source"], EXACT_APP);
                        break;
                    }
                    assert_eq!(report["status"], "unavailable", "{report}");
                    assert!(report["reason"] == "pending" || report["reason"] == "stale", "{report}");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }).await.expect("protocol app currentDiagnostics must become complete before navigation");
    }

    async fn finish(&mut self) {
        for task in self.startup.drain(..) {
            task.await.unwrap().unwrap();
        }
        let _ = self.request("shutdown", serde_json::Value::Null).await;
        self.responder.abort();
    }
}

fn exact_source_fixture() -> Fixture {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-exact-wasi-first-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("app.tcl"), EXACT_APP).unwrap();
    std::fs::write(root.join("lib.tcl"), EXACT_LIB).unwrap();
    Fixture {
        main: Uri::from_file_path(root.join("app.tcl")).unwrap(),
        child: Uri::from_file_path(root.join("lib.tcl")).unwrap(),
        root,
    }
}

/// Genuine `Backend::new` initialization/configuration precedes one definition.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exact_wasi_closed_child_first_public_definition_and_partial_hover() {
    let fixture = exact_source_fixture();
    let mut protocol = SourceProtocol::new();
    protocol.start(&fixture).await;
    protocol.wait_current(&fixture).await;
    assert!(protocol.backend.workspace_folders.lock().await.is_empty());
    assert!(
        !protocol
            .backend
            .documents
            .lock("exact child is closed")
            .await
            .contains_key(&fixture.child)
    );
    assert!(
        !protocol
            .backend
            .db_files
            .lock()
            .await
            .contains_key(&fixture.child)
    );
    assert!(
        !protocol
            .backend
            .workspace_index
            .read()
            .await
            .contains_document(fixture.child.as_str())
    );
    let response = protocol.request("textDocument/definition", serde_json::json!({"textDocument":{"uri":fixture.main},"position":{"line":9,"character":13}})).await;
    let expected_uri = fixture.child.as_str();
    let found = response.as_array().is_some_and(|locations| {
        locations.iter().any(|location| {
            location["uri"] == expected_uri && location["range"]["start"]["line"] == 8
        })
    });
    let hover = protocol.request("textDocument/hover", serde_json::json!({"textDocument":{"uri":fixture.main},"position":{"line":9,"character":13}})).await;
    let hover_text = serde_json::to_string(&hover).unwrap();
    // Cleanup still follows the preserved first response and separate hover.
    protocol.finish().await;
    assert!(
        found,
        "first public definition must locate actual helper declaration: {response}"
    );
    assert!(
        hover_text.contains("proc ::helper {value}")
            && hover_text.contains("Conditional source declaration"),
        "partial source hover must retain the actual helper metadata: {hover}"
    );
}

#[tokio::test]
async fn navigation_compute_first_lease_refuses_contention_and_rejoins_after_release() {
    use futures_util::FutureExt as _;
    let (fixture, backend) = fixture().await;
    let settlement = backend
        .refresh_original_source_view_graph()
        .await
        .unwrap()
        .unwrap();
    let projection = settlement
        .navigation_projection(&fixture.main, None)
        .unwrap();
    let computed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let held_table = backend.original_source_views.lock().unwrap();
        let attempted = Arc::clone(&computed);
        // The first try_lock refuses synchronously. Do not await while
        // holding the standard mutex, or schedule a worker without a lease.
        let refused = backend
            .compute_original_source_views(&projection, move |_| {
                attempted.store(true, std::sync::atomic::Ordering::SeqCst);
                Some(())
            })
            .now_or_never()
            .expect("table contention must refuse the first lease immediately")
            .unwrap();
        assert!(refused.is_none());
        assert!(!computed.load(std::sync::atomic::Ordering::SeqCst));
        drop(held_table);
    }
    let attempted = Arc::clone(&computed);
    let leased = backend
        .compute_original_source_views(&projection, move |_| {
            attempted.store(true, std::sync::atomic::Ordering::SeqCst);
            Some(())
        })
        .await
        .unwrap();
    assert!(leased.is_some());
    assert!(computed.load(std::sync::atomic::Ordering::SeqCst));
    // Actual navigation still needs its own new settlement and final lease.
    let route = backend
        .original_source_view_navigation_attempt(&fixture.main, Position::new(1, 24), None)
        .await
        .unwrap();
    assert!(
        matches!(route, Some(std::ops::ControlFlow::Break(Some(ref locations))) if locations.iter().any(|location| location.uri == fixture.child))
    );
    // Successful computation of an unavailable semantic answer is terminal;
    // it must not be confused with an unavailable owner lease.
    let semantic = backend
        .original_source_view_navigation_attempt(&fixture.main, Position::new(0, 1), None)
        .await
        .unwrap();
    assert!(matches!(semantic, Some(std::ops::ControlFlow::Break(None))));
    // Establish currentness immediately before changing saved bytes, so
    // the withdrawal below cannot pass on an already-retired projection.
    assert!(
        backend
            .compute_original_source_views(&projection, |_| Some(()))
            .await
            .unwrap()
            .is_some()
    );
    // The previous lease cannot survive an unannounced physical change.
    std::fs::write(fixture.root.join("child.tcl"), "proc replacement {} {}\n").unwrap();
    assert!(
        backend
            .compute_original_source_views(&projection, |_| Some(()))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn pending_parent_does_not_mint_an_interpretation_or_completion_receipt() {
    let (fixture, backend) = fixture().await;
    open(
        &backend,
        &fixture.main,
        "proc documented {value} {return $value}\n",
    )
    .await;
    let parent = Uri::from_file_path(fixture.root.join("pending.tcl")).unwrap();
    open(&backend, &parent, "source main.tcl\n").await;
    backend
        .documents
        .lock("pending source parent")
        .await
        .get_mut(&parent)
        .unwrap()
        .publication = DocumentPublication::Pending;
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(0, 7))
        .await
        .unwrap();
    assert!(matches!(route, std::ops::ControlFlow::Continue(())));
    let table = backend.original_source_views.lock().unwrap();
    assert!(!table.by_uri.contains_key(&fixture.main));
    assert!(!table.by_uri.contains_key(&parent));
}

#[tokio::test]
async fn ordinary_unknown_factory_hover_is_independent_of_unrelated_source_debt() {
    let (fixture, backend) = fixture().await;
    let caller = "UnknownFactory create Made {method go {} {return 1}}";
    open(&backend, &fixture.main, caller).await;
    let unrelated = Uri::from_file_path(fixture.root.join("unrelated.tcl")).unwrap();
    open(&backend, &unrelated, "source missing.tcl\n").await;
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(0, 36))
        .await
        .unwrap();
    assert!(matches!(route, std::ops::ControlFlow::Continue(())));
}

#[tokio::test]
async fn incoming_source_association_keeps_child_hover_in_its_inherited_namespace() {
    let (fixture, backend) = fixture().await;
    open(
        &backend,
        &fixture.main,
        "namespace eval ::ns {source child.tcl}\n",
    )
    .await;
    let route = backend
        .original_source_view_hover_route(&fixture.child, Position::new(0, 7))
        .await
        .unwrap();
    let std::ops::ControlFlow::Break(Some(hover)) = route else {
        panic!("inherited child documentation")
    };
    assert!(hover.value.contains("proc ::ns::helper"));
    assert!(!hover.value.contains("proc ::helper"));
}

#[tokio::test]
async fn sourced_class_declaration_hover_keeps_its_own_original_metadata() {
    let (fixture, backend) = fixture().await;
    open(
        &backend,
        &fixture.main,
        "source child.tcl\noo::class create Widget {\n    method run {} {return 1}\n}\n",
    )
    .await;
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(1, 18))
        .await
        .unwrap();
    let std::ops::ControlFlow::Break(Some(hover)) = route else {
        panic!("class source declaration card")
    };
    assert!(hover.value.contains("oo::class create ::Widget"));
    assert!(hover.value.contains("run"));
    assert!(hover.value.contains("availability depend on execution"));
}

#[tokio::test]
async fn sourced_schema_hover_survives_complete_and_unavailable_suffixes() {
    let (fixture, backend) = fixture().await;
    open(&backend, &fixture.main, "source child.tcl\nset value 1\n").await;
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(1, 1))
        .await
        .unwrap();
    let std::ops::ControlFlow::Break(Some(hover)) = route else {
        panic!("source schema card")
    };
    assert!(hover.value.contains("**`set`**"));
    open(
        &backend,
        &fixture.main,
        "source child.tcl\nset value 1\nputs $value\n",
    )
    .await;
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(2, 1))
        .await
        .unwrap();
    let std::ops::ControlFlow::Break(Some(hover)) = route else {
        panic!("source schema despite incomplete execution")
    };
    assert!(hover.value.contains("**`puts`**"));
    assert!(hover.value.contains("command availability is unresolved"));
}

#[tokio::test]
async fn sourced_special_variable_hover_keeps_exact_root_source_role() {
    let (fixture, backend) = fixture().await;
    open(
        &backend,
        &fixture.main,
        "source child.tcl\nputs $tcl_version\nputs $::env(PATH)\n",
    )
    .await;
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(2, 9))
        .await
        .unwrap();
    let std::ops::ControlFlow::Break(Some(hover)) = route else {
        panic!("root special source card")
    };
    assert!(hover.value.contains("env"));
    assert!(
        hover
            .value
            .contains("binding, availability and the current value are unresolved")
    );
    open(
        &backend,
        &fixture.main,
        "source child.tcl\nproc p {env} {puts $env(PATH)}\n",
    )
    .await;
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(1, 20))
        .await
        .unwrap();
    assert!(matches!(route, std::ops::ControlFlow::Break(None)));
}

#[tokio::test]
async fn bounded_source_documentation_keeps_live_source_and_input_leases() {
    let (fixture, backend) = fixture().await;
    open(&backend, &fixture.main, "source missing.tcl\nputs 7\n").await;
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(1, 1))
        .await
        .unwrap();
    let std::ops::ControlFlow::Break(Some(hover)) = route else {
        panic!("unavailable source vocabulary")
    };
    assert!(hover.value.contains("**`puts`**"));
    assert!(hover.value.contains("command availability is unresolved"));
    let snapshot = backend.read_document(&fixture.main).await.unwrap().unwrap();
    let owner = backend
        .capture_workspace_symbol_analysis(&fixture.main, &snapshot)
        .await
        .unwrap()
        .unwrap();
    let epoch = backend.diag_inputs_epoch();
    let generation = *backend.class_factory_generation.read().await;
    backend.invalidate_diag_inputs();
    assert!(
        backend
            .with_current_readonly_source_state(
                epoch,
                generation,
                &[owner.admission()],
                None,
                |state| owner
                    .matches_current_state(state, epoch, generation)
                    .then_some(()),
                |_, ()| Some(())
            )
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn ordinary_complete_hover_keeps_its_existing_provider_route() {
    let (fixture, backend) = fixture().await;
    open(&backend, &fixture.main, "set counter 1\n").await;
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(0, 1))
        .await
        .unwrap();
    assert!(matches!(route, std::ops::ControlFlow::Continue(())));
}

#[tokio::test]
async fn source_hover_withdraws_when_actual_analyser_inputs_change() {
    let (fixture, backend) = fixture().await;
    let settlement = backend
        .refresh_original_source_view_graph()
        .await
        .unwrap()
        .unwrap();
    let projection = settlement.projection(&fixture.main, false).unwrap();
    assert!(
        backend
            .with_settled_original_source_scope(&projection, |_| Some(()))
            .await
            .unwrap()
            .is_some()
    );
    backend.invalidate_diag_inputs();
    assert!(
        backend
            .with_settled_original_source_scope(&projection, |_| Some(()))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn completed_source_hover_uses_inherited_unicode_parameter_metadata() {
    use tower_lsp_server::LanguageServer;
    use tower_lsp_server::ls_types::{
        HoverParams, TextDocumentIdentifier, TextDocumentPositionParams,
    };
    let (fixture, backend) = fixture().await;
    std::fs::write(
        fixture.root.join("child.tcl"),
        "# Documented helper\nproc helper {value {suffix 🧪}} {return $value}\n",
    )
    .unwrap();
    let hover = backend
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: fixture.main.clone(),
                },
                position: Position::new(1, 24),
            },
            work_done_progress_params: tower_lsp_server::ls_types::WorkDoneProgressParams::default(
            ),
        })
        .await
        .unwrap()
        .unwrap();
    let text = serde_json::to_string(&hover).unwrap();
    assert!(text.contains("Conditional source declaration"));
    assert!(text.contains("proc ::helper {value {suffix 🧪}}"));
    assert!(text.contains("Documented helper"));
    let view = child_view(&fixture, &backend).await;
    assert!(
        view.draft
            .inherited
            .as_ref()
            .unwrap()
            .point
            .matches_child_analysis(&view.draft.producer.seed.analysis)
    );
}

#[tokio::test]
async fn source_hover_rechecks_changed_and_deleted_target_before_publication() {
    for deleted in [false, true] {
        let (fixture, backend) = fixture().await;
        let settlement = backend
            .refresh_original_source_view_graph()
            .await
            .unwrap()
            .unwrap();
        let projection = settlement.projection(&fixture.main, false).unwrap();
        let uri = fixture.main.clone();
        let (owners, output) = backend
            .compute_original_source_views(&projection, move |views| {
                Some(Backend::calculate_original_source_hover(
                    views,
                    &uri,
                    Position::new(1, 24),
                ))
            })
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            output,
            Some(std::ops::ControlFlow::Break(Some(_)))
        ));
        if deleted {
            std::fs::remove_file(fixture.root.join("child.tcl")).unwrap();
        } else {
            std::fs::write(fixture.root.join("child.tcl"), "proc changed {} {}\n").unwrap();
        }
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
}

#[tokio::test]
async fn source_hover_respects_known_replacement_in_the_same_suffix() {
    let (fixture, backend) = fixture().await;
    let source = "source child.tcl\nproc helper {local} {}\nproc caller {} {return [helper 7]}\n";
    open(&backend, &fixture.main, source).await;
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(2, 24))
        .await
        .unwrap();
    let std::ops::ControlFlow::Break(Some(hover)) = route else {
        panic!("expected selected replacement documentation")
    };
    assert!(hover.value.contains("proc ::helper {local}"));
    assert!(!hover.value.contains("proc ::helper {value}"));
}

#[tokio::test]
async fn unavailable_source_hover_never_falls_through_to_lexical_names() {
    let (fixture, backend) = fixture().await;
    std::fs::remove_file(fixture.root.join("child.tcl")).unwrap();
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(1, 24))
        .await
        .unwrap();
    assert!(matches!(route, std::ops::ControlFlow::Break(None)));
    let other = Uri::from_file_path(fixture.root.join("other.tcl")).unwrap();
    open(&backend, &other, "proc helper {wrong} {}\n").await;
    let route = backend
        .original_source_view_hover_route(&fixture.main, Position::new(1, 24))
        .await
        .unwrap();
    assert!(matches!(route, std::ops::ControlFlow::Break(None)));
}

#[tokio::test]
async fn closed_source_child_issues_inherited_view_from_published_physical_owner() {
    let (fixture, backend) = fixture().await;
    assert!(!backend.db_files.lock().await.contains_key(&fixture.child));
    assert!(
        !backend
            .documents
            .lock("unopened child")
            .await
            .contains_key(&fixture.child)
    );
    let view = child_view(&fixture, &backend).await;
    let inherited = view.draft.inherited.as_ref().unwrap();
    assert!(
        inherited
            .point
            .matches_parent_analysis(&inherited.parent.draft.producer.seed.analysis)
    );
    assert!(
        inherited
            .point
            .matches_child_analysis(&view.draft.producer.seed.analysis)
    );
    assert!(view.draft.matches_producer());
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    let file = files[&fixture.child];
    assert!(file == view.draft.physical.file);
    assert_eq!(file.text(&*db), CHILD);
    assert_eq!(
        file.path(&*db).as_deref(),
        view.draft.physical.admission.analyser_path()
    );
    let paths = backend.source_path_admissions.lock().unwrap();
    let published = &paths[&fixture.child];
    assert!(published.file == file);
    assert_eq!(
        published.admission.as_ref(),
        view.draft.physical.admission.as_ref()
    );
    assert!(view.draft.physical.publication.open_revision.is_none());
}

#[tokio::test]
async fn changed_closed_source_child_withdraws_saved_interpretation() {
    let (fixture, backend) = fixture().await;
    let view = child_view(&fixture, &backend).await;
    std::fs::write(fixture.root.join("child.tcl"), "proc replacement {} {}\n").unwrap();
    assert!(
        backend
            .with_current_original_source_view(view.id, |_| Some(()))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn deleted_closed_source_child_withdraws_saved_interpretation() {
    let (fixture, backend) = fixture().await;
    let view = child_view(&fixture, &backend).await;
    std::fs::remove_file(fixture.root.join("child.tcl")).unwrap();
    assert!(
        backend
            .with_current_original_source_view(view.id, |_| Some(()))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn refused_closed_source_child_path_creates_no_publication() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///source-%FF.tcl".parse().unwrap();
    assert!(matches!(
        backend.source_child_document_snapshot(&uri).await,
        Err(crate::DocumentSourceUnavailable::Path { .. })
    ));
    assert!(!backend.db_files.lock().await.contains_key(&uri));
    assert!(
        !backend
            .source_path_admissions
            .lock()
            .unwrap()
            .contains_key(&uri)
    );
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str())
    );
}

#[tokio::test]
async fn source_child_ingress_preserves_live_unsaved_document() {
    let (fixture, backend) = fixture().await;
    let unsaved = "proc helper {value} {return UNSAVED}\n";
    open(&backend, &fixture.child, unsaved).await;
    let RehomingDocumentSnapshot::Ready(snapshot) = backend
        .source_child_document_snapshot(&fixture.child)
        .await
        .unwrap()
    else {
        panic!("expected live child")
    };
    assert_eq!(snapshot.text.as_ref(), unsaved);
    let view = child_view(&fixture, &backend).await;
    assert_eq!(view.draft.physical.source.as_ref(), unsaved);
    assert!(view.draft.physical.publication.open_revision.is_some());
}

const AUTO_MAIN: &str = "source lib/helpers.tcl\n\nproc run {} {\n    puts [greet_helper world]\n    puts [mypkg_autoloaded]\n}\n";
const AUTO_HELPER: &str = "proc greet_helper {who} {\n    return \"hi $who\"\n}\n";
const AUTO_IMPL: &str = "proc mypkg_autoloaded {} {\n    return 42\n}\n";
const AUTO_INDEX: &str = "# Tcl autoload index file, version 2.0\nset auto_index(mypkg_autoloaded) [list source [file join $dir impl.tcl]]\n";
const AUTO_URI: &str = "file:///ws/main.tcl";
const AUTO_PROVIDER: &str = "file:///ws/tmp/mypkg/impl.tcl";

fn conditional_autoload_protocol(source: &str) -> (SourceProtocol, Arc<crate::vfs::MemoryStore>) {
    let store = Arc::new(crate::vfs::MemoryStore::new());
    for (uri, path, text) in [
        (AUTO_URI, "/ws/main.tcl", source),
        (
            "file:///ws/lib/helpers.tcl",
            "/ws/lib/helpers.tcl",
            AUTO_HELPER,
        ),
        (
            "file:///ws/tmp/mypkg/tclIndex",
            "/ws/tmp/mypkg/tclIndex",
            AUTO_INDEX,
        ),
        (AUTO_PROVIDER, "/ws/tmp/mypkg/impl.tcl", AUTO_IMPL),
    ] {
        store.upsert_uri(uri, path, text.as_bytes().to_vec());
    }
    assert_eq!(store.len(), 4);
    let backend_store: Arc<dyn crate::vfs::SourceStore> = store.clone();
    let (service, socket) =
        LspService::new(move |client| Backend::with_store(client, backend_store));
    let backend = service.inner().clone();
    let (mut outgoing, mut responses) = socket.split();
    let responder = tokio::spawn(async move {
        while let Some(request) = outgoing.next().await {
            let (method, id, params) = request.into_parts();
            if let Some(id) = id {
                let result = if method == "workspace/configuration" {
                    serde_json::Value::Array(
                        params.unwrap()["items"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|_| serde_json::json!({}))
                            .collect(),
                    )
                } else {
                    serde_json::Value::Null
                };
                responses.send(Response::from_ok(id, result)).await.unwrap();
            }
        }
    });
    let service =
        service.map_request(crate::service::normalise_request_uris as fn(Request) -> Request);
    (
        SourceProtocol {
            service: crate::transport_liveness::DeferredConcurrency::new(
                service,
                crate::transport_liveness::DEFAULT_HANDLER_CONCURRENCY,
            ),
            backend,
            responder,
            startup: Vec::new(),
            serial: 0,
        },
        store,
    )
}

async fn start_conditional_autoload(protocol: &mut SourceProtocol, source: &str) {
    let _=protocol.request("initialize",serde_json::json!({
            "processId":null,"rootUri":"file:///ws","workspaceFolders":[{"uri":"file:///ws","name":"ws"}],
            "capabilities":{"general":{"positionEncodings":["utf-16"]},"workspace":{"configuration":true,"symbol":{}},"textDocument":{"definition":{"linkSupport":false},"publishDiagnostics":{}}}
        })).await;
    protocol.notify("initialized", serde_json::json!({})).await;
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            protocol.backend.workspace_scan_ready.wait()
        )
        .await
        .unwrap()
    );
    protocol.notify("textDocument/didOpen",serde_json::json!({"textDocument":{"uri":AUTO_URI,"languageId":"tcl","version":1,"text":source}})).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(
        !protocol
            .backend
            .workspace_index
            .read()
            .await
            .contains_document(AUTO_PROVIDER)
    );
}

async fn conditional_autoload_definition(
    protocol: &mut SourceProtocol,
    source: &str,
) -> serde_json::Value {
    let offset = source.rfind("mypkg_autoloaded").unwrap();
    let position = tcl_lexer::LineIndex::new_lsp(source)
        .position_at_utf16(u32::try_from(offset + 4).unwrap(), source);
    protocol.request("textDocument/definition",serde_json::json!({"textDocument":{"uri":AUTO_URI},"position":{"line":position.line,"character":position.character.get()}})).await
}

fn has_conditional_autoload_provider(result: &serde_json::Value) -> bool {
    result.as_array().is_some_and(|rows| {
        rows.iter().any(|row| {
            row["uri"] == AUTO_PROVIDER
                && row["range"]["start"]["line"] == 0
                && row["range"]["start"]["character"] == 5
        })
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conditional_external_source_autoload_preserves_first_browser_sequence() {
    let (mut protocol, _store) = conditional_autoload_protocol(AUTO_MAIN);
    start_conditional_autoload(&mut protocol, AUTO_MAIN).await;
    let helper=protocol.request("textDocument/definition",serde_json::json!({"textDocument":{"uri":AUTO_URI},"position":{"line":3,"character":14}})).await;
    let _symbols = protocol
        .request(
            "workspace/symbol",
            serde_json::json!({"query":"greet_helper"}),
        )
        .await;
    let first = conditional_autoload_definition(&mut protocol, AUTO_MAIN).await;
    assert_eq!(protocol.serial, 4);
    protocol.finish().await;
    assert!(
        helper.as_array().is_some_and(|rows| rows
            .iter()
            .any(|row| row["uri"] == "file:///ws/lib/helpers.tcl")),
        "{helper}"
    );
    assert!(
        has_conditional_autoload_provider(&first),
        "first id4 external SOURCE candidate: {first}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conditional_external_source_autoload_first_query_keeps_provider_currency() {
    let (mut protocol, store) = conditional_autoload_protocol(AUTO_MAIN);
    start_conditional_autoload(&mut protocol, AUTO_MAIN).await;
    let first = conditional_autoload_definition(&mut protocol, AUTO_MAIN).await;
    let unchanged = conditional_autoload_definition(&mut protocol, AUTO_MAIN).await;
    store.upsert_uri(
        AUTO_PROVIDER,
        "/ws/tmp/mypkg/impl.tcl",
        b"proc replaced_header {} {}\n".to_vec(),
    );
    let changed = conditional_autoload_definition(&mut protocol, AUTO_MAIN).await;
    protocol.finish().await;
    assert!(has_conditional_autoload_provider(&first), "{first}");
    assert!(
        has_conditional_autoload_provider(&unchanged),
        "unchanged repeat is the currency prerequisite: {unchanged}"
    );
    assert!(
        !has_conditional_autoload_provider(&changed),
        "changed provider: {changed}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conditional_external_source_autoload_keeps_shadow_rename_unknown_and_dynamic_barriers() {
    for body in [
        "proc mypkg_autoloaded {} {}; puts [mypkg_autoloaded]",
        "proc held {} {}; rename held mypkg_autoloaded; puts [mypkg_autoloaded]",
        "rename mypkg_autoloaded removed; puts [mypkg_autoloaded]",
        "mystery; puts [mypkg_autoloaded]",
        "puts [mystery]; puts [mypkg_autoloaded]",
        "puts [mypkg_autoloaded $arg]",
    ] {
        let source = format!("source lib/helpers.tcl\nproc run {{arg}} {{{body}}}\n");
        let (mut protocol, _store) = conditional_autoload_protocol(&source);
        start_conditional_autoload(&mut protocol, &source).await;
        let result = conditional_autoload_definition(&mut protocol, &source).await;
        protocol.finish().await;
        assert!(
            !has_conditional_autoload_provider(&result),
            "{source}: {result}"
        );
    }
}
#[tokio::test]
async fn conditional_external_source_autoload_preserves_original_saved_byte_receipts() {
    let (protocol, store) = conditional_autoload_protocol(AUTO_MAIN);
    let providers = protocol
        .backend
        .capture_conditional_autoload_providers(vec![std::path::PathBuf::from(
            "/ws/tmp/mypkg/impl.tcl",
        )])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(providers.len(), 1);
    let provider = &providers[0];
    let publications = [&provider.source.publication];
    let admissions = [provider.source.admission.as_ref()];
    let saved = Backend::conditional_autoload_saved_sources(&publications, &admissions)
        .expect("original closed-file byte receipt");
    assert_eq!(saved[0].1.as_ref(), AUTO_IMPL.as_bytes());
    let selected = saved
        .iter()
        .map(|(admission, bytes)| (admission.as_ref(), bytes.as_ref()))
        .collect::<Vec<_>>();
    assert!(
        super::super::super::DocumentPathAdmission::with_current_saved_sources(
            &selected,
            store.as_ref(),
            || true
        )
        .unwrap()
    );
    let changed = AUTO_IMPL.replace('\n', "\r");
    assert_eq!(tcl_lexer::normalise_lone_cr(&changed).as_ref(), AUTO_IMPL);
    store.upsert_uri(
        AUTO_PROVIDER,
        "/ws/tmp/mypkg/impl.tcl",
        changed.into_bytes(),
    );
    // The actual original publication keeps its original bytes, even when
    // a newer read would normalise to precisely the same analysis source.
    let retained = Backend::conditional_autoload_saved_sources(&publications, &admissions).unwrap();
    assert_eq!(retained[0].1.as_ref(), AUTO_IMPL.as_bytes());
    assert!(
        super::super::super::DocumentPathAdmission::with_current_saved_sources(
            &selected,
            store.as_ref(),
            || true
        )
        .is_err()
    );
    protocol.responder.abort();
}

#[tokio::test]
async fn conditional_external_source_autoload_does_not_cross_donate_indexed_provider_headers() {
    let (protocol, store) = conditional_autoload_protocol(AUTO_MAIN);
    store.upsert_uri(
        "file:///ws/tmp/mypkg/other.tcl",
        "/ws/tmp/mypkg/other.tcl",
        b"proc unrelated_header {} {}\n".to_vec(),
    );
    let providers = protocol
        .backend
        .capture_conditional_autoload_providers(vec![
            std::path::PathBuf::from("/ws/tmp/mypkg/other.tcl"),
            std::path::PathBuf::from("/ws/tmp/mypkg/impl.tcl"),
        ])
        .await
        .unwrap()
        .unwrap();
    let first_files = ["file:///ws/tmp/mypkg/other.tcl".parse::<Uri>().unwrap()];
    let second_files = [AUTO_PROVIDER.parse::<Uri>().unwrap()];
    let first = Backend::conditional_autoload_provider_headers(&providers, &first_files);
    let second = Backend::conditional_autoload_provider_headers(&providers, &second_files);
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    let caller_source = "proc run {} {puts [mypkg_autoloaded]}";
    let caller = tcl_compiler::analyser::Analyser::new().analyse(caller_source, "tcl8.6");
    let publication =
        tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(91_u8);
    let documents = [
        tcl_lsp_core::original_source_views::OriginalSourceViewDocument {
            publication: &publication,
            document: tcl_lsp_core::original_declaration::OriginalDeclarationDocument {
                uri: AUTO_URI,
                source: caller_source,
                analysis: &caller,
            },
            region: None,
            ancestors: &[],
        },
    ];
    let at = u32::try_from(caller_source.find("mypkg_autoloaded").unwrap() + 4).unwrap();
    let requests = tcl_lsp_core::original_source_views::conditional_external_autoload_requests(
        AUTO_URI, at, &documents,
    )
    .expect("genuine conditional body request");
    assert_eq!(requests.len(), 1);
    assert!(
        tcl_lsp_core::original_source_views::conditional_external_autoload_header(
            &requests[0],
            &documents,
            &second
        )
        .is_some(),
        "the other indexed provider genuinely has the target header"
    );
    assert!(
        tcl_lsp_core::original_source_views::conditional_external_autoload_header(
            &requests[0],
            &documents,
            &first
        )
        .is_none(),
        "this request's own indexed file cannot borrow that header"
    );
    assert_eq!(first[0].uri, first_files[0].as_str());
    assert_eq!(second[0].uri, second_files[0].as_str());
    assert!(
        !first[0]
            .analysis
            .original_procedure_declarations()
            .any(|row| &first[0].source[row.name_input().span().as_range()] == "mypkg_autoloaded")
    );
    assert!(
        second[0]
            .analysis
            .original_procedure_declarations()
            .any(|row| &second[0].source[row.name_input().span().as_range()] == "mypkg_autoloaded")
    );
    protocol.responder.abort();
}
