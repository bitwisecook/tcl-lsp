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

//! The `array` ensemble builtin — a thin adapter over the shared
//! [`tcl_cmd_core::array`] core. The read-side (`exists`/`size`/`names`/`get`) and
//! `unset` are shared over the VM's `VarStore`/`Frames`/`ValueOps`; `set` (whose
//! per-element write traces must fail the command) stays here. Sharing fixed the
//! VM's `array unset a` (no pattern), which used to iterate-and-unset elements
//! (leaving an empty array) instead of removing the whole array.

use tcl_runtime_api::{Code, Completion};

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("array", cmd_array);
    // Ensemble member commands the codegen rewrites `array <sub>` into.
    vm.register("::tcl::array::exists", |vm, a| array_op(vm, "exists", a));
    vm.register("::tcl::array::names", |vm, a| array_op(vm, "names", a));
    vm.register("::tcl::array::get", |vm, a| array_op(vm, "get", a));
    vm.register("::tcl::array::set", |vm, a| array_op(vm, "set", a));
    vm.register("::tcl::array::size", |vm, a| array_op(vm, "size", a));
    vm.register("::tcl::array::unset", |vm, a| array_op(vm, "unset", a));
    vm.register("::tcl::array::for", |vm, a| array_op(vm, "for", a));
}

/// `array option arrayName ?arg ...?` — dispatch to the subcommand handler.
fn cmd_array(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"array subcommand ?arg ...?\"");
    };
    array_op(vm, &sub.to_str(), rest)
}

fn array_op(vm: &mut Vm, sub: &str, rest: &[Value]) -> Completion<Value> {
    // The read-side + `unset` live in the shared core.
    if let Some(result) = tcl_cmd_core::array::dispatch(vm, sub, rest) {
        return match result {
            Ok(v) => ok(v),
            Err(e) => err(e.into_message()),
        };
    }
    // Per-runtime: `array set` (its per-element write traces must fail the
    // command), `array for` (iterates a body), and the unknown-subcommand message.
    match sub {
        "for" => array_for(vm, rest),
        "set" => match rest {
            [n, list] => {
                // C's `Tcl_ArrayObjCmd` set path resolves the target through
                // the standard variable lookup *before* it looks at the list,
                // and that lookup parses the name: an element-form name yields
                // a scalar element cell, never an array, so the command refuses
                // it (issue #1578). The check therefore precedes both the list
                // parse and the even-length test.
                //
                // Oracle, identical on tclsh 8.4.20 / 8.5.19 / 8.6.14 / 9.0.4 /
                // 9.1 (`catch` result : message):
                //
                //   array set (x) {a 1}     -> 1:can't set "(x)": variable isn't array
                //   array set (x) {}        -> 1:can't set "(x)": variable isn't array
                //   array set (x) {a}       -> 1:can't set "(x)": variable isn't array
                //   array set (x) "a \{b"   -> 1:can't set "(x)": variable isn't array
                //   array set arr(k) {a 1}  -> 1:can't set "arr(k)": variable isn't array
                //   array set {arr(k)} {a 1}-> 1:can't set "arr(k)": variable isn't array
                //   array set okarr {a}     -> 1:list must have an even number of elements
                //   array set {a)b} {a 1}   -> 0:            (a `)` with no `(` is a name)
                //   array set {a(b} {a 1}   -> 0:            (an unclosed `(` is a name)
                //
                // 8.6+ also carry `errorCode` `TCL LOOKUP VARNAME <name>` (8.4 /
                // 8.5: `NONE`); the VM's shared `TCL LOOKUP VARNAME` spelling
                // omits the trailing name element here as it does at its
                // sibling site (`missing_parent_ns`).
                let name = n.to_str();
                if tcl_syntax::naming::split_element_ref(&name).is_some() {
                    return crate::command::err_with_code(
                        format!("can't set \"{name}\": variable isn't array"),
                        "TCL LOOKUP VARNAME",
                    );
                }
                let items = match list.as_list() {
                    Ok(i) => i,
                    Err(e) => return err(e.message),
                };
                if items.len() % 2 != 0 {
                    return err("list must have an even number of elements");
                }
                if items.is_empty() {
                    // `array set a {}` still materialises an empty array; onto an
                    // existing scalar it errors. C words *this* case as the
                    // command (`can't array set "a"`), distinct from the
                    // per-element `set` message taken on a non-empty list.
                    if let Err(e) = vm.ensure_array(&name) {
                        return e;
                    }
                } else {
                    // Write element by element so a scalar target fails at the
                    // *element* write — naming `a(key)`, as C's `TclArraySet`
                    // does — rather than pre-checked under the bare name.
                    let mut i = 0;
                    while i + 1 < items.len() {
                        if let Err(e) =
                            vm.set_array_elem(&name, &items[i].to_str(), items[i + 1].clone())
                        {
                            return e;
                        }
                        i += 2;
                    }
                }
                ok(Value::empty())
            }
            _ => err("wrong # args: should be \"array set arrayName list\""),
        },
        other => err(format!(
            "unknown or ambiguous subcommand \"{other}\": must be exists, for, get, names, set, size, or unset"
        )),
    }
}

/// `array for {keyVar valueVar} arrayName script` — iterate the array's elements,
/// binding the two vars and running the body once per pair (mirrors `dict for`;
/// `break`/`continue` apply, an error/return propagates). The element set is
/// snapshotted up front so body mutations don't perturb the walk.
fn array_for(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [vars, arrname, body] = rest else {
        return err("wrong # args: should be \"array for {key value} arrayName script\"");
    };
    let vnames = match vars.as_list() {
        Ok(v) => v,
        Err(e) => return err(e.message),
    };
    let [kvar, vvar] = vnames.as_slice() else {
        return err("must have two variable names");
    };
    let kvar = kvar.to_str().to_string();
    let vvar = vvar.to_str().to_string();
    let name = arrname.to_str();
    if !vm.array_is(&name) {
        return err(format!("\"{name}\" isn't an array"));
    }
    let body_src = body.to_str();
    // Snapshot the key set, but read each value live: C's enumeration walks the
    // hash by key and reports the element's *current* value, so a body that
    // rewrites an as-yet-unvisited element is observed (var-23.12). Adding or
    // removing an element, by contrast, perturbs the hash and C aborts the walk
    // with "array changed during iteration" (var-23.10 / var-23.11) — detected
    // here by the key set diverging from the snapshot.
    let keys: Vec<String> = vm.array_pairs(&name).into_iter().map(|(k, _)| k).collect();
    let orig: std::collections::BTreeSet<&str> = keys.iter().map(String::as_str).collect();
    for k in &keys {
        let Some(v) = vm.get_array_elem(&name, k) else {
            continue;
        };
        if let Err(e) = vm.set_var(&kvar, Value::string(k.clone())) {
            return e;
        }
        if let Err(e) = vm.set_var(&vvar, v) {
            return e;
        }
        match vm.eval_source(&body_src) {
            Ok(c) => match c.code {
                Code::Ok | Code::Continue => {}
                Code::Break => break,
                _ => return c,
            },
            Err(e) => return err(e.message),
        }
        // The body may have added/removed elements (a structural change), which
        // invalidates the enumeration — abort as C does.
        let cur: std::collections::BTreeSet<String> =
            vm.array_pairs(&name).into_iter().map(|(k, _)| k).collect();
        if cur.len() != orig.len() || !cur.iter().all(|k| orig.contains(k.as_str())) {
            return err("array changed during iteration");
        }
    }
    ok(Value::empty())
}
