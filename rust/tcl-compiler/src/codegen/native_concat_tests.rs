// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original concat emitter selection and captured command prerequisites.

use super::*;
use tcl_registry::native_compilation::NativeCompilationFrame;
use tcl_runtime_api::native_compilation::{
    NativeCommandImplementation, NativeCompilerHookPresence,
};

fn emit_original<'a>(
    registry: &'a tcl_registry::CommandRegistry,
    entry: &'a tcl_runtime_api::NativeCompilationEntry,
    source: &[u8],
) -> CodegenCtx<'a> {
    let point = entry.execution_point.unwrap();
    let dialect = tcl_registry::InvocationDialect::of_point(point);
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let parsed = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::native(source),
        tcl_lexer::Span::new(0, source.len() as u32),
        config,
    )
    .unwrap();
    let mut context = CodegenCtx::new(true, &["a", "b", "c"], registry);
    context.native_entry = Some(entry);
    context.invocation_dialect = Some(dialect);
    context.source_string_protocol = entry.source_string_protocol;
    context.compiled_variable_protocol = entry.compiled_variable_protocol;
    context.native_compilation.frame = NativeCompilationFrame::ProcedureCode;
    context.emit_native_words(&parsed.commands[0].words);
    context
}

#[test]
fn original_concat_emits_shared_recipe_with_captured_compiler_guards() {
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let inline =
            entry.execution_point.unwrap().tcl_version().unwrap() >= tcl_dialect::TclVersion::V8_6;
        for (source, runtime) in [
            (b"concat { A } { B }".as_slice(), false),
            (b"concat $a $b", true),
            (b"concat {*}{A B} $c", true),
        ] {
            if name == "tcl8.4" && source == b"concat {*}{A B} $c" {
                let dialect =
                    tcl_registry::InvocationDialect::of_point(entry.execution_point.unwrap());
                let parsed = tcl_lexer::native_script_words_in(
                    tcl_lexer::SourceImage::native(source),
                    tcl_lexer::Span::new(0, source.len() as u32),
                    tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
                )
                .unwrap();
                assert!(parsed.commands.is_empty());
                assert_eq!(
                    parsed.fatal_tail.unwrap().cut.message,
                    "extra characters after close-brace"
                );
                continue;
            }
            let context = emit_original(&registry, &entry, source);
            assert!(!context.native_dependency_refusal, "{name}: {source:?}");
            assert_eq!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.op == Op::CONCAT_STK),
                inline && runtime,
                "{name}: {source:?}"
            );
            assert_eq!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.native_compiler_selection.is_some()),
                inline,
                "{name}: original compiler prerequisite"
            );
            if inline && !runtime {
                assert!(
                    context
                        .literals
                        .entries()
                        .iter()
                        .any(|literal| literal.bytes() == b"A B")
                );
            }
        }
    }
}

#[test]
fn original_concat_changed_worker_keeps_generic_dispatch_and_unknown_withdrawal() {
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let mut entry = crate::environment_ingress::captured_native_entry(profile);
        let token = entry
            .lookup_command_bytes(entry.current_namespace, b"concat")
            .unwrap()
            .unwrap()
            .token;
        let target = entry
            .commands
            .iter_mut()
            .find(|binding| binding.token == token)
            .unwrap();
        target.token += 10_000;
        target.implementation_generation += 1;
        target.implementation = NativeCommandImplementation::Opaque;
        target.compiler_hook = NativeCompilerHookPresence::Absent;
        target.compiler = None;
        let context = emit_original(&registry, &entry, b"concat $a $b");
        assert!(!context.native_dependency_refusal, "{name}");
        assert!(
            !context
                .instructions
                .iter()
                .any(|instruction| instruction.op == Op::CONCAT_STK
                    || instruction.native_compiler_selection.is_some())
        );
        assert!(
            context
                .instructions
                .iter()
                .any(|instruction| matches!(instruction.op, Op::INVOKE_STK1 | Op::INVOKE_STK4))
        );
        entry
            .commands
            .iter_mut()
            .find(|binding| binding.slot.simple.as_bytes() == b"concat")
            .unwrap()
            .compiler_hook = NativeCompilerHookPresence::Unknown;
        assert!(
            emit_original(&registry, &entry, b"concat $a $b").native_dependency_refusal,
            "{name}: missing actual hook"
        );
    }
}
