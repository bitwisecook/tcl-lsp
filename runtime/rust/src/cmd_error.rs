//! `catch` + `error` — the exception foundation (PC-4, toward running tcltest).
//!
//! Modelled on C Tcl 9 (`tclCmdAH.c` `Tcl_CatchObjCmd`, `tclProc.c`/`tclResult.c`
//! `Tcl_ErrorObjCmd`) and the Zig oracle's catch discoveries
//! (`tcltest-bringup.md` appendix). `catch` snapshots the body's completion code
//! and result **before** resetting the interp result; `error` stamps the
//! `::errorInfo` / `::errorCode` globals on every error (`NONE` default).
//!
//! Conservative-first: the `-errorinfo` value is the message (or the explicit
//! info arg) — the incremental `while executing` / `invoked from within`
//! source-trace unwinder needs the `CmdFrame` source stack (PC-1) and lands with
//! it. `-errorstack`, `try`, `throw`, and full `return -options` follow.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::dict;
use crate::interp::{drop_fresh, new_string, obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register `catch`, `error`, `try`, and `throw`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"catch", catch_cmd);
    interp.register_builtin(b"error", error_cmd);
    interp.register_builtin(b"try", try_cmd);
    interp.register_builtin(b"throw", throw_cmd);
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

// -- catch -----------------------------------------------------------------

/// `catch script ?resultVarName? ?optionsVarName?` — evaluate `script`, trap any
/// completion code, and return it as an integer (0=ok … 4=continue).
fn catch_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 4 {
        return wrong_args(interp, b"catch script ?resultVarName? ?optionsVarName?");
    }
    let body = obj_bytes(argv[1]);
    let code = interp.eval_str(&body);
    // Snapshot the body's result BEFORE we overwrite the interp result with the
    // catch return value (the Zig "read before clear" discovery). `var_set`
    // retains it into the result var, so it survives the later `set_result`.
    let result = interp.get_obj_result();

    if let Some(&rv) = argv.get(2) {
        let name = obj_bytes(rv);
        if interp.var_set(&name, result).is_err() {
            return cant_set(interp, &name);
        }
    }
    if let Some(&ov) = argv.get(3) {
        let opts = build_options(interp, code); // rc 0
        let name = obj_bytes(ov);
        if interp.var_set(&name, opts).is_err() {
            drop_fresh(opts);
            return cant_set(interp, &name);
        }
    }
    // The error is now caught: publish the accumulated trace to the
    // `::errorInfo`/`::errorCode` globals (so a later `set ::errorInfo` reads it)
    // and reset the accumulator for the next error.
    if code == Code::Error {
        interp.publish_and_reset_error();
    }
    interp.set_result_bytes(code.as_int().to_string().as_bytes());
    Code::Ok
}

fn cant_set(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"can't set \"".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(b"\": variable is array");
    interp.set_error(&m)
}

/// Build `catch`'s `-options` dict: `-code N -level 0` (+ `-errorcode`/
/// `-errorinfo` from the live error accumulator on an error). Returns a fresh
/// (`rc 0`) dict.
fn build_options(interp: &mut Interp, code: Code) -> *mut TclObj {
    let code_str = code.as_int().to_string();
    let mut pairs: Vec<(*mut TclObj, *mut TclObj)> = vec![
        (new_string(b"-code"), new_string(code_str.as_bytes())),
        (new_string(b"-level"), new_string(b"0")),
    ];
    if code == Code::Error {
        // The full accumulated trace + code, not the (deferred) globals.
        pairs.push((new_string(b"-errorcode"), new_string(&interp.error_code())));
        pairs.push((new_string(b"-errorinfo"), new_string(&interp.error_info())));
    }
    dict::new_dict_obj(&pairs)
}

// -- error -----------------------------------------------------------------

/// `error message ?errorInfo? ?errorCode?` — raise an error. With an explicit
/// non-empty `errorInfo`, the trace is pre-seeded with it and the `error`
/// command itself is **not** re-logged (`ERR_ALREADY_LOGGED`); otherwise the
/// `while executing` / `invoked from within` trace accumulates as the error
/// unwinds. `errorCode` defaults to `NONE`. (`tclProc.c` `Tcl_ErrorObjCmd`.)
fn error_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 4 {
        return wrong_args(interp, b"error message ?errorInfo? ?errorCode?");
    }
    let ecode = if argv.len() == 4 {
        obj_bytes(argv[3])
    } else {
        b"NONE".to_vec()
    };
    // An empty explicit info is treated as absent (C: zero-length info arg).
    let info = if argv.len() >= 3 {
        obj_bytes(argv[2])
    } else {
        Vec::new()
    };
    let msg = obj_bytes(argv[1]);
    if info.is_empty() {
        interp.set_result(argv[1]);
        interp.set_error_state(&ecode)
    } else {
        interp.raise_with_info(&msg, &info, &ecode)
    }
}

// -- try / throw -----------------------------------------------------------

/// A `try` handler clause.
enum Handler {
    /// `on code varList script` — matches the body's completion code.
    On {
        code: Vec<u8>,
        vars: Vec<u8>,
        script: Vec<u8>,
    },
    /// `trap pattern varList script` — matches an error whose `-errorcode`
    /// has `pattern` (a list) as a leading sublist.
    Trap {
        pattern: Vec<u8>,
        vars: Vec<u8>,
        script: Vec<u8>,
    },
}

/// Map a `try`/`on` completion-code word (`ok`/`error`/`return`/`break`/
/// `continue` or an integer 0–4) to its numeric code.
fn code_word_to_int(spec: &[u8]) -> Option<i64> {
    match spec {
        b"ok" => Some(0),
        b"error" => Some(1),
        b"return" => Some(2),
        b"break" => Some(3),
        b"continue" => Some(4),
        _ => core::str::from_utf8(spec).ok()?.trim().parse::<i64>().ok(),
    }
}

/// Does `pattern` (a list) match `errorcode` (a list) as a leading sublist?
/// An empty pattern matches any error (`trap {} ...`).
fn errorcode_prefix_match(pattern: &[u8], errorcode: &[u8]) -> bool {
    let pat = match crate::parse::split_list(pattern) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let ec = crate::parse::split_list(errorcode).unwrap_or_default();
    pat.len() <= ec.len() && pat.iter().zip(ec.iter()).all(|(a, b)| a == b)
}

/// `try body ?handler ...? ?finally script?` — structured exception handling
/// (TIP 329). Handlers are `on code varList script` and `trap pattern varList
/// script`, tried in order; the first match runs and its completion becomes the
/// `try` result. `finally` always runs; only an error from it overrides the
/// result. Modelled on `tclCmdMZ.c` `Tcl_TryObjCmd`.
fn try_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    const USAGE: &[u8] = b"try body ?handler ...? ?finally script?";
    if argv.len() < 2 {
        return wrong_args(interp, USAGE);
    }
    let body = obj_bytes(argv[1]);

    let mut handlers: Vec<Handler> = Vec::new();
    let mut finally: Option<Vec<u8>> = None;
    let mut j = 2;
    while j < argv.len() {
        match obj_bytes(argv[j]).as_slice() {
            b"finally" => {
                if j + 2 != argv.len() {
                    return interp.set_error(
                        b"wrong # args to finally clause: must be \"... finally script\"",
                    );
                }
                finally = Some(obj_bytes(argv[j + 1]));
                j += 2;
            }
            b"on" if j + 4 <= argv.len() => {
                handlers.push(Handler::On {
                    code: obj_bytes(argv[j + 1]),
                    vars: obj_bytes(argv[j + 2]),
                    script: obj_bytes(argv[j + 3]),
                });
                j += 4;
            }
            b"trap" if j + 4 <= argv.len() => {
                handlers.push(Handler::Trap {
                    pattern: obj_bytes(argv[j + 1]),
                    vars: obj_bytes(argv[j + 2]),
                    script: obj_bytes(argv[j + 3]),
                });
                j += 4;
            }
            _ => {
                return interp.set_error(
                    b"bad handler clause: must be \"on code varList script\", \"trap pattern varList script\", or \"finally script\"",
                );
            }
        }
    }

    // Run the body, snapshotting its completion code, result, and -errorcode.
    let body_code = interp.eval_str(&body);
    let body_result = interp.result_bytes();
    let errorcode = if body_code == Code::Error {
        interp.error_code()
    } else {
        Vec::new()
    };

    // Locate the first matching handler.
    let mut outcome_code = body_code;
    let mut outcome_result = body_result.clone();
    for h in &handlers {
        let (matches, vars, script) = match h {
            Handler::On { code, vars, script } => (
                code_word_to_int(code) == Some(body_code.as_int()),
                vars,
                script,
            ),
            Handler::Trap {
                pattern,
                vars,
                script,
            } => (
                body_code == Code::Error && errorcode_prefix_match(pattern, &errorcode),
                vars,
                script,
            ),
        };
        if !matches {
            continue;
        }
        // Bind the handler's variables: [resultVar ?optionsVar?].
        let names = crate::parse::split_list(vars).unwrap_or_default();
        if let Some(rv) = names.first() {
            if !rv.is_empty() {
                let o = new_string(&body_result);
                if interp.var_set(rv, o).is_err() {
                    drop_fresh(o);
                    return cant_set(interp, rv);
                }
            }
        }
        if let Some(ov) = names.get(1) {
            let opts = build_options(interp, body_code);
            if interp.var_set(ov, opts).is_err() {
                drop_fresh(opts);
                return cant_set(interp, ov);
            }
        }
        // The body's error is now handled: publish + reset before the handler
        // runs (which starts its own error state if it throws).
        if body_code == Code::Error {
            interp.publish_and_reset_error();
        }
        outcome_code = interp.eval_str(script);
        outcome_result = interp.result_bytes();
        break;
    }

    // `finally` always runs; only its error overrides the result.
    if let Some(fin) = finally {
        let fc = interp.eval_str(&fin);
        if fc != Code::Ok {
            return fc; // finally's result is already the interp result
        }
    }

    interp.set_result_bytes(&outcome_result);
    outcome_code
}

/// `throw type message` — raise an error with `-errorcode type` (a non-empty
/// list). Equivalent to `return -code error -errorcode $type $message`.
fn throw_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"throw type message");
    }
    let ecode = obj_bytes(argv[1]);
    match crate::parse::split_list(&ecode) {
        Ok(parts) if !parts.is_empty() => {}
        _ => return interp.set_error(b"type must be non-empty list"),
    }
    interp.set_result(argv[2]);
    // Like `return -code error -errorcode $type $msg`: set the code, let the
    // `while executing`/`invoked from within` trace accumulate as it unwinds.
    interp.set_error_state(&ecode)
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
    fn catch_success_and_error_codes() {
        leak_free(|i| {
            // success → code 0, result var = body result (no tower needed).
            assert_eq!(run(i, b"catch {set z 2} m"), b"0");
            assert_eq!(run(i, b"set m"), b"2");
            // error → code 1, result var = error message.
            assert_eq!(run(i, b"catch {error boom} m"), b"1");
            assert_eq!(run(i, b"set m"), b"boom");
            // a no-such-variable read is caught too.
            assert_eq!(run(i, b"catch {set nope} m"), b"1");
            assert_eq!(run(i, b"set m"), b"can't read \"nope\": no such variable");
            // break/continue propagate their codes through catch.
            assert_eq!(run(i, b"catch {break} m"), b"3");
            assert_eq!(run(i, b"catch {continue} m"), b"4");
            i.eval_str(b"unset m");
        });
    }

    #[test]
    fn error_stamps_globals() {
        leak_free(|i| {
            // A bare `error` with no info accumulates the source trace as it
            // unwinds (verified byte-for-byte vs tclsh 9.0).
            assert_eq!(run(i, b"catch {error oops}"), b"1");
            assert_eq!(run(i, b"set ::errorCode"), b"NONE");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"oops\n    while executing\n\"error oops\""
            );
            // explicit info + code: the info is the trace verbatim (the `error`
            // command itself is not re-logged — ERR_ALREADY_LOGGED).
            assert_eq!(run(i, b"catch {error msg myinfo MYCODE}"), b"1");
            assert_eq!(run(i, b"set ::errorInfo"), b"myinfo");
            assert_eq!(run(i, b"set ::errorCode"), b"MYCODE");
            i.eval_str(b"unset ::errorInfo ::errorCode");
        });
    }

    /// The incremental `::errorInfo` stack trace (`while executing` / `invoked
    /// from within` / `(procedure "x" line N)` …), every expected string
    /// captured from real tclsh 9.0. See `proc-call-and-stack-traces.md` PC-4.
    #[test]
    fn error_info_stack_traces() {
        leak_free(|i| {
            // The worked example: proc body error → proc frame → call frame.
            run(i, b"proc p {} { error foo }");
            assert_eq!(i.eval_str(b"p"), Code::Error);
            assert_eq!(
                i.var_get(b"::errorInfo").map(crate::interp::obj_bytes),
                Some(
                    b"foo\n    while executing\n\"error foo \"\n    (procedure \"p\" line 1)\n    invoked from within\n\"p\""
                        .to_vec()
                )
            );
            // Multi-line body: the proc frame cites the body-relative line (3).
            run(i, b"proc q {} {\n    set x 1\n    error boom\n}");
            i.eval_str(b"catch q");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"boom\n    while executing\n\"error boom\"\n    (procedure \"q\" line 3)\n    invoked from within\n\"q\""
            );
            // Nested command substitution: the `[inner]` subst is logged, the
            // enclosing `set y [inner]` is suppressed.
            run(i, b"proc inner {} { error deep }");
            run(i, b"proc outer {} { set y [inner] }");
            i.eval_str(b"catch outer");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"deep\n    while executing\n\"error deep \"\n    (procedure \"inner\" line 1)\n    invoked from within\n\"inner\"\n    (procedure \"outer\" line 1)\n    invoked from within\n\"outer\""
            );
            // apply → a `(lambda term "..." line N)` frame.
            i.eval_str(b"catch { apply {{} { error fromLambda }} }");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"fromLambda\n    while executing\n\"error fromLambda \"\n    (lambda term \"{} { error fromLambda }\" line 1)\n    invoked from within\n\"apply {{} { error fromLambda }} \""
            );
            i.eval_str(b"unset -nocomplain ::errorInfo ::errorCode x y");
        });
    }

    /// `eval`/`uplevel`/`foreach` add a `("<cmd>" body line N)` frame; inline
    /// `if`/`while`/`for`/`switch` do not (tclsh 9.0).
    #[test]
    fn body_frame_commands() {
        leak_free(|i| {
            i.eval_str(b"catch { eval { error e } }");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"e\n    while executing\n\"error e \"\n    (\"eval\" body line 1)\n    invoked from within\n\"eval { error e } \""
            );
            i.eval_str(b"catch { foreach z {1} { error f } }");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"f\n    while executing\n\"error f \"\n    (\"foreach\" body line 1)\n    invoked from within\n\"foreach z {1} { error f } \""
            );
            // inline `if`: only the body command, no `if` frame.
            i.eval_str(b"catch { if {1} { error x } }");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"x\n    while executing\n\"error x \""
            );
            // `foreach` inside a proc is inlined (no foreach frame) — only the
            // proc frame; the top-level foreach above did show a frame.
            run(i, b"proc fe {} { foreach x {1} { error e } }");
            i.eval_str(b"catch fe");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"e\n    while executing\n\"error e \"\n    (procedure \"fe\" line 1)\n    invoked from within\n\"fe\""
            );
            i.eval_str(b"unset -nocomplain ::errorInfo ::errorCode");
        });
    }

    #[test]
    fn try_on_and_trap_and_finally() {
        leak_free(|i| {
            // on ok: the body succeeded; handler result becomes try's result.
            assert_eq!(run(i, b"try {set x 7} on ok {} {set y done}"), b"done");
            // on error msg: binds the result, runs the handler.
            assert_eq!(run(i, b"try {error boom} on error msg {set msg}"), b"boom");
            // trap: matches on the -errorcode leading sublist.
            assert_eq!(
                run(
                    i,
                    b"try {throw {POSIX EACCES} denied} trap {POSIX EACCES} {} {set r trapped}"
                ),
                b"trapped"
            );
            // no handler matches → the body's error propagates.
            assert_eq!(
                i.eval_str(b"try {error nope} on break {} {set r x}"),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"nope");
            // finally always runs; an OK finally doesn't change the result.
            assert_eq!(
                run(
                    i,
                    b"set fin 0; set v [try {set z 1} finally {set fin 1}]; list $v $fin"
                ),
                b"1 1"
            );
            i.eval_str(b"unset -nocomplain x msg r fin v z ::errorInfo ::errorCode");
        });
    }

    #[test]
    fn throw_sets_errorcode() {
        leak_free(|i| {
            assert_eq!(run(i, b"catch {throw {MY CODE} boom} m o"), b"1");
            assert_eq!(run(i, b"set m"), b"boom");
            assert_eq!(run(i, b"dict get $o -errorcode"), b"MY CODE");
            // an empty type is rejected.
            assert_eq!(i.eval_str(b"throw {} msg"), Code::Error);
            assert_eq!(i.result_bytes(), b"type must be non-empty list");
            i.eval_str(b"unset -nocomplain m o ::errorInfo ::errorCode");
        });
    }

    #[test]
    fn catch_options_dict() {
        leak_free(|i| {
            run(i, b"catch {error boom mycode MYERR} m o");
            // -code/-level always present; -errorcode/-errorinfo on error.
            assert_eq!(run(i, b"dict get $o -code"), b"1");
            assert_eq!(run(i, b"dict get $o -level"), b"0");
            assert_eq!(run(i, b"dict get $o -errorcode"), b"MYERR");
            assert_eq!(run(i, b"dict get $o -errorinfo"), b"mycode");
            // success path: -code 0.
            run(i, b"catch {set x 5} m o");
            assert_eq!(run(i, b"dict get $o -code"), b"0");
            i.eval_str(b"unset m o x ::errorInfo ::errorCode");
        });
    }
}
