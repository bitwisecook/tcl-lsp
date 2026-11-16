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

use super::{
    original_query_document_image, registered_original_words_at, source_registry_words_at,
};
use crate::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use std::sync::Arc;
use tcl_lexer::{SourceChannel, SourceImage};

fn analysed(source: &str) -> AnalysisResult {
    Analyser::new().analyse(source, "tcl8.6")
}

#[test]
fn original_queries_reborrow_same_retained_document_geometry() {
    let source = "list literal\n";
    let analysis = analysed(source);
    let config = analysis.body_lexer_config.unwrap();
    let retained = analysis
        .retained_command_realm()
        .unwrap()
        .original_source_image()
        .unwrap();
    let image = original_query_document_image(source, &analysis, config);
    assert!(Arc::ptr_eq(&image.shared_bytes(), &retained.shared_bytes()));
    assert_eq!(image, SourceImage::document(source));
    assert_eq!(
        source_registry_words_at(source, &analysis, 0)
            .unwrap()
            .command(),
        "list"
    );
    let (_, _, words) = registered_original_words_at(source, &analysis, 0).unwrap();
    assert_eq!(words.len(), 2);
    assert!(words.iter().all(|word| word.image() == retained));
    let other = analysed(source);
    let other_image = original_query_document_image(source, &other, config);
    let own = other
        .retained_command_realm()
        .unwrap()
        .original_source_image()
        .unwrap();
    assert!(Arc::ptr_eq(
        &other_image.shared_bytes(),
        &own.shared_bytes()
    ));
    assert!(source_registry_words_at(source, &other, 0).is_some());
}

#[test]
fn original_query_geometry_fallback_preserves_stale_and_missing_refusal() {
    let source = "list literal\n";
    let analysis = analysed(source);
    let config = analysis.body_lexer_config.unwrap();
    let retained = analysis
        .retained_command_realm()
        .unwrap()
        .original_source_image()
        .unwrap();
    let changed = "list another\n";
    let fallback = original_query_document_image(changed, &analysis, config);
    assert_eq!(fallback, SourceImage::document(changed));
    assert!(!Arc::ptr_eq(
        &fallback.shared_bytes(),
        &retained.shared_bytes()
    ));
    assert!(source_registry_words_at(changed, &analysis, 0).is_none());
    assert!(registered_original_words_at(changed, &analysis, 0).is_none());
    let missing = AnalysisResult::default();
    assert_eq!(
        original_query_document_image(source, &missing, config),
        SourceImage::document(source)
    );
    assert!(source_registry_words_at(source, &missing, 0).is_none());
    assert!(registered_original_words_at(source, &missing, 0).is_none());
    assert!(
        super::matching_document_image(source, &SourceImage::native(source.as_bytes())).is_none()
    );
    assert_eq!(fallback.channel(), SourceChannel::Document);
}

#[test]
fn original_query_geometry_keeps_full_configuration_and_registry_guards() {
    let source = "list literal\n";
    let analysis = analysed(source);
    assert!(source_registry_words_at(source, &analysis, 0).is_some());
    assert!(registered_original_words_at(source, &analysis, 0).is_some());
    let original = analysis.resolved_input.as_ref().unwrap();
    let mut config = original.lexer_config();
    config.leading_bom = match config.leading_bom {
        tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
        tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
    };
    let mut changed = analysis.clone();
    changed.body_lexer_config = Some(config);
    assert!(source_registry_words_at(source, &changed, 0).is_none());
    assert!(registered_original_words_at(source, &changed, 0).is_none());
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl8.5").default_context_registry();
    changed = analysis.clone();
    changed.resolved_input = Some(ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        foreign,
        original.lexer_config(),
    ));
    assert!(source_registry_words_at(source, &changed, 0).is_none());
    assert!(registered_original_words_at(source, &changed, 0).is_none());
}
