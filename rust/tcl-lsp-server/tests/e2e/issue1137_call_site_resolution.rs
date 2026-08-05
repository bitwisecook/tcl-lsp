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

//! End-to-end coverage for the call-site resolution seam shared by
//! `textDocument/definition` and `textDocument/hover` — issues #1137
//! (command-position gate + the `::oo::Helpers` cross-file path), #1133
//! (an already-resolved indirect head must beat a same-named decoy), and
//! the document-tier providers of #1140 that answer the same questions.
//!
//! The unifying property under test is **self-consistency**: every provider
//! in one response must agree about what a given span names, and none may
//! answer from a text guess where the analyser has already decided.

use serde_json::Value;

use crate::common::{Lsp, unique_uri};

/// The corpus shape from issue #1137 idx 50, reduced to its mechanism: an
/// argument word that happens to share a proc's name, beside a real call
/// head. `dump` is data; tclsh never looks it up as a command.
const ARGUMENT_DECOY: &str = concat!(
    "proc anotherproc {a b} { return $a }\n",
    "proc dump {x} { return $x }\n",
    "proc caller {} {\n",
    "    return [anotherproc dump 5]\n",
    "}\n",
);

/// Issue #1133: a constant-dominated indirect head beside a same-named decoy.
const INDIRECT_HEAD: &str = concat!(
    "namespace eval ::tc { proc setdef {a} { return $a } }\n",
    "namespace eval ::other { proc setdef {b c} { return $b } }\n",
    "set ns ::tc\n",
    "${ns}::setdef\n",
);

fn locations(result: &Value) -> Vec<(u32, u32)> {
    let Some(items) = result.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|loc| {
            let range = loc.get("range").or_else(|| loc.get("targetSelectionRange"))?;
            Some((
                u32::try_from(range["start"]["line"].as_u64()?).ok()?,
                u32::try_from(range["start"]["character"].as_u64()?).ok()?,
            ))
        })
        .collect()
}

fn hover_text(result: &Value) -> String {
    result
        .get("contents")
        .and_then(|c| c.get("value"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn definition_abstains_on_an_argument_that_shares_a_procs_name() {
    // FP guard — issue #1137 idx 50. Before the command-position gate the
    // final text-scan fallback resolved `dump` onto `proc ::dump`, an answer
    // no run of the program could produce.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, ARGUMENT_DECOY);
    let result = lsp.definition(&uri, 3, 24);
    assert!(
        locations(&result).is_empty(),
        "an argument word must not resolve as a command: {result}",
    );
}

#[test]
fn hover_abstains_on_the_same_argument() {
    // The two providers share the seam, so they must share the abstention —
    // the defect was visible as hover and definition agreeing on a wrong
    // answer.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, ARGUMENT_DECOY);
    let text = hover_text(&lsp.hover(&uri, 3, 24));
    assert!(
        !text.contains("proc ::dump"),
        "hover must not describe an argument as a command: {text}",
    );
}

#[test]
fn definition_and_hover_still_answer_the_real_head_beside_it() {
    // TP — the gate must cost the genuine call head nothing.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, ARGUMENT_DECOY);
    assert_eq!(
        locations(&lsp.definition(&uri, 3, 12)),
        vec![(0, 5)],
        "the call head must still resolve",
    );
    let text = hover_text(&lsp.hover(&uri, 3, 12));
    assert!(text.contains("anotherproc"), "hover on the head: {text}");
}

#[test]
fn definition_prefers_a_resolved_indirect_head_over_a_same_named_decoy() {
    // TP — issue #1133. The analyser settles `${ns}::setdef` to
    // `::tc::setdef`; the provider used to ignore that and answer with
    // `::other::setdef`, which the call can never reach.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, INDIRECT_HEAD);
    assert_eq!(
        locations(&lsp.definition(&uri, 3, 7)),
        vec![(0, 27)],
        "must reach ::tc::setdef on line 0, not the ::other decoy on line 1",
    );
}

#[test]
fn hover_agrees_with_definition_on_the_indirect_head() {
    // Self-consistency: the two providers describe the same command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, INDIRECT_HEAD);
    let text = hover_text(&lsp.hover(&uri, 3, 7));
    assert!(
        text.contains("::tc::setdef"),
        "hover must describe the resolved target: {text}",
    );
}

#[test]
fn a_bare_helper_call_in_a_method_body_resolves_across_files() {
    // TP — issue #1137 idx 51, the ticklecharts layout: the helper is
    // installed into `::oo::Helpers` in one file and called bare from a
    // method body in another. A method body's `namespace path` is
    // unconditionally `::oo::Helpers` (tclsh 8.6.16 / 9.0.4), so this is a
    // real cross-file reference; before the candidate-list fix the one
    // cross-file resolver could never see it.
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-oohelpers-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).expect("create workspace root");
    std::fs::write(
        root.join("utils.tcl"),
        "proc ::oo::Helpers::callback {method args} {\n    return [list my $method {*}$args]\n}\n",
    )
    .expect("write utils.tcl");
    let esnap = root.join("esnap.tcl");
    std::fs::write(
        &esnap,
        "oo::class create Snap {\n    method Start {} {\n        return [callback ReadBrowser]\n    }\n    method ReadBrowser {} { return 1 }\n}\n",
    )
    .expect("write esnap.tcl");

    let mut lsp = Lsp::at_workspace_root(&root);
    let uri = format!("file://{}", esnap.to_string_lossy());
    lsp.open_ready(&uri, &std::fs::read_to_string(&esnap).expect("read"));
    let result = lsp.definition(&uri, 2, 17);
    let targets = result.as_array().cloned().unwrap_or_default();
    assert!(
        targets.iter().any(|loc| {
            loc.get("uri")
                .or_else(|| loc.get("targetUri"))
                .and_then(Value::as_str)
                .is_some_and(|u| u.ends_with("utils.tcl"))
        }),
        "the bare `callback` call must reach ::oo::Helpers::callback in utils.tcl: {result}",
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn document_symbol_homes_a_qualified_proc_where_hover_says_it_lives() {
    // Issue #1140 idx 67 — the outline used to contradict hover inside the
    // very same session: hover said `::pix::svg::parse`, the outline showed
    // a bare top-level `parse` disconnected from the `pix > svg` tree.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let text = concat!(
        "namespace eval ::pix {\n",
        "    namespace eval svg {\n",
        "    }\n",
        "}\n",
        "proc pix::svg::parse {a} {\n",
        "    return $a\n",
        "}\n",
    );
    lsp.open_ready(&uri, text);

    let hover = hover_text(&lsp.hover(&uri, 4, 15));
    assert!(
        hover.contains("::pix::svg::parse"),
        "hover baseline: {hover}",
    );

    let symbols = lsp.document_symbols(&uri);
    let pix = symbols
        .as_array()
        .and_then(|top| top.first())
        .cloned()
        .unwrap_or(Value::Null);
    assert_eq!(pix["name"], "::pix", "{symbols}");
    let svg = &pix["children"][0];
    assert_eq!(svg["name"], "svg", "{symbols}");
    assert_eq!(
        svg["children"][0]["name"], "parse",
        "the outline must agree with hover: {symbols}",
    );
}


#[test]
fn document_link_never_fabricates_a_target_for_a_computed_source_path() {
    // Issue #1140 idx 41 — the provider percent-encoded the raw unevaluated
    // Tcl text and joined it onto the workspace directory, yielding a
    // syntactically valid but semantically bogus `file://` URI.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "source [file join $undeclared .. pkgfile.tcl]\nsource $other\n",
    );
    let links = lsp.document_links(&uri);
    let bogus = links
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| l.get("target").and_then(Value::as_str))
        .find(|t| t.contains("%5B") || t.contains('$') || t.contains('['));
    assert!(
        bogus.is_none(),
        "no link may encode unevaluated Tcl text: {links}",
    );
}
