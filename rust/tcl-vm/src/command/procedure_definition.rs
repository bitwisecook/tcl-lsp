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

//! `proc`: parsing formal parameters, preparing the body and publishing the procedure under its name.

use super::error_completion::completion_from_cmd_error;
use super::{Param, ProcDef, native_wrong_args};
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use std::rc::Rc;
use tcl_cmd_core::CmdError;
use tcl_runtime_api::{Completion, NsId};
use tcl_syntax::formal_params::{has_trailing_args, parse_formal_parameters_in};

/// Parse a proc parameter spec (`"a b {c 1} args"`) into params + `has_args`.
#[cfg(test)]
pub(crate) fn parse_params(spec: &str) -> Result<(Vec<Param>, bool), String> {
    parse_params_in(spec, tcl_dialect::ParameterGrammar::Tcl).map_err(|error| error.message())
}

pub(crate) fn parse_params_in(
    spec: &str,
    grammar: tcl_dialect::ParameterGrammar,
) -> Result<(Vec<Param>, bool), tcl_syntax::formal_params::FormalParameterError> {
    let parsed = parse_formal_parameters_in(spec, grammar)?;
    let has_args = match grammar {
        tcl_dialect::ParameterGrammar::Tcl => has_trailing_args(&parsed),
        tcl_dialect::ParameterGrammar::Jim => {
            parsed.iter().any(|parameter| parameter.name == "args")
        }
    };
    let params = parsed
        .into_iter()
        .map(|parameter| Param {
            name: parameter.name.into(),
            default: parameter.default.map(Value::string),
        })
        .collect();
    Ok((params, has_args))
}

/// Parse original formal objects under the interpreter's selected native policy.
/// Old C releases use `CString` list input at both levels; modern engines keep
/// native list element objects, including the original default value.
fn split_formal_values(
    vm: &mut Vm,
    value: &Value,
    protocol: tcl_syntax::naming::NativeNameProtocol,
) -> Result<Vec<Value>, CmdError> {
    use tcl_syntax::value::ValueError;
    let strings = vm
        .object_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "formal list producer",
        ))?;
    if matches!(
        protocol.tcl_version(),
        Some(tcl_dialect::TclVersion::V8_4 | tcl_dialect::TclVersion::V8_5)
    ) {
        let original = vm.native_name_operand_bytes_checked(value)?;
        let selected = protocol.formal_parameter_list_input(&original);
        let bytes = selected.selected();
        return tcl_syntax::list::split_native_list_bytes(bytes, strings)
            .map(|items| {
                items
                    .into_iter()
                    .map(|item| Value::from_string_bytes(item.into_owned()))
                    .collect()
            })
            .map_err(|error| {
                ValueError::ListParse {
                    error,
                    source: bytes.to_vec(),
                }
                .into()
            });
    }
    vm.native_object_list_elements_in(value, strings)
        .map(|items| items.as_ref().clone())
        .map_err(Into::into)
}

pub(crate) fn parse_params_value(
    vm: &mut Vm,
    spec: &Value,
    procedure: &[u8],
) -> Result<(Vec<Param>, bool), Completion<Value>> {
    use tcl_syntax::formal_params::{FormalParameterValueError, parse_formal_parameter_values};
    let Some(policy) = vm.name_policy_protocol() else {
        return Err(vm.refuse_host_command("formal storage policy is unavailable".into()));
    };
    let protocol = policy.recipe();
    let outer = match split_formal_values(vm, spec, protocol) {
        Ok(outer) => outer,
        Err(error) => return Err(completion_from_cmd_error(vm, error)),
    };
    let parsed = {
        let caller = std::cell::RefCell::new(&mut *vm);
        parse_formal_parameter_values(
            &outer,
            protocol,
            |value, _| split_formal_values(&mut caller.borrow_mut(), value, protocol),
            |value| {
                caller
                    .borrow_mut()
                    .native_name_operand_bytes_checked(value)
                    .map(|bytes| bytes.to_vec())
            },
        )
    };
    let parsed = match parsed {
        Ok(parsed) => parsed,
        Err(FormalParameterValueError::Access(error)) => {
            return Err(completion_from_cmd_error(vm, error));
        }
        Err(FormalParameterValueError::Format(error)) => {
            let message = error.message_for_definition(protocol, procedure);
            let error = match error.error_code_for_definition(protocol) {
                Some(code) => CmdError::with_error_code_bytes(message, code.to_vec()),
                None => CmdError::new_bytes(message),
            };
            return Err(completion_from_cmd_error(vm, error));
        }
    };
    if protocol.is_jim084() {
        for parameter in &parsed {
            if parameter.name == b"args"
                && let Some(default) = &parameter.default
                && let Err(error) = vm.native_name_operand_bytes_checked(default)
            {
                return Err(completion_from_cmd_error(vm, error));
            }
        }
    }
    let has_args = parsed.iter().enumerate().any(|(index, parameter)| {
        parameter.name == b"args" && (protocol.is_jim084() || index + 1 == parsed.len())
    });
    Ok((
        parsed
            .into_iter()
            .map(|parameter| Param {
                name: parameter.name.into(),
                default: parameter.default,
            })
            .collect(),
        has_args,
    ))
}

/// `proc name params body` — retain a procedure declaration. Its original
/// body is prepared by the activation owner when the procedure is called.
pub(super) fn cmd_proc(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    define_procedure(vm, args, None)
}

/// Authored callable publication retains a private command owner and ROOT body scope.
pub(crate) fn cmd_authored_rule_proc(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let original = vm.active_native_profile;
    vm.active_native_profile = Some(vm.actual_native_execution_profile());
    let completion = define_procedure(vm, args, Some(tcl_core_types::ROOT_NS));
    vm.active_native_profile = original;
    completion
}

struct PreparedProcedureBody {
    pub(super) body: Value,
    pub(super) statics: Option<Rc<crate::vars::StaticVariables>>,
    pub(super) parameters: Vec<Param>,
    has_args: bool,
}

fn prepare_procedure_before_name(
    vm: &mut Vm,
    args: &[Value],
    params: &Value,
    body_text: &Value,
    statics_at: Option<usize>,
    formals_precede_name: bool,
) -> Result<Option<PreparedProcedureBody>, Completion<Value>> {
    if !formals_precede_name {
        return Ok(None);
    }
    let body = vm
        .capture_procedure_declaration_body(params, body_text)
        .map_err(|error| completion_from_cmd_error(vm, error))?;
    let statics = statics_at
        .map(|index| vm.prepare_procedure_statics_value(&args[index]))
        .transpose()?;
    let (parameters, has_args) = parse_params_value(vm, params, b"")?;
    Ok(Some(PreparedProcedureBody {
        body,
        statics,
        parameters,
        has_args,
    }))
}

fn prepare_procedure_after_name(
    vm: &mut Vm,
    args: &[Value],
    params: &Value,
    body_text: &Value,
    statics_at: Option<usize>,
    written: &[u8],
) -> Result<PreparedProcedureBody, Completion<Value>> {
    let body = vm
        .capture_procedure_declaration_body(params, body_text)
        .map_err(|error| completion_from_cmd_error(vm, error))?;
    let statics = statics_at
        .map(|index| vm.prepare_procedure_statics_value(&args[index]))
        .transpose()?;
    let (parameters, has_args) = parse_params_value(vm, params, written)?;
    Ok(PreparedProcedureBody {
        body,
        statics,
        parameters,
        has_args,
    })
}

struct ProcedurePublication<'a> {
    pub(super) name: &'a Value,
    pub(super) params: &'a Value,
    written: Rc<[u8]>,
    pub(super) body: Value,
    pub(super) statics: Option<Rc<crate::vars::StaticVariables>>,
    pub(super) parameters: Vec<Param>,
    has_args: bool,
    pub(super) parameter_grammar: tcl_dialect::ParameterGrammar,
    pub(super) execution_namespace: Option<NsId>,
    pub(super) result: tcl_registry::native_procedure::ProcedureDefinitionResult,
}

fn publish_procedure(vm: &mut Vm, input: ProcedurePublication<'_>) -> Completion<Value> {
    let ProcedurePublication {
        name,
        params,
        written,
        body,
        statics,
        parameters,
        has_args,
        parameter_grammar,
        execution_namespace,
        result,
    } = input;
    let (reg_name, slot, ns_id) = match vm.native_procedure_publication(&written) {
        Ok(publication) => publication,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let ns_id = execution_namespace.unwrap_or(ns_id);
    let namespace = vm.namespace_path_for_token(ns_id);
    let native_header = match procedure_header_compilation(vm, params, &body) {
        Ok(header) => header,
        Err(completion) => return completion,
    };
    let report = match vm.original_jim_publication_key(
        name,
        tcl_syntax::naming::NativeNamePurpose::CommandPublication,
    ) {
        Ok(report) => report,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    vm.define_proc_binding_with_jim_key(
        ProcDef {
            formals: crate::command::FormalParameterCache::default(),
            native_resources: Rc::default(),
            name: reg_name,
            command_ns_id: slot.namespace,
            simple_name: slot.simple,
            execution_namespace: crate::interp::ProcedureNamespace::Resolved {
                path: namespace,
                token: ns_id,
            },
            params: parameters,
            parameter_grammar,
            has_args,
            native_jim_namespace: None,
            native_parameters: (parameter_grammar == tcl_dialect::ParameterGrammar::Jim)
                .then(|| params.clone()),
            native_header,
            statics,
            body: None,
            body_src: body,
            usage_name: None,
            call_identity: None,
            lambda_term: None,
        },
        report,
    );
    ok(match result {
        tcl_registry::native_procedure::ProcedureDefinitionResult::Empty => Value::empty(),
        tcl_registry::native_procedure::ProcedureDefinitionResult::NameArgument => name.clone(),
    })
}

fn define_procedure(
    vm: &mut Vm,
    args: &[Value],
    execution_namespace: Option<NsId>,
) -> Completion<Value> {
    use tcl_registry::native_procedure::NativeProcedureDefinitionSelection;
    let words = vec![tcl_registry::InvocationWord::Dynamic; args.len()];
    let dialect = vm.native_invocation_dialect();
    let arguments = tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect);
    let NativeProcedureDefinitionSelection::Valid(definition) =
        tcl_registry::native_procedure::select_native_procedure_definition(arguments)
    else {
        let usage = tcl_registry::native_procedure::NativeProcedureDefinitionSpec::Core
            .usage(Some(dialect))
            .unwrap_or("proc name args body");
        return native_wrong_args(vm, usage);
    };
    let name = &args[definition.name_at];
    let params = &args[definition.parameters_at];
    let body_text = &args[definition.body_at];
    let Some(creation) = tcl_registry::native_procedure::procedure_activation_protocol(
        vm.actual_native_invocation_dialect(),
    ) else {
        return completion_from_cmd_error(
            vm,
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original procedure definition order",
            )
            .into(),
        );
    };
    let prepared = match prepare_procedure_before_name(
        vm,
        args,
        params,
        body_text,
        definition.statics_at,
        creation.definition_formals_precede_name(),
    ) {
        Ok(prepared) => prepared,
        Err(error) => return error,
    };
    let written = match vm.native_name_operand_bytes_checked(name) {
        Ok(bytes) => bytes,
        Err(error) => {
            return completion_from_cmd_error(vm, error);
        }
    };
    if let Err(completion) = validate_procedure_publication_name(vm, &written) {
        return completion;
    }
    let Some(parameter_grammar) = vm.native_invocation_dialect().parameter_grammar() else {
        return err("native parameter grammar is not selected");
    };
    let prepared = match prepared {
        Some(prepared) => prepared,
        None => match prepare_procedure_after_name(
            vm,
            args,
            params,
            body_text,
            definition.statics_at,
            &written,
        ) {
            Ok(prepared) => prepared,
            Err(error) => return error,
        },
    };
    let PreparedProcedureBody {
        body,
        statics,
        parameters,
        has_args,
    } = prepared;
    publish_procedure(
        vm,
        ProcedurePublication {
            name,
            params,
            written,
            body,
            statics,
            parameters,
            has_args,
            parameter_grammar,
            execution_namespace,
            result: definition.result,
        },
    )
}

/// Reach native header getters only when the original compiler needs them.
pub(super) fn procedure_header_compilation(
    vm: &mut Vm,
    params: &Value,
    body_text: &Value,
) -> Result<tcl_dialect::NativeProcedureHeaderCompilation, Completion<Value>> {
    let dialect = vm.actual_native_invocation_dialect();
    let mut native_header = tcl_registry::native_procedure::procedure_header_compilation_bytes(
        dialect,
        None,
        None,
        Some(false),
    );
    if native_header == tcl_dialect::NativeProcedureHeaderCompilation::Unknown {
        let Some(protocol) = dialect.native_string_protocol() else {
            return Err(vm.refuse_host_command(
                "native procedure header string producer is unavailable".into(),
            ));
        };
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(vm)
            .map_err(|error| completion_from_cmd_error(vm, error))?;
        let reached = (|| {
            crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(vm, &scope)?
                .string(params, protocol)
        })();
        let parameter_bytes = match reached {
            Ok(bytes) => bytes,
            Err(error) => return Err(completion_from_cmd_error(vm, error)),
        };
        native_header = tcl_registry::native_procedure::procedure_header_compilation_bytes(
            dialect,
            Some(&parameter_bytes),
            None,
            Some(false),
        );
        if native_header == tcl_dialect::NativeProcedureHeaderCompilation::Unknown {
            let reached = (|| {
                crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(
                    vm, &scope,
                )?
                .string(body_text, protocol)
            })();
            let body_bytes = match reached {
                Ok(bytes) => bytes,
                Err(error) => return Err(completion_from_cmd_error(vm, error)),
            };
            native_header = tcl_registry::native_procedure::procedure_header_compilation_bytes(
                dialect,
                Some(&parameter_bytes),
                Some(&body_bytes),
                Some(false),
            );
        }
    }
    Ok(native_header)
}

fn validate_procedure_publication_name(
    vm: &mut Vm,
    written: &[u8],
) -> Result<(), Completion<Value>> {
    match vm.native_procedure_holder_exists(written) {
        Ok(true) => {}
        Ok(false) => {
            let Some((message, code)) =
                tcl_registry::native_procedure::procedure_unknown_namespace_error(
                    vm.native_invocation_dialect(),
                    written,
                )
            else {
                return Err(vm.refuse_host_command(
                    "native procedure namespace diagnostic is unavailable".into(),
                ));
            };
            let error = match code {
                Some(code) => tcl_cmd_core::CmdError::with_error_code_bytes(message, code),
                None => tcl_cmd_core::CmdError::new_bytes(message),
            };
            return Err(completion_from_cmd_error(vm, error));
        }
        Err(error) => return Err(vm.refuse_host_command(error.to_string())),
    }
    let Some(policy) = vm.name_policy_protocol() else {
        return Err(vm.refuse_host_command("procedure name validation is unavailable".into()));
    };
    if !policy.recipe().is_jim084() {
        let path = vm.namespace_path_for_token(vm.current_ns_id());
        let slot = match policy
            .recipe()
            .command_lookup_slot(tcl_syntax::naming::NativeNameContext::new(&path), written)
        {
            Ok(slot) => slot,
            Err(error) => return Err(vm.refuse_host_command(error.to_string())),
        };
        match tcl_registry::native_procedure::procedure_name_creation_error_for_policy(
            policy,
            slot.namespace.as_segments().is_empty(),
            slot.simple.as_bytes(),
        ) {
            Ok(()) => {}
            Err(message) => {
                let mut error = tcl_cmd_core::CmdError::new_bytes(message);
                if policy.authority() == tcl_syntax::naming::NamePolicyAuthority::Native {
                    error = error.with_native_string_result(policy.recipe().string_protocol());
                }
                return Err(completion_from_cmd_error(vm, error));
            }
        }
    }
    Ok(())
}
