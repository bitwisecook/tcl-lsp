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
fn event_boundary_keeps_full_input_and_nested_bom_purposes_distinct() {
    // naming.source.retained-explorer-consumers
    // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
    // Parser/registry boundary control; no executed BIG-IP handler claim.
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let parent = LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..LexerConfig::from_grammar(tcl_dialect::DialectProfile::irules().grammar)
    };
    let source = "\u{feff}when HTTP_REQUEST {}";
    let whole = top_level_when_handlers_with_source_config_and_head_resolver(
        source,
        registry,
        parent,
        SourceSegmentationPurpose::WholeInput,
        &WrittenHeadResolver,
    );
    assert_eq!(whole.len(), 1);
    assert_eq!(whole[0].event, "HTTP_REQUEST");
    assert_eq!(
        whole[0].event_span.start(),
        u32::try_from("\u{feff}when ".len()).unwrap()
    );
    assert_eq!(
        top_level_when_handlers_with_source_config_and_head_resolver(
            source,
            registry,
            parent,
            SourceSegmentationPurpose::NestedScript,
            &WrittenHeadResolver,
        )
        .len(),
        0
    );
    assert_eq!(
        top_level_when_handlers_with_source_config_and_head_resolver(
            source,
            registry,
            parent.nested(),
            SourceSegmentationPurpose::WholeInput,
            &WrittenHeadResolver,
        )
        .len(),
        0
    );
    assert_eq!(parent.leading_bom, tcl_lexer::LeadingBom::Skip);

    let source = "\u{feff}priority 730\nwhen HTTP_REQUEST {}";
    let whole = top_level_when_handlers_with_source_config_and_head_resolver(
        source,
        registry,
        parent,
        SourceSegmentationPurpose::WholeInput,
        &WrittenHeadResolver,
    );
    let nested = top_level_when_handlers_with_source_config_and_head_resolver(
        source,
        registry,
        parent,
        SourceSegmentationPurpose::NestedScript,
        &WrittenHeadResolver,
    );
    assert_eq!(whole.len(), 1);
    assert_eq!(nested.len(), 1);
    assert_eq!(whole[0].effective_priority, 730);
    assert_eq!(nested[0].effective_priority, 500);
}
