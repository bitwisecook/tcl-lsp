//! `PyO3` bindings for the compiler-checks aggregator.
//!
//! Exposes `run_all_checks(source, dialect)` to Python.
//!
//! Diagnostics are returned as tuples rather than a full `pyclass`
//! so the Python side can construct its own `Diagnostic` dataclass
//! (with `Range` / `SourcePosition` values built from a line index)
//! without this crate needing to know about the Python-side types.

use pyo3::prelude::*;

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;

/// Tuple shape of one compiler-check diagnostic returned to
/// Python: `(code, category, severity, message, start_offset,
/// end_offset, replacement_or_none)`.  The Python caller wraps
/// each row in its own `Diagnostic` dataclass.
type DiagnosticRow = (String, String, String, String, u32, u32, Option<String>);

/// Run every compiler check against `source` and return a
/// flat list of diagnostic tuples:
///
/// `(code, category, severity, message, start_offset, end_offset,
///   replacement_or_none)`
///
/// - `severity` is one of `"hint"`, `"suggestion"`, `"warning"`,
///   `"error"` — the wire form returned by
///   `tcl_compiler::compiler_checks::Severity::as_str`.
/// - `start_offset` / `end_offset` are absolute byte offsets into
///   `source`; the Python caller resolves them through its own
///   line index to build `SourcePosition` / `Range` values.
/// - `replacement` is a suggested fix text to replace `[start, end)`
///   with, or `None` when the diagnostic has no code action.
///
/// `dialect` is forwarded as-is; `None` selects plain Tcl.
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
pub fn compiler_checks_run_all(
    py: Python<'_>,
    source: &str,
    dialect: Option<&str>,
) -> Vec<DiagnosticRow> {
    let registry = crate::registry::default_registry();
    let source = source.to_owned();
    let dialect = dialect.map(str::to_owned);
    py.detach(move || {
        let dialect = dialect.as_deref();
        let cu = CompilationUnit::build_for(&source, registry, false)
            .with_interprocedural(registry, dialect);
        run_all_checks(&cu, registry, dialect)
            .into_iter()
            .map(|d| {
                (
                    d.code.to_string(),
                    d.category,
                    d.severity.as_str().to_owned(),
                    d.message,
                    d.span.start(),
                    d.span.end(),
                    d.replacement,
                )
            })
            .collect()
    })
}

pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compiler_checks_run_all, m)?)?;
    Ok(())
}
