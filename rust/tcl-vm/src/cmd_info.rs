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

/// `info complete`: whether `script` has no unbalanced `{}`/`[]`/`"` and does
/// not end in a line continuation — i.e. it is a syntactically complete command.
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
        // `info exists varName` — the shared Family-B core over `VarStore::exists`.
        "exists" => match rest {
            [name] => ok(tcl_cmd_core::info::exists(vm, name)),
            _ => err("wrong # args: should be \"info exists varName\""),
        },
        "complete" => match rest {
            [script] => ok(Value::bool(tcl_cmd_core::info::complete(
                script.to_str().as_bytes(),
            ))),
            _ => err("wrong # args: should be \"info complete command\""),
        },
        // `info level ?number?` — the shared Family-B core over `Introspect`
        // (`tcl_cmd_core::info::level`); the VM is a thin adapter mapping
        // `Result<Value, CmdError>` onto its completion ABI.
        "level" => {
            let number = match rest {
                [] => None,
                [n] => Some(n),
                _ => return err("wrong # args: should be \"info level ?number?\""),
            };
            match tcl_cmd_core::info::level(vm, number) {
                Ok(v) => ok(v),
                Err(e) => err(e.message()),
            }
        }
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
