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

//! Inter-procedural taint summary solver.
//!
//! Drives a per-proc return-taint summary plus a parameter entry-taint
//! worklist.
//!
//! The intra-procedural pass ([`crate::taint::propagate_taints`]) already
//! models a single return-passthrough parameter; this module adds two
//! more pieces:
//!
//! 1. **Colour-aware return summaries** — [`ProcTaintSummary`] records, per
//!    procedure, the taint a call returns as a function of which parameters
//!    are tainted (and with which colour basis). Computed by a monotone
//!    fixpoint ([`solve_interprocedural_taints`], the summary worklist).
//! 2. **Parameter entry-taint** — a worklist that flows tainted call
//!    arguments into the callee's parameters, so a tainted argument reaching
//!    a sink *inside* the callee is warned (cross-proc entry-taint).
//!
//! The result ([`InterprocTaintResult`]) carries `top_taints` (top-level)
//! and `proc_taints` (per procedure), which the warning consumers
//! (`compiler_checks::run_all_checks`, the `dataflow` view) read in place of
//! the bare per-function [`crate::compilation_unit::FunctionUnit::taints`].

use std::collections::{HashMap, HashSet, VecDeque};

use tcl_lexer::{Lexer, SourceMap, TokenType};
use tcl_registry::{Arity, CommandRegistry};

use crate::cfg::Terminator;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::interprocedural::resolve_call_target;
use crate::naming::normalise_var_name;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
#[cfg(test)]
use crate::taint::local_instance_classes;
use crate::taint::{
    LocalInstanceClasses, TaintColour, TaintCtx, TaintGraph, TaintLattice,
    instance_classes_for_function, propagate_taints, word_taint,
};
use crate::value_shapes::parse_command_substitution;

// Basis lattices

/// Closed vocabulary of taint-colour bases. The per-parameter return
/// scenarios are computed by seeding the parameter with each basis lattice in
/// turn, in [`TaintBasis::ALL`] order (which fixes `return_by_param_basis`'s
/// index-to-basis mapping).
///
/// Purely an internal solver vocabulary: never serialised, never a CLI/`.tclspec`
/// spelling, so no `FromStr`/`Display` boundary is needed (unlike the
/// dialect-name discipline in #1405).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TaintBasis {
    Generic,
    Path,
    NonDash,
    CrlfFree,
    ShellAtom,
    ListCanonical,
    RegexLiteral,
    PathNormalised,
    PathBounded,
    HeaderTokenSafe,
    HtmlEscaped,
    UrlEncoded,
    Ip,
    Port,
    Fqdn,
    PathJoined,
    Channel,
}

impl TaintBasis {
    /// Ordered taint-colour bases, fixing the index used by
    /// `return_by_param_basis`.
    const ALL: [TaintBasis; 17] = [
        TaintBasis::Generic,
        TaintBasis::Path,
        TaintBasis::NonDash,
        TaintBasis::CrlfFree,
        TaintBasis::ShellAtom,
        TaintBasis::ListCanonical,
        TaintBasis::RegexLiteral,
        TaintBasis::PathNormalised,
        TaintBasis::PathBounded,
        TaintBasis::HeaderTokenSafe,
        TaintBasis::HtmlEscaped,
        TaintBasis::UrlEncoded,
        TaintBasis::Ip,
        TaintBasis::Port,
        TaintBasis::Fqdn,
        TaintBasis::PathJoined,
        TaintBasis::Channel,
    ];

    /// The taint lattice this basis seeds a parameter with.
    fn lattice(self) -> TaintLattice {
        let t = TaintColour::TAINTED;
        let colour = match self {
            TaintBasis::Generic => t,
            TaintBasis::Path => t | TaintColour::PATH_PREFIXED,
            TaintBasis::NonDash => t | TaintColour::NON_DASH_PREFIXED,
            TaintBasis::CrlfFree => t | TaintColour::CRLF_FREE,
            TaintBasis::ShellAtom => t | TaintColour::SHELL_ATOM,
            TaintBasis::ListCanonical => t | TaintColour::LIST_CANONICAL,
            TaintBasis::RegexLiteral => t | TaintColour::REGEX_LITERAL,
            TaintBasis::PathNormalised => t | TaintColour::PATH_NORMALISED,
            // PATH_BOUNDED is set alongside PATH_NORMALISED (the basis entry
            // is reserved for future branch-dependent refinement).
            TaintBasis::PathBounded => t | TaintColour::PATH_NORMALISED | TaintColour::PATH_BOUNDED,
            TaintBasis::HeaderTokenSafe => t | TaintColour::HEADER_TOKEN_SAFE,
            TaintBasis::HtmlEscaped => t | TaintColour::HTML_ESCAPED,
            TaintBasis::UrlEncoded => t | TaintColour::URL_ENCODED,
            TaintBasis::Ip => t | TaintColour::IP_ADDRESS,
            TaintBasis::Port => t | TaintColour::PORT,
            TaintBasis::Fqdn => t | TaintColour::FQDN,
            TaintBasis::PathJoined => t | TaintColour::PATH_JOINED,
            TaintBasis::Channel => t | TaintColour::CHANNEL,
        };
        TaintLattice { colours: colour }
    }
}

/// Bases whose lattice colour intersects `taint`'s colour, excluding
/// [`TaintBasis::Generic`] (falling back to `[Generic]` when none match).
fn basis_names_for_taint(taint: TaintLattice) -> Vec<TaintBasis> {
    if !taint.is_tainted() {
        return Vec::new();
    }
    let mut names: Vec<TaintBasis> = TaintBasis::ALL
        .into_iter()
        .filter(|basis| {
            *basis != TaintBasis::Generic && basis.lattice().colours.intersects(taint.colours)
        })
        .collect();
    if names.is_empty() {
        names.push(TaintBasis::Generic);
    }
    names
}

// Proc taint summary

/// Context-insensitive return-taint transfer summary for one procedure.
///
/// `Hash` lets the LSP db intern a procedure's direct-callee summaries into the
/// `SummaryDepsKey` of its memoised `proc_summary_cascade` query — a body edit
/// that leaves a callee's summary unchanged keeps the key, and
/// the caller's inference is a cache hit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProcTaintSummary {
    /// Fully-qualified procedure name.
    pub qualified_name: String,
    /// Parameter names in declaration order.
    pub params: Vec<String>,
    /// Declared arity.
    pub arity: Arity,
    /// Taint the call returns when no parameter is tainted.
    pub return_base: TaintLattice,
    /// Per-parameter return taint, indexed by [`TaintBasis::ALL`]: for each
    /// `(param, [taint_per_basis])`, `taint_per_basis[i]` is the taint the
    /// call returns when `param` is seeded with `TaintBasis::ALL[i]`'s lattice.
    pub return_by_param_basis: Vec<(String, Vec<TaintLattice>)>,
}

impl ProcTaintSummary {
    /// An untainted seed summary (every scenario clean).
    ///
    /// Public so the LSP db's memoised `proc_summary_cascade` can reconstruct the
    /// *whole-module* seeded map the worklist passes to `infer_proc_summary` — a
    /// resolved callee must map to a (clean) summary, not be absent, or
    /// `propagate_taints` falls through to its conservative bare-argument join and
    /// over-taints (SRV-INCREMENTAL 2b).
    #[must_use]
    pub fn untainted(qname: &str, params: &[String]) -> Self {
        let clean = TaintLattice::clean();
        Self {
            qualified_name: qname.to_owned(),
            params: params.to_vec(),
            arity: crate::interprocedural::arity_from_names(params),
            return_base: clean,
            return_by_param_basis: params
                .iter()
                .map(|p| (p.clone(), vec![clean; TaintBasis::ALL.len()]))
                .collect(),
        }
    }

    /// The return taint when `param` is tainted with `basis`.
    fn scenario(&self, param: &str, basis_idx: usize) -> TaintLattice {
        for (p_name, values) in &self.return_by_param_basis {
            if p_name == param && basis_idx < values.len() {
                return values[basis_idx];
            }
        }
        TaintLattice::clean()
    }
}

/// Apply a procedure's return-taint summary to a tuple of argument taints.
///
/// Bind each parameter (handling
/// a trailing variadic `args`), then for every tainted bound argument join the
/// return scenario for each of its colour bases onto the base return taint.
#[must_use]
pub fn apply_proc_return_summary(
    summary: &ProcTaintSummary,
    arg_taints: &[TaintLattice],
) -> TaintLattice {
    let n = u16::try_from(arg_taints.len()).unwrap_or(u16::MAX);
    if !summary.arity.accepts(n) {
        return TaintLattice::clean();
    }

    let mut bound: Vec<(String, TaintLattice)> = Vec::with_capacity(summary.params.len());
    if summary.params.last().is_some_and(|p| p == "args") {
        let fixed = summary.params.len() - 1;
        for (i, p) in summary.params.iter().take(fixed).enumerate() {
            let t = arg_taints
                .get(i)
                .copied()
                .unwrap_or_else(TaintLattice::clean);
            bound.push((p.clone(), t));
        }
        let mut rest = TaintLattice::clean();
        for t in arg_taints.iter().skip(fixed) {
            rest = rest.join(*t);
        }
        bound.push(("args".to_owned(), rest));
    } else {
        for (i, p) in summary.params.iter().enumerate() {
            let t = arg_taints
                .get(i)
                .copied()
                .unwrap_or_else(TaintLattice::clean);
            bound.push((p.clone(), t));
        }
    }

    let mut out = summary.return_base;
    for (p, t) in &bound {
        if !t.is_tainted() {
            continue;
        }
        for basis in basis_names_for_taint(*t) {
            let idx = TaintBasis::ALL
                .iter()
                .position(|b| *b == basis)
                .unwrap_or(0);
            out = out.join(summary.scenario(p, idx));
        }
    }
    out
}

// Return-taint collection

/// Build a `symbol -> version` use map for `text` from a block's exit
/// versions, scanning the word for `$var` references. A scanned name not
/// interned in `ssa` is not an SSA variable here, so it is dropped.
fn word_uses_from_versions(
    text: &str,
    versions: &HashMap<Symbol, u32>,
    ssa: &SsaFunction,
    config: tcl_lexer::LexerConfig,
) -> HashMap<Symbol, u32> {
    let mut uses: HashMap<Symbol, u32> = HashMap::new();
    let source_map = SourceMap::new(text);
    let Ok(tokens) = Lexer::with_config(text, config).tokenise_all() else {
        return uses;
    };
    for tok in tokens {
        if tok.kind != TokenType::Var {
            continue;
        }
        let name = normalise_var_name(source_map.text(tok.span));
        if name.is_empty() {
            continue;
        }
        let Some(sym) = ssa.var_symbol(name) else {
            continue;
        };
        let ver = versions.get(&sym).copied().unwrap_or(0);
        uses.insert(sym, ver);
    }
    uses
}

/// Whether `fu`'s return taint is a **constant** — the same value whatever
/// taint map [`collect_return_taint`] is handed — so the whole summary can be
/// produced without running the dataflow at all (issue #1187).
///
/// [`collect_return_taint`] joins `word_taint` over the value word of every
/// executable block's `Return` terminator, and `word_taint` reads the taint map
/// only through `var_taint`, which is reachable only from its three
/// substitution branches: a pure `$var` reference, a `[cmd …]` substitution, and
/// the interpolated-word scan.  All three require the word to contain a `$` or a
/// `[`; a word with neither falls through to `TaintLattice::clean()` without
/// consulting the map.  So a function whose executable returns are all
/// value-less or all substitution-free returns `clean` unconditionally, and
/// every `(parameter, basis)` scenario — plus the clean base — is `clean` too.
///
/// This is the cheap end of the pruning ladder and by far the most common case
/// in real Tcl: a procedure that returns nothing, a literal, or a braced
/// constant does no work here at all instead of `1 + 17P` whole-CFG solves.
fn return_taint_is_constant(fu: &FunctionUnit) -> bool {
    fu.sccp.executable_blocks.iter().all(|bn| {
        let Some(block) = fu.cfg.blocks.get(bn) else {
            return true;
        };
        match &block.terminator {
            Some(Terminator::Return {
                value: Some(value), ..
            }) => !value.contains('$') && !value.contains('['),
            _ => true,
        }
    })
}

/// Join the taint of every executable block's return value.
fn collect_return_taint(
    fu: &FunctionUnit,
    taints: &HashMap<ValueKey, TaintLattice>,
    ctx: TaintCtx<'_>,
) -> TaintLattice {
    let mut ret = TaintLattice::clean();
    for bn in &fu.sccp.executable_blocks {
        let Some(block) = fu.cfg.blocks.get(bn) else {
            continue;
        };
        let Some(Terminator::Return {
            span,
            value: Some(value),
            ..
        }) = &block.terminator
        else {
            continue;
        };
        let Some(ssa_block) = fu.ssa.blocks.get(bn) else {
            continue;
        };
        let uses =
            word_uses_from_versions(value, &ssa_block.exit_versions, &fu.ssa, ctx.lexer_config());
        ret = ret.join(word_taint(
            value,
            &uses,
            taints,
            ctx.at(span.map_or(0, tcl_lexer::Span::start)),
        ));
    }
    ret
}

/// Run intra-procedural taint propagation over `fu` with the given entry
/// taints and summaries. A thin wrapper threading the common arguments.
///
/// `graph` carries the CFG-derived indices, built once by the caller: one
/// summary inference runs this `1 + params × TaintBasis::ALL` times and the
/// indices are identical across all of them (issue #1251).
#[allow(clippy::too_many_arguments)]
fn run_propagation(
    graph: &TaintGraph<'_>,
    fu: &FunctionUnit,
    instance_classes: &LocalInstanceClasses,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    dialect: Option<&tcl_dialect::DialectProfile>,
    param_taints: Option<&HashMap<String, TaintLattice>>,
    summaries: &HashMap<String, ProcTaintSummary>,
) -> HashMap<ValueKey, TaintLattice> {
    propagate_taints(
        graph,
        registry,
        Some(&fu.rendered_props),
        interproc,
        dialect,
        param_taints,
        Some(summaries),
        instance_classes,
    )
}

/// Build a [`TaintCtx`] for evaluating return-value words of `fu`.
fn return_ctx<'a>(
    fu: &'a FunctionUnit,
    registry: &'a CommandRegistry,
    interproc: Option<&'a crate::interprocedural::InterproceduralAnalysis>,
    dialect: Option<&'a tcl_dialect::DialectProfile>,
    known: &'a HashSet<String>,
    summaries: &'a HashMap<String, ProcTaintSummary>,
    instance_classes: &'a LocalInstanceClasses,
) -> TaintCtx<'a> {
    TaintCtx {
        registry,
        ssa: &fu.ssa,
        interproc,
        known_procs: Some(known),
        caller_qname: Some(fu.ssa.name.as_str()),
        dialect,
        taint_summaries: Some(summaries),
        instance_classes: Some(instance_classes),
        source_position: None,
    }
}

/// Per-proc summary-inference callback driven by the summary-fixpoint worklist
/// ([`converge_summaries_with`]). Receives `(qname, params, fu, known,
/// summaries)` and returns the procedure's inferred [`ProcTaintSummary`] under
/// the *current* summaries. The default is [`infer_proc_summary`]; the LSP db
/// injects a salsa-memoised variant (SRV-INCREMENTAL 2b) that returns an
/// unchanged proc's summary from cache (keyed on its offset-0 body + its
/// direct callees' summaries) instead of re-running the propagation here.
pub type InferProcSummaryFn<'a> = dyn FnMut(
        &str,
        &[String],
        &FunctionUnit,
        &HashSet<String>,
        &HashMap<String, ProcTaintSummary>,
    ) -> ProcTaintSummary
    + 'a;

/// Infer a procedure's return-taint summary under the current summaries.
///
/// Exposed (with [`InferProcSummaryFn`]) so the LSP db can both (a) re-run the
/// real inference inside its memoised `proc_summary_cascade` query and (b) keep
/// the debug fixpoint guard validating against the *genuine* result.
///
/// # Cost
///
/// The summary is a transfer function: a clean base plus, for each parameter, a
/// return taint per [`TaintBasis::ALL`] entry.  Computed naively that is
/// `1 + TaintBasis::ALL.len() * params.len()` — `1 + 17P` — complete dataflow
/// solves over the procedure's CFG, and this is the dominant cost of the whole
/// interprocedural taint pass (about 80% of `run_all_checks` on tcllib's
/// `practcl.tcl`).
///
/// Two prunes cut that down (issue #1187).  Both are **proofs that the solve
/// would return a value already in hand**, not approximations, so every summary
/// stays bit-identical to the unpruned one — which is what lets the debug
/// fixpoint guard and the `compiler_check` corpus differential keep validating
/// this function unchanged:
///
/// 1. [`return_taint_is_constant`] — the return taint cannot depend on the
///    taint map, so the whole summary is the clean one: **0** solves instead of
///    `1 + 17P`.
/// 2. An un-interned parameter — seeding a name the body never reads leaves the
///    initial taint map bit-identical to the base run's, so the scenario *is*
///    `return_base`: **0** solves instead of 17, per such parameter.
///
/// What remains is `1 + 17 × (parameters that are actually read, in a procedure
/// whose return value is substitution-bearing)`.  Collapsing that last `17×`
/// needs a genuinely different representation — one multi-colour symbolic
/// traversal carrying a per-`(parameter, basis)` dependency bitset — which
/// changes what the solver computes rather than skipping work it can prove
/// redundant, so it is not attempted here.
#[must_use]
// Both lints are forced by the public callback contract and the shared
// `TaintCtx` type, not code smell:
// * `too_many_arguments`: the list mirrors `InferProcSummaryFn` plus the
//   closure-captured `(registry, interproc, dialect)`, and `tcl-lsp-db` calls
//   this exact signature across the crate boundary — bundling breaks it.
// * `implicit_hasher`: `known` / `summaries` are stored verbatim into
//   `TaintCtx` (whose fields fix the default hasher) and the value is reached
//   via `InferProcSummaryFn`'s concrete signature, so generalising over the
//   hasher is impossible without making `TaintCtx` generic crate-wide.
#[allow(clippy::too_many_arguments, clippy::implicit_hasher)]
pub fn infer_proc_summary(
    qname: &str,
    params: &[String],
    fu: &FunctionUnit,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    dialect: Option<&tcl_dialect::DialectProfile>,
    known: &HashSet<String>,
    summaries: &HashMap<String, ProcTaintSummary>,
) -> ProcTaintSummary {
    // Prune 1 — the return taint does not depend on the taint map at all, so
    // neither the clean base nor any of the `17P` seeded scenarios needs a
    // solve.  Every scenario is `clean`, which is exactly what
    // `ProcTaintSummary::untainted` builds.  Checked before the index build
    // below, so a constant-return procedure costs nothing at all.
    if return_taint_is_constant(fu) {
        return ProcTaintSummary::untainted(qname, params);
    }

    // One index build for the baseline propagation and every scenario below.
    let graph = TaintGraph::new(&fu.cfg, &fu.ssa, &fu.sccp);
    let instance_classes = instance_classes_for_function(&fu.cfg, registry, interproc, true);
    let base_taints = run_propagation(
        &graph,
        fu,
        &instance_classes,
        registry,
        interproc,
        dialect,
        None,
        summaries,
    );
    let ctx = return_ctx(
        fu,
        registry,
        interproc,
        dialect,
        known,
        summaries,
        &instance_classes,
    );
    let return_base = collect_return_taint(fu, &base_taints, ctx);

    let mut by_param_basis: Vec<(String, Vec<TaintLattice>)> = Vec::with_capacity(params.len());
    for param in params {
        // Prune 2 — a parameter the body never reads is not interned in the
        // SSA, and `seed_entry_taints` skips an un-interned name.  Seeding it
        // therefore produces a *bit-identical* initial map to the clean base
        // run, hence a bit-identical fixpoint and a bit-identical return taint.
        // Rather than run 15 solves to rediscover `return_base` 15 times, take
        // it directly.  (This is a proof, not an approximation: the two runs
        // differ in no input.)
        if fu.ssa.var_symbol(param).is_none() {
            by_param_basis.push((param.clone(), vec![return_base; TaintBasis::ALL.len()]));
            continue;
        }
        let mut scenario_values: Vec<TaintLattice> = Vec::with_capacity(TaintBasis::ALL.len());
        for basis in TaintBasis::ALL {
            let mut seed: HashMap<String, TaintLattice> = HashMap::new();
            seed.insert(param.clone(), basis.lattice());
            let seeded = run_propagation(
                &graph,
                fu,
                &instance_classes,
                registry,
                interproc,
                dialect,
                Some(&seed),
                summaries,
            );
            scenario_values.push(collect_return_taint(fu, &seeded, ctx));
        }
        by_param_basis.push((param.clone(), scenario_values));
    }

    ProcTaintSummary {
        qualified_name: qname.to_owned(),
        params: params.to_vec(),
        arity: crate::interprocedural::arity_from_names(params),
        return_base,
        return_by_param_basis: by_param_basis,
    }
}

// Call-flow resolution (entry-taint worklist)

/// Resolve the (callee, arg-taints) flows a function makes to known procs.
#[allow(clippy::too_many_arguments)]
fn resolve_call_flows(
    fu: &FunctionUnit,
    taints: &HashMap<ValueKey, TaintLattice>,
    instance_classes: &LocalInstanceClasses,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    dialect: Option<&tcl_dialect::DialectProfile>,
    known: &HashSet<String>,
    summaries: &HashMap<String, ProcTaintSummary>,
) -> Vec<(String, Vec<TaintLattice>)> {
    let ctx = return_ctx(
        fu,
        registry,
        interproc,
        dialect,
        known,
        summaries,
        instance_classes,
    );
    let caller_qname = fu.ssa.name.as_str();
    let mut flows: Vec<(String, Vec<TaintLattice>)> = Vec::new();

    for bn in &fu.sccp.executable_blocks {
        let Some(block) = fu.cfg.blocks.get(bn) else {
            continue;
        };
        let Some(ssa_block) = fu.ssa.blocks.get(bn) else {
            continue;
        };
        let stmt_count = block.statements.len().min(ssa_block.statements.len());
        for idx in 0..stmt_count {
            let stmt = &block.statements[idx];
            let ssa_stmt = &ssa_block.statements[idx];

            let resolved: Option<(String, Vec<String>)> = match stmt {
                crate::ir::Statement::Call { command, args, .. } => {
                    Some((command.clone(), args.clone()))
                }
                crate::ir::Statement::AssignValue { value, .. } => {
                    parse_command_substitution(value)
                }
                _ => None,
            };
            let Some((cmd_name, cmd_args)) = resolved else {
                continue;
            };

            let Some(callee) = resolve_call_target(&cmd_name, &cmd_args, caller_qname, known)
            else {
                continue;
            };
            let Some(summary) = summaries.get(&callee) else {
                continue;
            };
            let n = u16::try_from(cmd_args.len()).unwrap_or(u16::MAX);
            if !summary.arity.accepts(n) {
                continue;
            }

            let statement_ctx = ctx.at(stmt.span().start());
            let arg_taints: Vec<TaintLattice> = cmd_args
                .iter()
                .map(|arg| word_taint(arg, &ssa_stmt.uses, taints, statement_ctx))
                .collect();
            flows.push((callee, arg_taints));
        }
    }

    flows
}

/// The known-proc callees a function references — every call target
/// [`infer_proc_summary`] resolves while propagating taints, scanned from the
/// same executable CFG blocks (`Statement::Call` + command-substitution
/// `AssignValue`) as [`resolve_call_flows`].
///
/// Unlike [`crate::interprocedural::ProcSummary::direct_calls`] — which misses a
/// call buried in a nested command substitution under a dynamic command (e.g.
/// `symbolNodeOf` in `[$t get [symbolNodeOf …] …]`) — this captures exactly the
/// callee summaries the inference reads.  The LSP summary-cascade memo keys on
/// this complete set so its reconstructed dependency context matches the
/// whole-module solve (and the debug fixpoint guard) instead of seeding a missed
/// callee clean and under-tainting the result.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn resolved_callees(fu: &FunctionUnit, known: &HashSet<String>) -> Vec<String> {
    let caller_qname = fu.ssa.name.as_str();
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for bn in &fu.sccp.executable_blocks {
        let Some(block) = fu.cfg.blocks.get(bn) else {
            continue;
        };
        for stmt in &block.statements {
            let resolved: Option<(String, Vec<String>)> = match stmt {
                crate::ir::Statement::Call { command, args, .. } => {
                    Some((command.clone(), args.clone()))
                }
                crate::ir::Statement::AssignValue { value, .. } => {
                    parse_command_substitution(value)
                }
                _ => None,
            };
            let Some((cmd_name, cmd_args)) = resolved else {
                continue;
            };
            if let Some(callee) = resolve_call_target(&cmd_name, &cmd_args, caller_qname, known)
                && seen.insert(callee.clone())
            {
                out.push(callee);
            }
        }
    }
    out.sort();
    out
}

/// Known-proc callees reached through a **command substitution** `[name …]`
/// embedded in a word (return value, argument, expression operand) of `caller`'s
/// body — the summaries [`infer_proc_summary`] reads when it evaluates those
/// words (`word_taint` recurses into nested substitutions), which neither
/// `direct_calls` nor the CFG-statement scan of [`resolved_callees`] captures
/// (e.g. `symbolNodeOf` in `return [$t get [symbolNodeOf …] …]`).
///
/// A deliberately conservative source scan: every `[` opens a candidate
/// substitution whose head word is resolved against `known` (a `$`-led dynamic
/// head is skipped).  It over-approximates (a `[` in a string/comment, an escaped
/// `\[`) — sound, since an unread callee summary only widens the memo key, never
/// changes the inferred result — but never misses a real `[name …]` head, so the
/// cascade memo sees every summary the whole-module solve does.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn command_subst_callees(
    body_source: &str,
    caller_qname: &str,
    known: &HashSet<String>,
) -> Vec<String> {
    let bytes = body_source.as_bytes();
    let is_word = |b: u8| !matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'[' | b']' | b';' | b'\\');
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'[' {
            continue;
        }
        // Skip leading whitespace after the bracket, then read the head word.
        let mut j = i + 1;
        while j < bytes.len() && matches!(bytes[j], b' ' | b'\t' | b'\r' | b'\n') {
            j += 1;
        }
        let start = j;
        while j < bytes.len() && is_word(bytes[j]) {
            j += 1;
        }
        if start == j {
            continue;
        }
        let Ok(head) = std::str::from_utf8(&bytes[start..j]) else {
            continue;
        };
        if head.starts_with('$') {
            continue;
        }
        if let Some(callee) = resolve_call_target(head, &[], caller_qname, known)
            && seen.insert(callee.clone())
        {
            out.push(callee);
        }
    }
    out.sort();
    out
}

// Solver

/// Result of the interprocedural taint solve. Summaries are dropped — the
/// warning consumers only read `top_taints` / `proc_taints`.
///
/// `PartialEq` lets the LSP db return this from the memoised `proc_taint_solve`
/// salsa query (early-cutoff via salsa's update-fallback: a re-solve that
/// produces the same taints backdates, waking no downstream check).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InterprocTaintResult {
    /// Taints for the top-level script.
    pub top_taints: HashMap<ValueKey, TaintLattice>,
    /// Per-procedure taints, keyed by qualified name.
    pub proc_taints: HashMap<String, HashMap<ValueKey, TaintLattice>>,
}

impl InterprocTaintResult {
    /// The solved taints for `name`, falling back to `fallback` (the bare
    /// per-function taints) when the function was not part of the solve.
    #[must_use]
    pub fn taints_for<'a>(
        &'a self,
        name: &str,
        fallback: &'a HashMap<ValueKey, TaintLattice>,
    ) -> &'a HashMap<ValueKey, TaintLattice> {
        if name == "::top" {
            return &self.top_taints;
        }
        self.proc_taints.get(name).unwrap_or(fallback)
    }
}

/// Update a callee's accumulated entry taints with an incoming argument tuple.
/// Returns `true` when any parameter's entry taint grew.
fn update_entry(
    summaries: &HashMap<String, ProcTaintSummary>,
    entry_taints: &mut HashMap<String, HashMap<String, TaintLattice>>,
    callee: &str,
    args: &[TaintLattice],
) -> bool {
    let Some(summary) = summaries.get(callee) else {
        return false;
    };
    let entry = entry_taints.entry(callee.to_owned()).or_default();
    let mut changed = false;

    if summary.params.last().is_some_and(|p| p == "args") {
        let fixed = summary.params.len() - 1;
        for (i, param) in summary.params.iter().take(fixed).enumerate() {
            let incoming = args.get(i).copied().unwrap_or_else(TaintLattice::clean);
            let slot = entry
                .entry(param.clone())
                .or_insert_with(TaintLattice::clean);
            let merged = slot.join(incoming);
            if merged != *slot {
                *slot = merged;
                changed = true;
            }
        }
        let mut rest = TaintLattice::clean();
        for t in args.iter().skip(fixed) {
            rest = rest.join(*t);
        }
        let slot = entry
            .entry("args".to_owned())
            .or_insert_with(TaintLattice::clean);
        let merged = slot.join(rest);
        if merged != *slot {
            *slot = merged;
            changed = true;
        }
        return changed;
    }

    for (i, param) in summary.params.iter().enumerate() {
        let incoming = args.get(i).copied().unwrap_or_else(TaintLattice::clean);
        let slot = entry
            .entry(param.clone())
            .or_insert_with(TaintLattice::clean);
        let merged = slot.join(incoming);
        if merged != *slot {
            *slot = merged;
            changed = true;
        }
    }
    changed
}

/// Run the per-procedure return-taint summary fixpoint to convergence — the
/// dominant cost of [`solve_interprocedural_taints`] (~95% of `run_all_checks`
/// on a large file). `infer_proc_summary(P)` is a pure function of `P`'s body and
/// its *callees'* summaries, so this is driven by a dirty set: a procedure is
/// re-inferred only once one of its direct callees' summaries changed on the
/// previous pass (`callers[Q]` = the procedures that directly call `Q`, from the
/// interprocedural call graph). The lattice is monotone, so this converges to the
/// same fixpoint a full round-robin would — guarded in debug by a final
/// round-robin pass and by the `compiler_check` corpus differential. Without a
/// call graph every procedure is re-queued, exactly reproducing the round-robin.
///
/// Split out of [`solve_interprocedural_taints`] so the cheap entry-taint worklist
/// stays separate, and so this — the expensive, per-procedure-memoisable phase —
/// can be served by a salsa-memoised variant (SRV-INCREMENTAL 2b): the per-proc
/// `infer` step is injectable via [`InferProcSummaryFn`], so the LSP db can plug
/// in a `proc_summary_cascade` query keyed on each proc's offset-0 body + its
/// callees' summaries. The worklist itself (monotone convergence, the call-graph
/// dirty-set, mutual-recursion handling) is unchanged; only `infer` is redirected.
/// The debug-only fixpoint guard deliberately re-runs the **real**
/// [`infer_proc_summary`] (not `infer_fn`), so it validates the worklist's
/// convergence *and* an injected memo's correctness at once — a stale memo entry
/// trips the same assertion a missed call-graph edge would.
///
/// Append `val` to `map[key]`, deduplicated — the caller-edge insert for the
/// reverse call graph in [`converge_summaries_with`].
fn push_unique_edge<'a>(map: &mut HashMap<&'a str, Vec<&'a str>>, key: &'a str, val: &'a str) {
    let entry = map.entry(key).or_default();
    if !entry.contains(&val) {
        entry.push(val);
    }
}

/// `registry` and `dialect` feed only the debug-only round-robin guard below, so
/// in a release build (where `debug_assertions` is off and the guard is compiled
/// out) they are genuinely unused.
#[cfg_attr(not(debug_assertions), allow(unused_variables))]
pub fn converge_summaries_with(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    dialect: Option<&tcl_dialect::DialectProfile>,
    infer_fn: &mut InferProcSummaryFn<'_>,
) -> HashMap<String, ProcTaintSummary> {
    let mut proc_names: Vec<&String> = cu.ir_module.procedures.keys().collect();
    proc_names.sort();

    // Seed summaries (all scenarios untainted).
    let mut summaries: HashMap<String, ProcTaintSummary> = HashMap::new();
    for qname in &proc_names {
        let proc = &cu.ir_module.procedures[*qname];
        summaries.insert(
            (*qname).clone(),
            ProcTaintSummary::untainted(qname, &proc.params),
        );
    }
    let known: HashSet<String> = summaries.keys().cloned().collect();

    // `callers[Q]` = procedures that directly call `Q`; when `Q`'s summary changes
    // its callers are re-queued. Without a call graph, re-queue everything.
    //
    // Two sources, unioned (issue #1187).  `InterproceduralAnalysis::direct_calls`
    // is the declared call graph, but it misses a callee reached through a
    // command substitution the analyser recorded as a plain value —
    // `symbolNodeOf` in `set n [$t get [symbolNodeOf …] …]`, or a self-call
    // inside `[expr {[fib …]}]`.  [`resolved_callees`] scans the very CFG
    // statements the inference resolves calls from, so it supplies exactly the
    // edges `direct_calls` drops there.  A missed edge is not a wrong answer —
    // the completion sweep below still reaches the true least fixpoint — but it
    // costs a whole extra round-robin round to discover, which is the expensive
    // thing this solve does.
    let callers: Option<HashMap<&str, Vec<&str>>> = interproc.map(|ia| {
        let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
        for (caller, summary) in &ia.procedures {
            for callee in &summary.direct_calls {
                push_unique_edge(&mut map, callee.as_str(), caller.as_str());
            }
        }
        for qname in &proc_names {
            let Some(fu) = cu.procedures.get(*qname) else {
                continue;
            };
            for callee in resolved_callees(fu, &known) {
                // `resolved_callees` returns owned names; the map borrows from
                // `known`'s keys, which outlive it and hold the same strings.
                if let Some(interned) = known.get(&callee) {
                    push_unique_edge(&mut map, interned.as_str(), qname.as_str());
                }
            }
        }
        map
    });
    let mut dirty: HashSet<&str> = proc_names.iter().map(|q| q.as_str()).collect();
    // Outer fixpoint-completion loop. The dirty-set worklist below converges
    // fast *when the call graph is complete*, but `callers` is derived from
    // `direct_calls`, which misses a callee edge buried in a nested command
    // substitution — e.g. `symbolNodeOf` inside `[$t get [symbolNodeOf …] …]`,
    // or a self-call inside `[expr {[fib …]}]`. With a missed edge the worklist
    // can empty one step short of the fixpoint (a taint false-negative the debug
    // guard below flags). After the worklist settles we run a full round-robin
    // pass and re-queue every proc whose summary still moves; the lattice is
    // monotone over a finite domain, so this reaches the true least fixpoint and
    // terminates. When the call graph *is* complete the completion pass finds no
    // movement and breaks after one clean sweep — the common, cheap case.
    loop {
        while !dirty.is_empty() {
            let mut next: HashSet<&str> = HashSet::new();
            for qname in &proc_names {
                if !dirty.contains(qname.as_str()) {
                    continue;
                }
                let Some(fu) = cu.procedures.get(*qname) else {
                    continue;
                };
                let proc = &cu.ir_module.procedures[*qname];
                let inferred = infer_fn(qname, &proc.params, fu, &known, &summaries);
                if summaries.get(*qname) != Some(&inferred) {
                    summaries.insert((*qname).clone(), inferred);
                    // Re-queue `qname` itself (a self-recursive proc reads its own
                    // summary) plus its known callers from the reverse call graph.
                    next.insert(qname.as_str());
                    match &callers {
                        Some(map) => next.extend(map.get(qname.as_str()).into_iter().flatten()),
                        None => next.extend(proc_names.iter().map(|q| q.as_str())),
                    }
                }
            }
            dirty = next;
        }
        // Completion pass: a full round-robin catches any proc still short of the
        // fixpoint because the worklist's `callers` map missed an inbound edge.
        let mut moved: HashSet<&str> = HashSet::new();
        for qname in &proc_names {
            let Some(fu) = cu.procedures.get(*qname) else {
                continue;
            };
            let proc = &cu.ir_module.procedures[*qname];
            let inferred = infer_fn(qname, &proc.params, fu, &known, &summaries);
            if summaries.get(*qname) != Some(&inferred) {
                summaries.insert((*qname).clone(), inferred);
                moved.insert(qname.as_str());
            }
        }
        if moved.is_empty() {
            break;
        }
        dirty = moved;
    }

    // Debug-only soundness guard: one full round-robin pass must find the dirty-set
    // fixpoint already stable. If this fires, the call graph (`direct_calls`) missed
    // a callee-summary dependency and the worklist under-converged (a taint
    // false-negative risk). Zero cost in release; proven clean over the tcllib corpus.
    #[cfg(debug_assertions)]
    for qname in &proc_names {
        if let Some(fu) = cu.procedures.get(*qname) {
            let proc = &cu.ir_module.procedures[*qname];
            let reinferred = infer_proc_summary(
                qname,
                &proc.params,
                fu,
                registry,
                interproc,
                dialect,
                &known,
                &summaries,
            );
            debug_assert_eq!(
                summaries.get(*qname),
                Some(&reinferred),
                "interproc taint summary fixpoint under-converged for `{qname}` \
                 (call graph missed a callee-summary edge)"
            );
        }
    }

    summaries
}

/// Solve interprocedural taints for a compilation unit.
///
/// The unit's `interproc` summary (when present) is threaded through so the
/// conservative global-write seeding and the dialect handling
/// match the per-function [`crate::compilation_unit::FunctionUnit::taints`]
/// baseline; on top of that the colour-aware return summaries and parameter
/// entry taints are applied.
#[must_use]
pub fn solve_interprocedural_taints(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> InterprocTaintResult {
    solve_interprocedural_taints_with_seed_option(cu, registry, dialect, None)
}

/// Solve interprocedural taints with explicit external variable sources.
///
/// This is the source-independent entry point for analyses that materialise
/// an external callback value in an otherwise synthetic script.  The seed is
/// applied to matching version-zero SSA slots before the normal call-flow
/// worklist runs, so synthetic namespaces and calls into real procedures both
/// receive the source through their ordinary parameter transfer. Unlike a
/// textual `[gets stdin]` scaffold it cannot be affected by user command
/// shadowing or aliases.
#[must_use]
// The concrete map type is part of this narrow synthetic-seed boundary and is
// threaded through the internal solver without abstraction.
#[allow(clippy::implicit_hasher)]
pub fn solve_interprocedural_taints_with_external_variable_seeds(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
    external_variable_seeds: &HashMap<String, TaintLattice>,
) -> InterprocTaintResult {
    solve_interprocedural_taints_with_seed_option(
        cu,
        registry,
        dialect,
        Some(external_variable_seeds),
    )
}

fn solve_interprocedural_taints_with_seed_option(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
    external_variable_seeds: Option<&HashMap<String, TaintLattice>>,
) -> InterprocTaintResult {
    // `find_taint_warnings_for_cu` is also a public entry point on a freshly
    // built unit, before `CompilationUnit::with_interprocedural` has attached
    // the full call/effect summary.  Preserve the smaller but security-critical
    // interpreter-global receiver facts in that mode: a top-level `ttk::entry
    // .user` remains the same command inside a later callback procedure
    // without retroactively typing a procedure invoked before construction.
    let global_instance_classes =
        crate::interprocedural::global_instance_classes(&cu.ir_module, registry);
    let fallback_interproc = crate::interprocedural::InterproceduralAnalysis {
        tainted_global_writes: crate::interprocedural::tainted_global_writes(
            &cu.ir_module,
            registry,
            &global_instance_classes,
        ),
        global_instance_classes,
        ..crate::interprocedural::InterproceduralAnalysis::default()
    };
    let interproc = cu.interproc.as_ref().or(Some(&fallback_interproc));
    let mut infer = |qname: &str,
                     params: &[String],
                     fu: &FunctionUnit,
                     known: &HashSet<String>,
                     summaries: &HashMap<String, ProcTaintSummary>| {
        infer_proc_summary(
            qname, params, fu, registry, interproc, dialect, known, summaries,
        )
    };
    solve_interprocedural_taints_with_context(
        cu,
        registry,
        dialect,
        interproc,
        &mut infer,
        external_variable_seeds,
    )
}

/// [`solve_interprocedural_taints`] with the per-proc summary inference
/// injectable via [`InferProcSummaryFn`]. Only the *summary fixpoint* phase
/// (`converge_summaries_with`) is redirected through `infer_fn`; the cheap
/// entry-taint worklist that follows is unchanged. The LSP db uses this to feed
/// a salsa-memoised `infer` (SRV-INCREMENTAL 2b) so an unchanged procedure's
/// summary is a cache hit instead of a re-propagation — the ~120 ms pass-1
/// floor the bare worklist still pays every edit.
#[must_use]
pub fn solve_interprocedural_taints_with(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
    infer_fn: &mut InferProcSummaryFn<'_>,
) -> InterprocTaintResult {
    let interproc = cu.interproc.as_ref();
    solve_interprocedural_taints_with_context(cu, registry, dialect, interproc, infer_fn, None)
}

fn solve_interprocedural_taints_with_context(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    infer_fn: &mut InferProcSummaryFn<'_>,
    external_variable_seeds: Option<&HashMap<String, TaintLattice>>,
) -> InterprocTaintResult {
    let summaries = converge_summaries_with(cu, registry, interproc, dialect, infer_fn);

    // Procedures in deterministic (sorted) order, and the name set — both consumed
    // by the entry-taint worklist below.
    let mut proc_names: Vec<&String> = cu.ir_module.procedures.keys().collect();
    proc_names.sort();
    let known: HashSet<String> = summaries.keys().cloned().collect();

    // Top-level taints under the converged summaries.
    let top_instance_classes =
        instance_classes_for_function(&cu.top_level.cfg, registry, interproc, false);
    let top_taints = run_propagation(
        &TaintGraph::new(&cu.top_level.cfg, &cu.top_level.ssa, &cu.top_level.sccp),
        &cu.top_level,
        &top_instance_classes,
        registry,
        interproc,
        dialect,
        external_variable_seeds,
        &summaries,
    );

    // Entry-taint worklist: flow tainted call arguments into callee params,
    // re-propagating each callee until a fixpoint. Every proc is seeded into
    // the queue (so each is propagated at least once, with its accumulated —
    // initially empty — entry taints).
    let mut entry_taints: HashMap<String, HashMap<String, TaintLattice>> = HashMap::new();
    let mut proc_taints: HashMap<String, HashMap<ValueKey, TaintLattice>> = HashMap::new();
    for qname in &proc_names {
        entry_taints.insert((*qname).clone(), HashMap::new());
        proc_taints.insert((*qname).clone(), HashMap::new());
    }

    let mut queue: VecDeque<String> = proc_names.iter().map(|q| (*q).clone()).collect();
    let mut queued: HashSet<String> = queue.iter().cloned().collect();

    // Initial flows from the top level.
    let initial_flows = resolve_call_flows(
        &cu.top_level,
        &top_taints,
        &top_instance_classes,
        registry,
        interproc,
        dialect,
        &known,
        &summaries,
    );
    for (callee, args) in initial_flows {
        if update_entry(&summaries, &mut entry_taints, &callee, &args)
            && queued.insert(callee.clone())
        {
            queue.push_back(callee);
        }
    }

    // The entry-taint worklist re-propagates a procedure every time one of its
    // callers' argument taints move, so its CFG-derived indices are built once
    // per procedure and cached rather than once per dequeue (issue #1251).
    let mut graphs: HashMap<String, TaintGraph<'_>> = HashMap::new();
    let mut instance_classes_by_proc: HashMap<String, LocalInstanceClasses> = HashMap::new();

    while let Some(qname) = queue.pop_front() {
        queued.remove(&qname);
        let Some(fu) = cu.procedures.get(&qname) else {
            continue;
        };
        let mut entry = entry_taints.get(&qname).cloned().unwrap_or_default();
        if let Some(external_variable_seeds) = external_variable_seeds {
            for (name, taint) in external_variable_seeds {
                let slot = entry
                    .entry(name.clone())
                    .or_insert_with(TaintLattice::clean);
                *slot = slot.join(*taint);
            }
        }
        let graph = graphs
            .entry(qname.clone())
            .or_insert_with(|| TaintGraph::new(&fu.cfg, &fu.ssa, &fu.sccp));
        let instance_classes = instance_classes_by_proc
            .entry(qname.clone())
            .or_insert_with(|| instance_classes_for_function(&fu.cfg, registry, interproc, true));
        let taints = run_propagation(
            graph,
            fu,
            instance_classes,
            registry,
            interproc,
            dialect,
            Some(&entry),
            &summaries,
        );

        let flows = resolve_call_flows(
            fu,
            &taints,
            instance_classes,
            registry,
            interproc,
            dialect,
            &known,
            &summaries,
        );
        proc_taints.insert(qname.clone(), taints);

        for (callee, args) in flows {
            if update_entry(&summaries, &mut entry_taints, &callee, &args)
                && queued.insert(callee.clone())
            {
                queue.push_back(callee);
            }
        }
    }

    InterprocTaintResult {
        top_taints,
        proc_taints,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_core_types::DiagCode;
    use tcl_registry::CommandRegistry;

    fn warnings(src: &str) -> Vec<crate::taint::TaintWarning> {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(&reg, None);
        crate::taint::find_taint_warnings_for_cu(&cu, &reg, None)
    }

    #[test]
    fn taint_basis_lattice_matches_expected_colours() {
        // Mutation guard for the basis-name -> lattice mapping (issue #1614):
        // pins each `TaintBasis` variant to its exact expected `TaintColour`
        // set. Swapping which colour two variants map to (e.g. giving `Path`
        // `NON_DASH_PREFIXED` and `NonDash` `PATH_PREFIXED`) flips two of
        // these assertions and fails the test — the array/match could
        // silently drift before #1614 (that was the whole point of the
        // `panic!` fallthrough this enum removes), so this test is the
        // regression net the exhaustiveness alone doesn't provide.
        let t = TaintColour::TAINTED;
        let expected: [(TaintBasis, TaintColour); 17] = [
            (TaintBasis::Generic, t),
            (TaintBasis::Path, t | TaintColour::PATH_PREFIXED),
            (TaintBasis::NonDash, t | TaintColour::NON_DASH_PREFIXED),
            (TaintBasis::CrlfFree, t | TaintColour::CRLF_FREE),
            (TaintBasis::ShellAtom, t | TaintColour::SHELL_ATOM),
            (TaintBasis::ListCanonical, t | TaintColour::LIST_CANONICAL),
            (TaintBasis::RegexLiteral, t | TaintColour::REGEX_LITERAL),
            (TaintBasis::PathNormalised, t | TaintColour::PATH_NORMALISED),
            (
                TaintBasis::PathBounded,
                t | TaintColour::PATH_NORMALISED | TaintColour::PATH_BOUNDED,
            ),
            (
                TaintBasis::HeaderTokenSafe,
                t | TaintColour::HEADER_TOKEN_SAFE,
            ),
            (TaintBasis::HtmlEscaped, t | TaintColour::HTML_ESCAPED),
            (TaintBasis::UrlEncoded, t | TaintColour::URL_ENCODED),
            (TaintBasis::Ip, t | TaintColour::IP_ADDRESS),
            (TaintBasis::Port, t | TaintColour::PORT),
            (TaintBasis::Fqdn, t | TaintColour::FQDN),
            (TaintBasis::PathJoined, t | TaintColour::PATH_JOINED),
            (TaintBasis::Channel, t | TaintColour::CHANNEL),
        ];
        assert_eq!(expected.len(), TaintBasis::ALL.len());
        for (basis, colour) in expected {
            assert_eq!(
                basis.lattice(),
                TaintLattice { colours: colour },
                "{basis:?} mapped to an unexpected lattice"
            );
        }
        // Every non-generic basis is pairwise distinct — a swap between two
        // bases (rather than a wrong constant) still fails via `intersects`
        // returning true for both instead of one.
        for i in 1..TaintBasis::ALL.len() {
            for j in (i + 1)..TaintBasis::ALL.len() {
                let a = TaintBasis::ALL[i].lattice();
                let b = TaintBasis::ALL[j].lattice();
                assert_ne!(
                    a,
                    b,
                    "{:?} and {:?} must map to distinct lattices",
                    TaintBasis::ALL[i],
                    TaintBasis::ALL[j]
                );
            }
        }
    }

    #[test]
    fn cross_proc_entry_taint_into_sink_warns() {
        // A tainted argument flowing into a proc parameter and then into a
        // sink inside that proc is reported (cross-proc entry-taint) — the
        // gap interprocedural taint solving closes.
        let src = "proc s {v} { eval $v }\nset x [gets stdin]\ns $x\n";
        let w = warnings(src);
        assert_eq!(w.len(), 1, "expected one cross-proc sink warning: {w:?}");
        assert_eq!(w[0].code, DiagCode::T100);
        assert_eq!(w[0].variable, "v");
        assert_eq!(w[0].sink_command, "eval");
    }

    #[test]
    fn clean_argument_into_proc_does_not_warn() {
        // A clean (literal) argument flowing into the same proc must NOT warn.
        let src = "proc s {v} { eval $v }\ns hello\n";
        assert!(warnings(src).is_empty());
    }

    #[test]
    fn return_summary_carries_taint_through_callee() {
        // A proc returning its tainted parameter taints the caller's value,
        // which then reaches a sink: exercises the return-summary half.
        let src = "proc id {v} { return $v }\nset x [gets stdin]\neval [id $x]\n";
        let w = warnings(src);
        assert!(
            w.iter()
                .any(|w| w.code == DiagCode::T100 && w.sink_command == "eval"),
            "expected eval injection via return summary: {w:?}"
        );
    }

    #[test]
    fn self_recursive_proc_taint_summary_converges() {
        // Regression: a directly self-recursive proc — `fib` calling itself
        // inside `[expr {[fib …]}]` — under-converged the interproc taint
        // fixpoint because the self-call edge is not extracted into
        // `direct_calls` (it is buried in a braced `expr`), so the worklist
        // never re-queued `fib` after its own summary changed. The debug-only
        // convergence guard then panicked, and the LSP diagnostic worker
        // caught the panic and published nothing — so the recursive-definition
        // and linked-editing e2e tests timed out waiting for diagnostics.
        // The worklist now re-queues a proc on its own change, so the fixpoint
        // settles and the guard holds.
        let reg = CommandRegistry::build_default();
        let src = "proc fib {n} {\n    if {$n < 2} { return $n }\n    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]\n}\nputs \"fib(10) = [fib 10]\"\n";
        let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(&reg, None);
        // Exercises `converge_summaries_with` and its debug fixpoint guard —
        // this panicked before the fix.
        let _ = solve_interprocedural_taints(&cu, &reg, None);
        // The full taint pass over the same source must also complete cleanly.
        let _ = warnings(src);
    }

    /// Build the whole-module summary map the way the solver does, so a test
    /// can compare summaries rather than only the diagnostics they produce.
    fn summaries_for(src: &str) -> HashMap<String, ProcTaintSummary> {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(&reg, None);
        let interproc = cu.interproc.as_ref();
        converge_summaries_with(
            &cu,
            &reg,
            interproc,
            None,
            &mut |qname, params, fu, known, summaries| {
                infer_proc_summary(qname, params, fu, &reg, interproc, None, known, summaries)
            },
        )
    }

    #[test]
    fn summary_scenarios_reuse_one_instance_class_solve() {
        // `id` requires the clean solve plus every taint-basis scenario for
        // its live parameter. Receiver typing is independent of those seeds,
        // so it must be solved once for the whole summary, not once per
        // propagation run (the old cost was 1 + TaintBasis::ALL.len()).
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for("proc id {v} { return $v }\n", &reg, false)
            .with_interprocedural(&reg, None);
        let interproc = cu.interproc.as_ref();
        let fu = &cu.procedures["::id"];
        let known = HashSet::from(["::id".to_owned()]);
        crate::taint::reset_instance_class_solve_count();

        let summary = infer_proc_summary(
            "::id",
            &["v".to_owned()],
            fu,
            &reg,
            interproc,
            None,
            &known,
            &HashMap::new(),
        );

        assert_eq!(
            summary.return_by_param_basis[0].1.len(),
            TaintBasis::ALL.len()
        );
        assert_eq!(crate::taint::instance_class_solve_count(), 1);
    }

    /// The unpruned reference: re-derive one procedure's summary the way
    /// `infer_proc_summary` did before issue #1187, by seeding every
    /// `(parameter, basis)` pair with a full dataflow solve.  A test can then
    /// assert the pruned result is bit-identical rather than merely plausible.
    fn summary_without_prunes(src: &str, target: &str) -> ProcTaintSummary {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(&reg, None);
        let interproc = cu.interproc.as_ref();
        let summaries = summaries_for(src);
        let known: HashSet<String> = summaries.keys().cloned().collect();
        let proc = &cu.ir_module.procedures[target];
        let fu = &cu.procedures[target];

        let graph = TaintGraph::new(&fu.cfg, &fu.ssa, &fu.sccp);
        let instance_classes = local_instance_classes(&fu.cfg, &reg);
        let base = run_propagation(
            &graph,
            fu,
            &instance_classes,
            &reg,
            interproc,
            None,
            None,
            &summaries,
        );
        let ctx = return_ctx(
            fu,
            &reg,
            interproc,
            None,
            &known,
            &summaries,
            &instance_classes,
        );
        let return_base = collect_return_taint(fu, &base, ctx);
        let mut by_param_basis = Vec::new();
        for param in &proc.params {
            let mut values = Vec::new();
            for basis in TaintBasis::ALL {
                let mut seed = HashMap::new();
                seed.insert(param.clone(), basis.lattice());
                let seeded = run_propagation(
                    &graph,
                    fu,
                    &instance_classes,
                    &reg,
                    interproc,
                    None,
                    Some(&seed),
                    &summaries,
                );
                values.push(collect_return_taint(fu, &seeded, ctx));
            }
            by_param_basis.push((param.clone(), values));
        }
        ProcTaintSummary {
            qualified_name: target.to_owned(),
            params: proc.params.clone(),
            arity: crate::interprocedural::arity_from_names(&proc.params),
            return_base,
            return_by_param_basis: by_param_basis,
        }
    }

    /// Assert every summary the solver produces for `src` is bit-identical to
    /// the unpruned computation — the acceptance bar for #1187.
    fn prunes_are_bit_identical(src: &str) {
        let pruned = summaries_for(src);
        assert!(!pruned.is_empty(), "fixture defines no procedures");
        for (qname, summary) in &pruned {
            let reference = summary_without_prunes(src, qname);
            assert_eq!(
                *summary, reference,
                "pruned summary for `{qname}` differs from the unpruned solve"
            );
        }
    }

    #[test]
    fn prune_keeps_summaries_bit_identical_across_the_matrix() {
        // TP — direct and transitive passthrough, several parameters, and a
        // basis-bearing (sanitised) path, so the pruned and unpruned solves
        // must agree on non-trivial colour values, not just on `clean`.
        prunes_are_bit_identical("proc id {v} { return $v }\n");
        prunes_are_bit_identical("proc pick {a b c} { return $b }\n");
        prunes_are_bit_identical(
            "proc inner {v} { return $v }\nproc outer {v} { return [inner $v] }\n",
        );
        prunes_are_bit_identical("proc norm {p} { return [file normalize $p] }\n");
        prunes_are_bit_identical("proc joined {a b} { return \"$a/$b\" }\n");
        // `args` — the variadic binding the summary applier special-cases.
        prunes_are_bit_identical("proc varargs {a args} { return $args }\n");
        // FP — a sanitised value, and parameters that never reach the return.
        prunes_are_bit_identical("proc clean {v} { return [string length $v] }\n");
        prunes_are_bit_identical("proc unused {a b} { return literal }\n");
        prunes_are_bit_identical("proc partly {a b} { set t $a\n return $t }\n");
        // TN — no return value at all, and a dynamic/unresolved call.
        prunes_are_bit_identical("proc silent {v} { puts $v }\n");
        prunes_are_bit_identical("proc dynamic {v} { return [$v run] }\n");
        // FN — a callee reached only through a nested command substitution,
        // and mutual recursion (an SCC the worklist must settle).
        prunes_are_bit_identical(
            "proc leaf {v} { return $v }\nproc nest {t v} { return [lindex [leaf $v] 0] }\n",
        );
        prunes_are_bit_identical(
            "proc ping {n} { if {$n < 1} { return $n }\n return [pong $n] }\n\
             proc pong {n} { return [ping $n] }\n",
        );
    }

    #[test]
    fn constant_return_prune_yields_the_untainted_summary() {
        // A procedure whose every executable return value is substitution-free
        // cannot carry taint out, so the whole summary is the clean one — with
        // no dataflow solve run at all.
        let summaries = summaries_for("proc lit {a b} { return ok }\n");
        let s = summaries.get("::lit").expect("::lit summarised");
        assert_eq!(*s, ProcTaintSummary::untainted("::lit", &s.params));
        assert_eq!(s.params, vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn unread_parameter_scenarios_equal_the_base_return() {
        // `b` is never read, so seeding it cannot change the return: every one
        // of its 17 basis scenarios is exactly `return_base`.
        let summaries = summaries_for("proc half {a b} { return [string cat $a x] }\n");
        let s = summaries.get("::half").expect("::half summarised");
        let (name, values) = s
            .return_by_param_basis
            .iter()
            .find(|(p, _)| p == "b")
            .expect("parameter b recorded");
        assert_eq!(name, "b");
        assert_eq!(values.len(), TaintBasis::ALL.len());
        assert!(
            values.iter().all(|v| *v == s.return_base),
            "unread parameter must reproduce the base return, got {values:?}"
        );
    }

    #[test]
    fn pruning_preserves_cross_proc_taint_diagnostics() {
        // The end-to-end guarantee: the diagnostics a user sees are unchanged.
        // A passthrough that must still warn…
        let w = warnings("proc id {v} { return $v }\nset x [gets stdin]\neval [id $x]\n");
        assert!(
            w.iter()
                .any(|w| w.code == DiagCode::T100 && w.sink_command == "eval"),
            "passthrough taint must survive the prunes: {w:?}"
        );
        // …and a procedure the constant-return prune skips *entirely* (zero
        // solves) must still produce exactly what the unpruned solver did.
        // The sink check reports `$x` here because the tainted variable is
        // written literally inside `eval`'s word, independently of what `lit`'s
        // summary says — behaviour this change does not touch, pinned so a
        // future prune cannot quietly move it.
        let src = "proc lit {v} { return safe }\nset x [gets stdin]\neval [lit $x]\n";
        let w = warnings(src);
        assert_eq!(w.len(), 1, "{w:?}");
        assert_eq!(w[0].code, DiagCode::T100);
        assert_eq!(w[0].variable, "x");
        // The summary itself is the clean one — the prune's actual claim.
        let summaries = summaries_for(src);
        let lit = summaries.get("::lit").expect("::lit summarised");
        assert_eq!(*lit, ProcTaintSummary::untainted("::lit", &lit.params));
    }

    #[test]
    fn untainted_param_summary_is_clean() {
        let reg = CommandRegistry::build_default();
        let summary = ProcTaintSummary::untainted("::p", &["a".to_owned(), "b".to_owned()]);
        let _ = &reg;
        // Untainted args → untainted return.
        let out =
            apply_proc_return_summary(&summary, &[TaintLattice::clean(), TaintLattice::clean()]);
        assert!(!out.is_tainted());
        // Wrong arity → untainted.
        let out = apply_proc_return_summary(&summary, &[TaintLattice::tainted()]);
        assert!(!out.is_tainted());
    }
}
