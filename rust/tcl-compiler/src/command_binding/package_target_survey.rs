// SPDX-License-Identifier: AGPL-3.0-or-later
//! Existing Logical source-point history for an explicitly hypothetical target survey.
use super::*;

/// Immutable source-model premise, never a package database/provision result.
#[derive(Debug, Clone)]
pub(crate) struct OriginalPackageIndexSourceModel {
    _origin: Arc<SourceOriginId>,
    _input: crate::analyser::ResolvedAnalysisInput,
    _words: Arc<[tcl_lexer::NativeWord]>,
    _namespaces: Vec<SourceNamespaceKey>,
    _subject: Option<String>,
    _advice: Arc<OriginalSourceCommandTransitionAdvice>,
    _layouts: Vec<Arc<OriginalCompilationLookupAdvice>>,
}

impl OriginalPackageIndexSourceModel {
    pub(crate) fn same_source_context(&self, parent: &Self) -> bool {
        self._origin == parent._origin
            && self._input == parent._input
            && self._subject == parent._subject
            && self._namespaces == parent._namespaces
    }
}

impl SourceCommandBindings {
    /// Reborrow actual Logical dispatch points and original whole vectors.
    /// This models the declared initial source world only, with all prior
    /// opaque/source/package effects retained. No Native acceptance is changed.
    pub(crate) fn original_package_index_model_at(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        command: &tcl_lexer::NativeScriptCommandWords,
        subject: Option<&str>,
        advice: &Arc<OriginalSourceCommandTransitionAdvice>,
    ) -> Option<OriginalPackageIndexSourceModel> {
        if self.original_logical_source_name_advice_input() != Some(input)
            || !input.has_logical_source_name_context()
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let origin = self.root_origin.as_ref()?;
        let offset = command.words.first()?.span().start();
        if advice.logical_source_input() != Some(input)
            || !advice.matches_source(image, input.lexer_config())
            || !advice.matches_context(input.borrowed_context_registry())
            || advice.original_words() != command.words.as_slice()
            || advice.original_head().logical_input().is_none()
            || advice
                .obligations()
                .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
            || !advice.uncertain_operations().is_empty()
        {
            return None;
        }
        let source = image.try_text().ok()?;
        let segments = crate::segmenter::segment_commands_with_offset_and_config(
            source.get(command.span.as_range())?,
            command.span.start(),
            input.lexer_config().nested(),
        );
        let [segment] = segments.as_slice() else {
            return None;
        };
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(image),
            input.lexer_config(),
            segment,
        );
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset,
        };
        let registry = input.borrowed_context_registry().commands();
        let registry_key = registry.snapshot().semantic_key();
        let points = self
            .dispatch_points_at(offset)
            .filter(|point| !point.declaration_preview)
            .collect::<Vec<_>>();
        if points.is_empty() {
            return None;
        }
        let mut namespaces = Vec::new();
        let mut layouts = Vec::new();
        for point in points {
            let state = &point.state;
            let binding = point.original_dispatch_binding();
            // Existing shared Logical layout projection from this exact point,
            // not a Native declaration row or a fabricated observation.
            let layout = original_site_operand_layout_advice(
                &site,
                &tokens,
                binding.lookup_state.as_ref()?,
                &point.namespace_key,
                input.lexer_config(),
            )?;
            if !layout.closed_logical_source_lookup()
                || layout.targets() != binding.targets.as_slice()
            {
                return None;
            }

            if binding.original_logical_prefix_source_input() != Some(input)
                || state.current_source_origin.as_ref() != Some(origin)
                || state.baseline.registry_snapshot.as_ref() != Some(&registry_key)
                || state.has_opaque_domain()
                || state.source_step_observed()
                || state.opaque_binding_mutation
                || state.unnameable_rebinding_subject
                || !state.command_observers.is_quiet()
                || state.loader_handler_unknown
                || state.tainted_provider_state.contains("*")
                || binding.targets.is_empty()
                || binding.targets.iter().any(|target| {
                    !target.registry_backed
                        || !target.prepended.is_empty()
                        || target.implementation_generation != 0
                        || !tcl_registry::command_descriptor_labels_match(
                            &target.command,
                            advice.command(),
                        )
                        || state.source_execution_observed(target.identity.as_ref())
                        || state.runtime_execution_observed(target.identity.as_ref())
                })
            {
                return None;
            }
            if let Some(subject) = subject
                && (state.packages.contains_key(subject)
                    || state.revoked_loaders.contains(subject)
                    || state.tainted_provider_state.contains(subject))
            {
                return None;
            }
            namespaces.push(point.namespace_key.clone());
            layouts.push(Arc::new(layout));
        }
        namespaces.sort();
        namespaces.dedup();
        Some(OriginalPackageIndexSourceModel {
            _origin: Arc::clone(origin),
            _input: input.clone(),
            _words: Arc::from(command.words.clone()),
            _namespaces: namespaces,
            _subject: subject.map(str::to_owned),
            _advice: Arc::clone(advice),
            _layouts: layouts,
        })
    }
}
