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

/// Run `f5-query explain-flow <pcap> <conf> [extra…]` from the fixtures dir,
/// returning `(stdout, exit_code)`.
fn run_with(pcap: &str, conf: &str, extra: &[&str]) -> (Vec<u8>, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .current_dir(fixtures_dir())
        .arg("explain-flow")
        .arg(pcap)
        .arg(conf)
        .args(extra)
        .output()
        .expect("run f5-query explain-flow");
    (out.stdout, out.status.code().unwrap_or(-1))
}

fn run_explain_flow(pcap: &str, conf: &str) -> (Vec<u8>, i32) {
    run_with(pcap, conf, &[])
}

fn assert_unmatched_parity(pcap: &str) {
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
    assert_unmatched_parity("pcap-remap-sample.pcap");
}

#[test]
fn pcapng_flows_match_golden() {
    assert_unmatched_parity("pcap-remap-sample.pcapng");
}

#[test]
fn libpcap_unknown_trailer_match_golden() {
    assert_unmatched_parity("pcap-remap-unknown.pcap");
}

#[test]
fn matched_virtual_with_back_side_matches_golden() {
    // A `:np`-style capture whose front flow targets the sample VS: exercises
    // profile-chain resolution (with types), observed pool member + SNAT from
    // the back side, the TLS/HTTP captured request, and the reused `f5 explain`
    // resolved-plan block. The matched VS carries no iRules, so the event /
    // policy sections (later increments) stay absent.
    let (stdout, code) = run_explain_flow("explain-flow-matched.pcap", "explain-flow-matched.conf");
    assert_eq!(
        stdout,
        golden("explain-flow-matched.golden"),
        "matched explain-flow stdout must match the Python golden"
    );
    assert_eq!(code, 0, "exit code for a matched capture");
}

#[test]
fn matched_virtual_with_ltm_policies_matches_golden() {
    // Three LTM policies exercise the evaluator end to end: all-match (multiple
    // FIRED rules), best-match-approx (condition-count scoring), and first-match
    // — across the equals / contains+case-insensitive / starts-with / ends-with
    // operators, the geoip unsupported-operand note, a header-not-seen note, and
    // forward / http-header / http-uri / tcp actions.
    let (stdout, code) = run_explain_flow("explain-flow-matched.pcap", "explain-flow-policy.conf");
    assert_eq!(
        stdout,
        golden("explain-flow-policy.golden"),
        "matched explain-flow with LTM policy decisions must match the Python golden"
    );
    assert_eq!(code, 0, "exit code for a matched capture");
}

#[test]
fn matched_virtual_with_irule_events_matches_golden() {
    // A rich iRule exercises the full event chain: ordered firing sequence
    // (incl. the sorted "extra" custom event and correctly-omitted unfired
    // events), verbatim event bodies, the LB::server back-side synthesis, and
    // every HUD command family (IP/TCP/SSL/SNI/HTTP/header).
    let (stdout, code) = run_explain_flow("explain-flow-matched.pcap", "explain-flow-rich.conf");
    assert_eq!(
        stdout,
        golden("explain-flow-rich.golden"),
        "matched explain-flow with iRule events must match the Python golden"
    );
    assert_eq!(code, 0, "exit code for a matched capture");
}

#[test]
fn json_output_matches_golden() {
    // `--json` mirrors `report_to_dict` serialised like `json.dumps(indent=2)`:
    // the full per-flow dicts, the event/annotation/policy structures, and the
    // empty `simulated_*` fields. Covered for an iRule-event capture, a
    // policy-bearing capture, and an unmatched multi-flow capture.
    for (pcap, conf, golden_name) in [
        (
            "explain-flow-matched.pcap",
            "explain-flow-rich.conf",
            "explain-flow-rich.json.golden",
        ),
        (
            "explain-flow-matched.pcap",
            "explain-flow-policy.conf",
            "explain-flow-policy.json.golden",
        ),
        (
            "pcap-remap-sample.pcap",
            "explain-flow-sample.conf",
            "explain-flow-unmatched.json.golden",
        ),
    ] {
        let (stdout, _code) = run_with(pcap, conf, &["--json"]);
        assert_eq!(
            stdout,
            golden(golden_name),
            "explain-flow --json for {conf} must match the Python golden"
        );
    }
}
