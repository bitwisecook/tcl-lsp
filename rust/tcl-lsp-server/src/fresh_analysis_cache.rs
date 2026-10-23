// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reuse of per-document fresh analyses keyed on their complete input
//! revision (user decision 2026-10-18T16:00Z, `Server108`; closed documents
//! included in a size-bounded LRU, decision 2026-10-19T11:00Z
//! `closed_document_analysis_cache`).
//!
//! A fresh analysis is a pure function of the document's bytes and dialect,
//! its checked path admission (which fixes the analyser's file path), and the
//! configured analyser inputs the driver resolved under one stable
//! analyser-inputs epoch and class-factory generation. A sidecar stub file is
//! the one disk input the analyser reads on its own; a sidecar change clears
//! the cache ([`FreshAnalysisCache::clear`]). Every hit is therefore the
//! analysis a recomputation would produce, so read-only requests re-analyse
//! only the documents whose input revision changed. A hit still passes the
//! same path-admission checks a fresh run does before it is handed out.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use tcl_compiler::analyser::NonAsciiMode;
use tower_lsp_server::ls_types::Uri;

use super::{DocumentPathAdmission, FreshAnalysisSeed, ResourceAnalyserInputs};

/// Entry cap. Open documents are a handful; the rest are closed documents a
/// workspace query touched.
const MAX_ENTRIES: usize = 512;

/// Cap on the retained source bytes, so a few very large files cannot pin an
/// unbounded amount of memory through their analyses.
const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;

/// The configured analyser inputs a [`super::FreshAnalysisDriver`] was built
/// from, other than the epoch/generation stamps it also carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FreshAnalyserInputs {
    pub(super) disabled: HashSet<String>,
    pub(super) non_ascii_mode: NonAsciiMode,
    pub(super) extra_commands: HashSet<String>,
    pub(super) pack_key: u64,
    pub(super) resource: ResourceAnalyserInputs,
}

/// Everything a fresh analysis of one document depends on.
#[derive(Debug, Clone)]
pub(super) struct FreshAnalysisKey {
    pub(super) text: Arc<str>,
    pub(super) dialect: String,
    pub(super) path_admission: Arc<DocumentPathAdmission>,
    pub(super) analyser_inputs_epoch: u64,
    pub(super) class_factory_generation: u64,
    pub(super) inputs: FreshAnalyserInputs,
}

impl PartialEq for FreshAnalysisKey {
    fn eq(&self, other: &Self) -> bool {
        self.analyser_inputs_epoch == other.analyser_inputs_epoch
            && self.class_factory_generation == other.class_factory_generation
            && self.dialect == other.dialect
            && (Arc::ptr_eq(&self.text, &other.text) || self.text == other.text)
            && self.path_admission == other.path_admission
            && self.inputs == other.inputs
    }
}

struct Entry {
    key: FreshAnalysisKey,
    seed: FreshAnalysisSeed,
}

/// Bounded least-recently-used map from a document URI to its latest fresh
/// analysis and the input revision it was computed from.
#[derive(Default)]
pub(super) struct FreshAnalysisCache {
    entries: HashMap<Uri, Entry>,
    /// Least recently used first.
    order: VecDeque<Uri>,
    source_bytes: usize,
}

impl FreshAnalysisCache {
    /// The cached analysis of `uri` when it was computed from exactly `key`.
    pub(super) fn get(&mut self, uri: &Uri, key: &FreshAnalysisKey) -> Option<FreshAnalysisSeed> {
        let entry = self.entries.get(uri)?;
        if entry.key != *key {
            return None;
        }
        let seed = entry.seed.clone();
        self.touch(uri);
        Some(seed)
    }

    /// Record `seed` as `uri`'s analysis for `key`, replacing any older
    /// revision of the same document.
    pub(super) fn insert(&mut self, uri: &Uri, key: FreshAnalysisKey, seed: FreshAnalysisSeed) {
        let bytes = key.text.len();
        if bytes > MAX_SOURCE_BYTES {
            self.remove(uri);
            return;
        }
        self.remove(uri);
        self.source_bytes += bytes;
        self.entries.insert(uri.clone(), Entry { key, seed });
        self.order.push_back(uri.clone());
        while self.entries.len() > MAX_ENTRIES || self.source_bytes > MAX_SOURCE_BYTES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                self.source_bytes -= entry.key.text.len();
            }
        }
    }

    /// Drop `uri`'s entry, if any.
    pub(super) fn remove(&mut self, uri: &Uri) {
        if let Some(entry) = self.entries.remove(uri) {
            self.source_bytes -= entry.key.text.len();
            self.order.retain(|queued| queued != uri);
        }
    }

    /// Drop every entry: an analyser input outside the key changed.
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.source_bytes = 0;
    }

    fn touch(&mut self, uri: &Uri) {
        if let Some(position) = self.order.iter().position(|queued| queued == uri)
            && let Some(queued) = self.order.remove(position)
        {
            self.order.push_back(queued);
        }
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;
    use std::sync::Arc;

    use tower_lsp_server::ls_types::Uri;

    /// Server108: an unchanged input revision is not analysed again; a
    /// changed text or analyser-inputs epoch is.
    #[tokio::test]
    async fn fresh_analysis_is_reused_only_for_an_unchanged_input_revision() {
        let backend = crate::tests::test_backend();
        let uri = Uri::from_str("file:///workspace/fresh-cache.tcl").unwrap();
        let source: Arc<str> = Arc::from("proc a {} {}\na\n");
        let first = backend
            .fresh_analysis_seed(&uri, Arc::clone(&source), "tcl8.6".to_owned())
            .await
            .unwrap();
        let again = backend
            .fresh_analysis_seed(&uri, Arc::from("proc a {} {}\na\n"), "tcl8.6".to_owned())
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&first.analysis, &again.analysis));
        let edited = backend
            .fresh_analysis_seed(&uri, Arc::from("proc b {} {}\nb\n"), "tcl8.6".to_owned())
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(&first.analysis, &edited.analysis));
        let other_dialect = backend
            .fresh_analysis_seed(&uri, Arc::from("proc b {} {}\nb\n"), "tcl9.0".to_owned())
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(&edited.analysis, &other_dialect.analysis));
        backend.invalidate_diag_inputs();
        let reconfigured = backend
            .fresh_analysis_seed(&uri, Arc::from("proc b {} {}\nb\n"), "tcl9.0".to_owned())
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(
            &other_dialect.analysis,
            &reconfigured.analysis
        ));
        assert_eq!(
            reconfigured.analyser_inputs_epoch,
            backend.diag_inputs_epoch()
        );
        assert_eq!(
            backend
                .fresh_analysis_cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .len(),
            1
        );
    }
}
