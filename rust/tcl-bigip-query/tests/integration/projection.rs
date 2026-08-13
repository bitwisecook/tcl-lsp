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

//! End-to-end golden differential test for the query *projection* layer.
//!
//! The pipeline captured in `tests/fixtures/projection.json`
//! from the captured query DSL fixtures: build a BIG-IP
//! `Root` from the real fixture config, parse → evaluate against the
//! projected `Container` tree → `output::render`. For each `(query, mode)`
//! the Rust output (or error message) must match the golden byte-for-byte.
//! Self-contained — no external reference at test time; the `bigip.conf` fixture is
//! embedded via `include_str!`.

use serde_json::Value as J;
use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip_query::eval::{EvalContext, Root, evaluate};
use tcl_bigip_query::output::render;
use tcl_bigip_query::parser::parse_query;

/// The same fixture the generator reads.
const FIXTURE: &str = include_str!("../../../tcl-bigip/tests/fixtures/bigip.conf");

fn run(query: &str, mode: &str) -> Result<String, String> {
    let config = parse_bigip_conf(FIXTURE, "Common");
    let root = Root::bigip("bigip.conf", FIXTURE.to_owned(), config);
    let mut ctx = EvalContext::new(root);
    let prog = parse_query(query).map_err(|e| e.to_string())?;
    let values = evaluate(&prog, &mut ctx).map_err(|e| e.to_string())?;
    render(&values, mode).map_err(|e| e.to_string())
}

#[test]
fn projection() {
    let raw = include_str!("../fixtures/projection.json");
    let cases: J = serde_json::from_str(raw).expect("fixture is valid JSON");
    let cases = cases.as_array().expect("fixture is an array");
    assert!(!cases.is_empty());

    let mut failures = Vec::new();
    for case in cases {
        let query = case["query"].as_str().unwrap();
        let mode = case["mode"].as_str().unwrap();
        let kind = case["kind"].as_str().unwrap();
        let expected = case["output"].as_str().unwrap();
        let got = run(query, mode);
        let ok = match (kind, &got) {
            ("ok", Ok(out)) => out == expected,
            ("err", Err(msg)) => msg == expected,
            _ => false,
        };
        if !ok {
            failures.push(format!(
                "query={query:?} mode={mode} kind={kind}\n  expected: {expected:?}\n  got:      {got:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} projection cases mismatched:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
