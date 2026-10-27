// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional source-cut body layouts, without dispatch or entered-body facts.

use super::super::{
    OriginalCompletedCommandWorld, OriginalSourceAnalysisOwner,
    OriginalSourceCutBodyEntryProjection, SourceInvocationBinding,
};
use super::{SourceCommandBindings, original_declaration_layouts};
use crate::analyser::ResolvedAnalysisInput;
use crate::ir::{CommandTokens, WordExpr};
use crate::registry_invocation::OriginalSourceScriptBody;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) struct OriginalSourceCutBodyRows {
    owner: Arc<OriginalSourceAnalysisOwner>,
    commands: BTreeMap<u32, (Arc<[WordExpr]>, SourceInvocationBinding)>,
}

impl OriginalSourceCutBodyRows {
    pub(crate) fn capture(
        bindings: &SourceCommandBindings,
        header: &CommandTokens,
        body: &OriginalSourceScriptBody,
        projection: &OriginalSourceCutBodyEntryProjection,
        input: &ResolvedAnalysisInput,
    ) -> Option<Self> {
        let owner = bindings.original_analysis_owner.as_ref()?;
        let site = header.source_binding.as_ref()?.invocation_site()?;
        let registry = input.borrowed_context_registry().commands();
        if !projection.matches_analysis_owner(owner)
            || !bindings.owns_original_analysis_attachment(header)
            || !Arc::ptr_eq(&owner.source, &site.source)
            || bindings.lexer_config != Some(input.lexer_config())
            || !body.matches_source(site.source.source_image(), input.lexer_config())
            || !body.matches_context(input.borrowed_context_registry())
        {
            return None;
        }
        let rows = original_declaration_layouts(bindings.declaration_layouts.get(site)?)?;
        let mut agreed = None;
        let mut snapshot = None;
        for row in rows {
            let candidate = super::declared_body(site, header, row, registry)?;
            Self::matches_body(&candidate, body, site)?;
            Self::world_owns_declaration(&candidate, projection.completed_world())?;
            let incoming = projection.snapshot(row, site, &candidate.namespace)?;
            if agreed.as_ref().is_some_and(|old| old != &candidate)
                || snapshot.as_ref().is_some_and(|old| old != &incoming)
            {
                return None;
            }
            agreed = Some(candidate);
            snapshot = Some(incoming);
        }
        let mut derived = SourceCommandBindings {
            root_origin: Some(Arc::clone(&owner.source)),
            lexer_config: Some(input.lexer_config()),
            original_analysis_owner: Some(Arc::clone(owner)),
            ..SourceCommandBindings::default()
        };
        derived.retain_declared_body_layouts_from_snapshot(
            agreed?,
            &snapshot?,
            input.lexer_config(),
            registry,
        );
        Self::from_complete_rows(&derived, body, owner, input)
    }

    fn matches_body(
        candidate: &super::DeclaredProcedureBody,
        body: &OriginalSourceScriptBody,
        site: &super::CommandAllocationSite,
    ) -> Option<()> {
        let content = body.content_span();
        if candidate.declaration != *site
            || !Arc::ptr_eq(&candidate.source.origin, &site.source)
            || candidate.source.base() != content.start()
            || candidate.source.text.bytes()
                != body
                    .original_container()
                    .image()
                    .bytes()
                    .get(content.as_range())?
            || candidate
                .authored_original_body_word
                .as_ref()
                .is_some_and(|word| word != body.original_container())
        {
            return None;
        }
        Some(())
    }

    fn world_owns_declaration(
        body: &super::DeclaredProcedureBody,
        world: &OriginalCompletedCommandWorld,
    ) -> Option<()> {
        let mut actual = world.declarations().filter_map(|publication| {
            let definition = publication.definition()?;
            let allocation = definition.allocation();
            (definition.kind() == super::super::SourceCommandDefinitionKind::Procedure
                && Arc::ptr_eq(&allocation.site.source, &body.declaration.source)
                && allocation.site.offset == body.declaration.offset
                && allocation.namespace == body.namespace)
                .then_some(allocation)
        });
        actual.next()?;
        actual.next().is_none().then_some(())
    }

    fn from_complete_rows(
        derived: &SourceCommandBindings,
        body: &OriginalSourceScriptBody,
        owner: &Arc<OriginalSourceAnalysisOwner>,
        input: &ResolvedAnalysisInput,
    ) -> Option<Self> {
        let context = input.borrowed_context_registry();
        let originals = body.original_commands(context)?;
        let flow = derived.current_declaration_flow_inventory();
        let mut commands = BTreeMap::new();
        for original in originals {
            let site = super::CommandAllocationSite {
                source: Arc::clone(&owner.source),
                offset: original.words.first()?.span().start(),
            };
            let observations = derived.declaration_layouts.get(&site)?;
            let rows = original_declaration_layouts(observations)?;
            for row in rows {
                let actual =
                    site.source
                        .original_lexical_command(&row.words, site.offset, row.config)?;
                if row.config != input.lexer_config()
                    || actual.native_words() != original.words.as_slice()
                {
                    return None;
                }
            }
            let first = original_declaration_layouts(observations)?.next()?;
            let binding = SourceInvocationBinding {
                unknown: true,
                dispatch_site: Some(site.clone()),
                original_analysis_owner: Some(Arc::clone(owner)),
                declaration_layout_observations: Some(Arc::from(observations.as_slice())),
                declaration_flow_inventory: Some(Arc::clone(&flow)),
                lookup_namespace: first.namespace.advisory_key()?,
                lookup_namespace_key: first.namespace.clone(),
                variable_context: Arc::clone(&first.snapshot.state.source_variables),
                variable_frame: first.snapshot.state.variable_frame.clone(),
                ..SourceInvocationBinding::default()
            };
            if commands
                .insert(site.offset, (Arc::clone(&first.words), binding))
                .is_some()
            {
                return None;
            }
        }
        Some(Self {
            owner: Arc::clone(owner),
            commands,
        })
    }

    pub(crate) fn stamp(&self, tokens: &mut CommandTokens) -> Option<()> {
        if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let (words, template) = self.commands.get(&offset)?;
        let binding = tokens.source_binding.as_ref()?;
        if words.as_ref() != tokens.words()
            || binding
                .original_analysis_owner
                .as_ref()
                .is_none_or(|owner| !Arc::ptr_eq(owner, &self.owner))
        {
            return None;
        }
        // Only the exact missing-point issuer state can receive new geometry.
        // Every other retained facet keeps the existing binding terminal.
        let minimal = SourceInvocationBinding {
            original_analysis_owner: binding.original_analysis_owner.clone(),
            original_argument_syntax: binding.original_argument_syntax.clone(),
            original_source_operand_attachment: binding.original_source_operand_attachment.clone(),
            ..SourceInvocationBinding::unknown()
        };
        if binding != &minimal {
            return Some(());
        }
        let binding = tokens.source_binding.as_mut()?;
        binding.dispatch_site.clone_from(&template.dispatch_site);
        binding
            .declaration_layout_observations
            .clone_from(&template.declaration_layout_observations);
        binding
            .declaration_flow_inventory
            .clone_from(&template.declaration_flow_inventory);
        binding
            .lookup_namespace
            .clone_from(&template.lookup_namespace);
        binding
            .lookup_namespace_key
            .clone_from(&template.lookup_namespace_key);
        binding
            .variable_context
            .clone_from(&template.variable_context);
        binding.variable_frame.clone_from(&template.variable_frame);
        Some(())
    }
}
