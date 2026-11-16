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
fn source_recipe_reporting_preserves_obligations_and_cannot_recreate_issuer() {
    let source =
        "proc target {a b c} {}; interp alias {} selected {} target first second; selected";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.user_source_arity().is_some())
        .unwrap();
    let value = super::diagnostic_subject_data(diagnostic).unwrap();
    assert_eq!(value["subject"]["kind"], "userSourceArity");
    assert_eq!(value["subject"]["declaredMinimum"], 3);
    assert_eq!(value["subject"]["capturedArgumentCount"], 2);
    assert_ne!(
        value["subject"]["obligations"]
            .as_array()
            .unwrap()
            .as_slice(),
        [] as [serde_json::Value; 0]
    );
    let decoded = super::DiagnosticSubjectData::from_value(&value, "E002").unwrap();
    assert_eq!(decoded.to_value().unwrap(), value);
    assert!(super::DiagnosticSubjectData::from_value(&value, "W001").is_none());
    let mut presentation = diagnostic.clone();
    presentation.message = "different display label".to_owned();
    assert_eq!(super::diagnostic_subject_data(&presentation), Some(value));
}
