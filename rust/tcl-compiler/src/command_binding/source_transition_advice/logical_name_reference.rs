// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Same-horizon Logical name operands from genuine selected Registry roles.

use super::{
    AdviceGraph, AdviceNamingPolicy, OriginalSourceCommandTransition,
    OriginalSourceTransitionAdviceTape,
};
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage};

/// A source command-name operand and its conditional procedure or class declaration. The
/// complete consuming vector, actual selected role and alias history remain
/// retained. This supplies no execution, installed lookup, absence or editing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalNameReference {
    input: ResolvedAnalysisInput,
    consumer: Arc<[NativeWord]>,
    operand: NativeWord,
    role: tcl_registry::ArgRole,
    target: SourceReferenceTarget,
    lineage: Vec<Arc<OriginalSourceCommandTransition>>,
    earlier_unknown_mutations: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceReferenceTarget {
    Procedure(Arc<OriginalSourceCommandTransition>),
    Class(Arc<super::OriginalSourceClassDeclaration>),
}

impl OriginalLogicalNameReference {
    /// Genuine complete written operand; its geometry is readonly source only.
    #[must_use]
    pub const fn operand(&self) -> &NativeWord {
        &self.operand
    }

    /// Registry-selected reference role, without runtime consumption or reach.
    #[must_use]
    pub const fn role(&self) -> tcl_registry::ArgRole {
        self.role
    }

    /// Actual alias/move steps retained at this reference horizon.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.lineage
    }

    /// Earlier unknown source effects remain an explicit applicability debt.
    #[must_use]
    pub const fn has_earlier_unknown_mutations(&self) -> bool {
        self.earlier_unknown_mutations
    }

    /// Same actual source header selected at the original consuming horizon.
    /// Missing/conflicting cards stay unavailable; final report maps are unused.
    #[must_use]
    pub fn declaration<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a crate::analyser::types::LogicalSourceProcedureDeclaration> {
        let input = analysis.resolved_input.as_ref()?;
        let head = self.consumer.first()?;
        self.matches_source(head.image(), input).then_some(())?;
        let current = analysis
            .retained_command_realm()?
            .original_logical_name_reference_inventory(input)?;
        current
            .iter()
            .any(|reference| reference == self)
            .then_some(())?;
        let SourceReferenceTarget::Procedure(definition) = &self.target else {
            return None;
        };
        let mut headers = analysis
            .logical_source_procedure_declarations()
            .filter(|header| header.matches_original_source_transition(analysis, definition));
        let header = headers.next()?;
        headers.next().is_none().then_some(header)
    }

    /// Genuine source class at this operand's retained consuming horizon.
    /// This supplies no runtime class occupation or edit permission.
    #[must_use]
    pub fn class_declaration<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a crate::analyser::ClassDef> {
        let input = analysis.resolved_input.as_ref()?;
        self.matches_source(self.consumer.first()?.image(), input)
            .then_some(())?;
        analysis
            .retained_command_realm()?
            .original_logical_name_reference_inventory(input)?
            .iter()
            .any(|reference| reference == self)
            .then_some(())?;
        let SourceReferenceTarget::Class(class) = &self.target else {
            return None;
        };
        class.logical_source_class(analysis)
    }

    fn matches_source(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        self.input == *input
            && input.has_logical_source_name_context()
            && self
                .consumer
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
            && self.consumer.iter().any(|word| word == &self.operand)
            && match &self.target {
                SourceReferenceTarget::Procedure(definition) => definition
                    .original_words()
                    .iter()
                    .all(|word| word.image() == image && word.config() == input.lexer_config()),
                SourceReferenceTarget::Class(class) => {
                    class.factory().logical_source_input() == Some(input)
                        && class.factory().matches_source(image, input.lexer_config())
                }
            }
    }
}

impl super::AdviceInvocationContext<'_> {
    pub(super) fn retain_logical_name_references(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: super::AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let AdviceNamingPolicy::Logical(input) = self.policy else {
            return;
        };
        let Some(head) = invocation.native.first() else {
            return;
        };
        let mut references = Vec::new();
        for (ordinal, role) in self.authored_roles(schema).0 {
            if !matches!(
                role,
                tcl_registry::ArgRole::CommandName | tcl_registry::ArgRole::CommandNameProbe
            ) {
                continue;
            }
            let Some(ordinal) = schema
                .facts()
                .argument_offset
                .checked_add(usize::from(ordinal))
            else {
                continue;
            };
            let Some(argument) = invocation.arguments.get(ordinal) else {
                continue;
            };
            let crate::registry_invocation::InvocationWordOrigin::Written(written) =
                argument.origin
            else {
                continue;
            };
            if invocation.native.get(written) != Some(&argument.original)
                || argument.original.group().expand
            {
                continue;
            }
            let Some(name) = argument.input.as_ref() else {
                continue;
            };
            if name.logical_input().is_none()
                || role == tcl_registry::ArgRole::CommandNameProbe
                    && !tcl_syntax::glob::is_literal_bytes(name.bytes())
            {
                continue;
            }
            let Some(selected) = graph.resolve_source_cell(name) else {
                continue;
            };
            let (target, own_lineage) = match selected.cell {
                super::AdviceCell::LogicalSourceProcedure {
                    definition,
                    lineage,
                } => (
                    SourceReferenceTarget::Procedure(Arc::clone(definition)),
                    lineage.as_slice(),
                ),
                super::AdviceCell::SourceClass(class) => (
                    SourceReferenceTarget::Class(Arc::clone(class)),
                    class.lineage(),
                ),
                _ => continue,
            };
            let mut steps = selected.lineage;
            steps.extend(own_lineage.iter().cloned());
            references.push(OriginalLogicalNameReference {
                input: input.clone(),
                consumer: Arc::from(invocation.native),
                operand: argument.original.clone(),
                role,
                target,
                lineage: steps,
                earlier_unknown_mutations: !graph.uncertain_operations.is_empty(),
            });
        }
        super::source_class::merge_receipt(
            &mut tape.logical_name_references,
            head.span().start(),
            Some(references),
        );
    }
}

impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn logical_name_reference_inventory(
        &self,
        input: &ResolvedAnalysisInput,
        image: &SourceImage,
    ) -> Vec<OriginalLogicalNameReference> {
        self.logical_name_references
            .values()
            .filter_map(Option::as_ref)
            .flatten()
            .filter(|reference| reference.matches_source(image, input))
            .cloned()
            .collect()
    }
}
