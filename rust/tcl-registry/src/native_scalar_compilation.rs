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

//! Original scalar compiler operands and their actual registration scopes.

use crate::SemanticOperationId;
use crate::native_compilation::{
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationWordShape as Shape,
    NativeCompilerImplementationLookup, NativeNamedInvocationProtocol,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;

/// Original argument vector accepted by an independently retained compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeScalarScope {
    /// The monolithic/public String compiler receives the original member word.
    PublicMember,
    /// The actual private registration receives only its scalar operands.
    PrivateOperands,
}

/// Native stack instruction, independent of any mutable runtime handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeScalarOperation {
    /// Compare two original strings through the selected native equality law.
    StringEqual,
    /// Reach the original character-length accessor.
    StringLength,
    /// Reach the original list-length accessor.
    ListLength,
}

impl NativeScalarOperation {
    /// Number of actual original stack operands consumed by this instruction.
    #[must_use]
    pub const fn arity(self) -> usize {
        match self {
            Self::StringEqual => 2,
            Self::StringLength | Self::ListLength => 1,
        }
    }

    /// Actual private String compiler registration, absent for list length.
    #[must_use]
    pub const fn implementation(self) -> Option<&'static NativeCompilerImplementationLookup> {
        match self {
            Self::StringEqual => Some(&STRING_EQUAL_IMPLEMENTATION),
            Self::StringLength => Some(&STRING_LENGTH_IMPLEMENTATION),
            Self::ListLength => None,
        }
    }
}

/// Chronological original operands; getters run only after every operand is evaluated.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeScalarInstruction {
    /// Selected native opcode behavior.
    pub operation: NativeScalarOperation,
    /// Exact original words or original static expansion member receipts.
    pub operands: Vec<NativeCompilerWordOperand>,
}

/// Genuine original private String equality registration.
pub const STRING_EQUAL_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "equal",
        slot: "::tcl::string::equal",
        command: "string",
        prepended: &["equal"],
    };
/// Genuine original private String character-length registration.
pub const STRING_LENGTH_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "length",
        slot: "::tcl::string::length",
        command: "string",
        prepended: &["length"],
    };

/// Select the actual compiler's argument geometry. This grants no registration
/// or native issuer authority; its caller independently retains both.
#[must_use]
pub fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    operation: NativeScalarOperation,
    scope: NativeScalarScope,
    version: TclVersion,
    semantic: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    let Ok(projected) = project_native_compiler_words(words, version) else {
        return Selection::Unknown;
    };
    if operand_from == 0 {
        return Selection::Unknown;
    }
    let public = scope == NativeScalarScope::PublicMember;
    if public {
        let Some(lookup) = operation.implementation() else {
            return Selection::Unknown;
        };
        if !projected.get(operand_from).is_some_and(|member| {
            matches!(
                member.shape,
                Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral
            ) && (version != TclVersion::V8_4 || member.shape == Shape::Literal)
                && member.literal.as_deref().is_some_and(|bytes| {
                    !bytes.is_empty() && lookup.member.as_bytes().starts_with(bytes)
                })
        }) {
            return Selection::Generic;
        }
    } else if operation != NativeScalarOperation::ListLength && version == TclVersion::V8_4 {
        return Selection::Generic;
    }
    let from = operand_from + usize::from(public);
    let Some(operands) = projected.get(from..) else {
        return Selection::Unknown;
    };
    if operands
        .iter()
        .any(|operand| operand.shape == Shape::Opaque)
    {
        return Selection::Unknown;
    }
    if operands
        .iter()
        .any(|operand| operand.shape == Shape::Expanded)
    {
        return Selection::Generic;
    }
    if operands.len() == operation.arity() {
        return Selection::Inline {
            operation: semantic,
            guard: if version == TclVersion::V8_4 {
                NativeCompilationGuard::ChunkEntry
            } else {
                NativeCompilationGuard::BeforeArguments
            },
        };
    }
    if operation == NativeScalarOperation::ListLength && version == TclVersion::V8_4 {
        return Selection::CompileError;
    }
    if public && version >= TclVersion::V8_6 {
        return Selection::NamedInvocation {
            lookup: operation.implementation().expect("public String compiler"),
            arguments_from: from - 1,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        };
    }
    Selection::Generic
}

/// Project only the independently admitted inline geometry onto original stack operands.
#[must_use]
pub fn compile_native_scalar(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    operation: NativeScalarOperation,
    scope: NativeScalarScope,
    version: TclVersion,
) -> Option<NativeScalarInstruction> {
    if !matches!(
        select_original(
            words,
            operand_from,
            operation,
            scope,
            version,
            SemanticOperationId::Invoke
        ),
        NativeCompilationSelection::Inline { .. }
    ) {
        return None;
    }
    let projected = project_native_compiler_words(words, version).ok()?;
    let from = operand_from + usize::from(scope == NativeScalarScope::PublicMember);
    Some(NativeScalarInstruction {
        operation,
        operands: projected
            .get(from..)?
            .iter()
            .map(|operand| operand.operand.clone())
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    };
    include!("../tests/data/native_scalar_compilation/cases.rs");

    #[test]
    fn original_scalar_recipes_match_seventy_five_native_compiler_windows() {
        let registry = crate::CommandRegistry::build_default();
        let mut windows = 0;
        for (version, table) in [
            (
                TclVersion::V8_4,
                include_str!("../tests/data/native_scalar_compilation/8.4.20.tsv"),
            ),
            (
                TclVersion::V8_5,
                include_str!("../tests/data/native_scalar_compilation/8.5.19.tsv"),
            ),
            (
                TclVersion::V8_6,
                include_str!("../tests/data/native_scalar_compilation/8.6.18.tsv"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../tests/data/native_scalar_compilation/9.0.4.tsv"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../tests/data/native_scalar_compilation/9.1.0.tsv"),
            ),
        ] {
            let dialect = crate::InvocationDialect::for_version(version);
            for row in table.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                let case = fields[0].parse::<usize>().unwrap();
                let source = CASES[case];
                let parsed = tcl_lexer::native_script_words_in(
                    tcl_lexer::SourceImage::native(source.as_bytes()),
                    tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                    tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
                );
                let observed_inline = fields[9].split(',').any(|opcode| {
                    matches!(opcode, "streq" | "strlen" | "listlength" | "listLength")
                });
                windows += 1;
                let Ok(parsed) = parsed else {
                    assert!(!observed_inline);
                    continue;
                };
                let words = NativeCompilerWords::capture(
                    &parsed.commands[0].words,
                    dialect.native_string_protocol().unwrap(),
                )
                .unwrap();
                let operation = if source.starts_with("llength") {
                    NativeScalarOperation::ListLength
                } else if matches!(case, 0..=3 | 11 | 14) {
                    NativeScalarOperation::StringEqual
                } else {
                    NativeScalarOperation::StringLength
                };
                let scope = if operation == NativeScalarOperation::ListLength {
                    NativeScalarScope::PrivateOperands
                } else {
                    NativeScalarScope::PublicMember
                };
                let semantic = if operation == NativeScalarOperation::ListLength {
                    SemanticOperationId::Intrinsic(crate::IntrinsicId::ListLength)
                } else {
                    SemanticOperationId::Invoke
                };
                let selected = select_original(&words, 1, operation, scope, version, semantic);
                assert_eq!(
                    matches!(selected, NativeCompilationSelection::Inline { .. }),
                    observed_inline,
                    "{version:?}/{case}"
                );
                if operation == NativeScalarOperation::ListLength {
                    let spec = registry
                        .native_compilation_for_registration("llength", dialect)
                        .unwrap();
                    let context = NativeCompilationContext {
                        mode: NativeCompilationMode::BytecodeObject,
                        frame: NativeCompilationFrame::ProcedureCode,
                        ..Default::default()
                    };
                    assert_eq!(
                        spec.select_native_words(&words, 1, Some(dialect), context),
                        selected
                    );
                    assert_eq!(
                        selected == NativeCompilationSelection::CompileError,
                        version == TclVersion::V8_4 && matches!(case, 8 | 9)
                    );
                }
                let recipe = compile_native_scalar(&words, 1, operation, scope, version);
                assert_eq!(recipe.is_some(), observed_inline);
                if let Some(recipe) = recipe {
                    assert_eq!(recipe.operands.len(), operation.arity());
                }
            }
        }
        assert_eq!(windows, 75);
    }

    #[test]
    fn scalar_registration_scope_is_not_inferred_from_the_original_head() {
        let registry = crate::CommandRegistry::build_default();
        for version in TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            let source = b"renamed equal $left $right";
            let parsed = tcl_lexer::native_script_words_in(
                tcl_lexer::SourceImage::native(source.as_slice()),
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
                compile_native_scalar(
                    &words,
                    1,
                    NativeScalarOperation::StringEqual,
                    NativeScalarScope::PublicMember,
                    version
                )
                .is_some()
            );
            assert_eq!(
                compile_native_scalar(
                    &words,
                    2,
                    NativeScalarOperation::StringEqual,
                    NativeScalarScope::PrivateOperands,
                    version
                )
                .is_some(),
                version >= TclVersion::V8_5
            );
            let original = registry
                .native_compilation_for_original_registration("string", &words, 1, dialect)
                .unwrap();
            assert_eq!(
                original.scalar_compilation().is_some(),
                version == TclVersion::V8_4
            );
            assert!(
                registry
                    .native_compilation_for_original_registration("unissued", &words, 1, dialect)
                    .is_none()
            );
            let mut unknown = dialect;
            unknown.core_point = None;
            unknown.native_family = None;
            assert!(unknown.native_object_vector_protocol().is_none());
            let spec = registry
                .native_compilation_for_registration("llength", dialect)
                .unwrap();
            assert_eq!(
                spec.select_native_words(
                    &words,
                    1,
                    Some(unknown),
                    crate::native_compilation::NativeCompilationContext {
                        mode: crate::native_compilation::NativeCompilationMode::BytecodeObject,
                        ..Default::default()
                    }
                ),
                NativeCompilationSelection::Unknown
            );
        }
    }
}
