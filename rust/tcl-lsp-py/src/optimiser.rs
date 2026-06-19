//! `PyO3` bindings for the optimiser passes (C32).
//!
//! Exposes `optimiser_find_optimisations(source, dialect)` to
//! Python. The Python `core.compiler.optimiser._manager`
//! `find_optimisations` entry point delegates here when the
//! Rust wheel is importable, falling back to the Python pass
//! pipeline otherwise (same pattern as L11's lexer flip).
//!
//! Optimisation records are returned as tuples rather than a
//! full `pyclass` so the Python side can construct its own
//! `Optimisation` dataclass (with `Range` / `SourcePosition`
//! values built from a line index) without this crate needing
//! to know about the Python-side types.

use pyo3::prelude::*;

use tcl_compiler::optimiser;

/// Tuple shape of one optimisation record returned to Python:
/// `(code, message, start_offset, end_offset, replacement,
///   group_or_none, hint_only)`.  The Python caller wraps each
/// row in its own `Optimisation` dataclass so this crate
/// doesn't need to know about Python-side types.
type OptimisationRow = (String, String, u32, u32, String, Option<u32>, bool);

fn lift_optimisation(o: optimiser::Optimisation) -> OptimisationRow {
    (
        o.code,
        o.message,
        o.span.start(),
        o.span.end(),
        o.replacement,
        o.group,
        o.hint_only,
    )
}

/// Run every landed optimisation pass against `source` and
/// return the overlap-free list of suggestions as a tuple per
/// diagnostic:
///
/// `(code, message, start_offset, end_offset, replacement,
///   group_or_none, hint_only)`
///
/// `start_offset` / `end_offset` are absolute byte offsets into
/// `source` (the offsets the Python `SourcePosition.offset`
/// field maps from). The Python caller converts those to
/// `Range` values via its own line index.
///
/// `dialect` is forwarded to the manager as-is; `None` selects
/// plain Tcl.
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
pub fn optimiser_find_optimisations(source: &str, dialect: Option<&str>) -> Vec<OptimisationRow> {
    let registry = crate::registry::default_registry();
    optimiser::optimise_with_dialect(source, registry, dialect)
        .into_iter()
        .map(lift_optimisation)
        .collect()
}

/// Run every landed optimisation pass against `source` and
/// return the **unfiltered** list (no overlap resolution). Same
/// tuple shape as [`optimiser_find_optimisations`]. Exposed for
/// tests + tooling that want to inspect the raw per-pass output.
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
pub fn optimiser_find_optimisations_raw(
    source: &str,
    dialect: Option<&str>,
) -> Vec<OptimisationRow> {
    let registry = crate::registry::default_registry();
    // optimise_raw already constructs a CompilationUnit internally;
    // no need to build one here.
    optimiser::optimise_raw(source, registry, dialect)
        .into_iter()
        .map(lift_optimisation)
        .collect()
}

/// Return the display priority for a given optimisation code.
/// Mirrors the Python `_OPT_PRIORITY` table.
#[pyfunction]
#[pyo3(signature = (code, /))]
pub fn optimiser_opt_priority(code: &str) -> u8 {
    optimiser::opt_priority(code)
}

pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(optimiser_find_optimisations, m)?)?;
    m.add_function(wrap_pyfunction!(optimiser_find_optimisations_raw, m)?)?;
    m.add_function(wrap_pyfunction!(optimiser_opt_priority, m)?)?;
    Ok(())
}
