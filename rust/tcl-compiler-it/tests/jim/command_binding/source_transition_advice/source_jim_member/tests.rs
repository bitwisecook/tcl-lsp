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
fn own_jim_member_keeps_before_and_after_factory_custody() {
    for source in [
        "class Point {x 0}\nproc {Point move} {dx} {return $dx}\n",
        "proc {Point move} {dx} {return $dx}\nclass Point {x 0}\n",
    ] {
        let result = analyse(source);
        let class = result
            .all_classes
            .values()
            .find(|c| c.name == "Point")
            .unwrap();
        let method = &class.methods["move"];
        assert_eq!(
            &source[tcl_lexer::word_span_at(source, method.body_span).as_range()],
            "{return $dx}"
        );
        assert!(!result.all_variables.contains_key("Point move::x"));
        assert!(!result.all_variables.contains_key("Point move::self"));
    }
}

#[test]
fn own_jim_member_refuses_foreign_and_corrupt_recipes() {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    let mut analyser = Analyser::new();
    for (source, expected, forbidden) in [
        (
            "class Point {x 0}\nproc {Point move} {dx} {return $dx}\n",
            "move",
            "stop",
        ),
        (
            "# different source\nclass Point {x 0}\nproc {Point stop} {dx} {return $dx}\n",
            "stop",
            "move",
        ),
        (
            "class Point {x 0}\nproc {Point move} {dx} {return $dx}\n",
            "move",
            "stop",
        ),
    ] {
        let result = analyser.analyse(source, "jim");
        let class = result
            .all_classes
            .values()
            .find(|c| c.name == "Point")
            .unwrap();
        assert!(class.methods.contains_key(expected));
        assert!(!class.methods.contains_key(forbidden));
        assert_eq!(
            &source[tcl_lexer::word_span_at(source, class.methods[expected].body_span).as_range()],
            "{return $dx}"
        );
    }
}

#[test]
fn own_jim_member_refuses_rebound_class_and_replaced_initialisers() {
    for source in [
        "class Point {x 0}\nrename Point OldPoint\nproc {Point move} {dx} {return $dx}\n",
        "class Point {x 0}\nproc Point {} {return OTHER}\nproc {Point move} {dx} {return $dx}\n",
        "class Point {x 0}\nalias Alias Point\nproc {Alias move} {dx} {return $dx}\n",
        "class Point {x 0}\nalias Point puts\nproc {Point move} {dx} {return $dx}\n",
        "class Point {x 0}\nproc {{Point} move} {dx} {return $dx}\n",
        "class Point {x 0}\nputs OPAQUE\nproc {Point move} {dx} {return $dx}\n",
    ] {
        let result = analyse(source);
        assert!(
            result
                .all_classes
                .values()
                .all(|c| !c.methods.contains_key("move")),
            "{source}"
        );
    }
    let result = analyse("proc {Point new} {} {return OLD}\nclass Point {x 0}\n");
    assert!(
        result
            .all_classes
            .values()
            .all(|c| !c.methods.contains_key("new"))
    );
}
