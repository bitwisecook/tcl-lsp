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

//! The Jim token rosters for a script and for a `subst` template.

use super::{Lexer, TokenProtocol};
use crate::span::Span;
use crate::tokens::TokenType;

impl Lexer<'_> {
    pub(crate) fn jim_script_roster(
        mut self,
        image: crate::SourceImage,
    ) -> Result<crate::JimScriptTokens, crate::JimScriptTokensUnavailable> {
        let jim_grammar = tcl_dialect::model::grammar(
            tcl_dialect::model::Family::Jim,
            tcl_dialect::model::Release::JIM_0_84,
        );
        if self.config.with_grammar(jim_grammar) != self.config || !self.ghosts.is_empty() {
            return Err(crate::JimScriptTokensUnavailable::Grammar);
        }
        self.token_protocol = TokenProtocol::JimNative;
        self.config.strict_quoting = false;
        let mut tokens = Vec::new();
        let mut line_delta = 0_u32;
        let mut previous = 0_usize;
        while let Some(result) = self.next() {
            let token = result.map_err(crate::JimScriptTokensUnavailable::Lexical)?;
            if token.kind == TokenType::Comment || token.span.is_empty() {
                continue;
            }
            let value_start = token.span.start() + u32::from(token.content_offset);
            let body = if token.kind == TokenType::Esc && self.source_map.bytes(token.span) == b"$"
            {
                self.source_map.bytes(token.span)
            } else {
                self.source_map.token_bytes(token)
            };
            let value_len = u32::try_from(body.len())
                .map_err(|_| crate::JimScriptTokensUnavailable::SourceGeometry)?;
            let mut value = Span::new(value_start, value_start + value_len);
            let kind = if token.kind == TokenType::Var && token.content_offset == 2 {
                crate::JimScriptTokenKind::Variable
            } else {
                crate::jim_script_tokens::token_kind(token.kind, self.source_map.token_bytes(token))
            };
            if token.kind == TokenType::ExprSugar {
                value = Span::new(token.span.start() + 1, self.pos);
            }
            line_delta += u32::try_from(bytecount::count(
                &self.source()[previous..token.span.start() as usize],
                b'\n',
            ))
            .map_err(|_| crate::JimScriptTokensUnavailable::SourceGeometry)?;
            previous = token.span.start() as usize;
            tokens.push(crate::JimScriptToken {
                source: Span::new(token.span.start(), self.pos),
                value,
                kind,
                line_delta,
            });
        }
        let end = self.pos;
        let at_end = Span::empty(end);
        tokens.push(crate::JimScriptToken {
            source: at_end,
            value: at_end,
            kind: crate::JimScriptTokenKind::EndCommand,
            line_delta: crate::jim_script_tokens::line_delta(self.source(), end),
        });
        tokens.push(crate::JimScriptToken {
            source: at_end,
            value: at_end,
            kind: crate::JimScriptTokenKind::EndSource,
            line_delta: 0,
        });
        let completeness_line =
            self.jim_missing_line
                .map_or(crate::JimScriptLine::Zero, |offset| {
                    crate::JimScriptLine::Original(crate::jim_script_tokens::line_delta(
                        self.source(),
                        offset,
                    ))
                });
        Ok(crate::JimScriptTokens {
            image,
            tokens,
            missing: self.jim_missing,
            completeness_line,
        })
    }

    pub(crate) fn jim_subst_roster(
        mut self,
        image: crate::SourceImage,
        flags: u8,
    ) -> Result<crate::JimSubstTokens, crate::JimScriptTokensUnavailable> {
        use crate::{JimScriptToken, JimScriptTokenKind, JimScriptTokensUnavailable};
        let grammar = tcl_dialect::model::grammar(
            tcl_dialect::model::Family::Jim,
            tcl_dialect::model::Release::JIM_0_84,
        );
        if self.config.with_grammar(grammar) != self.config
            || !self.ghosts.is_empty()
            || flags & !(7 | 128) != 0
        {
            return Err(JimScriptTokensUnavailable::Grammar);
        }
        self.token_protocol = TokenProtocol::JimNative;
        self.config.strict_quoting = false;
        let mut tokens = Vec::new();
        let mut line_delta = 0_u32;
        while let Some(byte) = self.current_byte() {
            let start = self.pos;
            let parsed = if byte == b'[' && flags & 2 == 0 {
                Some(self.parse_command())
            } else if byte == b'$' && flags & 1 == 0 {
                Some(self.parse_var())
            } else {
                None
            };
            if let Some(parsed) = parsed {
                let token = parsed.map_err(JimScriptTokensUnavailable::Lexical)?;
                if token.kind != TokenType::Esc {
                    let body = self.source_map.token_bytes(token);
                    let length = u32::try_from(body.len())
                        .map_err(|_| JimScriptTokensUnavailable::SourceGeometry)?;
                    let value_start = token.span.start() + u32::from(token.content_offset);
                    let mut value = Span::new(value_start, value_start + length);
                    let kind = if token.kind == TokenType::Var && token.content_offset == 2 {
                        JimScriptTokenKind::Variable
                    } else {
                        crate::jim_script_tokens::token_kind(token.kind, body)
                    };
                    if token.kind == TokenType::ExprSugar {
                        value = Span::new(token.span.start() + 1, self.pos);
                    }
                    tokens.push(JimScriptToken {
                        source: Span::new(start, self.pos),
                        value,
                        kind,
                        line_delta,
                    });
                    // JimParseSubst leaves ordinary-text newlines uncounted.
                    // Only these reached nested parsers advance its line.
                    if token.kind == TokenType::Cmd
                        || (token.kind == TokenType::Var && token.content_offset == 2)
                    {
                        line_delta += u32::try_from(bytecount::count(
                            &self.source()[start as usize..self.pos as usize],
                            b'\n',
                        ))
                        .map_err(|_| JimScriptTokensUnavailable::SourceGeometry)?;
                    }
                    continue;
                }
                // A bare dollar belongs to the following ordinary-text token.
                self.pos = start;
            }
            self.pos += 1;
            if byte == b'\\' && self.current_byte().is_some() {
                self.pos += 1;
            }
            while let Some(byte) = self.current_byte() {
                if (byte == b'$' && flags & 1 == 0) || (byte == b'[' && flags & 2 == 0) {
                    break;
                }
                self.pos += 1;
                if byte == b'\\' && self.current_byte().is_some() {
                    self.pos += 1;
                }
            }
            let span = Span::new(start, self.pos);
            tokens.push(JimScriptToken {
                source: span,
                value: span,
                kind: if flags & 4 == 0 {
                    JimScriptTokenKind::Escaped
                } else {
                    JimScriptTokenKind::String
                },
                line_delta,
            });
        }
        Ok(crate::JimSubstTokens {
            image,
            tokens,
            flags,
        })
    }
}
