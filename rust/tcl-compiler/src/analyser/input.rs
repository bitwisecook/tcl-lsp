// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retained editing inputs, independently of native execution admission.

use std::sync::Arc;

/// The exact profile, availability generation and body grammar supplied by a
/// driver. Clones retain the same immutable generation for body-cache keys;
/// independently assembled generations conservatively invalidate those keys.
#[derive(Clone)]
pub struct ResolvedAnalysisInput {
    pub(super) profile: &'static tcl_dialect::DialectProfile,
    pub(super) unit_profile: &'static tcl_dialect::DialectProfile,
    pub(super) context: Arc<tcl_registry::model::ContextRegistry>,
    pub(super) config: tcl_lexer::LexerConfig,
    vendor_source_policy: Option<tcl_syntax::naming::VendorSourceNamePolicy>,
}

impl ResolvedAnalysisInput {
    /// Retain the driver's editing inputs. The config describes a body at
    /// offset zero; nested walks supply their own offsets. Native execution
    /// admission remains the separate `SourceAnalysisEntry` contract.
    #[must_use]
    pub const fn new(
        profile: &'static tcl_dialect::DialectProfile,
        unit_profile: &'static tcl_dialect::DialectProfile,
        context: Arc<tcl_registry::model::ContextRegistry>,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        Self {
            profile,
            unit_profile,
            context,
            config,
            vendor_source_policy: None,
        }
    }

    /// Explicit hosted source policy, including contexts without a catalogue
    /// environment. This does not change runtime admission or the parser.
    #[must_use]
    pub fn with_vendor_source_policy(
        mut self,
        policy: tcl_syntax::naming::VendorSourceNamePolicy,
    ) -> Self {
        self.vendor_source_policy = Some(policy);
        self
    }

    /// Independently supplied hosted source policy; no compatibility fallback.
    #[must_use]
    pub const fn vendor_source_policy(&self) -> Option<tcl_syntax::naming::VendorSourceNamePolicy> {
        self.vendor_source_policy
    }

    /// The retained input independently selects a hosted source naming domain.
    /// Empty or unavailable source inventories cannot enable Logical naming
    /// compatibility. This supplies no execution, native recipe or allocation.
    #[must_use]
    pub fn has_hosted_source_name_context(&self) -> bool {
        self.vendor_source_policy.is_some()
            || tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
                &self.context.context().environment.id,
            )
            .is_some_and(tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::is_appliance_hosted)
    }

    /// Positively selected Logical editing inputs, independently of the
    /// unversioned Tcl authored naming simulation. A lexical point alone does
    /// not supply a Native naming policy; hosted source stays separate.
    pub(crate) fn has_logical_source_name_context(&self) -> bool {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        !self.has_hosted_source_name_context()
            && [self.profile, self.unit_profile]
                .into_iter()
                .all(|profile| {
                    let dialect = tcl_registry::InvocationDialect::of_profile(profile);
                    dialect.native_name_protocol().is_none()
                })
    }

    /// Exact profile used for analyser assistance, including custom policies.
    #[must_use]
    pub const fn analyser_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.profile
    }

    /// Exact compilation-unit profile, independently of analyser assistance.
    #[must_use]
    pub const fn unit_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.unit_profile
    }

    /// Share the immutable availability generation without resolving its label.
    #[must_use]
    pub fn context_registry(&self) -> Arc<tcl_registry::model::ContextRegistry> {
        Arc::clone(&self.context)
    }

    /// Borrow the actual immutable command store and complete availability.
    #[must_use]
    pub fn borrowed_context_registry(&self) -> &tcl_registry::model::ContextRegistry {
        &self.context
    }

    /// Borrow the complete retained authoring availability context. This
    /// metadata supplies no loaded package, runtime table or execution grant.
    #[must_use]
    pub fn availability_context(&self) -> &tcl_registry::model::ResolvedContext {
        self.context.context()
    }

    /// Exact body lexer configuration, before document-offset BOM handling.
    #[must_use]
    pub const fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
}

impl std::fmt::Debug for ResolvedAnalysisInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResolvedAnalysisInput")
            .field("profile", &self.profile)
            .field("unit_profile", &self.unit_profile)
            .field("context", self.context.context())
            .field("config", &self.config)
            .field("vendor_source_policy", &self.vendor_source_policy)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ResolvedAnalysisInput {
    fn eq(&self, other: &Self) -> bool {
        self.profile.cache_key() == other.profile.cache_key()
            && self.unit_profile.cache_key() == other.unit_profile.cache_key()
            && self.config == other.config
            && self.vendor_source_policy == other.vendor_source_policy
            && Arc::ptr_eq(&self.context, &other.context)
    }
}

impl Eq for ResolvedAnalysisInput {}

impl std::hash::Hash for ResolvedAnalysisInput {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.profile.cache_key().hash(state);
        self.unit_profile.cache_key().hash(state);
        self.config.hash(state);
        self.vendor_source_policy.hash(state);
        Arc::as_ptr(&self.context).hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    fn input() -> ResolvedAnalysisInput {
        let mut profile = tcl_registry::model::ingress::resolve_environment("tcl8.4")
            .unit_profile()
            .clone();
        profile.name = "tcl9.0";
        let profile = profile.intern();
        ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
    }

    #[test]
    fn logical_source_domain_keeps_authored_simulation_and_actual_native_entry_separate() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        assert!(
            tcl_registry::InvocationDialect::of_profile(profile)
                .authored_name_policy()
                .is_some()
        );
        assert!(input.has_logical_source_name_context());
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse("expr $a + $b", profile.name);
        assert_eq!(
            analysis
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .original_logical_source_name_advice_input(),
            Some(&input)
        );
        for native in ["tcl8.4", "tcl8.6", "tcl9.1", "jim"] {
            let profile = tcl_registry::model::ingress::resolve_environment(native).unit_profile();
            let actual = ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::context_for_profile(profile),
                input.config,
            );
            assert!(!actual.has_logical_source_name_context(), "{native}");
            let mixed = ResolvedAnalysisInput::new(
                input.profile,
                profile,
                input.context_registry(),
                input.config,
            );
            assert!(!mixed.has_logical_source_name_context());
        }
        let lexical_point = tcl_dialect::DialectProfile::projected_from_point(
            "explicit-logical-jim-grammar",
            &[],
            "Explicit Logical grammar",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )
        .intern();
        let lexical = ResolvedAnalysisInput::new(
            lexical_point,
            lexical_point,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_profile(Some(lexical_point)),
        );
        assert!(
            tcl_registry::InvocationDialect::of_profile(lexical_point)
                .core_point
                .is_some()
        );
        assert!(lexical.has_logical_source_name_context());
        let source = "missing 1 2";
        let analysis = Analyser::new()
            .with_resolved_input(lexical.clone())
            .analyse(source, lexical_point.name);
        assert_eq!(
            analysis
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .original_logical_source_name_advice_input(),
            Some(&lexical)
        );
        let hosted = tcl_dialect::DialectProfile::irules();
        let actual = ResolvedAnalysisInput::new(
            hosted,
            hosted,
            tcl_registry::model::ingress::context_for_profile(hosted),
            input.config,
        );
        assert!(!actual.has_logical_source_name_context());
    }

    fn advice_entry_points(input: &ResolvedAnalysisInput) -> Vec<crate::analyser::AnalysisResult> {
        let source = "proc p {} {global x; puts $x}";
        let profile = input.analyser_profile();
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        );
        let analyser = || Analyser::new().with_resolved_input(input.clone());
        vec![
            analyser().analyse(source, profile.name),
            analyser().analyse_per_item(source, profile.name),
            analyser()
                .analyse_chunked(source, vec![commands.clone()], profile.name)
                .0,
            analyser().analyse_commands(source, &commands, profile.name, true),
        ]
    }

    #[test]
    fn logical_declaration_advice_uses_actual_input_at_each_analysis_entry() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        // naming.core.original-declaration-navigation
        // docs/design/analysis/name-resolution-proofs/core-original-declaration-navigation.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        assert!(input.has_logical_source_name_context());
        assert!(
            tcl_registry::InvocationDialect::of_profile(profile)
                .authored_name_policy()
                .is_some()
        );
        for analysis in advice_entry_points(&input) {
            assert!(analysis.allows_retained_logical_declaration_advice());
            assert_eq!(analysis.resolved_input.as_ref(), Some(&input));
        }
        assert!(
            !crate::analyser::AnalysisResult::default()
                .allows_retained_logical_declaration_advice()
        );
        for dialect in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "jim",
            "f5-irules",
        ] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
            let actual = ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::context_for_profile(profile),
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            );
            for analysis in advice_entry_points(&actual) {
                assert!(!analysis.allows_lexical_declaration_advice(), "{dialect}");
                assert!(
                    !analysis.allows_retained_logical_declaration_advice(),
                    "{dialect}"
                );
            }
        }
        let native_profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mixed = ResolvedAnalysisInput::new(
            profile,
            native_profile,
            input.context_registry(),
            input.config,
        );
        assert!(
            advice_entry_points(&mixed)
                .iter()
                .all(|analysis| !analysis.allows_retained_logical_declaration_advice())
        );
    }

    #[test]
    fn logical_declaration_advice_requires_its_retained_input_after_copying_metadata() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let source = "proc lexical {} {}; set ordinary 1";
        let logical = Analyser::new().analyse(source, "tcl");
        assert!(logical.allows_lexical_declaration_advice());
        assert!(logical.allows_retained_logical_declaration_advice());
        let mut missing = logical.clone();
        missing.resolved_input = None;
        assert!(!missing.allows_lexical_declaration_advice());
        assert!(!missing.allows_retained_logical_declaration_advice());
        for dialect in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "jim",
            "f5-irules",
        ] {
            let original = Analyser::new().analyse(source, dialect);
            let mut copied = logical.clone();
            copied.resolved_input = original.resolved_input;
            assert!(!copied.allows_lexical_declaration_advice(), "{dialect}");
            assert!(
                !copied.allows_retained_logical_declaration_advice(),
                "{dialect}"
            );
        }
    }

    #[test]
    fn logical_declaration_advice_keeps_native_entry_and_recipe_as_independent_barriers() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let native_profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let recipe = crate::command_binding::SourceAnalysisEntry {
            execution_name_policy: Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
                tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                    native_profile.core_point.unwrap(),
                )
                .unwrap(),
            )),
            ..Default::default()
        };
        let (_owner, native) =
            crate::environment_ingress::captured_native_entry_with_owner(native_profile);
        let actual = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(Arc::new(native)),
            ..Default::default()
        };
        for entry in [recipe, actual] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .with_source_analysis_entry(Arc::new(entry))
                .analyse("proc p {} {}", profile.name);
            assert!(
                analysis
                    .resolved_input
                    .as_ref()
                    .unwrap()
                    .has_logical_source_name_context()
            );
            assert!(!analysis.allows_lexical_declaration_advice());
            assert!(!analysis.allows_retained_logical_declaration_advice());
        }
    }

    #[test]
    fn retained_input_keeps_release_and_variable_grammar_across_body_analysis() {
        let input = input();
        let source = "set \"::a{b\" 7\nset {::a{b}c} 8\nproc p {} {return ${::a{b}c}}\np\n";
        let full = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "tcl9.0");
        let per_item = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse_per_item(source, "tcl9.0");
        assert_eq!(full, per_item);
        assert_eq!(full.body_lexer_config, Some(input.config));
        let first_close = full.global_scope.variables.get("::a{b").unwrap();
        let nested_close = full.global_scope.variables.get("::a{b}c").unwrap();
        assert!(
            first_close.references.len() > nested_close.references.len(),
            "{full:?}"
        );
        let canonical = Analyser::new().analyse(source, "tcl9.0");
        let first_close = canonical.global_scope.variables.get("::a{b").unwrap();
        let nested_close = canonical.global_scope.variables.get("::a{b}c").unwrap();
        assert!(
            nested_close.references.len() > first_close.references.len(),
            "{canonical:?}"
        );
    }

    #[test]
    fn retained_input_survives_reuse_and_snapshot_restore() {
        let input = input();
        let mut analyser = Analyser::new().with_resolved_input(input.clone());
        analyser.analyse("proc p {} {return 1}", "presentation-only");
        let snapshot = analyser.snapshot();
        let mut restored = Analyser::new();
        restored.restore(snapshot);
        assert_eq!(restored.resolved_analysis_input(), input);
        let reused = restored.analyse("proc q {} {return 2}", "tcl9.0");
        assert_eq!(reused.body_lexer_config, Some(input.config));
        assert_eq!(restored.profile.core_point, input.profile.core_point);
    }

    #[test]
    fn input_cache_identity_retains_the_exact_generation() {
        let input = input();
        assert_eq!(input, input.clone());
        let other = ResolvedAnalysisInput::new(
            input.profile,
            input.unit_profile,
            Arc::new(
                input
                    .context
                    .with_command_store(Arc::clone(input.context.commands())),
            ),
            input.config,
        );
        assert_ne!(input, other);
    }

    #[test]
    fn original_file_ingress_keeps_supplied_bom_configuration_and_default_source_policy() {
        // Implementation contract: naming.consumer.original-ilx-method-source-candidates
        // docs/design/analysis/name-resolution-proofs/original-ilx-method-source-candidates.md
        // Source syntax correspondence only; no native source or event entry.
        for profile in [
            tcl_dialect::DialectProfile::irules(),
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
        ] {
            let context = tcl_registry::model::ingress::context_for_profile(profile);
            for leading_bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
                let config = tcl_lexer::LexerConfig {
                    leading_bom,
                    ..tcl_lexer::LexerConfig::for_profile(Some(profile))
                };
                let input =
                    ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
                let mut analyser = Analyser::new().with_resolved_input(input);
                assert_eq!(analyser.file_lexer_config(), config);
                let source = "\u{feff}set name value";
                let analysis = analyser.analyse(source, profile.name);
                assert_eq!(analysis.body_lexer_config, Some(config));
                assert!(analysis.matches_original_source_image(
                    &tcl_lexer::SourceImage::document(source),
                    config,
                ));
                if profile.is_irules() {
                    let retained_set = analysis.original_vendor_source_names().any(|row| {
                        row.name_input()
                            .literal_units(tcl_syntax::naming::VendorSourceNamePurpose::SourceName)
                            == Some(b"set".as_slice())
                    });
                    assert_eq!(retained_set, leading_bom == tcl_lexer::LeadingBom::Skip);
                }
            }
            let mut default = Analyser::new();
            default.resolve_walk_environment(profile.name);
            assert_eq!(
                default.file_lexer_config().leading_bom,
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar).leading_bom
            );
        }
    }
}
