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

//! Expression tree, source spans and traversal.

pub use tcl_expr::ast::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ternary_child_of_binary_is_parenthesised() {
        use crate::expr::parser::parse_expr;
        // A ternary child of a binary op must round-trip with
        // parens so it re-parses identically.
        for src in [
            "($a ? 1 : 2) + 3",
            "3 + ($a ? 1 : 2)",
            "($a ? 1 : 2) * ($b ? 3 : 4)",
        ] {
            let once = render_expr(&parse_expr(src, None));
            let twice = render_expr(&parse_expr(&once, None));
            assert_eq!(once, twice, "render must be stable for {src:?}");
            assert!(
                once.contains('('),
                "ternary child must keep parens: {src:?} → {once:?}",
            );
        }
        // Structural check: the rendered form must re-parse to a Binary at the
        // top (not a Ternary), proving the ternary stayed grouped.
        let node = parse_expr(&render_expr(&parse_expr("($a ? 1 : 2) + 3", None)), None);
        assert!(
            matches!(node, ExprNode::Binary { op: BinOp::Add, .. }),
            "top node must remain `+`, got {node:?}",
        );
    }
}
