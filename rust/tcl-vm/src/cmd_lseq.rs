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

//! `lseq` — the arithmetic-sequence generator, newly added to the VM over the
//! shared [`tcl_cmd_core::lseq`] core. The VM had no `lseq` before; it now gets
//! every form (`lseq 5`, `lseq 1 to 10 by 2`, `lseq 0 0.5 by 0.1`,
//! expression-valued arguments) over its `i64`+`double` number model — the same
//! core the WASM runtime drives over its bignum tower.
//!
//! The adapter supplies the two per-runtime edges: evaluating an
//! expression-valued argument through `vm.eval_expr`, and constructing the
//! element list over the VM's `ValueOps`.

use tcl_runtime_api::Completion;

use tcl_cmd_core::lseq::{self, LseqError, Num};

use crate::command::completion_from_tcl_error;
use crate::error::TclError;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// Register `lseq`.
pub(crate) fn register(vm: &mut Vm) {
    vm.register("lseq", cmd_lseq);
}

fn cmd_lseq(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // The VM's argv is already name-stripped; snapshot each arg's string rep so
    // the byte slices the core sees don't borrow through the values.
    let bytes: Vec<Vec<u8>> = args
        .iter()
        .map(|v| v.to_str().as_bytes().to_vec())
        .collect();
    let refs: Vec<&[u8]> = bytes.iter().map(Vec::as_slice).collect();

    // Decode first (the eval callback borrows the vm), then generate (the vm is
    // borrowed as the value-ops) — two separate calls, so no borrow conflict.
    let plan = match lseq::decode(&refs, |src| eval_num(vm, src)) {
        Ok(p) => p,
        Err(LseqError::Message(m)) => return err(String::from_utf8_lossy(&m).into_owned()),
        Err(LseqError::Eval(e)) => return completion_from_tcl_error(e),
    };
    match lseq::generate(vm, &plan) {
        Ok(v) => ok(v),
        Err(m) => err(String::from_utf8_lossy(m).into_owned()),
    }
}

/// Evaluate `src` as an expression and classify its result as a number — the
/// `lseq` expression-valued-argument edge. `Ok(None)` = evaluated but not a
/// number (the core maps that to a syntax error); `Err` = the evaluation failed.
fn eval_num(vm: &mut Vm, src: &[u8]) -> Result<Option<Num>, TclError> {
    let s =
        core::str::from_utf8(src).map_err(|_| TclError::new("expr operand is not valid UTF-8"))?;
    let v = vm.eval_expr(s)?;
    Ok(lseq::as_number(v.to_str().as_bytes()))
}
