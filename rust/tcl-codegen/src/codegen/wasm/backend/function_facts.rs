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

//! What the emitter may assume about a function: direct-procedure eligibility, shadow obligations, trusted assignments and leaf statement plans.

use super::super::leaf_invoke::{WasmLeafInvokeDecline, select_leaf_invocation};
use super::FunctionFacts;
use std::collections::{HashMap, HashSet};
use tcl_compiler::backend_registry::SelectionFacts;
use tcl_compiler::command_binding::{
    Binding, BindingKind, CommandBinding, ModuleCommandMutations, analyse_command_binding,
};
use tcl_compiler::common_aot_plan::{
    original_direct_expression_body_operations, semantic_operation_binding_is_trusted,
};
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::ir::{Module, Procedure, Statement};
use tcl_compiler::native_lowering::cells::{CellPlace, name_data_place};
use tcl_compiler::registry_invocation::{
    InvocationMetadataContext, RegistryInvocationResolution,
    resolve_command_tokens_with_metadata_context,
};
use tcl_lexer::Span;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::{CommandRegistry, SemanticOperationId, TclType};
use tcl_syntax::expr::{BinOp, ExprNode};

pub(super) fn span_key(span: Span) -> (u32, u32) {
    (span.start(), span.end())
}

pub(super) fn direct_expr_supported(
    expr: &ExprNode,
    params: &HashSet<&str>,
    config: tcl_lexer::LexerConfig,
) -> bool {
    match expr {
        ExprNode::Var { text, name, .. } => {
            matches!(
                tcl_compiler::native_lowering::cells::variable_reference_place(text, config),
                Ok(tcl_compiler::native_lowering::cells::CellPlace::Named { name: actual })
                    if actual == *name && params.contains(actual.as_str())
            )
        }
        ExprNode::Literal { .. } => true,
        ExprNode::Binary {
            op: BinOp::Add,
            left,
            right,
        } => {
            direct_expr_supported(left, params, config)
                && direct_expr_supported(right, params, config)
        }
        _ => false,
    }
}

/// Source naming coordinates retained from the actual issued primary bodies.
/// These facts only withdraw optional WASM specializations; they publish no
/// runtime command and grant no callable body, frame or Native admission.
pub(super) struct OriginalProcedureNames<'a> {
    publications: Vec<(
        tcl_compiler::ir::ProcedureId,
        &'a tcl_compiler::signature_scan::scope::SignatureSourceCommand,
    )>,
}

impl<'a> OriginalProcedureNames<'a> {
    pub(super) fn capture(module: &'a Module, registry: &CommandRegistry) -> Option<Self> {
        let retained = module.retained_source_bindings.as_ref()?;
        retained.original_top_namespace_for_module(module, registry)?;
        let publications = module
            .procedures
            .iter()
            .map(|(id, procedure)| {
                (module.procedure_ids.owns(*id)
                    && procedure.body_kind == tcl_compiler::ir::ProcedureBodyKind::Named
                    && retained.matches_original_procedure_summary(*id, procedure))
                .then_some(())?;
                Some((*id, procedure.publication.as_ref()?))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self { publications })
    }

    pub(super) fn redefined_ids(&self) -> HashSet<tcl_compiler::ir::ProcedureId> {
        let mut counts = HashMap::new();
        for (_, publication) in &self.publications {
            *counts.entry(*publication).or_insert(0usize) += 1;
        }
        self.publications
            .iter()
            .filter(|(_, publication)| counts.get(publication).is_some_and(|count| *count > 1))
            .map(|(id, _)| *id)
            .collect()
    }

    pub(super) fn shadow_obligations(&self) -> Option<Vec<(String, Binding)>> {
        self.publications
            .iter()
            .map(|(_, publication)| {
                Some((
                    publication.source_spelling()?,
                    Binding {
                        kind: BindingKind::Unknown,
                        target: None,
                    },
                ))
            })
            .collect()
    }
}

fn direct_proc_eligible(
    module: &Module,
    id: tcl_compiler::ir::ProcedureId,
    proc: &Procedure,
    unit: &FunctionUnit,
    registry: &CommandRegistry,
    mutations: &ModuleCommandMutations,
    redefined_ids: &HashSet<tcl_compiler::ir::ProcedureId>,
) -> bool {
    if !module.procedure_ids.owns(id)
        || module
            .procedures
            .get(&id)
            .is_none_or(|held| !std::ptr::eq(held, proc))
        || unit.body_id != Some(id)
        || module
            .source_entry
            .native_entry
            .as_ref()
            .is_none_or(|entry| {
                proc.body
                    .namespace_context
                    .as_deref()
                    .is_none_or(|namespace| !namespace.is_native_root_in_entry(entry))
            })
    {
        return false;
    }
    let Some(operations) = original_direct_expression_body_operations(
        registry,
        unit.invocation_metadata_context_for_module(registry, module),
        proc,
        unit.source_lexer_config(),
    ) else {
        return false;
    };
    if tcl_compiler::native_compilation_admission::script_requires_admission(&proc.body)
        || proc.namespace_scoped
        || redefined_ids.contains(&id)
        || unit.complexity_guarded
        || operations
            .into_iter()
            .any(|operation| !semantic_operation_binding_is_trusted(registry, mutations, operation))
        || !matches!(
            unit.return_type.tcl_type(),
            Some(TclType::Int | TclType::Double | TclType::Numeric)
        )
    {
        return false;
    }
    let Some(arguments) =
        tcl_compiler::var_escape::original_slots::original_procedure_argument_slots(module, proc)
    else {
        return false;
    };
    // This emitter's string-keyed ABI is admitted only after exact native
    // declaration bytes agree with every retained parameter position.
    if arguments.names().len() != proc.params.len()
        || !arguments
            .names()
            .iter()
            .zip(&proc.params)
            .all(|(original, name)| original.try_utf8().ok() == Some(name.as_str()))
    {
        return false;
    }
    let [
        Statement::Return {
            expr: Some(expr), ..
        },
    ] = proc.body.statements.as_slice()
    else {
        return false;
    };
    let params: HashSet<&str> = proc.params.iter().map(String::as_str).collect();
    direct_expr_supported(expr, &params, unit.source_lexer_config())
}

/// Retained declarations can withdraw a builtin proof, never supply an implementation.
pub(super) fn procedure_shadow_obligations(
    module: &Module,
    registry: &CommandRegistry,
    is_top: bool,
) -> Option<Vec<(String, Binding)>> {
    if is_top {
        return Some(Vec::new());
    }
    OriginalProcedureNames::capture(module, registry)?.shadow_obligations()
}

pub(super) fn function_facts(
    unit: &FunctionUnit,
    module: &Module,
    registry: &CommandRegistry,
    mutations: &ModuleCommandMutations,
    is_top: bool,
) -> FunctionFacts {
    let Some(initial) = procedure_shadow_obligations(module, registry, is_top) else {
        // Preserve original leaf source execution when declaration names cannot
        // independently withdraw every possibly shadowed specialization.
        return FunctionFacts::default();
    };
    let context = unit.invocation_metadata_context_for_module(registry, module);
    let bindings = analyse_command_binding(&unit.cfg, registry, &initial);
    let mut facts = FunctionFacts::default();
    let mut planned_spans: HashSet<(u32, u32)> = HashSet::new();
    for block in unit.cfg.reverse_postorder() {
        let Some(cfg_block) = unit.cfg.blocks.get(&block) else {
            continue;
        };
        for (stmt_idx, statement) in cfg_block.statements.iter().enumerate() {
            // An analysis marker shares its host's span and is no command to
            // plan: counted as a second sighting of that span it would send
            // the host back to the source-span fallback.
            if tcl_compiler::ssa::is_effect_marker(statement)
                || !statement.is_executable_invocation()
            {
                continue;
            }
            if let Statement::AssignConst {
                span,
                name,
                ..
            } | Statement::AssignValue {
                span,
                name,
                ..
            } = statement
                // `tcl_codegen_var_set` stores under the exact name, so an
                // array-element target — as opposed to a scalar whose name
                // merely contains parentheses — keeps the source-span
                // fallback. Original word data supplies the name independently.
                && let Some(original_name) = original_assignment_name(unit, module, registry, statement, name)
                && name_data_place(&original_name)
                    .is_some_and(|place| matches!(place, CellPlace::Named { .. }))
                && assignment_operation_is_trusted(
                    unit,
                    context,
                    registry,
                    mutations,
                    &bindings,
                    (block, stmt_idx),
                    statement,
                )
                && let Some(value) = original_assignment_literal(unit, module, registry, statement)
            {
                facts
                    .direct_assignments
                    .insert(span_key(*span), (original_name, value));
            }
            // Generic prebuilt-argv invocation is the normal leaf-command path
            // and needs no binding proof: the runtime resolves the command head
            // exactly as interpreted Tcl does, so a rename, alias, ensemble, or
            // trace stays honoured. Only the *words* have to be provable.
            if let Statement::Call { tokens, .. } = statement {
                plan_leaf_statement(
                    &mut facts,
                    &mut planned_spans,
                    registry,
                    context,
                    unit.source_lexer_config(),
                    statement.span(),
                    tokens.as_ref(),
                );
            }
        }
    }
    facts
}

pub(super) fn original_assignment_name(
    unit: &FunctionUnit,
    module: &Module,
    registry: &CommandRegistry,
    statement: &Statement,
    name: &str,
) -> Option<String> {
    let tokens = tcl_compiler::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &unit.cfg.command_binding_sites,
        statement,
    )?;
    (tokens.argv_texts.get(1).map(String::as_str) == Some(name)).then_some(())?;
    tcl_compiler::registry_invocation::original_literal_argument_data_for_module(
        registry, module, tokens, 0,
    )
}

/// The unchanged value operand uses its original point grammar, including
/// upstream `AssignValue` escape preservation. This supplies no store authority.
pub(super) fn original_assignment_literal(
    unit: &FunctionUnit,
    module: &Module,
    registry: &CommandRegistry,
    statement: &Statement,
) -> Option<String> {
    let (Statement::AssignConst { value, .. } | Statement::AssignValue { value, .. }) = statement
    else {
        return None;
    };
    let tokens = tcl_compiler::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &unit.cfg.command_binding_sites,
        statement,
    )?;
    if tokens.argv_texts.get(2) != Some(value) {
        return None;
    }
    tcl_compiler::type_infer::original_literal_argument_contents(registry, module, tokens, 1)
        .map(|literal| literal.value().to_owned())
}

/// A consumed assignment head keeps its original source selection and words.
/// Catalogue membership and flow binding alone cannot recreate that carrier.
fn assignment_operation_is_trusted(
    unit: &FunctionUnit,
    context: Option<InvocationMetadataContext<'_>>,
    registry: &CommandRegistry,
    mutations: &ModuleCommandMutations,
    bindings: &CommandBinding<'_>,
    site: (tcl_compiler::cfg::BlockId, usize),
    statement: &Statement,
) -> bool {
    let Some(context) = context else {
        return false;
    };
    let Some(tokens) = tcl_compiler::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &unit.cfg.command_binding_sites,
        statement,
    )
    .filter(|tokens| {
        tokens.synthetic.is_none()
            && tokens.words_align_with_argv_text()
            && tokens
                .source_binding
                .as_ref()
                .and_then(
                    tcl_compiler::command_binding::SourceInvocationBinding::proved_execution_target,
                )
                .is_some()
    }) else {
        return false;
    };
    let Ok(RegistryInvocationResolution::Resolved(invocation)) =
        resolve_command_tokens_with_metadata_context(registry, Some(context), tokens)
    else {
        return false;
    };
    let Some(effective) = tcl_compiler::registry_invocation::effective_command_words(tokens) else {
        return false;
    };
    let Some(written_head) = tokens.argv_texts.first() else {
        return false;
    };
    invocation.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Set)
        && effective.words.len() == 3
        && effective.words.len() == tokens.words().len()
        && !effective.origins.iter().any(|origin| {
            matches!(
                origin,
                tcl_compiler::registry_invocation::InvocationWordOrigin::BindingPrefix(_)
            )
        })
        && bindings.is_original_builtin_at(site.0, site.1, written_head)
        && mutations.trusts(invocation.descriptor_key().label())
}

/// Select and record the prebuilt-argv plan for one leaf `Statement::Call`.
///
/// A span is planned at most once. The control-flow graph can carry a
/// synthetic header call over the same source range as the statement the
/// structured walk emits (iterator headers do exactly that), and the two need
/// not describe the same argv, so a second sighting of a span poisons it back
/// to the source-span eval fallback rather than risk emitting the wrong argv.
fn plan_leaf_statement(
    facts: &mut FunctionFacts,
    planned_spans: &mut HashSet<(u32, u32)>,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    config: tcl_lexer::LexerConfig,
    span: Span,
    tokens: Option<&tcl_compiler::ir::CommandTokens>,
) {
    let key = span_key(span);
    if !planned_spans.insert(key) {
        facts.leaf_invocations.remove(&key);
        facts
            .leaf_declines
            .insert(key, WasmLeafInvokeDecline::DuplicateStatementSpan);
        return;
    }
    let Some(tokens) = tokens.filter(|tokens| tokens.words_align_with_argv_text()) else {
        facts
            .leaf_declines
            .insert(key, WasmLeafInvokeDecline::MissingCommandTokens);
        return;
    };
    // Missing actual metadata keeps generic argv planning, without entering
    // the resolver's independently supported standalone catalogue mode.
    let resolution = context.map(|context| {
        resolve_command_tokens_with_metadata_context(registry, Some(context), tokens)
    });
    let resolved = match &resolution {
        Some(Ok(RegistryInvocationResolution::Resolved(invocation))) => Some(invocation.as_ref()),
        _ => None,
    };
    let operation = resolved.map_or(SemanticOperationId::Invoke, |facts| facts.operation);
    let selection_facts = resolved.map_or_else(SelectionFacts::unavailable, |facts| {
        SelectionFacts::from_invocation(facts)
    });
    match select_leaf_invocation(tokens.words(), operation, selection_facts, config) {
        Ok(plan) => {
            facts.leaf_invocations.insert(key, plan);
        }
        Err(decline) => {
            facts.leaf_declines.insert(
                key,
                super::super::leaf_invoke::word_decline(&decline)
                    .unwrap_or(WasmLeafInvokeDecline::MissingCommandTokens),
            );
        }
    }
}

pub(super) struct ProcedurePlan<'a> {
    pub(super) procs: Vec<(tcl_compiler::ir::ProcedureId, &'a Procedure)>,
    pub(super) indices: HashMap<tcl_compiler::ir::ProcedureId, u32>,
    pub(super) arity: HashMap<tcl_compiler::ir::ProcedureId, usize>,
    pub(super) direct: HashSet<tcl_compiler::ir::ProcedureId>,
}

pub(super) fn procedure_plan<'a>(
    module: &'a Module,
    analysis: Option<(&CompilationUnit, &CommandRegistry)>,
    top_idx: u32,
) -> ProcedurePlan<'a> {
    let procs: Vec<_> = module
        .native_source_procedure_bodies_for_compilation()
        .filter(|(_, procedure)| {
            !procedure.namespace_scoped
                && !tcl_compiler::native_compilation_admission::script_requires_admission(
                    &procedure.body,
                )
        })
        .map(|(id, procedure)| (*id, procedure))
        .collect();
    let indices = procs
        .iter()
        .enumerate()
        .map(|(position, (id, _))| {
            (
                *id,
                top_idx
                    .saturating_add(1)
                    .saturating_add(u32::try_from(position).unwrap_or(u32::MAX)),
            )
        })
        .collect();
    let arity = procs
        .iter()
        .map(|(id, procedure)| (*id, procedure.params.len()))
        .collect();
    let direct = analysis.map_or_else(HashSet::new, |(unit, registry)| {
        let Some(names) = OriginalProcedureNames::capture(module, registry) else {
            return HashSet::new();
        };
        let redefined_ids = names.redefined_ids();
        procs
            .iter()
            .filter(|(id, procedure)| {
                unit.procedures.get(id).is_some_and(|function| {
                    direct_proc_eligible(
                        module,
                        *id,
                        procedure,
                        function,
                        registry,
                        &unit.command_mutations,
                        &redefined_ids,
                    )
                })
            })
            .map(|(id, _)| *id)
            .collect()
    });
    ProcedurePlan {
        procs,
        indices,
        arity,
        direct,
    }
}
