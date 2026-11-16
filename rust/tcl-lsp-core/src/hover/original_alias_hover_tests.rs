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
fn alias_advice_cannot_claim_a_same_named_data_argument() {
    let source = "interp alias {} shortcut {} missing; list shortcut";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let offset = u32::try_from(source.rfind("shortcut").unwrap()).unwrap();
    assert!(matches!(
        alias_hover_at(source, &analysis, offset, "shortcut"),
        std::ops::ControlFlow::Continue(())
    ));
}

#[test]
fn reporting_alias_target_does_not_replace_missing_original_recipe() {
    let source = "interp alias {} shortcut {} missing; shortcut";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let offset = u32::try_from(source.rfind("shortcut").unwrap()).unwrap();
    assert!(!analysis.allows_lexical_declaration_advice());
    analysis
        .command_aliases
        .get_mut("::shortcut")
        .unwrap()
        .extras
        .push("forged-display".to_owned());
    let answer = alias_hover_at(source, &analysis, offset, "shortcut");
    assert!(
        !matches!(answer, std::ops::ControlFlow::Break(Some(ref text)) if text.contains("forged-display"))
    );
    for invocation in &mut analysis.command_invocations {
        invocation.original_lookup = None;
    }
    assert!(!matches!(
        alias_hover_at(source, &analysis, offset, "shortcut"),
        std::ops::ControlFlow::Break(Some(_))
    ));
}
