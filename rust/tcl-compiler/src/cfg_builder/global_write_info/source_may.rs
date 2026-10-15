// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original source MAY effects at the actual emitted call.

use crate::ir::{CommandTokens, Module, ProcedureId, Statement};
use crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OriginalSourceCallMayWrites {
    source: std::sync::Arc<crate::command_binding::SourceOriginId>,
    input: crate::analyser::ResolvedAnalysisInput,
    original: CommandTokens,
    emitted: Statement,
    bodies: Vec<ProcedureId>,
    writes: Vec<SignatureSourceVariableOccurrence>,
}

impl OriginalSourceCallMayWrites {
    pub(in crate::cfg_builder) fn for_emission(
        module: &Module,
        registry: &tcl_registry::CommandRegistry,
        original: &CommandTokens,
        emitted: &Statement,
        bodies: Vec<ProcedureId>,
        writes: Vec<SignatureSourceVariableOccurrence>,
    ) -> Option<Self> {
        let input = module.source_metadata_input.as_ref()?;
        input.has_logical_source_name_context().then_some(())?;
        if module.has_dynamic_trace || !module.traced_commands.is_empty()
            || module.has_dynamic_variable_trace || !module.traced_variables.is_empty() { return None; }
        let binding = original.source_binding.as_ref()?;
        let metadata = binding.original_invocation_metadata_for_module(original, module, registry)?;
        (binding.original_invocation_source_purpose(original, metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical).then_some(())?;
        let Statement::Call { tokens: Some(tokens), defs, foreach_groups: None, .. } = emitted else { return None; };
        if tokens != original || original.synthetic.is_some()
            || bodies.iter().any(|body| !module.procedure_ids.owns(*body))
            || writes.iter().any(|place| !place.matches_logical_source(&module.source, input))
        {
            return None;
        }
        for place in &writes {
            let report = place.symbol().checked_c_namespace_key()?;
            let mut emitted_names = std::collections::BTreeSet::new();
            super::insert_outer_name(&mut emitted_names, &report);
            if emitted_names.iter().any(|name| !defs.contains(name)) { return None; }
        }
        Some(Self {
            source: std::sync::Arc::clone(&binding.invocation_site()?.source),
            input: input.clone(), original: original.clone(), emitted: emitted.clone(), bodies, writes,
        })
    }

    pub(crate) fn site_key(&self) -> (usize, u32) {
        (std::sync::Arc::as_ptr(&self.source) as usize,
         self.original.source_binding.as_ref().and_then(|binding| binding.invocation_site()).expect("retained original site").offset)
    }

    pub(crate) fn matches_statement(
        &self,
        statement: &Statement,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        let Some(metadata) = metadata.filter(|metadata| metadata.source_analysis_input() == Some(&self.input) && metadata.matches_registry(registry)) else { return false; };
        let Some(tokens) = statement.tokens() else { return false; };
        let Some(binding) = tokens.source_binding.as_ref() else { return false; };
        statement == &self.emitted
            && binding.invocation_site().is_some_and(|site| std::sync::Arc::ptr_eq(&site.source, &self.source))
            && binding.original_invocation_source_purpose(tokens, metadata, registry)
                == Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical)
    }

    pub(crate) fn matches_source_statement(
        &self,
        statement: &Statement,
        input: Option<&crate::analyser::ResolvedAnalysisInput>,
    ) -> bool {
        input == Some(&self.input) && statement == &self.emitted
            && statement.tokens().and_then(|tokens| tokens.source_binding.as_ref())
                .and_then(|binding| binding.invocation_site())
                .is_some_and(|site| std::sync::Arc::ptr_eq(&site.source, &self.source))
    }

    pub(crate) fn same_owner(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.source, &other.source) && self == other
    }

    pub(crate) fn writes(&self) -> &[SignatureSourceVariableOccurrence] { &self.writes }
}
