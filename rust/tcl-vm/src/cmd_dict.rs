//! The `dict` ensemble. A dict value is an even-length list; M3 keeps the
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
fn pairs(v: &Value) -> Result<Vec<(String, Value)>, Completion<Value>> {
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

#[allow(clippy::too_many_lines)]
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
            let inc = match amt.first() {
                Some(v) => match v.as_int() {
                    Ok(n) => n,
                    Err(e) => return err(e.message),
                },
                None => 1,
            };
            dict_update(vm, varname, key, |old| {
                let base = match old {
                    Some(v) => match v.as_int() {
                        Ok(n) => n,
                        Err(e) => return Err(e.message),
                    },
                    None => 0,
                };
                Ok(Value::int(base.wrapping_add(inc)))
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
        "filter" => cmd_dict_filter(vm, rest),
        other => err(format!("unknown or ambiguous subcommand \"{other}\"")),
    }
}

/// `dict for {keyVar valueVar} dictionary body`.
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
