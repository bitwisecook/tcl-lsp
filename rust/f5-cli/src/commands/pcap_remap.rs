//! The `pcap-remap` (`pcapmap`) verb — apply a redaction map to a PCAP.
//!
//! Port of `tooling/f5/verbs/pcap_remap.py` (`_run_pcap_remap`). Rewrites every
//! IPv4 / IPv6 source and destination address in a PCAP / PCAPNG capture using
//! the same TOML map produced by `f5 redact`, recomputing the IPv4 header and
//! TCP / UDP / ICMP / ICMPv6 checksums, plus the peer-IP fields in the F5
//! Ethernet trailer. See [`tcl_bigip::pcap_remap`].
//!
//! Custom `--schema` overlays (extra F5-trailer layouts as TOML) are supported:
//! each file is parsed by [`tcl_bigip::f5_trailer::load_schema_overlay`] and
//! merged (later files win), then threaded through the remap engine and the
//! `--list-schemas` summary.

#![allow(clippy::doc_markdown)]

use std::path::Path;

use tcl_bigip::f5_trailer::{SchemaOverlay, load_schema_overlay, schema_summary_with};
use tcl_bigip::pcap_remap::{PcapError, UnknownPolicy, remap_pcap_with};
use tcl_bigip::redact::RedactionMap;

/// Load and merge every `--schema` overlay file (later files override earlier
/// keys, matching the Python registry).
fn load_overlays(schema: &[std::path::PathBuf]) -> Result<SchemaOverlay, String> {
    let mut overlay = SchemaOverlay::default();
    for path in schema {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read schema {}: {e}", path.display()))?;
        overlay.merge(load_schema_overlay(&text).map_err(|e| format!("{}: {e}", path.display()))?);
    }
    Ok(overlay)
}

/// `f5 pcap-remap`. Errors are reported to stderr and surfaced as exit code 2
/// (matching the Python verb), so the handler returns the code directly.
pub fn run_pcap_remap(
    map_file: &Path,
    input: &Path,
    output: &Path,
    reverse: bool,
    on_unknown: &str,
    schema: &[std::path::PathBuf],
    list_schemas: bool,
) -> u8 {
    let overlay = match load_overlays(schema) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    if list_schemas {
        let summary = schema_summary_with(&overlay);
        println!("legacy:");
        for line in &summary.legacy {
            println!("  {line}");
        }
        println!("dpt:");
        for line in &summary.dpt {
            println!("  {line}");
        }
        return 0;
    }

    let map_text = match std::fs::read_to_string(map_file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    let mut rm = match RedactionMap::from_toml(&map_text) {
        Ok(rm) => rm,
        Err(e) => {
            eprintln!("error: cannot load map: {e}");
            return 2;
        }
    };

    let policy = match on_unknown {
        "preserve" => UnknownPolicy::Preserve,
        "sweep" => UnknownPolicy::Sweep,
        _ => UnknownPolicy::Error,
    };

    let input_bytes = match std::fs::read(input) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    let (out_bytes, result) =
        match remap_pcap_with(&input_bytes, &mut rm, reverse, policy, &overlay) {
            Ok(pair) => pair,
            Err(exc @ PcapError::UnknownTrailer { .. }) => {
                // The output may have been partially written by Python; we never
                // wrote it, but remove any stale file for parity.
                let _ = std::fs::remove_file(output);
                eprintln!(
                    "error: {exc}\n  -> rerun with --on-unknown=preserve|sweep, or supply a \
                 matching --schema file to add the missing layout."
                );
                return 2;
            }
            Err(e) => {
                eprintln!("error: {e}");
                return 2;
            }
        };

    if let Err(e) = std::fs::write(output, &out_bytes) {
        eprintln!("error: {e}");
        return 2;
    }

    eprintln!(
        "pcap-remap: {}/{} packet(s) rewritten, {} address(es) changed; \
         trailer TLVs: {} rewritten / {} unknown / {} total",
        result.packets_rewritten,
        result.packets_total,
        result.addresses_rewritten,
        result.trailer_tlvs_rewritten,
        result.trailer_tlvs_unknown,
        result.trailer_tlvs_total,
    );
    0
}
