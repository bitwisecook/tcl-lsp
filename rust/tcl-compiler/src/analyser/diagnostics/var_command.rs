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

//! Variable-as-command and `TclOO` method-dispatch checks (the
//! cross-function post-pass).
//!
//! Resolves `$var`-as-command call sites collected during the walk: a
//! non-literal command word that cannot be proved safe (W307), an
//! unknown method invoked on an object whose class is known (W308), using
//! MRO-aware method resolution over the class hierarchy with the usual
//! suppression paths (inherited `unknown` handler, external superclass,
//! `oo::objdefine` per-instance methods), and a dispatch with no method
//! word at all on a known `TclOO` object (E001 — see
//! [`Analyser::e001_for_bare_object_dispatch`]). Tracks the object types
//! produced by constructors and factory procedures so a later `$obj
//! badMethod` resolves against the right class, and resolves
//! partially-interpolated command heads that fold to a finite
//! known-command set (W123 suppression).

#[cfg(test)]
use std::collections::HashMap;
use std::collections::HashSet;
use tcl_core_types::{BodyFrameId, DiagCode};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;

mod abstract_instantiation;
mod computed_command_names;
mod dispatch_tables;
mod factory_objects;
mod method_dispatch;
mod method_resolution;
mod self_dispatch;

use computed_command_names::{
    aggregate_constsets, build_tainted_by_scope, w307_enclosing_idx, w307_function_ranges,
};
#[cfg(test)]
use computed_command_names::{
    command_namespaced_ensemble, harvest_array_set_constants, is_callback_array_slot,
    parse_namespaced_ensemble,
};
#[cfg(test)]
use factory_objects::{original_source_return_operand, returned_scalar_reference_name};

/// Dispatch reach and source anchors consumed together when W308 is built.
#[derive(Clone, Copy)]
struct W308DiagnosticSite {
    reach: tcl_registry::definer::MethodReach,
    method_span: Option<tcl_lexer::Span>,
    cmd_span: tcl_lexer::Span,
}

impl From<&super::state::VarCommandSite> for W308DiagnosticSite {
    fn from(site: &super::state::VarCommandSite) -> Self {
        Self {
            reach: site.receiver.method_reach(),
            method_span: site.method_span,
            cmd_span: site.cmd_span,
        }
    }
}

/// All the borrowed analysis data the W307 per-site suppression decision
/// reads, bundled so [`Analyser::w307_site_suppressed`] takes one context
/// argument instead of a dozen.
struct W307Ctx<'a> {
    all_constsets: &'a std::collections::HashMap<String, HashSet<String>>,
    func_ranges: &'a [W307FunctionRange<'a>],
    factory_object_ranges: &'a [(BodyFrameId, u32, u32, HashSet<String>)],
    snit_var_ranges: &'a [(u32, u32, &'a Vec<String>)],
    proc_body_ranges: &'a [(u32, u32, BodyFrameId, HashSet<String>)],
    top_scope: BodyFrameId,
    dispatch_counts: &'a FxHashMap<(BodyFrameId, String), usize>,
    tainted_by_scope: &'a FxHashMap<BodyFrameId, HashSet<String>>,
}

/// Exact declaration owner retained with its original range.
struct W307FunctionRange<'a> {
    function: &'a crate::compilation_unit::FunctionUnit,
    start: u32,
    end: u32,
}

impl Analyser {
    /// W307 — non-literal command name (variable / command-sub
    /// used as command head) and W308 (unknown method on object).
    ///
    /// Walks every recorded
    /// site in [`Self::var_command_sites`] / [`Self::cmd_command_sites`] and
    /// emits W307 unless the command head is statically resolvable to a finite
    /// set of known command names, an OBJECT of a known class (→ W308 method
    /// check), or a positive OO-dispatch signal (`$self`, `my`/`self`
    /// self-dispatch, namespaced ensemble, callback-array, dict-with unpack).
    pub(super) fn emit_var_command_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.var_command_sites.is_empty() && self.cmd_command_sites.is_empty() {
            return;
        }
        // Proc-name literals held as dispatch-table values
        // (`array set` pairs, `dict create`/`dict set` values, `set arr(k) v`)
        // become command references when the table is consumed by one of the
        // dispatch sites this pass walks — so find-references reaches the
        // table entry, and a rename rewrites it alongside the proc (keeping
        // the dispatch working).
        self.emit_dispatch_table_command_references(cu);
        // Build the class hierarchy once for W308 method
        // resolution (uses the ``ClassHierarchy``).
        let hierarchy = if self.result.all_classes.is_empty() {
            None
        } else {
            Some(super::class_hierarchy::build_class_hierarchy(
                self.result.all_classes.clone(),
            ))
        };

        // Aggregate constant-string knowledge (var name → flat CONST/CONSTSET
        // value set) across every function in the CompilationUnit.
        let all_constsets = aggregate_constsets(cu, &self.analysis_context());

        // Per-SSA-version refinement: map each
        // function to its source range + FunctionUnit so the W307
        // suppression can read the value at the dispatch's *exact* SSA
        // use-version instead of the merged set.  ``::top`` covers the
        // whole source; each original procedure or method owns its narrower
        // declaration range and its own symbolic value facts.
        let func_ranges = w307_function_ranges(cu);

        // Drain sites so we can borrow self.result mutably below.
        let sites = std::mem::take(&mut self.var_command_sites);
        // Object-factory locals: vars holding a factory result (`set x [Class
        // new]` / `set x [::ns::factory]` / `set x [object_returning_proc]`).
        // A `$x method` dispatch on one suppresses W307 (designed object usage).
        let FactoryObjectAdvice {
            ranges: factory_object_ranges,
            object_returning,
        } = self.compute_factory_object_ranges(cu, registry);
        // Snit / OO instance-variable dispatch: `$mytree get` where `mytree` is
        // a class instance variable and the dispatch sits inside the class body
        // (including non-method helper `proc`s that `upvar` it). An instance var
        // holds a component / sub-object, so dispatching on it is designed usage
        // — suppress W307.  `snit_var_ranges` is built from every
        // `ClassDef`'s body span + declared `variables`.
        let snit_var_ranges: Vec<(u32, u32, &Vec<String>)> = self
            .result
            .all_classes
            .values()
            .filter(|cd| !cd.variables.is_empty())
            .map(|cd| (cd.body_span.start(), cd.body_span.end(), &cd.variables))
            .collect();

        // **Proc-parameter / multi-dispatch object-dispatch suppression.**
        // A dispatch on a proc
        // *parameter* — `proc walk {tree} { $tree visit }` — is object
        // dispatch the user has documented as the proc's API contract, not a
        // static error.  A non-parameter local dispatched ≥2 times in the same
        // scope is likewise evidenced object usage (a single dispatch could be
        // a typo; repeated use is clearly designed).  Build, per enclosing
        // proc body, its parameter set and the per-var dispatch count, plus a
        // taint carve-out: a *tainted* var is never suppressed (dispatching a
        // user-controlled command name is an injection risk regardless of how
        // many times it appears).  `::top` is the sentinel for statements
        // outside any proc body.
        let top_scope = BodyFrameId::Top(cu.ir_module.procedure_ids.module_id());
        let mut proc_body_ranges: Vec<(u32, u32, BodyFrameId, HashSet<String>)> = cu
            .ir_module
            .procedures
            .iter()
            .chain(cu.ir_module.body_lowerings())
            .map(|(id, body)| {
                (
                    body.span.start(),
                    body.span.end(),
                    BodyFrameId::Body(*id),
                    body.params.iter().cloned().collect(),
                )
            })
            .chain(cu.ir_module.methods.iter().filter_map(|(id, method)| {
                let span = method.span?;
                Some((
                    span.start(),
                    span.end(),
                    BodyFrameId::Body(*id),
                    method.params.iter().cloned().collect(),
                ))
            }))
            .collect();
        // Innermost-enclosing wins: scan largest-start-first for a range that
        // contains the offset (procs don't nest, but `namespace eval` bodies
        // can wrap several, so this stays robust).  Returns the index into
        // `proc_body_ranges`, or `None` for the `::top` sentinel scope.
        proc_body_ranges.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        let mut dispatch_counts: FxHashMap<(BodyFrameId, String), usize> = FxHashMap::default();
        for site in &sites {
            let idx = w307_enclosing_idx(&proc_body_ranges, site.cmd_span.start());
            let owner = idx.map_or(top_scope, |i| proc_body_ranges[i].2);
            *dispatch_counts
                .entry((owner, site.var_name.clone()))
                .or_insert(0) += 1;
        }
        // Per-scope tainted var names — any tainted SSA version of a name
        // disqualifies it from dispatcher-suppression.
        let tainted_by_scope = build_tainted_by_scope(cu);

        let w307_ctx = W307Ctx {
            all_constsets: &all_constsets,
            func_ranges: &func_ranges,
            factory_object_ranges: &factory_object_ranges,
            snit_var_ranges: &snit_var_ranges,
            proc_body_ranges: &proc_body_ranges,
            top_scope,
            dispatch_counts: &dispatch_counts,
            tainted_by_scope: &tainted_by_scope,
        };

        // Collected rather than pushed in the loop: `w307_ctx` borrows out of
        // `self.result`, so the per-site decision has to run against `&self`.
        let emitted: Vec<super::types::Diagnostic> = sites
            .iter()
            .filter_map(|site| self.diagnose_var_command_site(site, hierarchy.as_ref(), &w307_ctx))
            .collect();
        self.result.diagnostics.extend(emitted);
        // Restore the sites list — snapshot/restore expects it
        // to round-trip independently of emission.
        self.var_command_sites = sites;

        // ``[cmd] method`` sites — W307/W308 on command-substitution heads.
        self.emit_cmd_command_diagnostics(
            registry,
            hierarchy.as_ref(),
            &object_returning,
            &cu.ir_module,
        );
    }

    /// The diagnostic one recorded dispatch site draws, if any — the body of
    /// [`Self::emit_var_command_diagnostics`]'s per-site loop, split out so
    /// the three mutually exclusive paths a site can take are visible as
    /// three arms rather than buried in a page of setup.
    ///
    /// Exactly one arm claims each site:
    ///
    /// * **self-dispatch** (`my <method>`) — the receiver is the
    ///   enclosing object, so it is neither a variable nor a resolvable
    ///   command name and W307's "non-literal command name" question never
    ///   arises. This arm always consumes the site;
    /// * **W308** — a current instance receipt has a closed method-name
    ///   inventory, so an absent literal method may be diagnosed;
    /// * **W307** — no usable object type, and the head could not be proved
    ///   to reach a finite set of known command names.
    fn diagnose_var_command_site(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        hierarchy: Option<&super::class_hierarchy::ClassHierarchy>,
        w307_ctx: &W307Ctx<'_>,
    ) -> Option<super::types::Diagnostic> {
        if site.receiver == crate::analyser::state::DispatchReceiver::SelfDispatch {
            return self.w308_for_self_dispatch(site, hierarchy);
        }

        if let Some(instance) = self.live_instance_at_dispatch(site) {
            return self.w308_for_object_var(site, &instance, hierarchy);
        }
        if let Some(classes) = self.logical_receiver_classes(site) {
            return self.w308_for_logical_object_var(site, &classes, hierarchy);
        }

        (!self.w307_site_suppressed(site, w307_ctx)).then(|| {
            crate::analyser::types::Diagnostic::new(
                DiagCode::W307,
                site.cmd_span,
                "Non-literal command name — cannot statically analyse".to_string(),
                Severity::Warning,
            )
        })
    }
}

/// The per-proc maps built by the factory-object analysis in
/// [`Analyser::compute_factory_object_ranges`]: factory-local vars, the
/// `{var -> rhs command head}` assignment map, the last returned var, and the
/// set of procedures with a possible object result.
#[derive(Default)]
struct FactoryObjectAdvice {
    ranges: Vec<(BodyFrameId, u32, u32, HashSet<String>)>,
    object_returning: FxHashSet<BodyFrameId>,
}

#[cfg(test)]
mod retained_constant_context_tests;

#[cfg(test)]
mod original_callback_slot_name_tests;

#[cfg(test)]
mod factory_metadata_context_tests;

#[cfg(test)]
mod original_abstract_factory_context_tests;

#[cfg(test)]
mod self_dispatch_observation_tests;

#[cfg(test)]
mod original_ensemble_head_tests;

#[cfg(test)]
mod configured_method_proposal_controls;

#[cfg(test)]
mod selected_method_return_source_tests;

#[cfg(test)]
mod logical_enclosing_member_inventory_tests;

#[cfg(test)]
mod tk_widget_path_tests;
