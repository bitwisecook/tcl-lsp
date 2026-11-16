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

//! Wrapping long lines: expression break points and backslash continuations.

use super::super::config::FormatterConfig;
use super::{ArgKind, CommandArg, MAX_FORMAT_DEPTH};

// Long-line expression wrapping

/// Operator geometry from the shared expression lexer under the complete
/// selected source grammar. Literal terms and comments are never scanned again.
fn find_expr_break_points(text: &str, config: &FormatterConfig) -> Option<Vec<usize>> {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    use tcl_lexer::ExprTokenType as E;
    let (tokens, unknown) = tcl_lexer::tokenise_expr_checked_with_expression_grammar(
        text,
        &config.lexer_config().grammar_over(config.profile.grammar),
        config.profile.expr_grammar_base,
        config.profile.f5_core_expr_grammar(),
    );
    if unknown || tokens.iter().any(|token| token.kind == E::Comment) {
        return None;
    }
    let mut depth = 0_u32;
    let mut breaks = Vec::new();
    for token in tokens {
        match token.kind {
            E::ParenOpen => depth = depth.checked_add(1)?,
            E::ParenClose => depth = depth.checked_sub(1)?,
            E::Operator if depth == 0 && matches!(token.text.as_str(), "&&" | "||") => {
                breaks.push(usize::try_from(token.start).ok()?);
            }
            _ => {}
        }
    }
    (depth == 0).then_some(breaks)
}

/// Try to wrap a braced expression at `&&` / `||`; returns the
/// wrapped inner text (no braces) or `None`.
pub(super) fn wrap_braced_expr(
    text: &str,
    config: &FormatterConfig,
    indent_level: usize,
) -> Option<String> {
    let breaks = find_expr_break_points(text, config)?;
    if breaks.is_empty() {
        return None;
    }
    let mut chunks: Vec<&str> = Vec::new();
    let mut last = 0;
    for &pos in &breaks {
        chunks.push(text[last..pos].trim());
        last = pos;
    }
    chunks.push(text[last..].trim());

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
pub(super) fn estimate_trailing_len(args: &[CommandArg], start: usize) -> usize {
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
pub(super) fn current_col(parts: &[String], indent_len: usize) -> usize {
    let text: String = parts.concat();
    match text.rfind('\n') {
        None => indent_len + text.len(),
        Some(nl) => text.len() - nl - 1,
    }
}

// Backslash-continuation line splitting

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
pub(super) fn split_long_line(
    line: &str,
    config: &FormatterConfig,
    cont_indent: &str,
) -> Option<String> {
    if line.len() <= config.max_line_length {
        return None;
    }
    let indent_len = line.len() - line.trim_start().len();
    let all_spaces = super::super::source_layout::separator_spaces(
        line,
        config.lexer_config(),
        MAX_FORMAT_DEPTH,
    )?
    .into_iter()
    .filter(|&(offset, _)| offset >= indent_len)
    .collect::<Vec<_>>();
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

    // Every break belongs to the original complete line. A remainder is not
    // re-parsed as a new script with a different bracket or quote context.
    Some(segments?.join("\n"))
}

/// Try to split a long commented-out command using `\`
/// continuation.
pub(super) fn split_commented_code(
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
