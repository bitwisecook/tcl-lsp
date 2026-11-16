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

//! Whether an operand is provably numeric, provably an integer or cannot be NaN, under every release or any.

use super::NumericCtx;
use crate::expr_ast::ExprNode;

/// Whether `node` is provably numeric for `expr` arithmetic — so dropping it
/// from an identity rewrite cannot hide Tcl's numeric-coercion error.
/// With no type context (`None`) every node is assumed numeric.
pub(super) fn node_provably_numeric(node: &ExprNode, numeric: NumericCtx<'_>) -> bool {
    let Some(ctx) = numeric else {
        return true;
    };
    match node {
        ExprNode::Literal { .. } => true,
        ExprNode::String { text, .. } => {
            fixed_operand_text(text).is_some_and(is_numeric_string_in_every_release)
        }
        ExprNode::Var { text, .. } => ctx.numeric.contains(text.as_str()),
        _ => false,
    }
}

/// Whether `node` is provably an *integer* for `expr` arithmetic — so folding
/// it to an integer literal, or dropping an integer-only operator around it,
/// matches Tcl's result and error behaviour. With no type context (`None`)
/// every node is assumed integer.
pub(super) fn node_provably_integer(node: &ExprNode, numeric: NumericCtx<'_>) -> bool {
    let Some(ctx) = numeric else {
        return true;
    };
    match node {
        ExprNode::Literal { text, .. } => is_integer_string(text),
        ExprNode::String { text, .. } => fixed_operand_text(text).is_some_and(is_integer_string),
        ExprNode::Var { text, .. } => ctx.integer.contains(text.as_str()),
        _ => false,
    }
}

/// Whether `node`'s value provably cannot be the IEEE NaN — the precondition
/// of [`BinOp::inverse`]'s ordered-comparison rows and of the `$x == $x`
/// self-comparison folds.
///
/// Tcl follows C's rule (`tclExecute.c`): with a NaN operand `!=` is true and
/// every other comparison is false. That breaks both `!(a < b) == (a >= b)`
/// and `$x == $x == 1`, so those rewrites need this proof.
///
/// Proof sources, deliberately narrow:
///
/// * a literal or string whose text is not a NaN spelling under **any** release
///   — either a non-NaN number, or not a number at all (a plain string can no
///   more be NaN than it can be `inf`);
/// * a variable the type lattice proves `Int`. *Numeric* is not enough: the
///   numeric set also holds `Double` and the catch-all `Numeric`, and a double
///   may perfectly well be NaN.
///
/// Everything else — a command substitution, an arithmetic subexpression, any
/// variable with no type context at all — is unproven, so the caller abstains.
/// Unlike [`node_provably_numeric`] / [`node_provably_integer`] this does *not*
/// fall back to "assume the best" when the context is `None`: a caller with no
/// lattice has proved nothing, and the rewrites this gates are unsound without
/// the proof.
pub(super) fn node_cannot_be_nan(node: &ExprNode, numeric: NumericCtx<'_>) -> bool {
    match node {
        ExprNode::Literal { text, .. } => !is_nan_string_in_any_release(text),
        ExprNode::String { text, .. } => {
            fixed_operand_text(text).is_some_and(|value| !is_nan_string_in_any_release(value))
        }
        ExprNode::Var { text, .. } => {
            numeric.is_some_and(|ctx| ctx.integer.contains(text.as_str()))
        }
        _ => false,
    }
}

/// Whether the (delimiter-stripped) text spells NaN under **any** release —
/// the sound direction for a gate that *refuses* a rewrite because a value
/// "could still be NaN", mirroring [`is_numeric_string_in_any_release`].
pub(super) fn is_nan_string_in_any_release(text: &str) -> bool {
    use tcl_syntax::number::{Number, ParseFlags};
    tcl_syntax::number::NumberSyntax::any(|n| {
        matches!(
            tcl_syntax::number::parse_whole_with(
                strip_literal_delims(text),
                ParseFlags::for_syntax(n),
            ),
            Some(Number::Nan { .. })
        )
    })
}

/// Strip the surrounding `"…"` / `{…}` delimiters (if any) from an `expr`
/// literal or a propagated constant's value text, for the numeric
/// classifiers below.
/// A string operand's value when the proofs here may read it: `None` for one
/// that substitutes, whose value is unknown (`"$x" == 1` is a numeric compare
/// when `x` is `1.0`, and `"$x"` may be `NaN`), and for a value that itself
/// holds a quote or brace, which [`strip_literal_delims`] would misread
/// (#2227, found in review).
pub(super) fn fixed_operand_text(text: &str) -> Option<&str> {
    tcl_syntax::expr::fixed_string_operand(text).filter(|value| !value.contains(['"', '{', '}']))
}

fn strip_literal_delims(text: &str) -> &str {
    text.trim()
        .trim_start_matches(['"', '{'])
        .trim_end_matches(['"', '}'])
        .trim()
}

/// Whether the (delimiter-stripped) text of an `expr` string literal or a
/// propagated constant's value parses as a Tcl number — the SCCP-inlined-
/// constant case. Backed by the shared `tcl_syntax::number` grammar (the
/// same one the const-folder and [`is_integer_string`] use) rather than
/// Rust's own `str::parse`, which rejects Tcl's `0x`/`0o`/`0b` prefixes and
/// `_` digit separators — a hex/octal/binary constant is a real Tcl number
/// and must classify as one here, or two callers (the O120 eq/ne string-
/// compare promotion gate and the O100/O101 constant-substitution binder)
/// wrongly treat it as "provably not a number".
/// [`is_numeric_string`] under one release's grammar, or — with `numbers`
/// [`None`] — under whichever grammar this process was built for.
pub(super) fn is_numeric_string_under(
    text: &str,
    numbers: Option<tcl_syntax::number::NumberSyntax>,
) -> bool {
    use tcl_syntax::number::{Number, ParseFlags};
    let t = strip_literal_delims(text);
    let flags = numbers.map_or_else(ParseFlags::default, ParseFlags::for_syntax);
    !t.is_empty()
        && matches!(
            tcl_syntax::number::parse_whole_with(t, flags),
            Some(Number::Int(_) | Number::Big { .. } | Number::Double(_) | Number::Nan { .. })
        )
}

/// Whether `text` is a number under **every** release — the sound reading of
/// "provably numeric" for an optimiser gate that has no target in hand.
///
/// A release-dependent spelling is not *provable*: `08` is a number from 9.0 and
/// an invalid octal before it, so a rewrite that requires a numeric operand must
/// not fire on it. Requiring unanimity keeps the gate correct whichever release
/// the program is eventually built for.
pub(super) fn is_numeric_string_in_every_release(text: &str) -> bool {
    tcl_syntax::number::NumberSyntax::every(|n| is_numeric_string_under(text, Some(n)))
}

/// Whether `text` is a number under **any** release — the sound reading for a
/// gate that *refuses* an optimisation because a value "could still be a
/// number". Here the permissive direction is the safe one: a spelling numeric on
/// even one release must block the rewrite.
pub(super) fn is_numeric_string_in_any_release(text: &str) -> bool {
    tcl_syntax::number::NumberSyntax::any(|n| is_numeric_string_under(text, Some(n)))
}

/// Whether the (delimiter-stripped) text of an `expr` literal parses as a Tcl
/// *integer* (decimal / `0x` / `0o` / `0b`, any magnitude — a bignum is still
/// an integer). A float literal (`1.5`) is not. Uses the shared number grammar
/// so it agrees with the const-folder.
fn is_integer_string(text: &str) -> bool {
    use tcl_syntax::number::{Number, ParseFlags};
    // Unanimous across releases, for the same reason as
    // [`is_numeric_string_in_every_release`]: this gates rewrites that need a
    // genuine integer operand, and a release-dependent spelling is not proof.
    tcl_syntax::number::NumberSyntax::every(|n| {
        matches!(
            tcl_syntax::number::parse_whole_with(
                strip_literal_delims(text),
                ParseFlags::for_syntax(n),
            ),
            Some(Number::Int(_) | Number::Big { .. })
        )
    })
}
