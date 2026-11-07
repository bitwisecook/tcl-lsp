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

//! C8.6 to C9.1's `TclCompileStringIsCmd` over a compile-known class.
//!
//! The subject is compiled once, followed by the class branch's test. A
//! dynamic class, `-failindex` and other options stay unknown here.

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

/// A class branch of `TclCompileStringIsCmd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeStringIsClass {
    /// `STR_CLASS` with C's `InstStringClassType` (`alnum` 0 … `xdigit` 12).
    Character(u8),
    /// `boolean`: `tryCvtToBoolean`.
    Boolean,
    /// `true`.
    True,
    /// `false`.
    False,
    /// `double`: any `numericType`.
    Double,
    /// `integer`: C8.6 `numericType == 1`, C9 `numericType <= 3`.
    Integer,
    /// `wideinteger`: `numericType <= 2`.
    Wide,
    /// `entier`: `numericType <= 3`.
    Entier,
    /// `dict` (C9): caught `dictVerify`.
    Dict,
    /// `list`: caught `listLength`.
    List,
}

/// One selected `string is` compilation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeStringIsInstruction {
    /// Selected class branch.
    pub class: NativeStringIsClass,
    /// `-strict`: the empty string does not satisfy the class.
    pub strict: bool,
    /// The single compiled subject word.
    pub subject: crate::native_compiler_word_projection::NativeCompilerWordOperand,
}

/// `isClasses` in the release's `TclCompileStringIsCmd`, in table order.
fn class_table(version: TclVersion) -> &'static [(&'static [u8], NativeStringIsClass)] {
    use NativeStringIsClass as C;
    const C86: &[(&[u8], NativeStringIsClass)] = &[
        (b"alnum", C::Character(0)),
        (b"alpha", C::Character(1)),
        (b"ascii", C::Character(2)),
        (b"control", C::Character(3)),
        (b"boolean", C::Boolean),
        (b"digit", C::Character(4)),
        (b"double", C::Double),
        (b"entier", C::Entier),
        (b"false", C::False),
        (b"graph", C::Character(5)),
        (b"integer", C::Integer),
        (b"list", C::List),
        (b"lower", C::Character(6)),
        (b"print", C::Character(7)),
        (b"punct", C::Character(8)),
        (b"space", C::Character(9)),
        (b"true", C::True),
        (b"upper", C::Character(10)),
        (b"wideinteger", C::Wide),
        (b"wordchar", C::Character(11)),
        (b"xdigit", C::Character(12)),
    ];
    const C9: &[(&[u8], NativeStringIsClass)] = &[
        (b"alnum", C::Character(0)),
        (b"alpha", C::Character(1)),
        (b"ascii", C::Character(2)),
        (b"control", C::Character(3)),
        (b"boolean", C::Boolean),
        (b"dict", C::Dict),
        (b"digit", C::Character(4)),
        (b"double", C::Double),
        (b"entier", C::Entier),
        (b"false", C::False),
        (b"graph", C::Character(5)),
        (b"integer", C::Integer),
        (b"list", C::List),
        (b"lower", C::Character(6)),
        (b"print", C::Character(7)),
        (b"punct", C::Character(8)),
        (b"space", C::Character(9)),
        (b"true", C::True),
        (b"upper", C::Character(10)),
        (b"wideinteger", C::Wide),
        (b"wordchar", C::Character(11)),
        (b"xdigit", C::Character(12)),
    ];
    if version >= TclVersion::V9_0 { C9 } else { C86 }
}

/// `Tcl_GetIndexFromObj` without `TCL_EXACT`: an exact name, else a unique
/// prefix. Anything else is the compile-time "bad class" syntax error.
fn resolve_class(name: &[u8], version: TclVersion) -> Option<NativeStringIsClass> {
    let table = class_table(version);
    if let Some((_, class)) = table.iter().find(|(entry, _)| *entry == name) {
        return Some(*class);
    }
    let mut matches = table.iter().filter(|(entry, _)| entry.starts_with(name));
    match (matches.next(), matches.next()) {
        (Some((_, class)), None) if !name.is_empty() => Some(*class),
        _ => None,
    }
}

/// C's `GotLiteral(tokenPtr, "-strict")`: a simple word of at least two
/// bytes that is a prefix of `-strict`.
fn strict_option(shape: Shape, literal: Option<&[u8]>) -> bool {
    simple(shape) && literal.is_some_and(|word| word.len() > 1 && b"-strict".starts_with(word))
}

/// The class and strictness `TclCompileStringIsCmd` compiles, or the
/// selection it makes instead.
fn select_class(
    shapes: &[Shape],
    literal: &dyn Fn(usize) -> Option<Vec<u8>>,
    from: usize,
    version: TclVersion,
) -> Result<(NativeStringIsClass, bool), NativeCompilationSelection> {
    use NativeCompilationSelection as Selection;
    let Some(arguments) = shapes.get(from..) else {
        return Err(Selection::Unknown);
    };
    if arguments
        .iter()
        .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded))
    {
        return Err(Selection::Unknown);
    }
    // Only a compile-known class with an optional `-strict` is modelled.
    // A dynamic class, `-failindex`, a bad class (its compile-time syntax
    // error) and other options keep their context-proven declines elsewhere.
    let Some(name) = literal(from).filter(|_| simple(arguments[0])) else {
        return Err(Selection::Unknown);
    };
    let class = resolve_class(&name, version).ok_or(Selection::Unknown)?;
    let strict = match arguments.len() {
        2 => false,
        3 if strict_option(arguments[1], literal(from + 1).as_deref()) => true,
        _ => return Err(Selection::Unknown),
    };
    Ok((class, strict))
}

/// Select `TclCompileStringIsCmd` from argument shapes and literals.
#[must_use]
pub fn select_shapes(
    shapes: &[Shape],
    literal: &dyn Fn(usize) -> Option<Vec<u8>>,
    scope: NativeScalarScope,
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_6 {
        return Selection::Generic;
    }
    let from = usize::from(scope == NativeScalarScope::PublicMember);
    if from == 1
        && !(shapes.first().is_some_and(|shape| simple(*shape))
            && literal(0).as_deref() == Some(b"is"))
    {
        return Selection::Unknown;
    }
    match select_class(shapes, literal, from, version) {
        Ok(_) => Selection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        },
        Err(selection) => selection,
    }
}

/// C9 declines a direct private `StringIs` compiler before emitting any word
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

/// C9 rejects the direct private failindex forms after resolving the exact
/// compile-known ASCII class in a fresh temporary and before emitting words.
pub(crate) fn private_ascii_failindex_declines(
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
    if arguments
        .iter()
        .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded))
        || arguments.first() != Some(&Shape::Literal)
        || words.literal(operand_from) != Some(b"ascii")
    {
        return false;
    }
    let option = match arguments.len() {
        4 => 1,
        5 if arguments[1] == Shape::Literal
            && words.literal(operand_from + 1) == Some(b"-strict") =>
        {
            2
        }
        _ => return false,
    };
    arguments[option] == Shape::Literal
        && words.literal(operand_from + option) == Some(b"-failindex")
}

/// A closed direct-private compiler decline with no emitted preparation.
pub(crate) fn private_original_declines(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
    operation: SemanticOperationId,
    body: crate::native_compilation::NativeBodyCompilation,
    context: crate::native_compilation::NativeCompilationContext,
) -> bool {
    private_dynamic_class_declines(
        words,
        operand_from,
        scope,
        version,
        operation,
        body,
        context,
    ) || private_ascii_failindex_declines(
        words,
        operand_from,
        scope,
        version,
        operation,
        body,
        context,
    )
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
    let shapes: Vec<_> = arguments.iter().map(|word| word.shape).collect();
    select_shapes(
        &shapes,
        &|index| arguments.get(index).and_then(|word| word.literal.clone()),
        scope,
        version,
        operation,
    )
}

/// The selected class branch over its single compiled subject word.
#[must_use]
pub fn string_is_instruction(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
) -> Option<NativeStringIsInstruction> {
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
    let arguments = projected.get(operand_from..)?;
    let from = usize::from(scope == NativeScalarScope::PublicMember);
    let shapes: Vec<_> = arguments.iter().map(|word| word.shape).collect();
    let (class, strict) = select_class(
        &shapes,
        &|index| arguments.get(index).and_then(|word| word.literal.clone()),
        from,
        version,
    )
    .ok()?;
    let subject = arguments
        .get(from + 1 + usize::from(strict))?
        .operand
        .clone();
    Some(NativeStringIsInstruction {
        class,
        strict,
        subject,
    })
}

/// `CompileWord(subject)` once; the compile-known class is never a runtime operand.
/// Only the non-strict `ascii` branch uses this scalar form.
#[must_use]
pub fn instruction(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
) -> Option<NativeScalarInstruction> {
    let recipe = string_is_instruction(words, operand_from, scope, version)?;
    (recipe.class == NativeStringIsClass::Character(2) && !recipe.strict).then(|| {
        NativeScalarInstruction {
            operation: NativeScalarOperation::StringIsAscii,
            operands: vec![recipe.subject],
        }
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
    fn private_ascii_failindex_decline_is_limited_to_original_proven_forms() {
        use crate::native_compilation::NativeBodyCompilation;
        let version = TclVersion::V9_0;
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            ..NativeCompilationContext::default()
        };
        let operation = SemanticOperationId::Intrinsic(crate::IntrinsicId::StringIs);
        for source in [
            b"::tcl::string::is ascii -failindex index $s".as_slice(),
            b"::tcl::string::is ascii -strict -failindex index $s",
        ] {
            let source_words = words(source, version);
            let captured =
                NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version))
                    .unwrap();
            assert!(private_ascii_failindex_declines(
                &captured,
                1,
                NativeScalarScope::PrivateOperands,
                version,
                operation,
                NativeBodyCompilation::Inherit,
                context
            ));
            assert!(!private_ascii_failindex_declines(
                &captured,
                2,
                NativeScalarScope::PrivateOperands,
                version,
                operation,
                NativeBodyCompilation::Inherit,
                context
            ));
            assert!(!private_ascii_failindex_declines(
                &captured,
                1,
                NativeScalarScope::PublicMember,
                version,
                operation,
                NativeBodyCompilation::Inherit,
                context
            ));
            assert!(!private_ascii_failindex_declines(
                &captured,
                1,
                NativeScalarScope::PrivateOperands,
                version,
                operation,
                NativeBodyCompilation::Direct,
                context
            ));
            let unknown = NativeCompilationContext {
                mode: NativeCompilationMode::Unknown,
                ..context
            };
            assert!(!private_ascii_failindex_declines(
                &captured,
                1,
                NativeScalarScope::PrivateOperands,
                version,
                operation,
                NativeBodyCompilation::Inherit,
                unknown
            ));
            let foreign = NativeCompilerWords::capture(
                &source_words,
                NativeStringProtocol::C(TclVersion::V8_6),
            )
            .unwrap();
            assert!(!private_ascii_failindex_declines(
                &foreign,
                1,
                NativeScalarScope::PrivateOperands,
                version,
                operation,
                NativeBodyCompilation::Inherit,
                context
            ));
        }
    }

    #[test]
    fn private_ascii_failindex_decline_rejects_other_shapes() {
        use crate::native_compilation::NativeBodyCompilation;
        let version = TclVersion::V9_0;
        let dialect = crate::InvocationDialect::for_version(version);
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            ..NativeCompilationContext::default()
        };
        let operation = SemanticOperationId::Intrinsic(crate::IntrinsicId::StringIs);
        for source in [
            b"::tcl::string::is $class -failindex index $s".as_slice(),
            b"::tcl::string::is integer -failindex index $s",
            b"::tcl::string::is ascii $option index $s",
            b"::tcl::string::is ascii -failindex {*}$args",
            b"::tcl::string::is ascii -strict $s",
            b"string is ascii -failindex index $s",
        ] {
            let source_words = words(source, version);
            let captured = NativeCompilerWords::capture(
                &source_words,
                dialect.native_string_protocol().unwrap(),
            )
            .unwrap();
            assert!(
                !private_ascii_failindex_declines(
                    &captured,
                    1,
                    NativeScalarScope::PrivateOperands,
                    version,
                    operation,
                    NativeBodyCompilation::Inherit,
                    context
                ),
                "{source:?}"
            );
        }
    }

    #[test]
    fn c90_private_ascii_failindex_decline_keeps_original_word_visits() {
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
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            ..NativeCompilationContext::default()
        };
        for source in [
            b"::tcl::string::is ascii -failindex [index] [subject]".as_slice(),
            b"::tcl::string::is ascii -strict -failindex index $s",
        ] {
            let source_words = words(source, version);
            let captured =
                NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version))
                    .unwrap();
            let selection = private.select_native_words(&captured, 1, Some(dialect), context);
            assert_eq!(selection, NativeCompilationSelection::Generic, "{source:?}");
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
                native_instruction_plan(private, selection, &captured, 1, dialect, context),
                Ok(NativeInstructionPlan::GenericPreparation(Vec::new()))
            );
        }
        let source_words = words(b"string is ascii -failindex index $s", version);
        let captured =
            NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version)).unwrap();
        let public = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::StringIsAscii(NativeScalarScope::PublicMember),
            ..private
        };
        assert_eq!(
            public.select_native_words(&captured, 1, Some(dialect), context),
            NativeCompilationSelection::Unknown
        );
        assert_eq!(
            private.select_native_words(&captured, 2, Some(dialect), context),
            NativeCompilationSelection::Unknown
        );
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
                b"string is ascii -failindex out $subject".as_slice(),
                b"string is $class $subject",
                b"string is ascii -bogus $subject",
                b"string is nosuchclass $subject",
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
    fn known_classes_prefixes_and_strict_compile_as_their_release_does() {
        use NativeStringIsClass as Class;
        for (version, source, expected) in [
            (
                TclVersion::V8_6,
                b"string is alpha $s".as_slice(),
                Some((Class::Character(1), false)),
            ),
            (
                TclVersion::V9_0,
                b"string is asc -strict $s",
                Some((Class::Character(2), true)),
            ),
            (
                TclVersion::V9_0,
                b"string is int -st $s",
                Some((Class::Integer, true)),
            ),
            (
                TclVersion::V9_0,
                b"string is dict $s",
                Some((Class::Dict, false)),
            ),
            // C8.6 has no dict class, and `d` is ambiguous in both tables.
            (TclVersion::V8_6, b"string is dict $s", None),
            (TclVersion::V9_0, b"string is d $s", None),
            // GotLiteral needs at least two bytes of `-strict`.
            (TclVersion::V9_0, b"string is upper - $s", None),
            (
                TclVersion::V9_1,
                b"string is upper $s",
                Some((Class::Character(10), false)),
            ),
            (
                TclVersion::V9_1,
                b"string is upper -strict $s",
                Some((Class::Character(10), true)),
            ),
            (
                TclVersion::V9_1,
                b"string is boolean $s",
                Some((Class::Boolean, false)),
            ),
        ] {
            let source_words = words(source, version);
            let original =
                NativeCompilerWords::capture(&source_words, NativeStringProtocol::C(version))
                    .unwrap();
            let recipe =
                string_is_instruction(&original, 1, NativeScalarScope::PublicMember, version)
                    .map(|recipe| (recipe.class, recipe.strict));
            assert_eq!(recipe, expected, "{version:?}: {source:?}");
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
