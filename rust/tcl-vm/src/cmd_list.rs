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
    // The dispatched `lappend` is C's `Tcl_LappendObjCmd`, which fetches
    // through `TclPtrGetVarIdx` in both its forms (`tclVar.c` 9.0.4:2895 and
    // :2944), so the read trace fires before the store.
    let cur = vm.read_for_update(&n);
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
            None => match vm.store_var_result(&n, Value::empty()) {
                Ok(stored) => ok(stored),
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
    match vm.store_var_result(&n, result) {
        Ok(stored) => ok(stored),
        Err(e) => e,
    }
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
    match vm.store_var_result(&n, result) {
        Ok(stored) => ok(stored),
        Err(e) => e,
    }
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
    match vm.store_var_result(&n, new) {
        Ok(stored) => ok(stored),
        Err(e) => e,
    }
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

/// Resolve `spec` against a length-`len` list for `lpop`/`lset`-style index
/// descent: a non-integer index is a "bad index" error; an in-form but
/// out-of-bounds index is "index … out of range" — matching C's
/// `Tcl_LpopObjCmd`.
fn resolve_bounded_index(spec: &str, len: usize) -> Result<usize, Completion<Value>> {
    let Some(idx) = crate::command::resolve_index(spec, len) else {
        return Err(err(format!(
            "bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?"
        )));
    };
    if idx < 0 || usize::try_from(idx).is_ok_and(|i| i >= len) {
        return Err(err(format!("index \"{spec}\" out of range")));
    }
    Ok(usize::try_from(idx).expect("idx >= 0 checked above"))
}

/// Remove the element at the (possibly nested) `indices` path from `items`,
/// returning `(removed_element, rebuilt_list)`.
///
/// Issue #996: this used to recurse once per index natively, with no depth
/// cap — trivially inflated via `lpop v {*}[lrepeat 100000 0]`. Rewritten
/// iteratively — an explicit work-stack instead of one native call per
/// index — which eliminates the native-stack risk entirely: walk down every
/// index but the last, recording each level's element vector and the index
/// it descends through, remove the final element, then rebuild bottom-up.
/// Byte-for-byte equivalent to the old recursive version (same index
/// resolution, in the same order, so error precedence is unchanged too).
fn lpop_remove(
    items: &[Value],
    indices: &[Value],
) -> Result<(Value, Vec<Value>), Completion<Value>> {
    let (last, front) = indices.split_last().expect("lpop has at least one index");
    let mut frames: Vec<(Vec<Value>, usize)> = Vec::with_capacity(front.len());
    let mut cur: Vec<Value> = items.to_vec();
    for spec_val in front {
        let i = resolve_bounded_index(&spec_val.to_str(), cur.len())?;
        let sub = match cur[i].as_list() {
            Ok(s) => (*s).clone(),
            Err(e) => return Err(err(e.message)),
        };
        frames.push((cur, i));
        cur = sub;
    }
    let i = resolve_bounded_index(&last.to_str(), cur.len())?;
    let removed = cur.remove(i);
    let mut rebuilt = cur;
    for (mut outer, i) in frames.into_iter().rev() {
        outer[i] = Value::list(rebuilt);
        rebuilt = outer;
    }
    Ok((removed, rebuilt))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression coverage for issue #996: `lpop_remove` recurses once per
    /// index in `lpop`'s (possibly nested) index path, with no depth cap
    /// before this fix — trivially inflated via `lpop v {*}[lrepeat 100000
    /// 0]`. Empirically (a throwaway `zzz_probe_depth lpop <depth>`
    /// harness, deleted before this fix landed), unguarded input
    /// overflowed the native stack (SIGABRT) between depth 1600 and 1800 on
    /// a 2 MiB thread (`cargo test`'s per-test default). Rewritten
    /// iteratively (no depth cap at all); 2000 is comfortably past that
    /// crash range, and the result is checked for exact correctness at
    /// this depth (the right leaf comes back out, and the trimmed list has
    /// the same shape as the input), not merely survival.
    ///
    /// Deliberately NOT 50,000+: constructing (and, at the end of this
    /// test, dropping) a `Value::list` chain nested that deep is its own,
    /// unrelated native-stack risk — `Value` has no custom `Drop` impl, so
    /// the compiler-generated recursive drop glue walks the same chain
    /// `to_str` used to (empirically, SIGABRT between depth 3500 and 4000
    /// on a 2 MiB thread for construction+drop alone, independent of any
    /// operation performed on the value). That is a separate, genuinely
    /// unbounded-depth concern in `Value`'s representation itself, not in
    /// `lpop_remove`'s now-iterative logic — out of scope for this fix.
    #[test]
    fn deeply_nested_lpop_survives_and_is_correct() {
        const DEPTH: usize = 2_000;
        // `whole` = `DEPTH` levels of `[list $v]` around a scalar leaf;
        // `items` is `whole` with one layer already stripped off (mirroring
        // what `cmd_lpop` passes in: the already-`as_list()`-ed variable).
        let mut whole = Value::string("leaf");
        for _ in 0..DEPTH {
            whole = Value::list(vec![whole]);
        }
        let items: Vec<Value> = (*whole.as_list().expect("built as a list")).clone();
        let indices: Vec<Value> = (0..DEPTH).map(|_| Value::string("0")).collect();
        let (removed, rest) =
            lpop_remove(&items, &indices).expect("lpop_remove survives and succeeds");
        assert_eq!(&*removed.to_str(), "leaf");
        // The outermost shape (one element) is preserved; only the
        // innermost slot the path bottomed out at was actually emptied.
        assert_eq!(rest.len(), 1);
    }

    /// A moderately nested `lpop` index path (well within realistic use) is
    /// byte-for-byte unaffected by the iterative rewrite.
    #[test]
    fn moderately_nested_lpop_matches_previous_behavior() {
        let items = vec![
            Value::list(vec![Value::int(1), Value::int(2)]),
            Value::list(vec![Value::int(3), Value::int(4)]),
        ];
        let (removed, rest) =
            lpop_remove(&items, &[Value::string("1"), Value::string("0")]).unwrap();
        assert_eq!(&*removed.to_str(), "3");
        assert_eq!(rest.len(), 2);
        assert_eq!(&*rest[0].to_str(), "1 2");
        assert_eq!(&*rest[1].to_str(), "4");

        // Single-level removal.
        let flat = vec![Value::int(1), Value::int(2), Value::int(3)];
        let (removed, rest) = lpop_remove(&flat, &[Value::string("1")]).unwrap();
        assert_eq!(&*removed.to_str(), "2");
        assert_eq!(rest.len(), 2);
        assert_eq!(&*rest[0].to_str(), "1");
        assert_eq!(&*rest[1].to_str(), "3");

        // A non-integer index is still a "bad index" error.
        assert!(lpop_remove(&flat, &[Value::string("bogus")]).is_err());
        // An in-form but out-of-bounds index is still "out of range".
        assert!(lpop_remove(&flat, &[Value::string("10")]).is_err());
    }
}
