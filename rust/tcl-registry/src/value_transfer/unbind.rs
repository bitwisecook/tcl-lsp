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

//! The unbind specialisation, derived from
//! [`Traits::DESTROYS_VARIABLE`](crate::traits::Traits::DESTROYS_VARIABLE):
//! every name the invocation's variable-writing operands resolve to is
//! unbound afterwards. The existence transfer is what the `info exists`
//! fold reads; the error on an absent place and the `-nocomplain` form's
//! completion domain arrive with the existence rung.

use crate::arg_role::ArgRole;
use crate::completion::{CompletionCode, CompletionCodeDomain};

use super::CommandSemantics;
use super::answers::{CompletionPath, ExistenceOutcome, ExistenceTransfer, TransferAnswer};
use super::context::Budget;
use super::decline::NoRouteReason;
use super::inputs::{AnalysisInputs, FactDomain, TargetId};
use super::route::EvalRoute;

const NORMAL: &[CompletionCode] = &[CompletionCode::Ok];

/// The derived unbind specialisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnbindSemantics;

impl CommandSemantics for UnbindSemantics {
    fn identity(&self) -> &'static str {
        "unbind"
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: NoRouteReason::Unauthored,
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        if domain != FactDomain::Existence {
            return TransferAnswer::Generic;
        }
        let view = input.invocation();
        // A name the driver cannot resolve to a place — a computed name,
        // an escaping or traced place — is left to the owner of that
        // decline: the dynamic-name barrier already blinds the existence
        // fold for a computed destroy.
        let outcomes = view
            .operands_with_role(ArgRole::VarWrite)
            .filter(|id| input.place(*id).is_ok())
            .map(|id| (TargetId(id), ExistenceOutcome::Unbind))
            .collect();
        TransferAnswer::Existence(ExistenceTransfer {
            paths: vec![CompletionPath {
                completion: CompletionCodeDomain::Exact(NORMAL),
                outcomes,
            }],
        })
    }
}
