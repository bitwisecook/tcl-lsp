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
use tcl_registry::CommandRegistry;

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
#[allow(clippy::type_complexity)]
pub fn optimiser_find_optimisations(
    source: &str,
    dialect: Option<&str>,
) -> Vec<(String, String, u32, u32, String, Option<u32>, bool)> {
    let registry = CommandRegistry::build_default();
    let opts = match dialect {
        Some(d) => optimiser::optimise_with_dialect(source, &registry, Some(d)),
        None => optimiser::optimise(source, &registry),
    };
    opts.into_iter()
        .map(|o| {
            (
                o.code,
                o.message,
                o.span.start(),
                o.span.end(),
                o.replacement,
                o.group,
                o.hint_only,
            )
        })
        .collect()
}

/// Run every landed optimisation pass against `source` and
/// return the **unfiltered** list (no overlap resolution). Same
/// tuple shape as [`optimiser_find_optimisations`]. Exposed for
/// tests + tooling that want to inspect the raw per-pass output.
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
#[allow(clippy::type_complexity)]
pub fn optimiser_find_optimisations_raw(
    source: &str,
    dialect: Option<&str>,
) -> Vec<(String, String, u32, u32, String, Option<u32>, bool)> {
    let registry = CommandRegistry::build_default();
    // optimise_raw already constructs a CompilationUnit internally;
    // no need to build one here.
    let opts = optimiser::optimise_raw(source, &registry, dialect);
    opts.into_iter()
        .map(|o| {
            (
                o.code,
                o.message,
                o.span.start(),
                o.span.end(),
                o.replacement,
                o.group,
                o.hint_only,
            )
        })
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
