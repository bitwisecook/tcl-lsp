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

//! Core formatting engine.
//!
//! Parses Tcl source into commands, identifies body / expr /
//! param-list arguments via the registry, recursively formats
//! bodies, and reconstructs the output (K&R braces, blank-line
//! policy, comment normalisation, switch bodies, long-line
//! backslash splitting, and `&&`/`||` expression wrapping).
//!
//! The entry point is [`format_tcl`]; everything else is the
//! private machinery [`format_tcl`] drives.

use tcl_compiler::lambda_literal::split_lambda_literal_decoded;
use tcl_lexer::{Lexer, SourceMap, Token, TokenType};
use tcl_registry::{ArgRole, CommandRegistry, Traits};

use super::config::FormatterConfig;

/// What kind of argument this is for formatting purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgKind {
    /// Plain argument — reconstruct from tokens.
    Word,
    /// Script body — recursively format, wrap in `{}`.
    Body,
    /// Structural keyword (`else`, `elseif`, `then`, `finally`,
    /// `on`, `trap`).
    Keyword,
    /// Parameter list — normalise internal whitespace.
    ParamList,
    /// `ArgRole::LambdaLiteral` argument (`apply`'s `{argList body ?ns?}`
    /// shape) — the parameter-list element is normalised and the body
    /// element recursively formatted, then reassembled; never fed to
    /// `format_body` as a whole (that would misread the parameter word as a
    /// command name — issue #954).
    LambdaLiteral,
}

/// A single argument to a command.
struct CommandArg {
    kind: ArgKind,
    tokens: Vec<Token>,
    /// Concatenated token text (delimiters stripped by the lexer).
    text: String,
    /// First token was a braced `{…}` literal.
    is_braced: bool,
    /// The argument was `"`-quoted.
    is_quoted: bool,
    /// The recursively reformatted script content of a `Body` argument, or
    /// (for `LambdaLiteral`) of just its body *element* — the parameter-list
    /// element is never fed here, only normalised at reconstruction time
    /// (`render_lambda_literal_arg`).
    formatted_body: Option<String>,
}

/// A comment line captured during parsing, with its original source
/// indentation so the formatter can either re-indent it to the code column
/// (`align_comments_to_code`, the default) or preserve where the author put it.
#[derive(Clone)]
struct CommentLine {
    /// Leading whitespace on the comment's original line, when the comment
    /// stood alone at the start of its line. `None` for a relocated inline
    /// (`;#`) comment, which has no standalone column to preserve.
    orig_indent: Option<String>,
    /// The comment text, starting at `#`.
    text: String,
}

/// A single Tcl command with its arguments, ready for reformatting.
struct ParsedCommand {
    name: String,
    args: Vec<CommandArg>,
    preceding_comments: Vec<CommentLine>,
    preceding_blank_lines: usize,
    /// The separator that terminated this command was a bare `;` on the same
    /// line (no newline). Lets the emitter keep `a; b` on one line when
    /// `replace_semicolons_with_newlines` is disabled.
    terminated_by_semicolon: bool,
}

// Token → raw source reconstruction

/// Collapse `\<newline>` continuations (LF, CR, or CRLF) to a single space.
///
/// `keep_preceding` controls whitespace *before* the backslash. Inside a
/// double-quoted string that whitespace is literal data, so it must survive
/// (`"a \<nl> b"` is the value `a  b`, two spaces — `RUST_ISSUE_104`); pass
/// `true`. Inside a command substitution `[…]` (and bare word contexts) the
/// whole `<ws>\<nl><ws>` run is inter-word spacing the lexer collapses to a
/// single space, so the preceding whitespace must be trimmed (`[cat a \<nl> b]`
/// → `[cat a b]`); pass `false`. Both delegate to the shared `tcl-syntax`
/// collapse, which also leaves an escaped backslash (`\\<nl>` — a literal `\`
/// before a real newline) alone rather than treating it as a continuation.
fn normalise_backslash_newline(text: &str, keep_preceding: bool) -> String {
    if keep_preceding {
        tcl_syntax::backslash::collapse_brace_continuations_str(text).into_owned()
    } else {
        tcl_syntax::backslash::collapse_separator_continuations_str(text).into_owned()
    }
}

/// Rebuild source text from a single token, re-adding delimiters.
///
/// `in_quotes` is set when the token is being reconstructed inside a
/// double-quoted argument (which the caller re-wraps in `"…"`). In that context
/// a `Str` token is literal string data — a lone `$` the lexer classified as
/// `Str`, for instance — so it is emitted verbatim rather than brace-wrapped as
/// `{$}`, which would change the string's value (`RUST_ISSUE_103`).
fn reconstruct_raw(sm: &SourceMap, tok: Token, in_quotes: bool) -> String {
    match tok.kind {
        TokenType::Str if in_quotes => sm.text(tok.span).to_owned(),
        TokenType::Str => format!("{{{}}}", sm.token_text(tok)),
        TokenType::Cmd => format!(
            "[{}]",
            normalise_backslash_newline(sm.token_text(tok), false)
        ),
        TokenType::Var => {
            let raw = sm.text(tok.span);
            if raw.starts_with("${") {
                format!("${{{}}}", sm.token_text(tok))
            } else {
                format!("${}", sm.token_text(tok))
            }
        }
        TokenType::Expand => "{*}".to_owned(),
        _ => sm.token_text(tok).to_owned(),
    }
}

/// Rebuild an argument's source text from its tokens, optionally
/// rewriting `$var` → `${var}`.
fn reconstruct_arg(sm: &SourceMap, arg: &CommandArg, braced_vars: bool) -> String {
    use std::fmt::Write as _;
    let mut raw = String::new();
    for &tok in &arg.tokens {
        if braced_vars && tok.kind == TokenType::Var {
            let _ = write!(raw, "${{{}}}", sm.token_text(tok));
        } else {
            raw.push_str(&reconstruct_raw(sm, tok, arg.is_quoted));
        }
    }
    if arg.is_quoted {
        format!("\"{raw}\"")
    } else {
        raw
    }
}

/// Normalise whitespace in a braced parameter list, joining
/// top-level elements with single spaces.
fn normalise_param_list(text: &str) -> String {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut elements: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < n {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' => {
                i += 1;
            }
            b'{' => {
                let start = i;
                let mut level = 1;
                i += 1;
                while i < n && level > 0 {
                    match bytes[i] {
                        b'{' => level += 1,
                        b'}' => level -= 1,
                        _ => {}
                    }
                    i += 1;
                }
                elements.push(&text[start..i]);
            }
            _ => {
                let start = i;
                while i < n && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                    i += 1;
                }
                elements.push(&text[start..i]);
            }
        }
    }
    format!("{{{}}}", elements.join(" "))
}

// Command parsing

/// Count blank lines represented by an EOL token's text (each
/// newline beyond the first is a blank line).
fn count_newlines(text: &str) -> usize {
    text.matches('\n').count().saturating_sub(1)
}

/// The leading whitespace on the source line containing `offset`, but only when
/// everything before `offset` on that line is whitespace — i.e. the comment
/// stands alone at line start. Returns `None` for a mid-line (inline `;#`)
/// comment, which has no standalone column to preserve.
fn comment_orig_indent(source: &str, offset: usize) -> Option<String> {
    let line_start = source.get(..offset)?.rfind('\n').map_or(0, |p| p + 1);
    let prefix = source.get(line_start..offset)?;
    if prefix.bytes().all(|b| b == b' ' || b == b'\t') {
        Some(prefix.to_owned())
    } else {
        None
    }
}

/// Parse Tcl source into structured commands plus any trailing
/// comments.
fn parse_commands(
    source: &str,
    sm: &SourceMap,
    tokens: &[Token],
) -> (Vec<ParsedCommand>, Vec<CommentLine>) {
    let mut commands: Vec<ParsedCommand> = Vec::new();
    let mut pending_comments: Vec<CommentLine> = Vec::new();
    let mut pending_blank_lines = 0usize;

    let mut argv: Vec<CommandArg> = Vec::new();
    let mut prev_type = TokenType::Eol;

    let flush = |argv: &mut Vec<CommandArg>,
                 pending_comments: &mut Vec<CommentLine>,
                 pending_blank_lines: usize,
                 terminated_by_semicolon: bool,
                 commands: &mut Vec<ParsedCommand>| {
        if argv.is_empty() {
            return;
        }
        let taken_args = std::mem::take(argv);
        let name = taken_args
            .first()
            .map(|a| a.text.clone())
            .unwrap_or_default();
        commands.push(ParsedCommand {
            name,
            args: taken_args,
            preceding_comments: std::mem::take(pending_comments),
            preceding_blank_lines: pending_blank_lines,
            terminated_by_semicolon,
        });
    };

    for &tok in tokens {
        match tok.kind {
            TokenType::Eof => break,
            TokenType::Comment => {
                pending_comments.push(CommentLine {
                    orig_indent: comment_orig_indent(source, tok.span.start() as usize),
                    text: sm.text(tok.span).to_owned(),
                });
                continue;
            }
            TokenType::Sep => {
                prev_type = TokenType::Sep;
                continue;
            }
            TokenType::Eol => {
                let newlines = count_newlines(sm.text(tok.span));
                if argv.is_empty() {
                    pending_blank_lines += newlines;
                } else {
                    // A pure `;` separator (no newline in the Eol run) means
                    // this command shared a line with the next.
                    flush(
                        &mut argv,
                        &mut pending_comments,
                        pending_blank_lines,
                        newlines == 0,
                        &mut commands,
                    );
                    pending_blank_lines = newlines;
                }
                prev_type = TokenType::Eol;
                continue;
            }
            _ => {}
        }

        let is_start_of_new_arg = matches!(prev_type, TokenType::Sep | TokenType::Eol);
        let detected_quoted =
            is_start_of_new_arg && source.as_bytes().get(tok.span.start() as usize) == Some(&b'"');
        let text = sm.token_text(tok).to_owned();

        if is_start_of_new_arg || argv.is_empty() {
            argv.push(CommandArg {
                kind: ArgKind::Word,
                tokens: vec![tok],
                text,
                is_braced: tok.kind == TokenType::Str,
                is_quoted: detected_quoted,
                formatted_body: None,
            });
        } else {
            let last = argv.last_mut().expect("argv non-empty");
            last.tokens.push(tok);
            last.text.push_str(&text);
        }
        prev_type = tok.kind;
    }

    flush(
        &mut argv,
        &mut pending_comments,
        pending_blank_lines,
        false,
        &mut commands,
    );
    (commands, pending_comments)
}

// Body / expr / param-list argument identification

/// Mark body / keyword / param-list arguments in place.
fn identify_body_args(cmd: &mut ParsedCommand, registry: &CommandRegistry) {
    // {*}-expanded command word: dynamic identity, skip.
    if cmd
        .args
        .first()
        .and_then(|a| a.tokens.first())
        .is_some_and(|t| t.kind == TokenType::Expand)
    {
        return;
    }

    let name = cmd.name.clone();
    // Post-name argument texts, owned so the immutable borrow of
    // `cmd.args` is released before the role-driven mutation below.
    let arg_texts: Vec<String> = cmd.args.iter().skip(1).map(|a| a.text.clone()).collect();

    // Formatter-specific `for` override: only the main body (arg 3)
    // expands; init / next stay inline.
    if name == "for" && arg_texts.len() >= 4 {
        if cmd.args[4].is_braced {
            cmd.args[4].kind = ArgKind::Body;
        }
        return;
    }

    let refs: Vec<&str> = arg_texts.iter().map(String::as_str).collect();
    let body_indices = registry.arg_indices_for_role(&name, &refs, ArgRole::Body);
    let param_indices = registry.arg_indices_for_role(&name, &refs, ArgRole::ParamList);
    let lambda_indices = registry.arg_indices_for_role(&name, &refs, ArgRole::LambdaLiteral);
    for idx in body_indices {
        let actual = idx + 1; // +1 for the command-name slot.
        if actual < cmd.args.len() && cmd.args[actual].is_braced {
            cmd.args[actual].kind = ArgKind::Body;
        }
    }
    for idx in lambda_indices {
        let actual = idx + 1; // +1 for the command-name slot.
        if actual < cmd.args.len() && cmd.args[actual].is_braced {
            cmd.args[actual].kind = ArgKind::LambdaLiteral;
        }
    }

    // Structural keywords.
    if name == "if" {
        for arg in cmd.args.iter_mut().skip(1) {
            if matches!(arg.text.as_str(), "then" | "else" | "elseif") {
                arg.kind = ArgKind::Keyword;
            }
        }
    } else if name == "try" {
        for arg in cmd.args.iter_mut().skip(1) {
            if matches!(arg.text.as_str(), "finally" | "on" | "trap") {
                arg.kind = ArgKind::Keyword;
            }
        }
    }

    // Parameter lists.
    for idx in param_indices {
        let actual = idx + 1;
        if actual < cmd.args.len() && cmd.args[actual].is_braced {
            cmd.args[actual].kind = ArgKind::ParamList;
        }
    }
}

/// Indices into `cmd.args` of braced expression arguments.
fn identify_expr_args(cmd: &ParsedCommand, registry: &CommandRegistry) -> Vec<usize> {
    let arg_texts: Vec<&str> = cmd.args.iter().skip(1).map(|a| a.text.as_str()).collect();
    registry
        .arg_indices_for_role(&cmd.name, &arg_texts, ArgRole::Expr)
        .into_iter()
        .map(|idx| idx + 1)
        .collect()
}

// Comment formatting

/// Format a comment per config.
fn format_comment(comment_text: &str, config: &FormatterConfig) -> String {
    if comment_text.is_empty() || comment_text == "#" {
        return "#".to_owned();
    }
    let after_hashes = comment_text.trim_start_matches('#');
    let num_hashes = comment_text.len() - after_hashes.len();
    if after_hashes.is_empty() {
        return "#".repeat(num_hashes);
    }
    let is_commented_code = !after_hashes.chars().next().is_some_and(char::is_whitespace);
    let hashes = "#".repeat(num_hashes);
    if is_commented_code {
        // Commented-out code is not string data; collapse continuations fully.
        format!(
            "{hashes}{}",
            normalise_backslash_newline(after_hashes, false)
        )
    } else if config.space_after_comment_hash {
        format!("{hashes}{}", after_hashes.trim_end())
    } else {
        format!("{hashes}{}", after_hashes.trim())
    }
}

// Blank-line computation

/// How many blank lines to insert before `commands[index]`.
fn compute_blank_lines(
    commands: &[ParsedCommand],
    index: usize,
    config: &FormatterConfig,
) -> usize {
    if index == 0 {
        return 0;
    }
    let current = &commands[index];
    let prev = &commands[index - 1];
    if prev.name == "proc" && current.name == "proc" {
        return config.blank_lines_between_procs;
    }
    if prev.name == "proc" || current.name == "proc" {
        return config.blank_lines_between_blocks;
    }
    current
        .preceding_blank_lines
        .min(config.max_consecutive_blank_lines)
}

// Switch body formatting

/// One element (pattern or body) of a `switch` body.
struct SwitchElem {
    tokens: Vec<Token>,
    text: String,
    is_braced: bool,
    /// A comment token standing on its own — emitted verbatim on its own line
    /// rather than paired as a pattern/body, so switch-body comments are not
    /// silently deleted (`RUST_ISSUE_038`).
    is_comment: bool,
}

/// Format the braced body of a `switch` command (pattern/body
/// pairs).
fn format_switch_body(
    body_text: &str,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    indent_level: usize,
) -> String {
    let sm = SourceMap::new(body_text);
    let Ok(tokens) =
        Lexer::with_source_map(SourceMap::new(body_text), config.lexer_config).tokenise_all()
    else {
        return body_text.to_owned();
    };

    let elements = segment_switch_elements(&sm, tokens);
    let lines = emit_switch_lines(&elements, &sm, config, registry, indent_level);
    lines.join("\n")
}

/// Re-segment a switch body's tokens into pattern/body/comment elements.
fn segment_switch_elements(sm: &SourceMap, tokens: Vec<Token>) -> Vec<SwitchElem> {
    let mut elements: Vec<SwitchElem> = Vec::new();
    let mut cur: Option<SwitchElem> = None;
    let mut prev_type = TokenType::Eol;
    for tok in tokens {
        match tok.kind {
            TokenType::Eof => break,
            TokenType::Sep | TokenType::Eol => {
                if let Some(e) = cur.take() {
                    elements.push(e);
                }
                prev_type = tok.kind;
                continue;
            }
            TokenType::Comment => {
                // Preserve the comment as a standalone element instead of
                // dropping it (RUST_ISSUE_038): flush any in-progress element,
                // then push the comment on its own so the emit loop renders it
                // on its own line.
                if let Some(e) = cur.take() {
                    elements.push(e);
                }
                elements.push(SwitchElem {
                    tokens: vec![tok],
                    text: sm.token_text(tok).to_owned(),
                    is_braced: false,
                    is_comment: true,
                });
                prev_type = tok.kind;
                continue;
            }
            _ => {}
        }
        let text = sm.token_text(tok).to_owned();
        if matches!(prev_type, TokenType::Sep | TokenType::Eol) {
            if let Some(e) = cur.take() {
                elements.push(e);
            }
            cur = Some(SwitchElem {
                tokens: vec![tok],
                text,
                is_braced: tok.kind == TokenType::Str,
                is_comment: false,
            });
        } else if let Some(e) = cur.as_mut() {
            e.tokens.push(tok);
            e.text.push_str(&text);
        } else {
            cur = Some(SwitchElem {
                tokens: vec![tok],
                text,
                is_braced: tok.kind == TokenType::Str,
                is_comment: false,
            });
        }
        prev_type = tok.kind;
    }
    if let Some(e) = cur.take() {
        elements.push(e);
    }
    elements
}

/// Render segmented switch elements into formatted lines.
fn emit_switch_lines(
    elements: &[SwitchElem],
    sm: &SourceMap,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    indent_level: usize,
) -> Vec<String> {
    let indent = config.make_indent(indent_level);
    let inner_level = indent_level + 1;
    let mut lines: Vec<String> = Vec::new();
    let mut i = 0;
    while i < elements.len() {
        // A comment element is emitted verbatim on its own line and never
        // consumed as a pattern (RUST_ISSUE_038).
        if elements[i].is_comment {
            let comment_raw: String = elements[i]
                .tokens
                .iter()
                .map(|&t| reconstruct_raw(sm, t, false))
                .collect();
            lines.push(format!("{indent}{}", comment_raw.trim_end()));
            i += 1;
            continue;
        }
        let pattern_raw: String = elements[i]
            .tokens
            .iter()
            .map(|&t| reconstruct_raw(sm, t, false))
            .collect();
        // If the *body* slot is a comment, don't pair it as this pattern's
        // body — emit the pattern alone and let the comment be handled on the
        // next iteration.
        if i + 1 < elements.len() && !elements[i + 1].is_comment {
            let body = &elements[i + 1];
            if body.text == "-" {
                lines.push(format!("{indent}{pattern_raw} -"));
            } else if body.is_braced {
                let formatted = format_body(&body.text, config, registry, inner_level);
                if formatted.trim().is_empty() {
                    lines.push(format!("{indent}{pattern_raw} {{}}"));
                } else {
                    lines.push(format!("{indent}{pattern_raw} {{"));
                    lines.push(formatted);
                    lines.push(format!("{indent}}}"));
                }
            } else {
                let body_raw: String = body
                    .tokens
                    .iter()
                    .map(|&t| reconstruct_raw(sm, t, false))
                    .collect();
                lines.push(format!("{indent}{pattern_raw} {body_raw}"));
            }
            i += 2;
        } else {
            lines.push(format!("{indent}{pattern_raw}"));
            i += 1;
        }
    }
    lines
}

// Long-line expression wrapping

/// Find top-level `&&` / `||` positions in an expression (not
/// nested in `[] {} () ""`).
fn find_expr_break_points(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut breaks = Vec::new();
    let (mut db, mut dbr, mut dp) = (0i32, 0i32, 0i32);
    let mut in_quotes = false;
    let mut i = 0;
    while i < n {
        let ch = bytes[i];
        if ch == b'\\' {
            i += 2;
            continue;
        }
        if ch == b'"' && db == 0 {
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }
        if in_quotes {
            i += 1;
            continue;
        }
        match ch {
            b'[' => dbr += 1,
            b']' => dbr = (dbr - 1).max(0),
            b'{' => db += 1,
            b'}' => db = (db - 1).max(0),
            b'(' => dp += 1,
            b')' => dp = (dp - 1).max(0),
            _ => {
                if dbr == 0 && db == 0 && dp == 0 && i + 1 < n {
                    let two = &bytes[i..i + 2];
                    if two == b"&&" || two == b"||" {
                        breaks.push(i);
                        i += 2;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    breaks
}

/// Try to wrap a braced expression at `&&` / `||`; returns the
/// wrapped inner text (no braces) or `None`.
fn wrap_braced_expr(text: &str, config: &FormatterConfig, indent_level: usize) -> Option<String> {
    let stripped: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let breaks = find_expr_break_points(&stripped);
    if breaks.is_empty() {
        return None;
    }
    let mut chunks: Vec<&str> = Vec::new();
    let mut last = 0;
    for &pos in &breaks {
        chunks.push(stripped[last..pos].trim_end());
        last = pos;
    }
    chunks.push(&stripped[last..]);

    let expr_indent = config.make_indent(indent_level + 1);
    let cmd_indent = config.make_indent(indent_level);
    let inner = chunks
        .iter()
        .map(|c| format!("{expr_indent}{c}"))
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!("\n{inner}\n{cmd_indent}"))
}

/// Estimate the content length after `args[start]` on the first
/// line.
fn estimate_trailing_len(args: &[CommandArg], start: usize) -> usize {
    let mut length = 0;
    for arg in &args[start..] {
        match arg.kind {
            ArgKind::Body => {
                length += 2; // " {"
                break;
            }
            ArgKind::Keyword => length += 1 + arg.text.len(),
            _ => length += 1 + arg.text.len() + if arg.is_braced { 2 } else { 0 },
        }
    }
    length
}

/// Current column from accumulated parts.
fn current_col(parts: &[String], indent_len: usize) -> usize {
    let text: String = parts.concat();
    match text.rfind('\n') {
        None => indent_len + text.len(),
        Some(nl) => text.len() - nl - 1,
    }
}

// Backslash-continuation line splitting

/// Find `(index, bracket_depth)` of spaces where `\`-continuation
/// is safe.
fn find_splittable_spaces(text: &str, start: usize) -> Vec<(usize, i32)> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut spaces = Vec::new();
    let (mut dbr, mut db) = (0i32, 0i32);
    let mut in_quotes = false;
    let mut i = start;
    while i < n {
        let ch = bytes[i];
        if ch == b'\\' && i + 1 < n {
            i += 2;
            continue;
        }
        if ch == b'"' && db == 0 {
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }
        if in_quotes {
            i += 1;
            continue;
        }
        match ch {
            b'[' => dbr += 1,
            b']' => dbr = (dbr - 1).max(0),
            b'{' => db += 1,
            b'}' => db = (db - 1).max(0),
            b' ' if db == 0 => spaces.push((i, dbr)),
            _ => {}
        }
        i += 1;
    }
    spaces
}

/// Greedy line splitting at the given space positions. Callers pass only
/// positions that are safe word separators (outside double-quoted strings); a
/// space *inside* a `"…"` is string data, not a separator, and breaking there
/// would alter the string's value, so such positions are never offered here.
fn greedy_split(
    line: &str,
    spaces: &[usize],
    max_len: usize,
    cont_indent: &str,
) -> Option<Vec<String>> {
    if spaces.is_empty() {
        return None;
    }
    let indent_len = line.len() - line.trim_start().len();
    let mut segments: Vec<String> = Vec::new();
    let mut seg_start = 0usize;
    let mut last_good: Option<usize> = None;

    let commit_break = |segments: &mut Vec<String>,
                        seg_start: &mut usize,
                        last_good: &mut Option<usize>,
                        break_pos: usize| {
        let mut text = line[*seg_start..break_pos].to_owned();
        if *seg_start > 0 && *seg_start > indent_len {
            text = format!("{cont_indent}{text}");
        }
        segments.push(format!("{text} \\"));
        *seg_start = break_pos + 1;
        *last_good = None;
    };

    for &sp in spaces {
        let seg_len = if seg_start == 0 {
            sp + 2
        } else {
            cont_indent.len() + (sp - seg_start) + 2
        };
        if seg_len <= max_len {
            last_good = Some(sp);
        } else if let Some(lg) = last_good {
            commit_break(&mut segments, &mut seg_start, &mut last_good, lg);
            let new_len = cont_indent.len() + (sp - seg_start) + 2;
            last_good = if new_len <= max_len { Some(sp) } else { None };
        } else {
            commit_break(&mut segments, &mut seg_start, &mut last_good, sp);
        }
    }

    if segments.is_empty() {
        {
            let lg = last_good?;
            commit_break(&mut segments, &mut seg_start, &mut last_good, lg);
        }
    }

    let mut remainder = line[seg_start..].to_owned();
    if seg_start > 0 && seg_start > indent_len {
        remainder = format!("{cont_indent}{remainder}");
    }
    segments.push(remainder);
    Some(segments)
}

/// Split a long line using `\` continuation, preferring shallow
/// breaks.
fn split_long_line(line: &str, config: &FormatterConfig, cont_indent: &str) -> Option<String> {
    if line.len() <= config.max_line_length {
        return None;
    }
    let indent_len = line.len() - line.trim_start().len();
    let all_spaces = find_splittable_spaces(line, indent_len);
    let max_len = config.max_line_length;
    let mut segments: Option<Vec<String>> = None;

    if !all_spaces.is_empty() {
        let max_depth = all_spaces.iter().map(|&(_, d)| d).max().unwrap_or(0);
        for target_depth in 0..=max_depth {
            let spaces: Vec<usize> = all_spaces
                .iter()
                .filter(|&&(_, d)| d <= target_depth)
                .map(|&(p, _)| p)
                .collect();
            let seg = greedy_split(line, &spaces, max_len, cont_indent);
            if seg.as_ref().is_some_and(|s| s.len() > 1) {
                segments = seg;
                break;
            }
        }
    }

    // No safe split found among word separators outside quotes. We deliberately
    // do NOT fall back to breaking at a space *inside* a double-quoted string:
    // that space is string data, and inserting `\<newline>` + continuation
    // indent there changes the string's value (Tcl collapses the
    // `\<newline><leading-ws>` to a single space, but the original space is kept,
    // so the literal silently gains a space). A line that can only be split
    // inside a string literal is left over-length instead — layout is best
    // effort, but the program's data must never change.
    if segments.as_ref().is_none_or(|s| s.len() < 2) {
        return None;
    }

    let segments = segments?;
    let mut final_lines: Vec<String> = Vec::new();
    for seg in segments {
        if seg.ends_with(" \\") {
            final_lines.push(seg);
        } else if seg.len() > max_len {
            match split_long_line(&seg, config, cont_indent) {
                Some(sub) => final_lines.push(sub),
                None => final_lines.push(seg),
            }
        } else {
            final_lines.push(seg);
        }
    }
    Some(final_lines.join("\n"))
}

/// Try to split a long commented-out command using `\`
/// continuation.
fn split_commented_code(
    comment_text: &str,
    config: &FormatterConfig,
    indent: &str,
    indent_level: usize,
) -> Option<String> {
    if comment_text.len() < 2 {
        return None;
    }
    let after_hash = &comment_text[1..];
    if after_hash.is_empty() || after_hash.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    let full_line = format!("{indent}{comment_text}");
    if full_line.len() <= config.max_line_length {
        return None;
    }
    let cmd_line = format!("{indent}{after_hash}");
    let cont_indent = config.make_indent(indent_level + 1);
    let split = split_long_line(&cmd_line, config, &cont_indent)?;
    let mut split_lines: Vec<String> = split.split('\n').map(str::to_owned).collect();
    let first = split_lines[0]
        .strip_prefix(indent)
        .unwrap_or(&split_lines[0]);
    split_lines[0] = format!("{indent}#{first}");
    Some(split_lines.join("\n"))
}

// Inline body detection

/// Count the commands in a raw body — statements separated by a newline or a
/// top-level `;` (both are Tcl command terminators). Brace / bracket nesting is
/// tracked so separators inside a nested `{…}` or `[…]` do not inflate the
/// count. Used to gate `expand_single_line_bodies` on
/// `min_body_commands_for_expansion`.
fn count_body_commands(body_text: &str) -> usize {
    let mut count = 0;
    let mut depth = 0i32;
    let mut in_statement = false;
    let mut escaped = false;
    for c in body_text.chars() {
        if escaped {
            escaped = false;
            in_statement = true;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '{' | '[' => {
                depth += 1;
                in_statement = true;
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                in_statement = true;
            }
            '\n' | ';' if depth == 0 => {
                if in_statement {
                    count += 1;
                    in_statement = false;
                }
            }
            c if c.is_whitespace() => {}
            _ => in_statement = true,
        }
    }
    if in_statement {
        count += 1;
    }
    count
}

/// Whether a body is short enough to keep on one line.
fn body_can_be_inline(
    body_text: &str,
    config: &FormatterConfig,
    current_line_len: usize,
    never_inline: bool,
) -> bool {
    if config.expand_single_line_bodies
        && count_body_commands(body_text) >= config.min_body_commands_for_expansion
    {
        // Only force expansion once the body carries at least
        // `min_body_commands_for_expansion` commands; a trivial one-command
        // body stays inline (RUST_ISSUE_133).
        return false;
    }
    let stripped = body_text.trim();
    if stripped.is_empty() {
        return true;
    }
    if never_inline {
        return false;
    }
    if stripped.contains('\n') || stripped.contains('{') || stripped.contains('}') {
        return false;
    }
    let inline_len = current_line_len + "{ ".len() + stripped.len() + " }".len();
    inline_len <= config.goal_line_length
}

// Command reconstruction

/// Push a leading separator space before the next part, unless
/// we're continuing a `}{` brace chain with
/// `space_between_braces == false`.
fn maybe_space(parts: &mut Vec<String>, in_brace_chain: bool, space_between_braces: bool) {
    let suppress = in_brace_chain && !space_between_braces;
    if !suppress && !parts.is_empty() {
        parts.push(" ".to_owned());
    }
}

/// Render a formatted body argument (inline `{ … }` or expanded
/// K&R block) into `parts`.  The caller has already emitted any
/// required leading separator space.
fn append_body_no_space(
    parts: &mut Vec<String>,
    body: &str,
    config: &FormatterConfig,
    indent: &str,
    current_line_len: usize,
    never_inline: bool,
) {
    let inline = body_can_be_inline(body, config, current_line_len, never_inline);
    let stripped = body.trim();
    if stripped.is_empty() {
        parts.push("{}".to_owned());
    } else if inline {
        parts.push(format!("{{ {stripped} }}"));
    } else {
        parts.push("{\n".to_owned());
        parts.push(body.to_owned());
        parts.push(format!("\n{indent}}}"));
    }
}

/// Reassemble an `ArgKind::LambdaLiteral` argument: its parameter-list
/// element (normalised, not code) and its body element (`arg.formatted_body`
/// — already recursively formatted by the caller from just the *decoded*
/// body span, never the whole lambda literal, so the parameter word is
/// never misread as a command name), plus an optional namespace element,
/// wrapped back in the outer `{}`. Falls back to the original literal
/// verbatim if the source can no longer be split (should not happen once
/// `identify_body_args` and this function agree, but a stale token must not
/// panic).
///
/// The parameter-list and namespace elements are decoded (backslash escapes
/// collapsed for a bare/quoted element — [`split_lambda_literal_decoded`])
/// before use, not pasted through as raw source spelling: a non-literal
/// element's escapes would otherwise survive reformatting and change what it
/// means (codex review of #954's follow-up). The namespace is re-quoted with
/// [`tcl_syntax::list::list_element`] on reassembly rather than always left
/// bare, so an element that needs quoting (e.g. an escaped space) still
/// round-trips safely; `normalise_param_list` already unconditionally
/// brace-wraps the parameter list, so no further quoting is needed there.
fn render_lambda_literal_arg(
    sm: &SourceMap,
    arg: &CommandArg,
    config: &FormatterConfig,
    indent: &str,
    current_line_len: usize,
    never_inline: bool,
) -> String {
    let source = sm.source();
    let fallback = || format!("{{{}}}", arg.text);
    let Some(&tok) = arg.tokens.first() else {
        return fallback();
    };
    let Some(elems) = split_lambda_literal_decoded(source, tok) else {
        return fallback();
    };
    let Some(formatted_body) = arg.formatted_body.as_deref() else {
        return fallback();
    };
    let mut body_parts: Vec<String> = Vec::new();
    append_body_no_space(
        &mut body_parts,
        formatted_body,
        config,
        indent,
        current_line_len,
        never_inline,
    );
    let params_rendered = normalise_param_list(&elems.params);
    let body_rendered = body_parts.concat();
    match elems.namespace.as_deref() {
        Some(ns) => {
            let ns_rendered = tcl_syntax::list::list_element(ns);
            format!("{{{params_rendered} {body_rendered} {ns_rendered}}}")
        }
        None => format!("{{{params_rendered} {body_rendered}}}"),
    }
}

/// Append a plain word argument (with optional expression
/// wrapping).  Returns `false` when nothing was emitted (an
/// all-continuation artifact), so the caller leaves the brace
/// chain untouched.
fn append_word_arg(
    sm: &SourceMap,
    args: &[CommandArg],
    i: usize,
    expr_args: &[usize],
    config: &FormatterConfig,
    indent_level: usize,
    parts: &mut Vec<String>,
) -> bool {
    let arg = &args[i];
    let mut raw = reconstruct_arg(sm, arg, config.enforce_braced_variables);
    if raw.contains('\n') {
        // Keep whitespace before the backslash only for a quoted string, where
        // it is literal data (RUST_ISSUE_104); a bare/continued word collapses.
        let collapsed = normalise_backslash_newline(&raw, arg.is_quoted)
            .trim()
            .to_owned();
        if collapsed.is_empty() {
            return false;
        }
        raw = collapsed;
    }
    // `enforceBracedExpr`, bounded form: a single unbraced expression argument
    // (an `if` / `while` / `for` condition, `control::assert`'s first arg) is
    // wrapped in braces (`if $x …` → `if {$x} …`). A `"…"`-quoted operand, or
    // one carrying a `{*}` expansion (whose expansion braces would demote to
    // literal text), is left alone.
    let has_expansion = arg.tokens.iter().any(|t| t.kind == TokenType::Expand);
    if config.enforce_braced_expr
        && !arg.is_braced
        && !arg.is_quoted
        && !has_expansion
        && expr_args.contains(&i)
    {
        maybe_space(parts, false, config.space_between_braces);
        parts.push(format!("{{{raw}}}"));
        return true;
    }
    if arg.is_braced && expr_args.contains(&i) {
        let indent_len = config.make_indent(indent_level).len();
        let mut col = current_col(parts, indent_len);
        if !parts.is_empty() {
            col += 1;
        }
        let trailing = estimate_trailing_len(args, i + 1);
        if col + raw.len() + trailing > config.max_line_length
            && let Some(wrapped) = wrap_braced_expr(&arg.text, config, indent_level)
        {
            maybe_space(parts, false, config.space_between_braces);
            parts.push(format!("{{{wrapped}}}"));
            return true;
        }
    }
    if !parts.is_empty() {
        parts.push(" ".to_owned());
    }
    parts.push(raw);
    true
}

/// Reconstruct a single command as formatted text.
fn reconstruct_command(
    sm: &SourceMap,
    cmd: &ParsedCommand,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    indent: &str,
    indent_level: usize,
) -> String {
    if cmd.name == "switch" {
        return reconstruct_switch(sm, cmd, config, indent);
    }

    let expr_args = identify_expr_args(cmd, registry);
    let spec_traits = registry.get(&cmd.name).map(|s| s.traits);
    let never_inline = spec_traits.is_some_and(|t| t.contains(Traits::NEVER_INLINE_BODY));

    let mut parts: Vec<String> = Vec::new();
    let mut in_brace_chain = false;

    // `enforceBracedExpr`, concatenating form: a command whose entire argument
    // tail is one expression (`expr $a + $b`) — brace the joined tail
    // (`expr {$a + $b}`) rather than the single marked argument, which would
    // corrupt it (RUST_ISSUE_133). Bounded-expression commands fall through to
    // the per-argument bracing in `append_word_arg`.
    if config.enforce_braced_expr
        && spec_traits.is_some_and(|t| t.contains(Traits::EXPR_CONCATENATES_ARGS))
        && let Some(braced) = concat_expr_parts(sm, cmd, config)
    {
        return finish_command_line(&braced, config, indent, indent_level);
    }

    for (i, arg) in cmd.args.iter().enumerate() {
        match arg.kind {
            ArgKind::Body if arg.formatted_body.is_some() => {
                let body = arg.formatted_body.as_deref().unwrap();
                let current_line_len = indent.len() + parts.concat().len();
                // The body's leading space depends on the running
                // brace chain; apply it here, then delegate the
                // body rendering.
                maybe_space(&mut parts, in_brace_chain, config.space_between_braces);
                append_body_no_space(
                    &mut parts,
                    body,
                    config,
                    indent,
                    current_line_len,
                    never_inline,
                );
                in_brace_chain = true;
            }
            ArgKind::LambdaLiteral if arg.formatted_body.is_some() => {
                let current_line_len = indent.len() + parts.concat().len();
                maybe_space(&mut parts, in_brace_chain, config.space_between_braces);
                parts.push(render_lambda_literal_arg(
                    sm,
                    arg,
                    config,
                    indent,
                    current_line_len,
                    never_inline,
                ));
                in_brace_chain = true;
            }
            ArgKind::ParamList => {
                maybe_space(&mut parts, false, config.space_between_braces);
                parts.push(normalise_param_list(&arg.text));
                in_brace_chain = false;
            }
            ArgKind::Keyword => {
                maybe_space(&mut parts, false, config.space_between_braces);
                parts.push(arg.text.clone());
                in_brace_chain = false;
            }
            _ => {
                if append_word_arg(
                    sm,
                    &cmd.args,
                    i,
                    &expr_args,
                    config,
                    indent_level,
                    &mut parts,
                ) {
                    in_brace_chain = false;
                }
            }
        }
    }

    finish_command_line(&parts, config, indent, indent_level)
}

/// Concatenate `parts` under `indent` and apply the long-line split. Shared by
/// the normal command path and the `enforceBracedExpr` concatenating path.
fn finish_command_line(
    parts: &[String],
    config: &FormatterConfig,
    indent: &str,
    indent_level: usize,
) -> String {
    let line = format!("{indent}{}", parts.concat());

    // Split the first line if it exceeds max_line_length.
    let first_nl = line.find('\n');
    let first_line = match first_nl {
        Some(nl) => &line[..nl],
        None => &line,
    };
    if first_line.len() > config.max_line_length {
        let cont_indent = config.make_indent(indent_level + 1);
        if let Some(split) = split_long_line(first_line, config, &cont_indent) {
            return match first_nl {
                Some(nl) => format!("{split}{}", &line[nl..]),
                None => split,
            };
        }
    }
    line
}

/// Build the `parts` for an `enforceBracedExpr` command whose entire argument
/// tail is one expression (`EXPR_CONCATENATES_ARGS`). Returns `None` when there
/// is nothing to brace — no arguments, or the tail is already a single braced
/// `{ … }` word (which the normal path renders, keeping its long-line wrap).
fn concat_expr_parts(
    sm: &SourceMap,
    cmd: &ParsedCommand,
    config: &FormatterConfig,
) -> Option<Vec<String>> {
    let args = &cmd.args;
    if args.len() < 2 {
        return None; // bare `expr` with no operands
    }
    if args.len() == 2 && args[1].is_braced {
        return None; // already `expr {…}`
    }
    // Never brace a tail containing an expansion: `expr {*}$pieces` expands the
    // list before evaluating, and `expr {{*}$pieces}` would demote `{*}` to
    // literal text, breaking the expression (Codex review). Leave such a
    // command untouched.
    if args[1..]
        .iter()
        .any(|a| a.tokens.iter().any(|t| t.kind == TokenType::Expand))
    {
        return None;
    }
    let joined = args[1..]
        .iter()
        .map(|a| reconstruct_arg(sm, a, config.enforce_braced_variables))
        .collect::<Vec<_>>()
        .join(" ");
    Some(vec![
        args[0].text.clone(),
        " ".to_owned(),
        format!("{{{joined}}}"),
    ])
}

/// Reconstruct a `switch` command, handling the braced body form.
fn reconstruct_switch(
    sm: &SourceMap,
    cmd: &ParsedCommand,
    config: &FormatterConfig,
    indent: &str,
) -> String {
    // The switch body is the last braced arg; recompute its
    // formatting through `format_switch_body` (parsed below by
    // re-deriving from the registry-marked Body arg or the last
    // braced arg).
    let mut parts: Vec<String> = Vec::new();
    for arg in &cmd.args {
        if arg.kind == ArgKind::Body && arg.is_braced {
            let formatted = arg.formatted_body.clone().unwrap_or_default();
            if !parts.is_empty() {
                parts.push(" ".to_owned());
            }
            if formatted.trim().is_empty() {
                parts.push("{}".to_owned());
            } else {
                parts.push("{\n".to_owned());
                parts.push(formatted);
                parts.push(format!("\n{indent}}}"));
            }
        } else {
            let raw = reconstruct_arg(sm, arg, config.enforce_braced_variables);
            if !parts.is_empty() {
                parts.push(" ".to_owned());
            }
            parts.push(raw);
        }
    }
    format!("{indent}{}", parts.concat())
}

// Main entry points

/// Format a Tcl script body at the given indent level.  The core
/// recursive function.
/// Format a script body at `indent_level`, applying every engine
/// rule (comments, switch bodies, recursion, long-line wrapping, …).
/// [`format_tcl`] calls this at level 0 for a whole document; range
/// formatting calls it for a line slice at the slice's brace depth so
/// both paths share identical layout rules.
pub(crate) fn format_body(
    source: &str,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    indent_level: usize,
) -> String {
    let sm = SourceMap::new(source);
    let Ok(tokens) =
        Lexer::with_source_map(SourceMap::new(source), config.lexer_config).tokenise_all()
    else {
        return source.to_owned();
    };
    let (mut commands, trailing_comments) = parse_commands(source, &sm, &tokens);

    let indent = config.make_indent(indent_level);
    let inner_level = indent_level + 1;
    let mut lines: Vec<String> = Vec::new();

    for i in 0..commands.len() {
        let blank_count = compute_blank_lines(&commands, i, config);
        for _ in 0..blank_count {
            lines.push(String::new());
        }

        // Preceding comments.
        let comments = commands[i].preceding_comments.clone();
        for comment in &comments {
            emit_comment_lines(comment, config, &indent, indent_level, &mut lines);
        }

        identify_body_args(&mut commands[i], registry);

        // Switch needs its body formatted from arg.text via the
        // pattern/body splitter; other bodies recurse normally.
        let is_switch = commands[i].name == "switch";
        let arg_count = commands[i].args.len();
        for a in 0..arg_count {
            if commands[i].args[a].kind == ArgKind::Body && commands[i].args[a].is_braced {
                let body_text = commands[i].args[a].text.clone();
                let formatted = if is_switch {
                    format_switch_body(&body_text, config, registry, inner_level)
                } else {
                    format_body(&body_text, config, registry, inner_level)
                };
                commands[i].args[a].formatted_body = Some(formatted);
            } else if commands[i].args[a].kind == ArgKind::LambdaLiteral
                && commands[i].args[a].is_braced
                && let Some(&tok) = commands[i].args[a].tokens.first()
                && let Some(elems) = split_lambda_literal_decoded(source, tok)
                && let Some(body_text) = elems.body
            {
                // Format only the real, *decoded* body element —
                // `render_lambda_literal_arg` re-derives the parameter list /
                // namespace elements from the original source at
                // reconstruction time and reassembles them. Decoding first
                // (rather than reformatting the raw source spelling) matters
                // for a non-literal (bare/quoted) body: its backslash escapes
                // must be collapsed before the result is parsed as a script,
                // exactly as Tcl's own list-then-script evaluation would
                // (codex review of #954's follow-up).
                let formatted = format_body(&body_text, config, registry, inner_level);
                commands[i].args[a].formatted_body = Some(formatted);
            }
        }

        let line = reconstruct_command(&sm, &commands[i], config, registry, &indent, indent_level);

        // Keep `a; b` on one line when the user disabled
        // `replace_semicolons_with_newlines`, but only in the simple case:
        // the previous command ended with a bare `;`, neither this command
        // nor the previous rendered line spans multiple lines, and this
        // command has no leading comment/blank that would force a break
        // (RUST_ISSUE_133). Any other shape falls back to the default
        // one-command-per-line layout.
        let joinable = !config.replace_semicolons_with_newlines
            && i > 0
            && commands[i - 1].terminated_by_semicolon
            && commands[i].preceding_comments.is_empty()
            && !line.contains('\n')
            && lines.last().is_some_and(|prev| !prev.contains('\n'));
        if joinable {
            let prev = lines.last_mut().expect("previous line exists");
            let trimmed = line.trim_start();
            prev.push_str("; ");
            prev.push_str(trimmed);
        } else {
            lines.push(line);
        }
    }

    for comment in &trailing_comments {
        emit_comment_lines(comment, config, &indent, indent_level, &mut lines);
    }

    lines.join("\n")
}

/// Emit a comment into `out`, either re-indented to the code column
/// (`align_comments_to_code`, the default) or at its original source column
/// (`RUST_ISSUE_133`). The commented-code long-line split only applies when
/// aligning — preserving the author's column means leaving the line as-is.
fn emit_comment_lines(
    comment: &CommentLine,
    config: &FormatterConfig,
    indent: &str,
    indent_level: usize,
    out: &mut Vec<String>,
) {
    let formatted = format_comment(&comment.text, config);
    if config.align_comments_to_code {
        match split_commented_code(&formatted, config, indent, indent_level) {
            Some(split) => out.push(split),
            None => out.push(format!("{indent}{formatted}")),
        }
    } else {
        let prefix = comment.orig_indent.as_deref().unwrap_or(indent);
        out.push(format!("{prefix}{formatted}"));
    }
}

/// Trim trailing whitespace from each line **except** lines whose terminating
/// newline sits inside a multi-line braced (`{…}`) or double-quoted (`"…"`)
/// word — i.e. inside string data. Trimming such a line changes the value of
/// the literal (`set x {line1␠␠␠\nline2}` must keep the spaces after `line1`),
/// which would violate "the formatter never changes semantics" (`RUST_ISSUE_037`).
///
/// A line is safe to trim when, after scanning it, the running brace depth is
/// zero and no double-quoted word is open — the newline is then a structural
/// (command) separator, not part of a literal. The scan carries brace / quote /
/// backslash state across lines and treats a `#`-first line at depth 0 as a
/// comment (whose braces/quotes don't count), matching `mod::brace_delta`.
///
/// This covers both braced and double-quoted multi-line words. A lexer-token
/// scan would miss the quoted case: `"…"` words tokenise as `ESC` runs rather
/// than a single `Str` span, so the running brace/quote scan is what satisfies
/// the issue's braced-*and-quoted* requirement.
pub(crate) fn trim_trailing_ws_preserving_literals(text: &str) -> String {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<&str> = Vec::with_capacity(lines.len());
    for line in &lines {
        let in_comment = line.trim_start().starts_with('#') && depth == 0 && !in_string;
        for c in line.chars() {
            if escaped {
                escaped = false;
                continue;
            }
            match c {
                '\\' => escaped = true,
                _ if in_comment => {}
                '"' if depth == 0 => in_string = !in_string,
                _ if in_string => {}
                '#' if depth == 0 => {
                    // A `#` mid-line only starts a comment at a command start;
                    // the conservative reuse of `brace_delta`'s rule (line
                    // begins with `#`) is already captured by `in_comment`
                    // above, so an inline `#` here is literal — no-op.
                }
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        // `in_comment` and a line-continuation `\` end at the newline for the
        // trim decision; only an open brace/quote makes the newline part of a
        // literal.
        let safe_to_trim = depth == 0 && !in_string;
        if safe_to_trim {
            out.push(line.trim_end());
        } else {
            out.push(line);
        }
    }
    out.join("\n")
}

/// Format a Tcl source string.  Pure function: source in,
/// formatted source out.
#[must_use]
pub fn format_tcl(source: &str, config: &FormatterConfig, registry: &CommandRegistry) -> String {
    let mut result = format_body(source, config, registry, 0);

    if config.trim_trailing_whitespace {
        result = trim_trailing_ws_preserving_literals(&result);
    }
    if config.ensure_final_newline && !result.ends_with('\n') {
        result.push('\n');
    }
    if config.line_ending != "\n" {
        result = result.replace('\n', &config.line_ending);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        format_tcl(src, &FormatterConfig::default(), &registry)
    }

    fn fmt_with(src: &str, config: &FormatterConfig) -> String {
        let registry = CommandRegistry::build_default();
        format_tcl(src, config, &registry)
    }

    #[test]
    fn count_body_commands_counts_top_level_statements() {
        assert_eq!(count_body_commands(""), 0);
        assert_eq!(count_body_commands("  \n "), 0);
        assert_eq!(count_body_commands("puts hi"), 1);
        assert_eq!(count_body_commands("puts a\nputs b"), 2);
        assert_eq!(count_body_commands("puts a; puts b"), 2);
        // A `;` inside a nested brace/bracket is not a separator.
        assert_eq!(count_body_commands("set x {a; b}"), 1);
        assert_eq!(count_body_commands("set x [expr {1; 2}]"), 1);
    }

    #[test]
    fn min_body_commands_keeps_single_command_body_inline() {
        // RUST_ISSUE_133: with expansion on but the threshold at 2, a
        // one-command body stays inline while a two-command body expands.
        // `catch` has an inline-able body (not NEVER_INLINE_BODY), so it
        // exercises the `expand_single_line_bodies` path.
        let config = FormatterConfig {
            expand_single_line_bodies: true,
            min_body_commands_for_expansion: 2,
            ..FormatterConfig::default()
        };
        let one = fmt_with("catch { puts hi }\n", &config);
        assert!(
            one.contains("{ puts hi }") && !one.contains("puts hi\n"),
            "single-command body should stay inline: {one:?}"
        );
        let two = fmt_with("catch { puts a; puts b }\n", &config);
        assert!(
            two.contains("puts a\n") && two.contains("puts b\n"),
            "two-command body should expand: {two:?}"
        );
    }

    #[test]
    fn enforce_braced_expr_braces_expr_and_conditions() {
        // RUST_ISSUE_133: `enforceBracedExpr` braces bare expressions.
        let config = FormatterConfig {
            enforce_braced_expr: true,
            ..FormatterConfig::default()
        };
        // `expr` concatenates its whole argument tail into one expression, so
        // the entire tail is braced (not just arg 0, which would corrupt it).
        assert_eq!(
            fmt_with("expr $a + $b\n", &config),
            "expr {$a + $b}\n",
            "expr tail must be braced as one group"
        );
        // A single-argument condition (bounded expr) is braced in place.
        assert_eq!(
            fmt_with("while $x {\n    incr y\n}\n", &config),
            "while {$x} {\n    incr y\n}\n"
        );
        // Already-braced expressions are left unchanged (idempotent).
        assert_eq!(fmt_with("expr {$a + $b}\n", &config), "expr {$a + $b}\n");
        assert_eq!(
            fmt_with("if {$x} {\n    puts hi\n}\n", &config),
            "if {$x} {\n    puts hi\n}\n"
        );
    }

    #[test]
    fn align_comments_to_code_toggles_reindentation() {
        // Default (true): a mis-indented standalone comment is re-indented to
        // the code column.
        let src = "proc f {} {\n        # over-indented\n    set x 1\n}\n";
        let aligned = fmt(src);
        assert!(
            aligned.contains("\n    # over-indented\n"),
            "default should re-indent comment to code column: {aligned:?}"
        );
        // Disabled: the comment keeps its original column.
        let config = FormatterConfig {
            align_comments_to_code: false,
            ..FormatterConfig::default()
        };
        let preserved = fmt_with(src, &config);
        assert!(
            preserved.contains("\n        # over-indented\n"),
            "disabled should preserve the original comment column: {preserved:?}"
        );
    }

    #[test]
    fn enforce_braced_expr_off_by_default() {
        // FP-guard: with the default config (off) bare exprs are untouched.
        assert_eq!(fmt("expr $a + $b\n"), "expr $a + $b\n");
    }

    #[test]
    fn enforce_braced_expr_preserves_expansion_tail() {
        // Codex review: `expr {*}$pieces` expands the list before evaluating;
        // bracing it (`expr {{*}$pieces}`) would demote `{*}` to literal text
        // and break the expression. The rewrite must leave it alone.
        let config = FormatterConfig {
            enforce_braced_expr: true,
            ..FormatterConfig::default()
        };
        assert_eq!(
            fmt_with("expr {*}$pieces\n", &config),
            "expr {*}$pieces\n",
            "an expansion tail must not be braced"
        );
    }

    #[test]
    fn replace_semicolons_disabled_keeps_commands_inline() {
        // RUST_ISSUE_133: default splits `a; b` onto two lines; disabling the
        // setting keeps them on one line joined by `; `.
        let split = fmt("puts a; puts b\n");
        assert!(
            split.contains("puts a\nputs b"),
            "default should split on `;`: {split:?}"
        );
        let config = FormatterConfig {
            replace_semicolons_with_newlines: false,
            ..FormatterConfig::default()
        };
        let joined = fmt_with("puts a; puts b\n", &config);
        assert!(
            joined.contains("puts a; puts b"),
            "disabled should keep `;`-joined commands inline: {joined:?}"
        );
    }

    #[test]
    fn bare_dollar_in_quoted_string_is_not_brace_wrapped() {
        // RUST_ISSUE_103: a lone `$` inside a double-quoted string is literal
        // data; formatting must not turn `"cost: $"` into `"cost: {$}"`.
        let out = fmt("puts \"cost: $\"\n");
        assert!(out.contains("\"cost: $\""), "{out:?}");
        assert!(!out.contains("{$}"), "bare $ was brace-wrapped: {out:?}");
    }

    #[test]
    fn backslash_newline_keeps_preceding_spaces_in_quoted_string() {
        // RUST_ISSUE_104: Tcl replaces `\<nl><following-ws>` with a single
        // space but keeps whitespace *before* the backslash. `"a \<nl> b"` is
        // the value `a  b` (two spaces) — the pre-backslash space must survive.
        let out = fmt("puts \"a \\\n b\"\n");
        assert!(
            out.contains("\"a  b\""),
            "expected two spaces preserved: {out:?}"
        );
    }

    #[test]
    fn backslash_newline_collapse_handles_crlf() {
        // A `\<CR><LF>` continuation collapses exactly like `\<LF>` (the old
        // LF-only normalisation left the `\` and CRLF in place). In a quoted
        // string the pre-backslash space is data and survives (`a  b`)…
        let out = fmt("puts \"a \\\r\n b\"\n");
        assert!(
            out.contains("\"a  b\""),
            "expected CRLF continuation collapsed with data space kept: {out:?}"
        );
        // …while in a command substitution the whole run is one separator.
        let out = fmt("set y [string cat a \\\r\n  b]\n");
        assert!(
            out.contains("[string cat a b]"),
            "expected CRLF continuation collapsed to one separator: {out:?}"
        );
    }

    #[test]
    fn trim_preserves_multiline_braced_string_interior() {
        // RUST_ISSUE_037: trailing spaces *inside* a multi-line braced word are
        // string data and must survive the trailing-whitespace pass.
        let input = "set x {line1   \nline2}\n";
        let out = fmt(input);
        assert!(
            out.contains("line1   \n"),
            "trailing spaces inside the braced string were stripped:\n{out:?}"
        );
    }

    #[test]
    fn trailing_trim_preserves_spaces_inside_multiline_braces() {
        // The spaces before the newline live inside the braced word, so they
        // are part of the Tcl string value — trimming them would change the
        // runtime string, not just presentation (default trim is enabled).
        let out = fmt("set x {foo   \n   bar}\n");
        assert!(
            out.contains("foo   \n"),
            "significant whitespace inside braces preserved: {out:?}",
        );
    }

    #[test]
    fn trim_preserves_multiline_quoted_string_interior() {
        let input = "set x \"line1   \nline2\"\n";
        let out = fmt(input);
        assert!(
            out.contains("line1   \n"),
            "trailing spaces inside the quoted string were stripped:\n{out:?}"
        );
    }

    #[test]
    fn trim_still_strips_structural_trailing_whitespace() {
        // Ordinary code lines (outside any literal) are still trimmed.
        let out = trim_trailing_ws_preserving_literals("set a 1   \nset b 2   ");
        assert_eq!(out, "set a 1\nset b 2");
    }

    #[test]
    fn trailing_trim_still_trims_ordinary_code_lines() {
        // A trailing run of spaces on a real code line (not inside a literal)
        // is still trimmed.
        let out = fmt("set a 1   \nset b 2\n");
        assert_eq!(out, "set a 1\nset b 2\n");
    }

    #[test]
    fn irule_brace_chain_gets_a_space() {
        // TMM accepts `}{` (e.g. `if {expr}{body}`); with the f5-irules lexer
        // preset the formatter parses it as two words and re-emits `} {`.
        let registry = CommandRegistry::build_default();
        let config = FormatterConfig {
            lexer_config: tcl_lexer::LexerConfig::for_dialect("f5-irules"),
            ..FormatterConfig::default()
        };
        let out = format_tcl("if { 1 }{\n    pool p\n}\n", &config, &registry);
        assert!(!out.contains("}{"), "left `}}{{` unfixed:\n{out}");
        assert!(out.contains("} {"), "no `}} {{`:\n{out}");

        // Plain-Tcl default preset does not synthesise the separator, so it
        // leaves the (invalid-in-stock-Tcl) input alone rather than inventing a
        // parse — no accidental change to non-iRule formatting.
        let plain = format_tcl(
            "if { 1 }{\n    pool p\n}\n",
            &FormatterConfig::default(),
            &registry,
        );
        assert!(
            plain.contains("}{"),
            "default preset should not rewrite `}}{{`"
        );
    }

    /// Each `(input, expected)` pair is the expected formatted output
    /// for the same input.
    fn check(input: &str, expected: &str) {
        let got = fmt(input);
        assert_eq!(
            got, expected,
            "\ninput:    {input:?}\ngot:      {got:?}\nexpected: {expected:?}"
        );
    }

    #[test]
    fn simple_command() {
        check("puts hi\n", "puts hi\n");
    }

    #[test]
    fn proc_body_indents() {
        check("proc f {} {\nset x 1\n}\n", "proc f {} {\n    set x 1\n}\n");
    }

    #[test]
    fn if_else() {
        check(
            "if {$x} {\nputs a\n} else {\nputs b\n}\n",
            "if {$x} {\n    puts a\n} else {\n    puts b\n}\n",
        );
    }

    #[test]
    fn nested_indentation() {
        check(
            "proc f {} {\nif {1} {\nset x 1\n}\n}\n",
            "proc f {} {\n    if {1} {\n        set x 1\n    }\n}\n",
        );
    }

    #[test]
    fn comments_preserved() {
        check("#hello\n# spaced\nputs hi\n", "#hello\n# spaced\nputs hi\n");
    }

    #[test]
    fn collapses_blank_lines_to_max() {
        check("set x 1\n\n\n\nset y 2\n", "set x 1\n\n\nset y 2\n");
    }

    #[test]
    fn switch_body() {
        check(
            "switch $x {\na {\nputs 1\n}\nb {\nputs 2\n}\n}\n",
            "switch $x {\n    a {\n        puts 1\n    }\n    b {\n        puts 2\n    }\n}\n",
        );
    }

    #[test]
    fn switch_body_preserves_comments() {
        // RUST_ISSUE_038: a comment line inside a switch body must survive
        // formatting rather than being silently deleted.
        let out = fmt("switch $x {\n# note\na { puts 1 }\n}\n");
        assert!(
            out.contains("# note"),
            "switch-body comment was deleted:\n{out}"
        );
        assert!(out.contains("puts 1"), "arm body lost:\n{out}");
    }

    #[test]
    fn control_flow_always_expands() {
        check("if {$x} { return }\n", "if {$x} {\n    return\n}\n");
    }

    #[test]
    fn while_body() {
        check("while {$x} {\nincr x\n}\n", "while {$x} {\n    incr x\n}\n");
    }

    #[test]
    fn foreach_body() {
        check(
            "foreach i {1 2 3} {\nputs $i\n}\n",
            "foreach i {1 2 3} {\n    puts $i\n}\n",
        );
    }

    #[test]
    fn param_list_normalised() {
        check(
            "proc f {a    b   c} {\nreturn\n}\n",
            "proc f {a b c} {\n    return\n}\n",
        );
    }

    #[test]
    fn quoted_string_preserved() {
        check("puts \"hello $name\"\n", "puts \"hello $name\"\n");
    }

    /// Issue #954: `apply`'s lambda-literal argument (`ArgRole::LambdaLiteral`,
    /// not `Body`) must have its real body element reformatted — not the
    /// whole `{argList} {body}` blob re-segmented as one script (which
    /// misreads the parameter word as a command name and never reaches the
    /// real body). The parameter list is normalised the same way a `proc`'s
    /// is (bare `dir` → braced `{dir}`), matching `param_list_normalised`.
    #[test]
    fn apply_lambda_body_indents_and_params_normalise() {
        check(
            "apply {dir {\nputs    $dir\nset x 1\n}} /tmp\n",
            "apply {{dir} {\n    puts $dir\n    set x 1\n}} /tmp\n",
        );
    }

    /// The optional namespace element (lambda literal position 2) survives
    /// reassembly untouched. A short single-command body inlines (matching
    /// ordinary body formatting), so this also confirms the inline path
    /// reassembles the namespace element correctly.
    #[test]
    fn apply_lambda_namespace_element_preserved() {
        check(
            "apply {dir {\nputs    $dir\n} ::foo} /tmp\n",
            "apply {{dir} { puts $dir } ::foo} /tmp\n",
        );
    }

    /// Codex review of #954's follow-up: a bare body element's backslash
    /// escape must be decoded before reformatting, not reformatted from its
    /// raw source spelling — `puts\ hi`'s real runtime body is the two-word
    /// command `puts hi`, not one word containing a literal backslash.
    #[test]
    fn apply_lambda_body_with_backslash_escape_decodes() {
        check(r"apply {{} puts\ hi}", "apply {{} { puts hi }}\n");
    }

    #[test]
    fn force_split_long_line() {
        check(
            "mycommand argumentone argumenttwo argumentthree argumentfour argumentfive argumentsix argumentseven argumenteight argumentnine\n",
            "mycommand argumentone argumenttwo argumentthree argumentfour argumentfive argumentsix argumentseven argumenteight \\\n    argumentnine\n",
        );
    }

    #[test]
    fn force_expr_wrap() {
        check(
            "if {$variableaaaa == 1 && $variablebbbb == 2 && $variablecccc == 3 && $variabledddd == 4 && $variableeeee == 5 && $variableffff == 6} {\nputs hi\n}\n",
            "if {\n    $variableaaaa == 1\n    && $variablebbbb == 2\n    && $variablecccc == 3\n    && $variabledddd == 4\n    && $variableeeee == 5\n    && $variableffff == 6\n} {\n    puts hi\n}\n",
        );
    }

    #[test]
    fn semicolon_splits_commands() {
        check("set x 1; set y 2\n", "set x 1\nset y 2\n");
    }

    #[test]
    fn empty_proc_body() {
        check("proc f {} {}\n", "proc f {} {}\n");
    }

    #[test]
    fn try_finally() {
        check(
            "try {\nfoo\n} finally {\nbar\n}\n",
            "try {\n    foo\n} finally {\n    bar\n}\n",
        );
    }

    #[test]
    fn command_substitution_preserved() {
        check("set y [expr {1+2}]\n", "set y [expr {1+2}]\n");
    }

    #[test]
    fn expand_prefix_preserved() {
        check("puts {*}$args\n", "puts {*}$args\n");
    }

    #[test]
    fn multi_hash_comment() {
        check("## section\nputs hi\n", "## section\nputs hi\n");
    }

    #[test]
    fn trailing_comment() {
        check("puts hi\n# trailing\n", "puts hi\n# trailing\n");
    }

    #[test]
    fn deeply_nested_bodies() {
        check(
            "proc f {} {\nforeach x $list {\nif {$x} {\nputs $x\n}\n}\n}\n",
            "proc f {} {\n    foreach x $list {\n        if {$x} {\n            puts $x\n        }\n    }\n}\n",
        );
    }

    #[test]
    fn enforce_braced_variables_rewrites_dollar_refs() {
        let registry = CommandRegistry::build_default();
        let cfg = FormatterConfig {
            enforce_braced_variables: true,
            ..FormatterConfig::default()
        };
        assert_eq!(format_tcl("puts $x\n", &cfg, &registry), "puts ${x}\n");
    }

    #[test]
    fn blank_lines_between_procs() {
        check(
            "proc a {} {\nreturn\n}\nproc b {} {\nreturn\n}\n",
            "proc a {} {\n    return\n}\n\nproc b {} {\n    return\n}\n",
        );
    }
}
