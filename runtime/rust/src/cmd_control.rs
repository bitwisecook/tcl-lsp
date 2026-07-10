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

/// Register the control-flow commands.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"break", break_cmd);
    interp.register_builtin(b"continue", continue_cmd);
    interp.register_builtin(b"foreach", foreach);
    interp.register_builtin(b"lmap", lmap);
    // `time` only evaluates a body `count` times — no numeric tower needed (the
    // count parses through the shared radix grammar, not `expr`).
    interp.register_builtin(b"time", time_cmd);
    interp.register_builtin(b"tailcall", tailcall_cmd);
    // `if`/`while`/`for` test Tcl expressions → need the numeric tower.
    #[cfg(have_tommath)]
    {
        interp.register_builtin(b"if", if_cmd);
        interp.register_builtin(b"while", while_cmd);
        interp.register_builtin(b"for", for_cmd);
    }
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

// -- break / continue ------------------------------------------------------

fn break_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 1 {
        return wrong_args(interp, b"break");
    }
    // `break`/`continue` carry no value — clear any prior result so `catch
    // {break}` (and a `break` propagated out of an expr substitution) report the
    // empty string, matching C (where each command entry resets the result).
    interp.set_result_bytes(b"");
    Code::Break
}

fn continue_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 1 {
        return wrong_args(interp, b"continue");
    }
    interp.set_result_bytes(b"");
    Code::Continue
}

// -- time ------------------------------------------------------------------

/// `time command ?count?` — evaluate `command` (in the current frame) `count`
/// times (default 1) and report the average as the 4-element list
/// `N microseconds per iteration` (C `Tcl_TimeObjCmd`). `N` is an integer for
/// `count <= 1` (0 when `count <= 0`) and a double otherwise. A body that does
/// not complete `OK` (error / break / continue / return) propagates.
fn time_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return wrong_args(interp, b"time command ?count?");
    }
    let count: i128 = if argv.len() == 3 {
        let bytes = obj_bytes(argv[2]);
        match tcl_cmd_core::sort::parse_wide(&bytes) {
            Some(n) => n,
            None => {
                let mut m = b"expected integer but got \"".to_vec();
                m.extend_from_slice(&bytes);
                m.push(b'"');
                return interp.set_error(&m);
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

// -- tailcall --------------------------------------------------------------

/// `tailcall command ?arg ...?` — arrange for `command args` to run and its
/// result to become the enclosing proc's result. Must be called from a proc /
/// lambda / method. This mirrors the bytecode VM's pragmatic form: `command
/// args` is dispatched now and a `TCL_RETURN` carries its result out of the
/// proc (rather than being deferred to run in the caller's frame after unwind —
/// the observable proc result is the same for the common case).
fn tailcall_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if !interp.in_proc() {
        return interp.set_error(b"tailcall can only be called from a proc, lambda or method");
    }
    if argv.len() < 2 {
        // `tailcall` with no command is a plain return of "".
        interp.set_result_bytes(b"");
        return Code::Return;
    }
    let code = interp.dispatch(&argv[1..]);
    if code == Code::Ok {
        Code::Return
    } else {
        code
    }
}

// -- if --------------------------------------------------------------------

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
            return interp.set_error(&m);
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
        return interp
            .set_error(b"wrong # args: extra words after \"else\" clause in \"if\" command");
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
fn no_script_following(interp: &mut Interp, token: &[u8]) -> Code {
    let mut m = b"wrong # args: no script following \"".to_vec();
    m.extend_from_slice(token);
    m.extend_from_slice(b"\" argument");
    interp.set_error(&m)
}

// -- while -----------------------------------------------------------------

/// `while test body`.
#[cfg(have_tommath)]
fn while_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"while test command");
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

// -- for -------------------------------------------------------------------

/// `for start test next body`.
#[cfg(have_tommath)]
fn for_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return wrong_args(interp, b"for start test next command");
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

// -- foreach / lmap --------------------------------------------------------

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

/// Shared `foreach`/`lmap` engine. With `collect`, each successful body result
/// is appended to a result list (the `lmap` return value); without, the result
/// is the empty string (`foreach`). The two differ only in result collection
/// and the body-frame command name — exactly as C factors them through one
/// `EachloopCmd`.
fn each_loop(interp: &mut Interp, argv: &[*mut TclObj], collect: bool) -> Code {
    let name: &[u8] = if collect { b"lmap" } else { b"foreach" };
    // [cmd] + N×(varlist, list) pairs + body ⇒ an even arg count ≥ 4.
    if argv.len() < 4 || argv.len() % 2 != 0 {
        let usage: &[u8] = if collect {
            b"lmap varList list ?varList list ...? command"
        } else {
            b"foreach varList list ?varList list ...? command"
        };
        return wrong_args(interp, usage);
    }
    let body = argv[argv.len() - 1];

    // Parse each group's variable names + values; track the iteration count.
    // A group is `(variable names, values)`.
    type Group = (Vec<Vec<u8>>, Vec<Vec<u8>>);
    let mut groups: Vec<Group> = Vec::new();
    let mut iterations = 0usize;
    let pairs = &argv[1..argv.len() - 1];
    for pair in pairs.chunks_exact(2) {
        let vars = match crate::parse::split_list(&obj_bytes(pair[0])) {
            Ok(v) => v,
            Err(e) => return interp.set_error(e.message()),
        };
        if vars.is_empty() {
            let mut m = name.to_vec();
            m.extend_from_slice(b" varlist is empty");
            return interp.set_error(&m);
        }
        let vals = match crate::parse::split_list(&obj_bytes(pair[1])) {
            Ok(v) => v,
            Err(e) => return interp.set_error(e.message()),
        };
        iterations = iterations.max(vals.len().div_ceil(vars.len()));
        groups.push((vars, vals));
    }

    // Collected body results (bytes; rematerialised into the result list at the
    // end). Only populated for `lmap`.
    let mut collected: Vec<Vec<u8>> = Vec::new();
    for it in 0..iterations {
        for (vars, vals) in &groups {
            for (k, var) in vars.iter().enumerate() {
                let val = vals.get(it * vars.len() + k).cloned().unwrap_or_default();
                let o = new_string(&val);
                if let Err(e) = interp.var_set(var, o) {
                    drop_fresh(o);
                    return crate::builtins::var_error(interp, var, e);
                }
            }
        }
        match interp.eval_control_body(body) {
            Code::Ok => {
                if collect {
                    collected.push(obj_bytes(interp.get_obj_result()));
                }
            }
            Code::Continue => {} // skip collection, keep looping
            Code::Break => break,
            Code::Error => {
                // `("<cmd>" body line N)` — only at top level. Tcl inlines
                // `foreach`/`lmap` when it compiles the enclosing script (a proc
                // body), so no body-frame appears there; an uncompiled top-level
                // form runs its command form, which does add the frame. A
                // tree-walker has no bytecode, so `!in_proc()` approximates the
                // compilation boundary — matches tclsh 9.0 for both cases.
                if !interp.in_proc() {
                    interp.append_body_frame(name);
                }
                return Code::Error;
            }
            other => return other,
        }
    }
    if collect {
        // Rematerialise into a list; `new_list_obj` takes a +1 on each fresh
        // (rc-0) element, so the list owns them — no `drop_fresh` here.
        let objs: Vec<*mut TclObj> = collected.iter().map(|b| new_string(b)).collect();
        interp.set_result(crate::list::new_list_obj(&objs));
    } else {
        interp.set_result_bytes(b"");
    }
    Code::Ok
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
