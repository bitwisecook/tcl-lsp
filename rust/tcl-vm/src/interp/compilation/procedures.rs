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

//! Procedure bodies: capturing the declaration body, compiling it with its entry and command plan, merging compiled procedures, and making a procedure ready to run.

use super::super::commands::CommandSidecarKey;
use super::super::{
    CommandSidecarHandle, NamespacePath, ProcedureBodySource, ProcedureCompilationInputs,
    ProcedureCompilationOwner, ProcedureCompilationTarget, Vm, namespace_context,
    native_name_consumers, native_numeric_ops,
};
use crate::command::{Command, ProcDef};
use crate::compiled::{CompiledUnit, CompilerProvenance};
use crate::error::TclError;
use crate::value::Value;
use std::rc::Rc;
use tcl_bytecode::ModuleAsm;
use tcl_runtime_api::{CompileService, FatalTail, NsId, ProcedureDispatch};

impl<'a> ProcedureCompilationInputs<'a> {
    pub(in super::super) fn new(
        source: ProcedureBodySource<'_>,
        parameters: &[crate::command::Param],
        namespace: &'a NamespacePath,
        complete_prefix_len: usize,
        issuer: &tcl_runtime_api::native_compiler_pass::NativeCompilerPassOwner,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
        activation: Option<tcl_registry::native_procedure::NativeProcedureActivationProtocol>,
    ) -> Result<Self, tcl_runtime_api::native_procedure_compilation::OriginalProcedureCompileError>
    {
        use tcl_runtime_api::native_procedure_compilation::OriginalProcedureCompileError;
        let original = match source.declaration.map(|declaration| {
            declaration.original_compilation_request(
                source.image,
                namespace,
                complete_prefix_len,
                issuer,
                interpreter,
            )
        }) {
            Some(Ok(request)) => Some(match activation {
                Some(tcl_registry::native_procedure::NativeProcedureActivationProtocol::C(
                    version,
                )) => request.with_ordinary_frame_initialization(issuer, version),
                _ => request,
            }),
            Some(Err(OriginalProcedureCompileError::ChangedOriginal)) | None => None,
            Some(Err(error)) => return Err(error),
        };
        let prefix = original.as_ref().map_or_else(
            || {
                tcl_runtime_api::SourceImage::from_bytes(
                    &source.image.bytes()[..complete_prefix_len],
                    source.image.channel(),
                )
            },
            |request| request.target().source.clone(),
        );
        Ok(Self {
            prefix,
            parameters: parameters
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect(),
            namespace,
            original,
        })
    }

    pub(in super::super) fn target(&self) -> ProcedureCompilationTarget<'_> {
        ProcedureCompilationTarget {
            ordinary: tcl_runtime_api::ProcedureCompileTargetBytes {
                source: &self.prefix,
                parameters: &self.parameters,
                namespace: self.namespace,
            },
            original: self.original.as_ref(),
        }
    }
}

impl ProcedureCompilationTarget<'_> {
    pub(in super::super) fn target(&self) -> tcl_runtime_api::ProcedureCompileTargetBytes<'_> {
        self.original
            .map_or(self.ordinary, |request| request.target())
    }
}

impl Vm {
    #[cfg(test)]
    /// Compile original procedure source and formal storage in an actual namespace.
    /// Capture the native definition owner before acquiring any working clone.
    /// C copies shared body bytes into a fresh string; Jim retains the original.
    pub(crate) fn capture_procedure_body_value(
        &mut self,
        original: &Value,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        let scope = native_name_consumers::OriginalNameConsumerScope::enter(self)?;
        let reached = self.capture_procedure_body_value_in_original_name(original, &scope);
        let reached = self.original_name_outcome(&scope, reached);
        self.original_name_finish(scope, reached)?
    }

    fn capture_procedure_body_value_in_original_name(
        &mut self,
        original: &Value,
        scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        use tcl_registry::native_procedure_body::NativeProcedureBodyAction;
        let ops = native_numeric_ops::OriginalNumericOps::in_original_scope(
            self,
            scope.operation_scope(),
        )?;
        ops.current_for_value(original)?;
        let creation = ops
            .dialect()
            .native_procedure_body_creation_protocol()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native procedure body capture",
            ))?;
        let shared = original.native_object_is_shared();
        if creation.action(shared) == NativeProcedureBodyAction::RetainOriginal {
            let captured = original.clone();
            ops.current_for_value(original)?;
            return Ok(captured);
        }
        let captured = Value::from_native_string_bytes(ops.string(original, creation.strings())?);
        if let Some(location) = original.source_location() {
            captured.retain_source_location(location);
        }
        ops.current_for_value(original)?;
        Ok(captured)
    }

    /// Select the declaration producer order without acquiring formal children.
    pub(crate) fn capture_procedure_declaration_body(
        &mut self,
        parameters: &Value,
        body: &Value,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        let scope = native_name_consumers::OriginalNameConsumerScope::enter(self)?;
        let reached =
            self.capture_procedure_declaration_body_in_original_name(parameters, body, &scope);
        let reached = self.original_name_outcome(&scope, reached);
        self.original_name_finish(scope, reached)?
    }

    fn capture_procedure_declaration_body_in_original_name(
        &mut self,
        parameters: &Value,
        body: &Value,
        scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        let ops = native_numeric_ops::OriginalNumericOps::in_original_scope(
            self,
            scope.operation_scope(),
        )?;
        ops.current_for_value(body)?;
        let dialect = ops.dialect();
        let activation = tcl_registry::native_procedure::procedure_activation_protocol(dialect)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native procedure definition",
            ))?;
        if activation.formal_list_precedes_body_capture() {
            let protocol = ops.object_protocol().ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native procedure formal list",
                ),
            )?;
            let reached = ops.list(parameters, protocol);
            ops.current_for_value(body)?;
            let members = reached?;
            drop(members);
            ops.current_for_value(body)?;
        }
        drop(ops);
        self.capture_procedure_body_value_in_original_name(body, scope)
    }

    pub(in super::super) fn original_name_finish<T>(
        &mut self,
        scope: native_name_consumers::OriginalNameConsumerScope,
        reached: T,
    ) -> Result<T, tcl_cmd_core::CmdError> {
        let operation = scope.operation_scope().clone();
        let checked = scope.finish(self).and_then(|()| operation.check(self));
        if let Err(first) = checked {
            drop(reached);
            return Err(operation.check(self).err().unwrap_or(first));
        }
        Ok(reached)
    }

    pub(in super::super) fn original_name_outcome<T>(
        &mut self,
        scope: &native_name_consumers::OriginalNameConsumerScope,
        reached: Result<T, tcl_cmd_core::CmdError>,
    ) -> Result<T, tcl_cmd_core::CmdError> {
        if let Err(error) = &reached
            && let Some(first) = error.native_execution_refusal()
        {
            self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first.clone()));
        }
        if let Err(first) = scope.check(self) {
            drop(reached);
            return Err(scope.check(self).err().unwrap_or(first));
        }
        reached
    }

    pub(in super::super) fn original_name_cmd_error(
        &mut self,
        error: tcl_cmd_core::CmdError,
    ) -> TclError {
        let completion = crate::command::completion_from_cmd_error(self, error);
        match self.execution_refusal.clone() {
            Some(first) => TclError::from_execution_failure(first),
            None => TclError::Guest(completion),
        }
    }

    fn prepare_procedure_body_value_with_owner(
        &mut self,
        admission: Option<(&str, &str)>,
        parameters: &[crate::command::Param],
        namespace: NsId,
        original: &Value,
        owner: Option<ProcedureCompilationOwner<'_>>,
    ) -> Result<CompiledUnit, TclError> {
        let scope = native_name_consumers::OriginalNameConsumerScope::enter(self)
            .map_err(|error| self.original_name_cmd_error(error))?;
        let reached = self.prepare_procedure_body_value_with_owner_in_original_name(
            admission, parameters, namespace, original, owner, &scope,
        );
        self.original_name_finish(scope, reached)
            .map_err(|error| self.original_name_cmd_error(error))?
    }

    fn original_name_tcl_outcome<T>(
        &mut self,
        scope: &native_name_consumers::OriginalNameConsumerScope,
        reached: Result<T, TclError>,
    ) -> Result<T, TclError> {
        if let Err(TclError::Host(first)) = &reached {
            self.retain_tcl_host_failure(first.clone());
        }
        if let Err(first) = scope.check(self) {
            drop(reached);
            let first = scope.check(self).err().unwrap_or(first);
            return Err(self.original_name_cmd_error(first));
        }
        reached
    }

    fn prepare_procedure_body_value_with_owner_in_original_name(
        &mut self,
        admission: Option<(&str, &str)>,
        parameters: &[crate::command::Param],
        namespace: NsId,
        original: &Value,
        owner: Option<ProcedureCompilationOwner<'_>>,
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<CompiledUnit, TclError> {
        original_name_scope
            .check(self)
            .map_err(|error| self.original_name_cmd_error(error))?;

        if self.actual_native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            let reached = self.prepare_native_jim_script_unit(
                original,
                namespace_context::NamespaceExecutionContext::Resolved(namespace),
            );
            return self.original_name_tcl_outcome(original_name_scope, reached);
        }
        let bytes = self
            .native_name_operand_bytes_checked(original)
            .map_err(|error| self.original_name_cmd_error(error))?;
        let source = tcl_runtime_api::SourceImage::native(bytes.as_ref());
        let path = self.ns_path(namespace);
        let mut entry = self.native_compilation_entry_for_namespace_token(namespace, true);
        let procedure = owner
            .and_then(|owner| owner.command_namespace)
            .and_then(|token| {
                let namespace = entry.retained_namespace_context(u64::from(token.0)).ok()?;
                let name = tcl_syntax::naming::native_namespace_full_name_bytes(&namespace.path);
                Some(
                    tcl_runtime_api::native_compiler_pass::NativeCompilerPassProcedure {
                        namespace,
                        namespace_full_name: tcl_runtime_api::NameBytes::from(name),
                    },
                )
            });
        entry.compiler_pass_environment = self.capture_native_compiler_pass_environment(procedure);
        let declaration = owner
            .filter(|owner| {
                std::ptr::eq(parameters, owner.declaration.params.as_slice())
                    && std::ptr::eq(original, &raw const owner.declaration.body_src)
                    && namespace == owner.declaration.actual_namespace_id()
            })
            .map(|owner| owner.declaration);
        let reached = self.prepare_procedure_body_with_entry(
            admission,
            parameters,
            &path,
            ProcedureBodySource {
                image: &source,
                declaration,
            },
            &entry,
            original_name_scope,
        );
        let mut unit = self.original_name_tcl_outcome(original_name_scope, reached)?;
        let actual = self.actual_native_invocation_dialect();
        let protocol = actual.native_bytecode_storage_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original C procedure bytecode issuer",
            ),
        )?;
        if unit.fatal_tail.is_none() || protocol.recipe().retains_parse_failure() {
            let strings = actual.native_string_protocol().ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original C procedure source",
                ),
            )?;
            let reached = unit
                .finalize_original_source_pool(original, strings)
                .map_err(TclError::from);
            unit = self.original_name_tcl_outcome(original_name_scope, reached)?;
        }
        Ok(unit)
    }

    fn validate_procedure_plain_dispatch(
        module: &ModuleAsm,
        dispatch: ProcedureDispatch,
    ) -> Result<(), TclError> {
        if (dispatch == ProcedureDispatch::Plain || module.top_level.plain_command_dispatch)
            && (!module.plain_command_dispatch
                || !module.top_level.plain_command_dispatch
                || !module.top_level.command_bindings.is_empty()
                || !module.top_level.procedure_bindings.is_empty()
                || module.procedures.values().any(|asm| {
                    !asm.plain_command_dispatch
                        || !asm.command_bindings.is_empty()
                        || !asm.procedure_bindings.is_empty()
                }))
        {
            return Err(TclError::new(
                "CompileService procedure plain-dispatch capability returned optimised bytecode",
            ));
        }
        Ok(())
    }

    fn procedure_command_plan(
        &mut self,
        compiler: &dyn CompileService<Module = ModuleAsm>,
        source: &tcl_runtime_api::SourceImage,
        namespace: &NamespacePath,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, TclError> {
        use tcl_runtime_api::NativeCompilationAdmissionScope::ProcedureBody;
        let reached =
            compiler.script_command_plan_bytes_with_entry(source, self.source_profile(), entry);
        original_name_scope
            .check(self)
            .map_err(|error| self.original_name_cmd_error(error))?;
        let reported = reached.map_err(|error| {
            self.compile_service_error_bytes(error, source, namespace, ProcedureBody)
        });
        let plan = self.original_name_tcl_outcome(original_name_scope, reported)?;
        if plan.complete_prefix_len > source.len() {
            return Err(self.compile_service_error_bytes(
                tcl_runtime_api::CompileError::Unsupported(
                    "CompileService returned an invalid procedure byte boundary".into(),
                ),
                source,
                namespace,
                ProcedureBody,
            ));
        }
        Ok(plan)
    }

    fn finish_procedure_unit(
        &mut self,
        module: ModuleAsm,
        dispatch: ProcedureDispatch,
        fatal_tail: Option<FatalTail>,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<CompiledUnit, TclError> {
        Self::validate_procedure_plain_dispatch(&module, dispatch)?;
        self.merge_procs(&module);
        original_name_scope
            .check(self)
            .map_err(|error| self.original_name_cmd_error(error))?;
        let mut unit = self
            .compiled_unit(Rc::new(module.top_level), module.source_namespace)
            .with_manifest(module.manifest)
            .with_fatal_tail(fatal_tail);
        original_name_scope
            .check(self)
            .map_err(|error| self.original_name_cmd_error(error))?;
        if let Ok(namespace) = u32::try_from(entry.current_namespace) {
            unit.native_cache = Some(self.native_cache_stamp(NsId(namespace)));
        }
        Ok(unit)
    }

    /// The dispatch `module` was compiled with: plain when the compile fell
    /// back to it ([`super::declines_native_selection`]).
    fn dispatch_compiled(module: &ModuleAsm, requested: ProcedureDispatch) -> ProcedureDispatch {
        if module.plain_command_dispatch {
            ProcedureDispatch::Plain
        } else {
            requested
        }
    }

    fn compile_procedure_module(
        &mut self,
        compiler: &dyn CompileService<Module = ModuleAsm>,
        target: ProcedureCompilationTarget<'_>,
        source: &tcl_runtime_api::SourceImage,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: ProcedureDispatch,
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<ModuleAsm, TclError> {
        use tcl_runtime_api::NativeCompilationAdmissionScope::ProcedureBody;
        let namespace = target.target().namespace;
        let reached = match target.original {
            Some(request) if !request.matches_entry(entry) => {
                Err(tcl_runtime_api::CompileError::Unsupported(
                    "original procedure declaration does not match its compiler entry".into(),
                ))
            }
            Some(request) => compiler.compile_original_procedure_bytes_with_entry(
                request,
                self.source_profile(),
                entry,
                dispatch,
            ),
            None => compiler.compile_procedure_bytes_with_entry(
                target.ordinary,
                self.source_profile(),
                entry,
                dispatch,
            ),
        };
        if dispatch == ProcedureDispatch::Optimised && super::declines_native_selection(&reached) {
            drop(reached);
            return self.compile_procedure_module(
                compiler,
                target,
                source,
                entry,
                ProcedureDispatch::Plain,
                original_name_scope,
            );
        }
        original_name_scope
            .check(self)
            .map_err(|error| self.original_name_cmd_error(error))?;
        let reached = if target
            .original
            .is_some_and(|request| !request.matches_entry(entry))
        {
            drop(reached);
            Err(tcl_runtime_api::CompileError::Unsupported(
                "original procedure declaration retired during compilation".into(),
            ))
        } else {
            reached
        };
        let reported = reached.map_err(|error| {
            self.compile_service_error_bytes(error, source, namespace, ProcedureBody)
        });
        let module = self.original_name_tcl_outcome(original_name_scope, reported)?;
        self.validate_module_profile(&module)?;
        Self::validate_module_namespace_bytes(&module, namespace)?;
        Ok(module)
    }

    fn prepare_procedure_body_with_entry(
        &mut self,
        admission: Option<(&str, &str)>,
        parameters: &[crate::command::Param],
        namespace: &NamespacePath,
        original: ProcedureBodySource<'_>,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<CompiledUnit, TclError> {
        use tcl_runtime_api::NativeCompilationAdmissionScope::ProcedureBody;
        let source = original.image;
        let Some(compiler) = self.compiler.clone() else {
            return Err(self.compile_service_error_bytes(
                tcl_runtime_api::CompileError::Unsupported(
                    "procedure compilation requires a byte CompileService".into(),
                ),
                source,
                namespace,
                ProcedureBody,
            ));
        };
        let plan = self.procedure_command_plan(
            compiler.as_ref(),
            source,
            namespace,
            entry,
            original_name_scope,
        )?;
        if plan.fatal_tail.is_none()
            && let Some((name, parameter_source)) = admission
            && let Ok(text) = std::str::from_utf8(source.bytes())
            && let Ok(selected) = u32::try_from(entry.current_namespace)
            && let Some(unit) = self.module_proc(name, parameter_source, text, NsId(selected))
            && unit.source_namespace.compiled_path() == Some(namespace)
        {
            let pool = self.create_native_literal_pool(&unit.asm, namespace);
            original_name_scope
                .check(self)
                .map_err(|error| self.original_name_cmd_error(error))?;
            return Ok(unit.with_literal_pool(pool));
        }
        let inputs = ProcedureCompilationInputs::new(
            original,
            parameters,
            namespace,
            plan.complete_prefix_len,
            &self.native_compiler_pass_owner,
            entry.interpreter,
            tcl_registry::native_procedure::procedure_activation_protocol(
                self.actual_native_invocation_dialect(),
            ),
        )
        .map_err(|error| {
            self.compile_service_error_bytes(
                tcl_runtime_api::CompileError::Unsupported(error.to_string()),
                source,
                namespace,
                ProcedureBody,
            )
        })?;
        let force_plain = self.requires_plain_command_dispatch();
        let mut dispatch = if force_plain {
            ProcedureDispatch::Plain
        } else {
            ProcedureDispatch::Optimised
        };
        let mut module = self.compile_procedure_module(
            compiler.as_ref(),
            inputs.target(),
            source,
            entry,
            dispatch,
            original_name_scope,
        )?;
        dispatch = Self::dispatch_compiled(&module, dispatch);
        if dispatch == ProcedureDispatch::Optimised
            && !self.function_command_bindings_match_with_manifest(
                &module.top_level,
                module.manifest.as_deref(),
            )
        {
            dispatch = ProcedureDispatch::Plain;
            module = self.compile_procedure_module(
                compiler.as_ref(),
                inputs.target(),
                source,
                entry,
                dispatch,
                original_name_scope,
            )?;
        }
        if module.source != inputs.prefix {
            return Err(self.compile_service_error_bytes(
                tcl_runtime_api::CompileError::Unsupported(
                    "CompileService returned another procedure source image or channel".into(),
                ),
                source,
                namespace,
                ProcedureBody,
            ));
        }
        self.finish_procedure_unit(
            module,
            dispatch,
            plan.fatal_tail,
            entry,
            original_name_scope,
        )
    }

    /// Merge procedure candidates retaining their original body namespace.
    pub(crate) fn merge_procs(&mut self, module: &ModuleAsm) {
        self.merge_procedure_artifacts(
            module,
            CompilerProvenance::CurrentService(self.compiler_generation),
        );
    }

    /// Retain foreign compiler authority separately from the current service.
    pub(crate) fn merge_foreign_procs(&mut self, module: &ModuleAsm) {
        self.merge_procedure_artifacts(
            module,
            CompilerProvenance::AdmittedForeign(self.compiler_generation),
        );
    }

    /// Ensure `proc`'s compiled body matches the interp's current command,
    /// dialect-profile, and compile-service generations, recompiling `body_src` with plain
    /// command dispatch ([`CompileService::compile_plain_dispatch_for_profile`])
    /// or the fast compiler, per [`Self::requires_plain_command_dispatch`] —
    /// when it doesn't. This is the general recompile-on-demand mechanism
    /// C Tcl needs too: an actually executing step-traced invocation installs
    /// `DONT_COMPILE_CMDS_INLINE` for entered bodies until the last active step
    /// scope settles. Merely registering a step trace does not install it. Returns `proc` unchanged when it is already
    /// current. A required plain compile fails closed: running the old body
    /// would bypass the replacement command that caused invalidation.
    /// A current, binding-valid foreign admission may run unchanged only when
    /// no compile service is installed; retaining the foreign marker ensures a
    /// later `set_compiler` still refreshes it.
    fn command_owned_procedure_namespace(
        &self,
        procedure: &Rc<crate::command::ProcDef>,
    ) -> Option<NsId> {
        self.commands
            .values()
            .chain(self.hidden.commands.values())
            .find_map(|command| {
                if let Command::Proc(binding) = command {
                    Rc::ptr_eq(&binding.declaration(), procedure)
                        .then(|| procedure.actual_command_slot().namespace)
                } else {
                    None
                }
            })
    }

    pub(crate) fn ensure_proc_traced(
        &mut self,
        command: crate::command::NativeProcedureCommand,
    ) -> Result<crate::command::PreparedProcedureActivation, TclError> {
        self.ensure_proc_traced_with(command, None)
    }

    /// `current` is a body the caller has just selected for this declaration.
    fn ensure_proc_traced_with(
        &mut self,
        command: crate::command::NativeProcedureCommand,
        current: Option<CompiledUnit>,
    ) -> Result<crate::command::PreparedProcedureActivation, TclError> {
        let proc = command.declaration();
        let Some(protocol) = tcl_registry::native_procedure::procedure_activation_protocol(
            self.actual_native_invocation_dialect(),
        ) else {
            let _ = self
                .refuse_host_command("native procedure activation protocol is unavailable".into());
            return Err(TclError::from_execution_failure(
                self.execution_refusal
                    .clone()
                    .expect("activation protocol refusal retained"),
            ));
        };
        if proc.native_reference_count() == 0 {
            let _ = self.refuse_host_command("native procedure declaration has retired".into());
            return Err(TclError::from_execution_failure(
                self.execution_refusal
                    .clone()
                    .expect("procedure retirement refusal retained"),
            ));
        }
        let body = if !protocol.retains_prepared_body() {
            self.prepare_native_jim_script_unit(
                &proc.body_src,
                proc.actual_namespace_context().context(),
            )?
            .with_source_location(proc.body_src.source_location())
        } else if let Some(body) = current.or_else(|| self.current_native_procedure_body(&proc)) {
            body
        } else {
            let body = self
                .prepare_procedure_body_value_with_owner(
                    None,
                    &proc.params,
                    proc.actual_namespace_id(),
                    &proc.body_src,
                    Some(ProcedureCompilationOwner {
                        declaration: &proc,
                        command_namespace: self.command_owned_procedure_namespace(&proc),
                    }),
                )?
                .with_source_location(proc.body_src.source_location());
            let storage = self
                .actual_native_invocation_dialect()
                .native_bytecode_storage_protocol()
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original C procedure bytecode issuer",
                ))?;
            if body.fatal_tail.is_none() || storage.recipe().retains_parse_failure() {
                proc.body_src.install_native_procedure_bytecode_cache(
                    storage.recipe().version(),
                    body.clone(),
                    &proc,
                )?;
            }
            body
        };
        Ok(crate::command::PreparedProcedureActivation {
            proc,
            body,
            declaration_binding: command,
        })
    }

    /// Validate the executable on the same original body and declaration.
    pub(in super::super) fn current_native_procedure_body(
        &self,
        proc: &Rc<ProcDef>,
    ) -> Option<CompiledUnit> {
        let cached = proc.body_src.native_bytecode_cache();
        let cache_body = cached
            .as_ref()
            .filter(|cache| {
                cache.context == crate::value::NativeBytecodeContext::Procedure
                    && cache
                        .procedure
                        .borrow()
                        .upgrade()
                        .is_some_and(|owner| Rc::ptr_eq(&owner, proc))
                    && self
                        .actual_native_invocation_dialect()
                        .native_bytecode_storage_protocol()
                        .is_some_and(|protocol| protocol.recipe().version() == cache.version)
            })
            .map(|cache| &cache.unit);
        // Foreign authored artifacts have their independent admission and never
        // manufacture a native body primary or declaration context.
        let retained_foreign = proc.retained_foreign_body();
        // A declaration adapter built per call (a `TclOO` method body) carries
        // the body its persistent owner prepared with this compile service last
        // time: C keeps one `Proc` per method and `TclProcCompileProc` reuses
        // its bytecode until an epoch says otherwise. It must pass exactly the
        // currency checks below.
        let retained = retained_foreign
            .as_ref()
            .filter(|body| body.compiler.is_current_service(self.compiler_generation));
        let foreign = retained_foreign.as_ref().filter(|body| {
            self.compiler.is_none()
                && body
                    .compiler
                    .is_current_foreign_admission(self.compiler_generation)
        });
        let native = cache_body.or(retained);
        let body = native.or(foreign)?;
        let is_foreign = native.is_none();
        if (!is_foreign
            && body.native_cache != Some(self.native_cache_stamp(proc.actual_namespace_id())))
            || body.profile_generation != self.profile_generation
            || (!is_foreign && !body.compiler.is_current_service(self.compiler_generation))
            || body.source_namespace.compiled_path()
                != Some(&self.ns_path(proc.actual_namespace_id()))
            || body.interpreter != self.native_interpreter_identity()
            || !self.retained_function_bindings_match(&body.asm, body.manifest.as_deref())
            || self.step_trace_active()
        {
            return None;
        }
        let mut selected = body.clone();
        selected.command_epoch = self.trace_deopt_epoch();
        Some(selected)
    }

    /// Native C84/C85 replace a shared declaration before compiling its body.
    /// The selected binding is updated by identity even if compilation fails.
    pub(crate) fn ensure_proc_ready_in(
        &mut self,
        command: crate::command::NativeProcedureCommand,
        _sidecar: Option<&CommandSidecarKey>,
        _sidecar_handle: Option<&CommandSidecarHandle>,
    ) -> Result<crate::command::PreparedProcedureActivation, TclError> {
        use tcl_registry::native_procedure::{
            NativeProcedureCompilationPurpose, NativeProcedureRecompileAction,
        };
        let proc = command.declaration();
        let protocol = tcl_registry::native_procedure::procedure_activation_protocol(
            self.actual_native_invocation_dialect(),
        )
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native procedure activation issuer",
        ))?;
        let current = protocol
            .retains_prepared_body()
            .then(|| self.current_native_procedure_body(&proc))
            .flatten();
        if protocol.retains_prepared_body()
            && current.is_none()
            && protocol.recompilation_action(
                NativeProcedureCompilationPurpose::CommandBody,
                proc.native_reference_count(),
            ) == Some(NativeProcedureRecompileAction::ReplaceSharedDeclaration)
        {
            let strings = self.actual_native_string_protocol().ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native procedure source duplication issuer",
                ),
            )?;
            let replacement = proc.duplicate_for_native_recompilation(strings);
            command.replace_declaration(&replacement);
        }
        self.ensure_proc_traced_with(command, current)
    }
}
