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
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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

/// The generation of this thread's evaluator set
/// (`docs/design/compiler/value-evaluation.md` § *The evaluator
/// generation*): which host serves the declared implementations, and in
/// what health. Carried as [`AnalysisContext::evaluator_revision`], so a
/// memoised answer names the evaluators it was computed with, and a
/// host-present and a host-absent worker never share one.
///
/// It changes at exactly the three places that clear the hook cache:
/// installing a host, clearing it, and the host quarantining a hook. Two
/// workers whose hosts were built from one published plan share a
/// generation, so their memoised answers are shared; a quarantine gives its
/// worker a generation no other state has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct EvaluatorGeneration(pub u32);

impl EvaluatorGeneration {
    /// A worker with no host: every declared implementation declines
    /// `Transient` there, and every such worker answers alike.
    pub const NO_HOST: Self = Self(0);
}

/// One enclosing level of the nesting — a request, or one solver
/// iteration inside it — shared by every budget nested under it, so work
/// any of them charges is charged here too
/// (`docs/design/compiler/value-evaluation.md` § *The three nested
/// budgets*).
#[derive(Debug)]
struct Level {
    /// Work remaining at this level.
    work: AtomicU64,
    /// Bytes this level may still retain; `None` for an iteration, whose
    /// retained bytes are its request's.
    retained: Option<AtomicU64>,
}

impl Level {
    fn new(work: u64, retained: Option<u64>) -> Arc<Self> {
        Arc::new(Self {
            work: AtomicU64::new(work),
            retained: retained.map(AtomicU64::new),
        })
    }

    /// Take `units` from `counter`, or empty it and answer `false` when it
    /// holds fewer — an exhausted level stays exhausted, and charges
    /// nothing more, not even a free step.
    fn take(counter: &AtomicU64, units: u64) -> bool {
        let mut enough = true;
        let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
            if left == 0 || left < units {
                enough = false;
                Some(0)
            } else {
                Some(left - units)
            }
        });
        enough
    }
}

/// The request-wide and per-evaluation limits every route charges.
///
/// Three budgets nest (`docs/design/compiler/value-evaluation.md` § *The
/// three nested budgets*): one [`Self::request`] per editor or CLI request,
/// one [`Self::iteration`] per solver pass inside it, and one
/// [`Self::evaluation_within`] per evaluation inside that. Work an
/// evaluation charges is charged to its iteration and its request as well,
/// so a request whose evaluations have spent it declines the rest with
/// `Budget(Request)`. A standalone [`Self::evaluation`] has no enclosing
/// levels.
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
    /// The iteration and the request this budget charges through,
    /// innermost first; empty for a standalone evaluation.
    enclosing: Vec<Arc<Level>>,
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
    /// The work one request may spend. What it bounds depends on the
    /// route: a native route's unit is calibrated at about four
    /// milliseconds per million, so native work stays near the
    /// interactive target of 200 ms; a declared implementation charges one
    /// unit per engine command, which is not calibrated to time, so there
    /// it bounds commands — about 500 calls at the bounded host's
    /// per-call allowance of 100,000 — and each call's time is the host's
    /// own clock to bound.
    pub const REQUEST_WORK: u64 = 50_000_000;
    /// The bytes one request may retain across everything it publishes.
    pub const REQUEST_RETAINED_BYTES: u64 = 64 * 1024 * 1024;
    /// The share of the request's remaining work one iteration may spend:
    /// one tenth, so the first pass of a fixed point cannot starve the last.
    const ITERATION_SHARE: u64 = 10;

    /// The per-evaluation budget a route runs under, standalone: the
    /// evaluation contract's defaults and no enclosing request.
    #[must_use]
    pub fn evaluation() -> Self {
        Self {
            fuel: Self::EVALUATION_WORK,
            depth: Self::EVALUATION_DEPTH,
            result_bytes: Self::EVALUATION_BYTES,
            allocation_bytes: Self::EVALUATION_BYTES,
            request_remaining: Duration::MAX,
            cancelled: AtomicBool::new(false),
            enclosing: Vec::new(),
        }
    }

    /// One request's budget: [`Self::REQUEST_WORK`] and
    /// [`Self::REQUEST_RETAINED_BYTES`].
    #[must_use]
    pub fn request() -> Self {
        Self::request_of(Self::REQUEST_WORK, Self::REQUEST_RETAINED_BYTES)
    }

    /// A request's budget of `work` units and `retained` bytes — the
    /// default's shape at another size.
    #[must_use]
    pub fn request_of(work: u64, retained: u64) -> Self {
        Self {
            enclosing: vec![Level::new(work, Some(retained))],
            ..Self::unbounded()
        }
    }

    /// One iteration inside this request: one tenth of the request's
    /// remaining work, charged through to the request.
    #[must_use]
    pub fn iteration(&mut self) -> Self {
        let remaining = self
            .enclosing
            .iter()
            .map(|level| level.work.load(Ordering::Relaxed))
            .min()
            .unwrap_or(u64::MAX);
        let mut enclosing = Vec::with_capacity(self.enclosing.len() + 1);
        enclosing.push(Level::new(remaining / Self::ITERATION_SHARE, None));
        enclosing.extend(self.enclosing.iter().cloned());
        Self {
            enclosing,
            ..Self::unbounded()
        }
    }

    /// One evaluation inside this iteration or request: the evaluation
    /// contract's defaults, every charge also charged to the levels this
    /// budget charges through.
    #[must_use]
    pub fn evaluation_within(&self) -> Self {
        Self {
            enclosing: self.enclosing.clone(),
            ..Self::evaluation()
        }
    }

    /// Charge `units` of work, to this budget and every level it charges
    /// through.
    ///
    /// # Errors
    ///
    /// `Budget(Fuel)` once this evaluation's work is spent,
    /// `Budget(Request)` once its iteration's or its request's is,
    /// `Budget(Cancelled)` once the request is cancelled; the budget stays
    /// exhausted afterwards.
    pub fn charge_work(&mut self, units: u64) -> Result<(), DeclineReason> {
        if self.is_cancelled() {
            return Err(DeclineReason::Budget(BudgetLimit::Cancelled));
        }
        if let Some(left) = self.fuel.checked_sub(units) {
            self.fuel = left;
        } else {
            self.fuel = 0;
            return Err(DeclineReason::Budget(BudgetLimit::Fuel));
        }
        let mut enough = true;
        for level in &self.enclosing {
            enough &= Level::take(&level.work, units);
        }
        if enough {
            Ok(())
        } else {
            Err(DeclineReason::Budget(BudgetLimit::Request))
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

    /// Charge `bytes` of published result, to this evaluation and to the
    /// bytes its request may retain.
    ///
    /// # Errors
    ///
    /// `Budget(ResultBytes)` once this evaluation's bound is reached,
    /// `Budget(Request)` once its request's retained bytes are.
    pub fn charge_result(&mut self, bytes: u64) -> Result<(), DeclineReason> {
        Self::charge_bytes(&mut self.result_bytes, bytes, BudgetLimit::ResultBytes)?;
        let mut enough = true;
        for retained in self
            .enclosing
            .iter()
            .filter_map(|level| level.retained.as_ref())
        {
            enough &= Level::take(retained, bytes);
        }
        if enough {
            Ok(())
        } else {
            Err(DeclineReason::Budget(BudgetLimit::Request))
        }
    }

    fn charge_bytes(left: &mut usize, bytes: u64, limit: BudgetLimit) -> Result<(), DeclineReason> {
        if let Some(remaining) = usize::try_from(bytes)
            .ok()
            .and_then(|bytes| left.checked_sub(bytes))
        {
            *left = remaining;
            Ok(())
        } else {
            *left = 0;
            Err(DeclineReason::Budget(limit))
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
            enclosing: Vec::new(),
        }
    }

    /// Whether the request was cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// The work this budget can still charge: the least of its own and of
    /// every level it charges through.
    #[must_use]
    pub fn remaining_work(&self) -> u64 {
        self.enclosing
            .iter()
            .map(|level| level.work.load(Ordering::Relaxed))
            .fold(self.fuel, u64::min)
    }
}
