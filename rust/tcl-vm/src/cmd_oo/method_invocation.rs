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

//! Invoking a method: running each step of the chain, intrinsic, property and forwarded methods, and the object built-ins `variable`, `varname` and `eval`.

use super::configurable_properties::configure_method;
use super::construction::{evaluate_class_definition, factory};
use super::declared_variables::{declared_variable_error, frame_private_storage_name};
use super::definition::definition_wrong_args;
use super::destruction::oo_destroy;
use super::method_resolution::{invocation_method_chain, is_exported_builtin, unknown_method};
use super::{
    DeclaredVariable, OoFrame, Step, StepRole, method_preparation, native_method_cache,
    native_properties, object_message, original_name_operands, variable_mapping,
};
use crate::command::ProcDef;
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_registry::native_tcloo_method_definition::NativeTclooMethodReportRole;
use tcl_runtime_api::{Completion, OoId};

/// Invoke `method` on object `obj_key`. `external` is a public `$obj m` call
/// (export-enforced); `false` is a `my m` internal call.
pub(super) fn oo_invoke(
    vm: &mut Vm,
    obj_key: OoId,
    method: &str,
    args: &[Value],
    external: bool,
    invoked: &Value,
) -> Completion<Value> {
    oo_invoke_value(
        vm,
        obj_key,
        method,
        &Value::string(method),
        args,
        external,
        invoked,
    )
}

pub(super) fn original_call_words<'a>(words: impl IntoIterator<Item = &'a Value>) -> Vec<Value> {
    words
        .into_iter()
        .map(|word| word.native_lifetime_lease().into_value())
        .collect()
}

pub(super) fn oo_invoke_value(
    vm: &mut Vm,
    obj_key: OoId,
    method: &str,
    original_method: &Value,
    args: &[Value],
    external: bool,
    invoked: &Value,
) -> Completion<Value> {
    oo_invoke_value_with_head(
        vm,
        obj_key,
        method,
        original_method,
        args,
        external,
        (invoked, None),
    )
}

pub(super) fn oo_invoke_value_with_head(
    vm: &mut Vm,
    obj_key: OoId,
    method: &str,
    original_method: &Value,
    args: &[Value],
    external: bool,
    invocation_heads: (&Value, Option<&Value>),
) -> Completion<Value> {
    let original_scope =
        match crate::interp::native_operation_currency::NativeOperationScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let (invoked, private_head) = invocation_heads;
    if !vm.oo.objects.contains_key(&obj_key) {
        return err(object_message(
            vm,
            obj_key,
            b"invalid command name \"",
            b"\"",
        ));
    }
    // Object built-in methods (available internally, or when exported).
    if matches!(method, "variable" | "varname" | "eval")
        && (!external || is_exported_builtin(vm, obj_key, method))
    {
        return builtin_method(
            vm,
            obj_key,
            method,
            args,
            invoked,
            original_method,
            external,
        );
    }

    let chain = match invocation_method_chain(vm, obj_key, method, original_method, external) {
        Ok(Some(chain)) => with_filters(vm, obj_key, chain),
        Ok(None) => {
            let mut uargs = Vec::with_capacity(args.len() + 1);
            uargs.push(original_method.native_lifetime_lease().into_value());
            uargs.extend_from_slice(args);
            let result = oo_invoke_unknown(vm, obj_key, Some(method), &uargs, external, invoked);
            if let Err(error) = original_scope.check(vm) {
                drop(result);
                return crate::command::completion_from_cmd_error(vm, error);
            }
            return result;
        }
        Err(completion) => return completion,
    };
    let usage = vec![invoked.clone(), original_method.clone()];
    let original_head = if let Some(head) = private_head {
        head.native_lifetime_lease().into_value()
    } else if external {
        invoked.native_lifetime_lease().into_value()
    } else {
        let Some(head) = vm.invoked_name_value() else {
            return vm.refuse_host_command("original TclOO invocation head is unavailable".into());
        };
        head.native_lifetime_lease().into_value()
    };
    let call_identity =
        original_call_words([&original_head, original_method].into_iter().chain(args));
    let result = run_step(
        vm,
        obj_key,
        &chain,
        0,
        method.to_string(),
        args,
        external,
        invoked.clone(),
        usage,
        call_identity,
    );
    drop(chain);
    if let Err(error) = original_scope.check(vm) {
        drop(result);
        return crate::command::completion_from_cmd_error(vm, error);
    }
    result
}

/// Prepend the object's active filters to `chain`, unless a filter is already
/// running on it.
fn with_filters(
    vm: &Vm,
    obj_key: OoId,
    chain: native_method_cache::ReachedMethodChain,
) -> native_method_cache::ReachedMethodChain {
    if filter_handling(vm, obj_key) {
        return chain;
    }
    let mut steps = vm.oo.active_filters(&vm.oo.object_precedence(obj_key));
    if steps.is_empty() {
        return chain;
    }
    steps.extend(chain.iter().cloned());
    steps.into()
}

/// Route a method-lookup miss through the object's `unknown` handlers (C's
/// `OO_UNKNOWN_METHOD` chain): every provider of `unknown` in precedence order,
/// behind the active filters. `uargs` already carries the missed name first;
/// `unknown` is found whatever its export state. With no method name at all
/// (`$obj` alone, C's `FORCE_UNKNOWN`), `missed` is `None` and filters do not
/// apply. With no user handler, `oo::object`'s built-in reports the miss.
pub(super) fn oo_invoke_unknown(
    vm: &mut Vm,
    obj_key: OoId,
    missed: Option<&str>,
    uargs: &[Value],
    external: bool,
    invoked: &Value,
) -> Completion<Value> {
    let precedence = vm.oo.object_precedence(obj_key);
    let mut steps = vm.oo.unknown_handlers(&precedence);
    if steps.is_empty() {
        if let Some(method) = missed {
            return unknown_method(vm, obj_key, method, external);
        }
        let mut usage = match vm.native_argument_usage_header(std::slice::from_ref(invoked)) {
            Ok(usage) => usage,
            Err(completion) => return completion,
        };
        usage.extend_from_slice(b" method ?arg ...?");
        return crate::command::native_wrong_args_bytes(vm, &usage);
    }
    if missed.is_some() && !filter_handling(vm, obj_key) {
        let mut filtered = vm.oo.active_filters(&precedence);
        filtered.append(&mut steps);
        steps = filtered;
    }
    let head = if external {
        invoked.native_lifetime_lease().into_value()
    } else {
        Value::string("my")
    };
    let call_identity = original_call_words(std::iter::once(&head).chain(uargs));
    let chain: native_method_cache::ReachedMethodChain = steps.into();
    run_step(
        vm,
        obj_key,
        &chain,
        0,
        "unknown".to_string(),
        uargs,
        external,
        invoked.clone(),
        vec![head],
        call_identity,
    )
}

/// Whether calls on `obj_key` currently bypass its filters (C's
/// `FILTER_HANDLING`): the object's innermost running step is a filter, or
/// belongs to a chain built while a filter ran — such a chain carries no
/// filter steps although the object has active filters. Constructor and
/// destructor chains never carry filters, so they do not count.
fn filter_handling(vm: &Vm, obj_key: OoId) -> bool {
    let Some(fr) = vm.oo.call_stack.iter().rev().find(|f| f.object == obj_key) else {
        return false;
    };
    if fr.chain.get(fr.index).is_some_and(|s| s.filter().is_some()) {
        return true;
    }
    if fr.chain.report_role() != NativeTclooMethodReportRole::Named || fr.method == "eval" {
        return false;
    }
    !fr.chain.iter().any(|s| s.filter().is_some())
        && !vm
            .oo
            .active_filters(&vm.oo.object_precedence(obj_key))
            .is_empty()
}

/// Execute step `index` of `chain` on `obj_key`, for the invoked `method`
/// (empty = constructor, `<destructor>` = destructor). Pushes an [`OoFrame`] so
/// `self`/`next` see the chain.
///
/// Native-stack safety net — see `interp::OO_DISPATCH_DEPTH_LIMIT`'s doc
/// comment. This is the single choke point every method-body
/// execution funnels through — `$obj method` (via [`oo_dispatch`] →
/// [`oo_invoke`]), `my method` (via `cmd_my` → [`oo_invoke`]), and
/// `next`/`nextto` (directly) — so guarding here covers every recursive
/// path, not just the `Command::Object` engine dispatch.
#[allow(clippy::too_many_arguments)] // A method activation genuinely carries this context.
pub(super) fn run_step(
    vm: &mut Vm,
    obj_key: OoId,
    chain: &native_method_cache::ReachedMethodChain,
    index: usize,
    method: String,
    args: &[Value],
    external: bool,
    invoked: Value,
    usage_prefix: Vec<Value>,
    call_identity: Vec<Value>,
) -> Completion<Value> {
    let cleanup = crate::interp::oo_cleanup::OriginalOoCleanup::enter(vm);
    if let Err(error) = cleanup.check(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let prior_depth = match cleanup.capture_dispatch_depth(vm) {
        Ok(depth) => depth,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    if let Err(c) = vm.enter_oo_dispatch() {
        return c;
    }
    let result = run_step_inner(
        vm,
        obj_key,
        chain,
        index,
        method,
        args,
        external,
        invoked,
        usage_prefix,
        call_identity,
    );
    if let Err(error) = cleanup.restore_dispatch_depth(vm, prior_depth) {
        crate::interp::oo_cleanup::retain_error(vm, &error);
    }
    cleanup.finish_completion(vm, result)
}

/// A method step after [`prepare_method_step`]: either it already finished
/// (an intrinsic, property or forward method, or a preparation failure), or it
/// is a procedure-bodied method ready to run with its instance-variable links.
enum MethodStep {
    Finished(Completion<Value>),
    Procedure(
        Box<crate::command::PreparedProcedureActivation>,
        Vec<(NameBytes, NameBytes)>,
    ),
}

#[allow(clippy::too_many_arguments)] // A method activation genuinely carries this context.
fn run_step_inner(
    vm: &mut Vm,
    obj_key: OoId,
    chain: &native_method_cache::ReachedMethodChain,
    index: usize,
    method: String,
    args: &[Value],
    external: bool,
    invoked: Value,
    usage_prefix: Vec<Value>,
    call_identity: Vec<Value>,
) -> Completion<Value> {
    // A procedure-bodied method recurses natively through this frame (see
    // `interp::OO_DISPATCH_DEPTH_LIMIT`), so all the work that finishes before
    // its body runs is kept in `prepare_method_step`'s frame, not this one.
    let (proc, link_vars) = match prepare_method_step(
        vm,
        obj_key,
        chain,
        index,
        &method,
        args,
        external,
        &invoked,
        &usage_prefix,
        call_identity,
    ) {
        MethodStep::Finished(completion) => return completion,
        MethodStep::Procedure(proc, link_vars) => (proc, link_vars),
    };
    let frame = OoFrame {
        context_birth: Rc::new(()),
        name_owner: Rc::clone(&vm.oo.objects[&obj_key].cached_name),
        activation: None,
        object: obj_key,
        chain: chain.clone(),
        index,
        method,
        external,
        invoked,
        usage_prefix,
    };
    vm.oo_run_method(proc, args, &link_vars, frame)
}

/// Everything [`run_step_inner`] does before a procedure-bodied method's body
/// runs: intrinsic, property and forward methods finish here; a procedure
/// method gets its compiled body and instance-variable links.
#[inline(never)]
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // A method activation genuinely carries this context and persistence seam.
fn prepare_method_step(
    vm: &mut Vm,
    obj_key: OoId,
    chain: &native_method_cache::ReachedMethodChain,
    index: usize,
    method: &str,
    args: &[Value],
    external: bool,
    invoked: &Value,
    usage_prefix: &[Value],
    call_identity: Vec<Value>,
) -> MethodStep {
    let step = chain[index].clone();
    let filter_name = step.filter().map(str::to_owned);
    let method = filter_name.as_deref().unwrap_or(method);
    // Only the originally entered record can supply this implementation.
    // An ownerless descriptive context cannot acquire a mutable table record.
    let Some(owner) = &step.method_owner else {
        return MethodStep::Finished(crate::command::completion_from_cmd_error(
            vm,
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original TclOO selected method record",
            )
            .into(),
        ));
    };
    let m = owner.borrow().clone();

    if let Some(intrinsic) = m.intrinsic {
        return MethodStep::Finished(run_intrinsic_method(
            vm,
            obj_key,
            intrinsic,
            args,
            invoked,
            usage_prefix,
        ));
    }
    if let Some(property) = &m.property {
        return MethodStep::Finished(run_property_method(
            vm,
            obj_key,
            chain,
            index,
            method,
            property,
            args,
            external,
            invoked,
            usage_prefix,
            &call_identity,
        ));
    }
    if let Some(prefix) = &m.forward {
        return MethodStep::Finished(run_forward_method(
            vm,
            obj_key,
            chain,
            index,
            method,
            prefix,
            args,
            external,
            invoked,
            usage_prefix,
        ));
    }

    let preparation =
        match method_preparation::OriginalMethodPreparation::enter(vm, obj_key, chain, index) {
            Ok(preparation) => preparation,
            Err(error) => {
                return MethodStep::Finished(crate::command::completion_from_cmd_error(vm, error));
            }
        };
    let obj_ns = vm.oo.objects.get(&obj_key).map_or_default(|o| o.ns.clone());
    // Instance variables to auto-link belong to the method's declaring
    // provider. An object-defined method sees the object's declarations; a
    // class-defined method sees that class's declarations.
    let mut decl: Vec<DeclaredVariable> = Vec::new();
    let mut private_decl: Vec<DeclaredVariable> = Vec::new();
    let selected_mapping = vm
        .name_policy_protocol()
        .is_some_and(variable_mapping::selected);
    let staging = |entry: &DeclaredVariable| {
        if selected_mapping {
            variable_mapping::observation(entry)
        } else {
            entry.clone()
        }
    };
    if step.is_object {
        if let Some(o) = vm.oo.objects.get(&step.provider) {
            decl.extend(o.variables.iter().map(&staging));
            private_decl.extend(o.private_variables.iter().map(&staging));
        }
    } else if let Some(c) = vm.oo.classes.get(&step.provider) {
        decl.extend(c.variables.iter().map(&staging));
        private_decl.extend(c.private_variables.iter().map(&staging));
    }
    // A class is an object too, so the declaring provider's creation id is in
    // `objects` either way; it names the private storage slot below.
    let provider_creation_id = vm
        .oo
        .objects
        .get(&step.provider)
        .map_or(0, |o| o.creation_id);
    // TclOO's automatic instance-variable resolver yields to a method formal of
    // the same name. Explicit `my variable x` still reaches the ordinary link
    // installer and reports the collision.
    let shadowed = |name: &NameBytes| m.parameters.values.iter().any(|param| param.name == *name);
    let ns_id = vm.definition_namespace_token(&obj_ns);
    let mut link_vars: Vec<(NameBytes, NameBytes)> = Vec::new();
    for variable in private_decl {
        if shadowed(&variable.name) || link_vars.iter().any(|(local, _)| *local == variable.name) {
            continue;
        }
        let storage = variable.private_storage(provider_creation_id);
        link_vars.push((variable.name, storage));
    }
    for variable in decl {
        if shadowed(&variable.name) || link_vars.iter().any(|(local, _)| *local == variable.name) {
            continue;
        }
        link_vars.push((variable.name.clone(), variable.name));
    }

    let body = m
        .compiled_body
        .as_ref()
        .filter(|(namespace, _)| namespace == &obj_ns)
        .map(|(_, body)| body.clone());
    let proc = ProcDef {
        formals: crate::command::FormalParameterCache::default(),
        native_resources: Rc::default(),
        name: tcl_syntax::naming::unrooted_member_key(&obj_ns, method),
        command_ns_id: ns_id,
        simple_name: tcl_core_types::NameBytes::from(vm.oo.method_name_bytes(method)),
        execution_namespace: crate::interp::ProcedureNamespace::Resolved {
            path: vm.namespace_path_for_token(ns_id),
            token: ns_id,
        },
        params: m.parameters.values.clone(),
        parameter_grammar: tcl_dialect::ParameterGrammar::Tcl,
        has_args: m.parameters.has_args,
        native_jim_namespace: None,
        native_parameters: None,
        native_header: tcl_dialect::NativeProcedureHeaderCompilation::Absent,
        statics: None,
        body,
        body_src: m.body_src.clone(),
        usage_name: Some(usage_prefix.to_vec()),
        call_identity: Some(call_identity),
        lambda_term: None,
    };
    // Refresh through the shared proc owner, then persist the replacement on
    // its defining facet.  Recompiling on every method call after a profile
    // switch is both wasteful and makes the class cache lie about its body.
    let reached = vm.ensure_proc_traced(crate::command::NativeProcedureCommand::new(proc));
    let proc = match preparation.settle(vm, reached) {
        Ok(proc) => proc,
        Err(error) => {
            return MethodStep::Finished(crate::command::completion_from_tcl_error(vm, error));
        }
    };
    let cache_is_current = m.compiled_body.as_ref().is_some_and(|(namespace, cached)| {
        namespace == &obj_ns
            && Rc::ptr_eq(&proc.body.asm, &cached.asm)
            && proc.body.command_epoch == cached.command_epoch
            && proc.body.profile_generation == cached.profile_generation
            && proc.body.compiler == cached.compiler
    });
    if !cache_is_current {
        let mut refreshed = m.clone();
        refreshed.compiled_body = Some((obj_ns, proc.body.clone()));
        let retired = {
            let mut current = owner.borrow_mut();
            if current.body_src.native_object_identity() == m.body_src.native_object_identity()
                && current.forward.is_none()
                && current.property.is_none()
            {
                std::mem::replace(&mut current.compiled_body, refreshed.compiled_body)
            } else {
                None
            }
        };
        drop(retired);
        if let Err(error) = preparation.check(vm) {
            return MethodStep::Finished(crate::command::completion_from_cmd_error(vm, error));
        }
    }
    if let Err(error) = preparation.finish(vm) {
        return MethodStep::Finished(crate::command::completion_from_cmd_error(vm, error));
    }
    MethodStep::Procedure(Box::new(proc), link_vars)
}

/// An intrinsic method step (`destroy`, `create`/`new`, `configure`, a
/// class constructor), out of line from [`prepare_method_step`].
#[inline(never)]
fn run_intrinsic_method(
    vm: &mut Vm,
    obj_key: OoId,
    intrinsic: tcl_runtime_api::native_oo::NativeOoIntrinsicMethod,
    args: &[Value],
    invoked: &Value,
    usage_prefix: &[Value],
) -> Completion<Value> {
    use tcl_runtime_api::native_oo::NativeOoIntrinsicMethod;
    match intrinsic {
        NativeOoIntrinsicMethod::Destroy => {
            if !args.is_empty() {
                let header = match vm.native_argument_usage_header(usage_prefix) {
                    Ok(header) => header,
                    Err(error) => return error,
                };
                return crate::command::native_wrong_args_bytes(vm, &header);
            }
            oo_destroy(vm, obj_key)
        }
        NativeOoIntrinsicMethod::Create | NativeOoIntrinsicMethod::New => {
            let Some(original_method) = usage_prefix.get(1) else {
                return vm.refuse_host_command(
                    "original factory method invocation is unavailable".into(),
                );
            };
            factory(
                vm,
                obj_key,
                invoked,
                original_method,
                intrinsic == NativeOoIntrinsicMethod::New,
                args,
            )
        }
        NativeOoIntrinsicMethod::Configure => configure_method(vm, obj_key, args),
        NativeOoIntrinsicMethod::ClassConstructor
        | NativeOoIntrinsicMethod::ConfigurableConstructor => {
            if args.len() > 1 {
                return definition_wrong_args(vm, "?definitionScript?");
            }
            args.first().map_or_else(
                || ok(Value::empty()),
                |body| evaluate_class_definition(vm, obj_key, body),
            )
        }
    }
}

/// A property accessor method step, out of line from [`prepare_method_step`].
#[allow(clippy::too_many_arguments)] // A method activation genuinely carries this context.
#[inline(never)]
fn run_property_method(
    vm: &mut Vm,
    obj_key: OoId,
    chain: &native_method_cache::ReachedMethodChain,
    index: usize,
    method: &str,
    property: &native_properties::NativePropertyAccessor,
    args: &[Value],
    external: bool,
    invoked: &Value,
    usage_prefix: &[Value],
    call_identity: &[Value],
) -> Completion<Value> {
    let mut active = crate::interp::oo_cleanup::OriginalOoActivation::enter(vm);
    let frame = OoFrame {
        context_birth: Rc::new(()),
        name_owner: Rc::clone(&vm.oo.objects[&obj_key].cached_name),
        activation: None,
        object: obj_key,
        chain: chain.clone(),
        index,
        method: method.to_owned(),
        external,
        invoked: invoked.clone(),
        usage_prefix: usage_prefix.to_vec(),
    };
    if let Err(error) = active.push_method(vm, frame) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let result = native_properties::invoke_default(vm, obj_key, property, args, call_identity);
    active.retire_method(vm);
    active.finish(vm, result)
}

// A forward: evaluate `prefix args…` with the OBJECT's namespace current,
// so the target resolves object-ns → global exactly as C TclOO does —
// never in the dispatch site's namespace (tclsh 8.6.16 / 9.0.4-pinned:
// `$obj fw` called from a namespace that has its own same-named proc
// still dispatches the global target).
#[allow(clippy::too_many_arguments)] // A method activation genuinely carries this context.
#[inline(never)]
fn run_forward_method(
    vm: &mut Vm,
    obj_key: OoId,
    chain: &native_method_cache::ReachedMethodChain,
    index: usize,
    method: &str,
    prefix: &[Value],
    args: &[Value],
    external: bool,
    invoked: &Value,
    usage_prefix: &[Value],
) -> Completion<Value> {
    let mut active = crate::interp::oo_cleanup::OriginalOoActivation::enter(vm);
    let mut words: Vec<Value> = prefix.to_vec();
    words.extend_from_slice(args);
    let ns = vm.oo.objects.get(&obj_key).map(|o| o.ns.clone());
    let frame = OoFrame {
        context_birth: Rc::new(()),
        name_owner: Rc::clone(&vm.oo.objects[&obj_key].cached_name),
        activation: None,
        object: obj_key,
        chain: chain.clone(),
        index,
        method: method.to_owned(),
        external,
        invoked: invoked.clone(),
        usage_prefix: usage_prefix.to_vec(),
    };
    if let Err(error) = active.push_method(vm, frame) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    vm.push_ns(ns.unwrap_or_default());
    if let Err(error) = active.capture_namespace(vm) {
        crate::interp::oo_cleanup::retain_error(vm, &error);
        active.retire_namespace(vm, false);
        active.retire_method(vm);
        let result = crate::command::completion_from_cmd_error(vm, error);
        return active.finish(vm, result);
    }
    let context = vm.current_ns_id();
    let res = vm.invoke_command_value_at(
        context,
        &words[0],
        &words[1..],
        &[],
        tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
    );
    active.retire_namespace(vm, false);
    active.retire_method(vm);
    active.finish(vm, res)
}

/// The object built-in methods `variable name…`, `varname name`, `eval script`.
pub(super) fn builtin_method(
    vm: &mut Vm,
    obj_key: OoId,
    method: &str,
    args: &[Value],
    invoked: &Value,
    original_method: &Value,
    external: bool,
) -> Completion<Value> {
    original_name_operands::run(vm, Some((false, obj_key)), |vm, original_names| {
        let ns = match vm.oo.objects.get(&obj_key) {
            Some(o) => o.ns.clone(),
            None => {
                return err(object_message(
                    vm,
                    obj_key,
                    b"invalid command name \"",
                    b"\"",
                ));
            }
        };
        match method {
            "variable" => builtin_variable(vm, obj_key, args),
            "varname" => builtin_varname(vm, obj_key, args, original_names),
            "eval" => {
                if args.is_empty() {
                    return crate::command::native_wrong_args(vm, "my eval arg ?arg ...?");
                }
                let script = if let [original] = args {
                    original.clone()
                } else {
                    match tcl_cmd_core::list::concat_selected(vm, args) {
                        Ok(script) => script,
                        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                    }
                };
                let namespace = vm.definition_namespace_token(&ns);
                let invocation =
                    original_call_words([invoked, original_method].into_iter().chain(args.iter()));
                let mut active = crate::interp::oo_cleanup::OriginalOoActivation::enter(vm);
                vm.push_ns_eval_token_frame(namespace, invocation);
                if let Err(error) = active.capture_call_and_namespace(vm) {
                    crate::interp::oo_cleanup::retain_error(vm, &error);
                    active.retire_call(vm, false);
                    active.retire_namespace(vm, false);
                    let result = crate::command::completion_from_cmd_error(vm, error);
                    return active.finish(vm, result);
                }
                let frame = OoFrame {
                    context_birth: Rc::new(()),
                    name_owner: Rc::clone(&vm.oo.objects[&obj_key].cached_name),
                    activation: Some(vm.native_oo_variable_activation()),
                    object: obj_key,
                    chain: native_method_cache::ReachedMethodChain::from(vec![Step {
                        method_owner: None,
                        provider: vm.oo.object_root.expect("native object eval provider"),
                        is_object: false,
                        role: StepRole::Method,
                    }]),
                    index: 0,
                    method: "eval".into(),
                    external,
                    invoked: invoked.clone(),
                    usage_prefix: original_call_words([invoked, original_method]),
                };
                if let Err(error) = active.push_method(vm, frame) {
                    crate::interp::oo_cleanup::retain_error(vm, &error);
                    active.retire_call(vm, false);
                    active.retire_namespace(vm, false);
                    let result = crate::command::completion_from_cmd_error(vm, error);
                    return active.finish(vm, result);
                }
                let res = match vm.eval_original_script_value(
                    &script,
                    tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
                    args.first()
                        .filter(|_| args.len() == 1)
                        .and_then(Value::source_location),
                ) {
                    Ok(c) => c,
                    Err(e) => crate::command::completion_from_tcl_error(vm, e),
                };
                active.retire_method(vm);
                active.retire_call(vm, false);
                active.retire_namespace(vm, false);
                active.finish(vm, res)
            }
            _ => unreachable!("builtin_method called with {method}"),
        }
    })
}

fn builtin_varname(
    vm: &mut Vm,
    obj_key: OoId,
    args: &[Value],
    original_names: &mut original_name_operands::OriginalOoNameScope,
) -> Completion<Value> {
    if args.len() != 1 {
        return crate::command::native_wrong_args(vm, "my varname varName");
    }
    if let Err(error) = original_names.select_declaring_provider(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let Some(namespace) = vm
        .oo
        .objects
        .get(&obj_key)
        .and_then(|object| object.namespace_token)
    else {
        return vm.refuse_host_command("actual TclOO variable namespace unavailable".into());
    };
    let bytes = match original_names.read(vm, &args[0]) {
        Ok(bytes) => bytes,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let storage = frame_private_storage_name(
        vm,
        &bytes,
        tcl_syntax::naming::NativeOoPrivateVariablePurpose::Varname,
    );
    match vm.original_c_oo_varname(
        &args[0],
        namespace,
        storage.as_ref().map_or(bytes.as_ref(), NameBytes::as_bytes),
    ) {
        Ok(result) => ok(result),
        Err(error) => error,
    }
}

pub(super) fn builtin_variable(vm: &mut Vm, obj_key: OoId, args: &[Value]) -> Completion<Value> {
    original_name_operands::run(vm, Some((false, obj_key)), |vm, original_names| {
        if let Err(error) = original_names.select_declaring_provider(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let Some(namespace) = vm
            .oo
            .objects
            .get(&obj_key)
            .and_then(|object| object.namespace_token)
        else {
            return vm.refuse_host_command("actual TclOO variable namespace unavailable".into());
        };
        let Some(policy) = vm.name_policy_protocol() else {
            return vm
                .refuse_host_command("actual TclOO variable name protocol unavailable".into());
        };
        for original in args {
            let bytes = match original_names.read(vm, original) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            let local = match tcl_syntax::naming::native_oo_explicit_variable_local_name(
                policy.recipe(),
                &bytes,
            ) {
                Ok(Ok(local)) => local.to_vec(),
                Ok(Err(error)) => return declared_variable_error(error),
                Err(error) => return vm.refuse_host_command(format!("{error:?}")),
            };
            let storage = frame_private_storage_name(
                vm,
                &bytes,
                tcl_syntax::naming::NativeOoPrivateVariablePurpose::ExplicitLink,
            );
            let mapped = storage
                .as_ref()
                .map(|storage| Value::new_native_string_bytes(storage.as_bytes()));
            if let Err(error) = vm.link_original_c_oo_variable(
                mapped.as_ref().unwrap_or(original),
                namespace,
                &local,
            ) {
                return error;
            }
        }
        ok(Value::empty())
    })
}
