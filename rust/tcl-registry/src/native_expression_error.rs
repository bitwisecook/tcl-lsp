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

//! Error-state updates at direct expression and script propagation boundaries.

use crate::native_numeric_error::{
    NativeExpressionErrorCodeUpdate, NativeExpressionInvalidTypeErrorCode,
};
use tcl_syntax::expr::parser::NativeExprSyntax;

/// The consuming boundary of a proved expression failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionErrorPublication {
    /// Return from the expression primitive without interpreter Eval propagation.
    Direct,
    /// Propagate the failure through an actual script evaluation.
    Eval,
}

/// Selected error-state updates for a reached expression failure.
/// Numeric invalid-type and syntax producers retain separate provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExpressionErrorStage {
    /// A failed operator operand conversion, excluding arithmetic/domain errors.
    InvalidType(NativeExpressionInvalidTypeErrorCode),
    /// A proved parser rejection under an independently selected syntax recipe.
    Syntax {
        /// Actual or explicitly authored parser recipe.
        syntax: NativeExprSyntax,
        /// Direct expression API error-code update.
        direct: NativeExpressionErrorCodeUpdate,
        /// Interpreter Eval error-code update.
        eval: NativeExpressionErrorCodeUpdate,
    },
}

impl NativeExpressionErrorStage {
    /// Retain the proved syntax producer's updates. The supplied structured code
    /// must belong to that producer; arbitrary command/getter failures cannot use
    /// this projection. Unknown syntax never supplies a state recipe.
    #[must_use]
    pub fn syntax(syntax: NativeExprSyntax, structured: Option<&[u8]>) -> Option<Self> {
        let state = syntax.error_state(structured)?;
        let convert = |update| match update {
            tcl_syntax::expr::parser::NativeExprSyntaxErrorCodeUpdate::Unchanged => {
                NativeExpressionErrorCodeUpdate::Unchanged
            }
            tcl_syntax::expr::parser::NativeExprSyntaxErrorCodeUpdate::Set(code) => {
                NativeExpressionErrorCodeUpdate::Set(code)
            }
        };
        let (direct, eval) = (convert(state.direct), convert(state.eval));
        Some(Self::Syntax {
            syntax,
            direct,
            eval,
        })
    }

    /// Update authored by the actual direct failure producer.
    #[must_use]
    pub fn direct_update(&self) -> &NativeExpressionErrorCodeUpdate {
        match self {
            Self::InvalidType(stage) => stage.direct_update(),
            Self::Syntax { direct, .. } => direct,
        }
    }

    /// Separate update at actual script propagation.
    #[must_use]
    pub fn eval_update(&self) -> &NativeExpressionErrorCodeUpdate {
        match self {
            Self::InvalidType(stage) => stage.eval_update(),
            Self::Syntax { eval, .. } => eval,
        }
    }

    /// Mandatory update for the consuming boundary.
    #[must_use]
    pub fn code_update(
        &self,
        publication: ExpressionErrorPublication,
    ) -> tcl_cmd_core::CmdErrorCodeUpdate {
        match match publication {
            ExpressionErrorPublication::Direct => self.direct_update(),
            ExpressionErrorPublication::Eval => self.eval_update(),
        } {
            NativeExpressionErrorCodeUpdate::Unchanged => {
                tcl_cmd_core::CmdErrorCodeUpdate::Unchanged
            }
            NativeExpressionErrorCodeUpdate::Set(code) => {
                tcl_cmd_core::CmdErrorCodeUpdate::Set(code.clone())
            }
        }
    }
}

impl From<NativeExpressionInvalidTypeErrorCode> for NativeExpressionErrorStage {
    fn from(stage: NativeExpressionInvalidTypeErrorCode) -> Self {
        Self::InvalidType(stage)
    }
}
