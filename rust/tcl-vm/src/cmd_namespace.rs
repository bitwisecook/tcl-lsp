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
        let target = format!("::tcl::namespace::{member}");
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
mod tests {
    use super::*;

    fn capture_namespace_frame(vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        ok(Value::list(vm.frame_argv(vm.current_level()).unwrap()))
    }

    #[test]
    fn namespace_frames_retain_actual_public_and_worker_argument_objects() {
        for version in tcl_dialect::TclVersion::ALL {
            for member in ["eval", "inscope"] {
                let profile =
                    crate::environment::profile_for_dialect(version.dialect_profile_name());
                for worker in [false, true] {
                    if worker
                        && matches!(
                            version,
                            tcl_dialect::TclVersion::V8_4 | tcl_dialect::TclVersion::V8_5
                        )
                    {
                        continue;
                    }
                    let mut vm = crate::native_fixture::interpreter(profile);
                    vm.register("captureNamespaceFrame", capture_namespace_frame);
                    let target = Value::string("n");
                    assert_eq!(
                        vm.eval_source("namespace eval n {}").unwrap().code,
                        Code::Ok
                    );
                    let head = Value::new_native_string_bytes(b"renamed namespace\xff".as_slice());
                    let source = if worker {
                        format!("::tcl::namespace::{member}")
                    } else {
                        "namespace".to_owned()
                    };
                    let renamed = vm.invoke_host_original_object_vector(
                        &Value::string("rename"),
                        &[Value::string(source), head.clone()],
                    );
                    assert_eq!(renamed.code, Code::Ok, "{version:?}/{member}/{worker}");
                    let selector = Value::string(if member == "eval" { "ev" } else { "insc" });
                    let body = Value::list(vec![Value::string("::captureNamespaceFrame")]);
                    let mut arguments = Vec::new();
                    if !worker {
                        arguments.push(selector.clone());
                    }
                    arguments.extend([target.clone(), body.clone()]);
                    let completion = vm.invoke_host_original_object_vector(&head, &arguments);
                    assert_eq!(
                        completion.code,
                        Code::Ok,
                        "{version:?}/{member}/{worker}: {:?}",
                        completion.result.string_bytes()
                    );
                    let returned = completion.result.as_list().unwrap();
                    if version == tcl_dialect::TclVersion::V8_4 && member == "inscope" {
                        assert!(returned.is_empty());
                    } else {
                        assert_eq!(returned.len(), arguments.len() + 1);
                        assert!(returned[0].is_same_object(&head));
                        for (actual, original) in returned[1..].iter().zip(&arguments) {
                            assert!(
                                actual.is_same_object(original),
                                "{version:?}/{member}/{worker}"
                            );
                        }
                    }
                    assert_eq!(vm.current_ns_id(), tcl_runtime_api::ROOT_NS);
                }
            }
        }
    }

    #[test]
    fn jim_namespace_frames_retain_the_actual_eval_and_scripted_tailcall_vectors() {
        use tcl_registry::native_scripted_distribution::NativeScriptedLibrary;
        let mut vm = crate::native_fixture::interpreter_with_scripted_libraries(
            crate::environment::profile_for_dialect("jimtcl"),
            &[NativeScriptedLibrary::NamespaceHelpers],
        );
        vm.register("captureNamespaceFrame", capture_namespace_frame);
        let body = Value::list(vec![Value::string("::captureNamespaceFrame")]);
        let target = Value::string("n");
        let selector = Value::string("eval");
        let head = Value::string("namespace");
        let arguments = [selector.clone(), target.clone(), body.clone()];
        let completion = vm.invoke_host_original_object_vector(&head, &arguments);
        assert_eq!(completion.code, Code::Ok);
        let returned = completion.result.as_list().unwrap();
        assert_eq!(returned.len(), 4);
        assert!(returned[0].is_same_object(&head));
        for (actual, original) in returned[1..].iter().zip(&arguments) {
            assert!(actual.is_same_object(original));
        }
        let inscope = vm.invoke_host_original_object_vector(
            &head,
            &[Value::string("inscope"), target.clone(), body.clone()],
        );
        assert_eq!(inscope.code, Code::Ok);
        let returned = inscope.result.as_list().unwrap();
        // The original distribution helper tailcalls namespace eval with its
        // actual args List as a fourth operand; its caller head is not retained.
        assert_eq!(returned.len(), 5);
        assert_eq!(returned[0].string_bytes().as_ref(), b"namespace");
        assert_eq!(returned[1].string_bytes().as_ref(), b"eval");
        assert!(returned[2].is_same_object(&target));
        assert!(returned[3].is_same_object(&body));
        assert!(returned[4].as_list().unwrap().is_empty());
    }

    #[test]
    fn namespace_frame_transport_borrows_headers_and_requires_original_invocation() {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
        let scope = crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm)
            .unwrap();
        assert!(
            namespace_frame_arguments(&mut vm, NativeNamespaceFramePurpose::Eval, &scope).is_err()
        );
        let words = std::rc::Rc::new(vec![
            Value::string("worker"),
            Value::string("n"),
            Value::string("body"),
        ]);
        let references: Vec<_> = words
            .iter()
            .map(Value::native_object_reference_count)
            .collect();
        vm.native_invocation.arguments = Some(crate::NativeListItems::invocation_view(
            std::rc::Rc::clone(&words),
        ));
        let transported =
            namespace_frame_arguments(&mut vm, NativeNamespaceFramePurpose::Eval, &scope).unwrap();
        for ((original, borrowed), references) in words.iter().zip(&transported).zip(references) {
            assert!(original.is_same_object(borrowed));
            assert_eq!(original.native_object_reference_count(), references);
        }
        drop(transported);
        scope.finish(&mut vm).unwrap();
    }

    #[test]
    fn namespace_frame_fetch_observes_withdrawn_ensemble_rewrite_at_publication() {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
        let scope = crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm)
            .unwrap();
        let worker = Value::string("actual worker");
        let original = Value::string("outer original");
        vm.native_invocation.arguments =
            Some(crate::NativeListItems::invocation_view(std::rc::Rc::new(
                vec![worker.clone(), Value::string("n"), Value::string("body")],
            )));
        vm.native_invocation.usage_rewrites = vec![crate::command::NativeArgumentUsageRewrite {
            original_prefix: vec![original.clone(), Value::string("member")],
            removed_words: 1,
        }];
        let before =
            namespace_frame_arguments(&mut vm, NativeNamespaceFramePurpose::Eval, &scope).unwrap();
        assert!(before[0].is_same_object(&original));
        vm.reset_native_ensemble_rewrite(
            tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::BeforeOrdinaryLookup,
        )
        .unwrap();
        let after =
            namespace_frame_arguments(&mut vm, NativeNamespaceFramePurpose::Eval, &scope).unwrap();
        assert!(after[0].is_same_object(&worker));
        assert_eq!(after.len(), 3);
        scope.finish(&mut vm).unwrap();
    }

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
            // These direct handler tests install the real caller vector explicitly;
            // frame construction no longer invents a public command head.
            let mut words = vec![Value::string("namespace"), Value::string("eval")];
            words.extend_from_slice(&arguments);
            vm.native_invocation.arguments = Some(crate::NativeListItems::invocation_view(
                std::rc::Rc::new(words),
            ));
            let returned = ns_eval(&mut vm, &arguments);
            assert_eq!(returned.code, Code::Return, "{}", profile.name);
            assert_eq!(returned.result.string_bytes().as_ref(), b"x");
            assert_eq!(vm.current_ns_id(), tcl_runtime_api::ROOT_NS);
            let mut words = vec![Value::string("namespace"), Value::string("inscope")];
            words.extend_from_slice(&arguments);
            vm.native_invocation.arguments = Some(crate::NativeListItems::invocation_view(
                std::rc::Rc::new(words),
            ));
            let returned = ns_inscope(&mut vm, &arguments);
            assert_eq!(returned.code, Code::Return, "{}", profile.name);
            let absent = [Value::string("absent"), Value::string("return y")];
            let mut words = vec![Value::string("namespace"), Value::string("inscope")];
            words.extend_from_slice(&absent);
            vm.native_invocation.arguments = Some(crate::NativeListItems::invocation_view(
                std::rc::Rc::new(words),
            ));
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
            let mut vm = if std::ptr::eq(profile, jim) {
                crate::native_fixture::interpreter_with_scripted_libraries(
                    profile,
                    &[tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers],
                )
            } else {
                let mut vm = Vm::with_output(Box::new(std::io::sink()));
                vm.set_dialect_profile(profile);
                vm
            };
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
                let result = if std::ptr::eq(profile, jim) {
                    // Jim's actual namespace code command is a distribution
                    // procedure; native C construction cannot supply its literals.
                    vm.invoke_host_original_object_vector(
                        &Value::string("namespace"),
                        &[Value::string("code"), script.clone()],
                    )
                } else {
                    namespace_code_invocation(
                        &mut vm,
                        std::slice::from_ref(&script),
                        NamespaceInvocation::Public,
                    )
                };
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
    fn code_construction_allocates_fresh_literal_headers_and_retains_original_roles() {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        use tcl_syntax::native_string::NativeStringStorageIdentity;
        for version in tcl_dialect::TclVersion::ALL {
            let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(
                version.dialect_profile_name(),
            ));
            let namespace = vm.activate_namespace_operand(b"opaque\xff").unwrap();
            vm.push_ns_eval_token_frame(namespace, Vec::new());
            let script = Value::new_native_string_bytes(b"command\xff\0tail".as_slice());
            let before = script.native_object_snapshot();
            let first = namespace_code_invocation(
                &mut vm,
                std::slice::from_ref(&script),
                NamespaceInvocation::Public,
            );
            let second = namespace_code_invocation(
                &mut vm,
                std::slice::from_ref(&script),
                NamespaceInvocation::Public,
            );
            assert_eq!(first.code, Code::Ok, "{version:?}");
            assert_eq!(second.code, Code::Ok, "{version:?}");
            assert!(!first.result.is_same_object(&second.result));
            let snapshot = first.result.native_object_snapshot();
            assert!(snapshot.resident.is_none());
            assert!(matches!(snapshot.cache, Cache::List { length: 4, .. }));
            let first_members = first.result.as_list().unwrap();
            let second_members = second.result.as_list().unwrap();
            for (index, bytes) in [b"::namespace".as_slice(), b"inscope".as_slice()]
                .into_iter()
                .enumerate()
            {
                assert!(!first_members[index].is_same_object(&second_members[index]));
                let snapshot = first_members[index].native_object_snapshot();
                assert_eq!(snapshot.resident.as_deref(), Some(bytes));
                assert_eq!(
                    snapshot.storage,
                    Some(NativeStringStorageIdentity::Allocated)
                );
                assert_eq!(snapshot.cache, Cache::None);
            }
            assert_eq!(
                first_members[2].resident_string_bytes().unwrap().as_ref(),
                b"::opaque\xff"
            );
            if matches!(
                version,
                tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1
            ) {
                assert_eq!(
                    first_members[2].native_object_snapshot().cache,
                    Cache::NamespaceName {
                        version,
                        resolved: true
                    }
                );
            } else {
                assert_eq!(first_members[2].native_object_snapshot().cache, Cache::None);
            }
            assert!(first_members[3].is_same_object(&script));
            assert!(second_members[3].is_same_object(&script));
            assert_eq!(script.native_object_snapshot(), before);
            vm.pop_ns();
            vm.pop_call_frame();
        }
    }

    #[test]
    fn jim_code_public_helper_retains_its_original_parsed_literals() {
        use tcl_registry::native_scripted_distribution::NativeScriptedLibrary;
        let mut vm = crate::native_fixture::interpreter_with_scripted_libraries(
            crate::environment::profile_for_dialect("jimtcl"),
            &[NativeScriptedLibrary::NamespaceHelpers],
        );
        let script = Value::new_native_string_bytes(b"command\xff\0tail".as_slice());
        let head = Value::string("namespace");
        let selector = Value::string("code");
        let arguments = [selector.clone(), script.clone()];
        let first = vm.invoke_host_original_object_vector(&head, &arguments);
        // Public completion transports borrow the result owner until retained.
        let _retained_first_result = first.result.clone();
        let second = vm.invoke_host_original_object_vector(&head, &arguments);
        assert_eq!(first.code, Code::Ok);
        assert_eq!(second.code, Code::Ok);
        assert!(!first.result.is_same_object(&second.result));
        let first_members = first.result.as_list().unwrap();
        let second_members = second.result.as_list().unwrap();
        assert_eq!(first_members.len(), 4);
        for index in 0..2 {
            assert!(first_members[index].is_same_object(&second_members[index]));
        }
        assert_eq!(first_members[0].string_bytes().as_ref(), b"::namespace");
        assert_eq!(first_members[1].string_bytes().as_ref(), b"inscope");
        assert!(first_members[3].is_same_object(&script));
        assert!(second_members[3].is_same_object(&script));
        assert_eq!(script.string_bytes().as_ref(), b"command\xff\0tail");
        assert!(!first_members[0].is_same_object(&head));
        assert!(!first_members[1].is_same_object(&selector));
    }

    #[test]
    fn jim_code_direct_native_route_preserves_selected_recognition_but_requires_the_helper_for_construction()
     {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jimtcl"));
        let recognised = Value::string("::namespace inscope ");
        let preserved = namespace_code_invocation(
            &mut vm,
            std::slice::from_ref(&recognised),
            NamespaceInvocation::Public,
        );
        assert_eq!(preserved.code, Code::Ok);
        assert!(preserved.result.is_same_object(&recognised));
        let script = Value::string("command");
        let unsupported = namespace_code_invocation(
            &mut vm,
            std::slice::from_ref(&script),
            NamespaceInvocation::Public,
        );
        assert_eq!(unsupported.code, Code::Error);
        assert!(vm.refused_completion().is_some());
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
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.set_native_engine_profile(tcl_dialect::DialectProfile::find("tcl9.0").unwrap()));
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
        assert!(vm.execution_refusal.is_none(), "{:?}", vm.execution_refusal);
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
                let mut vm = crate::native_fixture::interpreter_with_scripted_libraries(
                    profile,
                    &[tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers],
                );
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
            let mut vm = crate::native_fixture::interpreter_with_scripted_libraries(
                    profile,
                    &[tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers],
                );
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
        let mut vm = crate::native_fixture::interpreter_with_scripted_libraries(
            profile,
            &[tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers],
        );
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
mod original_export_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    use tcl_platform::{
        NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
        NativeIntegerKind,
    };

    fn invoke_export(vm: &mut Vm, patterns: &[Value]) -> Completion<Value> {
        let mut arguments = vec![Value::string("export")];
        arguments.extend_from_slice(patterns);
        // Handler getter order ends before the public evaluator logs an Error argv.
        vm.invoke_command_value_at(
            vm.current_ns_id(),
            &Value::string("namespace"),
            &arguments,
            &[],
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        )
    }

    fn exports(vm: &Vm) -> Vec<Vec<u8>> {
        vm.exports_get()
            .iter()
            .map(|name| name.as_bytes().to_vec())
            .collect()
    }

    fn pending_list(vm: &Vm) -> Value {
        Value::native_list_constructor(
            vec![Value::new_native_string_bytes(b"unreached".as_slice())],
            vm.actual_native_invocation_dialect()
                .native_string_protocol()
                .unwrap(),
        )
    }

    #[test]
    fn export_invalid_pattern_stops_before_later_original_getter_and_keeps_prior_commits() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm =
                crate::native_fixture::core(crate::environment::profile_for_dialect(release));
            vm.add_exports(&[tcl_core_types::NameBytes::from("before")]);
            let later = pending_list(&vm);
            let result = invoke_export(
                &mut vm,
                &[
                    Value::new_native_string_bytes(b"-clear\0tail".as_slice()),
                    Value::new_native_string_bytes(b"kept\xff".as_slice()),
                    Value::new_native_string_bytes(b"::invalid\0tail".as_slice()),
                    later.clone(),
                ],
            );
            assert_eq!(result.code, Code::Error, "{release}");
            assert_eq!(
                result.result.resident_string_bytes().unwrap().as_ref(),
                b"invalid export pattern \"::invalid\": pattern can't specify a namespace"
            );
            assert!(later.resident_string_bytes().is_none(), "{release}");
            assert_eq!(exports(&vm), [b"kept\xff".to_vec()], "{release}");
            assert!(vm.execution_refusal.is_none(), "{release}");
        }
    }

    #[test]
    fn export_clear_control_is_exact_leading_and_uses_cstring_extent() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for (patterns, expected) in [
                (
                    vec![b"-clear\0tail".as_slice(), b"p", b"-clear"],
                    vec![b"p".as_slice(), b"-clear"],
                ),
                (
                    vec![b"-cl".as_slice(), b"p"],
                    vec![b"before".as_slice(), b"-cl", b"p"],
                ),
                (
                    vec![b"--".as_slice(), b"p"],
                    vec![b"before".as_slice(), b"--", b"p"],
                ),
                (
                    vec![b"-clear\xc0\x80tail".as_slice(), b"p"],
                    vec![b"before".as_slice(), b"-clear\xc0\x80tail", b"p"],
                ),
                (
                    vec![b"p".as_slice(), b"-clear\0tail"],
                    vec![b"before".as_slice(), b"p", b"-clear"],
                ),
            ] {
                let mut vm =
                    crate::native_fixture::core(crate::environment::profile_for_dialect(release));
                vm.add_exports(&[tcl_core_types::NameBytes::from("before")]);
                let values: Vec<_> = patterns
                    .into_iter()
                    .map(Value::new_native_string_bytes)
                    .collect();
                let result = invoke_export(&mut vm, &values);
                assert_eq!(result.code, Code::Ok, "{release}");
                assert_eq!(exports(&vm), expected, "{release}");
                assert!(vm.execution_refusal.is_none(), "{release}");
            }
        }
    }

    struct FailingFormatter(Cell<usize>);
    impl NativeIntegerFormatter for FailingFormatter {
        fn build(&self) -> NativeIntegerFormatterBuild {
            NativeIntegerFormatterBuild {
                version: [8, 4, 20, 0],
                sha256: [7; 32],
                long_bits: 64,
            }
        }
        fn format(
            &self,
            _: NativeIntegerKind,
            _: i64,
        ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
            self.0.set(self.0.get() + 1);
            Err(NativeIntegerFormatterUnavailable::Operation)
        }
    }
    struct FormatterHost {
        actual: Rc<dyn tcl_platform::Host>,
        formatter: Rc<FailingFormatter>,
    }
    impl tcl_platform::Host for FormatterHost {
        fn capabilities(&self) -> tcl_platform::Capabilities {
            self.actual.capabilities()
        }
        fn clock(&self) -> &dyn tcl_platform::Clock {
            self.actual.clock()
        }
        fn stdio(&self) -> &dyn tcl_platform::StdIo {
            self.actual.stdio()
        }
        fn env(&self) -> &dyn tcl_platform::Env {
            self.actual.env()
        }
        fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
            Some(&*self.formatter)
        }
    }

    #[test]
    fn export_commits_each_original_pattern_before_a_later_getter_refusal() {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
        let formatter = Rc::new(FailingFormatter(Cell::new(0)));
        vm.set_host(Rc::new(FormatterHost {
            actual: vm.host_rc(),
            formatter: Rc::clone(&formatter),
        }));
        vm.add_exports(&[tcl_core_types::NameBytes::from("before")]);
        let failing = Value::from_native_scalar_cache(
            tcl_syntax::scalar_getter::NativeScalarCache::Number(tcl_syntax::number::Number::Int(
                i64::MIN,
            )),
            None,
            vm.actual_native_invocation_dialect(),
        )
        .unwrap();
        let later = pending_list(&vm);
        let result = invoke_export(
            &mut vm,
            &[
                Value::string("-clear"),
                Value::string("committed"),
                failing.clone(),
                later.clone(),
            ],
        );
        assert_eq!(result.code, Code::Error);
        assert_eq!(formatter.0.get(), 1);
        assert_eq!(exports(&vm), [b"committed".to_vec()]);
        assert!(failing.resident_string_bytes().is_none());
        assert!(later.resident_string_bytes().is_none());
        let first = vm.execution_refusal.as_ref().unwrap();
        assert!(!matches!(
            first,
            tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_)
        ));
    }

    #[test]
    fn jim_export_noop_does_not_reach_original_operand_getters() {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
        let original = pending_list(&vm);
        let result = invoke_export(&mut vm, &[original.clone(), Value::string("-clear")]);
        assert_eq!(result.code, Code::Ok);
        assert!(original.resident_string_bytes().is_none());
        assert!(result.result.resident_string_bytes().unwrap().is_empty());
        assert!(vm.execution_refusal.is_none());
    }
}

#[cfg(test)]
mod original_ensemble_tests {
    use super::*;

    #[test]
    fn configuration_namespace_results_keep_exact_selected_bytes_and_primary() {
        for release in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm =
                crate::native_fixture::core(crate::environment::profile_for_dialect(release));
            for (name, command) in [
                (b"::raw\xff".as_slice(), "::E1"),
                (b"::raw\xfe".as_slice(), "::E2"),
            ] {
                let selected = vm.activate_namespace_operand(name).unwrap();
                vm.push_ns_token(&name[2..], selected);
                let created = ns_ensemble_create(
                    &mut vm,
                    &[Value::string("-command"), Value::string(command)],
                );
                assert_eq!(created.code, Code::Ok, "{release}");
                vm.pop_ns();
                assert_ne!(vm.current_ns_id(), selected);
                let query = ns_ensemble_configure(
                    &mut vm,
                    &[Value::string(command), Value::string("-namespace")],
                );
                assert_eq!(query.code, Code::Ok, "{release}");
                let snapshot = ns_ensemble_configure(&mut vm, &[Value::string(command)]);
                assert_eq!(snapshot.code, Code::Ok, "{release}");
                let protocol = vm
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap();
                let entries = snapshot
                    .result
                    .native_object_list_elements(protocol)
                    .unwrap();
                let configured = entries
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .find(|pair| pair[0].resident_string_bytes().unwrap().as_ref() == b"-namespace")
                    .unwrap()[1]
                    .clone();
                for result in [query.result, configured] {
                    assert_eq!(result.resident_string_bytes().unwrap().as_ref(), name);
                    assert_eq!(
                        result.native_namespace_name_cache().is_some(),
                        matches!(release, "tcl9.0" | "tcl9.1"),
                    );
                    if let Some(cache) = result.native_namespace_name_cache() {
                        assert!(
                            cache
                                .namespace()
                                .unwrap()
                                .same_token(&vm.native_namespace_name_token(selected).unwrap())
                        );
                        assert!(cache.reference().is_none());
                    }
                    assert_eq!(vm.namespace_object_lookup(&result).unwrap(), Some(selected));
                }
            }
        }
    }

    #[test]
    fn configuration_namespace_result_retains_bytes_after_retirement_and_refuses_dead_owner() {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
        let selected = vm.activate_namespace_operand(b"::held").unwrap();
        vm.push_ns_token("held", selected);
        let created =
            ns_ensemble_create(&mut vm, &[Value::string("-command"), Value::string("::E")]);
        assert_eq!(created.code, Code::Ok);
        vm.pop_ns();
        let (_, crate::command::Command::Ensemble(token)) = vm
            .lookup_command_bytes_checked(vm.current_ns_id(), b"::E")
            .unwrap()
            .unwrap()
        else {
            panic!("actual ensemble token");
        };
        let def = token.config();
        let live = ensemble_option_value(
            &mut vm,
            &def,
            tcl_cmd_core::ensemble::ConfigOption::Namespace,
            NativeEnsembleNamespaceResultStage::Option,
        )
        .unwrap();
        let cache = live.native_namespace_name_cache().unwrap();
        assert!(vm.delete_namespace_operand(b"::held").unwrap());
        assert_eq!(
            cache.namespace().unwrap().lifecycle(),
            tcl_syntax::native_namespace_name::NativeNamespaceLifecycle::Dead,
        );
        assert_eq!(live.resident_string_bytes().unwrap().as_ref(), b"::held");
        assert!(
            ensemble_option_value(
                &mut vm,
                &def,
                tcl_cmd_core::ensemble::ConfigOption::Namespace,
                NativeEnsembleNamespaceResultStage::Option,
            )
            .is_err()
        );
    }

    #[test]
    fn configuration_namespace_results_preserve_distinct_owners_with_equal_name_bytes() {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
        let mut results = Vec::new();
        let mut namespaces = Vec::new();
        for (segments, command) in [
            ([b"a:".as_slice(), b"b".as_slice()], "::E1"),
            ([b"a".as_slice(), b":b".as_slice()], "::E2"),
        ] {
            let path = tcl_core_types::ByteNamespacePath::from_segments(segments);
            let selected = vm.install_constructed_namespace_test_fixture(path);
            let name = tcl_runtime_api::Namespaces::name_bytes(&vm, selected);
            vm.push_ns_token(&name[2..], selected);
            let created = ns_ensemble_create(
                &mut vm,
                &[Value::string("-command"), Value::string(command)],
            );
            assert_eq!(created.code, Code::Ok);
            vm.pop_ns();
            let queried = ns_ensemble_configure(
                &mut vm,
                &[Value::string(command), Value::string("-namespace")],
            );
            assert_eq!(queried.code, Code::Ok);
            let result = queried.result;
            assert!(
                result
                    .native_namespace_name_cache()
                    .unwrap()
                    .namespace()
                    .unwrap()
                    .same_token(&vm.native_namespace_name_token(selected).unwrap())
            );
            assert_eq!(vm.namespace_object_lookup(&result).unwrap(), Some(selected));
            results.push(result);
            namespaces.push(selected);
        }
        assert_ne!(namespaces[0], namespaces[1]);
        assert_eq!(
            results[0].resident_string_bytes(),
            results[1].resident_string_bytes()
        );
        assert!(
            !results[0]
                .native_namespace_name_cache()
                .unwrap()
                .namespace()
                .unwrap()
                .same_token(
                    results[1]
                        .native_namespace_name_cache()
                        .unwrap()
                        .namespace()
                        .unwrap()
                )
        );
    }

    #[test]
    fn configuration_namespace_result_refuses_component_with_no_native_c_spelling() {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
        let actual = vm.activate_namespace_operand(b"::a::b").unwrap();
        let impossible = tcl_core_types::ByteNamespacePath::from_segments([b"a::b".as_slice()]);
        let constructed = vm.install_constructed_namespace_test_fixture(impossible);
        assert_ne!(actual, constructed);
        assert_eq!(
            tcl_runtime_api::Namespaces::name_bytes(&vm, actual),
            tcl_runtime_api::Namespaces::name_bytes(&vm, constructed),
        );
        let result = vm
            .original_ensemble_namespace_result(
                actual,
                NativeEnsembleNamespaceResultStage::Snapshot,
            )
            .unwrap();
        assert!(
            result
                .native_namespace_name_cache()
                .unwrap()
                .namespace()
                .unwrap()
                .same_token(&vm.native_namespace_name_token(actual).unwrap())
        );
        assert!(
            vm.original_ensemble_namespace_result(
                constructed,
                NativeEnsembleNamespaceResultStage::Snapshot,
            )
            .is_err()
        );
    }

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
            let original = ensemble_option_value(
                &mut vm,
                &query,
                tcl_cmd_core::ensemble::ConfigOption::Map,
                NativeEnsembleNamespaceResultStage::Snapshot,
            )
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

#[cfg(test)]
mod native_map_prefix_tests;

#[cfg(test)]
mod original_jim_import_tests {
    use super::*;
    use crate::interp::native_operation_currency::NativeOperationScope;
    use tcl_runtime_api::{AliasPrefixLookup, Aliases, NsId, ROOT_NS};
    use tcl_syntax::value::ValueOps;

    fn native() -> Vm {
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jim"))
    }
    fn prefix(vm: &mut Vm, name: &Value, context: NsId) -> Value {
        let AliasPrefixLookup::Prefix(original) =
            Aliases::alias_prefix_original_value_at(vm, context, name).unwrap()
        else {
            panic!("actual stored original alias List");
        };
        original
    }
    fn define(vm: &mut Vm, name: &str, members: &[Value]) -> Value {
        let name = Value::string(name);
        let mut argv = vec![name.clone()];
        argv.extend_from_slice(members);
        assert_eq!(vm.invoke_command("alias", &argv).code, Code::Ok);
        prefix(vm, &name, ROOT_NS)
    }
    fn destination_frame(vm: &mut Vm) -> NsId {
        let token = vm.activate_namespace_operand(b"dest").unwrap();
        vm.push_ns_eval_token_frame(
            token,
            vec![
                Value::string("namespace"),
                Value::string("eval"),
                Value::string("dest"),
                Value::string(""),
            ],
        );
        token
    }
    fn slot(vm: &Vm) -> tcl_core_types::CommandSlot<tcl_core_types::NameBytes> {
        let selected = vm
            .name_policy_protocol()
            .unwrap()
            .recipe()
            .alias_publication_slot(
                tcl_syntax::naming::NativeNameContext::root(),
                b"dest::branch",
            )
            .unwrap();
        assert!(selected.namespace.is_root());
        tcl_core_types::CommandSlot {
            namespace: ROOT_NS,
            simple: selected.simple,
        }
    }

    #[test]
    fn original_vm_jim_alias_query_keeps_explicit_root_context_after_destination_cache() {
        // naming.alias.jim-original-core-prefix-object-storage

        // Authentic root/frame owners, stored List identity and cache context;
        // no original Enum header or C query/refcount equivalence is inferred.
        let mut vm = native();
        let root = define(&mut vm, "src::branch", &[Value::string("list")]);
        let collision = define(
            &mut vm,
            "dest::src::branch",
            &[Value::string("dest::branch")],
        );
        let destination = destination_frame(&mut vm);
        let original = Value::string("src::branch");
        let current = prefix(&mut vm, &original, destination);
        assert_eq!(
            current.native_object_identity(),
            collision.native_object_identity()
        );
        let queried = prefix(&mut vm, &original, ROOT_NS);
        assert_eq!(
            queried.native_object_identity(),
            root.native_object_identity()
        );
        assert_ne!(
            queried.native_object_identity(),
            collision.native_object_identity()
        );
        assert_eq!(tcl_runtime_api::Namespaces::current(&vm), destination);
        let imported = ns_import(&mut vm, &[Value::string("::src::branch")]);
        assert_eq!(imported.code, Code::Ok);
        assert!(
            vm.command_at_exact_slot_checked(&slot(&vm), false)
                .unwrap()
                .is_some()
        );
        assert!(vm.execution_refusal.is_none());
        vm.pop_ns();
        vm.pop_call_frame();

        let mut refused_vm = native();
        let refused_destination = destination_frame(&mut refused_vm);
        let original = Value::string("src::branch");
        assert!(
            Aliases::alias_prefix_original_value_at(&mut refused_vm, NsId(u32::MAX), &original)
                .is_err()
        );
        assert_eq!(
            tcl_runtime_api::Namespaces::current(&refused_vm),
            refused_destination
        );
        let first = refused_vm.execution_refusal.clone().unwrap();
        let later = ns_import(&mut refused_vm, &[Value::string("::src::branch")]);
        assert_eq!(later.code, Code::Error);
        assert_eq!(refused_vm.execution_refusal.as_ref(), Some(&first));
    }

    #[test]
    fn original_vm_jim_import_uses_cached_prefix_without_reading_original_members() {
        // naming.alias.jim-original-core-prefix-object-storage

        let mut vm = native();
        let target = Value::string("list");
        let member = Value::int(5);
        let stored = define(&mut vm, "src::branch", &[target.clone(), member.clone()]);
        assert_eq!(vm.native_string_bytes(&stored).unwrap().as_ref(), b"list 5");
        member.invalidate_native_string_for_test();
        assert!(member.resident_string_bytes().is_none());
        assert_eq!(stored.native_object_type_name(), "list");
        let refs = member.native_object_reference_count();
        let destination = destination_frame(&mut vm);
        let result = ns_import(&mut vm, &[Value::string("::src::branch")]);
        assert_eq!(result.code, Code::Ok);
        assert!(member.resident_string_bytes().is_none());
        assert_eq!(member.native_object_reference_count(), refs);
        assert_eq!(stored.native_object_type_name(), "list");
        let queried = prefix(&mut vm, &Value::string("src::branch"), ROOT_NS);
        assert_eq!(
            queried.native_object_identity(),
            stored.native_object_identity()
        );
        let members = vm.list_elements(&queried).unwrap();
        assert_eq!(
            members[0].native_object_identity(),
            target.native_object_identity()
        );
        assert_eq!(
            members[1].native_object_identity(),
            member.native_object_identity()
        );
        assert_eq!(tcl_runtime_api::Namespaces::current(&vm), destination);
        assert!(vm.execution_refusal.is_none());
        vm.pop_ns();
        vm.pop_call_frame();
    }

    #[test]
    fn original_vm_jim_import_compares_whole_prefix_before_following_alias_chains() {
        // naming.alias.jim-original-core-prefix-object-storage

        for case in 0..3 {
            let mut vm = native();
            let source = match case {
                0 => define(
                    &mut vm,
                    "src::branch",
                    &[Value::string("dest::branch"), Value::string("EXTRA")],
                ),
                1 => define(&mut vm, "src::branch", &[Value::string("dest::branch")]),
                2 => {
                    let _middle = define(&mut vm, "chain", &[Value::string("dest::branch")]);
                    define(&mut vm, "src::branch", &[Value::string("chain")])
                }
                _ => unreachable!(),
            };
            destination_frame(&mut vm);
            let destination_slot = slot(&vm);
            let result = ns_import(&mut vm, &[Value::string("::src::branch")]);
            assert_eq!(result.code, if case == 0 { Code::Ok } else { Code::Error });
            assert_eq!(
                vm.command_at_exact_slot_checked(&destination_slot, false)
                    .unwrap()
                    .is_some(),
                case == 0
            );
            if case != 0 {
                assert_eq!(
                    vm.native_string_bytes(&result.result).unwrap().as_ref(),
                    b"import pattern \"::src::branch\" would create a loop"
                );
            }
            assert!(source.resident_string_bytes().is_some());
            assert!(vm.execution_refusal.is_none());
            vm.pop_ns();
            vm.pop_call_frame();
        }
    }

    #[test]
    fn original_vm_jim_import_keeps_first_host_and_restored_context_terminal() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Real entered scope and actual context setters; not injected object
        // getter results or an original Native chronology comparison.
        for restore in [false, true] {
            let mut vm = native();
            let member = Value::int(5);
            let stored = define(
                &mut vm,
                "src::branch",
                &[Value::string("list"), member.clone()],
            );
            destination_frame(&mut vm);
            let destination_slot = slot(&vm);
            let scope = NativeOperationScope::enter(&mut vm).unwrap();
            let current = vm.runtime_context().clone();
            if restore {
                let mut changed = current.clone();
                changed.packages = vec![("original-vm-import".to_owned(), "1.0".to_owned())];
                vm.pin_context(&changed).unwrap();
                vm.pin_context(&current).unwrap();
            } else {
                let _ = vm.refuse_host_command("original VM import first cause".to_owned());
            }
            let first = scope.check(&mut vm).unwrap_err();
            let retained = vm.execution_refusal.clone().unwrap();
            let refs = member.native_object_reference_count();
            let result = ns_import(&mut vm, &[Value::string("::src::branch")]);
            assert_eq!(result.code, Code::Error);
            assert_eq!(vm.execution_refusal, Some(retained.clone()));
            assert_eq!(first.native_execution_refusal(), Some(&retained));
            assert!(
                vm.command_at_exact_slot_checked(&destination_slot, false)
                    .unwrap()
                    .is_none()
            );
            assert!(stored.resident_string_bytes().is_none());
            assert!(member.resident_string_bytes().is_none());
            assert_eq!(member.native_object_reference_count(), refs);
            assert_eq!(vm.runtime_context(), &current);
            drop(scope);
            vm.pop_ns();
            vm.pop_call_frame();
        }
    }
}

#[cfg(test)]
mod original_jim_namespace_text_result_tests {
    use super::*;

    #[test]
    fn qualifiers_selects_the_actual_shared_empty_without_changing_root_or_tail_identity() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let context = vm.native_jim_object_context().unwrap();
        let empty = context.empty_object().clone();
        for bytes in [b"absent".as_slice(), b":", b""] {
            let original = Value::from_native_string_bytes(bytes);
            let completion = ns_text_op(&mut vm, std::slice::from_ref(&original), false);
            assert_eq!(completion.code, Code::Ok);
            assert!(completion.result.is_same_object(&empty));
            assert!(!completion.result.is_same_object(&original));
        }
        let rooted = Value::from_native_string_bytes(b"::".as_slice());
        let completion = ns_text_op(&mut vm, &[rooted], false);
        assert_eq!(completion.code, Code::Ok);
        assert!(!completion.result.is_same_object(&empty));
        let tail = Value::from_native_string_bytes(b"absent".as_slice());
        let completion = ns_text_op(&mut vm, std::slice::from_ref(&tail), true);
        assert_eq!(completion.code, Code::Ok);
        assert!(completion.result.is_same_object(&tail));
        assert!(vm.refused_completion().is_none());
    }
}

#[cfg(test)]
mod original_jim_parent_current_holder_tests {
    use super::*;

    #[test]
    fn parent_reaches_the_original_current_holder_only_for_its_no_argument_form() {
        for explicit in [false, true] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect("jimtcl"),
            );
            let original = Value::int(42);
            let context =
                crate::interp::OriginalJimNamespaceContext::for_lambda(&mut vm, &original).unwrap();
            vm.push_call_frame(Some("parent_current_holder_control".into()), Vec::new());
            vm.install_jim_procedure_namespace_owner(None, original.clone());
            vm.push_procedure_namespace_context(crate::interp::ProcedureNamespace::OriginalJim(
                context.clone(),
            ));
            assert!(original.resident_string_bytes().is_none());
            let mut arguments = vec![Value::string("parent")];
            if explicit {
                arguments.push(Value::string("absent"));
            }
            let completion = cmd_namespace(&mut vm, &arguments);
            assert_eq!(completion.code, Code::Ok);
            assert_eq!(
                completion.result.resident_string_bytes().unwrap().as_ref(),
                b"::"
            );
            assert_eq!(original.resident_string_bytes().is_some(), !explicit);
            assert_eq!(
                vm.current_namespace_execution_context()
                    .resolved_token()
                    .is_some(),
                !explicit
            );
            if !explicit {
                assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"42");
            }
            assert!(vm.execution_refusal.is_none());
            vm.pop_ns();
            vm.pop_call_frame();
        }
    }
}

#[cfg(test)]
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
mod original_jim_canonical_current_holder_tests {
    use super::*;

    const OBSERVED: &str =
        include_str!("../tests/data/native_jim_namespace_canonical/current-holder-20261015.tsv");

    fn row(kind: &str, phase: &str) -> Vec<&'static str> {
        OBSERVED
            .lines()
            .find_map(|line| {
                let fields: Vec<_> = line.split('|').collect();
                (fields[0] == kind && fields[1] == phase).then(|| fields[2..].to_vec())
            })
            .unwrap_or_else(|| panic!("missing original observation {kind}/{phase}"))
    }

    #[test]
    fn canonical_uses_the_original_holder_and_reached_namespace_getter() {
        assert_eq!(OBSERVED.lines().count(), 80);
        for (label, name, explicit) in [
            ("CURRENT", None, false),
            ("ABSOLUTE", Some("::target"), false),
            ("RELATIVE", Some("child"), false),
            ("EXPLICIT", Some("child"), true),
        ] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect("jimtcl"),
            );
            let original = Value::int(42);
            let prefix = Value::int(84);
            let context =
                crate::interp::OriginalJimNamespaceContext::for_lambda(&mut vm, &original).unwrap();
            vm.push_call_frame(Some("canonical_current_holder_control".into()), Vec::new());
            vm.install_jim_procedure_namespace_owner(None, original.clone());
            vm.push_procedure_namespace_context(crate::interp::ProcedureNamespace::OriginalJim(
                context,
            ));
            assert!(original.resident_string_bytes().is_none());
            let mut arguments = vec![Value::string("canonical")];
            if explicit {
                arguments.push(prefix.clone());
            }
            if let Some(name) = name {
                arguments.push(Value::string(name));
            }
            let completion = cmd_namespace(&mut vm, &arguments);
            let phase = format!("{label}_REUSE");
            assert_eq!(completion.code, Code::Ok, "{phase}");
            assert_eq!(
                original.resident_string_bytes().is_some(),
                row("CALLER_AFTER", &phase)[0] == "1",
                "{phase}"
            );
            assert_eq!(
                prefix.resident_string_bytes().is_some(),
                row("PREFIX_AFTER", &phase)[0] == "1",
                "{phase}"
            );
            assert_eq!(
                completion.result.is_same_object(&original),
                row("RESULT_IS_CALLER", &phase)[0] == "1",
                "{phase}"
            );
            assert_eq!(
                vm.current_namespace_execution_context()
                    .resolved_token()
                    .is_some(),
                label == "RELATIVE",
                "{phase}"
            );
            let expected = row("RESULT", &phase);
            assert_eq!(expected[0], "0");
            let expected: Vec<_> = expected[1]
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(
                vm.native_name_operand_bytes_checked(&completion.result)
                    .unwrap()
                    .as_ref(),
                expected,
                "{phase}"
            );
            assert!(vm.execution_refusal.is_none(), "{phase}");
            vm.pop_ns();
            vm.pop_call_frame();
        }
    }

    #[test]
    fn equal_explicit_prefix_does_not_issue_the_current_namespace_coordinate() {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jimtcl"));
        let original = Value::int(42);
        let prefix = Value::int(42);
        assert!(!original.is_same_object(&prefix));
        let context =
            crate::interp::OriginalJimNamespaceContext::for_lambda(&mut vm, &original).unwrap();
        vm.push_call_frame(Some("canonical_explicit_prefix_control".into()), Vec::new());
        vm.install_jim_procedure_namespace_owner(None, original.clone());
        vm.push_procedure_namespace_context(crate::interp::ProcedureNamespace::OriginalJim(
            context,
        ));
        let completion = cmd_namespace(
            &mut vm,
            &[
                Value::string("canonical"),
                prefix.clone(),
                Value::string("child"),
            ],
        );
        assert_eq!(completion.code, Code::Ok);
        assert!(prefix.resident_string_bytes().is_some());
        assert!(original.resident_string_bytes().is_none());
        assert!(
            vm.current_namespace_execution_context()
                .resolved_token()
                .is_none()
        );
        assert!(vm.execution_refusal.is_none());
        vm.pop_ns();
        vm.pop_call_frame();
    }
}

#[cfg(test)]
mod original_c_import_tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    fn actual(release: &str) -> Vm {
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect(release))
    }
    fn import(vm: &mut Vm, patterns: &[Value]) -> Completion<Value> {
        let mut arguments = vec![Value::string("import")];
        arguments.extend_from_slice(patterns);
        // Handler getter order ends before the public evaluator logs an Error argv.
        vm.invoke_command_value_at(
            vm.current_ns_id(),
            &Value::string("namespace"),
            &arguments,
            &[],
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        )
    }
    fn setup(vm: &mut Vm) {
        assert_eq!(vm.eval_source("namespace eval source {proc first {} {return SOURCE}; proc victim {} {return IMPORTED}; namespace export first victim}; namespace eval destination {}").unwrap().code, Code::Ok);
    }

    #[test]
    fn import_commits_each_original_pattern_before_reaching_the_next_getter() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual(release);
            setup(&mut vm);
            let destination = vm.activate_namespace_operand(b"::destination").unwrap();
            vm.push_ns_token("destination", destination);
            let later = Value::int(42);
            let completion = import(
                &mut vm,
                &[
                    Value::string("::source::first"),
                    Value::string("unqualified"),
                    later.clone(),
                ],
            );
            assert_eq!(completion.code, Code::Error, "{release}");
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                b"no namespace specified in import pattern \"unqualified\""
            );
            assert!(later.resident_string_bytes().is_none());
            let imported = vm.invoke_host_original_object_vector(&Value::string("first"), &[]);
            assert_eq!(imported.code, Code::Ok);
            assert_eq!(imported.result.string_bytes().as_ref(), b"SOURCE");
            let from = tcl_runtime_api::Namespaces::find_command_bytes(
                &vm,
                destination,
                b"::source::first",
            )
            .unwrap();
            let to = tcl_runtime_api::Namespaces::find_command_bytes(&vm, destination, b"first")
                .unwrap();
            assert_eq!(
                tcl_runtime_api::Namespaces::command_origin(&vm, to),
                Some(from)
            );
            assert!(vm.refused_completion().is_none());
            vm.pop_ns();
        }
    }

    #[test]
    fn import_force_uses_exact_leading_c_extent_and_later_force_remains_a_pattern() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual(release);
            setup(&mut vm);
            assert_eq!(
                vm.eval_source("proc first {} {return OWN}").unwrap().code,
                Code::Ok
            );
            let flag = Value::new_native_string_bytes(b"-force\0ignored".as_slice());
            let pattern = Value::new_native_string_bytes(b"::source::first\0ignored".as_slice());
            let completion = import(&mut vm, &[flag.clone(), pattern.clone()]);
            assert_eq!(completion.code, Code::Ok, "{release}");
            assert_eq!(
                flag.resident_string_bytes().unwrap().as_ref(),
                b"-force\0ignored"
            );
            assert_eq!(
                pattern.resident_string_bytes().unwrap().as_ref(),
                b"::source::first\0ignored"
            );
            let imported = vm.invoke_host_original_object_vector(&Value::string("first"), &[]);
            assert_eq!(imported.result.string_bytes().as_ref(), b"SOURCE");
            for spelling in [b"-for".as_slice(), b"--", b"-force\xc0\x80tail"] {
                let later = Value::int(42);
                let completion = import(
                    &mut vm,
                    &[Value::new_native_string_bytes(spelling), later.clone()],
                );
                assert_eq!(completion.code, Code::Error);
                assert!(later.resident_string_bytes().is_none());
            }
            let later = Value::int(42);
            let completion = import(
                &mut vm,
                &[
                    Value::string("::source::victim"),
                    Value::string("-force"),
                    later.clone(),
                ],
            );
            assert_eq!(completion.code, Code::Error);
            assert!(
                vm.imported_command_tails()
                    .iter()
                    .any(|tail| tail.as_bytes() == b"victim")
            );
            assert!(later.resident_string_bytes().is_none());
            let query = import(&mut vm, &[]);
            assert_eq!(query.code, Code::Ok);
            if release == "tcl8.4" {
                assert_eq!(query.result.resident_string_bytes().unwrap().as_ref(), b"");
            } else {
                assert!(
                    query
                        .result
                        .as_list()
                        .unwrap()
                        .iter()
                        .any(|word| word.string_bytes().as_ref() == b"first")
                );
            }
        }
    }

    #[test]
    fn import_keeps_opaque_original_pattern_bytes_and_actual_source_token() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual(release);
            setup(&mut vm);
            let declared = vm.invoke_host_original_object_vector(
                &Value::string("proc"),
                &[
                    Value::new_native_string_bytes(b"::source::p\xff".as_slice()),
                    Value::empty(),
                    Value::string("return OPAQUE"),
                ],
            );
            assert_eq!(declared.code, Code::Ok, "{release}");
            let source = vm.activate_namespace_operand(b"::source").unwrap();
            vm.push_ns_token("source", source);
            let exported = vm.invoke_host_original_object_vector(
                &Value::string("namespace"),
                &[
                    Value::string("export"),
                    Value::new_native_string_bytes(b"p\xff".as_slice()),
                ],
            );
            assert_eq!(exported.code, Code::Ok);
            vm.pop_ns();
            let pattern = Value::new_native_string_bytes(b"::source::p\xff\0ignored".as_slice());
            assert_eq!(
                import(&mut vm, std::slice::from_ref(&pattern)).code,
                Code::Ok
            );
            assert_eq!(
                pattern.resident_string_bytes().unwrap().as_ref(),
                b"::source::p\xff\0ignored"
            );
            let called = vm.invoke_host_original_object_vector(
                &Value::new_native_string_bytes(b"p\xff".as_slice()),
                &[],
            );
            assert_eq!(called.code, Code::Ok);
            assert_eq!(called.result.string_bytes().as_ref(), b"OPAQUE");
            let original =
                tcl_runtime_api::Namespaces::find_command_bytes(&vm, source, b"::source::p\xff")
                    .unwrap();
            let imported = tcl_runtime_api::Namespaces::find_command_bytes(
                &vm,
                tcl_runtime_api::ROOT_NS,
                b"p\xff",
            )
            .unwrap();
            assert_eq!(
                tcl_runtime_api::Namespaces::command_origin(&vm, imported),
                Some(original)
            );
        }
    }

    #[test]
    fn import_preload_observes_actual_release_shadow_selection_before_validation() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual(release);
            let completion = vm.eval_source("proc auto_import {pattern} {error GLOBAL}; namespace eval destination {proc auto_import {pattern} {error LOCAL}; namespace import ::missing::*}").unwrap();
            assert_eq!(completion.code, Code::Error);
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                if release == "tcl8.4" {
                    b"GLOBAL".as_slice()
                } else {
                    b"LOCAL".as_slice()
                },
                "{release}"
            );
        }
    }

    #[test]
    fn import_preload_c84_direct_call_bypasses_ordinary_execution_trace() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual(release);
            let completion = vm.eval_source("set entered 0; proc count args {incr ::entered}; proc auto_import pattern {return}; trace add execution auto_import enter count; catch {namespace import ::missing::*}; set entered").unwrap();
            assert_eq!(completion.code, Code::Ok);
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                if release == "tcl8.4" {
                    b"0".as_slice()
                } else {
                    b"1".as_slice()
                },
                "{release}"
            );
        }
    }

    struct OriginalPreload {
        original: Value,
        calls: Rc<RefCell<Vec<Vec<u8>>>>,
        c84: bool,
        nested: bool,
    }
    impl crate::command::NativeCommand for OriginalPreload {
        fn invoke(&self, vm: &mut Vm, arguments: &[Value]) -> Completion<Value> {
            assert_eq!(arguments.len(), 1);
            let vector = vm
                .native_invocation
                .arguments
                .as_ref()
                .unwrap()
                .lifetime_view();
            let words = vector.elements().unwrap();
            assert_eq!(words.len(), 2);
            assert_eq!(
                words[0].resident_string_bytes().unwrap().as_ref(),
                b"auto_import"
            );
            assert!(words[1].is_same_object(&arguments[0]));
            assert!(!words[1].is_same_object(&self.original));
            assert_eq!(
                words[1].native_object_snapshot().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::None
            );
            assert_eq!(words[1].native_object_reference_count(), 1);
            if self.c84 {
                assert_eq!(words[0].native_object_reference_count(), 1);
            }
            let identities: Vec<_> = words.iter().map(Value::native_object_identity).collect();
            self.calls
                .borrow_mut()
                .push(arguments[0].resident_string_bytes().unwrap().to_vec());
            if self.nested {
                let nested = vm.invoke_host_original_object_vector(
                    &Value::string("set"),
                    &[Value::string("nestedImport"), Value::string("yes")],
                );
                assert_eq!(nested.code, Code::Ok);
                let unknown = vm
                    .invoke_host_original_object_vector(&Value::string("missingImportProbe"), &[]);
                assert_eq!(unknown.code, Code::Ok);
                assert_eq!(unknown.result.string_bytes().as_ref(), b"UNKNOWN_OK");
                let restored: Vec<_> = vm
                    .native_invocation
                    .arguments
                    .as_ref()
                    .unwrap()
                    .elements()
                    .unwrap()
                    .iter()
                    .map(Value::native_object_identity)
                    .collect();
                assert_eq!(restored, identities);
            }
            err("PRELOAD91")
        }
    }
    fn unknown_ok(_: &mut Vm, _: &[Value]) -> Completion<Value> {
        ok(Value::string("UNKNOWN_OK"))
    }

    #[test]
    fn import_preload_uses_fresh_c_extent_objects_and_restores_nested_original_vectors() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual(release);
            let original = Value::new_native_string_bytes(b"::missing::item\0ignored".as_slice());
            let calls = Rc::new(RefCell::new(Vec::new()));
            vm.register("unknown", unknown_ok);
            vm.register_native_command(
                "auto_import",
                Rc::new(OriginalPreload {
                    original: original.clone(),
                    calls: Rc::clone(&calls),
                    c84: release == "tcl8.4",
                    nested: true,
                }),
            );
            let later = Value::int(42);
            let completed = import(&mut vm, &[original.clone(), later.clone()]);
            assert_eq!(completed.code, Code::Error, "{release}");
            assert_eq!(completed.result.string_bytes().as_ref(), b"PRELOAD91");
            assert_eq!(&*calls.borrow(), &[b"::missing::item".to_vec()]);
            assert_eq!(
                original.resident_string_bytes().unwrap().as_ref(),
                b"::missing::item\0ignored"
            );
            assert!(later.resident_string_bytes().is_none());
            assert!(vm.native_invocation.arguments.is_none());
            assert!(vm.refused_completion().is_none());
        }
    }

    #[test]
    fn import_preload_can_publish_source_before_validation_and_absence_never_enters_unknown() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual(release);
            assert_eq!(vm.eval_source("proc unknown args {error WRONG_UNKNOWN}; proc auto_import pattern {namespace eval generated {proc member {} {return GENERATED}; namespace export member}}").unwrap().code, Code::Ok);
            let imported = import(&mut vm, &[Value::string("::generated::member")]);
            assert_eq!(imported.code, Code::Ok, "{release}");
            assert_eq!(
                vm.invoke_host_original_object_vector(&Value::string("member"), &[])
                    .result
                    .string_bytes()
                    .as_ref(),
                b"GENERATED"
            );
            assert_eq!(
                vm.eval_source("rename auto_import {}").unwrap().code,
                Code::Ok
            );
            let missing = import(&mut vm, &[Value::string("::absent::member")]);
            assert_eq!(missing.code, Code::Error);
            assert!(
                missing
                    .result
                    .string_bytes()
                    .windows(b"unknown namespace".len())
                    .any(|word| word == b"unknown namespace")
            );
        }
    }

    fn refuse_import(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        vm.refuse_host_command("original import publication first cause".into())
    }
    #[test]
    fn import_publication_callback_refusal_stops_before_import_stamp_and_later_getter() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual(release);
            setup(&mut vm);
            vm.register("refuseImport", refuse_import);
            assert_eq!(
                vm.eval_source(
                    "proc victim {} {return OWN}; trace add command victim delete refuseImport"
                )
                .unwrap()
                .code,
                Code::Ok
            );
            let later = Value::int(42);
            let completion = import(
                &mut vm,
                &[
                    Value::string("-force"),
                    Value::string("::source::victim"),
                    later.clone(),
                ],
            );
            assert_eq!(completion.code, Code::Error);
            assert!(vm.refused_completion().is_some());
            assert!(later.resident_string_bytes().is_none());
            assert!(
                !vm.imported_command_tails()
                    .iter()
                    .any(|name| name.as_bytes() == b"victim")
            );
            let first = vm.take_host_boundary_refusal().unwrap();
            assert!(format!("{first:?}").contains("original import publication first cause"));
        }
    }
}

#[cfg(test)]
mod original_c_namespace_error_log_tests {
    use super::*;

    #[test]
    fn public_namespace_error_logging_reaches_later_original_words_after_handler_refusal() {
        // Tcl_EvalObjv, tclBasic.c:3420 (8.4.20), generates the complete argv
        // only after TclEvalObjvInternal reports Error; the handler has returned.
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for selector in ["export", "import"] {
                let mut vm =
                    crate::native_fixture::core(crate::environment::profile_for_dialect(release));
                let later = Value::int(42);
                assert!(later.resident_string_bytes().is_none());
                let invalid = if selector == "export" {
                    "::invalid"
                } else {
                    "unqualified"
                };
                let arguments = [
                    Value::string(selector),
                    Value::string(invalid),
                    later.clone(),
                ];
                let result =
                    vm.invoke_host_original_object_vector(&Value::string("namespace"), &arguments);
                assert_eq!(result.code, Code::Error, "{release}/{selector}");
                assert_eq!(later.resident_string_bytes().unwrap().as_ref(), b"42");
                let info = vm.error_info_value().unwrap();
                assert!(info.windows(2).any(|part| part == b"42"));
                assert!(vm.execution_refusal.is_none());
            }
        }
    }
}
