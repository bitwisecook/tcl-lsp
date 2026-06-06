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

/// E201 (GAP-A1): a `[` command substitution with no closing `]`.
/// Mirrors the unterminated-bracket detectors of
/// `core/parsing/recovery.py` (the user-facing E201 diagnostic; the
/// ghost-token re-lex that produces a clean command stream is a
/// follow-up strip).  For each unterminated `Cmd` token it picks, in
/// priority order, where the `]` belongs: before a `#` comment line, a
/// known-command line, or a `{`; otherwise it anchors at the `[`.
pub(crate) fn unterminated_bracket_diagnostics(
    cmd: &SegmentedCommand,
    source: &str,
    registry: Option<&CommandRegistry>,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for tok in &cmd.all_tokens {
        if tok.kind != TokenType::Cmd || !is_unterminated_cmd(tok, source) {
            continue;
        }
        let bracket_off = tok.span.start();
        let content_start = tok.span.start() + u32::from(tok.content_offset);
        let (cs, ce) = (content_start as usize, tok.span.end() as usize);
        let content = if cs <= ce && ce <= source.len() {
            &source[cs..ce]
        } else {
            ""
        };
        out.push(detect_e201(content, content_start, bracket_off, registry));
    }
    out
}

/// True when `tok` (a `Cmd` token) has no closing `]`.  Mirrors
/// `_is_unterminated_cmd`.
///
/// For a non-empty `[…]` the inner-end span excludes the closing
/// bracket, so the `]` sits *at* `span.end()`.  The empty `[]` is a
/// lexer special case: its span covers *both* brackets, so the `]` sits
/// at `span.end() - 1` — accept that too, otherwise an empty command
/// substitution would raise a spurious E201.
fn is_unterminated_cmd(tok: &Token, source: &str) -> bool {
    let bytes = source.as_bytes();
    let end = tok.span.end() as usize;
    if bytes.get(end) == Some(&b']') {
        return false; // non-empty `[…]`: `]` sits at span end
    }
    // Empty `[]`: inner content length zero (`span.end() == content_start
    // + 1`), the lone byte being the closing `]` one before span end.
    let content_start = tok.span.start() + u32::from(tok.content_offset);
    let empty_closed =
        tok.span.end() == content_start + 1 && end > 0 && bytes.get(end - 1) == Some(&b']');
    !empty_closed
}

/// Build the E201 diagnostic for an unterminated `[`, choosing the
/// insertion point via the comment / known-command / brace heuristics
/// (in priority order), falling back to the bare `[`.
fn detect_e201(
    content: &str,
    content_start: u32,
    bracket_off: u32,
    registry: Option<&CommandRegistry>,
) -> Diagnostic {
    // GAP-A1 + recovery-rust-port: the heuristics *propose* an insertion
    // offset (semantic); the structural index *validates* it
    // (syntactic). A proposed `]` that lands inside an inert span — a
    // brace word, an escape pair, a `${…}`, or a command-sub brace
    // interior — would be a literal, not a real closer, so the re-lex
    // would split a brace word (`[foo {bar\nputs baz}` must close after
    // `}`, not after `bar`). Veto such a fix and fall through to the next
    // heuristic / the fix-less fallback. `is_inert` is the *sound*
    // signal (it never marks a structural position inert), so this can
    // only remove wrong fixes, never good ones. The index is built over
    // the bracket interior (`content`), so offsets are content-relative.
    let index = tcl_lexer::BracketIndex::build(content);
    let accept = |d: Diagnostic| -> Option<Diagnostic> {
        if let Some(fix) = d.fixes.first() {
            let rel = fix.span.start().saturating_sub(content_start);
            if index.is_inert(rel) {
                return None;
            }
        }
        Some(d)
    };
    if let Some(d) = e201_at_comment(content, content_start, bracket_off).and_then(&accept) {
        return d;
    }
    if let Some(reg) = registry {
        if let Some(d) =
            e201_at_command(content, content_start, bracket_off, reg, &index).and_then(&accept)
        {
            return d;
        }
    }
    if let Some(d) = e201_at_brace(content, content_start, bracket_off).and_then(&accept) {
        return d;
    }
    // Fallback: highlight just the opening `[`, no fix.
    Diagnostic {
        code: "E201".to_string(),
        span: Span::new(bracket_off, bracket_off),
        message: "missing close-bracket".to_string(),
        severity: Severity::Error,
        fixes: Vec::new(),
    }
}

/// Build an E201 anchored from the `[` to `insert_idx`-in-content, with
/// a `]`-insertion fix at `insert_idx` and the given fix description.
fn e201_with_insert(
    content_start: u32,
    bracket_off: u32,
    insert_idx: usize,
    fix_desc: &str,
) -> Diagnostic {
    let insert_off = content_start + u32::try_from(insert_idx).unwrap_or(0);
    // Diagnostic end: the last content byte before the insertion.
    let diag_end = content_start + u32::try_from(insert_idx.saturating_sub(1)).unwrap_or(0);
    Diagnostic {
        code: "E201".to_string(),
        span: Span::new(bracket_off, diag_end.max(bracket_off)),
        message: "missing close-bracket".to_string(),
        severity: Severity::Error,
        fixes: vec![CodeFix {
            span: Span::new(insert_off, insert_off),
            new_text: "]".to_string(),
            description: fix_desc.to_string(),
        }],
    }
}

/// E201 heuristic: a `#` comment line follows — insert `]` at the end of
/// the previous line's content.  Mirrors `_detect_missing_bracket_at_comment`.
fn e201_at_comment(content: &str, content_start: u32, bracket_off: u32) -> Option<Diagnostic> {
    let lines: Vec<&str> = content.split('\n').collect();
    if lines.len() < 2 {
        return None;
    }
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let stripped = line.trim_start();
        if stripped.starts_with('#') {
            let insert_idx = prev_line_content_end(&lines, i);
            return Some(e201_with_insert(
                content_start,
                bracket_off,
                insert_idx,
                "Insert missing ']' before comment",
            ));
        }
        if !stripped.is_empty() {
            break;
        }
    }
    None
}

/// E201 heuristic: a known-command line follows — insert `]` at the end
/// of the previous line.  Mirrors `_detect_missing_bracket_at_command`.
fn e201_at_command(
    content: &str,
    content_start: u32,
    bracket_off: u32,
    registry: &CommandRegistry,
    index: &tcl_lexer::BracketIndex,
) -> Option<Diagnostic> {
    let lines: Vec<&str> = content.split('\n').collect();
    if lines.len() < 2 {
        return None;
    }
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let stripped = line.trim_start();
        if stripped.is_empty() {
            continue;
        }
        let insert_idx = prev_line_content_end(&lines, i);
        // recovery-rust-port "syntactic validates": if this boundary
        // sits inside an inert span (a brace word / quoted run that
        // swallowed the line, e.g. `puts baz` inside `{bar\nputs baz}`),
        // the candidate `]` would be a literal — keep scanning for the
        // next real command past the inert span rather than giving up.
        if index.is_inert(u32::try_from(insert_idx).unwrap_or(0)) {
            continue;
        }
        let first_word = extract_first_word(stripped);
        if registry.get(first_word).is_some() {
            return Some(e201_with_insert(
                content_start,
                bracket_off,
                insert_idx,
                "Insert missing ']' before command",
            ));
        }
        break;
    }
    None
}

/// E201 heuristic: a `{` swallowed the rest — insert `]` before it
/// (after trailing whitespace).  Mirrors `_detect_missing_bracket_at_brace`.
fn e201_at_brace(content: &str, content_start: u32, bracket_off: u32) -> Option<Diagnostic> {
    let brace_idx = content.find('{')?;
    let bytes = content.as_bytes();
    let mut insert_idx = brace_idx;
    while insert_idx > 0 && matches!(bytes[insert_idx - 1], b' ' | b'\t') {
        insert_idx -= 1;
    }
    if content[..insert_idx].trim_end().is_empty() {
        return None;
    }
    Some(e201_with_insert(
        content_start,
        bracket_off,
        insert_idx,
        "Insert missing ']' before '{'",
    ))
}

/// E202 / E203 (GAP-A1): unterminated `"` / `{` recovery diagnostics.
///
/// Mirrors the E202 / E203 detectors of `core/parsing/recovery.py`
/// (`_is_suspicious_quote` + `_detect_missing_quote_*`,
/// `_is_suspicious_str` + `_detect_missing_brace_*`).  For each
/// genuinely unterminated quote / brace token in the command it emits one
/// diagnostic: the known-command heuristic with a closing-delimiter
/// insertion fix when one can be located, else the fix-less fallback.
/// Returns an empty vector for well-formed input.
pub(crate) fn unterminated_delimiter_diagnostics(
    cmd: &SegmentedCommand,
    source: &str,
    registry: Option<&CommandRegistry>,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for tok in &cmd.all_tokens {
        if is_suspicious_quote(tok, cmd, source) {
            out.push(detect_e202(tok, source, registry));
        } else if is_suspicious_str(tok, source) {
            out.push(detect_e203(tok, source, registry));
        }
    }
    out
}

/// The inner text of `tok` (past its opening delimiter), or `None` for
/// an out-of-bounds span.
fn token_inner<'a>(source: &'a str, tok: &Token) -> Option<&'a str> {
    let start = tok.span.start() as usize + tok.content_offset as usize;
    let end = tok.span.end() as usize;
    if start <= end && end <= source.len() {
        source.get(start..end)
    } else {
        None
    }
}

/// True when `tok` is an `Esc` from an unterminated `"` at end-of-line
/// that swallows the rest of the document.  Mirrors `_is_suspicious_quote`:
/// the token starts at a `"`, its inner text begins with a newline with
/// non-blank content after it, and the command reaches EOF.
fn is_suspicious_quote(tok: &Token, cmd: &SegmentedCommand, source: &str) -> bool {
    if tok.kind != TokenType::Esc {
        return false;
    }
    if source.as_bytes().get(tok.span.start() as usize) != Some(&b'"') {
        return false;
    }
    let Some(text) = token_inner(source, tok) else {
        return false;
    };
    if !text.starts_with('\n') || text[1..].trim().is_empty() {
        return false;
    }
    // A properly closed quote wouldn't run to EOF.
    if let Some(last) = cmd.all_tokens.last() {
        if (last.span.end() as usize) < source.len().saturating_sub(1) {
            return false;
        }
    }
    true
}

/// True when `tok` is a `Str` from an unterminated `{` spanning multiple
/// lines with no closing `}`.  Mirrors `_is_suspicious_str`: a token
/// text containing `}` is E103 territory (brace closed at the wrong
/// nesting level), not a truly missing brace.
fn is_suspicious_str(tok: &Token, source: &str) -> bool {
    if tok.kind != TokenType::Str {
        return false;
    }
    if source.as_bytes().get(tok.span.start() as usize) != Some(&b'{') {
        return false;
    }
    // Properly closed: the byte at the inner end is the `}`.
    if source.as_bytes().get(tok.span.end() as usize) == Some(&b'}') {
        return false;
    }
    let Some(text) = token_inner(source, tok) else {
        return false;
    };
    if text.contains('}') {
        return false;
    }
    // Must span at least two lines.
    if text.bytes().filter(|&b| b == b'\n').count() < 2 {
        return false;
    }
    true
}

/// Build the E202 diagnostic for an unterminated `"`: the known-command
/// heuristic (insert `"` right after the opener) or the fix-less
/// fallback.  Mirrors `_detect_missing_quote_at_newline` /
/// `_detect_missing_quote_no_heuristic`.
fn detect_e202(tok: &Token, source: &str, registry: Option<&CommandRegistry>) -> Diagnostic {
    let quote_off = tok.span.start();
    let diag_span = Span::new(quote_off, quote_off);
    if let Some(reg) = registry {
        let text = token_inner(source, tok).unwrap_or("");
        let lines: Vec<&str> = text.split('\n').collect();
        if lines.len() >= 2 {
            for (i, line) in lines.iter().enumerate() {
                if i == 0 {
                    continue;
                }
                let stripped = line.trim_start();
                if stripped.is_empty() {
                    continue;
                }
                if reg.get(extract_first_word(stripped)).is_some() {
                    // Virtual `"` right after the opening `"`.
                    let insert_off = quote_off + 1;
                    return Diagnostic {
                        code: "E202".to_string(),
                        span: diag_span,
                        message: "missing \"".to_string(),
                        severity: Severity::Error,
                        fixes: vec![CodeFix {
                            span: Span::new(insert_off, insert_off),
                            new_text: "\"".to_string(),
                            description: "Insert missing '\"' to close string".to_string(),
                        }],
                    };
                }
                // First non-blank line isn't a known command — stop.
                break;
            }
        }
    }
    Diagnostic {
        code: "E202".to_string(),
        span: diag_span,
        message: "missing \"".to_string(),
        severity: Severity::Error,
        fixes: Vec::new(),
    }
}

/// Build the E203 diagnostic for an unterminated `{`: the de-indented
/// known-command heuristic (insert `}` at the newline before that line)
/// or the fix-less fallback.  Mirrors `_detect_missing_brace_at_command`
/// / `_detect_missing_brace_no_heuristic`.
fn detect_e203(tok: &Token, source: &str, registry: Option<&CommandRegistry>) -> Diagnostic {
    let brace_off = tok.span.start();
    let diag_span = Span::new(brace_off, brace_off);
    let content_start = brace_off + u32::from(tok.content_offset);
    if let Some(reg) = registry {
        if let Some(fix) = e203_brace_fix(tok, source, content_start, reg) {
            return Diagnostic {
                code: "E203".to_string(),
                span: diag_span,
                message: "missing close-brace".to_string(),
                severity: Severity::Error,
                fixes: vec![fix],
            };
        }
    }
    Diagnostic {
        code: "E203".to_string(),
        span: diag_span,
        message: "missing close-brace".to_string(),
        severity: Severity::Error,
        fixes: Vec::new(),
    }
}

/// Locate the `}`-insertion fix for E203: scan the brace body for a
/// de-indented line starting with a known command whose preceding brace
/// content is balanced, and insert `}` at the newline before it.
/// Mirrors the scan in `_detect_missing_brace_at_command`.
fn e203_brace_fix(
    tok: &Token,
    source: &str,
    content_start: u32,
    registry: &CommandRegistry,
) -> Option<CodeFix> {
    let text = token_inner(source, tok)?;
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() < 3 {
        return None;
    }
    // Indentation of the first content line.
    let first_indent = lines[1..]
        .iter()
        .find(|l| !l.trim_start().is_empty())
        .map(|l| l.len() - l.trim_start().len())?;

    let mut cumulative: usize = 0;
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            cumulative += line.len() + 1;
            continue;
        }
        let stripped = line.trim_start();
        if stripped.is_empty() {
            cumulative += line.len() + 1;
            continue;
        }
        let indent = line.len() - stripped.len();
        if indent < first_indent && registry.get(extract_first_word(stripped)).is_some() {
            // The brace content before this line must be balanced —
            // otherwise a single `}` can't recover it.  (The token is
            // already known to contain no `}`, so balance reduces to
            // "no unmatched `{` before this point".)
            let before = &text[..cumulative];
            let opens = before.bytes().filter(|&b| b == b'{').count();
            let closes = before.bytes().filter(|&b| b == b'}').count();
            if opens == closes {
                let newline_idx = cumulative - 1;
                let insert_off = content_start + u32::try_from(newline_idx).unwrap_or(0);
                return Some(CodeFix {
                    span: Span::new(insert_off, insert_off),
                    new_text: "}".to_string(),
                    description: "Insert missing '}' before command".to_string(),
                });
            }
        }
        cumulative += line.len() + 1;
    }
    None
}

/// Content-text index of the end of the content on line `i-1` (its
/// length with trailing whitespace trimmed), counted from the start of
/// the content.  Mirrors the `insert_text_idx` computation.
fn prev_line_content_end(lines: &[&str], i: usize) -> usize {
    let content_end = lines[i - 1].trim_end().len();
    if i == 1 {
        content_end
    } else {
        // Sum lengths of lines 0..=i-2 plus their `\n` separators.
        lines[..i - 1].iter().map(|l| l.len() + 1).sum::<usize>() + content_end
    }
}

/// The first word of a stripped line (up to whitespace / `;` / `{` /
/// `[`).  Mirrors `_extract_first_word`.
fn extract_first_word(stripped: &str) -> &str {
    let end = stripped
        .find([' ', '\t', '\n', '\r', ';', '{', '['])
        .unwrap_or(stripped.len());
    &stripped[..end]
}

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

    fn e201(src: &str) -> Vec<(String, usize)> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == "E201")
            .map(|d| (d.message.clone(), d.fixes.len()))
            .collect()
    }

    /// Absolute byte offsets of every E201 `]`-insertion fix.
    fn e201_fix_offsets(src: &str) -> Vec<u32> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == "E201")
            .flat_map(|d| d.fixes.iter().map(|f| f.span.start()))
            .collect()
    }

    #[test]
    fn e201_fix_vetoed_inside_brace_word() {
        // `[foo {bar\nputs baz}` — the `[` is unterminated, but `puts baz`
        // is *inside* a balanced brace word, not a real command. The
        // command heuristic would propose inserting `]` after `bar`
        // (inside the brace); the structural-index veto rejects that
        // (the `]` would be a literal there), so no wrong fix is offered.
        //
        // Intentional divergence from the Python analyser, which emits
        // the after-`bar` fix — C Tcl 9.0.3 confirms it is wrong
        // (`info complete {set x [foo {bar]}` == 0, incomplete), whereas
        // the end-insert is complete. The veto does the correct
        // full-fidelity thing.
        let src = "set x [foo {bar\nputs baz}\n";
        // E201 still fires — there genuinely is an unterminated `[`.
        assert_eq!(e201(src).len(), 1, "expected one E201: {:?}", e201(src));
        // ...but no fix lands inside the brace word `{bar\nputs baz}`
        // (bytes 11..=24).
        for o in e201_fix_offsets(src) {
            assert!(
                !(11..25).contains(&o),
                "E201 fix at {o} lands inside the brace word",
            );
        }
    }

    #[test]
    fn e201_recovers_real_command_past_brace_word() {
        // `[foo {bar\nputs baz}\nputs done` — `puts baz` is inside the
        // brace word; `puts done` is the real swallowed command. The
        // index-aware command heuristic skips the inert boundary inside
        // the brace word and inserts `]` after `}` (offset 25, before
        // `puts done`), recovering `puts done` as its own command.
        let src = "set x [foo {bar\nputs baz}\nputs done\n";
        let offs = e201_fix_offsets(src);
        assert_eq!(
            offs,
            vec![25],
            "expected the close-bracket after the brace word: {offs:?}"
        );
    }

    #[test]
    fn e201_fix_still_offered_for_plain_text_recovery() {
        // The veto must not suppress legitimate recoveries: a plain
        // unterminated `[` followed by a known command still gets its
        // `]`-insertion fix (the offset is not inert).
        let src = "set x [foo bar\nputs done\n";
        let diags = e201(src);
        assert_eq!(diags, vec![("missing close-bracket".to_string(), 1)]);
    }

    fn lexer_recovery_codes(src: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "E204" | "E205" | "E206"))
            .map(|d| d.code.clone())
            .collect()
    }

    #[test]
    fn e204_e205_e206_from_lexer_warnings() {
        // Matches the live Python analyser.
        assert_eq!(lexer_recovery_codes("set x {abc}def\n"), vec!["E204"]);
        assert_eq!(lexer_recovery_codes("set y \"abc\"def\n"), vec!["E205"]);
        assert_eq!(lexer_recovery_codes("puts ${foo\n"), vec!["E206"]);
        // Well-formed input → none.
        assert!(lexer_recovery_codes("set ok {abc}\n").is_empty());
        assert!(lexer_recovery_codes("set q \"abc\"\n").is_empty());
    }

    #[test]
    fn e201_unterminated_bracket_fallback() {
        // EOF cases → E201, no fix (matches the live Python analyser).
        assert_eq!(
            e201("set x [foo\n"),
            vec![("missing close-bracket".into(), 0)]
        );
        assert_eq!(
            e201("set y [llength $a\n"),
            vec![("missing close-bracket".into(), 0)]
        );
        assert_eq!(e201("set z [\n"), vec![("missing close-bracket".into(), 0)]);
        // A balanced `[foo]` is fine.
        assert!(e201("set ok [foo]\n").is_empty());
    }

    #[test]
    fn empty_command_substitution_is_not_unterminated() {
        // A balanced *empty* `[]` is well-formed — its lexer span covers
        // both brackets, so the closing `]` sits at `span.end() - 1`.
        // It must not raise a spurious E201.
        assert!(e201("set x []\n").is_empty(), "bare empty []");
        assert!(e201("puts [llength []]\n").is_empty(), "nested empty []");
    }

    #[test]
    fn e201_recovery_replaces_e200_for_swallowed_command() {
        // GAP-A1 strip 3c: `set x [foo bar` whose next line is a known
        // command now emits a single E201 (not E200) and analyses
        // `puts done` as a real command.
        let mut a = Analyser::new();
        let r = a.analyse("set x [foo bar\nputs done\n", "tcl8.6");
        let codes: Vec<&str> = r
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "E200" | "E201"))
            .map(|d| d.code.as_str())
            .collect();
        assert_eq!(codes, vec!["E201"], "{codes:?}");
    }

    #[test]
    fn e201_heuristics_attach_a_fix() {
        // A following comment / brace → E201 with a `]`-insertion fix.
        assert_eq!(
            e201("set b [foo\n# comment\n"),
            vec![("missing close-bracket".into(), 1)]
        );
        assert_eq!(
            e201("set c [foo bar {body}\n"),
            vec![("missing close-bracket".into(), 1)]
        );
    }

    #[test]
    fn extract_first_word_stops_at_delimiters() {
        assert_eq!(super::extract_first_word("puts done"), "puts");
        assert_eq!(super::extract_first_word("set{x}"), "set");
        assert_eq!(super::extract_first_word("foo"), "foo");
    }

    #[test]
    fn bracketed_command_substitution_is_not_stray() {
        // A balanced `[llength $x]` is a command substitution, not a
        // stray closer.
        assert!(!codes("set n [llength $x]\n").contains(&"E100".to_string()));
    }

    // -- GAP-A1 E202 / E203 recovery detectors -----------------------
    //
    // Cross-checked against the live Python analyser.

    fn recovery_diags(src: &str, code: &str) -> Vec<(String, usize)> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == code)
            .map(|d| (d.message.clone(), d.fixes.len()))
            .collect()
    }

    fn codes_eq(src: &str, prefix: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with(prefix))
            .map(|d| d.code.clone())
            .collect()
    }

    #[test]
    fn e202_unterminated_quote() {
        // Known command on the next line → E202 with an insert fix.
        assert_eq!(
            recovery_diags("set x \"\nputs hello\n", "E202"),
            vec![("missing \"".to_string(), 1)]
        );
        // No known command → fix-less fallback.
        assert_eq!(
            recovery_diags("set x \"\nblah blah\n", "E202"),
            vec![("missing \"".to_string(), 0)]
        );
        // The unterminated quote emits E202, not the generic E200.
        assert_eq!(codes_eq("set x \"\nputs hello\n", "E20"), vec!["E202"]);
        // A well-formed quoted string is silent.
        assert!(recovery_diags("set x \"hello\"\n", "E202").is_empty());
    }

    #[test]
    fn e203_unterminated_brace() {
        // De-indented known command → E203 with an insert fix.
        assert_eq!(
            recovery_diags("set x {\n    aaa\n    bbb\nputs done\n", "E203"),
            vec![("missing close-brace".to_string(), 1)]
        );
        // No de-indented command → fix-less fallback.
        assert_eq!(
            recovery_diags("set y {\n    aaa\n    bbb\n", "E203"),
            vec![("missing close-brace".to_string(), 0)]
        );
        // E203 replaces the generic E200 for the unterminated brace.
        assert_eq!(
            codes_eq("set x {\n    aaa\n    bbb\nputs done\n", "E20"),
            vec!["E203"]
        );
        // A balanced brace body is silent.
        assert!(recovery_diags("set x {a b c}\n", "E203").is_empty());
    }

    #[test]
    fn e203_fix_lands_before_the_deindented_command() {
        let src = "set x {\n    aaa\n    bbb\nputs done\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let e203 = r.diagnostics.iter().find(|d| d.code == "E203").unwrap();
        let fix = &e203.fixes[0];
        // The `}` is inserted at the newline after `    bbb`.
        let off = fix.span.start() as usize;
        assert_eq!(&src[..off], "set x {\n    aaa\n    bbb");
        assert_eq!(fix.new_text, "}");
    }

    #[test]
    fn stray_close_bracket_fires_inside_proc_body() {
        // GAP-A6 follow-up: the E100 stray-`]` check runs on every
        // analysed body, not just the top level.  The live Python
        // analyser fires E100 at the `puts foo]` line inside the
        // proc body (`check_unmatched_close_bracket` is a
        // `_UNIVERSAL_CHECKS` member, run on every command).
        let mut a = Analyser::new();
        let r = a.analyse("proc p {} {\n  puts foo]\n}\n", "tcl8.6");
        let e100: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "E100").collect();
        assert_eq!(
            e100.len(),
            1,
            "expected one E100 in the body, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| (d.code.clone(), d.span))
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn body_check_does_not_false_positive_on_literals() {
        // Quoted `]` / `}` and balanced `[…]` inside a body are
        // literals / well-formed substitutions — none fires, matching
        // the live Python analyser.
        let mut a = Analyser::new();
        for src in [
            "proc p {} {\n  puts \"a ]\"\n}\n",
            "proc p {} {\n  set x [llength $l]\n}\n",
            "proc p {} {\n  puts \"a }\"\n}\n",
        ] {
            let r = a.analyse(src, "tcl8.6");
            let n = r
                .diagnostics
                .iter()
                .filter(|d| matches!(d.code.as_str(), "E100" | "E102"))
                .count();
            assert_eq!(n, 0, "unexpected stray-closer diag for {src:?}");
        }
    }

    #[test]
    fn stray_close_brace_fires_inside_nested_body() {
        // A bare `}` line inside a nested (control-flow) body must
        // also surface E102, matching the per-body universal check.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc p {} {\n  if {1} {\n    set x 1\n  }\n  puts foo]\n}\n",
            "tcl8.6",
        );
        assert_eq!(r.diagnostics.iter().filter(|d| d.code == "E100").count(), 1,);
    }
}
