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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Selected command delete-procedure and import retirement stages.
//!
//! The embedding owns the original command token, actual table and entered
//! cleanup currency. This coordinator owns no command/reference or lookup role.
use crate::CmdError;

/// Pure source order, selected independently of actual binding authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeCommandRetirementOrder {
    /// Tcl 8.4/8.5 calls deleteProc before walking the import references.
    ImplementationBeforeImports,
    /// Tcl 8.6 and later removes import references before deleteProc.
    ImportsBeforeImplementation,
}

/// One independently owned phase of a true command deletion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeCommandRetirementStage {
    Implementation,
    Imports,
}

/// Finish both actual mandatory retirement owners in the selected order.
/// The first full error survives later cleanup; no later stage obtains authority
/// from this pure order. Each adapter checks and retains its genuine first cause.
pub fn retire_original_command(
    order: NativeCommandRetirementOrder,
    mut retire: impl FnMut(NativeCommandRetirementStage) -> Result<(), CmdError>,
) -> Result<(), CmdError> {
    use NativeCommandRetirementStage::{Implementation, Imports};
    let stages = match order {
        NativeCommandRetirementOrder::ImplementationBeforeImports => [Implementation, Imports],
        NativeCommandRetirementOrder::ImportsBeforeImplementation => [Imports, Implementation],
    };
    let mut first = None;
    for stage in stages {
        if let Err(error) = retire(stage) {
            first.get_or_insert(error);
        }
    }
    first.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_command_retirement_keeps_both_owned_stages_and_first_full_error() {

        // Pure coordinator only; no interpreter, token or Native callback.
        for (order, expected) in [
            (
                NativeCommandRetirementOrder::ImplementationBeforeImports,
                [
                    NativeCommandRetirementStage::Implementation,
                    NativeCommandRetirementStage::Imports,
                ],
            ),
            (
                NativeCommandRetirementOrder::ImportsBeforeImplementation,
                [
                    NativeCommandRetirementStage::Imports,
                    NativeCommandRetirementStage::Implementation,
                ],
            ),
        ] {
            let mut reached = Vec::new();
            let first = CmdError::from(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "FIRST_RETIREMENT",
            ));
            let later = CmdError::from(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "LATER_RETIREMENT",
            ));
            let error = retire_original_command(order, |stage| {
                reached.push(stage);
                Err(if reached.len() == 1 {
                    first.clone()
                } else {
                    later.clone()
                })
            })
            .unwrap_err();
            assert_eq!(reached, expected);
            assert!(first.native_execution_refusal().is_some());
            assert_eq!(
                error.native_execution_refusal(),
                first.native_execution_refusal()
            );
            assert_ne!(
                error.native_execution_refusal(),
                later.native_execution_refusal()
            );
        }
    }
}
