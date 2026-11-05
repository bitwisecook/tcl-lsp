// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Architecture guard for the dispatch-stability proof pass and its GVN
//! consumer: per-command knowledge lives in `tcl-registry`, so no production
//! line of these modules may name a registered Tcl command.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Production modules that consume dispatch-stability facts.
const GUARDED_MODULES: [&str; 3] = [
    "rust/tcl-compiler/src/dispatch_proof.rs",
    "rust/tcl-compiler/src/gvn.rs",
    "rust/tcl-compiler/src/world_state_ssa.rs",
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn source(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

/// Everything before the first `#[cfg(test)]` attribute.
///
/// Test modules legitimately spell command names — they build real Tcl
/// scripts — so only the production region is guarded.
fn production_region(contents: &str) -> &str {
    match contents.find("#[cfg(test)]") {
        Some(offset) => &contents[..offset],
        None => contents,
    }
}

/// Collect the contents of double-quoted string literals.
///
/// A deliberately small scanner, not a Rust parser: it steps over character
/// and byte-character literals so an embedded `b'"'` cannot desynchronise it,
/// and treats a backslash escape as covering the next character so `\"` never
/// closes a literal early.
fn quoted_literals(source: &str) -> Vec<String> {
    let characters: Vec<char> = source.chars().collect();
    let mut literals = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        match characters[index] {
            '\'' => index += character_literal_width(&characters, index),
            '"' => {
                index += 1;
                let mut literal = String::new();
                while index < characters.len() && characters[index] != '"' {
                    if characters[index] == '\\' {
                        index += 1;
                    }
                    if index < characters.len() {
                        literal.push(characters[index]);
                        index += 1;
                    }
                }
                index += 1;
                literals.push(literal);
            }
            _ => index += 1,
        }
    }
    literals
}

/// How many characters a character literal starting at `index` occupies, or
/// `1` when the quote opens a lifetime rather than a literal.
fn character_literal_width(characters: &[char], index: usize) -> usize {
    if characters.get(index + 1) == Some(&'\\') {
        // The escaped character itself is at `index + 2` and may legitimately
        // be a quote, so look for the closer from `index + 3` onwards.
        let mut end = index + 3;
        while end < characters.len() && characters[end] != '\'' {
            end += 1;
        }
        return end + 1 - index;
    }
    if characters.get(index + 2) == Some(&'\'') {
        return 3;
    }
    1
}

/// Literals that equal a registered command name for a reason unrelated to
/// dispatch, with the justification that keeps them out of the guard.
///
/// Kept exact: a stale entry fails the test just as a new violation does.
const ALLOWED_LITERALS: [(&str, &str); 1] = [(
    "package",
    "WorldTrack::PackageState's value-key label; names a world-state partition \
     shared with world-state SSA, not the `package` command",
)];

#[test]
fn no_production_line_of_the_dispatch_proof_consumers_names_a_registry_command() {
    let root = workspace_root();
    let registry = tcl_registry::CommandRegistry::build_default();
    let command_names: BTreeSet<&str> = registry.command_names().collect();

    // Guard the guard: a scanner that silently found nothing would make the
    // whole check vacuous.
    let dispatch_proof = source(&root, GUARDED_MODULES[0]);
    assert!(
        quoted_literals(production_region(&dispatch_proof))
            .iter()
            .any(|literal| literal == "cmdbind"),
        "the literal scanner must reach the world-track value-key labels"
    );

    let mut violations: Vec<String> = Vec::new();
    let mut allowlist_hits: BTreeSet<&str> = BTreeSet::new();
    for relative in GUARDED_MODULES {
        let contents = source(&root, relative);
        for literal in quoted_literals(production_region(&contents)) {
            if !command_names.contains(literal.as_str()) {
                continue;
            }
            match ALLOWED_LITERALS
                .iter()
                .find(|(allowed, _)| *allowed == literal)
            {
                Some((allowed, _)) => {
                    allowlist_hits.insert(allowed);
                }
                None => violations.push(format!("{relative}: {literal:?}")),
            }
        }
    }

    assert!(
        violations.is_empty(),
        "per-command knowledge belongs in tcl-registry, not in a consumer: {violations:?}"
    );

    // The allowlist stays exact: an entry whose literal no longer appears in
    // any guarded production region is dead weight and fails here.
    for (allowed, justification) in ALLOWED_LITERALS {
        assert!(
            allowlist_hits.contains(allowed),
            "stale allowlist entry {allowed:?} ({justification}) — no guarded production region \
             still contains it"
        );
    }
}

#[test]
fn the_dispatch_proof_pass_never_matches_on_a_canonical_command_name() {
    let root = workspace_root();
    let contents = source(&root, "rust/tcl-compiler/src/dispatch_proof.rs");
    let production = production_region(&contents);
    for forbidden in ["canonical_command ==", "== canonical_command"] {
        assert!(
            !production.contains(forbidden),
            "dispatch_proof.rs must dispatch on typed registry facts, never on command text \
             ({forbidden})"
        );
    }
}
