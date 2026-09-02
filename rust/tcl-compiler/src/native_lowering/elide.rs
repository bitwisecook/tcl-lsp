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

//! The framing-elision decisions (plan §3.5): which trace barriers are
//! removed, which are guarded, which are kept, and which cells are demoted
//! from a named cell to a slot — every decision with its typed reason.
//!
//! Each decision is taken by one of the [`SemanticOptimisationPassId`]
//! passes the native tier owns, and a disabled pass answers "kept" with the
//! reason `pass-disabled`, so an ablation run shows every framing operation
//! that pass would have removed.

use std::collections::BTreeSet;

use super::cells::{CellPlace, CellStorage};
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::var_escape::ProcEscapeSummary;

/// Why a trace barrier was removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarrierElision {
    /// The module registers no variable trace that can reach the cell, and no
    /// dynamic trace target exists that could name it.
    NoTraceReachesCell,
}

/// Why a trace barrier was kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarrierKept {
    /// The module registers a variable trace on this exact name.
    VariableTraced,
    /// A `trace` call with a computed variable name exists, so the ledger is
    /// unknown for every cell.
    TraceLedgerUnknown,
    /// The `TraceBarrierElision` pass is disabled.
    PassDisabled,
}

/// The recorded decision for one cell access's trace barrier.
///
/// An elided barrier lets the value written or read stay in a native shadow
/// across the access; a kept barrier makes the cell the only holder of the
/// value, so every later read goes back to the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarrierDecision {
    /// Removed, with the proof that made it safe.
    Elided(BarrierElision),
    /// Kept, with the reason.
    Kept(BarrierKept),
}

impl BarrierDecision {
    /// Whether the barrier was removed.
    #[must_use]
    pub const fn is_elided(self) -> bool {
        matches!(self, Self::Elided(_))
    }

    /// Stable Explorer spelling of the decision.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Elided(BarrierElision::NoTraceReachesCell) => "elided:no-trace-reaches-cell",
            Self::Kept(BarrierKept::VariableTraced) => "kept:variable-traced",
            Self::Kept(BarrierKept::TraceLedgerUnknown) => "kept:trace-ledger-unknown",
            Self::Kept(BarrierKept::PassDisabled) => "kept:pass-disabled",
        }
    }
}

/// How a `CellIncr`'s native fast path is guarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IncrGuard {
    /// The trace ledger proves no trace reaches the cell: the native path
    /// needs no runtime test.
    Unguarded,
    /// The ledger is unknown: test the cell's runtime trace bit and take the
    /// runtime `incr` when it is set.
    RuntimeTraceBit,
    /// The cell is traced: always the runtime `incr`.
    RuntimeOnly,
}

impl IncrGuard {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unguarded => "unguarded",
            Self::RuntimeTraceBit => "runtime-trace-bit",
            Self::RuntimeOnly => "runtime-only",
        }
    }
}

/// The module's variable-trace ledger as the lowering sees it.
#[derive(Debug, Clone)]
pub struct TraceLedger<'a> {
    traced: &'a BTreeSet<String>,
    dynamic: bool,
    enabled: bool,
}

impl<'a> TraceLedger<'a> {
    /// Build the ledger from the module's literal trace targets and its
    /// dynamic-target flag, under `config`.
    #[must_use]
    pub fn new(
        traced: &'a BTreeSet<String>,
        dynamic: bool,
        config: SemanticOptimisationConfig,
    ) -> Self {
        Self {
            traced,
            dynamic,
            enabled: config.is_enabled(SemanticOptimisationPassId::TraceBarrierElision),
        }
    }

    /// Decide the trace barrier for an access to `place`.
    #[must_use]
    pub fn decide(&self, place: &CellPlace) -> BarrierDecision {
        if !self.enabled {
            return BarrierDecision::Kept(BarrierKept::PassDisabled);
        }
        if self.dynamic {
            return BarrierDecision::Kept(BarrierKept::TraceLedgerUnknown);
        }
        let base = place.base();
        let traced = self.traced.contains(base)
            || self
                .traced
                .contains(base.strip_prefix("::").unwrap_or(base))
            || self.traced.contains(&format!("::{base}"))
            || matches!(place, CellPlace::Element { .. } if self.traced.contains(&place.spelling()));
        if traced {
            BarrierDecision::Kept(BarrierKept::VariableTraced)
        } else {
            BarrierDecision::Elided(BarrierElision::NoTraceReachesCell)
        }
    }

    /// Decide how an `incr` of `place` guards its native path.
    #[must_use]
    pub fn incr_guard(&self, place: &CellPlace) -> IncrGuard {
        match self.decide(place) {
            BarrierDecision::Elided(_) => IncrGuard::Unguarded,
            BarrierDecision::Kept(BarrierKept::TraceLedgerUnknown) => IncrGuard::RuntimeTraceBit,
            BarrierDecision::Kept(BarrierKept::VariableTraced | BarrierKept::PassDisabled) => {
                IncrGuard::RuntimeOnly
            }
        }
    }
}

/// Why a cell kept (or lost) its named runtime storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellStorageReason {
    /// A top-level variable is a hosted module's observable global.
    TopLevelGlobal,
    /// The variable escapes its procedure (an `upvar` source, a dynamic
    /// barrier, or an unbounded observer).
    EscapesFrame,
    /// The procedure declares or links the name to another frame.
    LinkedScope,
    /// The `CellDemotion` pass is disabled.
    PassDisabled,
    /// Escape analysis proved the variable local and assigned it a slot.
    ProvedLocal,
}

impl CellStorageReason {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TopLevelGlobal => "top-level-global",
            Self::EscapesFrame => "escapes-frame",
            Self::LinkedScope => "linked-scope",
            Self::PassDisabled => "pass-disabled",
            Self::ProvedLocal => "proved-local",
        }
    }
}

/// The cell-storage decision for one variable of a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellDecision {
    /// The storage selected.
    pub storage: CellStorage,
    /// Why.
    pub reason: CellStorageReason,
}

/// Decide the storage of every variable a procedure body touches.
#[derive(Debug, Clone)]
pub struct CellDemotion<'a> {
    escape: Option<&'a ProcEscapeSummary>,
    top_level: bool,
    enabled: bool,
}

impl<'a> CellDemotion<'a> {
    /// The policy for a top-level script: every cell stays a named cell.
    #[must_use]
    pub fn top_level(config: SemanticOptimisationConfig) -> Self {
        Self {
            escape: None,
            top_level: true,
            enabled: config.is_enabled(SemanticOptimisationPassId::CellDemotion),
        }
    }

    /// The policy for a procedure body with its escape summary.
    #[must_use]
    pub fn procedure(
        escape: Option<&'a ProcEscapeSummary>,
        config: SemanticOptimisationConfig,
    ) -> Self {
        Self {
            escape,
            top_level: false,
            enabled: config.is_enabled(SemanticOptimisationPassId::CellDemotion),
        }
    }

    /// The storage decision for `name`.
    #[must_use]
    pub fn decide(&self, name: &str) -> CellDecision {
        if self.top_level {
            return CellDecision {
                storage: CellStorage::Cell,
                reason: CellStorageReason::TopLevelGlobal,
            };
        }
        if !self.enabled {
            return CellDecision {
                storage: CellStorage::Cell,
                reason: CellStorageReason::PassDisabled,
            };
        }
        let Some(escape) = self.escape else {
            return CellDecision {
                storage: CellStorage::Cell,
                reason: CellStorageReason::EscapesFrame,
            };
        };
        if escape.dynamic_barrier() || escape.is_frame(name) || escape.has_fallback() {
            return CellDecision {
                storage: CellStorage::Cell,
                reason: CellStorageReason::EscapesFrame,
            };
        }
        match escape.local_slots.get(name) {
            Some(slot) => CellDecision {
                storage: CellStorage::Slot(*slot),
                reason: CellStorageReason::ProvedLocal,
            },
            None => CellDecision {
                storage: CellStorage::Cell,
                reason: CellStorageReason::EscapesFrame,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled() -> SemanticOptimisationConfig {
        SemanticOptimisationConfig::new()
            .with_enabled(SemanticOptimisationPassId::TraceBarrierElision)
            .with_enabled(SemanticOptimisationPassId::CellDemotion)
    }

    fn named(name: &str) -> CellPlace {
        CellPlace::Named {
            name: name.to_owned(),
        }
    }

    #[test]
    fn a_disabled_pass_keeps_every_barrier_with_its_reason() {
        let traced = BTreeSet::new();
        let ledger = TraceLedger::new(&traced, false, SemanticOptimisationConfig::new());
        assert_eq!(
            ledger.decide(&named("a")),
            BarrierDecision::Kept(BarrierKept::PassDisabled)
        );
        assert_eq!(ledger.incr_guard(&named("a")), IncrGuard::RuntimeOnly);
    }

    #[test]
    fn the_ledger_decides_by_name_and_dynamic_targets() {
        let traced: BTreeSet<String> = ["a".to_owned()].into_iter().collect();
        let ledger = TraceLedger::new(&traced, false, enabled());
        assert_eq!(
            ledger.decide(&named("a")),
            BarrierDecision::Kept(BarrierKept::VariableTraced)
        );
        assert_eq!(
            ledger.decide(&named("::a")),
            BarrierDecision::Kept(BarrierKept::VariableTraced)
        );
        assert_eq!(
            ledger.decide(&named("b")),
            BarrierDecision::Elided(BarrierElision::NoTraceReachesCell)
        );
        assert_eq!(ledger.incr_guard(&named("b")), IncrGuard::Unguarded);
        let unknown = TraceLedger::new(&traced, true, enabled());
        assert_eq!(
            unknown.decide(&named("b")),
            BarrierDecision::Kept(BarrierKept::TraceLedgerUnknown)
        );
        assert_eq!(unknown.incr_guard(&named("b")), IncrGuard::RuntimeTraceBit);
        assert_eq!(
            BarrierDecision::Elided(BarrierElision::NoTraceReachesCell).as_str(),
            "elided:no-trace-reaches-cell"
        );
    }

    #[test]
    fn top_level_cells_are_never_demoted() {
        let policy = CellDemotion::top_level(enabled());
        assert_eq!(
            policy.decide("x"),
            CellDecision {
                storage: CellStorage::Cell,
                reason: CellStorageReason::TopLevelGlobal
            }
        );
        let disabled = CellDemotion::procedure(None, SemanticOptimisationConfig::new());
        assert_eq!(disabled.decide("x").reason, CellStorageReason::PassDisabled);
        let no_summary = CellDemotion::procedure(None, enabled());
        assert_eq!(
            no_summary.decide("x").reason,
            CellStorageReason::EscapesFrame
        );
    }
}
