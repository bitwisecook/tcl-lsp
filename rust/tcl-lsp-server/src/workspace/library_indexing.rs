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

//! On-demand indexing of library and package files a query reaches.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::{definition as core_definition, workspace_index as core_workspace_index};
use tower_lsp_server::ls_types::{Position, Uri};

use super::package_database::{document_auto_path_dirs, document_module_paths};
use crate::documents::{DocumentPublication, DocumentSourceResult, canonical_file_uri};
use crate::uri_norm::DocumentPath as _;
use crate::{Backend, DocumentState};

pub(in super::super) struct LibraryPublicationSource {
    pub(in super::super) uri: Uri,
    pub(in super::super) document: DocumentState,
    pub(in super::super) open_revision: Option<u64>,
    pub(in super::super) uri_generation: u64,
}

impl LibraryPublicationSource {
    pub(in super::super) fn matches_open_documents(
        &self,
        documents: &HashMap<Uri, DocumentState>,
    ) -> bool {
        match self.open_revision {
            Some(revision) => documents.get(&self.uri).is_some_and(|current| {
                let current = current.clone().normalised_for_analysis();
                current.publication == DocumentPublication::Indexed
                    && !current.backing_file_deleted
                    && current.revision == revision
                    && current.version == self.document.version
                    && current.text == self.document.text
                    && current.dialect == self.document.dialect
                    && current.dialect_resolution_generation
                        == self.document.dialect_resolution_generation
                    && current.resolved_dialect_generation
                        == self.document.resolved_dialect_generation
            }),
            None => !documents.contains_key(&self.uri),
        }
    }
}

impl Backend {
    /// Index library providers only after the checked Logical source owner has
    /// supplied ordered external header coordinates. The resolver receives
    /// those typed slots directly; rendered names are returned only after a
    /// current source declaration has been selected from the index.
    pub(in super::super) async fn ensure_logical_library_candidates(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<
        Option<(
            Vec<core_workspace_index::LogicalWorkspaceCommandCandidate>,
            u64,
        )>,
    > {
        let index = tcl_lexer::LineIndex::new(source);
        let offset =
            index.offset_at_utf16(pos.line, tcl_lexer::Utf16Col::new(pos.character), source);
        let Some(request) = tcl_compiler::registry_invocation::source_structure::
            source_logical_command_candidates_at(source, analysis, offset)
        else {
            return Ok(None);
        };
        if request.has_earlier_unknown_mutations() || request.has_lookup_operand_effects() {
            return Ok(None);
        }
        let Some(coordinates) = request.external_header_coordinates() else {
            return Ok(None);
        };

        drop(self.workspace_scan_gate.lock().await);
        let (mut files, mut resolver_revision) =
            self.logical_library_files_for_headers(coordinates).await;
        if files.is_empty() {
            (files, resolver_revision) = self
                .required_library_files(uri, source, analysis, offset)
                .await;
            let inheritance = self.source_inheritance_for(uri, analysis).await;
            if let Some(original) = inheritance.original.as_ref() {
                let available = original.available_at(offset, None);
                let target = analysis.resolved_profile().and_then(|profile| {
                    tcl_registry::InvocationDialect::of_profile(profile).tcl_version
                });
                let resolver = self.package_resolver.read().await;
                for requirement in &available {
                    for file in resolver.resolve_requirement_advice(requirement, target) {
                        if !files.contains(&file) {
                            files.push(file);
                        }
                    }
                }
            }
        }
        if !files.is_empty()
            && self
                .capture_and_publish_library_files(&files)
                .await?
                .is_none()
        {
            return Ok(None);
        }

        // Revalidate the caller's complete source and exact query after all
        // asynchronous capture/publication work. A stale request leaves no
        // selected provider candidate.
        let Some(current) = self.read_document(uri).await? else {
            return Ok(None);
        };
        if current.text.as_ref() != source {
            return Ok(None);
        }
        if self.package_resolver.read().await.revision() != resolver_revision {
            return Ok(None);
        }
        let current_analysis = self
            .analysis_for(uri, current.text.clone(), current.dialect.clone())
            .await?;
        let Some(current_request) = tcl_compiler::registry_invocation::source_structure::
            source_logical_command_candidates_at(source, &current_analysis, offset)
        else {
            return Ok(None);
        };
        if current_request != request {
            return Ok(None);
        }
        let index = self.workspace_index.read().await;
        Ok(Some((
            index
                .logical_command_candidates(&request, uri.as_str())
                .into_iter()
                .filter(|candidate| index.logical_command_candidate_is_current(candidate))
                .collect(),
            resolver_revision,
        )))
    }

    async fn logical_library_files_for_headers(
        &self,
        coordinates: &[tcl_core_types::ByteCommandSlot],
    ) -> (Vec<PathBuf>, u64) {
        let resolver = self.package_resolver.read().await;
        let files = coordinates
            .iter()
            .find_map(|coordinate| {
                let files = resolver.resolve_logical_auto_command(coordinate);
                (!files.is_empty()).then_some(files)
            })
            .unwrap_or_default();
        (files, resolver.revision())
    }

    #[cfg(test)]
    pub(in super::super) async fn ensure_autoload_indexed(
        &self,
        word: &str,
        namespace: &str,
    ) -> DocumentSourceResult<Option<String>> {
        drop(self.workspace_scan_gate.lock().await);
        let files = {
            let resolver = self.package_resolver.read().await;
            resolver.resolve_auto_command(word, namespace)
        };
        self.merge_library_files(word, namespace, files).await
    }

    pub(in super::super) async fn required_library_files(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
    ) -> (Vec<PathBuf>, u64) {
        let body = analysis.innermost_definition_body_span(offset);
        let requirements = analysis
            .package_requires
            .iter()
            .filter(|requirement| {
                let start = requirement.range.start();
                !requirement.conditional
                    && !requirement.control_flow
                    && analysis
                        .innermost_definition_body_span(start)
                        .is_none_or(|span| Some(span) == body)
                    && tcl_compiler::analyser::indirection::in_effect_within(start, offset, body)
            })
            .collect::<Vec<_>>();
        if requirements.is_empty() {
            return (Vec::new(), self.package_resolver.read().await.revision());
        }
        drop(self.workspace_scan_gate.lock().await);
        // This document's own `auto_path` mutations, folded in first: the
        // workspace scan already did this for every file it read from disk,
        // but an **open** document is deliberately skipped by that scan (its
        // buffer, not the disk copy, is authoritative), and a document opened
        // after the scan was never seen at all.  Bounded by this one
        // document's entries, and a directory the resolver has already
        // scanned costs nothing, so this stays a miss-path lookup rather than
        // a rescan.
        {
            let dirs = document_auto_path_dirs(uri, analysis);
            let modules = document_module_paths(uri, source, analysis);
            if !dirs.is_empty() || modules.as_ref().is_some_and(|(_, roots)| !roots.is_empty()) {
                let mut resolver = self.package_resolver.write().await;
                for dir in &dirs {
                    resolver.scan_path_in(self.store.as_ref(), dir);
                }
                if let Some((release, roots)) = modules {
                    for root in roots {
                        resolver.prepend_module_path_in(self.store.as_ref(), &root, release, 256);
                    }
                }
            }
        }
        // The interpreter's `package prefer` state on entry to this document:
        // the mode a file that (transitively) `source`s it already latched.
        // The state is interpreter-global, and along the `source` graph "ran
        // first" is a static fact, so a `package prefer latest` in the entry
        // file really is in force here.
        let inherited_prefer = if self
            .workspace_index
            .read()
            .await
            .source_ancestor_prefers_latest_from_resolved(uri.as_str())
        {
            tcl_lsp_core::package_resolver::PackagePrefer::Latest
        } else {
            self.default_package_prefer().await
        };

        {
            let resolver = self.package_resolver.read().await;
            let mut files: Vec<PathBuf> = Vec::new();
            for req in requirements {
                // The document's own constraint decides which release is
                // indexed: `package require widget 2.0` against a workspace
                // holding 1.5 and 2.3 must navigate into 2.3 (C Tcl's
                // `SelectPackage` rule — see
                // `PackageResolver::resolve_require`), not whichever directory
                // the scan happened to read first.  `-exact` is carried
                // through, so `package require -exact widget 2.0` navigates
                // into 2.0 or nothing at all — never into 2.3.
                // The `package prefer` mode is taken **at this require's own
                // position**, so a document that raised the interpreter to
                // `latest` above it navigates into the prerelease it really
                // loads.
                let prefer = tcl_lsp_core::package_resolver::package_prefer_at(
                    analysis,
                    req.range.start(),
                    inherited_prefer,
                );
                let requirement = tcl_lsp_core::package_resolver::PackageRequirementAdvice::from_source_requirement(req, prefer);
                let target = analysis.resolved_profile().and_then(|profile| {
                    tcl_registry::InvocationDialect::of_profile(profile).tcl_version
                });
                for f in resolver.resolve_requirement_advice(&requirement, target) {
                    if !files.contains(&f) {
                        files.push(f);
                    }
                }
            }
            (files, resolver.revision())
        }
    }

    /// Merge `files` (library / package implementation files) into the shared
    /// workspace index and report which auto-qualified candidate of
    /// `word` in `namespace` they define, if any. Used by the autoload
    /// parser fixtures through [`Self::ensure_autoload_indexed`].
    #[cfg(test)]
    async fn merge_library_files(
        &self,
        word: &str,
        namespace: &str,
        files: Vec<PathBuf>,
    ) -> DocumentSourceResult<Option<String>> {
        if files.is_empty() {
            return Ok(None);
        }
        let candidates = tcl_lsp_core::package_resolver::auto_qualify(word, namespace);
        // The index and `all_procs` key absolute (`::`-prefixed) names, while
        // `auto_qualify` yields a bare name for a global command.
        let absolute: Vec<String> = candidates
            .iter()
            .map(|cand| {
                if tcl_syntax::naming::unroot_rooted_key(cand).is_some() {
                    cand.clone()
                } else {
                    tcl_syntax::naming::root_unrooted_key(cand)
                }
            })
            .collect();
        {
            let index = self.workspace_index.read().await;
            if let Some(existing) = absolute
                .iter()
                .find(|cand| index.workspace_command_exists(cand))
            {
                return Ok(Some(existing.clone()));
            }
        }
        let Some(analysed) = self.capture_and_publish_library_files(&files).await? else {
            return Ok(None);
        };
        Ok(analysed.iter().find_map(|(_, _, receipt)| {
            let file_analysis = receipt.analysis();
            absolute
                .iter()
                .find(|cand| {
                    file_analysis.all_procs.contains_key(*cand)
                        || file_analysis.all_classes.contains_key(*cand)
                })
                .cloned()
        }))
    }

    pub(in super::super) async fn library_publication_source(
        &self,
        uri: &Uri,
        document: DocumentState,
    ) -> Option<LibraryPublicationSource> {
        let documents = self.documents.lock("library_source_capture").await;
        let source = LibraryPublicationSource {
            uri: uri.clone(),
            open_revision: documents.get(uri).map(|current| current.revision),
            uri_generation: self.live_publication_uri_generation(uri),
            document,
        };
        source.matches_open_documents(&documents).then_some(source)
    }

    pub(in super::super) async fn publish_original_library_analyses(
        &self,
        epoch: u64,
        caller: &LibraryPublicationSource,
        analysed: &[(LibraryPublicationSource, Arc<AnalysisResult>)],
    ) -> bool {
        let _inputs = self.analyser_inputs_gate.read().await;
        let _rehoming = self.rehoming_gate.lock().await;
        loop {
            if self.diag_inputs_epoch() != epoch {
                return false;
            }
            for source in std::iter::once(caller).chain(analysed.iter().map(|(source, _)| source)) {
                if source.open_revision.is_none() {
                    let Some(path) = source.uri.document_path().map(std::borrow::Cow::into_owned)
                    else {
                        return false;
                    };
                    let store = Arc::clone(&self.store);
                    let encoding = self.declared_script_encoding();
                    let Ok(Ok((text, _))) =
                        crate::rt::spawn_blocking(move || store.read_source_as(&path, encoding))
                            .await
                    else {
                        return false;
                    };
                    if tcl_lexer::normalise_lone_cr(&text).as_ref() != source.document.text.as_ref()
                    {
                        return false;
                    }
                }
            }
            let documents = self.documents.lock("original_library_publication").await;
            if self.diag_inputs_epoch() != epoch
                || std::iter::once(caller)
                    .chain(analysed.iter().map(|(source, _)| source))
                    .any(|source| {
                        !source.matches_open_documents(&documents)
                            || self.live_publication_uri_generation(&source.uri)
                                != source.uri_generation
                    })
            {
                return false;
            }
            let Ok(mut index) = self.workspace_index.try_write() else {
                drop(documents);
                drop(self.workspace_index.write().await);
                continue;
            };
            for (source, analysis) in analysed {
                let Some(config) = analysis.body_lexer_config else {
                    return false;
                };
                if !analysis.matches_original_source_image(
                    &tcl_lexer::SourceImage::document(&source.document.text),
                    config,
                ) {
                    return false;
                }
            }
            for (source, analysis) in analysed {
                index.replace_document(source.uri.as_str(), analysis);
            }
            drop(index);
            drop(documents);
            let mut seeds = self.rehomed_source_seeds.lock().await;
            let mut autoloaded = self.autoloaded_library_uris.lock().await;
            for (source, _) in analysed {
                seeds.remove(source.uri.as_str());
                autoloaded.insert(source.uri.as_str().to_owned());
            }
            return true;
        }
    }

    pub(in super::super) async fn ensure_original_library_indexed(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<bool> {
        let Some(request) = core_definition::original_autoload_request_at(
            source,
            analysis,
            pos.line,
            pos.character,
        ) else {
            return Ok(false);
        };
        let epoch = self.diag_inputs_epoch();
        drop(self.workspace_scan_gate.lock().await);
        let mut files = request.files(&*self.package_resolver.read().await);
        let (required, _) = self
            .required_library_files(uri, source, analysis, request.head_offset())
            .await;
        for file in required {
            if !files.contains(&file) {
                files.push(file);
            }
        }
        let mut analysed = Vec::new();
        for path in files {
            let Some(owner) = canonical_file_uri(&path) else {
                return Ok(false);
            };
            let Some(document) = self.read_document(&owner).await? else {
                return Ok(false);
            };
            let current = self
                .analysis_for(&owner, document.text.clone(), document.dialect.clone())
                .await?;
            let Some(config) = current.body_lexer_config else {
                return Ok(false);
            };
            if !current.matches_original_source_image(
                &tcl_lexer::SourceImage::document(&document.text),
                config,
            ) {
                return Ok(false);
            }
            let Some(publication) = self.library_publication_source(&owner, document).await else {
                return Ok(false);
            };
            analysed.push((publication, current));
        }
        let Some(document) = self.read_document(uri).await? else {
            return Ok(false);
        };
        let current = self
            .analysis_for(uri, document.text.clone(), document.dialect.clone())
            .await?;
        if document.text.as_ref() != source
            || self.diag_inputs_epoch() != epoch
            || core_definition::original_autoload_request_at(
                &document.text,
                &current,
                pos.line,
                pos.character,
            )
            .as_ref()
                != Some(&request)
        {
            return Ok(false);
        }
        let Some(caller) = self.library_publication_source(uri, document).await else {
            return Ok(false);
        };
        Ok(self
            .publish_original_library_analyses(epoch, &caller, &analysed)
            .await)
    }
}
