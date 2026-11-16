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

//! Conditional autoload advice: the current provider headers and the locations read from them after a view capture.

use super::super::{Arc, Backend, DocumentSourceResult};
use super::{
    ConditionalAutoloadProvider, ConditionalAutoloadProviderSource, OriginalSourceScopeProjection,
    PublishedOriginalSourceView,
};
use tower_lsp_server::ls_types::Uri;

impl Backend {
    pub(super) fn conditional_autoload_saved_sources(
        publications: &[&super::super::super::LibraryPublicationSource],
        admissions: &[&super::super::super::DocumentPathAdmission],
    ) -> Option<crate::SavedSourceBytes> {
        (publications.len() == admissions.len()).then_some(())?;
        let mut saved = Vec::new();
        for (publication, expected) in publications.iter().zip(admissions) {
            if publication.open_revision.is_some() {
                continue;
            }
            // Only this publication's original bytes can lease its producer.
            // Equal normalized text from a later read cannot renew the receipt.
            let admission = publication.document.path_admission.as_ref()?;
            let bytes = publication.document.saved_source_bytes.as_ref()?;
            if admission.as_ref() != *expected || admission.uri() != &publication.uri {
                return None;
            }
            saved.push((Arc::clone(admission), Arc::clone(bytes)));
        }
        Some(saved)
    }

    pub(super) async fn capture_conditional_autoload_providers(
        &self,
        files: Vec<std::path::PathBuf>,
    ) -> DocumentSourceResult<Option<Vec<ConditionalAutoloadProvider>>> {
        // Keep each actual source snapshot and its existing complete readonly
        // owner. A closed, unindexed index target has no Salsa SourceFile; the
        // fresh-source owner retains its real input and currency instead.
        let mut providers = Vec::new();
        for file in files {
            let Some(uri) = crate::canonical_file_uri(&file) else {
                return Ok(None);
            };
            let Some(document) = self.read_document(&uri).await? else {
                return Ok(None);
            };
            let Some(owner) = self
                .capture_workspace_symbol_analysis(&uri, &document)
                .await?
            else {
                return Ok(None);
            };
            let Some(admission) = document.path_admission.clone() else {
                return Ok(None);
            };
            if owner.admission() != admission.as_ref() || !owner.matches_snapshot(&uri, &document) {
                return Ok(None);
            }
            let source = Arc::clone(&document.text);
            let Some(publication) = self.library_publication_source(&uri, document).await else {
                return Ok(None);
            };
            providers.push(ConditionalAutoloadProvider {
                source: ConditionalAutoloadProviderSource {
                    uri,
                    source,
                    admission,
                    publication,
                },
                owner,
            });
        }
        Ok(Some(providers))
    }

    pub(super) fn conditional_autoload_provider_headers<'a>(
        providers: &'a [ConditionalAutoloadProvider],
        files: &[Uri],
    ) -> Vec<tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'a>> {
        providers
            .iter()
            .filter(|provider| files.contains(&provider.source.uri))
            .map(
                |provider| tcl_lsp_core::original_declaration::OriginalDeclarationDocument {
                    uri: provider.source.uri.as_str(),
                    source: &provider.source.source,
                    analysis: provider.owner.analysis(),
                },
            )
            .collect()
    }

    /// Read only real configured index candidates after exact view capture.
    /// Query and provider owners survive every await and rejoin atomically.
    pub(super) async fn conditional_source_view_autoload_locations(
        &self,
        projection: &OriginalSourceScopeProjection,
        owners: &[Arc<PublishedOriginalSourceView>],
        requests: &[tcl_lsp_core::original_source_views::ConditionalExternalAutoloadRequest],
    ) -> DocumentSourceResult<Option<Vec<tower_lsp_server::ls_types::Location>>> {
        if requests.is_empty() {
            return Ok(None);
        }
        // Join actual configured-index discovery, as the ordinary library
        // request does, before retaining its resolver generation.
        drop(self.workspace_scan_gate.lock().await);
        let (files, request_files, revision) = {
            let resolver = self.package_resolver.read().await;
            let mut files = Vec::new();
            let mut request_files = Vec::new();
            for request in requests {
                let candidates = request.files(&resolver);
                if candidates.is_empty() {
                    return Ok(None);
                }
                request_files.push(
                    candidates
                        .iter()
                        .map(crate::canonical_file_uri)
                        .collect::<Option<Vec<_>>>(),
                );
                for file in candidates {
                    if !files.contains(&file) {
                        files.push(file);
                    }
                }
            }
            let Some(request_files) = request_files.into_iter().collect::<Option<Vec<_>>>() else {
                return Ok(None);
            };
            (files, request_files, resolver.revision())
        };
        let Some(providers) = self.capture_conditional_autoload_providers(files).await? else {
            return Ok(None);
        };
        self.with_selected_original_source_views_and_provider_headers(
            &projection.ids,
            Some(projection.revision),
            &providers,
            |views| {
                Self::same_original_source_view_inventory(owners, views).then_some(())?;
                let resolver = self.package_resolver.try_read().ok()?;
                (resolver.revision() == revision).then_some(())?;
                let (publications, ancestors) = Self::original_source_view_labels(views)?;
                let inventory =
                    Self::original_source_view_inventory(views, &publications, &ancestors);
                let mut selected = None;
                for (request, files) in requests.iter().zip(&request_files) {
                    // Joint physical leasing does not let an interpretation
                    // borrow a header from another request's indexed file.
                    let headers = Self::conditional_autoload_provider_headers(&providers, files);
                    let header =
                        tcl_lsp_core::original_source_views::conditional_external_autoload_header(
                            request, &inventory, &headers,
                        )?;
                    if selected.as_ref().is_some_and(|old: &tcl_lsp_core::original_source_views::ConditionalExternalSourceHeader| old.declaration() != header.declaration()) {
                        return None;
                    }
                    selected = Some(header);
                }
                let header = selected?;
                let identity = header.declaration();
                let provider = providers
                    .iter()
                    .find(|provider| provider.source.uri.as_str() == identity.uri())?;
                identity
                    .is_current(identity.uri(), &provider.source.source, provider.owner.analysis())
                    .then_some(())?;
                let range = tcl_lsp_core::definition::span_to_range(
                    &provider.source.source,
                    &tcl_lexer::LineIndex::new_lsp(&provider.source.source),
                    identity.span(),
                );
                Some(vec![tower_lsp_server::ls_types::Location {
                    uri: provider.source.uri.clone(),
                    range: super::super::super::lift_lsp_range(range),
                }])
            },
        )
        .await
    }
}
