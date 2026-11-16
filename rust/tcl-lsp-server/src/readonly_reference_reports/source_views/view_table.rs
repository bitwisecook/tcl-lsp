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

//! The session table of published views: staging, reuse, retirement and replacement.

use super::super::Arc;
use super::{
    OriginalSourceViewDraft, OriginalSourceViewId, OriginalSourceViewTable,
    PublishedOriginalSourceView,
};
use std::collections::HashSet;
use tower_lsp_server::ls_types::Uri;

impl OriginalSourceViewTable {
    pub(super) fn current_revision(&self) -> Option<u64> {
        (!self.revision_exhausted).then_some(self.revision)
    }

    fn advance_revision(&mut self) {
        if let Some(next) = self.revision.checked_add(1) {
            self.revision = next;
        } else {
            self.revision_exhausted = true;
        }
    }

    pub(super) fn contains_actual(&self, view: &PublishedOriginalSourceView) -> bool {
        self.by_uri
            .get(&view.draft.physical.uri)
            .is_some_and(|ids| ids.contains(&view.id))
            && self
                .by_id
                .get(&view.id)
                .is_some_and(|current| current.draft.same_live_producer(&view.draft))
    }

    pub(super) fn parent_chain_is_live(&self, draft: &OriginalSourceViewDraft) -> bool {
        draft.ancestors().is_some_and(|ancestors| {
            ancestors
                .iter()
                .all(|ancestor| self.contains_actual(ancestor))
        })
    }

    pub(super) fn reuse_current_batch(
        &self,
        uri: &Uri,
        drafts: &[OriginalSourceViewDraft],
        index: &tcl_lsp_core::workspace_index::WorkspaceIndex,
    ) -> Option<Vec<Arc<PublishedOriginalSourceView>>> {
        let all_ids = self.by_uri.get(uri)?;
        let existing = all_ids
            .iter()
            .filter(|id| {
                self.by_id
                    .get(id)
                    .is_some_and(|view| view.draft.region.is_none())
            })
            .collect::<Vec<_>>();
        if existing.len() != drafts.len()
            || index.original_source_views(uri.as_str()).len() != all_ids.len()
        {
            return None;
        }

        existing
            .iter()
            .zip(drafts)
            .map(|(id, draft)| {
                let view = self.by_id.get(*id)?;
                (view.is_indexed(index) && view.draft.same_live_producer(draft))
                    .then(|| Arc::clone(view))
            })
            .collect()
    }

    pub(super) fn stage(
        &self,
        drafts: Vec<OriginalSourceViewDraft>,
    ) -> Option<(u64, Vec<(OriginalSourceViewId, OriginalSourceViewDraft)>)> {
        let mut next_id = self.next_id;
        let mut staged = Vec::with_capacity(drafts.len());
        for draft in drafts {
            if !draft.matches_producer() || !self.parent_chain_is_live(&draft) {
                return None;
            }
            if staged
                .iter()
                .any(|(_, previous): &(_, OriginalSourceViewDraft)| {
                    previous.same_view_request(&draft)
                })
            {
                return None;
            }
            let existing = self
                .by_id
                .values()
                .find(|view| view.draft.same_live_producer(&draft));
            let id = if let Some(view) = existing {
                view.id
            } else {
                next_id = next_id.checked_add(1)?;
                OriginalSourceViewId(next_id)
            };
            staged.push((id, draft));
        }
        Some((next_id, staged))
    }

    /// Retire both the changed physical owner and every retained descendant.
    /// URI generations and this table use the same URI-generation -> table order.
    pub(crate) fn retire_uri(&mut self, uri: &Uri) {
        // Private bootstrap participants may have no issued view yet. Their
        // physical invalidation must still invalidate an in-flight projection.
        self.advance_revision();
        let retired = self.by_uri.get(uri).cloned().unwrap_or_default();
        self.retire_ids(retired);
    }

    pub(super) fn retire_ids(&mut self, mut retired: Vec<OriginalSourceViewId>) {
        if retired.is_empty() {
            return;
        }
        self.advance_revision();
        loop {
            let more = self
                .by_id
                .iter()
                .filter_map(|(id, view)| {
                    (!retired.contains(id)
                        && view.draft.ancestors().is_none_or(|ancestors| {
                            ancestors.iter().any(|parent| retired.contains(&parent.id))
                        }))
                    .then_some(*id)
                })
                .collect::<Vec<_>>();
            if more.is_empty() {
                break;
            }
            retired.extend(more);
        }
        for id in &retired {
            self.by_id.remove(id);
        }
        self.by_uri.retain(|_, ids| {
            ids.retain(|id| !retired.contains(id));
            !ids.is_empty()
        });
    }

    pub(super) fn append_regions(
        &mut self,
        uri: &Uri,
        next_id: u64,
        views: &[Arc<PublishedOriginalSourceView>],
    ) {
        // All validation precedes the index commit under this same table lock.
        self.advance_revision();
        self.next_id = next_id;
        self.by_uri
            .entry(uri.clone())
            .or_default()
            .extend(views.iter().map(|view| view.id));
        for view in views {
            self.by_id.insert(view.id, Arc::clone(view));
        }
    }

    pub(super) fn replace(
        &mut self,
        uri: &Uri,
        next_id: u64,
        views: &[Arc<PublishedOriginalSourceView>],
    ) {
        let kept = views.iter().map(|view| view.id).collect::<HashSet<_>>();
        let removed = self
            .by_uri
            .get(uri)
            .into_iter()
            .flatten()
            .copied()
            .filter(|id| !kept.contains(id))
            .collect();
        self.retire_ids(removed);
        self.advance_revision();
        self.next_id = next_id;
        self.by_uri
            .insert(uri.clone(), views.iter().map(|view| view.id).collect());
        for view in views {
            self.by_id.insert(view.id, Arc::clone(view));
        }
        // Unchanged actual producers keep their issued IDs when a sibling
        // interpretation is removed. Only removed IDs and their descendants
        // retire; full physical/config invalidation still calls retire_uri.
    }
}

impl PublishedOriginalSourceView {
    pub(super) fn is_indexed(&self, index: &tcl_lsp_core::workspace_index::WorkspaceIndex) -> bool {
        let stamp =
            tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(self.id);
        let mut rows = index
            .original_source_views(self.draft.physical.uri.as_str())
            .iter()
            .filter(|row| row.publication() == &stamp);
        let Some(row) = rows.next() else {
            return false;
        };
        rows.next().is_none()
            && Arc::ptr_eq(row.analysis(), &self.draft.producer.seed.analysis)
            && row.source() == &self.draft.physical.source
    }
}
