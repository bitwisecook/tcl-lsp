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

//! The `info` ensemble — introspection over the retained frame/proc metadata.
//!
//! Implemented against the data the frame deliberately keeps (per-frame proc name +
//! invocation argv, `ProcDef.body_src`/`params`, the command table) so the
//! answers are correct rather than faked — this metadata must be retained or
//! the introspection answers cannot be computed.

use tcl_runtime_api::Completion;

use crate::command::completion_from_cmd_error;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

mod native_oo;

pub(crate) fn register(vm: &mut Vm) {
    if !native_ensemble_available(vm) {
        vm.register_stock_builtin("info", cmd_info);
        return;
    }
    let subs = crate::environment::release_subcommands(
        vm.actual_native_execution_profile().name,
        "info",
        INFO_SUBS,
    );
    vm.register_stock_namespace_ensemble("info", "::tcl::info", INFO_MEMBERS, subs);
    native_oo::register(vm);
}

/// Pinning a fresh interpreter changes bootstrap implementations, while user
/// replacements remain ordinary commands with their own lifecycle.
pub(crate) fn refresh_profile(vm: &mut Vm) {
    if vm.stock_native_identity("info").as_deref() != Some("info") {
        return;
    }
    for &(member, _) in INFO_MEMBERS {
        let target = format!("::tcl::info::{member}");
        if vm.stock_native_identity(&target).as_deref() == target.strip_prefix("::") {
            vm.remove_registered_command(target.trim_start_matches("::"));
        }
    }
    register(vm);
    if !native_ensemble_available(vm) {
        vm.retire_unused_stock_ensemble_namespace("info");
    }
}

fn native_ensemble_available(vm: &Vm) -> bool {
    let dialect = vm.native_invocation_dialect();
    dialect.family() == Some(tcl_dialect::model::Family::Tcl)
        && dialect
            .tcl_version
            .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_5)
}

macro_rules! info_members {
    ($($function:ident => $member:literal),+ $(,)?) => {
        const INFO_MEMBERS: &[(&str, crate::command::BuiltinFn)] = &[
            $(($member, $function)),+
        ];
        $(fn $function(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
            let mut invocation = Vec::with_capacity(args.len() + 1);
            invocation.push(Value::string($member));
            invocation.extend_from_slice(args);
            cmd_info(vm, &invocation)
        })+
    };
}

info_members! {
    info_args => "args", info_body => "body", info_class => "class",
    info_cmdcount => "cmdcount", info_cmdtype => "cmdtype", info_commands => "commands",
    info_complete => "complete", info_constant => "constant", info_consts => "consts",
    info_coroutine => "coroutine", info_default => "default", info_errorstack => "errorstack",
    info_exists => "exists", info_frame => "frame", info_functions => "functions",
    info_globals => "globals", info_hostname => "hostname", info_level => "level",
    info_library => "library", info_loaded => "loaded", info_locals => "locals",
    info_nameofexecutable => "nameofexecutable", info_object => "object",
    info_patchlevel => "patchlevel", info_procs => "procs", info_script => "script",
    info_sharedlibextension => "sharedlibextension", info_tclversion => "tclversion",
    info_vars => "vars",
}

/// `info`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it — the
/// full Tcl 9 table, so ambiguity matches C even for subcommands the VM does
/// not yet implement (those resolve, then fall through to the
/// unknown-subcommand arm).
const INFO_SUBS: &[&str] = &[
    "args",
    "body",
    "class",
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
    "stacktrace",
    "sharedlibextension",
    "tclversion",
    "vars",
];

/// `info`'s implementation namespace — the `ns_fqn` an empty ensemble's miss
/// message would name (`TclMakeEnsemble`, `tclBasic.c`).
const INFO_NS: &[u8] = b"::tcl::info";

/// Resolve an `info` subcommand word to its canonical Tcl 9 name through the
/// shared ensemble owner: an exact match wins, otherwise a unique prefix — so
/// `info command` resolves to `commands` (cmdAH.test). `None` when the word
/// matches nothing or prefixes several; the caller then reports the miss.
fn canonical_info_sub<'a>(subs: &[&'a str], sub: &str) -> Option<&'a str> {
    tcl_cmd_core::ensemble::resolve_subcommand(subs, sub.as_bytes(), true).map(|index| subs[index])
}

#[allow(clippy::too_many_lines)] // One subcommand-dispatch match; splitting obscures it.
fn cmd_info(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"info subcommand ?arg ...?\"",
        );
    };
    let sub_str = sub.to_str();
    // `cmdtype`, `constant` and `consts` are Tcl 9 (`class`, `coroutine`,
    // `errorstack` and `object` 8.6, `frame` 8.5), so the table is filtered to
    // the emulated release before the scan: on 8.6 `info cm` is `cmdcount`,
    // on 9.0 it is ambiguous with `cmdtype`.
    let subs = crate::environment::release_subcommands(
        vm.actual_native_execution_profile().name,
        "info",
        INFO_SUBS,
    );
    // A miss reports here rather than falling through with the raw word: the
    // arms below match on the canonical name, so a word the *pinned release*
    // does not have (`info cmdtype` under 8.6) would otherwise still dispatch.
    let Some(canon) = canonical_info_sub(subs, &sub_str) else {
        return err(
            String::from_utf8_lossy(&tcl_cmd_core::ensemble::unknown_subcommand_message(
                subs,
                sub_str.as_bytes(),
                true,
                INFO_NS,
            ))
            .into_owned(),
        );
    };
    match canon {
        // `info exists varName` — the shared Family-B core over `VarStore::exists`.
        "exists" => match rest {
            [name] => {
                if vm.native_c_variable_name_protocol().is_some() {
                    return match vm.exists_original_c_parts(name, None) {
                        Ok(found) => ok(Value::bool(found)),
                        Err(failure) => failure,
                    };
                }
                // `info exists` fires read traces first (a trace may create the
                // variable — tcltest's lazy `SafeFetch` constraint init relies
                // on this); a trace error does not abort the existence check.
                let name = match vm.native_name_operand_bytes(name) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        return vm
                            .refuse_host_command(format!("variable name is unavailable: {error}"));
                    }
                };
                ok(Value::bool(vm.exists_var_traced_bytes(&name)))
            }
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"info exists varName\"",
            ),
        },
        "complete" => match rest {
            [script] => ok(Value::bool(tcl_cmd_core::info::complete(
                script.to_str().as_bytes(),
            ))),
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"info complete command\"",
            ),
        },
        // `info level ?number?` — the shared Family-B core over `Introspect`
        // (`tcl_cmd_core::info::level`); the VM is a thin adapter mapping
        // `Result<Value, CmdError>` onto its completion ABI.
        "level" => {
            let number = match rest {
                [] => None,
                [n] => Some(n),
                _ => {
                    return crate::command::native_wrong_arguments_message(
                        vm,
                        "wrong # args: should be \"info level ?number?\"",
                    );
                }
            };
            let dialect = vm.actual_native_invocation_dialect();
            let result = if dialect.native_error_log_protocol().is_some()
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_6)
            {
                vm.native_info_level(number)
            } else {
                tcl_cmd_core::info::level(vm, number)
            };
            match result {
                Ok(v) => ok(v),
                Err(e) => crate::command::completion_from_cmd_error(vm, e),
            }
        }
        // commands/procs route through the shared namespace-aware core (over the
        // `Namespaces` enumeration rungs), which gives the VM correct qualified
        // patterns + global-scope visibility.
        "commands" => match tcl_cmd_core::info::command_list(vm, rest.first(), false) {
            Ok(value) => ok(value),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        },
        "procs" => match tcl_cmd_core::info::command_list(vm, rest.first(), true) {
            Ok(value) => ok(value),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        },
        // vars/locals/globals route through the shared variable-listing cores
        // (namespace-aware over `Namespaces::vars_in` + the active-frame
        // `Frames::var_names`/`in_proc`). This splits `vars` from `locals` (aliasing
        // them would drop `info vars`'s links in a proc) and
        // gives `info globals` the global-namespace-only filter.
        "vars" => match tcl_cmd_core::info::vars(vm, rest.first()) {
            Ok(value) => ok(value),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        },
        "locals" => match tcl_cmd_core::info::locals(vm, rest.first()) {
            Ok(value) => ok(value),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        },
        "globals" => match tcl_cmd_core::info::globals(vm, rest.first()) {
            Ok(value) => ok(value),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        },
        // `info constant name` — whether `name` is a `const`; `info consts
        // ?pattern?` — the constant names in scope (glob-filtered).
        "constant" => match rest {
            [name] => ok(Value::bool(vm.is_constant(&name.to_str()))),
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"info constant varname\"",
            ),
        },
        "consts" => match rest {
            [] | [_] => match tcl_cmd_core::info::consts(vm, rest.first()) {
                Ok(value) => ok(value),
                Err(error) => completion_from_cmd_error(vm, error),
            },
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"info consts ?pattern?\"",
            ),
        },
        // body/args/default route through the shared `info` core over the `Procs`
        // role trait; the var-write for `default` stays here (it is trace-aware).
        "body" => match rest {
            [name] => match tcl_cmd_core::info::body(vm, name) {
                Ok(v) => ok(v),
                Err(e) => crate::command::completion_from_cmd_error(vm, e),
            },
            _ => crate::command::native_wrong_args(vm, "info body procname"),
        },
        "args" => match rest {
            [name] => match tcl_cmd_core::info::args(vm, name) {
                Ok(v) => ok(v),
                Err(e) => crate::command::completion_from_cmd_error(vm, e),
            },
            _ => crate::command::native_wrong_args(vm, "info args procname"),
        },
        "default" => match rest {
            [name, arg, var] => match tcl_cmd_core::info::default(vm, name, arg) {
                Ok((val, has)) => {
                    let var = match vm.native_name_operand_bytes(var) {
                        Ok(name) => name,
                        Err(error) => {
                            return vm.refuse_host_command(format!(
                                "default output name is unavailable: {error}"
                            ));
                        }
                    };
                    if let Err(error) = vm.set_var_bytes(&var, val) {
                        return error;
                    }
                    ok(Value::bool(has))
                }
                Err(e) => crate::command::completion_from_cmd_error(vm, e),
            },
            _ => crate::command::native_wrong_args(vm, "info default procname arg varname"),
        },
        "tclversion" => info_global(vm, rest, "info tclversion", "tcl_version"),
        "patchlevel" => info_global(vm, rest, "info patchlevel", "tcl_patchLevel"),
        "sharedlibextension" => match rest {
            [] => ok(Value::string(
                tcl_platform::bootstrap::SHARED_LIBRARY_EXTENSION,
            )),
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"info sharedlibextension\"",
            ),
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
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"info functions ?pattern?\"",
            ),
        },
        // `info loaded ?interp? ?prefix?` — no binary extensions are loaded, so
        // the result is empty for the current interp; a named interp must exist.
        "loaded" => {
            let interp = match rest {
                [] => None,
                [i] | [i, _] => Some(i.to_str()),
                _ => {
                    return crate::command::native_wrong_arguments_message(
                        vm,
                        "wrong # args: should be \"info loaded ?interp? ?prefix?\"",
                    );
                }
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
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"info cmdtype commandName\"",
            ),
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
        "script"
            if vm
                .actual_native_invocation_dialect()
                .native_string_protocol()
                == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084) =>
        {
            if rest.len() > 1 {
                return crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"info script ?filename?\"",
                );
            }
            let context = match vm.native_jim_object_context() {
                Ok(context) => context,
                Err(error) => return crate::command::completion_from_tcl_error(vm, error.into()),
            };
            if let Some(value) = rest.first()
                && let Err(error) = context.replace_current_filename(value)
            {
                return crate::command::completion_from_tcl_error(vm, error.into());
            }
            let result = context.current_filename_object();
            context.publish_result(&result);
            ok(result)
        }
        "script" => ok(Value::string(vm.current_script())),
        "nameofexecutable" => ok(Value::empty()),
        // TclOO introspection — dispatched into the object system.
        "object" => crate::cmd_oo::info_object(vm, rest),
        "class" => crate::cmd_oo::info_class(vm, rest),
        // `info coroutine` — the running coroutine's name, or "" at top level.
        "coroutine" => match rest {
            [] => ok(crate::cmd_coro::current_coroutine(vm)),
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"info coroutine\"",
            ),
        },
        // TIP 348 (Tcl 8.6+). Availability is already filtered through the
        // registry-derived release subcommand set above; the state lives on
        // the selected interpreter, so child access uses the ordinary arena.
        "stacktrace" => match rest {
            [] => ok(vm.jim_stacktrace()),
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"info stacktrace\"",
            ),
        },
        "errorstack" => {
            let id = match rest {
                [] => None,
                [path] => match vm.resolve_interp_path(&path.to_str()) {
                    Ok(id) => Some(id),
                    Err(error) => return error,
                },
                _ => {
                    return crate::command::native_wrong_arguments_message(
                        vm,
                        "wrong # args: should be \"info errorstack ?interp?\"",
                    );
                }
            };
            match id {
                Some(id) => ok(vm.in_interp(id, |target| target.error_stack_value())),
                None => ok(vm.error_stack_value()),
            }
        }
        // Reached by a word that matched nothing, prefixed several entries, or
        // resolved to a subcommand this engine does not implement.
        other => err(
            String::from_utf8_lossy(&tcl_cmd_core::ensemble::unknown_subcommand_message(
                subs,
                other.as_bytes(),
                true,
                INFO_NS,
            ))
            .into_owned(),
        ),
    }
}

/// `info tclversion`/`patchlevel` read their live global, as C does with
/// `TCL_GLOBAL_ONLY`. This keeps a selected release's startup values visible,
/// while still honouring user writes and unsets.
fn info_global(vm: &mut Vm, rest: &[Value], usage: &str, name: &str) -> Completion<Value> {
    if !rest.is_empty() {
        return crate::command::native_wrong_args(vm, usage);
    }
    vm.get_var(&format!("::{name}")).map_or_else(
        || err(format!("can't read \"{name}\": no such variable")),
        ok,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;

    fn info(vm: &mut Vm, subcommand: &str) -> Completion<Value> {
        cmd_info(vm, &[Value::string(subcommand)])
    }

    #[test]
    fn release_info_reads_the_selected_runtime_globals() {
        for (version, expected_version, expected_patchlevel) in [
            (TclVersion::V8_4, "8.4", "8.4.20"),
            (TclVersion::V8_5, "8.5", "8.5.19"),
            (TclVersion::V8_6, "8.6", "8.6.18"),
            (TclVersion::V9_0, "9.0", "9.0.4"),
            (TclVersion::V9_1, "9.1", "9.1.0"),
        ] {
            let mut vm = Vm::new();
            vm.set_runtime_version(version);
            // TP: each runtime surface reports its own version table entry.
            assert_eq!(
                info(&mut vm, "tclversion").result.to_str().as_ref(),
                expected_version
            );
            assert_eq!(
                info(&mut vm, "patchlevel").result.to_str().as_ref(),
                expected_patchlevel
            );
        }

        let mut vm = Vm::new();
        vm.set_runtime_version(TclVersion::V8_6);
        vm.set_var("::tcl_version", Value::string("override"))
            .expect("set release global");
        // FP guard: the command reads Tcl's live global, rather than a second
        // hard-coded release value.
        assert_eq!(
            info(&mut vm, "tclversion").result.to_str().as_ref(),
            "override"
        );
        assert!(vm.unset_var("::tcl_patchLevel"));
        // FN: a missing release global has C Tcl's normal variable error.
        let missing = info(&mut vm, "patchlevel");
        assert!(!missing.code.is_ok());
        assert_eq!(
            missing.result.to_str().as_ref(),
            "can't read \"tcl_patchLevel\": no such variable"
        );
    }

    #[test]
    fn errorstack_surface_follows_the_selected_runtime_release() {
        let mut vm = Vm::new();
        vm.set_runtime_version(TclVersion::V8_5);
        assert!(!info(&mut vm, "errorstack").code.is_ok());

        vm.set_runtime_version(TclVersion::V8_6);
        let result = info(&mut vm, "errorstack");
        assert!(result.code.is_ok());
        assert!(result.result.as_list().expect("TIP 348 list").is_empty());
    }

    #[test]
    fn shared_library_extension_comes_from_the_platform_owner() {
        let mut vm = Vm::new();
        assert_eq!(
            info(&mut vm, "sharedlibextension").result.to_str().as_ref(),
            tcl_platform::bootstrap::SHARED_LIBRARY_EXTENSION
        );
    }

    #[test]
    fn math_enumeration_follows_the_registry_selected_surface() {
        let mut old = Vm::new();
        old.set_runtime_version(TclVersion::V8_6);
        // FN: a Tcl 9 builtin is hidden throughout both `info` enumeration
        // paths on the Tcl 8.6 surface.
        assert!(
            old.math_function_names()
                .iter()
                .all(|name| name != "isfinite")
        );
        assert!(
            old.names_directly_in("tcl::mathfunc", false)
                .iter()
                .all(|name| name != "isfinite")
        );
        // TN: an older builtin still remains visible.
        assert!(old.math_function_names().iter().any(|name| name == "sin"));

        let mut modern = Vm::new();
        modern.set_runtime_version(TclVersion::V9_0);
        // TP: its release floor exposes the builtin consistently in both lists.
        assert!(
            modern
                .math_function_names()
                .iter()
                .any(|name| name == "isfinite")
        );
        assert!(
            modern
                .names_directly_in("tcl::mathfunc", false)
                .iter()
                .any(|name| name == "isfinite")
        );

        let mut fixed = Vm::new();
        fixed.set_runtime_version(TclVersion::V8_4);
        // Tcl 8.4's `info functions` sees the registry's fixed table, although
        // the command wrappers themselves are not part of that release.
        assert!(fixed.math_function_names().iter().any(|name| name == "sin"));
        assert!(
            fixed
                .names_directly_in("tcl::mathfunc", false)
                .iter()
                .all(|name| name != "sin")
        );
    }
}

#[cfg(test)]
#[path = "cmd_info/native_commands_tests.rs"]
mod native_commands_tests;
