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

/// The convertible-code/conversion-hint table the MCP `find-legacy` tool
/// reads comes straight from `tcl_cli`, so this also guards against the
/// shared table silently losing the conversion hint.
#[test]
fn reports_unbraced_expr_with_its_conversion_hint() {
    let source = "proc check {a b} {\n    set total [expr $a + $b]\n    return $total\n}\n";
    let result = find_legacy(&json!({ "source": source }));
    let total = result["total"].as_u64().expect("total is a number");
    assert!(total >= 1, "{result}");
    let patterns = result["patterns"].as_array().expect("patterns array");
    let w100 = patterns
        .iter()
        .find(|p| p["code"] == "W100")
        .unwrap_or_else(|| panic!("no W100 pattern in {result}"));
    assert_eq!(w100["conversion"], "Unbraced expr -> braced expr");
}
