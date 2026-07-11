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

//! List builtins, reusing `tcl_syntax::list` for split/merge semantics.

use std::rc::Rc;

use tcl_cmd_core::list as list_core;
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, err_wrong_args, ok};
use crate::value::Value;

/// Map a portable `tcl-cmd-core` result onto the VM's `Completion`.
fn adapt(result: Result<Value, tcl_cmd_core::CmdError>) -> Completion<Value> {
    match result {
        Ok(v) => ok(v),
        Err(e) => err(e.into_message()),
    }
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register("list", cmd_list);
    vm.register("llength", cmd_llength);
    vm.register("lindex", cmd_lindex);
    vm.register("lrange", cmd_lrange);
    vm.register("lappend", cmd_lappend);
    vm.register("lassign", cmd_lassign);
    vm.register("lreverse", cmd_lreverse);
    vm.register("lrepeat", cmd_lrepeat);
    vm.register("linsert", cmd_linsert);
    vm.register("lreplace", cmd_lreplace);
    vm.register("ledit", cmd_ledit);
    vm.register("lset", cmd_lset);
    vm.register("lpop", cmd_lpop);
    vm.register("lsearch", cmd_lsearch);
    vm.register("lsort", cmd_lsort);
    vm.register("concat", cmd_concat);
    vm.register("join", cmd_join);
    vm.register("split", cmd_split);
}

fn as_list(v: &Value) -> Result<Rc<Vec<Value>>, Completion<Value>> {
    v.as_list().map_err(|e| err(e.message))
}

fn cmd_list(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    ok(list_core::list(vm, args))
}

fn cmd_llength(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [l] => adapt(list_core::llength(vm, l)),
        _ => err_wrong_args("llength list"),
    }
}

fn cmd_lindex(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((list, idxs)) = args.split_first() else {
        return err_wrong_args("lindex list ?index ...?");
    };
    adapt(list_core::lindex(vm, list, idxs))
}

fn cmd_lrange(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [list, from, to] = args else {
        return err_wrong_args("lrange list first last");
    };
    adapt(list_core::lrange(vm, list, from, to))
}

fn cmd_lappend(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((name, vals)) = args.split_first() else {
        return err_wrong_args("lappend varName ?value ...?");
    };
    let n = name.to_str();
    let cur = vm.var_get(&n);
    if vals.is_empty() {
        // `lappend var` with no values returns the variable's current value
        // *unchanged*: Tcl shimmer-validates it as a list (so a malformed value
        // errors) but never re-renders the string representation — re-rendering
        // would canonically requote elements (a leading `#` → `{#}`), diverging
        // from C Tcl. An unset variable is created as the empty string.
        return match cur {
            Some(v) => match as_list(&v) {
                Ok(_) => ok(v),
                Err(c) => c,
            },
            None => match vm.var_set(&n, Value::empty()) {
                Ok(()) => ok(Value::empty()),
                Err(e) => e,
            },
        };
    }
    // The COW-aware list append (rebuild on the VM; in place on the WASM runtime)
    // is shared via `tcl_cmd_core::var::lappend_value`; the single store fires the
    // write trace once.
    let result = match tcl_cmd_core::var::lappend_value(vm, cur, vals) {
        Ok(v) => v,
        Err(e) => return err(e.message()),
    };
    if let Err(e) = vm.var_set(&n, result.clone()) {
        return e;
    }
    ok(result)
}

fn cmd_lassign(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((list, names)) = args.split_first() else {
        return err_wrong_args("lassign list ?varName ...?");
    };
    let items = match as_list(list) {
        Ok(i) => i,
        Err(c) => return c,
    };
    for (i, name) in names.iter().enumerate() {
        let v = items.get(i).cloned().unwrap_or_else(Value::empty);
        if let Err(e) = vm.set_var(&name.to_str(), v) {
            return e;
        }
    }
    // Return the unassigned remainder.
    let rest = if names.len() < items.len() {
        items[names.len()..].to_vec()
    } else {
        Vec::new()
    };
    ok(Value::list(rest))
}

fn cmd_lreverse(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [l] => adapt(list_core::lreverse(vm, l)),
        _ => err_wrong_args("lreverse list"),
    }
}

fn cmd_lrepeat(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((count, elems)) = args.split_first() else {
        return err_wrong_args("lrepeat count ?value ...?");
    };
    adapt(list_core::lrepeat(vm, count, elems))
}

fn cmd_linsert(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [list, index, elems @ ..] = args else {
        return err_wrong_args("linsert list index ?element ...?");
    };
    adapt(list_core::linsert(vm, list, index, elems))
}

fn cmd_lreplace(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [list, from, to, rest @ ..] = args else {
        return err_wrong_args("lreplace list first last ?element ...?");
    };
    adapt(list_core::lreplace(vm, list, from, to, rest))
}

/// `ledit listVar first last ?element ...?` — the in-place `lreplace` (Tcl 9):
/// replace the `first..last` range of the list held in `listVar` with the given
/// elements, store the result back into the variable, and return it. The write
/// goes through `var_set`, so it fires the variable's write traces.
fn cmd_ledit(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name, from, to, rest @ ..] = args else {
        return err_wrong_args("ledit listVar first last ?element ...?");
    };
    let n = name.to_str();
    let Some(cur) = vm.var_get(&n) else {
        return err(vm.read_miss_msg(&n));
    };
    let result = match list_core::lreplace(vm, &cur, from, to, rest) {
        Ok(v) => v,
        Err(e) => return err(e.into_message()),
    };
    if let Err(e) = vm.var_set(&n, result.clone()) {
        return e;
    }
    ok(result)
}

/// `lset listVar ?index ...? newValue` — the runtime form of `lset` (the
/// compiler inlines the common compiled cases via `LSET_LIST`/`LSET_FLAT`; this
/// builtin is the fallback for the dynamic / wrong-arg / non-proc paths). It
/// always reads the variable first (so a no-index `lset x v` on an undefined
/// `x` still reports `can't read`), then descends the index path: a single
/// index argument is itself an index *list*, several arguments are a flat path.
fn cmd_lset(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.len() < 2 {
        return err_wrong_args("lset listVar ?index? ?index ...? value");
    }
    let n = args[0].to_str();
    let value = args.last().expect("args.len() >= 2");
    let indices = &args[1..args.len() - 1];
    let Some(cur) = vm.var_get(&n) else {
        return err(vm.read_miss_msg(&n));
    };
    let path: Vec<Value> = if indices.is_empty() {
        Vec::new()
    } else if let [single] = indices {
        match single.as_list() {
            Ok(p) => (*p).clone(),
            Err(e) => return err(e.message),
        }
    } else {
        indices.to_vec()
    };
    let new = match crate::exec::lset_descend(&cur, &path, value.clone()) {
        Ok(r) => r,
        Err(c) => return c,
    };
    if let Err(e) = vm.var_set(&n, new.clone()) {
        return e;
    }
    ok(new)
}

/// `lpop listVar ?index ...?` — remove and return an element of the list held
/// in `listVar` (Tcl 9), defaulting to the last element. With several indices it
/// descends into nested sublists and removes the deepest element. The trimmed
/// list is stored back (firing write traces).
fn cmd_lpop(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((name, indices)) = args.split_first() else {
        return err_wrong_args("lpop listvar ?index?");
    };
    let n = name.to_str();
    let Some(cur) = vm.var_get(&n) else {
        return err(vm.read_miss_msg(&n));
    };
    let items = match as_list(&cur) {
        Ok(i) => (*i).clone(),
        Err(c) => return c,
    };
    // No index means the last element (`end`).
    let default_end = [Value::string("end")];
    let path: &[Value] = if indices.is_empty() {
        &default_end
    } else {
        indices
    };
    let (removed, new_items) = match lpop_remove(&items, path) {
        Ok(r) => r,
        Err(c) => return c,
    };
    if let Err(e) = vm.var_set(&n, Value::list(new_items)) {
        return e;
    }
    ok(removed)
}

/// Remove the element at the (possibly nested) `indices` path from `items`,
/// returning `(removed_element, rebuilt_list)`. A non-integer index is a "bad
/// index" error; an in-form but out-of-bounds index is "index … out of range"
/// — matching C's `Tcl_LpopObjCmd`.
fn lpop_remove(
    items: &[Value],
    indices: &[Value],
) -> Result<(Value, Vec<Value>), Completion<Value>> {
    let (first, rest) = indices.split_first().expect("lpop has at least one index");
    let spec = first.to_str();
    let Some(idx) = crate::command::resolve_index(&spec, items.len()) else {
        return Err(crate::command::bad_index(&spec));
    };
    if idx < 0 || usize::try_from(idx).is_ok_and(|i| i >= items.len()) {
        return Err(err(format!("index \"{spec}\" out of range")));
    }
    let i = usize::try_from(idx).expect("idx >= 0 checked above");
    let mut new = items.to_vec();
    if rest.is_empty() {
        let removed = new.remove(i);
        Ok((removed, new))
    } else {
        let sub = match items[i].as_list() {
            Ok(s) => (*s).clone(),
            Err(e) => return Err(err(e.message)),
        };
        let (removed, new_sub) = lpop_remove(&sub, rest)?;
        new[i] = Value::list(new_sub);
        Ok((removed, new))
    }
}

/// `lsearch ?-option value ...? list pattern` — a thin adapter over the shared
/// [`tcl_cmd_core::lsearch`] core, driven by the VM's `regex`-crate engine for
/// `-regexp`. The VM previously had only a `-exact`/`-glob` stub; it now has the
/// full option set (the Tcl errorCodes the core carries are dropped — the VM has
/// no errorCode surface).
fn cmd_lsearch(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match tcl_cmd_core::lsearch::lsearch::<Vm, crate::cmd_regexp::CrateEngine>(vm, args) {
        Ok(v) => ok(v),
        Err(e) => err(String::from_utf8_lossy(&e.message).into_owned()),
    }
}

/// `lsort ?-option value ...? list` — a thin adapter over the shared
/// [`tcl_cmd_core::lsort`] core. The VM previously had only the comparison modes
/// over a flat option set; it now has `-index`/`-stride`/`-indices` and
/// `-command` (the comparator evaluates Tcl through `vm.dispatch`).
fn cmd_lsort(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    use tcl_cmd_core::lsort::{Lsort, build_command, prepare, sort_command};
    let mut job = match prepare(vm, args) {
        Ok(Lsort::Done(v)) => return ok(v),
        Ok(Lsort::Command(job)) => job,
        Err(e) => return err(String::from_utf8_lossy(&e.message).into_owned()),
    };
    // `-command`: split the comparison prefix into words, run the reentrant merge
    // sort over the VM comparator (no `ValueOps` borrow during the eval), build.
    let prefix = match job.cmd_prefix.as_list() {
        Ok(w) => w,
        Err(e) => return err(e.message),
    };
    if let Err(c) = sort_command(&mut job, |a, b| vm_compare(vm, &prefix, a, b)) {
        return c;
    }
    ok(build_command(vm, &job))
}

/// The `lsort -command` comparator: invoke `<prefix words...> a b` and read its
/// integer result as a sign. Uses `vm.dispatch` (argv-based — no re-parsing, so
/// elements containing `$`/`[` are passed literally).
fn vm_compare(
    vm: &mut Vm,
    prefix: &[Value],
    a: &Value,
    b: &Value,
) -> Result<i32, Completion<Value>> {
    use tcl_runtime_api::Commands;
    let Some((name, pre_args)) = prefix.split_first() else {
        return Err(err("-command comparison command is empty"));
    };
    let mut argv: Vec<Value> = pre_args.to_vec();
    argv.push(a.clone());
    argv.push(b.clone());
    let comp = vm.dispatch(&name.to_str(), &argv);
    if !comp.code.is_ok() {
        return Err(comp);
    }
    let r = comp.result.to_str();
    match tcl_cmd_core::sort::parse_wide(r.as_bytes()) {
        Some(v) => Ok(i32::try_from(v.signum()).unwrap_or(0)),
        None => Err(err(format!(
            "-command comparison script returned non-integer result: {r}"
        ))),
    }
}

fn cmd_concat(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    ok(list_core::concat(vm, args))
}

fn cmd_join(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [l] => adapt(list_core::join(vm, l, None)),
        [l, s] => adapt(list_core::join(vm, l, Some(s))),
        _ => err_wrong_args("join list ?joinString?"),
    }
}

fn cmd_split(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [s] => ok(list_core::split(vm, s, None)),
        [s, c] => ok(list_core::split(vm, s, Some(c))),
        _ => err_wrong_args("split string ?splitChars?"),
    }
}
