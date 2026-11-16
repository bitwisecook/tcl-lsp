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
fn logical_path_links_use_the_original_prefix_before_invocation_effects() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = LinkContext {
        workspace_root: Some("/workspace"),
        ..LinkContext::default()
    };
    let source = "set dir /ROOT; source [file join $dir leaf.tcl]; set dir /LATER";
    let analysis = super::tests::path_analysis(
        source,
        profile,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let links = document_links_from_analysis(source, &analysis, &context);
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///ROOT/leaf.tcl");
    for source in [
        "set dir /ROOT; opaque; source [file join $dir leaf.tcl]",
        "set dir /ROOT; proc source args {}; source [file join $dir leaf.tcl]",
    ] {
        assert_eq!(
            document_links_in_context(source, profile, &context),
            [] as [crate::document_links::DocumentLink; 0]
        );
    }
    assert_eq!(
        document_links_from_analysis(&format!("# changed\n{source}"), &analysis, &context,),
        [] as [crate::document_links::DocumentLink; 0]
    );
    let native = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    assert_eq!(
        document_links_in_context(source, native, &context),
        [] as [crate::document_links::DocumentLink; 0]
    );
}

#[test]
fn logical_path_link_anchors_use_authentic_static_tail_words() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = LinkContext {
        workspace_root: Some("/workspace"),
        ..LinkContext::default()
    };
    let source =
        "interp alias {} build {} file join {/ROOT with space}; source [build {literal$é.tcl}]";
    let links = document_links_in_context(source, profile, &context);
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(
        links[0].target,
        file_uri_for_path("/ROOT with space/literal$é.tcl")
    );
    let line_index = LineIndex::new(source);
    let start = u32::try_from(source.find("literal$é.tcl").unwrap()).unwrap();
    let end = start + u32::try_from("literal$é.tcl".len()).unwrap();
    assert_eq!(
        links[0].start_character,
        line_index.position_at_utf16(start, source).character.get()
    );
    assert_eq!(
        links[0].end_character,
        line_index.position_at_utf16(end, source).character.get()
    );
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    for source in [
        "source [file join /ROOT $tail]",
        "source [file join /ROOT leaf.tcl; file join /OTHER final.tcl]",
        "source prefix[file join /ROOT leaf.tcl]",
    ] {
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = &plan.commands[0].words[1];
        assert!(
            original_logical_link_anchor(source, config, word).is_none(),
            "{source}"
        );
        assert!(original_logical_link_anchor(&format!("{source} "), config, word).is_none());
    }
}
