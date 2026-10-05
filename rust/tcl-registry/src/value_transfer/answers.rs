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

//! What a specialisation answers: the structural plan, the per-domain
//! transfer, and the exact evaluation
//! (`docs/design/compiler/value-transfers.md` § *The interface*). Every
//! variant says who produces it and who consumes it.

use tcl_dialect::model::SpecSurface;
use tcl_dialect::{NumberSyntax, StringCharacterModel, TclVersion};
use tcl_lexer::Span;

use crate::completion::{CompletionCode, CompletionCodeDomain};
use crate::frame_effect::FrameLevel;
use crate::native_lowering::CellUpdate;
use crate::spec::CaseMatchMode;
use crate::substitution::SubstitutionKinds;
use crate::taint::TaintColour;
use crate::types::TclType;
use crate::world_effect::EffectFootprint;

use super::context::BindingIdentity;
use super::decline::DeclineReason;
use super::inputs::{BodyRegion, OperandId, PlaceRef, TargetId};
use super::iteration::LoopResult;
use super::route::EvalRoute;

/// The structural plan for one invocation. Produced by
/// `CommandSemantics::structure`; consumed by lowering, the CFG builder,
/// SSA construction, and the solvers that need the shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanAnswer {
    /// A read-modify-write of one place, derived from
    /// `NativeLowering::CellReadModifyWrite`.
    CellReadModifyWrite {
        /// The place read and written.
        target: TargetId,
        /// The operation applied to the cell.
        operation: CellUpdate,
        /// The value operand, when the form has one.
        amount: Option<OperandId>,
        /// The surfaces on which an absent cell is created: every surface
        /// for `append` and `lappend`, 8.5 onwards for `incr`, none where
        /// the operation raises.
        creates_absent: Option<&'static [SpecSurface]>,
    },
    /// A body run in a described scope.
    Body {
        /// The names bound in the body's scope.
        binders: Vec<Binder>,
        /// The body and its frame.
        body: BodyPlan,
        /// What is written back when the body completes.
        reconcile: Reconcile,
        /// How the body's completion becomes the command's.
        completion: CompletionProtocol,
    },
    /// An iteration protocol.
    Iterate(IterationPlan),
    /// A case list with its selection contract: produced from the
    /// command's `CaseListSpec` for `switch`
    /// ([`super::selection::SwitchSemantics`]); consumed by the `Selection`
    /// transfer, O112, `switch_body_is_selected`, `static_loops::exec_switch`
    /// and I231.
    CaseList {
        /// The subject operand.
        subject: OperandId,
        /// The arms, in order, where the invocation writes them.
        arms: CaseArms,
        /// The fall-through body's spelling (`-`), when the grammar has
        /// one: an arm whose body it is runs the next arm's body.
        fallthrough: Option<&'static str>,
        /// The selection semantics.
        selection: SelectionContract,
    },
    /// A template word — the final argument of a substituting command with
    /// the kinds that run over it.
    TemplateWord(TemplateWordPlan),
    /// An ordinary argv invocation with no structure of its own.
    NoStructure,
    /// The structure is declared unsupported or cannot be read for this
    /// call: the generic conservative shape is kept.
    Declined(DeclineReason),
}

/// What a bound place is. `array exists` distinguishes them; a parameter is
/// always `Scalar`; `Either` is the join of the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingKind {
    /// A scalar binding.
    Scalar,
    /// An array binding.
    Array,
    /// Either kind: the join.
    Either,
}

/// Where a binder's name comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinderName {
    /// An operand word names the binder.
    Operand(OperandId),
    /// The plan declares the name itself: a synthetic loop header's
    /// per-iteration variable, `try`'s message and options variables.
    Declared(String),
}

/// A name a plan binds in the body's scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binder {
    /// The name.
    pub name: BinderName,
    /// What kind of binding it is.
    pub kind: BindingKind,
}

/// A body operand and the frame it runs in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyPlan {
    /// The body operand.
    pub body: OperandId,
    /// The frame the body runs in.
    pub frame: FrameLevel,
}

/// What a body plan writes back when the body completes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconcile {
    /// Nothing.
    None,
    /// The bound keys into a dict operand.
    WriteBackKeys(OperandId),
    /// The loop's binders.
    Binders,
}

/// What the iterable of an iteration plan is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IterableKind {
    /// A Tcl list operand.
    List(OperandId),
    /// A dict operand.
    Dict(OperandId),
    /// A counted loop: `init`, condition, `next`.
    Counted {
        /// The initialisation script.
        init: OperandId,
        /// The condition expression.
        condition: OperandId,
        /// The step script.
        next: OperandId,
    },
    /// A bare condition.
    Condition(OperandId),
    /// Several lists stepped in lockstep (`foreach a $l1 b $l2 body`): each
    /// group's binders, the next ones of the plan's own in order, take
    /// consecutive elements of its list on every pass, a list too short for
    /// a pass supplying the empty string, and the loop runs until every list
    /// is used up.
    Lockstep(Vec<ListGroup>),
    /// A vendor collection with its declared cardinality operand.
    Vendor {
        /// The collection operand.
        collection: OperandId,
        /// The cardinality operand, when the pack declares one.
        cardinality: Option<OperandId>,
    },
}

/// One list of a lockstep iteration ([`IterableKind::Lockstep`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListGroup {
    /// How many of the plan's binders, after the groups before this one,
    /// this list binds.
    pub binders: usize,
    /// The list operand.
    pub list: OperandId,
}

/// What ends a loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExitRule {
    /// The iterable is exhausted.
    Exhaustion,
    /// The condition is false.
    FalseCondition,
    /// A `break`.
    Break,
    /// A non-normal completion of the body.
    NonNormalCompletion,
    /// The enumeration cap.
    EnumerationCap,
}

/// One iteration protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterationPlan {
    /// The names bound per iteration, in binding order.
    pub binders: Vec<Binder>,
    /// What is iterated.
    pub iterable: IterableKind,
    /// The body, when the layout carries one.
    pub body: Option<BodyPlan>,
    /// What ends the loop.
    pub exit: ExitRule,
    /// Whether the binders are bound on the zero-iteration path.
    pub zero_iterations_bind: bool,
    /// How the body's completion becomes the command's
    /// ([`IterationPlan::step`]).
    pub completion: CompletionProtocol,
    /// What the loop yields when it completes normally.
    pub result: LoopResult,
}

/// How a `try` handler's pattern word selects it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HandlerMatch {
    /// By completion code (`on`).
    CompletionCode,
    /// By `-errorcode` prefix (`trap`).
    ErrorCodePrefix,
}

/// One `try` handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerPlan {
    /// How the pattern selects the handler.
    pub matches: HandlerMatch,
    /// The pattern operand.
    pub pattern: OperandId,
    /// The message and options binders.
    pub binders: Vec<Binder>,
    /// The body, or `None` for a `-` handler sharing the next body.
    pub body: Option<OperandId>,
}

/// How a body plan's completion becomes the command's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionProtocol {
    /// The body's completion is the command's.
    TclBody,
    /// Loop bodies: the listed codes are absorbed, the rest pass
    /// ([`IterationPlan::step`]).
    Absorb(&'static [CompletionCode]),
    /// `catch`: every completion is absorbed.
    CatchAll {
        /// The result variable, when given.
        result_var: Option<TargetId>,
        /// The options variable, when given.
        options_var: Option<TargetId>,
    },
    /// `try`: the first matching handler runs; `finally` runs on every
    /// path.
    Handlers {
        /// The handlers, in order.
        handlers: Vec<HandlerPlan>,
        /// The `finally` body, when given.
        finally: Option<OperandId>,
    },
}

/// Where a case list's arms are written. The page's shape,
/// `Vec<(OperandId, Option<OperandId>)>`, is the inline form's alone: the
/// one-word form's patterns and bodies are elements of a single operand
/// that no `OperandId` names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseArms {
    /// Each pattern and body its own word, in order: `(pattern, body)`, a
    /// `None` body an arm spelled with the fall-through body.
    Words(Vec<(OperandId, Option<OperandId>)>),
    /// One clause-list word whose value, read as a list, pairs up as the
    /// patterns and bodies in order.
    List(OperandId),
}

/// The selection semantics of a case list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionContract {
    /// The comparison mode.
    pub mode: CaseMatchMode,
    /// Whether the comparison ignores case.
    pub nocase: bool,
    /// Whether a final `default` arm is recognised.
    pub final_default: bool,
}

/// A `[…]` region of a braced template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRegion {
    /// The region's span in the template.
    pub span: Span,
    /// The script inside the brackets.
    pub script: BodyRegion,
}

/// A `$name`, `${name}`, or `$arr(k)` read of a braced template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableRead {
    /// The read's span in the template.
    pub span: Span,
    /// The variable name.
    pub name: String,
    /// The element key, for an array read.
    pub element: Option<String>,
}

/// The final argument of a substituting command, with what runs over it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateWordPlan {
    /// The template operand.
    pub operand: OperandId,
    /// The kinds that run.
    pub kinds: SubstitutionKinds,
    /// Whether the template is a braced word.
    pub braced: bool,
    /// Whether the template reached the command already substituted.
    pub dynamic: bool,
    /// The `[…]` regions, in template order.
    pub script_regions: Vec<ScriptRegion>,
    /// The variable reads outside the script regions, in order.
    pub reads: Vec<VariableRead>,
    /// The backslash escapes that materialise.
    pub escapes: Vec<Span>,
}

/// The registry-described operation the interval domain interprets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RangeModel {
    /// An integer add under the target's overflow rules.
    IntegerAdd,
    /// A non-negative length.
    Length,
    /// A single point.
    Point,
    /// No abstract model.
    Top,
}

/// Exact prefix, suffix, or length facts around an unknown segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentFacts {
    /// The exact prefix, when known.
    pub prefix: Option<Vec<u8>>,
    /// The exact suffix, when known.
    pub suffix: Option<Vec<u8>>,
    /// The least possible length, when known.
    pub min_len: Option<usize>,
    /// The greatest possible length, when known.
    pub max_len: Option<usize>,
}

/// Colour flow through writes and the sanitiser identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaintTransfer {
    /// The colour each written target takes.
    pub per_target: Vec<(TargetId, TaintColour)>,
    /// The sanitiser the command is, when it is one.
    pub sanitiser: Option<&'static str>,
}

/// Internal-representation evidence, kept apart from the exact value and
/// from the semantic type: identical strings produced with different
/// representations stay exact while the representation is uncertain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RepresentationEvidence {
    /// No evidence.
    #[default]
    Unknown,
    /// The route constructed the value with this internal representation.
    Constructed(TclType),
}

/// One selected-edge fact per member of a finite subject, produced by the
/// `Selection` transfer ([`super::selection::SwitchSemantics`]); consumed
/// by the selection record and the consumers that read it. The three
/// vectors run in member order, one entry per member.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectionFact {
    /// The arm whose pattern matched — a final `default` among them — or
    /// `None` when no pattern did and no body runs.
    pub selected: Vec<Option<usize>>,
    /// The arm whose body runs: the selected arm's, or through a
    /// fall-through body the next arm's that has one; `None` with
    /// `selected`. It is recorded beside `selected` because a pattern that
    /// never matches can still supply the body a preceding `-` arm reaches.
    pub bodies: Vec<Option<usize>>,
    /// The writes the selection makes before its body runs — the
    /// `-indexvar` value, then the `-matchvar` one, each a `Write` of its
    /// option's variable word — in the command's order.
    pub writes: Vec<Vec<StoreOutcome>>,
}

/// The existence fact at one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Existence {
    /// The solver has not reached the place.
    Pending,
    /// Provably no binding.
    Unbound,
    /// Provably bound, with the kind when it is proven.
    Bound(BindingKind),
    /// Bound on some path and not on another, or unknowable here.
    MayBound,
}

impl BindingKind {
    /// The join of two kinds: equal kinds stay, a scalar and an array are
    /// `Either`.
    #[must_use]
    pub const fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Scalar, Self::Scalar) => Self::Scalar,
            (Self::Array, Self::Array) => Self::Array,
            _ => Self::Either,
        }
    }
}

impl Existence {
    /// The lattice join (`docs/design/compiler/value-transfers.md`
    /// § *Existence*): `Pending` is the identity, equal facts stay, two
    /// bound kinds join as kinds, and `Unbound` beside `Bound(_)`, or
    /// anything beside `MayBound`, is `MayBound`.
    #[must_use]
    pub const fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Pending, fact) | (fact, Self::Pending) => fact,
            (Self::Unbound, Self::Unbound) => Self::Unbound,
            (Self::Bound(left), Self::Bound(right)) => Self::Bound(left.join(right)),
            (Self::MayBound, _)
            | (_, Self::MayBound)
            | (Self::Unbound, Self::Bound(_))
            | (Self::Bound(_), Self::Unbound) => Self::MayBound,
        }
    }

    /// The fact after `outcome` runs on a place that held `self`: a bind
    /// is bound, an unbind unbound, a preserve keeps the fact, and a
    /// may-bind joins it with the bound kind.
    #[must_use]
    pub const fn after(self, outcome: ExistenceOutcome) -> Self {
        match outcome {
            ExistenceOutcome::Bind(kind) => Self::Bound(kind),
            ExistenceOutcome::Unbind => Self::Unbound,
            ExistenceOutcome::Preserve => self,
            ExistenceOutcome::MayBind(kind) => self.join(Self::Bound(kind)),
        }
    }
}

/// The per-target existence delta on one completion path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExistenceOutcome {
    /// The place is bound afterwards — a write.
    Bind(BindingKind),
    /// Untouched — a preserve.
    Preserve,
    /// Unbound afterwards — an unbind.
    Unbind,
    /// A may-write, joined with the prior fact.
    MayBind(BindingKind),
}

/// One completion path's storage consequences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionPath {
    /// The kind of completion, not an instance.
    pub completion: CompletionCodeDomain,
    /// Per target, in execution order; a target absent here is preserved.
    pub outcomes: Vec<(TargetId, ExistenceOutcome)>,
}

/// The per-target existence delta, indexed by completion path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistenceTransfer {
    /// The paths the invocation can complete on.
    pub paths: Vec<CompletionPath>,
}

/// List length, dict key set, or element type when proven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueShape {
    /// A list of exactly this many elements.
    ListLength(usize),
    /// A dict with exactly these keys.
    DictKeys(Vec<String>),
    /// Every element has this internal representation.
    ElementType(TclType),
}

/// Semantic types and shapes for a result and each written place.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeFacts {
    /// The result's internal representation, when declared.
    pub result: Option<TclType>,
    /// Per written target.
    pub per_target: Vec<(TargetId, TclType)>,
    /// Per target shape, when proven.
    pub shapes: Vec<(TargetId, ValueShape)>,
}

/// What a may-write or an unavailable value still knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactBounds {
    /// The existence fact.
    pub existence: Existence,
    /// The internal representation, when known.
    pub intrep: Option<TclType>,
    /// The shape, when known.
    pub shape: Option<ValueShape>,
    /// Segment facts, when known.
    pub segments: Option<SegmentFacts>,
    /// The taint transfer, when known.
    pub taint: Option<TaintTransfer>,
}

/// The abstract transfer for one domain. Produced by
/// `CommandSemantics::transfer`; consumed by the solver that owns the
/// domain.
#[derive(Debug, Clone, PartialEq)]
pub enum TransferAnswer {
    /// No domain-specific model: the driver applies the generic
    /// conservative transfer derived from the declared effects.
    Generic,
    /// Result and per-target semantic types and shapes.
    Type(TypeFacts),
    /// Per-target existence outcomes, indexed by completion path.
    Existence(ExistenceTransfer),
    /// The registry-described operation the interval domain interprets.
    Range(RangeModel),
    /// Exact prefix, suffix, or length facts around an unknown segment.
    Segments(SegmentFacts),
    /// Colour flow through writes and the sanitiser identity.
    Taint(TaintTransfer),
    /// Guaranteed construction and conversion of the result's intrep.
    Representation(RepresentationEvidence),
    /// One selected-edge fact per member of a finite subject.
    Selection(SelectionFact),
    /// The effect delta beyond the declared footprint.
    Effects(EffectFootprint),
    /// The completion codes this call can take.
    Completion(CompletionCodeDomain),
    /// This domain declines for this call, for a recorded reason.
    Declined(DeclineReason),
}

/// The numeric classification of an exact value: an additional fact, never
/// a respelling of the bytes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumericValue {
    /// A canonically spelled integer.
    Int(i64),
    /// A double.
    Float(f64),
    /// A boolean.
    Bool(bool),
}

/// A Tcl string preserved byte for byte, with its representation evidence
/// carried separately. Produced by the routes' value ingress; consumed by
/// every value consumer.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactValue {
    /// The value's bytes, exactly as computed: whitespace, NUL,
    /// backslashes, leading zeros, and signed zero included.
    pub bytes: Vec<u8>,
    /// The numeric classification, when the spelling has one.
    pub numeric: Option<NumericValue>,
    /// The representation evidence.
    pub representation: RepresentationEvidence,
}

impl ExactValue {
    /// A text value with no numeric classification.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            bytes: text.into().into_bytes(),
            numeric: None,
            representation: RepresentationEvidence::Unknown,
        }
    }

    /// An integer value, spelled canonically.
    #[must_use]
    pub fn int(value: i64) -> Self {
        Self {
            bytes: value.to_string().into_bytes(),
            numeric: Some(NumericValue::Int(value)),
            representation: RepresentationEvidence::Unknown,
        }
    }

    /// The value ingress for a literal: the text kept exactly, classified
    /// as an integer only when the canonical integer spelling round-trips
    /// (`str(int(s)) == s`). A leading-zero literal such as `08` or `010`
    /// parses as a number but does not round-trip, so it stays text and the
    /// dialect's leading-zero rule applies later; `+5` and `-0` stay text
    /// for the same reason. Nothing is trimmed: `{ a }` is the
    /// three-character string.
    #[must_use]
    pub fn from_literal(text: &str) -> Self {
        let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
        let is_decimal_int = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
        let numeric = if is_decimal_int {
            text.parse::<i64>()
                .ok()
                .filter(|i| i.to_string() == text)
                .map(NumericValue::Int)
        } else {
            None
        };
        Self {
            bytes: text.as_bytes().to_vec(),
            numeric,
            representation: RepresentationEvidence::Unknown,
        }
    }

    /// The bytes as text, or `NotText` when they are not UTF-8; never
    /// U+FFFD.
    pub fn as_str(&self) -> Result<&str, DeclineReason> {
        std::str::from_utf8(&self.bytes).map_err(|_| DeclineReason::NotText)
    }

    /// The integer classification, when the value has one.
    #[must_use]
    pub const fn as_int(&self) -> Option<i64> {
        match self.numeric {
            Some(NumericValue::Int(i)) => Some(i),
            _ => None,
        }
    }
}

/// A value the route may prove, or may only bound.
#[derive(Debug, Clone, PartialEq)]
pub enum ExactValueOrUnavailable {
    /// The exact value.
    Exact(ExactValue),
    /// The route proved the path but not the text.
    Unavailable(FactBounds),
}

impl ExactValueOrUnavailable {
    /// A string the route proves nothing of but that it is a string: bound,
    /// typed as a string, no text. What an error's message or `-errorcode`
    /// is where the route does not prove it under every release the profile
    /// names.
    #[must_use]
    pub const fn unproven_string() -> Self {
        Self::Unavailable(FactBounds {
            existence: Existence::Bound(BindingKind::Scalar),
            intrep: Some(TclType::String),
            shape: None,
            segments: None,
            taint: None,
        })
    }

    /// The exact text `text`.
    #[must_use]
    pub fn exact_text(text: impl Into<String>) -> Self {
        Self::Exact(ExactValue::text(text))
    }
}

/// How an evaluated invocation completes.
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionOutcome {
    /// `TCL_OK`: every ordered store ran, the result is the command's.
    Normal,
    /// A completion by `return`, `break`, `continue` or a numeric `-code`,
    /// as the return options state it. A caller observes `TCL_RETURN` while
    /// `level` is positive and `code` itself at level 0
    /// ([`CompletionOutcome::observed_code`]).
    Code {
        /// The `-code` the completion carries.
        code: CompletionCode,
        /// The `-level`.
        level: u32,
        /// The result on that path.
        result: ExactValueOrUnavailable,
    },
    /// `TCL_ERROR` after the first `written` stores ran.
    Error {
        /// How many ordered stores ran before the error.
        written: usize,
        /// The error message, when the route proves it.
        message: ExactValueOrUnavailable,
        /// The `-errorcode`, when the route proves it.
        error_code: ExactValueOrUnavailable,
    },
}

impl CompletionOutcome {
    /// `TCL_ERROR` after `written` stores, with the message and the
    /// `-errorcode` unproven.
    #[must_use]
    pub const fn error_unproven(written: usize) -> Self {
        Self::Error {
            written,
            message: ExactValueOrUnavailable::unproven_string(),
            error_code: ExactValueOrUnavailable::unproven_string(),
        }
    }

    /// Whether the invocation completed normally.
    #[must_use]
    pub const fn is_normal(&self) -> bool {
        matches!(self, Self::Normal)
    }

    /// The completion code a caller observes, which is what `catch` returns:
    /// 0 for a normal completion, 1 for an error, `TCL_RETURN` for a code
    /// completion that still has a level to climb, and its own code at
    /// level 0.
    #[must_use]
    pub fn observed_code(&self) -> i64 {
        match self {
            Self::Normal => 0,
            Self::Error { .. } => 1,
            Self::Code { code, level, .. } => {
                if *level > 0 {
                    CompletionCode::Return.as_int()
                } else {
                    code.as_int()
                }
            }
        }
    }

    /// The `-code` and `-level` the return options state for the
    /// completion: 0 and 0 for a normal one, 1 and 0 for an error, a code
    /// completion's own otherwise.
    #[must_use]
    pub fn options_code_and_level(&self) -> (i64, u32) {
        match self {
            Self::Normal => (0, 0),
            Self::Error { .. } => (1, 0),
            Self::Code { code, level, .. } => (code.as_int(), *level),
        }
    }
}

/// What happens to one place.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreOutcome {
    /// The place holds exactly `value` afterwards.
    Write {
        /// The target.
        target: TargetId,
        /// The written value.
        value: ExactValue,
    },
    /// The place is untouched: value, existence, and representation.
    Preserve {
        /// The target.
        target: TargetId,
    },
    /// The place is unbound afterwards.
    Unbind {
        /// The target.
        target: TargetId,
    },
    /// The place may have been written; what is known is bounded.
    MayWrite {
        /// The target.
        target: TargetId,
        /// What is still known.
        facts: FactBounds,
    },
    /// The place holds a value afterwards of which the route proved the
    /// binding and the bounds but not the text: `catch`'s options
    /// dictionary. Unlike a [`Self::MayWrite`] the write certainly happened,
    /// so the place is bound as `facts.existence` says whatever it held
    /// before.
    WriteUnavailable {
        /// The target.
        target: TargetId,
        /// What is known of the value.
        facts: FactBounds,
    },
    /// One element of the array `target` names holds exactly `value`
    /// afterwards: a whole-array writer's pairs (`array set arr {k v}`),
    /// whose places are elements no operand spells. The place is the
    /// element `key` of the target's place.
    WriteElement {
        /// The target naming the array.
        target: TargetId,
        /// The element key, exactly.
        key: String,
        /// The written value.
        value: ExactValue,
    },
}

impl StoreOutcome {
    /// The target of the outcome.
    #[must_use]
    pub const fn target(&self) -> TargetId {
        match self {
            Self::Write { target, .. }
            | Self::Preserve { target }
            | Self::Unbind { target }
            | Self::MayWrite { target, .. }
            | Self::WriteUnavailable { target, .. }
            | Self::WriteElement { target, .. } => *target,
        }
    }

    /// The element key an element write names within its target, `None`
    /// for an outcome on the target's own place.
    #[must_use]
    pub fn element(&self) -> Option<&str> {
        match self {
            Self::WriteElement { key, .. } => Some(key),
            Self::Write { .. }
            | Self::Preserve { .. }
            | Self::Unbind { .. }
            | Self::MayWrite { .. }
            | Self::WriteUnavailable { .. } => None,
        }
    }
}

/// The route, its implementation identity, and its revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RouteIdentity {
    /// The route.
    pub route: EvalRoute,
    /// The implementation identity.
    pub implementation: &'static str,
    /// The implementation revision.
    pub revision: u64,
}

/// Every assumption an answer rests on. Produced with the answer; consumed
/// by the memo key, by re-proof before emission, and by the Explorer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DependencyEvidence {
    /// The bindings used, transitively: the head, every nested command,
    /// every math function.
    pub bindings: Vec<BindingIdentity>,
    /// The places read, with the store version read.
    pub reads: Vec<(PlaceRef, u32)>,
    /// The route the answer came through, when it came through one.
    pub route: Option<RouteIdentity>,
    /// The numeral grammar the answer depended on, when it did.
    pub numerals: Option<NumberSyntax>,
    /// The character model the answer depended on, when it did.
    pub characters: Option<StringCharacterModel>,
    /// The release the answer depended on, when it did.
    pub release: Option<TclVersion>,
}

/// The evaluated outcome on the path the exact inputs select.
#[derive(Debug, Clone, PartialEq)]
pub struct InvocationOutcome {
    /// How the call completes with these inputs.
    pub completion: CompletionOutcome,
    /// The command result on that path.
    pub result: ExactValueOrUnavailable,
    /// The stores the invocation's own substitutions made before it
    /// completed — the ordered evaluation state's writes (§ `expr`), in
    /// execution order, each already resolved to its place, because a
    /// store's target names an operand of the invocation that ran it and
    /// these ran in others. Applied ahead of [`Self::ordered_stores`]; empty
    /// unless the evaluation admitted nested writes.
    pub nested_writes: Vec<(PlaceRef, StoreOutcome)>,
    /// In execution order, over validated targets, aliases reconciled; on
    /// an error completion only the first `written` ran, counted across
    /// [`Self::nested_writes`] and then these.
    pub ordered_stores: Vec<StoreOutcome>,
    /// Semantic type and shape facts for the result and each written place.
    pub types: TypeFacts,
    /// Every dependency the answer used.
    pub evidence: DependencyEvidence,
}

impl InvocationOutcome {
    /// Whether the outcome writes, unbinds, or may write any place, its
    /// substitutions' writes included.
    #[must_use]
    pub fn has_stores(&self) -> bool {
        self.ordered_stores
            .iter()
            .chain(self.nested_writes.iter().map(|(_, store)| store))
            .any(|store| !matches!(store, StoreOutcome::Preserve { .. }))
    }
}

/// The driver's structural checks on one outcome before anything publishes:
/// every store names a target the invocation declares — one of `targets`,
/// or the place a plan itself names — and no target has two outcomes; the
/// type facts name only those targets; and an error completion ran no more
/// stores than it lists. Two targets resolving to one place (`lassign …
/// a a`) are two outcomes composed in order once the driver resolves them
/// to places, which is also where an element and its array's base, or a
/// traced or escaping place, are refused.
///
/// # Errors
///
/// `MalformedAnswer` for an outcome that fails a check: a pack or
/// implementation failure, never a partial lattice update.
pub fn validate_outcome(
    plan: &PlanAnswer,
    targets: &[TargetId],
    outcome: &InvocationOutcome,
) -> Result<(), DeclineReason> {
    let planned = match plan {
        PlanAnswer::CellReadModifyWrite { target, .. } => Some(*target),
        PlanAnswer::Body {
            reconcile: Reconcile::WriteBackKeys(dict),
            ..
        } => Some(TargetId(*dict)),
        _ => None,
    };
    let declared = |target: &TargetId| targets.contains(target) || planned == Some(*target);
    // One outcome per place the answer names: a target's own place, or one
    // element of it for an element write.
    let mut seen: Vec<(TargetId, Option<&str>)> = Vec::with_capacity(outcome.ordered_stores.len());
    for store in &outcome.ordered_stores {
        let slot = (store.target(), store.element());
        if !declared(&slot.0) || seen.contains(&slot) {
            return Err(DeclineReason::MalformedAnswer);
        }
        seen.push(slot);
    }
    let typed = outcome
        .types
        .per_target
        .iter()
        .map(|(target, _)| target)
        .chain(outcome.types.shapes.iter().map(|(target, _)| target));
    for target in typed {
        if !declared(target) {
            return Err(DeclineReason::MalformedAnswer);
        }
    }
    if let CompletionOutcome::Error { written, .. } = outcome.completion
        && written > outcome.nested_writes.len() + outcome.ordered_stores.len()
    {
        return Err(DeclineReason::MalformedAnswer);
    }
    Ok(())
}

/// The exact answer. Produced by every route; consumed by the transfer
/// driver, which validates it before anything is published.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalAnswer {
    /// An input has not reached a usable fact; the answer stays pending.
    Pending,
    /// No exact fact, for a recorded reason. Never a speculative value.
    Declined(DeclineReason),
    /// The evaluated outcome on the path the exact inputs select.
    Evaluated(Box<InvocationOutcome>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_ingress_preserves_the_exact_text() {
        for text in [" a ", "a\0b", "a\\b", "\t42\n", "hello", "", "  "] {
            let value = ExactValue::from_literal(text);
            assert_eq!(value.bytes, text.as_bytes(), "{text:?}");
            assert_eq!(value.numeric, None, "{text:?}");
        }
    }

    #[test]
    fn a_code_completion_states_what_a_caller_observes() {
        let code = |code, level| CompletionOutcome::Code {
            code,
            level,
            result: ExactValueOrUnavailable::exact_text(""),
        };
        let observed = |completion: &CompletionOutcome| {
            (
                completion.observed_code(),
                completion.options_code_and_level(),
            )
        };
        assert_eq!(observed(&CompletionOutcome::Normal), (0, (0, 0)));
        assert_eq!(observed(&CompletionOutcome::error_unproven(2)), (1, (1, 0)));
        // A level still to climb is `TCL_RETURN` whatever the code.
        assert_eq!(observed(&code(CompletionCode::Ok, 1)), (2, (0, 1)));
        assert_eq!(observed(&code(CompletionCode::Other(5), 1)), (2, (5, 1)));
        assert_eq!(observed(&code(CompletionCode::Break, 2)), (2, (3, 2)));
        // At level 0 the code is the completion.
        assert_eq!(observed(&code(CompletionCode::Break, 0)), (3, (3, 0)));
        assert_eq!(observed(&code(CompletionCode::Continue, 0)), (4, (4, 0)));
        assert_eq!(observed(&code(CompletionCode::Other(-7), 0)), (-7, (-7, 0)));
    }

    #[test]
    fn literal_ingress_classifies_only_round_tripping_integers() {
        assert_eq!(ExactValue::from_literal("42").as_int(), Some(42));
        assert_eq!(ExactValue::from_literal("-5").as_int(), Some(-5));
        assert_eq!(ExactValue::from_literal("0").as_int(), Some(0));
        for text in ["08", "010", "+5", "-0", " 5", "5 ", "1_000", "0x10"] {
            assert_eq!(ExactValue::from_literal(text).as_int(), None, "{text:?}");
        }
    }
}
