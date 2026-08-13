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

//! Direct unit tests for the renderer plugins on synthetic `Value`s.
//!
//! Exercise the chain / dict / tree paths and the option parsers without
//! going through the CLI; complements the byte-for-byte golden comparison in
//! `f5-cli/tests/query_render_parity.rs`.

use std::collections::BTreeMap;

use tcl_bigip_query::errors::QueryError;
use tcl_bigip_query::renderers::{list_renderers, lookup, render};
use tcl_bigip_query::value::Value;

fn opts(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn s(text: &str) -> Value {
    Value::Str(text.to_owned())
}

#[test]
fn registry_lists_three_builtins_sorted() {
    let names: Vec<&str> = list_renderers().iter().map(|spec| spec.name).collect();
    assert_eq!(names, vec!["ascii-blocks", "gantt", "mermaid"]);
    assert!(lookup("mermaid").is_some());
    assert!(lookup("nope").is_none());
}

#[test]
fn unknown_renderer_errors_with_registered_list() {
    let err = render("nope", &[], &BTreeMap::new()).unwrap_err();
    assert_eq!(
        err,
        QueryError::Renderer(
            "unknown renderer 'nope' (registered: ascii-blocks, gantt, mermaid)".to_owned()
        )
    );
}

#[test]
fn mermaid_chain_of_scalars() {
    let values = vec![s("a"), s("b"), s("c")];
    let out = render("mermaid", &values, &BTreeMap::new()).unwrap();
    assert_eq!(
        out,
        "graph LR\n  n0[\"a\"]\n  n1[\"b\"]\n  n2[\"c\"]\n  n0 --> n1\n  n1 --> n2\n"
    );
}

#[test]
fn mermaid_direction_lowercased_and_validated() {
    let out = render("mermaid", &[s("x")], &opts(&[("direction", "tb")])).unwrap();
    assert!(out.starts_with("graph TB\n"));

    let err = render("mermaid", &[s("x")], &opts(&[("direction", "ZZ")])).unwrap_err();
    assert_eq!(
        err,
        QueryError::Renderer(
            "mermaid: unknown direction 'ZZ' (expected one of LR, RL, TB, BT)".to_owned()
        )
    );
}

#[test]
fn mermaid_empty_is_bare_graph_line() {
    let out = render("mermaid", &[], &BTreeMap::new()).unwrap();
    assert_eq!(out, "graph LR\n");
}

#[test]
fn gantt_tsv_string() {
    let tsv = "Jan 01 10:00:00\t/Common/web\tDOWN\nJan 01 10:30:00\t/Common/web\tUP";
    let out = render("gantt", &[s(tsv)], &BTreeMap::new()).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "members down/up over time (1 char = 5 min)");
    // Ruler hour label `10` at column 22 (after the 22-char name gutter).
    assert!(lines[1].starts_with(&format!("{}10", " ".repeat(22))));
    assert_eq!(lines[2], format!("{}+------------", " ".repeat(22)));
    // The member row: 20-char left-justified name, two spaces, `|`, then the
    // DOWN span (`v#####`) and the recovery `^`.
    assert_eq!(
        lines[3],
        format!("{}  |v#####^", "web".to_owned() + &" ".repeat(17))
    );
}

#[test]
fn gantt_no_events() {
    let out = render("gantt", &[Value::List(vec![])], &BTreeMap::new()).unwrap();
    assert_eq!(out, "(no events)\n");
}

#[test]
fn gantt_rejects_bad_unit_minutes() {
    let err = render("gantt", &[s("")], &opts(&[("unit-minutes", "7")])).unwrap_err();
    assert_eq!(
        err,
        QueryError::Renderer(
            "gantt: unit-minutes must be a positive divisor of 60 \
             (one of 1, 2, 3, 4, 5, 6, 10, 12, 15, 20, 30, 60); got 7"
                .to_owned()
        )
    );
}

#[test]
fn ascii_blocks_flat_box() {
    let block = Value::object([("title".to_owned(), s("hello"))]);
    let out = render("ascii-blocks", &[block], &BTreeMap::new()).unwrap();
    assert_eq!(
        out,
        "╭──────────────╮\n│ hello        │\n╰──────────────╯\n"
    );
}

#[test]
fn ascii_blocks_empty() {
    let out = render("ascii-blocks", &[], &BTreeMap::new()).unwrap();
    assert_eq!(out, "(no blocks)\n");
}

#[test]
fn ascii_blocks_missing_title_errors() {
    let block = Value::object([("foo".to_owned(), s("x"))]);
    let err = render("ascii-blocks", &[block], &BTreeMap::new()).unwrap_err();
    assert_eq!(
        err,
        QueryError::Renderer(
            "ascii-blocks: dict block must carry 'title' (or 'label' / 'name'); got ['foo']"
                .to_owned()
        )
    );
}

#[test]
fn ascii_blocks_nested_tree() {
    let child = Value::object([("title".to_owned(), s("child"))]);
    let parent = Value::object([
        ("title".to_owned(), s("parent")),
        ("rows".to_owned(), Value::List(vec![child])),
    ]);
    let out = render("ascii-blocks", &[parent], &opts(&[("min-width", "8")])).unwrap();
    assert_eq!(
        out,
        "╭──────────╮\n│ parent   │\n╰──────────╯\n│\n╰─ ╭──────────╮\n   │ child    │\n   ╰──────────╯\n"
    );
}
