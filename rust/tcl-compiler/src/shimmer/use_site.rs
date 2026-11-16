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

//! Use-site shimmer detection (S100 / S101).
//!
//! A use-site shimmer occurs when a variable holds a value of type A but
//! is passed to a command argument position that expects type B.  Tcl
//! silently converts the internal representation at runtime — e.g. a
//! `String` to `Int` for `incr`, or a `String` to `List` for `llength` —
//! potentially invalidating cached intreps on shared references.
//!
//! Diagnostics:
//! - **S100**: shimmer outside a loop.
//! - **S101**: shimmer inside a loop (higher severity — converts on every
//!   iteration).
//!
//! This pass covers:
//! - [`Statement::Call`] arguments tagged `shimmers=true` in the registry.
//! - [`Statement::Incr`] — always reads its variable as `Int`.

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, TclType};

use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::naming::element_var_name;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::types::{TypeKind, TypeLattice};

use super::hints::{
    ShimmerExpectation, invocation_shimmer_expectation, is_free_first_conversion,
    is_numeric_compatible,
};
use super::span::def_range_map;
use super::{ShimmerWarning, type_name};

macro_rules! sh21_trace {
    ($($argument:tt)*) => {
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SH21_PRODUCER").is_some() {
            eprintln!($($argument)*);
        }
    };
}

fn sh21_option<T>(stage: &str, value: Option<T>) -> Option<T> {
    sh21_trace!("SH21 stage={stage} present={}", value.is_some());
    #[cfg(not(test))]
    let _ = stage;
    value
}

fn sh21_flag(stage: &str, value: bool) -> bool {
    sh21_trace!("SH21 stage={stage} value={value}");
    #[cfg(not(test))]
    let _ = stage;
    value
}

/// Find use-site shimmer warnings for a function.
///
/// Walks every executable block in CFG order, checks each statement's
/// argument words against the registry's `arg_types`, and emits a
/// [`ShimmerWarning`] for each type mismatch where the variable's known
/// type differs from what the command requires.
#[must_use]
pub(crate) fn find_use_site_shimmers_with_context(
    cfg: &CfgFunction,
    commit_ctx: &super::commit::CommitCtx<'_>,
    (executable_blocks, in_force): (
        &HashSet<BlockId>,
        &HashMap<(BlockId, ValueKey), TypeLattice>,
    ),
    facts: &super::ShimmerFacts,
) -> Vec<ShimmerWarning> {
    let super::commit::CommitCtx {
        registry,
        ssa,
        types,
        values,
        folded,
        context,
        source: _,
    } = *commit_ctx;
    let loop_blocks = &facts.loop_blocks;
    let def_map = def_range_map(ssa);
    // Loop-invariance facts for the S101→S100 downgrade — only needed when
    // the function has at least one loop block.
    let loop_facts = LoopFacts::compute(cfg, ssa, loop_blocks, context);
    // Array elements share a root symbol. Exclude that shared version chain
    // so different element values cannot appear to change one scalar's intrep.
    let array_syms = super::thunking::array_element_symbols(cfg, ssa);
    let mut out: Vec<ShimmerWarning> = Vec::new();

    for block_id in cfg_order(cfg) {
        if !executable_blocks.contains(&block_id) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&block_id) else {
            continue;
        };
        let in_loop = loop_blocks.contains(cfg.block_name(block_id));
        // Per-block coercion ledger: once a use coerces `(var, ver)` to a
        // target intrep, the runtime representation has already changed, so a
        // later use to the *same* target in the same block is not a second
        // shimmer.
        let mut already_coerced: HashSet<(Symbol, u32, TclType)> = HashSet::new();
        // The committed-intrep walker replays the commit transfer function in
        // step with this walk, so each statement's checks see the state *just
        // before* it executes.
        let mut commit_walker = facts.commit.walker(commit_ctx, block_id);
        for (index, ss) in ssa_block.statements.iter().enumerate() {
            let mut ctx = UseSiteCtx {
                types: super::BlockTypes::at(types, in_force, block_id),
                registry,
                context,
                def_map: &def_map,
                values,
                folded,
                loop_facts: &loop_facts,

                source: crate::ssa::SsaSourceView::at_statement(ssa, block_id, index),
                array_syms: &array_syms,
                commit: &commit_walker,
                in_loop,
                already_coerced: &mut already_coerced,
                out: &mut out,
            };
            check_statement(&mut ctx, &ss.statement, &ss.uses);
            commit_walker.step(&ss.statement, &ss.uses);
        }
    }

    out
}

/// Function-wide read-only context + per-block warning sinks threaded
/// through the use-site shimmer walk.  `in_loop` / `already_coerced` are
/// per-block; `out` accumulates across the whole function.
struct UseSiteCtx<'a> {
    /// The type lattice as the block reads it.
    types: super::BlockTypes<'a>,
    registry: &'a CommandRegistry,
    context: super::ShimmerContext<'a>,
    def_map: &'a HashMap<ValueKey, Span>,
    values: &'a HashMap<ValueKey, LatticeValue>,
    /// The folded types SCCP's evaluations state: a use of a computed value
    /// reads the representation its route constructed.
    folded: &'a HashMap<ValueKey, crate::value_transfer::FoldedType>,
    loop_facts: &'a LoopFacts,
    source: crate::ssa::SsaSourceView<'a>,
    /// Array-base symbols excluded from shimmer reporting (FP-SH-13) — a
    /// conflated `arr(a)`/`arr(b)` symbol can hold either element's intrep.
    array_syms: &'a HashSet<Symbol>,
    /// The committed-intrep state just before the statement being checked —
    /// tells a use whether the value already committed a different intrep on
    /// every path (a genuine second conversion, [`super::commit`]).
    commit: &'a super::commit::CommitWalker<'a>,
    in_loop: bool,
    already_coerced: &'a mut HashSet<(Symbol, u32, TclType)>,
    out: &'a mut Vec<ShimmerWarning>,
}

impl UseSiteCtx<'_> {
    /// The representation the evaluation that produced `(sym, ver)` states
    /// it constructed, when it says.
    fn representation(
        &self,
        sym: Symbol,
        ver: u32,
    ) -> tcl_registry::value_transfer::RepresentationEvidence {
        self.folded.get(&(sym, ver)).map_or(
            tcl_registry::value_transfer::RepresentationEvidence::Unknown,
            |folded| folded.representation,
        )
    }
}

/// Loop-invariance facts used to refine the in-loop shimmer classification.
///
/// A variable used inside a loop is "per-iteration" (S101) only if its
/// intrep can be reset each iteration — i.e. something in the loop body
/// defines it. A loop-*invariant* variable (no def inside any loop block)
/// shimmers once at the first iteration and is cached for the rest, so the
/// right code is S100 (one-time) — *unless* it is coerced to two or more
/// distinct target intreps inside the loop, in which case the converters
/// re-thunk it each pass (genuine S101).
#[derive(Default)]
struct LoopFacts {
    /// Cells defined anywhere in a loop block (statement defs + phis).
    def_names: HashSet<Symbol>,
    /// Per-cell set of expected intreps requested at any use-site in a
    /// loop block.
    use_targets: HashMap<Symbol, HashSet<TclType>>,
}

impl LoopFacts {
    fn compute(
        cfg: &CfgFunction,
        ssa: &SsaFunction,
        loop_blocks: &HashSet<String>,
        context: super::ShimmerContext<'_>,
    ) -> Self {
        let mut facts = Self::default();
        if loop_blocks.is_empty() {
            return facts;
        }
        for lbn in loop_blocks {
            let Some(id) = cfg.block_id(lbn) else {
                continue;
            };
            if let Some(sb) = ssa.blocks.get(&id) {
                for st in &sb.statements {
                    facts.def_names.extend(st.defs.keys().copied());
                }
                for phi in &sb.phis {
                    facts.def_names.insert(phi.name);
                }
            }
            if let Some(cb) = cfg.blocks.get(&id) {
                for (index, stmt) in cb.statements.iter().enumerate() {
                    facts.record_use_targets(
                        stmt,
                        context,
                        crate::ssa::SsaSourceView::at_statement(ssa, id, index),
                    );
                }
            }
        }
        facts
    }

    /// Record the expected intrep of every `$var` argument at a shimmering
    /// position of `stmt` — its own words, and every `[cmd …]` nested in one.
    ///
    /// This reads a statement's arguments, and it must read them by the rules
    /// the *emitting* passes use, or its answer disagrees with theirs about the
    /// same code. Two rules matter:
    ///
    /// - **Nested substitutions count.** `puts [list [llength $x]]` converts
    ///   `x` to a list exactly as `llength $x` does — Tcl runs the inner
    ///   command either way. Walking only the flat argument text sees the
    ///   single word `[list [llength $x]]`, records nothing, and leaves
    ///   `use_targets` short of the two distinct targets `effective_in_loop`
    ///   needs, downgrading a genuinely per-iteration conversion to a one-time
    ///   one. `use_site::check_lifted_calls` and `commit::push_lifted_reads`
    ///   consult the same [`crate::word_subst::lifted_calls`] owner, and so
    ///   does this.
    /// - **Braced words are inert.** A `Statement::Call`'s `args` are
    ///   *de-braced*, so `lindex {$x} 0` would look like a live `$x` here even
    ///   though Tcl substitutes nothing inside braces. This function emits
    ///   nothing, but a spurious target flips a loop-invariance judgement
    ///   between S100 and S101 just the same.
    ///
    /// Retained invocation roles decide which effective words are inert,
    /// including value prefixes. Nested reads require the checked original
    /// lifted-call inventory and their own invocation owner.
    fn record_use_targets(
        &mut self,
        stmt: &Statement,
        context: super::ShimmerContext<'_>,
        source: crate::ssa::SsaSourceView<'_>,
    ) {
        let registry = context.registry();
        if let Some(invocation) = context.statement(stmt) {
            let inert = super::hints::inert_effective_args(&invocation);
            self.record_invocation(&invocation, &inert, source, registry);
        }
        let tokens = match stmt {
            Statement::Call { tokens, .. } | Statement::AssignValue { tokens, .. } => {
                tokens.as_ref()
            }
            _ => return,
        };
        for lifted in crate::word_subst::lifted_calls(tokens, context.config()) {
            if let Some(tokens) = &lifted.tokens
                && let Some(invocation) = context.invocation(tokens)
            {
                let inert = super::hints::inert_effective_args(&invocation);
                self.record_invocation(&invocation, &inert, source, registry);
            }
        }
    }

    /// Record one invocation's shimmering `$var` arguments, skipping the
    /// argument positions in `inert`.
    fn record_invocation(
        &mut self,
        invocation: &crate::registry_invocation::NormalRepresentationInvocation,
        inert: &[usize],
        source: crate::ssa::SsaSourceView<'_>,
        registry: &CommandRegistry,
    ) {
        for (i, word) in invocation
            .effective_words()
            .words
            .iter()
            .skip(1)
            .enumerate()
        {
            if inert.contains(&i) || word.sole_variable_substitution().is_none() {
                continue;
            }
            if super::hints::operand_preserves_captured_cache(
                invocation,
                i,
                source.read_word_representation_advice(word, registry),
            ) {
                continue;
            }
            if let Some(expected) =
                invocation_shimmer_expectation(invocation, i).map(|hint| hint.expected)
                && let Some(read) = invocation
                    .effective_words()
                    .words
                    .get(i + 1)
                    .and_then(|word| source.read_word(word))
            {
                self.use_targets
                    .entry(read.symbol)
                    .or_default()
                    .insert(expected);
            }
        }
    }

    /// Refine `in_loop` for a single use of `var`: a loop-invariant variable
    /// coerced to fewer than two distinct intreps inside the loop converts
    /// once and is cached, so it is S100 (not S101).
    fn effective_in_loop(&self, var: Symbol, in_loop: bool) -> bool {
        if in_loop && !self.def_names.contains(&var) {
            self.use_targets.get(&var).is_some_and(|t| t.len() >= 2)
        } else {
            in_loop
        }
    }
}

/// Grouped, per-call arguments for [`check_invocation`] — keeps that
/// function's own parameter list short (the `ctx` + `uses` thread-through
/// state are the only params that vary independently of "which invocation").
#[derive(Clone, Copy)]
struct InvocationSite<'a> {
    /// Source spelling, for the warning's `command` field and message.
    command: &'a str,
    /// Argument words.
    invocation: &'a crate::registry_invocation::NormalRepresentationInvocation,
    /// Effective argument expressions retain each individual reference site.
    words: &'a [crate::ir::WordExpr],
    /// Per-argument absolute spans, index-aligned with `args`, when
    /// available (see `fallback_span`).
    arg_spans: &'a [Span],
    /// Argument positions proved inert after alias origin and callee-role projection.
    inert_arguments: &'a [usize],
    /// Span used for any argument index `arg_spans` doesn't cover — a
    /// synthetic call built without per-word token spans (some test
    /// fixtures) or a substitution word count `arg_spans` didn't fully
    /// resolve.
    fallback_span: Span,
    target_uses: Option<&'a HashMap<Symbol, u32>>,
}

/// Check one command invocation's arguments for an intrep mismatch
/// against the variables' known types. Used for both a top-level
/// [`Statement::Call`] and a `[cmd …]` substitution lifted out of a
/// [`Statement::AssignValue`] value (`set b [lindex $x 0]`).
///
/// Effective alias arguments and source origins are projected together by the
/// shared invocation owner before this function receives any argument indices.
fn check_invocation(ctx: &mut UseSiteCtx<'_>, site: &InvocationSite<'_>) {
    // Positions whose brace-quoted word Tcl never substitutes: their
    // de-braced `args` text spells a live `$x` that is not one.
    let inert = site.inert_arguments;
    for i in 0..site.invocation.argument_count() {
        if inert.contains(&i) && !super::hints::read_modify_write_argument(site.invocation, i) {
            continue;
        }
        check_argument(ctx, site, i);
    }
}

/// Check one argument word of an invocation for an intrep mismatch, emitting a
/// [`ShimmerWarning`] when the variable's committed type genuinely differs from
/// what the position requires. Split out of [`check_invocation`]'s argument
/// loop so each stays a single, readable responsibility.
/// Resolve an argument word to a tracked variable use with a Known lattice
/// type: the normalised name, its SSA coordinates, and the current type.
/// `None` for complex words (which may produce the right type via their own
/// evaluation), array bases (FP-SH-13 — a conflated version chain), live-in
/// version 0, and Unknown/Overdefined lattice entries.
fn resolve_tracked_var_use(
    ctx: &UseSiteCtx<'_>,
    word: &crate::ir::WordExpr,
    expected: TclType,
) -> Option<(String, Symbol, u32, super::commit::RepresentationCost)> {
    let (spelling, _) = sh21_option("tracked.sole_variable", word.sole_variable_substitution())?;
    let var = element_var_name(spelling).to_owned();
    let read = sh21_option("tracked.ssa_read", ctx.source.read_word(word))?;
    let sym = read.symbol;
    if ctx.array_syms.contains(&sym) {
        return None;
    }
    let ver = sh21_option("tracked.version", read.version)?;
    if ver == 0 {
        return None;
    }
    let lattice = ctx
        .types
        .get((sym, ver))
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    sh21_trace!(
        "SH21 stage=tracked.lattice kind={:?} expected={expected:?}",
        lattice.kind()
    );
    if lattice.kind() != TypeKind::Known {
        return None;
    }
    let current = lattice.tcl_type()?;
    let cost = sh21_option(
        "tracked.captured_cost",
        ctx.commit
            .cost_for_word(ctx.source, word, read, current, expected),
    )?;
    sh21_trace!(
        "SH21 stage=tracked.cost current={:?} expected={expected:?} commitment={:?}",
        cost.current,
        cost.commitment
    );
    Some((var, sym, ver, cost))
}

/// A variable-name operand selects its cell after argv evaluation; the
/// represented pre-write version and selected handler role remain required.
fn resolve_tracked_argument_use(
    ctx: &UseSiteCtx<'_>,
    site: &InvocationSite<'_>,
    index: usize,
    expected: TclType,
) -> Option<(String, Symbol, u32, super::commit::RepresentationCost)> {
    if !super::hints::read_modify_write_argument(site.invocation, index) {
        return resolve_tracked_var_use(ctx, site.words.get(index)?, expected);
    }
    let (name, read) = super::hints::literal_target_read(
        site.invocation,
        index,
        ctx.source,
        site.target_uses?,
        ctx.registry,
    )?;
    if ctx.array_syms.contains(&read.symbol) {
        return None;
    }
    let version = read.version?;
    let lattice = ctx.types.get((read.symbol, version))?;
    if lattice.kind() != TypeKind::Known {
        return None;
    }
    let cost = ctx
        .commit
        .cost_for_native_read(ctx.source, read, lattice.tcl_type()?, expected)?;
    Some((name, read.symbol, version, cost))
}

fn check_argument(ctx: &mut UseSiteCtx<'_>, site: &InvocationSite<'_>, i: usize) {
    sh21_trace!("SH21 stage=argument command={} index={i}", site.command);
    let expectation = sh21_option(
        "argument.expectation",
        invocation_shimmer_expectation(site.invocation, i),
    );
    let Some(expectation) = expectation else {
        return;
    };
    let expected = expectation.expected;
    let Some((var, sym, ver, cost)) = resolve_tracked_argument_use(ctx, site, i, expected) else {
        return;
    };
    if super::hints::operand_preserves_captured_cache(
        site.invocation,
        i,
        ctx.source
            .read_word_representation_advice(&site.words[i], ctx.registry),
    ) {
        return;
    }
    // The committed-intrep state just before this statement: when a prior use
    // (`expr`, `llength`, …) has committed a different intrep on **every**
    // executable path, this read genuinely re-represents even where the
    // flow-insensitive lattice sees no mismatch — `set v 5; expr {$v+1};
    // lindex $v 0` shimmers Numeric → List though the lattice types `v` Int
    // (pure literal). Resolved before the lattice-equality skip below so the
    // second conversion is not masked.
    let current = cost.current;
    let commit_state = cost.commitment;
    if commit_state.must_pay(expected)
        && !is_suppressed_committed(&expectation, expected, commit_state.single_committed())
    {
        let (from, from_label) = super::committed_from_label(&commit_state, current, expected);
        emit_use_site_warning(
            ctx,
            site,
            i,
            &UseWarning {
                var: &var,
                sym,
                ver,
                from,
                from_label,
                expected,
                related_override: commit_state
                    .first_span()
                    .map(|sp| vec![(sp, "intrep first committed here".to_owned())]),
            },
        );
        return;
    }
    if current == expected || is_numeric_compatible(current, expected) {
        return;
    }
    // The registry marks intreps this operation reads directly without
    // installing `expected` (e.g. `string length`'s pure-byte-array fast
    // path): those operands keep their rep — no shimmer.
    if expectation.is_transparent_from(current) {
        return;
    }
    // A numeric lattice type does not prove a numeric *intrep*: a
    // literal-defined `set x 42` is a pure string at runtime
    // (tclsh-verified) with nothing for a String-installing operation to
    // destroy, and when the numeric rep IS installed its string
    // regenerates in O(digits). Only container intreps (List/Dict) lose
    // real structure to a String expectation, so numeric currents stay
    // silent there.
    if expected == TclType::String
        && matches!(
            current,
            TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean
        )
    {
        return;
    }
    // An uncommitted (pure) value — a literal, an interpolated string, or a
    // string-producing command's result — pays nothing on its first read as
    // `expected` when it is a valid instance of it. `set fontSizes {10.0
    // 12.0}; foreach s $fontSizes`, `set x 5; lindex $x 0`, and
    // `set d {a 1 b 2}; dict for {k v} $d` are all free promotions of a pure
    // string, not shimmers — see [`is_uncommitted_first_conversion`].
    //
    // The one exception is a value read as **two or more distinct intreps
    // inside a loop** (`llength $l` + `dict size $l` each pass): the loop
    // entry re-joins the pure preheader path every iteration, but at runtime
    // the converters re-thunk the value on every pass after the first —
    // list↔dict oscillation verified on tclsh — so the free-first-conversion
    // suppression must not apply (the multi-target `LoopFacts` fact, which
    // also drives the S101 escalation).
    let multi_target_in_loop = ctx.in_loop
        && ctx
            .loop_facts
            .use_targets
            .get(&sym)
            .is_some_and(|targets| targets.len() >= 2);
    if !multi_target_in_loop
        && sh21_flag(
            "free_first_conversion",
            is_free_first_conversion(
                current,
                expected,
                ctx.values.get(&(sym, ver)),
                ctx.representation(sym, ver),
                ctx.commit.numbers(),
                ctx.commit.word_rules(),
            ),
        )
    {
        return;
    }
    // Prefer the steady-state committed rep for the from-type (a loop-carried
    // value whose entry re-joins the pure preheader still reads as its
    // back-edge rep on every iteration after the first).
    let (from, from_label) = super::committed_from_label(&commit_state, current, expected);
    emit_use_site_warning(
        ctx,
        site,
        i,
        &UseWarning {
            var: &var,
            sym,
            ver,
            from,
            from_label,
            expected,
            related_override: None,
        },
    );
}

/// Whether a must-pay committed conversion is still suppressed by the
/// registry's transparency list or the numeric-current-vs-String rule (a
/// committed numeric intrep survives a String read — the string rep is
/// regenerated alongside in O(digits), tclsh-verified).
fn is_suppressed_committed(
    expectation: &ShimmerExpectation,
    expected: TclType,
    committed: Option<TclType>,
) -> bool {
    let Some(committed) = committed else {
        // A multi-type committed state has no single rep to test transparency
        // against; the conversion claim stands.
        return false;
    };
    if expectation.is_transparent_from(committed) {
        return true;
    }
    expected == TclType::String
        && matches!(
            committed,
            TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean
        )
}

/// One resolved use-site warning, ready to emit: the variable, its SSA
/// coordinates, and the conversion being reported. `related_override`
/// replaces the default "value defined here" related span (the committed
/// path points at the converting use instead). `from_label` overrides the
/// message's from-type wording (the path-dependent case, where no single
/// `TclType` is truthful).
struct UseWarning<'w> {
    var: &'w str,
    sym: Symbol,
    ver: u32,
    from: TclType,
    from_label: String,
    expected: TclType,
    related_override: Option<Vec<(Span, String)>>,
}

/// Emit one use-site shimmer warning, honouring the per-block coercion ledger
/// and the loop-invariance S101→S100 downgrade.
fn emit_use_site_warning(
    ctx: &mut UseSiteCtx<'_>,
    site: &InvocationSite<'_>,
    i: usize,
    warning: &UseWarning<'_>,
) {
    // A prior use in this block already coerced `(var, ver)` to this
    // intrep — the runtime representation has already changed, so this
    // is not a second shimmer.
    let coercion_key = (warning.sym, warning.ver, warning.expected);
    if !ctx.already_coerced.insert(coercion_key) {
        return;
    }
    let related = warning.related_override.clone().unwrap_or_else(|| {
        ctx.def_map
            .get(&(warning.sym, warning.ver))
            .map_or_default(|&sp| vec![(sp, "value defined here".to_owned())])
    });
    // A loop-invariant variable coerced to a single intrep inside the
    // loop converts once and is cached → S100, not the per-iteration
    // S101.
    let code = if ctx.loop_facts.effective_in_loop(warning.sym, ctx.in_loop) {
        DiagCode::S101
    } else {
        DiagCode::S100
    };
    let span = site.arg_spans.get(i).copied().unwrap_or(site.fallback_span);
    ctx.out.push(ShimmerWarning {
        span,
        variable: warning.var.to_owned(),
        from_type: warning.from,
        to_type: warning.expected,
        command: site.command.to_owned(),
        in_loop: ctx.in_loop,
        code,
        message: format!(
            "variable '{var}' has {from} intrep \
             but '{cmd}' expects {to} (argument {n})",
            var = warning.var,
            from = warning.from_label,
            cmd = site.command,
            to = type_name(warning.expected),
            n = i + 1,
        ),
        related,
    });
}

/// Check every `[cmd …]` nested in `tokens`' words as its own invocation.
///
/// Reads the segmenter's structured
/// [`word_exprs`](crate::ir::CommandTokens::word_exprs) rather than the flat
/// argument text, because the two are indistinguishable there: `puts [lindex $x 0]`
/// and `puts {[lindex $x 0]}` both arrive as the argument text `[lindex $x 0]`,
/// and only the braced one is a literal Tcl never runs. See
/// [`crate::word_subst`].
fn check_lifted_calls(
    ctx: &mut UseSiteCtx<'_>,
    tokens: Option<&crate::ir::CommandTokens>,
    fallback_span: Span,
) {
    let config = ctx.context.config();
    for lifted in crate::word_subst::lifted_calls(tokens, config) {
        let Some(tokens) = &lifted.tokens else {
            continue;
        };
        let Some(invocation) = ctx.context.invocation(tokens) else {
            continue;
        };
        let inert = super::hints::inert_effective_args(&invocation);
        let spans: Vec<_> = invocation
            .effective_words()
            .words
            .iter()
            .skip(1)
            .map(|word| word.source().span)
            .collect();
        check_invocation(
            ctx,
            &InvocationSite {
                command: &lifted.command,
                invocation: &invocation,
                words: &invocation.effective_words().words[1..],
                arg_spans: &spans,
                inert_arguments: &inert,
                fallback_span,
                target_uses: None,
            },
        );
    }
}

fn check_statement(ctx: &mut UseSiteCtx<'_>, stmt: &Statement, uses: &HashMap<Symbol, u32>) {
    sh21_trace!(
        "SH21 stage=statement span={:?} kind={:?} uses={}",
        stmt.span(),
        std::mem::discriminant(stmt),
        uses.len()
    );
    match stmt {
        Statement::Call {
            command, tokens, ..
        } => {
            // A `[cmd …]` in one of this call's words reads its arguments
            // exactly as the same command on its own line would — Tcl runs it
            // before the outer command either way — so check it as the
            // invocation it is.
            sh21_trace!(
                "SH21 stage=generic-call command={command} span={:?}",
                stmt.span()
            );
            check_lifted_calls(ctx, tokens.as_ref(), stmt.span());
            let Some(invocation) = sh21_option("generic.invocation", ctx.context.statement(stmt))
            else {
                return;
            };
            let inert = super::hints::inert_effective_args(&invocation);
            let arg_spans: Vec<_> = invocation
                .effective_words()
                .words
                .iter()
                .skip(1)
                .map(|word| word.source().span)
                .collect();
            check_invocation(
                ctx,
                &InvocationSite {
                    command,
                    invocation: &invocation,
                    words: &invocation.effective_words().words[1..],
                    arg_spans: &arg_spans,
                    inert_arguments: &inert,
                    fallback_span: stmt.span(),
                    target_uses: Some(uses),
                },
            );
        }

        // The words of an assignment substitute exactly as a call's do —
        // `set b [lindex $x 0]` runs `lindex` before it stores anything, and
        // `set b [list [lindex $x 0]]` runs it just the same, one level down.
        // So the value word goes through the same lift as a call's words
        // one owner decides which substitutions a statement
        // runs, and the outermost `[cmd …]` is simply its depth-zero case
        // rather than a shape this arm re-derives for itself.
        Statement::AssignValue { tokens, .. } => {
            check_lifted_calls(ctx, tokens.as_ref(), stmt.span());
        }

        Statement::Incr { name, amount, .. } => {
            sh21_trace!("SH21 stage=native-incr name={name} span={:?}", stmt.span());
            // `incr` reads both its target variable and (when present) its
            // increment argument as Int.  The two checks are independent —
            // an Int target with a String `$amount` still shimmers on the
            // amount — so neither must short-circuit the other.
            check_incr_var(ctx, element_var_name(name), stmt.span(), uses);
            if amount.is_some()
                && let Some(word) = ctx
                    .source
                    .source_tokens()
                    .and_then(|tokens| tokens.words().get(2))
                && let Some((spelling, _)) = word.sole_variable_substitution()
                && let Some(read) = ctx.source.read_word(word)
            {
                check_incr_read(
                    ctx,
                    element_var_name(spelling),
                    stmt.span(),
                    read,
                    Some(word),
                );
            }
        }

        _ => {}
    }
}

/// Check one `incr` operand (the target variable or the increment
/// argument) for an intrep shimmer to `Int`.
///
/// `var` is the normalised variable name. Fires when its known intrep is a
/// non-int, non-numeric type that is not a clean hex/octal/binary integer
/// literal string (that spelling promotes cleanly to int).
fn check_incr_var(ctx: &mut UseSiteCtx<'_>, var: &str, span: Span, uses: &HashMap<Symbol, u32>) {
    let Some(sym) = sh21_option("incr.symbol", ctx.source.symbol(var)) else {
        return;
    };
    let Some(&ver) = sh21_option("incr.version", uses.get(&sym)) else {
        return;
    };
    check_incr_read(
        ctx,
        var,
        span,
        crate::ssa::SsaReadReference {
            symbol: sym,
            version: Some(ver),
        },
        None,
    );
}

fn check_incr_read(
    ctx: &mut UseSiteCtx<'_>,
    var: &str,
    span: Span,
    read: crate::ssa::SsaReadReference,
    word: Option<&crate::ir::WordExpr>,
) {
    let sym = read.symbol;
    if ctx.array_syms.contains(&sym) {
        return;
    }
    sh21_trace!("SH21 stage=incr.read var={var} version={:?}", read.version);
    let Some(ver) = read.version else {
        return;
    };
    if ver == 0 {
        return;
    }
    let lattice = ctx
        .types
        .get((sym, ver))
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    sh21_trace!(
        "SH21 stage=incr.lattice var={var} kind={:?}",
        lattice.kind()
    );
    if lattice.kind() != TypeKind::Known {
        return;
    }
    let Some(current) = lattice.tcl_type() else {
        return;
    };
    // A prior use that committed a non-numeric intrep on every path makes this
    // `incr` a genuine second conversion — `set v 5; llength $v; incr v`
    // commits List at the `llength`, then re-represents List → Int here.
    let cost = match word {
        Some(word) => ctx
            .commit
            .cost_for_word(ctx.source, word, read, current, TclType::Int),
        None => ctx
            .commit
            .cost_for_native_read(ctx.source, read, current, TclType::Int),
    };
    let Some(cost) = sh21_option("incr.captured_cost", cost) else {
        return;
    };
    sh21_trace!(
        "SH21 stage=incr.cost var={var} current={:?} commitment={:?}",
        cost.current,
        cost.commitment
    );
    let current = cost.current;
    let commit_state = cost.commitment;
    let must_pay_committed = commit_state.must_pay(TclType::Int);
    if !must_pay_committed {
        if current == TclType::Int || is_numeric_compatible(current, TclType::Int) {
            return;
        }
        // A pure value whose string form is a whole integer (`set n 0x80;
        // incr n`, `set n 5; incr n`) promotes to `Int` for free — no shimmer.
        // A non-integer pure string (`set n hello; incr n`) is *not* a valid
        // instance, so it still fires: that conversion fails at runtime.
        if sh21_flag(
            "free_first_conversion",
            is_free_first_conversion(
                current,
                TclType::Int,
                ctx.values.get(&(sym, ver)),
                ctx.representation(sym, ver),
                ctx.commit.numbers(),
                ctx.commit.word_rules(),
            ),
        ) {
            return;
        }
    }
    let (from, from_label) = super::committed_from_label(&commit_state, current, TclType::Int);
    if from == TclType::Int || is_numeric_compatible(from, TclType::Int) {
        return;
    }
    let related: Vec<(Span, String)> = commit_state
        .first_span()
        .filter(|_| must_pay_committed)
        .map(|sp| vec![(sp, "intrep first committed here".to_owned())])
        .or_else(|| {
            ctx.def_map
                .get(&(sym, ver))
                .map(|&sp| vec![(sp, "value defined here".to_owned())])
        })
        .unwrap_or_default();
    let code = if ctx.in_loop {
        DiagCode::S101
    } else {
        DiagCode::S100
    };
    ctx.out.push(ShimmerWarning {
        span,
        variable: var.to_owned(),
        from_type: from,
        to_type: TclType::Int,
        command: "incr".to_owned(),
        in_loop: ctx.in_loop,
        code,
        message: format!("variable '{var}' has {from_label} intrep but 'incr' expects int"),
        related,
    });
}

#[cfg(test)]
pub(crate) fn find_use_site_shimmers(
    cfg: &CfgFunction,
    commit_ctx: &super::commit::CommitCtx<'_>,
    (executable_blocks, in_force): (
        &HashSet<BlockId>,
        &HashMap<(BlockId, ValueKey), TypeLattice>,
    ),
    facts: &super::ShimmerFacts,
) -> Vec<ShimmerWarning> {
    find_use_site_shimmers_with_context(cfg, commit_ctx, (executable_blocks, in_force), facts)
}

#[cfg(test)]
mod tests;
