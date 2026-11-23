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

//! Typed payload families of the compiler-check diagnostics and the contract
//! assigning each code its family.

use tcl_core_types::DiagCode;
use tcl_lexer::Span;

use super::TaintDiagnosticSubject;

/// Typed semantic payload of one compiler-check diagnostic, per family.
///
/// Every code the pipeline emits carries exactly one variant; the message and
/// fix descriptions are rendered presentation. The family contract is in
/// `docs/design/contracts/diagnostic-subjects.md`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CheckPayload {
    /// O100: a branch condition SCCP proved constant.
    ConstantBranch {
        /// The proved condition value.
        value: bool,
    },
    /// O105 / O106: a computation that repeats an earlier value.
    Redundancy {
        /// Span of the first computation producing the same value.
        first_span: Span,
        /// Expression text of the repeated computation.
        expression: String,
    },
    /// S100 / S101 / S110: a value converted between internal representations.
    Representation {
        /// Variable holding the value.
        variable: String,
        /// Representation the value held.
        from_type: tcl_registry::types::TclType,
        /// Representation the command required.
        to_type: tcl_registry::types::TclType,
        /// Command that required it.
        command: String,
        /// Whether the use is inside a loop body.
        in_loop: bool,
    },
    /// S102: a loop variable oscillating between two representations.
    Oscillation {
        /// Oscillating variable.
        variable: String,
        /// First observed representation.
        first_type: tcl_registry::types::TclType,
        /// Second observed representation.
        second_type: tcl_registry::types::TclType,
    },
    /// S103: a mutation that copies a value shared with another variable.
    SharedCopy {
        /// Mutated variable.
        variable: String,
        /// Other live holder of the value.
        shared_with: String,
        /// Mutating command.
        command: String,
    },
    /// T100–T106, W313, IRULE3001–IRULE3004, IRULE3101, IRULE3103.
    Taint(TaintDiagnosticSubject),
    /// W201: a path built by string concatenation.
    PathConcatenation {
        /// Variable receiving the concatenated path.
        variable: String,
    },
    /// IRULE1005–IRULE1008, IRULE1201, IRULE1202, IRULE3102, IRULE4002,
    /// IRULE4004, IRULE5002, IRULE5004.
    Irules(crate::irules_checks::IrulesCheckPayload),
}

impl CheckPayload {
    /// Every code the compiler-check pipeline emits, each with one payload family.
    pub const CODES: &'static [DiagCode] = &[
        DiagCode::O100,
        DiagCode::O105,
        DiagCode::O106,
        DiagCode::S100,
        DiagCode::S101,
        DiagCode::S102,
        DiagCode::S103,
        DiagCode::S110,
        DiagCode::T100,
        DiagCode::T101,
        DiagCode::T102,
        DiagCode::T103,
        DiagCode::T104,
        DiagCode::T105,
        DiagCode::T106,
        DiagCode::W201,
        DiagCode::W313,
        DiagCode::Irule1005,
        DiagCode::Irule1006,
        DiagCode::Irule1007,
        DiagCode::Irule1008,
        DiagCode::Irule1201,
        DiagCode::Irule1202,
        DiagCode::Irule3001,
        DiagCode::Irule3002,
        DiagCode::Irule3003,
        DiagCode::Irule3004,
        DiagCode::Irule3101,
        DiagCode::Irule3103,
        DiagCode::Irule4002,
        DiagCode::Irule4004,
        DiagCode::Irule5002,
        DiagCode::Irule5004,
    ];

    /// Payload family, without its fields.
    #[must_use]
    pub fn kind(&self) -> CheckPayloadKind {
        use crate::irules_checks::IrulesCheckPayload as I;
        match self {
            Self::ConstantBranch { .. } => CheckPayloadKind::ConstantBranch,
            Self::Redundancy { .. } => CheckPayloadKind::Redundancy,
            Self::Representation { .. } => CheckPayloadKind::Representation,
            Self::Oscillation { .. } => CheckPayloadKind::Oscillation,
            Self::SharedCopy { .. } => CheckPayloadKind::SharedCopy,
            Self::Taint(_) => CheckPayloadKind::Taint,
            Self::PathConcatenation { .. } => CheckPayloadKind::PathConcatenation,
            Self::Irules(payload) => match payload {
                I::UnnormalisedGetter { .. } => CheckPayloadKind::UnnormalisedGetter,
                I::UnguardedDrop { .. } => CheckPayloadKind::UnguardedDrop,
                I::UnguardedDnsReturn => CheckPayloadKind::UnguardedDnsReturn,
                I::EventWithoutCollect { .. } => CheckPayloadKind::EventWithoutCollect,
                I::PayloadWithoutCollect { .. } => CheckPayloadKind::PayloadWithoutCollect,
                I::CollectWithoutRelease { .. } => CheckPayloadKind::CollectWithoutRelease,
                I::ReleaseWithoutCollect { .. } => CheckPayloadKind::ReleaseWithoutCollect,
                I::ContextAfterResponse { .. } => CheckPayloadKind::ContextAfterResponse,
                I::RepeatedResponse { .. } => CheckPayloadKind::RepeatedResponse,
                I::GenericStaticName { .. } => CheckPayloadKind::GenericStaticName,
                I::HoistableAssignment { .. } => CheckPayloadKind::HoistableAssignment,
            },
        }
    }

    /// Whether this is the payload family the contract assigns to `code`.
    #[must_use]
    pub fn accepts(&self, code: DiagCode) -> bool {
        self.kind().accepts(code)
    }
}

/// Field-free family of a [`CheckPayload`]; owns the code-to-family contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckPayloadKind {
    /// [`CheckPayload::ConstantBranch`].
    ConstantBranch,
    /// [`CheckPayload::Redundancy`].
    Redundancy,
    /// [`CheckPayload::Representation`].
    Representation,
    /// [`CheckPayload::Oscillation`].
    Oscillation,
    /// [`CheckPayload::SharedCopy`].
    SharedCopy,
    /// [`CheckPayload::Taint`].
    Taint,
    /// [`CheckPayload::PathConcatenation`].
    PathConcatenation,
    /// [`crate::irules_checks::IrulesCheckPayload::UnnormalisedGetter`].
    UnnormalisedGetter,
    /// [`crate::irules_checks::IrulesCheckPayload::UnguardedDrop`].
    UnguardedDrop,
    /// [`crate::irules_checks::IrulesCheckPayload::UnguardedDnsReturn`].
    UnguardedDnsReturn,
    /// [`crate::irules_checks::IrulesCheckPayload::EventWithoutCollect`].
    EventWithoutCollect,
    /// [`crate::irules_checks::IrulesCheckPayload::PayloadWithoutCollect`].
    PayloadWithoutCollect,
    /// [`crate::irules_checks::IrulesCheckPayload::CollectWithoutRelease`].
    CollectWithoutRelease,
    /// [`crate::irules_checks::IrulesCheckPayload::ReleaseWithoutCollect`].
    ReleaseWithoutCollect,
    /// [`crate::irules_checks::IrulesCheckPayload::ContextAfterResponse`].
    ContextAfterResponse,
    /// [`crate::irules_checks::IrulesCheckPayload::RepeatedResponse`].
    RepeatedResponse,
    /// [`crate::irules_checks::IrulesCheckPayload::GenericStaticName`].
    GenericStaticName,
    /// [`crate::irules_checks::IrulesCheckPayload::HoistableAssignment`].
    HoistableAssignment,
}

impl CheckPayloadKind {
    /// Whether the contract assigns this family to `code`.
    #[must_use]
    pub fn accepts(self, code: DiagCode) -> bool {
        match self {
            Self::ConstantBranch => code == DiagCode::O100,
            Self::Redundancy => matches!(code, DiagCode::O105 | DiagCode::O106),
            Self::Representation => {
                matches!(code, DiagCode::S100 | DiagCode::S101 | DiagCode::S110)
            }
            Self::Oscillation => code == DiagCode::S102,
            Self::SharedCopy => code == DiagCode::S103,
            Self::Taint => matches!(
                code,
                DiagCode::T100
                    | DiagCode::T101
                    | DiagCode::T102
                    | DiagCode::T103
                    | DiagCode::T104
                    | DiagCode::T105
                    | DiagCode::T106
                    | DiagCode::W313
                    | DiagCode::Irule3001
                    | DiagCode::Irule3002
                    | DiagCode::Irule3003
                    | DiagCode::Irule3004
                    | DiagCode::Irule3101
                    | DiagCode::Irule3103
            ),
            Self::PathConcatenation => code == DiagCode::W201,
            Self::UnnormalisedGetter => code == DiagCode::Irule3102,
            Self::UnguardedDrop => code == DiagCode::Irule5002,
            Self::UnguardedDnsReturn => code == DiagCode::Irule5004,
            Self::EventWithoutCollect => code == DiagCode::Irule1005,
            Self::PayloadWithoutCollect => code == DiagCode::Irule1006,
            Self::CollectWithoutRelease => code == DiagCode::Irule1007,
            Self::ReleaseWithoutCollect => code == DiagCode::Irule1008,
            Self::ContextAfterResponse => code == DiagCode::Irule1201,
            Self::RepeatedResponse => code == DiagCode::Irule1202,
            Self::GenericStaticName => code == DiagCode::Irule4002,
            Self::HoistableAssignment => code == DiagCode::Irule4004,
        }
    }
}
