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

//! `try` / `throw` — structured exception handling (TIP 329).
//!
//! The VM compiles `try` to a runtime CALL (the bytecode backend has no
//! exception-range/`beginCatch` support), so the whole construct runs here.
//! Modelled on C Tcl 9's `Tcl_TryObjCmd`/`Tcl_ThrowObjCmd` (`tclCmdMZ.c`) and
//! `runtime/rust`'s `cmd_error.rs`: handlers (`on code varList script`,
//! `trap pattern varList script`) are tried in order, the first match runs and
//! its completion becomes the `try` result; a `-` body falls through to the next
//! clause; `finally` always runs and only its own exception overrides the
//! result. Semantics pinned against tclsh 9.0.

use tcl_runtime_api::{Code, Completion};

use crate::command::{completion_options, opt_get, options_dict};
use crate::interp::{Vm, err};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("try", cmd_try);
    vm.register("throw", cmd_throw);
}

/// A parsed `try` handler clause.
struct Handler {
    /// The completion code an `on` clause matches (a `trap` is always `1`).
    code: i64,
    /// `true` for a `trap` clause (match an error by `-errorcode` prefix).
    is_trap: bool,
    /// The `trap` errorcode prefix (a list value); unused for `on`.
    pattern: Value,
    /// The `[resultVar ?optionsVar?]` bind list.
    vars: Value,
    /// The handler body, or `-` (see `is_dash`).
    script: Value,
    /// Whether the body is the fall-through marker `-`.
    is_dash: bool,
}

/// Evaluate a `try` body/handler/finally script, mapping a hard error to an
/// error completion (the same shape `catch` uses).
fn eval_body(vm: &mut Vm, body: &Value) -> Completion<Value> {
    match vm.eval_source(&body.to_str()) {
        Ok(c) => c,
        Err(e) => Completion::new(Code::Error, e.into_value(), Value::empty()),
    }
}

/// Map a `try`/`on` completion-code word (`ok`/`error`/`return`/`break`/
/// `continue`, or an integer that fits a C `int`) to its numeric code.
fn code_word_to_int(spec: &str) -> Option<i64> {
    match spec {
        "ok" => Some(0),
        "error" => Some(1),
        "return" => Some(2),
        "break" => Some(3),
        "continue" => Some(4),
        // A completion code accepts a signed integer that fits a C `int`; a value
        // outside the `i32` range is *not* a valid code (error-20.2).
        _ => Value::string(spec)
            .as_int()
            .ok()
            .and_then(|n| i32::try_from(n).ok())
            .map(i64::from),
    }
}

/// Append `-during prior` to an error's options dict (TIP 329 exception
/// chaining): the superseded exception's options ride along on the new one.
/// Replaces an existing `-during` (a handler over a handler) rather than dup it.
fn add_during(options: &Value, prior: &Value) -> Value {
    let mut out: Vec<Value> = options.as_list().map(|l| (*l).clone()).unwrap_or_default();
    let mut i = 0;
    while i + 1 < out.len() {
        if &*out[i].to_str() == "-during" {
            out[i + 1] = prior.clone();
            return Value::list(out);
        }
        i += 2;
    }
    out.push(Value::string("-during"));
    out.push(prior.clone());
    Value::list(out)
}

/// `bad completion code "X": must be ok, error, return, break, continue, or an
/// integer` — an unrecognised `on` code word.
fn bad_completion_code(word: &str) -> String {
    format!(
        "bad completion code \"{word}\": must be ok, error, return, break, \
         continue, or an integer"
    )
}

/// Does `pattern` (a list) match `errorcode` (a list) as a leading sublist?
/// An empty pattern matches any error (`trap {} ...`).
fn errorcode_prefix_match(pattern: &Value, errorcode: &Value) -> bool {
    let Ok(pat) = pattern.as_list() else {
        return false;
    };
    let ec = errorcode.as_list().unwrap_or_default();
    pat.len() <= ec.len()
        && pat
            .iter()
            .zip(ec.iter())
            .all(|(a, b)| a.to_str() == b.to_str())
}

/// Bind a handler's `[resultVar ?optionsVar?]` variables. A failed set becomes
/// the handler outcome (and skips the body), matching C's `handlerFailed`.
fn bind_handler_vars(
    vm: &mut Vm,
    vars: &Value,
    result: &Value,
    opts: &Value,
) -> Result<(), Completion<Value>> {
    let names = vars.as_list().unwrap_or_default();
    // `var_set`, not `set_var`: a handler variable written as an array element
    // (`on error {x(y)}`) must resolve `x(y)` to the element — and fail if the
    // base `x` is a scalar (`can't set "x(y)": variable isn't array`), which C's
    // `handlerFailed` turns into the handler outcome, skipping the body.
    if let Some(rv) = names.first() {
        let n = rv.to_str();
        if !n.is_empty()
            && let Err(e) = vm.var_set(&n, result.clone())
        {
            return Err(e);
        }
    }
    if let Some(ov) = names.get(1) {
        vm.var_set(&ov.to_str(), opts.clone())?;
    }
    Ok(())
}

/// Parse a `try`'s handler (`on`/`trap`) and `finally` clauses, validating the
/// grammar (a bad clause errors before the body runs). Returns the handlers and
/// the optional `finally` script.
fn parse_clauses(rest: &[Value]) -> Result<(Vec<Handler>, Option<Value>), Completion<Value>> {
    let mut handlers: Vec<Handler> = Vec::new();
    let mut finally: Option<Value> = None;
    let mut j = 0;
    while j < rest.len() {
        match &*rest[j].to_str() {
            "finally" => {
                if j + 2 < rest.len() {
                    return Err(err("finally clause must be last"));
                }
                if j + 1 >= rest.len() {
                    return Err(err(
                        "wrong # args to finally clause: must be \"... finally script\"",
                    ));
                }
                finally = Some(rest[j + 1].clone());
                j += 2;
            }
            kind @ ("on" | "trap") => {
                if j + 4 > rest.len() {
                    return Err(err(format!(
                        "wrong # args to {kind} clause: must be \"... {kind} {} variableList script\"",
                        if kind == "on" { "code" } else { "pattern" }
                    )));
                }
                let is_trap = kind == "trap";
                let code = if is_trap {
                    if rest[j + 1].as_list().is_err() {
                        return Err(err(format!(
                            "bad prefix '{}': must be a list",
                            rest[j + 1].to_str()
                        )));
                    }
                    1
                } else {
                    match code_word_to_int(&rest[j + 1].to_str()) {
                        Some(c) => c,
                        None => return Err(err(bad_completion_code(&rest[j + 1].to_str()))),
                    }
                };
                let script = rest[j + 3].clone();
                let is_dash = &*script.to_str() == "-";
                handlers.push(Handler {
                    code,
                    is_trap,
                    pattern: rest[j + 1].clone(),
                    vars: rest[j + 2].clone(),
                    script,
                    is_dash,
                });
                j += 4;
            }
            other => {
                return Err(err(format!(
                    "bad handler type \"{other}\": must be finally, on, or trap"
                )));
            }
        }
    }
    if handlers.last().is_some_and(|h| h.is_dash) {
        return Err(err("last non-finally clause must not have a body of \"-\""));
    }
    Ok((handlers, finally))
}

/// `try body ?handler ...? ?finally script?` — structured exception handling.
fn cmd_try(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    const USAGE: &str = "wrong # args: should be \"try body ?handler ...? ?finally script?\"";
    let Some((body, rest)) = args.split_first() else {
        return err(USAGE);
    };
    let (handlers, finally) = match parse_clauses(rest) {
        Ok(parsed) => parsed,
        Err(e) => return e,
    };

    // run the body, then dispatch to the first matching handler
    let body_comp = eval_body(vm, body);
    // `exit` is not catchable (C Tcl's `Tcl_Exit`): propagate the unwind
    // without running handlers or the `finally` clause.
    if vm.exit_pending() {
        return body_comp;
    }
    let errorcode = if body_comp.code == Code::Error {
        crate::command::resolved_error_code(&body_comp)
    } else {
        Value::empty()
    };

    let mut outcome = body_comp.clone();
    // The body's options dict (bound to a handler's optionsVar and reused as the
    // `-during` chain link if a handler/finally throws over the body's exception).
    let body_opts = completion_options(&body_comp);
    let matched = handlers.iter().position(|h| {
        if h.is_trap {
            body_comp.code == Code::Error && errorcode_prefix_match(&h.pattern, &errorcode)
        } else {
            h.code == body_comp.code.as_int()
        }
    });
    if let Some(m) = matched {
        // Scan forward over `-` fall-through bodies to the clause that runs.
        let mut b = m;
        while handlers[b].is_dash {
            b += 1; // guaranteed to terminate (the last body is not `-`)
        }
        match bind_handler_vars(vm, &handlers[b].vars, &body_comp.result, &body_opts) {
            Ok(()) => {
                // The body's exception is now handled: publish `errorInfo`/
                // `errorCode` (so the handler reads the body's error) and reset the
                // trace so the handler's own errors start fresh.
                if body_comp.code == Code::Error {
                    let einfo = vm.take_error_info().unwrap_or_else(|| {
                        opt_get(&body_opts, "-errorinfo").map_or_else(
                            || body_comp.result.to_str().to_string(),
                            |v| v.to_str().to_string(),
                        )
                    });
                    vm.publish_error(&einfo, &errorcode);
                }
                outcome = eval_body(vm, &handlers[b].script);
                // A handler that threw over the body's exception chains it.
                if outcome.code == Code::Error {
                    outcome.options = add_during(&completion_options(&outcome), &body_opts);
                }
            }
            // A failed var bind also chains to the body (C's `handlerFailed`).
            Err(mut e) => {
                if e.code == Code::Error {
                    e.options = add_during(&completion_options(&e), &body_opts);
                }
                outcome = e;
            }
        }
    }

    // finally always runs; only its own exception overrides
    if let Some(fin) = finally {
        // The options that would otherwise propagate, captured so a throwing
        // `finally` can chain them as its `-during`.
        let prior_opts = completion_options(&outcome);
        let mut fc = eval_body(vm, &fin);
        if fc.code != Code::Ok {
            if fc.code == Code::Error {
                fc.options = add_during(&completion_options(&fc), &prior_opts);
            }
            return fc;
        }
    }

    outcome
}

/// `throw type message` — raise an error with `-errorcode type` (a non-empty
/// list). Equivalent to `return -code error -errorcode $type $message`.
fn cmd_throw(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [ty, msg] = args else {
        return err("wrong # args: should be \"throw type message\"");
    };
    match ty.as_list() {
        Ok(parts) if !parts.is_empty() => {}
        Ok(_) => return err("type must be non-empty list"),
        Err(e) => return err(e.message),
    }
    // Like `return -code error -errorcode $type $msg`: the message is the result,
    // the `while executing`/`invoked from within` trace accumulates as it unwinds.
    let options = options_dict(
        Code::Error,
        0,
        &[
            ("-errorcode", ty.clone()),
            ("-errorinfo", Value::string(msg.to_str().to_string())),
        ],
    );
    let _ = vm;
    Completion::new(Code::Error, msg.clone(), options)
}
