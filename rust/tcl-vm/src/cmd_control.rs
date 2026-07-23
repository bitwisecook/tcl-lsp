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

//! Control-flow *fallback* commands — `if` / `while` / `for` / `foreach` /
//! `lmap`.
//!
//! The VM compiles these inline for the static, literal forms; this module
//! supplies the **runtime command** forms the codegen falls back to when a
//! construct cannot be inlined — a computed command name (`set z if; $z …`), a
//! dynamic/computed body, or a malformed-grammar barrier. Without them the VM
//! reports `invalid command name "if"` for every such case (the bulk of the
//! `if`/`while` tcltest gap vs. the tree-walking `runtime/rust`).
//!
//! They mirror C's `Tcl_IfObjCmd`/`Tcl_WhileObjCmd`/`Tcl_ForObjCmd` /
//! `EachloopCmd` and `runtime/rust`'s `cmd_control.rs`, evaluating bodies as
//! transparent scripts (`break`/`continue`/`return` propagate) in the current
//! frame via [`Vm::eval_source`], and conditions as Tcl expressions via
//! [`Vm::eval_expr`]. Semantics pinned against tclsh 9.0.

use tcl_runtime_api::{Code, Completion};

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("if", cmd_if);
    vm.register("while", cmd_while);
    vm.register("for", cmd_for);
    vm.register("foreach", cmd_foreach);
    vm.register("lmap", cmd_lmap);
}

/// Evaluate a Tcl expression to a boolean (the `if`/`while`/`for` condition).
/// An expression error — or a non-boolean result — surfaces as an `Err`
/// completion that the caller propagates.
fn cond_bool(vm: &mut Vm, cond: &Value) -> Result<bool, Completion<Value>> {
    match vm.eval_expr(&cond.to_str()) {
        Ok(v) => v.as_bool().map_err(|e| err(e.message)),
        Err(e) => Err(err(e.message)),
    }
}

/// Evaluate a body as a transparent script in the current frame. The full
/// completion is returned so the caller can act on `break`/`continue`/`return`.
///
/// Native-stack safety net — see `interp::CONTROL_FALLBACK_DEPTH_LIMIT`'s
/// doc comment (issue #996). Every runtime-fallback body in this module
/// (`if`/`while`/`for`/`foreach`/`lmap`) funnels through here, so guarding
/// this one entry point covers all of them.
fn eval_body(vm: &mut Vm, body: &Value) -> Completion<Value> {
    if let Err(c) = vm.enter_control_fallback() {
        return c;
    }
    let result = match vm.eval_source(&body.to_str()) {
        Ok(c) => c,
        Err(e) => err(e.message),
    };
    vm.exit_control_fallback();
    result
}

/// `wrong # args: no script following "<token>" argument` (C's `missingScript`).
fn no_script_following(token: &str) -> Completion<Value> {
    err(format!(
        "wrong # args: no script following \"{token}\" argument"
    ))
}

// if

/// `if expr1 ?then? body1 elseif expr2 ?then? body2 ... ?else? ?bodyN?`.
fn cmd_if(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let objc = args.len();
    let mut i = 0;
    // The body of the first true condition, recorded but **not executed** until
    // the whole grammar validates (C's `thenScriptIndex`): a matched branch
    // still rejects malformed trailing clauses, and later conditions are not
    // evaluated (their side effects are skipped).
    let mut then_body: Option<Value> = None;
    // The keyword before the expected expression — for the "no expression" error.
    let mut clause = "if";

    loop {
        if i >= objc {
            return err(format!(
                "wrong # args: no expression after \"{clause}\" argument"
            ));
        }
        let cond = args[i].clone();
        i += 1;
        if i < objc && &*args[i].to_str() == "then" {
            i += 1;
        }
        if i >= objc {
            return no_script_following(&args[i - 1].to_str());
        }
        if then_body.is_none() {
            match cond_bool(vm, &cond) {
                Ok(true) => then_body = Some(args[i].clone()),
                Ok(false) => {}
                Err(c) => return c,
            }
        }
        i += 1; // consume the body
        if i >= objc {
            break; // no further clauses
        }
        if &*args[i].to_str() == "elseif" {
            i += 1;
            clause = "elseif";
            continue;
        }
        break;
    }

    // Past the `elseif` chain: an optional `else` then exactly one body, or a
    // single bare implicit-else body. Anything else is "extra words".
    if i < objc && &*args[i].to_str() == "else" {
        i += 1;
        if i >= objc {
            return no_script_following("else");
        }
    }
    if i + 1 < objc {
        return err("wrong # args: extra words after \"else\" clause in \"if\" command");
    }

    match then_body {
        Some(body) => eval_body(vm, &body),
        None if i < objc => eval_body(vm, &args[i]),
        None => ok(Value::empty()),
    }
}

// while

/// `while test body`.
fn cmd_while(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.len() != 2 {
        return err("wrong # args: should be \"while test command\"");
    }
    let cond = args[0].clone();
    let body = args[1].clone();
    loop {
        if let Some(c) = vm.limit_check_tick() {
            return c;
        }
        match cond_bool(vm, &cond) {
            Ok(true) => {}
            Ok(false) => break,
            Err(c) => return c,
        }
        let c = eval_body(vm, &body);
        match c.code {
            Code::Ok | Code::Continue => {}
            Code::Break => break,
            Code::Error => {
                // The uncompiled `while` adds a `("while" body line N)` frame.
                vm.append_body_frame("while");
                return c;
            }
            _ => return c, // return / Other propagate
        }
    }
    ok(Value::empty())
}

// for

/// `for start test next body`.
fn cmd_for(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.len() != 4 {
        return err("wrong # args: should be \"for start test next command\"");
    }
    let (init, cond, next, body) = (
        args[0].clone(),
        args[1].clone(),
        args[2].clone(),
        args[3].clone(),
    );
    let c = eval_body(vm, &init);
    if c.code != Code::Ok {
        return c;
    }
    loop {
        if let Some(c) = vm.limit_check_tick() {
            return c;
        }
        match cond_bool(vm, &cond) {
            Ok(true) => {}
            Ok(false) => break,
            Err(c) => return c,
        }
        let c = eval_body(vm, &body);
        match c.code {
            Code::Ok | Code::Continue => {} // `continue` still runs `next`
            Code::Break => break,
            Code::Error => {
                vm.append_body_frame("for");
                return c;
            }
            _ => return c,
        }
        let c = eval_body(vm, &next);
        match c.code {
            Code::Ok => {}
            // A `break` in the step clause ends the loop cleanly (C's
            // `Tcl_ForObjCmd`: `case TCL_BREAK: result = TCL_OK`).
            Code::Break => break,
            _ => return c,
        }
    }
    ok(Value::empty())
}

// foreach / lmap

/// `foreach varList list ?varList list ...? body`.
fn cmd_foreach(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    each_loop(vm, args, false)
}

/// `lmap varList list ?varList list ...? body` — `foreach` that collects each
/// (non-`continue`) body result into a list and returns it.
fn cmd_lmap(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    each_loop(vm, args, true)
}

/// Shared `foreach`/`lmap` engine. With `collect`, each successful body result
/// is appended to the result list; without, the result is the empty string.
fn each_loop(vm: &mut Vm, args: &[Value], collect: bool) -> Completion<Value> {
    let name = if collect { "lmap" } else { "foreach" };
    // (name-stripped) N×(varlist, list) pairs + body ⇒ an odd arg count ≥ 3.
    if args.len() < 3 || args.len() % 2 != 1 {
        let usage = if collect {
            "lmap varList list ?varList list ...? command"
        } else {
            "foreach varList list ?varList list ...? command"
        };
        return err(format!("wrong # args: should be \"{usage}\""));
    }
    let body = args[args.len() - 1].clone();

    // Parse each group's variable names + values; track the iteration count.
    let mut groups: Vec<(Vec<String>, Vec<Value>)> = Vec::new();
    let mut iterations = 0usize;
    for pair in args[..args.len() - 1].chunks_exact(2) {
        let vars: Vec<String> = match pair[0].as_list() {
            Ok(v) => v.iter().map(|n| n.to_str().to_string()).collect(),
            Err(e) => return err(e.message),
        };
        if vars.is_empty() {
            return err(format!("{name} varlist is empty"));
        }
        let values = match pair[1].as_list() {
            Ok(v) => (*v).clone(),
            Err(e) => return err(e.message),
        };
        iterations = iterations.max(values.len().div_ceil(vars.len()));
        groups.push((vars, values));
    }

    let mut collected: Vec<Value> = Vec::new();
    for it in 0..iterations {
        for (vars, vals) in &groups {
            for (k, var) in vars.iter().enumerate() {
                let val = vals
                    .get(it * vars.len() + k)
                    .cloned()
                    .unwrap_or_else(Value::empty);
                if let Err(e) = vm.set_var(var, val) {
                    return e;
                }
            }
        }
        let c = eval_body(vm, &body);
        match c.code {
            Code::Ok => {
                if collect {
                    collected.push(c.result.clone());
                }
            }
            Code::Continue => {} // skip collection, keep looping
            Code::Break => break,
            Code::Error => {
                vm.append_body_frame(name);
                return c;
            }
            _ => return c, // return / Other propagate
        }
    }
    if collect {
        ok(Value::list(collected))
    } else {
        ok(Value::empty())
    }
}
