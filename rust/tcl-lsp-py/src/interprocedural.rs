// The `?` operator in a `#[pyfunction]` that returns `PyResult<_>`
// trips `clippy::useless_conversion` inside the macro expansion
// because `PyErr: From<PyErr>` is identity. Silence for the
// whole module.
#![allow(clippy::useless_conversion)]

//! `PyO3` bindings for the interprocedural analysis (C32-shim).
//!
//! Exposes `interprocedural_summaries(source, dialect)` to Python.
//! Returns a dict keyed on qualified proc name, where each value
//! is a dict with the `ProcSummary` fields encoded as primitives
//! (ints, strs, lists, dicts). The Python caller reconstructs its
//! `ProcSummary` dataclass on top — this keeps the Rust crate
//! free of Python-type knowledge.
//!
//! The method-level `MethodSummary` is not yet surfaced — the
//! Rust side doesn't populate the OO fields today (C28-OO
//! deferred). When they land, this binding will grow a
//! parallel `methods` key.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::interprocedural::{ConstantReturn, ProcArgTrait, ProcSummary};

/// Return the full interprocedural summary table for `source`.
///
/// Returns a dict mapping qualified proc name → summary dict.
/// Each summary dict carries these keys:
///
/// - `qualified_name` (str)
/// - `params` (list[str])
/// - `arity_min` / `arity_max` (int; `arity_max == 65535` means unlimited)
/// - `calls` (list[str])
/// - `has_barrier` / `has_unknown_calls` / `writes_global` / `pure` (bool)
/// - `effect_reads` / `effect_writes` (int, `EffectRegion` bits)
/// - `returns_constant` (bool)
/// - `constant_return` (tuple[str, str] | None) — `(kind, text)` where
///   `kind` is one of `"int"`, `"float"`, `"bool"`, `"str"`
/// - `return_depends_on_params` (list[str])
/// - `return_passthrough_param` (str | None)
/// - `can_fold_static_calls` (bool)
/// - `param_traits` (dict[str, list[str]]) — e.g. `{"x": ["unused"]}`
///
/// `dialect` is forwarded as-is; `None` selects plain Tcl.
#[pyfunction]
#[pyo3(signature = (source, dialect = None, /))]
pub fn interprocedural_summaries<'py>(
    py: Python<'py>,
    source: &str,
    dialect: Option<&str>,
) -> PyResult<Bound<'py, PyDict>> {
    let registry = crate::registry::default_registry();
    let cu =
        CompilationUnit::build_for(source, registry, false).with_interprocedural(registry, dialect);
    let out = PyDict::new_bound(py);
    let Some(ia) = cu.interproc.as_ref() else {
        return Ok(out);
    };
    for (qname, summary) in &ia.procedures {
        let d = summary_to_dict(py, summary)?;
        out.set_item(qname, d)?;
    }
    Ok(out)
}

fn summary_to_dict<'py>(py: Python<'py>, s: &ProcSummary) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("qualified_name", &s.qualified_name)?;
    d.set_item("params", PyList::new_bound(py, &s.params))?;
    d.set_item("arity_min", s.arity.min)?;
    d.set_item("arity_max", s.arity.max)?;
    d.set_item("calls", PyList::new_bound(py, &s.calls))?;
    d.set_item("has_barrier", s.has_barrier)?;
    d.set_item("has_unknown_calls", s.has_unknown_calls)?;
    d.set_item("writes_global", s.writes_global)?;
    d.set_item("pure", s.pure)?;
    d.set_item("effect_reads", s.effect_reads.bits())?;
    d.set_item("effect_writes", s.effect_writes.bits())?;
    d.set_item("returns_constant", s.returns_constant)?;
    d.set_item(
        "constant_return",
        constant_return_to_py(s.constant_return.as_ref()),
    )?;
    d.set_item(
        "return_depends_on_params",
        PyList::new_bound(py, &s.return_depends_on_params),
    )?;
    d.set_item(
        "return_passthrough_param",
        s.return_passthrough_param.clone(),
    )?;
    d.set_item("can_fold_static_calls", s.can_fold_static_calls)?;
    d.set_item("param_traits", param_traits_to_dict(py, &s.param_traits)?)?;
    Ok(d)
}

fn constant_return_to_py(cr: Option<&ConstantReturn>) -> Option<(String, String)> {
    cr.map(|c| {
        let (kind, text) = c.as_kind_text();
        (kind.to_owned(), text)
    })
}

fn param_traits_to_dict<'py>(
    py: Python<'py>,
    traits: &HashMap<String, std::collections::HashSet<ProcArgTrait>>,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    for (name, set) in traits {
        let mut names: Vec<&'static str> = set.iter().map(|t| t.as_str()).collect();
        names.sort_unstable();
        d.set_item(name, PyList::new_bound(py, &names))?;
    }
    Ok(d)
}

pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(interprocedural_summaries, m)?)?;
    Ok(())
}
