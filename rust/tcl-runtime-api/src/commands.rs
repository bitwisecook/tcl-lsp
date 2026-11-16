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

//! The command table and its alias bindings: `Commands`, `Aliases` and Jim's inventory and alias lookups.

use super::native_execution_error::NativeExecutionError;
use tcl_core_types::{CommandId, Completion, NsId};

/// The command table and dispatch: builtins, procs, aliases, imports,
/// ensembles, child interps.'s `interp.rs` `Command`.
pub trait Commands {
    /// The runtime's value type.
    type Value;

    /// Dispatch a command by name with its argv, resolving the name in the
    /// current context. Successful transport includes every Guest completion
    /// code; a Host refusal returns its original typed cause separately.
    ///
    /// # Errors
    /// Returns the first reached native execution refusal without fabricating
    /// Guest result or options values.
    fn dispatch(
        &mut self,
        name: &str,
        argv: &[Self::Value],
    ) -> Result<Completion<Self::Value>, NativeExecutionError>;

    /// Dispatch a command already resolved to a [`CommandId`] (by
    /// [`Namespaces::find_command`]) with its argv — the resolve-then-invoke
    /// pairing (mirrors Tcl's `Tcl_GetCommandFromObj` + `Tcl_NRCallObjProc`). A
    /// stale or fabricated id yields a Guest error completion.
    ///
    /// # Errors
    /// Returns the first reached native execution refusal independently of
    /// command-id validity and all Guest completion codes.
    fn dispatch_id(
        &mut self,
        cmd: CommandId,
        argv: &[Self::Value],
    ) -> Result<Completion<Self::Value>, NativeExecutionError>;
}

/// The command flag selected by Jim's original inventory member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimCommandInventoryKind {
    /// Every live command entry.
    Commands,
    /// Actual procedure entries.
    Procs,
    /// Actual entries marked as aliases.
    Aliases,
}

/// Failure after original-object alias lookup has selected an actual outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimAliasLookupFailure {
    /// The original command getter selected no current binding.
    MissingCommand,
    /// The selected binding is not an alias.
    NotAlias,
}

impl NativeJimAliasLookupFailure {
    /// Jim's original-object diagnostic uses the formatter's `CString` extent.
    #[must_use]
    pub fn message(self, original_name: &[u8]) -> Vec<u8> {
        let (prefix, suffix): (&[u8], &[u8]) = match self {
            Self::MissingCommand => (b"invalid command name \"", b"\""),
            Self::NotAlias => (b"command \"", b"\" is not an alias"),
        };
        [
            prefix,
            tcl_core_types::c_string_extent(original_name),
            suffix,
        ]
        .concat()
    }
}

/// The result of original-object alias lookup in an actual command table.
pub enum AliasPrefixLookup<V> {
    /// No current command was selected by the original operand.
    MissingCommand,
    /// The selected command does not carry an alias prefix.
    NotAlias,
    /// The retained original prefix list, without rebuilding its header.
    Prefix(V),
}

/// Object introspection over actual retained alias bindings.
pub trait Aliases: tcl_syntax::value::ValueOps {
    /// Full failure of the retained original alias lookup and prefix owner.
    type OriginalAliasError: From<tcl_syntax::value::ValueError> + From<NativeExecutionError>;
    /// Resolve the original name object and retain its actual prefix header.
    /// A byte-only prefix or reporting name cannot supply this object.
    ///
    /// # Errors
    /// Propagates unavailable lookup, retired prefix storage or missing original ownership.
    fn alias_prefix_original_value(
        &mut self,
        original_name: &Self::Value,
    ) -> Result<AliasPrefixLookup<Self::Value>, Self::OriginalAliasError>;

    /// Resolve the original alias name in a separately retained lookup namespace.
    /// This does not change the caller's frame or destination namespace. The
    /// Runtime and VM adapters validate their actual original context holders;
    /// an implementation without that owner keeps the default refusal.
    ///
    /// # Errors
    /// Missing implementation/context or retired original storage refuses;
    /// the default never substitutes the current namespace.
    fn alias_prefix_original_value_at(
        &mut self,
        _context: NsId,
        _original_name: &Self::Value,
    ) -> Result<AliasPrefixLookup<Self::Value>, Self::OriginalAliasError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original alias lookup context",
        )
        .into())
    }
}
