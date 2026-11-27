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
//! - **O109** — dead stores (an SSA def with no use that can run and
//!   whose variable has at least one later definition — the
//!   write is overwritten before any read; a φ's read over the edge
//!   from the block before a `catch` or `try` body counts only where
//!   the solver opens that edge, [`has_running_use`]).
//! - **O126** — unused variable assignments (an SSA def whose
//!   chain is empty and is the only / last def of that variable
//!   — the value is never read). Skipped at the top level (the
//!   last command's result may be the script return value) and
//!   for scope-alias commands (`global` / `variable` / `upvar`).
//!
//! A store is removable only when no value read and no existence read of
//! its version remains: `[info exists x]`,
//! `[array exists x]` and an unbind — `unset x`, `array unset x` — read the
//! version they observe as an SSA use wherever they run, a statement, a
//! condition, a nested word or a `return` word, so the store they observe
//! stays. An unbind statement is never removed: the error on an absent
//! place and the binding's disappearance are its effects. A store whose
//! value can raise stays as well (`RaiseProof`): deleting it would drop
//! the error, so a value reading a variable the existence rung does not hold
//! bound as a scalar where it reads it, an `expr` that does not fold, a
//! command substitution that does not fold — but a call to a procedure the
//! summary proves pure and completing whatever its arguments hold — and an
//! `incr` that may find its place absent under a release the profile spans,
//! or holding something other than an integer, are kept.
//!
//! Emission order is the deterministic CFG `cfg_order` (reverse
//! post-order from the entry, unreachable blocks appended).

mod source_purity;
mod source_reads;

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;
use tcl_lexer::TokenType;

use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::{BindingKind, Existence, NumericValue};

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::def_use::DefKind;
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::sccp::{SccpResult, cfg_order};
use tcl_source::limits::MAX_EXPR_NODE_DEPTH;

use super::helpers::spans::{line_delete_span, original_statement_rewrite_span};
use super::{Optimisation, PassContext};

/// Actual source input for the substitution effect gate. Interprocedural
/// names assist only a Logical target joined to its original declaration.
#[derive(Clone, Copy)]
pub(crate) struct PurityCtx<'a> {
    pub(crate) registry: Option<&'a CommandRegistry>,
    pub(crate) interproc_pure: &'a HashSet<crate::ir::ProcedureId>,
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
        // observable effect, so deleting it is OK when v is dead and the
        // optional amount word is side-effect-free.
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
/// guards raises; so does `expr` arithmetic on a bad operand (`1/0`, `abc + 1`),
/// and `incr` of a value that is no integer, of an array, or of an absent
/// place under 8.4. O109, O126 and O108 deleted those statements and let the
/// program run on (#2249). The proof:
///
/// * a statement the solver proved raises where a handler is thrown to
///   ([`crate::sccp::SccpResult::raised`]) never qualifies: its effect is
///   the raise, and the handler runs on without its later statements;
/// * a literal (`AssignConst`) cannot raise;
/// * a value SCCP folds to a constant evaluated cleanly qualifies: SCCP reads
///   an undefined, traced or escaping name as overdefined, and a definition
///   a raise left holding the value before it is preserved
///   ([`crate::sccp::SccpResult::preserved`]), which is no clean evaluation;
/// * otherwise a word value (`AssignValue`) qualifies when the existence rung
///   holds every variable it reads bound as a scalar where the statement
///   reads it ([`crate::sccp::SccpResult::existence_before`]) and it has no
///   command substitution, or only calls the summary proves complete
///   ([`RaiseProof::calls_complete`]). A command substitution may raise on
///   its own words whatever they read (`[lindex {a b} 1.5]`), and purity
///   says a call changes nothing, not that it completes, so a call needs the
///   clean fold or a procedure that completes whatever its arguments
///   hold; an `expr` value's operators may raise on a bound operand, so it
///   needs the clean fold;
/// * an `incr` qualifies when its amount is an integer literal and its place
///   is a scalar that holds an integer wherever it is bound, and is bound
///   where the statement reads it unless every release the profile names
///   creates an absent cell.
struct RaiseProof<'a> {
    fu: &'a FunctionUnit,
    source_unit: Option<&'a CompilationUnit>,
    source: &'a str,
    source_config: Option<tcl_lexer::LexerConfig>,
    /// Names a scope alias binds, which another frame may unset.
    scope_aliases: HashSet<String>,
    /// Names a module-wide trace guards; a read trace may raise.
    module_traced: Option<&'a std::collections::BTreeSet<String>>,
    /// Alias/effect availability belongs to this exact supplied source input.
    metadata_available: bool,
    /// Ordinary source SSA names can assist only the retained Logical model.
    logical_source_assistance: bool,
    /// Whether every release the profile names creates the cell an `incr`
    /// of an absent place reads (8.5 onwards); 8.4 raises `can't read`.
    incr_creates_absent: bool,
    /// What proves a value's calls complete ([`Self::calls_complete`]).
    calls: CallCompletion<'a>,
}

/// What [`RaiseProof::calls_complete`] reads of the module: the procedures'
/// summaries and how their names stand.
struct CallCompletion<'a> {
    /// Each procedure's summary, by qualified name.
    procedures: &'a HashMap<crate::ir::ProcedureId, crate::interprocedural::ProcSummary>,
    /// The procedures the module defines more than once.
    redefined: Option<&'a HashSet<String>>,
    /// The module's command bindings: a renamed or aliased procedure's name
    /// denotes another command.
    mutations: &'a crate::command_binding::ModuleCommandMutations,
    /// Whether the module traces a command, whose callback may raise.
    traces_commands: bool,
    /// Same original Module/registry required by conditional call receipts.
    module: Option<&'a crate::ir::Module>,
    registry: &'a CommandRegistry,
    /// Which procedures are surely defined where the function's calls run.
    defined: Defined<'a>,
}

/// Which procedures are surely defined where a function's calls run.
enum Defined<'a> {
    /// A procedure's body: the callees its summary proves defined before the
    /// load may first run it.
    Callees(&'a [crate::ir::ProcedureId]),
    /// The load's own statements: a callee whose `proc` statement surely ran
    /// before the call's.
    Before,
    /// Nothing is proved: a method body, or a body a procedure runs.
    Nothing,
}

impl<'a> Defined<'a> {
    /// What `fu` can take as defined, read from the module and the summaries.
    fn of(
        ctx: &'a PassContext<'_>,
        fu: &FunctionUnit,
        runtime_namespace: bool,
        is_top_level: bool,
    ) -> Self {
        if runtime_namespace {
            return Self::Nothing;
        }
        if is_top_level {
            return Self::Before;
        }
        if let Some(module) = ctx.ir_module
            && let Some(id) = fu.body_id
            && let Some(unit) = module.body_lowering(&id)
        {
            let run_by_a_procedure = module.procedures.values().any(|procedure| {
                procedure.span.start() <= unit.span.start()
                    && unit.span.end() <= procedure.span.end()
            });
            return if run_by_a_procedure {
                Self::Nothing
            } else {
                Self::Before
            };
        }
        fu.body_id
            .and_then(|id| ctx.interproc.procedures.get(&id))
            .map_or(Self::Nothing, |summary| {
                Self::Callees(&summary.defined_callees)
            })
    }

    /// Whether `summary`'s procedure is surely defined where a call at
    /// `site` runs.
    fn admits(
        &self,
        id: &crate::ir::ProcedureId,
        summary: &crate::interprocedural::ProcSummary,
        site: u32,
    ) -> bool {
        match self {
            Self::Callees(callees) => callees.contains(id),
            Self::Before => summary.defined_at.is_some_and(|at| at < site),
            Self::Nothing => false,
        }
    }
}

impl CallCompletion<'_> {
    /// Whether a call to `qname` with `words` words after its head, made by
    /// the statement at `site`, cannot raise: the summary proves the
    /// procedure pure and completing, its parameters accept the count, its
    /// definition surely ran, and its name stands for it.
    fn completes(&self, qname: &crate::ir::ProcedureId, words: usize, site: u32) -> bool {
        self.procedures.get(qname).is_some_and(|summary| {
            summary.pure
                && summary.completes
                && self.defined.admits(qname, summary, site)
                && self
                    .module
                    .and_then(|module| {
                        crate::registry_invocation::original_procedure_formal_count_shape(
                            module,
                            module.procedures.get(qname)?,
                            self.registry,
                        )
                    })
                    .is_some_and(|shape| shape.accepts(words))
        }) && self.procedures.get(qname).is_some_and(|summary| {
            !self
                .redefined
                .is_some_and(|redefined| redefined.contains(&summary.qualified_name))
                && self.mutations.trusts_proc_binding(&summary.qualified_name)
        })
    }
}

impl<'a> RaiseProof<'a> {
    fn new(
        ctx: &'a PassContext<'_>,
        fu: &'a FunctionUnit,
        execution_namespace: Option<&crate::ir::ExecutionNamespace>,
        is_top_level: bool,
    ) -> Self {
        // Alias recognition is registry-driven; a registry-less context (unit
        // tests) falls back to the cached default.
        let registry = ctx.registry.unwrap_or_else(|| {
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands()
        });
        let metadata = ctx.ir_module.map_or_else(
            || fu.invocation_metadata_context(registry),
            |module| fu.invocation_metadata_context_for_module(registry, module),
        );
        let logical_source_assistance = metadata.is_some()
            && ctx.ir_module.is_some_and(|module| {
                crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)
                    .and_then(|metadata| {
                        module.original_invocation_source_purpose(registry, metadata)
                    })
                    == Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical)
            });
        Self {
            fu,
            source_unit: None,
            source: ctx.source,
            source_config: ctx.source_lexer_config,
            metadata_available: metadata.is_some(),
            logical_source_assistance,
            scope_aliases: scan_scope_aliases_with_metadata_context(&fu.cfg, registry, metadata),
            module_traced: ctx.ir_module.map(|m| &m.traced_variables),
            incr_creates_absent: ctx
                .registry
                .is_some_and(crate::value_transfer::typed_incr_creates_absent),
            calls: CallCompletion {
                procedures: &ctx.interproc.procedures,
                redefined: ctx.ir_module.map(|m| &m.redefined_procedures),
                mutations: &ctx.command_mutations,
                traces_commands: ctx
                    .ir_module
                    .is_some_and(|m| !m.traced_commands.is_empty() || m.has_dynamic_trace),
                module: ctx.ir_module,
                registry,
                defined: Defined::of(ctx, fu, execution_namespace.is_some(), is_top_level),
            },
        }
    }

    fn with_source_unit(mut self, unit: &'a CompilationUnit) -> Self {
        if self.source == unit.source
            && self
                .source_config
                .is_none_or(|config| config == unit.ir_module.lexer_config)
            && self
                .calls
                .module
                .is_none_or(|module| std::ptr::eq(module, &raw const unit.ir_module))
        {
            self.source_unit = Some(unit);
        }
        self
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
            || self.module_traced.is_some_and(|t| {
                crate::command_binding::trace_registrations::name_may_be_observed(
                    t,
                    base,
                    false,
                    crate::command_binding::trace_registrations::ObservationPolicy::Module,
                )
            })
    }

    /// Whether the statement at `idx` of `block`, whose def is `def`, can be
    /// deleted without losing an error its value would raise.
    fn value_cannot_raise(
        &self,
        block: crate::cfg::BlockId,
        idx: usize,
        stmt: &Statement,
        def: &(tcl_names::variable::VariableCellKey, u32),
    ) -> bool {
        if !self.metadata_available {
            return false;
        }
        if self.fu.sccp.raised.contains(&(block, idx)) {
            return false;
        }
        let folded = || {
            self.fu.ssa.cell_symbol(&def.0).is_some_and(|sym| {
                !self.fu.sccp.preserved.contains_key(&(sym, def.1))
                    && matches!(
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
            // scalar or lacks the element, and a command substitution on its
            // own words, neither of which the reads' existence can show.
            Statement::AssignValue { .. } => {
                folded()
                    || stmt.tokens().is_some_and(|tokens| {
                        tokens.source_binding.as_ref().is_some_and(|binding| {
                            binding.original_arguments_complete_normally(tokens)
                        })
                    })
                    || (self.logical_source_assistance
                        && self.calls_complete(stmt)
                        && self.reads_are_set(block, idx))
            }
            Statement::Incr { amount, .. } => {
                folded() || self.incr_cannot_raise(block, idx, amount.as_deref())
            }
            _ => folded(),
        }
    }

    /// The original value word's lexical reads and authentic child calls.
    /// Procedure targets join the same Logical Module's original allocation,
    /// header and effective argument vector. Supplied summaries, definition reach,
    /// purity and observer guards remain independent; variable existence is
    /// checked by `reads_are_set` before any deletion proof consumes this result.
    fn calls_complete(&self, stmt: &Statement) -> bool {
        let Statement::AssignValue {
            tokens: Some(tokens),
            ..
        } = stmt
        else {
            return false;
        };
        let [_, _, value] = tokens.word_exprs.as_slice() else {
            return false;
        };
        let calls = &self.calls;
        if calls.traces_commands {
            return false;
        }
        let Some(module) = calls.module else {
            return false;
        };
        let Some(mut walk) =
            crate::interprocedural::CompletionWalk::for_module(module, calls.registry, false)
        else {
            return false;
        };
        let site = stmt.span().start();
        walk.word_in(tokens, value)
            && walk
                .calls
                .iter()
                .all(|(qname, words)| calls.completes(qname, *words, site))
    }

    /// Whether every variable the statement substitutes is bound as a scalar
    /// where it reads it.
    fn reads_are_set(&self, block: crate::cfg::BlockId, idx: usize) -> bool {
        let Some(ssa_stmt) = self.statement(block, idx) else {
            return false;
        };
        ssa_stmt
            .uses
            .keys()
            .filter(|sym| !ssa_stmt.quoted_uses.contains(sym))
            .all(|&sym| {
                !self.observed(self.fu.ssa.var_name(sym))
                    && self.fu.sccp.existence_before(block, idx, sym)
                        == Some(Existence::Bound(BindingKind::Scalar))
            })
    }

    /// Whether `incr place ?amount?` at `idx` of `block` completes: an
    /// integer literal amount, a scalar place unseen code cannot guard, which
    /// holds an integer wherever it is bound, and a place bound where the
    /// statement reads it unless the profile's releases all create an absent
    /// one. An element's base may be a scalar, which the element's fact cannot
    /// show, so an element is never proved.
    fn incr_cannot_raise(
        &self,
        block: crate::cfg::BlockId,
        idx: usize,
        amount: Option<&str>,
    ) -> bool {
        let Some((sym, source_name)) = self.original_incr_target(block, idx) else {
            return false;
        };
        // Tcl literal receiver values have no substitution sigil to strip.
        // Elements remain excluded independently of their root's scalar facts.
        if tcl_syntax::naming::split_element_ref_bytes(&source_name).is_some() {
            return false;
        }
        let Ok(place) = std::str::from_utf8(&source_name) else {
            return false;
        };
        if self.observed(place) {
            return false;
        }
        if amount.is_some_and(|amount| {
            tcl_registry::value_transfer::ExactValue::from_literal(amount)
                .as_int()
                .is_none()
        }) {
            return false;
        }
        let holds_integer = || {
            self.statement(block, idx)
                .and_then(|ssa_stmt| ssa_stmt.uses.get(&sym))
                .is_some_and(|&ver| self.integer_where_bound(sym, ver, &mut HashSet::new()))
        };
        match self.fu.sccp.existence_before(block, idx, sym) {
            Some(Existence::Unbound) => self.incr_creates_absent,
            Some(Existence::MayBound) => self.incr_creates_absent && holds_integer(),
            Some(Existence::Bound(BindingKind::Scalar)) => holds_integer(),
            _ => false,
        }
    }

    /// Select the literal receiver from the current source-definition facade,
    /// not a reference-normalised name or displayed SSA label.
    fn original_incr_target(
        &self,
        block: crate::cfg::BlockId,
        index: usize,
    ) -> Option<(crate::ssa::Symbol, Vec<u8>)> {
        let unit = self.source_unit?;
        let statement = self.statement(block, index)?;
        let tokens =
            crate::ssa::SsaSourceView::at_statement(&self.fu.ssa, block, index).source_tokens()?;
        let mut original = tokens.clone();
        crate::lattice_rebase::rebase_command_tokens(&mut original, self.fu.base_offset);
        let mut selected = None;
        for word in original.words() {
            let Some(span) = word.original_written_span(unit.ir_module.source.bytes()) else {
                continue;
            };
            let Some(definition) =
                crate::ssa::original_analysis_definition_at_unit(unit, span, self.calls.registry)
            else {
                continue;
            };
            if !std::ptr::eq(definition.function(), self.fu)
                || statement.defs.get(&definition.symbol()) != Some(&definition.version())
                || statement.may_defs.contains(&definition.symbol())
            {
                continue;
            }
            let candidate = (definition.symbol(), definition.source_name_bytes().to_vec());
            if selected.as_ref().is_some_and(|old| old != &candidate) {
                return None;
            }
            selected = Some(candidate);
        }
        selected
    }

    /// Whether the version `(sym, ver)` holds an integer wherever the place
    /// is bound: a constant integer, a φ every incoming value of which does,
    /// or the entry value of a place nothing binds on entry. A φ cycle adds no
    /// value of its own, so a revisited value counts; any other value reaches
    /// the cycle through another φ operand.
    fn integer_where_bound(
        &self,
        sym: crate::ssa::Symbol,
        ver: crate::ssa::Version,
        visiting: &mut HashSet<crate::ssa::ValueKey>,
    ) -> bool {
        use crate::analyses::{ConstValue, LatticeValue};
        if !visiting.insert((sym, ver)) {
            return true;
        }
        match self.fu.sccp.values.get(&(sym, ver)) {
            Some(LatticeValue::Const(value)) => {
                return matches!(value, ConstValue::Number(NumericValue::Int(_)));
            }
            Some(LatticeValue::ConstSet(values)) => {
                return values
                    .iter()
                    .all(|value| matches!(value, ConstValue::Number(NumericValue::Int(_))));
            }
            _ => {}
        }
        if ver == 0 {
            return self.fu.sccp.existence.get(&(sym, 0)) == Some(&Existence::Unbound);
        }
        let incoming: Option<Vec<crate::ssa::Version>> =
            self.fu.ssa.blocks.values().find_map(|b| {
                b.phis
                    .iter()
                    .find(|phi| phi.name == sym && phi.version == ver)
                    .map(|phi| phi.incoming.values().copied().collect())
            });
        incoming.is_some_and(|incoming| {
            incoming
                .into_iter()
                .all(|v| self.integer_where_bound(sym, v, visiting))
        })
    }

    /// The SSA statement at `idx` of `block`.
    fn statement(
        &self,
        block: crate::cfg::BlockId,
        idx: usize,
    ) -> Option<&'a crate::ssa::SsaStatement> {
        self.fu
            .ssa
            .blocks
            .get(&block)
            .and_then(|b| b.statements.get(idx))
    }
}

/// Whether a word runs a command substitution: an unescaped `[`. A bracket
/// inside a braced part of the word is text, which a false positive only
/// treats as a command and so keeps a store.
/// Collect the qualified names of procs / methods that interprocedural
/// analysis has proven pure — threaded into the O109 / O126 RHS-purity
/// gates so `set unused [pureProc]` / `set unused [my pureMethod]` can
/// fold.
fn pure_call_targets(ctx: &PassContext<'_>) -> HashSet<crate::ir::ProcedureId> {
    ctx.interproc
        .procedures
        .iter()
        .filter(|(_, summary)| summary.pure)
        .map(|(name, _)| *name)
        .collect()
}

/// Run the elimination pass — emits O107, O108, O109, O126
/// across every function in `cu`.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    let is_top_level = |fu: &FunctionUnit| std::ptr::eq(fu, &raw const cu.top_level);
    // O109/O126 closure: the user-proc / TclOO-method purity sets
    // that gate RHS-side-effect-safe deletion (owned so the `&mut ctx`
    // calls below don't alias `ctx.interproc`).
    let interproc_pure = pure_call_targets(ctx);
    let proc_index = crate::interprocedural::build_proc_index_from_summaries(&ctx.interproc);

    if deep_analysis_available(&cu.top_level) {
        emit_unreachable(ctx, &cu.top_level);
        let purity = PurityCtx {
            registry: ctx.registry,
            interproc_pure: &interproc_pure,
            config: cu.top_level.source_lexer_config(),
            metadata: ctx.registry.and_then(|registry| {
                cu.top_level
                    .invocation_metadata_context_for_module(registry, &cu.ir_module)
            }),
            module: Some(&cu.ir_module),
        };
        let baseline = emit_dead_stores_and_unused(
            cu,
            ctx,
            &cu.top_level,
            is_top_level(&cu.top_level),
            purity,
            None,
            &proc_index,
        );
        emit_adce(cu, ctx, &cu.top_level, &baseline, purity, true, None);
    }

    // `manager::build_pass_context` populates this shared safety fact once,
    // before every pass runs. Retain the event-only projection here so plain
    // procedures with an equal local spelling are not needlessly suppressed.
    let saved_proc_cross = std::mem::take(&mut ctx.cross_event_vars);
    for (qname, fu) in &cu.procedures {
        if !deep_analysis_available(fu) {
            continue;
        }
        ctx.cross_event_vars = if ctx.registry.is_some_and(|registry| {
            cu.ir_module.procedures.get(qname).is_some_and(|procedure| {
                crate::source_graph::event_body_for_procedure(
                    &cu.ir_module,
                    qname,
                    procedure,
                    registry,
                )
                .is_some()
            })
        }) {
            cu.connection_scope
                .as_ref()
                .map_or_default(|scope| scope.handler_source_names(qname, false))
        } else {
            std::collections::HashSet::new()
        };
        emit_unreachable(ctx, fu);
        let purity = PurityCtx {
            registry: ctx.registry,
            interproc_pure: &interproc_pure,
            config: fu.source_lexer_config(),
            metadata: ctx.registry.and_then(|registry| {
                fu.invocation_metadata_context_for_module(registry, &cu.ir_module)
            }),
            module: Some(&cu.ir_module),
        };
        let baseline = emit_dead_stores_and_unused(cu, ctx, fu, false, purity, None, &proc_index);
        emit_adce(cu, ctx, fu, &baseline, purity, false, None);
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
        let execution_namespace = ir_method.map(|m| &m.execution_namespace);
        ctx.cross_event_vars = ir_method.map_or_default(|m| m.instance_vars.clone());
        emit_unreachable(ctx, fu);
        let purity = PurityCtx {
            registry: ctx.registry,
            interproc_pure: &interproc_pure,
            config: fu.source_lexer_config(),
            metadata: ctx.registry.and_then(|registry| {
                fu.invocation_metadata_context_for_module(registry, &cu.ir_module)
            }),
            module: Some(&cu.ir_module),
        };
        let baseline = emit_dead_stores_and_unused(
            cu,
            ctx,
            fu,
            false,
            purity,
            execution_namespace,
            &proc_index,
        );
        emit_adce(cu, ctx, fu, &baseline, purity, false, execution_namespace);
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
    let live = live_spans(&fu.cfg, &unreachable);
    // cfg_order is deterministic (RPO + trailing unreachables).
    for block_id in cfg_order(&fu.cfg) {
        if !unreachable.contains(&block_id) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&block_id) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
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
            // A statement whose span holds code that runs is not dead code
            // of its own: the binding of a `try` handler's variables carries
            // the span of the whole `try`, and a handler that never runs —
            // one an earlier handler pre-empts, or a `-` handler — leaves the
            // `try` that does.
            if encloses_live_code(statement.span(), &live) {
                continue;
            }
            let Some(rewrite_span) = original_statement_rewrite_span(ctx, span) else {
                continue;
            };
            ctx.report(Optimisation::new(
                DiagCode::O107,
                "Eliminate unreachable dead code",
                rewrite_span,
                "",
            ));
        }
    }
}

/// The spans of what the reachable blocks run — their statements and their
/// terminators — sorted by where they start.
fn live_spans(
    cfg: &CfgFunction,
    unreachable: &HashSet<crate::cfg::BlockId>,
) -> Vec<tcl_lexer::Span> {
    let mut spans: Vec<tcl_lexer::Span> = cfg
        .blocks
        .iter()
        .filter(|(id, _)| !unreachable.contains(*id))
        .flat_map(|(_, block)| {
            block
                .statements
                .iter()
                .map(Statement::span)
                .chain(block.terminator.as_ref().and_then(Terminator::span))
        })
        .filter(|span| !span.is_empty())
        .collect();
    spans.sort_unstable_by_key(|span| (span.start(), span.end()));
    spans
}

/// Whether `span` holds the whole of one of the `live` spans (sorted by
/// start).
fn encloses_live_code(span: tcl_lexer::Span, live: &[tcl_lexer::Span]) -> bool {
    let first = live.partition_point(|other| other.start() < span.start());
    live[first..]
        .iter()
        .take_while(|other| other.start() < span.end())
        .any(|other| other.end() <= span.end())
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
    /// Retained owning body identity, independent of its display name.
    pub body_id: Option<crate::ir::ProcedureId>,
    /// Whether the actual compilation root produced this row.
    pub is_top_level: bool,
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
    key: (tcl_names::variable::VariableCellKey, u32),
    block: String,
    statement_index: i32,
}

/// Emit O109 (dead store) + O126 (unused variable) for each
/// dead SSA def in `fu.def_use`. Returns the set of SSA value
/// keys that were reported — the ADCE pass uses it as the
/// "already eliminated" seed for its fixpoint.
/// The runtime consumes a write to `tcl_precision` (8.x), `auto_path`,
/// `env` and kin even when the script never reads it back.
fn store_is_externally_read(
    unit: &CompilationUnit,
    ctx: &PassContext<'_>,
    fu: &FunctionUnit,
    chain: &crate::def_use::DefUseChain,
) -> bool {
    fu.def_use
        .symbol(&fu.ssa, chain.key.0)
        .and_then(|symbol| fu.ssa.cell_key(symbol))
        .is_some_and(|cell| {
            crate::analyser::diagnostics::writes_externally_read_special_variable(
                fu,
                &chain.definition,
                cell,
                unit.ir_module.resolved_registry(),
                ctx.dialect.map(tcl_dialect::DialectProfile::surface_query),
            )
        })
}

fn emit_dead_stores_and_unused(
    unit: &CompilationUnit,
    ctx: &mut PassContext<'_>,
    fu: &FunctionUnit,
    is_top_level: bool,
    purity: PurityCtx<'_>,
    execution_namespace: Option<&crate::ir::ExecutionNamespace>,
    proc_index: &crate::interprocedural::ProcIndex,
) -> HashSet<(tcl_names::variable::VariableCellKey, u32)> {
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
    let raise_proof =
        RaiseProof::new(ctx, fu, execution_namespace, is_top_level).with_source_unit(unit);
    let call_by_name = crate::interprocedural::collect_call_by_name_reads(&fu.cfg, proc_index);
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
        let Some(physical_key) = fu.def_use.physical_value_key(&fu.ssa, chain.key) else {
            continue;
        };
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
            || !raise_proof.value_cannot_raise(def_block, idx, stmt, &physical_key)
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

        let Some(var) = dead_store_candidate_name(fu, chain) else {
            continue;
        };
        if dead_store_alias_observed(fu, chain, var, &raise_proof.scope_aliases, &call_by_name) {
            continue;
        }
        if dead_store_name_observed(ctx, var, module_traced, is_top_level, &bound_writes) {
            continue;
        }
        if store_is_externally_read(unit, ctx, fu, chain) {
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
            key: physical_key,
            block: chain.definition.block.clone(),
            statement_index: chain.definition.statement_index,
        });
    }

    emit_dse_entries(ctx, fu, is_top_level, entries)
}

fn dead_store_candidate_name<'a>(
    fu: &'a FunctionUnit,
    chain: &crate::def_use::DefUseChain,
) -> Option<&'a str> {
    let (cell_name, _) = &chain.key;
    let symbol = fu.def_use.symbol(&fu.ssa, *cell_name)?;
    let cell_name = fu.ssa.cell_key(symbol)?;
    // A synthetic may-def (base refresh / element fan) is not a write
    // the user made — never an O109/O126 candidate.
    if fu.ssa.is_synthetic_def(
        &chain.definition.block,
        chain.definition.statement_index,
        cell_name,
    ) {
        return None;
    }
    Some(fu.ssa.var_name(symbol))
}

fn dead_store_alias_observed(
    function: &FunctionUnit,
    chain: &crate::def_use::DefUseChain,
    variable: &str,
    scope_aliases: &HashSet<String>,
    call_by_name: &HashSet<String>,
) -> bool {
    let base = tcl_syntax::naming::split_element_ref(variable).map_or(variable, |(root, _)| root);
    store_is_seen_elsewhere(function, chain, scope_aliases)
        || call_by_name.contains(variable)
        || call_by_name.contains(base)
        || function.cfg.alias_observed_vars.contains(variable)
        || function.cfg.alias_observed_vars.contains(base)
}

fn observed_element_stores(
    ctx: &PassContext<'_>,
    fu: &FunctionUnit,
    contexts: &crate::variable_bindings::PointResolveContexts,
) -> HashSet<(String, i32)> {
    ctx.registry.map_or_default(|registry| {
        crate::place_bridge::element_writes_observed_with_contexts(&fu.cfg, contexts, registry)
    })
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
    if has_running_use(fu, chain) || chain.definition.kind != DefKind::Statement {
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
    writes: &[tcl_names::place::Place],
    contexts: &crate::variable_bindings::PointResolveContexts,
    registry: &CommandRegistry,
) -> bool {
    writes.iter().any(|written| {
        written.dynamic
            || written.observed
            || written.kind == tcl_names::place::PlaceKind::Unknown
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
    writes: &[tcl_names::place::Place],
) -> bool {
    // The SSA name is decoded literal naming data, not a `$` reference.
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let base = crate::naming::split_element_ref(var).map_or(var, |(root, _)| root);
    writes.iter().any(|written| {
        written.kind == tcl_names::place::PlaceKind::UpvarAlias
            || written.kind == tcl_names::place::PlaceKind::InstanceVar
            || (!is_top_level && written.ns != tcl_names::place::LOCAL_NS)
    }) || module_traced.is_some_and(|names| {
        crate::command_binding::trace_registrations::name_may_be_observed(
            names,
            base,
            false,
            crate::command_binding::trace_registrations::ObservationPolicy::Module,
        )
    }) || tcl_syntax::naming::is_absolute(var)
        || ctx.cross_event_vars.contains(var)
        || ctx.cross_event_vars.contains(base)
        // A callback runs at the global frame, so at the top level a name it
        // reads is this store's variable: `set x 5; after 0 {puts $x};
        // update; set x 6` prints 5 (#2308).
        || is_top_level
            && ctx.ir_module.is_some_and(|module| {
                module.deferred_writes.reads_any
                    || crate::command_binding::trace_registrations::name_may_be_observed(
                        &module.deferred_writes.reads,
                        base,
                        false,
                        crate::command_binding::trace_registrations::ObservationPolicy::Module,
                    )
            })
}

/// Whether the store `chain` defines is one the name-level SSA cannot call
/// dead: a synthetic may-def (base refresh / element fan) is no write the user
/// made; a scope alias is visible in other scopes (policy sets hold *base*
/// names, so an element symbol `a(k)` checks its base too); and a call to a
/// command the module cannot see may read the name the store leaves, as it may
/// read a `::`-qualified one.
pub(super) fn store_is_seen_elsewhere(
    fu: &FunctionUnit,
    chain: &crate::def_use::DefUseChain,
    scope_aliases: &HashSet<String>,
) -> bool {
    let (cell, version) = &chain.key;
    let Some(symbol) = fu.def_use.symbol(&fu.ssa, *cell) else {
        return true;
    };
    let var = fu.ssa.var_name(symbol);
    let var_base = tcl_syntax::naming::split_element_ref(var).map_or(var, |(root, _)| root);
    let Some(cell) = fu.ssa.cell_key(symbol) else {
        return true;
    };
    fu.ssa.is_synthetic_def(
        &chain.definition.block,
        chain.definition.statement_index,
        cell,
    ) || scope_aliases.contains(var)
        || scope_aliases.contains(var_base)
        || fu.ssa.is_observed_by_unseen_call(symbol, *version)
}

/// Whether `chain` has a use that can run. A use is a read of the value, save
/// one a φ takes over the edge from the block before a `catch` or `try` body
/// that the solver left closed: that edge carries the state before the body
/// to the handler only where the body's first command can fail before it
/// stores, and the solver opens it unless the command certainly raises after
/// a store (the prefix rule), so a store the command overwrites on every path
/// is read by no handler. Every other edge counts whether or not it runs. A
/// store a later definition preserves is read wherever that definition is
/// ([`preserved_and_read`]).
fn has_running_use(fu: &FunctionUnit, chain: &crate::def_use::DefUseChain) -> bool {
    preserved_and_read(fu, chain)
        || chain.uses.iter().any(|site| {
            site.kind != crate::def_use::UseKind::PhiIncoming || !over_a_closed_entry(fu, site)
        })
}

/// Whether a definition that left `chain`'s place as it was — a command that
/// raised before it reached the place, or whose outcome preserves it — is
/// read. Such a definition holds the store before it, though no use links
/// them: `lassign {x y z} a b c` raising at an array `b` leaves `c` as it was,
/// in a `try` body and in `[catch {…}]` alike, so `set c old` before it is
/// what a later read of `c` sees.
fn preserved_and_read(fu: &FunctionUnit, chain: &crate::def_use::DefUseChain) -> bool {
    let Some(symbol) = fu.def_use.symbol(&fu.ssa, chain.key.0) else {
        return false;
    };
    fu.sccp.preserved.iter().any(|(&(var, version), &prior)| {
        var == symbol
            && prior == chain.key.1
            && fu
                .def_use
                .chain_for(chain.key.0, version)
                .is_some_and(|keeper| !keeper.is_dead())
    })
}

/// Whether the φ-incoming `site` arrives only over region entries the solver
/// left closed ([`has_running_use`]): every edge from its predecessor into a
/// block whose φ takes the value is the edge from the block before a body to
/// its handler, and none of them is executable.
fn over_a_closed_entry(fu: &FunctionUnit, site: &crate::def_use::UseSite) -> bool {
    let (Some(pred), Some(symbol)) = (
        fu.cfg.block_id(&site.block),
        fu.def_use.symbol(&fu.ssa, site.variable),
    ) else {
        return false;
    };
    let targets: Vec<crate::cfg::BlockId> = fu
        .ssa
        .blocks
        .iter()
        .filter(|(_, block)| {
            block
                .phis
                .iter()
                .any(|phi| phi.name == symbol && phi.version == site.phi_version)
        })
        .map(|(&target, _)| target)
        .collect();
    !targets.is_empty()
        && targets.iter().all(|&target| {
            fu.cfg
                .region_entries
                .iter()
                .any(|entry| entry.source == pred && entry.handler == target)
                && !fu.sccp.executable_edges.contains(&(pred, target))
        })
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
    if let Some(symbol) = fu.def_use.symbol(&fu.ssa, *var)
        && fu.def_use.chains.iter().any(|(key, consumer)| {
            key.0 == *var
                && key.1 != chain.key.1
                && has_running_use(fu, consumer)
                && fu.ssa.binding_version(symbol, key.1) == chain.key.1
        })
    {
        return None;
    }
    let any_other_live = fu
        .def_use
        .chains
        .iter()
        .any(|(k, c)| k.0 == *var && k.1 != chain.key.1 && has_running_use(fu, c));
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
        .def_use
        .symbol(&fu.ssa, *var)
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
    is_top_level: bool,
    mut entries: Vec<DseEntry>,
) -> HashSet<(tcl_names::variable::VariableCellKey, u32)> {
    entries.sort_by_key(|e| e.span.start());
    let mut removed: HashSet<(tcl_names::variable::VariableCellKey, u32)> = HashSet::new();
    for e in entries {
        let Some(rewrite_span) = original_statement_rewrite_span(ctx, e.span) else {
            continue;
        };
        let rewrite_span = line_delete_span(ctx.source, rewrite_span);
        // Record O109 dead stores (not O126 unused vars) so tools can show
        // them from where Rust determines them. `run` collects these into
        // `ctx.dead_stores`.
        if e.code == DiagCode::O109 {
            ctx.dead_stores.push(DeadStore {
                function: fu.name.clone(),
                body_id: fu.body_id,
                is_top_level,
                block: e.block.clone(),
                statement_index: e.statement_index,
                variable: fu.ssa.cell_symbol(&e.key.0).map_or_else(
                    || e.key.0.compatibility_name(),
                    |symbol| fu.ssa.var_name(symbol).to_owned(),
                ),
                version: e.key.1,
            });
        }
        ctx.report(Optimisation::new(e.code, e.msg, rewrite_span, ""));
        removed.insert(e.key);
    }
    removed
}

/// Emit **O108** (transitively dead code) — the ADCE fixpoint.
fn emit_adce(
    unit: &CompilationUnit,
    ctx: &mut PassContext<'_>,
    fu: &FunctionUnit,
    baseline: &HashSet<(tcl_names::variable::VariableCellKey, u32)>,
    purity: PurityCtx<'_>,
    top_level: bool,
    execution_namespace: Option<&crate::ir::ExecutionNamespace>,
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
    let raise_proof =
        RaiseProof::new(ctx, fu, execution_namespace, top_level).with_source_unit(unit);
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
) -> HashSet<(tcl_names::variable::VariableCellKey, u32)> {
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
            let symbol = fu.def_use.symbol(&fu.ssa, chain.key.0)?;
            (dead_store_writes_observed(fu, block, index, &writes, contexts, registry)
                || dead_store_name_observed(
                    ctx,
                    fu.ssa.var_name(symbol),
                    ctx.ir_module.map(|module| &module.traced_variables),
                    top_level,
                    &writes,
                ))
            .then(|| fu.def_use.physical_value_key(&fu.ssa, chain.key))
            .flatten()
        })
        .collect()
}

type ConsumerMap = HashMap<(tcl_names::variable::VariableCellKey, u32), Vec<(String, usize)>>;

fn build_adce_consumers(
    fu: &FunctionUnit,
) -> (
    ConsumerMap,
    HashSet<(tcl_names::variable::VariableCellKey, u32)>,
) {
    use crate::def_use::UseKind;
    let mut consumer_stmt_keys: ConsumerMap = HashMap::new();
    let mut keep_forever: HashSet<(tcl_names::variable::VariableCellKey, u32)> = HashSet::new();
    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != DefKind::Statement {
            continue;
        }
        let Some(mut key) = fu.def_use.physical_value_key(&fu.ssa, chain.key) else {
            continue;
        };
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

type StmtDefsMap = HashMap<(String, usize), Vec<(tcl_names::variable::VariableCellKey, u32)>>;

fn build_stmt_to_defs(fu: &FunctionUnit) -> StmtDefsMap {
    let mut out: StmtDefsMap = HashMap::new();
    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != DefKind::Statement {
            continue;
        }
        if let Ok(idx) = usize::try_from(chain.definition.statement_index)
            && let Some(key) = fu.def_use.physical_value_key(&fu.ssa, chain.key)
        {
            out.entry((chain.definition.block.clone(), idx))
                .or_default()
                .push(key);
        }
    }
    out
}

fn run_adce_fixpoint(
    fu: &FunctionUnit,
    baseline: &HashSet<(tcl_names::variable::VariableCellKey, u32)>,
    consumer_stmt_keys: &ConsumerMap,
    keep_forever: &HashSet<(tcl_names::variable::VariableCellKey, u32)>,
    stmt_to_defs: &StmtDefsMap,
    effect: EffectCtx<'_>,
    raise_proof: &RaiseProof<'_>,
) -> HashSet<(tcl_names::variable::VariableCellKey, u32)> {
    let unreachable = unreachable_blocks(&fu.cfg, &fu.sccp);
    let mut removed = baseline.clone();
    loop {
        let mut changed = false;
        for chain in fu.def_use.chains.values() {
            if chain.definition.kind != DefKind::Statement {
                continue;
            }
            let Some(physical_key) = fu.def_use.physical_value_key(&fu.ssa, chain.key) else {
                continue;
            };
            let key = &physical_key;
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
                    |defs: &Vec<(tcl_names::variable::VariableCellKey, u32)>| {
                        defs.iter()
                            .all(|d: &(tcl_names::variable::VariableCellKey, u32)| {
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
    baseline: &HashSet<(tcl_names::variable::VariableCellKey, u32)>,
    removed: &HashSet<(tcl_names::variable::VariableCellKey, u32)>,
) {
    let mut new_reports: Vec<tcl_lexer::Span> = Vec::new();
    for key in removed.difference(baseline) {
        if let Some(chain) = fu.def_use.chain_for_cell(&fu.ssa, &key.0, key.1) {
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
        let Some(rewrite_span) = original_statement_rewrite_span(ctx, span) else {
            continue;
        };
        ctx.report(Optimisation::new(
            DiagCode::O108,
            "Eliminate transitively dead code",
            rewrite_span,
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
    collect_rmw_hidden_reads_outside(function, registry, &[])
}

/// [`collect_rmw_hidden_reads`] of the statements outside `deleted`.
pub(crate) fn collect_rmw_hidden_reads_outside(
    function: &FunctionUnit,
    registry: &CommandRegistry,
    deleted: &[tcl_lexer::Span],
) -> HashSet<String> {
    source_reads::original_hidden_reads_outside(
        function,
        registry,
        function.invocation_metadata_context(registry),
        deleted,
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
        .filter(|target| tcl_syntax::naming::is_qualified(target.as_bytes()))
        .collect()
}

#[cfg(test)]
mod tests;
