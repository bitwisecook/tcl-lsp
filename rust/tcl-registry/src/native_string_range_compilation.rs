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

//! C9 String-range compiler coordinates over original compile-known indices.

use crate::native_compilation::{
    NativeBodyCompilation, NativeCompilationContext, NativeCompilationGuard, NativeCompilationMode,
    NativeCompilationSelection, NativeCompilationSpec, NativeCompilationWordShape as Shape,
    NativeCompilerImplementationLookup,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_scalar_compilation::NativeScalarScope;
use tcl_dialect::TclVersion;
use tcl_syntax::native_compiled_index::NativeCompiledListRange;

pub(crate) const STRING_RANGE_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "range",
        slot: "::tcl::string::range",
        command: "string",
        prepended: &["range"],
    };

/// Original compiler action after preparing the subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeStringRangeOperation {
    /// POP the prepared subject and PUSH the compiled empty literal, without a getter.
    Empty,
    /// `STR_RANGE_IMM`; the empty-subject branch keeps that original stack object.
    Immediate(NativeCompiledListRange),
    /// `nonConstantIndices`: push both original index words, then `STR_RANGE`.
    Indices {
        /// The original first-index word.
        first: NativeCompilerWordOperand,
        /// The original last-index word.
        last: NativeCompilerWordOperand,
    },
}

/// Compile-known coordinates grant no current command or interpreter authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeStringRangeInstruction {
    /// The original subject, including its independently retained expansion member.
    pub operand: NativeCompilerWordOperand,
    /// Exact original compiler outcome, independent of runtime value contents.
    pub operation: NativeStringRangeOperation,
}

impl NativeStringRangeInstruction {
    /// Original words the recipe compiles, in order.
    #[must_use]
    pub fn compiled_words(&self) -> Vec<NativeCompilerWordOperand> {
        let mut words = vec![self.operand.clone()];
        if let NativeStringRangeOperation::Indices { first, last } = &self.operation {
            words.extend([first.clone(), last.clone()]);
        }
        words
    }
}

/// Select only the represented C9 literal-index compiler branch.
#[must_use]
pub fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
    spec: NativeCompilationSpec,
    context: NativeCompilationContext,
) -> NativeCompilationSelection {
    if spec.grammar != crate::native_compilation::NativeCompilationGrammar::StringRange(scope)
        || context.mode != NativeCompilationMode::BytecodeObject
        || spec.operation != crate::SemanticOperationId::Invoke
        || spec.body != NativeBodyCompilation::Inherit
        || compile_native_string_range(words, operand_from, scope, version).is_none()
    {
        return NativeCompilationSelection::Unknown;
    }
    NativeCompilationSelection::Inline {
        operation: spec.operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

/// Project the C8.6+ index-encoding branch from complete original words.
/// Uncertain index widths and releases without the compiler retain refusal.
#[must_use]
pub fn compile_native_string_range(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
) -> Option<NativeStringRangeInstruction> {
    if version < TclVersion::V8_6
        || words.source_protocol() != tcl_syntax::native_string::NativeStringProtocol::C(version)
        || operand_from == 0
        || words
            .shapes()
            .iter()
            .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded))
    {
        return None;
    }
    let projected = project_native_compiler_words(words, version).ok()?;
    let public = scope == NativeScalarScope::PublicMember;
    if public
        && (!matches!(
            projected.get(operand_from)?.shape,
            Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral
        ) || projected.get(operand_from)?.literal.as_deref() != Some(b"range"))
    {
        return None;
    }
    let from = operand_from.checked_add(usize::from(public))?;
    let [subject, first, last] = projected.get(from..)? else {
        return None;
    };
    let indices = NativeStringRangeOperation::Indices {
        first: first.operand.clone(),
        last: last.operand.clone(),
    };
    let after = if version < TclVersion::V9_0 {
        i32::MAX
    } else {
        -1
    };
    let operation = match index_token(first, version, 0, after)? {
        IndexToken::Dynamic => indices,
        IndexToken::Constant(first) if first.encoded() == after => {
            NativeStringRangeOperation::Empty
        }
        IndexToken::Constant(first) => match index_token(last, version, -1, -2)? {
            IndexToken::Dynamic => indices,
            IndexToken::Constant(last) if last.encoded() == -1 => NativeStringRangeOperation::Empty,
            IndexToken::Constant(last) => {
                NativeStringRangeOperation::Immediate(NativeCompiledListRange { first, last })
            }
        },
    };
    Some(NativeStringRangeInstruction {
        operand: subject.operand.clone(),
        operation,
    })
}

/// `TclGetIndexFromToken`'s outcome for one original index word.
enum IndexToken {
    Constant(tcl_syntax::native_compiled_index::NativeCompiledListIndex),
    /// The non-constant branch: the word is pushed for `STR_RANGE`.
    Dynamic,
}

/// `None` keeps refusal where the encoded width is not known.
fn index_token(
    word: &crate::native_compiler_word_projection::NativeProjectedCompilerWord,
    version: TclVersion,
    before: i32,
    after: i32,
) -> Option<IndexToken> {
    let Some(text) = word
        .literal
        .as_deref()
        .and_then(|literal| std::str::from_utf8(literal).ok())
    else {
        return Some(IndexToken::Dynamic);
    };
    Some(
        tcl_cmd_core::index::compiled_list_bound_in(text, version, before, after)
            .ok()?
            .map_or(IndexToken::Dynamic, IndexToken::Constant),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::{NativeCompilationFrame, NativeCompilationGrammar};

    fn original(
        source: &str,
        scope: NativeScalarScope,
        version: TclVersion,
    ) -> Option<NativeStringRangeInstruction> {
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
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::StringRange(scope),
            operation: crate::SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Inherit,
        };
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        if !matches!(
            spec.select_native_words(&words, 1, Some(dialect), context),
            NativeCompilationSelection::Inline {
                guard: NativeCompilationGuard::BeforeArguments,
                ..
            }
        ) {
            return None;
        }
        compile_native_string_range(&words, 1, scope, version)
    }

    #[test]
    fn original_c9_range_keeps_immediate_coordinates_and_subject_only() {
        for (source, scope, subject) in [
            (
                "string range $canonical 2 end",
                NativeScalarScope::PublicMember,
                2,
            ),
            (
                "::tcl::string::range $canonical 2 end",
                NativeScalarScope::PrivateOperands,
                1,
            ),
        ] {
            let recipe = original(source, scope, TclVersion::V9_0).unwrap();
            assert_eq!(recipe.operand, NativeCompilerWordOperand::Original(subject));
            let NativeStringRangeOperation::Immediate(range) = recipe.operation else {
                panic!("original STR_RANGE_IMM");
            };
            assert_eq!((range.first.encoded(), range.last.encoded()), (2, -2));
        }
        for source in [
            "string range [side_effect] end+2 end",
            "string range $s end+2 [side_effect]",
            "string range $s 0 -1",
        ] {
            assert_eq!(
                original(source, NativeScalarScope::PublicMember, TclVersion::V9_0)
                    .unwrap()
                    .operation,
                NativeStringRangeOperation::Empty
            );
        }
    }

    #[test]
    fn non_constant_indices_push_both_original_words_for_str_range() {
        for (source, first, last) in [
            ("string range $s $first end", 3, 4),
            ("string range $s 2 $last", 3, 4),
            ("string range $s bad end", 3, 4),
            ("string range $s 2 bad", 3, 4),
        ] {
            for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
                assert_eq!(
                    original(source, NativeScalarScope::PublicMember, version)
                        .unwrap()
                        .operation,
                    NativeStringRangeOperation::Indices {
                        first: NativeCompilerWordOperand::Original(first),
                        last: NativeCompilerWordOperand::Original(last),
                    },
                    "{source}"
                );
            }
        }
        // C8.6 encodes `end+1` as TCL_INDEX_AFTER before reaching the last word.
        assert_eq!(
            original(
                "string range $s end+1 [side_effect]",
                NativeScalarScope::PublicMember,
                TclVersion::V8_6
            )
            .unwrap()
            .operation,
            NativeStringRangeOperation::Empty
        );
    }

    #[test]
    fn original_unknown_indices_arity_expansion_and_other_releases_stay_unavailable() {
        for source in [
            "string range $s 2",
            "string range $s 2 end extra",
            "string ra $s 2 end",
            "string range {*}$args",
            // C9 `end+1` depends on the target's Tcl_Size width.
            "string range $s end+1 [side_effect]",
        ] {
            assert!(
                original(source, NativeScalarScope::PublicMember, TclVersion::V9_0).is_none(),
                "{source}"
            );
        }
        for version in [TclVersion::V8_4, TclVersion::V8_5] {
            assert!(
                original(
                    "string range $s 2 end",
                    NativeScalarScope::PublicMember,
                    version
                )
                .is_none()
            );
        }
    }
    #[test]
    fn foreign_operation_body_grammar_and_entry_mode_cannot_select_range() {
        let dialect = crate::InvocationDialect::for_version(TclVersion::V9_0);
        let source = "string range $s 2 end";
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
            grammar: NativeCompilationGrammar::StringRange(NativeScalarScope::PublicMember),
            operation: crate::SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Inherit,
        };
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            ..Default::default()
        };
        for foreign in [
            NativeCompilationSpec {
                operation: crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::StringRange),
                ..spec
            },
            NativeCompilationSpec {
                body: NativeBodyCompilation::Direct,
                ..spec
            },
            NativeCompilationSpec {
                grammar: NativeCompilationGrammar::NoHook,
                ..spec
            },
        ] {
            assert_eq!(
                select_original(
                    &words,
                    1,
                    NativeScalarScope::PublicMember,
                    TclVersion::V9_0,
                    foreign,
                    context
                ),
                NativeCompilationSelection::Unknown
            );
        }
        assert_eq!(
            select_original(
                &words,
                1,
                NativeScalarScope::PublicMember,
                TclVersion::V9_0,
                spec,
                NativeCompilationContext {
                    mode: NativeCompilationMode::Direct,
                    ..context
                }
            ),
            NativeCompilationSelection::Unknown
        );
        let effects =
            spec.original_argument_name_effects_in_context(&words, 1, Some(dialect), context);
        assert_eq!(
            effects,
            crate::native_compilation::NativeOriginalCompilerNameEffects::Visits(vec![
                crate::native_control_compilation::NativeControlPreparationStep::Word(
                    NativeCompilerWordOperand::Original(2)
                )
            ])
        );
    }
}
