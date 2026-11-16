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

//! Analyses run outside the query database: the seed, the driver and their currency checks.

use super::super::configuration::ResourceAnalyserInputs;
use super::super::documents::{
    DocumentPathAdmission, DocumentSourceResult, DocumentSourceUnavailable,
};
use super::super::{Backend, analysis_cooperation, fresh_analysis_cache, vfs};
use super::FreshAnalysisSeed;
use std::collections::HashSet;
use std::sync::Arc;
use tcl_compiler::analyser::Analyser;
use tower_lsp_server::ls_types::Uri;

pub(in super::super) struct FreshAnalysisDriver {
    pub(in super::super) path_admission: Arc<DocumentPathAdmission>,
    pub(in super::super) source_store: Arc<dyn vfs::SourceStore>,
    pub(in super::super) analyser: Analyser,
    pub(in super::super) analyser_inputs_epoch: u64,
    pub(in super::super) class_factory_generation: u64,
    /// The configured inputs `analyser` was built from, for the
    /// [`fresh_analysis_cache`] key.
    pub(in super::super) inputs: fresh_analysis_cache::FreshAnalyserInputs,
}

impl FreshAnalysisDriver {
    /// Keep the document's source identity and interpret nothing: all the
    /// first-paint token tier reads.
    pub(in super::super) fn into_source_shell(mut self: Box<Self>) -> Box<Self> {
        self.analyser = self.analyser.source_shell();
        self
    }

    pub(in super::super) fn analyse_readonly_document(
        mut self: Box<Self>,
        text: &str,
        dialect: &str,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
        external_call_sites: Option<&tcl_compiler::unit_scope::CallSiteEvidence>,
    ) -> DocumentSourceResult<FreshAnalysisSeed> {
        self.path_admission
            .check_current(self.source_store.as_ref())?;
        let text = tcl_lexer::normalise_lone_cr(text);
        let registry = input.borrowed_context_registry().commands();
        let declared = tcl_compiler::analyser::utils::document_declared_surface(
            &text,
            self.path_admission.analyser_path(),
            input.unit_profile().name,
        );
        let settings = self.analyser.source_compilation_settings();
        let unit = tcl_compiler::compilation_unit::CompilationUnit::build_with_analysis_input_and_source_settings(
            &text,
            tcl_compiler::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites,
                declared_commands: Some(&declared),
            },
            None,
            input,
            &settings,
        ).with_interprocedural(registry, Some(input.unit_profile()));
        self.analyser.set_cu_override(Arc::new(unit));
        self.analyse(&text, dialect, None)
    }

    pub(in super::super) fn analyse(
        mut self: Box<Self>,
        text: &str,
        dialect: &str,
        namespace: Option<&str>,
    ) -> DocumentSourceResult<FreshAnalysisSeed> {
        self.path_admission
            .check_current(self.source_store.as_ref())?;
        let text = tcl_lexer::normalise_lone_cr(text);
        let analysis = match namespace {
            Some(namespace) => self
                .analyser
                .analyse_with_source_namespace(&text, dialect, namespace),
            None => self.analyser.analyse(&text, dialect),
        };
        self.path_admission
            .check_current(self.source_store.as_ref())?;
        Ok(FreshAnalysisSeed {
            path_admission: self.path_admission,
            analysis: Arc::new(analysis),
            analyser_inputs_epoch: self.analyser_inputs_epoch,
            class_factory_generation: self.class_factory_generation,
        })
    }
    fn analyse_controlled(
        self: Box<Self>,
        text: &str,
        dialect: &str,
        control: tcl_compiler::analysis_control::AnalysisControl,
    ) -> Result<
        DocumentSourceResult<FreshAnalysisSeed>,
        tcl_compiler::analysis_control::AnalysisInterrupted,
    > {
        if let Err(error) = self
            .path_admission
            .check_current(self.source_store.as_ref())
        {
            return Ok(Err(error));
        }
        let text = tcl_lexer::normalise_lone_cr(text);
        let Self {
            path_admission,
            source_store,
            analyser,
            analyser_inputs_epoch,
            class_factory_generation,
            ..
        } = *self;
        let analysis = analyser.try_analyse_owned(&text, dialect, None, control)?;
        if let Err(error) = path_admission.check_current(source_store.as_ref()) {
            return Ok(Err(error));
        }
        Ok(Ok(FreshAnalysisSeed {
            path_admission,
            analysis: Arc::new(analysis),
            analyser_inputs_epoch,
            class_factory_generation,
        }))
    }
}

impl Backend {
    /// Resolve an analysis for the document.  Consults the
    /// cache first; computes a fresh analysis when no entry
    /// exists.  Returns a shared `Arc` the caller can move into
    /// a `spawn_blocking` worker (providers deref it via `&`).
    ///
    /// `text` is the document snapshot's shared handle, not a copy: the
    /// overwhelmingly common path is a cache hit, which never reads it at all,
    /// and the cache-miss path moves the handle into the blocking worker and
    /// analyses through a deref.  Taking `Arc<str>` is what keeps the ~30
    /// `analysis_for` call sites from each copying the whole document per
    /// request.
    pub(in super::super) async fn analysis_for(
        &self,
        uri: &Uri,
        text: Arc<str>,
        dialect: String,
    ) -> DocumentSourceResult<Arc<tcl_compiler::analyser::AnalysisResult>> {
        let admission = self.document_path_admission(uri).await?;
        let sidecar_revision = self
            .fresh_analysis_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .published_revision();
        let epoch = self.diag_inputs_epoch();
        if let Some(sidecar_revision) = sidecar_revision
            && let Some(cached) = self.cached_analysis(uri).await
        {
            let published = self
                .published_document_path_admission(uri)
                .await?
                .ok_or_else(|| DocumentSourceUnavailable::PublishedPathUnavailable {
                    uri: uri.clone(),
                })?;
            if published.as_ref() != admission.as_ref() {
                return Err(DocumentSourceUnavailable::PublishedPathUnavailable {
                    uri: uri.clone(),
                });
            }
            admission.check_for(uri, self.store.as_ref())?;
            let mut driver = self.fresh_analysis_driver(admission).await;
            if let Ok(input) = driver.analyser.prepare_analysis_input(&dialect) {
                let generation = self.class_factory_generation.read().await;
                if epoch == driver.analyser_inputs_epoch
                    && self.diag_inputs_epoch() == driver.analyser_inputs_epoch
                    && *generation == driver.class_factory_generation
                    && self
                        .fresh_analysis_cache
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .published_revision_is_current(&sidecar_revision)
                    && cached.resolved_input.as_ref() == Some(&input)
                    && cached.accepts_source_analyser_settings(
                        &text,
                        input.borrowed_context_registry().commands(),
                        &driver.analyser,
                    )
                {
                    driver.path_admission.check_for(uri, self.store.as_ref())?;
                    return Ok(cached);
                }
            }
        }
        self.fresh_analysis_for(uri, text, dialect).await
    }

    /// Analyse `text` without taking a Salsa snapshot.
    ///
    /// Most request paths should use [`Self::analysis_for`] so they share the
    /// memoised query. The cold `didOpen` index seed deliberately uses this
    /// path: a newer edit must be able to publish to Salsa and supersede that
    /// seed instead of waiting for its old-revision snapshot to retire.
    pub(in super::super) async fn fresh_analysis_for(
        &self,
        uri: &Uri,
        text: Arc<str>,
        dialect: String,
    ) -> DocumentSourceResult<Arc<tcl_compiler::analyser::AnalysisResult>> {
        Ok(self.fresh_analysis_seed(uri, text, dialect).await?.analysis)
    }

    pub(in super::super) async fn fresh_analysis_seed(
        &self,
        uri: &Uri,
        text: Arc<str>,
        dialect: String,
    ) -> DocumentSourceResult<FreshAnalysisSeed> {
        loop {
            let sidecar_revision = self
                .fresh_analysis_cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .revision();
            let path_admission = self.document_path_admission(uri).await?;
            let mut driver = self.fresh_analysis_driver(path_admission).await;
            let resolved_input = driver.analyser.prepare_analysis_input(&dialect).ok();
            if let Some(input) = &resolved_input {
                driver.analyser = driver.analyser.with_resolved_input(input.clone());
            }
            let key = fresh_analysis_cache::FreshAnalysisKey {
                text: Arc::clone(&text),
                dialect: dialect.clone(),
                path_admission: Arc::clone(&driver.path_admission),
                analyser_inputs_epoch: driver.analyser_inputs_epoch,
                class_factory_generation: driver.class_factory_generation,
                inputs: driver.inputs.clone(),
                resolved_input,
            };
            let cached = key.resolved_input.as_ref().and_then(|_| {
                self.fresh_analysis_cache
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(uri, &key)
            });
            let reused = cached.is_some();
            let seed = if let Some(mut seed) = cached {
                seed.path_admission = Arc::clone(&key.path_admission);
                seed.path_admission.check_current(self.store.as_ref())?;
                seed
            } else {
                let original_uri = uri.clone();
                let source = Arc::clone(&text);
                let dialect = dialect.clone();
                if let Some(cooperation) = &self.analysis_cooperation {
                    let ticket = cooperation.ticket();
                    let control = cooperation.control();
                    let attempt = crate::rt::spawn_blocking(move || {
                        driver.analyse_controlled(&source, &dialect, control)
                    })
                    .await
                    .map_err(|_| DocumentSourceUnavailable::Worker { uri: original_uri })?;
                    if let Ok(seed) = attempt {
                        seed?
                    } else {
                        if cooperation.wait_after(ticket).await
                            == analysis_cooperation::TransportProgress::Stopped
                        {
                            // The driver owns terminal completion. No warning,
                            // empty analysis or cache/index result is published.
                            return std::future::pending().await;
                        }
                        continue;
                    }
                } else {
                    crate::rt::spawn_blocking(move || driver.analyse(&source, &dialect, None))
                        .await
                        .map_err(|_| DocumentSourceUnavailable::Worker { uri: original_uri })??
                }
            };
            seed.path_admission.check_for(uri, self.store.as_ref())?;
            if !self.fresh_index_generation_is_current(&seed).await {
                continue;
            }
            seed.path_admission.check_for(uri, self.store.as_ref())?;
            let mut cache = self
                .fresh_analysis_cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if !cache.is_current(&sidecar_revision) {
                continue;
            }
            if !reused && key.resolved_input.is_some() {
                cache.insert(uri, key, seed.clone());
            }
            return Ok(seed);
        }
    }

    pub(in super::super) async fn fresh_analysis_driver(
        &self,
        path_admission: Arc<DocumentPathAdmission>,
    ) -> Box<FreshAnalysisDriver> {
        let uri = path_admission.uri();
        // Analyse outside Salsa with the same per-folder config the cached path
        // would have used, so the result still honours folder-scoped
        // suppression and cross-file factory oracle without retaining a
        // database snapshot. Resolve the independent config stores under one
        // stable epoch: a pack/config apply can otherwise split this snapshot
        // across two generations before the analysis even starts.
        let (
            analyser_inputs_epoch,
            class_factory_generation,
            workspace_class_factories,
            disabled,
            na_mode,
            extra,
            pack_key,
            resource,
        ) = loop {
            let analyser_inputs_epoch = self.diag_inputs_epoch();
            let config = self.resolved_db_config(uri).await;
            let (class_factory_generation, workspace_class_factories) =
                self.published_class_factories_for_fresh_analysis().await;
            let (disabled, na_mode, extra, pack_key, resource) = {
                let db = self.db.lock().await;
                (
                    config
                        .disabled_diagnostics(&*db)
                        .iter()
                        .cloned()
                        .collect::<HashSet<String>>(),
                    config.non_ascii_mode(&*db),
                    config
                        .extra_commands(&*db)
                        .iter()
                        .cloned()
                        .collect::<HashSet<String>>(),
                    config.spec_pack_key(&*db),
                    ResourceAnalyserInputs::from_db_config(config, &*db),
                )
            };
            if self.diag_inputs_epoch() == analyser_inputs_epoch {
                break (
                    analyser_inputs_epoch,
                    class_factory_generation,
                    workspace_class_factories,
                    disabled,
                    na_mode,
                    extra,
                    pack_key,
                    resource,
                );
            }
        };
        let file_path = path_admission.analyser_path().map(str::to_owned);
        let (workspace_declared_commands, workspace_source_modules) = {
            let db = self.db.lock().await;
            let db_files = self.db_files.lock().await;
            db_files.get(uri).map_or((None, None), |file| {
                (
                    file.workspace_declared_commands(&*db).clone(),
                    file.workspace_source_modules(&*db).clone(),
                )
            })
        };
        let inputs = fresh_analysis_cache::FreshAnalyserInputs {
            disabled: disabled.clone(),
            non_ascii_mode: na_mode,
            extra_commands: extra.clone(),
            pack_key,
            resource: resource.clone(),
            workspace_declared_commands: workspace_declared_commands.clone(),
            workspace_source_modules: workspace_source_modules.clone(),
        };
        let driver = FreshAnalysisDriver {
            path_admission,
            source_store: Arc::clone(&self.store),
            analyser: Self::configured_analyser(disabled, na_mode, extra, pack_key, resource)
                .with_workspace_class_factories(workspace_class_factories)
                .with_workspace_declared_commands(workspace_declared_commands)
                .with_workspace_source_modules(workspace_source_modules)
                .with_file_path(file_path),
            analyser_inputs_epoch,
            class_factory_generation,
            inputs,
        };
        Box::new(driver)
    }

    pub(in super::super) async fn fresh_index_generation_is_current(
        &self,
        seed: &FreshAnalysisSeed,
    ) -> bool {
        let generation = self.class_factory_generation.read().await;
        self.diag_inputs_epoch() == seed.analyser_inputs_epoch
            && *generation == seed.class_factory_generation
    }

    #[cfg(test)]
    pub(in super::super) async fn fresh_index_seed_is_current(
        &self,
        source: &str,
        seed: &FreshAnalysisSeed,
    ) -> bool {
        self.fresh_index_generation_is_current(seed).await
            && tcl_compiler::source_graph::current_analysis(source, &seed.analysis).is_some()
    }
}
