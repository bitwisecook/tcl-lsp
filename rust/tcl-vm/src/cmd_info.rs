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

mod native_jim;
#[cfg(test)]
mod native_jim_inventory_tests;
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
    info_vars => "vars", info_version => "version",
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
    "version",
];

/// `info`'s implementation namespace — the `ns_fqn` an empty ensemble's miss
/// message would name (`TclMakeEnsemble`, `tclBasic.c`).
const INFO_NS: &[u8] = b"::tcl::info";

/// Resolve an `info` subcommand word to its canonical Tcl 9 name through the
/// shared ensemble owner: an exact match wins, otherwise a unique prefix — so
/// `info command` resolves to `commands` (cmdAH.test). `None` when the word
/// matches nothing or prefixes several; the caller then reports the miss.
fn canonical_info_sub<'a>(subs: &[&'a str], sub: &[u8]) -> Option<&'a str> {
    tcl_cmd_core::ensemble::resolve_subcommand(subs, sub, true).map(|index| subs[index])
}

#[allow(clippy::too_many_lines)] // One subcommand-dispatch match; splitting obscures it.
fn cmd_info(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let subs = crate::environment::release_subcommands(
        vm.actual_native_execution_profile().name,
        "info",
        INFO_SUBS,
    );
    let (canon, rest) = if let Some(protocol) = vm
        .actual_native_invocation_dialect()
        .native_jim_info_protocol()
    {
        let Some(head) = vm.invoked_name_value() else {
            return vm.refuse_host_command("original Jim info invocation is unavailable".into());
        };
        let mut original = Vec::with_capacity(args.len() + 1);
        original.push(head);
        original.extend_from_slice(args);
        let dispatch = match protocol.dispatch(original.len(), |index| {
            vm.native_name_operand_bytes(&original[index])
        }) {
            Ok(dispatch) => dispatch,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        match dispatch {
            tcl_registry::commands::tcl::NativeJimInfoDispatch::Member {
                name,
                arguments,
                scope,
                ..
            } => {
                if let Some(kind) = protocol.command_inventory_kind(name) {
                    return native_jim::command_inventory(
                        vm,
                        scope,
                        &original[arguments - 1..],
                        kind,
                    );
                }
                (name, &args[arguments - 1..])
            }
            tcl_registry::commands::tcl::NativeJimInfoDispatch::Report(report) => {
                return native_jim::report(vm, protocol, report, &original);
            }
        }
    } else {
        let Some((sub, rest)) = args.split_first() else {
            return crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"info subcommand ?arg ...?\"",
            );
        };
        let sub_bytes = match vm.native_name_operand_bytes(sub) {
            Ok(bytes) => bytes,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        let Some(canon) = canonical_info_sub(subs, &sub_bytes) else {
            return err(tcl_cmd_core::ensemble::unknown_subcommand_message(
                subs, &sub_bytes, true, INFO_NS,
            ));
        };
        (canon, rest)
    };
    match canon {
        "alias" => match rest {
            [original_name] => match tcl_cmd_core::info::jim_original_alias(vm, original_name) {
                Ok(original) => ok(original),
                Err(error) => completion_from_cmd_error(vm, error),
            },
            _ => vm.refuse_host_command("selected Jim alias arity".into()),
        },
        // `info exists varName` — the shared Family-B core over `VarStore::exists`.
        "exists" => match rest {
            [name] => {
                if vm.native_c_variable_name_protocol().is_some() {
                    return match vm.exists_original_c_parts(name, None) {
                        Ok(found) => match vm.original_existence_result(found) {
                            Ok(value) => ok(value),
                            Err(failure) => failure,
                        },
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
                if vm.observed_name_policy_selected() {
                    return match vm.observed_variable_exists(&name, vm.current_level()) {
                        Ok(found) => ok(Value::bool(found)),
                        Err(error) => vm.refuse_host_command(format!(
                            "observed variable storage is unavailable: {error}"
                        )),
                    };
                }
                ok(Value::bool(vm.exists_var_traced_bytes(&name)))
            }
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"info exists varName\"",
            ),
        },
        "complete" => match rest {
            [script] => match vm.native_name_operand_bytes(script) {
                Ok(bytes) => ok(Value::bool(tcl_cmd_core::info::complete(&bytes))),
                Err(error) => vm.refuse_host_command(error.to_string()),
            },
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
            [name] => match vm.native_name_operand_bytes(name) {
                Ok(bytes) => ok(Value::bool(vm.is_constant(&bytes))),
                Err(error) => vm.refuse_host_command(error.to_string()),
            },
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
        "tclversion" | "patchlevel" | "version" => info_version_report(vm, rest, canon),
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
        "functions" => info_functions_impl(vm, rest),
        // The empty original list denotes this interpreter. Each reached child
        // element uses the shared original-list/C-string path owner.
        "loaded" => match rest {
            [] => ok(Value::empty()),
            [path] | [path, _] => match vm.resolve_interp_path_original(path) {
                Ok(_) => ok(Value::empty()),
                Err(error) => error,
            },
            _ => crate::command::native_wrong_args(vm, "info loaded ?interp? ?prefix?"),
        },
        // `info cmdtype commandName` — native / proc / alias (interp/object kinds
        // need those subsystems).
        "cmdtype" => match rest {
            [name] => {
                let bytes = match vm.native_name_operand_bytes(name) {
                    Ok(bytes) => bytes,
                    Err(error) => return vm.refuse_host_command(error.to_string()),
                };
                match vm.command_kind_bytes_checked(&bytes) {
                    Ok(Some(kind)) => ok(Value::string(kind)),
                    Ok(None) => completion_from_cmd_error(
                        vm,
                        tcl_cmd_core::CmdError::new_bytes(
                            [b"unknown command \"".as_slice(), &bytes, b"\""].concat(),
                        ),
                    ),
                    Err(error) => vm.refuse_host_command(error.to_string()),
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
                [path] => match vm.resolve_interp_path_original(path) {
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
        _ if vm
            .actual_native_invocation_dialect()
            .native_jim_info_member_names()
            .is_some() =>
        {
            vm.refuse_host_command("selected Jim info handler is unavailable".into())
        }
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

fn info_functions_impl(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.len() > 1 {
        return crate::command::native_wrong_args(vm, "info functions ?pattern?");
    }
    let Some(recipe) = tcl_registry::mathfunc::NativeInfoFunctionsRecipe::select(
        vm.actual_native_invocation_dialect(),
    ) else {
        return vm.refuse_host_command("selected math function information recipe".into());
    };
    let pattern = match args
        .first()
        .map(|word| vm.native_name_operand_bytes(word))
        .transpose()
    {
        Ok(pattern) => pattern,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    if let Some(bytes) = recipe.script(pattern.as_deref()) {
        let script = Value::new_native_string_bytes(bytes);
        return match vm.eval_original_script_value(
            &script,
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            None,
        ) {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(vm, error),
        };
    }
    let mut names = Vec::new();
    for name in vm.math_function_names() {
        if let Some(pattern) = &pattern {
            match vm.native_namespace_match(
                tcl_syntax::native_glob::NativeNameGlobPurpose::InfoFunctions84Scan,
                pattern,
                name.as_bytes(),
            ) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            }
        }
        names.push(Value::new_native_string_bytes(name.into_bytes()));
    }
    ok(Value::list(names))
}

fn info_version_report(vm: &mut Vm, rest: &[Value], member: &str) -> Completion<Value> {
    let usage = format!("info {member}");
    if !rest.is_empty() {
        return crate::command::native_wrong_args(vm, &usage);
    }
    match vm
        .actual_native_invocation_dialect()
        .native_info_version_source(member)
    {
        Some(tcl_registry::native_info_version::NativeInfoVersionSource::Global(name)) => {
            info_global(vm, rest, &usage, name)
        }
        Some(tcl_registry::native_info_version::NativeInfoVersionSource::CoreRelease(version)) => {
            ok(Value::new_native_string_bytes(version.as_bytes()))
        }
        None => {
            vm.refuse_host_command("native info version reporting source is unavailable".into())
        }
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
    use crate::Code;
    use tcl_dialect::TclVersion;

    fn info(vm: &mut Vm, subcommand: &str) -> Completion<Value> {
        cmd_info(vm, &[Value::string(subcommand)])
    }

    #[test]
    fn original_info_functions_follow_selected_native_script_and_helpers() {
        // Native controls: naming.info.functions-native-script
        // docs/design/analysis/name-resolution-proofs/info-functions-native-script.md
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = crate::environment::profile_for_dialect(engine);
            let mut vm = crate::native_fixture::core(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let completion = vm
                .eval_source("info functions sin")
                .unwrap_or_else(|error| panic!("{engine}/info functions sin: {error:?}"));
            assert_eq!(completion.code, Code::Ok, "{engine}: {completion:?}");
            assert_eq!(
                vm.native_name_operand_bytes(&completion.result)
                    .unwrap()
                    .as_ref(),
                b"sin",
                "{engine}"
            );
            let completion = vm.eval_source("namespace eval ::InfoScope085 {namespace eval tcl::mathfunc {proc local085 {} {return LOCAL}}; info functions local085}").unwrap();
            assert_eq!(completion.code, Code::Ok, "{engine}: {completion:?}");
            let expected: &[u8] = if engine == "tcl8.4" { b"" } else { b"local085" };
            assert_eq!(
                vm.native_name_operand_bytes(&completion.result)
                    .unwrap()
                    .as_ref(),
                expected,
                "{engine}"
            );
            if engine != "tcl8.4" {
                let completion = vm.eval_source("rename ::apply ::SavedInfoApply085; proc ::apply args {error APPLY_HELPER085}; info functions sin").unwrap();
                assert_eq!(completion.code, Code::Error, "{engine}: {completion:?}");
                assert_eq!(
                    vm.native_name_operand_bytes(&completion.result)
                        .unwrap()
                        .as_ref(),
                    b"APPLY_HELPER085",
                    "{engine}"
                );
            }
        }
    }

    #[test]
    fn original_info_loaded_uses_counted_interpreter_path_elements() {
        // Native controls: naming.info.loaded-original-interpreter-path
        // docs/design/analysis/name-resolution-proofs/info-loaded-original-interpreter-path.md
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = crate::environment::profile_for_dialect(engine);
            let mut vm = crate::native_fixture::core(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            for source in [
                "info loaded {}",
                "interp create InfoParent085; interp eval InfoParent085 {interp create InfoChild085}; info loaded {InfoParent085 InfoChild085}",
                "set child [binary format H* 496e666f5a65726f30383500ff]; interp create $child; info loaded $child",
            ] {
                let completion = vm.eval_source(source).unwrap();
                assert_eq!(completion.code, Code::Ok, "{engine}: {completion:?}");
                assert_eq!(
                    completion.result.resident_string_bytes().unwrap().as_ref(),
                    b"",
                    "{engine}"
                );
            }
            let completion = vm.eval_source("info loaded MissingInfo085").unwrap();
            assert_eq!(completion.code, Code::Error, "{engine}: {completion:?}");
            assert_eq!(
                completion.result.resident_string_bytes().unwrap().as_ref(),
                b"could not find interpreter \"MissingInfo085\"",
                "{engine}"
            );
        }
    }

    #[test]
    fn actual_jim_version_report_does_not_read_tcl_bootstrap_globals() {
        // Source proof: naming.info.version-source-owner
        // docs/design/analysis/name-resolution-proofs/info-version-source-owner.md
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
        assert!(vm.get_var("::tcl_patchLevel").is_none());
        for member in ["patchlevel", "version"] {
            let result = vm
                .try_invoke_command("info", &[Value::new_native_string_bytes(member.as_bytes())])
                .unwrap();
            assert_eq!(result.code, Code::Ok, "{result:?}");
            assert_eq!(
                result.result.resident_string_bytes().unwrap().as_ref(),
                b"0.84"
            );
        }
        assert_eq!(
            vm.try_invoke_command(
                "set",
                &[Value::string("::tcl_patchLevel"), Value::string("FOREIGN")]
            )
            .unwrap()
            .code,
            Code::Ok
        );
        assert_eq!(
            vm.try_invoke_command("info", &[Value::string("patchlevel")])
                .unwrap()
                .result
                .resident_string_bytes()
                .unwrap()
                .as_ref(),
            b"0.84"
        );
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
