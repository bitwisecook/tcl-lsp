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

use tcl_compiler::analyser::Analyser;

fn analyse(source: &str, per_item: bool) -> tcl_compiler::analyser::AnalysisResult {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    let mut analyser = Analyser::new();
    if per_item {
        analyser.analyse_per_item(source, "jim")
    } else {
        analyser.analyse(source, "jim")
    }
}

fn warnings(source: &str) -> Vec<String> {
    analyse(source, false)
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "W210")
        .map(|d| source[d.span.as_range()].to_owned())
        .collect()
}

fn assert_reads(source: &str, expected: &[&str]) {
    for per_item in [false, true] {
        let result = analyse(source, per_item);
        let found = result
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str() == "W210")
            .map(|d| &source[d.span.as_range()])
            .collect::<Vec<_>>();
        assert_eq!(found, expected, "per_item={per_item}: {source}");
    }
}

#[test]
fn original_nested_named_static_read_keeps_original_positive_expectations() {
    assert_reads("proc counter {} {{n 0}} { return [incr n] }; counter", &[]);
}

#[test]
fn original_nested_named_read_refuses_alias_rebinding_callee_and_opaque_children() {
    for source in [
        "proc counter {} {{n 0}} {upvar 1 other n; return [incr n]}; counter",
        "proc incr {name} {return 1}; proc counter {} {{n 0}} {return [incr n]}; counter",
        "proc child {} {incr n}; proc counter {} {{n 0}} {return [child]}; counter",
        "proc counter {} {{n 0}} {return [unknown_command [incr n]]}; counter",
        "interp alias {} readN {} incr n; proc counter {} {{n 0}} {return [readN n]}; counter",
        "proc counter {} {{n 0}} {return [incr n(x) n]}; counter",
        "proc counter {} {{n 0}} {return [set ::outside 1][incr n]}; counter",
    ] {
        let source = source.replace("{{n 0}} {", "{{n 0}} {set probe $missing; ");
        assert!(
            warnings(&source).iter().any(|name| name == "$missing"),
            "{source}"
        );
    }
}

#[test]
fn original_nested_named_read_refuses_equal_foreign_and_stale_source_owners() {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    let mut analyser = Analyser::new();
    for (source, expected) in [
        (
            "proc counter {} {{n 0}} {return [list $n]}; counter",
            vec![],
        ),
        (
            "proc counter {} {{m 0}} {return [list $n]}; counter",
            vec!["$n"],
        ),
        (
            "proc counter {} {{n 0}} {return [list $n]}; counter",
            vec![],
        ),
    ] {
        let result = analyser.analyse(source, "jim");
        let found = result
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str() == "W210")
            .map(|d| &source[d.span.as_range()])
            .collect::<Vec<_>>();
        assert_eq!(found, expected, "{source}");
    }
}

#[test]
fn original_nested_read_rejects_a_child_with_a_foreign_frame() {
    assert_reads(
        "proc counter {} {{n 0}} {proc child {} {return $n}; return [child]}; counter",
        &["$n"],
    );
}

#[test]
fn original_child_body_keeps_its_issued_frame_distinct_from_header_layout() {
    assert_reads(
        "proc counter {n} {{saved 0}} {return [list $n $saved]}; counter ARG",
        &[],
    );
}

#[test]
fn original_child_body_frame_refuses_missing_reissued_and_foreign_rows() {
    let original = "proc counter {} {{n 0}} {return [list $n]}; counter";
    for source in [
        original,
        "# another source\nproc counter {} {{n 0}} {return [list $n]}; counter",
        original,
    ] {
        assert_reads(source, &[]);
    }
    assert_reads(
        "proc counter {} {{n 0}} {return [list $missing]}; counter",
        &["$missing"],
    );
}

#[test]
fn original_child_body_frame_refuses_different_allocation_and_missing_formals() {
    for source in [
        "proc counter {n} {{saved 0}} {return [list $n $saved]}; proc counter {} {{saved 0}} {return [list $n $saved]}; counter",
        "proc f {n} {{saved 0}} {return [list $n $saved]}; proc g {} {{saved 0}} {return [list $n $saved]}; g",
    ] {
        assert_reads(source, &["$n"]);
    }
}

#[test]
fn known_child_projection_uses_live_producer_and_separate_body_scope() {
    assert_reads(
        "proc counter {} {{n 0}} {return [incr n]}; counter; puts $n",
        &["$n"],
    );
}

#[test]
fn known_child_projection_refuses_consistently_replaced_producer_fields() {
    assert_reads(
        "set body [list return $n]; proc counter {} {{n 0}} $body; counter",
        &["$n"],
    );
}

#[test]
fn known_child_projection_refuses_foreign_and_retired_source_owner() {
    assert_reads(
        "proc counter {} {{n 0}} {return [list $n]}; rename counter {}; proc counter {} {return [list $n]}; counter",
        &["$n"],
    );
}
