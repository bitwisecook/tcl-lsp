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

//! `ValueOps` for the VM — binds the portable `tcl-cmd-core` command logic to
//! the VM's `Rc<Obj>` value model.
//!
//! Value construction needs no interpreter state, so the seam is implemented
//! directly on [`Vm`] (the natural `ops` object a builtin already holds). The
//! copy-on-write asymmetry is explicit: the `Rc`-handle model cannot grow a
//! buffer in place, so [`ValueOps::try_append_bytes_in_place`] /
//! [`ValueOps::try_list_append_in_place`] keep their default (`false`) and
//! callers build a fresh value — the contrast with the WASM runtime's amortised
//! in-place growth that the contract is designed around. The VM's value is a
//! UTF-8 `Rc<str>`, so `as_bytes`/`new_bytes` use their (string-rep) defaults.

use std::rc::Rc;

use tcl_syntax::value::{ValueError, ValueOps};

use crate::interp::Vm;
use crate::value::Value;

/// `incr` / `dict incr` addition over the same integer tower `expr` uses: a
/// sum past `i64` promotes to `i128` (e.g. `incr` at `i64::MAX` yields
/// `9223372036854775808`), and one past `i128` promotes to an
/// **arbitrary-precision bignum** rather than erroring — matching tclsh
/// (). A free function so the
/// `dict incr` paths (command and `DICT_INCR_IMM` opcode) share it without
/// needing the [`ValueOps`] receiver.
pub(crate) fn int_add(a: Option<&Value>, b: &Value) -> Result<Value, ValueError> {
    // Fast `i128` tier: both operands fit and the sum doesn't overflow.
    let x_small = a.map_or(Some(0), Value::as_i128);
    if let (Some(x), Some(y)) = (x_small, b.as_i128())
        && let Some(sum) = x.checked_add(y)
    {
        return Ok(crate::expr::int_value(sum));
    }
    // Bignum tier: an operand (or the sum) exceeds `i128`. A non-integer
    // operand is still the `NotInteger` error — the stored value's error
    // surfacing before the increment's, as in C.
    let to_big = |v: &Value| {
        crate::expr::value_as_bigint(v)
            .ok_or_else(|| ValueError::NotInteger(v.to_str().to_string()))
    };
    let xb = match a {
        Some(v) => to_big(v)?,
        None => num_bigint::BigInt::from(0),
    };
    let yb = to_big(b)?;
    Ok(crate::expr::big_value(&(xb + yb)))
}

impl ValueOps for Vm {
    type Value = Value;

    fn new_str(&mut self, s: &str) -> Value {
        Value::string(s)
    }

    fn new_string(&mut self, s: String) -> Value {
        Value::string(s)
    }

    fn new_int(&mut self, n: i64) -> Value {
        Value::int(n)
    }

    fn new_double(&mut self, f: f64) -> Value {
        Value::double(f)
    }

    fn new_bool(&mut self, b: bool) -> Value {
        Value::bool(b)
    }

    fn new_list(&mut self, items: Vec<Value>) -> Value {
        Value::list(items)
    }

    fn as_str(&mut self, v: &Value) -> Rc<str> {
        v.to_str()
    }

    fn as_int(&mut self, v: &Value) -> Result<i64, ValueError> {
        v.as_int()
            .map_err(|_| ValueError::NotInteger(v.to_str().to_string()))
    }

    /// The tower addition shared with `dict incr` — see the free
    /// [`int_add`], which this merely re-exposes at the `ValueOps` seam.
    fn int_add(&mut self, a: Option<&Value>, b: &Value) -> Result<Value, ValueError> {
        int_add(a, b)
    }

    fn as_double(&mut self, v: &Value) -> Result<f64, ValueError> {
        v.as_double()
            .map_err(|_| ValueError::NotDouble(v.to_str().to_string()))
    }

    fn as_bool(&mut self, v: &Value) -> Result<bool, ValueError> {
        v.as_bool()
            .map_err(|_| ValueError::NotBoolean(v.to_str().to_string()))
    }

    fn list_elements(&mut self, v: &Value) -> Result<Vec<Value>, ValueError> {
        v.as_list()
            .map(|items| items.as_ref().clone())
            .map_err(|e| ValueError::BadList(e.message))
    }
}
