// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Full original Jim script tokens, independent of complete-command cuts.

use crate::{LexError, Lexer, LexerConfig, SourceImage, Span, TokenType};

/// Native Jim script token purpose before native object construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JimScriptTokenKind {
    /// Braced, unsubstituted original text.
    String,
    /// Text subject to native escape decoding.
    Escaped,
    /// Original scalar variable name.
    Variable,
    /// Original compound dictionary name and unsubstituted index.
    IndexedVariable,
    /// Original command body.
    Command,
    /// Original parenthesised expression substitution.
    Expression,
    /// Intra-command separators.
    Separator,
    /// End-of-command separators, including the empty end-of-input token.
    EndCommand,
    /// Final native parser sentinel; its native line is absolute zero.
    EndSource,
}

/// One native token retaining both consumed syntax and original operand bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JimScriptToken {
    /// Complete consumed source extent, including real delimiters.
    pub source: Span,
    /// Original token body, without wrappers and without escape decoding.
    pub value: Span,
    /// Native token purpose selected by the original scanner.
    pub kind: JimScriptTokenKind,
    /// Raw LF count before this token, added to the original signed source line.
    pub line_delta: u32,
}

/// Native source line selected independently of parser completeness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JimScriptLine {
    /// Add the raw LF delta to the original signed Source line.
    Original(u32),
    /// Native initial parser token line before any token was emitted.
    Zero,
}

/// Full selected token roster, including separators and malformed tail tokens.
/// This receipt grants no filename object, interpreter identity, or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JimScriptTokens {
    /// Original immutable source and its input channel.
    pub image: SourceImage,
    /// Every parser token, ending in the empty native `EndCommand` and `EndSource` tokens.
    pub tokens: Vec<JimScriptToken>,
    /// Retained completeness marker; absent means native space/complete.
    pub missing: Option<u8>,
    /// Native parser missing.line, retained even for a complete script.
    pub completeness_line: JimScriptLine,
}

impl JimScriptTokens {
    /// First parser-token line, including a leading separator or empty script.
    #[must_use]
    pub fn first_line_delta(&self) -> u32 {
        self.tokens.first().map_or(0, |token| token.line_delta)
    }

    /// Native expansion prefix, consumed before the remaining word's tokens.
    /// Its original Source token is not inserted into the prepared Script.
    #[must_use]
    pub fn expansion_prefix(&self, index: usize) -> bool {
        self.tokens.get(index).is_some_and(|token| {
            token.kind == JimScriptTokenKind::String
                && matches!(
                    self.image.bytes().get(token.value.as_range()),
                    Some(b"*" | b"expand")
                )
                && self
                    .tokens
                    .get(index + 1)
                    .is_some_and(|next| !next.kind.is_separator())
        })
    }
}

impl JimScriptTokenKind {
    /// Separator tokens terminate an original word's token sequence.
    #[must_use]
    pub const fn is_separator(self) -> bool {
        matches!(self, Self::Separator | Self::EndCommand | Self::EndSource)
    }
}

/// Missing authentic source or selected grammar ownership; never a guest error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JimScriptTokensUnavailable {
    /// The caller did not supply the selected Jim variable/script grammar.
    Grammar,
    /// The image uses translated document-source input rather than a native value.
    SourceChannel,
    /// Original counted source geometry cannot be retained.
    SourceGeometry,
    /// The existing lexer could not produce the source stream.
    Lexical(LexError),
}

/// Prepare the complete Jim script token roster before command-cut filtering.
/// Native object construction, escape materialization and original filename
/// ownership remain with the selected interpreter. Malformed input is retained.
///
/// # Errors
/// Returns unavailable selected grammar or original source ownership.
pub fn jim_script_tokens(
    image: &SourceImage,
    config: LexerConfig,
) -> Result<JimScriptTokens, JimScriptTokensUnavailable> {
    if image.channel() != crate::SourceChannel::NativeValue {
        return Err(JimScriptTokensUnavailable::SourceChannel);
    }
    let mut result = Lexer::with_source_image(image, config).jim_script_roster(image.clone())?;
    if result.missing.is_none()
        && let Some(line) = result
            .tokens
            .windows(2)
            .enumerate()
            .find_map(|(index, pair)| {
                (pair[0].kind == JimScriptTokenKind::String
                    && !pair[1].kind.is_separator()
                    && !result.expansion_prefix(index))
                .then_some(pair[1].line_delta)
            })
    {
        result.missing = Some(b'}');
        result.completeness_line = JimScriptLine::Original(line);
    }
    Ok(result)
}

pub(crate) fn line_delta(source: &[u8], at: u32) -> u32 {
    u32::try_from(bytecount::count(&source[..at as usize], b'\n'))
        .expect("source line count fits its u32 byte extent")
}

pub(crate) fn token_kind(kind: TokenType, value: &[u8]) -> JimScriptTokenKind {
    match kind {
        TokenType::Str | TokenType::Expand => JimScriptTokenKind::String,
        TokenType::Esc => JimScriptTokenKind::Escaped,
        TokenType::Var if value.contains(&b'(') => JimScriptTokenKind::IndexedVariable,
        TokenType::Var => JimScriptTokenKind::Variable,
        TokenType::Cmd => JimScriptTokenKind::Command,
        TokenType::ExprSugar => JimScriptTokenKind::Expression,
        TokenType::Sep => JimScriptTokenKind::Separator,
        TokenType::Eol | TokenType::Eof | TokenType::Comment => JimScriptTokenKind::EndCommand,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::{Family, Release, grammar};

    fn prepare(bytes: &[u8]) -> JimScriptTokens {
        jim_script_tokens(
            &SourceImage::native(bytes),
            LexerConfig::from_grammar(grammar(Family::Jim, Release::JIM_0_84)),
        )
        .unwrap()
    }

    fn bodies(plan: &JimScriptTokens) -> Vec<(JimScriptTokenKind, &[u8], u32)> {
        plan.tokens
            .iter()
            .map(|token| {
                (
                    token.kind,
                    &plan.image.bytes()[token.value.as_range()],
                    token.line_delta,
                )
            })
            .collect()
    }

    #[test]
    fn original_script_roster_keeps_native_fragments_and_lines() {
        use JimScriptTokenKind::{
            Command, EndCommand, EndSource, Escaped, Separator, String, Variable,
        };
        let source = b" \nset a \"x$y\\n[z]\"; set b {one\ntwo}";
        let plan = prepare(source);
        assert_eq!(plan.image.bytes(), source);
        assert_eq!(
            bodies(&plan),
            vec![
                (Separator, b" ".as_slice(), 0),
                (EndCommand, b"\n", 0),
                (Escaped, b"set", 1),
                (Separator, b" ", 1),
                (Escaped, b"a", 1),
                (Separator, b" ", 1),
                (Escaped, b"x", 1),
                (Variable, b"y", 1),
                (Escaped, b"\\n", 1),
                (Command, b"z", 1),
                (Escaped, b"", 1),
                (EndCommand, b"; ", 1),
                (Escaped, b"set", 1),
                (Separator, b" ", 1),
                (Escaped, b"b", 1),
                (Separator, b" ", 1),
                (String, b"one\ntwo", 1),
                (EndCommand, b"", 2),
                (EndSource, b"", 0),
            ]
        );
        assert_eq!(plan.first_line_delta(), 0);
        assert_eq!(plan.missing, None);
        assert_eq!(plan.completeness_line, JimScriptLine::Original(1));
        assert_eq!(&source[plan.tokens[9].source.as_range()], b"[z]");
    }

    #[test]
    fn malformed_script_roster_retains_tail_before_native_completeness() {
        use JimScriptTokenKind::{Command, Escaped, IndexedVariable, String, Variable};
        for (source, kind, body, marker) in [
            (
                b"set x {bad".as_slice(),
                String,
                b"bad".as_slice(),
                Some(b'{'),
            ),
            (b"set x \"bad\n$y", Variable, b"y", Some(b'"')),
            (b"set x [echo \"bad", Command, b"echo \"bad", Some(b'"')),
            (b"set x ${bad", Variable, b"bad", None),
            (b"set x $a(b", IndexedVariable, b"a(b", None),
            (b"set x \\", Escaped, b"\\", Some(b'\\')),
        ] {
            let plan = prepare(source);
            let tail = &plan.tokens[plan.tokens.len() - 3];
            assert_eq!(
                (tail.kind, &plan.image.bytes()[tail.value.as_range()]),
                (kind, body)
            );
            assert_eq!(plan.missing, marker, "{source:?}");
            assert_eq!(plan.completeness_line, JimScriptLine::Original(0));
        }
        let initial_quote = prepare(b"\"bad");
        assert_eq!(initial_quote.completeness_line, JimScriptLine::Zero);
        assert_eq!(initial_quote.missing, Some(b'"'));
    }

    #[test]
    fn script_token_objects_preserve_parentheses_expansion_and_counted_nul() {
        use JimScriptTokenKind::{EndCommand, EndSource, Escaped, Expression, Variable};
        let plan = prepare(b"a($y) $($y) \"$\"\n# A\0B\n");
        assert_eq!(
            bodies(&plan)[..5],
            [
                (Escaped, b"a".as_slice(), 0),
                (Escaped, b"(", 0),
                (Variable, b"y", 0),
                (Escaped, b")", 0),
                (JimScriptTokenKind::Separator, b" ", 0),
            ]
        );
        assert_eq!(bodies(&plan)[5], (Expression, b"($y)".as_slice(), 0));
        assert!(bodies(&plan).contains(&(Escaped, b"$".as_slice(), 0)));
        assert!(bodies(&plan).contains(&(Escaped, b"\0B".as_slice(), 1)));
        assert_eq!(plan.tokens.last().unwrap().kind, EndSource);
        assert_eq!(plan.tokens[plan.tokens.len() - 2].kind, EndCommand);
        for source in [b"{*}$x".as_slice(), b"{expand}$x"] {
            let expanded = prepare(source);
            assert!(expanded.expansion_prefix(0));
            assert_eq!(expanded.missing, None);
        }
        let welded = prepare(b"{bad}tail");
        assert!(!welded.expansion_prefix(0));
        assert_eq!(welded.missing, Some(b'}'));
        assert_eq!(welded.completeness_line, JimScriptLine::Original(0));
    }

    #[test]
    fn other_grammars_cannot_issue_jim_script_backing() {
        assert_eq!(
            jim_script_tokens(
                &SourceImage::native(b"set x 1".as_slice()),
                LexerConfig::default()
            ),
            Err(JimScriptTokensUnavailable::Grammar)
        );
    }
}
