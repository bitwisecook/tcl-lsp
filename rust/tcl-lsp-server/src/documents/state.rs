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

//! Constructors, revision bumps and publication checks of `DocumentState`.

use super::path_admission::DocumentPathAdmission;
use super::{Arc, Backend, DocumentPublication, DocumentState, Uri};

impl DocumentState {
    pub(in super::super) fn new(text: String, dialect: String) -> Self {
        let line_index = tcl_lexer::LineIndex::new_lsp(&text);
        let has_bare_cr = tcl_lexer::LineIndex::contains_bare_cr(&text);
        Self {
            path_admission: None,
            orphan_command_dependencies: None,
            text: text.into(),
            raw_text: None,
            saved_source_bytes: None,
            line_index,
            has_bare_cr,
            decode_report: None,
            dialect,
            language_id: String::new(),
            revision: 0,
            version: None,
            backing_file_deleted: false,
            publication: DocumentPublication::Indexed,
            dialect_resolution_generation: 0,
            resolved_dialect_generation: 0,
        }
    }

    pub(in super::super) fn with_version(text: String, dialect: String, version: i32) -> Self {
        let line_index = tcl_lexer::LineIndex::new_lsp(&text);
        let has_bare_cr = tcl_lexer::LineIndex::contains_bare_cr(&text);
        Self {
            path_admission: None,
            orphan_command_dependencies: None,
            text: text.into(),
            raw_text: None,
            saved_source_bytes: None,
            line_index,
            has_bare_cr,
            decode_report: None,
            dialect,
            language_id: String::new(),
            revision: 0,
            version: Some(version),
            backing_file_deleted: false,
            publication: DocumentPublication::Indexed,
            dialect_resolution_generation: 0,
            resolved_dialect_generation: 0,
        }
    }

    pub(in super::super) fn with_path_admission(
        mut self,
        admission: Arc<DocumentPathAdmission>,
    ) -> Self {
        self.path_admission = Some(admission);
        self
    }

    /// Record the LSP `languageId` the document was opened with (builder
    /// form, so the existing constructors stay two-argument).
    pub(in super::super) fn with_language_id(mut self, language_id: String) -> Self {
        self.language_id = language_id;
        self
    }

    pub(in super::super) fn with_publication_pending(mut self) -> Self {
        self.publication = DocumentPublication::Pending;
        self
    }

    /// The pre-normalisation Unicode text — `text` unless this snapshot has
    /// been normalised for analysis, in which case it is in `raw_text`.
    /// For a closed file this is decoded text, not original file bytes.
    pub(in super::super) fn raw(&self) -> &str {
        self.raw_text.as_deref().unwrap_or(&self.text)
    }

    pub(in super::super) fn saved_source_inputs<'a>(
        snapshots: impl IntoIterator<Item = &'a Self>,
    ) -> Vec<(&'a DocumentPathAdmission, &'a [u8])> {
        let mut retained: Vec<(&Arc<DocumentPathAdmission>, &Arc<[u8]>)> = Vec::new();
        for snapshot in snapshots {
            let (Some(admission), Some(bytes)) = (
                snapshot.path_admission.as_ref(),
                snapshot.saved_source_bytes.as_ref(),
            ) else {
                continue;
            };
            if !retained.iter().any(|(selected, image)| {
                Arc::ptr_eq(selected, admission) && Arc::ptr_eq(image, bytes)
            }) {
                retained.push((admission, bytes));
            }
        }
        retained
            .into_iter()
            .map(|(admission, bytes)| (admission.as_ref(), bytes.as_ref()))
            .collect()
    }

    /// Rewrite `text` into its analysis form, keeping the client's bytes in
    /// `raw_text`.  A no-op for the overwhelmingly common LF / CRLF document
    /// (`has_bare_cr` is already known, so this costs one branch); only an
    /// old-Mac / mixed buffer allocates.
    pub(in super::super) fn normalised_for_analysis(mut self) -> Self {
        if !self.has_bare_cr || self.raw_text.is_some() {
            return self;
        }
        let normalised: Arc<str> = tcl_lexer::normalise_lone_cr(&self.text).into_owned().into();
        self.raw_text = Some(std::mem::replace(&mut self.text, normalised));
        self
    }

    pub(in super::super) fn bump_revision(&mut self, version: i32) {
        self.revision = self.revision.saturating_add(1);
        self.version = Some(version);
        self.publication = DocumentPublication::Pending;
    }

    /// Invalidate derived state without changing the editor's text version.
    /// A dialect-only change is still a new analysis generation: diagnostics
    /// and index workers captured under the previous dialect must fail their
    /// revision currency check before they can publish.
    pub(in super::super) fn bump_analysis_generation(&mut self) {
        self.revision = self.revision.saturating_add(1);
        self.publication = DocumentPublication::Pending;
    }

    pub(in super::super) fn matches_open_publication(
        &self,
        text: &str,
        dialect: &str,
        version: i32,
    ) -> bool {
        self.version == Some(version)
            && self.text.as_ref() == text
            && self.dialect.as_str() == dialect
            && !self.backing_file_deleted
    }

    pub(in super::super) fn matches_live_publication(
        &self,
        text: &str,
        dialect: &str,
        revision: u64,
    ) -> bool {
        self.revision == revision
            && self.text.as_ref() == text
            && self.dialect.as_str() == dialect
            && !self.backing_file_deleted
    }

    /// Whether these bytes still identify this live revision, including an
    /// open buffer whose backing path has been deleted. Orphaned buffers keep
    /// serving local providers even though they have no workspace-index view.
    pub(in super::super) fn matches_live_source_publication(
        &self,
        text: &str,
        dialect: &str,
        revision: u64,
    ) -> bool {
        self.revision == revision && self.text.as_ref() == text && self.dialect.as_str() == dialect
    }

    pub(in super::super) fn mark_dialect_resolution_dirty(&mut self) {
        self.dialect_resolution_generation = self.dialect_resolution_generation.saturating_add(1);
    }

    pub(in super::super) fn dialect_resolution_pending(&self) -> bool {
        self.dialect_resolution_generation != self.resolved_dialect_generation
    }
}

impl Backend {
    pub(in super::super) async fn attach_open_disk_decode_report(
        &self,
        uri: &Uri,
        text: &str,
        version: i32,
    ) {
        let decode_report = Self::decode_report_for_matching_disk_text(
            &self.store,
            uri,
            text,
            self.declared_script_encoding(),
        )
        .await;
        if let Some(decode_report) = decode_report {
            let mut docs = self.documents.lock("did_open").await;
            if let Some(doc) = docs.get_mut(uri)
                && doc.version == Some(version)
                && doc.text.as_ref() == text
            {
                doc.decode_report = Some(decode_report);
            }
        }
    }
}
