//! Dump the BIG-IP object registry as JSONL — one line per spec with its
//! header types and property names. Used by the registry parity test
//! to assert the registry carries every property the golden registry declares.
//!
//! Usage: `cargo run -p tcl-registry --example dump_bigip_registry`

use std::fmt::Write as _;

fn main() {
    let reg = tcl_registry::bigip::BigipRegistry::build();
    let mut out = String::new();
    for spec in reg.specs() {
        let headers: Vec<String> = spec
            .header_types
            .iter()
            .map(|(m, o)| format!("{m}|{o}"))
            .collect();
        let props: Vec<&str> = spec.properties.iter().map(|p| p.name).collect();
        // Minimal JSON, no external serialiser dependency.
        let h = headers
            .iter()
            .map(|s| format!("{s:?}"))
            .collect::<Vec<_>>()
            .join(",");
        let p = props
            .iter()
            .map(|s| format!("{s:?}"))
            .collect::<Vec<_>>()
            .join(",");
        let _ = writeln!(out, "{{\"headers\":[{h}],\"props\":[{p}]}}");
    }
    print!("{out}");
}
