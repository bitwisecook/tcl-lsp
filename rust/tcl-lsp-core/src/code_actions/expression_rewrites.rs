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

//! Rewrites of a selected expression: De Morgan and comparison inversion.

use super::{ActionKind, CodeAction};
use crate::definition::{LspRange, utf16_col_to_char_col};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

// Expression rewrites: De Morgan + invert comparison.

pub(super) fn expr_rewrite_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<CodeAction> {
    use tcl_compiler::registry_invocation::source_structure::OriginalExpressionOperatorRewriteKind;
    // Single-line, non-empty selection only.
    if range.start_line != range.end_line || range.start_character >= range.end_character {
        return Vec::new();
    }
    let start = crate::definition::byte_offset_at(
        line_index,
        source,
        range.start_line,
        range.start_character,
    );
    let finish =
        crate::definition::byte_offset_at(line_index, source, range.end_line, range.end_character);
    let Some(operand) = crate::expr_context::source_expression_operand_at(source, analysis, start)
    else {
        return Vec::new();
    };
    let Some(extent) = operand.content_span() else {
        return Vec::new();
    };
    if start < extent.start() || finish > extent.end() {
        return Vec::new();
    }
    let Some(line_text) = source.split('\n').nth(range.start_line as usize) else {
        return Vec::new();
    };
    let chars: Vec<char> = line_text.chars().collect();
    let s = utf16_col_to_char_col(line_text, range.start_character).min(chars.len());
    let e = utf16_col_to_char_col(line_text, range.end_character).min(chars.len());
    if s >= e {
        return Vec::new();
    }
    let sel: String = chars[s..e].iter().collect();
    let mut out = Vec::new();
    let edit_range = LspRange {
        start_line: range.start_line,
        start_character: range.start_character,
        end_line: range.end_line,
        end_character: range.end_character,
    };
    if let Some(rewritten) = demorgan_transform(&sel) {
        out.push((
            CodeAction {
                title: "Apply De Morgan's law".to_string(),
                edits: vec![crate::rename::TextEdit {
                    range: edit_range,
                    new_text: rewritten,
                }],
                kind: ActionKind::RefactorRewrite,
                command: None,
                data_group_definition: None,
                disabled: None,
            },
            OriginalExpressionOperatorRewriteKind::DeMorgan,
        ));
    }
    if let Some(rewritten) = invert_comparison(&sel) {
        out.push((
            CodeAction {
                title: "Invert comparison".to_string(),
                edits: vec![crate::rename::TextEdit {
                    range: edit_range,
                    new_text: rewritten,
                }],
                kind: ActionKind::RefactorRewrite,
                command: None,
                data_group_definition: None,
                disabled: None,
            },
            OriginalExpressionOperatorRewriteKind::InvertComparison,
        ));
    }
    if !analysis.allows_retained_logical_declaration_advice() {
        for (action, kind) in &mut out {
            if start == extent.start()
                && finish == extent.end()
                && original_operator_rewrite_allowed(
                    source,
                    analysis,
                    extent,
                    &action.edits[0].new_text,
                    *kind,
                )
            {
                continue;
            }
            action.edits.clear();
            action.disabled = Some("missing-expression-operator-rewrite-permission: Original expression roles provide source geometry; Native or hosted operator rewrites require an independent selected evaluation and replacement contract".to_owned());
        }
    }
    out.into_iter().map(|(action, _)| action).collect()
}

fn original_operator_rewrite_allowed(
    source: &str,
    analysis: &AnalysisResult,
    extent: tcl_lexer::Span,
    candidate: &str,
    kind: tcl_compiler::registry_invocation::source_structure::OriginalExpressionOperatorRewriteKind,
) -> bool {
    let Some(config) = analysis.body_lexer_config else {
        return false;
    };
    let commands =
        tcl_compiler::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let [command] = commands.as_slice() else {
        return false;
    };
    let Some(words) = crate::original_invocation::source_registry_words(source, analysis, command)
    else {
        return false;
    };
    let Some(roles) = words.roles.as_ref() else {
        return false;
    };
    roles.iter().any(|&(argument, role)| {
        role == tcl_registry::ArgRole::Expr
            && tcl_compiler::registry_invocation::source_structure::original_expression_operator_rewrite(
                source, analysis, command, argument, candidate, kind,
            ).is_some_and(|proof| proof.original_operand().content_span().ok() == Some(extent))
    })
}

/// De Morgan: `!(X && Y)` ↔ `!X || !Y`, `!(X || Y)` ↔ `!X && !Y` — plus the
/// iRules word-operator equivalents (`not`/`and`/`or`, i.e.
/// `UnaryOp::WordNot`/`BinOp::WordAnd`/`BinOp::WordOr`). Recognising only
/// the symbolic forms would never offer the rewrite for a selection written
/// in iRules' word style (`!($a and $b)`) — an inconsistent gap given the
/// sibling `invert_comparison` rewrite in this same file handles TIP 461's
/// word operators (`lt`/`le`/`gt`/`ge`).
fn demorgan_transform(sel: &str) -> Option<String> {
    let t = sel.trim();
    let word_and = tcl_syntax::expr::ast::BinOp::WordAnd.spec().spelling;
    let word_or = tcl_syntax::expr::ast::BinOp::WordOr.spec().spelling;
    let word_not = tcl_syntax::expr::ast::UnaryOp::WordNot.spec().spelling;

    // Forward: `!( X <op> Y )` or `not ( X <op> Y )`. The outer negation
    // prefix and the inner operator's symbol/word spelling are independent
    // choices in iRules — `!($a and $b)` mixes both — so each outer prefix
    // tries every inner operator spelling, negating operands in the same
    // style as its own prefix.
    if let Some(inner) = t.strip_prefix("!(").and_then(|s| s.strip_suffix(')')) {
        return demorgan_forward_inner(inner, word_and, word_or, negate);
    }
    if let Some(inner) = t
        .strip_prefix(word_not)
        .map(str::trim_start)
        .and_then(|s| s.strip_prefix('('))
        .and_then(|s| s.strip_suffix(')'))
    {
        return demorgan_forward_inner(inner, word_and, word_or, |o| negate_word(o, word_not));
    }
    // Reverse: `!X || !Y` → `!(X && Y)`, `!X && !Y` → `!(X || Y)` — and the
    // word-operator equivalents (`not X or not Y` → `not (X and Y)`, …).
    if let Some((l, r)) = split_top_logical(t, "||")
        && let (Some(li), Some(ri)) = (l.trim().strip_prefix('!'), r.trim().strip_prefix('!'))
    {
        return Some(format!("!({} && {})", li.trim(), ri.trim()));
    }
    if let Some((l, r)) = split_top_logical(t, "&&")
        && let (Some(li), Some(ri)) = (l.trim().strip_prefix('!'), r.trim().strip_prefix('!'))
    {
        return Some(format!("!({} || {})", li.trim(), ri.trim()));
    }
    if let Some((l, r)) = split_top_logical_word(t, word_or)
        && let (Some(li), Some(ri)) = (
            strip_word_not(l.trim(), word_not),
            strip_word_not(r.trim(), word_not),
        )
    {
        return Some(format!(
            "{word_not} ({} {word_and} {})",
            li.trim(),
            ri.trim()
        ));
    }
    if let Some((l, r)) = split_top_logical_word(t, word_and)
        && let (Some(li), Some(ri)) = (
            strip_word_not(l.trim(), word_not),
            strip_word_not(r.trim(), word_not),
        )
    {
        return Some(format!(
            "{word_not} ({} {word_or} {})",
            li.trim(),
            ri.trim()
        ));
    }
    None
}

/// The body of forward-direction De Morgan (`negate_op` applies whichever
/// negation spelling matches the outer prefix that was stripped — `!` or
/// `not`), tried against every inner connective spelling (`&&`/`||` and
/// their word-operator equivalents `and`/`or`).
fn demorgan_forward_inner(
    inner: &str,
    word_and: &str,
    word_or: &str,
    negate_op: impl Fn(&str) -> String,
) -> Option<String> {
    if let Some((l, r)) = split_top_logical(inner, "&&") {
        return Some(format!(
            "{} || {}",
            negate_op(l.trim()),
            negate_op(r.trim())
        ));
    }
    if let Some((l, r)) = split_top_logical(inner, "||") {
        return Some(format!(
            "{} && {}",
            negate_op(l.trim()),
            negate_op(r.trim())
        ));
    }
    if let Some((l, r)) = split_top_logical_word(inner, word_and) {
        return Some(format!(
            "{} {word_or} {}",
            negate_op(l.trim()),
            negate_op(r.trim())
        ));
    }
    if let Some((l, r)) = split_top_logical_word(inner, word_or) {
        return Some(format!(
            "{} {word_and} {}",
            negate_op(l.trim()),
            negate_op(r.trim())
        ));
    }
    None
}

/// Like [`split_top_logical`], but for a whitespace-delimited word operator
/// (`and`/`or`) rather than a punctuation symbol — requires a single space
/// on each side (via [`find_top_level`]) so the word appearing inside a
/// longer identifier or string (`for`, `orange`, …) is never mistaken for
/// the operator.
fn split_top_logical_word<'a>(expr: &'a str, word: &str) -> Option<(&'a str, &'a str)> {
    let needle = format!(" {word} ");
    let pos = find_top_level(expr, &needle)?;
    Some((&expr[..pos], &expr[pos + needle.len()..]))
}

/// Negate a word-style operand: `$a` → `not $a`, `not $a` → `$a`, a bare
/// `!`-prefixed operand also collapses (mixed-style input) — mirrors
/// [`negate`] but for iRules' `not` spelling.
fn negate_word(operand: &str, word_not: &str) -> String {
    let o = operand.trim();
    if let Some(rest) = o.strip_prefix('!') {
        return rest.trim().to_string();
    }
    if let Some(rest) = strip_word_not(o, word_not) {
        return rest.trim().to_string();
    }
    format!("{word_not} {o}")
}

/// `Some(rest)` when `operand` is `"<word_not> rest"` — a word-boundary
/// check (a required space after `word_not`) so `notify_x` is never
/// mistaken for a negated `x`.
fn strip_word_not<'a>(operand: &'a str, word_not: &str) -> Option<&'a str> {
    operand.strip_prefix(word_not)?.strip_prefix(' ')
}

/// Negate an operand: `$a` → `!$a`, `!$a` → `$a`, `($a && $b)` → `!($a && $b)`.
fn negate(operand: &str) -> String {
    let o = operand.trim();
    if let Some(rest) = o.strip_prefix('!') {
        rest.trim().to_string()
    } else {
        format!("!{o}")
    }
}

/// Split `expr` on the top-level (brace/paren-depth 0) occurrence of `op`.
fn split_top_logical<'a>(expr: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    let bytes = expr.as_bytes();
    let opb = op.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i + opb.len() <= bytes.len() {
        match bytes[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            _ => {}
        }
        if depth == 0 && &bytes[i..i + opb.len()] == opb {
            return Some((&expr[..i], &expr[i + opb.len()..]));
        }
        i += 1;
    }
    None
}

/// Every comparison operator spelling paired with its inverse — derived
/// from `BinOp::inverse()` (`tcl_syntax::expr::operators`) rather than a
/// hand-typed list, which would miss the TIP 461 string-ordering four
/// (`lt`/`le`/`gt`/`ge`) and never offer the "Invert comparison" quick fix
/// for a selection containing one. Order doesn't matter for correctness — each
/// needle is matched as a *space-delimited* unit (`find_top_level` looks
/// for `" op "`), so e.g. `" < "` and `" <= "` can never collide as
/// substrings of each other regardless of which is tried first.
fn comparison_inversions() -> &'static [(&'static str, &'static str)] {
    static OPS: std::sync::OnceLock<Vec<(&'static str, &'static str)>> = std::sync::OnceLock::new();
    OPS.get_or_init(|| {
        tcl_syntax::expr::operators::ALL_BIN_OPS
            .iter()
            .filter_map(|op| {
                let inv = op.inverse()?;
                Some((op.spec().spelling, inv.spec().spelling))
            })
            .collect()
    })
}

/// Invert the (single) top-level comparison operator in `sel`.
fn invert_comparison(sel: &str) -> Option<String> {
    let t = sel.trim();
    for (from, to) in comparison_inversions() {
        // Require the operator to be surrounded by spaces so `$a == $b` matches
        // but a bare `<` inside a name doesn't; word ops need word boundaries.
        let needle = format!(" {from} ");
        if let Some(pos) = find_top_level(t, &needle) {
            let mut result = String::with_capacity(t.len());
            result.push_str(&t[..pos]);
            result.push(' ');
            result.push_str(to);
            result.push(' ');
            result.push_str(&t[pos + needle.len()..]);
            return Some(result);
        }
    }
    None
}

/// Find ` needle ` at brace/paren depth 0.
fn find_top_level(expr: &str, needle: &str) -> Option<usize> {
    let bytes = expr.as_bytes();
    let nb = needle.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i + nb.len() <= bytes.len() {
        match bytes[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            _ => {}
        }
        if depth == 0 && &bytes[i..i + nb.len()] == nb {
            return Some(i);
        }
        i += 1;
    }
    None
}
