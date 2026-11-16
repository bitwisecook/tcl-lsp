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
fn original_pack_links_keep_document_marks_distinct_from_include_body_content() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let profile = crate::profile_for_dialect("spectcl");
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..tcl_lexer::LexerConfig::for_profile(Some(profile))
    };
    let source =
        "\u{feff}speclib lib 1 {\u{feff}include hidden.tclspec\n    include visible.tclspec}";
    let mut analysis = super::tests::path_analysis(source, profile, config);
    let context = LinkContext {
        script_path: Some("/actual/pack.tclspec"),
        workspace_root: Some("/wrong"),
        ..LinkContext::default()
    };
    let links = document_links_from_analysis(source, &analysis, &context);
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///actual/visible.tclspec");
    let start = u32::try_from(source.find("visible.tclspec").unwrap()).unwrap();
    let line_index = LineIndex::new(source);
    let position = line_index.position_at_utf16(start, source);
    assert_eq!(
        (links[0].start_line, links[0].start_character),
        (position.line, position.character.get())
    );
    let end = line_index.position_at_utf16(
        start + u32::try_from("visible.tclspec".len()).unwrap(),
        source,
    );
    assert_eq!(
        (links[0].end_line, links[0].end_character),
        (end.line, end.character.get())
    );
    assert_eq!(
        document_links_from_analysis(&format!("# changed\n{source}"), &analysis, &context),
        [] as [crate::document_links::DocumentLink; 0]
    );
    analysis.resolved_input = None;
    assert_eq!(
        document_links_from_analysis(source, &analysis, &context),
        [] as [crate::document_links::DocumentLink; 0]
    );
}
#[test]
fn original_link_anchors_parse_actual_substitution_content_without_file_prologue_rules() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..tcl_lexer::LexerConfig::for_profile(Some(profile))
    };
    let source = "\u{feff}source [\u{feff}# ordinary arguments leaf.tcl]";
    let commands = segment_commands_with_offset_and_config(source, 0, config);
    let token = commands[0].argv[1];
    let start = u32::try_from(source.find("leaf.tcl").unwrap()).unwrap();
    assert_eq!(
        link_anchor_with_config(source, config, &token),
        Some((start, start + u32::try_from("leaf.tcl".len()).unwrap()))
    );
    let analysis = super::tests::path_analysis(source, profile, config);
    // Geometry cannot resolve the unknown marked command as a file producer.
    assert_eq!(
        document_links_from_analysis(
            source,
            &analysis,
            &LinkContext {
                workspace_root: Some("/actual"),
                ..LinkContext::default()
            }
        ),
        [] as [crate::document_links::DocumentLink; 0]
    );
    for source in [
        "source [# leaf.tcl]",
        "source [file join lib $name]",
        "source [pwd]",
    ] {
        let commands = segment_commands_with_offset_and_config(source, 0, config);
        assert!(link_anchor_with_config(source, config, &commands[0].argv[1]).is_none());
    }
}
