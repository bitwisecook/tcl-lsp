//! The `info` ensemble — introspection over the retained frame/proc metadata.
//!
//! Implemented against the data M2 deliberately kept (per-frame proc name +
//! invocation argv, `ProcDef.body_src`/`params`, the command table) so the
//! answers are correct rather than faked — the subsystem the WASM work found
//! painful when that metadata was missing.

use tcl_runtime_api::Completion;
use tcl_syntax::glob::string_match;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("info", cmd_info);
}

fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// `info complete`: whether `script` has no unbalanced `{}`/`[]`/`"` and does
/// not end in a line continuation — i.e. it is a syntactically complete command.
fn is_complete(script: &str) -> bool {
    let b = script.as_bytes();
    let mut i = 0;
    let n = b.len();
    let mut brace = 0i32;
    let mut bracket = 0i32;
    let mut quote = false;
    while i < n {
        match b[i] {
            b'\\' => i += 1, // skip the escaped char
            b'{' if !quote => brace += 1,
            b'}' if !quote => brace -= 1,
            b'[' if !quote => bracket += 1,
            b']' if !quote => bracket -= 1,
            b'"' if brace == 0 => quote = !quote,
            _ => {}
        }
        i += 1;
    }
    let trailing_backslash = b.last() == Some(&b'\\');
    brace <= 0 && bracket <= 0 && !quote && !trailing_backslash
}

/// Filter + sort names by an optional glob pattern.
fn filtered(mut names: Vec<String>, pat: Option<&str>) -> Value {
    names.retain(|n| pat.is_none_or(|p| string_match(p, n)));
    names.sort();
    Value::list(names.into_iter().map(Value::string).collect())
}

#[allow(clippy::too_many_lines)]
fn cmd_info(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"info subcommand ?arg ...?\"");
    };
    match &*sub.to_str() {
        "exists" => match rest {
            [name] => ok(Value::bool(vm.exists_var(&name.to_str()))),
            _ => err("wrong # args: should be \"info exists varName\""),
        },
        "complete" => match rest {
            [script] => ok(Value::bool(is_complete(&script.to_str()))),
            _ => err("wrong # args: should be \"info complete command\""),
        },
        "level" => match rest {
            [] => ok(Value::int(ilen(vm.current_level()))),
            [n] => match n.as_int() {
                // A positive number is an absolute stack level; zero or negative
                // is relative to the current level (`info level 0` is the current
                // procedure's invocation). Valid levels run 1..=current.
                Ok(req) => {
                    let cur = i64::try_from(vm.current_level()).unwrap_or(0);
                    let abs = if req > 0 { req } else { cur + req };
                    if abs >= 1 && abs <= cur {
                        match vm.frame_argv(usize::try_from(abs).unwrap_or(0)) {
                            Some(av) => ok(Value::list(av)),
                            None => err(format!("bad level \"{}\"", n.to_str())),
                        }
                    } else {
                        err(format!("bad level \"{}\"", n.to_str()))
                    }
                }
                _ => err(format!("bad level \"{}\"", n.to_str())),
            },
            _ => err("wrong # args: should be \"info level ?number?\""),
        },
        "commands" => ok(filtered(
            vm.command_names(),
            rest.first().map(Value::to_str).as_deref(),
        )),
        "procs" => ok(filtered(
            vm.proc_names(),
            rest.first().map(Value::to_str).as_deref(),
        )),
        "vars" | "locals" => ok(filtered(
            vm.local_scalar_names(),
            rest.first().map(Value::to_str).as_deref(),
        )),
        "globals" => ok(filtered(
            vm.global_names(),
            rest.first().map(Value::to_str).as_deref(),
        )),
        "body" => match rest {
            [name] => match vm.proc_def(&name.to_str()) {
                Some(p) => ok(p.body_src.clone()),
                None => err(format!("\"{}\" isn't a procedure", name.to_str())),
            },
            _ => err("wrong # args: should be \"info body procname\""),
        },
        "args" => match rest {
            [name] => match vm.proc_def(&name.to_str()) {
                Some(p) => ok(Value::list(
                    p.params
                        .iter()
                        .map(|pp| Value::string(pp.name.as_str()))
                        .collect(),
                )),
                None => err(format!("\"{}\" isn't a procedure", name.to_str())),
            },
            _ => err("wrong # args: should be \"info args procname\""),
        },
        "default" => match rest {
            [name, arg, var] => match vm.proc_def(&name.to_str()) {
                Some(p) => {
                    let an = arg.to_str();
                    match p.params.iter().find(|pp| pp.name == *an) {
                        Some(pp) => {
                            if let Some(d) = &pp.default {
                                if let Err(e) = vm.set_var(&var.to_str(), d.clone()) {
                                    return e;
                                }
                                ok(Value::bool(true))
                            } else {
                                if let Err(e) = vm.set_var(&var.to_str(), Value::empty()) {
                                    return e;
                                }
                                ok(Value::bool(false))
                            }
                        }
                        None => err(format!(
                            "procedure \"{}\" doesn't have an argument \"{an}\"",
                            name.to_str()
                        )),
                    }
                }
                None => err(format!("\"{}\" isn't a procedure", name.to_str())),
            },
            _ => err("wrong # args: should be \"info default procname arg varname\""),
        },
        "tclversion" => ok(Value::string("9.0")),
        "patchlevel" => ok(Value::string("9.0.0")),
        "script" => ok(Value::string(vm.current_script())),
        "nameofexecutable" => ok(Value::empty()),
        other => err(format!("unknown or ambiguous subcommand \"{other}\"")),
    }
}
