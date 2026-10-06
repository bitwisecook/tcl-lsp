// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native unset compiler validation and sequential variable operands.

use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, NativeProjectedCompilerWord, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;
use tcl_lexer::word_parts::{ExecutablePart, ExecutableText};
use tcl_syntax::native_variable_words::{NativeVariableWordOperand, native_variable_word};

/// Native validation completes before any variable operand is compiled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUnsetInstruction {
    /// The native opcode flag requests a message when the selected cell is absent.
    pub complain: bool,
    /// Each original variable operand executes immediately before its unset.
    pub variables: Vec<NativeUnsetVariable>,
}

/// One retained parser operand and its independent compiler variable geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUnsetVariable {
    /// Original word or a parser-projected constant expansion member.
    pub operand: NativeCompilerWordOperand,
    /// The compiler's root/index geometry; this grants no runtime cache primary.
    pub receiver: NativeUnsetReceiver,
}

/// Variable geometry after the native parser's constant expansion projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeUnsetReceiver {
    /// The existing original grouped-word owner retains root and index regions.
    Original(NativeVariableWordOperand),
    /// A native parser-projected TEXT member supplies counted root/index bytes.
    ExpandedLiteral {
        /// Exact source-protocol-selected root bytes.
        name: Vec<u8>,
        /// Counted element bytes, when the original parser member names an array.
        index: Option<Vec<u8>>,
    },
}

/// Missing original source evidence, distinct from a native compiler decline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeUnsetUnavailable {
    /// The complete original vector does not contain the selected operands.
    Geometry,
    /// Constant parser expansion ownership is unavailable.
    Projection(crate::native_compiler_word_projection::NativeCompilerProjectionUnavailable),
    /// Original variable-word source or escape ownership is unavailable.
    Variable(tcl_syntax::native_variable_words::NativeVariableWordUnavailable),
}

/// Validate the native flags/known-word pass and retain sequential receivers.
/// `None` is a genuine decline before literal/LVT preparation, including an
/// unresolved expansion. Older releases have no unset compiler registration.
///
/// # Errors
/// Rejects unavailable original parser/value/variable geometry.
pub fn compile_native_unset(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Result<Option<NativeUnsetInstruction>, NativeUnsetUnavailable> {
    if operand_from == 0 || operand_from > words.original_words().len() {
        return Err(NativeUnsetUnavailable::Geometry);
    }
    if version < TclVersion::V8_6 {
        return Ok(None);
    }
    let projected = project_native_compiler_words(words, version)
        .map_err(NativeUnsetUnavailable::Projection)?;
    if projected
        .iter()
        .any(|word| word.shape == crate::native_compilation::NativeCompilationWordShape::Expanded)
    {
        return Ok(None);
    }
    let operands: Vec<_> = projected
        .into_iter()
        .filter(|word| match &word.operand {
            NativeCompilerWordOperand::Original(index) => *index >= operand_from,
            NativeCompilerWordOperand::LiteralExpansion { original_word, .. } => {
                *original_word >= operand_from
            }
        })
        .collect();
    let Some((complain, flags)) = validate_unset_operands(words, &operands) else {
        return Ok(None);
    };
    let variables = operands
        .into_iter()
        .skip(flags)
        .map(|word| {
            let receiver = match &word.operand {
                NativeCompilerWordOperand::Original(index) => NativeUnsetReceiver::Original(
                    native_variable_word(
                        &words.original_words()[*index],
                        version,
                        words.source_protocol(),
                    )
                    .map_err(NativeUnsetUnavailable::Variable)?,
                ),
                NativeCompilerWordOperand::LiteralExpansion { value, .. } => {
                    let (name, index) = tcl_syntax::naming::split_element_ref_bytes(value)
                        .map_or((value.as_slice(), None), |(name, index)| {
                            (name, Some(index))
                        });
                    NativeUnsetReceiver::ExpandedLiteral {
                        name: name.to_vec(),
                        index: index.map(<[u8]>::to_vec),
                    }
                }
            };
            Ok(NativeUnsetVariable {
                operand: word.operand,
                receiver,
            })
        })
        .collect::<Result<Vec<_>, NativeUnsetUnavailable>>()?;
    Ok(Some(NativeUnsetInstruction {
        complain,
        variables,
    }))
}

fn validate_unset_operands(
    words: &NativeCompilerWords<'_>,
    operands: &[NativeProjectedCompilerWord],
) -> Option<(bool, usize)> {
    let mut complain = true;
    let mut flags = 0;
    let mut variables = 0;
    for (position, word) in operands.iter().enumerate() {
        let Some(value) = word.literal.as_deref() else {
            if variables == 0 && (flags != 0 || nonoption_prefix(words, &word.operand)) {
                continue;
            }
            return None;
        };
        if variables == 0 {
            if position == 0 && value == b"-nocomplain" {
                complain = false;
                flags += 1;
            } else if position == usize::from(!complain) && value == b"--" {
                flags += 1;
            } else {
                variables += 1;
            }
        } else {
            variables += 1;
        }
    }
    Some((complain, flags))
}

fn nonoption_prefix(words: &NativeCompilerWords<'_>, operand: &NativeCompilerWordOperand) -> bool {
    let NativeCompilerWordOperand::Original(index) = operand else {
        return false;
    };
    let original = &words.original_words()[*index];
    let arena = original.executable_parts();
    let Some(first) = arena.list(arena.root()).first() else {
        return false;
    };
    if !matches!(&first.part, ExecutablePart::Text(ExecutableText::Original)) {
        return false;
    }
    arena
        .bytes(first.span)
        .and_then(|bytes| bytes.first())
        .is_some_and(|byte| !matches!(*byte, b'-' | b'\\'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    fn unhex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn original_unset_validation_matches_85_native_compiler_windows() {
        let mut compared = 0;
        for row in include_str!("../tests/data/native_unset_compilation/windows.tsv")
            .lines()
            .skip(1)
        {
            let columns: Vec<_> = row.split('\t').collect();
            if columns[0] == "jim0.84" {
                continue;
            }
            let profile = tcl_dialect::DialectProfile::find(&format!("tcl{}", columns[0])).unwrap();
            let version = crate::InvocationDialect::of_profile(profile)
                .tcl_version
                .unwrap();
            let source = unhex(columns[2]);
            let parsed = native_script_words_in(
                SourceImage::native(source.as_slice()),
                Span::new(0, source.len() as u32),
                LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            compared += 1;
            if parsed.commands.is_empty() {
                assert_eq!(version, TclVersion::V8_4);
                assert!(parsed.fatal_tail.is_some());
                assert_eq!(columns[3], "0");
                continue;
            }
            let words = NativeCompilerWords::capture(
                &parsed.commands[0].words,
                NativeStringProtocol::C(version),
            )
            .unwrap();
            let selected = compile_native_unset(&words, 1, version).unwrap();
            assert_eq!(
                selected.is_some(),
                columns[3] == "1",
                "{}:{}",
                columns[0],
                columns[1]
            );
            if let Some(recipe) = selected {
                assert_eq!(
                    recipe.variables.len(),
                    columns[4].parse::<usize>().unwrap(),
                    "{}:{}",
                    columns[0],
                    columns[1]
                );
                assert_eq!(
                    recipe.complain,
                    !matches!(columns[1], "quiet" | "quietflags")
                );
            }
        }
        assert_eq!(compared, 85);
    }
}
