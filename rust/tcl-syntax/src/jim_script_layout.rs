// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native Jim Script WORD/LINE layout over the original full token roster.

use tcl_lexer::{JimScriptLine, JimScriptTokenKind, JimScriptTokens};

/// One prepared Script entry, without constructing any native object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JimScriptLayoutEntry {
    /// Construct this exact original token object from the retained roster.
    Token {
        /// Index into the unchanged full parser token roster.
        roster_index: usize,
    },
    /// Native token-combination count; negative selects expansion.
    Word(i32),
    /// Native command-argument count and original first-token line.
    Line {
        /// Native count, including the separate expansion accounting.
        argc: i32,
        /// Raw LF delta from the original signed Source line.
        line_delta: u32,
    },
}

/// Pure Script layout; filename, cache and object identities remain backend-owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JimScriptLayout {
    /// Native Script entries in their original construction order.
    pub entries: Vec<JimScriptLayoutEntry>,
    /// First parser-token line, including a leading separator or empty input.
    pub first_line_delta: u32,
    /// Original native completeness line, independent of the executable entries.
    pub completeness_line: JimScriptLine,
    /// Native missing marker; malformed scripts still retain their layout.
    pub missing: Option<u8>,
}

/// Unavailable original geometry or native counter representation; never guest syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JimScriptLayoutUnavailable {
    /// A token no longer refers to the retained counted source.
    SourceGeometry,
    /// The full parser roster lacks its native terminal sentinels.
    TokenRoster,
    /// A native signed token or argument counter cannot represent the layout.
    CounterRange,
}

/// Project native WORD/LINE construction from the selected full parser roster.
/// Expansion consumes its prefix and all but the last remaining token in that
/// pass; the last token is processed by the next native outer iteration.
///
/// # Errors
/// Returns unavailable original source geometry, token roster or native counters.
pub fn prepare_jim_script_layout(
    roster: &JimScriptTokens,
) -> Result<JimScriptLayout, JimScriptLayoutUnavailable> {
    validate_roster(roster)?;
    let mut entries = vec![JimScriptLayoutEntry::Line {
        argc: 0,
        line_delta: roster.first_line_delta(),
    }];
    let mut first = 0;
    let mut line = roster.first_line_delta();
    let mut argc = 0_i32;
    let mut index = 0;
    while index < roster.tokens.len() {
        while roster.tokens[index].kind == JimScriptTokenKind::Separator {
            index += 1;
        }
        let count = word_count(roster, index)?;
        if count == 0 {
            if argc != 0 {
                entries[first] = JimScriptLayoutEntry::Line {
                    argc,
                    line_delta: line,
                };
                argc = 0;
                first = entries.len();
                entries.push(JimScriptLayoutEntry::Line {
                    argc,
                    line_delta: line,
                });
            }
            index += 1;
            continue;
        }
        let mut remaining = count;
        if count != 1 {
            entries.push(JimScriptLayoutEntry::Word(count));
            if count < 0 {
                index += 1;
                remaining = -count - 1;
                argc = argc
                    .checked_sub(1)
                    .ok_or(JimScriptLayoutUnavailable::CounterRange)?;
            }
        }
        if argc == 0 {
            line = roster.tokens[index].line_delta;
        }
        argc = argc
            .checked_add(1)
            .ok_or(JimScriptLayoutUnavailable::CounterRange)?;
        for _ in 0..remaining {
            entries.push(JimScriptLayoutEntry::Token {
                roster_index: index,
            });
            index += 1;
        }
    }
    if argc == 0 {
        entries.pop();
    }
    Ok(JimScriptLayout {
        entries,
        first_line_delta: roster.first_line_delta(),
        completeness_line: roster.completeness_line,
        missing: roster.missing,
    })
}

fn word_count(roster: &JimScriptTokens, index: usize) -> Result<i32, JimScriptLayoutUnavailable> {
    let expanded = roster.expansion_prefix(index);
    let start = index + usize::from(expanded);
    let count = roster.tokens[start..]
        .iter()
        .take_while(|token| !token.kind.is_separator())
        .count();
    let count = i32::try_from(count).map_err(|_| JimScriptLayoutUnavailable::CounterRange)?;
    Ok(if expanded { -count } else { count })
}

fn validate_roster(roster: &JimScriptTokens) -> Result<(), JimScriptLayoutUnavailable> {
    if roster.tokens.len() < 2
        || roster
            .tokens
            .last()
            .is_none_or(|token| token.kind != JimScriptTokenKind::EndSource)
        || roster.tokens[roster.tokens.len() - 2].kind != JimScriptTokenKind::EndCommand
        || roster.tokens[..roster.tokens.len() - 1]
            .iter()
            .any(|token| token.kind == JimScriptTokenKind::EndSource)
    {
        return Err(JimScriptLayoutUnavailable::TokenRoster);
    }
    if roster.tokens.iter().any(|token| {
        roster.image.bytes().get(token.source.as_range()).is_none()
            || roster.image.bytes().get(token.value.as_range()).is_none()
            || token.value.start() < token.source.start()
            || token.value.end() > token.source.end()
    }) {
        return Err(JimScriptLayoutUnavailable::SourceGeometry);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::{Family, Release, grammar};
    use tcl_lexer::{LexerConfig, SourceImage, Span, jim_script_tokens};

    fn roster(source: &[u8]) -> JimScriptTokens {
        jim_script_tokens(
            &SourceImage::native(source),
            LexerConfig::from_grammar(grammar(Family::Jim, Release::JIM_0_84)),
        )
        .unwrap()
    }

    fn decode_hex(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    fn signature(layout: &JimScriptLayout, roster: &JimScriptTokens) -> String {
        layout
            .entries
            .iter()
            .map(|entry| match entry {
                JimScriptLayoutEntry::Line { argc, line_delta } => format!("L:{argc}:{line_delta}"),
                JimScriptLayoutEntry::Word(count) => format!("W:{count}"),
                JimScriptLayoutEntry::Token { roster_index } => {
                    let token = &roster.tokens[*roster_index];
                    let kind = match token.kind {
                        JimScriptTokenKind::String => 1,
                        JimScriptTokenKind::Escaped => 2,
                        JimScriptTokenKind::Variable => 3,
                        JimScriptTokenKind::IndexedVariable => 4,
                        JimScriptTokenKind::Command => 5,
                        JimScriptTokenKind::Expression => 17,
                        JimScriptTokenKind::Separator
                        | JimScriptTokenKind::EndCommand
                        | JimScriptTokenKind::EndSource => panic!("separator has no native object"),
                    };
                    format!("T:{kind}:{}", token.line_delta)
                }
            })
            .collect::<Vec<_>>()
            .join(";")
    }

    #[test]
    fn original_jim_script_layout_matches_native_word_line_and_malformed_tokens() {
        let mut checked = 0;
        for row in include_str!("../tests/data/jim_script_layout.txt")
            .lines()
            .filter(|row| !row.starts_with('#'))
        {
            let fields = row.split('|').collect::<Vec<_>>();
            assert_eq!(fields.len(), 5);
            let source = decode_hex(fields[0]);
            let roster = roster(&source);
            let layout = prepare_jim_script_layout(&roster).unwrap();
            assert_eq!(roster.image.bytes(), source);
            assert_eq!(layout.first_line_delta.to_string(), fields[1], "{source:?}");
            assert_eq!(
                layout.completeness_line,
                JimScriptLine::Original(fields[2].parse().unwrap()),
                "{source:?}"
            );
            let marker: u8 = fields[3].parse().unwrap();
            assert_eq!(
                layout.missing,
                (marker != b' ').then_some(marker),
                "{source:?}"
            );
            assert_eq!(signature(&layout, &roster), fields[4], "{source:?}");
            checked += 1;
        }
        assert_eq!(checked, 15);
    }

    #[test]
    fn layout_refuses_missing_roster_and_replaced_geometry() {
        let mut missing = roster(b"set x value");
        missing.tokens.pop();
        assert_eq!(
            prepare_jim_script_layout(&missing),
            Err(JimScriptLayoutUnavailable::TokenRoster)
        );
        let mut replaced = roster(b"set x value");
        replaced.tokens[0].value = Span::new(0, 100);
        assert_eq!(
            prepare_jim_script_layout(&replaced),
            Err(JimScriptLayoutUnavailable::SourceGeometry)
        );
    }
}
