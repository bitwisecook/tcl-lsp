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

//! Original Array compiler receivers, literal-list inspection and reached operand order.

use crate::native_compilation::{
    NativeArrayCommand, NativeCompilationContext, NativeCompilationFrame,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_info_exists_compilation::NativeInfoExistsReceiver;
use tcl_dialect::TclVersion;
use tcl_syntax::native_variable_words::{NativeVariableWordOperand, native_variable_word};

/// Selected array-existence result producer, independently from name lookup
/// and whether the original command is compiled or dispatched generically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeArrayExistenceResult {
    /// C8.4 and Jim create an Integer result rather than borrowing a constant.
    FreshInteger,
    /// C8.5+ publishes the actual interpreter execution-environment Boolean.
    ExecutionBooleanConstant,
}

/// Select only an authored actual-engine result recipe. This does not attest
/// an object, interpreter environment, source body or entered frame.
#[must_use]
pub fn native_array_existence_result(
    dialect: crate::InvocationDialect,
) -> Option<NativeArrayExistenceResult> {
    use tcl_syntax::native_string::NativeStringProtocol;
    match dialect.native_string_protocol()? {
        NativeStringProtocol::C(TclVersion::V8_4) | NativeStringProtocol::Jim084 => {
            Some(NativeArrayExistenceResult::FreshInteger)
        }
        NativeStringProtocol::C(
            TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1,
        ) => Some(NativeArrayExistenceResult::ExecutionBooleanConstant),
    }
}

/// Authentic inline Array operation; live receivers and callbacks are separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeArrayInstruction {
    /// Actual Array worker selected by its original registration.
    pub command: NativeArrayCommand,
    /// Same original array-name operand.
    pub operand: NativeCompilerWordOperand,
    /// Original variable layout, retaining counted root and source extents.
    pub receiver: NativeInfoExistsReceiver,
    /// Original RHS visited after `ARRAY_EXISTS/MAKE`, never before them.
    pub values: Option<NativeCompilerWordOperand>,
    /// Compile-known empty valid list selects ensure-array without foreach.
    pub empty: bool,
    /// Original compile-time List inspection proves even length without a runtime check.
    pub checked_even: bool,
}

/// Completed compiler-local prefix and its independently selected instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeArrayCompilation {
    /// Native scalar-name declarations survive a later compiler decline.
    pub declarations: Vec<Vec<u8>>,
    /// None preserves actual generic dispatch after the completed preparation.
    pub instruction: Option<NativeArrayInstruction>,
}

/// Capture completed original Array preparation, including a truthful decline.
#[must_use]
pub fn native_array_compilation(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    command: NativeArrayCommand,
    version: TclVersion,
    context: NativeCompilationContext,
) -> Option<NativeArrayCompilation> {
    if version < TclVersion::V8_6 || operand_from == 0 {
        return None;
    }
    let projected = project_native_compiler_words(words, version).ok()?;
    let arguments = projected.get(operand_from..)?;
    if arguments.iter().any(|word| {
        matches!(
            word.shape,
            crate::native_compilation::NativeCompilationWordShape::Expanded
                | crate::native_compilation::NativeCompilationWordShape::Opaque
        )
    }) {
        return None;
    }
    let instruction = compile_native_array(words, operand_from, command, version, context);
    let mut declarations = Vec::new();
    if instruction.is_none()
        && arguments.len()
            == if command == NativeArrayCommand::Set {
                2
            } else {
                1
            }
    {
        let values = arguments
            .get(1)
            .and_then(|word| word.literal.as_ref())
            .and_then(|bytes| {
                tcl_syntax::list::split_native_list_bytes(bytes, words.source_protocol()).ok()
            });
        let before_target = command != NativeArrayCommand::Set
            || values
                .as_ref()
                .is_none_or(|values| values.len().is_multiple_of(2))
                && matches!(
                    arguments[0].shape,
                    crate::native_compilation::NativeCompilationWordShape::Literal
                        | crate::native_compilation::NativeCompilationWordShape::QuotedLiteral
                        | crate::native_compilation::NativeCompilationWordShape::BracedLiteral
                )
                && (context.frame == NativeCompilationFrame::ProcedureCode
                    || values.as_ref().is_some_and(Vec::is_empty));
        if before_target
            && let NativeCompilerWordOperand::Original(index) = &arguments[0].operand
            && let Ok(
                NativeVariableWordOperand::Literal { name, .. }
                | NativeVariableWordOperand::CompoundArray { name, .. },
            ) = native_variable_word(
                &words.original_words()[*index],
                version,
                words.source_protocol(),
            )
        {
            declarations.push(name);
        }
    }
    Some(NativeArrayCompilation {
        declarations,
        instruction,
    })
}

/// Retain the exact C86+ inline Array compiler geometry.
#[must_use]
pub fn compile_native_array(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    command: NativeArrayCommand,
    version: TclVersion,
    context: NativeCompilationContext,
) -> Option<NativeArrayInstruction> {
    if version < TclVersion::V8_6 || operand_from == 0 {
        return None;
    }
    let projected = project_native_compiler_words(words, version).ok()?;
    let arguments = projected.get(operand_from..)?;
    if arguments.len()
        != if command == NativeArrayCommand::Set {
            2
        } else {
            1
        }
    {
        return None;
    }
    let target = &arguments[0];
    let values = arguments.get(1);
    let literal_list = values
        .and_then(|word| word.literal.as_ref())
        .and_then(|bytes| {
            tcl_syntax::list::split_native_list_bytes(bytes, words.source_protocol()).ok()
        });
    if literal_list
        .as_ref()
        .is_some_and(|values| !values.len().is_multiple_of(2))
    {
        return None;
    }
    let empty = literal_list.as_ref().is_some_and(Vec::is_empty);
    if command == NativeArrayCommand::Set
        && (!matches!(
            target.shape,
            crate::native_compilation::NativeCompilationWordShape::Literal
                | crate::native_compilation::NativeCompilationWordShape::QuotedLiteral
                | crate::native_compilation::NativeCompilationWordShape::BracedLiteral
        ) || context.frame != NativeCompilationFrame::ProcedureCode && !empty)
    {
        return None;
    }
    let receiver = match &target.operand {
        NativeCompilerWordOperand::Original(index) => NativeInfoExistsReceiver::Original(
            native_variable_word(
                &words.original_words()[*index],
                version,
                words.source_protocol(),
            )
            .ok()?,
        ),
        NativeCompilerWordOperand::LiteralExpansion { value, .. } => {
            let (name, index) = tcl_syntax::naming::split_element_ref_bytes(value)
                .map_or((value.as_slice(), None), |(name, index)| {
                    (name, Some(index))
                });
            NativeInfoExistsReceiver::ExpandedLiteral {
                name: name.to_vec(),
                index: index.map(<[u8]>::to_vec),
            }
        }
    };
    if matches!(
        &receiver,
        NativeInfoExistsReceiver::Original(
            NativeVariableWordOperand::Literal { index: Some(_), .. }
                | NativeVariableWordOperand::CompoundArray { .. }
        ) | NativeInfoExistsReceiver::ExpandedLiteral { index: Some(_), .. }
    ) {
        return None;
    }
    Some(NativeArrayInstruction {
        command,
        operand: target.operand.clone(),
        receiver,
        values: values.map(|word| word.operand.clone()),
        empty,
        checked_even: literal_list.is_some(),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn array_existence_result_recipes_keep_provider_and_purpose_boundaries() {
        // naming.compiler.introspection-source-and-effect-frontiers
        // docs/design/analysis/name-resolution-proofs/compiler-introspection-source-and-effect-frontiers.md
        for (engine, expected) in [
            ("tcl8.4", NativeArrayExistenceResult::FreshInteger),
            (
                "tcl8.5",
                NativeArrayExistenceResult::ExecutionBooleanConstant,
            ),
            (
                "tcl8.6",
                NativeArrayExistenceResult::ExecutionBooleanConstant,
            ),
            (
                "tcl9.0",
                NativeArrayExistenceResult::ExecutionBooleanConstant,
            ),
            (
                "tcl9.1",
                NativeArrayExistenceResult::ExecutionBooleanConstant,
            ),
            ("jim", NativeArrayExistenceResult::FreshInteger),
        ] {
            let profile = crate::model::ingress::resolve_environment(engine).unit_profile();
            assert_eq!(
                native_array_existence_result(crate::InvocationDialect::of_profile(profile)),
                Some(expected),
                "{engine}"
            );
        }
        for engine in ["f5-irules", "f5-iapps", "tcl"] {
            let profile = crate::model::ingress::resolve_environment(engine).unit_profile();
            assert_eq!(
                native_array_existence_result(crate::InvocationDialect::of_profile(profile)),
                None,
                "{engine}"
            );
        }
    }

    use super::*;
    include!("../tests/data/native_introspection_compilation/cases.rs");

    #[test]
    fn original_array_recipes_match_sixty_native_compiler_windows() {
        let mut windows = 0;
        for (version, table) in [
            (
                TclVersion::V8_4,
                include_str!("../tests/data/native_introspection_compilation/8.4.20.tsv"),
            ),
            (
                TclVersion::V8_5,
                include_str!("../tests/data/native_introspection_compilation/8.5.19.tsv"),
            ),
            (
                TclVersion::V8_6,
                include_str!("../tests/data/native_introspection_compilation/8.6.18.tsv"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../tests/data/native_introspection_compilation/9.0.4.tsv"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../tests/data/native_introspection_compilation/9.1.0.tsv"),
            ),
        ] {
            let dialect = crate::InvocationDialect::for_version(version);
            for row in table.lines().skip(15).take(12) {
                let fields: Vec<_> = row.split('\t').collect();
                let case = fields[0].parse::<usize>().unwrap();
                let source = CASES[case];
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
                let command = match case {
                    15..=17 => NativeArrayCommand::Exists,
                    18..=23 => NativeArrayCommand::Set,
                    _ => NativeArrayCommand::Unset,
                };
                let context = NativeCompilationContext {
                    mode: crate::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ProcedureCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                };
                let native_inline = fields[9]
                    .split(',')
                    .any(|op| matches!(op, "arrayExistsImm" | "arrayExistsStk"));
                assert_eq!(
                    compile_native_array(&words, 2, command, version, context).is_some(),
                    native_inline,
                    "{version:?}/{case}"
                );
                if version >= TclVersion::V8_6 && case == 17 {
                    assert_eq!(
                        native_array_compilation(&words, 2, command, version, context)
                            .unwrap()
                            .declarations,
                        vec![b"a".to_vec()],
                        "actual completed array-name declaration prefix"
                    );
                }
                windows += 1;
            }
        }
        assert_eq!(windows, 60);
    }
}
