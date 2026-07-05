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

//! Variable-mutating command cores — the *compute the new value* half of
//! `append`/`lappend`, shared over [`ValueOps`].
//!
//! These return the value to store; the caller (a thin per-runtime adapter)
//! owns the **store** (a single `set`, which fires the write trace) plus the
//! command-specific edges that differ across runtimes and can't live here: the
//! const-variable check, the array-element split, and the no-argument forms
//! (`append x` reads, erroring if unset; `lappend x` reads, creating an empty
//! list if unset). Keeping the store in the adapter means the write trace fires
//! exactly once over the whole operation — the common, user-visible case —
//! while the in-place growth below keeps a building loop amortised O(1) per byte
//! instead of O(n²).
//!
//! Both halves are **byte-exact**: `append` works over [`ValueOps::as_bytes`]/
//! [`ValueOps::new_bytes`], not the lossy char seam, so `append data $binary`
//! preserves bytes > 127; `lappend` works over list *element values*, which are
//! never stringified, so it is byte-exact for free.

use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// `append`'s new value: `cur` (the variable's current value, or `None` if it
/// was unset → empty) with every `values` element's bytes appended. Grows `cur`
/// in place when the runtime can (returning the same value), else builds a fresh
/// one. Byte-exact. Never fails (byte concatenation always succeeds).
///
/// `values` is assumed non-empty — the no-argument `append x` form is a read,
/// handled by the adapter.
pub fn append_bytes<O, V>(ops: &mut O, cur: Option<V>, values: &[V]) -> V
where
    O: ValueOps<Value = V>,
    V: Clone,
{
    let Some(mut v) = cur else {
        // Unset variable: build a fresh value from the appended bytes.
        let mut buf = Vec::new();
        for val in values {
            buf.extend_from_slice(&ops.as_bytes(val));
        }
        return ops.new_bytes(&buf);
    };
    // Grow `v` in place for as many leading values as the runtime allows. The
    // capability is all-or-nothing in practice (it depends on `v`'s shared/typed
    // state, which appending doesn't change), but the loop tolerates a partial
    // prefix and rebuilds from `v`'s current bytes for the remainder.
    let mut grew = 0;
    for val in values {
        let bytes = ops.as_bytes(val);
        if ops.try_append_bytes_in_place(&mut v, &bytes) {
            grew += 1;
        } else {
            break;
        }
    }
    if grew == values.len() {
        return v;
    }
    let mut buf = ops.as_bytes(&v).to_vec();
    for val in &values[grew..] {
        buf.extend_from_slice(&ops.as_bytes(val));
    }
    ops.new_bytes(&buf)
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
        if ops.try_list_append_in_place(&mut v, val) {
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
