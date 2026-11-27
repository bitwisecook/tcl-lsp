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

//! Inferring one procedure's return-taint summary: return-word uses, the joined return taint and the per-parameter probes.

use super::{
    ProcSummaryInputs, ProcTaintSummary, TaintBasis, formal_names_match, source_formal_arity,
};
use crate::cfg::Terminator;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::ProcedureId;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::taint::{
    LocalInstanceClasses, TaintCtx, TaintGraph, TaintLattice, TaintPropagationInputs,
    TaintSourceContext, instance_classes_for_function, propagate_taints, word_taint,
};
use std::collections::{HashMap, HashSet};
use tcl_registry::{Arity, CommandRegistry};

// Return-taint collection

/// Possible lexical return dependencies under the actual word grammar.
/// Only existing function symbols and exit versions enter the use map.
pub(super) struct ReturnWordUses {
    pub(super) uses: HashMap<Symbol, u32>,
    pub(super) unknown: bool,
}

pub(super) fn original_return_word_uses(
    text: &str,
    word: Option<&tcl_source::words::WordExpr>,
    versions: &HashMap<Symbol, u32>,
    ssa: &SsaFunction,
    config: tcl_lexer::LexerConfig,
) -> ReturnWordUses {
    use tcl_source::words::WordExpr;
    use tcl_source::words::WordPart;
    let Some(word) = word else {
        return return_word_uses_from_versions(text, versions, ssa, config);
    };
    let mut out = ReturnWordUses {
        uses: HashMap::new(),
        unknown: false,
    };
    let mut pending = vec![word];
    let mut spellings = Vec::new();
    while let Some(word) = pending.pop() {
        match word {
            WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => {}
            WordExpr::Variable { spelling, .. } => {
                out.unknown |= !matches!(
                    tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config),
                    Ok(Some(_))
                );
                spellings.push(spelling.as_str());
            }
            WordExpr::CommandSubstitution { spelling, .. } => spellings.push(spelling.as_str()),
            WordExpr::Template {
                parts, rejected, ..
            } => {
                out.unknown |= rejected.is_some();
                for part in parts {
                    match part {
                        WordPart::Text { .. } => {}
                        WordPart::Variable { spelling, .. } => {
                            out.unknown |= !matches!(
                                tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config),
                                Ok(Some(_))
                            );
                            spellings.push(spelling.as_str());
                        }
                        WordPart::CommandSubstitution { spelling, .. } => {
                            spellings.push(spelling.as_str());
                        }
                        WordPart::Opaque { text, .. } => {
                            out.unknown = true;
                            spellings.push(text.as_str());
                        }
                    }
                }
            }
            WordExpr::Expand { word, .. } => {
                out.unknown = true;
                pending.push(word);
            }
            WordExpr::Opaque { text, .. } => {
                out.unknown = true;
                spellings.push(text.as_str());
            }
        }
    }
    for spelling in spellings {
        let candidates =
            tcl_syntax::naming::substitution_variable_candidates(spelling.as_bytes(), config);
        out.unknown |= candidates.has_unknown_residual();
        for name in candidates.names() {
            match ssa.var_symbol(name) {
                // A name the block never defines still holds its entry value.
                Some(symbol) => {
                    out.uses
                        .insert(symbol, versions.get(&symbol).copied().unwrap_or(0));
                }
                None => out.unknown = true,
            }
        }
    }
    if out.unknown {
        out.uses.extend(versions);
    }
    out
}

pub(super) fn return_word_uses_from_versions(
    text: &str,
    versions: &HashMap<Symbol, u32>,
    ssa: &SsaFunction,
    config: tcl_lexer::LexerConfig,
) -> ReturnWordUses {
    let mut out = ReturnWordUses {
        uses: HashMap::new(),
        unknown: false,
    };
    let config = tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config);
    let image = tcl_lexer::SourceImage::document(text);
    let plan = u32::try_from(image.len()).ok().and_then(|end| {
        tcl_lexer::native_script_words_in_with_purpose(
            image,
            tcl_lexer::Span::new(0, end),
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()
    });
    let Some(plan) = plan else {
        return ReturnWordUses {
            uses: versions.clone(),
            unknown: true,
        };
    };
    out.unknown = plan.fatal_tail.is_some();
    for command in plan.commands {
        for word in command.words {
            if word.group().kind == tcl_lexer::WordKind::Braced {
                continue;
            }
            let Some(original) = text.as_bytes().get(word.word_span().as_range()) else {
                out.unknown = true;
                continue;
            };
            let candidates = tcl_syntax::naming::substitution_variable_candidates(original, config);
            out.unknown |= candidates.has_unknown_residual();
            for name in candidates.names() {
                let Some(symbol) = ssa.var_symbol(name) else {
                    out.unknown = true;
                    continue;
                };
                let Some(&version) = versions.get(&symbol) else {
                    out.unknown = true;
                    continue;
                };
                out.uses.insert(symbol, version);
            }
        }
    }
    if out.unknown {
        out.uses.extend(versions);
    }
    out
}

#[cfg(test)]
pub(super) fn word_uses_from_versions(
    text: &str,
    versions: &HashMap<Symbol, u32>,
    ssa: &SsaFunction,
    config: tcl_lexer::LexerConfig,
) -> HashMap<Symbol, u32> {
    return_word_uses_from_versions(text, versions, ssa, config).uses
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
pub(super) fn collect_return_taint(
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
        let inventory = original_return_word_uses(
            value,
            value_word.as_ref(),
            &ssa_block.exit_versions,
            &fu.ssa,
            ctx.lexer_config(),
        );
        let returned = TaintCtx {
            source_tokens: tokens.as_deref(),
            source_word: value_word.as_ref(),
            word_offset: value_word.as_ref().map(|word| word.source().span.start()),
            ..ctx.at(span.map_or(0, tcl_lexer::Span::start))
        };
        if inventory.unknown
            && !matches!(
                value_word,
                Some(
                    tcl_source::words::WordExpr::Literal { .. }
                        | tcl_source::words::WordExpr::BracedLiteral { .. }
                )
            )
        {
            ret = ret.join(TaintLattice::tainted());
        }
        // A return word that is one variable read returns that variable's
        // value at the block's exit, which the inventory already resolved.
        let copied = value_word
            .as_ref()
            .and_then(tcl_source::words::WordExpr::sole_variable_substitution)
            .filter(|_| !inventory.unknown && inventory.uses.len() == 1)
            .and_then(|_| inventory.uses.iter().next())
            .map(|(&symbol, &version)| {
                taints
                    .get(&(symbol, version))
                    .copied()
                    .unwrap_or_else(TaintLattice::clean)
            });
        ret = ret
            .join(copied.unwrap_or_else(|| word_taint(value, &inventory.uses, taints, returned)));
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
pub(super) fn run_propagation(
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
pub(super) fn return_ctx<'a>(
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
    let procedure = cu.original_procedure_for_body(body_id)?;
    let fu = cu.function_for_body(Some(body_id))?;
    if fu.body_id != Some(body_id) {
        return None;
    }
    Some(infer_proc_summary_with_source(ProcSummaryInputs {
        qname: &procedure.qualified_name,
        params: &procedure.params,
        fu,
        registry,
        interproc,
        source: TaintSourceContext::for_unit_may(registry, cu, fu),
        source_formals: TaintSourceContext::formals_for_unit_body(cu, body_id, registry),
        known,
        summaries,
    }))
}

pub(super) fn infer_proc_summary_with_source(inputs: ProcSummaryInputs<'_>) -> ProcTaintSummary {
    let ProcSummaryInputs {
        qname,
        params,
        fu,
        registry,
        interproc,
        source,
        source_formals,
        known,
        summaries,
    } = inputs;

    if !source_formals
        .as_ref()
        .is_some_and(|formals| formal_names_match(params, formals))
    {
        return ProcTaintSummary::unknown_for_source(qname, params, source_formals);
    }

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
            source_formals,
            source_formals_required: true,
        };
    }
    // Prune 1 — the return taint does not depend on the taint map at all, so
    // neither the clean base nor any of the `17P` seeded scenarios needs a
    // solve.  Every scenario is `clean`, which is exactly what
    // `ProcTaintSummary::untainted` builds.  Checked before the index build
    // below, so a constant-return procedure costs nothing at all.
    if return_taint_is_constant(fu) {
        return ProcTaintSummary::untainted_for_source(qname, params, source_formals);
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
        arity: source_formal_arity(source_formals.as_ref().expect("validated source formals"))
            .unwrap_or_else(Arity::any),
        return_base,
        return_by_param_basis: by_param_basis,
        source_formals,
        source_formals_required: true,
    }
}
