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

//! End-to-end golden differential test for the math-category builtins.
//!
//! The pipeline captured in `tests/fixtures/math.json`
//! from the captured query DSL fixtures: parse → evaluate
//! against a JSON-backed root → `output::render`. For each `(query, input,
//! mode)` the Rust output (or `error:` message) must match the expected value exactly.
//! Self-contained — no external reference at test time.

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

/// The most units-in-the-last-place two float results may differ by and still
/// count as equal.
///
/// IEEE-754 requires correct rounding (≤ 0.5 ULP) only for `+ − × ÷ √ fma`, not
/// for transcendentals like `asin`/`atanh`, so each platform's `libm` may land
/// ~1 ULP off the true value in its own direction and two platforms legitimately
/// differ by a ULP or two. Four ULP (the convention Google Test's float compare
/// uses) tolerates that platform variance while staying orders of magnitude
/// tighter than any genuine algorithmic error.
const MAX_ULPS: u64 = 4;

/// Whether two rendered outputs are equal, treating floating-point values as
/// equal within [`MAX_ULPS`].
fn outputs_match(expected: &str, got: &str) -> bool {
    if expected == got {
        return true;
    }
    // Fall back to a structural numeric compare only when both sides are JSON
    // (the `json` render mode); other modes must match verbatim.
    match (
        serde_json::from_str::<J>(expected),
        serde_json::from_str::<J>(got),
    ) {
        (Ok(e), Ok(g)) => json_ulps_eq(&e, &g),
        _ => false,
    }
}

/// Map an `f64` to a `u64` whose ordinary integer ordering matches float
/// ordering, so the unsigned difference between two such keys is the exact
/// number of representable doubles between the values (their ULP distance).
fn ulp_key(f: f64) -> u64 {
    let bits = f.to_bits();
    // Flip all bits of negatives (so they order below positives), and set the
    // sign bit of non-negatives — the standard monotonic transform.
    if bits >> 63 == 1 {
        !bits
    } else {
        bits | (1 << 63)
    }
}

/// Whether two floats are within [`MAX_ULPS`]. NaN and ∞ must match bit-for-bit;
/// the ULP compare itself covers exact equality (0 ULP) and `+0.0`/`-0.0`.
fn floats_close(a: f64, b: f64) -> bool {
    if !a.is_finite() || !b.is_finite() {
        return a.to_bits() == b.to_bits();
    }
    ulp_key(a).abs_diff(ulp_key(b)) <= MAX_ULPS
}

/// Recursive JSON equality with a ULP tolerance on numbers.
fn json_ulps_eq(a: &J, b: &J) -> bool {
    match (a, b) {
        (J::Number(x), J::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(fx), Some(fy)) => floats_close(fx, fy),
            _ => x == y,
        },
        (J::Array(xs), J::Array(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(p, q)| json_ulps_eq(p, q))
        }
        (J::Object(xs), J::Object(ys)) => {
            xs.len() == ys.len()
                && xs
                    .iter()
                    .all(|(k, v)| ys.get(k).is_some_and(|w| json_ulps_eq(v, w)))
        }
        _ => a == b,
    }
}

#[test]
fn math_builtins() {
    let raw = include_str!("../fixtures/math.json");
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
            ("ok", Ok(out)) => outputs_match(expected, out),
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
        "{} / {} math cases mismatched:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
