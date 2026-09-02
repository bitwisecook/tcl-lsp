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

//! Command-position completion inside an **authoring dialect**, where a
//! command position is not open: the words legal at a position are exactly
//! the enclosing definition-body grammar's members.
//!
//! The root of a document is the same question one level up — an authoring
//! dialect declares a `CommandRegistry::document_grammar` — so the whole
//! answer is registry data and this suite is the behavioural proof that the
//! provider names no dialect and no declaration.
//!
//! registry-metadata: these are our own DSLs, so their vocabulary tables
//! (`docs/design/sslictcl-vocabulary.md`, `docs/design/spec-packs.md`) are the
//! oracle, not C Tcl.

use tcl_compiler::analyser::Analyser;
use tcl_lsp_core::completion::completions;

/// Completion labels at `line`/`character` under `dialect`.
fn labels_at(source: &str, dialect: &'static str, line: u32, character: u32) -> Vec<String> {
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(source, dialect).clone();
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let mut labels: Vec<String> = completions(
        source,
        line,
        character,
        &analysis,
        Some(registry),
        None,
        tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
    )
    .into_iter()
    .map(|item| item.label)
    .collect();
    labels.sort();
    labels
}

/// At the root of a `.sslictcl` document the offer is exactly the nine
/// top-level declarations — not the whole command registry, which used to put
/// `hostname`, `severity`, and every base Tcl command at a position where none
/// of them is legal.
#[test]
fn the_root_of_a_document_offers_exactly_its_declarations() {
    let src = "sslictcl 1\n\nendpoint www {\n    hostname www.example.com\n}\n\n";
    let labels = labels_at(src, "sslictcl", 5, 0);
    assert_eq!(
        labels,
        [
            "certificate",
            "chain",
            "cipher",
            "endpoint",
            "policy",
            "protocol",
            "sslictcl",
            "testssl-import",
            "trust-program",
        ],
        "the root vocabulary is the document grammar and nothing else"
    );
}

/// Inside an `hsts { … }` block the offer is exactly its four members — not
/// the members of unrelated blocks, and not the top-level declarations.
#[test]
fn a_nested_block_offers_exactly_its_own_members() {
    let src = "sslictcl 1\n\
               endpoint www {\n\
               \x20   hostname www.example.com\n\
               \x20   hsts {\n\
               \x20       \n\
               \x20   }\n\
               }\n";
    let labels = labels_at(src, "sslictcl", 4, 8);
    assert_eq!(
        labels,
        ["enabled", "include-subdomains", "max-age", "preload"],
        "an hsts block's vocabulary is its own four members"
    );

    // …and one level out, the endpoint's own members — `hostname` is legal
    // here and was not legal inside the `hsts` block above.
    let outer = labels_at(src, "sslictcl", 2, 4);
    assert!(outer.contains(&"hostname".to_owned()), "{outer:?}");
    assert!(outer.contains(&"hsts".to_owned()), "{outer:?}");
    assert!(
        !outer.contains(&"enabled".to_owned()),
        "an hsts member is not legal in the endpoint body: {outer:?}"
    );
    assert!(
        !outer.contains(&"endpoint".to_owned()),
        "a top-level declaration is not legal inside a block: {outer:?}"
    );
}

/// A partial word filters the grammar's members, exactly as it filters an
/// ordinary command list.
#[test]
fn a_partial_word_filters_the_grammar_members() {
    let src = "sslictcl 1\nendpoint www {\n    prot\n}\n";
    let labels = labels_at(src, "sslictcl", 2, 8);
    assert_eq!(
        labels,
        ["protocols"],
        "only the matching member: {labels:?}"
    );
}

/// A body under a command with **no** grammar falls back to ordinary
/// completion. `SslicTcl`'s `predicate { … }` is retained data the loader never
/// evaluates, so nothing there is a declaration — and offering the enclosing
/// `check` members would be a lie about what the position means.
#[test]
fn a_body_with_no_grammar_falls_back_to_ordinary_completion() {
    let src = "sslictcl 1\n\
               policy baseline {\n\
               \x20   check bespoke {\n\
               \x20       predicate {\n\
               \x20           se\n\
               \x20       }\n\
               \x20   }\n\
               }\n";
    let labels = labels_at(src, "sslictcl", 4, 14);
    assert!(
        labels.contains(&"set".to_owned()),
        "ordinary Tcl completion resumes inside a grammarless body: {labels:?}"
    );
    assert!(
        !labels.contains(&"severity".to_owned()),
        "a `check` member is not legal inside a retained predicate: {labels:?}"
    );
}

/// The same machinery, unchanged, for the other authoring dialect: a
/// `.tclspec` pack's root offers `speclib` and nothing else, and a `command
/// { … }` body offers that block's own vocabulary.
#[test]
fn spectcl_packs_get_the_same_answer() {
    let root = labels_at("speclib mylib 1.0 {\n}\n\n", "spectcl", 2, 0);
    assert_eq!(
        root,
        ["speclib"],
        "a pack's only top-level word is `speclib`"
    );

    let src = "speclib mylib 1.0 {\n\
               \x20   command with_var {\n\
               \x20       \n\
               \x20   }\n\
               }\n";
    let inside = labels_at(src, "spectcl", 2, 8);
    assert!(inside.contains(&"arity".to_owned()), "{inside:?}");
    assert!(inside.contains(&"hover".to_owned()), "{inside:?}");
    assert!(
        !inside.contains(&"speclib".to_owned()),
        "`speclib` is a root word, not a command-block member: {inside:?}"
    );
    assert!(
        !inside.contains(&"summary".to_owned()),
        "a `hover` block key is not legal in a `command` body: {inside:?}"
    );
}

/// An ordinary Tcl document has no document grammar, so its root stays an
/// open command position and completion is unchanged.
#[test]
fn plain_tcl_completion_is_unchanged() {
    let labels = labels_at("whi\n", "tcl8.6", 0, 3);
    assert!(labels.contains(&"while".to_owned()), "{labels:?}");
    assert!(
        !labels.contains(&"puts".to_owned()),
        "the partial still filters: {labels:?}"
    );
    let all = labels_at("\n", "tcl9.0", 0, 0);
    assert!(
        all.contains(&"puts".to_owned()),
        "an open position offers the registry"
    );
    assert!(all.len() > 100, "…all of it, got {}", all.len());
}
