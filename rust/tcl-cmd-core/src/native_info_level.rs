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

//! Reached native caller-variable frame lookup over original object receivers.

use crate::CmdError;
use tcl_runtime_api::Introspect;
use tcl_syntax::scalar_getter::NativeScalarGetterKind;
use tcl_syntax::value::{ValueError, ValueOps};

/// Actual native integer getter and original frame-vector availability.
pub trait NativeInfoLevelObjects: ValueOps + Introspect<Value = <Self as ValueOps>::Value> {
    /// Reach Int/Wide on the same original header, including failed cache changes.
    fn native_level_integer(
        &mut self,
        original: &<Self as ValueOps>::Value,
        kind: NativeScalarGetterKind,
    ) -> Result<i64, ValueError>;
    /// Build a fresh native List from genuine retained frame argv children.
    /// Missing original frame data is a capability refusal, never rebuilt strings.
    fn native_level_arguments(
        &self,
        level: usize,
    ) -> Result<Option<<Self as ValueOps>::Value>, ValueError>;
}

/// Run an independently selected native opcode without dispatching a mutable handler.
///
/// # Errors
/// Preserves primitive getter/cache failures, genuine bad-level errors and missing
/// original frame-vector authority.
pub fn native_level<O: NativeInfoLevelObjects>(
    ops: &mut O,
    number: Option<&<O as ValueOps>::Value>,
    kind: NativeScalarGetterKind,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<<O as ValueOps>::Value, CmdError> {
    let current = i64::try_from(ops.level())
        .map_err(|_| ValueError::CommandProtocolUnavailable("native frame level width"))?;
    let Some(number) = number else {
        return Ok(ops.new_int(current));
    };
    let requested = ops.native_level_integer(number, kind)?;
    let target = if requested <= 0 {
        current.checked_add(requested)
    } else {
        Some(requested)
    };
    if let Some(target) = target.filter(|target| *target >= 1 && *target <= current)
        && let Some(argv) = ops
            .native_level_arguments(usize::try_from(target).map_err(|_| {
                ValueError::CommandProtocolUnavailable("native frame level width")
            })?)?
    {
        return Ok(argv);
    }
    let original = ops.native_string_bytes(number)?;
    let input = &original[..original
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(original.len())];
    let mut message = b"bad level \"".to_vec();
    message.extend_from_slice(input);
    message.push(b'"');
    let mut code = Vec::new();
    for element in [b"TCL".as_slice(), b"LOOKUP", b"STACK_LEVEL", input] {
        if !code.is_empty() {
            code.push(b' ');
        }
        tcl_syntax::list::append_list_element(&mut code, element, false);
    }
    Err(CmdError::with_error_code_bytes(message, code).with_native_string_result(protocol))
}
