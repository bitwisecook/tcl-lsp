//! iRule (Tcl) syntax highlighting using the real [`tcl_lexer`] tokeniser.
//!
//! Produces self-contained, pre-escaped HTML: each token is wrapped in a
//! `<span class="tk-…">`. Braced `{…}` words and `[…]` command substitutions
//! are recursed into (an iRule body is a script of nested scripts), so commands
//! and variables deep inside event bodies are highlighted too. On any lex error
//! it falls back to plain escaped text — the report never fails to render.

use crate::{Lexer, TokenType};

const MAX_DEPTH: usize = 64;

/// Highlight an iRule/Tcl source string to HTML.
#[must_use]
pub fn highlight_tcl(src: &str) -> String {
    let mut out = String::with_capacity(src.len() + src.len() / 2);
    highlight_into(src, 0, &mut out);
    out
}

fn highlight_into(src: &str, depth: usize, out: &mut String) {
    if src.is_empty() {
        return;
    }
    // Too deep, or the lexer rejects this fragment: emit it verbatim (escaped).
    let tokens = if depth >= MAX_DEPTH {
        None
    } else {
        Lexer::new(src).tokenise_all().ok()
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
                TokenType::Str => recurse_wrapped(text, '{', '}', depth, out),
                TokenType::Cmd => recurse_wrapped(text, '[', ']', depth, out),
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
fn recurse_wrapped(text: &str, open: char, close: char, depth: usize, out: &mut String) {
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
    highlight_into(inner, depth + 1, out);
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
