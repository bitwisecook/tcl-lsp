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

//! Shared knowledge and refusal vocabulary for naming queries and editor actions.

use alloc::{string::String, vec, vec::Vec};

/// Origin of the evidence supporting a naming answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamingProvenance {
    /// Source analysis; no actual runtime allocation is asserted.
    Analysis,
    /// Exact source instance and byte offset retained by the lookup owner.
    Source { instance: u64, offset: u32 },
    /// An observation of an actual interpreter and its retained token.
    Runtime { owner: u64, interpreter: u64 },
    /// Facts supplied by the selected command registry.
    Registry,
}

/// The purpose for which the naming owner supplied its evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamingAuthority {
    /// Assistance only; neither execution nor editing is authorised.
    Diagnostic,
    /// A retained target suitable for navigation.
    Navigation,
    /// Actual runtime identity; operation-specific lifetime checks still apply.
    Runtime,
}

/// Why a lookup cannot supply a closed answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnknownReason {
    /// The positioned lookup world or its routing is incomplete.
    IncompleteLookup,
    /// An evaluated name has no exact retained value.
    DynamicName,
    /// The source or runtime context is missing or stale.
    UnavailableContext,
    /// The finite alternatives exceed the caller's bound.
    AlternativeLimit,
    /// Repeated allocation cannot identify one incarnation.
    RepeatedAllocation,
}

/// Knowledge retained by an answer. Absence is distinct from incomplete evidence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Knowledge<T> {
    /// One value on every closed path.
    Proven(T),
    /// A complete finite set, with independently retained possible absence.
    BoundedSet {
        values: Vec<T>,
        includes_absent: bool,
    },
    /// No value on any closed path.
    Absent,
    /// No closed answer, with an explicit reason.
    Unknown(UnknownReason),
}

/// A naming answer together with its provenance and permitted purpose.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[must_use]
pub struct Answer<T> {
    /// The complete knowledge state.
    pub knowledge: Knowledge<T>,
    /// Where the evidence came from.
    pub provenance: NamingProvenance,
    /// The purpose supported by this evidence.
    pub authority: NamingAuthority,
}

impl<T> Answer<T> {
    /// Construct a proven answer without granting execution or edit authority.
    pub const fn proven(value: T) -> Self {
        Self::new(Knowledge::Proven(value))
    }

    /// Construct a closed absence answer.
    pub const fn absent() -> Self {
        Self::new(Knowledge::Absent)
    }

    /// Construct an abstention with a specific reason.
    pub const fn unknown(reason: UnknownReason) -> Self {
        Self::new(Knowledge::Unknown(reason))
    }

    /// Incomplete positioned lookup evidence.
    pub const fn unavailable() -> Self {
        Self::unknown(UnknownReason::IncompleteLookup)
    }

    const fn new(knowledge: Knowledge<T>) -> Self {
        Self {
            knowledge,
            provenance: NamingProvenance::Analysis,
            authority: NamingAuthority::Diagnostic,
        }
    }

    /// Retain the owner-supplied provenance and purpose of this answer.
    pub const fn with_evidence(
        mut self,
        provenance: NamingProvenance,
        authority: NamingAuthority,
    ) -> Self {
        self.provenance = provenance;
        self.authority = authority;
        self
    }

    /// Whether every closed path has the same value.
    pub const fn is_proven(&self) -> bool {
        matches!(self.knowledge, Knowledge::Proven(_))
    }

    /// Whether the owner has proved absence.
    pub const fn is_absent(&self) -> bool {
        matches!(self.knowledge, Knowledge::Absent)
    }

    /// Whether the owner abstained rather than proved absence.
    pub const fn is_unknown(&self) -> bool {
        matches!(self.knowledge, Knowledge::Unknown(_))
    }

    /// Borrow the unique proved value; conditional alternatives remain conditional.
    pub const fn as_proven(&self) -> Option<&T> {
        if let Knowledge::Proven(value) = &self.knowledge {
            Some(value)
        } else {
            None
        }
    }

    /// Extract only a unique proved value.
    pub fn into_proven(self) -> Option<T> {
        if let Knowledge::Proven(value) = self.knowledge {
            Some(value)
        } else {
            None
        }
    }

    /// Transform retained values without turning unknown into absent.
    pub fn map<U>(self, mut transform: impl FnMut(T) -> U) -> Answer<U> {
        let knowledge = match self.knowledge {
            Knowledge::Proven(value) => Knowledge::Proven(transform(value)),
            Knowledge::BoundedSet {
                values,
                includes_absent,
            } => Knowledge::BoundedSet {
                values: values.into_iter().map(transform).collect(),
                includes_absent,
            },
            Knowledge::Absent => Knowledge::Absent,
            Knowledge::Unknown(reason) => Knowledge::Unknown(reason),
        };
        Answer {
            knowledge,
            provenance: self.provenance,
            authority: self.authority,
        }
    }
}

impl<T: Eq> Answer<T> {
    /// Canonicalise a finite set, deduplicating before enforcing the bound.
    pub fn bounded(
        values: impl IntoIterator<Item = T>,
        includes_absent: bool,
        limit: usize,
    ) -> Self {
        let mut unique = Vec::new();
        for value in values {
            if !unique.contains(&value) {
                if unique.len() == limit {
                    return Self::unknown(UnknownReason::AlternativeLimit);
                }
                unique.push(value);
            }
        }
        if unique.is_empty() {
            return if includes_absent {
                Self::absent()
            } else {
                Self::unknown(UnknownReason::IncompleteLookup)
            };
        }
        if unique.len() == 1 && !includes_absent {
            return Self::proven(unique.pop().expect("one retained alternative"));
        }
        Self::new(Knowledge::BoundedSet {
            values: unique,
            includes_absent,
        })
    }
}

impl Answer<()> {
    /// Every closed route contains a slot, without claiming its occupant.
    pub const fn present() -> Self {
        Self::proven(())
    }

    /// Closed routes include both slot presence and absence.
    pub fn conditional() -> Self {
        Self::bounded(vec![()], true, 1)
    }
}

/// A user-visible refusal, optionally attached to a host-specific source range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal<R = ()> {
    /// Explanation that the host must show to the user.
    pub reason: String,
    /// The offending source range, when the host can identify it.
    pub range: Option<R>,
}

/// An editor action either completes or returns its visible refusal.
pub type EditorOutcome<T, R = ()> = Result<T, Refusal<R>>;

/// Independent conformance counts; abstention is never a wrong assertion.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ConformanceCounts {
    /// Answers agreeing with the oracle.
    pub correct: usize,
    /// Unknown or conditional answers that make no unique assertion.
    pub abstentions: usize,
    /// Closed assertions disagreeing with the oracle.
    pub wrong: usize,
}

impl ConformanceCounts {
    /// Compare one query with an oracle; `None` is the oracle's proved absence.
    pub fn observe<T: Eq>(&mut self, answer: &Answer<T>, expected: Option<&T>) {
        match &answer.knowledge {
            Knowledge::Proven(value) if Some(value) == expected => self.correct += 1,
            Knowledge::Absent if expected.is_none() => self.correct += 1,
            Knowledge::Proven(_) | Knowledge::Absent => self.wrong += 1,
            Knowledge::BoundedSet {
                values,
                includes_absent,
            } => {
                if expected.map_or(*includes_absent, |value| values.contains(value)) {
                    self.abstentions += 1;
                } else {
                    self.wrong += 1;
                }
            }
            Knowledge::Unknown(_) => self.abstentions += 1,
        }
    }
}

impl core::fmt::Display for ConformanceCounts {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            out,
            "correct={} abstentions={} wrong={}",
            self.correct, self.abstentions, self.wrong
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn conditional_presence_never_becomes_absence_or_a_unique_target() {
        let conditional = Answer::bounded([7, 7], true, 2);
        assert!(!conditional.is_absent());
        assert!(!conditional.is_proven());
        assert!(!conditional.is_unknown());
        assert_eq!(conditional.into_proven(), None);
        assert_eq!(Answer::bounded([7, 7], false, 1), Answer::proven(7));
        assert_eq!(
            Answer::bounded([7, 8], false, 1),
            Answer::unknown(UnknownReason::AlternativeLimit)
        );
    }

    #[test]
    fn mapping_preserves_abstention_and_evidence() {
        let answer = Answer::<u32>::unknown(UnknownReason::DynamicName).with_evidence(
            NamingProvenance::Source {
                instance: 12,
                offset: 3,
            },
            NamingAuthority::Navigation,
        );
        let mapped = answer.map(|value| value.to_string());
        assert_eq!(
            mapped.knowledge,
            Knowledge::Unknown(UnknownReason::DynamicName)
        );
        assert_eq!(
            mapped.provenance,
            NamingProvenance::Source {
                instance: 12,
                offset: 3
            }
        );
        assert_eq!(mapped.authority, NamingAuthority::Navigation);
    }

    #[test]
    fn report_counts_abstentions_separately_from_wrong_answers() {
        let mut counts = ConformanceCounts::default();
        for answer in [
            Answer::proven(7),
            Answer::proven(8),
            Answer::absent(),
            Answer::unknown(UnknownReason::DynamicName),
            Answer::bounded([7, 8], false, 2),
        ] {
            counts.observe(&answer, Some(&7));
        }
        assert_eq!(counts.to_string(), "correct=1 abstentions=2 wrong=2");
    }
}
