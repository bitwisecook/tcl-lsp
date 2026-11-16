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

//! The conditional regions of a view: their drafts, their cached rows and the ordered suffix chain the kernel issues.

use super::super::{Arc, Backend, DocumentSourceResult, WorkspaceSymbolFreshInput};
use super::{
    ConditionalRegionSourceView, OriginalSourceRegionWork, OriginalSourceViewAttempt,
    OriginalSourceViewCompletion, OriginalSourceViewDraft, PublishedOriginalSourceView,
};

impl OriginalSourceRegionWork {
    /// A kernel-issued outcome remains ownership only until the host checks
    /// this exact prefix and all retained physical dependencies again.
    pub(super) fn completion(&self) -> Option<OriginalSourceViewCompletion> {
        if !self.chain.complete() {
            return None;
        }
        let outcome = self.chain.normal_outcome()?;
        outcome
            .matches_prefix_analysis(&self.parent.draft.producer.seed.analysis)
            .then_some(())?;
        Some(self.retain_completion(outcome, &[]))
    }

    pub(super) fn completion_through_region(
        &self,
        completed: &OriginalSourceViewCompletion,
    ) -> Option<OriginalSourceViewCompletion> {
        let mut matches = self.views.iter().filter(|view| {
            view.id == completed.prefix.id && view.draft.same_live_producer(&completed.prefix.draft)
        });
        let view = matches.next()?;
        matches.next().is_none().then_some(())?;
        let owned = view.draft.region.as_ref()?;
        (owned.chain == self.chain).then_some(())?;
        let outcome = owned.region.return_outcome(
            &self.parent.draft.producer.seed.analysis,
            &self.child_prefix.draft.producer.seed.analysis,
            &view.draft.producer.seed.analysis,
            &completed.outcome,
        )?;
        Some(self.retain_completion(outcome, &completed.dependencies))
    }

    fn retain_completion(
        &self,
        outcome: tcl_compiler::command_binding::OriginalSourceConditionalOutcome,
        additional: &[Arc<PublishedOriginalSourceView>],
    ) -> OriginalSourceViewCompletion {
        let mut dependencies = self.completion_dependencies.clone();
        for view in std::iter::once(&self.child_prefix)
            .chain(&self.views)
            .chain(additional)
        {
            if !dependencies.iter().any(|old| old.id == view.id) {
                dependencies.push(Arc::clone(view));
            }
        }
        OriginalSourceViewCompletion {
            prefix: Arc::clone(&self.parent),
            outcome,
            dependencies,
        }
    }
}

impl Backend {
    pub(super) async fn capture_conditional_region_draft(
        &self,
        request: ConditionalRegionSourceView,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<OriginalSourceViewDraft>> {
        let ids = std::iter::once(request.parent.id)
            .chain(std::iter::once(request.child_prefix.id))
            .chain(request.completion_dependencies.iter().map(|view| view.id))
            .collect::<Vec<_>>();
        if self
            .with_original_source_scope(&ids, |views| {
                let dependencies = [request.parent.as_ref(), request.child_prefix.as_ref()];
                dependencies
                    .into_iter()
                    .chain(request.completion_dependencies.iter().map(AsRef::as_ref))
                    .all(|saved| {
                        views.iter().any(|current| {
                            current.id == saved.id && current.draft.same_live_producer(&saved.draft)
                        })
                    })
                    .then_some(())
            })
            .await?
            .is_none()
        {
            return Ok(OriginalSourceViewAttempt::RetryCapture);
        }
        let physical = Arc::clone(&request.parent.draft.physical);
        let mut driver = self
            .fresh_analysis_driver(Arc::clone(&physical.admission))
            .await;
        driver.analyser = driver
            .analyser
            .with_resolved_input(request.parent.draft.producer.input.as_ref().clone());
        let region = request.region.clone();
        let parent = Arc::clone(&request.parent.draft.producer.seed.analysis);
        let child = Arc::clone(&request.child_prefix.draft.producer.seed.analysis);
        let seed = super::super::super::rt::spawn_blocking(move || {
            driver.analyse_conditional_region(&region, &parent, &child)
        })
        .await
        .map_err(|_| super::super::super::DocumentSourceUnavailable::Worker {
            uri: physical.uri.clone(),
        })??;
        let Some(seed) = seed else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let Some(input) = seed.analysis.resolved_input.clone().map(Arc::new) else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let producer = Arc::new(WorkspaceSymbolFreshInput {
            purpose: super::super::FreshSourceAnalysisPurpose::ConditionalSourceRegion,
            uri: physical.uri.clone(),
            snapshot: physical.publication.document.clone(),
            was_open: physical.publication.open_revision.is_some(),
            seed,
            input,
            config: physical.config,
            configuration: physical.configuration.clone(),
            database_revision: physical.database_revision,
            uri_generation: physical.publication.uri_generation,
        });
        let draft = OriginalSourceViewDraft {
            physical,
            producer,
            inherited: None,
            region: Some(request),
            incoming_paths: None,
        };
        Ok(OriginalSourceViewAttempt::from_semantic(
            draft.matches_producer().then_some(draft),
        ))
    }

    async fn cached_conditional_source_region(
        &self,
        request: &ConditionalRegionSourceView,
    ) -> DocumentSourceResult<Option<Arc<PublishedOriginalSourceView>>> {
        let candidates = {
            let Ok(table) = self.original_source_views.try_lock() else {
                return Ok(None);
            };
            table
                .by_id
                .values()
                .filter(|view| {
                    view.draft
                        .region
                        .as_ref()
                        .is_some_and(|owned| owned.same_request(request))
                })
                .map(|view| view.id)
                .collect::<Vec<_>>()
        };
        if candidates.len() != 1 {
            return Ok(None);
        }
        self.with_current_original_source_view(candidates[0], |view| Some(Arc::clone(view)))
            .await
    }

    fn completed_source_child_matches_view(
        completed: &OriginalSourceViewCompletion,
        child_prefix: &PublishedOriginalSourceView,
    ) -> bool {
        completed.prefix.id == child_prefix.id
            && completed
                .prefix
                .draft
                .same_live_producer(&child_prefix.draft)
            && completed
                .outcome
                .matches_prefix_analysis(&child_prefix.draft.producer.seed.analysis)
    }

    /// Ask the existing kernel for the exact ordered suffix chain. There is no
    /// fresh whole-parent analysis or URI/text reconstruction on this route.
    pub(super) async fn materialize_conditional_source_regions(
        &self,
        parent: Arc<PublishedOriginalSourceView>,
        child_prefix: Arc<PublishedOriginalSourceView>,
        load: Arc<tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad>,
        completed_child: Option<&OriginalSourceViewCompletion>,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<OriginalSourceRegionWork>> {
        let outcome = if let Some(completed) = completed_child {
            if !Self::completed_source_child_matches_view(completed, &child_prefix) {
                return Ok(OriginalSourceViewAttempt::Unavailable);
            }
            Some(completed.outcome.clone())
        } else {
            None
        };
        let completion_dependencies =
            completed_child.map_or_default(|completed| completed.dependencies.clone());
        let parent_analysis = Arc::clone(&parent.draft.producer.seed.analysis);
        let child_analysis = Arc::clone(&child_prefix.draft.producer.seed.analysis);
        let chain = super::super::super::rt::spawn_blocking(move || {
            let continuation = if let Some(outcome) = outcome.as_ref() {
                load.conditional_continuation_with_outcome(
                    &parent_analysis,
                    &child_analysis,
                    outcome,
                )
            } else {
                load.conditional_continuation(&parent_analysis, &child_analysis)
            }?;
            continuation
                .source_regions(&parent_analysis, &child_analysis)
                .map(Arc::new)
        })
        .await
        .map_err(|_| super::super::super::DocumentSourceUnavailable::Worker {
            uri: parent.draft.physical.uri.clone(),
        })?;
        let Some(chain) = chain else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let mut views = Vec::new();
        let mut pending = Vec::new();
        for region in chain.regions() {
            let request = ConditionalRegionSourceView {
                parent: Arc::clone(&parent),
                child_prefix: Arc::clone(&child_prefix),
                completion_dependencies: completion_dependencies.clone(),
                chain: Arc::clone(&chain),
                region: region.clone(),
            };
            if let Some(cached) = self.cached_conditional_source_region(&request).await? {
                views.push(Some(cached));
            } else {
                let draft = match self.capture_conditional_region_draft(request).await? {
                    OriginalSourceViewAttempt::Ready(draft) => draft,
                    OriginalSourceViewAttempt::RetryCapture => {
                        return Ok(OriginalSourceViewAttempt::RetryCapture);
                    }
                    OriginalSourceViewAttempt::Unavailable => {
                        return Ok(OriginalSourceViewAttempt::Unavailable);
                    }
                };
                pending.push((views.len(), draft));
                views.push(None);
            }
        }
        if !pending.is_empty() {
            let (positions, drafts): (Vec<_>, Vec<_>) = pending.into_iter().unzip();
            let published = match self
                .append_original_source_regions(&parent.draft.physical.uri, drafts)
                .await?
            {
                OriginalSourceViewAttempt::Ready(published) => published,
                OriginalSourceViewAttempt::RetryCapture => {
                    return Ok(OriginalSourceViewAttempt::RetryCapture);
                }
                OriginalSourceViewAttempt::Unavailable => {
                    return Ok(OriginalSourceViewAttempt::Unavailable);
                }
            };
            if published.len() != positions.len() {
                return Ok(OriginalSourceViewAttempt::Unavailable);
            }
            for (position, view) in positions.into_iter().zip(published) {
                views[position] = Some(view);
            }
        }
        let Some(views) = views.into_iter().collect::<Option<Vec<_>>>() else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        Ok(OriginalSourceViewAttempt::Ready(OriginalSourceRegionWork {
            parent,
            child_prefix,
            chain,
            views,
            completion_dependencies,
        }))
    }
}
