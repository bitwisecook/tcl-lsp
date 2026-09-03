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
//! modules (``commands.rs``, ``proc.rs``, ``oo.rs``, ``diagnostics/``,
//! …) but all operate on the same ``&mut Analyser``.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_core_types::DiagCode;

use tcl_lexer::Span;

use super::types::AnalysisResult;

/// One captured instance-creation site on the per-item path: the raw
/// `(command, args, creation namespace, site offset)` handed back to
/// [`Analyser::record_instance_creation`] by the post-graft replay.
pub(super) type PendingInstanceCreation = (String, Vec<String>, String, u32);

/// One `$cmd`-head dispatch site (M7), pending settlement against the
/// compiler's flow-sensitive value model once the CFG/SSA
/// `CompilationUnit` is built (issue #945 faults 1–2: the value and its
/// writable provenance come from the SSA use-version's reaching
/// constant definitions, never from a lexical last-write-wins map).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ConstDispatchSite {
    /// The dispatching variable's name (no leading `$`).
    pub var_name: String,
    /// Span of the `$cmd` head token (the reference anchor).
    pub span: Span,
    /// Command-resolution namespace at the dispatch site.
    pub ns: String,
    /// The head word carries `{*}` expansion (`{*}$cmd args…`): the
    /// value is a **command prefix** — its first list element names the
    /// command, and the writable provenance narrows to that element's
    /// sub-span within the defining literal (issue #945 fault 1's
    /// list-prefix requirement).  Without expansion the whole value is
    /// the command name.
    pub head_expanded: bool,
}

/// One `$class`-headed `TclOO` instance-creation site (issue #923 idx
/// 121), pending settlement against the compiler's flow-sensitive value
/// model once the CFG/SSA `CompilationUnit` is built — the same
/// settle-late discipline [`ConstDispatchSite`] uses, since `class_var`'s
/// value can't be proven constant until SSA reaching-definitions exist.
/// Recorded by [`super::commands::Analyser::record_instance_creation`]
/// when the constructor call's class head is a plain `$var` reference
/// instead of the literal bareword
/// [`super::commands::Analyser::class_from_constructor_subst`] resolves
/// directly (`set class ::Derived; set obj [$class create NAME]`,
/// tcllib's `httpd/httpd.tcl`).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PendingInstanceClassSite {
    /// The class-naming variable's name (no leading `$`).
    pub class_var: String,
    /// The word dispatched on the class command.  It is retained until the
    /// class variable resolves so that class's registry grammar can decide
    /// whether the word is a manufacturer.
    pub manufacturer_word: String,
    /// Span of the `$class` head, for the SSA use-position lookup.
    pub span: Span,
    /// The `instance_classes` key to bind once `class_var` resolves to a
    /// single known user class — the assigned variable (`set obj [...]`)
    /// or created instance-command name.
    pub target_name: String,
}

/// One `<ensemble> <subcommand> …` call site the shell pass could not
/// resolve yet (issue #923 idx 85), held until every deferred proc/method
/// body has been walked.
///
/// The whole-file DFS walks a proc body at its *definition* point, so a
/// `namespace ensemble create -map` written inside `proc ::app::widget::Setup`
/// is on the books long before a top-level `::app::widget show` further down
/// the file. The per-item shell pass defers that body, so the identical call
/// site is walked while `ensemble_subcommand_targets` is still empty and the
/// subcommand reference is silently never recorded — go-to-definition
/// (an on-demand lookup against the *finished* analysis) answered, but
/// find-references / rename / code-lens / call-hierarchy (which enumerate
/// recorded invocations) could not. Deferring the miss and replaying it in
/// [`Analyser::flush_pending_ensemble_subcommand_invocations`] restores the
/// whole-file result exactly.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PendingEnsembleSubcommand {
    /// The ensemble's own resolved command name — the
    /// [`super::types::AnalysisResult::ensemble_subcommand_targets`] key the
    /// walk would have looked up, computed exactly as the immediate path
    /// computes it so the replay cannot resolve differently.
    pub ensemble: String,
    /// The static subcommand word.
    pub sub: String,
    /// The subcommand word's span — the reference this records.
    pub span: Span,
    /// Arguments *after* the consumed subcommand word (`None` when any is
    /// `{*}`-expanded), the same convention the immediate path uses.
    pub argc: Option<usize>,
}

/// One W315 candidate from an `oo::objdefine` walk (issue #1170), held
/// until the whole document is walked so
/// [`Analyser::flush_objdefine_abort_diagnostics`] can consult
/// document-wide facts before deciding it is real.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ObjdefineAbortCandidate {
    /// The definition-aborting word, as the seeded walk recorded it.
    pub abort: super::types::DefinitionAbort,
    /// The receiver key the block was recorded under (variable simple name
    /// or resolved object name).
    pub receiver: String,
    /// The binding's accumulated table held conditional evidence when this
    /// block was walked — absence/presence judgements against it are not
    /// order-provable, so the emission abstains.
    pub prior_state_conditional: bool,
}

/// The recorded state of one child interpreter (issue #945 faults 7–8):
/// safe flag plus the explicit hide / expose deltas layered over the
/// registry's [`tcl_registry::Traits::SAFE_INTERP_HIDDEN`] base set.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct InterpState {
    /// Created with `-safe`.
    pub safe: bool,
    /// Commands explicitly `interp hide`-den in this interpreter.
    pub hidden: HashSet<String>,
    /// Names callable regardless of the safe-hidden base set: explicit
    /// `interp expose` targets, **and** names the interpreter has locally
    /// (re)defined (e.g. `proc source {} {…}` inside its body) — C creates
    /// those in the ordinary command table, entirely independent of the
    /// separate hidden-command table, so a hidden built-in's name becomes
    /// callable the moment the child defines its own command by that name
    /// (tclsh 9.0.4-verified; issue #945 fault 7 follow-up).
    pub exposed: HashSet<String>,
    /// A hide / expose operation on this interpreter used a dynamic
    /// command operand — its visible command set is unknowable, so the
    /// safe-context gate abstains entirely for its evaluation bodies.
    pub tainted: bool,
}

/// One child-interpreter evaluation context on the walk stack — the
/// effective command-visibility state for the `interp eval` body being
/// walked.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct SafeInterpCtx {
    /// The registry's [`tcl_registry::Traits::SAFE_INTERP_HIDDEN`] base
    /// set applies (the interpreter was created `-safe`).  A normal
    /// interpreter with explicit `interp hide`s carries `false` — only
    /// its own hidden set applies.
    pub base_hidden: bool,
    /// Commands explicitly hidden in this interpreter.
    pub hidden_extra: HashSet<String>,
    /// Commands re-exposed over the base set.
    pub exposed: HashSet<String>,
}

/// One `interp eval` body on the walk stack — the interpreter-domain
/// identity of the script currently being analysed.
///
/// Pushed by `isolate_interp_eval_body` (`super::handlers`) for both
/// `interp eval PATH {…}` and the handle form `NAME eval {…}`, so every piece
/// of analyser state that models *per-interpreter runtime state* can key
/// itself by the domain the code actually executes in rather than merging
/// every interpreter's state into one flat file-wide bucket.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct InterpFrame {
    /// The interpreter path key relative-qualified against the enclosing
    /// frames (`s`, `s t`) — what `interp` operations *inside* the body
    /// qualify their own path operands against.
    pub key: String,
    /// The synthetic `@interp@<key>[#<epoch>]` domain identity minted for
    /// this body's scope (see `interp_domain_name` in `super::handlers`).
    /// Two evals into the same live interpreter share it; a
    /// deleted-and-recreated path does not.
    pub domain: String,
    /// `false` once this frame — or any frame enclosing it — targeted an
    /// interpreter whose path could not be resolved statically
    /// (`interp eval $unknown {…}`). The body then really runs in *some*
    /// interpreter we cannot name, so per-domain state must widen rather
    /// than treat the domain as distinct from every other.
    pub resolved: bool,
}

/// How a [`VarCommandSite`]'s command head names the object it dispatches
/// on — the axis that decides *where the receiver's class comes from* and
/// *which of its methods the call can reach*.
///
/// Every dispatch site the walker records is one of these three; adding a
/// fourth spelling means adding a variant here and a resolution arm beside
/// the others, never a name test in the walker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchReceiver {
    /// `$obj method` — the head is a variable whose *value* is an object
    /// handle.  A `$var` can hold anything at run time, so its class
    /// evidence comes from the SSA type lattice / constructor harvest only.
    Variable,
    /// `objcmd method` — a bareword *named* instance command bound by
    /// `CLASS create NAME` (issue #1312).  Its class comes from
    /// `AnalysisResult::instance_classes` gated on
    /// `AnalysisResult::created_instance_commands` — the same contract the
    /// LSP's `receiver_instance_class` uses for hover/definition/completion.
    InstanceCommand,
    /// `my method` — a bareword head the registry declares a **self-dispatch
    /// keyword** (`CommandRegistry::method_dispatch_keyword` answering
    /// `SelfDispatch`, issue #1050).  The receiver is the object whose
    /// method body encloses the call, so the class comes from
    /// `Analyser::enclosing_class_at_offset` with no name resolution at all
    /// — and, uniquely among the three, the dispatch bypasses export
    /// filtering, so it can reach unexported members (issue #1329).
    SelfDispatch,
}

impl DispatchReceiver {
    /// How this head reaches the receiver, for the registry's built-in
    /// object-method visibility rule.
    ///
    /// Only [`Self::SelfDispatch`] bypasses export filtering.  A bareword
    /// instance command is still the object's *own command*, so it sees
    /// exactly what `$obj` sees.
    pub(crate) fn method_reach(self) -> tcl_registry::definer::MethodReach {
        match self {
            Self::SelfDispatch => tcl_registry::definer::MethodReach::SelfDispatch,
            Self::Variable | Self::InstanceCommand => {
                tcl_registry::definer::MethodReach::ObjectCommand
            }
        }
    }
}

/// One entry in [`Analyser::var_command_sites`] —
/// `(var_name, method_name?, cmd_token_span, in_method)`.
///
/// Used by the W307 (variable-as-command misuse) post-pass.
#[derive(Debug, Clone, PartialEq)]
pub struct VarCommandSite {
    /// Variable name used as a command head (no leading ``$``).
    pub var_name: String,
    /// Optional method name when the call shape is
    /// ``$obj method args…``.
    pub method_name: Option<String>,
    /// Content span of the method-name word (delimiters trimmed), when
    /// [`Self::method_name`] is present — the tight anchor for W308.
    pub method_span: Option<Span>,
    /// Span of the command-head token.
    pub cmd_span: Span,
    /// True when the call site is inside a class method body.
    pub in_method: bool,
    /// Number of positional arguments passed at the dispatch site
    /// (`$cmd a b c` → 3, i.e. including the method-name word for a
    /// `$obj method a b` shape).  Used by the dispatch-protocol W214
    /// suppression to require an arity-compatible dispatcher.
    pub argc: usize,
    /// True when any word at the dispatch site (method name or
    /// arguments) is `{*}`-expanded, making the runtime argument count
    /// unknowable statically.  The W308 method-arity check abstains
    /// entirely when this is set, matching every other arity check's
    /// `{*}`-expansion convention.
    pub has_expand: bool,
    /// Which spelling named the receiver — see [`DispatchReceiver`].
    ///
    /// Kept as an explicit recorded fact rather than re-derived from the
    /// site's shape at diagnosis time, so the diagnostic and the LSP's
    /// shared resolver can never disagree about which sites a given
    /// receiver lookup applies to.
    pub receiver: DispatchReceiver,
}

impl VarCommandSite {
    /// Shift every span this site carries by `delta`.
    ///
    /// The **one** place a `VarCommandSite`'s spans are relocated from a
    /// per-item body fragment's local offsets into the whole document's, so
    /// a span field added later cannot be silently left un-rebased.  It once
    /// could: `method_span` was omitted from the hand-written rebase loop,
    /// and every W308 raised inside a proc or method body on the incremental
    /// path was reported — and its "did you mean" quick-fix anchored — at
    /// the *fragment's* offsets, landing on unrelated text elsewhere in the
    /// file (issue #1330).
    pub(crate) fn rebase(&mut self, delta: u32) {
        self.cmd_span = shift_span(self.cmd_span, delta);
        self.method_span = self.method_span.map(|sp| shift_span(sp, delta));
    }
}

/// One entry in [`Analyser::cmd_command_sites`] —
/// `([cmd] text, method_name?, cmd_token_span, in_method)`.
///
/// Same shape as [`VarCommandSite`] except the head is a
/// command-substitution rather than a variable.
#[derive(Debug, Clone, PartialEq)]
pub struct CmdCommandSite {
    /// Text of the bracketed command substitution (no brackets).
    pub cmd_text: String,
    /// Optional method name.
    pub method_name: Option<String>,
    /// Content span of the method-name word (delimiters trimmed), when
    /// [`Self::method_name`] is present — the tight anchor for W308.
    pub method_span: Option<Span>,
    /// Span of the whole `[…]` command-substitution head word, closing
    /// bracket included.
    ///
    /// Widened past the head token's own span with
    /// [`tcl_lexer::word_span`]: a `Cmd` token's raw span stops at the end
    /// of its *content*, so anchoring a diagnostic on it underlines
    /// `[Dog new` rather than `[Dog new]`, and on a head written across a
    /// line continuation it underlines a region that ends mid-word on the
    /// next line (issue #1330).  Only the end moves — every consumer that
    /// keys off `cmd_span.start()` (enclosing class, child-interp
    /// containment, head return type) is unaffected.
    pub cmd_span: Span,
    /// True when inside a class method body.
    pub in_method: bool,
}

impl CmdCommandSite {
    /// Shift every span this site carries by `delta` — see
    /// [`VarCommandSite::rebase`] for why this is a method rather than a
    /// per-field loop at the call site.
    pub(crate) fn rebase(&mut self, delta: u32) {
        self.cmd_span = shift_span(self.cmd_span, delta);
        self.method_span = self.method_span.map(|sp| shift_span(sp, delta));
    }
}

/// Shift `span` right by `delta` — the primitive
/// [`VarCommandSite::rebase`] / [`CmdCommandSite::rebase`] are built from.
fn shift_span(span: Span, delta: u32) -> Span {
    Span::new(span.start() + delta, span.end() + delta)
}

/// Single-pass Tcl analyser.
///
/// Constructed once per document, walked end-to-end, then dropped.
// False positive: a flat accumulator whose bools are independent pass
// flags / config toggles (`structure_only`, `defer_proc_bodies`,
// `deep_param_traits`, `took_fast_path`, the two `probe_skip_*` test hooks,
// …) that combine freely — not a state machine, so no natural enum.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct Analyser {
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
    /// source — recovery and CFG/SSA diagnostic emission.
    pub source: String,
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
    /// The resolved document environment (centralisation R-a): set beside
    /// [`Self::profile`] at each `analyse*` ingress by
    /// [`crate::environment_ingress::resolve_environment`]. The profile
    /// above is now *derived from* this resolution (wave-1 interop,
    /// retired with ledger C1's re-type); availability queries go through
    /// [`Self::analysis_context`] instead of the profile.
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
    /// all. [`Self::profile_registry`] turns it into the registry, falling
    /// back to the un-overlaid one when nothing has been installed under that
    /// key.
    ///
    /// It is load-bearing since the EDA vendor libraries became bundled
    /// `.tclspec` loadables (`docs/design/spec-packs.md`): without it a
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
    /// Source-file path (for the LSP `Diagnostic.uri` field), or
    /// `None` when analysing in-memory text.
    pub file_path: Option<String>,
    /// Per-scope const-string tracker:
    /// ``scope_kind_path → { var_name → (value, span) }``.
    /// Keyed on the path vector so snapshot/restore doesn't have to
    /// remap pointers.
    pub const_strings: HashMap<Vec<usize>, HashMap<String, (String, Span)>>,
    /// Per-scope set of const-string bindings whose *current* value was
    /// last written inside an ``if`` / ``try`` body (``conditional_depth >
    /// 0``) — a write that does **not** dominate code after the conditional
    /// join. [`Self::set_const_string`] adds a name here when the write is
    /// conditional and removes it on a later straight-line (depth-0) write;
    /// [`Self::lookup_dominating_const_string`] consults it so identity
    /// resolution (`source`/`rename` targets via
    /// [`Self::resolve_dynamic_word`]) abstains rather than pick a
    /// branch-dependent value (Codex review, PR #1020). Kept as a side set
    /// so the many other `const_strings` readers (regex vars, expansion
    /// counts, …) keep their existing last-write-wins behaviour untouched.
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
    /// P3 (ledger F4): resolved at ingest from the environment's placement
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
    /// them produced a false TK1001 and a missed TK1002 (issue #1141).
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
    /// D5 oldest-supported default; feeds
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
    /// through the whole call graph (issue #1015).
    ///
    /// Populated once by [`Self::finalise_invocation_resolutions`] from the
    /// already-settled `command_invocations`, and consulted by
    /// [`Self::fact_live_for_call`] when the call site under test is itself
    /// nested inside a body: a proven invocation of the *enclosing*
    /// definition that ran before a later unconditional deletion means that
    /// invocation's own nested calls already resolved (issue #1009 Codex
    /// review: `proc helper {}`, `proc caller {} { helper }`, `caller`,
    /// `rename helper {}` resolves in real Tcl — confirmed against tclsh
    /// 8.6.14 — because `caller`'s own top-level invocation runs before the
    /// rename; issue #1015 extends that through an arbitrary chain of
    /// enclosing definitions).
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
    /// Oracle (tclsh8.6, `review-probes-sound/r3.tcl`): `proc a {} {
    /// helper }`, `a`, `rename helper {}`, `a` — the second `a` really does
    /// fail with `invalid command name "helper"`, and no W123 is reported.
    ///
    /// This is unchanged by design: reporting it needs a per-invocation
    /// reachability interval rather than a single floor, and the escape
    /// hatch exists precisely to stop the far commoner call-before-deletion
    /// shape being flagged. Issue #1015 widened the false negative — a
    /// transitive chain now reaches through arbitrarily many bodies — but
    /// did not introduce it.
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
    /// `$cmd`-head dispatch sites (M7): every simple-`$var` command head,
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
    /// reference (issue #923 idx 121), pending settlement alongside
    /// [`Self::pending_const_dispatches`] once the CFG/SSA
    /// `CompilationUnit` exists.
    pub(super) pending_instance_class_sites: Vec<PendingInstanceClassSite>,
    /// M9: the namespace key a seeded analysis wraps the whole file in (set
    /// by [`Analyser::analyse_with_source_namespace`]); the scope chain it
    /// creates becomes the top-level walk's base path.
    pub(super) seed_namespace_key: Option<String>,
    /// Scope path of the innermost seeded namespace scope (empty when not
    /// seeded) — the base path for the top-level walk.
    pub(super) seed_scope_path: Vec<usize>,
    /// Tk widget instance-dispatch candidates (`.t instate …`, `$w tag
    /// configure …`) whose head the ordinary registry-command resolution
    /// could not resolve; resolved post-walk by
    /// [`super::diagnostics::widget_command::Analyser::flush_widget_dispatch_diagnostics`]
    /// once `instance_classes` is complete (issue #927).
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
    /// map by [`Self::flush_pending_ensemble_subcommand_invocations`]
    /// (issue #923 idx 85). Only the shell pass fills this: the whole-file
    /// DFS and the isolated body pass both resolve at walk time against a
    /// map that already holds everything their own walk order could see.
    pub(super) pending_ensemble_subcommands: Vec<PendingEnsembleSubcommand>,
    /// `namespace ensemble create|configure ... -map {sub target ...}`
    /// subcommand-to-target maps, keyed by the ensemble's own qualified
    /// command name (`-command NAME`, or the enclosing namespace's own
    /// qualified name when `-command` is absent — Tcl's default). Consulted
    /// by the W129 safe-interpreter gate (issue #1001 follow-up, tracked
    /// separately from #979's interprocedural call-site concern) so a
    /// hidden command reached only through an ensemble redirect (`myens sub
    /// ...` → target) is still flagged, mirroring a literal call to the
    /// target.
    pub ensemble_command_maps: HashMap<String, HashMap<String, String>>,
    /// Vars where ``oo::objdefine`` was applied — the per-instance
    /// method table may extend the class definition.
    pub objdefined_vars: HashSet<String>,
    /// Per-object member-state binding index (issue #1170): `(receiver key,
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
    /// The **interpreter-domain map** (issue #945 faults 7–8): every
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
    /// Per-interpreter **deletion epochs** (issue #945 fault 8's temporal
    /// identity): `interp delete` bumps the path's epoch, so a later
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
    /// state modelling *per-interpreter runtime state* keys itself by
    /// (issue #1141).
    pub(super) interp_path_stack: Vec<InterpFrame>,
    /// Safe-interpreter evaluation contexts currently on the walk stack:
    /// non-empty while walking an `interp eval` body whose target
    /// interpreter is safe.  The per-command gate consults the top —
    /// a command whose registry spec carries
    /// [`tcl_registry::Traits::SAFE_INTERP_HIDDEN`] (or was
    /// `interp hide`-den), and is not re-exposed, draws W129 and has no
    /// analysed effect (C raises `invalid command name` before any
    /// side-effect happens — so no source / package / definition edges
    /// may be built from it either; issue #945 fault 7).
    pub(super) safe_interp_stack: Vec<SafeInterpCtx>,
    /// The scope-chain-aware **interpreter value-flow map** (issue #923
    /// idx 9): `set VAR [interp create ...]` binds `VAR`, in the scope it
    /// was written, to the `interpreters` domain key recorded for that
    /// call — a literal path's qualified key, or a synthetic
    /// per-call-site key when the call captured no literal path. Mirrors
    /// `const_strings`'s scope-chain shape (never `instance_classes`'s
    /// flat, file-wide one — a raw-name collision there only softens a
    /// diagnostic; here it would corrupt real go-to-definition targets),
    /// so two unrelated procs binding the same variable name to
    /// different interpreters never collide. Consulted by
    /// [`Self::resolve_dynamic_interp_path`] from the call sites that
    /// used to require the interpreter path to be a source literal.
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
    /// constant command-substitution fold (issue #1132). The analyser's own
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
    /// provenance-tagged surface declarations (gap ruling R1).  Paired
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
    /// bindings rather than their written spellings (issue #1275).  Empty
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
    /// safely under the analyser's caps (issue #996). Use
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
    /// (not just ones with a registry signature) and resolved in the
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
    /// name — Tk's `::tk::IconList .il` (issue #1303).
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
    /// records nothing (issue #1276).  Each entry is the factory description
    /// the metaclass's *own* document derived
    /// ([`super::types::ClassDef::factory`]), so the consuming walk classifies
    /// the call from a proved fact rather than its shape.  `None` — the
    /// default, and every single-file analysis — keeps the abstention exactly
    /// as it was.
    pub workspace_class_factories: Option<std::sync::Arc<super::types::ClassFactoryIndex>>,
    /// Instance methods dispatchable on some workspace **descendant** of
    /// each class, keyed by ancestor qualified name — the cross-file half of
    /// the template-method abstention (issue #1367).  A base class calling
    /// `my Render` where `Render` is written only by a subclass in another
    /// document is the same deliberate pattern as the single-file shape the
    /// local hierarchy proves; without this view the workspace still drew
    /// the W308 the sibling file refutes.  `None` — the default, and every
    /// single-file analysis — changes nothing: the warning keeps firing
    /// exactly as the per-file evidence dictates.
    pub workspace_subclass_methods: Option<std::sync::Arc<super::types::SubclassProvidedMethods>>,
    /// When `true` (the per-item shell walk), `handle_proc_command` / OO method
    /// walks record their body for separate analysis (`deferred_bodies`) instead
    /// of recursing into it immediately.  Set only for the shell pass; the
    /// per-body passes run with it `false` so nested defs walk in place.
    pub defer_proc_bodies: bool,
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
    /// Deferred W103 / W300 dynamic-argument sites from an **isolated
    /// proc-body** pass (`(code, command name, argument token)`).  Their
    /// `$var` classification resolves the variable against the most recent
    /// literal `set` in the *whole file*
    /// ([`super::diagnostics::validity::last_literal_set_value_for_var`]
    /// scans `self.source`) — an isolated body's `self.source` is only the
    /// body, so resolving there both misses enclosing-scope sets *and* sees
    /// body-local sets the whole-file truncated-prefix scan cannot segment
    /// (they sit inside an unclosed `proc`).  Captured on the per-item path
    /// only and flushed by [`Self::flush_var_literal_checks`] in the tail,
    /// where `self.source` is the full file — the same split
    /// [`Self::pending_w304`] uses for the identical reason.
    pub(super) pending_var_literal_checks: Vec<(DiagCode, String, tcl_lexer::Token)>,
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
    pub(super) cu_override: Option<std::sync::Arc<crate::compilation_unit::CompilationUnit>>,
    /// When `Some`, an isolated proc-body analysis (the per-item path) records
    /// every qualified (`::` / `static::`) variable read that fell through to
    /// the (empty) enclosing global scope here, instead of dropping it.  The
    /// aggregator replays these on the shell's global scope during the graft, so
    /// a body's `$::g` read lands as a reference on the real enclosing `::g`
    /// def — one of the divergences the per-item path must reproduce.  `None` on
    /// the whole-file `analyse` path (reads resolve against the populated scope).
    pub(super) capture_global_reads: Option<Vec<(String, tcl_lexer::Span)>>,
    /// The write-side twin of [`Self::capture_global_reads`].  When `Some`,
    /// an isolated proc-body analysis (the per-item path) records every
    /// variable the body defines **directly in the global scope** — a body
    /// that names one fixed, frame-independent cell, which today means
    /// `upvar`'s `otherVar` word (`upvar ::tk::FocusGrab($i) data`, `upvar
    /// #0 counter c`; see
    /// [`Analyser::handle_upvar_command`](super::state::Analyser)).
    ///
    /// The graft merges the fragment's *proc* scope, never its (throwaway)
    /// root, so without this capture such a cell reached `all_variables` but
    /// never the shell's scope tree — and the scope tree is what
    /// `attach_qualified_var_references`, `replay_body_global_reads`, and
    /// every LSP navigation provider read.  The result was the issue #923
    /// audit idx 98 residual: the whole-file walk answered hover /
    /// definition / references for the cell and the live server's
    /// incremental walk answered nothing.  `None` on the whole-file
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
    /// Deferred W143 (private `::tcl::` implementation namespace)
    /// diagnostics, in the same `(command name, call-site namespace,
    /// enforce_order, diagnostic)` shape as
    /// [`Self::pending_disabled_commands`].  Always deferred: the
    /// suppressions W143 needs are whole-file facts an isolated body cannot
    /// see — a `proc ::tcl::dict::mine` defined anywhere in the document
    /// (`UserResolutionFacts`), and a `package require` covering the
    /// qualified name (`result.package_requires`).
    /// [`Self::flush_w143_diagnostics`] applies both in the tail.
    pub(super) pending_w143: Vec<(String, String, bool, super::types::Diagnostic)>,
    /// Deferred W304 (missing `--` option terminator) diagnostics whose
    /// severity/message depend on resolving a `$var` against the **most recent
    /// literal `set` in the whole file** ([`last_literal_set_value_for_var`],
    /// which scans `self.source`).  An isolated body's `self.source` is only the
    /// body, so an enclosing-scope `set` is invisible — the lone source-dependent
    /// W304 branch (`Var`, dynamic, not option-looking).  On the per-item path
    /// such sites are captured here (rebased token + label + body-local fix /
    /// span) instead of emitted, and [`Self::flush_w304_diagnostics`] classifies
    /// them in the tail where `self.source` is the full file.  Empty on the
    /// whole-file path (emitted inline) — byte-identical.
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
    /// Bareword `objcmd method` dispatch sites (issue #1312) captured while
    /// `pending_instances` is active — a `CLASS create NAME` creation earlier
    /// in the same deferred pass has not resolved `instance_classes` yet (it
    /// resolves only post-graft, in [`Self::replay_deferred_instances`]), so
    /// the site is held here instead of going straight into
    /// `var_command_sites`.  Finalised right after that replay: a candidate
    /// whose name the replay actually bound in `instance_classes` becomes a
    /// real `var_command_sites` entry; every other candidate (a coroutine /
    /// `interp create` / registry-factory / external-class name that merely
    /// *looked* like a pending class instance at record time) is dropped —
    /// the same soundness bar the non-deferred `analyse` path applies
    /// immediately.  `None` on the whole-file path (sites resolve inline).
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
    /// Creation calls the walk could not classify because their head was not
    /// yet known to be a class factory — replayed once the parameterised-class
    /// observation join has settled (issue #1660).
    ///
    /// A metaclass whose identity is proved only by that post-pass does not
    /// exist while the walk runs, so `Meta create ::T::W { … }` in the same
    /// document reads as an ordinary call to an unknown command and the class
    /// it makes goes unrecorded — no factory, no members, no completion. The
    /// verdict on such a head cannot be formed during the walk at all, so it
    /// is buffered and formed afterwards, the shape #1642 used for
    /// version-floor arity.
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

    /// The grammar this document is lexed under: the ingress-resolved one
    /// when the walk came through [`Self::resolve_walk_environment`], else
    /// the profile's. Every re-segmentation and every grammar-axis read in
    /// the handlers goes through here, never through `profile.grammar`
    /// directly, so the analyser cannot lex a document under a rule the
    /// ingress did not resolve.
    pub(super) fn grammar(&self) -> tcl_dialect::LexerGrammar {
        self.ingress_grammar.unwrap_or(self.profile.grammar)
    }

    /// The body-lexing config for this document: [`Self::grammar`] — the
    /// ingress-resolved grammar, else the profile's — carrying the
    /// dialect-dependent tokenisation flags (`{*}` expansion, the iRules
    /// `}{` word break, numerals, escapes, Jim's axes). Threaded into every
    /// analyser re-segmentation so a body is read under the document's
    /// grammar, never the default's. [`Self::file_lexer_config`] is the
    /// whole-file form.
    pub(super) fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig::from_grammar(self.grammar())
    }

    /// This document dialect's word-value rules: how a braced word's
    /// `\<newline>` folds and how list text divides. The [`Self::grammar`]
    /// twin for the re-parses that split a *word* rather than re-lex a
    /// script — a proc's parameter list, an OO member's — so they cannot
    /// answer C Tcl's question about a `JimTcl` document.
    pub(super) fn word_rules(&self) -> tcl_syntax::word_rules::WordValueRules {
        tcl_syntax::word_rules::WordValueRules::from_grammar(&self.grammar())
    }

    /// [`Self::lexer_config`] for the **whole-file** segmentation at the top of
    /// [`Self::analyse`] — the one place that stands where a Tcl runtime's
    /// `source` stands, and therefore the only place that may skip a leading
    /// byte-order mark (issue #1218).  Whether it does is the dialect's
    /// business: Tcl 9's `source` strips a leading U+FEFF, Tcl 8.x's does not
    /// and genuinely fails on such a file.
    pub(super) fn file_lexer_config(&self) -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig::for_file_grammar(self.grammar())
    }

    /// A [`tcl_lexer::SourceMap`] over [`Self::source`], built from the
    /// cached [`Self::cached_line_index`] rather than rescanning the
    /// document. Use this instead of `SourceMap::new(&self.source)` in any
    /// handler that runs once per command (i.e. potentially once per
    /// nesting level) — see the doc comment on [`Self::cached_line_index`]
    /// for why the naive rescan is a real `DoS`, not just an inefficiency.
    ///
    /// Falls back to a fresh `SourceMap::new(source)` scan when
    /// `cached_line_index_len != source.len()` — see
    /// [`Self::cached_line_index`]'s doc comment for why the cache can be
    /// stale and why a length mismatch is how this detects it.
    ///
    /// A free function taking the fields explicitly, **not** a `&self`
    /// method: a method call always borrows all of `self`, which would tie
    /// up `self.result` for the returned `SourceMap`'s whole lifetime and
    /// break the many call sites that build a map and later push a
    /// diagnostic in the same scope. Called as
    /// `Analyser::source_map(&self.source, &self.cached_line_index,
    /// self.cached_line_index_source_len)`.
    pub(super) fn source_map<'src>(
        source: &'src str,
        cached_line_index: &tcl_lexer::LineIndex,
        cached_line_index_len: usize,
    ) -> tcl_lexer::SourceMap<'src> {
        if source.len() == cached_line_index_len {
            tcl_lexer::SourceMap::with_line_index(source, cached_line_index.clone())
        } else {
            tcl_lexer::SourceMap::new(source)
        }
    }

    /// [`Self::source_map`] as a `&self` method, for the call sites that can
    /// confine the map to a scope that ends before they need `&mut self`
    /// again (typically: compute the spans into locals, then push the
    /// diagnostic). Borrowing all of `self` is why this cannot replace the
    /// free function — see its doc comment — but where the borrow *is*
    /// scoped this spelling keeps the three-argument call out of the way.
    pub(in crate::analyser) fn cached_source_map(&self) -> tcl_lexer::SourceMap<'_> {
        Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        )
    }

    /// [`Self::source`] sliced by absolute byte offsets, or `None` when the
    /// range is inverted, out of bounds, **or lands inside a multi-byte UTF-8
    /// sequence**.
    ///
    /// The one sanctioned way for a handler to turn a token span back into
    /// source text.  A bare `&self.source[start..end]` panics on a span whose
    /// end is not a `char` boundary, which is reachable from real-world input:
    /// a body word of the shape `{…}x` (Tcl's "extra characters after
    /// close-brace", which this analyser accepts leniently) has its closing
    /// `}` dropped from the segmenter's word *value*, so rebasing that value
    /// by a single offset shifts every token after the brace one byte left —
    /// harmless-looking on ASCII, mid-character on anything else.  An iRule
    /// carrying zero-width spaces (`U+200B`, the usual residue of a
    /// copy-paste from a web page) hit exactly that and aborted `fp-sweep`
    /// outright, while the LSP lost **every** diagnostic for the file and
    /// showed it as clean (issue #1325).
    ///
    /// [`crate::analyser::utils::contiguous_prefix`] fixes that producer, but
    /// spans reach these slices from the lexer, the segmenter, the CST, and
    /// the analyser's own arithmetic, so the consumer stays defensive: a span
    /// this rejects means "no text here", never an abort.
    ///
    /// A free function taking `source` explicitly, **not** a `&self` method,
    /// for the same reason as [`Self::source_map`]: a method call borrows all
    /// of `self`, which would tie up `self.result` for the returned slice's
    /// lifetime and break every call site that slices and then pushes a
    /// diagnostic in the same scope.
    pub(in crate::analyser) fn source_slice(
        source: &str,
        start: usize,
        end: usize,
    ) -> Option<&str> {
        source.get(start..end)
    }

    /// The active dialect's canonical name — the string that round-trips
    /// through configuration and the providers (`self.profile.name`).
    #[must_use]
    pub fn dialect(&self) -> &'static str {
        self.profile.name
    }

    /// The active dialect's `${…}` close rule — the input to the shared owner
    /// [`tcl_lexer::braced_var_name_end`].
    ///
    /// Every analyser scan that recovers a variable name out of free word text
    /// must resolve the closer through the owner under *this* style. Anything
    /// else contradicts the spans the document was lexed with: at 9.x the
    /// lexer makes `${a{b}c}` one `Var` naming `a{b}c`, while a hard-coded
    /// first-`}` scan reads `a{b` and reports on a variable the source never
    /// mentions (issue #1604).
    #[must_use]
    pub fn braced_var(&self) -> tcl_dialect::BracedVarStyle {
        self.profile.grammar.braced_var
    }

    /// Split a `${…}`-headed `Var` token's text (as
    /// [`tcl_lexer::SourceMap::token_text`] returns it, i.e. already past the
    /// `${`) into the dispatched variable name and the word suffix glued to
    /// it.
    ///
    /// The lexer merges a braced substitution and everything concatenated to
    /// it into **one** `Var` token, so `${ns}::setdef` arrives as the raw text
    /// `ns}::setdef` — the name is `ns` and `::setdef` is an ordinary suffix.
    /// A *pure* `${x}` reference arrives as `x`, its closer already excluded
    /// from the span, and is wholly the name.
    ///
    /// Both shapes fall out of the shared owner
    /// [`tcl_lexer::braced_var_name_end`] under this dialect's rule:
    /// `Closed` is the composite case, `Unterminated` the pure one. The
    /// first-`}` split this replaces split `${a{b}c}::setdef` after `a{b`,
    /// naming a variable the source never mentions and handing the rest to
    /// the suffix fold (issue #1604). The three call sites — the const
    /// dispatch record, the W307 head reading, and `resolve_dynamic_word` —
    /// share it so they cannot disagree about which bytes name the variable.
    #[must_use]
    pub(in crate::analyser) fn split_braced_head<'t>(&self, raw: &'t str) -> (&'t str, &'t str) {
        match tcl_lexer::braced_var_name_end(raw.as_bytes(), 0, self.braced_var()) {
            tcl_lexer::BracedVarEnd::Closed(end) => (&raw[..end], &raw[end + 1..]),
            tcl_lexer::BracedVarEnd::Unterminated => (raw, ""),
        }
    }

    /// `true` when `cmd_name`'s registry spec declares its pattern argument
    /// as a regular expression ([`tcl_registry::PatternType::Regex`]) —
    /// `regexp` / `regsub`.  The regex-specific analyses (W303 `ReDoS`, W306
    /// literal-expected, regex-pattern capture) key off this query instead
    /// of hardcoded command names, so a future regex-pattern command is
    /// registry data only.  Falls back to the cached default registry when
    /// the analyser has none loaded (direct handler calls in unit tests).
    pub(super) fn command_takes_regex_pattern(&self, cmd_name: &str) -> bool {
        let registry = self.registry.as_deref().map_or_else(
            || {
                tcl_registry::model::ingress::static_context_for("tcl8.6")
                    .commands()
                    .as_ref()
            },
            |r| r,
        );
        registry.get(cmd_name).and_then(|spec| spec.pattern_type)
            == Some(tcl_registry::PatternType::Regex)
    }

    /// Construct an analyser with a fixed set of diagnostic codes
    /// disabled (e.g. `"W210"`, `"W211"`).
    // One line per field of a struct with more than a hundred of them, which
    // rustfmt will not pack and splitting would only scatter across helpers
    // that each initialise a fifth of one value.
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn with_disabled_diagnostics(disabled: HashSet<String>) -> Self {
        Self {
            result: AnalysisResult::default(),
            current_scope_path: Vec::new(),
            source: String::new(),
            profile: tcl_dialect::DialectProfile::plain_tcl(),
            ingress_grammar: None,
            unit_profile: None,
            environment: None,
            context: None,
            pack_overlay: 0,
            disabled_diagnostics: disabled,
            extra_commands: Arc::new(HashSet::new()),
            last_comment: String::new(),
            file_path: None,
            const_strings: HashMap::new(),
            nondominating_consts: HashMap::new(),
            regex_vars: HashSet::new(),
            current_event: None,
            tk_accumulation_enabled: false,
            tk_ambient: false,
            tk_pending_diags: Vec::new(),
            tk_domains: std::collections::BTreeMap::new(),
            version_gate_sites: Vec::new(),
            dsl_gate_sites: Vec::new(),
            pending_option_conflicts: Vec::new(),
            pending_gated_arity: Vec::new(),
            pending_gated_bare_ensemble: Vec::new(),
            library_versions: tcl_dialect::LibraryVersionOverrides::default(),
            declared_targets: Vec::new(),
            range_context: None,
            range_numeral_grammars: Vec::new(),
            range_gate_sites: Vec::new(),
            builtin_names: None,
            builtin_dialect: None,
            conditional_depth: 0,
            control_flow_body_depth: 0,
            body_depth: 0,
            presubstituted_args: false,
            e207_emitted: false,
            body_scope_stack: Vec::new(),
            command_aliases: HashMap::new(),
            renamed_commands: HashMap::new(),
            alias_offsets: HashMap::new(),
            rename_offsets: HashMap::new(),
            deleted_commands: HashMap::new(),
            reachable_call_offsets: HashMap::new(),
            namespace_paths: HashMap::new(),
            var_command_sites: Vec::new(),
            pending_const_dispatches: Vec::new(),
            pending_instance_class_sites: Vec::new(),
            seed_namespace_key: None,
            seed_scope_path: Vec::new(),
            widget_dispatch_sites: Vec::new(),
            cmd_command_sites: Vec::new(),
            ensemble_namespaces: HashSet::new(),
            ensemble_command_maps: HashMap::new(),
            ensemble_record_offsets: HashMap::new(),
            pending_ensemble_subcommands: Vec::new(),
            objdefined_vars: HashSet::new(),
            objdefine_bindings: HashMap::new(),
            objdefine_abort_candidates: Vec::new(),
            objdefine_unresolved_receiver: false,
            interpreters: HashMap::new(),
            dynamic_interp_ops: false,
            interp_epochs: HashMap::new(),
            interp_path_stack: Vec::new(),
            safe_interp_stack: Vec::new(),
            interp_var_bindings: HashMap::new(),
            unresolved_commands_emitted: false,
            registry: None,
            command_trust: None,
            recovery_known_commands: super::utils::RecoveryKnownCommands::default(),
            deep_param_traits: false,
            declared_commands: None,
            head_identities: crate::realm::CommandBindingRealm::default(),
            line_offsets: None,
            cached_line_index: tcl_lexer::LineIndex::new(""),
            cached_line_index_source_len: 0,
            pending_arity: Vec::new(),
            pending_user_call_arity: Vec::new(),
            pending_ctor_arity: Vec::new(),
            pending_next_arity: Vec::new(),
            non_ascii_mode: NonAsciiMode::Default,
            structure_only: false,
            workspace_classes: std::collections::HashSet::new(),
            workspace_bare_word_classes: std::collections::HashSet::new(),
            workspace_class_factories: None,
            workspace_subclass_methods: None,
            defer_proc_bodies: false,
            structural_rebind: false,
            deferred_bodies: Vec::new(),
            minted_synthetic_names: std::collections::HashSet::new(),
            cu_override: None,
            capture_global_reads: None,
            capture_global_defs: None,
            pending_disabled_commands: Vec::new(),
            pending_w143: Vec::new(),
            pending_w304: Vec::new(),
            pending_var_literal_checks: Vec::new(),
            pending_instances: None,
            deferred_instance_replays: Vec::new(),
            pending_bareword_dispatch_sites: None,
            probe_skip_enclosing_fallback: false,
            probe_skip_duplicate_fallback: false,
            took_fast_path: false,
            per_item_fallback: None,
            irules_file_profiles: None,
            irules_event_bodies: None,
            deferred_class_creations: Vec::new(),
        }
    }

    /// Layer the `SpecTcl` pack set with this identity onto the registry this
    /// analysis reads (`PackSet::key`), returning `self` for builder-style
    /// configuration.
    ///
    /// `0` — the default — means "no packs" and is the plain per-profile
    /// registry. Any other value is looked up, never built: see
    /// [`Self::profile_registry`].
    #[must_use]
    pub fn with_pack_overlay(mut self, key: u64) -> Self {
        self.pack_overlay = key;
        self
    }

    /// The registry this analysis reads: the walk's generation's command
    /// store, carrying [`Self::pack_overlay`]'s packs when that entry
    /// exists.
    ///
    /// **Look-up only.** Building an overlay entry needs the pack
    /// *contents*, which only the loader has, so a miss falls back to the
    /// un-overlaid generation rather than caching a pack-less one under
    /// the pack's key forever. A miss means the packs are not installed
    /// yet — the state the process was in a moment ago — so the fallback
    /// is the honest answer, not a wrong one.
    #[must_use]
    pub fn profile_registry(&self) -> std::sync::Arc<tcl_registry::registry::CommandRegistry> {
        std::sync::Arc::clone(self.analysis_context().commands())
    }

    /// The registry generation this analysis answers under — the
    /// [`tcl_registry::model::ContextRegistry`] stashed at the `analyse*`
    /// ingress ([`Self::resolve_walk_environment`]), or — for a bare
    /// harness driving handlers without an ingress — the generation of
    /// the stashed profile's environment at this walk's overlay key.
    /// Availability queries read `.context()`; raw spec content reads
    /// `.commands()`.
    #[must_use]
    pub(crate) fn analysis_context(&self) -> std::sync::Arc<tcl_registry::model::ContextRegistry> {
        if let Some(context) = &self.context {
            return std::sync::Arc::clone(context);
        }
        let environment = crate::environment_ingress::resolve_environment(self.profile.name);
        let keyed =
            crate::environment_ingress::DocumentEnvironment::keyed_versions(&self.library_versions);
        environment.context_registry(&keyed, self.pack_overlay)
    }

    /// Resolve `dialect` at a walk ingress (centralisation R-a): stash
    /// the environment and this walk's registry generation, derive the
    /// interop [`Self::profile`], and return whether this environment
    /// ships `Tk` **ambient** ([`Self::tk_ambient`] — the fact
    /// `availability_for_name`'s `TK`-bit union used to carry, now a
    /// placement query on the walk's own context).
    pub(super) fn resolve_walk_environment(&mut self, dialect: &str) -> bool {
        let environment = crate::environment_ingress::resolve_environment(dialect);
        self.profile = environment.analyser_profile();
        self.ingress_grammar = Some(environment.grammar());
        self.unit_profile = Some(environment.unit_profile());
        let keyed =
            crate::environment_ingress::DocumentEnvironment::keyed_versions(&self.library_versions);
        let generation = environment.context_registry(&keyed, self.pack_overlay);
        let tk_ambient = generation.context().ambient_package("Tk");
        self.context = Some(generation);
        self.environment = Some(environment);
        tk_ambient
    }

    /// Set the W108 non-ASCII detection mode (`tclLsp.style.nonAscii`),
    /// returning `self` for builder-style configuration.
    #[must_use]
    pub fn with_non_ascii_mode(mut self, mode: NonAsciiMode) -> Self {
        self.non_ascii_mode = mode;
        self
    }

    /// Pin the session's target BIG-IP release (`tclLsp.bigipVersion` /
    /// `--bigip-version`): the keyed library-version axis every declared
    /// range compares against (baseline semantics, §7.1).
    #[must_use]
    pub fn with_bigip_version(mut self, version: Option<String>) -> Self {
        self.library_versions.bigip_version = version;
        self
    }

    /// Set the configuration-declared version targets (`tclLsp.targets`,
    /// §5.4 range targeting) as `(provider, range clauses)` pairs,
    /// returning `self` for builder-style configuration. A source-level
    /// `# tcl-lsp: supports NAME RANGE` directive overrides the
    /// configured pair for the same provider.
    #[must_use]
    pub fn with_declared_targets(mut self, targets: Vec<(String, String)>) -> Self {
        self.declared_targets = targets;
        self
    }

    /// Resolve the §5.4 declared version targets for this walk: the
    /// configured [`Self::declared_targets`] pairs plus the source's
    /// `# tcl-lsp: supports NAME RANGE` directives (the directive wins
    /// per provider — most specific source, §5.4), parsed under the
    /// targets grammar ([`tcl_registry::model::targets_from_clauses`])
    /// and recorded on a document-level clone of this walk's resolved
    /// context. A malformed or empty declaration is dropped rather than
    /// guessed at. A provider named after a **family** declares targets
    /// on that family's core axis and is honoured only when the
    /// document's own core is that family — the Tcl, iRules and Jim
    /// ladders are separate axes, and a declaration on one says nothing
    /// about another (invariant I2).
    ///
    /// Also derives the numeral-grammar era set the W151 cross-target
    /// numeral check walks under; fewer than two represented grammars
    /// switches that check off.
    pub(super) fn resolve_declared_targets(&mut self, source: &str) {
        use tcl_dialect::model::{Family, VersionAxisId};
        self.range_context = None;
        self.range_numeral_grammars = Vec::new();
        let directives = super::utils::parse_supports_directives(source);
        if self.declared_targets.is_empty() && directives.is_empty() {
            return;
        }
        let mut merged: Vec<(String, String)> = self.declared_targets.clone();
        for (name, clauses) in directives {
            merged.retain(|(existing, _)| *existing != name);
            merged.push((name, clauses));
        }
        let generation = self.analysis_context();
        let mut context = generation.context().clone();
        let core_family = context.environment.core.map(|core| core.family);
        let mut declared_any = false;
        for (name, clauses) in &merged {
            // A **family name** (`tcl`, `jim`, `f5-irules`, …) declares
            // targets on that family's own core axis, and only when the
            // document's core is that family: a `supports tcl 8.5-9.0`
            // under a jim core, or a `supports jim 0.81-` under a Tcl
            // one, is a declaration about a ladder this document is not
            // on, and invariant I2 says it must be dropped rather than
            // coerced. Before P6 only `tcl` was recognised here, so
            // `supports jim 0.81-` minted a fictitious *package* axis
            // named `jim` and switched range mode on against it.
            let family = Family::ALL
                .into_iter()
                .find(|family| name.eq_ignore_ascii_case(family.name()));
            let axis = match family {
                Some(family) if core_family == Some(family) => VersionAxisId::core(family),
                Some(_) => continue,
                None => VersionAxisId::package(name),
            };
            let clause_list: Vec<&str> = clauses.split_whitespace().collect();
            let Ok(targets) = tcl_registry::model::targets_from_clauses(&axis, &clause_list) else {
                continue;
            };
            if targets.is_empty() {
                continue;
            }
            context.declare_targets(targets);
            declared_any = true;
        }
        if !declared_any {
            return;
        }
        let core_axis = VersionAxisId::core(Family::Tcl);
        if let Some(declared) = context.declared_targets(&core_axis) {
            for &grammar in tcl_dialect::NumberSyntax::ALL {
                // Exhaustive on purpose: a future grammar variant must
                // name its interval here before it compiles.
                let requirement = match grammar {
                    tcl_dialect::NumberSyntax::Tcl84 => "0-8.5",
                    tcl_dialect::NumberSyntax::Tcl85 => "8.5-9.0",
                    tcl_dialect::NumberSyntax::Tcl90 => "9.0-",
                    // JimTcl's numeral grammars are not intervals on the
                    // Tcl core axis — Jim is a reimplementation, not a
                    // release of it — so no declared Tcl range can select
                    // one. They reach this diagnostic through the Jim
                    // ladder instead.
                    tcl_dialect::NumberSyntax::Jim | tcl_dialect::NumberSyntax::Jim080 => {
                        continue;
                    }
                };
                let Ok(era) = tcl_dialect::model::VersionSet::from_requirements(
                    core_axis.clone(),
                    &[requirement],
                ) else {
                    continue;
                };
                if declared
                    .intersect(&era)
                    .is_ok_and(|overlap| !overlap.is_empty())
                {
                    self.range_numeral_grammars.push(grammar);
                }
            }
        }
        self.range_context = Some(context);
    }

    /// Set the user-declared extra command names (`tclLsp.extraCommands`),
    /// returning `self` for builder-style configuration. These names are
    /// treated as known commands by the unknown-command (W123) check.
    #[must_use]
    pub fn with_extra_commands(self, extra: HashSet<String>) -> Self {
        self.with_shared_extra_commands(Arc::new(extra))
    }

    /// [`Self::with_extra_commands`] taking an already-shared set, so a caller
    /// that caches the (potentially very large) widened recovery name set hands
    /// it over as a refcount bump rather than a deep copy.
    #[must_use]
    pub fn with_shared_extra_commands(mut self, extra: Arc<HashSet<String>>) -> Self {
        self.extra_commands = extra;
        self
    }

    /// Enable structure-only mode (see [`Self::structure_only`]): the walk
    /// builds the declaration/scope structure but skips all diagnostic
    /// emission.  Returns `self` for builder-style configuration.  Used by the
    /// `item_tree` query for cheap offset-stable item extraction.
    #[must_use]
    pub fn structure_only(mut self) -> Self {
        self.structure_only = true;
        self
    }

    /// The workspace classes whose command bare-word-constructs — see
    /// [`Self::workspace_bare_word_classes`] (issue #1303).
    #[must_use]
    pub fn with_workspace_bare_word_classes(
        mut self,
        classes: std::collections::HashSet<String>,
    ) -> Self {
        self.workspace_bare_word_classes = classes;
        self
    }

    /// Supply the fully-qualified names of classes defined elsewhere in the
    /// workspace, so instance inference can resolve a constructor whose class
    /// lives in another file.  Used by the cross-file method reference /
    /// definition path; the normal single-file analysis leaves this empty.
    #[must_use]
    pub fn with_workspace_classes(mut self, classes: std::collections::HashSet<String>) -> Self {
        self.workspace_classes = classes;
        self
    }

    /// Supply the workspace's **class factory** index — the user-defined
    /// `TclOO` metaclasses declared in other documents — so a
    /// `::tk::Megawidget create IconList …` call whose metaclass lives in
    /// another file is classified instead of abstained on (issue #1276).
    ///
    /// See [`Self::workspace_class_factories`]. The normal single-file
    /// analysis leaves this `None`, which keeps the abstention intact.
    #[must_use]
    pub fn with_workspace_class_factories(
        mut self,
        factories: Option<std::sync::Arc<super::types::ClassFactoryIndex>>,
    ) -> Self {
        self.workspace_class_factories = factories;
        self
    }

    /// Supply the workspace's **subclass-provided method** view — the
    /// instance methods dispatchable on some descendant of each class, with
    /// the descendants written in other documents — so the template-method
    /// abstention (issue #1367) holds across the workspace boundary.
    ///
    /// See [`Self::workspace_subclass_methods`]. The normal single-file
    /// analysis leaves this `None`, which keeps W308 firing on the per-file
    /// evidence alone.
    #[must_use]
    pub fn with_workspace_subclass_methods(
        mut self,
        methods: Option<std::sync::Arc<super::types::SubclassProvidedMethods>>,
    ) -> Self {
        self.workspace_subclass_methods = methods;
        self
    }

    /// Set the source-file path, returning `self` for builder-style
    /// configuration.  Path-keyed behaviour: `pkgIndex.tcl` suppresses
    /// dead-store / unused hints for the loader-supplied `$dir`, and a
    /// file with a registry whole-file scoped environment (`tclpkg.tcl`
    /// manifests — [`tcl_registry::scoped::file_scope_env`]) is analysed
    /// with that environment ambient.
    #[must_use]
    pub fn with_file_path(mut self, path: Option<String>) -> Self {
        self.file_path = path;
        self
    }

    /// Supply a pre-built [`crate::compilation_unit::CompilationUnit`] for the
    /// CFG/SSA diagnostic tail, so [`Self::emit_cfg_ssa_diagnostics`] consumes
    /// it (once) instead of rebuilding the whole-file unit.  The supplied unit
    /// must be equal to what the tail would build for this document's source —
    /// the incremental path builds it from memoised per-function lattices, so
    /// only the unchanged-body lattice recompute is skipped.  Reset after the
    /// next `analyse`.
    pub fn set_cu_override(
        &mut self,
        cu: std::sync::Arc<crate::compilation_unit::CompilationUnit>,
    ) {
        self.cu_override = Some(cu);
    }

    /// Analyse a Tcl source for the given dialect, returning a
    /// fully-populated [`AnalysisResult`].
    ///
    /// Drives end-to-end:
    ///
    /// 1. Pre-scans the leading comment block for
    ///    `# tcl-lsp: disable=CODE` directives via
    ///    [`super::utils::parse_file_suppression`].
    /// 2. Segments `source` with the registry's known-commands
    ///    set (uses re-segmentation recovery) so unclosed delimiters mid-file
    ///    don't drop later declarations.
    /// 3. Walks each segmented command through
    ///    [`Self::process_command`].
    ///
    /// As [`Self::analyse`], but the whole file is walked **inside**
    /// `source_namespace` (a constructed `::`-rooted key) — the static
    /// equivalent of C Tcl evaluating a `source`d file in the caller's
    /// current namespace (M9).  Relative definitions home under the seed
    /// (`proc helper` → `<seed>::helper`), absolute ones are unaffected, and
    /// bare call sites gain the seeded namespace as their first resolution
    /// candidate — exactly the `namespace eval <seed> { <file> }` semantics,
    /// via the ordinary scope machinery.
    pub fn analyse_with_source_namespace(
        &mut self,
        source: &str,
        dialect: &str,
        source_namespace: &str,
    ) -> AnalysisResult {
        if source_namespace.is_empty() || source_namespace == "::" {
            return self.analyse(source, dialect);
        }
        self.seed_namespace_key = Some(source_namespace.to_owned());
        self.analyse(source, dialect)
    }

    /// `source` is consumed by reference so the analyser can hold
    /// per-walk references back into it; `dialect` is one of
    /// `"tcl"`, `"f5-irules"`, `"irules"`, `"iapps"`, etc. (kept in
    /// the analyser's per-walk state via [`Self::current_event`]
    /// elsewhere; this entry just records it for future use).
    pub fn analyse(&mut self, source: &str, dialect: &str) -> AnalysisResult {
        use std::collections::HashSet;

        // Stash the source so handlers (recovery, diagnostic
        // emitters) can re-slice it.
        self.source = source.to_string();
        let tk_ambient = self.resolve_walk_environment(dialect);
        // Tell pack hooks which dialect they are running under, for the
        // length of this walk. A hook's `ctx.dialect` used to be derived from
        // the call's `TclVersion`, which can only spell a release — so an
        // iRules document reported `tcl9.0` and no hook could tell a dialect
        // from a version. The guard restores the previous value on the way
        // out, since one worker analyses documents of different dialects in
        // turn.
        let _dialect_scope = tcl_registry::pack_hooks::DialectScope::enter(Some(self.profile.name));
        self.result.dialect = dialect.to_string();
        self.result.library_versions = self.library_versions.clone();
        self.tk_accumulation_enabled = super::tk_checks::tk_checks_could_apply(source, tk_ambient);
        self.tk_ambient = tk_ambient;
        // §5.4 range targeting: resolve configured + directive-declared
        // version targets for this walk (a no-op for the undeclared
        // majority).
        self.resolve_declared_targets(source);
        // Clear the per-run iRules file-profile memo so a reused analyser
        // instance recomputes it for the new source / dialect.
        self.irules_file_profiles = None;
        self.irules_event_bodies = None;
        // File-suppression pre-scan: merge codes from any
        // top-of-file ``# tcl-lsp: disable=CODE`` directives into
        // ``self.disabled_diagnostics`` so later emitter passes
        // honour them. The constructor-provided
        // ``disabled_diagnostics`` set (LSP user-config) and the
        // file-directive set are unioned — both sources should
        // take effect.
        //
        // File-level suppression also lives in
        // ``result.suppressed_lines[-1]`` (a per-line map keyed by a
        // sentinel ``-1`` for file-wide); merging the codes into
        // ``disabled_diagnostics`` gives the directives effect at the
        // analyser-internal level.
        let file_codes = super::utils::parse_file_suppression(source);
        for code in &file_codes {
            self.disabled_diagnostics.insert(code.clone());
        }
        if !file_codes.is_empty() {
            // Record the ``result.suppressed_lines[-1]`` sentinel so
            // downstream consumers (the LSP suppression filter,
            // code-action UX) see the file-wide directive set in one
            // place.
            self.result
                .suppressed_lines
                .insert(-1, file_codes.iter().cloned().collect());
        }
        // Pre-scan for next-line ``# noqa`` suppressions.  Handles
        // orphaned noqa at the tail of a brace body and noqa
        // before a comment line that itself generates a
        // diagnostic.  Merges into ``suppressed_lines`` alongside
        // the command-attached ``apply_preceding_noqa`` pass that
        // runs per segmented command in the dispatch loop below.
        merge_noqa_line_suppressions(
            &mut self.result.suppressed_lines,
            super::utils::parse_noqa_line_suppressions_for_dialect(source, self.profile),
        );
        // Inline ``# tcl-lsp: stub …`` block scan.  After
        // capturing the parsed records, build the per-document
        // overlay so analyser / compiler queries see the
        // user-declared stubs as first-class commands (without
        // mutating the global registry).
        let (stub_cmds, stub_exprs) = super::utils::scan_source_for_stubs(source);
        let (sidecar_cmds, sidecar_exprs) =
            super::utils::scan_sidecar_stubs(self.file_path.as_deref(), dialect);
        let mut overlay_cmds = sidecar_cmds;
        // The document-local declaration is nearest in scope and wins over a
        // workspace sidecar with the same name.
        overlay_cmds.extend(stub_cmds.iter().cloned());
        self.declared_commands = Some(super::types::build_declared_surface(&overlay_cmds));
        self.result.stub_commands = overlay_cmds;
        let mut all_exprs = sidecar_exprs;
        all_exprs.extend(stub_exprs);
        self.result.stub_expr_defs = all_exprs;

        // Segment with re-segmentation recovery so an unclosed delimiter
        // mid-file doesn't drop later top-level declarations.
        // Build the dialect-aware registry once and stash on
        // ``self`` so per-command handlers (registry-driven body
        // iteration in ``process_command``) reuse it.
        self.registry = Some(self.profile_registry());
        self.head_identities = crate::realm::document_realm_bindings_with_config(
            source,
            self.lexer_config(),
            self.registry.as_deref().expect("registry just stashed"),
        );
        // Precompute the iRules file-profile stack (no-op off f5-irules) so
        // the per-command IRULE1001 hint can consult it without recomputing.
        self.compute_irules_file_profiles();
        // Precompute newline offsets once for ``O(log N)``
        // byte-offset → line-number lookup in
        // ``apply_preceding_noqa`` (which runs per command and
        // would otherwise be ``O(N)`` per call).
        self.line_offsets = Some(compute_line_offsets(source));
        self.cached_line_index = tcl_lexer::LineIndex::new(source);
        self.cached_line_index_source_len = source.len();
        // The recovery known-command universe (registry + this document's
        // own procs/classes/aliases) — see `recovery_known_commands`. Stored
        // on `self` so the per-command E100/E201/E202/E203 detectors below
        // (and `apply_ghost_recovery`) consult the same set the segmenter
        // scan-to-next recovery uses here.
        self.recovery_known_commands = super::utils::recovery_known_commands(
            source,
            self.registry.as_deref().expect("registry just stashed"),
            &self.extra_commands,
        );
        let known_commands: HashSet<&str> = self.recovery_known_commands.iter().collect();
        let mut commands = crate::segmenter::segment_commands_with_recovery_and_config(
            source,
            &known_commands,
            self.file_lexer_config(),
        );
        drop(known_commands);

        // Ghost-token recovery (see method doc).
        let ghost_recovery_applied = self.apply_ghost_recovery(source, &mut commands);

        // Walk each command through the dispatcher.  The dispatcher
        // wires ``recover_stray_close_bracket``,
        // ``recover_missing_open_brace`` (for switch with a forgotten
        // body brace), ``detect_stolen_close_brace`` (E103), and the
        // generic E200 partial-command emitter.
        // M9: a seeded analysis (`analyse_with_source_namespace`) walks the
        // whole file inside the source-site namespace, exactly as if wrapped
        // in `namespace eval <ns> { ... }` — relative definitions re-home,
        // absolute ones stay put, and call-site candidates gain the seeded
        // tier, all through the ordinary scope machinery.
        if let Some(seed) = self.seed_namespace_key.take() {
            let end = u32::try_from(source.len()).unwrap_or(u32::MAX);
            let mut base: Vec<usize> = Vec::new();
            for segment in crate::naming::key_segments(&seed) {
                let mut child =
                    super::types::Scope::new(super::types::ScopeKind::Namespace, &segment);
                child.body_span = Some(tcl_lexer::Span::new(0, end));
                let parent = super::scope::scope_at_mut(&mut self.result.global_scope, &base)
                    .expect("seed scope path is self-built");
                parent.children.push(child);
                base.push(parent.children.len() - 1);
            }
            self.seed_scope_path = base;
        }
        let file_env_pushed = self.seed_file_scope_env(source);
        self.walk_commands_top_level(&commands, ghost_recovery_applied);
        if file_env_pushed {
            self.body_scope_stack.pop();
        }

        // Run the diagnostic-emission orchestrator and the post-pass
        // filters:
        //
        // 1. ``emit_unresolved_command_diagnostics``.
        // 2. ``emit_variable_usage_diagnostics``.
        // 3. ``emit_cfg_ssa_diagnostics(source)``.
        // 4. ``apply_disabled_diagnostics`` — filter codes the
        //    caller asked to silence (also covers the
        //    file-suppression directives merged at the top of
        //    ``analyse``).
        // 5. ``dedupe_diagnostics`` — drop exact duplicates and
        //    the line-based suppression pairs.
        // Structure-only mode (item_tree extraction) skips the entire
        // diagnostic-emission tail — the unresolved/arity/variable/CFG-SSA
        // emitters are the dominant cost and produce no structural facts.
        // Record the classes a proc manufactures under a computed name whose
        // value a literal call-site argument proves (issue #1306).  A
        // *structural* fact, so it runs on the item-tree path too — that is
        // the path the workspace class-factory index is computed from, and a
        // metaclass missing there is invisible to every other document.
        self.record_literal_parameter_definitions();
        if !self.structure_only {
            self.run_diagnostic_emitters(source);
        }

        let result = std::mem::take(&mut self.result);
        self.clear_run_state();
        result
    }

    /// Walk a top-level command stream through the dispatcher (the body of
    /// `analyse`'s main loop, extracted so the per-item path can reuse it
    /// verbatim).  `scope_path` is the global scope (`&[]`).  When
    /// `defer_proc_bodies` is set, `handle_proc_command` / OO method walks
    /// record the body for later isolated analysis instead of recursing.
    pub(super) fn walk_commands_top_level(
        &mut self,
        commands: &[crate::segmenter::SegmentedCommand],
        ghost_recovery_applied: bool,
    ) {
        self.record_path_constant_candidates(commands);
        let total = commands.len();
        let mut cmd_idx: usize = 0;
        while cmd_idx < total {
            let cmd_ref = &commands[cmd_idx];
            if cmd_ref.argv.is_empty() {
                cmd_idx += 1;
                continue;
            }
            if cmd_ref.is_partial {
                // An unterminated `"` / `{` emits E202 / E203
                // (with a closing-delimiter fix) instead of the generic
                // E200; only fall through to E103 / E200 when no
                // delimiter-recovery diagnostic applies.
                // Stolen-close-brace detection (E103) only applies to a
                // *brace* partial; a bracket / quote partial goes
                // straight to E200.
                let brace_partial = matches!(
                    cmd_ref.partial_delimiter,
                    Some(crate::segmenter::UnclosedDelimiter::Brace)
                );
                let region_end = self.source.len();
                if !(self.emit_unterminated_delimiter_diagnostics(cmd_ref, region_end)
                    || brace_partial && self.detect_stolen_close_brace(cmd_ref))
                {
                    self.emit_partial_command_diagnostic(cmd_ref);
                }
                cmd_idx += 1;
                continue;
            }
            let mut cmd = cmd_ref.clone();
            // E100 (stray `]`) / E102 (stray `}`) token checks, run on
            // the *original* token stream before recovery mutates the
            // clone.  Distinct from the recovery repairs below, which
            // handle unclosed openers; a stray closer otherwise goes
            // unreported.  Runs at the top level; nested bodies are
            // covered by ``analyse_body``'s own per-body check.
            self.emit_syntax_recovery_diagnostics(cmd_ref, ghost_recovery_applied);
            self.recover_stray_close_bracket(&mut cmd);
            let consumed = self.recover_missing_open_brace(&mut cmd, commands, cmd_idx);
            let single = cmd.single_token_word.clone();
            // ``# noqa[: CODE,...]`` directives in
            // ``cmd.preceding_comment`` suppress diagnostics on
            // the *following* command's line range.
            if let Some(line_offsets) = self.line_offsets.as_deref() {
                super::utils::apply_preceding_noqa(
                    &cmd,
                    line_offsets,
                    &mut self.result.suppressed_lines,
                );
            }
            // Record the preceding comment: handlers that consume one
            // (proc, oo::class) ``std::mem::take`` it; everything else
            // clears it on the next command.
            self.last_comment = cmd.preceding_comment.clone().unwrap_or_default();
            let base_path = self.seed_scope_path.clone();
            self.process_command(
                &cmd.texts,
                &cmd.argv,
                &single,
                cmd.expand_word.as_deref().unwrap_or(&[]),
                &base_path,
            );
            self.emit_w216_brace_then_paren(&cmd);
            self.record_arg_var_reads(&cmd, &base_path);
            cmd_idx += 1 + consumed;
        }
    }

    /// Ghost-token recovery.  When the scan-to-next
    /// recovery left a partial command (the shape that otherwise emits
    /// E200), re-lex the document with zero-width ghost `]` insertions
    /// derived from the E201 heuristics.  When any apply, the
    /// swallowed-command case (`set x [foo bar` then `puts done`) splits
    /// into a clean `[foo bar]` + `puts done` stream, `commands` is
    /// replaced with it, its E201 diagnostics are emitted, and `true` is
    /// returned (the caller then skips its own E201 detector to avoid a
    /// double-report).  Clean / fallback input has no partial command,
    /// so this never runs a second parse on the common path.
    pub(super) fn apply_ghost_recovery(
        &mut self,
        source: &str,
        commands: &mut Vec<crate::segmenter::SegmentedCommand>,
    ) -> bool {
        if !commands.iter().any(|c| c.is_partial) {
            return false;
        }
        let (clean, e201) = crate::segmenter::segment_with_recovery(
            source,
            self.lexer_config(),
            &self.recovery_known_commands,
        );
        if e201.is_empty() {
            return false;
        }
        *commands = clean;
        self.result.diagnostics.extend(e201);
        true
    }

    /// Emit the source-recovery syntax diagnostics for one command's
    /// token stream: E100 / E102 stray closers and, unless
    /// ghost recovery already emitted them, E201 unterminated `[`
    /// command substitutions.  When `ghost_recovery_applied`,
    /// the stream is the ghost-recovered one and the E201s came from
    /// `segment_with_recovery`; a ghost-terminated command would look
    /// unterminated against the original bytes, so the detector is
    /// skipped to avoid a double-report.
    fn emit_syntax_recovery_diagnostics(
        &mut self,
        cmd: &crate::segmenter::SegmentedCommand,
        ghost_recovery_applied: bool,
    ) {
        let stray = super::syntax_checks::stray_closer_diagnostics(
            cmd,
            &self.source,
            self.registry.as_deref(),
            || self.user_command_tail_names(),
        );
        self.result.diagnostics.extend(stray);
        if !ghost_recovery_applied {
            let e201 = super::syntax_checks::unterminated_bracket_diagnostics(
                cmd,
                &self.source,
                &self.recovery_known_commands,
            );
            self.result.diagnostics.extend(e201);
        }
        // E202 / E203 fire for unterminated `"` / `{` whose token
        // wasn't split into a partial command (e.g. a quote run below
        // the segmenter's recovery line threshold).  Top-level scan,
        // so the region ends at the document end.
        self.emit_unterminated_delimiter_diagnostics(cmd, self.source.len());
    }

    /// Emit the E202 (unterminated `"`) / E203 (unterminated `{`)
    /// recovery diagnostics for one command, honouring the disable set.
    /// Returns `true` when at least one was emitted — the partial-command
    /// path uses this to suppress the generic E200.
    pub(super) fn emit_unterminated_delimiter_diagnostics(
        &mut self,
        cmd: &crate::segmenter::SegmentedCommand,
        region_end: usize,
    ) -> bool {
        let diags = super::syntax_checks::unterminated_delimiter_diagnostics(
            cmd,
            &self.source,
            region_end,
            self.registry.as_deref(),
            &self.recovery_known_commands,
        );
        let mut emitted = false;
        for d in diags {
            if self.disabled_diagnostics.contains(d.code.as_str()) {
                continue;
            }
            self.result.diagnostics.push(d);
            emitted = true;
        }
        emitted
    }

    /// Convert the dialect-aware lexer's `LexWarning`s into recovery
    /// diagnostics.  E204 (extra characters after a close-brace), E205
    /// (after a close-quote) and E206 (missing close-brace for a
    /// `${…}` variable name) map by message; the "missing closer"
    /// messages that overlap E201 / E202 / E203 are skipped (the
    /// recovery detectors own those with better positions + fixes);
    /// every *other* message maps to the catch-all E200.
    fn emit_lexer_warning_diagnostics(&mut self) {
        let lexer = tcl_lexer::Lexer::with_source_map(
            Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            ),
            self.lexer_config(),
        );
        let Ok((_tokens, warnings)) = lexer.tokenise_all_with_warnings() else {
            return;
        };
        for w in warnings {
            let code = match w.message.as_str() {
                // Owned by the E201/E202/E203 recovery heuristics.
                "missing close-bracket" | "missing \"" | "missing close-brace" => continue,
                "extra characters after close-brace" => DiagCode::E204,
                "extra characters after close-quote" => DiagCode::E205,
                "missing close-brace for variable name" => DiagCode::E206,
                // Any unexpected lexer warning → catch-all E200.
                _ => DiagCode::E200,
            };
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    code,
                    Span::new(w.offset, w.offset),
                    w.message,
                    super::types::Severity::Error,
                ));
        }
    }

    /// Analyse pre-segmented commands chunk-by-chunk and capture
    /// per-chunk snapshots.
    ///
    /// Used by the LSP for incremental document re-analysis: when the
    /// user types into the document, dirty chunks are re-segmented
    /// and fed back through this entry while clean chunks are
    /// restored from a prior snapshot.
    ///
    /// Returns the final [`AnalysisResult`] plus a list of
    /// [`super::AnalyserSnapshot`]s, one per chunk in the input
    /// order.  The caller stores the snapshots alongside the
    /// chunk segmentation so a later edit can rewind to the
    /// matching prefix.
    ///
    /// `chunk_commands` is grouped already — each inner `Vec`
    /// is one chunk's worth of commands.  `dialect` matches the
    /// argument to [`Self::analyse`].
    pub fn analyse_chunked(
        &mut self,
        source: &str,
        chunk_commands: Vec<Vec<crate::segmenter::SegmentedCommand>>,
        dialect: &str,
    ) -> (AnalysisResult, Vec<super::snapshot::AnalyserSnapshot>) {
        self.source = source.to_string();
        // Same-source memo, cleared with the source it was derived from.
        self.irules_event_bodies = None;
        let tk_ambient = self.resolve_walk_environment(dialect);
        self.result.dialect = dialect.to_string();
        self.result.library_versions = self.library_versions.clone();
        self.tk_accumulation_enabled = super::tk_checks::tk_checks_could_apply(source, tk_ambient);
        self.tk_ambient = tk_ambient;
        // §5.4 range targeting — same resolution as `analyse`.
        self.resolve_declared_targets(source);
        self.unresolved_commands_emitted = false;

        let file_codes = super::utils::parse_file_suppression(source);
        for code in &file_codes {
            self.disabled_diagnostics.insert(code.clone());
        }
        if !file_codes.is_empty() {
            // Record the same ``result.suppressed_lines[-1]`` sentinel
            // as ``analyse`` so downstream consumers see file-wide
            // ``# tcl-lsp: disable=`` directives via the same surface
            // regardless of which entry point dispatched the analyse.
            self.result
                .suppressed_lines
                .insert(-1, file_codes.iter().cloned().collect());
        }
        // Next-line ``# noqa`` pre-scan — see ``analyse`` for
        // rationale.
        merge_noqa_line_suppressions(
            &mut self.result.suppressed_lines,
            super::utils::parse_noqa_line_suppressions_for_dialect(source, self.profile),
        );

        // Build + stash the dialect-aware registry so
        // ``process_command`` 's body-iteration loop has access
        // to it on every chunked / incremental call (the entry
        // point used by the LSP's incremental update path).
        // Without this, body recursion silently no-ops.  Same
        // for the ``line_offsets`` index used by
        // ``apply_preceding_noqa``.
        self.registry = Some(self.profile_registry());
        self.head_identities = crate::realm::document_realm_bindings_with_config(
            source,
            self.lexer_config(),
            self.registry.as_deref().expect("registry just stashed"),
        );
        self.recovery_known_commands = super::utils::recovery_known_commands(
            source,
            self.registry.as_deref().expect("registry just stashed"),
            &self.extra_commands,
        );
        self.line_offsets = Some(compute_line_offsets(source));
        self.cached_line_index = tcl_lexer::LineIndex::new(source);
        self.cached_line_index_source_len = source.len();

        // Whole-file scoped environment (tclpkg manifests) — same seeding as
        // `analyse`, spanning every chunk's walk.
        let file_env_pushed = self.seed_file_scope_env(source);
        let mut snapshots: Vec<super::snapshot::AnalyserSnapshot> =
            Vec::with_capacity(chunk_commands.len());
        for cmds in chunk_commands {
            self.analyse_commands_inner(&cmds);
            snapshots.push(self.snapshot());
        }
        if file_env_pushed {
            self.body_scope_stack.pop();
        }

        // Same structural + diagnostic-emission tail as ``analyse``.
        self.record_literal_parameter_definitions();
        self.run_diagnostic_emitters(source);

        let result = std::mem::take(&mut self.result);
        self.clear_run_state();
        (result, snapshots)
    }

    /// Analyse pre-segmented commands without re-segmenting `source`.
    ///
    /// This is the single-chunk variant used by the LSP's incremental
    /// path after a prior `restore` — the analyser starts from
    /// a snapshot covering earlier clean chunks, then walks the
    /// dirty chunk's commands through the dispatcher.
    ///
    /// When `finalise` is `true` the diagnostic-emission tail
    /// (orchestrator + filters) runs.  When `false` only the
    /// command walk happens — the caller is building a partial
    /// snapshot and will run the tail later.
    pub fn analyse_commands(
        &mut self,
        source: &str,
        commands: &[crate::segmenter::SegmentedCommand],
        dialect: &str,
        finalise: bool,
    ) -> AnalysisResult {
        self.source = source.to_string();
        // Same-source memo, cleared with the source it was derived from.
        self.irules_event_bodies = None;
        let tk_ambient = self.resolve_walk_environment(dialect);
        self.result.dialect = dialect.to_string();
        self.result.library_versions = self.library_versions.clone();
        self.tk_accumulation_enabled = super::tk_checks::tk_checks_could_apply(source, tk_ambient);
        self.tk_ambient = tk_ambient;
        // §5.4 range targeting — same resolution as `analyse`.
        self.resolve_declared_targets(source);
        self.unresolved_commands_emitted = false;

        let file_codes = super::utils::parse_file_suppression(source);
        for code in &file_codes {
            self.disabled_diagnostics.insert(code.clone());
        }
        if !file_codes.is_empty() {
            // The same ``-1`` sentinel ``analyse`` uses — see the
            // matching block in ``analyse_chunked``.
            self.result
                .suppressed_lines
                .insert(-1, file_codes.iter().cloned().collect());
        }
        // Next-line ``# noqa`` pre-scan — see ``analyse`` for
        // rationale.
        merge_noqa_line_suppressions(
            &mut self.result.suppressed_lines,
            super::utils::parse_noqa_line_suppressions_for_dialect(source, self.profile),
        );

        // Same registry + line-index prelude as
        // ``analyse_chunked`` — see that doc-comment.  Without
        // these the registry-driven body loop in
        // ``process_command`` silently skips body recursion on
        // the incremental path.
        self.registry = Some(self.profile_registry());
        self.head_identities = crate::realm::document_realm_bindings_with_config(
            source,
            self.lexer_config(),
            self.registry.as_deref().expect("registry just stashed"),
        );
        self.recovery_known_commands = super::utils::recovery_known_commands(
            source,
            self.registry.as_deref().expect("registry just stashed"),
            &self.extra_commands,
        );
        self.line_offsets = Some(compute_line_offsets(source));
        self.cached_line_index = tcl_lexer::LineIndex::new(source);
        self.cached_line_index_source_len = source.len();

        // Stub-directive pre-scan + overlay, matching ``analyse`` so command
        // resolution (W123 / W307 / param-trait inference) sees the same stub
        // surface and ``analyse_commands`` stays byte-identical to ``analyse``.
        let (stub_cmds, stub_exprs) = super::utils::scan_source_for_stubs(source);
        let (sidecar_cmds, sidecar_exprs) =
            super::utils::scan_sidecar_stubs(self.file_path.as_deref(), dialect);
        let mut overlay_cmds = sidecar_cmds;
        overlay_cmds.extend(stub_cmds.iter().cloned());
        self.declared_commands = Some(super::types::build_declared_surface(&overlay_cmds));
        self.result.stub_commands = overlay_cmds;
        let mut all_exprs = sidecar_exprs;
        all_exprs.extend(stub_exprs);
        self.result.stub_expr_defs = all_exprs;

        let file_env_pushed = self.seed_file_scope_env(source);
        self.analyse_commands_inner(commands);
        if file_env_pushed {
            self.body_scope_stack.pop();
        }

        if finalise {
            self.record_literal_parameter_definitions();
            self.run_diagnostic_emitters(source);
        }

        let result = std::mem::take(&mut self.result);
        self.clear_run_state();
        result
    }

    /// Incrementally analyse `new_text` given the previous document text
    /// and its top-level segmentation (`prev_commands` from
    /// [`crate::segmenter::segment_commands`]). Reuses the unchanged
    /// command prefix via [`crate::segmenter::segment_commands_incremental`]
    /// — avoiding a re-lex of everything before the edit — then runs the
    /// full analysis over the spliced command stream via
    /// [`Self::analyse_commands`].
    ///
    /// **Correctness-first.** The result is identical to a full
    /// [`Self::analyse`] of `new_text` (pinned by a differential fuzz
    /// oracle). The incremental path uses the plain segmenter and the
    /// pre-segmented `analyse_commands` entry, which match the full
    /// `analyse` only when `new_text` needs no error recovery and carries
    /// no inline stub directives; those cases fall back to a full
    /// re-analysis. The fast path is the common well-formed edit.
    pub fn analyse_incremental(
        &mut self,
        prev_text: &str,
        prev_commands: &[crate::segmenter::SegmentedCommand],
        new_text: &str,
        dialect: &str,
    ) -> AnalysisResult {
        // Error recovery (ghost-token re-lex, stray-closer repair) and
        // inline `# tcl-lsp: stub` overlays are only applied on the full
        // `analyse` path; when either could be in play, re-analyse fully
        // so the incremental result can never diverge.
        if !tcl_lexer::script_is_complete(new_text) || new_text.contains("tcl-lsp: stub") {
            return self.analyse(new_text, dialect);
        }
        let cmds = crate::segmenter::segment_commands_incremental(
            prev_text,
            prev_commands,
            new_text,
            // The document's whole-file grammar, resolved once through the
            // ingress exactly as `analyse` resolves it for the full walk.
            tcl_lexer::LexerConfig::for_file_grammar(
                crate::environment_ingress::resolve_environment(dialect).grammar(),
            ),
        );
        // `analyse` segments with *error recovery*; the fast path uses plain
        // incremental segmentation.  `script_is_complete` only checks overall
        // delimiter balance, so a locally-unbalanced brace (a stray `}` matched
        // by a missing `{` elsewhere) passes it yet makes recovery segmentation
        // split off a partial command — emitting E200/E102/E202 that the plain
        // walk never sees.  Replicate the recovery segmentation `analyse` uses
        // and fall back to a full rebuild when it differs from what the fast
        // path would walk, or leaves any partial command.  (This also subsumes
        // the plain-segmentation-metadata check: the recovery segmenter is the
        // authority on the command stream + its attached comments.)
        let environment = crate::environment_ingress::resolve_environment(dialect);
        let keyed =
            crate::environment_ingress::DocumentEnvironment::keyed_versions(&self.library_versions);
        let generation = environment.context_registry(&keyed, self.pack_overlay);
        let known: std::collections::HashSet<&str> =
            generation.commands().command_names().collect();
        let recovery_cmds = crate::segmenter::segment_commands_with_recovery_and_config(
            new_text,
            &known,
            tcl_lexer::LexerConfig::from_grammar(environment.analyser_profile().grammar),
        );
        if recovery_cmds != cmds || recovery_cmds.iter().any(|c| c.is_partial) {
            return self.fresh_full_analyse(new_text, dialect);
        }
        let result = self.analyse_commands(new_text, &cmds, dialect, true);
        // Correctness fallback (the maintainer's mandate: incremental must
        // always converge to a full rebuild).  The pre-segmented
        // `analyse_commands` walk is byte-identical to a full `analyse` for
        // well-formed input (verified over the whole corpus), but it diverges
        // from `analyse`'s error-*recovery* handling when the document carries
        // syntax errors: `analyse` runs ghost-token re-lexing,
        // recovery-segmentation, and body-level partial detection that the
        // pre-segmented entry only partially mirrors.  Every observed
        // `incremental != fresh` divergence on a well-segmented document carries
        // at least one syntax-error (E-code) diagnostic, so when the fast path
        // reports any such error we re-analyse fully — guaranteeing the result
        // equals a from-scratch `analyse`.  The fast path stays the common
        // well-formed edit; only mid-edit broken states (transient) take the
        // slow path.
        if result.diagnostics.iter().any(|d| d.code.is_error()) {
            return self.fresh_full_analyse(new_text, dialect);
        }
        result
    }

    /// Run a full [`Self::analyse`] on a *fresh* analyser carrying this one's
    /// config.  The incremental fast path consumes per-walk state (the scope
    /// walk leaves `var_command_sites` / `ensemble_namespaces` / … populated and
    /// `clear_run_state` resets only the registry + line index), so a second
    /// `analyse` on `self` would run on dirty state; a fresh analyser is the
    /// safe way to take the full-rebuild fallback mid-call.
    pub(super) fn fresh_full_analyse(&self, new_text: &str, dialect: &str) -> AnalysisResult {
        let mut fresh = Analyser::with_disabled_diagnostics(self.disabled_diagnostics.clone())
            .with_non_ascii_mode(self.non_ascii_mode)
            .with_shared_extra_commands(Arc::clone(&self.extra_commands));
        fresh.analyse(new_text, dialect)
    }

    /// Record the batch's raw path-constant candidates (see
    /// [`AnalysisResult::path_constant_assignments`]).  Called from the head
    /// of both top-level walks — `analyse`'s [`Self::walk_commands_top_level`]
    /// and the chunked/batched [`Self::analyse_commands_inner`] — which are
    /// sibling implementations, so no path records a batch twice.  Accumulated
    /// per batch (the chunked path walks one chunk at a time, and document
    /// order across chunks is exactly append order), with multi-write
    /// poisoning applied at fold time, where the whole document's write
    /// counts are in view.
    fn record_path_constant_candidates(&mut self, commands: &[crate::segmenter::SegmentedCommand]) {
        self.result.path_constant_assignments.extend(
            crate::auto_path_eval::constant_path_assignments_from_commands(commands, self.profile),
        );
    }

    /// Inner dispatch loop shared by [`Self::analyse_chunked`]
    /// and [`Self::analyse_commands`].  Walks pre-segmented
    /// commands at the current scope path.  Covers the dispatch
    /// portion that's load-bearing for incremental analysis.
    fn analyse_commands_inner(&mut self, commands: &[crate::segmenter::SegmentedCommand]) {
        self.record_path_constant_candidates(commands);
        let scope_path = self.current_scope_path.clone();
        let total = commands.len();
        let mut cmd_idx: usize = 0;
        while cmd_idx < total {
            let cmd_ref = &commands[cmd_idx];
            if cmd_ref.argv.is_empty() {
                cmd_idx += 1;
                continue;
            }
            if cmd_ref.is_partial {
                // Partial commands surface E103 / E200 in the chunked
                // path too so the LSP shows parse errors during
                // incremental analysis.  Stolen-close-brace (E103) is
                // brace-only.
                let brace_partial = matches!(
                    cmd_ref.partial_delimiter,
                    Some(crate::segmenter::UnclosedDelimiter::Brace)
                );
                if !(brace_partial && self.detect_stolen_close_brace(cmd_ref)) {
                    self.emit_partial_command_diagnostic(cmd_ref);
                }
                cmd_idx += 1;
                continue;
            }
            // Emit the same source-recovery syntax diagnostics as the
            // top-level loop in ``analyse``: E100 / E102 stray closers
            // *and* E201 (unterminated `[`) / E202 / E203 (unterminated
            // `"` / `{`).  ``analyse_commands`` never runs ghost
            // recovery, so pass ``false``.  Run on the original token
            // stream before ``recover_stray_close_bracket`` repairs the
            // clone.  (Nested bodies dispatched below are covered by
            // ``analyse_body``'s own per-body check.)
            self.emit_syntax_recovery_diagnostics(cmd_ref, false);
            // Repair stray ``]`` and missing ``{`` in a clone of the
            // segmented command before dispatch — chunked analysis keeps
            // the original snapshot copies untouched so re-runs are
            // deterministic.
            let mut cmd = cmd_ref.clone();
            self.recover_stray_close_bracket(&mut cmd);
            let consumed = self.recover_missing_open_brace(&mut cmd, commands, cmd_idx);
            if let Some(line_offsets) = self.line_offsets.as_deref() {
                super::utils::apply_preceding_noqa(
                    &cmd,
                    line_offsets,
                    &mut self.result.suppressed_lines,
                );
            }
            self.last_comment = cmd.preceding_comment.clone().unwrap_or_default();
            self.process_command(
                &cmd.texts,
                &cmd.argv,
                &cmd.single_token_word,
                cmd.expand_word.as_deref().unwrap_or(&[]),
                &scope_path,
            );
            // Like the top-level loop: W216 (brace-then-paren
            // name/value confusion) fires on top-level commands here too.
            self.emit_w216_brace_then_paren(&cmd);
            self.record_arg_var_reads(&cmd, &scope_path);
            cmd_idx += 1 + consumed;
        }
    }

    /// Resolve (and cache) the set of built-in command names that
    /// **exist** under the active dialect — the registry tier of the one
    /// `exists` oracle (centralisation R-c, ledger C5): every store name
    /// the resolved context actually provides
    /// ([`tcl_registry::model::ResolvedContext::resolve_spec`]), plus the
    /// measured iRules §4b interpreter-present extension
    /// ([`Self::extend_with_irules_interpreter_present_names`]).
    ///
    /// This is the same set the W123 unresolved-command pass resolves
    /// registry names against, so settlement
    /// (`finalise_invocation_resolutions`), constant-dispatch, W113, and
    /// W123 can no longer disagree about which registry commands exist —
    /// the pre-P1a split where settlement read the *unfiltered* store
    /// name set (and so believed in commands W123 did not) is retired.
    ///
    /// The name set is held on ``self.builtin_names`` for subsequent
    /// proc / class registrations to consult without rebuilding.
    /// The dialect resolves through the profile catalog; unknown dialect
    /// names sink to the permissive fallback profile's registry.
    pub(crate) fn builtin_command_names(&mut self) -> &std::collections::HashSet<String> {
        if self.builtin_dialect != Some(self.profile.name) || self.builtin_names.is_none() {
            let generation = self.analysis_context();
            let registry = generation.commands();
            let mut names: std::collections::HashSet<String> = registry
                .command_names()
                .filter(|name| generation.context().resolve_spec(registry, name).is_some())
                .map(str::to_string)
                .collect();
            self.extend_with_irules_interpreter_present_names(&mut names);
            self.builtin_names = Some(names);
            self.builtin_dialect = Some(self.profile.name);
        }
        // Safe: ``builtin_names`` was just set if it was missing.
        self.builtin_names
            .as_ref()
            .expect("builtin_names populated above")
    }

    /// measurements §4b: under `f5-irules` the 15 **compiler-refused**
    /// builtins are present in TMM's interpreter — reachable via `eval` at
    /// runtime — so the oracle must not claim they do not exist (no
    /// "Unknown command" W123). Their literal-source load refusal is
    /// IRULE2004's job (a policy warning, emitted by the disabled-command
    /// pass). The 16 interpreter-absent commands stay unknown here: their
    /// absence is a language fact. No-op outside the iRules profile.
    pub(in crate::analyser) fn extend_with_irules_interpreter_present_names(
        &self,
        names: &mut std::collections::HashSet<String>,
    ) {
        if self.profile.is_irules() {
            names.extend(
                tcl_registry::irules_policy::IRULES_COMPILER_REFUSED
                    .iter()
                    .map(|name| (*name).to_string()),
            );
        }
    }

    /// Run the post-walk diagnostic-emission tail: the W123 / arity / missing-
    /// package / variable-usage / CFG-SSA / lexer-warning emitters, then the
    /// disabled-code filter and the dedupe + canonical-order pass.  Shared by
    /// `analyse` / `analyse_chunked` / `analyse_commands` so the emitter set and
    /// ordering stay identical across every entry point.
    /// Drop the conditional entries from
    /// [`AnalysisResult::destroyed_commands`], keeping only destructions
    /// written at **load level**.
    ///
    /// A `rename OLD {}` inside a proc/class body runs at *call* time, when
    /// and if that definition is invoked — the same conditionality
    /// [`AnalysisResult::offset_is_inside_any_definition_body`] already
    /// governs for a nested `proc` shadow — and the consumer of this table
    /// revokes an import alias on it (issue #1103), which would drop a
    /// genuinely-live command. Removal events abstain toward keeping the
    /// alias, so the conditional ones are simply not published.
    ///
    /// It also keeps the incremental per-item path byte-identical to the
    /// whole-file one: an isolated body's destruction reaches the shell only
    /// on one of the two paths, and this filter removes it from both.
    fn publish_load_level_destructions(&mut self) {
        let all = std::mem::take(&mut self.result.destroyed_commands);
        self.result.destroyed_commands = all
            .into_iter()
            .filter(|&(_, off)| !self.result.offset_is_inside_any_definition_body(off))
            .collect();
    }

    pub(super) fn run_diagnostic_emitters(&mut self, source: &str) {
        // Whole-source security checks belong in the analyser result so direct
        // consumers receive the same verdict as the LSP and CLI adapters. The
        // pure producer is also reused by non-Tcl F5 document adapters.
        self.result.diagnostics.extend(
            super::source_integrity::bidi_control_diagnostics_with_suppressions(
                source,
                &self.disabled_diagnostics,
                &self.result.suppressed_lines,
            ),
        );
        // Replay the `<ensemble> <subcommand>` call sites the shell pass met
        // before the deferred body that declares the ensemble was walked
        // (issue #923 idx 85) — before `finalise_invocation_resolutions`, so
        // the replayed invocations go through exactly the same settlement
        // the walk-time ones do. A no-op on every other entry point.
        self.flush_pending_ensemble_subcommand_invocations();
        // Settle every invocation's `resolved_qualified_name` with Tcl's
        // existence-checked two-step rule now that the walk has recorded
        // every definition in the file (a local candidate defined later in
        // the file still wins; an absent one falls back to global).
        self.finalise_invocation_resolutions();
        self.publish_load_level_destructions();
        // Attach every namespace-qualified occurrence to the cell it names,
        // now that the whole file's `namespace eval` bodies have been walked
        // — a qualified read can precede its declaring `namespace eval`
        // textually and still resolve at run time.
        self.attach_qualified_var_references();
        let diag_registry = self.profile_registry();
        self.emit_unresolved_command_diagnostics(&diag_registry);
        self.flush_disabled_command_diagnostics();
        self.flush_w143_diagnostics();
        self.flush_w304_diagnostics();
        self.flush_var_literal_checks();
        // Decide every version-gated option relationship *before* the arity
        // flush: a conflict the resolved floor has is promoted onto
        // `pending_arity`, so it goes through the same shadowing suppression
        // an ungated one does.
        self.flush_gated_option_conflicts();
        // Same reason, one axis over: a call whose command changed shape
        // across releases could not be judged during the walk, and whatever
        // verdict the floor produces still has to face the shadowing check.
        self.flush_gated_arity_calls();
        self.flush_gated_bare_ensembles();
        self.flush_arity_diagnostics();
        self.flush_ctor_arity_diagnostics();
        self.flush_next_arity_diagnostics();
        self.emit_missing_package_require_diagnostics(&diag_registry);
        // Q8's assistance half. Disjoint from W120 by construction — that
        // fires when the requirement is absent, this when it is merely late.
        self.emit_package_require_ordering_hints(&diag_registry);
        self.emit_variable_usage_diagnostics();
        self.emit_cfg_ssa_diagnostics(source);
        self.flush_objdefine_abort_diagnostics();
        self.emit_lexer_warning_diagnostics();
        self.emit_w116_w117_stub_shadows();
        self.flush_widget_dispatch_diagnostics(&diag_registry);
        self.flush_tk_geometry_diagnostics();
        self.flush_version_gate_diagnostics();
        self.flush_dsl_gate_diagnostics();
        self.apply_disabled_diagnostics();
        self.dedupe_diagnostics();
        self.canonicalize_result_order();
    }

    /// Canonicalise the order of the walk-populated, order-sensitive result
    /// collections so the output is independent of *how* the walk was driven
    /// (whole-file vs per-item).  Generalises the diagnostic sort in
    /// `dedupe_diagnostics`: `command_invocations` and every `VarDef.references`
    /// list are sorted by source position.  These are set-like for their
    /// consumers (W123 already emitted; LSP references/rename filter by
    /// name/position and the client orders by position), so a canonical
    /// source order is behaviour-preserving and lets per-item analysis merge
    /// facts without reconstructing DFS order.
    fn canonicalize_result_order(&mut self) {
        self.result.command_invocations.sort_by(|a, b| {
            a.range
                .start()
                .cmp(&b.range.start())
                .then(a.range.end().cmp(&b.range.end()))
                .then_with(|| a.name.cmp(&b.name))
        });
        sort_scope_refs(&mut self.result.global_scope);
        for v in self.result.all_variables.values_mut() {
            v.references.sort_by_key(|s| (s.start(), s.end()));
        }
        // The remaining walk-populated record vecs are likewise set-like
        // (consumed by version inference / workspace index / regex checks),
        // so sort each by source position for walk-strategy independence.
        self.result
            .package_requires
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        self.result
            .package_provides
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        self.result
            .source_targets
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        self.result
            .namespace_imports
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        self.result
            .auto_path_entries
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        self.result
            .qualified_var_refs
            .sort_by_key(|r| (r.span.start(), r.span.end()));
        self.result
            .namespace_refs
            .sort_by_key(|r| (r.span.start(), r.span.end()));
        self.result
            .regex_patterns
            .sort_by_key(|r| (r.range.start(), r.range.end()));
        // `proc_declaration_sites` / `class_body_spans` are consumed purely
        // positionally (cursor-containment `find`s; `enclosing_class_at`'s
        // narrowest-span tie-break), so source order is canonical for them
        // too — the whole-file DFS interleaves nested declaration sites where
        // the per-item shell+graft appends them per body.
        self.result.proc_declaration_sites.sort_by(|a, b| {
            a.1.start()
                .cmp(&b.1.start())
                .then(a.1.end().cmp(&b.1.end()))
                .then_with(|| a.0.cmp(&b.0))
        });
        self.result.class_body_spans.sort_by(|a, b| {
            a.1.start()
                .cmp(&b.1.start())
                .then(a.1.end().cmp(&b.1.end()))
                .then_with(|| a.0.cmp(&b.0))
        });
        // `scoped_command_regions` is resolved purely by position
        // (containing-region lookups), so source order is canonical here too.
        self.result.scoped_command_regions.sort_by(|a, b| {
            a.span
                .start()
                .cmp(&b.span.start())
                .then(a.span.end().cmp(&b.span.end()))
                .then_with(|| a.env.name.cmp(b.env.name))
        });
        // `unresolved_command_sites` is a set of call sites consumed
        // order-independently (the cross-file arity resolver collects tail names +
        // per-site ranges); sort by `(span, name)` so the whole-file DFS and the
        // per-item shell+graft walks record it in the same order.
        self.result.unresolved_command_sites.sort_by(|a, b| {
            a.0.start()
                .cmp(&b.0.start())
                .then(a.0.end().cmp(&b.0.end()))
                .then_with(|| a.1.cmp(&b.1))
        });
    }

    /// Reset transient run state so the next ``analyse`` call
    /// starts from a clean slate.  Called at the end of every
    /// public entry point (``analyse`` / ``analyse_chunked`` /
    /// ``analyse_commands``).
    /// The whole-module command-mutation trust oracle for constant
    /// command-substitution folding (issue #1132), built lazily on first
    /// demand and cached for the rest of this run — see the
    /// [`Self::command_trust`] field doc for why the flow-sensitive
    /// `renamed_commands` map cannot serve here. Returns `None` when no
    /// registry is active (no fold can run then anyway).
    ///
    /// Cost note: building the oracle lowers the whole document once
    /// ([`crate::lowering::lower_to_ir_with_dialect`] +
    /// [`crate::command_binding::scan_module_command_mutations`]). Callers
    /// gate the first call behind [`crate::const_subst::head_may_fold`], so
    /// a document with no foldable `set VAR [cmd …]` never pays it.
    pub(super) fn whole_file_command_trust(
        &mut self,
    ) -> Option<std::sync::Arc<crate::command_binding::ModuleCommandMutations>> {
        if let Some(trust) = &self.command_trust {
            return Some(std::sync::Arc::clone(trust));
        }
        let registry = self.registry.as_deref()?;
        let module = crate::lowering::lower_to_ir_with_dialect(
            &self.source,
            registry,
            self.lexer_config(),
            // `None`, not `Some(plain_tcl)`, when the analysis named no
            // dialect: an unstated dialect is not the same input as an
            // explicit plain-Tcl one, and `Lowerer::dialect` feeds numeral
            // source selection. `self.profile` is always populated (it
            // defaults to the plain fallback), so gate on the recorded
            // spelling the way the pre-refactor `&str` boundary did.
            (!self.result.dialect.is_empty()).then_some(self.profile),
        );
        let trust = std::sync::Arc::new(crate::command_binding::scan_module_command_mutations(
            &module, registry,
        ));
        self.command_trust = Some(std::sync::Arc::clone(&trust));
        Some(trust)
    }

    /// Mint (and record) one offset-keyed synthetic identity —
    /// `@dynns@<off>` / `@dynclass@<off>` / `@autoname@<off>`.  All minting
    /// goes through here so [`Self::minted_synthetic_names`] can never miss
    /// a name the per-item graft later needs to rebase.
    pub(super) fn mint_synthetic_offset_name(&mut self, marker: &str, offset: u32) -> String {
        debug_assert!(
            crate::naming::SYNTHETIC_OFFSET_MARKERS.contains(&marker),
            "unknown synthetic marker {marker:?}"
        );
        let name = format!("{marker}{offset}");
        self.minted_synthetic_names.insert(name.clone());
        name
    }

    pub(super) fn clear_run_state(&mut self) {
        self.registry = None;
        self.context = None;
        self.environment = None;
        self.command_trust = None;
        self.objdefine_bindings.clear();
        self.objdefine_abort_candidates.clear();
        self.objdefine_unresolved_receiver = false;
        self.seed_namespace_key = None;
        self.seed_scope_path.clear();
        self.recovery_known_commands.clear();
        self.minted_synthetic_names.clear();
        self.pending_instances = None;
        self.deferred_instance_replays.clear();
        // Normally drained by the replay, but a walk that never runs the
        // post-walk tail (`analyse_commands` with `finalise: false`) leaves
        // entries behind, and a reused analyser must not carry one document's
        // deferred calls into the next.
        self.deferred_class_creations.clear();
        self.pending_bareword_dispatch_sites = None;
        self.line_offsets = None;
        self.cached_line_index = tcl_lexer::LineIndex::new("");
        self.cached_line_index_source_len = 0;
        self.e207_emitted = false;
        self.defer_proc_bodies = false;
        self.deferred_bodies.clear();
        self.cu_override = None;
    }

    /// Seed the whole-file scoped command environment for this document, if
    /// its file name declares one
    /// ([`tcl_registry::scoped::file_scope_env`]) — the file-level analogue
    /// of the `body_scope` push in `dispatch_body_arguments`.  Records the
    /// whole-document region (so the post-walk W123/W120 passes and the LSP
    /// providers resolve scoped heads by position) and pushes the
    /// environment so the in-walk arity / subcommand checks resolve them
    /// too.  Returns whether an environment was pushed, so the caller can
    /// balance the stack after its walk.
    pub(super) fn seed_file_scope_env(&mut self, source: &str) -> bool {
        let Some(env) = self
            .file_path
            .as_deref()
            .and_then(tcl_registry::scoped::file_scope_env)
        else {
            return false;
        };
        self.result
            .scoped_command_regions
            .push(super::types::ScopedBodyRegion {
                span: Span::new(0, u32::try_from(source.len()).unwrap_or(u32::MAX)),
                env,
            });
        self.body_scope_stack.push(env);
        true
    }
}

/// Precompute newline byte offsets for ``source``.  The returned
/// vector is sorted ascending — the byte offset of each ``\n``
/// in source order.  Callers (notably
/// ``apply_preceding_noqa``) use ``slice::partition_point`` /
/// ``binary_search`` on this vector to convert a byte offset to
/// a 0-based line number in ``O(log N)`` instead of a per-call
/// linear scan.
/// Recursively sort every `VarDef.references` list in a scope subtree by span,
/// for [`Analyser::canonicalize_result_order`].
fn sort_scope_refs(scope: &mut super::types::Scope) {
    for v in scope.variables.values_mut() {
        v.references.sort_by_key(|s| (s.start(), s.end()));
    }
    for child in &mut scope.children {
        sort_scope_refs(child);
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
mod tests {
    use super::*;
    use crate::analyser::types::ScopeKind;

    /// Issue #1604 — the composite-head split asks the shared owner where the
    /// `${…}` name ends, so the dispatched variable and the word suffix move
    /// with the release rather than always splitting at the first `}`.
    ///
    /// The lexer merges `${ns}::setdef` into one `Var` token whose text is
    /// `ns}::setdef`; a *pure* `${x}` arrives as `x`, its closer already
    /// outside the span. Both shapes fall out of `braced_var_name_end`:
    /// `Closed` is the composite case, `Unterminated` the pure one.
    #[test]
    fn split_braced_head_follows_the_release_close_rule() {
        let mut a = Analyser::new();

        // 9.x nests, so `${a{b}c}::setdef` dispatches on `a{b}c`.
        a.profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        assert_eq!(a.split_braced_head("ns}::setdef"), ("ns", "::setdef"));
        assert_eq!(a.split_braced_head("a{b}c}::setdef"), ("a{b}c", "::setdef"));
        // A pure `${…}` token has no closer inside its text: all name.
        assert_eq!(a.split_braced_head("a{b}c"), ("a{b}c", ""));
        assert_eq!(a.split_braced_head("x"), ("x", ""));
        // A simple `$obj` / `$ns::v` head is unchanged.
        assert_eq!(a.split_braced_head("ns::v"), ("ns::v", ""));

        // 8.x ends the name at the first literal `}`.
        a.profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        assert_eq!(a.split_braced_head("ns}::setdef"), ("ns", "::setdef"));
        assert_eq!(a.split_braced_head("a{b}c}::setdef"), ("a{b", "c}::setdef"));
    }

    #[test]
    fn new_analyser_starts_at_global_scope_with_empty_state() {
        let a = Analyser::new();
        assert_eq!(a.result.global_scope.kind, ScopeKind::Global);
        assert!(a.current_scope_path.is_empty());
        assert!(a.source.is_empty());
        assert!(a.disabled_diagnostics.is_empty());
        assert_eq!(a.conditional_depth, 0);
        assert_eq!(a.body_depth, 0);
        assert!(a.last_comment.is_empty());
        assert!(a.file_path.is_none());
        assert!(a.command_aliases.is_empty());
        assert!(a.var_command_sites.is_empty());
        assert!(a.cmd_command_sites.is_empty());
        assert!(a.const_strings.is_empty());
        assert!(a.regex_vars.is_empty());
        assert!(a.builtin_names.is_none());
        assert!(a.builtin_dialect.is_none());
        assert!(a.current_event.is_none());
        assert!(a.ensemble_namespaces.is_empty());
        assert!(a.objdefined_vars.is_empty());
        assert!(!a.unresolved_commands_emitted);
    }

    #[test]
    fn with_disabled_diagnostics_threads_through() {
        let disabled: HashSet<String> = ["W210", "W211"].iter().map(|s| (*s).to_string()).collect();
        let a = Analyser::with_disabled_diagnostics(disabled.clone());
        assert_eq!(a.disabled_diagnostics, disabled);
    }

    #[test]
    fn analyse_records_top_level_proc() {
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set x 1 }", "tcl");
        assert!(r.all_procs.contains_key("::foo"));
    }

    #[test]
    fn apply_body_proc_registers_in_global_namespace_not_caller() {
        // `apply`'s body runs in the *global* namespace (no lambda namespace
        // given), NOT the caller's — so a nested `proc` registers as `::helper`,
        // never `::foo::helper` (Tcl `apply` manual; matches the IR lowering).
        let mut a = Analyser::new();
        let r = a.analyse(
            "namespace eval foo { apply {{} { proc helper {} { return 1 } }} }",
            "tcl",
        );
        assert!(
            r.all_procs.contains_key("::helper"),
            "apply body proc must register globally: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
        assert!(
            !r.all_procs.contains_key("::foo::helper"),
            "apply body proc must NOT inherit the caller namespace: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    /// TP — the tcllib pki idiom: `proc [namespace current]::_x {...}` inside
    /// `::pki`.  The body's own definitions must home to `::pki`, not to a
    /// phantom namespace made out of the substitution's source text.
    ///
    /// Oracle (tclsh 8.6.16 and 9.0.4, probe `s5_probe.tcl`):
    ///
    /// ```text
    /// inner exists: ::pki::_inner
    /// helper homed at: ::pki::helper / global:
    /// ```
    #[test]
    fn a_dynamically_named_procs_body_homes_to_the_lexical_namespace() {
        let src = "namespace eval ::pki {\n\
                       proc _outer {} {\n\
                           proc [namespace current]::_inner {} {\n\
                               proc helper {} { return HELPED }\n\
                           }\n\
                       }\n\
                   }\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            r.all_procs.contains_key("::pki::helper"),
            "the body of a `[namespace current]`-named proc runs in ::pki: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
        assert!(
            !r.all_procs
                .keys()
                .any(|k| k.contains("[namespace current]") && k.ends_with("helper")),
            "no phantom substitution-text namespace may appear: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    /// TN — a statically-named proc is untouched: the scope name is the
    /// resolved name verbatim, so an ordinary qualified-name proc still homes
    /// its nested definitions to its own defining namespace.
    #[test]
    fn a_statically_named_procs_body_still_homes_to_its_defining_namespace() {
        let src = "proc ::pki::_outer {} { proc helper {} {} }\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            r.all_procs.contains_key("::pki::helper"),
            "{:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    /// A `$`-spelled dynamic tail (`proc _$n {...}`) keeps the lexical
    /// namespace too — the fallback is the trailing segment, whose holder is
    /// the enclosing namespace.
    #[test]
    fn a_dollar_named_procs_body_homes_to_the_lexical_namespace() {
        let src = "namespace eval ::pki { proc _$suffix {} { proc helper {} {} } }\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            r.all_procs.contains_key("::pki::helper"),
            "{:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn apply_body_proc_honours_explicit_lambda_namespace() {
        // A lambda pinning a namespace (element 2) runs its body there.
        let mut a = Analyser::new();
        let r = a.analyse("apply {{} { proc helper {} { return 1 } } ::bar}", "tcl");
        assert!(
            r.all_procs.contains_key("::bar::helper"),
            "explicit lambda namespace must be honoured: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    /// TP — a *relative* (non-`::`-prefixed) lambda namespace element is
    /// interpreted against the GLOBAL namespace, never the caller's.
    ///
    /// `doc/apply.n`: "If given, namespace is interpreted relative to the
    /// global namespace even if its name does not start with `::`";
    /// `tclProc.c`'s `TclNRApplyObjCmd` `::`-prefixes the word before the
    /// lookup. tclsh 9.0.4 oracle (probe `a2b_apply.tcl`), with `::sub`,
    /// `::pin::sub`, and `::lex::sub` all defined:
    ///
    /// ```text
    /// lex-body current=::lex -> ns=::sub who=GLOBAL-SUB
    /// proc current=::pin     -> ns=::sub who=GLOBAL-SUB
    /// c2 current=::pin       -> ns=::sub who=GLOBAL-SUB
    /// ```
    #[test]
    fn apply_relative_lambda_namespace_homes_globally_not_to_the_caller() {
        let src = "namespace eval ::sub {}\n\
                   namespace eval ::caller::sub {}\n\
                   proc ::caller::p {} {\n\
                       apply {{} { proc helper {} { return 1 } } sub}\n\
                   }\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            r.all_procs.contains_key("::sub::helper"),
            "relative lambda ns must resolve against ::, not the caller: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
        assert!(
            !r.all_procs.contains_key("::caller::sub::helper"),
            "relative lambda ns must NOT pin against the caller namespace: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
        let (_, ns) = r
            .namespace_overrides
            .first()
            .expect("the lambda body records a namespace override");
        assert_eq!(ns, "::sub");
    }

    /// TN — an *absolute* namespace element is unaffected by the
    /// global-relative rule: `::caller::sub` stays `::caller::sub`.
    #[test]
    fn apply_absolute_lambda_namespace_is_unchanged() {
        let src = "namespace eval ::sub {}\n\
                   namespace eval ::caller::sub {}\n\
                   proc ::caller::p {} {\n\
                       apply {{} { proc helper {} { return 1 } } ::caller::sub}\n\
                   }\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            r.all_procs.contains_key("::caller::sub::helper"),
            "absolute lambda ns must be honoured verbatim: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    /// Control — no namespace element at all still means the global
    /// namespace, from inside a qualified-name proc as much as anywhere.
    #[test]
    fn apply_without_a_namespace_element_stays_global_inside_a_qualified_proc() {
        let src = "proc ::caller::p {} {\n\
                       apply {{} { proc helper {} { return 1 } }}\n\
                   }\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            r.all_procs.contains_key("::helper"),
            "an absent lambda ns means global: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn apply_with_namespace_element_records_a_namespace_override() {
        // TP — issue #923 idx 116 Part 1 (the core mechanism): a literal
        // `apply {{params} body ns}` must record a namespace_overrides
        // entry spanning the body, so `tcl-lsp-core`'s command-resolution
        // lookups can pin bareword calls inside the body to `ns` — the
        // `Scope` subtree `reconstruct_proc_scope` builds for `ns` is
        // rooted under body_span-less wrapper nodes the ordinary lexical
        // walk can never reach.
        let mut a = Analyser::new();
        let src = "apply {{} { cleanup done } ::real}";
        let r = a.analyse(src, "tcl");
        assert_eq!(
            r.namespace_overrides.len(),
            1,
            "{:?}",
            r.namespace_overrides
        );
        let (span, ns) = &r.namespace_overrides[0];
        assert_eq!(ns, "::real");
        // The recorded span is the body token's raw span (opening brace
        // included, per this codebase's lexer-span convention — the
        // closing brace is excluded, one byte short of the token's end).
        assert_eq!(
            &src[span.start() as usize..span.end() as usize],
            "{ cleanup done "
        );
    }

    #[test]
    fn apply_without_namespace_element_does_not_record_an_override() {
        // TN — a 2-element lambda (no namespace argument) defaults to
        // global, which `command_resolution_namespace_at` already reports
        // with no override present; pushing a `(span, "::")` entry would
        // be a no-op relative to today's behaviour, but the plan
        // deliberately still records it (uniform code path) — confirm the
        // recorded namespace is exactly `"::"`, not skipped or wrong.
        let mut a = Analyser::new();
        let r = a.analyse("apply {{} { cleanup done }}", "tcl");
        assert_eq!(
            r.namespace_overrides.len(),
            1,
            "{:?}",
            r.namespace_overrides
        );
        assert_eq!(r.namespace_overrides[0].1, "::");
    }

    #[test]
    fn unknown_lexer_warning_maps_to_e200() {
        // An unterminated `$arr(idx` array index makes the lexer emit a
        // "missing )" warning, which has no dedicated recovery code — it
        // must surface as the catch-all E200 (not be silently dropped).
        let mut a = Analyser::new();
        let r = a.analyse("set x $arr(idx\n", "tcl8.6");
        let e200 = r.diagnostics.iter().find(|d| d.code == DiagCode::E200);
        assert!(
            e200.is_some_and(|d| d.message == "missing )"),
            "{:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_records_multiple_top_level_commands() {
        let mut a = Analyser::new();
        let r = a.analyse("set x 1\nproc foo {} {}\nglobal a b", "tcl");
        assert!(r.all_procs.contains_key("::foo"));
        assert!(r.global_scope.variables.contains_key("x"));
        assert!(r.global_scope.variables.contains_key("a"));
        assert!(r.global_scope.variables.contains_key("b"));
    }

    #[test]
    fn analyse_records_instance_class_set_new() {
        // `set d [Dog new]` maps `d` -> `::Dog`.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog { method bark {} {} }\nset d [Dog new]\n",
            "tcl",
        );
        assert_eq!(
            r.instance_classes.get("d").map(String::as_str),
            Some("::Dog")
        );
    }

    #[test]
    fn analyse_records_instance_class_create_named() {
        // `Dog create rex` maps `rex` -> `::Dog`.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog { method bark {} {} }\nDog create rex\n",
            "tcl",
        );
        assert_eq!(
            r.instance_classes.get("rex").map(String::as_str),
            Some("::Dog"),
        );
    }

    #[test]
    fn analyse_records_instance_class_set_create() {
        // `set d [Dog create rex]` maps `d` -> `::Dog`.
        let mut a = Analyser::new();
        let r = a.analyse("oo::class create Dog {}\nset d [Dog create rex]\n", "tcl");
        assert_eq!(
            r.instance_classes.get("d").map(String::as_str),
            Some("::Dog")
        );
    }

    #[test]
    fn analyse_records_instance_class_through_a_var_headed_create() {
        // TP — issue #923 idx 121: tcllib's `httpd/httpd.tcl` flows the
        // constructor's class name through a single, unconditional `set`
        // one line earlier (`set class ::Derived; set obj [$class create
        // NAME]`) rather than writing the class as a literal bareword.
        // Verified against real tclsh9.0/8.6: `$obj`'s methods dispatch to
        // `Dog` either way, so the analyser must bind `obj` -> `::Dog`
        // exactly like the literal-bareword shape already does.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog { method bark {} {} }\nset class Dog\nset obj [$class create rex]\n",
            "tcl",
        );
        assert_eq!(
            r.instance_classes.get("obj").map(String::as_str),
            Some("::Dog"),
            "{:?}",
            r.instance_classes
        );
    }

    #[test]
    fn analyse_records_instance_class_through_a_var_headed_new() {
        // TP — same gap, the `new` constructor spelling.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog { method bark {} {} }\nset class Dog\nset obj [$class new]\n",
            "tcl",
        );
        assert_eq!(
            r.instance_classes.get("obj").map(String::as_str),
            Some("::Dog"),
            "{:?}",
            r.instance_classes
        );
    }

    #[test]
    fn analyse_rejects_a_var_headed_non_manufacturer_word() {
        // FP guard: the shape parser accepts any literal method so that the
        // registry can decide after `$class` resolves. A normal method must
        // not become a constructor merely because it follows a class handle.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog { method inspect {} { return value } }\nset class Dog\nset obj [$class inspect]\n",
            "tcl",
        );
        assert_eq!(
            r.instance_classes.get("obj"),
            None,
            "{:?}",
            r.instance_classes
        );
    }

    #[test]
    fn analyse_rejects_unexported_create_with_namespace_dispatch() {
        // TN/FP guard pinned against tclsh 9.0.4: `Dog
        // createWithNamespace x ::xns` fails with TCL LOOKUP METHOD. The
        // registry retains its structural layout for self-dispatch, but an
        // ordinary `$class` command cannot use it as construction evidence.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog {}\nset class Dog\nset obj [$class createWithNamespace x ::xns]\n",
            "tcl",
        );
        assert_eq!(
            r.instance_classes.get("obj"),
            None,
            "{:?}",
            r.instance_classes
        );
    }

    #[test]
    fn analyse_abstains_on_a_branch_ambiguous_class_var() {
        // TN — a class variable whose reaching definitions genuinely
        // disagree (one arm `Dog`, the other `Cat`) is unprovable at the
        // constructor call: binding *either* class would be a guess, so
        // the analyser must abstain (no `instance_classes` entry) rather
        // than pick one arbitrarily.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog {}\noo::class create Cat {}\nif {$flag} { set class Dog } else { set class Cat }\nset obj [$class create x]\n",
            "tcl",
        );
        assert_eq!(
            r.instance_classes.get("obj"),
            None,
            "{:?}",
            r.instance_classes
        );
    }

    #[test]
    fn analyse_abstains_on_a_genuinely_dynamic_class_var() {
        // TN — a class variable fed by a computed value (no exact
        // constant) can't be proven at all; must abstain exactly like the
        // pre-fix behaviour rather than binding a wrong/empty class.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog {}\nset class [someFactory]\nset obj [$class create x]\n",
            "tcl",
        );
        assert_eq!(
            r.instance_classes.get("obj"),
            None,
            "{:?}",
            r.instance_classes
        );
    }

    #[test]
    fn analyse_does_not_record_class_definition_as_instance() {
        // `oo::class create Dog` defines a class, not an
        // instance — `Dog` must not appear in instance_classes.
        let mut a = Analyser::new();
        let r = a.analyse("oo::class create Dog {}\n", "tcl");
        assert!(
            !r.instance_classes.contains_key("Dog"),
            "class definition leaked into instance_classes: {:?}",
            r.instance_classes,
        );
    }

    #[test]
    fn analyse_ignores_instance_of_unknown_class() {
        // `set d [Widget new]` where Widget isn't a user class
        // records nothing.
        let mut a = Analyser::new();
        let r = a.analyse("set d [Widget new]\n", "tcl");
        assert!(r.instance_classes.is_empty(), "{:?}", r.instance_classes);
    }

    #[test]
    fn analyse_namespace_eval_opens_scope() {
        let mut a = Analyser::new();
        let r = a.analyse("namespace eval ns1 { }", "tcl");
        assert_eq!(r.global_scope.children.len(), 1);
        assert_eq!(r.global_scope.children[0].name, "ns1");
    }

    #[test]
    fn analyse_empty_source_is_empty_result() {
        let mut a = Analyser::new();
        let r = a.analyse("", "tcl");
        assert!(r.all_procs.is_empty());
        assert!(r.diagnostics.is_empty());
    }

    #[test]
    fn analyse_threads_file_suppression_into_disabled_diagnostics() {
        // ``# tcl-lsp: disable=W210,W211`` at the top of the file
        // must merge into ``self.disabled_diagnostics`` so
        // emitters honour the suppression.
        let mut a = Analyser::new();
        let _ = a.analyse("# tcl-lsp: disable=W210,W211\nproc foo {} {}\n", "tcl");
        assert!(a.disabled_diagnostics.contains("W210"));
        assert!(a.disabled_diagnostics.contains("W211"));
    }

    #[test]
    fn analyse_threads_next_line_noqa_into_suppressed_lines() {
        // A ``# noqa`` comment on its own line must seed
        // ``suppressed_lines`` for the *following* line via the
        // ``parse_noqa_line_suppressions`` pre-scan.  Line 0 carries
        // the ``# noqa`` directive so line 1 should be in the map.
        let mut a = Analyser::new();
        let r = a.analyse("# noqa\nset x 1\n", "tcl");
        let codes = r.suppressed_lines.get(&1).expect("line 1 entry");
        assert!(codes.contains("*"));
    }

    #[test]
    fn analyse_chunked_threads_next_line_noqa_into_suppressed_lines() {
        // Same wiring through ``analyse_chunked`` — the LSP's
        // primary incremental entry point.
        use crate::segmenter::SegmentedCommand;
        let mut a = Analyser::new();
        let cmds: Vec<Vec<SegmentedCommand>> = vec![Vec::new()];
        let (r, _) = a.analyse_chunked("# noqa: W210\nset x 1\n", cmds, "tcl");
        let codes = r.suppressed_lines.get(&1).expect("line 1 entry");
        assert!(codes.contains("W210"));
    }

    #[test]
    fn analyse_commands_threads_next_line_noqa_into_suppressed_lines() {
        // Same wiring through ``analyse_commands`` — the snapshot-
        // restore entry point.
        use crate::segmenter::SegmentedCommand;
        let mut a = Analyser::new();
        let cmds: Vec<SegmentedCommand> = Vec::new();
        let r = a.analyse_commands("# noqa: W210\nset x 1\n", &cmds, "tcl", true);
        let codes = r.suppressed_lines.get(&1).expect("line 1 entry");
        assert!(codes.contains("W210"));
    }

    #[test]
    fn analyse_file_suppression_unions_with_constructor_codes() {
        // Constructor-provided codes must survive file-suppression
        // merging — the two sources are unioned, not replaced.
        use std::collections::HashSet;
        let preconfigured: HashSet<String> = ["W120"].iter().map(|s| (*s).to_string()).collect();
        let mut a = Analyser::with_disabled_diagnostics(preconfigured);
        let _ = a.analyse("# tcl-lsp: disable=W210\n", "tcl");
        assert!(a.disabled_diagnostics.contains("W120"));
        assert!(a.disabled_diagnostics.contains("W210"));
    }

    #[test]
    fn analyse_runs_dedupe_and_disabled_filter_at_end() {
        // End-to-end: ``proc set {} {}`` emits W113.
        // ``# tcl-lsp: disable=W113`` at the top of the source
        // should silence it via ``apply_disabled_diagnostics``.
        let mut a = Analyser::new();
        let r = a.analyse("# tcl-lsp: disable=W113\nproc set {} {}\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W113),
            "W113 should be silenced by file-suppression directive",
        );
    }

    #[test]
    fn w113_silent_on_overridable_library_procs() {
        // FP-STY-13: the script-defined Tcl *library* procedures
        // (`unknown`, `history`, `auto_*`, `parray`, `pkg_mkIndex`,
        // `tcl_*` word helpers) are documented as user-replaceable
        // overlays, not C built-ins — redefining one must not fire
        // W113.
        let mut a = Analyser::new();
        for name in [
            "unknown",
            "history",
            "auto_execok",
            "auto_load",
            "auto_mkindex",
            "auto_qualify",
            "auto_reset",
            "parray",
            "pkg_mkIndex",
            "tcl_findLibrary",
            "tcl_wordBreakAfter",
            "tcl_endOfWord",
        ] {
            let r = a.analyse(&format!("proc {name} args {{ return }}\n"), "tcl");
            assert!(
                !r.diagnostics.iter().any(|d| d.code == DiagCode::W113),
                "W113 should be silent on overridable library proc {name:?}",
            );
        }
        // TP control: a genuine C built-in still fires.
        let r = a.analyse("proc set {} {}\n", "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == DiagCode::W113),
            "W113 must still fire when redefining a C built-in (`set`)",
        );
    }

    #[test]
    fn w307_array_set_const_element_suppression() {
        // TN: `array set state {-command puts}; $state(-command) hi` — the
        // literal element value `puts` is a known command, so the callback-key
        // heuristic must not fire W307. Harvested from the `array set` literal.
        let mut a = Analyser::new();
        let has_w307 = a
            .analyse(
                "proc f {} { array set state {-command puts}; $state(-command) hi }\n",
                "tcl",
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W307);
        assert!(
            !has_w307,
            "array-element holding a known command must not fire W307",
        );
        // TP control: a non-command literal value still fires W307.
        let mut b = Analyser::new();
        let has_w307 = b
            .analyse(
                "proc f {} { array set state {-command notACommand}; $state(-command) hi }\n",
                "tcl",
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W307);
        assert!(
            has_w307,
            "array-element holding a non-command must still fire W307",
        );
    }

    /// Helper: the RBS-family diagnostic codes (W210 read-before-set, W213
    /// unset-no-complain, W214 unused-param) emitted for `src`.
    fn rbs_codes(src: &str) -> Vec<String> {
        Analyser::new()
            .analyse(src, "tcl")
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W210" | "W213" | "W214"))
            .map(|d| d.code.to_string())
            .collect()
    }

    #[test]
    fn w220_must_alias_kill_same_array_element() {
        // FP-DS-06: two writes to the *same* literal-key array element with no
        // intervening read of it make the first dead — the later-version read
        // `$a(k)` reads the second write, not the first.  Must-alias kill
        // overrides the element-observed suppression.
        let mut a = Analyser::new();
        assert!(
            a.analyse(
                "proc f {} {\n    set a(k) 1\n    set a(k) 2\n    return $a(k)\n}",
                "tcl"
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
            "two writes to a(k) with no intervening read: first is dead",
        );
        // Control: different keys are independent — no dead store.
        let mut b = Analyser::new();
        assert!(
            !b.analyse(
                "proc f {} {\n    set a(k) 1\n    set a(j) 2\n    return $a(k)\n}",
                "tcl"
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
            "writes to different array elements are not dead stores",
        );
        // Control: an intervening read of the same element cancels the kill.
        let mut c = Analyser::new();
        assert!(
            !c.analyse(
                "proc f {} {\n    set a(k) 1\n    puts $a(k)\n    set a(k) 2\n    return $a(k)\n}",
                "tcl",
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
            "an intervening read of a(k) keeps the first write live",
        );
    }

    #[test]
    fn dead_store_recovers_rmw_read_in_command_sub() {
        // FP-DS-01: `[incr i $j]` buried in a substitution reads `i`'s prior
        // value, so the feeding `set i 0` is alive — no dead-store (W220/O109)
        // or unused (W211/O126).
        let mut a = Analyser::new();
        let codes: Vec<_> = a
            .analyse(
                "proc f {} {\n    set i 0\n    foreach j {1 2 3} { lappend r [incr i $j] }\n    return $r\n}\n",
                "tcl",
            )
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W220" | "W211" | "O109" | "O126"))
            .map(|d| d.code.to_string())
            .collect();
        assert!(
            codes.is_empty(),
            "RMW read in cmd-sub keeps set i 0 alive: {codes:?}"
        );
        // TP control: no read-modify-write of `i` — the first assignment is
        // truly dead, so a dead-store hint must still fire.
        let mut b = Analyser::new();
        assert!(
            b.analyse("proc f {} { set i 0\n set i 5\n return $i }", "tcl")
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
            "a genuine dead store must still fire",
        );
    }

    #[test]
    fn w210_dynamic_target_upvar_alias_read_is_silent() {
        // Issue #941. `upvar 1 $varName local` aliases `local` to the caller
        // variable named by `$varName` — the standard Tcl pass-by-reference
        // idiom. Reading `$local` is not read-before-set: it errors only when
        // the *caller* variable is missing, exactly the runtime condition that
        // would make the *literal*-target twin (`upvar 1 caller local`) error
        // too. tclsh 8.6/9.0 confirm the two forms are semantically identical,
        // so the analyser treats them alike — both silent. (Reverses the old
        // dynamic-target override, which flagged only the dynamic form and thus
        // fired on every by-name read helper.)
        let codes = rbs_codes("proc foo {varName} {\n  upvar 1 $varName local\n  puts $local\n}\n");
        assert!(
            !codes.iter().any(|c| c == "W210"),
            "dynamic-target upvar alias read must be silent: {codes:?}"
        );
        // Parity: the literal-target twin is silent too (always was).
        let codes = rbs_codes("proc foo {} {\n  upvar 1 caller local\n  puts $local\n}\n");
        assert!(
            !codes.iter().any(|c| c == "W210"),
            "literal-target upvar: {codes:?}"
        );
        // TP control: an *unrelated* local — not the alias, never set — is not a
        // scope alias, so it still fires W210.
        let codes =
            rbs_codes("proc foo {varName} {\n  upvar 1 $varName local\n  puts $unrelated\n}\n");
        assert!(
            codes.iter().any(|c| c == "W210"),
            "unrelated non-alias local must still fire W210: {codes:?}"
        );
        // An `[info exists local]` guard suppresses the read (always did).
        let codes = rbs_codes(
            "proc foo {varName} {\n  upvar 1 $varName local\n  if {[info exists local]} { puts $local }\n}\n",
        );
        assert!(
            !codes.iter().any(|c| c == "W210"),
            "guarded dynamic upvar: {codes:?}"
        );
        // A *write* through the alias is observable (goes to the caller var),
        // so it is not a dead store even with a dynamic target.
        let mut a = Analyser::new();
        assert!(
            !a.analyse(
                "proc foo {v} {\n  upvar 1 $v local\n  set local 5\n}\n",
                "tcl"
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220),
            "write through a dynamic upvar alias is observable, not a dead store",
        );
    }

    #[test]
    fn w210_gets_in_while_condition_writes_loop_var() {
        // FP-RBS-03: `[gets $fp line]` in the while condition writes `line`;
        // the body's `$line` reads must not fire W210.
        let codes = rbs_codes(
            "proc f {fp} {\n  while {[gets $fp line] >= 0} {\n    set n [string length $line]\n    puts \"$line ($n chars)\"\n  }\n}\n",
        );
        assert!(codes.is_empty(), "gets-in-condition writes line: {codes:?}");
    }

    #[test]
    fn w210_catch_in_expr_writes_result_var() {
        // FP-RBS-06: `[catch {…} tmp]` inside `[expr {…}]` writes `tmp`; the
        // same-expression `|| $tmp` read must not fire W210.
        let codes = rbs_codes(
            "proc f {sock} {\n  set eof [expr {[catch {eof $sock} tmp] || $tmp}]\n  return $eof\n}\n",
        );
        assert!(codes.is_empty(), "catch-in-expr writes tmp: {codes:?}");
    }

    #[test]
    fn w214_namespace_eval_body_does_not_recover_caller_param() {
        // FP-RBS-10: a `namespace eval ::ns {…}` body runs in the namespace
        // frame, so `$x` there is NOT a read of the caller's parameter `x` —
        // W214 (unused param) must still fire.  The `eval {…}` form runs in
        // the caller frame, so its `$x` read DOES recover the param (no W214).
        let mut a = Analyser::new();
        assert!(
            a.analyse(
                "proc g {x} {\n  namespace eval ::ns { puts \"hello $x\" }\n}\n",
                "tcl"
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W214),
            "namespace-eval body must not recover the caller's param read",
        );
        let mut b = Analyser::new();
        assert!(
            !b.analyse("proc f {x} {\n  eval { puts $x }\n}\n", "tcl")
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W214),
            "eval body runs in the caller frame; $x read recovers the param",
        );
    }

    #[test]
    fn w307_in_method_body_dispatch_fires_and_suppresses() {
        // TclOO method bodies are now walked in a `Method` scope, so their
        // `[cmd] method` dispatch sites are recorded for W307.
        // TP: `[format notACommand] run` returns a string, not an object.
        let mut a = Analyser::new();
        let fires = a
            .analyse(
                "oo::class create C { method m {} { [format notACommand] run } }",
                "tcl",
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W307);
        assert!(fires, "in-method [format ...] dispatch must fire W307");

        // Control: `[D new] run` where D is a known class returns an Object —
        // suppressed (and `run` resolves on D, so no W308 either).
        let mut b = Analyser::new();
        let fires = b
            .analyse(
                "oo::class create D { method run {} { return ok } }\n\
                 oo::class create C { method m {} { [D new] run } }\n",
                "tcl",
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W307);
        assert!(
            !fires,
            "in-method [D new] dispatch on known class must not fire W307"
        );
    }

    #[test]
    fn w307_my_method_literal_vs_object_return() {
        // TP: `[my plain] run` where `plain` returns a literal — the return is
        // a plain string, so the outer dispatch fires W307.
        let mut a = Analyser::new();
        let fires = a
            .analyse(
                "oo::class create C { method plain {} { return notACommand }\n\
                 method m {} { [my plain] run } }",
                "tcl",
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W307);
        assert!(fires, "[my plain] returning a literal must fire W307");

        // Control: `[my obj] run` where `obj` returns `[D new]` — object
        // handle, suppressed.
        let mut b = Analyser::new();
        let fires = b
            .analyse(
                "oo::class create D { method run {} { return ok } }\n\
                 oo::class create C { method obj {} { return [D new] }\n\
                 method m {} { [my obj] run } }",
                "tcl",
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W307);
        assert!(!fires, "[my obj] returning an object must not fire W307");
    }

    #[test]
    fn method_body_params_and_instance_vars_not_flagged() {
        // Walking method bodies must not false-fire read-before-set / unused
        // on the method's formal parameters or the class's instance variables.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create C {\n\
             variable count\n\
             method add {n} { incr count $n; return $count }\n\
             }\n",
            "tcl",
        );
        let offenders: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W210" | "W214"))
            .map(|d| d.code.to_string())
            .collect();
        assert!(
            offenders.is_empty(),
            "method params / instance vars must not fire W210/W214: {offenders:?}",
        );
    }

    #[test]
    fn w307_namespaced_ensemble_composed_name_resolution() {
        // TN: `${ns}::dowork` where `ns` is the const `mypkg` and
        // `::mypkg::dowork` is a known proc — the composed name resolves, so
        // no W307.
        let mut a = Analyser::new();
        let has = a
            .analyse(
                "namespace eval ::mypkg { proc dowork {arg} {} }\n\
                 proc f {} { set ns mypkg; ${ns}::dowork arg }\n",
                "tcl",
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W307);
        assert!(!has, "resolvable namespaced ensemble must not fire W307");
        // TP control: composed name with no known proc still fires.
        let mut b = Analyser::new();
        let has = b
            .analyse(
                "proc f {} { set ns mypkg; ${ns}::unknownproc arg }\n",
                "tcl",
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W307);
        assert!(
            has,
            "unresolvable composed namespaced ensemble must fire W307"
        );
    }

    #[test]
    fn w307_interproc_dict_with_const_element_suppression() {
        // TN: `dict with d { $cmd hi }` where the caller passes `{cmd puts}` —
        // the dict-with unpacks `cmd` to the known command `puts`, so the
        // body's `$cmd hi` dispatch must not fire W307.  The literal is
        // harvested from `d`'s call-site-propagated v0 SCCP const.
        let mut a = Analyser::new();
        let has_w307 = a
            .analyse(
                "proc f {d} { dict with d { $cmd hi } }\nf {cmd puts}\n",
                "tcl",
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W307);
        assert!(
            !has_w307,
            "dict-with-unpacked known command must not fire W307"
        );
        // TP control: a non-command unpacked value still fires.
        let mut b = Analyser::new();
        let has_w307 = b
            .analyse(
                "proc f {d} { dict with d { $cmd hi } }\nf {cmd notACommand}\n",
                "tcl",
            )
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W307);
        assert!(
            has_w307,
            "dict-with-unpacked non-command must still fire W307"
        );
    }

    #[test]
    fn w307_known_class_new_var_suppression() {
        // TN: `set x [C new]; $x run` where C is a known oo::class — `x` holds
        // an Object of class C, so the `$x run` dispatch resolves through the
        // method check (run exists on C) instead of firing W307.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create C { method run {} { return ok } }\n\
             proc f {} { set x [C new]; $x run }\n",
            "tcl",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
            "var assigned from a known-class constructor must not fire W307: {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w216_brace_then_paren_name_vs_value_positions() {
        // FP-STY-12: the braced indirect-array idiom `${name}(idx)` in a
        // *variable-name* position (`set`/`unset`/`incr`/`append`/`lappend`/
        // `vwait`/`info exists`) is the legitimate "array name held in a
        // scalar" access — neither W216 nor W212 fires.
        let mut a = Analyser::new();
        for src in [
            "set token ::http::1\nset ${token}(status) eof\n",
            "info exists ${token}(-pipeline)\n",
            "unset ${tok}(socketcoro)\n",
            "vwait ${token}(status)\n",
            "incr ${arr}(n)\n",
            "append ${arr}(buf) x\n",
            "lappend ${arr}(list) item\n",
        ] {
            let r = a.analyse(src, "tcl");
            assert!(
                !r.diagnostics
                    .iter()
                    .any(|d| d.code == DiagCode::W216 || d.code == DiagCode::W212),
                "indirect-array idiom must not fire W216/W212: {src:?} -> {:?}",
                r.diagnostics,
            );
        }
        // TP control: in a *value* position `${arr}(x)` is a broken read for
        // `$arr(x)` — W216 still fires.
        for src in ["puts ${arr}(x)\n", "set y ${arr}(x)\n"] {
            let r = a.analyse(src, "tcl");
            assert!(
                r.diagnostics.iter().any(|d| d.code == DiagCode::W216),
                "value-position ${{arr}}(x) must fire W216: {src:?} -> {:?}",
                r.diagnostics,
            );
        }
        // TP control: bare `set $x` / `set ${x}` (no `(idx)` suffix) is the
        // dynamic-name foot-gun, not the indirect idiom — W212 still fires.
        for src in ["set $x v\n", "set ${x} v\n"] {
            let r = a.analyse(src, "tcl");
            assert!(
                r.diagnostics.iter().any(|d| d.code == DiagCode::W212),
                "bare dynamic-name must still fire W212: {src:?} -> {:?}",
                r.diagnostics,
            );
        }
    }

    #[test]
    fn analyse_dedupes_back_to_back_identical_diagnostics() {
        // Two identical W113 emissions for the same proc name
        // should collapse to one.
        let mut a = Analyser::new();
        let r = a.analyse("proc set {} {}\nproc set {} {}\n", "tcl");
        // Re-defining ``set`` twice means handle_proc emits W113
        // twice — but the second emission is at a *different*
        // span (different proc-name token), so dedupe leaves
        // them both; the test that follows pins the actual count.
        let w113s: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W113)
            .collect();
        assert_eq!(
            w113s.len(),
            2,
            "two distinct ``proc set`` definitions → two distinct W113s at different spans",
        );
    }

    #[test]
    fn analyse_records_source_text_for_handler_re_slicing() {
        // Handlers re-slice ``self.source`` via spans returned by the
        // segmenter; the field must be populated at the top of
        // ``analyse``.
        let mut a = Analyser::new();
        let _ = a.analyse("set x 1", "tcl");
        assert_eq!(a.source, "set x 1");
    }

    #[test]
    fn analyse_records_dialect_for_w113_and_emitter_use() {
        // Handlers (W113 shadow check, dialect-only emitters) read
        // ``self.dialect()`` directly.  The field must be populated at
        // the top of ``analyse``.
        let mut a = Analyser::new();
        let _ = a.analyse("", "f5-irules");
        assert_eq!(a.dialect(), "f5-irules");
        assert!(a.profile.is_irules());
    }

    #[test]
    fn builtin_command_names_caches_per_dialect() {
        // First lookup populates the cache; subsequent lookups
        // with the same dialect return the same set.
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::plain_tcl();
        // ``set`` is a core built-in across all dialects.
        assert!(a.builtin_command_names().contains("set"));
        // I4/R-c (ledger C5): the set is the one `exists` oracle's
        // registry tier — the names the resolved context actually
        // provides, no longer the unfiltered store name set. Cache
        // invalidation: switching dialect rebuilds onto the new context.
        a.profile = tcl_dialect::DialectProfile::irules();
        assert!(
            a.builtin_command_names().contains("HTTP::header"),
            "the iRules surface is provided under f5-irules"
        );
        assert!(
            !a.builtin_command_names().contains("exec"),
            "a compiler-disabled, interpreter-absent core command does not \
             exist under the closed iRules world (R-c; pre-P1a the unfiltered \
             store set believed in it)"
        );
        assert!(
            a.builtin_command_names().contains("vwait"),
            "a §4b interpreter-present (compiler-refused) builtin still exists \
             for the oracle"
        );
    }

    #[test]
    fn analyse_commands_pre_segmented_records_proc() {
        // ``analyse_commands`` is the incremental entry — same
        // dispatcher as ``analyse``, but without re-segmentation.
        // Smoke-test that a pre-segmented chunk records its proc.
        use crate::segmenter::segment_commands;
        let source = "proc foo {} {}";
        let commands = segment_commands(source);
        let mut a = Analyser::new();
        let r = a.analyse_commands(source, &commands, "tcl", true);
        assert!(r.all_procs.contains_key("::foo"));
    }

    #[test]
    fn analyse_commands_fires_stray_close_bracket() {
        // The incremental / chunked path runs the E100 stray-`]`
        // check too, matching `analyse`.
        use crate::segmenter::segment_commands;
        let source = "puts foo]";
        let commands = segment_commands(source);
        let mut a = Analyser::new();
        let r = a.analyse_commands(source, &commands, "tcl", true);
        assert_eq!(
            r.diagnostics
                .iter()
                .filter(|d| d.code == DiagCode::E100)
                .count(),
            1,
            "expected one E100; got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.code.to_string())
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn analyse_chunked_returns_per_chunk_snapshots() {
        // ``analyse_chunked`` returns one snapshot per chunk.
        // Two chunks → two snapshots; the second snapshot
        // captures cumulative state.
        use crate::segmenter::segment_commands;
        let source = "set x 1\nproc foo {} {}";
        let chunk1 = segment_commands("set x 1");
        let chunk2 = segment_commands("proc foo {} {}");
        let mut a = Analyser::new();
        let (r, snapshots) = a.analyse_chunked(source, vec![chunk1, chunk2], "tcl");
        assert_eq!(snapshots.len(), 2);
        // After chunk 1, x is in scope.
        assert!(snapshots[0].result.global_scope.variables.contains_key("x"));
        // After chunk 2, foo is in all_procs.
        assert!(snapshots[1].result.all_procs.contains_key("::foo"));
        // The final result has both.
        assert!(r.global_scope.variables.contains_key("x"));
        assert!(r.all_procs.contains_key("::foo"));
    }

    #[test]
    fn analyse_commands_finalise_false_skips_diagnostic_tail() {
        // When ``finalise=false``, the dedupe/disabled-codes
        // filters don't run — useful for partial-snapshot paths
        // where the tail is deferred.
        use crate::segmenter::segment_commands;
        let source = "proc set {} {}"; // would normally trip W113
        let commands = segment_commands(source);
        let mut a = Analyser::new();
        a.profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let r = a.analyse_commands(source, &commands, "tcl", false);
        // W113 was emitted by handle_proc but the tail didn't
        // run, so apply_disabled_diagnostics / dedupe didn't
        // touch the diag list.  The diag is still there.
        assert!(r.diagnostics.iter().any(|d| d.code == DiagCode::W113));
    }

    #[test]
    fn default_constructs_via_new() {
        let a = Analyser::default();
        assert_eq!(a.current_scope_path.len(), 0);
        assert_eq!(a.body_depth, 0);
    }

    // -- tcllib `<NS>::import <ALIAS>` wrapper detection

    #[test]
    fn analyse_records_tcllib_import_wrapper_as_conjectured() {
        let mut a = Analyser::new();
        let r = a.analyse("term::ansi::send::import vt\n", "tcl");
        let conjectured: Vec<_> = r
            .namespace_imports
            .iter()
            .filter(|i| i.conjectured)
            .collect();
        assert_eq!(conjectured.len(), 1);
        assert_eq!(conjectured[0].ns, "::vt");
        assert_eq!(conjectured[0].pattern, "::term::ansi::send::*");
    }

    #[test]
    fn analyse_tcllib_import_wrapper_alias_relative_to_current_namespace() {
        // ``some::ns::import alias`` inside ``namespace eval outer``
        // creates ``::outer::alias``, not ``::alias``.
        let mut a = Analyser::new();
        let r = a.analyse(
            "namespace eval outer { term::ansi::send::import vt }\n",
            "tcl",
        );
        let conjectured: Vec<_> = r
            .namespace_imports
            .iter()
            .filter(|i| i.conjectured)
            .collect();
        assert_eq!(conjectured.len(), 1);
        assert_eq!(conjectured[0].ns, "::outer::vt");
        assert_eq!(conjectured[0].pattern, "::term::ansi::send::*");
    }

    #[test]
    fn analyse_tcllib_import_wrapper_absolute_alias_keeps_leading_colons() {
        // ``::alias`` argument is taken as an absolute namespace —
        // current-namespace prefixing is skipped.
        let mut a = Analyser::new();
        let r = a.analyse("term::ansi::send::import ::abs::vt\n", "tcl");
        let conjectured: Vec<_> = r
            .namespace_imports
            .iter()
            .filter(|i| i.conjectured)
            .collect();
        assert_eq!(conjectured.len(), 1);
        assert_eq!(conjectured[0].ns, "::abs::vt");
    }

    #[test]
    fn analyse_tcllib_import_wrapper_skips_substituted_alias() {
        // ``$var`` / ``[cmd]`` aliases can't be statically resolved —
        // the guard requires the alias to contain no ``$`` or ``[``.
        let mut a = Analyser::new();
        let r1 = a.analyse("term::ansi::send::import $alias\n", "tcl");
        assert!(r1.namespace_imports.iter().all(|i| !i.conjectured));
        let mut a = Analyser::new();
        let r2 = a.analyse("term::ansi::send::import [build]\n", "tcl");
        assert!(r2.namespace_imports.iter().all(|i| !i.conjectured));
    }

    #[test]
    fn analyse_tcllib_import_wrapper_requires_single_argument() {
        // ``X::import alias extras`` is a non-wrapper call — the
        // wrapper idiom takes exactly one alias word.
        let mut a = Analyser::new();
        let r = a.analyse("term::ansi::send::import vt extra\n", "tcl");
        assert!(r.namespace_imports.iter().all(|i| !i.conjectured));
    }

    #[test]
    fn analyse_tcllib_import_wrapper_qualifies_unprefixed_source_ns() {
        // Wrapper command names without a leading ``::`` still
        // resolve to absolute source namespaces — the helper
        // prepends the missing ``::``.
        let mut a = Analyser::new();
        let r = a.analyse("foo::import vt\n", "tcl");
        let conjectured: Vec<_> = r
            .namespace_imports
            .iter()
            .filter(|i| i.conjectured)
            .collect();
        assert_eq!(conjectured.len(), 1);
        assert_eq!(conjectured[0].pattern, "::foo::*");
    }

    // -- ``command_aliases`` tests
    //
    // Pin the no-transitive-chain behaviour of command aliases.

    #[test]
    fn analyse_alias_chain_records_each_step_independently() {
        // ``a -> b`` and ``b -> expr`` are recorded as two
        // independent entries — neither side resolves
        // transitively to ``expr``.
        let mut a = Analyser::new();
        let r = a.analyse("interp alias {} a {} b\ninterp alias {} b {} expr\n", "tcl");
        let alias_a = r.command_aliases.get("::a").expect("::a recorded");
        assert_eq!(alias_a.target, "b");
        assert!(alias_a.extras.is_empty());
        let alias_b = r.command_aliases.get("::b").expect("::b recorded");
        assert_eq!(alias_b.target, "expr");
        assert!(alias_b.extras.is_empty());
    }

    #[test]
    fn analyse_alias_redefinition_overwrites_target() {
        // The second declaration wins.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp alias {} myop {} expr\ninterp alias {} myop {} puts\n",
            "tcl",
        );
        let alias = r.command_aliases.get("::myop").expect("::myop recorded");
        assert_eq!(alias.target, "puts");
        assert!(alias.extras.is_empty());
    }

    #[test]
    fn analyse_alias_qualified_name_recorded() {
        // ``interp alias {} ::ns::myop {} expr`` records under the
        // fully-qualified key.
        let mut a = Analyser::new();
        let r = a.analyse("interp alias {} ::math::= {} expr\n", "tcl");
        let alias = r
            .command_aliases
            .get("::math::=")
            .expect("::math::= recorded");
        assert_eq!(alias.target, "expr");
    }

    #[test]
    fn analyse_alias_dynamic_name_not_recorded() {
        // ``$n`` in the alias name field doesn't resolve statically
        // — ``::=`` must not appear in ``command_aliases``.
        let mut a = Analyser::new();
        let r = a.analyse("set n \"=\"\ninterp alias {} $n {} expr\n", "tcl");
        assert!(!r.command_aliases.contains_key("::="));
    }

    // -- ``switch -regexp`` literal-pattern recording
    //
    // The pattern arms whose token is a literal are recorded as
    // ``RegexPattern { command = "switch" }``; ``default`` and
    // var / cmd-sub patterns are skipped (variable patterns are
    // handled by the regex-vars resolution instead).

    #[test]
    fn analyse_switch_regexp_form1_records_literal_patterns() {
        // Form 1: pattern/body pairs inline.
        let mut a = Analyser::new();
        let r = a.analyse(
            "switch -regexp -- $val \"^foo\" { puts foo } \"^bar\" { puts bar }\n",
            "tcl",
        );
        let switch_pats: Vec<_> = r
            .regex_patterns
            .iter()
            .filter(|p| p.command == "switch")
            .collect();
        assert_eq!(switch_pats.len(), 2);
        assert_eq!(switch_pats[0].pattern, "^foo");
        assert_eq!(switch_pats[1].pattern, "^bar");
    }

    #[test]
    fn analyse_switch_regexp_form2_records_literal_patterns() {
        // Form 2: braced body with pattern/body pairs.
        let mut a = Analyser::new();
        let r = a.analyse(
            "switch -regexp -- $val { ^foo { puts foo } ^bar { puts bar } }\n",
            "tcl",
        );
        let switch_pats: Vec<_> = r
            .regex_patterns
            .iter()
            .filter(|p| p.command == "switch")
            .collect();
        assert_eq!(switch_pats.len(), 2);
        assert_eq!(switch_pats[0].pattern, "^foo");
        assert_eq!(switch_pats[1].pattern, "^bar");
    }

    #[test]
    fn analyse_braced_expect_clause_regexp_records_literal_and_const_var_patterns() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "set pat {(a+)+$}\nexpect {-regexp {^lit$} { puts lit } -re $pat { puts var }}\n",
            "expect",
        );
        let patterns: Vec<_> = r
            .regex_patterns
            .iter()
            .filter(|p| p.command == "expect")
            .collect();
        assert!(
            patterns.iter().any(|pattern| pattern.pattern == "^lit$"),
            "literal per-clause regexp missing: {patterns:?}"
        );
        assert!(
            patterns.iter().any(|pattern| pattern.pattern == "(a+)+$"),
            "const variable per-clause regexp missing: {patterns:?}"
        );
    }

    #[test]
    fn analyse_switch_regexp_skips_default_arm() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "switch -regexp -- $val { ^foo { puts foo } default { puts none } }\n",
            "tcl",
        );
        let switch_pats: Vec<_> = r
            .regex_patterns
            .iter()
            .filter(|p| p.command == "switch")
            .collect();
        assert_eq!(switch_pats.len(), 1);
        assert_eq!(switch_pats[0].pattern, "^foo");
    }

    #[test]
    fn analyse_switch_without_regexp_records_nothing() {
        // No ``-regexp`` flag — patterns are glob, not regex.
        let mut a = Analyser::new();
        let r = a.analyse(
            "switch -- $val { foo { puts foo } bar { puts bar } }\n",
            "tcl",
        );
        let switch_pats: Vec<_> = r
            .regex_patterns
            .iter()
            .filter(|p| p.command == "switch")
            .collect();
        assert!(switch_pats.is_empty());
    }

    #[test]
    fn analyse_switch_regexp_skips_unresolved_var_pattern() {
        // ``$pat`` arm with no defining ``set`` — no const value
        // available, so the arm is dropped.
        let mut a = Analyser::new();
        let r = a.analyse(
            "switch -regexp -- $val { $pat { puts hit } ^lit { puts lit } }\n",
            "tcl",
        );
        let switch_pats: Vec<_> = r
            .regex_patterns
            .iter()
            .filter(|p| p.command == "switch")
            .collect();
        assert_eq!(switch_pats.len(), 1);
        assert_eq!(switch_pats[0].pattern, "^lit");
    }

    // -- ``regex-vars`` const-string propagation
    //
    // Verify that ``$var`` regex pattern arguments resolve to the
    // literal stored by a preceding ``set var "..."`` (regexp /
    // regsub and switch -regexp Form 2).

    #[test]
    fn analyse_regexp_resolves_var_pattern_to_const_string() {
        let mut a = Analyser::new();
        let r = a.analyse("set p {^foo}\nregexp $p $line\n", "tcl");
        // Two records: the use site (the ``$p`` token) and the
        // defining ``set`` value.
        let regexp_pats: Vec<_> = r
            .regex_patterns
            .iter()
            .filter(|p| p.command == "regexp")
            .collect();
        assert_eq!(regexp_pats.len(), 2);
        assert!(regexp_pats.iter().all(|p| p.pattern == "^foo"));
    }

    #[test]
    fn analyse_regsub_resolves_var_pattern_to_const_string() {
        let mut a = Analyser::new();
        let r = a.analyse("set p {a+}\nregsub -all $p $line - out\n", "tcl");
        let regsub_pats: Vec<_> = r
            .regex_patterns
            .iter()
            .filter(|p| p.command == "regsub")
            .collect();
        assert_eq!(regsub_pats.len(), 2);
        assert!(regsub_pats.iter().all(|p| p.pattern == "a+"));
    }

    #[test]
    fn analyse_switch_regexp_resolves_var_pattern_to_const_string() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "set p {^foo}\nswitch -regexp -- $val { $p { puts foo } ^bar { puts bar } }\n",
            "tcl",
        );
        let switch_pats: Vec<_> = r
            .regex_patterns
            .iter()
            .filter(|p| p.command == "switch")
            .collect();
        let pats: Vec<&str> = switch_pats.iter().map(|p| p.pattern.as_str()).collect();
        assert!(pats.contains(&"^foo"), "got {pats:?}");
        assert!(pats.contains(&"^bar"), "got {pats:?}");
    }

    #[test]
    fn analyse_regex_var_unresolved_records_nothing() {
        // No defining ``set`` — Var has no const value.  The
        // pattern arg is dropped.
        let mut a = Analyser::new();
        let r = a.analyse("regexp $p $line\n", "tcl");
        assert!(r.regex_patterns.is_empty());
    }

    // -- W105 unbraced-body emitter

    #[test]
    fn analyse_emits_w105_for_unbraced_if_body_with_substitution() {
        let mut a = Analyser::new();
        let r = a.analyse("if {$cond} \"puts $x\"\n", "tcl");
        let w105: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W105)
            .collect();
        assert!(!w105.is_empty(), "expected W105, got {:?}", r.diagnostics);
        // Substitution-bearing bodies are flagged at error severity.
        assert!(matches!(w105[0].severity, crate::analyser::Severity::Error));
    }

    #[test]
    fn analyse_skips_w105_for_braced_if_body() {
        // Braced ``{ ... }`` body — no W105.
        let mut a = Analyser::new();
        let r = a.analyse("if {$cond} { puts $x }\n", "tcl");
        assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W105));
    }

    #[test]
    fn analyse_skips_w105_for_command_substitution_body() {
        // A whole-word `[…]` command-substitution body is the safe
        // list-building idiom — `check_unbraced_body` exempts `Cmd`
        // tokens, so no W105.
        let mut a = Analyser::new();
        let r = a.analyse("eval [list set y $x]\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W105),
            "{:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_skips_w105_for_single_var_body() {
        // FP-STY-14: a body that is a *single bare variable* (`$body`,
        // `$cmd`, `$script`) is a script-valued reference — the variable
        // already holds the script — not an inline code block.  Bracing it
        // (`{$body}`) would turn the reference into the literal text, so the
        // W105 quick-fix is wrong and must not fire: `while {$cond} $body`
        // → no W105, as for `eval $cmd`, `proc $n $a $body`, `uplevel $script`.
        let mut a = Analyser::new();
        for src in [
            "while {$cond} $body\n",
            "eval $cmd\n",
            "after 0 $coroName\n",
            "proc $fakeName $arglist $body\n",
            "foreach name $nameList $body\n",
            "uplevel $script\n",
        ] {
            let r = a.analyse(src, "tcl");
            assert!(
                !r.diagnostics.iter().any(|d| d.code == DiagCode::W105),
                "W105 should be silent on single-var body {src:?}, got {:?}",
                r.diagnostics,
            );
        }
    }

    #[test]
    fn analyse_emits_w105_for_quoted_interpolated_body() {
        // TP control: a *quoted* body with interpolation (`eval "do $script"`)
        // really is an inline script woven from substitutions — it can and
        // should be braced, so W105 still fires at ERROR severity.
        let mut a = Analyser::new();
        let r = a.analyse("eval \"do $script\"\n", "tcl");
        let w105: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W105)
            .collect();
        assert!(!w105.is_empty(), "expected W105, got {:?}", r.diagnostics);
        assert!(matches!(w105[0].severity, crate::analyser::Severity::Error));
    }

    // -- W110 string-compare-in-expr emitter

    #[test]
    fn analyse_emits_w110_for_string_eq_in_if_condition() {
        let mut a = Analyser::new();
        let r = a.analyse("if {$x == \"foo\"} {puts yes}\n", "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
        assert!(w110[0].message.contains("eq"), "got {:?}", w110[0].message);
        assert!(matches!(w110[0].severity, crate::analyser::Severity::Hint));
    }

    #[test]
    fn analyse_emits_w110_for_string_ne_in_if_condition() {
        let mut a = Analyser::new();
        let r = a.analyse("if {$x != \"bar\"} {puts no}\n", "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
        assert!(w110[0].message.contains("ne"), "got {:?}", w110[0].message);
    }

    #[test]
    fn w110_span_anchors_on_the_operator() {
        // Range precision: the diagnostic anchors on the `==` itself, not
        // the whole condition.
        let src = "if {$x == \"foo\"} {puts yes}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
        let expected = src.find("==").unwrap();
        assert_eq!(
            (w110[0].span.start() as usize, w110[0].span.end() as usize),
            (expected, expected + 2),
            "W110 must cover exactly the operator, got {:?}",
            w110[0].span
        );
        // The code fix still replaces the whole condition text.
        assert!(!w110[0].fixes.is_empty(), "fix expected: {:?}", w110[0]);
        assert!(
            w110[0].fixes[0].span.start() < w110[0].span.start(),
            "fix span must cover the argument, not just the operator"
        );
    }

    #[test]
    fn w110_span_anchors_on_the_matched_operator_not_the_first() {
        // `$a == $b` (variable compare — not flagged) precedes the
        // string compare `$c == "x"`; the anchor must be the SECOND `==`.
        let src = "if {$a == $b && $c == \"x\"} {puts yes}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
        let expected = src.rfind("==").unwrap();
        assert_eq!(
            (w110[0].span.start() as usize, w110[0].span.end() as usize),
            (expected, expected + 2),
            "W110 must anchor the matched (second) `==`, got {:?}",
            w110[0].span
        );
        // Mixed string/non-string compares must not offer the blanket fix.
        assert!(w110[0].fixes.is_empty(), "no fix expected: {:?}", w110[0]);
    }

    #[test]
    fn analyse_no_w110_for_numeric_compare() {
        // ``$x == 42`` — numeric literal on the right, no string
        // operand, should not fire.
        let mut a = Analyser::new();
        let r = a.analyse("if {$x == 42} {puts yes}\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_no_w110_for_eq_operator() {
        // ``$x eq "foo"`` is the correct form — no W110.
        let mut a = Analyser::new();
        let r = a.analyse("if {$x eq \"foo\"} {puts yes}\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_w110_includes_eq_code_fix() {
        // Single ``==`` against a string literal — the blanket
        // rewrite should run and produce a fix containing ``eq``.
        let mut a = Analyser::new();
        let r = a.analyse("if {$x == \"foo\"} {puts yes}\n", "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
        assert_eq!(w110[0].fixes.len(), 1, "got {:?}", w110[0].fixes);
        assert!(
            w110[0].fixes[0].new_text.contains("eq"),
            "got {:?}",
            w110[0].fixes[0].new_text
        );
    }

    #[test]
    fn analyse_no_w110_for_variable_only_compare() {
        // ``$a == $b`` — both operands are variables, may hold
        // ints, no W110.
        let mut a = Analyser::new();
        let r = a.analyse("if {$a == $b} {puts yes}\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_w110_fires_for_numeric_string_literal() {
        // ``$x == "42"`` — user explicitly wrote a string literal
        // (with quotes), so W110 still fires.
        let mut a = Analyser::new();
        let r = a.analyse("if {$x == \"42\"} {puts yes}\n", "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    }

    #[test]
    fn analyse_w110_fires_for_boolean_string_literal() {
        // ``$x == "true"`` — boolean-spelled string literal still
        // counts as ExprString.
        let mut a = Analyser::new();
        let r = a.analyse("if {$x == \"true\"} {puts yes}\n", "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    }

    #[test]
    fn analyse_w110_fires_through_unary_negation() {
        // ``!($x == "foo")`` — W110 walks through ExprUnary.
        let mut a = Analyser::new();
        let r = a.analyse("if {!($x == \"foo\")} {puts no}\n", "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    }

    #[test]
    fn analyse_w110_no_fix_when_some_compare_is_non_string() {
        // ``$a == $b || $x == "foo"`` — only one of the two
        // ``==`` ops has a string operand; the blanket regex
        // rewrite would corrupt the var-only ``==``, so the fix
        // is suppressed.
        let mut a = Analyser::new();
        let r = a.analyse("if {$a == $b || $x == \"foo\"} {puts y}\n", "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
        assert_eq!(w110[0].fixes.len(), 0, "got {:?}", w110[0].fixes);
    }

    #[test]
    fn analyse_w110_fires_on_while_condition() {
        // ``while {EXPR} {body}`` — EXPR-role is at index 0.
        let mut a = Analyser::new();
        let r = a.analyse("while {$x == \"foo\"} { break }\n", "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    }

    #[test]
    fn analyse_w110_fires_on_top_level_expr_command() {
        // ``expr {$x == "foo"}`` — top-level invocation of
        // ``expr`` exercises the EXPR-role dispatch on the
        // single braced arg.  (Nested ``[expr ...]`` command
        // substitutions are recorded as invocations but the
        // analyser doesn't currently re-enter them for per-
        // command checks; that's a separate concern.)
        let mut a = Analyser::new();
        let r = a.analyse("expr {$x == \"foo\"}\n", "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    }

    #[test]
    fn analyse_w110_fires_on_multi_arg_expr_command() {
        // ``expr $x == "foo"`` (no braces, multiple argv slots): the args
        // are joined with spaces — the *substituted* word values, whose
        // quote delimiters Tcl's word splitting already stripped — so the
        // expression is ``$x == foo`` with ``foo`` a bareword, not an
        // ``ExprString``.  W110 (string ``==``) therefore does NOT fire on
        // the unbraced form (only W100 does); it fires on the *braced*
        // ``expr {$x == "foo"}`` where the string literal survives.
        let mut a = Analyser::new();
        let r = a.analyse("expr $x == \"foo\"\n", "tcl");
        let unbraced: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert!(
            unbraced.is_empty(),
            "unbraced expr must not fire W110, got {:?}",
            r.diagnostics
        );

        let mut a2 = Analyser::new();
        let r2 = a2.analyse("set z [expr {$x == \"foo\"}]\n", "tcl");
        let braced: Vec<_> = r2
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(
            braced.len(),
            1,
            "braced expr must fire W110, got {:?}",
            r2.diagnostics
        );
    }

    #[test]
    fn call_by_name_var_not_flagged_unused_or_dead() {
        // A caller-local passed *by name* to a proc that upvar-writes
        // its param (`fill tag`) must not be flagged W211 (unused) or
        // W220 (dead store).
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc ::fill {vn} { upvar 1 $vn v\nset v 1 }\n\
             proc ::f {} { set tag init\nset tag x\nfill tag }\n",
            "tcl",
        );
        assert!(
            r.diagnostics.iter().all(|d| {
                !((d.code == DiagCode::W211 || d.code == DiagCode::W220)
                    && d.message.contains("tag"))
            }),
            "call-by-name var `tag` must not be flagged unused/dead, got {:?}",
            r.diagnostics,
        );

        // Negative control: a genuinely unused local is still flagged
        // (the suppression didn't disable W211 wholesale).
        let mut a2 = Analyser::new();
        let r2 = a2.analyse("proc ::g {} { set unused 1\nputs hi }\n", "tcl");
        assert!(
            r2.diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W211 && d.message.contains("unused")),
            "a genuinely unused var should still be flagged, got {:?}",
            r2.diagnostics,
        );
    }

    #[test]
    fn dynamic_name_write_does_not_dead_store_the_name_var() {
        // `set $p 1` writes the variable *named by* `$p` and *reads* `p`
        // (verified against tclsh 8.4–9.0).  It must not produce a spurious
        // W220 dead-store on the parameter `p` itself.
        for body in ["set $p 1", "set ${p} 1", "set $p [expr {1}]"] {
            let mut a = Analyser::new();
            let r = a.analyse(&format!("proc ::f {{p}} {{ {body} }}\n"), "tcl");
            assert!(
                r.diagnostics
                    .iter()
                    .all(|d| !(matches!(d.code.as_str(), "W211" | "W220" | "W214")
                        && d.message.contains("'p'"))),
                "dynamic-name write `{body}` wrongly flagged `p`, got {:?}",
                r.diagnostics,
            );
        }
    }

    #[test]
    fn dynamic_name_out_var_does_not_suppress_caller_dead_store() {
        // A param used only as a *dynamic name* out-var (`scan`/`lassign`/
        // `regexp`/`regsub` target, or `set $p`) names a callee-local
        // variable — it does NOT write back to the caller's frame, so a
        // caller's literal arg is genuinely dead and must still fire W211 /
        // W220.
        let cases = [
            "proc maybe {target} { scan 42 %d $target }",
            "proc maybe {target} { lassign {1} $target }",
            "proc maybe {target} { regexp {(.)} a -> $target }",
            "proc maybe {target} { regsub a a b $target }",
            "proc maybe {target} { set $target 1 }",
        ];
        for callee in cases {
            let src = format!("{callee}\nproc caller {{}} {{ set x 1; maybe x }}\n");
            let mut a = Analyser::new();
            let r = a.analyse(&src, "tcl");
            assert!(
                r.diagnostics
                    .iter()
                    .any(|d| (d.code == DiagCode::W211 || d.code == DiagCode::W220)
                        && d.message.contains("'x'")),
                "caller's dead `x` must still fire for `{callee}`, got {:?}",
                r.diagnostics,
            );
        }
    }

    #[test]
    fn upvar_writeback_still_suppresses_caller_dead_store() {
        // Control: a genuine `upvar`-write-back callee DOES consume the
        // caller's variable, so the dead-store is correctly suppressed.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc fill {outvar} { upvar $outvar out\nset out 42 }\n\
             proc top {} { set result {}\nfill result\nreturn $result }\n",
            "tcl",
        );
        assert!(
            r.diagnostics
                .iter()
                .all(|d| !((d.code == DiagCode::W211 || d.code == DiagCode::W220)
                    && d.message.contains("result"))),
            "upvar write-back must suppress the caller dead-store, got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w128_fires_on_call_to_renamed_command() {
        // A builtin renamed/deleted away earlier in the file → the
        // later call falls through to `unknown` → W128.
        let mut a = Analyser::new();
        let r = a.analyse("rename string {}\nstring toupper x\n", "tcl");
        let w128: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W128)
            .collect();
        assert_eq!(w128.len(), 1, "expected one W128, got {:?}", r.diagnostics);

        // Negative: a normal builtin call with no rename → no W128.
        let mut a2 = Analyser::new();
        let r2 = a2.analyse("string toupper x\n", "tcl");
        assert!(
            r2.diagnostics.iter().all(|d| d.code != DiagCode::W128),
            "unexpected W128: {:?}",
            r2.diagnostics,
        );

        // Negative: an ordinary unknown external command (never rebound)
        // resolves opaque but must not fire W128.
        let mut a3 = Analyser::new();
        let r3 = a3.analyse("someunknowncmd a b\n", "tcl");
        assert!(
            r3.diagnostics.iter().all(|d| d.code != DiagCode::W128),
            "unexpected W128: {:?}",
            r3.diagnostics,
        );
    }

    #[test]
    fn analyse_w110_no_fire_on_for_clean_condition() {
        // ``for {set i 0} {$i < 10} {incr i} {body}`` — no ``==``
        // anywhere, but ensure the EXPR-role dispatch on ``for``
        // doesn't crash and produces no W110.
        let mut a = Analyser::new();
        let r = a.analyse("for {set i 0} {$i < 10} {incr i} { break }\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_w110_fires_on_for_condition() {
        // ``for {set i 0} {$x == "foo"} {incr i} {body}`` —
        // ``handle_for_command`` returns early from
        // ``process_command``, so the EXPR-role dispatch must
        // run *before* the early-return handlers (otherwise
        // W110 on a ``for`` condition would silently miss).
        let mut a = Analyser::new();
        let r = a.analyse("for {set i 0} {$x == \"foo\"} {incr i} { break }\n", "tcl");
        let w110: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W110)
            .collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    }

    // -- W302 catch-without-result-var emitter

    #[test]
    fn analyse_emits_w302_for_catch_without_result_var() {
        let mut a = Analyser::new();
        let r = a.analyse("catch { puts hi }\n", "tcl");
        let w302: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W302)
            .collect();
        assert_eq!(w302.len(), 1, "got {:?}", r.diagnostics);
        assert!(
            w302[0].message.contains("silently swallows errors"),
            "got {:?}",
            w302[0].message
        );
        assert!(matches!(w302[0].severity, crate::analyser::Severity::Hint));
    }

    #[test]
    fn analyse_no_w302_when_catch_has_result_var() {
        // ``catch BODY result`` — result variable is present, so
        // errors aren't silently swallowed.
        let mut a = Analyser::new();
        let r = a.analyse("catch { puts hi } result\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_no_w302_when_catch_has_options_var() {
        // ``catch BODY result options`` — both optional vars
        // present.  Still no W302.
        let mut a = Analyser::new();
        let r = a.analyse("catch { puts hi } result options\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_no_w302_for_multi_token_catch_body() {
        // ``catch pre$x`` — a multi-token-word body is a dynamic
        // body (not a single-token word), so it is not treated as a
        // braced catch body and W302 never fires.  The emitter gates
        // on the body being a single-token word.
        let mut a = Analyser::new();
        let r = a.analyse("catch pre$x\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_w302_anchors_at_catch_keyword() {
        // W302 highlights just the ``catch`` command token — the
        // narrowest span that identifies the issue — rather than the
        // whole ``catch {…}`` statement
        // (which also dropped the closing brace under the lexer's
        // inner-end convention).
        let mut a = Analyser::new();
        let src = "catch { puts hi }\n";
        let r = a.analyse(src, "tcl");
        let w302: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W302)
            .collect();
        assert_eq!(w302.len(), 1);
        let span = w302[0].span;
        let text = &src[span.start() as usize..span.end() as usize];
        assert_eq!(text, "catch", "W302 should span only the catch keyword");
    }

    // -- W302 quick-fix insertion anchor (issue #1190)
    //
    // The diagnostic anchors at the `catch` keyword, so the fix must carry
    // its **own** span: the point past the *body's* closing delimiter.  Each
    // test applies the fix to the source and asserts the resulting text, so a
    // wrong anchor cannot pass by matching only the inserted string.

    /// Apply W302's `nth` fix to `src` and return the rewritten source.
    fn apply_w302_fix(src: &str, nth: usize, dialect: &str) -> String {
        let mut a = Analyser::new();
        let r = a.analyse(src, dialect);
        let w302: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W302)
            .collect();
        assert_eq!(w302.len(), 1, "expected one W302, got {:?}", r.diagnostics);
        let fix = w302[0]
            .fixes
            .get(nth)
            .unwrap_or_else(|| panic!("no fix {nth}: {:?}", w302[0].fixes));
        let mut out = src.to_string();
        out.replace_range(
            fix.span.start() as usize..fix.span.end() as usize,
            &fix.new_text,
        );
        out
    }

    #[test]
    fn w302_result_fix_inserts_after_the_body() {
        // The issue's reproducer.  Anchoring at the diagnostic's end would
        // produce `catch result {error oops}`, which C Tcl reads as a catch
        // of the script `result` — completion code 1, result
        // `invalid command name "result"`.
        assert_eq!(
            apply_w302_fix("catch {error oops}\n", 0, "tcl9.0"),
            "catch {error oops} result\n"
        );
    }

    #[test]
    fn w302_options_fix_inserts_after_the_body() {
        assert_eq!(
            apply_w302_fix("catch {error oops}\n", 1, "tcl9.0"),
            "catch {error oops} result options\n"
        );
    }

    #[test]
    fn w302_fix_handles_a_multiline_body() {
        let src = "catch {\n    error oops\n}\n";
        assert_eq!(
            apply_w302_fix(src, 0, "tcl9.0"),
            "catch {\n    error oops\n} result\n"
        );
    }

    #[test]
    fn w302_fix_handles_a_trailing_comment() {
        let src = "catch {error oops} ;# ignore\n";
        assert_eq!(
            apply_w302_fix(src, 0, "tcl9.0"),
            "catch {error oops} result ;# ignore\n"
        );
    }

    #[test]
    fn w302_fix_handles_a_semicolon_separated_command() {
        let src = "catch {error oops}; puts done\n";
        assert_eq!(
            apply_w302_fix(src, 0, "tcl9.0"),
            "catch {error oops} result; puts done\n"
        );
    }

    #[test]
    fn w302_fix_handles_nested_substitutions_in_the_body() {
        let src = "catch {puts [expr {1 + [foo $x]}]}\n";
        assert_eq!(
            apply_w302_fix(src, 0, "tcl9.0"),
            "catch {puts [expr {1 + [foo $x]}]} result\n"
        );
    }

    #[test]
    fn w302_fix_does_not_overshoot_an_empty_body() {
        // `catch {}`'s body span already covers its own closer, so a naive
        // `span.end() + 1` anchor would insert one byte past the brace.
        let src = "catch {}\n";
        assert_eq!(apply_w302_fix(src, 0, "tcl9.0"), "catch {} result\n");
    }

    #[test]
    fn w302_fix_handles_a_leading_qualified_head() {
        // `::catch` resolves to the same registry spec (and the same
        // `AnalyserHookId::Catch`), so it is diagnosed and fixed identically.
        let src = "::catch {error oops}\n";
        assert_eq!(
            apply_w302_fix(src, 0, "tcl9.0"),
            "::catch {error oops} result\n"
        );
    }

    #[test]
    fn w302_offers_only_the_result_variable_under_tcl84() {
        // Tcl 8.4's `catch script ?varName?` has no options-dictionary
        // argument, so the second fix must not be offered there — the count
        // comes from the dialect-applicable synopsis, not from a constant.
        let mut a = Analyser::new();
        let r = a.analyse("catch {error oops}\n", "tcl8.4");
        let w302: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W302)
            .collect();
        assert_eq!(w302.len(), 1, "got {:?}", r.diagnostics);
        assert_eq!(
            w302[0].fixes.len(),
            1,
            "8.4 documents one optional word: {:?}",
            w302[0].fixes
        );
        assert_eq!(w302[0].fixes[0].new_text, " var");
    }

    #[test]
    fn w302_fixes_are_never_bulk_applicable() {
        // Capturing the result writes a new variable in the caller's frame,
        // which a program already using that name observes — so the fix is
        // hardening, not a semantics-preserving rewrite (issue #1195).
        let mut a = Analyser::new();
        let r = a.analyse("catch {error oops}\n", "tcl9.0");
        let w302 = r
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W302)
            .expect("W302");
        assert!(!w302.fixes.is_empty());
        assert!(
            w302.fixes.iter().all(|f| !f.safety.is_bulk_applicable()),
            "got {:?}",
            w302.fixes
        );
    }

    #[test]
    fn w302_carries_no_fix_for_a_malformed_catch() {
        // An unterminated body never reaches a well-formed insertion point;
        // whatever the recovery path emits, no W302 fix may point outside
        // the buffer.
        let mut a = Analyser::new();
        let src = "catch {error oops\n";
        let r = a.analyse(src, "tcl9.0");
        for diag in r.diagnostics.iter().filter(|d| d.code == DiagCode::W302) {
            for fix in &diag.fixes {
                assert!(
                    fix.span.end() as usize <= src.len(),
                    "fix span {:?} outside the buffer",
                    fix.span
                );
            }
        }
    }

    // -- W001 unknown-subcommand emitter

    #[test]
    fn analyse_emits_w001_for_unknown_string_subcommand() {
        let mut a = Analyser::new();
        let r = a.analyse("string bogus $x\n", "tcl");
        let w001: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W001)
            .collect();
        assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
        assert!(
            w001[0].message.contains("'bogus'") && w001[0].message.contains("'string'"),
            "got {:?}",
            w001[0].message
        );
        assert!(matches!(
            w001[0].severity,
            crate::analyser::Severity::Warning
        ));
    }

    #[test]
    fn analyse_w001_includes_did_you_mean_suggestion() {
        // ``string lenght`` — single-char typo for ``length``,
        // edit distance 2.  ``suggest_similar`` should surface
        // ``length``.
        let mut a = Analyser::new();
        let r = a.analyse("string lenght $x\n", "tcl");
        let w001: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W001)
            .collect();
        assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
        assert!(
            w001[0].message.contains("did you mean 'length'"),
            "got {:?}",
            w001[0].message
        );
        assert!(
            w001[0].fixes.iter().any(|f| f.new_text == "length"),
            "got {:?}",
            w001[0].fixes
        );
    }

    #[test]
    fn analyse_no_w001_for_known_subcommand() {
        // ``string length $x`` — known subcommand.  No W001.
        let mut a = Analyser::new();
        let r = a.analyse("string length $x\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_no_w001_for_dynamic_subcommand_position() {
        // ``string $sub $x`` — runtime-resolved subcommand;
        // can't statically check.  No W001.
        let mut a = Analyser::new();
        let r = a.analyse("string $sub $x\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_no_w001_for_command_substitution_in_subcommand_position() {
        // ``string [pick] $x`` — ``[…]`` in the subcommand
        // position is also a runtime-resolved value.  No W001.
        let mut a = Analyser::new();
        let r = a.analyse("string [pick] $x\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_no_w001_for_simple_command() {
        // ``set x 1`` — ``set`` has no SubcommandSig.  No W001.
        let mut a = Analyser::new();
        let r = a.analyse("set x 1\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_no_w001_for_unknown_command() {
        // ``unknownthing foo`` — registry doesn't know the
        // command, so no signature lookup, no W001.  (W123
        // owns the unknown-command diagnostic.)
        let mut a = Analyser::new();
        let r = a.analyse("unknownthing foo\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_w001_anchors_at_subcommand_token_only() {
        // The squiggle sits tightly on the offending word alone — not the
        // command name too — matching the "did you mean" fix's replacement
        // range and the KCS doc's documented "squiggle under the subcommand
        // token" behaviour.
        let mut a = Analyser::new();
        let src = "string bogus $x\n";
        let r = a.analyse(src, "tcl");
        let w001: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W001)
            .collect();
        assert_eq!(w001.len(), 1);
        let span = w001[0].span;
        let text = &src[span.start() as usize..span.end() as usize];
        assert_eq!(text, "bogus", "got {text:?}");
    }

    #[test]
    fn analyse_emits_w001_for_unknown_dict_subcommand() {
        // ``dict`` is also a SubcommandSig command — confirm
        // dispatch isn't ``string``-specific.
        let mut a = Analyser::new();
        let r = a.analyse("dict froob $d $k\n", "tcl");
        let w001: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W001)
            .collect();
        assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
        assert!(
            w001[0].message.contains("'dict'"),
            "got {:?}",
            w001[0].message
        );
    }

    #[test]
    fn analyse_w001_fix_replaces_wrapped_literal_subcommand() {
        // Wrapper tokens (``Str`` braced ``{lenght}`` / ``Esc``
        // quoted ``"lenght"``) carry the opening delimiter via
        // ``content_offset`` and the lexer span excludes the
        // closing delimiter; the W001 code-fix targets the
        // content range so the replacement preserves the
        // wrapping ``}`` / ``"`` rather than leaving a stray
        // trailing delimiter behind.
        for (src, expected) in [
            ("string {lenght} $x\n", "string {length} $x\n"),
            ("string \"lenght\" $x\n", "string \"length\" $x\n"),
            ("string lenght $x\n", "string length $x\n"),
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(src, "tcl");
            let w001: Vec<_> = r
                .diagnostics
                .iter()
                .filter(|d| d.code == DiagCode::W001)
                .collect();
            assert_eq!(w001.len(), 1, "src={src:?} got {:?}", r.diagnostics);
            assert!(
                w001[0].message.contains("did you mean 'length'"),
                "src={src:?} got {:?}",
                w001[0].message
            );

            let fix = w001[0]
                .fixes
                .iter()
                .find(|f| f.new_text == "length")
                .expect("expected replacement fix to 'length'");

            let mut fixed = src.to_string();
            let start = fix.span.start() as usize;
            let end = fix.span.end() as usize;
            fixed.replace_range(start..end, &fix.new_text);

            assert_eq!(fixed, expected, "src={src:?} fixes={:?}", w001[0].fixes);
            assert!(!fixed.contains("lenght"), "src={src:?} fixed={fixed:?}");
        }
    }

    #[test]
    fn analyse_w001_diagnostic_span_matches_fix_span() {
        // The squiggle and the "did you mean" quick-fix must target the
        // identical range — otherwise accepting the fix visibly leaves part
        // of the squiggled text unchanged.
        let mut a = Analyser::new();
        let src = "string lenght $x\n";
        let r = a.analyse(src, "tcl");
        let w001: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W001)
            .collect();
        assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
        let fix = w001[0]
            .fixes
            .first()
            .expect("expected a 'did you mean' fix");
        assert_eq!(w001[0].span, fix.span, "got {:?}", w001[0]);
    }

    #[test]
    fn analyse_w001_no_diagnostic_at_all_when_shadowed_by_proc() {
        // A same-file `proc string {...}` must suppress W001 (and its
        // subcommand-level W002 sibling) completely — not just avoid the
        // specific message text. The FP-STY-17 reproducers live in
        // `analyser/diagnostics/fp/sty.rs::fp_sty_17_*`.
        let mut a = Analyser::new();
        let src = "proc string {op args} { return $op }\nstring reverse hello\n";
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| matches!(d.code, DiagCode::W001 | DiagCode::W002)),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_w001_shadow_suppression_is_scoped_to_the_shadowed_name() {
        // Shadowing `string` must not suppress W001 for a genuinely unknown
        // subcommand on a different, unshadowed ensemble in the same file.
        let mut a = Analyser::new();
        let src = "proc string {op args} { return $op }\ninfo bogus\n";
        let r = a.analyse(src, "tcl");
        let w001: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W001)
            .collect();
        assert_eq!(w001.len(), 1, "got {:?}", r.diagnostics);
        assert!(w001[0].message.contains("'info'"), "got {:?}", w001[0]);
    }

    #[test]
    fn analyse_no_w001_for_expanded_literal_subcommand() {
        // `{*}{create a b}` splices the elements `create`, `a`, `b` into the
        // argument list (confirmed against tclsh 8.6.14) — the raw source
        // text "create a b" must never be compared against the subcommand
        // set. The FP-STY-18 reproducers live in
        // `analyser/diagnostics/fp/sty.rs::fp_sty_18_*`.
        let mut a = Analyser::new();
        let r = a.analyse("dict {*}{create a b}\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_w001_still_fires_for_unexpanded_unknown_subcommand() {
        // TP control for the `{*}`-expansion skip: the identical subcommand
        // set, without expansion, still fires on a genuine typo.
        let mut a = Analyser::new();
        let r = a.analyse("dict bogus a b\n", "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == DiagCode::W001),
            "got {:?}",
            r.diagnostics
        );
    }

    // -- E004 malformed-if emitter
    //
    // Fires ``Severity::Error`` when an ``if`` invocation's structural
    // shape doesn't match
    // ``if COND BODY ?elseif COND BODY ...? ?else BODY?``.  Detection
    // reads `tcl_registry::commands::tcl::if_::check_if_shape` via the
    // spec's `clause_shape_check` hook — the grammar itself is not
    // reimplemented here (see `emit_e004_clause_shape_diagnostic`).
    // Every case is cross-checked against tclsh 8.6 and Tcl 9.0.4's
    // `TclNRIfObjCmd` source; see the truth table in
    // `tcl-registry/src/commands/tcl/if_.rs`'s own tests for the
    // grammar-level cases this integration layer doesn't repeat.

    fn e004_diags(src: &str) -> Vec<crate::analyser::types::Diagnostic> {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        r.diagnostics
            .into_iter()
            .filter(|d| d.code == DiagCode::E004)
            .collect()
    }

    fn span_text(src: &str, span: tcl_lexer::Span) -> &str {
        &src[span.start() as usize..span.end() as usize]
    }

    // -- TP: genuinely malformed shapes, with the precise message and a
    // tight span (not the whole `if` statement).

    #[test]
    fn tp_bare_if_names_the_invoked_command_in_the_message() {
        // ``if`` alone — no expression at all.  Message and span both
        // name "if" itself (real Tcl: `no expression after "if"
        // argument`), and this must be the *only* diagnostic for the
        // line — no redundant generic arity error alongside it (see
        // `no_duplicate_e002_alongside_e004`).
        let src = "if\n";
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "got {e004:?}");
        assert_eq!(e004[0].message, "No expression after \"if\" argument");
        assert_eq!(span_text(src, e004[0].span), "if");
    }

    #[test]
    fn tp_condition_without_body_names_the_condition_text() {
        // ``if {1}`` — real Tcl: `no script following "1" argument`.
        let src = "if {1}\n";
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "got {e004:?}");
        assert_eq!(e004[0].message, "No script following \"1\" argument");
        assert_eq!(span_text(src, e004[0].span), "{1}");
    }

    #[test]
    fn tp_then_keyword_without_body_names_then() {
        // ``if {1} then`` — real Tcl: `no script following "then"
        // argument`.
        let src = "if {1} then\n";
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "got {e004:?}");
        assert_eq!(e004[0].message, "No script following \"then\" argument");
        assert_eq!(span_text(src, e004[0].span), "then");
    }

    #[test]
    fn tp_bare_else_without_body_names_else() {
        // ``if {1} { a } else`` — real Tcl: `no script following
        // "else" argument`.
        let src = "if {1} { a } else\n";
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "got {e004:?}");
        assert_eq!(e004[0].message, "No script following \"else\" argument");
        assert_eq!(span_text(src, e004[0].span), "else");
    }

    #[test]
    fn tp_elseif_without_expr_names_elseif() {
        // ``if {1} { a } elseif`` — real Tcl: `no expression after
        // "elseif" argument`.
        let src = "if {1} { a } elseif\n";
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "got {e004:?}");
        assert_eq!(e004[0].message, "No expression after \"elseif\" argument");
        assert_eq!(span_text(src, e004[0].span), "elseif");
    }

    #[test]
    fn tp_elseif_condition_without_body_names_the_condition_text() {
        // ``if {1} { a } elseif {2}`` — real Tcl: `no script
        // following "2" argument`.
        let src = "if {1} { a } elseif {2}\n";
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "got {e004:?}");
        assert_eq!(e004[0].message, "No script following \"2\" argument");
        assert_eq!(span_text(src, e004[0].span), "{2}");
    }

    #[test]
    fn tp_extra_words_after_explicit_else_anchors_only_the_extra_words() {
        // ``if {1} { a } else { b } extra`` — the span covers just
        // "extra", not the whole statement.
        let src = "if {1} { a } else { b } extra\n";
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "got {e004:?}");
        assert_eq!(
            e004[0].message,
            "Extra words after \"else\" clause in \"if\" command"
        );
        assert_eq!(span_text(src, e004[0].span), "extra");
        assert!(matches!(e004[0].severity, crate::analyser::Severity::Error));
    }

    #[test]
    fn tp_extra_words_after_implicit_else_anchors_only_the_extra_word() {
        // ``if {1} { a } { b } { c }`` — implicit else (no ``else``
        // keyword); "c" is the first extra word.
        let src = "if {1} { a } { b } { c }\n";
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "got {e004:?}");
        assert_eq!(span_text(src, e004[0].span), "{ c }");
    }

    #[test]
    fn tp_qualified_double_colon_if_is_checked_too() {
        // ``::if`` names the same global command as ``if`` — registry
        // name resolution strips the leading ``::`` for every command,
        // so the dispatch-generic hook lookup picks this up without any
        // `if`-specific handling in the compiler.
        let src = "::if {1} { a } { b } { c }\n";
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "got {e004:?}");
    }

    // -- FP fixed: a leading `else`/`elseif` is a well-formed `if` whose
    // condition happens to be that bareword — not a malformed `if`.
    // Verified against tclsh 8.6: both raise `invalid bareword "else"` /
    // `"elseif"` at *expression*-evaluation time, never a `wrong # args`
    // structural error.

    #[test]
    fn fp_leading_else_is_not_malformed() {
        let r_diags = e004_diags("if else { x }\n");
        assert!(r_diags.is_empty(), "got {r_diags:?}");
    }

    #[test]
    fn fp_leading_elseif_is_not_malformed() {
        let r_diags = e004_diags("if elseif { x }\n");
        assert!(r_diags.is_empty(), "got {r_diags:?}");
    }

    #[test]
    fn fp_else_in_elseif_condition_slot_is_not_malformed() {
        // ``if {1} { a } elseif else { b }`` — "else" sits in the
        // *elseif's* condition slot, never keyword-matched there
        // either (verified against tclsh 8.6: runs body "a", not a
        // wrong-#args error).
        let r_diags = e004_diags("if {1} { a } elseif else { b }\n");
        assert!(r_diags.is_empty(), "got {r_diags:?}");
    }

    // -- TN: well-formed shapes never flagged.

    #[test]
    fn tn_single_clause_if() {
        assert!(e004_diags("if {1} { a }\n").is_empty());
    }

    #[test]
    fn tn_if_else() {
        assert!(e004_diags("if {1} { a } else { b }\n").is_empty());
    }

    #[test]
    fn tn_if_elseif_else_chain() {
        assert!(e004_diags("if {$a} { x } elseif {$b} { y } else { z }\n").is_empty());
    }

    #[test]
    fn tn_if_with_then_keyword() {
        assert!(e004_diags("if {1} then { a }\n").is_empty());
    }

    #[test]
    fn tn_implicit_else_single_body() {
        // ``if {1} { a } { b }`` — one implicit-else body, no keyword.
        assert!(e004_diags("if {1} { a } { b }\n").is_empty());
    }

    #[test]
    fn tn_inside_tcloo_method_body() {
        // `if` structural checking is generic body-walking — it must
        // fire the same way inside a TclOO method body as at top level.
        let src = "oo::class create C {\n  method m {} {\n    if {1} { a } { b } { c }\n  }\n}\n";
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "got {e004:?}");
    }

    // -- FN (documented, intentional scope boundary): a renamed `if` is
    // not checked. `if`'s registry `clause_shape_check` hook is looked
    // up by resolving `cmd_name` as written — namespace-qualification
    // (`::if`) resolves through it (see
    // `tp_qualified_double_colon_if_is_checked_too`), but a `rename if
    // myif` target does not, since that requires chasing the same
    // same-file rename/alias graph the arity checker's
    // `resolve_indirect_call_target` uses — a distinct, heavier
    // mechanism not wired up to this dispatch-site check. Real Tcl
    // *does* validate a renamed `if`'s shape (the C source's own doc
    // comment: `Tcl_IfObjCmd` runs for "if" or "the name to which if was
    // renamed"), so this is a genuine, narrow gap — not a false
    // negative this test is happy about, just one it pins so a future
    // fix shows up as an intentional behaviour change.
    #[test]
    fn fn_renamed_if_is_not_currently_checked() {
        let src = "rename if myif\nmyif {1} { a } { b } { c }\n";
        let e004 = e004_diags(src);
        assert!(e004.is_empty(), "got {e004:?}");
    }

    // -- Redundant-diagnostic fix: `if`'s registry `arity` floor no
    // longer produces a second, generic E002 alongside the precise
    // E004 for the same defect.

    #[test]
    fn no_duplicate_e002_alongside_e004() {
        for src in ["if\n", "if {1}\n"] {
            let mut a = Analyser::new();
            let r = a.analyse(src, "tcl");
            assert!(
                !r.diagnostics.iter().any(|d| d.code == DiagCode::E002),
                "src={src:?} got {:?}",
                r.diagnostics
            );
            assert!(
                r.diagnostics.iter().any(|d| d.code == DiagCode::E004),
                "src={src:?} got {:?}",
                r.diagnostics
            );
        }
    }

    // -- Code fixes.

    fn apply_fix(src: &str, fix: &crate::analyser::types::CodeFix) -> String {
        let mut fixed = src.to_string();
        fixed.replace_range(
            fix.span.start() as usize..fix.span.end() as usize,
            &fix.new_text,
        );
        fixed
    }

    #[test]
    fn fix_merges_extra_words_into_the_final_body() {
        let src = "if {1} { a } { b } { c }";
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "got {e004:?}");
        assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
        let fixed = apply_fix(src, &e004[0].fixes[0]);
        assert_eq!(fixed, "if {1} { a } {{ b } { c }}");
        // The fixed source must itself be shape-well-formed.
        assert!(e004_diags(&fixed).is_empty(), "fixed={fixed:?}");
    }

    #[test]
    fn fix_merges_extra_words_after_explicit_else() {
        let src = "if {1} { a } else { b } extra";
        let e004 = e004_diags(src);
        assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
        let fixed = apply_fix(src, &e004[0].fixes[0]);
        assert_eq!(fixed, "if {1} { a } else {{ b } extra}");
        assert!(e004_diags(&fixed).is_empty(), "fixed={fixed:?}");
    }

    #[test]
    fn fix_removes_dangling_elseif_clause() {
        let src = "if {1} { a } elseif";
        let e004 = e004_diags(src);
        assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
        assert_eq!(e004[0].fixes[0].new_text, "");
        let fixed = apply_fix(src, &e004[0].fixes[0]);
        assert_eq!(fixed, "if {1} { a } ");
        assert!(e004_diags(&fixed).is_empty(), "fixed={fixed:?}");
    }

    #[test]
    fn fix_removes_dangling_else_with_no_body() {
        let src = "if {1} { a } else";
        let e004 = e004_diags(src);
        assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
        let fixed = apply_fix(src, &e004[0].fixes[0]);
        assert_eq!(fixed, "if {1} { a } ");
        assert!(e004_diags(&fixed).is_empty(), "fixed={fixed:?}");
    }

    #[test]
    fn fix_removes_dangling_elseif_condition_without_body() {
        let src = "if {1} { a } elseif {2}";
        let e004 = e004_diags(src);
        assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
        let fixed = apply_fix(src, &e004[0].fixes[0]);
        assert_eq!(fixed, "if {1} { a } ");
        assert!(e004_diags(&fixed).is_empty(), "fixed={fixed:?}");
    }

    #[test]
    fn no_fix_offered_when_the_first_clause_never_completed() {
        // ``if`` / ``if {1}`` — there is no well-formed prefix to fall
        // back to, so no fix is offered (never a guessed body).
        for src in ["if", "if {1}", "if {1} then"] {
            let e004 = e004_diags(src);
            assert_eq!(e004.len(), 1, "src={src:?}");
            assert!(
                e004[0].fixes.is_empty(),
                "src={src:?} got {:?}",
                e004[0].fixes
            );
        }
    }

    // -- W304 missing-option-terminator emitter
    //
    // Resolution profile lives in ``tcl-registry``; tristate severity
    // / two-diagnostic origin / code-fix logic lives in
    // ``analyser/diagnostics.rs``.

    fn w304_diags(src: &str) -> Vec<crate::analyser::types::Diagnostic> {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        r.diagnostics
            .into_iter()
            .filter(|d| d.code == DiagCode::W304)
            .collect()
    }

    #[test]
    fn analyse_emits_w304_for_regexp_pattern_variable() {
        let diags = w304_diags("regexp $pattern $text\n");
        assert_eq!(diags.len(), 1, "got {diags:?}");
        assert!(
            diags[0].message.to_lowercase().contains("option-injection"),
            "got {:?}",
            diags[0].message
        );
        assert!(matches!(
            diags[0].severity,
            crate::analyser::Severity::Suggestion
        ));
    }

    #[test]
    fn analyse_no_w304_for_regexp_safe_literal_pattern() {
        // Pattern starts with `(` — non-dynamic, doesn't start with
        // `-`, so the OFF gate suppresses regardless of command.
        let diags = w304_diags("regexp {(a+)+$} $text\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_emits_w304_for_regexp_literal_dash_pattern() {
        // ``regexp {-[0-9]+} $text`` — the literal pattern starts
        // with `-` so the positional scanner treats it as an
        // unknown option and lands on the next positional
        // (``$text``).  The diagnostic still fires; severity comes
        // from the dynamic-var INFO path because the diag anchors on
        // ``$text``, not the pattern literal.
        let diags = w304_diags("regexp {-[0-9]+} $text\n");
        assert_eq!(diags.len(), 1, "got {diags:?}");
    }

    #[test]
    fn analyse_no_w304_for_exec_literal_dash_after_first_positional() {
        // ``exec foo -bad`` — ``first_positional_without_terminator``
        // treats ``foo`` (index 0) as the first positional, so the
        // OFF gate suppresses W304 there (non-dynamic, doesn't start
        // with ``-``).  The later literal ``-bad`` is not
        // re-considered as a candidate "first positional" argument,
        // so no diagnostic fires.
        //
        // This pins the scanner / first-positional behaviour for
        // ``exec`` rather than exercising the literal-dash WARN
        // branch.  The WARN branch is covered by
        // `analyse_emits_w304_for_regexp_literal_dash_pattern`
        // (literal pattern starting with `-`) and
        // `analyse_w304_constant_propagation_dash_value_warns`
        // (variable resolved via constant-prop to a `-`-prefixed
        // value).
        let diags = w304_diags("exec foo -bad\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_no_w304_for_regexp_with_terminator() {
        let diags = w304_diags("regexp -- $pattern $text\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_emits_w304_for_regexp_with_option_value_then_variable() {
        // ``-start`` consumes the next arg as its value; the first
        // positional after it is the pattern variable.
        let diags = w304_diags("regexp -start 0 $pattern $text\n");
        assert_eq!(diags.len(), 1, "got {diags:?}");
    }

    #[test]
    fn analyse_emits_w304_for_regsub_variable() {
        let diags = w304_diags("regsub $pattern $text X out\n");
        assert_eq!(diags.len(), 1, "got {diags:?}");
    }

    #[test]
    fn analyse_no_w304_for_subst() {
        // ``subst`` does not declare a ``--`` option — registry-
        // level filter suppresses W304 entirely.
        let diags = w304_diags("subst $template\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_emits_w304_for_exec_variable() {
        let diags = w304_diags("exec $cmd\n");
        assert_eq!(diags.len(), 1, "got {diags:?}");
    }

    #[test]
    fn analyse_no_w304_for_exec_with_terminator() {
        let diags = w304_diags("exec -- $cmd\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_no_w304_for_glob_safe_literal() {
        // ``*.tcl`` does not start with `-`; OFF gate suppresses.
        let diags = w304_diags("glob *.tcl\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_no_w304_for_string_match() {
        // ``string match`` does not support ``--`` — registry filter.
        let diags = w304_diags("string match $pattern $value\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_no_w304_for_lsearch() {
        // ``lsearch`` does not declare ``--`` either.
        let diags = w304_diags("lsearch -exact $domain c\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_emits_w304_for_file_delete_variable() {
        // ``file delete`` is subcommand-scoped — profile.scan_start
        // == 1 to skip the ``delete`` keyword.
        let diags = w304_diags("file delete $path\n");
        assert_eq!(diags.len(), 1, "got {diags:?}");
    }

    #[test]
    fn analyse_no_w304_for_file_delete_with_terminator() {
        let diags = w304_diags("file delete -- $path\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_emits_w304_for_load_variable() {
        let diags = w304_diags("load $fileName\n");
        assert_eq!(diags.len(), 1, "got {diags:?}");
    }

    #[test]
    fn analyse_w304_constant_propagation_emits_info_with_origin() {
        // ``set X "datagroup"; exec $X`` — the variable resolves to a
        // literal value that doesn't start with `-`, so severity drops
        // to INFO and a second "origin" diagnostic anchors at the
        // literal's range.  (The two-arg braced ``switch $X { ... }``
        // form is exempt from W304 entirely — FP-NAB-05 — so an
        // option-bearing command that still flags is used here.)
        let src = "set totp_key_storage \"datagroup\"\n\
                   exec $totp_key_storage\n";
        let diags = w304_diags(src);
        assert_eq!(diags.len(), 2, "got {diags:?}");

        let main = diags
            .iter()
            .find(|d| !d.fixes.is_empty())
            .expect("main diag");
        let origin = diags
            .iter()
            .find(|d| d.fixes.is_empty())
            .expect("origin diag");

        assert!(
            matches!(main.severity, crate::analyser::Severity::Suggestion),
            "main severity {:?}",
            main.severity
        );
        assert!(
            main.message.contains("totp_key_storage") && main.message.contains("datagroup"),
            "main message {:?}",
            main.message
        );

        let highlighted = &src[main.span.start() as usize..main.span.end() as usize];
        assert_eq!(highlighted, "$totp_key_storage", "got {highlighted:?}");

        // The origin diag points at the ``"datagroup"`` literal in
        // the preceding ``set``.
        let origin_text = &src[origin.span.start() as usize..origin.span.end() as usize];
        assert!(
            origin_text.contains("datagroup"),
            "origin span text {origin_text:?}"
        );
    }

    #[test]
    fn analyse_w304_constant_propagation_dash_value_warns() {
        // The variable resolves to ``-something`` — escalates to
        // WARNING.
        let src = "set evil \"-rf\"\n\
                   exec $evil /\n";
        let diags = w304_diags(src);
        assert!(!diags.is_empty(), "got {diags:?}");
        let main = diags
            .iter()
            .find(|d| !d.fixes.is_empty())
            .expect("main diag");
        assert!(matches!(main.severity, crate::analyser::Severity::Warning));
    }

    // -- W101 eval-string-concat emitter
    //
    // Canonical-list-idiom suppression lives in the registry
    // (``is_canonical_list_command``); substitution-detection
    // approximation lives in the analyser.

    fn w101_diags(src: &str) -> Vec<crate::analyser::types::Diagnostic> {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        r.diagnostics
            .into_iter()
            .filter(|d| d.code == DiagCode::W101)
            .collect()
    }

    #[test]
    fn analyse_emits_w101_for_eval_with_variable() {
        let diags = w101_diags("eval \"puts $x\"\n");
        assert_eq!(diags.len(), 1, "got {diags:?}");
        assert!(
            diags[0].message.to_lowercase().contains("injection"),
            "got {:?}",
            diags[0].message
        );
        assert!(matches!(
            diags[0].severity,
            crate::analyser::Severity::Warning
        ));
    }

    #[test]
    fn analyse_no_w101_for_eval_braced_script() {
        let diags = w101_diags("eval {puts hello}\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_no_w101_for_eval_multiple_braced() {
        let diags = w101_diags("eval {set x 1} {puts $x}\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_emits_w101_for_eval_with_command_subst() {
        // ``eval [build_cmd]`` — single CMD token, but `build_cmd`
        // isn't a canonical-list-producing command.
        let diags = w101_diags("eval [build_cmd]\n");
        assert_eq!(diags.len(), 1, "got {diags:?}");
    }

    #[test]
    fn analyse_no_w101_for_eval_literal_no_substitution() {
        // ``eval puts hello`` — both args are bare literals; no
        // substitution at any level.
        let diags = w101_diags("eval puts hello\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_no_w101_for_eval_list_idiom() {
        // ``eval [list ...]`` — ``list`` produces a canonical list,
        // safe re-parse.
        let diags = w101_diags("eval [list set $varname $value]\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_no_w101_for_eval_linsert_idiom() {
        // ``linsert`` returns TclType::List → canonical.
        let diags = w101_diags("eval [linsert $cmdlist 0 extraarg]\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_no_w101_for_eval_split_idiom() {
        let diags = w101_diags("eval [split $line :]\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_emits_w101_for_eval_concat_idiom() {
        // ``concat`` is the explicit non-canonical exclusion —
        // strips one level of grouping, not safe for re-parse.
        let diags = w101_diags("eval [concat $script $args]\n");
        assert_eq!(diags.len(), 1, "got {diags:?}");
    }

    #[test]
    fn analyse_no_w101_for_non_eval_commands() {
        // The emitter is gated on ``cmd_name == "eval"`` — other
        // substitution-bearing commands are out of scope (W301
        // covers uplevel; W312 covers interp eval).
        let diags = w101_diags("uplevel 1 \"puts $x\"\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_w101_anchors_at_first_arg_token() {
        let src = "eval \"puts $x\"\n";
        let diags = w101_diags(src);
        assert_eq!(diags.len(), 1);
        let span = diags[0].span;
        let text = &src[span.start() as usize..span.end() as usize];
        // First arg is the quoted string ``"puts $x"`` — the
        // representative token's span anchors the diagnostic.
        assert!(text.contains("puts") || text.contains("$x"), "got {text:?}");
    }

    #[test]
    fn analyse_w101_rejects_multi_command_subscript() {
        // ``[list a; set x $user]`` — multi-command script can't be
        // proven safe (last command's result wins, and that's
        // ``set``, not ``list``).
        let diags = w101_diags("eval [list a\\; set x $user]\n");
        assert_eq!(diags.len(), 1, "got {diags:?}");
    }

    #[test]
    fn analyse_no_w101_for_eval_literal_multi_token_word() {
        // ``eval foo{bar}`` is a multi-token word (Esc + Str joined,
        // ``single_token_word == false``) that contains no Var/Cmd
        // substitution — W101 only fires on actual VAR/CMD tokens, so
        // this must not trigger it.  The check is a brace/backslash-aware
        // source-byte scan that looks for unescaped ``$`` / ``[``
        // outside ``{...}``.
        let diags = w101_diags("eval foo{bar}\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_no_w101_for_eval_backslash_escaped_dollar() {
        // ``eval "no\$x"`` — the ``\$`` is a backslash-escape, so
        // the lexer produces a single ESC token with no Var.  The
        // word-span scan must skip the next byte after ``\`` to
        // avoid mis-detecting the literal ``$``.
        let diags = w101_diags("eval no\\$x\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_w304_code_fix_inserts_terminator() {
        let src = "exec $cmd\n";
        let diags = w304_diags(src);
        assert_eq!(diags.len(), 1, "got {diags:?}");
        let fix = diags[0]
            .fixes
            .first()
            .expect("expected an insert-terminator fix");
        let mut applied = src.to_string();
        let start = fix.span.start() as usize;
        let end = fix.span.end() as usize;
        applied.replace_range(start..end, &fix.new_text);
        assert_eq!(applied, "exec -- $cmd\n", "got {applied:?}");
    }

    // -- body-walk and nested-cmd recovery
    //
    // Verify ``when EVENT { body }`` recurses (the registry
    // ``arg_role_resolver`` records BODY at the last index)
    // and that braced expr args (``Str`` tokens) have their
    // outer braces unwrapped before the nested-``[cmd]`` scan
    // (otherwise the scanner skips the entire braced region
    // opaquely).

    #[test]
    fn compound_cmd_word_descends_substitution_fragments_only() {
        // A command substitution that is the *first* fragment of a
        // compound word (`[foo]bar`, `[foo]$x`) merges with the trailing
        // literal into one argv token spanning the whole word.  The
        // nested-invocation recorder must descend only the `[…]`
        // fragment(s) — not the merged span — so it records the real
        // inner head (`foo`) rather than a bogus one (`foo]bar`).
        type Inv = (&'static str, u32, u32);
        let cases: &[(&str, &[Inv])] = &[
            ("puts [foo]bar\n", &[("foo", 6, 9)]),
            ("set x [foo]$suffix\n", &[("foo", 7, 10)]),
            // Substitution as the command head of a compound word.
            ("[foo]bar hi\n", &[("foo", 1, 4)]),
            // Two substitutions in one word — both must be found.
            ("puts [foo]bar[baz]\n", &[("foo", 6, 9), ("baz", 14, 17)]),
            // `;`-separated commands inside a compound substitution.
            ("puts [aa; bb]cc\n", &[("aa", 6, 8), ("bb", 10, 12)]),
        ];
        for (src, expected) in cases {
            let mut a = Analyser::new();
            let r = a.analyse(src, "f5-irules");
            let got: Vec<(String, u32, u32)> = r
                .command_invocations
                .iter()
                .map(|c| (c.name.clone(), c.range.start(), c.range.end()))
                .collect();
            for &(name, start, end) in *expected {
                assert!(
                    got.iter()
                        .any(|(n, s, e)| n == name && *s == start && *e == end),
                    "src {src:?}: expected ({name:?}, {start}, {end}) in {got:?}",
                );
            }
            // No over-read: a *descended* substitution head must never
            // carry the literal suffix after the matching `]` (`foo]bar`,
            // `foo]${suffix}`).  A whole-word head recorded verbatim by
            // `process_command` (`[foo]bar`) legitimately starts with `[`
            // and is excluded.
            assert!(
                !got.iter()
                    .any(|(n, _, _)| n.contains(']') && !n.starts_with('[')),
                "src {src:?}: a descended head leaked the literal suffix (over-read): {got:?}",
            );
        }
    }

    #[test]
    fn analyse_when_body_records_inner_command_invocations() {
        // ``when HTTP_REQUEST { body }`` — ``call`` and the
        // target ``myhelper`` should appear in
        // ``command_invocations`` from the body recursion.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc myhelper {} {}\nwhen HTTP_REQUEST { call myhelper }\n",
            "f5-irules",
        );
        let names: Vec<&str> = r
            .command_invocations
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(names.contains(&"call"), "got {names:?}");
        assert!(names.contains(&"myhelper"), "got {names:?}");
    }

    #[test]
    fn analyse_braced_expr_arg_records_inner_substitution() {
        // ``if { [HTTP::uri] eq "/foo" } { ... }`` — the
        // ``[HTTP::uri]`` substitution inside the braced expr
        // arg must surface in ``command_invocations``.  Without
        // the ``Str`` unwrap, the nested-cmd scanner sees the
        // outer ``{`` and skips the entire braced region
        // opaquely.
        let mut a = Analyser::new();
        let r = a.analyse("if { [HTTP::uri] eq \"/foo\" } { puts ok }\n", "f5-irules");
        let names: Vec<&str> = r
            .command_invocations
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(names.contains(&"HTTP::uri"), "got {names:?}");
    }

    #[test]
    fn analyse_records_every_command_in_a_substitution() {
        // A command substitution can hold more than one command (`;`- /
        // newline-separated), and substitutions nest.  The CST descent
        // records every inner command head (`set x [foo; bar]` ->
        // {foo, bar, set}).
        // A head is recorded in `word_piece` form for every command,
        // incl. a `$var` head (`${var}`), a `"quoted"` head (unquoted),
        // and a compound head (`set x [$cmd a; bar]` -> {${cmd}, bar,
        // set}).
        let cases: &[(&str, &[&str])] = &[
            ("set x [foo; bar]\n", &["set", "foo", "bar"]),
            ("puts [a; b; c]\n", &["puts", "a", "b", "c"]),
            ("set x [foo [bar; baz]]\n", &["set", "foo", "bar", "baz"]),
            ("set x [foo $y; bar]\n", &["set", "foo", "bar"]),
            ("set x [$cmd a; bar]\n", &["set", "${cmd}", "bar"]),
            ("set x [foo; $y arg]\n", &["set", "foo", "${y}"]),
            ("set x [\"q\" a; bar]\n", &["set", "q", "bar"]),
            // A control-flow command inside a substitution has body
            // arguments whose commands are inner invocations too
            // (descend_command resolves them via the registry).
            // `[if {$c} {puts hi}]` -> {if, puts}.
            ("set x [if {$c} {puts hi}]\n", &["set", "if", "puts"]),
            (
                "set x [foreach a $l {log $a}]\n",
                &["set", "foreach", "log"],
            ),
            ("set x [eval {one; two}]\n", &["set", "eval", "one", "two"]),
            // A command substitution inside an *expr* argument of a
            // command nested in a substitution is an invocation too
            // (collect_expr_substitutions). `[if {[check]} {fwd}]`
            // -> {if, check, fwd}; `[expr {[bar] + 1}]` -> {expr, bar}.
            (
                "set x [if {[check]} {fwd}]\n",
                &["set", "if", "check", "fwd"],
            ),
            ("set x [expr {[bar] + 1}]\n", &["set", "expr", "bar"]),
            (
                "set x [while {[cond]} {act}]\n",
                &["set", "while", "cond", "act"],
            ),
            // argv_texts[0] is recorded for every command, incl. a
            // `[subst]` head (recorded *and* descended) and a `{braced}`
            // head (its inner text).
            ("set x [[gen] arg]\n", &["set", "[gen]", "gen"]),
            ("puts [[a] [b]]\n", &["puts", "[a]", "a", "b"]),
        ];
        for (src, expected) in cases {
            let mut a = Analyser::new();
            let r = a.analyse(src, "tcl8.6");
            let names: Vec<&str> = r
                .command_invocations
                .iter()
                .map(|c| c.name.as_str())
                .collect();
            for want in *expected {
                assert!(
                    names.contains(want),
                    "{src:?}: missing {want:?}, got {names:?}"
                );
            }
        }
    }

    #[test]
    fn analyse_records_switch_arm_bodies_not_patterns() {
        // The `switch … {pat body …}` list-form arg is a Tcl *list*,
        // not a script.  The arm *bodies* are scripts (their commands
        // are invocations), but the *patterns* are not — descending
        // the whole list as a script would mis-record a pattern
        // (`a`/`b`) as a command head.  Parse the pairs and descend
        // each body.  A `default` keyword / `-` fall-through is a
        // pattern, not a body.  Result: {cmd1, cmd2, set, switch}.
        let mut a = Analyser::new();
        let r = a.analyse("set x [switch $v {a {cmd1} b {cmd2}}]\n", "tcl8.6");
        let names: Vec<&str> = r
            .command_invocations
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(names.contains(&"switch"), "got {names:?}");
        assert!(names.contains(&"cmd1"), "arm body missing: {names:?}");
        assert!(names.contains(&"cmd2"), "arm body missing: {names:?}");
        assert!(
            !names.contains(&"a"),
            "pattern recorded as command: {names:?}"
        );
        assert!(
            !names.contains(&"b"),
            "pattern recorded as command: {names:?}"
        );
    }

    #[test]
    fn analyse_braced_data_word_is_not_over_recorded() {
        // A `[...]` inside a braced *data* word is literal (braces
        // suppress substitution), so it must not be recorded — only an
        // *expr* arg's substitutions are.  And a command whose name is
        // itself a substitution (`[x] hi`) must still be descended (the
        // head token is iterated too).
        let cases: &[(&str, &[&str], &[&str])] = &[
            // (source, must-contain, must-NOT-contain)
            ("set x {[noeval]}\n", &["set"], &["noeval"]),
            ("set d {literal data}\n", &["set"], &["data", "literal"]),
            ("proc p {} {[x] hi}\n", &["proc", "x"], &[]),
            ("if {[chk]} {puts ok}\n", &["if", "chk", "puts"], &[]),
        ];
        for (src, want, unwant) in cases {
            let mut a = Analyser::new();
            let r = a.analyse(src, "tcl8.6");
            let names: Vec<&str> = r
                .command_invocations
                .iter()
                .map(|c| c.name.as_str())
                .collect();
            for w in *want {
                assert!(names.contains(w), "{src:?}: missing {w:?}, got {names:?}");
            }
            for u in *unwant {
                assert!(
                    !names.contains(u),
                    "{src:?}: over-recorded {u:?}, got {names:?}"
                );
            }
        }
    }

    #[test]
    fn analyse_regexp_var_added_to_regex_vars_set() {
        // Side effect: the var name is recorded in
        // ``regex_vars`` so downstream consumers (var-as-regex
        // hint, future W*-codes) can find the defining set.
        let mut a = Analyser::new();
        let _ = a.analyse("set p {^foo}\nregexp $p $line\n", "tcl");
        // The const-string scope is the global scope (path = []).
        assert!(a.regex_vars.contains(&(Vec::new(), "p".to_string())));
    }

    #[test]
    fn analyse_alias_with_prepended_args_recorded() {
        // Prepended args after the target are stored on
        // ``extras``.
        let mut a = Analyser::new();
        let r = a.analyse("interp alias {} logerr {} puts stderr\n", "tcl");
        let alias = r
            .command_aliases
            .get("::logerr")
            .expect("::logerr recorded");
        assert_eq!(alias.target, "puts");
        assert_eq!(alias.extras.as_slice(), &["stderr"]);
    }

    #[test]
    fn analyse_tcllib_import_wrapper_does_not_fire_on_namespace_import() {
        // The wrapper detector must not trip on Tcl's own
        // ``namespace import`` — that's handled by
        // ``handle_namespace_import_command`` and is never
        // conjectured.
        let mut a = Analyser::new();
        let r = a.analyse("namespace import ::foo::bar\n", "tcl");
        assert!(r.namespace_imports.iter().all(|i| !i.conjectured));
    }

    #[test]
    fn analyse_chunked_seeds_file_suppression_minus_one_sentinel() {
        // ``analyse`` populates ``result.suppressed_lines[-1]`` with
        // the file-level ``# tcl-lsp: disable=`` set; verify
        // ``analyse_chunked`` does the same so consumers see the
        // file-wide directives via the same surface regardless of
        // which entry point dispatched.
        use crate::segmenter::SegmentedCommand;
        let mut a = Analyser::new();
        let cmds: Vec<Vec<SegmentedCommand>> = vec![Vec::new()];
        let (r, _) = a.analyse_chunked("# tcl-lsp: disable=W210,W211\nset x 1\n", cmds, "tcl");
        let codes = r.suppressed_lines.get(&-1).expect("-1 sentinel");
        assert!(codes.contains("W210"));
        assert!(codes.contains("W211"));
    }

    #[test]
    fn analyse_commands_seeds_file_suppression_minus_one_sentinel() {
        // Same assertion through ``analyse_commands`` — the
        // snapshot-restore entry point.
        use crate::segmenter::SegmentedCommand;
        let mut a = Analyser::new();
        let cmds: Vec<SegmentedCommand> = Vec::new();
        let r = a.analyse_commands(
            "# tcl-lsp: disable=W210,W211\nset x 1\n",
            &cmds,
            "tcl",
            true,
        );
        let codes = r.suppressed_lines.get(&-1).expect("-1 sentinel");
        assert!(codes.contains("W210"));
        assert!(codes.contains("W211"));
    }

    // Incremental analysis differential oracle

    /// A projection of `AnalysisResult` capturing the observable
    /// identity an incremental analysis must preserve: sorted
    /// `(code, start, end)` diagnostics, proc names, global vars, and
    /// `(invocation name, start)` pairs.
    type ResultProjection = (
        Vec<(String, u32, u32)>,
        Vec<String>,
        Vec<String>,
        Vec<(String, u32)>,
    );

    fn project_result(r: &AnalysisResult) -> ResultProjection {
        let mut diags: Vec<_> = r
            .diagnostics
            .iter()
            .map(|d| (d.code.to_string(), d.span.start(), d.span.end()))
            .collect();
        diags.sort();
        let mut procs: Vec<_> = r.all_procs.keys().cloned().collect();
        procs.sort();
        let mut vars: Vec<_> = r.global_scope.variables.keys().cloned().collect();
        vars.sort();
        let mut invs: Vec<_> = r
            .command_invocations
            .iter()
            .map(|i| (i.name.clone(), i.range.start()))
            .collect();
        invs.sort();
        (diags, procs, vars, invs)
    }

    struct IncrLcg(u64);
    impl IncrLcg {
        fn next_u32(&mut self) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            (self.0 >> 33) as u32
        }
    }

    #[test]
    #[ignore = "seeded edit-storm oracle (350 edit/analyse pairs); the deterministic \
                incremental gates stay in CI — the *_incremental_matches_fresh* family \
                and per_item_corpus::per_item_matches_analyse_over_repo_samples; \
                run explicitly with --ignored"]
    fn analyse_incremental_matches_full_under_fuzz() {
        // The acceptance gate: a random edit applied to a base document,
        // then `analyse_incremental` must produce a result observably
        // identical to a full `analyse` of the edited text — diagnostics,
        // procs, globals, and recorded command invocations all match.
        let bases = [
            "set x 1\nputs hi\nproc p {} { return 1 }\np\nset y 2\n",
            "namespace eval n {\n  variable v 1\n  proc q {} { return $v }\n}\nn::q\n",
            "if {$a} {\n  puts a\n} else {\n  puts b\n}\nset z 3\nfoo $undef\n",
            "proc add {a b} { return [expr {$a + $b}] }\nputs [add 1 2]\nset s {x y z}\n",
            "set i 0\nwhile {$i < 10} { incr i }\nputs done\n# trailing comment\n",
        ];
        let inserts = [
            "",
            "z",
            "puts Z\n",
            "set q 9\n",
            " ",
            "\n",
            "x",
            "proc r {} {}\n",
            "incr i\n",
            "1",
        ];
        let mut rng = IncrLcg(0xD1FF_ACE5_1234_9876);
        let mut checked = 0usize;
        for base in bases {
            let prev_cmds = crate::segmenter::segment_commands(base);
            let blen = base.len();
            for _ in 0..70 {
                let mut s = (rng.next_u32() as usize) % (blen + 1);
                while !base.is_char_boundary(s) {
                    s -= 1;
                }
                let mut e = s + (rng.next_u32() as usize) % (blen + 1 - s);
                while !base.is_char_boundary(e) {
                    e += 1;
                }
                let ins = inserts[(rng.next_u32() as usize) % inserts.len()];
                let new = format!("{}{}{}", &base[..s], ins, &base[e..]);

                let mut af = Analyser::new();
                let full = af.analyse(&new, "tcl8.6");
                let mut ai = Analyser::new();
                let inc = ai.analyse_incremental(base, &prev_cmds, &new, "tcl8.6");
                assert_eq!(
                    project_result(&inc),
                    project_result(&full),
                    "incremental != full for base {base:?} edit [{s},{e}) ins {ins:?} -> {new:?}",
                );
                checked += 1;
            }
        }
        assert!(checked > 300, "fuzz corpus too small: {checked}");
    }

    #[test]
    fn analyse_incremental_falls_back_on_incomplete_and_stubs() {
        let base = "set x 1\nputs hi\n";
        let prev = crate::segmenter::segment_commands(base);
        // Incomplete (unterminated brace) -> full-analyse fallback, still
        // identical to a direct full analyse.
        let incomplete = "set x 1\nproc p {} {\n";
        let mut ai = Analyser::new();
        let inc = ai.analyse_incremental(base, &prev, incomplete, "tcl8.6");
        let mut af = Analyser::new();
        let full = af.analyse(incomplete, "tcl8.6");
        assert_eq!(project_result(&inc), project_result(&full));
        // Stub directive -> fallback path.
        let stubbed = "# tcl-lsp: stub mycmd\nmycmd a b\n";
        let mut ai2 = Analyser::new();
        let inc2 = ai2.analyse_incremental(base, &prev, stubbed, "tcl8.6");
        let mut af2 = Analyser::new();
        let full2 = af2.analyse(stubbed, "tcl8.6");
        assert_eq!(project_result(&inc2), project_result(&full2));
    }
}
