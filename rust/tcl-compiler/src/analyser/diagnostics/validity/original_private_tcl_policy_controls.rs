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

use super::*;

#[test]
fn actual_source_policy_keeps_unknown_helpers_and_whole_written_fix_boundary() {
    for dialect in ["tcl8.6", "tcl"] {
        for (source, fixed) in [
            ("::tcl::dict::create a 1", true),
            ("{::tcl::dict::create} a 1", false),
            ("::tcl::clock::GetSystemTimeZone", false),
        ] {
            let result = Analyser::new().analyse(source, dialect);
            let diagnostic = result
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == DiagCode::W143)
                .unwrap();
            let subject = diagnostic.private_tcl_namespace().unwrap();
            assert!(subject.matches_source(source, &result));
            assert!(!subject.matches_source(&format!("{source} changed"), &result));
            assert_eq!(!diagnostic.fixes.is_empty(), fixed);
            assert!(subject.obligations().contains(
                &crate::analyser::SourcePrivateTclNamespaceObligation::UnavailableActualLookup
            ));
            if source.contains("GetSystemTimeZone") {
                assert!(subject.selected_source_words().is_none());
            }
        }
    }
}

#[test]
fn actual_original_declaration_and_require_own_suppression_not_report_names() {
    for dialect in ["tcl8.6", "tcl"] {
        for source in [
            "proc ::tcl::dict::mine {d} {return $d}; ::tcl::dict::mine value",
            "::tcl::dict::mine value; package require tcl::dict",
            "package require tcl::dict; ::tcl::dict::mine value",
        ] {
            let result = Analyser::new().analyse(source, dialect);
            assert!(
                !result
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == DiagCode::W143),
                "{source}: {dialect}"
            );
        }
        let result = Analyser::new().analyse(
            "package require :tcl::dict; ::tcl::dict::mine value",
            dialect,
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::W143)
        );
    }
}
