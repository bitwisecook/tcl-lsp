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

//! Diagnostic-emission orchestrator.
//!
//! Three top-level methods:
//!
//! - [`Analyser::emit_variable_usage_diagnostics`] — a
//!   no-op hook for scope-tree consumers (W211 is emitted by the
//!   SSA-based pass instead).
//! - [`Analyser::emit_cfg_ssa_diagnostics`] — main entry; builds
//!   a [`crate::compilation_unit::CompilationUnit`] on demand, walks the top-level
//!   function and every procedure, dispatches per-function
//!   diagnostics, and runs the cross-function post-passes
//!   (var-as-command, interpolated-command resolution).
//! - [`Analyser::emit_cfg_ssa_diagnostics_for_function`] —
//!   per-function dispatcher; calls each emitter in
//!   declaration order.
//!
//! Two utility passes round things out:
//!
//! - [`Analyser::dedupe_diagnostics`] — drop exact duplicates
//!   plus the line-based pairs (E002 swallowed by E101 on the
//!   same line; W122 swallowed by W124 on the same line).
//! - [`Analyser::apply_disabled_diagnostics`] — filter out
//!   codes the caller asked to silence.
//!
//! The per-function dispatcher wires up the following emitters:
//!
//! - Variable lifecycle: W220 (dead store), W211 (unused
//!   variable), W214 (unused parameter), W210 (read-before-set),
//!   W213 (unset on possibly-undef), and H300 (paste error).
//!   W210 / W213 are gated on procs only.
//! - Var-as-command: **W307** (non-literal command name) and
//!   **W308** (unknown method on object) both emit via the
//!   cross-function post-pass. W308 uses
//!   ``ClassHierarchy::method_target`` for MRO-aware method
//!   resolution, with all the suppression paths wired (inherited
//!   ``unknown`` handler, external superclass, ``oo::objdefine``
//!   per-instance methods).
//! - Unknown commands: **W123** is wired via the cross-function
//!   post-pass; ``command_invocations`` are recorded for every
//!   command head during the walk dispatch.
//! - Branches and channels: I230 / I231 (constant branch /
//!   switch-arm) and W126 (channel argument validation) all wired
//!   through the per-function dispatcher. Info-severity diagnostics
//!   map to ``Severity::Hint`` (there is no Info variant here).
//! - IP literals: W124 (invalid IP address literal) — IPv4 octet
//!   validation (over-255 → Error, leading-zero → Warning) and
//!   IPv6 parsing via ``std::net::Ipv6Addr``. Anchors at the SSA
//!   def site; seen-offsets dedup avoids duplicates across SSA
//!   versions.

use std::collections::HashSet;
use tcl_core_types::DiagCode;

use rustc_hash::FxHashSet;

use helpers::{
    build_undef_suppression, collect_defined_vars, collect_existence_guards, globals_read_by_procs,
    globals_written_by_procs,
};

use super::state::Analyser;
use super::types::Severity;

// Re-export the sibling analyser modules the family submodules reference by
// relative path (`super::types::Diagnostic`, `super::utils::…`, …) so those
// references resolve from `analyser::diagnostics::<family>`.
pub(super) use super::{class_hierarchy, confusables_table, dispatch, state, types, utils};

// Re-export the family helpers exercised by this module's unit tests so the
// `tests` submodule reaches them through its `use super::*`.
#[cfg(test)]
pub(in crate::analyser::diagnostics) use dataflow::{body_references_param, find_ipv6_candidates};
#[cfg(test)]
pub(in crate::analyser::diagnostics) use helpers::find_dotted_quads;
#[cfg(test)]
pub(in crate::analyser::diagnostics) use security::has_redos_shape;
#[cfg(test)]
pub(in crate::analyser::diagnostics) use usage::{
    first_nested_expr, is_benign_unicode, is_safe_literal, is_safe_literal_expr,
    is_valid_subnet_mask, looks_like_subnet_mask, nearest_valid_mask,
};
#[cfg(test)]
pub(in crate::analyser::diagnostics) use validity::contains_gated_word;
pub(in crate::analyser) use validity::{
    emit_invalid_formal_parameter_list_diagnostics, emit_invalid_lambda_parameter_list_diagnostics,
};

// The W110 operator-anchor selector is consumed by the EXPR-argument
// dispatch in `crate::analyser::commands`.
pub(in crate::analyser) use usage::W110Anchor;

mod const_dispatch;
mod dataflow;
pub(in crate::analyser) mod helpers;
mod security;
mod unresolved;
mod usage;
mod validity;
mod var_command;
pub(in crate::analyser) mod version_gate;
pub(in crate::analyser) mod widget_command;

/// Whether a command head names something **this compilation unit can
/// resolve** — a registry command the active dialect enables, or a
/// definition the document makes itself.
///
/// The complement is what matters: a head this answers `false` for is a
/// callee whose body is simply not here (the ticklecharts layout of issue
/// #923 audit idx 59 — the helper in `utils.tcl`, the caller in
/// `options.tcl`), so nothing in this unit can say whether it writes the
/// caller's frame through `upvar`.
/// [`crate::interprocedural::collect_opaque_callee_name_args`] turns that
/// into the per-frame abstention the read-before-set emitters honour.
///
/// Resolution mirrors Tcl's own: an absolute head is looked up as written,
/// a relative one may name a definition in any namespace this document
/// declares (the leaf spellings), and the registry half goes through the
/// dialect profile — exactly the availability query
/// [`Analyser::build_w123_known_names`](super::state::Analyser) uses, so
/// "unknown command" and "opaque callee" cannot disagree about the same
/// head.  No command name appears here.
struct UnitCommandResolver<'a> {
    registry: &'a tcl_registry::CommandRegistry,
    generation: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    /// Every spelling under which this document's own definitions —
    /// procedures, classes, `interp alias` / `rename` targets, declared
    /// stubs, and created object-instance commands — can be called.
    defined: HashSet<String>,
}

impl UnitCommandResolver<'_> {
    fn resolves(&self, command: &str) -> bool {
        if self.defined.contains(command) {
            return true;
        }
        let bare = command.trim_start_matches(':');
        if self.defined.contains(bare) {
            return true;
        }
        self.generation
            .context()
            .resolve_spec(self.registry, command)
            .is_some()
    }
}

impl Analyser {
    /// Build the [`UnitCommandResolver`] for the document just walked.
    fn unit_command_resolver<'a>(
        &self,
        registry: &'a tcl_registry::CommandRegistry,
    ) -> UnitCommandResolver<'a> {
        let mut defined: HashSet<String> = HashSet::new();
        let mut add = |name: &str| {
            let bare = name.trim_start_matches(':');
            if bare.is_empty() {
                return;
            }
            defined.insert(name.to_owned());
            defined.insert(bare.to_owned());
            if let Some(leaf) = bare.rsplit("::").next() {
                defined.insert(leaf.to_owned());
            }
        };
        for name in self.result.all_procs.keys() {
            add(name);
        }
        for name in self.result.all_classes.keys() {
            add(name);
        }
        for name in self.result.command_aliases.keys() {
            add(name);
        }
        for name in self.result.renamed_commands.keys() {
            add(name);
        }
        for stub in &self.result.stub_commands {
            add(&stub.name);
        }
        if let Some(declared) = &self.declared_commands {
            for (name, _) in declared.iter() {
                add(name);
            }
        }
        for name in &self.result.created_instance_commands {
            add(name);
        }
        UnitCommandResolver {
            registry,
            generation: self.analysis_context(),
            defined,
        }
    }
}

/// Which IR object owns the body the per-function dispatcher is running over.
///
/// Supplied by the caller: every diagnostic loop iterates exactly one of the
/// compilation unit's body maps (`procedures`, `methods`, `body_units`) and
/// therefore already knows the kind.  It must **not** be re-derived by
/// probing those maps for the unit's qualified name, because a `TclOO` method
/// and a namespace procedure may legitimately carry the same key.  Verified
/// on tclsh 9.0.4 and 8.6.14 — `oo::class create C` does not create a
/// namespace, so the procedure needs one made first, after which both exist
/// as wholly separate entities:
///
/// ```tcl
/// oo::class create C { variable x; constructor {} { set x 42 }
///                      method m {p} { list method [info exists p] $p $x } }
/// namespace eval ::C {}
/// proc ::C::m {q} { list proc [info exists q] $q [info exists x] }
/// [C new] m hello    ;# → method 1 hello 42
/// ::C::m world       ;# → proc 1 world 0
/// ```
///
/// A name probe would hand the method the procedure's parameters (so its own
/// `[info exists p]` folds "always false") and hand the procedure the class's
/// instance variables (so it abstains on names it fully owns).
#[derive(Clone, Copy, Default)]
pub enum BodyFrame<'a> {
    /// The compilation unit's top level — no parameters, no object state.
    #[default]
    TopLevel,
    /// A named procedure, an `apply` lambda body, or a `namespace eval` body:
    /// a fresh frame whose bound-at-entry names are its formal parameters.
    Procedure(&'a crate::ir::Procedure),
    /// A `TclOO` method body: its own parameters *plus* the class's instance
    /// variables, which the runtime links into the frame at entry.
    Method(&'a crate::ir::MethodDef),
}

/// The two connection-scope name sets an iRule event handler inherits, as
/// `(dead-store / unused suppression, read-before-set suppression)`.
///
/// The first is `cross_event_defs | cross_event_imports`: a variable another
/// event on the same connection reads is not a dead store here.  The second is
/// `cross_event_imports` alone: a variable another event *defines* (and the
/// event registry's scope gate accepted the pair) is already set by the time
/// this event reads it — `set g 1` in `HTTP_REQUEST` feeds `set x $g` in
/// `HTTP_RESPONSE`, so that read is not read-before-set.
///
/// Both are empty for anything that is not a `::when::*` procedure, and for
/// any unit with no connection scope at all.
fn when_proc_cross_event_names(
    cu: &crate::compilation_unit::CompilationUnit,
    qname: &str,
) -> (HashSet<String>, HashSet<String>) {
    let Some(scope) = cu.connection_scope.as_ref().filter(|_| {
        // `crate::ir::when_event_name`'s prefix — an iRule event handler.
        qname.starts_with("::when::")
    }) else {
        return (HashSet::new(), HashSet::new());
    };
    (
        scope
            .cross_event_defs
            .iter()
            .chain(scope.cross_event_imports.iter())
            .cloned()
            .collect(),
        scope.cross_event_imports.iter().cloned().collect(),
    )
}

impl<'a> BodyFrame<'a> {
    /// Whether this is the document's initial global frame. Startup bindings
    /// apply only here; procedure and method locals with the same name remain
    /// ordinary fresh Tcl variables.
    #[must_use]
    fn is_initial_global(self) -> bool {
        matches!(self, Self::TopLevel)
    }

    /// The [`crate::ir::Procedure`] backing this frame, or `None` when the
    /// body is not a procedure-shaped one.  The parameter-specific emitters
    /// (W214 unused-parameter, and the W210 read-before-set parameter
    /// suppression) key off this.
    #[must_use]
    fn procedure(self) -> Option<&'a crate::ir::Procedure> {
        match self {
            Self::Procedure(p) => Some(p),
            Self::TopLevel | Self::Method(_) => None,
        }
    }

    /// The entry facts the `[info exists]` fold needs (issue #1129): the
    /// frame's formal parameters, plus a method's instance variables.
    #[must_use]
    fn existence_frame(self) -> crate::sccp::ExistenceFrame<'a> {
        match self {
            Self::TopLevel => crate::sccp::ExistenceFrame {
                params: &[],
                object_state: None,
                initial_global: true,
            },
            Self::Procedure(p) => crate::sccp::ExistenceFrame {
                params: &p.params,
                object_state: None,
                initial_global: false,
            },
            Self::Method(m) => crate::sccp::ExistenceFrame {
                params: &m.params,
                object_state: Some(&m.instance_vars),
                initial_global: false,
            },
        }
    }
}

impl Analyser {
    /// Scope-tree-driven variable diagnostic emitter.
    ///
    /// An empty hook: W211 (unused-variable) is emitted by the
    /// SSA-based pass in `emit_cfg_ssa_diagnostics_for_function`.
    /// The hook is preserved so future scope-tree-driven emitters
    /// (none currently planned) have a target.
    pub fn emit_variable_usage_diagnostics(&mut self) {
        // Intentionally empty — see module docstring.
    }

    /// CFG/SSA-backed diagnostic orchestrator.
    ///
    /// Builds a
    /// [`crate::compilation_unit::CompilationUnit`] for `source`,
    /// then walks the top-level + every procedure, dispatching
    /// per-function emitters.
    pub fn emit_cfg_ssa_diagnostics(&mut self, source: &str) {
        let generation = self.analysis_context();
        let registry = generation.commands().as_ref();
        // Seed each proc's SCCP with caller-side parameter constants so a
        // branch on a param every caller passes the same literal folds (the
        // `if {$x}` body is provably taken under uniform `q 1` callers, so a
        // var set only there is not read-before-set).
        // Incremental seam: when the per-item path has supplied a unit whose
        // per-function lattices were memoised, consume it instead of
        // rebuilding the whole-file unit.  Equal by construction to the
        // freshly-built unit.
        if let Some(cu) = self.cu_override.take() {
            self.emit_cfg_ssa_diagnostics_with_cu(&cu, registry);
            return;
        }
        // The profile name is `&'static str`, so no borrow of `self` is held
        // across the firewall closure below (which needs `&mut self`).
        let dialect_owned: Option<String> =
            (!self.dialect().is_empty()).then(|| self.dialect().to_string());
        // AN-H1: firewall the lowering→CFG→SSA→interprocedural build (and the
        // emission that consumes it). A panic on adversarial input is contained
        // to "no CFG/SSA diagnostics for this document" instead of crashing the
        // whole document's diagnostics — the same conservative containment the
        // `unknown`-proc lowering path uses (`oo.rs`). (Deep-nesting stack
        // overflow is separately bounded by the lowering depth guards;
        // `catch_unwind` cannot contain a SIGABRT.)
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let dialect_opt = dialect_owned
                .as_deref()
                .and_then(tcl_dialect::DialectProfile::find);
            // Build under the analyser's own dialect, not a blind default: the
            // lowering needs it to parse a dialect-only operator (an iRules
            // `contains` condition) as an operator, and the lattice pipeline
            // needs it to fold one.
            //
            // The *lexer* config is the document's own environment grammar,
            // and every host that supplies this unit through the
            // `cu_override` seam (`tcl diag`'s `collect_rows`,
            // `tcl_lsp_db::analyse_per_item_with`, `xtask fp_sweep`) builds it
            // the same way, so the supplied unit is the one this branch would
            // have built. It used to be `LexerConfig::default()` on all four —
            // agreeing, but wrong: an 8.x document lexed `${a{b}c}` under the
            // 9.0 close rule and decoded escapes as 9.0, and an iRules
            // document lexed with `{*}` expansion and no F5 word break
            // (redesign §11.4 row E1, §9.1 defect 1).
            let cu = crate::compilation_unit::CompilationUnit::build_with_options(
                source,
                crate::compilation_unit::UnitBuildOptions {
                    registry,
                    defer_top_level: false,
                    config: tcl_lexer::LexerConfig::for_dialect(
                        dialect_owned.as_deref().unwrap_or_default(),
                    ),
                    dialect: dialect_opt,
                    external_call_sites: None,
                },
            )
            .with_interprocedural(registry, dialect_opt);
            self.emit_cfg_ssa_diagnostics_with_cu(&cu, registry);
        }));
    }

    /// The compilation-unit-derived object-fact seam, run before any emitter
    /// that reads those facts.
    ///
    /// Issue #923 idx 121: the pending `$class`-headed `TclOO`
    /// instance-creation sites must settle against `cu`'s flow-sensitive value
    /// model **first** — `emit_var_command_diagnostics` reads
    /// `instance_classes` to suppress W307 / validate W308, so the settle must
    /// land before that read, not after (unlike `settle_const_dispatches`,
    /// which only feeds `command_invocations` and has no such in-pass reader).
    fn settle_cu_derived_object_facts(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        self.settle_pending_instance_class_sites(cu);
        self.produce_object_handle_facts(cu, registry);
    }

    /// Issue #994 (C5a): the **one** producer of
    /// [`crate::analyser::types::AnalysisResult::object_handle_facts`], from
    /// the same `cu` the CFG/SSA diagnostics ride on.
    ///
    /// Both analysis paths reach it — the whole-file
    /// [`Self::emit_cfg_ssa_diagnostics`] build and the per-item incremental
    /// path's memoised shell unit — so the fact is produced exactly once per
    /// analysis and never merged per grafted body (`analyser/per_item.rs`
    /// pins that with the same reasoning).
    fn produce_object_handle_facts(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        self.result.object_handle_facts = crate::object_types::object_handle_facts(cu, registry);
    }

    /// Emit the CFG/SSA-derived diagnostics from an already-built
    /// [`crate::compilation_unit::CompilationUnit`].
    ///
    /// Split out of [`Self::emit_cfg_ssa_diagnostics`] so the incremental
    /// per-item path can supply a `CompilationUnit` whose per-function
    /// lattices were memoised, instead of rebuilding the whole-file unit on
    /// every edit.  Behaviour is identical: the whole-file entry point builds
    /// the unit exactly as before and delegates here, and every cross-function
    /// pass below reads the supplied unit unchanged.
    pub fn emit_cfg_ssa_diagnostics_with_cu(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        self.settle_cu_derived_object_facts(cu, registry);

        // **W128.** Flag calls to commands renamed or
        // deleted earlier in the file via the flow-sensitive
        // command-binding lattice.  Independent of the CFG/SSA dead-store
        // machinery below, so run it up front against the same `cu`.
        self.emit_w128_renamed_command(cu, registry);

        // Compute the set of globals any
        // proc in this module writes to.  Top-level RBS (W210)
        // is suppressed for these variables — a helper proc may
        // populate them before the top-level read fires.
        let globals_written = globals_written_by_procs(cu);

        // **FP-DS-04 cross-scope traces.** A `::`-qualified global with a write
        // trace anywhere in the module is observable across scopes, so a
        // `set ::w 1` in one proc is neither a dead store (W220) nor unused
        // (W211) even when the `trace add variable ::w …` lives elsewhere. The
        // per-function `scan_scope_aliases` only sees a function's own traces;
        // fold the module-wide traced globals into every function's
        // suppression context (which already covers both W211 and W220).
        let mut traced_globals =
            crate::optimiser::elimination::scan_module_traced_globals(cu, registry);
        // The registry-driven whole-module fact stores the canonical
        // (`::`-stripped) spelling, so an *unqualified* top-level store
        // (`set g 1`, chain key `g`) is also suppressed when the trace
        // names `::g` (issue #1377) — the same fact SCCP and the O102 /
        // O109 gates consult.
        traced_globals.extend(cu.ir_module.traced_variables.iter().cloned());

        // **W220 call-by-name suppression.** Build the
        // interprocedural proc-index once so a caller-local passed *by
        // name* to a proc that consumes it via `upvar` (`set tag "";
        // asnPeekTag data tag type dummy`) is not flagged as a dead
        // store.  `collect_call_by_name_reads` then yields the suppressed
        // names per function, merged into the dead-store `cross_event_vars`.
        let cbn_proc_index = {
            // The call-by-name proc index (W220) needs only direct proc→proc
            // reachability, not object-instance callback edges, so no
            // object-type map is threaded here.
            let ia = crate::interprocedural::build_interprocedural_analysis(
                &cu.ir_module,
                registry,
                Some(self.profile),
                crate::interprocedural::ObjectTypeMap::none(),
                &self.head_identities,
            );
            crate::interprocedural::build_proc_index_from_summaries(&ia)
        };

        // pkgIndex.tcl files have ``$dir`` set by the package
        // loader before the script body runs — suppress dead-
        // store / unused-variable diagnostics for it at the
        // top-level.  Match the *basename* exactly (not a suffix):
        // ``ends_with("pkgIndex.tcl")`` would also swallow a file
        // literally named ``notpkgIndex.tcl``.
        let pkgindex_implicit_vars: HashSet<String> =
            if self.file_path.as_deref().is_some_and(|p| {
                std::path::Path::new(p)
                    .file_name()
                    .is_some_and(|n| n == "pkgIndex.tcl")
            }) {
                HashSet::from(["dir".to_string()])
            } else {
                HashSet::new()
            };
        let mut top_level_cross_event_vars: HashSet<String> = pkgindex_implicit_vars.clone();
        top_level_cross_event_vars.extend(crate::interprocedural::collect_call_by_name_reads(
            &cu.top_level.cfg,
            &cbn_proc_index,
        ));
        top_level_cross_event_vars.extend(traced_globals.iter().cloned());
        // A global a helper proc *reads* (`proc f {} { global cfg; return $cfg
        // }` or `$::cfg`) consumes the top-level `set cfg …` that runs in the
        // shared global namespace — the read-side mirror of `globals_written`
        // above. Fold those names in so the top-level assignment is neither a
        // dead store (W220) nor an unused variable (W211): both emitters honour
        // this set (W211 via `textually_referenced.extend(cross_event_vars)`).
        top_level_cross_event_vars.extend(globals_read_by_procs(cu));

        // pkgIndex.tcl's ``$dir`` is set by the package loader before the
        // index script runs, so a read of it is not read-before-set (W210).
        // `cross_event_vars` only reaches the dead-store / unused checks, so
        // fold the implicit set into `extra_known_defined` too — that is the
        // argument the W210 emitters consult (#955).
        let mut top_level_known_defined: HashSet<String> = if pkgindex_implicit_vars.is_empty() {
            globals_written.clone()
        } else {
            globals_written
                .iter()
                .chain(pkgindex_implicit_vars.iter())
                .cloned()
                .collect()
        };

        // **W210 opaque-callee abstention (issue #923 audit idx 59).** A call
        // to a command whose body this unit does not hold — the cross-file
        // helper the finding was mined from — may create any caller-frame
        // name it is handed, so the names it *spells* stop being provably
        // unset here.  Per frame, and per name: everything else in the frame
        // still reports.
        let unit_commands = self.unit_command_resolver(registry);
        let opaque_callee_defs = |fu: &crate::compilation_unit::FunctionUnit| {
            crate::interprocedural::collect_opaque_callee_name_args(&fu.cfg, &|cmd: &str| {
                unit_commands.resolves(cmd)
            })
        };
        top_level_known_defined.extend(opaque_callee_defs(&cu.top_level));

        // Top-level first, then procedures in insertion order —
        // matches the iteration order of
        // ``CompilationUnit::functions``.
        // Iterate top-level explicitly so we can pass the IR
        // module through.
        self.emit_cfg_ssa_diagnostics_for_function_full(
            &cu.top_level,
            BodyFrame::TopLevel,
            &top_level_known_defined,
            &top_level_cross_event_vars,
        );
        self.emit_channel_diagnostics(&cu.top_level, registry);
        for (qname, fu) in &cu.procedures {
            // For ``::when::*`` procs, threaded
            // ``cross_event_defs | cross_event_imports`` from the
            // ConnectionScope so dead-store / unused-variable
            // diagnostics suppress vars that may be read in a
            // different iRule event.
            let (mut cross_event_vars, mut extra_known_defined) =
                when_proc_cross_event_names(cu, qname);
            extra_known_defined.extend(opaque_callee_defs(fu));
            // Suppress dead-store on caller-locals this
            // proc passes by name to an upvar callee.
            cross_event_vars.extend(crate::interprocedural::collect_call_by_name_reads(
                &fu.cfg,
                &cbn_proc_index,
            ));
            cross_event_vars.extend(traced_globals.iter().cloned());
            self.emit_cfg_ssa_diagnostics_for_function_full(
                fu,
                cu.ir_module
                    .procedures
                    .get(qname)
                    .map_or(BodyFrame::TopLevel, BodyFrame::Procedure),
                &extra_known_defined,
                &cross_event_vars,
            );
            self.emit_channel_diagnostics(fu, registry);
            // IRULE4005 — racy ``static::``
            // cross-event flow.  Only fires for non-RULE_INIT
            // ``when`` procs when ``ConnectionScope::racy_static_defs``
            // is non-empty.
            if let Some(scope) = cu.connection_scope.as_ref()
                && qname.starts_with("::when::")
                && !scope.racy_static_defs.is_empty()
            {
                let event = crate::ir::when_event_name(qname);
                if event != "RULE_INIT" {
                    self.emit_racy_static_diagnostics(fu, &scope.racy_static_defs);
                }
            }
        }

        self.emit_fresh_frame_body_diagnostics(
            cu,
            registry,
            &cbn_proc_index,
            &traced_globals,
            &unit_commands,
        );

        // Cross-function post-pass: resolve $var-as-command sites
        // collected during the walk.
        self.emit_var_command_diagnostics(cu, registry);

        // W250 — instantiating an `oo::abstract` class.
        self.emit_abstract_instantiation_diagnostics(cu);

        // Suppress W123 for command-name
        // heads with partial interpolations like ``foo$suffix``
        // when ``$suffix`` resolves cleanly to a finite set of
        // known commands via SCCP.
        self.resolve_interpolated_w123_diagnostics(cu);

        // M7 settlement (issue #945 faults 1–2): resolve the constant-
        // `$cmd` dispatch sites against the flow-sensitive value model,
        // emitting the indirect head references and their writable
        // literal-anchored twins.
        self.settle_const_dispatches(cu);
    }

    /// `TclOO`/snit method bodies (issue #923 idx 77, main audit wave, high
    /// severity): `cu.methods` is kept in a *separate* map from
    /// `cu.procedures` precisely so [`Self::emit_cfg_ssa_diagnostics_with_cu`]'s
    /// procs loop was historically unaffected by their addition (see
    /// [`crate::compilation_unit::CompilationUnit::methods`]'s own doc) — but
    /// that meant the entire CFG/SSA dataflow family (W210 read-before-set
    /// and siblings) silently never ran on any method body at all, a
    /// systemic false-negative gap: tomato's real `Vector3d.tcl::* {type}`
    /// reads `$other` (a variable belonging to a *sibling* method, never
    /// bound in `*`'s own scope) and crashes at runtime the moment it's
    /// called with an object operand (tclsh8.6/9.0.4-verified) — the exact
    /// same unbound-read shape inside a plain `proc` already fires W210
    /// twice, but zero diagnostics fired here. No `::when::`/`ConnectionScope`
    /// handling needed — a method's qualified name (`{class}::{method}`)
    /// never has that prefix, so every one of the procs loop's
    /// iRule-specific branches would always take their empty-set arm
    /// anyway.
    fn emit_method_body_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
        cbn_proc_index: &crate::interprocedural::ProcIndex,
        traced_globals: &HashSet<String>,
        unit_commands: &UnitCommandResolver<'_>,
    ) {
        for (qname, fu) in &cu.methods {
            let method_ir = cu.ir_module.methods.get(qname);
            // Known-bound-at-entry names for this method: its own params
            // *plus* `MethodDef::instance_vars` (class-level `variable`
            // declarations + the method's own — TclOO auto-binds these in
            // every method's scope with no visible `variable` statement in
            // the body itself). Without instance vars, naively running W210
            // here would flood false positives on every ordinary
            // instance-variable read; without params,
            // `emit_read_before_set_diagnostics` / `emit_return_phi_undef_w210`
            // would *also* flood false positives on the method's own
            // parameters — both special-case a real parameter via a
            // *separate* `ir_proc.params` lookup keyed by
            // `ir_module.procedures`, which a method's qualified name is
            // never in (verified empirically: a throwaway probe against
            // `method DotProduct {other} { ... $other ... }` flagged
            // `other` — the method's own, used parameter — before params
            // were folded in here too). Both emitters already consult
            // `extra_known_defined` redundantly alongside `ir_proc.params`
            // for exactly this case, so this is the minimal fix — no need
            // to plumb a second `ir_proc`-style lookup through every
            // consumer. `cross_event_vars` (W211/W220 suppression) needs
            // the same set for a parallel reason: a "setter" method that
            // writes an instance var with no local read is not a dead
            // store, since another method reads it later — mirrors the
            // existing cross-function-global mechanism above
            // (`globals_written` / `cross_event_imports`), just
            // object-instance-scoped instead of interpreter-global-scoped.
            //
            // The set is read off the unit's own `method_facts` — the one
            // carrier `build_for_method` fills — rather than rebuilt from
            // the IR here, so this family and the existence fold (I230 /
            // O100 / O101) cannot source the same fact from different maps
            // (issue #1174; that divergence shape was issue #1129).
            let mut known_bound: HashSet<String> = fu.method_facts.as_deref().map_or_else(
                HashSet::new,
                crate::compilation_unit::MethodBodyFacts::known_bound_at_entry,
            );
            let mut cross_event_vars = known_bound.clone();
            known_bound.extend(crate::interprocedural::collect_opaque_callee_name_args(
                &fu.cfg,
                &|cmd: &str| unit_commands.resolves(cmd),
            ));
            cross_event_vars.extend(crate::interprocedural::collect_call_by_name_reads(
                &fu.cfg,
                cbn_proc_index,
            ));
            cross_event_vars.extend(traced_globals.iter().cloned());
            self.emit_cfg_ssa_diagnostics_for_function_full(
                fu,
                method_ir.map_or(BodyFrame::TopLevel, BodyFrame::Method),
                &known_bound,
                &cross_event_vars,
            );
            self.emit_channel_diagnostics(fu, registry);
        }
    }

    /// The CFG/SSA dataflow family over every body that runs in a **fresh
    /// frame** but is not a named procedure — a `TclOO`/snit method body and
    /// an `apply` lambda body.  Both have a closed set of names bound at
    /// entry, which is what the read-before-set family needs to be sound.
    fn emit_fresh_frame_body_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
        cbn_proc_index: &crate::interprocedural::ProcIndex,
        traced_globals: &HashSet<String>,
        unit_commands: &UnitCommandResolver<'_>,
    ) {
        self.emit_method_body_diagnostics(
            cu,
            registry,
            cbn_proc_index,
            traced_globals,
            unit_commands,
        );
        self.emit_lambda_body_diagnostics(
            cu,
            registry,
            cbn_proc_index,
            traced_globals,
            unit_commands,
        );
    }

    /// The same CFG/SSA dataflow family over an `apply` **lambda body**.
    ///
    /// A lambda is an anonymous procedure: C Tcl gives it a fresh frame whose
    /// only bound names are its parameter list, so a read of anything else is
    /// the same error a `proc` body's unbound read is (tclsh 9.0.4 / 8.6.14,
    /// identical: `set x 7; apply {{} {puts $x}}` →
    /// `can't read "x": no such variable`).  The enclosing frame's scan skips
    /// the lambda literal entirely
    /// ([`crate::ssa::structural_body_indices`]), so this loop is what keeps
    /// the *body's* own genuine unbound reads visible — without it, moving the
    /// literal out of the caller's frame would trade a false positive on the
    /// lambda's parameters for a false negative on its body (issue #1070).
    ///
    /// Restricted to [`crate::ir::Module::lambda_body_units`]: a
    /// `namespace eval` body unit shares its namespace's variables with every
    /// other body that opens it, so it has no closed-frame guarantee to read
    /// this family against.
    fn emit_lambda_body_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
        cbn_proc_index: &crate::interprocedural::ProcIndex,
        traced_globals: &HashSet<String>,
        unit_commands: &UnitCommandResolver<'_>,
    ) {
        for qname in &cu.ir_module.lambda_body_units {
            let (Some(fu), Some(ir_proc)) =
                (cu.body_units.get(qname), cu.ir_module.body_units.get(qname))
            else {
                continue;
            };
            let mut known_bound: HashSet<String> = ir_proc.params.iter().cloned().collect();
            let mut cross_event_vars = known_bound.clone();
            known_bound.extend(crate::interprocedural::collect_opaque_callee_name_args(
                &fu.cfg,
                &|cmd: &str| unit_commands.resolves(cmd),
            ));
            cross_event_vars.extend(crate::interprocedural::collect_call_by_name_reads(
                &fu.cfg,
                cbn_proc_index,
            ));
            cross_event_vars.extend(traced_globals.iter().cloned());
            self.emit_cfg_ssa_diagnostics_for_function_full(
                fu,
                BodyFrame::Procedure(ir_proc),
                &known_bound,
                &cross_event_vars,
            );
            self.emit_channel_diagnostics(fu, registry);
        }
    }

    /// Per-function diagnostic dispatcher.
    ///
    /// Called once for the top-level
    /// script and once per procedure.  Each per-emitter call is
    /// gated on its own predicate inside the helper.
    pub fn emit_cfg_ssa_diagnostics_for_function(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        frame: BodyFrame<'_>,
    ) {
        self.emit_cfg_ssa_diagnostics_for_function_full(
            function_unit,
            frame,
            &HashSet::new(),
            &HashSet::new(),
        );
    }

    /// Per-function diagnostic dispatcher with an extra
    /// "known-defined" set passed through to RBS suppression.
    ///
    /// Same as [`Self::emit_cfg_ssa_diagnostics_for_function`]
    /// but accepts an additional set of variable names that
    /// should be treated as already-defined for the W210
    /// (read-before-set) emitter.  Used at the top-level to
    /// suppress RBS for variables that any proc in the module
    /// writes.
    pub fn emit_cfg_ssa_diagnostics_for_function_with_extra(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        frame: BodyFrame<'_>,
        extra_known_defined: &HashSet<String>,
    ) {
        self.emit_cfg_ssa_diagnostics_for_function_full(
            function_unit,
            frame,
            extra_known_defined,
            &HashSet::new(),
        );
    }

    /// Per-function diagnostic dispatcher with the full
    /// suppression context.
    ///
    /// Adds `cross_event_vars` on top of
    /// [`Self::emit_cfg_ssa_diagnostics_for_function_with_extra`].
    /// Used by the W220 IR-paths port to suppress dead-store
    /// diagnostics for variables a `::when::*` proc may carry
    /// across iRule events (`cu.connection_scope.cross_event_defs
    /// | cross_event_imports`) and for `pkgIndex.tcl` `$dir`,
    /// which the package loader assigns before the script body
    /// runs.
    pub fn emit_cfg_ssa_diagnostics_for_function_full(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        frame: BodyFrame<'_>,
        extra_known_defined: &HashSet<String>,
        cross_event_vars: &HashSet<String>,
    ) {
        let defined = collect_defined_vars(&function_unit.cfg);
        // Alias recognition is registry-driven; fall back to the cached
        // default registry when the analyser has none loaded.
        let scan_registry = self.registry.as_deref().map_or_else(
            || {
                tcl_registry::model::ingress::static_context_for("tcl8.6")
                    .commands()
                    .as_ref()
            },
            |r| r,
        );
        let scope_aliases =
            crate::optimiser::elimination::scan_scope_aliases(&function_unit.cfg, scan_registry);
        let global_aliases = crate::optimiser::elimination::scan_global_scope_aliases(
            &function_unit.cfg,
            scan_registry,
        );
        let mut textually_referenced =
            crate::optimiser::elimination::collect_textual_var_references(
                &self.source,
                &function_unit.cfg,
                function_unit.base_offset,
                self.profile.grammar.braced_var,
            );
        // A var read in another iRule event, or consumed *by name* via a
        // call-by-name upvar callee, is "used" — suppress
        // the unused-variable (W211) hint too, not just the dead store
        // (W220).
        textually_referenced.extend(cross_event_vars.iter().cloned());
        // A read-modify-write command's target buried in a substitution
        // (`lappend r [incr i $j]` reads `i`) keeps a feeding `set i 0` alive —
        // recover those name-level reads so they suppress the dead-store /
        // unused-variable hints.
        if let Some(registry) = self.registry.as_deref() {
            textually_referenced.extend(crate::optimiser::elimination::collect_rmw_hidden_reads(
                function_unit,
                registry,
            ));
        }
        // Frame identity comes from the caller, which knows which of the
        // compilation unit's body maps it is iterating.  It must not be
        // re-derived by probing those maps for `function_unit.name`: a
        // `TclOO` method and a namespace procedure may legitimately share a
        // qualified name, so the name alone cannot tell them apart (see
        // [`BodyFrame`]).
        let ir_proc = frame.procedure();
        let initial_global = frame.is_initial_global();
        let existence_frame = frame.existence_frame();
        self.emit_dead_store_diagnostics(function_unit, &defined, &scope_aliases, cross_event_vars);
        self.emit_unused_variable_diagnostics(
            function_unit,
            &defined,
            &scope_aliases,
            &textually_referenced,
        );
        self.emit_possible_paste_error_diagnostics(function_unit);
        // Shared read-before-set context: the SCCP-executable block set and
        // the name-level suppression (`dict with` keys, qualified-`variable`
        // alias tails, dict vars), threaded through both the version-0
        // statement/branch emitter and the `Terminator::Return` pass.
        let considered: HashSet<crate::cfg::BlockId> =
            if function_unit.sccp.executable_blocks.is_empty() {
                function_unit.ssa.blocks.keys().copied().collect()
            } else {
                function_unit.sccp.executable_blocks.clone()
            };
        let supp = build_undef_suppression(
            function_unit,
            &considered,
            initial_global,
            &global_aliases,
            self.analysis_context().context().authoring_mask(),
        );
        let exists_guards = collect_existence_guards(function_unit);
        let rbs_params: HashSet<&str> = ir_proc
            .map(|p| p.params.iter().map(String::as_str).collect())
            .unwrap_or_default();
        let read_before_set_ctx = dataflow::ReadBeforeSetCtx {
            initial_global,
            global_aliases: &global_aliases,
            defined_vars: &defined,
            scope_aliases: &scope_aliases,
            extra_known_defined,
            supp: &supp,
        };
        self.emit_read_before_set_diagnostics(function_unit, ir_proc, &read_before_set_ctx);
        // Phi-from-undef on `return $v` reads (the def-use builder records
        // statement + branch-condition uses but NOT `Terminator::Return`
        // values).
        self.emit_return_phi_undef_w210(
            function_unit,
            &dataflow::ReturnUndefCtx {
                initial_global,
                global_aliases: &global_aliases,
                dialect: self.analysis_context().context().authoring_mask(),
                params: &rbs_params,
                exists_guards: &exists_guards,
                scope_aliases: &scope_aliases,
                extra_known_defined,
                defined_vars: &defined,
                considered: &considered,
                supp: &supp,
            },
        );
        // W210 on reads of a provably-no-match regexp / scan output var.
        self.emit_provably_unset_w210(function_unit, &considered, &defined);
        self.emit_constant_branch_diagnostics(function_unit);
        self.emit_existence_constant_branch_diagnostics(function_unit, existence_frame);
        self.emit_invalid_ip_diagnostics(function_unit);
        self.emit_w233_divide_by_zero(function_unit);
        self.emit_interval_bounds_diagnostics(function_unit);
        if let Some(ir_proc) = ir_proc {
            self.emit_unused_param_diagnostics(function_unit, ir_proc);
        }
    }

    /// Drop exact-duplicate diagnostics + line-based suppression
    /// pairs.
    ///
    /// Two passes:
    ///
    /// 1. Compute the set of source lines on which `E101`
    ///    (missing-open-brace) fired — a sentinel for the related
    ///    redundant-message code.
    /// 2. Walk diagnostics in source order, deduplicating by
    ///    `(code, span, message, severity)` and dropping `E002` on a
    ///    line where `E101` fired (the recovered switch makes the
    ///    arity message a false positive).
    ///
    /// Lines come from the [`SourceMap`] over `self.source`.
    ///
    /// (Formerly also suppressed `W122` on a line where `W124` fired —
    /// removed with `W122` itself, which duplicated `W124`'s IPv4 octet
    /// check under a superseded code with no independent producer;
    /// issue #1317.)
    pub fn dedupe_diagnostics(&mut self) {
        let sm = Analyser::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let mut e101_lines: FxHashSet<u32> = FxHashSet::default();
        for d in &self.result.diagnostics {
            if d.code.as_str() == "E101" {
                let line = sm.range_positions(d.span).0.line;
                e101_lines.insert(line);
            }
        }

        let mut seen: FxHashSet<(DiagCode, u32, u32, String, Severity)> = FxHashSet::default();
        let drained = std::mem::take(&mut self.result.diagnostics);
        let mut deduped = Vec::with_capacity(drained.len());
        for d in drained {
            let key = (
                d.code,
                d.span.start(),
                d.span.end(),
                d.message.clone(),
                d.severity,
            );
            if seen.contains(&key) {
                continue;
            }
            let line = sm.range_positions(d.span).0.line;
            if d.code == DiagCode::E002 && e101_lines.contains(&line) {
                continue;
            }
            seen.insert(key);
            deduped.push(d);
        }
        self.result.diagnostics = deduped;

        // Canonical, deterministic order. The post-walk emitters
        // (`emit_variable_usage_diagnostics` etc.) iterate the scope tree's
        // `HashMap`s, whose per-instance iteration order is non-deterministic —
        // so emission order varied run-to-run and, critically, between
        // `analyse` and `analyse_commands` (the per-item incremental path).
        // That non-determinism meant the multiset always
        // matched; only the `Vec` order differed. Sorting by source position
        // here makes the output deterministic and path-independent — required
        // for `incremental == fresh`, and a saner source-ordered contract for
        // the LSP. Dedupe above guarantees `(code, start, end, message,
        // severity)` is unique, so this key is a total order (no ties).
        self.result.diagnostics.sort_by(|a, b| {
            a.span
                .start()
                .cmp(&b.span.start())
                .then(a.span.end().cmp(&b.span.end()))
                .then_with(|| a.code.cmp(&b.code))
                .then_with(|| a.severity.as_str().cmp(b.severity.as_str()))
                .then_with(|| a.message.cmp(&b.message))
        });
    }

    /// Filter out diagnostics whose codes are in
    /// [`Self::disabled_diagnostics`].
    ///
    /// Centralising the filter on the orchestrator
    /// side keeps the per-emitter code
    /// from having to thread the check at every emit site —
    /// emitters can push freely and the orchestrator drops the
    /// silenced codes at the end.
    ///
    /// Idempotent on an empty filter set (no allocations).
    pub fn apply_disabled_diagnostics(&mut self) {
        if self.disabled_diagnostics.is_empty() {
            return;
        }
        // Borrow-checker dance: `retain` closure can't capture
        // `&self.disabled_diagnostics` while ``self.result`` is
        // mut-borrowed; clone the set into a local first.  The
        // disabled set is small (LSP-config-scale) so the clone
        // cost is negligible vs. the rest of the diagnostics
        // pipeline.
        let disabled = self.disabled_diagnostics.clone();
        self.result
            .diagnostics
            .retain(|d| !disabled.contains(d.code.as_str()));
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fp;
