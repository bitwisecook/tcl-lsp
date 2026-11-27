// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional procedure declarations and the declaration values an advice tape selects.

use super::tape_preparation::{OriginalSourceCataloguePurpose, OriginalSourceTransitionSeed};
use super::{
    OriginalConditionalProcedureDeclarationMemo, OriginalSourceCommandTransitionAdvice,
    OriginalSourceProcedureDeclarationValues, OriginalSourceTransitionAdviceTape, source_procedure,
};
use crate::command_binding::SourceOriginLexical;
use std::collections::BTreeMap;
use std::sync::Arc;
use tcl_core_types::ByteNamespacePath;
use tcl_lexer::{LexerConfig, NativeWord};
use tcl_registry::model::ContextRegistry;

impl OriginalSourceProcedureDeclarationValues {
    pub(crate) fn head_correspondence(
        self,
        source: &str,
        config: LexerConfig,
        input: &crate::analyser::ResolvedAnalysisInput,
        selected: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> Option<crate::registry_invocation::source_structure::OriginalSourceHeadCorrespondence>
    {
        use crate::registry_invocation::{
            InvocationWordOrigin, source_cut_refuses, source_cut_stage,
        };
        if source_cut_refuses(
            "declaration-values.input",
            &self.input != input || config != input.lexer_config(),
        ) {
            return None;
        }
        let context = input.borrowed_context_registry();
        let original = source_cut_stage(
            "pair.words",
            crate::registry_invocation::source_structure::source_transition_words_from_advice(
                source,
                config,
                context,
                self.advice.as_ref().clone(),
            ),
        )?;
        let shape = source_cut_stage(
            "declaration-values.schema",
            original.with_source_schema(context, |schema| {
                schema.authored_source_procedure_arguments()
            }),
        )??;
        if source_cut_refuses("declaration-values.shape", shape != self.shape) {
            return None;
        }
        if source_cut_refuses(
            "pair.operands",
            original.origins().len() != selected.origins().len()
                || original
                    .origins()
                    .iter()
                    .enumerate()
                    .any(|(index, origin)| *origin != InvocationWordOrigin::Written(index))
                || (0..selected.arguments().len()).any(|index| {
                    original.original_argument_word(index) != selected.original_argument_word(index)
                }),
        ) {
            return None;
        }
        source_cut_stage(
            "pair.correspondence",
            original.original_source_head_correspondence(selected, context),
        )
    }
}

pub(super) type ConditionalProcedureDeclarationSelection = (
    Arc<source_procedure::OriginalSourceProcedureBody>,
    tcl_registry::AuthoredSourceProcedureArguments,
);

pub(super) type ConditionalProcedureDeclarationSelections =
    BTreeMap<u32, Option<ConditionalProcedureDeclarationSelection>>;

pub(super) fn conditional_procedure_declaration_selections(
    owner: &Arc<super::super::OriginalSourceAnalysisOwner>,
    seed: &OriginalSourceTransitionSeed,
    context: &ContextRegistry,
) -> Option<Arc<ConditionalProcedureDeclarationSelections>> {
    if !Arc::ptr_eq(&owner.source, &seed.origin)
        || owner.config != seed.config
        || !owner.metadata.is_standalone()
        || owner.metadata != seed.baseline.metadata_context
        || seed.baseline.native_entry.is_some()
        || seed.baseline.logical_source_input.is_some()
        || seed.baseline.vendor_source_input.is_some()
        || seed.baseline.hosted_execution_context.is_some()
        || seed.baseline.unknown_entry
        || !seed.matches_registry(context.commands())
    {
        return None;
    }
    let derived = owner.derived_memo();
    let baseline = Arc::downgrade(&seed.baseline);
    let registry = context.commands().snapshot().semantic_key();
    let matches = |memo: &OriginalConditionalProcedureDeclarationMemo| {
        std::sync::Weak::ptr_eq(&memo.baseline, &baseline) && memo.registry == registry
    };
    if let Some(memo) = derived
        .conditional_procedure_declarations
        .get()
        .filter(|memo| matches(memo))
    {
        return Some(Arc::clone(&memo.selections));
    }
    // The source interpreter runs outside OnceLock publication. It can neither
    // observe a partly built product nor hold a cache lock during recursion.
    let mut tape = seed.prepare_for(context, OriginalSourceCataloguePurpose::CompletePrefix)?;
    let mut selected = std::mem::take(&mut tape.conditional_procedure_declarations);
    for (offset, declaration) in &mut selected {
        if tape.blocks_registry_source(*offset) {
            *declaration = None;
        }
    }
    let selections = Arc::new(selected);
    if derived.conditional_procedure_declarations.get().is_none() {
        let _ = derived.conditional_procedure_declarations.set(
            OriginalConditionalProcedureDeclarationMemo {
                baseline: baseline.clone(),
                registry: registry.clone(),
                selections: Arc::clone(&selections),
            },
        );
    }
    // Another actual baseline must compute its own product. A same-key racing
    // publication may reuse only the genuinely identical owner's selection map.
    Some(
        derived
            .conditional_procedure_declarations
            .get()
            .filter(|memo| matches(memo))
            .map_or(selections, |memo| Arc::clone(&memo.selections)),
    )
}

impl OriginalSourceTransitionAdviceTape {
    /// Primary operand values only: final conflicts, absence and barriers refuse.
    pub(crate) fn procedure_declaration_values(
        &self,
        owner: &Arc<super::super::OriginalSourceAnalysisOwner>,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
        original: &[NativeWord],
        binding: &super::super::SourceInvocationBinding,
    ) -> Option<OriginalSourceProcedureDeclarationValues> {
        use crate::registry_invocation::{source_cut_refuses, source_cut_stage};
        if source_cut_refuses(
            "declaration-values.registry-barrier",
            self.blocks_registry_source(offset),
        ) {
            return None;
        }
        let record = source_cut_stage(
            "declaration-values.record",
            self.conditional_procedure_declarations.get(&offset),
        )?;
        let (procedure, shape) = source_cut_stage("declaration-values.unanimity", record.as_ref())?;
        let advice = &procedure.declaration;
        let origin = &owner.source;
        let site = source_cut_stage(
            "declaration-values.binding-site",
            binding.dispatch_site.as_ref(),
        )?;
        if source_cut_refuses(
            "declaration-values.owner-source-context",
            owner.config != input.lexer_config()
                || !Arc::ptr_eq(&site.source, origin)
                || site.offset != offset
                || binding
                    .original_analysis_owner
                    .as_ref()
                    .is_none_or(|actual| !Arc::ptr_eq(actual, owner))
                || !Arc::ptr_eq(&advice.site.source, origin)
                || advice.site.offset != offset
                || !advice.matches_source(origin.source_image(), input.lexer_config())
                || !advice.matches_context(input.borrowed_context_registry())
                || advice.original_words() != original,
        ) {
            return None;
        }
        Some(OriginalSourceProcedureDeclarationValues {
            advice: Arc::clone(advice),
            shape: *shape,
            input: input.clone(),
        })
    }
}

/// Original declaration source selected by the existing transition interpreter.
/// This receipt supplies no current command target, publication or entered frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct OriginalConditionalProcedureDeclaration {
    pub(in super::super) advice: Arc<OriginalSourceCommandTransitionAdvice>,
    pub(in super::super) body: Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    pub(in super::super) name: NativeWord,
    pub(in super::super) parameters: NativeWord,
    pub(in super::super) namespace: ByteNamespacePath,
    pub(in super::super) label: String,
}

/// Reborrow an immutable original source observation, never its withdrawn
/// execution policy or frame. The ordinary layout/table admission is unchanged.
pub(in super::super) fn original_conditional_procedure_declaration(
    site: &tcl_names::command::CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    row: &super::super::declaration_layout::DeclarationLayoutObservation,
    owner: &Arc<super::super::OriginalSourceAnalysisOwner>,
    registry: &tcl_registry::CommandRegistry,
) -> Option<OriginalConditionalProcedureDeclaration> {
    let baseline = &row.snapshot.state.baseline;
    if !Arc::ptr_eq(&owner.source, &site.source)
        || owner.config != row.config
        || !owner.metadata.is_standalone()
        || !baseline.metadata_context.is_standalone()
        || baseline.native_entry.is_some()
        || baseline.logical_source_input.is_some()
        || baseline.vendor_source_input.is_some()
        || baseline.hosted_execution_context.is_some()
        || baseline.unknown_entry
        || row.words.as_ref() != tokens.words()
        || row.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
        || !row.entry.owns_source(&site.source, site.offset)
        || !matches!(&row.namespace, tcl_names::namespace::SourceNamespaceKey::Authored(namespace) if namespace == "::")
        || baseline.registry_owner.as_ref()?.semantic_key() != registry.snapshot().semantic_key()
    {
        return None;
    }
    // This bounded producer owns the original root script, not a reconstructed
    // child activation or the current (possibly Unknown) execution frame.
    let super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
        source,
        namespace,
        ..
    } = row.entry.as_ref()
    else {
        return None;
    };
    if !Arc::ptr_eq(&source.origin, &site.source)
        || source.base() != 0
        || source.text.bytes() != site.source.source_image().bytes()
        || namespace != &row.namespace
    {
        return None;
    }
    let original = site
        .source
        .original_lexical_command(tokens.words(), site.offset, row.config)?;
    let seed = OriginalSourceTransitionSeed {
        analysis_owner: Some(Arc::clone(owner)),
        origin: Arc::clone(&site.source),
        config: row.config,
        baseline: Arc::clone(baseline),
        logical_input: None,
        stock_source_effects: Vec::new(),
    };
    let context = seed.source_context(registry)?;
    let selections = conditional_procedure_declaration_selections(owner, &seed, &context)?;
    let (declaration, shape) = selections.get(&site.offset)?.as_ref()?;
    let advice = &declaration.declaration;
    if advice.site() != site
        || advice.original_words() != original.native_words()
        || original
            .native_words()
            .iter()
            .any(|word| word.group().expand)
        || !advice.matches_source(site.source.source_image(), row.config)
        || !advice.matches_context(&context)
        || advice.uncertain_operations().is_empty()
        || !declaration
            .body
            .matches_source(site.source.source_image(), row.config)
        || !declaration.body.matches_context(&context)
    {
        return None;
    }
    let parameter = advice.arguments.get(shape.parameters)?;
    let name = advice.arguments.get(shape.name)?;
    let body = advice.arguments.get(shape.body)?;
    if [name, parameter, body].iter().any(|word| {
        !matches!(
            word.origin,
            crate::registry_invocation::InvocationWordOrigin::Written(_)
        )
    }) || Some(&name.original) != declaration.name.original_word()
        || &body.original != declaration.body.original_container()
    {
        return None;
    }
    Some(OriginalConditionalProcedureDeclaration {
        advice: Arc::clone(advice),
        body: Arc::clone(&declaration.body),
        name: declaration.name.original_word()?.clone(),
        parameters: parameter.original.clone(),
        namespace: declaration.declaration_slot.namespace.clone(),
        label: tcl_syntax::naming::authored_source_command_spelling(&declaration.declaration_slot)?,
    })
}
