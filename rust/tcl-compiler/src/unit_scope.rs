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
use crate::value_shapes::{
    is_pure_var_ref, parse_command_substitution_with_config, whole_word_scalar_var_name,
};

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

/// Call-site literal evidence for a set of callees, keyed by resolved
/// qualified name.
///
/// The same shape backs the in-unit scan and the cross-unit one, so a host
/// supplying workspace evidence is feeding the seed exactly what the unit
/// would have collected had the other file been part of it.
#[derive(Debug, Clone, Default)]
pub struct CallSiteEvidence {
    by_callee: BTreeMap<String, CalleeEvidence>,
    /// Whether this round consulted a parameter's value set, i.e. whether
    /// the evidence depends on the previous round and the fixpoint must
    /// iterate.  Not part of the evidence's meaning, so it is excluded from
    /// equality.
    consulted_value_sets: bool,
}

impl PartialEq for CallSiteEvidence {
    fn eq(&self, other: &Self) -> bool {
        self.by_callee == other.by_callee
    }
}

impl Eq for CallSiteEvidence {}

impl CallSiteEvidence {
    /// Whether no call site at all was recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_callee.is_empty()
    }

    /// The evidence recorded for `qname`, if any.
    #[must_use]
    pub fn get(&self, qname: &str) -> Option<&CalleeEvidence> {
        self.by_callee.get(qname)
    }

    /// Every callee name evidence was recorded for.
    pub fn callees(&self) -> impl Iterator<Item = &str> {
        self.by_callee.keys().map(String::as_str)
    }

    /// Fold `other`'s call sites in.  See [`CalleeEvidence::merge_from`] —
    /// merging is monotone, so it can only ever retract a fold.
    pub fn merge_from(&mut self, other: &Self) {
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
        for qname in callees {
            if let Some(evidence) = self.by_callee.get(qname) {
                out.by_callee.insert(qname.to_owned(), evidence.clone());
            }
        }
        out
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
        evidence.arg_counts.insert(prefix.len() + argument_count);
        for index in 0..prefix.len() + argument_count {
            let slot = evidence.slots.entry(index).or_default();
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
    pub declared: Option<&'a tcl_registry::model::DeclaredSurface>,
    pub dialect: &'static tcl_dialect::DialectProfile,
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
    source_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    metadata_context: Option<&'a tcl_registry::model::ContextRegistry>,
    identities: &'a crate::realm::CommandBindingRealm,
    /// The document's own `# tcl-lsp: stub` declarations, when it has any.
    /// Read through [`CallSiteScanCtx::surface`] so a declared argument role
    /// reaches this scan exactly as a catalogue one does.
    declared: Option<&'a tcl_registry::model::DeclaredSurface>,
    dialect: &'static tcl_dialect::DialectProfile,
    /// Per-scope literal-value facts for local variables, from
    /// [`collect_module_scope_var_facts`] — what a dispatch word `$cmd` may
    /// evaluate to.
    var_facts: &'a ModuleVarFacts,
    /// The previous fixpoint round's evidence, consulted (never written)
    /// when resolving a parameter's value set.
    previous: &'a CallSiteEvidence,
    /// The unit's procedures, for the declared parameter list of a body the
    /// scan walks as a caller.
    procedures: &'a HashMap<String, crate::ir::Procedure>,
    /// The procedures an unenumerable dispatch in this scan may reach — see
    /// [`unenumerable_reach`] and
    /// [`CallSiteEvidence::record_unenumerable_caller`].
    unenumerable_reach: &'a [String],
    /// The qualified name of the module's own unresolved-command handler,
    /// when it defines one — see [`unresolved_command_handler`].
    unresolved_handler: Option<&'a str>,
}

impl<'a, S> CallSiteScanCtx<'a, S> {
    /// The catalogue extended by the document's declarations — the one door
    /// every argument-role question in this scan goes through.
    fn surface(&self) -> tcl_registry::model::DocumentCommandSurface<'a> {
        tcl_registry::model::DocumentCommandSurface::new(self.registry, self.declared)
    }
}

/// One body the scan walks as a *caller*.
///
/// `resolve_as` and `scope` differ for a `TclOO` method: its bare command
/// words resolve against the global namespace (tclsh-confirmed), while its
/// local variables live in the method's own frame.
struct CallerFrame<'a> {
    /// Qualified-name context bare command words resolve against.
    _resolve_as: &'a str,
    /// Variable-scope identity — the key into `var_facts`, and the callee
    /// key whose recorded arguments a `$param` dispatch word may take.
    scope: &'a str,
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
    scopes: HashMap<String, ScopeVars>,
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
    let Some(var) = whole_word_scalar_var_name(word) else {
        return WordValues::Unknown;
    };
    // A namespace-qualified reference names a variable any body in the
    // module (or another file) may write; this scan models local frames
    // only.
    if var.contains("::") {
        return WordValues::Unknown;
    }
    // A script running in a frame this scan cannot read may have assigned
    // any local of any scope.
    if ctx.var_facts.cross_frame_write {
        return WordValues::Unknown;
    }
    let Some(scope_vars) = ctx.var_facts.scopes.get(caller.scope) else {
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
    match ctx
        .previous
        .get(caller.scope)
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
    is_pure_var_ref(word) || parse_command_substitution_with_config(word, config).is_some()
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
    if tokens
        .and_then(|tokens| tokens.source_binding.as_ref())
        .is_some_and(|binding| {
            binding.runtime_reachability()
                == crate::command_binding::SourceRuntimeReachability::NotEntered
        })
    {
        // Complete source execution coverage proves this dispatch was not
        // entered. Its independent compiler visitation still matters to
        // admission, but cannot introduce an arbitrary runtime caller.
        return;
    }
    let Some(context) = ctx.metadata_context else {
        out.record_unenumerable_caller(ctx.unenumerable_reach);
        return;
    };
    record_original_callback_callers(out, ctx, tokens);
    let context = Some(context.into());
    if let Some(tokens) = tokens
        && let Some(invocation) =
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
    record_invocation(out, ctx, caller, command, IndirectArgs::Words(args), tokens);
    if is_dynamic_word(command) {
        // Which of a computed head's arguments carry scripts, callbacks, or a
        // callee name is unknowable; the dispatch itself is already accounted
        // for above.
        return;
    }
    let Some(invocation) = tokens.and_then(|tokens| {
        crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
            ctx.registry,
            context,
            tokens,
        )
    }) else {
        return;
    };
    record_indirect_callers(out, ctx, caller, &invocation, tokens);
    record_invocation_body_evidence(out, ctx, caller, &invocation, tokens, depth);
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
        out.record_unenumerable_caller(ctx.unenumerable_reach);
    }
    for target in &callee.targets {
        if target.kind == crate::command_binding::BindingKind::Proc
            && ctx.known.contains(&target.command)
        {
            out.record_source_call(
                target.command.clone(),
                &target.prepended,
                invocation.arguments.len(),
                &invocation.arguments,
                target.implementation_allocation.as_ref(),
            );
        } else if !target.registry_backed {
            out.record_unenumerable_caller(ctx.unenumerable_reach);
        }
    }
}

fn record_invocation_body_evidence(
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
            out.record_unenumerable_caller(ctx.unenumerable_reach);
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
                word_is_whole_substitution(
                    word,
                    tcl_lexer::LexerConfig::from_grammar(ctx.dialect.grammar),
                )
            })
        }) {
            out.record_unenumerable_caller(ctx.unenumerable_reach);
        }
    }
    for idx in role_indices(tcl_registry::ArgRole::Body) {
        let Some(body_text) = args.get(idx).and_then(Option::as_deref) else {
            out.record_unenumerable_caller(ctx.unenumerable_reach);
            continue;
        };
        // `eval $script` / `catch $body` carry no script *text* at all, only
        // a reference to one: unreadable, and it may call any procedure with
        // any argument.  A literal body that merely *mentions* a variable
        // (`catch {puts $x}`) is still walked — the discriminator is "the
        // whole word is one substitution", not "contains a `$`".
        if word_is_whole_substitution(
            body_text,
            tcl_lexer::LexerConfig::from_grammar(ctx.dialect.grammar),
        ) {
            out.record_unenumerable_caller(ctx.unenumerable_reach);
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
        .any(|name| name.starts_with("::"))
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
        let config = tcl_lexer::LexerConfig::from_grammar(ctx.dialect.grammar);
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

fn record_source_target(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    args: IndirectArgs<'_>,
    source: &crate::command_binding::SourceInvocationBinding,
    target: &crate::command_binding::SourceCommandTarget,
) {
    if target.kind == crate::command_binding::BindingKind::Proc
        && ctx.known.contains(&target.command)
    {
        match args {
            IndirectArgs::Words(words) => out.record_source_call(
                target.command.clone(),
                &target.prepended,
                words.len(),
                &source.evaluated_argument_values,
                target.implementation_allocation.as_ref(),
            ),
            IndirectArgs::Unknowable => out.record_opaque_caller(&target.command),
        }
    } else if !target.registry_backed {
        out.record_unenumerable_caller(ctx.unenumerable_reach);
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
            out.record_unenumerable_caller(ctx.unenumerable_reach);
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
            out.record_unenumerable_caller(ctx.unenumerable_reach);
            return;
        }
    };
    let Some(source) = tokens.and_then(|tokens| tokens.source_binding.as_ref()) else {
        out.record_unenumerable_caller(ctx.unenumerable_reach);
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
            out.record_unenumerable_caller(ctx.unenumerable_reach);
        }
        if binding.may_be_absent {
            record_unresolved_word_dispatch(out, ctx, name, args);
        }
        for target in &binding.targets {
            if target.kind == crate::command_binding::BindingKind::Proc
                && ctx.known.contains(&target.command)
            {
                match args {
                    IndirectArgs::Words(words) => {
                        out.record_source_call(
                            target.command.clone(),
                            &target.prepended,
                            words.len(),
                            &source.evaluated_argument_values,
                            target.implementation_allocation.as_ref(),
                        );
                    }
                    IndirectArgs::Unknowable => out.record_opaque_caller(&target.command),
                }
            } else if !target.registry_backed {
                out.record_unenumerable_caller(ctx.unenumerable_reach);
            }
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
    if ctx
        .registry
        .get(word.strip_prefix("::").unwrap_or(word))
        .is_some()
    {
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
    // A call that *moves* a binding — `rename OLD NEW`, `interp alias {} NEW
    // {} TARGET` — makes every command name it touches one whose binding has
    // moved, in either direction.  A definition does not: `proc` binds a new
    // name without disturbing an existing one.  Which is which is registry
    // data, read through the one transition vocabulary (ledger C8).
    let tcl_registry::StateTransitionKnowledge::Declared(transitions) =
        &invocation.facts.state_transitions
    else {
        return;
    };
    if transitions.command_bindings().any(|transition| {
        !matches!(
            transition,
            tcl_registry::CommandBindingTransition::Define { .. }
        )
    }) {
        for word in args {
            let Some(word) = word.as_deref() else {
                out.record_unenumerable_caller(ctx.unenumerable_reach);
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
            if let Some(target) = row.target()
                .and_then(|target| target.original_ir_procedure_name(ctx.procedures))
                .filter(|target| ctx.known.contains(*target))
            { out.record_opaque_caller(target); }
            else if row.refusal() != Some(crate::command_binding::OriginalSourceCallbackProcedureRefusal::KnownSourceBarrier) {
                out.record_unenumerable_caller(ctx.unenumerable_reach);
            }
        }
    } else if ctx
        .metadata_context
        .and_then(|context| {
            crate::registry_invocation::original_callback_invocation_with_metadata_context(
                ctx.registry,
                context.into(),
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
        out.record_unenumerable_caller(ctx.unenumerable_reach);
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
    walk_module_scripts(ir_module, &mut |stmt| {
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
    /// Qualified-name context the body's bare command words resolve against.
    resolve_as: String,
    /// Synthetic, occurrence-unique variable-scope identity for its CFG.
    scope: String,
    /// The lowered body.
    body: &'a crate::ir::Script,
}

/// Every static-body `uplevel` in the module, paired with the namespace
/// context its body's bare command words resolve against.
///
/// `uplevel #0` (`absolute`, shift `0`) is the *absolute* frame form: its
/// body runs in the global frame, so bare command words resolve against the
/// global namespace, not the enclosing proc's — tclsh8.6/9.0-confirmed,
/// `uplevel #0 { helper b }` inside `::foo::runIt` calls `::helper`, never
/// `::foo::helper`.
///
/// Every other level keeps the enclosing unit's own namespace:
///
/// * `uplevel 0` is the *relative* current-frame form. Despite sharing the
///   magnitude `0` with `#0` it is the opposite case — tclsh8.6/9.0 confirm
///   `uplevel 0 { helper c }` inside `::foo::runIt` calls `::foo::helper`.
///   Resolving it against the enclosing unit is exact, not an approximation.
/// * Any other relative level (`uplevel 1`, `uplevel 2`, …) targets a frame
///   whose namespace depends on the live call stack, which single-file
///   static analysis cannot decide. Those keep the enclosing unit's
///   namespace as the documented, permanent approximation this scan has
///   always used for them.
/// * An absolute `#N` for `N > 0` names a frame counted down from the
///   global one, equally undecidable statically, so it takes the same
///   approximation.
fn upframe_scan_bodies(ir_module: &IrModule) -> Vec<UpFrameCaller<'_>> {
    let mut out = Vec::new();
    collect_upframes("::top", &ir_module.top_level, &mut out);
    for (qname, proc) in &ir_module.procedures {
        collect_upframes(qname, &proc.body, &mut out);
    }
    // A `TclOO` method body resolves bare words against the global namespace
    // (see `build_extra_call_site_scan_contexts`), so an `uplevel` inside one
    // inherits `"::top"` for the relative case too.
    for method in ir_module.methods.values() {
        collect_upframes("::top", &method.body, &mut out);
    }
    for (qname, unit) in &ir_module.body_units {
        collect_upframes(qname, &unit.body, &mut out);
    }
    out
}

/// The [`upframe_scan_bodies`] half that walks one enclosing unit's script.
fn collect_upframes<'a>(
    enclosing: &str,
    script: &'a crate::ir::Script,
    out: &mut Vec<UpFrameCaller<'a>>,
) {
    walk_script(script, &mut |stmt| {
        if let crate::ir::Statement::UpFrame {
            frame_shift,
            absolute,
            body,
            span,
            ..
        } = stmt
        {
            out.push(UpFrameCaller {
                resolve_as: if *absolute && *frame_shift == 0 {
                    "::top".to_owned()
                } else {
                    enclosing.to_owned()
                },
                scope: format!("{UPFRAME_SCOPE_PREFIX}{}", span.start()),
                body,
            });
        }
    });
}

/// One extra executable body walked as a call-site source.
pub(crate) struct ExtraCallSiteScanContext {
    /// Method/body-unit identity when this CFG is also consumed by the full
    /// per-unit lattice build. Upframe-only scan contexts leave this absent.
    analysis_unit: Option<String>,
    /// Qualified-name-shaped context used by `resolve_internal_call` for the
    /// exact fallback calls retained in this CFG.
    resolve_as: String,
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
    pub(crate) fn analysis_cfg(&self, qname: &str) -> Option<&CfgFunction> {
        (self.analysis_unit.as_deref() == Some(qname)).then_some(&self.cfg)
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
        .map(|(mqname, method)| {
            let execution_namespace = method.execution_namespace.clone();
            let requires_runtime_command_namespace = matches!(
                &execution_namespace,
                crate::ir::ExecutionNamespace::RuntimeSelected
            )
                && crate::ir_helpers::requires_runtime_command_namespace(&method.body, registry);
            let resolve_as = match &execution_namespace {
                // A qname ending in the method leaf gives
                // `resolve_internal_call` exactly this defining namespace.
                crate::ir::ExecutionNamespace::Exact(_)
                | crate::ir::ExecutionNamespace::SourceContext(_) => mqname.clone(),
                // Only absolute calls are trustworthy in a runtime-selected
                // method.  They ignore this fallback; relative calls cause the
                // whole reachable callee set to be poisoned below.
                crate::ir::ExecutionNamespace::RuntimeSelected => "::top".to_owned(),
            };
            let cfg = crate::cfg_builder::build_cfg_method_function_with_prepared_context(
                mqname,
                &method.body,
                execution_namespace.clone(),
                true,
                registry,
                ir_module.plain_command_dispatch,
                cfg_context,
                !dispatch_barrier.allows_locals(mqname),
                config,
            );
            ExtraCallSiteScanContext {
                analysis_unit: Some(mqname.clone()),
                resolve_as,
                cfg,
                execution_namespace,
                requires_runtime_command_namespace,
            }
        })
        .chain(
            ir_module
                .body_units
                .iter()
                .map(|(qname, unit)| ExtraCallSiteScanContext {
                    analysis_unit: Some(qname.clone()),
                    resolve_as: qname.clone(),
                    cfg: build(qname, &unit.body),
                    execution_namespace: crate::ir::ExecutionNamespace::exact(
                        tcl_syntax::naming::key_holder_and_tail(qname).0,
                    ),
                    requires_runtime_command_namespace: false,
                }),
        )
        .chain(upframes.into_iter().map(|up| {
            // Same shape as the method case: the caller context bare command
            // words resolve against is chosen by the frame the body runs in
            // (see [`upframe_scan_bodies`]), while the CFG keeps a distinct
            // identity of its own. Here that identity must be *synthetic* —
            // the CFG's name is this scan's variable-scope key, and reusing
            // the resolution context would overwrite that scope's real
            // variable facts in `collect_module_scope_var_facts`.
            let execution_namespace = crate::ir::ExecutionNamespace::exact(
                tcl_syntax::naming::key_holder_and_tail(&up.resolve_as).0,
            );
            ExtraCallSiteScanContext {
                analysis_unit: None,
                resolve_as: up.resolve_as,
                cfg: build(&up.scope, up.body),
                execution_namespace,
                requires_runtime_command_namespace: false,
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
/// Each call site is resolved to its callee via
/// [`crate::interprocedural::resolve_internal_call`] — Tcl's real,
/// existence-checked, namespace-relative resolution order, evaluated in the
/// *calling* function's own namespace, not the global one. This is the same
/// resolver the analyser and optimiser use for identical same-file call
/// resolution; a bespoke or partial resolver here could disagree with them
/// on which callee a bare name reaches.
///
/// The namespace context matters because a call site this scan fails to
/// resolve doesn't just go uncounted — it *vanishes* from
/// [`params_constants_from_native_call_sites`]'s "every caller passes the same
/// literal" evidence, which can flip an absence of contradicting evidence
/// into a false positive. A proc declared inside a `namespace eval` block
/// recurses into itself by its bare (unqualified) name; a resolver that only
/// tries global-qualified spellings of the command word
/// so it could never match the proc's namespaced qualified name, and the
/// recursive self-call — whose argument necessarily varies call to call —
/// was silently dropped. Only the one external, fully-qualified caller's
/// literal remained, so the loop/recursion-varying parameter was seeded as
/// that one constant and folded a genuinely alternating condition (`$count &
/// 1`) to a fixed boolean.
pub(crate) fn collect_call_site_constants(
    cfg_module: &CfgModule,
    extra_callers: &[ExtraCallSiteScanContext],
    procedures: &HashMap<String, crate::ir::Procedure>,
    future_call_sites: &[crate::command_binding::SourceFutureCallSite],
    source: CallSiteSourceContext<'_>,
) -> CallSiteEvidence {
    let CallSiteSourceContext {
        registry,
        declared,
        dialect,
        input,
        identities,
    } = source;
    let known: HashSet<String> = procedures.keys().cloned().collect();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, declared);
    let metadata_context =
        crate::registry_invocation::retained_source_metadata_context(registry, input);
    let var_facts = collect_module_scope_var_facts(
        cfg_module,
        extra_callers,
        &surface,
        metadata_context.as_deref(),
    );
    // Within one unit the reach is simply its own procedures, so an
    // unenumerable dispatch withdraws every seed the unit would have taken —
    // identical to the module-wide rule it replaces, but expressed per callee
    // so it merges and slices correctly across files.
    let reach = unenumerable_reach(procedures, Traits::empty(), &known);
    let unresolved_handler = unresolved_command_handler(registry, &known);
    // The top level has no qualified name of its own; `"::top"` (the same
    // pseudo-qname `FunctionUnit::build_full` uses for it) resolves to the
    // global namespace via `resolve_internal_call`'s "drop the last
    // segment" rule, matching a bare top-level call's real resolution
    // scope.
    let round = |previous: &CallSiteEvidence| {
        let ctx = CallSiteScanCtx {
            known: &known,
            registry,
            source_input: input,
            metadata_context: metadata_context.as_deref(),
            identities,
            declared,
            dialect,
            var_facts: &var_facts,
            previous,
            procedures,
            unenumerable_reach: &reach,
            unresolved_handler,
        };
        let funcs = std::iter::once(("::top", &cfg_module.top_level))
            .chain(cfg_module.procedures.iter().map(|(q, f)| (q.as_str(), f)))
            .chain(
                extra_callers
                    .iter()
                    .map(|caller| (caller.resolve_as.as_str(), &caller.cfg)),
            );
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
            out.record_unenumerable_caller(ctx.unenumerable_reach);
        }
        // A future caller withdraws assumptions about actual runtime targets.
        // Independent compiler uncertainty does not add a runtime callee.
        for target in &site.binding.targets {
            if target.kind == crate::command_binding::BindingKind::Proc
                && ctx.known.contains(&target.command)
            {
                out.record_opaque_caller(&target.command);
            } else if !target.registry_backed {
                out.record_unenumerable_caller(ctx.unenumerable_reach);
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
    declared: &HashMap<String, crate::ir::Procedure>,
    declared_linkage: Traits,
    project_wide: &HashSet<String, impl std::hash::BuildHasher>,
) -> Vec<String> {
    if declared_linkage.intersects(Traits::LOADS_EXTERNAL_UNIT) {
        let mut reach: Vec<String> = project_wide.iter().cloned().collect();
        reach.sort();
        return reach;
    }
    let mut reach: Vec<String> = declared.keys().cloned().collect();
    reach.sort();
    reach
}

/// Collect per-scope variable literal facts for every body the scan walks as
/// a caller — the input a dispatch word's value set is read from.
fn collect_module_scope_var_facts(
    cfg_module: &CfgModule,
    extra_callers: &[ExtraCallSiteScanContext],
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    context: Option<&tcl_registry::model::ContextRegistry>,
) -> ModuleVarFacts {
    let mut out = ModuleVarFacts::default();
    let funcs = std::iter::once(&cfg_module.top_level)
        .chain(cfg_module.procedures.values())
        .chain(extra_callers.iter().map(|caller| &caller.cfg));
    for func in funcs {
        let facts = collect_scope_var_facts(func, surface, context);
        out.cross_frame_write |= facts.cross_frame_write;
        out.scopes.insert(func.name.clone(), facts);
    }
    out
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
        out.record_unenumerable_caller(ctx.unenumerable_reach);
    }
}

/// Walk each `(caller qname, CFG)` pair's flattened statements, recording
/// every `Call`/`Barrier` as a call site.  Shared by the in-unit scan and
/// [`scan_source_call_sites`] so the two can never diverge on what counts.
fn scan_cfg_callers<'a>(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    funcs: impl Iterator<Item = (&'a str, &'a CfgFunction)>,
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
    for (resolve_as, func) in funcs {
        // The CFG function's own name is the *variable-scope* identity, which
        // differs from `resolve_as` for a `TclOO` method (lexical fallback
        // resolution, own frame).  A body the unit declares as a procedure is
        // one this evidence map attributes calls *to*; a method or `apply`
        // lambda is not, so its parameters may hold values never seen here.
        let declared = ctx.procedures.get(func.name.as_str());
        let caller = CallerFrame {
            _resolve_as: resolve_as,
            scope: func.name.as_str(),
            params: declared.map_or(&[][..], |p| p.params.as_slice()),
            callers_tracked: declared.is_some() || func.name == "::top",
        };
        let config = tcl_lexer::LexerConfig::from_grammar(ctx.dialect.grammar);
        for (&block_id, block) in &func.blocks {
            for stmt in &block.statements {
                // Synthetic analysis markers share the Call/Barrier shapes
                // but are never runtime invocations. They cannot supply
                // caller evidence for a user procedure with the same text.
                if !stmt.is_executable_invocation() {
                    continue;
                }
                if let Statement::Call { command, args, .. }
                | Statement::Barrier { command, args, .. } = stmt
                {
                    record_call_site_evidence(out, ctx, &caller, command, args, stmt.tokens(), 0);
                }
                // A command substitution nested in a word is a call site too.
                // Enumerating only the statements that *are* a command let
                // `[bump m]` in `set z [bump m]` go unattributed, so a single
                // visible `bump n` read as the complete caller set and O100
                // specialised the body to `upvar 1 n v`. Measured on tclsh
                // 9.0.4: `2 11 11` became `3 10 3` (#2134). An absence of
                // contradicting evidence must not read as agreement.
                //
                // Registry-aware because a brace-quoted *expression* word runs
                // its `[…]` too: `return [expr {[fact …]}]` hid a recursive
                // call from this scan, leaving the visible sites to read as the
                // complete caller set (#2118).
                for lifted in crate::word_subst::lifted_calls_with_surface(
                    statement_tokens(stmt),
                    config,
                    &ctx.surface(),
                ) {
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
            }
            record_terminator_call_sites(
                out,
                ctx,
                &caller,
                config,
                block.terminator.as_ref(),
                func.source_tokens_at(block_id, usize::MAX),
            );
        }
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

/// The lexed words a statement kept, for lifting the calls nested in them.
///
/// `None` for a statement that kept no token record — a fused `AssignExpr`,
/// `ExprEval` or `Incr` holds a parsed expression or an amount string instead.
/// Those are a known remaining gap rather than a claim that they run nothing.
fn statement_tokens(stmt: &Statement) -> Option<&crate::ir::CommandTokens> {
    match stmt {
        Statement::Call { tokens, .. }
        | Statement::Barrier { tokens, .. }
        | Statement::AssignValue { tokens, .. } => tokens.as_ref(),
        _ => None,
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
        lowerer = lowerer.with_context_registry(input.context_registry());
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
    let config = ir_module.lexer_config;
    // The cross-file scan resolves a dispatch word exactly as the in-unit one
    // does — `scan_cfg_callers`/`record_call_site_evidence` are shared, so a
    // `set cmd helper; $cmd dev` in *another* file retracts this unit's seed
    // just as an in-unit one would.  Without the same var facts and fixpoint
    // here, an unreadable dispatch would stop retracting seeds across the
    // file boundary.
    let metadata_context = crate::registry_invocation::retained_source_metadata_context(
        registry,
        ir_module.source_metadata_input.as_ref(),
    );
    let var_facts = collect_module_scope_var_facts(
        &cfg_module,
        &extra,
        &tcl_registry::model::DocumentCommandSurface::new(registry, declared),
        metadata_context.as_deref(),
    );
    // What this file's own unreadable dispatches can reach is a *project*
    // fact, not a file one — `source` puts two files in one interpreter in
    // both directions, so a library reaches its sourcer's procedures just as
    // its sourcer reaches the library's.  Only the host knows that graph, so
    // it supplies the set (`tcl_lsp_db::file_dispatch_reach`); with none
    // given, the file's own declarations are the honest bound.
    let reach: Vec<String> = if dispatch_reach.is_empty() {
        let mut own: Vec<String> = ir_module.procedures.keys().cloned().collect();
        own.sort();
        own
    } else {
        dispatch_reach.to_vec()
    };
    // `known` here is the *project*-wide procedure set, so a handler defined
    // in any scanned file is visible — matching Tcl, where `::unknown` is one
    // command shared by the whole interpreter, not a per-file one.
    let unresolved_handler = unresolved_command_handler(registry, known);
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        source,
        config,
        registry,
        &ir_module.source_entry,
    );
    let identities = ir_module.source_metadata_input.as_ref().map_or_else(
        || identities.clone(),
        |input| {
            identities
                .clone()
                .with_resolved_analysis_input(input.clone())
        },
    );
    out = run_to_fixpoint(|previous| {
        let ctx = CallSiteScanCtx {
            known,
            registry,
            source_input: ir_module.source_metadata_input.as_ref(),
            metadata_context: metadata_context.as_deref(),
            identities: &identities,
            declared,
            dialect,
            var_facts: &var_facts,
            previous,
            procedures: &ir_module.procedures,
            unenumerable_reach: &reach,
            unresolved_handler,
        };
        let funcs = std::iter::once(("::top", &cfg_module.top_level))
            .chain(cfg_module.procedures.iter().map(|(q, f)| (q.as_str(), f)))
            .chain(
                extra
                    .iter()
                    .map(|caller| (caller.resolve_as.as_str(), &caller.cfg)),
            );
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
    let Some(context) = crate::registry_invocation::retained_source_metadata_context(
        registry,
        ir_module.source_metadata_input.as_ref(),
    ) else {
        return Traits::empty();
    };
    let context = Some(context.as_ref().into());
    let mut found = Traits::empty();

    let mut visit = |stmt: &crate::ir::Statement| {
        if let Some(invocation) =
            crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
                registry, context, stmt,
            )
        {
            found |= invocation
                .facts
                .traits
                .intersection(tcl_registry::UNIT_LINKAGE_TRAITS);
        }
        // A possible publishing/loading boundary is enough to withdraw a
        // closed caller set. This union consumes metadata-only candidates;
        // it does not turn uncertain dispatch into executable handler facts.
        if let Some(assistance) = stmt.tokens().and_then(|tokens| {
            crate::registry_invocation::registry_invocation_assistance_with_metadata_context(
                registry, context, tokens,
            )
        }) {
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
    visit: &mut impl FnMut(&'a crate::ir::Statement),
) {
    walk_script(&ir_module.top_level, visit);
    for proc in ir_module.procedures.values() {
        walk_script(&proc.body, visit);
    }
    for method in ir_module.methods.values() {
        walk_script(&method.body, visit);
    }
    for unit in ir_module.body_units.values() {
        walk_script(&unit.body, visit);
    }
}

/// Visit every statement in `script`, descending into every nested
/// control-flow body.  Written as a module-level pair with
/// [`walk_statement`] rather than nested inside its callers so the
/// mutual recursion reads as two ordinary functions.
fn walk_script<'a>(
    script: &'a crate::ir::Script,
    visit: &mut impl FnMut(&'a crate::ir::Statement),
) {
    for stmt in &script.statements {
        walk_statement(stmt, visit);
    }
}

/// The [`walk_script`] half that dispatches one statement.
fn walk_statement<'a>(
    stmt: &'a crate::ir::Statement,
    visit: &mut impl FnMut(&'a crate::ir::Statement),
) {
    use crate::ir::Statement;
    visit(stmt);
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
    let callee = evidence.get(&procedure.qualified_name)?;
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
mod tests {
    use super::*;
    use crate::cfg_builder::build_cfg;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// The catalogue with no document declarations — what every fixture here
    /// analyses against unless it states otherwise.
    /// A `# tcl-lsp: stub myexpr {value:expr}` declares an expression word
    /// exactly as a shipped command does, and lowering honours that role — so
    /// the `[…]` inside `myexpr {[id 9]}` is a call site this scan must see.
    ///
    /// The walk read the bare catalogue, which knows no declared command at
    /// all, so the call was invisible and a lone visible `id 7` could seed
    /// `id`'s parameter as the constant `7` (#2118, found in review).
    #[test]
    fn a_declared_expression_role_reaches_the_call_site_walk() {
        use tcl_dialect::model::Provenance;
        use tcl_registry::model::{DeclaredArgument, DeclaredCommand, DeclaredSurface};

        let reg = CommandRegistry::build_default();
        let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {x} {\n myexpr {[id 9]}\n}",
            &reg,
            false,
        );
        let fu = cu.function("::f").expect("proc lowered");
        let tokens = fu.cfg.blocks.values().find_map(|block| {
            block.statements.iter().find_map(|stmt| match stmt {
                Statement::Call { tokens, .. } => tokens.as_ref(),
                _ => None,
            })
        });

        assert!(
            crate::word_subst::lifted_calls_with_surface(tokens, config, &surface(&reg)).is_empty(),
            "undeclared, `myexpr`'s braced word is ordinary literal text"
        );

        let mut declared = DeclaredSurface::new();
        declared.declare(DeclaredCommand::new(
            "myexpr".to_owned(),
            vec![DeclaredArgument {
                name: "value".to_owned(),
                role: tcl_registry::ArgRole::Expr,
                optional: false,
            }],
            Provenance::Document,
        ));
        let document = tcl_registry::model::DocumentCommandSurface::new(&reg, Some(&declared));
        assert_eq!(
            crate::word_subst::lifted_calls_with_surface(tokens, config, &document)
                .iter()
                .map(|lifted| (lifted.command.clone(), lifted.args.clone()))
                .collect::<Vec<_>>(),
            vec![("id".to_owned(), vec!["9".to_owned()])],
            "the declaration says the word is an expression, so its `[…]` runs"
        );
    }

    fn retained_scope_fixture(source: &str, registry: &CommandRegistry) -> IrModule {
        let profile = registry
            .profile()
            .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl);
        let context = std::sync::Arc::new(
            crate::environment_ingress::context_for_profile(profile)
                .with_command_store(registry.snapshot().shared_registry()),
        );
        crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(registry, tcl_lexer::LexerConfig::default())
                .with_dialect(Some(profile))
                .with_context_registry(context),
            source,
        )
    }

    fn surface(reg: &CommandRegistry) -> tcl_registry::model::DocumentCommandSurface<'_> {
        tcl_registry::model::DocumentCommandSurface::new(reg, None)
    }

    fn known(names: &[&str]) -> HashSet<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    fn params(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    fn seed(
        params: &[String],
        evidence: &CallSiteEvidence,
        qname: &str,
        linkage: Traits,
        has_cross_file_evidence: bool,
    ) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
        let proc_binding_trust = crate::command_binding::ProcBindingTrustProjection::default();
        params_constants_from_call_sites(
            params,
            evidence,
            qname,
            &UnitCallerView {
                linkage,
                has_cross_file_evidence,
                proc_binding_trust: &proc_binding_trust,
                source: None,
            },
        )
    }

    fn native_seed(
        params_raw: &str,
        grammar: Option<tcl_dialect::ParameterGrammar>,
        evidence: &CallSiteEvidence,
    ) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
        let procedure = crate::ir::Procedure {
            name: "helper".into(),
            qualified_name: "::helper".into(),
            params: Vec::new(),
            span: tcl_lexer::Span::new(0, 0),
            body: crate::ir::Script::default(),
            params_raw: params_raw.into(),
            body_source: None,
            body_offset: 0,
            namespace_scoped: false,
            base_priority: 500,
        };
        let trust = crate::command_binding::ProcBindingTrustProjection::default();
        params_constants_from_native_call_sites(
            &procedure,
            grammar,
            evidence,
            &UnitCallerView {
                linkage: Traits::empty(),
                has_cross_file_evidence: false,
                proc_binding_trust: &trust,
                source: None,
            },
        )
    }

    fn source_loader_entry(text: &str) -> crate::command_binding::SourceAnalysisEntry {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
        crate::command_binding::SourceAnalysisEntry {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            trusted_source_modules: vec![crate::command_binding::TrustedSourceModuleLoader::new(
                tcl_dialect::model::Family::Tcl,
                "library.tcl".into(),
                None,
                "workspace/library/revision-one".into(),
                &std::sync::Arc::from(text),
            )],
            ..crate::command_binding::SourceAnalysisEntry::default()
        }
    }

    fn scan_loaded_caller(
        source: &str,
        entry: &crate::command_binding::SourceAnalysisEntry,
    ) -> CallSiteEvidence {
        let registry = CommandRegistry::build_default();
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
        let known = HashSet::from(["::helper".to_owned()]);
        let context = std::sync::Arc::new(
            crate::environment_ingress::context_for_profile(profile)
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );
        scan_source_call_sites_with_source_input(source, None, &known, &[], &input, entry)
    }

    #[test]
    fn trusted_file_entry_retains_actual_loaded_declaration_identity() {
        let library = "proc helper {mode} {return $mode}";
        let entry = source_loader_entry(library);
        let evidence = scan_loaded_caller("source library.tcl; helper dev", &entry);
        let helper = evidence.get("::helper").expect("actual sourced caller");
        assert_eq!(helper.uniform_literal_at(0), Some("dev"));
        assert_eq!(helper.source_implementations.len(), 1);
        let allocation = &helper.source_implementations[0];
        assert!(allocation.matches_source_declaration(library, 0));
        assert!(!allocation.matches_source_declaration("proc helper {mode} {return WRONG}", 0));
        assert!(
            matches!(allocation.site.source.kind(), crate::command_binding::SourceOriginKind::Loaded {path, ..} if path.as_ref() == "library.tcl")
        );
    }

    #[test]
    fn file_entry_does_not_license_unloaded_shadowed_or_unknown_file_reads() {
        let entry = source_loader_entry("proc helper {mode} {return $mode}");
        for source in [
            "helper dev",
            "source $path; helper dev",
            "source other.tcl; helper dev",
            "proc source {path} {}; source library.tcl; helper dev",
        ] {
            let evidence = scan_loaded_caller(source, &entry);
            assert!(
                evidence
                    .get("::helper")
                    .is_none_or(|callee| callee.uniform_literal_at(0).is_none()),
                "{source}"
            );
        }
        let absent = crate::command_binding::SourceAnalysisEntry {
            trusted_source_modules: Vec::new(),
            ..entry
        };
        assert!(
            scan_loaded_caller("source library.tcl; helper dev", &absent)
                .get("::helper")
                .is_none_or(|callee| callee.uniform_literal_at(0).is_none())
        );
    }

    #[test]
    fn sourced_partial_errors_and_redefinition_preserve_real_target_identity() {
        let partial =
            "proc helper {mode} {return $mode}; error BOOM; proc helper {mode} {return WRONG}";
        let entry = source_loader_entry(partial);
        let evidence = scan_loaded_caller("catch {source library.tcl}; helper dev", &entry);
        let helper = evidence
            .get("::helper")
            .expect("definition preceding error survives");
        assert_eq!(helper.uniform_literal_at(0), Some("dev"));
        assert!(
            helper
                .source_implementations
                .iter()
                .all(|allocation| allocation.matches_source_declaration(partial, 0))
        );
        let replaced = scan_loaded_caller(
            "source library.tcl; proc helper {mode} {return NEW}; helper dev",
            &source_loader_entry("proc helper {mode} {return OLD}"),
        );
        assert!(
            replaced
                .get("::helper")
                .expect("replacement caller")
                .source_implementations
                .iter()
                .all(|allocation| !allocation
                    .matches_source_declaration("proc helper {mode} {return OLD}", 0))
        );
    }

    #[test]
    fn native_formal_seed_distinguishes_zero_arguments_from_an_opaque_caller() {
        let mut evidence = CallSiteEvidence::default();
        evidence.record_call("::helper".into(), &[]);
        let grammar = Some(tcl_dialect::ParameterGrammar::Tcl);
        let seed = native_seed("{mode prod}", grammar, &evidence).expect("default binding");
        assert_eq!(
            seed.get(&("mode".into(), 0)),
            Some(&crate::analyses::LatticeValue::Const(
                crate::analyses::ConstValue::String("prod".into())
            ))
        );
        evidence.record_opaque_caller("::helper");
        assert!(native_seed("{mode prod}", grammar, &evidence).is_none());
    }

    #[test]
    fn native_formal_seed_requires_a_proved_ordinary_activation_layout() {
        let mut evidence = CallSiteEvidence::default();
        evidence.record_call("::helper".into(), &["same".into(), "same".into()]);
        assert!(
            native_seed(
                "mode mode",
                Some(tcl_dialect::ParameterGrammar::Tcl),
                &evidence
            )
            .is_none()
        );
        for params in ["&mode other", "args mode", "{args rest} mode", "mode other"] {
            assert!(
                native_seed(params, Some(tcl_dialect::ParameterGrammar::Jim), &evidence).is_none()
            );
        }
        assert!(native_seed("mode other", None, &evidence).is_none());
        assert!(
            native_seed(
                "mode other",
                Some(tcl_dialect::ParameterGrammar::Tcl),
                &evidence
            )
            .is_some()
        );
    }

    #[test]
    fn finite_dispatch_evidence_converges_beyond_six_dependencies() {
        let evidence = run_to_fixpoint(|previous| {
            let mut next = CallSiteEvidence {
                consulted_value_sets: true,
                ..Default::default()
            };
            next.record_call("::entry".to_owned(), &["literal".to_owned()]);
            for index in 0..9 {
                let predecessor = if index == 0 {
                    "::entry".to_owned()
                } else {
                    format!("::dispatch{}", index - 1)
                };
                if let Some(value) = previous
                    .get(&predecessor)
                    .and_then(|callee| callee.uniform_literal_at(0))
                {
                    next.record_call(format!("::dispatch{index}"), &[value.to_owned()]);
                }
            }
            next
        });
        assert_eq!(
            evidence.get("::dispatch8").unwrap().uniform_literal_at(0),
            Some("literal")
        );
    }

    /// A call that omits a defaulted parameter binds it to its **default**, an
    /// unknown value — so a literal at another call site cannot claim the
    /// position. `binds_position` is what encodes that.
    #[test]
    fn an_omitted_argument_stops_a_position_being_uniform() {
        let reg = registry();
        let evidence = scan_source_call_sites(
            "proc helper {a {b x}} { return $a }\nhelper 1 two\nhelper 1\n",
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
            &known(&["::helper"]),
            &[],
        );
        let helper = evidence.get("::helper").expect("calls recorded");
        assert_eq!(helper.uniform_literal_at(0), Some("1"));
        assert_eq!(helper.arg_counts, [1, 2].into_iter().collect());
        assert!(!helper.binds_position(1));
        assert_eq!(helper.uniform_literal_at(1), None);
    }

    /// Merging is monotone: extra evidence widens the value set and the
    /// observed argument counts, so a second file can retract a fold but
    /// never manufacture one.
    #[test]
    fn merging_only_ever_widens() {
        let reg = registry();
        let src = "proc helper {mode} { return $mode }\n";
        let mut a = scan_source_call_sites(
            &format!("{src}helper prod\n"),
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
            &known(&["::helper"]),
            &[],
        );
        assert_eq!(
            a.get("::helper").unwrap().uniform_literal_at(0),
            Some("prod")
        );
        let b = scan_source_call_sites(
            &format!("{src}helper dev\n"),
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
            &known(&["::helper"]),
            &[],
        );
        a.merge_from(&b);
        assert_eq!(a.get("::helper").unwrap().uniform_literal_at(0), None);
    }

    /// A `TclOO` method's relative command resolves in the receiver object's
    /// namespace.  The selected implementation may itself call any procedure
    /// reachable from this source with arguments the lexical CFG never sees,
    /// so cross-file evidence must withdraw an otherwise-uniform literal.
    #[test]
    fn runtime_selected_method_command_poisons_cross_file_param_evidence() {
        let reg = registry();
        let evidence = scan_source_call_sites(
            "::helper prod\noo::class create C { method run {} { hook } }\n",
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
            &known(&["::helper"]),
            &["::helper".to_owned()],
        );
        let helper = evidence.get("::helper").expect("direct call recorded");
        assert_eq!(
            helper.uniform_literal_at(0),
            None,
            "an object-local hook can call helper with arbitrary arguments: {evidence:?}",
        );
    }

    /// Absolute method commands do not depend on the receiver namespace and
    /// therefore must not discard unrelated exact call-site evidence.
    #[test]
    fn absolute_only_method_keeps_cross_file_param_evidence_exact() {
        let reg = registry();
        let evidence = scan_source_call_sites(
            "proc ::helper {mode} {return $mode}\n::helper prod\noo::class create C { method run {} { ::puts ok } }\n",
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
            &known(&["::helper"]),
            &["::helper".to_owned()],
        );
        let helper = evidence.get("::helper").expect("direct call recorded");
        assert_eq!(helper.uniform_literal_at(0), Some("prod"));
    }

    #[test]
    fn known_external_name_does_not_prove_a_runtime_procedure_binding() {
        let reg = registry();
        let evidence = scan_source_call_sites(
            "helper prod\nhelper prod\n",
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &known(&["::helper"]),
            &["::helper".to_owned()],
        );
        assert!(
            evidence
                .get("::helper")
                .is_none_or(|helper| helper.uniform_literal_at(0).is_none())
        );
    }

    /// A deferred command prefix (`ArgRole::CommandPrefix`) invokes the proc
    /// with runtime-supplied words appended, so no position stays uniform.
    /// The callback slot comes from the registry, not a command-name list.
    #[test]
    fn a_command_prefix_callback_is_recorded_as_an_opaque_caller() {
        let reg = registry();
        let evidence = scan_source_call_sites(
            "proc helper {mode} {return $mode}\nhelper prod\nafter 0 helper\n",
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
            &known(&["::helper"]),
            &[],
        );
        let helper = evidence.get("::helper").expect("calls recorded");
        assert!(helper.arg_counts.contains(&0), "{helper:?}");
        assert_eq!(helper.uniform_literal_at(0), None);
    }

    /// A `rename` / `interp alias` naming a known command moves its binding,
    /// so a call can reach it under a name no scan attributed to it.
    #[test]
    fn a_rebinding_call_is_recorded_as_an_opaque_caller() {
        let reg = registry();
        for src in [
            "proc helper {mode} {return $mode}\nhelper prod\nrename helper legacy\n",
            "proc helper {mode} {return $mode}\nhelper prod\ninterp alias {} h {} helper\n",
        ] {
            let evidence = scan_source_call_sites(
                src,
                &reg,
                None,
                tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
                &known(&["::helper"]),
                &[],
            );
            let helper = evidence.get("::helper").expect("calls recorded");
            assert_eq!(helper.uniform_literal_at(0), None, "{src}");
        }
    }

    /// Only a **trailing** `args` is Tcl's variadic catch-all
    /// (`TclCreateProc`, `generic/tclProc.c`) — in `proc f {args x}` the
    /// first word is an ordinary parameter and both positions may be seeded.
    #[test]
    fn only_a_trailing_args_stops_the_seed() {
        let reg = registry();
        let evidence = scan_source_call_sites(
            "proc helper {args x} {}\nhelper one two\nhelper one two\n",
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
            &known(&["::helper"]),
            &[],
        );
        let leading = seed(
            &params(&["args", "x"]),
            &evidence,
            "::helper",
            Traits::empty(),
            false,
        )
        .expect("both positions seeded");
        assert_eq!(leading.len(), 2);
        let trailing = seed(
            &params(&["x", "args"]),
            &evidence,
            "::helper",
            Traits::empty(),
            false,
        )
        .expect("the leading parameter is still seeded");
        assert_eq!(trailing.len(), 1);
        assert!(trailing.contains_key(&("x".to_owned(), 0)));
    }

    /// A registry-declared boundary declines the seed on its own, and a
    /// host-supplied cross-file view re-enables it — but only for the kind of
    /// boundary that view can actually bound.
    ///
    /// Publishing this file's commands as an API surface (`PROVIDES_PACKAGE`
    /// / `EXPORTS_COMMAND`) admits callers no project scan covers — another
    /// checkout can `package require` this one — so it declines outright.
    /// Pulling another unit in (`LOADS_EXTERNAL_UNIT`) names a caller the
    /// host's project normally *does* contain, so it defers to the evidence.
    #[test]
    fn boundary_gating_distinguishes_publishing_from_loading() {
        let reg = registry();
        let evidence = scan_source_call_sites(
            "proc helper {mode} {}\nhelper prod\nhelper prod\n",
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
            &known(&["::helper"]),
            &[],
        );
        let p = params(&["mode"]);
        assert!(
            seed(&p, &evidence, "::helper", Traits::empty(), false).is_some(),
            "no boundary at all: the visible callers are the whole story"
        );
        for boundary in [Traits::PROVIDES_PACKAGE, Traits::EXPORTS_COMMAND] {
            for has_view in [false, true] {
                assert!(
                    seed(&p, &evidence, "::helper", boundary, has_view).is_none(),
                    "{boundary:?} publishes to callers no enumeration bounds \
                     (cross-file view: {has_view})"
                );
            }
        }
        assert!(
            seed(
                &p,
                &evidence,
                "::helper",
                Traits::LOADS_EXTERNAL_UNIT,
                false
            )
            .is_none(),
            "a loaded unit is an unenumerated caller without a host view"
        );
        assert!(
            seed(&p, &evidence, "::helper", Traits::LOADS_EXTERNAL_UNIT, true).is_some(),
            "with the project enumerated, the loaded unit's callers are in the evidence"
        );
    }

    /// `scan_unit_linkage` reads the registry, so it recognises a boundary
    /// however it is spelled — and never fires on a mere mention.
    #[test]
    fn unit_linkage_is_scanned_from_the_lowered_ir_not_the_raw_text() {
        let reg = registry();
        let cases = [
            ("package provide mylib 1.0\n", Traits::PROVIDES_PACKAGE),
            ("::package\tprovide mylib 1.0\n", Traits::PROVIDES_PACKAGE),
            ("if {1} { source other.tcl }\n", Traits::LOADS_EXTERNAL_UNIT),
            (
                "namespace eval ::a { namespace export * }\n",
                Traits::EXPORTS_COMMAND,
            ),
            (
                "# this file does not package provide anything\n",
                Traits::empty(),
            ),
            ("set msg \"package provide\"\n", Traits::empty()),
            ("namespace import ::lib::helper\n", Traits::empty()),
        ];
        for (src, want) in cases {
            let module = retained_scope_fixture(src, &reg);
            assert_eq!(scan_unit_linkage(&module, &reg, None), want, "{src:?}");
        }
    }

    /// A `namespace eval ::a { helper }` body calls `::a::helper`, never a
    /// same-named proc in the *caller's* namespace (tclsh8.6-confirmed).
    ///
    /// The body is scanned once, as the properly-namespaced body unit lowering
    /// registers for it. Walking it a second time through the enclosing
    /// statement's `ArgRole::Body` — with the caller's namespace — invents a
    /// call to whatever `::helper` happens to exist. Within one file that is
    /// invisible (a bare global `::helper` is rarely in a single file's own
    /// `known` set); across a project it is a false edge into *another file's*
    /// procedure.
    #[test]
    fn a_namespace_eval_body_resolves_against_its_own_namespace() {
        let reg = registry();
        let src = "namespace eval ::a {\n    proc helper {} { return 1 }\n    proc run {} { helper }\n}\n";
        // `::helper` is in scope project-wide, as a global proc in some other
        // file would be — the call inside `::a` must not reach it.
        let evidence = scan_source_call_sites(
            src,
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &known(&["::a::helper", "::a::run", "::helper"]),
            &[],
        );
        assert_eq!(
            evidence.callees().collect::<Vec<_>>(),
            vec!["::a::helper"],
            "the call binds to ::a::helper only"
        );
        // The in-unit scan is the reference: cross-file must not see more.
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
        assert_eq!(
            cu.caller_scope.call_sites.callees().collect::<Vec<_>>(),
            vec!["::a::helper"],
        );
    }

    /// A command substitution nested in a word is a call site (#2134).
    ///
    /// Enumerating only the statements that *are* a command let `[bump m]`
    /// go unattributed, so the single visible `bump n` read as the complete
    /// caller set and O100 specialised the body to `upvar 1 n v`. Measured on
    /// tclsh 9.0.4, the program printed `2 11 11` and the rewritten one
    /// `3 10 3`.
    #[test]
    fn a_nested_substitution_counts_as_a_caller() {
        let reg = registry();
        let src = "proc bump {name} {\n  upvar 1 $name v\n  incr v\n}\nset n 1\nset m 10\nbump n\nset z [bump m]\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
        assert!(
            cu.caller_scope
                .call_sites
                .callees()
                .any(|callee| callee == "::bump"),
            "the nested `[bump m]` must be attributed to ::bump",
        );
        // The two callers disagree on the argument, so nothing may be seeded.
        assert_eq!(
            cu.caller_scope
                .call_sites
                .get("::bump")
                .and_then(|evidence| evidence.uniform_literal_at(0)),
            None,
            "disagreeing callers must not look uniform",
        );
    }

    /// Callers that agree still seed — the nested site is *counted*, not
    /// treated as opaque.
    #[test]
    fn agreeing_callers_still_seed_through_a_substitution() {
        let reg = registry();
        let src =
            "proc bump {name} {\n  upvar 1 $name v\n  incr v\n}\nset n 1\nbump n\nset z [bump n]\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
        assert_eq!(
            cu.caller_scope
                .call_sites
                .get("::bump")
                .and_then(|evidence| evidence.uniform_literal_at(0)),
            Some("n"),
            "callers that agree on `n` must still seed",
        );
    }

    /// An `uplevel` body nested inside another `ArgRole::Body` must not be
    /// re-walked with the enclosing unit's namespace.
    ///
    /// The `catch` body is walked correctly — `catch` shifts nothing — and
    /// the `uplevel #0` body inside it was then walked again as `::foo`,
    /// inventing a call to `::foo::helper` alongside the correct `::helper`
    /// that `upframe_scan_bodies` had already recorded with the right
    /// frame. Both the phantom callee and the double attribution are wrong:
    /// tclsh8.6/9.0 confirm `uplevel #0 { helper b }` inside `::foo::runIt`
    /// calls `::helper` and nothing else.
    ///
    /// The `Traits::EVALUATES_IN_SHIFTED_FRAME` skip is what stops it, so
    /// no command name appears in the walker.
    #[test]
    fn an_uplevel_body_inside_a_catch_body_is_not_reattributed() {
        let reg = registry();
        let src = "proc ::helper {mode} {return $mode}\nnamespace eval ::foo {\n    proc helper {mode} { return $mode }\n    proc runIt {} { catch { uplevel #0 { helper b } } }\n}\n";
        let evidence = scan_source_call_sites(
            src,
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &known(&["::foo::helper", "::foo::runIt", "::helper"]),
            &[],
        );
        let mut callees: Vec<&str> = evidence.callees().collect();
        callees.sort_unstable();
        assert_eq!(
            callees,
            vec!["::helper"],
            "the global helper takes the evidence and ::foo::helper gets no phantom call",
        );
    }

    /// `catch { … }` does *not* shift namespace, so its body must still be
    /// walked with the caller's — the guard above keys on an absolute
    /// `ArgRole::Name`, which `catch` has none of.
    #[test]
    fn a_catch_body_is_still_walked_with_the_callers_namespace() {
        let reg = registry();
        let evidence = scan_source_call_sites(
            "proc helper {mode} { return $mode }\ncatch { helper prod }\n",
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &known(&["::helper"]),
            &[],
        );
        assert_eq!(
            evidence
                .get("::helper")
                .and_then(|e| e.uniform_literal_at(0)),
            Some("prod"),
            "the call inside catch is still evidence"
        );
    }

    /// `slice_for` is driven by the *callee* names a file declares, so its
    /// cost is that file's procedure count rather than the project's.
    #[test]
    fn slice_for_keeps_only_the_named_callees() {
        let reg = registry();
        let evidence = scan_source_call_sites(
            "proc a {value} {}\nproc b {value} {}\na 1\nb 2\n",
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
            &known(&["::a", "::b"]),
            &[],
        );
        let sliced = evidence.slice_for(["::a"].into_iter());
        assert_eq!(sliced.callees().collect::<Vec<_>>(), vec!["::a"]);
        assert!(evidence.slice_for(["::zzz"].into_iter()).is_empty());
    }

    /// Build the evidence for `src` under a named dialect's registry, the
    /// way `CompilationUnit` does minus the method / body-unit callers
    /// (which need a CFG context these unit tests do not exercise).
    fn evidence_for_dialect(
        src: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> CallSiteEvidence {
        let reg = tcl_registry::model::ingress::static_context_for_profile(dialect).commands();
        let ir = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(
                reg,
                tcl_lexer::LexerConfig::from_grammar(dialect.grammar),
            )
            .with_dialect(Some(dialect))
            .with_context_registry(crate::environment_ingress::context_for_profile(dialect)),
            src,
        );
        let cfg_module = crate::cfg_builder::build_cfg(&ir, false);
        let identities = crate::realm::document_realm_bindings_with_source_entry(
            src,
            ir.lexer_config,
            reg,
            &ir.source_entry,
        );
        let identities = ir.source_metadata_input.as_ref().map_or_else(
            || identities.clone(),
            |input| {
                identities
                    .clone()
                    .with_resolved_analysis_input(input.clone())
            },
        );
        collect_call_site_constants(
            &cfg_module,
            &[],
            &ir.procedures,
            &ir.future_call_sites,
            CallSiteSourceContext {
                registry: reg,
                declared: None,
                dialect,
                input: ir.source_metadata_input.as_ref(),
                identities: &identities,
            },
        )
    }

    fn evidence(src: &str) -> CallSiteEvidence {
        evidence_for_dialect(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        )
    }

    #[test]
    fn deferred_global_callers_retract_only_their_retained_possible_targets() {
        let evidence = evidence(
            "proc helper {x} {return $x}\nproc other {x} {return $x}\nhelper prod\nother prod\nafter 0 {other $::runtime_value}",
        );
        assert_eq!(
            uniform(&evidence, "::helper", 0).as_deref(),
            Some("prod"),
            "{evidence:#?}"
        );
        assert!(
            evidence
                .get("::other")
                .is_some_and(|site| site.opaque_caller)
        );
        let unknown = self::evidence(
            "proc helper {x} {return $x}\nhelper prod\nafter idle {$::runtime_command $::runtime_value}",
        );
        assert!(
            unknown
                .get("::helper")
                .is_some_and(|site| site.opaque_caller)
        );
    }

    /// Whether `index` still has a single agreed literal across every
    /// recorded caller — the exact predicate the seed reads.  This model
    /// records "a caller that never reached this position" in `arg_counts`
    /// rather than in the slot, so probing the slot alone would miss it.
    fn uniform(ev: &CallSiteEvidence, qname: &str, index: usize) -> Option<String> {
        ev.get(qname)?
            .uniform_literal_at(index)
            .map(ToOwned::to_owned)
    }

    /// The literal values recorded at `index` for `qname`, sorted, plus
    /// whether that position is poisoned.
    fn slot(ev: &CallSiteEvidence, qname: &str, index: usize) -> (Vec<String>, bool) {
        let Some(consts) = ev.get(qname).and_then(|e| e.slots.get(&index)) else {
            return (Vec::new(), false);
        };
        let mut values: Vec<String> = consts.values.iter().cloned().collect();
        values.sort();
        (values, consts.unknown)
    }

    #[test]
    fn literal_call_sites_are_recorded_and_nothing_is_withdrawn() {
        let ev = evidence("proc helper {mode} { return $mode }\nhelper a\nhelper b\n");
        assert_eq!(
            slot(&ev, "::helper", 0),
            (vec!["a".into(), "b".into()], false)
        );
        // Two disagreeing literals, so no uniform value — but the position is
        // *bound* by every recorded call, which a withdrawal would undo.
        assert!(ev.get("::helper").unwrap().binds_position(0));
    }

    #[test]
    fn registry_barrier_marker_is_not_a_zero_argument_caller() {
        let ev = evidence(
            "proc {<registry-barrier>} {mode} { return $mode }\n\
             {<registry-barrier>} fixed\n\
             set ignored [missing_command]\n",
        );
        assert_eq!(
            uniform(&ev, "::<registry-barrier>", 0),
            Some("fixed".into()),
            "the synthetic marker is analysis-only; the real invocation remains evidence",
        );
    }

    // A module's own `unknown` handler. Tcl dispatches every
    // unresolved command word to it, so its direct callers are never its
    // complete caller set. tclsh8.6/9.0-confirmed: with `proc unknown {cmd
    // args}` in scope, `bogus beta gamma` runs the handler with
    // `cmd` = `bogus`, `args` = `beta gamma`.

    #[test]
    fn an_unresolved_word_is_a_call_site_of_the_modules_unknown_handler_1044() {
        // TP: `bogus` names nothing, so real Tcl calls the handler with
        // `bogus`. Seeing only the two `unknown alpha`
        // calls, the scan would bind `cmd` to the constant `"alpha"` and fold
        // `$cmd eq "alpha"` on a genuinely runtime-varying condition.
        let ev = evidence(
            "proc unknown {cmd args} { if {$cmd eq \"alpha\"} { return 1 } else { return 2 } }\nunknown alpha\nunknown alpha\nbogus beta\n",
        );
        assert_eq!(
            slot(&ev, "::unknown", 0),
            (vec!["alpha".into(), "bogus".into()], false),
            "the unresolved word itself is the handler's first argument",
        );
        assert_eq!(uniform(&ev, "::unknown", 0), None, "must not fold");
    }

    #[test]
    fn the_unresolved_words_own_arguments_follow_it_into_the_handler_1044() {
        // TP: Tcl passes the failed call's arguments after the word, so
        // `args`' positions carry them — evidence the scan really can see,
        // recorded in full rather than merely poisoned.
        let ev = evidence("proc unknown {cmd args} { return $cmd }\nbogus beta gamma\n");
        assert_eq!(slot(&ev, "::unknown", 0), (vec!["bogus".into()], false));
        assert_eq!(slot(&ev, "::unknown", 1), (vec!["beta".into()], false));
        assert_eq!(slot(&ev, "::unknown", 2), (vec!["gamma".into()], false));
    }

    #[test]
    fn a_module_with_no_unknown_handler_is_unaffected_1044() {
        // TN, the common case and the whole regression risk: an unresolved
        // word in a module that defines no handler must change nothing.
        let ev = evidence("proc helper {mode} { return $mode }\nhelper a\nbogus beta\n");
        assert_eq!(slot(&ev, "::helper", 0), (vec!["a".into()], false));
        assert_eq!(uniform(&ev, "::helper", 0).as_deref(), Some("a"));
        assert!(
            ev.get("::unknown").is_none(),
            "no handler defined, so nothing may be attributed to one: {ev:?}",
        );
    }

    #[test]
    fn a_handler_never_seeds_even_when_every_visible_caller_agrees_1044() {
        // The handler's caller set is unenumerable *by construction*, so
        // agreement among the callers a scan can see proves nothing.
        //
        // "With no unresolved word in the file the direct callers really are
        // all of them" is false: real Tcl routes to the handler
        // every word that resolves to nothing at the instant of the call:
        // an auto-loaded name, a name another sourced file introduces, a
        // name built by string arithmetic, a name typed at a prompt. None of
        // those appear in the source for any scan to find.
        //
        // On tclsh8.6, the seeded words are
        // wrong in *both* directions. `Dog new` after `oo::class create Dog`
        // and `worker` after `coroutine worker body` are recorded as
        // dispatches, yet neither ever reaches the handler. And a `bogus
        // beta` written *before* `proc unknown` is handled by the builtin
        // `::unknown` — it errors — so it is not a call site of this
        // handler either.
        let ev = evidence(
            "proc unknown {cmd args} { return $cmd }\nunknown alpha\nunknown alpha\nputs hi\n",
        );
        assert_eq!(
            uniform(&ev, "::unknown", 0),
            None,
            "defining the handler is itself the unenumerable caller: {ev:?}",
        );
    }

    #[test]
    fn a_class_command_never_seeds_the_handler_1044() {
        // On tclsh8.6, `oo::class create Dog` binds `Dog`, so `Dog
        // new` dispatches to the class command and the handler is never
        // called. The scan cannot resolve `Dog` and records it as an
        // unresolved-word dispatch anyway; the unconditional poison is what
        // stops that invented evidence becoming a fold.
        let ev = evidence(
            "proc unknown {cmd args} { if {$cmd eq \"Dog\"} { return 1 } else { return 2 } }\noo::class create Dog {\n    method bark {} { return woof }\n}\nDog new\n",
        );
        assert_eq!(
            uniform(&ev, "::unknown", 0),
            None,
            "a class command is not a handler call site: {ev:?}",
        );
    }

    #[test]
    fn a_coroutine_command_never_seeds_the_handler_1044() {
        // On tclsh8.6, `coroutine worker body` binds `worker`, so
        // calling it resumes the coroutine and the handler is never called.
        let ev = evidence(
            "proc unknown {cmd args} { if {$cmd eq \"worker\"} { return 1 } else { return 2 } }\nproc body {} { yield ; return done }\ncoroutine worker body\nworker\n",
        );
        assert_eq!(
            uniform(&ev, "::unknown", 0),
            None,
            "a coroutine command is not a handler call site: {ev:?}",
        );
    }

    #[test]
    fn a_word_written_before_the_handler_never_seeds_it_1044() {
        // On tclsh8.6, `bogus beta` on line 1, with `proc unknown`
        // defined only afterwards, is handled by the *builtin* `::unknown`
        // and errors with `invalid command name "bogus"`. The scan is
        // definition-order-insensitive, so it records the dispatch anyway.
        let ev = evidence(
            "bogus beta\nproc unknown {cmd args} { if {$cmd eq \"bogus\"} { return 1 } else { return 2 } }\n",
        );
        assert_eq!(
            uniform(&ev, "::unknown", 0),
            None,
            "an order-insensitive scan may not seed an order-sensitive dispatch: {ev:?}",
        );
    }

    #[test]
    fn a_registry_builtin_is_not_an_unresolved_word_1044() {
        // FP guard: a builtin resolves to no *user proc*, but it is not an
        // unresolved word — Tcl never routes `puts`/`set` to the handler.
        let ev = evidence(
            "proc unknown {cmd args} { return $cmd }\nunknown alpha\nunknown alpha\nputs hi\nset x 1\nincr x\n",
        );
        assert_eq!(
            slot(&ev, "::unknown", 0),
            (vec!["alpha".into()], false),
            "builtins must not appear as handler arguments: {ev:?}",
        );
    }

    #[test]
    fn an_unenumerable_dispatch_word_still_poisons_rather_than_naming_the_handler_1044() {
        // FN guard: a dynamic word whose value set is unreadable is not an
        // *unresolved* word — the scan cannot say it resolves to nothing —
        // so it keeps withdrawing every seed, the handler's included.
        let ev = evidence(
            "proc unknown {cmd args} { return $cmd }\nunknown alpha\nunknown alpha\nset c [gets stdin]\n$c beta\n",
        );
        assert_eq!(
            uniform(&ev, "::unknown", 0),
            None,
            "an unreadable dispatch may name anything, handler included: {ev:?}",
        );
    }

    #[test]
    fn a_cross_file_dispatch_never_seeds_another_files_handler_1044() {
        // The cross-file scan resolves against the *project's* names, so a
        // file that defines no handler still attributes its unresolved words
        // to one another file defines. That path needs the same poison, or
        // the miscompile simply moves across the file boundary.
        //
        // Shape: `h.tcl` holds `proc unknown`, `c.tcl` holds
        // `oo::class create Dog` + `Dog new`. The `Dog` dispatch is invented
        // (tclsh8.6: the class command answers, the handler is never called),
        // and it would be the handler's only recorded call site.
        let reg = registry();
        let evidence = scan_source_call_sites(
            "oo::class create Dog { method bark {} { return woof } }\nDog new\n",
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
            &known(&["::unknown"]),
            &[],
        );
        assert_eq!(
            evidence
                .get("::unknown")
                .and_then(|e| e.uniform_literal_at(0)),
            None,
            "a sibling file's dispatch may not seed the project's handler: {evidence:?}",
        );
    }

    #[test]
    fn only_one_command_carries_the_unresolved_handler_trait() {
        // `unresolved_command_handler` takes the first match from a hash-map
        // walk, so more than one carrier would make the answer depend on
        // iteration order. Sorting makes it deterministic; this pins the
        // stronger property that there is nothing to choose between.
        let reg = registry();
        let carriers = reg.commands_with_trait(Traits::UNRESOLVED_COMMAND_HANDLER);
        assert_eq!(
            carriers.len(),
            1,
            "a second carrier needs a resolution rule, not an arbitrary pick: {carriers:?}",
        );
    }

    #[test]
    fn a_dispatch_through_a_literal_variable_is_recorded_as_a_call_site() {
        let ev = evidence("proc helper {mode} { return $mode }\nset cmd helper\n$cmd dev\n");
        assert_eq!(slot(&ev, "::helper", 0), (vec!["dev".into()], false));
        assert_eq!(
            uniform(&ev, "::helper", 0).as_deref(),
            Some("dev"),
            "an enumerable dispatch is an ordinary call site, not a withdrawal",
        );
    }

    /// A registry-declared user-proc invoker (`Traits::INVOKES_USER_PROC`,
    /// the iRules `call PROC ?args?` form) names its callee in the first
    /// argument and passes the rest — a call site a walk that only ever
    /// looked at the command word would miss entirely.
    #[test]
    fn a_registry_declared_user_proc_invoker_is_a_call_site() {
        let ev = evidence_for_dialect(
            "proc helper {mode} { return $mode }\nwhen RULE_INIT { call helper dev }\n",
            tcl_dialect::DialectProfile::irules(),
        );
        assert_eq!(uniform(&ev, "::helper", 0).as_deref(), Some("dev"));
    }

    #[test]
    fn normal_procedure_invoker_retains_operand_offsets_and_real_uncertainty() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Conditional source caller metadata; no appliance or entered-frame claim.
        let profile = tcl_dialect::DialectProfile::irules();
        let exact = evidence_for_dialect(
            "proc helper {mode} {return $mode}\nwhen RULE_INIT {call -debug helper dev}\n",
            profile,
        );
        assert_eq!(uniform(&exact, "::helper", 0).as_deref(), Some("dev"));
        let shadowed = evidence_for_dialect(
            "proc helper {mode} {return $mode}\nproc call {args} {}\nwhen RULE_INIT {call helper dev}\n",
            profile,
        );
        assert_eq!(uniform(&shadowed, "::helper", 0), None);
        let unknown = evidence_for_dialect(
            "proc helper {mode} {return $mode}\nwhen RULE_INIT {call $target dev}\n",
            profile,
        );
        assert_eq!(uniform(&unknown, "::helper", 0), None);
    }

    #[test]
    fn an_unenumerable_dispatch_marks_the_module_non_enumerable() {
        let ev = evidence("proc helper {mode} { return $mode }\nset cmd [gets stdin]\n$cmd dev\n");
        assert_eq!(uniform(&ev, "::helper", 0), None);
    }

    #[test]
    fn a_callback_prefix_poisons_every_parameter_of_its_target() {
        let ev = evidence("proc cmp {a b} { return 0 }\nlsort -command cmp {x y}\n");
        assert_eq!(uniform(&ev, "::cmp", 0), None);
        assert_eq!(uniform(&ev, "::cmp", 1), None);
    }

    /// The `trace add variable v write cb` shape. The
    /// callback's position is registry data (`ArgRole::CommandPrefix` on the
    /// `trace add variable` subcommand), and the runtime appends the trace's
    /// own three arguments, so every parameter of the named proc is poisoned.
    #[test]
    fn a_trace_callback_registration_is_a_call_site() {
        for prefix in ["cb", "[list cb]"] {
            let src = format!(
                "proc cb {{name1 name2 op}} {{ return }}\nproc go {{}} {{ trace add variable v write {prefix} }}\n"
            );
            let ev = evidence(&src);
            assert_eq!(
                uniform(&ev, "::cb", 0),
                None,
                "`trace … write {prefix}` invokes cb with runtime-supplied arguments",
            );
        }
    }

    /// An unenumerable dispatch withdraws every seed in the unit that
    /// contains it — expressed per callee, so it survives a merge.
    #[test]
    fn an_unenumerable_dispatch_withdraws_this_units_seeds() {
        let ev = evidence(
            "proc helper {mode} { return $mode }\nhelper prod\nhelper prod\nset cmd [gets stdin]\n$cmd dev\n",
        );
        assert_eq!(uniform(&ev, "::helper", 0), None);
    }

    /// ...but only that unit's.  A file declaring no linkage cannot dispatch
    /// against a command table it never loaded, so merging its evidence into
    /// a project must not disturb an unrelated file's sound seed.  Without
    /// the reach bound, one `eval $script` anywhere in a workspace withdrew
    /// every interprocedural seed in every file.
    #[test]
    fn an_unenumerable_dispatch_does_not_reach_an_unlinked_file() {
        let opaque = evidence("proc mine {x} { return $x }\nset cmd [gets stdin]\n$cmd dev\n");
        let mut project =
            evidence("proc theirs {mode} { return $mode }\ntheirs prod\ntheirs prod\n");
        project.merge_from(&opaque);
        assert_eq!(
            uniform(&project, "::theirs", 0).as_deref(),
            Some("prod"),
            "the dispatching file declares no `source`/`package require`, so it \
             cannot name `theirs` at all",
        );
        assert_eq!(
            uniform(&project, "::mine", 0),
            None,
            "its own procedure is still withdrawn",
        );
    }

    /// The host supplies the reach, so a file linked to the callee's — in
    /// *either* direction — retracts its seed.
    ///
    /// The inbound direction is the one a file-local bound gets wrong: a
    /// library declaring no `source` of its own still runs inside its
    /// sourcer's interpreter, so its unreadable dispatch can name the
    /// sourcer's procedures. Bounding by what the *scanning* file loads
    /// missed exactly that.
    #[test]
    fn an_unenumerable_dispatch_reaches_whatever_the_host_says_it_links_to() {
        let reg = registry();
        let known: HashSet<String> = ["::theirs".to_owned()].into_iter().collect();
        let linked = ["::theirs".to_owned()];
        for src in [
            // Outbound: this file sources the other.
            "source other.tcl\nset cmd [gets stdin]\n$cmd dev\n",
            // Inbound: this file declares no linkage at all, but the host put
            // it in the callee's component because something sources it.
            "set cmd [gets stdin]\n$cmd dev\n",
        ] {
            let opaque = scan_source_call_sites(
                src,
                &reg,
                None,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                &known,
                &linked,
            );
            let mut project =
                evidence("proc theirs {mode} { return $mode }\ntheirs prod\ntheirs prod\n");
            project.merge_from(&opaque);
            assert_eq!(
                uniform(&project, "::theirs", 0),
                None,
                "a linked file's unreadable dispatch can name ::theirs: {src}",
            );
        }
    }

    /// ...and an *unlinked* file cannot: the host leaves it out of the reach,
    /// so its `$cmd` does not disturb a sound seed in a file it shares only a
    /// workspace folder with.
    #[test]
    fn an_unenumerable_dispatch_does_not_reach_an_unlinked_project_file() {
        let reg = registry();
        let known: HashSet<String> = ["::theirs".to_owned()].into_iter().collect();
        let opaque = scan_source_call_sites(
            "proc mine {x} { return $x }\nset cmd [gets stdin]\n$cmd dev\n",
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &known,
            // Not in the callee's component: its reach is its own procs.
            &["::mine".to_owned()],
        );
        let mut project =
            evidence("proc theirs {mode} { return $mode }\ntheirs prod\ntheirs prod\n");
        project.merge_from(&opaque);
        assert_eq!(
            uniform(&project, "::theirs", 0).as_deref(),
            Some("prod"),
            "an unlinked file shares no interpreter, so it cannot name ::theirs",
        );
    }

    #[test]
    fn an_omitted_defaulted_argument_poisons_its_slot() {
        let ev = evidence("proc helper {a {b DEFAULT}} { return $a }\nhelper one\n");
        assert_eq!(slot(&ev, "::helper", 0), (vec!["one".into()], false));
        assert_eq!(
            uniform(&ev, "::helper", 1),
            None,
            "the omitted argument leaves b bound to its default",
        );
    }

    #[test]
    fn a_script_the_scan_cannot_read_makes_the_module_non_enumerable() {
        for src in [
            "proc helper {mode} { return $mode }\nset s [gets stdin]\neval $s\n",
            "proc helper {mode} { return $mode }\ncatch $body\n",
            "proc helper {mode} { return $mode }\napply $fn 1\n",
        ] {
            assert!(
                uniform(&evidence(src), "::helper", 0).is_none(),
                "a script received as a value may call anything: {src}",
            );
        }
    }

    #[test]
    fn a_retained_script_value_records_its_actual_caller() {
        let ev = evidence("proc helper {mode} { return $mode }\nset s {helper dev}\neval $s\n");
        assert_eq!(uniform(&ev, "::helper", 0).as_deref(), Some("dev"));
        assert!(!ev.get("::helper").unwrap().opaque_caller);
    }

    #[test]
    fn a_literal_body_mentioning_a_variable_is_still_walked() {
        let ev = evidence("proc helper {mode} { return $mode }\ncatch {helper $x}\n");
        assert_eq!(slot(&ev, "::helper", 0), (Vec::new(), true));
        // The distinction that matters: the body was *walked*, so the call
        // was recorded at its real arity of one.  A withdrawal would have
        // recorded a zero-argument opaque caller instead.
        assert_eq!(
            ev.get("::helper")
                .unwrap()
                .arg_counts
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![1],
            "`catch {{helper $x}}` carries readable script text",
        );
    }

    #[test]
    fn whole_substitution_recognises_only_a_single_reference() {
        assert!(word_is_whole_substitution(
            "$script",
            tcl_lexer::LexerConfig::default()
        ));
        assert!(word_is_whole_substitution(
            "${script}",
            tcl_lexer::LexerConfig::default()
        ));
        assert!(word_is_whole_substitution(
            "[build]",
            tcl_lexer::LexerConfig::default()
        ));
        assert!(
            !word_is_whole_substitution("puts $v", tcl_lexer::LexerConfig::default()),
            "a braced body that merely mentions a variable is literal script text",
        );
        assert!(!word_is_whole_substitution(
            "helper dev",
            tcl_lexer::LexerConfig::default()
        ));
    }

    #[test]
    fn command_prefix_head_treats_literal_brackets_as_list_data() {
        let reg = registry();
        assert_eq!(
            command_prefix_head(&reg, "cmp -nocase").as_deref(),
            Some("cmp")
        );
        assert_eq!(
            command_prefix_head(&reg, "[list cmp $x]").as_deref(),
            Some("[list")
        );
        assert_eq!(
            command_prefix_head(&reg, "[pickCallback]").as_deref(),
            Some("[pickCallback]")
        );
        assert_eq!(
            command_prefix_head(&reg, "{cmp name} tail").as_deref(),
            Some("cmp name")
        );
    }

    #[test]
    fn original_callback_callers_share_source_horizons_without_parameter_seeds() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        for prefix in ["{cb extra}", "[list cb $x]", "[make $x]"] {
            let source = format!(
                "proc cb {{value args}} {{}}; interp alias {{}} make {{}} list cb; cb FIXED; lsort -command {prefix} {{3 1 2}}"
            );
            let result = evidence_for_dialect(&source, profile);
            assert!(
                result.get("::cb").is_some_and(|site| site.opaque_caller),
                "{source}: {result:?}"
            );
            assert_eq!(uniform(&result, "::cb", 0), None);
        }
        let source = "proc cb {value args} {}; cb FIXED; proc list args {}; lsort -command [list cb $x] {3 1 2}";
        let result = evidence_for_dialect(source, profile);
        assert_eq!(
            uniform(&result, "::cb", 0),
            None,
            "a computed prefix's target is unavailable"
        );
    }

    #[test]
    fn scope_var_facts_separate_literal_from_unknown_writes() {
        let reg = registry();
        let ir = retained_scope_fixture(
            "proc p {} {\n set a one\n set b [gets stdin]\n set c two\n set c three\n}\n",
            &reg,
        );
        let cfg_module = build_cfg(&ir, false);
        let cfg = cfg_module.procedures.get("::p").expect("p lowered");
        let facts = collect_scope_var_facts(
            cfg,
            &surface(&reg),
            ir.source_metadata_input
                .as_ref()
                .map(crate::analyser::ResolvedAnalysisInput::context_registry)
                .as_deref(),
        );
        assert!(!facts.dynamic_name_write);
        assert_eq!(
            facts.vars["a"].values.iter().cloned().collect::<Vec<_>>(),
            vec!["one".to_owned()],
        );
        assert!(facts.vars["b"].unknown);
        assert_eq!(
            facts.vars["c"].values.iter().cloned().collect::<Vec<_>>(),
            vec!["three".to_owned(), "two".to_owned()],
        );
    }

    #[test]
    fn a_write_through_a_computed_variable_name_poisons_the_whole_scope() {
        let reg = registry();
        let ir = retained_scope_fixture("proc p {n} {\n set a one\n set $n two\n}\n", &reg);
        let cfg_module = build_cfg(&ir, false);
        let cfg = cfg_module.procedures.get("::p").expect("p lowered");
        assert!(
            collect_scope_var_facts(
                cfg,
                &surface(&reg),
                ir.source_metadata_input
                    .as_ref()
                    .map(crate::analyser::ResolvedAnalysisInput::context_registry)
                    .as_deref()
            )
            .dynamic_name_write
        );
    }

    #[test]
    fn a_scope_alias_declaration_makes_the_aliased_name_unknown() {
        let reg = registry();
        let ir = retained_scope_fixture("proc p {} {\n global cmd\n set other one\n}\n", &reg);
        let cfg_module = build_cfg(&ir, false);
        let cfg = cfg_module.procedures.get("::p").expect("p lowered");
        let facts = collect_scope_var_facts(
            cfg,
            &surface(&reg),
            ir.source_metadata_input
                .as_ref()
                .map(crate::analyser::ResolvedAnalysisInput::context_registry)
                .as_deref(),
        );
        assert!(
            facts.vars["cmd"].unknown,
            "`global cmd` binds an outer variable any other body may write",
        );
    }
    fn supplied_unit_fixture(
        source: &str,
        context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> IrModule {
        let registry = context.commands();
        crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(
                registry,
                tcl_lexer::LexerConfig::for_profile(registry.profile()),
            )
            .with_dialect(registry.profile())
            .with_context_registry(std::sync::Arc::clone(context)),
            source,
        )
    }

    fn supplied_unit_input(
        current: &crate::analyser::ResolvedAnalysisInput,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> crate::analyser::ResolvedAnalysisInput {
        crate::analyser::ResolvedAnalysisInput::new(
            current.analyser_profile(),
            current.unit_profile(),
            context,
            current.lexer_config(),
        )
    }

    #[test]
    fn unit_linkage_keeps_supplied_availability_without_inventing_missing_boundaries() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Conditional Registry trait inventory only; no actual loading or export claim.
        let baseline = crate::environment_ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        );
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut descriptor = registry.get("dict").unwrap().clone();
        descriptor.traits |= Traits::LOADS_EXTERNAL_UNIT;
        registry.insert(descriptor);
        let context =
            std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(registry)));
        let mut module = supplied_unit_fixture("dict create key value", &context);
        let registry = context.commands();
        let input = module.source_metadata_input.clone().unwrap();
        assert!(std::sync::Arc::ptr_eq(&input.context_registry(), &context));
        assert_eq!(
            scan_unit_linkage(&module, registry, None),
            Traits::LOADS_EXTERNAL_UNIT
        );
        let older = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(std::sync::Arc::clone(registry)),
        );
        assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
        let original_script = module.top_level.clone();
        for withheld in [
            Some(supplied_unit_input(&input, older)),
            Some(supplied_unit_input(
                &input,
                crate::environment_ingress::context_for_profile(
                    tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
                ),
            )),
            None,
        ] {
            module.source_metadata_input = withheld;
            assert_eq!(
                scan_unit_linkage(&module, registry, Some(input.unit_profile())),
                Traits::empty()
            );
            assert_eq!(module.top_level, original_script);
        }
    }

    #[test]
    fn scope_variable_facts_keep_actual_availability_and_refuse_missing_or_foreign_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Original normal-write metadata only; no physical variable or frame claim.
        let context = crate::environment_ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        );
        let registry = context.commands();
        let module = supplied_unit_fixture("dict set target key value", &context);
        let cfg = crate::cfg_builder::build_cfg(&module, false);
        let current = collect_scope_var_facts(&cfg.top_level, &surface(registry), Some(&context));
        assert!(!current.dynamic_name_write && !current.cross_frame_write);
        assert!(current.vars["target"].unknown);
        let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry));
        assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
        let foreign = crate::environment_ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
        );
        for withheld in [Some(&older), Some(foreign.as_ref()), None] {
            let facts = collect_scope_var_facts(&cfg.top_level, &surface(registry), withheld);
            assert!(facts.dynamic_name_write && facts.cross_frame_write);
        }
    }

    fn supplied_unit_callers(
        source: &str,
        module: &IrModule,
        input: Option<&crate::analyser::ResolvedAnalysisInput>,
    ) -> CallSiteEvidence {
        let registry = module.resolved_registry();
        let cfg = crate::cfg_builder::build_cfg(module, false);
        let identities = crate::realm::document_realm_bindings_with_source_entry(
            source,
            module.lexer_config,
            registry,
            &module.source_entry,
        );
        let identities = input.map_or_else(
            || identities.clone(),
            |input| {
                identities
                    .clone()
                    .with_resolved_analysis_input(input.clone())
            },
        );
        collect_call_site_constants(
            &cfg,
            &[],
            &module.procedures,
            &module.future_call_sites,
            CallSiteSourceContext {
                registry,
                declared: None,
                dialect: module.dialect_profile.unwrap(),
                input,
                identities: &identities,
            },
        )
    }

    #[test]
    fn unit_caller_seeds_require_the_retained_metadata_owner() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Actual source-owned caller operands, not a complete Native command world.
        let context = crate::environment_ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        );
        let source = "proc helper {mode} {return $mode}; helper FIXED";
        let module = supplied_unit_fixture(source, &context);
        let input = module.source_metadata_input.as_ref().unwrap();
        let current = supplied_unit_callers(source, &module, Some(input));
        assert_eq!(uniform(&current, "::helper", 0).as_deref(), Some("FIXED"));
        let foreign = supplied_unit_input(
            input,
            crate::environment_ingress::context_for_profile(
                tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
            ),
        );
        for withheld in [Some(&foreign), None] {
            let evidence = supplied_unit_callers(source, &module, withheld);
            assert!(evidence.get("::helper").unwrap().opaque_caller);
            assert_eq!(uniform(&evidence, "::helper", 0), None);
        }
    }

    #[test]
    fn source_entry_caller_metadata_requires_an_independent_supplied_input() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Trusted source allocation remains independent of source metadata availability.
        let source = "source library.tcl; helper dev";
        let entry = source_loader_entry("proc helper {mode} {return $mode}");
        let current = scan_loaded_caller(source, &entry);
        assert_eq!(uniform(&current, "::helper", 0).as_deref(), Some("dev"));
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let missing = scan_source_call_sites_with_source_entry(
            source,
            &registry(),
            None,
            profile,
            &known(&["::helper"]),
            &["::helper".to_owned()],
            &entry,
        );
        assert_eq!(uniform(&missing, "::helper", 0), None);
        assert!(missing.get("::helper").unwrap().opaque_caller);
    }
}
