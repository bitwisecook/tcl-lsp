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
fn selected_original_method_body_requires_real_return_schema_and_static_operand() {
    // tclsh8.6.18/9.0.4: `[my literal] action` where `literal` returns VALUE
    // -> invalid command name "VALUE": an unknown-command finding naming
    // VALUE, not W307. A `$dynamic` return or a redefined `return` keeps the
    // abstention: nothing names VALUE.
    for (prefix, value, expected) in [
        ("", "VALUE", true),
        ("", "$dynamic", false),
        ("proc return args {}\n", "VALUE", false),
    ] {
        let source = format!(
            "{prefix}oo::class create C {{method literal {{}} {{return {value}}}; method use {{}} {{[my literal] action}}}}"
        );
        let result = Analyser::new().analyse(&source, "tcl8.6");
        let names_value = result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123 && d.message.contains("'VALUE'"));
        assert_eq!(names_value, expected, "{source}: {:?}", result.diagnostics);
        if expected {
            assert!(
                !result.diagnostics.iter().any(|d| d.code == DiagCode::W307),
                "{source}: {:?}",
                result.diagnostics
            );
        }
    }
}
