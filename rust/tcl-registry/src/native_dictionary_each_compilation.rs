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

//! `CompileDictEachCmd` (`dict for` and `dict map`, C8.6 to C9.1) and C8.5's
//! `TclCompileDictForCmd`.
//!
//! In a procedure body a two-name variable list of local scalars and a
//! literal body compile to a `DICT_FIRST`/`DICT_NEXT` loop under a catch
//! range that releases the iterator. Every other shape is the private
//! worker's `Basic3Arg` invocation (C8.5: an ordinary invocation).

use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    NativeCompilationWordShape, NativeCompiledBodyContext,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_control_compilation::{
    NativeControlCompilation, NativeControlOutcome, NativeControlPreparationStep,
    native_control_body_span, project_native_local_scalar,
};
use tcl_dialect::TclVersion;
use tcl_lexer::Span;

/// The inline dictionary iteration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDictionaryEachInstruction {
    /// Compiler release; C9.1 swaps where earlier releases use `OVER 1`.
    pub version: TclVersion,
    /// `dict map` collects each body result under its key.
    pub collect: bool,
    /// The key variable's local name.
    pub key: Vec<u8>,
    /// The value variable's local name.
    pub value: Vec<u8>,
    /// The original dictionary operand.
    pub dictionary: NativeCompilerWordOperand,
    /// The original body operand.
    pub body: NativeCompilerWordOperand,
    /// The body's extent in the original source.
    pub body_span: Span,
}

fn simple(shape: NativeCompilationWordShape) -> bool {
    matches!(
        shape,
        NativeCompilationWordShape::Literal
            | NativeCompilationWordShape::QuotedLiteral
            | NativeCompilationWordShape::BracedLiteral
    )
}

/// The original compiler's outcome over complete words. `None` is missing
/// original evidence; a `Generic` outcome is the `Basic3Arg` fallback.
#[must_use]
pub fn compile_native_dictionary_each(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: NativeCompilationContext,
    collect: bool,
) -> Option<NativeControlCompilation<NativeDictionaryEachInstruction>> {
    use NativeControlPreparationStep as Step;
    let mut selected = NativeControlCompilation {
        outcome: NativeControlOutcome::Generic,
        preparations: Vec::new(),
    };
    let c85 = version == TclVersion::V8_5;
    if !matches!(
        version,
        TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
    ) || (c85 && collect)
        || context.mode != NativeCompilationMode::BytecodeObject
    {
        return None;
    }
    if context.frame != NativeCompilationFrame::ProcedureCode {
        return Some(selected);
    }
    let projected = project_native_compiler_words(words, version).ok()?;
    let operands = projected.get(operand_from..)?;
    if operands
        .iter()
        .any(|word| word.shape == NativeCompilationWordShape::Expanded)
    {
        return None;
    }
    let [variables, dictionary, body] = operands else {
        return Some(selected);
    };
    // C8.5 reads any compile-known variable list; later releases need a
    // simple word.
    if (!c85 && !simple(variables.shape)) || !simple(body.shape) {
        return Some(selected);
    }
    if collect {
        selected.preparations.push(Step::DeclareAnonymousLocal);
    }
    let Ok(names) = tcl_syntax::list::split_native_list_bytes(
        variables.literal.as_deref()?,
        tcl_syntax::native_string::NativeStringProtocol::C(version),
    ) else {
        return Some(selected);
    };
    let [key, value] = names.as_slice() else {
        return Some(selected);
    };
    let mut scalars = true;
    for name in [key, value] {
        let projection = project_native_local_scalar(name, version);
        if let Some(declaration) = projection.declaration {
            selected
                .preparations
                .push(Step::DeclareLocal(declaration.to_vec()));
        }
        scalars &= projection.scalar;
        // C8.5 returns before it reaches the value name.
        if c85 && !scalars {
            break;
        }
    }
    if !scalars {
        return Some(selected);
    }
    selected.preparations.push(Step::DeclareAnonymousLocal);
    selected
        .preparations
        .push(Step::Word(dictionary.operand.clone()));
    let body_span = native_control_body_span(words, &body.operand).ok()?;
    selected.preparations.push(Step::Script {
        operand: body.operand.clone(),
        span: body_span,
        context: NativeCompiledBodyContext::ProtectedLoop,
    });
    if !collect {
        selected.preparations.push(Step::Literal(Vec::new()));
    }
    selected.outcome = NativeControlOutcome::Inline(NativeDictionaryEachInstruction {
        version,
        collect,
        key: key.to_vec(),
        value: value.to_vec(),
        dictionary: dictionary.operand.clone(),
        body: body.operand.clone(),
        body_span,
    });
    Some(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compiled(
        source: &str,
        version: TclVersion,
        frame: NativeCompilationFrame,
        collect: bool,
    ) -> Option<NativeControlCompilation<NativeDictionaryEachInstruction>> {
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
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame,
            ..Default::default()
        };
        compile_native_dictionary_each(&words, 2, version, context, collect)
    }

    #[test]
    fn procedure_bodies_declare_locals_in_compiler_order() {
        use NativeCompilerWordOperand::Original;
        use NativeControlPreparationStep as Step;
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let map = compiled(
                "dict map {k v} $d {set v}",
                version,
                NativeCompilationFrame::ProcedureCode,
                true,
            )
            .unwrap();
            assert!(matches!(map.outcome, NativeControlOutcome::Inline(_)));
            assert_eq!(
                map.preparations[..5],
                [
                    Step::DeclareAnonymousLocal,
                    Step::DeclareLocal(b"k".to_vec()),
                    Step::DeclareLocal(b"v".to_vec()),
                    Step::DeclareAnonymousLocal,
                    Step::Word(Original(3)),
                ]
            );
            let each = compiled(
                "dict for {k v} $d {set v}",
                version,
                NativeCompilationFrame::ProcedureCode,
                false,
            )
            .unwrap();
            assert_eq!(each.preparations.last(), Some(&Step::Literal(Vec::new())));
        }
    }

    #[test]
    fn other_shapes_take_the_basic_invocation() {
        for (source, frame) in [
            (
                "dict for {k v} $d {set v}",
                NativeCompilationFrame::ScriptCode,
            ),
            (
                "dict for {k} $d {set v}",
                NativeCompilationFrame::ProcedureCode,
            ),
            (
                "dict for {k v w} $d {}",
                NativeCompilationFrame::ProcedureCode,
            ),
            (
                "dict for {a(1) v} $d {}",
                NativeCompilationFrame::ProcedureCode,
            ),
            (
                "dict for $vars $d {}",
                NativeCompilationFrame::ProcedureCode,
            ),
            (
                "dict for {k v} $d $body",
                NativeCompilationFrame::ProcedureCode,
            ),
            ("dict for {k v} $d", NativeCompilationFrame::ProcedureCode),
        ] {
            let outcome = compiled(source, TclVersion::V9_0, frame, false).unwrap();
            assert!(
                matches!(outcome.outcome, NativeControlOutcome::Generic),
                "{source}"
            );
        }
    }
}
