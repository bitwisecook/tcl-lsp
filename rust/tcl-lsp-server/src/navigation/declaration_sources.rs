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

//! Revalidating declaration targets against their current documents and lifting them to locations.

use super::{
    AnalysisResult, Arc, Backend, DocumentPathAdmission, DocumentSourceResult, DocumentState,
    FromStr, HashMap, HashSet, LibraryPublicationSource, Location, OriginalIndexedSourceLocation,
    Path, Position, Range, ReadonlyDeclarationLocation, ReadonlyDeclarationTarget,
    ReadonlySavedSource, Uri, core_definition, lift_lsp_range,
};

impl Backend {
    /// Revalidate indexed facts in each actual current document before using
    /// their spans or counting them. Configuration changes withdraw the batch.
    pub(super) async fn validated_original_indexed_locations(
        &self,
        targets: Vec<OriginalIndexedSourceLocation>,
    ) -> DocumentSourceResult<Vec<(OriginalIndexedSourceLocation, Location)>> {
        let epoch = self.diag_inputs_epoch();
        let mut documents = HashMap::new();
        for target in &targets {
            if documents.contains_key(target.uri()) {
                continue;
            }
            let resolved = if let Ok(uri) = target.uri().parse::<Uri>() {
                let generation = self.live_publication_uri_generation(&uri);
                if let Some(document) = self.read_document(&uri).await? {
                    let analysis = self
                        .analysis_for(&uri, document.text.clone(), document.dialect.clone())
                        .await?;
                    self.class_type_source(&uri, document, generation)
                        .await
                        .map(|source| (source, analysis))
                } else {
                    None
                }
            } else {
                None
            };
            documents.insert(target.uri().to_owned(), resolved);
        }
        let mut saved = Vec::new();
        for (source, _) in documents.values().flatten() {
            if source.open_revision.is_some() {
                continue;
            }
            let Some(admission) = source.document.path_admission.as_ref() else {
                return Ok(Vec::new());
            };
            admission.check_for(&source.uri, self.store.as_ref())?;
            let Some(path) = admission.path().map(Path::to_path_buf) else {
                return Ok(Vec::new());
            };
            let store = Arc::clone(&self.store);
            let Ok(Ok(bytes)) = crate::rt::spawn_blocking(move || store.read(&path)).await else {
                return Ok(Vec::new());
            };
            let (decoded, _) = tcl_lsp_core::source_decode::decode_source_as(
                &bytes,
                self.declared_script_encoding(),
            );
            if tcl_lexer::normalise_lone_cr(&decoded).as_ref() != source.document.text.as_ref() {
                return Ok(Vec::new());
            }
            saved.push((Arc::clone(admission), bytes));
        }
        let _inputs = self.analyser_inputs_gate.read().await;
        let _rehoming = self.rehoming_gate.lock().await;
        let current = self
            .documents
            .lock("original_indexed_location_publish")
            .await;
        if self.diag_inputs_epoch() != epoch
            || documents.values().flatten().any(|(source, _)| {
                !source.matches_open_documents(&current)
                    || self.live_publication_uri_generation(&source.uri) != source.uri_generation
            })
        {
            return Ok(Vec::new());
        }
        for (source, _) in documents.values().flatten() {
            let Some(admission) = source.document.path_admission.as_ref() else {
                return Ok(Vec::new());
            };
            admission.check_for(&source.uri, self.store.as_ref())?;
        }
        let selected = saved
            .iter()
            .map(|(admission, bytes)| (admission.as_ref(), bytes.as_slice()))
            .collect::<Vec<_>>();
        let mut result = Vec::new();
        DocumentPathAdmission::with_current_saved_sources(&selected, self.store.as_ref(), || {
            let mut seen = HashSet::new();
            result = targets
                .iter()
                .filter_map(|target| {
                    let (source, analysis) = documents.get(target.uri())?.as_ref()?;
                    let span = target.validated_span(&source.document.text, analysis)?;
                    seen.insert((source.uri.clone(), span)).then_some(())?;
                    let range = lift_lsp_range(core_definition::span_to_range(
                        &source.document.text,
                        &source.document.line_index,
                        span,
                    ));
                    Some((
                        target.clone(),
                        Location {
                            uri: source.uri.clone(),
                            range,
                        },
                    ))
                })
                .collect();
        })?;
        Ok(result)
    }

    pub(in super::super) async fn resolve_original_indexed_locations(
        &self,
        targets: Vec<OriginalIndexedSourceLocation>,
    ) -> DocumentSourceResult<Vec<Location>> {
        Ok(self
            .validated_original_indexed_locations(targets)
            .await?
            .into_iter()
            .map(|(_, location)| location)
            .collect())
    }

    /// Capture this exact request snapshot without retagging a same-text
    /// dependency revision as the source that produced its analysis.
    pub(in super::super) async fn class_type_source(
        &self,
        uri: &Uri,
        document: DocumentState,
        uri_generation: u64,
    ) -> Option<LibraryPublicationSource> {
        let documents = self.documents.lock("class_type_source_capture").await;
        if self.live_publication_uri_generation(uri) != uri_generation {
            return None;
        }
        if documents
            .get(uri)
            .is_some_and(|current| current.revision != document.revision)
        {
            return None;
        }
        let source = LibraryPublicationSource {
            uri: uri.clone(),
            open_revision: documents.get(uri).map(|current| current.revision),
            uri_generation,
            document,
        };
        source.matches_open_documents(&documents).then_some(source)
    }

    /// Lift one genuine Native declaration against its whole issuer
    /// document. Final source/configuration checks follow every await; saved
    /// sources use the actual provider's byte observation callback.
    pub(in super::super) async fn resolve_current_original_class_type_location(
        &self,
        caller: (Uri, DocumentState, u64),
        target: (Uri, DocumentState, Arc<AnalysisResult>, u64),
        location: OriginalIndexedSourceLocation,
        epoch: u64,
    ) -> DocumentSourceResult<Vec<Location>> {
        let (uri, document, analysis, generation) = target;
        self.resolve_current_declaration_locations(
            caller,
            vec![ReadonlyDeclarationTarget {
                uri,
                document,
                analysis,
                generation,
                location: ReadonlyDeclarationLocation::Original(Box::new(location)),
            }],
            epoch,
        )
        .await
    }

    pub(in super::super) async fn readonly_saved_sources(
        &self,
        sources: &[&LibraryPublicationSource],
    ) -> DocumentSourceResult<Option<Vec<ReadonlySavedSource>>> {
        let mut saved = Vec::new();
        for source in sources {
            if source.open_revision.is_some() {
                continue;
            }
            let Some(admission) = source.document.path_admission.as_ref() else {
                return Ok(None);
            };
            admission.check_for(&source.uri, self.store.as_ref())?;
            let Some(path) = admission.path().map(Path::to_path_buf) else {
                return Ok(None);
            };
            let store = Arc::clone(&self.store);
            let Ok(Ok(bytes)) = crate::rt::spawn_blocking(move || store.read(&path)).await else {
                return Ok(None);
            };
            let (decoded, _) = tcl_lsp_core::source_decode::decode_source_as(
                &bytes,
                self.declared_script_encoding(),
            );
            if tcl_lexer::normalise_lone_cr(&decoded).as_ref() != source.document.text.as_ref() {
                return Ok(None);
            }
            saved.push(ReadonlySavedSource {
                admission: Arc::clone(admission),
                bytes,
            });
        }
        Ok(Some(saved))
    }

    pub(in super::super) async fn resolve_current_declaration_locations(
        &self,
        caller: (Uri, DocumentState, u64),
        targets: Vec<ReadonlyDeclarationTarget>,
        epoch: u64,
    ) -> DocumentSourceResult<Vec<Location>> {
        self.with_current_declaration_sources(caller, targets, epoch, |uri, document, _, span| {
            Some(Location {
                uri: uri.clone(),
                range: lift_lsp_range(core_definition::span_to_range(
                    &document.text,
                    &document.line_index,
                    span,
                )),
            })
        })
        .await
    }

    pub(super) async fn with_current_declaration_sources<T>(
        &self,
        caller: (Uri, DocumentState, u64),
        targets: Vec<ReadonlyDeclarationTarget>,
        epoch: u64,
        render: impl Fn(&Uri, &DocumentState, &AnalysisResult, tcl_lexer::Span) -> Option<T>,
    ) -> DocumentSourceResult<Vec<T>> {
        let Some(caller) = self.class_type_source(&caller.0, caller.1, caller.2).await else {
            return Ok(Vec::new());
        };
        let mut retained = Vec::new();
        for target in targets {
            let Some(source) = self
                .class_type_source(&target.uri, target.document, target.generation)
                .await
            else {
                return Ok(Vec::new());
            };
            retained.push((source, target.analysis, target.location));
        }
        let sources = std::iter::once(&caller)
            .chain(retained.iter().map(|(source, _, _)| source))
            .collect::<Vec<_>>();
        let Some(saved) = self.readonly_saved_sources(&sources).await? else {
            return Ok(Vec::new());
        };
        let _inputs = self.analyser_inputs_gate.read().await;
        let _rehoming = self.rehoming_gate.lock().await;
        let documents = self.documents.lock("class_type_location_publish").await;
        if self.diag_inputs_epoch() != epoch
            || sources.iter().any(|source| {
                !source.matches_open_documents(&documents)
                    || self.live_publication_uri_generation(&source.uri) != source.uri_generation
            })
        {
            return Ok(Vec::new());
        }
        for source in &sources {
            let Some(admission) = source.document.path_admission.as_ref() else {
                return Ok(Vec::new());
            };
            admission.check_for(&source.uri, self.store.as_ref())?;
        }
        // Hold actual call/header allocation membership through the final
        // synchronous saved-source observation, after every asynchronous read.
        let index = if retained
            .iter()
            .any(|(_, _, location)| !matches!(location, ReadonlyDeclarationLocation::Original(_)))
        {
            let Ok(index) = self.workspace_index.try_read() else {
                return Ok(Vec::new());
            };
            if retained
                .iter()
                .any(|(_, _, location)| !location.retained_in(&index))
            {
                return Ok(Vec::new());
            }
            Some(index)
        } else {
            None
        };
        let selected = saved
            .iter()
            .map(|source| (source.admission.as_ref(), source.bytes.as_slice()))
            .collect::<Vec<_>>();
        DocumentPathAdmission::with_current_saved_sources(&selected, self.store.as_ref(), || {
            let _retained_index = &index;
            let mut seen = HashSet::new();
            retained
                .iter()
                .filter_map(|(target, analysis, location)| {
                    (location.uri() == target.uri.as_str()).then_some(())?;
                    let span = location.validated_span(&target.document.text, analysis)?;
                    seen.insert((target.uri.clone(), span)).then_some(())?;
                    render(&target.uri, &target.document, analysis, span)
                })
                .collect()
        })
    }

    /// Resolve a list of `(target-uri, byte-span)` pairs to LSP
    /// `Location`s, converting each span against the *target*
    /// document's source.  Targets whose document isn't in the
    /// store, or whose URI doesn't parse, are dropped.
    ///
    /// `targets` holds one entry per reference **site**, but many sites
    /// share a URI (a heavily-`source`d proc can have hundreds of call sites
    /// in a handful of files). Resolving per site would call
    /// [`Self::read_document`] — a full `DocumentState` clone, or a
    /// `std::fs::read_to_string` for a closed file, plus an `edits_settled()`
    /// wait — once per site: 800 sites across 60 files would cost 800 reads
    /// for 60 documents' worth of real work. Each unique URI is read once
    /// instead, and every site's span is looked up against its document's
    /// already-resolved `line_index`. `targets`' own order is untouched (a
    /// document is read once, on its first site, but sites are still emitted
    /// in their original relative order — some callers dedup results and
    /// others, such as the code-lens click target, rely on the order matching
    /// `textDocument/references`).
    pub(in super::super) async fn resolve_target_locations(
        &self,
        targets: Vec<(String, tcl_lexer::Span)>,
    ) -> DocumentSourceResult<Vec<Location>> {
        let mut docs: HashMap<&str, Option<(Uri, DocumentState)>> = HashMap::new();
        for (target_uri, _) in &targets {
            if docs.contains_key(target_uri.as_str()) {
                continue;
            }
            let resolved = match Uri::from_str(target_uri) {
                Ok(parsed) => self.read_document(&parsed).await?.map(|doc| (parsed, doc)),
                Err(_) => None,
            };
            docs.insert(target_uri.as_str(), resolved);
        }
        Ok(targets
            .iter()
            .filter_map(|(target_uri, span)| {
                let (uri, target_doc) = docs.get(target_uri.as_str())?.as_ref()?;
                let start = target_doc
                    .line_index
                    .position_at_utf16(span.start(), &target_doc.text);
                let end = target_doc
                    .line_index
                    .position_at_utf16(span.end(), &target_doc.text);
                Some(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position {
                            line: start.line,
                            character: start.character.get(),
                        },
                        end: Position {
                            line: end.line,
                            character: end.character.get(),
                        },
                    },
                })
            })
            .collect())
    }
}
