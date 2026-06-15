//! List builtins, reusing `tcl_syntax::list` for split/merge semantics.

use std::cmp::Ordering;
use std::rc::Rc;

use tcl_runtime_api::Completion;

use crate::command::resolve_index;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

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

fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

fn cmd_list(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    ok(Value::list(args.to_vec()))
}

fn cmd_llength(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [l] => match as_list(l) {
            Ok(items) => ok(Value::int(ilen(items.len()))),
            Err(c) => c,
        },
        _ => err("wrong # args: should be \"llength list\""),
    }
}

/// Resolve an index against `len`, returning `Some(i)` only when in range.
fn in_range(spec: &str, len: usize) -> Option<usize> {
    let idx = resolve_index(spec, len)?;
    usize::try_from(idx).ok().filter(|&i| i < len)
}

fn cmd_lindex(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((list, idxs)) = args.split_first() else {
        return err("wrong # args: should be \"lindex list ?index ...?\"");
    };
    let mut cur = list.clone();
    for idx in idxs {
        let items = match as_list(&cur) {
            Ok(i) => i,
            Err(c) => return c,
        };
        match in_range(&idx.to_str(), items.len()) {
            Some(i) => cur = items[i].clone(),
            None => return ok(Value::empty()),
        }
    }
    ok(cur)
}

fn cmd_lrange(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [list, from, to] = args else {
        return err("wrong # args: should be \"lrange list first last\"");
    };
    let items = match as_list(list) {
        Ok(i) => i,
        Err(c) => return c,
    };
    let len = items.len();
    let lo = resolve_index(&from.to_str(), len).unwrap_or(0).max(0);
    let hi = resolve_index(&to.to_str(), len).unwrap_or(0);
    let lo = usize::try_from(lo).unwrap_or(0);
    if hi < 0 || lo >= len {
        return ok(Value::empty());
    }
    let hi = usize::try_from(hi).unwrap_or(0).min(len - 1);
    if lo > hi {
        return ok(Value::empty());
    }
    ok(Value::list(items[lo..=hi].to_vec()))
}

fn cmd_lappend(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((name, vals)) = args.split_first() else {
        return err("wrong # args: should be \"lappend varName ?value ...?\"");
    };
    let n = name.to_str();
    let mut items: Vec<Value> = match vm.var_get(&n) {
        Some(v) => match as_list(&v) {
            Ok(i) => (*i).clone(),
            Err(c) => return c,
        },
        None => Vec::new(),
    };
    items.extend(vals.iter().cloned());
    let result = Value::list(items);
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

fn cmd_lreverse(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [l] => match as_list(l) {
            Ok(items) => {
                let mut v = (*items).clone();
                v.reverse();
                ok(Value::list(v))
            }
            Err(c) => c,
        },
        _ => err("wrong # args: should be \"lreverse list\""),
    }
}

fn cmd_lrepeat(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((count, elems)) = args.split_first() else {
        return err("wrong # args: should be \"lrepeat count ?value ...?\"");
    };
    let n = match count.as_int() {
        Ok(n) if n >= 0 => usize::try_from(n).unwrap_or(0),
        Ok(_) => return err("bad count \"lrepeat\": must be >= 0"),
        Err(e) => return err(e.message),
    };
    let mut v = Vec::with_capacity(n.saturating_mul(elems.len()));
    for _ in 0..n {
        v.extend(elems.iter().cloned());
    }
    ok(Value::list(v))
}

fn cmd_linsert(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((list, tail)) = args.split_first() else {
        return err("wrong # args: should be \"linsert list index ?element ...?\"");
    };
    let Some((idx, elems)) = tail.split_first() else {
        return err("wrong # args: should be \"linsert list index ?element ...?\"");
    };
    let mut items = match as_list(list) {
        Ok(i) => (*i).clone(),
        Err(c) => return c,
    };
    let at = resolve_index(&idx.to_str(), items.len())
        .unwrap_or(0)
        .clamp(0, isize::try_from(items.len()).unwrap_or(isize::MAX));
    let at = usize::try_from(at).unwrap_or(0).min(items.len());
    for (k, e) in elems.iter().enumerate() {
        items.insert(at + k, e.clone());
    }
    ok(Value::list(items))
}

fn cmd_lreplace(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [list, from, to, rest @ ..] = args else {
        return err("wrong # args: should be \"lreplace list first last ?element ...?\"");
    };
    let mut items = match as_list(list) {
        Ok(i) => (*i).clone(),
        Err(c) => return c,
    };
    let len = items.len();
    let lo = resolve_index(&from.to_str(), len).unwrap_or(0).max(0);
    let lo = usize::try_from(lo).unwrap_or(0).min(len);
    let hi = resolve_index(&to.to_str(), len).unwrap_or(-1);
    let end = if hi < 0 {
        lo
    } else {
        (usize::try_from(hi).unwrap_or(0) + 1).clamp(lo, len)
    };
    let replacement = rest.to_vec();
    items.splice(lo..end, replacement);
    ok(Value::list(items))
}

fn cmd_lsearch(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // Minimal: lsearch ?-exact|-glob? list pattern  (default -glob).
    let mut glob = true;
    let mut rest = args;
    while let Some(first) = rest.first() {
        match &*first.to_str() {
            "-exact" => {
                glob = false;
                rest = &rest[1..];
            }
            "-glob" => {
                glob = true;
                rest = &rest[1..];
            }
            s if s.starts_with('-') => rest = &rest[1..], // ignore unknown options (M3)
            _ => break,
        }
    }
    let [list, pat] = rest else {
        return err("wrong # args: should be \"lsearch ?options? list pattern\"");
    };
    let items = match as_list(list) {
        Ok(i) => i,
        Err(c) => return c,
    };
    let p = pat.to_str();
    for (i, item) in items.iter().enumerate() {
        let s = item.to_str();
        let hit = if glob {
            tcl_syntax::glob::string_match(&p, &s)
        } else {
            *s == *p
        };
        if hit {
            return ok(Value::int(ilen(i)));
        }
    }
    ok(Value::int(-1))
}

fn cmd_lsort(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let mut integer = false;
    let mut decreasing = false;
    let mut unique = false;
    let mut rest = args;
    while let Some(first) = rest.first() {
        match &*first.to_str() {
            "-integer" | "-real" => {
                integer = true;
                rest = &rest[1..];
            }
            "-ascii" | "-dictionary" => rest = &rest[1..],
            "-decreasing" => {
                decreasing = true;
                rest = &rest[1..];
            }
            "-increasing" => {
                decreasing = false;
                rest = &rest[1..];
            }
            "-unique" => {
                unique = true;
                rest = &rest[1..];
            }
            s if s.starts_with('-') => rest = &rest[1..],
            _ => break,
        }
    }
    let [list] = rest else {
        return err("wrong # args: should be \"lsort ?options? list\"");
    };
    let mut items = match as_list(list) {
        Ok(i) => (*i).clone(),
        Err(c) => return c,
    };
    items.sort_by(|a, b| {
        let ord = if integer {
            a.as_double()
                .unwrap_or(0.0)
                .partial_cmp(&b.as_double().unwrap_or(0.0))
                .unwrap_or(Ordering::Equal)
        } else {
            (*a.to_str()).cmp(&b.to_str())
        };
        if decreasing { ord.reverse() } else { ord }
    });
    if unique {
        items.dedup_by(|a, b| *a.to_str() == *b.to_str());
    }
    ok(Value::list(items))
}

fn cmd_concat(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let joined = args
        .iter()
        .map(|v| v.to_str().trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    ok(Value::string(joined))
}

fn cmd_join(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (list, sep) = match args {
        [l] => (l, " ".to_string()),
        [l, s] => (l, s.to_str().to_string()),
        _ => return err("wrong # args: should be \"join list ?joinString?\""),
    };
    let items = match as_list(list) {
        Ok(i) => i,
        Err(c) => return c,
    };
    let parts: Vec<String> = items.iter().map(|v| v.to_str().to_string()).collect();
    ok(Value::string(parts.join(&sep)))
}

fn cmd_split(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (s, chars) = match args {
        [s] => (s.to_str(), " \t\n".to_string()),
        [s, c] => (s.to_str(), c.to_str().to_string()),
        _ => return err("wrong # args: should be \"split string ?splitChars?\""),
    };
    let pieces: Vec<Value> = if chars.is_empty() {
        // Split into individual characters.
        s.chars().map(|c| Value::string(c.to_string())).collect()
    } else {
        let set: Vec<char> = chars.chars().collect();
        s.split(|c| set.contains(&c)).map(Value::string).collect()
    };
    ok(Value::list(pieces))
}
