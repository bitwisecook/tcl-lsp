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

//! The shipped value-position specialisations whose routes the registry
//! declares while the compiler still carries their implementations.
//!
//! Each declaration names its route — a catalogued direct evaluator or the
//! shared expression engine — and its result type. The direct evaluators
//! are transitional ([`NativeEvalId::owner`]): the compiler's value-transfer
//! driver runs today's fold until the shared cores replace it, and the
//! migration plan's ledger names each one with its expiry. The expression
//! route is run by the driver's engine adapter by construction; the
//! registry-owned argument assembly lands with the expression slice.

use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{TransferAnswer, TypeFacts};
use super::context::Budget;
use super::inputs::{AnalysisInputs, FactDomain};
use super::route::{EvalRoute, LanguageProfileId, NativeEvalId};

/// A specialisation that declares a catalogued direct route and a result
/// type, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectRoute {
    /// The catalogued evaluator.
    pub id: NativeEvalId,
    /// The result's internal representation.
    pub result_type: TclType,
}

/// `list ?arg …?`.
pub static LIST_OF_ARGS: DirectRoute = DirectRoute {
    id: NativeEvalId::ListOfArgs,
    result_type: TclType::List,
};

/// `format template ?arg …?`.
pub static FORMAT_TEMPLATE: DirectRoute = DirectRoute {
    id: NativeEvalId::FormatTemplate,
    result_type: TclType::String,
};

/// `llength list`.
pub static LIST_LENGTH: DirectRoute = DirectRoute {
    id: NativeEvalId::ListLength,
    result_type: TclType::Int,
};

/// `string length string`.
pub static STRING_LENGTH: DirectRoute = DirectRoute {
    id: NativeEvalId::StringLength,
    result_type: TclType::Int,
};

impl CommandSemantics for DirectRoute {
    fn identity(&self) -> &'static str {
        self.id.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct { id: self.id }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        _input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        match domain {
            FactDomain::Type => TransferAnswer::Type(TypeFacts {
                result: Some(self.result_type),
                per_target: Vec::new(),
                shapes: Vec::new(),
            }),
            _ => TransferAnswer::Generic,
        }
    }
}

/// A specialisation that declares the shared expression engine under a
/// language profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressionRoute {
    /// The language whose arithmetic the engine runs under.
    pub language: LanguageProfileId,
}

/// `expr arg ?arg …?`.
pub static EXPR: ExpressionRoute = ExpressionRoute {
    language: LanguageProfileId::TclExpr,
};

impl CommandSemantics for ExpressionRoute {
    fn identity(&self) -> &'static str {
        match self.language {
            LanguageProfileId::TclExpr => "expression:tcl",
            LanguageProfileId::BpfExpr => "expression:bpf",
        }
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Expression {
            language: self.language,
        }
    }
}
