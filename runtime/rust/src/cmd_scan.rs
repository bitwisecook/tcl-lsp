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

//! `scan` — parse a string under a format (the `sscanf` analogue). The matching
//! engine is shared (`tcl_cmd_core::scan`); this is the runtime's thin adapter:
//! it reads argv as code points, calls [`scan_match`], then either assigns the
//! scanned values to `varName`s (returning the conversion count, `-1` on EOF
//! before any conversion) or, with no vars, collects them into a list (*inline*
//! mode).

use tcl_cmd_core::scan::{scan_match, validate_format, Scanned};

use crate::interp::{drop_fresh, obj_bytes, Code, Interp};
use crate::obj::{new_double_obj, new_string_bytes, new_wide_int_obj, TclObj};

/// Register `scan`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"scan", scan_cmd);
}

/// Build the runtime object for a scanned value: `%d`→wide int, `%f`→double,
/// `%s`/`%[`→string. Returned fresh (rc-0); the caller adopts it.
fn scanned_obj(v: &Scanned) -> *mut TclObj {
    match v {
        Scanned::Int(n) => new_wide_int_obj(*n),
        Scanned::Double(d) => new_double_obj(*d),
        Scanned::Str(s) => new_string_bytes(s.as_bytes()),
    }
}

fn scan_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"scan string format ?varName ...?");
    }
    let input: Vec<char> = String::from_utf8_lossy(&obj_bytes(argv[1]))
        .chars()
        .collect();
    let fmt: Vec<char> = String::from_utf8_lossy(&obj_bytes(argv[2]))
        .chars()
        .collect();
    let vars = &argv[3..];
    let inline = vars.is_empty();

    // Reject malformed format strings up front, as C's `ValidateFormat` does.
    if let Err(msg) = validate_format(&fmt, vars.len()) {
        return interp.set_error(msg.as_bytes());
    }

    let outcome = scan_match(&input, &fmt);

    if inline {
        // The list of scanned values; a failed (trailing) conversion renders as
        // an empty string, but an outright EOF-before-anything is empty (the
        // analogue of variable mode's -1).
        if outcome.values.is_empty() || (outcome.nconv == 0 && outcome.eof_before_conv) {
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
        let objs: Vec<*mut TclObj> = outcome
            .values
            .iter()
            .map(|v| {
                v.as_ref()
                    .map_or_else(|| new_string_bytes(b""), scanned_obj)
            })
            .collect();
        interp.set_result(crate::list::new_list_obj(&objs));
        return Code::Ok;
    }

    // Variable mode: assign each successful value to its var, return the count
    // (or -1 if EOF hit before any conversion matched).
    if outcome.nconv == 0 && outcome.eof_before_conv {
        interp.set_result(new_wide_int_obj(-1));
        return Code::Ok;
    }
    let mut assigned = 0;
    for (v, &var) in outcome.values.iter().zip(vars.iter()) {
        let Some(value) = v else { break };
        let name = obj_bytes(var);
        let o = scanned_obj(value);
        if let Err(e) = interp.var_set(&name, o) {
            drop_fresh(o);
            // Surface the real variable error (`: variable is array`, …) the way
            // the VM and the other writers do (scan-8.12..8.16).
            return crate::builtins::var_error(interp, &name, e);
        }
        assigned += 1;
    }
    interp.set_result(new_wide_int_obj(assigned as i64));
    Code::Ok
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }
    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?} -> {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    #[test]
    fn scan_basic_conversions() {
        leak_free(|i| {
            assert_eq!(ok(i, b"scan {42 abc} {%d %s} a b"), b"2");
            assert_eq!(ok(i, b"set a"), b"42");
            assert_eq!(ok(i, b"set b"), b"abc");
            assert_eq!(ok(i, b"scan 0xff %x v"), b"1");
            assert_eq!(ok(i, b"set v"), b"255");
            assert_eq!(ok(i, b"scan Z %c c"), b"1");
            assert_eq!(ok(i, b"set c"), b"90");
            // inline mode returns the values as a list.
            assert_eq!(ok(i, b"scan {12 34} {%d %d}"), b"12 34");
            // scanset.
            assert_eq!(ok(i, b"scan hello123 {%[a-z]} w"), b"1");
            assert_eq!(ok(i, b"set w"), b"hello");
            // EOF before any conversion → -1.
            assert_eq!(ok(i, b"scan {} %d x"), b"-1");
            i.eval_str(b"unset -nocomplain a b c v w x");
        });
    }
}
