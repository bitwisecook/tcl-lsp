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
use tcl_compiler::analyser::Analyser;

#[test]
fn parameter_positions_use_whole_grouped_literal_and_computed_words() {
    for (source, needle) in [
        (
            "proc {p} {mé {option {default with spaces}}} {return 1}",
            "default",
        ),
        ("proc p \"mé option\" {return 1}", "option"),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let at = u32::try_from(source.find(needle).unwrap()).unwrap();
        assert_eq!(
            parameter_list_position_at(&analysis, source, at),
            ParamListPosition::LiteralData,
            "{source}"
        );
        let body = u32::try_from(source.find("return").unwrap()).unwrap();
        assert_eq!(
            parameter_list_position_at(&analysis, source, body),
            ParamListPosition::Outside
        );
    }
    let source = "proc p \"[list {name with spaces}]\" {return 1}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let at = u32::try_from(source.find("name").unwrap()).unwrap();
    assert_eq!(
        parameter_list_position_at(&analysis, source, at),
        ParamListPosition::Computed
    );
}

#[test]
fn parameter_positions_refuse_foreign_source_and_configuration() {
    let source = "proc p {mé {option default}} {return 1}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let at = u32::try_from(source.find("option").unwrap()).unwrap();
    assert_eq!(
        parameter_list_position_at(&analysis, source, at),
        ParamListPosition::LiteralData
    );
    assert_eq!(
        parameter_list_position_at(&analysis, &source.replace("default", "changed"), at),
        ParamListPosition::Outside
    );
    analysis.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    assert_eq!(
        parameter_list_position_at(&analysis, source, at),
        ParamListPosition::Outside
    );
}
