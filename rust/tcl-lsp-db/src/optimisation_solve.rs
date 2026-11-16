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

//! The per-procedure optimisation memo and the document's optimisations assembled from it.

#[cfg(test)]
use super::ProcSummary;
use super::TclDb;
use super::function_lattices::{FnLatticeKey, function_lattice};
use std::collections::HashMap;
use std::sync::Arc;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::optimiser::Optimisation;
use tcl_registry::CommandRegistry;

/// Hashable normalisation of a callee's `ConstantReturn` (`f64` isn't `Hash`/`Eq`)
/// for the interned [`OptDepsKey`].
#[cfg(test)]
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ConstReturnKey {
    Int(i64),
    FloatBits(u64),
    Bool(bool),
    Str(String),
}

/// The opt-relevant projection of a direct-callee `ProcSummary` — the fields the
/// optimiser passes read from `cu.interproc` (O103 static-call folding + the
/// purity / effect gates). Hashable, so it can live in the interned [`OptDepsKey`].
#[cfg(test)]
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[allow(clippy::struct_excessive_bools)] // a faithful projection of ProcSummary's fold/purity flags
pub struct OptCalleeSummary {
    pub qname: String,
    pub params: Vec<String>,
    pub can_fold_static_calls: bool,
    pub returns_constant: bool,
    pub constant_return: Option<ConstReturnKey>,
    pub pure: bool,
    pub writes_global: bool,
    pub has_barrier: bool,
    pub has_unknown_calls: bool,
    pub return_passthrough_param: Option<String>,
    pub return_depends_on_params: Vec<String>,
    /// Per-parameter traits (`upvar` / call-by-name / passthrough / …), sorted —
    /// the `call_by_name` O109/O126 suppression reads a callee's by-name params.
    pub param_traits: Vec<(String, Vec<tcl_compiler::interprocedural::ProcArgTrait>)>,
    /// Each parameter's default, sorted by parameter — the same suppression
    /// reads the place an omitted by-name argument's default names.
    pub param_defaults: Vec<(String, String)>,
    /// Whether a call completes normally whatever its arguments hold — the
    /// O109/O126/O108 raise proof takes a pure callee's unused store as dead
    /// where it does.
    pub completes: bool,
    /// The callees surely defined before the load may first run this
    /// procedure, which the same proof takes as callable from its body. The
    /// positions they are proved from stay out of the key, so an edit that
    /// only moves text leaves it alone.
    pub defined_callees: Vec<tcl_compiler::ir::ProcedureId>,
}

#[cfg(test)]
pub(super) fn opt_callee_from_summary(s: &ProcSummary) -> OptCalleeSummary {
    use tcl_compiler::interprocedural::ConstantReturn;
    let mut param_traits: Vec<(String, Vec<tcl_compiler::interprocedural::ProcArgTrait>)> = s
        .param_traits
        .iter()
        .map(|(p, traits)| {
            let mut ts: Vec<_> = traits.iter().copied().collect();
            ts.sort_unstable();
            (p.clone(), ts)
        })
        .collect();
    param_traits.sort_by(|a, b| a.0.cmp(&b.0));
    let mut param_defaults: Vec<(String, String)> = s
        .param_defaults
        .iter()
        .map(|(param, default)| (param.clone(), default.clone()))
        .collect();
    param_defaults.sort();
    OptCalleeSummary {
        qname: s.qualified_name.clone(),
        params: s.params.clone(),
        can_fold_static_calls: s.can_fold_static_calls,
        returns_constant: s.returns_constant,
        constant_return: s.constant_return.as_ref().map(|cr| match cr {
            ConstantReturn::Int(i) => ConstReturnKey::Int(*i),
            ConstantReturn::Float(f) => ConstReturnKey::FloatBits(f.to_bits()),
            ConstantReturn::Bool(b) => ConstReturnKey::Bool(*b),
            ConstantReturn::Str(t) => ConstReturnKey::Str(t.clone()),
        }),
        pure: s.pure,
        writes_global: s.writes_global,
        has_barrier: s.has_barrier,
        has_unknown_calls: s.has_unknown_calls,
        return_passthrough_param: s.return_passthrough_param.clone(),
        return_depends_on_params: s.return_depends_on_params.clone(),
        param_traits,
        param_defaults,
        completes: s.completes,
        defined_callees: s.defined_callees.clone(),
    }
}

#[cfg(test)]
pub(super) fn opt_callee_to_summary(o: &OptCalleeSummary) -> ProcSummary {
    use tcl_compiler::interprocedural::ConstantReturn;
    let mut s = ProcSummary::unknown(&o.qname);
    s.params.clone_from(&o.params);
    s.can_fold_static_calls = o.can_fold_static_calls;
    s.returns_constant = o.returns_constant;
    s.constant_return = o.constant_return.as_ref().map(|cr| match cr {
        ConstReturnKey::Int(i) => ConstantReturn::Int(*i),
        ConstReturnKey::FloatBits(b) => ConstantReturn::Float(f64::from_bits(*b)),
        ConstReturnKey::Bool(b) => ConstantReturn::Bool(*b),
        ConstReturnKey::Str(t) => ConstantReturn::Str(t.clone()),
    });
    s.pure = o.pure;
    s.writes_global = o.writes_global;
    s.has_barrier = o.has_barrier;
    s.has_unknown_calls = o.has_unknown_calls;
    s.return_passthrough_param
        .clone_from(&o.return_passthrough_param);
    s.return_depends_on_params
        .clone_from(&o.return_depends_on_params);
    s.param_traits = o
        .param_traits
        .iter()
        .map(|(p, ts)| (p.clone(), ts.iter().copied().collect()))
        .collect();
    s.param_defaults = o.param_defaults.iter().cloned().collect();
    s.completes = o.completes;
    s.defined_callees.clone_from(&o.defined_callees);
    s
}

/// Original compilation owner for one body's source optimisations.
#[salsa::interned]
pub struct OptDepsKey<'db> {
    #[returns(ref)]
    pub capsule: tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule,
    /// The actual worker argument; None cannot be replaced by a module label.
    #[returns(copy)]
    pub effective_dialect: Option<tcl_dialect::DialectProfileKey>,
    /// Actual prepared worker context; None is a reporting-only purpose.
    #[returns(ref)]
    pub original_context: Option<tcl_compiler::value_transfer::AnalysisContextKey>,
}

#[salsa::tracked(lru = 512, returns(clone))]
pub fn function_optimisations<'db>(
    db: &'db dyn TclDb,
    deps: OptDepsKey<'db>,
) -> Arc<tcl_compiler::optimiser::OriginalBodyOptimiserReport> {
    let capsule = deps.capsule(db);
    let registry = capsule.registry_snapshot().shared_registry();
    let dialect = deps.effective_dialect(db).map(|profile| profile.profile());
    Arc::new(capsule.original_body_optimiser_report(&registry, dialect))
}

/// Whether `module` carries any whole-module trace fact (execution *or*
/// variable — `Module::traced_commands` / `has_dynamic_trace` /
/// `traced_variables` / `has_dynamic_variable_trace`) or any callback script
/// that writes a variable (`Module::deferred_writes`).
///
/// Whole-unit trace and deferred-write obligations require a whole-unit solve.
pub(super) fn module_has_trace_facts(module: &tcl_compiler::ir::Module) -> bool {
    !module.traced_commands.is_empty()
        || module.has_dynamic_trace
        || !module.traced_variables.is_empty()
        || module.has_dynamic_variable_trace
        || !module.deferred_writes.is_clear()
}

/// Assemble a document's optimisations from checked per-body memo owners.
/// Each selected body retains its current lattice key and complete capsule;
/// raw optimisations are computed on the authentic full unit, filtered to that
/// body's groups and rebased before whole-module finalisation. Source mutation
/// and trace hazards remain dependencies of an independently certified readonly
/// source analysis. Other world hazards, missing keys, methods, cross-function
/// lattice reads and argument-sensitive folds require the whole-unit solve.
pub(super) fn solve_optimisations<'db>(
    db: &'db dyn TclDb,
    cu: &CompilationUnit,
    lattice_keys: &HashMap<tcl_compiler::ir::ProcedureId, FnLatticeKey<'db>>,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<Optimisation> {
    let mutations = cu.command_mutations.clone();
    let source_bodies: Vec<_> = cu
        .all_body_function_units()
        .filter_map(|function| {
            let id = function.body_id?;
            let current = cu.function_for_body(Some(id))?;
            (std::ptr::eq(current, function) && cu.original_procedure_for_body(id).is_some())
                .then_some(id)
        })
        .collect();
    let every_proc_keyed = source_bodies.iter().all(|id| lattice_keys.contains_key(id))
        && cu
            .procedures
            .keys()
            .chain(cu.body_lowering_functions().map(|(id, _)| id))
            .all(|id| {
                cu.original_procedure_for_body(*id).is_some()
                    && cu.function_for_body(Some(*id)).is_some()
            });
    let is_irules = dialect.is_some_and(tcl_dialect::DialectProfile::is_irules);
    let dialect_opt = dialect;
    // The **argument-sensitive** O103 fold re-runs a *pure* callee's body with the
    // call's constant arguments (`evaluate_proc_with_constants`), so it reads the
    // callee's whole `FunctionUnit` (`cu.procedures.get(callee)`), not just its
    // summary — a genuine cross-function *body* dependency the single-proc unit
    // cannot serve. Fall back to the whole-module optimise when any proc could be
    // such a fold target: `pure` but not an argument-independent constant return
    // (the latter is summary-level and the memo handles it).
    let has_arg_sensitive_target = cu.interproc.as_ref().is_some_and(|ia| {
        ia.procedures
            .values()
            .any(|s| s.pure && !(s.can_fold_static_calls && s.constant_return.is_some()))
    });
    // A memoised lattice that read another procedure is not the unit's: the
    // unit built that procedure afresh (`SccpResult::reads_module`).
    let reads_module = lattice_keys
        .values()
        .any(|&key| function_lattice(db, key).sccp.reads_module);
    if is_irules
        || !cu.methods.is_empty()
        || has_arg_sensitive_target
        || !every_proc_keyed
        || reads_module
    {
        return tcl_compiler::optimiser::optimise_unit(cu, registry, dialect_opt);
    }

    let retained = Arc::new(cu.clone());
    let capsules: Option<HashMap<_, _>> = source_bodies
        .iter()
        .map(|id| {
            tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::retain(
                Arc::clone(&retained),
                *id,
                registry,
            )
            .map(|capsule| (*id, capsule))
        })
        .collect();
    let Some(capsules) = capsules else {
        return tcl_compiler::optimiser::optimise_unit(cu, registry, dialect_opt);
    };
    // Readonly source certificates retain the actual mutation/trace facts in
    // their dependency value. Reusing that same conservative analysis does not
    // turn unknown source effects into a quiet runtime or entered execution.
    let world_hazards = mutations
        != tcl_compiler::command_binding::ModuleCommandMutations::default()
        || module_has_trace_facts(&cu.ir_module);
    if world_hazards
        && (capsules.is_empty()
            || !capsules.values().all(tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::has_closed_readonly_source_correspondence))
    {
        return tcl_compiler::optimiser::optimise_unit(cu, registry, dialect_opt);
    }
    let body_capsules: Vec<_> = source_bodies
        .iter()
        .map(|id| capsules[id].clone())
        .collect();
    let mut assembly = tcl_compiler::optimiser::OriginalOptimiserAssembly::for_body_partition(
        cu,
        registry,
        dialect_opt,
        &body_capsules,
    );
    // Each entire memo report (including complete-empty) is consumed once.
    // Actual capsule correspondence owns span/group rebasing; callers do not
    // reconstruct a batch from mutable members or infer missing-body absence.
    for body_id in &source_bodies {
        if !lattice_keys.contains_key(body_id) || cu.original_procedure_for_body(*body_id).is_none()
        {
            continue;
        }
        let capsule = &capsules[body_id];
        let deps = OptDepsKey::new(
            db,
            capsule.clone(),
            dialect_opt.map(tcl_dialect::DialectProfile::cache_key),
            capsule.original_optimiser_analysis_context(registry, dialect_opt),
        );
        let report = function_optimisations(db, deps);
        assembly.append_body(capsule, report.as_ref());
    }

    // This parent-derived top view retains the actual current root witness.
    // The same single raw worker and unchanged whole-module tail are reused.
    let top_unit = top_level_only_unit(&assembly);
    let top_report =
        tcl_compiler::optimiser::optimise_unit_raw_report(&top_unit, registry, dialect_opt);
    assembly.append_top_level(&top_report);
    assembly.finish_report()
}

/// The source producer derives the existing top-level-only analysis view.
/// Its actual root owns the complete body roster, full prepared analyses and
/// effective dialect; a public clone-and-clear cannot supply that receipt.
fn top_level_only_unit(
    assembly: &tcl_compiler::optimiser::OriginalOptimiserAssembly,
) -> CompilationUnit {
    assembly.top_level_unit()
}
