// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reference-free definition prefixes of the fresh authored Jim root.

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, NamePolicyProtocol, NativeWord,
    OriginalJimStockClass, SourceCommandBindings,
};
use crate::command_binding::declaration_layout::{
    OriginalDiagnosticFrameEntry, original_declaration_layouts,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalJimFreshReferencePrefix {
    owner: Arc<crate::command_binding::OriginalSourceAnalysisOwner>,
    root: Arc<OriginalDiagnosticFrameEntry>,
    definitions: Vec<ReferencePreservingDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReferencePreservingDefinition {
    Class(Arc<OriginalJimStockClass>),
    Method(
        Arc<OriginalJimStockClass>,
        Arc<super::method::OriginalJimStockMethod>,
    ),
}

impl ReferencePreservingDefinition {
    fn preserves_references(&self) -> bool {
        match self {
            Self::Class(class) | Self::Method(class, _) => {
                class.recipe.publication_preserves_references()
            }
        }
    }

    fn words(&self) -> &[NativeWord] {
        match self {
            Self::Class(class) => &class.original,
            Self::Method(_, method) => method.original_words(),
        }
    }
}

impl OriginalJimFreshReferencePrefix {
    pub(super) fn capture(
        bindings: &SourceCommandBindings,
        world: &ModuleCommandBindings,
        input: &crate::analyser::ResolvedAnalysisInput,
        policy: NamePolicyProtocol,
    ) -> Option<Arc<Self>> {
        let owner = Arc::clone(bindings.original_analysis_owner()?);
        (owner.source_arguments_empty && owner.matches(bindings, input)).then_some(())?;
        let origin = bindings.root_origin.as_ref()?;
        if !matches!(
            origin.kind(),
            crate::command_binding::SourceOriginKind::Authored(_)
        ) || world
            .current_source_origin
            .as_ref()
            .is_none_or(|source| !Arc::ptr_eq(source, origin))
            || world.baseline.source_point_entry.is_some()
            || world.baseline.native_entry.is_some()
            || world.baseline.logical_source_input.is_some()
            || world.baseline.vendor_source_input.is_some()
            || world.baseline.hosted_execution_context.is_some()
            || world.baseline.unknown_entry
        {
            return None;
        }
        let plan = origin
            .original_script_words_in(
                tcl_lexer::Span::new(0, u32::try_from(origin.source_image().len()).ok()?),
                input.lexer_config(),
                tcl_lexer::SourceSegmentationPurpose::WholeInput,
            )
            .ok()?;
        if plan.fatal_tail.is_some()
            || plan.commands.is_empty()
            || plan.commands.len() > super::super::original_name_value::MAX_ORIGINS
        {
            return None;
        }
        let candidates = current_definitions(world, policy)?;
        let mut definitions = Vec::with_capacity(plan.commands.len());
        for command in &plan.commands {
            let mut selected = candidates.iter().filter(|definition| {
                definition.preserves_references() && definition.words() == command.words.as_slice()
            });
            let first = selected.next()?;
            if selected.any(|other| other != first) {
                return None;
            }
            definitions.push(first.clone());
        }
        let first = definitions.first()?.words();
        let root = fresh_root_entry(bindings, world, first, input, policy)?;
        Some(Arc::new(Self {
            owner,
            root,
            definitions,
        }))
    }

    pub(super) fn matches(
        &self,
        bindings: &SourceCommandBindings,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        bindings
            .original_analysis_owner()
            .is_some_and(|owner| Arc::ptr_eq(owner, &self.owner))
            && self.owner.matches(bindings, input)
            && bindings
                .root_origin
                .as_ref()
                .is_some_and(|source| Arc::ptr_eq(source, &self.root.source().origin))
            && !self.definitions.is_empty()
    }
}

fn current_definitions(
    world: &ModuleCommandBindings,
    policy: NamePolicyProtocol,
) -> Option<Vec<ReferencePreservingDefinition>> {
    let mut definitions = Vec::new();
    for key in world.bindings.keys() {
        let slot = world.original_byte_slot_for_key(key, policy)?;
        let name = super::entry::current_stock_name(world, &slot, policy)?;
        if !name.owner.recipe.publication_preserves_references()
            || name
                .owner
                .recipe
                .object_command_footprint()
                .may_be_named_by(name.bytes())
        {
            return None;
        }
        let class = ReferencePreservingDefinition::Class(Arc::clone(&name.owner));
        if !definitions.contains(&class) {
            definitions.push(class);
        }
        if let Some(method) = &name.method {
            let method =
                ReferencePreservingDefinition::Method(Arc::clone(&name.owner), Arc::clone(method));
            if !definitions.contains(&method) {
                definitions.push(method);
            }
        }
    }
    Some(definitions)
}

fn fresh_root_entry(
    bindings: &SourceCommandBindings,
    world: &ModuleCommandBindings,
    first: &[NativeWord],
    input: &crate::analyser::ResolvedAnalysisInput,
    policy: NamePolicyProtocol,
) -> Option<Arc<OriginalDiagnosticFrameEntry>> {
    let origin = bindings.root_origin.as_ref()?;
    let site = CommandAllocationSite {
        source: Arc::clone(origin),
        offset: first.first()?.span().start(),
    };
    let rows = original_declaration_layouts(bindings.declaration_layouts.get(&site)?)?;
    let mut selected = None;
    for row in rows {
        let OriginalDiagnosticFrameEntry::RootScript {
            source,
            frame,
            namespace,
        } = row.entry.as_ref()
        else {
            return None;
        };
        let initial = &row.snapshot.state;
        super::bounded_source_entry(initial, policy)?;
        if !Arc::ptr_eq(&source.origin, origin)
            || source.base() != 0
            || source.text.bytes() != origin.source_image().bytes()
            || frame.layout() != &crate::var_resolve::VariableExecutionFrame::Global
            || namespace != &world.source_root_namespace_key()?
            || row.config != input.lexer_config()
            || initial.baseline.source_point_entry.is_some()
            || initial.baseline.metadata_context != world.baseline.metadata_context
            || initial.baseline.execution_name_policy != world.baseline.execution_name_policy
            || !initial.bindings.is_empty()
            || !initial.allocation_counts.is_empty()
            || origin
                .original_lexical_command(&row.words, site.offset, row.config)?
                .native_words()
                != first
        {
            return None;
        }
        if selected
            .as_ref()
            .is_some_and(|previous| previous != &row.entry)
        {
            return None;
        }
        selected = Some(Arc::clone(&row.entry));
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_incoming_values_cannot_donate_reference_freshness_through_the_memo() {
        tcl_spectcl::core_surfaces::ensure();
        let text = "class C {x 0}; C method m {} {missing_after_incoming $self $x}";
        let analysis = crate::analyser::Analyser::new().analyse(text, "jim");
        let input = analysis.resolved_input.as_ref().unwrap();
        let registry = input.borrowed_context_registry().commands();
        let incoming = ["existing_reference".to_owned()];
        for names in [&[][..], incoming.as_slice(), &[][..]] {
            let bindings = SourceCommandBindings::analyse_in_frame_with_options(
                text,
                &crate::var_resolve::VariableExecutionFrame::Global,
                input.lexer_config(),
                registry,
                super::super::super::SourceAnalysisOptions {
                    metadata_context:
                        crate::registry_invocation::InvocationMetadataInput::SuppliedSource(Some(
                            input,
                        )),
                    invocation_dialect: Some(
                        super::super::super::source_analysis_entry::source_input_dialect(input),
                    ),
                    incoming_formals: names,
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..super::super::super::SourceAnalysisOptions::default()
                },
            );
            let owner = bindings.original_analysis_owner().unwrap();
            assert_eq!(owner.source_arguments_empty, names.is_empty());
            let world = &bindings.final_state;
            let policy = world
                .baseline
                .execution_name_policy
                .unwrap()
                .native_recipe()
                .unwrap();
            let prefix = OriginalJimFreshReferencePrefix::capture(&bindings, world, input, policy);
            assert_eq!(prefix.is_some(), names.is_empty());
            if !names.is_empty() {
                assert!(!world.source_variables.contents_presence.is_empty());
            }
        }
    }
}
