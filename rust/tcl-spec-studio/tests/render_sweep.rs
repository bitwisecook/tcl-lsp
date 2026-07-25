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

//! Render every command in every browsable dialect and check the output holds.
//!
//! The studio's promise is that a spec loaded out of the registry renders back
//! to a file you can drop into `tcl-registry` — so the renderer has to survive
//! the whole real command surface, not a handful of hand-picked examples. This
//! sweep runs the renderer over every command in every dialect the studio
//! offers (several thousand specs) and asserts the structural invariants a
//! valid spec file has.
//!
//! ## Checking that the output actually compiles
//!
//! These assertions are structural; they cannot prove the result is valid
//! Rust. To check that for real, render the specs into the registry and build
//! it — the four renderer bugs this file's invariants do *not* catch (a
//! non-const `|` in a hoisted table, a nested enum payload losing its type
//! path, an associated function called as a method, an unmapped dialect name)
//! were all found this way:
//!
//! ```text
//! # from the repository root, with a throwaway example that calls
//! # tcl_spec_studio::render_rs::render over command_names("tcl9.0"):
//! cargo run -p tcl-spec-studio --example gen -- \
//!     rust/tcl-registry/src/commands/studio_gen_check
//! # add `pub mod studio_gen_check;` to commands/mod.rs, then:
//! cargo check -p tcl-registry
//! # …and revert both when done.
//! ```

use tcl_spec_studio::{BROWSABLE_DIALECTS, command_names, draft, load_command, render_rs};

/// Whether every delimiter in `source` is balanced, ignoring the contents of
/// string literals and line comments (where a stray brace is legitimate prose).
fn delimiters_balanced(source: &str) -> bool {
    let mut depth: Vec<char> = Vec::new();
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut in_comment = false;
    while let Some(ch) = chars.next() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }
        if in_string {
            if ch == '\\' {
                chars.next();
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '/' if chars.peek() == Some(&'/') => in_comment = true,
            '{' | '[' | '(' => depth.push(ch),
            '}' => {
                if depth.pop() != Some('{') {
                    return false;
                }
            }
            ']' => {
                if depth.pop() != Some('[') {
                    return false;
                }
            }
            ')' if depth.pop() != Some('(') => {
                return false;
            }
            _ => {}
        }
    }
    depth.is_empty() && !in_string
}

#[test]
fn every_command_in_every_dialect_renders() {
    let mut rendered = 0usize;
    for (dialect, _) in BROWSABLE_DIALECTS {
        for name in command_names(dialect) {
            let Some(value) = load_command(name, dialect) else {
                panic!("{name} is listed in {dialect} but does not load");
            };
            let draft = value
                .as_object()
                .unwrap_or_else(|| panic!("{name} did not load as an object"))
                .clone();
            let source = render_rs::render(&draft);

            assert!(
                source.starts_with(render_rs::COPYRIGHT_BANNER),
                "{dialect}/{name}: the copyright banner must lead every rendered file"
            );
            assert!(
                source.contains("use crate::prelude::*;"),
                "{dialect}/{name}: the prelude import is missing"
            );
            assert!(
                source.contains("pub fn spec() -> CommandSpec {"),
                "{dialect}/{name}: the spec constructor is missing"
            );
            assert!(
                source.trim_end().ends_with('}'),
                "{dialect}/{name}: the file is truncated"
            );
            assert!(
                source.contains("..CommandSpec::DEFAULT"),
                "{dialect}/{name}: the DEFAULT fill is missing, so unset fields would be dropped"
            );
            assert!(
                delimiters_balanced(&source),
                "{dialect}/{name}: unbalanced delimiters in the rendered source"
            );
            // A bitflags `|` chain is not const-evaluable inside the hoisted
            // `const OPTIONS` / `const SUBCOMMANDS` tables; the renderer must
            // emit `.union(…)` everywhere instead.
            assert!(
                !source.contains("Traits::") || !source.contains(" | Traits::"),
                "{dialect}/{name}: a `|` trait chain will not compile in a const table"
            );
            rendered += 1;
        }
    }
    assert!(
        rendered > 2000,
        "expected the full command surface, rendered only {rendered}"
    );
}

#[test]
fn a_default_draft_renders_a_minimal_but_complete_spec() {
    let mut d = draft::default_command_draft();
    d.insert("name".into(), serde_json::json!("mycommand"));
    let source = render_rs::render(&d);
    assert!(source.starts_with(render_rs::COPYRIGHT_BANNER));
    assert!(source.contains("name: \"mycommand\","));
    assert!(delimiters_balanced(&source));
    // Nothing else is set, so nothing else should be emitted.
    let body = source
        .split_once("CommandSpec {")
        .expect("the spec literal is present")
        .1;
    let fields: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|line| line.contains(':') && !line.starts_with("//") && !line.starts_with(".."))
        .collect();
    assert_eq!(
        fields,
        vec!["name: \"mycommand\","],
        "a default draft must emit only what the author actually set"
    );
}
