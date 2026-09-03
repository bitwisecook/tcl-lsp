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

//! `regexp` / `regsub` — a thin adapter over the shared
//! [`tcl_cmd_core::regex`] plumbing, driving it with the **pure-Rust Tcl 9 ARE
//! engine** ([`tcl_regex`]).
//!
//! The command logic — option parsing, the match/advance loop, `-indices`/
//! `-inline`/`-start`/`-all` handling, submatch-variable assignment, and the
//! `regsub` substitution-spec expansion — lives once in `tcl-cmd-core`; this
//! file wires in the engine via [`tcl_regex::cmd_core::AreEngine`] and supplies
//! the two per-runtime edges that stay Family-B state: the match-variable /
//! result-variable writes (with the const-variable check and refcount
//! discipline) and the result protocol.
//!
//! The engine was previously the C Henry-Spencer engine linked in by `build.rs`
//! (and stubbed out on wasm32, where the C FFI cannot link); it is now the
//! safe-Rust `tcl-regex` crate, which works on every target and is validated
//! against tclsh 9.0 (`reg.test`). The same engine is re-exported to C via the
//! C-ABI shim in [`crate::regex_capi`].

use crate::interp::{drop_fresh, obj_bytes, Code, Interp};
use crate::obj::{new_string_bytes, new_wide_int_obj, TclObj};
use tcl_cmd_core::regex::{self as core_re, RegexpResult, RegsubResult};

/// The pure-Rust Tcl 9 ARE engine as the shared plumbing's [`RegexEngine`]
/// provider. Reused by `lsearch -regexp` (`cmd_list`) and `switch -regexp`
/// (`cmd_switch`).
pub(crate) use tcl_regex::cmd_core::AreEngine;

/// Register `regexp` and `regsub`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"regexp", regexp_cmd);
    interp.register_builtin(b"regsub", regsub_cmd);
}

fn regexp_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let args: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    let refs: Vec<&[u8]> = args.iter().map(Vec::as_slice).collect();
    match core_re::regexp::<Interp, AreEngine>(interp, &refs) {
        Ok(RegexpResult::Inline(v)) => {
            interp.set_result(v);
            Code::Ok
        }
        Ok(RegexpResult::Count { assign, count }) => {
            if let Some(pairs) = assign {
                let mut it = pairs.into_iter();
                while let Some((name, val)) = it.next() {
                    // `arr(a)` writes the array *element*, not a literal
                    // scalar named `arr(a)` (issue #1577) — the same
                    // `split_array_ref` + `var_set`/`var_set_elem` routing
                    // `set` uses, so this doesn't hand-roll a second name
                    // parser.
                    let (base, elem) = crate::frame::split_array_ref(&name);
                    let stored = match &elem {
                        Some(k) => interp.var_set_elem(&base, k, val),
                        None => interp.var_set(&base, val),
                    };
                    if stored.is_err() {
                        // `var_set`/`var_set_elem` do not consume on error;
                        // drop this value and every still-unconsumed one to
                        // stay leak-free.
                        drop_fresh(val);
                        for (_, v) in it {
                            drop_fresh(v);
                        }
                        return interp.set_error(b"couldn't set match variable");
                    }
                }
            }
            set_int(interp, count);
            Code::Ok
        }
        Err(e) => interp.set_error(&e.0),
    }
}

fn regsub_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let args: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    let refs: Vec<&[u8]> = args.iter().map(Vec::as_slice).collect();
    let RegsubResult { text, count, var } = match core_re::regsub::<AreEngine>(&refs) {
        Ok(r) => r,
        Err(e) => return interp.set_error(&e.0),
    };

    match var {
        Some(name) => {
            // A constant target is rejected with the standard message (a write
            // trace / array mismatch is reported by `var_error`).
            if let Some(c) = interp.const_write_check(&name) {
                return c;
            }
            // `arr(k)` writes the array *element*, not a literal scalar named
            // `arr(k)` (issue #1577's shape, R4's fix elsewhere) — the same
            // `split_array_ref` + `var_set`/`var_set_elem` routing `set` and
            // `regexp`'s match-var loop use, so this doesn't hand-roll a
            // second name parser.
            let (base, elem) = crate::frame::split_array_ref(&name);
            let o = new_string_bytes(&text);
            let stored = match &elem {
                Some(k) => interp.var_set_elem(&base, k, o),
                None => interp.var_set(&base, o),
            };
            match stored {
                Ok(()) => {
                    set_int(interp, count);
                    Code::Ok
                }
                Err(e) => {
                    drop_fresh(o);
                    crate::builtins::var_error(interp, &name, e)
                }
            }
        }
        None => {
            interp.set_result(new_string_bytes(&text));
            Code::Ok
        }
    }
}

fn set_int(interp: &mut Interp, n: i64) {
    interp.set_result(new_wide_int_obj(n));
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
            "eval {:?} → {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    #[test]
    fn regexp_match_and_captures() {
        leak_free(|i| {
            assert_eq!(ok(i, b"regexp {ab+c} xxabbbcyy"), b"1");
            assert_eq!(ok(i, b"regexp {z} abc"), b"0");
            ok(i, br"regexp {(\w+)@(\w+)} user@host m u h");
            assert_eq!(ok(i, b"set m"), b"user@host");
            assert_eq!(ok(i, b"set u"), b"user");
            assert_eq!(ok(i, b"set h"), b"host");
        });
    }

    #[test]
    fn regexp_all_inline_indices_nocase() {
        leak_free(|i| {
            assert_eq!(ok(i, b"regexp -all {a} banana"), b"3");
            assert_eq!(ok(i, br"regexp -inline {(\d+)} abc123def"), b"123 123");
            ok(i, b"regexp -indices {bc} abcd m");
            assert_eq!(ok(i, b"set m"), b"1 2");
            assert_eq!(ok(i, b"regexp -nocase {ABC} xabcy"), b"1");
        });
    }

    #[test]
    fn regexp_nomatch_leaves_vars_untouched() {
        // tclsh: a failed match does not modify the match variables.
        leak_free(|i| {
            ok(i, b"set m PRESET");
            assert_eq!(ok(i, b"regexp {z} abc m"), b"0");
            assert_eq!(ok(i, b"set m"), b"PRESET");
        });
    }

    #[test]
    fn regsub_basic_all_and_backrefs() {
        leak_free(|i| {
            assert_eq!(ok(i, b"regsub {b} abc X"), b"aXc");
            assert_eq!(ok(i, b"regsub -all {a} banana _"), b"b_n_n_");
            assert_eq!(
                ok(i, br"regsub {(\w+)@(\w+)} user@host {\2.\1}"),
                b"host.user"
            );
            assert_eq!(
                ok(i, b"regsub -all {[aeiou]} {hello world} {}"),
                b"hll wrld"
            );
            // with a result variable, returns the match count.
            assert_eq!(ok(i, b"regsub -all {a} banana _ out"), b"3");
            assert_eq!(ok(i, b"set out"), b"b_n_n_");
            // no match leaves the string unchanged.
            assert_eq!(ok(i, b"regsub {z} abc X"), b"abc");
            // anchor edge: `^` matches once at the start (notbol suppresses it
            // at resumed offsets), per tclsh.
            assert_eq!(ok(i, b"regsub -all {^} abc >"), b">abc");
        });
    }

    #[test]
    fn start_option() {
        leak_free(|i| {
            assert_eq!(ok(i, b"regexp -start 3 {a} {a a a}"), b"1");
            assert_eq!(ok(i, b"regsub -start 2 -all {a} aaaa X"), b"aaXX");
            assert_eq!(ok(i, b"regexp -start 1+1 {a} aaaa"), b"1");
            assert_eq!(ok(i, b"regsub -start 0x2 {a} aaaa X"), b"aaXa");
            assert_eq!(i.eval_str(b"regexp -start bogus {a} aaaa"), Code::Error);
            assert!(i.result_bytes().starts_with(b"bad index \"bogus\""));
            assert_eq!(
                i.eval_str(b"regsub -start {end - 2} {a} aaaa X"),
                Code::Error
            );
            assert!(i.result_bytes().starts_with(b"bad index \"end - 2\""));
        });
    }

    #[test]
    fn bad_pattern_errors() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"regexp {a(} b"), Code::Error);
            assert!(i
                .result_bytes()
                .starts_with(b"cannot compile regular expression pattern"));
        });
    }
}
