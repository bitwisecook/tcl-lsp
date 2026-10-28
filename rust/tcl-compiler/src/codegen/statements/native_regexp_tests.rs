// SPDX-License-Identifier: AGPL-3.0-or-later
//! C9 regexp compiler emission against the release disassembly.

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
    let mut context = CodegenCtx::new(true, &["re", "s"], registry);
    context.native_entry = Some(entry);
    context.invocation_dialect = Some(dialect);
    context.source_string_protocol = entry.source_string_protocol;
    context.compiled_variable_protocol = entry.compiled_variable_protocol;
    context.native_compilation.frame =
        tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
    context.emit_native_words(&parsed.commands[0].words);
    context
}

fn literal(context: &CodegenCtx<'_>, instruction: &tcl_bytecode::Instruction) -> Vec<u8> {
    let [Operand::Imm(index)] = instruction.operands.as_slice() else {
        panic!("{instruction:?}");
    };
    context.literals.entries()[usize::try_from(*index).unwrap()]
        .bytes()
        .to_vec()
}

#[derive(Debug)]
enum Pattern {
    Literal(&'static [u8]),
    Local,
}

// Expectations are the tclsh 9.0.4 `disassemble lambda {re s} ...` listings:
// the pattern push or load, `loadScalar1 %v1`, then the matcher.
#[test]
fn c9_regexp_emission_matches_the_release_disassembly() {
    let cases: [(&[u8], Pattern, Op, &[i32]); 10] = [
        (b"regexp -- $re $s", Pattern::Local, Op::REGEXP, &[3]),
        (b"regexp -nocase $re $s", Pattern::Local, Op::REGEXP, &[11]),
        (
            b"regexp abc $s",
            Pattern::Literal(b"*abc*"),
            Op::STR_MATCH,
            &[0],
        ),
        (
            b"regexp -n abc $s",
            Pattern::Literal(b"*abc*"),
            Op::STR_MATCH,
            &[1],
        ),
        (
            b"regexp {^abc$} $s",
            Pattern::Literal(b"abc"),
            Op::STR_EQ,
            &[],
        ),
        (
            b"regexp -nocase {^abc$} $s",
            Pattern::Literal(b"abc"),
            Op::STR_MATCH,
            &[1],
        ),
        (
            b"regexp ^abc$ $s",
            Pattern::Literal(b"^abc$"),
            Op::REGEXP,
            &[3],
        ),
        (
            b"regexp -nocase ^abc$ $s",
            Pattern::Literal(b"^abc$"),
            Op::REGEXP,
            &[11],
        ),
        (b"regexp {a+} $s", Pattern::Literal(b"a+"), Op::REGEXP, &[3]),
        (
            b"regexp -- -abc $s",
            Pattern::Literal(b"*-abc*"),
            Op::STR_MATCH,
            &[0],
        ),
    ];
    for engine in ["tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry: crate::environment_ingress::CapturedNativeEntry =
            crate::environment_ingress::captured_native_entry(profile);
        let version = entry.execution_point.unwrap().tcl_version().unwrap();
        for (source, pattern, op, immediates) in &cases {
            let name = String::from_utf8_lossy(source);
            let context = emit(&registry, &entry, source);
            assert!(!context.native_dependency_refusal, "{engine}/{name}");
            let instructions = &context.instructions;
            assert!(
                !instructions
                    .iter()
                    .any(|item| matches!(item.op, Op::INVOKE_STK1 | Op::INVOKE_STK4)),
                "{engine}/{name}"
            );
            let at = instructions
                .iter()
                .position(|item| matches!(item.op, Op::REGEXP | Op::STR_MATCH | Op::STR_EQ))
                .unwrap_or_else(|| panic!("{engine}/{name}: {instructions:?}"));
            let matcher = &instructions[at];
            assert_eq!(matcher.op, *op, "{engine}/{name}");
            let expected: Vec<_> = immediates.iter().copied().map(Operand::Imm).collect();
            assert_eq!(matcher.operands, expected, "{engine}/{name}");
            assert_eq!(
                matcher.native_switch_version,
                Some(version),
                "{engine}/{name}"
            );
            let subject = &instructions[at - 1];
            assert!(
                matches!(subject.op, Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4),
                "{engine}/{name}"
            );
            assert_eq!(subject.operands, [Operand::Imm(1)], "{engine}/{name}");
            let first = &instructions[at - 2];
            match pattern {
                Pattern::Local => {
                    assert!(
                        matches!(first.op, Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4),
                        "{engine}/{name}"
                    );
                    assert_eq!(first.operands, [Operand::Imm(0)], "{engine}/{name}");
                }
                Pattern::Literal(bytes) => {
                    assert!(matches!(first.op, Op::PUSH1 | Op::PUSH4), "{engine}/{name}");
                    assert_eq!(literal(&context, first), *bytes, "{engine}/{name}");
                }
            }
        }
    }
}

// tclsh 9.0.4 pushes "regexp" and invokes the command for these shapes.
#[test]
fn c9_regexp_declined_shapes_invoke_the_command() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    for source in [
        b"regexp -abc $s".as_slice(),
        b"regexp -all abc $s",
        b"regexp abc $s re",
    ] {
        let name = String::from_utf8_lossy(source);
        let context = emit(&registry, &entry, source);
        assert!(!context.native_dependency_refusal, "{name}");
        assert!(
            context
                .instructions
                .iter()
                .any(|item| matches!(item.op, Op::INVOKE_STK1 | Op::INVOKE_STK4)),
            "{name}"
        );
        assert!(
            !context
                .instructions
                .iter()
                .any(|item| matches!(item.op, Op::REGEXP | Op::STR_MATCH | Op::STR_EQ)),
            "{name}"
        );
    }
}
