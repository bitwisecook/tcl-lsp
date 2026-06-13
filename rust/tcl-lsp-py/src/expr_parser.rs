//! `PyO3` wrappers for `tcl_compiler::expr_parser`.
//!
//! The full `ExprNode` type bridge (constructing Python-native
//! `ExprLiteral`, `ExprBinary`, etc.) is deferred until the lowering
//! phase moves to Rust — at that point the entire tokens→IR pipeline
//! runs natively and the bridge becomes unnecessary. For now, we
//! expose `parse_expr_render` and `parse_expr_vars` for differential
//! testing and `parse_expr_tag` for structural comparison.

use pyo3::prelude::*;
use std::collections::BTreeSet;

use tcl_compiler::expr_ast::{render_expr, ExprNode};
use tcl_compiler::expr_parser::parse_expr;

/// Parse a Tcl expression and return its rendered (round-tripped) form.
///
/// This calls the full Rust pipeline: tokenise → parse → render.
/// For `ExprRaw` nodes the original text is returned unchanged.
#[pyfunction]
#[pyo3(signature = (source, /, dialect=None))]
pub fn parse_expr_render(source: &str, dialect: Option<&str>) -> String {
    let node = parse_expr(source, dialect);
    render_expr(&node)
}

/// Parse a Tcl expression and return the set of variable names.
///
/// Uses the Rust expression parser's structured variable extraction.
#[pyfunction]
#[pyo3(signature = (source, /, dialect=None))]
pub fn parse_expr_vars(source: &str, dialect: Option<&str>) -> BTreeSet<String> {
    let node = parse_expr(source, dialect);
    node.vars().into_iter().collect()
}

/// Parse a Tcl expression and return its structural tag.
///
/// Returns a string like `"Binary:Add"`, `"Literal"`, `"Var"`,
/// `"Raw"`, etc. — useful for differential tests that verify the
/// parse tree shape without needing to bridge the full `ExprNode`
/// enum to Python.
#[pyfunction]
#[pyo3(signature = (source, /, dialect=None))]
pub fn parse_expr_tag(source: &str, dialect: Option<&str>) -> String {
    let node = parse_expr(source, dialect);
    node_tag(&node)
}

/// Produce a structural tag for an expression node.
fn node_tag(node: &ExprNode) -> String {
    match node {
        ExprNode::Literal { .. } => "Literal".into(),
        ExprNode::String { .. } => "String".into(),
        ExprNode::Var { name, .. } => format!("Var:{name}"),
        ExprNode::Command { .. } => "Command".into(),
        ExprNode::Binary { op, .. } => format!("Binary:{op}"),
        ExprNode::Unary { op, .. } => format!("Unary:{op}"),
        ExprNode::Ternary { .. } => "Ternary".into(),
        ExprNode::Call { function, .. } => format!("Call:{function}"),
        ExprNode::Raw { .. } => "Raw".into(),
    }
}

pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_expr_render, m)?)?;
    m.add_function(wrap_pyfunction!(parse_expr_vars, m)?)?;
    m.add_function(wrap_pyfunction!(parse_expr_tag, m)?)?;
    Ok(())
}
