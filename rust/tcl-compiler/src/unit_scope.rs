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
//! ([`params_constants_from_call_sites`]) binds a parameter to a compile-time
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
//!    seed it can reach and no more (issue #976).  Because that reads
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
//!    ([`CallSiteEvidence::merge_from`]).  Issue #977: a plain library file
//!    with no `package provide`, `source`d by a file that calls its procs
//!    with a different literal, folded a genuinely varying parameter.
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
//! The gate [`params_constants_from_call_sites`] applies is stated in full on
//! that function.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::interprocedural::command_prefix_head;
use crate::ir::{Module as IrModule, Statement};
use crate::naming::is_dynamic_word;
use crate::value_shapes::{
    is_pure_var_ref, parse_command_substitution, whole_word_scalar_var_name,
};

/// Recursion cap for [`record_call_site_evidence`]'s descent into nested
/// `ArgRole::Body` arguments (`catch { catch { catch { … } } }` and similar) —
/// defensive against a pathological or generated nesting depth; real code
/// never approaches it.
const MAX_CALL_SITE_BODY_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(16);

/// Iteration cap for the call-site evidence fixpoint ([`run_to_fixpoint`]).
///
/// Each round re-derives the whole evidence set from the previous round's,
/// which only ever grows, so the chain is increasing and terminates on its
/// own; the cap is the defensive backstop for a pathological module.  A
/// module that has not converged by then is treated as having an
/// unenumerable caller rather than being trusted at a non-fixpoint.
const MAX_FIXPOINT_ROUNDS: u32 = 6;

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
    /// Argument counts observed across the recorded call sites.  A call that
    /// supplies fewer arguments than the callee has parameters leaves the
    /// remaining parameters bound to their **defaults**, an unknown value at
    /// those positions — so a parameter is only literal-uniform when every
    /// observed call actually reached it (see [`Self::binds_position`]).
    pub arg_counts: BTreeSet<usize>,
    /// Literal evidence per 0-based argument position.
    pub slots: BTreeMap<usize, ArgConsts>,
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
        if slot.unknown || slot.values.len() != 1 || !self.binds_position(index) {
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
        self.arg_counts.insert(0);
        for slot in self.slots.values_mut() {
            slot.unknown = true;
        }
    }

    /// Fold `other`'s call sites into this one.  Merging only ever widens
    /// (more values, more unknowns, more argument counts), so evidence from a
    /// second file can retract a fold but never manufacture one.
    fn merge_from(&mut self, other: &Self) {
        self.arg_counts.extend(other.arg_counts.iter().copied());
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
    /// damage PR #970 reverted the module-wide dispatch wildcard for, at
    /// project scope.
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
struct CallSiteScanCtx<'a, S> {
    /// The qualified names a call may resolve to.  For the in-unit scan this
    /// is the unit's own procedures; for [`scan_source_call_sites`] it is the
    /// whole project's, so a cross-file call resolves to the file that
    /// actually defines the callee.
    known: &'a HashSet<String, S>,
    registry: &'a CommandRegistry,
    dialect: &'a str,
    /// `namespace import` directives (`(importing_namespace, absolute_pattern)`
    /// pairs), from [`crate::ir::Module::namespace_imports`] — see
    /// [`resolve_via_namespace_import`].
    namespace_imports: &'a [(String, String)],
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

/// One body the scan walks as a *caller*.
///
/// `resolve_as` and `scope` differ for a `TclOO` method: its bare command
/// words resolve against the global namespace (tclsh-confirmed), while its
/// local variables live in the method's own frame.
struct CallerFrame<'a> {
    /// Qualified-name context bare command words resolve against.
    resolve_as: &'a str,
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
/// generic call name a variable is registry data ([`ArgRole::VarWrite`],
/// plus [`Traits::CREATES_SCOPE_ALIAS`] for the vararg alias forms `global
/// x y z` / `variable a b` / `upvar 1 a b 1 c d`, whose per-argument name
/// list the role query deliberately does not expand) — no command name is
/// matched here.
///
/// Structured bodies (`if`, `while`, `catch`, a *literal* `eval`/`uplevel
/// #0` body) are already flattened into the caller's own CFG blocks by
/// lowering, so their writes are ordinary block statements here.  The two
/// that are not — a frame-shifting [`Statement::UpFrame`] body and a body
/// reached only through a substitution on a [`Traits::EVALUATES_CODE`]
/// command — set [`ScopeVars::cross_frame_write`] instead.
fn collect_scope_var_facts(cfg: &CfgFunction, registry: &CommandRegistry) -> ScopeVars {
    let mut out = ScopeVars::default();
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
                Statement::Call { args, defs, .. } => {
                    for d in defs {
                        out.note_write_word(d);
                    }
                    note_registry_var_writes(&mut out, registry, stmt, args);
                }
                Statement::Barrier { args, .. } => {
                    note_registry_var_writes(&mut out, registry, stmt, args);
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

/// Record the variable writes the registry declares for one call.
fn note_registry_var_writes(
    out: &mut ScopeVars,
    registry: &CommandRegistry,
    stmt: &Statement,
    args: &[String],
) {
    let command = stmt.canonical_command_or_source();
    let bare = command.strip_prefix("::").unwrap_or(command);
    if is_dynamic_word(bare) {
        // A computed head (`$cmd 5`) has no registry spec to read roles
        // from, so there is nothing to record. It is deliberately not
        // treated as perturbing the scope either: a *user proc* reached
        // this way can only write the caller's locals through `upvar`,
        // which this scan does not model for a literal call site either.
        // The residual — a computed head that resolves to a
        // variable-writing *builtin* (`set cmd set; $cmd x 5`) — would
        // require a builtin's own name to be among the literals a local
        // holds, and is documented rather than paid for by disqualifying
        // every dispatch-table body's own variables.
        return;
    }
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    for idx in registry.arg_indices_for_role(bare, &arg_strs, ArgRole::VarWrite) {
        if let Some(word) = args.get(idx) {
            out.note_write_word(word);
        }
    }
    let Some(spec) = registry.get(bare) else {
        return;
    };
    // `global` / `variable` / `upvar` bind *every* name in a vararg list to
    // an outer-scope variable another body may write — the registry marks
    // exactly these with `CREATES_SCOPE_ALIAS`, because the per-argument
    // list is not expressible as fixed role indices.
    if spec.traits.contains(Traits::CREATES_SCOPE_ALIAS) {
        for word in args {
            out.note_write_word(word);
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
fn word_is_whole_substitution(word: &str) -> bool {
    is_pure_var_ref(word) || parse_command_substitution(word).is_some()
}

/// Resolve `command` to a qualified proc name via a `namespace import`
/// directive active in `caller_qname`'s own namespace, when
/// [`crate::interprocedural::resolve_internal_call`] found no direct match.
///
/// `namespace import ::lib::helper` binds the bare name `helper` in the
/// importing namespace to `::lib::helper` — a real command-resolution path
/// distinct from (and checked *after*) plain namespace-relative lookup, so a
/// call site reached only this way was invisible to the collector before
/// this: `::lib::helper`'s only *visible* caller was some unrelated external
/// call, while every importing-namespace caller's (potentially differing)
/// argument silently vanished from the evidence, exactly like the
/// namespace-blind recursion and `ArgRole::Body` gaps.
///
/// `::foo::*` (wildcard) imports resolve `command` under `::foo`; an exact
/// pattern (`::foo::bar`) binds only its own leaf name — `namespace_imports`
/// already only ever records absolute patterns (relative ones need runtime
/// namespace-path walking this compile-time pass does not model, per
/// [`crate::ir::Module::namespace_imports`]'s own doc).
fn resolve_via_namespace_import<S: std::hash::BuildHasher>(
    command: &str,
    caller_qname: &str,
    namespace_imports: &[(String, String)],
    known: &HashSet<String, S>,
) -> Option<String> {
    if namespace_imports.is_empty() {
        return None;
    }
    let ns_parts = crate::interprocedural::namespace_parts_from_proc(caller_qname);
    let caller_ns = if ns_parts.is_empty() {
        "::".to_owned()
    } else {
        format!("::{}", ns_parts.join("::"))
    };
    for (import_ns, pattern) in namespace_imports {
        if *import_ns != caller_ns {
            continue;
        }
        if let Some(ns_prefix) = pattern.strip_suffix("::*") {
            let candidate = format!("{ns_prefix}::{command}");
            if known.contains(&candidate) {
                return Some(candidate);
            }
        } else if pattern.rsplit("::").next().unwrap_or(pattern.as_str()) == command
            && known.contains(pattern)
        {
            return Some(pattern.clone());
        }
    }
    None
}

/// Record one call site's literal-argument evidence into `out`, then recurse
/// into any `ArgRole::Body` argument of `command` (regardless of whether
/// `command` itself is a user proc) — a nested script embedded in a nested
/// script embedded in a nested script, and so on, up to
/// [`MAX_CALL_SITE_BODY_DEPTH`].
///
/// This is the fix for the residual gap issue #969's own root cause left
/// open: `catch { isEven 4 }`, a non-exact `switch` arm, a literal `uplevel
/// {…}` / `apply {{…} {…}}` body, and friends all carry their nested script
/// as one opaque *argument string* to a builtin (`catch`, `switch`,
/// `uplevel`, `apply`) that is never itself a user proc — so a flat,
/// one-level `Statement::Call`/`Statement::Barrier` walk resolves `catch`
/// (finds no matching proc, moves on) and never notices `isEven 4` sitting
/// inside its body argument at all. That's a *second* proc call this scan
/// cannot see, exactly like the namespace-resolution gap: an invisible call
/// site with a differing argument silently vanishes from
/// [`params_constants_from_call_sites`]'s "every caller agrees" evidence.
///
/// The registry already knows which argument position of which command is a
/// script body (`ArgRole::Body`, driving the identical recursive call-graph
/// walk in [`crate::interprocedural`] and the `BODY`-role scans in
/// `ir_helpers.rs` / `place_bridge.rs` / `ssa.rs`) — so this reuses that one
/// fact via [`tcl_registry::CommandRegistry::arg_indices_for_role`] and the
/// shared [`crate::segmenter`] rather than hand-rolling a second "which
/// commands embed scripts" list here.
fn record_call_site_evidence(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    command: &str,
    args: &[String],
    depth: u32,
) {
    // A dispatched command word (`$cmd args`) is resolved by *value*, not
    // skipped: the scope's own literal assignments (unioned, for a parameter,
    // with the literals its callers pass) give the set of names it may hold,
    // and each becomes an ordinary call site.  Skipping it — the behaviour
    // before issue #976 — let a dispatch reach a proc this scan had already
    // seeded from its literal call sites, silently unsoundly.  A word whose
    // value set is not enumerable withdraws every seed instead.
    record_invocation(out, ctx, caller, command, IndirectArgs::Words(args));
    if is_dynamic_word(command) {
        // Which of a computed head's arguments carry scripts, callbacks, or a
        // callee name is unknowable; the dispatch itself is already accounted
        // for above.
        return;
    }
    record_indirect_callers(out, ctx, caller, command, args);
    if MAX_CALL_SITE_BODY_DEPTH.exceeded(depth + 1) {
        return;
    }
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    // A lambda the command receives as a *value* (`apply $fn`) is exactly as
    // unreadable as a script body received as one, and may likewise call
    // anything.  A *literal* lambda needs no walking here — lowering gives it
    // its own body unit, which the scan already visits as a caller.
    for idx in ctx
        .registry
        .arg_indices_for_role(command, &arg_strs, ArgRole::LambdaLiteral)
    {
        if args.get(idx).is_some_and(|w| word_is_whole_substitution(w)) {
            out.record_unenumerable_caller(ctx.unenumerable_reach);
        }
    }
    for idx in ctx
        .registry
        .arg_indices_for_role(command, &arg_strs, tcl_registry::ArgRole::Body)
    {
        let Some(body_text) = args.get(idx) else {
            continue;
        };
        // `eval $script` / `catch $body` carry no script *text* at all, only
        // a reference to one: unreadable, and it may call any procedure with
        // any argument.  A literal body that merely *mentions* a variable
        // (`catch {puts $x}`) is still walked — the discriminator is "the
        // whole word is one substitution", not "contains a `$`".
        if word_is_whole_substitution(body_text) {
            out.record_unenumerable_caller(ctx.unenumerable_reach);
            continue;
        }
        // A body whose resolution namespace differs from the caller's must not
        // be walked with the caller's namespace: `namespace eval ::a { helper }`
        // calls `::a::helper`, never the caller's own `helper`
        // (tclsh8.6-confirmed; see `lowering`'s `register_body_unit` call for
        // the same reasoning). Such a command carries an absolute `ArgRole::Name`
        // naming that namespace, and lowering has already registered its body as
        // a body unit whose qname encodes it — which
        // `build_extra_call_site_scan_contexts` scans with the *correct*
        // namespace. Recursing here as well would scan it a second time under
        // the wrong one, inventing a call to a same-named proc in the caller's
        // namespace. Cross-file that is a false edge into another file's
        // procedure; in-unit it was merely invisible, because a bare global
        // `::helper` is rarely in a single file's own `known` set (issue #977).
        if ctx
            .registry
            .arg_indices_for_role(command, &arg_strs, tcl_registry::ArgRole::Name)
            .into_iter()
            .filter_map(|i| args.get(i))
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
        // only re-walk it under the definer's. Issue #980: `namespace eval
        // ::foo { proc runIt {} { uplevel #0 { helper b } } }` invented a
        // call to `::foo::helper` — a proc tclsh8.6/9.0 confirm real Tcl
        // never reaches this way.
        if ctx
            .registry
            .get(command.strip_prefix("::").unwrap_or(command))
            .is_some_and(|spec| spec.traits.contains(Traits::DEFINES_PROCEDURE))
        {
            continue;
        }
        let nested = crate::segmenter::segment_commands_with_offset_and_config(
            body_text,
            0,
            tcl_lexer::LexerConfig::for_dialect(ctx.dialect),
        );
        for cmd in &nested {
            let name = cmd.name();
            if name.is_empty() {
                continue;
            }
            record_call_site_evidence(out, ctx, caller, name, cmd.args(), depth + 1);
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
) {
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
    for name in &values {
        let Some(target) = resolve_target(ctx, caller.resolve_as, name) else {
            record_unresolved_word_dispatch(out, ctx, name, args);
            continue;
        };
        match args {
            IndirectArgs::Words(words) => out.record_call(target, words),
            IndirectArgs::Unknowable => out.record_opaque_caller(&target),
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
/// the unresolved words genuinely vary (issue #1044).
///
/// Which command is the handler comes from
/// [`Traits::UNRESOLVED_COMMAND_HANDLER`], never a literal name here. The
/// lookup is global-scope only: a namespace-local `proc unknown` is *not*
/// consulted for unresolved words in that namespace — tclsh8.6/9.0 both
/// dispatch to `::unknown` regardless of the calling namespace.
fn unresolved_command_handler<'a, S: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    known: &'a HashSet<String, S>,
) -> Option<&'a str> {
    registry
        .commands_with_trait(Traits::UNRESOLVED_COMMAND_HANDLER)
        .into_iter()
        .find_map(|name| {
            let qualified = crate::naming::qualify("::", name);
            known.get(&qualified).map(String::as_str)
        })
}

/// Record the unresolved-command handler's own invocation for a literal
/// command word this scan could not resolve (issue #1044).
///
/// Only fires when the module defines a handler, and only for a word the
/// registry does not know either — everything else either resolves or is a
/// builtin. The word becomes the handler's first argument and the call's
/// own arguments follow, exactly as Tcl passes them, so the evidence union
/// is the precise set of values the handler's parameters really see.
///
/// Deliberately over-inclusive on the residue: a word bound by something
/// this scan does not model (a class command, an ensemble, a coroutine)
/// also reads as unresolved and contributes an extra value. That can only
/// *retract* a fold, never manufacture one, and it costs nothing in a
/// module with no handler.
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

/// The user procedure a (literal) command word names when invoked from
/// `caller_qname`'s namespace, or `None` when it names no procedure this
/// scan knows.
fn resolve_target(
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller_qname: &str,
    command: &str,
) -> Option<String> {
    crate::interprocedural::resolve_internal_call(command, caller_qname, ctx.known).or_else(|| {
        resolve_via_namespace_import(command, caller_qname, ctx.namespace_imports, ctx.known)
    })
}

/// Record the callers a statement creates *without* naming their arguments:
/// a deferred command prefix, and a rebinding of a known command's name.
///
/// Both are real call paths whose argument list this scan can never see —
/// `after 0 helper`, `trace add variable v write helper`, `-command helper`
/// all invoke `helper` with runtime-supplied words appended, and `rename
/// helper other` / `interp alias {} h {} helper` let a call reach `helper`
/// under a name no scan attributed to it. Before this they were simply
/// *absent* from the evidence, which reads to
/// [`params_constants_from_call_sites`] as "no caller disagrees" — the exact
/// shape issue #969 reported, reached through a callback instead of a missed
/// call site. Recording them as opaque callers states the truth instead: a
/// call site exists whose arguments are unknown.
///
/// Registry-driven throughout — the callback positions come from
/// [`tcl_registry::ArgRole::CommandPrefix`]
/// ([`CommandRegistry::arg_indices_for_role`]) and the rebinding forms from
/// [`CommandRegistry::command_table_effect`], so no command name appears
/// here.  Closes, for both the in-unit and the cross-unit scan, the
/// `CommandPrefix` limitation PR #970 documented as shared and pre-existing.
fn record_indirect_callers(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    command: &str,
    args: &[String],
) {
    let caller_qname = caller.resolve_as;
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    // A command the registry marks `INVOKES_USER_PROC` (the iRules `call
    // PROC ?args?` form) invokes the procedure its *first* argument names,
    // passing the rest — a call site whose arguments this scan really can
    // see, so it is recorded in full rather than merely poisoned.
    if ctx
        .registry
        .get(command.strip_prefix("::").unwrap_or(command))
        .is_some_and(|spec| spec.traits.contains(Traits::INVOKES_USER_PROC))
        && let Some((name, rest)) = args.split_first()
    {
        record_invocation(out, ctx, caller, name, IndirectArgs::Words(rest));
    }
    for idx in ctx
        .registry
        .arg_indices_for_role(command, &arg_strs, ArgRole::CommandPrefix)
    {
        // A command prefix is a list whose first word is the command; the
        // rest are leading arguments the runtime appends to. Only the head
        // names a callee — and when the prefix was *built* by a registry-
        // declared builder (`[list cb $x]`), the head is that builder's own
        // first argument, not the literal text `[list`.
        let Some(prefix) = args.get(idx) else {
            continue;
        };
        match command_prefix_head(ctx.registry, prefix) {
            Some(head) => record_invocation(out, ctx, caller, &head, IndirectArgs::Unknowable),
            // Some other substitution computed the prefix: a caller exists
            // naming a command this scan cannot identify.
            None => out.record_unenumerable_caller(ctx.unenumerable_reach),
        }
    }
    if ctx
        .registry
        .command_table_effect(command, arg_strs.first().copied())
        .is_some_and(|effect| {
            matches!(
                effect,
                tcl_registry::CommandTableEffect::RenamesCommands
                    | tcl_registry::CommandTableEffect::CreatesAliases
            )
        })
    {
        // Every word of a rebinding call that names a known command is a
        // name whose binding has moved — `rename OLD NEW` in either
        // direction, `interp alias {} NEW {} TARGET` in either.
        for word in &arg_strs {
            if word.is_empty() || word.contains(['$', '[']) {
                continue;
            }
            if let Some(target) =
                crate::interprocedural::resolve_internal_call(word, caller_qname, ctx.known)
            {
                out.record_opaque_caller(&target);
            }
        }
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
/// another scope's (issue #980).
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
/// `::foo::helper` (issue #980).
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

/// Build bare CFGs (no further per-function analysis) for every `TclOO` method,
/// synthetic body unit (`apply` lambda, `namespace eval` body), and static
/// `uplevel` body, so [`collect_call_site_constants`] can walk them as
/// *callers* too.
///
/// Neither is itself ever seeded with `param_constants`
/// (`build_method_units` / `build_body_units` always pass `None` for their
/// own analysis), but a call *from* one of their bodies *to* an ordinary
/// user proc is a real call site whose argument can vary between call
/// sites, exactly like a bare top-level or proc-body call — invisible to
/// the collector before this, which walked only `cfg_module.top_level` and
/// `cfg_module.procedures`. The same class of bug as issue #969's own root
/// cause (a real, varying call site silently missing from the "every
/// caller agrees" evidence), reached through a method/lambda body instead
/// of namespace-blind recursion or a `catch`/`uplevel` body.
///
/// Returns an empty `Vec` (no cost beyond
/// [`needs_extra_call_site_scan_contexts`]) when the module has none of the
/// three — the overwhelmingly common case — or when `cfg_context` is `None`
/// (all three require it; the builders compute it exactly when
/// [`needs_extra_call_site_scan_contexts`] says so).
pub(crate) fn build_extra_call_site_scan_contexts(
    ir_module: &IrModule,
    cfg_context: Option<&crate::cfg_builder::CfgContext>,
) -> Vec<(String, CfgFunction)> {
    let upframes = upframe_scan_bodies(ir_module);
    if ir_module.methods.is_empty() && ir_module.body_units.is_empty() && upframes.is_empty() {
        return Vec::new();
    }
    let Some((upvar_procs, proc_params, global_write_procs)) = cfg_context else {
        return Vec::new();
    };
    let build = |qname: &str, body: &crate::ir::Script| {
        crate::cfg_builder::build_cfg_function_with_upvars(
            qname,
            body,
            true,
            upvar_procs.clone(),
            proc_params.clone(),
            global_write_procs.clone(),
        )
    };
    ir_module
        .methods
        .iter()
        .map(|(mqname, method)| {
            // tclsh8.6-confirmed (live): a bare command inside a `TclOO`
            // method body resolves against the GLOBAL namespace, never the
            // class's own declaring namespace — `method go {} { helper }`
            // calls `::helper` even when a proc of the same name sits in
            // the class's own namespace. `mqname`'s own namespace (e.g.
            // `::foo::Widget` for method `::foo::Widget::go`) would be
            // wrong here, so the caller-context string this scan resolves
            // against is forced to global — reusing `"::top"`, the same
            // pseudo-qname the top-level script already uses to mean
            // exactly that. The CFG's own identity still uses the real
            // `mqname` (unrelated to this scan's namespace concern).
            ("::top".to_owned(), build(mqname, &method.body))
        })
        .chain(
            ir_module
                .body_units
                .iter()
                .map(|(qname, unit)| (qname.clone(), build(qname, &unit.body))),
        )
        .chain(upframes.into_iter().map(|up| {
            // Same shape as the method case: the caller context bare command
            // words resolve against is chosen by the frame the body runs in
            // (see [`upframe_scan_bodies`]), while the CFG keeps a distinct
            // identity of its own. Here that identity must be *synthetic* —
            // the CFG's name is this scan's variable-scope key, and reusing
            // the resolution context would overwrite that scope's real
            // variable facts in `collect_module_scope_var_facts`.
            (up.resolve_as, build(&up.scope, up.body))
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
/// [`params_constants_from_call_sites`]'s "every caller passes the same
/// literal" evidence, which can flip an absence of contradicting evidence
/// into a false positive. Issue #969: a proc declared inside a `namespace
/// eval` block recursed into itself by its bare (unqualified) name; the old
/// resolver only ever tried global-qualified spellings of the command word,
/// so it could never match the proc's namespaced qualified name, and the
/// recursive self-call — whose argument necessarily varies call to call —
/// was silently dropped. Only the one external, fully-qualified caller's
/// literal remained, so the loop/recursion-varying parameter was seeded as
/// that one constant and folded a genuinely alternating condition (`$count &
/// 1`) to a fixed boolean.
pub(crate) fn collect_call_site_constants(
    cfg_module: &CfgModule,
    extra_callers: &[(String, CfgFunction)],
    procedures: &HashMap<String, crate::ir::Procedure>,
    namespace_imports: &[(String, String)],
    registry: &CommandRegistry,
    dialect: &str,
) -> CallSiteEvidence {
    let known: HashSet<String> = procedures.keys().cloned().collect();
    let var_facts = collect_module_scope_var_facts(cfg_module, extra_callers, registry);
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
            dialect,
            namespace_imports,
            var_facts: &var_facts,
            previous,
            procedures,
            unenumerable_reach: &reach,
            unresolved_handler,
        };
        let funcs = std::iter::once(("::top", &cfg_module.top_level))
            .chain(cfg_module.procedures.iter().map(|(q, f)| (q.as_str(), f)))
            .chain(extra_callers.iter().map(|(q, f)| (q.as_str(), f)));
        let mut out = CallSiteEvidence::default();
        scan_cfg_callers(&mut out, &ctx, funcs);
        out
    };
    run_to_fixpoint(&reach, round)
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
/// the overwhelmingly common case pays nothing.  [`MAX_FIXPOINT_ROUNDS`] is
/// a backstop against a pathological module, not an expected exit.
fn run_to_fixpoint(
    reach: &[String],
    round: impl Fn(&CallSiteEvidence) -> CallSiteEvidence,
) -> CallSiteEvidence {
    let mut evidence = CallSiteEvidence::default();
    for _ in 0..MAX_FIXPOINT_ROUNDS {
        let next = round(&evidence);
        if !next.consulted_value_sets || next == evidence {
            return next;
        }
        evidence = next;
    }
    // Did not converge within the cap: report the honest thing rather than a
    // half-derived seed — every seed is withdrawn.
    let mut exhausted = evidence;
    exhausted.record_unenumerable_caller(reach);
    exhausted
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
/// every interprocedural seed in every file (issue #976's cross-file half).
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
    extra_callers: &[(String, CfgFunction)],
    registry: &CommandRegistry,
) -> ModuleVarFacts {
    let mut out = ModuleVarFacts::default();
    let funcs = std::iter::once(&cfg_module.top_level)
        .chain(cfg_module.procedures.values())
        .chain(extra_callers.iter().map(|(_, f)| f));
    for func in funcs {
        let facts = collect_scope_var_facts(func, registry);
        out.cross_frame_write |= facts.cross_frame_write;
        out.scopes.insert(func.name.clone(), facts);
    }
    out
}

/// Walk each `(caller qname, CFG)` pair's flattened statements, recording
/// every `Call`/`Barrier` as a call site.  Shared by the in-unit scan and
/// [`scan_source_call_sites`] so the two can never diverge on what counts.
fn scan_cfg_callers<'a>(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    funcs: impl Iterator<Item = (&'a str, &'a CfgFunction)>,
) {
    for (resolve_as, func) in funcs {
        // The CFG function's own name is the *variable-scope* identity, which
        // differs from `resolve_as` for a `TclOO` method (global command
        // resolution, own frame).  A body the unit declares as a procedure is
        // one this evidence map attributes calls *to*; a method or `apply`
        // lambda is not, so its parameters may hold values never seen here.
        let declared = ctx.procedures.get(func.name.as_str());
        let caller = CallerFrame {
            resolve_as,
            scope: func.name.as_str(),
            params: declared.map_or(&[][..], |p| p.params.as_slice()),
            callers_tracked: declared.is_some() || func.name == "::top",
        };
        for block in func.blocks.values() {
            for stmt in &block.statements {
                let (Statement::Call { command, args, .. }
                | Statement::Barrier { command, args, .. }) = stmt
                else {
                    continue;
                };
                record_call_site_evidence(out, ctx, &caller, command, args, 0);
            }
        }
    }
}

/// Collect the call sites **another** file contributes, resolved against the
/// whole project's procedure names.
///
/// This is the cross-file half of issue #977: a plain library file with no
/// `package provide` is `source`d by a file that calls its procs with a
/// different literal, and the library's own compilation unit — single-source
/// by construction — can never see that caller.  A host with a workspace view
/// (the LSP; the `tcl` CLI when given several files) runs this over every
/// other file and hands the result to
/// [`crate::compilation_unit::CompilationUnit`]'s build, which merges it into
/// the in-unit evidence before seeding.
///
/// `known` must be the **project-wide** set of procedure qualified names, so
/// a bare call in the scanned file resolves to the file that really defines
/// it rather than resolving to nothing.  The scan is deliberately the same
/// lowering → CFG → [`record_call_site_evidence`] path the in-unit scan takes
/// (including its `TclOO` method / `apply` / `namespace eval` body-unit
/// callers and its `ArgRole::Body` recursion), so cross-file evidence is
/// exactly what this unit would have collected had the two files been one.
#[must_use]
pub fn scan_source_call_sites<S: std::hash::BuildHasher>(
    source: &str,
    registry: &CommandRegistry,
    dialect: &str,
    known: &HashSet<String, S>,
    dispatch_reach: &[String],
) -> CallSiteEvidence {
    let mut out = CallSiteEvidence::default();
    if known.is_empty() {
        return out;
    }
    let config = tcl_lexer::LexerConfig::for_dialect(dialect);
    let mut ir_module = crate::lowering::lower_to_ir_with_config(source, registry, config);
    crate::specialise_factories::specialise_factories(&mut ir_module, registry);
    crate::inline_uplevel::inline_uplevel_passthrough(&mut ir_module, registry);
    let cfg_module = crate::cfg_builder::build_cfg(&ir_module, false);
    let cfg_context = needs_extra_call_site_scan_contexts(&ir_module)
        .then(|| crate::cfg_builder::prepare_cfg_context(&ir_module));
    let extra = build_extra_call_site_scan_contexts(&ir_module, cfg_context.as_ref());
    // The cross-file scan resolves a dispatch word exactly as the in-unit one
    // does — `scan_cfg_callers`/`record_call_site_evidence` are shared, so a
    // `set cmd helper; $cmd dev` in *another* file retracts this unit's seed
    // just as an in-unit one would.  Without the same var facts and fixpoint
    // here, issue #976 would simply reopen across the file boundary.
    let var_facts = collect_module_scope_var_facts(&cfg_module, &extra, registry);
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
    out = run_to_fixpoint(&reach, |previous| {
        let ctx = CallSiteScanCtx {
            known,
            registry,
            dialect,
            namespace_imports: &ir_module.namespace_imports,
            var_facts: &var_facts,
            previous,
            procedures: &ir_module.procedures,
            unenumerable_reach: &reach,
            unresolved_handler,
        };
        let funcs = std::iter::once(("::top", &cfg_module.top_level))
            .chain(cfg_module.procedures.iter().map(|(q, f)| (q.as_str(), f)))
            .chain(extra.iter().map(|(q, f)| (q.as_str(), f)));
        let mut round = CallSiteEvidence::default();
        scan_cfg_callers(&mut round, &ctx, funcs);
        round
    });
    // A `namespace import` in the scanned file binds one of `known`'s
    // commands under a *new* bare name, so a call through that name never
    // reaches `record_call_site_evidence`'s resolver at all. Record it as an
    // opaque caller: a real call path whose arguments are unknown. (Renames
    // and aliases are already covered per-statement by
    // `record_indirect_callers`.)
    for (_, pattern) in &ir_module.namespace_imports {
        if let Some(ns_prefix) = pattern.strip_suffix("::*") {
            let prefix = format!("{ns_prefix}::");
            let imported: Vec<String> = known
                .iter()
                .filter(|q| q.starts_with(&prefix) && q[prefix.len()..].find("::").is_none())
                .cloned()
                .collect();
            for target in imported {
                out.record_opaque_caller(&target);
            }
        } else if known.contains(pattern) {
            out.record_opaque_caller(pattern);
        }
    }
    out
}

/// Which registry-declared unit boundaries this module crosses.
///
/// A pure union of [`CommandRegistry::unit_linkage`] over every resolved
/// `Call`/`Barrier` statement in the module — top level, every
/// proc/method/body-unit, and every nested control-flow body.  No command
/// name appears here: `package provide`, `source`, `namespace export` and
/// friends are recognised only through the traits their specs carry, so
/// teaching the compiler about a new boundary command is a registry edit
/// (see [`tcl_registry::traits::UNIT_LINKAGE_TRAITS`]).
///
/// Replaces the raw-text `package provide` substring scan (PR #970) and then
/// the IR walk that hardcoded the `package`/`provide` word pair: the former
/// both over-triggered (any script merely *mentioning* the phrase in a
/// comment or string disabled every interprocedural seed in the file) and
/// under-triggered (`package\tprovide`, `::package provide`); the latter was
/// correct but knew a command by name, and missed every other way a file
/// admits to being part of a larger program.
#[must_use]
pub fn scan_unit_linkage(
    ir_module: &IrModule,
    registry: &CommandRegistry,
    dialect: &str,
) -> Traits {
    let dialect_set =
        tcl_dialect::DialectSet::parse(dialect).unwrap_or_else(tcl_dialect::DialectSet::empty);
    let mut found = Traits::empty();

    let mut visit = |stmt: &crate::ir::Statement| {
        if let crate::ir::Statement::Call { command, args, .. }
        | crate::ir::Statement::Barrier { command, args, .. } = stmt
        {
            let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
            found |= registry.unit_linkage(command, &arg_strs, dialect_set);
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
    /// registry-declared boundary no longer has to be treated as an unknown
    /// caller; without it, the unit is on its own and any boundary sinks the
    /// seed.
    pub has_cross_file_evidence: bool,
    /// Whole-module `rename` / `interp alias` / dynamic-redefinition trust
    /// lattice.
    pub command_mutations: &'a crate::command_binding::ModuleCommandMutations,
}

/// Boundaries that publish this file's commands to callers **no** host
/// enumeration can bound — another checkout can `package require` this one,
/// or `namespace import` from it.  Distinct from `LOADS_EXTERNAL_UNIT`, whose
/// implied caller is normally a project file the host has already scanned.
const UNBOUNDABLE_BOUNDARIES: Traits = Traits::PROVIDES_PACKAGE.union(Traits::EXPORTS_COMMAND);

impl UnitCallerView<'_> {
    /// Whether a registry-declared boundary rules out seeding outright — see
    /// [`params_constants_from_call_sites`]'s gate for the full rule.
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
/// - `!command_mutations.trusts_proc_binding(qname)` — `qname`'s own
///   binding may have been perturbed by `rename` / `interp alias` / a
///   dynamic proc redefinition anywhere in the module, so a call reaching
///   it at runtime need not be one this scan attributed to it (and vice
///   versa).
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
///   seed in the unit is withdrawn (issue #976).
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
    if !view.command_mutations.trusts_proc_binding(qname) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::build_cfg;
    use crate::lowering::lower_to_ir;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
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
        let mutations = crate::command_binding::ModuleCommandMutations::default();
        params_constants_from_call_sites(
            params,
            evidence,
            qname,
            &UnitCallerView {
                linkage,
                has_cross_file_evidence,
                command_mutations: &mutations,
            },
        )
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
            "",
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
            "",
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
            "",
            &known(&["::helper"]),
            &[],
        );
        a.merge_from(&b);
        assert_eq!(a.get("::helper").unwrap().uniform_literal_at(0), None);
    }

    /// A deferred command prefix (`ArgRole::CommandPrefix`) invokes the proc
    /// with runtime-supplied words appended, so no position stays uniform.
    /// The callback slot comes from the registry, not a command-name list.
    #[test]
    fn a_command_prefix_callback_is_recorded_as_an_opaque_caller() {
        let reg = registry();
        let evidence = scan_source_call_sites(
            "helper prod\nafter 0 helper\n",
            &reg,
            "",
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
            "helper prod\nrename helper legacy\n",
            "helper prod\ninterp alias {} h {} helper\n",
        ] {
            let evidence = scan_source_call_sites(src, &reg, "", &known(&["::helper"]), &[]);
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
            "helper one two\nhelper one two\n",
            &reg,
            "",
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
            "helper prod\nhelper prod\n",
            &reg,
            "",
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
            let module = crate::lowering::lower_to_ir_with_config(
                src,
                &reg,
                tcl_lexer::LexerConfig::default(),
            );
            assert_eq!(scan_unit_linkage(&module, &reg, ""), want, "{src:?}");
        }
    }

    /// A `namespace eval ::a { helper }` body calls `::a::helper`, never a
    /// same-named proc in the *caller's* namespace (tclsh8.6-confirmed).
    ///
    /// The body is scanned once, as the properly-namespaced body unit lowering
    /// registers for it. Walking it a second time through the enclosing
    /// statement's `ArgRole::Body` — with the caller's namespace — invents a
    /// call to whatever `::helper` happens to exist. Within one file that was
    /// invisible (a bare global `::helper` is rarely in a single file's own
    /// `known` set); across a project it is a false edge into *another file's*
    /// procedure, which is how it surfaced (issue #977).
    #[test]
    fn a_namespace_eval_body_resolves_against_its_own_namespace() {
        let reg = registry();
        let src = "namespace eval ::a {\n    proc helper {} { return 1 }\n    proc run {} { helper }\n}\n";
        // `::helper` is in scope project-wide, as a global proc in some other
        // file would be — the call inside `::a` must not reach it.
        let evidence = scan_source_call_sites(
            src,
            &reg,
            "tcl8.6",
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

    /// `catch { … }` does *not* shift namespace, so its body must still be
    /// walked with the caller's — the guard above keys on an absolute
    /// `ArgRole::Name`, which `catch` has none of.
    #[test]
    fn a_catch_body_is_still_walked_with_the_callers_namespace() {
        let reg = registry();
        let evidence = scan_source_call_sites(
            "proc helper {mode} { return $mode }\ncatch { helper prod }\n",
            &reg,
            "tcl8.6",
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
        let evidence = scan_source_call_sites("a 1\nb 2\n", &reg, "", &known(&["::a", "::b"]), &[]);
        let sliced = evidence.slice_for(["::a"].into_iter());
        assert_eq!(sliced.callees().collect::<Vec<_>>(), vec!["::a"]);
        assert!(evidence.slice_for(["::zzz"].into_iter()).is_empty());
    }

    /// Build the evidence for `src` under a named dialect's registry, the
    /// way `CompilationUnit` does minus the method / body-unit callers
    /// (which need a CFG context these unit tests do not exercise).
    fn evidence_for_dialect(src: &str, dialect: &str) -> CallSiteEvidence {
        let reg = tcl_registry::registry_for_dialect(dialect);
        let ir = crate::lowering::lower_to_ir(src, reg);
        let cfg_module = crate::cfg_builder::build_cfg(&ir, false);
        collect_call_site_constants(
            &cfg_module,
            &[],
            &ir.procedures,
            &ir.namespace_imports,
            reg,
            dialect,
        )
    }

    fn evidence(src: &str) -> CallSiteEvidence {
        evidence_for_dialect(src, "tcl")
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

    // Issue #1044 — a module's own `unknown` handler. Tcl dispatches every
    // unresolved command word to it, so its direct callers are never its
    // complete caller set. tclsh8.6/9.0-confirmed: with `proc unknown {cmd
    // args}` in scope, `bogus beta gamma` runs the handler with
    // `cmd` = `bogus`, `args` = `beta gamma`.

    #[test]
    fn an_unresolved_word_is_a_call_site_of_the_modules_unknown_handler_1044() {
        // TP, the issue's repro: `bogus` names nothing, so real Tcl calls
        // the handler with `bogus`. Seeing only the two `unknown alpha`
        // calls, the scan bound `cmd` to the constant `"alpha"` and folded
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
    fn a_handler_with_agreeing_callers_and_no_unresolved_words_still_seeds_1044() {
        // TN: the gate must not blanket-disable seeding for any module that
        // happens to define a handler — with every caller agreeing and no
        // unresolved word anywhere, the seed still stands.
        let ev = evidence(
            "proc unknown {cmd args} { return $cmd }\nunknown alpha\nunknown alpha\nputs hi\n",
        );
        assert_eq!(
            uniform(&ev, "::unknown", 0).as_deref(),
            Some("alpha"),
            "no unresolved word exists, so the direct callers really are all of them",
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
            "irules",
        );
        assert_eq!(uniform(&ev, "::helper", 0).as_deref(), Some("dev"));
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

    /// Issue #978's reported shape: `trace add variable v write cb`. The
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
            let opaque = scan_source_call_sites(src, &reg, "tcl", &known, &linked);
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
            "tcl",
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
        let ev = evidence("proc helper {a b} { return $a }\nhelper one\n");
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
            "proc helper {mode} { return $mode }\nset s {helper dev}\neval $s\n",
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
        assert!(word_is_whole_substitution("$script"));
        assert!(word_is_whole_substitution("${script}"));
        assert!(word_is_whole_substitution("[build]"));
        assert!(
            !word_is_whole_substitution("puts $v"),
            "a braced body that merely mentions a variable is literal script text",
        );
        assert!(!word_is_whole_substitution("helper dev"));
    }

    #[test]
    fn command_prefix_head_reads_a_built_prefix_through_the_registry() {
        let reg = registry();
        assert_eq!(
            command_prefix_head(&reg, "cmp -nocase").as_deref(),
            Some("cmp"),
        );
        assert_eq!(
            command_prefix_head(&reg, "[list cmp $x]").as_deref(),
            Some("cmp"),
            "`list` carries BUILDS_COMMAND_PREFIX, so its first argument is the head",
        );
        assert_eq!(
            command_prefix_head(&reg, "[pickCallback]"),
            None,
            "some other substitution computed the prefix — the head is unknown",
        );
    }

    #[test]
    fn scope_var_facts_separate_literal_from_unknown_writes() {
        let reg = registry();
        let ir = lower_to_ir(
            "proc p {} {\n set a one\n set b [gets stdin]\n set c two\n set c three\n}\n",
            &reg,
        );
        let cfg_module = build_cfg(&ir, false);
        let cfg = cfg_module.procedures.get("::p").expect("p lowered");
        let facts = collect_scope_var_facts(cfg, &reg);
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
        let ir = lower_to_ir("proc p {n} {\n set a one\n set $n two\n}\n", &reg);
        let cfg_module = build_cfg(&ir, false);
        let cfg = cfg_module.procedures.get("::p").expect("p lowered");
        assert!(collect_scope_var_facts(cfg, &reg).dynamic_name_write);
    }

    #[test]
    fn a_scope_alias_declaration_makes_the_aliased_name_unknown() {
        let reg = registry();
        let ir = lower_to_ir("proc p {} {\n global cmd\n set other one\n}\n", &reg);
        let cfg_module = build_cfg(&ir, false);
        let cfg = cfg_module.procedures.get("::p").expect("p lowered");
        let facts = collect_scope_var_facts(cfg, &reg);
        assert!(
            facts.vars["cmd"].unknown,
            "`global cmd` binds an outer variable any other body may write",
        );
    }
}
