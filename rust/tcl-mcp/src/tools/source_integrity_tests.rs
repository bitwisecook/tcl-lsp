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

fn diagnostic_codes(result: &Value) -> Vec<&str> {
    result["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| d["code"].as_str())
        .collect()
}

#[test]
fn analyse_reports_bidi_controls_as_security_without_w108_duplication() {
    let source = "puts \"a\u{202e}b\"\n";
    let result = dispatch("analyze", &json!({ "source": source, "dialect": "tcl9.0" }))
        .expect("analyze tool");
    let diagnostics = result["diagnostics"].as_array().expect("diagnostics");
    let w305: Vec<_> = diagnostics.iter().filter(|d| d["code"] == "W305").collect();
    assert_eq!(w305.len(), 1, "{result}");
    assert_eq!(w305[0]["category"], "security");
    assert_eq!(w305[0]["severity"], "error");
    assert_eq!(w305[0].pointer("/range/start/character"), Some(&json!(7)));
    assert!(!diagnostic_codes(&result).contains(&"W108"), "{result}");
}

#[test]
fn validate_and_review_include_w305_in_the_security_group() {
    let args = json!({ "source": "# \u{202e}hidden\n", "dialect": "tcl9.0" });
    let validate = dispatch("validate", &args).expect("validate tool");
    assert_eq!(
        validate.pointer("/categories/security/items/0/code"),
        Some(&json!("W305")),
        "{validate}"
    );
    let review = dispatch("review", &args).expect("review tool");
    assert_eq!(review.pointer("/security/0/code"), Some(&json!("W305")));
}

#[test]
fn event_order_deduplicates_repeated_handlers() {
    let source = "when HTTP_REQUEST { return }\nwhen CLIENT_ACCEPTED {}\nwhen HTTP_REQUEST {}\n";
    let analysis = analyse(source, "f5-irules");
    let result = event_order_list(source, &analysis);
    let names: Vec<_> = result
        .iter()
        .filter_map(|row| row["name"].as_str())
        .collect();
    assert_eq!(names, ["CLIENT_ACCEPTED", "HTTP_REQUEST"]);
    assert_eq!(result[0]["index"], 1);
    assert_eq!(result[1]["index"], 2);
}
