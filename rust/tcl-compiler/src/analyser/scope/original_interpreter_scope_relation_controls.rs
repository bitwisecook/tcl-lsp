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
fn actual_source_body_subject_is_not_a_namespace_display_marker() {
    for (source, needle, expected) in [
        (
            "namespace eval @interp@user {set value 1}",
            "set value",
            SourceInterpreterScopeRelation::OriginalRoot,
        ),
        (
            "interp create sub; interp eval sub {set value 1}",
            "set value",
            SourceInterpreterScopeRelation::IsolatedBody,
        ),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
        let offset = u32::try_from(source.find(needle).unwrap()).unwrap();
        assert_eq!(
            analysis.source_interpreter_scope_relation(&image, config, offset),
            Some(expected)
        );
        let foreign = tcl_lexer::SourceImage::document(&format!("{source} changed"));
        assert!(
            analysis
                .source_interpreter_scope_relation(&foreign, config, offset)
                .is_none()
        );
        let mut changed = config;
        changed.strict_quoting = !changed.strict_quoting;
        assert!(
            analysis
                .source_interpreter_scope_relation(&image, changed, offset)
                .is_none()
        );
    }
}
