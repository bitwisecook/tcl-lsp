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
use tcl_core_types::DiagCode;

#[test]
fn original_static_alternatives_withdraw_only_their_synthetic_local_absence() {
    tcl_spectcl::core_surfaces::ensure();
    for (source, expected) in [
        ("proc f {} {{n 0}} {return [incr n]}; f", vec![]),
        ("set n 1; proc f {} {n} {return $n}; f", vec![]),
        ("set n 1; proc f {} {&n} {return $n}; f", vec![]),
        ("proc f {} {{n 0}} {not_a_command_anywhere $n}; f", vec![]),
        (
            "proc f {} {{n 0}} {unset -nocomplain n; return $n}; f",
            vec![],
        ),
        ("proc f {n} {{n 0}} {return $n}; f ARG", vec![]),
        ("proc f {} {{n 0}} {return $missing}; f", vec!["$missing"]),
        (
            "proc f {} {{n 0}} {return $n}; proc g {} {return $n}; g",
            vec!["$n"],
        ),
    ] {
        for per_item in [false, true] {
            let mut analyser = Analyser::new();
            let result = if per_item {
                analyser.analyse_per_item(source, "jim")
            } else {
                analyser.analyse(source, "jim")
            };
            let actual = result
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::W210)
                .map(|diagnostic| &source[diagnostic.span.as_range()])
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "per_item={per_item}: {source}");
        }
    }
}
