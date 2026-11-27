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

//! Entering a procedure, lambda or `TclOO` method: arity checks, the usage message, the call frame and argument binding.

use super::super::{Frame, OriginalProcedureCallReq};
use crate::command::ProcDef;
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use std::rc::Rc;
use tcl_runtime_api::Completion;
use tcl_syntax::value::ValueOps;

/// The Tcl `wrong # args` usage message for a proc. C's `ProcWrongNumArgs`
/// names the procedure by the word it was invoked as (`objv[0]`), so
/// `::a::b` reports `::a::b x`, not its namespace tail.
fn proc_usage(vm: &mut Vm, proc: &ProcDef, invoked: &Value) -> Result<Vec<u8>, Completion<Value>> {
    let parameters = proc.formal_parameters();
    if let Some(words) = proc.usage_name.as_deref() {
        return procedure_usage_message(vm, &parameters, proc.parameter_grammar, words);
    }
    let called = vm
        .native_name_operand_bytes_checked(invoked)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let protocol = procedure_usage_protocol(vm)?;
    let header = protocol.render_procedure_name(&called, true);
    let suffix = formal_usage_suffix(vm, &parameters, proc.parameter_grammar)?;
    Ok(protocol.render_procedure_message(&header, &suffix))
}

fn procedure_usage_protocol(
    vm: &mut Vm,
) -> Result<tcl_native_model::native_usage::NativeUsageProtocol, Completion<Value>> {
    vm.native_invocation_dialect()
        .usage_protocol(Some(
            tcl_native_model::native_usage::LogicalUsageProvider::Tcl84CoreSimulation,
        ))
        .ok_or_else(|| {
            vm.refuse_host_command("native procedure usage protocol is unavailable".into())
        })
}

fn formal_usage_suffix(
    vm: &mut Vm,
    parameters: &[tcl_syntax::formal_params::ByteFormalParameter],
    grammar: tcl_dialect::ParameterGrammar,
) -> Result<Vec<u8>, Completion<Value>> {
    let activation = tcl_registry::native_procedure::procedure_activation_protocol(
        vm.actual_native_invocation_dialect(),
    )
    .ok_or_else(|| {
        vm.refuse_host_command("native procedure activation protocol is unavailable".into())
    })?;
    Ok(tcl_syntax::formal_params::formal_parameter_usage_bytes(
        parameters,
        grammar,
        activation.variadic_usage_word(),
    ))
}

fn procedure_usage_message(
    vm: &mut Vm,
    parameters: &[tcl_syntax::formal_params::ByteFormalParameter],
    grammar: tcl_dialect::ParameterGrammar,
    words: &[Value],
) -> Result<Vec<u8>, Completion<Value>> {
    let usage = vm.native_argument_usage_header(words)?;
    let suffix = formal_usage_suffix(vm, parameters, grammar)?;
    let protocol = procedure_usage_protocol(vm)?;
    Ok(protocol.render_procedure_message(&usage, &suffix))
}

impl Vm {
    /// Push a call-frame and bind `argv` to the proc's parameters.
    fn procedure_argument_bindings(
        &mut self,
        proc: &ProcDef,
        invoked: &Value,
        argv: &[Value],
    ) -> Result<Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>, Completion<Value>> {
        tcl_syntax::formal_params::bind_formal_argument_bytes(
            &proc.formal_parameters(),
            argv.len(),
            proc.parameter_grammar,
        )
        .map_err(|_| match proc_usage(self, proc, invoked) {
            Ok(message) => crate::command::native_wrong_arguments_message(self, message),
            Err(error) => error,
        })
    }

    /// Native Jim validates arity and handles an empty body before creating
    /// any frame or acquiring a Script activation. C prepares its body first.
    pub(in super::super) fn early_procedure_activation(
        &mut self,
        proc: &ProcDef,
        invoked: &Value,
        argv: &[Value],
    ) -> Result<Option<Completion<Value>>, Completion<Value>> {
        self.early_procedure_activation_inputs(
            &proc.formal_parameters(),
            proc.parameter_grammar,
            &proc.body_src,
            argv,
            |vm| proc_usage(vm, proc, invoked),
        )
    }

    pub(crate) fn preflight_original_lambda_activation(
        &mut self,
        params: &[crate::command::Param],
        grammar: tcl_dialect::ParameterGrammar,
        body: &Value,
        argv: &[Value],
        usage_words: &[Value],
    ) -> Result<Option<Completion<Value>>, Completion<Value>> {
        let parameters = crate::command::formal_parameters(params, grammar);
        self.early_procedure_activation_inputs(&parameters, grammar, body, argv, |vm| {
            procedure_usage_message(vm, &parameters, grammar, usage_words)
        })
    }

    pub(in super::super) fn early_procedure_activation_inputs(
        &mut self,
        parameters: &[tcl_syntax::formal_params::ByteFormalParameter],
        grammar: tcl_dialect::ParameterGrammar,
        body: &Value,
        argv: &[Value],
        usage: impl FnOnce(&mut Self) -> Result<Vec<u8>, Completion<Value>>,
    ) -> Result<Option<Completion<Value>>, Completion<Value>> {
        let Some(protocol) = tcl_registry::native_procedure::procedure_activation_protocol(
            self.actual_native_invocation_dialect(),
        ) else {
            return Err(self.refuse_host_command(
                "native procedure activation protocol is unavailable".into(),
            ));
        };
        if protocol.validates_arguments_before_body() {
            tcl_syntax::formal_params::bind_formal_argument_bytes(parameters, argv.len(), grammar)
                .map_err(|_| match usage(self) {
                    Ok(message) => crate::command::native_wrong_arguments_message(self, message),
                    Err(error) => error,
                })?;
        }
        if protocol.empty_body_skips_activation() {
            let body = self
                .native_name_operand_bytes_checked(body)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            if body.is_empty() {
                return Ok(Some(ok(Value::empty())));
            }
        }
        Ok(None)
    }

    /// Prepare an actual manufactured lambda and retain its original argument objects.
    pub(crate) fn defer_original_lambda_call(
        &mut self,
        definition: crate::command::OriginalLambdaDefinition,
        invoked: Value,
        arguments: &[Value],
    ) -> Completion<Value> {
        if !definition.activation_preflight_complete {
            match self.early_procedure_activation(&definition.binding, &invoked, arguments) {
                Ok(Some(completion)) | Err(completion) => {
                    self.retire_original_lambda_registration(&definition.registration);
                    return completion;
                }
                Ok(None) => {}
            }
        }
        let proc = match self.ensure_proc_ready_in(definition.binding, None, None) {
            Ok(proc) => proc,
            Err(error) => {
                self.retire_original_lambda_registration(&definition.registration);
                return crate::command::completion_from_tcl_error(self, error);
            }
        };
        self.pending.procedure_call = Some(OriginalProcedureCallReq {
            proc,
            invoked,
            arguments: arguments.to_vec(),
            registration: definition.registration,
        });
        ok(Value::empty())
    }

    /// Push a call-frame and bind `argv` to the prepared procedure activation.
    pub(in super::super) fn enter_proc(
        &mut self,
        proc: &Rc<ProcDef>,
        body: &crate::compiled::CompiledUnit,
        invoked: &Value,
        argv: &[Value],
    ) -> Result<(), Completion<Value>> {
        if body.jim_script.is_none() {
            body.asm
                .validate_native_compilation_entry()
                .expect("compiler provider admits native preflight before formal binding");
        }
        if let Some(failure) = &body.asm.native_compilation_failure {
            let name = self
                .native_name_operand_bytes_checked(invoked)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            return Err(self.raise_native_compilation_failure(failure, Some(&name)));
        }
        let Some(protocol) = tcl_registry::native_procedure::procedure_activation_protocol(
            self.actual_native_invocation_dialect(),
        ) else {
            return Err(self.refuse_host_command(
                "native procedure activation protocol is unavailable".into(),
            ));
        };
        if protocol.parse_failure_precedes_arguments()
            && let Some(tail) = body.fatal_tail.clone()
        {
            return Err(self.raise_procedure_parse_failure(tail, invoked));
        }
        let bindings = self.procedure_argument_bindings(proc, invoked, argv)?;
        let skip_bindings = if proc.parameter_grammar == tcl_dialect::ParameterGrammar::Jim {
            self.native_name_operand_bytes_checked(&proc.body_src)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
                .is_empty()
        } else {
            false
        };
        if !skip_bindings && self.recursion_depth() >= self.recursion_limit() {
            return Err(err("too many nested evaluations (infinite loop?)"));
        }
        let call_argv = proc.call_identity.as_ref().map_or_else(
            || {
                let mut words = Vec::with_capacity(argv.len() + 1);
                words.push(invoked.native_lifetime_lease().into_value());
                words.extend(
                    argv.iter()
                        .map(|word| word.native_lifetime_lease().into_value()),
                );
                words
            },
            |words| {
                words
                    .iter()
                    .map(|word| word.native_lifetime_lease().into_value())
                    .collect()
            },
        );
        self.push_call_frame(Some(proc.actual_name()), call_argv);
        if let Some(namespace) = proc.retained_jim_namespace() {
            self.install_jim_procedure_namespace_owner(
                proc.actual_namespace_context().context().resolved_token(),
                namespace,
            );
        }
        if !protocol.retains_prepared_body() {
            self.install_procedure_object_owners(
                proc.body_src.clone(),
                proc.native_parameters.clone(),
            );
        }
        self.install_procedure_statics(proc.retained_statics());

        self.push_procedure_namespace_context(proc.actual_namespace_context());
        if !skip_bindings
            && let Err(error) = body
                .compiled_local_layout
                .as_ref()
                .map_or(Ok(()), |layout| self.install_compiled_local_layout(layout))
                .and_then(|()| self.install_native_variable_name_owners(proc, body))
                .and_then(|()| self.bind_proc_arguments(proc, argv, &bindings))
        {
            self.pop_call_frame();
            self.pop_ns();
            return Err(error);
        }
        self.acquire_native_procedure_execution(proc);
        Ok(())
    }

    fn bind_proc_arguments(
        &mut self,
        proc: &ProcDef,
        argv: &[Value],
        bindings: &[tcl_syntax::formal_params::FormalByteArgumentBinding],
    ) -> Result<(), Completion<Value>> {
        use tcl_syntax::formal_params::FormalByteArgumentBinding;
        for binding in bindings {
            let parameter_slot = match binding {
                FormalByteArgumentBinding::Value { parameter, .. }
                | FormalByteArgumentBinding::Default { parameter }
                | FormalByteArgumentBinding::Rest { parameter, .. }
                | FormalByteArgumentBinding::CallerLink { parameter, .. } => *parameter,
            };
            let (name, value) = match binding {
                FormalByteArgumentBinding::Value {
                    parameter,
                    argument,
                } => (
                    proc.params[*parameter].name.as_bytes(),
                    argv[*argument].clone(),
                ),
                FormalByteArgumentBinding::Default { parameter } => {
                    let parameter = &proc.params[*parameter];
                    (
                        parameter.name.as_bytes(),
                        parameter.default.clone().expect("planned default"),
                    )
                }
                FormalByteArgumentBinding::Rest {
                    name, start, len, ..
                } => (
                    name.as_slice(),
                    Value::list(argv[*start..*start + *len].to_vec()),
                ),
                FormalByteArgumentBinding::CallerLink { name, argument, .. } => {
                    let target = self
                        .native_name_operand_bytes_checked(&argv[*argument])
                        .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                    let caller = self.current_level().saturating_sub(1);
                    if self.get_var_from_bytes(caller, &target).is_none() {
                        return Err(self.read_miss_completion_bytes_from(caller, &target));
                    }
                    self.link_upvar_original(
                        caller,
                        &argv[*argument],
                        &Value::new_native_string_bytes(name.as_slice()),
                    )?;
                    continue;
                }
            };
            match proc.parameter_grammar {
                tcl_dialect::ParameterGrammar::Tcl => {
                    self.bind_compiled_formal_slot(parameter_slot, value)?;
                }
                tcl_dialect::ParameterGrammar::Jim => self.set_var_bytes(name, value)?,
            }
        }
        Ok(())
    }

    /// Run a `TclOO` method body: enter the proc activation, link the object's
    /// instance variables into the fresh frame, push the OO call context, run
    /// the body to completion, then pop the context. This is the OO analogue of
    /// [`invoke_command`](Self::invoke_command)'s `Proc` arm plus the
    /// instance-variable linking the runtime's `run_proc` does.
    ///
    /// `link_vars` retain exact local and object-namespace storage keys. The
    /// actual procedure namespace token selects their physical table. The `frame` carries
    /// the resolved method chain that `self`/`my`/
    /// `next` consult while the body runs.
    pub(crate) fn oo_run_method(
        &mut self,
        activation: Box<crate::command::PreparedProcedureActivation>,
        argv: &[Value],
        link_vars: &[(tcl_core_types::NameBytes, tcl_core_types::NameBytes)],
        frame: crate::cmd_oo::OoFrame,
    ) -> Completion<Value> {
        let mut active = crate::interp::oo_cleanup::OriginalOoActivation::enter(self);
        // The method body recurses natively through this frame, so its entry
        // work runs in `enter_oo_method`'s frame, which has returned by then.
        let body = match self.enter_oo_method(&mut active, activation, argv, link_vars, frame) {
            Ok(body) => body,
            Err(result) => return active.finish(self, result),
        };
        let result = self.run_activation(body);
        active.retire_call(self, true);
        active.retire_namespace(self, true);
        active.retire_method(self);
        active.finish(self, result)
    }

    /// [`Self::oo_run_method`]'s entry: enter the procedure frame, link the
    /// instance variables and push the OO call context. Returns the body's
    /// activation, or the completion `active` must finish with.
    #[inline(never)]
    fn enter_oo_method(
        &mut self,
        active: &mut crate::interp::oo_cleanup::OriginalOoActivation,
        activation: Box<crate::command::PreparedProcedureActivation>,
        argv: &[Value],
        link_vars: &[(tcl_core_types::NameBytes, tcl_core_types::NameBytes)],
        frame: crate::cmd_oo::OoFrame,
    ) -> Result<Box<Frame>, Completion<Value>> {
        let activation = *activation;
        let proc = &activation.proc;
        let invoked =
            Value::from_native_string_bytes(proc.actual_command_slot().simple.as_bytes().to_vec());
        self.enter_proc(proc, &activation.body, &invoked, argv)?;
        if let Err(error) = active.capture_call_and_namespace(self) {
            crate::interp::oo_cleanup::retain_error(self, &error);
            active.retire_call(self, false);
            active.retire_namespace(self, false);
            let result = crate::command::completion_from_cmd_error(self, error);
            return Err(result);
        }
        if let Some(layout) = &activation.body.compiled_local_layout {
            let protocol = self
                .name_policy_protocol()
                .expect("entered method naming protocol")
                .recipe();
            for (slot, primary) in layout.names.iter().enumerate() {
                let Some(primary) = primary else { continue };
                for (local, storage) in link_vars {
                    let matches = tcl_syntax::naming::native_oo_variable_resolver_matches(
                        protocol,
                        tcl_syntax::naming::NativeOoVariableResolverPurpose::CompiledPrimary,
                        local.as_bytes(),
                        primary.as_bytes(),
                    );
                    match matches {
                        Ok(false) => continue,
                        Ok(true) => {}
                        Err(error) => {
                            let result = self.refuse_host_command(format!(
                                "TclOO compiled variable resolver: {error:?}"
                            ));
                            active.retire_call(self, false);
                            active.retire_namespace(self, false);
                            return Err(result);
                        }
                    }
                    if let Err(error) = self.add_tcloo_compiled_instance_link(
                        slot,
                        proc.actual_namespace_id(),
                        storage.as_bytes(),
                    ) {
                        let result = crate::command::upvar_link_error_bytes(
                            error,
                            storage.as_bytes(),
                            primary.as_bytes(),
                        );
                        active.retire_call(self, false);
                        active.retire_namespace(self, false);
                        return Err(result);
                    }
                    break;
                }
            }
        }
        self.install_native_procedure_binding(activation.declaration_binding);
        let mut frame = frame;
        frame.activation = Some(self.native_oo_variable_activation());
        if let Err(error) = active.push_method(self, frame) {
            crate::interp::oo_cleanup::retain_error(self, &error);
            active.retire_call(self, false);
            active.retire_namespace(self, false);
            let result = crate::command::completion_from_cmd_error(self, error);
            return Err(result);
        }
        let mut body = Box::new(Frame::new(activation.body, true));
        body.oo_call_exit = active.saved_call_exit();
        Ok(body)
    }
}
