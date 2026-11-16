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
fn selected_case_bodies_and_command_roles_keep_the_actual_source_owners() {
    let source = "switch $n {1 {set x 1} default {set x 2}}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let walk = super::super::FrameWalk::new(source, &analysis).unwrap();
    let commands = walk.segment_document(source);
    let selection =
        LogicalExtractionSourceSelection::capture(source, &analysis, &[&commands[0]], &walk)
            .unwrap();
    assert_eq!(selection.bodies.len(), 2);
    assert!(
        selection
            .bodies
            .iter()
            .all(|body| body.matches_source(&selection.image, selection.input.lexer_config()))
    );
    let roles =
        classify_selected_variables(source, &[&commands[0]], &walk, Some(&selection)).unwrap();
    assert!(roles.written.contains_key("x"));
    assert!(
        selection
            .block_span(&format!("{source} "), &analysis)
            .is_none()
    );
    let mut foreign = commands[0].clone();
    foreign.all_tokens.clear();
    assert!(selection.words_for(&foreign).is_none());
}
