// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C compiler variable-word layouts, separate from static values.

use crate::backslash::{
    NativeSourceUnavailable, native_source_escape_channel_in, native_source_literal_bytes,
};
use crate::native_string::NativeStringProtocol;
use tcl_dialect::TclVersion;
use tcl_lexer::{
    ExecutableInput, ExecutablePart, ExecutablePartArena, ExecutablePartsUnavailable, NativeWord,
    NativeWordError, Span, SubstFlags, WordKind,
};

/// The original `TclPushVarName` operand layout. This grants no local slot,
/// physical cell, selected handler or conversion-effect proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeVariableWordOperand {
    /// One original `SIMPLE_WORD/TEXT`, with a counted literal name and index.
    Literal {
        /// Source-channel-selected root/scalar value bytes, without name lookup.
        name: Vec<u8>,
        /// Original root/scalar source extent, before channel translation.
        name_span: Span,
        /// Literal element bytes. `Some([])` is an empty index, not a scalar.
        index: Option<Vec<u8>>,
    },
    /// Original first TEXT contains `(` and last TEXT ends in `)`.
    CompoundArray {
        /// Source-channel-selected literal root bytes, without name lookup.
        name: Vec<u8>,
        /// Original root extent within the first TEXT component.
        name_span: Span,
        /// Original index component arena, retaining the same image/channel.
        index: ExecutablePartArena,
    },
    /// Compute the complete original word; a known static value does not turn
    /// a BS token or substituted root into a compiler-local literal operand.
    DynamicWord,
}

/// Unavailable original compiler operand ownership, not a guest syntax error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVariableWordUnavailable {
    /// Original expansion has no one-word variable operand layout.
    Expanded,
    /// Original source/channel/string recipe does not retain matching geometry.
    SourceGeometry,
    /// The shared grouped-word owner declined its original content extent.
    Word(NativeWordError),
    /// The independently selected source decoder declined a BS token extent.
    Source(NativeSourceUnavailable),
    /// The shared executable arena could not retain original index regions.
    Arena(ExecutablePartsUnavailable),
}

/// Select the original C compiler's variable operand shape. `version` selects
/// compiler rules; `protocol` independently selects source BS token extents.
/// Names remain counted bytes. LVT eligibility and native cell lookup belong
/// to their independent purpose owners and cannot be inferred from this plan.
///
/// # Errors
/// Returns missing original geometry/decoder capability. A legitimate dynamic
/// word is returned explicitly rather than being confused with unavailable data.
pub fn native_variable_word(
    word: &NativeWord,
    version: TclVersion,
    protocol: NativeStringProtocol,
) -> Result<NativeVariableWordOperand, NativeVariableWordUnavailable> {
    if word.group().expand {
        return Err(NativeVariableWordUnavailable::Expanded);
    }
    if word.config().escapes != protocol.escape_syntax() {
        return Err(NativeVariableWordUnavailable::Source(
            NativeSourceUnavailable::GrammarProtocolMismatch,
        ));
    }
    if version == TclVersion::V8_4 && word.group().kind == WordKind::Braced {
        return Ok(NativeVariableWordOperand::DynamicWord);
    }
    let content = word
        .content_span()
        .map_err(NativeVariableWordUnavailable::Word)?;
    let components = original_components(word, content, protocol)?;
    if let [component] = components.as_slice()
        && component.text
    {
        return literal_operand(word, component.span, protocol);
    }
    let Some(first) = components.first().filter(|component| component.text) else {
        return Ok(NativeVariableWordOperand::DynamicWord);
    };
    let Some(last) = components.last().filter(|component| component.text) else {
        return Ok(NativeVariableWordOperand::DynamicWord);
    };
    let prefix = original(word, first.span)?;
    let Some(open) = prefix.iter().position(|byte| *byte == b'(') else {
        return Ok(NativeVariableWordOperand::DynamicWord);
    };
    if original(word, last.span)?.last() != Some(&b')') {
        return Ok(NativeVariableWordOperand::DynamicWord);
    }
    let name_end = first
        .span
        .start()
        .checked_add(
            u32::try_from(open).map_err(|_| NativeVariableWordUnavailable::SourceGeometry)?,
        )
        .ok_or(NativeVariableWordUnavailable::SourceGeometry)?;
    let name_span = Span::new(first.span.start(), name_end);
    let index_span = Span::new(name_end + 1, last.span.end() - 1);
    let index = if word.group().kind == WordKind::Braced {
        braced_index(word, index_span, &components)?
    } else {
        ExecutablePartArena::decompose(
            word.image().clone(),
            index_span,
            SubstFlags::default(),
            word.config(),
        )
        .map_err(NativeVariableWordUnavailable::Arena)?
    };
    Ok(NativeVariableWordOperand::CompoundArray {
        name: native_source_literal_bytes(
            original(word, name_span)?,
            word.image().channel(),
            protocol,
        )
        .map_err(NativeVariableWordUnavailable::Source)?
        .into_owned(),
        name_span,
        index,
    })
}

#[derive(Clone, Copy)]
struct Component {
    text: bool,
    span: Span,
}

fn original(word: &NativeWord, span: Span) -> Result<&[u8], NativeVariableWordUnavailable> {
    word.executable_parts()
        .bytes(span)
        .ok_or(NativeVariableWordUnavailable::SourceGeometry)
}

fn literal_operand(
    word: &NativeWord,
    span: Span,
    protocol: NativeStringProtocol,
) -> Result<NativeVariableWordOperand, NativeVariableWordUnavailable> {
    let bytes = original(word, span)?;
    let (name, index) = crate::naming::split_element_ref_bytes(bytes)
        .map_or((bytes, None), |(name, index)| (name, Some(index)));
    let name_span = Span::new(
        span.start(),
        span.start()
            + u32::try_from(name.len())
                .map_err(|_| NativeVariableWordUnavailable::SourceGeometry)?,
    );
    let channel = word.image().channel();
    Ok(NativeVariableWordOperand::Literal {
        name: native_source_literal_bytes(name, channel, protocol)
            .map_err(NativeVariableWordUnavailable::Source)?
            .into_owned(),
        name_span,
        index: index
            .map(|index| {
                native_source_literal_bytes(index, channel, protocol)
                    .map(std::borrow::Cow::into_owned)
            })
            .transpose()
            .map_err(NativeVariableWordUnavailable::Source)?,
    })
}

fn original_components(
    word: &NativeWord,
    content: Span,
    protocol: NativeStringProtocol,
) -> Result<Vec<Component>, NativeVariableWordUnavailable> {
    let mut components = Vec::new();
    if word.group().kind == WordKind::Braced {
        append_text_components(word, content, true, protocol, &mut components)?;
    } else {
        let arena = word.executable_parts();
        for part in arena.list(arena.root()) {
            if matches!(part.part, ExecutablePart::Text(_)) {
                append_text_components(word, part.span, false, protocol, &mut components)?;
            } else {
                components.push(Component {
                    text: false,
                    span: part.span,
                });
            }
        }
    }
    if components.is_empty() && content.is_empty() {
        components.push(Component {
            text: true,
            span: content,
        });
    }
    Ok(components)
}

fn append_text_components(
    word: &NativeWord,
    span: Span,
    braced: bool,
    protocol: NativeStringProtocol,
    components: &mut Vec<Component>,
) -> Result<(), NativeVariableWordUnavailable> {
    let bytes = original(word, span)?;
    let mut at = 0;
    let mut start = 0;
    while at < bytes.len() {
        if bytes[at] != b'\\' {
            at += 1;
            continue;
        }
        let end = if braced {
            if !word.config().brace_backslash_newline.folds() {
                at += 2;
                continue;
            }
            let Some(end) =
                tcl_lexer::source_backslash_continuation_end(bytes, at, word.image().channel())
            else {
                at += 2;
                continue;
            };
            end
        } else {
            native_source_escape_channel_in(
                bytes,
                at,
                word.image().channel(),
                word.config().escapes,
                protocol,
            )
            .map_err(NativeVariableWordUnavailable::Source)?
            .end
        };
        if start < at {
            components.push(Component {
                text: true,
                span: subspan(span, start, at)?,
            });
        }
        components.push(Component {
            text: false,
            span: subspan(span, at, end)?,
        });
        start = end;
        at = end;
    }
    if start < bytes.len() || bytes.is_empty() {
        components.push(Component {
            text: true,
            span: subspan(span, start, bytes.len())?,
        });
    }
    Ok(())
}

fn subspan(span: Span, start: usize, end: usize) -> Result<Span, NativeVariableWordUnavailable> {
    let start = u32::try_from(start).map_err(|_| NativeVariableWordUnavailable::SourceGeometry)?;
    let end = u32::try_from(end).map_err(|_| NativeVariableWordUnavailable::SourceGeometry)?;
    if start > end || end > span.len() {
        return Err(NativeVariableWordUnavailable::SourceGeometry);
    }
    Ok(Span::new(span.start() + start, span.start() + end))
}

fn braced_index(
    word: &NativeWord,
    index: Span,
    components: &[Component],
) -> Result<ExecutablePartArena, NativeVariableWordUnavailable> {
    let flags = SubstFlags {
        vars: false,
        cmds: false,
        ..SubstFlags::default()
    };
    let inputs = components
        .iter()
        .filter_map(|component| {
            let start = component.span.start().max(index.start());
            let end = component.span.end().min(index.end());
            (start < end).then(|| {
                if component.text {
                    ExecutableInput::Literal(Span::new(start, end))
                } else {
                    ExecutableInput::Substitute {
                        span: Span::new(start, end),
                        flags,
                    }
                }
            })
        })
        .collect::<Vec<_>>();
    ExecutablePartArena::from_inputs(word.image().clone(), &inputs, word.config())
        .map_err(NativeVariableWordUnavailable::Arena)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceChannel, SourceImage, native_script_words_in};

    fn capture(source: &[u8], channel: SourceChannel, version: TclVersion) -> NativeWord {
        let image = SourceImage::from_bytes(source, channel);
        let config = LexerConfig::from_grammar(
            tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string()))
                .unwrap()
                .grammar,
        );
        let end = u32::try_from(image.len()).unwrap();
        native_script_words_in(image, Span::new(0, end), config)
            .unwrap()
            .commands
            .remove(0)
            .words
            .remove(1)
    }

    #[test]
    fn literal_layout_retains_counted_names_and_the_c84_braced_exclusion() {
        for version in TclVersion::ALL {
            let word = capture(b"set {a\xff($name)} V", SourceChannel::NativeValue, version);
            let operand =
                native_variable_word(&word, version, NativeStringProtocol::C(version)).unwrap();
            if version == TclVersion::V8_4 {
                assert_eq!(operand, NativeVariableWordOperand::DynamicWord);
            } else {
                let NativeVariableWordOperand::Literal {
                    name,
                    name_span,
                    index,
                } = operand
                else {
                    panic!("one original TEXT");
                };
                assert_eq!(name, b"a\xff");
                assert_eq!(
                    word.executable_parts().bytes(name_span),
                    Some(b"a\xff".as_slice())
                );
                assert_eq!(index, Some(b"$name".to_vec()));
            }
            let empty = capture(b"set a() V", SourceChannel::NativeValue, version);
            assert!(
                matches!(native_variable_word(&empty, version, NativeStringProtocol::C(version)).unwrap(),
                NativeVariableWordOperand::Literal { index: Some(index), .. } if index.is_empty())
            );
        }
    }

    #[test]
    fn compound_layout_uses_original_text_edges_instead_of_static_decoded_values() {
        for version in TclVersion::ALL {
            for source in [
                b"set a\\x41(k) V".as_slice(),
                b"set a\\($i) V",
                b"set a($i\\) V",
            ] {
                let word = capture(source, SourceChannel::NativeValue, version);
                assert_eq!(
                    native_variable_word(&word, version, NativeStringProtocol::C(version)).unwrap(),
                    NativeVariableWordOperand::DynamicWord,
                    "{source:?}"
                );
            }
            let word = capture(b"set a\xff($i\\n) V", SourceChannel::NativeValue, version);
            let NativeVariableWordOperand::CompoundArray {
                name,
                name_span,
                index,
            } = native_variable_word(&word, version, NativeStringProtocol::C(version)).unwrap()
            else {
                panic!("original first/last TEXT array layout");
            };
            assert_eq!(name, b"a\xff");
            assert_eq!(index.image(), word.image());
            assert_eq!(index.bytes(name_span), Some(b"a\xff".as_slice()));
            assert!(
                index
                    .all_parts()
                    .any(|part| matches!(part.part, ExecutablePart::Variable { .. }))
            );
        }
    }

    #[test]
    fn braced_continuation_index_never_reinterprets_literal_substitutions() {
        let version = TclVersion::V9_1;
        let source = b"set {a($notRead\\\r\n\t [notRun])} V";
        for (channel, compound) in [
            (SourceChannel::Document, true),
            (SourceChannel::NativeValue, false),
        ] {
            let word = capture(source, channel, version);
            let operand =
                native_variable_word(&word, version, NativeStringProtocol::C(version)).unwrap();
            if compound {
                let NativeVariableWordOperand::CompoundArray { name, index, .. } = operand else {
                    panic!("document continuation BS token");
                };
                assert_eq!(name, b"a");
                assert_eq!(index.image(), word.image());
                assert!(
                    index
                        .all_parts()
                        .all(|part| matches!(part.part, ExecutablePart::Text(_)))
                );
                let value = index
                    .list(index.root())
                    .iter()
                    .flat_map(|part| {
                        crate::backslash::native_arena_text(
                            &index,
                            part,
                            word.config().escapes,
                            NativeStringProtocol::C(version),
                        )
                        .unwrap()
                        .into_owned()
                    })
                    .collect::<Vec<_>>();
                assert_eq!(value, b"$notRead [notRun]");
            } else {
                assert!(
                    matches!(operand, NativeVariableWordOperand::Literal { index: Some(index), .. }
                    if index == b"$notRead\\\r\n\t [notRun]")
                );
            }
        }
    }
}
