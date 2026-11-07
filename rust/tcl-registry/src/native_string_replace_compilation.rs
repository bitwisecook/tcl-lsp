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

//! `TclCompileStringReplaceCmd` (C8.6, C9.0 and C9.1).
//!
//! The subject word is always compiled first. Compile-known indices select
//! one of the immediate-range recipes; otherwise `genericReplace` pushes the
//! index words and the replacement (or `""`) for `STR_REPLACE`. Any other
//! word count fails the member compiler.

use crate::native_compilation::{
    NativeCompilationSelection, NativeCompilationWordShape as Shape,
    NativeCompilerImplementationLookup, NativeNamedInvocationProtocol,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, NativeProjectedCompilerWord, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_scalar_compilation::NativeScalarScope;
use tcl_dialect::TclVersion;
use tcl_syntax::native_compiled_index::{
    NativeCompiledListIndex as Index, NativeCompiledListRange,
};

pub(crate) const STRING_REPLACE_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "replace",
        slot: "::tcl::string::replace",
        command: "string",
        prepended: &["replace"],
    };

const START: i32 = 0;
const END: i32 = -2;
const BEFORE_OR_NONE: i32 = -1;

/// The instructions emitted after the subject word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStringReplaceOperation {
    /// The subject is the result; a replacement word is pushed and popped.
    Unchanged,
    /// `POP` the subject and push `""`.
    Empty,
    /// One `STR_RANGE_IMM` over the subject.
    Range(NativeCompiledListRange),
    /// `DUP`, the head range, a swap, the tail range, `STR_CONCAT1 2`.
    Splice {
        /// Last kept index before the replaced span.
        head: Index,
        /// First kept index after it.
        tail: Index,
    },
    /// Push the replacement and swap; then `POP` the subject, or append its
    /// tail range with `STR_CONCAT1 2`.
    Prefix {
        /// First kept index after the replaced span, if any.
        tail: Option<Index>,
    },
    /// C8.6: the head range, then the replacement, `STR_CONCAT1 2`.
    Suffix {
        /// Last kept index before the replaced span.
        head: Index,
    },
    /// `genericReplace`: push both index words and the replacement (or
    /// `""`), then `STR_REPLACE`.
    Generic,
}

/// One selected `TclCompileStringReplaceCmd` recipe over the original words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeStringReplaceInstruction {
    /// The original subject word.
    pub subject: NativeCompilerWordOperand,
    /// The original first-index word.
    pub first: NativeCompilerWordOperand,
    /// The original last-index word.
    pub last: NativeCompilerWordOperand,
    /// The original replacement word, when written.
    pub replacement: Option<NativeCompilerWordOperand>,
    /// The instructions after the subject.
    pub operation: NativeStringReplaceOperation,
}

impl NativeStringReplaceInstruction {
    /// Original words the recipe compiles, in order.
    #[must_use]
    pub fn compiled_words(&self) -> Vec<NativeCompilerWordOperand> {
        use NativeStringReplaceOperation as Operation;
        let mut words = vec![self.subject.clone()];
        match self.operation {
            Operation::Generic => {
                words.extend([self.first.clone(), self.last.clone()]);
                words.extend(self.replacement.clone());
            }
            Operation::Unchanged | Operation::Prefix { .. } | Operation::Suffix { .. } => {
                words.extend(self.replacement.clone());
            }
            Operation::Empty | Operation::Range(_) | Operation::Splice { .. } => {}
        }
        words
    }
}

enum IndexToken {
    Constant(i32),
    Dynamic,
}

fn index_token(
    word: &NativeProjectedCompilerWord,
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
            .map_or(IndexToken::Dynamic, |index| {
                IndexToken::Constant(index.encoded())
            }),
    )
}

fn range(first: i32, last: i32) -> NativeStringReplaceOperation {
    NativeStringReplaceOperation::Range(NativeCompiledListRange {
        first: Index::from_encoded(first),
        last: Index::from_encoded(last),
    })
}

/// The C branch for compile-known encoded indices. `None` keeps refusal
/// where C's `first - 1` or `last + 1` would leave the `int` range.
fn constant_operation(
    first: i32,
    last: i32,
    replaced: bool,
    version: TclVersion,
) -> Option<NativeStringReplaceOperation> {
    use NativeStringReplaceOperation as Operation;
    let c86 = version == TclVersion::V8_6;
    let first_after = if c86 { i32::MAX } else { BEFORE_OR_NONE };
    if last == BEFORE_OR_NONE
        || first == first_after
        || (first <= END && last <= END && last < first)
        || (first >= START && last >= START && last < first)
    {
        return Some(Operation::Unchanged);
    }
    let head = || first.checked_sub(1).map(Index::from_encoded);
    let tail = || last.checked_add(1).map(Index::from_encoded);
    if replaced {
        let prefix_first = if c86 { BEFORE_OR_NONE } else { START };
        if first == prefix_first && last >= START {
            return Some(Operation::Prefix {
                tail: if last == i32::MAX {
                    None
                } else {
                    Some(tail()?)
                },
            });
        }
        if c86 && last == i32::MAX && first <= END {
            return Some(Operation::Suffix { head: head()? });
        }
        return Some(Operation::Generic);
    }
    let from_start = first == START || (c86 && first == BEFORE_OR_NONE);
    let to_end = last == END || (c86 && last == i32::MAX);
    Some(match (from_start, to_end) {
        (true, true) => Operation::Empty,
        (true, false) => range(tail()?.encoded(), END),
        (false, true) => range(START, head()?.encoded()),
        (false, false) => Operation::Splice {
            head: head()?,
            tail: tail()?,
        },
    })
}

/// Project the original compiler's recipe from complete original words.
#[must_use]
pub fn compile_native_string_replace(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
) -> Option<NativeStringReplaceInstruction> {
    if !matches!(
        version,
        TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
    ) || words.source_protocol() != tcl_syntax::native_string::NativeStringProtocol::C(version)
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
    if public && projected.get(operand_from)?.literal.as_deref() != Some(b"replace") {
        return None;
    }
    let from = operand_from.checked_add(usize::from(public))?;
    let (subject, first, last, replacement) = match projected.get(from..)? {
        [subject, first, last] => (subject, first, last, None),
        [subject, first, last, replacement] => (subject, first, last, Some(replacement)),
        _ => return None,
    };
    let (first_before, first_after, last_before, last_after) = if version == TclVersion::V8_6 {
        (BEFORE_OR_NONE, i32::MAX, BEFORE_OR_NONE, i32::MAX)
    } else {
        (START, BEFORE_OR_NONE, BEFORE_OR_NONE, END)
    };
    let operation = match (
        index_token(first, version, first_before, first_after)?,
        index_token(last, version, last_before, last_after)?,
    ) {
        (IndexToken::Constant(first), IndexToken::Constant(last)) => {
            constant_operation(first, last, replacement.is_some(), version)?
        }
        _ => NativeStringReplaceOperation::Generic,
    };
    Some(NativeStringReplaceInstruction {
        subject: subject.operand.clone(),
        first: first.operand.clone(),
        last: last.operand.clone(),
        replacement: replacement.map(|word| word.operand.clone()),
        operation,
    })
}

/// Select the original compiler outcome, including its failed word counts.
#[must_use]
pub fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if compile_native_string_replace(words, operand_from, scope, version).is_some() {
        return Selection::Inline {
            operation: crate::SemanticOperationId::Invoke,
            guard: crate::native_compilation::NativeCompilationGuard::BeforeArguments,
        };
    }
    let public = scope == NativeScalarScope::PublicMember;
    let count = words
        .shapes()
        .len()
        .checked_sub(operand_from + usize::from(public));
    if !matches!(
        version,
        TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
    ) || words.source_protocol() != tcl_syntax::native_string::NativeStringProtocol::C(version)
        || operand_from == 0
        || words
            .shapes()
            .iter()
            .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded))
        || (public && words.literal(operand_from) != Some(b"replace".as_slice()))
        || count.is_none_or(|count| matches!(count, 3 | 4))
    {
        return Selection::Unknown;
    }
    if public {
        Selection::NamedInvocation {
            lookup: &STRING_REPLACE_IMPLEMENTATION,
            arguments_from: 1,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        }
    } else {
        Selection::Generic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use NativeStringReplaceOperation as Operation;

    fn recipe(source: &str, version: TclVersion) -> Option<NativeStringReplaceInstruction> {
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
        compile_native_string_replace(&words, 1, NativeScalarScope::PublicMember, version)
    }

    fn operation(source: &str, version: TclVersion) -> Operation {
        recipe(source, version).unwrap().operation
    }

    #[test]
    fn four_word_constant_indices_select_immediate_ranges() {
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(
                operation("string replace $s 0 end", version),
                Operation::Empty
            );
            assert_eq!(operation("string replace $s 0 1", version), range(2, END));
            assert_eq!(operation("string replace $s 2 end", version), range(0, 1));
            assert_eq!(
                operation("string replace $s 1 end-1", version),
                Operation::Splice {
                    head: Index::from_encoded(0),
                    tail: Index::from_encoded(END),
                }
            );
            for source in [
                "string replace $s 2 1",
                "string replace $s end end-1",
                "string replace $s 0 -1",
            ] {
                assert_eq!(operation(source, version), Operation::Unchanged, "{source}");
            }
        }
        // C8.6 folds a negative first index into TCL_INDEX_BEFORE.
        assert_eq!(
            operation("string replace $s -5 end", TclVersion::V8_6),
            Operation::Empty
        );
    }

    #[test]
    fn five_word_prefixes_follow_each_release() {
        assert_eq!(
            operation("string replace $s 0 1 x", TclVersion::V9_0),
            Operation::Prefix {
                tail: Some(Index::from_encoded(2))
            }
        );
        // C8.6 tests for TCL_INDEX_BEFORE, which a literal 0 is not.
        assert_eq!(
            operation("string replace $s 0 1 x", TclVersion::V8_6),
            Operation::Generic
        );
        assert_eq!(
            operation("string replace $s -1 1 x", TclVersion::V8_6),
            Operation::Prefix {
                tail: Some(Index::from_encoded(2))
            }
        );
        assert_eq!(
            operation("string replace $s end-1 2147483647 x", TclVersion::V8_6),
            Operation::Suffix {
                head: Index::from_encoded(-4)
            }
        );
        assert_eq!(
            operation("string replace $s 1 2 x", TclVersion::V9_0),
            Operation::Generic
        );
    }

    #[test]
    fn dynamic_indices_push_every_original_word() {
        use NativeCompilerWordOperand::Original;
        let recipe = recipe("string replace $s $i end $r", TclVersion::V9_0).unwrap();
        assert_eq!(recipe.operation, Operation::Generic);
        assert_eq!(
            recipe.compiled_words(),
            [Original(2), Original(3), Original(4), Original(5)]
        );
        assert_eq!(
            operation("string replace $s bad end", TclVersion::V9_1),
            Operation::Generic
        );
    }

    #[test]
    fn range_errors_replace_generically_and_other_releases_refuse() {
        assert_eq!(
            operation("string replace $s end+1 end", TclVersion::V9_0),
            Operation::Generic
        );
        assert!(recipe("string replace $s 0 1", TclVersion::V8_5).is_none());
        assert!(recipe("string replace $s 0", TclVersion::V9_0).is_none());
    }
}
