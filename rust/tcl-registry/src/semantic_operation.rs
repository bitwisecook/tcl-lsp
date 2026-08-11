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

//! Target-neutral identities for registry-described semantic operations.

use crate::hooks::LoweringHookId;
use crate::intrinsic::IntrinsicId;

/// Target-neutral error context contributed by an inlined command body.
///
/// This describes Tcl evaluation semantics, not an emitter callback or a
/// source command spelling.  Front ends retain it when a registry-described
/// operation is lowered into an inline body; backends convert the closed enum
/// to their own error-frame ABI representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InlineBodyErrorContext {
    /// A script evaluated transparently in the current Tcl frame.
    SameFrameScriptEvaluation,
}

/// A static target-neutral semantic operation identity.
///
/// This is deliberately **not** a live Tcl command identity.  It says what a
/// registry-described invocation means after command, subcommand, and form
/// resolution; it says nothing about whether that command is still bound to
/// the described implementation in a mutable interpreter.  Command-binding,
/// namespace, alias, `unknown`, and trace analysis establish those runtime
/// proofs separately.
///
/// The initial catalogue reuses the existing common structural-lowering
/// identities.  New target-neutral operations should be added here and stamped
/// on registry descriptors, instead of growing target-specific codegen hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SemanticOperationId {
    /// Generic Tcl argv invocation.
    ///
    /// Every registry resolution has this conservative fallback, so a backend
    /// registry can decline native selection without losing the invocation's
    /// Tcl semantics.
    Invoke,
    /// A target-neutral intrinsic selected entirely from registry metadata.
    Intrinsic(IntrinsicId),
    /// A common structural operation represented by an existing front-end
    /// lowering descriptor.
    StructuredLowering(LoweringHookId),
}

impl SemanticOperationId {
    /// Return the Tcl error context contributed when this operation's body is
    /// lowered inline.
    ///
    /// `None` is meaningful: most structural bodies either supply their error
    /// context through another control-flow construct or deliberately remain
    /// transparent when inlined.
    #[must_use]
    pub const fn inline_body_error_context(self) -> Option<InlineBodyErrorContext> {
        match self {
            Self::StructuredLowering(LoweringHookId::Eval) => {
                Some(InlineBodyErrorContext::SameFrameScriptEvaluation)
            }
            Self::Invoke | Self::Intrinsic(_) | Self::StructuredLowering(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_frame_eval_operation_supplies_inline_error_context() {
        assert_eq!(
            SemanticOperationId::StructuredLowering(LoweringHookId::Eval)
                .inline_body_error_context(),
            Some(InlineBodyErrorContext::SameFrameScriptEvaluation),
        );
        assert_eq!(
            SemanticOperationId::StructuredLowering(LoweringHookId::Uplevel)
                .inline_body_error_context(),
            None,
        );
    }
}
