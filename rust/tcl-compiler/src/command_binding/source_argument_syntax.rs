// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original argv syntax is independent of dispatch and compiler visitation.

use super::{Arc, CommandAllocationSite, SourceCommandBindings, SourceInvocationBinding};
use crate::ir::CommandTokens;
use tcl_lexer::LexerConfig;

/// A sealed vector from an actual kernel-segmented command, or one of its
/// complete checked argv substitutions. No lookup world, descriptor, variable
/// frame, value, entered handler, Normal completion or compiler visit follows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalSourceArgumentSyntax {
    owner: Arc<super::OriginalSourceAnalysisOwner>,
    parent: Arc<CommandTokens>,
    site: CommandAllocationSite,
    words: Arc<[crate::ir::WordExpr]>,
    config: LexerConfig,
}

impl OriginalSourceArgumentSyntax {
    pub(super) fn child(&self, tokens: &CommandTokens) -> Option<Self> {
        let children =
            crate::word_subst::checked_argument_lifted_calls(&self.parent, self.owner.config)?;
        let mut matching = children
            .into_iter()
            .filter_map(|child| child.tokens)
            .filter(|child| child.words() == tokens.words());
        matching.next()?;
        if matching.next().is_some()
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        let config = self.owner.config.nested();
        let site = CommandAllocationSite {
            source: Arc::clone(&self.site.source),
            offset: tokens.words().first()?.source().span.start(),
        };
        site.source
            .original_lexical_command(tokens.words(), site.offset, config)?;
        Some(Self {
            owner: Arc::clone(&self.owner),
            parent: Arc::clone(&self.parent),
            site,
            words: Arc::from(tokens.words()),
            config,
        })
    }
}

pub(crate) struct OriginalSourceArgumentSyntaxView<'a>(&'a OriginalSourceArgumentSyntax);
impl OriginalSourceArgumentSyntaxView<'_> {
    pub(crate) fn image(&self) -> &tcl_lexer::SourceImage {
        self.0.site.source.source_image()
    }
    pub(crate) fn words(&self) -> &[crate::ir::WordExpr] {
        &self.0.words
    }
    pub(crate) fn offset(&self) -> u32 {
        self.0.site.offset
    }
    pub(crate) fn config(&self) -> LexerConfig {
        self.0.config
    }
}

impl SourceInvocationBinding {
    pub(crate) fn original_source_argument_syntax<'a>(
        &'a self,
        tokens: &CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalSourceArgumentSyntaxView<'a>> {
        let syntax = self.original_argument_syntax.as_deref()?;
        if tokens.source_binding.as_ref() != Some(self)
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || tokens.words() != syntax.words.as_ref()
            || !self
                .original_analysis_owner
                .as_ref()
                .is_some_and(|owner| Arc::ptr_eq(owner, &syntax.owner))
            || !Arc::ptr_eq(&syntax.owner.source, &syntax.site.source)
        {
            return None;
        }
        let original = syntax.owner.metadata.source_analysis_input()?;
        if original != input && original != &input.for_nested_source() {
            return None;
        }
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry,
            original,
            syntax.config,
            Some(original.unit_profile()),
        )?;
        syntax.site.source.original_lexical_command(
            tokens.words(),
            syntax.site.offset,
            syntax.config,
        )?;
        Some(OriginalSourceArgumentSyntaxView(syntax))
    }
}

impl SourceCommandBindings {
    /// Record syntax from the real segment plan even if evaluation later stops.
    /// This is a parser inventory, never an inventory of reached operations.
    pub(super) fn retain_original_argument_syntax_commands(
        &mut self,
        base: u32,
        segments: &[crate::segmenter::SegmentedCommand],
        state: &super::ModuleCommandBindings,
        context: &super::SourceExecutionContext<'_>,
    ) {
        // This first purpose covers the actual whole-input parser plan only.
        // Body/region issuers need their own explicit command extent receipt.
        if state
            .baseline
            .source_point_entry
            .as_ref()
            .is_some_and(|entry| entry.conditional_region().is_some())
            || base != 0
            || context.depth != 0
            || context.source_purpose != tcl_lexer::SourceSegmentationPurpose::WholeInput
            || self
                .original_analysis_owner
                .as_ref()
                .and_then(|owner| owner.metadata.source_analysis_input())
                .is_none()
        {
            return;
        }
        let config = context.config;
        let Some(origin) = state
            .current_source_origin
            .as_ref()
            .filter(|origin| self.root_origin.as_ref() == Some(*origin))
        else {
            return;
        };
        for segment in segments {
            let site = CommandAllocationSite {
                source: Arc::clone(origin),
                offset: segment.span.start(),
            };
            self.original_argument_syntax_commands
                .entry(site)
                .and_modify(|prior| {
                    if *prior != Some(config) {
                        *prior = None;
                    }
                })
                .or_insert(Some(config));
        }
    }

    /// Region producers must retain only their issuer-owned command inventory.
    /// A full source image/current origin alone never grants this predicate.
    pub(super) fn owns_original_argument_syntax_command(
        &self,
        site: &CommandAllocationSite,
        config: LexerConfig,
    ) -> bool {
        self.original_argument_syntax_commands.get(site) == Some(&Some(config))
    }

    pub(super) fn original_argument_syntax_for_tokens(
        &self,
        tokens: &CommandTokens,
        config: LexerConfig,
    ) -> Option<OriginalSourceArgumentSyntax> {
        let owner = self.original_analysis_owner.as_ref()?;
        if tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || owner.config != config
        {
            return None;
        }
        let site = CommandAllocationSite {
            source: Arc::clone(&owner.source),
            offset: tokens.words().first()?.source().span.start(),
        };
        if !self.owns_original_argument_syntax_command(&site, config) {
            return None;
        }
        owner
            .source
            .original_lexical_command(tokens.words(), site.offset, config)?;
        let mut parent = tokens.clone();
        parent.source_binding = None;
        parent.nested_bindings.clear();
        Some(OriginalSourceArgumentSyntax {
            owner: Arc::clone(owner),
            parent: Arc::new(parent),
            site,
            words: Arc::from(tokens.words()),
            config,
        })
    }
}

impl SourceInvocationBinding {
    /// All genuine original Direct root pre-argv worlds for a conditional
    /// source argument query. This grants no handler/Normal/compiler authority.
    /// A child is selected in those worlds only after the caller separately
    /// proves every intervening operand preserves lookup and observer state.
    pub(crate) fn original_direct_argument_lookups(
        &self,
        parent: &CommandTokens,
        child: Option<&CommandTokens>,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<super::OriginalCompilationLookupAdvice>> {
        self.original_source_argument_syntax(parent, input, registry)?;
        if self.original_compiler_words.is_some()
            || self.compiler_lookup_state.is_some()
            || self.compiler_policy.is_some()
        {
            return None;
        }
        let syntax = self.original_argument_syntax.as_deref()?;
        if syntax.words.as_ref() != syntax.parent.words() {
            return None;
        }
        let actual = if let Some(child) = child {
            let binding = child.source_binding.as_ref()?;
            binding.original_source_argument_syntax(child, input, registry)?;
            let child_syntax = binding.original_argument_syntax.as_deref()?;
            if !Arc::ptr_eq(&syntax.owner, &child_syntax.owner)
                || !Arc::ptr_eq(&syntax.parent, &child_syntax.parent)
            {
                return None;
            }
            child
        } else {
            parent
        };
        let site = CommandAllocationSite {
            source: Arc::clone(&syntax.site.source),
            offset: actual.words().first()?.source().span.start(),
        };
        let config = if child.is_some() {
            syntax.config.nested()
        } else {
            syntax.config
        };
        let rows = super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let mut lookups = Vec::new();
        for row in rows {
            direct_original_root_argument_row(row, syntax, parent, input)?;
            let original = syntax.owner.metadata.source_analysis_input()?;
            let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
                registry,
                original,
                syntax.config,
                Some(original.unit_profile()),
            )?;
            if row.snapshot.state.baseline.registry_snapshot
                != Some(registry.snapshot().semantic_key())
                || row
                    .snapshot
                    .state
                    .retained_original_source_purpose(metadata, registry)?
                    != super::OriginalInvocationSourcePurpose::AuthoredSimulation
            {
                return None;
            }
            let advice = super::original_site_operand_layout_advice(
                &site,
                actual,
                &row.snapshot,
                &row.namespace,
                config,
            )?;
            let [target] = advice.targets() else {
                return None;
            };
            if advice.has_unclosed_source_coverage()
                || target.kind != super::BindingKind::Builtin
                || !target.registry_backed
                || !target.prepended.is_empty()
                || target.implementation_generation != 0
                || target.runtime_implementation_generation.is_some()
                || !row.snapshot.state.retained_target_is_current(target)
            {
                return None;
            }
            lookups.push(advice);
        }
        (!lookups.is_empty()).then_some(lookups)
    }
}

fn direct_original_root_argument_row(
    row: &super::declaration_layout::DeclarationLayoutObservation,
    syntax: &OriginalSourceArgumentSyntax,
    parent: &CommandTokens,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<()> {
    let super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
        source,
        frame,
        namespace,
    } = row.entry.as_ref()
    else {
        return None;
    };
    let state = &row.snapshot.state;
    let variables = &state.source_variables;
    let original = syntax.owner.metadata.source_analysis_input()?;
    if (original != input && original != &input.for_nested_source())
        || row.words.as_ref() != parent.words()
        || row.config != syntax.config
        || !Arc::ptr_eq(&source.origin, &syntax.owner.source)
        || state.current_source_origin.as_ref() != Some(&syntax.owner.source)
        || frame.layout() != &crate::var_resolve::VariableExecutionFrame::Global
        || frame != &state.variable_frame
        || namespace != &row.namespace
        || state.baseline.metadata_context.source_analysis_input() != Some(original)
        || state.baseline.native_entry.is_some()
        || state.baseline.unknown_entry
        || state.baseline.native_compilation.mode
            != tcl_registry::native_compilation::NativeCompilationMode::Direct
        || state.has_opaque_domain()
        || state.unclosed_command_world
        || state.source_step_observed()
        || state.source_execution_observed(None)
        || !state.command_observers.is_quiet()
        || variables.dynamic_traces
        || !variables.traced.is_empty()
        || !variables.trace_registrations.is_empty()
        || !variables.trace_registration_receivers.is_empty()
        || !variables.upvar_aliases.is_empty()
        || !variables.alias_bindings.is_empty()
        || !variables.namespace_alias_bindings.is_empty()
        || !variables.name_alias_bindings.is_empty()
        || !variables.namespace_name_alias_bindings.is_empty()
        || !variables.globals.is_empty()
        || !variables.ns_vars.is_empty()
        || !variables.instance_vars.is_empty()
        || !variables.possible_trace_registrations.is_empty()
        || !variables.untracked_traces.is_empty()
    {
        return None;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(
        source: &str,
    ) -> (
        SourceCommandBindings,
        CommandTokens,
        crate::analyser::ResolvedAnalysisInput,
    ) {
        let context = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .unwrap()
            .default_context_registry();
        let profile = context.commands().profile().unwrap();
        let config = LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let entry = super::super::SourceAnalysisEntry::for_supplied_source(
            context.commands(),
            &input,
            config,
            Some(profile),
        );
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            context.commands(),
            entry.options(),
        );
        let image = tcl_lexer::SourceImage::document(source);
        let commands =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap();
        let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &commands[0]);
        bindings.stamp_original_tokens(&mut tokens);
        (bindings, tokens, input)
    }

    #[test]
    fn direct_argument_lookup_refuses_each_typed_alias_map() {
        let (_, tokens, input) = fixture("interp eval $child [list puts $raw]");
        let binding = tokens.source_binding.as_ref().unwrap();
        let syntax = binding.original_argument_syntax.as_deref().unwrap();
        let row = super::super::declaration_layout::original_declaration_layouts(
            binding.declaration_layout_observations.as_deref().unwrap(),
        )
        .unwrap()
        .next()
        .unwrap();
        assert!(direct_original_root_argument_row(row, syntax, &tokens, &input).is_some());
        let target = crate::var_resolve::resolve_literal_place(
            "foreign",
            &row.snapshot.state.source_variables,
            false,
            input.borrowed_context_registry().commands(),
        );
        for index in 0..4 {
            let mut changed = row.clone();
            let snapshot = Arc::make_mut(&mut changed.snapshot);
            let variables = Arc::make_mut(&mut snapshot.state.source_variables);
            let aliases = match index {
                0 => &mut variables.alias_bindings,
                1 => &mut variables.namespace_alias_bindings,
                2 => &mut variables.name_alias_bindings,
                _ => &mut variables.namespace_name_alias_bindings,
            };
            aliases.insert("linked", target.clone());
            assert!(
                direct_original_root_argument_row(&changed, syntax, &tokens, &input).is_none(),
                "typed alias map {index}"
            );
        }
    }

    #[test]
    fn direct_argument_geometry_survives_an_undefined_read_without_a_compiler_visit() {
        let (_, tokens, input) = fixture("puts [list $arr($idx)]");
        let registry = input.borrowed_context_registry().commands();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.original_compiler_words.is_none());
        let syntax = binding
            .original_source_argument_syntax(&tokens, &input, registry)
            .unwrap();
        assert_eq!(syntax.words(), tokens.words());
        let child = crate::word_subst::checked_argument_lifted_calls(&tokens, input.lexer_config())
            .unwrap()
            .into_iter()
            .find_map(|child| child.tokens)
            .unwrap();
        assert!(
            child
                .source_binding
                .as_ref()
                .unwrap()
                .original_source_argument_syntax(&child, &input, registry)
                .is_some()
        );
        let mut changed = tokens.clone();
        changed.argv_texts[1].push('x');
        assert!(
            changed
                .source_binding
                .as_ref()
                .unwrap()
                .original_source_argument_syntax(&changed, &input, registry)
                .is_none()
        );
        let mut changed = tokens.clone();
        changed.synthetic = Some(crate::ir::SyntheticMarker::UnseenCall);
        assert!(
            changed
                .source_binding
                .as_ref()
                .unwrap()
                .original_source_argument_syntax(&changed, &input, registry)
                .is_none()
        );
        let foreign = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::new(
                input
                    .context_registry()
                    .with_command_store(Arc::clone(registry)),
            ),
            input.lexer_config(),
        );
        assert!(
            binding
                .original_source_argument_syntax(&tokens, &foreign, registry)
                .is_none()
        );
    }

    #[test]
    fn raw_image_and_analysis_owner_cannot_replace_the_recorded_command_scope() {
        let (mut bindings, mut tokens, input) = fixture("puts [list $arr($idx)]");
        bindings.original_argument_syntax_commands.clear();
        bindings.stamp_original_tokens(&mut tokens);
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_analysis_owner
                .is_some()
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_source_argument_syntax(
                    &tokens,
                    &input,
                    input.borrowed_context_registry().commands()
                )
                .is_none()
        );
    }
}

#[cfg(test)]
mod purpose_controls {
    use super::*;

    #[test]
    fn a_nested_script_parser_purpose_cannot_borrow_whole_input_syntax_scope() {
        let source = "puts [list $arr($idx)]";
        let context = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .unwrap()
            .default_context_registry();
        let registry = context.commands();
        let profile = registry.profile().unwrap();
        let config = LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let entry = super::super::SourceAnalysisEntry::for_supplied_source(
            registry,
            &input,
            config,
            Some(profile),
        );
        let image = tcl_lexer::SourceImage::document(source);
        let bindings = SourceCommandBindings::analyse_image_in_frame_with_purpose(
            &image,
            &crate::var_resolve::VariableExecutionFrame::Global,
            config,
            registry,
            entry.options(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .unwrap();
        let commands =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap();
        let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &commands[0]);
        bindings.stamp_original_tokens(&mut tokens);
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_analysis_owner
                .is_some()
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_source_argument_syntax(&tokens, &input, registry)
                .is_none()
        );
    }
}
