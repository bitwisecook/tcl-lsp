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

/// Resolve an `info` subcommand word to its canonical Tcl 9 name with Tcl's
/// unambiguous-prefix rule (`Tcl_GetIndexFromObj`): an exact match wins,
/// otherwise a unique prefix — so `info command` resolves to `commands`
/// (cmdAH.test). Returns `None` when the word matches nothing or is an
/// ambiguous prefix of several; the caller then reports the error. The table is
/// the full Tcl 9 `info` option set so ambiguity matches C even for
/// subcommands the VM does not yet implement (those resolve then fall through
/// to the unknown-subcommand arm).
fn canonical_info_sub(sub: &str) -> Option<&'static str> {
    const SUBS: &[&str] = &[
        "args",
        "body",
        "cmdcount",
        "cmdtype",
        "commands",
        "complete",
        "constant",
        "consts",
        "coroutine",
        "default",
        "errorstack",
        "exists",
        "frame",
        "functions",
        "globals",
        "hostname",
        "level",
        "library",
        "loaded",
        "locals",
        "nameofexecutable",
        "object",
        "patchlevel",
        "procs",
        "script",
        "sharedlibextension",
        "tclversion",
        "vars",
    ];
    if sub.is_empty() {
        return None;
    }
    if let Some(&exact) = SUBS.iter().find(|&&s| s == sub) {
        return Some(exact);
    }
    let mut found = None;
    let mut count = 0u32;
    for &s in SUBS {
        if s.starts_with(sub) {
            found = Some(s);
            count += 1;
        }
    }
    if count == 1 { found } else { None }
}

#[allow(clippy::too_many_lines)]
fn cmd_info(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"info subcommand ?arg ...?\"");
    };
    let sub_str = sub.to_str();
    let canon: &str = match canonical_info_sub(&sub_str) {
        Some(c) => c,
        None => &sub_str,
    };
    match canon {
        // `info exists varName` — the shared Family-B core over `VarStore::exists`.
        "exists" => match rest {
            [name] => {
                // `info exists` fires read traces first (a trace may create the
                // variable — tcltest's lazy `SafeFetch` constraint init relies
                // on this); a trace error does not abort the existence check.
                let _ = vm.fire_var_traces(&name.to_str(), "read");
                ok(tcl_cmd_core::info::exists(vm, name))
            }
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
        // `info constant name` — whether `name` is a `const`; `info consts
        // ?pattern?` — the constant names in scope (glob-filtered).
        "constant" => match rest {
            [name] => ok(Value::bool(vm.is_constant(&name.to_str()))),
            _ => err("wrong # args: should be \"info constant varname\""),
        },
        "consts" => {
            let names = vm.constant_names().into_iter().filter(|n| {
                rest.first()
                    .is_none_or(|p| tcl_syntax::glob::string_match(&p.to_str(), n))
            });
            ok(Value::list(names.map(Value::string).collect()))
        }
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
        "tclversion" => match rest {
            [] => ok(Value::string("9.0")),
            _ => err("wrong # args: should be \"info tclversion\""),
        },
        "patchlevel" => match rest {
            [] => ok(Value::string("9.0.3")),
            _ => err("wrong # args: should be \"info patchlevel\""),
        },
        // `info functions ?pattern?` — the registered `tcl::mathfunc::*` names.
        "functions" => match rest {
            [] => ok(Value::list(
                vm.math_function_names()
                    .into_iter()
                    .map(Value::string)
                    .collect(),
            )),
            [pat] => {
                let p = pat.to_str();
                ok(Value::list(
                    vm.math_function_names()
                        .into_iter()
                        .filter(|n| tcl_syntax::glob::string_match(&p, n))
                        .map(Value::string)
                        .collect(),
                ))
            }
            _ => err("wrong # args: should be \"info functions ?pattern?\""),
        },
        // `info loaded ?interp? ?prefix?` — no binary extensions are loaded, so
        // the result is empty for the current interp; a named interp must exist.
        "loaded" => {
            let interp = match rest {
                [] => None,
                [i] | [i, _] => Some(i.to_str()),
                _ => return err("wrong # args: should be \"info loaded ?interp? ?prefix?\""),
            };
            match interp {
                Some(i) if !i.is_empty() => err(format!("could not find interpreter \"{i}\"")),
                _ => ok(Value::empty()),
            }
        }
        // `info cmdtype commandName` — native / proc / alias (interp/object kinds
        // need those subsystems).
        "cmdtype" => match rest {
            [name] => {
                let n = name.to_str();
                match vm.command_kind(&n) {
                    Some(kind) => ok(Value::string(kind)),
                    None => err(format!("unknown command \"{n}\"")),
                }
            }
            _ => err("wrong # args: should be \"info cmdtype commandName\""),
        },
        // `info library` is the script library directory — the `::tcl_library`
        // global the bootstrap seeds from `$env(TCL_LIBRARY)`. Read it as a
        // global (the `::` prefix): C's `info library` returns the global
        // regardless of the calling frame, so library procs (`::tcl::tm::path`,
        // the Safe Base) reach it from inside a namespace/proc.
        "library" => match vm.get_var("::tcl_library") {
            Some(v) if !v.to_str().is_empty() => ok(v),
            _ => err("no library has been specified for Tcl"),
        },
        "script" => ok(Value::string(vm.current_script())),
        "nameofexecutable" => ok(Value::empty()),
        other => err(format!("unknown or ambiguous subcommand \"{other}\"")),
    }
}
