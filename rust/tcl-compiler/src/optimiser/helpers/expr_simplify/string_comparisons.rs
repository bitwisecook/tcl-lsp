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

//! `string length` tests and `==`/`!=` against a string rewritten to string comparisons.

use super::numeric_proofs::{fixed_operand_text, is_numeric_string_in_any_release};
use crate::expr_ast::{BinOp, ExprNode, render_expr};

/// Simplify `[string length $s] == 0` → `$s eq ""` and
/// related strlen shapes.
///
/// The Rust lowering's `ExprNode` does not model `[string length …]`
/// as a first-class call — it's a command substitution (`ExprNode::Command`).
/// We detect the shape via the `text` field when the expression
/// is a binary `==` / `!=` against `0`.
#[must_use]
pub fn try_strlen_simplify_expr(
    expr: &str,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = crate::expr_parser::parse_expr_for_profile(trimmed, dialect);
    let ExprNode::Binary { op, left, right } = &parsed else {
        return (expr.to_owned(), false);
    };
    // Match the two commutative shapes: `[cmd] == 0` and
    // `0 == [cmd]`. The earlier `let…else` returns on any
    // other shape.
    let cmd_text: &String;
    let lit_text: &String;
    if let ExprNode::Command { text, .. } = left.as_ref() {
        if let ExprNode::Literal { text: lit, .. } = right.as_ref() {
            cmd_text = text;
            lit_text = lit;
        } else {
            return (expr.to_owned(), false);
        }
    } else if let ExprNode::Literal { text: lit, .. } = left.as_ref() {
        if let ExprNode::Command { text, .. } = right.as_ref() {
            cmd_text = text;
            lit_text = lit;
        } else {
            return (expr.to_owned(), false);
        }
    } else {
        return (expr.to_owned(), false);
    }
    let cmd = cmd_text;
    let lit = lit_text;
    if lit.trim() != "0" {
        return (expr.to_owned(), false);
    }
    let Some(inner) = cmd.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
        return (expr.to_owned(), false);
    };
    let inner = inner.trim();
    let Some(rest) = inner.strip_prefix("string length") else {
        return (expr.to_owned(), false);
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return (expr.to_owned(), false);
    }
    // Preserve `{…}` / `"…"` wrapping when deciding how to emit
    // the operand: if the user wrote `string length "foo bar"`,
    // keep the quoting.
    let operand = rest.to_owned();
    let new_op = match op {
        BinOp::Eq | BinOp::StrEq => "eq",
        BinOp::Ne | BinOp::StrNe => "ne",
        _ => return (expr.to_owned(), false),
    };
    let new_text = format!("{operand} {new_op} \"\"");
    (new_text, true)
}

/// Promote numeric `==` / `!=` against a quoted-string
/// literal to `eq` / `ne`. Safer: a numeric compare shimmers the
/// LHS to a number at runtime; the string form avoids that.
///
/// Only fires when one side is a string literal (produced by the
/// parser as `ExprNode::String`) — numeric literals keep the
/// numeric compare.
#[must_use]
pub fn try_eq_ne_string_compare_simplify_expr(
    expr: &str,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = crate::expr_parser::parse_expr_for_profile(trimmed, dialect);
    let Some(rewritten) = streq_promote_node(&parsed) else {
        return (expr.to_owned(), false);
    };
    let rendered = render_expr(&rewritten);
    let changed = rendered != trimmed;
    (rendered, changed)
}

/// One pass of eq/ne-promotion: rewrite `==` / `!=` to `eq` / `ne`
/// **only when at least one operand is provably non-numeric**.
///
/// D5-O120 soundness gate. Tcl's `==`/`!=` parse *both* operands as a
/// number first and only fall through to a string compare when at least
/// one parse fails — so the promotion is sound iff one operand can never
/// be a number. A bare string literal is not enough: `$x == "1"` must
/// stay numeric (`"1"` parses as the integer 1), or the rewrite flips the
/// result when `$x` is numeric. We require a string literal whose
/// delimiter-stripped text does **not** parse as a number to prove an
/// operand non-numeric. The variable-with-SCCP-CONST refinement needs
/// lattice values not threaded here, so it is conservatively skipped (a
/// missed rewrite, never an unsound one).
pub(super) fn streq_promote_node(node: &ExprNode) -> Option<ExprNode> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    let new_op = match op {
        BinOp::Eq => BinOp::StrEq,
        BinOp::Ne => BinOp::StrNe,
        _ => return None,
    };
    if !node_provably_non_numeric(left) && !node_provably_non_numeric(right) {
        return None;
    }
    Some(ExprNode::Binary {
        op: new_op,
        left: left.clone(),
        right: right.clone(),
    })
}

/// Whether `left == right` (and `!=`) compares its operands as strings
/// whatever the other one holds, so `eq` / `ne` give the same answer.
///
/// Tcl compares numerically only when both operands are numbers, so it is
/// enough that one is a fixed string that is not a number under any release.
/// A boolean word counts as a string here: `==` never reads `true` as `1`
/// (`expr {"true" == 1}` is `0` on tclsh 8.4.20 through 9.1b0), while `nan`,
/// ` 1`, `0x10`, `08` (9.0) and `1_0` (9.0) are numbers. A substituting
/// operand proves nothing: `"$x" == 1` is numeric when `x` is `1.0`.
pub(crate) fn eq_ne_compares_as_strings(left: &ExprNode, right: &ExprNode) -> bool {
    let fixed_non_numeric = |node: &ExprNode| {
        matches!(node, ExprNode::String { text, .. }
            if fixed_operand_text(text).is_some_and(|value| !is_numeric_string_in_any_release(value)))
    };
    fixed_non_numeric(left) || fixed_non_numeric(right)
}

/// Whether `node` is provably **not** a number for `expr` — the dual of
/// [`node_provably_numeric`], used to gate the eq/ne string-compare
/// promotion (O120). Only a string literal whose stripped text is neither
/// numeric nor a boolean word qualifies; everything else (variables, command
/// substitutions, arithmetic) is conservatively rejected.
pub(super) fn node_provably_non_numeric(node: &ExprNode) -> bool {
    matches!(node, ExprNode::String { text, .. }
        if fixed_operand_text(text).is_some_and(|value| !is_numeric_or_boolean_string(value)))
}

/// Whether the (delimiter-stripped) text parses as a number **or** is one of
/// Tcl's boolean words (`true`/`false`/`yes`/`no`/`on`/`off`,
/// case-insensitive). The `==` promotion treats such a value as "could still
/// be a number" and so refuses to promote.
/// Kept separate from [`is_numeric_string`] (used by the arithmetic-identity
/// gate, where a boolean word is *not* a valid arithmetic operand).
fn is_numeric_or_boolean_string(text: &str) -> bool {
    // The permissive direction: this gate refuses a promotion when the value
    // could still be a number on some release.
    if is_numeric_string_in_any_release(text) {
        return true;
    }
    let stripped = text
        .trim()
        .trim_start_matches(['"', '{'])
        .trim_end_matches(['"', '}'])
        .trim();
    tcl_syntax::boolean::is_boolean_full_word(stripped)
}
