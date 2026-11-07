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

//! `TclCompileLreplaceCmd` (C8.6, C9.0 and C9.1).
//!
//! C9 compiles every word and emits one `LREPLACE4` (C9.1 `LREPLACE`) with
//! `END_IS_LAST`. C8.6 needs both indices compile-known and of one sign; it
//! then splices with list ranges, and otherwise declines to an ordinary
//! invocation.

use crate::native_compilation::{NativeCompilationSelection, NativeCompilationWordShape as Shape};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, NativeProjectedCompilerWord, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;
use tcl_syntax::native_compiled_index::{
    NativeCompiledListIndex as Index, NativeCompiledListRange,
};

const START: i32 = 0;
const END: i32 = -2;
const BEFORE: i32 = -1;
const AFTER: i32 = i32::MAX;

/// One C8.6 stack step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeListReplaceStep {
    /// Compile this original word.
    Operand(NativeCompilerWordOperand),
    /// `LIST` over the last pushed values.
    List(usize),
    /// `DUP`.
    Dup,
    /// `OVER` at this depth.
    Over(usize),
    /// `REVERSE` this many values.
    Reverse(usize),
    /// `LIST_RANGE_IMM`.
    Range(NativeCompiledListRange),
    /// `LIST_CONCAT`.
    Concat,
    /// `POP`.
    Pop,
    /// Push the empty literal.
    Empty,
}

/// The original compiler's recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeListReplaceInstruction {
    /// C9: push every original word, then `LREPLACE4`/`LREPLACE` with
    /// `END_IS_LAST`.
    Replace {
        /// List, first, last, then the new elements.
        operands: Vec<NativeCompilerWordOperand>,
    },
    /// C8.6's splice over compile-known indices.
    Splice(Vec<NativeListReplaceStep>),
}

impl NativeListReplaceInstruction {
    /// Original words the recipe compiles, in order.
    #[must_use]
    pub fn compiled_words(&self) -> Vec<NativeCompilerWordOperand> {
        match self {
            Self::Replace { operands } => operands.clone(),
            Self::Splice(steps) => steps
                .iter()
                .filter_map(|step| match step {
                    NativeListReplaceStep::Operand(operand) => Some(operand.clone()),
                    _ => None,
                })
                .collect(),
        }
    }
}

enum Outcome {
    Inline(NativeListReplaceInstruction),
    Generic,
}

/// `TclGetIndexFromToken`'s outcome for one original index word.
enum IndexToken {
    Constant(i32),
    /// Not compile-known: C8.6 declines to an ordinary invocation.
    Dynamic,
}

fn index_token(word: &NativeProjectedCompilerWord, before: i32, after: i32) -> Option<IndexToken> {
    let Some(text) = word
        .literal
        .as_deref()
        .and_then(|literal| std::str::from_utf8(literal).ok())
    else {
        return Some(IndexToken::Dynamic);
    };
    Some(
        tcl_cmd_core::index::compiled_list_bound_in(text, TclVersion::V8_6, before, after)
            .ok()?
            .map_or(IndexToken::Dynamic, |index| {
                IndexToken::Constant(index.encoded())
            }),
    )
}

fn range(first: i32, last: i32) -> NativeListReplaceStep {
    NativeListReplaceStep::Range(NativeCompiledListRange {
        first: Index::from_encoded(first),
        last: Index::from_encoded(last),
    })
}

fn c86_splice(
    source: &NativeProjectedCompilerWord,
    first: i32,
    last: i32,
    elements: &[NativeProjectedCompilerWord],
) -> Option<Outcome> {
    use NativeListReplaceStep as Step;
    let suffix_start = if first == AFTER || last == BEFORE {
        first
    } else if last == END {
        AFTER
    } else if (last < END && first <= END) || (last >= START && first >= START) {
        first.max(last.checked_add(1)?)
    } else {
        return Some(Outcome::Generic);
    };
    let mut steps = vec![Step::Operand(source.operand.clone())];
    let mut empty_prefix = true;
    if !elements.is_empty() {
        steps.extend(
            elements
                .iter()
                .map(|word| Step::Operand(word.operand.clone())),
        );
        steps.push(Step::List(elements.len()));
        empty_prefix = false;
    }
    if first == suffix_start && elements.is_empty() {
        steps.push(range(START, END));
        return Some(Outcome::Inline(NativeListReplaceInstruction::Splice(steps)));
    }
    if first != START {
        steps.push(if empty_prefix {
            Step::Dup
        } else {
            Step::Over(1)
        });
        steps.push(range(START, first.checked_sub(1)?));
        if !empty_prefix {
            steps.extend([Step::Reverse(2), Step::Concat]);
        }
        empty_prefix = false;
    }
    if !empty_prefix {
        steps.push(Step::Reverse(2));
    }
    if suffix_start == AFTER {
        steps.push(Step::Pop);
        if empty_prefix {
            steps.push(Step::Empty);
        }
    } else {
        steps.push(range(suffix_start, END));
        if !empty_prefix {
            steps.push(Step::Concat);
        }
    }
    Some(Outcome::Inline(NativeListReplaceInstruction::Splice(steps)))
}

fn outcome(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Option<Outcome> {
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
    let operands = projected.get(operand_from..)?;
    let [source, first, last, elements @ ..] = operands else {
        return Some(Outcome::Generic);
    };
    if version != TclVersion::V8_6 {
        return Some(Outcome::Inline(NativeListReplaceInstruction::Replace {
            operands: operands.iter().map(|word| word.operand.clone()).collect(),
        }));
    }
    match (
        index_token(first, START, AFTER)?,
        index_token(last, BEFORE, END)?,
    ) {
        (IndexToken::Constant(first), IndexToken::Constant(last)) => {
            c86_splice(source, first, last, elements)
        }
        _ => Some(Outcome::Generic),
    }
}

/// The original recipe when the compiler accepts these words.
#[must_use]
pub fn compile_native_list_replace(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Option<NativeListReplaceInstruction> {
    match outcome(words, operand_from, version)? {
        Outcome::Inline(instruction) => Some(instruction),
        Outcome::Generic => None,
    }
}

/// Select the original compiler outcome.
#[must_use]
pub fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> NativeCompilationSelection {
    match outcome(words, operand_from, version) {
        Some(Outcome::Inline(_)) => NativeCompilationSelection::Inline {
            operation: crate::SemanticOperationId::Invoke,
            guard: crate::native_compilation::NativeCompilationGuard::BeforeArguments,
        },
        Some(Outcome::Generic) => NativeCompilationSelection::Generic,
        None if matches!(version, TclVersion::V8_4 | TclVersion::V8_5) => {
            NativeCompilationSelection::Generic
        }
        None => NativeCompilationSelection::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use NativeCompilerWordOperand::Original;
    use NativeListReplaceStep as Step;

    fn compiled(source: &str, version: TclVersion) -> NativeCompilationSelection {
        with_words(source, version, |words| select_original(words, 1, version))
    }

    fn recipe(source: &str, version: TclVersion) -> Option<NativeListReplaceInstruction> {
        with_words(source, version, |words| {
            compile_native_list_replace(words, 1, version)
        })
    }

    fn with_words<T>(
        source: &str,
        version: TclVersion,
        select: impl FnOnce(&NativeCompilerWords<'_>) -> T,
    ) -> T {
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
        select(&words)
    }

    #[test]
    fn c9_pushes_every_word_for_lreplace4() {
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(
                recipe("lreplace $l $i end x y", version),
                Some(NativeListReplaceInstruction::Replace {
                    operands: vec![
                        Original(1),
                        Original(2),
                        Original(3),
                        Original(4),
                        Original(5)
                    ],
                })
            );
            assert_eq!(
                compiled("lreplace $l 0", version),
                NativeCompilationSelection::Generic
            );
        }
    }

    #[test]
    fn c86_splices_constant_indices() {
        let version = TclVersion::V8_6;
        assert_eq!(
            recipe("lreplace $l 0 end", version),
            Some(NativeListReplaceInstruction::Splice(vec![
                Step::Operand(Original(1)),
                Step::Pop,
                Step::Empty,
            ]))
        );
        assert_eq!(
            recipe("lreplace $l 1 1 x", version),
            Some(NativeListReplaceInstruction::Splice(vec![
                Step::Operand(Original(1)),
                Step::Operand(Original(4)),
                Step::List(1),
                Step::Over(1),
                range(0, 0),
                Step::Reverse(2),
                Step::Concat,
                Step::Reverse(2),
                range(2, END),
                Step::Concat,
            ]))
        );
        assert_eq!(
            recipe("lreplace $l 3 1", version),
            Some(NativeListReplaceInstruction::Splice(vec![
                Step::Operand(Original(1)),
                range(0, END),
            ]))
        );
    }

    #[test]
    fn c86_declines_dynamic_and_mixed_sign_indices() {
        for source in [
            "lreplace $l $i 1",
            "lreplace $l 1 end-1",
            "lreplace $l bad 1",
        ] {
            assert_eq!(
                compiled(source, TclVersion::V8_6),
                NativeCompilationSelection::Generic,
                "{source}"
            );
        }
        assert_eq!(
            compiled("lreplace $l 0 0", TclVersion::V8_5),
            NativeCompilationSelection::Generic
        );
    }
}
