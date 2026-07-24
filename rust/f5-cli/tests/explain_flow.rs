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

//! Tests for the `f5 explain-flow` verb (static path) — the PCAP flow tracer
//! (`tcl-bigip::flow` + the `explain_flow` driver).
//!
//! Runs the built `f5-query` binary against a committed libpcap fixture plus
//! small `bigip.conf` variants, and asserts on specific substrings of stdout
//! / the `--json` output. The binary runs with its working directory set to
//! the fixtures dir and bare filenames, so output is path-stable across
//! machines.
//!
//! Covers the `--tshark` / `--tshark-filter` enrichment overlay (`used_tshark`
//! bookkeeping and the repr-style filter echo) and the `--simulate` iRule
//! simulation path (pool selection + captured log lines).

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
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

/// Whether a `tshark` binary is available to drive the L7-enrichment paths.
fn tshark_available() -> bool {
    Command::new("tshark")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Whether this tshark accepts the exact EK command the CLI builds —
/// `-T ek -l --no-duplicate-keys`. tshark gained `--no-duplicate-keys` for EK
/// output in 4.4; on older builds the overlay run exits non-zero and the
/// report degrades to `tshark: no` (the golden captures this same degradation,
/// so the command is kept as-is rather than "fixed" locally).
fn tshark_ek_ok() -> bool {
    Command::new("tshark")
        .args([
            "-r",
            "explain-flow-matched.pcap",
            "-T",
            "ek",
            "-l",
            "--no-duplicate-keys",
        ])
        .current_dir(fixtures_dir())
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn tshark_enrichment_marks_used_tshark() {
    // The `--tshark` overlay path runs the built-in walker, then enriches via
    // `tcl_bigip::flow::tshark`. `used_tshark` reflects whether the tshark run
    // succeeded — which depends on this tshark's EK support, exactly as the
    // golden was captured. The byte-level output of the enriched
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
    let want = if expect_yes {
        "tshark: yes"
    } else {
        "tshark: no"
    };
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
fn tshark_filter_is_reported_repr_style() {
    // `--tshark-filter` makes tshark the canonical flow source: `used_tshark`
    // is true whenever a tshark binary exists, independent of EK support, and
    // the filter is echoed in the header rendered `repr()`-style
    // (single-quoted).
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
        "header must echo the filter in repr form: {text}"
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
    assert!(text.contains("decision: lb pool_select pool_api"), "{text}");
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
