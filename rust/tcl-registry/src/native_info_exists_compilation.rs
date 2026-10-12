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

//! Original info-exists variable geometry, independent of namespace/frame lookup.

use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;
use tcl_syntax::native_variable_words::{NativeVariableWordOperand, native_variable_word};

/// Actual parser variable layout of the admitted existence operand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeInfoExistsReceiver {
    /// Unchanged grouped source word retains root/index regions.
    Original(NativeVariableWordOperand),
    /// A constant parser expansion supplies the native TEXT root/index.
    ExpandedLiteral {
        /// Counted original literal root bytes.
        name: Vec<u8>,
        /// Literal array element, including a distinct empty index.
        index: Option<Vec<u8>>,
    },
}

/// One existence instruction; no values, cells or trace authority are donated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeInfoExistsInstruction {
    /// Exact original parser operand or constant expanded member.
    pub operand: NativeCompilerWordOperand,
    /// Shared native variable-word preparation.
    pub receiver: NativeInfoExistsReceiver,
}

/// An independently admitted compiler still needs its original source geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeInfoExistsUnavailable {
    /// Wrong operand count, unresolved expansion or missing original source.
    Geometry,
    /// This native release has no existence compiler.
    Version,
}

/// Retain the original C8.5+ existence root/index operands.
///
/// # Errors
/// Refuses absent original geometry or an unsupported release.
pub fn compile_native_info_exists(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Result<NativeInfoExistsInstruction, NativeInfoExistsUnavailable> {
    use NativeInfoExistsUnavailable as Unavailable;
    if version < TclVersion::V8_5 {
        return Err(Unavailable::Version);
    }
    if operand_from == 0 {
        return Err(Unavailable::Geometry);
    }
    let projected =
        project_native_compiler_words(words, version).map_err(|_| Unavailable::Geometry)?;
    let [word] = projected.get(operand_from..).ok_or(Unavailable::Geometry)? else {
        return Err(Unavailable::Geometry);
    };
    let receiver = match &word.operand {
        NativeCompilerWordOperand::Original(index) => NativeInfoExistsReceiver::Original(
            native_variable_word(
                &words.original_words()[*index],
                version,
                words.source_protocol(),
            )
            .map_err(|_| Unavailable::Geometry)?,
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
    Ok(NativeInfoExistsInstruction {
        operand: word.operand.clone(),
        receiver,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("../tests/data/native_upvar_info_exists/cases.rs");
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;
    fn capture_plan(source: &[u8], version: TclVersion) -> tcl_lexer::NativeScriptWordsPlan {
        let dialect = crate::InvocationDialect::for_version(version);
        native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap()
    }
    fn capture(source: &[u8], version: TclVersion) -> Vec<tcl_lexer::NativeWord> {
        capture_plan(source, version)
            .commands
            .into_iter()
            .next()
            .unwrap()
            .words
    }
    #[test]
    fn original_exists_retains_array_index_arena_and_static_expansion() {
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let original = capture(b"info exists \xff([index])", version);
            let words =
                NativeCompilerWords::capture(&original, NativeStringProtocol::C(version)).unwrap();
            let recipe = compile_native_info_exists(&words, 2, version).unwrap();
            assert_eq!(recipe.operand, NativeCompilerWordOperand::Original(2));
            let NativeInfoExistsReceiver::Original(NativeVariableWordOperand::CompoundArray {
                name,
                index,
                ..
            }) = recipe.receiver
            else {
                panic!("original compound array");
            };
            assert_eq!(name, b"\xff");
            assert_eq!(index.image(), original[2].image());
            let original = capture(b"info exists {*}{a(k)}", version);
            let words =
                NativeCompilerWords::capture(&original, NativeStringProtocol::C(version)).unwrap();
            let recipe = compile_native_info_exists(&words, 2, version).unwrap();
            assert_eq!(
                recipe.receiver,
                NativeInfoExistsReceiver::ExpandedLiteral {
                    name: b"a".to_vec(),
                    index: Some(b"k".to_vec())
                }
            );
        }
    }
    #[test]
    fn original_exists_declines_unknown_expansion_extra_operands_and_c84() {
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            for source in [
                b"info exists {*}$unknown".as_slice(),
                b"info exists x y",
                b"info exists",
            ] {
                let original = capture(source, version);
                let words =
                    NativeCompilerWords::capture(&original, NativeStringProtocol::C(version))
                        .unwrap();
                assert!(
                    compile_native_info_exists(&words, 2, version).is_err(),
                    "{version:?}/{source:?}"
                );
            }
        }
        let original = capture(b"info exists x", TclVersion::V8_4);
        let words =
            NativeCompilerWords::capture(&original, NativeStringProtocol::C(TclVersion::V8_4))
                .unwrap();
        assert_eq!(
            compile_native_info_exists(&words, 2, TclVersion::V8_4),
            Err(NativeInfoExistsUnavailable::Version)
        );
    }
    #[test]
    fn original_exists_selection_matches_60_native_compiler_opcode_windows() {
        use crate::native_compilation::{
            NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
            NativeCompilationSelection,
        };
        let tables = [
            (
                TclVersion::V8_4,
                include_str!("../tests/data/native_upvar_info_exists/8.4.20.txt"),
            ),
            (
                TclVersion::V8_5,
                include_str!("../tests/data/native_upvar_info_exists/8.5.19.txt"),
            ),
            (
                TclVersion::V8_6,
                include_str!("../tests/data/native_upvar_info_exists/8.6.18.txt"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../tests/data/native_upvar_info_exists/9.0.4.txt"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../tests/data/native_upvar_info_exists/9.1.0.txt"),
            ),
        ];
        let registry = crate::CommandRegistry::build_default();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            loop_depth: 0,
            catch_depth: Some(0),
        };
        let mut checked = 0;
        for (version, table) in tables {
            let dialect = crate::InvocationDialect::for_version(version);
            let spec = registry
                .native_compilation_for_registration(
                    if version == TclVersion::V8_4 {
                        "info"
                    } else {
                        "tcl::info::exists"
                    },
                    dialect,
                )
                .unwrap();
            assert_eq!(
                spec.compiler_hook_presence(dialect),
                Some(version >= TclVersion::V8_5)
            );
            for row in table.lines().filter(|row| row.starts_with("R|")) {
                let fields: Vec<_> = row.split('|').collect();
                let case = fields[1].parse::<usize>().unwrap();
                if !CASES[case].starts_with("info exists ") {
                    continue;
                }
                let plan = capture_plan(CASES[case].as_bytes(), version);
                if let Some(tail) = plan.fatal_tail {
                    assert_eq!(version, TclVersion::V8_4);
                    assert_eq!(case, 22);
                    assert!(plan.commands.is_empty());
                    assert_eq!(tail.command_start, 0);
                    assert_eq!(tail.cut.command, 0);
                    assert_eq!(tail.cut.message, "extra characters after close-brace");
                    assert_eq!(fields[2], "1");
                    assert_eq!(
                        fields[6],
                        "6578747261206368617261637465727320616674657220636c6f73652d6272616365"
                    );
                    assert!(fields[7].is_empty());
                    checked += 1;
                    continue;
                }
                let original = plan.commands.into_iter().next().unwrap().words;
                let words =
                    NativeCompilerWords::capture(&original, NativeStringProtocol::C(version))
                        .unwrap();
                let selected = spec.select_native_words(&words, 2, Some(dialect), context);
                if version == TclVersion::V8_4 {
                    assert_eq!(selected, NativeCompilationSelection::Generic);
                }
                assert_eq!(
                    matches!(selected, NativeCompilationSelection::Inline { .. }),
                    fields[7].contains("exist"),
                    "{version:?}/{case}"
                );
                if matches!(selected, NativeCompilationSelection::Inline { .. }) {
                    assert!(
                        compile_native_info_exists(&words, 2, version).is_ok(),
                        "{version:?}/{case}"
                    );
                }
                checked += 1;
            }
        }
        assert_eq!(checked, 60);
    }
}
