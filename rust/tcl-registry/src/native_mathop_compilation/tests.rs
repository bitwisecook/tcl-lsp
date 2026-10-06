// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
use tcl_syntax::native_string::NativeStringProtocol;
fn recipe(
    source: &[u8],
    operator: NativeMathOperator,
    version: TclVersion,
    frame: NativeCompilationFrame,
) -> Option<NativeMathopInstruction> {
    let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
    let native = native_script_words_in(
        SourceImage::native(source),
        Span::new(0, u32::try_from(source.len()).unwrap()),
        LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap()
    .commands
    .remove(0)
    .words;
    let words = NativeCompilerWords::capture(&native, NativeStringProtocol::C(version)).unwrap();
    compile_native_mathop(
        &words,
        1,
        operator,
        version,
        NativeCompilationContext {
            frame,
            ..NativeCompilationContext::default()
        },
    )
    .unwrap()
}
#[test]
fn original_mathop_wrong_arity_declines_before_any_operand() {
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert!(
            recipe(
                b"{odd name}",
                NativeMathOperator::Not,
                version,
                NativeCompilationFrame::ProcedureCode
            )
            .is_none()
        );
        assert!(
            recipe(
                b"{odd name} [first] [second]",
                NativeMathOperator::Not,
                version,
                NativeCompilationFrame::ProcedureCode
            )
            .is_none()
        );
    }
}
#[test]
fn original_mathop_vacuous_comparison_does_not_visit_operand() {
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(
            recipe(
                b"cmp [unused]",
                NativeMathOperator::Less,
                version,
                NativeCompilationFrame::ProcedureCode
            )
            .unwrap()
            .steps,
            [NativeMathopStep::Literal(b"1".to_vec())]
        );
    }
}
#[test]
fn original_mathop_comparison_preserves_temporary_and_interleaved_visits() {
    use NativeMathopStep as S;
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let selected = recipe(
            b"cmp [one] [two] [three]",
            NativeMathOperator::Less,
            version,
            NativeCompilationFrame::ProcedureCode,
        )
        .unwrap();
        assert_eq!(
            &selected.steps[..8],
            &[
                S::DeclareTemporary,
                S::Word(NativeCompilerWordOperand::Original(1)),
                S::Word(NativeCompilerWordOperand::Original(2)),
                S::StoreTemporary,
                S::Primitive(NativeMathOperator::Less),
                S::LoadTemporary,
                S::Word(NativeCompilerWordOperand::Original(3)),
                S::Primitive(NativeMathOperator::Less)
            ]
        );
        assert_eq!(selected.steps[8], S::Primitive(NativeMathOperator::BitAnd));
        if version == TclVersion::V8_5 {
            assert_eq!(
                &selected.steps[9..],
                &[S::Literal(Vec::new()), S::StoreTemporary, S::Pop]
            );
        } else {
            assert_eq!(&selected.steps[9..], &[S::UnsetTemporary]);
        }
        assert!(
            recipe(
                b"cmp [one] [two] [three]",
                NativeMathOperator::Less,
                version,
                NativeCompilationFrame::ScriptCode
            )
            .is_none()
        );
    }
}
#[test]
fn original_mathop_parser_expansion_retains_real_source_members() {
    let selected = recipe(
        b"sum {*}{1 2 3}",
        NativeMathOperator::Add,
        TclVersion::V9_1,
        NativeCompilationFrame::ProcedureCode,
    )
    .unwrap();
    assert_eq!(
        selected
            .steps
            .iter()
            .filter(|step| matches!(
                step,
                NativeMathopStep::Word(NativeCompilerWordOperand::LiteralExpansion {
                    original_word: 1,
                    ..
                })
            ))
            .count(),
        3
    );
    assert!(
        recipe(
            b"sum {*}$values",
            NativeMathOperator::Add,
            TclVersion::V9_1,
            NativeCompilationFrame::ProcedureCode
        )
        .is_none()
    );
}
#[test]
fn registry_mathop_registration_retains_operator_identity_and_hook_floor() {
    let registry = crate::CommandRegistry::build_default();
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let dialect = crate::InvocationDialect::for_version(version);
        for spelling in [
            "~", "!", "+", "*", "&", "|", "^", "**", "<<", ">>", "%", "!=", "ne", "in", "ni", "-",
            "/", "<", "<=", ">", ">=", "==", "eq", "lt", "le", "gt", "ge",
        ] {
            let operator = NativeMathOperator::from_spelling(spelling).unwrap();
            let spec = registry.native_compilation_for_registration(
                &format!("::tcl::mathop::{spelling}"),
                dialect,
            );
            if version < operator.first_version() {
                assert!(spec.is_none());
                continue;
            }
            let spec = spec.unwrap();
            assert_eq!(
                spec.grammar,
                crate::native_compilation::NativeCompilationGrammar::MathOperator(operator)
            );
            assert_eq!(spec.compiler_hook_presence(dialect), Some(true));
            assert_eq!(
                spec.compiler_hook_presence(crate::InvocationDialect::for_version(
                    TclVersion::V8_4
                )),
                Some(false)
            );
        }
    }
    assert!(NativeMathOperator::from_spelling("&&").is_none());
}

#[test]
fn shared_mathop_factory_keeps_original_operator_after_command_rename() {
    let source = b"{odd name} [one] [two] [three]";
    let version = TclVersion::V9_1;
    let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
    let native = native_script_words_in(
        SourceImage::native(source.as_slice()),
        Span::new(0, u32::try_from(source.len()).unwrap()),
        LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap()
    .commands
    .remove(0)
    .words;
    let words = NativeCompilerWords::capture(&native, NativeStringProtocol::C(version)).unwrap();
    let dialect = crate::InvocationDialect::for_version(version);
    let spec = crate::CommandRegistry::build_default()
        .native_compilation_for_registration("::tcl::mathop::+", dialect)
        .unwrap();
    let context = NativeCompilationContext {
        frame: NativeCompilationFrame::ProcedureCode,
        ..NativeCompilationContext::default()
    };
    let selection = spec.select_native_words(&words, 1, Some(dialect), context);
    let plan = crate::native_instruction_plan::native_instruction_plan(
        spec, selection, &words, 1, dialect, context,
    )
    .unwrap();
    let crate::native_instruction_plan::NativeInstructionPlan::MathOperator(recipe) = plan else {
        panic!("genuine mathop original instruction");
    };
    assert_eq!(
        recipe,
        compile_native_mathop(&words, 1, NativeMathOperator::Add, version, context)
            .unwrap()
            .unwrap()
    );
    assert!(recipe.steps.contains(&NativeMathopStep::Reverse(3)));
}
