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

//! E100 / E102 stray-closer syntax checks.
//!
//! Detects an unmatched close bracket (E100) and an unmatched close
//! brace (E102), plus their bracket-insertion-point and stray-brace
//! quick-fixes.
//!
//! These are *targeted token checks* — a bare `]` outside command
//! substitution almost always means a missing `[`, and a bare `}`
//! outside a brace word means a missing `{`.  They are distinct from
//! the parser-recovery path (which repairs *unclosed openers*): a stray
//! `]` / `}` produces no recovery diagnostic today, so these emitters
//! add genuinely-missing coverage and never double-report.
//!
//! A `]` / `}` inside a double-quoted string (`puts "a ]"`) is a literal
//! character and must not fire; quoted context is classified by
//! `classify_quoted_contexts`.

use std::collections::HashSet;
use tcl_core_types::DiagCode;

use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::CommandRegistry;

use super::types::{CodeFix, Diagnostic, Severity};
use crate::segmenter::SegmentedCommand;

/// E201: a `[` command substitution with no closing `]`.  Emits the
/// user-facing E201 diagnostic (the ghost-token re-lex that would produce
/// a clean command stream is not done here).  For each unterminated `Cmd`
/// token it picks, in priority order, where the `]` belongs: before a `#`
/// comment line, a known-command line, or a `{`; otherwise it anchors at
/// the `[`.
pub(crate) fn unterminated_bracket_diagnostics(
    cmd: &SegmentedCommand,
    source: &str,
    known: &super::utils::RecoveryKnownCommands,
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
        out.push(detect_e201(content, content_start, bracket_off, known));
    }
    out
}

/// True when `tok` (a `Cmd` token) has no closing `]`.
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
    known: &super::utils::RecoveryKnownCommands,
) -> Diagnostic {
    // The heuristics *propose* an insertion offset (semantic); the
    // structural index *validates* it (syntactic). A proposed `]` that
    // lands inside an inert span — a
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
    let (cmd_diag, swallowed_known_command) =
        e201_at_command(content, content_start, bracket_off, known, &index);
    if let Some(d) = cmd_diag.and_then(&accept) {
        return d;
    }
    // A known command was swallowed into a brace word inside the bracket
    // (e.g. `proc …` inside `[foo {bar\nproc …}`). The `e201_at_brace`
    // fallback would insert `]` before the `{`, folding the swallowed
    // command into a brace-word argument and hiding it from analysis.
    // Bail to the fix-less fallback instead: with no ghost `]`, the
    // scan-to-next recovery's partial command stands, the unterminated
    // `[` is still flagged, and the tail is analysed as real code.
    if !swallowed_known_command
        && let Some(d) = e201_at_brace(content, content_start, bracket_off).and_then(&accept)
    {
        return d;
    }
    // Fallback: highlight just the opening `[`, no fix.
    crate::analyser::types::Diagnostic::new(
        DiagCode::E201,
        Span::new(bracket_off, bracket_off),
        "missing close-bracket".to_string(),
        Severity::Error,
    )
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
    crate::analyser::types::Diagnostic::new(
        DiagCode::E201,
        Span::new(bracket_off, diag_end.max(bracket_off)),
        "missing close-bracket".to_string(),
        Severity::Error,
    )
    .with_fixes(vec![CodeFix {
        span: Span::new(insert_off, insert_off),
        new_text: "]".to_string(),
        description: fix_desc.to_string(),
        // E201: a close-bracket recovery heuristic.
        safety: crate::irules_checks::FixSafety::RequiresReview,
    }])
}

/// E201 heuristic: a `#` comment line follows — insert `]` at the end of
/// the previous line's content.
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
/// of the previous line.
///
/// Returns `(diagnostic, swallowed_known_command)`. The second field is
/// `true` when a *known-command* line was found but its `]`-insertion
/// boundary sits inside an inert span (a brace word / quoted run swallowed
/// it) and no later non-inert command line was reached — i.e. the bracket's
/// content captured real commands inside a brace word. `detect_e201` uses
/// that to suppress the `e201_at_brace` fallback, which would otherwise
/// insert `]` before the `{` and hide the swallowed command from analysis.
fn e201_at_command(
    content: &str,
    content_start: u32,
    bracket_off: u32,
    known: &super::utils::RecoveryKnownCommands,
    index: &tcl_lexer::BracketIndex,
) -> (Option<Diagnostic>, bool) {
    let lines: Vec<&str> = content.split('\n').collect();
    if lines.len() < 2 {
        return (None, false);
    }
    let mut swallowed_known_command = false;
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let stripped = line.trim_start();
        if stripped.is_empty() {
            continue;
        }
        let insert_idx = prev_line_content_end(&lines, i);
        let first_word = extract_first_word(stripped);
        let is_known = known.contains(first_word);
        // The structural index validates the boundary: if it sits inside
        // an inert span (a brace word / quoted run that swallowed the
        // line, e.g. `puts baz` inside `{bar\nputs baz}`), the candidate
        // `]` would be a literal — keep scanning for the next real command
        // past the inert span rather than giving up.
        // Remember that a *known* command was swallowed so the caller can
        // skip the brace-break fallback rather than paper over it.
        if index.is_inert(u32::try_from(insert_idx).unwrap_or(0)) {
            if is_known {
                swallowed_known_command = true;
            }
            continue;
        }
        if is_known {
            return (
                Some(e201_with_insert(
                    content_start,
                    bracket_off,
                    insert_idx,
                    "Insert missing ']' before command",
                )),
                swallowed_known_command,
            );
        }
        break;
    }
    (None, swallowed_known_command)
}

/// E201 heuristic: a `{` swallowed the rest — insert `]` before it
/// (after trailing whitespace).
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

/// E202 / E203: unterminated `"` / `{` recovery diagnostics.
///
/// For each genuinely unterminated quote / brace token in the command it
/// emits one diagnostic: the known-command heuristic with a
/// closing-delimiter insertion fix when one can be located, else the
/// fix-less fallback.  Returns an empty vector for well-formed input.
/// `region_end` is the absolute byte offset at which the analysed region
/// ends — `source.len()` at the top level, but `base_offset + body_len`
/// when scanning a nested body whose tokens are absolute spans into the
/// full `source`. The "reaches EOF" / "no closing delimiter" tests are
/// relative to that end.
pub(crate) fn unterminated_delimiter_diagnostics(
    cmd: &SegmentedCommand,
    source: &str,
    region_end: usize,
    registry: Option<&CommandRegistry>,
    known: &super::utils::RecoveryKnownCommands,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (idx, tok) in cmd.all_tokens.iter().enumerate() {
        if is_suspicious_quote(idx, cmd, source, region_end) {
            out.push(detect_e202(tok, source, known));
        } else if is_suspicious_str(tok, source, region_end) {
            out.push(detect_e203(tok, cmd, source, registry, known));
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

/// `true` when `tok`'s delimiter genuinely closes within `region_end`: a
/// non-empty word closed by `closer` right at `tok.span.end()`, or the
/// empty `{}` / `""` special case where the lexer's inner-end span
/// convention already covers the closer (content length exactly one byte —
/// the closer itself — mirroring [`is_unterminated_cmd`]'s empty-`[]`
/// handling for E201). Shared by [`is_suspicious_quote`] /
/// [`is_suspicious_str`] so E202 / E203 detection doesn't need its own
/// copy of the empty-delimiter special case.
fn is_closed_within_region(tok: &Token, source: &str, region_end: usize, closer: u8) -> bool {
    let bytes = source.as_bytes();
    let end = tok.span.end() as usize;
    if end < region_end && bytes.get(end) == Some(&closer) {
        return true;
    }
    let content_start = tok.span.start() as usize + tok.content_offset as usize;
    end == content_start + 1 && end > 0 && bytes.get(end - 1) == Some(&closer)
}

/// True when `cmd.all_tokens[idx]` is an `Esc` from an unterminated `"`
/// that swallows the rest of the region: the token starts at a `"` and
/// never closes before `region_end`. Not gated on line count or content
/// shape — a single-line `set x "hello` (no newline at all) is just as
/// unterminated as a multi-line run, and must be flagged identically; the
/// only thing that distinguishes "genuinely unterminated" from "closed" is
/// whether a closing `"` actually appears (see [`is_closed_within_region`]).
fn is_suspicious_quote(
    idx: usize,
    cmd: &SegmentedCommand,
    source: &str,
    region_end: usize,
) -> bool {
    let tok = &cmd.all_tokens[idx];
    if tok.kind != TokenType::Esc {
        return false;
    }
    if source.as_bytes().get(tok.span.start() as usize) != Some(&b'"') {
        return false;
    }
    // A quoted word containing a `$var`/`[cmd]` substitution is segmented
    // into several sibling `Esc`/`Var` fragments, and the fragment right
    // after the substitution starts with the closing `"` itself — the
    // *closer*, not a new opener, even though its own first byte matches.
    // A fragment continuing an already-open quote is never itself an
    // opener, so it's excluded here rather than independently re-flagged.
    if idx > 0 && cmd.all_tokens[idx - 1].in_quote {
        return false;
    }
    if is_closed_within_region(tok, source, region_end, b'"') {
        return false;
    }
    // A multi-fragment quoted word closes the moment the fragment chain
    // started by this opener steps out of quote state; only a chain that
    // stays in quote all the way to the region boundary (or the command's
    // last token) is genuinely unterminated.
    let mut i = idx;
    loop {
        if cmd.all_tokens[i].span.start() as usize >= region_end {
            return true;
        }
        if !cmd.all_tokens[i].in_quote {
            return false;
        }
        if i + 1 >= cmd.all_tokens.len() {
            return true;
        }
        i += 1;
    }
}

/// True when `tok` is a `Str` from an unterminated `{` with no closing
/// `}`: a token text containing `}` is E103 territory (brace closed at the
/// wrong nesting level), not a truly missing brace. Not gated on line
/// count — see [`is_suspicious_quote`]'s doc for why a short, single-line
/// unterminated brace must be flagged exactly like a long multi-line one.
fn is_suspicious_str(tok: &Token, source: &str, region_end: usize) -> bool {
    if tok.kind != TokenType::Str {
        return false;
    }
    if source.as_bytes().get(tok.span.start() as usize) != Some(&b'{') {
        return false;
    }
    if is_closed_within_region(tok, source, region_end, b'}') {
        return false;
    }
    let Some(text) = token_inner(source, tok) else {
        return false;
    };
    !text.contains('}')
}

/// Build the E202 diagnostic for an unterminated `"`: the known-command
/// heuristic (insert `"` right after the opener) or the fix-less
/// fallback.
fn detect_e202(
    tok: &Token,
    source: &str,
    known: &super::utils::RecoveryKnownCommands,
) -> Diagnostic {
    let quote_off = tok.span.start();
    let diag_span = Span::new(quote_off, quote_off);
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
            if known.contains(extract_first_word(stripped)) {
                // Virtual `"` right after the opening `"`.
                let insert_off = quote_off + 1;
                return crate::analyser::types::Diagnostic::new(
                    DiagCode::E202,
                    diag_span,
                    "missing \"".to_string(),
                    Severity::Error,
                )
                .with_fixes(vec![CodeFix {
                    span: Span::new(insert_off, insert_off),
                    new_text: "\"".to_string(),
                    description: "Insert missing '\"' to close string".to_string(),
                    // E202: a close-quote recovery heuristic.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                }]);
            }
            // First non-blank line isn't a known command — stop.
            break;
        }
    }
    crate::analyser::types::Diagnostic::new(
        DiagCode::E202,
        diag_span,
        "missing \"".to_string(),
        Severity::Error,
    )
}

/// Build the E203 diagnostic for an unterminated `{`: the de-indented
/// known-command heuristic (insert `}` at the newline before that line)
/// or the fix-less fallback.
fn detect_e203(
    tok: &Token,
    cmd: &SegmentedCommand,
    source: &str,
    registry: Option<&CommandRegistry>,
    known: &super::utils::RecoveryKnownCommands,
) -> Diagnostic {
    let brace_off = tok.span.start();
    let diag_span = Span::new(brace_off, brace_off);
    let content_start = brace_off + u32::from(tok.content_offset);
    // `ArgRole` routing: when the unterminated `{` is an
    // **expression** argument (`if {…`, `while {…`, `expr {…`,
    // `for`'s condition, …) a following line that starts with a known
    // command is a strong forgotten-close signal *even without a
    // de-indent* — so EXPR braces recover with the aggressive
    // command-break, unlike BODY / data which keep the conservative
    // de-indent heuristic. Role never *suppresses* recovery, only
    // makes it more eager. The role lookup needs the real registry (for
    // `arg_indices_for_role`); the fix search itself only needs the
    // known-command-name universe.
    let expr_role = registry.is_some_and(|reg| unterminated_arg_is_expr(tok, cmd, reg));
    if let Some(fix) = e203_brace_fix(tok, source, content_start, known, expr_role) {
        return crate::analyser::types::Diagnostic::new(
            DiagCode::E203,
            diag_span,
            "missing close-brace".to_string(),
            Severity::Error,
        )
        .with_fixes(vec![fix]);
    }
    crate::analyser::types::Diagnostic::new(
        DiagCode::E203,
        diag_span,
        "missing close-brace".to_string(),
        Severity::Error,
    )
}

/// `true` when the unterminated brace-word token `tok` sits in an
/// expression-role argument of its command (`if`/`while`/`for` condition,
/// `expr`, …). Resolves the token's argv index, then queries the registry
/// for the command's `ArgRole::Expr` argument indices.
fn unterminated_arg_is_expr(
    tok: &Token,
    cmd: &SegmentedCommand,
    registry: &CommandRegistry,
) -> bool {
    // Find the argv word whose representative token is this brace word.
    let Some(word_idx) = cmd
        .argv
        .iter()
        .position(|w| w.span.start() == tok.span.start())
    else {
        return false;
    };
    if word_idx == 0 || cmd.texts.is_empty() {
        return false; // the command name itself is never an expr arg
    }
    let name = cmd.texts[0].as_str();
    let args: Vec<&str> = cmd.texts[1..].iter().map(String::as_str).collect();
    // `arg_indices_for_role` indexes into `args` (argv[1..]); the token's
    // arg index is therefore `word_idx - 1`.
    registry
        .arg_indices_for_role(name, &args, tcl_registry::arg_role::ArgRole::Expr)
        .contains(&(word_idx - 1))
}

/// Locate the `}`-insertion fix for E203: scan the brace body for a line
/// starting with a known command whose preceding brace content is
/// balanced, and insert `}` at the newline before it. For BODY / data
/// braces the line must be **de-indented** (the conservative heuristic);
/// for `expr_role` braces any following known-command line qualifies (the
/// aggressive command-break via `ArgRole` routing).
fn e203_brace_fix(
    tok: &Token,
    source: &str,
    content_start: u32,
    known: &super::utils::RecoveryKnownCommands,
    expr_role: bool,
) -> Option<CodeFix> {
    let text = token_inner(source, tok)?;
    let lines: Vec<&str> = text.split('\n').collect();
    // The de-indent heuristic needs a line *after* the first content
    // line; the aggressive expr heuristic only needs a following line.
    if lines.len() < if expr_role { 2 } else { 3 } {
        return None;
    }
    // Indentation of the first content line (only the de-indent path
    // consults it; default to 0 so the expr path is unaffected).
    let first_indent = lines[1..]
        .iter()
        .find(|l| !l.trim_start().is_empty())
        .map_or(0, |l| l.len() - l.trim_start().len());

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
        if (expr_role || indent < first_indent) && known.contains(extract_first_word(stripped)) {
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
                    // E203: a close-brace recovery heuristic.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                });
            }
        }
        cumulative += line.len() + 1;
    }
    None
}

/// Content-text index of the end of the content on line `i-1` (its
/// length with trailing whitespace trimmed), counted from the start of
/// the content.
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
/// `[`).
fn extract_first_word(stripped: &str) -> &str {
    let end = stripped
        .find([' ', '\t', '\n', '\r', ';', '{', '['])
        .unwrap_or(stripped.len());
    &stripped[..end]
}

/// Scan one command's token stream for stray `]` (E100) / `}` (E102)
/// closers, returning the diagnostics (with quick-fixes where one can
/// be derived).
///
/// `extra_known` lazily builds the set of user-declared command-like
/// names seen so far in the walk (proc / class / alias / ensemble
/// tails, tclOO instance commands, `unknown`-dispatch targets, inline
/// stubs, …, see [`super::recovery::Analyser::user_command_tail_names`])
/// — consulted alongside the registry so the E100 bracket-insertion
/// heuristic recognises a call to an already-defined local command, not
/// just a registry builtin. It is a closure rather than a pre-built set
/// because this function runs on *every* command in the document while
/// a stray closer is rare: building the set (a source-wide stub scan
/// plus several map traversals) is only worth paying when a `]` is
/// actually found, not on every clean command.
pub(crate) fn stray_closer_diagnostics(
    cmd: &SegmentedCommand,
    source: &str,
    registry: Option<&CommandRegistry>,
    extra_known: impl Fn() -> HashSet<String>,
) -> Vec<Diagnostic> {
    let tokens = &cmd.all_tokens;
    let in_quoted = classify_quoted_contexts(tokens);
    let mut out: Vec<Diagnostic> = Vec::new();
    let mut cached_extra_known: Option<HashSet<String>> = None;

    for (idx, tok) in tokens.iter().enumerate() {
        if tok.kind != TokenType::Esc || in_quoted.get(idx).copied().unwrap_or(false) {
            continue;
        }
        let Some(text) = token_text(source, tok) else {
            continue;
        };

        // E102: the first unescaped `}` anywhere in the token text (not
        // just a token that is *only* `}` — `foo}bar` is just as stray
        // as a lone `}`, matching the E100 compound-token scan below).
        if let Some(rel) = first_unescaped_delim(text, b'}') {
            out.push(make_e102(tok, rel, source));
        }

        // E100: the first unescaped `]` in the token text.
        if let Some(rel) = first_unescaped_delim(text, b']') {
            let known = cached_extra_known.get_or_insert_with(&extra_known);
            out.push(make_e100(cmd, tokens, idx, rel, source, registry, known));
        }
    }
    out
}

/// Mark each token that lies inside a double-quoted word: a
/// self-contained quoted ESC (`"}"`) has a content shift > 0; a leading /
/// trailing quoted part is recognised via the cross-token `in_quote`
/// flag.  Separators reset the tracker.
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

/// Byte index of the first unescaped `delim` in `text`, or `None`.
///
/// A `delim` is escaped only when it is preceded by an *odd* run of
/// backslashes — Tcl's `\\` is a literal backslash that consumes both
/// bytes, so the next character is unaffected by it: `\]` escapes the
/// bracket (odd run, length 1) but `\\]` does not (even run, length 2 —
/// the pair collapses to one literal `\`, leaving the `]` bare).  A
/// naive "preceded by exactly one `\`" check gets the even case wrong.
fn first_unescaped_delim(text: &str, delim: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == delim {
            let mut backslashes = 0;
            while backslashes < i && bytes[i - 1 - backslashes] == b'\\' {
                backslashes += 1;
            }
            if backslashes % 2 == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Locate the first *trailing* stray, unescaped `]` in a non-quoted
/// `Esc` token of `tokens` — the `]` must be the last byte of its token
/// text (`foo]`, not `foo]bar`), since only a trailing bracket fits the
/// `recover_stray_close_bracket` merge-repair shape (a mid-word bracket
/// has trailing text that wouldn't fit inside `[...]`; that shape is
/// still diagnosed by E100, just not auto-repaired). Returns
/// `(token_index, byte_index_within_token_text)`.
///
/// Built on the same escape/quote primitives as the E100 diagnostic
/// scan in [`stray_closer_diagnostics`] (`classify_quoted_contexts`,
/// `first_unescaped_delim`) so the two can never disagree about what
/// counts as a stray bracket — a prior version kept an independent,
/// escape-unaware detector here and it drifted, letting the repair fire
/// (and corrupt downstream command-invocation recording) for brackets
/// the diagnostic correctly treated as escaped.
pub(crate) fn find_first_stray_bracket(tokens: &[Token], source: &str) -> Option<(usize, usize)> {
    let in_quoted = classify_quoted_contexts(tokens);
    for (idx, tok) in tokens.iter().enumerate() {
        if tok.kind != TokenType::Esc || in_quoted.get(idx).copied().unwrap_or(false) {
            continue;
        }
        let Some(text) = token_text(source, tok) else {
            continue;
        };
        if let Some(rel) = first_unescaped_delim(text, b']')
            && rel + 1 == text.len()
        {
            return Some((idx, rel));
        }
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
    extra_known: &HashSet<String>,
) -> Diagnostic {
    let bracket_off = tokens[bracket_idx].span.start() + u32::try_from(rel).unwrap_or(0);
    // The span end is exclusive (`span_to_range` maps it straight to an
    // LSP position with no adjustment), so it must sit one byte past the
    // `]` itself — otherwise the highlighted range never covers the very
    // character the diagnostic is about.
    let bracket_end = bracket_off + 1;
    let insert = registry.and_then(|reg| {
        find_bracket_insertion_point(
            cmd,
            tokens,
            bracket_idx,
            bracket_off,
            source,
            reg,
            extra_known,
        )
    });

    let mut fixes: Vec<CodeFix> = Vec::new();
    // When no insertion point can be inferred, keep the highlight tight
    // around the stray `]` itself rather than the whole command — a
    // fix-less diagnostic still needs to point precisely at the problem.
    let diag_start = if let Some(off) = insert {
        // Zero-width insertion of `[` at `off`.
        fixes.push(CodeFix {
            span: Span::new(off, off),
            new_text: "[".to_string(),
            description: "Insert missing '['".to_string(),
            // E204: an open-bracket recovery heuristic.
            safety: crate::irules_checks::FixSafety::RequiresReview,
        });
        off
    } else {
        bracket_off
    };

    crate::analyser::types::Diagnostic::new(
        DiagCode::E100,
        Span::new(diag_start.min(bracket_off), bracket_end),
        "Unmatched ']' \u{2014} missing opening '['?".to_string(),
        Severity::Error,
    )
    .with_fixes(fixes)
}

/// Find where the missing `[` should go.  Heuristics, in order: a
/// command name in the text before the `]`; a backward scan for a
/// known command-name ESC token; an arity overflow on the enclosing
/// command.
///
/// `extra_known` widens "known command name" beyond the registry to
/// user-declared names already seen in the walk (procs, tclOO classes /
/// instance commands, aliases, ensembles, `unknown`-dispatch targets, …)
/// — a call to an already-defined local proc missing its `[` is just as
/// recoverable as a call to a registry builtin.
pub(crate) fn find_bracket_insertion_point(
    cmd: &SegmentedCommand,
    tokens: &[Token],
    bracket_idx: usize,
    bracket_off: u32,
    source: &str,
    registry: &CommandRegistry,
    extra_known: &HashSet<String>,
) -> Option<u32> {
    let known: HashSet<&str> = registry
        .command_names()
        .chain(extra_known.iter().map(String::as_str))
        .collect();
    let tok = &tokens[bracket_idx];
    let text = token_text(source, tok)?;

    // 1a: the text before `]` in the same token is a command name.
    if let Some(bidx) = text.find(']')
        && bidx > 0
        && known.contains(&text[..bidx])
    {
        return Some(tok.span.start());
    }
    // 1b: backward scan (skip the command word at index 0) for a known
    // command-name ESC token.
    for i in (1..bracket_idx).rev() {
        let t = &tokens[i];
        if t.kind == TokenType::Esc
            && let Some(name) = token_text(source, t)
            && known.contains(name)
        {
            return Some(t.span.start());
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

/// Build the E102 diagnostic for a `}` at byte `rel` within token `tok`,
/// attaching the stray-brace removal fix when the `}` owns its line —
/// `tok` may be a compound word (`foo}bar`), so the highlighted span
/// covers only the `}` character itself, not the whole token.
fn make_e102(tok: &Token, rel: usize, source: &str) -> Diagnostic {
    let brace_off = tok.span.start() + u32::try_from(rel).unwrap_or(0);
    let fixes = stray_brace_fix(tok, brace_off, source)
        .into_iter()
        .collect();
    crate::analyser::types::Diagnostic::new(
        DiagCode::E102,
        Span::new(brace_off, brace_off + 1),
        "Unmatched '}' \u{2014} missing opening '{'?".to_string(),
        Severity::Error,
    )
    .with_fixes(fixes)
}

/// Build a fix that deletes a stray `}` and its line, when the line
/// holds nothing but optional whitespace and the brace (the `}` is not
/// embedded in a larger word).
fn stray_brace_fix(tok: &Token, brace_off: u32, source: &str) -> Option<CodeFix> {
    let start_off = brace_off as usize;
    let end_off = start_off + 1;
    if tok.span.end() as usize != end_off || end_off > source.len() {
        // Only a `}` that is its token's last byte can plausibly be the
        // whole line's content; an embedded `foo}bar` never qualifies.
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
        // E205: deleting a stray `}` assumes it is the extra one rather
        // than a body opener being unclosed earlier.
        safety: crate::irules_checks::FixSafety::RequiresReview,
    })
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;
    use tcl_core_types::DiagCode;

    fn codes(src: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect()
    }

    #[test]
    fn stray_close_bracket_emits_e100() {
        // `puts foo]` → E100.
        assert!(codes("puts foo]\n").contains(&"E100".to_string()));
        // `]` after a `$var` word also fires.
        assert!(codes("set x $y]\n").contains(&"E100".to_string()));
    }

    #[test]
    fn stray_close_brace_emits_e102() {
        // `}` on its own line → E102 (alongside the W123 unknown-command
        // also emitted for the bare `}`).
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
            .filter(|d| d.code == DiagCode::E201)
            .map(|d| (d.message.clone(), d.fixes.len()))
            .collect()
    }

    /// Absolute byte offsets of every E201 `]`-insertion fix.
    fn e201_fix_offsets(src: &str) -> Vec<u32> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E201)
            .flat_map(|d| d.fixes.iter().map(|f| f.span.start()))
            .collect()
    }

    #[test]
    fn e201_brace_swallowed_command_falls_back_to_e200() {
        // `[foo {bar\nputs baz}` — the `[` is unterminated and `puts baz` is
        // *inside* a balanced brace word, not a real command. The command
        // heuristic's after-`bar` boundary is inert (the `]` would be a
        // literal), and the brace-break fallback (insert `]` before the `{`)
        // would fold the whole brace word — including the swallowed command —
        // into an argument, hiding it from analysis. So the bracket recovery
        // bails: no ghost `]` is inserted, the scan-to-next partial command
        // stands, and the unterminated `[` is flagged with the generic E200
        // (matching C Tcl 9.0.3, for which
        // `info complete {set x [foo {bar]}` == 0 — the after-`bar` insert is
        // incomplete, so no fix is the honest answer).
        let src = "set x [foo {bar\nputs baz}\n";
        // The unterminated `[` is still flagged — as E200, not a wrong-fix
        // E201.
        assert!(
            codes(src).contains(&"E200".to_string()),
            "expected E200 for the unterminated bracket: {:?}",
            codes(src)
        );
        // No E201 fix is offered, and certainly none inside the brace word
        // `{bar\nputs baz}` (bytes 11..=24).
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
            .map(|d| d.code.to_string())
            .collect()
    }

    #[test]
    fn e204_e205_e206_from_lexer_warnings() {
        assert_eq!(lexer_recovery_codes("set x {abc}def\n"), vec!["E204"]);
        assert_eq!(lexer_recovery_codes("set y \"abc\"def\n"), vec!["E205"]);
        assert_eq!(lexer_recovery_codes("puts ${foo\n"), vec!["E206"]);
        // Well-formed input → none.
        assert!(lexer_recovery_codes("set ok {abc}\n").is_empty());
        assert!(lexer_recovery_codes("set q \"abc\"\n").is_empty());
    }

    #[test]
    fn e201_unterminated_bracket_fallback() {
        // EOF cases → E201, no fix.
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
        // `set x [foo bar` whose next line is a known command emits a
        // single E201 (not E200) and analyses `puts done` as a real
        // command.
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

    // E202 / E203 recovery detectors

    fn recovery_diags(src: &str, code: &str) -> Vec<(String, usize)> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str() == code)
            .map(|d| (d.message.clone(), d.fixes.len()))
            .collect()
    }

    fn codes_eq(src: &str, prefix: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str().starts_with(prefix))
            .map(|d| d.code.to_string())
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
    fn e202_fires_inside_a_nested_body() {
        // The proc's brace word is balanced, so the body is re-segmented and
        // analysed; a run-away `"` inside it must still surface E202 — not
        // only at the top level.
        let src = "proc p {} {\n    set x \"\n    puts hello\n}\n";
        assert_eq!(
            recovery_diags(src, "E202"),
            vec![("missing \"".to_string(), 1)]
        );
        // The detector reaches arbitrarily deep — here the quote is two
        // bodies down (`proc` body → `if` body).
        let deep = "proc p {} {\n  if {1} {\n    set x \"\n    puts hi\n  }\n}\n";
        assert_eq!(
            recovery_diags(deep, "E202"),
            vec![("missing \"".to_string(), 1)]
        );
        // A well-formed quoted string inside a body stays silent.
        assert!(recovery_diags("proc p {} {\n    set x \"hello\"\n}\n", "E202").is_empty());
    }

    #[test]
    fn e202_not_emitted_for_closed_multiline_quote_at_body_end() {
        // A *closed* multi-line quoted word whose closing `"` is the body's
        // last byte — `proc p {} {set x "\nhello"}` — must not be flagged:
        // the command "reaches EOF" only because the body ends there, but the
        // quote is terminated (`info complete` == 1 in tclsh 8.6/9.0).
        // Regression guard for the body scan.
        assert!(recovery_diags("proc p {} {set x \"\nhello\"}\n", "E202").is_empty());
        // The unterminated counterpart in the same shape still fires.
        assert_eq!(
            recovery_diags("proc p {} {set x \"\nhello\n}\n", "E202"),
            vec![("missing \"".to_string(), 0)],
        );
    }

    #[test]
    fn e203_fix_lands_before_the_deindented_command() {
        let src = "set x {\n    aaa\n    bbb\nputs done\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let e203 = r
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::E203)
            .unwrap();
        let fix = &e203.fixes[0];
        // The `}` is inserted at the newline after `    bbb`.
        let off = fix.span.start() as usize;
        assert_eq!(&src[..off], "set x {\n    aaa\n    bbb");
        assert_eq!(fix.new_text, "}");
    }

    fn e203_fix_offsets(src: &str) -> Vec<u32> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E203)
            .flat_map(|d| d.fixes.iter().map(|f| f.span.start()))
            .collect()
    }

    #[test]
    fn e203_expr_brace_recovers_without_deindent() {
        // `ArgRole` routing: an unterminated `if` *condition* (EXPR role)
        // recovers on the next known-command line even though it is not
        // de-indented — the aggressive command break. The `}` lands right
        // after `$x == 1` (offset 11), so `puts hi` is recovered as its
        // own command.
        let src = "if {$x == 1\nputs hi\n";
        assert_eq!(
            e203_fix_offsets(src),
            vec![11],
            "{:?}",
            e203_fix_offsets(src)
        );
        // The repaired source parses `puts hi` as a separate command.
        let repaired = format!("{}}}{}", &src[..11], &src[11..]);
        assert_eq!(repaired, "if {$x == 1}\nputs hi\n");
    }

    #[test]
    fn e203_data_brace_keeps_conservative_deindent() {
        // A non-EXPR (data) brace must NOT recover aggressively: a
        // *non-de-indented* following known command does not trigger a
        // fix (only the conservative de-indent heuristic applies), so the
        // recovery behaviour is unchanged for data braces.
        // `set x {` — arg 1 of `set` is data, not expr.
        let src = "set x {\nputs hi\n";
        assert!(
            e203_fix_offsets(src).is_empty(),
            "data brace should not aggressively recover: {:?}",
            e203_fix_offsets(src),
        );
        // But a *de-indented* known command still recovers (unchanged).
        let deindented = "set x {\n    aaa\nputs done\n";
        assert_eq!(e203_fix_offsets(deindented).len(), 1);
    }

    #[test]
    fn stray_close_bracket_fires_inside_proc_body() {
        // The E100 stray-`]` check runs on every analysed body, not just
        // the top level — it is a universal check run on every command, so
        // it fires at the `puts foo]` line inside the proc body.
        let mut a = Analyser::new();
        let r = a.analyse("proc p {} {\n  puts foo]\n}\n", "tcl8.6");
        let e100: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E100)
            .collect();
        assert_eq!(
            e100.len(),
            1,
            "expected one E100 in the body, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| (d.code.to_string(), d.span))
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn body_check_does_not_false_positive_on_literals() {
        // Quoted `]` / `}` and balanced `[…]` inside a body are
        // literals / well-formed substitutions — none fires.
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
        assert_eq!(
            r.diagnostics
                .iter()
                .filter(|d| d.code == DiagCode::E100)
                .count(),
            1,
        );
    }

    // E200/E201/E202/E203 deep-review regression suite — TP/FP/TN/FN cases
    // for the "known command" generality fix (procs/classes/aliases as
    // recovery signals), the E202/E203 line-count false-negative fix, and
    // the E200 fallback's tight-highlighting fix. Each case is checked
    // against the C-Tcl `info complete` oracle in the comment (verified
    // manually against tclsh8.6/tclsh9.0; see the review notes).

    fn e20x_span(src: &str, code: &str) -> Option<(u32, u32)> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .find(|d| d.code.as_str() == code)
            .map(|d| (d.span.start(), d.span.end()))
    }

    // --- Known-command generality: user procs / classes / aliases ---------

    #[test]
    fn tp_e201_fix_recognises_user_defined_proc() {
        // True positive: a `[` breaks before a call to a proc the document
        // itself defines (not a registry builtin) — the fix must still be
        // found, matching the same shape that already works for `puts`.
        let src = "proc my_helper {x} {puts $x}\n\nset y [foo bar\nmy_helper hi\n";
        let diags = recovery_diags(src, "E201");
        assert_eq!(
            diags,
            vec![("missing close-bracket".to_string(), 1)],
            "user proc call should be recognised as a recovery signal"
        );
    }

    #[test]
    fn tp_e203_fix_recognises_user_defined_class() {
        // A `oo::class create` name is a document-local "known command" via
        // the same signature-scan the fix draws on.
        let src = "oo::class create Widget {\n  method draw {} {}\n}\n\nset q {\n    aaa\nWidget create obj1\n";
        let diags = recovery_diags(src, "E203");
        assert_eq!(diags, vec![("missing close-brace".to_string(), 1)]);
    }

    #[test]
    fn tp_e203_fix_recognises_interp_alias() {
        let src = "interp alias {} greet {} puts hi\n\nset q {\n    aaa\ngreet\n";
        let diags = recovery_diags(src, "E203");
        assert_eq!(diags, vec![("missing close-brace".to_string(), 1)]);
    }

    #[test]
    fn tp_e203_fix_recognises_namespace_qualified_proc() {
        // Both the qualified form (`myns::helper`) and the bare tail
        // (`helper`) resolve against a namespace-nested proc definition.
        let src = "namespace eval myns {\n  proc helper {x} {return $x}\n}\n\nset q {\n  aaa\nmyns::helper 1\n";
        let diags = recovery_diags(src, "E203");
        assert_eq!(diags, vec![("missing close-brace".to_string(), 1)]);
    }

    #[test]
    fn tn_e203_does_not_recognise_undefined_identifier() {
        // False-negative guard in the other direction: a genuine typo /
        // undefined name must NOT be treated as a recovery signal — the
        // fix-less fallback is the honest answer when nothing resolves.
        let src = "set q {\n    aaa\nthis_is_not_a_real_command\n";
        let diags = recovery_diags(src, "E203");
        assert_eq!(diags, vec![("missing close-brace".to_string(), 0)]);
    }

    #[test]
    fn tp_scan_to_next_recovers_document_analysis_past_user_proc() {
        // The segmenter-level scan-to-next recovery (not just the E20x fix)
        // must also recognise a user proc: previously, when nothing but
        // user-defined calls followed a break, the whole tail was silently
        // dropped from analysis (no E002 for a genuinely bad call). This is
        // the true positive for that generality gap, pinned end-to-end.
        let src = "proc my_helper {x} {puts $x}\n\nset q {\n  aaa\nmy_helper\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        assert!(
            r.diagnostics.iter().any(|d| d.code == DiagCode::E002),
            "the swallowed `my_helper` call (wrong arity) must still be \
             analysed as code, not silently dropped: {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.code.to_string())
                .collect::<Vec<_>>()
        );
    }

    // --- E202/E203: short / single-line unterminated delimiters ------------
    //
    // Previously `is_suspicious_quote` required the token's inner text to
    // *start* with a newline, and `is_suspicious_str` required at least two
    // newlines — so the overwhelmingly common real-world shape (content on
    // the same line as the opener, then EOF or one line break) went
    // completely unflagged: no E202/E203, not even the generic E200
    // fallback. `info complete` on each of these (tclsh 8.6/9.0) is `0`.

    #[test]
    fn tp_e202_fires_for_single_line_unterminated_quote_no_newline() {
        // `set x "hello` — no newline at all, EOF right after the content.
        assert_eq!(
            recovery_diags("set x \"hello", "E202"),
            vec![("missing \"".to_string(), 0)]
        );
    }

    #[test]
    fn tp_e202_fires_when_content_precedes_the_break() {
        // `set x "hello\nworld` — content on the opening line, one break.
        assert_eq!(
            recovery_diags("set x \"hello\nworld\n", "E202"),
            vec![("missing \"".to_string(), 0)]
        );
    }

    #[test]
    fn tp_e203_fires_for_single_line_unterminated_brace_no_newline() {
        // `set x {hello` — no newline, EOF right after the content.
        assert_eq!(
            recovery_diags("set x {hello", "E203"),
            vec![("missing close-brace".to_string(), 0)]
        );
    }

    #[test]
    fn tp_e203_fires_when_content_precedes_the_break() {
        // `set x {hello\nworld` — content on the opening line, one break —
        // previously silent because it fell one newline short of the old
        // ">= 2 newlines" gate.
        assert_eq!(
            recovery_diags("set x {hello\nworld\n", "E203"),
            vec![("missing close-brace".to_string(), 0)]
        );
    }

    #[test]
    fn tn_well_formed_empty_and_short_delimiters_are_silent() {
        // Regression guard for the empty-`{}`/`""` special case: removing
        // the line-count gate must not turn well-formed short/empty
        // delimiters into false positives.
        for src in [
            "set x \"\"\n",
            "set x {}\n",
            "set x \"hello\"\n",
            "set x {hello}\n",
            "set x \"hello\nworld\"\n",
            "set x {hello\nworld}\n",
            "puts [llength []]\n",
        ] {
            assert!(
                recovery_diags(src, "E202").is_empty(),
                "unexpected E202 for {src:?}"
            );
            assert!(
                recovery_diags(src, "E203").is_empty(),
                "unexpected E203 for {src:?}"
            );
        }
    }

    #[test]
    fn tn_closed_multiline_quote_inside_body_stays_silent_when_short() {
        // A closed multi-line quote at a body's end (the fix's target
        // scenario for `e202_not_emitted_for_closed_multiline_quote_at_body_end`)
        // must remain silent even with the line-count gate removed —
        // "properly closed" is still detected purely from the closing
        // delimiter, never from line count.
        assert!(recovery_diags("proc p {} {set x \"\nhello\"}\n", "E202").is_empty());
    }

    #[test]
    fn tn_closed_quote_with_substitution_fragments_is_silent() {
        // A quoted word containing a `$var`/`[cmd]` substitution is
        // segmented by the lexer into several sibling Esc/Var fragments.
        // The fragment right after the substitution starts with the
        // closing `"` itself, which without the continuation check below
        // looks exactly like a *second* unterminated opener — firing E202
        // twice for a string that is properly closed.
        for src in [
            "puts \"$sum\"\n",
            "puts \"hello $name world\"\n",
            "puts \"$a$b\"\n",
            "puts \"[foo]\"\n",
        ] {
            assert!(
                recovery_diags(src, "E202").is_empty(),
                "unexpected E202 for {src:?}"
            );
        }
    }

    #[test]
    fn tp_e202_fires_for_unterminated_quote_with_substitution_fragments() {
        // The continuation-skip must not create a false negative: a quote
        // that opens, contains a substitution, and then genuinely never
        // closes must still be flagged exactly once, at the opener.
        assert_eq!(
            recovery_diags("puts \"hello $name", "E202"),
            vec![("missing \"".to_string(), 0)]
        );
    }

    // --- E200: tight highlighting -------------------------------------

    #[test]
    fn tp_e200_anchors_at_the_unclosed_delimiter_not_the_whole_command() {
        // A tclOO class whose method body never closes: the outer class
        // body's own `{` never closes either (its content contains a
        // stray `}` from the balanced `method bar {}` parameter list, so
        // it's E103/E200 territory, not the precise E203 detector). The
        // diagnostic must anchor at the unclosed `{` — not span the whole
        // multi-line `oo::class create Foo { ... }` command through EOF.
        let src = "oo::class create Foo {\n  method bar {} {\n    puts hi\n";
        let (start, end) = e20x_span(src, "E200").expect("E200 should fire");
        assert_eq!((start, end), (start, start), "E200 span must be zero-width");
        // The anchor sits at the outer class-body `{` (right after `Foo `,
        // byte offset 21), not at byte 0 (`oo::class`).
        assert_eq!(
            start, 21,
            "E200 must anchor at the unclosed `{{`, not the command start"
        );
        assert_eq!(&src[start as usize..=start as usize], "{");
    }

    #[test]
    fn tp_e200_message_matches_the_actual_unclosed_delimiter_kind() {
        // The E200 message must still name the *right* delimiter kind after
        // the span fix — this pins the suffix selection logic, not just
        // the anchor.
        let src = "oo::class create Foo {\n  method bar {} {\n    puts hi\n";
        let mut a = Analyser::new();
        let d = a
            .analyse(src, "tcl8.6")
            .diagnostics
            .into_iter()
            .find(|d| d.code == DiagCode::E200)
            .expect("E200 should fire");
        assert_eq!(d.message, "missing close-brace");
    }

    // Span precision: the highlighted range must cover the stray `]` /
    // `}` character itself (issue: `span.end()` is exclusive per
    // `span_to_range`, so an end offset *at* the delimiter excludes it
    // from the editor's underline).

    fn e100_span(src: &str) -> tcl_lexer::Span {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::E100)
            .unwrap_or_else(|| panic!("no E100 for {src:?}"))
            .span
    }

    fn e102_span(src: &str) -> tcl_lexer::Span {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::E102)
            .unwrap_or_else(|| panic!("no E102 for {src:?}"))
            .span
    }

    #[test]
    fn e100_span_includes_the_bracket_with_a_fix() {
        // `puts string]` — `string` is a known command name immediately
        // before the `]`, so heuristic 1a anchors the fix at `string`'s
        // own token start; the span must still run through (and
        // include) the `]` itself.
        let src = "puts string]";
        let span = e100_span(src);
        assert_eq!(
            &src[span.start() as usize..span.end() as usize],
            "string]",
            "highlighted text must include the ']' — span={span:?}"
        );
        assert_eq!(src.as_bytes()[span.end() as usize - 1], b']');
    }

    #[test]
    fn e100_span_is_tight_around_bracket_without_a_fix() {
        // `set x blah]` — no known command / arity overflow, so no fix
        // is available. The highlight must be just the `]` itself, not
        // the whole command from `set`.
        let src = "set x blah]";
        let span = e100_span(src);
        assert_eq!(
            &src[span.start() as usize..span.end() as usize],
            "]",
            "fix-less E100 must highlight only the stray ']', not the whole command"
        );
    }

    #[test]
    fn e102_span_includes_the_brace() {
        let src = "set x 1\n}\n";
        let span = e102_span(src);
        assert_eq!(&src[span.start() as usize..span.end() as usize], "}");
    }

    // Backslash-run parity: `\]` (odd run) is escaped and must not
    // fire; `\\]` (even run — a literal backslash followed by a bare
    // `]`) is NOT escaped and must fire. A naive "one preceding
    // backslash" check gets the even case wrong.

    #[test]
    fn single_backslash_escapes_the_bracket() {
        assert!(!codes(r"puts foo\]").contains(&"E100".to_string()));
    }

    #[test]
    fn double_backslash_does_not_escape_the_bracket() {
        // `\\` is a literal backslash (even run); the following `]` is
        // bare and must still be flagged.
        assert!(codes(r"puts foo\\]").contains(&"E100".to_string()));
    }

    #[test]
    fn triple_backslash_escapes_the_bracket() {
        // Odd run (3): the last backslash pairs with `]`.
        assert!(!codes(r"puts foo\\\]").contains(&"E100".to_string()));
    }

    // E102 embedded-`}` detection: a `}` need not be a token's entire
    // text to be stray — `foo}bar` is just as unmatched as a lone `}`,
    // matching E100's compound-token scan (`foo]bar`).

    #[test]
    fn embedded_close_brace_emits_e102() {
        assert!(codes("set x foo}bar\n").contains(&"E102".to_string()));
    }

    #[test]
    fn embedded_close_brace_escaped_does_not_fire() {
        assert!(!codes(r"set x foo\}bar").contains(&"E102".to_string()));
    }

    #[test]
    fn embedded_close_brace_no_fix_offered() {
        // The whole-line-deletion fix only applies when `}` is the
        // entire line; an embedded brace gets a diagnostic but no fix
        // (deleting one character out of a bareword isn't a safe guess).
        let mut a = Analyser::new();
        let d = a
            .analyse("set x foo}bar\n", "tcl8.6")
            .diagnostics
            .into_iter()
            .find(|d| d.code == DiagCode::E102)
            .expect("E102 expected");
        assert!(d.fixes.is_empty());
    }

    // Known-name breadth: the bracket-insertion heuristic must
    // recognise a call to an already-declared user proc / tclOO class /
    // alias / ensemble, not just a registry builtin — the general form
    // of "unknown, aliasing, tclOO" command resolution.

    #[test]
    fn insertion_point_recognises_user_defined_proc() {
        let src = "proc myHelper {a b} {return $a}\nset y myHelper arg1 arg2]\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let e100 = r
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::E100)
            .expect("E100 expected");
        assert_eq!(e100.fixes.len(), 1, "{:?}", e100.fixes);
        assert_eq!(e100.fixes[0].new_text, "[");
        let insert_off = e100.fixes[0].span.start() as usize;
        assert_eq!(
            &src[insert_off..insert_off + "myHelper".len()],
            "myHelper",
            "fix should insert '[' right before the user proc call"
        );
        // No phantom unknown-command diagnostic on the repaired name.
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W123 && d.message.contains("elper")),
            "repair must not corrupt the command name: {:?}",
            r.diagnostics
                .iter()
                .map(|d| (d.code.to_string(), d.message.clone()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn insertion_point_recognises_tcloo_class() {
        let src = "oo::class create Widget {}\nset y Widget create]\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let e100 = r
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::E100)
            .expect("E100 expected");
        assert_eq!(e100.fixes.len(), 1, "{:?}", e100.fixes);
    }

    // Recovery/diagnostic unification: the repair must never fire (and
    // corrupt downstream command-invocation recording) where E100 does
    // not — regression coverage for the drift between the old
    // independent `find_stray_close_bracket` and the escape-aware E100
    // scan.

    #[test]
    fn escaped_bracket_with_arity_overflow_neither_fires_nor_repairs() {
        // `set` only takes 1-2 args; the third + a *genuinely escaped*
        // trailing `]` used to still trigger the old recovery's arity
        // fallback (which ignored escaping entirely), silently
        // "repairing" a bracket E100 correctly treats as a literal.
        let src = r"set y bar baz\]";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::E100),
            "escaped bracket must not be flagged: {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.code.to_string())
                .collect::<Vec<_>>()
        );
        // Only the real arity error (E003 too many args) should fire —
        // no phantom unknown-command from a bad repair.
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
            "no repair should have run: {:?}",
            r.diagnostics
                .iter()
                .map(|d| (d.code.to_string(), d.message.clone()))
                .collect::<Vec<_>>()
        );
    }
}
