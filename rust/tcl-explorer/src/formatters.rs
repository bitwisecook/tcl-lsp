//! Display/serialisation helpers shared by the explorer views.
//!
//! Faithful port of the relevant pieces of `tooling/cli/formatters.py`:
//! `preview`, `range_dict`, and the IR statement `stmt_kind` /
//! `stmt_summary` / `stmt_color_class` projections. These are the single
//! source of truth for both the JSON serialiser and (later) the text/TUI
//! renderers, exactly as on the Python side.

use serde_json::{Value, json};

use tcl_compiler::ir::Statement;
use tcl_lexer::{LineIndex, Span};
use tcl_syntax::expr::ast::render_expr;

/// Escape and truncate text for display. Mirrors `formatters.preview`.
///
/// Escapes `\`, newline, and tab, then truncates on the *escaped* string's
/// character length: `escaped[:limit-3] + "..."` when longer than `limit`.
#[must_use]
pub fn preview(text: &str, limit: usize) -> String {
    let escaped: String = text
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t");
    let len = escaped.chars().count();
    if len > limit {
        let head: String = escaped.chars().take(limit.saturating_sub(3)).collect();
        format!("{head}...")
    } else {
        escaped
    }
}

/// The closing delimiter for an opening `{` / `"` / `[`, if any.
fn closer_for(byte: u8) -> Option<u8> {
    match byte {
        b'{' => Some(b'}'),
        b'"' => Some(b'"'),
        b'[' => Some(b']'),
        _ => None,
    }
}

/// The inclusive end offset of `span`, widened to cover a braced/quoted/
/// bracketed closer when one immediately follows — the Rust equivalent of
/// `shared.ranges.widen_range_for_closer` operating on a `[start, end)`
/// span.
///
/// A `Span` is `[start, end)` (exclusive), so its inclusive last byte is
/// `end - 1`, matching Python's inclusive-`Range.end`. The widen rules
/// then mirror Python exactly: when the span opens with a delimiter and
/// the matching closer sits one past the inclusive end, extend by one;
/// the empty `{}` / `[]` / `""` case (which already ends *on* its closer)
/// is never widened.
fn widened_inclusive_end(span: Span, source: &str) -> u32 {
    let start = span.start();
    let end = span.end();
    let default = end.saturating_sub(1);
    let bytes = source.as_bytes();
    let (a, e) = (start as usize, end as usize);
    if a >= bytes.len() || end == 0 {
        return default;
    }
    let Some(closer) = closer_for(bytes[a]) else {
        return default;
    };
    // Empty opener+closer (`{}`): inclusive end is `end - 1`, which sits on
    // the closer and `end - 1 == start + 1`. Never widen (issue #527).
    let inclusive = e - 1;
    let is_empty = end == start + 2 && inclusive < bytes.len() && bytes[inclusive] == closer;
    if !is_empty && e < bytes.len() && bytes[e] == closer {
        return end; // widened: inclusive end advances to the closer at `end`.
    }
    default
}

/// Convert a [`Span`] to the explorer's range dict. Mirrors
/// `formatters.range_dict` composed with `widen_for_highlight`.
///
/// `endCol` / `endOffset` are the inclusive end plus one (the front-end
/// slices with an exclusive end), computed from the *inclusive* end
/// position so a closer on the next line does not roll the column over.
#[must_use]
pub fn range_dict(span: Span, line_index: &LineIndex, source: &str) -> Value {
    let start = line_index.position_at(span.start());
    let inclusive_end = widened_inclusive_end(span, source);
    let end = line_index.position_at(inclusive_end);
    json!({
        "startLine": start.line,
        "startCol": start.character,
        "startOffset": start.offset,
        "endLine": end.line,
        "endCol": end.character + 1,
        "endOffset": end.offset + 1,
    })
}

/// Short name for an IR statement kind — the Python `IR*` class name.
/// Mirrors `formatters.stmt_kind`.
#[must_use]
pub fn stmt_kind(stmt: &Statement) -> &'static str {
    match stmt {
        Statement::AssignConst { .. } => "IRAssignConst",
        Statement::AssignExpr { .. } => "IRAssignExpr",
        Statement::AssignValue { .. } => "IRAssignValue",
        Statement::Incr { .. } => "IRIncr",
        Statement::ExprEval { .. } => "IRExprEval",
        Statement::Call { .. } => "IRCall",
        Statement::Return { .. } => "IRReturn",
        Statement::Barrier { .. } => "IRBarrier",
        Statement::Block { .. } => "IRBlock",
        Statement::UpFrame { .. } => "IRUpFrame",
        Statement::If { .. } => "IRIf",
        Statement::For { .. } => "IRFor",
        Statement::While { .. } => "IRWhile",
        Statement::Foreach { .. } => "IRForeach",
        Statement::Catch { .. } => "IRCatch",
        Statement::Try { .. } => "IRTry",
        Statement::Switch { .. } => "IRSwitch",
    }
}

/// CSS class for an IR statement kind. Mirrors `formatters.stmt_color_class`.
#[must_use]
pub fn stmt_color_class(stmt: &Statement) -> &'static str {
    match stmt {
        Statement::Barrier { .. } => "ir-barrier",
        Statement::Call { .. } => "ir-call",
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. } => "ir-assign",
        Statement::Return { .. } => "ir-return",
        Statement::If { .. } | Statement::For { .. } | Statement::Switch { .. } => "ir-control",
        _ => "ir-other",
    }
}

/// One-line summary of an IR statement. Mirrors `formatters.stmt_summary`.
///
/// The single source of truth for both the text renderers and the JSON
/// serialiser. Statement kinds without an explicit case fall through to
/// the class name, exactly as Python's `stmt.__class__.__name__` default.
#[must_use]
pub fn stmt_summary(stmt: &Statement) -> String {
    match stmt {
        Statement::AssignConst { name, value, .. } => {
            format!("assign-const {name} = {value}")
        }
        Statement::AssignExpr { name, expr, .. } => {
            format!(
                "assign-expr {name} = [expr {{{}}}]",
                preview(&render_expr(expr), 48)
            )
        }
        Statement::AssignValue { name, value, .. } => {
            format!("assign-value {name} = {}", preview(value, 48))
        }
        Statement::Incr { name, amount, .. } => match amount {
            Some(a) => format!("incr {name} {}", preview(a, 32)),
            None => format!("incr {name}"),
        },
        Statement::Call { command, args, .. } => {
            let mut rendered = args
                .iter()
                .take(4)
                .map(|a| preview(a, 20))
                .collect::<Vec<_>>()
                .join(" ");
            if args.len() > 4 {
                rendered.push_str(" ...");
            }
            if rendered.is_empty() {
                format!("call {command}")
            } else {
                format!("call {command} {rendered}")
            }
        }
        Statement::Return { value, .. } => match value {
            Some(v) => format!("return {}", preview(v, 48)),
            None => "return".to_owned(),
        },
        Statement::Barrier {
            reason, command, ..
        } => {
            if command.is_empty() {
                format!("barrier {reason}")
            } else {
                format!("barrier {reason} ({command})")
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            let els = if else_body.is_some() { ", else" } else { "" };
            format!("if ({} clause(s){els})", clauses.len())
        }
        Statement::For { condition, .. } => {
            format!("for ({})", preview(&render_expr(condition), 40))
        }
        Statement::Switch { subject, arms, .. } => {
            format!("switch {} ({} arm(s))", preview(subject, 40), arms.len())
        }
        // ExprEval / Block / UpFrame / While / Foreach / Catch / Try fall
        // through to the class name, mirroring Python's default.
        other => stmt_kind(other).to_owned(),
    }
}
