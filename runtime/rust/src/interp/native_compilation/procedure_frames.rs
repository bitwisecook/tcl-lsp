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

//! Entering a procedure through its compiled body: the frame, the formal parameters and the admission that keeps the body active.

use super::super::{
    drop_fresh, obj, obj_bytes, CallMeta, CmdFrame, Code, FrameKind, Interp, MethodFrameWhat, NsId,
    Param, ProcFrame, Rc, TclObj,
};
use super::{direct_activation, CompiledChunk, NativeCompilerAdmissionFailure, ScriptActivation};
use tcl_registry::native_compilation::{
    NativeCompilationContext as Context, NativeCompilationFrame, NativeCompilationMode,
};

impl Interp {
    pub(in super::super) fn enter_native_procedure_frame<O: obj::ObjectPointer>(
        &mut self,
        params: &[Param<O>],
        call_args: &[*mut TclObj],
        usage_called: &[u8],
        context: (NsId, &mut CallMeta<'_>),
        bindings: Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>,
        compiled_layout: Option<&tcl_runtime_api::native_compilation::NativeCompiledLocalLayout>,
    ) -> Result<(NsId, Box<CmdFrame>), Code> {
        use tcl_syntax::formal_params::FormalByteArgumentBinding as Binding;
        let (ns, meta) = context;
        let caller_level = self.current_level();
        // Recursion bound (catchable, not a stack overflow).
        if self.recursion_depth.get() >= self.recursion_limit.get() {
            return Err(self.error(b"too many nested evaluations (infinite loop?)"));
        }
        self.recursion_depth.set(self.recursion_depth.get() + 1);

        self.enter_namespace_activation(ns);
        if meta.same_level {
            self.frames.borrow_mut().push_same_level(ns);
        } else {
            self.frames.borrow_mut().push(ns);
        }
        if self.native_invocation_dialect().family() == Some(tcl_dialect::model::Family::Tcl) {
            if let Some(procedure) = meta.c_procedure {
                self.frames
                    .borrow_mut()
                    .retain_c_procedure(Rc::clone(procedure));
            }
        }
        if let Some(argv) =
            CallMeta::original_frame_arguments(self.native_invocation_dialect(), meta.original_argv)
        {
            let installed = self
                .frames
                .borrow_mut()
                .install_original_error_stack_argv(argv);
            if let Err(error) = installed {
                let popped = self.pop_native_call_frame();
                self.leave_namespace_activation(popped);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return Err(self.report_cmd_error(error.into()));
            }
        }
        if self.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            let namespace = match meta.jim_namespace {
                Some(original) => Some(Rc::new(obj::Owned::retain(original.as_ptr()))),
                None => self.namespaces.borrow_mut().take_jim_namespace_owner(ns),
            };
            let Some(namespace) = namespace else {
                let popped = self.pop_native_call_frame();
                self.leave_namespace_activation(popped);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return Err(self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim procedure frame namespace",
                    )
                    .into(),
                ));
            };
            let registered = crate::namespace::Namespaces::remember_jim_namespace_owner(
                &mut self.namespaces.borrow_mut(),
                ns,
                &namespace,
            );
            if let Err(error) = registered {
                let popped = self.pop_native_call_frame();
                self.leave_namespace_activation(popped);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return Err(self.report_cmd_error(error.into()));
            }
            self.frames.borrow_mut().retain_jim_activation_objects(
                meta.jim_parameters.map(obj::Owned::retain),
                meta.jim_body.map(obj::Owned::retain),
                namespace,
            );
            if let Some(procedure) = meta.c_procedure {
                let observed = self
                    .frames
                    .borrow_mut()
                    .observe_original_jim_procedure(procedure);
                if let Err(error) = observed {
                    let popped = self.pop_native_call_frame();
                    self.leave_namespace_activation(popped);
                    self.recursion_depth.set(self.recursion_depth.get() - 1);
                    return Err(self.report_cmd_error(error.into()));
                }
            }
        }
        if let Some(statics) = meta.statics.take() {
            let activation_level = self.current_level();
            self.frames
                .borrow_mut()
                .table_mut(activation_level)
                .expect("new procedure frame")
                .install_statics(statics);
        }
        // Record the invocation words for `info level N`: an OO constructor
        // supplies the `create`/`new` invocation words verbatim; otherwise the
        // invoked name plus the supplied arguments.
        let words = if CallMeta::original_frame_arguments(
            self.native_invocation_dialect(),
            meta.original_argv,
        )
        .is_some()
        {
            // Native C and Jim borrow their actual invocation objects without
            // getters merely to install the frame's invocation transport.
            Vec::new()
        } else {
            meta.level_words.take().unwrap_or_else(|| {
                let mut words = Vec::with_capacity(call_args.len() + 1);
                words.push(usage_called.to_vec());
                words.extend(call_args.iter().map(|&arg| obj_bytes(arg)));
                words
            })
        };
        self.frames.borrow_mut().set_words(words);
        let saved_ns = self.current_ns.get();
        self.current_ns.set(ns);

        let indexed = self
            .native_invocation_dialect()
            .native_compiled_variable_protocol()
            .filter(|protocol| protocol.has_indexed_locals());
        if let Some(protocol) = indexed {
            let names: Vec<_> = params
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect();
            self.frames
                .borrow_mut()
                .install_formal_cells(&names, protocol);
        }
        if let Some(layout) = compiled_layout {
            let installed = self
                .frames
                .borrow_mut()
                .install_native_compiled_local_layout(layout);
            if let Err(error) = installed {
                let popped = self.pop_native_call_frame();
                self.current_ns.set(saved_ns);
                self.leave_namespace_activation(popped);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return Err(crate::builtins::var_error(self, usage_called, error));
            }
        }
        if let Err(error) = self.refresh_native_local_name_table() {
            let popped = self.pop_native_call_frame();
            self.current_ns.set(saved_ns);
            self.leave_namespace_activation(popped);
            self.recursion_depth.set(self.recursion_depth.get() - 1);
            return Err(self.report_cmd_error(error));
        }

        if let Some(resolver) = meta.oo_variable_resolver.take() {
            let installed = self
                .frames
                .borrow_mut()
                .install_tcloo_variable_resolver(resolver);
            if let Err(error) = installed {
                let popped = self.pop_native_call_frame();
                self.current_ns.set(saved_ns);
                self.leave_namespace_activation(popped);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return Err(crate::builtins::var_error(self, usage_called, error));
            }
        }
        if let Some(layout) = compiled_layout {
            let protocol = match self.require_variable_name_protocol() {
                Ok(protocol) => protocol,
                Err(error) => {
                    let popped = self.pop_native_call_frame();
                    self.current_ns.set(saved_ns);
                    self.leave_namespace_activation(popped);
                    self.recursion_depth.set(self.recursion_depth.get() - 1);
                    return Err(crate::builtins::var_error(self, usage_called, error));
                }
            };
            // C compiled-local initialisers exclude arguments and temporaries:
            // TclInitCompiledLocals in C8, InitResolvedLocals in C9. Actual
            // formal slots keep their local cells; retained layout None entries
            // independently exclude unnamed temporaries.
            // naming.procedure.compiled-local-resolver-formal-exclusion
            // docs/design/analysis/name-resolution-proofs/procedure-compiled-local-resolver-formal-exclusion.md
            for (slot, primary) in layout.names.iter().enumerate().skip(params.len()) {
                let Some(primary) = primary else { continue };
                for (local, target) in meta.link_vars {
                    let selected = tcl_syntax::naming::native_oo_variable_resolver_matches(
                        protocol,
                        tcl_syntax::naming::NativeOoVariableResolverPurpose::CompiledPrimary,
                        local,
                        primary.as_bytes(),
                    );
                    match selected {
                        Ok(false) => continue,
                        Ok(true) => {}
                        Err(_) => {
                            let popped = self.pop_native_call_frame();
                            self.current_ns.set(saved_ns);
                            self.leave_namespace_activation(popped);
                            self.recursion_depth.set(self.recursion_depth.get() - 1);
                            return Err(self.report_cmd_error(
                                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                    "TclOO compiled variable resolver",
                                )
                                .into(),
                            ));
                        }
                    }
                    let bound =
                        self.link_original_tcloo_compiled_namespace_variable(ns, slot, target);
                    if bound != Code::Ok {
                        let popped = self.pop_native_call_frame();
                        self.current_ns.set(saved_ns);
                        self.leave_namespace_activation(popped);
                        self.recursion_depth.set(self.recursion_depth.get() - 1);
                        return Err(bound);
                    }
                    break;
                }
            }
        }

        for binding in bindings {
            let stored = match binding {
                Binding::Value {
                    parameter,
                    argument,
                } => (if indexed.is_some() {
                    self.store_native_formal_cell_checked(parameter, call_args[argument])
                } else {
                    self.var_set_named(&params[parameter].name, call_args[argument])
                })
                .map_err(|_| {
                    if self.native_execution_refusal().is_none() {
                        self.set_error(b"proc parameter binding failed");
                    }
                }),
                Binding::Default { parameter } => {
                    let value = params[parameter].default.as_ref().unwrap().as_ptr();
                    if let Err(error) = obj::check_native_liveness(value) {
                        let popped = self.pop_native_call_frame();
                        self.current_ns.set(saved_ns);
                        self.leave_namespace_activation(popped);
                        self.recursion_depth.set(self.recursion_depth.get() - 1);
                        return Err(self.report_cmd_error(error.into()));
                    }
                    let result = if indexed.is_some() {
                        self.store_native_formal_cell_checked(parameter, value)
                    } else {
                        self.var_set_named(&params[parameter].name, value)
                    };
                    result.map_err(|_| {
                        if self.native_execution_refusal().is_none() {
                            self.set_error(b"proc parameter binding failed");
                        }
                    })
                }
                Binding::Rest {
                    parameter,
                    name,
                    start,
                    len,
                } => {
                    let value = self.new_list_object(&call_args[start..start + len]);
                    let original = if self.invocation_string_protocol()
                        == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
                    {
                        match crate::frame::OriginalJimVariadic::capture(value) {
                            Ok(original) => Some(original),
                            Err(error) => {
                                self.report_cmd_error(error.into());
                                drop_fresh(value);
                                let popped = self.pop_native_call_frame();
                                self.current_ns.set(saved_ns);
                                self.leave_namespace_activation(popped);
                                self.recursion_depth.set(self.recursion_depth.get() - 1);
                                return Err(Code::Error);
                            }
                        }
                    } else {
                        None
                    };
                    let result = if indexed.is_some() {
                        self.store_native_formal_cell_checked(parameter, value)
                    } else {
                        self.var_set(&name, value)
                    };
                    if result.is_err() {
                        drop_fresh(value);
                    } else if let Some(original) = original {
                        let observed = self
                            .frames
                            .borrow_mut()
                            .observe_original_jim_variadic(original);
                        if let Err(error) = observed {
                            self.report_cmd_error(error.into());
                            let popped = self.pop_native_call_frame();
                            self.current_ns.set(saved_ns);
                            self.leave_namespace_activation(popped);
                            self.recursion_depth.set(self.recursion_depth.get() - 1);
                            return Err(Code::Error);
                        }
                    }
                    result.map_err(|_| {
                        if self.native_execution_refusal().is_none() {
                            self.set_error(b"proc parameter binding failed");
                        }
                    })
                }
                Binding::CallerLink { name, argument, .. } => {
                    let target = obj_bytes(call_args[argument]);
                    let exists = self.var_exists_at(&target, caller_level);
                    if self.native_execution_refusal().is_some() {
                        Err(())
                    } else if !exists {
                        let mut message = b"can't read \"".to_vec();
                        message.extend_from_slice(&target);
                        message.extend_from_slice(b"\": no such variable");
                        self.set_error(&message);
                        Err(())
                    } else if crate::cmd_var::bind_upvar_at(self, caller_level, &target, &name)
                        == Code::Ok
                    {
                        Ok(())
                    } else {
                        Err(())
                    }
                }
            };
            if stored.is_err() {
                let popped = self.pop_native_call_frame();
                self.current_ns.set(saved_ns);
                self.leave_namespace_activation(popped);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return Err(Code::Error);
            }
        }

        // The proc body runs as its own `info frame` level: `type proc` (or
        // `source` if defined in a sourced file), the proc FQN, and the new call
        // level (set after `frames.push`, so `current_level` is the proc's).
        // A TclOO method body carries its method context for `info frame`
        // (`method`/`class`|`object`), which displaces the `proc` key.
        let oo = match &meta.err {
            ProcFrame::Method { kind, owner, what } => {
                let method = match what {
                    MethodFrameWhat::Named(n) => n.to_vec(),
                    MethodFrameWhat::Constructor | MethodFrameWhat::Destructor => Vec::new(),
                };
                Some((method, kind.to_vec(), owner.to_vec()))
            }
            _ => None,
        };
        // An `apply` lambda reports `lambda <expr>` (not `proc`) in `info frame`.
        let lambda = match &meta.err {
            ProcFrame::Lambda(expr) => Some(expr.to_vec()),
            _ => None,
        };
        let (proc_lvl, proc_idx) = {
            let f = self.frames.borrow();
            (f.current_level(), f.current_frame_index())
        };
        let proc_frame = CmdFrame {
            kind: if meta.source.is_some() {
                FrameKind::Source
            } else {
                FrameKind::Proc
            },
            file: meta.source.take(),
            proc: meta.fqn.map(<[u8]>::to_vec),
            level: proc_lvl,
            omit_level: false,
            frame_index: proc_idx,
            line_base: meta.body_line_base,
            proc_line_base: meta.body_line_base,
            cmd: Vec::new(),
            original_command: None,
            line: 1,
            oo,
            lambda,
        };
        Ok((saved_ns, Box::new(proc_frame)))
    }

    pub(in super::super) fn prepare_native_procedure(
        &self,
        source: &[u8],
        namespace: NsId,
    ) -> Result<Box<NativeProcedureAdmission>, NativeCompilerAdmissionFailure> {
        let context =
            if self.native_invocation_dialect().family() == Some(tcl_dialect::model::Family::Tcl) {
                Context {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ProcedureCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                }
            } else {
                direct_activation().context
            };
        let mut chunk = CompiledChunk::default();
        if self.native_invocation_dialect().tcl_version == Some(tcl_dialect::TclVersion::V8_4) {
            if let Some(failure) =
                self.compile_native_script(source, namespace, context, &mut chunk, 0)
            {
                return Err(NativeCompilerAdmissionFailure::Presented(Box::new(failure)));
            }
        }
        if chunk.unpresented_failure {
            return Err(NativeCompilerAdmissionFailure::Unpresented);
        }
        Ok(Box::new(NativeProcedureAdmission {
            interpreter: self.clone(),
            activation: Some(ScriptActivation {
                context,
                chunk: Rc::new(chunk),
            }),
            active: false,
        }))
    }

    pub(in super::super) fn release_native_procedure(&self) {
        self.native_compilation.borrow_mut().pending.pop();
    }
}

pub(in super::super) fn formal_parameters<O: obj::ObjectPointer>(
    parameters: &[Param<O>],
    grammar: tcl_dialect::ParameterGrammar,
) -> Vec<tcl_syntax::formal_params::ByteFormalParameter> {
    parameters
        .iter()
        .map(|parameter| tcl_syntax::formal_params::ByteFormalParameter {
            name: parameter.name.clone(),
            default: parameter.default.as_ref().map(|value| {
                if grammar == tcl_dialect::ParameterGrammar::Jim && parameter.name == b"args" {
                    obj_bytes(value.as_ptr())
                } else {
                    Vec::new()
                }
            }),
        })
        .collect()
}

pub(in super::super) fn native_parameter_plan<O: obj::ObjectPointer>(
    parameters: &[Param<O>],
    argument_count: usize,
    grammar: tcl_dialect::ParameterGrammar,
) -> Result<
    Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>,
    tcl_syntax::formal_params::FormalArityError,
> {
    tcl_syntax::formal_params::bind_formal_argument_bytes(
        &formal_parameters(parameters, grammar),
        argument_count,
        grammar,
    )
}

pub(in super::super) struct NativeProcedureAdmission {
    pub(super) interpreter: Interp,
    activation: Option<ScriptActivation>,
    active: bool,
}

impl NativeProcedureAdmission {
    pub(in super::super) fn activate(&mut self) {
        self.interpreter
            .native_compilation
            .borrow_mut()
            .pending
            .push(self.activation.take());
        self.active = true;
    }
}

impl Drop for NativeProcedureAdmission {
    fn drop(&mut self) {
        if self.active {
            self.interpreter.release_native_procedure();
        }
    }
}
