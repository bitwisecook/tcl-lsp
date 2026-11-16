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

/// TMM accepts the `}{` ghost separator (`if {expr}{body}`); stock Tcl
/// does not, so the two dialects format these bytes differently.
const GHOST_SEPARATOR_IRULE: &str =
    "when HTTP_REQUEST {\n    if { 1 }{\n        pool p\n    }\n}\n";

#[test]
fn the_format_tool_formats_under_the_requested_dialect() {
    // `format_source` must project the tool's resolved dialect onto the
    // formatter config, not start from `FormatterConfig::default()`,
    // which would tokenise an iRule with the Tcl 9 lexer.
    for (dialect, separated) in [("f5-irules", true), ("irules", true), ("tcl9.0", false)] {
        let result = format_source(&json!({
            "source": GHOST_SEPARATOR_IRULE,
            "dialect": dialect,
        }));
        let formatted = result["formatted"].as_str().expect("formatted string");
        assert_eq!(
            formatted.contains("} {"),
            separated,
            "{dialect}: {formatted}"
        );
        assert_eq!(
            formatted.contains("}{"),
            !separated,
            "{dialect}: {formatted}"
        );
    }
}
