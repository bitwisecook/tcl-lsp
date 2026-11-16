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

//! Who can call this compilation unit's procedures?
//!
//! [`CompilationUnit`](crate::compilation_unit::CompilationUnit) is
//! single-source-text by construction, but Tcl has no `static`: every `proc`
//! lands in a global command table any file sharing the interpreter can
//! reach.  The interprocedural SCCP seed
//! ([`params_constants_from_native_call_sites`]) binds a parameter to a compile-time
//! literal only when **every** caller passes that literal — so the seed is
//! only as sound as the claim "the call sites I found are all of them".
//!
//! This module owns that claim, in three layers:
//!
//! 1. **In-unit evidence** — [`collect_call_site_constants`] walks the
//!    module's own CFGs (top level, every proc, plus the `TclOO` method and
//!    `apply` / `namespace eval` body units [`build_extra_call_site_scan_contexts`]
//!    supplies) and records each resolvable call's literal arguments.  A call
//!    dispatched through a variable (`set cmd helper; $cmd dev`) is resolved
//!    by *value* — the scope's literal assignments, unioned for a parameter
//!    with the literals its own callers pass — and recorded as an ordinary
//!    call site for each name it may hold, so a dispatch retracts exactly the
//!    seed it can reach and no more.  Because that reads
//!    evidence this scan itself produces, layer 1 is a monotone fixpoint
//!    ([`run_to_fixpoint`]); a module with no dispatch converges in one walk.
//!    A dispatch whose values cannot be enumerated, or a script a command
//!    receives only as a value (`eval $script`, `apply $fn`), names a caller
//!    of *something* — so it withdraws every seed in the unit
//!    ([`CallSiteEvidence::record_unenumerable_caller`], bounded by
//!    [`unenumerable_reach`]).
//! 2. **Cross-unit evidence** — [`scan_source_call_sites`] runs the identical
//!    registry-driven walk over *another* file's source text, resolving each
//!    call against the whole project's proc names, so a host with a workspace
//!    view can hand this unit the call sites it could never see itself
//!    ([`CallSiteEvidence::merge_from`]).  Without it, a plain library file
//!    with no `package provide`, `source`d by a file that calls its procs
//!    with a different literal, folds a genuinely varying parameter.
//! 3. **Registry-declared boundaries** — [`scan_unit_linkage`] asks the
//!    registry ([`CommandRegistry::unit_linkage`]) whether the file itself
//!    admits to being part of a bigger program: `package provide` /
//!    `ifneeded` publish it as a package, `namespace export` / `ensemble`
//!    publish command names, and `source` / `load` / `package require` /
//!    `auto_load` / `auto_import` pull another unit into the same
//!    interpreter.  The first two admit callers *no* enumeration bounds, so
//!    they decline the seed outright; the last defers to layer 2's evidence
//!    when a host supplied any.  No command name appears here — the traits
//!    are registry data ([`tcl_registry::UNIT_LINKAGE_TRAITS`]).
//!
//! The gate [`params_constants_from_native_call_sites`] applies is stated in full on
//! that function.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use tcl_registry::{CommandRegistry, Traits};

use crate::cfg::{CfgModule, Function as CfgFunction};
#[cfg(test)]
use crate::interprocedural::command_prefix_head;
use crate::ir::Module as IrModule;
#[cfg(test)]
use crate::ir::Statement;

mod body_invocations;
mod call_recording;
mod cfg_scan;
mod extra_scan_contexts;
mod param_seeds;
mod scope_var_facts;
mod source_scan;
mod unit_linkage;

#[cfg(test)]
use body_invocations::word_is_whole_substitution;
#[cfg(test)]
use call_recording::record_call_site_evidence;
use call_recording::{record_unenumerable_targets, unresolved_command_handler};
#[cfg(test)]
use cfg_scan::record_original_lifted_call_sites;
use cfg_scan::scan_cfg_callers;
#[cfg(test)]
use extra_scan_contexts::collect_upframes;
pub(crate) use extra_scan_contexts::{
    build_extra_call_site_scan_contexts, needs_extra_call_site_scan_contexts,
};
#[cfg(test)]
pub(crate) use param_seeds::params_constants_from_call_sites;
pub(crate) use param_seeds::{UnitCallerView, params_constants_from_native_call_sites};
use scope_var_facts::collect_module_scope_var_facts;
#[cfg(test)]
use scope_var_facts::collect_scope_var_facts;
#[cfg(test)]
use source_scan::lower_source_callers;
pub use source_scan::{
    ExternalSourceProcedureFacts, scan_source_call_sites,
    scan_source_call_sites_with_original_source, scan_source_call_sites_with_source_entry,
    scan_source_call_sites_with_source_input,
};
pub use unit_linkage::scan_unit_linkage;

/// Per-arg-position call-site literal evidence for one callee.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArgConsts {
    /// At least one call passed a non-literal (`$`/`[`) value here.
    pub unknown: bool,
    /// Distinct literal values seen at this position.  Ordered so the
    /// evidence — and therefore every seed derived from it and every memo key
    /// that interns one — is independent of scan order.
    pub values: BTreeSet<String>,
}

/// Every call site the scans could attribute to one callee.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalleeEvidence {
    /// At least one possible caller has no enumerable native argv layout.
    /// This is distinct from a proved zero-argument call using defaults.
    pub opaque_caller: bool,
    /// Argument counts observed across the recorded call sites.  A call that
    /// supplies fewer arguments than the callee has parameters leaves the
    /// remaining parameters bound to their **defaults**, an unknown value at
    /// those positions — so a parameter is only literal-uniform when every
    /// observed call actually reached it (see [`Self::binds_position`]).
    pub arg_counts: BTreeSet<usize>,
    /// Literal evidence per 0-based argument position.
    pub slots: BTreeMap<usize, ArgConsts>,
    /// Exact source implementations actually reached by these callers.
    /// Contents alone do not correlate a `QName` with an analysed declaration.
    pub source_implementations: Vec<crate::command_binding::CommandAllocation>,
}

impl CalleeEvidence {
    /// Whether every recorded call site supplied an argument at `index`.
    #[must_use]
    pub fn binds_position(&self, index: usize) -> bool {
        self.arg_counts.iter().all(|&n| n > index)
    }

    /// The single literal every recorded call passes at `index`, or `None`
    /// when the position is unknown, absent, omitted by some call, or
    /// disagreed on.
    #[must_use]
    pub fn uniform_literal_at(&self, index: usize) -> Option<&str> {
        let slot = self.slots.get(&index)?;
        if self.opaque_caller
            || slot.unknown
            || slot.values.len() != 1
            || !self.binds_position(index)
        {
            return None;
        }
        slot.values.first().map(String::as_str)
    }

    /// Mark every recorded position — and every position a future merge adds
    /// — as carrying an unknown value.  Used when a caller exists that the
    /// scan cannot attribute argument-by-argument (an alias, an imported
    /// name, a `CommandPrefix` callback), where the honest record is "this
    /// callee has a call site whose arguments I do not know".
    pub fn poison(&mut self) {
        self.opaque_caller = true;
        self.arg_counts.insert(0);
        for slot in self.slots.values_mut() {
            slot.unknown = true;
        }
    }

    /// Fold `other`'s call sites into this one.  Merging only ever widens
    /// (more values, more unknowns, more argument counts), so evidence from a
    /// second file can retract a fold but never manufacture one.
    fn merge_from(&mut self, other: &Self) {
        self.opaque_caller |= other.opaque_caller;
        self.arg_counts.extend(other.arg_counts.iter().copied());
        for allocation in &other.source_implementations {
            if !self.source_implementations.contains(allocation) {
                self.source_implementations.push(allocation.clone());
            }
        }
        for (index, slot) in &other.slots {
            let mine = self.slots.entry(*index).or_default();
            mine.unknown |= slot.unknown;
            mine.values.extend(slot.values.iter().cloned());
        }
    }
}

/// Caller evidence joined through retained body IDs or authentic source allocations.
/// Display rows retain reporting and conservative unknown-call reach only.
#[derive(Debug, Clone, Default)]
pub struct CallSiteEvidence {
    by_callee: BTreeMap<String, CalleeEvidence>,
    by_body: BTreeMap<crate::ir::ProcedureId, CalleeEvidence>,
    body_labels: BTreeMap<crate::ir::ProcedureId, String>,
    external_opaque_declarations: BTreeSet<crate::command_binding::CommandAllocationSite>,
    by_implementation: BTreeMap<crate::command_binding::CommandAllocation, CalleeEvidence>,
    /// Whether this round consulted a parameter's value set, i.e. whether
    /// the evidence depends on the previous round and the fixpoint must
    /// iterate.  Not part of the evidence's meaning, so it is excluded from
    /// equality.
    consulted_value_sets: bool,
}

impl PartialEq for CallSiteEvidence {
    fn eq(&self, other: &Self) -> bool {
        self.by_callee == other.by_callee
            && self.by_body == other.by_body
            && self.by_implementation == other.by_implementation
            && self.external_opaque_declarations == other.external_opaque_declarations
    }
}

impl Eq for CallSiteEvidence {}

impl CallSiteEvidence {
    /// Whether no call site at all was recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_callee.is_empty()
            && self.by_body.is_empty()
            && self.by_implementation.is_empty()
            && self.external_opaque_declarations.is_empty()
    }

    /// The evidence recorded for `qname`, if any.
    #[must_use]
    pub fn get(&self, qname: &str) -> Option<&CalleeEvidence> {
        self.by_callee.get(qname)
    }

    /// Evidence for this exact retained body, including matching external allocations.
    #[must_use]
    pub fn get_for_body(
        &self,
        module: &IrModule,
        id: crate::ir::ProcedureId,
    ) -> Option<CalleeEvidence> {
        let procedure = module
            .procedures
            .get(&id)
            .or_else(|| module.body_lowering(&id))?;
        let mut result = self.by_body.get(&id).cloned();
        if procedure
            .declaration_site
            .as_ref()
            .is_some_and(|site| self.external_opaque_declarations.contains(site))
        {
            result.get_or_insert_with(CalleeEvidence::default).poison();
        }
        for (allocation, evidence) in &self.by_implementation {
            if module
                .source_procedure_for_allocation(allocation)
                .is_some_and(|(actual, _)| actual == id)
            {
                result
                    .get_or_insert_with(CalleeEvidence::default)
                    .merge_from(evidence);
            }
        }
        if self
            .by_callee
            .get(&procedure.qualified_name)
            .is_some_and(|evidence| evidence.opaque_caller)
        {
            result.get_or_insert_with(CalleeEvidence::default).poison();
        }
        result
    }

    /// Local body IDs with recorded evidence; external-only bodies require `get_for_body`.
    pub fn body_callees(&self) -> impl Iterator<Item = crate::ir::ProcedureId> + '_ {
        self.by_body.keys().copied()
    }

    /// Presentation of a recorded body, without selecting a callee.
    #[must_use]
    pub fn display_for_body(&self, id: crate::ir::ProcedureId) -> Option<&str> {
        self.body_labels.get(&id).map(String::as_str)
    }

    fn record_opaque_body(&mut self, id: crate::ir::ProcedureId, label: &str) {
        self.body_labels.insert(id, label.to_owned());
        self.by_body.entry(id).or_default().poison();
    }

    /// Every callee name evidence was recorded for.
    pub fn callees(&self) -> impl Iterator<Item = &str> {
        self.by_callee.keys().map(String::as_str)
    }

    /// Fold `other`'s call sites in.  See [`CalleeEvidence::merge_from`] —
    /// merging is monotone, so it can only ever retract a fold.
    pub fn merge_from(&mut self, other: &Self) {
        self.external_opaque_declarations
            .extend(other.external_opaque_declarations.iter().cloned());
        for (id, evidence) in &other.by_body {
            self.by_body.entry(*id).or_default().merge_from(evidence);
        }
        self.body_labels.extend(
            other
                .body_labels
                .iter()
                .map(|(id, label)| (*id, label.clone())),
        );
        for (allocation, evidence) in &other.by_implementation {
            self.by_implementation
                .entry(allocation.clone())
                .or_default()
                .merge_from(evidence);
        }
        for (qname, evidence) in &other.by_callee {
            self.by_callee
                .entry(qname.clone())
                .or_default()
                .merge_from(evidence);
        }
    }

    /// Record a caller that names no callee this scan can determine — an
    /// unenumerable dispatch word (`set cmd [gets stdin]; $cmd …`) or a
    /// script a command receives only as a value (`eval $script`, `apply
    /// $fn`), either of which may call anything with anything.
    ///
    /// Recorded by poisoning every procedure in `reach` rather than as one
    /// module-wide flag.  The two are equivalent within a single unit, but
    /// only the per-callee form survives [`Self::merge_from`] and
    /// [`Self::slice_for`] correctly: a flag has no callee to narrow by, so
    /// merging a project's evidence would spread one file's `eval $script`
    /// to every other file's seed — the same disproportionate collateral
    /// damage a module-wide dispatch wildcard causes, at project scope.
    ///
    /// `reach` is what bounds it: the value of an unenumerable word is
    /// resolved against the command table *the scanning unit has loaded*, so
    /// a file that pulls in no other unit can only dispatch to procedures it
    /// declares itself.  See [`unenumerable_reach`].
    pub fn record_unenumerable_caller(&mut self, reach: &[String]) {
        for qname in reach {
            self.record_opaque_caller(qname);
        }
    }

    /// The sub-table covering just `callees` — what a host hands one file's
    /// build after merging the whole project's evidence.
    ///
    /// Narrowing to the procedures a file actually declares is what keeps
    /// invalidation precise: a call site edited in one file changes only the
    /// slice of the file that *defines* the callee.  Driven by `callees`
    /// (the file's own declarations) rather than by filtering the merged
    /// table, so the cost is the file's procedure count, not the project's.
    #[must_use]
    pub fn slice_for<'n>(&self, callees: impl Iterator<Item = &'n str>) -> Self {
        let mut out = Self::default();
        let callees: BTreeSet<&str> = callees.collect();
        for (id, label) in &self.body_labels {
            if callees.contains(label.as_str()) {
                out.body_labels.insert(*id, label.clone());
                if let Some(evidence) = self.by_body.get(id) {
                    out.by_body.insert(*id, evidence.clone());
                }
            }
        }
        for (allocation, evidence) in &self.by_implementation {
            if callees.contains(allocation.command.as_str()) {
                out.by_implementation
                    .insert(allocation.clone(), evidence.clone());
            }
        }
        for qname in callees {
            if let Some(evidence) = self.by_callee.get(qname) {
                out.by_callee.insert(qname.to_owned(), evidence.clone());
            }
        }
        out
    }

    /// Project evidence through this Module's exact bodies and original allocations.
    #[must_use]
    pub fn slice_for_module(&self, module: &IrModule) -> Self {
        let mut out = Self::default();
        for (id, procedure) in module.procedures.iter().chain(module.body_lowerings()) {
            if let Some(evidence) = self.by_body.get(id) {
                out.by_body.insert(*id, evidence.clone());
                out.body_labels
                    .insert(*id, procedure.qualified_name.clone());
            }
            if self
                .by_callee
                .get(&procedure.qualified_name)
                .is_some_and(|row| row.opaque_caller)
            {
                let mut opaque = CalleeEvidence::default();
                opaque.poison();
                out.by_callee
                    .insert(procedure.qualified_name.clone(), opaque);
            }
        }
        for (allocation, evidence) in &self.by_implementation {
            if module.source_procedure_for_allocation(allocation).is_some() {
                out.by_implementation
                    .insert(allocation.clone(), evidence.clone());
            }
        }
        for site in &self.external_opaque_declarations {
            if module.procedures.at_declaration(site).next().is_some() {
                out.external_opaque_declarations.insert(site.clone());
            }
        }
        out
    }

    /// Retain original allocation rows for these declarations and conservative obligations.
    #[must_use]
    pub fn slice_for_original_declarations(
        &self,
        declarations: &[crate::command_binding::CommandAllocationSite],
    ) -> Self {
        let mut result = Self::default();
        result.by_implementation.extend(
            self.by_implementation
                .iter()
                .filter(|(allocation, _)| declarations.contains(&allocation.site))
                .map(|(allocation, evidence)| (allocation.clone(), evidence.clone())),
        );
        result.external_opaque_declarations.extend(
            self.external_opaque_declarations
                .iter()
                .filter(|site| declarations.contains(site))
                .cloned(),
        );
        for (label, evidence) in &self.by_callee {
            if evidence.opaque_caller {
                let mut opaque = CalleeEvidence::default();
                opaque.poison();
                result.by_callee.insert(label.clone(), opaque);
            }
        }
        result
    }

    /// Exclude only callers attributed to these original declaration sites.
    /// Equal labels belonging to another source remain represented.
    #[must_use]
    pub fn without_original_declarations(
        &self,
        declarations: &[crate::command_binding::CommandAllocationSite],
    ) -> Self {
        let mut result = self.clone();
        result
            .by_implementation
            .retain(|allocation, _| !declarations.contains(&allocation.site));
        // Body-ID rows have no source-site inverse; retain their obligations.
        result
    }

    /// Record one call: `args` as written, at `arg_count` words.
    fn record_call(&mut self, qname: String, args: &[String]) {
        let evidence = self.by_callee.entry(qname).or_default();
        evidence.arg_counts.insert(args.len());
        for (index, arg) in args.iter().enumerate() {
            let slot = evidence.slots.entry(index).or_default();
            if arg.contains(['$', '[']) {
                slot.unknown = true;
            } else {
                slot.values.insert(arg.clone());
            }
        }
    }

    fn record_source_call(
        &mut self,
        qname: String,
        prefix: &[crate::registry_invocation::EffectiveInvocationWord],
        argument_count: usize,
        values: &[Option<String>],
        allocation: Option<&crate::command_binding::CommandAllocation>,
    ) {
        let evidence = self.by_callee.entry(qname).or_default();
        let Some(allocation) = allocation else {
            evidence.poison();
            return;
        };
        if !evidence.source_implementations.contains(allocation) {
            evidence.source_implementations.push(allocation.clone());
        }
        let mut reached = CalleeEvidence::default();
        reached.source_implementations.push(allocation.clone());
        reached.arg_counts.insert(prefix.len() + argument_count);
        for index in 0..prefix.len() + argument_count {
            let slot = reached.slots.entry(index).or_default();
            let value = if index < prefix.len() {
                match &prefix[index] {
                    crate::registry_invocation::EffectiveInvocationWord::Literal(value) => {
                        Some(value)
                    }
                    _ => None,
                }
            } else {
                values.get(index - prefix.len()).and_then(Option::as_ref)
            };
            if let Some(value) = value {
                slot.values.insert(value.clone());
            } else {
                slot.unknown = true;
            }
        }
        evidence.merge_from(&reached);
        self.by_implementation
            .entry(allocation.clone())
            .or_default()
            .merge_from(&reached);
    }

    /// Record that `qname` has a caller whose arguments are unattributable.
    pub fn record_opaque_caller(&mut self, qname: &str) {
        self.by_callee.entry(qname.to_owned()).or_default().poison();
    }
}

/// Invariant context [`record_call_site_evidence`] threads unchanged through
/// its recursion into nested `ArgRole::Body` arguments — grouped into one
/// struct (rather than passed as separate parameters) purely to keep the
/// recursive function's own argument count down to the things that actually
/// change per call (`caller_qname` stays fixed across one top-level
/// statement's recursion, `command`/`args`/`depth` do not).
/// Complete retained callback source ingress, separate from call arguments.
#[derive(Clone, Copy)]
pub(crate) struct CallSiteSourceContext<'a> {
    pub registry: &'a CommandRegistry,
    pub module: &'a IrModule,
    pub declared: Option<&'a tcl_registry::model::DeclaredSurface>,
    pub input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    pub identities: &'a crate::realm::CommandBindingRealm,
}

struct CallSiteScanCtx<'a, S> {
    /// The qualified names a call may resolve to.  For the in-unit scan this
    /// is the unit's own procedures; for [`scan_source_call_sites`] it is the
    /// whole project's, so a cross-file call resolves to the file that
    /// actually defines the callee.
    known: &'a HashSet<String, S>,
    registry: &'a CommandRegistry,
    module: &'a IrModule,
    source_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    identities: &'a crate::realm::CommandBindingRealm,
    /// The document's own `# tcl-lsp: stub` declarations, when it has any.
    /// Read through [`CallSiteScanCtx::surface`] so a declared argument role
    /// reaches this scan exactly as a catalogue one does.
    declared: Option<&'a tcl_registry::model::DeclaredSurface>,
    /// Per-scope literal-value facts for local variables, from
    /// [`collect_module_scope_var_facts`] — what a dispatch word `$cmd` may
    /// evaluate to.
    var_facts: &'a ModuleVarFacts,
    /// The previous fixpoint round's evidence, consulted (never written)
    /// when resolving a parameter's value set.
    previous: &'a CallSiteEvidence,
    /// The unit's procedures, for the declared parameter list of a body the
    /// scan walks as a caller.
    procedures: &'a crate::ir::ProcedureInventory,
    /// The procedures an unenumerable dispatch in this scan may reach — see
    /// [`unenumerable_reach`] and
    /// [`CallSiteEvidence::record_unenumerable_caller`].
    unenumerable_reach: &'a [String],
    /// The qualified name of the module's own unresolved-command handler,
    /// when it defines one — see [`unresolved_command_handler`].
    unresolved_handler: Option<&'a str>,
}

impl<'a, S> CallSiteScanCtx<'a, S> {
    /// Join this request's Module and CFG owners to the exact original point.
    /// Only independently explicit Standalone owners can return `Ok(None)`;
    /// supplied missing, foreign or stale input never reopens that route.
    fn original_metadata<'b>(
        &self,
        tokens: &'b crate::ir::CommandTokens,
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
    ) -> Result<
        Option<crate::registry_invocation::InvocationMetadataContext<'b>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        self.original_metadata_for_purpose(
            tokens,
            owner,
            crate::command_binding::original_name_value::OriginalWordSourcePurpose::Invocation,
        )
    }

    fn original_argument_metadata<'b>(
        &self,
        tokens: &'b crate::ir::CommandTokens,
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
    ) -> Result<
        Option<crate::registry_invocation::InvocationMetadataContext<'b>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        self.original_metadata_for_purpose(
            tokens,
            owner,
            crate::command_binding::original_name_value::OriginalWordSourcePurpose::ArgumentEvaluation,
        )
    }

    fn original_metadata_for_purpose<'b>(
        &self,
        tokens: &'b crate::ir::CommandTokens,
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
        purpose: crate::command_binding::original_name_value::OriginalWordSourcePurpose,
    ) -> Result<
        Option<crate::registry_invocation::InvocationMetadataContext<'b>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        use crate::command_binding::original_name_value::OriginalWordSourcePurpose;
        use crate::registry_invocation::{InvocationMetadataContext, RegistryInvocationDecline};
        let refusal = RegistryInvocationDecline::IncompleteResolution;
        let binding = tokens.source_binding.as_ref().ok_or(refusal)?;
        if !self
            .module
            .retained_source_bindings
            .as_deref()
            .is_some_and(|retained| retained.matches_module(self.module, self.registry))
        {
            return Err(refusal);
        }
        if let Some(input) = self.source_input {
            let module =
                InvocationMetadataContext::for_module(self.registry, self.module).ok_or(refusal)?;
            if module.source_analysis_input() != Some(input) {
                return Err(refusal);
            }
            // The Module is the original producer. Only its exact input or
            // explicit nested projection can own this caller; projecting an
            // independently changed file input cannot authenticate the owner.
            let caller_input = owner.source_analysis_input().ok_or(refusal)?;
            if caller_input != input && caller_input != &input.for_nested_source() {
                return Err(refusal);
            }
            if purpose == OriginalWordSourcePurpose::ArgumentEvaluation {
                binding
                    .original_argument_metadata_for_owner(tokens, owner, self.registry)?
                    .ok_or(refusal)?;
                binding
                    .original_argument_metadata_for_input(tokens, input, self.registry)
                    .ok_or(refusal)?;
                return binding
                    .original_argument_metadata_for_module(tokens, self.module, self.registry)
                    .map(Some)
                    .ok_or(refusal);
            }
            binding
                .original_invocation_metadata_for_owner(tokens, owner, self.registry)?
                .ok_or(refusal)?;
            binding
                .original_invocation_metadata_for_input(tokens, input, self.registry)
                .ok_or(refusal)?;
            return binding
                .original_invocation_metadata_for_module(tokens, self.module, self.registry)
                .map(Some)
                .ok_or(refusal);
        }
        if !self.module.source_entry.metadata_context.is_standalone() || !owner.is_standalone() {
            return Err(refusal);
        }
        if purpose == OriginalWordSourcePurpose::ArgumentEvaluation {
            binding.original_argument_metadata_for_owner(tokens, owner, self.registry)
        } else {
            binding.original_invocation_metadata_for_owner(tokens, owner, self.registry)
        }
    }

    /// The catalogue extended by the document's declarations — the one door
    /// every argument-role question in this scan goes through.
    fn surface(&self) -> tcl_registry::model::DocumentCommandSurface<'a> {
        tcl_registry::model::DocumentCommandSurface::new(self.registry, self.declared)
    }
}

/// One body the scan walks as a *caller*.
///
/// Actual body IDs and independently retained extra occurrences own local facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CallerScopeId {
    TopLevel,
    Procedure(crate::ir::ProcedureId),
    Extra(usize),
}

struct CallerFrame<'a> {
    metadata_owner: &'a crate::registry_invocation::OwnedInvocationMetadataContext,
    /// Variable-scope identity — the key into `var_facts`, and the callee
    /// key whose recorded arguments a `$param` dispatch word may take.
    scope: CallerScopeId,
    /// The body's declared parameters, in order.
    params: &'a [String],
    /// Whether this evidence map attributes calls *to* this body. True for
    /// the top level and ordinary procedures; false for a `TclOO` method or
    /// an `apply` lambda, whose invocations are not call sites this scan
    /// resolves — so their parameters can hold values it never saw.
    callers_tracked: bool,
}

/// Literal-value facts for one variable in one scope.
#[derive(Default, Clone)]
struct VarLiterals {
    /// Distinct literal values assigned to it.
    values: BTreeSet<String>,
    /// A write whose value this scan cannot pin to a literal.
    unknown: bool,
}

/// Literal-value facts for every local variable of one scope.
#[derive(Default)]
struct ScopeVars {
    vars: HashMap<String, VarLiterals>,
    /// A write through a *dynamic variable name* (`set $n $v`, `upvar 1 x
    /// $alias`) happened somewhere in this scope, so any local may hold a
    /// value this scan never saw.
    dynamic_name_write: bool,
    /// A script this scan cannot read runs in *some* variable frame it does
    /// not own — a frame-shifting `uplevel` body, or a body reached only
    /// through a substitution (`eval $script`). Such a script may assign a
    /// local of any scope, including this one, so it is a module-wide fact
    /// rather than a per-scope one; it is discovered per scope and unioned
    /// by [`collect_module_scope_var_facts`].
    cross_frame_write: bool,
}

/// Per-scope literal-value facts for the whole module, plus the
/// module-wide "a script wrote a frame this scan does not own" fact.
#[derive(Default)]
struct ModuleVarFacts {
    scopes: HashMap<CallerScopeId, ScopeVars>,
    cross_frame_write: bool,
}

/// One extra executable body walked as a call-site source.
pub(crate) struct ExtraCallSiteScanContext {
    /// Method/body-unit identity when this CFG is also consumed by the full
    /// per-unit lattice build. Upframe-only scan contexts leave this absent.
    analysis_body: Option<crate::ir::ProcedureId>,
    /// Variable-scope identity carried by the CFG itself.
    cfg: CfgFunction,
    /// The namespace Tcl selects when this body actually executes.  `TclOO`
    /// instance methods receive their object's namespace only at runtime.
    execution_namespace: crate::ir::ExecutionNamespace,
    /// Whether this body contains a command whose target can therefore differ
    /// from the lexical fallback represented by `cfg`.
    requires_runtime_command_namespace: bool,
}

impl ExtraCallSiteScanContext {
    pub(crate) fn body_analysis_cfg(&self, id: crate::ir::ProcedureId) -> Option<&CfgFunction> {
        (self.analysis_body == Some(id)).then_some(&self.cfg)
    }
}

/// Collect literal arg values per user-proc call site across the whole
/// module's CFGs (top-level + every proc/method/body-unit, statements
/// already flattened), including calls nested inside `ArgRole::Body`
/// arguments (`catch { … }`, a literal `uplevel { … }`, `apply {{…} {…}}`, a
/// non-exact `switch` arm, …) via [`record_call_site_evidence`].
///
/// Original call allocations join the Module's issued body coordinates.
/// Unknown dispatch remains in the caller evidence and withdraws parameter seeds.
pub(crate) fn collect_call_site_constants(
    cfg_module: &CfgModule,
    extra_callers: &[ExtraCallSiteScanContext],
    procedures: &crate::ir::ProcedureInventory,
    future_call_sites: &[crate::command_binding::SourceFutureCallSite],
    source: CallSiteSourceContext<'_>,
) -> CallSiteEvidence {
    let CallSiteSourceContext {
        registry,
        module,
        declared,
        input,
        identities,
    } = source;
    let known: HashSet<String> = procedures
        .values()
        .map(|procedure| procedure.qualified_name.clone())
        .collect();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, declared);
    let var_facts =
        collect_module_scope_var_facts(cfg_module, extra_callers, &surface, module, identities);
    // Within one unit the reach is simply its own procedures, so an
    // unenumerable dispatch withdraws every seed the unit would have taken —
    // identical to the module-wide rule it replaces, but expressed per callee
    // so it merges and slices correctly across files.
    let reach = unenumerable_reach(procedures, Traits::empty(), &known);
    let unresolved_handler = unresolved_command_handler(registry, &known);
    let round = |previous: &CallSiteEvidence| {
        let ctx = CallSiteScanCtx {
            known: &known,
            registry,
            module,
            source_input: input,
            identities,
            declared,
            var_facts: &var_facts,
            previous,
            procedures,
            unenumerable_reach: &reach,
            unresolved_handler,
        };
        let funcs = call_site_scan_functions(cfg_module, extra_callers);
        let mut out = CallSiteEvidence::default();
        record_future_callers(&mut out, &ctx, future_call_sites);
        record_runtime_selected_extra_callers(&mut out, &ctx, extra_callers);
        scan_cfg_callers(&mut out, &ctx, funcs);
        out
    };
    run_to_fixpoint(round)
}

/// Future entries can retract every affected seed, but never supply a seed.
/// Exact command alternatives remain bounded; unknown runtime lookup retains
/// the same explicit reach bound as an unreadable actual caller.
fn record_future_callers(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    sites: &[crate::command_binding::SourceFutureCallSite],
) {
    for site in sites {
        if site.binding.unknown {
            record_unenumerable_targets(out, ctx);
        }
        // A future caller withdraws assumptions about actual runtime targets.
        // Independent compiler uncertainty does not add a runtime callee.
        for target in &site.binding.targets {
            if let Some((id, procedure)) = ctx.module.source_procedure_for_target(target) {
                out.record_opaque_body(id, &procedure.qualified_name);
            }
            if target.kind == crate::command_binding::BindingKind::Proc
                && ctx.known.contains(&target.command)
            {
                out.record_opaque_caller(&target.command);
            } else if !target.registry_backed {
                record_unenumerable_targets(out, ctx);
            }
        }
    }
}

/// Iterate `round` until the evidence stops changing.
///
/// A dispatch word that names one of its own body's parameters reads the
/// literals *callers* pass there — evidence this very scan produces — so the
/// two are mutually recursive.  Resolving that against the SCCP result would
/// be circular (the seed this evidence feeds is an SCCP input), so the scan
/// closes over itself instead: round 0 runs with "no callers seen yet" and
/// each round re-derives from the last until stable.
///
/// Monotone and therefore terminating: a round only ever adds call sites,
/// literals, or unknowns, all drawn from the module's finite set of literal
/// words.  A module with no such dispatch consults no value set at all, so
/// the first round reports it and the loop exits after exactly one walk —
/// the overwhelmingly common case pays nothing.
fn run_to_fixpoint(round: impl Fn(&CallSiteEvidence) -> CallSiteEvidence) -> CallSiteEvidence {
    let mut evidence = CallSiteEvidence::default();
    loop {
        let mut next = round(&evidence);
        next.merge_from(&evidence);
        if !next.consulted_value_sets || next == evidence {
            return next;
        }
        evidence = next;
    }
}

/// The procedures an unenumerable dispatch in one unit may reach.
///
/// The value of `$cmd` in `set cmd [gets stdin]; $cmd dev` is resolved
/// against the command table the *scanning unit* has loaded — not against
/// every procedure that happens to exist in the host's workspace.  A file
/// that pulls in no other unit (`declared_linkage` carries no
/// [`Traits::LOADS_EXTERNAL_UNIT`]) therefore cannot dispatch outside its own
/// declarations, however many files the project holds; one that does `source`
/// / `package require` can reach whatever that brought in, which the host's
/// project-wide `known` set over-approximates.
///
/// This is what keeps an unreadable dispatch's blast radius proportionate:
/// without it, one `eval $script` anywhere in a workspace would withdraw
/// every interprocedural seed in every file.
fn unenumerable_reach(
    declared: &crate::ir::ProcedureInventory,
    declared_linkage: Traits,
    project_wide: &HashSet<String, impl std::hash::BuildHasher>,
) -> Vec<String> {
    if declared_linkage.intersects(Traits::LOADS_EXTERNAL_UNIT) {
        let mut reach: Vec<String> = project_wide.iter().cloned().collect();
        reach.sort();
        return reach;
    }
    let mut reach: Vec<String> = declared
        .values()
        .map(|procedure| procedure.qualified_name.clone())
        .collect();
    reach.sort();
    reach
}

fn call_site_scan_functions<'a>(
    cfg: &'a CfgModule,
    extra: &'a [ExtraCallSiteScanContext],
) -> impl Iterator<Item = (CallerScopeId, &'a CfgFunction)> {
    std::iter::once((CallerScopeId::TopLevel, &cfg.top_level))
        .chain(
            cfg.procedures
                .iter()
                .map(|(id, body)| (CallerScopeId::Procedure(*id), body)),
        )
        .chain(extra.iter().enumerate().map(|(index, caller)| {
            let scope = caller
                .analysis_body
                .map_or(CallerScopeId::Extra(index), CallerScopeId::Procedure);
            (scope, &caller.cfg)
        }))
}

/// Withdraw every parameter seed reachable from an extra body whose command
/// namespace is selected only when that body runs.
///
/// A `TclOO` method's lexical CFG retains useful fallback calls, but an
/// object-local command may shadow any relative head and itself invoke any
/// procedure in `unenumerable_reach` with arbitrary arguments.  That is the
/// same incomplete-caller condition as an unreadable `$cmd` dispatch, so it
/// must enter the same evidence lattice.  Keeping this beside
/// [`scan_cfg_callers`] makes the in-unit and cross-file scans share the rule.
fn record_runtime_selected_extra_callers(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    extra_callers: &[ExtraCallSiteScanContext],
) {
    if extra_callers.iter().any(|caller| {
        matches!(
            &caller.execution_namespace,
            crate::ir::ExecutionNamespace::RuntimeSelected
        ) && caller.requires_runtime_command_namespace
    }) {
        record_unenumerable_targets(out, ctx);
    }
}

#[derive(Clone, Copy)]
struct SourceCallSiteEntry<'a> {
    entry: Option<&'a crate::command_binding::SourceAnalysisEntry>,
    input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
}

#[cfg(test)]
mod tests;
