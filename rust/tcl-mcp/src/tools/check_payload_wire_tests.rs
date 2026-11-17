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

//! `check_payload` survives the MCP diagnostic object.

use super::diagnostics::diag_to_json;
use super::*;
use tcl_lsp_core::check_payload::{CheckPayloadData, family_fixtures};
use tcl_lsp_core::diagnostic_policy::{Finding, Outcome, Report};

/// Implementation contract: every compiler-check family's payload
/// survives the MCP diagnostic object, as JSON text, without the message.
#[test]
fn every_check_family_payload_survives_the_tool_object() {
    let fixtures = family_fixtures();
    let source = "x".repeat(64);
    let sm = SourceMap::new(&source);
    let report = Report::new(
        fixtures
            .iter()
            .cloned()
            .map(|diagnostic| {
                (
                    Finding::from(diagnostic),
                    Outcome::Shown {
                        severity: tcl_compiler::analyser::Severity::Warning,
                        tag: None,
                    },
                )
            })
            .collect(),
    );
    let mut seen = 0;
    for shown in report.shown() {
        let text = serde_json::to_string(&diag_to_json(&shown, &sm)).unwrap();
        let parsed: Value = serde_json::from_str(&text).unwrap();
        let code = parsed["code"].as_str().unwrap();
        let decoded = CheckPayloadData::from_value(&parsed["data"], code)
            .unwrap_or_else(|| panic!("{code}: {text}"));
        let original = fixtures
            .iter()
            .find(|diagnostic| diagnostic.code.as_str() == code)
            .unwrap();
        assert!(decoded.describes(&original.payload), "{text}");
        seen += 1;
    }
    assert_eq!(seen, fixtures.len());
}
