//! The `pcap-remap` (`pcapmap`) verb — apply a redaction map to a PCAP.
//!
//! Port of `tooling/f5/verbs/pcap_remap.py` (`_run_pcap_remap`). Rewrites every
//! IPv4 / IPv6 source and destination address in a PCAP / PCAPNG capture using
//! the same TOML map produced by `f5 redact`, recomputing the IPv4 header and
//! TCP / UDP / ICMP / ICMPv6 checksums, plus the peer-IP fields in the F5
//! Ethernet trailer. See [`tcl_bigip::pcap_remap`].
//!
//! Note: the `--schema` overlay (custom TOML schema) is deferred; the built-in
//! legacy + DPT schemas are ported. Passing `--schema` is rejected cleanly.

#![allow(clippy::doc_markdown)]

use std::path::Path;

use tcl_bigip::f5_trailer::schema_summary;
use tcl_bigip::pcap_remap::{PcapError, UnknownPolicy, remap_pcap};
use tcl_bigip::redact::RedactionMap;

/// `f5 pcap-remap`.
pub fn run_pcap_remap(
    map_file: &Path,
    input: &Path,
    output: &Path,
    reverse: bool,
    on_unknown: &str,
    schema: &[std::path::PathBuf],
    list_schemas: bool,
) -> anyhow::Result<u8> {
    // Deferred: custom `--schema` overlays are not yet ported (the built-in
    // legacy + DPT schemas are). Reject cleanly so output is never half-written.
    if !schema.is_empty() {
        anyhow::bail!(
            "`f5 pcap-remap --schema` (custom schema overlay) is not yet ported in the Rust port"
        );
    }

    if list_schemas {
        let summary = schema_summary();
        println!("legacy:");
        for line in &summary.legacy {
            println!("  {line}");
        }
        println!("dpt:");
        for line in &summary.dpt {
            println!("  {line}");
        }
        return Ok(0);
    }

    let map_text = match std::fs::read_to_string(map_file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    let mut rm = match RedactionMap::from_toml(&map_text) {
        Ok(rm) => rm,
        Err(e) => {
            eprintln!("error: cannot load map: {e}");
            return Ok(2);
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
            return Ok(2);
        }
    };

    let (out_bytes, result) = match remap_pcap(&input_bytes, &mut rm, reverse, policy) {
        Ok(pair) => pair,
        Err(exc @ PcapError::UnknownTrailer { .. }) => {
            // The output may have been partially written by Python; we never
            // wrote it, but remove any stale file for parity.
            let _ = std::fs::remove_file(output);
            eprintln!(
                "error: {exc}\n  -> rerun with --on-unknown=preserve|sweep, or supply a \
                 matching --schema file to add the missing layout."
            );
            return Ok(2);
        }
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    if let Err(e) = std::fs::write(output, &out_bytes) {
        eprintln!("error: {e}");
        return Ok(2);
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
    Ok(0)
}
