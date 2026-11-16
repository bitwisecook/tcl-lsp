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

use super::*;
use crate::expr_ast::UnaryOp;

/// Depth coverage: `walk_string_eq_ne` and `node_extent` each recurse once per `ExprNode` level
/// (`walk_string_eq_ne` also drives `node_extent` via
/// `op_offset_between`), so without a depth cap they overflow the native
/// stack (SIGABRT) in the low thousands of levels on a 2 MiB thread.  A
/// tree built directly is unbounded (the Pratt parser caps its own output
/// at 256); 3000 is past that crash range and past
/// `MAX_EXPR_NODE_DEPTH` (256); the assertion is that each returns at all.
#[test]
fn deeply_nested_eq_ne_walks_survive() {
    let mut node = ExprNode::Var {
        text: "$x".into(),
        name: "x".into(),
        start: 0,
        end: 2,
    };
    for _ in 0..3000 {
        node = ExprNode::Unary {
            op: UnaryOp::Not,
            operand: Box::new(node),
        };
    }
    let _ = find_string_eq_ne_ops(&node, "");
    let _ = node_extent(&node);
}
