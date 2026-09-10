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

//! End-to-end golden differential test for the query *graph* layer.
//!
//! The pipeline captured in `tests/fixtures/graph.json`
//! from the captured query DSL fixtures: build a BIG-IP `Root`
//! from the real fixture config, parse → evaluate against the projected
//! `Container` tree (which walks the reference graph for the graph-backed
//! builtins `refs` / `referenced_by` / `references_to` /
//! `check_partition_visibility` and the synthesised rule `.refs` sub-object)
//! → `output::render`. For each `(query, mode)` the Rust output (or error
//! message) must match the golden byte-for-byte. Self-contained — no external reference at
//! test time; the `bigip.conf` fixture is embedded via `include_str!`.

use std::rc::Rc;

use serde_json::Value as J;
use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip_query::eval::{EvalContext, MergedView, Root, evaluate};
use tcl_bigip_query::output::render;
use tcl_bigip_query::parser::parse_query;
use tcl_bigip_query::{QueryOptions, run_query};

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
fn graph() {
    let raw = include_str!("../fixtures/graph.json");
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

/// `references_to` consults the merged graph under `--merge`, so a cross-file
/// referrer resolves rather than being missed by the single originating root.
#[test]
fn references_to_uses_merged_graph_in_merge_mode() {
    let conf_a = "ltm virtual /Common/vsA {\n    destination /Common/1.2.3.4:80\n    pool /Common/poolB\n}\n";
    let conf_b = "ltm pool /Common/poolB {\n    members none\n}\n";
    let cfg_a = parse_bigip_conf(conf_a, "Common");
    let cfg_b = parse_bigip_conf(conf_b, "Common");
    let root_a = Root::bigip("a.conf", conf_a.to_owned(), cfg_a);
    let root_b = Root::bigip("b.conf", conf_b.to_owned(), cfg_b);

    // ctx.root is file B (the pool). A graph over file B alone has no referrer
    // for poolB; only one spanning file A does.
    MergedView::install(&[Rc::clone(&root_a), Rc::clone(&root_b)]);
    let mut ctx = EvalContext::new(Rc::clone(&root_b));

    let prog = parse_query("references_to(\"/Common/poolB\")").expect("query parses");
    let values = evaluate(&prog, &mut ctx).expect("evaluates");
    let out = render(&values, "json").expect("renders");
    assert!(
        out.contains("/Common/vsA"),
        "merged cross-file referrer must appear: {out}"
    );
}

// Reference walks under `--merge`. Every walk off any root in the merged
// namespace spans all of them, so a referrer, a rename safety check, and an
// iRule's `.refs` all see objects defined in a sibling source.

/// A virtual in one source pointing at a pool in another, in a partition that
/// cannot see the pool's — a visibility violation that is only apparent when
/// both sources are read as one namespace.
const PV_REFERRER_CONF: &str = "\
ltm virtual /Part1/v1 {
    destination /Part1/10.0.0.1:80
    pool /Part2/other_pool
}
";

const PV_TARGET_CONF: &str = "\
ltm pool /Part2/other_pool {
    members {
        /Part2/n1:80 {
            address 10.0.1.1
        }
    }
}
";

/// An iRule and the pool it names, split across two sources.
const RULE_CONF: &str = "\
ltm rule /Common/r1 {
    when HTTP_REQUEST {
        pool /Common/api_pool
    }
}
";

const RULE_POOL_CONF: &str = "\
ltm pool /Common/api_pool {
    members {
        /Common/n1:80 {
            address 10.0.1.1
        }
    }
}
";

/// A pool in one source referenced by a virtual in another.
const SHARED_POOL_REFERRER_CONF: &str = "\
ltm virtual /Part1/v1 {
    destination /Part1/10.0.0.1:80
    pool /Common/shared_pool
}
";

const SHARED_POOL_CONF: &str = "\
ltm pool /Common/shared_pool {
    members {
        /Common/n1:80 {
            address 10.0.1.1
        }
    }
}
";

fn run_merged_named(
    query: &str,
    sources: &[(&str, &str)],
    names: &[(&str, &str)],
) -> Result<String, String> {
    let owned: Vec<(String, String)> = sources
        .iter()
        .map(|(u, s)| ((*u).to_owned(), (*s).to_owned()))
        .collect();
    let opts = QueryOptions {
        merge: true,
        names: names
            .iter()
            .map(|(n, u)| ((*n).to_owned(), (*u).to_owned()))
            .collect(),
        ..QueryOptions::default()
    };
    let result = run_query(query, &owned, &opts).map_err(|e| e.to_string())?;
    let values: Vec<tcl_bigip_query::Value> = result
        .values_per_file
        .iter()
        .flat_map(|(_, vals)| vals.iter().cloned())
        .collect();
    render(&values, "json").map_err(|e| e.to_string())
}

fn run_merged(query: &str, sources: &[(&str, &str)]) -> Result<String, String> {
    let owned: Vec<(String, String)> = sources
        .iter()
        .map(|(u, s)| ((*u).to_owned(), (*s).to_owned()))
        .collect();
    let opts = QueryOptions {
        merge: true,
        ..QueryOptions::default()
    };
    let result = run_query(query, &owned, &opts).map_err(|e| e.to_string())?;
    let values: Vec<tcl_bigip_query::Value> = result
        .values_per_file
        .iter()
        .flat_map(|(_, vals)| vals.iter().cloned())
        .collect();
    render(&values, "json").map_err(|e| e.to_string())
}

/// `check_partition_visibility()` audits the merged namespace, so a referrer
/// and its target in different sources are compared.
#[test]
fn partition_visibility_audit_spans_merged_sources() {
    let out = run_merged(
        "check_partition_visibility()",
        &[
            ("file:///pv-a.conf", PV_REFERRER_CONF),
            ("file:///pv-b.conf", PV_TARGET_CONF),
        ],
    )
    .expect("audit runs");
    assert!(
        out.contains("/Part1/v1 -> /Part2/other_pool"),
        "cross-source violation reported: {out}"
    );
}

/// `rename()` refuses a cross-partition move that would strand a referrer in a
/// sibling source, the same as when both objects share one source.
#[test]
fn rename_partition_check_sees_referrers_in_sibling_sources() {
    let err = run_merged(
        r#"rename("/Common/shared_pool", "/Part2/shared_pool")"#,
        &[
            ("file:///a.conf", SHARED_POOL_REFERRER_CONF),
            ("file:///b.conf", SHARED_POOL_CONF),
        ],
    )
    .expect_err("the move strands /Part1/v1 and must be refused");
    assert!(
        err.contains("partition visibility") && err.contains("/Part1/v1"),
        "the refusal names the stranded referrer: {err}"
    );
}

/// A `$name` binding is the same root as the source it names, so a projection
/// reached through it sees the merged namespace on every iteration.
///
/// Merge mode evaluates the statement once per root, so a `$name`-anchored
/// query yields its values once per root. A named binding that did not carry
/// the merged view would resolve on the iteration naming its own source and
/// come back empty on the others.
#[test]
fn a_named_binding_sees_the_merged_namespace_on_every_iteration() {
    let sources = [
        ("file:///rule.conf", RULE_CONF),
        ("file:///pool.conf", RULE_POOL_CONF),
    ];
    let out = run_merged_named(
        "$rules.ltm.rule[] | .refs.pools[]",
        &sources,
        &[("rules", "file:///rule.conf")],
    )
    .expect("projection runs");
    let hits = out.matches("/Common/api_pool").count();
    assert_eq!(hits, sources.len(), "one hit per root, got {out}");
}

/// An `ltm rule`'s synthesised `.refs` resolves objects defined in a sibling
/// source.
#[test]
fn rule_refs_span_merged_sources() {
    let out = run_merged(
        ".ltm.rule[] | .refs.pools",
        &[
            ("file:///rule.conf", RULE_CONF),
            ("file:///pool.conf", RULE_POOL_CONF),
        ],
    )
    .expect("projection runs");
    assert!(
        out.contains("/Common/api_pool"),
        "the pool named by the rule resolves across sources: {out}"
    );
}
