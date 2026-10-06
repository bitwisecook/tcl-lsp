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

//! Control flow (toward executing scripts) — `if` / `while` / `for` / `foreach`
//! / `lmap` plus `break` / `continue`.
//!
//! Bodies evaluate through the eval loop ([`Interp::eval_str`]); a body that
//! completes with `break`/`continue` is caught by the enclosing loop, while
//! `return`/error propagate. Conditions (`if`/`while`/`for`) are Tcl
//! **expressions** evaluated via the shared `expr` walk, so those three are
//! gated on the numeric tower like `expr` itself; `foreach`/`break`/`continue`
//! need no tower.
//!
//! Semantics verified against tclsh 9.0.

use crate::interp::{drop_fresh, new_string, obj_bytes, Code, Interp};
use crate::obj::TclObj;
use tcl_syntax::value::ValueOps;

/// Register the control-flow commands.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"break", break_cmd);
    interp.register_builtin(b"continue", continue_cmd);
    interp.register_builtin(b"foreach", foreach);
    interp.register_builtin(b"lmap", lmap);
    // `time` only evaluates a body `count` times — no numeric tower needed (the
    // count parses through the shared radix grammar, not `expr`).
    interp.register_builtin(b"time", time_cmd);
    interp.register_builtin(b"timerate", timerate_cmd);
    interp.register_builtin(b"tailcall", tailcall_cmd);
    // `if`/`while`/`for` test Tcl expressions → need the numeric tower.
    #[cfg(have_tommath)]
    {
        interp.register_builtin(b"if", if_cmd);
        interp.register_builtin(b"while", while_cmd);
        interp.register_builtin(b"for", for_cmd);
    }
}

// break / continue

fn break_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 1 {
        return interp.wrong_args(b"break");
    }
    // `break`/`continue` carry no value — clear any prior result so `catch
    // {break}` (and a `break` propagated out of an expr substitution) report the
    // empty string, matching C (where each command entry resets the result).
    interp.set_result_bytes(b"");
    Code::Break
}

fn continue_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 1 {
        return interp.wrong_args(b"continue");
    }
    interp.set_result_bytes(b"");
    Code::Continue
}

// time

/// `time command ?count?` — evaluate `command` (in the current frame) `count`
/// times (default 1) and report the average as the 4-element list
/// `N microseconds per iteration` (C `Tcl_TimeObjCmd`). `N` is an integer for
/// `count <= 1` (0 when `count <= 0`) and a double otherwise. A body that does
/// not complete `OK` (error / break / continue / return) propagates.
fn time_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return interp.wrong_args(b"time command ?count?");
    }
    let count: i128 = if argv.len() == 3 {
        let bytes = obj_bytes(argv[2]);
        match tcl_cmd_core::sort::parse_wide(&bytes) {
            Some(n) => n,
            None => {
                let mut m = b"expected integer but got \"".to_vec();
                m.extend_from_slice(&bytes);
                m.push(b'"');
                return interp.wrong_arguments_message(&m);
            }
        }
    } else {
        1
    };
    let script = obj_bytes(argv[1]);
    let start = interp.host().clock().now_micros();
    let mut i = count;
    while i > 0 {
        let code = interp.eval_body(&script);
        if code != Code::Ok {
            return code;
        }
        i -= 1;
    }
    let elapsed = interp
        .host()
        .clock()
        .now_micros()
        .saturating_sub(start)
        .max(0);
    // The result's first word matches C's list element type: an integer for
    // `count <= 1`, a double (`microseconds / count`) otherwise. Built as list
    // text (four simple words) so no per-element object bookkeeping is needed.
    let num = if count <= 1 {
        i64::try_from(if count <= 0 { 0 } else { elapsed })
            .unwrap_or(i64::MAX)
            .to_string()
    } else {
        #[allow(clippy::cast_precision_loss)]
        let per = elapsed as f64 / count as f64;
        let mut s = format!("{per}");
        if !s.contains(['.', 'e', 'E']) {
            s.push_str(".0");
        }
        s
    };
    let mut res = num.into_bytes();
    res.extend_from_slice(b" microseconds per iteration");
    interp.set_result_bytes(&res);
    Code::Ok
}

// timerate

/// `timerate ?-direct? ?-calibrate? ?-overhead double? command ?time ?max-count??`
/// (C `Tcl_TimeRateObjCmd`) — repeatedly evaluate `command` for up to `time`
/// milliseconds (default 1000) or `max-count` iterations, reporting the 8-word
/// list `<µs/#> µs/# <count> # <rate> #/sec <net-ms> net-ms`.
///
/// `-overhead d` subtracts `d` µs/iteration of measurement overhead from the
/// timing; `-calibrate` instead *measures* that overhead (the result then leads
/// with `<overhead> µs/#-overhead` and omits net-ms) and stores it as the default
/// overhead for later plain calls. `-direct` (don't byte-compile) is accepted for
/// compatibility but is a no-op here — the runtime is a tree-walker. A `break` in
/// the body forces an immediate stop; `continue` counts as a normal iteration;
/// error / return propagate. `time` defaults to 1000 ms; `max-count` to
/// unlimited, and `<= 0` means zero iterations.
fn timerate_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // The overhead sentinel: -1 means "use the calibrated default".
    let mut overhead: f64 = -1.0;
    let mut calibrate = false;
    // Option scan: everything up to the last three args (command ?time ?count??).
    let mut i = 1;
    while i + 1 < argv.len() {
        match obj_bytes(argv[i]).as_slice() {
            b"-direct" => {}
            b"-calibrate" => calibrate = true,
            b"-overhead" => {
                if i + 1 >= argv.len() - 1 {
                    return timerate_usage(interp);
                }
                i += 1;
                let b = obj_bytes(argv[i]);
                match core::str::from_utf8(&b)
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                {
                    Some(v) => overhead = v,
                    None => {
                        let mut m = b"expected floating-point number but got \"".to_vec();
                        m.extend_from_slice(&b);
                        m.push(b'"');
                        return interp.wrong_arguments_message(&m);
                    }
                }
            }
            b"--" => {
                i += 1;
                break;
            }
            _ => break,
        }
        i += 1;
    }
    // command ?time ?max-count??: after the options, 1..=3 words remain.
    if i >= argv.len() || i < argv.len().saturating_sub(3) {
        return timerate_usage(interp);
    }
    let script = obj_bytes(argv[i]);
    i += 1;
    // `time` present? (WIDE_MIN sentinel = "not given".)
    let mut max_ms: i128 = i128::MIN;
    let mut max_cnt: u128 = u128::MAX;
    if i < argv.len() {
        let b = obj_bytes(argv[i]);
        let Some(v) = tcl_cmd_core::sort::parse_wide(&b) else {
            return expected_integer(interp, &b);
        };
        max_ms = v;
        i += 1;
        if i < argv.len() {
            let b = obj_bytes(argv[i]);
            let Some(v) = tcl_cmd_core::sort::parse_wide(&b) else {
                return expected_integer(interp, &b);
            };
            max_cnt = if v > 0 { v as u128 } else { 0 };
        }
    }

    if calibrate {
        // No time given: run the self-refining calibration cycle.
        if max_ms == i128::MIN {
            return timerate_calibrate_cycle(interp, &script);
        }
        // Explicit zero time resets the stored overhead.
        if max_ms == 0 {
            interp.set_measure_overhead(0.0);
            interp.set_result_bytes(b"0");
            return Code::Ok;
        }
        // A positive time takes a fresh minimum; a negative time refines the
        // running one (keeping the current value as the ceiling).
        if max_ms > 0 {
            interp.set_measure_overhead(f64::MAX);
        } else {
            max_ms = -max_ms;
        }
    }
    if max_ms == i128::MIN {
        max_ms = 1000;
    }
    if overhead < 0.0 {
        overhead = interp.measure_overhead();
    }
    match timerate_measure(interp, &script, max_ms, max_cnt) {
        Ok((count, usec)) => {
            let res = timerate_format(interp, usec, count, overhead, calibrate);
            interp.set_result_bytes(&res);
            Code::Ok
        }
        Err(code) => code,
    }
}

fn timerate_usage(interp: &mut Interp) -> Code {
    interp.wrong_args(
        b"timerate ?-direct? ?-calibrate? ?-overhead double? command ?time ?max-count??",
    )
}

fn expected_integer(interp: &mut Interp, got: &[u8]) -> Code {
    let mut m = b"expected integer but got \"".to_vec();
    m.extend_from_slice(got);
    m.push(b'"');
    interp.wrong_arguments_message(&m)
}

/// The measurement loop: evaluate `script` until `max_ms` elapses or `max_cnt`
/// iterations run, returning `(count, elapsed-µs)`. A non-`Ok`/`Continue`/`Break`
/// completion propagates as `Err(code)`; `Break` stops immediately.
fn timerate_measure(
    interp: &mut Interp,
    script: &[u8],
    max_ms: i128,
    max_cnt: u128,
) -> Result<(u128, u128), Code> {
    if max_cnt == 0 {
        return Ok((0, 0));
    }
    let start = interp.host().clock().now_micros();
    let stop = start + max_ms.max(0) * 1000;
    let mut count: u128 = 0;
    loop {
        let code = interp.eval_body(script);
        count += 1;
        let forced_stop = match code {
            Code::Ok | Code::Continue => false,
            Code::Break => true,
            other => return Err(other),
        };
        let middle = interp.host().clock().now_micros();
        if forced_stop || middle >= stop || count >= max_cnt {
            let usec = u128::try_from(middle.saturating_sub(start).max(0)).unwrap_or(0);
            return Ok((count, usec));
        }
    }
}

/// Format the 8-element result list. In `calibrate` mode the list leads with the
/// (re-)measured overhead and omits net-ms; otherwise the given `overhead` is
/// subtracted from the elapsed time first.
fn timerate_format(
    interp: &Interp,
    usec_raw: u128,
    count: u128,
    overhead: f64,
    calibrate: bool,
) -> Vec<u8> {
    // Zero iterations: the fixed all-zero shape (avoids divide-by-zero).
    if count == 0 {
        return b"0 \xc2\xb5s/# 0 # 0 #/sec 0 net-ms".to_vec();
    }
    let mut out: Vec<u8> = Vec::new();
    let mut usec = usec_raw;
    if calibrate {
        // New overhead estimate = min(stored, µs/iter).
        let cur = usec_raw as f64 / count as f64;
        let est = interp.measure_overhead().min(cur);
        interp.set_measure_overhead(est);
        out.extend_from_slice(format!("{est}").as_bytes());
        out.extend_from_slice(b" \xc2\xb5s/#-overhead ");
    } else if overhead > 0.0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let cur = (overhead * count as f64) as u128;
        usec = usec.saturating_sub(cur);
    }
    // µs per iteration (objs[0]), digits chosen by magnitude of the integer part.
    let val = usec / count;
    if val >= 1_000_000 {
        out.extend_from_slice(val.to_string().as_bytes());
    } else {
        let digits = if val < 10 {
            6
        } else if val < 100 {
            4
        } else if val < 1000 {
            3
        } else if val < 10000 {
            2
        } else {
            1
        };
        out.extend_from_slice(format!("{:.*}", digits, usec as f64 / count as f64).as_bytes());
    }
    out.extend_from_slice(b" \xc2\xb5s/# ");
    out.extend_from_slice(count.to_string().as_bytes());
    out.extend_from_slice(b" # ");
    // Rate (#/sec), computed from the elapsed time (bumped to >=1 to avoid /0).
    let rate_usec = usec.max(1);
    if count < (i64::MAX as u128) / 1_000_000 {
        let rate = (count * 1_000_000) / rate_usec;
        if rate < 100_000 {
            let digits = if rate < 100 {
                3
            } else if rate < 1000 {
                2
            } else {
                1
            };
            out.extend_from_slice(
                format!(
                    "{:.*}",
                    digits,
                    (count * 1_000_000) as f64 / rate_usec as f64
                )
                .as_bytes(),
            );
        } else {
            out.extend_from_slice(rate.to_string().as_bytes());
        }
    } else {
        out.extend_from_slice(((count / rate_usec) * 1_000_000).to_string().as_bytes());
    }
    out.extend_from_slice(b" #/sec");
    if !calibrate {
        // Net execution time (ms).
        out.extend_from_slice(b" ");
        if rate_usec >= 1 && usec >= 1 {
            out.extend_from_slice(format!("{:.3}", usec as f64 / 1000.0).as_bytes());
        } else {
            out.extend_from_slice(b"0");
        }
        out.extend_from_slice(b" net-ms");
    }
    out
}

/// The `-calibrate` cycle with no explicit time (C's self-recursive path): a
/// warm-up run, then repeated refinement measurements with a growing budget
/// until the overhead estimate stops improving (or a ~5 s cap is hit). Returns
/// the last refinement's calibrate-format result.
fn timerate_calibrate_cycle(interp: &mut Interp, script: &[u8]) -> Code {
    // Warm-up: a plain 100 ms run (result discarded), overhead reset to zero.
    interp.set_measure_overhead(0.0);
    if let Err(code) = timerate_measure(interp, script, 100, u128::MAX) {
        return code;
    }
    // Seed the running minimum high so the first measurement sets it.
    interp.set_measure_overhead(f64::MAX);
    let mut max_ms: i128 = 1000;
    let mut budget: i128 = 5000;
    let result = loop {
        let last = interp.measure_overhead();
        let formatted = match timerate_measure(interp, script, max_ms, u128::MAX) {
            Ok((count, usec)) => timerate_format(interp, usec, count, 0.0, true),
            Err(code) => return code,
        };
        budget -= max_ms;
        max_ms += max_ms / 4;
        let now = interp.measure_overhead();
        // Keep refining while the estimate is still moving (worse/equal, or a
        // > 0.05% improvement); stop once it settles into a <= 0.05% improvement
        // or the ~5 s budget is spent (C's do/while convergence test).
        let settled = now < last && now / last > 0.9995;
        if settled || budget <= 0 {
            break formatted;
        }
    };
    interp.set_result_bytes(&result);
    Code::Ok
}

// tailcall

/// `tailcall command ?arg ...?` — arrange for `command args` to run and its
/// result to become the enclosing proc's result. Must be called from a proc /
/// lambda / method. The actual selected native pending code retains the
/// replacement on that frame. Dispatch occurs in the caller's variable frame
/// after issuer teardown and execution-leave callbacks.
fn tailcall_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.schedule_tailcall(&argv[1..])
}

// if

/// `if expr1 ?then? body1 elseif expr2 ?then? body2 ... ?else? ?bodyN?`.
#[cfg(have_tommath)]
fn if_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let objc = argv.len();
    let mut i = 1;
    // The body of the first true condition, recorded but **not executed** until
    // the whole `if` grammar is validated (C's `Tcl_IfObjCmd` `thenScriptIndex`):
    // a matched branch still rejects malformed trailing clauses, and conditions
    // after the first true one are not evaluated (their side effects are skipped).
    let mut then_body: Option<*mut TclObj> = None;
    // The keyword before the expected expression — for the "no expression" error.
    let mut clause: &[u8] = b"if";

    loop {
        if i >= objc {
            let mut m = b"wrong # args: no expression after \"".to_vec();
            m.extend_from_slice(clause);
            m.extend_from_slice(b"\" argument");
            return interp.wrong_arguments_message(&m);
        }
        let cond_obj = argv[i];
        i += 1;
        // Optional `then` keyword.
        if i < objc && obj_bytes(argv[i]).as_slice() == b"then" {
            i += 1;
        }
        if i >= objc {
            return no_script_following(interp, &obj_bytes(argv[i - 1]));
        }
        if then_body.is_none() {
            match crate::builtins::eval_bool_expr(interp, cond_obj) {
                Ok(true) => then_body = Some(argv[i]),
                Ok(false) => {}
                Err(code) => return code,
            }
        }
        i += 1; // consume the body
        if i >= objc {
            break; // no further clauses
        }
        if obj_bytes(argv[i]).as_slice() == b"elseif" {
            i += 1;
            clause = b"elseif";
            continue;
        }
        break;
    }

    // Past the `elseif` chain: an optional `else` then exactly one body, or a
    // single bare implicit-else body. Anything else is "extra words".
    if i < objc && obj_bytes(argv[i]).as_slice() == b"else" {
        i += 1;
        if i >= objc {
            return no_script_following(interp, b"else");
        }
    }
    if i < objc.saturating_sub(1) {
        return interp.wrong_arguments_message(
            b"wrong # args: extra words after \"else\" clause in \"if\" command",
        );
    }

    match then_body {
        Some(body) => interp.eval_control_body(body),
        None if i < objc => interp.eval_control_body(argv[i]),
        None => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
    }
}

/// `wrong # args: no script following "<token>" argument` (C's `missingScript`).
/// Tower-gated with its only callers (`if`'s clause scan).
#[cfg(have_tommath)]
fn no_script_following(interp: &mut Interp, token: &[u8]) -> Code {
    let mut m = b"wrong # args: no script following \"".to_vec();
    m.extend_from_slice(token);
    m.extend_from_slice(b"\" argument");
    interp.wrong_arguments_message(&m)
}

// while

/// `while test body`.
#[cfg(have_tommath)]
fn while_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"while test command");
    }
    let cond = argv[1];
    let body = argv[2];
    loop {
        if let Some(code) = interp.limit_check_tick() {
            return code;
        }
        match crate::builtins::eval_bool_expr(interp, cond) {
            Ok(true) => {}
            Ok(false) => break,
            Err(code) => return code,
        }
        match interp.eval_control_body(body) {
            Code::Ok | Code::Continue => {}
            Code::Break => break,
            Code::Error => {
                // `("while" body line N)` — the interpreted `Tcl_WhileObjCmd`
                // frame, added only outside a proc (Tcl inlines `while` when it
                // compiles a proc body, so no frame there; see `foreach`).
                if !interp.in_proc() {
                    interp.append_body_frame(b"while");
                }
                return Code::Error;
            }
            other => return other, // return propagates
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// for

/// `for start test next body`.
#[cfg(have_tommath)]
fn for_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return interp.wrong_args(b"for start test next command");
    }
    let (init, cond, next, body) = (argv[1], argv[2], argv[3], argv[4]);
    // `start`/`next` are scripts too — run them through `eval_control_body` so a
    // located literal reports `type source` at its own line (TIP 280), matching
    // the body. (Their result is discarded; only their completion code matters.)
    match interp.eval_control_body(init) {
        Code::Ok => {}
        Code::Error => {
            // `("for" initial command)` (no line) — interpreted `Tcl_ForObjCmd`.
            if !interp.in_proc() {
                interp.append_frame_noline(b"\"for\" initial command");
            }
            return Code::Error;
        }
        other => return other,
    }
    loop {
        if let Some(code) = interp.limit_check_tick() {
            return code;
        }
        match crate::builtins::eval_bool_expr(interp, cond) {
            Ok(true) => {}
            Ok(false) => break,
            Err(code) => return code,
        }
        match interp.eval_control_body(body) {
            Code::Ok | Code::Continue => {} // `continue` still runs `next`
            Code::Break => break,
            Code::Error => {
                // `("for" body line N)` (interpreted `Tcl_ForObjCmd`), outside a
                // proc only — as for `while`/`foreach`.
                if !interp.in_proc() {
                    interp.append_body_frame(b"for");
                }
                return Code::Error;
            }
            other => return other,
        }
        match interp.eval_control_body(next) {
            Code::Ok => {}
            // A `break` in the step clause ends the loop cleanly (C's
            // `Tcl_ForObjCmd`: `case TCL_BREAK: result = TCL_OK`), unlike
            // `continue`/`return`, which propagate.
            Code::Break => break,
            Code::Error => {
                // `("for" loop-end command)` (no line) — interpreted `Tcl_ForObjCmd`.
                if !interp.in_proc() {
                    interp.append_frame_noline(b"\"for\" loop-end command");
                }
                return Code::Error;
            }
            other => return other,
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// foreach / lmap

/// `foreach varList list ?varList list ...? body` — iterate one or more
/// (var-list, value-list) groups in parallel, padding exhausted lists with `""`.
fn foreach(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    each_loop(interp, argv, false)
}

/// `lmap varList list ?varList list ...? body` — `foreach` that collects each
/// (non-`continue`) body result into a list and returns it. `break` ends the
/// loop and returns the list accumulated so far. Mirrors C's `EachloopCmd`
/// (`tclCmdAH.c`) with `TCL_EACH_COLLECT`.
fn lmap(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    each_loop(interp, argv, true)
}

/// Execute the authenticated original-object generic loop schedule.
fn each_loop(interp: &mut Interp, argv: &[*mut TclObj], collect: bool) -> Code {
    use tcl_cmd_core::native_each_loop::{BodyDecision, EachLoopAction};
    use tcl_runtime_api::completion_options::ControlOptionPolicy;
    use tcl_runtime_api::native_each_loop::NativeEachLoopKind;
    let kind = if collect {
        NativeEachLoopKind::Lmap
    } else {
        NativeEachLoopKind::Foreach
    };
    if argv.len() < 4 || argv.len() % 2 != 0 {
        return interp.wrong_args(if collect {
            b"lmap varList list ?varList list ...? command"
        } else {
            b"foreach varList list ?varList list ...? command"
        });
    }
    let dialect = interp.native_invocation_dialect();
    let Some(protocol) = dialect.native_each_loop_protocol(kind) else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("native generic each-loop")
                .into(),
        );
    };
    let Some(strings) = dialect.native_string_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native each-loop List storage",
            )
            .into(),
        );
    };
    let recipe = protocol.recipe();
    struct Group {
        variables: *mut TclObj,
        values: *mut TclObj,
        _variable_owner: Option<crate::obj::Owned>,
        _value_owner: Option<crate::obj::Owned>,
        _variable_lifetime: crate::obj::NativeObjectLifetime,
        _value_lifetime: crate::obj::NativeObjectLifetime,
        variable_items: Option<crate::list::NativeListBacking>,
        value_items: Option<crate::list::NativeListBacking>,
        abstract_values: Option<crate::native_arithseries::NativeEachLoopAbstractValues>,
    }
    fn members(
        root: *mut TclObj,
        strings: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<crate::list::NativeListBacking, tcl_syntax::value::ValueError> {
        drop(crate::list::list_elements_native_checked(root, strings)?);
        crate::list::native_list_backing(root).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native each-loop concrete List backing",
            ),
        )
    }
    if let Err(error) = interp.associate_native_jim_arguments(argv) {
        return interp.report_cmd_error(error.into());
    }
    let mut groups = Vec::new();
    let mut lengths = Vec::new();
    let mut empty = false;
    for pair in argv[1..argv.len() - 1].chunks_exact(2) {
        let variable_owner = if recipe.copies_headers() {
            match crate::list::native_list_copy(pair[0], strings) {
                Ok(owner) => Some(owner),
                Err(error) => return interp.report_cmd_error(error.into()),
            }
        } else {
            None
        };
        let variables = variable_owner
            .as_ref()
            .map_or(pair[0], crate::obj::Owned::as_ptr);
        let variable_items = match members(variables, strings) {
            Ok(items) => items,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let variable_count = variable_items.len();
        empty |= variable_count == 0;
        if empty && !recipe.live_iterators() {
            return interp.report_cmd_error(tcl_cmd_core::native_each_loop::empty_variables(
                recipe, kind,
            ));
        }
        let abstract_values =
            match crate::native_arithseries::capture_native_each_loop_abstract(pair[1], strings) {
                Ok(values) => values,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
        let value_owner = if abstract_values.is_some() {
            None
        } else if recipe.copies_headers() {
            match crate::list::native_list_copy(pair[1], strings) {
                Ok(owner) => Some(owner),
                Err(error) => return interp.report_cmd_error(error.into()),
            }
        } else {
            None
        };
        let values = value_owner
            .as_ref()
            .map_or(pair[1], crate::obj::Owned::as_ptr);
        let value_items = if abstract_values.is_some() || recipe.live_iterators() {
            None
        } else {
            match members(values, strings) {
                Ok(items) => Some(items),
                Err(error) => return interp.report_cmd_error(error.into()),
            }
        };
        lengths.push((
            variable_count,
            abstract_values.as_ref().map_or_else(
                || value_items.as_ref().map_or(0, |items| items.len()),
                crate::native_arithseries::NativeEachLoopAbstractValues::length,
            ),
        ));
        groups.push(Group {
            variables,
            values,
            _variable_owner: variable_owner,
            _value_owner: value_owner,
            _variable_lifetime: crate::obj::NativeObjectLifetime::retain(variables),
            _value_lifetime: crate::obj::NativeObjectLifetime::retain(values),
            variable_items: Some(variable_items),
            value_items,
            abstract_values,
        });
    }
    if empty {
        return interp.report_cmd_error(tcl_cmd_core::native_each_loop::empty_variables(
            recipe, kind,
        ));
    }
    let jim_empty = if recipe.live_iterators() {
        match interp.native_jim_object_context() {
            Ok(context) => {
                // Jim retains resultObj, which is emptyObj only for foreach.
                // lmap borrows emptyObj for padding and owns a separate List.
                let result_owner =
                    (!collect).then(|| crate::obj::Owned::retain(context.empty_object().as_ptr()));
                Some((context, result_owner))
            }
            Err(error) => return interp.report_cmd_error(error.into()),
        }
    } else {
        None
    };
    let policy = if collect {
        ControlOptionPolicy::FRESH_FORWARDED
    } else {
        ControlOptionPolicy::FRESH_SETTLED
    };
    interp.begin_control_options(policy);
    let mut cursor = tcl_cmd_core::native_each_loop::EachLoopState::new(recipe, kind, lengths);
    let mut collected = Vec::<crate::obj::Owned>::new();
    loop {
        match cursor.advance() {
            EachLoopAction::Check(index) => {
                let group = &mut groups[index];
                group.value_items = None;
                let items = match members(group.values, strings) {
                    Ok(items) => items,
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                cursor.set_value_length(index, items.len());
                group.value_items = Some(items);
            }
            EachLoopAction::Refresh(index) => {
                let group = &mut groups[index];
                group.variable_items = None;
                let variables = match members(group.variables, strings) {
                    Ok(items) => items,
                    Err(_) if recipe.refetches_groups() => {
                        return interp.report_cmd_error(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "native Tcl 8.4 each-loop variable-list refetch fatal boundary",
                            )
                            .into(),
                        );
                    }
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                let count = variables.len();
                group.variable_items = Some(variables);
                if !recipe.live_iterators() || cursor.variable_cursor() < count {
                    group.value_items = None;
                    let values =
                        match members(group.values, strings) {
                            Ok(items) => items,
                            Err(_) if recipe.refetches_groups() => return interp.report_cmd_error(
                                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                    "native Tcl 8.4 each-loop value-list refetch fatal boundary",
                                )
                                .into(),
                            ),
                            Err(error) => return interp.report_cmd_error(error.into()),
                        };
                    cursor.set_lengths(index, count, values.len());
                    group.value_items = Some(values);
                } else {
                    cursor.set_lengths(
                        index,
                        count,
                        group.value_items.as_ref().map_or(0, |items| items.len()),
                    );
                }
            }
            EachLoopAction::Assign {
                group,
                variable,
                value,
            } => {
                let group = &groups[group];
                let name = match group
                    .variable_items
                    .as_ref()
                    .expect("native variable header")
                    .elements()
                {
                    Ok(elements) => elements[variable],
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                let fresh_value = (value.is_none() && jim_empty.is_none())
                    || (value.is_some() && group.abstract_values.is_some());
                let assigned = match value {
                    Some(index) if group.abstract_values.is_some() => {
                        match group.abstract_values.as_ref().unwrap().element(index) {
                            Ok(value) => value,
                            Err(error) => return interp.report_cmd_error(error.into()),
                        }
                    }
                    Some(index) => match group
                        .value_items
                        .as_ref()
                        .expect("native value header")
                        .elements()
                    {
                        Ok(elements) => elements[index],
                        Err(error) => return interp.report_cmd_error(error.into()),
                    },
                    None => jim_empty.as_ref().map_or_else(
                        || new_string(b""),
                        |(context, _)| context.empty_object().as_ptr(),
                    ),
                };
                let fresh_lifetime =
                    fresh_value.then(|| crate::obj::NativeObjectLifetime::retain(assigned));
                let transient = recipe
                    .pins_assignment_value()
                    .then(|| crate::obj::Owned::retain(assigned));
                let stored = interp.assign_original_named_variable(name, assigned);
                drop(transient);
                if fresh_value && crate::obj::allocation_is_live(assigned) {
                    // SAFETY: the allocation-only lease keeps the header valid.
                    // A trace can withdraw the native reference while the
                    // original setter is running; such a retired header must
                    // never be promoted again during caller cleanup.
                    if unsafe { (*assigned).ref_count == 0 } {
                        drop_fresh(assigned);
                    }
                }
                drop(fresh_lifetime);
                if let Err(code) = stored {
                    let original = match interp.native_string_bytes(&name) {
                        Ok(bytes) => bytes,
                        Err(error) => return interp.report_cmd_error(error.into()),
                    };
                    match tcl_cmd_core::native_each_loop::setter_failure(recipe, kind, &original) {
                        tcl_cmd_core::native_each_loop::SetterFailure::Preserve => return code,
                        tcl_cmd_core::native_each_loop::SetterFailure::Replace(error) => {
                            return interp.report_cmd_error(error);
                        }
                        tcl_cmd_core::native_each_loop::SetterFailure::Context(context) => {
                            interp.append_error_info_context(&context);
                            return code;
                        }
                    }
                }
            }
            EachLoopAction::Body => {
                interp.begin_control_options(policy);
                let code = interp.eval_generic_control_body(argv[argv.len() - 1]);
                match cursor.body_completion(tcl_core_types::Code::from_int(
                    i32::try_from(code.as_int()).expect("runtime completion code is a C int"),
                )) {
                    BodyDecision::Collect => {
                        collected.push(crate::obj::Owned::retain(interp.get_obj_result()))
                    }
                    BodyDecision::Continue | BodyDecision::Finish => {}
                    BodyDecision::Propagate => {
                        if code == Code::Error {
                            interp.append_body_frame(kind.name().as_bytes());
                        }
                        return code;
                    }
                }
            }
            EachLoopAction::Finish => {
                if collect && (cursor.entered_body() || recipe.empty_lmap_publishes_list()) {
                    let values = collected
                        .iter()
                        .map(crate::obj::Owned::as_ptr)
                        .collect::<Vec<_>>();
                    let list = interp.new_list_object(&values);
                    drop(collected);
                    interp.set_result(list);
                } else if let Some((context, _result_owner)) = jim_empty {
                    interp.set_result(context.empty_object().as_ptr());
                } else {
                    interp.set_result_bytes(b"");
                }
                interp.settle_control_options(policy, Code::Ok);
                return Code::Ok;
            }
        }
    }
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
            "eval {:?} → {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    #[test]
    fn time_reports_microseconds_and_propagates() {
        leak_free(|i| {
            // `count 0` is deterministic (no iterations run).
            assert_eq!(run(i, b"time {} 0"), b"0 microseconds per iteration");
            // A real body: the report ends in the fixed suffix (the count is
            // timing-dependent, so only the shape is asserted).
            let r = run(i, b"time {set x 1}");
            assert!(
                r.ends_with(b" microseconds per iteration"),
                "{:?}",
                String::from_utf8_lossy(&r)
            );
            // A non-`OK` body propagates its code + message.
            assert_eq!(i.eval_str(b"time {error boom}"), Code::Error);
            assert_eq!(i.result_bytes(), b"boom");
            // A non-integer count is the standard error.
            assert_eq!(i.eval_str(b"time {} xyz"), Code::Error);
            assert_eq!(i.result_bytes(), b"expected integer but got \"xyz\"");
            // Arity.
            assert_eq!(i.eval_str(b"time"), Code::Error);
        });
    }

    #[test]
    fn timerate_reports_the_eight_word_list_and_honours_limits() {
        leak_free(|i| {
            // `max-count 0` runs nothing: the fixed all-zero shape.
            assert_eq!(
                run(i, b"timerate {set x 1} 100 0"),
                b"0 \xc2\xb5s/# 0 # 0 #/sec 0 net-ms"
            );
            // A real run: eight words with the fixed labels (numbers vary). A
            // large time budget with a small max-count makes the count exact
            // (the count limit is reached long before the time limit).
            run(i, b"set r [timerate {set x 1} 100000 50]");
            assert_eq!(run(i, b"llength $r"), b"8");
            assert_eq!(run(i, b"lindex $r 1"), b"\xc2\xb5s/#");
            assert_eq!(run(i, b"lindex $r 2"), b"50");
            assert_eq!(run(i, b"lindex $r 3"), b"#");
            assert_eq!(run(i, b"lindex $r 5"), b"#/sec");
            assert_eq!(run(i, b"lindex $r 7"), b"net-ms");
            // `break` in the body forces a stop after one iteration.
            assert_eq!(run(i, b"lindex [timerate {break} 1000] 2"), b"1");
            // `continue` counts as a normal iteration.
            assert_eq!(run(i, b"lindex [timerate {continue} 100000 3] 2"), b"3");
            // An error / return propagates.
            assert_eq!(i.eval_str(b"timerate {error boom} 100"), Code::Error);
            assert_eq!(i.result_bytes(), b"boom");
        });
    }

    #[test]
    fn timerate_options_calibrate_and_errors() {
        leak_free(|i| {
            // `-overhead` is accepted and the shape is unchanged.
            assert_eq!(
                run(i, b"llength [timerate -overhead 0.01 {set x 1} 20]"),
                b"8"
            );
            // `-direct` and `--` are accepted.
            assert_eq!(run(i, b"llength [timerate -direct -- {set x 1} 20]"), b"8");
            // `-calibrate` leads with the overhead word and omits net-ms.
            run(i, b"set c [timerate -calibrate {set x 1} 20]");
            assert_eq!(run(i, b"llength $c"), b"8");
            assert_eq!(run(i, b"lindex $c 1"), b"\xc2\xb5s/#-overhead");
            assert_eq!(run(i, b"lindex $c 7"), b"#/sec");
            // `-calibrate ... 0` resets the stored overhead and returns 0.
            assert_eq!(run(i, b"timerate -calibrate {set x 1} 0"), b"0");
            // Error paths: bad time, bad overhead, arity.
            assert_eq!(i.eval_str(b"timerate {set x 1} notint"), Code::Error);
            assert_eq!(i.result_bytes(), b"expected integer but got \"notint\"");
            assert_eq!(
                i.eval_str(b"timerate -overhead xx {set x 1} 20"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"expected floating-point number but got \"xx\""
            );
            assert_eq!(i.eval_str(b"timerate"), Code::Error);
            assert!(i.result_bytes().starts_with(b"wrong # args"));
        });
    }

    #[test]
    fn tailcall_resolves_after_issuer_leave_in_the_callers_variable_frame() {
        let cases = [
            "proc target {} {return OLD}; proc leave {cmd code result op} {set ::leave [list $code $result]; rename target old; proc target {} {return NEW}}; proc P {} {tailcall target}; trace add execution P leave leave; list [P] $::leave",
            "proc target {} {set x NEW}; proc issuer {} {set x WRONG; tailcall target}; proc caller {} {set x OLD; issuer; set x}; caller",
        ];
        for reference in tcl_test_support::available_tclshs()
            .into_iter()
            .filter(|reference| reference.version >= tcl_dialect::TclVersion::V8_6)
        {
            let profile = tcl_registry::model::ingress::resolve_environment(
                reference.version.dialect_profile_name(),
            )
            .unit_profile();
            for script in cases {
                let source = format!("puts [eval {{{script}}}]\n");
                let expected = tcl_test_support::run_script(&reference.path, source.as_bytes())
                    .expect("native tailcall observation")
                    .strict_text()
                    .expect("native result");
                leak_free(|interp| {
                    interp.set_dialect_profile(profile);
                    assert_eq!(interp.eval_str(script.as_bytes()), Code::Ok);
                    assert_eq!(
                        interp.result_bytes(),
                        expected.as_bytes(),
                        "{:?}: {script}",
                        reference.version
                    );
                });
            }
        }
    }

    #[test]
    fn tailcall_returns_the_invoked_result_from_the_proc() {
        leak_free(|i| {
            run(i, b"proc g {} {return G}");
            run(i, b"proc f {} {tailcall g; return NOPE}");
            assert_eq!(run(i, b"f"), b"G");
            // With arguments.
            run(i, b"proc h {a b} {return $a$b}");
            run(i, b"proc f2 {} {tailcall h X Y}");
            assert_eq!(run(i, b"f2"), b"XY");
            // No command → returns "" from the proc.
            run(i, b"proc f3 {} {tailcall; return NOPE}");
            assert_eq!(run(i, b"f3"), b"");
            // Outside a proc → error.
            assert_eq!(i.eval_str(b"tailcall x"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"tailcall can only be called from a proc, lambda or method"
            );
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn if_elseif_else() {
        leak_free(|i| {
            assert_eq!(run(i, b"if {1 < 2} {set x yes}"), b"yes");
            assert_eq!(run(i, b"if {0} {set x a} else {set x b}"), b"b");
            assert_eq!(
                run(i, b"if {0} {set x a} elseif {1} {set x c} else {set x d}"),
                b"c"
            );
            // false with no else → empty result.
            assert_eq!(run(i, b"if {0} {set x a}"), b"");
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn while_and_for_accumulate() {
        leak_free(|i| {
            assert_eq!(run(i, b"set n 0; while {$n < 3} {incr n}; set n"), b"3");
            assert_eq!(
                run(
                    i,
                    b"set s 0; for {set i 1} {$i <= 5} {incr i} {incr s $i}; set s"
                ),
                b"15"
            );
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn break_and_continue() {
        leak_free(|i| {
            assert_eq!(
                run(
                    i,
                    b"set out {}; foreach x {1 2 3 4} {if {$x==3} break; append out $x}; set out"
                ),
                b"12"
            );
            assert_eq!(
                run(
                    i,
                    b"set out {}; foreach x {1 2 3 4} {if {$x==2} continue; append out $x}; set out"
                ),
                b"134"
            );
        });
    }

    #[test]
    fn foreach_single_multi_var_and_parallel() {
        leak_free(|i| {
            assert_eq!(
                run(i, b"set out {}; foreach x {a b c} {append out $x}; set out"),
                b"abc"
            );
            assert_eq!(
                run(
                    i,
                    b"set out {}; foreach {k v} {1 2 3 4} {append out \"$k=$v \"}; set out"
                ),
                b"1=2 3=4 "
            );
            // parallel lists pad the exhausted one with "".
            assert_eq!(
                run(
                    i,
                    b"set out {}; foreach a {1 2} b {x y z} {append out \"$a$b \"}; set out"
                ),
                b"1x 2y z "
            );
        });
    }

    // Needs the numeric tower: the continue/break cases branch via `if`.
    #[cfg(have_tommath)]
    #[test]
    fn lmap_collects_results() {
        leak_free(|i| {
            // Single-var collect.
            assert_eq!(run(i, b"lmap x {a b c} {string toupper $x}"), b"A B C");
            // Multi-var per iteration → one element per iteration.
            assert_eq!(run(i, b"lmap {a b} {1 2 3 4} {list $a $b}"), b"{1 2} {3 4}");
            // Parallel lists pad with "".
            assert_eq!(
                run(i, b"lmap a {1 2} b {x y z} {list $a $b}"),
                b"{1 x} {2 y} {{} z}"
            );
            // `continue` skips collection; `break` ends with the list so far.
            assert_eq!(
                run(i, b"lmap x {1 2 3 4} {if {$x % 2 == 0} continue; set x}"),
                b"1 3"
            );
            assert_eq!(
                run(i, b"lmap x {1 2 3 4} {if {$x == 3} break; set x}"),
                b"1 2"
            );
            // Empty body → one empty element per iteration.
            assert_eq!(run(i, b"lmap x {1 2 3} {}"), b"{} {} {}");
        });
    }
}

#[cfg(test)]
mod native_each_loop_tests;
