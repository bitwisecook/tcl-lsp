// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Advice domain selection retains the actual input independently of syntax.

use super::AnalysisResult;
use crate::analyser::{Analyser, ResolvedAnalysisInput};
use std::sync::Arc;
use tcl_dialect::model::bigip_execution_context::BigIpExecutionContext;
use tcl_lexer::{LexerConfig, SourceImage};

fn logical_profile() -> &'static tcl_dialect::DialectProfile {
    tcl_dialect::DialectProfile::projected_from_point(
        "explicit-lexical-source-advice",
        &[],
        "Logical source advice with independently selected context",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )
    .intern()
}

fn analyse_input(source: &str, input: ResolvedAnalysisInput) -> AnalysisResult {
    let profile = input.analyser_profile();
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn hosted_source_advice_domain_does_not_require_complete_name_occurrences() {
    // naming.consumer.original-workspace-diagnostic-refinement
    // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
    for dialect in ["f5-irules", "f5-tmsh", "f5-iapps"] {
        for source in ["", "# No command or name occurrence\n"] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            let input = analysis.resolved_input.as_ref().unwrap();
            assert!(input.has_hosted_source_name_context(), "{dialect}");
            assert!(!analysis.has_original_vendor_source_names(), "{dialect}");
            assert!(!analysis.allows_lexical_declaration_advice(), "{dialect}");
            assert!(analysis.matches_original_source_image(
                &SourceImage::document(source),
                analysis.body_lexer_config.unwrap(),
            ));
            analysis.dialect = "copied Logical report label".to_owned();
            assert!(!analysis.allows_lexical_declaration_advice(), "{dialect}");
        }
    }
}

#[test]
fn hosted_source_policy_and_context_remain_independent_of_occurrence_inventory() {
    // naming.consumer.original-workspace-diagnostic-refinement
    // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
    let profile = logical_profile();
    let config = LexerConfig::for_profile(Some(profile));
    let logical_context = tcl_registry::model::ingress::context_for_profile(profile);
    for context in [
        BigIpExecutionContext::TmmIRule,
        BigIpExecutionContext::TmshCliScript,
        BigIpExecutionContext::IAppImplementation,
        BigIpExecutionContext::ICallScript,
    ] {
        let policy = tcl_syntax::naming::VendorSourceNamePolicy::authored(context).unwrap();
        let input =
            ResolvedAnalysisInput::new(profile, profile, Arc::clone(&logical_context), config)
                .with_vendor_source_policy(policy);
        assert!(input.has_hosted_source_name_context(), "{context:?}");
        let analysis = analyse_input("# No original word vector\n", input);
        assert!(!analysis.has_original_vendor_source_names(), "{context:?}");
        assert!(!analysis.allows_lexical_declaration_advice(), "{context:?}");
    }

    // A genuine hosted ContextRegistry remains sufficient to select the
    // naming domain when no explicit source policy field was supplied.
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("f5-irules").default_context_registry(),
        config,
    );
    assert!(input.vendor_source_policy().is_none());
    assert!(input.has_hosted_source_name_context());
    let analysis = analyse_input("", input);
    assert!(!analysis.has_original_vendor_source_names());
    assert!(!analysis.allows_lexical_declaration_advice());
}

#[test]
fn explicit_logical_source_advice_keeps_its_own_context_without_native_promotion() {
    // naming.consumer.original-workspace-diagnostic-refinement
    // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
    let profile = logical_profile();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        LexerConfig::for_profile(Some(profile)),
    );
    assert!(!input.has_hosted_source_name_context());
    let mut analysis = analyse_input("", input);
    assert!(analysis.allows_lexical_declaration_advice());
    analysis.dialect = "f5-irules report label".to_owned();
    assert!(analysis.allows_lexical_declaration_advice());

    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let analysis = Analyser::new().analyse("", dialect);
        assert!(
            !analysis
                .resolved_input
                .as_ref()
                .unwrap()
                .has_hosted_source_name_context()
        );
        assert!(!analysis.allows_lexical_declaration_advice(), "{dialect}");
    }
    assert!(!AnalysisResult::default().allows_lexical_declaration_advice());
}
