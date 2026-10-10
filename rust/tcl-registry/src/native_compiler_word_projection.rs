// SPDX-License-Identifier: AGPL-3.0-or-later
//! C parser projection of literal argument expansion over retained source words.
//!
//! A projected member keeps its original word and exact source value span.
//! This provides compiler token geometry only, without runtime object authority.

use crate::native_compilation::NativeCompilationWordShape;
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;
use tcl_lexer::{ExecutablePart, ExecutableText, Span, WordKind};

/// Operand in the parser's compiler-word vector.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeCompilerWordOperand {
    /// Evaluate one unchanged original word.
    Original(usize),
    /// The C parser creates a `SIMPLE_WORD` from this literal list member.
    LiteralExpansion {
        /// Original expanded word in the unchanged vector.
        original_word: usize,
        /// Literal member bytes in the original source image.
        value_span: Span,
        /// Native source-channel value, without runtime List construction.
        value: Vec<u8>,
    },
}

/// One native compiler token, retaining its original source operand.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeProjectedCompilerWord {
    /// Original word or exact literal expansion member.
    pub operand: NativeCompilerWordOperand,
    /// Shape supplied to the native compiler after parser expansion.
    pub shape: NativeCompilationWordShape,
    /// Compile-known value; absent means a genuinely substituted word.
    pub literal: Option<Vec<u8>>,
}

/// The parser cannot establish the supplied original compiler geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompilerProjectionUnavailable {
    /// The retained original source cannot supply a literal content span.
    SourceGeometry,
    /// A source-channel conversion changes the literal list's byte geometry.
    SourceChannel,
}

/// Apply C `TclParseCommand`'s pure TEXT expansion rule.
///
/// Static expansion is performed only when every original parser component is
/// TEXT and every list element is a literal substring. Backslash-generated
/// members, malformed lists and substituted expansions retain `EXPAND_WORD`;
/// their compiler's expanded-word policy is a separate selection.
///
/// # Errors
/// Returns unavailable geometry instead of reparsing a generated source string.
pub fn project_native_compiler_words(
    words: &NativeCompilerWords<'_>,
    version: TclVersion,
) -> Result<Vec<NativeProjectedCompilerWord>, NativeCompilerProjectionUnavailable> {
    let mut projected = Vec::new();
    for (index, original) in words.original_words().iter().enumerate() {
        let unchanged = || NativeProjectedCompilerWord {
            operand: NativeCompilerWordOperand::Original(index),
            shape: words.shapes()[index],
            literal: words.literal(index).map(<[u8]>::to_vec),
        };
        if version < TclVersion::V8_5 || !original.group().expand {
            projected.push(unchanged());
            continue;
        }
        let arena = original.executable_parts();
        if arena.list(arena.root()).iter().any(|component| {
            !matches!(
                component.part,
                ExecutablePart::Text(ExecutableText::Original)
            )
        }) {
            projected.push(unchanged());
            continue;
        }
        let content = original
            .content_span()
            .map_err(|_| NativeCompilerProjectionUnavailable::SourceGeometry)?;
        let raw = &original.image().bytes()[content.as_range()];
        if original.group().kind == WordKind::Braced
            && tcl_syntax::backslash::collapse_brace_continuations_for(
                raw,
                original.config().brace_backslash_newline,
            )
            .as_ref()
                != raw
        {
            projected.push(unchanged());
            continue;
        }
        if words.literal(index)
            != Some(
                tcl_syntax::backslash::native_source_literal_bytes(
                    raw,
                    original.image().channel(),
                    words.source_protocol(),
                )
                .map_err(|_| NativeCompilerProjectionUnavailable::SourceChannel)?
                .as_ref(),
            )
        {
            return Err(NativeCompilerProjectionUnavailable::SourceChannel);
        }
        let mut members = Vec::new();
        let mut cursor = 0;
        let mut literal = true;
        loop {
            match tcl_syntax::list::find_element_bytes(raw, cursor) {
                Ok(Some(element)) if element.literal => {
                    let start = u32::try_from(element.value.start)
                        .ok()
                        .and_then(|offset| content.start().checked_add(offset))
                        .ok_or(NativeCompilerProjectionUnavailable::SourceGeometry)?;
                    let end = u32::try_from(element.value.end)
                        .ok()
                        .and_then(|offset| content.start().checked_add(offset))
                        .ok_or(NativeCompilerProjectionUnavailable::SourceGeometry)?;
                    let value = tcl_syntax::backslash::native_source_literal_bytes(
                        &raw[element.value.clone()],
                        original.image().channel(),
                        words.source_protocol(),
                    )
                    .map_err(|_| NativeCompilerProjectionUnavailable::SourceChannel)?
                    .into_owned();
                    members.push(NativeProjectedCompilerWord {
                        operand: NativeCompilerWordOperand::LiteralExpansion {
                            original_word: index,
                            value_span: Span::new(start, end),
                            value: value.clone(),
                        },
                        shape: NativeCompilationWordShape::Literal,
                        literal: Some(value),
                    });
                    cursor = element.next;
                }
                Ok(None) => break,
                _ => {
                    literal = false;
                    break;
                }
            }
        }
        if literal {
            projected.extend(members);
        } else {
            projected.push(unchanged());
        }
    }
    Ok(projected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    #[test]
    fn parser_expansion_retains_exact_member_source_and_native_channel() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let source = b"variable {*}{v {a b} {} \xff\0}";
        let image = SourceImage::native(source.as_slice());
        let original = native_script_words_in(
            image,
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap()
        .commands
        .remove(0)
        .words;
        let words =
            NativeCompilerWords::capture(&original, NativeStringProtocol::C(TclVersion::V9_1))
                .unwrap();
        let projected = project_native_compiler_words(&words, TclVersion::V9_1).unwrap();
        assert_eq!(projected.len(), 5);
        assert_eq!(projected[0].operand, NativeCompilerWordOperand::Original(0));
        for (word, expected) in projected[1..]
            .iter()
            .zip([b"v".as_slice(), b"a b", b"", b"\xff\0"])
        {
            let NativeCompilerWordOperand::LiteralExpansion {
                original_word,
                value_span,
                value,
            } = &word.operand
            else {
                panic!("native pure TEXT expansion creates SIMPLE_WORD members");
            };
            assert_eq!(*original_word, 1);
            assert_eq!(&source[value_span.as_range()], expected);
            assert_eq!(value, expected);
            assert_eq!(word.literal.as_deref(), Some(expected));
            assert_eq!(word.shape, NativeCompilationWordShape::Literal);
        }
    }

    #[test]
    fn document_expansion_preserves_source_spans_and_native_produced_units() {
        // Native proof: naming.source.document-native-utf-ingress
        // docs/design/analysis/name-resolution-proofs/document-native-utf-ingress.md
        let source = "variable {*}{A\0😀}";
        let observations =
            include_str!("../../tcl-syntax/tests/data/native_source_ingress/observations.tsv");
        for version in TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= TclVersion::V8_5)
        {
            let profile = tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap();
            let config = LexerConfig::from_grammar(profile.grammar);
            for channel in [
                tcl_lexer::SourceChannel::Document,
                tcl_lexer::SourceChannel::NativeValue,
            ] {
                let image = SourceImage::from_bytes(source.as_bytes(), channel);
                let original = native_script_words_in(
                    image,
                    Span::new(0, u32::try_from(source.len()).unwrap()),
                    config,
                )
                .unwrap()
                .commands
                .remove(0)
                .words;
                let words =
                    NativeCompilerWords::capture(&original, NativeStringProtocol::C(version))
                        .unwrap();
                let projected = project_native_compiler_words(&words, version).unwrap();
                let NativeCompilerWordOperand::LiteralExpansion {
                    value_span, value, ..
                } = &projected[1].operand
                else {
                    panic!("the exact original static list member must retain its source extent");
                };
                assert_eq!(
                    &source.as_bytes()[value_span.as_range()],
                    "A\0😀".as_bytes()
                );
                let expected = if channel == tcl_lexer::SourceChannel::NativeValue {
                    "A\0😀".as_bytes().to_vec()
                } else {
                    let observed = observations
                        .lines()
                        .find_map(|row| {
                            let fields = row.split('\t').collect::<Vec<_>>();
                            (fields[0] == version.patchlevel()
                                && fields[1] == "character-channel-source"
                                && fields[2] == "plain")
                                .then_some(fields[3])
                        })
                        .expect("retained native input value");
                    let bytes = observed
                        .as_bytes()
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|pair| {
                            u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()
                        })
                        .collect::<Vec<_>>();
                    bytes[..bytes.iter().position(|byte| *byte == b'\n').unwrap()].to_vec()
                };
                assert_eq!(value, &expected, "{version:?} {channel:?}");
                assert_eq!(projected[1].literal.as_deref(), Some(expected.as_slice()));
            }
        }
    }

    #[test]
    fn generated_or_invalid_list_members_keep_original_expansion_obligations() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        for source in [
            b"variable {*}{v\\x31 2}".as_slice(),
            b"variable {*}$members",
            b"variable {*}{v {a}",
        ] {
            let image = SourceImage::native(source);
            let parsed = native_script_words_in(
                image,
                Span::new(0, u32::try_from(source.len()).unwrap()),
                LexerConfig::from_grammar(profile.grammar),
            );
            if let Ok(mut parsed) = parsed {
                if parsed.commands.is_empty() {
                    assert!(parsed.fatal_tail.is_some(), "{source:?}");
                    assert_eq!(source, b"variable {*}{v {a}");
                    continue;
                }
                assert!(parsed.fatal_tail.is_none(), "{source:?}");
                let original = parsed.commands.remove(0).words;
                let words = NativeCompilerWords::capture(
                    &original,
                    NativeStringProtocol::C(TclVersion::V9_1),
                )
                .unwrap();
                let projected = project_native_compiler_words(&words, TclVersion::V9_1).unwrap();
                assert_eq!(projected.len(), original.len());
                assert!(
                    projected
                        .iter()
                        .all(|word| matches!(word.operand, NativeCompilerWordOperand::Original(_)))
                );
                assert!(
                    projected
                        .iter()
                        .any(|word| word.shape == NativeCompilationWordShape::Expanded)
                );
            }
        }
    }
}
