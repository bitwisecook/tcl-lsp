//! `PyO3` bindings for the GVN passes (C32-shim).
//!
//! Exposes three entry points mirroring
//! `tcl_compiler::gvn::{find_redundancies_for_cu,
//! find_partial_redundancies_for_cu, find_loop_invariants_for_cu}`:
//!
//! - `gvn_redundancies(source, dialect)` — full redundancies (O105)
//! - `gvn_partial_redundancies(source, dialect)` — partial (O106)
//! - `gvn_loop_invariants(source, dialect)` — loop invariants (O107)
//!
//! Each returns a list of tuples
//! `(code, span_start, span_end, first_span_start, first_span_end,
//!   expression_text, message)`.
//!
//! The Python caller builds its native `RedundantComputation`
//! dataclass from these primitives, so this crate stays free of
//! Python-type knowledge. ARCH7 moved the per-function iteration
//! into `tcl_compiler::gvn::find_*_for_cu` so this module is pure
//! conversion glue.

use pyo3::prelude::*;

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::gvn::{
    find_loop_invariants_for_cu, find_partial_redundancies_for_cu, find_redundancies_for_cu,
    RedundantComputation,
};

type GvnTuple = (String, u32, u32, u32, u32, String, String);

fn lift(r: RedundantComputation) -> GvnTuple {
    (
        r.code,
        r.span.start(),
        r.span.end(),
        r.first_span.start(),
        r.first_span.end(),
        r.expression_text,
        r.message,
    )
}

/// Full-redundancy detection (O105).
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
pub fn gvn_redundancies(source: &str, dialect: Option<&str>) -> Vec<GvnTuple> {
    let registry = crate::registry::default_registry();
    let cu = CompilationUnit::build_for(source, registry, false);
    find_redundancies_for_cu(&cu, registry, dialect)
        .into_iter()
        .map(lift)
        .collect()
}

/// Partial-redundancy detection (O106).
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
pub fn gvn_partial_redundancies(source: &str, dialect: Option<&str>) -> Vec<GvnTuple> {
    let registry = crate::registry::default_registry();
    let cu = CompilationUnit::build_for(source, registry, false);
    find_partial_redundancies_for_cu(&cu, registry, dialect)
        .into_iter()
        .map(lift)
        .collect()
}

/// Loop-invariant detection (O107).
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
pub fn gvn_loop_invariants(source: &str, dialect: Option<&str>) -> Vec<GvnTuple> {
    let registry = crate::registry::default_registry();
    let cu = CompilationUnit::build_for(source, registry, false);
    find_loop_invariants_for_cu(&cu, registry, dialect)
        .into_iter()
        .map(lift)
        .collect()
}

pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(gvn_redundancies, m)?)?;
    m.add_function(wrap_pyfunction!(gvn_partial_redundancies, m)?)?;
    m.add_function(wrap_pyfunction!(gvn_loop_invariants, m)?)?;
    Ok(())
}
