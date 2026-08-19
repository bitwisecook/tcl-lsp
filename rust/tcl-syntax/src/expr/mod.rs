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

//! Tcl `expr` syntax: the AST ([`ast`]) and the precedence-climbing (Pratt)
//! parser ([`parser`]), shared by the LSP/compiler (const-fold + codegen) and
//! the runtime (the `expr` evaluator over its numeric tower). The expression
//! *lexer* is [`tcl_lexer::tokenise_expr`]; the tower-aware *evaluator* and the
//! compiler's const-fold evaluator are consumer-specific (they own the value
//! type) and live with each consumer.

pub mod ast;
pub mod eval;
pub mod mathfunc;
pub mod operators;
pub mod parser;
pub mod rand;
pub mod substitution;
pub mod syntax_error;

pub use ast::{BinOp, ExprNode, ExprOffset, UnaryOp};
pub use eval::{ExprOps, NumericCompare, eval};
pub use mathfunc::MathFuncSpec;
pub use operators::{ALL_BIN_OPS, ALL_UNARY_OPS, CommandArity, OperatorShape, OperatorSpec};
pub use parser::parse_expr;
pub use substitution::{
    LiveExpressionSubstitutions, command_substitution_spans, live_expression_substitutions,
};
pub use syntax_error::{ExprSyntaxError, ExprSyntaxErrorKind};
