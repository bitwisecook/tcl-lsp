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

use super::{matching_document_image, retained_document_image, source_registry_words};

#[test]
fn source_schema_geometry_reuses_the_whole_retained_image_and_keeps_currency() {
    // naming.source.original-immutable-image-geometry
    // docs/design/analysis/name-resolution-proofs/original-immutable-image-geometry.md
    let source = "set café 1\r\nset λ 2";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        config,
    );
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    let realm = analysis.retained_command_realm().unwrap();
    let original = realm.original_source_image().unwrap();
    let reused = retained_document_image(source, realm, config).unwrap();
    assert_eq!(reused.bytes().as_ptr(), original.bytes().as_ptr());
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    assert_eq!(segments.len(), 2);
    for segment in &segments {
        let words = source_registry_words(source, &analysis, segment).unwrap();
        assert!(words.matches_source(original, config));
        let word = words.operands()[0].as_ref().unwrap().word().unwrap();
        assert_eq!(word.image().bytes().as_ptr(), original.bytes().as_ptr());
        assert_eq!(word.config(), config);
    }
    let start = u32::try_from(source.find("set λ").unwrap()).unwrap();
    let position = reused.source_map().position_at(start);
    assert_eq!(position.line, 1);
    assert_eq!(position.character.get(), 0);
    assert!(retained_document_image(&format!("{source} "), realm, config).is_none());
    let changed = tcl_lexer::LexerConfig {
        strict_quoting: !config.strict_quoting,
        ..config
    };
    assert!(retained_document_image(source, realm, changed).is_none());
    assert!(
        matching_document_image(source, &tcl_lexer::SourceImage::native(source.as_bytes()))
            .is_none()
    );
}
