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

//! Original C list-index compiler operands and immediate-index selection.

use crate::native_compilation::NativeCompilationWordShape as Shape;
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;
use tcl_syntax::native_compiled_index::NativeCompiledListIndex;

/// Exact native instruction after original operands have been evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeListIndexOperation {
    /// One original index object, including its native index-list fallback.
    Single,
    /// Original List plus this many total evaluated stack operands.
    Multi(u32),
    /// Compile-known native coordinate; no index object is evaluated or retained.
    Immediate(NativeCompiledListIndex),
}

/// Ordered parser operands of one independently admitted `ListIndex` compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeListIndexInstruction {
    /// List followed by each original index, excluding an immediate index.
    pub operands: Vec<NativeCompilerWordOperand>,
    /// Actual selected native stack operation.
    pub operation: NativeListIndexOperation,
}

/// Retain unavailable geometry or target-width evidence separately from a
/// malformed index, which the native compiler evaluates at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeListIndexUnavailable {
    /// Original parser vector or operand count is inconsistent.
    Geometry,
    /// Native compiler selection must have declined unresolved expansion.
    Expansion,
    /// Native immediate selection depends on unavailable container-size width.
    Encoding,
}

/// Select original `ListIndex` compilation after native parser expansion.
/// Registration and compilation context remain independent caller obligations.
#[must_use]
pub fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    spec: crate::native_compilation::NativeCompilationSpec,
) -> crate::native_compilation::NativeCompilationSelection {
    use crate::native_compilation::NativeCompilationSelection as Selection;
    let Ok(projected) = project_native_compiler_words(words, version) else {
        return Selection::Unknown;
    };
    if operand_from == 0 {
        return Selection::Unknown;
    }
    let Some(arguments) = projected.get(operand_from..) else {
        return Selection::Unknown;
    };
    if arguments.is_empty() {
        return Selection::Generic;
    }
    match compile_native_list_index(words, operand_from, version) {
        Ok(_) => Selection::Inline {
            operation: spec.operation,
            guard: if version == TclVersion::V8_4 {
                crate::native_compilation::NativeCompilationGuard::ChunkEntry
            } else {
                crate::native_compilation::NativeCompilationGuard::BeforeArguments
            },
        },
        Err(NativeListIndexUnavailable::Expansion) => Selection::Generic,
        Err(NativeListIndexUnavailable::Geometry | NativeListIndexUnavailable::Encoding) => {
            Selection::Unknown
        }
    }
}

/// Select operands only; the caller must authenticate the actual compiler.
///
/// # Errors
/// Rejects missing original geometry, unresolved expansion and unknown native
/// immediate-index width.
pub fn compile_native_list_index(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Result<NativeListIndexInstruction, NativeListIndexUnavailable> {
    use NativeListIndexUnavailable as Unavailable;
    let projected =
        project_native_compiler_words(words, version).map_err(|_| Unavailable::Geometry)?;
    let arguments = projected.get(operand_from..).ok_or(Unavailable::Geometry)?;
    if operand_from == 0 || arguments.is_empty() {
        return Err(Unavailable::Geometry);
    }
    if arguments.iter().any(|word| word.shape == Shape::Expanded) {
        return Err(Unavailable::Expansion);
    }
    let immediate = if let [_, index] = arguments {
        if version == TclVersion::V8_5 && index.shape == Shape::BackslashLiteral {
            None
        } else {
            index
                .literal
                .as_deref()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(|text| tcl_cmd_core::index::compiled_list_index_in(text, version))
                .transpose()
                .map_err(|_| Unavailable::Encoding)?
                .flatten()
        }
    } else {
        None
    };
    let (operation, count) = match immediate {
        Some(index) => (NativeListIndexOperation::Immediate(index), 1),
        None if arguments.len() == 2 => (NativeListIndexOperation::Single, 2),
        None => (
            NativeListIndexOperation::Multi(
                u32::try_from(arguments.len()).map_err(|_| Unavailable::Geometry)?,
            ),
            arguments.len(),
        ),
    };
    Ok(NativeListIndexInstruction {
        operands: arguments[..count]
            .iter()
            .map(|word| word.operand.clone())
            .collect(),
        operation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    };
    include!("../tests/data/native_list_index_compilation/cases.rs");
    pub const TABLES: &[(TclVersion, &str)] = &[
        (
            TclVersion::V8_4,
            include_str!("../tests/data/native_list_index_compilation/8.4.20.txt"),
        ),
        (
            TclVersion::V8_5,
            include_str!("../tests/data/native_list_index_compilation/8.5.19.txt"),
        ),
        (
            TclVersion::V8_6,
            include_str!("../tests/data/native_list_index_compilation/8.6.18.txt"),
        ),
        (
            TclVersion::V9_0,
            include_str!("../tests/data/native_list_index_compilation/9.0.4.txt"),
        ),
        (
            TclVersion::V9_1,
            include_str!("../tests/data/native_list_index_compilation/9.1.0.txt"),
        ),
    ];
    #[test]
    fn original_list_index_recipes_match_95_native_instruction_and_result_windows() {
        // Native proof naming.list-index.literal-native-instruction-coordinates:
        // docs/design/analysis/name-resolution-proofs/list-index-literal-native-instruction-coordinates.md
        // Native proof naming.list-index.multi-path-and-empty-validation:
        // docs/design/analysis/name-resolution-proofs/list-index-multi-path-and-empty-validation.md
        // Native proof naming.list-index.expansion-and-abrupt-child-boundaries:
        // docs/design/analysis/name-resolution-proofs/list-index-expansion-and-abrupt-child-boundaries.md
        let registry = crate::CommandRegistry::build_default();
        let mut windows = 0;
        for &(version, table) in TABLES {
            let dialect = crate::InvocationDialect::for_version(version);
            let spec = registry
                .native_compilation_for_registration("lindex", dialect)
                .unwrap();
            for row in table.lines() {
                let fields: Vec<_> = row.split('|').collect();
                let source = CASES[fields[0].parse::<usize>().unwrap()];
                if fields[7].is_empty() {
                    windows += 1;
                    continue;
                }
                let parsed = tcl_lexer::native_script_words_in(
                    tcl_lexer::SourceImage::native(source.as_bytes()),
                    tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                    tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
                )
                .unwrap();
                let words = NativeCompilerWords::capture(
                    &parsed.commands[0].words,
                    tcl_syntax::native_string::NativeStringProtocol::C(version),
                )
                .unwrap();
                let context = NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ProcedureCode,
                    ..Default::default()
                };
                let selection = spec.select_native_words(&words, 1, Some(dialect), context);
                if !fields[7].is_empty() {
                    assert!(
                        matches!(
                            selection,
                            crate::native_compilation::NativeCompilationSelection::Inline { .. }
                        ),
                        "{version:?}/{source}"
                    );
                    let recipe = compile_native_list_index(&words, 1, version).unwrap();
                    let operation = match recipe.operation {
                        NativeListIndexOperation::Single => {
                            if version == TclVersion::V8_4 {
                                "listindex".to_owned()
                            } else {
                                "listIndex".to_owned()
                            }
                        }
                        NativeListIndexOperation::Multi(count) => format!("lindexMulti:{count}"),
                        NativeListIndexOperation::Immediate(index) => {
                            format!("listIndexImm:{}", index.encoded())
                        }
                    };
                    assert_eq!(operation, fields[7], "{version:?}/{source}");
                }
                windows += 1;
            }
        }
        assert_eq!(windows, 95);
    }
    #[test]
    fn compiled_index_retains_target_width_uncertainty_and_malformed_runtime_operands() {
        assert!(
            tcl_cmd_core::index::compiled_list_index_in("2147483648", TclVersion::V9_0).is_err()
        );
        assert_eq!(
            tcl_cmd_core::index::compiled_list_index_in("bad", TclVersion::V9_0).unwrap(),
            None
        );
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(
                tcl_cmd_core::index::compiled_list_index_in("-1", version)
                    .unwrap()
                    .unwrap()
                    .encoded(),
                -1,
            );
            assert_eq!(
                tcl_cmd_core::index::compiled_list_index_in("end+1", version)
                    .unwrap()
                    .map(NativeCompiledListIndex::encoded),
                (version == TclVersion::V8_6).then_some(-1),
            );
            let end = tcl_cmd_core::index::compiled_list_index_in("end-1", version)
                .unwrap()
                .unwrap();
            assert_eq!(end.encoded(), -3);
            assert_eq!(end.resolve(3), Some(1));
            assert_eq!(end.resolve(0), None);
        }
    }

    #[test]
    fn original_list_index_expansion_preserves_decline_and_projected_operands() {
        let registry = crate::CommandRegistry::build_default();
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let dialect = crate::InvocationDialect::for_version(version);
            let spec = registry
                .native_compilation_for_registration("lindex", dialect)
                .unwrap();
            let context = NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ProcedureCode,
                ..Default::default()
            };
            for (source, compiled) in [("lindex $x {*}$i", false), ("lindex {*}{a b} 0", true)] {
                let parsed = tcl_lexer::native_script_words_in(
                    tcl_lexer::SourceImage::native(source.as_bytes()),
                    tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                    tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
                )
                .unwrap();
                let words = NativeCompilerWords::capture(
                    &parsed.commands[0].words,
                    tcl_syntax::native_string::NativeStringProtocol::C(version),
                )
                .unwrap();
                let selected = spec.select_native_words(&words, 1, Some(dialect), context);
                if compiled {
                    assert!(matches!(
                        selected,
                        crate::native_compilation::NativeCompilationSelection::Inline { .. }
                    ));
                    let recipe = compile_native_list_index(&words, 1, version).unwrap();
                    assert_eq!(recipe.operation, NativeListIndexOperation::Multi(3));
                    assert!(matches!(
                        recipe.operands[0],
                        NativeCompilerWordOperand::LiteralExpansion {
                            original_word: 1,
                            ..
                        }
                    ));
                } else {
                    assert_eq!(
                        selected,
                        crate::native_compilation::NativeCompilationSelection::Generic
                    );
                    assert_eq!(
                        compile_native_list_index(&words, 1, version),
                        Err(NativeListIndexUnavailable::Expansion)
                    );
                }
                assert_eq!(
                    spec.select_native_words(&words, 1, None, context),
                    crate::native_compilation::NativeCompilationSelection::Unknown
                );
            }
        }
    }
}
