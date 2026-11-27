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

//! Sink warnings for one function: classification, the per-statement and branch-condition emitters and the sink-specific checks.

use super::{
    BlockId, CfgFunction, CommandHeadContext, CommandRegistry, HashMap, HashSet,
    SinkWarningContext, Span, SsaFunction, TaintLattice, TaintReplayAnalysis,
    TaintReplaySourceSnapshot, TaintSourceContext, TaintWarning, TaintWarningReport,
    UnavailableTaintReplay, UnavailableTaintReplayReason, ValueKey, cfg_order,
};

mod branch_conditions;
mod classification;
mod command_heads;
mod double_encode;
mod isolated_scripts;
mod numeric_coercion;
mod option_injection;
mod pattern_operands;
mod sink_emission;
mod sink_wrappers;
mod source_reads;
mod statement_warnings;
mod validators;

pub(super) use branch_conditions::emit_branch_condition_warnings;
#[cfg(test)]
pub(super) use classification::selected_sink_schema;
#[cfg(test)]
pub(super) use classification::{classify_network_interp_sinks, classify_sink};
pub(super) use command_heads::static_word_expr_value;
#[cfg(test)]
pub(super) use command_heads::{
    command_substitutions_in_word, emit_dynamic_command_head_warning, resolve_taint_command_heads,
};
#[cfg(test)]
pub(super) use isolated_scripts::{
    ScaffoldConstructors, emit_isolated_script_warnings, global_heads_name_one_command,
};
#[cfg(test)]
pub(super) use numeric_coercion::{collect_coercion_operands, holds_one_whole_var_ref};
pub(super) use option_injection::arg_index_span;
#[cfg(test)]
pub(super) use option_injection::option_injection_arguments;
#[cfg(test)]
pub(super) use pattern_operands::emit_regexp_pattern_warnings;
#[cfg(test)]
pub(super) use sink_emission::sink_var_position_safe;
#[cfg(test)]
pub(super) use sink_emission::{
    irule3002_name_position_safe, sink_arg_span, sink_colour_mitigated,
};
#[cfg(test)]
pub(super) use sink_wrappers::{
    list_wrapped_arg_command_is_literal, standalone_list_wrapper_head, var_consumed_by_sanitiser,
    var_wrapper_colours,
};
pub(super) use source_reads::{SourceWarningMetadata, original_scalar_reference_name};
#[cfg(test)]
pub(super) use source_reads::{operand_read_taint, reference_root_with_config};
pub(super) use statement_warnings::emit_statement_warnings;
#[cfg(test)]
pub(super) use statement_warnings::parse_taint_command_substitution;

/// Constructed publication labels used by the isolated lookup fixtures.
#[cfg(test)]
#[must_use]
pub(super) fn shadowed_builtin_names<'a>(
    namespace: &str,
    proc_qnames: impl Iterator<Item = &'a str>,
    registry: &CommandRegistry,
) -> HashSet<String> {
    /// `qname`'s bare tail when it sits directly under `ns_prefix` (e.g.
    /// `"::foo::"` or the global `"::"`) — `None` for a name that's either
    /// outside `ns_prefix` or in one of ITS OWN nested children.
    fn direct_member<'a>(qname: &'a str, ns_prefix: &str) -> Option<&'a str> {
        let bare = qname.strip_prefix(ns_prefix)?;
        (!bare.is_empty() && !bare.contains("::")).then_some(bare)
    }

    let ns_norm = crate::naming::normalise_qualified_name(namespace);
    let own_prefix = if ns_norm == "::" {
        "::".to_owned()
    } else {
        format!("{ns_norm}::")
    };
    let is_global = own_prefix == "::";

    let mut out = HashSet::new();
    for q in proc_qnames {
        let bare = match direct_member(q, &own_prefix) {
            Some(bare) => Some(bare),
            None if is_global => None,
            None => direct_member(q, "::"),
        };
        if let Some(bare) = bare
            && registry.get(bare).is_some()
        {
            out.insert(bare.to_owned());
        }
    }
    out
}

/// Source shadowing from a body retained by this actual compilation unit.
/// Unknown namespace, publication or issuer ownership refuses the projection.
#[must_use]
pub fn shadowed_builtin_names_for_module_function(
    function: &crate::compilation_unit::FunctionUnit,
    unit: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
) -> Option<HashSet<String>> {
    use crate::command_binding::SourceNamespaceKey;
    fn path(context: &SourceNamespaceKey) -> Option<tcl_core_types::ByteNamespacePath> {
        context.exact_native_path().cloned().or_else(|| {
            let authored = context.advisory_key()?;
            Some(tcl_core_types::ByteNamespacePath::from_segments(
                tcl_syntax::naming::key_segments(&authored),
            ))
        })
    }
    if !unit
        .all_body_function_units()
        .any(|owned| std::ptr::eq(owned, function))
    {
        return None;
    }
    let module = &unit.ir_module;
    let namespace = if let Some(id) = function.body_id {
        module.procedure_ids.owns(id).then_some(())?;
        if let Some(procedure) = module
            .procedures
            .get(&id)
            .or_else(|| module.body_lowering(&id))
        {
            path(procedure.body.namespace_context.as_deref()?)?
        } else {
            let method = module.methods.get(&id)?;
            match &method.execution_namespace {
                crate::ir::ExecutionNamespace::SourceContext(context) => path(context)?,
                crate::ir::ExecutionNamespace::Exact(context) => {
                    path(&SourceNamespaceKey::authored(context))?
                }
                crate::ir::ExecutionNamespace::RuntimeSelected => return None,
            }
        }
    } else {
        std::ptr::eq(function, std::ptr::from_ref(&unit.top_level)).then_some(())?;
        if let Some(context) = module.top_level_namespace_context.as_ref() {
            path(context)?
        } else if module.source_entry.metadata_context.is_standalone() {
            tcl_core_types::ByteNamespacePath::root()
        } else {
            return None;
        }
    };
    let mut shadowed = HashSet::new();
    for (id, procedure) in &module.procedures {
        module.procedure_ids.owns(*id).then_some(())?;
        // An event handler, a lambda and a namespace script are bodies with
        // no command slot, so they shadow nothing.
        if matches!(
            procedure.body_kind,
            crate::ir::ProcedureBodyKind::Event
                | crate::ir::ProcedureBodyKind::Lambda
                | crate::ir::ProcedureBodyKind::NamespaceScript
        ) {
            continue;
        }
        let slot = procedure.publication.as_ref()?.slot();
        if slot.namespace == namespace || slot.namespace.is_root() {
            let tail = slot.simple.try_utf8().ok()?;
            if registry
                .get_for_surface(tail, registry.own_surface_query())
                .is_some()
            {
                shadowed.insert(tail.to_owned());
            }
        }
    }
    Some(shadowed)
}

/// Emit taint sink warnings (`T100` family) for one function.
///
/// For each SSA use of a tainted variable in a sink statement, emits
/// one `TaintWarning`. Iterates blocks in `cfg_order` for deterministic
/// diagnostic ordering (matching the other shimmer/taint passes).
///
/// This is the per-function sink scan; the whole-unit `dataflow` / `diag`
/// aggregation calls it over the top level and every procedure unit. The
/// other warning kinds (setter-constraint / uri-split / path-concat /
/// destructive-file) are not emitted here yet.
///
/// `shadowed_builtins` (see [`shadowed_builtin_names_for_module_function`]) lists the bare
/// command names that resolve to a user proc rather than the registry
/// builtin from this function's namespace — those are skipped by the
/// registry-driven sink/source/pattern checks below.
#[must_use]
pub fn find_taint_warnings<
    S: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    shadowed_builtins: &HashSet<String, H>,
) -> Vec<TaintWarning> {
    taint_warning_report_impl(
        cfg,
        ssa,
        taints,
        executable_blocks,
        &SinkWarningContext {
            registry,
            dialect,
            source: TaintSourceContext::standalone(registry, dialect),
            shadowed_builtins,
            original_module: None,
            unavailable_replays: None,
        },
        None,
    )
    .warnings
}

/// Emit taint sink warnings for one complete compiler function unit.
///
/// Unlike [`find_taint_warnings`], this form can settle a whole-word variable
/// command head (`$command`) from its phase-correct reaching constant
/// definitions. Metadata uses the function's retained input; the scalar dialect
/// parameter does not restore missing availability. The value-provenance walk is shared with constant-dispatch
/// navigation and remains conservative across traces, frame aliases, dynamic
/// writes, and unprovable joins.
#[must_use]
pub fn find_taint_warnings_for_function<
    S: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    fu: &crate::compilation_unit::FunctionUnit,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    _dialect: Option<&'static tcl_dialect::DialectProfile>,
    shadowed_builtins: &HashSet<String, H>,
    module_traces: crate::compilation_unit::ModuleTraceFacts<'_>,
) -> Vec<TaintWarning> {
    find_taint_warnings_for_function_with_identities(
        fu,
        taints,
        executable_blocks,
        registry,
        TaintSourceContext::for_function(registry, fu),
        shadowed_builtins,
        module_traces,
        crate::realm::CommandBindingRealm::none(),
    )
}

/// Emit conditional sink warnings from a function in its original module.
/// The shared module/function owner validates availability, source profiles
/// and each retained lexer configuration. Missing agreement stays unavailable.
#[must_use]
pub fn find_taint_warnings_for_module_function<
    S: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    fu: &crate::compilation_unit::FunctionUnit,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    module: &crate::ir::Module,
    shadowed_builtins: &HashSet<String, H>,
    module_traces: crate::compilation_unit::ModuleTraceFacts<'_>,
) -> Vec<TaintWarning> {
    find_taint_warnings_for_function_with_identities(
        fu,
        taints,
        executable_blocks,
        registry,
        TaintSourceContext::for_module_function(registry, module, fu),
        shadowed_builtins,
        module_traces,
        crate::realm::CommandBindingRealm::none(),
    )
}

/// Retain original Module correspondence and command-head replay availability.
/// Stored callbacks are included by the whole-unit report. `solved` is the
/// interprocedural solve the taints came from, whose return summaries judge an
/// inline procedure call in a sink operand.
#[must_use]
pub fn taint_warning_report_for_module_function<
    S: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    fu: &crate::compilation_unit::FunctionUnit,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    (module, solved): (
        &crate::ir::Module,
        Option<&crate::taint_interproc::InterprocTaintResult>,
    ),
    shadowed_builtins: &HashSet<String, H>,
    module_traces: crate::compilation_unit::ModuleTraceFacts<'_>,
) -> TaintWarningReport {
    taint_warning_report_for_function_with_identities(
        fu,
        taints,
        executable_blocks,
        registry,
        TaintSourceContext::for_module_function(registry, module, fu)
            .with_procedure_summaries(solved.map(|solved| &solved.summaries)),
        shadowed_builtins,
        (
            module_traces,
            crate::realm::CommandBindingRealm::none(),
            Some(module),
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn find_taint_warnings_for_function_with_identities<
    S: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    fu: &crate::compilation_unit::FunctionUnit,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
    shadowed_builtins: &HashSet<String, H>,
    module_traces: crate::compilation_unit::ModuleTraceFacts<'_>,
    identities: &crate::realm::CommandBindingRealm,
) -> Vec<TaintWarning> {
    taint_warning_report_for_function_with_identities(
        fu,
        taints,
        executable_blocks,
        registry,
        source,
        shadowed_builtins,
        (module_traces, identities, None),
    )
    .warnings
}

pub(super) fn taint_warning_report_for_function_with_identities<
    S: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    fu: &crate::compilation_unit::FunctionUnit,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
    shadowed_builtins: &HashSet<String, H>,
    (module_traces, identities, original_module): (
        crate::compilation_unit::ModuleTraceFacts<'_>,
        &crate::realm::CommandBindingRealm,
        Option<&crate::ir::Module>,
    ),
) -> TaintWarningReport {
    taint_warning_report_impl(
        &fu.cfg,
        &fu.ssa,
        taints,
        executable_blocks,
        &SinkWarningContext {
            registry,
            dialect: source.dialect,
            source,
            shadowed_builtins,
            original_module,
            unavailable_replays: None,
        },
        Some(CommandHeadContext {
            fu,
            module_traces,
            identities,
        }),
    )
}

impl<H> SinkWarningContext<'_, H> {
    /// The document's lexer grammar, read off the resolved profile — the
    /// expression AST's `Raw` fallback text is re-lexed under it.
    pub(super) fn grammar(&self) -> tcl_dialect::LexerGrammar {
        self.source.config.grammar_over(
            self.dialect
                .map_or_else(tcl_dialect::LexerGrammar::default, |profile| {
                    profile.grammar
                }),
        )
    }
    fn unavailable_replay(
        &self,
        span: Span,
        reason: UnavailableTaintReplayReason,
        command_head: Option<CommandHeadContext<'_>>,
    ) {
        let Some(out) = self.unavailable_replays else {
            return;
        };
        let span = command_head.map_or(span, |owner| owner.fu.abs_span(span));
        let residual = UnavailableTaintReplay {
            span,
            reason,
            source: self
                .original_module
                .map(TaintReplaySourceSnapshot::for_module),
        };
        let mut out = out.borrow_mut();
        if !out.contains(&residual) {
            out.push(residual);
        }
    }
}

fn taint_warning_report_impl<
    S: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    context: &SinkWarningContext<'_, H>,
    command_head_ctx: Option<CommandHeadContext<'_>>,
) -> TaintWarningReport {
    let unavailable_replays = std::cell::RefCell::new(Vec::new());
    let context = &SinkWarningContext {
        registry: context.registry,
        source: context.source,
        dialect: context.dialect,
        shadowed_builtins: context.shadowed_builtins,
        original_module: context.original_module,
        unavailable_replays: Some(&unavailable_replays),
    };
    let mut warnings: Vec<TaintWarning> = Vec::new();
    let guards =
        validators::ValidatorGuards::new(cfg, ssa, context.registry, context.source.config);
    let rebound = names_written_as_arguments(ssa);

    for bn in cfg_order(cfg) {
        if !executable_blocks.contains(&bn) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&bn) else {
            continue;
        };

        for (index, ssa_stmt) in ssa_block.statements.iter().enumerate() {
            let from = warnings.len();
            emit_statement_warnings(
                ssa_stmt,
                taints,
                &mut warnings,
                crate::ssa::SsaSourceView::at_statement(ssa, bn, index),
                context,
                command_head_ctx,
            );
            retain_unrebound_advice(&ssa_stmt.statement, &rebound, &mut warnings, from);
            guards.retain_unvalidated(
                bn,
                &ssa_stmt.uses,
                context.shadowed_builtins,
                &mut warnings,
                from,
            );
        }
        let branch_ctx = BranchTaintCtx {
            ssa_block,
            ssa,
            taints,
            registry: context.registry,
            dialect: context.dialect,
            source: context.source,
        };
        emit_branch_condition_warnings(cfg, bn, &branch_ctx, &mut warnings);
    }

    let metadata_available =
        context.source.allows_nominal_metadata() || context.source.metadata.is_some();
    let unavailable = unavailable_replays.into_inner();
    let replays = if metadata_available {
        TaintReplayAnalysis::Analysed { unavailable }
    } else {
        TaintReplayAnalysis::Unavailable { unavailable }
    };
    TaintWarningReport { warnings, replays }
}

/// Every word a statement of this function passes as an argument. A
/// withdrawn-world head named here may have been rebound (`rename p puts`,
/// `interp alias {} puts {} p`), so it is no advice.
fn names_written_as_arguments(ssa: &SsaFunction) -> HashSet<String> {
    ssa.blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(|statement| statement.statement.tokens())
        .flat_map(|tokens| tokens.argv_texts.iter().skip(1))
        .map(|word| tcl_syntax::naming::without_leading_colons(word).to_owned())
        .collect()
}

/// Drop the warnings a statement's withdrawn-world advice raised when the
/// function may have rebound that head.
fn retain_unrebound_advice(
    statement: &crate::ir::Statement,
    rebound: &HashSet<String>,
    warnings: &mut Vec<TaintWarning>,
    from: usize,
) {
    let Some(head) = statement
        .tokens()
        .and_then(|tokens| tokens.source_binding.as_ref())
        .filter(|binding| binding.unknown)
        .and_then(|binding| binding.withdrawn_world_head.as_deref())
        .map(tcl_syntax::naming::without_leading_colons)
        .filter(|head| rebound.contains(*head))
    else {
        return;
    };
    let mut index = from;
    while index < warnings.len() {
        if warnings[index].sink_command == head {
            warnings.remove(index);
        } else {
            index += 1;
        }
    }
}

/// Per-block context for the branch-condition taint scan, bundled so
/// [`emit_branch_condition_warnings`] / [`emit_branch_condition_nested_command_warnings`]
/// stay under the 7-argument clippy limit — mirrors [`TaintScan`] /
/// [`SinkCall`] elsewhere in this module.
pub(super) struct BranchTaintCtx<'a, S> {
    pub(super) source: TaintSourceContext<'a>,
    /// The condition's own block — `exit_versions` gives the SSA version of
    /// each condition variable reaching the terminator.
    ssa_block: &'a crate::ssa::SsaBlock,
    pub(super) ssa: &'a SsaFunction,
    pub(super) taints: &'a HashMap<ValueKey, TaintLattice, S>,
    pub(super) registry: &'a CommandRegistry,
    pub(super) dialect: Option<&'static tcl_dialect::DialectProfile>,
}
