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
//! Dict shape:
//!
//! - `procs` — `{qualified_name: {name, qualified_name, params,
//!   name_range, body_range}}` (params is a list of
//!   `{name, has_default, default_value}` dicts)
//! - `classes` — `{qualified_name: {name, qualified_name,
//!   name_range, body_range}}`
//! - `command_invocations` — list of `{name, range}` dicts
//! - `package_requires` — list of `{name, version, range, conditional}`
//! - `source_targets` — list of `{raw_path, range, is_literal}`
//! - `command_aliases` — `{qualified_name: {qualified_name, target, extras}}`
//! - `namespace_imports` — list of `{ns, pattern, range, conjectured}`
//! - `auto_path_entries` — list of `{raw, range}`
//!
//! [`SignatureScanResult`]: tcl_compiler::signature_scan::SignatureScanResult

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use tcl_compiler::signature_scan::{
    extract_signatures,
    types::{
        ParamDef, SignatureAutoPathEntry, SignatureClass, SignatureCommandAlias,
        SignatureCommandInvocation, SignatureNamespaceImport, SignaturePackageRequire,
        SignatureProc, SignatureSource,
    },
};
use tcl_lexer::Span;

/// Extract a [`SignatureScanResult`] for `source` and serialise it
/// to a Python dict.
///
/// See the module-level docs for the dict shape.
///
/// [`SignatureScanResult`]: tcl_compiler::signature_scan::SignatureScanResult
#[pyfunction]
#[pyo3(signature = (source, /))]
pub fn signature_scan_extract<'py>(py: Python<'py>, source: &str) -> PyResult<Bound<'py, PyDict>> {
    let result = extract_signatures(source, crate::registry::default_registry());
    let out = PyDict::new_bound(py);

    let procs = PyDict::new_bound(py);
    for (qname, p) in &result.procs {
        procs.set_item(qname, proc_to_dict(py, p)?)?;
    }
    out.set_item("procs", procs)?;

    let classes = PyDict::new_bound(py);
    for (qname, c) in &result.classes {
        classes.set_item(qname, class_to_dict(py, c)?)?;
    }
    out.set_item("classes", classes)?;

    let invocations = PyList::empty_bound(py);
    for inv in &result.command_invocations {
        invocations.append(invocation_to_dict(py, inv)?)?;
    }
    out.set_item("command_invocations", invocations)?;

    let packages = PyList::empty_bound(py);
    for pr in &result.package_requires {
        packages.append(package_require_to_dict(py, pr)?)?;
    }
    out.set_item("package_requires", packages)?;

    let sources = PyList::empty_bound(py);
    for s in &result.source_targets {
        sources.append(source_to_dict(py, s)?)?;
    }
    out.set_item("source_targets", sources)?;

    let aliases = PyDict::new_bound(py);
    for (qname, a) in &result.command_aliases {
        aliases.set_item(qname, alias_to_dict(py, a)?)?;
    }
    out.set_item("command_aliases", aliases)?;

    let imports = PyList::empty_bound(py);
    for imp in &result.namespace_imports {
        imports.append(namespace_import_to_dict(py, imp)?)?;
    }
    out.set_item("namespace_imports", imports)?;

    let autopath = PyList::empty_bound(py);
    for entry in &result.auto_path_entries {
        autopath.append(auto_path_entry_to_dict(py, entry)?)?;
    }
    out.set_item("auto_path_entries", autopath)?;

    Ok(out)
}

fn proc_to_dict<'py>(py: Python<'py>, p: &SignatureProc) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &p.name)?;
    d.set_item("qualified_name", &p.qualified_name)?;
    let params = PyList::empty_bound(py);
    for param in &p.params {
        params.append(param_to_dict(py, param)?)?;
    }
    d.set_item("params", params)?;
    d.set_item("name_range", span_tuple(p.name_range))?;
    d.set_item("body_range", span_tuple(p.body_range))?;
    Ok(d)
}

fn class_to_dict<'py>(py: Python<'py>, c: &SignatureClass) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &c.name)?;
    d.set_item("qualified_name", &c.qualified_name)?;
    d.set_item("name_range", span_tuple(c.name_range))?;
    d.set_item("body_range", span_tuple(c.body_range))?;
    Ok(d)
}

fn invocation_to_dict<'py>(
    py: Python<'py>,
    inv: &SignatureCommandInvocation,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &inv.name)?;
    d.set_item("range", span_tuple(inv.range))?;
    Ok(d)
}

fn param_to_dict<'py>(py: Python<'py>, p: &ParamDef) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &p.name)?;
    d.set_item("has_default", p.has_default)?;
    d.set_item("default_value", p.default_value.clone())?;
    Ok(d)
}

fn package_require_to_dict<'py>(
    py: Python<'py>,
    pr: &SignaturePackageRequire,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &pr.name)?;
    d.set_item("version", pr.version.clone())?;
    d.set_item("range", span_tuple(pr.range))?;
    d.set_item("conditional", pr.conditional)?;
    Ok(d)
}

fn source_to_dict<'py>(py: Python<'py>, s: &SignatureSource) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("raw_path", &s.raw_path)?;
    d.set_item("range", span_tuple(s.range))?;
    d.set_item("is_literal", s.is_literal)?;
    Ok(d)
}

fn alias_to_dict<'py>(py: Python<'py>, a: &SignatureCommandAlias) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("qualified_name", &a.qualified_name)?;
    d.set_item("target", &a.target)?;
    d.set_item("extras", PyList::new_bound(py, &a.extras))?;
    Ok(d)
}

fn namespace_import_to_dict<'py>(
    py: Python<'py>,
    imp: &SignatureNamespaceImport,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("ns", &imp.ns)?;
    d.set_item("pattern", &imp.pattern)?;
    d.set_item("range", span_tuple(imp.range))?;
    d.set_item("conjectured", imp.conjectured)?;
    Ok(d)
}

fn auto_path_entry_to_dict<'py>(
    py: Python<'py>,
    entry: &SignatureAutoPathEntry,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("raw", &entry.raw)?;
    d.set_item("range", span_tuple(entry.range))?;
    Ok(d)
}

fn span_tuple(span: Span) -> (u32, u32) {
    (span.start(), span.end())
}

/// Register `signature_scan_extract` on the Python module.
pub fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(signature_scan_extract, m)?)?;
    Ok(())
}
