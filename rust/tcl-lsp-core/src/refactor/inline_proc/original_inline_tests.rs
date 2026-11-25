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

fn action(source: &str, call: &str, clear_reports: bool) -> Option<Refactoring> {
    let mut analyser = Analyser::new();
    let mut analysis = analyser.analyse(source, "tcl8.6").clone();
    if clear_reports {
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
    }
    let cursor = u32::try_from(source.rfind(call).unwrap()).unwrap();
    inline_proc(
        source,
        cursor,
        &analysis,
        analysis.resolved_registry().unwrap(),
    )
}

#[test]
fn original_inline_uses_opaque_allocation_formals_and_absolute_body_with_reports_cleared() {
    // Implementation contract: naming.refactor.original-procedure-inline-binding
    // docs/design/analysis/name-resolution-proofs/refactor-original-procedure-inline-binding.md
    let source = r"proc p\uD800 {x} {puts $x}
 p\uD800 VALUE
";
    let result = action(source, r"p\uD800 VALUE", true).expect("actual original call");
    assert!(result.disabled.is_none(), "{:?}", result.disabled);
    assert!(result.try_apply(source).unwrap().ends_with(" puts VALUE\n"));
}

#[test]
fn original_inline_keeps_inert_braced_data_and_refuses_erased_reads_or_free_cells() {
    // Implementation contract: naming.refactor.original-procedure-inline-binding
    // docs/design/analysis/name-resolution-proofs/refactor-original-procedure-inline-binding.md
    let literal = "proc p {x} {puts {$x}}\np VALUE\n";
    let result = action(literal, "p VALUE", false).unwrap();
    assert!(result.disabled.is_none(), "{:?}", result.disabled);
    assert!(result.try_apply(literal).unwrap().ends_with("puts {$x}\n"));
    for source in [
        "proc p {unused} {puts SAFE}\nset value VALUE\np $value\n",
        "proc p {x} {puts $outside}\np VALUE\n",
    ] {
        let call = if source.contains("p $value") {
            "p $value"
        } else {
            "p VALUE"
        };
        let result = action(source, call, false).expect("actual original call");
        assert!(result.disabled.is_some());
        assert_eq!(result.edits, [] as [crate::refactor::RefactorEdit; 0]);
    }
}

#[test]
fn original_inline_bindings_keep_alias_prefix_expansion_defaults_and_arity() {
    // Implementation contract: naming.refactor.original-procedure-inline-binding
    // docs/design/analysis/name-resolution-proofs/refactor-original-procedure-inline-binding.md
    for (source, call, expected) in [
        (
            "proc p {x {y 2}} {expr {$x + $y}}\ninterp alias {} wrapped {} p 3\nwrapped\n",
            "wrapped\n",
            "expr {3 + 2}",
        ),
        (
            "proc p {x y} {expr {$x + $y}}\np {*}{3 4}\n",
            "p {*}",
            "expr {3 + 4}",
        ),
    ] {
        let result = action(source, call, true).expect("actual original call");
        assert!(result.disabled.is_none(), "{:?}", result.disabled);
        assert!(result.try_apply(source).unwrap().contains(expected));
    }
    let arity = "proc p {x} {puts $x}\np 1 2\n";
    let result = action(arity, "p 1 2", true).unwrap();
    assert!(result.disabled.is_some());
    assert_eq!(result.edits, [] as [crate::refactor::RefactorEdit; 0]);
}
