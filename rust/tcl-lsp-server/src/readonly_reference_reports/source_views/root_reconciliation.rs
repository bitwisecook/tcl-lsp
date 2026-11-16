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

//! Refreshing the view graph: root discovery, the publication of initial roots and the reconciliation of roots a source association excludes.

use super::super::{Arc, Backend, DocumentSourceResult};
use super::{
    OriginalSourceRootCandidate, OriginalSourceRootReconciliation, OriginalSourceRootScope,
    OriginalSourceRootWork, OriginalSourceViewAttempt, OriginalSourceViewDraft,
    OriginalSourceViewRecapture, OriginalSourceViewSettlement, PublishedOriginalSourceView,
    SOURCE_VIEW_CONTENTION_ATTEMPTS, SOURCE_VIEW_CONTENTION_BACKOFF,
};
use std::collections::{HashMap, HashSet};
use tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceFrontier as SourceFrontier;
use tower_lsp_server::ls_types::Uri;

impl OriginalSourceRootScope {
    pub(super) fn retrying(mut self) -> Self {
        self.recapture = Some(OriginalSourceViewRecapture);
        self.complete = false;
        self.partial_definition = None;
        self.completion = None;
        // No row rejected by publication is surveyed as a current producer.
        self.ids.clear();
        self
    }

    pub(super) fn require_known_targets(&mut self, incoming: &HashMap<Uri, HashSet<Uri>>) {
        loop {
            let targets = incoming
                .iter()
                .filter(|(_, parents)| !parents.is_disjoint(&self.required))
                .map(|(target, _)| target.clone())
                .collect::<Vec<_>>();
            let before = self.required.len();
            self.required.extend(targets);
            if self.required.len() == before {
                break;
            }
        }
    }

    pub(super) fn pending(uri: Uri) -> Self {
        Self {
            recapture: None,
            root: None,
            root_uri: uri.clone(),
            required: HashSet::from([uri]),
            association_owners: Vec::new(),
            ids: Vec::new(),
            complete: false,
            partial_definition: None,
            completion: None,
            eligibility: HashSet::new(),
            conditional_obligations: false,
            root_world_uncertain: false,
        }
    }
}

impl Backend {
    /// Build serialization is separate from the final rehoming publication
    /// gate. No source read/analysis awaits while holding that publication gate.
    pub(crate) async fn refresh_original_source_view_graph(
        &self,
    ) -> DocumentSourceResult<Option<OriginalSourceViewSettlement>> {
        // The view table is read with `try_lock`, so a publisher holding it
        // (the one that follows `didOpen`) refuses a settlement that a moment
        // later succeeds. Only a refusal that persists is reported.
        for _ in 1..SOURCE_VIEW_CONTENTION_ATTEMPTS {
            if let Some(settlement) = self.refresh_original_source_view_graph_once().await? {
                return Ok(Some(settlement));
            }
            crate::rt::sleep(SOURCE_VIEW_CONTENTION_BACKOFF).await;
        }
        self.refresh_original_source_view_graph_once().await
    }

    pub(super) async fn refresh_original_source_view_graph_for_uri(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<Option<OriginalSourceViewSettlement>> {
        for _ in 1..SOURCE_VIEW_CONTENTION_ATTEMPTS {
            if let Some(settlement) = self.refresh_original_source_view_graph_once().await?
                && !settlement.requires_recapture(uri)
            {
                return Ok(Some(settlement));
            }
            crate::rt::sleep(SOURCE_VIEW_CONTENTION_BACKOFF).await;
        }
        self.refresh_original_source_view_graph_once().await
    }

    pub(super) async fn refresh_original_source_view_graph_once(
        &self,
    ) -> DocumentSourceResult<Option<OriginalSourceViewSettlement>> {
        let _build = self.original_source_view_build_gate.lock().await;
        if !self
            .prune_retired_original_source_view_presentations()
            .await
        {
            return Ok(None);
        }
        let Some(work) = self.original_source_root_work().await? else {
            return Ok(None);
        };
        let mut reconciliation = OriginalSourceRootReconciliation {
            bootstrap: work
                .candidates
                .iter()
                .map(|candidate| candidate.draft.clone())
                .collect(),
            scopes: work
                .pending
                .into_iter()
                .map(OriginalSourceRootScope::pending)
                .collect(),
            excluded: Vec::new(),
            incoming: work.incoming,
        };
        let roots = self
            .publish_initial_original_roots(work.candidates, &work.associated, &mut reconciliation)
            .await?;
        // Each actual root retains its own pending state; failure cannot erase
        // debt or globally suppress a different completed producer.
        for root in roots {
            let mut scope = self.run_original_source_view_work(root).await?;
            if scope.recapture.is_some() {
                scope.require_known_targets(&reconciliation.incoming);
            }
            reconciliation.scopes.push(scope);
        }
        if !self
            .reconcile_excluded_original_roots(&mut reconciliation)
            .await?
        {
            return Ok(None);
        }
        let result = self
            .finish_original_root_reconciliation(reconciliation)
            .await?;
        Ok(result)
    }

    async fn publish_initial_original_roots(
        &self,
        candidates: Vec<OriginalSourceRootCandidate>,
        associated: &HashSet<Uri>,
        reconciliation: &mut OriginalSourceRootReconciliation,
    ) -> DocumentSourceResult<Vec<Arc<PublishedOriginalSourceView>>> {
        let mut roots = Vec::new();
        for candidate in candidates {
            let uri = candidate.draft.physical.uri.clone();
            if associated.contains(&uri) {
                reconciliation.excluded.push(candidate);
                continue;
            }
            if let Some(cached) = candidate.published {
                roots.push(cached);
                continue;
            }
            let views = match self
                .publish_original_source_views(&uri, vec![candidate.draft])
                .await?
            {
                OriginalSourceViewAttempt::Ready(views) => views,
                OriginalSourceViewAttempt::RetryCapture => {
                    let mut scope = OriginalSourceRootScope::pending(uri).retrying();
                    scope.require_known_targets(&reconciliation.incoming);
                    reconciliation.scopes.push(scope);
                    continue;
                }
                OriginalSourceViewAttempt::Unavailable => {
                    reconciliation
                        .scopes
                        .push(OriginalSourceRootScope::pending(uri));
                    continue;
                }
            };
            roots.extend(views);
        }
        Ok(roots)
    }

    async fn reconcile_excluded_original_roots(
        &self,
        reconciliation: &mut OriginalSourceRootReconciliation,
    ) -> DocumentSourceResult<bool> {
        // Reconsider only candidates whose tentative incoming parents were
        // replaced by genuine completed source associations. An unrelated
        // completed root cannot break a rootless cycle or choose its entry.
        loop {
            let Some((actual, completed)) = self
                .current_original_scope_associations(&reconciliation.scopes)
                .await?
            else {
                return Ok(false);
            };
            let mut progress = false;
            let mut remaining = Vec::new();
            for candidate in std::mem::take(&mut reconciliation.excluded) {
                let uri = candidate.draft.physical.uri.clone();
                if actual.contains(&uri) {
                    continue;
                }
                let eligible = reconciliation.incoming.get(&uri).is_some_and(|parents| {
                    !parents.is_empty() && parents.iter().all(|parent| completed.contains(parent))
                });
                if !eligible {
                    remaining.push(candidate);
                    continue;
                }
                let view = if let Some(view) = candidate.published {
                    let Some(current) = self
                        .with_current_original_source_view(view.id, |current| {
                            current
                                .draft
                                .same_live_producer(&view.draft)
                                .then(|| Arc::clone(current))
                        })
                        .await?
                    else {
                        reconciliation
                            .scopes
                            .push(OriginalSourceRootScope::pending(uri));
                        continue;
                    };
                    current
                } else {
                    let mut views = match self
                        .publish_original_source_views(&uri, vec![candidate.draft])
                        .await?
                    {
                        OriginalSourceViewAttempt::Ready(views) => views,
                        OriginalSourceViewAttempt::RetryCapture => {
                            let mut scope = OriginalSourceRootScope::pending(uri).retrying();
                            scope.require_known_targets(&reconciliation.incoming);
                            reconciliation.scopes.push(scope);
                            continue;
                        }
                        OriginalSourceViewAttempt::Unavailable => {
                            reconciliation
                                .scopes
                                .push(OriginalSourceRootScope::pending(uri));
                            continue;
                        }
                    };
                    if views.len() != 1 {
                        return Ok(false);
                    }
                    views.remove(0)
                };
                let mut scope = self.run_original_source_view_work(view).await?;
                if scope.recapture.is_some() {
                    scope.require_known_targets(&reconciliation.incoming);
                }
                scope.eligibility = reconciliation
                    .incoming
                    .get(&uri)
                    .cloned()
                    .unwrap_or_default();
                reconciliation.scopes.push(scope);
                progress = true;
            }
            reconciliation.excluded = remaining;
            if !progress {
                break;
            }
        }
        Ok(true)
    }

    pub(super) async fn finish_original_root_reconciliation(
        &self,
        mut reconciliation: OriginalSourceRootReconciliation,
    ) -> DocumentSourceResult<Option<OriginalSourceViewSettlement>> {
        let final_revision = {
            let Ok(table) = self.original_source_views.try_lock() else {
                return Ok(None);
            };
            let Some(revision) = table.current_revision() else {
                return Ok(None);
            };
            revision
        };
        // Completion dependencies may have retired during another root's
        // reconciliation. Propagate those losses through root eligibility.
        loop {
            let Some((_, completed)) = self
                .current_original_scope_associations(&reconciliation.scopes)
                .await?
            else {
                return Ok(None);
            };
            let mut changed = false;
            for scope in &mut reconciliation.scopes {
                if !scope.eligibility.is_subset(&completed) {
                    if scope.complete {
                        scope.complete = false;
                        changed = true;
                    }
                    scope.partial_definition = None;
                }
            }
            if !changed {
                break;
            }
        }
        let Some((actual_associations, _)) = self
            .current_original_scope_associations(&reconciliation.scopes)
            .await?
        else {
            return Ok(None);
        };
        for candidate in reconciliation.excluded {
            let uri = candidate.draft.physical.uri.clone();
            if !actual_associations.contains(&uri) {
                reconciliation
                    .scopes
                    .push(OriginalSourceRootScope::pending(uri.clone()));
            }
        }
        let superseded = {
            let Ok(table) = self.original_source_views.try_lock() else {
                return Ok(None);
            };
            reconciliation
                .scopes
                .iter()
                .filter_map(|scope| {
                    let id = scope.root?;
                    (!table.by_id.contains_key(&id)
                        && actual_associations.contains(&scope.root_uri))
                    .then_some(id)
                })
                .collect::<HashSet<_>>()
        };
        // A root disappears only after an actual association has retired its
        // exact publication. No failed or unknown replay can discharge it.
        reconciliation.scopes.retain(|scope| match scope.root {
            Some(root) => !superseded.contains(&root),
            None => !actual_associations.contains(&scope.root_uri),
        });
        // Capturing a final revision after a concurrent physical/config
        // mutation cannot bless an older bootstrap classification.
        for draft in &reconciliation.bootstrap {
            if !self.original_source_bootstrap_is_current(draft).await? {
                return Ok(None);
            }
        }
        let revision = {
            let Ok(table) = self.original_source_views.try_lock() else {
                return Ok(None);
            };
            if table.current_revision() != Some(final_revision) {
                return Ok(None);
            }
            final_revision
        };
        Ok(Some(OriginalSourceViewSettlement {
            scopes: reconciliation.scopes,
            revision,
        }))
    }

    async fn original_source_bootstrap_is_current(
        &self,
        draft: &OriginalSourceViewDraft,
    ) -> DocumentSourceResult<bool> {
        let epoch = draft.producer.seed.analyser_inputs_epoch;
        let factory = draft.producer.seed.class_factory_generation;
        let Some(saved) = self
            .readonly_saved_sources(&[&draft.physical.publication])
            .await?
        else {
            return Ok(false);
        };
        let saved = saved
            .iter()
            .map(|source| (source.admission.as_ref(), source.bytes.as_slice()))
            .collect::<Vec<_>>();
        let current = self
            .with_current_readonly_source_state(
                epoch,
                factory,
                &[draft.physical.admission.as_ref()],
                Some(&saved),
                |state| {
                    draft
                        .matches_physical_state(state, epoch, factory)
                        .then_some(())
                },
                |_, ()| Some(()),
            )
            .await?;
        Ok(current.is_some())
    }

    /// Coverage here means every current association to a candidate parent
    /// owns an actual materialized producer and completed source-view scope.
    /// A target URI or potential-body `NoCut` cannot manufacture that producer.
    async fn current_original_scope_associations(
        &self,
        scopes: &[OriginalSourceRootScope],
    ) -> DocumentSourceResult<Option<(HashSet<Uri>, HashSet<Uri>)>> {
        let mut actual = HashSet::new();
        let mut coverage = HashMap::<Uri, bool>::new();
        for scope in scopes {
            if scope.root.is_some_and(|root| {
                self.original_source_views
                    .try_lock()
                    .is_ok_and(|table| !table.by_id.contains_key(&root))
            }) {
                continue;
            }
            let owners = self
                .with_original_source_scope(&scope.ids, |views| Some(views.to_vec()))
                .await?;
            let complete = scope.complete && owners.is_some();
            for association in &scope.association_owners {
                let owner = &association.owner;
                if self
                    .with_current_original_source_view(owner.id, |current| {
                        current.draft.same_live_producer(&owner.draft).then_some(())
                    })
                    .await?
                    .is_none()
                {
                    coverage.insert(association.target.clone(), false);
                    continue;
                }
                actual.insert(association.target.clone());
                let materialized = complete
                    && owners.as_ref().is_some_and(|views| {
                        let Some(SourceFrontier::CurrentCut(cut)) =
                            association.edge.original_source_frontier.as_deref()
                        else {
                            return false;
                        };
                        cut.matches_parent_analysis(&owner.draft.producer.seed.analysis)
                            && association.edge.original_project_source_load.as_deref()
                                == Some(cut.source_load())
                            && views.iter().any(|child| {
                                child.draft.physical.uri == association.target
                                    && child.draft.region.is_none()
                                    && child.draft.inherited.as_ref().is_some_and(|inherited| {
                                        inherited.parent.id == owner.id
                                            && inherited
                                                .parent
                                                .draft
                                                .same_live_producer(&owner.draft)
                                            && inherited.point.as_ref() == cut.point()
                                            && cut.point().matches_child_analysis(
                                                &child.draft.producer.seed.analysis,
                                            )
                                    })
                            })
                    });
                coverage
                    .entry(association.target.clone())
                    .and_modify(|old| *old &= materialized)
                    .or_insert(materialized);
            }
        }
        let completed = coverage
            .into_iter()
            .filter_map(|(uri, complete)| complete.then_some(uri))
            .collect();
        Ok(Some((actual, completed)))
    }

    /// Synchronous retirement already made saved handles terminal. Pruning
    /// retracts their compatibility rows under the publication transaction; it
    /// never reacquires a retired producer from URI, text or equal metadata.
    async fn prune_retired_original_source_view_presentations(&self) -> bool {
        let _publication = self.rehoming_gate.lock().await;
        let Ok(mut table) = self.original_source_views.try_lock() else {
            return false;
        };
        let Ok(mut index) = self.workspace_index.try_write() else {
            return false;
        };
        // A legacy index replacement has already revoked membership: saved
        // handles fail their exact indexed-owner check. Retire those locators
        // and descendants before building any replacement source views.
        let lost = table
            .by_id
            .values()
            .filter(|view| !view.is_indexed(&index))
            .map(|view| view.id)
            .collect();
        table.retire_ids(lost);
        let live = table
            .by_id
            .values()
            .map(|view| {
                tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(view.id)
            })
            .collect::<Vec<_>>();
        index
            .retain_original_source_view_publications(&live)
            .is_some()
    }

    /// Root discovery never treats flattened edge rows as live compiler
    /// authority. Existing interpreted membership has no fresh-root fallback.
    pub(super) async fn original_source_root_work(
        &self,
    ) -> DocumentSourceResult<Option<OriginalSourceRootWork>> {
        let uris = self.workspace_index.read().await.document_uris();
        let mut work = OriginalSourceRootWork {
            candidates: Vec::new(),
            pending: HashSet::new(),
            associated: HashSet::new(),
            incoming: HashMap::new(),
        };
        for uri in uris {
            let Ok(uri) = uri.parse::<Uri>() else {
                return Ok(None);
            };
            if let Some(view) = self.cached_original_source_view(&uri, None).await? {
                work.candidates.push(OriginalSourceRootCandidate {
                    draft: view.draft.clone(),
                    published: Some(view),
                });
                continue;
            }
            let interpreted = {
                let Ok(table) = self.original_source_views.try_lock() else {
                    return Ok(None);
                };
                table.by_uri.get(&uri).into_iter().flatten().any(|id| {
                    table
                        .by_id
                        .get(id)
                        .is_some_and(|view| view.draft.is_interpreted_member())
                })
            };
            if interpreted {
                continue;
            }
            let snapshot = self.rehoming_document_snapshot(&uri).await?;
            let super::super::super::RehomingDocumentSnapshot::Ready(document) = snapshot else {
                work.pending.insert(uri);
                continue;
            };
            match Self::original_declaration_document_participates(&uri, &document) {
                Some(false) => continue,
                Some(true) => {}
                None => {
                    work.pending.insert(uri);
                    continue;
                }
            }
            let Some(draft) = self
                .capture_initial_original_source_root(&uri, *document)
                .await?
            else {
                work.pending.insert(uri);
                continue;
            };
            work.candidates.push(OriginalSourceRootCandidate {
                draft,
                published: None,
            });
        }
        // Classification is tentative until every candidate's exact physical,
        // config, input and producer lease has rejoined after its worker.
        for candidate in &work.candidates {
            let Some(edges) = self.original_source_candidate_edges(candidate).await? else {
                work.pending.insert(candidate.draft.physical.uri.clone());
                continue;
            };
            for edge in edges {
                if let Some(target) = Self::source_draft_target(&candidate.draft, &edge) {
                    work.incoming
                        .entry(target.clone())
                        .or_default()
                        .insert(candidate.draft.physical.uri.clone());
                    work.associated.insert(target);
                }
            }
        }
        Ok(Some(work))
    }

    async fn original_source_candidate_edges(
        &self,
        candidate: &OriginalSourceRootCandidate,
    ) -> DocumentSourceResult<Option<Vec<tcl_lsp_core::workspace_index::WorkspaceSource>>> {
        let draft = candidate.draft.clone();
        let edges = super::super::super::rt::spawn_blocking(move || {
            tcl_lsp_core::workspace_index::WorkspaceOriginalSourceView::source_edges_for_analysis(
                draft.physical.uri.as_str(),
                &draft.physical.source,
                &draft.producer.seed.analysis,
                None,
            )
        })
        .await
        .map_err(|_| super::super::super::DocumentSourceUnavailable::Worker {
            uri: candidate.draft.physical.uri.clone(),
        })?;
        let Some(edges) = edges else {
            return Ok(None);
        };
        self.with_current_original_root_candidate(candidate, || Some(edges.clone()))
            .await
    }

    /// Private bootstrap lease: no publication key is minted to validate a
    /// tentative authoring candidate. Equal text cannot renew a published view.
    async fn with_current_original_root_candidate<T>(
        &self,
        candidate: &OriginalSourceRootCandidate,
        mut render: impl FnMut() -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        let draft = &candidate.draft;
        let epoch = draft.producer.seed.analyser_inputs_epoch;
        let factory = draft.producer.seed.class_factory_generation;
        let Some(saved) = self
            .readonly_saved_sources(&[&draft.physical.publication])
            .await?
        else {
            return Ok(None);
        };
        let saved = saved
            .iter()
            .map(|source| (source.admission.as_ref(), source.bytes.as_slice()))
            .collect::<Vec<_>>();
        self.with_current_readonly_source_state(
            epoch,
            factory,
            &[draft.physical.admission.as_ref()],
            Some(&saved),
            |state| {
                draft
                    .matches_physical_state(state, epoch, factory)
                    .then_some(())
            },
            |_, ()| {
                let _publication = self.rehoming_gate.try_lock().ok()?;
                let table = self.original_source_views.try_lock().ok()?;
                let index = self.workspace_index.try_read().ok()?;
                if let Some(view) = &candidate.published {
                    (table.contains_actual(view)
                        && view.is_indexed(&index)
                        && view.draft.same_live_producer(draft))
                    .then_some(())?;
                } else {
                    (!table
                        .by_uri
                        .get(&draft.physical.uri)
                        .into_iter()
                        .flatten()
                        .any(|id| {
                            table
                                .by_id
                                .get(id)
                                .is_some_and(|view| view.draft.is_interpreted_member())
                        }))
                    .then_some(())?;
                }
                render()
            },
        )
        .await
    }
}
