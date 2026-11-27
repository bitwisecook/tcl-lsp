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

//! Summaries of `TclOO` method bodies and the instance-variable writes they make.

use super::body_scan::scan_script;
use super::return_summaries::{SeedlessAnswer, script_always_returns, summarise_returns};
use super::{
    LocalFacts, MAX_INTERPROCEDURAL_WALK_DEPTH, MethodSummary, ObjectTypeMap, ProcFixpoints,
    ProcSummary, ReturnKind, ScanCtx, arity_from_names,
};
use crate::ir::ProcedureId;
use crate::side_effects::EffectRegion;
use std::collections::{HashMap, HashSet};

/// Summarise `TclOO` method bodies into [`MethodSummary`] entries.
/// The purity rule is intentionally conservative — a method is
/// pure iff its own body has no observable side effect (no barrier, no
/// unknown call, no global write, no instance-var write, no local
/// effect write) **and** every *proc* it calls is pure. A `my
/// <method>` / `next` self-dispatch resolves to no registry trait, so
/// it falls through to an unknown-state effect write and forces the
/// method impure — we never mark a method pure on the strength of an
/// unproven peer method (sound: false negatives only). A redefined
/// method is forced impure (we can't prove which body a dispatch
/// runs).
pub(super) fn build_method_summaries(
    ir_module: &crate::ir::Module,
    known: &HashSet<ProcedureId>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    identities: &crate::realm::CommandBindingRealm,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    procs: ProcFixpoints<'_>,
) -> HashMap<crate::ir::ProcedureId, MethodSummary> {
    let ProcFixpoints {
        pure,
        effect_reads,
        effect_writes,
    } = procs;
    if ir_module.methods.is_empty() {
        return HashMap::new();
    }
    // A call resolving to a method qname is "known" (not unknown), so
    // seed the resolver with both procs and method names.
    let method_known = known.clone();

    let source_input =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, ir_module)
            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input);
    let mut out: HashMap<crate::ir::ProcedureId, MethodSummary> =
        HashMap::with_capacity(ir_module.methods.len());
    for (id, method) in &ir_module.methods {
        let mqname = method.reporting_label();
        let mut facts = LocalFacts {
            local_pure: true,
            ..LocalFacts::default()
        };
        // Scan the primary body, then — the lowering retains them — every
        // replacement body of a redefined method, all into the SAME fact
        // accumulators: the summary describes the union of every body a
        // dispatch may run — pure only when all are, constant-return only
        // when every body's exits agree on the one constant. A join rather
        // than an abstain-on-redefinition kill switch: more precise, equally
        // sound.
        let mut written_ivars: HashSet<String> = HashSet::new();
        for (_, body_def) in ir_module.method_definition_bodies(*id) {
            scan_method_body_facts(
                body_def,
                MethodScan {
                    method_known: &method_known,
                    registry,
                    dialect,
                    identities,
                    declared,
                    source_input,
                    source_procedures: &ir_module.procedures,
                    source_module: ir_module,
                },
                &mut facts,
                &mut written_ivars,
            );
        }

        // `local_pure` is the authoritative local-impurity signal (the
        // proc fixpoint keys off the same flag): it is cleared by
        // barriers, unknown calls, global writes, AND any command whose
        // classification is impure — including region-free effects like
        // `puts`/`log` (FILE_IO/LOG_IO write, `EffectRegion::NONE`). The
        // old `effect_writes == NONE` proxy missed that last case, marking
        // a `puts`-calling method spuriously pure. Instance-var writes
        // (plain local `set ivar …`, region-free + locally pure) stay a
        // separate method-only term.
        let pure_base = facts.local_pure && written_ivars.is_empty();
        let mut is_pure = pure_base
            && facts
                .direct_calls
                .iter()
                .all(|c| pure.get(c).copied().unwrap_or(false));
        // A class with a member the lowering could not model (dynamic
        // member name / body), or a module with a dynamic OO definition
        // target, may have replaced ANY of its methods with an unknown
        // body — force impure, since no retained-body scan covers what
        // was never readable. Statically-retained redefinitions are
        // covered by the replacement-body scan above instead.
        if ir_module.oo_evidence.dynamic_target
            || ir_module.oo_unanalysed_classes.contains(&method.class_name)
        {
            is_pure = false;
        }

        // Effects: the method's own local effects unioned with the
        // (transitive) effects of every proc callee. Non-proc callees
        // (method qnames) default to NONE.
        let mut m_reads = facts.effect_reads;
        let mut m_writes = facts.effect_writes;
        for c in &facts.direct_calls {
            m_reads |= effect_reads.get(c).copied().unwrap_or(EffectRegion::NONE);
            m_writes |= effect_writes.get(c).copied().unwrap_or(EffectRegion::NONE);
        }

        let mut calls: Vec<ProcedureId> = facts.direct_calls.iter().copied().collect();
        calls.sort();
        let direct_calls = calls.clone();
        let (returns_constant, constant_return, passthrough, depends) =
            summarise_returns(&facts.returns, SeedlessAnswer::NotRun);

        out.insert(
            *id,
            MethodSummary {
                base: ProcSummary {
                    qualified_name: mqname.clone(),
                    params: method.params.clone(),
                    arity: arity_from_names(&method.params),
                    calls,
                    direct_calls,
                    has_barrier: facts.has_barrier,
                    has_unknown_calls: facts.has_unknown_calls,
                    writes_global: facts.writes_global,
                    pure: is_pure,
                    // A method's command is looked up at run time, so no
                    // call to it is proved to complete.
                    completes: false,
                    defined_at: None,
                    defined_callees: Vec::new(),
                    effect_reads: m_reads,
                    effect_writes: m_writes,
                    returns_constant,
                    constant_return,
                    return_depends_on_params: depends,
                    return_passthrough_param: passthrough,
                    // Methods are not folded at static call sites.
                    can_fold_static_calls: false,
                    param_traits: HashMap::new(),
                    param_defaults: HashMap::new(),
                },
                class_name: method.class_name.clone(),
                method_kind: method.kind,
                // Read-set / MRO-dispatch tracking is not implemented;
                // left empty (the purity gate consumes only `pure`).
                reads_instance_vars: HashSet::new(),
                writes_instance_vars: written_ivars,
                calls_my: Vec::new(),
                calls_next: false,
            },
        );
    }
    out
}

/// Scan one method body's facts into the shared accumulators for
/// [`build_method_summaries`]: the local-purity / call / effect scan, the
/// fall-through return, and the instance-variable writes. Called once for
/// the primary [`crate::ir::MethodDef`] and once per retained replacement
/// body, so the summary joins over every body a dispatch may run.
#[derive(Clone, Copy)]
struct MethodScan<'a> {
    method_known: &'a HashSet<ProcedureId>,
    pub(super) registry: &'a tcl_registry::CommandRegistry,
    pub(super) dialect: Option<&'static tcl_dialect::DialectProfile>,
    pub(super) identities: &'a crate::realm::CommandBindingRealm,
    /// The document's own command declarations — see [`ProcScan::declared`].
    pub(super) declared: Option<&'a tcl_registry::model::DeclaredSurface>,
    pub(super) source_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    pub(super) source_procedures: &'a crate::ir::ProcedureInventory,
    pub(super) source_module: &'a crate::ir::Module,
}

fn scan_method_body_facts(
    body_def: &crate::ir::MethodDef,
    scan: MethodScan<'_>,
    facts: &mut LocalFacts,
    written_ivars: &mut HashSet<String>,
) {
    let MethodScan {
        method_known,
        registry,
        dialect,
        identities,
        declared,
        source_input,
        source_procedures,
        source_module,
    } = scan;
    let params: HashSet<String> = body_def.params.iter().cloned().collect();
    if matches!(
        body_def.execution_namespace,
        crate::ir::ExecutionNamespace::RuntimeSelected
    ) && crate::ir_helpers::requires_runtime_command_namespace(&body_def.body, registry)
    {
        // A TclOO receiver namespace can contain an object-local command that
        // shadows every relative fallback considered below. Preserve useful
        // known fallback edges, but never prove purity or bounded state from
        // that incomplete target set.
        facts.has_barrier = true;
        facts.has_unknown_calls = true;
        facts.mark_impure();
        facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
        facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
    }
    let ctx = ScanCtx {
        known: method_known,
        registry,
        dialect,
        params: &params,
        // Method bodies are not call-graph nodes; no object-type map needed.
        object_types: ObjectTypeMap::none().candidates,
        identities,
        declared,
        positioned_substitutions: false,
        source_input,
        source_procedures,
        source_module,
        current_tokens: None,
        parameter_body: None,
    };
    scan_script(&body_def.body, ctx, facts, 0);
    // Fall-through exit is non-constant (O103); see `scan_proc`.
    if !script_always_returns(&body_def.body, 0) {
        facts.returns.push(ReturnKind::Other);
    }
    // Instance-variable writes mutate object state and survive the call —
    // so a method that writes any in-scope instance var is impure even
    // though the write looks like a plain local `set`.
    if body_def.body.executed_source.is_some() {
        if original_method_has_outward_write(&body_def.body, registry, source_input) {
            facts.local_pure = false;
            facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
        }
    } else if !body_def.instance_vars.is_empty() {
        collect_instance_var_writes(&body_def.body, &body_def.instance_vars, written_ivars, 0);
        collect_possible_instance_var_writes(
            &body_def.body,
            &body_def.instance_vars,
            registry,
            source_input,
            written_ivars,
        );
    }
}

/// Original object writes are selected from actual operand/context owners.
/// Missing normal binding evidence remains a hazard, never a private local.
fn original_method_has_outward_write(
    script: &crate::ir::Script,
    registry: &tcl_registry::CommandRegistry,
    source_input: Option<&crate::analyser::ResolvedAnalysisInput>,
) -> bool {
    let Some(context) = source_input.map(crate::analyser::ResolvedAnalysisInput::context_registry)
    else {
        return true;
    };
    let metadata = Some(context.as_ref().into());
    let mut pending = vec![(script, 0)];
    while let Some((script, depth)) = pending.pop() {
        if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
            return true;
        }
        for statement in &script.statements {
            let tokens = script.retained_source_tokens_for_statement(statement);
            if let Some(names) = tokens.and_then(|tokens| {
                crate::registry_invocation::possible_variable_name_operands_with_metadata_context(
                    registry, metadata, tokens,
                )
            }) {
                if names
                    .unresolved_roles()
                    .any(|role| role == tcl_registry::ArgRole::VarWrite)
                {
                    return true;
                }
                if names.contents_write_operands().next().is_some() {
                    let Some(tokens) = tokens else {
                        return true;
                    };
                    let Some(binding) = tokens.source_binding.as_ref() else {
                        return true;
                    };
                    let Some(normal) = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                        registry, metadata, tokens,
                    ) else {
                        return true;
                    };
                    let context = binding
                        .normal_variable_continuation()
                        .unwrap_or(&binding.variable_context);
                    let writes = normal.mutation_places(context, registry);
                    if writes.is_empty()
                        || writes.iter().any(|place| {
                            place.dynamic || place.observed || place.cell.as_ref().is_none_or(|cell|
                            !matches!(&cell.owner, tcl_names::place::CellOwner::Activation(identity)
                                if context.activation.as_ref() == Some(identity)))
                        })
                    {
                        return true;
                    }
                }
            }
            pending.extend(
                statement
                    .child_scripts()
                    .into_iter()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
    false
}

/// Candidate name operands identify possible instance-write hazards, even
/// when receiver lookup prevents a Must handler or physical destination proof.
fn collect_possible_instance_var_writes(
    script: &crate::ir::Script,
    ivars: &HashSet<String>,
    registry: &tcl_registry::CommandRegistry,
    source_input: Option<&crate::analyser::ResolvedAnalysisInput>,
    out: &mut HashSet<String>,
) {
    use crate::registry_invocation::EffectiveInvocationWord;
    let Some(context) = source_input.map(crate::analyser::ResolvedAnalysisInput::context_registry)
    else {
        out.extend(ivars.iter().cloned());
        return;
    };
    let metadata = Some(context.as_ref().into());
    let mut pending = vec![(script, 0)];
    while let Some((script, depth)) = pending.pop() {
        if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
            continue;
        }
        for statement in &script.statements {
            if let Some(names) = statement.tokens().and_then(|tokens| {
                crate::registry_invocation::possible_variable_name_operands_with_metadata_context(
                    registry, metadata, tokens,
                )
            }) {
                for operand in names.contents_write_operands() {
                    match operand {
                        EffectiveInvocationWord::Literal(name) => {
                            check_ivar_write(name, ivars, out);
                        }
                        EffectiveInvocationWord::ArrayElementName { root } => {
                            check_ivar_write(root, ivars, out);
                        }
                        EffectiveInvocationWord::ByteLiteral(_)
                        | EffectiveInvocationWord::Dynamic
                        | EffectiveInvocationWord::Expanded
                        | EffectiveInvocationWord::KnownExpansion(_)
                        | EffectiveInvocationWord::KnownByteExpansion(_)
                        | EffectiveInvocationWord::Opaque => out.extend(ivars.iter().cloned()),
                    }
                }
                if names
                    .unresolved_roles()
                    .any(|role| role == tcl_registry::ArgRole::VarWrite)
                {
                    out.extend(ivars.iter().cloned());
                }
            }
            pending.extend(
                statement
                    .child_scripts()
                    .into_iter()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
}

/// Recursively collect the base names of instance-variable *writes* in
/// a method body, comparing each written name against `ivars`. Walks
/// the CFG, counting explicit assign / incr / loop / catch targets.
/// Positioned Call hazards come from the candidate contents-write owner;
/// unpositioned IR conservatively retains its declared `defs`. Array-element
/// writes (`counter(0)`) compare on the base scalar name so they are
/// not missed. Over-approximating writes is the sound direction (a
/// spurious write only costs an O126 fold; a missed one would wrongly
/// delete a state mutation).
#[allow(clippy::too_many_lines)] // the depth-cap check adds a few lines over the threshold
pub(super) fn collect_instance_var_writes(
    script: &crate::ir::Script,
    ivars: &HashSet<String>,
    out: &mut HashSet<String>,
    depth: u32,
) {
    use crate::ir::Statement;
    if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::AssignConst { .. }
            | Statement::AssignExpr { .. }
            | Statement::AssignValue { .. }
            | Statement::Incr { .. } => {
                let projection = crate::ir_helpers::structural_variable_write_projection(stmt);
                if projection.opaque_variable_frame {
                    out.extend(ivars.iter().cloned());
                }
                for name in &projection.literal_names {
                    check_ivar_write(name, ivars, out);
                }
            }
            Statement::Call { defs, tokens, .. }
                if tokens
                    .as_ref()
                    .is_none_or(|tokens| tokens.source_binding.is_none()) =>
            {
                for d in defs {
                    check_ivar_write(d, ivars, out);
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    collect_instance_var_writes(&clause.body, ivars, out, depth + 1);
                }
                if let Some(eb) = else_body {
                    collect_instance_var_writes(eb, ivars, out, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                collect_instance_var_writes(init, ivars, out, depth + 1);
                collect_instance_var_writes(next, ivars, out, depth + 1);
                collect_instance_var_writes(body, ivars, out, depth + 1);
            }
            Statement::Foreach {
                iterators, body, ..
            } => {
                for it in iterators {
                    for v in &it.vars {
                        check_ivar_write(v, ivars, out);
                    }
                }
                collect_instance_var_writes(body, ivars, out, depth + 1);
            }
            Statement::Catch {
                body,
                result_var,
                options_var,
                ..
            } => {
                collect_instance_var_writes(body, ivars, out, depth + 1);
                if let Some(rv) = result_var {
                    check_ivar_write(rv, ivars, out);
                }
                if let Some(ov) = options_var {
                    check_ivar_write(ov, ivars, out);
                }
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                collect_instance_var_writes(body, ivars, out, depth + 1);
                for h in handlers {
                    if let Some(v) = &h.var_name {
                        check_ivar_write(v, ivars, out);
                    }
                    if let Some(ov) = &h.options_var {
                        check_ivar_write(ov, ivars, out);
                    }
                    collect_instance_var_writes(&h.body, ivars, out, depth + 1);
                }
                if let Some(fb) = finally_body {
                    collect_instance_var_writes(fb, ivars, out, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(b) = &arm.body {
                        collect_instance_var_writes(b, ivars, out, depth + 1);
                    }
                }
                if let Some(db) = default_body {
                    collect_instance_var_writes(db, ivars, out, depth + 1);
                }
            }
            Statement::While { body, .. }
            | Statement::Block { body, .. }
            | Statement::UpFrame { body, .. } => {
                collect_instance_var_writes(body, ivars, out, depth + 1);
            }
            _ => {}
        }
    }
}

/// Record `raw` as an instance-var write when its base scalar name (an
/// array element `arr(idx)` compares on `arr`) is a declared instance
/// var.
pub(super) fn check_ivar_write(raw: &str, ivars: &HashSet<String>, out: &mut HashSet<String>) {
    let base = tcl_syntax::naming::split_element_ref(raw).map_or(raw, |(root, _)| root);
    if ivars.contains(base) {
        out.insert(base.to_owned());
    }
}
