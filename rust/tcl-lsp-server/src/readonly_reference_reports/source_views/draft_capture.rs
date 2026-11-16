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

//! Producing the draft of an initial root and of a child inherited through a source edge.

use super::super::{Arc, Backend, DocumentSourceResult, WorkspaceSymbolFreshInput};
use super::{
    IncomingSourcePathAdvice, InheritedSourceView, OriginalSourceViewAttempt,
    OriginalSourceViewDraft, PublishedOriginalSourceView,
};
use tcl_compiler::command_binding::OriginalSourcePointEntry;
use tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceCurrentCut;
use tower_lsp_server::ls_types::Uri;

impl super::super::super::FreshAnalysisDriver {
    pub(super) fn analyse_conditional_region(
        mut self: Box<Self>,
        region: &tcl_compiler::command_binding::OriginalSourceConditionalRegion,
        parent: &tcl_compiler::analyser::AnalysisResult,
        child_prefix: &tcl_compiler::analyser::AnalysisResult,
    ) -> DocumentSourceResult<Option<super::super::super::FreshAnalysisSeed>> {
        self.path_admission
            .check_current(self.source_store.as_ref())?;
        let analysis = self
            .analyser
            .analyse_source_region(region, parent, child_prefix);
        self.path_admission
            .check_current(self.source_store.as_ref())?;
        let path_admission = self.path_admission;
        let analyser_inputs_epoch = &self.analyser_inputs_epoch;
        let class_factory_generation = &self.class_factory_generation;
        Ok(
            analysis.map(|analysis| super::super::super::FreshAnalysisSeed {
                path_admission,
                analysis: Arc::new(analysis),
                analyser_inputs_epoch: *analyser_inputs_epoch,
                class_factory_generation: *class_factory_generation,
            }),
        )
    }

    fn analyse_source_view(
        mut self: Box<Self>,
        source: &str,
        dialect: &str,
        point: &OriginalSourcePointEntry,
    ) -> DocumentSourceResult<Option<super::super::super::FreshAnalysisSeed>> {
        self.path_admission
            .check_current(self.source_store.as_ref())?;
        let source = tcl_lexer::normalise_lone_cr(source);
        let analysis = self
            .analyser
            .analyse_with_source_point_entry(&source, dialect, point);
        self.path_admission
            .check_current(self.source_store.as_ref())?;
        let path_admission = self.path_admission;
        let analyser_inputs_epoch = &self.analyser_inputs_epoch;
        let class_factory_generation = &self.class_factory_generation;
        Ok(
            analysis.map(|analysis| super::super::super::FreshAnalysisSeed {
                path_admission,
                analysis: Arc::new(analysis),
                analyser_inputs_epoch: *analyser_inputs_epoch,
                class_factory_generation: *class_factory_generation,
            }),
        )
    }
}

impl Backend {
    /// Initial root production only. The reconciler must select this route
    /// from an actual root work item; a missing inherited entry never calls it.
    pub(super) async fn capture_initial_original_source_root(
        &self,
        uri: &Uri,
        document: super::super::super::DocumentState,
    ) -> DocumentSourceResult<Option<OriginalSourceViewDraft>> {
        let Some(physical) = self
            .capture_readonly_source_document(uri.as_str(), Arc::clone(&document.text))
            .await?
        else {
            return Ok(None);
        };
        let Some(analysis) = self
            .capture_fresh_source_analysis(
                uri,
                document,
                super::super::FreshSourceAnalysisPurpose::ReadonlyComplete,
            )
            .await?
        else {
            return Ok(None);
        };
        let super::super::WorkspaceSymbolAnalysisKind::Fresh(producer) = analysis.kind else {
            return Ok(None);
        };
        let draft = OriginalSourceViewDraft {
            physical: Arc::new(physical),
            producer: Arc::from(producer),
            inherited: None,
            region: None,
            incoming_paths: None,
        };
        Ok(draft.matches_producer().then_some(draft))
    }

    /// Child production uses the exact still-live parent point once. A child
    /// refusal remains unavailable rather than entering a root/namespace route.
    pub(super) async fn capture_inherited_original_source_view(
        &self,
        uri: &Uri,
        source: Arc<str>,
        parent: Arc<PublishedOriginalSourceView>,
        cut: Arc<OriginalProjectSourceCurrentCut>,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<OriginalSourceViewDraft>> {
        let point = Arc::new(cut.point().clone());
        if self
            .with_current_original_source_view(parent.id, |current| {
                (current.draft.same_live_producer(&parent.draft)
                    && point.matches_parent_analysis(&current.draft.producer.seed.analysis))
                .then_some(())
            })
            .await?
            .is_none()
        {
            return Ok(OriginalSourceViewAttempt::RetryCapture);
        }
        let Some(physical) = self
            .capture_readonly_source_document(uri.as_str(), source)
            .await?
        else {
            return Ok(OriginalSourceViewAttempt::RetryCapture);
        };
        let mut driver = self
            .fresh_analysis_driver(Arc::clone(&physical.admission))
            .await;
        // Physical child configuration is retained independently; the
        // inherited interpreter input belongs to the exact actual point.
        driver.analyser = driver
            .analyser
            .with_resolved_input(parent.draft.producer.input.as_ref().clone());
        if !point.configure_analyser(&parent.draft.producer.seed.analysis, &mut driver.analyser) {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        }
        let text = Arc::clone(&physical.source);
        let dialect = parent.draft.producer.seed.analysis.dialect.clone();
        let point_for_worker = Arc::clone(&point);
        let seed = super::super::super::rt::spawn_blocking(move || {
            driver.analyse_source_view(&text, &dialect, &point_for_worker)
        })
        .await
        .map_err(|_| super::super::super::DocumentSourceUnavailable::Worker {
            uri: uri.clone(),
        })??;
        let Some(seed) = seed else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let Some(interpretation_input) = seed.analysis.resolved_input.clone().map(Arc::new) else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let producer = Arc::new(WorkspaceSymbolFreshInput {
            purpose: super::super::FreshSourceAnalysisPurpose::ReadonlyComplete,
            uri: uri.clone(),
            snapshot: physical.publication.document.clone(),
            was_open: physical.publication.open_revision.is_some(),
            seed,
            input: interpretation_input,
            config: physical.config,
            configuration: physical.configuration.clone(),
            database_revision: physical.database_revision,
            uri_generation: physical.publication.uri_generation,
        });
        let incoming_paths = Some(Arc::new(IncomingSourcePathAdvice {
            constants: parent
                .draft
                .path_constants_before_cut(&cut)
                .map(|values| values.exported()),
            parent: Arc::clone(&parent),
            cut,
        }));
        let draft = OriginalSourceViewDraft {
            physical: Arc::new(physical),
            producer,
            inherited: Some(InheritedSourceView { parent, point }),
            region: None,
            incoming_paths,
        };
        Ok(OriginalSourceViewAttempt::from_semantic(
            draft.matches_producer().then_some(draft),
        ))
    }
}
