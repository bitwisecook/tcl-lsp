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

use tcl_registry::native_eval_object::{EvalObjectPurpose, NativeNamespaceFramePurpose};
use tcl_runtime_api::completion_options::ControlOptionPolicy;
use tcl_runtime_api::{Code, Completion};

use crate::command::{err_with_code, settle_control_options};
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use tcl_dialect::model::surface_admits;
use tcl_syntax::native_namespace_name::NativeEnsembleNamespaceResultStage;

mod native_jim_helpers;
mod native_jim_subcommands;

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
    body: NamespaceBody<'_>,
    frame_purpose: NativeNamespaceFramePurpose,
    create: bool,
) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        // Native eval first resolves the original namespace object. Creation on a
        // miss uses its written spelling, without installing a cache for that miss.
        let token = match vm.namespace_object_lookup(original) {
            Ok(Some(token)) => token,
            Ok(None) => {
                let written = match original_names.read(vm, original) {
                    Ok(bytes) => bytes,
                    Err(error) => return crate::command::completion_from_cmd_error(vm, error),
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
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let selected_current = vm.native_namespace_token_has_owner(token);
        if let Err(error) = original_names.require(
            vm,
            selected_current,
            "selected original namespace evaluation holder",
        ) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        // Native handlers fetch their frame argv after namespace lookup, whose
        // getters can independently withdraw an active ensemble root rewrite.
        let call_argv = match namespace_frame_arguments(vm, frame_purpose, &original_names) {
            Ok(arguments) => arguments,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        vm.push_ns_eval_token_frame(token, call_argv);
        vm.enter_ns_script();
        let result = eval_namespace_body(vm, body);
        // The namespace the script ran in names its error frame (C's
        // `NsEval_Callback`): read while it is still current.
        let failed = match &result {
            Ok(completion) => completion.code == Code::Error,
            Err(_) => true,
        };
        let frame_namespace = if failed {
            tcl_cmd_core::namespace::current_original(vm)
                .ok()
                .map(|fqn| fqn.string_bytes().to_vec())
        } else {
            None
        };
        vm.leave_ns_script();
        vm.pop_ns();
        vm.pop_call_frame();
        if let Err(error) = original_names.check(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let completion = match result {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(vm, error),
        };
        if completion.code == Code::Error
            && let Some(fqn) = frame_namespace
        {
            let command = match frame_purpose {
                NativeNamespaceFramePurpose::Eval => "eval",
                NativeNamespaceFramePurpose::Inscope => "inscope",
            };
            let text =
                tcl_cmd_core::namespace::script_frame_text(command, &fqn, vm.runtime_version());
            let line = vm.error_line();
            vm.append_frame_text(&format!("{} line {line}", String::from_utf8_lossy(&text)));
        }
        settle_control_options(vm, completion, ControlOptionPolicy::FRESH_FORWARDED)
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

/// Original body operands retain their own source and concatenation protocols.
/// Assembly occurs in the already selected namespace frame, after argv is set.
#[derive(Clone, Copy)]
enum NamespaceBody<'a> {
    Eval(&'a [Value]),
    Inscope {
        script: &'a Value,
        extra: &'a [Value],
        jim: bool,
    },
}

fn eval_namespace_body(
    vm: &mut Vm,
    body: NamespaceBody<'_>,
) -> Result<Completion<Value>, crate::error::TclError> {
    let concatenated;
    let (body, purpose, location) = match body {
        NamespaceBody::Eval([original]) => (
            original,
            EvalObjectPurpose::NamespaceBody,
            original.source_location(),
        ),
        NamespaceBody::Eval(parts) => {
            concatenated = tcl_cmd_core::list::concat_selected(vm, parts).map_err(|error| {
                crate::error::TclError::from_completion(crate::command::completion_from_cmd_error(
                    vm, error,
                ))
            })?;
            (&concatenated, EvalObjectPurpose::NamespaceConcat, None)
        }
        NamespaceBody::Inscope { script, extra, jim } => {
            let location = extra.is_empty().then(|| script.source_location()).flatten();
            if extra.is_empty() && !jim {
                (script, EvalObjectPurpose::NamespaceBody, location)
            } else {
                concatenated = inscope_script(vm, script, extra)
                    .map_err(crate::error::TclError::from_completion)?;
                (&concatenated, EvalObjectPurpose::NamespaceConcat, location)
            }
        }
    };
    vm.eval_original_script_value(body, purpose, location)
}

/// Carry the genuine invocation values into the selected namespace frame.
/// A worker selector manufactured only for local dispatch is never an argv word.
fn namespace_frame_arguments(
    vm: &mut Vm,
    purpose: NativeNamespaceFramePurpose,
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
) -> Result<Vec<Value>, tcl_cmd_core::CmdError> {
    use tcl_registry::native_eval_object::NativeNamespaceFrameArguments;
    use tcl_syntax::value::ValueError;

    original_names.check(vm)?;
    let recipe = vm
        .eval_object_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original namespace frame argument issuer",
        ))?
        .namespace_frame_arguments(purpose);
    if recipe == NativeNamespaceFrameArguments::Empty {
        return Ok(Vec::new());
    }
    let entered = vm
        .native_invocation
        .arguments
        .as_ref()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original namespace frame invocation vector",
        ))?
        .lifetime_view();
    let words: Vec<_> = entered
        .elements()?
        .iter()
        .map(Value::native_lifetime_lease)
        .collect();
    let words = if recipe == NativeNamespaceFrameArguments::EnsembleRoot {
        let rewrites: Vec<_> = vm
            .native_invocation
            .usage_rewrites
            .iter()
            .map(|rewrite| tcl_cmd_core::ensemble::ArgumentUsageRewrite {
                original_prefix: rewrite
                    .original_prefix
                    .iter()
                    .map(Value::native_lifetime_lease)
                    .collect(),
                removed_words: rewrite.removed_words,
            })
            .collect();
        tcl_cmd_core::ensemble::rewrite_argument_usage(&words, &rewrites)
    } else {
        words
    };
    for word in &words {
        original_names.current_for_value(vm, word.value())?;
    }
    entered.elements()?;
    original_names.check(vm)?;
    Ok(words
        .into_iter()
        .map(crate::value::NativeObjectLifetimeLease::into_value)
        .collect())
}

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

fn ns_export(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        if rest.is_empty() {
            return ok(Value::list(
                vm.exports_get()
                    .into_iter()
                    .map(|bytes| Value::from_native_string_bytes(bytes.as_bytes()))
                    .collect(),
            ));
        }
        let Some(policy) = vm.name_policy_protocol() else {
            return vm.refuse_host_command("native export protocol is unavailable".into());
        };
        if policy.recipe().is_jim084() {
            return ok(Value::empty());
        }
        let mut values = rest;
        if let Some(first) = values.first() {
            let bytes = match original_names.read(vm, first) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            let Ok(clear) = policy.recipe().namespace_export_clears(&bytes) else {
                return vm
                    .refuse_host_command("native export control projection is unavailable".into());
            };
            if clear {
                vm.clear_exports();
                values = &values[1..];
            }
        }
        // Each native getter is followed by validation and publication before
        // visiting the next original operand. Earlier clear/exports survive failure.
        for value in values {
            let bytes = match original_names.read(vm, value) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            let selected = match policy.recipe().namespace_pattern_input(
                &bytes,
                tcl_syntax::naming::NativeNamePurpose::NamespaceExportPattern,
            ) {
                Ok(selected) => tcl_core_types::NameBytes::from(selected.selected()),
                Err(_) => {
                    return vm.refuse_host_command(
                        "native export pattern projection is unavailable".into(),
                    );
                }
            };
            if tcl_syntax::naming::is_qualified(selected.as_bytes()) {
                let mut message = b"invalid export pattern \"".to_vec();
                message.extend_from_slice(selected.as_bytes());
                message.extend_from_slice(b"\": pattern can't specify a namespace");
                return crate::command::completion_from_cmd_error(
                    vm,
                    tcl_cmd_core::CmdError::new_bytes(message),
                );
            }
            vm.add_exports(std::slice::from_ref(&selected));
        }
        ok(Value::empty())
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

/// `namespace import ?-force? ?pattern ...?` (`NamespaceImportCmd` /
/// `Tcl_Import`, `tclNamesp.c:3668-3732`).
///
/// `-force` is positional in the same way: only `objv[1]` is read as the flag,
/// and every later word is a pattern — including a trailing `-force`, which
/// then fails the "the pattern must name a source namespace" check.
fn import_source_name_patterns(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let operation = match crate::interp::native_operation_currency::NativeOperationScope::enter(vm)
    {
        Ok(operation) => operation,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    for pattern in rest {
        let reached = tcl_syntax::value::ValueOps::native_string_bytes(vm, pattern);
        if let Err(error) = operation.check(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let original = match reached {
            Ok(bytes) => bytes,
            Err(error) => {
                return crate::command::completion_from_cmd_error(vm, error.into());
            }
        };
        let result = vm.import_source_names(&original, &operation);
        if let Err(error) = operation.check(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if result.code != Code::Ok {
            return result;
        }
    }
    ok(Value::empty())
}

fn ns_import(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        if vm.dialect_profile().namespace_import_binding()
            == Some(tcl_dialect::NamespaceImportBinding::SourceName)
        {
            return import_source_name_patterns(vm, rest);
        }
        if rest.is_empty() {
            // Pinned C8.4 predates the later imported-command query form.
            let Some(protocol) = vm
                .actual_native_invocation_dialect()
                .native_namespace_name_protocol()
            else {
                return vm
                    .refuse_host_command("native import query protocol is unavailable".into());
            };
            if !protocol.import_queries_existing_commands() {
                return ok(Value::empty());
            }
            return ok(Value::list(
                vm.imported_command_tails()
                    .into_iter()
                    .map(|bytes| Value::from_native_string_bytes(bytes.as_bytes()))
                    .collect(),
            ));
        }
        let Some(policy) = vm.name_policy_protocol() else {
            return vm.refuse_host_command("native import protocol is unavailable".into());
        };
        let first = match original_names.read(vm, &rest[0]) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let Ok(allow_overwrite) = policy.recipe().namespace_import_forces(&first) else {
            return vm.refuse_host_command("native import control is unavailable".into());
        };
        for operand in &rest[usize::from(allow_overwrite)..] {
            // Native repeats the original first getter when it was not -force.
            let pattern = match original_names.read(vm, operand) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            let Ok(projection) = policy.recipe().namespace_pattern_input(
                &pattern,
                tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern,
            ) else {
                return vm
                    .refuse_host_command("native import pattern projection is unavailable".into());
            };
            let selected = projection.selected();
            match vm.original_c_import_preload(selected, &original_names) {
                Ok(Some(completion)) => return completion,
                Ok(None) => {}
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            }
            let destination = match original_names.current_namespace_token(vm) {
                Ok(destination) => destination,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            let imported =
                match tcl_cmd_core::namespace::import_pattern_checked(vm, destination, selected) {
                    Ok(Ok(imported)) => imported,
                    Ok(Err(problem)) => return err(problem.message()),
                    Err(error) => {
                        return crate::command::completion_from_cmd_error(vm, error.into());
                    }
                };
            if let Err(problem) =
                vm.import_commands_selected(selected, &imported, allow_overwrite, &original_names)
            {
                return problem.completion(vm);
            }
            if let Err(error) = original_names.check(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
        }
        ok(Value::empty())
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

/// `namespace ensemble create|exists|configure` (`TclNamespaceEnsembleCmd`,
/// `tclEnsemble.c:140`). The subcommand word resolves through the shared
/// `ensembleSubcommands` table, so `namespace ensemble cr` is `create` and a
/// miss reads `bad subcommand "…": must be configure, create, or exists`.
fn ns_ensemble(vm: &mut Vm, selector: &Value, rest: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        if !rest.is_empty() {
            let original = match original_names.read(vm, selector) {
                Ok(original) => original,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
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
        let index = match vm.native_static_option_index_checked(
            op,
            tcl_cmd_core::ensemble::SUBCOMMANDS.names(),
            false,
            "subcommand",
        ) {
            Ok(index) => index,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        if let Err(error) = original_names.check(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        match index {
            // configure
            0 => ns_ensemble_configure(vm, args),
            // create
            1 => ns_ensemble_create(vm, args),
            // exists
            _ => match args {
                [cmd] => {
                    let bytes = match original_names.read(vm, cmd) {
                        Ok(bytes) => bytes,
                        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
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
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
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

fn ensemble_original_list_member(
    vm: &mut Vm,
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
    elements: &crate::NativeListItems,
    index: usize,
) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError> {
    let value = elements.elements()?.get(index).ok_or(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original ensemble List member index",
        ),
    )?;
    let bytes = original_names.read(vm, value)?;
    elements.elements()?;
    Ok(bytes)
}

fn ensemble_original_list_words(
    vm: &mut Vm,
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
    elements: &crate::NativeListItems,
) -> Result<Vec<tcl_core_types::NameBytes>, tcl_cmd_core::CmdError> {
    (0..elements.elements()?.len())
        .map(|index| {
            ensemble_original_list_member(vm, original_names, elements, index)
                .map(|bytes| tcl_core_types::NameBytes::from(bytes.as_ref()))
        })
        .collect()
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
    let original_names = crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let reached = (|| {
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
                let elems = original_names
                    .list(vm, val, protocol)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                let names = ensemble_original_list_words(vm, &original_names, &elems)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                opts.subcommands = (!names.is_empty()).then_some(names);
                opts.originals.subcommands = opts
                    .subcommands
                    .as_ref()
                    .map(|_| EnsembleObjectRole::new(NativeEnsembleRoot::pending(val)));
            }
            SharedOption::Parameters => {
                let elems = original_names
                    .list(vm, val, protocol)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                opts.parameters = ensemble_original_list_words(vm, &original_names, &elems)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                opts.originals.parameters = (!opts.parameters.is_empty())
                    .then(|| EnsembleObjectRole::new(NativeEnsembleRoot::pending(val)));
            }
            SharedOption::Prefixes => {
                opts.prefixes = tcl_syntax::value::ValueOps::as_bool(vm, val)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            }
            SharedOption::Unknown => {
                let elements = original_names
                    .list(vm, val, protocol)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                let words = ensemble_original_list_words(vm, &original_names, &elements)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                opts.unknown = (!words.is_empty()).then_some(words);
                opts.originals.unknown = opts
                    .unknown
                    .as_ref()
                    .map(|_| EnsembleObjectRole::new(NativeEnsembleRoot::pending(val)));
            }
        }
        Ok(())
    })();
    original_names
        .finish(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    reached
}

fn apply_ensemble_map(
    opts: &mut EnsembleOptions,
    val: &Value,
    vm: &mut Vm,
    creating: bool,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<(), Completion<Value>> {
    let original_names = crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let reached = (|| {
        use crate::command::native_ensemble_objects::NativeEnsembleRoot;
        use tcl_cmd_core::ensemble::EnsembleObjectRole;
        let mut search = original_names
            .dictionary_search(vm, val, protocol)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        let mut patched = None;
        let mut map = Vec::new();
        while let Some((key, value)) = search
            .next_original_pair_checked(&mut || original_names.current_for_value(vm, val))
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?
        {
            let elements = original_names
                .list(vm, &value, protocol)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
            if elements
                .elements()
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?
                .is_empty()
            {
                return Err(err_with_code(
                    "ensemble subcommand implementations must be non-empty lists",
                    "TCL ENSEMBLE EMPTY_TARGET",
                ));
            }
            let original = ensemble_original_list_member(vm, &original_names, &elements, 0)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
            let mut prefix = None;
            let selected = vm
                .native_ensemble_map_prefix_input(
                    tcl_runtime_api::Namespaces::current(vm),
                    &original,
                )
                .map_err(|error| vm.refuse_host_command(error.to_string()))?;
            if let std::borrow::Cow::Owned(qualified) = selected {
                let mut words = elements
                    .elements()
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?
                    .to_vec();
                words[0] = Value::from_native_string_bytes(qualified);
                let copy = if creating {
                    Value::native_list_constructor(
                        elements
                            .elements()
                            .map_err(|error| {
                                crate::command::completion_from_cmd_error(vm, error.into())
                            })?
                            .to_vec(),
                        protocol,
                    )
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
            let name = original_names
                .read(vm, &key)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
            let words = if let Some(prefix) = prefix {
                original_names
                    .list(vm, &prefix, protocol)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?
            } else {
                elements
            };
            let words = ensemble_original_list_words(vm, &original_names, &words)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?
                .into_iter()
                .map(Some)
                .collect();
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
    })();
    original_names
        .finish(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
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

fn ns_ensemble_create_entered(
    vm: &mut Vm,
    args: &[Value],
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
) -> Completion<Value> {
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
        let resolved = match vm.native_static_option_index_checked(
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
            command = Some(match original_names.read(vm, &pair[1]) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            });
            continue;
        };
        if let Err(completion) = apply_shared_option(&mut opts, shared, &pair[1], vm, true) {
            return completion;
        }
    }
    // The default command is the namespace itself; an explicit -command binds in
    // the current namespace when unqualified.
    let selected_current = vm.native_namespace_token_has_owner(ns_id);
    if let Err(error) = original_names.require(
        vm,
        selected_current,
        "selected original ensemble creation namespace",
    ) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let slot = match vm.native_namespace_ensemble_publication_slot(command.as_deref()) {
        Ok(slot) => slot,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let reporting =
        tcl_syntax::naming::root_unrooted_key_bytes(&vm.command_slot_display_bytes(&slot));
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
    let selected_current = vm.native_namespace_token_has_owner(ns_id);
    if let Err(error) = original_names.require(
        vm,
        selected_current,
        "selected original ensemble holder before publication",
    ) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    vm.register_namespace_ensemble_slot(slot, &token);
    if let Err(error) = original_names.check(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let selected_current = !token.is_deleted() && vm.native_namespace_token_has_owner(ns_id);
    if let Err(error) = original_names.require(
        vm,
        selected_current,
        "selected original ensemble before object-role adoption",
    ) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    token.config().originals.activate();
    ok(Value::from_native_string_bytes(reporting))
}

fn ns_ensemble_create(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = ns_ensemble_create_entered(vm, args, &original_names);
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

type NamespaceEnsembleToken = std::rc::Rc<
    tcl_cmd_core::ensemble::EnsembleToken<crate::command::EnsembleDef, tcl_core_types::NameBytes>,
>;

fn lookup_namespace_ensemble_token(
    vm: &mut Vm,
    written: &[u8],
) -> Result<NamespaceEnsembleToken, Completion<Value>> {
    let selected =
        match vm.lookup_command_bytes_checked(tcl_runtime_api::Namespaces::current(vm), written) {
            Ok(selected) => selected,
            Err(error) => return Err(vm.refuse_host_command(error.to_string())),
        };
    match selected {
        Some((_, crate::command::Command::Ensemble(token))) => Ok(token),
        Some(_) => Err(namespace_command_lookup_error(
            vm,
            written,
            b"",
            b" is not an ensemble command",
            b"ENSEMBLE",
        )),
        None => Err(namespace_command_lookup_error(
            vm,
            written,
            b"unknown command ",
            b"",
            b"COMMAND",
        )),
    }
}

fn report_namespace_ensemble_configuration(
    vm: &mut Vm,
    token: &NamespaceEnsembleToken,
    def: &crate::command::EnsembleDef,
    rest: &[Value],
    options: tcl_registry::native_ensemble::NativeEnsembleConfigurationProtocol,
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
) -> Option<Completion<Value>> {
    if rest.is_empty() {
        return Some(ensemble_configuration_snapshot(vm, def, options));
    }
    if let [only] = rest {
        return Some(
            match vm.native_static_option_index_checked(
                only,
                options.configure_options(),
                false,
                "option",
            ) {
                Ok(index) => {
                    let selected_current =
                        !token.is_deleted() && vm.native_namespace_token_has_owner(def.namespace);
                    if let Err(error) = original_names.require(
                        vm,
                        selected_current,
                        "selected original ensemble report target",
                    ) {
                        return Some(crate::command::completion_from_cmd_error(vm, error));
                    }
                    match ensemble_option_value(
                        vm,
                        def,
                        options.configure_option(index),
                        NativeEnsembleNamespaceResultStage::Option,
                    ) {
                        Ok(value) => ok(value),
                        Err(error) => crate::command::completion_from_cmd_error(vm, error),
                    }
                }
                Err(error) => crate::command::completion_from_cmd_error(vm, error),
            },
        );
    }
    None
}

fn ns_ensemble_configure_entered(
    vm: &mut Vm,
    args: &[Value],
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
) -> Completion<Value> {
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
    let written = match original_names.read(vm, cmd_val) {
        Ok(bytes) => bytes,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    // `Tcl_FindEnsemble` reports the two failures separately: a name that
    // resolves to no command at all is `unknown command "x"` (the
    // `Tcl_FindCommand` miss), while a name that *is* a command but carries a
    // different implementation is `"x" is not an ensemble command`. Collapsing
    // them into the second message misreports a plain typo.
    let token = match lookup_namespace_ensemble_token(vm, &written) {
        Ok(token) => token,
        Err(error) => return error,
    };
    let selected_current = !token.is_deleted();
    if let Err(error) =
        original_names.require(vm, selected_current, "selected original ensemble token")
    {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let def = token.config();
    if let Some(report) =
        report_namespace_ensemble_configuration(vm, &token, &def, rest, options, original_names)
    {
        return report;
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
        let resolved = match vm.native_static_option_index_checked(
            &pair[0],
            options.configure_options(),
            false,
            "option",
        ) {
            Ok(index) => options.configure_option(index),
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let selected_current =
            !token.is_deleted() && vm.native_namespace_token_has_owner(def.namespace);
        if let Err(error) = original_names.require(
            vm,
            selected_current,
            "selected original ensemble target after option getter",
        ) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let Some(shared) = resolved.shared() else {
            return err_with_code("option -namespace is read-only", "TCL ENSEMBLE READ_ONLY");
        };
        if let Err(completion) = apply_shared_option(&mut opts, shared, &pair[1], vm, false) {
            return completion;
        }
    }
    let selected_current =
        !token.is_deleted() && vm.native_namespace_token_has_owner(def.namespace);
    if let Err(error) = original_names.require(
        vm,
        selected_current,
        "selected original ensemble target before adoption",
    ) {
        return crate::command::completion_from_cmd_error(vm, error);
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
    let selected_current =
        !token.is_deleted() && vm.native_namespace_token_has_owner(def.namespace);
    if let Err(error) = original_names.require(
        vm,
        selected_current,
        "selected original ensemble after configuration retirement",
    ) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    vm.note_ensemble_configuration_changed(&token);
    ok(Value::empty())
}

/// `namespace ensemble configure cmdname ?-option? ?value ...?`
/// (`tclEnsemble.c:377-630`): with no options an ordered key/value List of settings, with a
/// single option that option's value, otherwise `-option value` updates.
/// `ensembleConfigOptions` differs from the create table — it carries
/// `-namespace` (readable, never writable) and has no `-command`.
fn ns_ensemble_configure(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = ns_ensemble_configure_entered(vm, args, &original_names);
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
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
        let value = match ensemble_option_value(
            vm,
            def,
            option,
            NativeEnsembleNamespaceResultStage::Snapshot,
        ) {
            Ok(value) => value,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        pairs.push(Value::string(option.name()));
        pairs.push(value);
    }
    ok(Value::native_list_constructor(pairs, recipe.protocol()))
}

/// One `namespace ensemble configure` option's value.
fn ensemble_option_value(
    vm: &mut Vm,
    def: &crate::command::EnsembleDef,
    option: tcl_cmd_core::ensemble::ConfigOption,
    stage: NativeEnsembleNamespaceResultStage,
) -> Result<Value, tcl_cmd_core::CmdError> {
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
        ConfigOption::Namespace => vm.original_ensemble_namespace_result(def.namespace, stage)?,
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
    let create = vm
        .name_policy_protocol()
        .is_some_and(|policy| policy.recipe().is_jim084());
    eval_in_ns(
        vm,
        ns,
        NamespaceBody::Inscope { script, extra, jim },
        NativeNamespaceFramePurpose::Inscope,
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
    eval_in_ns(
        vm,
        ns,
        NamespaceBody::Eval(body_parts),
        NativeNamespaceFramePurpose::Eval,
        true,
    )
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_upvar_fixture_tests;

#[cfg(test)]
mod original_export_tests;

#[cfg(test)]
mod original_ensemble_tests;

#[cfg(test)]
mod native_store_tests;

#[cfg(test)]
mod native_map_prefix_tests;

#[cfg(test)]
mod original_jim_import_tests;

#[cfg(test)]
mod original_jim_namespace_text_result_tests;

#[cfg(test)]
mod original_jim_parent_current_holder_tests;

#[cfg(all(test, debug_assertions))]
mod original_c_parent_current_holder_tests {
    use super::*;

    #[test]
    fn parent_without_operand_uses_the_entered_namespace_incarnation() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect(release),
            );
            let parent = vm.activate_namespace_operand(b"::outer").unwrap();
            let entered = vm.activate_namespace_operand(b"::outer::inner").unwrap();
            vm.push_ns_token("outer::inner", entered);
            let arguments = [Value::string("parent")];
            let result = cmd_namespace(&mut vm, &arguments);
            assert_eq!(result.code, Code::Ok);
            assert_eq!(result.result.string_bytes().as_ref(), b"::outer");
            assert!(vm.native_namespace_token_has_owner(parent));

            assert!(vm.delete_namespace_operand(b"::outer::inner").unwrap());
            let replacement = vm.activate_namespace_operand(b"::outer::inner").unwrap();
            assert_ne!(entered, replacement);
            // Tcl_DeleteNamespace detaches an active root's physical parent;
            // the original frame retains that birth, independently of recreation.
            let result = cmd_namespace(&mut vm, &arguments);
            assert_eq!(result.code, Code::Ok);
            assert_eq!(result.result.string_bytes().as_ref(), b"");
            assert_eq!(vm.current_ns_id(), entered);
            vm.pop_ns();
            assert!(!vm.native_namespace_token_has_owner(entered));

            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect(release),
            );
            let entered = vm.activate_namespace_operand(b"::outer::inner").unwrap();
            vm.push_ns_token("outer::inner", entered);
            let original =
                crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm)
                    .unwrap();
            assert!(vm.delete_namespace_operand(b"::outer::inner").unwrap());
            assert!(original.current_namespace_token(&mut vm).is_err());
            let first = vm.execution_refusal.clone().unwrap();
            let result = cmd_namespace(&mut vm, &arguments);
            assert_eq!(result.code, Code::Error);
            assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
        }
    }
}

#[cfg(test)]
mod original_jim_canonical_current_holder_tests;

#[cfg(test)]
mod original_c_import_tests;

#[cfg(test)]
mod original_c_namespace_error_log_tests;
