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

//! Authentic C trace callbacks and their purpose-specific interpreter state.

use crate::InvocationDialect;
use tcl_dialect::TclVersion;

/// Selected direct C trace machinery. It supplies no variable cell or callback identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeVariableTraceProtocol {
    version: TclVersion,
}

#[cfg(test)]
mod tests {
    use super::{NativeTraceStatePurpose as Purpose, NativeTraceStateRecipe as Recipe, *};

    #[test]
    fn callback_purposes_select_native_result_and_metadata_ownership() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeVariableTraceProtocol { version };
            let chain = if version == TclVersion::V8_4 {
                Recipe::None
            } else {
                Recipe::Interpreter
            };
            assert_eq!(protocol.state_recipe(Purpose::VariableChain), chain);
            assert_eq!(protocol.state_recipe(Purpose::CommandChain), chain);
            assert_eq!(
                protocol.state_recipe(Purpose::VariableScript),
                if version == TclVersion::V8_4 {
                    Recipe::MoveResult
                } else {
                    Recipe::None
                }
            );
            assert_eq!(
                protocol.state_recipe(Purpose::CommandScript),
                if version == TclVersion::V8_4 {
                    Recipe::MoveResultAndCode
                } else {
                    Recipe::None
                }
            );
        }
    }
}

/// The reached callback boundary whose interpreter state is preserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeTraceStatePurpose {
    /// An entire variable callback chain.
    VariableChain,
    /// One evaluated variable trace script.
    VariableScript,
    /// An entire command rename/delete callback chain.
    CommandChain,
    /// One evaluated command rename/delete trace script.
    CommandScript,
}

/// Actual ownership and metadata preserved at a selected callback boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeTraceStateRecipe {
    /// This boundary does not save interpreter state.
    None,
    /// Transfer the result role and install an empty result for evaluation.
    MoveResult,
    /// Transfer the result role and preserve the private completion code.
    MoveResultAndCode,
    /// Retain the result and the complete interpreter return/error state.
    Interpreter,
}

impl NativeVariableTraceProtocol {
    /// Exact actual release, independently of the caller's source grammar.
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.version
    }

    /// Whether the whole native callback chain owns one saved result reference.
    #[must_use]
    pub const fn saves_chain_result(self) -> bool {
        !matches!(self.version, TclVersion::V8_4)
    }

    /// Whether each script callback saves its result independently of the chain.
    #[must_use]
    pub const fn saves_script_result(self) -> bool {
        matches!(self.version, TclVersion::V8_4)
    }

    /// Select state preservation independently for variable and command callbacks.
    #[must_use]
    pub const fn state_recipe(self, purpose: NativeTraceStatePurpose) -> NativeTraceStateRecipe {
        use NativeTraceStatePurpose as Purpose;
        use NativeTraceStateRecipe as Recipe;
        match (self.version, purpose) {
            (TclVersion::V8_4, Purpose::VariableScript) => Recipe::MoveResult,
            (TclVersion::V8_4, Purpose::CommandScript) => Recipe::MoveResultAndCode,
            (TclVersion::V8_4, Purpose::VariableChain | Purpose::CommandChain)
            | (_, Purpose::VariableScript | Purpose::CommandScript) => Recipe::None,
            (_, Purpose::VariableChain | Purpose::CommandChain) => Recipe::Interpreter,
        }
    }
}
impl InvocationDialect {
    /// Select only an actual supported C engine's direct variable trace interface.
    #[must_use]
    pub fn native_variable_trace_protocol(self) -> Option<NativeVariableTraceProtocol> {
        match self.native_string_protocol()? {
            tcl_syntax::native_string::NativeStringProtocol::C(version) => {
                Some(NativeVariableTraceProtocol { version })
            }
            tcl_syntax::native_string::NativeStringProtocol::Jim084 => None,
        }
    }

    /// Select authentic C callback state without borrowing a logical source provider.
    #[must_use]
    pub fn native_trace_state_recipe(
        self,
        purpose: NativeTraceStatePurpose,
    ) -> Option<NativeTraceStateRecipe> {
        Some(self.native_variable_trace_protocol()?.state_recipe(purpose))
    }
}
