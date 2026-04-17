//! `PyO3` bindings for the compiler-checks aggregator (C32-shim).
//!
//! Exposes `run_all_checks(source, dialect)` to Python. The binding
//! is available for the Python layer to call, but the Python
//! `core.compiler.compiler_checks` entry point is **not yet** wired
//! to delegate here in this PR — no `TCL_LSP_RUST_CHECKS` env-var
//! gate consumes this binding today. That follow-up is blocked on
//! landing Rust-side shimmer / taint bodies (C27d / C29) so the
//! delegated path doesn't silently drop those diagnostic classes.
//! When the consumer lands it will follow the same try-import +
//! env-gate pattern as L11's lexer flip.
//!
//! Diagnostics are returned as tuples rather than a full `pyclass`
//! so the Python side can construct its own `Diagnostic` dataclass
//! (with `Range` / `SourcePosition` values built from a line index)
//! without this crate needing to know about the Python-side types.
//!
//! Note that the Rust implementations of shimmer and taint are
//! still stubs (C31 deferred work); those diagnostic classes will
//! silently return empty lists until the full ports land.

use pyo3::prelude::*;

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::{run_all_checks, Severity};
use tcl_registry::CommandRegistry;

/// Run every landed compiler check against `source` and return a
/// flat list of diagnostic tuples:
///
/// `(code, category, severity, message, start_offset, end_offset,
///   replacement_or_none)`
///
/// - `severity` is one of `"hint"`, `"suggestion"`, `"warning"`,
///   `"error"` — matching Python's `Severity` enum string values.
/// - `start_offset` / `end_offset` are absolute byte offsets into
///   `source`; the Python caller resolves them through its own
///   line index to build `SourcePosition` / `Range` values.
/// - `replacement` is a suggested fix text to replace `[start, end)`
///   with, or `None` when the diagnostic has no code action.
///
/// `dialect` is forwarded as-is; `None` selects plain Tcl.
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
#[allow(clippy::type_complexity)]
pub fn compiler_checks_run_all(
    source: &str,
    dialect: Option<&str>,
) -> Vec<(String, String, String, String, u32, u32, Option<String>)> {
    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(source, &registry, false)
        .with_interprocedural(&registry, dialect);
    let diagnostics = run_all_checks(&cu, &registry, dialect);
    diagnostics
        .into_iter()
        .map(|d| {
            (
                d.code,
                d.category,
                severity_as_str(d.severity).to_owned(),
                d.message,
                d.span.start(),
                d.span.end(),
                d.replacement,
            )
        })
        .collect()
}

fn severity_as_str(severity: Severity) -> &'static str {
    match severity {
        Severity::Hint => "hint",
        Severity::Suggestion => "suggestion",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compiler_checks_run_all, m)?)?;
    Ok(())
}
