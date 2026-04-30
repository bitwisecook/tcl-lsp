//! Transitional alias for [`tcl_lsp_py`].
//!
//! ARCH9 split the public `PyO3` surface into a dedicated
//! [`tcl_lsp_py`] crate. This crate now exists only to expose the
//! same surface under the legacy `tcl_lsp_rust` module name so
//! existing wheel consumers keep working for one release cycle —
//! the alias retires in vNext.
//!
//! New Python code should `import tcl_lsp_py` directly. Both
//! modules resolve to the same Rust functions and classes; the
//! `__module__` attribute on `#[pyclass]` types reads
//! `"tcl_lsp_py"` regardless of which alias the caller imported.

#![deny(missing_docs)]

use pyo3::prelude::*;

/// Python-visible module. Registers every binding from
/// [`tcl_lsp_py::register_with`] under the legacy
/// `tcl_lsp_rust` name.
#[pymodule]
fn tcl_lsp_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    tcl_lsp_py::register_with(m)
}
