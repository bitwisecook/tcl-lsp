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
//! The engine is the safe-Rust `tcl-regex` crate, which works on every
//! target — unlike a linked-in C engine, which would need stubbing out on
//! wasm32 where the C FFI cannot link — and is validated against tclsh 9.0
//! (`reg.test`). The same engine is re-exported to C via the C-ABI shim in
//! [`crate::regex_capi`].

use crate::interp::{drop_fresh, new_string, obj_bytes, Code, Interp};
use crate::obj::{self, new_string_bytes, new_wide_int_obj, TclObj};
use tcl_cmd_core::regex::{self as core_re, RegexpResult, RegsubError, RegsubResult};

/// The `errorInfo` frame C appends when a `regsub -command` prefix fails
/// (`Tcl_RegsubObjCmd`'s `Tcl_AppendObjToErrorInfo`). tclsh 9.0.4 / 9.1b0,
/// `regsub -command {.x.} {abcxdef} error`:
///
/// ```text
/// cxd
///     while executing
/// "error cxd"
///     (-command substitution computation script)
///     invoked from within
/// "regsub -command {.x.} {abcxdef} error"
/// ```
const COMMAND_SUBST_FRAME: &[u8] = b"-command substitution computation script";

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
                    // scalar named `arr(a)` — the same `split_array_ref` +
                    // `var_set`/`var_set_elem` routing `set` uses, so this
                    // doesn't hand-roll a second name parser.
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
    // The core owns `-command` (option table, prefix split, per-match word
    // list); this adapter supplies only the evaluator and the release the
    // interpreter is pinned to, which is what decides whether `-command` is an
    // option at all (a `bad switch` through 8.5, `bad option` on 8.6, served
    // from 9.0).
    let version = interp.runtime_version();
    let outcome = core_re::regsub_eval::<AreEngine, Code>(&refs, version, |words| {
        regsub_command_call(interp, words)
    });
    let RegsubResult { text, count, var } = match outcome {
        Ok(r) => r,
        Err(RegsubError::Regex(e)) => return interp.set_error(&e.0),
        Err(RegsubError::Eval(code)) => {
            // C adds the context frame only for a genuine error; a
            // `break`/`continue`/custom code from the prefix propagates
            // untouched (tclsh 9.0.4: `proc q args {return -code continue}`,
            // `catch {regsub -command {.x.} abcxdef q}` → 4, `::errorInfo`
            // never set).
            if code == Code::Error {
                interp.append_frame_noline(COMMAND_SUBST_FRAME);
            }
            return code;
        }
    };

    match var {
        Some(name) => {
            // A constant target is rejected with the standard message (a write
            // trace / array mismatch is reported by `var_error`).
            if let Some(c) = interp.const_write_check(&name) {
                return c;
            }
            // `arr(k)` writes the array *element*, not a literal scalar named
            // `arr(k)` — the same `split_array_ref` +
            // `var_set`/`var_set_elem` routing `set` and `regexp`'s
            // match-var loop use, so this doesn't hand-roll a second name
            // parser.
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

/// Evaluate one `regsub -command` substitution: `words` is the whole command —
/// the prefix's own words followed by the matched text and each submatch — and
/// its result is the replacement text. Argv-based (`Interp::dispatch`), so a
/// word containing `$`/`[` is passed literally, exactly as C's `Tcl_EvalObjv`
/// does. The freshly-built words are ref-counted across the dispatch and
/// released afterwards, the same discipline `lsort -command` uses.
fn regsub_command_call(interp: &mut Interp, words: &[Vec<u8>]) -> Result<Vec<u8>, Code> {
    let call: Vec<*mut TclObj> = words.iter().map(|w| new_string(w)).collect();
    for &o in &call {
        unsafe { obj::incr_ref_count(o) };
    }
    let code = interp.dispatch(&call);
    let result = interp.result_bytes();
    for &o in &call {
        unsafe { obj::decr_ref_count(o) };
    }
    if code == Code::Ok {
        Ok(result)
    } else {
        Err(code)
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

    /// `regsub -command` evaluates the prefix once per substitution with the
    /// whole match and each submatch appended. Verified on tclsh 9.0.4 and
    /// 9.1b0:
    ///
    /// ```text
    /// % regsub -command {.x.} {abcxdef} {string length}
    /// ab3ef
    /// % regsub -command -all {(.)(.)} {abcdef} {list ,}
    /// , ab a b, cd c d, ef e f
    /// % regsub -command {(a)|(b)} ab {list <}
    /// < a a {}b
    /// % set n [regsub -command {.x.} abcxdef {string length} out]; list $n $out
    /// 1 ab3ef
    /// % regsub -command {z} abc {string toupper}
    /// abc
    /// ```
    #[test]
    fn regsub_command_evaluates_the_prefix() {
        leak_free(|i| {
            assert_eq!(
                ok(i, b"regsub -command {.x.} {abcxdef} {string length}"),
                b"ab3ef"
            );
            assert_eq!(
                ok(i, b"regsub -command -all {(.)(.)} {abcdef} {list ,}"),
                b", ab a b, cd c d, ef e f"
            );
            // A submatch that did not participate is an empty word, not a
            // missing one — the prefix still sees one word per submatch.
            assert_eq!(
                ok(i, b"regsub -command {(a)|(b)} ab {list <}"),
                b"< a a {}b"
            );
            assert_eq!(
                ok(
                    i,
                    b"set n [regsub -command {.x.} abcxdef {string length} out]; list $n $out"
                ),
                b"1 ab3ef"
            );
            // A pattern that never matches never calls the prefix.
            assert_eq!(ok(i, b"regsub -command {z} abc {string toupper}"), b"abc");
        });
    }

    /// A script error inside the `-command` prefix propagates, and C's context
    /// frame is appended to `errorInfo`. tclsh 9.0.4 / 9.1b0:
    ///
    /// ```text
    /// % proc boomp args { error boom }
    /// % catch {regsub -command {.x.} abcxdef boomp} e; set ::errorInfo
    /// boom
    ///     while executing
    /// "error boom "
    ///     (procedure "boomp" line 1)
    ///     invoked from within
    /// "boomp cxd"
    ///     (-command substitution computation script)
    ///     invoked from within
    /// "regsub -command {.x.} abcxdef boomp"
    /// ```
    ///
    /// The trace omits the `invoked from within "boomp cxd"` frame because the
    /// prefix is invoked argv-wise (`Interp::dispatch`), which carries no
    /// source text to quote — the same pre-existing shape `lsort -command`
    /// has. The message, the `(-command substitution computation script)`
    /// frame and its position before the enclosing command's frame all match
    /// C.
    #[test]
    fn regsub_command_error_appends_the_c_error_info_trailer() {
        leak_free(|i| {
            let got = ok(
                i,
                b"proc boomp args { error boom }\n\
                  catch {regsub -command {.x.} abcxdef boomp} e\n\
                  list $e $::errorInfo",
            );
            assert_eq!(
                String::from_utf8_lossy(&got),
                "boom {boom\n    while executing\n\"error boom \"\n    \
                 (procedure \"boomp\" line 1)\n    \
                 (-command substitution computation script)\n    \
                 invoked from within\n\"regsub -command {.x.} abcxdef boomp\"}"
            );
        });
    }

    /// A non-error completion code from the prefix propagates untouched, with
    /// no `errorInfo` trailer. tclsh 9.0.4:
    ///
    /// ```text
    /// % proc q args { return -code continue }
    /// % catch {regsub -command {.x.} {abcxdef} q} r
    /// 4
    /// % info exists ::errorInfo
    /// 0
    /// ```
    #[test]
    fn regsub_command_non_error_code_propagates_untouched() {
        leak_free(|i| {
            let got = ok(
                i,
                b"proc q args { return -code continue }\n\
                  list [catch {regsub -command {.x.} {abcxdef} q} r] $r \
                  [info exists ::errorInfo]",
            );
            assert_eq!(String::from_utf8_lossy(&got), "4 {} 0");
        });
    }

    /// `-command` is a 9.0 option: before it, `regsub` rejects it in that
    /// release's own noun and enumeration. The adapter passes the
    /// interpreter's pinned release to the core, so the refusal follows
    /// `info patchlevel`:
    ///
    /// ```text
    /// $ tclsh8.4 / tclsh8.5   (8.4.20 / 8.5.19)
    /// % catch {regsub -command {a} abc {string toupper}} r; set r
    /// bad switch "-command": must be -all, -nocase, -expanded, -line, -linestop, -lineanchor, -start, or --
    /// $ tclsh8.6              (8.6.18)
    /// bad option "-command": must be -all, -nocase, -expanded, -line, -linestop, -lineanchor, -start, or --
    /// $ tclsh9.0 / tclsh9.1   (9.0.4 / 9.1b0)
    /// Abc
    /// ```
    #[test]
    fn regsub_command_is_a_9_0_option() {
        use tcl_dialect::TclVersion;
        const ENUM: &str =
            "must be -all, -nocase, -expanded, -line, -linestop, -lineanchor, -start, or --";
        const SRC: &[u8] = b"regsub -command {a} abc {string toupper}";
        for (version, noun) in [
            (TclVersion::V8_4, "switch"),
            (TclVersion::V8_5, "switch"),
            (TclVersion::V8_6, "option"),
        ] {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(i.eval_str(SRC), Code::Error, "for {version:?}");
                assert_eq!(
                    String::from_utf8_lossy(&i.result_bytes()),
                    format!("bad {noun} \"-command\": {ENUM}"),
                    "for {version:?}"
                );
            });
        }
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(i.eval_str(SRC), Code::Ok, "for {version:?}");
                assert_eq!(i.result_bytes(), b"Abc", "for {version:?}");
            });
        }
    }
}
