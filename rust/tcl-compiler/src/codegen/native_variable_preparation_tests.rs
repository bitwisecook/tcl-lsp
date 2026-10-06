// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual selected load/store preparation preserves its original source owner.

use super::*;
use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings, SourceNamespaceKey};
use crate::ir::CommandTokens;
use tcl_registry::native_compilation::{NativeCompilationContext, NativeCompilationFrame};
use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;

fn original_tokens(
    source: &str,
    entry: &tcl_runtime_api::NativeCompilationEntry,
    registry: &tcl_registry::CommandRegistry,
    context: NativeCompilationContext,
) -> CommandTokens {
    let dialect = tcl_registry::InvocationDialect::of_point(entry.execution_point.unwrap());
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
        namespace: "::".to_owned(),
        identity: "original-variable-preparation".to_owned(),
    }
    .with_namespace_identity(SourceNamespaceKey::Native(
        entry
            .retained_namespace_context(entry.current_namespace)
            .unwrap(),
    ));
    let bindings = SourceCommandBindings::analyse_in_frame_with_options(
        source,
        &frame,
        config,
        registry,
        SourceAnalysisOptions {
            native_entry: Some(entry),
            invocation_dialect: Some(dialect),
            native_compilation: context,
            ..Default::default()
        },
    );
    let command =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &command);
    bindings.stamp_original_tokens(&mut tokens);
    tokens
}

#[test]
fn original_c91_uplevel_emits_native_frame_operation_and_defers_body_compilation() {
    for (source, concatenated) in [
        ("uplevel 1 {set x OUTER}", false),
        ("uplevel {set x OUTER}", false),
        ("uplevel 1 set x OUTER", true),
    ] {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let image = tcl_lexer::SourceImage::native(source.as_bytes());
        let parsed = tcl_lexer::native_script_words_in(
            image,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let mut context = CodegenCtx::new(true, &[], &registry);
        context.native_entry = Some(&entry);
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(profile));
        context.source_string_protocol = entry.source_string_protocol;
        context.compiled_variable_protocol = entry.compiled_variable_protocol;
        context.native_compilation.frame = NativeCompilationFrame::ProcedureCode;
        context.emit_native_words(&parsed.commands[0].words);
        assert!(!context.native_dependency_refusal, "{source}");
        assert!(
            context
                .instructions
                .iter()
                .any(|instruction| instruction.op == Op::UPLEVEL)
        );
        assert_eq!(
            context
                .instructions
                .iter()
                .any(|instruction| instruction.op == Op::CONCAT_STK),
            concatenated
        );
        assert!(
            context
                .instructions
                .iter()
                .any(|instruction| instruction.native_compiler_selection.is_some())
        );
        assert!(
            context.lvt.native_slot_names().is_empty(),
            "script body is compiled only after frame selection"
        );
    }
}

#[test]
fn original_variable_read_uses_selected_plan_and_withdraws_missing_compiler_receipts() {
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let mut context = CodegenCtx::new(true, &["x", "name"], &registry);
        context.native_entry = Some(&entry);
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_point(
            entry.execution_point.unwrap(),
        ));
        context.source_string_protocol = entry.source_string_protocol;
        context.compiled_variable_protocol = entry.compiled_variable_protocol;
        context.native_compilation.frame = NativeCompilationFrame::ProcedureCode;
        let tokens = original_tokens("set x", &entry, &registry, context.native_compilation);
        assert!(
            context.emit_native_original_preparation_invocation(Some(&tokens)),
            "{name}"
        );
        assert!(
            context
                .instructions
                .iter()
                .any(|instruction| instruction.op == Op::LOAD_SCALAR1)
        );
        assert!(
            context
                .instructions
                .iter()
                .any(|instruction| instruction.native_compiler_selection.is_some())
        );
        assert!(
            context.literals.is_empty(),
            "{name}: original local read allocates no command/name literal"
        );

        let mut no_source = tokens.clone();
        no_source.source_binding = None;
        assert!(!context.emit_native_original_preparation_invocation(Some(&no_source)));
        let mut no_entry = CodegenCtx::new(true, &["x"], &registry);
        no_entry.invocation_dialect = context.invocation_dialect;
        assert!(!no_entry.emit_native_original_preparation_invocation(Some(&tokens)));
        assert!(no_entry.instructions.is_empty());
        assert!(no_entry.literals.is_empty());

        for hook in [
            NativeCompilerHookPresence::Absent,
            NativeCompilerHookPresence::Unknown,
        ] {
            let mut changed = entry.clone();
            let selected = changed
                .lookup_command_bytes(changed.current_namespace, b"set")
                .unwrap()
                .unwrap()
                .token;
            let binding = changed
                .commands
                .iter_mut()
                .find(|binding| binding.token == selected)
                .unwrap();
            binding.token += 10_000;
            binding.implementation_generation += 1;
            binding.compiler_hook = hook;
            binding.compiler = None;
            let mut changed_context = CodegenCtx::new(true, &["x"], &registry);
            changed_context.native_entry = Some(&changed);
            changed_context.invocation_dialect = context.invocation_dialect;
            changed_context.source_string_protocol = entry.source_string_protocol;
            changed_context.compiled_variable_protocol = entry.compiled_variable_protocol;
            changed_context.native_compilation = context.native_compilation;
            assert!(
                !changed_context.emit_native_original_preparation_invocation(Some(&tokens)),
                "{name}/{hook:?}"
            );
            assert!(changed_context.instructions.is_empty());
            assert!(changed_context.literals.is_empty());
        }
    }
}

#[test]
fn original_dynamic_variable_target_keeps_stack_lookup_and_original_local_operand() {
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let mut context = CodegenCtx::new(true, &["name"], &registry);
        context.native_entry = Some(&entry);
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_point(
            entry.execution_point.unwrap(),
        ));
        context.source_string_protocol = entry.source_string_protocol;
        context.compiled_variable_protocol = entry.compiled_variable_protocol;
        context.native_compilation.frame = NativeCompilationFrame::ProcedureCode;
        let tokens = original_tokens("set $name", &entry, &registry, context.native_compilation);
        assert!(
            context.emit_native_original_preparation_invocation(Some(&tokens)),
            "{name}"
        );
        assert!(
            context
                .instructions
                .iter()
                .any(|instruction| instruction.op == Op::LOAD_STK)
        );
        assert_eq!(
            context.lvt.native_slot_names(),
            vec![Some(tcl_core_types::NameBytes::from(b"name".as_slice()))]
        );
        assert!(context.literals.is_empty());
    }
}
