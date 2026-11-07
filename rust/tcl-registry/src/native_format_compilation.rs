// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-word selection for the native string-concatenation format compiler.

use tcl_dialect::TclVersion;

use crate::SemanticOperationId;
use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationGuard,
    NativeCompilationMode, NativeCompilationSelection, NativeCompilationWordShape,
};

/// C8.6+ declines before emitting any word when the format word itself needs
/// substitution.
pub(crate) fn dynamic_format_word_declines(
    words: &crate::native_compiler_words::NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: NativeCompilationContext,
    operation: SemanticOperationId,
    body: crate::native_compilation::NativeBodyCompilation,
) -> bool {
    use NativeCompilationWordShape as Shape;
    matches!(
        version,
        TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
    ) && context.mode == NativeCompilationMode::BytecodeObject
        && context.frame != NativeCompilationFrame::Unknown
        && operation == SemanticOperationId::Invoke
        && body == crate::native_compilation::NativeBodyCompilation::Inherit
        && words.source_protocol() == tcl_syntax::native_string::NativeStringProtocol::C(version)
        && operand_from == 1
        && words.shapes().len() >= 2
        && words.shapes()[1] == Shape::Substituted
        && words.literal(1).is_none()
        && words.shapes()[2..]
            .iter()
            .all(|shape| !matches!(shape, Shape::Opaque | Shape::Expanded))
}

/// [`select`] plus `TclCompileFormatCmd`'s all-literal branch: when every
/// argument is compile-known, C runs `Tcl_Format` and pushes its result.
/// A format that fails compiles to its syntax error, which stays unmodelled.
pub(crate) fn select_with_values(
    values: &dyn Fn(usize) -> Option<Vec<u8>>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    match static_result(values, shapes, version, context, operation) {
        StaticFormat::Folded(_) => NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        },
        StaticFormat::Unfolded => NativeCompilationSelection::Unknown,
        StaticFormat::NotStatic => {
            select(values(0).as_deref(), shapes, version, context, operation)
        }
    }
}

/// The all-literal branch's outcome.
enum StaticFormat {
    /// Some argument is substituted, or the context does not compile.
    NotStatic,
    /// `Tcl_Format`'s result.
    Folded(Vec<u8>),
    /// `Tcl_Format` fails or cannot be folded here.
    Unfolded,
}

/// Fold `format` when every argument is compile-known in a compiling context.
fn static_result(
    values: &dyn Fn(usize) -> Option<Vec<u8>>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
    operation: SemanticOperationId,
) -> StaticFormat {
    use NativeCompilationWordShape as Shape;
    if operation != SemanticOperationId::Invoke
        || !matches!(
            version,
            TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
        )
        || context.mode != NativeCompilationMode::BytecodeObject
        || context.frame == NativeCompilationFrame::Unknown
        || shapes.is_empty()
        || shapes
            .iter()
            .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded | Shape::Substituted))
    {
        return StaticFormat::NotStatic;
    }
    let Some(literals) = (0..shapes.len()).map(values).collect::<Option<Vec<_>>>() else {
        return StaticFormat::NotStatic;
    };
    let arguments = literals.iter().map(Vec::as_slice).collect::<Vec<_>>();
    crate::value_transfer::builtins::compile_time_format(version, &arguments)
        .map_or(StaticFormat::Unfolded, StaticFormat::Folded)
}

/// The folded literal of an all-literal `format`, as [`select_with_values`]
/// selects it.
pub(crate) fn static_instruction(
    values: &dyn Fn(usize) -> Option<Vec<u8>>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> Option<crate::native_instruction_plan::NativeFormatStringInstruction> {
    use crate::native_instruction_plan::{
        NativeFormatLiteralAllocation, NativeFormatStringInstruction, NativeFormatStringPiece,
    };
    let StaticFormat::Folded(folded) = static_result(
        values,
        shapes,
        version,
        context,
        SemanticOperationId::Invoke,
    ) else {
        return None;
    };
    // 8.6/9.0 PushLiteral; 9.1 PUSH_OBJ adds an unshared literal object.
    let literal_allocation = if version == TclVersion::V9_1 {
        NativeFormatLiteralAllocation::PrivateString
    } else {
        NativeFormatLiteralAllocation::Registered
    };
    Some(NativeFormatStringInstruction {
        pieces: vec![NativeFormatStringPiece::Literal(folded)],
        literal_allocation,
    })
}

pub(crate) fn select(
    format: Option<&[u8]>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    use NativeCompilationWordShape as Shape;
    if operation != SemanticOperationId::Invoke
        || !matches!(
            version,
            TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
        )
        || context.mode != NativeCompilationMode::BytecodeObject
        || context.frame == NativeCompilationFrame::Unknown
        || !(matches!(
            shapes.first(),
            Some(Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral)
        ) || (version == TclVersion::V9_0 && shapes.first() == Some(&Shape::BackslashLiteral)))
        || !shapes.contains(&Shape::Substituted)
        || shapes
            .iter()
            .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded))
    {
        return Selection::Unknown;
    }
    let Some(format) = format else {
        return Selection::Unknown;
    };
    let arguments = shapes.len() - 1;
    // The dynamic branch accepts only %s and %%, then compiles the value
    // words once in their original order; it prepares no script or expression.
    // Any other conversion, or a count mismatch, declines before emitting
    // anything and the words compile as an ordinary invocation.
    let mut conversions = 0;
    let mut bytes = format.iter();
    while let Some(byte) = bytes.next() {
        if *byte == b'%' {
            match bytes.next() {
                Some(b's') => conversions += 1,
                Some(b'%') => {}
                _ => return Selection::Generic,
            }
        }
    }
    if conversions != arguments || conversions > 125 {
        return Selection::Generic;
    }
    if !format.is_ascii() || format.contains(&0) {
        return Selection::Unknown;
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

pub(crate) fn instruction(
    words: &crate::native_compiler_words::NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: NativeCompilationContext,
) -> Option<crate::native_instruction_plan::NativeFormatStringInstruction> {
    use crate::native_instruction_plan::{
        NativeFormatLiteralAllocation, NativeFormatStringInstruction, NativeFormatStringPiece,
    };
    if operand_from == 0 {
        return None;
    }
    let shapes = words.shapes().get(operand_from..)?;
    if let Some(folded) = static_instruction(
        &|index| words.literal(operand_from + index).map(<[u8]>::to_vec),
        shapes,
        version,
        context,
    ) {
        return Some(folded);
    }
    let format = words.literal(operand_from)?;
    let shapes = words.shapes().get(operand_from..)?;
    if !matches!(
        select(
            Some(format),
            shapes,
            version,
            context,
            SemanticOperationId::Invoke
        ),
        NativeCompilationSelection::Inline { .. }
    ) {
        return None;
    }
    let literal_allocation = match version {
        TclVersion::V8_6 | TclVersion::V9_0 => NativeFormatLiteralAllocation::Registered,
        TclVersion::V9_1 => NativeFormatLiteralAllocation::PrivateString,
        _ => return None,
    };
    let mut pieces = Vec::new();
    let mut literal = Vec::new();
    let mut value = operand_from + 1;
    let mut bytes = format.iter().copied();
    while let Some(byte) = bytes.next() {
        if byte != b'%' {
            literal.push(byte);
            continue;
        }
        match bytes.next()? {
            b'%' => literal.push(b'%'),
            b's' => {
                if !literal.is_empty() {
                    pieces.push(NativeFormatStringPiece::Literal(std::mem::take(
                        &mut literal,
                    )));
                }
                words.original_words().get(value)?;
                pieces.push(NativeFormatStringPiece::Value(value));
                value += 1;
            }
            _ => return None,
        }
    }
    if !literal.is_empty() {
        pieces.push(NativeFormatStringPiece::Literal(literal));
    }
    Some(NativeFormatStringInstruction {
        pieces,
        literal_allocation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> NativeCompilationContext {
        NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..NativeCompilationContext::default()
        }
    }

    #[test]
    fn simple_dynamic_string_concatenation_keeps_the_native_selection_boundary() {
        use NativeCompilationWordShape as Shape;
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            for (format, shapes) in [
                (b"return [uplevel 1 [linsert $args 0 ::tmm::_orig_%s]]".as_slice(), vec![Shape::BracedLiteral, Shape::Substituted]),
                (b"error \"invalid command name \\\"%s\\\"\" \"invalid command name \\\"%s\\\"\"".as_slice(), vec![Shape::BracedLiteral, Shape::Substituted, Shape::Substituted]),
                (b"%s%%".as_slice(), vec![Shape::Literal, Shape::Substituted]),
                (b"%s%s".as_slice(), vec![Shape::QuotedLiteral, Shape::Literal, Shape::Substituted]),
            ] {
                assert!(matches!(select(Some(format), &shapes, version, context(), SemanticOperationId::Invoke), NativeCompilationSelection::Inline { operation: SemanticOperationId::Invoke, guard: NativeCompilationGuard::BeforeArguments }));
            }
        }
        let shapes = [Shape::Literal, Shape::Substituted];
        // TclCompileFormatCmd's scan returns TCL_ERROR for these: an ordinary invocation.
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            for format in [b"%u".as_slice(), b"%2$s", b"%5s", b"%", b"%s%s", b"%%"] {
                assert_eq!(
                    select(
                        Some(format),
                        &shapes,
                        version,
                        context(),
                        SemanticOperationId::Invoke
                    ),
                    NativeCompilationSelection::Generic
                );
            }
        }
        for format in [b"%s\0".as_slice(), b"\xff%s"] {
            assert_eq!(
                select(
                    Some(format),
                    &shapes,
                    TclVersion::V9_0,
                    context(),
                    SemanticOperationId::Invoke
                ),
                NativeCompilationSelection::Unknown
            );
        }
        assert_eq!(
            select(
                None,
                &shapes,
                TclVersion::V9_0,
                context(),
                SemanticOperationId::Invoke
            ),
            NativeCompilationSelection::Unknown
        );
        for shape in [
            Shape::Literal,
            Shape::BackslashLiteral,
            Shape::QuotedLiteral,
            Shape::BracedLiteral,
            Shape::Opaque,
            Shape::Expanded,
        ] {
            assert_eq!(
                select(
                    Some(b"%s"),
                    &[Shape::Literal, shape],
                    TclVersion::V9_0,
                    context(),
                    SemanticOperationId::Invoke
                ),
                NativeCompilationSelection::Unknown
            );
        }
        for version in [TclVersion::V8_4, TclVersion::V8_5] {
            assert_eq!(
                select(
                    Some(b"%s"),
                    &shapes,
                    version,
                    context(),
                    SemanticOperationId::Invoke
                ),
                NativeCompilationSelection::Unknown
            );
        }
    }

    #[test]
    fn retained_original_words_keep_value_operand_visits_and_no_body_visits() {
        use crate::native_compilation::{
            NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
            NativeCompilationSteps, NativeOriginalCompilerNameEffects,
        };
        use crate::native_compiler_word_projection::NativeCompilerWordOperand;
        use crate::native_compiler_words::NativeCompilerWords;
        use crate::native_control_compilation::NativeControlPreparationStep;
        let version = TclVersion::V9_0;
        let dialect = crate::InvocationDialect::for_version(version);
        let source = tcl_lexer::SourceImage::native(b"format {%s%s} literal $cmd".as_slice());
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let parsed = tcl_lexer::native_script_words_in(
            source.clone(),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        assert!(parsed.fatal_tail.is_none());
        assert_eq!(parsed.commands.len(), 1);
        let words = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            tcl_syntax::native_string::NativeStringProtocol::C(version),
        )
        .unwrap();
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::FormatStringConcat,
            operation: SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Inherit,
        };
        let selection = spec.select_native_words(&words, 1, Some(dialect), context());
        assert!(matches!(
            selection,
            NativeCompilationSelection::Inline { .. }
        ));
        let effects =
            spec.original_argument_name_effects_in_context(&words, 1, Some(dialect), context());
        let NativeOriginalCompilerNameEffects::Visits(visits) = effects else {
            panic!("missing original format compiler visits");
        };
        assert!(matches!(
            visits.as_slice(),
            [
                NativeControlPreparationStep::Word(NativeCompilerWordOperand::Original(2)),
                NativeControlPreparationStep::Word(NativeCompilerWordOperand::Original(3))
            ]
        ));
        let arguments = [
            crate::InvocationWord::Literal("%s%s"),
            crate::InvocationWord::Literal("literal"),
            crate::InvocationWord::Dynamic,
        ];
        let invocation = crate::InvocationWords::structured(
            crate::InvocationWord::Literal("format"),
            &arguments,
        );
        let shapes = &words.shapes()[1..];
        assert_eq!(
            spec.select(invocation, shapes, Some(dialect), context()),
            selection
        );
        let registry = crate::CommandRegistry::build_default();
        let facts = registry
            .resolve_structured_invocation(
                invocation.with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .expect("actual format invocation")
            .facts();
        assert!(
            matches!(spec.compilation_steps(selection, invocation, shapes, &facts, Some(dialect)), NativeCompilationSteps::Known(steps) if steps.is_empty())
        );
        assert!(spec.requires_original_word_preparation());
        assert_eq!(
            registry.native_compilation_for_registration("::format", dialect),
            Some(spec)
        );
        assert!(
            registry
                .native_compilation_for_registration("::renamedFormat", dialect)
                .is_none()
        );
        for version in [TclVersion::V8_4, TclVersion::V8_5] {
            let dialect = crate::InvocationDialect::for_version(version);
            assert_eq!(spec.compiler_hook_presence(dialect), Some(false));
            assert_eq!(
                spec.select(invocation, shapes, Some(dialect), context()),
                NativeCompilationSelection::Generic
            );
        }
        assert_eq!(
            spec.select(invocation, shapes, None, context()),
            NativeCompilationSelection::Unknown
        );
    }

    #[test]
    fn backslash_format_uses_decoded_original_word() {
        use crate::native_compilation::{
            NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
        };
        use crate::native_compiler_words::NativeCompilerWords;
        use crate::native_instruction_plan::NativeFormatStringPiece;
        let version = TclVersion::V9_0;
        let source = tcl_lexer::SourceImage::native(br"format x\n%s $value".as_slice());
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let parsed = tcl_lexer::native_script_words_in(
            source.clone(),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            tcl_syntax::native_string::NativeStringProtocol::C(version),
        )
        .unwrap();
        assert_eq!(words.literal(1), Some(b"x\n%s".as_slice()));
        assert_eq!(
            words.shapes()[1],
            NativeCompilationWordShape::BackslashLiteral
        );
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::FormatStringConcat,
            operation: SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Inherit,
        };
        assert!(matches!(
            spec.select_native_words(
                &words,
                1,
                Some(crate::InvocationDialect::for_version(version)),
                context()
            ),
            NativeCompilationSelection::Inline { .. }
        ));
        assert_eq!(
            instruction(&words, 1, version, context()).unwrap().pieces,
            vec![
                NativeFormatStringPiece::Literal(b"x\n".to_vec()),
                NativeFormatStringPiece::Value(2),
            ]
        );
    }

    #[test]
    fn malformed_original_operand_coordinates_refuse() {
        use crate::native_compilation::{
            NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
            NativeOriginalCompilerNameEffects,
        };
        use crate::native_compiler_words::NativeCompilerWords;
        let version = TclVersion::V9_0;
        let dialect = crate::InvocationDialect::for_version(version);
        let source = tcl_lexer::SourceImage::native(b"format {%s} $cmd".as_slice());
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let parsed = tcl_lexer::native_script_words_in(
            source.clone(),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            tcl_syntax::native_string::NativeStringProtocol::C(version),
        )
        .unwrap();
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::FormatStringConcat,
            operation: SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Inherit,
        };
        for operand in [0, words.original_words().len() + 1] {
            assert_eq!(
                spec.select_native_words(&words, operand, Some(dialect), context()),
                NativeCompilationSelection::Unknown
            );
            assert_eq!(
                spec.original_argument_name_effects_in_context(
                    &words,
                    operand,
                    Some(dialect),
                    context()
                ),
                NativeOriginalCompilerNameEffects::Unknown
            );
        }
    }

    #[test]
    fn source_compiler_context_and_concatenation_byte_count_remain_bounded() {
        use NativeCompilationWordShape as Shape;
        for changed in [
            NativeCompilationContext {
                mode: NativeCompilationMode::Direct,
                ..context()
            },
            NativeCompilationContext {
                frame: NativeCompilationFrame::Unknown,
                ..context()
            },
        ] {
            assert_eq!(
                select(
                    Some(b"%s"),
                    &[Shape::Literal, Shape::Substituted],
                    TclVersion::V9_0,
                    changed,
                    SemanticOperationId::Invoke
                ),
                NativeCompilationSelection::Unknown
            );
        }
        for count in [125, 126] {
            let mut shapes = vec![Shape::Substituted; count + 1];
            shapes[0] = Shape::Literal;
            let format = "%s".repeat(count);
            assert_eq!(
                matches!(
                    select(
                        Some(format.as_bytes()),
                        &shapes,
                        TclVersion::V9_0,
                        context(),
                        SemanticOperationId::Invoke
                    ),
                    NativeCompilationSelection::Inline { .. }
                ),
                count == 125
            );
        }
        assert_eq!(
            select(
                Some(b"%s"),
                &[Shape::Literal, Shape::Substituted],
                TclVersion::V9_0,
                context(),
                SemanticOperationId::Intrinsic(crate::IntrinsicId::ListConstruct)
            ),
            NativeCompilationSelection::Unknown
        );
    }
    #[test]
    fn original_instruction_pieces_preserve_percent_bytes_word_order_and_headers() {
        use crate::native_compiler_words::NativeCompilerWords;
        use crate::native_instruction_plan::{
            NativeFormatLiteralAllocation as Allocation, NativeFormatStringPiece as Piece,
        };
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            for (source, expected) in [
                (
                    b"format {a%%b%s%s%%} $x $y".as_slice(),
                    vec![
                        Piece::Literal(b"a%b".to_vec()),
                        Piece::Value(2),
                        Piece::Value(3),
                        Piece::Literal(b"%".to_vec()),
                    ],
                ),
                (b"format {%s} $x".as_slice(), vec![Piece::Value(2)]),
            ] {
                let image = tcl_lexer::SourceImage::native(source);
                let profile =
                    tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
                let parsed = tcl_lexer::native_script_words_in(
                    image.clone(),
                    tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                    tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap();
                let words = NativeCompilerWords::capture(
                    &parsed.commands[0].words,
                    tcl_syntax::native_string::NativeStringProtocol::C(version),
                )
                .unwrap();
                let recipe = instruction(&words, 1, version, context()).unwrap();
                assert_eq!(recipe.pieces, expected);
                assert_eq!(
                    recipe.literal_allocation,
                    if version == TclVersion::V9_1 {
                        Allocation::PrivateString
                    } else {
                        Allocation::Registered
                    }
                );
                for from in [0, words.original_words().len() + 1] {
                    assert!(instruction(&words, from, version, context()).is_none());
                }
            }
        }
    }
    #[test]
    fn c90_dynamic_hex_decline_retains_all_original_word_preparations() {
        use crate::native_compilation::{
            NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
            NativeOriginalCompilerNameEffects,
        };
        use crate::native_compiler_word_projection::NativeCompilerWordOperand;
        use crate::native_compiler_words::NativeCompilerWords;
        use crate::native_control_compilation::NativeControlPreparationStep as Step;
        use crate::native_instruction_plan::{NativeInstructionPlan, native_instruction_plan};
        let version = TclVersion::V9_0;
        let dialect = crate::InvocationDialect::for_version(version);
        let source = tcl_lexer::SourceImage::native(b"format %x $x".as_slice());
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let parsed = tcl_lexer::native_script_words_in(
            source.clone(),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            tcl_syntax::native_string::NativeStringProtocol::C(version),
        )
        .unwrap();
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::FormatStringConcat,
            operation: SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Inherit,
        };
        let selection = spec.select_native_words(&words, 1, Some(dialect), context());
        assert_eq!(selection, NativeCompilationSelection::Generic);
        let expected = vec![
            Step::Word(NativeCompilerWordOperand::Original(0)),
            Step::Word(NativeCompilerWordOperand::Original(1)),
            Step::Word(NativeCompilerWordOperand::Original(2)),
        ];
        assert_eq!(
            spec.original_argument_name_effects_in_context(&words, 1, Some(dialect), context()),
            NativeOriginalCompilerNameEffects::Visits(expected),
        );
        assert_eq!(
            native_instruction_plan(spec, selection, &words, 1, dialect, context()).unwrap(),
            NativeInstructionPlan::GenericPreparation(Vec::new()),
        );
        assert!(
            native_instruction_plan(
                spec,
                NativeCompilationSelection::Inline {
                    operation: SemanticOperationId::Invoke,
                    guard: NativeCompilationGuard::BeforeArguments,
                },
                &words,
                1,
                dialect,
                context(),
            )
            .is_err()
        );
    }

    #[test]
    fn dynamic_decimal_and_float_declines_keep_original_word_visits() {
        use crate::native_compilation::{
            NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
            NativeOriginalCompilerNameEffects,
        };
        use crate::native_compiler_word_projection::NativeCompilerWordOperand as Word;
        use crate::native_compiler_words::NativeCompilerWords;
        use crate::native_control_compilation::NativeControlPreparationStep as Step;
        use crate::native_instruction_plan::{NativeInstructionPlan, native_instruction_plan};
        let version = TclVersion::V9_0;
        let dialect = crate::InvocationDialect::for_version(version);
        for format in [b"%d".as_slice(), b"%f".as_slice()] {
            for (version, expected) in [
                (TclVersion::V8_4, NativeCompilationSelection::Unknown),
                (TclVersion::V8_5, NativeCompilationSelection::Unknown),
                (TclVersion::V8_6, NativeCompilationSelection::Generic),
                (TclVersion::V9_1, NativeCompilationSelection::Generic),
            ] {
                assert_eq!(
                    select(
                        Some(format),
                        &[
                            NativeCompilationWordShape::Literal,
                            NativeCompilationWordShape::Substituted
                        ],
                        version,
                        context(),
                        SemanticOperationId::Invoke
                    ),
                    expected
                );
            }
            assert_eq!(
                select(
                    Some(format),
                    &[
                        NativeCompilationWordShape::Literal,
                        NativeCompilationWordShape::Substituted,
                        NativeCompilationWordShape::Literal,
                    ],
                    TclVersion::V9_0,
                    context(),
                    SemanticOperationId::Invoke
                ),
                NativeCompilationSelection::Generic
            );
            for shapes in [
                vec![NativeCompilationWordShape::Literal],
                vec![
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Expanded,
                ],
                vec![
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Opaque,
                ],
            ] {
                assert_eq!(
                    select(
                        Some(format),
                        &shapes,
                        TclVersion::V9_0,
                        context(),
                        SemanticOperationId::Invoke
                    ),
                    NativeCompilationSelection::Unknown
                );
            }
        }
        for source in ["format %d $x", "format %f [side]"] {
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
                grammar: NativeCompilationGrammar::FormatStringConcat,
                operation: SemanticOperationId::Invoke,
                body: NativeBodyCompilation::Inherit,
            };
            let selection = spec.select_native_words(&words, 1, Some(dialect), context());
            assert_eq!(selection, NativeCompilationSelection::Generic, "{source}");
            assert_eq!(
                spec.original_argument_name_effects_in_context(&words, 1, Some(dialect), context()),
                NativeOriginalCompilerNameEffects::Visits(vec![
                    Step::Word(Word::Original(0)),
                    Step::Word(Word::Original(1)),
                    Step::Word(Word::Original(2))
                ])
            );
            assert_eq!(
                native_instruction_plan(spec, selection, &words, 1, dialect, context()).unwrap(),
                NativeInstructionPlan::GenericPreparation(Vec::new())
            );
        }
    }

    #[test]
    fn c90_dynamic_format_word_declines_before_other_compiler_visits() {
        use crate::native_compilation::{
            NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
            NativeOriginalCompilerNameEffects,
        };
        use crate::native_compiler_word_projection::NativeCompilerWordOperand as Word;
        use crate::native_compiler_words::NativeCompilerWords;
        use crate::native_control_compilation::NativeControlPreparationStep as Step;
        use crate::native_instruction_plan::{NativeInstructionPlan, native_instruction_plan};
        let version = TclVersion::V9_0;
        let dialect = crate::InvocationDialect::for_version(version);
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::FormatStringConcat,
            operation: SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Inherit,
        };
        for source in [
            "format $fmt $value",
            "format [side fmt] [side value]",
            "format $fmt fixed",
            "format $fmt",
            "format $fmt $a $b",
            "format [side fmt] [side a] [side b]",
        ] {
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
            let selection = spec.select_native_words(&words, 1, Some(dialect), context());
            assert_eq!(selection, NativeCompilationSelection::Generic, "{source}");
            assert_eq!(
                spec.original_argument_name_effects_in_context(&words, 1, Some(dialect), context()),
                NativeOriginalCompilerNameEffects::Visits(
                    (0..words.shapes().len())
                        .map(|index| Step::Word(Word::Original(index)))
                        .collect()
                )
            );
            assert_eq!(
                native_instruction_plan(spec, selection, &words, 1, dialect, context()).unwrap(),
                NativeInstructionPlan::GenericPreparation(Vec::new())
            );
        }
    }

    #[test]
    fn c90_dynamic_format_word_decline_rejects_foreign_contracts() {
        use crate::native_compilation::{
            NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
            NativeOriginalCompilerNameEffects,
        };
        use crate::native_compiler_words::NativeCompilerWords;
        let version = TclVersion::V9_0;
        let dialect = crate::InvocationDialect::for_version(version);
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::FormatStringConcat,
            operation: SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Inherit,
        };
        let source = "format $fmt $value";
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
        for body in [
            NativeBodyCompilation::Direct,
            NativeBodyCompilation::ScriptObject,
            NativeBodyCompilation::ProcedureObject,
        ] {
            let foreign = NativeCompilationSpec { body, ..spec };
            assert_eq!(
                foreign.select_native_words(&words, 1, Some(dialect), context()),
                NativeCompilationSelection::Unknown
            );
            assert_eq!(
                foreign.original_argument_name_effects_in_context(
                    &words,
                    1,
                    Some(dialect),
                    context()
                ),
                NativeOriginalCompilerNameEffects::Unknown
            );
        }
        let unknown = NativeCompilationContext {
            mode: NativeCompilationMode::Unknown,
            ..context()
        };
        assert_eq!(
            spec.select_native_words(&words, 1, Some(dialect), unknown),
            NativeCompilationSelection::Unknown
        );
        let direct = NativeCompilationContext {
            mode: NativeCompilationMode::Direct,
            ..context()
        };
        assert!(!dynamic_format_word_declines(
            &words,
            1,
            version,
            direct,
            spec.operation,
            spec.body
        ));
        let foreign = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            tcl_syntax::native_string::NativeStringProtocol::C(TclVersion::V8_6),
        )
        .unwrap();
        assert!(!dynamic_format_word_declines(
            &foreign,
            1,
            version,
            context(),
            spec.operation,
            spec.body
        ));
        for other in [TclVersion::V8_6, TclVersion::V9_1] {
            assert_eq!(
                spec.select_native_words(
                    &words,
                    1,
                    Some(crate::InvocationDialect::for_version(other)),
                    context()
                ),
                NativeCompilationSelection::Unknown
            );
        }
    }

    #[test]
    fn c90_dynamic_format_word_decline_rejects_other_shapes() {
        use crate::native_compilation::NativeBodyCompilation;
        use crate::native_compiler_words::NativeCompilerWords;
        let version = TclVersion::V9_0;
        let dialect = crate::InvocationDialect::for_version(version);
        for source in ["format $fmt {*}$values", "format %s $value"] {
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
            assert!(
                !dynamic_format_word_declines(
                    &words,
                    1,
                    version,
                    context(),
                    SemanticOperationId::Invoke,
                    NativeBodyCompilation::Inherit
                ),
                "{source}"
            );
        }
    }

    #[test]
    fn dynamic_hex_decline_does_not_cover_other_points_or_unknown_vectors() {
        let version = TclVersion::V9_0;
        let mut unknown = context();
        unknown.frame = NativeCompilationFrame::Unknown;
        assert_eq!(
            select(
                Some(b"%x"),
                &[
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Substituted
                ],
                version,
                unknown,
                SemanticOperationId::Invoke
            ),
            NativeCompilationSelection::Unknown
        );
        assert_eq!(
            select(
                None,
                &[
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Substituted
                ],
                version,
                context(),
                SemanticOperationId::Invoke
            ),
            NativeCompilationSelection::Unknown
        );
        for (version, expected) in [
            (TclVersion::V8_4, NativeCompilationSelection::Unknown),
            (TclVersion::V8_5, NativeCompilationSelection::Unknown),
            (TclVersion::V8_6, NativeCompilationSelection::Generic),
            (TclVersion::V9_1, NativeCompilationSelection::Generic),
        ] {
            assert_eq!(
                select(
                    Some(b"%x"),
                    &[
                        NativeCompilationWordShape::Literal,
                        NativeCompilationWordShape::Substituted
                    ],
                    version,
                    context(),
                    SemanticOperationId::Invoke
                ),
                expected,
            );
        }
        assert_eq!(
            select(
                Some(b"%x"),
                &[
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Substituted,
                    NativeCompilationWordShape::Substituted,
                ],
                version,
                context(),
                SemanticOperationId::Invoke
            ),
            NativeCompilationSelection::Generic
        );
        for shapes in [
            vec![
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Literal,
            ],
            vec![
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Expanded,
            ],
            vec![
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Opaque,
            ],
        ] {
            assert_eq!(
                select(
                    Some(b"%x"),
                    &shapes,
                    version,
                    context(),
                    SemanticOperationId::Invoke
                ),
                NativeCompilationSelection::Unknown
            );
        }
    }
}
