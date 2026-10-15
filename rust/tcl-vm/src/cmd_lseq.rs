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

//! `lseq` — the arithmetic-sequence generator, over the
//! shared [`tcl_cmd_core::lseq`] core. Supports
//! every form (`lseq 5`, `lseq 1 to 10 by 2`, `lseq 0 0.5 by 0.1`) over its
//! `i64`+`double` number model — the same core the WASM runtime drives over
//! its bignum tower. The adapter only constructs the element list over the
//! VM's `ValueOps`.

use tcl_runtime_api::Completion;

use tcl_cmd_core::lseq;

use crate::command::completion_from_cmd_error;
use crate::interp::{Vm, ok};
use crate::value::Value;

/// Register `lseq`.
pub(crate) fn register(vm: &mut Vm) {
    vm.register("lseq", cmd_lseq);
}

fn cmd_lseq(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // Snapshot each arg's string rep so the byte slices the core sees don't
    // borrow through the values.
    let bytes: Vec<Vec<u8>> = args
        .iter()
        .map(|v| v.to_str().as_bytes().to_vec())
        .collect();
    let refs: Vec<&[u8]> = bytes.iter().map(Vec::as_slice).collect();
    match lseq::decode(&refs).and_then(|plan| lseq::generate(vm, &plan)) {
        Ok(v) => ok(v),
        Err(e) => completion_from_cmd_error(e),
    }
}
