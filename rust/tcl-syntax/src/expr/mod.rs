//! Tcl `expr` syntax: the AST ([`ast`]) and the precedence-climbing (Pratt)
//! parser ([`parser`]), shared by the LSP/compiler (const-fold + codegen) and
//! the runtime (the `expr` evaluator over its numeric tower). The expression
//! *lexer* is [`tcl_lexer::tokenise_expr`]; the tower-aware *evaluator* and the
//! compiler's const-fold evaluator are consumer-specific (they own the value
//! type) and live with each consumer.

pub mod ast;
pub mod eval;
pub mod mathfunc;
pub mod parser;

pub use ast::{BinOp, ExprNode, ExprOffset, UnaryOp};
pub use eval::{eval, ExprOps};
pub use parser::parse_expr;
