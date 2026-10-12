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
    fn command_prefix_storage_reporting_removal_and_eval_extents_are_distinct() {
        for version in TclVersion::ALL {
            let protocol = NativeVariableTraceProtocol { version };
            assert!(protocol.command_prefix_matches(b"cb\xff\0A", b"cb\xff\0B"));
            assert!(!protocol.command_prefix_matches(b"cb\xff\0A", b"cb\xff\0BB"));
            assert!(!protocol.command_prefix_matches(b"cb\xff\0A", b"cbx\0A"));
            assert_eq!(protocol.command_prefix_report(b"cb\xff\0A"), b"cb\xff");
            let script = b"cb\xff\0A target enter";
            assert_eq!(protocol.command_callback_source(false, script), script);
            assert_eq!(
                protocol.command_callback_source(true, script),
                if version <= TclVersion::V8_5 {
                    b"cb\xff".as_slice()
                } else {
                    script
                }
            );
        }
    }

    #[test]
    fn variable_prefix_copy_remove_report_and_counted_eval_keep_independent_extents() {
        // Native proof: naming.variable.copied-prefix-storage-removal-report-evaluation
        // docs/design/analysis/name-resolution-proofs/copied-prefix-storage-removal-report-evaluation.md
        for version in TclVersion::ALL {
            let protocol = NativeVariableTraceProtocol { version };
            let original = b"watch A\0X";
            assert_eq!(protocol.variable_prefix_storage(original), original);
            assert!(protocol.variable_prefix_matches(original, b"watch A\0Y"));
            assert!(!protocol.variable_prefix_matches(original, b"watch A\0YY"));
            assert!(!protocol.variable_prefix_matches(original, b"watch A\xc0\x80X"));
            assert_eq!(protocol.variable_prefix_report(original), b"watch A");
            assert_eq!(protocol.variable_callback_source(original), original);
        }
    }

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
    /// Character-script trace callbacks enter the direct evaluator in the
    /// triggering frame; they do not inherit the installer's compiler mode.
    #[must_use]
    pub const fn callback_compilation(self) -> crate::native_compilation::NativeCompilationContext {
        // Source proof: naming.variable.trace-callback-direct-source (docs/design/analysis/name-resolution-proofs/trace-callback-direct-source.md).
        crate::native_compilation::NativeCompilationContext {
            mode: crate::native_compilation::NativeCompilationMode::Direct,
            frame: crate::native_compilation::NativeCompilationFrame::ScriptCode,
            loop_depth: 0,
            catch_depth: Some(0),
        }
    }

    /// Variable trace registration copies the complete counted prefix into
    /// immutable storage. This pure extent supplies no registration or cell.
    #[must_use]
    pub fn variable_prefix_storage(self, original: &[u8]) -> &[u8] {
        original
    }

    /// Variable trace removal compares counted length and native `strncmp`.
    /// A common embedded zero stops comparison, while length remains counted.
    #[must_use]
    pub fn variable_prefix_matches(self, retained: &[u8], original: &[u8]) -> bool {
        retained.len() == original.len()
            && tcl_core_types::c_string_extent(retained)
                == tcl_core_types::c_string_extent(original)
    }

    /// Variable `trace info` reports a prefix using a `CString` constructor.
    #[must_use]
    pub fn variable_prefix_report(self, retained: &[u8]) -> &[u8] {
        tcl_core_types::c_string_extent(retained)
    }

    /// Variable callbacks append the complete copied prefix and evaluate the
    /// assembled counted script with `Tcl_EvalEx` on all selected C releases.
    #[must_use]
    pub fn variable_callback_source(self, source: &[u8]) -> &[u8] {
        source
    }

    /// Match the counted command/execution trace prefix with native `strncmp`.
    /// Storage keeps every original byte; bytes after a common NUL do not
    /// distinguish prefixes of the same counted length during removal.
    #[must_use]
    pub fn command_prefix_matches(self, retained: &[u8], original: &[u8]) -> bool {
        retained.len() == original.len()
            && tcl_core_types::c_string_extent(retained)
                == tcl_core_types::c_string_extent(original)
    }

    /// `trace info` creates its prefix member with a `CString` constructor.
    #[must_use]
    pub fn command_prefix_report(self, retained: &[u8]) -> &[u8] {
        tcl_core_types::c_string_extent(retained)
    }

    /// C8.4/8.5 execution traces use `Tcl_Eval`; command traces and later
    /// execution traces pass the counted assembled script to `Tcl_EvalEx`.
    #[must_use]
    pub fn command_callback_source(self, execution: bool, source: &[u8]) -> &[u8] {
        if execution && self.version <= TclVersion::V8_5 {
            tcl_core_types::c_string_extent(source)
        } else {
            source
        }
    }

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
