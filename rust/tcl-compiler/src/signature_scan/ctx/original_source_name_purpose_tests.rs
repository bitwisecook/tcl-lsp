// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::analyser::{Analyser, ResolvedAnalysisInput};
use crate::command_binding::{SourceAnalysisEntry, SourceAnalysisOptions, SourceCommandBindings};
use crate::signature_scan::original_name::{LogicalSourceNamePurpose, OriginalSourceNamePurpose};
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_syntax::word_rules::WordValueRules;

fn input() -> ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        LexerConfig::for_file_grammar(profile.grammar),
    )
}

fn context<'a>(
    source: &str,
    config: LexerConfig,
    registry: &'a tcl_registry::CommandRegistry,
    bindings: Option<&'a SourceCommandBindings>,
) -> ScanCtx<'a> {
    let image = SourceImage::document(source);
    let plan = tcl_lexer::native_script_words_in(
        image.clone(),
        Span::new(0, u32::try_from(image.len()).unwrap()),
        config,
    )
    .unwrap();
    ScanCtx {
        registry: Some(registry),
        original_bindings: bindings,
        original_image: Some(image),
        original_words: plan.commands[0].words.clone(),
        config,
        rules: WordValueRules::from_config(&config),
        ..Default::default()
    }
}

#[test]
fn original_standalone_scan_keeps_static_unicode_namespace_metadata_without_name_keys() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Public standalone scanning owns source metadata only. No interpreter,
    // input generation, Native naming key or command lookup is supplied.
    let registry = tcl_registry::CommandRegistry::build_default();
    assert!(registry.profile().is_none());
    let config = LexerConfig::for_file_grammar(tcl_dialect::LexerGrammar::default());
    let source = r"namespace eval n\u03bb {proc p {} {}; namespace eval : {proc q {} {}}}";
    let result = crate::signature_scan::extract_signatures_with_config(source, config, &registry);
    let procedure = &result.procs["::nλ::p"];
    assert!(procedure.source_name.is_none());
    assert_eq!(
        procedure.body_namespace,
        SignatureNamespaceScope::Symbolic("::nλ".to_owned())
    );
    assert!(result.procs.contains_key("::nλ:::::q"));
    assert!(
        result
            .command_invocations
            .iter()
            .all(|call| call.original_name_input.is_none())
    );
    let ctx = context(source, config, &registry, None);
    let span = ctx.original_words[2].tokens()[0].span;
    assert!(ctx.name_policy().is_none());
    assert!(ctx.original_name_key(span).is_none());
    assert_eq!(
        ctx.namespace_context("::", "ignored display", span),
        Some(SignatureNamespaceScope::Symbolic("::nλ".to_owned()))
    );
    for source in [
        r"namespace eval $ns {}",
        r"namespace eval {*}{ns} {}",
        r"namespace eval n\u0000tail {}",
        r"namespace eval n\uD800 {}",
    ] {
        let ctx = context(source, config, &registry, None);
        let span = ctx.original_words[2].tokens()[0].span;
        assert!(
            ctx.namespace_context("::", "ns", span).is_none(),
            "{source}"
        );
    }
}

#[test]
fn original_logical_scanner_and_analyser_select_the_same_complete_word_owner() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let input = input();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let source = r"proc name\U0001F600 {} {}";
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        SourceAnalysisOptions::for_logical_source(&input).unwrap(),
    );
    assert_eq!(
        bindings.original_logical_source_name_advice_input(),
        Some(&input)
    );
    let ctx = context(source, config, registry, Some(&bindings));
    let word = &ctx.original_words[1];
    let policy = ctx.name_policy().unwrap();
    let scan = ctx.original_name_key(word.tokens()[0].span).unwrap();
    let analyser_purpose = OriginalSourceNamePurpose::for_analysis_input(&input, None, policy);
    let analyser_key = analyser_purpose.key(word, ctx.rules).unwrap();
    assert_eq!(scan, analyser_key);
    assert_eq!(scan.bytes(), "name😀".as_bytes());
    assert_eq!(scan.source_image(), word.image());
    assert_eq!(scan.lexer_config(), word.config());
    assert!(
        crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
            word, ctx.rules, policy,
        )
        .is_none()
    );
    let analysis = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, input.unit_profile().name);
    assert!(analysis.allows_lexical_declaration_advice());
    assert!(LogicalSourceNamePurpose::for_input(&input, None).is_some());
    let mut changed = context(source, config, registry, Some(&bindings));
    changed.config = config.nested();
    assert!(changed.original_name_key(word.tokens()[0].span).is_none());
    let foreign = tcl_registry::CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let foreign_ctx = context(source, config, &foreign, Some(&bindings));
    assert!(
        foreign_ctx
            .original_name_key(word.tokens()[0].span)
            .is_none()
    );
}

#[test]
fn original_source_name_purpose_preserves_native_entry_recipe_and_hosted_barriers() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let input = input();
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let (_owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let policy = tcl_registry::InvocationDialect::of_profile(profile)
        .authored_name_policy()
        .unwrap();
    let actual = SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(captured)),
        ..Default::default()
    };
    let recipe = SourceAnalysisEntry {
        execution_name_policy: Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
            policy,
        )),
        ..Default::default()
    };
    let config = input.lexer_config();
    let registry = input.borrowed_context_registry().commands();
    let ctx = context(r"proc name\U0001F600 {} {}", config, registry, None);
    let word = &ctx.original_words[1];
    for entry in [&actual, &recipe] {
        assert!(LogicalSourceNamePurpose::for_input(&input, Some(entry)).is_none());
        let purpose = OriginalSourceNamePurpose::for_analysis_input(&input, Some(entry), policy);
        assert!(matches!(purpose, OriginalSourceNamePurpose::Native(_)));
        assert!(purpose.key(word, ctx.rules).is_none());
    }
    let hosted_profile = tcl_dialect::DialectProfile::find("f5-irules").unwrap();
    let hosted = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::context_for_profile(hosted_profile),
        config,
    );
    assert!(hosted.has_hosted_source_name_context());
    assert!(LogicalSourceNamePurpose::for_input(&hosted, None).is_none());
    let native_registry =
        tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let native_config = LexerConfig::for_file_grammar(profile.grammar);
    let native = context("namespace eval n {}", native_config, &native_registry, None);
    let span = native.original_words[2].tokens()[0].span;
    assert!(native.original_name_key(span).is_some());
    assert!(matches!(
        native.namespace_context("::", "n", span),
        Some(SignatureNamespaceScope::C(_))
    ));
    let dynamic = context(
        "namespace eval $n {}",
        native_config,
        &native_registry,
        None,
    );
    let span = dynamic.original_words[2].tokens()[0].span;
    assert!(dynamic.namespace_context("::", "n", span).is_none());
    let mut duplicate = context("namespace eval n {}", native_config, &native_registry, None);
    let span = duplicate.original_words[2].tokens()[0].span;
    duplicate
        .original_words
        .push(duplicate.original_words[2].clone());
    assert!(duplicate.original_name_key(span).is_none());
    assert!(duplicate.namespace_context("::", "n", span).is_none());
    let mut conflicting = profile.clone();
    conflicting.core_point = tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl9.0"));
    let conflicting = conflicting.intern();
    let unknown = tcl_registry::CommandRegistry::build_default().project_for_profile(conflicting);
    assert!(
        tcl_registry::InvocationDialect::of_profile(conflicting)
            .authored_name_policy()
            .is_none()
    );
    let unknown = context("namespace eval n {}", native_config, &unknown, None);
    let span = unknown.original_words[2].tokens()[0].span;
    assert!(unknown.source_name_purpose().is_none());
    assert!(unknown.namespace_context("::", "n", span).is_none());
}
