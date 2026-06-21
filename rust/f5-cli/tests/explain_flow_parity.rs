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

/// Whether a `tshark` binary is available to drive the L7-enrichment paths.
fn tshark_available() -> bool {
    Command::new("tshark")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Whether this tshark accepts the exact EK command the port (and the Python
/// reference) builds — `-T ek -l --no-duplicate-keys`. tshark gained
/// `--no-duplicate-keys` for EK output in 4.4; on older builds the overlay run
/// exits non-zero and the report degrades to `tshark: no` (the Python
/// reference degrades identically — this is the parity contract, so the port
/// keeps the command byte-for-byte rather than "fixing" it locally).
fn tshark_ek_ok() -> bool {
    Command::new("tshark")
        .args(["-r", "explain-flow-matched.pcap", "-T", "ek", "-l", "--no-duplicate-keys"])
        .current_dir(fixtures_dir())
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn tshark_enrichment_marks_used_tshark() {
    // The `--tshark` overlay path runs the built-in walker, then enriches via
    // `tcl_bigip::flow::tshark`. `used_tshark` reflects whether the tshark run
    // succeeded — which depends on this tshark's EK support, exactly as the
    // Python reference does. The byte-level Rust↔Python parity of the enriched
    // fields is checked by the differential harness offline.
    if !tshark_available() {
        eprintln!("skipping: tshark not on PATH");
        return;
    }
    let expect_yes = tshark_ek_ok();
    let (stdout, code) = run_with(
        "explain-flow-matched.pcap",
        "explain-flow-matched.conf",
        &["--tshark"],
    );
    let text = String::from_utf8(stdout).expect("utf8 report");
    assert_eq!(code, 0, "matched VS exits 0");
    let want = if expect_yes { "tshark: yes" } else { "tshark: no" };
    assert!(
        text.contains(want),
        "text header must record `{want}` for this tshark: {text}"
    );

    let (json_out, _) = run_with(
        "explain-flow-matched.pcap",
        "explain-flow-matched.conf",
        &["--tshark", "--json"],
    );
    let json = String::from_utf8(json_out).expect("utf8 json");
    let want_json = format!("\"used_tshark\": {expect_yes}");
    assert!(
        json.contains(&want_json),
        "json must record {want_json}: {json}"
    );
}

#[test]
fn tshark_filter_is_reported_python_repr_style() {
    // `--tshark-filter` makes tshark the canonical flow source: `used_tshark`
    // is true whenever a tshark binary exists (mirroring Python's
    // `bool(flows) or tshark_available()`), independent of EK support, and the
    // filter is echoed in the header rendered like Python's `repr()`.
    if !tshark_available() {
        eprintln!("skipping: tshark not on PATH");
        return;
    }
    let (stdout, _code) = run_with(
        "explain-flow-matched.pcap",
        "explain-flow-matched.conf",
        &["--tshark-filter", "tcp.port == 443"],
    );
    let text = String::from_utf8(stdout).expect("utf8 report");
    assert!(
        text.contains("tshark: yes | filter: 'tcp.port == 443'"),
        "header must echo the filter in Python repr form: {text}"
    );
}

#[test]
fn without_tshark_used_tshark_is_false() {
    // The default built-in-walker path never sets used_tshark, regardless of
    // whether a tshark binary happens to be installed.
    let (json_out, _) = run_with(
        "explain-flow-matched.pcap",
        "explain-flow-matched.conf",
        &["--json"],
    );
    let json = String::from_utf8(json_out).expect("utf8 json");
    assert!(
        json.contains("\"used_tshark\": false"),
        "default path must record used_tshark=false: {json}"
    );
}

#[test]
fn simulate_runs_irule_and_selects_pool() {
    // `--simulate` drives the matched VS's iRule under the embedded TMM-sim
    // orchestrator on `tcl-vm`: the `when HTTP_REQUEST { pool pool_api }` rule
    // selects a pool, records the lb decision, and captures its log line.
    let (stdout, code) = run_with(
        "explain-flow-matched.pcap",
        "explain-flow-simulate.conf",
        &["--simulate"],
    );
    let text = String::from_utf8(stdout).expect("utf8 report");
    assert_eq!(code, 0, "matched capture exits 0: {text}");
    assert!(text.contains("iRule simulation:"), "{text}");
    assert!(text.contains("pool: pool_api"), "{text}");
    assert!(
        text.contains("decision: lb pool_select pool_api"),
        "{text}"
    );
    assert!(text.contains("routing host="), "captured iRule log: {text}");
}

#[test]
fn simulate_json_populates_simulated_fields() {
    let (json_out, code) = run_with(
        "explain-flow-matched.pcap",
        "explain-flow-simulate.conf",
        &["--simulate", "--json"],
    );
    let json = String::from_utf8(json_out).expect("utf8 json");
    assert_eq!(code, 0, "matched capture exits 0");
    assert!(json.contains("\"simulated_pool\": \"pool_api\""), "{json}");
    assert!(
        json.contains("\"simulated_node\": \"192.0.2.30:80\""),
        "{json}"
    );
    assert!(json.contains("\"action\": \"pool_select\""), "{json}");
}
