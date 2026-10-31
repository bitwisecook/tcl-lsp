// SPDX-License-Identifier: AGPL-3.0-or-later
//! Clock compiler emission against Tcl 9.0.4 disassembly.

use super::*;

#[test]
fn c90_clock_read_matches_native_disassembly() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    let dialect = tcl_registry::InvocationDialect::of_point(entry.execution_point.unwrap());
    for (source, expected) in [
        (b"clock clicks -mic".as_slice(), 1),
        (b"clock clicks -mil".as_slice(), 2),
        (b"clock seconds".as_slice(), 3),
    ] {
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let mut context = CodegenCtx::new(true, &[], &registry);
        context.native_entry = Some(&entry);
        context.invocation_dialect = Some(dialect);
        context.source_string_protocol = entry.source_string_protocol;
        context.compiled_variable_protocol = entry.compiled_variable_protocol;
        context.native_compilation.frame =
            tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
        context.emit_native_words(&parsed.commands[0].words);
        assert!(!context.native_dependency_refusal, "{:?}", source);
        let reads: Vec<_> = context
            .instructions
            .iter()
            .filter(|instruction| instruction.op == Op::CLOCK_READ)
            .collect();
        assert_eq!(reads.len(), 1, "{:?}", source);
        assert_eq!(reads[0].operands, [Operand::Imm(expected)]);
    }
}
