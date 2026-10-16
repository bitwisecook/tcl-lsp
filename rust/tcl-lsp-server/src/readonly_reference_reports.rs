// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reporting-only source reference locations and their independent coverage.

use super::{
    Backend, DocumentPathAdmission, DocumentSourceResult, LibraryPublicationSource, with_pack_hooks,
};
use std::sync::Arc;
use tcl_compiler::analyser::{AnalysisResult, ResolvedAnalysisInput};
use tcl_lsp_core::original_declaration::{OriginalDeclarationIdentity, ReadonlyReferenceCoverage};
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};

struct OriginalReadonlyReferenceDocument {
    uri: Uri,
    source: Arc<str>,
    analysis: Arc<AnalysisResult>,
    file: tcl_lsp_db::SourceFile,
    config: tcl_lsp_db::AnalyserConfig,
    input: Arc<ResolvedAnalysisInput>,
    admission: Arc<DocumentPathAdmission>,
    publication: LibraryPublicationSource,
    database_revision: salsa::Revision,
}

pub(super) struct OriginalReadonlyReferenceReport {
    pub(super) locations: Vec<Location>,
    pub(super) coverage: ReadonlyReferenceCoverage,
}

pub(super) const fn coverage_tag(coverage: ReadonlyReferenceCoverage) -> &'static str {
    match coverage {
        ReadonlyReferenceCoverage::ExistingInventory => "existing-inventory",
        ReadonlyReferenceCoverage::Partial => "partial",
        ReadonlyReferenceCoverage::Unavailable => "unavailable",
    }
}

impl OriginalReadonlyReferenceReport {
    pub(super) fn unavailable() -> Self {
        Self {
            locations: Vec::new(),
            coverage: ReadonlyReferenceCoverage::Unavailable,
        }
    }

    pub(super) fn lens_command(
        self,
        uri: Uri,
        position: Position,
    ) -> tower_lsp_server::ls_types::Command {
        let title = tcl_lsp_core::code_lens::readonly_reference_count_title(
            self.locations.len(),
            self.coverage,
        );
        let clickable = self.coverage == ReadonlyReferenceCoverage::ExistingInventory
            || self.coverage == ReadonlyReferenceCoverage::Partial && !self.locations.is_empty();
        tower_lsp_server::ls_types::Command {
            title,
            command: if clickable {
                "tcl-lsp.showReferences".to_owned()
            } else {
                String::new()
            },
            arguments: clickable.then(|| {
                vec![
                    serde_json::Value::String(uri.to_string()),
                    serde_json::to_value(position).unwrap_or(serde_json::Value::Null),
                    serde_json::to_value(self.locations).unwrap_or(serde_json::Value::Null),
                ]
            }),
        }
    }
}

impl Backend {
    async fn capture_readonly_reference_document(
        &self,
        owner: &str,
        source: Arc<str>,
    ) -> DocumentSourceResult<Option<OriginalReadonlyReferenceDocument>> {
        let Ok(uri) = owner.parse::<Uri>() else {
            return Ok(None);
        };
        let Some(document) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        if document.text != source {
            return Ok(None);
        }
        let config = self.resolved_db_config(&uri).await;
        let Some(publication) = self.library_publication_source(&uri, document).await else {
            return Ok(None);
        };
        let (file, input, admission, database_revision) = {
            let db = self.db.lock().await;
            let files = self.db_files.lock().await;
            let Some(&file) = files.get(&uri) else {
                return Ok(None);
            };
            let published = self
                .source_path_admissions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(retained) = published.get(&uri) else {
                return Ok(None);
            };
            if retained.file != file
                || file.path(&*db).as_deref() != retained.admission.analyser_path()
                || file.text(&*db).as_str() != source.as_ref()
                || file.dialect(&*db) != &publication.document.dialect
                || publication
                    .document
                    .path_admission
                    .as_deref()
                    .is_some_and(|admission| admission != retained.admission.as_ref())
            {
                return Ok(None);
            }
            let Ok(input) =
                with_pack_hooks(|| tcl_lsp_db::document_analysis_input(&*db, file, config))
            else {
                return Ok(None);
            };
            (
                file,
                input,
                retained.admission.clone(),
                tcl_lsp_db::database_revision(&db),
            )
        };
        // Reissue this report's metadata from the genuine retained provider;
        // a later equal-path publication cannot relabel an old analysis.
        let mut driver = self.fresh_analysis_driver(admission.clone()).await;
        driver.analyser = driver.analyser.with_resolved_input(input.as_ref().clone());
        let producer_source = source.clone();
        let dialect = publication.document.dialect.clone();
        let original_uri = uri.clone();
        let seed =
            crate::rt::spawn_blocking(move || driver.analyse(&producer_source, &dialect, None))
                .await
                .map_err(|_| super::DocumentSourceUnavailable::Worker { uri: original_uri })??;
        if seed.path_admission.as_ref() != admission.as_ref() {
            return Ok(None);
        }
        let current = OriginalReadonlyReferenceDocument {
            uri,
            source,
            analysis: seed.analysis,
            file,
            config,
            input,
            admission,
            publication,
            database_revision,
        };
        Ok(current.matches_original_input().then_some(current))
    }

    async fn finish_readonly_reference_report(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        targets: Vec<(String, tcl_lexer::Span)>,
        coverage: ReadonlyReferenceCoverage,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<Option<OriginalReadonlyReferenceReport>> {
        let _inputs = self.analyser_inputs_gate.read().await;
        let generation = self.class_factory_generation.read().await;
        let open = self
            .documents
            .lock("readonly_reference_final_snapshot")
            .await;
        let Ok(db) = self.db.try_lock() else {
            return Ok(None);
        };
        let Ok(files) = self.db_files.try_lock() else {
            return Ok(None);
        };
        let published = self
            .source_path_admissions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let uri_generations = self
            .live_publication_uri_generations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.diag_inputs_epoch() != inputs_epoch
            || *generation != factory_generation
            || documents.iter().any(|document| {
                !document.matches_current_snapshot(&uri_generations, &open, &db, &files, &published)
            })
        {
            return Ok(None);
        }
        let admissions = documents
            .iter()
            .map(|document| document.admission.as_ref())
            .collect::<Vec<_>>();
        DocumentPathAdmission::with_current(&admissions, self.store.as_ref(), || {
            lift_readonly_reference_targets(&targets, documents).map(|locations| {
                OriginalReadonlyReferenceReport {
                    locations,
                    coverage,
                }
            })
        })
    }

    /// Source-only occurrences join the original declaration in its real URI.
    /// All participants retain their actual SourceFile and full checked input;
    /// this report cannot be consumed as call edges or editable occurrences.
    pub(super) async fn readonly_original_declaration_reference_report(
        &self,
        identity: &OriginalDeclarationIdentity,
        selected: &Uri,
        include_declaration: bool,
    ) -> DocumentSourceResult<Option<OriginalReadonlyReferenceReport>> {
        let inputs_epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        let documents = self.original_declaration_documents(selected).await?;
        let mut retained = Vec::with_capacity(documents.len());
        for (owner, source, _) in documents {
            let Some(document) = self
                .capture_readonly_reference_document(&owner, source)
                .await?
            else {
                return Ok(None);
            };
            retained.push(document);
        }
        let Some(owner) = retained
            .iter()
            .find(|document| document.uri.as_str() == identity.uri())
        else {
            return Ok(None);
        };
        if !identity.is_current(identity.uri(), &owner.source, &owner.analysis) {
            return Ok(None);
        }
        if !original_declaration_owner_is_unique(identity, &retained) {
            return Ok(None);
        }
        let Some(report) = tcl_lsp_core::original_declaration::readonly_reference_report(
            identity,
            &owner.source,
            &owner.analysis,
            true,
        ) else {
            return Ok(None);
        };
        let inventory = retained
            .iter()
            .map(
                |document| tcl_lsp_core::original_call_hierarchy::OriginalCallDocument {
                    uri: document.uri.as_str(),
                    source: &document.source,
                    analysis: &document.analysis,
                },
            )
            .collect::<Vec<_>>();
        let mut targets = {
            let workspace = self.workspace_index.read().await;
            tcl_lsp_core::original_call_hierarchy::references(
                identity,
                &inventory,
                Some(&workspace),
            )
        };
        targets.extend(
            report
                .spans()
                .iter()
                .map(|span| (identity.uri().to_owned(), *span)),
        );
        if include_declaration {
            targets.push((identity.uri().to_owned(), identity.span()));
        }
        super::dedup_span_targets(&mut targets);
        self.finish_readonly_reference_report(
            &retained,
            targets,
            report.coverage(),
            inputs_epoch,
            factory_generation,
        )
        .await
    }

    pub(super) async fn readonly_original_method_reference_locations_at(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        position: Position,
        include_declaration: bool,
    ) -> DocumentSourceResult<std::ops::ControlFlow<Vec<Location>>> {
        use std::ops::ControlFlow;
        let target = match self
            .original_method_candidate_at(uri, source, analysis, position)
            .await?
        {
            ControlFlow::Continue(()) => return Ok(ControlFlow::Continue(())),
            ControlFlow::Break(None) => return Ok(ControlFlow::Break(Vec::new())),
            ControlFlow::Break(Some(target)) => target,
        };
        let Some(owner) = target.uri().parse::<Uri>().ok() else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let Some(document) = self.read_document(&owner).await? else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let current = self
            .analysis_for(&owner, document.text.clone(), document.dialect)
            .await?;
        let Some(identity) = OriginalDeclarationIdentity::for_member_candidate(
            owner.as_str(),
            &document.text,
            &current,
            &target,
        ) else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        Ok(ControlFlow::Break(
            self.readonly_original_declaration_reference_report(
                &identity,
                uri,
                include_declaration,
            )
            .await?
            .map_or_else(Vec::new, |report| report.locations),
        ))
    }
}

fn lift_readonly_reference_targets(
    targets: &[(String, tcl_lexer::Span)],
    documents: &[OriginalReadonlyReferenceDocument],
) -> Option<Vec<Location>> {
    targets
        .iter()
        .map(|(uri, span)| {
            let document = documents
                .iter()
                .find(|document| document.uri.as_str() == uri.as_str())?;
            if span.start() > span.end() || span.end() as usize > document.source.len() {
                return None;
            }
            let index = tcl_lexer::LineIndex::new_lsp(&document.source);
            let start = index.position_at_utf16(span.start(), &document.source);
            let end = index.position_at_utf16(span.end(), &document.source);
            Some(Location {
                uri: document.uri.clone(),
                range: Range {
                    start: Position::new(start.line, start.character.get()),
                    end: Position::new(end.line, end.character.get()),
                },
            })
        })
        .collect()
}

fn original_declaration_owner_is_unique(
    identity: &OriginalDeclarationIdentity,
    documents: &[OriginalReadonlyReferenceDocument],
) -> bool {
    documents
        .iter()
        .filter(|document| {
            tcl_lsp_core::original_declaration::declarations(
                document.uri.as_str(),
                &document.source,
                &document.analysis,
            )
            .is_some_and(|rows| {
                rows.iter().any(|row| {
                    row.role() == identity.role()
                        && row.site() == identity.site()
                        && row.image() == identity.image()
                        && row.config() == identity.config()
                })
            })
        })
        .count()
        == 1
}

impl OriginalReadonlyReferenceDocument {
    fn matches_current_snapshot(
        &self,
        uri_generations: &std::collections::HashMap<Uri, u64>,
        open: &std::collections::HashMap<Uri, super::DocumentState>,
        db: &tcl_lsp_db::TclDatabase,
        files: &std::collections::HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &std::collections::HashMap<Uri, super::PublishedSourcePath>,
    ) -> bool {
        self.matches_original_input()
            && self.publication.matches_open_documents(open)
            && uri_generations.get(&self.uri).copied().unwrap_or_default()
                == self.publication.uri_generation
            && tcl_lsp_db::database_revision(db) == self.database_revision
            && files.get(&self.uri) == Some(&self.file)
            && published.get(&self.uri).is_some_and(|retained| {
                retained.file == self.file && retained.admission.as_ref() == self.admission.as_ref()
            })
            && self.file.path(db).as_deref() == self.admission.analyser_path()
            && self.file.text(db).as_str() == self.source.as_ref()
            && self.file.dialect(db) == &self.publication.document.dialect
            && with_pack_hooks(|| tcl_lsp_db::document_analysis_input(db, self.file, self.config))
                .is_ok_and(|input| input.as_ref() == self.input.as_ref())
    }

    fn matches_original_input(&self) -> bool {
        let config = self.input.lexer_config();
        self.analysis.resolved_input.as_ref() == Some(self.input.as_ref())
            && self.analysis.analysis_context_unavailable.is_none()
            && self.analysis.body_lexer_config == Some(config)
            && self.analysis.matches_original_source_image(
                &tcl_lexer::SourceImage::document(&self.source),
                config,
            )
            && self
                .analysis
                .retained_command_realm()
                .is_some_and(|realm| realm.matches_resolved_analysis_input(self.input.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentState;
    use tower_lsp_server::LanguageServer;
    use tower_lsp_server::ls_types::{CodeLensParams, TextDocumentIdentifier};

    const SOURCE: &str =
        "oo::class create C {method ping {} {}; method body {} {my ping; my ping}}";

    async fn register_current(backend: &Backend, uri: &Uri, source: &str) {
        backend
            .documents
            .lock("readonly reference test")
            .await
            .insert(
                uri.clone(),
                DocumentState::new(source.to_owned(), "tcl8.6".to_owned()),
            );
        backend
            .db_set_source(uri, source, "tcl8.6".to_owned())
            .await
            .unwrap();
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

    async fn ping_identity(backend: &Backend, uri: &Uri) -> OriginalDeclarationIdentity {
        let document = backend.read_document(uri).await.unwrap().unwrap();
        let analysis = backend
            .analysis_for(uri, document.text.clone(), document.dialect)
            .await
            .unwrap();
        let class = analysis.original_class_declarations().next().unwrap();
        let method = class
            .metadata()
            .original_members
            .declarations()
            .find(|method| method.original_name_input().bytes() == b"ping")
            .unwrap();
        OriginalDeclarationIdentity::for_method(
            uri.as_str(),
            &document.text,
            &analysis,
            class,
            method,
        )
        .unwrap()
    }

    async fn snapshot_still_current(
        backend: &Backend,
        retained: &OriginalReadonlyReferenceDocument,
    ) -> bool {
        let generation = *backend.class_factory_generation.read().await;
        backend
            .finish_readonly_reference_report(
                std::slice::from_ref(retained),
                Vec::new(),
                ReadonlyReferenceCoverage::Partial,
                backend.diag_inputs_epoch(),
                generation,
            )
            .await
            .unwrap()
            .is_some()
    }

    #[tokio::test]
    async fn readonly_member_wire_retains_owner_and_partial_coverage() {
        // naming.server.original-reference-and-lens-source-selection
        // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///readonly-member-wire.tcl".parse().unwrap();
        register_current(&backend, &uri, SOURCE).await;
        let identity = ping_identity(&backend, &uri).await;
        let report = backend
            .readonly_original_declaration_reference_report(&identity, &uri, false)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(report.coverage, ReadonlyReferenceCoverage::Partial);
        assert_eq!(report.locations.len(), 2);
        assert!(report.locations.iter().all(|location| location.uri == uri));
        assert!(
            backend
                .original_declaration_reference_locations(&identity, &uri, false)
                .await
                .unwrap()
                .is_empty()
        );
        let lenses = backend
            .code_lens(CodeLensParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            })
            .await
            .unwrap()
            .unwrap();
        let mut lens = lenses
            .into_iter()
            .find(|lens| {
                lens.range.start.character == u32::try_from(SOURCE.find("ping").unwrap()).unwrap()
            })
            .unwrap();
        assert_eq!(
            lens.data.as_ref().unwrap()["sourceReferenceCoverage"],
            "partial"
        );
        let data = lens.data.as_mut().unwrap();
        data["sourceReferenceCoverage"] = serde_json::json!("existing-inventory");
        data["qname"] = serde_json::json!("::unrelated");
        let lens = backend.code_lens_resolve(lens).await.unwrap();
        assert_eq!(
            lens.data.as_ref().unwrap()["sourceReferenceCoverage"],
            "partial"
        );
        let command = lens.command.unwrap();
        assert_eq!(command.title, "at least 2 references");
        assert_eq!(command.command, "tcl-lsp.showReferences");
        let locations: Vec<Location> =
            serde_json::from_value(command.arguments.unwrap()[2].clone()).unwrap();
        assert_eq!(locations, report.locations);
    }

    #[tokio::test]
    async fn readonly_member_wire_withdraws_missing_or_replaced_source_owners() {
        // naming.server.original-reference-and-lens-source-selection
        // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///readonly-member-current.tcl".parse().unwrap();
        let foreign: Uri = "file:///readonly-member-foreign.tcl".parse().unwrap();
        register_current(&backend, &uri, SOURCE).await;
        let identity = ping_identity(&backend, &uri).await;
        let document = backend.read_document(&uri).await.unwrap().unwrap();
        let retained = backend
            .capture_readonly_reference_document(uri.as_str(), document.text.clone())
            .await
            .unwrap()
            .unwrap();
        assert!(snapshot_still_current(&backend, &retained).await);
        register_current(&backend, &foreign, SOURCE).await;
        let foreign_identity = ping_identity(&backend, &foreign).await;
        assert_ne!(identity, foreign_identity);
        assert!(
            backend
                .readonly_original_declaration_reference_report(&identity, &uri, false)
                .await
                .unwrap()
                .is_none()
        );
        backend.db_files.lock().await.remove(&uri);
        assert!(!snapshot_still_current(&backend, &retained).await);
        assert!(
            backend
                .readonly_original_declaration_reference_report(&identity, &uri, false)
                .await
                .unwrap()
                .is_none()
        );
        register_current(&backend, &uri, "oo::class create C {method changed {} {}}").await;
        assert!(!snapshot_still_current(&backend, &retained).await);
        assert!(
            backend
                .readonly_original_declaration_reference_report(&identity, &uri, false)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn readonly_member_wire_rejects_equal_path_provider_republication() {
        // naming.server.original-reference-and-lens-source-selection
        // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
        let mut backend = crate::tests::test_backend();
        let store = Arc::new(crate::vfs::MemoryStore::new());
        let uri: Uri = "file:///readonly-provider-aba.tcl".parse().unwrap();
        let path = "/same/readonly-provider-aba.tcl";
        store.upsert_uri(uri.as_str(), path, SOURCE.as_bytes().to_vec());
        backend.store = store.clone();
        register_current(&backend, &uri, SOURCE).await;
        let retained = backend
            .capture_readonly_reference_document(uri.as_str(), Arc::from(SOURCE))
            .await
            .unwrap()
            .unwrap();
        assert!(snapshot_still_current(&backend, &retained).await);
        store.upsert_uri(
            uri.as_str(),
            "/other/readonly-provider-aba.tcl",
            SOURCE.as_bytes().to_vec(),
        );
        store.upsert_uri(uri.as_str(), path, SOURCE.as_bytes().to_vec());
        register_current(&backend, &uri, SOURCE).await;
        let current = backend
            .capture_readonly_reference_document(uri.as_str(), Arc::from(SOURCE))
            .await
            .unwrap()
            .unwrap();
        assert!(current.file == retained.file);
        assert_eq!(current.input, retained.input);
        assert_ne!(current.admission.as_ref(), retained.admission.as_ref());
        assert!(!snapshot_still_current(&backend, &retained).await);
        assert!(snapshot_still_current(&backend, &current).await);
        backend.invalidate_live_publication(std::iter::once(&uri));
        assert!(!snapshot_still_current(&backend, &current).await);
        let current = backend
            .capture_readonly_reference_document(uri.as_str(), Arc::from(SOURCE))
            .await
            .unwrap()
            .unwrap();
        assert!(snapshot_still_current(&backend, &current).await);
        backend
            .documents
            .lock("readonly_same_bytes_revision")
            .await
            .get_mut(&uri)
            .unwrap()
            .revision += 1;
        assert!(!snapshot_still_current(&backend, &current).await);
    }

    #[test]
    fn unavailable_member_wire_has_no_clickable_empty_census() {
        // naming.server.original-reference-and-lens-source-selection
        // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
        for coverage in [
            ReadonlyReferenceCoverage::Partial,
            ReadonlyReferenceCoverage::Unavailable,
        ] {
            let report = OriginalReadonlyReferenceReport {
                locations: Vec::new(),
                coverage,
            };
            let command = report.lens_command(
                "file:///readonly-empty.tcl".parse().unwrap(),
                Position::new(0, 0),
            );
            assert_eq!(command.title, "References unavailable");
            assert!(command.command.is_empty());
            assert!(command.arguments.is_none());
        }
        let report = OriginalReadonlyReferenceReport {
            locations: Vec::new(),
            coverage: ReadonlyReferenceCoverage::ExistingInventory,
        };
        let command = report.lens_command(
            "file:///readonly-empty.tcl".parse().unwrap(),
            Position::new(0, 0),
        );
        assert_eq!(command.title, "0 references");
        assert_eq!(command.command, "tcl-lsp.showReferences");
    }
}
