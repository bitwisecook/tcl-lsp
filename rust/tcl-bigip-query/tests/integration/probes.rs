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

//! Golden differential test for the deterministic probe surface.
//!
//! Two byte-for-byte guarantees are asserted against `tests/fixtures/probes.json`:
//!
//! 1. `x509_parse(<pem>)` — the rich x509 dict (subject /
//!    issuer / serial / validity / SANs / fingerprint / key-alg / key-size /
//!    sig-alg / version / public-key PEM) must match the golden exactly.
//! 2. The `--enable-probes` gating error for every gated network builtin
//!    (`ping`, `portping`, `traceroute`, `socket_get`, `tls_handshake`, the
//!    `url_*` family) must reproduce the expected wording verbatim.
//!
//! The live network probes themselves (faithful-but-not-golden) are exercised
//! through the gating path only — they never touch the network here.

use serde_json::Value as J;
use tcl_bigip_query::eval::{EvalContext, Root, evaluate};
use tcl_bigip_query::parser::parse_query;
use tcl_bigip_query::value::Value;

const PEM: &str = include_str!("../fixtures/x509_example.pem");

/// Run a query against an empty root, with probes disabled (the default).
fn run_disabled(query: &str) -> Result<Vec<Value>, String> {
    let root = Root::json("data.json", Value::Null);
    let mut ctx = EvalContext::new(root);
    let prog = parse_query(query).map_err(|e| e.to_string())?;
    evaluate(&prog, &mut ctx).map_err(|e| e.to_string())
}

/// Convert a [`Value`] to a `serde_json::Value` for structural comparison with
/// the captured golden (objects preserve insertion order).
fn value_to_json(v: &Value) -> J {
    match v {
        Value::Null => J::Null,
        Value::Bool(b) => J::Bool(*b),
        Value::Int(i) => J::from(*i),
        Value::Float(f) => serde_json::Number::from_f64(*f).map_or(J::Null, J::Number),
        Value::Str(s) => J::String(s.clone()),
        Value::PathRef(p) => J::String(p.full_path.clone()),
        Value::List(items) | Value::Stream(items) => {
            J::Array(items.iter().map(value_to_json).collect())
        }
        Value::Object(map) => {
            let mut m = serde_json::Map::new();
            for (k, val) in map {
                m.insert(k.clone(), value_to_json(val));
            }
            J::Object(m)
        }
        other => J::String(format!("{other:?}")),
    }
}

fn fixture() -> Vec<J> {
    let raw = include_str!("../fixtures/probes.json");
    serde_json::from_str::<J>(raw)
        .expect("fixture is valid JSON")
        .as_array()
        .expect("fixture is an array")
        .clone()
}

#[test]
fn x509_parse_byte_for_byte() {
    let cases = fixture();
    let case = cases
        .iter()
        .find(|c| c["kind"] == "x509_parse")
        .expect("x509_parse golden case present");
    let golden = &case["output"];

    // Bind the PEM into `$pem` and parse it through the engine, exactly as a
    // `x509_parse($pem)` query would resolve.
    let escaped = PEM
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    let query = format!("x509_parse(\"{escaped}\")");
    let values = run_disabled(&query).expect("x509_parse succeeds");
    assert_eq!(values.len(), 1, "x509_parse yields a single dict");
    let got = value_to_json(&values[0]);
    assert_eq!(&got, golden, "x509_parse output diverged from the golden");
}

#[test]
fn x509_parse_bad_input_error_prefix() {
    let cases = fixture();
    let case = cases
        .iter()
        .find(|c| c["kind"] == "x509_parse_err")
        .expect("x509_parse_err case present");
    let prefix = case["output_prefix"].as_str().unwrap();
    let err = run_disabled("x509_parse(\"not a cert\")").expect_err("bad PEM raises");
    assert!(
        err.starts_with(prefix),
        "expected error to start with {prefix:?}, got {err:?}"
    );
}

#[test]
fn gated_probes_report_disabled_verbatim() {
    let cases = fixture();
    // One representative call per gated builtin, with probes disabled.
    let invocations = [
        ("ping", "ping(\"127.0.0.1\")"),
        ("portping", "portping(\"127.0.0.1\", 443)"),
        ("traceroute", "traceroute(\"127.0.0.1\")"),
        ("socket_get", "socket_get(\"127.0.0.1\", 22)"),
        ("tls_handshake", "tls_handshake(\"example.test\", 443)"),
        ("url_get", "url_get(\"https://example.test/\")"),
        ("url_head", "url_head(\"https://example.test/\")"),
        ("url_options", "url_options(\"https://example.test/\")"),
        ("url_post", "url_post(\"https://example.test/\", \"body\")"),
    ];
    let mut failures = Vec::new();
    for (builtin, query) in invocations {
        let expected = cases
            .iter()
            .find(|c| c["kind"] == "gating" && c["builtin"] == builtin)
            .and_then(|c| c["output"].as_str())
            .unwrap_or_else(|| panic!("gating golden for {builtin}"));
        match run_disabled(query) {
            Err(msg) if msg == expected => {}
            Err(msg) => failures.push(format!("{builtin}: got {msg:?}, want {expected:?}")),
            Ok(_) => failures.push(format!("{builtin}: expected gating error, query succeeded")),
        }
    }
    assert!(
        failures.is_empty(),
        "gating mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn dns_is_ungated() {
    // `dns` / `rev_dns` are NOT gated — a bare lookup must not raise
    // the probes-disabled error (it may return an empty list with no network).
    let out = run_disabled("dns(\"localhost\")");
    assert!(
        out.is_ok(),
        "dns must run without --enable-probes, got {out:?}"
    );
}
