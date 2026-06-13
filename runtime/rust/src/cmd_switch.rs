//! `switch` — multi-way branch (toward running tcltest). C ref `tclCmdMZ.c`
//! (`Tcl_SwitchObjCmd`).
//!
//! `switch ?options? string pattern body ?pattern body ...?` or
//! `switch ?options? string {pattern body ...}`. A `default` pattern matches
//! anything; a body of `-` falls through to the next pattern's body. The chosen
//! body is a **script** evaluated in the current scope (transparent — its code,
//! incl. `return`/`break`/`continue`, propagates). Modes: `-exact` (default),
//! `-glob`, `-nocase`, `--`. (`-regexp` lands with the regex engine.)
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register `switch`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"switch", switch_cmd);
}

#[derive(Clone, Copy)]
enum Mode {
    Exact,
    Glob,
}

fn switch_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let usage = b"switch ?-exact|-glob|-nocase|--? string {?pattern body ...?}";
    // Parse leading options.
    let mut mode = Mode::Exact;
    let mut nocase = false;
    let mut i = 1;
    while i < argv.len() {
        let a = obj_bytes(argv[i]);
        match a.as_slice() {
            b"-exact" => mode = Mode::Exact,
            b"-glob" => mode = Mode::Glob,
            b"-nocase" => nocase = true,
            b"-regexp" => return interp.set_error(b"switch -regexp is not yet supported"),
            b"--" => {
                i += 1;
                break;
            }
            opt if opt.starts_with(b"-") => {
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(opt);
                m.extend_from_slice(b"\": must be -exact, -glob, -nocase, or --");
                return interp.set_error(&m);
            }
            _ => break,
        }
        i += 1;
    }
    if i >= argv.len() {
        return wrong_args(interp, usage);
    }
    let string = obj_bytes(argv[i]);
    i += 1;

    // Collect the pattern/body words: either a single list arg, or the rest
    // inline. Keep them as owned byte vectors (the list form is split once).
    let words: Vec<Vec<u8>> = if i + 1 == argv.len() {
        match crate::parse::split_list(&obj_bytes(argv[i])) {
            Ok(v) => v,
            Err(e) => return interp.set_error(e.message()),
        }
    } else {
        argv[i..].iter().map(|&a| obj_bytes(a)).collect()
    };
    if words.is_empty() || words.len() % 2 != 0 {
        return interp.set_error(b"extra switch pattern with no body");
    }

    // Find the first matching pattern; `default` (as the last pattern) matches
    // anything.
    let npairs = words.len() / 2;
    for p in 0..npairs {
        let pat = &words[p * 2];
        let is_default = pat.as_slice() == b"default" && p == npairs - 1;
        if is_default || matches(mode, nocase, pat, &string) {
            // Resolve a `-` fall-through to the next non-`-` body.
            let mut b = p;
            while words[b * 2 + 1].as_slice() == b"-" {
                b += 1;
                if b >= npairs {
                    return interp.set_error(b"no body specified for pattern \"-\"");
                }
            }
            return interp.eval_str(&words[b * 2 + 1]);
        }
    }
    // No match → empty result.
    interp.set_result_bytes(b"");
    Code::Ok
}

fn matches(mode: Mode, nocase: bool, pat: &[u8], string: &[u8]) -> bool {
    match mode {
        Mode::Exact => {
            if nocase {
                pat.eq_ignore_ascii_case(string)
            } else {
                pat == string
            }
        }
        Mode::Glob => match (core::str::from_utf8(pat), core::str::from_utf8(string)) {
            (Ok(p), Ok(s)) => tcl_syntax::glob::string_case_match(p, s, nocase),
            _ => false,
        },
    }
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
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

    fn run(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    #[test]
    fn switch_exact_glob_default_fallthrough() {
        leak_free(|i| {
            // list form, exact.
            assert_eq!(
                run(i, b"switch b {a {set r A} b {set r B} c {set r C}}"),
                b"B"
            );
            // default (last) matches anything.
            assert_eq!(
                run(i, b"switch -- z {a {set r 1} default {set r def}}"),
                b"def"
            );
            // `-` falls through to the next body.
            assert_eq!(
                run(i, b"switch x {a {set r 1} x - y {set r both} z {set r 3}}"),
                b"both"
            );
            // glob mode.
            assert_eq!(
                run(
                    i,
                    b"switch -glob foobar {f* {set r glob} default {set r no}}"
                ),
                b"glob"
            );
            // nocase exact.
            assert_eq!(
                run(i, b"switch -nocase ABC {abc {set r m} default {set r no}}"),
                b"m"
            );
            // inline pattern/body form.
            assert_eq!(run(i, b"switch 2 1 {set r one} 2 {set r two}"), b"two");
            // no match → empty.
            assert_eq!(run(i, b"switch q {a {set r 1} b {set r 2}}"), b"");
            i.eval_str(b"unset r");
        });
    }

    #[test]
    fn switch_propagates_body_code() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"switch a {a {break} b {continue}}"),
                Code::Break
            );
        });
    }
}
