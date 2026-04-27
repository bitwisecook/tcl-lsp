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
//! - `unknown_proc_info` — `{dispatch_targets, chains_original,
//!   empty_stub, case_insensitive, has_pattern_dispatch, has_exec,
//!   has_auto_load}` or ``None`` (C41e3)
//!
//! Class dicts (since C41e3) carry the full Python `ClassDef`
//! field set: `metaclass`, `constructors`, `destructor`,
//! `variables`, `properties`, `filters`, `exports`, `unexports`,
//! and `doc` in addition to the C41e0/e1/e2 fields.
//!
//! [`AnalysisResult`]: tcl_compiler::analyser::AnalysisResult

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use tcl_compiler::analyser::{
    Analyser, AnalysisResult, ClassDef, CodeFix, Diagnostic, ProcDef, PropertyDef, Scope,
    ScopeKind, Severity, UnknownProcInfo, VarDef,
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

    let provides = PyList::empty_bound(py);
    for pp in &r.package_provides {
        let d = PyDict::new_bound(py);
        d.set_item("name", &pp.name)?;
        d.set_item("version", pp.version.clone())?;
        d.set_item("range", span_tuple(pp.range))?;
        provides.append(d)?;
    }
    out.set_item("package_provides", provides)?;
    out.set_item("has_dynamic_providers", r.has_dynamic_providers)?;

    let auto_paths = PyList::empty_bound(py);
    for ap in &r.auto_path_entries {
        let d = PyDict::new_bound(py);
        d.set_item("raw_path", &ap.raw_path)?;
        d.set_item("range", span_tuple(ap.range))?;
        auto_paths.append(d)?;
    }
    out.set_item("auto_path_entries", auto_paths)?;

    let stub_cmds = PyList::empty_bound(py);
    for sc in &r.stub_commands {
        let d = PyDict::new_bound(py);
        d.set_item("name", &sc.name)?;
        let args = PyList::empty_bound(py);
        for a in &sc.args {
            let ad = PyDict::new_bound(py);
            ad.set_item("name", &a.name)?;
            ad.set_item("role", &a.role)?;
            ad.set_item("optional", a.optional)?;
            args.append(ad)?;
        }
        d.set_item("args", args)?;
        d.set_item("range", span_tuple(sc.range))?;
        d.set_item("barrier", sc.barrier)?;
        d.set_item("loop", sc.r#loop)?;
        d.set_item("pure", sc.pure)?;
        d.set_item("mutator", sc.mutator)?;
        d.set_item("unsafe", sc.r#unsafe)?;
        d.set_item("scope_alias", sc.scope_alias)?;
        stub_cmds.append(d)?;
    }
    out.set_item("stub_commands", stub_cmds)?;

    let stub_exprs = PyList::empty_bound(py);
    for se in &r.stub_expr_defs {
        let d = PyDict::new_bound(py);
        d.set_item("name", &se.name)?;
        d.set_item("kind", &se.kind)?;
        d.set_item("arity", se.arity)?;
        d.set_item("range", span_tuple(se.range))?;
        stub_exprs.append(d)?;
    }
    out.set_item("stub_expr_defs", stub_exprs)?;

    let regex = PyList::empty_bound(py);
    for rp in &r.regex_patterns {
        let d = PyDict::new_bound(py);
        d.set_item("range", span_tuple(rp.range))?;
        d.set_item("pattern", &rp.pattern)?;
        d.set_item("command", &rp.command)?;
        regex.append(d)?;
    }
    out.set_item("regex_patterns", regex)?;

    let suppressed = PyDict::new_bound(py);
    for (line, codes) in &r.suppressed_lines {
        let code_list = PyList::empty_bound(py);
        for c in codes {
            code_list.append(c.as_str())?;
        }
        suppressed.set_item(line, code_list)?;
    }
    out.set_item("suppressed_lines", suppressed)?;

    // **C41e3.** Optional unknown-proc-info dict; ``None`` when
    // the document didn't define a ``proc unknown`` (the W123
    // emitter then runs unconditionally).
    match &r.unknown_proc_info {
        Some(info) => out.set_item("unknown_proc_info", unknown_proc_info_to_dict(py, info)?)?,
        None => out.set_item("unknown_proc_info", py.None())?,
    }

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
    // Per-parameter inferred traits — keys are parameter names,
    // values are lists of stable lower-case trait names
    // (``"eval"`` / ``"body"`` / ``"var_write"`` / ``"var_read"``
    // / ``"expr"`` / ``"loop_list"``).  Empty entries are
    // omitted.
    let traits_dict = PyDict::new_bound(py);
    for (param_name, set) in &p.param_traits {
        let trait_list = PyList::empty_bound(py);
        for t in set {
            trait_list.append(t.as_str())?;
        }
        traits_dict.set_item(param_name, trait_list)?;
    }
    d.set_item("param_traits", traits_dict)?;
    Ok(d)
}

fn class_to_dict<'py>(py: Python<'py>, c: &ClassDef) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &c.name)?;
    d.set_item("qualified_name", &c.qualified_name)?;
    d.set_item("name_range", span_tuple(c.name_span))?;
    d.set_item("body_range", span_tuple(c.body_span))?;
    d.set_item("metaclass", &c.metaclass)?;
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
    let constructors = PyList::empty_bound(py);
    for ctor in &c.constructors {
        constructors.append(method_to_dict(py, ctor)?)?;
    }
    d.set_item("constructors", constructors)?;
    match &c.destructor {
        Some(dtor) => d.set_item("destructor", method_to_dict(py, dtor)?)?,
        None => d.set_item("destructor", py.None())?,
    }
    d.set_item("variables", PyList::new_bound(py, &c.variables))?;
    let properties = PyDict::new_bound(py);
    for (name, p) in &c.properties {
        properties.set_item(name, property_to_dict(py, p)?)?;
    }
    d.set_item("properties", properties)?;
    d.set_item("filters", PyList::new_bound(py, &c.filters))?;
    // ``HashSet`` iteration is non-deterministic; sort for stable
    // output so downstream callers (and golden tests) see a
    // consistent ordering.
    let mut exports: Vec<&String> = c.exports.iter().collect();
    exports.sort();
    d.set_item("exports", PyList::new_bound(py, &exports))?;
    let mut unexports: Vec<&String> = c.unexports.iter().collect();
    unexports.sort();
    d.set_item("unexports", PyList::new_bound(py, &unexports))?;
    d.set_item("doc", &c.doc)?;
    Ok(d)
}

fn property_to_dict<'py>(py: Python<'py>, p: &PropertyDef) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &p.name)?;
    d.set_item("name_range", span_tuple(p.name_span))?;
    d.set_item("kind", &p.kind)?;
    d.set_item("has_getter", p.has_getter)?;
    d.set_item("has_setter", p.has_setter)?;
    Ok(d)
}

fn unknown_proc_info_to_dict<'py>(
    py: Python<'py>,
    info: &UnknownProcInfo,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    let targets: Vec<&String> = info.dispatch_targets.iter().collect();
    d.set_item("dispatch_targets", PyList::new_bound(py, &targets))?;
    d.set_item("chains_original", info.chains_original)?;
    d.set_item("empty_stub", info.empty_stub)?;
    d.set_item("case_insensitive", info.case_insensitive)?;
    d.set_item("has_pattern_dispatch", info.has_pattern_dispatch)?;
    d.set_item("has_exec", info.has_exec)?;
    d.set_item("has_auto_load", info.has_auto_load)?;
    Ok(d)
}

fn method_to_dict<'py>(
    py: Python<'py>,
    m: &tcl_compiler::analyser::MethodDef,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("name", &m.name)?;
    let params = PyList::empty_bound(py);
    for param in &m.params {
        let pd = PyDict::new_bound(py);
        pd.set_item("name", &param.name)?;
        pd.set_item("has_default", param.has_default)?;
        pd.set_item("default_value", param.default_value.clone())?;
        params.append(pd)?;
    }
    d.set_item("params", params)?;
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
    let fixes = PyList::empty_bound(py);
    for fix in &d.fixes {
        fixes.append(code_fix_to_dict(py, fix)?)?;
    }
    out.set_item("fixes", fixes)?;
    Ok(out)
}

fn code_fix_to_dict<'py>(py: Python<'py>, fix: &CodeFix) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("range", span_tuple(fix.span))?;
    d.set_item("new_text", &fix.new_text)?;
    d.set_item("description", &fix.description)?;
    Ok(d)
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
