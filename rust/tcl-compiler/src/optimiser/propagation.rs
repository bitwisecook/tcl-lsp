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

//! Constant / copy propagation optimiser pass.
//!
//! Entry points:
//!
//! - **`optimise_constant_var_refs`** (`O100`) — replace a
//!   single-token `$var` call argument with its SCCP-proved
//!   literal, when the value is safe to inline as a bare word.
//! - **`optimise_static_proc_calls`** (`O103`) — fold calls to
//!   pure procs whose return is a proven constant
//!   (`can_fold_static_calls`). Fires
//!   applicable rewrites when the call appears as a `[proc …]`
//!   command substitution inside another call's argv (the argv
//!   span is the rewrite target); the bare statement form stays
//!   hint-only because the call result is discarded and folding
//!   `::answer` to `42` would leave an invalid command name.
//! - **`optimise_return_terminator`** (`O100`) — rewrite
//!   `return $v` as `return K` when `v` is SCCP-constant.
//!   (`O104` is reserved by the canonical optimisation-codes
//!   table for the string-build chain fold, so this path uses
//!   `O100`.)
//!
//! - **`optimise_string_interpolation_var_refs`** (`O100`) —
//!   inline SCCP-proved constants into `"…$x…"` double-quoted
//!   string arguments of calls (only when the interpolation is
//!   safe: the string contains no other substitutions, the
//!   constant value contains no Tcl metacharacters).
//! - **`optimise_load_forwarding`** (`O102`) — forward the
//!   single-reaching literal value of a variable at a use site,
//!   even when SCCP didn't fold it (e.g., when another path
//!   through the CFG makes the lattice Overdefined, but this
//!   particular use is dominated by one literal def). Fires
//!   applicable rewrites with argv-level spans when the use is
//!   a bare `$var` / `${var}` word in a `Statement::Call`; falls
//!   back to hint-only on statements without `CommandTokens` or
//!   on uses inside interpolated strings (the latter covered by
//!   the O100 string-interpolation path).
//!
//! Constant propagation into the condition sub-expressions of
//! `if` / `while` / `for` and the bodies of standalone `expr`
//! commands is handled elsewhere:
//! `branch_folding::propagate_into_branches` (for branch
//! conditions) and [`super::expr_simplify::run`] (for standalone
//! `expr` commands).

use crate::analyses::{ConstValue, LatticeValue};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{CommandTokens, Script, Statement};
use crate::tcl_expr_eval::FoldPolicy;
use tcl_core_types::DiagCode;

use super::helpers::expr_simplify::{NumericCtx, operand_types};
use super::helpers::literals::is_safe_word;
use super::{Optimisation, PassContext};
#[cfg(test)]
use tcl_syntax::naming::whole_scalar_variable_reference_name;

mod declared_body;

mod command_substitution_folds;
mod constant_substitution;
mod load_forwarding;
mod oo_method_folds;
mod proc_folds;
mod return_folds;
mod store_to_load_forwarding;

use command_substitution_folds::{fold_retained_builtin, visit_call_cmd_subst_folds};
pub(crate) use command_substitution_folds::{retained_call_at, retained_substitution_calls};
#[cfg(test)]
use constant_substitution::{substitute_dollar_refs, visit_string_interpolation};
use constant_substitution::{try_substitute_assign_expr, visit_call_tokens};
use load_forwarding::run_load_forwarding;
use oo_method_folds::run_oo_method_folds;
use proc_folds::try_fold_static_proc_call;
#[cfg(test)]
use proc_folds::try_o101_expr_arg_fold;
#[cfg(test)]
use proc_folds::{parse_cmd_subst_head, resolve_proc_qname, seed_params_from_args};
#[cfg(test)]
use return_folds::original_return_value_replacement;
#[cfg(test)]
use return_folds::original_return_word_replacement;
use return_folds::try_fold_return_terminator;
#[cfg(test)]
use store_to_load_forwarding::locate_use_var;
use store_to_load_forwarding::run_store_to_load_forwarding;

/// Run the propagation pass across every function.
///
/// Emits one `O100` per single-token `$var` argument that SCCP
/// proved to be a safe literal value. "Safe" means the value
/// parses as a decimal integer or as a bare identifier-shaped
/// word; string literals with Tcl metacharacters (`$`, `[`, `\`,
/// …) are skipped because substituting them as a bare word
/// would change the command's interpretation.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    run_oo_method_folds(ctx, cu);
    // The top-level body's extra escaping set (names some other procedure
    // declares `global`) — shared by the top-level constants projection
    // below and by `run_load_forwarding`.
    //
    // `scan_module_global_names` resolves the `global` alias grammar through
    // the registry, so it answers only for the dialect `cu` was lowered
    // under — and a *missing* name is the unsound direction here: it tells
    // the top level a name does not escape, licensing a fold of a value some
    // other procedure reassigns through `global`. So this deliberately has no
    // default-dialect fallback: with no registry the top-level body is left
    // un-propagated rather than propagated against a guess.
    //
    // Every production entry point does set `ctx.registry` —
    // `super::manager::build_pass_context` is the single chokepoint every
    // `optimise*` / `find_dead_stores` entry builds through — so this `None`
    // arm is reachable only from a hand-built `PassContext` in a pass-level
    // unit test, and a test that wants top-level coverage wires the registry
    // the way this pass's own `run_pass` helper does.
    let top_level_extra_escaping = ctx.registry.map(|registry| {
        crate::var_observability::scan_module_global_names(&cu.ir_module, registry)
    });
    let no_extra_escaping = std::collections::HashSet::new();
    if let Some(top_level_extra_escaping) = &top_level_extra_escaping {
        run_function(
            ctx,
            cu,
            &cu.top_level,
            &cu.ir_module.top_level,
            "::",
            top_level_extra_escaping,
        );
    }
    for (qname, proc) in cu.ir_module.original_source_procedure_bodies() {
        let Some(fu) = cu.function_for_body(Some(*qname)) else {
            continue;
        };
        // The call-site namespace for O103 chain resolution is the proc's
        // own namespace (`::ns::foo` resolves a bare `bar` against `::ns`).
        let Some(namespace) = proc
            .body
            .namespace_context
            .as_deref()
            .and_then(tcl_names::namespace::SourceNamespaceKey::advisory_key)
        else {
            continue;
        };

        run_function(ctx, cu, fu, &proc.body, &namespace, &no_extra_escaping);
    }
    declared_body::run(ctx, cu);
    // Load-forwarding runs a separate per-function pass on top
    // of the SCCP-based substitutions. It consults the def-use
    // chains directly and fires independently of the SCCP
    // lattice — a variable whose *sole* reaching def is a
    // literal Assign is forwarded even when other paths make
    // the lattice Overdefined. Being independent of the SCCP lattice, it
    // must apply the *same* externally-mutable guard SCCP applies itself
    // (`global`/`variable`/`upvar`/`trace` aliasing, plus — for the
    // top-level body specifically — any name some other procedure declares
    // via `global`; see `scan_module_global_names`, and, for every function,
    // a name traced by some *other* proc, via the registry-driven whole-
    // module `Module::traced_variables`/`has_dynamic_variable_trace` facts),
    // or it would forward a stale literal past a call that reassigns or
    // traces the "sole" def.
    if let (Some(registry), Some(top_level_extra_escaping)) =
        (ctx.registry, &top_level_extra_escaping)
    {
        let trace = crate::sccp::TraceInputs {
            registry,
            source_metadata_input:
                crate::registry_invocation::InvocationMetadataContext::for_module(
                    registry,
                    &cu.ir_module,
                )
                .and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                ),
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: &cu.ir_module.deferred_writes,
            analysis_context: None,
            existence: None,
        };
        run_load_forwarding(
            ctx,
            &cu.ir_module,
            &cu.top_level,
            top_level_extra_escaping,
            trace,
        );
        for fu in cu.procedures.values() {
            run_load_forwarding(ctx, &cu.ir_module, fu, &no_extra_escaping, trace);
        }
    }
    // O127 store-to-load forwarding for *computed* single-use
    // assignments (`set x [cmd]` inlined to its sole use site, then the
    // store deleted).  Skips the top-level body.
    for fu in cu.procedures.values() {
        run_store_to_load_forwarding(ctx, fu);
    }
}

fn run_function(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    fu: &FunctionUnit,
    script: &Script,
    namespace: &str,
    extra_escaping: &std::collections::HashSet<String>,
) {
    // A computed or opaque variable name can rewrite any local binding under
    // a spelling absent from SCCP's per-name lattice.  Do not let its raw
    // constants reach the text-level folds below: unlike
    // `run_load_forwarding`, this path would otherwise bypass the shared
    // dynamic-name guard and could emit (for example) `return 1` after a
    // depth-capped nested substitution may have run `set $name 2`.
    if fu.dynamic_barrier_blocks_value_motion() {
        // A local value-motion barrier withdraws lattice substitutions. Exact
        // original invocation receipts still independently authorise closed
        // constant calls that consume no such local facts.
        walk_script(
            ctx,
            cu,
            script,
            &std::collections::HashMap::new(),
            None,
            namespace,
            0,
        );
        return;
    }

    // Project the per-function SCCP lattice into a name → literal
    // map that survives only when every tracked version of the
    // variable collapses to the same single constant value. No
    // additional trace/alias filtering is needed here: `sccp()` itself
    // forces a traced or frame-aliased name's own lattice entry (and
    // anything derived from it) to `Overdefined`, so a projected literal
    // is already trace/alias-safe by construction — `run_load_forwarding`
    // (below) is the one exception that still needs its own check, since it
    // runs an independent def-use-chain scan that never consults `fu.sccp`.
    let constants = constants_with_builtin_folds(ctx, cu, fu, extra_escaping);

    let numeric = operand_types(fu);
    walk_script(ctx, cu, script, &constants, Some(&numeric), namespace, 0);
}

/// The function's projected constants map, widened with the registry
/// builtin-fold lattice.
///
/// The baseline is the shared [`FunctionUnit::sccp`] projection
/// ([`sccp_constants_for`]) — untouched, so every existing single-hop fold
/// stays byte-identical and the per-unit lattice memo (which cannot carry
/// the whole-module command-mutation fact in its key) is not disturbed.
/// When the body contains a command-substitution assignment, SCCP is
/// re-run with [`crate::sccp::BuiltinFoldInputs`] so chains like `set a
/// [string length abcdef]; set b [expr {$a + 1}]` close inside the
/// lattice; the re-run's projection is overlaid **additively**
/// (`entry().or_insert`) — it can only add names, never change or drop a
/// baseline constant. The re-run passes no interprocedural seeds (they
/// are not stored on the unit); a fold needing a seeded parameter is
/// simply missed, never wrong.
///
/// Cost containment: the re-run is skipped entirely (returning the
/// baseline) when the registry is absent, the unit is complexity-guarded,
/// or no `AssignValue` RHS is a whole-word `[…]` substitution — so a body
/// with nothing to fold pays one cheap scan and nothing else.
fn constants_with_builtin_folds(
    ctx: &PassContext<'_>,
    cu: &CompilationUnit,
    fu: &FunctionUnit,
    extra_escaping: &std::collections::HashSet<String>,
) -> std::collections::HashMap<String, String> {
    let mut constants = sccp_constants_for(fu, ctx.fold_policy());
    let Some(registry) = ctx.registry else {
        return constants;
    };
    if fu.complexity_guarded || !has_cmd_subst_assignment(fu) {
        return constants;
    }
    let rerun = crate::sccp::sccp_with_builtin_folds(
        &fu.cfg,
        &fu.ssa,
        None,
        FoldPolicy::from_registry(registry),
        extra_escaping,
        crate::sccp::TraceInputs {
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
        Some(crate::sccp::BuiltinFoldInputs {
            registry,
            source_metadata_input: fu
                .invocation_metadata_context_for_module(registry, &cu.ir_module)
                .and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                ),
            mutations: &ctx.command_mutations,
            dialect: ctx.dialect,
            // No method frame here — `[self class]`-style frame facts fold
            // only in `run_oo_method_folds`' proven method re-runs.
            defining_class: None,
            registry_engine: true,
            trust: crate::sccp::FoldTrust::WholeModule,
            proven_pure_parameters: false,
        }),
    );
    for (name, text) in sccp_constants_from(&rerun, &fu.ssa, ctx.fold_policy()) {
        constants.entry(name).or_insert(text);
    }
    constants
}

/// Whether any statement in the unit assigns a whole-word `[cmd …]`
/// command substitution — the only shape the registry builtin-fold
/// lattice can improve, and therefore the cost gate for re-running SCCP
/// in [`constants_with_builtin_folds`].
fn has_cmd_subst_assignment(fu: &FunctionUnit) -> bool {
    fu.cfg.blocks.values().any(|block| {
        block.statements.iter().any(|stmt| {
            matches!(stmt, Statement::AssignValue { value, .. } if {
                let trimmed = value.trim();
                trimmed.starts_with('[') && trimmed.ends_with(']')
            })
        })
    })
}

/// `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn walk_script(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    script: &Script,
    constants: &std::collections::HashMap<String, String>,
    numeric: NumericCtx<'_>,
    namespace: &str,
    depth: u32,
) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) || !script.is_authored_source() {
        return;
    }
    for stmt in &script.statements {
        walk_statement(
            ctx,
            cu,
            StatementSource {
                statement: stmt,
                tokens: script.retained_source_tokens_for_statement(stmt),
            },
            constants,
            numeric,
            namespace,
            depth,
        );
    }
}

#[derive(Clone, Copy)]
struct StatementSource<'a> {
    statement: &'a Statement,
    tokens: Option<&'a CommandTokens>,
}

fn walk_statement(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    source: StatementSource<'_>,
    constants: &std::collections::HashMap<String, String>,
    numeric: NumericCtx<'_>,
    namespace: &str,
    depth: u32,
) {
    match source.statement {
        Statement::Call {
            span,
            command,
            tokens,
            ..
        } => {
            if let Some(t) = tokens {
                visit_call_tokens(ctx, cu, t, constants);
                visit_call_cmd_subst_folds(ctx, cu, t, constants, namespace, *span);
            }
            try_fold_static_proc_call(ctx, cu, *span, command, tokens.as_ref(), namespace);
        }
        // `set TARGET [cmd-sub]` lowers to `AssignValue` carrying the
        // full command's tokens (`["set", TARGET, "[cmd-sub]"]`). Walk
        // its value words for the value-position cmd-sub folds — O115
        // redundant-nested-expr collapse and the O103 pure-proc
        // constant-return fold — so a `set` target gets the same folds a
        // command-argument position already gets. Only the cmd-sub fold
        // path is wired (not `visit_call_tokens`): a bare `set y $c` RHS
        // is handled by SCCP / load-forwarding, and folding it here would
        // change behaviour beyond the documented gap.
        //
        // Note: the *canonical* `set x [expr {[expr {E}]}]` does not reach
        // this arm — `extract_single_expr_arg` strips the outer `expr` at
        // lowering time, producing an `AssignExpr` whose `ExprCommand`
        // holds the inner `[expr {E}]`. So O115 fires here only for the
        // rarer AssignValue forms that retain a nested-expr cmd-sub word;
        // the O103 pure-proc fold is the common reachable case.
        Statement::AssignValue {
            span,
            tokens: Some(t),
            ..
        } => {
            visit_call_cmd_subst_folds(ctx, cu, t, constants, namespace, *span);
        }
        statement @ Statement::Return { .. } => {
            try_fold_return_terminator(ctx, statement, constants, &cu.ir_module, source.tokens);
        }
        Statement::AssignExpr { span, expr, .. } => {
            try_substitute_assign_expr(ctx, source, *span, expr, constants, numeric);
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(ctx, cu, &c.body, constants, numeric, namespace, depth + 1);
            }
            if let Some(b) = else_body {
                walk_script(ctx, cu, b, constants, numeric, namespace, depth + 1);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(ctx, cu, init, constants, numeric, namespace, depth + 1);
            walk_script(ctx, cu, next, constants, numeric, namespace, depth + 1);
            walk_script(ctx, cu, body, constants, numeric, namespace, depth + 1);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => {
            walk_script(ctx, cu, body, constants, numeric, namespace, depth + 1);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, cu, body, constants, numeric, namespace, depth + 1);
            for h in handlers {
                walk_script(ctx, cu, &h.body, constants, numeric, namespace, depth + 1);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, cu, fb, constants, numeric, namespace, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, cu, b, constants, numeric, namespace, depth + 1);
                }
            }
            if let Some(b) = default_body {
                walk_script(ctx, cu, b, constants, numeric, namespace, depth + 1);
            }
        }
        _ => {}
    }
}

/// Allow a literal to be inlined as a bare word if it is
/// decimal-integer-shaped or matches the safe-word grammar.
/// Rejects anything containing Tcl metacharacters that would
/// introduce new substitutions.
fn is_value_safe_bare_word(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.parse::<i64>().is_ok() {
        return true;
    }
    is_safe_word(value)
}

/// Construct one literal source word under the caller's complete configuration.
/// The shared Syntax owner verifies its decoded value and complete word shape;
/// failure supplies no replacement. Evaluation and erasure proofs stay separate.
fn render_propagation_word(value: &str, config: tcl_lexer::LexerConfig) -> Option<String> {
    tcl_syntax::word_rules::lexical_unicode_source_word(value, config)
}

/// The SCCP-proved constant for one exact SSA value, rendered as source text.
///
/// The per-value counterpart of [`sccp_constants_for`]'s name-keyed map: that
/// projection answers "is this *name* always this constant", which is the
/// question a by-name source substitution has to ask, and it necessarily
/// abstains for a name whose versions hold different constants. A def-use
/// consumer already knows which version it is forwarding, so it can ask the
/// stronger question and get an answer where the map has none.
fn sccp_value_literal(
    fu: &FunctionUnit,
    var_name: &tcl_names::variable::VariableCellKey,
    version: crate::ssa::Version,
    policy: FoldPolicy,
) -> Option<String> {
    use super::helpers::literals::format_constant_with_policy;

    let sym = fu.ssa.cell_symbol(var_name)?;
    if !fu.sccp.materialises((sym, version)) {
        return None;
    }
    match fu.sccp.values.get(&(sym, version))? {
        LatticeValue::Const(cv) => format_constant_with_policy(cv, policy),
        _ => None,
    }
}

pub(super) fn sccp_constants_for(
    fu: &FunctionUnit,
    policy: FoldPolicy,
) -> std::collections::HashMap<String, String> {
    sccp_constants_from(&fu.sccp, &fu.ssa, policy)
}

/// [`sccp_constants_for`]'s projection, over an explicitly-supplied lattice.
///
/// Split out so a caller that re-runs SCCP under a *wider* escaping set can
/// project the result the same way — see [`oo_method_constants`], which does
/// exactly that for a `TclOO` method body without disturbing the shared
/// [`FunctionUnit`] lattice other consumers read.
fn sccp_constants_from(
    sccp: &crate::sccp::SccpResult,
    ssa: &crate::ssa::SsaFunction,
    policy: FoldPolicy,
) -> std::collections::HashMap<String, String> {
    use super::helpers::literals::format_constant_with_policy;

    let mut per_var: std::collections::HashMap<crate::ssa::Symbol, Vec<&ConstValue>> =
        std::collections::HashMap::new();
    let mut dirty: std::collections::HashSet<crate::ssa::Symbol> = std::collections::HashSet::new();
    for (&(sym, ver), lv) in &sccp.values {
        if dirty.contains(&sym) {
            continue;
        }
        if let LatticeValue::Const(cv) = lv
            && sccp.materialises((sym, ver))
        {
            per_var.entry(sym).or_default().push(cv);
        } else {
            dirty.insert(sym);
            per_var.remove(&sym);
        }
    }
    let mut out = std::collections::HashMap::new();
    let dialect = policy.invocation_dialect.unwrap_or_else(|| {
        tcl_native_model::InvocationDialect::of_profile(
            policy
                .dialect
                .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl),
        )
    });
    for (sym, cvs) in per_var {
        if ssa
            .cell_key(sym)
            .is_some_and(|key| key.is_native_error_storage(dialect))
        {
            continue;
        }
        let first = cvs[0];
        if !cvs.iter().all(|cv| *cv == first) {
            continue;
        }
        let display = ssa.var_name(sym);
        if ssa.var_symbol(display) == Some(sym)
            && let Some(text) = format_constant_with_policy(first, policy)
        {
            out.insert(display.to_owned(), text);
        }
    }
    out
}

#[cfg(test)]
mod tests;
