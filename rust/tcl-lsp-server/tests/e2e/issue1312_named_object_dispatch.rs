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

//! Issue #1312 — `ClassName create objName` resolves no members, where
//! `[ClassName new]` gives full member support (W308, hover, completion,
//! go-to-definition, semantic tokens).
//!
//! Go-to-definition, hover, and completion already resolved this shape
//! (`AnalysisResult::instance_classes` / `created_instance_commands`, read
//! by `tcl_lsp_core::definition::receiver_instance_class`) before this fix —
//! only the W308 diagnostic and the semantic-token `method` classification
//! abstained.  This file exercises all four so a future regression in any
//! one of them is caught here.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};
use serde_json::Value;

const SRC: &str =
    "oo::class create C {\n    method mrun {} { return 1 }\n}\nC create obj\nobj nosuchmethod\n";
const SRC_GOOD: &str =
    "oo::class create C {\n    method mrun {} { return 1 }\n}\nC create obj\nobj mrun\n";

/// TP — the ticket's own repro: W308 fires on the named-object form exactly
/// as it already does on the handle form (`[C new]`).
#[test]
fn named_object_unknown_method_draws_w308() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, SRC);
    let codes: Vec<&str> = diags
        .iter()
        .filter_map(|d| d.get("code").and_then(Value::as_str))
        .collect();
    assert!(
        codes.contains(&"W308"),
        "named-object dispatch must draw W308: {diags:?}"
    );
}

/// TN — a named object calling a method its class actually declares must
/// not draw a spurious W308.
#[test]
fn named_object_real_method_draws_no_w308() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, SRC_GOOD);
    let codes: Vec<&str> = diags
        .iter()
        .filter_map(|d| d.get("code").and_then(Value::as_str))
        .collect();
    assert!(
        !codes.contains(&"W308"),
        "a real method must not draw W308: {diags:?}"
    );
}

/// TP — the semantic-token half: `mrun` on line 4 (`obj mrun`) is coloured
/// as a `method`, not a plain string.
#[test]
fn named_object_method_word_is_a_method_token() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, SRC_GOOD);
    let legend =
        lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .expect("tokenTypes legend")
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_owned())
            .collect::<Vec<_>>();
    let raw = lsp.semantic_tokens_settled(&uri);
    let tokens = decode_semantic_tokens(&raw);
    let method_line = 4; // 0-based: `obj mrun`
    let found = tokens.iter().any(|t| {
        t.line == method_line
            && legend
                .get(usize::try_from(t.ttype).unwrap_or(usize::MAX))
                .is_some_and(|ty| ty == "method")
    });
    assert!(
        found,
        "expected a `method` token on line {method_line}: {tokens:?} (legend={legend:?})"
    );
}

/// TP (regression guard) — go-to-definition on the named object's method
/// already resolved before this fix; this pins it against a future
/// regression from the shared-resolver change.
#[test]
fn named_object_go_to_definition_resolves() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, SRC_GOOD);
    // `obj mrun` — `mrun` starts at character 4 of line 4 (0-based).
    let found = locations(&lsp.definition(&uri, 4, 4));
    assert_eq!(
        found.len(),
        1,
        "named-object go-to-definition must resolve `mrun`: {found:?}"
    );
}

/// TP (regression guard) — completion on the named object offers the
/// class's declared method.
#[test]
fn named_object_completion_offers_the_declared_method() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, SRC_GOOD);
    // Cursor right after `obj ` on line 4 (0-based), before `mrun`.
    let result = lsp.completion(&uri, 4, 4);
    let labels = completion_labels(&result);
    assert!(
        labels.iter().any(|l| l == "mrun"),
        "expected `mrun` in named-object completions: {labels:?}"
    );
}
