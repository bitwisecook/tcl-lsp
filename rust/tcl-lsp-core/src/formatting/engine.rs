//! Core formatting engine — Rust port of
//! `core/formatting/engine.py`.
//!
//! Parses Tcl source into commands, identifies body / expr /
//! param-list arguments via the registry, recursively formats
//! bodies, and reconstructs the output (K&R braces, blank-line
//! policy, comment normalisation, switch bodies, long-line
//! backslash splitting, and `&&`/`||` expression wrapping).
//!
//! The entry point is [`format_tcl`]; everything else is the
//! private machinery [`format_tcl`] drives.

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
    formatted_body: Option<String>,
}

/// A single Tcl command with its arguments, ready for reformatting.
struct ParsedCommand {
    name: String,
    args: Vec<CommandArg>,
    preceding_comments: Vec<String>,
    preceding_blank_lines: usize,
}

// ---------------------------------------------------------------------------
// Token → raw source reconstruction
// ---------------------------------------------------------------------------

/// Collapse `\<newline>` continuations to a single space.  Inside
/// `[]` and `"…"` this is semantics-preserving and keeps the
/// formatter idempotent.
fn normalise_backslash_newline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            // Skip surrounding [ \t] then the backslash-newline.
            while out.ends_with([' ', '\t']) {
                out.pop();
            }
            i += 2;
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            out.push(' ');
        } else {
            // Copy one UTF-8 char.
            let ch_len = utf8_len(bytes[i]);
            out.push_str(&text[i..i + ch_len]);
            i += ch_len;
        }
    }
    out
}

/// Byte length of the UTF-8 char whose lead byte is `b`.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// Rebuild source text from a single token, re-adding delimiters.
/// Mirrors Python's `_reconstruct_raw`.
fn reconstruct_raw(sm: &SourceMap, tok: Token) -> String {
    match tok.kind {
        TokenType::Str => format!("{{{}}}", sm.token_text(tok)),
        TokenType::Cmd => format!("[{}]", normalise_backslash_newline(sm.token_text(tok))),
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
            raw.push_str(&reconstruct_raw(sm, tok));
        }
    }
    if arg.is_quoted {
        format!("\"{raw}\"")
    } else {
        raw
    }
}

/// Normalise whitespace in a braced parameter list, joining
/// top-level elements with single spaces.  Mirrors
/// `_normalize_param_list`.
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

// ---------------------------------------------------------------------------
// Command parsing
// ---------------------------------------------------------------------------

/// Count blank lines represented by an EOL token's text (each
/// newline beyond the first is a blank line).
fn count_newlines(text: &str) -> usize {
    text.matches('\n').count().saturating_sub(1)
}

/// Parse Tcl source into structured commands plus any trailing
/// comments.  Mirrors Python's `parse_commands`.
fn parse_commands(
    source: &str,
    sm: &SourceMap,
    tokens: &[Token],
) -> (Vec<ParsedCommand>, Vec<String>) {
    let mut commands: Vec<ParsedCommand> = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();
    let mut pending_blank_lines = 0usize;

    let mut argv: Vec<CommandArg> = Vec::new();
    let mut prev_type = TokenType::Eol;

    let flush = |argv: &mut Vec<CommandArg>,
                 pending_comments: &mut Vec<String>,
                 pending_blank_lines: usize,
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
        });
    };

    for &tok in tokens {
        match tok.kind {
            TokenType::Eof => break,
            TokenType::Comment => {
                pending_comments.push(sm.text(tok.span).to_owned());
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
                    flush(
                        &mut argv,
                        &mut pending_comments,
                        pending_blank_lines,
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
        &mut commands,
    );
    (commands, pending_comments)
}

// ---------------------------------------------------------------------------
// Body / expr / param-list argument identification
// ---------------------------------------------------------------------------

/// Mark body / keyword / param-list arguments in place.  Mirrors
/// `_identify_body_args`.
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
    // Post-name argument texts (Python's `args = cmd.args[1:]`),
    // owned so the immutable borrow of `cmd.args` is released
    // before the role-driven mutation below.
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
    for idx in body_indices {
        let actual = idx + 1; // +1 for the command-name slot.
        if actual < cmd.args.len() && cmd.args[actual].is_braced {
            cmd.args[actual].kind = ArgKind::Body;
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

/// Indices into `cmd.args` of braced expression arguments.  Mirrors
/// `_identify_expr_args`.
fn identify_expr_args(cmd: &ParsedCommand, registry: &CommandRegistry) -> Vec<usize> {
    let arg_texts: Vec<&str> = cmd.args.iter().skip(1).map(|a| a.text.as_str()).collect();
    registry
        .arg_indices_for_role(&cmd.name, &arg_texts, ArgRole::Expr)
        .into_iter()
        .map(|idx| idx + 1)
        .collect()
}

// ---------------------------------------------------------------------------
// Comment formatting
// ---------------------------------------------------------------------------

/// Format a comment per config.  Mirrors `_format_comment`.
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
        format!("{hashes}{}", normalise_backslash_newline(after_hashes))
    } else if config.space_after_comment_hash {
        format!("{hashes}{}", after_hashes.trim_end())
    } else {
        format!("{hashes}{}", after_hashes.trim())
    }
}

// ---------------------------------------------------------------------------
// Blank-line computation
// ---------------------------------------------------------------------------

/// How many blank lines to insert before `commands[index]`.
/// Mirrors `_compute_blank_lines`.
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

// ---------------------------------------------------------------------------
// Switch body formatting
// ---------------------------------------------------------------------------

/// One element (pattern or body) of a `switch` body.
struct SwitchElem {
    tokens: Vec<Token>,
    text: String,
    is_braced: bool,
}

/// Format the braced body of a `switch` command (pattern/body
/// pairs).  Mirrors `_format_switch_body`.
fn format_switch_body(
    body_text: &str,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    indent_level: usize,
) -> String {
    let sm = SourceMap::new(body_text);
    let Ok(tokens) = Lexer::new(body_text).tokenise_all() else {
        return body_text.to_owned();
    };

    // Re-segment into elements (tokens, text, is_braced).
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
            TokenType::Comment => continue,
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
            });
        } else if let Some(e) = cur.as_mut() {
            e.tokens.push(tok);
            e.text.push_str(&text);
        } else {
            cur = Some(SwitchElem {
                tokens: vec![tok],
                text,
                is_braced: tok.kind == TokenType::Str,
            });
        }
        prev_type = tok.kind;
    }
    if let Some(e) = cur.take() {
        elements.push(e);
    }

    let indent = config.make_indent(indent_level);
    let inner_level = indent_level + 1;
    let mut lines: Vec<String> = Vec::new();
    let mut i = 0;
    while i < elements.len() {
        let pattern_raw: String = elements[i]
            .tokens
            .iter()
            .map(|&t| reconstruct_raw(&sm, t))
            .collect();
        if i + 1 < elements.len() {
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
                    .map(|&t| reconstruct_raw(&sm, t))
                    .collect();
                lines.push(format!("{indent}{pattern_raw} {body_raw}"));
            }
            i += 2;
        } else {
            lines.push(format!("{indent}{pattern_raw}"));
            i += 1;
        }
    }
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Long-line expression wrapping
// ---------------------------------------------------------------------------

/// Find top-level `&&` / `||` positions in an expression (not
/// nested in `[] {} () ""`).  Mirrors `_find_expr_break_points`.
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

/// Try to wrap a braced expression at `&&` / `||`.  Mirrors
/// `_wrap_braced_expr`; returns the wrapped inner text (no braces)
/// or `None`.
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
/// line.  Mirrors `_estimate_trailing_len`.
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

/// Current column from accumulated parts.  Mirrors `_current_col`.
fn current_col(parts: &[String], indent_len: usize) -> usize {
    let text: String = parts.concat();
    match text.rfind('\n') {
        None => indent_len + text.len(),
        Some(nl) => text.len() - nl - 1,
    }
}

// ---------------------------------------------------------------------------
// Backslash-continuation line splitting
// ---------------------------------------------------------------------------

/// Find `(index, bracket_depth)` of spaces where `\`-continuation
/// is safe.  Mirrors `_find_splittable_spaces`.
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

/// Find spaces inside double-quoted strings where `\<newline>` is
/// safe.  Mirrors `_find_quoted_string_spaces`.
fn find_quoted_string_spaces(text: &str, start: usize) -> Vec<usize> {
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
        if in_quotes {
            match ch {
                b'"' => in_quotes = false,
                b'[' => dbr += 1,
                b']' => dbr = (dbr - 1).max(0),
                b' ' if dbr == 0 => spaces.push(i),
                _ => {}
            }
        } else {
            match ch {
                b'{' => db += 1,
                b'}' => db = (db - 1).max(0),
                b'"' if db == 0 => {
                    in_quotes = true;
                    dbr = 0;
                }
                b'[' => dbr += 1,
                b']' => dbr = (dbr - 1).max(0),
                _ => {}
            }
        }
        i += 1;
    }
    spaces
}

/// Greedy line splitting at the given space positions.  Mirrors
/// `_greedy_split`.
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
        if let Some(lg) = last_good {
            commit_break(&mut segments, &mut seg_start, &mut last_good, lg);
        } else {
            return None;
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
/// breaks.  Mirrors `_split_long_line`.
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

    if segments.as_ref().map_or(true, |s| s.len() < 2) {
        let quoted = find_quoted_string_spaces(line, indent_len);
        if !quoted.is_empty() {
            let mut all: std::collections::BTreeSet<usize> =
                all_spaces.iter().map(|&(p, _)| p).collect();
            all.extend(quoted);
            let positions: Vec<usize> = all.into_iter().collect();
            segments = greedy_split(line, &positions, max_len, cont_indent);
        }
        if segments.as_ref().map_or(true, |s| s.len() < 2) {
            return None;
        }
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
/// continuation.  Mirrors `_split_commented_code`.
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

// ---------------------------------------------------------------------------
// Inline body detection
// ---------------------------------------------------------------------------

/// Whether a body is short enough to keep on one line.  Mirrors
/// `_body_can_be_inline`.
fn body_can_be_inline(
    body_text: &str,
    config: &FormatterConfig,
    current_line_len: usize,
    never_inline: bool,
) -> bool {
    if config.expand_single_line_bodies {
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

// ---------------------------------------------------------------------------
// Command reconstruction
// ---------------------------------------------------------------------------

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
        let collapsed = normalise_backslash_newline(&raw).trim().to_owned();
        if collapsed.is_empty() {
            return false;
        }
        raw = collapsed;
    }
    if arg.is_braced && expr_args.contains(&i) {
        let indent_len = config.make_indent(indent_level).len();
        let mut col = current_col(parts, indent_len);
        if !parts.is_empty() {
            col += 1;
        }
        let trailing = estimate_trailing_len(args, i + 1);
        if col + raw.len() + trailing > config.max_line_length {
            if let Some(wrapped) = wrap_braced_expr(&arg.text, config, indent_level) {
                maybe_space(parts, false, config.space_between_braces);
                parts.push(format!("{{{wrapped}}}"));
                return true;
            }
        }
    }
    if !parts.is_empty() {
        parts.push(" ".to_owned());
    }
    parts.push(raw);
    true
}

/// Reconstruct a single command as formatted text.  Mirrors
/// `_reconstruct_command`.
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
    let never_inline = registry
        .get(&cmd.name)
        .is_some_and(|s| s.traits.contains(Traits::NEVER_INLINE_BODY));

    let mut parts: Vec<String> = Vec::new();
    let mut in_brace_chain = false;

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

/// Reconstruct a `switch` command, handling the braced body form.
/// Mirrors `_reconstruct_switch`.
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

// ---------------------------------------------------------------------------
// Main entry points
// ---------------------------------------------------------------------------

/// Format a Tcl script body at the given indent level.  The core
/// recursive function.  Mirrors `format_body`.
fn format_body(
    source: &str,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    indent_level: usize,
) -> String {
    let sm = SourceMap::new(source);
    let Ok(tokens) = Lexer::new(source).tokenise_all() else {
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
            let formatted = format_comment(comment, config);
            match split_commented_code(&formatted, config, &indent, indent_level) {
                Some(split) => lines.push(split),
                None => lines.push(format!("{indent}{formatted}")),
            }
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
            }
        }

        let line = reconstruct_command(&sm, &commands[i], config, registry, &indent, indent_level);
        lines.push(line);
    }

    for comment in &trailing_comments {
        let formatted = format_comment(comment, config);
        match split_commented_code(&formatted, config, &indent, indent_level) {
            Some(split) => lines.push(split),
            None => lines.push(format!("{indent}{formatted}")),
        }
    }

    lines.join("\n")
}

/// Format a Tcl source string.  Pure function: source in,
/// formatted source out.  Mirrors `format_tcl`.
#[must_use]
pub fn format_tcl(source: &str, config: &FormatterConfig, registry: &CommandRegistry) -> String {
    let mut result = format_body(source, config, registry, 0);

    if config.trim_trailing_whitespace {
        result = result
            .split('\n')
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n");
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

    /// Each `(input, expected)` pair is the verbatim output of the
    /// Python `format_tcl` reference for the same input.
    fn check(input: &str, expected: &str) {
        let got = fmt(input);
        assert_eq!(
            got, expected,
            "\ninput:    {input:?}\ngot:      {got:?}\nexpected: {expected:?}"
        );
    }

    #[test]
    fn parity_simple_command() {
        check("puts hi\n", "puts hi\n");
    }

    #[test]
    fn parity_proc_body_indents() {
        check("proc f {} {\nset x 1\n}\n", "proc f {} {\n    set x 1\n}\n");
    }

    #[test]
    fn parity_if_else() {
        check(
            "if {$x} {\nputs a\n} else {\nputs b\n}\n",
            "if {$x} {\n    puts a\n} else {\n    puts b\n}\n",
        );
    }

    #[test]
    fn parity_nested_indentation() {
        check(
            "proc f {} {\nif {1} {\nset x 1\n}\n}\n",
            "proc f {} {\n    if {1} {\n        set x 1\n    }\n}\n",
        );
    }

    #[test]
    fn parity_comments_preserved() {
        check("#hello\n# spaced\nputs hi\n", "#hello\n# spaced\nputs hi\n");
    }

    #[test]
    fn parity_collapses_blank_lines_to_max() {
        check("set x 1\n\n\n\nset y 2\n", "set x 1\n\n\nset y 2\n");
    }

    #[test]
    fn parity_switch_body() {
        check(
            "switch $x {\na {\nputs 1\n}\nb {\nputs 2\n}\n}\n",
            "switch $x {\n    a {\n        puts 1\n    }\n    b {\n        puts 2\n    }\n}\n",
        );
    }

    #[test]
    fn parity_control_flow_always_expands() {
        check("if {$x} { return }\n", "if {$x} {\n    return\n}\n");
    }

    #[test]
    fn parity_while_body() {
        check("while {$x} {\nincr x\n}\n", "while {$x} {\n    incr x\n}\n");
    }

    #[test]
    fn parity_foreach_body() {
        check(
            "foreach i {1 2 3} {\nputs $i\n}\n",
            "foreach i {1 2 3} {\n    puts $i\n}\n",
        );
    }

    #[test]
    fn parity_param_list_normalised() {
        check(
            "proc f {a    b   c} {\nreturn\n}\n",
            "proc f {a b c} {\n    return\n}\n",
        );
    }

    #[test]
    fn parity_quoted_string_preserved() {
        check("puts \"hello $name\"\n", "puts \"hello $name\"\n");
    }

    #[test]
    fn parity_force_split_long_line() {
        check(
            "mycommand argumentone argumenttwo argumentthree argumentfour argumentfive argumentsix argumentseven argumenteight argumentnine\n",
            "mycommand argumentone argumenttwo argumentthree argumentfour argumentfive argumentsix argumentseven argumenteight \\\n    argumentnine\n",
        );
    }

    #[test]
    fn parity_force_expr_wrap() {
        check(
            "if {$variableaaaa == 1 && $variablebbbb == 2 && $variablecccc == 3 && $variabledddd == 4 && $variableeeee == 5 && $variableffff == 6} {\nputs hi\n}\n",
            "if {\n    $variableaaaa == 1\n    && $variablebbbb == 2\n    && $variablecccc == 3\n    && $variabledddd == 4\n    && $variableeeee == 5\n    && $variableffff == 6\n} {\n    puts hi\n}\n",
        );
    }

    #[test]
    fn parity_semicolon_splits_commands() {
        check("set x 1; set y 2\n", "set x 1\nset y 2\n");
    }

    #[test]
    fn parity_empty_proc_body() {
        check("proc f {} {}\n", "proc f {} {}\n");
    }

    #[test]
    fn parity_try_finally() {
        check(
            "try {\nfoo\n} finally {\nbar\n}\n",
            "try {\n    foo\n} finally {\n    bar\n}\n",
        );
    }

    #[test]
    fn parity_command_substitution_preserved() {
        check("set y [expr {1+2}]\n", "set y [expr {1+2}]\n");
    }

    #[test]
    fn parity_expand_prefix_preserved() {
        check("puts {*}$args\n", "puts {*}$args\n");
    }

    #[test]
    fn parity_multi_hash_comment() {
        check("## section\nputs hi\n", "## section\nputs hi\n");
    }

    #[test]
    fn parity_trailing_comment() {
        check("puts hi\n# trailing\n", "puts hi\n# trailing\n");
    }

    #[test]
    fn parity_deeply_nested_bodies() {
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
    fn parity_blank_lines_between_procs() {
        check(
            "proc a {} {\nreturn\n}\nproc b {} {\nreturn\n}\n",
            "proc a {} {\n    return\n}\n\nproc b {} {\n    return\n}\n",
        );
    }
}
