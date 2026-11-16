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
use crate::analyser::ResolvedAnalysisInput;
use std::sync::Arc;

type ResolvedRealmProducer = fn(
    &str,
    &ResolvedAnalysisInput,
    Option<&crate::command_binding::SourceDeclaredCommandContracts>,
) -> CommandBindingRealm;

fn original_inputs() -> Vec<(&'static str, ResolvedAnalysisInput, ResolvedRealmProducer)> {
    let native = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
        .expect("actual known C Tcl environment");
    let hosted = tcl_registry::model::ingress::resolve_known_environment("f5-irules")
        .expect("actual known authored hosted environment");
    let context = native.default_context_registry();
    [
        (
            "authoring",
            native.unit_profile(),
            Arc::clone(&context),
            document_realm_bindings_with_resolved_input as ResolvedRealmProducer,
        ),
        (
            "logical",
            tcl_dialect::DialectProfile::plain_tcl(),
            context,
            document_lexical_realm_with_resolved_input as ResolvedRealmProducer,
        ),
        (
            "hosted",
            hosted.unit_profile(),
            hosted.default_context_registry(),
            document_vendor_realm_with_resolved_input as ResolvedRealmProducer,
        ),
    ]
    .into_iter()
    .map(|(label, profile, context, producer)| {
        let config = tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Skip,
            strict_quoting: true,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        (
            label,
            ResolvedAnalysisInput::new(profile, profile, context, config),
            producer,
        )
    })
    .collect()
}

#[test]
fn original_resolved_realm_producers_keep_exact_input_and_independent_source_currency() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    // Retaining editing input supplies no Native admission or frame entry.
    let source = "\u{feff}set saved 1\nset final 2";
    for (label, input, producer) in original_inputs() {
        let realm = producer(source, &input, None);
        let image = tcl_lexer::SourceImage::document(source);
        assert!(realm.matches_resolved_analysis_input(&input), "{label}");
        assert!(
            realm.matches_original_source_image(&image, input.lexer_config()),
            "{label}"
        );
        assert!(
            Arc::ptr_eq(
                &realm
                    .source_metadata_context()
                    .expect("original retained availability"),
                &input.context_registry(),
            ),
            "{label}"
        );
        // Same exact input is independent of source-byte/channel currency.
        assert!(
            !realm.matches_original_source_image(
                &tcl_lexer::SourceImage::document(&source.replace("saved", "other")),
                input.lexer_config(),
            ),
            "{label}"
        );
        assert!(
            !realm.matches_original_source_image(
                &tcl_lexer::SourceImage::native(source.as_bytes()),
                input.lexer_config(),
            ),
            "{label}"
        );
        assert!(realm.matches_resolved_analysis_input(&input), "{label}");
    }
}

#[test]
fn original_resolved_realm_producers_refuse_missing_foreign_and_changed_inputs() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    let source = "set retained 1";
    for (label, input, producer) in original_inputs() {
        let realm = producer(source, &input, None);
        let context = input.context_registry();
        let foreign_context = Arc::new(
            tcl_registry::model::ingress::resolve_known_environment("tcl8.4")
                .expect("actual independently selected older availability")
                .default_context_registry()
                .with_command_store(Arc::clone(context.commands())),
        );
        assert!(
            Arc::ptr_eq(context.commands(), foreign_context.commands()),
            "{label}"
        );
        let foreign = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            foreign_context,
            input.lexer_config(),
        );
        assert!(!realm.matches_resolved_analysis_input(&foreign), "{label}");
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let changed = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::clone(&context),
            config,
        );
        assert!(!realm.matches_resolved_analysis_input(&changed), "{label}");
        assert!(
            !realm
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config,),
            "{label}"
        );
        let mut missing = realm.clone();
        missing.source_input = None;
        assert!(!missing.matches_resolved_analysis_input(&input), "{label}");
        assert!(missing.source_metadata_context().is_none(), "{label}");
        assert_eq!(missing.bindings, realm.bindings, "{label}");
        let standalone =
            document_realm_bindings_with_config(source, input.lexer_config(), context.commands());
        assert!(
            !standalone.matches_resolved_analysis_input(&input),
            "{label}"
        );
        assert!(realm.matches_resolved_analysis_input(&input), "{label}");
    }
}
