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
fn logical_inline_uses_shared_actual_word_and_expression_constructors() {
    for (source, call, suffix) in [
        (
            "proc p {x} {puts pre-${x}-post}\np VALUE",
            "p VALUE",
            "puts pre-VALUE-post",
        ),
        ("proc p {x} {expr {$x + 1}}\np 7", "p 7", "expr {7 + 1}"),
        (
            "proc p {x} {expr {{${x}} eq \"$x\"}}\np 7",
            "p 7",
            "expr {{${x}} eq \"7\"}",
        ),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let action = inline_proc(
            source,
            u32::try_from(source.rfind(call).unwrap()).unwrap(),
            &analysis,
            analysis.resolved_registry().unwrap(),
        )
        .unwrap();
        assert!(action.disabled.is_none(), "{:?}", action.disabled);
        assert!(action.try_apply(source).unwrap().ends_with(suffix));
    }
}
