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

//! The embedder's value-size limit, charged by the builders whose result can
//! outgrow what they were given.
//!
//! A builder that joins many values, or multiplies one, can be asked for more
//! memory than the process has while the command and clock budgets are still
//! nearly full; these builders measure what they would build and refuse
//! before allocating. Every other value is measured by its runtime once built.

use tcl_syntax::value::ValueOps;

use crate::CmdError;

/// The error a builder refused by the value-size limit fails with.
pub const VALUE_SIZE_EXCEEDED: &str = "value size limit exceeded";

/// Charge a value about to be built against the value-size limit. `size`
/// measures it, counting no further than just past the cap it is given, and
/// runs only when a limit is set.
///
/// # Errors
/// [`VALUE_SIZE_EXCEEDED`] when the value would be over the limit; the runtime
/// has recorded the breach.
pub fn charge<O: ValueOps>(
    ops: &mut O,
    size: impl FnOnce(&mut O, u64) -> u64,
) -> Result<(), CmdError> {
    let Some(limit) = ops.value_size_limit() else {
        return Ok(());
    };
    let bytes = size(ops, limit);
    if ops.admit_value_bytes(bytes) {
        Ok(())
    } else {
        Err(CmdError::new(VALUE_SIZE_EXCEEDED))
    }
}

/// The bytes `values` take joined by `separator` bytes, stopping once past
/// `cap`.
pub fn joined<O: ValueOps>(ops: &mut O, values: &[O::Value], separator: u64, cap: u64) -> u64 {
    let mut total = 0u64;
    for (i, value) in values.iter().enumerate() {
        if i > 0 {
            total = total.saturating_add(separator);
        }
        total = total.saturating_add(ops.measure_value(value, cap.saturating_sub(total)));
        if total > cap {
            break;
        }
    }
    total
}
