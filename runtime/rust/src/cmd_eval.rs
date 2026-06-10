//! `eval` + `uplevel` — running scripts in the current / an enclosing scope.
//!
//! Both are **transparent**: the body's completion code (including `return`)
//! propagates unchanged (unlike a proc/`source`, which map `return`→Ok). `eval`
//! runs in the current scope; `uplevel ?level?` runs in an enclosing frame's
//! variable scope *and* namespace (C Tcl `tclProc.c` `Tcl_UplevelObjCmd`; the
//! Zig oracle's "restore caller ns + depth together" discovery — see
//! `tcltest-bringup.md`). Multiple args are space-joined (the `concat`-style
//! eval form). Level parsing is shared with `upvar` ([`crate::cmd_var::parse_level`]).
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::cmd_var::parse_level;
use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register `eval` and `uplevel`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"eval", eval_cmd);
    interp.register_builtin(b"uplevel", uplevel_cmd);
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

/// Join `args` with single spaces (the multi-arg `eval`/`uplevel` body form). A
/// single arg is used verbatim.
fn join_body(args: &[*mut TclObj]) -> Vec<u8> {
    if let [one] = args {
        return obj_bytes(*one);
    }
    let mut out = Vec::new();
    for (i, &a) in args.iter().enumerate() {
        if i > 0 {
            out.push(b' ');
        }
        out.extend_from_slice(&obj_bytes(a));
    }
    out
}

/// `eval arg ?arg ...?` — concatenate the args and evaluate in the current scope.
fn eval_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"eval arg ?arg ...?");
    }
    let script = join_body(&argv[1..]);
    // `eval` runs its body as its own `info frame` level (inheriting the
    // enclosing proc/level).
    let code = interp.eval_body(&script);
    if code == Code::Error {
        // `("eval" body line N)` — a body evaluated through a fresh frame.
        interp.append_body_frame(b"eval");
    }
    code
}

/// `uplevel ?level? arg ?arg ...?` — evaluate in an enclosing frame's scope.
/// The optional level is `#N` (absolute) or `N` (relative; default 1); it is
/// present iff the first arg is level-shaped (`#`digits / all-digits).
fn uplevel_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let usage = b"uplevel ?level? command ?arg ...?";
    if argv.len() < 2 {
        return wrong_args(interp, usage);
    }
    let spec = obj_bytes(argv[1]);
    let (target, body_start) = if looks_like_level(&spec) {
        match parse_level(&spec, interp.current_level()) {
            Some(t) => (t, 2),
            None => return bad_level(interp, &spec),
        }
    } else {
        // Default relative level 1 (one frame up from the active level).
        match interp.current_level().checked_sub(1) {
            Some(t) => (t, 1),
            None => return bad_level(interp, b"1"),
        }
    };
    if body_start >= argv.len() {
        return wrong_args(interp, usage);
    }
    let script = join_body(&argv[body_start..]);
    let code = interp.eval_uplevel(target, &script);
    if code == Code::Error {
        // `("uplevel" body line N)`.
        interp.append_body_frame(b"uplevel");
    }
    code
}

/// Whether `s` is level-shaped: `#` + digits, or all digits.
fn looks_like_level(s: &[u8]) -> bool {
    match s.strip_prefix(b"#") {
        Some(rest) => !rest.is_empty() && rest.iter().all(u8::is_ascii_digit),
        None => !s.is_empty() && s.iter().all(u8::is_ascii_digit),
    }
}

fn bad_level(interp: &mut Interp, spec: &[u8]) -> Code {
    let mut m = b"bad level \"".to_vec();
    m.extend_from_slice(spec);
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
    fn eval_runs_in_current_scope() {
        leak_free(|i| {
            assert_eq!(run(i, b"eval {set x 5}"), b"5");
            assert_eq!(run(i, b"set x"), b"5");
            // multi-arg eval concatenates.
            assert_eq!(run(i, b"eval set y 7"), b"7");
            assert_eq!(run(i, b"set y"), b"7");
            i.eval_str(b"unset x y");
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn uplevel_runs_in_callers_scope() {
        leak_free(|i| {
            // setit uplevels into its caller's frame to set a local there.
            run(i, b"proc setit {} {uplevel 1 {set local fromproc}}");
            run(i, b"proc caller {} {setit; return $local}");
            assert_eq!(run(i, b"caller"), b"fromproc");
            // uplevel #0 reaches the global scope.
            run(i, b"proc setglobal {} {uplevel #0 {set g globalval}}");
            run(i, b"setglobal");
            assert_eq!(run(i, b"set g"), b"globalval");
            i.eval_str(b"unset g");
        });
    }

    #[test]
    fn uplevel_bad_level() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"uplevel #5 {set x 1}"), Code::Error);
            assert_eq!(i.result_bytes(), b"bad level \"#5\"");
            // from the global scope, default level 1 has no caller.
            assert_eq!(i.eval_str(b"uplevel {set x 1}"), Code::Error);
            assert_eq!(i.result_bytes(), b"bad level \"1\"");
        });
    }
}
