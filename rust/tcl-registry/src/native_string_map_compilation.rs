// SPDX-License-Identifier: AGPL-3.0-or-later
//! The C9 private String-map compiler's `Basic2Arg` fallback.
//!
//! `TclCompileStringMapCmd` parses the compile-known map in a fresh private
//! temporary. A non-two-element list selects `TclCompileBasic2ArgCmd` after
//! releasing that temporary. The inline two-element path stays unresolved.

use crate::native_compilation::{
    NativeCompilationSelection, NativeCompilationWordShape as Shape,
    NativeCompilerImplementationLookup, NativeNamedInvocationProtocol,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_scalar_compilation::NativeScalarScope;
use tcl_dialect::TclVersion;

pub(crate) const STRING_MAP_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "map",
        slot: "::tcl::string::map",
        command: "string",
        prepended: &["map"],
    };

pub(crate) fn select(
    shapes: &[Shape],
    member: Option<&[u8]>,
    map: Option<&[u8]>,
    scope: NativeScalarScope,
    version: TclVersion,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version != TclVersion::V9_0 {
        return Selection::Unknown;
    }
    let from = usize::from(scope == NativeScalarScope::PublicMember);
    if from == 1
        && (!shapes.first().is_some_and(|shape| {
            matches!(
                shape,
                Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral
            )
        }) || member != Some(b"map"))
    {
        return Selection::Unknown;
    }
    let Some(operands) = shapes.get(from..) else {
        return Selection::Unknown;
    };
    if operands.len() != 2
        || operands
            .iter()
            .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded))
    {
        return Selection::Unknown;
    }
    let Some(map) = map else {
        return Selection::Unknown;
    };
    let Ok(elements) = tcl_syntax::list::split_native_list_bytes(
        map,
        tcl_syntax::native_string::NativeStringProtocol::C(version),
    ) else {
        return Selection::Unknown;
    };
    if elements.len() == 2 {
        return Selection::Unknown;
    }
    Selection::NamedInvocation {
        lookup: &STRING_MAP_IMPLEMENTATION,
        arguments_from: from,
        protocol: NativeNamedInvocationProtocol::Direct,
    }
}

pub(crate) fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
) -> NativeCompilationSelection {
    if words.source_protocol() != tcl_syntax::native_string::NativeStringProtocol::C(version) {
        return NativeCompilationSelection::Unknown;
    }
    let Some(shapes) = operand_from
        .checked_sub(1)
        .and_then(|_| words.shapes().get(operand_from..))
    else {
        return NativeCompilationSelection::Unknown;
    };
    let from = usize::from(scope == NativeScalarScope::PublicMember);
    select(
        shapes,
        words.literal(operand_from),
        words.literal(operand_from + from),
        scope,
        version,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::{
        NativeCompilationContext, NativeCompilationSpec, NativeOriginalCompilerNameEffects,
    };

    fn selected(
        source: &str,
        version: TclVersion,
        scope: NativeScalarScope,
    ) -> NativeCompilationSelection {
        let dialect = crate::InvocationDialect::for_version(version);
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source.as_bytes()),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            dialect.native_string_protocol().unwrap(),
        )
        .unwrap();
        select_original(&words, 1, scope, version)
    }

    #[test]
    fn non_pair_mapping_keeps_the_actual_private_worker_and_original_values() {
        for source in [
            "string map {a b c d} $s",
            "string map {} $s",
            "string map {a} [side_effect]",
            "string map {a b c} $s",
        ] {
            assert!(
                matches!(selected(source, TclVersion::V9_0, NativeScalarScope::PublicMember), NativeCompilationSelection::NamedInvocation { lookup, arguments_from: 1, protocol: NativeNamedInvocationProtocol::Direct } if *lookup == STRING_MAP_IMPLEMENTATION)
            );
        }
        assert!(matches!(
            selected(
                "::tcl::string::map {a b c d} $s",
                TclVersion::V9_0,
                NativeScalarScope::PrivateOperands
            ),
            NativeCompilationSelection::NamedInvocation {
                arguments_from: 0,
                protocol: NativeNamedInvocationProtocol::Direct,
                ..
            }
        ));
    }

    #[test]
    fn inline_pair_options_unknown_words_and_other_releases_stay_residual() {
        for source in [
            "string map {a b} $s",
            "string map {{} x} $s",
            "string map $mapping $s",
            "string map -nocase {a b c d} $s",
            "string map {a b c d}",
            "string map {a b c d} $s extra",
            "string map \"{\" $s",
            "string m {a b c d} $s",
        ] {
            assert_eq!(
                selected(source, TclVersion::V9_0, NativeScalarScope::PublicMember),
                NativeCompilationSelection::Unknown,
                "{source}"
            );
        }
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_1,
        ] {
            assert_eq!(
                selected(
                    "string map {a b c d} $s",
                    version,
                    NativeScalarScope::PublicMember
                ),
                NativeCompilationSelection::Unknown
            );
        }
    }

    #[test]
    fn named_lookup_precedes_the_existing_original_word_effect_visits() {
        let source = "string map {a b c d} [rename x y]";
        let dialect = crate::InvocationDialect::for_version(TclVersion::V9_0);
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source.as_bytes()),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            dialect.native_string_protocol().unwrap(),
        )
        .unwrap();
        let spec = NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringMap(
                NativeScalarScope::PublicMember,
            ),
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        };
        let NativeOriginalCompilerNameEffects::FixedLookupThenVisits { lookup, visits } = spec
            .original_argument_name_effects_in_context(
                &words,
                1,
                Some(dialect),
                NativeCompilationContext::default(),
            )
        else {
            panic!("selected Basic2Arg must retain both independent effects");
        };
        assert_eq!(*lookup, STRING_MAP_IMPLEMENTATION);
        for body in [
            crate::native_compilation::NativeBodyCompilation::ScriptObject,
            crate::native_compilation::NativeBodyCompilation::ProcedureObject,
            crate::native_compilation::NativeBodyCompilation::Direct,
        ] {
            let foreign = NativeCompilationSpec { body, ..spec };
            assert_eq!(
                foreign.original_argument_name_effects(&words, 1, Some(dialect)),
                NativeOriginalCompilerNameEffects::Unknown
            );
        }
        let foreign = NativeCompilationSpec {
            operation: crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::StringIs),
            ..spec
        };
        assert_eq!(
            foreign.original_argument_name_effects(&words, 1, Some(dialect)),
            NativeOriginalCompilerNameEffects::Unknown
        );
        assert_eq!(
            visits,
            vec![
                crate::native_control_compilation::NativeControlPreparationStep::Word(
                    crate::native_compiler_word_projection::NativeCompilerWordOperand::Original(2)
                ),
                crate::native_control_compilation::NativeControlPreparationStep::Word(
                    crate::native_compiler_word_projection::NativeCompilerWordOperand::Original(3)
                )
            ]
        );
        assert_eq!(
            select_original(&words, 0, NativeScalarScope::PublicMember, TclVersion::V9_0),
            NativeCompilationSelection::Unknown
        );
        assert_eq!(
            select_original(
                &words,
                usize::MAX,
                NativeScalarScope::PublicMember,
                TclVersion::V9_0
            ),
            NativeCompilationSelection::Unknown
        );
    }
}
