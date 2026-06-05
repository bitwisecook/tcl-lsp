//! E100 / E102 stray-closer syntax checks (GAP-A6).
//!
//! Port of `core/analysis/checks/_syntax.py`'s
//! `check_unmatched_close_bracket` (E100) and
//! `check_unmatched_close_brace` (E102), plus their
//! `_helpers.py::_find_bracket_insertion_point` / `_stray_brace_fix`
//! quick-fixes.
//!
//! These are *targeted token checks* — a bare `]` outside command
//! substitution almost always means a missing `[`, and a bare `}`
//! outside a brace word means a missing `{`.  They are distinct from
//! the parser-recovery path (which repairs *unclosed openers*): a stray
//! `]` / `}` produces no recovery diagnostic today, so these emitters
//! add genuinely-missing coverage and never double-report (verified
//! against the live Python and Rust analysers).
//!
//! A `]` / `}` inside a double-quoted string (`puts "a ]"`) is a literal
//! character and must not fire; quoted context is classified exactly as
//! `core/parsing/token_positions.py::classify_quoted_contexts`.

use std::collections::HashSet;

use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::CommandRegistry;

use super::types::{CodeFix, Diagnostic, Severity};
use crate::segmenter::SegmentedCommand;

/// Scan one command's token stream for stray `]` (E100) / `}` (E102)
/// closers, returning the diagnostics (with quick-fixes where one can
/// be derived).
pub(crate) fn stray_closer_diagnostics(
    cmd: &SegmentedCommand,
    source: &str,
    registry: Option<&CommandRegistry>,
) -> Vec<Diagnostic> {
    let tokens = &cmd.all_tokens;
    let in_quoted = classify_quoted_contexts(tokens);
    let mut out: Vec<Diagnostic> = Vec::new();

    for (idx, tok) in tokens.iter().enumerate() {
        if tok.kind != TokenType::Esc || in_quoted.get(idx).copied().unwrap_or(false) {
            continue;
        }
        let Some(text) = token_text(source, tok) else {
            continue;
        };

        // E102: a bare `}`.
        if text == "}" {
            out.push(make_e102(tok, source));
            continue;
        }

        // E100: the first unescaped `]` in the token text.
        if let Some(rel) = first_unescaped_bracket(text) {
            out.push(make_e100(cmd, tokens, idx, rel, source, registry));
        }
    }
    out
}

/// Mark each token that lies inside a double-quoted word.  Faithful
/// port of `classify_quoted_contexts`: a self-contained quoted ESC
/// (`"}"`) has a content shift > 0; a leading / trailing quoted part is
/// recognised via the cross-token `in_quote` flag.  Separators reset
/// the tracker.
fn classify_quoted_contexts(tokens: &[Token]) -> Vec<bool> {
    let mut result = vec![false; tokens.len()];
    let mut prev_in_quote = false;
    for (i, tok) in tokens.iter().enumerate() {
        if matches!(tok.kind, TokenType::Sep | TokenType::Eol) {
            prev_in_quote = false;
            continue;
        }
        let self_opens = tok.kind == TokenType::Esc && tok.content_offset > 0;
        result[i] = prev_in_quote || self_opens;
        prev_in_quote = tok.in_quote;
    }
    result
}

/// The source text of an ESC token (content-offset bytes are leading
/// delimiters; for the unquoted closers we process the offset is 0).
fn token_text<'a>(source: &'a str, tok: &Token) -> Option<&'a str> {
    let start = tok.span.start() as usize;
    let end = tok.span.end() as usize;
    if start <= end && end <= source.len() {
        source.get(start..end)
    } else {
        None
    }
}

/// Byte index of the first `]` not preceded by a backslash, or `None`.
fn first_unescaped_bracket(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b']' {
            if i > 0 && bytes[i - 1] == b'\\' {
                i += 1;
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Build the E100 diagnostic for the `]` at byte `rel` within token
/// `bracket_idx`, attaching the `[`-insertion fix when an insertion
/// point can be inferred.
fn make_e100(
    cmd: &SegmentedCommand,
    tokens: &[Token],
    bracket_idx: usize,
    rel: usize,
    source: &str,
    registry: Option<&CommandRegistry>,
) -> Diagnostic {
    let bracket_off = tokens[bracket_idx].span.start() + u32::try_from(rel).unwrap_or(0);
    let insert = registry.and_then(|reg| {
        find_bracket_insertion_point(cmd, tokens, bracket_idx, bracket_off, source, reg)
    });

    let mut fixes: Vec<CodeFix> = Vec::new();
    let diag_start = if let Some(off) = insert {
        // Zero-width insertion of `[` at `off`.
        fixes.push(CodeFix {
            span: Span::new(off, off),
            new_text: "[".to_string(),
            description: "Insert missing '['".to_string(),
        });
        off
    } else if let Some(first) = tokens.first() {
        first.span.start()
    } else {
        bracket_off
    };

    Diagnostic {
        code: "E100".to_string(),
        span: Span::new(diag_start.min(bracket_off), bracket_off),
        message: "Unmatched ']' \u{2014} missing opening '['?".to_string(),
        severity: Severity::Error,
        fixes,
    }
}

/// Find where the missing `[` should go.  Heuristics, in order: a
/// command name in the text before the `]`; a backward scan for a
/// known command-name ESC token; an arity overflow on the enclosing
/// command.  Mirrors `_find_bracket_insertion_point`.
fn find_bracket_insertion_point(
    cmd: &SegmentedCommand,
    tokens: &[Token],
    bracket_idx: usize,
    bracket_off: u32,
    source: &str,
    registry: &CommandRegistry,
) -> Option<u32> {
    let known: HashSet<&str> = registry.command_names().collect();
    let tok = &tokens[bracket_idx];
    let text = token_text(source, tok)?;

    // 1a: the text before `]` in the same token is a command name.
    if let Some(bidx) = text.find(']') {
        if bidx > 0 && known.contains(&text[..bidx]) {
            return Some(tok.span.start());
        }
    }
    // 1b: backward scan (skip the command word at index 0) for a known
    // command-name ESC token.
    for i in (1..bracket_idx).rev() {
        let t = &tokens[i];
        if t.kind == TokenType::Esc {
            if let Some(name) = token_text(source, t) {
                if known.contains(name) {
                    return Some(t.span.start());
                }
            }
        }
    }
    // 2: arity overflow on the enclosing command.
    let cmd_name = cmd.texts.first()?;
    let spec = registry.get(cmd_name)?;
    let arity = spec.arity;
    if !arity.is_unlimited() && arity.max >= 1 {
        // `argv[1..]` are the arguments (excluding the command name).
        let arg_tokens = cmd.argv.get(1..).unwrap_or(&[]);
        let max = usize::from(arity.max);
        if arg_tokens.len() > max {
            let insert_tok = arg_tokens.get(max - 1)?;
            if insert_tok.span.start() < bracket_off {
                return Some(insert_tok.span.start());
            }
        }
    }
    None
}

/// Build the E102 diagnostic for a bare `}` token, attaching the
/// stray-brace removal fix when the `}` owns its line.
fn make_e102(tok: &Token, source: &str) -> Diagnostic {
    let fixes = stray_brace_fix(tok, source).into_iter().collect();
    Diagnostic {
        code: "E102".to_string(),
        span: tok.span,
        message: "Unmatched '}' \u{2014} missing opening '{'?".to_string(),
        severity: Severity::Error,
        fixes,
    }
}

/// Build a fix that deletes a stray `}` and its line, when the line
/// holds nothing but optional whitespace and the brace.  Mirrors
/// `_stray_brace_fix`.
fn stray_brace_fix(tok: &Token, source: &str) -> Option<CodeFix> {
    let start_off = tok.span.start() as usize;
    let end_off = tok.span.end() as usize;
    if end_off > source.len() {
        return None;
    }
    let line_content_start = source[..start_off].rfind('\n').map_or(0, |p| p + 1);
    let next_nl = source[end_off..].find('\n').map(|p| end_off + p);
    let line_end_off = next_nl.map_or(source.len(), |p| p + 1);

    // Only auto-fix a line that is just optional whitespace + `}`.
    if source[line_content_start..line_end_off].trim() != "}" {
        return None;
    }

    let (del_start, del_end) = if let Some(nl) = next_nl {
        // Delete the whole line including its trailing newline.
        (line_content_start, nl + 1)
    } else if let Some(prev_nl) = source[..start_off].rfind('\n') {
        // No trailing newline: eat the preceding newline through EOF.
        (prev_nl, line_end_off)
    } else {
        // Only line in the file.
        (0, line_end_off)
    };

    Some(CodeFix {
        span: Span::new(
            u32::try_from(del_start).unwrap_or(0),
            u32::try_from(del_end).unwrap_or(0),
        ),
        new_text: String::new(),
        description: "Remove extra '}'".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;

    fn codes(src: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .map(|d| d.code.clone())
            .collect()
    }

    #[test]
    fn stray_close_bracket_emits_e100() {
        // Matches the live Python analyser: `puts foo]` → E100.
        assert!(codes("puts foo]\n").contains(&"E100".to_string()));
        // `]` after a `$var` word also fires.
        assert!(codes("set x $y]\n").contains(&"E100".to_string()));
    }

    #[test]
    fn stray_close_brace_emits_e102() {
        // `}` on its own line → E102 (alongside the W123 unknown-command
        // that both Python and Rust already emit for the bare `}`).
        let c = codes("set x 1\n}\n");
        assert!(c.contains(&"E102".to_string()), "{c:?}");
    }

    #[test]
    fn quoted_closer_is_not_stray() {
        // A `]` inside a double-quoted string is a literal — no E100.
        assert!(!codes("puts \"a ]\"\n").contains(&"E100".to_string()));
    }

    #[test]
    fn bracketed_command_substitution_is_not_stray() {
        // A balanced `[llength $x]` is a command substitution, not a
        // stray closer.
        assert!(!codes("set n [llength $x]\n").contains(&"E100".to_string()));
    }
}
