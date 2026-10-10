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

//! Original native namespace and frame introspection instruction geometry.

use crate::native_compilation::NativeCompilationWordShape as Shape;
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;

/// Actual independently retained compiler operation, without registration authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeIntrospectionKind {
    /// Entered frame's actual namespace object producer.
    NamespaceCurrent,
    /// Original command cache and imported command origin.
    NamespaceOrigin,
    /// Original scoped-list constructor.
    NamespaceCode,
    /// Original caller-variable frame introspection.
    InfoLevel,
    /// Original absolute literal command resolution and conditional List result.
    InfoCommands,
}

/// Selected original operands and physical introspection instruction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeIntrospectionInstruction {
    /// Independently selected compiler operation.
    pub kind: NativeIntrospectionKind,
    /// Chronological original operands; Current and numeric Level have none.
    pub operands: Vec<NativeCompilerWordOperand>,
}

/// Capture the actual compiler's original operand geometry. The caller retains
/// the registration, selected compiler, namespace and native interpreter proofs.
#[must_use]
pub fn compile_native_introspection(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    kind: NativeIntrospectionKind,
    version: TclVersion,
) -> Option<NativeIntrospectionInstruction> {
    if version < TclVersion::V8_6 || operand_from == 0 {
        return None;
    }
    let projected = project_native_compiler_words(words, version).ok()?;
    let arguments = projected.get(operand_from..)?;
    if arguments
        .iter()
        .any(|word| matches!(word.shape, Shape::Expanded | Shape::Opaque))
    {
        return None;
    }
    let valid = match kind {
        NativeIntrospectionKind::NamespaceCurrent => arguments.is_empty(),
        NativeIntrospectionKind::NamespaceOrigin => arguments.len() == 1,
        NativeIntrospectionKind::InfoLevel => arguments.len() <= 1,
        NativeIntrospectionKind::InfoCommands => {
            matches!(arguments, [word] if matches!(word.shape, Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral)
                && word.literal.as_deref().is_some_and(native_info_commands_literal_is_trivial))
        }
        NativeIntrospectionKind::NamespaceCode => {
            matches!(arguments, [word] if matches!(word.shape, Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral)
                && word.literal.as_ref().is_some_and(|bytes| !(bytes.len() > 20 && bytes.starts_with(b"::namespace inscope "))))
        }
    };
    valid.then(|| NativeIntrospectionInstruction {
        kind,
        operands: arguments.iter().map(|word| word.operand.clone()).collect(),
    })
}

/// Pure compile-known pattern restriction of the original C compiler. The
/// caller supplies its original source-channel value and selected registration.
#[must_use]
pub fn native_info_commands_literal_is_trivial(bytes: &[u8]) -> bool {
    bytes.starts_with(b"::")
        && !bytes
            .iter()
            .any(|byte| matches!(byte, b'*' | b'[' | b'?' | b'\\'))
}

/// The opcode's reached numeric extraction width, distinct from frame selectors.
#[must_use]
pub const fn native_info_level_getter(
    version: TclVersion,
) -> tcl_syntax::scalar_getter::NativeScalarGetterKind {
    use tcl_syntax::scalar_getter::NativeScalarGetterKind as Getter;
    if matches!(version, TclVersion::V9_1) {
        Getter::Wide
    } else {
        Getter::Int
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("../tests/data/native_introspection_compilation/cases.rs");
    #[test]
    fn original_introspection_recipes_match_seventy_five_native_compiler_windows() {
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
            for row in table.lines().take(15) {
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
                let kind = match case {
                    0..=1 => NativeIntrospectionKind::NamespaceCurrent,
                    2..=3 => NativeIntrospectionKind::NamespaceOrigin,
                    4..=7 => NativeIntrospectionKind::NamespaceCode,
                    _ => NativeIntrospectionKind::InfoLevel,
                };
                let observed = fields[9].split(',').any(|op| {
                    matches!(
                        op,
                        "currentNamespace" | "originCmd" | "infoLevelNumber" | "infoLevelArgs"
                    )
                });
                assert_eq!(
                    compile_native_introspection(&words, 2, kind, version).is_some(),
                    observed,
                    "{version:?}/{case}"
                );
                windows += 1;
            }
        }
        assert_eq!(windows, 75);
    }
}
