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

//! Byte-preserving completion capture at the public runtime boundary.
//!
//! The tree-walking runtime keeps an interpreter-owned result object
//! internally. This module projects the existing Family-B object completion to
//! the public byte boundary, so object-handle consumers and byte-oriented
//! embedders observe the same code, result, and return-options snapshot.

use tcl_runtime_api::ScriptCompletion;

use crate::interp::{Code as RuntimeCode, Interp};
use crate::obj;

/// Snapshot one completion as exact, owned Tcl string-representation bytes.
pub(crate) fn capture_bytes(interp: &mut Interp, code: RuntimeCode) -> ScriptCompletion {
    let completion = crate::state_traits::capture_completion(interp, code);
    let result = obj::bytes_of(completion.result);
    let options = obj::bytes_of(completion.options);
    // SAFETY: `capture_completion` returned one owned reference to each object;
    // both byte projections are complete, so release those references now.
    unsafe {
        obj::decr_ref_count(completion.result);
        obj::decr_ref_count(completion.options);
    }
    ScriptCompletion::new(completion.code, result, options)
}
