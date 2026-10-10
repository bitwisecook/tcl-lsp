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

//! The namespace command adapter over retained namespace tokens and native byte names.
//!
//! Namespace operands use the selected engine's purpose-specific extent and
//! qualification. Original values survive script assembly, lookup, reporting,
//! usage rewriting and callback dispatch. Jim's namespace objects remain flat
//! objects independently of C namespace-tree components.

use tcl_registry::native_eval_object::EvalObjectPurpose;
use tcl_runtime_api::completion_options::ControlOptionPolicy;
use tcl_runtime_api::{Code, Completion};

use crate::command::{err_with_code, settle_control_options};
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use tcl_dialect::model::surface_admits;

/// Run `body` as a script in namespace `target`, absorbing a top-level
/// `return` at the boundary (a namespace body completes like a proc body).
///
/// The body runs in its own call frame (like a proc) so `info level` counts it
/// and `uplevel`/`upvar` from a proc called within reach it (and its namespace
/// variables). `call_argv` is the invoking command (e.g. `namespace eval ::ns
/// {…}`) for `info level N`.
fn eval_in_ns(
    vm: &mut Vm,
    original: &Value,
    body: &Value,
    purpose: EvalObjectPurpose,
    call_argv: Vec<Value>,
    location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    create: bool,
) -> Completion<Value> {
    // Native eval first resolves the original namespace object. Creation on a
    // miss uses its written spelling, without installing a cache for that miss.
    let token = match vm.namespace_object_lookup(original) {
        Ok(Some(token)) => token,
        Ok(None) => {
            let written = match vm.native_name_operand_bytes(original) {
                Ok(bytes) => bytes,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            };
            if create {
                match vm.activate_namespace_operand(&written) {
                    Ok(token) => token,
                    Err(completion) => return completion,
                }
            } else {
                return vm.namespace_lookup_error_at(
                    &written,
                    tcl_syntax::naming::NativeNamespaceLookupOperation::Inscope,
                );
            }
        }
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    vm.push_ns_eval_token_frame(token, call_argv);
    vm.enter_ns_script();
    let result = vm.eval_original_script_value(body, purpose, location);
    vm.leave_ns_script();
    vm.pop_ns();
    vm.pop_call_frame();
    let completion = match result {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    };
    settle_control_options(completion, ControlOptionPolicy::FRESH_FORWARDED)
}

pub(crate) fn register(vm: &mut Vm) {
    let Some(namespace) = vm
        .native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Namespace)
    else {
        vm.register_stock_builtin("namespace", cmd_namespace);
        return;
    };
    let subs = crate::environment::release_subcommands(
        vm.runtime_version().dialect_profile_name(),
        "namespace",
        NAMESPACE_SUBS,
    );
    vm.register_stock_namespace_ensemble("namespace", namespace, NAMESPACE_MEMBERS, subs);
}

pub(crate) fn refresh_profile(vm: &mut Vm) {
    if vm.stock_native_identity("namespace").as_deref() != Some("namespace") {
        return;
    }
    for &(member, _) in NAMESPACE_MEMBERS {
        let target = format!("::tcl::namespace::{member}");
        if vm.stock_native_identity(&target).as_deref() == target.strip_prefix("::") {
            vm.remove_registered_command(target.trim_start_matches("::"));
        }
    }
    register(vm);
    if vm
        .native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Namespace)
        .is_none()
    {
        vm.retire_unused_stock_ensemble_namespace("namespace");
    }
}

macro_rules! namespace_members {
    ($($function:ident => $member:literal),+ $(,)?) => {
        const NAMESPACE_MEMBERS: &[(&str, crate::command::BuiltinFn)] = &[
            $(($member, $function)),+
        ];
        const NAMESPACE_SUBS: &[&str] = &[$($member),+];
        $(fn $function(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
            let mut invocation = Vec::with_capacity(args.len() + 1);
            invocation.push(Value::string($member));
            invocation.extend_from_slice(args);
            cmd_namespace_in(vm, &invocation, NamespaceInvocation::Worker)
        })+
    };
}

namespace_members! {
    namespace_children => "children", namespace_code => "code",
    namespace_current => "current", namespace_delete => "delete",
    namespace_ensemble => "ensemble", namespace_eval => "eval",
    namespace_exists => "exists", namespace_export => "export",
    namespace_forget => "forget", namespace_import => "import",
    namespace_inscope => "inscope", namespace_origin => "origin",
    namespace_parent => "parent", namespace_path => "path",
    namespace_qualifiers => "qualifiers", namespace_tail => "tail",
    namespace_unknown => "unknown", namespace_upvar => "upvar",
    namespace_which => "which",
}

fn namespace_current_invocation(
    vm: &mut Vm,
    args: &[Value],
    invocation: NamespaceInvocation,
) -> Completion<Value> {
    if args.is_empty() {
        return match tcl_cmd_core::namespace::current_original(vm) {
            Ok(value) => ok(value),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        };
    }
    let usage = match namespace_usage_header(vm, "current", invocation) {
        Ok(usage) => usage,
        Err(refusal) => return refusal,
    };
    match tcl_cmd_core::namespace::current_original_with_arguments(vm, args, &usage) {
        Ok(value) => ok(value),
        Err(error) => crate::command::completion_from_cmd_error(vm, error),
    }
}

fn namespace_usage_header(
    vm: &mut Vm,
    member: &str,
    invocation: NamespaceInvocation,
) -> Result<Vec<u8>, Completion<Value>> {
    let mut header = vec![
        vm.invoked_name_value()
            .unwrap_or_else(|| Value::string("namespace")),
    ];
    if matches!(invocation, NamespaceInvocation::Public) {
        header.push(Value::string(member));
    }
    let usage = tcl_cmd_core::ensemble::rewrite_argument_usage(
        &header,
        &vm.native_invocation.usage_rewrites,
    );
    vm.native_argument_usage_header(&usage)
}

fn namespace_code_invocation(
    vm: &mut Vm,
    args: &[Value],
    invocation: NamespaceInvocation,
) -> Completion<Value> {
    let [script] = args else {
        let mut usage = match namespace_usage_header(vm, "code", invocation) {
            Ok(usage) => usage,
            Err(refusal) => return refusal,
        };
        usage.extend_from_slice(b" arg");
        return crate::command::native_wrong_args_bytes(vm, &usage);
    };
    let Some(policy) = vm
        .native_invocation_dialect()
        .namespace_code_handler_policy(Some(
            tcl_registry::native_namespace_code::LogicalNamespaceCodeProvider::Tcl84CoreSimulation,
        ))
    else {
        return vm
            .refuse_host_command("namespace code native handler policy is not selected".into());
    };
    let bytes = match vm.native_name_operand_bytes(script) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    if policy.preserves_argument(&bytes) {
        return ok(script.clone());
    }
    let context = match vm.native_namespace_result_object(
        vm.current_ns_id(),
        tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer::CodeContext,
    ) {
        Ok(context) => context,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    ok(tcl_syntax::value::ValueOps::new_list(
        vm,
        vec![
            Value::string("::namespace"),
            Value::string("inscope"),
            context,
            script.clone(),
        ],
    ))
}

#[derive(Clone, Copy)]
enum NamespaceInvocation {
    Public,
    Worker,
}

fn cmd_namespace(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    cmd_namespace_in(vm, args, NamespaceInvocation::Public)
}

#[allow(clippy::too_many_lines)] // One subcommand-dispatch match; splitting obscures it.
fn cmd_namespace_in(
    vm: &mut Vm,
    args: &[Value],
    invocation: NamespaceInvocation,
) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace subcommand ?arg ...?\"",
        );
    };
    let original_sub = match vm.native_name_operand_bytes(sub) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let Some(name_policy) = vm.name_policy_protocol() else {
        return vm
            .refuse_host_command("native namespace subcommand protocol is unavailable".into());
    };
    let selected_sub = name_policy
        .recipe()
        .namespace_subcommand_input(&original_sub);
    // Availability follows the interpreter's command surface. An embedding
    // host may provide broader private machinery than the source grammar.
    let profile = vm.command_surface_profile();
    let dialect = Some(crate::environment::surface_point(profile));
    let registry = tcl_registry::default_registry();
    let spec = registry
        .get_for_surface("namespace", dialect)
        .expect("the core namespace command is registered for every Tcl release");
    let Some(subcommand) =
        spec.resolve_subcommand_bytes_for_dialect(selected_sub.selected(), dialect)
    else {
        let available: Vec<&str> = spec
            .subcommands
            .iter()
            .filter(|candidate| {
                candidate
                    .surface
                    .or(spec.surface)
                    .is_none_or(|gate| surface_admits(gate, dialect.as_ref()))
            })
            .map(|candidate| candidate.name)
            .collect();
        // The original dispatcher owns its noun, ambiguity and enumeration;
        // older C releases use option lookup rather than the modern ensemble.
        return err(tcl_cmd_core::namespace::unknown_subcommand_message(
            name_policy.recipe(),
            &available,
            selected_sub.selected(),
        ));
    };
    if matches!(
        subcommand.name,
        "exists" | "parent" | "children" | "tail" | "qualifiers"
    ) && !subcommand
        .arity
        .accepts(u16::try_from(rest.len()).unwrap_or(u16::MAX))
    {
        return crate::command::completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::wrong_args(subcommand.synopsis),
        );
    }
    match subcommand.name {
        "canonical" => {
            let (prefix, name) = match rest {
                [] => {
                    return ok(Value::from_native_string_bytes(
                        vm.namespace_object_bytes(vm.current_ns_id()).as_bytes(),
                    ));
                }
                [name] => (None, name),
                [prefix, name] => (Some(prefix), name),
                _ => {
                    return crate::command::native_wrong_arguments_message(
                        vm,
                        "wrong # args: should be \"namespace canonical ?current? ?name?\"",
                    );
                }
            };
            let name = match vm.native_name_operand_bytes(name) {
                Ok(bytes) => bytes,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            };
            let prefix = match prefix
                .map(|value| vm.native_name_operand_bytes(value))
                .transpose()
            {
                Ok(bytes) => bytes,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            };
            match vm.jim_canonical_namespace_bytes(prefix.as_deref(), &name) {
                Ok(bytes) => ok(Value::from_native_string_bytes(bytes)),
                Err(error) => vm.refuse_host_command(error.to_string()),
            }
        }
        "eval" => ns_eval(vm, rest),
        "current" => namespace_current_invocation(vm, rest, invocation),
        "qualifiers" => ns_text_op(vm, rest, false),
        "tail" => ns_text_op(vm, rest, true),
        // exists/parent/children route through the shared core over `Namespaces`
        // (the VM's String model honours the `NsId` handles via its arena). This
        // also gave `children` its missing `?pattern?` filter and made
        // parent/children on a missing namespace error, both matching tclsh.
        // During `TclTeardownNamespace` the namespace token is already dead
        // even though its command table stays reachable for delete callbacks.
        // The shared handle lookup intentionally still exposes that table to
        // `info commands`; this lifecycle-aware predicate distinguishes the
        // namespace-existence query.
        "exists" => {
            let Some(original) = rest.first() else {
                return crate::command::native_wrong_arguments_message(vm, subcommand.synopsis);
            };
            match vm.namespace_object_lookup(original) {
                // The native object getter itself owns lifetime validity:
                // C8.4 accepts an original cache for an activated dying token.
                Ok(namespace) => ok(Value::bool(namespace.is_some())),
                Err(error) => vm.refuse_host_command(error.to_string()),
            }
        }
        "parent" | "children" => {
            if subcommand.name == "parent" && name_policy.recipe().is_jim084() {
                let original = match rest.first() {
                    Some(original) => match vm.native_name_operand_bytes(original) {
                        Ok(bytes) => bytes,
                        Err(error) => return vm.refuse_host_command(error.to_string()),
                    },
                    None => {
                        std::rc::Rc::from(vm.namespace_object_bytes(vm.current_ns_id()).as_bytes())
                    }
                };
                return match name_policy.recipe().jim_namespace_parent_bytes(&original) {
                    Ok(bytes) => ok(Value::from_native_string_bytes(bytes)),
                    Err(error) => vm.refuse_host_command(format!("{error:?}")),
                };
            }
            let namespace = match rest.first() {
                Some(original) => match vm.namespace_object_lookup(original) {
                    Ok(Some(namespace)) => namespace,
                    Ok(None) => {
                        let written = match vm.native_name_operand_bytes(original) {
                            Ok(bytes) => bytes,
                            Err(error) => return vm.refuse_host_command(error.to_string()),
                        };
                        return vm.namespace_lookup_error_at(
                            &written,
                            if subcommand.name == "parent" {
                                tcl_syntax::naming::NativeNamespaceLookupOperation::Parent
                            } else {
                                tcl_syntax::naming::NativeNamespaceLookupOperation::Children
                            },
                        );
                    }
                    Err(error) => return vm.refuse_host_command(error.to_string()),
                },
                None => vm.current_ns_id(),
            };
            if subcommand.name == "parent" {
                match tcl_cmd_core::namespace::parent_original(vm, namespace) {
                    Ok(parent) => ok(parent),
                    Err(error) => vm.refuse_host_command(error.to_string()),
                }
            } else {
                let pattern = match rest
                    .get(1)
                    .map(|value| vm.native_name_operand_bytes(value))
                    .transpose()
                {
                    Ok(bytes) => bytes,
                    Err(error) => return vm.refuse_host_command(error.to_string()),
                };
                match tcl_cmd_core::namespace::children_tokens_checked(
                    vm,
                    namespace,
                    pattern.as_deref(),
                ) {
                    Ok(children) => match tcl_cmd_core::namespace::children_original(vm, &children)
                    {
                        Ok(value) => ok(value),
                        Err(error) => vm.refuse_host_command(error.to_string()),
                    },
                    Err(error) => vm.refuse_host_command(error.to_string()),
                }
            }
        }
        "code" => namespace_code_invocation(vm, rest, invocation),
        // `namespace inscope ns script ?arg ...?` runs `script` (with any extra
        // args appended as list elements) in namespace `ns`.
        "inscope" => ns_inscope(vm, rest),
        "which" => {
            let words = match rest
                .iter()
                .map(|word| vm.native_name_operand_bytes(word))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(words) => words,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            };
            let Some((kind, name_index)) = tcl_cmd_core::namespace::which_request(&words) else {
                return crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"namespace which ?-command? ?-variable? name\"",
                );
            };
            let result = match kind {
                tcl_cmd_core::namespace::WhichKind::Variable => {
                    tcl_cmd_core::namespace::which_variable_bytes_checked(vm, &words[name_index])
                }
                tcl_cmd_core::namespace::WhichKind::Command => {
                    vm.native_namespace_command_name(&rest[name_index], false)
                }
            };
            match result {
                Ok(bytes) => ok(bytes.map_or_else(Value::empty, Value::from_native_string_bytes)),
                Err(error) => vm.refuse_host_command(error.to_string()),
            }
        }
        "origin" => {
            if rest.len() != 1 {
                return crate::command::completion_from_cmd_error(
                    vm,
                    tcl_cmd_core::CmdError::wrong_args(subcommand.synopsis),
                );
            }
            match vm.native_namespace_command_name(&rest[0], true) {
                Ok(Some(bytes))
                    if vm
                        .actual_native_invocation_dialect()
                        .native_command_name_protocol()
                        .is_some() =>
                {
                    match vm.native_namespace_origin_result(&bytes) {
                        Ok(result) => ok(result),
                        Err(error) => vm.refuse_host_command(error.to_string()),
                    }
                }
                Ok(Some(bytes)) => ok(Value::from_native_string_bytes(bytes)),
                Ok(None)
                    if vm
                        .actual_native_invocation_dialect()
                        .native_command_name_protocol()
                        .is_some() =>
                {
                    vm.native_namespace_origin_failure(&rest[0])
                }
                Ok(None) => {
                    let name = match vm.native_name_operand_bytes(&rest[0]) {
                        Ok(name) => name,
                        Err(error) => return vm.refuse_host_command(error.to_string()),
                    };
                    namespace_command_lookup_error(
                        vm,
                        &name,
                        b"invalid command name ",
                        b"",
                        b"COMMAND",
                    )
                }
                Err(error) => vm.refuse_host_command(error.to_string()),
            }
        }

        "export" => {
            if vm.dialect_profile().namespace_import_binding()
                == Some(tcl_dialect::NamespaceImportBinding::SourceName)
            {
                ok(Value::empty())
            } else {
                ns_export(vm, rest)
            }
        }
        "import" => ns_import(vm, rest),
        // C validates all written names before retiring any selected token.
        // Jim's authored helper performs its separate sequential flat operation.
        "delete" => {
            if !name_policy.recipe().is_jim084() {
                return match tcl_cmd_core::namespace::delete_original(vm, rest) {
                    Ok(None) => ok(Value::empty()),
                    Ok(Some(index)) => {
                        let written = match vm.native_name_operand_bytes(&rest[index]) {
                            Ok(bytes) => bytes,
                            Err(error) => return vm.refuse_host_command(error.to_string()),
                        };
                        vm.namespace_lookup_error_at(
                            &written,
                            tcl_syntax::naming::NativeNamespaceLookupOperation::Delete,
                        )
                    }
                    Err(error) => vm.refuse_host_command(error.to_string()),
                };
            }
            for original in rest {
                let written = match vm.native_name_operand_bytes(original) {
                    Ok(bytes) => bytes,
                    Err(error) => return vm.refuse_host_command(error.to_string()),
                };
                match vm.delete_namespace_operand(&written) {
                    Ok(true) => {}
                    Ok(false) => {
                        return vm.namespace_lookup_error_at(
                            &written,
                            tcl_syntax::naming::NativeNamespaceLookupOperation::Delete,
                        );
                    }
                    Err(completion) => return completion,
                }
            }
            ok(Value::empty())
        }
        "path" => match rest {
            [] => {
                let mut objects = Vec::new();
                for namespace in vm.ns_path_tokens() {
                    match vm.native_namespace_result_object(
                        namespace,
                        tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer::Path,
                    ) {
                        Ok(object) => objects.push(object),
                        Err(error) => return vm.refuse_host_command(error.to_string()),
                    }
                }
                ok(tcl_syntax::value::ValueOps::new_list(vm, objects))
            }
            [list] => {
                let elems = match tcl_syntax::value::ValueOps::list_elements(vm, list) {
                    Ok(elements) => elements,
                    Err(error) => {
                        return crate::command::completion_from_cmd_error(vm, error.into());
                    }
                };
                // C resolves every entry with `TclGetNamespaceFromObj` *before*
                // installing the path (`NamespacePathCmd`), so an unresolvable
                // entry errors and leaves the old path in place.
                let mut path = Vec::with_capacity(elems.len());
                for e in &elems {
                    match vm.namespace_object_lookup(e) {
                        Ok(Some(namespace)) => path.push(namespace),
                        Ok(None) => {
                            let written = match vm.native_name_operand_bytes(e) {
                                Ok(bytes) => bytes,
                                Err(error) => return vm.refuse_host_command(error.to_string()),
                            };
                            return vm.namespace_lookup_error_bytes(&written);
                        }
                        Err(error) => return vm.refuse_host_command(error.to_string()),
                    }
                }
                vm.ns_path_set(path);
                ok(Value::empty())
            }
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"namespace path ?nsList?\"",
            ),
        },
        // `namespace forget ?pattern ...?` — remove previously imported commands
        // matching each pattern from the current namespace.
        "forget" => {
            for p in rest {
                let bytes = match vm.native_name_operand_bytes(p) {
                    Ok(bytes) => bytes,
                    Err(error) => return vm.refuse_host_command(error.to_string()),
                };
                if let Err(error) = vm.forget_imports(&bytes) {
                    return error;
                }
            }
            ok(Value::empty())
        }
        "ensemble" => ns_ensemble(vm, sub, rest),
        // `namespace unknown ?handler?` (TIP 181) — get/set the CURRENT
        // namespace's resolution-miss handler (a command prefix). Handlers
        // are per-namespace, NOT inherited by children; the global
        // namespace's handler is the interp-wide default; unset reports
        // `::unknown` (the default chain). Consulted by the dispatch miss
        // path before the plain `unknown` proc.
        "unknown" => match rest {
            [] => ok(vm.ns_unknown_get().unwrap_or_else(Value::empty)),
            [handler] => {
                let length = match tcl_syntax::value::ValueOps::list_len(vm, handler) {
                    Ok(length) => length,
                    Err(error) => {
                        return crate::command::completion_from_cmd_error(vm, error.into());
                    }
                };
                vm.ns_unknown_set(handler.clone(), length);
                ok(handler.clone())
            }
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"namespace unknown ?script?\"",
            ),
        },
        "upvar" => ns_upvar(vm, rest),
        _ => unreachable!("every registry namespace subcommand has VM dispatch"),
    }
}

/// `namespace upvar ns ?otherVar myVar ...?` — link variables in `ns` into
/// the active frame. Subcommand spelling and release availability are resolved
/// from the registry above; this function owns only the storage operation.
fn ns_upvar(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("namespace upvar naming protocol is unavailable".into());
    };
    let Some(grammar) = vm
        .actual_native_invocation_dialect()
        .native_namespace_upvar_protocol()
    else {
        return vm.refuse_host_command("namespace upvar argument grammar is unavailable".into());
    };
    let arguments = match grammar.arguments(rest.len()) {
        Ok(arguments) => arguments,
        Err(usage) => return crate::command::native_wrong_args_bytes(vm, usage),
    };
    let namespace_bytes = match vm.native_name_operand_bytes(&rest[0]) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    if policy.recipe().is_jim084() {
        let prefix = match vm.jim_namespace_upvar_target_bytes(&namespace_bytes, b"") {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
        };
        let mut argv = vec![Value::string("0")];
        for (target_word, local_word) in arguments.pairs() {
            let other = match vm.native_name_operand_bytes(&rest[target_word]) {
                Ok(bytes) => bytes,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            };
            let mut target = prefix.clone();
            target.extend_from_slice(&other);
            argv.push(Value::from_string_bytes(target));
            argv.push(local_word.map_or_else(Value::empty, |word| rest[word].clone()));
        }
        return vm.invoke_command_in_lookup_namespace("::", "upvar", &argv);
    }
    let namespace = match vm.namespace_object_lookup(&rest[0]) {
        Ok(Some(namespace)) => namespace,
        Ok(None) => return vm.namespace_lookup_error_bytes(&namespace_bytes),
        Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
    };
    for (target_word, local_word) in arguments.pairs() {
        let local_word = local_word.expect("selected C grammar has complete pairs");
        let local = match vm.native_name_operand_bytes(&rest[local_word]) {
            Ok(bytes) => bytes,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        let local = match policy.recipe().namespace_upvar_local_input(&local) {
            Ok(local) => Value::new_native_string_bytes(local.selected()),
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        if let Err(error) = vm.link_original_c_variable_objects(
            &rest[target_word],
            vm.current_level(),
            Some(namespace),
            &local,
        ) {
            return error;
        }
    }
    ok(Value::empty())
}

fn ns_export(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    if rest.is_empty() {
        return ok(Value::list(
            vm.exports_get()
                .into_iter()
                .map(|bytes| Value::from_native_string_bytes(bytes.as_bytes()))
                .collect(),
        ));
    }
    let mut values = rest;
    if let Some(first) = values.first() {
        let bytes = match vm.native_name_operand_bytes(first) {
            Ok(bytes) => bytes,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        if bytes.as_ref() == b"-clear" {
            vm.clear_exports();
            values = &values[1..];
        }
    }
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("native export protocol is unavailable".into());
    };
    if policy.recipe().is_jim084() {
        return ok(Value::empty());
    }
    let mut patterns = Vec::new();
    for value in values {
        let bytes = match vm.native_name_operand_bytes(value) {
            Ok(bytes) => bytes,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        let selected = match policy.recipe().namespace_pattern_input(
            &bytes,
            tcl_syntax::naming::NativeNamePurpose::NamespaceExportPattern,
        ) {
            Ok(selected) => tcl_core_types::NameBytes::from(selected.selected()),
            Err(_) => {
                return vm
                    .refuse_host_command("native export pattern projection is unavailable".into());
            }
        };
        patterns.push(selected);
    }
    // `NamespaceExportCmd` calls `Tcl_Export` once per pattern and returns on
    // the first failure, so the patterns before an invalid one are already
    // committed — validation is NOT a batch gate. (`-clear` is committed
    // earlier still: C spends a whole `Tcl_Export(…, "::", 1)` call on it,
    // which resets the list and then fails its own qualifier check, an error
    // `NamespaceExportCmd` deliberately discards with `Tcl_ResetResult`.)
    // An export pattern names commands in the *current* namespace, so it may
    // not carry a namespace qualifier.
    for pattern in &patterns {
        if tcl_syntax::naming::is_qualified(pattern.as_bytes()) {
            let mut message = b"invalid export pattern \"".to_vec();
            message.extend_from_slice(pattern.as_bytes());
            message.extend_from_slice(b"\": pattern can't specify a namespace");
            return crate::command::completion_from_cmd_error(
                vm,
                tcl_cmd_core::CmdError::new_bytes(message),
            );
        }
        vm.add_exports(std::slice::from_ref(pattern));
    }
    ok(Value::empty())
}

/// `namespace import ?-force? ?pattern ...?` (`NamespaceImportCmd` /
/// `Tcl_Import`, `tclNamesp.c:3668-3732`).
///
/// `-force` is positional in the same way: only `objv[1]` is read as the flag,
/// and every later word is a pattern — including a trailing `-force`, which
/// then fails the "the pattern must name a source namespace" check.
fn ns_import(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    if vm.dialect_profile().namespace_import_binding()
        == Some(tcl_dialect::NamespaceImportBinding::SourceName)
    {
        for pattern in rest {
            let original = match vm.native_name_operand_bytes(pattern) {
                Ok(bytes) => bytes,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            };
            let result = vm.import_source_names(&original);
            if result.code != Code::Ok {
                return result;
            }
        }
        return ok(Value::empty());
    }
    if rest.is_empty() {
        // The introspection form: the current namespace's imported commands.
        return ok(Value::list(
            vm.imported_command_tails()
                .into_iter()
                .map(|bytes| Value::from_native_string_bytes(bytes.as_bytes()))
                .collect(),
        ));
    }
    let mut words = Vec::new();
    for value in rest {
        match vm.native_name_operand_bytes(value) {
            Ok(bytes) => words.push(bytes),
            Err(error) => return vm.refuse_host_command(error.to_string()),
        }
    }
    let allow_overwrite = words
        .first()
        .is_some_and(|bytes| bytes.as_ref() == b"-force");
    for pattern in &words[usize::from(allow_overwrite)..] {
        let destination = tcl_runtime_api::Namespaces::current(vm);
        let Some(policy) = vm.name_policy_protocol() else {
            return vm.refuse_host_command("native import protocol is unavailable".into());
        };
        let Ok(projection) = policy.recipe().namespace_pattern_input(
            pattern,
            tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern,
        ) else {
            return vm
                .refuse_host_command("native import pattern projection is unavailable".into());
        };
        let selected = projection.selected();
        let qualifier = tcl_cmd_core::namespace::qualifiers(selected);
        if let Err(error) =
            tcl_runtime_api::Namespaces::find_namespace_bytes_checked(vm, destination, qualifier)
        {
            return crate::command::completion_from_cmd_error(
                vm,
                tcl_cmd_core::CmdError::from(error),
            );
        }
        if let Err(problem) = tcl_cmd_core::namespace::import_pattern(vm, destination, selected) {
            return err(problem.message());
        }
        if let Err(problem) = vm.import_commands(pattern, allow_overwrite) {
            return problem.completion(vm);
        }
    }
    ok(Value::empty())
}

/// `namespace ensemble create|exists|configure` (`TclNamespaceEnsembleCmd`,
/// `tclEnsemble.c:140`). The subcommand word resolves through the shared
/// `ensembleSubcommands` table, so `namespace ensemble cr` is `create` and a
/// miss reads `bad subcommand "…": must be configure, create, or exists`.
fn ns_ensemble(vm: &mut Vm, selector: &Value, rest: &[Value]) -> Completion<Value> {
    if !rest.is_empty() {
        let original = match vm.native_name_operand_bytes(selector) {
            Ok(original) => original,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        if let Some(helper) = vm
            .native_invocation_dialect()
            .native_namespace_scripted_helper(&original)
        {
            let target = Value::from_native_string_bytes(helper);
            return vm.invoke_command_value_at(
                vm.current_ns_id(),
                &target,
                rest,
                &[],
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            );
        }
    }
    if let Some(protocol) = vm
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
    {
        let lifecycle = match vm.native_namespace_name_token(vm.current_ns_id()) {
            Ok(token) => token.lifecycle(),
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        if !protocol.permits_namespace_lifecycle(lifecycle) {
            return crate::command::completion_from_cmd_error(
                vm,
                tcl_cmd_core::CmdError::with_error_code_bytes(
                    b"tried to manipulate ensemble of deleted namespace".to_vec(),
                    b"TCL ENSEMBLE DEAD".to_vec(),
                ),
            );
        }
    }

    let Some((op, args)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace ensemble subcommand ?arg ...?\"",
        );
    };
    let index = match vm.native_static_option_index(
        op,
        tcl_cmd_core::ensemble::SUBCOMMANDS.names(),
        false,
        "subcommand",
    ) {
        Ok(index) => index,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    match index {
        // configure
        0 => ns_ensemble_configure(vm, args),
        // create
        1 => ns_ensemble_create(vm, args),
        // exists
        _ => match args {
            [cmd] => {
                let bytes = match vm.native_name_operand_bytes(cmd) {
                    Ok(bytes) => bytes,
                    Err(error) => return vm.refuse_host_command(error.to_string()),
                };
                match vm.lookup_command_bytes_checked(vm.current_ns_id(), &bytes) {
                    Ok(command) => ok(Value::bool(matches!(
                        command,
                        Some((_, crate::command::Command::Ensemble(_)))
                    ))),
                    Err(error) => vm.refuse_host_command(error.to_string()),
                }
            }
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"namespace ensemble exists cmdname\"",
            ),
        },
    }
}

/// The mutable half of an ensemble definition — the options `create` and
/// `configure` share. `-command` is create-only and `-namespace` is
/// configure-read-only, so neither lives here.
struct EnsembleOptions {
    map: Vec<(
        tcl_core_types::NameBytes,
        Vec<Option<tcl_core_types::NameBytes>>,
    )>,
    originals: crate::command::native_ensemble_objects::NativeEnsembleObjects,
    subcommands: Option<Vec<tcl_core_types::NameBytes>>,
    prefixes: bool,
    parameters: Vec<tcl_core_types::NameBytes>,
    unknown: Option<Vec<tcl_core_types::NameBytes>>,
}

impl EnsembleOptions {
    fn from_def(def: &crate::command::EnsembleDef) -> Self {
        Self {
            map: def.map.clone(),
            originals: def.originals.clone(),
            subcommands: def.subcommands.clone(),
            prefixes: def.prefixes,
            parameters: def.parameters.clone(),
            unknown: def.unknown.clone(),
        }
    }
}

/// Apply one `-option value` pair that `create` and `configure` share. The
/// caller has already resolved the option word against its own table, so this
/// only owns the value parsing (C's per-`case` bodies).
///
/// Relative `-map` targets are qualified against the current namespace, which
/// is what both C paths use: `CRT_MAP` against the ensemble's own namespace and
/// `CONF_MAP` against `TclGetCurrentNamespace(interp)` — and each is the current
/// namespace at the point its command runs.
fn apply_shared_option(
    opts: &mut EnsembleOptions,
    which: tcl_cmd_core::ensemble::SharedOption,
    val: &Value,
    vm: &mut Vm,
    creating: bool,
) -> Result<(), Completion<Value>> {
    use crate::command::native_ensemble_objects::NativeEnsembleRoot;
    use tcl_cmd_core::ensemble::{EnsembleObjectRole, SharedOption};
    let protocol = vm
        .native_invocation_dialect()
        .native_string_materialization(None)
        .ok_or_else(|| vm.refuse_host_command("ensemble original configured objects".into()))?
        .protocol();
    match which {
        SharedOption::Map => apply_ensemble_map(opts, val, vm, creating, protocol)?,
        SharedOption::Subcommands => {
            let elems = val
                .native_object_list_elements(protocol)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            let names = elems
                .iter()
                .map(|value| {
                    vm.native_name_operand_bytes(value)
                        .map(|bytes| tcl_core_types::NameBytes::from(bytes.as_ref()))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| vm.refuse_host_command(error.to_string()))?;
            opts.subcommands = (!names.is_empty()).then_some(names);
            opts.originals.subcommands = opts
                .subcommands
                .as_ref()
                .map(|_| EnsembleObjectRole::new(NativeEnsembleRoot::pending(val)));
        }
        SharedOption::Parameters => {
            let elems = val
                .native_object_list_elements(protocol)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            opts.parameters = elems
                .iter()
                .map(|value| {
                    vm.native_name_operand_bytes(value)
                        .map(|bytes| tcl_core_types::NameBytes::from(bytes.as_ref()))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| vm.refuse_host_command(error.to_string()))?;
            opts.originals.parameters = (!opts.parameters.is_empty())
                .then(|| EnsembleObjectRole::new(NativeEnsembleRoot::pending(val)));
        }
        SharedOption::Prefixes => {
            opts.prefixes = tcl_syntax::value::ValueOps::as_bool(vm, val)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
        }
        SharedOption::Unknown => {
            let elements = val
                .native_object_list_elements(protocol)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            let words = elements
                .iter()
                .map(|word| {
                    vm.native_name_operand_bytes(word)
                        .map(|bytes| tcl_core_types::NameBytes::from(bytes.as_ref()))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| vm.refuse_host_command(error.to_string()))?;
            opts.unknown = (!words.is_empty()).then_some(words);
            opts.originals.unknown = opts
                .unknown
                .as_ref()
                .map(|_| EnsembleObjectRole::new(NativeEnsembleRoot::pending(val)));
        }
    }
    Ok(())
}

fn apply_ensemble_map(
    opts: &mut EnsembleOptions,
    val: &Value,
    vm: &mut Vm,
    creating: bool,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<(), Completion<Value>> {
    use crate::command::native_ensemble_objects::NativeEnsembleRoot;
    use tcl_cmd_core::ensemble::EnsembleObjectRole;
    let mut search = val
        .native_lifetime_lease()
        .into_value()
        .into_native_dictionary_search(protocol)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
    let mut patched = None;
    let mut map = Vec::new();
    while let Some((key, value)) = search
        .next_original_pair()
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?
    {
        let elements = value
            .native_object_list_elements(protocol)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
        if elements.is_empty() {
            return Err(err_with_code(
                "ensemble subcommand implementations must be non-empty lists",
                "TCL ENSEMBLE EMPTY_TARGET",
            ));
        }
        let original = vm
            .native_name_operand_bytes(&elements[0])
            .map_err(|error| vm.refuse_host_command(error.to_string()))?;
        let mut prefix = None;
        if !original.starts_with(b"::") {
            let qualified = vm
                .qualify_native_command_prefix_bytes(
                    tcl_runtime_api::Namespaces::current(vm),
                    &original,
                )
                .map_err(|error| vm.refuse_host_command(error.to_string()))?;
            let mut words = elements.to_vec();
            words[0] = Value::from_native_string_bytes(qualified);
            let copy = if creating {
                Value::native_list_constructor(elements.to_vec(), protocol)
            } else {
                value.duplicate_native_object_in(protocol)
            };
            let replacement = Value::native_list_replace_elements(&copy, &words, protocol)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            let root = patched.get_or_insert_with(|| val.duplicate_native_object_in(protocol));
            let updated = root
                .native_dictionary_set_member(key.clone(), replacement.clone(), protocol)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            *root = updated;
            prefix = Some(replacement);
        }
        let name = vm
            .native_name_operand_bytes(&key)
            .map_err(|error| vm.refuse_host_command(error.to_string()))?;
        let words = if let Some(prefix) = prefix {
            prefix
                .native_object_list_elements(protocol)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?
        } else {
            elements
        };
        let words = words
            .iter()
            .map(|word| {
                vm.native_name_operand_bytes(word)
                    .map(|bytes| Some(tcl_core_types::NameBytes::from(bytes.as_ref())))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| vm.refuse_host_command(error.to_string()))?;
        map.push((tcl_core_types::NameBytes::from(name.as_ref()), words));
    }
    drop(search);
    opts.map = map;
    opts.originals.map = (!opts.map.is_empty()).then(|| {
        EnsembleObjectRole::new(patched.map_or_else(
            || NativeEnsembleRoot::pending(val),
            NativeEnsembleRoot::owned,
        ))
    });
    Ok(())
}

/// `namespace ensemble create ?option value ...?` — build the ensemble command.
///
/// C checks the pair arity *before* looking at any option word (`if (objc & 1)`
/// → `wrong # args`, `tclEnsemble.c:192-196`), then resolves each option
/// through `ensembleCreateOptions` with `Tcl_GetIndexFromObj` flags `0` — a
/// table that has `-command` and, deliberately, **no** `-namespace`: an
/// ensemble is always created over the namespace the command runs in.
pub(crate) fn namespace_command_lookup_error(
    vm: &mut Vm,
    written: &[u8],
    prefix: &[u8],
    suffix: &[u8],
    kind: &[u8],
) -> Completion<Value> {
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("native command reporting protocol is unavailable".into());
    };
    let name = match tcl_syntax::naming::report_native_name_bytes(
        policy.recipe(),
        tcl_syntax::naming::NativeNameReportPurpose::CommandLookupError,
        written,
    ) {
        Ok(name) => name,
        Err(error) => return vm.refuse_host_command(format!("{error:?}")),
    };
    let mut message = prefix.to_vec();
    message.push(b'"');
    message.extend_from_slice(name);
    message.push(b'"');
    message.extend_from_slice(suffix);
    let mut code = b"TCL LOOKUP ".to_vec();
    code.extend_from_slice(kind);
    code.push(b' ');
    tcl_syntax::list::append_list_element(&mut code, name, false);
    crate::command::completion_from_cmd_error(
        vm,
        tcl_cmd_core::CmdError::with_error_code_bytes(message, code),
    )
}

fn ns_ensemble_create(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    use crate::command::EnsembleDef;
    let Some(options) = vm
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
    else {
        return vm.refuse_host_command("ensemble configuration options".into());
    };

    if !args.len().is_multiple_of(2) {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace ensemble create ?option value ...?\"",
        );
    }
    let ns_id = tcl_runtime_api::Namespaces::current(vm);
    let mut command: Option<std::rc::Rc<[u8]>> = None;
    let mut opts = EnsembleOptions {
        map: Vec::new(),
        originals: crate::command::native_ensemble_objects::NativeEnsembleObjects::default(),
        subcommands: None,
        prefixes: true,
        parameters: Vec::new(),
        unknown: None,
    };
    for pair in args.as_chunks::<2>().0 {
        let resolved = match vm.native_static_option_index(
            &pair[0],
            options.create_options(),
            false,
            "option",
        ) {
            Ok(index) => options.create_option(index),
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let Some(shared) = resolved.shared() else {
            // `-command` names the command rather than configuring it.
            command = Some(match vm.native_name_operand_bytes(&pair[1]) {
                Ok(bytes) => bytes,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            });
            continue;
        };
        if let Err(completion) = apply_shared_option(&mut opts, shared, &pair[1], vm, true) {
            return completion;
        }
    }
    // The default command is the namespace itself; an explicit -command binds in
    // the current namespace when unqualified.
    let slot = match vm.native_namespace_ensemble_publication_slot(command.as_deref()) {
        Ok(slot) => slot,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let mut reporting = b"::".to_vec();
    reporting.extend_from_slice(&vm.command_slot_display_bytes(&slot));
    let def = EnsembleDef {
        originals: opts.originals,
        native: None,
        namespace: ns_id,
        map: opts.map,
        subcommands: opts.subcommands,
        prefixes: opts.prefixes,
        parameters: opts.parameters,
        unknown: opts.unknown,
    };
    let token = std::rc::Rc::new(
        tcl_cmd_core::ensemble::EnsembleToken::with_configuration_retirement(
            def,
            tcl_core_types::NameBytes::from(reporting.as_slice()),
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
            crate::command::native_ensemble_objects::retire_configuration,
        ),
    );
    vm.register_namespace_ensemble_slot(slot, &token);
    token.config().originals.activate();
    ok(Value::from_native_string_bytes(reporting))
}

/// `namespace ensemble configure cmdname ?-option? ?value ...?`
/// (`tclEnsemble.c:377-630`): with no options an ordered key/value List of settings, with a
/// single option that option's value, otherwise `-option value` updates.
/// `ensembleConfigOptions` differs from the create table — it carries
/// `-namespace` (readable, never writable) and has no `-command`.
fn ns_ensemble_configure(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    const USAGE: &str = "wrong # args: should be \"namespace ensemble configure cmdname ?-option value ...? ?arg ...?\"";
    let Some(options) = vm
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
    else {
        return vm.refuse_host_command("ensemble configuration options".into());
    };

    let Some((cmd_val, rest)) = args.split_first() else {
        return crate::command::native_wrong_arguments_message(vm, USAGE);
    };
    // C's arity gate: one bare option word is a read, anything else must be
    // `-option value` pairs.
    if rest.len() > 1 && !rest.len().is_multiple_of(2) {
        return crate::command::native_wrong_arguments_message(vm, USAGE);
    }
    let written = match vm.native_name_operand_bytes(cmd_val) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    // `Tcl_FindEnsemble` reports the two failures separately: a name that
    // resolves to no command at all is `unknown command "x"` (the
    // `Tcl_FindCommand` miss), while a name that *is* a command but carries a
    // different implementation is `"x" is not an ensemble command`. Collapsing
    // them into the second message misreports a plain typo.
    let selected =
        match vm.lookup_command_bytes_checked(tcl_runtime_api::Namespaces::current(vm), &written) {
            Ok(selected) => selected,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
    let token = match selected {
        Some((_, crate::command::Command::Ensemble(token))) => token,
        Some(_) => {
            return namespace_command_lookup_error(
                vm,
                &written,
                b"",
                b" is not an ensemble command",
                b"ENSEMBLE",
            );
        }
        None => {
            return namespace_command_lookup_error(
                vm,
                &written,
                b"unknown command ",
                b"",
                b"COMMAND",
            );
        }
    };
    let def = token.config();
    if rest.is_empty() {
        return ensemble_configuration_snapshot(vm, &def, options);
    }
    if let [only] = rest {
        return match vm.native_static_option_index(
            only,
            options.configure_options(),
            false,
            "option",
        ) {
            Ok(index) => match ensemble_option_value(vm, &def, options.configure_option(index)) {
                Ok(value) => ok(value),
                Err(error) => crate::command::completion_from_cmd_error(vm, error.into()),
            },
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        };
    }
    let mut opts = EnsembleOptions::from_def(&def);
    // `apply_shared_option` qualifies relative `-map` targets against `vm`'s
    // current namespace, which is what CONF_MAP wants here: it uses
    // `TclGetCurrentNamespace(interp)` — the namespace current at the
    // `configure` call, NOT the ensemble's own namespace (which CRT_MAP uses
    // at create time). They coincide in the common
    // `namespace eval M {namespace ensemble configure …}` shape, but
    // configuring an ensemble from outside its namespace resolves relative
    // targets against the caller.
    for pair in rest.as_chunks::<2>().0 {
        let resolved = match vm.native_static_option_index(
            &pair[0],
            options.configure_options(),
            false,
            "option",
        ) {
            Ok(index) => options.configure_option(index),
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let Some(shared) = resolved.shared() else {
            return err_with_code("option -namespace is read-only", "TCL ENSEMBLE READ_ONLY");
        };
        if let Err(completion) = apply_shared_option(&mut opts, shared, &pair[1], vm, false) {
            return completion;
        }
    }
    opts.originals.activate();
    token.configure(crate::command::EnsembleDef {
        originals: opts.originals,
        native: def.native,
        namespace: def.namespace,
        map: opts.map,
        subcommands: opts.subcommands,
        prefixes: opts.prefixes,
        parameters: opts.parameters,
        unknown: opts.unknown,
    });
    vm.note_ensemble_configuration_changed(&token);
    ok(Value::empty())
}

fn ensemble_configuration_snapshot(
    vm: &mut Vm,
    def: &crate::command::EnsembleDef,
    options: tcl_registry::native_ensemble::NativeEnsembleConfigurationProtocol,
) -> Completion<Value> {
    let Some(recipe) = vm
        .native_invocation_dialect()
        .native_string_materialization(None)
    else {
        return vm.refuse_host_command("ensemble configuration List recipe".into());
    };
    let mut pairs = Vec::new();
    for option in options.configuration_options() {
        let value = match ensemble_option_value(vm, def, option) {
            Ok(value) => value,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
        };
        pairs.push(Value::string(option.name()));
        pairs.push(value);
    }
    ok(Value::native_list_constructor(pairs, recipe.protocol()))
}

/// One `namespace ensemble configure` option's value.
fn ensemble_option_value(
    vm: &Vm,
    def: &crate::command::EnsembleDef,
    option: tcl_cmd_core::ensemble::ConfigOption,
) -> Result<Value, tcl_syntax::value::ValueError> {
    use tcl_cmd_core::ensemble::ConfigOption;
    let role = match option {
        ConfigOption::Map => def.originals.map.as_ref(),
        ConfigOption::Unknown => def.originals.unknown.as_ref(),
        ConfigOption::Subcommands => def.originals.subcommands.as_ref(),
        ConfigOption::Parameters => def.originals.parameters.as_ref(),
        ConfigOption::Namespace | ConfigOption::Prefixes => None,
    };
    if let Some(original) =
        role.and_then(|role| role.inspect(|root| root.inspect(Value::clone)).flatten())
    {
        return Ok(original);
    }
    let recipe = vm
        .native_invocation_dialect()
        .native_string_materialization(None)
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "ensemble configuration original values",
        ))?;
    let protocol = recipe.protocol();
    let list = |words: &[tcl_core_types::NameBytes]| {
        Value::native_list_constructor(
            words
                .iter()
                .map(|word| Value::from_native_string_bytes(word.as_bytes()))
                .collect(),
            protocol,
        )
    };
    Ok(match option {
        ConfigOption::Namespace => {
            Value::string(tcl_runtime_api::Namespaces::name(vm, def.namespace))
        }
        ConfigOption::Prefixes => Value::bool(def.prefixes),
        ConfigOption::Parameters => {
            if def.parameters.is_empty() {
                Value::empty()
            } else {
                list(&def.parameters)
            }
        }
        ConfigOption::Unknown => def.unknown.as_deref().map_or_else(Value::empty, list),
        ConfigOption::Subcommands => def.subcommands.as_deref().map_or_else(Value::empty, list),
        ConfigOption::Map if def.map.is_empty() => Value::empty(),
        ConfigOption::Map => {
            let pairs = def
                .map
                .iter()
                .map(|(key, words)| {
                    let words = words.iter().cloned().collect::<Option<Vec<_>>>().ok_or(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "ensemble original target contents",
                        ),
                    )?;
                    Ok((
                        Value::from_native_string_bytes(key.as_bytes()),
                        list(&words),
                    ))
                })
                .collect::<Result<Vec<_>, tcl_syntax::value::ValueError>>()?;
            Value::native_dictionary_constructor(pairs, None, protocol)?
        }
    })
}

/// Native namespace text reporting selects its purpose-specific input extent.
fn ns_text_op(vm: &mut Vm, rest: &[Value], tail: bool) -> Completion<Value> {
    use tcl_syntax::naming::NativeNamespaceTextResult;

    let Some(original) = rest.first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            if tail {
                "wrong # args: should be \"namespace tail string\""
            } else {
                "wrong # args: should be \"namespace qualifiers string\""
            },
        );
    };
    let bytes = match vm.native_name_operand_bytes(original) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("native namespace text protocol is unavailable".into());
    };
    let result = match policy.recipe().namespace_text_result(&bytes, tail) {
        NativeNamespaceTextResult::Original => original.clone(),
        NativeNamespaceTextResult::Counted(bytes) => Value::from_native_string_bytes(bytes),
        NativeNamespaceTextResult::Append(bytes) => {
            let Some(append) = vm
                .native_invocation_dialect()
                .native_object_append_protocol(None)
            else {
                return vm
                    .refuse_host_command("namespace text append protocol is unavailable".into());
            };
            let receiver = Value::new_native_string_bytes(b"".as_slice());
            match tcl_cmd_core::native_append::append_counted_bytes(
                &crate::value::VmAppendObjects,
                append.recipe(),
                &receiver,
                bytes,
            ) {
                Ok(result) => result,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            }
        }
    };
    ok(result)
}

fn ns_inscope(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((ns, parts)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace inscope namespace arg ?arg ...?\"",
        );
    };
    let Some((script, extra)) = parts.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace inscope namespace arg ?arg ...?\"",
        );
    };
    let jim = vm
        .name_policy_protocol()
        .is_some_and(|policy| policy.recipe().is_jim084());
    let concatenated;
    let (body, purpose) = if extra.is_empty() && !jim {
        (script, EvalObjectPurpose::NamespaceBody)
    } else {
        concatenated = match inscope_script(vm, script, extra) {
            Ok(body) => body,
            Err(completion) => return completion,
        };
        (&concatenated, EvalObjectPurpose::NamespaceConcat)
    };
    let mut call_argv = vec![Value::string("namespace"), Value::string("inscope")];
    call_argv.extend(rest.iter().cloned());
    let create = vm
        .name_policy_protocol()
        .is_some_and(|policy| policy.recipe().is_jim084());
    eval_in_ns(
        vm,
        ns,
        body,
        purpose,
        call_argv,
        extra.is_empty().then(|| script.source_location()).flatten(),
        create,
    )
}

/// The script `namespace inscope ns script ?arg ...?` evaluates:
/// `Tcl_ConcatObj(script, list(arg …))` — `NamespaceInscopeCmd`
/// (`generic/tclNamesp.c`) collects the trailing words into a **list object**
/// and concatenates that list's string representation onto `script`. So the
/// tail arrives as list *elements*, not as space-joined script text, and each
/// word reaches the invoked command as exactly one argument however much
/// whitespace or list punctuation it holds:
///
/// ```text
/// namespace inscope :: {puts} {a b}   → prints "a b"  (one argument)
/// namespace eval    :: {puts} {a b}   → error: can not find channel named "a"
/// ```
///
/// (`namespace eval`'s plain space-join is right for *its* concat semantics;
/// `inscope` is the one family member that list-quotes — the registry models
/// the split as `SCRIPT_APPENDS_LIST_ARGS` refining `SCRIPT_CONCATENATES_ARGS`.
/// Space-joining here too would turn `{x y}` into two arguments.)
///
/// Both halves reuse the canonical implementations rather than re-deriving
/// them: the list's string rep comes from `Value::list`'s
/// `tcl_syntax::list::join_list` quoting (`Tcl_ScanElement` /
/// `Tcl_ConvertElement`), and the concatenation itself is the shared
/// `tcl_cmd_core::list::concat_selected`, which owns the selected engine's
/// string or representation-sensitive concatenation protocol. C Tcl trims
/// each part and drops one that is empty after trimming, so an all-whitespace
/// `script` contributes no leading separator.
///
/// With no trailing words C takes the `objc == 3` arm and evaluates `script`
/// verbatim (no concat, hence no trim and no trailing space), which the early
/// return mirrors.
fn inscope_script(
    vm: &mut Vm,
    script: &Value,
    extra: &[Value],
) -> Result<Value, Completion<Value>> {
    let tail = Value::list(extra.to_vec());
    tcl_cmd_core::list::concat_selected(vm, &[script.clone(), tail])
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
}

fn ns_eval(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((ns, body_parts)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace eval name arg ?arg ...?\"",
        );
    };
    if body_parts.is_empty() {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace eval name arg ?arg ...?\"",
        );
    }
    let concatenated;
    let (body, purpose) = if let [original] = body_parts {
        (original, EvalObjectPurpose::NamespaceBody)
    } else {
        concatenated = match tcl_cmd_core::list::concat_selected(vm, body_parts) {
            Ok(value) => value,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        (&concatenated, EvalObjectPurpose::NamespaceConcat)
    };
    let mut call_argv = vec![Value::string("namespace"), Value::string("eval")];
    call_argv.extend(rest.iter().cloned());
    eval_in_ns(
        vm,
        ns,
        body,
        purpose,
        call_argv,
        body_parts
            .first()
            .filter(|_| body_parts.len() == 1)
            .and_then(Value::source_location),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_namespace_bodies_propagate_return_and_inscope_requires_c_target() {
        use tcl_compiler::compile_service::BytecodeCompileService;
        for profile in tcl_dialect::TclVersion::ALL
            .map(|version| crate::environment::profile_for_dialect(version.dialect_profile_name()))
            .into_iter()
            .chain(std::iter::once(crate::environment::profile_for_dialect(
                "jim",
            )))
        {
            let mut vm = Vm::with_output(Box::new(std::io::sink()));
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            let arguments = [Value::string("n"), Value::string("return x")];
            let returned = ns_eval(&mut vm, &arguments);
            assert_eq!(returned.code, Code::Return, "{}", profile.name);
            assert_eq!(returned.result.string_bytes().as_ref(), b"x");
            assert_eq!(vm.current_ns_id(), tcl_runtime_api::ROOT_NS);
            let returned = ns_inscope(&mut vm, &arguments);
            assert_eq!(returned.code, Code::Return, "{}", profile.name);
            let absent = [Value::string("absent"), Value::string("return y")];
            let result = ns_inscope(&mut vm, &absent);
            if vm.name_policy_protocol().unwrap().recipe().is_jim084() {
                assert_eq!(result.code, Code::Return);
                assert_eq!(result.result.string_bytes().as_ref(), b"y");
            } else {
                assert_eq!(result.code, Code::Error);
                assert!(
                    result
                        .result
                        .string_bytes()
                        .windows(b"absent".len())
                        .any(|word| word == b"absent")
                );
                assert!(vm.namespace_object_lookup(&absent[0]).unwrap().is_none());
            }
        }
    }

    #[test]
    fn namespace_text_preserves_original_native_byte_names() {
        let profiles = tcl_dialect::TclVersion::ALL
            .map(|version| crate::environment::profile_for_dialect(version.dialect_profile_name()));
        for profile in
            profiles
                .into_iter()
                .chain(std::iter::once(crate::environment::profile_for_dialect(
                    "jim",
                )))
        {
            let mut vm = Vm::with_output(Box::new(std::io::sink()));
            vm.set_dialect_profile(profile);
            let raw = Value::from_native_string_bytes(b"a\0z::p".as_slice());
            let tail = ns_text_op(&mut vm, std::slice::from_ref(&raw), true);
            assert_eq!(tail.code, Code::Ok);
            let expected: &[u8] = if vm.name_policy_protocol().unwrap().recipe().is_jim084() {
                b"a\0z::p"
            } else {
                b"a"
            };
            assert_eq!(tail.result.string_bytes().as_ref(), expected);
            let opaque = Value::from_native_string_bytes(b"n::\xff".as_slice());
            let tail = ns_text_op(&mut vm, std::slice::from_ref(&opaque), true);
            assert_eq!(tail.result.string_bytes().as_ref(), b"\xff");
            assert_eq!(opaque.string_bytes().as_ref(), b"n::\xff");
            let modified = Value::from_native_string_bytes(b"a\xc0\x80z::p".as_slice());
            let qualifiers = ns_text_op(&mut vm, std::slice::from_ref(&modified), false);
            assert_eq!(qualifiers.result.string_bytes().as_ref(), b"a\xc0\x80z");
        }
    }

    #[test]
    fn code_retains_original_objects_for_all_measured_native_prefixes() {
        let specimens: [&[u8]; 11] = [
            b"::namespace inscope ",
            b"::namespace inscope :: cmd",
            b"namespace inscope :: cmd",
            b":namespace inscope :: cmd",
            b"::::namespace inscope :: cmd",
            b"namespace    inscope :: cmd",
            b"namespace\tinscope :: cmd",
            b"namespaceinscopeXX",
            b"namespaceinscopeX",
            b"::namespace inscopeX",
            b"::namespace inscope \0",
        ];
        let jim = crate::environment::profile_for_dialect("jim");
        let mut rows = 0;
        for profile in tcl_dialect::TclVersion::ALL
            .map(|version| crate::environment::profile_for_dialect(version.dialect_profile_name()))
            .into_iter()
            .chain(std::iter::once(jim))
        {
            let mut vm = Vm::with_output(Box::new(std::io::sink()));
            vm.set_dialect_profile(profile);
            let preserved: &[usize] = if profile.vm_runtime_version == tcl_dialect::TclVersion::V8_4
                && !std::ptr::eq(profile, jim)
            {
                &[0, 1, 2, 3, 4, 5, 7, 9, 10]
            } else if std::ptr::eq(profile, jim) {
                &[0, 1, 10]
            } else {
                &[1, 10]
            };
            for (index, bytes) in specimens.into_iter().enumerate() {
                let script = Value::from_string_bytes(bytes);
                let result = namespace_code_invocation(
                    &mut vm,
                    std::slice::from_ref(&script),
                    NamespaceInvocation::Public,
                );
                assert_eq!(result.code, Code::Ok, "{}: {index}", profile.name);
                if preserved.contains(&index) {
                    assert!(
                        result.result.is_same_object(&script),
                        "{}: {index}",
                        profile.name
                    );
                } else {
                    let elements = result.result.as_list().expect("command-prefix list");
                    assert_eq!(elements.len(), 4);
                    assert_eq!(elements[0].string_bytes().as_ref(), b"::namespace");
                    assert_eq!(elements[1].string_bytes().as_ref(), b"inscope");
                    assert_eq!(elements[2].string_bytes().as_ref(), b"::");
                    assert!(
                        elements[3].is_same_object(&script),
                        "{}: {index}",
                        profile.name
                    );
                }
                assert_eq!(script.string_bytes().as_ref(), bytes);
                rows += 1;
            }
        }
        assert_eq!(rows, 66);
    }

    #[test]
    fn code_keeps_non_unicode_bytes_and_uses_native_arity() {
        let mut vm = Vm::with_output(Box::new(std::io::sink()));
        vm.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        for bytes in [
            b"\xff\0script".as_slice(),
            b"::namespace inscope \xff\0".as_slice(),
        ] {
            let script = Value::from_string_bytes(bytes);
            let completion = namespace_code_invocation(
                &mut vm,
                std::slice::from_ref(&script),
                NamespaceInvocation::Public,
            );
            assert_eq!(completion.code, Code::Ok);
            let retained = if completion.result.is_same_object(&script) {
                completion.result
            } else {
                completion.result.as_list().unwrap()[3].clone()
            };
            assert!(retained.is_same_object(&script));
            assert_eq!(retained.string_bytes().as_ref(), bytes);
        }
        let completion = namespace_code_invocation(&mut vm, &[], NamespaceInvocation::Public);
        assert_eq!(completion.code, Code::Error);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            b"wrong # args: should be \"namespace code arg\""
        );
    }

    #[test]
    fn current_and_code_wrong_arity_retain_original_header_words() {
        for version in tcl_dialect::TclVersion::ALL {
            let mut vm = Vm::with_output(Box::new(std::io::sink()));
            vm.set_runtime_version(version);
            vm.set_invoked_name("n s\0tail");
            let expected: &[u8] = match version {
                tcl_dialect::TclVersion::V8_4 => b"n s",
                tcl_dialect::TclVersion::V8_5 | tcl_dialect::TclVersion::V8_6 => b"n s\0tail",
                tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1 => b"{n s\0tail}",
            };
            let completion = namespace_current_invocation(
                &mut vm,
                &[Value::string("extra")],
                NamespaceInvocation::Public,
            );
            let mut message = b"wrong # args: should be \"".to_vec();
            message.extend_from_slice(expected);
            message.extend_from_slice(b" current\"");
            assert_eq!(completion.result.string_bytes().as_ref(), message);
            let completion = namespace_code_invocation(&mut vm, &[], NamespaceInvocation::Public);
            let mut message = b"wrong # args: should be \"".to_vec();
            message.extend_from_slice(expected);
            message.extend_from_slice(b" code arg\"");
            assert_eq!(completion.result.string_bytes().as_ref(), message);
        }
    }

    #[test]
    fn code_refuses_an_unaudited_engine_outside_guest_completion() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )));
        let mut vm = Vm::with_output(Box::new(std::io::sink()));
        vm.set_dialect_profile(profile);
        let script = Value::string("puts example");
        let completion = namespace_code_invocation(
            &mut vm,
            std::slice::from_ref(&script),
            NamespaceInvocation::Public,
        );
        assert_eq!(completion.code, Code::Error);
        assert!(vm.refused_completion().is_some());
        assert_eq!(completion.result.string_bytes().as_ref(), b"");
        assert_eq!(script.string_bytes().as_ref(), b"puts example");
    }

    #[test]
    fn code_uses_the_explicit_f5_logical_provider() {
        let mut vm = Vm::with_output(Box::new(std::io::sink()));
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.set_native_engine_profile(tcl_dialect::DialectProfile::find("tcl9.0").unwrap()));
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
        let script = Value::string("namespaceinscopeXX");
        let completion = namespace_code_invocation(
            &mut vm,
            std::slice::from_ref(&script),
            NamespaceInvocation::Public,
        );
        assert_eq!(completion.code, Code::Ok);
        assert!(completion.result.is_same_object(&script));
        assert!(
            vm.native_invocation_dialect()
                .native_namespace_code_policy()
                .is_none()
        );
    }
}

#[cfg(test)]
mod native_upvar_fixture_tests {
    use super::*;
    use tcl_syntax::value::ValueOps;

    fn bytes_from_hex(hex: &str) -> Vec<u8> {
        assert_eq!(hex.len() % 2, 0);
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    // Native proof: naming.variable.namespace-upvar-target-and-empty-pair-grammar
    // docs/design/analysis/name-resolution-proofs/variable.namespace-upvar-target-and-empty-pair-grammar.md
    #[test]
    fn namespace_upvar_grammar_and_target_cells_match_36_native_results() {
        const CASES: &str = include_str!("../tests/data/native_namespace_upvar/cases.tsv");
        let engines = [
            (
                "tcl8.4",
                include_str!("../tests/data/native_namespace_upvar/8.4.20.tsv"),
            ),
            (
                "tcl8.5",
                include_str!("../tests/data/native_namespace_upvar/8.5.19.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../tests/data/native_namespace_upvar/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../tests/data/native_namespace_upvar/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../tests/data/native_namespace_upvar/9.1.0.tsv"),
            ),
            (
                "jim",
                include_str!("../tests/data/native_namespace_upvar/Jim.tsv"),
            ),
        ];
        let mut compared = 0;
        for (engine, expected) in engines {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            assert_eq!(CASES.lines().count(), expected.lines().count());
            for (input, expected) in CASES.lines().zip(expected.lines()) {
                let (name, source) = input.split_once('\t').unwrap();
                let mut wanted = expected.splitn(3, '\t');
                assert_eq!(wanted.next().unwrap(), name);
                let code: i64 = wanted.next().unwrap().parse().unwrap();
                let result = bytes_from_hex(wanted.next().unwrap());
                let mut vm = crate::native_fixture::interpreter(profile);
                let completion = vm
                    .try_eval_source_bytes(&bytes_from_hex(source))
                    .unwrap_or_else(|error| panic!("{engine}/{name}: {error:?}"));
                assert_eq!(completion.code.as_int(), code, "{engine}/{name}");
                let actual = ValueOps::native_string_bytes(&mut vm, &completion.result).unwrap();
                assert_eq!(actual.as_ref(), result.as_slice(), "{engine}/{name}");
                compared += 1;
            }
        }
        assert_eq!(compared, 36);
    }
    #[test]
    fn namespace_upvar_original_vectors_match_eighteen_native_arity_windows() {
        let rows =
            include_str!("../../tcl-registry/tests/data/native_namespace_upvar_arguments/rows.tsv");
        let mut compared = 0;
        for row in rows.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let profile =
                tcl_registry::model::ingress::resolve_environment(fields[0]).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            assert_eq!(
                vm.eval_source("namespace eval n {}").unwrap().code,
                Code::Ok
            );
            let head = Value::string("namespace");
            let mut args = vec![Value::string("upvar"), Value::string("n")];
            match fields[1] {
                "0" => {}
                "1" => args.push(Value::string("a")),
                "2" => args.extend([Value::string("a"), Value::string("b")]),
                other => panic!("unknown original case {other}"),
            }
            let result = vm.invoke_host_original_object_vector(&head, &args);
            assert_eq!(
                result.code.as_int(),
                fields[2].parse::<i64>().unwrap(),
                "{row}"
            );
            assert_eq!(
                ValueOps::native_string_bytes(&mut vm, &result.result)
                    .unwrap()
                    .as_ref(),
                bytes_from_hex(fields[3]).as_slice(),
                "{row}"
            );
            compared += 1;
        }
        assert_eq!(compared, 18);
    }

    #[test]
    fn jim_namespace_upvar_forwards_root_worker_without_changing_variable_frame() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let source = include_str!(
            "../../tcl-registry/tests/data/native_namespace_upvar_arguments/jim-forwarding.tcl"
        );
        let completion = vm.eval_source(source).unwrap();
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
            ValueOps::native_string_bytes(&mut vm, &completion.result)
                .unwrap()
                .as_ref(),
            b"VALUE {FORWARD 0 ::missing::a {}}"
        );
    }
}

#[cfg(test)]
mod original_ensemble_tests {
    use super::*;

    #[test]
    fn map_queries_keep_the_same_original_and_do_not_own_retired_configuration() {
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = Vm::new();
            vm.set_dialect_profile(
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
            );
            let args: Vec<_> = ["-command", "::E", "-map", "a ::target"]
                .into_iter()
                .map(Value::string)
                .collect();
            let created = ns_ensemble_create(&mut vm, &args);
            assert_eq!(created.code, Code::Ok, "{version:?}");
            let (_, crate::command::Command::Ensemble(token)) = vm
                .lookup_command_bytes_checked(vm.current_ns_id(), b"::E")
                .unwrap()
                .unwrap()
            else {
                panic!("actual ensemble token");
            };
            let query = token.config();
            let references = args[3].native_object_reference_count();
            let another = query.clone();
            assert_eq!(args[3].native_object_reference_count(), references);
            let original =
                ensemble_option_value(&vm, &query, tcl_cmd_core::ensemble::ConfigOption::Map)
                    .unwrap();
            assert_eq!(
                original.native_object_identity(),
                args[3].native_object_identity()
            );
            drop(original);
            let deleted = vm.invoke_command("rename", &[Value::string("::E"), Value::string("")]);
            assert_eq!(deleted.code, Code::Ok, "{version:?}");
            assert!(
                query
                    .originals
                    .map
                    .as_ref()
                    .unwrap()
                    .inspect(|_| ())
                    .is_none()
            );
            assert!(
                another
                    .originals
                    .map
                    .as_ref()
                    .unwrap()
                    .inspect(|_| ())
                    .is_none()
            );
            assert_eq!(args[3].native_object_reference_count(), references - 1);
        }
    }

    #[test]
    fn original_ensemble_options_use_actual_release_tables() {
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = Vm::new();
            vm.set_dialect_profile(
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
            );
            assert_eq!(
                ns_ensemble_create(&mut vm, &[Value::string("-command"), Value::string("::E")])
                    .code,
                Code::Ok
            );
            let query = ns_ensemble_configure(
                &mut vm,
                &[Value::string("::E"), Value::string("-parameters")],
            );
            assert_eq!(
                query.code,
                if version == tcl_dialect::TclVersion::V8_5 {
                    Code::Error
                } else {
                    Code::Ok
                }
            );
            let query =
                ns_ensemble_configure(&mut vm, &[Value::string("::E"), Value::string("-p")]);
            assert_eq!(
                query.code,
                if version == tcl_dialect::TclVersion::V8_5 {
                    Code::Ok
                } else {
                    Code::Error
                }
            );
            let query = ns_ensemble_configure(&mut vm, &[Value::string("::E")]);
            let recipe = vm
                .native_invocation_dialect()
                .native_string_materialization(None)
                .unwrap();
            let entries = query
                .result
                .native_object_list_elements(recipe.protocol())
                .unwrap();
            assert_eq!(
                entries.len(),
                if version == tcl_dialect::TclVersion::V8_5 {
                    10
                } else {
                    12
                }
            );
        }
    }

    #[test]
    fn counted_map_roots_and_native_cstring_collision_owners_are_independent() {
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = Vm::new();
            vm.set_dialect_profile(
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
            );
            let protocol = vm
                .native_invocation_dialect()
                .native_string_materialization(None)
                .unwrap()
                .protocol();
            let left = Value::from_native_string_bytes(b"m\0left".as_slice());
            let right = Value::from_native_string_bytes(b"m\0right".as_slice());
            let first = Value::native_list_constructor(
                vec![
                    Value::string("::set"),
                    Value::string("selected"),
                    Value::string("FIRST"),
                ],
                protocol,
            );
            let second = Value::native_list_constructor(
                vec![
                    Value::string("::set"),
                    Value::string("selected"),
                    Value::string("SECOND"),
                ],
                protocol,
            );
            let mapping = Value::native_dictionary_constructor(
                vec![
                    (left.clone(), first.clone()),
                    (right.clone(), second.clone()),
                ],
                None,
                protocol,
            )
            .unwrap();
            let args = [
                Value::string("-command"),
                Value::string("::E"),
                Value::string("-map"),
                mapping.clone(),
            ];
            assert_eq!(ns_ensemble_create(&mut vm, &args).code, Code::Ok);
            let result = vm.invoke_command("::E", &[Value::string("m")]);
            assert_eq!(result.code, Code::Ok);
            assert_eq!(result.result.to_str().as_ref(), "SECOND");
            assert_eq!(first.native_object_reference_count(), 3);
            assert_eq!(second.native_object_reference_count(), 3);
            let original =
                ns_ensemble_configure(&mut vm, &[Value::string("::E"), Value::string("-map")]);
            assert_eq!(
                original.result.native_object_identity(),
                mapping.native_object_identity()
            );
            assert_eq!(
                original
                    .result
                    .with_cached_dictionary_representation(|pairs, _| pairs.len()),
                Some(2)
            );
            drop(original);
            let subcommands =
                Value::native_list_constructor(vec![left.clone(), right.clone()], protocol);
            assert_eq!(
                ns_ensemble_configure(
                    &mut vm,
                    &[
                        Value::string("::E"),
                        Value::string("-subcommands"),
                        subcommands
                    ]
                )
                .code,
                Code::Ok
            );
            let result = vm.invoke_command("::E", &[Value::string("m")]);
            assert_eq!(result.result.to_str().as_ref(), "FIRST");
            assert_eq!(first.native_object_reference_count(), 4);
            assert_eq!(second.native_object_reference_count(), 2);
            drop(args);
            drop(mapping);
            drop(vm);
            assert_eq!(first.native_object_reference_count(), 2);
            assert_eq!(second.native_object_reference_count(), 1);
            assert!(first.native_object_is_live());
        }
    }

    #[test]
    fn explicit_subcommands_restrict_the_actual_map() {
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = Vm::new();
            vm.set_dialect_profile(
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
            );
            let args: Vec<_> = [
                "-command",
                "::E",
                "-map",
                "a ::target b ::target",
                "-subcommands",
                "a",
            ]
            .into_iter()
            .map(Value::string)
            .collect();
            assert_eq!(ns_ensemble_create(&mut vm, &args).code, Code::Ok);
            let result = vm.invoke_command("::E", &[Value::string("b")]);
            assert_eq!(result.code, Code::Error);
            assert_eq!(
                result.result.to_str().as_ref(),
                "unknown or ambiguous subcommand \"b\": must be a"
            );
        }
    }
}

#[cfg(test)]
mod native_store_tests;
