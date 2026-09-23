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

//! The declared way an exact answer is computed
//! (`docs/design/compiler/value-evaluation.md` § *Three routes, declared on
//! the spec*). A route is a capability the spec names; purity never selects
//! one.

use super::decline::NoRouteReason;

/// The declared way an exact answer is computed. Resolved once per
/// invocation, from the spec's three declaration states at command,
/// subcommand, and form scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvalRoute {
    /// A registry-owned Rust function over the shared cores, named by its
    /// catalogue id.
    Direct {
        /// The catalogue identity of the evaluator.
        id: NativeEvalId,
    },
    /// The shared expression engine under a named language profile.
    Expression {
        /// The language whose arithmetic the engine runs under.
        language: LanguageProfileId,
    },
    /// An implementation the spec names, run in the bounded host.
    Implementation(EvaluatorCapability),
    /// Declared absence: classification only. The driver answers
    /// `Declined(NoRoute)` without consulting purity.
    None {
        /// Why there is no route.
        reason: NoRouteReason,
    },
}

impl EvalRoute {
    /// Whether the route evaluates anything at all.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::None { .. })
    }

    /// Stable spelling of the route family for the inventory and the
    /// Explorer.
    #[must_use]
    pub const fn family(self) -> &'static str {
        match self {
            Self::Direct { .. } => "direct",
            Self::Expression { .. } => "expression",
            Self::Implementation(_) => "implementation",
            Self::None { .. } => "none",
        }
    }
}

/// The language profile an expression route evaluates under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageProfileId {
    /// Tcl `expr` arithmetic under the target's numeric tower.
    TclExpr,
    /// BPF-Tcl arithmetic: fixed width, truncating division.
    BpfExpr,
}

impl LanguageProfileId {
    /// Stable spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TclExpr => "tcl.expr",
            Self::BpfExpr => "bpf.expr",
        }
    }
}

/// An implementation the spec names for the bounded engine. The capability
/// declaration — inputs, dependencies, budget, exactness — lands with the
/// declared-implementation route; the identity is what a route can carry
/// before then, so the enumeration is closed now and consumers do not
/// change when the declaration fills in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EvaluatorCapability {
    /// The implementation identity, in the pack's `SCOPE::FIELD` form.
    pub identity: &'static str,
}

/// The catalogue of registry-named direct evaluators.
///
/// The registry owns the catalogue; who implements each entry is stated by
/// [`Self::owner`]. A transitional entry is implemented by the compiler's
/// value-transfer driver until the slice that retires it, and the migration
/// plan's ledger lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NativeEvalId {
    /// The cell increment behind `incr`: read the cell, add under the
    /// target's integer tower, write it back, return the new value.
    CellIncrement,
    /// The cell append behind `append`: read the cell, append the values'
    /// bytes, write it back, return the new value.
    CellAppend,
    /// The cell list-append behind `lappend`: read the cell as a list,
    /// append the values as elements, write it back, return the new value.
    CellListAppend,
    /// The exact-value write behind `set`: write the value and return it,
    /// or return the value the cell holds.
    CellWrite,
    /// `dict set`: the dictionary with a value at a key path.
    DictSet,
    /// `dict unset`: the dictionary without the key at a key path.
    DictUnset,
    /// `dict incr`: the dictionary with one key's integer incremented.
    DictIncr,
    /// `dict append`: the dictionary with strings appended to one key's
    /// value.
    DictAppend,
    /// `dict lappend`: the dictionary with elements appended to one key's
    /// list.
    DictListAppend,
    /// `string range string first last`: the shared string core, with the
    /// index numerals pre-resolved under the target's grammar.
    StringRange,
    /// `list ?arg …?`: the arguments as one canonical list.
    ListOfArgs,
    /// `format template ?arg …?`: the rendered template.
    FormatTemplate,
    /// `llength list`: the element count.
    ListLength,
    /// `string length string`: the character count under the target's
    /// character model.
    StringLength,
}

impl NativeEvalId {
    /// Every catalogued evaluator, in catalogue order.
    pub const ALL: &'static [Self] = &[
        Self::CellIncrement,
        Self::CellAppend,
        Self::CellListAppend,
        Self::CellWrite,
        Self::DictSet,
        Self::DictUnset,
        Self::DictIncr,
        Self::DictAppend,
        Self::DictListAppend,
        Self::StringRange,
        Self::ListOfArgs,
        Self::FormatTemplate,
        Self::ListLength,
        Self::StringLength,
    ];

    /// Stable spelling for the inventory and the Explorer.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CellIncrement => "cell-increment",
            Self::CellAppend => "cell-append",
            Self::CellListAppend => "cell-list-append",
            Self::CellWrite => "cell-write",
            Self::DictSet => "dict-set",
            Self::DictUnset => "dict-unset",
            Self::DictIncr => "dict-incr",
            Self::DictAppend => "dict-append",
            Self::DictListAppend => "dict-lappend",
            Self::StringRange => "string-range",
            Self::ListOfArgs => "list-of-args",
            Self::FormatTemplate => "format-template",
            Self::ListLength => "list-length",
            Self::StringLength => "string-length",
        }
    }

    /// Who implements the evaluator.
    #[must_use]
    pub const fn owner(self) -> EvaluatorOwner {
        match self {
            Self::CellIncrement
            | Self::CellAppend
            | Self::CellListAppend
            | Self::CellWrite
            | Self::DictSet
            | Self::DictUnset
            | Self::DictIncr
            | Self::DictAppend
            | Self::DictListAppend
            | Self::StringRange
            | Self::ListOfArgs
            | Self::ListLength
            | Self::StringLength
            | Self::FormatTemplate => EvaluatorOwner::Registry,
        }
    }
}

/// Who implements a catalogued direct evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvaluatorOwner {
    /// The registry: `CommandSemantics::evaluate` is the implementation.
    Registry,
    /// The compiler's driver, during the migration, with the slice of the
    /// migration plan that retires the handler.
    Transitional {
        /// The delivery slice that moves the implementation into the
        /// registry.
        retires_in_slice: u8,
    },
}
