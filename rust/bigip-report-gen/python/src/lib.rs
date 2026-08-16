// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
use pyo3::exceptions::{PyException, PyIOError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};

use tcl_bigip_io::{
    PassphraseOptions, is_pgp_bytes, is_ucs_bytes, read_ucs_member, resolve_passphrase,
    ucs_archive_to_scf,
};
use tcl_bigip_query::value::Value;
use tcl_bigip_query::{InputSpec, QueryOptions, SideInput, run_query};

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
        Value::Bool(b) => pyo3::types::PyBool::new(py, *b)
            .to_owned()
            .into_any()
            .unbind(),
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
        Value::Container(c) => PyString::new(py, &format!("container({})", c.kind))
            .into_any()
            .unbind(),
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
        let pair = item
            .cast::<PyTuple>()
            .map_err(|_| QueryError::new_err("each source must be a (uri, text) tuple"))?;
        if pair.len() != 2 {
            return Err(QueryError::new_err(
                "each source must be a (uri, text) 2-tuple",
            ));
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
#[pyo3(signature = (expr, sources, *, merge=false, partitions=None, enable_probes=false, per_file=false, side_inputs=None))]
fn query(
    py: Python<'_>,
    expr: &str,
    sources: &Bound<'_, PyAny>,
    merge: bool,
    partitions: Option<HashMap<String, String>>,
    enable_probes: bool,
    per_file: bool,
    // Enrichment side-inputs: `[(name, kind, source_text), …]`, each parsed by
    // the named input format (json/jsonl/csv/f5log/zone) and bound to `$name`
    // for the query to reference (e.g. `$nat`, `$dns`, `$services`, `$cidrNames`).
    side_inputs: Option<Vec<(String, String, String)>>,
) -> PyResult<Py<PyAny>> {
    let sources = coerce_sources(sources)?;
    let side = side_inputs
        .unwrap_or_default()
        .into_iter()
        .map(|(name, kind, source)| SideInput {
            uri: format!("<{kind}:{name}>"),
            name,
            source,
            spec: InputSpec::new(kind),
        })
        .collect::<Vec<_>>();
    let opts = QueryOptions {
        partitions: partitions.unwrap_or_default(),
        enable_probes,
        merge,
        side_inputs: side,
        ..QueryOptions::default()
    };

    let result =
        run_query(expr, &sources, &opts).map_err(|e| QueryError::new_err(e.to_string()))?;
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
            out.append(PyTuple::new(
                py,
                [PyString::new(py, uri).into_any(), vals.into_any()],
            )?)?;
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
fn read_one(
    path: &str,
    passphrase: &Option<String>,
    include_extras: bool,
) -> PyResult<(String, String)> {
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

/// Project every `sys file ssl-cert` object from one or more configs.
///
/// The `f5-query` DSL only projects the `ltm` module, so the certificate
/// inventory is read from the parsed [`tcl_bigip`] model directly (the same
/// `parse_bigip_conf` the query engine is built on). Returns a JSON string — an
/// ordered array of `{name, full_path, subject, issuer, expiration_string,
/// expiration_date, fingerprint, key_type, key_size, serial_number,
/// subject_alternative_name, is_bundle, source_path, …}` field maps (snake-cased
/// canonical fields) — which the pure-Python report layer shapes and
/// cross-references against the SSL profiles that use each cert.
#[pyfunction]
fn sys_file_ssl_certs(sources: &Bound<'_, PyAny>) -> PyResult<String> {
    collect_sys_file_objects(sources, SysFileKind::Cert)
}

/// Project every `sys file ssl-cert`, enriched with the **real certificate**
/// parsed out of the UCS filestore.
///
/// Modern BIG-IP exports carry a metadata-free `sys file ssl-cert` stanza — the
/// subject / issuer / validity / SANs live only in the PEM in the filestore, not
/// in the config text — so a report built from the config alone shows a blank
/// certificate row. For every source whose `uri` is a readable `.ucs` on disk,
/// this reads the cert's `cache-path` (falling back to `source-path`) member out
/// of the archive and runs the same `x509_parse` the engine uses, attaching:
///   * `x509` — the leaf cert dict (`subject`, `issuer`, `not_before`,
///     `not_after`, `serial`, `sans`, `key_alg`, `key_size`, `sig_alg`,
///     `fingerprint_sha256`, …). `not_before` (the issue date) and the exact
///     SAN list are only recoverable this way.
///   * `x509_embedded` — the remaining PEM blocks when the file is a bundle
///     (leaf + intermediate(s)/root), leaf-first, for the chain view.
/// Sources without a locatable file (a plain `bigip.conf`, or a source with no
/// filesystem backing) keep just the config-metadata fields.
#[pyfunction]
#[pyo3(signature = (sources, *, passphrase=None))]
fn sys_file_ssl_certs_x509(
    sources: &Bound<'_, PyAny>,
    passphrase: Option<String>,
) -> PyResult<String> {
    use tcl_bigip::canonical::Canon;
    use tcl_bigip::model::ModelObject;
    use tcl_bigip::parser::driver::parse_bigip_conf;

    let sources = coerce_sources(sources)?;
    let mut out: Vec<serde_json::Value> = Vec::new();
    for (uri, text) in &sources {
        // Read the archive bytes once per source, but only when the uri actually
        // resolves to a UCS/OpenPGP file — a plain config has no filestore.
        let ucs_bytes: Option<Vec<u8>> = std::fs::read(uri)
            .ok()
            .filter(|raw| is_pgp_bytes(raw) || is_ucs_bytes(raw));

        let config = parse_bigip_conf(text, "Common");
        for placed in &config.objects {
            let ModelObject::SysFileSslCert(c) = &placed.object else {
                continue;
            };
            let serde_json::Value::Object(mut fields) = c.canon_fields() else {
                continue;
            };
            fields
                .entry("full_path".to_owned())
                .or_insert_with(|| serde_json::Value::String(placed.full_path.clone()));

            if let Some(raw) = &ucs_bytes {
                let member = fields
                    .get("cache_path")
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        fields
                            .get("source_path")
                            .and_then(serde_json::Value::as_str)
                            .map(|s| s.strip_prefix("file://").unwrap_or(s))
                            .filter(|s| !s.is_empty())
                    });
                if let Some(member) = member {
                    let opts = PassphraseOptions {
                        explicit: passphrase.clone(),
                        allow_prompt: false,
                        ..PassphraseOptions::default()
                    };
                    let provider = || resolve_passphrase(&opts);
                    if let Ok(pem) = read_ucs_member(raw, member, &provider, uri) {
                        let pem_str = String::from_utf8_lossy(&pem);
                        let parsed = tcl_bigip_query::probes::x509_parse_all(&pem_str);
                        let mut iter = parsed.into_iter();
                        if let Some(leaf) = iter.next() {
                            fields.insert("x509".to_owned(), value_to_json(&leaf));
                            let rest: Vec<serde_json::Value> =
                                iter.map(|v| value_to_json(&v)).collect();
                            if !rest.is_empty() {
                                fields.insert(
                                    "x509_embedded".to_owned(),
                                    serde_json::Value::Array(rest),
                                );
                            }
                        }
                    }
                }
            }
            out.push(serde_json::Value::Object(fields));
        }
    }
    serde_json::to_string(&serde_json::Value::Array(out))
        .map_err(|e| QueryError::new_err(e.to_string()))
}

/// Convert an engine [`Value`] to a `serde_json::Value` via the canonical
/// compact JSON encoder (order-preserving), so x509 dicts embed cleanly.
fn value_to_json(v: &Value) -> serde_json::Value {
    serde_json::from_str(&tcl_bigip_query::jsonfmt::to_compact(v))
        .unwrap_or(serde_json::Value::Null)
}

/// Project every `sys file ssl-key` object from one or more configs.
///
/// Companion to [`sys_file_ssl_certs`] — the pure-Python report layer pairs each
/// key with its certificate to surface the private-key `security_type` and
/// (`f5mku`-encrypted) `passphrase`. Returns the same snake-cased JSON array.
#[pyfunction]
fn sys_file_ssl_keys(sources: &Bound<'_, PyAny>) -> PyResult<String> {
    collect_sys_file_objects(sources, SysFileKind::Key)
}

enum SysFileKind {
    Cert,
    Key,
}

fn collect_sys_file_objects(sources: &Bound<'_, PyAny>, kind: SysFileKind) -> PyResult<String> {
    use tcl_bigip::canonical::Canon;
    use tcl_bigip::model::ModelObject;
    use tcl_bigip::parser::driver::parse_bigip_conf;

    let sources = coerce_sources(sources)?;
    let mut out: Vec<serde_json::Value> = Vec::new();
    for (_uri, text) in &sources {
        let config = parse_bigip_conf(text, "Common");
        for placed in &config.objects {
            let fields = match (&kind, &placed.object) {
                (SysFileKind::Cert, ModelObject::SysFileSslCert(c)) => c.canon_fields(),
                (SysFileKind::Key, ModelObject::SysFileSslKey(k)) => k.canon_fields(),
                _ => continue,
            };
            let serde_json::Value::Object(mut fields) = fields else {
                continue;
            };
            fields
                .entry("full_path".to_owned())
                .or_insert_with(|| serde_json::Value::String(placed.full_path.clone()));
            out.push(serde_json::Value::Object(fields));
        }
    }
    serde_json::to_string(&serde_json::Value::Array(out))
        .map_err(|e| QueryError::new_err(e.to_string()))
}

/// Decrypt every `f5mku` `$M$…` secret in `text` with the base64 master key.
///
/// `master_key` is the value `f5mku -K` prints on the device. Every encrypted
/// secret-bearing field (SSL key passphrases, monitor / RADIUS / SNMP secrets,
/// …) is rewritten in place to its clear text; non-secret and already-clear
/// values are left untouched. Raises [`QueryError`] if the config has encrypted
/// secrets but none could be decrypted (a wrong / malformed master key).
#[pyfunction]
fn decrypt_secrets(text: &str, master_key: &str) -> PyResult<String> {
    let master_key = master_key.trim();
    if master_key.is_empty() {
        return Ok(text.to_owned());
    }
    let mut first_err: Option<String> = None;
    let report = tcl_bigip::secrets::rewrite_secrets(text, |val| {
        if !tcl_f5mku::is_ciphertext(val) {
            return None;
        }
        match tcl_f5mku::decrypt(val, master_key) {
            Ok(plain) => Some(plain),
            Err(e) => {
                if first_err.is_none() {
                    first_err = Some(e.to_string());
                }
                None
            }
        }
    });
    if report.transformed == 0
        && let Some(e) = first_err
    {
        return Err(QueryError::new_err(format!("f5mku decryption failed: {e}")));
    }
    Ok(report.new_source)
}

/// Inventory the secret-bearing fields in a config as JSON.
///
/// Returns an ordered array of `{object, field, value, encrypted}` maps — every
/// credential-bearing field (SSL key passphrases, monitor / RADIUS / SNMP
/// secrets), tagged with the object it lives in and whether it is still an
/// `f5mku` `$M$…` ciphertext. Powers the report's Secrets tab; run the text
/// through [`decrypt_secrets`] first to get clear values.
#[pyfunction]
fn list_secrets(text: &str) -> PyResult<String> {
    let out: Vec<serde_json::Value> = tcl_bigip::secrets::collect_secrets(text)
        .iter()
        .map(tcl_bigip::secrets::FoundSecret::to_json)
        .collect();
    serde_json::to_string(&serde_json::Value::Array(out))
        .map_err(|e| QueryError::new_err(e.to_string()))
}

/// Run every model-level BIG-IP configuration diagnostic as JSON.
#[pyfunction]
fn config_diagnostics(text: &str) -> PyResult<String> {
    serde_json::to_string(&bigip_report_gen_rust::collect_config_diagnostics(text))
        .map_err(|e| QueryError::new_err(e.to_string()))
}

/// Run every report-relevant F5 diagnostic for one source, including the iApp
/// presentation/implementation checks when the supplied source set contains a
/// conventional pair in one directory.
#[pyfunction]
fn report_diagnostics(uri: &str, text: &str, sources: Vec<(String, String)>) -> PyResult<String> {
    serde_json::to_string(&bigip_report_gen_rust::collect_report_diagnostics(
        uri, text, &sources,
    ))
    .map_err(|e| QueryError::new_err(e.to_string()))
}

/// Describe whether the full iApp presentation/implementation evidence was
/// available for a source. This keeps a partial report from looking clean.
#[pyfunction]
fn iapp_diagnostic_evidence(uri: &str, sources: Vec<(String, String)>) -> PyResult<String> {
    serde_json::to_string(&bigip_report_gen_rust::collect_iapp_diagnostic_evidence(
        uri, &sources,
    ))
    .map_err(|e| QueryError::new_err(e.to_string()))
}

/// Syntax-highlight an iRule/Tcl source string to HTML.
///
/// Uses the real [`tcl_lexer`] tokeniser (the same one the LSP/compiler use) with
/// the **`f5-irules`** lexer preset, so `if {expr}{body}` (`}{` valid in TMM) is
/// tokenised correctly rather than the stock-Tcl way. Returns self-contained,
/// pre-escaped HTML (`<span class="tk-…">`).
#[pyfunction]
fn highlight_tcl(body: &str) -> String {
    tcl_lexer::highlight_tcl_with_config(body, tcl_lexer::LexerConfig::for_dialect("f5-irules"))
}

/// Render Markdown (CommonMark + GFM) to an HTML fragment for report
/// front-matter. Delegates to the single shared renderer so the Python, Rust
/// and wasm backends produce identical output; raw HTML is stripped.
#[pyfunction]
fn render_markdown(md: &str) -> String {
    bigip_report_gen_rust::render_markdown(md)
}

/// Sort iRule `when` event names into canonical firing order.
///
/// Wraps the shared [`tcl_registry`] event registry — the same firing-order
/// table the Rust report and LSP use — so the Python and Rust report models
/// list a rule's events identically (not alphabetically). Events absent from
/// the master order are appended in sorted order, so none are dropped.
#[pyfunction]
fn order_events(events: Vec<String>) -> Vec<String> {
    tcl_registry::events::EventRegistry::build().order_events(&events)
}

/// Sort a virtual server's attached profiles into traffic (protocol-stack)
/// order — transport → TLS → application → … — the order the device processes
/// them, not the arbitrary config order.
///
/// Wraps the shared [`tcl_bigip_query`] traffic-order core so the Python and
/// Rust report models order profiles identically. `types` optionally maps a
/// profile full-path to its authoritative profile type (e.g. from the report's
/// typed profile inventory); profiles it doesn't cover fall back to well-known
/// default-profile-name inference. Layering always comes from the registry.
#[pyfunction]
#[pyo3(signature = (profiles, types=None))]
fn order_profiles(profiles: Vec<String>, types: Option<HashMap<String, String>>) -> Vec<String> {
    let types = types.unwrap_or_default();
    tcl_bigip_query::builtins::f5profile::order_profiles_with_types(&profiles, &types)
}

/// Build a control-flow graph for an iRule as `{nodes, edges}` JSON.
///
/// Uses the IR-based [`tcl_diagram`] (the same control-flow extraction the CLI
/// and MCP use), serialised to a deterministic graph for the report's elkjs
/// renderer — no LLM, no network. Returns `""` when there is nothing to draw.
#[pyfunction]
fn irule_flowchart(body: &str) -> String {
    tcl_diagram::irule_flowchart_graph(body, tcl_registry::registry_for_dialect("f5-irules"))
}

/// Reconstruct the object-name patterns an iRule could dynamically attach.
///
/// Returns a JSON string `{"pools": [...], "nodes": [...], "snatpools": [...]}`
/// where each entry is `{raw, prefix, contains, suffix, glob, exact,
/// unconstrained}`. The report's orphan analysis uses these prefix / contained /
/// suffix fragments to filter candidate objects by name instead of demoting a
/// whole object type — powered by the same [`tcl_diagram::attach_reach`] the
/// Rust/WASM report uses (real IR walk + `set` constant propagation).
#[pyfunction]
fn irule_attach_patterns(body: &str) -> String {
    tcl_diagram::irule_attach_patterns(body).to_string()
}

/// iRule names referenced via the F5 cross-iRule `call <rule>::<proc>` form.
///
/// Returns a JSON array of the `<rule>` names (the iRule that defines the proc);
/// the caller resolves each to an actual iRule so a rule that only exists to
/// provide procs is linked in (not orphaned) and shows on its callers.
#[pyfunction]
fn irule_proc_call_refs(body: &str) -> String {
    serde_json::to_string(&tcl_diagram::proc_call_refs(body)).unwrap_or_else(|_| "[]".to_string())
}

/// Build the cross-device *architecture* model (roles, tiers, inter-device
/// links, and an elkjs topology graph) from the report's device list.
///
/// `devices_json` is the JSON array of shaped device objects (`collect_model`'s
/// `devices`); `manifest` is an optional architecture manifest (a small Tcl
/// script declaring per-device role/tier and explicit links). Auto-detection
/// (IP overlap between one device's members and another's virtuals) always
/// runs; the manifest overrides it. Returns the `architecture` object as a JSON
/// string — reusing the Rust generator's detection so the Python report exposes
/// the identical tier model (and cross-device diagram).
#[pyfunction]
#[pyo3(signature = (devices_json, manifest=None))]
fn build_architecture(devices_json: &str, manifest: Option<&str>) -> PyResult<String> {
    let devices: Vec<serde_json::Value> = serde_json::from_str(devices_json)
        .map_err(|e| QueryError::new_err(format!("invalid devices JSON: {e}")))?;
    let arch = bigip_report_gen_rust::build_architecture(&devices, manifest);
    serde_json::to_string(&arch).map_err(|e| QueryError::new_err(e.to_string()))
}

/// The full f5-query manual (grammar + builtins + cookbook), for embedding a
/// reference panel next to the report's query console.
#[pyfunction]
fn manual() -> String {
    tcl_bigip_query::manual::format_manual()
}

/// K5903 BIG-IP release lifecycle metadata for the report model.
#[pyfunction]
fn bigip_release_lifecycle(version: &str) -> PyResult<String> {
    serde_json::to_string(&bigip_report_gen_rust::bigip_release_lifecycle(version))
        .map_err(|e| QueryError::new_err(e.to_string()))
}

/// Build the address/port *enrichment* block from the model devices and the
/// architecture object (zone + CIDR-name labels for addresses, service names
/// for ports). Reuses the Rust resolver so the Python report matches the native
/// generator. Returns the `enrichment` object as JSON.
#[pyfunction]
fn build_enrichment(devices_json: &str, architecture_json: &str) -> PyResult<String> {
    let devices: Vec<serde_json::Value> = serde_json::from_str(devices_json)
        .map_err(|e| QueryError::new_err(format!("invalid devices JSON: {e}")))?;
    let arch: serde_json::Value = serde_json::from_str(architecture_json)
        .map_err(|e| QueryError::new_err(format!("invalid architecture JSON: {e}")))?;
    let e = bigip_report_gen_rust::enrich::build_enrichment(&devices, &arch);
    serde_json::to_string(&e).map_err(|e| QueryError::new_err(e.to_string()))
}

/// The native BIG-IP query engine binding.
#[pymodule]
fn _engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // `tcl_version::VERSION`, not `CARGO_PKG_VERSION`: the workspace manifest
    // carries `0.1.0` and is never bumped (releases are tag-only), so the
    // manifest version would have `f5-report --version` report `engine 0.1.0`
    // forever. `tcl-version` resolves TCL_LSP_VERSION, else `git describe`.
    m.add("__version__", tcl_version::VERSION)?;
    m.add("__git_hash__", env!("GIT_HASH"))?;
    // Single `git describe --tags` version (v-tag + commits + hash) for the footer.
    m.add("__git_describe__", env!("GIT_DESCRIBE"))?;
    m.add("QueryError", m.py().get_type::<QueryError>())?;
    m.add_function(wrap_pyfunction!(query, m)?)?;
    m.add_function(wrap_pyfunction!(load_paths, m)?)?;
    m.add_function(wrap_pyfunction!(ucs_to_scf, m)?)?;
    m.add_function(wrap_pyfunction!(sys_file_ssl_certs, m)?)?;
    m.add_function(wrap_pyfunction!(sys_file_ssl_certs_x509, m)?)?;
    m.add_function(wrap_pyfunction!(sys_file_ssl_keys, m)?)?;
    m.add_function(wrap_pyfunction!(decrypt_secrets, m)?)?;
    m.add_function(wrap_pyfunction!(list_secrets, m)?)?;
    m.add_function(wrap_pyfunction!(config_diagnostics, m)?)?;
    m.add_function(wrap_pyfunction!(report_diagnostics, m)?)?;
    m.add_function(wrap_pyfunction!(iapp_diagnostic_evidence, m)?)?;
    m.add_function(wrap_pyfunction!(highlight_tcl, m)?)?;
    m.add_function(wrap_pyfunction!(render_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(order_events, m)?)?;
    m.add_function(wrap_pyfunction!(order_profiles, m)?)?;
    m.add_function(wrap_pyfunction!(irule_flowchart, m)?)?;
    m.add_function(wrap_pyfunction!(irule_attach_patterns, m)?)?;
    m.add_function(wrap_pyfunction!(irule_proc_call_refs, m)?)?;
    m.add_function(wrap_pyfunction!(build_architecture, m)?)?;
    m.add_function(wrap_pyfunction!(build_enrichment, m)?)?;
    m.add_function(wrap_pyfunction!(bigip_release_lifecycle, m)?)?;
    m.add_function(wrap_pyfunction!(manual, m)?)?;
    Ok(())
}
