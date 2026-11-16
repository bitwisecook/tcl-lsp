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

#[test]
fn unknown_helper_reports_unknown_lookup_without_reconstructing_selection() {
    let source = "::tcl::clock::GetSystemTimeZone";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.private_tcl_namespace().is_some())
        .unwrap();
    let value = super::diagnostic_subject_data(diagnostic).unwrap();
    assert_eq!(value["subject"]["kind"], "privateTclNamespace");
    assert_eq!(value["subject"]["selectedSourceDescriptor"], false);
    assert!(
        value["subject"]["obligations"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("unavailableActualLookup"))
    );
    assert!(super::DiagnosticSubjectData::from_value(&value, "W143").is_some());
    assert!(super::DiagnosticSubjectData::from_value(&value, "E002").is_none());
}
