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

//! iRule (Tcl) syntax highlighting using the real [`tcl_lexer`] tokeniser.
//!
//! Produces self-contained, pre-escaped HTML: each token is wrapped in a
//! `<span class="tk-…">`. Braced `{…}` words and `[…]` command substitutions
//! are recursed into (an iRule body is a script of nested scripts), so commands
//! and variables deep inside event bodies are highlighted too. On any lex error
//! it falls back to plain escaped text — the report never fails to render.

use crate::{Lexer, LexerConfig, SourceMap, TokenType};

const MAX_DEPTH: usize = 64;

/// Highlight an iRule/Tcl source string to HTML using stock-Tcl lexing.
#[must_use]
pub fn highlight_tcl(src: &str) -> String {
    highlight_tcl_with_config(src, LexerConfig::default())
}

/// Highlight with an explicit lexer preset — pass
/// [`LexerConfig::for_dialect("f5-irules")`](LexerConfig::for_dialect) to
/// colour an iRule correctly, so `if {expr}{body}` (`}{` valid in TMM) is not
/// mis-tokenised the way the stock-Tcl preset would.
#[must_use]
pub fn highlight_tcl_with_config(src: &str, config: LexerConfig) -> String {
    let mut out = String::with_capacity(src.len() + src.len() / 2);
    highlight_into(src, 0, config, &mut out);
    out
}

/// One highlighted token range, as absolute byte offsets into the source and
/// the CSS class (`"tk-cmd"`, `"tk-var"`, …) it should carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HlRange {
    /// Absolute byte offset of the range start.
    pub start: usize,
    /// Absolute byte offset of the range end (exclusive).
    pub end: usize,
    /// CSS class for the token colour.
    pub class: &'static str,
}

/// The classed token ranges of a Tcl/iRule source, as absolute byte offsets —
/// the same colouring [`highlight_tcl`] applies, but as data rather than HTML so
/// a caller can overlay other span layers (e.g. analyser diagnostics) before
/// rendering. Ranges are non-overlapping and in source order; bytes not covered
/// by any range are plain text. Returns an empty vec on lex failure (matching
/// [`highlight_tcl`]'s verbatim fallback, which classes nothing).
#[must_use]
pub fn highlight_ranges(src: &str) -> Vec<HlRange> {
    highlight_ranges_with_config(src, LexerConfig::default())
}

/// [`highlight_ranges`] with an explicit lexer preset (see
/// [`highlight_tcl_with_config`]).
#[must_use]
pub fn highlight_ranges_with_config(src: &str, config: LexerConfig) -> Vec<HlRange> {
    let mut ranges = Vec::new();
    collect_ranges(src, 0, 0, config, &mut ranges);
    ranges
}

/// Recursive twin of [`highlight_into`] that records absolute classed ranges
/// instead of emitting HTML. `base` is the absolute offset of `src` within the
/// whole document, so nested braced/bracketed scripts report document-absolute
/// spans. Kept structurally identical to `highlight_into` so the colouring can
/// never drift (the `ranges_reproduce_highlight_html` test pins this).
fn collect_ranges(
    src: &str,
    depth: usize,
    base: usize,
    config: LexerConfig,
    out: &mut Vec<HlRange>,
) {
    if src.is_empty() {
        return;
    }
    let tokens = if depth >= MAX_DEPTH {
        None
    } else {
        Lexer::with_source_map(SourceMap::new(src), config)
            .tokenise_all()
            .ok()
    };
    let Some(tokens) = tokens else {
        return; // verbatim fallback → nothing classed
    };

    let mut cmd_start = true;
    for tok in &tokens {
        let s = (tok.span.start() as usize).min(src.len());
        let e = (tok.span.end() as usize).min(src.len());
        if e > s {
            let text = &src[s..e];
            match tok.kind {
                TokenType::Comment => out.push(range(base + s, base + e, "tk-comment")),
                TokenType::Var => out.push(range(base + s, base + e, "tk-var")),
                TokenType::Str => collect_recurse(text, '{', '}', depth, base + s, config, out),
                TokenType::Cmd => collect_recurse(text, '[', ']', depth, base + s, config, out),
                TokenType::Esc => {
                    if cmd_start {
                        out.push(range(
                            base + s,
                            base + e,
                            if text.contains("::") {
                                "tk-ns"
                            } else {
                                "tk-cmd"
                            },
                        ));
                    } else if let Some(cls) = classify_word(text) {
                        out.push(range(base + s, base + e, cls));
                    }
                }
                _ => {}
            }
        }
        match tok.kind {
            TokenType::Eol => cmd_start = true,
            TokenType::Sep | TokenType::Eof => {}
            _ => cmd_start = false,
        }
    }
}

/// Range-collecting twin of [`recurse_wrapped`]: skip the delimiters (which are
/// plain text) and recurse into the inner script at the right absolute base.
fn collect_recurse(
    text: &str,
    open: char,
    close: char,
    depth: usize,
    text_base: usize,
    config: LexerConfig,
    out: &mut Vec<HlRange>,
) {
    let mut inner = text;
    let mut inner_base = text_base;
    if inner.starts_with(open) {
        inner = &inner[open.len_utf8()..];
        inner_base += open.len_utf8();
    }
    if inner.ends_with(close) {
        inner = &inner[..inner.len() - close.len_utf8()];
    }
    collect_ranges(inner, depth + 1, inner_base, config, out);
}

const fn range(start: usize, end: usize, class: &'static str) -> HlRange {
    HlRange { start, end, class }
}

fn highlight_into(src: &str, depth: usize, config: LexerConfig, out: &mut String) {
    if src.is_empty() {
        return;
    }
    // Too deep, or the lexer rejects this fragment: emit it verbatim (escaped).
    let tokens = if depth >= MAX_DEPTH {
        None
    } else {
        Lexer::with_source_map(SourceMap::new(src), config)
            .tokenise_all()
            .ok()
    };
    let Some(tokens) = tokens else {
        push_escaped(src, out);
        return;
    };

    let mut pos = 0usize;
    let mut cmd_start = true; // the next bare word is a command name
    for tok in &tokens {
        let s = (tok.span.start() as usize).min(src.len());
        let e = (tok.span.end() as usize).min(src.len());
        if s > pos {
            push_escaped(&src[pos..s], out);
        }
        if e > s {
            let text = &src[s..e];
            match tok.kind {
                TokenType::Comment => wrap(out, "tk-comment", text),
                TokenType::Var => wrap(out, "tk-var", text),
                TokenType::Str => recurse_wrapped(text, '{', '}', depth, config, out),
                TokenType::Cmd => recurse_wrapped(text, '[', ']', depth, config, out),
                TokenType::Esc => {
                    if cmd_start {
                        // A command name; a namespaced one (`HTTP::respond`) keeps
                        // its namespace colour.
                        wrap(
                            out,
                            if text.contains("::") {
                                "tk-ns"
                            } else {
                                "tk-cmd"
                            },
                            text,
                        );
                    } else if let Some(cls) = classify_word(text) {
                        wrap(out, cls, text);
                    } else {
                        push_escaped(text, out);
                    }
                }
                _ => push_escaped(text, out),
            }
        }
        // Track command position: a terminator starts a new command; any real
        // word after the first is an argument.
        match tok.kind {
            TokenType::Eol => cmd_start = true,
            TokenType::Sep | TokenType::Eof => {}
            _ => cmd_start = false,
        }
        pos = e.max(pos);
    }
    if pos < src.len() {
        push_escaped(&src[pos..], out);
    }
}

/// A braced `{…}` word or `[…]` command substitution: recurse into the inner
/// script so nested commands/vars are highlighted. The lexer strips the
/// delimiters from the token span (they arrive as the surrounding gap text), so
/// the token text is normally already the inner script and is recursed
/// directly; if a lexer path does keep the delimiters, strip them first.
fn recurse_wrapped(
    text: &str,
    open: char,
    close: char,
    depth: usize,
    config: LexerConfig,
    out: &mut String,
) {
    // The token span keeps the opening delimiter but drops the closing one
    // (which arrives as the following gap); strip a leading `{`/`[`, and a
    // trailing `}`/`]` if this path happens to include it, then recurse.
    let mut inner = text;
    if inner.starts_with(open) {
        out.push(open);
        inner = &inner[open.len_utf8()..];
    }
    let mut trailer = "";
    if inner.ends_with(close) {
        trailer = &inner[inner.len() - close.len_utf8()..];
        inner = &inner[..inner.len() - close.len_utf8()];
    }
    highlight_into(inner, depth + 1, config, out);
    out.push_str(trailer);
}

/// Classify a non-command bare word: an iRule event / TMSH constant
/// (`HTTP_REQUEST`), a namespaced command/proc (`HTTP::host`), or nothing.
fn classify_word(text: &str) -> Option<&'static str> {
    if text.contains("::") {
        return Some("tk-ns");
    }
    // ALL_CAPS identifiers are iRule events and TCL-level constants.
    if text.len() >= 2
        && text.starts_with(|c: char| c.is_ascii_uppercase())
        && text
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return Some("tk-event");
    }
    None
}

fn wrap(out: &mut String, class: &str, text: &str) {
    out.push_str("<span class=\"");
    out.push_str(class);
    out.push_str("\">");
    push_escaped(text, out);
    out.push_str("</span>");
}

fn push_escaped(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Render highlight ranges back to HTML the same way `highlight_tcl` does —
    /// escaped gaps, `<span class>`-wrapped classed ranges. If this reproduces
    /// `highlight_tcl` byte-for-byte, the data and HTML paths cannot drift.
    fn render_from_ranges(src: &str) -> String {
        let mut out = String::new();
        let mut pos = 0usize;
        for r in highlight_ranges(src) {
            if r.start > pos {
                push_escaped(&src[pos..r.start], &mut out);
            }
            wrap(&mut out, r.class, &src[r.start..r.end]);
            pos = r.end;
        }
        if pos < src.len() {
            push_escaped(&src[pos..], &mut out);
        }
        out
    }

    #[test]
    fn ranges_reproduce_highlight_html() {
        let corpus = [
            "",
            "pool $html-pool",
            "when HTTP_REQUEST {\n  set x 1\n  pool css_pool\n}",
            "log local0. \"uri=[HTTP::uri] & <tag>\"",
            "if { [HTTP::path] ends_with \".css\" } { pool css_pool }",
            "# a comment\nset ns::var [expr {$a + $b}]",
            "set html-pool web1Pool\npool $html-pool",
            "when RULE_INIT { set ::static::x {a {b} c} }",
        ];
        for src in corpus {
            assert_eq!(
                render_from_ranges(src),
                highlight_tcl(src),
                "range render diverged from highlight_tcl for: {src:?}"
            );
        }
    }

    #[test]
    fn f5irules_config_highlights_body_after_brace_chain() {
        // `if {1}{ pool p }` — TMM `}{`. With the stock preset the `}{` derails
        // command tracking so `pool` isn't seen as a command; with the f5-irules
        // preset the ghost SEP makes `{ pool p }` a separate word, so `pool`
        // inside it is highlighted as a command.
        let src = "if {1}{ pool p }";
        let irules = LexerConfig::for_dialect("f5-irules");
        let ranges = highlight_ranges_with_config(src, irules);
        let pool = ranges.iter().find(|r| &src[r.start..r.end] == "pool");
        assert_eq!(
            pool.map(|r| r.class),
            Some("tk-cmd"),
            "f5-irules preset should treat `pool` after `}}{{` as a command"
        );
        // The HTML variant agrees with the ranges variant.
        assert!(
            highlight_tcl_with_config(src, irules).contains("<span class=\"tk-cmd\">pool</span>")
        );
    }

    #[test]
    fn ranges_are_absolute_and_ordered() {
        let src = "when HTTP_REQUEST {\n  pool css_pool\n}";
        let ranges = highlight_ranges(src);
        // ordered, non-overlapping, in-bounds
        let mut prev_end = 0;
        for r in &ranges {
            assert!(r.start >= prev_end, "overlap/out-of-order at {r:?}");
            assert!(r.end <= src.len());
            prev_end = r.end;
        }
        // the event token is classed as an event, at its real offset
        let ev = ranges
            .iter()
            .find(|r| &src[r.start..r.end] == "HTTP_REQUEST");
        assert_eq!(ev.map(|r| r.class), Some("tk-event"));
    }
}
