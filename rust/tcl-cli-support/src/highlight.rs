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

//! Syntax highlighter (ANSI + HTML).
//!
//! Lexer-driven (no analyser/optimiser dependency), so this reaches byte-for-byte
//! agreement with the captured golden output. Command heads and registry-resolved subcommands
//! are detected via the command segmenter + registry, exactly as
//! `collect_command_spans` does.

use std::collections::HashSet;

use crate::registry_for_dialect;
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{Lexer, LexerConfig, TokenType};

const ANSI_RESET: &str = "\x1b[0m";

/// Highlight kind → ANSI SGR escape.
fn ansi_code(kind: HighlightKind) -> &'static str {
    match kind {
        HighlightKind::Command => "\x1b[1;34m",
        HighlightKind::Subcommand => "\x1b[34m",
        HighlightKind::Comment => "\x1b[90m",
        HighlightKind::Variable => "\x1b[35m",
        HighlightKind::CommandSubst => "\x1b[36m",
        HighlightKind::Braced => "\x1b[32m",
        HighlightKind::Expand => "\x1b[33m",
    }
}

/// Highlight kind → inline CSS.
fn html_style(kind: HighlightKind) -> &'static str {
    match kind {
        HighlightKind::Command => "color:#2b6cb0;font-weight:600;",
        HighlightKind::Subcommand => "color:#2c5282;",
        HighlightKind::Comment => "color:#6b7280;",
        HighlightKind::Variable => "color:#9f3ec7;",
        HighlightKind::CommandSubst => "color:#0f766e;",
        HighlightKind::Braced => "color:#2f855a;",
        HighlightKind::Expand => "color:#b7791f;",
    }
}

#[derive(Clone, Copy)]
enum HighlightKind {
    Command,
    Subcommand,
    Comment,
    Variable,
    CommandSubst,
    Braced,
    Expand,
}

/// Byte-span key `(start, end)` used to tag command / subcommand heads.
type SpanKey = (u32, u32);

/// Classify a token.
fn token_kind(
    ty: TokenType,
    span: SpanKey,
    command_spans: &HashSet<SpanKey>,
    subcommand_spans: &HashSet<SpanKey>,
) -> Option<HighlightKind> {
    match ty {
        TokenType::Comment => Some(HighlightKind::Comment),
        TokenType::Var => Some(HighlightKind::Variable),
        TokenType::Cmd => Some(HighlightKind::CommandSubst),
        TokenType::Str => Some(HighlightKind::Braced),
        TokenType::Expand => Some(HighlightKind::Expand),
        TokenType::Esc => {
            if command_spans.contains(&span) {
                Some(HighlightKind::Command)
            } else if subcommand_spans.contains(&span) {
                Some(HighlightKind::Subcommand)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Collect the spans of command heads and resolved subcommands.
fn collect_command_spans(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> (HashSet<SpanKey>, HashSet<SpanKey>) {
    // `crate::registry_for_dialect`, **not** `tcl_registry`'s
    // `registry_for_profile`: this crate's own by-name entry layers the
    // discovered SpecTcl packs (`cli_packs()`) on top of the catalogue
    // registry. Resolving straight from the profile would silently drop them,
    // so a project-local pack's commands and subcommands would stop being
    // highlighted — pinned by `highlight_uses_the_current_projects_spec_pack_registry`.
    let registry = registry_for_dialect(dialect.name);
    let mut command_spans = HashSet::new();
    let mut subcommand_spans = HashSet::new();
    for command in segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
    ) {
        if let Some(first) = command.argv.first() {
            command_spans.insert((first.span.start(), first.span.end()));
        }
        if command.texts.len() >= 2
            && let Some(spec) = registry.get(&command.texts[0])
        {
            let candidate = &command.texts[1];
            if !spec.subcommands.is_empty()
                && spec.subcommands.iter().any(|s| s.name == candidate)
                && let Some(second) = command.argv.get(1)
            {
                subcommand_spans.insert((second.span.start(), second.span.end()));
            }
        }
    }
    (command_spans, subcommand_spans)
}

/// A braced word that looks like a command body.
fn is_body_token(text: &str) -> bool {
    if !text.starts_with('{') {
        return false;
    }
    let inner = text[1..].trim_end_matches('}');
    inner.contains('\n') || inner.contains(';')
}

/// ANSI-highlight `source` for the given dialect. Caller decides whether colour
/// is wanted (ANSI colour variant).
#[must_use]
pub fn highlight_ansi(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> String {
    highlight_ansi_inner(source, dialect, 0)
}

fn highlight_ansi_inner(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    depth: u32,
) -> String {
    if source.is_empty() || depth > 8 {
        return source.to_owned();
    }
    // Depth 0 is the whole file (BOM prologue applies); deeper is a braced body.
    let config = if depth == 0 {
        LexerConfig::for_file_grammar(dialect.grammar)
    } else {
        LexerConfig::for_profile(Some(dialect))
    };
    let Ok(tokens) = Lexer::with_config(source, config).tokenise_all() else {
        return source.to_owned();
    };
    let (command_spans, subcommand_spans) = collect_command_spans(source, dialect);

    let mut out = String::with_capacity(source.len());
    let mut cursor = 0usize;
    for token in &tokens {
        let start = token.span.start() as usize;
        let end = token.span.end() as usize;
        if end <= start {
            continue;
        }
        if start > cursor {
            out.push_str(&source[cursor..start]);
        }
        let key = (token.span.start(), token.span.end());
        let text = &source[start..end];

        if token.kind == TokenType::Str && is_body_token(text) {
            let (inner, suffix) = if text.ends_with('}') {
                (&text[1..text.len() - 1], "}")
            } else {
                (&text[1..], "")
            };
            out.push('{');
            out.push_str(&highlight_ansi_inner(inner, dialect, depth + 1));
            out.push_str(suffix);
            cursor = end;
            continue;
        }

        match token_kind(token.kind, key, &command_spans, &subcommand_spans) {
            None => out.push_str(text),
            Some(kind) => {
                out.push_str(ansi_code(kind));
                out.push_str(text);
                out.push_str(ANSI_RESET);
            }
        }
        cursor = end;
    }
    if cursor < source.len() {
        out.push_str(&source[cursor..]);
    }
    out
}

/// HTML-highlight `source`.
#[must_use]
pub fn highlight_html(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> String {
    if source.is_empty() {
        return "<pre></pre>\n".to_owned();
    }
    let Ok(tokens) =
        Lexer::with_config(source, LexerConfig::for_file_grammar(dialect.grammar)).tokenise_all()
    else {
        return format!("<pre>\n{}\n</pre>\n", html_escape(source));
    };
    let (command_spans, subcommand_spans) = collect_command_spans(source, dialect);

    let mut out = String::with_capacity(source.len());
    let mut cursor = 0usize;
    for token in &tokens {
        let start = token.span.start() as usize;
        let end = token.span.end() as usize;
        if end <= start {
            continue;
        }
        if start > cursor {
            out.push_str(&html_escape(&source[cursor..start]));
        }
        let key = (token.span.start(), token.span.end());
        let text = html_escape(&source[start..end]);
        match token_kind(token.kind, key, &command_spans, &subcommand_spans) {
            None => out.push_str(&text),
            Some(kind) => {
                out.push_str("<span style=\"");
                out.push_str(html_style(kind));
                out.push_str("\">");
                out.push_str(&text);
                out.push_str("</span>");
            }
        }
        cursor = end;
    }
    if cursor < source.len() {
        out.push_str(&html_escape(&source[cursor..]));
    }
    format!("<pre>\n{out}\n</pre>\n")
}

/// Escape text for HTML, replacing `&`, `<`, `>`, `"`, and `'` with entities.
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            other => out.push(other),
        }
    }
    out
}
