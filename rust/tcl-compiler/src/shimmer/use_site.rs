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
use crate::value_shapes::is_pure_var_ref;

use super::hints::{
    ShimmerExpectation, arg_shimmer_expectation, arg_shimmer_type, inert_braced_args,
    is_numeric_compatible, is_uncommitted_first_conversion,
};
use super::span::def_range_map;
use super::{ShimmerWarning, type_name};

/// Find use-site shimmer warnings for a function.
///
/// Walks every executable block in CFG order, checks each statement's
/// argument words against the registry's `arg_types`, and emits a
/// [`ShimmerWarning`] for each type mismatch where the variable's known
/// type differs from what the command requires.
#[must_use]
pub(crate) fn find_use_site_shimmers(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<BlockId>,
    registry: &CommandRegistry,
    values: &HashMap<ValueKey, LatticeValue>,
    facts: &super::ShimmerFacts,
) -> Vec<ShimmerWarning> {
    let loop_blocks = &facts.loop_blocks;
    let def_map = def_range_map(ssa);
    // Loop-invariance facts for the S101→S100 downgrade — only needed when
    // the function has at least one loop block.
    let loop_facts = LoopFacts::compute(cfg, ssa, loop_blocks, registry);
    // Array-base symbols are excluded (FP-SH-13): `normalise_var_name` strips
    // the `(key)` suffix, so `arr(a)` and `arr(b)` share one symbol / version
    // chain. Two individually-stable but different elements then look, at a
    // use site, exactly like one variable holding the wrong intrep — the same
    // conflation the S102 pass already guards. Reuse its exclusion so S100 /
    // S101 don't false-positive on independent array elements.
    let array_syms = super::thunking::array_element_symbols(cfg, ssa);
    let commit_ctx = super::commit::CommitCtx {
        registry,
        ssa,
        types,
        values,
    };
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
        let mut already_coerced: HashSet<(String, u32, TclType)> = HashSet::new();
        // The committed-intrep walker replays the commit transfer function in
        // step with this walk, so each statement's checks see the state *just
        // before* it executes.
        let mut commit_walker = facts.commit.walker(&commit_ctx, block_id);
        for ss in &ssa_block.statements {
            let mut ctx = UseSiteCtx {
                types,
                registry,
                def_map: &def_map,
                values,
                loop_facts: &loop_facts,
                ssa,
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
    types: &'a HashMap<ValueKey, TypeLattice>,
    registry: &'a CommandRegistry,
    def_map: &'a HashMap<ValueKey, Span>,
    values: &'a HashMap<ValueKey, LatticeValue>,
    loop_facts: &'a LoopFacts,
    ssa: &'a SsaFunction,
    /// Array-base symbols excluded from shimmer reporting (FP-SH-13) — a
    /// conflated `arr(a)`/`arr(b)` symbol can hold either element's intrep.
    array_syms: &'a HashSet<Symbol>,
    /// The committed-intrep state just before the statement being checked —
    /// tells a use whether the value already committed a different intrep on
    /// every path (a genuine second conversion, [`super::commit`]).
    commit: &'a super::commit::CommitWalker<'a>,
    in_loop: bool,
    already_coerced: &'a mut HashSet<(String, u32, TclType)>,
    out: &'a mut Vec<ShimmerWarning>,
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
    /// Names defined anywhere in a loop block (statement defs + phis).
    def_names: HashSet<String>,
    /// Per-name set of expected intreps requested at any use-site in a
    /// loop block.
    use_targets: HashMap<String, HashSet<TclType>>,
}

impl LoopFacts {
    fn compute(
        cfg: &CfgFunction,
        ssa: &SsaFunction,
        loop_blocks: &HashSet<String>,
        registry: &CommandRegistry,
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
                    facts
                        .def_names
                        .extend(st.defs.keys().map(|&sym| ssa.var_name(sym).to_owned()));
                }
                for phi in &sb.phis {
                    facts.def_names.insert(ssa.var_name(phi.name).to_owned());
                }
            }
            if let Some(cb) = cfg.blocks.get(&id) {
                for stmt in &cb.statements {
                    facts.record_use_targets(stmt, registry);
                }
            }
        }
        facts
    }

    /// Record the expected intrep of every `$var` argument at a shimmering
    /// position of `stmt` (a direct call or a `[cmd …]` substitution
    /// inside an assignment value).
    fn record_use_targets(&mut self, stmt: &Statement, registry: &CommandRegistry) {
        let mut record = |command: &str, args: &[String]| {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            for (i, word) in args.iter().enumerate() {
                if !word.trim_start().starts_with('$') {
                    continue;
                }
                if let Some(expected) = arg_shimmer_type(registry, command, &arg_refs, i) {
                    let var = element_var_name(word.trim()).to_owned();
                    self.use_targets.entry(var).or_default().insert(expected);
                }
            }
        };
        match stmt {
            Statement::Call { command, args, .. } => record(command, args),
            Statement::AssignValue { value, .. } => {
                if let Some((command, args)) =
                    crate::value_shapes::parse_command_substitution_with_config(
                        value.trim(),
                        tcl_lexer::LexerConfig::for_profile(registry.profile()),
                    )
                {
                    record(&command, &args);
                }
            }
            _ => {}
        }
    }

    /// Refine `in_loop` for a single use of `var`: a loop-invariant variable
    /// coerced to fewer than two distinct intreps inside the loop converts
    /// once and is cached, so it is S100 (not S101).
    fn effective_in_loop(&self, var: &str, in_loop: bool) -> bool {
        if in_loop && !self.def_names.contains(var) {
            self.use_targets.get(var).is_some_and(|t| t.len() >= 2)
        } else {
            in_loop
        }
    }
}

/// Expected intrep for every list/dict argument of a synthetic loop-header
/// call: the CFG builder lowers `foreach` / `lmap` / `dict for` / `dict map`
/// to a `Statement::Call` whose `command` is that keyword (or, for the dict
/// forms, the two-word compound `"dict for"` / `"dict map"`) and whose
/// `args` are *only* the list/dict arguments — one per iterator group, in
/// order (see `cfg_builder::cfg_lower::lower_foreach`; identified by
/// `Statement::Call::foreach_groups` being `Some`, not by the command name).
///
/// Every iterator group's argument expects the *same* intrep, so this reads
/// the registry once and the caller applies the result uniformly — unlike a
/// real call's `arg_types`, which is keyed per source-position index and
/// so can't reach a multi-group loop's later arguments at all.
///
/// `dict for` / `dict map` are two-word compound names (AGENTS.md's
/// compound-command pattern — a base command with a subcommand argument,
/// like `namespace upvar`): split at the space and dispatch through the
/// `dict` subcommand table via [`arg_shimmer_type`]'s existing subcommand
/// path, requesting sub-index 1 (both subcommands declare their dict
/// argument there).
pub(super) fn foreach_header_expected_type(
    registry: &CommandRegistry,
    command: &str,
) -> Option<TclType> {
    if let Some((base, sub)) = command.split_once(' ') {
        // `arg_shimmer_type`'s subcommand path computes `sub_idx =
        // arg_index - 1`; requesting `arg_index = 2` reads sub-index 1.
        arg_shimmer_type(registry, base, &[sub], 2)
    } else {
        arg_shimmer_type(registry, command, &[], 0)
    }
}

/// Grouped, per-call arguments for [`check_invocation`] — keeps that
/// function's own parameter list short (the `ctx` + `uses` thread-through
/// state are the only params that vary independently of "which invocation").
#[derive(Clone, Copy)]
struct InvocationSite<'a> {
    /// Source spelling, for the warning's `command` field and message.
    command: &'a str,
    /// Registry lookup key — the *canonical* command name when the call is
    /// an `interp alias` target (`interp alias {} myindex {} ::lindex;
    /// myindex $x 0` resolves `lookup_command = "::lindex"`), so an aliased
    /// call to a shimmering builtin is still recognised, matching
    /// [`Statement::Call::canonical_command`]'s documented "diagnostics
    /// read better with the spelling the user wrote" rationale.
    lookup_command: &'a str,
    /// Argument words.
    args: &'a [String],
    /// Per-argument absolute spans, index-aligned with `args`, when
    /// available (see `fallback_span`).
    arg_spans: &'a [Span],
    /// The statement's word snapshot, when this site is a whole statement.
    /// `args` is de-braced, so only these token kinds can tell a
    /// brace-quoted literal from a live substitution — see
    /// [`inert_braced_args`]. `None` for a lifted `[cmd …]`, whose argument
    /// words are raw source text with the braces still on.
    tokens: Option<&'a crate::ir::CommandTokens>,
    /// True for the synthetic loop-header shape [`foreach_header_expected_type`]
    /// covers (`foreach` / `lmap` / `dict for` / `dict map`) — every
    /// argument shares one expected type instead of a per-index one.
    is_foreach_header: bool,
    /// Span used for any argument index `arg_spans` doesn't cover — a
    /// synthetic call built without per-word token spans (some test
    /// fixtures) or a substitution word count `arg_spans` didn't fully
    /// resolve.
    fallback_span: Span,
}

/// Check one command invocation's arguments for an intrep mismatch
/// against the variables' known types. Used for both a top-level
/// [`Statement::Call`] and a `[cmd …]` substitution lifted out of a
/// [`Statement::AssignValue`] value (`set b [lindex $x 0]`).
///
/// Two narrower residual gaps remain, both strictly better than today's "no
/// alias detection at all":
/// - Argument *indices* are unadjusted, so an alias that prepends fixed
///   arguments (`interp alias {} foo {} ::bar prefix`) can index-shift the
///   wrong argument.
/// - A read-modify-write shimmering argument that is a **bare variable
///   name**, not a `$`-prefixed read (`incr`/`append`/`lappend`'s target) is
///   never seen here even when aliased: `is_pure_var_ref` below only matches
///   `$`-style reads, and `incr`'s own canonical name bypasses this function
///   entirely via the dedicated [`Statement::Incr`] node (see
///   [`check_incr_var`]) — a form `lower_command` only builds for the literal
///   command name, not an alias target.
fn check_invocation(
    ctx: &mut UseSiteCtx<'_>,
    site: &InvocationSite<'_>,
    uses: &HashMap<Symbol, u32>,
) {
    let arg_refs: Vec<&str> = site.args.iter().map(String::as_str).collect();
    // Positions whose brace-quoted word Tcl never substitutes: their
    // de-braced `args` text spells a live `$x` that is not one (issue #1845).
    let inert = inert_braced_args(ctx.registry, site.lookup_command, &arg_refs, site.tokens);
    for (i, word) in site.args.iter().enumerate() {
        if inert.contains(&i) {
            continue;
        }
        check_argument(ctx, site, i, word, &arg_refs, uses);
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
    word: &str,
    uses: &HashMap<Symbol, u32>,
) -> Option<(String, Symbol, u32, TclType)> {
    let stripped = word.trim();
    if !is_pure_var_ref(stripped) {
        return None;
    }
    let var = element_var_name(stripped).to_owned();
    let sym = ctx.ssa.var_symbol(&var)?;
    if ctx.array_syms.contains(&sym) {
        return None;
    }
    let &ver = uses.get(&sym)?;
    if ver == 0 {
        return None;
    }
    let lattice = ctx
        .types
        .get(&(sym, ver))
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    if lattice.kind() != TypeKind::Known {
        return None;
    }
    lattice.tcl_type().map(|current| (var, sym, ver, current))
}

fn check_argument(
    ctx: &mut UseSiteCtx<'_>,
    site: &InvocationSite<'_>,
    i: usize,
    word: &str,
    arg_refs: &[&str],
    uses: &HashMap<Symbol, u32>,
) {
    let expectation = if site.is_foreach_header {
        foreach_header_expected_type(ctx.registry, site.lookup_command).map(|expected| {
            ShimmerExpectation {
                expected,
                transparent_from: &[],
            }
        })
    } else {
        arg_shimmer_expectation(ctx.registry, site.lookup_command, arg_refs, i)
    };
    let Some(expectation) = expectation else {
        return;
    };
    let expected = expectation.expected;
    let Some((var, sym, ver, current)) = resolve_tracked_var_use(ctx, word, uses) else {
        return;
    };
    // The committed-intrep state just before this statement: when a prior use
    // (`expr`, `llength`, …) has committed a different intrep on **every**
    // executable path, this read genuinely re-represents even where the
    // flow-insensitive lattice sees no mismatch — `set v 5; expr {$v+1};
    // lindex $v 0` shimmers Numeric → List though the lattice types `v` Int
    // (pure literal). Resolved before the lattice-equality skip below so the
    // second conversion is not masked.
    let commit_state = ctx.commit.state_of(sym, ver);
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
    // 12.0}; foreach s $fontSizes` (issue #940), `set x 5; lindex $x 0`, and
    // `set d {a 1 b 2}; dict for {k v} $d` are all free promotions of a pure
    // string, not shimmers — see [`is_uncommitted_first_conversion`].
    //
    // The one exception is a value read as **two or more distinct intreps
    // inside a loop** (`llength $l` + `dict size $l` each pass): the loop
    // entry re-joins the pure preheader path every iteration, but at runtime
    // the converters re-thunk the value on every pass after the first —
    // oracle-verified list↔dict oscillation — so the free-first-conversion
    // suppression must not apply (the multi-target `LoopFacts` fact, which
    // also drives the S101 escalation).
    let multi_target_in_loop = ctx.in_loop
        && ctx
            .loop_facts
            .use_targets
            .get(&var)
            .is_some_and(|targets| targets.len() >= 2);
    if !multi_target_in_loop
        && is_uncommitted_first_conversion(
            current,
            expected,
            ctx.values.get(&(sym, ver)),
            ctx.commit.numbers(),
            ctx.commit.word_rules(),
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
    let coercion_key = (warning.var.to_owned(), warning.ver, warning.expected);
    if !ctx.already_coerced.insert(coercion_key) {
        return;
    }
    let related = warning.related_override.clone().unwrap_or_else(|| {
        ctx.def_map
            .get(&(warning.sym, warning.ver))
            .map(|&sp| vec![(sp, "value defined here".to_owned())])
            .unwrap_or_default()
    });
    // A loop-invariant variable coerced to a single intrep inside the
    // loop converts once and is cached → S100, not the per-iteration
    // S101.
    let code = if ctx.loop_facts.effective_in_loop(warning.var, ctx.in_loop) {
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
    uses: &HashMap<Symbol, u32>,
) {
    let config = tcl_lexer::LexerConfig::for_profile(ctx.registry.profile());
    for lifted in crate::word_subst::lifted_calls(tokens, config) {
        check_invocation(
            ctx,
            &InvocationSite {
                command: &lifted.command,
                lookup_command: &lifted.command,
                args: &lifted.args,
                arg_spans: &lifted.arg_spans,
                tokens: None,
                is_foreach_header: false,
                fallback_span,
            },
            uses,
        );
    }
}

fn check_statement(ctx: &mut UseSiteCtx<'_>, stmt: &Statement, uses: &HashMap<Symbol, u32>) {
    match stmt {
        Statement::Call {
            command,
            args,
            tokens,
            foreach_groups,
            ..
        } => {
            // A `[cmd …]` in one of this call's words reads its arguments
            // exactly as the same command on its own line would — Tcl runs it
            // before the outer command either way — so check it as the
            // invocation it is (issue #1814 follow-up).
            check_lifted_calls(ctx, tokens.as_ref(), stmt.span(), uses);

            let lookup = stmt.canonical_command_or_source();
            // `tokens.argv[0]` is the command word; `argv[1..]` are the
            // per-argument spans, index-aligned with `args` (both are the
            // literal source words — alias resolution never reorders or
            // reparents them, see `check_invocation`'s doc comment).
            let arg_spans: Vec<Span> = tokens
                .as_ref()
                .map(|t| t.argv.iter().skip(1).copied().collect())
                .unwrap_or_default();
            check_invocation(
                ctx,
                &InvocationSite {
                    command,
                    lookup_command: lookup,
                    args,
                    arg_spans: &arg_spans,
                    tokens: tokens.as_ref(),
                    is_foreach_header: foreach_groups.is_some(),
                    fallback_span: stmt.span(),
                },
                uses,
            );
        }

        // The words of an assignment substitute exactly as a call's do —
        // `set b [lindex $x 0]` runs `lindex` before it stores anything, and
        // `set b [list [lindex $x 0]]` runs it just the same, one level down.
        // So the value word goes through the same lift as a call's words
        // (issue #1844): one owner decides which substitutions a statement
        // runs, and the outermost `[cmd …]` is simply its depth-zero case
        // rather than a shape this arm re-derives for itself.
        Statement::AssignValue { tokens, .. } => {
            check_lifted_calls(ctx, tokens.as_ref(), stmt.span(), uses);
        }

        Statement::Incr { name, amount, .. } => {
            // `incr` reads both its target variable and (when present) its
            // increment argument as Int.  The two checks are independent —
            // an Int target with a String `$amount` still shimmers on the
            // amount — so neither must short-circuit the other.
            check_incr_var(ctx, element_var_name(name), stmt.span(), uses);
            if let Some(amt) = amount.as_deref().map(str::trim)
                && amt.starts_with('$')
            {
                check_incr_var(ctx, element_var_name(amt), stmt.span(), uses);
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
    let Some(sym) = ctx.ssa.var_symbol(var) else {
        return;
    };
    // Skip an array base (FP-SH-13) — see `check_invocation`.
    if ctx.array_syms.contains(&sym) {
        return;
    }
    let Some(&ver) = uses.get(&sym) else { return };
    if ver == 0 {
        return;
    }
    let lattice = ctx
        .types
        .get(&(sym, ver))
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    if lattice.kind() != TypeKind::Known {
        return;
    }
    let Some(current) = lattice.tcl_type() else {
        return;
    };
    // A prior use that committed a non-numeric intrep on every path makes this
    // `incr` a genuine second conversion — `set v 5; llength $v; incr v`
    // commits List at the `llength`, then re-represents List → Int here.
    let commit_state = ctx.commit.state_of(sym, ver);
    let must_pay_committed = commit_state.must_pay(TclType::Int);
    if !must_pay_committed {
        if current == TclType::Int || is_numeric_compatible(current, TclType::Int) {
            return;
        }
        // A pure value whose string form is a whole integer (`set n 0x80;
        // incr n`, `set n 5; incr n`) promotes to `Int` for free — no shimmer.
        // A non-integer pure string (`set n hello; incr n`) is *not* a valid
        // instance, so it still fires: that conversion fails at runtime.
        if is_uncommitted_first_conversion(
            current,
            TclType::Int,
            ctx.values.get(&(sym, ver)),
            ctx.commit.numbers(),
            ctx.commit.word_rules(),
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
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// Compute commit facts + run the use-site detector for one function unit
    /// — the production wiring `find_shimmer_warnings` performs.
    fn use_site_shimmers(
        fu: &crate::compilation_unit::FunctionUnit,
        registry: &CommandRegistry,
    ) -> Vec<ShimmerWarning> {
        let ctx = super::super::commit::CommitCtx {
            registry,
            ssa: &fu.ssa,
            types: &fu.types,
            values: &fu.sccp.values,
        };
        let facts = super::super::ShimmerFacts {
            commit: super::super::commit::compute_commit_facts(
                &fu.cfg,
                &ctx,
                &fu.sccp.executable_blocks,
                &fu.sccp.executable_edges,
            ),
            loop_blocks: super::super::graph::loop_body_blocks(&fu.cfg),
        };
        find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            registry,
            &fu.sccp.values,
            &facts,
        )
    }

    /// A String variable passed to `incr` triggers S100.
    #[test]
    fn shimmer_detected_for_string_used_with_incr() {
        let cu = CompilationUnit::build_for("set x \"hello\"\nincr x", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings
            .iter()
            .find(|w| w.command == "incr" && w.from_type == TclType::String);
        assert!(
            w.is_some(),
            "expected incr/String shimmer, got: {warnings:?}"
        );
        assert_eq!(w.unwrap().to_type, TclType::Int);
    }

    /// A hex literal string (`0x80`) classifies as
    /// String but promotes cleanly to Int at `incr` — no shimmer.
    #[test]
    fn no_shimmer_for_hex_literal_string_used_with_incr() {
        let cu = CompilationUnit::build_for("set n 0x80\nincr n", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let incr_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "incr").collect();
        assert!(
            incr_shimmers.is_empty(),
            "0x80 promotes cleanly to int — no incr shimmer expected: {incr_shimmers:?}",
        );
    }

    /// An Int variable passed to `incr` has no shimmer.
    #[test]
    fn no_shimmer_for_int_used_with_incr() {
        let cu = CompilationUnit::build_for("set x 5\nincr x", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let incr_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "incr").collect();
        assert!(
            incr_shimmers.is_empty(),
            "unexpected incr shimmer for Int: {incr_shimmers:?}"
        );
    }

    /// A *committed* Int variable passed to `lindex` triggers a shimmer
    /// (Int → List at arg 0). The Int comes from `[llength]` (a committed
    /// numeric intrep), not a bare literal — `set x 5; lindex $x 0` is a *pure*
    /// value that promotes for free (see `no_shimmer_for_pure_literal_as_list`).
    #[test]
    fn shimmer_detected_for_int_used_with_lindex() {
        let cu = CompilationUnit::build_for(
            "proc f {lst} {\n set x [llength $lst]\n lindex $x 0\n}",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings.iter().find(|w| w.command == "lindex");
        assert!(
            w.is_some(),
            "expected lindex shimmer for committed Int var, got: {warnings:?}"
        );
        assert_eq!(w.unwrap().from_type, TclType::Int);
        assert_eq!(w.unwrap().to_type, TclType::List);
    }

    /// TN (issue #940): a *pure* literal — even a number-shaped or word-shaped
    /// one — used as a list must NOT shimmer. `set a 1; foreach b $a` and `set x
    /// 5; lindex $x 0` promote a pure string to a list for free (oracle: `set x
    /// 5` is a pure string, `lindex` parses it into a 1-element list once). The
    /// define-time shape (`1` looks integer-ish) does not make the value any
    /// less a valid list.
    #[test]
    fn no_shimmer_for_pure_literal_as_list() {
        for src in [
            "set a 1\nforeach b $a {}",
            "set x 5\nlindex $x 0",
            "set p 3.14\nlindex $p 0",
        ] {
            let cu = CompilationUnit::build_for(src, &registry(), false);
            let fu = cu.function("::top").unwrap();
            let warnings = use_site_shimmers(fu, &registry());
            assert!(
                warnings.is_empty(),
                "pure literal used as a list must not shimmer for {src:?}: {warnings:?}"
            );
        }
    }

    /// The warning's span is tight around the offending `$x` argument, not
    /// the whole `lindex $x 0` invocation — the developer's eye should land
    /// on the variable, not have to scan the whole call.
    #[test]
    fn shimmer_span_is_tight_around_call_argument() {
        let src = "set x [dict create a 1 b 2]\nlindex $x 0";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings
            .iter()
            .find(|w| w.command == "lindex")
            .unwrap_or_else(|| panic!("expected lindex shimmer, got: {warnings:?}"));
        let text = &src[w.span.start() as usize..w.span.end() as usize];
        assert_eq!(
            text, "$x",
            "span should cover only the '$x' argument, got {text:?} from {w:?}"
        );
    }

    /// Issue #1814 follow-up: a `[cmd …]` in a call argument reads its
    /// arguments exactly as the same command on its own line does — Tcl runs
    /// it either way.
    #[test]
    fn use_site_shimmer_fires_in_a_nested_call_argument() {
        let cu = CompilationUnit::build_for(
            "proc f {lst} {\n set x [llength $lst]\n puts [lindex $x 0]\n}",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = use_site_shimmers(fu, &registry());
        let hit = w.iter().find(|w| w.command == "lindex");
        assert!(hit.is_some(), "nested lindex is still a list read: {w:?}");
        assert_eq!(hit.unwrap().from_type, TclType::Int);
        assert_eq!(hit.unwrap().to_type, TclType::List);
    }

    /// The braced spelling of that word is a literal Tcl never runs, and the
    /// flat argument text cannot tell the two apart — only `word_exprs` can.
    #[test]
    fn no_use_site_shimmer_in_a_braced_call_argument() {
        let cu = CompilationUnit::build_for(
            "proc f {lst} {\n set x [llength $lst]\n puts {[lindex $x 0]}\n}",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = use_site_shimmers(fu, &registry());
        assert!(
            !w.iter().any(|w| w.command == "lindex"),
            "a braced word is data, not a call: {w:?}"
        );
    }

    /// Issue #1845: the braced / quoted / bare spellings of one argument
    /// word, pinned against each other for a command that consumes its word
    /// as **data** (`lindex`) and one that re-evaluates it **in the caller's
    /// frame** (`expr`) — the two halves of the rule, so neither can drift.
    ///
    /// tclsh 9.0.4, `set x 5`: `lindex {$x} 0` yields the two characters
    /// `$x` (no read at all) while `lindex "$x" 0` and `lindex $x 0` both
    /// yield `5`; `expr {$x} + 1` yields `6`, so every `expr` spelling reads
    /// `x`. A statement's IR `args` hold the *de-braced* word, so all six
    /// spellings look alike there — the answer comes from the segmenter's
    /// token kinds plus the registry's argument role.
    #[test]
    fn a_braced_argument_reads_only_where_the_callee_evaluates_it_in_frame() {
        for (body, reads) in [
            ("set x [llength $lst]\n lindex {$x} 0", false),
            ("set x [llength $lst]\n lindex \"$x\" 0", true),
            ("set x [llength $lst]\n lindex $x 0", true),
            ("set x [lrange $lst 0 1]\n expr {$x} + 1", true),
            ("set x [lrange $lst 0 1]\n expr \"$x\" + 1", true),
            ("set x [lrange $lst 0 1]\n expr $x + 1", true),
        ] {
            let src = format!("proc f {{lst}} {{\n {body}\n}}");
            let cu = CompilationUnit::build_for(&src, &registry(), false);
            let w = super::super::find_shimmer_warnings_for_cu(&cu, &registry());
            assert_eq!(
                !w.is_empty(),
                reads,
                "expected reads={reads} for {src:?}, got: {w:?}"
            );
        }
    }

    /// The same braced word must not move the **committed-intrep state**
    /// either. `commit.rs` emits nothing itself, so a gate only in the
    /// emitting pass left `lindex {$x} 0` recording a list commitment that
    /// made the genuine read on the next line report a conversion the
    /// runtime never performs.
    #[test]
    fn a_braced_argument_commits_no_intrep_for_later_reads() {
        let cu = CompilationUnit::build_for(
            "proc f {lst} {\n set x [llength $lst]\n lindex {$x} 0\n expr {$x + 1}\n}",
            &registry(),
            false,
        );
        let w = super::super::find_shimmer_warnings_for_cu(&cu, &registry());
        assert!(
            w.is_empty(),
            "the int `x` is only ever read as a number here: {w:?}"
        );
    }

    /// Issue #1844: the two statement kinds that carry substitutable words
    /// must agree about which commands a word runs. `puts <word>` and
    /// `set r <word>` differ only in the outer command, and neither outer
    /// command changes what Tcl evaluates inside the word — so any word that
    /// shimmers in a call argument shimmers identically in an assignment
    /// value, and any word that does not, does not.
    ///
    /// Written as a paired assertion rather than two independent expectations
    /// on purpose: it is the *symmetry* that regressed (the `AssignValue` arm
    /// parsed only its outermost `[cmd …]` and so went silent below depth
    /// one), and only a test that compares the paths can catch them drifting
    /// apart again.
    #[test]
    fn assign_value_lifts_the_same_substitutions_a_call_does() {
        // (word, does its `lindex` run?) — a bare substitution, then the
        // nesting depths the `AssignValue` arm used to lose, then the braced
        // spelling Tcl never substitutes.
        for (word, runs) in [
            ("[lindex $x 0]", true),
            ("[list [lindex $x 0]]", true),
            ("[list \"[lindex $x 0]\"]", true),
            ("[list a[lindex $x 0]b]", true),
            ("[list {[lindex $x 0]}]", false),
        ] {
            let mut verdicts = Vec::new();
            for stmt in [format!("puts {word}"), format!("set r {word}")] {
                let src = format!("proc f {{lst}} {{\n set x [llength $lst]\n {stmt}\n}}");
                let cu = CompilationUnit::build_for(&src, &registry(), false);
                let fu = cu.function("::f").unwrap();
                let w = use_site_shimmers(fu, &registry());
                let hit = w.iter().find(|w| w.command == "lindex").cloned();
                if let Some(hit) = hit.as_ref() {
                    assert_eq!(hit.from_type, TclType::Int, "in {stmt:?}");
                    assert_eq!(hit.to_type, TclType::List, "in {stmt:?}");
                }
                verdicts.push((stmt, hit.is_some()));
            }
            let [(call, call_fired), (assign, assign_fired)] = verdicts.as_slice() else {
                unreachable!("one verdict per spelling")
            };
            assert_eq!(
                call_fired, assign_fired,
                "{call:?} and {assign:?} run the same commands and must report alike"
            );
            assert_eq!(*call_fired, runs, "wrong verdict for {call:?}");
        }
    }

    /// The lift is the *only* path into the `AssignValue` arm, so the
    /// depth-zero substitution it used to special-case must still be reported
    /// exactly once — a second, arm-local parse of the outermost `[cmd …]`
    /// would double every `set b [lindex $x 0]` diagnostic.
    #[test]
    fn assign_value_substitution_is_reported_once() {
        let cu = CompilationUnit::build_for(
            "proc f {lst} {\n set x [llength $lst]\n set b [lindex $x 0]\n}",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = use_site_shimmers(fu, &registry());
        assert_eq!(
            w.iter().filter(|w| w.command == "lindex").count(),
            1,
            "one substitution, one report: {w:?}"
        );
    }

    /// The span stays tight around the `$x` of the *nested* command, not the
    /// outer `[list …]` and not the whole `set` — the developer's eye has to
    /// land on the read that actually converts.
    #[test]
    fn assign_value_nested_substitution_span_is_tight() {
        let src = "proc f {lst} {\n set x [llength $lst]\n set r [list [lindex $x 0]]\n}";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings
            .iter()
            .find(|w| w.command == "lindex")
            .unwrap_or_else(|| panic!("expected nested lindex shimmer, got: {warnings:?}"));
        let text = &src[w.span.start() as usize..w.span.end() as usize];
        assert_eq!(text, "$x", "got {text:?} from {w:?}");
    }

    /// The conversion a nested substitution performs must also reach the
    /// commit state, or every later read of the same variable is judged
    /// against a stale representation. Before this, the `incr` here reported
    /// nothing at all — while the identical code with `lindex $x 0` on its own
    /// line reported it.
    #[test]
    fn a_nested_conversion_is_visible_to_later_reads() {
        let cu = CompilationUnit::build_for(
            "proc f {lst} {\n set x [llength $lst]\n puts [lindex $x 0]\n incr x\n}",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = use_site_shimmers(fu, &registry());
        let incr = w.iter().find(|w| w.command == "incr");
        assert!(incr.is_some(), "the incr after a nested list read: {w:?}");
        assert_eq!(
            incr.unwrap().from_type,
            TclType::List,
            "it converted back from the list the nested `lindex` installed"
        );
    }

    /// Issue #1844, the commit half: the same must hold when the nested
    /// substitution sits in an assignment value. The `AssignValue` transfer
    /// function used to walk only the outermost `[cmd …]`, so `[list […]]`
    /// moved no state and the following `incr` was judged against the stale
    /// `Int` — reporting nothing.
    #[test]
    fn a_nested_conversion_in_an_assignment_is_visible_to_later_reads() {
        let cu = CompilationUnit::build_for(
            "proc f {lst} {\n set x [llength $lst]\n set r [list [lindex $x 0]]\n incr x\n}",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = use_site_shimmers(fu, &registry());
        let incr = w.iter().find(|w| w.command == "incr");
        assert!(incr.is_some(), "the incr after a nested list read: {w:?}");
        assert_eq!(
            incr.unwrap().from_type,
            TclType::List,
            "it converted back from the list the nested `lindex` installed"
        );
    }

    /// Same tightness property through the `[cmd …]`-in-`AssignValue` path
    /// (`set b [lindex $x 0]`) — the span still lands on `$x`, not the
    /// whole `set b […]` statement.
    #[test]
    fn shimmer_span_is_tight_around_assign_value_substitution_argument() {
        let src = "set x [dict create a 1 b 2]\nset b [lindex $x 0]";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings
            .iter()
            .find(|w| w.command == "lindex")
            .unwrap_or_else(|| panic!("expected lindex shimmer, got: {warnings:?}"));
        let text = &src[w.span.start() as usize..w.span.end() as usize];
        assert_eq!(
            text, "$x",
            "span should cover only the '$x' argument, got {text:?} from {w:?}"
        );
    }

    /// Two shimmering arguments of the *same* call each get their *own*
    /// tight span — proves the per-index span lookup, not just "arg 0's
    /// span reused everywhere". `linsert list index element`: `list_var`
    /// (committed Dict, arg 0) expects List; `index_var` (String `world`, arg 1)
    /// expects Int — `world` is not a valid integer, so it still fires.
    #[test]
    fn shimmer_span_is_tight_per_argument_not_reused_across_args() {
        let src = "set list_var [dict create a 1 b 2]\n\
                    set index_var world\n\
                    linsert $list_var $index_var x";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let list_w = warnings
            .iter()
            .find(|w| w.command == "linsert" && w.variable == "list_var")
            .unwrap_or_else(|| panic!("expected linsert shimmer for list_var, got: {warnings:?}"));
        let list_text = &src[list_w.span.start() as usize..list_w.span.end() as usize];
        assert_eq!(list_text, "$list_var", "got {list_w:?}");

        let index_w = warnings
            .iter()
            .find(|w| w.command == "linsert" && w.variable == "index_var")
            .unwrap_or_else(|| panic!("expected linsert shimmer for index_var, got: {warnings:?}"));
        let index_text = &src[index_w.span.start() as usize..index_w.span.end() as usize];
        assert_eq!(index_text, "$index_var", "got {index_w:?}");
    }

    /// TN: a `foreach` element variable is typed String — each element is a
    /// *pure* string, and reading it as a list via `[lindex $x 0]` is a free
    /// first conversion (oracle: a list element goes pure → list once, per
    /// value). It is NOT a per-iteration shimmer: no cached intrep is thrown
    /// away, because a fresh element value arrives each pass. The substitution
    /// lives in an `AssignValue` value, so this also guards the `AssignValue`
    /// arm against re-flagging pure elements.
    #[test]
    fn no_shimmer_for_foreach_element_used_as_list_in_cmd_sub() {
        let src = "proc f {l} {\n    foreach x $l {\n        set b [lindex $x 0]\n    }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        assert!(
            warnings.iter().all(|w| w.command != "lindex"),
            "a pure foreach element read as a list must not shimmer: {warnings:?}"
        );
    }

    /// TP control (the committed counterpart): a *committed* value re-read as a
    /// list inside a loop, via the same `[cmd …]`-in-`AssignValue` path, DOES
    /// shimmer per iteration (S101). `[dict create k $x]` builds a fresh dict
    /// each pass (defined in the loop, so not the loop-invariant downgrade), and
    /// `[llength $d]` re-represents it list-ward every iteration.
    #[test]
    fn shimmer_detected_for_committed_value_as_list_in_cmd_sub() {
        let src = "proc f {l} {\n    foreach x $l {\n        set d [dict create k $x]\n        set n [llength $d]\n    }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings
            .iter()
            .find(|w| w.command == "llength" && w.variable == "d")
            .unwrap_or_else(|| {
                panic!("expected llength S101 for committed dict, got: {warnings:?}")
            });
        assert_eq!(w.code, DiagCode::S101);
        assert_eq!(w.from_type, TclType::Dict);
        assert_eq!(w.to_type, TclType::List);
    }

    /// TP: `foreach`'s own list argument shimmers when the variable holds a
    /// *committed* non-list intrep — the CFG builder lowers the header to a
    /// synthetic `Statement::Call` (`command="foreach", args=[list_arg]`) that
    /// never reaches the per-index `arg_types` path, so this is the
    /// `foreach_header_expected_type` path specifically. A committed `Dict`
    /// (from `[dict create]`) genuinely re-represents when foreach reads it as a
    /// list; a *pure* string would be a free first conversion (see the
    /// `no_shimmer_for_foreach_header_with_pure_string` TN below).
    #[test]
    fn shimmer_detected_for_foreach_header_list_argument() {
        let src = "proc f {} {\n    set l [dict create a 1 b 2]\n    foreach x $l { puts $x }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings
            .iter()
            .find(|w| w.command == "foreach" && w.variable == "l")
            .unwrap_or_else(|| panic!("expected foreach header shimmer, got: {warnings:?}"));
        assert_eq!(w.from_type, TclType::Dict);
        assert_eq!(w.to_type, TclType::List);
    }

    /// TN control: a genuine list variable in `foreach` must not shimmer.
    #[test]
    fn no_shimmer_for_foreach_header_with_real_list() {
        let src = "proc f {} {\n    set l [list 1 2 3]\n    foreach x $l { puts $x }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        assert!(
            warnings.iter().all(|w| w.command != "foreach"),
            "unexpected foreach header shimmer for a real list: {warnings:?}"
        );
    }

    /// TN (issue #940): a *pure* string literal in `foreach` must not shimmer —
    /// `{a b c}` (and any well-formed list literal) is a valid list, and Tcl
    /// parses it into a list intrep once, for free (oracle: the value goes pure
    /// → list). This is the exact false positive issue #940 reported.
    #[test]
    fn no_shimmer_for_foreach_header_with_pure_string() {
        for literal in ["{10.0 12.0 16.0 24.0}", "hello", "{}", "1"] {
            let src = format!(
                "proc f {{}} {{\n    set l {literal}\n    foreach x $l {{ puts $x }}\n}}\n"
            );
            let cu = CompilationUnit::build_for(&src, &registry(), false);
            let fu = cu.function("::f").unwrap();
            let warnings = use_site_shimmers(fu, &registry());
            assert!(
                warnings.iter().all(|w| w.variable != "l"),
                "pure string literal {literal:?} in foreach must not shimmer: {warnings:?}"
            );
        }
    }

    /// TP: `lmap`'s list argument shares the same synthetic header shape.
    #[test]
    fn shimmer_detected_for_lmap_header_list_argument() {
        let src = "proc f {} {\n    set l [dict create a 1 b 2]\n    lmap x $l { set x $x }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings
            .iter()
            .find(|w| w.command == "lmap" && w.variable == "l")
            .unwrap_or_else(|| panic!("expected lmap header shimmer, got: {warnings:?}"));
        assert_eq!(w.from_type, TclType::Dict);
        assert_eq!(w.to_type, TclType::List);
    }

    /// TP: `dict for`'s dict argument shimmers to Dict — the compound
    /// two-word command name (`"dict for"`) routes through the `dict`
    /// subcommand table via `foreach_header_expected_type`'s split.
    #[test]
    fn shimmer_detected_for_dict_for_header_argument() {
        let src = "proc f {} {\n    set d hello\n    dict for {k v} $d { puts $k }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings
            .iter()
            .find(|w| w.command == "dict for" && w.variable == "d")
            .unwrap_or_else(|| panic!("expected dict-for header shimmer, got: {warnings:?}"));
        assert_eq!(w.from_type, TclType::String);
        assert_eq!(w.to_type, TclType::Dict);
    }

    /// TP: `dict map` shares `dict for`'s compound-name dispatch.
    #[test]
    fn shimmer_detected_for_dict_map_header_argument() {
        let src = "proc f {} {\n    set d hello\n    dict map {k v} $d { set v $v }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings
            .iter()
            .find(|w| w.command == "dict map" && w.variable == "d")
            .unwrap_or_else(|| panic!("expected dict-map header shimmer, got: {warnings:?}"));
        assert_eq!(w.from_type, TclType::String);
        assert_eq!(w.to_type, TclType::Dict);
    }

    /// A loop-invariant variable (defined outside the loop) coerced to a
    /// single intrep inside the loop converts once and is cached — that is a
    /// one-time S100, not the per-iteration S101.
    #[test]
    fn loop_invariant_single_target_downgrades_to_s100() {
        let src = "proc f {} {\n  set data [dict create a 1 b 2]\n  for {set i 0} {$i < 3} {incr i} {\n    set x [lindex $data $i]\n  }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings
            .iter()
            .find(|w| w.command == "lindex" && w.variable == "data");
        assert!(
            w.is_some(),
            "expected lindex shimmer for data: {warnings:?}"
        );
        assert_eq!(
            w.unwrap().code,
            DiagCode::S100,
            "loop-invariant single-target use is one-time (S100): {warnings:?}"
        );
    }

    /// `incr n $step` where `$step` holds a String shimmers on the increment
    /// argument (Tcl coerces it to int).
    #[test]
    fn incr_amount_string_var_shimmers() {
        let cu = CompilationUnit::build_for(
            "set n 0\nset step \"hello\"\nincr n $step",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings
            .iter()
            .find(|w| w.command == "incr" && w.variable == "step");
        assert!(
            w.is_some(),
            "expected incr-amount shimmer for step: {warnings:?}"
        );
        assert_eq!(w.unwrap().from_type, TclType::String);
        assert_eq!(w.unwrap().to_type, TclType::Int);
    }

    /// Array elements collapse onto one SSA symbol (the `(key)` suffix is
    /// stripped before interning), so two individually-stable but different
    /// elements must not use-site shimmer against each other (FP-SH-13):
    /// `set arr(n) 5; set arr(label) "text"; incr arr(n)` must not report
    /// `arr` as "string used with incr" — `arr(n)` is always int.
    #[test]
    fn no_use_site_shimmer_for_array_element_conflation() {
        let cu = CompilationUnit::build_for(
            "proc f {} { set arr(n) 5\n set arr(label) \"text\"\n incr arr(n) }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        assert!(
            warnings.is_empty(),
            "array-element conflation must not use-site shimmer: {warnings:?}"
        );
    }

    /// TP control: the identical shape on a plain scalar still fires — the
    /// array guard must not blanket-silence use-site shimmer.
    /// `string length`/`index`/`range` install the string intrep on their
    /// subject (tclsh-verified: a list becomes `string`) — genuine shimmer —
    /// but a pure byte array short-circuits and keeps its rep
    /// (`transparent_from`), so it must stay silent.
    #[test]
    fn string_subject_shimmers_list_but_not_bytearray() {
        // TP: a List-typed subject is coerced to the string intrep.
        let cu =
            CompilationUnit::build_for("set l [list 1 2 3]\nstring length $l", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        assert!(
            warnings
                .iter()
                .any(|w| w.variable == "l" && w.to_type == TclType::String),
            "list subject of `string length` must shimmer: {warnings:?}"
        );

        // FP guard: a ByteArray-typed subject passes through untouched.
        let cu2 = CompilationUnit::build_for(
            "set ba [binary format c* {200 201}]\nstring length $ba\nstring range $ba 0 1",
            &registry(),
            false,
        );
        let fu2 = cu2.function("::top").unwrap();
        let warnings2 = use_site_shimmers(fu2, &registry());
        assert!(
            !warnings2.iter().any(|w| w.variable == "ba"),
            "byte-array subject is transparent — no shimmer: {warnings2:?}"
        );
    }

    #[test]
    fn use_site_shimmer_still_fires_for_scalar_control() {
        let cu =
            CompilationUnit::build_for("proc f {} { set n hello\n incr n }", &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        assert!(
            warnings
                .iter()
                .any(|w| w.command == "incr" && w.variable == "n"),
            "plain scalar must still use-site shimmer: {warnings:?}"
        );
    }

    /// Variables with Unknown type do not produce false-positive shimmers.
    #[test]
    fn no_shimmer_for_unknown_type() {
        // `set x $other` — type of x is Unknown (other has no known type).
        let cu = CompilationUnit::build_for("set x $other\nlindex $x 0", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        // x has Unknown type; should not produce a shimmer.
        let lindex_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "lindex").collect();
        assert!(
            lindex_shimmers.is_empty(),
            "unexpected shimmer for Unknown type: {lindex_shimmers:?}"
        );
    }

    /// TP: a call through an `interp alias` to a shimmering builtin is still
    /// detected — the registry lookup keys off the resolved
    /// `canonical_command`, not the alias's source spelling. `$x` holds a
    /// *committed* Dict so the list conversion is a genuine shimmer (a pure
    /// string would promote for free through the alias too).
    #[test]
    fn shimmer_detected_through_interp_alias_to_lindex() {
        let cu = CompilationUnit::build_for(
            "interp alias {} myindex {} ::lindex\nset x [dict create a 1 b 2]\nmyindex $x 0",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings.iter().find(|w| w.variable == "x");
        assert!(
            w.is_some(),
            "expected shimmer through interp alias, got: {warnings:?}"
        );
        let w = w.unwrap();
        assert_eq!(w.from_type, TclType::Dict);
        assert_eq!(w.to_type, TclType::List);
        // The message keeps the alias spelling the user wrote, not the
        // resolved canonical target.
        assert_eq!(w.command, "myindex");
    }

    /// Helper for the string-subcommand family tests below: all use-site
    /// warnings for `src`'s `::top`.
    fn warnings_for(src: &str) -> Vec<ShimmerWarning> {
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        use_site_shimmers(fu, &registry())
    }

    /// `string first`/`last` convert BOTH the needle and the haystack to the
    /// string intrep; a pure byte array stays silent (the 9.0-safe
    /// under-approximation — see the registry note). tclsh 8.6.14 probe:
    ///
    /// ```text
    /// string first pat SUBJ   list:  list -> string
    /// string first SUBJ hay   list:  list -> string
    /// string last  pat SUBJ   list:  list -> string
    /// string last  SUBJ hay   list:  list -> string
    /// string first pat SUBJ   bytes: bytearray -> string   (silenced: 9.0
    /// string first SUBJ hay   bytes: bytearray -> string    keeps the rep
    ///                                                       for a both-
    ///                                                       bytearray pair)
    /// ```
    #[test]
    fn string_first_last_needle_and_haystack_shimmer_list_not_bytearray() {
        for sub in ["first", "last"] {
            // TP: a List-typed needle and a List-typed haystack each fire.
            let warnings = warnings_for(&format!(
                "set n [list 1 2]\nset h [list 3 4]\nstring {sub} $n $h",
            ));
            for var in ["n", "h"] {
                assert!(
                    warnings
                        .iter()
                        .any(|w| w.variable == var && w.to_type == TclType::String),
                    "string {sub}: list-typed '{var}' must shimmer to string: {warnings:?}"
                );
            }
            // FP guard: byte-array operands stay silent.
            let warnings = warnings_for(&format!(
                "set n [binary format c* {{200}}]\nset h [binary format c* {{199 200}}]\nstring {sub} $n $h",
            ));
            assert!(
                warnings
                    .iter()
                    .all(|w| w.variable != "n" && w.variable != "h"),
                "string {sub}: byte-array operands must stay silent: {warnings:?}"
            );
        }
    }

    /// `string map` subject: converts to the string intrep from a List AND
    /// from a pure `ByteArray` (no fast path in 8.6 or 9.0 — the subject is
    /// read via `Tcl_GetUnicodeFromObj` unconditionally). tclsh 8.6.14:
    ///
    /// ```text
    /// string map {a b} SUBJ   list:  list -> string
    /// string map {a b} SUBJ   bytes: bytearray -> string
    /// ```
    #[test]
    fn string_map_subject_shimmers_list_and_bytearray() {
        let warnings = warnings_for("set s [list 1 2 3]\nstring map {a b} $s");
        assert!(
            warnings
                .iter()
                .any(|w| w.variable == "s" && w.to_type == TclType::String),
            "list-typed map subject must shimmer to string: {warnings:?}"
        );
        let warnings = warnings_for("set s [binary format c* {200 201}]\nstring map {a b} $s");
        assert!(
            warnings.iter().any(|w| w.variable == "s"
                && w.from_type == TclType::ByteArray
                && w.to_type == TclType::String),
            "byte-array map subject genuinely converts — must fire: {warnings:?}"
        );
    }

    /// `string map` mapping: the dict iteration path keeps a pure dict, and
    /// `TclListObjGetElements` keeps a list — neither may fire. tclsh
    /// 8.6.14 (probe refuting the former `expected: Dict` hint):
    ///
    /// ```text
    /// map mapping list:   list -> list
    /// map mapping dict:   dict -> dict
    /// map mapping string: pure -> list
    /// ```
    #[test]
    fn string_map_mapping_list_and_dict_stay_silent() {
        for mapping in ["[list a b]", "[dict create a b]"] {
            let warnings = warnings_for(&format!("set m {mapping}\nstring map $m xyz"));
            assert!(
                warnings.iter().all(|w| w.variable != "m"),
                "mapping {mapping} keeps its intrep — must stay silent: {warnings:?}"
            );
        }
    }

    /// The `-nocase` option shifts `string map`'s positionals by one; the
    /// leading-option skip keeps the mapping/subject hints aligned, so the
    /// mapping stays silent and the subject still fires.
    #[test]
    fn string_map_nocase_keeps_hint_alignment() {
        let warnings = warnings_for("set m [list a b]\nset s [list 1 2]\nstring map -nocase $m $s");
        assert!(
            warnings.iter().all(|w| w.variable != "m"),
            "-nocase mapping keeps its list intrep — must stay silent: {warnings:?}"
        );
        assert!(
            warnings
                .iter()
                .any(|w| w.variable == "s" && w.to_type == TclType::String),
            "-nocase subject must still shimmer: {warnings:?}"
        );
    }

    /// `string replace`/`reverse` subjects: List converts to the string
    /// intrep, a pure byte array keeps its rep in both 8.6 and 9.0. tclsh
    /// 8.6.14:
    ///
    /// ```text
    /// string replace SUBJ 0 0 Z   list:  list -> string
    /// string replace SUBJ 0 0 Z   bytes: bytearray -> bytearray
    /// string reverse SUBJ         list:  list -> string
    /// string reverse SUBJ         bytes: bytearray -> bytearray
    /// ```
    #[test]
    fn string_replace_reverse_subject_shimmers_list_not_bytearray() {
        for call in ["string replace $s 0 0 Z", "string reverse $s"] {
            let warnings = warnings_for(&format!("set s [list 1 2 3]\n{call}"));
            assert!(
                warnings
                    .iter()
                    .any(|w| w.variable == "s" && w.to_type == TclType::String),
                "{call}: list subject must shimmer: {warnings:?}"
            );
            let warnings = warnings_for(&format!("set s [binary format c* {{200 201}}]\n{call}"));
            assert!(
                warnings.iter().all(|w| w.variable != "s"),
                "{call}: byte-array subject is transparent — silent: {warnings:?}"
            );
        }
    }

    /// Oracle-verified negatives (tclsh 8.6.14: a list subject stays `list`
    /// through every one of these — the subject is read via its string rep
    /// only, dual-ported):
    ///
    /// ```text
    /// string compare SUBJ y / y SUBJ    list -> list
    /// string equal SUBJ y               list -> list
    /// string match p* SUBJ / SUBJ s     list -> list
    /// string repeat SUBJ 2              list -> list
    /// string tolower|toupper|totitle    list -> list   (also with indices)
    /// string trim|trimleft|trimright    list -> list
    /// string wordend|wordstart SUBJ 1   list -> list   (9.0 converts — see
    ///                                                   the registry note)
    /// string cat SUBJ / SUBJ y          list -> list
    /// ```
    #[test]
    fn dual_ported_string_subcommands_stay_silent_for_list_subject() {
        for call in [
            "string compare $s y",
            "string compare y $s",
            "string equal $s y",
            "string match p* $s",
            "string match $s y",
            "string repeat $s 2",
            "string tolower $s",
            "string toupper $s 0 1",
            "string totitle $s",
            "string trim $s",
            "string trimleft $s x",
            "string trimright $s",
            "string wordend $s 1",
            "string wordstart $s 1",
            "string cat $s y",
        ] {
            let warnings = warnings_for(&format!("set s [list 1 2 3]\n{call}"));
            assert!(
                warnings.iter().all(|w| w.variable != "s"),
                "{call}: dual-ported read keeps the list intrep — silent: {warnings:?}"
            );
        }
    }

    /// TN control: an alias to a *non*-shimmering command (`puts`) must not
    /// spuriously fire just because the alias itself resolved.
    #[test]
    fn no_shimmer_through_interp_alias_to_non_shimmering_command() {
        let cu = CompilationUnit::build_for(
            "interp alias {} say {} ::puts\nset x hello\nsay $x",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        assert!(
            warnings.iter().all(|w| w.variable != "x"),
            "unexpected shimmer through non-shimmering alias: {warnings:?}"
        );
    }

    /// Issue #1720: `regexp -all -inline` returns the matched substrings as a
    /// list, so iterating the result converts nothing.  The reporter's own
    /// code, trimmed: it drew "variable 'vinfo' has int intrep but 'foreach'
    /// expects list (argument 1)".
    #[test]
    fn no_shimmer_iterating_a_regexp_inline_result() {
        let src = "proc f {s} {\n\
                   set vinfo [regexp -all -inline {(?:^|\\s)(\\d+)(?:\\.(\\d+))?} $s]\n\
                   foreach {match v1 v2} $vinfo { lappend out $v1$v2 }\n\
                   }";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let on_vinfo: Vec<_> = warnings.iter().filter(|w| w.variable == "vinfo").collect();
        assert!(
            on_vinfo.is_empty(),
            "regexp -inline returns a list — iterating it is not a shimmer: {on_vinfo:?}"
        );
    }

    /// The control for the case above: a bare `regexp` really does return a
    /// match flag, so consuming it as a list is a genuine conversion.  Proves
    /// the per-form typing reads the switches rather than blanket-silencing
    /// `regexp`.
    #[test]
    fn shimmer_still_detected_iterating_a_bare_regexp_result() {
        let src = "proc f {s} {\n\
                   set m [regexp {(\\d+)} $s]\n\
                   foreach v $m { lappend out $v }\n\
                   }";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let w = warnings.iter().find(|w| w.variable == "m");
        assert!(
            w.is_some(),
            "an int match flag used as a list must still shimmer: {warnings:?}"
        );
        assert_eq!(w.unwrap().from_type, TclType::Int);
        assert_eq!(w.unwrap().to_type, TclType::List);
    }

    /// The same defect on `lsearch`: `-all` returns the list of matching
    /// indices, not one index.
    #[test]
    fn no_shimmer_iterating_an_lsearch_all_result() {
        let src = "proc f {l} {\n\
                   set hits [lsearch -all $l a]\n\
                   foreach i $hits { lappend out $i }\n\
                   }";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let on_hits: Vec<_> = warnings.iter().filter(|w| w.variable == "hits").collect();
        assert!(
            on_hits.is_empty(),
            "lsearch -all returns a list of indices: {on_hits:?}"
        );
    }

    /// And on `regsub`: with no `varName` the result is the substituted
    /// string, so measuring its length converts nothing.
    #[test]
    fn no_shimmer_measuring_a_varname_less_regsub_result() {
        let src = "proc f {s} {\n\
                   set out [regsub -all a $s b]\n\
                   set n [string length $out]\n\
                   }";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = use_site_shimmers(fu, &registry());
        let on_out: Vec<_> = warnings.iter().filter(|w| w.variable == "out").collect();
        assert!(
            on_out.is_empty(),
            "a varName-less regsub returns the substituted string: {on_out:?}"
        );
    }
}
