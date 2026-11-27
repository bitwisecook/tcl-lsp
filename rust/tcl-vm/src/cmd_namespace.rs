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

#[cfg(test)]
use tcl_registry::native_eval_object::NativeNamespaceFramePurpose;
use tcl_runtime_api::{Code, Completion};

use crate::interp::{Vm, err, ok};
use crate::value::Value;
use tcl_dialect::model::surface_admits;
#[cfg(test)]
use tcl_syntax::native_namespace_name::NativeEnsembleNamespaceResultStage;

mod native_jim_helpers;
mod native_jim_subcommands;

pub(crate) fn register(vm: &mut Vm) {
    let Some(namespace) = vm
        .actual_native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Namespace)
    else {
        vm.register_stock_builtin("namespace", cmd_namespace);
        return;
    };
    let subs = crate::environment::release_subcommands(
        vm.actual_native_execution_profile().name,
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
        let target = tcl_syntax::naming::rooted_member_key("::tcl::namespace", member);
        if vm.stock_native_identity(&target).as_deref()
            == tcl_syntax::naming::unroot_rooted_key(&target)
        {
            vm.remove_registered_command(
                tcl_syntax::naming::unroot_rooted_key(&target).unwrap_or(&target),
            );
        }
    }
    register(vm);
    if vm
        .actual_native_invocation_dialect()
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

mod body_evaluation;
mod ensemble;
mod import_export;

#[cfg(test)]
use body_evaluation::namespace_frame_arguments;
use body_evaluation::{ns_eval, ns_inscope};
use ensemble::ns_ensemble;
#[cfg(test)]
use ensemble::{ensemble_option_value, ns_ensemble_configure, ns_ensemble_create};
use import_export::{ns_export, ns_import};

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
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
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
        )) else {
            return vm.refuse_host_command(
                "namespace code native handler policy is not selected".into(),
            );
        };
        let bytes = match original_names.read(vm, script) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        if policy.preserves_argument(&bytes) {
            return ok(script.clone());
        }
        let tcl_registry::native_namespace_code::NamespaceCodeConstruction::FreshList {
            fresh_string_literals: [head, selector],
        } = policy.construction()
        else {
            return vm.refuse_host_command(
                "namespace code construction requires the original scripted helper".into(),
            );
        };
        // C's two generated constants each receive a fresh untyped String
        // header. They do not borrow the caller's head or selector object.
        let head = Value::new_native_string_bytes(head);
        let selector = Value::new_native_string_bytes(selector);
        let context = match vm.native_namespace_result_object(
            vm.current_ns_id(),
            tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer::CodeContext,
        ) {
            Ok(context) => context,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        ok(tcl_syntax::value::ValueOps::new_list(
            vm,
            vec![head, selector, context, script.clone()],
        ))
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
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
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        let Some((sub, rest)) = args.split_first() else {
            return crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"namespace subcommand ?arg ...?\"",
            );
        };
        let Some(name_policy) = vm.name_policy_protocol() else {
            return vm
                .refuse_host_command("native namespace subcommand protocol is unavailable".into());
        };
        let (subcommand_name, synopsis) = if name_policy.recipe().is_jim084() {
            let entry = match native_jim_subcommands::select(vm, sub, rest.len(), &original_names) {
                Ok(entry) => entry,
                Err(completion) => return completion,
            };
            if entry.scripted_helper_kind().is_some() {
                return native_jim_helpers::dispatch(vm, sub, rest, &original_names);
            }
            (entry.name(), entry.usage())
        } else {
            let original_sub = match original_names.read(vm, sub) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
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
            let Some(subcommand) = spec.resolve_native_namespace_subcommand_bytes_for_dialect(
                name_policy.recipe(),
                &original_sub,
                dialect,
            ) else {
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
            (subcommand.name, subcommand.synopsis)
        };
        match subcommand_name {
            "canonical" => {
                let (namespace, name) = match rest {
                    [] => {
                        return match vm.with_jim_current_namespace(|original| Ok(original.clone()))
                        {
                            Ok(original) => match original_names.current_for_value(vm, &original) {
                                Ok(()) => ok(original),
                                Err(error) => crate::command::completion_from_cmd_error(vm, error),
                            },
                            Err(error) => {
                                crate::command::completion_from_cmd_error(vm, error.into())
                            }
                        };
                    }
                    [name] => {
                        let namespace = match vm
                            .with_jim_current_namespace(|original| Ok(original.clone()))
                        {
                            Ok(namespace) => namespace,
                            Err(error) => {
                                return crate::command::completion_from_cmd_error(vm, error.into());
                            }
                        };
                        (namespace, name)
                    }
                    [namespace, name] => (namespace.clone(), name),
                    _ => {
                        return crate::command::native_wrong_arguments_message(
                            vm,
                            "wrong # args: should be \"namespace canonical ?current? ?name?\"",
                        );
                    }
                };
                match vm.original_jim_canonical_namespace_object(&namespace, name, &|vm| {
                    original_names.check(vm)
                }) {
                    Ok(value) => ok(value),
                    Err(error) => crate::command::completion_from_cmd_error(vm, error),
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
                    return crate::command::native_wrong_arguments_message(vm, synopsis);
                };
                match vm.namespace_object_lookup(original) {
                    // The native object getter itself owns lifetime validity:
                    // C8.4 accepts an original cache for an activated dying token.
                    Ok(namespace) => ok(Value::bool(namespace.is_some())),
                    Err(error) => crate::command::completion_from_cmd_error(vm, error),
                }
            }
            "parent" | "children" => {
                if subcommand_name == "parent" && name_policy.recipe().is_jim084() {
                    let original = if let Some(original) = rest.first() {
                        original.native_lifetime_lease().into_value()
                    } else {
                        if let crate::interp::NamespaceExecutionContext::OriginalJim(context) =
                            vm.current_namespace_execution_context()
                            && let Err(error) =
                                context.materialize_using(vm, &|vm| original_names.check(vm))
                        {
                            return crate::command::completion_from_cmd_error(vm, error);
                        }
                        match vm.with_jim_current_namespace(|original| {
                            Ok(original.native_lifetime_lease().into_value())
                        }) {
                            Ok(original) => original,
                            Err(error) => {
                                return crate::command::completion_from_cmd_error(vm, error.into());
                            }
                        }
                    };
                    return match vm.original_jim_namespace_parent_object(&original, &|vm| {
                        original_names.check(vm)
                    }) {
                        Ok(value) => ok(value),
                        Err(error) => crate::command::completion_from_cmd_error(vm, error),
                    };
                }
                let namespace = match rest.first() {
                    Some(original) => match vm.namespace_object_lookup(original) {
                        Ok(Some(namespace)) => namespace,
                        Ok(None) => {
                            let written = match original_names.read(vm, original) {
                                Ok(bytes) => bytes,
                                Err(error) => {
                                    return crate::command::completion_from_cmd_error(vm, error);
                                }
                            };
                            return vm.namespace_lookup_error_at(
                                &written,
                                if subcommand_name == "parent" {
                                    tcl_syntax::naming::NativeNamespaceLookupOperation::Parent
                                } else {
                                    tcl_syntax::naming::NativeNamespaceLookupOperation::Children
                                },
                            );
                        }
                        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                    },
                    None if subcommand_name == "parent" => {
                        match original_names.current_namespace_token(vm) {
                            Ok(namespace) => namespace,
                            Err(error) => {
                                return crate::command::completion_from_cmd_error(vm, error);
                            }
                        }
                    }
                    None => vm.current_ns_id(),
                };
                let selected_current = vm.native_namespace_token_has_owner(namespace);
                if let Err(error) = original_names.require(
                    vm,
                    selected_current,
                    "selected original parent namespace holder",
                ) {
                    return crate::command::completion_from_cmd_error(vm, error);
                }
                if subcommand_name == "parent" {
                    match tcl_cmd_core::namespace::parent_original(vm, namespace) {
                        Ok(parent) => ok(parent),
                        Err(error) => vm.refuse_host_command(error.to_string()),
                    }
                } else {
                    let pattern = match rest
                        .get(1)
                        .map(|value| original_names.read(vm, value))
                        .transpose()
                    {
                        Ok(bytes) => bytes,
                        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                    };
                    let selected_current = vm.native_namespace_token_has_owner(namespace);
                    if let Err(error) = original_names.require(
                        vm,
                        selected_current,
                        "selected original children namespace after pattern getter",
                    ) {
                        return crate::command::completion_from_cmd_error(vm, error);
                    }
                    match tcl_cmd_core::namespace::children_tokens_checked(
                        vm,
                        namespace,
                        pattern.as_deref(),
                    ) {
                        Ok(children) => {
                            match tcl_cmd_core::namespace::children_original(vm, &children) {
                                Ok(value) => ok(value),
                                Err(error) => vm.refuse_host_command(error.to_string()),
                            }
                        }
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
                    .map(|word| original_names.read(vm, word))
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(words) => words,
                    Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                };
                let Some(protocol) = vm.name_policy_protocol() else {
                    return vm.refuse_host_command(
                        "namespace which name protocol is unavailable".into(),
                    );
                };
                let Some((kind, name_index)) =
                    tcl_cmd_core::namespace::which_request_in(protocol.recipe(), &words)
                else {
                    return crate::command::native_wrong_arguments_message(
                        vm,
                        "wrong # args: should be \"namespace which ?-command? ?-variable? name\"",
                    );
                };
                let result = match kind {
                    tcl_cmd_core::namespace::WhichKind::Variable => {
                        tcl_cmd_core::namespace::which_variable_bytes_checked(
                            vm,
                            &words[name_index],
                        )
                        .map_err(tcl_cmd_core::CmdError::from)
                    }
                    tcl_cmd_core::namespace::WhichKind::Command => {
                        vm.native_namespace_command_name(&rest[name_index], false)
                    }
                };
                match result {
                    Ok(bytes) => {
                        ok(bytes.map_or_else(Value::empty, Value::from_native_string_bytes))
                    }
                    Err(error) => crate::command::completion_from_cmd_error(vm, error),
                }
            }
            "origin" => {
                if rest.len() != 1 {
                    return crate::command::completion_from_cmd_error(
                        vm,
                        tcl_cmd_core::CmdError::wrong_args(synopsis),
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
                        let name = match original_names.read(vm, &rest[0]) {
                            Ok(name) => name,
                            Err(error) => {
                                return crate::command::completion_from_cmd_error(vm, error);
                            }
                        };
                        namespace_command_lookup_error(
                            vm,
                            &name,
                            b"invalid command name ",
                            b"",
                            b"COMMAND",
                        )
                    }
                    Err(error) => crate::command::completion_from_cmd_error(vm, error),
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
                    return match tcl_cmd_core::namespace::delete_original_with_selected_retirement(
                        vm,
                        rest,
                        |vm, selected| original_names.retire_selected_namespace(vm, selected),
                    ) {
                        Ok(None) => ok(Value::empty()),
                        Ok(Some(index)) => {
                            let written = match original_names.read(vm, &rest[index]) {
                                Ok(bytes) => bytes,
                                Err(error) => {
                                    return crate::command::completion_from_cmd_error(vm, error);
                                }
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
                    let written = match original_names.read(vm, original) {
                        Ok(bytes) => bytes,
                        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
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
                                let written = match original_names.read(vm, e) {
                                    Ok(bytes) => bytes,
                                    Err(error) => {
                                        return crate::command::completion_from_cmd_error(
                                            vm, error,
                                        );
                                    }
                                };
                                return vm.namespace_lookup_error_bytes(&written);
                            }
                            Err(error) => {
                                return crate::command::completion_from_cmd_error(vm, error);
                            }
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
                    let bytes = match original_names.read(vm, p) {
                        Ok(bytes) => bytes,
                        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
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
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

/// `namespace upvar ns ?otherVar myVar ...?` — link variables in `ns` into
/// the active frame. Subcommand spelling and release availability are resolved
/// from the registry above; this function owns only the storage operation.
fn ns_upvar(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        let Some(policy) = vm.name_policy_protocol() else {
            return vm.refuse_host_command("namespace upvar naming protocol is unavailable".into());
        };
        let Some(grammar) = vm
            .actual_native_invocation_dialect()
            .native_namespace_upvar_protocol()
        else {
            return vm
                .refuse_host_command("namespace upvar argument grammar is unavailable".into());
        };
        let arguments = match grammar.arguments(rest.len()) {
            Ok(arguments) => arguments,
            Err(usage) => return crate::command::native_wrong_args_bytes(vm, usage),
        };
        let namespace_bytes = match original_names.read(vm, &rest[0]) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        if policy.recipe().is_jim084() {
            let Some(protocol) = vm
                .actual_native_invocation_dialect()
                .native_jim_lookup_protocol()
            else {
                return vm.refuse_host_command(
                    "Jim namespace upvar target protocol is unavailable".into(),
                );
            };
            let canonical = match vm.jim_namespace_upvar_canonical_bytes(&namespace_bytes) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
            };
            let mut argv = vec![Value::string("0")];
            for (target_word, local_word) in arguments.pairs() {
                let other = match original_names.read(vm, &rest[target_word]) {
                    Ok(bytes) => bytes,
                    Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                };
                let target = protocol.namespace_upvar_target(canonical.as_bytes(), &other);
                argv.push(Value::from_string_bytes(target));
                argv.push(local_word.map_or_else(Value::empty, |word| rest[word].clone()));
            }
            return vm.invoke_command_value_at(
                tcl_core_types::ROOT_NS,
                &Value::string("upvar"),
                &argv,
                &[],
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            );
        }
        let namespace = match vm.namespace_object_lookup(&rest[0]) {
            Ok(Some(namespace)) => namespace,
            Ok(None) => return vm.namespace_lookup_error_bytes(&namespace_bytes),
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        for (target_word, local_word) in arguments.pairs() {
            let local_word = local_word.expect("selected C grammar has complete pairs");
            let local = match original_names.read(vm, &rest[local_word]) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            let local = match policy.recipe().namespace_upvar_local_input(&local) {
                Ok(local) => Value::new_native_string_bytes(local.selected()),
                Err(error) => return vm.refuse_host_command(error.to_string()),
            };
            let selected_current = vm.native_namespace_token_has_owner(namespace);
            if let Err(error) = original_names.require(
                vm,
                selected_current,
                "selected original namespace upvar target",
            ) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
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
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
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

/// Native namespace text reporting selects its purpose-specific input extent.
fn ns_text_op(vm: &mut Vm, rest: &[Value], tail: bool) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
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
        let bytes = match original_names.read(vm, original) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let Some(policy) = vm.name_policy_protocol() else {
            return vm.refuse_host_command("native namespace text protocol is unavailable".into());
        };
        let result = match policy.recipe().namespace_text_result(&bytes, tail) {
            NativeNamespaceTextResult::Original => original.clone(),
            NativeNamespaceTextResult::SharedEmpty => match vm.native_jim_object_context() {
                Ok(context) => context.empty_object().clone(),
                Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
            },
            NativeNamespaceTextResult::Counted(bytes) => Value::from_native_string_bytes(bytes),
            NativeNamespaceTextResult::Append(bytes) => {
                let Some(append) = vm
                    .native_invocation_dialect()
                    .native_object_append_protocol(None)
                else {
                    return vm.refuse_host_command(
                        "namespace text append protocol is unavailable".into(),
                    );
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
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_export_tests;

#[cfg(test)]
mod original_ensemble_tests;

#[cfg(test)]
mod original_jim_namespace_text_result_tests;

#[cfg(test)]
mod original_jim_parent_current_holder_tests;

#[cfg(all(test, debug_assertions))]
mod original_c_parent_current_holder_tests;

#[cfg(test)]
mod original_jim_canonical_current_holder_tests;

#[cfg(test)]
mod original_c_namespace_error_log_tests;
