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

//! Globals that hold factory instances and globals tainted by registry-declared instance configuration, applied to the CFG before SSA.

use super::eager::EagerInvocations;
use super::summary_fixpoints::summary_procedure_bodies;
use crate::ir::ProcedureId;
use std::collections::{HashMap, HashSet};

/// Conditional factory receiver candidates at each procedure's earliest
/// original source phase. Availability and original naming operands must join
/// the same retained Module interpretation. No factory execution, current
/// command allocation, callback frame or instance dispatch follows from this
/// reporting projection.
#[must_use]
pub(crate) fn global_instance_classes(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<ProcedureId, crate::taint::InstanceClassState> {
    global_instance_classes_with(
        ir_module,
        registry,
        &EagerInvocations::of(ir_module, registry),
        &summary_procedure_bodies(ir_module, registry),
    )
}

/// Retain the producer's complete source world once for either interprocedural
/// summaries or their smaller receiver-only projection.
pub(crate) fn global_instance_class_source(
    module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
) -> Option<(
    crate::analyser::ResolvedAnalysisInput,
    std::sync::Arc<crate::command_binding::RetainedSourceModuleBindings>,
)> {
    let source = crate::taint::TaintSourceContext::for_module(registry, module);
    let input = source.metadata_context()?.source_analysis_input()?.clone();
    let owner = module.retained_source_bindings.as_ref()?;
    Some((input, std::sync::Arc::clone(owner)))
}

/// [`global_instance_classes`] over the module's eager invocations, which a
/// summary build computes once for this proof and the completion proof.
///
/// A procedure the top-level script may run while it is still running may be
/// called before a later constructor; one only a callback runs starts after
/// direct top-level setup ([`EagerInvocations::named`]).
#[must_use]
pub(crate) fn global_instance_classes_with(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    invocations: &EagerInvocations,
    source_bodies: &HashMap<ProcedureId, &crate::ir::Procedure>,
) -> HashMap<ProcedureId, crate::taint::InstanceClassState> {
    let source = crate::taint::TaintSourceContext::for_module(registry, ir_module);
    // Each statement retains its own original words and lookup horizon.
    // Nested source effects only withdraw candidates; they do not prove that
    // a conditional factory, mutation or teardown actually ran.
    let mut sites = Vec::new();
    let mut scripts = vec![(&ir_module.top_level, true)];
    while let Some((script, direct)) = scripts.pop() {
        for statement in &script.statements {
            let tokens = script.retained_source_tokens_for_statement(statement);
            if tokens.is_some()
                || matches!(
                    statement,
                    crate::ir::Statement::Call { .. }
                        | crate::ir::Statement::Barrier { .. }
                        | crate::ir::Statement::NativeCall { .. }
                )
            {
                sites.push((statement, tokens, direct));
            }
            scripts.extend(
                crate::ir_helpers::nested_bodies(statement)
                    .into_iter()
                    .map(|body| (body, false)),
            );
        }
    }
    sites.sort_by_key(|(statement, _, _)| statement.span().start());
    // Callable bodies keep each actual eager horizon. A readonly declaration
    // can only borrow setup preceding its own source site. Each horizon sees
    // a prefix of the ordered sites, so one ordered pass serves them all.
    let mut horizons: Vec<(Option<u32>, ProcedureId)> = Vec::new();
    for (id, procedure) in source_bodies {
        if ir_module.procedures.contains_key(id) {
            let before = horizons.len();
            horizons.extend(invocations.named_positions(id).map(|at| (Some(at), *id)));
            if horizons.len() == before {
                horizons.push((None, *id));
            }
        } else {
            horizons.push((Some(procedure.span.start()), *id));
        }
    }
    horizons.sort_by_key(|(eager, _)| eager.map_or(u64::MAX, u64::from));
    let mut classes = crate::taint::InstanceClassState::new();
    let mut applied = 0;
    let mut out: HashMap<ProcedureId, crate::taint::InstanceClassState> =
        HashMap::with_capacity(horizons.len());
    for (eager, id) in horizons {
        while let Some(&(statement, tokens, direct)) = sites.get(applied)
            && eager.is_none_or(|call| statement.span().start() < call)
        {
            applied += 1;
            if direct {
                crate::taint::transfer_instance_command(&mut classes, registry, source, tokens);
            } else {
                let mut candidate = classes.clone();
                crate::taint::transfer_instance_command(&mut candidate, registry, source, tokens);
                classes.retain(|name, class| candidate.get(name) == Some(class));
            }
        }
        // A receiver is typed by every invocation that sees it. One that runs
        // before the constructor fails at the lookup, before any sink, so it
        // withdraws nothing a later invocation proves; a class that differs
        // between invocations joins to a mixed set the consumers abstain on.
        let joined = out.entry(id).or_default();
        for (name, class) in &classes {
            joined
                .entry(name.clone())
                .or_default()
                .extend(class.iter().cloned());
        }
    }
    out
}

/// Global variables tainted by registry-declared instance configuration in a
/// procedure, closed transitively over calls to other procedures.
#[must_use]
pub(crate) fn tainted_global_writes(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    global_classes: &HashMap<ProcedureId, crate::taint::InstanceClassState>,
) -> HashMap<ProcedureId, HashSet<String>> {
    let mut cfg_module = crate::cfg_builder::build_cfg_with_registry_and_config(
        ir_module,
        false,
        registry,
        ir_module.lexer_config,
    );
    let writes =
        direct_instance_option_writes(ir_module, &mut cfg_module, registry, global_classes);
    close_tainted_global_writes(ir_module, registry, writes)
}

pub(super) fn tainted_global_writes_from_cfg(
    ir_module: &crate::ir::Module,
    cfg_module: &crate::cfg::CfgModule,
    registry: &tcl_registry::CommandRegistry,
    global_classes: &HashMap<ProcedureId, crate::taint::InstanceClassState>,
) -> HashMap<ProcedureId, HashSet<String>> {
    let mut cfg_module = cfg_module.clone();
    let writes =
        direct_instance_option_writes(ir_module, &mut cfg_module, registry, global_classes);
    close_tainted_global_writes(ir_module, registry, writes)
}

fn retain_global_call_defs(defs: &mut HashSet<String>, call_defs: &[String]) {
    defs.extend(
        call_defs
            .iter()
            .filter(|name| tcl_syntax::naming::is_absolute(name))
            .cloned(),
    );
}

fn direct_instance_option_writes(
    ir_module: &crate::ir::Module,
    cfg_module: &mut crate::cfg::CfgModule,
    registry: &tcl_registry::CommandRegistry,
    global_classes: &HashMap<ProcedureId, crate::taint::InstanceClassState>,
) -> HashMap<ProcedureId, HashSet<String>> {
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, ir_module);
    // Only an ownerless Module is the standalone compatibility path. A module
    // that carries source input/bindings but cannot reborrow a selected source
    // invocation must conservatively retain its existing global defs.
    let standalone =
        ir_module.source_metadata_input.is_none() && ir_module.retained_source_bindings.is_none();
    cfg_module
        .procedures
        .iter_mut()
        .map(|(id, cfg)| {
            let initial = global_classes.get(id).cloned().unwrap_or_default();
            let mut defs =
                crate::ssa::enrich_instance_option_defs_with_initial(cfg, registry, &initial);
            defs.retain(|name| tcl_syntax::naming::is_absolute(name));
            for block in cfg.blocks.values() {
                for statement in &block.statements {
                    let crate::ir::Statement::Call {
                        command,
                        canonical_command,
                        args,
                        defs: call_defs,
                        tokens,
                        ..
                    } = statement
                    else {
                        continue;
                    };
                    if let Some(metadata) = metadata {
                        let Some(tokens) = tokens.as_ref() else {
                            retain_global_call_defs(&mut defs, call_defs);
                            continue;
                        };
                        // Prefer the genuine point-local registered receiver
                        // owner. The command source owner covers ordinary
                        // selected option writers when no instance applies.
                        let selected =
                            crate::ssa::original_instance_option_writes(tokens, metadata, registry)
                                .or_else(|| {
                                    crate::ssa::original_command_taint_option_writes(
                                        tokens, metadata, registry,
                                    )
                                });
                        let Some(selected) = selected else {
                            retain_global_call_defs(&mut defs, call_defs);
                            continue;
                        };
                        let mut joined_every_target = true;
                        for write in selected
                            .iter()
                            .filter(|write| write.scope == tcl_registry::VariableScope::Global)
                        {
                            let Some(name) = crate::ssa::linked_option_definition_name(
                                tokens,
                                write.written_argument,
                                true,
                                metadata,
                                registry,
                            ) else {
                                joined_every_target = false;
                                continue;
                            };
                            // The selected source write and original written
                            // name are the authority for this summary. Do not
                            // rejoin them through the IR's display-name set.
                            defs.insert(name);
                        }
                        if !joined_every_target {
                            retain_global_call_defs(&mut defs, call_defs);
                        }
                        continue;
                    }
                    if !standalone {
                        retain_global_call_defs(&mut defs, call_defs);
                        continue;
                    }
                    let lookup = canonical_command.as_deref().unwrap_or(command);
                    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                    for variable in call_defs
                        .iter()
                        .filter(|name| tcl_syntax::naming::is_absolute(name))
                    {
                        if tcl_registry::taint::taints_var_write(
                            registry,
                            lookup,
                            &arg_refs,
                            registry.own_surface_query(),
                            variable,
                        ) {
                            defs.insert(variable.clone());
                        }
                    }
                }
            }
            (*id, defs)
        })
        .collect()
}

fn close_tainted_global_writes(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    mut writes: HashMap<ProcedureId, HashSet<String>>,
) -> HashMap<ProcedureId, HashSet<String>> {
    let known: HashSet<ProcedureId> = ir_module.procedures.keys().copied().collect();
    let mut calls: HashMap<ProcedureId, HashSet<ProcedureId>> = HashMap::new();
    for (id, procedure) in ir_module.procedures.iter() {
        let proc_calls = calls.entry(*id).or_default();
        crate::ir::for_each_statement(&procedure.body, &mut |statement| {
            let crate::ir::Statement::Call {
                command,
                tokens: Some(tokens),
                ..
            } = statement
            else {
                return;
            };
            if let Some(targets) =
                crate::registry_invocation::original_source_procedure_ids_for_word_for_module(
                    tokens, command, ir_module, registry,
                )
            {
                proc_calls.extend(targets.into_iter().filter(|id| known.contains(id)));
            }
        });
    }

    let mut changed = true;
    while changed {
        changed = false;
        for (caller, callees) in &calls {
            let inherited: HashSet<String> = callees
                .iter()
                .filter_map(|callee| writes.get(callee))
                .flatten()
                .cloned()
                .collect();
            let caller_writes = writes.entry(*caller).or_default();
            let old_len = caller_writes.len();
            caller_writes.extend(inherited);
            changed |= caller_writes.len() != old_len;
        }
    }
    writes
}

/// Apply the instance facts needed before SSA construction.
///
/// Direct configuration calls gain the registry-declared variable defs, and
/// internal procedure calls gain the transitive defs of their callees. This is
/// deliberately CFG annotation, not command-name logic: both the receiver
/// class and the option write come from registry descriptors.
pub(crate) fn enrich_instance_taint_cfg(
    ir_module: &crate::ir::Module,
    cfg_module: &mut crate::cfg::CfgModule,
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<ProcedureId, HashSet<String>> {
    let global_classes = global_instance_classes(ir_module, registry);
    let direct_writes =
        direct_instance_option_writes(ir_module, cfg_module, registry, &global_classes);
    let writes = close_tainted_global_writes(ir_module, registry, direct_writes);

    let annotate_calls = |cfg: &mut crate::cfg::Function| {
        for block in cfg.blocks.values_mut() {
            for statement in &mut block.statements {
                let crate::ir::Statement::Call {
                    command,
                    tokens: Some(tokens),
                    defs,
                    ..
                } = statement
                else {
                    continue;
                };
                let Some(targets) =
                    crate::registry_invocation::original_source_procedure_ids_for_word_for_module(
                        tokens, command, ir_module, registry,
                    )
                else {
                    continue;
                };
                for target in targets {
                    for variable in writes.get(&target).into_iter().flatten() {
                        if !defs.contains(variable) {
                            defs.push(variable.clone());
                        }
                    }
                }
            }
        }
    };
    annotate_calls(&mut cfg_module.top_level);
    for cfg in cfg_module.procedures.values_mut() {
        annotate_calls(cfg);
    }
    writes
}
