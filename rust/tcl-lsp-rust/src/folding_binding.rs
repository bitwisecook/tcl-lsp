// The `?` operator in a `#[pyfunction]` returning `PyResult<_>` trips
// `clippy::useless_conversion` inside the macro expansion.
#![allow(clippy::useless_conversion)]

//! `PyO3` binding for the folding-range provider.
//!
//! Exposes ``folding_ranges(source, dialect)`` to Python.  Returns
//! a list of `{"start_line", "end_line", "kind"}` dicts where
//! `start_line` / `end_line` are 0-based inclusive line numbers and
//! `kind` is the lower-case wire form of
//! [`tcl_lsp_core::folding::FoldKind`] (`"region"` / `"comment"`) —
//! the same string `lsprotocol`'s `FoldingRangeKind` enum uses.
//!
//! Folding ranges are line-based, not byte-based, so the dict shape
//! omits the `(start, end)` `u32` byte tuple that other Rust→Python
//! bindings (`signature_scan`, `analyser`) use; resolving via
//! `core/compiler/rust_spans.py::build_position_resolver` would be
//! a no-op here (the line index Rust already computed is the
//! authoritative answer).

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use tcl_lsp_core::folding::{folding_ranges as compute_folding_ranges, FoldingRange};

/// Compute folding ranges for a Tcl source document, returning a
/// list of dicts.
///
/// See the module-level docs for the dict shape.
#[pyfunction]
#[pyo3(signature = (source, dialect, /))]
pub fn folding_ranges<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyList>> {
    let ranges = compute_folding_ranges(source, dialect);
    let out = PyList::empty_bound(py);
    for r in ranges {
        out.append(folding_range_to_dict(py, r)?)?;
    }
    Ok(out)
}

fn folding_range_to_dict(py: Python<'_>, r: FoldingRange) -> PyResult<Bound<'_, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("start_line", r.start_line)?;
    d.set_item("end_line", r.end_line)?;
    d.set_item("kind", r.kind.as_str())?;
    Ok(d)
}

/// Register `folding_ranges` on the Python module.
pub fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(folding_ranges, m)?)?;
    Ok(())
}
