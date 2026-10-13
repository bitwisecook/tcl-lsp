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
use crate::ir::ProcedureId;
use crate::naming::normalise_var_name;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
#[cfg(test)]
use crate::taint::local_instance_classes;
use crate::taint::{
    LocalInstanceClasses, TaintColour, TaintCtx, TaintGraph, TaintLattice, TaintPropagationInputs,
    TaintSourceContext, instance_classes_for_function, propagate_taints, word_taint,
};

// Basis lattices

/// Closed vocabulary of taint-colour bases. The per-parameter return
/// scenarios are computed by seeding the parameter with each basis lattice in
/// turn, in [`TaintBasis::ALL`] order (which fixes `return_by_param_basis`'s
/// index-to-basis mapping).
///
/// Purely an internal solver vocabulary: never serialised, never a CLI/`.tclspec`
/// spelling, so no `FromStr`/`Display` boundary is needed (unlike the
/// dialect-name discipline).
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
    /// over-taints.
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
    let Ok(tokens) = Lexer::with_config(
        text,
        tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
    )
    .tokenise_all() else {
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
/// produced without running the dataflow at all.
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
            tokens,
            value_word,
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
        let returned = TaintCtx {
            source_tokens: tokens.as_deref(),
            source_word: value_word.as_ref(),
            word_offset: value_word.as_ref().map(|word| word.source().span.start()),
            ..ctx.at(span.map_or(0, tcl_lexer::Span::start))
        };
        ret = ret.join(word_taint(value, &uses, taints, returned));
    }
    ret
}

/// Run intra-procedural taint propagation over `fu` with the given entry
/// taints and summaries. A thin wrapper threading the common arguments.
///
/// `graph` carries the CFG-derived indices, built once by the caller: one
/// summary inference runs this `1 + params × TaintBasis::ALL` times and the
/// indices are identical across all of them.
#[allow(clippy::too_many_arguments)]
fn run_propagation(
    graph: &TaintGraph<'_>,
    fu: &FunctionUnit,
    instance_classes: &LocalInstanceClasses,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    source: TaintSourceContext<'_>,
    param_taints: Option<&HashMap<String, TaintLattice>>,
    summaries: &HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
) -> HashMap<ValueKey, TaintLattice> {
    propagate_taints(
        graph,
        TaintPropagationInputs {
            registry,
            rendered_props: Some(&fu.rendered_props),
            interproc,
            source,
            param_taints,
            taint_summaries: Some(summaries),
            instance_classes,
        },
    )
}

/// Build a [`TaintCtx`] for evaluating return-value words of `fu`.
fn return_ctx<'a>(
    fu: &'a FunctionUnit,
    registry: &'a CommandRegistry,
    interproc: Option<&'a crate::interprocedural::InterproceduralAnalysis>,
    source: TaintSourceContext<'a>,
    known: &'a HashSet<crate::ir::ProcedureId>,
    summaries: &'a HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
    instance_classes: &'a LocalInstanceClasses,
) -> TaintCtx<'a> {
    TaintCtx {
        registry,
        ssa: &fu.ssa,
        interproc,
        known_procs: Some(known),
        dialect: source.dialect(),
        source,
        taint_summaries: Some(summaries),
        instance_classes: Some(instance_classes),
        source_position: None,
        source_tokens: None,
        word_offset: None,
        source_word: None,
    }
}

/// Per-proc summary-inference callback driven by the summary-fixpoint worklist
/// ([`converge_summaries_with`]). Receives `(qname, params, fu, known,
/// summaries)` and returns the procedure's inferred [`ProcTaintSummary`] under
/// the *current* summaries. The default is [`infer_proc_summary`]; the LSP db
/// injects a salsa-memoised variant that returns an
/// unchanged proc's summary from cache (keyed on its offset-0 body + its
/// direct callees' summaries) instead of re-running the propagation here.
pub type InferProcSummaryFn<'a> = dyn FnMut(
        &CompilationUnit,
        crate::ir::ProcedureId,
        Option<&crate::interprocedural::InterproceduralAnalysis>,
        &HashSet<crate::ir::ProcedureId>,
        &HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
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
/// Two prunes cut that down.  Both are **proofs that the solve
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
    cu: &CompilationUnit,
    body_id: ProcedureId,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    _dialect: Option<&'static tcl_dialect::DialectProfile>,
    known: &HashSet<ProcedureId>,
    summaries: &HashMap<ProcedureId, ProcTaintSummary>,
) -> Option<ProcTaintSummary> {
    let procedure = cu.ir_module.procedures.get(&body_id)?;
    let fu = cu.procedures.get(&body_id)?;
    if fu.body_id != Some(body_id) {
        return None;
    }
    Some(infer_proc_summary_with_source(ProcSummaryInputs {
        qname: &procedure.qualified_name,
        params: &procedure.params,
        fu,
        registry,
        interproc,
        source: TaintSourceContext::for_module_function(registry, &cu.ir_module, fu),
        known,
        summaries,
    }))
}

struct ProcSummaryInputs<'a> {
    qname: &'a str,
    params: &'a [String],
    fu: &'a FunctionUnit,
    registry: &'a CommandRegistry,
    interproc: Option<&'a crate::interprocedural::InterproceduralAnalysis>,
    source: TaintSourceContext<'a>,
    known: &'a HashSet<crate::ir::ProcedureId>,
    summaries: &'a HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
}

fn infer_proc_summary_with_source(inputs: ProcSummaryInputs<'_>) -> ProcTaintSummary {
    let ProcSummaryInputs {
        qname,
        params,
        fu,
        registry,
        interproc,
        source,
        known,
        summaries,
    } = inputs;

    if fu.cfg.has_opaque_native_accesses() {
        return ProcTaintSummary {
            qualified_name: qname.to_owned(),
            params: params.to_vec(),
            arity: crate::interprocedural::arity_from_names(params),
            return_base: TaintLattice::tainted(),
            return_by_param_basis: params
                .iter()
                .map(|param| {
                    (
                        param.clone(),
                        vec![TaintLattice::tainted(); TaintBasis::ALL.len()],
                    )
                })
                .collect(),
        };
    }
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
    let instance_classes =
        instance_classes_for_function(&fu.cfg, registry, interproc, true, source);
    let base_taints = run_propagation(
        &graph,
        fu,
        &instance_classes,
        registry,
        interproc,
        source,
        None,
        summaries,
    );
    let ctx = return_ctx(
        fu,
        registry,
        interproc,
        source,
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
                source,
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
    source: TaintSourceContext<'_>,
    known: &HashSet<crate::ir::ProcedureId>,
    summaries: &HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
) -> Vec<(crate::ir::ProcedureId, Vec<TaintLattice>)> {
    let ctx = return_ctx(
        fu,
        registry,
        interproc,
        source,
        known,
        summaries,
        instance_classes,
    );
    let mut flows = Vec::new();
    let Some(module) = source.module() else {
        return flows;
    };
    let mut collect = |tokens: &crate::ir::CommandTokens,
                       uses: &HashMap<Symbol, u32>,
                       position: u32| {
        let mut invocations = vec![tokens.clone()];
        let lifted = source.metadata_context().and_then(|metadata| {
            crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                tokens,
                source.lexer_config(),
                registry,
                metadata,
            )
        });
        if let Some(lifted) = lifted {
            invocations.extend(lifted.into_iter().filter_map(|call| call.tokens));
        }
        for invocation in invocations {
            let Some(head) = original_head(&invocation) else {
                continue;
            };
            let Some(calls) =
                crate::registry_invocation::original_source_procedure_calls_for_word_for_module(
                    &invocation,
                    &head,
                    module,
                    registry,
                )
            else {
                continue;
            };
            for call in calls {
                if !known.contains(&call.id()) {
                    continue;
                }
                let Some(summary) = summaries.get(&call.id()) else {
                    continue;
                };
                let Some(count) = call.argument_count() else {
                    flows.push((
                        call.id(),
                        vec![TaintLattice::tainted(); summary.params.len()],
                    ));
                    continue;
                };
                if !summary
                    .arity
                    .accepts(u16::try_from(count).unwrap_or(u16::MAX))
                {
                    continue;
                }
                let effective = call.effective_arguments();
                let arguments = effective.words.iter().skip(1).enumerate().map(|(argument, word)| {
                        match effective.origins.get(argument + 1) {
                            Some(crate::registry_invocation::InvocationWordOrigin::Written(index)) => {
                                let Some(original) = invocation.words().get(*index) else { return TaintLattice::tainted(); };
                                if matches!(original, crate::ir::WordExpr::Literal { .. } | crate::ir::WordExpr::BracedLiteral { .. }) {
                                    return TaintLattice::clean();
                                }
                                let argument_context = TaintCtx {
                                    source_tokens: Some(&invocation),
                                    source_word: Some(original),
                                    word_offset: Some(original.source().span.start()),
                                    ..ctx.at(position)
                                };
                                word_taint(&word.legacy_text(), uses, taints, argument_context)
                            }
                            Some(crate::registry_invocation::InvocationWordOrigin::BindingPrefix(index)) => {
                                match effective.binding_prefix.get(*index) {
                                    Some(crate::registry_invocation::EffectiveInvocationWord::Literal(_)
                                        | crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(_)) => TaintLattice::clean(),
                                    _ => TaintLattice::tainted(),
                                }
                            }
                            _ => TaintLattice::tainted(),
                        }
                    }).collect();
                flows.push((call.id(), arguments));
            }
        }
    };
    for bn in &fu.sccp.executable_blocks {
        let Some(block) = fu.cfg.blocks.get(bn) else {
            continue;
        };
        let Some(ssa_block) = fu.ssa.blocks.get(bn) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
            let Some(ssa_statement) = ssa_block.statements.get(index) else {
                continue;
            };
            let Some(tokens) = fu.cfg.source_input_tokens_at(*bn, index) else {
                continue;
            };
            collect(tokens, &ssa_statement.uses, statement.span().start());
        }
        if let Some(Terminator::Return {
            tokens: Some(tokens),
            span,
            ..
        }) = &block.terminator
        {
            collect(
                tokens,
                &ssa_block.exit_versions,
                span.map_or(0, tcl_lexer::Span::start),
            );
        }
    }
    flows
}
fn original_head(tokens: &crate::ir::CommandTokens) -> Option<String> {
    let binding = tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let words = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        config,
    )?;
    let head = tcl_syntax::word_rules::original_static_word_source_bytes(words.first()?)?;
    String::from_utf8(head).ok()
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
pub fn resolved_callees(
    cu: &CompilationUnit,
    body_id: Option<crate::ir::ProcedureId>,
    known: &HashSet<crate::ir::ProcedureId>,
    registry: &CommandRegistry,
) -> Vec<crate::ir::ProcedureId> {
    let fu = match body_id {
        Some(id) => cu.procedures.get(&id),
        None => Some(&cu.top_level),
    };
    let Some(fu) = fu else {
        return known.iter().copied().collect();
    };
    let source = TaintSourceContext::for_module_function(registry, &cu.ir_module, fu);
    let mut out = std::collections::BTreeSet::new();
    for block_id in &fu.sccp.executable_blocks {
        let Some(block) = fu.cfg.blocks.get(block_id) else {
            continue;
        };
        for (index, _) in block.statements.iter().enumerate() {
            let Some(tokens) = fu.cfg.source_input_tokens_at(*block_id, index) else {
                out.extend(known.iter().copied());
                continue;
            };
            if let Some(head) = original_head(tokens) {
                if let Some(ids) = source.procedure_ids(tokens, &head, registry) {
                    out.extend(ids.into_iter().filter(|id| known.contains(id)));
                }
            }
            let lifted = source.metadata_context().and_then(|metadata| {
                crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                    tokens,
                    source.lexer_config(),
                    registry,
                    metadata,
                )
            });
            if let Some(lifted) = lifted {
                for call in lifted {
                    let Some(child) = call.tokens else {
                        out.extend(known.iter().copied());
                        continue;
                    };
                    let selected = original_head(&child)
                        .and_then(|head| source.procedure_ids(&child, &head, registry));
                    if let Some(ids) = selected {
                        out.extend(ids.into_iter().filter(|id| known.contains(id)));
                    } else {
                        out.extend(known.iter().copied());
                    }
                }
            } else if tokens
                .words()
                .iter()
                .any(|word| word.legacy_text().contains('['))
            {
                out.extend(known.iter().copied());
            }
        }
        if let Some(Terminator::Return {
            value: Some(value), ..
        }) = &block.terminator
        {
            if value.contains('[') {
                out.extend(known.iter().copied());
            }
        }
    }
    out.into_iter().collect()
}

/// Memo dependencies of original substitutions in one Module-owned body.
/// Unresolved or incomplete source retains every candidate; this selects no call edge.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn command_subst_callees(
    cu: &CompilationUnit,
    body_id: ProcedureId,
    known: &HashSet<ProcedureId>,
    registry: &CommandRegistry,
) -> Vec<ProcedureId> {
    let module = &cu.ir_module;
    let mut dependencies = std::collections::BTreeSet::new();
    let original = (|| {
        let procedure = module.procedures.get(&body_id)?;
        let retained = module.retained_source_bindings.as_deref()?;
        if !retained.matches_original_procedure(procedure)
            || module.source.try_text().ok()? != cu.source
        {
            return None;
        }
        let bindings = retained.positioned_for_module(module, registry)?;
        let body = procedure.body_source.as_deref()?;
        let base = usize::try_from(procedure.body_offset).ok()?;
        if cu.source.get(base..base.checked_add(body.len())?) != Some(body) {
            return None;
        }
        Some((body, base, bindings))
    })();
    let Some((body, base, bindings)) = original else {
        return known.iter().copied().collect();
    };
    let config = module.lexer_config.nested();
    for (open, _) in body.match_indices('[') {
        let Some(end) = tcl_lexer::command_substitution_end(body, open) else {
            dependencies.extend(known.iter().copied());
            continue;
        };
        let start = base + open + 1;
        let finish = base + end - 1;
        let Some(inner) = cu.source.get(start..finish) else {
            dependencies.extend(known.iter().copied());
            continue;
        };
        let Ok(offset) = u32::try_from(start) else {
            dependencies.extend(known.iter().copied());
            continue;
        };
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(inner, offset, config);
        let map = SourceMap::new(inner).with_base(offset, 0, 0);
        for command in commands {
            if command.is_partial {
                dependencies.extend(known.iter().copied());
                continue;
            }
            let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, &command);
            bindings.stamp_original_tokens(&mut tokens);
            let ids = original_head(&tokens).and_then(|head| {
                crate::registry_invocation::original_source_procedure_ids_for_word_for_module(
                    &tokens, &head, module, registry,
                )
            });
            if let Some(ids) = ids {
                dependencies.extend(ids.into_iter().filter(|id| known.contains(id)));
            } else {
                dependencies.extend(known.iter().copied());
            }
        }
    }
    dependencies.into_iter().collect()
}

/// Per-body solved taints and the separate actual top-level solve.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InterprocTaintResult {
    /// Taints for the top-level script.
    pub top_taints: HashMap<ValueKey, TaintLattice>,
    /// Per-procedure taints, keyed by qualified name.
    pub proc_taints: HashMap<crate::ir::ProcedureId, HashMap<ValueKey, TaintLattice>>,
}

impl InterprocTaintResult {
    /// Select the actual top-level or Module-owned procedure solve.
    /// Methods and unavailable body owners retain their own conservative lattice.
    #[must_use]
    pub fn taints_for_function<'a>(
        &'a self,
        cu: &'a CompilationUnit,
        fu: &'a FunctionUnit,
    ) -> &'a HashMap<ValueKey, TaintLattice> {
        if std::ptr::eq(fu, &cu.top_level) {
            return &self.top_taints;
        }
        let Some(id) = fu.body_id else {
            return &fu.taints;
        };
        if cu
            .procedures
            .get(&id)
            .is_none_or(|current| !std::ptr::eq(current, fu))
        {
            return &fu.taints;
        }
        self.proc_taints.get(&id).unwrap_or(&fu.taints)
    }
}

/// Update a callee's accumulated entry taints with an incoming argument tuple.
/// Returns `true` when any parameter's entry taint grew.
fn update_entry(
    summaries: &HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
    entry_taints: &mut HashMap<crate::ir::ProcedureId, HashMap<String, TaintLattice>>,
    callee: &crate::ir::ProcedureId,
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
/// can be served by a salsa-memoised variant: the per-proc
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
fn push_unique_edge(
    map: &mut HashMap<crate::ir::ProcedureId, Vec<crate::ir::ProcedureId>>,
    key: crate::ir::ProcedureId,
    val: crate::ir::ProcedureId,
) {
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
    _dialect: Option<&'static tcl_dialect::DialectProfile>,
    infer_fn: &mut InferProcSummaryFn<'_>,
) -> HashMap<crate::ir::ProcedureId, ProcTaintSummary> {
    let mut proc_names: Vec<&crate::ir::ProcedureId> = cu.ir_module.procedures.keys().collect();
    proc_names.sort();

    // Seed summaries (all scenarios untainted).
    let mut summaries: HashMap<crate::ir::ProcedureId, ProcTaintSummary> = HashMap::new();
    for qname in &proc_names {
        let proc = &*cu
            .ir_module
            .procedures
            .get(qname)
            .expect("unambiguous summary label");
        summaries.insert(
            (*qname).clone(),
            ProcTaintSummary::untainted(&proc.qualified_name, &proc.params),
        );
    }
    let known: HashSet<crate::ir::ProcedureId> = summaries.keys().cloned().collect();

    // `callers[Q]` = procedures that directly call `Q`; when `Q`'s summary changes
    // its callers are re-queued. Without a call graph, re-queue everything.
    //
    // Two sources, unioned.  `InterproceduralAnalysis::direct_calls`
    // is the declared call graph, but it misses a callee reached through a
    // command substitution the analyser recorded as a plain value —
    // `symbolNodeOf` in `set n [$t get [symbolNodeOf …] …]`, or a self-call
    // inside `[expr {[fib …]}]`.  [`resolved_callees`] scans the very CFG
    // statements the inference resolves calls from, so it supplies exactly the
    // edges `direct_calls` drops there.  A missed edge is not a wrong answer —
    // the completion sweep below still reaches the true least fixpoint — but it
    // costs a whole extra round-robin round to discover, which is the expensive
    // thing this solve does.
    let callers: Option<HashMap<crate::ir::ProcedureId, Vec<crate::ir::ProcedureId>>> = interproc
        .map(|ia| {
            let mut map: HashMap<crate::ir::ProcedureId, Vec<crate::ir::ProcedureId>> =
                HashMap::new();
            for (caller, summary) in &ia.procedures {
                for callee in &summary.direct_calls {
                    push_unique_edge(&mut map, *callee, *caller);
                }
            }
            for qname in &proc_names {
                for callee in resolved_callees(cu, Some(**qname), &known, registry) {
                    // `resolved_callees` returns owned names; the map borrows from
                    // `known`'s keys, which outlive it and hold the same strings.
                    if known.contains(&callee) {
                        push_unique_edge(&mut map, callee, **qname);
                    }
                }
            }
            map
        });
    let mut dirty: HashSet<crate::ir::ProcedureId> = proc_names.iter().map(|q| **q).collect();
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
            let mut next: HashSet<crate::ir::ProcedureId> = HashSet::new();
            for qname in &proc_names {
                if !dirty.contains(*qname) {
                    continue;
                }
                let Some(fu) = cu.procedures.get(qname) else {
                    continue;
                };
                let proc = &*cu
                    .ir_module
                    .procedures
                    .get(qname)
                    .expect("unambiguous summary label");
                let source = TaintSourceContext::for_module_function(registry, &cu.ir_module, fu);
                let inferred = if source.metadata_context().is_some() {
                    infer_fn(cu, **qname, interproc, &known, &summaries)
                } else {
                    infer_proc_summary_with_source(ProcSummaryInputs {
                        qname: &proc.qualified_name,
                        params: &proc.params,
                        fu,
                        registry,
                        interproc,
                        source,
                        known: &known,
                        summaries: &summaries,
                    })
                };
                if summaries.get(*qname) != Some(&inferred) {
                    summaries.insert((*qname).clone(), inferred);
                    // Re-queue `qname` itself (a self-recursive proc reads its own
                    // summary) plus its known callers from the reverse call graph.
                    next.insert(**qname);
                    match &callers {
                        Some(map) => next.extend(map.get(*qname).into_iter().flatten()),
                        None => next.extend(proc_names.iter().map(|q| **q)),
                    }
                }
            }
            dirty = next;
        }
        // Completion pass: a full round-robin catches any proc still short of the
        // fixpoint because the worklist's `callers` map missed an inbound edge.
        let mut moved: HashSet<crate::ir::ProcedureId> = HashSet::new();
        for qname in &proc_names {
            let Some(fu) = cu.procedures.get(qname) else {
                continue;
            };
            let proc = &*cu
                .ir_module
                .procedures
                .get(qname)
                .expect("unambiguous summary label");
            let source = TaintSourceContext::for_module_function(registry, &cu.ir_module, fu);
            let inferred = if source.metadata_context().is_some() {
                infer_fn(cu, **qname, interproc, &known, &summaries)
            } else {
                infer_proc_summary_with_source(ProcSummaryInputs {
                    qname: &proc.qualified_name,
                    params: &proc.params,
                    fu,
                    registry,
                    interproc,
                    source,
                    known: &known,
                    summaries: &summaries,
                })
            };
            if summaries.get(*qname) != Some(&inferred) {
                summaries.insert((*qname).clone(), inferred);
                moved.insert(**qname);
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
        if let Some(fu) = cu.procedures.get(qname) {
            let proc = &*cu
                .ir_module
                .procedures
                .get(qname)
                .expect("unambiguous summary label");
            let reinferred = infer_proc_summary_with_source(ProcSummaryInputs {
                qname: &proc.qualified_name,
                params: &proc.params,
                fu,
                registry,
                interproc,
                source: TaintSourceContext::for_module_function(registry, &cu.ir_module, fu),
                known: &known,
                summaries: &summaries,
            });
            debug_assert_eq!(
                summaries.get(*qname),
                Some(&reinferred),
                "interproc taint summary fixpoint under-converged for `{qname:?}` \
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
    dialect: Option<&'static tcl_dialect::DialectProfile>,
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
    dialect: Option<&'static tcl_dialect::DialectProfile>,
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
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    external_variable_seeds: Option<&HashMap<String, TaintLattice>>,
) -> InterprocTaintResult {
    // A fresh actual CU can retain conditional receiver setup before the full
    // summary is attached. The map and its Module owner travel together; no
    // callback frame or live object is supplied by these source candidates.
    let global_instance_classes =
        crate::interprocedural::global_instance_classes(&cu.ir_module, registry);
    let global_source =
        crate::interprocedural::global_instance_class_source(&cu.ir_module, registry);
    let fallback_interproc = crate::interprocedural::InterproceduralAnalysis {
        tainted_global_writes: crate::interprocedural::tainted_global_writes(
            &cu.ir_module,
            registry,
            &global_instance_classes,
        ),
        global_instance_classes,
        global_instance_class_input: global_source.as_ref().map(|(input, _)| input.clone()),
        global_instance_class_owner: global_source.map(|(_, owner)| owner),
        ..crate::interprocedural::InterproceduralAnalysis::default()
    };
    let interproc = cu.interproc.as_ref().or(Some(&fallback_interproc));
    let mut infer = |cu: &CompilationUnit,
                     body_id: ProcedureId,
                     interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
                     known: &HashSet<ProcedureId>,
                     summaries: &HashMap<ProcedureId, ProcTaintSummary>| {
        infer_proc_summary(cu, body_id, registry, interproc, dialect, known, summaries)
            .expect("worklist retains the actual procedure and function")
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
/// a salsa-memoised `infer` so an unchanged procedure's
/// summary is a cache hit instead of a re-propagation — the ~120 ms pass-1
/// floor the bare worklist still pays every edit.
#[must_use]
pub fn solve_interprocedural_taints_with(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    infer_fn: &mut InferProcSummaryFn<'_>,
) -> InterprocTaintResult {
    let interproc = cu.interproc.as_ref();
    solve_interprocedural_taints_with_context(cu, registry, dialect, interproc, infer_fn, None)
}

fn solve_interprocedural_taints_with_context(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    infer_fn: &mut InferProcSummaryFn<'_>,
    external_variable_seeds: Option<&HashMap<String, TaintLattice>>,
) -> InterprocTaintResult {
    let summaries = converge_summaries_with(cu, registry, interproc, dialect, infer_fn);

    // Procedures in deterministic (sorted) order, and the name set — both consumed
    // by the entry-taint worklist below.
    let mut proc_names: Vec<&crate::ir::ProcedureId> = cu.ir_module.procedures.keys().collect();
    proc_names.sort();
    let known: HashSet<crate::ir::ProcedureId> = summaries.keys().cloned().collect();

    // Top-level taints under the converged summaries.
    let top_instance_classes = instance_classes_for_function(
        &cu.top_level.cfg,
        registry,
        interproc,
        false,
        TaintSourceContext::for_module_function(registry, &cu.ir_module, &cu.top_level),
    );
    let top_taints = run_propagation(
        &TaintGraph::new(&cu.top_level.cfg, &cu.top_level.ssa, &cu.top_level.sccp),
        &cu.top_level,
        &top_instance_classes,
        registry,
        interproc,
        TaintSourceContext::for_module_function(registry, &cu.ir_module, &cu.top_level),
        external_variable_seeds,
        &summaries,
    );

    // Entry-taint worklist: flow tainted call arguments into callee params,
    // re-propagating each callee until a fixpoint. Every proc is seeded into
    // the queue (so each is propagated at least once, with its accumulated —
    // initially empty — entry taints).
    let mut entry_taints: HashMap<crate::ir::ProcedureId, HashMap<String, TaintLattice>> =
        HashMap::new();
    let mut proc_taints: HashMap<crate::ir::ProcedureId, HashMap<ValueKey, TaintLattice>> =
        HashMap::new();
    for qname in &proc_names {
        entry_taints.insert((*qname).clone(), HashMap::new());
        proc_taints.insert((*qname).clone(), HashMap::new());
    }

    let mut queue: VecDeque<crate::ir::ProcedureId> =
        proc_names.iter().map(|q| (*q).clone()).collect();
    let mut queued: HashSet<crate::ir::ProcedureId> = queue.iter().cloned().collect();

    // Initial flows from the top level.
    let initial_flows = resolve_call_flows(
        &cu.top_level,
        &top_taints,
        &top_instance_classes,
        registry,
        interproc,
        TaintSourceContext::for_module_function(registry, &cu.ir_module, &cu.top_level),
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
    // per procedure and cached rather than once per dequeue.
    let mut graphs: HashMap<crate::ir::ProcedureId, TaintGraph<'_>> = HashMap::new();
    let mut instance_classes_by_proc: HashMap<crate::ir::ProcedureId, LocalInstanceClasses> =
        HashMap::new();

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
            .or_insert_with(|| {
                instance_classes_for_function(
                    &fu.cfg,
                    registry,
                    interproc,
                    true,
                    TaintSourceContext::for_module_function(registry, &cu.ir_module, fu),
                )
            });
        let taints = run_propagation(
            graph,
            fu,
            instance_classes,
            registry,
            interproc,
            TaintSourceContext::for_module_function(registry, &cu.ir_module, fu),
            Some(&entry),
            &summaries,
        );

        let flows = resolve_call_flows(
            fu,
            &taints,
            instance_classes,
            registry,
            interproc,
            TaintSourceContext::for_module_function(registry, &cu.ir_module, fu),
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
        // Mutation guard for the basis-name -> lattice mapping: pins each
        // `TaintBasis` variant to its exact expected `TaintColour` set.
        // Swapping which colour two variants map to (e.g. giving `Path`
        // `NON_DASH_PREFIXED` and `NonDash` `PATH_PREFIXED`) flips two of
        // these assertions and fails the test — the mapping can drift without
        // breaking exhaustiveness, so this is the net exhaustiveness alone
        // does not provide.
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
    fn summary_dependencies_read_original_static_heads_under_source_grammar() {
        // naming.compiler.original-summary-dependency-source-heads
        // docs/design/analysis/name-resolution-proofs/compiler-original-summary-dependency-source-heads.md
        // Lexical memo dependencies supply no selected command or execution.
        let known: HashSet<_> = ["::foo", "::café", "::ns::local", "::unused"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let config =
            tcl_lexer::LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
        let dependencies = command_subst_callees_with_config(
            r#"return ["foo"] [{café}] [\u0066oo] [local]"#,
            "::ns::caller",
            &known,
            config,
        );
        assert_eq!(dependencies, ["::café", "::foo", "::ns::local"]);
        assert_eq!(
            command_subst_callees_with_config("return [foo; café]", "::ns::caller", &known, config),
            ["::café", "::foo"],
        );
        assert!(
            command_subst_callees_with_config("return []", "::ns::caller", &known, config)
                .is_empty()
        );
    }

    #[test]
    fn summary_dependencies_keep_unresolved_and_dialect_residuals() {
        // naming.compiler.original-summary-dependency-source-heads
        // docs/design/analysis/name-resolution-proofs/compiler-original-summary-dependency-source-heads.md
        let known: HashSet<_> = ["::foo", "::other"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let config =
            tcl_lexer::LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
        for source in ["return [$head]", "return [installed_alias]", "return [foo"] {
            assert_eq!(
                command_subst_callees_with_config(source, "::caller", &known, config),
                ["::foo", "::other"],
                "unresolved source dependency: {source}",
            );
        }
        assert_eq!(
            command_subst_callees_with_config(r"return [\U00000066oo]", "::caller", &known, config),
            ["::foo"],
        );
        let older =
            tcl_lexer::LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.4"));
        assert_eq!(
            command_subst_callees_with_config(r"return [\U00000066oo]", "::caller", &known, older),
            ["::foo", "::other"],
        );
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
        assert_eq!(warnings(src), [] as [crate::taint::TaintWarning; 0]);
    }

    #[test]
    fn original_return_substitution_flows_arguments_into_the_actual_callee() {
        let source = "proc sink {argument} {eval $argument}\n\
            proc wrapper {value} {return [sink $value]}\n\
            set input [gets stdin]\nwrapper $input\n";
        let findings = warnings(source);
        assert!(
            findings.iter().any(|finding| {
                finding.code == DiagCode::T100
                    && finding.variable == "argument"
                    && finding.sink_command == "eval"
            }),
            "the original returned invocation must taint its actual callee's parameter: {findings:?}"
        );
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
    fn original_return_lookup_without_a_target_cannot_supply_a_clean_summary() {
        let (context, cu) = crate::interprocedural::logical_completion_unit(
            "proc wrapper {} {return [missing]}",
            "tcl8.6",
        );
        let body = *cu.ir_module.procedures.keys().next().unwrap();
        assert!(cu.procedures[&body].cfg.blocks.values().any(|block| {
            matches!(
                &block.terminator,
                Some(Terminator::Return {
                    tokens: Some(_),
                    value_word: Some(_),
                    ..
                })
            )
        }));
        let summary = infer_proc_summary(
            &cu,
            body,
            context.commands(),
            cu.interproc.as_ref(),
            None,
            &HashSet::from([body]),
            &HashMap::new(),
        )
        .expect("actual original wrapper declaration");
        assert_eq!(summary.return_base, TaintLattice::tainted());
    }

    #[test]
    fn self_recursive_proc_taint_summary_converges() {
        // A directly self-recursive proc — `fib` calling itself inside
        // `[expr {[fib …]}]` — has no self-call edge in `direct_calls` (it is
        // buried in a braced `expr`). A worklist that does not re-queue a proc
        // on its own summary change therefore under-converges the interproc
        // taint fixpoint, the debug-only convergence guard panics, and the LSP
        // diagnostic worker publishes nothing. Re-queueing on a proc's own
        // change is what lets the fixpoint settle and the guard hold.
        let reg = CommandRegistry::build_default();
        let src = "proc fib {n} {\n    if {$n < 2} { return $n }\n    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]\n}\nputs \"fib(10) = [fib 10]\"\n";
        let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(&reg, None);
        // Exercises `converge_summaries_with` and its debug fixpoint guard.
        let _ = solve_interprocedural_taints(&cu, &reg, None);
        // The full taint pass over the same source must also complete cleanly.
        let _ = warnings(src);
    }

    fn fixture_body(cu: &CompilationUnit, display: &str) -> ProcedureId {
        *cu.ir_module
            .procedures
            .unique_display(display)
            .expect("fixture has one independently retained declaration")
            .0
    }

    fn summaries_for_unit(
        cu: &CompilationUnit,
        registry: &CommandRegistry,
    ) -> HashMap<ProcedureId, ProcTaintSummary> {
        converge_summaries_with(
            cu,
            registry,
            cu.interproc.as_ref(),
            None,
            &mut |unit, id, interproc, known, summaries| {
                infer_proc_summary(unit, id, registry, interproc, None, known, summaries)
                    .expect("actual fixture body")
            },
        )
    }

    fn command_subst_callees_with_config(
        body: &str,
        caller: &str,
        known: &HashSet<String>,
        config: tcl_lexer::LexerConfig,
    ) -> Vec<String> {
        let mut declarations: Vec<_> = known.iter().collect();
        declarations.sort();
        let mut source = String::from("namespace eval ::ns {}\n");
        for name in declarations {
            source.push_str(&format!("proc {name} {{}} {{}}\n"));
        }
        source.push_str(&format!("proc {caller} {{}} {{{body}}}\n"));
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for_with_config(&source, &registry, false, config);
        let known_ids: HashSet<_> = known
            .iter()
            .filter_map(|name| {
                cu.ir_module
                    .procedures
                    .unique_display(name)
                    .map(|(id, _)| *id)
            })
            .collect();
        assert_eq!(
            known_ids.len(),
            known.len(),
            "every dependency fixture retains its actual declaration"
        );
        let ids = if let Some((id, _)) = cu.ir_module.procedures.unique_display(caller) {
            command_subst_callees(&cu, *id, &known_ids, &registry)
        } else {
            known_ids.iter().copied().collect()
        };
        let mut displays: Vec<_> = ids
            .into_iter()
            .map(|id| {
                cu.ir_module
                    .procedures
                    .get(&id)
                    .unwrap()
                    .qualified_name
                    .clone()
            })
            .collect();
        displays.sort();
        displays
    }

    /// Build the whole-module summary map the way the solver does, so a test
    /// can compare summaries rather than only the diagnostics they produce.
    fn summaries_for(src: &str) -> HashMap<String, ProcTaintSummary> {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(&reg, None);
        let mut shown = HashMap::new();
        for (id, summary) in summaries_for_unit(&cu, &reg) {
            let body = cu
                .ir_module
                .procedures
                .get(&id)
                .expect("actual taint fixture body");
            assert!(
                shown.insert(body.qualified_name.clone(), summary).is_none(),
                "authored taint fixture labels must be unique"
            );
        }
        shown
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
        let id = fixture_body(&cu, "::id");
        let known = HashSet::from([id]);
        crate::taint::reset_instance_class_solve_count();

        let summary = infer_proc_summary(&cu, id, &reg, interproc, None, &known, &HashMap::new())
            .expect("actual fixture body");

        assert_eq!(
            summary.return_by_param_basis[0].1.len(),
            TaintBasis::ALL.len()
        );
        assert_eq!(crate::taint::instance_class_solve_count(), 1);
    }

    /// The unpruned reference: re-derive one procedure's summary without the
    /// prunes, by seeding every
    /// `(parameter, basis)` pair with a full dataflow solve.  A test can then
    /// assert the pruned result is bit-identical rather than merely plausible.
    fn summary_without_prunes(src: &str, target: &str) -> ProcTaintSummary {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(&reg, None);
        let interproc = cu.interproc.as_ref();
        let summaries = summaries_for_unit(&cu, &reg);
        let known: HashSet<_> = summaries.keys().copied().collect();
        let id = fixture_body(&cu, target);
        let proc = cu.ir_module.procedures.get(&id).unwrap();
        let fu = &cu.procedures[&id];

        let graph = TaintGraph::new(&fu.cfg, &fu.ssa, &fu.sccp);
        let instance_classes = local_instance_classes(&fu.cfg, &reg);
        let base = run_propagation(
            &graph,
            fu,
            &instance_classes,
            &reg,
            interproc,
            TaintSourceContext::for_module_function(&reg, &cu.ir_module, fu),
            None,
            &summaries,
        );
        let ctx = return_ctx(
            fu,
            &reg,
            interproc,
            TaintSourceContext::for_module_function(&reg, &cu.ir_module, fu),
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
                    TaintSourceContext::for_module_function(&reg, &cu.ir_module, fu),
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
    /// the unpruned computation.
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
    fn opaque_native_call_cannot_produce_a_clean_constant_return_summary() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut unit = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {value} {return LITERAL}",
            registry,
            false,
        );
        let id = fixture_body(&unit, "::p");
        let function = unit.procedures.get_mut(&id).unwrap();
        function
            .cfg
            .blocks
            .get_mut(&function.cfg.entry)
            .unwrap()
            .statements
            .push(crate::ir::native_call_for_test(b"opaque \xff"));
        let summary = infer_proc_summary(
            &unit,
            id,
            registry,
            None,
            registry.profile(),
            &HashSet::from([id]),
            &HashMap::new(),
        )
        .expect("actual fixture body");
        assert_eq!(summary.return_base, TaintLattice::tainted());
        assert_eq!(
            apply_proc_return_summary(&summary, &[TaintLattice::clean()]),
            TaintLattice::tainted()
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
        // The original `eval` operand contains only the selected procedure's
        // clean result. An input read inside that procedure call cannot supply
        // bytes to the outer sink independently of the returned value.
        let src = "proc lit {v} { return safe }\nset x [gets stdin]\neval [lit $x]\n";
        let w = warnings(src);
        assert!(w.is_empty(), "constant result must remain clean: {w:?}");
        let w = warnings("proc lit {v} { return safe }\nset x [gets stdin]\neval [lit $x]$x\n");
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

#[cfg(test)]
mod original_return_taint_word_purpose_tests {
    use super::*;

    #[test]
    fn original_return_word_keeps_bom_and_actual_ssa_exit_versions() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        // Lexical SSA-use projection only; no executable return-taint closure.
        let config = tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Skip,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            strict_quoting: true,
            ..tcl_lexer::LexerConfig::default()
        };
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for_with_config(
            "\u{feff}set x VALUE\nreturn $x",
            &registry,
            false,
            config,
        );
        let function = &unit.top_level;
        assert_eq!(function.source_lexer_config(), config);
        let symbol = function.ssa.var_symbol("x").expect("actual SSA variable");
        let versions = function
            .ssa
            .blocks
            .values()
            .find_map(|block| {
                block
                    .exit_versions
                    .contains_key(&symbol)
                    .then_some(&block.exit_versions)
            })
            .expect("actual exit version");
        let expected = HashMap::from([(symbol, versions[&symbol])]);
        let text = "\u{feff}#prefix $x $not_in_this_ssa";
        assert_eq!(
            word_uses_from_versions(
                text,
                versions,
                &function.ssa,
                function.source_lexer_config()
            ),
            expected
        );
        assert_eq!(
            word_uses_from_versions(text, versions, &function.ssa, config.nested()),
            expected
        );
        assert!(
            word_uses_from_versions("{\u{feff}#prefix $x}", versions, &function.ssa, config)
                .is_empty()
        );
        assert_eq!(config.leading_bom, tcl_lexer::LeadingBom::Skip);
    }
}
