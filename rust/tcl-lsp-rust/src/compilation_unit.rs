//! `PyO3` binding for building a Rust [`CompilationUnit`] handle
//! that Python callers can re-use across multiple analyses.
//!
//! Exposes `build_compilation_unit(source, dialect=None)` returning
//! a `CompilationUnitHandle` pyclass. The handle is opaque — Python
//! treats it as a capsule and passes it back to other bindings
//! (shipped in follow-ups) that want to share a CU across calls
//! without re-lowering the source.
//!
//! Today a single Rust [`CompilationUnit`] covers: IR lowering,
//! CFG + SSA, SCCP, def-use, memory-SSA, interprocedural
//! summaries. Callers that need any combination of those pass the
//! same handle back, amortising the analysis cost.

use std::sync::Arc;

use pyo3::prelude::*;

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

/// Opaque handle wrapping an `Arc<CompilationUnit>`.
///
/// The wrapper is immutable from Python; all state lives on the
/// Rust side. `Arc` lets the handle be cheap-cloneable so future
/// follow-up bindings can take it by value without sacrificing
/// amortised reuse.
#[pyclass(module = "tcl_lsp_rust", name = "CompilationUnit", frozen)]
#[derive(Clone)]
pub struct CompilationUnitHandle {
    pub(crate) inner: Arc<CompilationUnit>,
}

#[pymethods]
impl CompilationUnitHandle {
    /// Number of procedures discovered in the source.
    #[getter]
    fn procedure_count(&self) -> usize {
        self.inner.procedures.len()
    }

    /// True when the unit carries interprocedural summaries.
    #[getter]
    fn has_interprocedural(&self) -> bool {
        self.inner.interproc.is_some()
    }

    /// Length of the source text.
    #[getter]
    fn source_len(&self) -> usize {
        self.inner.source.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "CompilationUnit(procs={}, interproc={}, source_len={})",
            self.inner.procedures.len(),
            self.inner.interproc.is_some(),
            self.inner.source.len(),
        )
    }
}

/// Build a compilation unit for `source` (optionally with
/// interprocedural summaries populated when `dialect` is not None
/// — the `build_interprocedural_analysis` call is dialect-aware).
///
/// Returns a [`CompilationUnitHandle`] that Python can pass to
/// other bindings (CU-aware wrappers shipped in follow-ups).
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
pub fn build_compilation_unit(source: &str, dialect: Option<&str>) -> CompilationUnitHandle {
    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(source, &registry, false)
        .with_interprocedural(&registry, dialect);
    CompilationUnitHandle {
        inner: Arc::new(cu),
    }
}

pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<CompilationUnitHandle>()?;
    m.add_function(wrap_pyfunction!(build_compilation_unit, m)?)?;
    Ok(())
}
