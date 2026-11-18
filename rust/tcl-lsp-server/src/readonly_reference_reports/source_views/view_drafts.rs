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

//! A view draft: whether it still matches its producer and its physical state, which request it answers and which views it descends from.

use super::super::{Arc, ReadonlySourceState, original_readonly_analysis_matches_input};
use super::{
    ConditionalRegionSourceView, IncomingSourcePathAdvice, InheritedSourceView,
    OriginalSourceViewDraft, PublishedOriginalSourceView,
};
use std::collections::HashSet;
use tcl_compiler::auto_path_eval::{
    FoldedPathConstants, PathConstantAssignments, PathConstantValue,
};
use tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceCurrentCut;

impl IncomingSourcePathAdvice {
    pub(super) fn matches_inherited(&self, inherited: &InheritedSourceView) -> bool {
        Arc::ptr_eq(&self.parent, &inherited.parent)
            && self.cut.point() == inherited.point.as_ref()
            && self
                .cut
                .matches_parent_analysis(&self.parent.draft.producer.seed.analysis)
    }

    pub(super) fn same_request(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.parent, &other.parent)
            && self.cut == other.cut
            && self.constants == other.constants
    }
}

impl ConditionalRegionSourceView {
    pub(super) fn same_request(&self, other: &Self) -> bool {
        self.parent.id == other.parent.id
            && self.child_prefix.id == other.child_prefix.id
            && self.chain == other.chain
            && self.region == other.region
            && self.completion_dependencies.len() == other.completion_dependencies.len()
            && self
                .completion_dependencies
                .iter()
                .zip(&other.completion_dependencies)
                .all(|(left, right)| left.id == right.id)
    }
}

impl OriginalSourceViewDraft {
    pub(super) fn matches_producer(&self) -> bool {
        let physical = &self.physical;
        let producer = &self.producer;
        if self.incoming_paths.as_ref().is_some_and(|advice| {
            self.inherited
                .as_ref()
                .is_none_or(|inherited| !advice.matches_inherited(inherited))
        }) {
            return false;
        }
        if physical.uri != producer.uri
            || physical.source != producer.snapshot.text
            || physical.publication.uri_generation != producer.uri_generation
            || !producer.open_snapshot_matches(&physical.publication.document)
            || physical.publication.open_revision.is_some() != producer.was_open
            || physical.admission != producer.seed.path_admission
            || physical.config != producer.config
            || physical.configuration != producer.configuration
            || !original_readonly_analysis_matches_input(
                &physical.source,
                &producer.seed.analysis,
                &producer.input,
            )
        {
            return false;
        }
        if let Some(region) = &self.region {
            return self.inherited.is_none()
                && region.chain.regions().contains(&region.region)
                && region.region.matches_dependencies(
                    &region.parent.draft.producer.seed.analysis,
                    &region.child_prefix.draft.producer.seed.analysis,
                )
                && region
                    .region
                    .matches_region_analysis(&producer.seed.analysis);
        }
        match &self.inherited {
            Some(inherited) => {
                inherited
                    .point
                    .matches_parent_analysis(&inherited.parent.draft.producer.seed.analysis)
                    && inherited
                        .point
                        .matches_child_analysis(&producer.seed.analysis)
            }
            // Only the private initial-root producer constructor can mint
            // this variant; inherited/missing-point work cannot call it.
            None => physical.input == producer.input,
        }
    }

    pub(super) fn path_constants_before_cut(
        &self,
        cut: &OriginalProjectSourceCurrentCut,
    ) -> Option<FoldedPathConstants> {
        if !self.matches_producer() {
            return None;
        }
        let raw = cut.path_advice_before_handler(&self.producer.seed.analysis)?;
        self.fold_source_path_prefix(&raw)
    }

    fn fold_source_path_prefix(
        &self,
        raw: &PathConstantAssignments,
    ) -> Option<FoldedPathConstants> {
        let empty = FoldedPathConstants::default();
        let mut imported = &empty;
        if self.inherited.is_some() || self.region.is_some() {
            if raw.is_empty() {
                if let Some(advice) = &self.incoming_paths {
                    imported = advice.constants.as_ref()?;
                }
            } else {
                // Value-only exports cannot prove unknown incoming shadows absent.
                // Only variable-free own RHSs can supply new inherited advice.
                let filename = self.physical.admission.analyser_path();
                if !raw.iter().all(|write| match &write.value {
                    PathConstantValue::Original(expression) => {
                        expression.matches_assignments(raw)
                            && expression.evaluate(filename, &|_| None).is_some()
                    }
                    _ => false,
                }) {
                    return None;
                }
            }
        }
        Some(
            tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports(
                raw,
                self.physical.admission.analyser_path(),
                imported,
            ),
        )
    }

    pub(super) fn matches_physical_state(
        &self,
        state: &ReadonlySourceState<'_>,
        epoch: u64,
        factory: u64,
    ) -> bool {
        let physical = &self.physical;
        let producer = &self.producer;
        // Source-view currency checks the actual source/config/provider inputs.
        // The existing report's global database-revision guard stays unchanged:
        // unrelated child materialisation cannot renew or retire this producer.
        physical.publication.matches_open_documents(state.open)
            && state
                .uri_generations
                .get(&physical.uri)
                .copied()
                .unwrap_or_default()
                == physical.publication.uri_generation
            && state.files.get(&physical.uri) == Some(&physical.file)
            && state.published.get(&physical.uri).is_some_and(|published| {
                published.file == physical.file
                    && published.admission.as_ref() == physical.admission.as_ref()
            })
            && physical.file.path(state.db).as_deref() == physical.admission.analyser_path()
            && physical.file.text(state.db).as_str() == physical.source.as_ref()
            && physical.file.dialect(state.db) == &physical.publication.document.dialect
            && physical.config.diagnostic_report_configuration(state.db) == physical.configuration
            && { tcl_lsp_db::document_analysis_input(state.db, physical.file, physical.config) }
                .is_ok_and(|input| input.as_ref() == physical.input.as_ref())
            && producer.uri_generation_matches(state)
            && producer.matches_input_state(state, epoch, factory)
            && self.matches_producer()
    }

    pub(super) fn same_live_producer(&self, other: &Self) -> bool {
        // This is reuse of an already current producer, never standalone
        // reanalysis or reconstruction from equal bytes/configuration.
        Arc::ptr_eq(&self.producer, &other.producer)
            && Arc::ptr_eq(&self.physical, &other.physical)
            && match (&self.incoming_paths, &other.incoming_paths) {
                (None, None) => true,
                (Some(left), Some(right)) => left.same_request(right),
                _ => false,
            }
            && match (&self.region, &other.region) {
                (None, None) => true,
                (Some(left), Some(right)) => left.same_request(right),
                _ => false,
            }
            && match (&self.inherited, &other.inherited) {
                (None, None) => true,
                (Some(a), Some(b)) => a.parent.id == b.parent.id && a.point == b.point,
                _ => false,
            }
    }

    pub(super) fn same_view_request(&self, other: &Self) -> bool {
        // Subtractive duplicate detection only; equality cannot admit a view
        // or replace the same-live-producer reuse/currentness checks.
        let a = &self.physical;
        let b = &other.physical;
        a.uri == b.uri
            && a.file == b.file
            && a.admission == b.admission
            && a.source == b.source
            && a.input == b.input
            && a.config == b.config
            && a.configuration == b.configuration
            && a.publication.uri_generation == b.publication.uri_generation
            && a.publication.open_revision == b.publication.open_revision
            && self
                .producer
                .open_snapshot_matches(&other.producer.snapshot)
            && match (&self.incoming_paths, &other.incoming_paths) {
                (None, None) => true,
                (Some(left), Some(right)) => left.same_request(right),
                _ => false,
            }
            && match (&self.region, &other.region) {
                (None, None) => true,
                (Some(left), Some(right)) => left.same_request(right),
                _ => false,
            }
            && match (&self.inherited, &other.inherited) {
                (None, None) => true,
                (Some(a), Some(b)) => a.parent.id == b.parent.id && a.point == b.point,
                _ => false,
            }
    }

    fn direct_dependencies(&self) -> Vec<Arc<PublishedOriginalSourceView>> {
        if let Some(region) = &self.region {
            let mut owners = vec![Arc::clone(&region.parent), Arc::clone(&region.child_prefix)];
            owners.extend(region.completion_dependencies.iter().cloned());
            owners
        } else {
            self.inherited
                .as_ref()
                .map_or_default(|inherited| vec![Arc::clone(&inherited.parent)])
        }
    }

    pub(super) fn is_interpreted(&self) -> bool {
        self.inherited.is_some() || self.region.is_some()
    }

    /// A region continues its own physical owner; only an inherited view makes
    /// the URI another root's interpreted member.
    pub(super) fn is_interpreted_member(&self) -> bool {
        self.inherited.is_some()
    }

    /// `true` when `uri` is physically being executed by a view on this draft's
    /// call chain (inherited parents and a region's owner). Completed earlier
    /// children, which a region records as completion dependencies, are not on
    /// the call chain: `source b; source b` is not recursion.
    pub(super) fn call_chain_executes(&self, uri: &tower_lsp_server::ls_types::Uri) -> bool {
        let mut next = self
            .inherited
            .as_ref()
            .map(|inherited| &inherited.parent)
            .or_else(|| self.region.as_ref().map(|region| &region.parent));
        let mut steps = 0usize;
        while let Some(view) = next {
            if &view.draft.physical.uri == uri {
                return true;
            }
            steps += 1;
            if steps > 4096 {
                return true;
            }
            next = view
                .draft
                .inherited
                .as_ref()
                .map(|inherited| &inherited.parent)
                .or_else(|| view.draft.region.as_ref().map(|region| &region.parent));
        }
        false
    }

    pub(super) fn ancestors(&self) -> Option<Vec<Arc<PublishedOriginalSourceView>>> {
        let mut ancestors = Vec::new();
        let mut active = HashSet::new();
        let mut complete = HashSet::new();
        let mut pending = self
            .direct_dependencies()
            .into_iter()
            .map(|view| (view, false))
            .collect::<Vec<_>>();
        while let Some((view, exiting)) = pending.pop() {
            if exiting {
                active.remove(&view.id);
                complete.insert(view.id);
                continue;
            }
            if complete.contains(&view.id) {
                continue;
            }
            if ancestors.len() >= 4096 || !active.insert(view.id) {
                return None;
            }
            pending.push((Arc::clone(&view), true));
            pending.extend(
                view.draft
                    .direct_dependencies()
                    .into_iter()
                    .map(|view| (view, false)),
            );
            ancestors.push(view);
        }
        Some(ancestors)
    }
}
