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

//! Handler composition (issue #1204).
//!
//! Multiple `when EVENT priority N { … }` blocks for the same event compose
//! into one **handler chain** with explicit, testable semantics:
//!
//! 1. **Lower priority numbers run first.** The chain is evaluated in ascending
//!    priority order (F5-inspired), with the event name as a stable tiebreaker.
//! 2. **A terminal verdict stops evaluation.** The first handler that returns a
//!    terminal verdict (`accept`/`drop`/`pass`/`tx`) decides the packet; later
//!    handlers do not run.
//! 3. **Continuation is explicit.** A handler continues to the next only by
//!    ending its path with the non-terminal `next` outcome, which lowers to the
//!    reserved [`crate::ir::CONTINUE_SENTINEL`] return value.
//! 4. **Each event declares its default.** If every handler on a path yields
//!    `next`, the event's [`default_verdict`](tcl_registry::bpf_op::BpfEventSpec::default_verdict)
//!    applies.
//!
//! These are the *source* semantics. The *deployment* semantics are a verified
//! program chain: each handler is compiled independently, and the chain is
//! evaluated by running the handlers in order and interpreting each one's return
//! value with [`classify_outcome`] — a dispatcher expressed in the loader rather
//! than fused into one eBPF object. This module is the backend-agnostic model of
//! that chain; the actual per-handler execution (rbpf, or the kernel) lives in
//! the harness that owns a run engine. Because the rules are pure over the
//! per-handler outcomes, the composition is deterministic and unit-testable in
//! userspace without a kernel.

use crate::ir::{BpfModule, CONTINUE_SENTINEL};

/// A single handler's outcome once run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The handler ended with `next` (the continuation sentinel) — evaluation
    /// proceeds to the next handler.
    Continue,
    /// The handler returned a terminal verdict with this value (an XDP/TC action
    /// number, or a socket-filter accepted byte count / `0` drop).
    Terminal(i64),
}

impl Outcome {
    /// Classify a raw handler return value (the low 32 bits of `r0`). The
    /// reserved continuation sentinel maps to [`Outcome::Continue`]; every other
    /// value is a terminal verdict.
    #[must_use]
    pub fn classify(run_result: u64) -> Self {
        let low = run_result & 0xffff_ffff;
        if low == (CONTINUE_SENTINEL as u64 & 0xffff_ffff) {
            Outcome::Continue
        } else {
            // Sign-extend the 32-bit action back to i64 for display parity with
            // the verdict formatting (all real verdicts are small non-negative).
            Outcome::Terminal(i64::from(low as u32))
        }
    }
}

/// The classification of a raw handler return value (free function form of
/// [`Outcome::classify`], for callers that prefer it).
#[must_use]
pub fn classify_outcome(run_result: u64) -> Outcome {
    Outcome::classify(run_result)
}

/// The result of evaluating a handler chain over one packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainResult {
    /// The handler at `index` (into the chain, priority order) returned a
    /// terminal verdict; that value is the decision.
    Decided {
        /// Index of the deciding handler within the chain.
        index: usize,
        /// The terminal verdict value it returned.
        verdict: i64,
    },
    /// Every handler yielded `next`; the event's default verdict applies.
    Default,
}

/// Evaluate a handler chain's composition rules over the per-handler outcomes,
/// which must already be in ascending-priority order.
///
/// Returns [`ChainResult::Decided`] for the first terminal outcome, or
/// [`ChainResult::Default`] when every handler continued.
#[must_use]
pub fn compose(outcomes: &[Outcome]) -> ChainResult {
    for (index, outcome) in outcomes.iter().enumerate() {
        if let Outcome::Terminal(verdict) = outcome {
            return ChainResult::Decided {
                index,
                verdict: *verdict,
            };
        }
    }
    ChainResult::Default
}

/// One event's ordered handler chain, projected from a compiled [`BpfModule`].
#[derive(Debug, Clone)]
pub struct EventChain {
    /// Canonical/declared event name shared by every handler in the chain.
    pub event: String,
    /// Indices into [`BpfModule::programs`] for this event's handlers, already
    /// in ascending-priority order (the module is pre-sorted).
    pub handler_indices: Vec<usize>,
    /// Each handler's declared priority, parallel to `handler_indices`.
    pub priorities: Vec<u32>,
}

impl EventChain {
    /// The number of handlers in the chain.
    #[must_use]
    pub fn len(&self) -> usize {
        self.handler_indices.len()
    }

    /// Whether the chain has no handlers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handler_indices.is_empty()
    }
}

/// Project a compiled module into per-event handler chains. Handlers keep the
/// module's ascending-priority order; events appear in first-seen order.
///
/// Two handlers of the same event at the *same* priority are a composition
/// hazard — their relative order is decided only by the event-name tiebreaker,
/// which is identical here, so the source order is not preserved deterministically
/// across edits. [`ambiguous_priorities`] flags that so the front-end can reject
/// it (issue #1204: "sorting independent object files is not sufficient").
#[must_use]
pub fn event_chains(module: &BpfModule) -> Vec<EventChain> {
    let mut chains: Vec<EventChain> = Vec::new();
    for (index, decl) in module.programs.iter().enumerate() {
        if let Some(chain) = chains.iter_mut().find(|c| c.event == decl.event) {
            chain.handler_indices.push(index);
            chain.priorities.push(decl.priority);
        } else {
            chains.push(EventChain {
                event: decl.event.clone(),
                handler_indices: vec![index],
                priorities: vec![decl.priority],
            });
        }
    }
    chains
}

/// Report any event whose chain has two handlers sharing one priority — an
/// ambiguous, non-deterministic composition order. Returns `(event, priority)`
/// pairs.
#[must_use]
pub fn ambiguous_priorities(module: &BpfModule) -> Vec<(String, u32)> {
    let mut clashes = Vec::new();
    for chain in event_chains(module) {
        let mut seen = chain.priorities.clone();
        seen.sort_unstable();
        for pair in seen.windows(2) {
            if pair[0] == pair[1]
                && !clashes
                    .iter()
                    .any(|(e, p)| e == &chain.event && *p == pair[0])
            {
                clashes.push((chain.event.clone(), pair[0]));
            }
        }
    }
    clashes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuation_sentinel_classifies_as_continue() {
        assert_eq!(
            Outcome::classify(CONTINUE_SENTINEL as u64),
            Outcome::Continue
        );
        // Upper bits ignored: the kernel only defines the low 32.
        assert_eq!(Outcome::classify(0xdead_0000_ffff_ffff), Outcome::Continue);
    }

    #[test]
    fn real_verdicts_classify_as_terminal() {
        assert_eq!(Outcome::classify(0), Outcome::Terminal(0)); // socket drop
        assert_eq!(Outcome::classify(2), Outcome::Terminal(2)); // XDP_PASS
        assert_eq!(Outcome::classify(1500), Outcome::Terminal(1500)); // accept 1500
    }

    #[test]
    fn first_terminal_wins_and_stops_the_chain() {
        // priority order: continue, then a drop, then an (unreached) pass.
        let outcomes = [
            Outcome::Continue,
            Outcome::Terminal(1),
            Outcome::Terminal(2),
        ];
        assert_eq!(
            compose(&outcomes),
            ChainResult::Decided {
                index: 1,
                verdict: 1
            }
        );
    }

    #[test]
    fn all_continue_falls_through_to_default() {
        let outcomes = [Outcome::Continue, Outcome::Continue];
        assert_eq!(compose(&outcomes), ChainResult::Default);
        assert_eq!(compose(&[]), ChainResult::Default);
    }

    #[test]
    fn a_leading_terminal_short_circuits() {
        let outcomes = [Outcome::Terminal(1), Outcome::Continue];
        assert_eq!(
            compose(&outcomes),
            ChainResult::Decided {
                index: 0,
                verdict: 1
            }
        );
    }
}
