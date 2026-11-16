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

//! The premises of the sealed native integer addition and why one was rejected.

use super::closed_program_coverage::ClosedProgramCoverageDecline;
use super::direct_calls::{DirectProcBodyDecline, DirectProcDecline};
use super::materialisable_slots::MaterialisableSlotDecline;
use super::{CommonAotCoverageDecline, DirectCallSiteId};
use crate::native_integer_proof::NativeIntegerDeclineReason;
use crate::semantic_optimisation::SemanticOptimisationPassId;

/// A condition the sealed native integer addition requires, answered by a
/// compile option or by one common proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePremise {
    /// The target lowers semantic plans at all.
    SemanticPlans,
    /// The package shape is one executable IR expresses.
    Packaging,
    /// The compiled program owns the interpreter lifetime and its final state.
    SealedProgram,
    /// A common pass the composition consumes is enabled.
    Pass(SemanticOptimisationPassId),
    /// The unit holds no surface the common tier deliberately excludes.
    Coverage,
    /// A binding-safe direct call to an in-unit procedure.
    DirectCall,
    /// The callee's body may run specialised.
    DirectBody,
    /// Every top-level statement is accounted for by the addition.
    ClosedProgram,
    /// The callee's frame may be omitted.
    Frame,
    /// Both actuals are materialisable integer slots.
    Actuals,
    /// A registry-proved boundary consumes the result.
    Boundary,
    /// The integer proof accepts the addition.
    NativeInteger,
    /// The proved operand values are the constants the program defines.
    Operands,
}

impl NativePremise {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SemanticPlans => "semantic-plans",
            Self::Packaging => "packaging",
            Self::SealedProgram => "sealed-program",
            Self::Pass(_) => "pass",
            Self::Coverage => "coverage",
            Self::DirectCall => "direct-call",
            Self::DirectBody => "direct-body",
            Self::ClosedProgram => "closed-program",
            Self::Frame => "frame",
            Self::Actuals => "actuals",
            Self::Boundary => "boundary",
            Self::NativeInteger => "native-integer",
            Self::Operands => "operands",
        }
    }
}

/// Why one premise of the sealed native integer addition was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDeclineReason {
    /// A pass the composition consumes is not enabled.
    PassDisabled,
    /// The caller's isolated host implements only the source-evaluation ABI.
    EvalOnlyTestHost,
    /// The standalone bootstrap is not expressed by executable IR.
    StandaloneBootstrap,
    /// Hosted compilation preserves externally observable interpreter state.
    HostedEnvironment,
    /// The unit holds surfaces the common tier deliberately excludes.
    ExcludedSurfaces(Vec<CommonAotCoverageDecline>),
    /// No call in the unit resolved to one of its own procedures.
    NoDirectCall,
    /// The direct-procedure proof declined the call.
    DirectCall(DirectProcDecline),
    /// The body of the directly called procedure cannot run specialised.
    DirectBody(DirectProcBodyDecline),
    /// Exact sealed-program statement accounting was unavailable.
    ClosedProgram(ClosedProgramCoverageDecline),
    /// The accounted statements are not the procedure definition, two
    /// constants and the boundary this addition consumes.
    ClosedProgramShape,
    /// Escape analysis found the callee's frame observable.
    FrameEscapes,
    /// The frame may not be omitted: the body or the frame pass is not selected.
    FrameNotElidable,
    /// The call does not pass exactly two fixed formals two actuals.
    OperandCount,
    /// Lowering retained no exact value identity for an actual.
    ActualUnproven,
    /// The slot proof declined an actual.
    Slot(MaterialisableSlotDecline),
    /// An actual's slot is proved, but not as an integer.
    SlotNotInteger,
    /// The slot proof holds no decision for an actual.
    NoSlotDecision,
    /// No selected registry boundary consumes the call's result.
    NoSelectedBoundary,
    /// The callee has no function unit to analyse.
    ProofFunctionUnavailable,
    /// The complexity guard disabled the integer analysis.
    ProofComplexityGuarded,
    /// The actual source metadata needed by the integer proof is unavailable.
    ProofSourceMetadataUnavailable,
    /// The integer proof lacks the selected math command binding premise.
    ProofMathBindingPrerequisiteRequired,
    /// The callee holds no addition of two variables.
    NoAddCandidate,
    /// The callee holds more than one candidate addition.
    AmbiguousAdd,
    /// The integer proof declined the sole addition.
    Integer(NativeIntegerDeclineReason),
    /// The proved addition is not a non-overflowing return of the callee.
    AddShape,
    /// The proof rests on callers beyond the selected call.
    CallerSetDiffers,
    /// An operand's proved range is not a single value.
    OperandNotExact,
    /// The proved operands differ from the constants the program defines.
    OperandsDiffer,
}

impl NativeDeclineReason {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PassDisabled => "pass-disabled",
            Self::EvalOnlyTestHost => "eval-only-test-host",
            Self::StandaloneBootstrap => "standalone-bootstrap",
            Self::HostedEnvironment => "hosted-environment",
            Self::ExcludedSurfaces(_) => "excluded-surface",
            Self::NoDirectCall => "no-direct-call",
            Self::DirectCall(decline) => decline.as_str(),
            Self::DirectBody(decline) => decline.as_str(),
            Self::ClosedProgram(decline) => decline.as_str(),
            Self::ClosedProgramShape => "closed-program-shape",
            Self::FrameEscapes => "frame-escapes",
            Self::FrameNotElidable => "frame-not-elidable",
            Self::OperandCount => "operand-count",
            Self::ActualUnproven => "actual-unproven",
            Self::Slot(decline) => decline.as_str(),
            Self::SlotNotInteger => "slot-not-integer",
            Self::NoSlotDecision => "no-slot-decision",
            Self::NoSelectedBoundary => "no-selected-boundary",
            Self::ProofFunctionUnavailable => "function-unavailable",
            Self::ProofComplexityGuarded => "complexity-guarded",
            Self::ProofSourceMetadataUnavailable => "proof-source-metadata-unavailable",
            Self::ProofMathBindingPrerequisiteRequired => {
                "proof-math-binding-prerequisite-required"
            }
            Self::NoAddCandidate => "no-add-candidate",
            Self::AmbiguousAdd => "ambiguous-add",
            Self::Integer(reason) => reason.as_str(),
            Self::AddShape => "add-shape",
            Self::CallerSetDiffers => "caller-set-differs",
            Self::OperandNotExact => "operand-not-exact",
            Self::OperandsDiffer => "operands-differ",
        }
    }
}

/// One premise the sealed native integer addition rejected, and why.
///
/// A compile that does not select the addition records one of these for every
/// premise it evaluated and found wanting, in evaluation order, so the record
/// names each obstacle rather than the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDecline {
    /// The condition that failed.
    pub premise: NativePremise,
    /// The typed reason it failed.
    pub reason: NativeDeclineReason,
    /// The direct call sites it failed at; empty for a premise about the
    /// unit or its options.
    pub sites: Vec<DirectCallSiteId>,
}
