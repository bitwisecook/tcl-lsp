// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact analysis owners retained alongside flattened workspace presentations.

use super::{AnalysisResult, WorkspaceIndex};
use crate::original_declaration::{OriginalDeclarationDocument, OriginalSourceViewPublication};
use std::sync::Arc;

/// A host-selected source interpretation. This carrier supplies ownership only;
/// its host publication must still be rejoined before rendering a response.
#[derive(Debug, Clone)]
pub struct WorkspaceOriginalSourceView {
    publication: OriginalSourceViewPublication,
    source: Arc<str>,
    analysis: Arc<AnalysisResult>,
    region: Option<tcl_compiler::command_binding::OriginalSourceConditionalRegion>,
}

impl WorkspaceOriginalSourceView {
    /// Retain the actual current producer, never rebuild it from URI/text.
    #[must_use]
    pub fn capture(
        publication: OriginalSourceViewPublication,
        source: Arc<str>,
        analysis: Arc<AnalysisResult>,
    ) -> Option<Self> {
        let row = Self {
            publication,
            source,
            analysis,
            region: None,
        };
        row.matches_owned_scope().then_some(row)
    }

    /// Retain one exact issued partial view without relabelling it whole.
    #[must_use]
    pub fn capture_region(
        publication: OriginalSourceViewPublication,
        source: Arc<str>,
        analysis: Arc<AnalysisResult>,
        region: tcl_compiler::command_binding::OriginalSourceConditionalRegion,
    ) -> Option<Self> {
        let row = Self {
            publication,
            source,
            analysis,
            region: Some(region),
        };
        row.matches_owned_scope().then_some(row)
    }

    fn matches_owned_scope(&self) -> bool {
        Self::matches_analysis_scope(&self.source, &self.analysis, self.region.as_ref())
    }

    fn matches_analysis_scope(
        source: &str,
        analysis: &AnalysisResult,
        region: Option<&tcl_compiler::command_binding::OriginalSourceConditionalRegion>,
    ) -> bool {
        let Some(input) = analysis.resolved_input.as_ref() else {
            return false;
        };
        if analysis.analysis_context_unavailable.is_some()
            || analysis.body_lexer_config != Some(input.lexer_config())
            || !analysis
                .retained_command_realm()
                .is_some_and(|realm| realm.matches_resolved_analysis_input(input))
        {
            return false;
        }
        match region {
            None => {
                crate::original_context::CurrentSourceContext::capture(source, analysis).is_some()
            }
            Some(region) => analysis.body_lexer_config.is_some_and(|config| {
                region.matches_region_analysis(analysis)
                    && analysis.original_source_region() == Some(region)
                    && analysis.matches_original_source_image(
                        &tcl_lexer::SourceImage::document(source),
                        config,
                    )
            }),
        }
    }

    /// Exact partial purpose retained with this publication, if any.
    #[must_use]
    pub fn region(
        &self,
    ) -> Option<&tcl_compiler::command_binding::OriginalSourceConditionalRegion> {
        self.region.as_ref()
    }

    /// Genuine source rows reissued from this exact retained analysis.
    /// Unavailable ownership is distinct from a completed empty inventory.
    #[must_use]
    pub fn source_edges(&self, uri: &str) -> Option<Vec<super::WorkspaceSource>> {
        Self::source_edges_for_analysis(uri, &self.source, &self.analysis, self.region.as_ref())
    }

    /// Inspect actual source association issuers before a host publishes a
    /// locator. This creates no view key, child interpretation, loading receipt,
    /// completed world or permission to analyse a missing inherited context.
    /// The host independently retains and rejoins the physical publication.
    #[must_use]
    pub fn source_edges_for_analysis(
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        region: Option<&tcl_compiler::command_binding::OriginalSourceConditionalRegion>,
    ) -> Option<Vec<super::WorkspaceSource>> {
        Self::matches_analysis_scope(source, analysis, region).then_some(())?;
        let mut rows = super::DocumentRecords::default();
        rows.index_sources(uri, analysis);
        Some(rows.sources)
    }

    /// Host-owned equality stamp; equality alone does not establish currency.
    #[must_use]
    pub fn publication(&self) -> &OriginalSourceViewPublication {
        &self.publication
    }

    /// The same producer analysis allocation retained during publication.
    #[must_use]
    pub fn analysis(&self) -> &Arc<AnalysisResult> {
        &self.analysis
    }

    /// Complete physical source supplied to this interpretation.
    #[must_use]
    pub fn source(&self) -> &Arc<str> {
        &self.source
    }

    /// Borrow this exact view for a Core query after the host currency join.
    #[must_use]
    pub fn document<'a>(&'a self, uri: &'a str) -> OriginalDeclarationDocument<'a> {
        OriginalDeclarationDocument {
            uri,
            source: &self.source,
            analysis: &self.analysis,
        }
    }
}

impl WorkspaceIndex {
    /// Publish all first-edge interpretations as one document transaction.
    /// Scalar presentation caches remain compatibility data; source-aware
    /// providers consume the retained rows instead of the last scalar owner.
    pub fn replace_original_source_views(
        &mut self,
        uri: &str,
        views: Vec<WorkspaceOriginalSourceView>,
    ) -> Option<u64> {
        if views.is_empty()
            || views
                .iter()
                .any(|view| view.region.is_some() || !view.matches_owned_scope())
            || views.iter().enumerate().any(|(i, view)| {
                views[..i]
                    .iter()
                    .any(|old| old.publication == view.publication)
            })
        {
            return None;
        }
        let revision = self.allocate_document_revision();
        let slot = self.slot_for(uri);
        self.docs[slot].clear();
        for view in &views {
            self.docs[slot].index_document(uri, &view.analysis);
        }
        self.docs[slot].original_source_views = views;
        self.docs[slot].revision = revision;
        self.invalidate(true, slot);
        Some(revision)
    }

    /// Append actual partial regions atomically while retaining every prefix
    /// membership. Scalar compatibility records receive no partial metadata.
    pub fn append_original_source_regions(
        &mut self,
        uri: &str,
        regions: Vec<WorkspaceOriginalSourceView>,
    ) -> Option<u64> {
        let slot = *self.slots.get(uri)?;
        let existing = &self.docs[slot].original_source_views;
        if existing.is_empty()
            || regions.is_empty()
            || regions.iter().any(|row| {
                row.region.is_none()
                    || !row.matches_owned_scope()
                    || existing
                        .iter()
                        .any(|old| old.publication == row.publication || old.source != row.source)
            })
            || regions.iter().enumerate().any(|(index, row)| {
                regions[..index]
                    .iter()
                    .any(|old| old.publication == row.publication)
            })
        {
            return None;
        }
        let revision = self.allocate_document_revision();
        self.docs[slot].original_source_views.extend(regions);
        self.docs[slot].revision = revision;
        self.invalidate(true, slot);
        Some(revision)
    }

    /// Add another authentic whole interpretation without retiring current
    /// sibling or suffix memberships. The host owns their physical currency.
    pub fn append_original_source_interpretations(
        &mut self,
        uri: &str,
        views: Vec<WorkspaceOriginalSourceView>,
    ) -> Option<u64> {
        if views.is_empty()
            || views
                .iter()
                .any(|row| row.region.is_some() || !row.matches_owned_scope())
            || views.iter().enumerate().any(|(at, row)| {
                views[..at]
                    .iter()
                    .any(|old| old.publication == row.publication)
            })
        {
            return None;
        }
        let slot = self.slot_for(uri);
        let existing = &self.docs[slot].original_source_views;
        if existing.is_empty() {
            return self.replace_original_source_views(uri, views);
        }
        if views.iter().any(|row| {
            existing
                .iter()
                .any(|old| old.publication == row.publication || old.source != row.source)
        }) {
            return None;
        }
        let revision = self.allocate_document_revision();
        for row in &views {
            self.docs[slot].index_document(uri, &row.analysis);
        }
        self.docs[slot].original_source_views.extend(views);
        self.docs[slot].revision = revision;
        self.invalidate(true, slot);
        Some(revision)
    }

    /// Remove retired interpretation presentations without analysing source,
    /// issuing replacement views or changing any surviving publication stamp.
    /// Partial regions remain excluded from scalar compatibility metadata.
    pub fn retain_original_source_view_publications(
        &mut self,
        live: &[OriginalSourceViewPublication],
    ) -> Option<()> {
        let mut changes = Vec::new();
        for (slot, document) in self.docs.iter().enumerate() {
            let old = &document.original_source_views;
            if old.is_empty() {
                continue;
            }
            let retained = old
                .iter()
                .filter(|row| live.contains(row.publication()))
                .cloned()
                .collect::<Vec<_>>();
            if retained.len() == old.len() {
                continue;
            }
            if retained.iter().any(|row| !row.matches_owned_scope()) {
                return None;
            }
            changes.push((slot, document.uri.clone(), retained));
        }
        // Validate all surviving owners before retracting any presentation.
        for (slot, uri, retained) in changes {
            let revision = self.allocate_document_revision();
            self.docs[slot].clear();
            self.docs[slot].uri.clone_from(&uri);
            for row in &retained {
                if row.region.is_none() {
                    self.docs[slot].index_document(&uri, &row.analysis);
                }
            }
            self.docs[slot].original_source_views = retained;
            self.docs[slot].revision = revision;
            self.invalidate(true, slot);
        }
        Some(())
    }

    /// Actual interpretation rows for this physical URI. A legacy replacement
    /// or accumulation clears this inventory, so it cannot renew old view IDs.
    #[must_use]
    pub fn original_source_views(&self, uri: &str) -> &[WorkspaceOriginalSourceView] {
        self.slots.get(uri).map_or(&[], |slot| {
            self.docs[*slot].original_source_views.as_slice()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_source_view_capture_keeps_unavailable_context_terminal() {
        let source: Arc<str> = Arc::from("proc sample {} {return VALUE}\n");
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl8.6");
        let publication = OriginalSourceViewPublication::retain(1_u64);
        assert!(
            WorkspaceOriginalSourceView::capture(
                publication.clone(),
                Arc::clone(&source),
                Arc::new(analysis.clone()),
            )
            .is_some()
        );
        let mut unavailable = analysis.clone();
        unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
            environment: "tcl8.6".to_owned(),
            overlay: 1,
        });
        assert!(
            WorkspaceOriginalSourceView::capture(
                publication.clone(),
                Arc::clone(&source),
                Arc::new(unavailable),
            )
            .is_none()
        );
        let mut no_input = analysis.clone();
        no_input.resolved_input = None;
        assert!(
            WorkspaceOriginalSourceView::capture(
                publication.clone(),
                Arc::clone(&source),
                Arc::new(no_input),
            )
            .is_none()
        );
        let mut wrong_config = analysis;
        wrong_config.body_lexer_config = None;
        assert!(
            WorkspaceOriginalSourceView::capture(publication, source, Arc::new(wrong_config),)
                .is_none()
        );
    }
}
