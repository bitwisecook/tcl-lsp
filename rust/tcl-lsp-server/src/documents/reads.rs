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

//! Snapshot reads of a document at the readiness a provider needs.

use super::{DocumentReadiness, DocumentSourceResult, DocumentSourceUnavailable};
use crate::uri_norm::DocumentPath as _;
use crate::{Arc, Backend, DocumentState, Path, Uri, vfs};

impl Backend {
    /// A snapshot of `url`'s text, **normalised for analysis**: every lone
    /// `\r` in the open buffer (or the on-disk fallback) is rewritten to `\n`
    /// before the snapshot is handed out, with the client's own bytes kept in
    /// [`DocumentState::raw`].
    ///
    /// This is the single choke point between the shadow buffer and every
    /// feature provider, so the ~80 `LineIndex::new` sites in `tcl-lsp-core`
    /// and the lexer's `\n`-only line model agree with the client's
    /// `\n`/`\r\n`/lone-`\r` model without any of them changing — and so the
    /// analysis matches what `tclsh` would parse from the same file, where the
    /// `-translation auto` channel makes a lone `\r` a command terminator.
    /// Only the formatter and the W118 line-ending lint read
    /// [`DocumentState::raw`] instead.
    pub(in super::super) fn source_read_is_absent(
        store: &dyn vfs::SourceStore,
        path: &Path,
        cause: &std::io::Error,
    ) -> bool {
        cause.kind() == std::io::ErrorKind::NotFound
            || cause.kind() == std::io::ErrorKind::NotADirectory
            || cause.kind() == std::io::ErrorKind::IsADirectory
            || store.metadata(path).is_ok_and(|metadata| metadata.is_dir)
    }

    pub(in super::super) async fn read_document_at(
        &self,
        url: &Uri,
        readiness: DocumentReadiness,
    ) -> DocumentSourceResult<Option<DocumentState>> {
        let path_admission = self.document_path_admission(url).await?;
        self.edits_settled().await;
        // A document-sync notification publishes the shadow buffer first and
        // shared derived stores second so it can release the global request
        // barrier before a contended store. Preserve revision
        // consistency by waiting only requests for this URI until the store
        // tier their provider needs has caught up.
        loop {
            let advanced = self.live_publication_advanced.notified();
            tokio::pin!(advanced);
            advanced.as_mut().enable();
            let doc = {
                let docs = self.documents.lock("read_document").await;
                docs.get(url).cloned()
            };
            match doc {
                Some(doc) if readiness.is_ready(&doc) => {
                    let admission = doc.path_admission.as_ref().unwrap_or(&path_admission);
                    admission.check_for(url, self.store.as_ref())?;
                    return Ok(Some(
                        doc.clone()
                            .with_path_admission(Arc::clone(admission))
                            .normalised_for_analysis(),
                    ));
                }
                Some(_) => {
                    crate::transport_liveness::await_outside_request_admission(advanced).await;
                }
                None => break,
            }
        }
        // On-disk fallback: files the folder scan indexed but the
        // editor hasn't opened aren't in the open-document map.
        // Read them from disk so cross-document span→range
        // resolution (references / rename / call-hierarchy) can
        // reach their sources.  Non-`file://` URLs and unreadable
        // paths have no fallback read. The actual selected provider path moves
        // into the worker; URI/provider refusal remains distinct from absence.
        let Some(path) = path_admission.path().map(Path::to_path_buf) else {
            return Ok(None);
        };
        // Through the shared decoder, not `read_to_string`: a closed file with
        // one ill-formed byte must still resolve its spans rather than vanish
        // from cross-document navigation.
        let store = Arc::clone(&self.store);
        let selected_path = path.clone();
        let bytes = match crate::rt::spawn_blocking(move || store.read(&path)).await {
            Ok(Ok(source)) => source,
            Ok(Err(cause))
                if Self::source_read_is_absent(self.store.as_ref(), &selected_path, &cause) =>
            {
                path_admission.check_for(url, self.store.as_ref())?;
                return Ok(None);
            }
            Ok(Err(cause)) => {
                return Err(DocumentSourceUnavailable::Read {
                    uri: url.clone(),
                    cause: Arc::new(cause),
                });
            }
            Err(_) => return Err(DocumentSourceUnavailable::Worker { uri: url.clone() }),
        };
        let (text, decode_report) =
            tcl_lsp_core::source_decode::decode_source_as(&bytes, self.declared_script_encoding());
        // The same selected decoded source and original URI drive the
        // closed-file dialect owner used by indexing and diagnostics. Its
        // document override, source detection and configured fallbacks remain
        // independent of the provider path's spelling.
        let dialect = self.dialect_for_closed(url, &text).await;
        path_admission.check_for(url, self.store.as_ref())?;
        let mut document = DocumentState::new(text, dialect).with_path_admission(path_admission);
        // Preserve the decoder's evidence before lone-CR normalisation. Neither
        // a lossy decode nor the normalised analysis view is original bytes;
        // downstream source/input/Native admission remains independent.
        document.decode_report = Some(decode_report);
        document.saved_source_bytes = Some(bytes.into());
        Ok(Some(document.normalised_for_analysis()))
    }

    /// Read a document only after its Salsa source **and** workspace-index view
    /// are current. This is the ordinary provider boundary: navigation and
    /// workspace-aware analysis must not observe the deliberately empty index
    /// slot between `didOpen` retirement and live re-indexing.
    pub(in super::super) async fn read_document(
        &self,
        url: &Uri,
    ) -> DocumentSourceResult<Option<DocumentState>> {
        self.read_document_at(url, DocumentReadiness::Indexed).await
    }

    /// Read the current live bytes once their matching Salsa source exists.
    ///
    /// Formatting, folding, outlines, linked editing, and selection ranges are
    /// document-local operations. They must not inherit the independent
    /// workspace-index barrier, which can remain pending while a cold large
    /// file's first index analysis runs.
    pub(in super::super) async fn read_local_document(
        &self,
        url: &Uri,
    ) -> DocumentSourceResult<Option<DocumentState>> {
        self.read_document_at(url, DocumentReadiness::Salsa).await
    }

    /// Read the current document as soon as its matching Salsa source exists.
    ///
    /// Semantic tokens do not consume the workspace index. Their enriched
    /// Salsa read is explicitly raced against a small budget and falls back to
    /// lexical tokens over this returned text. Making them wait for the
    /// independent whole-file index seed defeats that fast path on cold large
    /// files.
    pub(in super::super) async fn read_document_for_semantic_tokens(
        &self,
        url: &Uri,
    ) -> DocumentSourceResult<Option<DocumentState>> {
        self.read_document_at(url, DocumentReadiness::Salsa).await
    }

    /// Return byte-level decoding evidence only when `text` is exactly the
    /// lossy decoding of `uri`'s current on-disk bytes. LSP synchronisation
    /// transports Unicode text rather than those bytes, so this equality is
    /// the boundary between a fact (safe to diagnose) and a guess (not safe).
    pub(in super::super) async fn decode_report_for_matching_disk_text(
        store: &Arc<dyn vfs::SourceStore>,
        uri: &Uri,
        text: &str,
        encoding: tcl_registry::value_transfer::ScriptEncoding,
    ) -> Option<tcl_lsp_core::source_decode::DecodeReport> {
        let path = uri.document_path()?.into_owned();
        let store = Arc::clone(store);
        let (decoded, report) =
            crate::rt::spawn_blocking(move || store.read_source_as(&path, encoding))
                .await
                .ok()?
                .ok()?;
        (decoded == text).then_some(report)
    }
}
