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

//! IRULE2001 and IRULE2002 deprecated iRules commands and the W143 private-namespace advice.

use super::*;

#[test]
fn irule2001_carries_matchclass_replacement_fix() {
    // `matchclass <item> <class>` → `class match <item> equals <class>`,
    // replacing the whole command. Raw source slices preserve `$var`. The
    // fix span is independent of the diagnostic span (which is the head).
    let mut a = Analyser::new();
    let r = a.analyse("matchclass $u ::lib\n", "f5-irules");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::Irule2001)
        .expect("IRULE2001");
    assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
    let fix = &d.fixes[0];
    assert_eq!(fix.new_text, "class match $u equals ::lib");
    assert_eq!(fix.description, "Replace with 'class match'");
    // Fix range spans the whole command (head + both args), not just the
    // diagnostic's head span.
    assert_eq!(fix.span.start(), d.span.start());
    assert!(fix.span.end() > d.span.end());
}

#[test]
fn irule2001_three_arg_matchclass_fix_preserves_operator_and_class() {
    // The 3-arg form `matchclass <item> <operator> <class>` is a 1:1 rename
    // to `class match <item> <operator> <class>` — it must NOT be forced to
    // `equals` (which dropped the real class).
    let mut a = Analyser::new();
    let r = a.analyse(
        "matchclass [HTTP::uri] starts_with $::admin_paths\n",
        "f5-irules",
    );
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::Irule2001)
        .expect("IRULE2001");
    assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
    assert_eq!(
        d.fixes[0].new_text,
        "class match [HTTP::uri] starts_with $::admin_paths"
    );
}

#[test]
fn irule2001_ambiguous_arity_matchclass_warns_without_fix() {
    // A 1-arg (or any non-2/3-arg) `matchclass` is ambiguous: still warn,
    // but offer no quick-fix rather than corrupt the command.
    let mut a = Analyser::new();
    let r = a.analyse("matchclass [HTTP::uri]\n", "f5-irules");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::Irule2001)
        .expect("IRULE2001");
    assert_eq!(d.fixes.len(), 0, "expected no fix, got {:?}", d.fixes);
}

#[test]
fn irule2002_flags_deprecated_irules_command() {
    // `HTTP::class` is deprecated → `CLASSIFY::application`.
    let mut a = Analyser::new();
    let r = a.analyse("when HTTP_REQUEST {\n  HTTP::class\n}\n", "f5-irules");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::Irule2002)
        .expect("IRULE2002");
    assert_eq!(d.severity, Severity::Warning);
    assert!(
        d.message.contains(
            "'HTTP::class' is deprecated in iRules. Use 'CLASSIFY::application' instead."
        ),
        "{d:?}"
    );
}

#[test]
fn irule2002_silent_in_plain_tcl_dialect() {
    // The deprecation check is iRules-only.
    assert!(!has_code("HTTP::class\n", "tcl8.6", "IRULE2002"));
}

#[test]
fn w143_package_namespace_advice_keeps_literal_package_names() {
    for (package, expected) in [
        ("tcl::dict", false),
        (":tcl::dict", true),
        ("::tcl::dict", true),
        ("tcl::dictionary", true),
    ] {
        let source = format!("package require {package}\n::tcl::dict::unregistered_member\n");
        let result = Analyser::new().analyse(&source, "tcl8.6");
        assert_eq!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::W143),
            expected,
            "{package}"
        );
    }
}

#[test]
fn w143_withdraws_for_a_private_namespace_command_the_document_defines() {
    let own =
        "namespace eval ::tcl::dict { proc mine {d} { return $d } }\n::tcl::dict::mine {a 1}\n";
    assert!(!has_code(own, "tcl8.6", "W143"));
    assert!(has_code("::tcl::dict::mine {a 1}\n", "tcl8.6", "W143"));
}
