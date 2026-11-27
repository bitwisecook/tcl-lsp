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
//! The Jim roster these cases read is registered by installing
//! `tcl_spectcl::core_surfaces::SURFACES`, so they run here rather than beside
//! the analyser.

use tcl_compiler::analyser::Analyser;
use tcl_core_types::DiagCode;

/// The diagnostics a document draws on the head `word`: its code and message.
fn on_head(dialect: &str, word: &str, body: &str) -> Vec<(DiagCode, String)> {
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

/// A command some dialect related to the document has is disabled, and where
/// only another family has it the report names that family:
///
/// - a `tcl8.4` document writing `lmap`: the command exists in the family the
///   document belongs to, only not at the release it targets;
/// - a `jim` document writing `coroutine`: Jim reimplements Tcl's surface, so
///   the command is Tcl's and Jim's roster omits it;
/// - a Tcl 8.4, 8.6 or 9.0 document writing `curry`, which only Jim has: Jim
///   is on Tcl 8.6's line, so the report says `available in: jim`;
/// - a `tcl8.6` document writing Expect's `system`: a Tcl document could
///   `package require Expect`.
#[test]
fn a_command_a_related_dialect_has_is_disabled_under_tcl_8_4_to_9_0_and_jim() {
    const DISABLED: &str = "disabled in the active dialect profile";
    crate::install();
    for (dialect, head, body, wording) in [
        ("tcl8.4", "lmap", "lmap x {1 2} {puts $x}", &[DISABLED][..]),
        ("jim", "coroutine", "coroutine gen {puts 1}", &[DISABLED]),
        (
            "tcl8.4",
            "curry",
            "curry puts hello",
            &[DISABLED, "available in: jim"],
        ),
        (
            "tcl8.6",
            "curry",
            "curry puts hello",
            &[DISABLED, "available in: jim"],
        ),
        (
            "tcl9.0",
            "curry",
            "curry puts hello",
            &[DISABLED, "available in: jim"],
        ),
        ("tcl8.6", "system", "system ls", &[]),
    ] {
        let found = on_head(dialect, head, body);
        assert!(
            found.iter().any(|(code, message)| *code == DiagCode::W002
                && wording.iter().all(|part| message.contains(part))),
            "{dialect} `{head}`: {found:?}"
        );
    }
}

/// A `jim` document writing Expect's `system`: no family Jim derives from or
/// shares packages with has it, so it is an unknown command, not a disabled
/// one.
#[test]
fn a_jim_document_using_an_expect_command_finds_it_unknown() {
    crate::install();
    let found = on_head("jim", "system", "system ls");
    let codes: Vec<DiagCode> = found.iter().map(|(code, _)| *code).collect();
    assert_eq!(codes, [DiagCode::W123], "{found:?}");
    assert!(found[0].1.contains("Unknown command"), "{found:?}");
}
