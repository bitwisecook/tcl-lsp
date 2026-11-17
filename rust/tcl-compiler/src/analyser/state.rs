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

//! The [`Analyser`] struct and its per-walk state.
//!
//! The analyser is a single struct whose methods are grouped across
//! modules (``commands.rs``, ``handlers.rs``, ``oo.rs``, ``diagnostics/``,
//! …) but all operate on the same ``&mut Analyser``.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_core_types::DiagCode;

use tcl_lexer::Span;

use super::types::AnalysisResult;

mod configuration;
mod full_analysis;
mod post_walk;
mod realms;
mod run_state;
mod segmented_analysis;
mod syntax_recovery;
mod walk_environment;
mod walk_records;

pub(super) use realms::{CompilationUnitOverride, PrebuiltRealm, RealmBuildInputs};
pub use walk_records::{CmdCommandSite, DispatchReceiver, VarCommandSite};
pub(super) use walk_records::{
    ConstDispatchSite, InterpFrame, InterpState, ObjdefineAbortCandidate,
    PendingEnsembleSubcommand, PendingInstanceClassSite, PendingInstanceCreation, SafeInterpCtx,
};

/// Tcl analysis state and the retained inputs for successive document walks.
// False positive: independent pass flags, not mutually exclusive states.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct Analyser {
    pub(super) authenticated_report_request:
        Option<std::sync::Weak<super::authenticated_report::ReportCaptureRequest>>,
    pub(super) resolved_input: Option<super::input::ResolvedAnalysisInput>,
    /// Public accumulator returned by [`Analyser::analyse`].
    pub result: AnalysisResult,
    /// Path through ``result.global_scope`` to the currently-active
    /// scope. Each entry is the index into the parent's
    /// ``children`` list; an empty path means "currently in the
    /// global scope". An index path is used rather than a
    /// back-pointer so the scope tree stays a strict ownership graph.
    pub current_scope_path: Vec<usize>,
    /// Full source text being analysed.
    ///
    /// Set at the top of [`Self::analyse`] (and the chunked
    /// entries) and read by handlers that need to re-slice the outer
    /// source — recovery and CFG/SSA diagnostic emission. A walk holds the
    /// document image's own buffer, so checking an image against this text
    /// compares addresses, not the document.
    pub source: Arc<str>,
    /// Static original words from this walk's genuine complete command
    /// vectors. Missing or conflicting producers never use a display name.
    pub(super) original_static_source_names:
        HashMap<Span, Option<crate::signature_scan::original_name::SourceOriginalNameOccurrence>>,
    /// The resolved dialect profile — the ingest identity
    /// (dialect-profile-model.md §2.4). Set once at the top of
    /// [`Self::analyse`] from the caller's dialect string via
    /// `DialectProfile::by_name` (aliases canonicalise; unknown names sink
    /// to the permissive fallback). Handlers derive every dialect-specific
    /// answer — availability masks, behaviour policies, the lexer grammar —
    /// from this; the original string round-trips as [`Self::dialect`].
    pub profile: &'static tcl_dialect::DialectProfile,
    /// The grammar the **ingress** resolved for this document — the
    /// environment's
    /// [`grammar`](crate::environment_ingress::DocumentEnvironment::grammar),
    /// set beside [`Self::profile`] by [`Self::resolve_walk_environment`].
    /// `None` when the walk was set up from a profile directly (tests, and
    /// any host that hands the analyser a profile rather than a name), in
    /// which case [`Self::grammar`] answers with the profile's own grammar.
    ///
    /// It exists because the two are not always one value: `tk`'s analyser
    /// profile is deliberately the anonymous fallback (see
    /// `DocumentEnvironment::analyser_profile`) while its documents lex
    /// under the `tk` environment's 8.6 core, so a walk that read
    /// `profile.grammar` would lex a `tk` document under 9.x rules.
    pub(super) ingress_grammar: Option<tcl_dialect::LexerGrammar>,
    /// The **unit** profile the ingress resolved — the promoting form a
    /// compilation unit is built under (`tk`'s own row, a projection for
    /// `jim`), set beside [`Self::ingress_grammar`]. The CFG/SSA unit the
    /// diagnostics walk builds takes its dialect from here so it agrees with
    /// the unit the LSP builds for the same document, rather than from a
    /// catalogue lookup of the name (which sinks `jim` and `tk` to the
    /// fallback).
    pub(super) unit_profile: Option<&'static tcl_dialect::DialectProfile>,
    /// The resolved document environment: set beside [`Self::profile`] at
    /// each `analyse*` ingress by
    /// [`crate::environment_ingress::resolve_environment`]. The profile
    /// above is *derived from* this resolution; availability queries go
    /// through [`Self::analysis_context`] rather than the profile.
    pub(super) environment: Option<crate::environment_ingress::DocumentEnvironment>,
    /// The registry generation this walk reads — the per-context
    /// [`tcl_registry::model::ContextRegistry`] carrying both the
    /// availability context and the (possibly pack-overlaid) command
    /// store [`Self::registry`] aliases. Stashed by the `analyse*`
    /// entries; [`Self::analysis_context`] supplies the bare-harness
    /// fallback.
    pub(super) context: Option<std::sync::Arc<tcl_registry::model::ContextRegistry>>,
    /// The identity of the `SpecTcl` pack set layered onto this dialect's
    /// registry — the `PackSet::key` its owner installed under, or `0` for
    /// "no packs".
    ///
    /// A number rather than a registry handle, deliberately: the analyser must
    /// not depend on the pack loader (which depends on *it*), and a `u64`
    /// travels through a salsa input and a config struct with no new edge at
    /// all. [`Self::profile_registry`] turns it into the registry, reading the
    /// un-overlaid one until something has been installed under that key.
    ///
    /// It is load-bearing since the EDA vendor libraries became bundled
    /// `.tclspec` loadables (`docs/design/registry/spec-packs.md`): without it a
    /// Vivado document's every command reads as unknown, because no compiled-in
    /// spec answers for `synth_design` any more.
    pub pack_overlay: u64,
    /// Diagnostic codes that should not be emitted.
    pub disabled_diagnostics: HashSet<String>,
    /// User-declared extra command names (`tclLsp.extraCommands`) treated as
    /// known, so calling them never draws an unknown-command W123.
    ///
    /// Shared behind an `Arc` because the LSP's unclosed-delimiter recovery
    /// path widens this set with every workspace-indexed proc / class and every
    /// auto-loadable command name (`widen_recovery_extra_commands` in
    /// `tcl-lsp-server`) — tens of thousands of names on a large workspace.
    /// That set is a pure function of the workspace index and the package
    /// database, so the server caches it and hands the same allocation to every
    /// analyser it configures; an owned `HashSet` here would deep-copy it on
    /// each keystroke.
    pub extra_commands: Arc<HashSet<String>>,
    /// Last seen comment text, for proc / class doc-comment
    /// harvesting.
    pub last_comment: String,
    /// Genuine lexical definition parent during its member inventory only.
    pub(super) original_class_definition_body:
        Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
    /// Source-file path (for the LSP `Diagnostic.uri` field), or
    /// `None` when analysing in-memory text.
    pub file_path: Option<String>,
    /// Per-scope const-string tracker:
    /// ``scope_kind_path → { var_name → (value, span) }``.
    /// Keyed on the path vector so snapshot/restore doesn't have to
    /// remap pointers.
    pub const_strings: HashMap<Vec<usize>, HashMap<String, (String, Span)>>,
    pub(super) finite_foreach_names: super::handlers::FiniteForeachReportingState,
    pub(super) logical_const_string_provenance:
        HashMap<Vec<usize>, HashMap<String, super::scope::LogicalConstStringProvenance>>,
    pub(super) logical_interp_path_provenance:
        HashMap<Vec<usize>, HashMap<String, super::scope::LogicalInterpreterPathProvenance>>,
    /// Conditional writes whose values do not dominate uses after a branch joins.
    /// A later unconditional assignment restores value eligibility.
    pub nondominating_consts: HashMap<Vec<usize>, HashSet<String>>,
    /// Variables known to contain regex patterns:
    /// ``(scope_path, var_name)``.
    pub regex_vars: HashSet<(Vec<usize>, String)>,
    /// iRules: enclosing ``when EVENT`` name.
    pub current_event: Option<String>,
    /// Tk checks: whether the per-command widget/geometry accumulation is
    /// worth running for this document.  A pure performance precheck
    /// ([`super::tk_checks::tk_checks_could_apply`]) — a sound
    /// over-approximation, never the activation decision, which is the exact
    /// `tk` dialect / `package require Tk` fact resolved at flush time.  Set
    /// per walk by the `analyse*` entry points so non-Tk files pay nothing.
    pub(super) tk_accumulation_enabled: bool,
    /// Whether this document's environment ships the `Tk` package
    /// **ambient** — i.e. Tk is already loaded before the first byte runs,
    /// so no `package require Tk` exists to find (a `wish` shell).
    ///
    /// Resolved at ingest from the environment's placement
    /// (`ResolvedContext::ambient_package("Tk")`), not from the ingest
    /// dialect *string* being literally `"tk"`. `Tk` is a package with a
    /// placement, so any environment — compiled or pack-declared — that
    /// says "Tk is ambient here" activates the Tk geometry checks without
    /// a require, and nothing has to be named `tk` to do it.
    pub(super) tk_ambient: bool,
    /// Tk checks (TK1002 / TK1003): diagnostics buffered during the walk and
    /// emitted post-walk by [`Self::flush_tk_geometry_diagnostics`], once the
    /// `tk` dialect / `package require Tk` activation condition is resolved.
    pub(super) tk_pending_diags: Vec<super::types::Diagnostic>,
    /// Tk checks (TK1001 / TK1002): the created-widget set and per-parent
    /// geometry-manager usage accumulated across the walk, **keyed by the
    /// interpreter domain** the commands execute in (`""` = the main
    /// interpreter, otherwise the `@interp@…` identity on
    /// [`Self::interp_path_stack`]).  Every interpreter that loads Tk gets
    /// its own `TkMainInfo` with its own widget-path `nameTable` and its own
    /// `.` root (Tk 9.0.4 `generic/tkWindow.c`, `TkCreateMainWindow`), so the
    /// same widget path in two domains is two unrelated windows — merging
    /// them produced a false TK1001 and a missed TK1002.
    /// Cleared by [`Self::flush_tk_geometry_diagnostics`].
    pub(super) tk_domains: std::collections::BTreeMap<String, super::tk_checks::TkDomainState>,
    /// Version-aware diagnostics (W135 / W136): command/option uses gated behind
    /// a package `min_version`, buffered during the walk and decided post-walk by
    /// [`Self::flush_version_gate_diagnostics`] once every `package require` is
    /// known.  See [`super::diagnostics::version_gate`].
    pub(super) version_gate_sites: Vec<super::diagnostics::version_gate::VersionGateSite>,
    /// Argument-DSL uses gated behind a Tcl release (W137 / W138, design
    /// doc §6), buffered during the walk and decided post-walk by
    /// [`Self::flush_dsl_gate_diagnostics`] against the effective Tcl
    /// version.
    pub(super) dsl_gate_sites: Vec<super::diagnostics::version_gate::DslGateSite>,
    /// The calls the walk dispatched with a word a literal-only check could
    /// not read, which the CFG/SSA pass checks again over the words' proven
    /// values.
    pub(super) proven_sites: Vec<super::diagnostics::ProvenSite>,
    /// The out-of-range `$var` index accesses the interval checks reported:
    /// the code, the absolute span of the statement or terminator the
    /// finding anchors at, and the index variable. The proven-word re-run
    /// leaves each of these sites to them.
    pub(super) interval_index_sites: HashSet<(DiagCode, Span, String)>,
    /// The conditional loops the walk examined, each with what its text says
    /// about its termination (W240 / W241 / W242). The CFG/SSA pass resolves
    /// each against its unit's branch fact at the condition's span
    /// ([`Self::resolve_loop_terminations`]); what no unit decides is
    /// reported by [`Self::flush_loop_terminations`] as its text says.
    pub(super) loop_candidates: Vec<super::bounds_checks::LoopTerminationCandidate>,
    /// What the module's callback scripts and variable traces may write, for
    /// the loops the CFG/SSA pass settles
    /// ([`Self::resolve_loop_terminations`]): set from the compilation unit
    /// the pass reads.
    pub(super) loop_unseen_writes: super::bounds_checks::ModuleUnseenWrites,
    /// Proven W147 option conflicts whose `OptionRelation` is version-gated
    /// — decided post-walk by [`Self::flush_gated_option_conflicts`], which
    /// promotes the ones the resolved floor actually has onto
    /// [`Self::pending_arity`]. A constraint with no lifecycle bypasses this
    /// buffer entirely and is queued inline at the dispatch site.
    pub(super) pending_option_conflicts: Vec<super::diagnostics::version_gate::GatedOptionConflict>,
    /// Calls to commands whose signature changed across their owning
    /// package's releases — decided post-walk by
    /// [`Self::flush_gated_arity_calls`], which selects the window the
    /// resolved floor covers and only then forms a verdict. A command with no
    /// `arity_windows` (almost every one) never reaches this buffer and keeps
    /// the inline path exactly as it was.
    pub(super) pending_gated_arity: Vec<super::diagnostics::version_gate::GatedArityCall>,
    /// Bare calls to *ensembles* whose parent arity is versioned — whether a
    /// subcommand is required at all can flip across releases, so the E001
    /// verdict is deferred to [`Self::flush_gated_bare_ensembles`] for the
    /// same reason the count verdict is.
    pub(super) pending_gated_bare_ensemble:
        Vec<super::diagnostics::version_gate::GatedBareEnsemble>,
    /// Session/file pins for the keyed library-version axes
    /// (`--bigip-version`-style overrides, dialect-profile-model.md §7.1).
    /// Defaults to empty, in which case each keyed axis falls back to its
    /// oldest-supported default; feeds
    /// [`tcl_dialect::DialectProfile::library_floor`].
    pub library_versions: tcl_dialect::LibraryVersionOverrides,
    /// §5.4 range targeting — configuration-declared version targets
    /// (`tclLsp.targets`) as `(provider, range clauses)` pairs, e.g.
    /// `("tcl", "8.5-9.0")` / `("Tk", "8.5-8.6")`. Merged at each walk
    /// ingress with the source's `# tcl-lsp: supports NAME RANGE`
    /// directives (the directive wins per provider) by
    /// [`Self::resolve_declared_targets`]. Empty — the default — leaves
    /// range mode off and every answer byte-identical to today.
    pub declared_targets: Vec<(String, String)>,
    /// Configured "requiring this package also loads these" edges
    /// (`tclLsp.packages.provides` / `.tcl-lsp.ini` `[packages.provides]`),
    /// in configuration order. Each pair is a package name and the packages
    /// its loader brings in behind the analyser's back — the binary
    /// extension whose `Init` calls `Tcl_PkgRequire` / `Tk_InitStubs`, where
    /// no Tcl source anywhere states the dependency, so it can only be
    /// declared. Applied to the walk's own
    /// `package require`s by [`Self::expand_implied_package_requires`].
    /// Empty — the default — leaves every answer as before.
    pub package_provides: Vec<(String, Vec<String>)>,
    /// The encoding the target reads this document's script in; see
    /// [`Self::with_script_encoding`].
    pub script_encoding: tcl_registry::value_transfer::ScriptEncoding,
    /// The `# tcl-lsp: package NAME provides …` edges declared in this walk's
    /// source —
    /// the same `(package, packages it also loads)` edges
    /// [`Self::package_provides`] carries for the whole workspace, declared
    /// for one file. Re-scanned at every walk ingress, so it is
    /// a pure function of the source and cannot go stale.
    pub(super) directive_provides: Vec<(String, Vec<String>)>,
    /// The resolved range-mode context: the walk generation's
    /// [`tcl_registry::model::ResolvedContext`] with the merged target
    /// declarations recorded on it (§5.4). `None` whenever nothing is
    /// declared — the no-range fast path every existing document takes.
    pub(super) range_context: Option<tcl_registry::model::ResolvedContext>,
    /// The numeral grammars represented across the declared core-Tcl
    /// targets (fewer than two ⇒ no numeral divergence is possible and
    /// the W151 walk is off). Derived beside [`Self::range_context`].
    pub(super) range_numeral_grammars: Vec<tcl_dialect::NumberSyntax>,
    /// §5.4 mask-gated range sites (W150): availability gates whose
    /// spelling is `SpecSurface` ladder bits rather than a lifecycle,
    /// buffered during the walk and decided post-walk beside the
    /// lifecycle sites so one word still draws one diagnostic.
    pub(super) range_gate_sites: Vec<super::diagnostics::version_gate::RangeGateSite>,
    /// Cached set of built-in command names for redefined-builtin
    /// detection. `None` until first lookup; filled lazily.
    pub builtin_names: Option<HashSet<String>>,
    /// The dialect ``builtin_names`` was built for, for cache
    /// invalidation.
    pub builtin_dialect: Option<&'static str>,
    /// Conditional-nesting depth — incremented on entry to
    /// `if` / `catch` / `try` arms, used to mark
    /// ``package require`` records as ``conditional=true``.
    pub conditional_depth: u32,
    /// Nesting depth inside the body of a [`tcl_registry::Traits::CONTROL_FLOW`]
    /// command — `if`, `while`, `for`, `foreach`, `lmap`, `switch`, `try`,
    /// `catch` and their dialect siblings.
    ///
    /// A body at depth 0 is *straight-line*: it runs exactly once when its
    /// enclosing script runs. Depth above 0 means the walker cannot say
    /// whether the command runs at all, which is what
    /// [`Self::handle_rename`] needs before recording a command deletion as
    /// unconditional. Deliberately distinct from `conditional_depth`, which
    /// covers only `if`/`try` and drives the `package require` conditional
    /// flag.
    ///
    /// Registry-driven: the trait decides, so no command name appears in the
    /// walker. `namespace eval`, `eval`, and `uplevel` bodies do *not* count
    /// — they run unconditionally, so a deletion inside one is still
    /// straight-line.
    pub control_flow_body_depth: u32,
    /// Depth of enclosing branch-selected bodies whose selector reads a debug
    /// flag — a `static::` variable, or a name assigned by an event that runs
    /// less often than once per request.
    /// IRULE5001 stays silent while this is non-zero, because the `log` it
    /// would report is already gated the way its own message prescribes.
    /// Nested bodies inherit the gate: the depth only returns to zero once
    /// the walk leaves every gating body.
    pub irules_debug_gate_depth: u32,
    /// Body-nesting depth — incremented on entry to a braced
    /// body. Used for top-level-only command checks.
    pub body_depth: u32,
    /// One-shot: the next `process_command` call dispatches a command whose
    /// argument words are **pre-substituted** `list` elements
    /// (`analyse_list_quoted_body`).  Its nested `[…]`-substitution walks
    /// must not run again — they already ran, in the *building* frame, when
    /// the enclosing command's own walk descended the `[list …]` argument;
    /// re-running them here would record those substitutions a second time
    /// under the built script's target scope (`namespace eval :: [list puts
    /// [helper]]` must not manufacture a `::helper` invocation).  Consumed
    /// (reset) at the top of `process_command`, so recursion *below* the
    /// built command — a literal body among its words — behaves normally.
    pub presubstituted_args: bool,
    /// Exact evaluated-body entry for a reanchored single-command list script.
    /// The display span remains authored; semantic origin and phase belong to
    /// the retained body invocation, rather than the list construction site.
    pub evaluated_body_invocation: Option<(u32, crate::command_binding::SourceInvocationBinding)>,
    /// Whether **E207** (nesting depth exceeds the analysis limit — see
    /// `commands::MAX_BODY_DEPTH`) has already been emitted for this walk.
    /// The depth cap trips once per nested body past the limit — on
    /// pathologically deep input that could be hundreds of bodies — so this
    /// flags it emitted at most once rather than flooding
    /// `result.diagnostics` with duplicates that all say the same thing.
    pub(super) e207_emitted: bool,
    /// Stack of active scoped command environments — pushed while walking a
    /// command body whose spec carries a
    /// [`body_scope`](tcl_registry::CommandSpec::body_scope) (e.g. inside a
    /// `report::defstyle` style script).  The innermost environment (stack top)
    /// resolves bare command heads to their scoped signatures for the
    /// arity / subcommand checks; the post-walk W123 pass instead uses the
    /// recorded [`AnalysisResult::scoped_command_regions`] spans.  Push/pop is
    /// balanced within [`Self::dispatch_body_arguments`], so it self-clears.
    pub body_scope_stack: Vec<&'static tcl_registry::scoped::ScopedCommandEnv>,
    /// Command-alias records:
    /// ``alias_name → (target_cmd, prepended_args)``.
    pub command_aliases: HashMap<String, (String, Vec<String>)>,
    /// Static `rename OLD NEW` records: ``new_qname → old_qname``.
    /// `NEW` inherits whatever `OLD` denoted (a proc's own signature,
    /// unchanged) — mirrors [`Self::command_aliases`]'s shape, but with
    /// no prepended-argument shift, since a rename is a pure name move.
    /// Only populated for a *static* rename (see
    /// [`tcl_syntax::naming::is_dynamic_word`]); a dynamic
    /// `rename $x y` / `rename x [y]` instead sets
    /// [`super::types::AnalysisResult::has_dynamic_providers`] and is not
    /// recorded here.
    pub renamed_commands: HashMap<String, String>,
    /// Byte offset of the `interp alias` command token that established
    /// each [`Self::command_aliases`] entry (keyed the same way, by alias
    /// name) — kept as a parallel map rather than widening
    /// `command_aliases` itself, since that type (`alias::CommandAliasMap`)
    /// is shared with the lowering/IR pipeline. Used to order-gate a
    /// *top-level* call against the alias: a call lexically before the
    /// `interp alias` statement runs first at run time, so the alias
    /// doesn't exist yet there (confirmed against tclsh 9.0.4). Proc-body
    /// calls are not order-gated (the whole file loads, establishing
    /// every alias, before any proc body runs).
    pub alias_offsets: HashMap<String, u32>,
    /// Byte offset of the `rename` command token that established each
    /// [`Self::renamed_commands`] entry (keyed by `new_qname`) — the
    /// same order-gating role as [`Self::alias_offsets`], kept parallel
    /// rather than widening `renamed_commands` for the same reason.
    pub rename_offsets: HashMap<String, u32>,
    /// Reverse index: ``old_qname → rename_offset`` for every static
    /// `rename OLD NEW`. A rename removes `OLD` as a command entirely
    /// (confirmed against tclsh 9.0.4: calling `OLD` afterwards fails
    /// "invalid command name", not a "wrong # args" on its original
    /// signature) — this lets the same-file arity resolver recognise a
    /// call to `OLD` as no longer reaching the original proc once the
    /// rename is in effect (order-gated the same way as
    /// [`Self::rename_offsets`]), instead of still validating it against
    /// a definition that's no longer callable under that name.
    pub deleted_commands: HashMap<String, u32>,
    /// Earliest source offset at which this file's own top-level execution
    /// provably *reaches* each qualified name — keyed by
    /// `resolved_qualified_name`. A top-level (not inside any proc/class
    /// body) call contributes its own offset; a call inside definition
    /// `E`'s body contributes `E`'s own reachable offset, transitively
    /// through the whole call graph.
    ///
    /// Populated once by [`Self::finalise_invocation_resolutions`] from the
    /// already-settled `command_invocations`, and consulted by
    /// [`crate::command_binding::SourceInvocationBinding::selected_slot_presence`] when the call site under test is itself
    /// nested inside a body: a proven invocation of the *enclosing*
    /// definition that ran before a later unconditional deletion means that
    /// invocation's own nested calls already resolved (`proc helper {}`,
    /// `proc caller {} { helper }`, `caller`, `rename helper {}` resolves in
    /// real Tcl — confirmed against tclsh 8.6.14 — because `caller`'s own
    /// top-level invocation runs before the rename; the same holds through an
    /// arbitrary chain of enclosing definitions).
    ///
    /// A name with no entry is one this file never reaches — including
    /// every member of a mutual-recursion cycle no top-level call enters.
    ///
    /// # Known false negative: only the *earliest* reach is recorded
    ///
    /// The map holds one offset per name — the earliest — so a definition
    /// called both before and after a deletion looks reached-before, and the
    /// deletion diagnostic on its body is withdrawn for *all* of its
    /// invocations rather than just the early ones.
    ///
    /// Oracle (tclsh8.6): `proc a {} {
    /// helper }`, `a`, `rename helper {}`, `a` — the second `a` really does
    /// fail with `invalid command name "helper"`, and no W123 is reported.
    ///
    /// This is deliberate: reporting it needs a per-invocation
    /// reachability interval rather than a single floor, and the escape
    /// hatch exists precisely to stop the far commoner call-before-deletion
    /// shape being flagged. The false negative reaches transitively — a
    /// chain of calls carries it through arbitrarily many bodies.
    pub(super) reachable_call_offsets: HashMap<String, u32>,
    /// Static `namespace path {…}` declarations: ``declaring namespace →
    /// raw path entries`` (each declaration replaces the whole path, as in
    /// C Tcl, so the lexically-last one wins). Entries are stored as
    /// written — absolute or relative — and normalised post-walk by
    /// [`Self::finalise_invocation_resolutions`], which resolves a
    /// relative entry against the namespaces declared in the file
    /// (current-first, then global — Tcl's namespace-name resolution at
    /// `namespace path` set time). Only a literal list argument is
    /// recorded; a dynamic one (`$var` / `[cmd]`) is skipped, keeping the
    /// conservative empty path.
    pub(super) namespace_paths: HashMap<String, Vec<String>>,
    /// Variable-as-command call sites; resolved post-walk by W307.
    pub var_command_sites: Vec<VarCommandSite>,
    /// `$cmd`-head dispatch sites: every simple-`$var` command head,
    /// pending settlement against the compiler's flow-sensitive value
    /// model (`value_provenance::const_contributors`) in the CFG/SSA
    /// diagnostic phase, where the `CompilationUnit` exists.  A site whose
    /// contributors form a finite set of written constants settles into
    /// [`SignatureCommandInvocation`]s — an *indirect* one at the `$cmd`
    /// head per resolved user-command target (navigation), plus a
    /// *writable* literal-anchored one at each contributing definition
    /// (rename rewrites the defining constant, keeping the dispatch
    /// alive).  Anything unprovable is dropped (sound abstention — no
    /// phantom invocation, no W123 delta).
    pub(super) pending_const_dispatches: Vec<ConstDispatchSite>,
    /// `TclOO` instance-creation sites whose class head is a `$var`
    /// reference, pending settlement alongside
    /// [`Self::pending_const_dispatches`] once the CFG/SSA
    /// `CompilationUnit` exists.
    pub(super) pending_instance_class_sites: Vec<PendingInstanceClassSite>,
    /// Dispatch-table value literals whose consuming sites reach different
    /// implementations. Each such literal keeps navigation but no rename
    /// authority, whichever producer anchors a reference there.
    pub(super) divergent_table_literals: HashSet<(u32, u32)>,
    /// The namespace key a seeded analysis wraps the whole file in (set
    /// by [`Analyser::analyse_with_source_namespace`]); the scope chain it
    /// creates becomes the top-level walk's base path.
    pub(super) seed_namespace_key: Option<String>,
    path_constant_source_namespace: Option<String>,
    /// Scope path of the innermost seeded namespace scope (empty when not
    /// seeded) — the base path for the top-level walk.
    pub(super) seed_scope_path: Vec<usize>,
    /// Tk widget instance-dispatch candidates (`.t instate …`, `$w tag
    /// configure …`) whose head the ordinary registry-command resolution
    /// could not resolve; resolved post-walk by
    /// [`super::diagnostics::widget_command::Analyser::flush_widget_dispatch_diagnostics`]
    /// once `instance_classes` is complete.
    pub widget_dispatch_sites: Vec<super::diagnostics::widget_command::WidgetDispatchSite>,
    /// Command-substitution-as-command call sites; same dispatch
    /// as [`Self::var_command_sites`] but for ``[cmd] args``
    /// shapes.
    pub cmd_command_sites: Vec<CmdCommandSite>,
    /// Namespaces where ``namespace ensemble create`` was seen —
    /// their tail names become valid commands.
    pub ensemble_namespaces: HashSet<String>,

    /// Source offset of the **latest** `namespace ensemble` list token that
    /// (re)filed each [`super::types::AnalysisResult::ensemble_subcommand_targets`]
    /// key.  Analyser-side book-keeping for the per-item path only: the
    /// snapshot attached to a deferred body includes an ensemble exactly
    /// when its recording precedes the body in source order — the same
    /// visibility the whole-file DFS gives a body walked at its definition
    /// point.  Never merged into the result.
    pub(super) ensemble_record_offsets: HashMap<String, u32>,
    /// `<ensemble> <subcommand> …` call sites the **shell pass** met before
    /// the ensemble that maps them was known, replayed against the finished
    /// map by [`Self::flush_pending_ensemble_subcommand_invocations`].
    /// Only the shell pass fills this: the whole-file
    /// DFS and the isolated body pass both resolve at walk time against a
    /// map that already holds everything their own walk order could see.
    pub(super) pending_ensemble_subcommands: Vec<PendingEnsembleSubcommand>,
    /// `namespace ensemble create|configure ... -map {sub target ...}`
    /// subcommand-to-target maps, keyed by the ensemble's own qualified
    /// command name (`-command NAME`, or the enclosing namespace's own
    /// qualified name when `-command` is absent — Tcl's default). Consulted
    /// by the W129 safe-interpreter gate so a
    /// hidden command reached only through an ensemble redirect (`myens sub
    /// ...` → target) is still flagged, mirroring a literal call to the
    /// target.
    pub ensemble_command_maps: HashMap<String, HashMap<String, String>>,
    /// Vars where ``oo::objdefine`` was applied — the per-instance
    /// method table may extend the class definition.
    pub objdefined_vars: HashSet<String>,
    /// Per-object member-state binding index: `(receiver key,
    /// innermost proc/method frame extent)` → position in
    /// `result.object_member_state[key]`.  The frame extent is the walk-time
    /// spelling of the binding identity consumers re-derive from
    /// [`super::types::ObjectMemberState::anchor_offset`].
    pub(super) objdefine_bindings: HashMap<(String, Option<(u32, u32)>), usize>,
    /// W315 candidates from `oo::objdefine` walks, held until the whole
    /// document is walked so the emission gates can consult document-wide
    /// facts (per-object declarations under *any* key, receiver creations).
    pub(super) objdefine_abort_candidates: Vec<ObjdefineAbortCandidate>,
    /// `true` when any `oo::objdefine` receiver in the document did not
    /// resolve statically — an unknown object may be any object, so the
    /// per-object W315 abstains file-wide.
    pub(super) objdefine_unresolved_receiver: bool,
    /// The **interpreter-domain map**: every
    /// child interpreter this document creates with a literal path,
    /// keyed by the whitespace-normalised path list, carrying its safe
    /// state and per-interpreter hide/expose deltas.  Flow-insensitive
    /// union across the file (like the rest of the environment model).
    pub(super) interpreters: HashMap<String, InterpState>,
    /// `true` when an `interp` create / delete / hide / expose operation
    /// used a **dynamic** path or command operand — interpreter
    /// existence is then unknowable and the W140 unknown-interpreter
    /// diagnostic abstains file-wide.
    pub(super) dynamic_interp_ops: bool,
    /// Per-interpreter **deletion epochs** (temporal identity): `interp
    /// delete` bumps the path's epoch, so a later
    /// re-creation of the same path is a *fresh* domain — its evaluation
    /// bodies home under a new `@interp@<path>#<epoch>` identity and never
    /// merge with the deleted interpreter's definitions (as in C, where
    /// the recreated child starts with an empty command table).
    pub(super) interp_epochs: HashMap<String, u32>,
    /// The `interp eval` bodies currently on the walk stack: an `interp`
    /// operation *inside* a child body names paths relative to that child
    /// (`interp create t` inside `interp eval s {…}` creates `s t`), so
    /// handlers qualify their literal path operands against the top frame's
    /// [`key`](InterpFrame::key).  The frame also carries the body's
    /// [`domain`](InterpFrame::domain) identity, which is what analyser
    /// state modelling *per-interpreter runtime state* keys itself by.
    pub(super) interp_path_stack: Vec<InterpFrame>,
    /// The document did not parse cleanly and its command stream is the
    /// recovery segmenter's. No interpreter runs such a document, so the
    /// interpretation holds nothing for the recovered tail.
    pub(super) recovered_source: bool,
    /// Walking a script an eval-family command builds by concatenating its
    /// words. The lowering retains no command at those source offsets, so
    /// a declaration there keeps its written layout as in a recovered tail.
    pub(super) concatenated_script: bool,
    /// Every fully static concatenated script walked, by its in-place source
    /// window, for the registry's count errors on its commands.
    pub(super) concatenated_windows: Vec<(String, tcl_lexer::Span)>,
    /// The first syntax cut of such a document and the registry's verdicts
    /// on the commands recovered after it, published once every other
    /// emitter has run.
    pub(super) recovered_tail: Option<(u32, Vec<super::types::Diagnostic>)>,
    /// Safe-interpreter evaluation contexts currently on the walk stack:
    /// non-empty while walking an `interp eval` body whose target
    /// interpreter is safe.  The per-command gate consults the top —
    /// a command whose registry spec carries
    /// [`tcl_registry::Traits::SAFE_INTERP_HIDDEN`] (or was
    /// `interp hide`-den), and is not re-exposed, draws W129 and has no
    /// analysed effect (C raises `invalid command name` before any
    /// side-effect happens — so no source / package / definition edges
    /// may be built from it either).
    pub(super) safe_interp_stack: Vec<SafeInterpCtx>,
    /// The scope-chain-aware **interpreter value-flow map**:
    /// `set VAR [interp create ...]` binds `VAR`, in the scope it
    /// was written, to the `interpreters` domain key recorded for that
    /// call — a literal path's qualified key, or a synthetic
    /// per-call-site key when the call captured no literal path. Mirrors
    /// `const_strings`'s scope-chain shape (never `instance_classes`'s
    /// flat, file-wide one — a raw-name collision there only softens a
    /// diagnostic; here it would corrupt real go-to-definition targets),
    /// so two unrelated procs binding the same variable name to
    /// different interpreters never collide. Consulted by
    /// [`Self::resolve_dynamic_interp_path`] from the call sites that would
    /// otherwise require the interpreter path to be a source literal.
    pub(super) interp_var_bindings: HashMap<Vec<usize>, HashMap<String, String>>,
    /// Guard against double W123 emission across
    /// ``analyse_commands`` / ``analyse_irule_event``.
    pub unresolved_commands_emitted: bool,
    /// Command registry for the active dialect.  Populated at the
    /// top of [`Self::analyse`] so per-command handlers
    /// (especially the registry-driven body iteration in
    /// `process_command` for `if` / `while` / `when` / OO method
    /// bodies) don't have to rebuild it on every command.  `None`
    /// outside an active analysis run; handlers that need the
    /// registry must check `self.registry.is_some()`.
    ///
    /// Held as an owning **handle**, not a `&'static`: a pack overlay
    /// generation is retired once the cache has superseded it and the last
    /// holder drops it, so keeping this handle for the whole analysis is
    /// exactly what stops the walk reading a registry that a concurrent pack
    /// reload has replaced underneath it. Cloning it is an `Arc` bump.
    pub registry: Option<std::sync::Arc<tcl_registry::CommandRegistry>>,
    /// Lazily-built whole-module command-mutation trust oracle for the
    /// constant command-substitution fold. The analyser's own
    /// `renamed_commands` is flow-sensitive — populated only up to the
    /// current point of the walk — which is NOT a sound trust source for
    /// folding (a `rename` buried in a proc body later in the file can fire
    /// before an earlier `set`'s consumer runs). This oracle is the same
    /// whole-module, flow-insensitive
    /// [`crate::command_binding::scan_module_command_mutations`] scan the
    /// optimiser's O129 fold gates on, built on first demand (the fold is
    /// attempted only for a `set VAR [cmd …]` whose head could actually
    /// fold — see [`crate::const_subst::head_may_fold`]) and cached for the
    /// rest of the run. `None` = not built yet this run.
    pub(super) command_trust:
        Option<std::sync::Arc<crate::command_binding::ModuleCommandMutations>>,
    /// The "known command" universe for unclosed-delimiter recovery: the
    /// active registry's names plus every proc / class / command-alias the
    /// document itself defines. Populated alongside `registry` at the top
    /// of every entry point via
    /// [`super::utils::recovery_known_commands`] so the segmenter's
    /// scan-to-next recovery and the E100/E201/E202/E203 diagnostics agree
    /// on what counts as a "real command" starting a new line. Empty
    /// outside an active analysis run.
    pub recovery_known_commands: super::utils::RecoveryKnownCommands,
    /// When `true`, the proc handler runs the deep-recursive
    /// pass of [`super::param_traits::infer_param_traits_deep`]
    /// after the shallow pass and unions the results via
    /// [`super::param_traits::merge_traits`].  Off by default —
    /// the shallow pass is fast enough for synchronous analysis
    /// and catches the common patterns; the deep pass is
    /// intended for asynchronous use behind the `S*` call-graph
    /// / symbol-graph / dataflow-graph / semantic-graph builders.
    pub deep_param_traits: bool,
    /// The commands this **document** declares for itself, ingested at
    /// the top of [`Self::analyse`] from `result.stub_commands` via
    /// [`super::types::build_declared_surface`] — inline
    /// `# tcl-lsp: stub` blocks and workspace `.tcl.stubs` sidecars as
    /// provenance-tagged surface declarations.  Paired
    /// with the walk's registry generation by
    /// [`Self::command_surface`], the one door analyser and compiler
    /// queries ask; nothing mutates the shared
    /// [`tcl_registry::CommandRegistry`].  `None` outside an active
    /// analysis run.  Tied to the (single-threaded) analyser instance
    /// rather than a thread-local.
    pub declared_commands: Option<tcl_registry::model::DeclaredSurface>,
    /// The document's statically proven command-identity facts
    /// ([`crate::realm`]) — which registry command each head spelling
    /// really names, folding in `namespace import`, `interp alias`, `rename`,
    /// and a built-in-shadowing `proc`.  Rebuilt alongside
    /// [`Self::registry`] at the top of every entry point so a per-proc
    /// param-trait scan resolves a body's heads against the *document's*
    /// bindings rather than their written spellings.  Empty
    /// outside an active analysis run, and for the overwhelmingly common
    /// document that binds nothing.
    pub head_identities: crate::realm::CommandBindingRealm,
    /// Sorted byte offsets of every ``\n`` in [`Self::source`],
    /// precomputed at the top of [`Self::analyse`] /
    /// [`Self::analyse_chunked`] / [`Self::analyse_commands`] so
    /// per-command line-number lookups (notably
    /// [`super::utils::apply_preceding_noqa`] which runs once
    /// per command) cost ``O(log N)`` instead of ``O(N)`` per
    /// call.  ``None`` outside an active analysis run.
    pub line_offsets: Option<Vec<usize>>,
    /// Cached [`tcl_lexer::LineIndex`] over [`Self::source`], precomputed
    /// once at the top of [`Self::analyse`] / [`Self::analyse_chunked`] /
    /// [`Self::analyse_commands`] (alongside [`Self::line_offsets`]).
    ///
    /// Every recursive-body handler needs a [`tcl_lexer::SourceMap`] to
    /// resolve token text / positions; before this cache existed, each of
    /// the ~14 call sites called `SourceMap::new(&self.source)`, which
    /// rescans the **entire document** to rebuild the line index from
    /// scratch. Called once per command at every nesting level, that
    /// turned an `O(document size)` per-command cost into `O(document
    /// size × nesting depth)` overall — a genuine, severe (though
    /// non-crashing) `DoS`: deeply-nested or merely large documents could
    /// take many seconds even where the recursion depth itself stayed
    /// safely under the analyser's caps. Use
    /// [`Self::source_map`] instead of `SourceMap::new(&self.source)` —
    /// cloning a [`tcl_lexer::LineIndex`] is one allocation plus a copy of
    /// its line-start offsets (`O(line count)`), not a document rescan.
    ///
    /// [`Self::source`] is a `pub` field that plenty of unit tests (and, in
    /// principle, any other consumer) assign directly rather than through
    /// [`Self::analyse`], so this cache can legitimately go stale relative
    /// to it. [`Self::cached_line_index_source_len`] guards that: a length
    /// mismatch means the cache doesn't describe the current `source`, and
    /// [`Self::source_map`] falls back to a fresh scan rather than trusting
    /// it. This is a cheap, not perfect, staleness check (same length,
    /// different content, set outside `analyse` would slip through) — good
    /// enough because every real entry point keeps the two in lock-step;
    /// only direct test-only field pokes can desync them, and those change
    /// the length in every case in this codebase's test suite.
    pub(super) cached_line_index: tcl_lexer::LineIndex,
    /// `self.source.len()` at the point [`Self::cached_line_index`] was
    /// built — see that field's doc comment.
    pub(super) cached_line_index_source_len: usize,
    /// Statement the static provenance evaluator is reading, so a nested
    /// substitution can be found at its own source position.
    pub(super) static_eval_site: Option<Span>,
    /// Candidate E002 / E003 arity diagnostics, W004
    /// (dialect-invalid-option) diagnostics, **and** W001 (unknown
    /// subcommand) diagnostics collected during the command walk, as
    /// `(command name, call-site namespace, enforce_order, diagnostic)`.
    /// All three are "this registry builtin's rule was violated" candidates
    /// with the identical suppression condition, so they share one queue.
    /// Emitted in a post-walk pass
    /// ([`Self::flush_arity_diagnostics`]) so a command that resolves
    /// to a user-defined proc / class / alias / ensemble / stub —
    /// which may be defined *after* its call site — suppresses the
    /// builtin check (the call dispatches to that definition at run
    /// time, not to the registry builtin the candidate describes).  The
    /// namespace is captured so suppression is scoped to the command the
    /// call actually resolves to (current namespace → global), not to
    /// every same-tail-named definition anywhere in the file.
    /// `enforce_order` is `true` for top-level calls (module body,
    /// `namespace eval` bodies, conditionals) which execute in source
    /// order during load: a shadowing proc only silences such a call
    /// when its definition lexically precedes it.  Proc-body calls
    /// (`enforce_order == false`) resolve after load, so any same-named
    /// definition shadows regardless of order.  The arity check runs
    /// over the fully-resolved IR rather than inline during the walk.
    pub pending_arity: Vec<(String, String, bool, super::types::Diagnostic)>,
    /// Same-file user-call arity candidates — see
    /// [`super::types::PendingUserCallArity`]. Queued for *every* call
    /// (not just ones with a registry signature). Only display and position
    /// are queued; actual original call/header/argv are reborrowed in the
    /// same post-walk pass as [`Self::pending_arity`]
    /// ([`Self::flush_arity_diagnostics`]).
    pub pending_user_call_arity: Vec<super::types::PendingUserCallArity>,
    /// Registry-declared class-constructor arity candidates — see
    /// [`super::types::PendingCtorArity`]. Queued whenever a call's first
    /// word has a possible manufacturer descriptor and
    /// resolved in the same post-walk pass as [`Self::pending_arity`]
    /// ([`Self::flush_ctor_arity_diagnostics`]).
    pub pending_ctor_arity: Vec<super::types::PendingCtorArity>,
    /// `TclOO` `next` / `nextto` call-site arity candidates — see
    /// [`super::types::PendingNextArity`]. Queued whenever such a call
    /// sits inside a method body and resolved in the same post-walk pass
    /// as [`Self::pending_arity`] ([`Self::flush_next_arity_diagnostics`]).
    pub pending_next_arity: Vec<super::types::PendingNextArity>,
    /// W108 non-ASCII detection mode (`tclLsp.style.nonAscii`).
    /// [`NonAsciiMode::Default`] resolves per dialect at emit time
    /// (strict for F5 iRules/iApps, confusables otherwise).
    pub non_ascii_mode: NonAsciiMode,
    /// When `true`, the walk builds only the **structural** facts (procs,
    /// classes, aliases, ensembles, namespace/scope tree) and skips every
    /// diagnostic-emission and cross-feature recording pass — the per-command
    /// diagnostic dispatch *and* the post-walk emitter tail.  Used by the
    /// `item_tree` query to extract the offset-stable declaration set cheaply
    /// (the diagnostics are the bulk of the cost).  The structural handlers are
    /// unchanged, so the resulting decl set is byte-identical to a full
    /// `analyse` (gated by the `file_decls_corpus` corpus test).  Defaults to
    /// `false`; normal `analyse` is unaffected.
    pub structure_only: bool,
    /// When `true`, [`Analyser::analyse`] retains only the document's source
    /// identity and resolved input: the realm interprets no command and the
    /// walk visits none, so the cost does not grow with the file. The result
    /// carries no declaration, binding or diagnostic. It exists for display
    /// consumers that read written words and the registry (the first-paint
    /// semantic-token tier). Defaults to `false`.
    pub source_shell: bool,
    /// Test-only capture of the diagnostics the whole-document CFG/SSA phase
    /// adds, so fixtures that assert on that phase alone run through the
    /// checked `analyse` ingress instead of a detached unit build.
    #[cfg(test)]
    pub(crate) cfg_ssa_capture_for_test: Option<Vec<crate::analyser::types::Diagnostic>>,
    /// Fully-qualified names of classes defined **elsewhere in the workspace**,
    /// supplied by the cross-file reference/definition path so instance
    /// inference can resolve a constructor whose class lives in another file
    /// (`set d [::other::Cls new]`).  Empty for the normal single-file analysis,
    /// so diagnostics that rely on `instance_classes` naming a *locally-known*
    /// class (e.g. method-existence checks) are unaffected — only the opt-in
    /// cross-file re-analysis populates it.
    pub workspace_classes: std::collections::HashSet<String>,
    /// The qualified names of workspace classes whose **own command**
    /// constructs an instance from a bare unrecognised word and yields its
    /// name — Tk's `::tk::IconList .il`.
    ///
    /// A strict subset of [`Self::workspace_classes`], carried separately
    /// because the proof lives on the class's *metaclass*, which a pure
    /// consumer document never sees. Each entry was proved where the class
    /// was written; an empty set (the overwhelmingly common case) leaves
    /// every consumer behaving exactly as it did before.
    pub workspace_bare_word_classes: std::collections::HashSet<String>,
    /// Class **factories** — user-defined `TclOO` metaclasses — declared
    /// elsewhere in the workspace, keyed by fully-qualified name.
    ///
    /// The per-file walk cannot tell `::tk::Megawidget create IconList
    /// FocusableWidget {…}` from `interp create` without knowing that
    /// `::tk::Megawidget` is a metaclass, so with no index it abstains and
    /// records nothing.  Each entry is the factory description
    /// the metaclass's *own* document derived
    /// ([`super::types::ClassDef::factory`]), so the consuming walk classifies
    /// the call from a proved fact rather than its shape.  `None` — the
    /// default, and every single-file analysis — keeps the abstention exactly
    /// as it was.
    pub workspace_class_factories: Option<std::sync::Arc<super::types::ClassFactoryIndex>>,
    /// Instance methods dispatchable on some workspace **descendant** of
    /// each class, keyed by ancestor qualified name — the cross-file half of
    /// the template-method abstention.  A base class calling
    /// `my Render` where `Render` is written only by a subclass in another
    /// document is the same deliberate pattern as the single-file shape the
    /// local hierarchy proves; without this view the workspace still drew
    /// the W308 the sibling file refutes.  `None` — the default, and every
    /// single-file analysis — changes nothing: the warning keeps firing
    /// exactly as the per-file evidence dictates.
    pub workspace_subclass_methods: Option<std::sync::Arc<super::types::SubclassProvidedMethods>>,
    /// Procedures other workspace documents define that a call may trust,
    /// as declarations; the document's own declarations win over them.
    pub workspace_declared_commands: Option<std::sync::Arc<tcl_registry::model::DeclaredSurface>>,
    /// Sibling workspace files an implicit auto-load index offers, sourced in
    /// full when the document calls one of their commands while it is absent.
    pub workspace_source_modules:
        Option<std::sync::Arc<Vec<crate::command_binding::TrustedSourceModuleLoader>>>,
    /// When `true` (the per-item shell walk), `handle_proc_command` / OO method
    /// walks record their body for separate analysis (`deferred_bodies`) instead
    /// of recursing into it immediately.  Set only for the shell pass; the
    /// per-body passes run with it `false` so nested defs walk in place.
    pub defer_proc_bodies: bool,
    /// Set by the per-item shell walk when it meets a definer only the
    /// workspace's packs declare — a command whose overlaid spec carries a
    /// definition-body grammar the un-overlaid store the shell reads lacks —
    /// so [`Self::analyse_per_item_with`] takes the full path
    /// ([`super::per_item::PerItemFallback::PackDefiner`]).
    pub(super) pack_definer_seen: bool,
    /// When `true`, [`Self::define_var`] runs in **structural rebind** mode:
    /// it skips the W215 unreachable-name check *and* the
    /// `record_qualified_var_ref` occurrence record.  Set only while the
    /// per-item path re-binds a deferred body's *parameters* / instance
    /// variables into its isolated scope — a structural rebind so body
    /// references resolve. The shell walk (`handle_proc`) already emitted
    /// W215 and recorded the qualified occurrence for those declarations,
    /// byte-identically to the full `analyse` path, so the isolated rebind
    /// must stay record-free or the per-item result gains a spurious
    /// duplicate — a W215 (the synthetic rebind token would also flip the
    /// `braced` reachability heuristic), or a phantom zero-width
    /// `QualifiedVarRef` for a `::`-qualified parameter name (caught by the
    /// `per_item_matches_analyse_under_edits` fuzzer).
    /// See `analyse_proc_body_isolated`.
    pub(super) structural_rebind: bool,
    /// Bodies deferred by the shell walk (see [`Self::defer_proc_bodies`]),
    /// each analysed in a second pass that fills its already-created scope.
    pub(super) deferred_bodies: Vec<super::per_item::DeferredBody>,
    /// Every offset-keyed synthetic identity this run minted
    /// ([`Self::mint_synthetic_offset_name`]): `@dynns@<off>` /
    /// `@dynclass@<off>` / `@autoname@<off>`.  An isolated proc-body
    /// analysis mints these from **body-relative** offsets (keeping the
    /// memoised fragment offset-invariant), and this set is what lets the
    /// per-item graft rebase exactly those names — and no look-alike literal
    /// from the source — to the absolute offsets the whole-file walk mints
    /// (see [`tcl_syntax::naming::rebase_synthetic_offset_names`]).  Unused
    /// on the whole-file path beyond the insert itself.
    pub(super) minted_synthetic_names: std::collections::HashSet<String>,
    /// Pre-built compilation unit for the CFG/SSA diagnostic tail.  When
    /// `Some`, [`Self::emit_cfg_ssa_diagnostics`] consumes it instead of
    /// rebuilding the whole-file unit — the seam the incremental per-item
    /// path uses to supply a unit whose per-function lattices were memoised.
    /// `None` for the whole-file `analyse` path (byte-identical: it builds
    /// the unit itself, exactly as before).
    pub(super) cu_override: Option<CompilationUnitOverride>,
    /// The inputs the per-item shell built its realm under, for the fallback.
    pub(super) per_item_realm_inputs: Option<(String, RealmBuildInputs)>,
    /// A realm handed over by the per-item fallback; consumed by `analyse`.
    pub(super) prebuilt_realm: Option<Box<PrebuiltRealm>>,
    /// Driver-supplied execution entry shared by realm, lowering and diagnostics.
    pub(super) source_analysis_entry:
        Option<std::sync::Arc<crate::command_binding::SourceAnalysisEntry>>,
    source_region_override: Option<crate::command_binding::OriginalSourceConditionalRegion>,
    /// When `Some`, an isolated proc-body analysis (the per-item path) records
    /// every qualified (`::` / `static::`) variable read that fell through to
    /// the (empty) enclosing global scope here, instead of dropping it.  The
    /// aggregator replays these on the shell's global scope during the graft, so
    /// a body's `$::g` read lands as a reference on the real enclosing `::g`
    /// def — one of the divergences the per-item path must reproduce.  `None` on
    /// the whole-file `analyse` path (reads resolve against the populated scope).
    pub(super) capture_global_reads: Option<Vec<super::per_item::CapturedBodyGlobalRead>>,
    /// The write-side twin of [`Self::capture_global_reads`].  When `Some`,
    /// an isolated proc-body analysis (the per-item path) records every
    /// variable the body defines **directly in the global scope** — a body
    /// that names one fixed, frame-independent cell, which today means
    /// `upvar`'s `otherVar` word (`upvar ::tk::FocusGrab($i) data`, `upvar
    /// #0 counter c`; see
    /// [`Analyser::apply_state_transitions`](super::state::Analyser)).
    ///
    /// The graft merges the fragment's *proc* scope, never its (throwaway)
    /// root, so without this capture such a cell reached `all_variables` but
    /// never the shell's scope tree — and the scope tree is what
    /// `attach_qualified_var_references`, `replay_body_global_reads`, and
    /// every LSP navigation provider read.  Without the capture the
    /// whole-file walk answers hover / definition / references for the cell
    /// while the live server's incremental walk answers nothing.  `None` on the whole-file
    /// `analyse` path, where the body walk writes the real global scope
    /// directly.
    pub(super) capture_global_defs: Option<Vec<(String, tcl_lexer::Token, tcl_lexer::Span)>>,
    /// Deferred W002 (disabled-in-dialect command) diagnostics — both the
    /// whole-command form ([`Self::emit_w002_disabled_command`]) and the
    /// subcommand form embedded in
    /// [`Self::emit_w001_unknown_subcommand`]. Always deferred (never emitted
    /// inline) because the user-definition-shadowing suppression check needs
    /// the fully-merged, whole-file facts (`all_procs`, `command_aliases`,
    /// `renamed_commands`, `all_classes`, `ensemble_namespaces`) — on the
    /// per-item path an isolated body only has its own, so a would-be-W002
    /// site that the body can't prove shadowed is captured here as `(command
    /// name, call-site namespace, enforce_order, diagnostic)` — the same
    /// shape as [`Self::pending_arity`] — and [`Self::graft_proc_body`]
    /// rebases the span via [`super::per_item::rebase_fragment`].
    /// [`Self::flush_disabled_command_diagnostics`] resolves every candidate
    /// (both paths alike) against the merged facts in the tail, using the
    /// same current-namespace-then-global resolution and top-level order gate
    /// as [`Self::flush_arity_diagnostics`].
    pub(super) pending_disabled_commands: Vec<(String, String, bool, super::types::Diagnostic)>,
    /// Original private namespace source-policy receipts and their reporting
    /// diagnostics. Whole-source/input/head currency is rejoined at the final
    /// flush; source declaration/dependency suppressions use actual issuers.
    pub(super) pending_w143: Vec<super::diagnostics::PendingPrivateTclSourceAdvice>,
    /// Compatibility buffer for deferred W304 presentation. Original source
    /// option advice emits at its authenticated invocation; the tail clears
    /// this buffer without inferring a value from text or another scope.
    pub(super) pending_w304: Vec<(
        tcl_lexer::Token,
        String,
        Vec<super::types::CodeFix>,
        tcl_lexer::Span,
    )>,
    /// **Experimental probe flag.**  When `true`, the per-item path does *not*
    /// take the `body_needs_enclosing_context` fallback (bodies that declare /
    /// write enclosing-scope variables or define classes).  Used by the
    /// `per_item_divergence` probe to surface exactly what still diverges so the
    /// fast path can be widened to cover them.  The genuine correctness
    /// fallbacks (recovery, duplicate definitions) still fire.  Defaults to
    /// `false`; production paths are unaffected.
    pub probe_skip_enclosing_fallback: bool,
    /// Deferred `TclOO` instance-creation candidates (`set v [Cls new]` /
    /// `Cls create v`) captured on the per-item path — by an isolated proc
    /// body (whose `all_classes` is empty, so the sibling class can't
    /// resolve there) *and* by the shell pass (whose `all_classes` lacks
    /// every deferred body's classes).  Each is the raw `(command, args,
    /// creation namespace, site offset)`;
    /// [`Self::replay_deferred_instances`] replays them all post-graft, in
    /// source order, resolving each against only the classes whose
    /// definition precedes the site — the class universe the whole-file
    /// DFS had at that walk point.  `None` on the whole-file path
    /// (instances resolve inline).
    pub(super) pending_instances: Option<Vec<PendingInstanceCreation>>,
    /// Post-graft instance-creation replay queue for
    /// [`Self::replay_deferred_instances`]: `(site offset, from a method
    /// body, command, args, creation namespace)` — the shell's captures
    /// plus every grafted body's (already rebased to absolute offsets),
    /// merged so the replay can run in one global source-order pass.
    pub(super) deferred_instance_replays: Vec<(u32, bool, String, Vec<String>, String)>,
    /// Complete original bareword receiver sites retained during per-item scans.
    /// Once canonical declarations join, the shared source-instance issuer
    /// validates each site's ordered context and mutation blockers. Labels in
    /// presentation maps never admit a receiver. Whole-document scans use the
    /// same issuer inline.
    pub(super) pending_bareword_dispatch_sites: Option<Vec<VarCommandSite>>,
    /// **Experimental probe flag.**  When `true`, the per-item path does *not*
    /// take the duplicate-definition fallback, to measure the residual
    /// divergence the duplicate fast-path must still close.  Defaults to `false`.
    pub probe_skip_duplicate_fallback: bool,
    /// **Probe telemetry.**  Set by [`Self::analyse_per_item_with`] to `true`
    /// when the run completed on the incremental per-item path and `false` when
    /// it fell back to a full rebuild.  Read by perf/coverage probes; ignored by
    /// production callers (which only consume the returned `AnalysisResult`).
    ///
    /// Equivalent to `per_item_fallback.is_none()` — read that instead when you
    /// need to know *which* gate fired.
    pub took_fast_path: bool,
    /// **Probe telemetry.**  `None` when the run stayed on the incremental
    /// per-item path; otherwise the gate that forced the full rebuild.
    ///
    /// `took_fast_path` answers "did we pay for a whole-file walk?", which is
    /// the latency question; this answers "why?", which is the only actionable
    /// one.  The two are always set together.
    pub per_item_fallback: Option<super::per_item::PerItemFallback>,
    /// iRules file-profile cache for IRULE1001's informational profile hint:
    /// the sorted, fully-expanded profile stack derived from any
    /// `# profiles:` directive plus the profiles implied by the file's
    /// `when` events.  Computed once per
    /// `analyse` run (only under the `f5-irules` dialect) and cleared at the
    /// top of each run.  `None` outside an active iRules analysis.
    pub(super) irules_file_profiles: Option<Vec<String>>,
    /// IRULE4003's per-file event-body index: `(event name, body texts)` for
    /// every event-handler command in the document, built once from the
    /// segmenter and the registry's `IS_EVENT_HANDLER` / `ArgRole::Body`
    /// facts.  Cleared at the top of each analysis run alongside
    /// [`Self::irules_file_profiles`]; `None` until the first IRULE4003
    /// candidate asks for it.
    pub(super) irules_event_bodies: Option<Vec<(String, Vec<String>)>>,
    /// IRULE5001's debug-flag index: every variable name assigned by an event
    /// that runs **less often than once per request**, so a hot-event `log`
    /// gated on one of them is recognised as gated.  This is the half of the
    /// check that makes its own advice
    /// ("set a debug flag in a setup event and gate with `if {$debug}`")
    /// true; the `static::` half needs no index.  Built lazily from
    /// [`Self::irules_event_bodies`] and cleared with it at the top of each
    /// analysis run.
    pub(super) irules_debug_flags: Option<Vec<crate::connection_scope::EventCell>>,
    /// Creation calls the walk could not classify because their head was not
    /// yet known to be a class factory — replayed once the parameterised-class
    /// observation join has settled.
    ///
    /// A metaclass whose identity is proved only by that post-pass does not
    /// exist while the walk runs, so `Meta create ::T::W { … }` in the same
    /// document reads as an ordinary call to an unknown command and the class
    /// it makes goes unrecorded — no factory, no members, no completion. The
    /// verdict on such a head cannot be formed during the walk at all, so it
    /// is buffered and formed afterwards, the same shape version-floor arity
    /// resolution uses.
    ///
    /// Cleared at the top of each `analyse` run.
    pub(super) deferred_class_creations: Vec<super::types::DeferredClassCreation>,
}

/// W108 non-ASCII detection mode for the `tclLsp.style.nonAscii`
/// setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NonAsciiMode {
    /// No explicit setting: resolve per dialect at emit time — `Strict`
    /// for F5 iRules/iApps (ASCII-only environments), `Confusables`
    /// otherwise.
    #[default]
    Default,
    /// Disable W108 entirely.
    Off,
    /// Flag every non-ASCII character.
    Strict,
    /// Flag Unicode confusables + known copy-paste artifacts only.
    Confusables,
    /// Allow intentional Unicode (letters / numbers / marks / symbols /
    /// punctuation in any script); flag only confusables, artifacts, and
    /// non-benign characters (control / format / separators / surrogates
    /// / private-use / unassigned).
    Common,
}

impl Analyser {
    /// Construct a fresh analyser with no disabled diagnostics.
    ///
    /// All state defaults to empty. The result's ``global_scope``
    /// is the canonical top-level ``::`` scope; ``current_scope_path``
    /// starts empty so the analyser begins at the global scope.
    #[must_use]
    pub fn new() -> Self {
        Self::with_disabled_diagnostics(HashSet::new())
    }
}

pub(super) fn compute_line_offsets(source: &str) -> Vec<usize> {
    source
        .as_bytes()
        .iter()
        .enumerate()
        .filter_map(|(i, &b)| (b == b'\n').then_some(i))
        .collect()
}

/// Convert a byte offset to a 0-based line number using a
/// precomputed sorted ``line_offsets`` vector (see
/// [`compute_line_offsets`]).
pub(super) fn line_at_offset(line_offsets: &[usize], offset: usize) -> i32 {
    // Each offset in ``line_offsets`` is the byte position of a
    // ``\n``.  The line number containing byte `offset` is the
    // count of newlines strictly before ``offset``.  The map
    // value type is ``i32`` because the ``-1`` sentinel encodes
    // file-wide ``# tcl-lsp: disable=`` directives — see the
    // dispatch in ``Analyser::analyse``.  Realistic source files
    // have far fewer than ``i32::MAX`` lines, so saturate
    // gracefully rather than panic on the unrealistic overflow
    // case (a 2-billion-line file would have already exceeded
    // every other in-memory limit).
    i32::try_from(line_offsets.partition_point(|&p| p < offset)).unwrap_or(i32::MAX)
}

/// Merge a set of ``# noqa``-derived line suppressions into the
/// analyser's ``suppressed_lines`` map.
///
/// Called after each ``parse_noqa_line_suppressions`` by all three
/// entry points so the merge logic stays in one place.
pub(super) fn merge_noqa_line_suppressions(
    suppressed_lines: &mut std::collections::HashMap<i32, std::collections::HashSet<String>>,
    line_codes: std::collections::HashMap<i32, std::collections::HashSet<String>>,
) {
    for (line, codes) in line_codes {
        suppressed_lines.entry(line).or_default().extend(codes);
    }
}

impl Default for Analyser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod retained_configuration_consumer_tests;
