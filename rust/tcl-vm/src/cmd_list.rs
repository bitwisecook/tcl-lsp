//! List builtins, reusing `tcl_syntax::list` for split/merge semantics.

use std::cmp::Ordering;
use std::rc::Rc;

use tcl_cmd_core::list as list_core;
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
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
        _ => err("wrong # args: should be \"llength list\""),
    }
}

fn cmd_lindex(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((list, idxs)) = args.split_first() else {
        return err("wrong # args: should be \"lindex list ?index ...?\"");
    };
    adapt(list_core::lindex(vm, list, idxs))
}

fn cmd_lrange(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [list, from, to] = args else {
        return err("wrong # args: should be \"lrange list first last\"");
    };
    adapt(list_core::lrange(vm, list, from, to))
}

fn cmd_lappend(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((name, vals)) = args.split_first() else {
        return err("wrong # args: should be \"lappend varName ?value ...?\"");
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
        return err("wrong # args: should be \"lassign list ?varName ...?\"");
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
        _ => err("wrong # args: should be \"lreverse list\""),
    }
}

fn cmd_lrepeat(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((count, elems)) = args.split_first() else {
        return err("wrong # args: should be \"lrepeat count ?value ...?\"");
    };
    adapt(list_core::lrepeat(vm, count, elems))
}

fn cmd_linsert(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [list, index, elems @ ..] = args else {
        return err("wrong # args: should be \"linsert list index ?element ...?\"");
    };
    adapt(list_core::linsert(vm, list, index, elems))
}

fn cmd_lreplace(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [list, from, to, rest @ ..] = args else {
        return err("wrong # args: should be \"lreplace list first last ?element ...?\"");
    };
    adapt(list_core::lreplace(vm, list, from, to, rest))
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

fn cmd_lsort(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    use tcl_cmd_core::sort::SortMode;
    let mut mode = SortMode::Ascii;
    let mut nocase = false;
    let mut decreasing = false;
    let mut unique = false;
    let mut rest = args;
    while let Some(first) = rest.first() {
        match &*first.to_str() {
            "-ascii" => mode = SortMode::Ascii,
            "-dictionary" => mode = SortMode::Dictionary,
            "-integer" => mode = SortMode::Integer,
            "-real" => mode = SortMode::Real,
            "-nocase" => nocase = true,
            "-decreasing" => decreasing = true,
            "-increasing" => decreasing = false,
            "-unique" => unique = true,
            // `-index`/`-command`/`-stride` aren't supported here yet (skipped).
            s if s.starts_with('-') => {}
            _ => break,
        }
        rest = &rest[1..];
    }
    let [list] = rest else {
        return err("wrong # args: should be \"lsort ?options? list\"");
    };
    let mut items = match as_list(list) {
        Ok(i) => (*i).clone(),
        Err(c) => return c,
    };
    // The comparison modes (incl. the subtle dictionary order, and the
    // integer-vs-real distinction the VM previously collapsed) are the shared
    // `tcl_cmd_core::sort` core.
    let cmp = |a: &Value, b: &Value| {
        tcl_cmd_core::sort::key_compare(mode, nocase, a.to_str().as_bytes(), b.to_str().as_bytes())
    };
    items.sort_by(|a, b| {
        let ord = cmp(a, b);
        if decreasing { ord.reverse() } else { ord }
    });
    if unique {
        // `-unique` removes elements equal *under the sort mode* (e.g. `1`/`01`
        // for `-integer`), not just byte-equal ones.
        items.dedup_by(|a, b| cmp(a, b) == Ordering::Equal);
    }
    ok(Value::list(items))
}

fn cmd_concat(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    ok(list_core::concat(vm, args))
}

fn cmd_join(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [l] => adapt(list_core::join(vm, l, None)),
        [l, s] => adapt(list_core::join(vm, l, Some(s))),
        _ => err("wrong # args: should be \"join list ?joinString?\""),
    }
}

fn cmd_split(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [s] => ok(list_core::split(vm, s, None)),
        [s, c] => ok(list_core::split(vm, s, Some(c))),
        _ => err("wrong # args: should be \"split string ?splitChars?\""),
    }
}
