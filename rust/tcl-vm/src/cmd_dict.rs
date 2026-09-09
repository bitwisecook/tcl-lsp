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

//! The `dict` ensemble over the VM's typed dictionary representation.

use tcl_runtime_api::completion_options::ControlOptionPolicy;
use tcl_runtime_api::{Code, Completion};

use crate::command::{BuiltinFn, settle_control_options};
use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("dict", cmd_dict);
    // Ensemble member commands the codegen rewrites `dict <sub>` into.
    let registry = crate::environment::universal_store();
    let mut member = |name: &'static str, handler: BuiltinFn| {
        let spec = registry.get(name).expect("qualified dict member spec");
        vm.register_spec_builtin(spec, handler);
    };
    member("::tcl::dict::create", |vm, a| member_op(vm, "create", a));
    member("::tcl::dict::get", |vm, a| member_op(vm, "get", a));
    member("::tcl::dict::exists", |vm, a| member_op(vm, "exists", a));
    member("::tcl::dict::keys", |vm, a| member_op(vm, "keys", a));
    member("::tcl::dict::values", |vm, a| member_op(vm, "values", a));
    member("::tcl::dict::size", |vm, a| member_op(vm, "size", a));
    member("::tcl::dict::merge", |vm, a| member_op(vm, "merge", a));
    member("::tcl::dict::set", |vm, a| member_op(vm, "set", a));
    member("::tcl::dict::unset", |vm, a| member_op(vm, "unset", a));
    member("::tcl::dict::for", |vm, a| member_op(vm, "for", a));
    member("::tcl::dict::map", |vm, a| member_op(vm, "map", a));
    member("::tcl::dict::incr", |vm, a| member_op(vm, "incr", a));
    member("::tcl::dict::info", |vm, a| member_op(vm, "info", a));
    member("::tcl::dict::append", |vm, a| member_op(vm, "append", a));
    member("::tcl::dict::lappend", |vm, a| member_op(vm, "lappend", a));
}

/// `dict`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it. The
/// registry filters this full implemented table for the selected Tcl release.
const DICT_SUBS: &[&str] = &[
    "append",
    "create",
    "exists",
    "filter",
    "for",
    "get",
    "getdef",
    "getwithdefault",
    "incr",
    "info",
    "keys",
    "lappend",
    "map",
    "merge",
    "remove",
    "replace",
    "set",
    "size",
    "unset",
    "update",
    "values",
    "with",
];

type StringPairs = Vec<(String, Value)>;
type DictPathFrame = (StringPairs, String, Option<usize>);

/// `dict subcommand ?arg ...?` — dispatch to the subcommand handler.
fn cmd_dict(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"dict subcommand ?arg ...?\"");
    };
    let word = sub.to_str();
    // `dict` is a `TclMakeEnsemble` command: exact match, else a unique
    // prefix, so `dict k` is `dict keys`.
    // `getdef`/`getwithdefault` are Tcl 9 (TIP 342): under an 8.6 pin they
    // must not resolve, and — the reason this matters for words that have
    // nothing to do with them — must not make `dict g` ambiguous either.
    let subs = crate::environment::release_subcommands(
        vm.runtime_version().dialect_profile_name(),
        "dict",
        DICT_SUBS,
    );
    let Some(index) = tcl_cmd_core::ensemble::resolve_subcommand(subs, word.as_bytes(), true)
    else {
        return err(
            String::from_utf8_lossy(&tcl_cmd_core::ensemble::unknown_subcommand_message(
                subs,
                word.as_bytes(),
                true,
                b"::tcl::dict",
            ))
            .into_owned(),
        );
    };
    let invoked = vm.invoked_name().unwrap_or("dict").to_owned();
    let usage_prefix = format!("{invoked} {}", subs[index]);
    dict_op(vm, subs[index], rest, &usage_prefix)
}

/// Dispatch a separately invocable ensemble implementation command. Its arity
/// text uses the actual command spelling, including a name installed by
/// `rename`, rather than reconstructing `dict <subcommand>`.
fn member_op(vm: &mut Vm, sub: &str, args: &[Value]) -> Completion<Value> {
    let invoked = vm
        .invoked_name()
        .map_or_else(|| format!("::tcl::dict::{sub}"), str::to_owned);
    dict_op(vm, sub, args, &invoked)
}

/// The dict's **canonical** ordered `(key-string, value)` pairs, from the one
/// [`ValueOps::dict_pairs`](tcl_syntax::value::ValueOps::dict_pairs) owner:
/// first-occurrence position, **last value winning** on a duplicate key
/// (`SetDictFromAny`, tclDictObj.c(9.0.4):589 → `Tcl_DictObjPut`). Decoding the
/// list rep straight into `chunks_exact(2)` pairs instead leaves both values of
/// a duplicate key present, so every [`lookup`] reads the *first* (issue #1427).
pub(crate) fn pairs(vm: &mut Vm, v: &Value) -> Result<StringPairs, Completion<Value>> {
    vm.dict_pairs(v)
}

fn from_pairs(ps: &[(String, Value)]) -> Value {
    from_pairs_with_hash_bucket_count(ps, None)
}

fn from_pairs_with_hash_bucket_count(ps: &[(String, Value)], bucket_count: Option<usize>) -> Value {
    let mut v = Vec::with_capacity(ps.len());
    for (k, val) in ps {
        v.push((Value::string(k.as_str()), val.clone()));
    }
    Value::dict_with_hash_bucket_count(v, bucket_count)
}

fn mutation_hash_state(
    v: &Value,
    owned_references: usize,
    parent_was_copied: bool,
) -> Result<(Option<usize>, bool), Completion<Value>> {
    v.dict_mutation_hash_state(owned_references, parent_was_copied)
        .map_err(|e| crate::exec::dict_parse_err(&e.message))
}

fn lookup<'a>(ps: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    ps.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Read the dict in variable `varname`, transform the value at `key` via `f`
/// (given the current value, if any), write it back, and return the new dict.
/// Shared by `dict incr`/`append`/`lappend`.
fn dict_update(
    vm: &mut Vm,
    varname: &Value,
    key: &Value,
    f: impl FnOnce(Option<&Value>) -> Result<Value, String>,
) -> Completion<Value> {
    let name = varname.to_str();
    let cur = vm.get_var(&name).unwrap_or_else(Value::empty);
    let (bucket_count, _) = match mutation_hash_state(&cur, 2, false) {
        Ok(state) => state,
        Err(c) => return c,
    };
    let mut ps = match pairs(vm, &cur) {
        Ok(pairs) => pairs,
        Err(c) => return c,
    };
    let k = key.to_str();
    let newv = match f(lookup(&ps, &k)) {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    upsert(&mut ps, &k, newv);
    let result = from_pairs_with_hash_bucket_count(&ps, bucket_count);
    if let Err(e) = vm.set_var(&name, result.clone()) {
        return e;
    }
    ok(result)
}

/// Set the nested `keys` path of dict `cur` to `value`, creating intermediate
/// dicts as needed (`dict set` with multiple keys).
///
/// Issue #996: `dict set d {*}[lrepeat 100000 k] v` inflates `keys` to
/// arbitrary length trivially, and this used to recurse once per key
/// segment with no depth cap. Rewritten iteratively — an explicit
/// work-stack instead of one native call per key — the same pattern
/// [`get_path`] (this file) already uses to walk a key path without native
/// recursion at all: walk down recording each level's parsed pairs and the
/// key being set, then rebuild bottom-up. This eliminates the native-stack
/// risk entirely rather than just capping it, and is byte-for-byte
/// equivalent to the old recursive version (same `pairs`/`upsert` calls, in
/// the same order, so error precedence is unchanged too).
fn set_path(
    vm: &mut Vm,
    cur: &Value,
    keys: &[Value],
    value: Value,
) -> Result<Value, Completion<Value>> {
    let mut frames: Vec<DictPathFrame> = Vec::with_capacity(keys.len());
    let mut node = cur.clone();
    let mut parent_was_copied = false;
    for key in keys {
        // `node` has the variable/traversal owner, the caller's root handle and
        // this loop's handle. Extra references are Tcl-level aliases.
        let (bucket_count, copied) = mutation_hash_state(&node, 3, parent_was_copied)?;
        let ps = pairs(vm, &node)?;
        let k = key.to_str().to_string();
        let next = lookup(&ps, &k).cloned().unwrap_or_else(Value::empty);
        frames.push((ps, k, bucket_count));
        node = next;
        parent_was_copied = copied;
    }
    let mut new_value = value;
    for (mut ps, k, bucket_count) in frames.into_iter().rev() {
        upsert(&mut ps, &k, new_value);
        new_value = from_pairs_with_hash_bucket_count(&ps, bucket_count);
    }
    Ok(new_value)
}

/// Remove the nested `keys` path from dict `cur` (`dict unset` with multiple
/// keys). A missing intermediate key is a no-op, matching `dict unset`.
///
/// Issue #996: same unbounded per-key-segment recursion as [`set_path`]
/// (`dict unset d {*}[lrepeat 100000 k]`), rewritten iteratively for the
/// same reason. Walk down while each key is present, recording each level's
/// parsed pairs and the key followed; stop early — without erroring or
/// descending further — at the first missing intermediate key, matching
/// `dict unset`'s no-op semantics for that case exactly (the halted level's
/// pairs are simply re-serialised unchanged, precisely what the old
/// recursive version did by falling through its `if`/`else if` with neither
/// arm taken). The final key is removed via `retain`, matching the old
/// leaf case. Rebuild bottom-up via the recorded frames.
fn unset_path(vm: &mut Vm, cur: &Value, keys: &[Value]) -> Result<Value, Completion<Value>> {
    let mut frames: Vec<DictPathFrame> = Vec::with_capacity(keys.len());
    let mut node = cur.clone();
    let mut parent_was_copied = false;
    let mut new_value;
    let mut i = 0;
    loop {
        let (bucket_count, copied) = mutation_hash_state(&node, 3, parent_was_copied)?;
        let ps = pairs(vm, &node)?;
        let k = keys[i].to_str().to_string();
        if i + 1 == keys.len() {
            // Last key: remove it outright, regardless of whether it was
            // present (matching `dict unset`'s leaf semantics).
            let mut ps = ps;
            ps.retain(|(pk, _)| pk != &k);
            new_value = from_pairs_with_hash_bucket_count(&ps, bucket_count);
            break;
        }
        if let Some(sub) = lookup(&ps, &k).cloned() {
            frames.push((ps, k, bucket_count));
            node = sub;
            parent_was_copied = copied;
            i += 1;
        } else {
            // Missing intermediate key: no-op at and below this level.
            new_value = from_pairs_with_hash_bucket_count(&ps, bucket_count);
            break;
        }
    }
    for (mut ps, k, bucket_count) in frames.into_iter().rev() {
        upsert(&mut ps, &k, new_value);
        new_value = from_pairs_with_hash_bucket_count(&ps, bucket_count);
    }
    Ok(new_value)
}

fn upsert(ps: &mut Vec<(String, Value)>, key: &str, value: Value) {
    if let Some(slot) = ps.iter_mut().find(|(k, _)| k == key) {
        slot.1 = value;
    } else {
        ps.push((key.to_owned(), value));
    }
}

/// Follow the nested `keys` path into dict `cur`, returning the value at the
/// leaf (`cur` itself when `keys` is empty). Errors if a key is absent or an
/// intermediate value is not a dict (`dict get` / `dict with` path semantics).
fn get_path(vm: &mut Vm, cur: &Value, keys: &[Value]) -> Result<Value, Completion<Value>> {
    let mut v = cur.clone();
    for key in keys {
        let ps = pairs(vm, &v)?;
        let ks = key.to_str();
        match lookup(&ps, &ks) {
            Some(found) => v = found.clone(),
            None => return Err(err(format!("key \"{ks}\" not known in dictionary"))),
        }
    }
    Ok(v)
}

#[allow(clippy::too_many_lines)] // One subcommand-dispatch match; splitting obscures it.
fn dict_op(vm: &mut Vm, sub: &str, rest: &[Value], invoked: &str) -> Completion<Value> {
    // Pure dict subcommands live in the shared command core; the VM is a thin
    // adapter. Only the variable-*mutating* subcommands fall through to the
    // arms below.
    //
    // `dispatch_canon` answers `Some` unconditionally for the pure subcommands,
    // including `info`, so the VM's former own arms for those were unreachable.
    // They are gone (issue #1427's cleanup) — and not merely as tidying: the
    // dead `create` arm still built its result with a plain `Value::list` of the
    // arguments, so had anything ever routed back to it, it would have
    // re-introduced the duplicate-key bug this issue fixes.
    if let Some(result) = tcl_cmd_core::dict::dispatch_canon(vm, invoked, sub, rest) {
        return match result {
            Ok(v) => ok(v),
            // A *value-parse* failure is re-worded to C's dict spelling and
            // given its `TCL VALUE DICTIONARY …` code; anything else (wrong #
            // args, unknown key) passes through unchanged (issue #1573).
            Err(e) if e.error_code().is_some() => crate::command::completion_from_cmd_error(e),
            Err(e) => crate::exec::dict_parse_err(&e.into_message()),
        };
    }
    match sub {
        "set" => {
            // dict set dictVarName key ?key ...? value
            let [varname, keys @ .., value] = rest else {
                return err("wrong # args: should be \"dict set dictVarName key ?key ...? value\"");
            };
            if keys.is_empty() {
                return err("wrong # args: should be \"dict set dictVarName key ?key ...? value\"");
            }
            let name = varname.to_str();
            let cur = vm.get_var(&name).unwrap_or_else(Value::empty);
            let result = match set_path(vm, &cur, keys, value.clone()) {
                Ok(v) => v,
                Err(c) => return c,
            };
            if let Err(e) = vm.set_var(&name, result.clone()) {
                return e;
            }
            ok(result)
        }
        "unset" => {
            let [varname, keys @ ..] = rest else {
                return err("wrong # args: should be \"dict unset dictVarName key ?key ...?\"");
            };
            if keys.is_empty() {
                return err("wrong # args: should be \"dict unset dictVarName key ?key ...?\"");
            }
            let name = varname.to_str();
            let cur = vm.get_var(&name).unwrap_or_else(Value::empty);
            let result = match unset_path(vm, &cur, keys) {
                Ok(v) => v,
                Err(c) => return c,
            };
            if let Err(e) = vm.set_var(&name, result.clone()) {
                return e;
            }
            ok(result)
        }
        "incr" => {
            let [varname, key, amt @ ..] = rest else {
                return err("wrong # args: should be \"dict incr dictVarName key ?increment?\"");
            };
            // The same tower addition `incr` uses (`value_ops::int_add`): a sum
            // past `i64` promotes to `i128` and past that to an
            // arbitrary-precision bignum, matching tclsh ( —
            // `dict incr` at `i64::MAX` yields `9223372036854775808`, never an
            // overflow error).
            let one = Value::int(1);
            let inc = amt.first().unwrap_or(&one);
            dict_update(vm, varname, key, |old| {
                crate::value_ops::int_add(old, inc).map_err(|e| e.message())
            })
        }
        "append" => {
            let [varname, key, strs @ ..] = rest else {
                return err("wrong # args: should be \"dict append dictVarName key ?value ...?\"");
            };
            dict_update(vm, varname, key, |old| {
                let mut s = old.map(|v| v.to_str().to_string()).unwrap_or_default();
                for v in strs {
                    s.push_str(&v.to_str());
                }
                Ok(Value::string(s))
            })
        }
        "lappend" => {
            let [varname, key, vals @ ..] = rest else {
                return err("wrong # args: should be \"dict lappend dictVarName key ?value ...?\"");
            };
            dict_update(vm, varname, key, |old| {
                let mut items = match old {
                    Some(v) => match v.as_list() {
                        Ok(i) => (*i).clone(),
                        Err(e) => return Err(e.message),
                    },
                    None => Vec::new(),
                };
                items.extend(vals.iter().cloned());
                Ok(Value::list(items))
            })
        }
        "for" => cmd_dict_for(vm, rest),
        "map" => cmd_dict_map(vm, rest),
        "update" => cmd_dict_update(vm, rest),
        "filter" => cmd_dict_filter(vm, rest),
        "with" => cmd_dict_with(vm, rest),
        // Unreachable from `cmd_dict` (every `DICT_SUBS` name is handled above
        // or by the shared core); the registered `::tcl::dict::*` entry points
        // pass canonical names.
        other => err(
            String::from_utf8_lossy(&tcl_cmd_core::ensemble::unknown_subcommand_message(
                DICT_SUBS,
                other.as_bytes(),
                true,
                b"::tcl::dict",
            ))
            .into_owned(),
        ),
    }
}

/// `dict for {keyVar valueVar} dictionary body`.
/// `dict with dictVarName ?key ...? body` — map the keys of the dict (at the
/// `key` path) to like-named local variables, run `body`, then reflect the
/// variables back into the dictionary and store it: an originally-mapped key
/// whose variable still exists is updated, one whose variable was unset is
/// removed, and variables the body merely created are not added. The write-back
/// happens even when the body raises (matching C), after which the body's
/// completion (its result on success, else the error/break/continue/return) is
/// returned.
fn cmd_dict_with(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((dictvar, tail)) = rest.split_first() else {
        return err("wrong # args: should be \"dict with dictVarName ?key ...? script\"");
    };
    let Some((body, keys)) = tail.split_last() else {
        return err("wrong # args: should be \"dict with dictVarName ?key ...? script\"");
    };
    let varname = dictvar.to_str().to_string();
    let Some(root_dict) = vm.get_var(&varname) else {
        return err(format!("can't read \"{varname}\": no such variable"));
    };
    let leaf = match get_path(vm, &root_dict, keys) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let leaf_pairs = match pairs(vm, &leaf) {
        Ok(p) => p,
        Err(c) => return c,
    };
    for (k, v) in &leaf_pairs {
        if let Err(e) = vm.set_var(k, v.clone()) {
            return e;
        }
    }

    // These traversal handles are implementation detail. Keep only the
    // canonical key/value snapshot needed for write-back, so re-reading the
    // variable below observes actual Tcl aliases rather than our own handles.
    drop(leaf);
    drop(root_dict);
    let outcome = vm.eval_source(&body.to_str());

    // Reflect the mapped variables back into the dictionary. Re-read the
    // variable first (the body may have replaced it outright); if the body
    // unset it, skip the write-back entirely (matching C).
    if let Some(cur) = vm.get_var(&varname)
        && let Ok((_, root_was_copied)) = mutation_hash_state(&cur, 2, false)
        && let Ok(cur_leaf) = get_path(vm, &cur, keys)
        && let Ok((bucket_count, _)) = mutation_hash_state(
            &cur_leaf,
            if keys.is_empty() { 3 } else { 2 },
            root_was_copied,
        )
        && let Ok(mut new_pairs) = pairs(vm, &cur_leaf)
    {
        for (k, _) in &leaf_pairs {
            match vm.get_var(k) {
                Some(val) => upsert(&mut new_pairs, k, val),
                None => new_pairs.retain(|(pk, _)| pk != k),
            }
        }
        let new_leaf = from_pairs_with_hash_bucket_count(&new_pairs, bucket_count);
        let new_dict = if keys.is_empty() {
            new_leaf
        } else {
            match set_path(vm, &cur, keys, new_leaf) {
                Ok(d) => d,
                Err(c) => return c,
            }
        };
        if let Err(e) = vm.set_var(&varname, new_dict) {
            return e;
        }
    }

    match outcome {
        Ok(c) if c.code == Code::Ok => ok(c.result),
        Ok(c) => c,
        Err(e) => err(e.message),
    }
}

fn cmd_dict_for(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [vars, dict, body] = rest else {
        return err("wrong # args: should be \"dict for {keyVar valueVar} dictionary script\"");
    };
    let vnames = match vars.as_list() {
        Ok(v) => v,
        Err(e) => return err(e.message),
    };
    let [kvar, vvar] = vnames.as_slice() else {
        return err("must have exactly two variable names");
    };
    let ps = match pairs(vm, dict) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let body_src = body.to_str();
    for (k, v) in ps {
        if let Err(e) = vm.set_var(&kvar.to_str(), Value::string(k)) {
            return e;
        }
        if let Err(e) = vm.set_var(&vvar.to_str(), v) {
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
    }
    settle_control_options(ok(Value::empty()), ControlOptionPolicy::FRESH_SETTLED)
}

/// `dict map {keyVar valueVar} dictionary body` — like `dict for`, but collect
/// each iteration's body result into a new dictionary keyed by the (possibly
/// body-modified) `keyVar`. `continue` drops the pair, `break` stops, and an
/// error/`return` propagates.
fn cmd_dict_map(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [vars, dict, body] = rest else {
        return err(
            "wrong # args: should be \"dict map {keyVarName valueVarName} dictionary script\"",
        );
    };
    let vnames = match vars.as_list() {
        Ok(v) => v,
        Err(e) => return err(e.message),
    };
    let [kvar, vvar] = vnames.as_slice() else {
        return err("must have exactly two variable names");
    };
    let ps = match pairs(vm, dict) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let (kname, vname) = (kvar.to_str(), vvar.to_str());
    let body_src = body.to_str();
    let mut out: Vec<(String, Value)> = Vec::new();
    let mut last_options = Value::empty();
    for (k, v) in ps {
        if let Err(e) = vm.set_var(&kname, Value::string(k.clone())) {
            return e;
        }
        if let Err(e) = vm.set_var(&vname, v) {
            return e;
        }
        match vm.eval_source(&body_src) {
            Ok(c) => match c.code {
                Code::Ok => {
                    let key = vm.get_var(&kname).map_or(k, |kv| kv.to_str().to_string());
                    upsert(&mut out, &key, c.result);
                    last_options = c.options;
                }
                Code::Continue => last_options = c.options,
                // `break` discards the *whole* accumulated result (C
                // `DictMapNRCmd` drops it on TCL_BREAK), returning the empty dict
                // — not the pairs collected before the break.
                Code::Break => {
                    return settle_control_options(
                        ok(Value::empty()),
                        ControlOptionPolicy::FRESH_SETTLED,
                    );
                }
                _ => return c,
            },
            Err(e) => return err(e.message),
        }
    }
    settle_control_options(
        Completion::new(Code::Ok, from_pairs(&out), last_options),
        ControlOptionPolicy::FRESH_FORWARDED,
    )
}

/// `dict update dictVar key varName ?key varName ...? body` — expose the named
/// keys as local variables, run `body`, then reflect the variables back into the
/// dictionary (an unset variable removes its key). The write-back re-reads the
/// dict variable (the body may have changed it) and runs even when the body
/// raises, after which the body's completion is returned.
fn cmd_dict_update(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    const USAGE: &str =
        "wrong # args: should be \"dict update dictVarName key varName ?key varName ...? script\"";
    let [dictvar, pairs_and_body @ ..] = rest else {
        return err(USAGE);
    };
    let Some((body, kv)) = pairs_and_body.split_last() else {
        return err(USAGE);
    };
    if kv.is_empty() || !kv.len().is_multiple_of(2) {
        return err(USAGE);
    }
    let dname = dictvar.to_str().to_string();
    let cur = vm.get_var(&dname).unwrap_or_else(Value::empty);
    let ps = match pairs(vm, &cur) {
        Ok(p) => p,
        Err(c) => return c,
    };
    // Bind each key's value to its variable (a missing key leaves it unset).
    let mut i = 0;
    while i + 1 < kv.len() {
        let key = kv[i].to_str();
        let var = kv[i + 1].to_str();
        match lookup(&ps, &key) {
            Some(v) => {
                if let Err(e) = vm.set_var(&var, v.clone()) {
                    return e;
                }
            }
            None => {
                let _ = vm.unset_one(&var, false);
            }
        }
        i += 2;
    }
    // The initial read and decoded snapshot are command-internal handles, not
    // Tcl-level aliases. Release them before the post-body COW decision.
    drop(ps);
    drop(cur);
    let comp = match vm.eval_source(&body.to_str()) {
        Ok(c) => c,
        Err(e) => return err(e.message),
    };
    // Write-back (always, even on error): re-read the dict, apply each variable.
    let cur2 = vm.get_var(&dname).unwrap_or_else(Value::empty);
    if let Ok((bucket_count, _)) = mutation_hash_state(&cur2, 2, false)
        && let Ok(mut ps2) = pairs(vm, &cur2)
    {
        let mut i = 0;
        while i + 1 < kv.len() {
            let key = kv[i].to_str().to_string();
            let var = kv[i + 1].to_str();
            match vm.get_var(&var) {
                Some(v) => upsert(&mut ps2, &key, v),
                None => ps2.retain(|(k, _)| k != &key),
            }
            i += 2;
        }
        if let Err(e) = vm.set_var(
            &dname,
            from_pairs_with_hash_bucket_count(&ps2, bucket_count),
        ) {
            return e;
        }
    }
    comp
}

/// `dict filter dictionary script {keyVar valueVar} body` — the Family-B `script`
/// filter type (the pure `key`/`value` globs are handled by the shared
/// `tcl_cmd_core::dict` core, which returns `None` only for `script`). Keeps each
/// pair whose body result is true; the body's completion code drives the loop
/// (OK ⇒ keep iff true; CONTINUE ⇒ skip; BREAK ⇒ stop; else ⇒ propagate).
fn cmd_dict_filter(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [dict, _script, vars, body] = rest else {
        return err(
            "wrong # args: should be \"dict filter dictionary script {keyVarName valueVarName} filterScript\"",
        );
    };
    let ps = match pairs(vm, dict) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let vnames = match vars.as_list() {
        Ok(v) => v,
        Err(e) => return err(e.message),
    };
    let [kvar, vvar] = vnames.as_slice() else {
        return err("must have exactly two variable names");
    };
    let body_src = body.to_str();
    let mut kept: Vec<(String, Value)> = Vec::new();
    for (k, v) in ps {
        if let Err(e) = vm.set_var(&kvar.to_str(), Value::string(k.as_str())) {
            return e;
        }
        if let Err(e) = vm.set_var(&vvar.to_str(), v.clone()) {
            return e;
        }
        match vm.eval_source(&body_src) {
            Ok(c) => match c.code {
                Code::Ok => match c.result.as_bool() {
                    Ok(true) => kept.push((k, v)),
                    Ok(false) => {}
                    Err(e) => return err(e.message),
                },
                Code::Continue => {}
                Code::Break => break,
                _ => return c,
            },
            Err(e) => return err(e.message),
        }
    }
    ok(from_pairs(&kept))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dict `Value`'s top-level `(key, value-string)` pairs.
    fn top_pairs(vm: &mut Vm, v: &Value) -> Vec<(String, String)> {
        pairs(vm, v)
            .expect("dict value is a valid list")
            .into_iter()
            .map(|(k, val)| (k, val.to_str().to_string()))
            .collect()
    }

    /// Regression coverage for issue #996: `set_path`/`unset_path` recurse
    /// once per multi-key `dict set`/`dict unset` path segment, with no
    /// depth cap before this fix — trivially inflated via `dict set d
    /// {*}[lrepeat 100000 k] v`. Empirically (a throwaway `zzz_probe_depth
    /// dict_set <depth>` harness, deleted before this fix landed),
    /// unguarded input overflowed the native stack (SIGABRT) between depth
    /// 3000 and 3500 on a 2 MiB thread (`cargo test`'s per-test default).
    /// Rewritten iteratively (no depth cap at all, mirroring `get_path`'s
    /// existing iterative style); 2000 is comfortably past that crash
    /// range, and `set_path`'s result is checked for exact correctness at
    /// this depth (descending back down through the same key at every
    /// level must land on the value that was set), not merely survival.
    ///
    /// Deliberately NOT 50,000+: a dict this deep is represented as an
    /// equally deep nested `Value::list` chain, and `Value` has no custom
    /// `Drop` impl — the compiler-generated recursive drop glue that runs
    /// when `set`/`cur` go out of scope at the end of this test is its own,
    /// unrelated native-stack risk (empirically, SIGABRT between depth 3500
    /// and 4000 on a 2 MiB thread for construction+drop alone), a separate
    /// concern in `Value`'s representation itself, out of scope here.
    #[test]
    fn deeply_nested_dict_set_and_unset_survive() {
        const DEPTH: usize = 2_000;
        let vm = &mut Vm::new();
        let keys: Vec<Value> = (0..DEPTH).map(|_| Value::string("k")).collect();
        let set =
            set_path(vm, &Value::empty(), &keys, Value::string("v")).expect("set_path survives");
        let mut cur = set.clone();
        for _ in 0..DEPTH {
            let ps = pairs(vm, &cur).expect("valid dict at every level");
            assert_eq!(ps.len(), 1);
            assert_eq!(ps[0].0, "k");
            cur = ps[0].1.clone();
        }
        assert_eq!(&*cur.to_str(), "v");

        // `unset_path` must also survive the same depth — the assertion is
        // that it returns at all, not the exact shape of what remains (each
        // level's "k" entry survives holding an emptied-out subdict, matching
        // `dict unset`'s "leaf only" removal semantics — the whole chain does
        // not collapse away).
        let _ = unset_path(vm, &set, &keys).expect("unset_path survives");
    }

    /// A moderately nested `dict set`/`dict unset` path (well within
    /// realistic use) is byte-for-byte unaffected by the iterative
    /// rewrite.
    #[test]
    fn moderately_nested_dict_set_and_unset_unaffected() {
        let vm = &mut Vm::new();
        let s = Value::string;

        // `dict set {} a 1` -> {a 1}.
        let d = set_path(vm, &Value::list(vec![]), &[s("a")], s("1")).unwrap();
        assert_eq!(top_pairs(vm, &d), [("a".into(), "1".into())]);

        // Updating an existing key keeps its position (order-preserving).
        let base = Value::list(vec![s("a"), s("1"), s("b"), s("2")]);
        let updated = set_path(vm, &base, &[s("a")], s("9")).unwrap();
        assert_eq!(
            top_pairs(vm, &updated),
            [("a".into(), "9".into()), ("b".into(), "2".into())]
        );

        // A new key appends at the end.
        let appended = set_path(vm, &base, &[s("c")], s("3")).unwrap();
        assert_eq!(
            top_pairs(vm, &appended),
            [
                ("a".into(), "1".into()),
                ("b".into(), "2".into()),
                ("c".into(), "3".into())
            ]
        );

        // A multi-key path auto-vivifies intermediate dicts.
        let nested = set_path(vm, &Value::list(vec![]), &[s("a"), s("b")], s("1")).unwrap();
        assert_eq!(top_pairs(vm, &nested), [("a".into(), "b 1".into())]);

        // Removing a present key drops just that pair.
        let removed = unset_path(vm, &base, &[s("a")]).unwrap();
        assert_eq!(top_pairs(vm, &removed), [("b".into(), "2".into())]);

        // Removing an absent key leaves the dict unchanged.
        let untouched = unset_path(vm, &base, &[s("z")]).unwrap();
        assert_eq!(
            top_pairs(vm, &untouched),
            [("a".into(), "1".into()), ("b".into(), "2".into())]
        );

        // A nested unset rewrites only the inner dict.
        let inner = Value::list(vec![s("b"), s("1"), s("c"), s("2")]);
        let outer = Value::list(vec![s("a"), inner]);
        let nested_unset = unset_path(vm, &outer, &[s("a"), s("b")]).unwrap();
        assert_eq!(top_pairs(vm, &nested_unset), [("a".into(), "c 2".into())]);

        // A missing intermediate key two levels deep is a no-op, not an
        // error, and does not disturb sibling keys.
        let nested_absent = unset_path(vm, &outer, &[s("a"), s("z"), s("q")]).unwrap();
        assert_eq!(
            top_pairs(vm, &nested_absent),
            [("a".into(), "b 1 c 2".into())]
        );
    }
}
