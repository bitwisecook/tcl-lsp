// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C9 immediate range stack-result ownership.

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

/// Execute C9's original immediate range over its retained subject.
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
            if version != TclVersion::V9_0
                || ops.name_policy_protocol().is_none_or(|policy| {
                    policy.string_protocol() != NativeStringProtocol::C(version)
                })
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "compiled C9 String range protocol",
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
