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

//! W113: a `proc` that shadows a built-in command.

use super::*;

// handle_proc_command W113 shadow check

#[test]
fn handle_proc_emits_w113_for_builtin_shadow() {
    // ``proc set {} {}`` — the proc name is a built-in.
    // W113 should anchor at the proc-name span and carry
    // the canonical message shape. A *real* dialect is named in the
    // parenthetical label; the permissive fallback (both the empty
    // string and its canonical "tcl" spelling resolve there —
    // dialect-profile-model.md §8, one sink) carries no label, covered
    // by the sibling no-label test below.
    let mut a = Analyser::new();
    a.profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    a.handle_proc_command(
        "proc",
        &["set".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    let w113s: Vec<&crate::analyser::types::Diagnostic> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W113)
        .collect();
    assert_eq!(w113s.len(), 1);
    assert_eq!(w113s[0].span, span(5, 8));
    assert!(w113s[0].message.contains("'set' shadows built-in"));
    assert!(w113s[0].message.contains("(tcl8.6)"));
    assert_eq!(w113s[0].severity, crate::analyser::types::Severity::Warning);
}

#[test]
fn irules_profile_without_attached_registry_keeps_valid_proc_semantics() {
    let mut analyser = Analyser::new();
    analyser.profile = tcl_dialect::DialectProfile::irules();
    analyser.source = "proc set {} {}".to_owned().into();
    assert!(analyser.registry.is_none());
    let handled = analyser.handle_proc_command(
        "proc",
        &["set".to_owned(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[true, true, true],
        &[],
    );
    assert!(handled);
    assert!(analyser.result.all_procs.contains_key("::set"));
    assert!(
        analyser
            .result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W113)
    );
}

#[test]
fn irules_profile_without_attached_registry_rejects_malformed_proc() {
    let mut analyser = Analyser::new();
    analyser.profile = tcl_dialect::DialectProfile::irules();
    assert!(analyser.registry.is_none());
    let handled = analyser.handle_proc_command(
        "proc",
        &[
            "set".to_owned(),
            String::new(),
            "set leaked 1".to_owned(),
            "extra".to_owned(),
        ],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 26)),
            esc_tok(span(27, 32)),
        ],
        &[],
        &[],
    );
    assert!(handled);
    assert_eq!(analyser.result.all_procs.len(), 0);
    assert_eq!(analyser.result.global_scope.variables.len(), 0);
    assert_eq!(
        analyser.result.diagnostics,
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn handle_proc_no_w113_for_non_builtin_name() {
    // ``foo`` is not a built-in — no W113.
    let mut a = Analyser::new();
    a.profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113),
        "should NOT emit W113 for non-built-in name 'foo'",
    );
}

#[test]
fn handle_proc_w113_matches_qualified_form() {
    // ``proc ::set {} {}`` — qualified form also shadows
    // ``set`` because the registry indexes by bare command
    // name (``::`` is trimmed at lookup).
    let mut a = Analyser::new();
    a.profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    a.handle_proc_command(
        "proc",
        &["::set".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 10)),
            str_tok(span(11, 13)),
            str_tok(span(14, 16)),
        ],
        &[],
        &[],
    );
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113)
    );
}

#[test]
fn handle_proc_w113_no_dialect_label_when_dialect_empty() {
    // Empty dialect → no parenthetical label in the message.
    let mut a = Analyser::new();
    // dialect intentionally left empty
    a.handle_proc_command(
        "proc",
        &["set".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    let w113 = a
        .result
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W113)
        .expect("W113 expected");
    assert!(w113.message.contains("'set' shadows built-in"));
    assert!(!w113.message.contains('('), "no dialect label expected");
}

#[test]
fn handle_proc_w113_dialect_specific_command_only_shadows_in_that_dialect() {
    // ``pool`` is an *unqualified* iRules-specific command; under the
    // ``f5-irules`` dialect a proc named ``pool`` shadows a built-in,
    // but under plain ``tcl`` it does not.  (A *namespace-qualified*
    // iRules command like ``HTTP::respond`` is never flagged, because a
    // qualified match is a library/package command living in its own
    // namespace, not a core-global shadow.)
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::irules();
    a.source = "proc pool {} {}".to_owned().into();
    a.handle_proc_command(
        "proc",
        &["pool".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 9)),
            str_tok(span(10, 12)),
            str_tok(span(13, 15)),
        ],
        &[true, true, true],
        &[],
    );
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113),
        "f5-irules dialect should treat pool as built-in",
    );

    // Same proc, plain tcl dialect → no W113.
    let mut b = Analyser::new();
    b.profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    b.handle_proc_command(
        "proc",
        &["pool".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 9)),
            str_tok(span(10, 12)),
            str_tok(span(13, 15)),
        ],
        &[],
        &[],
    );
    assert!(
        !b.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113),
        "plain tcl dialect should NOT flag pool",
    );
}

#[test]
fn handle_proc_w113_silent_on_namespace_qualified_shadow() {
    // FP: a namespace-qualified match (`HTTP::respond` under f5-irules)
    // is a library/package command in its own namespace, not a
    // core-global shadow — never W113.
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::irules();
    a.handle_proc_command(
        "proc",
        &["HTTP::respond".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 18)),
            str_tok(span(19, 21)),
            str_tok(span(22, 24)),
        ],
        &[],
        &[],
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113),
        "namespace-qualified HTTP::respond must not fire W113",
    );
}
