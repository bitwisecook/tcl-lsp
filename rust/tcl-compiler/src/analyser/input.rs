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
        }
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
            .finish_non_exhaustive()
    }
}

impl PartialEq for ResolvedAnalysisInput {
    fn eq(&self, other: &Self) -> bool {
        self.profile.cache_key() == other.profile.cache_key()
            && self.unit_profile.cache_key() == other.unit_profile.cache_key()
            && self.config == other.config
            && Arc::ptr_eq(&self.context, &other.context)
    }
}

impl Eq for ResolvedAnalysisInput {}

impl std::hash::Hash for ResolvedAnalysisInput {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.profile.cache_key().hash(state);
        self.unit_profile.cache_key().hash(state);
        self.config.hash(state);
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
}
