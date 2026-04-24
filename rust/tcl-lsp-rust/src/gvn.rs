//! `PyO3` bindings for the GVN passes (C32-shim).
//!
//! Exposes three entry points mirroring
//! `tcl_compiler::gvn::{find_redundancies, find_partial_redundancies,
//! find_loop_invariants}`:
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
//! Python-type knowledge.

use pyo3::prelude::*;

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::gvn::{
    find_loop_invariants, find_partial_redundancies, find_redundancies, RedundantComputation,
};
use tcl_registry::CommandRegistry;

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
    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(source, &registry, false);
    let mut out = Vec::new();
    for fu in cu.functions() {
        for r in find_redundancies(&registry, &fu.cfg, &fu.ssa, dialect) {
            out.push(lift(r));
        }
    }
    out
}

/// Partial-redundancy detection (O106).
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
pub fn gvn_partial_redundancies(source: &str, dialect: Option<&str>) -> Vec<GvnTuple> {
    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(source, &registry, false);
    let mut out = Vec::new();
    for fu in cu.functions() {
        for r in find_partial_redundancies(&registry, &fu.cfg, &fu.ssa, dialect) {
            out.push(lift(r));
        }
    }
    out
}

/// Loop-invariant detection (O107).
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
pub fn gvn_loop_invariants(source: &str, dialect: Option<&str>) -> Vec<GvnTuple> {
    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(source, &registry, false);
    let mut out = Vec::new();
    for fu in cu.functions() {
        for r in find_loop_invariants(&registry, &fu.cfg, &fu.ssa, dialect) {
            out.push(lift(r));
        }
    }
    out
}

pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(gvn_redundancies, m)?)?;
    m.add_function(wrap_pyfunction!(gvn_partial_redundancies, m)?)?;
    m.add_function(wrap_pyfunction!(gvn_loop_invariants, m)?)?;
    Ok(())
}
