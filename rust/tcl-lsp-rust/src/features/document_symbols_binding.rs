// The `?` operator in a `#[pyfunction]` returning `PyResult<_>` trips
// `clippy::useless_conversion` inside the macro expansion.
#![allow(clippy::useless_conversion)]

//! `PyO3` binding for the document-symbol provider.
//!
//! Exposes ``document_symbols(source, dialect)`` to Python.  Returns
//! a list of nested dicts mirroring the LSP `DocumentSymbol` shape:
//!
//! ```text
//! {
//!   "name": str,
//!   "detail": Optional[str],
//!   "kind": str,                # SymbolKind identifier (e.g. "Function")
//!   "range": {"start_line", "start_character", "end_line", "end_character"},
//!   "selection_range": {...},   # same shape as `range`
//!   "children": list[dict],     # may be empty
//! }
//! ```
//!
//! The Python dispatcher in `lsp/features/document_symbols.py`
//! materialises these into `lsprotocol.types.DocumentSymbol`
//! values.  Keeping the materialisation in Python preserves the
//! existing `lsprotocol`-coupled tests under `tests/` without a
//! dependency-graph rewrite, mirroring the folding port's
//! split.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use super::document_symbols::{
    document_symbols as compute_document_symbols, DocumentSymbol, LineRange,
};

/// Compute document symbols for a Tcl source document, returning a
/// list of nested dicts (see the module-level docs for the shape).
#[pyfunction]
#[pyo3(signature = (source, dialect, /))]
pub fn document_symbols<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyList>> {
    let symbols = compute_document_symbols(source, dialect);
    symbols_to_list(py, &symbols)
}

fn symbols_to_list<'py>(
    py: Python<'py>,
    symbols: &[DocumentSymbol],
) -> PyResult<Bound<'py, PyList>> {
    let out = PyList::empty_bound(py);
    for s in symbols {
        out.append(symbol_to_dict(py, s)?)?;
    }
    Ok(out)
}

fn symbol_to_dict<'py>(py: Python<'py>, s: &DocumentSymbol) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &s.name)?;
    match &s.detail {
        Some(text) => d.set_item("detail", text)?,
        None => d.set_item("detail", py.None())?,
    }
    d.set_item("kind", s.kind.as_str())?;
    d.set_item("range", range_to_dict(py, s.range)?)?;
    d.set_item("selection_range", range_to_dict(py, s.selection_range)?)?;
    d.set_item("children", symbols_to_list(py, &s.children)?)?;
    Ok(d)
}

fn range_to_dict(py: Python<'_>, r: LineRange) -> PyResult<Bound<'_, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("start_line", r.start_line)?;
    d.set_item("start_character", r.start_character)?;
    d.set_item("end_line", r.end_line)?;
    d.set_item("end_character", r.end_character)?;
    Ok(d)
}

/// Register `document_symbols` on the Python module.
pub fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(document_symbols, m)?)?;
    Ok(())
}
