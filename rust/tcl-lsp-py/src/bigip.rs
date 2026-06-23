//! `PyO3` binding for the F5 BIG-IP config parser.
//!
//! [`bigip_parse_conf_json`] parses a BIG-IP `.conf` / `.scf` source with
//! the native Rust parser and returns the canonical JSON document the
//! Python F5 bridge reconstructs the typed dataclasses from — the path
//! the `f5` command consumes the Rust parser through.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

/// Parse `source` as a BIG-IP configuration and return its canonical
/// JSON document (a string). `default_partition` names the partition the
/// source belongs to (bare object names are rewritten under it).
#[pyfunction]
#[pyo3(signature = (source, default_partition = "Common"))]
#[must_use]
pub fn bigip_parse_conf_json(py: Python<'_>, source: &str, default_partition: &str) -> String {
    let source = source.to_owned();
    let default_partition = default_partition.to_owned();
    py.detach(move || {
        let config = tcl_bigip::parser::parse_bigip_conf(&source, &default_partition);
        let json = tcl_bigip::canonical::config_to_canonical(&config);
        serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_owned())
    })
}

/// Parse `source` as F5 iApp APL (presentation language) and return its
/// canonical JSON document, which the Python F5 bridge reconstructs the
/// `AplModel` dataclasses from.
#[pyfunction]
#[pyo3(signature = (source))]
#[must_use]
pub fn apl_parse_json(py: Python<'_>, source: &str) -> String {
    let source = source.to_owned();
    py.detach(move || {
        let model = tcl_bigip::apl::parse_apl(&source);
        let json = tcl_bigip::apl::model_to_canonical(&model);
        serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_owned())
    })
}

/// Register the BIG-IP bindings on the module.
pub fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(bigip_parse_conf_json, m)?)?;
    m.add_function(wrap_pyfunction!(apl_parse_json, m)?)?;
    Ok(())
}
