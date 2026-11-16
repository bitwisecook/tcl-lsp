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

use tcl_registry::{Arity, CommandRegistry};

#[cfg(test)]
use crate::cfg::Terminator;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::ProcedureId;
use crate::ssa::ValueKey;
#[cfg(test)]
use crate::taint::local_instance_classes;
use crate::taint::{
    LocalInstanceClasses, TaintColour, TaintGraph, TaintLattice, TaintSourceContext,
    instance_classes_for_function,
};

mod call_flows;
mod summary_fixpoint;
mod summary_inference;

#[cfg(test)]
use call_flows::original_head;
use call_flows::resolve_call_flows;
pub use call_flows::{command_subst_callees, resolved_callees};
pub use summary_fixpoint::converge_summaries_with;
#[cfg(test)]
use summary_inference::return_ctx;
use summary_inference::run_propagation;
pub use summary_inference::{InferProcSummaryFn, infer_proc_summary};
#[cfg(test)]
use summary_inference::{
    collect_return_taint, original_return_word_uses, return_word_uses_from_versions,
    word_uses_from_versions,
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
        TaintLattice::from_colours(colour)
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
    /// Original selected formal topology, when the Module issuer reborrowed it.
    /// This is syntax/binding structure only; it grants no activation.
    pub source_formals:
        Option<crate::signature_scan::formal_parameters::SourceProcedureFormalTopology>,
    /// Production summaries require source formals; absent topology is unknown.
    pub source_formals_required: bool,
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
            source_formals: None,
            source_formals_required: false,
        }
    }

    fn untainted_for_source(
        qname: &str,
        params: &[String],
        source_formals: Option<
            crate::signature_scan::formal_parameters::SourceProcedureFormalTopology,
        >,
    ) -> Self {
        let Some(formals) = source_formals
            .as_ref()
            .filter(|formals| formal_names_match(params, formals))
        else {
            return Self::unknown_for_source(qname, params, source_formals);
        };
        let mut summary = Self::untainted(qname, params);
        summary.arity = source_formal_arity(formals).unwrap_or_else(Arity::any);
        summary.source_formals = source_formals;
        summary.source_formals_required = true;
        summary
    }

    fn unknown_for_source(
        qname: &str,
        params: &[String],
        source_formals: Option<
            crate::signature_scan::formal_parameters::SourceProcedureFormalTopology,
        >,
    ) -> Self {
        let tainted = TaintLattice::tainted();
        Self {
            qualified_name: qname.to_owned(),
            params: params.to_vec(),
            arity: Arity::any(),
            return_base: tainted,
            return_by_param_basis: params
                .iter()
                .map(|parameter| (parameter.clone(), vec![tainted; TaintBasis::ALL.len()]))
                .collect(),
            source_formals,
            source_formals_required: true,
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

fn formal_names_match(
    report_names: &[String],
    formals: &crate::signature_scan::formal_parameters::SourceProcedureFormalTopology,
) -> bool {
    let mut seen = HashSet::new();
    match formals {
        crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Native(
            formals,
        ) => {
            report_names.len() == formals.parameters().len()
                && report_names
                    .iter()
                    .zip(formals.parameters())
                    .all(|(reported, formal)| {
                        reported.as_bytes() == formal.name.as_slice()
                            && seen.insert(formal.name.as_slice().to_vec())
                    })
        }
        crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Logical(
            formals,
        ) => {
            report_names.len() == formals.parameters().len()
                && report_names
                    .iter()
                    .zip(formals.parameters())
                    .all(|(reported, formal)| {
                        reported.as_bytes() == formal.name.as_bytes()
                            && seen.insert(formal.name.as_bytes().to_vec())
                    })
        }
        crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Authored(
            formals,
        ) => {
            report_names.len() == formals.parameters().len()
                && report_names
                    .iter()
                    .zip(formals.parameters())
                    .all(|(reported, formal)| {
                        reported == &formal.name && seen.insert(formal.name.as_bytes().to_vec())
                    })
        }
    }
}

fn source_formal_arity(
    formals: &crate::signature_scan::formal_parameters::SourceProcedureFormalTopology,
) -> Option<Arity> {
    let shape = formals.argument_count_shape();
    let min = u16::try_from(shape.minimum).ok()?;
    let max = shape
        .maximum
        .map(u16::try_from)
        .transpose()
        .ok()?
        .unwrap_or(Arity::UNLIMITED);
    Some(Arity::new(min, max))
}

fn authored_formal_argument_taints(
    formals: &crate::registry_invocation::AuthoredProcedureFormalTopology,
    arguments: &[TaintLattice],
) -> Option<Vec<(String, TaintLattice)>> {
    use tcl_syntax::formal_params::FormalByteArgumentBinding as Binding;
    let mut bound = vec![TaintLattice::clean(); formals.parameters().len()];
    for binding in formals.bindings(arguments.len())? {
        match binding {
            Binding::Value {
                parameter,
                argument,
            } => bound[parameter] = *arguments.get(argument)?,
            Binding::Default { .. } => {}
            Binding::Rest {
                parameter,
                start,
                len,
                ..
            } => {
                for value in arguments.get(start..start.checked_add(len)?)? {
                    bound[parameter] = bound[parameter].join(*value);
                }
            }
            Binding::CallerLink { parameter, .. } => bound[parameter] = TaintLattice::tainted(),
        }
    }
    Some(
        formals
            .parameters()
            .iter()
            .map(|formal| formal.name.clone())
            .zip(bound)
            .collect(),
    )
}

fn source_formal_argument_taints(
    summary: &ProcTaintSummary,
    args: &[TaintLattice],
) -> Option<Vec<(String, TaintLattice)>> {
    let formals = summary.source_formals.as_ref()?;
    if !formal_names_match(&summary.params, formals) {
        return None;
    }
    let mut bound = vec![TaintLattice::clean(); formals.len()];
    match formals {
        crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Native(
            formals,
        ) => {
            for binding in formals.bindings(args.len()).ok()? {
                use tcl_syntax::formal_params::FormalByteArgumentBinding as Binding;
                match binding {
                    Binding::Value {
                        parameter,
                        argument,
                    } => {
                        bound[parameter] = *args.get(argument)?;
                    }
                    Binding::Default { .. } => {}
                    Binding::Rest {
                        parameter,
                        start,
                        len,
                        ..
                    } => {
                        let end = start.checked_add(len)?;
                        for value in args.get(start..end)? {
                            bound[parameter] = bound[parameter].join(*value);
                        }
                    }
                    // Caller-link bytes name a caller location; they are not
                    // the value taint, so preserve uncertainty.
                    Binding::CallerLink { parameter, .. } => {
                        bound[parameter] = TaintLattice::tainted();
                    }
                }
            }
        }
        crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Logical(
            formals,
        ) => {
            let shape = formals.argument_count_shape();
            if !shape.accepts(args.len()) {
                return None;
            }
            let parameters = formals.parameters();
            let mut argument = 0;
            for (parameter, formal) in parameters.iter().enumerate() {
                if parameter + 1 == parameters.len()
                    && tcl_syntax::formal_params::is_variadic_formal_name(&formal.name)
                {
                    for value in args.get(argument..)? {
                        bound[parameter] = bound[parameter].join(*value);
                    }
                    argument = args.len();
                } else if let Some(value) = args.get(argument) {
                    bound[parameter] = *value;
                    argument += 1;
                } else if formal.default.is_none() {
                    return None;
                }
            }
            if argument != args.len() {
                return None;
            }
        }
        crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Authored(
            formals,
        ) => {
            return authored_formal_argument_taints(formals, args);
        }
    }
    Some(summary.params.iter().cloned().zip(bound).collect())
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

    let source_bound = if summary.source_formals_required {
        let Some(bound) = source_formal_argument_taints(summary, arg_taints) else {
            return TaintLattice::tainted();
        };
        Some(bound)
    } else {
        None
    };

    let mut bound: Vec<(String, TaintLattice)> = Vec::with_capacity(summary.params.len());
    if let Some(source_bound) = source_bound {
        bound = source_bound;
    } else if summary
        .params
        .last()
        .is_some_and(|p| tcl_syntax::formal_params::is_variadic_formal_name(p))
    {
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

struct ProcSummaryInputs<'a> {
    qname: &'a str,
    params: &'a [String],
    fu: &'a FunctionUnit,
    registry: &'a CommandRegistry,
    interproc: Option<&'a crate::interprocedural::InterproceduralAnalysis>,
    source: TaintSourceContext<'a>,
    source_formals: Option<crate::signature_scan::formal_parameters::SourceProcedureFormalTopology>,
    known: &'a HashSet<crate::ir::ProcedureId>,
    summaries: &'a HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
}

/// Per-body solved taints and the separate actual top-level solve.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InterprocTaintResult {
    /// Taints for the top-level script.
    pub top_taints: HashMap<ValueKey, TaintLattice>,
    /// Per-procedure taints, keyed by qualified name.
    pub proc_taints: HashMap<crate::ir::ProcedureId, HashMap<ValueKey, TaintLattice>>,
    /// Converged return summaries the taints were solved under.
    pub summaries: HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
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
        if std::ptr::eq(fu, std::ptr::from_ref(&cu.top_level)) {
            return &self.top_taints;
        }
        let Some(id) = fu.body_id else {
            return &fu.taints;
        };
        if cu
            .function_for_body(Some(id))
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

    if summary.source_formals_required {
        let bindings = source_formal_argument_taints(summary, args).unwrap_or_else(|| {
            summary
                .params
                .iter()
                .cloned()
                .map(|parameter| (parameter, TaintLattice::tainted()))
                .collect()
        });
        for (parameter, incoming) in bindings {
            let slot = entry.entry(parameter).or_insert_with(TaintLattice::clean);
            let merged = slot.join(incoming);
            if merged != *slot {
                *slot = merged;
                changed = true;
            }
        }
        return changed;
    }

    if summary
        .params
        .last()
        .is_some_and(|p| tcl_syntax::formal_params::is_variadic_formal_name(p))
    {
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

fn ordered_taint_procedure_ids(cu: &CompilationUnit) -> Vec<&crate::ir::ProcedureId> {
    let mut proc_names: Vec<&crate::ir::ProcedureId> = cu
        .ir_module
        .original_source_procedure_bodies()
        .filter_map(|(id, _)| cu.function_for_body(Some(*id)).map(|_| id))
        .collect();
    proc_names.sort();
    proc_names
}

fn merge_external_taint_seeds(
    entry: &mut HashMap<String, TaintLattice>,
    external_variable_seeds: Option<&HashMap<String, TaintLattice>>,
) {
    if let Some(external_variable_seeds) = external_variable_seeds {
        for (name, taint) in external_variable_seeds {
            let slot = entry
                .entry(name.clone())
                .or_insert_with(TaintLattice::clean);
            *slot = slot.join(*taint);
        }
    }
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
    let proc_names = ordered_taint_procedure_ids(cu);
    let known: HashSet<crate::ir::ProcedureId> = summaries.keys().copied().collect();

    // Top-level taints under the converged summaries.
    let top_instance_classes = instance_classes_for_function(
        &cu.top_level.cfg,
        registry,
        interproc,
        false,
        TaintSourceContext::for_unit_function(registry, cu, &cu.top_level),
    );
    let top_taints = run_propagation(
        &TaintGraph::new(&cu.top_level.cfg, &cu.top_level.ssa, &cu.top_level.sccp),
        &cu.top_level,
        &top_instance_classes,
        registry,
        interproc,
        TaintSourceContext::for_unit_function(registry, cu, &cu.top_level),
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
        entry_taints.insert(**qname, HashMap::new());
        proc_taints.insert(**qname, HashMap::new());
    }

    let mut queue: VecDeque<crate::ir::ProcedureId> = proc_names.iter().map(|q| **q).collect();
    let mut queued: HashSet<crate::ir::ProcedureId> = queue.iter().copied().collect();

    // Initial flows from the top level.
    let initial_flows = resolve_call_flows(
        &cu.top_level,
        &top_taints,
        &top_instance_classes,
        registry,
        interproc,
        TaintSourceContext::for_unit_function(registry, cu, &cu.top_level),
        &known,
        &summaries,
    );
    for (callee, args) in initial_flows {
        if update_entry(&summaries, &mut entry_taints, &callee, &args) && queued.insert(callee) {
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
        let Some(fu) = cu.function_for_body(Some(qname)) else {
            continue;
        };
        let mut entry = entry_taints.get(&qname).cloned().unwrap_or_default();
        merge_external_taint_seeds(&mut entry, external_variable_seeds);
        let graph = graphs
            .entry(qname)
            .or_insert_with(|| TaintGraph::new(&fu.cfg, &fu.ssa, &fu.sccp));
        let instance_classes = instance_classes_by_proc.entry(qname).or_insert_with(|| {
            instance_classes_for_function(
                &fu.cfg,
                registry,
                interproc,
                true,
                TaintSourceContext::for_unit_may(registry, cu, fu),
            )
        });
        let taints = run_propagation(
            graph,
            fu,
            instance_classes,
            registry,
            interproc,
            TaintSourceContext::for_unit_may(registry, cu, fu),
            Some(&entry),
            &summaries,
        );

        let flows = resolve_call_flows(
            fu,
            &taints,
            instance_classes,
            registry,
            interproc,
            TaintSourceContext::for_unit_may(registry, cu, fu),
            &known,
            &summaries,
        );
        proc_taints.insert(qname, taints);

        for (callee, args) in flows {
            if update_entry(&summaries, &mut entry_taints, &callee, &args) && queued.insert(callee)
            {
                queue.push_back(callee);
            }
        }
    }

    InterprocTaintResult {
        top_taints,
        proc_taints,
        summaries,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_return_taint_word_purpose_tests;

#[cfg(test)]
mod consumed_source_dependency_tests;
