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
use crate::taint::{TaintColour, TaintCtx, TaintLattice, propagate_taints, word_taint};
use crate::value_shapes::parse_command_substitution;

// Basis lattices

/// Ordered taint-colour bases. The
/// per-parameter return scenarios are computed by seeding the parameter with
/// each basis lattice in turn.
const BASIS_ORDER: [&str; 15] = [
    "generic",
    "path",
    "non_dash",
    "crlf_free",
    "shell_atom",
    "list_canonical",
    "regex_literal",
    "path_normalised",
    "path_bounded",
    "header_token_safe",
    "html_escaped",
    "url_encoded",
    "ip",
    "port",
    "fqdn",
];

/// The taint lattice each basis name seeds a parameter with.
fn basis_lattice(basis: &str) -> TaintLattice {
    let t = TaintColour::TAINTED;
    let colour = match basis {
        "path" => t | TaintColour::PATH_PREFIXED,
        "non_dash" => t | TaintColour::NON_DASH_PREFIXED,
        "crlf_free" => t | TaintColour::CRLF_FREE,
        "shell_atom" => t | TaintColour::SHELL_ATOM,
        "list_canonical" => t | TaintColour::LIST_CANONICAL,
        "regex_literal" => t | TaintColour::REGEX_LITERAL,
        "path_normalised" => t | TaintColour::PATH_NORMALISED,
        // PATH_BOUNDED is set alongside PATH_NORMALISED (the basis entry is
        // reserved for future branch-dependent refinement).
        "path_bounded" => t | TaintColour::PATH_NORMALISED | TaintColour::PATH_BOUNDED,
        "header_token_safe" => t | TaintColour::HEADER_TOKEN_SAFE,
        "html_escaped" => t | TaintColour::HTML_ESCAPED,
        "url_encoded" => t | TaintColour::URL_ENCODED,
        "ip" => t | TaintColour::IP_ADDRESS,
        "port" => t | TaintColour::PORT,
        "fqdn" => t | TaintColour::FQDN,
        _ => t,
    };
    TaintLattice { colours: colour }
}

/// Basis names whose lattice colour intersects `taint`'s colour, excluding
/// `"generic"` (falling back to `["generic"]` when none match).
fn basis_names_for_taint(taint: TaintLattice) -> Vec<&'static str> {
    if !taint.is_tainted() {
        return Vec::new();
    }
    let mut names: Vec<&'static str> = BASIS_ORDER
        .iter()
        .copied()
        .filter(|basis| {
            *basis != "generic" && basis_lattice(basis).colours.intersects(taint.colours)
        })
        .collect();
    if names.is_empty() {
        names.push("generic");
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
    /// Per-parameter return taint, indexed by [`BASIS_ORDER`]: for each
    /// `(param, [taint_per_basis])`, `taint_per_basis[i]` is the taint the
    /// call returns when `param` is seeded with `BASIS_ORDER[i]`'s lattice.
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
                .map(|p| (p.clone(), vec![clean; BASIS_ORDER.len()]))
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
            let idx = BASIS_ORDER.iter().position(|b| *b == basis).unwrap_or(0);
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
) -> HashMap<Symbol, u32> {
    let mut uses: HashMap<Symbol, u32> = HashMap::new();
    let source_map = SourceMap::new(text);
    let Ok(tokens) = Lexer::new(text).tokenise_all() else {
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
            value: Some(value), ..
        }) = &block.terminator
        else {
            continue;
        };
        let Some(ssa_block) = fu.ssa.blocks.get(bn) else {
            continue;
        };
        let uses = word_uses_from_versions(value, &ssa_block.exit_versions, &fu.ssa);
        ret = ret.join(word_taint(value, &uses, taints, ctx));
    }
    ret
}

/// Run intra-procedural taint propagation over `fu` with the given entry
/// taints and summaries. A thin wrapper threading the common arguments.
fn run_propagation(
    fu: &FunctionUnit,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    dialect: Option<&str>,
    param_taints: Option<&HashMap<String, TaintLattice>>,
    summaries: &HashMap<String, ProcTaintSummary>,
) -> HashMap<ValueKey, TaintLattice> {
    propagate_taints(
        &fu.cfg,
        &fu.ssa,
        &fu.sccp,
        registry,
        Some(&fu.rendered_props),
        interproc,
        dialect,
        param_taints,
        Some(summaries),
    )
}

/// Build a [`TaintCtx`] for evaluating return-value words of `fu`.
fn return_ctx<'a>(
    fu: &'a FunctionUnit,
    registry: &'a CommandRegistry,
    interproc: Option<&'a crate::interprocedural::InterproceduralAnalysis>,
    dialect: Option<&'a str>,
    known: &'a HashSet<String>,
    summaries: &'a HashMap<String, ProcTaintSummary>,
) -> TaintCtx<'a> {
    TaintCtx {
        registry,
        ssa: &fu.ssa,
        interproc,
        known_procs: Some(known),
        caller_qname: Some(fu.ssa.name.as_str()),
        dialect,
        taint_summaries: Some(summaries),
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
    dialect: Option<&str>,
    known: &HashSet<String>,
    summaries: &HashMap<String, ProcTaintSummary>,
) -> ProcTaintSummary {
    let base_taints = run_propagation(fu, registry, interproc, dialect, None, summaries);
    let ctx = return_ctx(fu, registry, interproc, dialect, known, summaries);
    let return_base = collect_return_taint(fu, &base_taints, ctx);

    let mut by_param_basis: Vec<(String, Vec<TaintLattice>)> = Vec::with_capacity(params.len());
    for param in params {
        let mut scenario_values: Vec<TaintLattice> = Vec::with_capacity(BASIS_ORDER.len());
        for basis in BASIS_ORDER {
            let mut seed: HashMap<String, TaintLattice> = HashMap::new();
            seed.insert(param.clone(), basis_lattice(basis));
            let seeded = run_propagation(fu, registry, interproc, dialect, Some(&seed), summaries);
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
fn resolve_call_flows(
    fu: &FunctionUnit,
    taints: &HashMap<ValueKey, TaintLattice>,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    dialect: Option<&str>,
    known: &HashSet<String>,
    summaries: &HashMap<String, ProcTaintSummary>,
) -> Vec<(String, Vec<TaintLattice>)> {
    let ctx = return_ctx(fu, registry, interproc, dialect, known, summaries);
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

            let arg_taints: Vec<TaintLattice> = cmd_args
                .iter()
                .map(|arg| word_taint(arg, &ssa_stmt.uses, taints, ctx))
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
/// `registry` and `dialect` feed only the debug-only round-robin guard below, so
/// in a release build (where `debug_assertions` is off and the guard is compiled
/// out) they are genuinely unused.
#[cfg_attr(not(debug_assertions), allow(unused_variables))]
pub fn converge_summaries_with(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    dialect: Option<&str>,
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
    let callers: Option<HashMap<&str, Vec<&str>>> = interproc.map(|ia| {
        fn add<'a>(map: &mut HashMap<&'a str, Vec<&'a str>>, callee: &'a str, from: &'a str) {
            let entry = map.entry(callee).or_default();
            if !entry.contains(&from) {
                entry.push(from);
            }
        }
        let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
        for (caller, summary) in &ia.procedures {
            for callee in &summary.direct_calls {
                add(&mut map, callee.as_str(), caller.as_str());
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
                    add(&mut map, interned.as_str(), qname.as_str());
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
/// Rust-specific conservative global-write seeding and the dialect handling
/// match the per-function [`crate::compilation_unit::FunctionUnit::taints`]
/// baseline; on top of that the colour-aware return summaries and parameter
/// entry taints are applied.
#[must_use]
pub fn solve_interprocedural_taints(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> InterprocTaintResult {
    let interproc = cu.interproc.as_ref();
    solve_interprocedural_taints_with(
        cu,
        registry,
        dialect,
        &mut |qname, params, fu, known, summaries| {
            infer_proc_summary(
                qname, params, fu, registry, interproc, dialect, known, summaries,
            )
        },
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
    dialect: Option<&str>,
    infer_fn: &mut InferProcSummaryFn<'_>,
) -> InterprocTaintResult {
    let interproc = cu.interproc.as_ref();
    let summaries = converge_summaries_with(cu, registry, interproc, dialect, infer_fn);

    // Procedures in deterministic (sorted) order, and the name set — both consumed
    // by the entry-taint worklist below.
    let mut proc_names: Vec<&String> = cu.ir_module.procedures.keys().collect();
    proc_names.sort();
    let known: HashSet<String> = summaries.keys().cloned().collect();

    // Top-level taints under the converged summaries.
    let top_taints = run_propagation(
        &cu.top_level,
        registry,
        interproc,
        dialect,
        None,
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

    while let Some(qname) = queue.pop_front() {
        queued.remove(&qname);
        let Some(fu) = cu.procedures.get(&qname) else {
            continue;
        };
        let entry = entry_taints.get(&qname).cloned().unwrap_or_default();
        let taints = run_propagation(fu, registry, interproc, dialect, Some(&entry), &summaries);

        let flows = resolve_call_flows(
            fu, &taints, registry, interproc, dialect, &known, &summaries,
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
