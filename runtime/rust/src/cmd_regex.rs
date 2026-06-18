//! `regexp` / `regsub` — a thin adapter over the shared
//! [`tcl_cmd_core::regex`] plumbing, driving it with the **real Tcl 9
//! Henry-Spencer ARE engine** (linked via `build.rs`, gated `have_regex`).
//!
//! The command logic — option parsing, the match/advance loop, `-indices`/
//! `-inline`/`-start`/`-all` handling, submatch-variable assignment, and the
//! `regsub` substitution-spec expansion — lives once in `tcl-cmd-core`; this
//! file supplies the engine ([`AreEngine`], a `RegexEngine` over
//! [`crate::regex`]) and the two per-runtime edges that stay Family-B state:
//! the match-variable / result-variable writes (with the const-variable check
//! and refcount discipline) and the result protocol.
//!
//! The Zig runtime (`runtime/zig/valtypes/tcl_regex.zig`) and tclsh 9.0 are the
//! behavioural oracles. This is the M3 regex wall in
//! `docs/design/runtime/tcltest-bringup.md`.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{drop_fresh, obj_bytes, Code, Interp};
use crate::obj::{new_string_bytes, new_wide_int_obj, TclObj};
use crate::regex::{Regex, REG_EXPANDED, REG_ICASE, REG_NLANCH, REG_NLSTOP};
use tcl_cmd_core::regex::{
    self as core_re, RegMatch, RegexEngine, RegexFlags, RegexpResult, RegsubResult,
};

/// Register `regexp` and `regsub`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"regexp", regexp_cmd);
    interp.register_builtin(b"regsub", regsub_cmd);
}

/// The real Tcl 9 ARE engine, presented as the shared plumbing's
/// [`RegexEngine`] provider. Maps the engine-neutral [`RegexFlags`] onto the
/// engine's compile flags and bridges its codepoint-native [`crate::regex`]
/// match vector to the core's [`RegMatch`].
struct AreEngine;

impl RegexEngine for AreEngine {
    type Regex = Regex;

    fn compile(pattern: &[u8], flags: RegexFlags) -> Result<Regex, Vec<u8>> {
        let mut cflags = 0i32;
        if flags.nocase {
            cflags |= REG_ICASE;
        }
        if flags.expanded {
            cflags |= REG_EXPANDED;
        }
        if flags.linestop {
            cflags |= REG_NLSTOP;
        }
        if flags.lineanchor {
            cflags |= REG_NLANCH;
        }
        Regex::compile(pattern, cflags)
    }

    fn nsub(re: &Regex) -> usize {
        re.nsub()
    }

    fn exec(re: &mut Regex, cps: &[i32], offset: usize, notbol: bool) -> Option<Vec<RegMatch>> {
        re.exec(cps, offset, notbol).map(|v| {
            v.iter()
                .map(|m| RegMatch { so: m.so, eo: m.eo })
                .collect()
        })
    }
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
                    if interp.var_set(&name, val).is_err() {
                        // `var_set` does not consume on error; drop this value
                        // and every still-unconsumed one to stay leak-free.
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
            let o = new_string_bytes(&text);
            match interp.var_set(&name, o) {
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
