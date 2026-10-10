// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original namespace-tail opcode selection and independent worker guards.

use super::*;

fn emit<'a>(
    registry: &'a tcl_registry::CommandRegistry,
    entry: &'a tcl_runtime_api::NativeCompilationEntry,
) -> CodegenCtx<'a> {
    emit_source(registry, entry, b"namespace tail $value")
}

fn emit_source<'a>(
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
    let mut context = CodegenCtx::new(true, &["value"], registry);
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
fn original_namespace_tail_emits_the_observed_counted_compiler_program() {
    // naming.namespace.original-counted-tail-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
    // Proof: naming.namespace.original-counted-tail-compiler-and-runtime. The three
    // C8.6+ disassemblies share this program; C8.4/8.5 invoke the worker.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let context = emit(&registry, &entry);
        assert!(!context.native_dependency_refusal, "{engine}");
        let operations: Vec<_> = context.instructions.iter().map(|item| item.op).collect();
        if entry.execution_point.unwrap().tcl_version().unwrap() < tcl_dialect::TclVersion::V8_6 {
            assert!(!operations.contains(&Op::STR_RFIND), "{engine}");
            assert!(!operations.contains(&Op::STR_RANGE), "{engine}");
            assert!(
                operations
                    .iter()
                    .any(|op| matches!(op, Op::INVOKE_STK1 | Op::INVOKE_STK4))
            );
            continue;
        }
        let search = operations
            .iter()
            .position(|op| *op == Op::STR_RFIND)
            .unwrap();
        assert_eq!(operations[search - 1], Op::OVER, "{engine}");
        assert_eq!(operations[search + 1], Op::DUP, "{engine}");
        assert_eq!(operations[search + 3], Op::GE, "{engine}");
        assert!(matches!(
            operations[search + 4],
            Op::JUMP_FALSE1 | Op::JUMP_FALSE4
        ));
        assert_eq!(operations[search + 6], Op::ADD, "{engine}");
        assert_eq!(operations[search + 8], Op::STR_RANGE, "{engine}");
        assert_eq!(
            context.instructions[search].native_switch_version,
            entry.execution_point.unwrap().tcl_version(),
            "{engine}"
        );
    }
}

#[test]
fn original_namespace_tail_unknown_worker_withdraws_the_compiler_program() {
    // naming.namespace.original-counted-tail-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
    use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
    let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let mut entry = crate::environment_ingress::captured_native_entry(profile);
    let selected = entry
        .lookup_command_bytes(entry.current_namespace, b"::tcl::namespace::tail")
        .unwrap()
        .unwrap()
        .token;
    entry
        .commands
        .iter_mut()
        .find(|command| command.token == selected)
        .unwrap()
        .compiler_hook = NativeCompilerHookPresence::Unknown;
    let context = emit(&registry, &entry);
    assert!(context.native_dependency_refusal);
    assert!(
        !context
            .instructions
            .iter()
            .any(|item| matches!(item.op, Op::STR_RFIND | Op::STR_RANGE))
    );
}

#[test]
fn original_namespace_qualifiers_emits_the_observed_counted_compiler_loop() {
    // naming.namespace.original-counted-qualifiers-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
    // Proof: naming.namespace.original-counted-qualifiers-compiler-and-runtime.
    // The C8.6+ disassemblies retain the back edge that strips a colon run.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let context = emit_source(&registry, &entry, b"namespace qualifiers $value");
        assert!(!context.native_dependency_refusal, "{engine}");
        let operations: Vec<_> = context.instructions.iter().map(|item| item.op).collect();
        let version = entry.execution_point.unwrap().tcl_version().unwrap();
        if version < tcl_dialect::TclVersion::V8_6 {
            assert!(!operations.contains(&Op::STR_RFIND), "{engine}");
            assert!(!operations.contains(&Op::STR_INDEX), "{engine}");
            assert!(
                operations
                    .iter()
                    .any(|op| matches!(op, Op::INVOKE_STK1 | Op::INVOKE_STK4))
            );
            continue;
        }
        let search = operations
            .iter()
            .position(|op| *op == Op::STR_RFIND)
            .unwrap();
        assert_eq!(operations[search - 1], Op::OVER, "{engine}");
        assert_eq!(operations[search + 2], Op::SUB, "{engine}");
        assert_eq!(operations[search + 3], Op::OVER, "{engine}");
        assert_eq!(operations[search + 4], Op::OVER, "{engine}");
        assert_eq!(operations[search + 5], Op::STR_INDEX, "{engine}");
        assert_eq!(operations[search + 7], Op::STR_EQ, "{engine}");
        assert!(matches!(
            operations[search + 8],
            Op::JUMP_TRUE1 | Op::JUMP_TRUE4
        ));
        assert_eq!(operations[search + 9], Op::STR_RANGE, "{engine}");
        for at in [search, search + 5, search + 7, search + 9] {
            assert_eq!(
                context.instructions[at].native_switch_version,
                Some(version),
                "{engine}"
            );
        }
    }
}
