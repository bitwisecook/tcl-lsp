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

//! W002 says a command is *disabled in the active dialect profile*, which is
//! true only where some dialect the document is related to has the command.
//! A name only an unrelated environment offers is unknown, and W123 says so.
//!
//! The Jim roster these cases read is registered by
//! `tcl_spectcl::core_surfaces::ensure`, so they run here rather than beside
//! the analyser.

use tcl_compiler::analyser::Analyser;
use tcl_core_types::DiagCode;

/// The diagnostics a document draws on the head `word`: its code and message.
fn on_head(dialect: &str, word: &str, body: &str) -> Vec<(DiagCode, String)> {
    tcl_spectcl::core_surfaces::ensure();
    let source = format!("# tcl-dialect: {dialect}\n{body}\n");
    let result = Analyser::new().analyse(
        &source,
        tcl_registry::dialects::detect_dialect(&source, None, "tcl"),
    );
    result
        .diagnostics
        .iter()
        .filter(|d| {
            let text = &source[d.span.start() as usize..d.span.end() as usize];
            text == word
        })
        .map(|d| (d.code, d.message.clone()))
        .collect()
}

/// A `tcl8.4` document writing a `tcl8.6` command: the command exists in the
/// family the document belongs to, only not at the release it targets.
#[test]
fn an_older_tcl_document_using_a_newer_tcl_command_is_disabled() {
    let found = on_head("tcl8.4", "lmap", "lmap x {1 2} {puts $x}");
    assert!(
        found.iter().any(|(code, message)| *code == DiagCode::W002
            && message.contains("disabled in the active dialect profile")),
        "{found:?}"
    );
}

/// A `jim` document writing a Tcl command Jim's roster omits: Jim
/// reimplements Tcl's surface, so the command is Tcl's and Jim does not
/// have it.
#[test]
fn a_jim_document_using_a_tcl_command_jim_omits_is_disabled() {
    let found = on_head("jim", "coroutine", "coroutine gen {puts 1}");
    assert!(
        found.iter().any(|(code, message)| *code == DiagCode::W002
            && message.contains("disabled in the active dialect profile")),
        "{found:?}"
    );
}

/// A Tcl document writing a command only Jim has: Jim is on Tcl 8.6's line, so
/// the command is disabled here and the report names the dialect that has it.
#[test]
fn a_tcl_document_using_a_jim_only_command_is_disabled_and_names_jim() {
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let found = on_head(dialect, "curry", "curry puts hello");
        assert!(
            found.iter().any(|(code, message)| *code == DiagCode::W002
                && message.contains("disabled in the active dialect profile")
                && message.contains("available in: jim")),
            "{dialect}: {found:?}"
        );
    }
}

/// A `jim` document writing Expect's `system`: no family Jim derives from or
/// shares packages with has it, so it is an unknown command, not a disabled
/// one.
#[test]
fn a_jim_document_using_an_expect_command_finds_it_unknown() {
    let found = on_head("jim", "system", "system ls");
    let codes: Vec<DiagCode> = found.iter().map(|(code, _)| *code).collect();
    assert_eq!(codes, [DiagCode::W123], "{found:?}");
    assert!(found[0].1.contains("Unknown command"), "{found:?}");
}

/// The same command in a document of a family that ships Expect keeps the
/// disabled wording: `system` exists in the `expect` environment, and a
/// `tcl8.6` document could `package require Expect`.
#[test]
fn a_tcl_document_using_an_expect_command_keeps_the_disabled_wording() {
    let found = on_head("tcl8.6", "system", "system ls");
    assert!(
        found.iter().any(|(code, _)| *code == DiagCode::W002),
        "{found:?}"
    );
}
