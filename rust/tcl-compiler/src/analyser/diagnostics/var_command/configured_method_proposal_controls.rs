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
fn actual_method_word_and_parent_argv_own_configured_proposal() {
    // tclsh8.6.18/9.0.4: `[Dog new] {bakr}` -> unknown method "bakr": must be
    // bark or destroy. The proposal replaces the whole written method word.
    let source = "oo::class create Dog {method bark {} {return {}}}\n[Dog new] {bakr}";
    let result = Analyser::new().analyse(source, "tcl");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W308)
        .unwrap_or_else(|| panic!("W308: {:?}", result.diagnostics));
    let fix = diagnostic
        .fixes
        .iter()
        .find(|fix| fix.new_text == "bark")
        .expect("a did-you-mean fix");
    assert_eq!(source.get(fix.span.as_range()), Some("{bakr}"));
    let source = "oo::class create Dog {method bark {} {return {}}}\n[Dog new] bakr";
    let result = Analyser::new().analyse_per_item(source, "tcl");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W308)
        .unwrap();
    assert_eq!(diagnostic.fixes[0].new_text, "bark");
    assert_eq!(diagnostic.fixes[0].span, diagnostic.span);
}
