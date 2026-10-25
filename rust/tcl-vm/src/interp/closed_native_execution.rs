// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Restrictive observations and execution used by the private transcript owner.

use super::{Code, ROOT_INTERP, ROOT_NS, Vm};
use tcl_bytecode::{FunctionAsm, ModuleAsm, Op};
use tcl_runtime_api::closed_native_consumer::ClosedSourceRefusal;
use tcl_runtime_api::native_compilation::{NativeCommandImplementation, NativeCompilationEntry};

/// Private preparation retained by its concrete owner. This opaque artifact
/// grants no closed-consumer authority to an ordinary VM or its `CompileService`.
pub struct PreparedClosedNativeSource {
    module: ModuleAsm,
    entry: NativeCompilationEntry,
}

impl Vm {
    /// Retain authenticated original commands only. This removes capabilities;
    /// the returned observation is not a closed-consumer execution permission.
    pub fn restrict_closed_native_commands(
        &mut self,
        commands: &[String],
        scalars: &[String],
    ) -> Result<NativeCompilationEntry, ClosedSourceRefusal> {
        let initial = self.capture_closed_native_state(commands, scalars, false, None)?;
        let tokens = commands
            .iter()
            .map(|name| {
                self.registered_command_token_bytes(name.as_bytes())
                    .ok_or(ClosedSourceRefusal::NativeContext)
            })
            .collect::<Result<Vec<_>, _>>()?;
        for name in commands {
            original_binding(&initial, name)?;
        }
        self.retain_registered_command_tokens(&tokens);
        self.capture_closed_native_state(commands, scalars, true, None)
    }

    /// Recapture actual root, observer, callable and cell state without reading
    /// any variable value. Undefined and linked roots also count as present.
    pub fn observe_closed_native_state(
        &self,
        commands: &[String],
        scalars: &[String],
        observation: &NativeCompilationEntry,
    ) -> Result<NativeCompilationEntry, ClosedSourceRefusal> {
        self.capture_closed_native_state(commands, scalars, true, Some(observation))
    }

    fn capture_closed_native_state(
        &self,
        commands: &[String],
        scalars: &[String],
        restricted: bool,
        retained: Option<&NativeCompilationEntry>,
    ) -> Result<NativeCompilationEntry, ClosedSourceRefusal> {
        if self.cur != ROOT_INTERP
            || self.current_ns_id() != ROOT_NS
            || self.frames.len() != 1
            || self.execution_refusal.is_some()
            || !self.cmd_traces.is_empty()
            || !self.exec_traces.is_empty()
            || !self.firing_cmd_traces.is_empty()
            || self.trace_in_progress.get()
            || !self.native_publications_are_committed()
        {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        let entry = if let Some(retained) = retained {
            let receipt = retained
                .empty_literal_world
                .as_ref()
                .ok_or(ClosedSourceRefusal::Currency)?;
            self.native_compilation_entry_with_literal_observation(
                ROOT_NS,
                false,
                |owner, epoch| {
                    self.native_literal_world
                        .borrow()
                        .retain_empty_observation(receipt, owner, epoch)
                },
            )
        } else {
            self.native_compilation_entry_for_namespace_token(ROOT_NS, false)
        };
        if entry.empty_literal_world.is_none()
            || !entry.closed
            || entry.execution_point.is_none()
            || entry.invocation_policy.as_ref() != Some(&entry.profile)
            || !entry.expression_policy.as_ref().is_some_and(|policy| {
                policy.profile == entry.profile
                    && Some(policy.origin)
                        == entry.execution_point.map(
                            tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin::Native,
                        )
                    && policy.numeric_simulation.is_none()
                    && policy.authored_functions.is_none()
            })
            || !entry.variable_observers.permits_no_callbacks()
            || !entry
                .command_resolvers
                .is_some_and(|resolver| resolver.permits_no_callbacks(&entry))
        {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        let tables = entry
            .namespace_variable_tables
            .as_ref()
            .ok_or(ClosedSourceRefusal::NativeContext)?;
        let root = tables
            .iter()
            .find(|table| table.namespace.token == entry.current_namespace)
            .ok_or(ClosedSourceRefusal::NativeContext)?;
        for name in scalars {
            if tcl_registry::special_vars::special_var(name).is_some()
                || root
                    .roots
                    .iter()
                    .any(|key| key.as_bytes() == name.as_bytes())
            {
                return Err(ClosedSourceRefusal::NativeContext);
            }
        }
        for name in commands {
            original_binding(&entry, name)?;
        }
        if restricted && entry.commands.len() != commands.len() {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        Ok(entry)
    }

    /// Visit the original actual compiler before a private owner can issue a
    /// scope. Ordinary callers gain no erasure or closed-consumer permission.
    pub fn prepare_closed_native_source(
        &self,
        source: &str,
        commands: &[String],
        scalars: &[String],
        observation: &NativeCompilationEntry,
    ) -> Result<PreparedClosedNativeSource, ClosedSourceRefusal> {
        let before = self.observe_closed_native_state(commands, scalars, observation)?;
        let image = tcl_runtime_api::SourceImage::native(source.as_bytes());
        let namespace = self.source_namespace_path();
        let module = self
            .compiler
            .as_ref()
            .ok_or(ClosedSourceRefusal::NativeContext)?
            .compile_plain_script_bytes_with_entry(
                tcl_runtime_api::ScriptCompileTargetBytes {
                    source: &image,
                    namespace: &namespace,
                },
                self.source_profile(),
                &before,
            )
            .map_err(|_| ClosedSourceRefusal::Execution)?;
        check_module(&module, &image, &namespace, self.source_profile())?;
        self.validate_module_profile(&module)
            .map_err(|_| ClosedSourceRefusal::NativeContext)?;
        for function in [&module.top_level, &module.top_level_body] {
            function
                .validate_native_compilation_entry()
                .map_err(|_| ClosedSourceRefusal::NativeContext)?;
            if !self.native_compiler_prerequisites_match(function)
                || !self.native_math_table_prerequisite_matches(function)
            {
                return Err(ClosedSourceRefusal::Currency);
            }
        }
        let after = self.observe_closed_native_state(commands, scalars, observation)?;
        if before != after {
            return Err(ClosedSourceRefusal::Currency);
        }
        Ok(PreparedClosedNativeSource {
            module,
            entry: after,
        })
    }

    /// Recheck the entire actual entry retained by an opaque preparation.
    #[must_use]
    pub fn closed_native_preparation_is_current(
        &self,
        prepared: &PreparedClosedNativeSource,
        commands: &[String],
        scalars: &[String],
    ) -> bool {
        self.observe_closed_native_state(commands, scalars, &prepared.entry)
            .is_ok_and(|current| current == prepared.entry)
    }

    /// Execute only a freshly checked plain preparation. No completion value
    /// escapes. This is not a closed-consumer grant for ordinary VM callers.
    pub fn execute_closed_native_preparation(
        &mut self,
        prepared: PreparedClosedNativeSource,
        commands: &[String],
        scalars: &[String],
    ) -> Result<(), ClosedSourceRefusal> {
        if !self.closed_native_preparation_is_current(&prepared, commands, scalars) {
            return Err(ClosedSourceRefusal::Currency);
        }
        let completion = self.run_current_module_at(&prepared.module, None);
        if completion.code != Code::Ok || self.execution_refusal.is_some() {
            return Err(ClosedSourceRefusal::Execution);
        }
        drop(completion);
        drop(prepared);
        Ok(())
    }
}

fn original_binding(entry: &NativeCompilationEntry, name: &str) -> Result<(), ClosedSourceRefusal> {
    let binding = entry
        .lookup_command_bytes(entry.current_namespace, name.as_bytes())
        .map_err(|_| ClosedSourceRefusal::NativeContext)?
        .ok_or(ClosedSourceRefusal::NativeContext)?;
    if !matches!(&binding.implementation,
        NativeCommandImplementation::Registry { identity, .. } if identity == name)
        || binding.has_execution_trace
    {
        return Err(ClosedSourceRefusal::NativeContext);
    }
    Ok(())
}

fn check_module(
    module: &ModuleAsm,
    source: &tcl_runtime_api::SourceImage,
    namespace: &tcl_runtime_api::ByteNamespacePath,
    profile: &'static tcl_dialect::DialectProfile,
) -> Result<(), ClosedSourceRefusal> {
    if module.profile.cache_key() != profile.cache_key()
        || &module.source != source
        || &module.source_namespace != namespace
        || !module.plain_command_dispatch
        || !module.procedures.is_empty()
        || !module.procedure_provenance.is_empty()
    {
        return Err(ClosedSourceRefusal::NativeContext);
    }
    check_function(&module.top_level, false)?;
    check_function(&module.top_level_body, true)
}

fn check_function(function: &FunctionAsm, body: bool) -> Result<(), ClosedSourceRefusal> {
    if function.native_compilation_failure.is_some()
        || !function.plain_command_dispatch
        || !function.command_bindings.is_empty()
        || !function.procedure_bindings.is_empty()
        || !function.loop_targets.is_empty()
        || function.instructions.len() > 4096
    {
        return Err(ClosedSourceRefusal::NativeContext);
    }
    for instruction in &function.instructions {
        let allowed = matches!(
            instruction.op,
            Op::NOP
                | Op::PUSH1
                | Op::PUSH4
                | Op::POP
                | Op::DUP
                | Op::ADD
                | Op::SUB
                | Op::MULT
                | Op::LOAD_STK
                | Op::LOAD_SCALAR_STK
                | Op::INVOKE_STK1
                | Op::INVOKE_STK4
                | Op::DONE
                | Op::START_CMD
        ) || (body
            && matches!(
                instruction.op,
                Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4 | Op::RETURN_IMM
            ));
        if !allowed {
            return Err(ClosedSourceRefusal::NativeContext);
        }
    }
    Ok(())
}
