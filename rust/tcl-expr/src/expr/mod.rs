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

//! Expression grammar, checked parsing and pure evaluation callbacks.

pub use crate::{BinOp, ExprOffset, ExprText, UnaryOp, ast, operators};
pub use ast::ExprNode;
pub use eval::NumericCompare;
pub use syntax_error::{ExprSyntaxError, ExprSyntaxErrorKind};
pub mod errors;
pub mod eval;
pub mod jim_function_tree;
pub mod mathfunc;
pub mod native_objects;
pub mod parser;
pub mod rand;
pub mod syntax_84;
pub mod syntax_error;
pub mod wide;

pub use parser::parse_expr;
