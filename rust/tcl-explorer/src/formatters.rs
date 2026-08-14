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

//! Display/serialisation helpers shared by the explorer views.
//!
//! `preview`, `range_dict`, and the IR statement `stmt_kind` /
//! `stmt_summary` / `stmt_color_class` projections. These are the single
//! source of truth for both the JSON serialiser and the text/TUI
//! renderers.

use serde_json::{Value, json};

use tcl_compiler::analyses::{ConstValue, LatticeValue};
use tcl_compiler::interprocedural::{ConstantReturn, ProcSummary};
use tcl_compiler::ir::Statement;
use tcl_compiler::shimmer::type_name;
use tcl_compiler::taint::{TaintColour, TaintLattice};
use tcl_compiler::types::{TypeKind, TypeLattice};
use tcl_lexer::{LineIndex, Span};
use tcl_syntax::expr::ast::render_expr;

/// Escape and truncate text for display.
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

/// The inclusive end offset of `span`, widened to cover a braced/quoted/
/// bracketed closer when one immediately follows, operating on a
/// `[start, end)` span.
///
/// A `Span` is `[start, end)` (exclusive), so its inclusive last byte is
/// `end - 1`. Whether a delimited word's closer sits past that is the
/// [`tcl_lexer::word_span_at`] family's question, not the explorer's: an
/// empty `{}` / `[]` / `""` already ends *on* its closer and must never be
/// widened, while a word whose last inner byte happens to be a closer
/// (`{$x eq {}}`) still needs widening (issue #1423). Re-deriving that here
/// was one of six independent copies of the same arithmetic.
fn widened_inclusive_end(span: Span, source: &str) -> u32 {
    tcl_lexer::word_span_at(source, span)
        .end()
        .saturating_sub(1)
}

/// Convert a [`Span`] to the explorer's range dict.
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
        "startCol": start.character.get(),
        "startOffset": start.offset,
        "endLine": end.line,
        "endCol": end.character.get() + 1,
        "endOffset": end.offset + 1,
    })
}

/// Short name for an IR statement kind.
///
/// The returned `IR*` strings are the explorer's stable wire labels for
/// [`Statement`] variants, not Rust type names; the JSON schema, the
/// snapshot fixtures, and the web UI all key off them.
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

/// CSS class for an IR statement kind.
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

/// One-line summary of an IR statement.
///
/// The single source of truth for both the text renderers and the JSON
/// serialiser. Statement kinds without an explicit case fall through to
/// the statement's type name by default.
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
        // through to the class name, using the default.
        other => stmt_kind(other).to_owned(),
    }
}

/// Renders a constant string the way `repr()` would — the form
/// `format_return_shape` embeds. Chooses the quote accordingly (double
/// quotes when the string has a `'` but no `"`, else single) and escapes
/// backslash, the chosen quote, and the common control chars.
pub(crate) fn py_repr_str(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// `repr()`-style rendering of a constant return value.
fn py_repr_constant(c: &ConstantReturn) -> String {
    match c {
        ConstantReturn::Int(n) => n.to_string(),
        ConstantReturn::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
        ConstantReturn::Str(s) => py_repr_str(s),
        // Rust's Debug for f64 is shortest-round-trip with a decimal point,
        // matching `repr` for the common cases.
        ConstantReturn::Float(f) => format!("{f:?}"),
    }
}

/// Project a [`ProcSummary`]'s return facts to a display string.
#[must_use]
pub fn format_return_shape(s: &ProcSummary) -> String {
    if s.returns_constant {
        let r = s
            .constant_return
            .as_ref()
            .map_or_else(|| "None".to_owned(), py_repr_constant);
        return format!("const({r})");
    }
    if let Some(param) = &s.return_passthrough_param {
        return format!("passthrough({param})");
    }
    if !s.return_depends_on_params.is_empty() {
        return format!("depends({})", s.return_depends_on_params.join(","));
    }
    "unknown".to_owned()
}

/// Project a type lattice to its display string.
#[must_use]
pub fn format_type(tl: &TypeLattice) -> String {
    match tl.kind() {
        TypeKind::Unknown => "?".to_owned(),
        TypeKind::Overdefined => "*".to_owned(),
        // A container with tracked element facts renders them
        // (`List<Numeric, ?>` / `Dict<OBJECT(C)*>`); scalar shapes and
        // fact-free containers keep the bare lowercase name.
        TypeKind::Known => match tl.single_shape() {
            Some(shape)
                if !matches!(
                    tl.elements(),
                    None | Some(tcl_compiler::types::Elements::Unknown)
                ) =>
            {
                shape.to_string()
            }
            _ => tl.tcl_type().map_or_else(|| "?".to_owned(), type_name),
        },
        // A multi-member union renders every member (`shimmered(a/b)`,
        // `shimmered(a/b/c)` for the 3+-way merges the union lattice now
        // tracks instead of collapsing to overdefined).
        TypeKind::Shimmered => {
            let names: Vec<String> = tl.shapes().iter().map(|s| type_name(s.coarse())).collect();
            format!("shimmered({})", names.join("/"))
        }
    }
}

/// The lattice-kind name, lowercased (`unknown` / `known` / `shimmered` /
/// `overdefined`) — `tl.kind.name.lower()`.
#[must_use]
pub fn type_kind_name(kind: TypeKind) -> String {
    format!("{kind:?}").to_ascii_lowercase()
}

/// Project a taint lattice to its display string: `untainted`, `tainted`, or
/// `tainted(<colours>)` with the non-`TAINTED` colour names lowercased in
/// declaration order.
#[must_use]
pub fn format_taint(tl: &TaintLattice) -> String {
    if !tl.colours.contains(TaintColour::TAINTED) {
        return "untainted".to_owned();
    }
    let colours: Vec<String> = tl
        .colours
        .iter_names()
        .filter(|(name, _)| *name != "TAINTED")
        .map(|(name, _)| name.to_ascii_lowercase())
        .collect();
    if colours.is_empty() {
        "tainted".to_owned()
    } else {
        format!("tainted({})", colours.join(","))
    }
}

/// `repr()`-style rendering of a single SCCP constant value.
fn const_value_repr(c: &ConstValue) -> String {
    match c {
        ConstValue::Int(n) => n.to_string(),
        ConstValue::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
        ConstValue::String(s) => py_repr_str(s),
        ConstValue::Float(f) => format!("{f:?}"),
    }
}

/// Project an SCCP lattice value to its display string
/// (`unknown` / `overdefined` / `const(<repr>)`).
/// A `ConstSet` renders as `const(None)` because the constset's scalar
/// `value` field is `None`.
#[must_use]
pub fn format_lattice(value: &LatticeValue) -> String {
    match value {
        LatticeValue::Unknown => "unknown".to_owned(),
        LatticeValue::Overdefined => "overdefined".to_owned(),
        LatticeValue::Const(c) => format!("const({})", const_value_repr(c)),
        LatticeValue::ConstSet(_) => "const(None)".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inclusive_end_widens_a_delimited_word_to_its_closer() {
        // `{a b}` — the lexer's span stops on the last inner byte, so the
        // explorer's inclusive end must advance onto the `}`.
        assert_eq!(widened_inclusive_end(Span::new(0, 4), "{a b}"), 4);
        assert_eq!(widened_inclusive_end(Span::new(0, 2), "[x]"), 2);
        assert_eq!(widened_inclusive_end(Span::new(0, 3), "\"ab\""), 3);
    }

    #[test]
    fn inclusive_end_does_not_overshoot_an_empty_pair() {
        // `a {}}` — the empty `{}`'s span already ends on its own closer;
        // widening it would report the *enclosing* body's `}`.
        assert_eq!(widened_inclusive_end(Span::new(2, 4), "a {}}"), 3);
    }

    #[test]
    fn inclusive_end_widens_a_word_ending_in_a_nested_empty_pair() {
        // Issue #1423: `{$x eq {}}` already ends in a `}` — the inner
        // pair's — and still needs its own closer.
        let source = "while {$x eq {}} {}";
        assert_eq!(widened_inclusive_end(Span::new(6, 15), source), 15);
        assert_eq!(source.as_bytes()[15], b'}');
    }

    #[test]
    fn inclusive_end_widens_a_braced_variable_word() {
        // `${x}` — the `Var` token span excludes the closing `}`, so the
        // explorer used to report a range one byte short of the word.
        assert_eq!(widened_inclusive_end(Span::new(0, 3), "${x}"), 3);
        // `${}` is the degenerate empty name: already whole.
        assert_eq!(widened_inclusive_end(Span::new(0, 3), "${}}"), 2);
    }

    #[test]
    fn inclusive_end_leaves_undelimited_and_unterminated_spans_alone() {
        assert_eq!(widened_inclusive_end(Span::new(0, 5), "plain"), 4);
        assert_eq!(widened_inclusive_end(Span::new(0, 2), "{a"), 1);
        assert_eq!(widened_inclusive_end(Span::new(0, 0), ""), 0);
    }

    #[test]
    fn range_dict_reports_the_exclusive_end_past_the_closer() {
        let source = "{a b}";
        let li = LineIndex::new(source);
        let d = range_dict(Span::new(0, 4), &li, source);
        assert_eq!(d["startOffset"], 0);
        assert_eq!(d["endOffset"], 5);
        assert_eq!(d["endCol"], 5);
    }
}
