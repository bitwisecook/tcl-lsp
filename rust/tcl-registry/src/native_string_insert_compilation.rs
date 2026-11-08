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

//! `TclCompileStringInsertCmd` (C9.0 and C9.1).
//!
//! A compile-known index selects a prepend, an append or a splice of the
//! string and the insertion. A dynamic index, or any word count but three
//! operands, fails the member compiler.

use crate::native_compilation::{
    NativeCompilationSelection, NativeCompilationWordShape as Shape,
    NativeCompilerImplementationLookup, NativeNamedInvocationProtocol,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_scalar_compilation::NativeScalarScope;
use tcl_dialect::TclVersion;

pub(crate) const STRING_INSERT_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "insert",
        slot: "::tcl::string::insert",
        command: "string",
        prepended: &["insert"],
    };

const START: i32 = 0;
const END: i32 = -2;

/// The instructions after both operands are pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStringInsertOperation {
    /// Swap, then `STR_CONCAT1 2`.
    Prepend,
    /// `STR_CONCAT1 2`.
    Append,
    /// `OVER 1`, `STR_RANGE_IMM 0 head`, `REVERSE 3`, `STR_RANGE_IMM tail end`,
    /// `STR_CONCAT1 3`.
    Splice {
        /// Encoded last index of the kept prefix.
        head: i32,
        /// Encoded first index of the kept suffix.
        tail: i32,
    },
}

/// One selected `TclCompileStringInsertCmd` recipe over the original words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeStringInsertInstruction {
    /// The original string word.
    pub subject: NativeCompilerWordOperand,
    /// The original insertion word.
    pub insertion: NativeCompilerWordOperand,
    /// The instructions after both words.
    pub operation: NativeStringInsertOperation,
}

impl NativeStringInsertInstruction {
    /// Original words the recipe compiles, in order.
    #[must_use]
    pub fn compiled_words(&self) -> Vec<NativeCompilerWordOperand> {
        vec![self.subject.clone(), self.insertion.clone()]
    }
}

fn applies(words: &NativeCompilerWords<'_>, operand_from: usize, version: TclVersion) -> bool {
    matches!(version, TclVersion::V9_0 | TclVersion::V9_1)
        && words.source_protocol() == tcl_syntax::native_string::NativeStringProtocol::C(version)
        && operand_from != 0
        && !words
            .shapes()
            .iter()
            .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded))
}

/// Project the original compiler's recipe from complete original words.
#[must_use]
pub fn compile_native_string_insert(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
) -> Option<NativeStringInsertInstruction> {
    if !applies(words, operand_from, version) {
        return None;
    }
    let projected = project_native_compiler_words(words, version).ok()?;
    let public = scope == NativeScalarScope::PublicMember;
    if public && projected.get(operand_from)?.literal.as_deref() != Some(b"insert") {
        return None;
    }
    let from = operand_from.checked_add(usize::from(public))?;
    let [subject, index, insertion] = projected.get(from..)? else {
        return None;
    };
    let text = std::str::from_utf8(index.literal.as_deref()?).ok()?;
    let mut index = tcl_cmd_core::index::compiled_list_bound_in(text, version, START, END)
        .ok()??
        .encoded();
    let operation = match index {
        START => NativeStringInsertOperation::Prepend,
        END => NativeStringInsertOperation::Append,
        _ => {
            // As for `linsert`, an end-relative index inserts after it.
            if index < END {
                index = index.checked_add(1)?;
            }
            NativeStringInsertOperation::Splice {
                head: index.checked_sub(1)?,
                tail: index,
            }
        }
    };
    Some(NativeStringInsertInstruction {
        subject: subject.operand.clone(),
        insertion: insertion.operand.clone(),
        operation,
    })
}

/// Select the original compiler outcome, including its failed forms.
#[must_use]
pub fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if compile_native_string_insert(words, operand_from, scope, version).is_some() {
        return Selection::Inline {
            operation: crate::SemanticOperationId::Invoke,
            guard: crate::native_compilation::NativeCompilationGuard::BeforeArguments,
        };
    }
    let public = scope == NativeScalarScope::PublicMember;
    if !applies(words, operand_from, version)
        || (public && words.literal(operand_from) != Some(b"insert".as_slice()))
    {
        return Selection::Unknown;
    }
    // A compile-known index the encoder rejects is not modelled.
    let index_at = operand_from + usize::from(public) + 1;
    if words.shapes().len() == index_at + 2
        && let Some(text) = words
            .literal(index_at)
            .and_then(|literal| std::str::from_utf8(literal).ok())
        && tcl_cmd_core::index::compiled_list_bound_in(text, version, START, END).is_err()
    {
        return Selection::Unknown;
    }
    if public {
        Selection::NamedInvocation {
            lookup: &STRING_INSERT_IMPLEMENTATION,
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
    use NativeStringInsertOperation as Operation;

    fn recipe(source: &str, version: TclVersion) -> Option<NativeStringInsertInstruction> {
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
        compile_native_string_insert(&words, 1, NativeScalarScope::PublicMember, version)
    }

    #[test]
    fn compile_known_indices_select_the_release_recipe() {
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let operation = |source| recipe(source, version).map(|recipe| recipe.operation);
            assert_eq!(operation("string insert $s 0 $x"), Some(Operation::Prepend));
            assert_eq!(
                operation("string insert $s -5 $x"),
                Some(Operation::Prepend)
            );
            assert_eq!(
                operation("string insert $s end $x"),
                Some(Operation::Append)
            );
            assert_eq!(
                operation("string insert $s 2 $x"),
                Some(Operation::Splice { head: 1, tail: 2 })
            );
            assert_eq!(
                operation("string insert $s end-1 $x"),
                Some(Operation::Splice {
                    head: END - 1,
                    tail: END
                })
            );
            assert_eq!(operation("string insert $s $i $x"), None);
            assert_eq!(operation("string insert $s 1"), None);
        }
        assert_eq!(recipe("string insert $s 1 $x", TclVersion::V8_6), None);
    }
}
