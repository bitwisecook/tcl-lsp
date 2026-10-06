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

//! `proc` (define a user procedure) + `puts` (output) — toward executing scripts.
//!
//! `proc name params body` parses the parameter spec (names, `{name default}`
//! pairs, and a trailing `args` catch-all) and registers a
//! [`Command::Proc`](crate::interp::Command); the call protocol
//! (`Interp::call_proc`) pushes a frame, binds the args, and runs the body —
//! see `proc-call-and-stack-traces.md`. `puts` writes to stdout/stderr.

use crate::interp::{obj_bytes, CallMeta, Code, Interp, Param, ProcFrame};
use crate::obj::TclObj;
use crate::obj::{self, Owned};
use tcl_cmd_core::CmdError;
use tcl_syntax::value::ValueOps;
#[cfg(test)]
mod native_name_tests;

/// Register `proc`, `apply`, and `puts`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"proc", proc_cmd);
    interp.register_builtin(b"apply", apply_cmd);
}

/// Install audited distribution wrappers without replacing host or user commands.
pub(crate) fn install_stock_scripted_wrappers(interp: &mut Interp) {
    for wrapper in
        tcl_registry::dictionary_scope::stock_scripted_wrappers(interp.native_invocation_dialect())
    {
        let qualified_name = tcl_syntax::naming::qualify("::", wrapper.command);
        if interp.command_exists(qualified_name.as_bytes()) {
            continue;
        }
        let original_parameters =
            Owned::fresh(crate::interp::new_string(wrapper.parameters.as_bytes()));
        let Ok(parameters) = parse_params_object(
            interp,
            original_parameters.as_ptr(),
            wrapper.command.as_bytes(),
        ) else {
            continue;
        };
        let body = crate::interp::new_string(wrapper.body.as_bytes());
        let body = crate::obj::Owned::fresh(body);
        interp.define_proc_original_storage(
            qualified_name.as_bytes(),
            parameters,
            Some(original_parameters.as_ptr()),
            body.as_ptr(),
            None,
            None,
        );
    }
}

/// `proc name params body` — define a procedure.
fn proc_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use tcl_registry::native_procedure::{
        NativeProcedureDefinitionSelection, NativeProcedureDefinitionSpec,
        ProcedureDefinitionResult,
    };
    let dialect = interp.native_invocation_dialect();
    let argument_count = argv.len() - 1;
    // This selected definition grammar needs cardinality only. Its name,
    // parameter bytes and body remain their actual evaluated values.
    let words = vec![tcl_registry::InvocationWord::Dynamic; argument_count];
    let selected = NativeProcedureDefinitionSpec::Core
        .select(tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect));
    let NativeProcedureDefinitionSelection::Valid(definition) = selected else {
        return interp.wrong_args(
            NativeProcedureDefinitionSpec::Core
                .usage(Some(dialect))
                .unwrap_or("proc name args body")
                .as_bytes(),
        );
    };
    let name = obj_bytes(argv[1]);
    // A namespace-qualified proc name requires that namespace to already exist
    // (C's `Tcl_ProcObjCmd` via `TclGetNamespaceForQualName`).
    let flat_jim = interp
        .name_policy_protocol()
        .is_some_and(|protocol| protocol.recipe().is_jim084());
    if !flat_jim {
        let selected_holder = {
            let namespaces = interp.namespaces();
            namespaces.procedure_lookup_at(interp.current_ns(), &name)
        };
        let Some((holder, simple)) = selected_holder else {
            let Some((message, code)) =
                tcl_registry::native_procedure::procedure_unknown_namespace_error(dialect, &name)
            else {
                return interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "procedure name validation",
                    )
                    .into(),
                );
            };
            return interp.report_cmd_error(match code {
                Some(code) => CmdError::with_error_code_bytes(message, code),
                None => CmdError::new_bytes(message),
            });
        };
        match tcl_registry::native_procedure::procedure_name_creation_error(
            dialect,
            holder == crate::namespace::GLOBAL,
            &simple,
        ) {
            Some(Ok(())) => {}
            Some(Err(message)) => {
                let protocol = dialect
                    .native_name_protocol()
                    .expect("selected procedure name error recipe");
                return interp.report_cmd_error(
                    CmdError::new_bytes(message)
                        .with_native_string_result(protocol.string_protocol()),
                );
            }
            None => {
                return interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "procedure name validation",
                    )
                    .into(),
                )
            }
        }
    }
    let original_body = argv[definition.body_at + 1];
    // C chooses its body before parsing the formal list, whose conversions
    // can enter callbacks and change the incoming object's sharing.
    let chosen_body = match interp.choose_original_procedure_body(original_body) {
        Ok(body) => body,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let params = match parse_params_object(
        interp,
        argv[definition.parameters_at + 1],
        tcl_syntax::naming::written_command_tail(&name),
    ) {
        Ok(parameters) => parameters,
        Err(error) => return interp.report_cmd_error(error),
    };
    let statics = if let Some(index) = definition.statics_at {
        match prepare_static_variables(interp, &obj_bytes(argv[index + 1])) {
            Ok(statics) => Some(std::rc::Rc::new(statics)),
            Err(error) => return interp.set_error(&error),
        }
    } else {
        None
    };
    interp.define_proc_chosen_storage(
        &name,
        params,
        Some(argv[definition.parameters_at + 1]),
        (original_body, chosen_body),
        None,
        statics,
    );
    if interp.host_refusal_pending() {
        return Code::Error;
    }
    match definition.result {
        ProcedureDefinitionResult::Empty => interp.set_result_bytes(b""),
        ProcedureDefinitionResult::NameArgument => interp.set_result(argv[definition.name_at + 1]),
    }
    Code::Ok
}

fn prepare_static_variables(
    interp: &mut Interp,
    source: &[u8],
) -> Result<crate::frame::StaticVariables, Vec<u8>> {
    use tcl_registry::native_procedure::{parse_static_variables, StaticVariableInitialiser};
    let source = core::str::from_utf8(source)
        .map_err(|_| b"invalid statics list (not valid UTF-8)".to_vec())?;
    let declarations =
        parse_static_variables(source).map_err(|error| error.message().into_bytes())?;
    let mut statics = crate::frame::StaticVariables::default();
    for declaration in declarations {
        let name = declaration.name.as_bytes();
        match declaration.initialiser {
            StaticVariableInitialiser::Literal(value) => {
                statics.insert_literal(name, crate::interp::new_string(value.as_bytes()));
            }
            initializer @ (StaticVariableInitialiser::CopyCurrent(_)
            | StaticVariableInitialiser::CaptureCurrentCell(_)) => {
                let reference = matches!(
                    initializer,
                    StaticVariableInitialiser::CaptureCurrentCell(_)
                );
                let source_name = match initializer {
                    StaticVariableInitialiser::CopyCurrent(source)
                    | StaticVariableInitialiser::CaptureCurrentCell(source) => source,
                    StaticVariableInitialiser::Literal(_) => unreachable!(),
                };
                let Some(source) = crate::vars::capture_static_source(
                    &interp.frames.borrow(),
                    &interp.namespaces(),
                    interp.current_ns(),
                    source_name.as_bytes(),
                    reference,
                ) else {
                    return Err(format!(
                        "variable for initialization of static \"{}\" not found in the local context",
                        declaration.name,
                    ).into_bytes());
                };
                if reference {
                    statics.insert_capture(name, source);
                } else {
                    assert!(statics.insert_copy(name, source));
                }
            }
        }
    }
    Ok(statics)
}

/// Parse a newly constructed formal-list string under the actual interpreter policy.
pub(crate) fn parse_params_in(interp: &mut Interp, spec: &[u8]) -> Result<Vec<Param>, CmdError> {
    let original = Owned::fresh(crate::interp::new_string(spec));
    parse_params_object(interp, original.as_ptr(), b"")
}

#[cfg(test)]
pub(crate) fn parse_params(spec: &[u8]) -> Result<Vec<Param>, Vec<u8>> {
    parse_params_in(&mut Interp::new(), spec).map_err(|error| error.into_byte_details().message)
}

fn split_formal_objects(
    interp: &mut Interp,
    value: &Owned,
    protocol: tcl_syntax::naming::NativeNameProtocol,
) -> Result<Vec<Owned>, CmdError> {
    let pointer = value.as_ptr();
    let string_protocol = protocol.string_protocol();
    if matches!(
        protocol.tcl_version(),
        Some(tcl_dialect::TclVersion::V8_4 | tcl_dialect::TclVersion::V8_5)
    ) {
        let original = ValueOps::native_string_bytes(interp, &pointer)?;
        let bytes = &original[..original
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(original.len())];
        return tcl_syntax::list::split_native_list_bytes(bytes, string_protocol)
            .map(|elements| {
                elements
                    .into_iter()
                    .map(|element| Owned::fresh(crate::interp::new_string(&element)))
                    .collect()
            })
            .map_err(|error| {
                tcl_syntax::value::ValueError::ListParse {
                    error,
                    source: bytes.to_vec(),
                }
                .into()
            });
    }
    let elements = ValueOps::list_elements(interp, &pointer)?;
    Ok(elements.into_iter().map(Owned::retain).collect())
}

/// Parse the original native formal objects, retaining default object identity.
pub(crate) fn parse_params_object(
    interp: &mut Interp,
    spec: *mut TclObj,
    procedure: &[u8],
) -> Result<Vec<Param>, CmdError> {
    use tcl_syntax::formal_params::{parse_formal_parameter_values, FormalParameterValueError};
    let protocol = interp
        .name_policy_protocol()
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "formal storage",
        ))?
        .recipe();
    let original = Owned::retain(spec);
    let outer = split_formal_objects(interp, &original, protocol)?;
    let interpreter = std::cell::RefCell::new(interp);
    let parsed = parse_formal_parameter_values(
        &outer,
        protocol,
        |value, _| split_formal_objects(&mut interpreter.borrow_mut(), value, protocol),
        |value| {
            ValueOps::native_string_bytes(&mut **interpreter.borrow_mut(), &value.as_ptr())
                .map(|bytes| bytes.to_vec())
                .map_err(CmdError::from)
        },
    )
    .map_err(|error| match error {
        FormalParameterValueError::Access(error) => error,
        FormalParameterValueError::Format(error) => {
            CmdError::new_bytes(error.message_for_definition(protocol, procedure))
        }
    })?;
    if protocol.is_jim084() {
        for parameter in &parsed {
            if parameter.name == b"args" {
                if let Some(default) = &parameter.default {
                    ValueOps::native_string_bytes(
                        &mut **interpreter.borrow_mut(),
                        &default.as_ptr(),
                    )?;
                }
            }
        }
    }
    Ok(parsed
        .into_iter()
        .map(|parameter| Param {
            name: parameter.name,
            default: parameter.default,
        })
        .collect())
}

/// `apply {params body ?namespace?} ?arg ...?` — invoke an anonymous procedure.
/// The lambda runs in `namespace` (default global), via the shared proc-call
/// protocol (`Interp::run_proc`).
fn apply_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"apply lambdaExpr ?arg ...?");
    }
    let Some(protocol) = interp.name_policy_protocol().map(|policy| policy.recipe()) else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("lambda list").into(),
        );
    };
    let original = Owned::retain(argv[1]);
    if interp
        .native_invocation_dialect()
        .native_string_protocol()
        .is_some_and(|protocol| protocol.tcl_version().is_some())
    {
        return apply_native_c(interp, argv, &original, protocol);
    }
    let mut parts = match split_formal_objects(interp, &original, protocol) {
        Ok(parts) => parts,
        Err(error) => return interp.report_cmd_error(error),
    };
    let lambda = obj_bytes(argv[1]);
    if parts.len() < 2 || parts.len() > 3 {
        let mut m = b"can't interpret \"".to_vec();
        m.extend_from_slice(&lambda);
        m.extend_from_slice(b"\" as a lambda expression");
        return interp.set_error(&m);
    }
    let params = match parse_params_object(interp, parts[0].as_ptr(), b"") {
        Ok(parameters) => parameters,
        Err(error) => {
            let code = interp.report_cmd_error(error);
            interp.append_lambda_parse_frame(&lambda);
            return code;
        }
    };
    let jim = interp.native_invocation_dialect().native_string_protocol()
        == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084);
    let ns = if jim {
        let namespace = if parts.len() == 3 {
            parts.pop().expect("original lambda namespace")
        } else {
            match interp.native_jim_object_context() {
                Ok(context) => context.empty_object().clone(),
                Err(error) => return interp.report_cmd_error(error.into()),
            }
        };
        let bytes = match ValueOps::native_string_bytes(interp, &namespace.as_ptr()) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        interp
            .namespaces_mut()
            .retain_jim_namespace(namespace, bytes)
    } else if parts.len() == 3 {
        // C prepends `::` to a non-global-qualified namespace name, then resolves
        // it (`TclGetNamespaceFromObj`); a missing namespace is an error — apply
        // does **not** create it.
        let namespace = match ValueOps::native_string_bytes(interp, &parts[2].as_ptr()) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let full: Vec<u8> = if namespace.starts_with(b"::") {
            namespace.to_vec()
        } else {
            let mut f = b"::".to_vec();
            f.extend_from_slice(&namespace);
            f
        };
        match interp.find_namespace_id(&full) {
            Some(id) => id,
            None => {
                let mut m = b"namespace \"".to_vec();
                m.extend_from_slice(&full);
                m.extend_from_slice(b"\" not found");
                let mut ecode = b"TCL LOOKUP NAMESPACE ".to_vec();
                ecode.extend_from_slice(&full);
                return interp.error_with_code(&m, &ecode);
            }
        }
    } else {
        crate::namespace::GLOBAL
    };
    // A literal lambda in a sourced file reports `type source` at its body's line
    // (element 1 of the lambda list — TIP 280); a dynamic lambda is body-relative.
    let (source, body_line_base) = match interp.list_element_location(argv[1], 1) {
        Some((file, line)) => (Some(file), line.saturating_sub(1)),
        None => (None, 0),
    };
    // `info level N` of a lambda reports the actual invocation words
    // (`apply <lambdaExpr> ?arg ...?`), not the `apply lambdaExpr` usage prefix
    // used for `wrong # args` (C records `objv` verbatim for the lambda frame).
    let level_words: Vec<Vec<u8>> = argv.iter().map(|&a| obj_bytes(a)).collect();
    let body = match ValueOps::native_string_bytes(interp, &parts[1].as_ptr()) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let jim_namespace = if jim {
        interp.namespaces_mut().take_jim_namespace_owner(ns)
    } else {
        None
    };
    interp.run_proc(
        &params,
        &body,
        ns,
        &argv[2..],
        b"apply lambdaExpr",
        CallMeta {
            original_argv: Some(argv),
            err: ProcFrame::Lambda(&lambda),
            fqn: None,
            source,
            body_line_base,
            link_vars: &[],
            keep_loop_codes: false,
            same_level: false,
            usage_prefix: None,
            level_words: Some(level_words),
            quote_name: false,
            // A lambda has no definition to carry a compiled body on.
            native: None,
            statics: None,
            c_procedure: None,
            c_method_client_data: None,
            jim_parameters: jim.then_some(parts[0].as_ptr()),
            jim_body: jim.then_some(parts[1].as_ptr()),
            jim_namespace: jim_namespace.as_ref(),
        },
    )
}

/// C keeps a genuine commandless Proc in the original lambdaExpr primary.
fn apply_native_c(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    original: &Owned,
    protocol: tcl_syntax::naming::NativeNameProtocol,
) -> Code {
    let interpreter = interp.native_callable_interpreter();
    let (owner, namespace, lambda) =
        match obj::native_lambda_expression::cached(original.as_ptr(), interpreter) {
            Some((owner, namespace)) => {
                let lambda = match ValueOps::native_string_bytes(interp, &original.as_ptr()) {
                    Ok(bytes) => bytes,
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                (owner, namespace, lambda)
            }
            None => {
                let parts = match crate::list::list_elements_native_checked(
                    original.as_ptr(),
                    protocol.string_protocol(),
                ) {
                    Ok(parts) => parts,
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                let lambda = match ValueOps::native_string_bytes(interp, &original.as_ptr()) {
                    Ok(bytes) => bytes,
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                if !(2..=3).contains(&parts.len()) {
                    let mut message = b"can't interpret \"".to_vec();
                    message.extend_from_slice(&lambda);
                    message.extend_from_slice(b"\" as a lambda expression");
                    return interp.error_with_code(&message, b"TCL VALUE LAMBDA");
                }
                let chosen = match interp.choose_original_procedure_body(parts[1]) {
                    Ok(chosen) => chosen,
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                let params = match parse_params_object(interp, parts[0], b"") {
                    Ok(params) => params,
                    Err(error) => {
                        let code = interp.report_cmd_error(error);
                        interp.append_lambda_parse_frame(&lambda);
                        return code;
                    }
                };
                let (source, body_line_base) = interp
                    .list_element_location(original.as_ptr(), 1)
                    .map_or((None, 0), |(file, line)| {
                        (Some(file), line.saturating_sub(1))
                    });
                let owner = interp.create_native_callable_from_chosen(
                    params,
                    chosen,
                    crate::namespace::GLOBAL,
                    source,
                    body_line_base,
                );
                let namespace = if parts.len() == 2 {
                    Owned::fresh(obj::new_string_bytes(b"::"))
                } else {
                    let bytes = match ValueOps::native_string_bytes(interp, &parts[2]) {
                        Ok(bytes) => bytes,
                        Err(error) => return interp.report_cmd_error(error.into()),
                    };
                    if bytes.starts_with(b"::") {
                        Owned::retain(parts[2])
                    } else {
                        let mut absolute = b"::".to_vec();
                        absolute.extend_from_slice(&bytes);
                        Owned::fresh(obj::new_string_bytes(&absolute))
                    }
                };
                let namespace_ptr = namespace.as_ptr();
                let invocation = owner.clone();
                obj::native_lambda_expression::install(
                    original.as_ptr(),
                    obj::native_lambda_expression::LambdaExpression {
                        interpreter,
                        procedure: owner,
                        namespace,
                    },
                );
                (invocation, namespace_ptr, lambda)
            }
        };
    let ns = match interp.native_namespace_object_lookup(namespace) {
        Ok(Some(ns)) => ns,
        Ok(None) => {
            let name = match ValueOps::native_string_bytes(interp, &namespace) {
                Ok(name) => name,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
            let mut message = b"namespace \"".to_vec();
            message.extend_from_slice(&name);
            message.extend_from_slice(b"\" not found");
            let mut code = b"TCL LOOKUP NAMESPACE ".to_vec();
            code.extend_from_slice(&name);
            return interp.error_with_code(&message, &code);
        }
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let procedure = owner.declaration();
    let body = match procedure
        .body
        .checked_ptr()
        .and_then(|original| ValueOps::native_string_bytes(interp, &original))
    {
        Ok(body) => body,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let level_words = argv.iter().map(|&argument| obj_bytes(argument)).collect();
    interp.run_proc(
        &procedure.params,
        &body,
        ns,
        &argv[2..],
        b"apply lambdaExpr",
        CallMeta {
            original_argv: Some(argv),
            err: ProcFrame::Lambda(&lambda),
            fqn: None,
            source: procedure.source.clone(),
            body_line_base: procedure.body_line_base,
            link_vars: &[],
            keep_loop_codes: false,
            same_level: false,
            usage_prefix: None,
            level_words: Some(level_words),
            quote_name: false,
            native: None,
            statics: None,
            c_procedure: Some(&procedure),
            c_method_client_data: None,
            jim_parameters: None,
            jim_body: None,
            jim_namespace: None,
        },
    )
}

// `puts` lives in `cmd_chan` (it is a channel write — stdout/stderr/file).

#[cfg(test)]
mod tests {
    use super::{obj_bytes, parse_params};
    use crate::counters;
    use crate::interp::{Code, Interp};
    use crate::obj::Owned;

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    #[test]
    fn ordinary_c_definition_keeps_the_body_chosen_before_formal_parsing() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interpreter = Interp::new();
            interpreter.set_dialect_profile(crate::environment::profile_for_dialect(profile));
            let command = Owned::fresh(crate::interp::new_string(b"proc"));
            let name = Owned::fresh(crate::interp::new_string(b"original"));
            let parameters = Owned::fresh(crate::interp::new_string(b""));
            let recipe = interpreter
                .native_invocation_dialect()
                .byte_array_string_recipe(None)
                .unwrap();
            let body = Owned::fresh(crate::bytearray::new_byte_array(b"return ORIGINAL", recipe));
            let original_type = crate::obj::obj_type_ptr(body.as_ptr());
            assert_eq!(
                super::proc_cmd(
                    &mut interpreter,
                    &[
                        command.as_ptr(),
                        name.as_ptr(),
                        parameters.as_ptr(),
                        body.as_ptr()
                    ]
                ),
                Code::Ok
            );
            let definition = interpreter.proc_def(b"original").unwrap();
            assert_eq!(definition.body.as_ptr(), body.as_ptr());
            assert_eq!(crate::obj::obj_type_ptr(body.as_ptr()), original_type);
            assert!(!crate::obj::has_string_rep(body.as_ptr()));

            let alias = Owned::retain(body.as_ptr());
            assert_eq!(
                super::proc_cmd(
                    &mut interpreter,
                    &[
                        command.as_ptr(),
                        name.as_ptr(),
                        parameters.as_ptr(),
                        body.as_ptr()
                    ]
                ),
                Code::Ok
            );
            let replacement = interpreter.proc_def(b"original").unwrap();
            assert_ne!(replacement.body.as_ptr(), body.as_ptr());
            assert!(crate::obj::obj_type_ptr(replacement.body.as_ptr()).is_null());
            assert_eq!(crate::obj::obj_type_ptr(body.as_ptr()), original_type);
            drop(alias);

            let rest_parameters = Owned::fresh(crate::interp::new_string(b" args "));
            let whitespace = Owned::fresh(crate::bytearray::new_byte_array(b" \t\n", recipe));
            assert_eq!(
                super::proc_cmd(
                    &mut interpreter,
                    &[
                        command.as_ptr(),
                        name.as_ptr(),
                        rest_parameters.as_ptr(),
                        whitespace.as_ptr()
                    ]
                ),
                Code::Ok
            );
            let no_op = interpreter.proc_def(b"original").unwrap();
            assert_eq!(
                no_op.compiler_header.get(),
                tcl_dialect::NativeProcedureHeaderCompilation::NoOp
            );
            assert!(crate::obj::has_string_rep(whitespace.as_ptr()));
            let token = interpreter
                .namespaces()
                .command_generation(crate::namespace::GLOBAL, b"original")
                .unwrap();
            assert_eq!(
                interpreter.namespaces().native_compiler_hook(token),
                Some(tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Present)
            );
        }
    }

    #[test]
    fn native_formals_keep_storage_keys_and_original_default_objects() {
        use crate::obj::{self, Owned};
        let native_defaults = [
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-8.4.20.jsonl"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-8.5.19.jsonl"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-8.6.18.jsonl"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-9.0.4.jsonl"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-9.1.0.jsonl"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-jim0.84.jsonl"
            ),
        ];
        for (profile, observation) in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"]
            .into_iter()
            .zip(native_defaults)
        {
            let mut interpreter = Interp::new();
            interpreter.set_dialect_profile(crate::environment::profile_for_dialect(profile));
            let name_spec = Owned::fresh(crate::interp::new_string(b"name\0tail"));
            let parameters = super::parse_params_object(&mut interpreter, name_spec.as_ptr(), b"p")
                .unwrap_or_else(|error| panic!("{profile}: {:?}", error.message_bytes()));
            let old_c = matches!(profile, "tcl8.4" | "tcl8.5");
            assert_eq!(
                parameters[0].name,
                if old_c {
                    &b"name"[..]
                } else {
                    &b"name\0tail"[..]
                }
            );

            let default = Owned::fresh(obj::new_double_obj(1.5));
            let name = Owned::fresh(crate::interp::new_string(b"n\xFF"));
            let pair = Owned::fresh(crate::list::new_list_obj(&[
                name.as_ptr(),
                default.as_ptr(),
            ]));
            let formals = Owned::fresh(crate::list::new_list_obj(&[pair.as_ptr()]));
            let parameters = super::parse_params_object(&mut interpreter, formals.as_ptr(), b"p")
                .unwrap_or_else(|error| panic!("{profile}: {:?}", error.message_bytes()));
            assert_eq!(parameters[0].name, b"n\xFF");
            let retained = parameters[0].default.as_ref().unwrap();
            assert_eq!(
                retained.as_ptr() == default.as_ptr(),
                observation.contains("\"same_object\":true"),
                "{profile}: default object"
            );
            assert_eq!(obj_bytes(retained.as_ptr()), b"1.5");
        }
    }

    #[test]
    fn info_queries_preserve_the_retained_default_object() {
        use crate::obj::{self, Owned};
        let mut interpreter = Interp::new();
        let original = Owned::fresh(obj::new_double_obj(1.5));
        let name = Owned::fresh(crate::interp::new_string(b"n\xFF"));
        let pair = Owned::fresh(crate::list::new_list_obj(&[
            name.as_ptr(),
            original.as_ptr(),
        ]));
        let formals = Owned::fresh(crate::list::new_list_obj(&[pair.as_ptr()]));
        let parameters =
            super::parse_params_object(&mut interpreter, formals.as_ptr(), b"p").unwrap();
        let body = Owned::fresh(crate::interp::new_string(b"return OK"));
        interpreter.define_proc(b"p", parameters, body.as_ptr());
        let procedure = Owned::fresh(crate::interp::new_string(b"p"));
        let names =
            Owned::fresh(tcl_cmd_core::info::args(&mut interpreter, &procedure.as_ptr()).unwrap());
        assert!(!obj::has_string_rep(original.as_ptr()));
        let _ = names;
        let (value, declared) =
            tcl_cmd_core::info::default(&mut interpreter, &procedure.as_ptr(), &name.as_ptr())
                .unwrap();
        assert!(declared);
        assert_eq!(value, original.as_ptr());
        assert!(!obj::has_string_rep(original.as_ptr()));
    }

    fn run(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?} → {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    #[test]
    fn jim_static_cells_match_native_definition_and_retirement() {
        const CASES: &[(&[u8], Code, &[u8])] = &[
            (b"proc p {} {{x 0}} {incr x}; list [p] [p]", Code::Ok, b"1 2"),
            (b"set x OLD; proc p {} {x} {set x}; set x NEW; list [p] $x", Code::Ok, b"OLD NEW"),
            (b"set x OLD; proc p {} {&x} {set x NEW}; p; set x", Code::Ok, b"NEW"),
            (b"set x OLD; proc p {} {&x} {set x}; unset x; set x NEW; list [p] $x", Code::Ok, b"OLD NEW"),
            (b"set x OUTER; proc p {x} {&x} {set x}; list [p PARAM] $x", Code::Ok, b"PARAM PARAM"),
            (b"proc p {} {{x STATIC}} {set x LOCAL; set x}; list [p] [p]", Code::Ok, b"LOCAL LOCAL"),
            (b"proc maker {} {set x KEEP; proc p {} {&x} {set x}}; maker; p", Code::Ok, b"KEEP"),
            (b"proc p {} {{x 0}} {incr x}; rename p q; list [q] [q]", Code::Ok, b"1 2"),
            (b"proc p {} {{x 0}} {incr x}; set first [p]; proc p {} {{x 10}} {incr x}; list $first [p]", Code::Ok, b"1 11"),
            (b"set a OLD; upvar 0 a x; proc p {} {&x} {set x}; set b NEW; upvar 0 b x; list [p] $x", Code::Ok, b"NEW NEW"),
            (b"set a(k) VALUE; proc p {} {a(k)} {set a(k)}", Code::Error, b"Can't initialise array element \"a(k)\""),
            (b"set a(k) VALUE; proc p {} {&a(k)} {set a(k)}", Code::Error, b"Can't link to array element \"a(k)\""),
            (b"proc p {} {absent} {set absent}", Code::Error, b"variable for initialization of static \"absent\" not found in the local context"),
            (b"set x X; proc p {} {x &x} {set x}", Code::Error, b"static variable name \"x\" duplicated in statics list"),
            (b"proc p {} {{x STATIC}} {unset x; set x NEW; set x}; p", Code::Error, b"can't unset \"x\": no such variable"),
            (b"set x OUTER; proc p {{x DEFAULT}} {&x} {set x}; list [p] $x", Code::Ok, b"DEFAULT DEFAULT"),
            (b"set a(k) VALUE; proc p {} {a} {array get a}; set a(k) NEW; list [p] [array get a]", Code::Ok, b"{k VALUE} {k NEW}"),
            (b"set a(k) OLD; proc p {} {&a} {set a(k)}; unset a; set a(k) NEW; list [p] $a(k)", Code::Ok, b"OLD NEW"),
            (b"proc p {} {{x VALUE}} {rename p {}; set x}; p", Code::Ok, b"VALUE"),
            (b"proc maker {} {set a KEEP;upvar 0 a x;proc p {} {&x} {set x}};maker;p", Code::Error, b"can't read \"x\": no such variable"),
            (b"proc maker {} {set a KEEP;upvar 0 a x;proc p {} {&x} {set x}};maker;proc unrelated {} {set a OTHER;p};unrelated", Code::Ok, b"OTHER"),
            (b"proc maker {} {set a OLD;upvar 0 a x;proc p {} {&x} {set x MODIFIED};proc r {} {&x} {set x}};maker;p;r", Code::Error, b"can't read \"x\": no such variable"),
            (b"set a(k) V;proc p {} {a} {unset a(k);info exists a(k)};list [p] $a(k)", Code::Ok, b"0 V"),
            (b"set a(k) V;proc p {} {&a} {unset a(k);info exists a(k)};list [p] [info exists a(k)]", Code::Ok, b"0 0"),
        ];
        for &(source, code, wanted) in CASES {
            leak_free(|interp| {
                interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
                assert_eq!(
                    interp.eval_str(source),
                    code,
                    "{}",
                    String::from_utf8_lossy(source)
                );
                assert_eq!(
                    interp.result_bytes(),
                    wanted,
                    "{}",
                    String::from_utf8_lossy(source)
                );
            });
        }
    }

    #[test]
    fn proc_basic_args_defaults_and_locals() {
        leak_free(|i| {
            run(i, b"proc pick {a b} {return $b}");
            assert_eq!(run(i, b"pick 1 2"), b"2");
            // default parameter
            run(
                i,
                b"proc greet {name {greeting hello}} {return $name-$greeting}",
            );
            assert_eq!(run(i, b"greet bob"), b"bob-hello");
            assert_eq!(run(i, b"greet bob hi"), b"bob-hi");
            // proc locals don't leak to the caller
            run(i, b"proc uselocal {} {set v 99; return $v}");
            assert_eq!(run(i, b"set v outer"), b"outer");
            assert_eq!(run(i, b"uselocal"), b"99");
            assert_eq!(run(i, b"set v"), b"outer");
            i.eval_str(b"unset v");
        });
    }

    #[test]
    fn proc_parameter_parser_preserves_byte_errors() {
        let params = parse_params(b"a {b {hello world}} args").unwrap();
        assert_eq!(params[1].name, b"b");
        assert_eq!(
            params[1]
                .default
                .as_ref()
                .map(|value| obj_bytes(value.as_ptr())),
            Some(b"hello world".to_vec())
        );
        assert_eq!(
            parse_params(b"{{} default}").err().unwrap(),
            b"argument with no name"
        );
        assert_eq!(
            parse_params(b"{array(index)}").err().unwrap(),
            b"formal parameter \"array(index)\" is an array element"
        );
    }

    #[test]
    fn proc_default_arity_edges_dont_panic() {
        // Defaulted positionals must not panic when fewer args than positionals
        // are supplied (the `args` split) or when a *required* parameter
        // follows a defaulted one (non-trailing default). Matches tclsh 9.0.
        leak_free(|i| {
            // All-defaulted positionals + args, called with none.
            run(i, b"proc q {{a 1} {b 2} args} {list $a $b $args}");
            assert_eq!(run(i, b"q"), b"1 2 {}");
            assert_eq!(run(i, b"q 5"), b"5 2 {}");
            assert_eq!(run(i, b"q 5 6 7 8"), b"5 6 {7 8}");
            // Non-trailing default: a required param after a defaulted one.
            run(i, b"proc p {a {b 2} c} {list $a $b $c}");
            assert_eq!(run(i, b"p 1 2 3"), b"1 2 3");
            assert_eq!(i.eval_str(b"p 1 2"), Code::Error);
            assert_eq!(i.result_bytes(), b"wrong # args: should be \"p a ?b? c\"");
        });
    }

    #[test]
    fn proc_args_catch_all() {
        leak_free(|i| {
            run(i, b"proc va {first args} {return $first|$args}");
            assert_eq!(run(i, b"va 1"), b"1|");
            assert_eq!(run(i, b"va 1 2 3"), b"1|2 3");
        });
    }

    #[test]
    fn proc_wrong_args() {
        leak_free(|i| {
            run(i, b"proc add {a b} {return x}");
            assert_eq!(i.eval_str(b"add 1"), Code::Error);
            assert_eq!(i.result_bytes(), b"wrong # args: should be \"add a b\"");
            assert_eq!(i.eval_str(b"add 1 2 3"), Code::Error);
            assert_eq!(i.result_bytes(), b"wrong # args: should be \"add a b\"");
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn proc_recursion_fib() {
        leak_free(|i| {
            run(
                i,
                b"proc fib {n} {if {$n < 2} {return $n}; return [expr {[fib [expr {$n-1}]] + [fib [expr {$n-2}]]}]}",
            );
            assert_eq!(run(i, b"fib 10"), b"55");
            assert_eq!(run(i, b"fib 0"), b"0");
            assert_eq!(run(i, b"fib 1"), b"1");
        });
    }

    #[test]
    fn unqualified_proc_in_namespace_eval_binds_in_that_ns() {
        // `proc p` inside `namespace eval X` binds ::X::p (not a global) — the
        // command-home-ns fix.
        leak_free(|i| {
            run(i, b"namespace eval X { proc p {} {return inX} }");
            assert_eq!(run(i, b"X::p"), b"inX");
            assert_eq!(run(i, b"namespace eval X { p }"), b"inX");
            // it is NOT a global command.
            assert_eq!(i.eval_str(b"p"), Code::Error);
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn proc_in_namespace_uses_its_ns() {
        leak_free(|i| {
            // a proc defined in ::ctr sees ::ctr's variables via `variable`.
            run(i, b"namespace eval ctr { variable n 0 }");
            run(i, b"proc ctr::bump {} { variable n; incr n; return $n }");
            assert_eq!(run(i, b"ctr::bump"), b"1");
            assert_eq!(run(i, b"ctr::bump"), b"2");
            assert_eq!(run(i, b"set ::ctr::n"), b"2");
            i.eval_str(b"unset ::ctr::n");
        });
    }

    #[test]
    fn apply_lambda() {
        leak_free(|i| {
            assert_eq!(run(i, b"apply {{a b} {return $b}} 1 2"), b"2");
            assert_eq!(run(i, b"apply {{a {b 9}} {return $b}} 1"), b"9");
            assert_eq!(run(i, b"apply {{args} {return $args}} a b c"), b"a b c");
            assert_eq!(i.eval_str(b"apply {{a b} {return x}} 1"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"apply lambdaExpr a b\""
            );
            // a 3-element lambda runs in the named namespace.
            run(i, b"namespace eval foo { variable v 42 }");
            assert_eq!(run(i, b"apply {{} {variable v; return $v} foo}"), b"42");
            i.eval_str(b"unset ::foo::v");
        });
    }

    #[test]
    fn break_continue_escaping_a_proc_is_an_error() {
        leak_free(|i| {
            run(i, b"proc f {} {break}");
            assert_eq!(i.eval_str(b"f"), Code::Error);
            assert_eq!(i.result_bytes(), b"invoked \"break\" outside of a loop");
            run(i, b"proc g {} {continue}");
            assert_eq!(i.eval_str(b"g"), Code::Error);
            assert_eq!(i.result_bytes(), b"invoked \"continue\" outside of a loop");
        });
    }

    #[test]
    fn infinite_recursion_is_caught() {
        // An unbounded proc loop raises a catchable error, not a stack overflow.
        // The tree-walking interpreter recurses on the native stack (~one deep
        // chain per Tcl level), so the 1000-level bound needs a production-sized
        // stack to be *reached* — the default 2 MiB test-thread stack is too
        // small. Run it on a large-stack thread (the main thread / a configured
        // wasm stack have ample room); the leak counters are thread-local, so the
        // check runs inside the spawned thread.
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                leak_free(|i| {
                    run(i, b"proc loop {} {loop}");
                    assert_eq!(i.eval_str(b"loop"), Code::Error);
                    assert_eq!(
                        i.result_bytes(),
                        b"too many nested evaluations (infinite loop?)"
                    );
                });
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn puts_runs() {
        leak_free(|i| {
            // writes to stdout (captured/discarded by the test harness); returns "".
            assert_eq!(i.eval_str(b"puts -nonewline {}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            assert_eq!(i.eval_str(b"puts stdout hello"), Code::Ok);
            assert_eq!(i.eval_str(b"puts nosuchchan x"), Code::Error);
        });
    }
}
