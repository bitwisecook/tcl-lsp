//! The `info` ensemble — introspection over the retained frame/proc metadata.
//!
//! Implemented against the data the frame deliberately keeps (per-frame proc name +
//! invocation argv, `ProcDef.body_src`/`params`, the command table) so the
//! answers are correct rather than faked — this metadata must be retained or
//! the introspection answers cannot be computed.

use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("info", cmd_info);
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
        // commands/procs route through the shared namespace-aware core (over the
        // `Namespaces` enumeration rungs), which gives the VM correct qualified
        // patterns + global-scope visibility (it previously listed all keys flat).
        "commands" => ok(tcl_cmd_core::info::command_list(vm, rest.first(), false)),
        "procs" => ok(tcl_cmd_core::info::command_list(vm, rest.first(), true)),
        // vars/locals/globals route through the shared variable-listing cores
        // (namespace-aware over `Namespaces::vars_in` + the active-frame
        // `Frames::var_names`/`in_proc`). This split `vars` from `locals` (the VM
        // previously aliased them, so `info vars` in a proc dropped its links) and
        // gave `info globals` the global-namespace-only filter.
        "vars" => ok(tcl_cmd_core::info::vars(vm, rest.first())),
        "locals" => ok(tcl_cmd_core::info::locals(vm, rest.first())),
        "globals" => ok(tcl_cmd_core::info::globals(vm, rest.first())),
        // body/args/default route through the shared `info` core over the `Procs`
        // role trait; the var-write for `default` stays here (it is trace-aware).
        "body" => match rest {
            [name] => match tcl_cmd_core::info::body(vm, name) {
                Ok(v) => ok(v),
                Err(e) => err(e.into_message()),
            },
            _ => err("wrong # args: should be \"info body procname\""),
        },
        "args" => match rest {
            [name] => match tcl_cmd_core::info::args(vm, name) {
                Ok(v) => ok(v),
                Err(e) => err(e.into_message()),
            },
            _ => err("wrong # args: should be \"info args procname\""),
        },
        "default" => match rest {
            [name, arg, var] => match tcl_cmd_core::info::default(vm, name, arg) {
                Ok((val, has)) => {
                    if let Err(e) = vm.set_var(&var.to_str(), val) {
                        return e;
                    }
                    ok(Value::bool(has))
                }
                Err(e) => err(e.into_message()),
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
