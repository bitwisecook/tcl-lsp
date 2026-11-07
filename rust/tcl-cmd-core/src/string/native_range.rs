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

//! Original C8.6+ immediate range stack-result ownership.

use crate::index::NativeIndexOps;
use crate::{CmdError, index};
use tcl_dialect::TclVersion;
use tcl_syntax::native_compiled_index::NativeCompiledListRange;
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::value::ValueError;

/// Native opcode result disposition, independent of pointer or count equality.
#[derive(Debug)]
pub enum NativeCompiledRangeResult<V> {
    /// The empty original subject remains on the native stack.
    Borrowed(V),
    /// The original `GetRange` constructor produced a fresh object.
    Created(V),
}

/// Execute C8.6+'s original immediate range over its retained subject.
///
/// # Errors
/// Retains missing physical protocol, original getter, constructor and currentness failures.
pub fn compiled_immediate_range<O: NativeIndexOps>(
    ops: &mut O,
    subject: &O::Value,
    coordinates: NativeCompiledListRange,
    version: TclVersion,
) -> Result<NativeCompiledRangeResult<O::Value>, CmdError> {
    index::run_index_output(
        ops,
        |ops| {
            if version < TclVersion::V8_6
                || ops.name_policy_protocol().is_none_or(|policy| {
                    policy.string_protocol() != NativeStringProtocol::C(version)
                })
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "compiled String range protocol",
                )
                .into());
            }
            let length = ops.index_character_count(subject)?;
            if length == 0 {
                return Ok(NativeCompiledRangeResult::Borrowed(subject.clone()));
            }
            let end = i128::try_from(length).map_err(|_| {
                ValueError::CommandProtocolUnavailable("compiled String range length width")
            })? - 1;
            // C decodes the last coordinate before the first coordinate.
            let last = i64::try_from(coordinates.last.decode(end).max(-1)).map_err(|_| {
                ValueError::CommandProtocolUnavailable("compiled String range last width")
            })?;
            let first = i64::try_from(coordinates.first.decode(end).max(-1)).map_err(|_| {
                ValueError::CommandProtocolUnavailable("compiled String range first width")
            })?;
            super::native_c::parsed_range(ops, subject, first, last, version)
                .map(NativeCompiledRangeResult::Created)
        },
        |ops, result| match result {
            NativeCompiledRangeResult::Created(value) => ops.discard_owned_index_result(value),
            NativeCompiledRangeResult::Borrowed(value) => drop(value),
        },
        |error| error,
    )
}
