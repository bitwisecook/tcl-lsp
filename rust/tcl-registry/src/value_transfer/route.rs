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

use tcl_dialect::model::SpecSurface;

use super::const_ops::Needs;
use super::context::BindingIdentity;
use super::decline::{Axis, DeclineReason, NoRouteReason};

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

/// What an option row states about evaluation while the option is present
/// (`-evaluate none`, `-evaluate-reason WORD`): the selected form has no
/// evaluator, and the driver records this decline. A route belongs to a
/// form, so these two flags are the whole of the option-level vocabulary;
/// an option that selects a different evaluator is a `refine` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionEvaluation {
    /// `NoRoute` with the reason: `declared` for a bare `-evaluate none`,
    /// `form_unsupported` or `callback` when the row names one.
    NoRoute(NoRouteReason),
    /// `release_ambiguous`: `ReleaseAmbiguous` on the option's availability
    /// axis.
    ReleaseAmbiguous,
}

impl OptionEvaluation {
    /// The `-evaluate-reason` words and what each records.
    pub const REASONS: &'static [(&'static str, Self)] = &[
        (
            "form_unsupported",
            Self::NoRoute(NoRouteReason::FormUnsupported),
        ),
        ("callback", Self::NoRoute(NoRouteReason::Callback)),
        ("release_ambiguous", Self::ReleaseAmbiguous),
    ];

    /// The option's decline for a bare `-evaluate none`.
    pub const DECLARED: Self = Self::NoRoute(NoRouteReason::Declared);

    /// The decline the driver records when the option is present, given
    /// the option's own availability row: `release_ambiguous` is
    /// `ReleaseAmbiguous` on that row's axis, so an option that declares no
    /// availability of its own has no axis to name and gets `None`.
    #[must_use]
    pub const fn decline(self, surface: Option<SpecSurface>) -> Option<DeclineReason> {
        match (self, surface) {
            (Self::NoRoute(reason), _) => Some(DeclineReason::NoRoute(reason)),
            (Self::ReleaseAmbiguous, Some(surface)) => {
                Some(DeclineReason::ReleaseAmbiguous(Axis::Availability(surface)))
            }
            (Self::ReleaseAmbiguous, None) => None,
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
    /// Every language profile.
    pub const ALL: &'static [Self] = &[Self::TclExpr, Self::BpfExpr];

    /// Stable spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TclExpr => "tcl.expr",
            Self::BpfExpr => "bpf.expr",
        }
    }
}

/// Everything a declared implementation states about itself
/// (`docs/design/compiler/value-evaluation.md` § *The capability
/// declaration*). Part of the route, so of the specialisation's identity and
/// of every memo key: two declarations that differ in any field — the body's
/// content hash, one input, one dependency, the budget — are two routes.
///
/// The page's shape with the tree's two constraints: [`EvalRoute`] is
/// `Copy`, so the lists are `&'static` slices the loader leaks as it leaks
/// every other pack field; and the registry does not depend on
/// `tcl-engine-api`, so the budget is the registry-side
/// [`ImplementationBudget`] the host converts and caps by its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EvaluatorCapability {
    /// Which implementation this evaluator models, and at what revision.
    pub identity: ImplementationIdentity,
    /// Where it runs.
    pub host: HostKind,
    /// The target axes it supports, as the bits the direct route admits. An
    /// axis absent here is one the evaluator declines; `PLATFORM` and
    /// `WALL_CLOCK` are never satisfiable, because the host denies both.
    pub target: Needs,
    /// Exactly the inputs it reads, in the order its body's parameters bind
    /// them. Nothing outside this list is supplied.
    pub inputs: &'static [DeclaredInput],
    /// The context dependencies the answer carries and the memo key holds,
    /// in declaration order.
    pub depends: &'static [ContextDependency],
    /// Its own budget, capped by the host's and charged to the request.
    pub budget: ImplementationBudget,
    /// Which completions it models.
    pub completion: CompletionSupport,
}

/// Which implementation a declared evaluator models: the pack, the declared
/// id, and the content hash of the body, so an edited body is a different
/// implementation even under the same id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImplementationIdentity {
    /// The pack that declares it.
    pub pack: &'static str,
    /// The declared id (`tenant.label.v1`).
    pub id: &'static str,
    /// The content hash of the body.
    pub content_hash: u64,
}

/// Where a declared implementation runs. One word today; the variant exists
/// so a second host is a declaration rather than a reinterpretation of the
/// first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostKind {
    /// The bounded Tcl engine behind the hook host.
    BoundedTcl,
}

impl HostKind {
    /// Every host word.
    pub const ALL: &'static [Self] = &[Self::BoundedTcl];

    /// The DSL spelling (`-host bounded_tcl`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BoundedTcl => "bounded_tcl",
        }
    }
}

/// How exact a declared operand input must be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Exactness {
    /// The operand's exact value: a body is never invoked with a
    /// placeholder.
    Exact,
}

impl Exactness {
    /// Every exactness word.
    pub const ALL: &'static [Self] = &[Self::Exact];

    /// The DSL spelling (`arg 0 exact`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
        }
    }
}

/// One input a declared implementation reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeclaredInput {
    /// Operand `index` must be an exact value (`arg N exact`).
    Operand {
        /// The operand index.
        index: usize,
        /// How exact it must be.
        exactness: Exactness,
    },
    /// The incoming value and existence of target `index`
    /// (`target N incoming`).
    IncomingTarget {
        /// The target's operand index.
        index: usize,
    },
    /// The value of option `name`, when present (`option -NAME exact`).
    OptionValue {
        /// The option, as written.
        name: &'static str,
    },
}

/// One context dependency an answer carries and the memo key holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContextDependency {
    /// The target profile (`tcl_profile`).
    TclProfile,
    /// The implementation identity (`implementation_identity`).
    ImplementationIdentity,
    /// The registry and overlay generation (`registry_generation`).
    RegistryGeneration,
    /// The evaluator generation (`evaluator_generation`).
    EvaluatorGeneration,
    /// One named command or math-function binding (`binding NAME`).
    Binding(BindingIdentity),
}

impl ContextDependency {
    /// The fieldless dependency words, in the DSL's order.
    pub const WORDS: &'static [Self] = &[
        Self::TclProfile,
        Self::ImplementationIdentity,
        Self::RegistryGeneration,
        Self::EvaluatorGeneration,
    ];

    /// The DSL spelling (`depends {tcl_profile …}`); `binding` for a named
    /// binding, whose name follows it.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::TclProfile => "tcl_profile",
            Self::ImplementationIdentity => "implementation_identity",
            Self::RegistryGeneration => "registry_generation",
            Self::EvaluatorGeneration => "evaluator_generation",
            Self::Binding(_) => "binding",
        }
    }
}

/// A declared implementation's own per-evaluation budget: the registry-side
/// mirror of `tcl_engine_api::Budget`. It narrows the host's, never widens
/// it — the host converts it and caps each field by its own. `None` leaves
/// the host's value for that field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ImplementationBudget {
    /// Dispatched commands (`-commands N`).
    pub commands: Option<u64>,
    /// Wall clock, in milliseconds (`-wall-clock MS`).
    pub wall_clock_ms: Option<u64>,
    /// The largest value the body may build, in bytes (`-value-bytes N`).
    pub value_bytes: Option<u64>,
}

/// Which completions a declared implementation models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionSupport {
    /// The normal path only: an implementation that raises declines, under
    /// the DSL's "error means abstain" rule, and is never a completion fact.
    NormalOnly,
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
