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

//! End-to-end golden differential test for the query evaluator + builtins.
//!
//! The pipeline (parse → evaluate against a JSON-backed root →
//! `output::render`) checked against the query DSL fixtures captured in
//! `tests/fixtures/eval.json`. For each `(query, input, mode)` the Rust
//! output (or `error:` message) must match the expected value exactly.
//! Self-contained — no external reference at test time.

use indexmap::IndexMap;
use serde_json::Value as J;
use tcl_bigip_query::eval::{EvalContext, Root, evaluate};
use tcl_bigip_query::output::render;
use tcl_bigip_query::parser::parse_query;
use tcl_bigip_query::value::Value;
use tcl_bigip_query::{QueryOptions, run_query};

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
fn evaluator() {
    let raw = include_str!("../fixtures/eval.json");
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

// Binding, merge-mode dereference, and unresolved-reference behaviour. These
// drive the full `run_query` runner rather than the JSON-root golden above,
// because the behaviour they pin lives in the config-backed projection and
// merge paths.

/// One LTM config: `v1` has a pool, `v2` has none, so `select(.pool != "")`
/// rejects exactly one virtual.
const LTM_CONF: &str = "\
ltm pool /Common/p1 {
    members {
        /Common/n1:80 {
            address 10.0.1.1
        }
    }
}

ltm virtual /Common/v1 {
    destination /Common/10.0.0.1:80
    pool /Common/p1
}

ltm virtual /Common/v2 {
    destination /Common/10.0.0.2:80
}
";

fn run_conf(query: &str, sources: &[(&str, &str)], merge: bool) -> Result<Vec<Value>, String> {
    let owned: Vec<(String, String)> = sources
        .iter()
        .map(|(u, s)| ((*u).to_owned(), (*s).to_owned()))
        .collect();
    let opts = QueryOptions {
        merge,
        ..QueryOptions::default()
    };
    let result = run_query(query, &owned, &opts).map_err(|e| e.to_string())?;
    Ok(result
        .values_per_file
        .iter()
        .flat_map(|(_, vals)| vals.iter().cloned())
        .collect())
}

fn strings(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .map(|v| match v {
            Value::Str(s) => s.clone(),
            Value::PathRef(p) => p.full_path.clone(),
            other => tcl_bigip_query::jsonfmt::to_pretty(other),
        })
        .collect()
}

/// `select(...) as $v | body` skips a rejected item exactly as `|` does.
///
/// `select` rejects by returning the internal `Drop` sentinel, which must
/// never reach the bound name: the item is skipped and the body does not run
/// for it.
#[test]
fn select_drop_short_circuits_an_as_binding() {
    let sources = [("file:///ltm.conf", LTM_CONF)];
    let bound = run_conf(
        r#".ltm.virtual[] | select(.pool != "") as $vs | $vs.name"#,
        &sources,
        false,
    )
    .expect("the binding skips the dropped virtual instead of binding Drop");
    assert_eq!(strings(&bound), vec!["v1"]);

    // The binding form and the plain-pipe form it mirrors must agree.
    let piped = run_conf(
        r#".ltm.virtual[] | select(.pool != "") | .name"#,
        &sources,
        false,
    )
    .expect("run");
    assert_eq!(strings(&bound), strings(&piped));

    // A binding whose whole source is dropped yields nothing, not an error.
    let none = run_conf(
        r#".ltm.virtual[] | select(.pool == "nope") as $vs | $vs.name"#,
        &sources,
        false,
    )
    .expect("an all-dropped source is an empty stream");
    assert!(none.is_empty(), "expected no values, got {none:?}");
}

// A GTM wideip whose pool lives in a *different* document — the reference
// crosses a document boundary, so it only resolves when the merged view is
// consulted.
const GTM_WIDEIP_CONF: &str = "\
gtm wideip a /Common/www.example.com {
    pools {
        /Common/gp1 { }
    }
}
";

const GTM_POOL_CONF: &str = "\
gtm pool a /Common/gp1 {
    members {
        /Common/srv1:/Common/vs1 {
            order 0
        }
    }
}
";

/// Under `--merge` a dereference resolves against the merged view at every
/// hop, not just the root that happens to be iterating.
///
/// Merge mode evaluates each statement once per root. The wideip is only in
/// the first document, so it is produced during that root's turn — at which
/// point its `pools` reference points into the *second* document.
#[test]
fn merge_resolves_a_deref_into_another_document() {
    let sources = [
        ("file:///gtm-wideip.conf", GTM_WIDEIP_CONF),
        ("file:///gtm-pool.conf", GTM_POOL_CONF),
    ];

    let merged = run_conf(
        ".gtm.wideip[] | .pools[] | .members[] | .name",
        &sources,
        true,
    )
    .expect("the cross-document pool resolves under --merge");
    assert_eq!(strings(&merged), vec!["/Common/srv1:/Common/vs1"]);

    // Same chain behind an `as` binding: the binding keeps the merged view.
    let bound = run_conf(
        ".gtm.wideip[] as $w | $w.pools[] | .members[] | .name",
        &sources,
        true,
    )
    .expect("the binding does not lose the merged view");
    assert_eq!(strings(&bound), strings(&merged));

    // And a further hop off the resolved object still resolves.
    let deeper = run_conf(
        ".gtm.wideip[] | .pools[] | .members[] | .order",
        &sources,
        true,
    )
    .expect("a third hop resolves too");
    assert_eq!(strings(&deeper), vec!["0"]);
}

/// A reference that resolves nowhere in the view is visible, never a silent
/// empty stream. It reads as `null`, carrying the path that failed so a
/// further field read or subscript names it, and iterating it is empty
/// because there is nothing to iterate. Plain `null` keeps its own
/// semantics — the dangling case does not loosen them.
#[test]
fn an_unresolvable_reference_is_never_silent() {
    let sources = [
        ("file:///gtm-wideip.conf", GTM_WIDEIP_CONF),
        ("file:///gtm-pool.conf", GTM_POOL_CONF),
    ];

    // Without --merge the pool is in the other document and cannot resolve.
    let unmerged = run_conf(".gtm.wideip[] | .pools[] | .members", &sources, false)
        .expect("an unresolved deref reads as null, not an error");
    assert_eq!(unmerged.len(), 1, "one value, got {unmerged:?}");
    assert!(
        matches!(unmerged[0], Value::Unresolved(_)),
        "carries the dangling path, got {:?}",
        unmerged[0]
    );
    // It renders as null and has no length, so a projection shows the gap.
    assert_eq!(strings(&unmerged), vec!["null"]);

    // Reading on through it names what failed to resolve.
    let err = run_conf(".gtm.wideip[] | .pools[] | .members.foo", &sources, false)
        .expect_err("a field read through an unresolved reference is an error");
    assert!(
        err.contains("/Common/gp1") && err.contains("unresolved reference"),
        "the error names the path: {err}"
    );
    assert!(
        err.contains("--merge"),
        "with several sources loaded the error points at --merge: {err}"
    );

    // Iterating it is empty — there is nothing to iterate.
    let iterated = run_conf(
        ".gtm.wideip[] | .pools[] | [.members[]] | length",
        &sources,
        false,
    )
    .expect("iterating an unresolved reference is empty, not an error");
    assert_eq!(strings(&iterated), vec!["0"]);

    // It compares equal to null, so a query can select the gaps.
    let selected = run_conf(
        ".gtm.wideip[] | .pools[] | select(.members == null)",
        &sources,
        false,
    )
    .expect("an unresolved reference compares equal to null");
    assert_eq!(strings(&selected), vec!["/Common/gp1"]);

    // An empty reference — no object named at all — contributes nothing at
    // every kind of step, including an indexed one.
    let empty = run_conf(
        r#".ltm.virtual[] | select(.pool == "") | .pool["members"]"#,
        &[("file:///ltm.conf", LTM_CONF)],
        false,
    )
    .expect("an empty reference is not a dangling one");
    assert!(empty.is_empty(), "expected no values, got {empty:?}");
}

/// A plain `null` keeps jq-ish semantics: it refuses to iterate. Only an
/// unresolved reference iterates empty.
#[test]
fn plain_null_still_refuses_to_iterate() {
    let err =
        run("null | .[]", &serde_json::json!({}), "json").expect_err("iterating null is an error");
    assert!(
        err.contains("cannot iterate null"),
        "null keeps its own semantics: {err}"
    );
}
