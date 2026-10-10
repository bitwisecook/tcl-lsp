// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original hosted source names without borrowing a C/Jim recipe.

use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};
use tcl_syntax::naming::{VendorSourceNamePolicy, VendorSourceNamePurpose};

/// A complete original hosted source word and independently selected context.
/// Unsupported materialisation still retains this producer; it cannot fall
/// back to a reporting string, C compatibility recipe or another context.
/// Registry operand roles, source advice, execution, storage and edits remain
/// separate consumers of this read-only syntax input.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VendorSourceNameInput {
    original: NativeWord,
    policy: VendorSourceNamePolicy,
}

impl VendorSourceNameInput {
    pub(crate) fn from_original_word(
        original: &NativeWord,
        policy: VendorSourceNamePolicy,
    ) -> Self {
        Self {
            original: original.clone(),
            policy,
        }
    }

    /// Full original word, including source channel, grouping and arena.
    #[must_use]
    pub const fn original_word(&self) -> &NativeWord {
        &self.original
    }

    /// Whole original producer image, rather than a name substring.
    #[must_use]
    pub fn source_image(&self) -> &SourceImage {
        self.original.image()
    }

    /// Full original parser configuration, independently of display labels.
    #[must_use]
    pub fn lexer_config(&self) -> LexerConfig {
        self.original.executable_parts().config()
    }

    /// Explicit hosted source context and authoring/observed authority.
    #[must_use]
    pub const fn policy(&self) -> VendorSourceNamePolicy {
        self.policy
    }

    /// Actual complete original word extent; no partial root is forged here.
    #[must_use]
    pub fn span(&self) -> Span {
        self.original.span()
    }

    /// Purpose-selected literal source units, when that producer is supported.
    /// A missing projection leaves the original input owned and unavailable.
    #[must_use]
    pub fn literal_units(&self, purpose: VendorSourceNamePurpose) -> Option<&[u8]> {
        tcl_syntax::naming::vendor_source_literal_units(self.policy, &self.original, purpose)
    }

    /// Nominal root command coordinate from the original authored head.
    /// Shared written-name segmentation handles repeated namespace separators.
    /// This source coordinate supplies no current namespace or runtime lookup;
    /// catalogue and stored-body callers must retain their own applicability.
    #[must_use]
    pub fn root_command_coordinate(&self) -> Option<tcl_core_types::NameBytes> {
        let units = self.literal_units(VendorSourceNamePurpose::CommandHead)?;
        let written = std::str::from_utf8(units).ok()?;
        let mut candidates = tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
            "::",
            &[] as &[&str],
            written,
        );
        (candidates.len() == 1).then_some(())?;
        Some(tcl_core_types::NameBytes::from(
            candidates.pop()?.as_bytes(),
        ))
    }

    /// Complete consumer source/configuration correspondence; no live grant.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.source_image() == image && self.lexer_config() == config
    }
}

/// Actual lexical command site and complete vendor input retained before
/// runtime lookup. A name role or declaration is not inferred from this site.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VendorSourceNameOccurrence {
    site: crate::command_binding::CommandAllocationSite,
    input: VendorSourceNameInput,
    original_words: std::sync::Arc<[NativeWord]>,
    body_origin: Option<std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBodyOrigin>>,
}

impl VendorSourceNameOccurrence {
    pub(crate) fn new(
        site: &crate::command_binding::CommandAllocationSite,
        original: &NativeWord,
        policy: VendorSourceNamePolicy,
        original_words: std::sync::Arc<[NativeWord]>,
    ) -> Option<Self> {
        (site.source.source_image() == original.image()
            && site.offset == original_words.first()?.group().span.start()
            && original_words.contains(original)
            && original_words.iter().all(|word| {
                word.image() == original.image()
                    && word.executable_parts().config() == original.executable_parts().config()
            })
            && original_words
                .windows(2)
                .all(|words| words[0].span().end() <= words[1].span().start()))
        .then(|| Self {
            site: site.clone(),
            input: VendorSourceNameInput::from_original_word(original, policy),
            original_words,
            body_origin: None,
        })
    }

    pub(crate) fn with_body_origin(
        mut self,
        origin: Option<&std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBodyOrigin>>,
    ) -> Self {
        self.body_origin = origin.cloned();
        self
    }
    pub(crate) fn body_origin(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceScriptBodyOrigin> {
        self.body_origin.as_deref()
    }
    pub(crate) fn retained_body_origin(
        &self,
    ) -> Option<std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBodyOrigin>> {
        self.body_origin.clone()
    }
    /// Combine the same lexical occurrence's optional sealed source ancestry.
    /// An ordinary walk cannot erase a previously issued body-role receipt.
    pub(crate) fn merge_source_origin(&self, other: &Self) -> Option<Self> {
        if self.site != other.site
            || self.input != other.input
            || self.original_words != other.original_words
        {
            return None;
        }
        match (&self.body_origin, &other.body_origin) {
            (Some(left), Some(right)) if left == right => Some(self.clone()),
            (Some(left), Some(right)) => {
                let mut merged = self.clone();
                merged.body_origin = Some(std::sync::Arc::new(left.merge(right)?));
                Some(merged)
            }
            (None, Some(_)) => Some(other.clone()),
            _ => Some(self.clone()),
        }
    }

    /// Genuine pre-walk original command layout, without reached invocation.
    #[must_use]
    pub const fn site(&self) -> &crate::command_binding::CommandAllocationSite {
        &self.site
    }

    /// Complete original word and its independently selected hosted context.
    #[must_use]
    pub const fn name_input(&self) -> &VendorSourceNameInput {
        &self.input
    }

    /// Complete original lexical vector retained by the same pre-walk issuer.
    /// No effective argv, Registry role or handler preparation is inferred.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original_words
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::bigip_execution_context::BigIpExecutionContext;
    use tcl_syntax::naming::VendorSourceNameAuthority;

    #[test]
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    fn original_vendor_words_keep_context_units_and_unknown_producers_separate() {
        let source = "proc {p\\uD800} {arg} {return $arg}\npackage require {pkg\\uD800}\nsource {file\\uD800}\nset {v\\uD800} 1";
        for (profile, context) in [
            ("f5-irules", BigIpExecutionContext::TmmIRule),
            ("f5-iapps", BigIpExecutionContext::IAppImplementation),
            ("f5-tmsh", BigIpExecutionContext::TmshCliScript),
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, profile);
            assert!(analysis.has_original_vendor_source_names(), "{profile}");
            assert!(
                !analysis.allows_lexical_declaration_advice(),
                "{profile}: owned vendor input cannot fall back"
            );
            let image = SourceImage::document(source);
            let mut opaque = 0;
            let mut literal = 0;
            for occurrence in analysis.original_vendor_source_names() {
                let input = occurrence.name_input();
                assert_eq!(input.policy().context(), context);
                assert_eq!(
                    input.policy().authority(),
                    VendorSourceNameAuthority::AuthoredContext
                );
                assert_eq!(input.source_image(), &image);
                assert!(
                    analysis
                        .original_vendor_source_name_in_source(
                            &image,
                            input.lexer_config(),
                            input.span()
                        )
                        .is_some()
                );
                let content = input.original_word().content_span().unwrap();
                let raw = &image.bytes()[content.as_range()];
                if raw.contains(&b'\\') {
                    opaque += 1;
                    assert!(
                        input
                            .literal_units(VendorSourceNamePurpose::ProcedureName)
                            .is_none()
                    );
                } else if input
                    .literal_units(VendorSourceNamePurpose::CommandHead)
                    .is_some()
                {
                    literal += 1;
                }
                assert!(
                    analysis
                        .original_vendor_source_name_in_source(
                            &SourceImage::document(&format!("{source}\n# changed")),
                            input.lexer_config(),
                            input.span()
                        )
                        .is_none()
                );
            }
            assert!(
                opaque >= 4,
                "{profile}: original escaped names remain owned"
            );
            assert!(literal >= 4, "{profile}: supported source literals");
            assert!(
                analysis.original_variable_symbols.is_empty(),
                "source policy does not issue storage symbols"
            );
        }
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.4");
        assert!(!analysis.has_original_vendor_source_names());
    }

    #[test]
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    fn explicit_vendor_context_does_not_rewrite_grammar_or_attest_a_build() {
        let environment = tcl_registry::model::ingress::resolve_environment("f5-iapps");
        let config = LexerConfig::from_grammar(environment.grammar());
        let policy = VendorSourceNamePolicy::authored(BigIpExecutionContext::ICallScript).unwrap();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            tcl_registry::model::ingress::context_for_profile(environment.unit_profile()),
            config,
        )
        .with_vendor_source_policy(policy);
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse("proc p {arg} {return $arg}", "presentation-only");
        assert!(analysis.has_original_vendor_source_names());
        assert!(
            analysis
                .original_vendor_source_names()
                .all(|occurrence| occurrence.name_input().policy() == policy
                    && occurrence.name_input().lexer_config() == config)
        );
    }
}

/// Registry-selected hosted declaration advice. The authentic original name
/// remains present when its literal units are unsupported. This record grants
/// no canonical command slot, runtime cell, lookup, frame, or source edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorSourceDeclarationMetadata<T> {
    original: VendorSourceNameOccurrence,
    purpose: VendorSourceNamePurpose,
    metadata: T,
}

impl<T> VendorSourceDeclarationMetadata<T> {
    pub(crate) fn new(
        original: &VendorSourceNameOccurrence,
        purpose: VendorSourceNamePurpose,
        metadata: T,
    ) -> Self {
        Self {
            original: original.clone(),
            purpose,
            metadata,
        }
    }

    /// Actual selected declaration worker and independently retained word.
    #[must_use]
    pub const fn original_occurrence(&self) -> &VendorSourceNameOccurrence {
        &self.original
    }

    /// Complete hosted source producer; unavailable units never mean absent input.
    #[must_use]
    pub const fn name_input(&self) -> &VendorSourceNameInput {
        self.original.name_input()
    }

    /// Source role selected independently by the genuine Registry worker.
    #[must_use]
    pub const fn purpose(&self) -> VendorSourceNamePurpose {
        self.purpose
    }

    /// Source-card metadata, independent of runtime publication or identity.
    #[must_use]
    pub const fn metadata(&self) -> &T {
        &self.metadata
    }
}
