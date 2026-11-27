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

//! O129: folding the command substitutions a `TclOO` method frame makes constant.

use super::super::{Optimisation, PassContext};
use super::command_substitution_folds::{
    fold_retained_builtin, module_script_encoding, retained_call_at, retained_substitution_calls,
};
use super::constant_substitution::visit_call_tokens;
use super::return_folds::original_return_value_replacement;
use super::{render_propagation_word, sccp_constants_from};
use crate::compilation_unit::CompilationUnit;
use crate::ir::{CommandTokens, Script, Statement};
use crate::tcl_expr_eval::FoldPolicy;
use tcl_core_types::DiagCode;

/// Original declaring-class report, separate from method display metadata.
#[derive(Debug, Clone)]
pub(super) struct OoFrame {
    pub(super) defining_class: String,
}

fn oo_frame_for(
    method: &crate::ir::MethodDef,
    mutations: &crate::command_binding::ModuleCommandMutations,
    world: &crate::command_binding::OriginalCompletedCommandWorld,
) -> Option<OoFrame> {
    // Relative `self` can be shadowed in a receiver-selected namespace.
    // The natural `[self class]` spelling deliberately remains unchanged
    // until the original command target is proved for that method frame.
    if mutations.has_runtime_selected_frames() || method.kind == crate::ir::MethodKind::ClassMethod
    {
        return None;
    }
    let (defining_class, original_operand) = world.defining_class_name_for_body(method)?;
    if !mutations.trusts_proc_binding(&original_operand) {
        return None;
    }
    Some(OoFrame { defining_class })
}

/// The method-local constants a method body may propagate, or an empty map
/// when it may propagate none.
///
/// This applies `elimination.rs`'s escaping model to the propagation
/// lattice.  `elimination.rs` already knows that a `TclOO` instance variable
/// escapes the method frame — it feeds
/// [`crate::ir::MethodDef::instance_vars`] through the same channel iRules
/// cross-event state uses, so a state-mutating `set ivar …` is never deleted
/// as a dead store.  The propagation lattice needs the same model, or this
/// walk can only carry an empty constants map.
///
/// SCCP is therefore **re-run** for the method with those names in its
/// escaping set, rather than the shared [`FunctionUnit::sccp`] being rebuilt
/// that way: forcing them `Overdefined` also poisons everything *derived*
/// from them (`set a $ivar ; set b $a`), which no filter on a projected map
/// could do — but it is a deliberately propagation-only view.  The unit's own
/// lattice stays as built, because other consumers read facts an instance
/// variable legitimately carries (object-collection element typing harvests
/// `dict set pins $k [Pin new]` out of exactly such a name).
///
/// What survives the projection is provably method-local: a name the class
/// never declares as state, never aliased by `variable` / `my variable` /
/// `upvar` / `global` inside the body (the per-function
/// [`crate::var_observability`] scan SCCP runs covers those), and not under a
/// trace.  A `my …` / `next` / `[self …]` dispatch is then no longer a barrier
/// for it: object state is the only thing such a dispatch can reach, and every
/// name that names object state has already left the map.
pub(super) fn oo_method_constants(
    ctx: &PassContext<'_>,
    cu: &CompilationUnit,
    body_id: &crate::ir::ProcedureId,
    frame: &OoFrame,
    allow_locals: bool,
) -> std::collections::HashMap<String, String> {
    if !allow_locals {
        return std::collections::HashMap::new();
    }
    let (Some(fu), Some(registry)) = (cu.methods.get(body_id), ctx.registry) else {
        return std::collections::HashMap::new();
    };
    if fu.complexity_guarded {
        return std::collections::HashMap::new();
    }
    // This re-run bypasses the unit build's dynamic-name widening of the
    // SCCP escaping switch, so apply the same abstention here: a computed
    // variable name in the body makes no method-local constant trustworthy
    if fu.dynamic_barrier_blocks_value_motion() {
        return std::collections::HashMap::new();
    }
    // The re-run also carries the registry builtin-fold context:
    // `set base [self class]; set ns [namespace qualifiers $base]`
    // folds to fixpoint *inside* the lattice, so `base` and `ns` both
    // project as method-local constants rather than the chain stopping
    // after the first O129 suggestion. The frame's defining class is
    // proven by [`oo_frame_for`], so the `[self class]` frame fact is
    // sound here and only here.
    //
    // The escaping set is the unit's own `method_facts` carrier — the same
    // struct the existence fold and the W-family diagnostics read — never a
    // second lookup of `MethodDef::instance_vars`.
    let Some(facts) = fu.method_facts.as_deref() else {
        return std::collections::HashMap::new();
    };
    let sccp = crate::sccp::sccp_in_module(&crate::sccp::SolveInputs {
        cfg: &fu.cfg,
        ssa: &fu.ssa,
        param_constants: None,
        policy: FoldPolicy::from_registry(registry),
        extra_escaping: &facts.instance_vars,
        trace: crate::sccp::TraceInputs {
            registry,
            source_metadata_input: fu
                .invocation_metadata_context_for_module(registry, &cu.ir_module)
                .and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                ),
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: &cu.ir_module.deferred_writes,
            analysis_context: None,
            existence: None,
        },
        folds: Some(crate::sccp::BuiltinFoldInputs {
            registry,
            source_metadata_input: fu
                .invocation_metadata_context_for_module(registry, &cu.ir_module)
                .and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                ),
            mutations: &ctx.command_mutations,
            dialect: ctx.dialect,
            defining_class: Some(&frame.defining_class),
            registry_engine: true,
            trust: crate::sccp::FoldTrust::WholeModule,
            proven_pure_parameters: false,
        }),
        module: crate::sccp::ModuleRun {
            analysis_body: Some(crate::sccp::AnalysisBody::Procedure(*body_id)),
            ..crate::sccp::ModuleRun::NONE
        },
    });
    sccp_constants_from(&sccp, &fu.ssa, ctx.fold_policy())
}

/// Fold the command substitutions in every `TclOO` method body that the
/// enclosing method frame makes constant — `[self class]`, anything built
/// purely out of it (`[namespace qualifiers [self class]]` folds in one step,
/// since the builtin fold resolves nested substitutions first), and anything
/// built out of a *provably method-local* variable as well.
///
/// Deliberately narrower than [`run_function`]: no O103 static-proc-call fold
/// and no `namespace`-relative chain resolution runs here.  The constants map
/// comes from [`oo_method_constants`], which models which names are object
/// state.
pub(super) fn run_oo_method_folds(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let Some(bindings) = cu
        .ir_module
        .retained_source_bindings
        .as_deref()
        .filter(|bindings| bindings.matches_module(&cu.ir_module, registry))
    else {
        return;
    };
    let Some(world) = bindings
        .positioned_for_module(&cu.ir_module, registry)
        .and_then(crate::command_binding::SourceCommandBindings::original_completed_command_world)
    else {
        return;
    };
    // The PER-METHOD dispatch barrier: a method is barred
    // only when its dispatches can actually reach a caller-frame-reaching
    // (or unreadable) method — see `super::super::method_barrier`. The registry is
    // present (checked above), so unwrap-by-default to an all-barred
    // stance is unreachable; a bare context without one folds nothing
    // anyway.
    let barrier = ctx
        .registry
        .map(|registry| super::super::method_barrier::compute(&cu.ir_module, registry));
    // Sorted so the emitted rewrite order is deterministic across runs
    // (`ir_module.methods` is a `HashMap`).
    let mut qnames: Vec<&crate::ir::ProcedureId> = cu.ir_module.methods.keys().collect();
    qnames.sort();
    for qname in qnames {
        let Some(method) = cu.ir_module.methods.get(qname) else {
            continue;
        };
        let Some(frame) = oo_frame_for(method, &ctx.command_mutations, &world) else {
            continue;
        };
        let allow_locals = barrier.as_ref().is_some_and(|b| b.allows_locals(*qname));
        let constants = oo_method_constants(ctx, cu, qname, &frame, allow_locals);
        walk_oo_script(ctx, cu, &method.body, &frame, &constants, 0);
    }
}

/// `depth` is the nesting level of `script` — see
/// [`super::super::MAX_OPTIMISER_WALK_DEPTH`].
fn walk_oo_script(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    script: &Script,
    frame: &OoFrame,
    constants: &std::collections::HashMap<String, String>,
    depth: u32,
) {
    if super::super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) || !script.is_authored_source() {
        return;
    }
    for stmt in &script.statements {
        walk_oo_statement(ctx, cu, script, stmt, frame, constants, depth);
    }
}

/// Visit one statement's command words for the frame-constant fold, then
/// recurse into any nested body it carries.
fn walk_oo_statement(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    script: &Script,
    stmt: &Statement,
    frame: &OoFrame,
    constants: &std::collections::HashMap<String, String>,
    depth: u32,
) {
    match stmt {
        Statement::Call {
            tokens: Some(t), ..
        }
        | Statement::AssignValue {
            tokens: Some(t), ..
        } => {
            // O100 / O129-in-interpolation over the method-local constants
            // (empty unless the escaping model proved some name
            // method-local), then the frame-constant cmd-sub folds.
            visit_call_tokens(ctx, cu, t, constants);
            visit_oo_frame_folds(ctx, cu, t, frame, constants);
        }
        // `return [self class]` is the single most common shape of all, and a
        // Retain its actual Script-owned vector and result word when replacing
        // the value; the original head and unrelated argv stay untouched.
        Statement::Return {
            value: Some(raw),
            expr: None,
            ..
        } => try_oo_frame_return_fold(
            ctx,
            cu,
            stmt,
            raw,
            frame,
            script.retained_source_tokens_for_statement(stmt),
        ),
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_oo_script(ctx, cu, &c.body, frame, constants, depth + 1);
            }
            if let Some(b) = else_body {
                walk_oo_script(ctx, cu, b, frame, constants, depth + 1);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_oo_script(ctx, cu, init, frame, constants, depth + 1);
            walk_oo_script(ctx, cu, next, frame, constants, depth + 1);
            walk_oo_script(ctx, cu, body, frame, constants, depth + 1);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Block { body, .. } => {
            walk_oo_script(ctx, cu, body, frame, constants, depth + 1);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_oo_script(ctx, cu, body, frame, constants, depth + 1);
            for h in handlers {
                walk_oo_script(ctx, cu, &h.body, frame, constants, depth + 1);
            }
            if let Some(fb) = finally_body {
                walk_oo_script(ctx, cu, fb, frame, constants, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(body) = &a.body {
                    walk_oo_script(ctx, cu, body, frame, constants, depth + 1);
                }
            }
            if let Some(b) = default_body {
                walk_oo_script(ctx, cu, b, frame, constants, depth + 1);
            }
        }
        _ => {}
    }
}

/// Report an `O129` for each single-token `[cmd …]` word of this command that
/// the method frame folds to a constant.
fn visit_oo_frame_folds(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    tokens: &CommandTokens,
    frame: &OoFrame,
    _constants: &std::collections::HashMap<String, String>,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &cu.ir_module);
    let Some(calls) = retained_substitution_calls(tokens, &cu.ir_module, registry) else {
        return;
    };
    let mut rewrites: Vec<(tcl_lexer::Span, String)> = Vec::new();
    for word in tokens.words() {
        let tcl_source::words::WordExpr::CommandSubstitution { source, .. } = word else {
            continue;
        };
        let Some(call) = retained_call_at(&calls, source.span.start()) else {
            continue;
        };
        if let Some(value) = fold_retained_builtin(
            &cu.ir_module,
            registry,
            call,
            &calls,
            Some(&frame.defining_class),
            metadata,
            (ctx.dialect, None, module_script_encoding(cu)),
        ) && let Some(word) = render_propagation_word(&value, ctx.lexer_config())
        {
            rewrites.push((call.span, word));
        }
    }
    for (span, folded) in rewrites {
        ctx.report(Optimisation::new(
            DiagCode::O129,
            "Fold constant builtin command substitution",
            tcl_lexer::word_span_at(ctx.source, span),
            folded,
        ));
    }
}

/// `return [self class]` → `return ::TheClass`.
///
/// The original whole statement is the rewrite target. Its retained result
/// word selects the replacement site, preserving the actual written head.
fn try_oo_frame_return_fold(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    statement: &Statement,
    raw: &str,
    frame: &OoFrame,
    tokens: Option<&CommandTokens>,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let Some(tokens) = tokens else {
        return;
    };
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &cu.ir_module);
    let Some(calls) = retained_substitution_calls(tokens, &cu.ir_module, registry) else {
        return;
    };
    let mut roots = calls.iter().filter(|call| {
        ctx.source
            .get(call.span.start() as usize..call.span.end() as usize)
            == Some(raw.trim())
    });
    let Some(call) = roots.next() else {
        return;
    };
    if roots.next().is_some() {
        return;
    }
    let Some(value) = fold_retained_builtin(
        &cu.ir_module,
        registry,
        call,
        &calls,
        Some(&frame.defining_class),
        metadata,
        (ctx.dialect, None, module_script_encoding(cu)),
    ) else {
        return;
    };
    let Some((rewrite_span, replacement)) =
        original_return_value_replacement(ctx, statement, tokens, &value)
    else {
        return;
    };
    ctx.report(Optimisation::new(
        DiagCode::O129,
        "Fold constant builtin command substitution",
        rewrite_span,
        replacement,
    ));
}
