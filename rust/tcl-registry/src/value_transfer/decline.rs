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

//! Why an answer is not exact.
//!
//! [`DeclineReason`] is the interface contract's list
//! (`docs/design/compiler/value-transfers.md` § *Decline conditions, stated
//! once*), grouped by the step of the lift that records it. The two payloads
//! the evaluation contract defines — [`NoRouteReason`] and [`Axis`] — live
//! beside it. A decline widens the affected definitions; it is never a Tcl
//! error, a no-match, or an absent variable.

use tcl_dialect::model::SpecSurface;

/// The precision tier an analysis request runs at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalysisTier {
    /// Structure only: tokens, symbols, no lattice.
    Structure,
    /// The fast tier: no deep facts.
    Fast,
    /// The deep tier: the per-function lattices.
    Deep,
    /// A function over the complexity ceiling: the deep tier declined to
    /// build its lattices.
    ComplexityGuarded,
}

impl AnalysisTier {
    /// Stable spelling for the inventory and the Explorer.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Structure => "structure",
            Self::Fast => "fast",
            Self::Deep => "deep",
            Self::ComplexityGuarded => "complexity-guarded",
        }
    }
}

/// Which resource limit a route ran into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BudgetLimit {
    /// The evaluation fuel.
    Fuel,
    /// The nesting depth.
    Depth,
    /// The result size, bounded before allocation.
    ResultBytes,
    /// The allocation bound.
    AllocationBytes,
    /// The request-wide time budget.
    Request,
    /// The request was cancelled.
    Cancelled,
}

/// Why a specialisation has no route: the payload of
/// [`DeclineReason::NoRoute`], and the "none" column of the generated
/// inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoRouteReason {
    /// `evaluate none`: the author abstained at this scope.
    Declared,
    /// No evaluator is authored for the form — the state of most pure
    /// commands today.
    Unauthored,
    /// The form or option is outside what the route models
    /// (`regexp -about`).
    FormUnsupported,
    /// The form runs a callback (`regsub -command`, a `-command`
    /// comparison), which needs a declared route of its own.
    Callback,
    /// The value is the host platform's to decide (a `stat` call, a
    /// temporary file's name): never satisfiable from the source alone
    /// (`Axis::Platform`, `Needs::PLATFORM`).
    Platform,
}

impl NoRouteReason {
    /// Stable spelling for the inventory and the Explorer.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Unauthored => "unauthored",
            Self::FormUnsupported => "form-unsupported",
            Self::Callback => "callback",
            Self::Platform => "platform",
        }
    }
}

/// The release axis two answers differed on: the payload of
/// [`DeclineReason::ReleaseAmbiguous`]. One variant per admissibility bit
/// the evaluation contract lists, plus the availability of a command, form,
/// or option that the profile's releases do not all have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    /// How a numeral operand is read.
    NumeralGrammar,
    /// How an index numeral is read.
    IndexGrammar,
    /// The character-counting rule.
    CharacterModel,
    /// The addressing unit for a character index.
    CharIndexing,
    /// Whether an integer operation widens or raises.
    IntTower,
    /// The `binary` field-letter set and the `u` suffix.
    BinaryFields,
    /// The `format` conversion set.
    FormatVerbs,
    /// The `string is` class set and its bounds.
    StringClasses,
    /// ARE syntax, flags, and engine limits.
    RegexpFeatures,
    /// The canonical quoting of a list result.
    ListRendering,
    /// Canonical dict key order and the duplicate rule.
    DictOrder,
    /// Case folding and ordering in a comparison.
    Collation,
    /// How a code point above `U+00FF` crosses to bytes.
    ByteStrings,
    /// How a non-ASCII source literal was decoded.
    SourceEncoding,
    /// Host facts a platform-backed core reads.
    Platform,
    /// A clock, locale, or timezone read.
    WallClock,
    /// A command, form, or option the profile's releases do not all have.
    Availability(SpecSurface),
}

impl Axis {
    /// Stable spelling for the Explorer and the inventory.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NumeralGrammar => "numeral-grammar",
            Self::IndexGrammar => "index-grammar",
            Self::CharacterModel => "character-model",
            Self::CharIndexing => "char-indexing",
            Self::IntTower => "int-tower",
            Self::BinaryFields => "binary-fields",
            Self::FormatVerbs => "format-verbs",
            Self::StringClasses => "string-classes",
            Self::RegexpFeatures => "regexp-features",
            Self::ListRendering => "list-rendering",
            Self::DictOrder => "dict-order",
            Self::Collation => "collation",
            Self::ByteStrings => "byte-strings",
            Self::SourceEncoding => "source-encoding",
            Self::Platform => "platform",
            Self::WallClock => "wall-clock",
            Self::Availability(_) => "availability",
        }
    }
}

/// Why an answer is not exact. Each group names the lane of the lift
/// diagram in the interface contract that records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeclineReason {
    // before step 1 · availability
    /// The fact is not computed at this tier, or the function is over the
    /// complexity ceiling: not a negative, and every consumer that would
    /// need it stays silent.
    Unavailable(AnalysisTier),
    // step 1 · resolve
    /// No structural or transfer declaration, or an explicit abstention
    /// (`semantics none`).
    NoSemantics,
    /// A declared `EvalRoute::None`: the specialisation classifies the
    /// command and evaluates nothing, and the payload says why.
    NoRoute(NoRouteReason),
    /// Binding validity: the head, a nested command, or a math function is
    /// renamed, aliased, redefined, or in an opaque namespace.
    RebindingSuspected,
    /// The command has structure but no value (`switch`, a body).
    NotAValue,
    // step 2 · inputs
    /// A word is not an exact value at this use: multi-token, `{*}`, an
    /// unresolvable variable, `JimTcl` `$(…)`.
    NotExact,
    /// The place is `::`-qualified or in the escaping set.
    EscapingPlace,
    /// The place is in the module's traced variables, or every place is
    /// (a dynamic variable trace).
    TracedPlace,
    /// The name is computed (the dynamic-name barrier): a permanent miss.
    DynamicName,
    /// Duplicate or aliased targets, an array-element base write, or a
    /// trace-visible target: a precision limit, not a soundness rule.
    OverlappingTargets,
    // step 4 · finite sets
    /// More than one distinct SSA value is a set.
    CorrelatedSets,
    /// Combinations or result bytes over the cap.
    TooManyMembers,
    // step 5 · evaluate
    /// The route does not model this form, option, or operation.
    Unsupported,
    /// The operation needs a bound place and existence is not proven, or
    /// no release in the profile completes normally on an absent one.
    UnboundPlace,
    /// The prior value has the wrong shape for the operation: the program
    /// errors at run time, and an error is never a value.
    WrongRepresentation,
    /// The value's bytes are not text where a core needs a string; never
    /// U+FFFD.
    NotText,
    /// A nested substitution's effects reach an observed input and the
    /// ordered evaluation state cannot own them.
    StatefulNested,
    /// The answers differ between the profile's releases on one axis.
    ReleaseAmbiguous(Axis),
    /// Fuel, depth, bytes before allocation, the request budget, or
    /// cancellation — distinct from `Unsupported`, never a negative.
    Budget(BudgetLimit),
    /// A regexp search cut short, or an approximate capture.
    Approximate,
    /// A nested query would demand the query being computed.
    Cycle,
    /// The host is unavailable or quarantined; never cached as
    /// `Unsupported`.
    Transient,
    // step 6 · validate
    /// Cardinality, index, overlap, effect, dependency, or limit check
    /// failed: a pack or implementation failure with a notice.
    MalformedAnswer,
}

impl DeclineReason {
    /// Stable spelling for the Explorer and the inventory; the payload of
    /// a carrying variant is rendered by the caller when it matters.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable(_) => "unavailable",
            Self::NoSemantics => "no-semantics",
            Self::NoRoute(_) => "no-route",
            Self::RebindingSuspected => "rebinding-suspected",
            Self::NotAValue => "not-a-value",
            Self::NotExact => "not-exact",
            Self::EscapingPlace => "escaping-place",
            Self::TracedPlace => "traced-place",
            Self::DynamicName => "dynamic-name",
            Self::OverlappingTargets => "overlapping-targets",
            Self::CorrelatedSets => "correlated-sets",
            Self::TooManyMembers => "too-many-members",
            Self::Unsupported => "unsupported",
            Self::UnboundPlace => "unbound-place",
            Self::WrongRepresentation => "wrong-representation",
            Self::NotText => "not-text",
            Self::StatefulNested => "stateful-nested",
            Self::ReleaseAmbiguous(_) => "release-ambiguous",
            Self::Budget(_) => "budget",
            Self::Approximate => "approximate",
            Self::Cycle => "cycle",
            Self::Transient => "transient",
            Self::MalformedAnswer => "malformed-answer",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The contract lists the reasons by lift step; every variant has one
    /// spelling and no two variants share it.
    #[test]
    fn every_reason_has_a_distinct_spelling() {
        let reasons = [
            DeclineReason::Unavailable(AnalysisTier::Deep),
            DeclineReason::NoSemantics,
            DeclineReason::NoRoute(NoRouteReason::Unauthored),
            DeclineReason::RebindingSuspected,
            DeclineReason::NotAValue,
            DeclineReason::NotExact,
            DeclineReason::EscapingPlace,
            DeclineReason::TracedPlace,
            DeclineReason::DynamicName,
            DeclineReason::OverlappingTargets,
            DeclineReason::CorrelatedSets,
            DeclineReason::TooManyMembers,
            DeclineReason::Unsupported,
            DeclineReason::UnboundPlace,
            DeclineReason::WrongRepresentation,
            DeclineReason::NotText,
            DeclineReason::StatefulNested,
            DeclineReason::ReleaseAmbiguous(Axis::IntTower),
            DeclineReason::Budget(BudgetLimit::Fuel),
            DeclineReason::Approximate,
            DeclineReason::Cycle,
            DeclineReason::Transient,
            DeclineReason::MalformedAnswer,
        ];
        let spellings: std::collections::BTreeSet<&str> =
            reasons.iter().map(|r| r.as_str()).collect();
        assert_eq!(spellings.len(), reasons.len());
    }
}
