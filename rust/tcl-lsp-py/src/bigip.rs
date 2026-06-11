//! `PyO3` binding for the F5 BIG-IP config parser.
//!
//! [`bigip_parse_conf_json`] parses a BIG-IP `.conf` / `.scf` source with
//! the native Rust parser and returns the canonical JSON document the
//! Python `dialects.f5.bigip.parser._rust_bridge.rebuild` reconstructs
//! the typed dataclasses from — the path the `f5` command consumes the
//! Rust parser through.

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

/// Parse `source` as a BIG-IP configuration and return its canonical
/// JSON document (a string). `default_partition` names the partition the
/// source belongs to (bare object names are rewritten under it).
#[pyfunction]
#[pyo3(signature = (source, default_partition = "Common"))]
#[must_use]
pub fn bigip_parse_conf_json(source: &str, default_partition: &str) -> String {
    let config = tcl_bigip::parser::parse_bigip_conf(source, default_partition);
    let json = tcl_bigip::canonical::config_to_canonical(&config);
    serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_owned())
}

/// Register the BIG-IP bindings on the module.
pub fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(bigip_parse_conf_json, m)?)?;
    Ok(())
}
