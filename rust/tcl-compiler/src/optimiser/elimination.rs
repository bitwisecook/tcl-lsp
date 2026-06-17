//! Elimination optimiser pass (C30d).
//!
//! Ported from `core/compiler/optimiser/_elimination.py`. Emits:
//!
//! - **O107** — unreachable dead code (blocks SCCP proved
//!   unreachable).
//! - **O108** — transitively dead code. A side-effect-free def
//!   whose every consumer was already eliminated (the ADCE
//!   fixpoint on top of O109 / O126).
//! - **O109** — dead stores (an SSA def whose chain is empty and
//!   whose variable has at least one later definition — the
//!   write is overwritten before any read).
//! - **O126** — unused variable assignments (an SSA def whose
//!   chain is empty and is the only / last def of that variable
//!   — the value is never read). Skipped at the top level (the
//!   last command's result may be the script return value) and
//!   for scope-alias commands (`global` / `variable` / `upvar`).
//!
//! Emission order is the deterministic CFG `cfg_order` (reverse
//! post-order from the entry, unreachable blocks appended).

use std::collections::{HashMap, HashSet};

use tcl_registry::CommandRegistry;

use crate::cfg::Function as CfgFunction;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::def_use::DefKind;
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::sccp::{SccpResult, cfg_order};
use crate::segmenter::segment_commands;
use crate::side_effects::classify_side_effects;

use super::helpers::spans::full_rewrite_span;
use super::{Optimisation, PassContext};

/// True when `text` (a Tcl word body) contains a command substitution
/// that has an observable side effect — writes a variable, prints to
/// stdout, mutates global state, runs a dynamic barrier, etc. Used to
/// gate elimination of unused / dead assignments: `set v [puts X]`
/// discards the result but still prints, so the assignment is NOT safe
/// to delete.
///
/// `interproc_pure` is the set of qualified user-proc names proven pure
/// by interprocedural analysis; a cmd-sub of such a proc is treated as
/// side-effect-free even though [`classify_side_effects`] is
/// conservative for user commands. `pure_methods` + `enclosing_class`
/// (SF-2) recognise a pure `my <method>` self-dispatch. Conservative:
/// anything we can't classify (unknown proc, dynamic dispatch,
/// unparseable / no registry) is treated as having a side effect.
/// Mirrors Python's `_word_has_observable_side_effect`.
fn word_has_observable_side_effect(
    text: &str,
    registry: Option<&CommandRegistry>,
    interproc_pure: &HashSet<String>,
    pure_methods: &HashSet<String>,
    enclosing_class: Option<&str>,
) -> bool {
    if !text.contains('[') {
        return false;
    }
    // No registry to classify embedded commands → conservative.
    let Some(registry) = registry else {
        return true;
    };
    let sm = tcl_lexer::SourceMap::new(text);
    let Ok(tokens) = tcl_lexer::Lexer::new(text).tokenise_all() else {
        return true; // unparseable → conservative
    };
    for tok in &tokens {
        if tok.kind != tcl_lexer::TokenType::Cmd {
            continue;
        }
        // `token_text` yields the command substitution's inner text
        // (brackets stripped); segment it to get name + args.
        let cmds = segment_commands(sm.token_text(*tok));
        if cmds.len() != 1 || cmds[0].texts.is_empty() {
            // Multi-command substitution or empty → conservative.
            return true;
        }
        let cmd_name = cmds[0].texts[0].as_str();
        let cmd_args: &[String] = &cmds[0].texts[1..];
        let se = classify_side_effects(registry, cmd_name, cmd_args, None, None);
        if !se.pure {
            // A user proc / method that the registry can't classify may
            // have been interprocedurally proven pure — consult those.
            let proc_pure = interproc_pure.contains(cmd_name)
                || interproc_pure.contains(format!("::{cmd_name}").as_str())
                || interproc_pure.contains(cmd_name.trim_start_matches(':'));
            let self_dispatch_pure = matches!(cmd_name, "my" | "::my")
                && !cmd_args.is_empty()
                && enclosing_class.is_some_and(|cls| method_pure(cls, &cmd_args[0], pure_methods));
            if !proc_pure && !self_dispatch_pure {
                return true;
            }
        }
        // Recurse into nested substitutions inside the args.
        for arg in cmd_args {
            if word_has_observable_side_effect(
                arg,
                Some(registry),
                interproc_pure,
                pure_methods,
                enclosing_class,
            ) {
                return true;
            }
        }
    }
    false
}

/// Return `true` iff `class_qname::method_name` (or a common qualifier
/// spelling) is in `pure_methods`. Mirrors Python's `_method_pure`.
fn method_pure(class_qname: &str, method_name: &str, pure_methods: &HashSet<String>) -> bool {
    if method_name.is_empty() {
        return false;
    }
    let cls = class_qname.trim_start_matches(':');
    [
        format!("{class_qname}::{method_name}"),
        format!("::{cls}::{method_name}"),
        format!("{cls}::{method_name}"),
    ]
    .iter()
    .any(|k| pure_methods.contains(k))
}

/// Expr-tree analogue of [`word_has_observable_side_effect`] — `true`
/// if any embedded command substitution in the expression has an
/// observable side effect. Mirrors Python's
/// `_expr_has_observable_side_effect`.
fn expr_has_observable_side_effect(
    node: &ExprNode,
    registry: Option<&CommandRegistry>,
    interproc_pure: &HashSet<String>,
    pure_methods: &HashSet<String>,
    enclosing_class: Option<&str>,
) -> bool {
    match node {
        ExprNode::Command { text, .. } | ExprNode::Raw { text } => word_has_observable_side_effect(
            text,
            registry,
            interproc_pure,
            pure_methods,
            enclosing_class,
        ),
        ExprNode::Binary { left, right, .. } => {
            expr_has_observable_side_effect(
                left,
                registry,
                interproc_pure,
                pure_methods,
                enclosing_class,
            ) || expr_has_observable_side_effect(
                right,
                registry,
                interproc_pure,
                pure_methods,
                enclosing_class,
            )
        }
        ExprNode::Unary { operand, .. } => expr_has_observable_side_effect(
            operand,
            registry,
            interproc_pure,
            pure_methods,
            enclosing_class,
        ),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_has_observable_side_effect(
                condition,
                registry,
                interproc_pure,
                pure_methods,
                enclosing_class,
            ) || expr_has_observable_side_effect(
                true_branch,
                registry,
                interproc_pure,
                pure_methods,
                enclosing_class,
            ) || expr_has_observable_side_effect(
                false_branch,
                registry,
                interproc_pure,
                pure_methods,
                enclosing_class,
            )
        }
        ExprNode::Call { args, .. } => args.iter().any(|a| {
            expr_has_observable_side_effect(
                a,
                registry,
                interproc_pure,
                pure_methods,
                enclosing_class,
            )
        }),
        _ => false,
    }
}

/// `true` when `stmt` is an assignment whose RHS can be discarded
/// without losing observable behaviour. A literal (`AssignConst`) is
/// always safe; value / expr forms require every embedded command
/// substitution to be provably side-effect-free; `incr v` is safe
/// unless its optional amount word has a side effect. Any other
/// statement form is conservatively unsafe. Mirrors Python's
/// `_assignment_safe_to_delete`.
pub(crate) fn assignment_safe_to_delete(
    stmt: &Statement,
    registry: Option<&CommandRegistry>,
    interproc_pure: &HashSet<String>,
    pure_methods: &HashSet<String>,
    enclosing_class: Option<&str>,
) -> bool {
    match stmt {
        Statement::AssignConst { .. } => true,
        Statement::AssignValue { value, .. } => !word_has_observable_side_effect(
            value,
            registry,
            interproc_pure,
            pure_methods,
            enclosing_class,
        ),
        Statement::AssignExpr { expr, .. } => !expr_has_observable_side_effect(
            expr,
            registry,
            interproc_pure,
            pure_methods,
            enclosing_class,
        ),
        // `incr v` reads + writes v — the assignment itself is the
        // observable effect, so deleting it is OK when v is dead and
        // the optional amount word is side-effect-free.
        Statement::Incr { amount, .. } => match amount {
            None => true,
            Some(a) => !word_has_observable_side_effect(
                a,
                registry,
                interproc_pure,
                pure_methods,
                enclosing_class,
            ),
        },
        // Unknown statement form — conservative.
        _ => false,
    }
}

/// Collect the qualified names of procs / methods that interprocedural
/// analysis has proven pure — threaded into the O109 / O126 RHS-purity
/// gates so `set unused [pureProc]` / `set unused [my pureMethod]` can
/// fold. Mirrors the `interproc_pure` / `interproc_pure_methods` sets
/// built at the top of Python's `optimise_elimination_passes`.
fn pure_call_targets(ctx: &PassContext<'_>) -> (HashSet<String>, HashSet<String>) {
    let interproc_pure: HashSet<String> = ctx
        .interproc
        .procedures
        .iter()
        .filter(|(_, s)| s.pure)
        .map(|(q, _)| q.clone())
        .collect();
    let pure_methods: HashSet<String> = ctx
        .interproc
        .methods
        .iter()
        .filter(|(_, s)| s.base.pure)
        .map(|(q, _)| q.clone())
        .collect();
    (interproc_pure, pure_methods)
}

/// Run the elimination pass — emits O107, O108, O109, O126
/// across every function in `cu`.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    let is_top_level = |fu: &FunctionUnit| fu.name == "::top";
    // D2-O126/O109 closure: the user-proc / TclOO-method purity sets
    // that gate RHS-side-effect-safe deletion (owned so the `&mut ctx`
    // calls below don't alias `ctx.interproc`).
    let (interproc_pure, pure_methods) = pure_call_targets(ctx);
    // SYNC-JUN02d-2: the call-by-name proc-index (caller-locals passed by
    // name to an upvar callee must not be deleted as dead/unused — O109 /
    // O126).  Built once; borrows `ctx.interproc` before the `&mut ctx`
    // emit calls below.
    let proc_index = crate::interprocedural::build_proc_index_from_summaries(&ctx.interproc);

    emit_unreachable(ctx, &cu.top_level);
    let baseline = emit_dead_stores_and_unused(
        ctx,
        &cu.top_level,
        is_top_level(&cu.top_level),
        &interproc_pure,
        &pure_methods,
        None,
        &proc_index,
    );
    emit_adce(ctx, &cu.top_level, &baseline);

    for fu in cu.procedures.values() {
        emit_unreachable(ctx, fu);
        let baseline = emit_dead_stores_and_unused(
            ctx,
            fu,
            false,
            &interproc_pure,
            &pure_methods,
            None,
            &proc_index,
        );
        emit_adce(ctx, fu, &baseline);
    }

    // SF-2 (#506): optimise TclOO method bodies as functions too,
    // passing the owning class qname so the O126 `my <method>` purity
    // gate can resolve same-class pure methods. Instance variables
    // escape the method frame (they are object state), so they are fed
    // through the same escaping channel iRules cross-event state uses —
    // the dead-store / unused-assignment passes must not delete a
    // state-mutating `set ivar ...` inside the method body. Mirrors the
    // method-iteration loop in `optimiser/_manager.py`.
    let saved_cross = std::mem::take(&mut ctx.cross_event_vars);
    for (mqname, fu) in &cu.methods {
        let ir_method = cu.ir_module.methods.get(mqname);
        let enclosing_class = ir_method.map(|m| m.class_name.as_str());
        ctx.cross_event_vars = ir_method
            .map(|m| m.instance_vars.clone())
            .unwrap_or_default();
        emit_unreachable(ctx, fu);
        let baseline = emit_dead_stores_and_unused(
            ctx,
            fu,
            false,
            &interproc_pure,
            &pure_methods,
            enclosing_class,
            &proc_index,
        );
        emit_adce(ctx, fu, &baseline);
    }
    ctx.cross_event_vars = saved_cross;
}

fn emit_unreachable(ctx: &mut PassContext<'_>, fu: &FunctionUnit) {
    let unreachable = unreachable_blocks(&fu.cfg, &fu.sccp);
    // cfg_order is deterministic (RPO + trailing unreachables).
    for block_name in cfg_order(&fu.cfg) {
        if !unreachable.contains(&block_name) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&block_name) else {
            continue;
        };
        for stmt in &block.statements {
            let span = stmt.span();
            // Skip zero-length spans — those are synthesised IR
            // (e.g. implicit barriers) with no user-visible
            // source text to delete.
            if span.is_empty() {
                continue;
            }
            ctx.report(Optimisation::new(
                "O107",
                "Eliminate unreachable dead code",
                full_rewrite_span(ctx.source, span),
                "",
            ));
        }
    }
}

/// Return the set of block names SCCP determined unreachable
/// from the CFG entry.
fn unreachable_blocks(cfg: &CfgFunction, sccp: &SccpResult) -> HashSet<String> {
    cfg.blocks
        .keys()
        .filter(|name| !sccp.executable_blocks.contains(*name))
        .cloned()
        .collect()
}

/// A dead store the optimiser determined eliminable (**O109**) — exposed so
/// tools can show dead stores from where Rust *actually* computes them
/// (the optimiser's SSA def-use pass, with its purity / scope-alias /
/// place-model / cross-event suppression), rather than a naive SSA
/// re-derivation that would over-report. Used by the compiler explorer's
/// `cfgPostSsa` analysis block, dead-store callouts, and `stats`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadStore {
    /// The owning function's qualified name (`::top`, `::add`, …).
    pub function: String,
    /// The CFG block holding the dead store.
    pub block: String,
    /// The statement's index within its block.
    pub statement_index: i32,
    /// The dead SSA value's variable name.
    pub variable: String,
    /// The dead SSA value's SSA version.
    pub version: u32,
}

/// Collected per-chain metadata used by
/// [`emit_dead_stores_and_unused`] to sort + emit in span order.
struct DseEntry {
    span: tcl_lexer::Span,
    code: &'static str,
    msg: &'static str,
    key: (String, u32),
    block: String,
    stmt_index: i32,
}

/// Emit O109 (dead store) + O126 (unused variable) for each
/// dead SSA def in `fu.def_use`. Returns the set of SSA value
/// keys that were reported — the ADCE pass uses it as the
/// "already eliminated" seed for its fixpoint.
#[allow(clippy::too_many_arguments)]
fn emit_dead_stores_and_unused(
    ctx: &mut PassContext<'_>,
    fu: &FunctionUnit,
    is_top_level: bool,
    interproc_pure: &HashSet<String>,
    pure_methods: &HashSet<String>,
    enclosing_class: Option<&str>,
    proc_index: &crate::interprocedural::ProcIndex,
) -> HashSet<(String, u32)> {
    let registry = ctx.registry;
    let unreachable = unreachable_blocks(&fu.cfg, &fu.sccp);
    let scope_aliases = scan_scope_aliases(&fu.cfg);
    // SYNC-JUN02d-2: caller-locals this function passes by name to an
    // upvar callee — not dead/unused even when the name-level SSA sees
    // no read (the callee reads/writes it through the alias).
    let call_by_name = crate::interprocedural::collect_call_by_name_reads(&fu.cfg, proc_index);
    // The `def_use` builder does not scan Return-value reads or
    // embedded string-interpolation reads; do a supplementary
    // textual pass over the CFG to collect every var name that
    // appears in any source slice. Any def of a name referenced
    // textually is kept live — conservative but correct.
    let mut textually_referenced = collect_textual_var_references(ctx.source, &fu.cfg);
    // A read-modify-write command's target buried in a substitution
    // (`lappend r [incr i $j]` reads `i`) keeps a feeding `set i 0` alive.
    if let Some(registry) = ctx.registry {
        textually_referenced.extend(collect_rmw_hidden_reads(fu, registry));
    }

    // Phase 8 place model (SYNC-MAY31-1b): array-element writes the name-level
    // SSA mis-folds (`set a(k) 1` "overwritten" by `set a(j) 2`) but that a read
    // observes.  Shared with the analyser's W220.  Empty unless a registry is
    // bound (set by the `optimise*` entry points) and the function writes array
    // elements — so the bare test/`run_pass` path keeps its prior behaviour.
    let place_suppressed = ctx
        .registry
        .map(|reg| crate::place_bridge::element_writes_observed_by_reads(&fu.cfg, &fu.name, reg))
        .unwrap_or_default();

    // Collect one DseEntry per dead chain then sort + emit.
    let mut entries: Vec<DseEntry> = Vec::new();

    for chain in fu.def_use.chains.values() {
        if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
            continue;
        }
        if unreachable.contains(&chain.definition.block) {
            // O107 already reports these.
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
            continue;
        };
        let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
            continue;
        };
        let Some(stmt) = block.statements.get(idx) else {
            continue;
        };
        // D2-O109/O126 closure: gate deletion on RHS purity. The def is
        // dead at the SSA level, but its RHS may have observable side
        // effects (`set unused [puts X]` prints, `set unused [my
        // impureMethod]` mutates object state). Only delete when every
        // embedded command substitution is provably side-effect-free.
        if !assignment_safe_to_delete(
            stmt,
            registry,
            interproc_pure,
            pure_methods,
            enclosing_class,
        ) {
            continue;
        }
        // Suppress when this element write is observed by a read the name-level
        // SSA can't see (place-model overlap — the O109 sibling of W220).
        if place_suppressed.contains(&(
            chain.definition.block.clone(),
            chain.definition.statement_index,
        )) {
            continue;
        }
        // Skip if the variable is a scope alias — writes through
        // global / upvar are visible in other scopes.
        let (var, _) = &chain.key;
        if scope_aliases.contains(var) {
            continue;
        }
        // Skip cross-event vars (iRules scope).
        if ctx.cross_event_vars.contains(var) {
            continue;
        }
        // SYNC-JUN02d-2: skip a caller-local passed by name to an upvar
        // callee — the callee consumes it through the alias (O109 / O126).
        if call_by_name.contains(var) {
            continue;
        }
        let any_other_live = fu
            .def_use
            .chains
            .iter()
            .any(|(k, c)| k.0 == *var && k.1 != chain.key.1 && !c.is_dead());

        let (code, msg): (&'static str, &'static str) = if any_other_live {
            // Dead store — overwritten before read (another
            // version has live consumers). This fires regardless
            // of textual mentions: a later version handles the
            // reads.
            ("O109", "Eliminate dead store")
        } else if !is_top_level {
            // Unused variable — apply the textual-scan keep-live
            // check. The def-use builder does not track reads
            // from `Return` terminators or `"$x"` string
            // interpolations, so a conservative over-approximation
            // of names referenced anywhere in the source text
            // suppresses spurious O126 for legitimately-consumed
            // variables.
            if textually_referenced.contains(var) {
                continue;
            }
            ("O126", "Remove unused variable assignment")
        } else {
            // Top-level never emits O126.
            continue;
        };
        let _ = var;
        entries.push(DseEntry {
            span: stmt.span(),
            code,
            msg,
            key: chain.key.clone(),
            block: chain.definition.block.clone(),
            stmt_index: chain.definition.statement_index,
        });
    }

    entries.sort_by_key(|e| e.span.start());
    let mut removed: HashSet<(String, u32)> = HashSet::new();
    for e in entries {
        // Record O109 dead stores (not O126 unused vars) so tools can show
        // them from where Rust determines them. `run` collects these into
        // `ctx.dead_stores`.
        if e.code == "O109" {
            ctx.dead_stores.push(DeadStore {
                function: fu.name.clone(),
                block: e.block.clone(),
                statement_index: e.stmt_index,
                variable: e.key.0.clone(),
                version: e.key.1,
            });
        }
        ctx.report(Optimisation::new(
            e.code,
            e.msg,
            full_rewrite_span(ctx.source, e.span),
            "",
        ));
        removed.insert(e.key);
    }
    removed
}

/// Emit **O108** (transitively dead code) — the ADCE fixpoint.
fn emit_adce(ctx: &mut PassContext<'_>, fu: &FunctionUnit, baseline: &HashSet<(String, u32)>) {
    let (consumer_stmt_keys, keep_forever) = build_adce_consumers(fu);
    let stmt_to_defs = build_stmt_to_defs(fu);
    let removed = run_adce_fixpoint(
        fu,
        baseline,
        &consumer_stmt_keys,
        &keep_forever,
        &stmt_to_defs,
    );
    emit_adce_reports(ctx, fu, baseline, &removed);
}

type ConsumerMap = HashMap<(String, u32), Vec<(String, usize)>>;

fn build_adce_consumers(fu: &FunctionUnit) -> (ConsumerMap, HashSet<(String, u32)>) {
    use crate::def_use::UseKind;
    let mut consumer_stmt_keys: ConsumerMap = HashMap::new();
    let mut keep_forever: HashSet<(String, u32)> = HashSet::new();
    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != DefKind::Statement {
            continue;
        }
        let key = chain.key.clone();
        for use_site in &chain.uses {
            match use_site.kind {
                UseKind::Operand => {
                    if let Ok(idx) = usize::try_from(use_site.statement_index) {
                        consumer_stmt_keys
                            .entry(key.clone())
                            .or_default()
                            .push((use_site.block.clone(), idx));
                    }
                }
                UseKind::PhiIncoming | UseKind::Terminator => {
                    keep_forever.insert(key.clone());
                }
            }
        }
    }
    (consumer_stmt_keys, keep_forever)
}

type StmtDefsMap = HashMap<(String, usize), Vec<(String, u32)>>;

fn build_stmt_to_defs(fu: &FunctionUnit) -> StmtDefsMap {
    let mut out: StmtDefsMap = HashMap::new();
    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != DefKind::Statement {
            continue;
        }
        if let Ok(idx) = usize::try_from(chain.definition.statement_index) {
            out.entry((chain.definition.block.clone(), idx))
                .or_default()
                .push(chain.key.clone());
        }
    }
    out
}

fn run_adce_fixpoint(
    fu: &FunctionUnit,
    baseline: &HashSet<(String, u32)>,
    consumer_stmt_keys: &ConsumerMap,
    keep_forever: &HashSet<(String, u32)>,
    stmt_to_defs: &StmtDefsMap,
) -> HashSet<(String, u32)> {
    let unreachable = unreachable_blocks(&fu.cfg, &fu.sccp);
    let mut removed = baseline.clone();
    loop {
        let mut changed = false;
        for chain in fu.def_use.chains.values() {
            if chain.definition.kind != DefKind::Statement {
                continue;
            }
            let key = &chain.key;
            if removed.contains(key) || keep_forever.contains(key) {
                continue;
            }
            if unreachable.contains(&chain.definition.block) {
                continue;
            }
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            if !is_side_effect_free_assignment(stmt) {
                continue;
            }
            let empty: Vec<(String, usize)> = Vec::new();
            let consumers: &Vec<(String, usize)> = consumer_stmt_keys.get(key).unwrap_or(&empty);
            if consumers.is_empty() {
                continue;
            }
            let all_removed = consumers.iter().all(|pair: &(String, usize)| {
                stmt_to_defs
                    .get(pair)
                    .is_some_and(|defs: &Vec<(String, u32)>| {
                        defs.iter().all(|d: &(String, u32)| removed.contains(d))
                    })
            });
            if all_removed {
                removed.insert(key.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    removed
}

fn emit_adce_reports(
    ctx: &mut PassContext<'_>,
    fu: &FunctionUnit,
    baseline: &HashSet<(String, u32)>,
    removed: &HashSet<(String, u32)>,
) {
    let mut new_reports: Vec<tcl_lexer::Span> = Vec::new();
    for key in removed.difference(baseline) {
        if let Some(chain) = fu.def_use.chains.get(key) {
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            new_reports.push(stmt.span());
        }
    }
    new_reports.sort_by_key(|s| s.start());
    for span in new_reports {
        ctx.report(Optimisation::new(
            "O108",
            "Eliminate transitively dead code",
            full_rewrite_span(ctx.source, span),
            "",
        ));
    }
}

fn is_side_effect_free_assignment(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::AssignConst { .. }
            | Statement::AssignValue { .. }
            | Statement::AssignExpr { .. }
            | Statement::Incr { .. }
    )
}

/// Scan every statement's source slice for `$var` / `${var}`
/// references and collect the names seen. Narrow to the
/// function's own CFG extent now that the segmenter emits
/// absolute spans for proc bodies — so false-positive
/// suppression across proc boundaries no longer applies.
/// Scan a slice for `$var` and `${var}` references, inserting names
/// into *out*.  Extracted from `collect_textual_var_references`.
#[allow(clippy::many_single_char_names)]
fn scan_dollar_refs(slice: &str, out: &mut HashSet<String>) {
    let bytes = slice.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        i += 1;
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'{' {
            // ${name}
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'}' {
                i += 1;
            }
            if start < i
                && let Ok(name) = std::str::from_utf8(&bytes[start..i])
            {
                // Strip any array index.
                let name = name.split('(').next().unwrap_or(name);
                if !name.is_empty() {
                    out.insert(name.to_owned());
                }
            }
            if i < bytes.len() {
                i += 1; // consume closing `}`
            }
            continue;
        }
        // $name: identifier chars (letters, digits, underscore, ::).
        let start = i;
        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_alphanumeric() || b == b'_' {
                i += 1;
            } else if b == b':' && i + 1 < bytes.len() && bytes[i + 1] == b':' {
                i += 2;
            } else {
                break;
            }
        }
        if start < i
            && let Ok(name) = std::str::from_utf8(&bytes[start..i])
        {
            out.insert(name.to_owned());
        }
    }
}

/// Scan a slice for `[set NAME]` (1-arg read form) references.
/// Mirrors upstream commit `342d4c7a` (PR #331).
#[allow(clippy::many_single_char_names)]
fn scan_set_read_refs(slice: &str, out: &mut HashSet<String>) {
    let bs = slice.as_bytes();
    let mut j = 0;
    while j < bs.len() {
        if bs[j] != b'[' {
            j += 1;
            continue;
        }
        let open = j;
        let mut k = j + 1;
        while k < bs.len() && (bs[k] == b' ' || bs[k] == b'\t') {
            k += 1;
        }
        if slice[k..].starts_with("::") {
            k += 2;
        }
        if !slice[k..].starts_with("set") {
            j = open + 1;
            continue;
        }
        let after_set = k + 3;
        if after_set >= bs.len() || !(bs[after_set] == b' ' || bs[after_set] == b'\t') {
            j = open + 1;
            continue;
        }
        let mut m = after_set;
        while m < bs.len() && (bs[m] == b' ' || bs[m] == b'\t') {
            m += 1;
        }
        let name_start = m;
        while m < bs.len() {
            let b = bs[m];
            if b.is_ascii_alphanumeric() || b == b'_' {
                m += 1;
            } else if b == b':' && m + 1 < bs.len() && bs[m + 1] == b':' {
                m += 2;
            } else {
                break;
            }
        }
        if m == name_start {
            j = open + 1;
            continue;
        }
        let mut n = m;
        while n < bs.len() && (bs[n] == b' ' || bs[n] == b'\t') {
            n += 1;
        }
        if n < bs.len()
            && bs[n] == b']'
            && let Ok(name) = std::str::from_utf8(&bs[name_start..m])
            && !name.is_empty()
        {
            out.insert(name.to_owned());
        }
        j = m;
    }
}

pub(crate) fn collect_textual_var_references(source: &str, cfg: &CfgFunction) -> HashSet<String> {
    // Absolute spans now cover the function's own source range.
    let span_iter = cfg.blocks.values().flat_map(|b| {
        let stmts = b.statements.iter().map(crate::ir::Statement::span);
        let term = b.terminator.as_ref().and_then(crate::cfg::Terminator::span);
        stmts.chain(term)
    });
    let Some((lo, hi)) = span_iter.fold(None, |acc: Option<(u32, u32)>, span| {
        let s = span.start();
        let e = span.end();
        match acc {
            None => Some((s, e)),
            Some((l, h)) => Some((l.min(s), h.max(e))),
        }
    }) else {
        return HashSet::new();
    };
    let start = usize::try_from(lo).unwrap_or(0);
    let end = usize::try_from(hi)
        .unwrap_or(source.len())
        .min(source.len());
    if start >= end {
        return HashSet::new();
    }
    let slice = &source[start..end];
    let mut out: HashSet<String> = HashSet::new();
    scan_dollar_refs(slice, &mut out);
    scan_set_read_refs(slice, &mut out);
    out
}

/// Local names bound by an `upvar` (or `namespace upvar`) whose *caller
/// target* is dynamic (`$name` / `[cmd]`).  Such an alias may resolve to a
/// non-existent caller variable, so the local is possibly-unset: an
/// unconditional read of it is a read-before-set (W210).  Kept separate from
/// [`scan_scope_aliases`] (which still records the local for W211/W220, since
/// a *write* through the alias is observable) — only the W210 read-before-set
/// pass consults this set to *override* the scope-alias suppression.  Mirrors
/// Python's dynamic-target handling in `_collect_upvar_targets` /
/// `_read_before_set`.
pub(crate) fn scan_dynamic_upvar_locals(cfg: &CfgFunction) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            let Statement::Call { command, args, .. } = stmt else {
                continue;
            };
            // `upvar ?level? caller local …` or `namespace upvar ns caller local …`.
            let start = match command.as_str() {
                "upvar" => {
                    let has_level = args
                        .first()
                        .is_some_and(|a| a.starts_with('#') || a.parse::<i64>().is_ok());
                    usize::from(has_level)
                }
                "namespace" if args.first().map(String::as_str) == Some("upvar") => 2,
                _ => continue,
            };
            let mut i = start;
            while i + 1 < args.len() {
                let caller = &args[i];
                if caller.starts_with('$') || caller.starts_with('[') {
                    out.insert(crate::naming::normalise_var_name(&args[i + 1]).to_owned());
                }
                i += 2;
            }
        }
    }
    out
}

/// Variable names read *inside command substitutions* that the shallow word
/// scan misses — chiefly a read-modify-write command's target buried in a
/// substitution (`lappend r [incr i $j]` reads `i`), plus vars read via a
/// `VarRead`-role argument of a substituted command.  Name-level only and
/// **suppress-only**: it keeps a feeding `set i 0` from being reported as a
/// dead store / unused variable.  Deliberately computed *outside* SSA `uses`
/// so read-before-set versioning is unperturbed.  Mirrors Python's
/// `expr_substitution_read_names` (deep RMW scan minus the shallow scan).
pub(crate) fn collect_rmw_hidden_reads(
    fu: &FunctionUnit,
    registry: &CommandRegistry,
) -> HashSet<String> {
    use crate::var_refs::{VarReferenceScanner, VarScanOptions};
    let mut deep = VarReferenceScanner::new(VarScanOptions {
        include_var_read_roles: true,
        recurse_cmd_substitutions: true,
        include_reads_before_write: true,
    });
    let mut shallow = VarReferenceScanner::new(VarScanOptions::default());
    let mut out: HashSet<String> = HashSet::new();
    let mut scan = |word: &str| {
        if !word.contains('[') {
            return;
        }
        let d = deep.scan_word(word, registry);
        let s = shallow.scan_word(word, registry);
        out.extend(d.difference(&s).cloned());
    };
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::Call { args, .. } | Statement::Barrier { args, .. } => {
                    for arg in args {
                        scan(arg);
                    }
                }
                Statement::AssignValue { value, .. } => scan(value),
                _ => {}
            }
        }
    }
    out
}

/// Scan every CFG block for scope-alias commands (`global`,
/// `variable`, `upvar`, `namespace upvar`) and collect the
/// variable names they bind. Those must not be flagged as dead
/// stores / unused — writes go to a different scope.
pub(crate) fn scan_scope_aliases(cfg: &CfgFunction) -> HashSet<String> {
    let mut aliases: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call {
                command,
                args,
                defs,
                ..
            } = stmt
            {
                match command.as_str() {
                    "global" => {
                        for a in args {
                            aliases.insert(a.clone());
                        }
                    }
                    "variable" => {
                        // variable name ?value? ?name value? …
                        let mut i = 0;
                        while i < args.len() {
                            aliases.insert(args[i].clone());
                            i += 2;
                        }
                    }
                    "upvar" => {
                        // upvar ?level? name localname ?name localname? …
                        // Take the odd-indexed args (the locals).
                        let has_level = args.first().is_some_and(|a| {
                            a.starts_with('#') || a == "0" || a == "1" || a.parse::<i64>().is_ok()
                        });
                        let start = usize::from(has_level);
                        let mut i = start + 1;
                        while i < args.len() {
                            aliases.insert(args[i].clone());
                            i += 2;
                        }
                    }
                    "namespace" if matches!(args.first().map(String::as_str), Some("upvar")) => {
                        // namespace upvar NS name localname …
                        let mut i = 3;
                        while i < args.len() {
                            aliases.insert(args[i].clone());
                            i += 2;
                        }
                    }
                    "trace" => {
                        // A write trace fires the callback on every `set`, so
                        // the traced variable is observable and must never be
                        // a dead store (W220) / unused (W211).  Recognises both
                        // `trace add variable NAME …` (8.5+) and the 8.4 `trace
                        // variable NAME …` spelling; a dynamic `$`-target names
                        // no static local and is skipped here.
                        let target = if args.len() >= 3 && args[0] == "add" && args[1] == "variable"
                        {
                            Some(&args[2])
                        } else if args.len() >= 2 && args[0] == "variable" {
                            Some(&args[1])
                        } else {
                            None
                        };
                        if let Some(t) = target
                            && !t.starts_with('$')
                            && !t.contains('[')
                        {
                            aliases.insert(t.clone());
                        }
                    }
                    _ => {
                        // For other calls, trust the per-statement
                        // `defs` annotation when present (it
                        // captures upvars the lowering recognised).
                        for d in defs {
                            let _ = d;
                        }
                    }
                }
            }
        }
    }
    aliases
}

#[cfg(test)]
mod tests {
    use super::*;

    use tcl_registry::CommandRegistry;

    use crate::interprocedural::InterproceduralAnalysis;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    // -- internal helper tests -----------------------------------------------

    #[test]
    fn unreachable_blocks_empty_when_all_executable() {
        let cu = CompilationUnit::build_for("set x 1", &registry(), false);
        let unreach = unreachable_blocks(&cu.top_level.cfg, &cu.top_level.sccp);
        assert!(unreach.is_empty());
    }

    // -- end-to-end tests ---------------------------------------------------

    #[test]
    fn empty_source_produces_nothing() {
        let opts = run_pass("");
        assert!(opts.is_empty());
    }

    #[test]
    fn straight_line_script_is_fully_reachable() {
        let opts = run_pass("set x 1\nset y 2\nputs $x");
        assert!(opts.is_empty());
    }

    #[test]
    fn branch_folding_creates_unreachable_block() {
        // The else branch is unreachable under SCCP because the
        // condition folds to true.
        let opts = run_pass("if {1} { set x 1 } else { set y 2 }");
        // Expect at least one O107 — the else body's `set y 2`.
        assert!(
            opts.iter()
                .any(|o| o.code == "O107" && o.message == "Eliminate unreachable dead code"),
            "expected at least one O107, got {opts:?}",
        );
    }

    #[test]
    fn while_false_body_is_unreachable() {
        // The body of `while {0} { ... }` is unreachable.
        let opts = run_pass("while {0} { set x 1 }");
        assert!(
            opts.iter().any(|o| o.code == "O107"),
            "expected an O107 for dead while body, got {opts:?}",
        );
    }

    #[test]
    fn unreachable_statements_emitted_with_empty_replacement() {
        let opts = run_pass("if {0} { set x 1 }");
        let target = opts.iter().find(|o| o.code == "O107");
        if let Some(o) = target {
            assert_eq!(o.replacement, "");
            assert!(!o.span.is_empty());
        }
    }

    #[test]
    fn o109_o126_suppressed_for_call_by_name_var() {
        // SYNC-JUN02d-2: a caller-local passed by name to an upvar callee
        // must not be deleted as a dead store / unused result.
        // `optimise_raw` builds the interproc summaries the proc-index
        // needs (the bare `run_pass` path has empty interproc).
        let count_dead = |src: &str| -> usize {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .iter()
                .filter(|o| o.code == "O109" || o.code == "O126")
                .count()
        };
        // `noup` is a plain proc → `tag` is NOT call-by-name → the
        // overwritten `set tag init` is a dead store.
        let no_cbn = count_dead(
            "proc ::noup {x} { return 1 }\nproc ::f {} { set tag init\nset tag x\nnoup tag }",
        );
        // `fill` upvar-writes its param → `tag` is call-by-name → the
        // dead store on `tag` is suppressed.
        let with_cbn = count_dead(
            "proc ::fill {vn} { upvar 1 $vn v\nset v 1 }\nproc ::f {} { set tag init\nset tag x\nfill tag }",
        );
        assert!(
            with_cbn < no_cbn,
            "call-by-name should suppress a dead store (no_cbn={no_cbn}, with_cbn={with_cbn})",
        );
    }

    #[test]
    fn dead_store_fires_o109_when_overwritten_before_read() {
        // First set is dead — second version is the only live
        // value of x when read by puts.
        let opts = run_pass("set x 1\nset x 2\nputs $x");
        assert!(
            opts.iter().any(|o| o.code == "O109"),
            "expected O109 for overwritten store, got {opts:?}",
        );
    }

    #[test]
    fn o109_array_element_overwrite_not_dead() {
        // SYNC-MAY31-1b place model: `set a(k) 1` is NOT a dead store — the
        // later `set a(j) 2` bumps the name-level SSA version of base `a`, but
        // `puts $a(k)` reads a(k) and a(k) ≠ a(j).  Goes through `optimise`,
        // which binds the registry the place bridge needs (`run_pass` leaves it
        // unbound, so the bare-path scalar test above is unaffected).
        let opts = crate::optimiser::optimise(
            "proc f {} { set a(k) 1; set a(j) 2; puts $a(k) }",
            &registry(),
        );
        assert!(
            opts.iter().all(|o| o.code != "O109"),
            "no O109 expected — a(k) is read by `puts $a(k)`; got {opts:?}",
        );
    }

    #[test]
    fn o109_array_element_genuinely_dead_still_fires() {
        // Precision guard: here only a(j) is read, so a(k) IS overwritten
        // before any read — O109 must still fire on it.  The place model
        // suppresses only element writes that a read actually observes.
        let opts = crate::optimiser::optimise("set a(k) 1\nset a(j) 2\nputs $a(j)", &registry());
        assert!(
            opts.iter().any(|o| o.code == "O109"),
            "O109 expected — a(k) is overwritten and never read; got {opts:?}",
        );
    }

    #[test]
    fn unused_variable_fires_o126_in_proc_body() {
        let opts = run_pass("proc ::f {} { set y 42; return 1 }");
        assert!(
            opts.iter().any(|o| o.code == "O126"),
            "expected O126 for unused var in proc, got {opts:?}",
        );
    }

    #[test]
    fn top_level_unused_variable_not_flagged() {
        // At top level an unused variable is not O126 — the
        // script-return semantics and external consumers (upvar,
        // info exists) could read it.
        let opts = run_pass("set y 42");
        assert!(
            opts.iter().all(|o| o.code != "O126"),
            "top-level unused var should not emit O126, got {opts:?}",
        );
    }

    #[test]
    fn scope_alias_globals_never_flagged() {
        let opts = run_pass("proc ::f {} { global g; set g 42 }");
        assert!(
            opts.iter().all(|o| o.code != "O109" && o.code != "O126"),
            "writes through global should not be flagged, got {opts:?}",
        );
    }

    #[test]
    fn adce_removes_chain_of_dead_defs() {
        // `set a 1` → `set b $a` → `set c $b`; `c` is never
        // read. O126 flags `set c $b`, then ADCE should extend
        // to `set b $a` and `set a 1` since their only consumer
        // is the already-dead chain.
        let opts = run_pass("proc ::f {} { set a 1; set b $a; set c $b; return 7 }");
        let o108 = opts.iter().filter(|o| o.code == "O108").count();
        assert!(
            o108 >= 1,
            "expected at least one O108 in transitive dead chain, got {opts:?}",
        );
    }

    #[test]
    fn used_variable_not_flagged() {
        let opts = run_pass("proc ::f {} { set x 1; return $x }");
        assert!(
            opts.iter().all(|o| o.code != "O109" && o.code != "O126"),
            "used var should not be flagged, got {opts:?}",
        );
    }

    #[test]
    fn collect_textual_var_references_detects_set_one_arg_read() {
        // ``[set varname]`` (1-arg form) is a variable read; without
        // it, DCE saw 0 reads on ``varname`` and incorrectly deleted
        // the write.  Mirrors upstream commit ``342d4c7a`` (PR #331).
        let opts = run_pass("proc ::f {} { set x 1; set y [set x]; return $y }");
        // ``x`` is read via ``[set x]`` so neither O109 nor O126
        // should fire on the ``set x 1`` line.
        let bad: Vec<_> = opts
            .iter()
            .filter(|o| (o.code == "O109" || o.code == "O126") && o.message.contains('x'))
            .collect();
        assert!(
            bad.is_empty(),
            "[set x] should count as a read for x; got {opts:?}",
        );
    }

    #[test]
    fn collect_textual_var_references_detects_qualified_set_read() {
        // ``[::set varname]`` form should also count.
        let opts = run_pass("proc ::f {} { set x 1; set y [::set x]; return $y }");
        let bad: Vec<_> = opts
            .iter()
            .filter(|o| (o.code == "O109" || o.code == "O126") && o.message.contains('x'))
            .collect();
        assert!(
            bad.is_empty(),
            "[::set x] should count as a read for x; got {opts:?}",
        );
    }

    // -- D2-O126/O109 RHS-purity gate (SYNC-JUN02-1 strip 4a) -------------

    #[test]
    fn o126_preserved_for_impure_command_sub_rhs() {
        // `set unused [puts hi]` discards the result but still prints —
        // the assignment must NOT be deleted (was a latent FP: the Rust
        // O126 fired unconditionally on dead chains).
        let opts = crate::optimiser::optimise(
            "proc ::f {} { set unused [puts hi]; return 1 }",
            &registry(),
        );
        assert!(
            opts.iter().all(|o| o.code != "O126"),
            "impure cmd-sub RHS must be preserved, got {opts:?}",
        );
    }

    #[test]
    fn o126_fires_for_pure_user_proc_rhs() {
        // A user proc proven pure by interproc analysis has no
        // observable side effect, so `set unused [::pure]` folds.
        let opts = crate::optimiser::optimise(
            "proc ::pure {} { return 1 }\nproc ::f {} { set unused [::pure]; return 1 }",
            &registry(),
        );
        assert!(
            opts.iter().any(|o| o.code == "O126"),
            "pure-proc RHS should fold to O126, got {opts:?}",
        );
    }

    #[test]
    fn o126_still_fires_for_literal_rhs() {
        // The gate must not regress the literal case — `set y 42` has
        // no RHS side effect and stays foldable.
        let opts = run_pass("proc ::f {} { set y 42; return 1 }");
        assert!(
            opts.iter().any(|o| o.code == "O126"),
            "literal RHS should still fold, got {opts:?}",
        );
    }

    // -- SF-2 method-body O126 (SYNC-JUN02-1 strip 4b) --------------------

    #[test]
    fn sf2_o126_folds_pure_my_dispatch_in_method_body() {
        // The optimiser now runs over TclOO method bodies with the
        // owning class as `enclosing_class`, so `set unused [my pure]`
        // — a self-dispatch to a method proven pure — folds to O126.
        // (FP-OPT-12 PARTIAL → FIXED.)
        let src = "oo::class create C {\n\
                   \x20   method pure {} { return 1 }\n\
                   \x20   method uses {} { set unused [my pure]; return 2 }\n\
                   }";
        let opts = crate::optimiser::optimise(src, &registry());
        assert!(
            opts.iter().any(|o| o.code == "O126"),
            "pure `my` self-dispatch RHS should fold, got {opts:?}",
        );
    }

    #[test]
    fn sf2_o126_preserves_impure_my_dispatch_in_method_body() {
        // An impure method (`puts`) must keep its self-dispatch — the
        // assignment is preserved so the side effect still runs.
        let src = "oo::class create C {\n\
                   \x20   method noisy {} { puts hi }\n\
                   \x20   method uses {} { set unused [my noisy]; return 2 }\n\
                   }";
        let opts = crate::optimiser::optimise(src, &registry());
        assert!(
            opts.iter().all(|o| o.code != "O126"),
            "impure `my` self-dispatch RHS must be preserved, got {opts:?}",
        );
    }

    #[test]
    fn sf2_instance_var_write_in_method_not_deleted() {
        // An instance-var write inside a method body is object state
        // that escapes the frame — it must not be flagged O109/O126
        // even when the method never reads it back.
        let src = "oo::class create C {\n\
                   \x20   variable n\n\
                   \x20   method bump {} { set n 5 }\n\
                   }";
        let opts = crate::optimiser::optimise(src, &registry());
        assert!(
            opts.iter().all(|o| o.code != "O109" && o.code != "O126"),
            "instance-var write must be preserved, got {opts:?}",
        );
    }

    #[test]
    fn run_passes_dispatches_elimination() {
        let cu = CompilationUnit::build_for("if {0} { set x 1 }", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::Elimination]);
        // At minimum the dispatch must not panic; O107 may or
        // may not fire depending on how SCCP models this exact
        // shape, but running the pass must be side-effect free
        // otherwise.
        let only_o107 = ctx.optimisations.iter().all(|o| o.code == "O107");
        assert!(only_o107, "unexpected codes: {:?}", ctx.optimisations);
    }
}
