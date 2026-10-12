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

//! List append value computation over original list elements.
//!
//! The retained variable adapter owns read observers, publication and write
//! observers. Native object append uses [`crate::native_append`] to preserve
//! source representations and each engine's publication boundaries.

use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// Concatenate already decoded logical string values for source-value analysis.
/// Checked text access supplies their counted contents; this door does not run
/// native append, infer a physical representation, or publish variable observers.
/// Native adapters use the independently authenticated original-object owner.
pub fn logical_append_value<O: ValueOps>(
    ops: &mut O,
    current: Option<O::Value>,
    values: &[O::Value],
) -> Result<O::Value, CmdError> {
    if values.is_empty() {
        return Ok(current.unwrap_or_else(|| ops.empty()));
    }
    let mut contents = match current.as_ref() {
        Some(value) => ops.try_as_str(value)?.to_string(),
        None => String::new(),
    };
    for value in values {
        contents.push_str(&ops.try_as_str(value)?);
    }
    Ok(ops.new_string(contents))
}

/// `lappend`'s new value: `cur` (or `None` → an empty list) with every `values`
/// element appended as a list element. Grows `cur` in place when the runtime can
/// (returning the same value), else builds a fresh list. Byte-exact (elements
/// are never stringified). Errors with the canonical list-parse message if `cur`
/// is not a well-formed list.
///
/// `values` is assumed non-empty — the no-argument `lappend x` form (read, or
/// create-empty-if-unset) is handled by the adapter.
pub fn lappend_value<O, V>(ops: &mut O, cur: Option<V>, values: &[V]) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
    V: Clone,
{
    let Some(mut v) = cur else {
        return Ok(ops.new_list(values.to_vec()));
    };
    let mut grew = 0;
    for val in values {
        if ops.try_list_append_in_place(&mut v, val)? {
            grew += 1;
        } else {
            break;
        }
    }
    if grew == values.len() {
        return Ok(v);
    }
    // Rebuild from `v`'s elements (which validates it as a list) + the rest.
    let mut items = ops.list_elements(&v)?;
    items.extend(values[grew..].iter().cloned());
    Ok(ops.new_list(items))
}

/// Publish a selected Jim link failure with its actual String producer and
/// default error-code update. No C UPVAR classification is borrowed.
#[must_use]
pub fn native_jim_alias_error(
    protocol: tcl_syntax::native_jim_lookup::NativeJimLookupProtocol,
    failure: tcl_syntax::native_jim_lookup::NativeJimAliasFailure,
    original: &[u8],
) -> CmdError {
    // naming.variable.original-upvar-error-publication
    // docs/design/analysis/name-resolution-proofs/variable-original-upvar-error-publication.md
    CmdError::new_bytes(protocol.alias_failure_message(failure, original))
        .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::Jim084)
}
