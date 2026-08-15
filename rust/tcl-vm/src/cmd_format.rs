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

//! The `scan` + `format` adapters.
//!
//! Both rendering directions now live in the shared `tcl_cmd_core`: `format`'s
//! output logic in `tcl_cmd_core::format` (over `ValueOps`) and `scan`'s
//! matching engine in `tcl_cmd_core::scan` (a pure code-point parser). This
//! module is the VM's thin adapter onto each.

use tcl_cmd_core::scan::{Scanned, scan_match};
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("format", cmd_format);
    vm.register("scan", cmd_scan);
}

/// `scan string format ?varName ...?` — parse `string` per the conversion
/// `format`. With `varName`s, assign each conversion and return the count (`-1`
/// on EOF before any conversion); with none ("inline" form), return the
/// conversions as a list. The matching engine is shared (`tcl_cmd_core::scan`),
/// so the VM and `runtime/rust` accept the same conversions.
fn cmd_scan(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (input, fmt, vars) = match args {
        [s, f, vars @ ..] => (s.to_str(), f.to_str(), vars),
        _ => return err("wrong # args: should be \"scan string format ?varName ...?\""),
    };
    let inp: Vec<char> = input.chars().collect();
    let fch: Vec<char> = fmt.chars().collect();
    // Reject malformed format strings up front, as C's `ValidateFormat` does.
    if let Err(msg) = tcl_cmd_core::scan::validate_format(&fch, vars.len()) {
        return err(msg);
    }
    let outcome = scan_match(&inp, &fch);
    if vars.is_empty() {
        // Inline form: the conversions as a list (a failed field is an empty
        // string); an outright EOF-before-anything is the empty string (the
        // analogue of variable mode's -1, scan-3.4).
        if outcome.values.is_empty() || (outcome.nconv == 0 && outcome.eof_before_conv) {
            return ok(Value::empty());
        }
        return ok(Value::list(
            outcome
                .values
                .iter()
                .map(|v| v.as_ref().map_or_else(Value::empty, scanned_value))
                .collect(),
        ));
    }
    // Variable form: -1 if EOF preceded any conversion; else assign and count.
    if outcome.nconv == 0 && outcome.eof_before_conv {
        return ok(Value::int(-1));
    }
    let mut count = 0;
    for (v, var) in outcome.values.iter().zip(vars.iter()) {
        let Some(value) = v else { break };
        if let Err(c) = vm.set_var(&var.to_str(), scanned_value(value)) {
            return c;
        }
        count += 1;
    }
    ok(Value::int(count))
}

/// Build the VM value for a scanned conversion (`%d`/`%x`/`%c`→int,
/// `%e`/`%f`/`%g`→double, `%s`/`%[`→string).
fn scanned_value(v: &Scanned) -> Value {
    match v {
        Scanned::Int(n) => Value::int(*n),
        Scanned::Double(d) => Value::double(*d),
        Scanned::Str(s) => Value::string(s.as_str()),
    }
}

fn cmd_format(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let syntax = vm.runtime_version().number_syntax();
    match tcl_cmd_core::format::format_cmd_with_syntax(vm, args, syntax) {
        Ok(v) => ok(v),
        Err(e) => err(e.into_message()),
    }
}
