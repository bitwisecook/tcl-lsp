// SPDX-License-Identifier: AGPL-3.0-or-later
//! The measured non-strict ASCII branch of C's `StringIs` compiler.
//!
//! C8.6/9.0/9.1 compile exactly one original subject followed by `STR_CLASS`
//! ASCII. Class/options outside this branch stay unknown: their native
//! compiler can have different getters, preparation, errors and instructions.

use crate::SemanticOperationId;
use crate::native_compilation::{
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationWordShape as Shape,
    NativeCompilerImplementationLookup,
};
use crate::native_compiler_word_projection::project_native_compiler_words;
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_scalar_compilation::{
    NativeScalarInstruction, NativeScalarOperation, NativeScalarScope,
};
use tcl_dialect::TclVersion;

/// The real C8.5+ private worker; its compiler hook arrives in C8.6.
pub const STRING_IS_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "is",
        slot: "::tcl::string::is",
        command: "string",
        prepended: &["is"],
    };

fn simple(shape: Shape) -> bool {
    matches!(
        shape,
        Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral
    )
}

/// Select only the exact class/value branch exercised by the pack hook.
#[must_use]
pub fn select_shapes(
    shapes: &[Shape],
    member: Option<&[u8]>,
    class: Option<&[u8]>,
    scope: NativeScalarScope,
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_6 {
        return Selection::Generic;
    }
    let from = usize::from(scope == NativeScalarScope::PublicMember);
    if from == 1 && !(shapes.first().is_some_and(|shape| simple(*shape)) && member == Some(b"is")) {
        return Selection::Unknown;
    }
    let Some(arguments) = shapes.get(from..) else {
        return Selection::Unknown;
    };
    if arguments.len() != 2
        || !simple(arguments[0])
        || class != Some(b"ascii")
        || arguments
            .iter()
            .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded))
    {
        return Selection::Unknown;
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

/// C9 declines a direct private StringIs compiler before emitting any word
/// when its class token requires runtime substitution. Public ensemble
/// fallback has a different recipe and must not use this proof.
pub(crate) fn private_dynamic_class_declines(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
    operation: SemanticOperationId,
    body: crate::native_compilation::NativeBodyCompilation,
    context: crate::native_compilation::NativeCompilationContext,
) -> bool {
    if body != crate::native_compilation::NativeBodyCompilation::Inherit
        || context.mode != crate::native_compilation::NativeCompilationMode::BytecodeObject
        || scope != NativeScalarScope::PrivateOperands
        || version != TclVersion::V9_0
        || operand_from != 1
        || operation != SemanticOperationId::Intrinsic(crate::IntrinsicId::StringIs)
        || words.source_protocol() != tcl_syntax::native_string::NativeStringProtocol::C(version)
    {
        return false;
    }
    let Some(arguments) = words.shapes().get(operand_from..) else {
        return false;
    };
    (2..=5).contains(&arguments.len())
        && arguments[0] == Shape::Substituted
        && arguments
            .iter()
            .all(|shape| !matches!(shape, Shape::Opaque | Shape::Expanded))
}

/// Retain source operands independently of names or computed argv values.
#[must_use]
pub fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if operand_from == 0 {
        return NativeCompilationSelection::Unknown;
    }
    let Some(original_shapes) = words.shapes().get(operand_from..) else {
        return NativeCompilationSelection::Unknown;
    };
    // Static and dynamic expansions are deliberately outside this narrow model.
    if original_shapes
        .iter()
        .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded))
    {
        return NativeCompilationSelection::Unknown;
    }
    let Ok(projected) = project_native_compiler_words(words, version) else {
        return NativeCompilationSelection::Unknown;
    };
    let Some(arguments) = projected.get(operand_from..) else {
        return NativeCompilationSelection::Unknown;
    };
    let from = usize::from(scope == NativeScalarScope::PublicMember);
    let shapes: Vec<_> = arguments.iter().map(|word| word.shape).collect();
    select_shapes(
        &shapes,
        arguments.first().and_then(|word| word.literal.as_deref()),
        arguments.get(from).and_then(|word| word.literal.as_deref()),
        scope,
        version,
        operation,
    )
}

/// `CompileWord(subject)` once; the compile-known class is never a runtime operand.
#[must_use]
pub fn instruction(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
) -> Option<NativeScalarInstruction> {
    if !matches!(
        select_original(
            words,
            operand_from,
            scope,
            version,
            SemanticOperationId::Intrinsic(crate::IntrinsicId::StringIs)
        ),
        NativeCompilationSelection::Inline { .. }
    ) {
        return None;
    }
    let projected = project_native_compiler_words(words, version).ok()?;
    let from = operand_from + usize::from(scope == NativeScalarScope::PublicMember);
    Some(NativeScalarInstruction {
        operation: NativeScalarOperation::StringIsAscii,
        operands: vec![projected.get(from + 1)?.operand.clone()],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::{
        NativeCompilationContext, NativeCompilationGrammar, NativeCompilationMode,
    };
    use crate::native_compiler_word_projection::NativeCompilerWordOperand;
    use crate::native_instruction_plan::{NativeInstructionPlan, native_instruction_plan};
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    fn words(source: &[u8], version: TclVersion) -> Vec<tcl_lexer::NativeWord> {
        // Keep original source words, not a generated argv spelling.
        // This source-geometry fixture grants no live registration authority.
        let profile = tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap();
        let mut parsed = native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        parsed.commands.remove(0).words
    }

    #[test]
    fn c90_private_dynamic_class_declines_with_all_original_word_visits() {
        use crate::native_compilation::{
            NativeBodyCompilation, NativeCompilationSpec, NativeOriginalCompilerNameEffects,
        };
        use crate::native_control_compilation::NativeControlPreparationStep as Step;
        let version = TclVersion::V9_0;
        let dialect = crate::InvocationDialect::for_version(version);
        let private = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::StringIsAscii(NativeScalarScope::PrivateOperands),
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::StringIs),
            body: NativeBodyCompilation::Inherit,
        };
        let public = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::StringIsAscii(NativeScalarScope::PublicMember),
            ..private
        };
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            ..NativeCompilationContext::default()
        };
        for source in [
            b"::tcl::string::is $c $s".as_slice(),
            b"::tcl::string::is [class] -strict [subject]",
            b"::tcl::string::is $c -failindex index $s",
            b"::tcl::string::is $c -strict -failindex index $s",
        ] {
            let source_words = words(source, version);
            let captured =
                NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version))
                    .unwrap();
            let selected = private.select_native_words(&captured, 1, Some(dialect), context);
            assert_eq!(selected, NativeCompilationSelection::Generic, "{source:?}");
            let visits = (0..source_words.len())
                .map(|index| Step::Word(NativeCompilerWordOperand::Original(index)))
                .collect();
            assert_eq!(
                private.original_argument_name_effects_in_context(
                    &captured,
                    1,
                    Some(dialect),
                    context
                ),
                NativeOriginalCompilerNameEffects::Visits(visits)
            );
            assert_eq!(
                native_instruction_plan(private, selected, &captured, 1, dialect, context),
                Ok(NativeInstructionPlan::GenericPreparation(Vec::new()))
            );
        }
        for source in [
            b"string is $c $s".as_slice(),
            b"string is $c -strict $s",
            b"string is $c -failindex index $s",
        ] {
            let source_words = words(source, version);
            let captured =
                NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version))
                    .unwrap();
            assert_eq!(
                public.select_native_words(&captured, 1, Some(dialect), context),
                NativeCompilationSelection::Unknown
            );
            assert_eq!(
                private.select_native_words(&captured, 2, Some(dialect), context),
                NativeCompilationSelection::Unknown
            );
        }
        for source in [
            b"::tcl::string::is $c".as_slice(),
            b"::tcl::string::is $c {*}$s",
            b"::tcl::string::is ascii $s",
        ] {
            let source_words = words(source, version);
            let captured =
                NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version))
                    .unwrap();
            assert_ne!(
                private.select_native_words(&captured, 1, Some(dialect), context),
                NativeCompilationSelection::Generic
            );
        }
        for version in [TclVersion::V8_6, TclVersion::V9_1] {
            let source_words = words(b"::tcl::string::is $c $s", version);
            let captured =
                NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version))
                    .unwrap();
            assert_eq!(
                private.select_native_words(
                    &captured,
                    1,
                    Some(crate::InvocationDialect::for_version(version)),
                    context
                ),
                NativeCompilationSelection::Unknown
            );
        }
    }

    #[test]
    fn private_dynamic_decline_rejects_foreign_body_and_compiler_context() {
        use crate::native_compilation::{
            NativeBodyCompilation, NativeCompilationSpec, NativeOriginalCompilerNameEffects,
        };
        let version = TclVersion::V9_0;
        let dialect = crate::InvocationDialect::for_version(version);
        let source_words = words(b"::tcl::string::is $c $s", version);
        let captured =
            NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version)).unwrap();
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::StringIsAscii(NativeScalarScope::PrivateOperands),
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::StringIs),
            body: NativeBodyCompilation::Inherit,
        };
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            ..NativeCompilationContext::default()
        };
        for body in [
            NativeBodyCompilation::Direct,
            NativeBodyCompilation::ScriptObject,
            NativeBodyCompilation::ProcedureObject,
        ] {
            let foreign = NativeCompilationSpec { body, ..spec };
            assert_eq!(
                foreign.select_native_words(&captured, 1, Some(dialect), context),
                NativeCompilationSelection::Unknown
            );
            assert_eq!(
                foreign.original_argument_name_effects_in_context(
                    &captured,
                    1,
                    Some(dialect),
                    context
                ),
                NativeOriginalCompilerNameEffects::Unknown
            );
        }
        let unknown = NativeCompilationContext {
            mode: NativeCompilationMode::Unknown,
            ..context
        };
        assert_eq!(
            spec.select_native_words(&captured, 1, Some(dialect), unknown),
            NativeCompilationSelection::Unknown
        );
        assert_eq!(
            spec.original_argument_name_effects_in_context(&captured, 1, Some(dialect), unknown),
            NativeOriginalCompilerNameEffects::Unknown
        );
        let direct = NativeCompilationContext {
            mode: NativeCompilationMode::Direct,
            ..context
        };
        assert!(!private_dynamic_class_declines(
            &captured,
            1,
            NativeScalarScope::PrivateOperands,
            version,
            spec.operation,
            spec.body,
            direct
        ));
        assert!(
            native_instruction_plan(
                spec,
                NativeCompilationSelection::Generic,
                &captured,
                1,
                dialect,
                direct
            )
            .is_err()
        );
        let foreign_protocol =
            NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(TclVersion::V8_6))
                .unwrap();
        assert!(!private_dynamic_class_declines(
            &foreign_protocol,
            1,
            NativeScalarScope::PrivateOperands,
            version,
            spec.operation,
            spec.body,
            context
        ));
    }

    #[test]
    fn a_real_private_descriptor_keeps_the_single_original_subject() {
        let registry = crate::CommandRegistry::build_default();
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let dialect = crate::InvocationDialect::for_version(version);
            let private = registry
                .native_compilation_for_registration("::tcl::string::is", dialect)
                .expect("the actual private StringIs compiler has a descriptor");
            assert_eq!(
                private.grammar,
                NativeCompilationGrammar::StringIsAscii(NativeScalarScope::PrivateOperands)
            );
            assert_eq!(private.compiler_hook_presence(dialect), Some(true));
            let public = registry
                .get("string")
                .unwrap()
                .resolve_subcommand("is")
                .unwrap()
                .native_compilation
                .expect("public member compiler");
            assert_eq!(
                public.implementation_prerequisites(dialect).unwrap(),
                vec![STRING_IS_IMPLEMENTATION]
            );
            let context = NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                ..NativeCompilationContext::default()
            };
            for (source, spec, subject) in [
                (b"renamed is ascii $subject".as_slice(), public, 3),
                (b"renamed ascii $subject".as_slice(), private, 2),
            ] {
                let source_words = words(source, version);
                let original =
                    NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version))
                        .unwrap();
                let selection = spec.select_native_words(&original, 1, Some(dialect), context);
                assert_eq!(
                    native_instruction_plan(spec, selection, &original, 1, dialect, context),
                    Ok(NativeInstructionPlan::Scalar(NativeScalarInstruction {
                        operation: NativeScalarOperation::StringIsAscii,
                        operands: vec![NativeCompilerWordOperand::Original(subject)],
                    })),
                    "{version:?}: {source:?}"
                );
                assert_eq!(
                    select_original(
                        &original,
                        0,
                        NativeScalarScope::PrivateOperands,
                        version,
                        SemanticOperationId::Intrinsic(crate::IntrinsicId::StringIs)
                    ),
                    NativeCompilationSelection::Unknown
                );
            }
        }
        assert!(
            registry
                .native_compilation_for_registration(
                    "::tcl::string::not_is",
                    crate::InvocationDialect::for_version(TclVersion::V9_0)
                )
                .is_none()
        );
        let c85 = crate::InvocationDialect::for_version(TclVersion::V8_5);
        assert_eq!(
            registry
                .native_compilation_for_registration("::tcl::string::is", c85)
                .unwrap()
                .compiler_hook_presence(c85),
            Some(false)
        );
    }

    #[test]
    fn unmodeled_original_class_options_and_expansions_keep_refusing() {
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            for source in [
                b"string is ascii -strict $subject".as_slice(),
                b"string is ascii -failindex out $subject",
                b"string is alpha $subject",
                b"string is $class $subject",
                b"string is asc $subject",
                b"string is ascii {*}$subject",
                b"string is {*}{ascii value}",
            ] {
                let source_words = words(source, version);
                let original =
                    NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version))
                        .unwrap();
                assert_eq!(
                    select_original(
                        &original,
                        1,
                        NativeScalarScope::PublicMember,
                        version,
                        SemanticOperationId::Intrinsic(crate::IntrinsicId::StringIs)
                    ),
                    NativeCompilationSelection::Unknown,
                    "{version:?}: {source:?}"
                );
            }
        }
    }
    #[test]
    fn old_or_missing_compiler_point_cannot_make_a_native_plan() {
        let registry = crate::CommandRegistry::build_default();
        let spec = registry
            .get("string")
            .unwrap()
            .resolve_subcommand("is")
            .unwrap()
            .native_compilation
            .unwrap();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            ..NativeCompilationContext::default()
        };
        for version in [TclVersion::V8_4, TclVersion::V8_5] {
            let source_words = words(b"string is ascii $subject", version);
            let original =
                NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version))
                    .unwrap();
            let dialect = crate::InvocationDialect::for_version(version);
            let selection = spec.select_native_words(&original, 1, Some(dialect), context);
            assert!(!matches!(
                selection,
                NativeCompilationSelection::Inline { .. }
            ));
            assert!(
                native_instruction_plan(spec, selection, &original, 1, dialect, context).is_err()
            );
        }
        let version = TclVersion::V9_0;
        let source_words = words(b"string is ascii $subject", version);
        let original =
            NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version)).unwrap();
        assert_eq!(
            spec.select_native_words(&original, 1, None, context),
            NativeCompilationSelection::Unknown
        );
        let dialect = crate::InvocationDialect::for_version(version);
        let selection = spec.select_native_words(&original, 0, Some(dialect), context);
        assert_eq!(selection, NativeCompilationSelection::Unknown);
        assert!(native_instruction_plan(spec, selection, &original, 0, dialect, context).is_err());
    }
}
