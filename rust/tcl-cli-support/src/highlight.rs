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

//! Syntax highlighting over original readonly script regions.
//!
//! One actual analysis retains the CLI's full dialect grammar and discovered
//! command store. The shared source-structure owner selects script interiors;
//! lexical token spans supply the styles. Highlighting supplies syntax advice
//! only and establishes no Native dispatch, entered body or execution result.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;

use crate::registry_for_dialect;
use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_lexer::{LexerConfig, Span, TokenType};
use tcl_registry::{CommandRegistry, SubcommandResolution};

const ANSI_RESET: &str = "\x1b[0m";

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum HighlightKind {
    Command,
    Subcommand,
    Comment,
    Variable,
    CommandSubst,
    Braced,
    Expand,
}

type SpanKey = (u32, u32);

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
        TokenType::Esc if command_spans.contains(&span) => Some(HighlightKind::Command),
        TokenType::Esc if subcommand_spans.contains(&span) => Some(HighlightKind::Subcommand),
        _ => None,
    }
}

fn analyse_source(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> AnalysisResult {
    let config = LexerConfig::for_file_grammar(dialect.grammar);
    let generation = tcl_lsp_core::context_for_dialect_profile(dialect);
    let context = Arc::new(generation.with_command_store(registry.snapshot().shared_registry()));
    let input = ResolvedAnalysisInput::new(dialect, dialect, context, config);
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, dialect.name)
}

struct HighlightPlan {
    structure: tcl_lsp_core::SourceSyntaxStructure,
    commands: HashSet<SpanKey>,
    subcommands: HashSet<SpanKey>,
}

impl HighlightPlan {
    fn capture(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        // Implementation contract: naming.cli.original-source-highlight-geometry
        // docs/design/analysis/name-resolution-proofs/cli-original-source-highlight-geometry.md
        let structure = tcl_lsp_core::SourceSyntaxStructure::capture(source, analysis)?;
        let context = analysis.resolved_input.as_ref()?.context_registry();
        let mut commands = HashSet::new();
        let mut subcommands = HashSet::new();
        for command in structure.commands() {
            if let Some(head) = command.argv.first() {
                commands.insert((head.span.start(), head.span.end()));
            }
            let Some(words) =
                tcl_compiler::registry_invocation::source_structure::source_registry_words(
                    source, analysis, command,
                )
            else {
                continue;
            };
            if words.with_source_schema(&context, |schema| {
                matches!(
                    schema.subcommand,
                    SubcommandResolution::Exact(_) | SubcommandResolution::UniquePrefix(_)
                )
            }) != Some(true)
            {
                continue;
            }
            let Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)) =
                words.origins().get(1)
            else {
                continue;
            };
            if let Some(word) = command.argv.get(*written) {
                subcommands.insert((word.span.start(), word.span.end()));
            }
        }
        Some(Self {
            structure,
            commands,
            subcommands,
        })
    }

    fn styles(&self, source: &str) -> Vec<StyleSpan> {
        let mut styles = Vec::new();
        for syntax in self.structure.lexical_regions() {
            let region = syntax.span();
            if source.get(region.as_range()).is_none() {
                continue;
            }
            let region_len = region.end() - region.start();
            // The region's whitespace and delimiters are source syntax too.
            // This clear layer prevents an outer data-word style from hiding
            // the independently selected interior's lexical tokens.
            styles.push(StyleSpan {
                span: region,
                region_len,
                token: false,
                kind: None,
            });
            for token in syntax.tokens() {
                if token.span.is_empty()
                    || (token.span.start() < region.start() || token.span.end() > region.end())
                {
                    continue;
                }
                styles.push(StyleSpan {
                    span: token.span,
                    region_len,
                    token: true,
                    kind: token_kind(
                        token.kind,
                        (token.span.start(), token.span.end()),
                        &self.commands,
                        &self.subcommands,
                    ),
                });
            }
        }
        styles
    }
}

#[derive(Clone, Copy)]
struct StyleSpan {
    span: Span,
    region_len: u32,
    token: bool,
    kind: Option<HighlightKind>,
}

#[derive(Clone, Copy)]
struct HighlightPiece {
    span: Span,
    kind: Option<HighlightKind>,
}

fn pieces(source: &str, styles: &[StyleSpan]) -> Vec<HighlightPiece> {
    let mut events: BTreeMap<u32, Vec<(usize, bool)>> = BTreeMap::new();
    for (index, style) in styles.iter().enumerate() {
        if style.span.is_empty() || source.get(style.span.as_range()).is_none() {
            continue;
        }
        events
            .entry(style.span.start())
            .or_default()
            .push((index, true));
        events
            .entry(style.span.end())
            .or_default()
            .push((index, false));
    }
    let mut active: BTreeSet<(u32, bool, usize)> = BTreeSet::new();
    let mut out: Vec<HighlightPiece> = Vec::new();
    let mut cursor = 0;
    for (offset, changes) in events {
        if cursor < offset {
            let kind = active.first().and_then(|&(_, _, index)| styles[index].kind);
            if let Some(last) = out
                .last_mut()
                .filter(|piece| piece.kind == kind && piece.span.end() == cursor)
            {
                last.span = Span::new(last.span.start(), offset);
            } else {
                out.push(HighlightPiece {
                    span: Span::new(cursor, offset),
                    kind,
                });
            }
        }
        for (index, starts) in changes {
            let style = styles[index];
            let key = (style.region_len, !style.token, index);
            if starts {
                active.insert(key);
            } else {
                active.remove(&key);
            }
        }
        cursor = offset;
    }
    if let Ok(end) = u32::try_from(source.len())
        && cursor < end
    {
        out.push(HighlightPiece {
            span: Span::new(cursor, end),
            kind: None,
        });
    }
    out
}

fn source_pieces(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<HighlightPiece> {
    // Project, user and bundled packs belong to this same actual input; a
    // nominal catalogue lookup cannot replace the discovered command store.
    let registry = registry_for_dialect(dialect.name);
    let analysis = analyse_source(source, dialect, &registry);
    let styles =
        HighlightPlan::capture(source, &analysis).map_or_else(Vec::new, |plan| plan.styles(source));
    pieces(source, &styles)
}

/// ANSI-highlight current original source syntax under the supplied dialect.
/// Caller chooses whether colour is wanted. Styles preserve source bytes.
#[must_use]
pub fn highlight_ansi(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> String {
    if source.is_empty() {
        return String::new();
    }
    render_ansi(source, &source_pieces(source, dialect))
}

fn render_ansi(source: &str, pieces: &[HighlightPiece]) -> String {
    let mut out = String::with_capacity(source.len());
    for piece in pieces {
        let text = &source[piece.span.as_range()];
        if let Some(kind) = piece.kind {
            out.push_str(ansi_code(kind));
            out.push_str(text);
            out.push_str(ANSI_RESET);
        } else {
            out.push_str(text);
        }
    }
    out
}

/// HTML-highlight the same original readonly syntax regions as ANSI output.
#[must_use]
pub fn highlight_html(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> String {
    if source.is_empty() {
        return "<pre></pre>\n".to_owned();
    }
    let mut out = String::with_capacity(source.len());
    for piece in source_pieces(source, dialect) {
        let text = html_escape(&source[piece.span.as_range()]);
        if let Some(kind) = piece.kind {
            out.push_str("<span style=\"");
            out.push_str(html_style(kind));
            out.push_str("\">");
            out.push_str(&text);
            out.push_str("</span>");
        } else {
            out.push_str(&text);
        }
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
