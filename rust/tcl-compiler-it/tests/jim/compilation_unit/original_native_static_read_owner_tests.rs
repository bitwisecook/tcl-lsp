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

fn reads(source: &str, per_item: bool) -> Vec<String> {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    let mut analyser = Analyser::new();
    let result = if per_item {
        analyser.analyse_per_item(source, "jim")
    } else {
        analyser.analyse(source, "jim")
    };
    result
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "W210")
        .map(|d| source[d.span.as_range()].to_owned())
        .collect()
}

#[test]
fn native_static_read_receipt_keeps_the_current_unit_allocation_owner() {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    let mut analyser = Analyser::new();
    for (source, expected) in [
        ("proc f {} {{n 0}} {return $n}; f", vec![]),
        ("proc f {} {{other 0}} {return $n}; f", vec!["$n"]),
        ("proc f {} {{n 0}} {return $n}; f", vec![]),
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
fn native_static_read_receipt_refuses_wrapped_and_relabelled_cells() {
    for source in [
        "proc f {} {{n 0}} {return $other}; f",
        "proc f {} {{n 0}} {proc g {} {return $n}; return [g]}; f",
        "proc f {} {{n 0}} {return $n}; f; puts $n",
    ] {
        assert!(!reads(source, false).is_empty(), "{source}");
    }
}

#[test]
fn direct_original_static_reads_keep_missing_and_other_body_diagnostics() {
    for (source, expected) in [
        ("proc f {} {{n 0}} {return $n}; f", vec![]),
        ("set n 1; proc f {} {n} {return $n}; f", vec![]),
        ("proc f {n} {{n 0}} {return $n}; f ARG", vec![]),
        ("proc f {} {{n 0}} {return $missing}; f", vec!["$missing"]),
        (
            "proc f {} {{n 0}} {return $n}; proc g {} {return $n}; g",
            vec!["$n"],
        ),
    ] {
        for per_item in [false, true] {
            assert_eq!(reads(source, per_item), expected, "{source}");
        }
    }
}
