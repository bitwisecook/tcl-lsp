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

//! Original Error compiler stack preparation and immediate error completion.

use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;

/// One selected Error stack operation, separate from compiler admission.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeErrorStep {
    /// Evaluate this unchanged original operand.
    Word(NativeCompilerWordOperand),
    /// Push the selected compiler-owned String literal.
    Literal(Vec<u8>),
    /// Construct the original ordered options List.
    List(u32),
    /// Insert the original key/value into the current options Dictionary.
    DictionaryPut,
    /// Apply native `RETURN_IMM` with error code one and level zero.
    ReturnError,
    /// Discard the value on top of the stack.
    Pop,
    /// Throw's run-time type check over `msg -errorcode type`: a non-empty
    /// type raises it, an empty one leaves the BADEXCEPTION result and
    /// options for the following [`Self::ReturnError`].
    ThrowTypeCheck,
}

/// Native Error preparation in compiler and runtime evaluation order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeErrorInstruction {
    /// Original operands and compiler-owned options construction.
    pub steps: Vec<NativeErrorStep>,
}

/// Actual compiler decline is distinct from unavailable original geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeErrorCompilationUnavailable {
    /// This release, arity or unresolved expansion uses ordinary invocation.
    Generic,
    /// The retained parser/source vector is unavailable.
    Geometry,
}

/// Plan Error for already selected original argv operands. This pure stack
/// recipe grants no registration, source provenance or callback authority.
///
/// # Errors
/// Returns native decline for an unsupported release or argument count.
pub fn native_error_instruction(
    operands: &[NativeCompilerWordOperand],
    version: TclVersion,
) -> Result<NativeErrorInstruction, NativeErrorCompilationUnavailable> {
    use NativeErrorStep as Step;
    if version < TclVersion::V8_6 || !(1..=3).contains(&operands.len()) {
        return Err(NativeErrorCompilationUnavailable::Generic);
    }
    let mut steps = vec![Step::Word(operands[0].clone())];
    if operands.len() == 1 || version >= TclVersion::V9_1 {
        steps.push(Step::Literal(Vec::new()));
    }
    for (operand, keyword) in operands[1..]
        .iter()
        .zip([b"-errorinfo".as_slice(), b"-errorcode".as_slice()])
    {
        steps.push(Step::Literal(keyword.to_vec()));
        steps.push(Step::Word(operand.clone()));
        if version >= TclVersion::V9_1 {
            steps.push(Step::DictionaryPut);
        }
    }
    if operands.len() > 1 && version < TclVersion::V9_1 {
        steps.push(Step::List(
            u32::try_from(2 * (operands.len() - 1)).expect("bounded native Error operands"),
        ));
    }
    steps.push(Step::ReturnError);
    Ok(NativeErrorInstruction { steps })
}

/// Select the actual Error compiler from the parser-projected original vector.
/// Literal expansion retains original member spans, without generated Tcl.
///
/// # Errors
/// Distinguishes native decline from unavailable source geometry.
pub fn compile_native_error(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Result<NativeErrorInstruction, NativeErrorCompilationUnavailable> {
    let projected = project_native_compiler_words(words, version)
        .map_err(|_| NativeErrorCompilationUnavailable::Geometry)?;
    let operands = projected
        .get(operand_from..)
        .ok_or(NativeErrorCompilationUnavailable::Geometry)?;
    if operands
        .iter()
        .any(|word| word.shape == crate::native_compilation::NativeCompilationWordShape::Expanded)
    {
        return Err(NativeErrorCompilationUnavailable::Generic);
    }
    native_error_instruction(
        &operands
            .iter()
            .map(|word| word.operand.clone())
            .collect::<Vec<_>>(),
        version,
    )
}

/// Plan `throw type message` as `TclCompileThrowCmd` does: a literal valid
/// type becomes one options literal, a literal empty list raises
/// BADEXCEPTION, and any other type is checked at run time.
///
/// # Errors
/// Native decline for an unsupported release, arity or expansion; geometry
/// for a literal type that is not a list, which C reports as a compile-time
/// syntax error this recipe does not model.
pub fn compile_native_throw(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Result<NativeErrorInstruction, NativeErrorCompilationUnavailable> {
    use NativeErrorStep as Step;
    if version < TclVersion::V8_6 {
        return Err(NativeErrorCompilationUnavailable::Generic);
    }
    let projected = project_native_compiler_words(words, version)
        .map_err(|_| NativeErrorCompilationUnavailable::Geometry)?;
    let operands = projected
        .get(operand_from..)
        .ok_or(NativeErrorCompilationUnavailable::Geometry)?;
    if operands.len() != 2
        || operands.iter().any(|word| {
            word.shape == crate::native_compilation::NativeCompilationWordShape::Expanded
        })
    {
        return Err(NativeErrorCompilationUnavailable::Generic);
    }
    let code = &operands[0].operand;
    let message = Step::Word(operands[1].operand.clone());
    let known = match code {
        NativeCompilerWordOperand::Original(index) => words.literal(*index),
        _ => None,
    };
    let Some(literal) = known else {
        return Ok(NativeErrorInstruction {
            steps: vec![
                Step::Word(code.clone()),
                Step::Literal(b"-errorcode".to_vec()),
                message,
                Step::ThrowTypeCheck,
                Step::ReturnError,
            ],
        });
    };
    let elements = tcl_syntax::list::split_native_list_bytes(literal, words.source_protocol())
        .map_err(|_| NativeErrorCompilationUnavailable::Geometry)?;
    let steps = if elements.is_empty() {
        vec![
            message,
            Step::Pop,
            Step::Literal(THROW_EMPTY_TYPE_MESSAGE.to_vec()),
            Step::Literal(THROW_EMPTY_TYPE_OPTIONS.to_vec()),
            Step::ReturnError,
        ]
    } else {
        let mut options = b"-errorcode ".to_vec();
        tcl_syntax::list::append_list_element(&mut options, literal, false);
        vec![message, Step::Literal(options), Step::ReturnError]
    };
    Ok(NativeErrorInstruction { steps })
}

/// Result of `throw` with an empty type.
pub const THROW_EMPTY_TYPE_MESSAGE: &[u8] = b"type must be non-empty list";
/// Options literal `TclCompileThrowCmd` pushes for an empty type.
pub const THROW_EMPTY_TYPE_OPTIONS: &[u8] = b"-errorcode {TCL OPERATION THROW BADEXCEPTION}";

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    fn compile(
        source: &[u8],
        version: TclVersion,
    ) -> Result<NativeErrorInstruction, NativeErrorCompilationUnavailable> {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let image = SourceImage::native(source);
        let commands = native_script_words_in(
            image,
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &commands.commands[0].words,
            NativeStringProtocol::C(version),
        )
        .unwrap();
        compile_native_error(&words, 1, version)
    }

    #[test]
    fn error_options_follow_actual_list_and_dictionary_instruction_order() {
        for (version, rows) in [
            (
                TclVersion::V8_6,
                include_str!("../tests/data/native_error_compilation/8.6.tsv"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../tests/data/native_error_compilation/9.0.tsv"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../tests/data/native_error_compilation/9.1.tsv"),
            ),
        ] {
            let recipe = compile(
                b"error [set message BODY] [set info STACK] [set code {FOO BAR}]",
                version,
            )
            .unwrap();
            let row = rows.lines().nth(3).unwrap().split('\t').collect::<Vec<_>>();
            assert_eq!(row[3], "message info code");
            let dict_count = row[4]
                .split_whitespace()
                .filter(|op| *op == "dictPut")
                .count();
            assert_eq!(
                recipe
                    .steps
                    .iter()
                    .filter(|step| matches!(step, NativeErrorStep::DictionaryPut))
                    .count(),
                dict_count
            );
            assert_eq!(dict_count == 2, version == TclVersion::V9_1);
            assert_eq!(
                recipe
                    .steps
                    .iter()
                    .filter_map(|step| if let NativeErrorStep::Word(word) = step {
                        Some(word.clone())
                    } else {
                        None
                    })
                    .collect::<Vec<_>>(),
                [
                    NativeCompilerWordOperand::Original(1),
                    NativeCompilerWordOperand::Original(2),
                    NativeCompilerWordOperand::Original(3)
                ]
            );
            assert_eq!(recipe.steps.last(), Some(&NativeErrorStep::ReturnError));
            assert_eq!(
                matches!(recipe.steps.get(1), Some(NativeErrorStep::Literal(value)) if value.is_empty()),
                version == TclVersion::V9_1
            );
            let expanded = compile(b"error {*}{BODY STACK {FOO BAR}}", version).unwrap();
            assert_eq!(
                expanded
                    .steps
                    .iter()
                    .filter_map(|step| if let NativeErrorStep::Word(word) = step {
                        Some(word)
                    } else {
                        None
                    })
                    .count(),
                3
            );
            assert!(expanded.steps.iter().all(|step| !matches!(
                step,
                NativeErrorStep::Word(NativeCompilerWordOperand::Original(_))
            )));
        }
    }

    #[test]
    fn error_compiler_declines_missing_registration_arity_and_runtime_expansion() {
        for version in [TclVersion::V8_4, TclVersion::V8_5] {
            assert_eq!(
                compile(b"error BODY", version),
                Err(NativeErrorCompilationUnavailable::Generic)
            );
        }
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            for source in [b"error".as_slice(), b"error A B C D", b"error {*}$args"] {
                assert_eq!(
                    compile(source, version),
                    Err(NativeErrorCompilationUnavailable::Generic)
                );
            }
        }
    }
}
