// The `?` operator in a `#[pyfunction]` returning `PyResult<_>` trips
// `clippy::useless_conversion` inside the macro expansion.
#![allow(clippy::useless_conversion)]

//! `PyO3` binding for the `signature_scan` module.
//!
//! Exposes `signature_scan_extract(source)` to Python. Returns a
//! dict with one key per collection in the underlying
//! [`SignatureScanResult`]; spans are encoded as `(start, end)`
//! `u32` tuples, leaving the materialiser on the Python side to
//! resolve them to LSP `Range` via
//! `core/compiler/rust_spans.py::build_position_resolver`.
//!
//! Collections are emitted incrementally by C40e2/e3 sub-strips;
//! this scaffold strip wires up the binding shape and returns an
//! empty dict.
//!
//! [`SignatureScanResult`]: tcl_compiler::signature_scan::SignatureScanResult

use pyo3::prelude::*;
use pyo3::types::PyDict;

use tcl_compiler::signature_scan::extract_signatures;
use tcl_registry::CommandRegistry;

/// Extract a [`SignatureScanResult`] for `source` and serialise it
/// to a Python dict.
///
/// See the module-level docs for the dict shape.
///
/// [`SignatureScanResult`]: tcl_compiler::signature_scan::SignatureScanResult
// `PyResult` is the long-term shape; later strips populate the dict
// via fallible `set_item` calls. Suppress the unnecessary-wraps lint
// for the scaffold strip.
#[allow(clippy::unnecessary_wraps)]
#[pyfunction]
#[pyo3(signature = (source, /))]
pub fn signature_scan_extract<'py>(py: Python<'py>, source: &str) -> PyResult<Bound<'py, PyDict>> {
    let registry = CommandRegistry::build_default();
    let _result = extract_signatures(source, &registry);
    let out = PyDict::new_bound(py);
    // Collections wired by C40e2 / C40e3.
    Ok(out)
}

/// Register `signature_scan_extract` on the Python module.
pub fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(signature_scan_extract, m)?)?;
    Ok(())
}
