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

use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::cfg::{CfgModule, Function as CfgFunction};
#[cfg(test)]
use crate::interprocedural::command_prefix_head;
use crate::ir::{Module as IrModule, Statement};
use crate::naming::is_dynamic_word;
use crate::value_shapes::whole_word_scalar_var_name;
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution_with_config};

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

/// The set of literal strings a word may evaluate to.
enum WordValues {
    /// Fully enumerated — the word can only ever be one of these.
    Literals(BTreeSet<String>),
    /// Not enumerable by this scan.
    Unknown,
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

impl ScopeVars {
    /// Record an assignment to the variable `name` names — `Some(value)`
    /// when the assigned value is a compile-time literal, `None` when it
    /// is not.  Poisons the whole scope when the *name* is computed.
    fn note_assignment(&mut self, name: &str, value: Option<&str>) {
        if is_dynamic_word(name) {
            self.dynamic_name_write = true;
            return;
        }
        let facts = self.vars.entry(name.to_owned()).or_default();
        match value {
            Some(literal) => {
                facts.values.insert(literal.to_owned());
            }
            None => facts.unknown = true,
        }
    }

    fn note_unknown(&mut self, name: &str) {
        self.vars.entry(name.to_owned()).or_default().unknown = true;
    }

    /// Record a write to the variable a *word* names — poisoning the whole
    /// scope when the name itself is computed.
    fn note_write_word(&mut self, word: &str) {
        if is_dynamic_word(word) {
            self.dynamic_name_write = true;
        } else {
            self.note_unknown(word);
        }
    }
}

/// Collect literal-value facts for one scope's local variables.
///
/// Constant assignments (`set cmd helper`, or the plain-bareword
/// `AssignValue` shape lowering leaves alone) contribute a literal; every
/// other write contributes "unknown" for that name.  Which *words* of a
/// generic call name a variable is command-surface data
/// ([`ArgRole::VarWrite`], plus [`Traits::CREATES_SCOPE_ALIAS`] for the
/// vararg alias forms `global x y z` / `variable a b` / `upvar 1 a b 1 c d`,
/// whose per-argument name list the role query deliberately does not expand) — no command name is
/// matched here.
///
/// Structured bodies (`if`, `while`, `catch`, a *literal* `eval`/`uplevel
/// #0` body) are already flattened into the caller's own CFG blocks by
/// lowering, so their writes are ordinary block statements here.  The two
/// that are not — a frame-shifting [`Statement::UpFrame`] body and a body
/// reached only through a substitution on a [`Traits::EVALUATES_CODE`]
/// command — set [`ScopeVars::cross_frame_write`] instead.
#[cfg(test)]
fn collect_scope_var_facts(
    cfg: &CfgFunction,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    context: Option<&tcl_registry::model::ContextRegistry>,
) -> ScopeVars {
    let mut out = ScopeVars::default();
    if context.is_none_or(|context| {
        !crate::registry_invocation::InvocationMetadataContext::from(context)
            .matches_registry(surface.commands())
    }) {
        out.dynamic_name_write = true;
        out.cross_frame_write = true;
        return out;
    }
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { name, value, .. } => {
                    out.note_assignment(name, Some(value));
                }
                // Lowering only produces `AssignConst` for a *recognised*
                // constant shape; a plain bareword value (`set cmd helper`)
                // arrives as `AssignValue` and is just as literal — provided
                // it neither substitutes nor needs backslash resolution.
                Statement::AssignValue {
                    name,
                    value,
                    value_needs_backsubst,
                    ..
                } => {
                    let literal =
                        (!*value_needs_backsubst && !is_dynamic_word(value)).then_some(value);
                    out.note_assignment(name, literal.map(String::as_str));
                }
                Statement::AssignExpr { name, .. } | Statement::Incr { name, .. } => {
                    out.note_write_word(name);
                }
                Statement::Call { defs, .. } => {
                    for d in defs {
                        out.note_write_word(d);
                    }
                    note_surface_var_writes(&mut out, surface, stmt, context);
                }
                Statement::Barrier { .. } => {
                    note_surface_var_writes(&mut out, surface, stmt, context);
                }
                // `uplevel N {…}`: the body's writes land in another
                // frame, which this per-scope walk does not own.
                Statement::UpFrame { .. } => out.cross_frame_write = true,
                _ => {}
            }
        }
    }
    out
}

/// Record the variable writes the command surface declares for one call.
#[cfg(test)]
fn note_surface_var_writes(
    out: &mut ScopeVars,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    stmt: &Statement,
    context: Option<&tcl_registry::model::ContextRegistry>,
) {
    let registry = surface.commands();
    let Some(invocation) = stmt.tokens().and_then(|tokens| {
        crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
            registry,
            context.map(Into::into),
            tokens,
        )
    }) else {
        if stmt.tokens().is_some_and(|tokens| {
            tokens.has_unproved_source_binding()
                || crate::registry_invocation::registry_invocation_assistance_with_metadata_context(
                    registry,
                    context.map(Into::into),
                    tokens,
                )
                .is_none_or(|assistance| {
                    assistance.candidates.is_empty()
                        || assistance.unknown_residual
                        || assistance.may_be_absent
                })
        }) {
            out.dynamic_name_write = true;
            out.cross_frame_write = true;
        }
        return;
    };
    for (index, role) in invocation.variable_roles() {
        if role == ArgRole::VarWrite {
            if let Some(name) = invocation.argument_literal(index) {
                out.note_unknown(&name);
            } else {
                out.dynamic_name_write = true;
            }
        }
    }
    for alias in invocation.variable_alias_transitions() {
        if let Some(local) = alias.local.literal() {
            out.note_unknown(local);
        } else {
            out.dynamic_name_write = true;
        }
    }
}

/// The literal values `word` may take when evaluated in `caller`'s scope.
///
/// A word with no substitution is its own only value.  A whole-word scalar
/// reference (`$cmd` / `${cmd}`) resolves against the scope's literal-value
/// facts, unioned — when the name is one of the body's own parameters —
/// with the literals its callers pass at that position
/// ([`parameter_values`]).  Anything else is [`WordValues::Unknown`].
fn word_values(
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    word: &str,
) -> WordValues {
    if !is_dynamic_word(word) {
        return WordValues::Literals(std::iter::once(word.to_owned()).collect());
    }
    let Some(var) = whole_word_scalar_var_name(word, ctx.module.lexer_config) else {
        return WordValues::Unknown;
    };
    // A namespace-qualified reference names a variable any body in the
    // module (or another file) may write; this scan models local frames
    // only.
    if crate::naming::is_qualified(var.as_bytes()) {
        return WordValues::Unknown;
    }
    // A script running in a frame this scan cannot read may have assigned
    // any local of any scope.
    if ctx.var_facts.cross_frame_write {
        return WordValues::Unknown;
    }
    let Some(scope_vars) = ctx.var_facts.scopes.get(&caller.scope) else {
        return WordValues::Unknown;
    };
    if scope_vars.dynamic_name_write {
        return WordValues::Unknown;
    }
    let mut values = BTreeSet::new();
    if let Some(facts) = scope_vars.vars.get(var) {
        if facts.unknown {
            return WordValues::Unknown;
        }
        values.extend(facts.values.iter().cloned());
    }
    match parameter_values(ctx, caller, var) {
        Some(WordValues::Unknown) => return WordValues::Unknown,
        Some(WordValues::Literals(from_callers)) => values.extend(from_callers),
        None => {}
    }
    WordValues::Literals(values)
}

/// The literals `var`'s callers pass when `var` is one of `caller`'s
/// declared parameters, or `None` when it is not a parameter at all.
///
/// A parameter with no recorded caller contributes nothing: within a
/// compilation unit already gated on having no `package provide`, an
/// uncalled procedure's body never runs. A parameter of a body whose
/// callers this scan does not attribute at all (a method, an `apply`
/// lambda) can hold anything.
fn parameter_values(
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    var: &str,
) -> Option<WordValues> {
    let index = caller.params.iter().position(|p| p == var)?;
    // `args` slurps every remaining argument into a *list*, so its value is
    // not any one caller's word.
    if var == "args" || !caller.callers_tracked {
        return Some(WordValues::Unknown);
    }
    let CallerScopeId::Procedure(id) = caller.scope else {
        return Some(WordValues::Unknown);
    };
    let previous = ctx.previous.get_for_body(ctx.module, id);
    match previous
        .as_ref()
        .and_then(|evidence| evidence.slots.get(&index))
    {
        Some(slot) if slot.unknown => Some(WordValues::Unknown),
        Some(slot) => Some(WordValues::Literals(slot.values.clone())),
        None => Some(WordValues::Literals(BTreeSet::new())),
    }
}

/// True when the whole word is a single substitution — `$v`, `${v}`, or
/// `[cmd …]` — so its *value*, not its text, is the script / lambda /
/// command prefix the command receives.
///
/// The distinction matters for every script-bearing argument: `catch {puts
/// $x}` carries the literal script text `puts $x` (which merely *contains*
/// a substitution), whereas `eval $script` carries no script text at all —
/// only a reference to one.  Testing `is_dynamic_word` alone conflates the
/// two and would treat every ordinary braced body that mentions a variable
/// as unresolvable.
fn word_is_whole_substitution(word: &str, config: tcl_lexer::LexerConfig) -> bool {
    is_pure_var_ref(word, config) || parse_command_substitution_with_config(word, config).is_some()
}

/// Record one call site's literal-argument evidence into `out`, then recurse
/// into any `ArgRole::Body` argument of `command` (regardless of whether
/// `command` itself is a user proc) — a nested script embedded in a nested
/// script embedded in a nested script, and so on, up to
/// the shared source-walk depth budget.
///
/// `catch { isEven 4 }`, a non-exact `switch` arm, a literal `uplevel
/// {…}` / `apply {{…} {…}}` body, and friends all carry their nested script
/// as one opaque *argument string* to a builtin (`catch`, `switch`,
/// `uplevel`, `apply`) that is never itself a user proc — so a flat,
/// one-level `Statement::Call`/`Statement::Barrier` walk resolves `catch`
/// (finds no matching proc, moves on) and never notices `isEven 4` sitting
/// inside its body argument at all. That's a *second* proc call this scan
/// cannot see, exactly like the namespace-resolution gap: an invisible call
/// site with a differing argument silently vanishes from
/// [`params_constants_from_native_call_sites`]'s "every caller agrees" evidence.
///
/// The command surface already knows which argument position of which
/// command is a script body (`ArgRole::Body`, driving the identical recursive
/// call-graph walk in [`crate::interprocedural`] and the `BODY`-role scans in
/// `ir_helpers.rs` / `place_bridge.rs` / `ssa.rs`) — so this reuses that one
/// fact via
/// [`tcl_registry::model::DocumentCommandSurface::arg_indices_for_role`] and
/// the shared [`crate::segmenter`] rather than hand-rolling a second "which
/// commands embed scripts" list here.
/// Whether the unseen code an `<unseen-call>` marker stands for is already
/// accounted for by its host call (the nearest preceding non-marker
/// statement): a call to one retained in-module procedure, whose body is
/// scanned as its own caller, or a computed head, whose dispatch the host's
/// own scan enumerates or withdraws.
fn unseen_code_is_scanned_host(
    func: &crate::cfg::Function,
    block_id: crate::cfg::BlockId,
    statements: &[Statement],
    marker: usize,
) -> bool {
    let Some(host) = (0..marker).rev().find(|&index| {
        statements[index]
            .tokens()
            .is_none_or(|tokens| tokens.synthetic.is_none())
    }) else {
        return false;
    };
    if matches!(&statements[host], Statement::Call { command, .. } if is_dynamic_word(command)) {
        return true;
    }
    func.source_tokens_at(block_id, host)
        .and_then(|tokens| tokens.source_binding.as_ref())
        .and_then(crate::command_binding::SourceInvocationBinding::proved_target)
        .is_some_and(|target| {
            target.kind == crate::command_binding::BindingKind::Proc
                && !target.registry_backed
                && target.implementation_allocation.is_some()
        })
}

fn record_call_site_evidence(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    command: &str,
    args: &[String],
    tokens: Option<&crate::ir::CommandTokens>,
    depth: u32,
) {
    if tokens.is_some_and(|tokens| tokens.synthetic.is_some()) {
        return;
    }
    let Some(tokens) = tokens else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    let Ok(context) = ctx.original_metadata(tokens, caller.metadata_owner) else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    if tokens.source_binding.as_ref().is_some_and(|binding| {
        binding.runtime_reachability()
            == crate::command_binding::SourceRuntimeReachability::NotEntered
    }) {
        // Complete source execution coverage proves this dispatch was not
        // entered. Its independent compiler visitation still matters to
        // admission, but cannot introduce an arbitrary runtime caller.
        return;
    }
    record_original_callback_callers(out, ctx, Some(tokens), context);
    if let Some(invocation) =
        crate::registry_invocation::normal_user_procedure_invocation_with_metadata_context(
            ctx.registry,
            context,
            tokens,
        )
    {
        record_normal_user_procedure_call(out, ctx, tokens, &invocation);
        return;
    }
    // A dispatched command word (`$cmd args`) is resolved by *value*, not
    // skipped: the scope's own literal assignments (unioned, for a parameter,
    // with the literals its callers pass) give the set of names it may hold,
    // and each becomes an ordinary call site.  Skipping it would let a
    // dispatch reach a proc this scan had already seeded from its literal call
    // sites, silently unsoundly.  A word whose value set is not enumerable
    // withdraws every seed instead.
    record_invocation(
        out,
        ctx,
        caller,
        command,
        IndirectArgs::Words(args),
        Some(tokens),
    );
    if is_dynamic_word(command) {
        // Which of a computed head's arguments carry scripts, callbacks, or a
        // callee name is unknowable; the dispatch itself is already accounted
        // for above.
        return;
    }
    let invocation = context
        .filter(|metadata| {
            tokens.source_binding.as_ref().and_then(|binding| {
                binding.original_invocation_source_purpose(tokens, *metadata, ctx.registry)
            }) == Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical)
        })
        .and_then(|metadata| {
            crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                ctx.registry,
                metadata,
                tokens,
            )
        })
        .or_else(|| {
            crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                ctx.registry,
                context,
                tokens,
            )
        });
    let Some(invocation) = invocation else {
        if context
            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input)
            .is_some_and(|input| {
                tokens.source_binding.as_ref().is_some_and(|binding| {
                    binding
                        .original_source_procedure_argument_roles(tokens, input, ctx.registry)
                        .is_some()
                })
            })
        {
            return;
        }
        record_unenumerable_targets(out, ctx);
        return;
    };
    record_indirect_callers(out, ctx, caller, &invocation, Some(tokens));
    record_invocation_body_evidence(out, ctx, caller, &invocation, tokens, context, depth);
}

fn record_normal_user_procedure_call(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    tokens: &crate::ir::CommandTokens,
    invocation: &crate::registry_invocation::NormalUserProcedureInvocation,
) {
    let Some(source) = tokens.source_binding.as_ref() else {
        return;
    };
    let callee = source.lookup_command_word(&invocation.target);
    if invocation.unknown_runtime || callee.unknown {
        record_unenumerable_targets(out, ctx);
    }
    for target in &callee.targets {
        if target.kind == crate::command_binding::BindingKind::Proc
            && (ctx.module.source_procedure_for_target(target).is_some()
                || ctx.known.contains(&target.command))
        {
            out.record_source_call(
                target.command.clone(),
                &target.prepended,
                invocation.arguments.len(),
                &invocation.arguments,
                target.implementation_allocation.as_ref(),
            );
        } else if !target.registry_backed {
            record_unenumerable_targets(out, ctx);
        }
    }
}

fn record_invocation_body_evidence(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    tokens: &crate::ir::CommandTokens,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    depth: u32,
) {
    let Some(metadata) = metadata else {
        // This compatibility utility is available only after original_metadata
        // has independently checked both explicit Standalone owners.
        record_standalone_invocation_body_evidence(
            out,
            ctx,
            caller,
            invocation,
            Some(tokens),
            depth,
        );
        return;
    };
    let Some(input) = metadata.source_analysis_input() else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    let Some(words) =
        crate::registry_invocation::source_structure::original_registry_words_for_supplied_tokens(
            ctx.identities,
            input,
            tokens,
        )
    else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    let Some(roles) = words.roles() else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    let has_bodies = roles.iter().any(|&(_, role)| role == ArgRole::Body);
    for &(argument, role) in roles {
        if role == ArgRole::LambdaLiteral
            && words
                .arguments()
                .get(argument)
                .and_then(|word| word.literal_bytes())
                .is_none()
        {
            record_unenumerable_targets(out, ctx);
        }
    }
    if !has_bodies {
        return;
    }
    if crate::depth_guard::MAX_SOURCE_NEST_DEPTH.exceeded(depth + 1) {
        record_unenumerable_targets(out, ctx);
        return;
    }
    // Definitions and frame-shifting bodies have independent caller units.
    // The selected authored trait description supplies no entered frame.
    let context = input.context_registry();
    let Some(traits) = words.with_source_schema(&context, |schema| schema.semantics.traits) else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    if traits.intersects(Traits::DEFINES_PROCEDURE | Traits::EVALUATES_IN_SHIFTED_FRAME) {
        return;
    }
    let Some(bodies) = words.source_script_bodies_for_mutation_coverage(&context) else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    for body in bodies {
        record_nested_body_invocation_evidence(out, ctx, caller, tokens, &body, &context, depth);
    }
}

fn record_nested_body_invocation_evidence(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    tokens: &crate::ir::CommandTokens,
    body: &crate::registry_invocation::OriginalSourceScriptBody,
    context: &tcl_registry::model::ContextRegistry,
    depth: u32,
) {
    let Some(commands) = body.original_commands(context) else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    for command in commands {
        let Some(binding) = unique_nested_binding(tokens, command.span.start()) else {
            record_unenumerable_targets(out, ctx);
            continue;
        };
        let Some(site) = binding.invocation_site() else {
            record_unenumerable_targets(out, ctx);
            continue;
        };
        let Some((segment, mut original)) = binding.original_recorded_command() else {
            record_unenumerable_targets(out, ctx);
            continue;
        };
        let config = binding.original_lexer_config_for_tokens(&original);
        if site.offset != command.span.start()
            || site.source.source_image() != body.original_container().image()
            || config != Some(body.original_container().config())
            || crate::registry_invocation::original_native_compiler_words(
                site.source.source_image(),
                original.words(),
                site.offset,
                body.original_container().config(),
            )
            .as_deref()
                != Some(command.words.as_slice())
        {
            record_unenumerable_targets(out, ctx);
            continue;
        }
        original.inherit_nested_bindings(tokens);
        record_call_site_evidence(
            out,
            ctx,
            caller,
            segment.name(),
            segment.args(),
            Some(&original),
            depth + 1,
        );
    }
}

fn unique_nested_binding(
    tokens: &crate::ir::CommandTokens,
    offset: u32,
) -> Option<&crate::command_binding::SourceInvocationBinding> {
    let mut children = tokens
        .nested_bindings
        .iter()
        .filter(|(nested_offset, _)| *nested_offset == offset)
        .map(|(_, binding)| binding);
    let binding = children.next()?;
    children.all(|other| other == binding).then_some(binding)
}

/// Scalar lexical compatibility under independently explicit Standalone ownership.
fn record_standalone_invocation_body_evidence(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    tokens: Option<&crate::ir::CommandTokens>,
    depth: u32,
) {
    let args = invocation.arguments.as_slice();
    if crate::depth_guard::MAX_SOURCE_NEST_DEPTH.exceeded(depth + 1) {
        if invocation
            .facts
            .arg_roles
            .iter()
            .any(|&(_, role)| matches!(role, ArgRole::Body | ArgRole::LambdaLiteral))
        {
            record_unenumerable_targets(out, ctx);
        }
        return;
    }
    // A lambda the command receives as a *value* (`apply $fn`) is exactly as
    // unreadable as a script body received as one, and may likewise call
    // anything.  A *literal* lambda needs no walking here — lowering gives it
    // its own body unit, which the scan already visits as a caller.
    let offset = invocation.facts.argument_offset;
    let roles = &invocation.facts.arg_roles;
    let role_indices = |role| {
        roles.iter().filter_map(move |&(index, found)| {
            (found == role).then_some(offset + usize::from(index))
        })
    };
    for idx in role_indices(ArgRole::LambdaLiteral) {
        if args.get(idx).is_some_and(|w| {
            w.as_deref().is_none_or(|word| {
                word_is_whole_substitution(word, ctx.module.lexer_config.nested())
            })
        }) {
            record_unenumerable_targets(out, ctx);
        }
    }
    for idx in role_indices(tcl_registry::ArgRole::Body) {
        let Some(body_text) = args.get(idx).and_then(Option::as_deref) else {
            record_unenumerable_targets(out, ctx);
            continue;
        };
        // `eval $script` / `catch $body` carry no script *text* at all, only
        // a reference to one: unreadable, and it may call any procedure with
        // any argument.  A literal body that merely *mentions* a variable
        // (`catch {puts $x}`) is still walked — the discriminator is "the
        // whole word is one substitution", not "contains a `$`".
        if word_is_whole_substitution(body_text, ctx.module.lexer_config.nested()) {
            record_unenumerable_targets(out, ctx);
            continue;
        }
        // A body whose resolution namespace differs from the caller's must not
        // be walked with the caller's namespace: `namespace eval ::a { helper }`
        // calls `::a::helper`, never the caller's own `helper`
        // (tclsh8.6-confirmed; see `lowering`'s `register_body_unit` call for
        // the same reasoning). Such a command carries an absolute namespace
        // name argument, and lowering has already registered its body as
        // a body unit whose qname encodes it — which
        // `build_extra_call_site_scan_contexts` scans with the *correct*
        // namespace. Recursing here as well would scan it a second time under
        // the wrong one, inventing a call to a same-named proc in the caller's
        // namespace. Cross-file that is a false edge into another file's
        // procedure; in-unit it is merely invisible, because a bare global
        // `::helper` is rarely in a single file's own `known` set.
        //
        // Both name roles are consulted: `ArgRole::NamespaceName` is the
        // precise one (`namespace eval` / `inscope`), while a
        // generic `ArgRole::Name` still covers any other command whose
        // symbolic name word is written absolutely.
        if [
            tcl_registry::ArgRole::NamespaceName,
            tcl_registry::ArgRole::Name,
        ]
        .into_iter()
        .flat_map(role_indices)
        .filter_map(|i| args.get(i).and_then(Option::as_deref))
        .any(crate::naming::is_absolute)
        {
            continue;
        }
        // The same rule for a *definition* body (`Traits::DEFINES_PROCEDURE`
        // — `proc`, `oo::class create`, …): it does not run at the
        // definition site at all, and when it runs it resolves in the
        // defined procedure's own namespace and frame. Lowering has already
        // registered it as a procedure / method / body unit, all of which
        // this scan visits with the right context, so recursing here would
        // only re-walk it under the definer's. Otherwise `namespace eval
        // ::foo { proc runIt {} { uplevel #0 { helper b } } }` invents a
        // call to `::foo::helper` — a proc tclsh8.6/9.0 confirm real Tcl
        // never reaches this way.
        //
        // And the same rule again for a body that runs in another *frame*
        // (`Traits::EVALUATES_IN_SHIFTED_FRAME` — `uplevel`). The frame the
        // level argument selects decides which namespace the body's bare
        // command words resolve against, and the enclosing unit is not it.
        // `upframe_scan_bodies` visits every such body with the frame it
        // actually targets, so recursing here would scan it a second time
        // under the wrong one.
        //
        // Sound-7: `proc runIt {} { catch { uplevel #0 { helper b } } }`
        // inside `::foo`. The `catch` body is walked correctly (catch does
        // not shift anything), and the `uplevel` body inside it was then
        // re-walked as `::foo`, inventing a call to `::foo::helper` on top
        // of the correct `::helper` the upframe scan had already recorded.
        // tclsh8.6/9.0 confirm only `::helper` runs.
        if invocation
            .facts
            .traits
            .intersects(Traits::DEFINES_PROCEDURE | Traits::EVALUATES_IN_SHIFTED_FRAME)
        {
            continue;
        }
        let config = ctx.module.lexer_config.nested();
        let body_base = invocation.effective.words.get(idx + 1).map_or(0, |word| {
            word.source().span.start()
                + u32::from(matches!(word, crate::ir::WordExpr::BracedLiteral { .. }))
        });
        let nested =
            crate::segmenter::segment_commands_with_offset_and_config(body_text, body_base, config);
        let source_map = tcl_lexer::SourceMap::new(body_text).with_base(body_base, 0, 0);
        for cmd in &nested {
            let name = cmd.name();
            if name.is_empty() {
                continue;
            }
            let mut nested_tokens =
                crate::ir::CommandTokens::from_segmented(&source_map, config, cmd);
            if let Some(parent) = tokens {
                nested_tokens.inherit_nested_bindings(parent);
            }
            record_call_site_evidence(
                out,
                ctx,
                caller,
                name,
                cmd.args(),
                Some(&nested_tokens),
                depth + 1,
            );
        }
    }
}

/// What a resolved callee receives at an invocation site: the call's own
/// argument words (an ordinary or dispatched call, and the tail for a
/// registry-declared user-proc invoker), or nothing this scan can see (a
/// callback prefix, whose arguments the runtime appends).
#[derive(Clone, Copy)]
enum IndirectArgs<'a> {
    /// The callee receives exactly these argument words.
    Words(&'a [String]),
    /// The runtime appends arguments this scan cannot see.
    Unknowable,
}

fn record_unenumerable_targets(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
) {
    out.record_unenumerable_caller(ctx.unenumerable_reach);
    for (id, procedure) in ctx.module.procedures.iter() {
        out.record_opaque_body(*id, &procedure.qualified_name);
    }
}

fn record_source_target(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    args: IndirectArgs<'_>,
    source: &crate::command_binding::SourceInvocationBinding,
    target: &crate::command_binding::SourceCommandTarget,
) {
    let local = ctx.module.source_procedure_for_target(target);
    if target.kind == crate::command_binding::BindingKind::Proc
        && (local.is_some() || ctx.known.contains(&target.command))
    {
        match args {
            IndirectArgs::Words(words) => out.record_source_call(
                target.command.clone(),
                &target.prepended,
                words.len(),
                &source.evaluated_argument_values,
                target.implementation_allocation.as_ref(),
            ),
            IndirectArgs::Unknowable => {
                if let Some((id, procedure)) = local {
                    out.record_opaque_body(id, &procedure.qualified_name);
                }
                out.record_opaque_caller(&target.command);
            }
        }
    } else if !target.registry_backed {
        record_unenumerable_targets(out, ctx);
    }
}

/// Attribute one invocation of `word` (a command name, possibly dispatched
/// through a variable) to every procedure it may reach.
///
/// A literal word reaches at most one procedure and costs one resolution.  A
/// dispatch word reaches every procedure in its value set — recorded as an
/// ordinary call site for each, which is what keeps the fix per-target
/// rather than a module-wide wildcard: `$cmd prod` where `cmd` is provably
/// `helper` still lets an agreeing seed stand, and a dispatch that provably
/// names some *other* procedure leaves this one's seed alone.  A word whose
/// values cannot be enumerated names a caller of *something* the scan cannot
/// identify, so it withdraws every seed in the unit.
fn record_invocation(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    word: &str,
    args: IndirectArgs<'_>,
    tokens: Option<&crate::ir::CommandTokens>,
) {
    if let Some(source) = tokens.and_then(|tokens| tokens.source_binding.as_ref())
        && tokens.is_some_and(|tokens| tokens.argv_texts.first().is_some_and(|head| head == word))
    {
        // A selected native operation and later generic lookup are different
        // execution alternatives. Exact execution proof takes precedence over
        // a command table changed by an argument or an earlier chunk command.
        if let Some(target) = source.proved_execution_target() {
            record_source_target(out, ctx, args, source, target);
            return;
        }
        // Compiler rejection can stop before dispatch; it cannot introduce an
        // arbitrary caller. Only actual runtime lookup uncertainty widens the
        // call graph. The retained execution candidates still cover a captured
        // native operation and a possible later generic dispatch.
        if source.unknown {
            record_unenumerable_targets(out, ctx);
        }
        if source.may_be_absent {
            record_unresolved_word_dispatch(out, ctx, word, args);
        }
        for target in source.execution_targets() {
            record_source_target(out, ctx, args, source, target);
        }
        return;
    }
    // A dynamic word is the only thing that reads the previous round, so it
    // alone makes another round necessary.  A module without one converges
    // after a single walk.
    if is_dynamic_word(word) {
        out.consulted_value_sets = true;
    }
    let values = match word_values(ctx, caller, word) {
        WordValues::Literals(values) => values,
        WordValues::Unknown => {
            record_unenumerable_targets(out, ctx);
            return;
        }
    };
    let Some(source) = tokens.and_then(|tokens| tokens.source_binding.as_ref()) else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    for name in &values {
        let binding = if !is_dynamic_word(word)
            && tokens
                .is_some_and(|tokens| tokens.argv_texts.first().is_some_and(|head| head == word))
        {
            source.clone()
        } else {
            source.lookup_command_word(name)
        };
        if binding.unknown {
            record_unenumerable_targets(out, ctx);
        }
        if binding.may_be_absent {
            record_unresolved_word_dispatch(out, ctx, name, args);
        }
        for target in &binding.targets {
            record_source_target(out, ctx, args, source, target);
        }
    }
}

/// The qualified name of the unresolved-command handler this unit defines,
/// or `None` when it defines none — the overwhelmingly common case.
///
/// Tcl routes every command word that resolves to nothing to a single
/// global handler, passing the word itself followed by that call's own
/// arguments (tclsh8.6/9.0-confirmed). A module that defines one therefore
/// has callers no scan of its *direct* call sites can enumerate, and a
/// coincidentally-uniform set of those direct calls would fold a parameter
/// the unresolved words genuinely vary.
///
/// Which command is the handler comes from
/// [`Traits::UNRESOLVED_COMMAND_HANDLER`], never a literal name here. The
/// lookup is global-scope only: a namespace-local `proc unknown` is *not*
/// consulted for unresolved words in that namespace — tclsh8.6/9.0 both
/// dispatch to `::unknown` regardless of the calling namespace.
///
/// [`CommandRegistry::commands_with_trait`] iterates a hash map, so the
/// candidates are sorted before the first match is taken. Exactly one
/// command carries the trait today (pinned by
/// `only_one_command_carries_the_unresolved_handler_trait`), but an
/// order-dependent answer would be a silent source of build-to-build drift
/// the day a dialect adds a second carrier.
fn unresolved_command_handler<'a, S: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    known: &'a HashSet<String, S>,
) -> Option<&'a str> {
    let mut candidates = registry.commands_with_trait(Traits::UNRESOLVED_COMMAND_HANDLER);
    candidates.sort_unstable();
    candidates.into_iter().find_map(|name| {
        let qualified = crate::naming::qualify("::", name);
        known.get(&qualified).map(String::as_str)
    })
}

/// Record the unresolved-command handler's own invocation for a literal
/// command word this scan could not resolve.
///
/// Only fires when the module defines a handler, and only for a word the
/// registry does not know either — everything else either resolves or is a
/// builtin. The word becomes the handler's first argument and the call's
/// own arguments follow, exactly as Tcl passes them.
///
/// **This is additional evidence, never a complete caller set.** The
/// handler is poisoned unconditionally in [`scan_cfg_callers`] the moment
/// the module defines one, so nothing recorded here can seed a fold. It can
/// only widen an already-unfoldable value set, which is what makes the two
/// known imprecisions harmless:
///
/// * *Over-inclusive.* A word bound by something this scan does not model —
///   a `TclOO` class command, an ensemble, a coroutine — also reads as
///   unresolved and contributes a value real Tcl never passes the handler.
/// * *Under-inclusive.* Most of the handler's real callers are words that
///   exist nowhere in the source at all, so no scan can name them.
///
/// An earlier revision claimed the residue "can only retract a fold, never
/// manufacture one". That was false while these dispatches were the
/// handler's *only* recorded call sites: with no other evidence, a
/// coincidentally-uniform set of invented words *was* a fold. The
/// unconditional poison is what makes the claim true.
fn record_unresolved_word_dispatch(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    word: &str,
    args: IndirectArgs<'_>,
) {
    let Some(handler) = ctx.unresolved_handler else {
        return;
    };
    if ctx.registry.get(word).is_some() {
        return;
    }
    match args {
        IndirectArgs::Words(words) => {
            let mut dispatched = Vec::with_capacity(words.len() + 1);
            dispatched.push(word.to_owned());
            dispatched.extend_from_slice(words);
            out.record_call(handler.to_owned(), &dispatched);
        }
        IndirectArgs::Unknowable => out.record_opaque_caller(handler),
    }
}

/// Record the callers a statement creates *without* naming their arguments:
/// a deferred command prefix, and a rebinding of a known command's name.
///
/// Both are real call paths whose argument list this scan can never see —
/// `after 0 helper`, `trace add variable v write helper`, `-command helper`
/// all invoke `helper` with runtime-supplied words appended, and `rename
/// helper other` / `interp alias {} h {} helper` let a call reach `helper`
/// under a name no scan attributed to it. Left out of the evidence entirely
/// they read to [`params_constants_from_native_call_sites`] as "no caller
/// disagrees". Recording them as opaque callers states the truth instead: a
/// call site exists whose arguments are unknown.
///
/// Command-surface-driven throughout — the callback positions come from
/// [`tcl_registry::ArgRole::CommandPrefix`]
/// ([`tcl_registry::model::DocumentCommandSurface::arg_indices_for_role`])
/// and the rebinding forms from
/// [`crate::alias::command_table_transitions`], so no command name appears
/// here.  Covers the `CommandPrefix` forms for both the in-unit and the
/// cross-unit scan.
fn record_indirect_callers(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    tokens: Option<&crate::ir::CommandTokens>,
) {
    let args = invocation.arguments.as_slice();
    // Moving a binding invalidates source and destination names.
    // Registry transitions distinguish movement from definition.
    let tcl_registry::StateTransitionKnowledge::Declared(transitions) =
        &invocation.facts.state_transitions
    else {
        return;
    };
    if transitions.command_bindings().any(|transition| {
        if matches!(
            transition,
            tcl_registry::CommandBindingTransition::Define { .. }
        ) {
            return false;
        }
        let covered = tokens.zip(ctx.source_input).is_some_and(|(tokens, input)| {
            tokens.source_binding.as_ref().is_some_and(|binding| {
                binding
                    .original_covered_alias_transition(
                        tokens,
                        ctx.module,
                        input,
                        ctx.registry,
                        transition,
                    )
                    .is_some()
            })
        });
        !covered
    }) {
        for word in args {
            let Some(word) = word.as_deref() else {
                record_unenumerable_targets(out, ctx);
                continue;
            };
            if word.is_empty() || word.contains(['$', '[']) {
                continue;
            }
            record_invocation(out, ctx, caller, word, IndirectArgs::Unknowable, tokens);
        }
    }
}

/// A genuine source target is only an opaque caller: no fixed callback argv,
/// successful registration, reached frame or current lookup is supplied.
fn record_original_callback_callers(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    tokens: Option<&crate::ir::CommandTokens>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) {
    let Some(tokens) = tokens else {
        return;
    };
    let rows = ctx.source_input.and_then(|input| {
        ctx.identities
            .original_source_callback_targets_for_tokens(input, tokens)
    });
    if let Some(rows) = rows {
        for row in rows {
            if let Some(id) = row.target().and_then(|target| target.original_ir_procedure_id(ctx.procedures)) {
                if let Some(procedure) = ctx.procedures.get(&id) {
                    out.record_opaque_body(id, &procedure.qualified_name);
                }
            } else if row.refusal() != Some(crate::command_binding::OriginalSourceCallbackProcedureRefusal::KnownSourceBarrier) {
                record_unenumerable_targets(out, ctx);
            }
        }
    } else if metadata
        .and_then(|context| {
            crate::registry_invocation::original_callback_invocation_with_metadata_context(
                ctx.registry,
                context,
                tokens,
            )
        })
        .is_some_and(|invocation| {
            invocation
                .facts
                .arg_roles
                .iter()
                .any(|(_, role)| *role == ArgRole::CommandPrefix)
        })
    {
        record_unenumerable_targets(out, ctx);
    }
}

/// Whether [`build_extra_call_site_scan_contexts`] has anything to build for
/// this module — and therefore whether its
/// [`crate::cfg_builder::prepare_cfg_context`] must be computed.
///
/// A single predicate so the two builders that gate on it
/// ([`crate::compilation_unit::CompilationUnit`]'s build and
/// [`scan_source_call_sites`]) cannot drift from what the context builder
/// actually consumes.
pub(crate) fn needs_extra_call_site_scan_contexts(ir_module: &IrModule) -> bool {
    if !ir_module.methods.is_empty() || !ir_module.body_units.is_empty() {
        return true;
    }
    // Only a module with neither pays for the statement walk, and it costs
    // no allocation — a bare `uplevel {…}` is rare enough that building the
    // caller list here just to test it for emptiness would be wasteful.
    let mut found = false;
    walk_module_scripts(ir_module, &mut |_, stmt| {
        found |= matches!(stmt, crate::ir::Statement::UpFrame { .. });
    });
    found
}

/// The synthetic caller name prefix an `uplevel` body's bare CFG carries:
/// unique per occurrence, so its own variable-scope facts never clobber
/// another scope's.
const UPFRAME_SCOPE_PREFIX: &str = "@upframe@";

/// One `uplevel ?level? { … }` body the call-site scan visits as a caller.
struct UpFrameCaller<'a> {
    /// Actual retained frame namespace; unknown callers remain unavailable.
    execution_namespace: crate::ir::ExecutionNamespace,
    /// Synthetic, occurrence-unique variable-scope identity for its CFG.
    scope: String,
    /// The lowered body.
    body: &'a crate::ir::Script,
}

/// Collect each literal upframe body with its independently retained parent namespace.
fn upframe_scan_bodies(ir_module: &IrModule) -> Vec<UpFrameCaller<'_>> {
    let mut out = Vec::new();
    collect_upframes(
        &ir_module.top_level,
        &ir_module
            .top_level
            .execution_namespace(crate::ir::ExecutionNamespace::RuntimeSelected),
        ir_module.resolved_registry(),
        &mut out,
    );
    for procedure in ir_module
        .procedures
        .values()
        .chain(ir_module.body_lowerings().map(|(_, procedure)| procedure))
    {
        collect_upframes(
            &procedure.body,
            &procedure
                .body
                .execution_namespace(crate::ir::ExecutionNamespace::RuntimeSelected),
            ir_module.resolved_registry(),
            &mut out,
        );
    }
    for method in ir_module.methods.values() {
        collect_upframes(
            &method.body,
            &method.execution_namespace,
            ir_module.resolved_registry(),
            &mut out,
        );
    }
    out
}

fn collect_upframes<'a>(
    script: &'a crate::ir::Script,
    parent_namespace: &crate::ir::ExecutionNamespace,
    registry: &CommandRegistry,
    out: &mut Vec<UpFrameCaller<'a>>,
) {
    for statement in &script.statements {
        if let crate::ir::Statement::UpFrame {
            frame_shift,
            absolute,
            body,
            span,
            tokens,
        } = statement
        {
            let execution_namespace = if *frame_shift != 0 {
                crate::ir::ExecutionNamespace::RuntimeSelected
            } else if *absolute {
                tokens
                    .as_ref()
                    .and_then(|tokens| {
                        tokens
                            .source_binding
                            .as_ref()?
                            .original_native_root_namespace_context(tokens, registry)
                    })
                    .map_or(
                        crate::ir::ExecutionNamespace::RuntimeSelected,
                        crate::ir::ExecutionNamespace::SourceContext,
                    )
            } else {
                parent_namespace.clone()
            };
            out.push(UpFrameCaller {
                execution_namespace: execution_namespace.clone(),
                scope: format!("{UPFRAME_SCOPE_PREFIX}{}", span.start()),
                body,
            });
            collect_upframes(body, &execution_namespace, registry, out);
        } else {
            for child in statement.child_scripts() {
                collect_upframes(child, parent_namespace, registry, out);
            }
        }
    }
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

/// Build bare CFGs (no further per-function analysis) for every `TclOO` method,
/// synthetic body unit (`apply` lambda, `namespace eval` body), and static
/// `uplevel` body, so [`collect_call_site_constants`] can walk them as
/// *callers* too.
///
/// Neither is itself ever seeded with `param_constants`
/// (`build_method_units` / `build_body_units` always pass `None` for their
/// own analysis), but a call *from* one of their bodies *to* an ordinary
/// user proc is a real call site whose argument can vary between call
/// sites, exactly like a bare top-level or proc-body call — and invisible to
/// a collector that walks only `cfg_module.top_level` and
/// `cfg_module.procedures`. That is a real, varying call site silently
/// missing from the "every caller agrees" evidence, reached through a
/// method/lambda body instead
/// of namespace-blind recursion or a `catch`/`uplevel` body.
///
/// Returns an empty `Vec` (no cost beyond
/// [`needs_extra_call_site_scan_contexts`]) when the module has none of the
/// three — the overwhelmingly common case — or when `cfg_context` is `None`
/// (all three require it; the builders compute it exactly when
/// [`needs_extra_call_site_scan_contexts`] says so).
pub(crate) fn build_extra_call_site_scan_contexts(
    ir_module: &IrModule,
    cfg_context: Option<&crate::cfg_builder::PreparedCfgContext>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Vec<ExtraCallSiteScanContext> {
    let upframes = upframe_scan_bodies(ir_module);
    if ir_module.methods.is_empty() && ir_module.body_units.is_empty() && upframes.is_empty() {
        return Vec::new();
    }
    let Some(cfg_context) = cfg_context else {
        return Vec::new();
    };
    let build = |qname: &str, body: &crate::ir::Script| {
        crate::cfg_builder::build_cfg_function_with_prepared_context(
            qname,
            body,
            true,
            registry,
            ir_module.plain_command_dispatch,
            cfg_context,
            config,
        )
    };
    let dispatch_barrier = crate::optimiser::method_barrier::compute(ir_module, registry);
    ir_module
        .methods
        .iter()
        .map(|(id, method)| {
            let mqname = method.reporting_label();
            let execution_namespace = method.execution_namespace.clone();
            let requires_runtime_command_namespace = matches!(
                &execution_namespace,
                crate::ir::ExecutionNamespace::RuntimeSelected
            )
                && crate::ir_helpers::requires_runtime_command_namespace(&method.body, registry);
            let cfg = crate::cfg_builder::build_cfg_method_function_with_prepared_context(
                &mqname,
                &method.body,
                execution_namespace.clone(),
                true,
                registry,
                ir_module.plain_command_dispatch,
                cfg_context,
                !dispatch_barrier.allows_locals(*id),
                config,
            );
            ExtraCallSiteScanContext {
                analysis_body: Some(*id),
                cfg,
                execution_namespace,
                requires_runtime_command_namespace,
            }
        })
        .chain(ir_module.body_lowerings().map(|(id, unit)| {
            let execution_namespace = unit
                .body
                .execution_namespace(crate::ir::ExecutionNamespace::RuntimeSelected);
            ExtraCallSiteScanContext {
                analysis_body: Some(*id),
                cfg: build(&unit.qualified_name, &unit.body),
                execution_namespace,
                requires_runtime_command_namespace: false,
            }
        }))
        .chain(upframes.into_iter().map(|up| {
            let requires_runtime_command_namespace = matches!(
                &up.execution_namespace,
                crate::ir::ExecutionNamespace::RuntimeSelected
            )
                && crate::ir_helpers::requires_runtime_command_namespace(up.body, registry);
            ExtraCallSiteScanContext {
                analysis_body: None,
                cfg: build(&up.scope, up.body),
                execution_namespace: up.execution_namespace,
                requires_runtime_command_namespace,
            }
        }))
        .collect()
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

/// Collect per-scope variable literal facts for every body the scan walks as
/// a caller — the input a dispatch word's value set is read from.
fn collect_module_scope_var_facts(
    cfg_module: &CfgModule,
    extra_callers: &[ExtraCallSiteScanContext],
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    module: &IrModule,
    identities: &crate::realm::CommandBindingRealm,
) -> ModuleVarFacts {
    let mut out = ModuleVarFacts::default();
    for (scope, func) in call_site_scan_functions(cfg_module, extra_callers) {
        let facts = collect_original_scope_var_facts(func, surface, module, identities);
        out.cross_frame_write |= facts.cross_frame_write;
        out.scopes.insert(scope, facts);
    }
    out
}

/// Local literal sets and possible output names under the actual Module/CFG
/// owners. A missing source operation poisons dispatch-value enumeration; a
/// conditional authored write never certifies an executed variable-cell store.
fn collect_original_scope_var_facts(
    cfg: &CfgFunction,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    module: &IrModule,
    identities: &crate::realm::CommandBindingRealm,
) -> ScopeVars {
    let mut out = ScopeVars::default();
    let registry = surface.commands();
    let Some(metadata) = crate::registry_invocation::InvocationMetadataContext::for_module(
        registry, module,
    )
    .filter(|_| {
        module
            .retained_source_bindings
            .as_deref()
            .is_some_and(|owner| owner.matches_module(module, registry))
    }) else {
        out.dynamic_name_write = true;
        out.cross_frame_write = true;
        return out;
    };
    let Some(input) = metadata.source_analysis_input() else {
        out.dynamic_name_write = true;
        out.cross_frame_write = true;
        return out;
    };
    for (&block_id, block) in &cfg.blocks {
        for (index, stmt) in block.statements.iter().enumerate() {
            if stmt
                .tokens()
                .is_some_and(|tokens| tokens.synthetic.is_some())
            {
                continue;
            }
            let original = (|| {
                let tokens = cfg.source_tokens_at(block_id, index)?;
                let binding = tokens.source_binding.as_ref()?;
                binding
                    .original_invocation_metadata_for_owner(tokens, &cfg.metadata_context, registry)
                    .ok()??;
                binding.original_invocation_metadata_for_module(tokens, module, registry)?;
                Some(tokens)
            })();
            let Some(tokens) = original else {
                out.dynamic_name_write = true;
                out.cross_frame_write = true;
                continue;
            };
            if note_original_scope_var_statement(stmt, &mut out) {
                continue;
            }
            record_original_scope_call_facts(tokens, identities, input, &mut out);
        }
    }
    out
}

fn record_original_scope_call_facts(
    tokens: &crate::ir::CommandTokens,
    identities: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    out: &mut ScopeVars,
) {
    let Some(words) =
        crate::registry_invocation::source_structure::original_registry_words_for_supplied_tokens(
            identities, input, tokens,
        )
    else {
        out.dynamic_name_write = true;
        out.cross_frame_write = true;
        return;
    };
    let context = input.context_registry();
    let selected = words
        .with_source_schema(&context, |schema| {
            let (_, complete) = schema.authored_source_argument_roles();
            complete.then(|| {
                (
                    schema.authored_source_variable_name_arguments(),
                    schema.semantics.traits,
                )
            })
        })
        .flatten();
    let Some((roles, traits)) = selected else {
        out.dynamic_name_write = true;
        out.cross_frame_write = true;
        return;
    };
    for (ordinal, role) in roles {
        if role != ArgRole::VarWrite {
            continue;
        }
        if let Some(name) = words
            .arguments()
            .get(ordinal)
            .and_then(|word| word.literal_bytes())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        {
            out.note_unknown(name);
        } else {
            out.dynamic_name_write = true;
        }
    }
    // Alias destinations and a shifted/unavailable body are independently possible writes.
    if traits.contains(Traits::CREATES_SCOPE_ALIAS) {
        out.dynamic_name_write = true;
    }
    if traits.contains(Traits::EVALUATES_IN_SHIFTED_FRAME)
        || (traits.contains(Traits::EVALUATES_CODE)
            && words
                .source_script_bodies_for_mutation_coverage(&context)
                .is_none())
    {
        out.cross_frame_write = true;
    }
}

fn note_original_scope_var_statement(stmt: &Statement, out: &mut ScopeVars) -> bool {
    match stmt {
        Statement::AssignConst { name, value, .. } => out.note_assignment(name, Some(value)),
        Statement::AssignValue {
            name,
            value,
            value_needs_backsubst,
            ..
        } => {
            let value =
                (!*value_needs_backsubst && !is_dynamic_word(value)).then_some(value.as_str());
            out.note_assignment(name, value);
        }
        Statement::AssignExpr { name, .. } | Statement::Incr { name, .. } => {
            out.note_write_word(name);
        }
        Statement::UpFrame { .. } => out.cross_frame_write = true,
        Statement::Call { .. } | Statement::Barrier { .. } => return false,
        _ => return true,
    }
    true
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

/// Walk each `(caller qname, CFG)` pair's flattened statements, recording
/// every `Call`/`Barrier` as a call site.  Shared by the in-unit scan and
/// [`scan_source_call_sites`] so the two can never diverge on what counts.
fn scan_cfg_callers<'a>(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    funcs: impl Iterator<Item = (CallerScopeId, &'a CfgFunction)>,
) {
    // The unresolved-command handler's caller set is *never* enumerable, so
    // it is poisoned before a single statement is read.
    //
    // Tcl routes to it every command word that resolves to nothing at the
    // moment of the call — a name typed at a prompt, a name a package
    // autoloads, a name another file introduces, a name produced by string
    // arithmetic. `record_unresolved_word_dispatch` can name *some* of those
    // words, but naming some of a set is not enumerating it: treating those
    // as the complete caller set let a coincidentally-uniform handful seed
    // the handler's parameters and fold its body against values real Tcl
    // never passes. The concrete dispatches stay, purely as extra retracting
    // evidence.
    if let Some(handler) = ctx.unresolved_handler {
        out.record_opaque_caller(handler);
    }
    for (scope, func) in funcs {
        // The typed caller coordinate owns variable facts. A retained procedure is
        // one this evidence map attributes calls *to*; a method or `apply`
        // lambda is not, so its parameters may hold values never seen here.
        let declared = match scope {
            CallerScopeId::Procedure(id) => ctx.procedures.get(&id),
            _ => None,
        };
        let caller = CallerFrame {
            metadata_owner: &func.metadata_context,
            scope,
            params: declared.map_or(&[][..], |p| p.params.as_slice()),
            callers_tracked: declared.is_some() || scope == CallerScopeId::TopLevel,
        };
        let standalone = ctx.source_input.is_none()
            && ctx.module.source_entry.metadata_context.is_standalone()
            && func.metadata_context.is_standalone();
        let config = ctx.module.lexer_config;
        for (&block_id, block) in &func.blocks {
            for (index, stmt) in block.statements.iter().enumerate() {
                if stmt
                    .tokens()
                    .is_some_and(|tokens| !tokens.evaluates_words())
                {
                    continue;
                }
                if stmt.tokens().is_some_and(|tokens| {
                    tokens.synthetic == Some(crate::ir::SyntheticMarker::CallerFrameOpaque)
                }) {
                    // Widens the caller frame for the host call beside it,
                    // which performs (and is scanned for) the invocation.
                    continue;
                }
                if crate::ssa::is_unseen_call_marker(stmt)
                    && unseen_code_is_scanned_host(func, block_id, &block.statements, index)
                {
                    continue;
                }
                let tokens = func.source_tokens_at(block_id, index);
                if let Statement::Call { command, args, .. }
                | Statement::Barrier { command, args, .. } = stmt
                {
                    record_call_site_evidence(out, ctx, &caller, command, args, tokens, 0);
                }
                if standalone {
                    // Explicit compatibility uses local scalar syntax only.
                    for lifted in
                        crate::word_subst::lifted_calls_with_surface(tokens, config, &ctx.surface())
                    {
                        record_call_site_evidence(
                            out,
                            ctx,
                            &caller,
                            &lifted.command,
                            &lifted.args,
                            lifted.tokens.as_ref(),
                            0,
                        );
                    }
                    record_surface_call_sites(out, ctx, &caller, config, stmt);
                } else {
                    record_original_lifted_call_sites(out, ctx, &caller, tokens);
                }
            }
            let tokens = func.source_tokens_at(block_id, usize::MAX);
            if standalone {
                record_terminator_call_sites(
                    out,
                    ctx,
                    &caller,
                    config,
                    block.terminator.as_ref(),
                    tokens,
                );
            } else if matches!(
                block.terminator,
                Some(crate::cfg::Terminator::Return {
                    tokens: None,
                    value: None,
                    value_word: None,
                    span: None,
                    expr: None,
                    ..
                })
            ) || (matches!(
                block.terminator,
                Some(crate::cfg::Terminator::Branch { .. })
            ) && block.statements.iter().any(|statement| {
                statement.tokens().is_some_and(|tokens| {
                    matches!(
                        tokens.synthetic,
                        Some(crate::ir::SyntheticMarker::IterationBindings(_))
                    )
                })
            })) {
                // An implicit exit and typed frozen iterator boundary do not
                // evaluate another original command or substitution.
            } else if matches!(
                block.terminator,
                Some(crate::cfg::Terminator::Return { .. } | crate::cfg::Terminator::Branch { .. })
            ) {
                // The CFG owner is the sole carrier door, including consumed
                // expression conditions and returns. A failed consensus does
                // not reopen inline tokens or parsed AST text.
                record_original_lifted_call_sites(out, ctx, &caller, tokens);
            }
        }
    }
}

/// Complete caller substitutions from the exact retained original vector.
/// Body and expression roles remain source-only selected metadata; missing
/// inventory retracts seeds rather than constructing a source offset.
fn record_original_lifted_call_sites(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    tokens: Option<&crate::ir::CommandTokens>,
) {
    let inventory = (|| {
        let tokens = tokens?;
        if !crate::place_bridge::evaluates_arguments(tokens) {
            return Some(Vec::new());
        }
        if tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments) {
            let metadata = ctx
                .original_argument_metadata(tokens, caller.metadata_owner)
                .ok()??;
            let mut calls = crate::word_subst::checked_original_argument_lifted_calls(
                tokens,
                ctx.registry,
                metadata,
            )?;
            // The enclosing argv has no expression/body-role authority. Each
            // actual nested invocation independently retains its own selected
            // expression roles and complete source inventory.
            for call in calls.clone() {
                let child = call.tokens.as_ref()?;
                let metadata = ctx.original_metadata(child, caller.metadata_owner).ok()??;
                let config = child
                    .source_binding
                    .as_ref()?
                    .original_lexer_config_for_tokens(child)?;
                for nested in
                    crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                        child,
                        config,
                        ctx.registry,
                        metadata,
                    )?
                {
                    if let Some(previous) =
                        calls.iter().find(|previous| previous.span == nested.span)
                    {
                        if previous != &nested {
                            return None;
                        }
                    } else {
                        calls.push(nested);
                    }
                }
            }
            return Some(calls);
        }
        let metadata = ctx
            .original_metadata(tokens, caller.metadata_owner)
            .ok()??;
        let binding = tokens.source_binding.as_ref()?;
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        let input = metadata.source_analysis_input()?;
        if binding
            .original_source_procedure_argument_roles(tokens, input, ctx.registry)
            .is_some()
        {
            // An authenticated original procedure call evaluates lexical
            // argument substitutions; braces cannot become expression roles.
            let mut calls = crate::word_subst::checked_lifted_calls(tokens, config)?;
            for call in &mut calls {
                let child = call.tokens.as_mut()?;
                child.inherit_nested_bindings(tokens);
                call.words = Some(child.clone());
            }
            Some(calls)
        } else {
            crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                tokens,
                config,
                ctx.registry,
                metadata,
            )
        }
    })();
    let Some(inventory) = inventory else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    for lifted in inventory {
        // record_call_site_evidence independently joins every original child
        // to the current Module and this CFG's tagged metadata owner.
        record_call_site_evidence(
            out,
            ctx,
            caller,
            &lifted.command,
            &lifted.args,
            lifted.tokens.as_ref(),
            0,
        );
    }
}

/// Record the call sites a statement runs through a surface its **words** do
/// not carry.
///
/// A fused `AssignExpr`, `ExprEval` or `Incr` keeps a parsed expression or an
/// amount string in place of a `CommandTokens`, so
/// [`crate::word_subst::lifted_calls_with_surface`] above sees nothing at all
/// in it. `set r [expr {[id 9]}]` and `incr t [id 9]` therefore contributed no
/// evidence, and a lone visible `id 7` read as `id`'s complete caller set:
/// O100 folded `return $v` to `return 7` and tclsh 8.6.18's `7 9` became
/// `7 7` (#2118).
///
/// The surfaces are the same ones
/// [`crate::ir_helpers::evaluated_command_substitution_surfaces`] names for
/// the variable-effect walk — the two disagreeing about which commands a
/// statement runs is the underlying defect.
fn record_surface_call_sites(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    config: tcl_lexer::LexerConfig,
    stmt: &Statement,
) {
    let lifted = match stmt {
        Statement::AssignExpr {
            expr, expr_base, ..
        }
        | Statement::ExprEval {
            expr, expr_base, ..
        } => crate::word_subst::lifted_calls_in_expr(expr, *expr_base, config, &ctx.surface()),
        // The amount has no retained word, so a braced `{[f]}` — which runs
        // nothing — is read as a surface too. Over-reporting a caller only
        // retracts evidence; under-reporting one invents agreement.
        Statement::Incr {
            amount: Some(amount),
            ..
        } => crate::word_subst::lifted_calls_in_text(amount, None, config, &ctx.surface()),
        _ => return,
    };
    for mut lifted in lifted {
        if let (Some(nested), Some(parent)) = (&mut lifted.tokens, stmt.tokens()) {
            nested.inherit_nested_bindings(parent);
        }
        record_call_site_evidence(
            out,
            ctx,
            caller,
            &lifted.command,
            &lifted.args,
            lifted.tokens.as_ref(),
            0,
        );
    }
}

/// Record the call sites a block's terminator runs.
///
/// A `return` value and an `if`/`while` condition are terminators, not
/// statements, so the statement loop never reached them:
/// `proc a {} { return [id 9] }` contributed no evidence at all (#2118).
fn record_terminator_call_sites(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    config: tcl_lexer::LexerConfig,
    terminator: Option<&crate::cfg::Terminator>,
    source_tokens: Option<&crate::ir::CommandTokens>,
) {
    let parent = match terminator {
        Some(crate::cfg::Terminator::Return { tokens, .. }) => tokens.as_deref().or(source_tokens),
        _ => source_tokens,
    };
    let lifted = match terminator {
        Some(crate::cfg::Terminator::Return {
            value_word, expr, ..
        }) => {
            let mut lifted = crate::word_subst::lifted_calls_in_word(
                value_word.as_ref(),
                config,
                &ctx.surface(),
            );
            if let Some(expr) = expr {
                lifted.extend(crate::word_subst::lifted_calls_in_expr(
                    expr,
                    None,
                    config,
                    &ctx.surface(),
                ));
            }
            lifted
        }
        Some(crate::cfg::Terminator::Branch {
            condition,
            condition_base,
            ..
        }) => crate::word_subst::lifted_calls_in_expr(
            condition,
            *condition_base,
            config,
            &ctx.surface(),
        ),
        Some(crate::cfg::Terminator::Goto { .. } | crate::cfg::Terminator::Complete { .. })
        | None => return,
    };
    for mut lifted in lifted {
        if let (Some(nested), Some(parent)) = (&mut lifted.tokens, parent) {
            nested.inherit_nested_bindings(parent);
        }
        record_call_site_evidence(
            out,
            ctx,
            caller,
            &lifted.command,
            &lifted.args,
            lifted.tokens.as_ref(),
            0,
        );
    }
}

/// Collect the call sites **another** file contributes, resolved against the
/// whole project's procedure names.
///
/// The cross-file half: a plain library file with no
/// `package provide` is `source`d by a file that calls its procs with a
/// different literal, and the library's own compilation unit — single-source
/// by construction — can never see that caller.  A host with a workspace view
/// (the LSP; the `tcl` CLI when given several files) runs this over every
/// other file and hands the result to
/// [`crate::compilation_unit::CompilationUnit`]'s build, which merges it into
/// the in-unit evidence before seeding.
///
/// `known` is the **project-wide** set of procedure qualified names whose
/// evidence the caller wants to collect. It does not certify their runtime
/// bindings, immutable bodies, or compiler-hook registration. Exact evidence
/// also requires a reached source-owned definition or actual runtime entry;
/// a name-only external declaration remains uncertain. `declared` is the **scanned file's
/// own** declaration set: a stub binds the file it is written in, so the
/// argument roles that decide what counts as a call site here are that
/// file's, not the host's.  The scan is deliberately the same
/// lowering → CFG → [`record_call_site_evidence`] path the in-unit scan takes
/// (including its `TclOO` method / `apply` / `namespace eval` body-unit
/// callers and its `ArgRole::Body` recursion), so cross-file evidence is
/// exactly what this unit would have collected had the two files been one.
#[must_use]
pub fn scan_source_call_sites<S: std::hash::BuildHasher>(
    source: &str,
    registry: &CommandRegistry,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    dialect: &'static tcl_dialect::DialectProfile,
    known: &HashSet<String, S>,
    dispatch_reach: &[String],
) -> CallSiteEvidence {
    scan_source_call_sites_with_entry(
        source,
        registry,
        declared,
        dialect,
        known,
        dispatch_reach,
        SourceCallSiteEntry {
            entry: None,
            input: None,
        },
    )
}

/// Collect cross-file caller evidence with the driver's actual source entry.
/// Loaded provider and source implementation identity are retained throughout
/// lowering and CFG construction; a known procedure name alone grants no proof.
#[must_use]
pub fn scan_source_call_sites_with_source_entry<S: std::hash::BuildHasher>(
    source: &str,
    registry: &CommandRegistry,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    dialect: &'static tcl_dialect::DialectProfile,
    known: &HashSet<String, S>,
    dispatch_reach: &[String],
    entry: &crate::command_binding::SourceAnalysisEntry,
) -> CallSiteEvidence {
    scan_source_call_sites_with_entry(
        source,
        registry,
        declared,
        dialect,
        known,
        dispatch_reach,
        SourceCallSiteEntry {
            entry: Some(entry),
            input: None,
        },
    )
}

/// Current external source declarations; these receipts grant no loaded provider.
#[derive(Debug, Clone)]
pub struct ExternalSourceProcedureFacts {
    input: crate::analyser::ResolvedAnalysisInput,
    image: tcl_lexer::SourceImage,
    declarations: Vec<
        crate::signature_scan::original_name::SourceDeclarationMetadata<crate::analyser::ProcDef>,
    >,
}

impl ExternalSourceProcedureFacts {
    /// Retain only original declarations from this complete input and source image.
    #[must_use]
    pub fn capture(
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        declarations: &[crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >],
    ) -> Option<Self> {
        let context = input.context_registry();
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            context.commands(),
            input,
            input.lexer_config(),
            Some(input.unit_profile()),
        )?;
        if declarations.iter().any(|declaration| {
            declaration.name_input().source_image() != image
                || declaration.name_input().lexer_config() != input.lexer_config()
        }) {
            return None;
        }
        Some(Self {
            input: input.clone(),
            image: image.clone(),
            declarations: declarations.to_vec(),
        })
    }
}

/// Scan the actual source entry while preserving independently supplied external declarations.
/// External metadata only withdraws seeds; it never installs a command or proves argv.
#[must_use]
pub fn scan_source_call_sites_with_original_source<S: std::hash::BuildHasher>(
    source: &str,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    known: &HashSet<String, S>,
    dispatch_reach: &[String],
    input: &crate::analyser::ResolvedAnalysisInput,
    entry: Option<&crate::command_binding::SourceAnalysisEntry>,
    external: &[ExternalSourceProcedureFacts],
) -> CallSiteEvidence {
    let context = input.context_registry();
    let mut evidence = scan_source_call_sites_with_entry(
        source,
        context.commands(),
        declared,
        input.unit_profile(),
        known,
        dispatch_reach,
        SourceCallSiteEntry {
            entry,
            input: Some(input),
        },
    );
    for facts in external {
        for declaration in &facts.declarations {
            if declaration.name_input().source_image() != &facts.image
                || declaration.name_input().lexer_config() != facts.input.lexer_config()
            {
                continue;
            }
            // A declaration cannot prove that this interpreter loaded its provider.
            // Current external reach therefore remains an opaque caller obligation.
            if known.contains(&declaration.metadata().qualified_name) {
                evidence
                    .external_opaque_declarations
                    .insert(declaration.declaration_site().clone());
            }
        }
    }
    evidence
}

/// Collect caller evidence with complete retained source metadata and the
/// independent driver entry. Missing metadata in the entry-only API retracts
/// seeds; this supplied input never grants execution or a loaded provider.
#[must_use]
pub fn scan_source_call_sites_with_source_input<S: std::hash::BuildHasher>(
    source: &str,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    known: &HashSet<String, S>,
    dispatch_reach: &[String],
    input: &crate::analyser::ResolvedAnalysisInput,
    entry: &crate::command_binding::SourceAnalysisEntry,
) -> CallSiteEvidence {
    let context = input.context_registry();
    scan_source_call_sites_with_entry(
        source,
        context.commands(),
        declared,
        input.unit_profile(),
        known,
        dispatch_reach,
        SourceCallSiteEntry {
            entry: Some(entry),
            input: Some(input),
        },
    )
}

#[derive(Clone, Copy)]
struct SourceCallSiteEntry<'a> {
    entry: Option<&'a crate::command_binding::SourceAnalysisEntry>,
    input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
}

fn lower_source_callers(
    source: &str,
    registry: &CommandRegistry,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    dialect: &'static tcl_dialect::DialectProfile,
    supplied: SourceCallSiteEntry<'_>,
) -> (IrModule, CfgModule, Vec<ExtraCallSiteScanContext>) {
    let config = supplied.input.map_or_else(
        || tcl_lexer::LexerConfig::from_grammar(dialect.grammar),
        crate::analyser::ResolvedAnalysisInput::lexer_config,
    );
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
        .with_dialect(Some(dialect))
        .with_declared_commands(declared);
    if let Some(input) = supplied.input {
        lowerer = lowerer.with_resolved_analysis_input(input.clone());
    }
    if let Some(entry) = supplied.entry {
        lowerer.set_source_analysis_options(entry.options());
    }
    let mut ir_module = crate::lowering::lower_to_ir_with(lowerer, source);
    crate::specialise_factories::specialise_factories(&mut ir_module, registry);
    crate::inline_uplevel::inline_uplevel_passthrough(&mut ir_module, registry);
    let prepared_cfg_context = crate::cfg_builder::prepare_cfg_context_bundle(&ir_module, registry);
    let config = ir_module.lexer_config;
    let cfg_module = crate::cfg_builder::build_cfg_with_registry_and_context(
        &ir_module,
        false,
        registry,
        &prepared_cfg_context,
        config,
    );
    let cfg_context =
        needs_extra_call_site_scan_contexts(&ir_module).then_some(prepared_cfg_context);
    let extra =
        build_extra_call_site_scan_contexts(&ir_module, cfg_context.as_ref(), registry, config);
    (ir_module, cfg_module, extra)
}

fn scan_source_call_sites_with_entry<S: std::hash::BuildHasher>(
    source: &str,
    registry: &CommandRegistry,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    dialect: &'static tcl_dialect::DialectProfile,
    known: &HashSet<String, S>,
    dispatch_reach: &[String],
    supplied: SourceCallSiteEntry<'_>,
) -> CallSiteEvidence {
    let mut out = CallSiteEvidence::default();
    if known.is_empty() {
        return out;
    }
    // The explicit ingress profile owns native semantics as well as parsing.
    // A declaration catalogue without a profile cannot prove native dispatch.
    let projected;
    let registry = if registry.profile().is_none() && supplied.input.is_none() {
        projected = registry.project_for_profile(dialect);
        &projected
    } else {
        registry
    };
    let (ir_module, cfg_module, extra) =
        lower_source_callers(source, registry, declared, dialect, supplied);
    // The cross-file scan resolves a dispatch word exactly as the in-unit one
    // does — `scan_cfg_callers`/`record_call_site_evidence` are shared, so a
    // `set cmd helper; $cmd dev` in *another* file retracts this unit's seed
    // just as an in-unit one would.  Without the same var facts and fixpoint
    // here, an unreadable dispatch would stop retracting seeds across the
    // file boundary.
    // What this file's own unreadable dispatches can reach is a *project*
    // fact, not a file one — `source` puts two files in one interpreter in
    // both directions, so a library reaches its sourcer's procedures just as
    // its sourcer reaches the library's.  Only the host knows that graph, so
    // it supplies the set (`tcl_lsp_db::file_dispatch_reach`); with none
    // given, the file's own declarations are the honest bound.
    let reach: Vec<String> = if dispatch_reach.is_empty() {
        let mut own: Vec<String> = ir_module
            .procedures
            .values()
            .map(|procedure| procedure.qualified_name.clone())
            .collect();
        own.sort();
        own
    } else {
        dispatch_reach.to_vec()
    };
    // `known` here is the *project*-wide procedure set, so a handler defined
    // in any scanned file is visible — matching Tcl, where `::unknown` is one
    // command shared by the whole interpreter, not a per-file one.
    let unresolved_handler = unresolved_command_handler(registry, known);
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    // Lowering owns this exact independent source ingress. Query that owner;
    // refusal retracts reachable caller seeds instead of replaying its text.
    let Some(identities) = crate::realm::document_realm_bindings_for_module(&ir_module, registry)
    else {
        out.record_unenumerable_caller(&reach);
        return out;
    };
    let var_facts = collect_module_scope_var_facts(
        &cfg_module,
        &extra,
        &tcl_registry::model::DocumentCommandSurface::new(registry, declared),
        &ir_module,
        &identities,
    );
    out = run_to_fixpoint(|previous| {
        let ctx = CallSiteScanCtx {
            known,
            registry,
            module: &ir_module,
            source_input: crate::registry_invocation::InvocationMetadataContext::for_module(
                registry, &ir_module,
            )
            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input),
            identities: &identities,
            declared,
            var_facts: &var_facts,
            previous,
            procedures: &ir_module.procedures,
            unenumerable_reach: &reach,
            unresolved_handler,
        };
        let funcs = call_site_scan_functions(&cfg_module, &extra);
        let mut round = CallSiteEvidence::default();
        record_future_callers(&mut round, &ctx, &ir_module.future_call_sites);
        record_runtime_selected_extra_callers(&mut round, &ctx, &extra);
        scan_cfg_callers(&mut round, &ctx, funcs);
        round
    });

    out
}

/// Possible registry-declared unit boundaries under retained source metadata.
/// Missing or foreign input contributes no invented boundary trait; the caller
/// collector independently retracts seeds when it cannot own the source context.
///
/// A pure union of [`CommandRegistry::unit_linkage`] over every resolved
/// `Call`/`Barrier` statement in the module — top level, every
/// proc/method/body-unit, and every nested control-flow body.  No command
/// name appears here: `package provide`, `source`, `namespace export` and
/// friends are recognised only through the traits their specs carry, so
/// teaching the compiler about a new boundary command is a registry edit
/// (see [`tcl_registry::traits::UNIT_LINKAGE_TRAITS`]).
///
/// Neither a raw-text `package provide` substring scan nor an IR walk keyed
/// on the `package`/`provide` word pair would do: the first both
/// over-triggers (any script merely *mentioning* the phrase in a comment or
/// string disables every interprocedural seed in the file) and under-triggers
/// (`package\tprovide`, `::package provide`); the second knows a command by
/// name and misses every other way a file admits to being part of a larger
/// program.
#[must_use]
pub fn scan_unit_linkage(
    ir_module: &IrModule,
    registry: &CommandRegistry,
    _dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Traits {
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, ir_module);
    let mut found = Traits::empty();
    let mut visit = |script: &crate::ir::Script, stmt: &crate::ir::Statement| {
        let Some(tokens) = script.retained_source_tokens_for_statement(stmt) else {
            return;
        };
        let Some(binding) = tokens.source_binding.as_ref() else {
            return;
        };
        let context = if metadata.is_some() {
            binding
                .original_invocation_metadata_for_module(tokens, ir_module, registry)
                .map(Some)
        } else if ir_module.source_entry.metadata_context.is_standalone() {
            binding
                .original_invocation_metadata_for_owner(
                    tokens,
                    &ir_module.source_entry.metadata_context,
                    registry,
                )
                .ok()
        } else {
            None
        };
        let Some(context) = context else {
            return;
        };
        let invocation = context.filter(|metadata| binding.original_invocation_source_purpose(
            tokens, *metadata, registry,
        ) == Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical))
            .and_then(|metadata| crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(registry, metadata, tokens))
            .or_else(|| crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(registry, context, tokens));
        if let Some(invocation) = invocation {
            found |= invocation
                .facts
                .traits
                .intersection(tcl_registry::UNIT_LINKAGE_TRAITS);
        }
        // Possibility withdraws caller closure, independently of execution.
        if let Some(assistance) =
            crate::registry_invocation::registry_invocation_assistance_with_metadata_context(
                registry, context, tokens,
            )
        {
            for candidate in assistance.candidates {
                found |= candidate
                    .possible_traits
                    .intersection(tcl_registry::UNIT_LINKAGE_TRAITS);
            }
        }
    };
    walk_module_scripts(ir_module, &mut visit);
    found
}

/// Visit every statement of every script the module owns — top level,
/// procedures, `TclOO` methods, and synthetic body units — descending into
/// nested control-flow bodies.
fn walk_module_scripts<'a>(
    ir_module: &'a IrModule,
    visit: &mut impl FnMut(&'a crate::ir::Script, &'a crate::ir::Statement),
) {
    walk_script(&ir_module.top_level, visit);
    for proc in ir_module.procedures.values() {
        walk_script(&proc.body, visit);
    }
    for method in ir_module.methods.values() {
        walk_script(&method.body, visit);
    }
    for (_, unit) in ir_module.body_lowerings() {
        walk_script(&unit.body, visit);
    }
}

/// Visit every statement in `script`, descending into every nested
/// control-flow body.  Written as a module-level pair with
/// [`walk_statement`] rather than nested inside its callers so the
/// mutual recursion reads as two ordinary functions.
fn walk_script<'a>(
    script: &'a crate::ir::Script,
    visit: &mut impl FnMut(&'a crate::ir::Script, &'a crate::ir::Statement),
) {
    for stmt in &script.statements {
        walk_statement(script, stmt, visit);
    }
}

/// The [`walk_script`] half that dispatches one statement.
fn walk_statement<'a>(
    script: &'a crate::ir::Script,
    stmt: &'a crate::ir::Statement,
    visit: &mut impl FnMut(&'a crate::ir::Script, &'a crate::ir::Statement),
) {
    use crate::ir::Statement;
    visit(script, stmt);
    // Only the body-bearing arms have anything left to do — everything else
    // is a leaf the visit above has already seen.
    match stmt {
        Statement::Block { body, .. }
        | Statement::UpFrame { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. } => walk_script(body, visit),
        Statement::If {
            clauses, else_body, ..
        } => {
            for clause in clauses {
                walk_script(&clause.body, visit);
            }
            if let Some(body) = else_body {
                walk_script(body, visit);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(init, visit);
            walk_script(next, visit);
            walk_script(body, visit);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(body, visit);
            for handler in handlers {
                walk_script(&handler.body, visit);
            }
            if let Some(body) = finally_body {
                walk_script(body, visit);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for arm in arms {
                if let Some(body) = &arm.body {
                    walk_script(body, visit);
                }
            }
            if let Some(body) = default_body {
                walk_script(body, visit);
            }
        }
        Statement::Call { .. }
        | Statement::Barrier { .. }
        | Statement::NativeCall { .. }
        | Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Return { .. } => {}
    }
}

/// Everything the interprocedural seed needs to know about *who else* can
/// call this unit's procedures.
pub(crate) struct UnitCallerView<'a> {
    /// Registry-declared boundaries the file itself crosses
    /// ([`scan_unit_linkage`]).
    pub linkage: Traits,
    /// Whether a host supplied cross-file call-site evidence for this unit.
    /// With evidence, `merged` is the whole project's view and a
    /// registry-declared boundary does not have to be treated as an unknown
    /// caller; without it, the unit is on its own and any boundary sinks the
    /// seed.
    pub has_cross_file_evidence: bool,
    /// Prepared trust projection for the declared identity of retained
    /// procedures. This is intentionally narrower than the optimiser's
    /// command-mutation projection: an unresolved command or an external
    /// body can make builtin effects opaque without itself proving that a
    /// retained procedure name was rebound.
    pub proc_binding_trust: &'a crate::command_binding::ProcBindingTrustProjection,
    /// Actual analysed source bytes for declaration correlation.
    pub source: Option<&'a str>,
    /// The procedures the file's own code can enter: reachable over direct
    /// calls from the top level (method and lambda bodies count as entry
    /// points). `None` when every procedure may be entered. A procedure
    /// outside the set runs only if a caller this unit cannot see calls it,
    /// so the callers it does see — its own recursive call, say — say nothing
    /// of its parameters on entry (#2522).
    pub entered: Option<&'a std::collections::HashSet<crate::ir::ProcedureId>>,
}

/// Boundaries that publish this file's commands to callers **no** host
/// enumeration can bound — another checkout can `package require` this one,
/// or `namespace import` from it.  Distinct from `LOADS_EXTERNAL_UNIT`, whose
/// implied caller is normally a project file the host has already scanned.
const UNBOUNDABLE_BOUNDARIES: Traits = Traits::PROVIDES_PACKAGE.union(Traits::EXPORTS_COMMAND);

impl UnitCallerView<'_> {
    /// Whether a registry-declared boundary rules out seeding outright — see
    /// [`params_constants_from_native_call_sites`]'s gate for the full rule.
    fn declines_seeding(&self) -> bool {
        if self.linkage.intersects(UNBOUNDABLE_BOUNDARIES) {
            return true;
        }
        !self.has_cross_file_evidence && self.linkage.intersects(Traits::LOADS_EXTERNAL_UNIT)
    }
}

/// Build the SCCP `param_constants` seed for `qname` from collected call-site
/// literals: bind `(param, 0)` only when every caller passes the same single
/// literal at that position.
///
/// Beyond the per-slot literal-uniformity test
/// ([`CalleeEvidence::uniform_literal_at`]), two whole-module gates must also
/// hold — each closes a way the recorded call sites could be an incomplete
/// picture of every real caller (an unproven "every caller I found agrees"
/// says nothing about callers no scan could see):
///
/// - `!proc_binding_trust.trusts_proc_binding(qname)` — `qname`'s own
///   binding may have been perturbed by `rename` / `interp alias` / a
///   dynamic proc redefinition anywhere in the module, so a call reaching
///   it at runtime need not be one this scan attributed to it (and vice
///   versa). The projection deliberately does not inherit broader
///   command-effect opacity from an unrelated unresolved or external body.
/// - **No registry-declared unit boundary the evidence cannot cover.** The
///   two kinds are treated differently, because a host's enumeration can only
///   ever bound one of them:
///   - `PROVIDES_PACKAGE` (`package provide` / `ifneeded`) and
///     `EXPORTS_COMMAND` (`namespace export`, `namespace ensemble`) publish
///     this file's commands as an API surface. Their consumers need not be in
///     the host's project at all — another checkout can `package require`
///     this one — so no enumeration bounds them and the seed is declined
///     **unconditionally**.
///   - `LOADS_EXTERNAL_UNIT` (`source`, `load`, `package require`,
///     `auto_load`, `auto_import`) says another unit's script runs here and
///     can call back in. That unit is normally a project file, so a host that
///     supplied cross-file evidence
///     ([`UnitCallerView::has_cross_file_evidence`]) has already contributed
///     its call sites and the seed proceeds on the union; with no host view
///     it is a blind spot and the seed is declined.
///
/// - **The scan enumerated every caller** ([`CallSiteEvidence::
///   record_unenumerable_caller`]). A dispatch word (`$cmd args`) is resolved by
///   *value*, so an enumerable one is ordinary evidence attributed to each
///   name it may hold — deliberately not a module-wide wildcard, which is
///   what lets `$cmd prod` keep a sound seed while `$cmd dev` retracts only
///   the proc it can reach. But a dispatch whose values cannot be
///   enumerated, or a script a command receives only as a *value* (`eval
///   $script`, `apply $fn`), names a caller of *something* this scan cannot
///   identify — and since it could be any procedure with any argument, every
///   seed in the unit is withdrawn.
#[cfg(test)]
pub(crate) fn params_constants_from_call_sites(
    params: &[String],
    evidence: &CallSiteEvidence,
    qname: &str,
    view: &UnitCallerView<'_>,
) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
    use crate::analyses::{ConstValue, LatticeValue};
    if view.declines_seeding() {
        return None;
    }
    if !view.proc_binding_trust.trusts_proc_binding(qname) {
        return None;
    }
    let callee = evidence.get(qname)?;
    let mut consts: HashMap<(String, crate::ssa::Version), LatticeValue> = HashMap::new();
    for (index, pname) in params.iter().enumerate() {
        // Only a *trailing* `args` is Tcl's variadic catch-all
        // (`TclCreateProc`, `generic/tclProc.c`): in `proc f {args x}` the
        // first word is an ordinary parameter and `x` is required.  Every
        // position from a trailing `args` on absorbs an unbounded, per-call
        // word list, so no single position beyond it can be literal-uniform.
        if pname == "args" && index + 1 == params.len() {
            break;
        }
        if let Some(value) = callee.uniform_literal_at(index) {
            consts.insert(
                (pname.clone(), 0),
                LatticeValue::Const(ConstValue::String(value.to_owned())),
            );
        }
    }
    if consts.is_empty() {
        None
    } else {
        Some(consts)
    }
}

/// Caller contents projected through the selected native activation plan.
/// Names identify logical formal inputs, never physical cell or object identity;
/// consumers still need the captured incoming binding before using a seed.
pub(crate) fn params_constants_from_native_call_sites(
    module: &IrModule,
    id: crate::ir::ProcedureId,
    procedure: &crate::ir::Procedure,
    grammar: Option<tcl_dialect::ParameterGrammar>,
    evidence: &CallSiteEvidence,
    view: &UnitCallerView<'_>,
) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
    use crate::analyses::{ConstValue, LatticeValue};
    use tcl_syntax::formal_params::{
        FormalArgumentBinding, bind_formal_arguments, parse_formal_parameters_in,
    };

    if view.declines_seeding()
        || !view
            .proc_binding_trust
            .trusts_proc_binding(&procedure.qualified_name)
    {
        return None;
    }
    let grammar = grammar?;
    // Jim's ordinary value binding can write a retained static fallback.
    // This logical caller table does not carry that activation's physical
    // formal binding, so it cannot project a fresh incoming slot for Jim.
    if grammar != tcl_dialect::ParameterGrammar::Tcl {
        return None;
    }
    let formals = parse_formal_parameters_in(&procedure.params_raw, grammar).ok()?;
    let mut names = BTreeSet::new();
    if formals.iter().any(|formal| !names.insert(&formal.name)) {
        return None;
    }
    if !view.has_cross_file_evidence && view.entered.is_some_and(|entered| !entered.contains(&id)) {
        return None;
    }
    let callee = evidence.get_for_body(module, id)?;
    if callee.opaque_caller
        || view.source.is_some_and(|source| {
            callee.source_implementations.iter().any(|allocation| {
                !allocation.matches_source_declaration(source, procedure.span.start())
            })
        })
    {
        return None;
    }
    let mut agreed: Option<BTreeMap<usize, String>> = None;
    for &count in &callee.arg_counts {
        let bindings = bind_formal_arguments(&formals, count, grammar).ok()?;
        let mut values = BTreeMap::new();
        for binding in bindings {
            match binding {
                FormalArgumentBinding::Value {
                    parameter,
                    argument,
                } => {
                    if let Some(value) = callee.uniform_literal_at(argument) {
                        values.insert(parameter, value.to_owned());
                    }
                }
                FormalArgumentBinding::Default { parameter } => {
                    values.insert(parameter, formals[parameter].default.clone()?);
                }
                FormalArgumentBinding::CallerLink { .. } => return None,
                FormalArgumentBinding::Rest {
                    parameter, name, ..
                } => {
                    if parameter + 1 != formals.len() || name != formals[parameter].name {
                        return None;
                    }
                }
            }
        }
        if let Some(agreed) = &mut agreed {
            agreed.retain(|parameter, value| values.get(parameter) == Some(value));
        } else {
            agreed = Some(values);
        }
    }
    let constants: HashMap<_, _> = agreed?
        .into_iter()
        .map(|(parameter, value)| {
            (
                (formals[parameter].name.clone(), 0),
                LatticeValue::Const(ConstValue::String(value)),
            )
        })
        .collect();
    (!constants.is_empty()).then_some(constants)
}

#[cfg(test)]
mod tests;
