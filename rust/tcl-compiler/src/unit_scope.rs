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
//!    supplies) and records each resolvable call's literal arguments.
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

use tcl_registry::{CommandRegistry, Traits};

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::ir::Module as IrModule;

/// Recursion cap for [`record_call_site_evidence`]'s descent into nested
/// `ArgRole::Body` arguments (`catch { catch { catch { … } } }` and similar) —
/// defensive against a pathological or generated nesting depth; real code
/// never approaches it.
const MAX_CALL_SITE_BODY_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(16);

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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallSiteEvidence {
    by_callee: BTreeMap<String, CalleeEvidence>,
}

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
    caller_qname: &str,
    command: &str,
    args: &[String],
    depth: u32,
) {
    // A dynamically-dispatched command word (`$cmd args`) can't be resolved
    // to any specific callee, so it never counts as a call site — but nor
    // does it disqualify anyone else's: this scan has never claimed to
    // enumerate every indirect dispatch, only every statically-resolvable
    // one (including one level of script-body nesting at a time).
    if !command.contains(['$', '['])
        && let Some(target) = crate::interprocedural::resolve_internal_call(
            command,
            caller_qname,
            ctx.known,
        )
        .or_else(|| {
            resolve_via_namespace_import(command, caller_qname, ctx.namespace_imports, ctx.known)
        })
    {
        out.record_call(target, args);
    }
    record_indirect_callers(out, ctx, caller_qname, command, args);
    if MAX_CALL_SITE_BODY_DEPTH.exceeded(depth + 1) {
        return;
    }
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    for idx in ctx
        .registry
        .arg_indices_for_role(command, &arg_strs, tcl_registry::ArgRole::Body)
    {
        let Some(body_text) = args.get(idx) else {
            continue;
        };
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
            record_call_site_evidence(out, ctx, caller_qname, name, cmd.args(), depth + 1);
        }
    }
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
    caller_qname: &str,
    command: &str,
    args: &[String],
) {
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut poison = |word: &str| {
        if word.is_empty() || word.contains(['$', '[']) {
            return;
        }
        if let Some(target) =
            crate::interprocedural::resolve_internal_call(word, caller_qname, ctx.known)
        {
            out.record_opaque_caller(&target);
        }
    };
    for idx in
        ctx.registry
            .arg_indices_for_role(command, &arg_strs, tcl_registry::ArgRole::CommandPrefix)
    {
        // A command prefix is a list whose first word is the command; the
        // rest are leading arguments the runtime appends to. Only the head
        // names a callee.
        if let Some(head) = args.get(idx).and_then(|p| p.split_whitespace().next()) {
            poison(head);
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
            poison(word);
        }
    }
}

/// Build bare CFGs (no further per-function analysis) for every `TclOO` method
/// and synthetic body unit (`apply` lambda, `namespace eval` body), so
/// [`collect_call_site_constants`] can walk them as *callers* too.
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
/// Returns an empty `Vec` (no cost beyond the emptiness checks) when the
/// module has neither methods nor body units — the overwhelmingly common
/// case — or when `cfg_context` is `None` (methods/body units require it;
/// [`crate::compilation_unit::CompilationUnit`]'s builder only omits it when
/// both are empty).
pub(crate) fn build_extra_call_site_scan_contexts(
    ir_module: &IrModule,
    cfg_context: Option<&crate::cfg_builder::CfgContext>,
) -> Vec<(String, CfgFunction)> {
    if ir_module.methods.is_empty() && ir_module.body_units.is_empty() {
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
    let ctx = CallSiteScanCtx {
        known: &known,
        registry,
        dialect,
        namespace_imports,
    };
    // The top level has no qualified name of its own; `"::top"` (the same
    // pseudo-qname `FunctionUnit::build_full` uses for it) resolves to the
    // global namespace via `resolve_internal_call`'s "drop the last
    // segment" rule, matching a bare top-level call's real resolution
    // scope.
    let funcs = std::iter::once(("::top", &cfg_module.top_level))
        .chain(cfg_module.procedures.iter().map(|(q, f)| (q.as_str(), f)))
        .chain(extra_callers.iter().map(|(q, f)| (q.as_str(), f)));
    let mut out = CallSiteEvidence::default();
    scan_cfg_callers(&mut out, &ctx, funcs);
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
    use crate::ir::Statement;
    for (caller_qname, func) in funcs {
        for block in func.blocks.values() {
            for stmt in &block.statements {
                let (Statement::Call { command, args, .. }
                | Statement::Barrier { command, args, .. }) = stmt
                else {
                    continue;
                };
                record_call_site_evidence(out, ctx, caller_qname, command, args, 0);
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
    let cfg_context = (!ir_module.methods.is_empty() || !ir_module.body_units.is_empty())
        .then(|| crate::cfg_builder::prepare_cfg_context(&ir_module));
    let extra = build_extra_call_site_scan_contexts(&ir_module, cfg_context.as_ref());
    let ctx = CallSiteScanCtx {
        known,
        registry,
        dialect,
        namespace_imports: &ir_module.namespace_imports,
    };
    let funcs = std::iter::once(("::top", &cfg_module.top_level))
        .chain(cfg_module.procedures.iter().map(|(q, f)| (q.as_str(), f)))
        .chain(extra.iter().map(|(q, f)| (q.as_str(), f)));
    scan_cfg_callers(&mut out, &ctx, funcs);
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

    let mut visit = |command: &str, args: &[String]| {
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        found |= registry.unit_linkage(command, &arg_strs, dialect_set);
    };
    walk_script(&ir_module.top_level, &mut visit);
    for proc in ir_module.procedures.values() {
        walk_script(&proc.body, &mut visit);
    }
    for method in ir_module.methods.values() {
        walk_script(&method.body, &mut visit);
    }
    for unit in ir_module.body_units.values() {
        walk_script(&unit.body, &mut visit);
    }
    found
}

/// Visit every `Call` / `Barrier` statement in `script`, descending into every
/// nested control-flow body.  Written as a module-level pair with
/// [`walk_statement`] rather than nested inside [`scan_unit_linkage`] so the
/// mutual recursion reads as two ordinary functions.
fn walk_script(script: &crate::ir::Script, visit: &mut impl FnMut(&str, &[String])) {
    for stmt in &script.statements {
        walk_statement(stmt, visit);
    }
}

/// The [`walk_script`] half that dispatches one statement.
fn walk_statement(stmt: &crate::ir::Statement, visit: &mut impl FnMut(&str, &[String])) {
    use crate::ir::Statement;
    match stmt {
        Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
            visit(command, args);
        }
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
        Statement::AssignConst { .. }
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
/// A call site dispatched through a non-literal command word (`$cmd args`)
/// is deliberately NOT treated as a module-wide wildcard here: it can't be
/// resolved to any specific callee, so the scans already never count it as
/// evidence for (or against) any particular proc's params, the same way they
/// have always treated any other unresolvable call.
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
            let evidence = scan_source_call_sites(src, &reg, "", &known(&["::helper"]));
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
        let evidence = scan_source_call_sites("a 1\nb 2\n", &reg, "", &known(&["::a", "::b"]));
        let sliced = evidence.slice_for(["::a"].into_iter());
        assert_eq!(sliced.callees().collect::<Vec<_>>(), vec!["::a"]);
        assert!(evidence.slice_for(["::zzz"].into_iter()).is_empty());
    }
}
