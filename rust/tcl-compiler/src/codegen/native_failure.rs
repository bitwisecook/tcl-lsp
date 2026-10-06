// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Transfer a proved source compiler rejection to the runtime admission ABI.
//!
//! Compiler errors have dependencies even though their rejected invocation
//! never reaches runtime dispatch. Retaining only ordinary reached command
//! sites would let a cached error survive replacement of its native handler.

use super::CodegenCtx;
use crate::command_binding::SourceNativeCompilationFailure;
use tcl_runtime_api::{NativeCompilationError, NativeCompilationErrorCommand};

/// Project exact presentation without guessing missing source information.
pub(super) fn error(failure: &SourceNativeCompilationFailure) -> Option<NativeCompilationError> {
    let command_contexts = failure
        .contexts
        .iter()
        .map(|context| {
            Some(NativeCompilationErrorCommand {
                text: context.command.clone()?,
                line: context.line_in_chunk.filter(|line| *line != 0)?,
                before_context: context.before_context.clone(),
                after_context: context.after_context.clone(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let body_line = command_contexts.last()?.line;
    Some(NativeCompilationError {
        message: failure.failure.message.clone()?,
        error_code: failure.failure.error_code.clone(),
        command_contexts,
        body_line,
    })
}

/// Record the lookup assumptions of an entry rejection before collecting the
/// function's binding requirements. This is independent of opcode emission.
pub(super) fn retain_dependencies(ctx: &mut CodegenCtx, failure: &SourceNativeCompilationFailure) {
    for dependency in &failure.dependencies {
        ctx.retain_native_compilation_dependency(
            crate::registry_invocation::native_compilation_dependency(dependency),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::command_binding::SourceAnalysisOptions;
    use tcl_registry::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    };
    use tcl_runtime_api::CommandBindingGuard;

    fn module() -> (
        crate::ir::Module,
        std::sync::Arc<tcl_registry::CommandRegistry>,
    ) {
        module_for("set earlier 1; set x extra bad")
    }

    fn module_for(
        source: &str,
    ) -> (
        crate::ir::Module,
        std::sync::Arc<tcl_registry::CommandRegistry>,
    ) {
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        module_for_entry(
            source,
            Some(std::sync::Arc::new(
                crate::environment_ingress::captured_native_entry(profile),
            )),
        )
    }

    fn module_for_entry(
        source: &str,
        native_entry: Option<std::sync::Arc<tcl_runtime_api::NativeCompilationEntry>>,
    ) -> (
        crate::ir::Module,
        std::sync::Arc<tcl_registry::CommandRegistry>,
    ) {
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile)
            .commands()
            .clone();
        let module = crate::lowering::lower_proc_body_module_for_bytecode_with_options(
            source,
            "",
            &registry,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
            Some(profile),
            false,
            Some(SourceAnalysisOptions {
                native_entry: native_entry.as_deref(),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ProcedureCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                ..SourceAnalysisOptions::default()
            }),
        );
        (module, registry)
    }

    fn compiler_registration<'a>(
        function: &'a tcl_bytecode::FunctionAsm,
        name: &str,
    ) -> &'a tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite {
        function
            .native_compiler_prerequisites
            .iter()
            .find_map(|required| {
                use tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite;
                match required {
                    NativeCompilerSelectionPrerequisite::Command(required)
                    | NativeCompilerSelectionPrerequisite::Ensemble(required)
                        if required.invocation_word.as_bytes() == name.as_bytes()
                            && required.guard == CommandBindingGuard::ChunkEntry =>
                    {
                        Some(required.as_ref())
                    }
                    _ => None,
                }
            })
            .expect("original compiler registration at chunk entry")
    }

    #[test]
    fn missing_original_entry_keeps_compiler_rejection_as_an_admission_obligation() {
        let (module, registry) = module_for_entry("set x extra bad", None);
        let execution =
            crate::cfg_builder::build_cfg_codegen_with_registry(&module, false, &registry);
        let function = super::super::codegen_module(&execution, &module, &registry).top_level;
        assert_eq!(
            function.validate_native_compilation_entry(),
            Err(tcl_runtime_api::NativeCompilationAdmissionError::NativePreflightRequired),
        );
    }

    #[test]
    fn missing_original_entry_does_not_invent_a_guest_compiler_error() {
        let (module, registry) = module_for_entry("", None);
        let execution =
            crate::cfg_builder::build_cfg_codegen_with_registry(&module, false, &registry);
        let function = super::super::codegen_module(&execution, &module, &registry).top_level;
        assert!(function.native_compilation_failure.is_none());
        assert_eq!(
            function.validate_native_compilation_entry(),
            Err(tcl_runtime_api::NativeCompilationAdmissionError::NativePreflightRequired),
        );
        let (module, registry) = module_for("");
        let execution =
            crate::cfg_builder::build_cfg_codegen_with_registry(&module, false, &registry);
        let function = super::super::codegen_module(&execution, &module, &registry).top_level;
        assert!(function.native_compilation_failure.is_none());
        assert!(function.validate_native_compilation_entry().is_ok());
    }

    #[test]
    fn compiler_rejection_retains_undispatched_native_guard_and_no_analysis_effects() {
        let (module, registry) = module();
        let analysis = crate::cfg_builder::build_cfg_with_registry(&module, false, &registry);
        assert!(
            analysis
                .top_level
                .blocks
                .values()
                .all(|block| block.statements.is_empty())
        );
        let entry = &analysis.top_level.blocks[&analysis.top_level.entry];
        assert!(matches!(
            entry.terminator,
            Some(crate::cfg::Terminator::Complete { .. })
        ));

        let execution =
            crate::cfg_builder::build_cfg_codegen_with_registry(&module, false, &registry);
        let bytecode = super::super::codegen_module(&execution, &module, &registry);
        let function = &bytecode.top_level;
        let error = function.native_compilation_failure.as_ref().unwrap();
        assert_eq!(
            error.message,
            "wrong # args: should be \"set varName ?newValue?\""
        );
        assert_eq!(error.error_code.as_deref(), Some("NONE"));
        assert_eq!(error.command_contexts[0].text, "set x extra bad");
        let required = compiler_registration(function, "set");
        assert_eq!(required.compiler.registry_identity, "set");
        assert!(function.validate_native_compilation_entry().is_ok());
    }

    #[test]
    fn rejected_command_compiler_guard_withdraws_on_original_registration_mutation() {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;

        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let (module, registry) =
            module_for_entry("set x extra bad", Some(std::sync::Arc::new(entry.clone())));
        let execution =
            crate::cfg_builder::build_cfg_codegen_with_registry(&module, false, &registry);
        let function = super::super::codegen_module(&execution, &module, &registry).top_level;
        let required = compiler_registration(&function, "set");
        let matches = |candidate: &tcl_runtime_api::NativeCompilationEntry| {
            required
                .matches_registration_with(|namespace, word| {
                    candidate
                        .lookup_command_bytes(namespace, word.as_bytes())
                        .map(|binding| binding.cloned())
                })
                .unwrap()
        };
        assert!(matches(&entry));
        let index = entry
            .commands
            .iter()
            .position(|binding| {
                binding.slot == required.slot && binding.namespace_token == required.namespace_token
            })
            .unwrap();
        // Each control preserves the same interpreter and all other original
        // rows, so the retained compiler-registration axis must reject it.
        for mutation in 0..7 {
            let mut changed = entry.clone();
            let binding = &mut changed.commands[index];
            match mutation {
                0 => binding.token = binding.token.wrapping_add(1),
                1 => {
                    binding.implementation_generation =
                        binding.implementation_generation.wrapping_add(1)
                }
                2 => binding.compiler_hook = NativeCompilerHookPresence::Absent,
                3 => binding.compiler_hook = NativeCompilerHookPresence::Unknown,
                4 => binding.compiler = None,
                5 => {
                    binding.compiler.as_mut().unwrap().registry_identity =
                        "different compiler".into()
                }
                6 => binding.has_execution_trace = true,
                _ => unreachable!(),
            }
            assert!(!matches(&changed), "registration mutation {mutation}");
        }
        let mut removed = entry.clone();
        removed.commands.remove(index);
        assert!(!matches(&removed));
    }

    #[test]
    fn closed_fixed_math_absence_is_the_original_rejection_before_later_catch() {
        use tcl_runtime_api::native_compilation::NativeMathFunctionResolution;

        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let entry = std::sync::Arc::new(crate::environment_ingress::captured_native_entry(profile));
        let table = entry.math_functions.as_ref().unwrap();
        assert!(table.closed);
        assert_eq!(
            table.lookup("future_function"),
            NativeMathFunctionResolution::Absent
        );
        let mut incomplete = table.clone();
        incomplete.closed = false;
        assert_eq!(
            incomplete.lookup("future_function"),
            NativeMathFunctionResolution::Unknown
        );
        let (module, registry) = module_for_entry(
            "expr {future_function(1)}; catch {error CHILD} result options; list ignored",
            Some(std::sync::Arc::clone(&entry)),
        );
        let execution =
            crate::cfg_builder::build_cfg_codegen_with_registry(&module, false, &registry);
        let function = super::super::codegen_module(&execution, &module, &registry).top_level;
        let error = function.native_compilation_failure.as_ref().unwrap();
        assert_eq!(error.message, "unknown math function \"future_function\"");
        assert_eq!(error.error_code.as_deref(), Some("NONE"));
        assert!(function.instructions.is_empty());
        let required = function.native_math_table_prerequisite.as_ref().unwrap();
        assert_eq!(required.interpreter, entry.interpreter);
        assert_eq!(&required.table, table);
        assert!(
            function
                .native_compiler_prerequisites
                .iter()
                .all(|required| {
                    use tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite;
                    match required {
                        NativeCompilerSelectionPrerequisite::Command(required)
                        | NativeCompilerSelectionPrerequisite::Ensemble(required) => {
                            !matches!(required.invocation_word.as_bytes(), b"catch" | b"list")
                        }
                        NativeCompilerSelectionPrerequisite::ProcedureHeader(_) => false,
                    }
                })
        );
        assert!(function.validate_native_compilation_entry().is_ok());
    }

    #[test]
    fn entry_rejection_retains_original_prefix_guards_without_later_emission() {
        let (module, registry) = module_for(
            "incr before; catch {error CHILD} result options; expr {future_function(1)}; list ignored",
        );
        let execution =
            crate::cfg_builder::build_cfg_codegen_with_registry(&module, false, &registry);
        let function = super::super::codegen_module(&execution, &module, &registry).top_level;
        let error = function.native_compilation_failure.as_ref().unwrap();
        assert_eq!(
            error.message,
            "wrong # args: should be \"catch command ?varName?\""
        );
        assert_eq!(error.error_code, None);
        assert!(function.instructions.is_empty());
        compiler_registration(&function, "incr");
        compiler_registration(&function, "catch");
        assert!(
            function
                .native_compiler_prerequisites
                .iter()
                .all(|required| {
                    use tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite;
                    match required {
                        NativeCompilerSelectionPrerequisite::Command(required)
                        | NativeCompilerSelectionPrerequisite::Ensemble(required) => {
                            !matches!(required.invocation_word.as_bytes(), b"list" | b"expr")
                        }
                        NativeCompilerSelectionPrerequisite::ProcedureHeader(_) => false,
                    }
                })
        );
        assert!(function.validate_native_compilation_entry().is_ok());
    }

    #[test]
    fn entry_rejection_retains_the_original_prefix_fixed_math_table() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let entry = std::sync::Arc::new(crate::environment_ingress::captured_native_entry(profile));
        let (module, registry) = module_for_entry(
            "expr {abs(1)}; catch {error CHILD} result options",
            Some(std::sync::Arc::clone(&entry)),
        );
        let execution =
            crate::cfg_builder::build_cfg_codegen_with_registry(&module, false, &registry);
        let function = super::super::codegen_module(&execution, &module, &registry).top_level;
        assert_eq!(
            function
                .native_compilation_failure
                .as_ref()
                .unwrap()
                .error_code,
            None
        );
        let required = function.native_math_table_prerequisite.as_ref().unwrap();
        assert_eq!(required.interpreter, entry.interpreter);
        assert_eq!(Some(&required.table), entry.math_functions.as_ref());
        assert!(function.instructions.is_empty());
        assert!(function.validate_native_compilation_entry().is_ok());
    }

    #[test]
    fn unresolved_earlier_compiler_visit_withdraws_later_rejection_presentation() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let mut entry = crate::environment_ingress::captured_native_entry(profile);
        entry.math_functions.as_mut().unwrap().closed = false;
        let (module, registry) = module_for_entry(
            "expr {future_function(1)}; catch {error CHILD} result options; list ignored",
            Some(std::sync::Arc::new(entry)),
        );
        let execution =
            crate::cfg_builder::build_cfg_codegen_with_registry(&module, false, &registry);
        let function = super::super::codegen_module(&execution, &module, &registry).top_level;
        assert!(function.native_compilation_failure.is_none());
        assert!(function.instructions.is_empty());
        assert_eq!(
            function.native_compilation_preflight,
            tcl_runtime_api::NativeCompilationPreflight::UnpresentedDefiniteFailure
        );
        assert_eq!(
            function.validate_native_compilation_entry(),
            Err(tcl_runtime_api::NativeCompilationAdmissionError::NativePreflightRequired)
        );
    }

    #[test]
    fn unpresented_failure_is_a_host_admission_obligation() {
        let (module, registry) = module();
        let mut execution =
            crate::cfg_builder::build_cfg_codegen_with_registry(&module, false, &registry);
        execution
            .top_level
            .native_compilation_failure
            .as_mut()
            .unwrap()
            .failure
            .message = None;
        let bytecode = super::super::codegen_module(&execution, &module, &registry);
        let function = &bytecode.top_level;
        assert!(function.native_compilation_failure.is_none());
        assert_eq!(
            function.native_compilation_preflight,
            tcl_runtime_api::NativeCompilationPreflight::UnpresentedDefiniteFailure
        );
        assert_eq!(
            function.validate_native_compilation_entry(),
            Err(tcl_runtime_api::NativeCompilationAdmissionError::NativePreflightRequired)
        );
        compiler_registration(function, "set");
    }
}
