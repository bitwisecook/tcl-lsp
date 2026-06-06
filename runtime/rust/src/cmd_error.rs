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

/// Register `catch` and `error`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"catch", catch_cmd);
    interp.register_builtin(b"error", error_cmd);
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

/// Set a global variable (`::name`) to `bytes` (for `::errorInfo`/`::errorCode`).
fn set_global(interp: &mut Interp, name: &[u8], bytes: &[u8]) {
    let o = new_string(bytes);
    if interp.var_set(name, o).is_err() {
        drop_fresh(o);
    }
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
/// `-errorinfo` from the globals on an error). Returns a fresh (`rc 0`) dict.
fn build_options(interp: &mut Interp, code: Code) -> *mut TclObj {
    let code_str = code.as_int().to_string();
    let mut pairs: Vec<(*mut TclObj, *mut TclObj)> = vec![
        (new_string(b"-code"), new_string(code_str.as_bytes())),
        (new_string(b"-level"), new_string(b"0")),
    ];
    if code == Code::Error {
        if let Some(ec) = interp.var_get(b"::errorCode") {
            pairs.push((new_string(b"-errorcode"), ec));
        }
        if let Some(ei) = interp.var_get(b"::errorInfo") {
            pairs.push((new_string(b"-errorinfo"), ei));
        }
    }
    dict::new_dict_obj(&pairs)
}

// -- error -----------------------------------------------------------------

/// `error message ?errorInfo? ?errorCode?` — raise an error, stamping
/// `::errorInfo` (explicit info, else the message) and `::errorCode` (else
/// `NONE`).
fn error_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 4 {
        return wrong_args(interp, b"error message ?errorInfo? ?errorCode?");
    }
    let info = if argv.len() >= 3 {
        obj_bytes(argv[2])
    } else {
        obj_bytes(argv[1])
    };
    let ecode = if argv.len() == 4 {
        obj_bytes(argv[3])
    } else {
        b"NONE".to_vec()
    };
    set_global(interp, b"::errorInfo", &info);
    set_global(interp, b"::errorCode", &ecode);
    // The result is the message.
    interp.set_result(argv[1]);
    Code::Error
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
            assert_eq!(run(i, b"catch {error oops}"), b"1");
            assert_eq!(run(i, b"set ::errorCode"), b"NONE");
            assert_eq!(run(i, b"set ::errorInfo"), b"oops");
            // explicit info + code.
            assert_eq!(run(i, b"catch {error msg myinfo MYCODE}"), b"1");
            assert_eq!(run(i, b"set ::errorInfo"), b"myinfo");
            assert_eq!(run(i, b"set ::errorCode"), b"MYCODE");
            i.eval_str(b"unset ::errorInfo ::errorCode");
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
