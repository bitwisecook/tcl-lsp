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

//! The final transactions that publish a readonly answer only while its captured sources are current: documents, factory sources and library analyses.

use super::super::{Backend, DocumentPathAdmission, DocumentSourceResult};
#[cfg(test)]
use super::UNRELATED_WRITES_BEFORE_FINAL;
use super::{
    OriginalReadonlyReferenceDocument, ReadonlyPublicationInputs, ReadonlySavedSources,
    ReadonlySourceDocument, ReadonlySourceState, ReadonlyTransaction,
    WorkspaceSymbolSourceAnalysis, retry_readonly_attempt,
};
use std::ops::ControlFlow;
use tower_lsp_server::ls_types::Uri;

impl Backend {
    pub(in super::super) async fn publish_factory_source_if_current<T>(
        &self,
        documents: &[ReadonlySourceDocument],
        inputs_epoch: u64,
        factory_generation: u64,
        revision: salsa::Revision,
        caller: (&Uri, &super::super::DocumentState, u64),
        value: T,
    ) -> DocumentSourceResult<Option<T>> {
        let (caller_uri, caller, caller_generation) = caller;
        let selected = documents
            .iter()
            .find(|document| &document.uri == caller_uri);
        let Some(selected) = selected else {
            return Ok(None);
        };
        let selected_caller = &selected.publication.document;
        if selected_caller.text != caller.text
            || selected_caller.raw() != caller.raw()
            || selected_caller.dialect != caller.dialect
            || selected_caller.revision != caller.revision
            || selected_caller.path_admission != caller.path_admission
            || selected_caller.saved_source_bytes.is_some() != caller.saved_source_bytes.is_some()
            || selected.publication.open_revision.is_none() != caller.saved_source_bytes.is_some()
        {
            return Ok(None);
        }
        let mut admissions = Vec::new();
        let mut saved_documents = Vec::new();
        if let Some(admission) = caller.path_admission.as_deref() {
            if admission.uri() != caller_uri {
                return Ok(None);
            }
            admissions.push(admission);
        }
        if caller.saved_source_bytes.is_some() {
            if caller.path_admission.is_none() {
                return Ok(None);
            }
            saved_documents.push(caller);
        }
        for document in documents {
            admissions.push(document.admission.as_ref());
            let source = &document.publication.document;
            if let Some(admission) = source.path_admission.as_deref() {
                if admission.uri() != &document.uri {
                    return Ok(None);
                }
                admissions.push(admission);
            }
            if document.publication.open_revision.is_none() {
                if source.path_admission.is_none() || source.saved_source_bytes.is_none() {
                    return Ok(None);
                }
                saved_documents.push(source);
            }
        }
        let saved_sources = super::super::DocumentState::saved_source_inputs(saved_documents);
        let mut value = Some(value);
        self.readonly_source_transaction(
            inputs_epoch,
            factory_generation,
            &admissions,
            (!saved_sources.is_empty()).then_some(saved_sources.as_slice()),
            |state| {
                (tcl_lsp_db::database_revision(state.db) == revision
                    && state
                        .uri_generations
                        .get(caller_uri)
                        .copied()
                        .unwrap_or_default()
                        == caller_generation
                    && state.open.get(caller_uri).is_none_or(|current| {
                        current.revision == caller.revision
                            && current.raw() == caller.raw()
                            && current.dialect == caller.dialect
                    })
                    && documents.iter().all(|document| {
                        document.matches_current_snapshot(
                            state.uri_generations,
                            state.open,
                            state.db,
                            state.files,
                            state.published,
                        )
                    }))
                .then_some(())
            },
            |_, ()| value.take(),
        )
        .await
        .map(ReadonlyTransaction::published)
    }

    pub(super) async fn readonly_documents_transaction<T>(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        inputs_epoch: u64,
        factory_generation: u64,
        mut publish: impl FnMut() -> Option<T>,
    ) -> DocumentSourceResult<ReadonlyTransaction<T>> {
        let admissions = documents
            .iter()
            .map(|document| document.admission.as_ref())
            .collect::<Vec<_>>();
        self.readonly_source_transaction(
            inputs_epoch,
            factory_generation,
            &admissions,
            None,
            |state| {
                documents
                    .iter()
                    .all(|document| {
                        document.matches_current_snapshot(
                            state.uri_generations,
                            state.open,
                            state.db,
                            state.files,
                            state.published,
                        )
                    })
                    .then_some(())
            },
            |_, ()| publish(),
        )
        .await
    }

    pub(super) async fn with_current_readonly_source_state<T, Checked>(
        &self,
        inputs_epoch: u64,
        factory_generation: u64,
        admissions: &[&DocumentPathAdmission],
        saved_sources: Option<&[(&DocumentPathAdmission, &[u8])]>,
        check: impl FnMut(&ReadonlySourceState<'_>) -> Option<Checked>,
        publish: impl FnMut(&ReadonlySourceState<'_>, &Checked) -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        self.readonly_source_transaction(
            inputs_epoch,
            factory_generation,
            admissions,
            saved_sources,
            check,
            publish,
        )
        .await
        .map(ReadonlyTransaction::published)
    }

    /// Rewrite one retained file's cross-file evidence with its own value, as
    /// the diagnostics worker does: the revision moves, captured values do not.
    #[cfg(test)]
    async fn write_unrelated_input_before_final(&self) {
        use salsa::Setter as _;
        if UNRELATED_WRITES_BEFORE_FINAL.get() == 0 {
            return;
        }
        UNRELATED_WRITES_BEFORE_FINAL.set(UNRELATED_WRITES_BEFORE_FINAL.get() - 1);
        let mut db = self.db.lock().await;
        let files = self.db_files.lock().await;
        let file = *files.values().next().expect("a retained source file");
        let evidence = file.external_call_sites(&*db).clone();
        file.set_external_call_sites(&mut *db).to(evidence);
    }

    pub(super) async fn readonly_source_transaction<T, Checked>(
        &self,
        inputs_epoch: u64,
        factory_generation: u64,
        admissions: &[&DocumentPathAdmission],
        saved_sources: Option<&[(&DocumentPathAdmission, &[u8])]>,
        check: impl FnMut(&ReadonlySourceState<'_>) -> Option<Checked>,
        publish: impl FnMut(&ReadonlySourceState<'_>, &Checked) -> Option<T>,
    ) -> DocumentSourceResult<ReadonlyTransaction<T>> {
        self.readonly_source_transaction_diagnostic(
            (inputs_epoch, factory_generation),
            admissions,
            move |_| saved_sources.map(std::borrow::Cow::Borrowed),
            &super::super::WorkspaceSymbolDiagnostic::default(),
            check,
            publish,
        )
        .await
    }

    async fn readonly_source_transaction_diagnostic<'saved, T, Checked>(
        &self,
        epochs: (u64, u64),
        admissions: &'saved [&'saved DocumentPathAdmission],
        mut saved_sources: impl FnMut(&Checked) -> Option<ReadonlySavedSources<'saved>>,
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
        check: impl FnMut(&ReadonlySourceState<'_>) -> Option<Checked>,
        publish: impl FnMut(&ReadonlySourceState<'_>, &Checked) -> Option<T>,
    ) -> DocumentSourceResult<ReadonlyTransaction<T>> {
        self.readonly_source_transaction_with_inputs(
            epochs,
            |checked| ReadonlyPublicationInputs {
                admissions: std::borrow::Cow::Borrowed(admissions),
                saved_sources: saved_sources(checked),
            },
            diagnostic,
            check,
            publish,
        )
        .await
    }

    pub(super) async fn readonly_source_transaction_with_inputs<'saved, T, Checked>(
        &self,
        epochs: (u64, u64),
        mut select_inputs: impl FnMut(&Checked) -> ReadonlyPublicationInputs<'saved>,
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
        mut check: impl FnMut(&ReadonlySourceState<'_>) -> Option<Checked>,
        mut publish: impl FnMut(&ReadonlySourceState<'_>, &Checked) -> Option<T>,
    ) -> DocumentSourceResult<ReadonlyTransaction<T>> {
        let (inputs_epoch, factory_generation) = epochs;
        #[cfg(test)]
        self.write_unrelated_input_before_final().await;
        let _inputs = self.analyser_inputs_gate.read().await;
        let generation = self.class_factory_generation.read().await;
        let open = self
            .documents
            .lock("readonly_reference_final_snapshot")
            .await;
        let Ok(db) = self.db.try_lock() else {
            diagnostic.record("lease.db_busy", &[]);
            return Ok(ReadonlyTransaction::CurrencyRefused);
        };
        let Ok(files) = self.db_files.try_lock() else {
            diagnostic.record("lease.files_busy", &[]);
            return Ok(ReadonlyTransaction::CurrencyRefused);
        };
        let published = self
            .source_path_admissions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let uri_generations = self
            .live_publication_uri_generations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !(self.diag_inputs_epoch() == inputs_epoch && *generation == factory_generation) {
            diagnostic.record("lease.epoch_or_factory", &[]);
            return Ok(ReadonlyTransaction::CurrencyRefused);
        }
        let state = ReadonlySourceState {
            open: &open,
            db: &db,
            files: &files,
            published: &published,
            uri_generations: &uri_generations,
        };
        let Some(checked) = check(&state) else {
            diagnostic.record("lease.source_or_index_state", &[]);
            return Ok(ReadonlyTransaction::CurrencyRefused);
        };
        let selected = select_inputs(&checked);
        let admissions = selected.admissions.as_ref();
        let issued = if let Some(sources) = selected.saved_sources.as_deref() {
            DocumentPathAdmission::with_current_sources(
                admissions,
                sources,
                self.store.as_ref(),
                || publish(&state, &checked),
            )
        } else {
            DocumentPathAdmission::with_current(admissions, self.store.as_ref(), || {
                publish(&state, &checked)
            })
        }?;
        Ok(issued.map_or(
            ReadonlyTransaction::PublishRefused,
            ReadonlyTransaction::Published,
        ))
    }

    #[cfg(test)]
    pub(in super::super) async fn publish_checked_library_analyses(
        &self,
        analysed: &[(
            Uri,
            super::super::DocumentState,
            WorkspaceSymbolSourceAnalysis,
        )],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<bool> {
        self.library_analyses_transaction(analysed, inputs_epoch, factory_generation)
            .await
            .map(|transaction| transaction.published().is_some())
    }

    /// Capture each library file and publish the whole set, recapturing only
    /// after a currency refusal. `None` withholds every captured analysis.
    pub(in super::super) async fn capture_and_publish_library_files(
        &self,
        files: &[std::path::PathBuf],
    ) -> DocumentSourceResult<
        Option<
            Vec<(
                Uri,
                super::super::DocumentState,
                WorkspaceSymbolSourceAnalysis,
            )>,
        >,
    > {
        retry_readonly_attempt(None, move || {
            self.capture_and_publish_library_attempt(files)
        })
        .await
    }

    async fn capture_and_publish_library_attempt(
        &self,
        files: &[std::path::PathBuf],
    ) -> DocumentSourceResult<
        ControlFlow<
            Option<
                Vec<(
                    Uri,
                    super::super::DocumentState,
                    WorkspaceSymbolSourceAnalysis,
                )>,
            >,
        >,
    > {
        let inputs_epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        let mut analysed = Vec::new();
        for path in files {
            let Some(target_uri) = super::super::canonical_file_uri(path) else {
                continue;
            };
            let Some(snapshot) = self.read_document(&target_uri).await? else {
                continue;
            };
            let Some(receipt) = self
                .capture_workspace_symbol_analysis(&target_uri, &snapshot)
                .await?
            else {
                continue;
            };
            analysed.push((target_uri, snapshot, receipt));
        }
        if analysed.is_empty() {
            return Ok(ControlFlow::Break(Some(analysed)));
        }
        Ok(self
            .library_analyses_transaction(&analysed, inputs_epoch, factory_generation)
            .await?
            .into_attempt()
            .map_break(|published| published.map(|()| analysed)))
    }

    async fn library_analyses_transaction(
        &self,
        analysed: &[(
            Uri,
            super::super::DocumentState,
            WorkspaceSymbolSourceAnalysis,
        )],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<ReadonlyTransaction<()>> {
        let mut saved = Vec::new();
        let mut admissions = Vec::new();
        for (uri, snapshot, analysis) in analysed {
            if snapshot.path_admission.is_none()
                && self.store.workspace_source_inventory_provider()
                    == super::super::vfs::WorkspaceSourceInventoryProvider::HostReported
                && uri.scheme().as_str() == "file"
            {
                return Ok(ReadonlyTransaction::PublishRefused);
            }
            for admission in
                std::iter::once(analysis.admission()).chain(snapshot.path_admission.as_deref())
            {
                if admission.uri() != uri {
                    return Ok(ReadonlyTransaction::PublishRefused);
                }
                admissions.push(admission);
                if let Some(path) = admission.path() {
                    admission.check_current(self.store.as_ref())?;
                    let Ok(bytes) = self.store.read(path) else {
                        return Ok(ReadonlyTransaction::PublishRefused);
                    };
                    let (text, _) = tcl_lsp_core::source_decode::decode_source_as(
                        &bytes,
                        self.declared_script_encoding(),
                    );
                    if tcl_lexer::normalise_lone_cr(&text).as_ref() != snapshot.text.as_ref() {
                        return Ok(ReadonlyTransaction::PublishRefused);
                    }
                    saved.push((admission, bytes));
                }
            }
        }
        let sources = saved
            .iter()
            .map(|(admission, bytes)| (*admission, bytes.as_slice()))
            .collect::<Vec<_>>();
        self.readonly_source_transaction(
            inputs_epoch,
            factory_generation,
            &admissions,
            Some(&sources),
            |state| {
                analysed
                    .iter()
                    .all(|(uri, snapshot, analysis)| {
                        analysis.matches_snapshot(uri, snapshot)
                            && analysis.matches_current_state(
                                state,
                                inputs_epoch,
                                factory_generation,
                            )
                    })
                    .then_some(())
            },
            |_, ()| {
                let _rehoming = self.rehoming_gate.try_lock().ok()?;
                let mut index = self.workspace_index.try_write().ok()?;
                let mut seeds = self.rehomed_source_seeds.try_lock().ok()?;
                let mut autoloaded = self.autoloaded_library_uris.try_lock().ok()?;
                for (uri, _, analysis) in analysed {
                    index.replace_document(uri.as_str(), analysis.analysis());
                    seeds.remove(uri.as_str());
                    autoloaded.insert(uri.as_str().to_owned());
                }
                Some(())
            },
        )
        .await
    }
}
