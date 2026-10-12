// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original excluded command syntax retains its independently selected domain.

use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use crate::command_binding::{
    OriginalCatalogueSourceObligation, OriginalCommandLookup, SourceInvocationBinding,
};
use crate::signature_scan::vendor_name::{VendorSourceNameInput, VendorSourceNameOccurrence};
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage};
use tcl_registry::model::CommandSourceUnavailability;

/// Independently selected naming domain of excluded original source syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSourceAvailabilityDomain {
    /// The original Native source word and retained counted lookup geometry.
    Native,
    /// Positively selected Logical source input, without a Native key.
    Logical,
    /// Hosted source input, without borrowing a C or Jim naming recipe.
    Hosted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AvailabilityOrigin {
    Native {
        lookup: OriginalCommandLookup,
        obligations: Vec<OriginalCatalogueSourceObligation>,
    },
    Logical {
        paths: Vec<Vec<tcl_core_types::NameBytes>>,
    },
    Hosted {
        occurrence: VendorSourceNameOccurrence,
        paths: Vec<Vec<tcl_core_types::NameBytes>>,
        barriers: Box<super::VendorSourceCatalogueBarriers>,
    },
}

/// Sealed original source advice about an excluded Registry command.
/// No admitted schema, selected handler, runtime absence or completion follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceCommandAvailability {
    descriptor: CommandSourceUnavailability,
    original: Arc<[NativeWord]>,
    input: ResolvedAnalysisInput,
    origin: AvailabilityOrigin,
    invocation: Arc<SourceInvocationBinding>,
    realm: tcl_dialect::model::InvocationRealm,
}
impl OriginalSourceCommandAvailability {
    /// Negative metadata from the same complete availability generation.
    #[must_use]
    pub const fn descriptor(&self) -> &CommandSourceUnavailability {
        &self.descriptor
    }
    /// Whole original unchanged vector, retaining grouping and source channel.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Actual original command head, never derived from diagnostic text.
    #[must_use]
    pub fn original_head(&self) -> &NativeWord {
        &self.original[0]
    }
    /// Complete source input, including exact availability generation and policy.
    #[must_use]
    pub const fn input(&self) -> &ResolvedAnalysisInput {
        &self.input
    }
    /// Positively selected source purpose, separate from native handler identity.
    #[must_use]
    pub const fn domain(&self) -> CommandSourceAvailabilityDomain {
        match &self.origin {
            AvailabilityOrigin::Native { .. } => CommandSourceAvailabilityDomain::Native,
            AvailabilityOrigin::Logical { .. } => CommandSourceAvailabilityDomain::Logical,
            AvailabilityOrigin::Hosted { .. } => CommandSourceAvailabilityDomain::Hosted,
        }
    }
    /// Genuine counted Native lookup geometry, if that independent owner exists.
    #[must_use]
    pub const fn native_lookup(&self) -> Option<&OriginalCommandLookup> {
        match &self.origin {
            AvailabilityOrigin::Native { lookup, .. } => Some(lookup),
            _ => None,
        }
    }
    /// Genuine hosted original name policy, if independently retained.
    #[must_use]
    pub fn hosted_input(&self) -> Option<&VendorSourceNameInput> {
        match &self.origin {
            AvailabilityOrigin::Hosted { occurrence, .. } => Some(occurrence.name_input()),
            _ => None,
        }
    }
    /// Same original source and complete retained input; display changes confer
    /// no currency. The exact immutable Registry generation remains retained.
    #[must_use]
    pub fn matches_source_input(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        self.input == *input
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
            && self
                .input
                .context_registry()
                .command_source_unavailability(self.descriptor.command(), self.realm)
                .as_ref()
                == Some(&self.descriptor)
    }
    /// Original same-document publication geometry can withdraw nominal
    /// Registry advice. This does not establish that a declaration executes.
    pub(crate) fn has_source_declaration_shadow(
        &self,
        analysis: &AnalysisResult,
        enforce_order: bool,
    ) -> bool {
        let Some(lookup) = self.native_lookup() else {
            return false;
        };
        let before = |site: &crate::command_binding::CommandAllocationSite| {
            !enforce_order || site.offset < self.original_head().span().start()
        };
        let procedures = analysis
            .original_procedure_declarations()
            .filter(|row| before(row.declaration_site()))
            .map(|row| (row.name(), row.declaration_site().clone()));
        let classes = analysis
            .original_class_declarations()
            .filter(|row| before(row.declaration_site()))
            .map(|row| (row.name(), row.declaration_site().clone()));
        lookup
            .matching_publications(procedures.chain(classes))
            .is_some_and(|rows| !rows.is_empty())
    }

    /// Current analysis and original Realm retain the same immutable issuer.
    #[must_use]
    pub fn matches_analysis(&self, analysis: &AnalysisResult) -> bool {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        analysis.resolved_input.as_ref().is_some_and(|input| {
            self.matches_source_input(self.original_head().image(), input)
                && analysis.matches_original_source_image(
                    self.original_head().image(),
                    input.lexer_config(),
                )
                && analysis.retained_command_realm().is_some_and(|realm| {
                    realm.invocation_at_source("", self.original_head().span().start())
                        == *self.invocation
                })
        })
    }
}

/// Authenticate excluded metadata from a genuine original whole invocation.
/// A refused Native or hosted issuer cannot enter Logical compatibility.
#[must_use]
pub fn original_source_command_availability(
    source: &str,
    analysis: &AnalysisResult,
    tokens: &crate::ir::CommandTokens,
) -> Option<OriginalSourceCommandAvailability> {
    // naming.diagnostic.original-command-source-unavailability
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
    let input = analysis.resolved_input.as_ref()?;
    let image = SourceImage::document(source);
    let config = input.lexer_config();
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let realm = analysis.retained_command_realm()?;
    realm.matches_resolved_analysis_input(input).then_some(())?;
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let offset = tokens.argv.first()?.start();
    if realm.invocation_at_source("", offset) != *binding {
        return None;
    }
    let original = super::original_native_compiler_words(&image, tokens.words(), offset, config)?;
    if original.iter().any(|word| word.group().expand) {
        return None;
    }
    let first = original.first()?;
    let invocation_realm = binding.invocation_realm().unwrap_or_default();
    let context = input.context_registry();
    let (descriptor, origin) = if input.has_hosted_source_name_context() {
        hosted_unavailability(
            analysis,
            binding,
            tokens,
            &original,
            input,
            invocation_realm,
        )?
    } else if realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        == Some(input)
    {
        (binding.logical_source_name_advice_input() == Some(input)).then_some(())?;
        let bytes = tcl_syntax::word_rules::original_static_word_ascii_presentation(first)?;
        if bytes.contains(&0) {
            return None;
        }
        let written = std::str::from_utf8(&bytes).ok()?;
        let (descriptor, paths) = binding.original_authored_unavailable_command(
            tokens,
            first,
            &context,
            invocation_realm,
            written,
        )?;
        (descriptor, AvailabilityOrigin::Logical { paths })
    } else {
        let selected =
            binding.original_native_unavailable_command(tokens, &context, invocation_realm)?;
        (
            selected.descriptor,
            AvailabilityOrigin::Native {
                lookup: selected.lookup,
                obligations: selected.obligations,
            },
        )
    };
    Some(OriginalSourceCommandAvailability {
        descriptor,
        original: Arc::from(original),
        input: input.clone(),
        origin,
        invocation: Arc::new(binding.clone()),
        realm: invocation_realm,
    })
}

fn hosted_unavailability(
    analysis: &AnalysisResult,
    binding: &SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    original: &[NativeWord],
    input: &ResolvedAnalysisInput,
    invocation_realm: tcl_dialect::model::InvocationRealm,
) -> Option<(CommandSourceUnavailability, AvailabilityOrigin)> {
    let first = original.first()?;
    let image = first.image();
    let config = input.lexer_config();
    let context = input.context_registry();
    let occurrence = analysis.original_vendor_source_name_in_source(image, config, first.span())?;
    if occurrence.original_words() != original
        || occurrence.name_input().policy() != input.vendor_source_policy()?
    {
        return None;
    }
    let head = occurrence.name_input();
    let written = std::str::from_utf8(
        head.literal_units(tcl_syntax::naming::VendorSourceNamePurpose::CommandHead)?,
    )
    .ok()?;
    let (descriptor, paths) = if binding.has_original_lookup_snapshot() {
        binding.original_authored_unavailable_command(
            tokens,
            first,
            &context,
            invocation_realm,
            written,
        )?
    } else {
        let coordinate = head.root_command_coordinate()?;
        let descriptors = context
            .commands()
            .command_names_in_any_dialect()
            .filter(|name| {
                tcl_syntax::naming::qualify("::", name).as_bytes() == coordinate.as_bytes()
            })
            .filter_map(|name| context.command_source_unavailability(name, invocation_realm))
            .collect::<Vec<_>>();
        let [descriptor] = descriptors.as_slice() else {
            return None;
        };
        if let Some(body) = occurrence.body_origin() {
            body.matches_child(&context, original).then_some(())?;
        } else {
            let root = tcl_lexer::native_script_words_in(
                (*image).clone(),
                tcl_lexer::Span::new(0, u32::try_from(image.bytes().len()).ok()?),
                config,
            )
            .ok()?;
            if root.fatal_tail.is_some()
                || !root
                    .commands
                    .iter()
                    .any(|command| command.words == original)
            {
                return None;
            }
        }
        (descriptor.clone(), vec![vec![coordinate]])
    };
    let barriers = if let Some(body) = occurrence.body_origin() {
        super::vendor_source_barriers::capture_in_script_body(&context, head, &paths, body)?
    } else {
        super::vendor_source_barriers::capture(&context, head, &paths)?
    };
    Some((
        descriptor,
        AvailabilityOrigin::Hosted {
            occurrence: occurrence.clone(),
            paths,
            barriers: Box::new(barriers),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    fn last_receipt(
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<OriginalSourceCommandAvailability> {
        let config = analysis.resolved_input.as_ref()?.lexer_config();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).pop()?;
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        analysis
            .retained_command_realm()?
            .stamp_original_tokens(&mut tokens);
        original_source_command_availability(source, analysis, &tokens)
    }

    #[test]
    fn unavailable_command_source_owns_native_words_context_and_shadow_barriers() {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        let source = "::dict get {a 1} a";
        let mut analysis = Analyser::new().analyse(source, "tcl8.4");
        let receipt = last_receipt(source, &analysis).unwrap();
        assert_eq!(receipt.domain(), CommandSourceAvailabilityDomain::Native);
        assert_eq!(receipt.descriptor().command(), "dict");
        assert!(receipt.native_lookup().is_some());
        assert_eq!(receipt.original_head().try_text().unwrap(), "::dict");
        assert!(receipt.matches_analysis(&analysis));
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        analysis.diagnostics.clear();
        assert!(receipt.matches_analysis(&analysis));
        assert!(last_receipt(source, &analysis).is_some());
        assert!(
            !receipt.matches_source_input(&SourceImage::native(source.as_bytes()), receipt.input())
        );
        assert!(!receipt.matches_source_input(
            &SourceImage::document("::dict get {a 2} a"),
            receipt.input()
        ));
        let mut config = receipt.input().lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let foreign = ResolvedAnalysisInput::new(
            receipt.input().analyser_profile(),
            receipt.input().unit_profile(),
            receipt.input().context_registry(),
            config,
        );
        assert!(!receipt.matches_source_input(receipt.original_head().image(), &foreign));
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let foreign = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            receipt.input().lexer_config(),
        );
        assert!(!receipt.matches_source_input(receipt.original_head().image(), &foreign));
        for source in ["dict get {a 1} a", "::dict get {a 1} a"] {
            assert!(last_receipt(source, &Analyser::new().analyse(source, "tcl8.6")).is_none());
        }
        for source in [
            "proc dict {args} {}; dict get {a 1} a",
            "rename set dict; dict x",
            "set cmd dict; $cmd get {a 1} a",
        ] {
            assert!(
                last_receipt(source, &Analyser::new().analyse(source, "tcl8.4")).is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn unavailable_hosted_literal_retains_source_policy_and_separate_loader_metadata() {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        let source = "rename a b";
        let analysis = Analyser::new().analyse(source, "f5-irules");
        let receipt = last_receipt(source, &analysis).unwrap();
        assert_eq!(receipt.domain(), CommandSourceAvailabilityDomain::Hosted);
        assert!(receipt.native_lookup().is_none());
        assert!(receipt.hosted_input().is_some());
        assert_eq!(
            receipt.descriptor().kind(),
            tcl_registry::model::CommandSourceUnavailabilityKind::RuleLoaderRefused
        );
        assert!(receipt.matches_analysis(&analysis));
        assert!(last_receipt(source, &Analyser::new().analyse(source, "f5-iapps")).is_none());
        let nested = "when RULE_INIT {rename a b}";
        let analysis = Analyser::new().analyse(nested, "f5-irules");
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|d| d.code == tcl_core_types::DiagCode::Irule2004)
            .unwrap();
        let receipt = diagnostic.command_availability().unwrap();
        assert_eq!(receipt.original_head().try_text().unwrap(), "rename");
        assert_eq!(receipt.domain(), CommandSourceAvailabilityDomain::Hosted);
        assert!(receipt.matches_analysis(&analysis));
    }

    #[test]
    fn unavailable_logical_source_requires_its_complete_positive_domain() {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "logical-unavailability-advice",
            &[],
            "Logical source availability",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )
        .intern();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let source = "dict get {a 1} a";
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let receipt = last_receipt(source, &analysis).unwrap();
        assert_eq!(receipt.domain(), CommandSourceAvailabilityDomain::Logical);
        assert!(receipt.native_lookup().is_none());
        assert!(receipt.hosted_input().is_none());
        assert_eq!(receipt.input(), &input);
        assert!(receipt.matches_analysis(&analysis));
        let unknown = Analyser::new().analyse(source, "unknown-unavailability-domain");
        assert!(last_receipt(source, &unknown).is_none());
    }
}
