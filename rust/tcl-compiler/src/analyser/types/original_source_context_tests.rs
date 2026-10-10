// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Whole editing-input correspondence at the shared source boundary.

use crate::analyser::{Analyser, ResolvedAnalysisInput};
use crate::registry_invocation::source_structure::source_registry_words;
use tcl_lexer::SourceImage;

#[test]
fn original_source_correspondence_keeps_its_retained_full_editing_input() {
    // Implementation contract: naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    // This checks retained Rust source metadata, not interpreter behaviour.
    for (dialect, source) in [
        ("tcl8.4", "string length value"),
        ("tcl8.5", "string length value"),
        ("tcl8.6", "string length value"),
        ("tcl9.0", "string length value"),
        ("tcl9.1", "string length value"),
        ("jim", "string length value"),
        ("f5-irules", "set text value"),
    ] {
        let analysis = Analyser::new().analyse(source, dialect);
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        assert!(
            analysis.matches_original_source_image(&image, config),
            "{dialect}"
        );
        assert!(
            source_registry_words(source, &analysis, &segment).is_some(),
            "{dialect}"
        );
        let cloned = analysis.clone();
        assert!(cloned.matches_original_source_image(&image, config));
        let mut changed = analysis.clone();
        let foreign_profile =
            crate::environment_ingress::resolve_environment(if dialect == "jim" {
                "tcl8.6"
            } else {
                "jim"
            })
            .analyser_profile();
        changed.resolved_input = Some(ResolvedAnalysisInput::new(
            foreign_profile,
            foreign_profile,
            tcl_registry::model::ingress::context_for_profile(foreign_profile),
            config,
        ));
        assert!(
            !changed.matches_original_source_image(&image, config),
            "{dialect}"
        );
        assert!(
            source_registry_words(source, &changed, &segment).is_none(),
            "{dialect}"
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(!missing.matches_original_source_image(&image, config));
        assert!(source_registry_words(source, &missing, &segment).is_none());
        let mut changed = analysis.clone();
        let mut override_config = config;
        override_config.leading_bom = match config.leading_bom {
            tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
            tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
        };
        changed.body_lexer_config = Some(override_config);
        assert!(!changed.matches_original_source_image(&image, override_config));
        assert!(source_registry_words(source, &changed, &segment).is_none());
        assert!(!analysis.matches_original_source_image(
            &SourceImage::document(&format!("{source}\n# changed")),
            config
        ));
    }
}

#[test]
fn original_hosted_source_input_keeps_catalogue_naming_and_storage_separate() {
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    for environment in ["f5-irules", "f5-iapps", "f5-tmsh"] {
        let source = "if 1 { puts baseline }";
        let analysis = Analyser::new().analyse(source, environment);
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        assert!(input.has_hosted_source_name_context(), "{environment}");
        let realm = crate::realm::document_vendor_realm_with_resolved_input(source, input, None);
        let invocation = realm.invocation_at_source("if", 0);
        assert!(
            invocation.variable_context.execution_name_policy.is_none(),
            "{environment}"
        );
        assert!(
            invocation
                .variable_context
                .hosted_execution_context
                .is_none(),
            "{environment}"
        );
        let child_offset = u32::try_from(source.find("puts").unwrap()).unwrap();
        let child = analysis
            .original_vendor_source_names()
            .find(|occurrence| occurrence.site().offset == child_offset)
            .unwrap();
        assert!(child.body_origin().is_some(), "{environment}");
        let (metadata, _) =
            crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                source,
                &analysis,
                child_offset,
            )
            .unwrap();
        assert_eq!(metadata.shape().command(), "puts");
        assert!(metadata.matches_registry(context.commands()));
        let image = SourceImage::document(source);
        let config = input.lexer_config();
        assert!(analysis.matches_original_source_image(&image, config));
        let mut changed = analysis.clone();
        let foreign = crate::environment_ingress::resolve_environment("tcl8.6");
        changed.resolved_input = Some(ResolvedAnalysisInput::new(
            foreign.analyser_profile(),
            foreign.unit_profile(),
            tcl_registry::model::ingress::context_for_profile(foreign.analyser_profile()),
            config,
        ));
        assert!(!changed.matches_original_source_image(&image, config));
        assert!(
            crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                source,
                &changed,
                child_offset,
            )
            .is_none()
        );
        assert!(
            crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                "if 1 { puts changed }",
                &analysis,
                child_offset,
            )
            .is_none()
        );
        let mut changed_config = analysis.clone();
        let mut altered = config;
        altered.leading_bom = match config.leading_bom {
            tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
            tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
        };
        changed_config.body_lexer_config = Some(altered);
        assert!(
            crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                source,
                &changed_config,
                child_offset,
            )
            .is_none()
        );
    }

    assert_independent_icall_source_context();
}

fn assert_independent_icall_source_context() {
    // An independently selected iCall source context can coexist with a
    // C-compatible parser without lending C names or runtime storage.
    let environment = crate::environment_ingress::resolve_environment("tcl8.6");
    let profile = environment.analyser_profile();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = ResolvedAnalysisInput::new(profile, environment.unit_profile(), context, config)
        .with_vendor_source_policy(
            tcl_syntax::naming::VendorSourceNamePolicy::authored(
                tcl_registry::f5::BigIpExecutionContext::ICallScript,
            )
            .unwrap(),
        );
    let realm =
        crate::realm::document_vendor_realm_with_resolved_input("set text value", &input, None);
    let invocation = realm.invocation_at_source("set", 0);
    assert!(invocation.variable_context.execution_name_policy.is_none());
    assert!(
        invocation
            .variable_context
            .hosted_execution_context
            .is_none()
    );
    let native = crate::realm::document_realm_bindings_with_config(
        "set text value",
        config,
        input.context_registry().commands(),
    );
    assert!(
        native
            .invocation_at_source("set", 0)
            .variable_context
            .execution_name_policy
            .is_some()
    );
}
