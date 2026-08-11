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

//! Bounded completion facts shared by common compiler passes.
//!
//! This is deliberately independent of the current CFG terminators.  It gives
//! analysis and future executable-IR work one representation for Tcl's
//! code/result/options triple without changing the existing source CFG or
//! enabling any specialisation.

use tcl_core_types::Code as CompletionCode;
use tcl_registry::{
    CompletionCodeDomain, CompletionDescriptor, CompletionPayloadObligation,
    CompletionPayloadObligations,
};

use crate::bounded_set::BoundedSet;

/// Maximum number of distinct completion codes tracked exactly before widening.
///
/// The five standard codes leave room for target-specific or user-defined
/// `return -code N` paths while ensuring fixed-point analysis remains bounded.
pub const MAX_EXACT_COMPLETION_CODES: usize = 8;

/// A bounded lattice of Tcl completion codes.
///
/// `Bottom` represents no executable contribution yet, `Exact` tracks a
/// finite set of possible standard or custom integer codes, and `Top` means
/// any Tcl completion code may occur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionCodeLattice {
    /// No reachable completion has contributed a fact.
    Bottom,
    /// A bounded exact set of possible Tcl completion codes.
    Exact(BoundedSet<CompletionCode, MAX_EXACT_COMPLETION_CODES>),
    /// The completion code is not bounded precisely.
    Top,
}

impl CompletionCodeLattice {
    /// The bottom element, used before a reachable completion contributes.
    #[must_use]
    pub const fn bottom() -> Self {
        Self::Bottom
    }

    /// The top element, representing any Tcl integer completion code.
    #[must_use]
    pub const fn top() -> Self {
        Self::Top
    }

    /// Build an exact singleton completion-code fact.
    #[must_use]
    pub fn exact(code: CompletionCode) -> Self {
        Self::Exact(BoundedSet::singleton(code))
    }

    /// Build an exact bounded fact from a sequence of completion codes.
    ///
    /// Duplicates are ignored. An empty input is bottom; more than
    /// [`MAX_EXACT_COMPLETION_CODES`] distinct codes widens to top.
    #[must_use]
    pub fn exact_codes(codes: impl IntoIterator<Item = CompletionCode>) -> Self {
        let Some(mut codes) = BoundedSet::from_iter_bounded(codes) else {
            return Self::Top;
        };
        if codes.is_empty() {
            Self::Bottom
        } else {
            codes.canonicalise_by_key(|code| code.as_int());
            Self::Exact(codes)
        }
    }

    /// Convert the registry's target-neutral completion-code declaration.
    #[must_use]
    pub fn from_domain(domain: CompletionCodeDomain) -> Self {
        match domain {
            CompletionCodeDomain::Exact(codes) => Self::exact_codes(codes.iter().copied()),
            CompletionCodeDomain::Any => Self::Top,
        }
    }

    /// Join two completion-code facts, widening only when their exact union
    /// exceeds [`MAX_EXACT_COMPLETION_CODES`].
    #[must_use]
    pub fn join(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Bottom, value) | (value, Self::Bottom) => value.clone(),
            (Self::Top, _) | (_, Self::Top) => Self::Top,
            (Self::Exact(left), Self::Exact(right)) => {
                let mut joined = left.clone();
                for code in right {
                    if !joined.insert(*code) {
                        return Self::Top;
                    }
                }
                joined.canonicalise_by_key(|code| code.as_int());
                Self::Exact(joined)
            }
        }
    }

    /// Widen successive loop/fixpoint contributions.
    ///
    /// This first foundation has no threshold distinct from its bounded exact
    /// carrier, so widening is the same monotone operation as [`Self::join`].
    #[must_use]
    pub fn widen(&self, next: &Self) -> Self {
        self.join(next)
    }

    /// Whether `code` is a possible completion according to this fact.
    #[must_use]
    pub fn contains(&self, code: CompletionCode) -> bool {
        match self {
            Self::Bottom => false,
            Self::Exact(codes) => codes.contains(&code),
            Self::Top => true,
        }
    }

    /// The exact codes when this fact did not widen.
    #[must_use]
    pub fn exact_codes_slice(&self) -> Option<&[CompletionCode]> {
        match self {
            Self::Exact(codes) => Some(codes.as_slice()),
            Self::Bottom | Self::Top => None,
        }
    }
}

/// Completion data-flow obligations for a future executable CFG edge.
///
/// Every edge must carry a code, result, and return-options value. The code
/// is bounded by [`Self::codes`]; the two payload fields record whether an
/// operation produces, forwards, or conservatively obscures each value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionObligations {
    /// Possible Tcl completion codes.
    pub codes: CompletionCodeLattice,
    /// Result and return-options obligations.
    pub payloads: CompletionPayloadObligations,
}

impl CompletionObligations {
    /// The conservative generic-invocation contract.
    #[must_use]
    pub fn conservative() -> Self {
        Self::from_descriptor(CompletionDescriptor::CONSERVATIVE)
    }

    /// Build completion facts from a registry descriptor.
    #[must_use]
    pub fn from_descriptor(descriptor: CompletionDescriptor) -> Self {
        Self {
            codes: CompletionCodeLattice::from_domain(descriptor.codes),
            payloads: descriptor.payloads,
        }
    }

    /// Join two successor-edge obligations.
    #[must_use]
    pub fn join(&self, other: &Self) -> Self {
        Self {
            codes: self.codes.join(&other.codes),
            payloads: CompletionPayloadObligations {
                result: join_payload(self.payloads.result, other.payloads.result),
                options: join_payload(self.payloads.options, other.payloads.options),
            },
        }
    }

    /// Widen repeated loop/fixpoint contributions.
    #[must_use]
    pub fn widen(&self, next: &Self) -> Self {
        Self {
            codes: self.codes.widen(&next.codes),
            payloads: CompletionPayloadObligations {
                result: join_payload(self.payloads.result, next.payloads.result),
                options: join_payload(self.payloads.options, next.payloads.options),
            },
        }
    }
}

fn join_payload(
    left: CompletionPayloadObligation,
    right: CompletionPayloadObligation,
) -> CompletionPayloadObligation {
    if left == right {
        left
    } else {
        CompletionPayloadObligation::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_codes_track_standard_and_custom_integer_codes() {
        let codes = CompletionCodeLattice::exact_codes([
            CompletionCode::Error,
            CompletionCode::Other(37),
            CompletionCode::Error,
        ]);

        assert_eq!(
            codes.exact_codes_slice(),
            Some(&[CompletionCode::Error, CompletionCode::Other(37)][..])
        );
        assert!(codes.contains(CompletionCode::Other(37)));
        assert!(!codes.contains(CompletionCode::Ok));
    }

    #[test]
    fn exact_code_sets_are_deterministic_across_predecessor_order() {
        let forward =
            CompletionCodeLattice::exact_codes([CompletionCode::Other(71), CompletionCode::Error]);
        let reverse =
            CompletionCodeLattice::exact_codes([CompletionCode::Error, CompletionCode::Other(71)]);

        assert_eq!(forward, reverse);
        assert_eq!(
            forward.exact_codes_slice(),
            Some(&[CompletionCode::Error, CompletionCode::Other(71)][..])
        );
    }

    #[test]
    fn join_and_widen_keep_small_unions_then_widen_to_top() {
        let joined = CompletionCodeLattice::exact(CompletionCode::Ok)
            .join(&CompletionCodeLattice::exact(CompletionCode::Other(41)));
        assert_eq!(
            joined.exact_codes_slice(),
            Some(&[CompletionCode::Ok, CompletionCode::Other(41)][..])
        );

        let within_limit = CompletionCodeLattice::exact_codes(
            (0..i32::try_from(MAX_EXACT_COMPLETION_CODES).expect("small fixed capacity"))
                .map(CompletionCode::from_int),
        );
        let widened = within_limit.widen(&CompletionCodeLattice::exact(CompletionCode::Other(
            i32::try_from(MAX_EXACT_COMPLETION_CODES).expect("small fixed capacity"),
        )));
        assert_eq!(widened, CompletionCodeLattice::Top);
    }

    #[test]
    fn obligation_join_preserves_matching_payloads_and_widens_mismatches() {
        let produced = CompletionObligations::from_descriptor(CompletionDescriptor::exact(&[
            CompletionCode::Ok,
        ]));
        let forwarded = CompletionObligations {
            codes: CompletionCodeLattice::exact(CompletionCode::Error),
            payloads: CompletionPayloadObligations::FORWARDED,
        };

        let joined = produced.join(&forwarded);
        assert_eq!(
            joined.codes.exact_codes_slice(),
            Some(&[CompletionCode::Ok, CompletionCode::Error][..])
        );
        assert_eq!(joined.payloads, CompletionPayloadObligations::UNKNOWN);
    }

    #[test]
    fn conservative_obligations_model_the_generic_invoke_fallback() {
        let fallback = CompletionObligations::conservative();
        assert_eq!(fallback.codes, CompletionCodeLattice::Top);
        assert_eq!(fallback.payloads, CompletionPayloadObligations::UNKNOWN);
    }
}
