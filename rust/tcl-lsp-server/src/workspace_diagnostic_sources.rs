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
//! Actual source declarations consulted by the workspace diagnostic refinement.
//! These inputs are not runtime command lookup or a complete declaration census.

#[cfg(test)]
use super::{Analyser, FromStr};
use super::{
    Arc, DiagnosticUnavailable, DocumentPathAdmission, DocumentPublication, DocumentSourceResult,
    DocumentSourceUnavailable, DocumentState, HashMap, PublishedSourcePath, Uri,
    core_workspace_index, vfs,
};

pub(super) struct WorkspaceDiagnosticSource {
    candidate: core_workspace_index::LogicalWorkspaceCommandCandidate,
    uri: Uri,
    document: Option<DocumentState>,
    admission: Arc<DocumentPathAdmission>,
    saved_bytes: Option<Vec<u8>>,
}

struct WorkspaceConfigurations {
    root: std::path::PathBuf,
    documents: HashMap<Uri, DocumentState>,
}

fn workspace_configuration_documents<'a>(
    documents: &'a HashMap<Uri, DocumentState>,
    root: &'a std::path::Path,
) -> impl Iterator<Item = (&'a Uri, &'a DocumentState)> {
    documents.iter().filter(move |(_, document)| {
        super::Backend::is_bigip_dialect(&document.dialect)
            && !document.backing_file_deleted
            && !tcl_lsp_core::source_decode::should_abstain(document.decode_report.as_ref())
            && document
                .path_admission
                .as_ref()
                .and_then(|admission| admission.path())
                .is_some_and(|path| path.starts_with(root))
    })
}

#[derive(Default)]
pub(super) struct WorkspaceDiagnosticSources {
    selected: Vec<WorkspaceDiagnosticSource>,
    configurations: Option<WorkspaceConfigurations>,
}

impl WorkspaceDiagnosticSources {
    pub(super) fn is_empty(&self) -> bool {
        self.selected.is_empty() && self.configurations.is_none()
    }

    /// Called while the actual document and index query guards are both held.
    /// Capturing a later same-text document would renew an older source issuer.
    pub(super) fn capture(
        candidates: &[core_workspace_index::LogicalWorkspaceCommandCandidate],
        index: &core_workspace_index::WorkspaceIndex,
        documents: &HashMap<Uri, DocumentState>,
        published: &HashMap<Uri, PublishedSourcePath>,
        store: &dyn vfs::SourceStore,
    ) -> Result<Self, DiagnosticUnavailable> {
        let mut selected = Vec::new();
        for candidate in candidates {
            if !index.logical_command_candidate_is_current(candidate) {
                return Err(DiagnosticUnavailable::Stale);
            }
            let uri = candidate
                .uri()
                .parse::<Uri>()
                .map_err(|_| DiagnosticUnavailable::Context)?;
            let document = documents.get(&uri).cloned();
            let admission = if let Some(document) = &document {
                let current = document.clone().normalised_for_analysis();
                if tcl_lsp_core::source_decode::should_abstain(current.decode_report.as_ref()) {
                    return Err(DiagnosticUnavailable::Context);
                }
                if current.publication != DocumentPublication::Indexed {
                    return Err(DiagnosticUnavailable::Pending);
                }
                if current.backing_file_deleted
                    || candidate.source_image().bytes() != current.text.as_bytes()
                {
                    return Err(DiagnosticUnavailable::Stale);
                }
                current
                    .path_admission
                    .clone()
                    .ok_or(DiagnosticUnavailable::Context)?
            } else {
                Arc::clone(
                    &published
                        .get(&uri)
                        .ok_or(DiagnosticUnavailable::Context)?
                        .admission,
                )
            };
            admission
                .check_for(&uri, store)
                .map_err(|_| DiagnosticUnavailable::Context)?;
            selected.push(WorkspaceDiagnosticSource {
                candidate: candidate.clone(),
                uri,
                document,
                admission,
                saved_bytes: None,
            });
        }
        Ok(Self {
            selected,
            configurations: None,
        })
    }

    /// Retain the current BIG-IP source buffers in the caller's workspace folder.
    /// A same-path document replacement cannot renew the captured source.
    pub(super) fn capture_configurations(
        &mut self,
        documents: &HashMap<Uri, DocumentState>,
        root: &std::path::Path,
        store: &dyn vfs::SourceStore,
    ) -> Result<(), DiagnosticUnavailable> {
        let mut providers = HashMap::new();
        for (uri, state) in workspace_configuration_documents(documents, root) {
            let admission = state
                .path_admission
                .as_ref()
                .ok_or(DiagnosticUnavailable::Context)?;
            admission
                .check_for(uri, store)
                .map_err(|_| DiagnosticUnavailable::Context)?;
            providers.insert(uri.clone(), state.clone());
        }
        self.configurations = Some(WorkspaceConfigurations {
            root: root.to_owned(),
            documents: providers,
        });
        Ok(())
    }

    pub(super) fn configuration_texts(&self) -> impl Iterator<Item = &str> {
        self.configurations
            .iter()
            .flat_map(|sources| sources.documents.values())
            .map(|source| source.text.as_ref())
    }

    /// Read only the exact selected saved inputs, after releasing query guards.
    pub(super) async fn read_saved(
        &mut self,
        store: &Arc<dyn vfs::SourceStore>,
        encoding: tcl_registry::value_transfer::ScriptEncoding,
    ) -> Result<(), DiagnosticUnavailable> {
        for selected in &mut self.selected {
            if selected.document.is_some() {
                continue;
            }
            let path = selected
                .admission
                .path()
                .ok_or(DiagnosticUnavailable::Context)?
                .to_owned();
            let store = Arc::clone(store);
            let bytes = crate::rt::spawn_blocking(move || store.read(&path))
                .await
                .map_err(|_| DiagnosticUnavailable::ProviderError)?
                .map_err(|_| DiagnosticUnavailable::Context)?;
            let (text, decoding) = tcl_lsp_core::source_decode::decode_source_as(&bytes, encoding);
            if tcl_lsp_core::source_decode::should_abstain(Some(&decoding)) {
                return Err(DiagnosticUnavailable::Context);
            }
            if tcl_lexer::normalise_lone_cr(&text).as_bytes()
                != selected.candidate.source_image().bytes()
            {
                return Err(DiagnosticUnavailable::Stale);
            }
            selected.saved_bytes = Some(bytes);
        }
        Ok(())
    }

    /// Reborrow actual source revisions and declaration allocation membership.
    /// The caller retains both guards through the subsequent source callback.
    pub(super) fn matches(
        &self,
        documents: &HashMap<Uri, DocumentState>,
        index: &core_workspace_index::WorkspaceIndex,
    ) -> bool {
        self.configurations.as_ref().is_none_or(|capture| {
            let mut seen = 0;
            let current =
                workspace_configuration_documents(documents, &capture.root).all(|(uri, state)| {
                    seen += 1;
                    capture
                        .documents
                        .get(uri)
                        .is_some_and(|retained| same_document_revision(retained, state))
                });
            current && seen == capture.documents.len()
        }) && self.selected.iter().all(|selected| {
            if !index.logical_command_candidate_is_current(&selected.candidate) {
                return false;
            }
            match (&selected.document, documents.get(&selected.uri)) {
                (Some(retained), Some(current)) => {
                    same_document_revision(retained, current)
                        && current.publication == DocumentPublication::Indexed
                        && !current.backing_file_deleted
                        && current
                            .path_admission
                            .as_ref()
                            .is_some_and(|admission| Arc::ptr_eq(admission, &selected.admission))
                }
                (None, None) => selected.saved_bytes.is_some(),
                _ => false,
            }
        })
    }

    pub(super) fn inputs<'a>(
        &'a self,
        store: &dyn vfs::SourceStore,
    ) -> DocumentSourceResult<WorkspaceDiagnosticSourceInputs<'a>> {
        let mut mappings = Vec::new();
        let mut saved = Vec::new();
        for selected in &self.selected {
            selected.admission.check_for(&selected.uri, store)?;
            if selected.document.is_some()
                && matches!(
                    selected.admission.as_ref(),
                    DocumentPathAdmission::NoFilesystemPath { .. }
                )
            {
                continue;
            }
            let path = selected.admission.current_saved_source_path(store)?;
            mappings.push(path);
            if let Some(bytes) = &selected.saved_bytes {
                saved.push((path, bytes.as_slice()));
            }
        }
        for (uri, state) in self
            .configurations
            .iter()
            .flat_map(|sources| &sources.documents)
        {
            let admission = state
                .path_admission
                .as_ref()
                .expect("captured configuration admission");
            admission.check_for(uri, store)?;
            mappings.push(admission.current_saved_source_path(store)?);
        }
        Ok(WorkspaceDiagnosticSourceInputs { mappings, saved })
    }
}

fn same_document_revision(retained: &DocumentState, current: &DocumentState) -> bool {
    current.revision == retained.revision
        && current.version == retained.version
        && current.text == retained.text
        && current.dialect == retained.dialect
        && current.dialect_resolution_generation == retained.dialect_resolution_generation
        && current.resolved_dialect_generation == retained.resolved_dialect_generation
        && match (
            retained.path_admission.as_ref(),
            current.path_admission.as_ref(),
        ) {
            (Some(left), Some(right)) => Arc::ptr_eq(left, right),
            (None, None) => true,
            _ => false,
        }
}

#[derive(Default)]
pub(super) struct WorkspaceDiagnosticSourceInputs<'a> {
    pub(super) mappings: Vec<&'a vfs::SourceUriPath>,
    pub(super) saved: Vec<(&'a vfs::SourceUriPath, &'a [u8])>,
}

impl WorkspaceDiagnosticSourceInputs<'_> {
    pub(super) fn publish_base<R, F: FnOnce(Vec<tower_lsp_server::ls_types::Diagnostic>) -> R>(
        &self,
        store: &dyn vfs::SourceStore,
        publish: &mut Option<F>,
        diagnostics: &mut Option<Vec<tower_lsp_server::ls_types::Diagnostic>>,
    ) -> Option<R> {
        let mut result = None;
        let mut commit = || {
            result = Some(publish.take().expect("publication callback is available")(
                diagnostics.take().expect("base report is available"),
            ));
        };
        if self.mappings.is_empty() && self.saved.is_empty() {
            commit();
        } else if store
            .with_current_source_inputs(&self.mappings, &self.saved, &[], &mut commit)
            .is_err()
        {
            return None;
        }
        result
    }
}

impl WorkspaceDiagnosticSources {
    /// Join caller admissions and all consulted saved-provider images once.
    /// Callers retain their actual documents/index/input guards through publish.
    pub(super) fn with_current_batch<T>(
        sources: &[&Self],
        admissions: &[&DocumentPathAdmission],
        store: &dyn vfs::SourceStore,
        mut publish: impl FnMut() -> T,
    ) -> DocumentSourceResult<T> {
        if sources.iter().all(|sources| sources.is_empty()) {
            return DocumentPathAdmission::with_current(admissions, store, publish);
        }
        let mut mappings = Vec::new();
        let mut saved = Vec::new();
        for admission in admissions {
            admission.check_current(store)?;
            if !matches!(admission, DocumentPathAdmission::NoFilesystemPath { .. }) {
                mappings.push(admission.current_saved_source_path(store)?);
            }
        }
        for sources in sources {
            let inputs = sources.inputs(store)?;
            mappings.extend(inputs.mappings);
            saved.extend(inputs.saved);
        }
        if mappings.is_empty() && saved.is_empty() {
            return Ok(publish());
        }
        let uri = admissions
            .first()
            .map(|admission| admission.uri())
            .or_else(|| {
                sources.iter().find_map(|sources| {
                    sources
                        .selected
                        .first()
                        .map(|source| &source.uri)
                        .or_else(|| sources.configurations.as_ref()?.documents.keys().next())
                })
            })
            .expect("nonempty selected sources have an original URI");
        let mut result = None;
        store
            .with_current_source_inputs(&mappings, &saved, &[], &mut || result = Some(publish()))
            .map_err(|cause| DocumentSourceUnavailable::Store {
                uri: uri.clone(),
                cause,
            })?;
        result.ok_or_else(|| DocumentSourceUnavailable::Store {
            uri: uri.clone(),
            cause: vfs::SourceUriPathUnavailable::ProviderUnavailable,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfs::{MemoryStore, SourceStore};

    #[test]
    fn workspace_bigip_configs_promote_only_current_data_group_keys() {
        let store = MemoryStore::new();
        let index = core_workspace_index::WorkspaceIndex::new();
        let root = std::path::Path::new("/workspace");
        let uri = Uri::from_str("file:///workspace/bigip.conf").unwrap();
        let config = "ltm data-group internal /Common/int_dg { type integer\n records { 129 { data hit } } }";
        store.upsert_uri(
            uri.as_str(),
            "/workspace/bigip.conf",
            config.as_bytes().to_vec(),
        );
        let admission = Arc::new(DocumentPathAdmission::capture(&uri, &store).unwrap());
        let document = DocumentState::with_version(config.to_owned(), "f5-bigip".to_owned(), 1)
            .with_path_admission(admission);
        let mut documents = HashMap::new();
        let mut empty = WorkspaceDiagnosticSources::default();
        empty
            .capture_configurations(&documents, root, &store)
            .unwrap();
        documents.insert(uri.clone(), document);
        assert!(!empty.matches(&documents, &index));

        let mut sources = WorkspaceDiagnosticSources::default();
        sources
            .capture_configurations(&documents, root, &store)
            .unwrap();
        assert!(sources.matches(&documents, &index));
        let irule = "when CLIENT_DATA {\n binary scan [TCP::payload] c item\n class match -- $item equals int_dg\n}";
        let mut result = Analyser::new().analyse(irule, "f5-irules");
        let original = result
            .diagnostics
            .iter()
            .find(|diag| diag.code.as_str() == "W156")
            .unwrap();
        assert_eq!(original.severity, tcl_compiler::analyser::Severity::Hint);
        super::super::diagnostics::refine_signed_data_group_diagnostics(
            &mut result.diagnostics,
            sources.configuration_texts(),
        );
        let warning = result
            .diagnostics
            .iter()
            .find(|diag| diag.code.as_str() == "W156")
            .unwrap();
        assert_eq!(warning.severity, tcl_compiler::analyser::Severity::Warning);
        assert!(warning.message.contains("Configured key 129"));

        documents.get_mut(&uri).unwrap().bump_analysis_generation();
        assert!(!sources.matches(&documents, &index));
        documents.remove(&uri);
        assert!(!sources.matches(&documents, &index));
    }

    #[test]
    fn actual_selected_source_revision_and_header_allocation_are_not_renewed() {
        let store = MemoryStore::new();
        let uri = Uri::from_str("file:///provider.tcl").unwrap();
        let source = "proc helper {} {}";
        store.upsert_uri(uri.as_str(), "/provider.tcl", source.as_bytes().to_vec());
        let admission = Arc::new(DocumentPathAdmission::capture(&uri, &store).unwrap());
        let provider = Analyser::new().analyse(source, "tcl");
        let caller = Analyser::new().analyse("helper", "tcl");
        let request = tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(
            "helper", &caller, 0,
        ).unwrap();
        let mut index =
            core_workspace_index::WorkspaceIndex::from_documents([(uri.as_str(), &provider)]);
        let candidates = index.logical_command_candidates(&request, "file:///caller.tcl");
        assert_eq!(candidates.len(), 1);
        let document = DocumentState::with_version(source.to_owned(), "tcl".to_owned(), 1)
            .with_path_admission(admission);
        let mut documents = HashMap::from([(uri.clone(), document)]);
        let dependencies = WorkspaceDiagnosticSources::capture(
            &candidates,
            &index,
            &documents,
            &HashMap::new(),
            &store,
        )
        .unwrap();
        assert!(dependencies.matches(&documents, &index));
        let indexed_documents = documents.clone();
        documents.get_mut(&uri).unwrap().bump_analysis_generation();
        assert!(!dependencies.matches(&documents, &index));
        assert!(
            WorkspaceDiagnosticSources::capture(
                &candidates,
                &index,
                &documents,
                &HashMap::new(),
                &store
            )
            .is_err()
        );
        let fresh = WorkspaceDiagnosticSources::capture(
            &candidates,
            &index,
            &indexed_documents,
            &HashMap::new(),
            &store,
        )
        .unwrap();
        assert!(fresh.matches(&indexed_documents, &index));
        let replacement = Analyser::new().analyse(source, "tcl");
        index.replace_document(uri.as_str(), &replacement);
        assert!(!fresh.matches(&indexed_documents, &index));
        assert!(store.document_uri_path(uri.as_str()).is_ok());
    }
}
