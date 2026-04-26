// The `?` operator in a `#[pyfunction]` returning `PyResult<_>` trips
// `clippy::useless_conversion` inside the macro expansion.
#![allow(clippy::useless_conversion)]

//! `PyO3` binding for the analyser orchestrator.
//!
//! Exposes ``analyser_analyse(source, dialect)`` to Python.
//! Returns a dict serialising the [`AnalysisResult`] produced
//! by the Rust analyser walk + diagnostic emitters.  Spans are
//! encoded as `(start, end)` `u32` tuples; the materialiser on
//! the Python side resolves them to LSP `Range` via
//! `core/compiler/rust_spans.py::build_position_resolver` (same
//! pattern as the C40e ``signature_scan_extract`` binding).
//!
//! Dict shape:
//!
//! - `global_scope` — recursive scope dict
//!   ``{kind, name, body_range, variables, procs, classes, children}``
//! - `all_procs` — `{qualified_name: proc_dict}`
//! - `all_classes` — `{qualified_name: class_dict}`
//! - `all_variables` — `{qualified_name: var_dict}`
//! - `diagnostics` — list of `{code, range, message, severity}`
//! - `command_invocations` — list of `{name, range}`
//! - `package_requires` — list of `{name, version, range, conditional}`
//! - `source_targets` — list of `{raw_path, range, is_literal}`
//! - `command_aliases` — `{qualified_name: {qualified_name, target, extras}}`
//! - `namespace_imports` — list of `{ns, pattern, range, conjectured}`
//!
//! [`AnalysisResult`]: tcl_compiler::analyser::AnalysisResult

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use tcl_compiler::analyser::{
    Analyser, AnalysisResult, ClassDef, Diagnostic, ProcDef, Scope, ScopeKind, Severity, VarDef,
};
use tcl_compiler::signature_scan::types::{
    SignatureCommandAlias, SignatureCommandInvocation, SignatureNamespaceImport,
    SignaturePackageRequire, SignatureSource,
};
use tcl_lexer::Span;

/// Analyse `source` for the given `dialect`, returning a Python
/// dict serialising the [`AnalysisResult`].
///
/// See the module-level docs for the dict shape.
///
/// [`AnalysisResult`]: tcl_compiler::analyser::AnalysisResult
#[pyfunction]
#[pyo3(signature = (source, dialect, /))]
pub fn analyser_analyse<'py>(
    py: Python<'py>,
    source: &str,
    dialect: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let mut analyser = Analyser::new();
    let result = analyser.analyse(source, dialect);
    result_to_dict(py, &result)
}

fn result_to_dict<'py>(py: Python<'py>, r: &AnalysisResult) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new_bound(py);

    out.set_item("global_scope", scope_to_dict(py, &r.global_scope)?)?;

    let procs = PyDict::new_bound(py);
    for (qname, p) in &r.all_procs {
        procs.set_item(qname, proc_to_dict(py, p)?)?;
    }
    out.set_item("all_procs", procs)?;

    let classes = PyDict::new_bound(py);
    for (qname, c) in &r.all_classes {
        classes.set_item(qname, class_to_dict(py, c)?)?;
    }
    out.set_item("all_classes", classes)?;

    let variables = PyDict::new_bound(py);
    for (qname, v) in &r.all_variables {
        variables.set_item(qname, var_to_dict(py, v)?)?;
    }
    out.set_item("all_variables", variables)?;

    let diagnostics = PyList::empty_bound(py);
    for d in &r.diagnostics {
        diagnostics.append(diagnostic_to_dict(py, d)?)?;
    }
    out.set_item("diagnostics", diagnostics)?;

    let invocations = PyList::empty_bound(py);
    for inv in &r.command_invocations {
        invocations.append(invocation_to_dict(py, inv)?)?;
    }
    out.set_item("command_invocations", invocations)?;

    let packages = PyList::empty_bound(py);
    for pr in &r.package_requires {
        packages.append(package_require_to_dict(py, pr)?)?;
    }
    out.set_item("package_requires", packages)?;

    let sources = PyList::empty_bound(py);
    for s in &r.source_targets {
        sources.append(source_target_to_dict(py, s)?)?;
    }
    out.set_item("source_targets", sources)?;

    let aliases = PyDict::new_bound(py);
    for (qname, a) in &r.command_aliases {
        aliases.set_item(qname, alias_to_dict(py, a)?)?;
    }
    out.set_item("command_aliases", aliases)?;

    let imports = PyList::empty_bound(py);
    for imp in &r.namespace_imports {
        imports.append(namespace_import_to_dict(py, imp)?)?;
    }
    out.set_item("namespace_imports", imports)?;

    Ok(out)
}

fn scope_to_dict<'py>(py: Python<'py>, s: &Scope) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("kind", scope_kind_str(s.kind))?;
    d.set_item("name", &s.name)?;
    d.set_item("body_range", s.body_span.map(span_tuple))?;

    let variables = PyDict::new_bound(py);
    for (name, v) in &s.variables {
        variables.set_item(name, var_to_dict(py, v)?)?;
    }
    d.set_item("variables", variables)?;

    let procs = PyDict::new_bound(py);
    for (name, p) in &s.procs {
        procs.set_item(name, proc_to_dict(py, p)?)?;
    }
    d.set_item("procs", procs)?;

    let classes = PyDict::new_bound(py);
    for (name, c) in &s.classes {
        classes.set_item(name, class_to_dict(py, c)?)?;
    }
    d.set_item("classes", classes)?;

    let children = PyList::empty_bound(py);
    for child in &s.children {
        children.append(scope_to_dict(py, child)?)?;
    }
    d.set_item("children", children)?;
    Ok(d)
}

fn scope_kind_str(kind: ScopeKind) -> &'static str {
    match kind {
        ScopeKind::Global => "global",
        ScopeKind::Namespace => "namespace",
        ScopeKind::Proc => "proc",
    }
}

fn proc_to_dict<'py>(py: Python<'py>, p: &ProcDef) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &p.name)?;
    d.set_item("qualified_name", &p.qualified_name)?;
    let params = PyList::empty_bound(py);
    for param in &p.params {
        let pd = PyDict::new_bound(py);
        pd.set_item("name", &param.name)?;
        pd.set_item("has_default", param.has_default)?;
        pd.set_item("default_value", param.default_value.clone())?;
        params.append(pd)?;
    }
    d.set_item("params", params)?;
    d.set_item("name_range", span_tuple(p.name_span))?;
    d.set_item("body_range", span_tuple(p.body_span))?;
    d.set_item("doc", &p.doc)?;
    Ok(d)
}

fn class_to_dict<'py>(py: Python<'py>, c: &ClassDef) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &c.name)?;
    d.set_item("qualified_name", &c.qualified_name)?;
    d.set_item("name_range", span_tuple(c.name_span))?;
    d.set_item("body_range", span_tuple(c.body_span))?;
    d.set_item("superclasses", PyList::new_bound(py, &c.superclasses))?;
    d.set_item("mixins", PyList::new_bound(py, &c.mixins))?;
    let methods = PyDict::new_bound(py);
    for (name, m) in &c.methods {
        methods.set_item(name, method_to_dict(py, m)?)?;
    }
    d.set_item("methods", methods)?;
    let class_methods = PyDict::new_bound(py);
    for (name, m) in &c.class_methods {
        class_methods.set_item(name, method_to_dict(py, m)?)?;
    }
    d.set_item("class_methods", class_methods)?;
    Ok(d)
}

fn method_to_dict<'py>(
    py: Python<'py>,
    m: &tcl_compiler::analyser::MethodDef,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &m.name)?;
    d.set_item("name_range", span_tuple(m.name_span))?;
    d.set_item("body_range", span_tuple(m.body_span))?;
    d.set_item("kind", &m.kind)?;
    d.set_item("visibility", &m.visibility)?;
    d.set_item("doc", &m.doc)?;
    Ok(d)
}

fn var_to_dict<'py>(py: Python<'py>, v: &VarDef) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &v.name)?;
    d.set_item("definition_range", span_tuple(v.definition_span))?;
    let refs = PyList::empty_bound(py);
    for r in &v.references {
        refs.append(span_tuple(*r))?;
    }
    d.set_item("references", refs)?;
    d.set_item("warn_if_unused", v.warn_if_unused)?;
    Ok(d)
}

fn diagnostic_to_dict<'py>(py: Python<'py>, d: &Diagnostic) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new_bound(py);
    out.set_item("code", &d.code)?;
    out.set_item("range", span_tuple(d.span))?;
    out.set_item("message", &d.message)?;
    out.set_item("severity", severity_str(d.severity))?;
    Ok(out)
}

fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Hint => "hint",
        Severity::Suggestion => "suggestion",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
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

fn source_target_to_dict<'py>(
    py: Python<'py>,
    s: &SignatureSource,
) -> PyResult<Bound<'py, PyDict>> {
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

fn span_tuple(span: Span) -> (u32, u32) {
    (span.start(), span.end())
}

/// Register `analyser_analyse` on the Python module.
pub fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(analyser_analyse, m)?)?;
    Ok(())
}
