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

use tcl_compiler::analyser::{Analyser, AnalysisResult};

fn analyse(source: &str) -> AnalysisResult {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    Analyser::new().analyse(source, "jimtcl")
}

#[test]
fn stock_source_effects_need_the_actual_analysis_owner_and_complete_publications() {
    let source = "class Point {x 0}\nproc {Point move} {dx} {return $dx}\n";
    for result in [analyse(source), analyse(source)] {
        let class = result
            .all_classes
            .values()
            .find(|c| c.name == "Point")
            .unwrap();
        assert!(class.methods.contains_key("move"));
        assert_eq!(
            &source[tcl_lexer::word_span_at(source, class.methods["move"].body_span).as_range()],
            "{return $dx}"
        );
    }
}

#[test]
fn stock_source_effects_keep_unknown_operations_and_changed_dependencies_terminal() {
    for source in [
        "class Point {x 0}\nputs OPAQUE\nproc {Point move} {dx} {return $dx}\n",
        "proc alias {args} {return CLOSED}\nclass Point {x 0}\nproc {Point move} {dx} {return $dx}\n",
    ] {
        let result = analyse(source);
        assert!(
            result
                .all_classes
                .values()
                .all(|c| !c.methods.contains_key("move")),
            "{source}"
        );
        assert!(result.all_procs.values().any(|p| p.name == "Point move"));
    }
}
