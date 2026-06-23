//! `tcl::prefix` — match a string against a table of valid prefixes.
//!
//! Implements the `match` / `all` / `longest` subcommands the Tcl ensemble
//! dispatch (and a few stdlib helpers) rely on. The `match` error messages and
//! `-message` / `-error` / `-exact` options mirror `tclIndexObj.c`.

use tcl_runtime_api::{Code, Completion};
use tcl_syntax::list::split_list;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("tcl::prefix", cmd_prefix);
    vm.register("::tcl::prefix", cmd_prefix);
}

fn cmd_prefix(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"tcl::prefix subcommand ?arg ...?\"");
    };
    match &*sub.to_str() {
        "all" => prefix_all(rest),
        "longest" => prefix_longest(rest),
        "match" => prefix_match(vm, rest),
        other => err(format!(
            "unknown or ambiguous subcommand \"{other}\": must be all, longest, or match"
        )),
    }
}

/// Split a list value, surfacing a parse error as a completion.
fn entries(v: &Value) -> Result<Vec<String>, Completion<Value>> {
    split_list(&v.to_str())
        .map(|e| e.iter().map(ToString::to_string).collect())
        .map_err(|e| err(e.message().to_string()))
}

/// `tcl::prefix all table string` — every table entry with `string` as a prefix.
fn prefix_all(rest: &[Value]) -> Completion<Value> {
    let [table, s] = rest else {
        return err("wrong # args: should be \"tcl::prefix all table string\"");
    };
    let table = match entries(table) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let s = s.to_str();
    let out: Vec<Value> = table
        .into_iter()
        .filter(|e| e.starts_with(&*s))
        .map(Value::string)
        .collect();
    ok(Value::list(out))
}

/// `tcl::prefix longest table string` — the longest common prefix of the table
/// entries that have `string` as a prefix (empty when none match).
fn prefix_longest(rest: &[Value]) -> Completion<Value> {
    let [table, s] = rest else {
        return err("wrong # args: should be \"tcl::prefix longest table string\"");
    };
    let table = match entries(table) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let s = s.to_str();
    let matches: Vec<&String> = table.iter().filter(|e| e.starts_with(&*s)).collect();
    let Some((first, others)) = matches.split_first() else {
        return ok(Value::empty());
    };
    // Longest common prefix of all matching entries.
    let mut len = first.chars().count();
    for e in others {
        let common = first
            .chars()
            .zip(e.chars())
            .take_while(|(a, b)| a == b)
            .count();
        len = len.min(common);
    }
    ok(Value::string(first.chars().take(len).collect::<String>()))
}

/// Format a table for an error message: `a`, `a or b`, or `a, b, or c` — Tcl
/// omits the comma for a two-element list.
fn oxford_or(table: &[String]) -> String {
    match table {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a} or {b}"),
        _ => {
            let mut out = String::new();
            for (i, e) in table.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                if i == table.len() - 1 {
                    out.push_str("or ");
                }
                out.push_str(e);
            }
            out
        }
    }
}

/// `tcl::prefix match ?-exact? ?-message s? ?-error opts? table string`.
fn prefix_match(_vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let mut exact = false;
    let mut message = "option".to_string();
    let mut error_opts: Option<Value> = None;
    // The trailing two words are always `table string`; everything before them
    // is the option region (C parses `objv[2 .. objc-2]`). So a non-option there
    // is `bad option` (not `wrong # args`), and a `-message`/`-error` with no
    // following word *within* the region is `missing value` (string-26.x).
    if rest.len() < 2 {
        return err("wrong # args: should be \"tcl::prefix match ?options? table string\"");
    }
    let (opts, tail) = rest.split_at(rest.len() - 2);
    let [table, sv] = tail else {
        return err("wrong # args: should be \"tcl::prefix match ?options? table string\"");
    };
    let mut i = 0;
    while i < opts.len() {
        match &*opts[i].to_str() {
            "-exact" => {
                exact = true;
                i += 1;
            }
            "-message" => {
                let Some(v) = opts.get(i + 1) else {
                    return err("missing value for -message");
                };
                message = v.to_str().to_string();
                i += 2;
            }
            "-error" => {
                let Some(v) = opts.get(i + 1) else {
                    return err("missing value for -error");
                };
                error_opts = Some(v.clone());
                i += 2;
            }
            other => {
                return err(format!(
                    "bad option \"{other}\": must be -error, -exact, or -message"
                ));
            }
        }
    }
    let table = match entries(table) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let s = sv.to_str();

    // An exact table entry always wins; otherwise a unique prefix matches.
    if let Some(e) = table.iter().find(|e| **e == *s) {
        return ok(Value::string(e.clone()));
    }
    let matches: Vec<&String> = if exact {
        Vec::new()
    } else {
        table.iter().filter(|e| e.starts_with(&*s)).collect()
    };
    if let [only] = matches.as_slice() {
        return ok(Value::string((*only).clone()));
    }

    // No (or ambiguous) match: build the error.
    let kind = if matches.len() > 1 {
        "ambiguous"
    } else {
        "bad"
    };
    let list = if table.is_empty() {
        "no valid options".to_string()
    } else {
        format!("must be {}", oxford_or(&table))
    };
    let msg = format!("{kind} {message} \"{s}\": {list}");
    match error_opts {
        // No `-error`: a normal error.
        None => err(msg),
        Some(opts) => match opts.as_list() {
            // The `-error` value must be a proper, even-length list (a
            // return-options dict): a malformed or odd one is reported as such
            // (string-26.3).
            Err(e) => err(e.message),
            Ok(list) if list.is_empty() => ok(Value::empty()),
            Ok(list) if list.len() % 2 != 0 => {
                err("error options must have an even number of elements")
            }
            // `-error <opts>`: report the message with the caller's return
            // options attached (the trampoline applies `-code`/`-level`).
            Ok(_) => Completion::new(Code::Error, Value::string(msg), opts),
        },
    }
}
