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

//! `apply`: preparing a lambda declaration, building its procedure and resolving its namespace.

use super::error_completion::completion_from_cmd_error;
use super::native_procedure::NativeProcedureCommand;
use super::procedure_definition::parse_params_value;
use super::{Param, ProcDef, native_wrong_args};
use crate::interp::{Vm, err};
use crate::value::Value;
use std::rc::Rc;
use tcl_runtime_api::{Completion, NsId};

/// Monotonic counter minting unique temporary command names for `apply`.
fn fresh_apply_name() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    // Canonical *unrooted* key form — `register_command` takes keys verbatim.
    format!("tcl::apply::lambda{n}")
}

/// A manufactured lambda binding and its independent temporary registration lifetime.
pub(crate) struct OriginalLambdaDefinition {
    pub(crate) binding: NativeProcedureCommand,
    pub(crate) registration: crate::interp::CommandSidecarHandle,
    pub(crate) activation_preflight_complete: bool,
}

struct PreparedLambdaDeclaration {
    parts: crate::NativeListItems,
    pub(super) parameter_grammar: tcl_dialect::ParameterGrammar,
    pub(super) body: Value,
    pub(super) parameters: Vec<Param>,
    has_args: bool,
}

/// Read and validate the lambda declaration before it is published.
fn prepare_lambda_declaration(
    vm: &mut Vm,
    lambda: &Value,
) -> Result<PreparedLambdaDeclaration, Completion<Value>> {
    let Some(protocol) = vm.native_invocation_dialect().native_string_protocol() else {
        return Err(vm.refuse_host_command("native lambda list producer is unavailable".into()));
    };
    let Some(diagnostics) = tcl_native_model::native_lambda::NativeLambdaDiagnosticProtocol::select(
        vm.native_invocation_dialect(),
    ) else {
        return Err(
            vm.refuse_host_command("native lambda diagnostic protocol is unavailable".into())
        );
    };
    let parts = match vm.native_object_list_elements_in(lambda, protocol) {
        Ok(parts) => parts,
        Err(error) if diagnostics.wraps_list_failure(&error) => {
            let bytes = match tcl_syntax::value::ValueOps::native_string_bytes(vm, lambda) {
                Ok(bytes) => bytes,
                Err(error) => return Err(completion_from_cmd_error(vm, error.into())),
            };
            return Err(completion_from_cmd_error(
                vm,
                diagnostics.conversion_error(&bytes),
            ));
        }
        Err(error) => return Err(completion_from_cmd_error(vm, error.into())),
    };
    if !(2..=3).contains(&parts.len()) {
        let bytes = match tcl_syntax::value::ValueOps::native_string_bytes(vm, lambda) {
            Ok(bytes) => bytes,
            Err(error) => return Err(completion_from_cmd_error(vm, error.into())),
        };
        return Err(completion_from_cmd_error(
            vm,
            diagnostics.conversion_error(&bytes),
        ));
    }
    let Some(parameter_grammar) = vm.native_invocation_dialect().parameter_grammar() else {
        return Err(err("native parameter grammar is not selected"));
    };
    let body = match vm.capture_procedure_declaration_body(&parts[0], &parts[1]) {
        Ok(body) => body,
        Err(error) => return Err(completion_from_cmd_error(vm, error)),
    };
    let (params_vec, has_args) = match parse_params_value(vm, &parts[0], b"") {
        Ok(parsed) => parsed,
        Err(error) => {
            let bytes = match tcl_syntax::value::ValueOps::native_string_bytes(vm, lambda) {
                Ok(bytes) => bytes,
                Err(error) => return Err(completion_from_cmd_error(vm, error.into())),
            };
            if let Some(frame) = diagnostics.parameter_error_frame(&bytes) {
                vm.seed_error_info_frame(error.result.string_bytes(), frame);
            }
            return Err(error);
        }
    };
    Ok(PreparedLambdaDeclaration {
        parts,
        parameter_grammar,
        body,
        parameters: params_vec,
        has_args,
    })
}

/// Manufacture the actual temporary lambda declaration from original list
/// elements. The caller retains its returned binding independently of the
/// original publication handle and owns cleanup on every completion path. This is not
/// an assertion about native C commandless lambda cache storage.
pub(crate) fn build_lambda_proc(
    vm: &mut Vm,
    lambda: &Value,
    call_identity: Vec<Value>,
) -> Result<OriginalLambdaDefinition, Completion<Value>> {
    let PreparedLambdaDeclaration {
        parts,
        parameter_grammar,
        body,
        parameters: params_vec,
        has_args,
    } = prepare_lambda_declaration(vm, lambda)?;
    let activation_preflight_complete = vm.uses_native_jim_lookup();
    let usage_name = activation_preflight_complete
        .then(|| vec![Value::string("apply"), Value::string("lambdaExpr")]);
    if let Some(usage_name) = &usage_name {
        let Some(arguments) = call_identity.get(2..) else {
            return Err(
                vm.refuse_host_command("original lambda call arguments are unavailable".into())
            );
        };
        if let Some(completion) = vm.preflight_original_lambda_activation(
            &params_vec,
            parameter_grammar,
            &body,
            arguments,
            usage_name,
        )? {
            return Err(completion);
        }
    }
    let (execution_namespace, native_jim_namespace) = if vm.uses_native_jim_lookup() {
        let original = parts
            .get(2)
            .cloned()
            .unwrap_or_else(|| vm.new_jim_declaration_namespace(tcl_core_types::ROOT_NS));
        let context = crate::interp::OriginalJimNamespaceContext::for_lambda(vm, &original)
            .map_err(|error| completion_from_cmd_error(vm, error))?;
        (
            crate::interp::ProcedureNamespace::OriginalJim(context),
            Some(original),
        )
    } else {
        let token = lambda_namespace(vm, parts.get(2))?;
        (
            crate::interp::ProcedureNamespace::Resolved {
                path: vm.namespace_path_for_token(token),
                token,
            },
            None,
        )
    };
    let name = fresh_apply_name();
    // This VM-owned temporary publication is independent of the lambda's
    // original execution namespace. Jim owns a flat root command table;
    // its existing slot publisher constructs the actual CString key object.
    let (command_ns_id, simple_name) = if vm.uses_native_jim_lookup() {
        (
            tcl_core_types::ROOT_NS,
            tcl_core_types::NameBytes::from(name.as_str()),
        )
    } else {
        (
            vm.definition_namespace_token("tcl::apply"),
            tcl_core_types::NameBytes::from(tcl_syntax::naming::written_command_tail(
                name.as_bytes(),
            )),
        )
    };
    let (binding, publication) = vm.define_proc_binding(ProcDef {
        formals: crate::command::FormalParameterCache::default(),
        native_resources: Rc::default(),
        name: name.clone(),
        command_ns_id,
        simple_name,
        execution_namespace,
        params: params_vec,
        parameter_grammar,
        has_args,
        native_jim_namespace,
        native_parameters: (parameter_grammar == tcl_dialect::ParameterGrammar::Jim)
            .then(|| parts[0].clone()),

        native_header: tcl_dialect::NativeProcedureHeaderCompilation::Absent,
        statics: None,
        body: None,
        body_src: body,
        usage_name: Some(
            usage_name.unwrap_or_else(|| vec![Value::string("apply"), Value::string("lambdaExpr")]),
        ),
        call_identity: Some(call_identity),
        lambda_term: Some(lambda.clone()),
    });
    let registration = vm.active_sidecar(crate::interp::CommandSidecarKey::visible(publication));
    Ok(OriginalLambdaDefinition {
        binding,
        registration,
        activation_preflight_complete,
    })
}

fn lambda_namespace(vm: &mut Vm, original: Option<&Value>) -> Result<NsId, Completion<Value>> {
    if let Some(original) = original {
        let bytes = match vm.native_name_operand_bytes_checked(original) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Err(completion_from_cmd_error(vm, error));
            }
        };
        let recipe = vm
            .native_invocation_dialect()
            .native_name_protocol()
            .ok_or_else(|| {
                vm.refuse_host_command("lambda namespace has no original naming recipe".to_owned())
            })?;
        if recipe.is_jim084() {
            let selected = recipe.jim_lambda_namespace_input(&bytes).map_err(|error| {
                vm.refuse_host_command(format!("Jim lambda namespace input unavailable: {error}"))
            })?;
            return Ok(vm.intern_jim_namespace_object(selected.selected()));
        }
        let selected = recipe.lambda_namespace_input(&bytes).map_err(|error| {
            vm.refuse_host_command(format!("lambda namespace recipe is unavailable: {error}"))
        })?;
        let lookup = selected.selected();
        match tcl_runtime_api::Namespaces::find_namespace_bytes_checked(
            vm,
            tcl_core_types::ROOT_NS,
            lookup,
        ) {
            Ok(Some(namespace)) => Ok(namespace),
            Ok(None) => {
                let report = tcl_syntax::naming::report_native_namespace_operation_error(
                    recipe,
                    tcl_syntax::naming::NativeNamespaceLookupOperation::ObjectLookup,
                    lookup,
                    b"::",
                )
                .map_err(|_| {
                    vm.refuse_host_command("lambda namespace lookup diagnostic unavailable".into())
                })?;
                Err(vm.namespace_operation_error(report))
            }
            Err(error) => Err(completion_from_cmd_error(vm, error.into())),
        }
    } else {
        Ok(tcl_core_types::ROOT_NS)
    }
}

/// Invoke the actual lambda declaration with the original caller objects.
/// The prepared procedure request enters the explicit activation stack, so a
/// yield suspends that body and cleanup follows its actual lifetime. No source
/// word, List header, command lookup or extra trace event is manufactured.
pub(super) fn cmd_apply(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((lambda, call_args)) = args.split_first() else {
        return native_wrong_args(vm, "apply lambdaExpr ?arg ...?");
    };
    let invoked = vm
        .invoked_name_value()
        .unwrap_or_else(|| Value::string("apply"));
    let mut call_identity = Vec::with_capacity(args.len() + 1);
    call_identity.push(invoked.clone());
    call_identity.extend_from_slice(args);
    let definition = match build_lambda_proc(vm, lambda, call_identity) {
        Ok(definition) => definition,
        Err(completion) => return completion,
    };
    vm.defer_original_lambda_call(definition, invoked, call_args)
}
