// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hosted conditional source roles share the catalogue barrier owner.

use super::VendorRegistryInvocationShape;
use super::source_structure::{trace_remote_source_flag, trace_remote_source_option};
use crate::command_binding::{VendorCatalogueSourceCandidate, VendorCatalogueSourceObligation};
use crate::ir::CommandTokens;
use crate::signature_scan::vendor_name::VendorSourceNameInput;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::model::ContextRegistry;

/// Authored hosted Registry shape with explicit source applicability obligations.
/// This is suitable for MAY source advice only, not handler effects or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalConditionalVendorRegistryMetadata {
    catalogue: VendorCatalogueSourceCandidate,
    shape: VendorRegistryInvocationShape,
    authored_barriers: super::VendorSourceCatalogueBarriers,
}
impl OriginalConditionalVendorRegistryMetadata {
    /// Actual full context and purpose-selected hosted source schema.
    #[must_use]
    pub const fn shape(&self) -> &VendorRegistryInvocationShape {
        &self.shape
    }
    /// Separately retained source-written barriers and unknown mutation
    /// residuals. Neither supplies a current runtime publication or Native slot.
    #[must_use]
    pub const fn authored_barriers(&self) -> &super::VendorSourceCatalogueBarriers {
        &self.authored_barriers
    }

    /// Authored coordinate and unknown/alternate applicability obligations.
    #[must_use]
    pub fn obligations(&self) -> &[VendorCatalogueSourceObligation] {
        self.catalogue.obligations()
    }
    /// Separate vendor source coordinates, without a native slot projection.
    #[must_use]
    pub const fn catalogue(&self) -> &VendorCatalogueSourceCandidate {
        &self.catalogue
    }
    /// Exact source/channel and full configuration correspondence.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.catalogue.matches_source(image, config)
    }
    /// Same independently selected Registry semantic store.
    #[must_use]
    pub fn matches_registry(&self, registry: &tcl_registry::CommandRegistry) -> bool {
        self.catalogue.matches_registry(registry)
    }
}
/// Guard hosted source metadata with the same cell-barrier classifier used by
/// Native source cards, while retaining vendor input and coordinate purposes.
#[must_use]
pub fn original_conditional_vendor_registry_metadata(
    context: &ContextRegistry,
    tokens: &CommandTokens,
    head: &VendorSourceNameInput,
    original: &[NativeWord],
) -> Option<OriginalConditionalVendorRegistryMetadata> {
    let shape = trace_remote_source_option(
        "metadata.shape",
        tokens.argv.first().map(|word| word.start()),
        super::vendor_registry_invocation_shape(context, head, original),
    )?;
    let catalogue = trace_remote_source_option(
        "metadata.lookup_catalogue",
        tokens.argv.first().map(|word| word.start()),
        tokens
            .source_binding
            .as_ref()?
            .original_vendor_catalogue_source_candidate(tokens, head, original, context.commands()),
    )?;
    if !trace_remote_source_flag(
        "metadata.catalogue_shape",
        tokens.argv.first().map(|word| word.start()),
        catalogue.command() == shape.command(),
    ) {
        return None;
    }
    let authored_barriers = trace_remote_source_option(
        "metadata.prefix_barriers",
        tokens.argv.first().map(|word| word.start()),
        super::vendor_source_barriers::capture(
            context,
            head,
            &trace_remote_source_option(
                "metadata.prefixes",
                tokens.argv.first().map(|word| word.start()),
                catalogue.selected_source_prefixes(),
            )?,
        ),
    )?;
    Some(OriginalConditionalVendorRegistryMetadata {
        catalogue,
        shape,
        authored_barriers,
    })
}

/// Genuine retained occurrence, optionally beneath an independently selected
/// original script body. Unavailable child lookup stays an explicit premise.
pub(crate) fn original_vendor_occurrence_registry_metadata(
    context: &ContextRegistry,
    tokens: &CommandTokens,
    occurrence: &crate::signature_scan::vendor_name::VendorSourceNameOccurrence,
) -> Option<OriginalConditionalVendorRegistryMetadata> {
    let binding = trace_remote_source_option(
        "metadata.binding",
        tokens.argv.first().map(|word| word.start()),
        tokens.source_binding.as_ref(),
    )?;
    let body = trace_remote_source_option(
        "metadata.body_origin",
        tokens.argv.first().map(|word| word.start()),
        occurrence.body_origin(),
    );
    if body.is_some_and(|body| {
        !trace_remote_source_flag(
            "metadata.current_child",
            tokens.argv.first().map(|word| word.start()),
            body.matches_child(context, occurrence.original_words()),
        )
    }) {
        return None;
    }
    let mut metadata = if trace_remote_source_flag(
        "metadata.lookup_state_present",
        tokens.argv.first().map(|word| word.start()),
        binding.lookup_state.is_some(),
    ) {
        // A present lookup cannot fall through when its own cell classifier
        // rejects the descriptor. The script receipt supplies no other table.
        trace_remote_source_option(
            "metadata.lookup_classifier",
            tokens.argv.first().map(|word| word.start()),
            original_conditional_vendor_registry_metadata(
                context,
                tokens,
                occurrence.name_input(),
                occurrence.original_words(),
            ),
        )?
    } else {
        let body = body?;
        let shape = trace_remote_source_option(
            "metadata.shape",
            tokens.argv.first().map(|word| word.start()),
            super::vendor_registry_invocation_shape(
                context,
                occurrence.name_input(),
                occurrence.original_words(),
            ),
        )?;
        let catalogue = trace_remote_source_option(
            "metadata.body_catalogue",
            tokens.argv.first().map(|word| word.start()),
            binding.original_vendor_script_body_source_candidate(
                context,
                tokens,
                &shape,
                occurrence.site(),
                body,
            ),
        )?;
        let authored_barriers = trace_remote_source_option(
            "metadata.prefix_barriers",
            tokens.argv.first().map(|word| word.start()),
            super::vendor_source_barriers::capture(
                context,
                occurrence.name_input(),
                &trace_remote_source_option(
                    "metadata.prefixes",
                    tokens.argv.first().map(|word| word.start()),
                    catalogue.selected_source_prefixes(),
                )?,
            ),
        )?;
        OriginalConditionalVendorRegistryMetadata {
            catalogue,
            shape,
            authored_barriers,
        }
    };
    if let Some(body) = body {
        metadata.catalogue.retain_script_body_origin(body);
        metadata.authored_barriers = trace_remote_source_option(
            "metadata.body_barriers",
            tokens.argv.first().map(|word| word.start()),
            super::vendor_source_barriers::capture_in_script_body(
                context,
                occurrence.name_input(),
                &trace_remote_source_option(
                    "metadata.retained_prefixes",
                    tokens.argv.first().map(|word| word.start()),
                    metadata.catalogue.selected_source_prefixes(),
                )?,
                body,
            ),
        )?;
    }
    Some(metadata)
}

#[cfg(test)]
mod script_body_metadata_tests {
    use super::*;
    use crate::command_binding::VendorCatalogueSourceObligation;

    #[test]
    fn original_script_child_without_lookup_keeps_its_sealed_source_premises() {
        // Implementation contract: naming.vendor.original-source-context-input
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
        let source = "proc f {} { ILX::call $h m }";
        let analysis = crate::analyser::Analyser::new().analyse(source, "f5-irules");
        let offset = u32::try_from(source.find("ILX::call").unwrap()).unwrap();
        let occurrence = analysis
            .original_vendor_source_names()
            .find(|occurrence| {
                occurrence.site().offset == offset
                    && occurrence.original_words().first()
                        == Some(occurrence.name_input().original_word())
            })
            .unwrap();
        let config = analysis.body_lexer_config.unwrap();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let (_, segment) =
            crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                source, &analysis, offset,
            )
            .unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        analysis
            .retained_command_realm()
            .unwrap()
            .stamp_original_tokens(&mut tokens);
        // This control removes only the modeled child lookup point. The whole
        // original source carrier and genuine issued body ancestry stay owned.
        tokens.source_binding.as_mut().unwrap().lookup_state = None;
        let metadata =
            original_vendor_occurrence_registry_metadata(&context, &tokens, occurrence).unwrap();
        for obligation in [
            VendorCatalogueSourceObligation::AuthoredSourceCoordinates,
            VendorCatalogueSourceObligation::UnknownCurrentLookup,
            VendorCatalogueSourceObligation::UnknownSourceNamespace,
        ] {
            assert!(metadata.obligations().contains(&obligation));
        }
        assert!(
            original_conditional_vendor_registry_metadata(
                &context,
                &tokens,
                occurrence.name_input(),
                occurrence.original_words(),
            )
            .is_none(),
            "a raw vector without issued body provenance cannot fill absent lookup"
        );
        let changed_context = tcl_registry::model::ingress::static_context_for("f5-iapps");
        assert!(
            original_vendor_occurrence_registry_metadata(changed_context, &tokens, occurrence)
                .is_none()
        );
        let mut changed_source = tokens.clone();
        changed_source.source_binding = None;
        assert!(
            original_vendor_occurrence_registry_metadata(&context, &changed_source, occurrence)
                .is_none()
        );
    }
}
