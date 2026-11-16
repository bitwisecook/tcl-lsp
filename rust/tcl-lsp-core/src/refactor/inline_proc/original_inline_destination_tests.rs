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
fn original_inline_refuses_a_body_builtin_shadowed_at_the_call_destination() {
    // Implementation contract: naming.refactor.original-inline-destination-command
    // docs/design/analysis/name-resolution-proofs/refactor-original-inline-destination-command.md
    let source =
        "proc p {x} {puts $x}\nnamespace eval Other {\nproc puts {x} {return SHADOW}\np VALUE\n}\n";
    let mut analyser = tcl_compiler::analyser::Analyser::new();
    let analysis = analyser.analyse(source, "tcl8.6").clone();
    let cursor = u32::try_from(source.find("p VALUE").unwrap()).unwrap();
    let result = inline_proc(
        source,
        cursor,
        &analysis,
        analysis.resolved_registry().unwrap(),
    )
    .expect("actual original procedure call");
    assert!(result.disabled.is_some());
    assert_eq!(result.edits, [] as [crate::refactor::RefactorEdit; 0]);
}
