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

//! Open documents: their state, the map that holds them, the order edits apply in and the reads requests take.

use super::*;

mod content_changes;
mod edit_order;
mod path_admission;
mod per_uri_caches;
mod reads;
mod stall_report;
mod state;
mod store;
mod unpark_watchdog;

#[cfg(test)]
pub(super) use content_changes::apply_content_change;
#[cfg(test)]
pub(super) use content_changes::apply_content_change_indexed;
#[cfg(test)]
pub(super) use content_changes::edit_introduces_bare_cr;
#[cfg(test)]
pub(super) use edit_order::TURN_PHASE_GRANTED;
pub(super) use edit_order::{EDIT_BARRIER_STALL_WARN, EditOrder};
pub use path_admission::canonical_file_uri;
pub(super) use path_admission::{DocumentPathAdmission, SavedSourceBytes, constructed_file_uri};
#[cfg(test)]
pub(super) use stall_report::{
    DOCUMENTS_CONTENTION_SAMPLE_GAP, describe_documents_contention, describe_documents_waiters,
};
#[cfg(test)]
pub(super) use store::WaiterTelemetry;
pub(super) use store::{DocumentStore, DocumentsGuard};
#[cfg(not(target_family = "wasm"))]
#[cfg(test)]
pub(super) use unpark_watchdog::{
    NUDGE_WAITER_THRESHOLD, NudgeTarget, nudge_reason, unpark_watchdog_step,
};
#[cfg(not(target_family = "wasm"))]
pub use unpark_watchdog::{UNPARK_NUDGES, spawn_unpark_watchdog};

#[derive(Debug, Clone)]
pub(super) enum DocumentSourceUnavailable {
    ConditionalAdvice {
        uri: Uri,
    },
    Path {
        uri: Uri,
        cause: uri_norm::DocumentFilePathUnavailable,
    },
    ForeignReceipt {
        expected: Uri,
        // Boxed so `DocumentSourceResult` stays small on every success path.
        actual: Box<Uri>,
    },
    Worker {
        uri: Uri,
    },
    Store {
        uri: Uri,
        cause: vfs::SourceUriPathUnavailable,
    },
    Read {
        uri: Uri,
        cause: Arc<std::io::Error>,
    },
    ScanWorker {
        cause: String,
    },
    PublishedPathUnavailable {
        uri: Uri,
    },
    InvalidStoredUri {
        original: String,
    },
}

pub(super) type DocumentSourceResult<T> = Result<T, DocumentSourceUnavailable>;

impl std::fmt::Display for DocumentSourceUnavailable {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConditionalAdvice { uri } => write!(
                out,
                "conditional source advice for {} is unavailable",
                uri.as_str()
            ),
            Self::Path { uri, cause } => {
                write!(out, "source URI {} is unavailable: {cause:?}", uri.as_str())
            }
            Self::ForeignReceipt { expected, actual } => write!(
                out,
                "source path receipt for {} does not belong to {}",
                actual.as_str(),
                expected.as_str()
            ),
            Self::ScanWorker { cause } => write!(out, "source scan worker is unavailable: {cause}"),
            Self::PublishedPathUnavailable { uri } => write!(
                out,
                "published source path is unavailable for {}",
                uri.as_str()
            ),
            Self::InvalidStoredUri { original } => {
                write!(out, "stored source URI is invalid: {original}")
            }
            Self::Read { uri, cause } => write!(
                out,
                "source read for {} is unavailable: {cause}",
                uri.as_str()
            ),
            Self::Store { uri, cause } => {
                write!(out, "source URI {} is unavailable: {cause}", uri.as_str())
            }
            Self::Worker { uri } => write!(
                out,
                "source analysis worker for {} is unavailable",
                uri.as_str()
            ),
        }
    }
}

impl std::error::Error for DocumentSourceUnavailable {}

impl From<DocumentSourceUnavailable> for jsonrpc::Error {
    fn from(error: DocumentSourceUnavailable) -> Self {
        Self {
            code: if matches!(
                error,
                DocumentSourceUnavailable::Worker { .. }
                    | DocumentSourceUnavailable::ScanWorker { .. }
            ) {
                jsonrpc::ErrorCode::InternalError
            } else {
                jsonrpc::ErrorCode::InvalidParams
            },
            message: error.to_string().into(),
            data: None,
        }
    }
}

impl From<DocumentSourceUnavailable> for core_rename_safety::RenameRefusal {
    fn from(error: DocumentSourceUnavailable) -> Self {
        Self {
            reason: error.to_string(),
            range: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DocumentPublication {
    /// The live buffer is authoritative, but Salsa still names an older source.
    Pending,
    /// Salsa names these live bytes; the workspace index is still catching up.
    Salsa,
    /// Salsa and the workspace-index view both name this live revision.
    Indexed,
}

impl DocumentPublication {
    fn salsa_ready(self) -> bool {
        self != Self::Pending
    }

    pub(super) fn indexed(self) -> bool {
        self == Self::Indexed
    }
}

#[derive(Clone, Copy)]
pub(super) enum DocumentReadiness {
    /// The current live bytes have reached Salsa. This is sufficient for the
    /// semantic-token bounded-enrichment/coarse-fallback pipeline.
    Salsa,
    /// Salsa and every derived cross-document index view are current.
    Indexed,
}

impl DocumentReadiness {
    pub(super) fn is_ready(self, doc: &DocumentState) -> bool {
        match self {
            Self::Salsa => doc.publication.salsa_ready(),
            Self::Indexed => doc.publication.indexed(),
        }
    }
}

/// One waiter's telemetry, aged against a single clock reading for the report.
#[derive(Debug, Clone, Copy)]
pub(super) struct WaiterSnapshot {
    /// Identity of the telemetry registration, stable until deregistration.
    #[cfg(not(target_family = "wasm"))]
    pub(super) telemetry_id: usize,
    pub(super) site: &'static str,
    pub(super) parked_for: std::time::Duration,
    pub(super) polls: u64,
    pub(super) since_last_poll: std::time::Duration,
    pub(super) wakes: u64,
    pub(super) since_last_wake: Option<std::time::Duration>,
    #[cfg(not(target_family = "wasm"))]
    pub(super) task_id: Option<tokio::task::Id>,
}

/// A finished phase of a hold: where it ran, and for how long.
pub(super) type PhaseSpan = (&'static str, std::time::Duration);

/// A released hold as the stall line reports it: its last site, how long ago
/// its hold began, and its longest finished phase.
pub(super) type DepartedHolder = (&'static str, std::time::Duration, Option<PhaseSpan>);

/// A live hold on the open-document map: where it is now, how long it has held
/// in total, and how long it has been at that point.
///
/// The pair is the reading that matters. `held` alone says a hold is long;
/// `in_phase` says whether *this* step is what is long. Without both, a long
/// hold sitting at `publish send` cannot be told apart from a send that has
/// just started after a slow earlier phase.
#[derive(Debug, Clone, Copy)]
pub(super) struct HeldFor {
    pub(super) site: &'static str,
    pub(super) held: std::time::Duration,
    pub(super) in_phase: std::time::Duration,
    /// The longest phase this hold has *finished*, and where — the third
    /// reading, and the one that names an earlier phase a retag has since
    /// relabelled away.
    pub(super) longest_phase: Option<PhaseSpan>,
}

/// One instant's view of [`DocumentStore`] contention, for the stall line.
pub(super) struct DocumentsContention {
    pub(super) held_by: Option<HeldFor>,
    /// The last holder's site, how long ago its hold began, and its longest
    /// finished phase — the same third reading the live case reports, kept for
    /// a hold that has already released.
    pub(super) last: Option<DepartedHolder>,
    pub(super) acquisitions: u64,
}
