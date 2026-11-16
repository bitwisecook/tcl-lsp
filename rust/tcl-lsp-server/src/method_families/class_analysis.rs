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

//! Analyses carrying the workspace class set, memoised per document.

use super::super::Backend;
use super::super::configuration::ResourceAnalyserInputs;
use super::super::documents::DocumentSourceResult;
use std::collections::HashSet;
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tower_lsp_server::ls_types::Uri;

/// One memoised workspace-class-oracle analysis — see
/// [`crate::BackendState::workspace_class_analyses`].
pub(in super::super) struct WorkspaceClassAnalysis {
    /// Hash of everything the analysis depends on: the document's text and
    /// dialect, both workspace class sets, and the class-factory oracle.
    /// Recomputing it is a few microseconds against the several milliseconds
    /// of a re-analysis.
    pub(in super::super) fingerprint: (u64, u64, u64, u64),
    /// The complete source/context owner the secondary analysis borrowed.
    pub(in super::super) source_analysis: Option<Arc<AnalysisResult>>,
    pub(in super::super) resource_inputs: Option<ResourceAnalyserInputs>,
    pub(in super::super) analysis: Arc<AnalysisResult>,
}

/// Hash of one document's analysis inputs, for
/// [`WorkspaceClassAnalysis::fingerprint`].
fn document_fingerprint(source: &str, dialect: &str) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::hash::DefaultHasher::new();
    source.hash(&mut hasher);
    dialect.hash(&mut hasher);
    hasher.finish()
}

/// Order-independent hash of the workspace class set, for
/// [`WorkspaceClassAnalysis::fingerprint`].  The set comes out of a
/// `HashSet`, so iteration order carries no meaning and the combining step
/// must not depend on it.
fn class_set_fingerprint(classes: &std::collections::HashSet<String>) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut combined = classes.len() as u64;
    for name in classes {
        let mut hasher = std::hash::DefaultHasher::new();
        name.hash(&mut hasher);
        combined = combined.wrapping_add(hasher.finish());
    }
    combined
}

/// Deterministic hash of the registry-derived class-factory oracle, for
/// [`WorkspaceClassAnalysis::fingerprint`].  The oracle is a `BTreeMap`, so
/// its iteration order is part of its stable representation.
fn class_factory_fingerprint(factories: Option<&tcl_compiler::analyser::ClassFactoryIndex>) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::hash::DefaultHasher::new();
    factories.hash(&mut hasher);
    hasher.finish()
}

pub(in super::super) fn workspace_class_analysis_fingerprint(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    classes: &HashSet<String>,
    bare_word_classes: &HashSet<String>,
    factories: Option<&tcl_compiler::analyser::ClassFactoryIndex>,
) -> (u64, u64, u64, u64) {
    (
        document_fingerprint(source, dialect.name),
        class_set_fingerprint(classes),
        class_set_fingerprint(bare_word_classes),
        class_factory_fingerprint(factories),
    )
}

impl Backend {
    /// Analyse current source with workspace class hints and its complete
    /// document input. The cache retains that owner, configuration and class
    /// oracle; hints grant no Native class identity or execution entry.
    pub(in super::super) async fn analyse_with_workspace_classes(
        &self,
        uri: &Uri,
        source: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> DocumentSourceResult<Arc<AnalysisResult>> {
        let base = self
            .analysis_for(uri, Arc::from(source), dialect.name.to_owned())
            .await?;
        Ok(self
            .analyse_with_workspace_classes_from_analysis(uri, source, dialect, base)
            .await)
    }

    /// Fingerprints only choose a cache bucket. The actual retained settings
    /// and actual source owner independently authorize reuse and reanalysis.
    fn workspace_class_source_analyser(
        base: &AnalysisResult,
        source: &str,
        resource: &ResourceAnalyserInputs,
        classes: std::collections::HashSet<String>,
        bare_classes: std::collections::HashSet<String>,
        factories: Option<Arc<tcl_compiler::analyser::ClassFactoryIndex>>,
    ) -> Option<Box<Analyser>> {
        let input = base.resolved_input.as_ref()?;
        let registry = input.borrowed_context_registry().commands();
        let entry = base.original_source_analysis_entry(source, registry, None)?;
        let inherited = entry.options().source_point_entry.is_some();
        let mut analyser = resource.clone().apply(Analyser::new());
        if !base.configure_source_analyser(source, registry, &mut analyser) {
            return None;
        }
        let analyser = analyser
            .with_resolved_input(input.clone())
            .with_source_analysis_entry(entry)
            .with_workspace_classes(classes)
            .with_workspace_bare_word_classes(bare_classes)
            .with_workspace_class_factories(factories);
        if inherited && !base.accepts_source_analyser_settings(source, registry, &analyser) {
            return None;
        }
        Some(Box::new(analyser))
    }

    pub(in super::super) async fn analyse_with_workspace_classes_from_analysis(
        &self,
        uri: &Uri,
        source: &str,
        dialect: &'static tcl_dialect::DialectProfile,
        base: Arc<AnalysisResult>,
    ) -> Arc<AnalysisResult> {
        // Typed unavailable status and missing/stale source ownership are
        // terminal before either a cache hit or a secondary source walk.
        if tcl_compiler::source_graph::current_analysis(source, &base).is_none() {
            return base;
        }
        let Some(input) = base.resolved_input.clone() else {
            return base;
        };
        let resource = self.resource_analyser_inputs(Some(uri)).await;
        let (workspace_classes, workspace_bare_word_classes) = {
            let index = self.workspace_index.read().await;
            (
                index.all_class_qnames(),
                index.bare_word_construction_class_qnames(),
            )
        };
        // The class-factory oracle travels with the class-name set:
        // without it this tier would not see a class the document
        // manufactures through a cross-file metaclass at all, and a method
        // lookup on one would miss where the cached tier resolves it.
        let workspace_class_factories = self.class_factories_for(uri).await;
        let fingerprint = workspace_class_analysis_fingerprint(
            source,
            dialect,
            &workspace_classes,
            &workspace_bare_word_classes,
            workspace_class_factories.as_deref(),
        );
        let Some(analyser) = Self::workspace_class_source_analyser(
            &base,
            source,
            &resource,
            workspace_classes,
            workspace_bare_word_classes,
            workspace_class_factories,
        ) else {
            return base;
        };
        if let Some(hit) = self
            .workspace_class_analyses
            .lock()
            .await
            .get(uri)
            .filter(|entry| {
                entry.fingerprint == fingerprint
                    && entry
                        .source_analysis
                        .as_ref()
                        .is_some_and(|owner| Arc::ptr_eq(owner, &base))
                    && entry.analysis.accepts_source_analyser_settings(
                        source,
                        input.borrowed_context_registry().commands(),
                        &analyser,
                    )
                    && entry.resource_inputs.as_ref() == Some(&resource)
                    && tcl_compiler::source_graph::current_analysis(source, &entry.analysis)
                        .is_some()
            })
        {
            return Arc::clone(&hit.analysis);
        }
        let owned_source = source.to_owned();
        let analysis: Arc<AnalysisResult> = crate::rt::spawn_blocking(move || {
            let mut analyser = analyser;
            Arc::new(analyser.analyse(&owned_source, input.analyser_profile().name))
        })
        .await
        .unwrap_or_else(|_| Arc::clone(&base));
        if Arc::ptr_eq(&analysis, &base) {
            return base;
        }
        if tcl_compiler::source_graph::current_analysis(source, &analysis).is_none() {
            return analysis;
        }
        self.workspace_class_analyses.lock().await.insert(
            uri.clone(),
            WorkspaceClassAnalysis {
                fingerprint,
                source_analysis: Some(base),
                resource_inputs: Some(resource),
                analysis: Arc::clone(&analysis),
            },
        );
        analysis
    }
}
