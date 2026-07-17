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

//! The `dict` ensemble. A dict value is an even-length list; this currently keeps the
//! list rep (a typed dict intrep is a later optimisation).

use tcl_runtime_api::{Code, Completion};
use tcl_syntax::glob::string_match;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("dict", cmd_dict);
    // Ensemble member commands the codegen rewrites `dict <sub>` into.
    vm.register("::tcl::dict::create", |vm, a| dict_op(vm, "create", a));
    vm.register("::tcl::dict::get", |vm, a| dict_op(vm, "get", a));
    vm.register("::tcl::dict::exists", |vm, a| dict_op(vm, "exists", a));
    vm.register("::tcl::dict::keys", |vm, a| dict_op(vm, "keys", a));
    vm.register("::tcl::dict::values", |vm, a| dict_op(vm, "values", a));
    vm.register("::tcl::dict::size", |vm, a| dict_op(vm, "size", a));
    vm.register("::tcl::dict::merge", |vm, a| dict_op(vm, "merge", a));
    vm.register("::tcl::dict::set", |vm, a| dict_op(vm, "set", a));
    vm.register("::tcl::dict::unset", |vm, a| dict_op(vm, "unset", a));
    vm.register("::tcl::dict::for", |vm, a| dict_op(vm, "for", a));
    vm.register("::tcl::dict::map", |vm, a| dict_op(vm, "map", a));
    vm.register("::tcl::dict::incr", |vm, a| dict_op(vm, "incr", a));
    vm.register("::tcl::dict::append", |vm, a| dict_op(vm, "append", a));
    vm.register("::tcl::dict::lappend", |vm, a| dict_op(vm, "lappend", a));
}

/// `dict subcommand ?arg ...?` — dispatch to the subcommand handler.
fn cmd_dict(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"dict subcommand ?arg ...?\"");
    };
    dict_op(vm, &sub.to_str(), rest)
}

fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Parse a dict value into (key-string, value) pairs, preserving order.
pub(crate) fn pairs(v: &Value) -> Result<Vec<(String, Value)>, Completion<Value>> {
    let items = v.as_list().map_err(|e| err(e.message))?;
    if items.len() % 2 != 0 {
        return Err(err("missing value to go with key"));
    }
    Ok(items
        .chunks_exact(2)
        .map(|c| (c[0].to_str().to_string(), c[1].clone()))
        .collect())
}

fn from_pairs(ps: &[(String, Value)]) -> Value {
    let mut v = Vec::with_capacity(ps.len() * 2);
    for (k, val) in ps {
        v.push(Value::string(k.as_str()));
        v.push(val.clone());
    }
    Value::list(v)
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
    let mut ps = match pairs(&cur) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let k = key.to_str();
    let newv = match f(lookup(&ps, &k)) {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    upsert(&mut ps, &k, newv);
    let result = from_pairs(&ps);
    if let Err(e) = vm.set_var(&name, result.clone()) {
        return e;
    }
    ok(result)
}

/// Set the nested `keys` path of dict `cur` to `value`, creating intermediate
/// dicts as needed (`dict set` with multiple keys).
fn set_path(cur: &Value, keys: &[Value], value: Value) -> Result<Value, Completion<Value>> {
    let mut ps = pairs(cur)?;
    let k = keys[0].to_str().to_string();
    let newv = if keys.len() == 1 {
        value
    } else {
        let sub = lookup(&ps, &k).cloned().unwrap_or_else(Value::empty);
        set_path(&sub, &keys[1..], value)?
    };
    upsert(&mut ps, &k, newv);
    Ok(from_pairs(&ps))
}

/// Remove the nested `keys` path from dict `cur` (`dict unset` with multiple
/// keys). A missing intermediate key is a no-op, matching `dict unset`.
fn unset_path(cur: &Value, keys: &[Value]) -> Result<Value, Completion<Value>> {
    let mut ps = pairs(cur)?;
    let k = keys[0].to_str().to_string();
    if keys.len() == 1 {
        ps.retain(|(pk, _)| pk != &k);
    } else if let Some(idx) = ps.iter().position(|(pk, _)| pk == &k) {
        ps[idx].1 = unset_path(&ps[idx].1.clone(), &keys[1..])?;
    }
    Ok(from_pairs(&ps))
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
fn get_path(cur: &Value, keys: &[Value]) -> Result<Value, Completion<Value>> {
    let mut v = cur.clone();
    for key in keys {
        let ps = pairs(&v)?;
        let ks = key.to_str();
        match lookup(&ps, &ks) {
            Some(found) => v = found.clone(),
            None => return Err(err(format!("key \"{ks}\" not known in dictionary"))),
        }
    }
    Ok(v)
}

#[allow(clippy::too_many_lines)] // One subcommand-dispatch match; splitting obscures it.
fn dict_op(vm: &mut Vm, sub: &str, rest: &[Value]) -> Completion<Value> {
    // Pure dict subcommands now live in the shared command core; the VM is a
    // thin adapter. Variable-mutating subcommands fall through to the legacy
    // arms below.
    if let Some(result) = tcl_cmd_core::dict::dispatch_canon(vm, sub, rest) {
        return match result {
            Ok(v) => ok(v),
            Err(e) => err(e.into_message()),
        };
    }
    match sub {
        "create" => {
            if !rest.len().is_multiple_of(2) {
                return err("wrong # args: should be \"dict create ?key value ...?\"");
            }
            ok(Value::list(rest.to_vec()))
        }
        "get" => {
            let Some((d, keys)) = rest.split_first() else {
                return err("wrong # args: should be \"dict get dictionary ?key ...?\"");
            };
            let mut cur = d.clone();
            for k in keys {
                let ps = match pairs(&cur) {
                    Ok(p) => p,
                    Err(c) => return c,
                };
                match lookup(&ps, &k.to_str()) {
                    Some(v) => cur = v.clone(),
                    None => return err(format!("key \"{}\" not known in dictionary", k.to_str())),
                }
            }
            ok(cur)
        }
        "exists" => {
            let Some((d, keys)) = rest.split_first() else {
                return err("wrong # args: should be \"dict exists dictionary key ?key ...?\"");
            };
            let mut cur = d.clone();
            for k in keys {
                let Ok(ps) = pairs(&cur) else {
                    return ok(Value::bool(false));
                };
                match lookup(&ps, &k.to_str()) {
                    Some(v) => cur = v.clone(),
                    None => return ok(Value::bool(false)),
                }
            }
            ok(Value::bool(true))
        }
        "keys" => match rest {
            [d] | [d, _] => {
                let ps = match pairs(d) {
                    Ok(p) => p,
                    Err(c) => return c,
                };
                let pat = rest.get(1).map(Value::to_str);
                ok(Value::list(
                    ps.into_iter()
                        .filter(|(k, _)| pat.as_deref().is_none_or(|p| string_match(p, k)))
                        .map(|(k, _)| Value::string(k))
                        .collect(),
                ))
            }
            _ => err("wrong # args: should be \"dict keys dictionary ?pattern?\""),
        },
        "values" => match rest {
            [d] | [d, _] => {
                let ps = match pairs(d) {
                    Ok(p) => p,
                    Err(c) => return c,
                };
                let pat = rest.get(1).map(Value::to_str);
                ok(Value::list(
                    ps.into_iter()
                        .filter(|(_, v)| {
                            pat.as_deref().is_none_or(|p| string_match(p, &v.to_str()))
                        })
                        .map(|(_, v)| v)
                        .collect(),
                ))
            }
            _ => err("wrong # args: should be \"dict values dictionary ?pattern?\""),
        },
        "size" => match rest {
            [d] => match pairs(d) {
                Ok(p) => ok(Value::int(ilen(p.len()))),
                Err(c) => c,
            },
            _ => err("wrong # args: should be \"dict size dictionary\""),
        },
        "merge" => {
            let mut acc: Vec<(String, Value)> = Vec::new();
            for d in rest {
                let ps = match pairs(d) {
                    Ok(p) => p,
                    Err(c) => return c,
                };
                for (k, v) in ps {
                    upsert(&mut acc, &k, v);
                }
            }
            ok(from_pairs(&acc))
        }
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
            let result = match set_path(&cur, keys, value.clone()) {
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
            let result = match unset_path(&cur, keys) {
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
            // arbitrary-precision bignum, matching tclsh (RUST_ISSUE_095 —
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
        other => err(format!("unknown or ambiguous subcommand \"{other}\"")),
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
    let leaf = match get_path(&root_dict, keys) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let leaf_pairs = match pairs(&leaf) {
        Ok(p) => p,
        Err(c) => return c,
    };
    for (k, v) in &leaf_pairs {
        if let Err(e) = vm.set_var(k, v.clone()) {
            return e;
        }
    }

    let outcome = vm.eval_source(&body.to_str());

    // Reflect the mapped variables back into the dictionary. Re-read the
    // variable first (the body may have replaced it outright); if the body
    // unset it, skip the write-back entirely (matching C).
    if let Some(cur) = vm.get_var(&varname)
        && let Ok(cur_leaf) = get_path(&cur, keys)
        && let Ok(mut new_pairs) = pairs(&cur_leaf)
    {
        for (k, _) in &leaf_pairs {
            match vm.get_var(k) {
                Some(val) => upsert(&mut new_pairs, k, val),
                None => new_pairs.retain(|(pk, _)| pk != k),
            }
        }
        let new_leaf = from_pairs(&new_pairs);
        let new_dict = if keys.is_empty() {
            new_leaf
        } else {
            match set_path(&cur, keys, new_leaf) {
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
    let ps = match pairs(dict) {
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
    ok(Value::empty())
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
    let ps = match pairs(dict) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let (kname, vname) = (kvar.to_str(), vvar.to_str());
    let body_src = body.to_str();
    let mut out: Vec<(String, Value)> = Vec::new();
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
                }
                Code::Continue => {}
                // `break` discards the *whole* accumulated result (C
                // `DictMapNRCmd` drops it on TCL_BREAK), returning the empty dict
                // — not the pairs collected before the break.
                Code::Break => return ok(Value::empty()),
                _ => return c,
            },
            Err(e) => return err(e.message),
        }
    }
    ok(from_pairs(&out))
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
    let ps = match pairs(&cur) {
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
    let comp = match vm.eval_source(&body.to_str()) {
        Ok(c) => c,
        Err(e) => return err(e.message),
    };
    // Write-back (always, even on error): re-read the dict, apply each variable.
    let cur2 = vm.get_var(&dname).unwrap_or_else(Value::empty);
    if let Ok(mut ps2) = pairs(&cur2) {
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
        if let Err(e) = vm.set_var(&dname, from_pairs(&ps2)) {
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
    let ps = match pairs(dict) {
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
