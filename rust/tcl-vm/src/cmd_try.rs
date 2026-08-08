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
//!
//! Body/handler/finally each run as a phase of an explicit-stack state machine
//! (`TryState`/`TryPhase`/[`advance_try`]) rather than through
//! `Vm::eval_source`'s nested drive, so a `yield` inside any of them stays
//! yieldable (issue #1311) — the phase transitions (handler matching, var
//! binding, `-during` chaining) are the same Rust-side logic the old
//! synchronous version had, just resumed from `Vm::unwind` instead of run
//! inline between two `eval_source` calls.

use std::rc::Rc;

use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::{Code, Completion};

use crate::command::{completion_options, opt_get, options_dict};
use crate::interp::{Vm, err, ok};
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

/// A `try`'s handlers + `finally` script, parsed once up front and shared
/// (via `Rc`) across every phase of the state machine — nothing here changes
/// once `cmd_try` has validated the grammar.
struct TryPlan {
    handlers: Vec<Handler>,
    finally: Option<Value>,
}

/// Which phase of a `try` an explicit-stack activation is running, and the
/// state carried into the *next* phase once this one completes (see
/// [`advance_try`]).
pub(crate) enum TryPhase {
    /// Running the body.
    Body,
    /// Running the matched handler's script. `body_opts` is the body's options
    /// dict, kept for `-during` chaining if the handler itself throws.
    Handler { body_opts: Value },
    /// Running `finally`. `outcome` is the completion `finally` will restore
    /// on success (the body's or handler's) — captured before `finally` ran, so
    /// its own `-during` chaining (if `finally` throws) sees the right prior
    /// options.
    Finally { outcome: Completion<Value> },
}

/// A `try`'s live state, carried on the currently-running phase's activation
/// ([`Frame::try_ctx`](crate::exec::Frame)) until [`advance_try`] either moves
/// it to the next phase or delivers a final completion.
pub(crate) struct TryState {
    plan: Rc<TryPlan>,
    phase: TryPhase,
}

/// One phase of a `try` deferred to the explicit stack: the phase's compiled
/// script plus the state to resume from once it completes. Parked in
/// `Vm.pending_try` by `cmd_try`/[`advance_try`], drained into a try activation
/// (see [`Frame::new_try`](crate::exec::Frame::new_try)).
pub(crate) struct TryReq {
    pub(crate) asm: Rc<FunctionAsm>,
    pub(crate) state: TryState,
}

/// The outcome of folding a completed try-phase activation (see
/// [`advance_try`]): either move on to the next phase (push a new try
/// activation for it) or the whole `try` is done.
pub(crate) enum TryOutcome {
    Push(TryReq),
    Deliver(Completion<Value>),
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
///
/// Parses and validates the grammar synchronously (unchanged), then defers the
/// body to the explicit stack via `vm.pending_try` (issue #1311) instead of
/// running it through `Vm::eval_source`. [`advance_try`] carries the
/// handler-matching / `finally` logic forward from there, one phase per
/// `Vm::unwind` fold.
fn cmd_try(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    const USAGE: &str = "wrong # args: should be \"try body ?handler ...? ?finally script?\"";
    let Some((body, rest)) = args.split_first() else {
        return err(USAGE);
    };
    let (handlers, finally) = match parse_clauses(rest) {
        Ok(parsed) => parsed,
        Err(e) => return e,
    };
    let plan = Rc::new(TryPlan { handlers, finally });
    match vm.compile_source_cached(&body.to_str()) {
        Ok(asm) => {
            vm.pending_try = Some(TryReq {
                asm,
                state: TryState {
                    plan,
                    phase: TryPhase::Body,
                },
            });
            ok(Value::empty())
        }
        // A body parse error is a regular runtime error, subject to the same
        // `on error`/`trap`/`finally` handling as any other body error —
        // mirrors the old `Vm::eval_source`-based version, whose `Err
        // (TclError)` became a plain `Completion{Error}` fed through
        // `advance_after_body`'s handler matching, not returned as a hard
        // failure that skips it (try-body-parse-error tclsh-pinned test).
        Err(e) => match advance_after_body(vm, &plan, err(e.message)) {
            TryOutcome::Push(req) => {
                vm.pending_try = Some(req);
                ok(Value::empty())
            }
            TryOutcome::Deliver(c) => c,
        },
    }
}

/// Advance a `try`'s state machine once its current phase's activation
/// completes with `c`: either the next phase to push (a matched handler, or
/// `finally`), or the whole construct's final completion. Called from
/// `Vm::unwind` when a `try_ctx`-tagged frame completes.
pub(crate) fn advance_try(vm: &mut Vm, state: TryState, c: Completion<Value>) -> TryOutcome {
    let TryState { plan, phase } = state;
    match phase {
        TryPhase::Body => advance_after_body(vm, &plan, c),
        TryPhase::Handler { body_opts } => advance_after_handler(vm, &plan, &body_opts, c),
        TryPhase::Finally { outcome } => advance_after_finally(outcome, c),
    }
}

/// The body just completed as `body_comp`: match a handler (if any), bind its
/// variables, and either push it as the next phase or fall through to
/// [`finish_body_or_handler`].
fn advance_after_body(vm: &mut Vm, plan: &Rc<TryPlan>, body_comp: Completion<Value>) -> TryOutcome {
    // `exit` is not catchable (C Tcl's `Tcl_Exit`): propagate the unwind
    // without running handlers or the `finally` clause.
    if vm.exit_pending() {
        return TryOutcome::Deliver(body_comp);
    }
    let errorcode = if body_comp.code == Code::Error {
        crate::command::resolved_error_code(&body_comp)
    } else {
        Value::empty()
    };
    // The body's options dict (bound to a handler's optionsVar and reused as the
    // `-during` chain link if a handler/finally throws over the body's exception).
    let body_opts = completion_options(&body_comp);
    let matched = plan.handlers.iter().position(|h| {
        if h.is_trap {
            body_comp.code == Code::Error && errorcode_prefix_match(&h.pattern, &errorcode)
        } else {
            h.code == body_comp.code.as_int()
        }
    });
    let Some(m) = matched else {
        return finish_body_or_handler(vm, plan, body_comp);
    };
    // Scan forward over `-` fall-through bodies to the clause that runs.
    let mut b = m;
    while plan.handlers[b].is_dash {
        b += 1; // guaranteed to terminate (the last body is not `-`)
    }
    match bind_handler_vars(vm, &plan.handlers[b].vars, &body_comp.result, &body_opts) {
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
            match vm.compile_source_cached(&plan.handlers[b].script.to_str()) {
                Ok(asm) => TryOutcome::Push(TryReq {
                    asm,
                    state: TryState {
                        plan: Rc::clone(plan),
                        phase: TryPhase::Handler { body_opts },
                    },
                }),
                Err(e) => finish_body_or_handler(vm, plan, err(e.message)),
            }
        }
        // A failed var bind also chains to the body (C's `handlerFailed`).
        Err(mut e) => {
            if e.code == Code::Error {
                e.options = add_during(&completion_options(&e), &body_opts);
            }
            finish_body_or_handler(vm, plan, e)
        }
    }
}

/// The matched handler just completed as `handler_comp`: it becomes the
/// outcome (chaining `-during` if it threw over the body's exception), then
/// falls through to [`finish_body_or_handler`].
fn advance_after_handler(
    vm: &mut Vm,
    plan: &Rc<TryPlan>,
    body_opts: &Value,
    handler_comp: Completion<Value>,
) -> TryOutcome {
    let mut outcome = handler_comp;
    if outcome.code == Code::Error {
        outcome.options = add_during(&completion_options(&outcome), body_opts);
    }
    finish_body_or_handler(vm, plan, outcome)
}

/// After the body (no handler matched, or a matched handler's var bind
/// failed) or after the matched handler ran: `finally` always runs next if
/// present, else `outcome` is the whole `try`'s final completion.
fn finish_body_or_handler(
    vm: &mut Vm,
    plan: &Rc<TryPlan>,
    outcome: Completion<Value>,
) -> TryOutcome {
    let Some(fin) = plan.finally.clone() else {
        return TryOutcome::Deliver(outcome);
    };
    // Compiled lazily here rather than in `cmd_try` up front: a `finally`
    // never runs before this point, matching the old synchronous ordering (a
    // body/handler compile error is reported before `finally`'s own grammar
    // is ever touched).
    let asm = match vm.compile_source_cached(&fin.to_str()) {
        Ok(asm) => asm,
        // A `finally` parse error is `finally`'s own exception overriding the
        // prior outcome, same as a runtime error in `finally` would (chains
        // `-during` to what `finally` superseded).
        Err(e) => return advance_after_finally(outcome, err(e.message)),
    };
    TryOutcome::Push(TryReq {
        asm,
        state: TryState {
            plan: Rc::clone(plan),
            phase: TryPhase::Finally { outcome },
        },
    })
}

/// `finally` just completed as `fc`: only its own non-`Ok` completion
/// overrides — chaining `-during` to the prior outcome's options if it threw.
fn advance_after_finally(outcome: Completion<Value>, fc: Completion<Value>) -> TryOutcome {
    if fc.code == Code::Ok {
        return TryOutcome::Deliver(outcome);
    }
    let mut fc = fc;
    if fc.code == Code::Error {
        let prior_opts = completion_options(&outcome);
        fc.options = add_during(&completion_options(&fc), &prior_opts);
    }
    TryOutcome::Deliver(fc)
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
