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

fn upsert(ps: &mut Vec<(String, Value)>, key: &str, value: Value) {
    if let Some(slot) = ps.iter_mut().find(|(k, _)| k == key) {
        slot.1 = value;
    } else {
        ps.push((key.to_owned(), value));
    }
}

#[allow(clippy::too_many_lines)]
fn dict_op(vm: &mut Vm, sub: &str, rest: &[Value]) -> Completion<Value> {
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
            let [varname, mid @ .., value] = rest else {
                return err("wrong # args: should be \"dict set dictVarName key ?key ...? value\"");
            };
            if mid.is_empty() {
                return err("wrong # args: should be \"dict set dictVarName key ?key ...? value\"");
            }
            let name = varname.to_str();
            let cur = vm.get_var(&name).unwrap_or_else(Value::empty);
            let mut ps = match pairs(&cur) {
                Ok(p) => p,
                Err(c) => return c,
            };
            // M3: single level of nesting only.
            upsert(&mut ps, &mid[0].to_str(), value.clone());
            let result = from_pairs(&ps);
            vm.set_var(&name, result.clone());
            ok(result)
        }
        "unset" => {
            let [varname, key] = rest else {
                return err("wrong # args: should be \"dict unset dictVarName key ?key ...?\"");
            };
            let name = varname.to_str();
            let cur = vm.get_var(&name).unwrap_or_else(Value::empty);
            let mut ps = match pairs(&cur) {
                Ok(p) => p,
                Err(c) => return c,
            };
            let k = key.to_str();
            ps.retain(|(pk, _)| pk != &*k);
            let result = from_pairs(&ps);
            vm.set_var(&name, result.clone());
            ok(result)
        }
        "for" => cmd_dict_for(vm, rest),
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
        vm.set_var(&kvar.to_str(), Value::string(k));
        vm.set_var(&vvar.to_str(), v);
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
