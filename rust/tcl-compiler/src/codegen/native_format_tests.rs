// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original format compiler instruction and literal routes.

use super::*;

#[test]
fn original_format_instruction_keeps_single_value_and_mixed_piece_order() {
    for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        for (source, concatenates) in [
            (b"format {%s} $a".as_slice(), false),
            (b"format {A%s%%B%s} $a $b".as_slice(), true),
        ] {
            let dialect = tcl_registry::InvocationDialect::of_point(entry.execution_point.unwrap());
            let image = tcl_lexer::SourceImage::native(source);
            let parsed = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            )
            .unwrap();
            let mut context = CodegenCtx::new(true, &["a", "b"], &registry);
            context.native_entry = Some(&entry);
            context.invocation_dialect = Some(dialect);
            context.source_string_protocol = entry.source_string_protocol;
            context.compiled_variable_protocol = entry.compiled_variable_protocol;
            context.native_compilation.frame =
                tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
            context.emit_native_words(&parsed.commands[0].words);
            assert!(!context.native_dependency_refusal, "{name}: {source:?}");
            let counts = context
                .instructions
                .iter()
                .filter(|instruction| instruction.op == Op::STR_CONCAT1)
                .map(|instruction| instruction.operands.clone())
                .collect::<Vec<_>>();
            assert_eq!(
                counts,
                if concatenates {
                    vec![vec![Operand::Imm(4)]]
                } else {
                    vec![]
                },
                "{name}: {source:?}"
            );
            if concatenates {
                let fragments = context
                    .literals
                    .entries()
                    .iter()
                    .filter(|literal| literal.bytes() == b"A" || literal.bytes() == b"%B")
                    .map(|literal| literal.bytes())
                    .collect::<Vec<_>>();
                assert_eq!(fragments, vec![b"A".as_slice(), b"%B".as_slice()]);
            }
        }
    }
}

#[test]
fn original_c90_hex_decline_keeps_ordinary_invocation_emission() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let entry = crate::environment_ingress::captured_native_entry(profile);
    let dialect = tcl_registry::InvocationDialect::of_point(entry.execution_point.unwrap());
    let image = tcl_lexer::SourceImage::native(b"format %x $x".as_slice());
    let parsed = tcl_lexer::native_script_words_in(
        image.clone(),
        tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    )
    .unwrap();
    let mut context = CodegenCtx::new(true, &["x"], &registry);
    context.native_entry = Some(&entry);
    context.invocation_dialect = Some(dialect);
    context.source_string_protocol = entry.source_string_protocol;
    context.compiled_variable_protocol = entry.compiled_variable_protocol;
    context.native_compilation.frame =
        tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
    context.emit_native_words(&parsed.commands[0].words);
    assert!(!context.native_dependency_refusal);
    let invokes = context
        .instructions
        .iter()
        .filter(|instruction| instruction.op == Op::INVOKE_STK1)
        .map(|instruction| instruction.operands.clone())
        .collect::<Vec<_>>();
    assert_eq!(invokes, vec![vec![Operand::Imm(3)]]);
    assert!(
        !context
            .instructions
            .iter()
            .any(|instruction| instruction.op == Op::STR_CONCAT1)
    );
}
