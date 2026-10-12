// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proposed argument storage joined to exact reached incoming reads.

use super::{
    AotMetadataSelection, CompilationUnit, EscapeTag, MaterialisableSlotDecline,
    MaterialisationGuardRequirements, MaterialisationRecipe, ProcEscapeSummary, ProcedureIdentity,
    SemanticOptimisationConfig, SemanticOptimisationPassId, SharingState, TypeLattice, TypeShape,
    VarStorage,
};
use crate::ssa::{SsaIncomingSlotRead, SsaSourceView};
use crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

/// A proposed argument position in one authentic source declaration.
/// This identity is independent of every reached physical SSA cell.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclaredArgumentIdentity {
    /// Canonical original declaration and its complete source extent.
    pub procedure: ProcedureIdentity,
    /// Original fixed argument position in the proposed calling convention.
    pub ordinal: u32,
}

/// Original declaration storage and the exact reads that can consume it.
/// Physical cell alternatives remain distinct; this grants no borrowed frame,
/// object representation, coercion erasure or Native body admission.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclaredArgumentEvidence {
    declaration: Arc<OriginalDeclaredProcedureArgumentSlots>,
    reads: Vec<SsaIncomingSlotRead>,
    shape: TypeShape,
}

impl DeclaredArgumentEvidence {
    /// Authenticated original declaration and proposed argument convention.
    #[must_use]
    pub fn declaration(&self) -> &OriginalDeclaredProcedureArgumentSlots {
        &self.declaration
    }

    /// Exact reached reads, each retaining its distinct physical activations.
    #[must_use]
    pub fn reads(&self) -> &[SsaIncomingSlotRead] {
        &self.reads
    }

    /// Singleton caller type joined across every selected calling site.
    #[must_use]
    pub const fn shape(&self) -> &TypeShape {
        &self.shape
    }

    /// Storage of the newly compiled argument, independently of existing SSA.
    #[must_use]
    pub const fn storage(&self) -> VarStorage {
        VarStorage::MaterializableSlot
    }

    /// Preserve the actual incoming Tcl object when materialising this argument.
    #[must_use]
    pub const fn recipe(&self) -> MaterialisationRecipe {
        MaterialisationRecipe::RetainOriginalTclObject {
            sharing: SharingState::Shared,
        }
    }

    /// Runtime observers remain independent of the original source proof.
    #[must_use]
    pub const fn runtime_guards(&self) -> MaterialisationGuardRequirements {
        MaterialisationGuardRequirements {
            variable_trace_epoch: true,
            interpreter_policy_epoch: true,
        }
    }
}

/// Selection for declared argument storage, never a synthetic SSA version.
#[derive(Debug, Clone, PartialEq)]
pub enum DeclaredArgumentDecision {
    /// Original declaration, closed incoming reads and caller type all agree.
    Selected(Box<DeclaredArgumentEvidence>),
    /// The proposed storage lacks one of its independent prerequisites.
    Declined(MaterialisableSlotDecline),
}

struct ArgumentInputs<'a> {
    unit: &'a CompilationUnit,
    function: &'a crate::compilation_unit::FunctionUnit,
    registry: &'a CommandRegistry,
    declaration: &'a Arc<OriginalDeclaredProcedureArgumentSlots>,
    summary: Option<&'a ProcEscapeSummary>,
    propagated: &'a HashMap<(String, usize), Option<TypeLattice>>,
    metadata_available: bool,
    enabled: bool,
    qname: &'a str,
    ordinal: usize,
}

pub(super) fn collect(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    selection: AotMetadataSelection,
    escape: &HashMap<String, ProcEscapeSummary>,
    propagated: &HashMap<(String, usize), Option<TypeLattice>>,
    config: SemanticOptimisationConfig,
) -> BTreeMap<DeclaredArgumentIdentity, DeclaredArgumentDecision> {
    let mut arguments = BTreeMap::new();
    for (qname, function) in &unit.procedures {
        let Some(procedure) = unit.ir_module.procedures.get(qname) else {
            continue;
        };
        let Some(declaration) =
            OriginalDeclaredProcedureArgumentSlots::from_module(&unit.ir_module, procedure)
        else {
            continue;
        };
        let declaration = Arc::new(declaration);
        for ordinal in 0..declaration.arguments().names().len() {
            let identity = DeclaredArgumentIdentity {
                procedure: ProcedureIdentity {
                    qualified_name: qname.clone(),
                    definition_start: procedure.span.start(),
                    definition_end: procedure.span.end(),
                },
                ordinal: u32::try_from(ordinal).expect("original argument count fits u32"),
            };
            arguments.insert(
                identity,
                decide(&ArgumentInputs {
                    unit,
                    function,
                    registry,
                    declaration: &declaration,
                    summary: escape.get(qname),
                    propagated,
                    metadata_available: selection
                        .for_function(function, registry, &unit.ir_module)
                        .is_some(),
                    enabled: config.is_enabled(SemanticOptimisationPassId::MaterialisableSlot),
                    qname,
                    ordinal,
                }),
            );
        }
    }
    arguments
}

fn decide(input: &ArgumentInputs<'_>) -> DeclaredArgumentDecision {
    use MaterialisableSlotDecline as Decline;
    let select = || -> Result<DeclaredArgumentEvidence, MaterialisableSlotDecline> {
        if !input.enabled {
            return Err(Decline::PassDisabled);
        }
        if !input.metadata_available {
            return Err(Decline::OriginalSlotUnavailable);
        }
        if input.function.requires_native_math_binding_validation() {
            return Err(Decline::MathBindingPrerequisiteRequired);
        }
        if !input.function.cfg.exception_edges.is_empty() {
            return Err(Decline::ExceptionalControlFlow);
        }
        let name = input.declaration.arguments().names()[input.ordinal]
            .try_utf8()
            .map_err(|_| Decline::OriginalSlotUnavailable)?;
        let summary = input.summary.ok_or(Decline::NoLocalSlot)?;
        if summary.tags.get(name) == Some(&EscapeTag::Frame) {
            return Err(Decline::EscapesToFrame);
        }
        if input.unit.ir_module.has_dynamic_variable_trace
            || input.unit.ir_module.traced_variables.contains(name)
        {
            return Err(Decline::VariableTrace);
        }
        let reads = original_reads(input, name).ok_or(Decline::OriginalSlotUnavailable)?;
        let shape = input
            .propagated
            .get(&(input.qname.to_owned(), input.ordinal))
            .and_then(Option::as_ref)
            .and_then(TypeLattice::single_shape)
            .cloned()
            .ok_or(Decline::TypeNotSingleton)?;
        Ok(DeclaredArgumentEvidence {
            declaration: Arc::clone(input.declaration),
            reads,
            shape,
        })
    };
    match select() {
        Ok(evidence) => DeclaredArgumentDecision::Selected(Box::new(evidence)),
        Err(reason) => DeclaredArgumentDecision::Declined(reason),
    }
}

fn original_reads(input: &ArgumentInputs<'_>, name: &str) -> Option<Vec<SsaIncomingSlotRead>> {
    let mut reads = Vec::new();
    for (&block, data) in &input.function.ssa.blocks {
        for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = SsaSourceView::at_statement(&input.function.ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for access in &tokens.variable_accesses {
                let dialect = access.variable_context.invocation_dialect?;
                if tcl_syntax::naming::var_reference_for_style(
                    &access.original_spelling,
                    dialect.lexer_grammar.braced_var,
                ) != name
                {
                    continue;
                }
                let read = input.declaration.incoming_source_read(
                    access,
                    input.ordinal,
                    input.registry,
                )?;
                if !reads.contains(&read) {
                    reads.push(read);
                }
            }
        }
    }
    (!reads.is_empty()).then_some(reads)
}
