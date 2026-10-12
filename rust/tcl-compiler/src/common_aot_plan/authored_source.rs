// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly original schemas and storage templates under authored simulation.

use super::ProcedureIdentity;
use crate::ir::{CommandTokens, Module, Statement, TopLevelKind};
use crate::registry_invocation::original_structured_invocation_with_metadata_context;
use crate::types::TypeLattice;
use crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots;
use tcl_core_types::NameBytes;
use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, SemanticOperationId, hooks::LoweringHookId};
use tcl_syntax::naming::{NamePolicyAuthority, NativeNameContext, NativeVariableRootGeometry};

/// A future scalar destination and literal contents described by original
/// source. This is neither an allocated cell nor a reached write or SSA value.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginalAuthoredStorageTemplate {
    name: NameBytes,
    namespace: crate::command_binding::SourceNamespaceKey,
    literal: String,
    contents_type: TypeLattice,
}

impl OriginalAuthoredStorageTemplate {
    /// Counted, decoded original scalar spelling; it is not a physical key.
    #[must_use]
    pub const fn name(&self) -> &NameBytes {
        &self.name
    }

    /// Retained source lookup scope, without an allocated namespace or cell.
    #[must_use]
    pub const fn namespace(&self) -> &crate::command_binding::SourceNamespaceKey {
        &self.namespace
    }

    /// Original literal contents under the retained source word grammar.
    #[must_use]
    pub fn literal(&self) -> &str {
        &self.literal
    }

    /// Contents classification, independent of a Native object representation.
    #[must_use]
    pub const fn contents_type(&self) -> &TypeLattice {
        &self.contents_type
    }
}

/// Readonly accounting category for one original source statement.
#[derive(Debug, Clone, PartialEq)]
pub enum OriginalAuthoredSourceStatementKind {
    /// A source declaration and its proposed argument calling convention.
    ProcedureDeclaration {
        /// Exact original header/body identity, independently of installation.
        procedure: ProcedureIdentity,
        /// New declaration ordinals, never a borrowed activation layout.
        arguments: OriginalDeclaredProcedureArgumentSlots,
    },
    /// A possible literal scalar store under the selected source schema.
    LiteralStore(OriginalAuthoredStorageTemplate),
    /// A selected source schema and original nested procedure templates.
    /// Argument evaluation and future dispatch remain independent.
    Invocation {
        /// Source allocations of the original nested procedure candidates.
        procedures: Vec<ProcedureIdentity>,
    },
}

/// One whole original statement whose source schema was selected.
/// No category can be consumed as executable closed-program coverage.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginalAuthoredSourceStatement {
    span: Span,
    operation: SemanticOperationId,
    kind: OriginalAuthoredSourceStatementKind,
}

impl OriginalAuthoredSourceStatement {
    /// Exact original statement coordinates.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Selected source operation, not a current handler identity.
    #[must_use]
    pub const fn operation(&self) -> SemanticOperationId {
        self.operation
    }

    /// Source declaration, literal destination or invocation shape.
    #[must_use]
    pub const fn kind(&self) -> &OriginalAuthoredSourceStatementKind {
        &self.kind
    }
}

/// A missing source premise in an otherwise owned authored inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalAuthoredSourceResidual {
    /// No unanimous original vector belongs to this statement.
    StatementCarrierUnavailable,
    /// Actual point metadata or a complete source schema was unavailable.
    StatementSchemaUnavailable,
    /// Original nested geometry or its retained source target was incomplete.
    NestedCallUnavailable,
    /// The original procedure has no unique header/body/argument receipt.
    DeclarationUnavailable,
    /// The source store is not one supported unchanged literal scalar template.
    LiteralStoreUnavailable,
}

/// Why the readonly authored query has no current source owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalAuthoredProgramSourceDecline {
    /// Missing, stale or foreign Module source/input/configuration ownership.
    SourceOwnerUnavailable,
    /// No explicitly selected authored simulation, or a Native/hosted entry.
    AuthoredPurposeUnavailable,
    /// The source is not a whole original top-level script.
    RootSourceUnavailable,
}

/// Original schemas and future storage templates, independently of admission.
/// Incomplete source targets remain explicit. This type supplies no executable
/// coverage, allocated cell, receiver, frame, emitted operation or bootstrap.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginalAuthoredProgramSourcePlan {
    statements: Vec<OriginalAuthoredSourceStatement>,
    residuals: Vec<(Span, OriginalAuthoredSourceResidual)>,
}

impl OriginalAuthoredProgramSourcePlan {
    /// Read the real Module source owner and its original per-point schemas.
    /// A naming simulation cannot fill a missing Native entry or provider.
    ///
    /// # Errors
    /// Returns a typed decline when the complete source owner, authored purpose
    /// or original whole-module script is unavailable.
    pub fn build(
        module: &Module,
        registry: &CommandRegistry,
    ) -> Result<Self, OriginalAuthoredProgramSourceDecline> {
        use OriginalAuthoredProgramSourceDecline as Decline;
        if !module
            .retained_source_bindings
            .as_deref()
            .is_some_and(|owner| owner.matches_module(module, registry))
            || crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)
                .is_none()
        {
            return Err(Decline::SourceOwnerUnavailable);
        }
        if module.source_entry.native_entry.is_some()
            || module.source_entry.hosted_execution_context.is_some()
            || module
                .source_entry
                .options()
                .execution_name_policy()
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                .is_none_or(|policy| policy.authority() != NamePolicyAuthority::AuthoredSimulation)
        {
            return Err(Decline::AuthoredPurposeUnavailable);
        }
        if module.top_level_kind != TopLevelKind::Script
            || module.source_entry.compilation_scope
                != tcl_runtime_api::SourceCompilationScope::WholeModule
            || !module.top_level.is_authored_source()
        {
            return Err(Decline::RootSourceUnavailable);
        }
        let original = crate::segmenter::segment_commands_image_with_offset_and_config(
            &module.source,
            0,
            module.native_lexer_config(),
        )
        .ok_or(Decline::RootSourceUnavailable)?;
        if original.iter().any(|command| command.is_partial)
            || !original.iter().map(|command| command.span).eq(module
                .top_level
                .statements
                .iter()
                .filter(|statement| statement.is_executable_invocation())
                .map(Statement::span))
        {
            return Err(Decline::RootSourceUnavailable);
        }
        let mut plan = Self {
            statements: Vec::new(),
            residuals: Vec::new(),
        };
        for statement in &module.top_level.statements {
            if !statement.is_executable_invocation() {
                continue;
            }
            match original_statement(module, registry, statement) {
                Ok(schema) => plan.statements.push(schema),
                Err(residual) => plan.residuals.push((statement.span(), residual)),
            }
        }
        Ok(plan)
    }

    /// Original statement schemas in direct source order.
    #[must_use]
    pub fn statements(&self) -> &[OriginalAuthoredSourceStatement] {
        &self.statements
    }

    /// Missing source premises, without recovering from reporting labels.
    #[must_use]
    pub fn residuals(&self) -> &[(Span, OriginalAuthoredSourceResidual)] {
        &self.residuals
    }

    /// Whether each direct original statement has a source accounting category.
    /// This answer says nothing about executable program closure.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.residuals.is_empty()
    }

    /// Possible literal scalar destinations, never current allocation facts.
    pub fn storage_templates(&self) -> impl Iterator<Item = &OriginalAuthoredStorageTemplate> {
        self.statements.iter().filter_map(|statement| {
            if let OriginalAuthoredSourceStatementKind::LiteralStore(template) = &statement.kind {
                Some(template)
            } else {
                None
            }
        })
    }
}

fn procedure_identity(procedure: &crate::ir::Procedure) -> ProcedureIdentity {
    ProcedureIdentity {
        qualified_name: procedure.qualified_name.clone(),
        definition_start: procedure.span.start(),
        definition_end: procedure.span.end(),
    }
}

fn original_statement(
    module: &Module,
    registry: &CommandRegistry,
    statement: &Statement,
) -> Result<OriginalAuthoredSourceStatement, OriginalAuthoredSourceResidual> {
    use OriginalAuthoredSourceResidual as Residual;
    let tokens = module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .ok_or(Residual::StatementCarrierUnavailable)?;
    let metadata = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| {
            binding.original_invocation_metadata_for_module(tokens, module, registry)
        })
        .ok_or(Residual::StatementSchemaUnavailable)?;
    let invocation =
        original_structured_invocation_with_metadata_context(registry, metadata, tokens)
            .ok_or(Residual::StatementSchemaUnavailable)?;
    let kind = match invocation.facts.operation {
        SemanticOperationId::StructuredLowering(LoweringHookId::Proc) => {
            let mut declarations = module
                .procedures
                .values()
                .filter(|procedure| procedure.span == statement.span());
            let procedure = declarations
                .next()
                .ok_or(Residual::DeclarationUnavailable)?;
            if declarations.next().is_some()
                || !module
                    .retained_source_bindings
                    .as_deref()
                    .is_some_and(|owner| owner.matches_original_procedure(procedure))
            {
                return Err(Residual::DeclarationUnavailable);
            }
            let arguments = OriginalDeclaredProcedureArgumentSlots::from_module(module, procedure)
                .ok_or(Residual::DeclarationUnavailable)?;
            OriginalAuthoredSourceStatementKind::ProcedureDeclaration {
                procedure: procedure_identity(procedure),
                arguments,
            }
        }
        SemanticOperationId::StructuredLowering(LoweringHookId::Set) => {
            OriginalAuthoredSourceStatementKind::LiteralStore(
                original_storage_template(module, registry, tokens)
                    .ok_or(Residual::LiteralStoreUnavailable)?,
            )
        }
        _ => OriginalAuthoredSourceStatementKind::Invocation {
            procedures: original_nested_procedures(module, registry, tokens)
                .ok_or(Residual::NestedCallUnavailable)?,
        },
    };
    Ok(OriginalAuthoredSourceStatement {
        span: statement.span(),
        operation: invocation.facts.operation,
        kind,
    })
}

fn original_storage_template(
    module: &Module,
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<OriginalAuthoredStorageTemplate> {
    if tokens.words().len() != 3 {
        return None;
    }
    let namespace = tokens
        .source_binding
        .as_ref()?
        .declaration_operand_layout_advice(tokens)?
        .namespace()
        .clone();
    let name = crate::type_infer::original_literal_argument_contents(registry, module, tokens, 0)?;
    let literal =
        crate::type_infer::original_literal_argument_contents(registry, module, tokens, 1)?;
    let policy = module
        .source_entry
        .options()
        .execution_name_policy()?
        .native_recipe()?;
    let bytes = name.value().as_bytes();
    let input = policy.recipe().combined_variable_input(bytes);
    if bytes.is_empty()
        || input.element().is_some()
        || input.root().selected() != bytes
        || !matches!(policy.recipe().variable_root_geometry(NativeNameContext::root(), bytes),
            NativeVariableRootGeometry::Local(ref root) if root.as_bytes() == bytes)
    {
        return None;
    }
    Some(OriginalAuthoredStorageTemplate {
        name: bytes.into(),
        namespace,
        literal: literal.value().to_owned(),
        contents_type: literal.contents_type(),
    })
}

fn original_nested_procedures(
    module: &Module,
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<Vec<ProcedureIdentity>> {
    let metadata = tokens
        .source_binding
        .as_ref()?
        .original_invocation_metadata_for_module(tokens, module, registry)?;
    let config = metadata.source_analysis_input()?.lexer_config();
    let calls = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
        tokens, config, registry, metadata,
    )?;
    let mut procedures = Vec::new();
    for call in calls {
        let child = call.tokens.as_ref()?;
        let child_metadata = child
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_module(child, module, registry)?;
        if original_structured_invocation_with_metadata_context(registry, child_metadata, child)
            .is_some()
        {
            continue;
        }
        let targets = crate::registry_invocation::original_authored_procedure_calls_for_module(
            child, module, registry,
        )?;
        for target in targets {
            if !target.accepts_arguments() {
                return None;
            }
            procedures.push(procedure_identity(target.procedure()));
        }
    }
    Some(procedures)
}
