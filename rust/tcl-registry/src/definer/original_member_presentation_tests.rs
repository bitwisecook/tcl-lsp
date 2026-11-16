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

use super::{SourceDefinitionMemberBodyPresentation, TCLOO_GRAMMAR};
use crate::{InvocationArguments, InvocationWord};
use tcl_dialect::model::{Family, SurfaceQuery};

#[test]
fn original_member_presentation_follows_callable_body_slot_and_selected_layout() {
    // naming.editor.original-source-formatting
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
    let query = Some(SurfaceQuery::core(Family::Tcl, "9.0"));
    for (keyword, arguments, ordinal) in [
        ("method", vec!["m", "", "puts α"], 2),
        ("method", vec!["m", "-private", "", "puts α"], 3),
        ("private", vec!["method", "m", "", "puts α"], 3),
        ("constructor", vec!["", "puts α"], 1),
        ("destructor", vec!["puts α"], 0),
    ] {
        let bodies = TCLOO_GRAMMAR
            .source_member_script_arguments_in(
                keyword.as_bytes(),
                InvocationArguments::literals(&arguments),
                query,
            )
            .unwrap();
        assert_eq!(bodies.len(), 1, "{keyword} {arguments:?}");
        assert_eq!(bodies[0].argument(), ordinal);
        assert_eq!(
            bodies[0].presentation(),
            SourceDefinitionMemberBodyPresentation::CallableBlock
        );
        assert!(!bodies[0].retains_definition_grammar());
    }
    let wrapper = TCLOO_GRAMMAR
        .source_member_script_arguments_in(
            b"private",
            InvocationArguments::literals(&["method m {} {puts α}"]),
            query,
        )
        .unwrap();
    assert_eq!(
        wrapper[0].presentation(),
        SourceDefinitionMemberBodyPresentation::OrdinaryBlock
    );
    assert!(wrapper[0].retains_definition_grammar());
    let unknown = [
        InvocationWord::Literal("m"),
        InvocationWord::Dynamic,
        InvocationWord::Literal(""),
        InvocationWord::Literal("puts α"),
    ];
    assert!(
        TCLOO_GRAMMAR
            .source_member_script_arguments_in(
                b"method",
                InvocationArguments::structured(&unknown),
                query,
            )
            .is_none(),
        "unknown optional selector cannot choose a body presentation"
    );
    assert!(
        TCLOO_GRAMMAR
            .source_member_script_arguments_in(
                b"method",
                InvocationArguments::literals(&["m", "-private", "", "puts α"]),
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .unwrap()
            .is_empty(),
        "unavailable option cannot shift the selected body"
    );
}
