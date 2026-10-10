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

//! Elimination optimiser pass.
//!
//! Emits:
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

mod source_purity;
mod source_reads;

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;
use tcl_lexer::TokenType;

use tcl_registry::CommandRegistry;

use crate::cfg::Function as CfgFunction;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::def_use::DefKind;
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::sccp::{SccpResult, cfg_order};

use super::helpers::spans::full_rewrite_span;
use super::{Optimisation, PassContext};

/// Actual source input for the substitution effect gate. Interprocedural
/// names assist only a Logical target joined to its original declaration.
#[derive(Clone, Copy)]
pub(crate) struct PurityCtx<'a> {
    pub(crate) registry: Option<&'a CommandRegistry>,
    pub(crate) interproc_pure: &'a HashSet<String>,
    pub(crate) enclosing_class: Option<&'a str>,
    /// The document's lexer configuration.  The purity walk re-lexes an
    /// already-extracted word only to recognise command substitution syntax;
    /// original child receipts supply dispatch and arguments. It reads under
    /// the grammar the document was lexed with — an iRules `}{`, a Jim
    /// `$(…)` or an 8.4 `{*}` puts the head in a different place.
    pub(crate) config: tcl_lexer::LexerConfig,
    pub(crate) metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    pub(crate) module: Option<&'a crate::ir::Module>,
}

#[derive(Clone, Copy)]
struct EffectCtx<'a> {
    purity: PurityCtx<'a>,
    source_tokens: Option<&'a crate::ir::CommandTokens>,
}

fn word_has_observable_side_effect(text: &str, effect: EffectCtx<'_>) -> bool {
    if !text.contains('[') {
        return false;
    }
    let Ok(tokens) = tcl_lexer::Lexer::with_config(text, effect.purity.config)
        .as_quoted_body()
        .tokenise_all()
    else {
        return true;
    };
    if !tokens.iter().any(|token| token.kind == TokenType::Cmd) {
        return false;
    }
    !source_purity::substitutions_are_pure(effect)
}

/// Expr-tree analogue of [`word_has_observable_side_effect`] — `true`
/// if any embedded command substitution in the expression has an
/// observable side effect.
fn expr_has_observable_side_effect(node: &ExprNode, effect: EffectCtx<'_>, depth: u32) -> bool {
    // Native-stack safety net: walks the `ExprNode` tree, one
    // native frame per level. Past the cap, assume an observable side effect
    // — the conservative direction, so an expression assignment nested deeper
    // than we can walk is never wrongly deleted. (Calls into
    // `word_has_observable_side_effect` only recognises a leaf's selected
    // lexical substitution syntax; it does not recursively parse commands.)
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return true;
    }
    match node {
        // Command nodes retain the inner script, without bracket delimiters.
        // The AST itself proves evaluation syntax; dispatch still requires
        // the original nested receipts rather than re-lexing that script.
        ExprNode::Command { .. } => !source_purity::substitutions_are_pure(effect),
        ExprNode::Raw { text } => word_has_observable_side_effect(text, effect),
        ExprNode::Binary { left, right, .. } => {
            expr_has_observable_side_effect(left, effect, depth + 1)
                || expr_has_observable_side_effect(right, effect, depth + 1)
        }
        ExprNode::Unary { operand, .. } => {
            expr_has_observable_side_effect(operand, effect, depth + 1)
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_has_observable_side_effect(condition, effect, depth + 1)
                || expr_has_observable_side_effect(true_branch, effect, depth + 1)
                || expr_has_observable_side_effect(false_branch, effect, depth + 1)
        }
        ExprNode::Call { args, .. } => args
            .iter()
            .any(|a| expr_has_observable_side_effect(a, effect, depth + 1)),
        // A `"…"` operand substitutes, so its `[cmd]` runs: O126 deleted
        // `set y [expr {"[incr x]"}]` and lost the `incr` (#2227, found in
        // review).
        ExprNode::String { text, .. } => crate::word_subst::quoted_operand_body(text)
            .is_some_and(|body| word_has_observable_side_effect(body, effect)),
        _ => false,
    }
}

/// `true` when `stmt` is an assignment whose RHS can be discarded
/// without losing observable behaviour. A literal (`AssignConst`) is
/// always safe; value / expr forms require every embedded command
/// substitution to be provably side-effect-free; `incr v` is safe
/// unless its optional amount word has a side effect. Any other
/// statement form is conservatively unsafe.
pub(crate) fn assignment_safe_to_delete_at(
    function: &FunctionUnit,
    block: crate::cfg::BlockId,
    index: usize,
    purity: PurityCtx<'_>,
) -> bool {
    let Some(statement) = function
        .cfg
        .blocks
        .get(&block)
        .and_then(|block| block.statements.get(index))
    else {
        return false;
    };
    assignment_safe_to_delete_with_effect(
        statement,
        EffectCtx {
            purity,
            source_tokens: function.cfg.source_tokens_at(block, index),
        },
    )
}

fn assignment_safe_to_delete_with_effect(stmt: &Statement, effect: EffectCtx<'_>) -> bool {
    match stmt {
        Statement::AssignConst { .. } => true,
        Statement::AssignValue { value, .. } => !word_has_observable_side_effect(value, effect),
        Statement::AssignExpr { expr, .. } => !expr_has_observable_side_effect(expr, effect, 0),
        // `incr v` reads + writes v — the assignment itself is the
        // observable effect, so deleting it is OK when v is dead and
        // the optional amount word is side-effect-free.
        Statement::Incr { amount, .. } => match amount {
            None => true,
            Some(a) => !word_has_observable_side_effect(a, effect),
        },
        // Unknown statement form — conservative.
        _ => false,
    }
}

/// Whether evaluating a dead assignment's value provably cannot raise, so
/// deleting the statement cannot remove an error the program would stop on.
///
/// Reading an unset variable, an array as a scalar, or a variable a read trace
/// guards raises; so does `expr` arithmetic on a bad operand (`1/0`, `abc + 1`).
/// O109, O126 and O108 deleted those statements and let the program run on
/// (#2249). The proof:
///
/// * a literal (`AssignConst`) cannot raise;
/// * a value SCCP folds to a constant evaluated cleanly: SCCP declines on an
///   evaluation error and reads an undefined, traced or escaping name as
///   overdefined, so a `Const` for the def is a clean evaluation;
/// * otherwise a word value (`AssignValue`) qualifies when every variable it
///   reads is definitely set ([`Self::definitely_set`]). An `expr` or `incr`
///   value needs the SCCP proof, since its operators can raise on a defined
///   operand.
struct RaiseProof<'a> {
    fu: &'a FunctionUnit,
    /// The procedure's parameters: bound on entry, so a version-0 read of one
    /// is set.
    params: Vec<String>,
    /// At top level, the dialect whose interpreter binds its startup scalars
    /// (`argv`, `tcl_version`) before user code; `None` in a procedure.
    startup: Option<tcl_dialect::model::SurfaceQuery<'a>>,
    /// Names a scope alias binds, which another frame may unset.
    scope_aliases: HashSet<String>,
    /// Names a module-wide trace guards; a read trace may raise.
    module_traced: Option<&'a std::collections::BTreeSet<String>>,
    /// Alias/effect availability belongs to this exact supplied source input.
    metadata_available: bool,
    /// Ordinary source SSA names can assist only the retained Logical model.
    logical_source_assistance: bool,
    /// Every SSA value's definition site.
    sites: HashMap<crate::ssa::ValueKey, DefSite>,
}

enum DefSite {
    /// A φ: set when every incoming value is set.
    Phi(Vec<crate::ssa::Version>),
    /// A statement: `true` when it certainly writes the variable once it
    /// completes — an assignment or `incr` of the variable itself, or a
    /// variable target of a command the registry marks as always writing it
    /// (`catch`, `gets`, `lassign`, `regsub`). Never a synthetic array
    /// may-def, nor a command (`regexp`, `scan`, `unset`, a `foreach` header)
    /// that may leave it unset.
    Stmt(bool),
    /// A conditional writer's target (`regexp`, `scan`, `binary scan`), or a
    /// read-modify-write target (`lset`, `lpop`, `ledit`): set exactly when
    /// the version it keeps or reads was.
    Carry(crate::ssa::Version),
}

impl<'a> RaiseProof<'a> {
    /// `enclosing_class` is set only for a `TclOO` method unit, and
    /// `top_level` only for the module's own top-level unit (a procedure may
    /// share its `::top` name).
    fn new(
        ctx: &PassContext<'a>,
        fu: &'a FunctionUnit,
        enclosing_class: Option<&str>,
        top_level: bool,
    ) -> Self {
        // Alias recognition is registry-driven; a registry-less context (unit
        // tests) falls back to the cached default.
        let registry = ctx.registry.unwrap_or_else(|| {
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands()
        });
        let metadata = fu.invocation_metadata_context(registry);
        let logical_source_assistance = metadata.is_some()
            && fu.source_metadata_input().is_some_and(
                crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
            );
        // A proc or a method binds its parameters on entry. A proc and a
        // method may share a qualified name, so the unit's kind picks the map;
        // the top level has none, even beside a procedure named `::top`.
        let params = ctx
            .ir_module
            .filter(|_| !top_level)
            .and_then(|m| {
                if enclosing_class.is_some() {
                    m.methods.get(&fu.name).map(|d| &d.params)
                } else {
                    m.procedures.get(&fu.name).map(|p| &p.params)
                }
            })
            .cloned()
            .unwrap_or_default();
        let mut sites = HashMap::new();
        for block in fu.ssa.blocks.values() {
            for phi in &block.phis {
                sites.insert(
                    (phi.name, phi.version),
                    DefSite::Phi(phi.incoming.values().copied().collect()),
                );
            }
            for ssa_stmt in &block.statements {
                let writes = matches!(
                    ssa_stmt.statement,
                    Statement::AssignConst { .. }
                        | Statement::AssignValue { .. }
                        | Statement::AssignExpr { .. }
                        | Statement::Incr { .. }
                );
                let targets = command_write_targets(&ssa_stmt.statement, registry, metadata);
                for (&sym, &ver) in &ssa_stmt.defs {
                    let name = fu.ssa.var_name(sym);
                    let site = if ssa_stmt.may_defs.contains(&sym) {
                        DefSite::Stmt(false)
                    } else if writes || targets.always.contains(&name) {
                        DefSite::Stmt(true)
                    } else if targets.maybe.contains(&name) {
                        // The SSA records the kept value as the statement's
                        // own read of the target (#2051).
                        ssa_stmt
                            .uses
                            .get(&sym)
                            .map_or(DefSite::Stmt(false), |&prev| DefSite::Carry(prev))
                    } else {
                        DefSite::Stmt(false)
                    };
                    sites.insert((sym, ver), site);
                }
            }
        }
        Self {
            fu,
            params,
            startup: (top_level && logical_source_assistance)
                .then_some(metadata)
                .flatten()
                .map(|metadata| metadata.context().authoring_query()),
            metadata_available: metadata.is_some(),
            logical_source_assistance,
            scope_aliases: scan_scope_aliases_with_metadata_context(&fu.cfg, registry, metadata),
            module_traced: ctx.ir_module.map(|m| &m.traced_variables),
            sites,
        }
    }

    /// Whether a trace, a scope alias or an alias-observed link may unset or
    /// guard `name` where this function cannot see it. Element names check
    /// their base too.
    fn observed(&self, name: &str) -> bool {
        let base = tcl_syntax::naming::split_element_ref(name).map_or(name, |(base, _)| base);
        self.scope_aliases.contains(name)
            || self.scope_aliases.contains(base)
            || self.fu.cfg.alias_observed_vars.contains(name)
            || self.fu.cfg.alias_observed_vars.contains(base)
            || self
                .module_traced
                .is_some_and(|t| t.contains(base.trim_start_matches("::")))
    }

    /// Whether the statement at `idx` of `block`, whose def is `def`, can be
    /// deleted without losing an error its value would raise.
    fn value_cannot_raise(
        &self,
        block: crate::cfg::BlockId,
        idx: usize,
        stmt: &Statement,
        def: &(crate::var_resolve::VariableCellKey, u32),
    ) -> bool {
        if !self.metadata_available {
            return false;
        }
        let folded = || {
            self.fu.ssa.cell_symbol(&def.0).is_some_and(|sym| {
                matches!(
                    self.fu.sccp.values.get(&(sym, def.1)),
                    Some(
                        crate::analyses::LatticeValue::Const(_)
                            | crate::analyses::LatticeValue::ConstSet(_)
                    )
                )
            })
        };
        match stmt {
            Statement::AssignConst { .. } => true,
            // An element read (`$a(k)`, `$a($i)`) raises when its base is a
            // scalar or lacks the element, which the reads' definedness
            // cannot show.
            Statement::AssignValue { value, .. } => {
                folded()
                    || stmt.tokens().is_some_and(|tokens| {
                        tokens.source_binding.as_ref().is_some_and(|binding| {
                            binding.original_arguments_complete_normally(tokens)
                        })
                    })
                    || (self.logical_source_assistance
                        && !has_element_substitution(value)
                        && self.reads_are_set(block, idx))
            }
            _ => folded(),
        }
    }

    /// Whether every variable the statement substitutes is definitely set.
    fn reads_are_set(&self, block: crate::cfg::BlockId, idx: usize) -> bool {
        let Some(ssa_stmt) = self
            .fu
            .ssa
            .blocks
            .get(&block)
            .and_then(|b| b.statements.get(idx))
        else {
            return false;
        };
        ssa_stmt
            .uses
            .iter()
            .filter(|(sym, _)| !ssa_stmt.quoted_uses.contains(sym))
            .all(|(&sym, &ver)| {
                let name = self.fu.ssa.var_name(sym);
                !self.observed(name) && self.definitely_set(sym, ver, &mut HashSet::new())
            })
    }

    /// Whether `(sym, ver)` is set on every path that reaches it: a parameter
    /// on entry, or a value every definition of which certainly writes it.
    /// A φ cycle adds no unset input of its own, so a revisited value counts
    /// as set; any unset input reaches the cycle through another φ operand.
    fn definitely_set(
        &self,
        sym: crate::ssa::Symbol,
        ver: crate::ssa::Version,
        visiting: &mut HashSet<crate::ssa::ValueKey>,
    ) -> bool {
        if ver == 0 {
            let name = self.fu.ssa.var_name(sym);
            return self.params.iter().any(|p| p == name)
                || self.startup.is_some_and(|dialect| {
                    let name = name.strip_prefix("::").unwrap_or(name);
                    tcl_registry::special_vars::special_var(name).is_some_and(|v| {
                        v.kind == tcl_registry::special_vars::SpecialVarKind::Scalar
                    }) && tcl_registry::special_vars::is_initially_bound(name, Some(dialect))
                });
        }
        if !visiting.insert((sym, ver)) {
            return true;
        }
        match self.sites.get(&(sym, ver)) {
            Some(DefSite::Stmt(writes)) => *writes,
            Some(DefSite::Carry(prev)) => self.definitely_set(sym, *prev, visiting),
            Some(DefSite::Phi(incoming)) => incoming
                .iter()
                .all(|&v| self.definitely_set(sym, v, visiting)),
            None => false,
        }
    }
}

/// Whether a word substitutes an array element: an unescaped `$name(`, or
/// `$(` for the empty-named array. A braced `${a(k)}` names a scalar and is
/// not one; a false positive only keeps a store.
fn has_element_substitution(word: &str) -> bool {
    let bytes = word.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'$' if bytes.get(i + 1) != Some(&b'{') => {
                let mut j = i + 1;
                while j < bytes.len()
                    && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b':')
                {
                    j += 1;
                }
                if bytes.get(j) == Some(&b'(') {
                    return true;
                }
                i = j.max(i + 1);
            }
            _ => i += 1,
        }
    }
    false
}

/// The variable targets of a command statement, split by how the registry
/// says the invocation writes them. The targets are the call's `defs` that
/// sit at the invocation's own `VarWrite` positions (see
/// [`command_write_targets`]); a def taken from a script argument
/// (`catch {set a 1} x` defining `a`) or a nested substitution is in neither.
#[derive(Default)]
struct CommandWriteTargets<'s> {
    /// Written whenever the command completes (`UNCONDITIONAL_VARIABLE_WRITE`).
    always: Vec<&'s str>,
    /// Written only on a runtime match, else left as they were
    /// (`CONDITIONAL_VARIABLE_WRITE`), or read before the write
    /// (`READS_BEFORE_WRITE`: `lset`, `lpop`, `ledit`), so set after the
    /// command exactly when they were set before it.
    maybe: Vec<&'s str>,
}

fn command_write_targets<'s>(
    stmt: &'s Statement,
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> CommandWriteTargets<'s> {
    let Statement::Call { defs, .. } = stmt else {
        return CommandWriteTargets::default();
    };
    let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
        return CommandWriteTargets::default();
    };
    let Some(invocation) =
        crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
            registry,
            Some(context),
            stmt,
        )
    else {
        return CommandWriteTargets::default();
    };
    let traits = invocation.facts.traits;
    let always = traits.contains(tcl_registry::Traits::UNCONDITIONAL_VARIABLE_WRITE);
    let maybe = traits.contains(tcl_registry::Traits::CONDITIONAL_VARIABLE_WRITE)
        || (traits.contains(tcl_registry::Traits::READS_BEFORE_WRITE)
            && !traits.contains(tcl_registry::Traits::DESTROYS_VARIABLE));
    if !always && !maybe {
        return CommandWriteTargets::default();
    }
    let targets: Vec<String> = invocation
        .facts
        .arg_roles
        .iter()
        .filter(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
        .filter_map(|(index, _)| {
            invocation
                .evaluated_arguments
                .get(invocation.facts.argument_offset + usize::from(*index))
                .cloned()
                .flatten()
        })
        .collect();
    let names: Vec<&str> = defs
        .iter()
        .map(String::as_str)
        .filter(|name| targets.iter().any(|target| target == name))
        .collect();
    if always {
        CommandWriteTargets {
            always: names,
            maybe: Vec::new(),
        }
    } else {
        CommandWriteTargets {
            always: Vec::new(),
            maybe: names,
        }
    }
}

/// Collect the qualified names of procs / methods that interprocedural
/// analysis has proven pure — threaded into the O109 / O126 RHS-purity
/// gates so `set unused [pureProc]` / `set unused [my pureMethod]` can
/// fold.
fn pure_call_targets(ctx: &PassContext<'_>) -> HashSet<String> {
    ctx.interproc
        .procedures
        .iter()
        .filter(|(_, summary)| summary.pure)
        .map(|(name, _)| name.clone())
        .collect()
}

/// Run the elimination pass — emits O107, O108, O109, O126
/// across every function in `cu`.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    let is_top_level = |fu: &FunctionUnit| fu.name == "::top";
    // O109/O126 closure: the user-proc / TclOO-method purity sets
    // that gate RHS-side-effect-safe deletion (owned so the `&mut ctx`
    // calls below don't alias `ctx.interproc`).
    let interproc_pure = pure_call_targets(ctx);
    if deep_analysis_available(&cu.top_level) {
        emit_unreachable(ctx, &cu.top_level);
        let purity = PurityCtx {
            registry: ctx.registry,
            interproc_pure: &interproc_pure,
            enclosing_class: None,
            config: cu.top_level.source_lexer_config(),
            metadata: ctx.registry.and_then(|registry| {
                cu.top_level
                    .invocation_metadata_context_for_module(registry, &cu.ir_module)
            }),
            module: Some(&cu.ir_module),
        };
        let baseline =
            emit_dead_stores_and_unused(ctx, &cu.top_level, is_top_level(&cu.top_level), purity);
        emit_adce(ctx, &cu.top_level, &baseline, purity, true);
    }

    // `manager::build_pass_context` populates this shared safety fact once,
    // before every pass runs. Retain the event-only projection here so plain
    // procedures with an equal local spelling are not needlessly suppressed.
    let saved_proc_cross = std::mem::take(&mut ctx.cross_event_vars);
    for (qname, fu) in &cu.procedures {
        if !deep_analysis_available(fu) {
            continue;
        }
        ctx.cross_event_vars = if qname.starts_with("::when::") {
            cu.connection_scope
                .as_ref()
                .map(|scope| scope.handler_source_names(qname, false))
                .unwrap_or_default()
        } else {
            std::collections::HashSet::new()
        };
        emit_unreachable(ctx, fu);
        let purity = PurityCtx {
            registry: ctx.registry,
            interproc_pure: &interproc_pure,
            enclosing_class: None,
            config: fu.source_lexer_config(),
            metadata: ctx.registry.and_then(|registry| {
                fu.invocation_metadata_context_for_module(registry, &cu.ir_module)
            }),
            module: Some(&cu.ir_module),
        };
        let baseline = emit_dead_stores_and_unused(ctx, fu, false, purity);
        emit_adce(ctx, fu, &baseline, purity, false);
    }
    ctx.cross_event_vars = saved_proc_cross;

    // Method functions retain the same original nested dispatch requirements.
    // The owning class selects parameter metadata, not receiver-method purity.
    // Instance variables
    // escape the method frame (they are object state), so they are fed
    // through the same escaping channel iRules cross-event state uses —
    // the dead-store / unused-assignment passes must not delete a
    // state-mutating `set ivar ...` inside the method body.
    let saved_cross = std::mem::take(&mut ctx.cross_event_vars);
    for (mqname, fu) in &cu.methods {
        if !deep_analysis_available(fu) {
            continue;
        }
        let ir_method = cu.ir_module.methods.get(mqname);
        let enclosing_class = ir_method.map(|m| m.class_name.as_str());
        ctx.cross_event_vars = ir_method
            .map(|m| m.instance_vars.clone())
            .unwrap_or_default();
        emit_unreachable(ctx, fu);
        let purity = PurityCtx {
            registry: ctx.registry,
            interproc_pure: &interproc_pure,
            enclosing_class,
            config: fu.source_lexer_config(),
            metadata: ctx.registry.and_then(|registry| {
                fu.invocation_metadata_context_for_module(registry, &cu.ir_module)
            }),
            module: Some(&cu.ir_module),
        };
        let baseline = emit_dead_stores_and_unused(ctx, fu, false, purity);
        emit_adce(ctx, fu, &baseline, purity, false);
    }
    ctx.cross_event_vars = saved_cross;
}

/// Whether `fu`'s lattices are real facts this pass may read.
///
/// A unit the complexity guard excludes from deep analysis
/// ([`FunctionUnit::trivial_guarded`]) carries trivial SSA, empty def-use
/// chains, and a `SccpResult::default()` whose executable-block set is empty
/// because nothing computed one — an absence of facts, not the fact that SCCP
/// proved every block dead. O107 is the one report in this pass that reads a
/// *missing* block as a positive fact, so it must not run over such a unit:
/// every statement in the body would answer "unreachable" and the whole body
/// would be offered for deletion.
///
/// Guarded units are common rather than exotic. `TclOO` method bodies reach
/// the guard through `ir_helpers::requires_runtime_command_namespace`: a
/// method runs in the receiver's namespace, chosen at run time, which can
/// shadow any relative command name, so one unqualified head anywhere in the
/// body (`return [self class]`, `puts hi`) is enough.
///
/// The remaining reports (O108 ADCE, O109 dead stores, O126 unused
/// assignments) walk def-use chains and so find nothing in a guarded unit
/// either way. They are skipped alongside O107 so the whole pass honours one
/// rule rather than three: a guarded unit's lattices are never read as facts.
fn deep_analysis_available(fu: &FunctionUnit) -> bool {
    !fu.complexity_guarded
}

fn emit_unreachable(ctx: &mut PassContext<'_>, fu: &FunctionUnit) {
    let unreachable = unreachable_blocks(&fu.cfg, &fu.sccp);
    // cfg_order is deterministic (RPO + trailing unreachables).
    for block_id in cfg_order(&fu.cfg) {
        if !unreachable.contains(&block_id) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&block_id) else {
            continue;
        };
        for (index, _statement) in block.statements.iter().enumerate() {
            let Some(command_span) = fu.cfg.statement_source_edit_span(block_id, index) else {
                continue;
            };
            // CFG statement spans are relative to the unit's `base_offset`;
            // absolutise before slicing `ctx.source` / emitting.
            let span = fu.abs_span(command_span);
            // Skip zero-length spans — those are synthesised IR
            // (e.g. implicit barriers) with no user-visible
            // source text to delete.
            if span.is_empty() {
                continue;
            }
            ctx.report(Optimisation::new(
                DiagCode::O107,
                "Eliminate unreachable dead code",
                full_rewrite_span(ctx.source, span),
                "",
            ));
        }
    }
}

/// Return the set of block ids SCCP determined unreachable
/// from the CFG entry.
fn unreachable_blocks(cfg: &CfgFunction, sccp: &SccpResult) -> HashSet<crate::cfg::BlockId> {
    cfg.blocks
        .keys()
        .filter(|id| !sccp.executable_blocks.contains(*id))
        .copied()
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
    code: DiagCode,
    msg: &'static str,
    key: (crate::var_resolve::VariableCellKey, u32),
    block: String,
    statement_index: i32,
}

/// Emit O109 (dead store) + O126 (unused variable) for each
/// dead SSA def in `fu.def_use`. Returns the set of SSA value
/// keys that were reported — the ADCE pass uses it as the
/// "already eliminated" seed for its fixpoint.
fn emit_dead_stores_and_unused(
    ctx: &mut PassContext<'_>,
    fu: &FunctionUnit,
    is_top_level: bool,
    purity: PurityCtx<'_>,
) -> HashSet<(crate::var_resolve::VariableCellKey, u32)> {
    // A dynamic read (`[set $name]`, `subst $tmpl`) can observe *any* store,
    // so no assignment in this function is provably dead.  Deleting one would
    // change what the program prints, so the
    // optimiser abstains toward not folding — no O109/O126, and no ADCE seed.
    // Whole-module variable-trace facts — the same
    // canonicalised (`::`-stripped) fact SCCP and O102 consult. A write
    // trace fires its callback on every store, so no store to a traced
    // name is provably dead even when the `trace add variable` lives in a
    // different proc or spells the target `::var` while the store is
    // unqualified; a dynamic trace target makes *every* name potentially
    // traced, so the whole function abstains.
    if dead_store_observation_unbounded(ctx, fu) {
        return HashSet::new();
    }
    let module_traced = ctx.ir_module.map(|m| &m.traced_variables);
    let unreachable = unreachable_blocks(&fu.cfg, &fu.sccp);
    // Alias recognition is registry-driven; a registry-less context (unit
    // tests) falls back to the cached default so `global`/`upvar` bindings
    // are still respected.
    let scan_registry = purity
        .registry
        .unwrap_or_else(|| tcl_registry::model::ingress::static_context_for("tcl8.6").commands());
    let fallback_contexts;
    let point_contexts = if let Some(contexts) = &fu.ssa.point_contexts {
        contexts
    } else {
        fallback_contexts = crate::variable_bindings::build_point_resolve_contexts(
            &fu.cfg,
            &fu.name,
            scan_registry,
        );
        &fallback_contexts
    };
    let raise_proof = RaiseProof::new(ctx, fu, purity.enclosing_class, is_top_level);
    // The `def_use` builder does not scan Return-value reads or
    // embedded string-interpolation reads; do a supplementary
    // textual pass over the CFG to collect every var name that
    // appears in any source slice. Any def of a name referenced
    // textually is kept live — conservative but correct.
    let textually_referenced = dead_store_textual_reads(ctx, fu);

    // The place model: array-element writes the name-level
    // SSA mis-folds (`set a(k) 1` "overwritten" by `set a(j) 2`) but that a read
    // observes.  Shared with the analyser's W220.  Empty unless a registry is
    // bound (set by the `optimise*` entry points) and the function writes array
    // elements — so the bare test/`run_pass` path keeps its prior behaviour.
    let place_suppressed = observed_element_stores(ctx, fu, point_contexts);

    // Collect one DseEntry per dead chain then sort + emit.
    let mut entries: Vec<DseEntry> = Vec::new();

    for chain in fu.def_use.chains.values() {
        let Some((def_block, idx, stmt)) = live_dead_chain_statement(fu, chain, &unreachable)
        else {
            continue;
        };
        let bound_writes = crate::place_bridge::def_places(
            stmt,
            point_contexts.before_statement(def_block, idx),
            scan_registry,
        );
        if dead_store_writes_observed(
            fu,
            def_block,
            idx,
            &bound_writes,
            point_contexts,
            scan_registry,
        ) {
            continue;
        }

        // O109/O126 closure: gate deletion on RHS purity. The def is
        // dead at the SSA level, but its RHS may have observable side
        // effects (`set unused [puts X]` prints, `set unused [my
        // impureMethod]` mutates object state). Only delete when every
        // embedded command substitution is provably side-effect-free, and
        // when evaluating the value cannot raise (#2249).
        let effect = EffectCtx {
            purity,
            source_tokens: fu.cfg.source_tokens_at(def_block, idx),
        };
        if !assignment_safe_to_delete_with_effect(stmt, effect)
            || !raise_proof.value_cannot_raise(def_block, idx, stmt, &chain.key)
        {
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
        // global / upvar are visible in other scopes. Policy sets hold
        // *base* names, so an element symbol (`a(k)`) checks its base too.

        let (cell_name, _) = &chain.key;
        let Some(symbol) = fu.ssa.cell_symbol(cell_name) else {
            continue;
        };
        let var = fu.ssa.var_name(symbol);
        // A synthetic may-def (base refresh / element fan) is not a write
        // the user made — never an O109/O126 candidate.
        if fu.ssa.is_synthetic_def(
            &chain.definition.block,
            chain.definition.statement_index,
            cell_name,
        ) {
            continue;
        }
        if dead_store_name_observed(ctx, var, module_traced, is_top_level, &bound_writes) {
            continue;
        }
        let Some((code, msg)) = dead_chain_code(fu, chain, is_top_level, &textually_referenced)
        else {
            continue;
        };
        entries.push(DseEntry {
            // CFG statement span is relative to the unit's `base_offset`.
            span: fu.abs_span(
                stmt.source_edit_span()
                    .expect("editable dead-store statement"),
            ),
            code,
            msg,
            key: chain.key.clone(),
            block: chain.definition.block.clone(),
            statement_index: chain.definition.statement_index,
        });
    }

    emit_dse_entries(ctx, fu, entries)
}

fn observed_element_stores(
    ctx: &PassContext<'_>,
    fu: &FunctionUnit,
    contexts: &crate::variable_bindings::PointResolveContexts,
) -> HashSet<(String, i32)> {
    ctx.registry
        .map(|registry| {
            crate::place_bridge::element_writes_observed_with_contexts(&fu.cfg, contexts, registry)
        })
        .unwrap_or_default()
}

/// Unbounded name reads or variable observers can consume every contents store.
fn dead_store_observation_unbounded(ctx: &PassContext<'_>, fu: &FunctionUnit) -> bool {
    fu.cfg.has_opaque_native_accesses()
        || (fu.ssa.point_contexts.is_none()
            && (fu.dynamic_names.reads
                || ctx.ir_module.is_some_and(|m| m.has_dynamic_variable_trace)))
}

/// Supplementary reads retained when the SSA model cannot represent the
/// corresponding interpolation or nested read-modify-write invocation.
fn dead_store_textual_reads(ctx: &PassContext<'_>, fu: &FunctionUnit) -> HashSet<String> {
    let Some(registry) = ctx.registry else {
        return fu.ssa.var_names().iter().cloned().collect();
    };
    let metadata = ctx.ir_module.map_or_else(
        || fu.invocation_metadata_context(registry),
        |module| fu.invocation_metadata_context_for_module(registry, module),
    );
    let mut reads = source_reads::original_hidden_reads(fu, registry, metadata).possible_names(fu);
    reads.extend(collect_textual_var_references(ctx.source, fu, registry));
    reads
}

/// Select an actual assignment from an executable block; synthetic and
/// unreachable definitions have no reportable source statement.
fn live_dead_chain_statement<'a>(
    fu: &'a FunctionUnit,
    chain: &crate::def_use::DefUseChain,
    unreachable: &HashSet<crate::cfg::BlockId>,
) -> Option<(crate::cfg::BlockId, usize, &'a Statement)> {
    if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
        return None;
    }
    let block_id = fu.cfg.block_id(&chain.definition.block)?;
    if unreachable.contains(&block_id) {
        return None;
    }
    let block = fu.cfg.blocks.get(&block_id)?;
    let index = usize::try_from(chain.definition.statement_index).ok()?;
    let statement = block.statements.get(index)?;
    fu.cfg.statement_source_edit_span(block_id, index)?;
    Some((block_id, index, statement))
}

/// Place callbacks and unknown accesses retain a store even when SSA has no use.
fn dead_store_writes_observed(
    fu: &FunctionUnit,
    block: crate::cfg::BlockId,
    index: usize,
    writes: &[crate::place::Place],
    contexts: &crate::variable_bindings::PointResolveContexts,
    registry: &CommandRegistry,
) -> bool {
    writes.iter().any(|written| {
        written.dynamic
            || written.observed
            || written.kind == crate::place::PlaceKind::Unknown
            || crate::place_bridge::write_observed_by_unknown_access(
                &fu.cfg, block, index, written, contexts, registry,
            )
    })
}

/// Text-based external-observation policies are separate from canonical SSA identity.
fn dead_store_name_observed(
    ctx: &PassContext<'_>,
    var: &str,
    module_traced: Option<&std::collections::BTreeSet<String>>,
    is_top_level: bool,
    writes: &[crate::place::Place],
) -> bool {
    // The SSA name is decoded literal naming data, not a `$` reference.
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let base = crate::naming::split_element_ref(var).map_or(var, |(root, _)| root);
    writes.iter().any(|written| {
        written.kind == crate::place::PlaceKind::UpvarAlias
            || written.kind == crate::place::PlaceKind::InstanceVar
            || (!is_top_level && written.ns != crate::place::LOCAL_NS)
    }) || module_traced.is_some_and(|names| names.contains(base.trim_start_matches("::")))
        || var.starts_with("::")
        || ctx.cross_event_vars.contains(var)
        || ctx.cross_event_vars.contains(base)
}

/// Classify one dead def-use chain as O109 (dead store) or O126 (unused
/// variable), or `None` when it must not be reported. Extracted from
/// [`emit_dead_stores_and_unused`].
fn dead_chain_code(
    fu: &FunctionUnit,
    chain: &crate::def_use::DefUseChain,
    is_top_level: bool,
    textually_referenced: &HashSet<String>,
) -> Option<(DiagCode, &'static str)> {
    let var = &chain.key.0;
    // A scalar-fact invalidation does not overwrite the executable cell.
    // Reads of its fresh version still observe the earlier store.
    if let Some(symbol) = fu.ssa.cell_symbol(var)
        && fu.def_use.chains.iter().any(|(key, consumer)| {
            key.0 == *var
                && !consumer.is_dead()
                && fu.ssa.binding_version(symbol, key.1) == chain.key.1
        })
    {
        return None;
    }
    let any_other_live = fu
        .def_use
        .chains
        .iter()
        .any(|(k, c)| k.0 == *var && k.1 != chain.key.1 && !c.is_dead());
    if any_other_live {
        // Dead store — overwritten before read (another version has live
        // consumers). This fires regardless of textual mentions: a later
        // version handles the reads.
        return Some((DiagCode::O109, "Eliminate dead store"));
    }
    if is_top_level {
        // Top-level never emits O126.
        return None;
    }
    // Unused variable — apply the textual-scan keep-live check. The def-use
    // builder does not track reads from `Return` terminators or `"$x"`
    // string interpolations, so a conservative over-approximation of names
    // referenced anywhere in the source text suppresses spurious O126 for
    // legitimately-consumed variables.
    if fu
        .ssa
        .cell_symbol(var)
        .is_some_and(|symbol| textually_referenced.contains(fu.ssa.var_name(symbol)))
    {
        return None;
    }
    Some((DiagCode::O126, "Remove unused variable assignment"))
}

/// Sort the collected dead-store / unused entries by span, record each O109
/// into `ctx.dead_stores`, emit every optimisation, and return the set of
/// eliminated SSA keys (the ADCE seed). Extracted from
/// [`emit_dead_stores_and_unused`].
fn emit_dse_entries(
    ctx: &mut PassContext<'_>,
    fu: &FunctionUnit,
    mut entries: Vec<DseEntry>,
) -> HashSet<(crate::var_resolve::VariableCellKey, u32)> {
    entries.sort_by_key(|e| e.span.start());
    let mut removed: HashSet<(crate::var_resolve::VariableCellKey, u32)> = HashSet::new();
    for e in entries {
        // Record O109 dead stores (not O126 unused vars) so tools can show
        // them from where Rust determines them. `run` collects these into
        // `ctx.dead_stores`.
        if e.code == DiagCode::O109 {
            ctx.dead_stores.push(DeadStore {
                function: fu.name.clone(),
                block: e.block.clone(),
                statement_index: e.statement_index,
                variable: fu.ssa.cell_symbol(&e.key.0).map_or_else(
                    || e.key.0.compatibility_name(),
                    |symbol| fu.ssa.var_name(symbol).to_owned(),
                ),
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
fn emit_adce(
    ctx: &mut PassContext<'_>,
    fu: &FunctionUnit,
    baseline: &HashSet<(crate::var_resolve::VariableCellKey, u32)>,
    purity: PurityCtx<'_>,
    top_level: bool,
) {
    if dead_store_observation_unbounded(ctx, fu) {
        return;
    }
    let (consumer_stmt_keys, mut keep_forever) = build_adce_consumers(fu);
    let registry = purity
        .registry
        .unwrap_or_else(|| tcl_registry::model::ingress::static_context_for("tcl8.6").commands());
    keep_forever.extend(observed_store_definitions(ctx, fu, top_level, registry));
    let stmt_to_defs = build_stmt_to_defs(fu);
    let raise_proof = RaiseProof::new(ctx, fu, purity.enclosing_class, top_level);
    let removed = run_adce_fixpoint(
        fu,
        baseline,
        &consumer_stmt_keys,
        &keep_forever,
        &stmt_to_defs,
        EffectCtx {
            purity,
            source_tokens: None,
        },
        &raise_proof,
    );
    emit_adce_reports(ctx, fu, baseline, &removed);
}

/// A target observed outside the removable chain remains a store even after
/// its local consumers disappear. Reuse the ordinary dead-store place policy.
fn observed_store_definitions(
    ctx: &PassContext<'_>,
    fu: &FunctionUnit,
    top_level: bool,
    registry: &CommandRegistry,
) -> HashSet<(crate::var_resolve::VariableCellKey, u32)> {
    let fallback_contexts;
    let contexts = if let Some(contexts) = &fu.ssa.point_contexts {
        contexts
    } else {
        fallback_contexts =
            crate::variable_bindings::build_point_resolve_contexts(&fu.cfg, &fu.name, registry);
        &fallback_contexts
    };
    fu.def_use
        .chains
        .values()
        .filter_map(|chain| {
            if chain.definition.kind != DefKind::Statement {
                return None;
            }
            let block = fu.cfg.block_id(&chain.definition.block)?;
            let index = usize::try_from(chain.definition.statement_index).ok()?;
            let statement = fu.cfg.blocks.get(&block)?.statements.get(index)?;
            let writes = crate::place_bridge::def_places(
                statement,
                contexts.before_statement(block, index),
                registry,
            );
            let symbol = fu.ssa.cell_symbol(&chain.key.0)?;
            (dead_store_writes_observed(fu, block, index, &writes, contexts, registry)
                || dead_store_name_observed(
                    ctx,
                    fu.ssa.var_name(symbol),
                    ctx.ir_module.map(|module| &module.traced_variables),
                    top_level,
                    &writes,
                ))
            .then(|| chain.key.clone())
        })
        .collect()
}

type ConsumerMap = HashMap<(crate::var_resolve::VariableCellKey, u32), Vec<(String, usize)>>;

fn build_adce_consumers(
    fu: &FunctionUnit,
) -> (
    ConsumerMap,
    HashSet<(crate::var_resolve::VariableCellKey, u32)>,
) {
    use crate::def_use::UseKind;
    let mut consumer_stmt_keys: ConsumerMap = HashMap::new();
    let mut keep_forever: HashSet<(crate::var_resolve::VariableCellKey, u32)> = HashSet::new();
    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != DefKind::Statement {
            continue;
        }
        let mut key = chain.key.clone();
        if let Some(symbol) = fu.ssa.cell_symbol(&key.0) {
            key.1 = fu.ssa.binding_version(symbol, key.1);
        }
        for use_site in &chain.uses {
            match use_site.kind {
                // A name position consumes the value exactly as an operand
                // does — `incr a` reads `a` — so it keeps the feeding store
                // alive at the statement that names it. Only *rewriting*
                // passes have to tell the two apart.
                UseKind::Operand | UseKind::VariableName => {
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

type StmtDefsMap = HashMap<(String, usize), Vec<(crate::var_resolve::VariableCellKey, u32)>>;

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
    baseline: &HashSet<(crate::var_resolve::VariableCellKey, u32)>,
    consumer_stmt_keys: &ConsumerMap,
    keep_forever: &HashSet<(crate::var_resolve::VariableCellKey, u32)>,
    stmt_to_defs: &StmtDefsMap,
    effect: EffectCtx<'_>,
    raise_proof: &RaiseProof<'_>,
) -> HashSet<(crate::var_resolve::VariableCellKey, u32)> {
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
            let Some(def_block) = fu.cfg.block_id(&chain.definition.block) else {
                continue;
            };
            if unreachable.contains(&def_block) {
                continue;
            }
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(block) = fu.cfg.blocks.get(&def_block) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            // O108 purity gate: a transitively-dead assignment can only be
            // removed when its RHS has no observable side effect — an
            // embedded `[cmd …]` that mutates state or escapes must keep the
            // statement live. This reuses the same `PurityCtx` /
            // `assignment_safe_to_delete` gate O109/DSE applies rather than
            // treating every assignment as pure.
            if !assignment_safe_to_delete_with_effect(
                stmt,
                EffectCtx {
                    source_tokens: fu.cfg.source_tokens_at(def_block, idx),
                    ..effect
                },
            ) || !raise_proof.value_cannot_raise(def_block, idx, stmt, key)
            {
                continue;
            }
            let empty: Vec<(String, usize)> = Vec::new();
            let consumers: &Vec<(String, usize)> = consumer_stmt_keys.get(key).unwrap_or(&empty);
            if consumers.is_empty() {
                continue;
            }
            let all_removed = consumers.iter().all(|pair: &(String, usize)| {
                stmt_to_defs.get(pair).is_some_and(
                    |defs: &Vec<(crate::var_resolve::VariableCellKey, u32)>| {
                        defs.iter()
                            .all(|d: &(crate::var_resolve::VariableCellKey, u32)| {
                                removed.contains(d)
                            })
                    },
                )
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
    baseline: &HashSet<(crate::var_resolve::VariableCellKey, u32)>,
    removed: &HashSet<(crate::var_resolve::VariableCellKey, u32)>,
) {
    let mut new_reports: Vec<tcl_lexer::Span> = Vec::new();
    for key in removed.difference(baseline) {
        if let Some(chain) = fu.def_use.chains.get(key) {
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(block) = fu.cfg.block_by_name(&chain.definition.block) else {
                continue;
            };
            if block.statements.get(idx).is_none() {
                continue;
            }
            // CFG statement span is relative to the unit's `base_offset`.
            if let Some(id) = fu.cfg.block_id(&chain.definition.block)
                && let Some(span) = fu.cfg.statement_source_edit_span(id, idx)
            {
                new_reports.push(fu.abs_span(span));
            }
        }
    }
    new_reports.sort_by_key(|s| s.start());
    for span in new_reports {
        ctx.report(Optimisation::new(
            DiagCode::O108,
            "Eliminate transitively dead code",
            full_rewrite_span(ctx.source, span),
            "",
        ));
    }
}

/// Possible source reads using actual function grammar and original targets.
pub(crate) fn collect_textual_var_references(
    source: &str,
    function: &FunctionUnit,
    registry: &CommandRegistry,
) -> HashSet<String> {
    source_reads::original_textual_reads(source, function, registry)
}

/// By-name/expression reads hidden in genuine nested source invocations.
/// Missing owners retain unknown reads rather than donating catalogue roles.
pub(crate) fn collect_rmw_hidden_reads(
    function: &FunctionUnit,
    registry: &CommandRegistry,
) -> HashSet<String> {
    source_reads::original_hidden_reads(
        function,
        registry,
        function.invocation_metadata_context(registry),
    )
    .possible_names(function)
}

/// Supplied availability refines original alias and write-trace declarations.
/// Missing or foreign metadata contributes no declarations; callers must not
/// interpret this absence as completed lookup or an unobserved physical cell.
pub(crate) fn scan_scope_aliases_with_metadata_context(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> HashSet<String> {
    let mut aliases = HashSet::new();
    let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
        return aliases;
    };
    for statement in cfg.blocks.values().flat_map(|block| &block.statements) {
        let Some(invocation) =
            crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
                registry,
                Some(context),
                statement,
            )
        else {
            continue;
        };
        if let Some(transitions) = invocation.facts.state_transitions.declared() {
            for fact in transitions.facts() {
                if let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition
                    && let Some(local) = alias.local.literal()
                {
                    aliases.insert(local.to_owned());
                }
            }
        }
        aliases.extend(statement_write_trace_targets(
            statement,
            registry,
            Some(context),
        ));
    }
    aliases
}

fn statement_write_trace_targets(
    statement: &Statement,
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Vec<String> {
    let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
        return Vec::new();
    };
    let Some(invocation) =
        crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
            registry,
            Some(context),
            statement,
        )
    else {
        return Vec::new();
    };
    let Some(transitions) = invocation.facts.state_transitions.declared() else {
        return Vec::new();
    };
    transitions.facts().iter().filter_map(|fact| {
        let tcl_registry::StateTransition::Trace(trace) = &fact.transition else { return None; };
        let (target, operations) = match trace { tcl_registry::TraceTransition::Add { target, operations, .. } | tcl_registry::TraceTransition::Remove { target, operations, .. } => (target, operations) };
        let tcl_registry::TraceTarget::Variable(target) = target else { return None; };
        if matches!(operations, tcl_registry::TraceOperationSet::Known(operations) if !operations.contains(&tcl_registry::TraceOperation::Write)) { return None; }
        target.literal().map(str::to_owned)
    }).collect()
}

/// Literal source aliases that name the interpreter's root startup variable.
/// A qualified target in another namespace does not inherit a root binding.
/// These are analytical source names; Native storage identity remains separate.
pub(crate) fn scan_global_scope_aliases_with_metadata_context(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> HashSet<String> {
    let mut aliases = HashSet::new();
    let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
        return aliases;
    };
    for statement in cfg.blocks.values().flat_map(|block| &block.statements) {
        let Some(invocation) = statement.tokens().and_then(|tokens| {
            crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                registry,
                Some(context),
                tokens,
            )
        }) else {
            continue;
        };
        for alias in invocation.variable_alias_transitions() {
            let tcl_registry::VariableAliasTarget::Global { variable } = &alias.target else {
                continue;
            };
            let (Some(local), Some(target)) = (alias.local.literal(), variable.literal()) else {
                continue;
            };
            let qualified = tcl_syntax::naming::qualify("::", target);
            let (holder, tail) = tcl_syntax::naming::key_holder_and_tail(&qualified);
            if holder == "::" && tail == local {
                aliases.insert(local.to_owned());
            }
        }
    }
    aliases
}

/// Namespace-qualified write-trace hazards under supplied source availability.
/// Missing metadata does not certify an empty physical trace table.
pub(crate) fn scan_module_traced_globals_with_metadata_context(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> HashSet<String> {
    let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
        return HashSet::new();
    };
    cu.all_body_function_units()
        .flat_map(|unit| unit.cfg.blocks.values().flat_map(|block| &block.statements))
        .flat_map(|statement| statement_write_trace_targets(statement, registry, Some(context)))
        .filter(|target| target.contains("::"))
        .collect()
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

    #[test]
    fn original_global_alias_scan_retains_literal_target_and_command_identity() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Conditional source startup names are independent of Native cell identity.
        let registry = std::sync::Arc::new(CommandRegistry::build_default());
        let context = tcl_registry::model::ingress::static_context_for("tcl9.0")
            .with_command_store(std::sync::Arc::clone(&registry));
        for (source, expected) in [
            (
                "proc p {} {global argc {$argv} {scalar(open} ::other::argc}",
                vec!["argc", "$argv", "scalar(open"],
            ),
            ("proc global args {}; proc p {} {global argc}", vec![]),
            ("rename global moved; proc p {} {moved argc}", vec!["argc"]),
            (
                "interp alias {} link {} global argc; proc p {} {link}",
                vec!["argc"],
            ),
            (
                "rename global moved; proc global args {}; proc p {} {global argc}",
                vec![],
            ),
        ] {
            let compilation = CompilationUnit::build_for(source, &registry, false);
            let function = compilation.function("::p").unwrap();
            let names = scan_global_scope_aliases_with_metadata_context(
                &function.cfg,
                &registry,
                Some((&context).into()),
            );
            assert_eq!(
                names,
                expected.into_iter().map(str::to_owned).collect(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_alias_scans_refuse_unavailable_and_foreign_metadata() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "gated-global",
            surface: registry.get("dict").unwrap().surface,
            ..registry.get("global").unwrap().clone()
        });
        let registry = std::sync::Arc::new(registry);
        let compilation =
            CompilationUnit::build_for("proc p {} {gated-global argc}", &registry, false);
        let function = compilation.function("::p").unwrap();
        for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
            let context = tcl_registry::model::ingress::static_context_for(environment)
                .with_command_store(std::sync::Arc::clone(&registry));
            let metadata = Some((&context).into());
            assert_eq!(
                scan_scope_aliases_with_metadata_context(&function.cfg, &registry, metadata)
                    .contains("argc"),
                available,
                "{environment}"
            );
            assert_eq!(
                scan_global_scope_aliases_with_metadata_context(&function.cfg, &registry, metadata)
                    .contains("argc"),
                available,
                "{environment}"
            );
        }
        assert!(
            scan_global_scope_aliases_with_metadata_context(&function.cfg, &registry, None)
                .is_empty()
        );
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0");
        assert!(
            scan_scope_aliases_with_metadata_context(
                &function.cfg,
                &registry,
                Some(foreign.into())
            )
            .is_empty()
        );
    }

    // internal helper tests

    /// Deep analytical expressions remain stack-safe; source substitutions
    /// without original receipts remain uncertain even under a pure name label.
    #[test]
    fn deeply_nested_side_effect_walks_survive() {
        let reg = registry();
        let interproc_pure = ["a".to_owned()].into_iter().collect();
        let purity = PurityCtx {
            registry: Some(&reg),
            interproc_pure: &interproc_pure,
            enclosing_class: None,
            config: tcl_lexer::LexerConfig::default(),
            metadata: None,
            module: None,
        };
        let effect = EffectCtx {
            purity,
            source_tokens: None,
        };
        let mut deep = "x".to_owned();
        for _ in 0..3000 {
            deep = format!("[a {deep}]");
        }
        assert!(word_has_observable_side_effect(&deep, effect));
        let mut node = ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: crate::expr_ast::UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        assert!(expr_has_observable_side_effect(&node, effect, 0));
    }

    #[test]
    fn resolved_store_observer_advice_preserves_literal_sigil_and_scalar_names() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // This is the store-observer suppression gate, not a current Native
        // trace, reaching cell, SSA use or deletion proof.
        let mut context = PassContext::new("", InterproceduralAnalysis::default());
        let traced = ["counter".to_owned(), "café".to_owned()]
            .into_iter()
            .collect();
        assert!(dead_store_name_observed(
            &context,
            "counter",
            Some(&traced),
            true,
            &[]
        ));
        assert!(!dead_store_name_observed(
            &context,
            "$counter",
            Some(&traced),
            true,
            &[]
        ));
        assert!(!dead_store_name_observed(
            &context,
            "café(open",
            Some(&traced),
            true,
            &[]
        ));
        assert!(dead_store_name_observed(
            &context,
            "café(key)",
            Some(&traced),
            true,
            &[]
        ));
        context.cross_event_vars.insert("$counter".into());
        assert!(dead_store_name_observed(
            &context,
            "$counter(key)",
            None,
            true,
            &[]
        ));
        assert!(!dead_store_name_observed(
            &context,
            "counter",
            None,
            true,
            &[]
        ));
    }

    #[test]
    fn unreachable_blocks_empty_when_all_executable() {
        let cu = CompilationUnit::build_for("set x 1", &registry(), false);
        let unreach = unreachable_blocks(&cu.top_level.cfg, &cu.top_level.sccp);
        assert_eq!(unreach.len(), 0);
    }

    // end-to-end tests

    #[test]
    fn opaque_native_reads_keep_named_stores_live_without_synthetic_uses() {
        let registry = registry();
        let mut unit = CompilationUnit::build_for("set retained VALUE", &registry, false);
        let function = &mut unit.top_level;
        let native = crate::ir::native_call_for_test(b"opaque \xff");
        function
            .cfg
            .blocks
            .get_mut(&function.cfg.entry)
            .unwrap()
            .statements
            .push(native);
        let mut context = PassContext::new(&unit.source, InterproceduralAnalysis::default());
        context.registry = Some(&registry);
        assert!(dead_store_observation_unbounded(&context, &unit.top_level));
        assert!(unit.top_level.dynamic_barrier_blocks_value_motion());
        run(&mut context, &unit);
        assert!(!context.optimisations.iter().any(|optimisation| {
            matches!(
                optimisation.code,
                DiagCode::O108 | DiagCode::O109 | DiagCode::O126
            )
        }));
    }

    #[test]
    fn empty_source_produces_nothing() {
        let opts = run_pass("");
        assert_eq!(opts, [] as [crate::optimiser::Optimisation; 0]);
    }

    /// Like [`run_pass`] but with `ctx.ir_module` wired the way the
    /// production entry points wire it, so the whole-module variable-trace
    /// facts reach the O109 / O126 gate.
    fn run_pass_with_module(source: &str) -> Vec<Optimisation> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.ir_module = Some(&cu.ir_module);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    /// The trace names `::g`, the store is spelled `g`, and
    /// both name the same top-level global; the write trace observes
    /// `set g 1`, so it is not a dead store.
    #[test]
    fn traced_global_unqualified_store_is_not_dead() {
        let src = "proc onw {a b c} { puts trace }\ntrace add variable ::g write ::onw\nset g 1\nset g 2\nputs $g";
        let opts = run_pass_with_module(src);
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O109),
            "traced global's store must not be a dead store, got {opts:?}",
        );
    }

    /// A dynamic trace target makes every name potentially
    /// traced, so no store anywhere in the module is provably dead.
    #[test]
    fn dynamic_trace_target_blocks_dead_stores() {
        let src = "proc onw {a b c} { puts trace }\ntrace add variable $n write ::onw\nset g 1\nset g 2\nputs $g";
        let opts = run_pass_with_module(src);
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O109),
            "dynamic trace target must block dead-store elimination, got {opts:?}",
        );
    }

    /// Control for the two tests above: without any trace the overwritten
    /// store still reports O109.
    #[test]
    fn untraced_overwritten_store_still_reports_o109() {
        let opts = run_pass_with_module("set g 1\nset g 2\nputs $g");
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O109),
            "control: untraced overwritten store must still fire, got {opts:?}",
        );
    }

    #[test]
    fn straight_line_script_is_fully_reachable() {
        let opts = run_pass("set x 1\nset y 2\nputs $x");
        assert_eq!(opts, [] as [crate::optimiser::Optimisation; 0]);
    }

    #[test]
    fn branch_folding_creates_unreachable_block() {
        // The else branch is unreachable under SCCP because the
        // condition folds to true.
        let opts = run_pass("if {1} { set x 1 } else { set y 2 }");
        // Expect at least one O107 — the else body's `set y 2`.
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O107 && o.message == "Eliminate unreachable dead code"),
            "expected at least one O107, got {opts:?}",
        );
    }

    #[test]
    fn while_false_body_is_unreachable() {
        // The body of `while {0} { ... }` is unreachable.
        let opts = run_pass("while {0} { set x 1 }");
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O107),
            "expected an O107 for dead while body, got {opts:?}",
        );
    }

    #[test]
    fn unreachable_for_clause_markers_are_not_source_commands() {
        for source in [
            "for {set previous NONEMPTY} {0} {} {}",
            "for {} {0} {} {puts unreachable}",
        ] {
            let opts = run_pass(source);
            for edit in opts.iter().filter(|edit| edit.code == DiagCode::O107) {
                let written = &source[edit.span.start() as usize..edit.span.end() as usize];
                assert_ne!(written, "{}", "synthetic clause must retain argv: {opts:?}");
            }
            if source.contains("puts unreachable") {
                assert!(
                    opts.iter().any(|edit| edit.code == DiagCode::O107
                        && source[edit.span.start() as usize..edit.span.end() as usize]
                            .contains("puts unreachable")),
                    "actual dead command remains editable: {opts:?}"
                );
            }
        }
    }

    #[test]
    fn unreachable_statements_emitted_with_empty_replacement() {
        let opts = run_pass("if {0} { set x 1 }");
        let target = opts.iter().find(|o| o.code == DiagCode::O107);
        if let Some(o) = target {
            assert_eq!(o.replacement, "");
            assert!(!o.span.is_empty());
        }
    }

    #[test]
    fn callee_read_preserves_only_the_contents_that_reach_it() {
        let source = "proc read_write {name} {upvar 1 $name value; puts $value; set value done}\nproc f {} {set tag init; set tag live; read_write tag}\nf";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let optimisations = crate::optimiser::optimise_raw(source, registry, None);
        let removes = |text: &str| {
            let start = u32::try_from(source.find(text).unwrap()).unwrap();
            optimisations.iter().any(|optimisation| {
                matches!(optimisation.code, DiagCode::O109 | DiagCode::O126)
                    && optimisation.span.start() <= start
                    && optimisation.span.end() > start
            })
        };
        assert!(
            removes("set tag init"),
            "overwritten contents: {optimisations:?}"
        );
        assert!(
            !removes("set tag live"),
            "callee observes live contents: {optimisations:?}"
        );
    }

    #[test]
    fn native_getter_in_a_callee_argument_preserves_the_reaching_caller_store() {
        let source = "proc read_value {name} {upvar 1 $name value; set value}\nproc f {} {set tag init; set tag live; puts [read_value tag]}\nf";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let optimisations = crate::optimiser::optimise_raw(source, registry, None);
        let removes = |text: &str| {
            let start = u32::try_from(source.find(text).unwrap()).unwrap();
            optimisations.iter().any(|optimisation| {
                matches!(optimisation.code, DiagCode::O109 | DiagCode::O126)
                    && optimisation.span.start() <= start
                    && optimisation.span.end() > start
            })
        };
        assert!(removes("set tag init"), "overwritten: {optimisations:?}");
        assert!(
            !removes("set tag live"),
            "nested native getter: {optimisations:?}"
        );
    }

    #[test]
    fn analysis_value_clobber_keeps_executable_store_live() {
        for body in [
            "set local 1; upvar 1 $v alias; return $local",
            "set local 1; upvar 1 $v alias; puts $local",
            "set local 1; upvar 1 $v alias; set copy $local; return $copy",
        ] {
            let source = format!("proc p {{v}} {{{body}}}");
            let opts = crate::optimiser::optimise(&source, &registry());
            assert!(
                opts.iter().all(|opt| !matches!(
                    opt.code,
                    DiagCode::O108 | DiagCode::O109 | DiagCode::O126
                )),
                "{source}: {opts:?}"
            );
        }
    }

    #[test]
    fn dead_store_fires_o109_when_overwritten_before_read() {
        // First set is dead — second version is the only live
        // value of x when read by puts.
        let opts = run_pass("set x 1\nset x 2\nputs $x");
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O109),
            "expected O109 for overwritten store, got {opts:?}",
        );
    }

    #[test]
    fn qualified_global_write_is_not_a_dead_store() {
        // `set ::counter 42` inside a proc is a global write
        // visible to other scopes; eliminating it (O109/O126) leaves
        // `::counter` undefined. It must be kept even though the proc
        // never reads it.
        let src = "proc ::setit {} { set ::counter 42 }\n::setit\nputs $::counter";
        let opts = crate::optimiser::optimise(src, &registry());
        let removes_counter = opts.iter().any(|o| {
            (o.code == DiagCode::O109 || o.code == DiagCode::O126)
                && src
                    .get(o.span.start() as usize..o.span.end() as usize)
                    .is_some_and(|t| t.contains("::counter"))
        });
        assert!(
            !removes_counter,
            "must not remove the `set ::counter` global write, got {opts:?}",
        );
    }

    #[test]
    fn o109_array_element_overwrite_not_dead() {
        // Per-element SSA: `a(k)` and `a(j)` are independent variables, so
        // `$a(k)` reads the value of `set a(k)` — never `set a(j)`. With a
        // constant value the pipeline may forward + inline it and then
        // delete the store as a *coupled* rewrite, but the forwarded value
        // must be the same element's `1` (conflation would forward `2`).
        let opts = crate::optimiser::optimise(
            "proc f {} { set a(k) 1; set a(j) 2; puts $a(k) }",
            &registry(),
        );
        // Scoped to the payload-bearing rewrites: a hint-only O102 spans the
        // whole consuming statement and deliberately carries no replacement
        // so it cannot conflate anything.
        assert!(
            opts.iter()
                .all(|o| o.code != DiagCode::O102 || o.hint_only || o.replacement == "1"),
            "a forwarded a(k) load must carry a(k)'s value, got {opts:?}",
        );
        // A non-constant element value cannot be forwarded, so the store
        // stays live through its read — no dead-store report of any kind.
        let live = crate::optimiser::optimise(
            "proc f {x} { set a(k) $x; set a(j) 2; puts $a(k) }",
            &registry(),
        );
        assert!(
            live.iter().all(|o| o.code != DiagCode::O109),
            "a(k) is read by `puts $a(k)`; got {live:?}",
        );
    }

    #[test]
    fn o109_array_element_genuinely_dead_still_fires() {
        // Precision guard: here only a(j) is read, so a(k) IS overwritten
        // before any read — O109 must still fire on it.  The place model
        // suppresses only element writes that a read actually observes.
        let opts = crate::optimiser::optimise("set a(k) 1\nset a(j) 2\nputs $a(j)", &registry());
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O109),
            "O109 expected — a(k) is overwritten and never read; got {opts:?}",
        );
    }

    #[test]
    fn unused_variable_fires_o126_in_proc_body() {
        let opts = run_pass("proc ::f {} { set y 42; return 1 }");
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O126),
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
            opts.iter().all(|o| o.code != DiagCode::O126),
            "top-level unused var should not emit O126, got {opts:?}",
        );
    }

    #[test]
    fn scope_alias_globals_never_flagged() {
        let opts = run_pass("proc ::f {} { global g; set g 42 }");
        assert!(
            opts.iter()
                .all(|o| o.code != DiagCode::O109 && o.code != DiagCode::O126),
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
        let o108 = opts.iter().filter(|o| o.code == DiagCode::O108).count();
        assert!(
            o108 >= 1,
            "expected at least one O108 in transitive dead chain, got {opts:?}",
        );
    }

    #[test]
    fn original_adce_keeps_actual_remote_stores_after_local_consumers_disappear() {
        // Implementation contract: naming.variable.transitive-store-observation
        // docs/design/analysis/name-resolution-proofs/transitive-store-observation.md
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
            let source = "proc f {} {global exported; set exported OLD; set first $exported; set unused $first; return OK}; f";
            let unit = CompilationUnit::build_for_dialect(source, registry, false, engine);
            let function = unit.procedures.get("::f").unwrap();
            let offset = u32::try_from(source.find("set exported OLD").unwrap()).unwrap();
            let (&block, index, statement) = function
                .cfg
                .blocks
                .iter()
                .find_map(|(block, body)| {
                    body.statements
                        .iter()
                        .enumerate()
                        .find_map(|(index, statement)| {
                            (function.abs_span(statement.span()).start() == offset)
                                .then_some((block, index, statement))
                        })
                })
                .unwrap();
            let context = function
                .ssa
                .point_contexts
                .as_ref()
                .unwrap()
                .context_before(block, index)
                .unwrap();
            let writes = crate::place_bridge::def_places(statement, context, registry);
            assert!(writes.iter().any(|place| {
                matches!(crate::var_resolve::cell_key(place).root(), crate::var_resolve::VariableCellKey::Namespace { simple, .. }
                    if simple.as_bytes() == b"exported")
            }), "actual remote target retained for {engine}");
            let mut pass = PassContext::new(&unit.source, InterproceduralAnalysis::default());
            pass.registry = Some(registry);
            pass.ir_module = Some(&unit.ir_module);
            run(&mut pass, &unit);
            assert!(
                pass.optimisations.iter().any(|optimisation| {
                    matches!(
                        optimisation.code,
                        DiagCode::O108 | DiagCode::O109 | DiagCode::O126
                    ) && source
                        .get(optimisation.span.start() as usize..optimisation.span.end() as usize)
                        .is_some_and(|text| text.contains("set unused"))
                }),
                "the private consumer still qualifies for {engine}"
            );
            assert!(
                pass.optimisations.iter().all(|optimisation| {
                    !matches!(
                        optimisation.code,
                        DiagCode::O108 | DiagCode::O109 | DiagCode::O126
                    ) || source
                        .get(optimisation.span.start() as usize..optimisation.span.end() as usize)
                        .is_none_or(|text| !text.contains("set exported OLD"))
                }),
                "remote store survives its dead local chain for {engine}"
            );
        }
    }

    #[test]
    fn adce_preserves_impure_link_in_dead_chain() {
        // `b` is read only by the dead `set c $b`, so the transitive
        // chain `set a [puts hi]` → `set b $a` → `set c $b` is dead by
        // def-use. But `set a [puts hi]` still prints — the O108 purity
        // gate must keep it, deleting at most the pure links.
        let src = "proc ::f {} { set a [puts hi]; set b $a; set c $b; return 7 }";
        let opts = crate::optimiser::optimise(src, &registry());
        let removes_puts_line = opts.iter().any(|o| {
            matches!(o.code.as_str(), "O108" | "O109" | "O126")
                && src
                    .get(o.span.start() as usize..o.span.end() as usize)
                    .is_some_and(|slice| slice.contains("puts"))
        });
        assert!(
            !removes_puts_line,
            "impure `set a [puts hi]` must survive ADCE, got {opts:?}",
        );
    }

    #[test]
    fn used_variable_not_flagged() {
        let opts = run_pass("proc ::f {} { set x 1; return $x }");
        assert!(
            opts.iter()
                .all(|o| o.code != DiagCode::O109 && o.code != DiagCode::O126),
            "used var should not be flagged, got {opts:?}",
        );
    }

    #[test]
    fn collect_textual_var_references_detects_set_one_arg_read() {
        // ``[set varname]`` (1-arg form) is a variable read; without
        // it, DCE saw 0 reads on ``varname`` and incorrectly deleted
        // the write.
        let opts = run_pass("proc ::f {} { set x 1; set y [set x]; return $y }");
        // ``x`` is read via ``[set x]`` so neither O109 nor O126
        // should fire on the ``set x 1`` line.
        let bad: Vec<_> = opts
            .iter()
            .filter(|o| {
                (o.code == DiagCode::O109 || o.code == DiagCode::O126) && o.message.contains('x')
            })
            .collect();
        assert_eq!(
            bad.len(),
            0,
            "[set x] should count as a read for x; got {opts:?}",
        );
    }

    /// The same shape through the real entry point: an incomplete command in
    /// a document must return conservative results, not crash the pass.
    #[test]
    fn textual_var_reference_scan_survives_an_incomplete_command_in_a_document() {
        for src in [
            "proc ::f {} { set x 1; return [set ",
            "proc ::f {} { set x 1; return [set {",
            "proc ::f {} { set x 1; return [set {$n",
        ] {
            let opts = run_pass(src);
            // No assertion on the verdicts — a half-typed document may
            // legitimately produce anything or nothing. The point is that it
            // returns at all.
            let _ = opts;
        }
    }

    #[test]
    fn collect_textual_var_references_detects_qualified_set_read() {
        // ``[::set varname]`` form should also count.
        let opts = run_pass("proc ::f {} { set x 1; set y [::set x]; return $y }");
        let bad: Vec<_> = opts
            .iter()
            .filter(|o| {
                (o.code == DiagCode::O109 || o.code == DiagCode::O126) && o.message.contains('x')
            })
            .collect();
        assert_eq!(
            bad.len(),
            0,
            "[::set x] should count as a read for x; got {opts:?}",
        );
    }

    // O126/O109 RHS-purity gate

    #[test]
    fn o126_preserved_for_impure_command_sub_rhs() {
        // `set unused [puts hi]` discards the result but still prints —
        // the assignment must NOT be deleted (a latent FP: O126 would
        // otherwise fire unconditionally on dead chains).
        let opts = crate::optimiser::optimise(
            "proc ::f {} { set unused [puts hi]; return 1 }",
            &registry(),
        );
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O126),
            "impure cmd-sub RHS must be preserved, got {opts:?}",
        );
    }

    #[test]
    fn o126_uses_original_callee_integer_completion_and_preserves_failures() {
        for (body, actual, erasable) in [
            ("expr {$a+$b}", "1 2", true),
            ("expr {$a+$b}", "abc 2", false),
            ("expr {$a/$b}", "1 0", false),
            ("expr {$a+$b}", "1", false),
            ("puts EFFECT; expr {$a+$b}", "1 2", false),
        ] {
            let source = format!(
                "proc add {{a b}} {{{body}}}; proc f {{}} {{set unused [add {actual}]; puts done}}"
            );
            let opts = crate::optimiser::optimise(&source, &registry());
            assert_eq!(
                opts.iter()
                    .any(|opt| opt.code == DiagCode::O126 && opt.message.contains("unused")),
                erasable,
                "{source}: {opts:?}"
            );
        }
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
            opts.iter().any(|o| o.code == DiagCode::O126),
            "pure-proc RHS should fold to O126, got {opts:?}",
        );
    }

    #[test]
    fn o126_still_fires_for_literal_rhs() {
        // The gate must not regress the literal case — `set y 42` has
        // no RHS side effect and stays foldable.
        let opts = run_pass("proc ::f {} { set y 42; return 1 }");
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O126),
            "literal RHS should still fold, got {opts:?}",
        );
    }

    // method-body O126

    #[test]
    fn runtime_selected_my_dispatch_is_not_assumed_pure() {
        // Tcl 9.0.4 resolves bare commands in the receiving object's runtime
        // namespace. Installing `${object_namespace}::my` shadows TclOO's
        // normal self-dispatch command, so even a same-named lexical method
        // proven pure is not an exact binding proof for `[my pure]`.
        let src = "oo::class create C {\n\
                   \x20   method pure {} { ::return 1 }\n\
                   \x20   method uses {} { ::set unused [my pure]; ::return 2 }\n\
                   }";
        let opts = crate::optimiser::optimise(src, &registry());
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O126),
            "runtime-selected `my` binding must preserve the RHS, got {opts:?}",
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
            opts.iter().all(|o| o.code != DiagCode::O126),
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
            opts.iter()
                .all(|o| o.code != DiagCode::O109 && o.code != DiagCode::O126),
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
        let only_o107 = ctx.optimisations.iter().all(|o| o.code == DiagCode::O107);
        assert!(only_o107, "unexpected codes: {:?}", ctx.optimisations);
    }

    /// A write the profile's registry does not have is not a write: under
    /// `tcl8.4` there is no `lassign`, so the store ahead of one stays (#2144).
    ///
    /// Measured on the real interpreters. For
    ///
    /// ```tcl
    /// proc unknown {args} {return -code error absent}
    /// set a old
    /// catch {lassign {new second} a b} m
    /// puts $a
    /// ```
    ///
    /// tclsh 8.4.20 prints `old` — the body raises
    /// the explicitly closed error-only fallback before any write and `catch` swallows
    /// it — while 8.6.18 prints `new`. O109 used to delete `set a old` under
    /// both, and the 8.4 program then failed with
    /// `can't read "a": no such variable`.
    #[test]
    fn a_write_by_a_command_the_profile_lacks_does_not_kill_the_store() {
        let source = "proc unknown {args} {return -code error absent}\nset a old\ncatch {lassign {new second} a b} m\nputs $a\n";

        // The registry has to be the dialect's own, as production builds it
        // (`static_context_for_profile`): availability is a property of the
        // loaded surface, so a default registry would still carry `lassign`.
        let codes = |dialect: &str| {
            let profile =
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
            crate::optimiser::optimise_raw_for_profile(
                source,
                tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
                Some(profile),
            )
            .into_iter()
            .map(|o| o.code)
            .collect::<Vec<_>>()
        };

        // Under 8.4 the actual error-only fallback preserves `a`; catalogue
        // absence alone cannot exclude autoload or a mutating unknown handler.
        // Its value is forwarded and the store
        // that fed it is then genuinely dead. The rewritten program prints
        // `old`, which is what tclsh 8.4.20 prints.
        let early = codes("tcl8.4");
        assert!(
            early.contains(&DiagCode::O102),
            "8.4 must forward `old` — no lassign there to overwrite it: {early:?}",
        );

        // From 8.5 `lassign` really does write `a`, so its value at the `puts`
        // is unknown and nothing may be forwarded; the store is dead on its own
        // and the program still prints `new`.
        let late = codes("tcl8.6");
        assert!(
            !late.contains(&DiagCode::O102),
            "8.6 must not forward a value lassign overwrites: {late:?}",
        );
        assert!(
            late.contains(&DiagCode::O109),
            "but the store is still dead there: {late:?}",
        );
    }
}
