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

//! Native lowering: the target-neutral projection of the executable semantic
//! IR into the native lowered IR (NLIR), the representation and cell-storage
//! lattices over it, and the framing-elision decisions — plan §3.3–§3.5 and
//! §7 row P3.
//!
//! The pipeline is:
//!
//! ```text
//! executable IR (blocks, completion switches, registry facts)
//!   -> lower::lower_function          every statement -> native ops + explicit framing
//!        representation::…            NativeInt/NativeDouble/NativeBool/Boxed per value
//!        cells::ShadowState           values stay native between statements
//!        elide::TraceLedger           TraceBarrier removed / guarded / kept, with reasons
//!        elide::CellDemotion          Cell -> Slot for proven-local procedure variables
//!   -> NativeFunction + FunctionReport
//! ```
//!
//! Every decision is recorded per statement in a [`FunctionReport`] so the
//! Explorer can show why framing was kept. Four
//! [`SemanticOptimisationPassId`](crate::semantic_optimisation::SemanticOptimisationPassId)
//! passes gate the work — `NativeLowering`, `RepresentationInference`,
//! `TraceBarrierElision`, `CellDemotion` — all off by default and enabled
//! together by the WASM pipeline's native tier.

pub mod cells;
pub mod elide;
pub mod ir;
pub mod lower;
pub mod representation;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use tcl_registry::hooks::LoweringHookId;

pub use lower::{LoweringInput, lower_function};

use self::elide::{BarrierDecision, CellDecision};
use crate::ir::NodeId;

/// Why one statement kept the source-text rung ([`ir::NativeOp::EvalSource`])
/// or a nested word declined native evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeLoweringDecline {
    /// The executable IR left the region opaque (the retained structural
    /// identity, when the lowering kept one).
    OpaqueRegion(Option<LoweringHookId>),
    /// Lowering retained no structured word snapshot for the statement.
    MissingCommandTokens,
    /// `{*}` expansion needs runtime Tcl list processing.
    ArgumentExpansion,
    /// A word's substitution behaviour is not modelled by the word snapshot.
    OpaqueWord,
    /// The word contains backslash substitution the lowering does not perform.
    BackslashSubstitution,
    /// A variable reference computes its own name.
    DynamicVariableName,
    /// The variable spelling cannot be told apart from another Tcl meaning.
    AmbiguousVariableSpelling,
    /// A command substitution is not one complete command.
    UnmodelledCommandSubstitution,
    /// Word nesting exceeded the lowering's recursion cap.
    WordNestingTooDeep,
    /// The statement writes a cell whose name is computed at run time.
    ComputedCellName,
    /// A retained operand (an `incr` amount, a `return` value) carries
    /// substitutions the statement snapshot cannot evaluate structurally.
    SubstitutedOperand,
}

impl NativeLoweringDecline {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpaqueRegion(_) => "opaque-region",
            Self::MissingCommandTokens => "missing-command-tokens",
            Self::ArgumentExpansion => "argument-expansion",
            Self::OpaqueWord => "opaque-word",
            Self::BackslashSubstitution => "backslash-substitution",
            Self::DynamicVariableName => "dynamic-variable-name",
            Self::AmbiguousVariableSpelling => "ambiguous-variable-spelling",
            Self::UnmodelledCommandSubstitution => "unmodelled-command-substitution",
            Self::WordNestingTooDeep => "word-nesting-too-deep",
            Self::ComputedCellName => "computed-cell-name",
            Self::SubstitutedOperand => "substituted-operand",
        }
    }
}

/// Why a whole function stayed on the legacy structured emission path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionDecline {
    /// The `NativeLowering` pass is not enabled.
    PassDisabled,
    /// The compilation unit retained no executable function for the body.
    NoExecutableFunction,
    /// The executable IR failed validation.
    InvalidExecutableIr,
    /// An executable instruction kind the native lowering does not project
    /// yet (a cursor loop, a pattern match, a completion handler, a
    /// structured operand). The spelling names the kind.
    UnloweredInstruction(&'static str),
}

impl FunctionDecline {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PassDisabled => "pass-disabled",
            Self::NoExecutableFunction => "no-executable-function",
            Self::InvalidExecutableIr => "invalid-executable-ir",
            Self::UnloweredInstruction(_) => "unlowered-instruction",
        }
    }

    /// The instruction kind for an [`Self::UnloweredInstruction`].
    #[must_use]
    pub const fn detail(self) -> Option<&'static str> {
        match self {
            Self::UnloweredInstruction(kind) => Some(kind),
            _ => None,
        }
    }
}

/// How one executable instruction was lowered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatementOutcome {
    /// Native operations only (values, cells, arithmetic, completion).
    Native,
    /// A registry intrinsic taken directly under a dispatch proof.
    NativeIntrinsic,
    /// A fixed completion taken directly under a dispatch proof.
    NativeCompletion,
    /// A definition-time command bound through the runtime's proc-definition
    /// ABI, carrying the compiled body's entry when the module installed one.
    NativeDefinition,
    /// A generic prebuilt-argv invocation through runtime dispatch.
    GenericInvoke,
    /// The source-text rung, with the typed reason.
    EvalSource(NativeLoweringDecline),
    /// The instruction contributes no operation of its own (an argv assembly,
    /// a region completion, a word of a declined statement).
    Empty,
}

impl StatementOutcome {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::NativeIntrinsic => "native-intrinsic",
            Self::NativeCompletion => "native-completion",
            Self::NativeDefinition => "native-definition",
            Self::GenericInvoke => "generic-invoke",
            Self::EvalSource(_) => "eval-source",
            Self::Empty => "empty",
        }
    }
}

/// Which access a [`CellAccessRecord`] describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellAccessKind {
    /// A read of the cell.
    Read,
    /// A write of the cell.
    Write,
    /// A read-modify-write (`incr`, `append`, `lappend`).
    Update,
}

impl CellAccessKind {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Update => "update",
        }
    }
}

/// One recorded cell access with every framing decision taken for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellAccessRecord {
    /// The cell as Tcl spells it.
    pub place: String,
    /// The access kind.
    pub access: CellAccessKind,
    /// The storage decided for the cell.
    pub storage: CellDecision,
    /// The trace-barrier decision.
    pub barrier: BarrierDecision,
    /// Whether the access reused a native shadow instead of the runtime cell.
    pub shadowed: bool,
}

/// The lowering record of one executable instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementRecord {
    /// The source-semantic node, when the instruction has one.
    pub node: Option<NodeId>,
    /// The executable instruction kind.
    pub instruction: &'static str,
    /// How it was lowered.
    pub outcome: StatementOutcome,
    /// Every cell access the instruction performs.
    pub cells: Vec<CellAccessRecord>,
    /// Representation kinds of the values the instruction defines, in
    /// definition order (stable lattice spellings).
    pub representations: Vec<&'static str>,
}

/// Why a lowered procedure body is not bound as the procedure's native entry.
///
/// The body still compiled and still appears in the module; the runtime just
/// keeps running the source body, exactly as it does for a procedure the tier
/// declined outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcEntryDecline {
    /// A completion the body returns on its own normal edge carries no Tcl
    /// result, so the procedure would answer with the empty string where Tcl
    /// answers with a value — `append`/`lappend`, or a structured region
    /// whose completion the executable IR produces with no result.
    UndeterminedResult,
}

impl ProcEntryDecline {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UndeterminedResult => "undetermined-result",
        }
    }
}

/// Whether a compiled body is bound to its procedure's definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeBinding {
    /// Not a procedure body, or the tier never lowered one.
    NotApplicable,
    /// The compiled body is installed in the runtime's shared function table
    /// and bound to the procedure's definition, so calling the procedure runs
    /// it instead of the source body.
    BoundNatively,
    /// The body compiled but is not bound; the source body runs.
    SourceOnly(ProcEntryDecline),
}

impl NativeBinding {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotApplicable => "not-applicable",
            Self::BoundNatively => "bound-natively",
            Self::SourceOnly(_) => "source-only",
        }
    }

    /// The reason a compiled body is not bound.
    #[must_use]
    pub const fn reason(self) -> Option<ProcEntryDecline> {
        match self {
            Self::SourceOnly(reason) => Some(reason),
            _ => None,
        }
    }
}

/// Whether a function was lowered, and its per-statement record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionStatus {
    /// Lowered to NLIR and emitted natively.
    Lowered,
    /// Kept on the legacy structured path.
    Declined(FunctionDecline),
}

/// The complete lowering record of one function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionReport {
    /// Lowered or declined.
    pub status: FunctionStatus,
    /// Whether the emitted body is bound as the procedure's native entry.
    pub binding: NativeBinding,
    /// One record per executable instruction, in block order.
    pub statements: Vec<StatementRecord>,
}

impl FunctionReport {
    /// A report for a function that never reached the lowering.
    #[must_use]
    pub const fn declined(reason: FunctionDecline) -> Self {
        Self {
            status: FunctionStatus::Declined(reason),
            binding: NativeBinding::NotApplicable,
            statements: Vec::new(),
        }
    }
}

/// The native tier's record for a whole module.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeTierReport {
    /// Whether the native tier was requested at all.
    pub enabled: bool,
    /// Per-function reports keyed by qualified function name (`::top` for
    /// the top-level script).
    pub functions: BTreeMap<String, FunctionReport>,
}

impl NativeTierReport {
    /// The report of one function.
    #[must_use]
    pub fn function(&self, name: &str) -> Option<&FunctionReport> {
        self.functions.get(name)
    }
}
