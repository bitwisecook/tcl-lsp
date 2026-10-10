// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original selected command introspection over retained compiler inputs.

use super::*;

fn emit<'a>(
    registry: &'a tcl_registry::CommandRegistry,
    entry: &'a tcl_runtime_api::NativeCompilationEntry,
    source: &[u8],
) -> CodegenCtx<'a> {
    let dialect = tcl_registry::InvocationDialect::of_point(entry.execution_point.unwrap());
    let parsed = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::native(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    )
    .unwrap();
    let mut context = CodegenCtx::new(true, &["pattern"], registry);
    context.native_entry = Some(entry);
    context.invocation_dialect = Some(dialect);
    context.source_string_protocol = entry.source_string_protocol;
    context.compiled_variable_protocol = entry.compiled_variable_protocol;
    context.native_compilation.frame =
        tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
    context.emit_native_words(&parsed.commands[0].words);
    context
}

#[test]
fn original_info_commands_compiler_keeps_conditional_list_geometry() {
    // naming.compiler.original-info-commands-literal-resolution
    // docs/design/analysis/name-resolution-proofs/compiler-original-info-commands-literal-resolution.md
    // Software instruction-consumer control; native measurements are recorded
    // independently in the proof with this question identifier.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        for source in [
            b"info commands ::set".as_slice(),
            b"info commands ::missing",
        ] {
            let context = emit(&registry, &entry, source);
            assert!(!context.native_dependency_refusal, "{engine}");
            let operations: Vec<_> = context.instructions.iter().map(|item| item.op).collect();
            if entry.execution_point.unwrap().tcl_version().unwrap() < tcl_dialect::TclVersion::V8_6
            {
                assert!(!operations.contains(&Op::RESOLVE_CMD), "{engine}");
                assert!(
                    operations
                        .iter()
                        .any(|op| matches!(op, Op::INVOKE_STK1 | Op::INVOKE_STK4))
                );
                continue;
            }
            let resolve = operations
                .iter()
                .position(|op| *op == Op::RESOLVE_CMD)
                .unwrap();
            assert_eq!(operations[resolve + 1], Op::DUP, "{engine}");
            assert_eq!(operations[resolve + 2], Op::STR_LEN, "{engine}");
            assert!(matches!(
                operations[resolve + 3],
                Op::JUMP_FALSE1 | Op::JUMP_FALSE4
            ));
            assert_eq!(operations[resolve + 4], Op::LIST, "{engine}");
            assert_eq!(
                context.instructions[resolve + 4].operands,
                vec![Operand::Imm(1)]
            );
        }
    }
}

#[test]
fn original_info_commands_compiler_requires_literal_pattern_and_actual_worker() {
    // naming.compiler.original-info-commands-literal-resolution
    // docs/design/analysis/name-resolution-proofs/compiler-original-info-commands-literal-resolution.md
    // This tests producer refusal, not runtime command absence or Tcl semantics.
    use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
    let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let mut entry = crate::environment_ingress::captured_native_entry(profile);
    for source in [b"info commands $pattern".as_slice(), b"info commands ::s*"] {
        let context = emit(&registry, &entry, source);
        assert!(!context.native_dependency_refusal);
        assert!(
            !context
                .instructions
                .iter()
                .any(|item| item.op == Op::RESOLVE_CMD)
        );
    }
    let token = entry
        .lookup_command_bytes(entry.current_namespace, b"::tcl::info::commands")
        .unwrap()
        .unwrap()
        .token;
    entry
        .commands
        .iter_mut()
        .find(|command| command.token == token)
        .unwrap()
        .compiler_hook = NativeCompilerHookPresence::Unknown;
    let context = emit(&registry, &entry, b"info commands ::set");
    assert!(context.native_dependency_refusal);
    assert!(
        !context
            .instructions
            .iter()
            .any(|item| item.op == Op::RESOLVE_CMD)
    );
}
