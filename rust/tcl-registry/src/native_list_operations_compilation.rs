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

//! Original list operators retain native operand/store order and coordinates.

use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;
use tcl_syntax::{
    native_compiled_index::NativeCompiledListIndex,
    native_variable_words::{NativeVariableWordOperand, native_variable_word},
};

/// Independently selected native list compiler operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeListOperationKind {
    /// Compile-known inclusive range coordinates, C8.6 and later.
    Range,
    /// Ordered original target evaluations interleaved with stores, C8.5+.
    Assign,
    /// C8.6 splice instructions or C9 original LREPLACE4 operands.
    Insert,
    /// Original variable preparation followed by index/value, read and store.
    Set,
}

/// Original variable operand after native parser expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeListVariableOperand {
    /// An unchanged source word owns its root/index geometry.
    Original {
        /// Exact original word, not a diagnostic name reconstruction.
        operand: NativeCompilerWordOperand,
        /// Shared native compiler variable-word preparation.
        variable: NativeVariableWordOperand,
    },
    /// A parser-expanded TEXT member retains its own literal object recipe.
    ExpandedLiteral {
        /// Genuine parser member and its original expansion source.
        operand: NativeCompilerWordOperand,
        /// Counted root/scalar spelling.
        name: Vec<u8>,
        /// Literal element bytes, including a distinct empty index.
        index: Option<Vec<u8>>,
    },
}

/// Actual C9 insertion instruction selected by the native release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeListInsertionInstruction {
    /// C9.0 consumes list, first, last and replacement List objects.
    ReplaceFour,
    /// C9.1 consumes the original list/index and counted replacement objects.
    Replace,
}

impl NativeListInsertionInstruction {
    /// Exact native disassembly operation; this does not grant execution.
    #[must_use]
    pub const fn opcode_name(self) -> &'static str {
        match self {
            Self::ReplaceFour => "lreplace4",
            Self::Replace => "lreplace",
        }
    }
}

/// Native insertion evaluates its index only on C9 and later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeListInsertionIndex {
    /// C8.6 clamped coordinate; no runtime index object is created.
    Immediate(NativeCompiledListIndex),
    /// Original C9 index, evaluated after the list and before inserted elements.
    Original {
        /// Original index object evaluation.
        operand: NativeCompilerWordOperand,
        /// Release-specific actual native replacement instruction.
        instruction: NativeListInsertionInstruction,
    },
}

/// Complete native list stack recipe, independent of its registration issuer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeListOperationInstruction {
    /// Evaluate the original list and reach its selected native range getter.
    Range {
        /// Original list operand.
        list: NativeCompilerWordOperand,
        /// Native first coordinate with the release's before/after clamp.
        first: NativeCompiledListIndex,
        /// Native last coordinate with the release's before/after clamp.
        last: NativeCompiledListIndex,
    },
    /// Keep the original list while each target is evaluated and stored.
    Assign {
        /// Original list operand, converted at the first list-index instruction.
        list: NativeCompilerWordOperand,
        /// Original ordered target preparations and actual stores.
        targets: Vec<NativeListVariableOperand>,
    },
    /// The C8.6 list/range/concat burst or C9 LREPLACE4 insertion.
    Insert {
        /// Original list operand.
        list: NativeCompilerWordOperand,
        /// Actual native index evaluation strategy.
        index: NativeListInsertionIndex,
        /// Inserted original objects, evaluated in written order.
        elements: Vec<NativeCompilerWordOperand>,
    },
    /// Prepare the receiver, evaluate indices/value, then read and update it.
    Set {
        /// Original receiver and its native compiler geometry.
        target: NativeListVariableOperand,
        /// One index uses native index-List fallback; other counts use flat indices.
        indices: Vec<NativeCompilerWordOperand>,
        /// Original replacement value, evaluated before reading the receiver.
        value: NativeCompilerWordOperand,
    },
}

/// Distinguish native compiler decline from unavailable original evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeListOperationUnavailable {
    /// Native syntax/version/compiler geometry declines this optimisation.
    Geometry,
    /// Original parser source or expanded member ownership is unavailable.
    Source,
    /// Native immediate encoding depends on unavailable container width.
    IndexEncoding,
}

/// Retain an original native list compiler recipe without granting execution.
///
/// # Errors
/// Rejects native declines or unavailable original source/index evidence.
pub fn compile_native_list_operation(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    kind: NativeListOperationKind,
) -> Result<NativeListOperationInstruction, NativeListOperationUnavailable> {
    use NativeListOperationInstruction as Plan;
    use NativeListOperationUnavailable as U;
    let projected = original_list_operation_words(words, operand_from, version)?;
    let operands = &projected[operand_from..];
    Ok(match kind {
        NativeListOperationKind::Range => {
            let [source_list, first, last] = operands else {
                return Err(U::Geometry);
            };
            if version < TclVersion::V8_6 {
                return Err(U::Geometry);
            }
            let first = bound(
                first,
                version,
                0,
                if version < TclVersion::V9_0 {
                    i32::MAX
                } else {
                    -1
                },
            )?;
            if first.encoded() == -1 {
                return Err(U::Geometry);
            }
            Plan::Range {
                list: source_list.operand.clone(),
                first,
                last: bound(last, version, -1, -2)?,
            }
        }
        NativeListOperationKind::Insert => {
            let [list, index, elements @ ..] = operands else {
                return Err(U::Geometry);
            };
            if version < TclVersion::V8_6 {
                return Err(U::Geometry);
            }
            let index = if version < TclVersion::V9_0 {
                NativeListInsertionIndex::Immediate(bound(index, version, 0, -2)?)
            } else {
                NativeListInsertionIndex::Original {
                    operand: index.operand.clone(),
                    instruction: if version == TclVersion::V9_0 {
                        NativeListInsertionInstruction::ReplaceFour
                    } else {
                        NativeListInsertionInstruction::Replace
                    },
                }
            };
            // C8.6 validates the compile-known index first, then emits
            // LIST_RANGE_IMM 0,end for the no-element listiness check.
            // C9 retains its separate runtime-index insertion recipe.
            if version == TclVersion::V8_6 && elements.is_empty() {
                return Ok(Plan::Range {
                    list: list.operand.clone(),
                    first: NativeCompiledListIndex::from_encoded(0),
                    last: NativeCompiledListIndex::from_encoded(-2),
                });
            }
            Plan::Insert {
                list: list.operand.clone(),
                index,
                elements: elements.iter().map(|w| w.operand.clone()).collect(),
            }
        }
        NativeListOperationKind::Assign => {
            let [list, targets @ ..] = operands else {
                return Err(U::Geometry);
            };
            if version < TclVersion::V8_5 || targets.is_empty() {
                return Err(U::Geometry);
            }
            Plan::Assign {
                list: list.operand.clone(),
                targets: targets
                    .iter()
                    .map(|w| variable(words, w, version))
                    .collect::<Result<_, _>>()?,
            }
        }
        NativeListOperationKind::Set => {
            let [target, rest @ ..] = operands else {
                return Err(U::Geometry);
            };
            let [indices @ .., value] = rest else {
                return Err(U::Geometry);
            };
            Plan::Set {
                target: variable(words, target, version)?,
                indices: indices.iter().map(|w| w.operand.clone()).collect(),
                value: value.operand.clone(),
            }
        }
    })
}

fn original_list_operation_words(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Result<
    Vec<crate::native_compiler_word_projection::NativeProjectedCompilerWord>,
    NativeListOperationUnavailable,
> {
    use NativeListOperationUnavailable as U;
    let projected = project_native_compiler_words(words, version).map_err(|_| U::Source)?;
    let operands = projected
        .get(operand_from..)
        .filter(|_| operand_from > 0)
        .ok_or(U::Geometry)?;
    if operands
        .iter()
        .any(|word| word.shape == crate::native_compilation::NativeCompilationWordShape::Expanded)
    {
        return Err(U::Geometry);
    }
    Ok(projected)
}

fn bound(
    word: &crate::native_compiler_word_projection::NativeProjectedCompilerWord,
    version: TclVersion,
    before: i32,
    after: i32,
) -> Result<NativeCompiledListIndex, NativeListOperationUnavailable> {
    use NativeListOperationUnavailable as U;
    let bytes = word.literal.as_deref().ok_or(U::Geometry)?;
    let text = std::str::from_utf8(bytes).map_err(|_| U::Geometry)?;
    tcl_cmd_core::index::compiled_list_bound_in(text, version, before, after)
        .map_err(|_| U::IndexEncoding)?
        .ok_or(U::Geometry)
}

fn variable(
    words: &NativeCompilerWords<'_>,
    word: &crate::native_compiler_word_projection::NativeProjectedCompilerWord,
    version: TclVersion,
) -> Result<NativeListVariableOperand, NativeListOperationUnavailable> {
    use NativeListOperationUnavailable as U;
    Ok(match &word.operand {
        NativeCompilerWordOperand::Original(index) => NativeListVariableOperand::Original {
            operand: word.operand.clone(),
            variable: native_variable_word(
                &words.original_words()[*index],
                version,
                words.source_protocol(),
            )
            .map_err(|_| U::Source)?,
        },
        NativeCompilerWordOperand::LiteralExpansion { value, .. } => {
            let (name, index) = tcl_syntax::naming::split_element_ref_bytes(value)
                .map_or((value.as_slice(), None), |(n, i)| (n, Some(i)));
            NativeListVariableOperand::ExpandedLiteral {
                operand: word.operand.clone(),
                name: name.to_vec(),
                index: index.map(<[u8]>::to_vec),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;
    include!("../tests/data/native_list_operations/cases.rs");
    fn original(source: &[u8], version: TclVersion) -> Vec<tcl_lexer::NativeWord> {
        let dialect = crate::InvocationDialect::for_version(version);
        native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap()
        .commands
        .remove(0)
        .words
    }
    fn recipe(
        source: &[u8],
        version: TclVersion,
        kind: NativeListOperationKind,
    ) -> Result<NativeListOperationInstruction, NativeListOperationUnavailable> {
        let original = original(source, version);
        let words =
            NativeCompilerWords::capture(&original, NativeStringProtocol::C(version)).unwrap();
        compile_native_list_operation(&words, 1, version, kind)
    }
    #[test]
    fn original_list_operations_match_150_native_compile_windows() {
        // Native proof: naming.list.original-set-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-set-objects-and-instructions.md

        // Native proof: naming.list.original-assign-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-assign-objects-and-instructions.md

        // Native proof: naming.list.original-insert-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-insert-objects-and-instructions.md

        // Native proof: naming.list.original-range-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-range-objects-and-instructions.md

        let mut count = 0;
        for (version, rows) in [
            (
                TclVersion::V8_4,
                include_str!("../tests/data/native_list_operations/8.4.20.txt"),
            ),
            (
                TclVersion::V8_5,
                include_str!("../tests/data/native_list_operations/8.5.19.txt"),
            ),
            (
                TclVersion::V8_6,
                include_str!("../tests/data/native_list_operations/8.6.18.txt"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../tests/data/native_list_operations/9.0.4.txt"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../tests/data/native_list_operations/9.1.0.txt"),
            ),
        ] {
            for row in rows.lines().filter(|r| r.starts_with("R|")) {
                let fields: Vec<_> = row.split('|').collect();
                let case = fields[1].parse::<usize>().unwrap();
                let source = CASES[case];
                let (kind, opcode) = if source.starts_with("lrange") {
                    (NativeListOperationKind::Range, "listRangeImm")
                } else if source.starts_with("linsert") {
                    (
                        NativeListOperationKind::Insert,
                        if version == TclVersion::V8_6 && case == 10 {
                            "listRangeImm"
                        } else if version < TclVersion::V9_0 {
                            "list"
                        } else if version == TclVersion::V9_0 {
                            NativeListInsertionInstruction::ReplaceFour.opcode_name()
                        } else {
                            NativeListInsertionInstruction::Replace.opcode_name()
                        },
                    )
                } else if source.starts_with("lassign") {
                    (NativeListOperationKind::Assign, "listIndexImm")
                } else {
                    (NativeListOperationKind::Set, "lset")
                };
                assert_eq!(
                    recipe(source.as_bytes(), version, kind).is_ok(),
                    fields[7].split(',').any(|observed| {
                        let instruction = observed.split(':').next();
                        if kind == NativeListOperationKind::Set {
                            matches!(instruction, Some("lsetList" | "lsetFlat"))
                        } else {
                            instruction == Some(opcode)
                        }
                    }),
                    "{version:?}/{case}"
                );
                if version == TclVersion::V8_6 && case == 10 {
                    assert!(
                        matches!(recipe(source.as_bytes(), version, kind), Ok(NativeListOperationInstruction::Range { first, last, .. }) if first.encoded() == 0 && last.encoded() == -2)
                    );
                }
                count += 1;
            }
        }
        assert_eq!(count, 150);
    }
    #[test]
    fn list_assignment_retains_interleaved_original_target_geometry() {
        // Native proof: naming.list.assignment-interleaved-target-effects
        // docs/design/analysis/name-resolution-proofs/list.assignment-interleaved-target-effects.md

        let NativeListOperationInstruction::Assign { list, targets } = recipe(
            b"lassign $list [first] a([second])",
            TclVersion::V8_6,
            NativeListOperationKind::Assign,
        )
        .unwrap() else {
            panic!("actual assignment recipe")
        };
        assert_eq!(list, NativeCompilerWordOperand::Original(1));
        assert!(matches!(
            &targets[0],
            NativeListVariableOperand::Original {
                operand: NativeCompilerWordOperand::Original(2),
                variable: NativeVariableWordOperand::DynamicWord
            }
        ));
        assert!(matches!(
            &targets[1],
            NativeListVariableOperand::Original {
                operand: NativeCompilerWordOperand::Original(3),
                variable: NativeVariableWordOperand::CompoundArray { .. }
            }
        ));
    }
    #[test]
    fn range_boundary_and_insert_index_recipes_preserve_release_selection() {
        let NativeListOperationInstruction::Range { first, last, .. } = recipe(
            b"lrange $list -8 end+9",
            TclVersion::V8_6,
            NativeListOperationKind::Range,
        )
        .unwrap() else {
            panic!("actual range")
        };
        assert_eq!((first.encoded(), last.encoded()), (0, -2));
        assert!(
            recipe(
                b"lrange $list end+9 end",
                TclVersion::V9_0,
                NativeListOperationKind::Range
            )
            .is_err()
        );
        assert!(
            recipe(
                b"linsert $list $index X",
                TclVersion::V8_6,
                NativeListOperationKind::Insert
            )
            .is_err()
        );
        assert!(matches!(
            recipe(
                b"linsert $list $index X",
                TclVersion::V9_0,
                NativeListOperationKind::Insert
            )
            .unwrap(),
            NativeListOperationInstruction::Insert {
                index: NativeListInsertionIndex::Original {
                    operand: NativeCompilerWordOperand::Original(2),
                    instruction: NativeListInsertionInstruction::ReplaceFour,
                },
                ..
            }
        ));
    }
    #[test]
    fn insertion_retains_the_exact_c90_and_c91_instruction() {
        for (version, expected) in [
            (
                TclVersion::V9_0,
                NativeListInsertionInstruction::ReplaceFour,
            ),
            (TclVersion::V9_1, NativeListInsertionInstruction::Replace),
        ] {
            let NativeListOperationInstruction::Insert {
                index:
                    NativeListInsertionIndex::Original {
                        operand,
                        instruction,
                    },
                ..
            } = recipe(
                b"linsert $list $index X",
                version,
                NativeListOperationKind::Insert,
            )
            .unwrap()
            else {
                panic!("native C9 insertion");
            };
            assert_eq!(operand, NativeCompilerWordOperand::Original(2));
            assert_eq!(instruction, expected);
        }
    }
    #[test]
    fn list_set_preserves_operand_order_and_unresolved_expansion_declines() {
        let NativeListOperationInstruction::Set {
            target,
            indices,
            value,
        } = recipe(
            b"lset a([key]) [first] [second] [value]",
            TclVersion::V8_6,
            NativeListOperationKind::Set,
        )
        .unwrap()
        else {
            panic!("original list update")
        };
        assert!(matches!(
            target,
            NativeListVariableOperand::Original {
                operand: NativeCompilerWordOperand::Original(1),
                variable: NativeVariableWordOperand::CompoundArray { .. }
            }
        ));
        assert_eq!(
            indices,
            vec![
                NativeCompilerWordOperand::Original(2),
                NativeCompilerWordOperand::Original(3)
            ]
        );
        assert_eq!(value, NativeCompilerWordOperand::Original(4));
        for (source, kind) in [
            (
                b"lassign $list {*}$unknown".as_slice(),
                NativeListOperationKind::Assign,
            ),
            (b"lset x {*}$unknown", NativeListOperationKind::Set),
            (
                b"linsert $list 0 {*}$unknown",
                NativeListOperationKind::Insert,
            ),
        ] {
            assert_eq!(
                recipe(source, TclVersion::V9_0, kind),
                Err(NativeListOperationUnavailable::Geometry)
            );
        }
    }
}
