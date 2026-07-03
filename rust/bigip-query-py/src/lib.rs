//! PyO3 bindings for the F5 BIG-IP query engine.
//!
//! This module (`f5report._engine`) is the native bridge the pure-Python report
//! generator imports. It exposes three things:
//!
//! * [`query`] — run a `f5-query` DSL expression over one or more in-memory
//!   BIG-IP configs and get the results back as native Python objects
//!   (`dict` / `list` / `str` / … — no JSON round-trip);
//! * [`load_paths`] — resolve file paths to `(uri, scf_text)`, transparently
//!   extracting `.ucs` archives (plain or OpenPGP-encrypted) to SCF;
//! * [`ucs_to_scf`] — the same extraction for a UCS already held in memory as
//!   `bytes`.
//!
//! Everything runs in-process against the same Rust crates the `f5-query`
//! binary uses; there is no subprocess and no shelling out.

use std::collections::HashMap;

use pyo3::create_exception;
use pyo3::exceptions::{PyIOError, PyException};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};

use tcl_bigip_io::{
    PassphraseOptions, is_pgp_bytes, is_ucs_bytes, resolve_passphrase, ucs_archive_to_scf,
};
use tcl_bigip_query::value::Value;
use tcl_bigip_query::{QueryOptions, run_query};

create_exception!(
    _engine,
    QueryError,
    PyException,
    "Raised when a BIG-IP query fails to parse or evaluate, or a UCS/​config \
     source cannot be loaded."
);

/// Convert an engine [`Value`] into a native Python object.
///
/// The mapping mirrors the DSL's canonical JSON spelling (see
/// `tcl_bigip_query::jsonfmt`) so Python sees exactly what `--json` would emit,
/// but as live `dict`/`list`/`str`/`int`/`float`/`bool`/`None` objects rather
/// than a serialised string:
///
/// * `PathRef`  → the target full-path `str`;
/// * `Stream`   → `list` (top-level streams are flattened by [`query`]);
/// * `ObjectRef`→ `{"kind", "full-path", "fields": {…}}`;
/// * `Container`→ `"container(<kind>)"` (matching the JSON fallback);
/// * `Drop`/`Null` → `None`.
fn value_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    let obj = match value {
        Value::Null | Value::Drop => py.None(),
        Value::Bool(b) => pyo3::types::PyBool::new(py, *b).to_owned().into_any().unbind(),
        Value::Int(i) => i.into_pyobject(py)?.into_any().unbind(),
        Value::Float(f) => f.into_pyobject(py)?.into_any().unbind(),
        Value::Str(s) => PyString::new(py, s).into_any().unbind(),
        Value::PathRef(p) => PyString::new(py, &p.full_path).into_any().unbind(),
        Value::List(items) | Value::Stream(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(value_to_py(py, item)?)?;
            }
            list.into_any().unbind()
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, value_to_py(py, v)?)?;
            }
            dict.into_any().unbind()
        }
        Value::Container(c) => {
            PyString::new(py, &format!("container({})", c.kind))
                .into_any()
                .unbind()
        }
        Value::ObjectRef(o) => {
            let dict = PyDict::new(py);
            dict.set_item("kind", &o.kind)?;
            dict.set_item("full-path", &o.full_path)?;
            let fields = PyDict::new(py);
            for (k, v) in &o.fields {
                fields.set_item(k, value_to_py(py, v)?)?;
            }
            dict.set_item("fields", fields)?;
            dict.into_any().unbind()
        }
    };
    Ok(obj)
}

/// Coerce the Python `sources` argument into an ordered `[(uri, text)]` list.
///
/// Accepts either a mapping (`{uri: text}`) or an iterable of 2-item
/// `(uri, text)` pairs — the two natural shapes a caller already has after
/// [`load_paths`].
fn coerce_sources(sources: &Bound<'_, PyAny>) -> PyResult<Vec<(String, String)>> {
    if let Ok(dict) = sources.cast::<PyDict>() {
        let mut out = Vec::with_capacity(dict.len());
        for (k, v) in dict.iter() {
            out.push((k.extract::<String>()?, v.extract::<String>()?));
        }
        return Ok(out);
    }
    let mut out = Vec::new();
    for item in sources.try_iter()? {
        let item = item?;
        let pair = item.cast::<PyTuple>().map_err(|_| {
            QueryError::new_err("each source must be a (uri, text) tuple")
        })?;
        if pair.len() != 2 {
            return Err(QueryError::new_err("each source must be a (uri, text) 2-tuple"));
        }
        out.push((
            pair.get_item(0)?.extract::<String>()?,
            pair.get_item(1)?.extract::<String>()?,
        ));
    }
    Ok(out)
}

/// Run a BIG-IP query DSL expression against in-memory configs.
///
/// `expr` is any `f5-query` expression (`.ltm.virtual[]`, `... | referenced_by`,
/// etc.). `sources` is a `{uri: text}` mapping or a list of `(uri, text)` pairs,
/// each the SCF/`bigip.conf` text for one config (as returned by
/// [`load_paths`]).
///
/// Read-only queries only: a mutating expression (`=` / `rename` / …) raises
/// [`QueryError`], since a report never rewrites its input.
///
/// Returns a flat `list` of result values (top-level streams flattened), unless
/// `per_file=True`, in which case it returns `[(uri, [values]), …]` in source
/// order.
#[pyfunction]
#[pyo3(signature = (expr, sources, *, merge=false, partitions=None, enable_probes=false, per_file=false))]
fn query(
    py: Python<'_>,
    expr: &str,
    sources: &Bound<'_, PyAny>,
    merge: bool,
    partitions: Option<HashMap<String, String>>,
    enable_probes: bool,
    per_file: bool,
) -> PyResult<Py<PyAny>> {
    let sources = coerce_sources(sources)?;
    let opts = QueryOptions {
        partitions: partitions.unwrap_or_default(),
        enable_probes,
        merge,
        ..QueryOptions::default()
    };

    let result = run_query(expr, &sources, &opts)
        .map_err(|e| QueryError::new_err(e.to_string()))?;
    if result.has_mutation {
        return Err(QueryError::new_err(
            "mutating queries are not supported by the report engine (read-only)",
        ));
    }

    if per_file {
        let out = PyList::empty(py);
        for (uri, values) in &result.values_per_file {
            let vals = PyList::empty(py);
            for v in values {
                vals.append(value_to_py(py, v)?)?;
            }
            out.append(PyTuple::new(py, [PyString::new(py, uri).into_any(), vals.into_any()])?)?;
        }
        return Ok(out.into_any().unbind());
    }

    // Default: one flat list of every value across every source.
    let out = PyList::empty(py);
    for (_uri, values) in &result.values_per_file {
        for v in values {
            out.append(value_to_py(py, v)?)?;
        }
    }
    Ok(out.into_any().unbind())
}

/// Resolve a single path to `(uri, text)`, extracting a UCS when needed.
fn read_one(path: &str, passphrase: &Option<String>, include_extras: bool) -> PyResult<(String, String)> {
    let raw = std::fs::read(path).map_err(|e| PyIOError::new_err(format!("{path}: {e}")))?;
    let is_ucs_ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ucs"));

    // OpenPGP magic is unambiguous; plain gzip is only treated as a UCS for a
    // `.ucs` path. Everything else is decoded as UTF-8 config text.
    if is_pgp_bytes(&raw) || (is_ucs_bytes(&raw) && is_ucs_ext) {
        let opts = PassphraseOptions {
            explicit: passphrase.clone(),
            allow_prompt: false,
            ..PassphraseOptions::default()
        };
        let provider = move || resolve_passphrase(&opts);
        let scf = ucs_archive_to_scf(&raw, &provider, include_extras, path)
            .map_err(|e| QueryError::new_err(e.to_string()))?;
        Ok((path.to_owned(), scf))
    } else {
        Ok((path.to_owned(), String::from_utf8_lossy(&raw).into_owned()))
    }
}

/// Load one or more BIG-IP config / UCS paths into `[(uri, text)]`.
///
/// A `.ucs` archive — plain gzip-tar or OpenPGP-encrypted (F5 KB K5437) — is
/// transparently decrypted (when `passphrase` or `$F5_UCS_PASSPHRASE` is set)
/// and flattened to SCF text. Plain `bigip.conf`/SCF files are read as UTF-8.
///
/// `include_extras=True` folds every `config/*.conf` member (partition configs,
/// GTM, base) into the SCF, not just the canonical five — useful for a full
/// multi-partition report.
#[pyfunction]
#[pyo3(signature = (paths, *, passphrase=None, include_extras=false))]
fn load_paths(
    paths: Vec<String>,
    passphrase: Option<String>,
    include_extras: bool,
) -> PyResult<Vec<(String, String)>> {
    paths
        .iter()
        .map(|p| read_one(p, &passphrase, include_extras))
        .collect()
}

/// Extract a UCS archive already held in memory to SCF text.
///
/// `data` is the raw `.ucs` bytes (plain or OpenPGP-encrypted). Mirrors the
/// extraction [`load_paths`] performs, for callers that already have the bytes
/// (e.g. a UCS downloaded over the network).
#[pyfunction]
#[pyo3(signature = (data, *, passphrase=None, include_extras=false, label="<memory>".to_string()))]
fn ucs_to_scf(
    data: &[u8],
    passphrase: Option<String>,
    include_extras: bool,
    label: String,
) -> PyResult<String> {
    let opts = PassphraseOptions {
        explicit: passphrase,
        allow_prompt: false,
        ..PassphraseOptions::default()
    };
    let provider = move || resolve_passphrase(&opts);
    ucs_archive_to_scf(data, &provider, include_extras, &label)
        .map_err(|e| QueryError::new_err(e.to_string()))
}

/// The native BIG-IP query engine binding.
#[pymodule]
fn _engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("QueryError", m.py().get_type::<QueryError>())?;
    m.add_function(wrap_pyfunction!(query, m)?)?;
    m.add_function(wrap_pyfunction!(load_paths, m)?)?;
    m.add_function(wrap_pyfunction!(ucs_to_scf, m)?)?;
    Ok(())
}
