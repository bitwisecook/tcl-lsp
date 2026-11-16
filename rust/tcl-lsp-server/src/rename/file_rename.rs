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

//! `workspace/willRenameFiles`: edits to the `source` literals that name a moved file.

use super::{
    AnalysisResult, Arc, Backend, ControlFlow, DocumentChanges, DocumentState, DocumentsGuard,
    HashMap, OneOf, OptionalVersionedTextDocumentIdentifier, PathBuf, RenameFilesParams,
    TextDocumentEdit, TextEdit, Uri, WorkspaceEdit, core_file_ops, core_workspace_index,
    fresh_analysis_cache, jsonrpc, lift_span, uri_norm,
};

pub(in super::super) struct FileRenameInputSnapshot {
    pub(in super::super) analyser_inputs_epoch: u64,
    pub(in super::super) class_factory_generation: u64,
    pub(in super::super) sidecar_revision: Arc<fresh_analysis_cache::SidecarRevision>,
}

pub(super) struct FileRenamePublicationGuards<'a> {
    _inputs: tokio::sync::RwLockReadGuard<'a, ()>,
    _factory: tokio::sync::RwLockReadGuard<'a, u64>,
    pub(in super::super) documents: DocumentsGuard<'a>,
}

pub(super) enum FileRenamePublicationWait {
    Inputs,
    Factory,
    Documents,
    IndexRead,
    IndexWrite,
}

pub(in super::super) struct FileRenameSourcePublication {
    pub(in super::super) uri: Uri,
    pub(in super::super) document: DocumentState,
    pub(in super::super) analysis: Arc<AnalysisResult>,
}

pub(in super::super) type FileRenameSources = HashMap<Uri, (DocumentState, Arc<AnalysisResult>)>;

pub(in super::super) type HeldFileRename = (
    Uri,
    DocumentState,
    Arc<AnalysisResult>,
    Vec<core_file_ops::RenameEdit>,
);

impl Backend {
    pub(in super::super) fn file_rename_paths(
        params: &RenameFilesParams,
    ) -> Option<Vec<(PathBuf, PathBuf)>> {
        params
            .files
            .iter()
            .map(|file| {
                let old: Uri = file.old_uri.parse().ok()?;
                let new: Uri = file.new_uri.parse().ok()?;
                Some((
                    uri_norm::file_action_path(&old)?,
                    uri_norm::file_action_path(&new)?,
                ))
            })
            .collect::<Option<Vec<_>>>()
    }

    pub(in super::super) async fn file_rename_input_snapshot(
        &self,
    ) -> Option<FileRenameInputSnapshot> {
        let _inputs = self.analyser_inputs_gate.read().await;
        let factory_generation = self.class_factory_generation.read().await;
        let sidecar_revision = self
            .fresh_analysis_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .published_revision()?;
        Some(FileRenameInputSnapshot {
            analyser_inputs_epoch: self.diag_inputs_epoch(),
            class_factory_generation: *factory_generation,
            sidecar_revision,
        })
    }

    pub(super) fn try_file_rename_publication_guards<'a>(
        &'a self,
        snapshot: &FileRenameInputSnapshot,
        site: &'static str,
    ) -> Result<Option<FileRenamePublicationGuards<'a>>, FileRenamePublicationWait> {
        let inputs = self
            .analyser_inputs_gate
            .try_read()
            .map_err(|_| FileRenamePublicationWait::Inputs)?;
        let factory = self
            .class_factory_generation
            .try_read()
            .map_err(|_| FileRenamePublicationWait::Factory)?;
        let sidecar_current = self
            .fresh_analysis_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .published_revision_is_current(&snapshot.sidecar_revision);
        if self.diag_inputs_epoch() != snapshot.analyser_inputs_epoch
            || *factory != snapshot.class_factory_generation
            || !sidecar_current
        {
            return Ok(None);
        }
        let documents = self
            .documents
            .try_lock(site)
            .ok_or(FileRenamePublicationWait::Documents)?;
        Ok(Some(FileRenamePublicationGuards {
            _inputs: inputs,
            _factory: factory,
            documents,
        }))
    }

    pub(super) async fn wait_for_file_rename_publication(
        &self,
        wait: FileRenamePublicationWait,
        site: &'static str,
    ) {
        match wait {
            FileRenamePublicationWait::Inputs => drop(self.analyser_inputs_gate.read().await),
            FileRenamePublicationWait::Factory => drop(self.class_factory_generation.read().await),
            FileRenamePublicationWait::Documents => drop(self.documents.lock(site).await),
            FileRenamePublicationWait::IndexRead => drop(self.workspace_index.read().await),
            FileRenamePublicationWait::IndexWrite => drop(self.workspace_index.write().await),
        }
        crate::rt::yield_now().await;
    }

    pub(in super::super) async fn refresh_file_rename_source(
        &self,
        owner: &str,
        current_sources: &mut FileRenameSources,
        generation: &mut u64,
        inputs: &FileRenameInputSnapshot,
    ) -> jsonrpc::Result<ControlFlow<()>> {
        let Ok(uri) = owner.parse::<Uri>() else {
            return Ok(ControlFlow::Continue(()));
        };
        if uri_norm::file_action_path(&uri).is_none() {
            return Ok(ControlFlow::Continue(()));
        }
        let Some(document) = self.read_document(&uri).await? else {
            return Ok(ControlFlow::Break(()));
        };
        let analysis = self
            .analysis_for(&uri, document.text.clone(), document.dialect.clone())
            .await?;
        let Some(config) = analysis.body_lexer_config else {
            return Ok(ControlFlow::Break(()));
        };
        if !analysis.matches_original_source_image(
            &tcl_lexer::SourceImage::document(document.text.as_ref()),
            config,
        ) {
            return Ok(ControlFlow::Break(()));
        }
        let Some(current) = self.read_document(&uri).await? else {
            return Ok(ControlFlow::Break(()));
        };
        if current.text != document.text
            || current.dialect != document.dialect
            || current.revision != document.revision
            || current.version != document.version
        {
            return Ok(ControlFlow::Break(()));
        }
        self.publish_file_rename_source(
            FileRenameSourcePublication {
                uri,
                document,
                analysis,
            },
            current_sources,
            generation,
            inputs,
        )
        .await
    }

    pub(in super::super) async fn publish_file_rename_source(
        &self,
        source: FileRenameSourcePublication,
        current_sources: &mut FileRenameSources,
        generation: &mut u64,
        inputs: &FileRenameInputSnapshot,
    ) -> jsonrpc::Result<ControlFlow<()>> {
        let FileRenameSourcePublication {
            uri,
            document,
            analysis,
        } = source;
        let site = "will_rename_source_refresh";
        loop {
            let guards = match self.try_file_rename_publication_guards(inputs, site) {
                Ok(Some(guards)) => guards,
                Ok(None) => return Ok(ControlFlow::Break(())),
                Err(wait) => {
                    self.wait_for_file_rename_publication(wait, site).await;
                    continue;
                }
            };
            if guards
                .documents
                .get(&uri)
                .map_or(document.version.is_some(), |current| {
                    let current = current.clone().normalised_for_analysis();
                    current.backing_file_deleted
                        || current.text != document.text
                        || current.dialect != document.dialect
                        || current.revision != document.revision
                        || current.version != document.version
                })
            {
                return Ok(ControlFlow::Break(()));
            }
            let Ok(mut index) = self.workspace_index.try_write() else {
                drop(guards);
                self.wait_for_file_rename_publication(FileRenamePublicationWait::IndexWrite, site)
                    .await;
                continue;
            };
            if index.generation() != *generation {
                return Ok(ControlFlow::Break(()));
            }
            index.replace_document(uri.as_str(), &analysis);
            *generation = index.generation();
            current_sources.insert(uri, (document, analysis));
            return Ok(ControlFlow::Continue(()));
        }
    }

    pub(in super::super) async fn hold_file_rename_edits(
        &self,
        by_dep: HashMap<Uri, Vec<core_file_ops::RenameEdit>>,
        current_sources: &mut FileRenameSources,
    ) -> jsonrpc::Result<Option<Vec<HeldFileRename>>> {
        let mut held = Vec::new();
        for (uri, mut edits) in by_dep {
            edits.sort_by_key(|edit| (edit.span.start(), edit.span.end()));
            edits.dedup();
            if edits
                .windows(2)
                .any(|pair| pair[0].span.end() > pair[1].span.start())
            {
                return Ok(None);
            }
            let Some((document, analysis)) = current_sources.remove(&uri) else {
                return Ok(None);
            };
            {
                let index = self.workspace_index.read().await;
                if !edits.iter().all(|edit| {
                    uri_norm::file_action_path(&uri)
                        .is_some_and(|path| edit.matches_current(&index, &analysis, &path))
                }) {
                    return Ok(None);
                }
            }
            held.push((uri, document, analysis, edits));
        }
        Ok(Some(held))
    }

    fn file_rename_row_is_unavailable(
        row: &HeldFileRename,
        documents: &HashMap<Uri, DocumentState>,
        index: &core_workspace_index::WorkspaceIndex,
    ) -> bool {
        let (uri, document, analysis, edits) = row;
        documents
            .get(uri)
            .map_or(document.version.is_some(), |current| {
                let current = current.clone().normalised_for_analysis();
                current.backing_file_deleted
                    || current.text != document.text
                    || current.dialect != document.dialect
                    || current.revision != document.revision
                    || current.version != document.version
            })
            || !edits.iter().all(|edit| {
                uri_norm::file_action_path(uri)
                    .is_some_and(|path| edit.matches_current(index, analysis, &path))
            })
    }

    pub(in super::super) async fn file_rename_edit_if_current(
        &self,
        held: Vec<HeldFileRename>,
        generation: u64,
        inputs: &FileRenameInputSnapshot,
    ) -> Option<WorkspaceEdit> {
        let site = "will_rename_source_currency";
        loop {
            let guards = match self.try_file_rename_publication_guards(inputs, site) {
                Ok(Some(guards)) => guards,
                Ok(None) => return None,
                Err(wait) => {
                    self.wait_for_file_rename_publication(wait, site).await;
                    continue;
                }
            };
            let Ok(index) = self.workspace_index.try_read() else {
                drop(guards);
                self.wait_for_file_rename_publication(FileRenamePublicationWait::IndexRead, site)
                    .await;
                continue;
            };
            if index.generation() != generation
                || held
                    .iter()
                    .any(|row| Self::file_rename_row_is_unavailable(row, &guards.documents, &index))
            {
                return None;
            }
            return Some(Self::file_rename_workspace_edit(held));
        }
    }

    fn file_rename_workspace_edit(held: Vec<HeldFileRename>) -> WorkspaceEdit {
        let document_changes: Vec<TextDocumentEdit> = held
            .into_iter()
            .map(|(uri, document, _, edits)| TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri,
                    version: document.version,
                },
                edits: edits
                    .into_iter()
                    .map(|edit| {
                        OneOf::Left(TextEdit {
                            range: lift_span(&document.text, &document.line_index, edit.span),
                            new_text: edit.new_text,
                        })
                    })
                    .collect(),
            })
            .collect();
        WorkspaceEdit {
            changes: None,
            document_changes: Some(DocumentChanges::Edits(document_changes)),
            change_annotations: None,
        }
    }
}
