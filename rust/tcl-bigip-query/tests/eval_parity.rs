//! End-to-end golden differential test for the query evaluator + builtins.
//!
//! The pipeline captured in `tests/fixtures/eval.json`
//! (`scripts/codegen/gen_f5_query_eval_fixtures.py`): parse → evaluate
//! against a JSON-backed root → `output::render`. For each `(query, input,
//! mode)` the Rust output (or `error:` message) must match the expected value exactly.
//! Self-contained — no external oracle at test time.

use indexmap::IndexMap;
use serde_json::Value as J;
use tcl_bigip_query::eval::{EvalContext, Root, evaluate};
use tcl_bigip_query::output::render;
use tcl_bigip_query::parser::parse_query;
use tcl_bigip_query::value::Value;

fn json_to_value(j: &J) -> Value {
    match j {
        J::Null => Value::Null,
        J::Bool(b) => Value::Bool(*b),
        J::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(u) = n.as_u64() {
                Value::Int(i64::try_from(u).unwrap_or(i64::MAX))
            } else {
                Value::Float(n.as_f64().unwrap())
            }
        }
        J::String(s) => Value::Str(s.clone()),
        J::Array(items) => Value::List(items.iter().map(json_to_value).collect()),
        J::Object(map) => {
            let mut m = IndexMap::new();
            for (k, v) in map {
                m.insert(k.clone(), json_to_value(v));
            }
            Value::Object(m)
        }
    }
}

fn run(query: &str, input: &J, mode: &str) -> Result<String, String> {
    let data = json_to_value(input);
    let root = Root::json("data.json", data);
    let mut ctx = EvalContext::new(root);
    let prog = parse_query(query).map_err(|e| e.to_string())?;
    let values = evaluate(&prog, &mut ctx).map_err(|e| e.to_string())?;
    render(&values, mode).map_err(|e| e.to_string())
}

#[test]
fn evaluator_matches_python() {
    let raw = include_str!("fixtures/eval.json");
    let cases: J = serde_json::from_str(raw).expect("fixture is valid JSON");
    let cases = cases.as_array().expect("fixture is an array");
    assert!(!cases.is_empty());

    let mut failures = Vec::new();
    for case in cases {
        let query = case["query"].as_str().unwrap();
        let mode = case["mode"].as_str().unwrap();
        let kind = case["kind"].as_str().unwrap();
        let expected = case["output"].as_str().unwrap();
        let got = run(query, &case["input"], mode);
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
        "{} / {} eval cases mismatched:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
