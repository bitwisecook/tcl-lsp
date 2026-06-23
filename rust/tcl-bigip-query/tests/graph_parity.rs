//! End-to-end golden differential test for the query *graph* layer.
//!
//! The pipeline captured in `tests/fixtures/graph.json`
//! (`scripts/codegen/gen_f5_query_graph_fixtures.py`): build a BIG-IP `Root`
//! from the real fixture config, parse → evaluate against the projected
//! `Container` tree (which walks the reference graph for the graph-backed
//! builtins `refs` / `referenced_by` / `references_to` /
//! `check_partition_visibility` and the synthesised rule `.refs` sub-object)
//! → `output::render`. For each `(query, mode)` the Rust output (or error
//! message) must match the golden byte-for-byte. Self-contained — no external oracle at
//! test time; the `bigip.conf` fixture is embedded via `include_str!`.

use serde_json::Value as J;
use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip_query::eval::{EvalContext, Root, evaluate};
use tcl_bigip_query::output::render;
use tcl_bigip_query::parser::parse_query;

/// The same fixture the generator reads.
const FIXTURE: &str = include_str!("../../tcl-bigip/tests/fixtures/bigip.conf");

fn run(query: &str, mode: &str) -> Result<String, String> {
    let config = parse_bigip_conf(FIXTURE, "Common");
    let root = Root::bigip("bigip.conf", FIXTURE.to_owned(), config);
    let mut ctx = EvalContext::new(root);
    let prog = parse_query(query).map_err(|e| e.to_string())?;
    let values = evaluate(&prog, &mut ctx).map_err(|e| e.to_string())?;
    render(&values, mode).map_err(|e| e.to_string())
}

#[test]
fn graph_matches_python() {
    let raw = include_str!("fixtures/graph.json");
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
        "{} / {} graph cases mismatched:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
