// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_syntax::word_rules::WordValueRules;

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
fn original_hosted_standalone_scan_retains_symbolic_metadata_and_nested_bom_purpose() {
    // naming.source.original-configured-script-completeness

    // Actual hosted source descriptors and configured Document geometry only,
    // without a host execution, Native naming provider or logical F5 simulation.
    let source = r"namespace eval n\u03bb {proc visible {} {}; if 1 {proc nested {} {}}}";
    for environment in ["f5-irules", "f5-iapps", "f5-tmsh"] {
        let actual = tcl_registry::model::ingress::resolve_known_environment(environment).unwrap();
        let owner = actual.default_context_registry();
        let registry = owner.commands();
        let profile = registry.profile().unwrap();
        let config = LexerConfig {
            strict_quoting: true,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..LexerConfig::for_file_grammar(profile.grammar)
        };
        let dialect = tcl_registry::InvocationDialect::of_profile(profile);
        assert!(dialect.authored_f5_tcl84_core().is_some());
        assert!(dialect.authored_name_policy().is_none());
        let scan = crate::signature_scan::extract_signatures_with_config(source, config, registry);
        for name in ["::nλ::visible", "::nλ::nested"] {
            let procedure = &scan.procs[name];
            assert!(procedure.source_name.is_none());
            assert_eq!(
                procedure.body_namespace,
                SignatureNamespaceScope::Symbolic("::nλ".to_owned())
            );
        }
        assert!(
            scan.command_invocations
                .iter()
                .all(|call| call.original_name_input.is_none())
        );
        let ctx = context(source, config, registry, None);
        let word_location = ctx.original_words[2].tokens()[0].span;
        assert!(ctx.original_name_key(word_location).is_none());
        assert_eq!(
            ctx.namespace_context("::", "display donor", word_location),
            Some(SignatureNamespaceScope::Symbolic("::nλ".to_owned()))
        );
        let bom = "\u{feff}proc visible {} {}\nif 1 {\u{feff}proc hidden {} {}}\nif 1 {proc nested {} {}}";
        let scan = crate::signature_scan::extract_signatures(bom, registry);
        assert_eq!(
            scan.procs.contains_key("::visible"),
            profile.grammar.script_skips_leading_bom
        );
        assert!(!scan.procs.contains_key("::hidden"));
        assert!(scan.procs.contains_key("::nested"));
        for source in [
            r"namespace eval $ns {}",
            r"namespace eval {*}{ns} {}",
            r"namespace eval n\u0000tail {}",
        ] {
            let ctx = context(source, config, registry, None);
            let word_location = ctx.original_words[2].tokens()[0].span;
            assert!(ctx.namespace_context("::", "ns", word_location).is_none());
        }
    }
}

#[test]
fn original_hosted_standalone_source_cannot_replace_supplied_binding_purposes() {
    // naming.compiler.original-analysis-metadata-context

    // Genuine full hosted source input and held software Native entry stay
    // separate from the header-only symbolic metadata query.
    let environment = tcl_registry::model::ingress::resolve_known_environment("f5-irules").unwrap();
    let owner = environment.default_context_registry();
    let registry = owner.commands();
    let profile = registry.profile().unwrap();
    let config = LexerConfig::for_file_grammar(profile.grammar);
    let source = "namespace eval n {}";
    let input = ResolvedAnalysisInput::new(profile, profile, owner.clone(), config);
    let options = SourceAnalysisOptions::for_hosted_source(&input).unwrap();
    let (_native_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    for options in [
        options,
        SourceAnalysisOptions {
            native_entry: Some(&native),
            ..options
        },
        SourceAnalysisOptions {
            unknown_entry: true,
            ..SourceAnalysisOptions::default()
        },
    ] {
        let bindings =
            SourceCommandBindings::analyse_with_options(source, config, registry, options);
        let ctx = context(source, config, registry, Some(&bindings));
        let span = ctx.original_words[2].tokens()[0].span;
        assert!(ctx.source_name_purpose().is_none());
        assert!(ctx.original_name_key(span).is_none());
        assert!(ctx.namespace_context("::", "n", span).is_none());
    }
    let c86 = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut conflict = c86.clone();
    conflict.name = "hosted-source-conflicting-native-point";
    conflict.core_point = Some(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::TCL_9_0,
    ));
    let conflict = conflict.intern();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(conflict);
    let ctx = context(source, config, &registry, None);
    assert!(ctx.source_name_purpose().is_none());
}
