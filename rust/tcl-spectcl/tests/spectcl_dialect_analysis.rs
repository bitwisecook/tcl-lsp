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

//! Editing a `.tclspec` is editing Tcl under the `spectcl` dialect, so the
//! pack's own vocabulary has to analyse cleanly.
//!
//! A block statement's inner words are **not** commands anywhere else — that
//! context-sensitivity is the whole reason `properties.rs` gives them no
//! `CommandSpec` (it is what stops a `hover` key from shadowing Tcl's real
//! `source`). The registry's answer for a curated body vocabulary is
//! `CommandSpec::body_scope`, and this file is the claim that the `hover`
//! block really carries one: every documentation key the loader reads resolves
//! inside the block, and nowhere outside it.

use std::fmt::Write as _;

use tcl_compiler::analyser::Analyser;
use tcl_core_types::DiagCode;

/// The six keys `loader::hover_block` reads, in the order the DSL documents
/// them.
const HOVER_KEYS: &[&str] = &[
    "summary",
    "synopsis",
    "description",
    "source",
    "example",
    "returns",
];

fn diagnostics(source: &str) -> Vec<(DiagCode, String)> {
    let mut analyser = Analyser::new();
    analyser
        .analyse(source, "spectcl")
        .diagnostics
        .iter()
        .map(|d| (d.code, d.message.clone()))
        .collect()
}

/// A pack shaped like the shipped ones: a command with a full `hover` block.
fn pack_with_a_full_hover_block() -> String {
    let mut body = String::new();
    for key in HOVER_KEYS {
        let _ = writeln!(body, "            {key} {{Prose for {key}.}}");
    }
    format!(
        "speclib probe 1.1 {{\n    command demo {{\n        arity 1\n        hover {{\n{body}        }}\n    }}\n}}\n"
    )
}

/// The regression: every one of the six keys drew W123 "Unknown command",
/// because a `hover` body is walked as an ordinary script and only the two
/// keys that happen to name something else (`synopsis`, a scalar property;
/// `source`, a real Tcl command) resolved. The shipped `specs/` packs write
/// hundreds of these blocks.
#[test]
fn a_hover_block_draws_no_unknown_command_diagnostics() {
    let source = pack_with_a_full_hover_block();
    let unknown: Vec<String> = diagnostics(&source)
        .into_iter()
        .filter(|(code, _)| *code == DiagCode::W123)
        .map(|(_, message)| message)
        .collect();
    assert!(
        unknown.is_empty(),
        "every `hover` key must resolve inside the block: {unknown:?}\n{source}"
    );
}

/// The other half of the claim: the keys are ambient *only* inside the block.
/// A `summary` row written at command level is not a command, and the loader
/// drops it as an unknown property — so the editor must say so too.
#[test]
fn a_hover_key_written_outside_the_block_is_still_unknown() {
    let source = "speclib probe 1.1 {\n    command demo {\n        arity 1\n        summary {Not here.}\n    }\n}\n";
    assert!(
        diagnostics(source)
            .iter()
            .any(|(code, message)| *code == DiagCode::W123 && message.contains("'summary'")),
        "`summary` is a hover key, not a command property: {:?}",
        diagnostics(source)
    );
    // …and the loader agrees, which is what makes the diagnostic true.
    let pack = tcl_spectcl::load_pack(source);
    assert!(
        pack.notices.iter().any(|notice| notice
            .message
            .contains("unknown property `summary` dropped")),
        "{:?}",
        pack.notices
    );
}

/// The two halves of the block vocabulary cannot drift: the grammar is what
/// folds and paints the block, the scoped environment is what resolves its
/// words as commands, and a key missing from either is invisible in one of
/// the two.
#[test]
fn the_hover_grammar_and_its_command_environment_list_the_same_keys() {
    let spec = tcl_registry::model::static_context_for("spectcl")
        .commands()
        .get("hover")
        .expect("the spectcl dialect declares `hover`")
        .clone();
    let grammar = spec
        .definition_body
        .expect("`hover` carries its block grammar");
    let env = spec
        .body_scope
        .expect("`hover` carries its block command environment");

    let mut members: Vec<&str> = grammar.members.iter().map(|m| m.keyword).collect();
    let mut commands: Vec<&str> = env.commands.iter().map(|c| c.name).collect();
    let mut expected: Vec<&str> = HOVER_KEYS.to_vec();
    members.sort_unstable();
    commands.sort_unstable();
    expected.sort_unstable();
    assert_eq!(members, expected, "the hover grammar drifted");
    assert_eq!(commands, expected, "the hover command environment drifted");

    // Each key is documented where the editor shows it.
    for command in env.commands {
        assert!(
            command.hover.is_some_and(|h| !h.summary.is_empty()),
            "`{}` needs hover text",
            command.name
        );
    }
}
