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

//! Ordinary UPVAR retains a level object, followed by ordered caller-name operands.

use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    NativeCompilationSelection, NativeCompilationWordShape, original_upvar_level_width,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;

/// One original otherVar and a compiler-declared scalar destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUpvarBinding {
    /// Original operand, evaluated immediately before this UPVAR.
    pub other: NativeCompilerWordOperand,
    /// Counted native local declaration, never a namespace/frame selector.
    pub local: Vec<u8>,
}

/// Ordinary frame-level UPVAR, distinct from namespace NSUPVAR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUpvarInstruction {
    /// Explicit original level operand. Absence pushes registered literal `1`.
    pub level: Option<NativeCompilerWordOperand>,
    /// Native ordered local declarations and alias operations.
    pub bindings: Vec<NativeUpvarBinding>,
}

/// Missing original geometry or actual compiler-stack evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeUpvarUnavailable {
    /// Original operands do not have the native pair layout.
    Geometry,
    /// The compiler's frame conversion depends on unavailable stack context.
    Frame,
    /// The known leading object cannot be supplied to the checked frame grammar.
    LevelBytes,
    /// Original source or constant expansion ownership is unavailable.
    Source,
}

/// Project an independently admitted ordinary C UPVAR compiler.
/// This supplies neither runtime frame selection nor variable ownership.
///
/// # Errors
/// Rejects unavailable original geometry or compiler invocation context.
pub fn compile_native_upvar(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<NativeUpvarInstruction, NativeUpvarUnavailable> {
    use NativeUpvarUnavailable as Unavailable;
    if operand_from == 0
        || version < TclVersion::V8_5
        || context.mode != NativeCompilationMode::BytecodeObject
        || context.frame != NativeCompilationFrame::ProcedureCode
    {
        return Err(Unavailable::Frame);
    }
    let projected =
        project_native_compiler_words(words, version).map_err(|_| Unavailable::Source)?;
    let operands = projected.get(operand_from..).ok_or(Unavailable::Geometry)?;
    if operands.len() < 2
        || operands
            .iter()
            .any(|word| word.shape == NativeCompilationWordShape::Expanded)
    {
        return Err(Unavailable::Geometry);
    }
    let first = operands[0]
        .literal
        .as_deref()
        .ok_or(Unavailable::Geometry)?;
    let first = std::str::from_utf8(first).map_err(|_| Unavailable::LevelBytes)?;
    let width =
        original_upvar_level_width(first, operands.len(), version).map_err(|selection| {
            if selection == NativeCompilationSelection::Unknown {
                Unavailable::Frame
            } else {
                Unavailable::Geometry
            }
        })?;
    let mut bindings = Vec::new();
    for pair in operands[width..].as_chunks::<2>().0 {
        if !matches!(
            pair[1].shape,
            NativeCompilationWordShape::Literal
                | NativeCompilationWordShape::QuotedLiteral
                | NativeCompilationWordShape::BracedLiteral
        ) {
            return Err(Unavailable::Geometry);
        }
        let local = pair[1].literal.as_deref().ok_or(Unavailable::Geometry)?;
        let selected = tcl_syntax::naming::NativeCompiledVariableRecipe::C(version)
            .scalar_name(local)
            .ok_or(Unavailable::Geometry)?;
        if !selected.scalar || selected.declaration.is_none() {
            return Err(Unavailable::Geometry);
        }
        bindings.push(NativeUpvarBinding {
            other: pair[0].operand.clone(),
            local: local.to_vec(),
        });
    }
    if bindings.is_empty() || (operands.len() - width) % 2 != 0 {
        return Err(Unavailable::Geometry);
    }
    Ok(NativeUpvarInstruction {
        level: (width == 1).then(|| operands[0].operand.clone()),
        bindings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("../tests/data/native_upvar_info_exists/cases.rs");
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    fn context() -> NativeCompilationContext {
        NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            loop_depth: 0,
            catch_depth: Some(0),
        }
    }
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
    fn original_upvar_keeps_ordered_targets_and_opaque_local_names() {
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let originals = capture(b"upvar 0 [first] alias [second] \xff", version);
            let words =
                NativeCompilerWords::capture(&originals, NativeStringProtocol::C(version)).unwrap();
            let recipe = compile_native_upvar(&words, 1, version, context()).unwrap();
            assert_eq!(recipe.level, Some(NativeCompilerWordOperand::Original(1)));
            assert_eq!(recipe.bindings.len(), 2);
            assert_eq!(
                recipe.bindings[0].other,
                NativeCompilerWordOperand::Original(2)
            );
            assert_eq!(
                recipe.bindings[1].other,
                NativeCompilerWordOperand::Original(4)
            );
            assert_eq!(recipe.bindings[0].local, b"alias");
            assert_eq!(recipe.bindings[1].local, b"\xff");
        }
    }
    #[test]
    fn upvar_original_recipe_requires_procedure_scope_and_native_simple_locals() {
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            for source in [
                b"upvar 0 x alias(k)".as_slice(),
                b"upvar 0 x ::alias",
                b"upvar $level x alias",
                b"upvar 0 x a\\154ias",
                b"upvar {*}$unknown",
            ] {
                let originals = capture(source, version);
                let words =
                    NativeCompilerWords::capture(&originals, NativeStringProtocol::C(version))
                        .unwrap();
                assert!(
                    compile_native_upvar(&words, 1, version, context()).is_err(),
                    "{version:?}/{source:?}"
                );
            }
            let originals = capture(b"upvar 0 x alias", version);
            let words =
                NativeCompilerWords::capture(&originals, NativeStringProtocol::C(version)).unwrap();
            for frame in [
                NativeCompilationFrame::ScriptCode,
                NativeCompilationFrame::Unknown,
            ] {
                assert_eq!(
                    compile_native_upvar(
                        &words,
                        1,
                        version,
                        NativeCompilationContext { frame, ..context() }
                    ),
                    Err(NativeUpvarUnavailable::Frame)
                );
            }
            assert_eq!(
                compile_native_upvar(
                    &words,
                    1,
                    version,
                    NativeCompilationContext {
                        mode: NativeCompilationMode::Direct,
                        ..context()
                    }
                ),
                Err(NativeUpvarUnavailable::Frame)
            );
        }
    }
    #[test]
    fn original_upvar_selection_matches_native_compiler_opcode_presence() {
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
        let mut checked = 0;
        for (version, table) in tables {
            let dialect = crate::InvocationDialect::for_version(version);
            let spec = registry
                .native_compilation_for_registration("upvar", dialect)
                .unwrap();
            assert_eq!(
                spec.compiler_hook_presence(dialect),
                Some(version >= TclVersion::V8_5)
            );
            for row in table.lines().filter(|row| row.starts_with("R|")) {
                let fields: Vec<_> = row.split('|').collect();
                let case = fields[1].parse::<usize>().unwrap();
                if !CASES[case].starts_with("upvar ") {
                    continue;
                }
                let plan = capture_plan(CASES[case].as_bytes(), version);
                if let Some(tail) = plan.fatal_tail {
                    assert_eq!(version, TclVersion::V8_4);
                    assert_eq!(case, 10);
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
                let selected = spec.select_native_words(&words, 1, Some(dialect), context());
                if version == TclVersion::V8_4 {
                    assert_eq!(selected, NativeCompilationSelection::Generic);
                }
                assert_eq!(
                    matches!(selected, NativeCompilationSelection::Inline { .. }),
                    fields[7].contains("upvar:"),
                    "{version:?}/{case}"
                );
                if matches!(selected, NativeCompilationSelection::Inline { .. }) {
                    let recipe = compile_native_upvar(&words, 1, version, context()).unwrap();
                    assert_eq!(
                        recipe.bindings.len(),
                        fields[7].matches("upvar:").count(),
                        "{version:?}/{case}"
                    );
                }
                checked += 1;
            }
        }
        assert_eq!(checked, 80);
    }
}
