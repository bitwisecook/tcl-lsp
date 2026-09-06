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

//! `catch` + `error` — the exception foundation (PC-4, toward running tcltest).
//!
//! Modelled on C Tcl 9 (`tclCmdAH.c` `Tcl_CatchObjCmd`, `tclProc.c`/`tclResult.c`
//! `Tcl_ErrorObjCmd`). `catch` snapshots the body's completion code
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
use crate::frame::VarError;
use crate::interp::{drop_fresh, new_string, obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};

/// Register `catch`, `error`, `try`, and `throw`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"catch", catch_cmd);
    interp.register_builtin(b"error", error_cmd);
    interp.register_builtin(b"try", try_cmd);
    interp.register_builtin(b"throw", throw_cmd);
}

// -- catch -----------------------------------------------------------------

/// `catch script ?resultVarName? ?optionsVarName?` — evaluate `script`, trap any
/// completion code, and return it as an integer (0=ok … 4=continue).
fn catch_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 4 {
        return interp.wrong_args(b"catch script ?resultVarName? ?optionVarName?");
    }
    // `catch` is bytecode-compiled inline (C's `TclCompileCatchCmd`): a literal
    // body runs in the **same** `info frame` level and the same `codePtr->source`
    // as the enclosing proc/script. `eval_control_body` reproduces that — sharing
    // the enclosing frame in a proc (so `info frame` depth and the body-relative
    // `errorLine` for `MakeProcError` stay correct), while a top-level or dynamic
    // body still evaluates as its own frame.
    let code = interp.eval_control_body(argv[1]);
    // An `exit` in the body is uncatchable (C's `Tcl_Exit`): re-propagate it
    // instead of turning it into a caught return code.
    if interp.exit_pending() {
        return code;
    }
    // Snapshot the body's result BEFORE we overwrite the interp result with the
    // catch return value (read the value before clearing the result). `var_set`
    // retains it into the result var, so it survives the later `set_result`.
    let result = interp.get_obj_result();

    if let Some(&rv) = argv.get(2) {
        let name = obj_bytes(rv);
        if let Err(e) = set_var_or_elem(interp, &name, result) {
            return crate::builtins::var_error(interp, &name, e);
        }
    }
    if let Some(&ov) = argv.get(3) {
        let opts = completion_options(interp, code); // rc 0
        let name = obj_bytes(ov);
        if let Err(e) = set_var_or_elem(interp, &name, opts) {
            drop_fresh(opts);
            return crate::builtins::var_error(interp, &name, e);
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

/// Write `obj` to `name`, routing `arr(a)` to the array *element* rather than
/// a literal scalar named `arr(a)` (issue #1577) — the same
/// `split_array_ref`/`var_set`/`var_set_elem` routing `set` uses, so
/// `catch`'s result/options vars and `try`'s handler vars don't hand-roll a
/// second name parser.
fn set_var_or_elem(interp: &mut Interp, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
    let (base, elem) = crate::frame::split_array_ref(name);
    match &elem {
        Some(k) => interp.var_set_elem(&base, k, obj),
        None => interp.var_set(&base, obj),
    }
}

/// Build a completion's return-options dict from the live interpreter state.
///
/// This is the one implementation used by `catch`, `try`, and the shared
/// [`tcl_runtime_api::Completion`] adapter. It returns a fresh (`rc 0`) dict
/// containing `-code` and `-level`, plus the live error state when applicable.
/// A caller that exports the dict across an ABI must take an owning reference
/// before returning it.
pub(crate) fn completion_options(interp: &mut Interp, code: Code) -> *mut TclObj {
    // A body that completed via `return` propagates the return's *own* requested
    // options (`-code C -level L`), not the settled `RETURN`(2)/level-0 — what
    // `catch`'s options dict and TIP 329 `-during` chaining record
    // (`Tcl_GetReturnOptions`, `tclResult.c`). Every other code is at level 0.
    let (eff_code, level) = if code == Code::Return {
        (interp.pending_return_code(), interp.pending_return_level())
    } else {
        (code, 0)
    };
    let code_str = eff_code.as_int().to_string();
    let level_str = level.to_string();
    let mut pairs: Vec<(*mut TclObj, *mut TclObj)> = vec![
        (new_string(b"-code"), new_string(code_str.as_bytes())),
        (new_string(b"-level"), new_string(level_str.as_bytes())),
    ];
    if eff_code == Code::Error {
        // `-errorcode` rides along with any error completion (incl. a pending
        // `return -code error`). The accumulated trace + stack and the `-during`
        // chain only exist once the error has actually been raised (level 0).
        pairs.push((new_string(b"-errorcode"), new_string(&interp.error_code())));
        if level == 0 {
            pairs.push((new_string(b"-errorinfo"), new_string(&interp.error_info())));
            // TIP 348: the error stack built as the error unwound.
            pairs.push((
                new_string(b"-errorstack"),
                new_string(&interp.error_stack_value()),
            ));
            // TIP 329 exception chaining: when a `try` handler/`finally` threw
            // over a prior exception, that prior exception's options ride along as
            // `-during` (`During()` in `tclCmdMZ.c`). `new_dict_obj` retains it.
            if let Some(during) = interp.during_opts() {
                pairs.push((new_string(b"-during"), during));
            }
        }
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
        return interp.wrong_args(b"error message ?errorInfo? ?errorCode?");
    }
    // An explicit `errorCode` arg is honoured verbatim — even when empty, it
    // reads back empty rather than the `NONE` default (error-4.5).
    let explicit_code = argv.len() == 4;
    let ecode = if explicit_code {
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
    let rc = if info.is_empty() {
        interp.set_result(argv[1]);
        interp.set_error_state(&ecode)
    } else {
        interp.raise_with_info(&msg, &info, &ecode)
    };
    if explicit_code {
        interp.mark_error_code_explicit();
    }
    rc
}

// -- try / throw -----------------------------------------------------------

/// A `try` handler clause. The handler `script` is kept as its argument object
/// (not flattened to bytes) so it evaluates through `eval_control_body`, which
/// A parsed `try` handler clause (`on code …` or `trap pattern …`). The
/// completion code / errorcode prefix are resolved at parse time so a bad code
/// word or trap prefix errors before the body runs; `is_dash` marks a `-`
/// fall-through body (the next non-`-` clause's body runs instead).
struct Handler {
    /// The completion code an `on` clause matches (a `trap` is always `1`).
    code: i64,
    /// `true` for a `trap` clause (matches an error by `-errorcode` prefix).
    is_trap: bool,
    /// The `trap` errorcode prefix (a list); empty for `on`.
    pattern: Vec<u8>,
    /// The `[resultVar ?optionsVar?]` bind list.
    vars: Vec<u8>,
    /// The handler body, or `-` (see `is_dash`).
    script: *mut TclObj,
    /// Whether the body is the fall-through marker `-`.
    is_dash: bool,
}

/// Map a `try`/`on` completion-code word (`ok`/`error`/`return`/`break`/
/// `continue` or an integer that fits a C `int`) to its numeric code.
fn code_word_to_int(spec: &[u8]) -> Option<i64> {
    match spec {
        b"ok" => Some(0),
        b"error" => Some(1),
        b"return" => Some(2),
        b"break" => Some(3),
        b"continue" => Some(4),
        // A completion code accepts the full signed/unsigned 32-bit range (C's
        // `Tcl_GetIntFromObj`), with `0x`/`0o`/`0b`/`0d` prefixes and a sign.
        _ => crate::interp::parse_completion_int(spec).map(i64::from),
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

/// `try`'s handler-type words, in C table order (`TryObjCmd`'s `handlerNames`,
/// `tclCmdMZ.c`): `Tcl_GetIndexFromObj(…, "handler type", 0)`, so `f`/`o`/`t`
/// abbreviate and the empty word — a prefix of all three — is
/// `ambiguous handler type ""`. The type is resolved before the clause's
/// arity, as it is in C (`try {} x` → `bad handler type "x"`).
const HANDLER_TYPES: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("handler type", &[b"finally", b"on", b"trap"]);

/// `try body ?handler ...? ?finally script?` — structured exception handling
/// (TIP 329). Handlers are `on code varList script` and `trap pattern varList
/// script`, tried in order; the first match runs and its completion becomes the
/// `try` result. `finally` always runs; only an error from it overrides the
/// result. Modelled on `tclCmdMZ.c` `Tcl_TryObjCmd`.
fn try_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    const USAGE: &[u8] = b"try body ?handler ...? ?finally script?";
    if argv.len() < 2 {
        return interp.wrong_args(USAGE);
    }
    let body = argv[1];

    let mut handlers: Vec<Handler> = Vec::new();
    let mut finally: Option<*mut TclObj> = None;
    let mut j = 2;
    while j < argv.len() {
        let handler_type = match HANDLER_TYPES.index_of(&obj_bytes(argv[j])) {
            Ok(i) => HANDLER_TYPES.names()[i],
            Err(m) => return interp.set_error(&m),
        };
        match handler_type {
            b"finally" => {
                if j < argv.len() - 2 {
                    return interp.set_error(b"finally clause must be last");
                }
                if j == argv.len() - 1 {
                    return interp.set_error(
                        b"wrong # args to finally clause: must be \"... finally script\"",
                    );
                }
                finally = Some(argv[j + 1]);
                j += 2;
            }
            b"on" => {
                if j + 4 > argv.len() {
                    return interp.set_error(
                        b"wrong # args to on clause: must be \"... on code variableList script\"",
                    );
                }
                let code = match code_word_to_int(&obj_bytes(argv[j + 1])) {
                    Some(c) => c,
                    None => return bad_completion_code(interp, &obj_bytes(argv[j + 1])),
                };
                let script = argv[j + 3];
                handlers.push(Handler {
                    code,
                    is_trap: false,
                    pattern: Vec::new(),
                    vars: obj_bytes(argv[j + 2]),
                    script,
                    is_dash: obj_bytes(script).as_slice() == b"-",
                });
                j += 4;
            }
            b"trap" => {
                if j + 4 > argv.len() {
                    return interp.set_error(
                        b"wrong # args to trap clause: must be \"... trap pattern variableList script\"",
                    );
                }
                let pattern = obj_bytes(argv[j + 1]);
                if crate::parse::split_list(&pattern).is_err() {
                    let mut m = b"bad prefix '".to_vec();
                    m.extend_from_slice(&pattern);
                    m.extend_from_slice(b"': must be a list");
                    return interp.set_error(&m);
                }
                let script = argv[j + 3];
                handlers.push(Handler {
                    code: 1,
                    is_trap: true,
                    pattern,
                    vars: obj_bytes(argv[j + 2]),
                    script,
                    is_dash: obj_bytes(script).as_slice() == b"-",
                });
                j += 4;
            }
            // Unreachable: `HANDLER_TYPES` has exactly the three arms above.
            other => {
                let mut m = b"bad handler type \"".to_vec();
                m.extend_from_slice(other);
                m.extend_from_slice(b"\": must be ");
                m.extend_from_slice(&tcl_cmd_core::prefix::choice_list_bytes(
                    HANDLER_TYPES.names(),
                ));
                return interp.set_error(&m);
            }
        }
    }
    // The last non-finally clause may not be a `-` fall-through (nothing follows).
    if handlers.last().is_some_and(|h| h.is_dash) {
        return interp.set_error(b"last non-finally clause must not have a body of \"-\"");
    }

    // Run the body, snapshotting its completion code, result, and -errorcode.
    // `eval_control_body` recovers the body literal's TIP 280 source location so
    // an `info frame` inside reports the right `type source` line.
    let body_code = interp.eval_control_body(body);
    let body_result = interp.result_bytes();
    let errorcode = if body_code == Code::Error {
        interp.error_code()
    } else {
        Vec::new()
    };

    // Locate the first matching handler, then scan forward over `-` bodies to the
    // clause whose body actually runs (binding *that* clause's variables).
    let mut outcome_code = body_code;
    let mut outcome_result = body_result.clone();
    let matched = handlers.iter().position(|h| {
        if h.is_trap {
            body_code == Code::Error && errorcode_prefix_match(&h.pattern, &errorcode)
        } else {
            h.code == body_code.as_int()
        }
    });
    if let Some(m) = matched {
        let mut b = m;
        while handlers[b].is_dash {
            b += 1; // guaranteed to terminate (the last body is not `-`)
        }
        // Build the body's options dict from the *live* body error/return state
        // (before it is published+reset). It is bound to the handler's optionsVar
        // and reused as the `-during` chain link if the handler itself throws
        // (TIP 329 exception chaining). Retained for the duration of the handler.
        let body_opts = completion_options(interp, body_code);
        // SAFETY: keep `body_opts` alive across the handler eval / var binding.
        unsafe { obj::incr_ref_count(body_opts) };
        // Bind the running clause's variables: [resultVar ?optionsVar?]. A failed
        // bind becomes the outcome (and skips the handler body), but `finally`
        // still runs; the bind error chains to the body via `-during` (C's
        // `handlerFailed`).
        match bind_handler_vars(interp, &handlers[b].vars, &body_result, body_opts) {
            Ok(()) => {
                // The body's exception is now handled: publish + reset so the
                // handler starts with clean error state.
                if body_code == Code::Error {
                    interp.publish_and_reset_error();
                }
                outcome_code = interp.eval_control_body(handlers[b].script);
                outcome_result = interp.result_bytes();
                if outcome_code == Code::Error {
                    // The handler threw over the body's exception: chain it.
                    interp.set_during(body_opts);
                }
            }
            Err(()) => {
                outcome_code = Code::Error;
                outcome_result = interp.result_bytes();
                interp.set_during(body_opts);
            }
        }
        // SAFETY: release our hold; `set_during`/the optionsVar keep their own.
        unsafe { obj::decr_ref_count(body_opts) };
    }

    // `finally` always runs; only an exception from it overrides the result.
    if let Some(fin) = finally {
        // Capture the options that would propagate from the body/handler stage
        // (carrying any `-during` already chained) in case `finally` throws and
        // must chain them in turn.
        let prior_opts = completion_options(interp, outcome_code);
        // SAFETY: keep `prior_opts` alive across the finally eval.
        unsafe { obj::incr_ref_count(prior_opts) };
        let fc = interp.eval_control_body(fin);
        if fc != Code::Ok {
            if fc == Code::Error {
                interp.set_during(prior_opts); // chain the superseded exception
            } else {
                interp.clear_during(); // a non-error finally exception does not chain
            }
            // SAFETY: release our hold (`set_during` kept its own if it ran).
            unsafe { obj::decr_ref_count(prior_opts) };
            return fc; // finally's result is already the interp result
        }
        // SAFETY: finally completed OK — discard the speculative capture.
        unsafe { obj::decr_ref_count(prior_opts) };
    }

    interp.set_result_bytes(&outcome_result);
    outcome_code
}

/// `bad completion code "X": must be ok, error, return, break, continue, or an
/// integer` — an unrecognised `on` code word.
fn bad_completion_code(interp: &mut Interp, word: &[u8]) -> Code {
    let mut m = b"bad completion code \"".to_vec();
    m.extend_from_slice(word);
    m.extend_from_slice(b"\": must be ok, error, return, break, continue, or an integer");
    interp.set_error(&m)
}

/// Bind a `try` handler clause's `[resultVar ?optionsVar?]` to the body's result
/// and the prebuilt option dict (`body_opts`, retained into the variable).
/// `Err(())` if a variable can't be set (the interp error is left in place); the
/// result variable is set before the options variable, so a later failure still
/// leaves the result variable assigned (error-19.11).
fn bind_handler_vars(
    interp: &mut Interp,
    vars: &[u8],
    body_result: &[u8],
    body_opts: *mut TclObj,
) -> Result<(), ()> {
    let names = crate::parse::split_list(vars).unwrap_or_default();
    if let Some(rv) = names.first() {
        if !rv.is_empty() {
            let o = new_string(body_result);
            if let Err(e) = set_var_or_elem(interp, rv, o) {
                drop_fresh(o);
                crate::builtins::var_error(interp, rv, e);
                return Err(());
            }
        }
    }
    if let Some(ov) = names.get(1) {
        if let Err(e) = set_var_or_elem(interp, ov, body_opts) {
            crate::builtins::var_error(interp, ov, e);
            return Err(());
        }
    }
    Ok(())
}

/// `throw type message` — raise an error with `-errorcode type` (a non-empty
/// list). Equivalent to `return -code error -errorcode $type $message`.
fn throw_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"throw type message");
    }
    let ecode = obj_bytes(argv[1]);
    match crate::parse::split_list(&ecode) {
        Ok(parts) if !parts.is_empty() => {}
        // A malformed type list reports the list parse error verbatim
        // (error-8.8/8.11); a well-formed but empty list is the type error.
        Ok(_) => return interp.set_error(b"type must be non-empty list"),
        Err(e) => return interp.set_error(&crate::parse::list_error_message(&ecode, e)),
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

    /// Issue #1607: `try`'s handler-type word is a `Tcl_GetIndexFromObj(…,
    /// "handler type", 0)` table, so the three types abbreviate and the empty
    /// word — a prefix of all three — is `ambiguous handler type ""`.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   try {} o error {} {}  -> {}   ;  try {} f {} -> {}  ;  try {} t {} {} {} -> {}
    ///   try {} {} error {} {} -> ambiguous handler type "": must be finally, on, or trap
    ///   try {} x              -> bad handler type "x": … (the type precedes the arity)
    #[test]
    fn try_handler_type_resolves_like_tcl_get_index_from_obj() {
        const MUST: &str = "must be finally, on, or trap";
        leak_free(|i| {
            // Unique prefixes resolve to their clause grammar.
            assert_eq!(run(i, b"try {set x ok} o error {} {set x h}"), b"ok");
            assert_eq!(run(i, b"try {set x ok} f {set y 1}"), b"ok");
            assert_eq!(run(i, b"try {set x ok} t {} {} {set x h}"), b"ok");
            // The empty word prefixes all three ⇒ ambiguous.
            assert_eq!(i.eval_str(b"try good {} error {} {x}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("ambiguous handler type \"\": {MUST}").as_bytes()
            );
            // The type is resolved before the clause's arity.
            assert_eq!(i.eval_str(b"try good x"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("bad handler type \"x\": {MUST}").as_bytes()
            );
        });
    }

    #[test]
    fn exit_records_code_and_is_uncatchable() {
        leak_free(|i| {
            // `exit N` records the code and unwinds (never terminates the host).
            assert_eq!(i.eval_str(b"exit 7"), Code::Error);
            assert_eq!(i.take_exit(), Some(7));
            // Default code is 0.
            assert_eq!(i.eval_str(b"exit"), Code::Error);
            assert_eq!(i.take_exit(), Some(0));
            // Uncatchable: `catch {exit}` re-propagates, so the trailing command
            // never runs.
            assert_eq!(i.eval_str(b"catch {exit 5}; set marker ran"), Code::Error);
            assert_eq!(i.take_exit(), Some(5));
            assert_eq!(run(i, b"info exists marker"), b"0");
            // A non-integer code is the standard error; no exit is pending.
            assert_eq!(i.eval_str(b"exit foo"), Code::Error);
            assert_eq!(i.take_exit(), None);
            assert_eq!(i.result_bytes(), b"expected integer but got \"foo\"");
            // Arity.
            assert_eq!(i.eval_str(b"exit a b"), Code::Error);
            assert_eq!(i.take_exit(), None);
        });
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
    // Needs the numeric tower: the inline-`if` frame case dispatches `if`.
    #[cfg(have_tommath)]
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
