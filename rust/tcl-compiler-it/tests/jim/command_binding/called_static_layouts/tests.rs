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
    Analyser::new().analyse(source, "jim")
}

#[test]
fn called_static_body_keeps_a_separate_original_declaration_layout() {
    for source in [
        "proc f {} {{n 0}} {return $n}; f",
        "proc f {} {{n 0}} {return $n}",
    ] {
        let result = analyse(source);
        let procedure = result.all_procs.values().find(|p| p.name == "f").unwrap();
        assert_eq!(
            &source[tcl_lexer::word_span_at(source, procedure.body_span).as_range()],
            "{return $n}"
        );
        assert!(result.diagnostics.iter().all(|d| d.code.as_str() != "W210"));
    }
}

#[test]
fn called_static_layout_collection_does_not_publish_results_or_entered_inventories() {
    for source in [
        "proc f {} {{n 0}} {set body_only 7; return $n}; f; puts $body_only",
        "proc f {} {{n 0}} {set ::body_only 7; return $n}; puts $::body_only",
    ] {
        let result = analyse(source);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code.as_str() == "W210"
                    && source[d.span.as_range()].contains("body_only")),
            "{source}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn static_layout_collection_does_not_reinterpret_other_callable_purposes() {
    for (source, body) in [
        ("proc f {} {return 7}; f", "{return 7}"),
        ("proc f {} {{n 0}} {return $n}", "{return $n}"),
        (
            "proc f {} {missing} {return $missing}; f",
            "{return $missing}",
        ),
    ] {
        let result = analyse(source);
        let procedure = result.all_procs.values().find(|p| p.name == "f").unwrap();
        assert_eq!(
            &source[tcl_lexer::word_span_at(source, procedure.body_span).as_range()],
            body
        );
    }
}
