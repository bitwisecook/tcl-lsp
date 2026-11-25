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

//! The whole-unit taint pass: module variable traces, the per-function reports and the report with replay availability.

use std::collections::HashMap;

use tcl_registry::CommandRegistry;

use super::callback_replay::taint_callback_substitution_report;
use super::destructive_files::find_destructive_file_warnings;
use super::propagation::collect_global_reads;
use super::setter_constraints::find_setter_constraint_warnings;
use super::sink_warnings::{
    shadowed_builtin_names_for_module_function, taint_warning_report_for_function_with_identities,
};
use super::source_context::TaintSourceContext;
use super::source_warnings::{
    find_destructive_file_warnings_for_module_function,
    find_setter_constraint_warnings_for_module_function,
};
use super::{TaintLattice, TaintReplayAnalysis, TaintWarning, TaintWarningReport};
use crate::ssa::{SsaFunction, ValueKey};

/// Conservatively force every taint value touching a module-traced variable
/// to fully tainted — the "unknown taint" fix for `trace add variable` /
/// `trace variable`.
///
/// `trace add variable x write {handler}` (or a `read` trace) lets a runtime
/// callback rewrite `x` on every future access in ways static analysis
/// cannot see: the handler might sanitise a value the analysis considers
/// tainted (so a warning is arguably unwarranted), or it might inject
/// attacker-controlled content into a value the source only ever assigns
/// literals (so a real sink would be silently missed). Rather than model
/// trace semantics, this takes the safe side of that ambiguity — treat a
/// traced name as tainted everywhere, so the analysis never launders a
/// value through an opaque handler it can't inspect. This also incidentally
/// fixes a sharper, pre-existing bug: `trace add variable x write …` is
/// itself registered as a *write* to `x` (`ArgRole::VarWrite`, so SSA gives
/// it a fresh version for soundness elsewhere), and the fallback "propagate
/// from arguments" rule in [`evaluate_taint_def`] computed that fresh
/// version's taint from the trace-add call's own *literal* argument words
/// (`add`, `variable`, `x`, `write`, `{handler}` — none of them a `$`
/// reference) — always clean, silently dropping any taint `x` carried
/// *before* the trace was installed.
///
/// Uses [`crate::ir::Module::traced_variables`] /
/// [`crate::ir::Module::has_dynamic_variable_trace`] — the same
/// flow-insensitive, whole-module over-approximation (the same "traced
/// anywhere ⇒ traced everywhere, for the module's lifetime" rule already
/// applied to the optimiser's constant-fold trust gate via
/// [`crate::compilation_unit::ModuleTraceFacts`]) rather than a bespoke
/// traced-name scan.
///
/// Two passes:
/// 1. Every `(symbol, version)` already in `taints` for a traced name is
///    forced to [`TaintLattice::tainted()`] — this is every non-zero SSA
///    version, since [`propagate_statement_taints`] always inserts an entry
///    for a def, tainted or not.
/// 2. A traced namespace-owned SSA cell read at version 0 with **no** local
///    def in this function at all (a pure cross-proc read reusing
///    [`collect_global_reads`]) is seeded tainted too, so a function that
///    only reads a traced global still sees it as tainted even without the
///    interprocedural global-write seeding this mirrors.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn apply_module_variable_traces(
    mut taints: HashMap<ValueKey, TaintLattice>,
    ssa: &SsaFunction,
    traces: crate::compilation_unit::ModuleTraceFacts<'_>,
) -> HashMap<ValueKey, TaintLattice> {
    for (key, lattice) in &mut taints {
        if traces.variable_may_be_traced(ssa.var_name(key.0)) {
            *lattice = TaintLattice::tainted();
        }
    }
    for symbol in collect_global_reads(ssa) {
        if traces.variable_may_be_traced(ssa.var_name(symbol)) {
            taints
                .entry((symbol, 0))
                .and_modify(|t| *t = TaintLattice::tainted())
                .or_insert_with(TaintLattice::tainted);
        }
    }
    taints
}

/// Find every taint warning across a whole compilation unit.
///
/// Public `*_for_cu` entry point (mirroring
/// [`crate::shimmer::find_shimmer_warnings_for_cu`]) composing the
/// `TaintWarning`-producing passes — sink detection, setter-constraint
/// violations, iRules URI-split suggestions, and destructive-file warnings
/// — over each function in the unit, in `cu.functions()` order. The
/// path-concatenation pass is omitted: its lattice colours are not yet
/// assigned (latent, always empty today).
#[must_use]
pub fn find_taint_warnings_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<TaintWarning> {
    taint_warning_report_for_cu(cu, registry, dialect).warnings
}

/// Analyse proven warnings while retaining every required replay refusal.
#[must_use]
pub fn taint_warning_report_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> TaintWarningReport {
    let mut out =
        taint_warning_report_for_cu_base_with_external_variable_seeds(cu, registry, dialect, None);
    let callback_warnings =
        taint_callback_substitution_report(cu, registry, dialect, &out.warnings);
    out.merge(callback_warnings);
    out
}

/// The ordinary whole-unit taint pass, without a second pass through stored
/// callback text. Kept separate so callback replay can analyse a synthetic
/// invocation without recursively replaying every registration in its source
/// module.
#[cfg(test)]
pub(super) fn find_taint_warnings_for_cu_base(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<TaintWarning> {
    find_taint_warnings_for_cu_base_with_external_variable_seeds(cu, registry, dialect, None)
}

/// The ordinary warning pass with explicitly seeded top-level external values.
///
/// Synthetic callback replays use this rather than spelling a source command
/// such as `[gets stdin]`: a registration's user input is independent of
/// source-level command shadowing and aliases.
#[cfg(test)]
fn find_taint_warnings_for_cu_base_with_external_variable_seeds(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    external_variable_seeds: Option<&HashMap<String, TaintLattice>>,
) -> Vec<TaintWarning> {
    taint_warning_report_for_cu_base_with_external_variable_seeds(
        cu,
        registry,
        dialect,
        external_variable_seeds,
    )
    .warnings
}

fn setter_constraint_warnings_for_unit_function(
    fu: &crate::compilation_unit::FunctionUnit,
    module: &crate::ir::Module,
    registry: &CommandRegistry,
    taints: &HashMap<ValueKey, TaintLattice>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    standalone_warnings: bool,
) -> Vec<TaintWarning> {
    if standalone_warnings {
        find_setter_constraint_warnings(
            registry,
            &fu.cfg,
            &fu.ssa,
            taints,
            Some(&fu.sccp.values),
            &fu.sccp.executable_blocks,
            dialect,
        )
    } else {
        find_setter_constraint_warnings_for_module_function(fu, module, registry, taints)
    }
}

pub(super) fn taint_warning_report_for_cu_base_with_external_variable_seeds(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    external_variable_seeds: Option<&HashMap<String, TaintLattice>>,
) -> TaintWarningReport {
    let mut out = TaintWarningReport::analysed(Vec::new(), Vec::new());
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    // Missing original ownership is terminal for this whole-unit query.
    let Some(identities) =
        crate::realm::document_realm_bindings_for_module(&cu.ir_module, registry)
    else {
        out.replays = TaintReplayAnalysis::unavailable();
        return out;
    };
    let solved = external_variable_seeds.map_or_else(
        || crate::taint_interproc::solve_interprocedural_taints(cu, registry, dialect),
        |seeds| {
            crate::taint_interproc::solve_interprocedural_taints_with_external_variable_seeds(
                cu, registry, dialect, seeds,
            )
        },
    );
    let module_traces = crate::compilation_unit::ModuleTraceFacts {
        traced_variables: &cu.ir_module.traced_variables,
        has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
        deferred_writes: &cu.ir_module.deferred_writes,
    };
    // `analysable_body_function_units` (not `analysable_functions`) so a
    // sink inside a TclOO method body — or an `apply` lambda / `namespace
    // eval` body — is still checked; see its doc comment.
    for fu in cu.analysable_body_function_units() {
        let exec = &fu.sccp.executable_blocks;
        let taints = apply_module_variable_traces(
            solved.taints_for_function(cu, fu).clone(),
            &fu.ssa,
            module_traces,
        );
        let Some(shadowed) = shadowed_builtin_names_for_module_function(fu, cu, registry) else {
            out.replays.merge(TaintReplayAnalysis::unavailable());
            continue;
        };
        out.merge(taint_warning_report_for_function_with_identities(
            fu,
            &taints,
            exec,
            registry,
            TaintSourceContext::for_unit_may(registry, cu, fu)
                .with_procedure_summaries(Some(&solved.summaries)),
            &shadowed,
            (module_traces, &identities, Some(&cu.ir_module)),
        ));
        // Only an explicitly standalone Module and CFG may use the legacy
        // source-warning utilities. Missing or withdrawn supplied input keeps
        // its retained tag and cannot reopen catalogue compatibility. These
        // warnings do not supply Native handler, read or filesystem authority.
        let standalone_warnings = cu.ir_module.source_entry.metadata_context.is_standalone()
            && fu.cfg.metadata_context.is_standalone()
            && cu.ir_module.source_metadata_input.is_none()
            && fu.source_metadata_input().is_none();
        out.warnings
            .extend(setter_constraint_warnings_for_unit_function(
                fu,
                &cu.ir_module,
                registry,
                &taints,
                dialect,
                standalone_warnings,
            ));
        out.warnings
            .extend(crate::uri_split::find_uri_split_suggestions(
                &fu.cfg,
                &fu.ssa,
                Some(&fu.sccp.values),
                exec,
                registry,
                dialect,
            ));
        if standalone_warnings {
            out.warnings.extend(find_destructive_file_warnings(
                &fu.cfg, &fu.ssa, &taints, exec, registry, dialect,
            ));
        } else {
            out.warnings
                .extend(find_destructive_file_warnings_for_module_function(
                    fu,
                    &cu.ir_module,
                    registry,
                    &taints,
                ));
        }
    }
    out
}
