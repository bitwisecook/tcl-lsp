//! Differential parity tests for the `f5 explain-flow` verb (static path) — the
//! PCAP flow tracer (`tcl-bigip::flow` + the `explain_flow` driver).
//!
//! Runs the built `f5-query` binary against committed libpcap / PCAPNG fixtures
//! plus a small `bigip.conf`, and asserts stdout is byte-for-byte identical to a
//! golden produced by `python -m tooling.f5.main explain-flow`. The binary runs
//! with its working directory set to the fixtures dir and bare filenames, so the
//! `explain-flow: <pcap>` header line is path-stable across machines.
//!
//! These captures do not target the sample VS, so they exercise the
//! flow-extraction + session-pairing + unmatched-report path end to end
//! (libpcap and pcapng readers, IPv4/IPv6, TCP/UDP/ICMP, HTTP request peek).
//! Self-contained: no Python at test time.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn golden(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("read golden {}: {e}", path.display()))
}

/// Run `f5-query explain-flow <pcap> <conf>` from the fixtures dir, returning
/// `(stdout, exit_code)`.
fn run_explain_flow(pcap: &str, conf: &str) -> (Vec<u8>, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .current_dir(fixtures_dir())
        .arg("explain-flow")
        .arg(pcap)
        .arg(conf)
        .output()
        .expect("run f5-query explain-flow");
    (out.stdout, out.status.code().unwrap_or(-1))
}

fn assert_parity(pcap: &str) {
    let (stdout, code) = run_explain_flow(pcap, "explain-flow-sample.conf");
    assert_eq!(
        stdout,
        golden(&format!("explain-flow-{pcap}.golden")),
        "explain-flow stdout for {pcap} must match the Python golden"
    );
    // No session matches the sample VS, so the verb exits 1.
    assert_eq!(code, 1, "exit code for {pcap}");
}

#[test]
fn libpcap_mixed_flows_match_golden() {
    // 7 flows: HTTP GET, UDP, ICMP, TCP, IPv6, same-host, plain TCP.
    assert_parity("pcap-remap-sample.pcap");
}

#[test]
fn pcapng_flows_match_golden() {
    assert_parity("pcap-remap-sample.pcapng");
}

#[test]
fn libpcap_unknown_trailer_match_golden() {
    assert_parity("pcap-remap-unknown.pcap");
}
