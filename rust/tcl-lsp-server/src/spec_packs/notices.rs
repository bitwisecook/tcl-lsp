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

//! Pack load notices published as diagnostics on the pack files.

use std::collections::HashMap;
use std::sync::Arc;

use tcl_lsp_core::code_actions as core_code_actions;
use tower_lsp_server::ls_types::{MessageType, Uri};

use crate::diagnostics::{
    PackNoticeContext, PackNoticeEntry, PackNoticeSourceOwner, PreparedPackNotice,
};
use crate::documents::{DocumentSourceResult, canonical_file_uri};
use crate::{Backend, studio_source};

impl Backend {
    /// Opt-in captured notice evidence before URI/source publication filters.
    /// This reports no URI admission, live source currency or release verdict.
    async fn trace_raw_pack_notices(
        &self,
        packs: &tcl_spectcl::PackSet,
        collisions: &[tcl_spectcl::PackNotice],
        seq: u64,
    ) {
        if !matches!(option_env!("TCL_LSP_TRACE_PACK_NOTICES"), Some("1")) {
            return;
        }
        let bounded = |text: &str| {
            let mut characters = text.chars();
            let prefix = characters.by_ref().take(512).collect::<String>();
            (prefix, characters.next().is_some())
        };
        // Ten total rows per reload also bounds every individual mount. The
        // original PackNotice and exact PackSet file membership own context;
        // no diagnostic URI or reconstructed command name supplies identity.
        let notices = packs
            .notices
            .iter()
            .map(|notice| ("load_notice", notice))
            .chain(collisions.iter().map(|notice| ("collision", notice)));
        for (index, (kind, notice)) in notices.take(10).enumerate() {
            let (path, path_truncated) = bounded(&format!("{:?}", notice.path.to_string_lossy()));
            let (context, context_truncated) = bounded(&notice.context);
            let (message, message_truncated) = bounded(&notice.message);
            let pack_names = packs
                .packs
                .iter()
                .filter(|pack| pack.files.contains(&notice.path))
                .take(10)
                .map(|pack| bounded(&pack.name).0)
                .collect::<Vec<_>>();
            let row = serde_json::json!({
                "reload_sequence": seq.to_string(), "pack_set_key": packs.key.to_string(),
                "kind": kind, "notice_index": index, "notice_limit": 10,
                "load_notice_count": packs.notices.len(), "collision_count": collisions.len(),
                "notice_list_truncated": packs.notices.len().saturating_add(collisions.len()) > 10,
                "pack_names": pack_names, "path_debug": path, "path_truncated": path_truncated,
                "line": notice.line, "severity": format!("{:?}", notice.severity),
                "context": context, "context_truncated": context_truncated,
                "message": message, "message_truncated": message_truncated,
            });
            // JSON serialization escapes NUL and control characters. Each
            // descriptive string is at most 512 Unicode scalar values.
            self.client
                .log_message(MessageType::INFO, format!("[pack-notice-trace] {row}"))
                .await;
        }
    }

    /// Publish every pack's load notices — plus the `collisions` the caller
    /// computed against the plain registry — as diagnostics on the pack file
    /// they came from, and clear the ones a previous load left on files that
    /// are now clean.
    ///
    /// A notice is not a compile error — the pack loaded, minus the thing the
    /// notice names — so this is the natural surface for it: the author sees
    /// "unknown property `arty` dropped" on the line they typed it, in the
    /// editor they typed it in, exactly as they would a diagnostic on Tcl code.
    ///
    /// # An *open* pack file
    ///
    /// `.tclspec` is in `TCL_SOURCE_EXTENSIONS` (a pack is one Tcl script), so
    /// an open pack buffer is also an analysed document, and `publishDiagnostics`
    /// replaces a URI's whole set. So this is not a second publisher: it hands
    /// the notices to [`DiagnosticPublisher::set_pack_notices`], which unions
    /// them into every set published for that URI — the push path, the fast
    /// tier, and the pull cache alike. A URI nobody has open is then published
    /// straight from here (its analysed layer is empty, so the union is the
    /// notices); an open one is rescheduled instead, so the set that lands
    /// carries the analyser's findings *and* the loader's rather than
    /// whichever producer ran last.
    #[allow(clippy::too_many_lines)] // one request handler, step by step
    pub(super) async fn publish_spec_pack_notices(
        &self,
        packs: &Arc<tcl_spectcl::PackSet>,
        collisions: &[tcl_spectcl::PackNotice],
        seq: u64,
        sources: super::PackReloadSourceViews<'_>,
    ) -> DocumentSourceResult<()> {
        let super::PackReloadSourceViews {
            loader_inputs,
            studio_snapshot,
            overlays,
            read,
        } = sources;
        self.trace_raw_pack_notices(packs, collisions, seq).await;
        let mut by_uri: HashMap<Uri, Vec<PackNoticeEntry>> = HashMap::new();
        for notice in packs.notices.iter().chain(collisions) {
            let Some(prepared) = self
                .prepare_spec_pack_notice(notice, loader_inputs, studio_snapshot)
                .await?
            else {
                continue;
            };
            by_uri
                .entry(prepared.uri)
                .or_default()
                .push(PackNoticeEntry {
                    diagnostic: prepared.diagnostic,
                    source: prepared.source,
                    owner: prepared.owner,
                });
        }

        // The settings that promise *no* diagnostics for a file —
        // `tclLsp.features.diagnostics = false` and a `tclLsp.diagnostics.exclude`
        // match — are honoured here, where the notice layer is set,
        // rather than at the two sites that union it in
        // ([`DiagnosticPublisher::with_pack_notices`], reached from both the
        // pull report and every push). A layer that does not stand cannot be
        // unioned back by either, so the pulled report and the pushed set agree
        // by construction instead of by two checks that have to be kept in
        // step; gating the unions instead leaves whichever site is missed
        // republishing the squiggles the switch has just cleared.
        //
        // A URI a setting now silences drops out of `by_uri`, joins the stale
        // set below and has its standing notices cleared on this pass; either
        // setting moving reloads packs (`ReloadTrigger::Config`), so the flip
        // itself is what runs it, and flipping back restores them the same way.
        let candidates: Vec<Uri> = by_uri.keys().cloned().collect();
        for uri in candidates {
            if !self.feature_enabled("diagnostics", &uri).await
                || self.diagnostics_excluded(&uri).await
            {
                by_uri.remove(&uri);
            }
        }

        // A file that had notices last time and does not now must be cleared,
        // or a fixed pack keeps its badge until the editor restarts — which
        // `set_pack_notices` reports as a touched URI just as a new notice is.
        let notices_for_publish = by_uri.clone();
        let registry_owners = overlays
            .iter()
            .map(|(_, registry)| Arc::clone(registry))
            .collect();
        let touched = self.diagnostic_publisher.set_pack_notices(
            seq,
            Arc::clone(packs),
            registry_owners,
            by_uri,
            read.files.to_vec(),
            read.workspace_trust,
        );
        // Open documents go back through their current report owner. Closed
        // files rejoin the installed pack publication and provider source at
        // the synchronous enqueue boundary below.
        let session_dialect = self.session_dialect().await;
        let session_profile = tcl_lsp_core::profile_for_dialect(&session_dialect);
        let registry = overlays
            .iter()
            .find(|(profile, _)| profile.name == session_profile.name)
            .map(|(_, registry)| Arc::clone(registry));
        for uri in touched {
            let docs = self.documents.lock("publish_spec_pack_notices").await;
            if let Some(document) = docs.get(&uri) {
                let dialect = document.dialect.clone();
                drop(docs);
                self.reschedule_diagnostics(uri, dialect).await;
                continue;
            }
            let Some(registry) = registry.as_ref() else {
                continue;
            };
            // Keep both the closed-document decision and the installed pack
            // generation through the final provider/source rejoin and enqueue.
            let published = self.spec_packs.lock().await;
            if published.seq != seq || !Arc::ptr_eq(&published.packs, packs) {
                continue;
            }
            let receipt = notices_for_publish
                .get(&uri)
                .and_then(|entries| entries.first())
                .map_or_else(
                    || {
                        self.diagnostic_publisher.enqueue_assembled(
                            uri.clone(),
                            Vec::new(),
                            None,
                            false,
                            false,
                        )
                    },
                    |entry| {
                        self.diagnostic_publisher.with_pack_notices_during(
                            PackNoticeContext {
                                uri: &uri,
                                source: &entry.source,
                                pack_seq: seq,
                                packs,
                                registry,
                                store: self.store.as_ref(),
                            },
                            Vec::new(),
                            |diagnostics| {
                                self.diagnostic_publisher.enqueue_assembled(
                                    uri.clone(),
                                    diagnostics,
                                    None,
                                    false,
                                    false,
                                )
                            },
                        )
                    },
                );
            drop(published);
            drop(docs);
            let _ = receipt.await;
        }
        Ok(())
    }

    async fn prepare_spec_pack_notice(
        &self,
        notice: &tcl_spectcl::PackNotice,
        loader_inputs: &tcl_spectcl::pack::LoadedSourceInputs,
        studio_snapshot: &studio_source::StudioSourceSnapshot,
    ) -> DocumentSourceResult<Option<PreparedPackNotice>> {
        let Some(uri) = canonical_file_uri(&notice.path) else {
            return Ok(None);
        };
        let paths = [notice.path.clone()];
        let (source_owner, source) = if let Some(mut selected) = loader_inputs.selected(&paths) {
            let Some((origin, bytes)) = selected.pop() else {
                return Ok(None);
            };
            if self.store.validate_source_path_origin(origin).is_err() {
                return Ok(None);
            }
            let Ok(source) = std::str::from_utf8(bytes) else {
                return Ok(None);
            };
            if !matches!(self.store.read_to_string(&notice.path), Ok(current) if current == source)
            {
                return Ok(None);
            }
            (
                PackNoticeSourceOwner::Saved(origin.clone()),
                Arc::<str>::from(source),
            )
        } else {
            let Some((admission, source)) = studio_snapshot.source_for_saved_path(&notice.path)
            else {
                return Ok(None);
            };
            if admission.check_current(self.store.as_ref()).is_err()
                || !matches!(self.store.read_to_string(&notice.path), Ok(current) if current.as_str() == source.as_ref())
            {
                return Ok(None);
            }
            (PackNoticeSourceOwner::Studio(admission), source)
        };
        if let Some(subject) = &notice.subject
            && !subject
                .image()
                .try_text()
                .is_ok_and(|image| image == source.as_ref())
        {
            return Ok(None);
        }
        let notice_data = if let Some(subject) = &notice.subject {
            if let Ok(source) = subject.image().try_text() {
                let analysis = self
                    .analyse_with_workspace_classes(
                        &uri,
                        source,
                        tcl_lsp_core::profile_for_dialect("spectcl"),
                    )
                    .await?;
                core_code_actions::ContextDiagnosticData::from_spec_pack_notice(subject, &analysis)
                    .map(|data| data.to_value())
            } else {
                None
            }
        } else {
            None
        };
        let diagnostic = pack_notice_diagnostic(notice, notice_data);
        Ok(Some(PreparedPackNotice {
            uri,
            diagnostic,
            source,
            owner: source_owner,
            _paths: paths,
        }))
    }
}

/// The wire diagnostic for a loader notice, positioned by its own subject.
pub(in super::super) fn pack_notice_diagnostic(
    notice: &tcl_spectcl::PackNotice,
    data: Option<serde_json::Value>,
) -> tower_lsp_server::ls_types::Diagnostic {
    use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity, NumberOrString};

    let severity = match notice.severity {
        tcl_spectcl::pack::Severity::Warning => DiagnosticSeverity::WARNING,
        tcl_spectcl::pack::Severity::Information => DiagnosticSeverity::INFORMATION,
    };
    Diagnostic {
        range: whole_line_range(notice),
        severity: Some(severity),
        code: Some(NumberOrString::String(
            core_code_actions::SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
        )),
        source: Some("tcl-lsp".to_owned()),
        message: format!("{}: {}", notice.context, notice.message),
        data,
        ..Diagnostic::default()
    }
}

/// Whole-line presentation from the loader's retained source subject.
/// Notices without an original line subject use the whole-file anchor.
pub(in super::super) fn whole_line_range(
    notice: &tcl_spectcl::PackNotice,
) -> tower_lsp_server::ls_types::Range {
    use tower_lsp_server::ls_types::{Position, Range};
    let Some(subject) = notice.subject.as_ref() else {
        return Range::default();
    };
    let Ok(source) = subject.image().try_text() else {
        return Range::default();
    };
    if notice.line == 0 || notice.line != subject.line() {
        return Range::default();
    }
    let index = tcl_lexer::LineIndex::new_lsp(source);
    let line = index.position_at_utf16(subject.span().start(), source).line;
    let end = index.offset_at_utf16(line, tcl_lexer::Utf16Col::new(u32::MAX), source);
    let end = index.position_at_utf16(end, source);
    Range {
        start: Position::new(line, 0),
        end: Position::new(end.line, end.character.get()),
    }
}
