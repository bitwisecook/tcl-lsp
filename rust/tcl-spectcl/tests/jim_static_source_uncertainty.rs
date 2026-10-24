// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use tcl_compiler::analyser::Analyser;
use tcl_core_types::DiagCode;

#[test]
fn original_jim_statics_withhold_absence_without_borrowing_other_locals() {
    tcl_spectcl::core_surfaces::ensure();
    for (source, expected) in [
        ("proc f {} {{n 0}} {set anchor 1; return $n}", vec![]),
        ("set n 1; proc f {} {n} {set anchor 1; return $n}", vec![]),
        ("set n 1; proc f {} {&n} {set anchor 1; return $n}", vec![]),
        (
            "proc f {} {{n 0}} {set anchor 1; return $missing}",
            vec!["$missing"],
        ),
        (
            "proc f {} {{n 0}} {set anchor 1; unset -nocomplain n; return $n}",
            vec![],
        ),
        (
            "proc f {} {{n 0}} {set anchor 1; return $n}; proc g {} {set anchor 1; return $n}",
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
            let warnings = result
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::W210)
                .map(|diagnostic| &source[diagnostic.span.as_range()])
                .collect::<Vec<_>>();
            assert_eq!(warnings, expected, "per_item={per_item}: {source}");
        }
    }
}
