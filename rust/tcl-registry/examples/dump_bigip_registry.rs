//! Dump the BIG-IP object registry as JSONL — one line per spec with its
//! header types and property names. Used by the registry-completeness
//! parity test (`tests/test_bigip_registry_parity.py`) to assert the Rust
//! registry carries every property the Python registry declares.
//!
//! Usage: `cargo run -p tcl-registry --example dump_bigip_registry`

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
        out.push_str(&format!("{{\"headers\":[{h}],\"props\":[{p}]}}\n"));
    }
    print!("{out}");
}
