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

//! The immutable identity every query and evaluation runs under, and the
//! budget every route charges
//! (`docs/design/compiler/value-transfers.md` § *One invocation, one
//! context*).

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tcl_dialect::{DialectProfile, LexerGrammar};

use super::decline::{AnalysisTier, BudgetLimit, DeclineReason};

/// A command binding the analysed program uses: the spelling, the namespace
/// it resolves in, and the registry identity expected there. The
/// registry-neutral twin of the runtime's binding identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BindingIdentity {
    /// The namespace the spelling resolves in, unrooted (`""` is global).
    pub resolution_namespace: String,
    /// The spelling the program wrote.
    pub name: String,
    /// The registry identity expected at that binding.
    pub identity: String,
}

/// The module's command-binding evidence: which builtins the module
/// rebinds, which procedures it redefines, and whether any binding is
/// computed. Carried as a value so a `rename` anywhere in the module is
/// part of every answer's identity. `Default` is the "no mutations
/// observed" baseline.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct BindingEvidence {
    /// Core builtins the module rebinds by `rename`, `interp alias`, or a
    /// shadowing `proc`, sorted.
    pub untrusted_builtins: Vec<String>,
    /// Procedures the module redefines, sorted.
    pub rebound: Vec<String>,
    /// Whether a computed rebinding makes every name suspect.
    pub dynamic: bool,
    /// Namespaces whose command resolution the module cannot see through,
    /// sorted.
    pub opaque_namespaces: Vec<String>,
}

/// The immutable identity every query and evaluation runs under. Produced
/// once by the composition root; consumed by every memo key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisContext {
    /// The effective registry generation.
    pub registry_generation: u64,
    /// The overlay generation, when a workspace pack overlay is installed.
    pub overlay_generation: Option<u64>,
    /// The module's command-binding evidence.
    pub bindings: BindingEvidence,
    /// The namespace the head resolves in.
    pub namespace: String,
    /// The target profile, when the request names one.
    pub profile: Option<&'static DialectProfile>,
    /// The grammar the document was read under.
    pub grammar: LexerGrammar,
    /// Every literal variable name a trace targets anywhere in the module.
    pub traced_variables: BTreeSet<String>,
    /// Whether a trace targets a computed name, making every place traced.
    pub has_dynamic_variable_trace: bool,
    /// The escaping set: places writable from outside the function.
    pub escaping: BTreeSet<String>,
    /// The call-site seeds and transfer-summary revision.
    pub seeds_revision: u64,
    /// Evaluator and implementation revisions the registry does not fix.
    pub evaluator_revision: u64,
    /// The precision tier the request runs at.
    pub tier: AnalysisTier,
}

impl AnalysisContext {
    /// A context for a consumer with no module view: no bindings evidence,
    /// no traces, no escaping set, the deep tier, and the profile's grammar
    /// when it has one.
    #[must_use]
    pub fn detached(profile: Option<&'static DialectProfile>) -> Self {
        Self {
            registry_generation: 0,
            overlay_generation: None,
            bindings: BindingEvidence::default(),
            namespace: "::".to_owned(),
            profile,
            grammar: profile.map_or_else(LexerGrammar::default, |p| p.grammar),
            traced_variables: BTreeSet::new(),
            has_dynamic_variable_trace: false,
            escaping: BTreeSet::new(),
            seeds_revision: 0,
            evaluator_revision: 0,
            tier: AnalysisTier::Deep,
        }
    }
}

/// The request-wide and per-evaluation limits every route charges.
#[derive(Debug)]
pub struct Budget {
    /// Evaluation fuel remaining.
    pub fuel: u64,
    /// Nesting depth remaining.
    pub depth: u32,
    /// Result bytes remaining.
    pub result_bytes: usize,
    /// Allocation bytes remaining.
    pub allocation_bytes: usize,
    /// Request time remaining.
    pub request_remaining: Duration,
    /// Whether the request was cancelled.
    pub cancelled: AtomicBool,
}

impl Budget {
    /// The work one evaluation may spend: a million elementary steps, a
    /// few milliseconds of native work on the machines the acceptance
    /// suite runs on.
    pub const EVALUATION_WORK: u64 = 1_000_000;
    /// The bytes one evaluation may allocate or publish: the bounded
    /// host's per-value cap.
    pub const EVALUATION_BYTES: usize = 16 * 1024 * 1024;
    /// The nesting depth one evaluation may reach.
    pub const EVALUATION_DEPTH: u32 = 64;

    /// The per-evaluation budget a route runs under: the evaluation
    /// contract's defaults, with the request and iteration ceilings above
    /// it still unbounded until the slices that charge them.
    #[must_use]
    pub fn evaluation() -> Self {
        Self {
            fuel: Self::EVALUATION_WORK,
            depth: Self::EVALUATION_DEPTH,
            result_bytes: Self::EVALUATION_BYTES,
            allocation_bytes: Self::EVALUATION_BYTES,
            request_remaining: Duration::MAX,
            cancelled: AtomicBool::new(false),
        }
    }

    /// Charge `units` of work.
    ///
    /// # Errors
    ///
    /// `Budget(Fuel)` once the work is spent, `Budget(Cancelled)` once the
    /// request is cancelled; the budget stays exhausted afterwards.
    pub fn charge_work(&mut self, units: u64) -> Result<(), DeclineReason> {
        if self.is_cancelled() {
            return Err(DeclineReason::Budget(BudgetLimit::Cancelled));
        }
        match self.fuel.checked_sub(units) {
            Some(left) => {
                self.fuel = left;
                Ok(())
            }
            None => {
                self.fuel = 0;
                Err(DeclineReason::Budget(BudgetLimit::Fuel))
            }
        }
    }

    /// Charge `bytes` of allocation, before the allocation happens.
    ///
    /// # Errors
    ///
    /// `Budget(AllocationBytes)` once the bound is reached.
    pub fn charge_allocation(&mut self, bytes: u64) -> Result<(), DeclineReason> {
        Self::charge_bytes(
            &mut self.allocation_bytes,
            bytes,
            BudgetLimit::AllocationBytes,
        )
    }

    /// Charge `bytes` of published result.
    ///
    /// # Errors
    ///
    /// `Budget(ResultBytes)` once the bound is reached.
    pub fn charge_result(&mut self, bytes: u64) -> Result<(), DeclineReason> {
        Self::charge_bytes(&mut self.result_bytes, bytes, BudgetLimit::ResultBytes)
    }

    fn charge_bytes(left: &mut usize, bytes: u64, limit: BudgetLimit) -> Result<(), DeclineReason> {
        match usize::try_from(bytes)
            .ok()
            .and_then(|bytes| left.checked_sub(bytes))
        {
            Some(remaining) => {
                *left = remaining;
                Ok(())
            }
            None => {
                *left = 0;
                Err(DeclineReason::Budget(limit))
            }
        }
    }

    /// A budget no route can exhaust, for a caller that bounds the work
    /// some other way.
    #[must_use]
    pub fn unbounded() -> Self {
        Self {
            fuel: u64::MAX,
            depth: u32::MAX,
            result_bytes: usize::MAX,
            allocation_bytes: usize::MAX,
            request_remaining: Duration::MAX,
            cancelled: AtomicBool::new(false),
        }
    }

    /// Whether the request was cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}
