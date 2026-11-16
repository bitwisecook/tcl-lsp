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

/// One captured instance-creation site on the per-item path: the raw
/// `(command, args, creation namespace, site offset)` handed back to
/// [`Analyser::record_instance_creation`] by the post-graft replay.
pub(super) type PendingInstanceCreation = (String, Vec<String>, String, u32);

/// One `$cmd`-head dispatch site, pending settlement against the
/// compiler's flow-sensitive value model once the CFG/SSA
/// `CompilationUnit` is built: the value and its writable provenance come
/// from the SSA use-version's reaching constant definitions, never from a
/// lexical last-write-wins map.
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
    /// sub-span within the defining literal.  Without expansion the whole
    /// value is
    /// the command name.
    pub head_expanded: bool,
}

/// One `$class`-headed `TclOO` instance-creation site, pending settlement
/// against the compiler's flow-sensitive value
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
/// resolve yet, held until every deferred proc/method
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
    /// The ensemble invocation's head offset, where dispatch looks up the
    /// target; `None` for a callback prefix that dispatches later.
    pub head: Option<u32>,
    /// Arguments *after* the consumed subcommand word (`None` when any is
    /// `{*}`-expanded), the same convention the immediate path uses.
    pub argc: Option<usize>,
}

/// One W315 candidate from an `oo::objdefine` walk, held
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

/// Child source declaration and scoped visibility advice, independently of
/// actual entered interpreter state or native hidden allocations.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct InterpState {
    /// Created with `-safe`.
    pub safe: bool,
    /// Original scoped source visibility, independent of current callability.
    pub visibility: Option<super::SourceInterpreterVisibilitySnapshot>,
    /// An original visibility operation could not retain its source operands.
    /// Its evaluation bodies cannot reuse the withdrawn source snapshot.
    pub tainted: bool,
}

/// Source-only child visibility snapshots retain complete original inputs.
pub(super) type SafeInterpCtx = super::SourceInterpreterVisibilitySnapshot;

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
    /// `objcmd method` — a named receiver retained by the genuine original
    /// source-instance issuer. The carrier supplies conditional class metadata;
    /// actual Native dispatch and object lifetime require their own proofs.
    InstanceCommand,
    /// `my method` — a bareword head the registry declares a **self-dispatch
    /// keyword** (`CommandRegistry::method_dispatch_keyword` answering
    /// `SelfDispatch`).  The receiver is the object whose
    /// method body encloses the call, so the class comes from
    /// `Analyser::enclosing_class_at_offset` with no name resolution at all
    /// — and, uniquely among the three, the dispatch bypasses export
    /// filtering, so it can reach unexported members.
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
    /// file.
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
    /// Original nested invocation carriers stamped by the source owner.
    pub commands: Vec<crate::ir::CommandTokens>,
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
    /// next line.  Only the end moves — every consumer that
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
        for tokens in &mut self.commands {
            crate::lattice_rebase::rebase_command_tokens(tokens, i64::from(delta));
        }
        self.cmd_span = shift_span(self.cmd_span, delta);
        self.method_span = self.method_span.map(|sp| shift_span(sp, delta));
    }
}

/// Shift `span` right by `delta` — the primitive
/// [`VarCommandSite::rebase`] / [`CmdCommandSite::rebase`] are built from.
fn shift_span(span: Span, delta: u32) -> Span {
    Span::new(span.start() + delta, span.end() + delta)
}

/// Everything [`Analyser::document_command_realm`] reads besides the source.
/// Two analysers with equal inputs build equal realms for equal source.
#[derive(Debug, PartialEq)]
pub(super) struct RealmBuildInputs {
    input: crate::analyser::ResolvedAnalysisInput,
    entry: Option<Arc<crate::command_binding::SourceAnalysisEntry>>,
    file_path: Option<String>,
    settings: super::source_settings::OriginalAnalyserSourceSettings,
}

/// A realm already built for `source` under `inputs`, offered to the next walk.
#[derive(Debug)]
pub(super) struct PrebuiltRealm {
    source: String,
    inputs: RealmBuildInputs,
    realm: crate::realm::CommandBindingRealm,
    /// The whole-file command-trust scan, when the shell already lowered the
    /// document for it; it reads the same source, store, grammar and entry.
    command_trust: Option<Arc<crate::command_binding::ModuleCommandMutations>>,
}

/// Ownership of the pre-built unit consumed by one diagnostic tail.
#[derive(Debug)]
pub(super) enum CompilationUnitOverride {
    Shared(std::sync::Arc<crate::compilation_unit::CompilationUnit>),
    Owned(Box<crate::compilation_unit::CompilationUnit>),
}

impl CompilationUnitOverride {
    pub(super) fn as_unit(&self) -> &crate::compilation_unit::CompilationUnit {
        match self {
            Self::Shared(unit) => unit,
            Self::Owned(unit) => unit,
        }
    }
}

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

    /// The grammar this document is lexed under: the ingress-resolved one
    /// when the walk came through [`Self::resolve_walk_environment`], else
    /// the profile's. Every re-segmentation and every grammar-axis read in
    /// the handlers goes through here, never through `profile.grammar`
    /// directly, so the analyser cannot lex a document under a rule the
    /// ingress did not resolve.
    pub(super) fn grammar(&self) -> tcl_dialect::LexerGrammar {
        self.ingress_grammar.unwrap_or(self.profile.grammar)
    }

    /// Complete source invocation axes from the retained entry or actual
    /// authoring environment. This supplies no entered handler or name lookup.
    pub(super) fn original_source_invocation_dialect(
        &self,
    ) -> Option<tcl_registry::InvocationDialect> {
        match self.source_analysis_entry.as_deref() {
            Some(entry) => entry
                .options()
                .source_invocation_dialect(self.lexer_config()),
            None => self.registry.as_deref().map(|registry| {
                crate::environment_ingress::authoring_invocation_dialect(
                    registry,
                    None,
                    self.lexer_config(),
                )
            }),
        }
    }

    /// The command surface this document analyses against: `registry` plus
    /// the document's own `# tcl-lsp: stub` declarations.
    ///
    /// A stub states the same kind of fact a `CommandSpec` does, so every
    /// role query in the walk goes through here rather than reaching for the
    /// bare catalogue. The registry is passed in because a caller has already
    /// resolved which one this document analyses under, and sometimes holds
    /// it as a clone so a `&mut self` step can run beside it.
    pub(super) fn command_surface<'a>(
        &'a self,
        registry: &'a tcl_registry::CommandRegistry,
    ) -> tcl_registry::model::DocumentCommandSurface<'a> {
        tcl_registry::model::DocumentCommandSurface::new(registry, self.declared_commands.as_ref())
    }

    /// The body-lexing config for this document: [`Self::grammar`] — the
    /// ingress-resolved grammar, else the profile's — carrying the
    /// dialect-dependent tokenisation flags (`{*}` expansion, the iRules
    /// `}{` word break, numerals, escapes, Jim's axes). Threaded into every
    /// analyser re-segmentation so a body is read under the document's
    /// grammar, never the default's. [`Self::file_lexer_config`] is the
    /// whole-file form.
    pub(super) fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.resolved_input.as_ref().map_or_else(
            || tcl_lexer::LexerConfig::from_grammar(self.grammar()),
            |input| input.config,
        )
    }

    /// This document dialect's word-value rules: how a braced word's
    /// `\<newline>` folds and how list text divides. The [`Self::grammar`]
    /// twin for the re-parses that split a *word* rather than re-lex a
    /// script — a proc's parameter list, an OO member's — so they cannot
    /// answer C Tcl's question about a `JimTcl` document.
    pub(super) fn word_rules(&self) -> tcl_syntax::word_rules::WordValueRules {
        tcl_syntax::word_rules::WordValueRules::from_grammar(&self.grammar())
    }

    /// The release-dependent facts the index-bounds proofs read for this
    /// document: its character model, its numeral grammar, and whether `lset`
    /// appends at a list's length — `Some` when the profile names a measured
    /// release, or every release agrees, and `None` where they split
    /// ([`tcl_dialect::TclVersion::lset_appends_at_length_for`]).
    pub(super) fn bounds_release(&self) -> crate::interval_bounds::BoundsRelease {
        let release = (!self.result.dialect.is_empty())
            .then(|| self.profile.const_fold_version())
            .flatten();
        crate::interval_bounds::BoundsRelease {
            characters: self.profile.character_model(),
            numbers: crate::intervals::numbers_for_dialect(Some(self.profile)),
            lset_appends_at_length: tcl_dialect::TclVersion::lset_appends_at_length_for(release),
        }
    }

    /// [`Self::lexer_config`] for the **whole-file** segmentation at the top of
    /// [`Self::analyse`] — the one place that stands where a Tcl runtime's
    /// `source` stands, and therefore the only place that may skip a leading
    /// byte-order mark. An independently supplied full input retains its
    /// own BOM treatment. Default ingress uses the selected file grammar:
    /// Tcl 9's `source` strips a leading U+FEFF; Tcl 8.x's does not.
    pub(super) fn file_lexer_config(&self) -> tcl_lexer::LexerConfig {
        // Implementation contract: naming.consumer.original-ilx-method-source-candidates
        // docs/design/analysis/name-resolution-proofs/original-ilx-method-source-candidates.md
        // A supplied whole configuration owns the document boundary as well
        // as its retained body/Realm input. Only default ingress selects the
        // profile's source-file BOM treatment.
        self.resolved_input.as_ref().map_or_else(
            || tcl_lexer::LexerConfig {
                leading_bom: tcl_lexer::LexerConfig::for_file_grammar(self.grammar()).leading_bom,
                ..self.lexer_config()
            },
            |input| input.config,
        )
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
    /// showed it as clean.
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
    /// mentions.
    #[must_use]
    pub fn braced_var(&self) -> tcl_dialect::BracedVarStyle {
        self.lexer_config().braced_var
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
    /// the suffix fold. The three call sites — the const
    /// dispatch record, the W307 head reading, and `resolve_dynamic_word` —
    /// share it so they cannot disagree about which bytes name the variable.
    #[must_use]
    pub(in crate::analyser) fn split_braced_head<'t>(&self, raw: &'t str) -> (&'t str, &'t str) {
        match tcl_lexer::braced_var_name_end(raw.as_bytes(), 0, self.braced_var()) {
            tcl_lexer::BracedVarEnd::Closed(end) => (&raw[..end], &raw[end + 1..]),
            tcl_lexer::BracedVarEnd::Unterminated => (raw, ""),
        }
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
            authenticated_report_request: None,
            result: AnalysisResult::default(),
            current_scope_path: Vec::new(),
            source: Arc::from(""),
            original_static_source_names: HashMap::new(),
            profile: tcl_dialect::DialectProfile::plain_tcl(),
            ingress_grammar: None,
            unit_profile: None,
            resolved_input: None,
            environment: None,
            context: None,
            pack_overlay: 0,
            disabled_diagnostics: disabled,
            extra_commands: Arc::new(HashSet::new()),
            last_comment: String::new(),
            original_class_definition_body: None,
            file_path: None,
            const_strings: HashMap::new(),
            finite_foreach_names: super::handlers::FiniteForeachReportingState::default(),
            logical_const_string_provenance: HashMap::new(),
            logical_interp_path_provenance: HashMap::new(),
            nondominating_consts: HashMap::new(),
            regex_vars: HashSet::new(),
            current_event: None,
            tk_accumulation_enabled: false,
            tk_ambient: false,
            tk_pending_diags: Vec::new(),
            tk_domains: std::collections::BTreeMap::new(),
            version_gate_sites: Vec::new(),
            dsl_gate_sites: Vec::new(),
            proven_sites: Vec::new(),
            interval_index_sites: HashSet::new(),
            loop_candidates: Vec::new(),
            loop_unseen_writes: super::bounds_checks::ModuleUnseenWrites::default(),
            pending_option_conflicts: Vec::new(),
            pending_gated_arity: Vec::new(),
            pending_gated_bare_ensemble: Vec::new(),
            library_versions: tcl_dialect::LibraryVersionOverrides::default(),
            declared_targets: Vec::new(),
            package_provides: Vec::new(),
            script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
            directive_provides: Vec::new(),
            range_context: None,
            range_numeral_grammars: Vec::new(),
            range_gate_sites: Vec::new(),
            builtin_names: None,
            builtin_dialect: None,
            conditional_depth: 0,
            control_flow_body_depth: 0,
            irules_debug_gate_depth: 0,
            body_depth: 0,
            presubstituted_args: false,
            evaluated_body_invocation: None,
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
            divergent_table_literals: HashSet::new(),
            seed_namespace_key: None,
            path_constant_source_namespace: None,
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
            recovered_source: false,
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
            static_eval_site: None,
            pending_arity: Vec::new(),
            pending_user_call_arity: Vec::new(),
            pending_ctor_arity: Vec::new(),
            pending_next_arity: Vec::new(),
            non_ascii_mode: NonAsciiMode::Default,
            structure_only: false,
            source_shell: false,
            #[cfg(test)]
            cfg_ssa_capture_for_test: None,
            workspace_classes: std::collections::HashSet::new(),
            workspace_bare_word_classes: std::collections::HashSet::new(),
            workspace_class_factories: None,
            workspace_subclass_methods: None,
            workspace_declared_commands: None,
            workspace_source_modules: None,
            defer_proc_bodies: false,
            pack_definer_seen: false,
            structural_rebind: false,
            deferred_bodies: Vec::new(),
            minted_synthetic_names: std::collections::HashSet::new(),
            cu_override: None,
            per_item_realm_inputs: None,
            prebuilt_realm: None,
            source_analysis_entry: None,
            source_region_override: None,
            capture_global_reads: None,
            capture_global_defs: None,
            pending_disabled_commands: Vec::new(),
            pending_w143: Vec::new(),
            pending_w304: Vec::new(),
            pending_instances: None,
            deferred_instance_replays: Vec::new(),
            pending_bareword_dispatch_sites: None,
            probe_skip_enclosing_fallback: false,
            probe_skip_duplicate_fallback: false,
            took_fast_path: false,
            per_item_fallback: None,
            irules_file_profiles: None,
            irules_event_bodies: None,
            irules_debug_flags: None,
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
        if self.resolved_input.is_none() {
            self.builtin_names = None;
            self.builtin_dialect = None;
            self.context = None;
            self.registry = None;
            self.result.analysis_context_unavailable = None;
        }
        self
    }

    /// Current command store after a checked analysis ingress.
    #[must_use]
    pub fn profile_registry(&self) -> std::sync::Arc<tcl_registry::registry::CommandRegistry> {
        std::sync::Arc::clone(self.analysis_context().commands())
    }

    /// Resolve the actual generation without replacing an unavailable overlay.
    pub(crate) fn try_analysis_context(
        &self,
    ) -> Result<
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        tcl_registry::model::OverlayMiss,
    > {
        if let Some(input) = &self.resolved_input {
            return Ok(std::sync::Arc::clone(&input.context));
        }
        if let Some(miss) = &self.result.analysis_context_unavailable {
            return Err(miss.clone());
        }
        if let Some(context) = &self.context {
            return Ok(std::sync::Arc::clone(context));
        }
        let environment = crate::environment_ingress::resolve_environment(self.profile.name);
        let keyed =
            crate::environment_ingress::DocumentEnvironment::keyed_versions(&self.library_versions);
        crate::environment_ingress::analysis_registry(&environment, &keyed, self.pack_overlay)
    }

    /// Internal generation access after an entry has checked availability.
    #[must_use]
    pub(crate) fn analysis_context(&self) -> std::sync::Arc<tcl_registry::model::ContextRegistry> {
        self.try_analysis_context()
            .expect("analysis consumers require a checked available ingress")
    }

    /// An unavailable generation is a typed incomplete analysis, not a clean
    /// successful result. No source or declaration walk is performed.
    pub(super) fn unavailable_analysis_result(&mut self, dialect: &str) -> Option<AnalysisResult> {
        let miss = self.result.analysis_context_unavailable.clone()?;
        let result = AnalysisResult {
            dialect: dialect.to_owned(),
            analysis_context_unavailable: Some(miss),
            ..AnalysisResult::default()
        };
        self.result = result.clone();
        Some(result)
    }

    /// Resolve `dialect` at a walk ingress: stash
    /// the environment and this walk's registry generation, derive the
    /// interop [`Self::profile`], and return whether this environment
    /// ships `Tk` **ambient** ([`Self::tk_ambient`] — the fact
    /// carried as a placement query on the walk's own context, not as an
    /// `availability_for_name` `TK`-bit union).
    pub(super) fn resolve_walk_environment(&mut self, dialect: &str) -> bool {
        if let Some(input) = &self.resolved_input {
            self.result.analysis_context_unavailable = None;
            self.profile = input.profile;
            self.ingress_grammar = Some(input.config.grammar_over(input.profile.grammar));
            self.unit_profile = Some(input.unit_profile);
            self.context = Some(std::sync::Arc::clone(&input.context));
            self.environment = None;
            return input.context.context().ambient_package("Tk");
        }
        let environment = crate::environment_ingress::resolve_environment(dialect);
        self.profile = environment.analyser_profile();
        self.ingress_grammar = Some(environment.grammar());
        self.unit_profile = Some(environment.unit_profile());
        let keyed =
            crate::environment_ingress::DocumentEnvironment::keyed_versions(&self.library_versions);
        let generation = match crate::environment_ingress::analysis_registry(
            &environment,
            &keyed,
            self.pack_overlay,
        ) {
            Ok(generation) => generation,
            Err(miss) => {
                self.context = None;
                self.registry = None;
                self.result.analysis_context_unavailable = Some(miss);
                self.environment = Some(environment);
                return false;
            }
        };
        self.result.analysis_context_unavailable = None;
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

    /// Declare what a `package require` additionally loads
    /// ([`Self::package_provides`]) — `("myExtension", ["Tk"])` says a
    /// document that requires `myExtension` has `Tk` available too.
    #[must_use]
    pub fn with_package_provides(mut self, provides: Vec<(String, Vec<String>)>) -> Self {
        self.package_provides = provides;
        self
    }

    /// Declare the encoding the target reads this script in
    /// (`tclLsp.sourceEncoding`). The caller decoded the text with it, so a
    /// fold over a non-ASCII literal is exact under every release; `Auto`
    /// leaves 8.x folds over such literals declined.
    #[must_use]
    pub const fn with_script_encoding(
        mut self,
        encoding: tcl_registry::value_transfer::ScriptEncoding,
    ) -> Self {
        self.script_encoding = encoding;
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
            // coerced. Recognising only `tcl` as a family here would mint a
            // fictitious *package* axis named `jim` for `supports jim 0.81-`
            // and switch range mode on against it.
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

    /// Enable source-shell mode (see [`Self::source_shell`]): keep the
    /// document's source identity and interpret nothing.
    #[must_use]
    pub fn source_shell(mut self) -> Self {
        self.source_shell = true;
        self
    }

    /// The workspace classes whose command bare-word-constructs — see
    /// [`Self::workspace_bare_word_classes`].
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
    /// another file is classified instead of abstained on.
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

    /// Declare the procedures other workspace documents define that a call
    /// may trust, returning `self` for builder-style chaining.
    #[must_use]
    pub fn with_workspace_declared_commands(
        mut self,
        commands: Option<std::sync::Arc<tcl_registry::model::DeclaredSurface>>,
    ) -> Self {
        self.workspace_declared_commands = commands;
        self
    }

    /// Offer sibling workspace files for auto-loading, returning `self` for
    /// builder-style chaining.
    #[must_use]
    pub fn with_workspace_source_modules(
        mut self,
        modules: Option<std::sync::Arc<Vec<crate::command_binding::TrustedSourceModuleLoader>>>,
    ) -> Self {
        self.workspace_source_modules = modules;
        self
    }

    /// `own` over the workspace declarations: the document's own win.
    pub(super) fn with_workspace_declarations(
        &self,
        own: tcl_registry::model::DeclaredSurface,
    ) -> tcl_registry::model::DeclaredSurface {
        let Some(workspace) = self.workspace_declared_commands.as_deref() else {
            return own;
        };
        // An unavailable generation performs no walk, so nothing reads this.
        let Ok(context) = self.try_analysis_context() else {
            return own;
        };
        own.over_workspace_procedures(Some(workspace), context.commands())
    }

    /// Supply the workspace's **subclass-provided method** view — the
    /// instance methods dispatchable on some descendant of each class, with
    /// the descendants written in other documents — so the template-method
    /// abstention holds across the workspace boundary.
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

    /// Retain the driver's actual command/package entry for every analysis phase.
    /// Package advertisements and assistance declarations do not supply this proof.
    #[must_use]
    pub fn with_source_analysis_entry(
        mut self,
        entry: std::sync::Arc<crate::command_binding::SourceAnalysisEntry>,
    ) -> Self {
        self.source_analysis_entry = Some(entry);
        self
    }

    /// Use the driver's retained editing inputs at every full, chunked and
    /// per-item ingress. The dialect string passed to `analyse` remains a
    /// presentation label. Reusing this analyser preserves the explicit input;
    /// no display-name lookup replaces its grammar or command generation.
    #[must_use]
    pub fn with_resolved_input(mut self, input: super::input::ResolvedAnalysisInput) -> Self {
        self.builtin_names = None;
        self.builtin_dialect = None;
        self.resolved_input = Some(input);
        self
    }

    /// Resolve the driver's complete editing input without walking source.
    /// An unavailable overlay is returned unchanged, without a donor generation.
    ///
    /// # Errors
    /// Returns the selected overlay miss when its command store is unavailable.
    pub fn prepare_analysis_input(
        &mut self,
        dialect: &str,
    ) -> Result<super::input::ResolvedAnalysisInput, tcl_registry::model::OverlayMiss> {
        self.resolve_walk_environment(dialect);
        self.try_analysis_context()?;
        Ok(self.resolved_analysis_input())
    }

    /// Retain the current whole-source editing generation and configuration.
    /// Supplied inputs stay exact; default ingress records its file boundary.
    /// Extracted bodies explicitly use the retained input's nested projection.
    /// This does not retain the document's temporal execution world.
    #[must_use]
    pub fn resolved_analysis_input(&self) -> super::input::ResolvedAnalysisInput {
        let input = super::input::ResolvedAnalysisInput::new(
            self.profile,
            self.unit_profile.unwrap_or(self.profile),
            self.analysis_context(),
            self.file_lexer_config(),
        )
        .with_workspace_source_modules(self.workspace_source_modules.clone())
        .with_script_encoding(self.script_encoding);
        // A supplied input keeps the policy its supplier gave it, including
        // none; only the default ingress derives one from the environment.
        let policy = match &self.resolved_input {
            Some(supplied) => supplied.vendor_source_policy(),
            None => self.vendor_source_name_policy(),
        };
        match policy {
            Some(policy) => input.with_vendor_source_policy(policy),
            None => input,
        }
    }

    /// Select the same source interpretation without walking diagnostics or
    /// constructing an analysis projection. Its editing input is sealed once.
    pub(crate) fn readonly_source_registry_realm(
        source: &str,
        input: super::input::ResolvedAnalysisInput,
    ) -> (
        crate::realm::CommandBindingRealm,
        super::input::ResolvedAnalysisInput,
    ) {
        Self::readonly_realm(input, false, |analyser| {
            analyser.document_command_realm(source)
        })
    }

    /// The fixed package-index survey's readonly realm, with its scoped
    /// conditional source control. Any input the survey realm declines keeps
    /// the ordinary readonly source realm.
    pub(crate) fn readonly_package_index_survey_realm(
        source: &str,
        input: super::input::ResolvedAnalysisInput,
    ) -> (
        crate::realm::CommandBindingRealm,
        super::input::ResolvedAnalysisInput,
    ) {
        Self::readonly_realm(input, false, |analyser| {
            analyser
                .package_index_survey_realm(source)
                .unwrap_or_else(|| analyser.document_command_realm(source))
        })
    }

    /// Select exactly the standalone structural consumer's pre-walk realm and
    /// settings. No source walk, diagnostics or analysis projection is made.
    pub(crate) fn readonly_standalone_structure_realm(
        source: &str,
        input: super::input::ResolvedAnalysisInput,
    ) -> (
        crate::realm::CommandBindingRealm,
        super::input::ResolvedAnalysisInput,
    ) {
        Self::readonly_realm(input, true, |analyser| {
            analyser.document_command_realm(source)
        })
    }

    fn readonly_realm(
        input: super::input::ResolvedAnalysisInput,
        structure_only: bool,
        realm: impl FnOnce(&Self) -> crate::realm::CommandBindingRealm,
    ) -> (
        crate::realm::CommandBindingRealm,
        super::input::ResolvedAnalysisInput,
    ) {
        let profile = input.analyser_profile();
        let mut analyser = Self::new().with_resolved_input(input);
        analyser.structure_only = structure_only;
        analyser.resolve_walk_environment(profile.name);
        analyser.registry = Some(analyser.profile_registry());
        let realm = realm(&analyser);
        (realm, analyser.resolved_analysis_input())
    }

    fn package_index_survey_realm(
        &self,
        source: &str,
    ) -> Option<crate::realm::CommandBindingRealm> {
        if self.source_analysis_entry.is_some() {
            return None;
        }
        let input = self.resolved_analysis_input();
        let declarations = crate::command_binding::SourceDeclaredCommandContracts::for_document(
            source,
            self.file_path.as_deref(),
            &input,
            None,
            self.workspace_declared_commands.as_deref(),
            self.workspace_source_modules
                .as_deref()
                .map_or(&[][..], Vec::as_slice),
        );
        crate::realm::document_package_index_survey_realm_with_resolved_input(
            source,
            &input,
            &declarations,
        )
    }

    pub(super) fn realm_build_inputs(&self) -> RealmBuildInputs {
        RealmBuildInputs {
            input: self.resolved_analysis_input(),
            entry: self.source_analysis_entry.clone(),
            file_path: self.file_path.clone(),
            settings: super::source_settings::OriginalAnalyserSourceSettings::capture(self),
        }
    }

    /// The realm for `source`: the one the per-item shell already built when it
    /// was built for the same source under equal inputs, otherwise a new one.
    fn document_command_realm_reusing(
        &mut self,
        source: &str,
    ) -> crate::realm::CommandBindingRealm {
        if self.source_shell {
            return crate::realm::document_shell_realm_with_resolved_input(
                source,
                &self.resolved_analysis_input(),
            )
            .with_analyser_source_settings(Arc::new(
                super::source_settings::OriginalAnalyserSourceSettings::capture(self),
            ));
        }
        if let Some(prebuilt) = self.prebuilt_realm.take()
            && self.source_region_override.is_none()
            && prebuilt.source == source
            && prebuilt.inputs == self.realm_build_inputs()
        {
            self.command_trust = prebuilt.command_trust;
            return prebuilt.realm;
        }
        self.document_command_realm(source)
    }

    pub(super) fn document_command_realm(&self, source: &str) -> crate::realm::CommandBindingRealm {
        if let Some(region) = &self.source_region_override {
            // Only the private region entry installs this override after exact
            // input/settings checks. It reuses the actual kernel attachment.
            return crate::realm::conditional_region_realm(region);
        }

        // naming.minifier.logical-source-header
        // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let input = self.resolved_analysis_input();
        let unavailable_entry = self.source_analysis_entry.as_deref().and_then(|entry| {
            entry
                .source_point_entry
                .as_ref()
                .filter(|point| !point.accepts_analyser_source_settings(self))
                .map(|_| crate::command_binding::SourceAnalysisEntry {
                    metadata_context:
                        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
                    ..entry.clone()
                })
        });
        let entry = unavailable_entry
            .as_ref()
            .or(self.source_analysis_entry.as_deref());
        let declarations = crate::command_binding::SourceDeclaredCommandContracts::for_document(
            source,
            self.file_path.as_deref(),
            &input,
            entry,
            self.workspace_declared_commands.as_deref(),
            self.workspace_source_modules
                .as_deref()
                .map_or(&[][..], Vec::as_slice),
        );
        crate::realm::document_realm_with_declared_contracts(source, &input, entry, &declarations)
            .with_analyser_source_settings(Arc::new(
                super::source_settings::OriginalAnalyserSourceSettings::capture(self),
            ))
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
        self.cu_override = Some(CompilationUnitOverride::Shared(cu));
    }

    /// Supply an owned unit. The same source-equivalence
    /// and consume-once contract as [`Self::set_cu_override`] applies.
    pub fn set_owned_cu_override(&mut self, cu: crate::compilation_unit::CompilationUnit) {
        self.cu_override = Some(CompilationUnitOverride::Owned(Box::new(cu)));
    }

    /// Analyse exactly one issued conditional suffix, reusing its kernel world.
    /// The result remains a partial view, never a whole-document analysis/CU.
    pub fn analyse_source_region(
        &mut self,
        region: &crate::command_binding::OriginalSourceConditionalRegion,
        parent: &AnalysisResult,
        child_prefix: &AnalysisResult,
    ) -> Option<AnalysisResult> {
        if self.source_analysis_entry.is_some()
            || self.source_region_override.is_some()
            || self.cu_override.is_some()
            || self.seed_namespace_key.is_some()
            || !region.matches_dependencies(parent, child_prefix)
            || self
                .resolved_input
                .as_ref()
                .is_some_and(|input| input != region.input())
        {
            return None;
        }
        region.configure_analyser(self);
        self.resolved_input = Some(region.input().clone());
        let source = region.source_image().try_text().ok()?;
        let entry = region.entry()?;
        let contracts = crate::command_binding::SourceDeclaredCommandContracts::for_document(
            source,
            self.file_path.as_deref(),
            region.input(),
            Some(&entry),
            self.workspace_declared_commands.as_deref(),
            self.workspace_source_modules
                .as_deref()
                .map_or(&[][..], Vec::as_slice),
        );
        if !region.accepts_analyser(self)
            || !entry
                .source_point_entry
                .as_ref()?
                .accepts_child_contracts(contracts.with_options(entry.options()))
        {
            return None;
        }
        self.source_analysis_entry = Some(Arc::new(entry));
        self.source_region_override = Some(region.clone());
        self.result.original_source_region = Some(region.clone());
        let previous_scope = std::mem::take(&mut self.current_scope_path);
        let mut result = self.analyse_commands(
            source,
            region.commands(),
            region.input().analyser_profile().name,
            false,
        );
        self.source_analysis_entry = None;
        self.source_region_override = None;
        self.current_scope_path = previous_scope;
        // The unavailable early-return path does not take the per-run result.
        // Never leave this private partial marker on a reused analyser.
        self.result.original_source_region = None;
        result.original_source_region = Some(region.clone());
        region.matches_region_analysis(&result).then_some(result)
    }

    /// Analyse one source-only child view under the exact retained parent point.
    /// Missing or incompatible ownership refuses without standalone reanalysis.
    pub fn analyse_with_source_point_entry(
        &mut self,
        source: &str,
        dialect: &str,
        point: &crate::command_binding::OriginalSourcePointEntry,
    ) -> Option<AnalysisResult> {
        if self.source_analysis_entry.is_some()
            || self.cu_override.is_some()
            || self.seed_namespace_key.is_some()
            || point.namespace_scope().is_none()
            || !point.accepts_analyser_source_settings(self)
        {
            return None;
        }
        self.resolve_walk_environment(dialect);
        let input = self.resolved_analysis_input();
        let context = input.context_registry();
        let entry = point.source_analysis_entry(context.commands(), &input)?;
        let contracts = crate::command_binding::SourceDeclaredCommandContracts::for_document(
            source,
            self.file_path.as_deref(),
            &input,
            Some(&entry),
            self.workspace_declared_commands.as_deref(),
            self.workspace_source_modules
                .as_deref()
                .map_or(&[][..], Vec::as_slice),
        );
        if !point.accepts_child_contracts(contracts.with_options(entry.options())) {
            return None;
        }
        self.source_analysis_entry = Some(Arc::new(entry));
        let result = self.analyse(source, dialect);
        self.source_analysis_entry = None;
        (result.resolved_input.as_ref() == Some(point.input())
            && result.matches_original_source_image(
                &tcl_lexer::SourceImage::document(source),
                input.lexer_config(),
            ))
        .then_some(result)
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
    /// current namespace.  Relative definitions home under the seed
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

    /// Test fixtures with private post-analysis queries retain the genuine result.
    #[cfg(test)]
    pub(crate) fn analyse_and_retain_result_for_test(
        &mut self,
        source: &str,
        dialect: &str,
    ) -> &AnalysisResult {
        self.result = self.analyse(source, dialect);
        &self.result
    }

    /// Own all provisional analysis until its interruption boundary has closed.
    /// A stopped job is consumed, so its partial realm/scope cannot be reused.
    pub fn try_analyse_owned(
        mut self,
        source: &str,
        dialect: &str,
        namespace: Option<&str>,
        control: crate::analysis_control::AnalysisControl,
    ) -> Result<AnalysisResult, crate::analysis_control::AnalysisInterrupted> {
        crate::analysis_control::run(control, || match namespace {
            Some(namespace) => self.analyse_with_source_namespace(source, dialect, namespace),
            None => self.analyse(source, dialect),
        })
    }

    /// `source` is consumed by reference so the analyser can hold
    /// per-walk references back into it; `dialect` is one of
    /// `"tcl"`, `"f5-irules"`, `"irules"`, `"iapps"`, etc. (kept in
    /// the analyser's per-walk state via [`Self::current_event`]
    /// elsewhere; this entry just records it for future use).
    pub fn analyse(&mut self, source: &str, dialect: &str) -> AnalysisResult {
        use crate::analysis_control::{AnalysisCheckpoint, checkpoint};
        use std::collections::HashSet;
        if checkpoint(AnalysisCheckpoint::Start) {
            return AnalysisResult::default();
        }

        let phase_start = ((cfg!(debug_assertions)
            && tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some())
            || tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_WORKSPACE_SCAN").is_some())
        .then(std::time::Instant::now);
        Self::trace_analysis_phase(phase_start, source.len(), "initial");
        self.unresolved_commands_emitted = false;
        // Stash the source so handlers (recovery, diagnostic
        // emitters) can re-slice it.
        let retained = self.retain_walk_source(source);
        let source: &str = &retained;
        self.original_static_source_names.clear();
        self.original_class_definition_body = None;
        let tk_ambient = self.resolve_walk_environment(dialect);
        if let Some(result) = self.unavailable_analysis_result(dialect) {
            return result;
        }
        // Tell pack hooks which dialect they are running under, for the
        // length of this walk. Deriving a hook's `ctx.dialect` from the call's
        // `TclVersion` instead can only spell a release — an iRules document
        // would report `tcl9.0` and no hook could tell a dialect from a
        // version. The guard restores the previous value on the way
        // out, since one worker analyses documents of different dialects in
        // turn.
        let _dialect_scope = tcl_registry::pack_hooks::DialectScope::enter(Some(self.profile.name));
        self.result.dialect = dialect.to_string();
        self.result.body_lexer_config = Some(self.file_lexer_config());
        self.result.lexical_declaration_advice = self.selected_logical_declaration_advice();
        self.result.resolved_input = Some(self.resolved_analysis_input());
        self.result.library_versions = self.library_versions.clone();
        self.tk_accumulation_enabled =
            super::tk_checks::tk_checks_could_apply(source, tk_ambient, &self.package_provides);
        self.directive_provides = super::utils::parse_provides_directives(source);
        self.tk_ambient = tk_ambient;
        // §5.4 range targeting: resolve configured + directive-declared
        // version targets for this walk (a no-op for the undeclared
        // majority).
        self.resolve_declared_targets(source);
        // Clear the per-run iRules file-profile memo so a reused analyser
        // instance recomputes it for the new source / dialect.
        self.irules_file_profiles = None;
        self.irules_event_bodies = None;
        self.irules_debug_flags = None;
        self.retain_walk_directives(source, dialect);

        // Segment with re-segmentation recovery so an unclosed delimiter
        // mid-file doesn't drop later top-level declarations.
        // Build the dialect-aware registry once and stash on
        // ``self`` so per-command handlers (registry-driven body
        // iteration in ``process_command``) reuse it.
        self.registry = Some(self.profile_registry());
        Self::trace_analysis_phase(phase_start, source.len(), "realm-start");
        if checkpoint(AnalysisCheckpoint::Realm) {
            return AnalysisResult::default();
        }
        self.head_identities = self.document_command_realm_reusing(source);
        Self::trace_analysis_phase(phase_start, source.len(), "realm-complete");
        if crate::analysis_control::interrupted() {
            return AnalysisResult::default();
        }
        self.result.command_realm = Some(Arc::new(self.head_identities.clone()));
        self.result.original_command_world =
            self.head_identities.original_completed_command_world();
        self.retain_original_comment_suppressions(source);
        self.record_original_formal_variables();
        Self::trace_analysis_phase(phase_start, source.len(), "retained-inventory");
        // Precompute the iRules file-profile stack (no-op off f5-irules) so
        // the per-command IRULE1001 hint can consult it without recomputing.
        self.prepare_source_walk_indexes(source);
        let known_commands: HashSet<&str> = self.recovery_known_commands.iter().collect();
        let mut commands = crate::segmenter::segment_commands_with_recovery_and_config(
            source,
            &known_commands,
            self.file_lexer_config(),
        );
        drop(known_commands);

        // Ghost-token recovery (see method doc).
        let ghost_recovery_applied = self.apply_ghost_recovery(source, &mut commands);
        self.recovered_source =
            ghost_recovery_applied || commands.iter().any(|command| command.is_partial);
        Self::trace_analysis_phase(phase_start, source.len(), "segmented");

        // Walk each command through the dispatcher.  The dispatcher
        // wires ``recover_stray_close_bracket``,
        // ``recover_missing_open_brace`` (for switch with a forgotten
        // body brace), ``detect_stolen_close_brace`` (E103), and the
        // generic E200 partial-command emitter.
        // A seeded analysis (`analyse_with_source_namespace`) walks the
        // whole file inside the source-site namespace, exactly as if wrapped
        // in `namespace eval <ns> { ... }` — relative definitions re-home,
        // absolute ones stay put, and call-site candidates gain the seeded
        // tier, all through the ordinary scope machinery.
        self.seed_walk_namespace(source);
        let file_env_pushed = self.seed_file_scope_env(source);
        Self::trace_analysis_phase(phase_start, source.len(), "walk-start");
        let workspace_walk = super::commands::WorkspaceWalkPhases::start();
        self.walk_commands_top_level(&commands, ghost_recovery_applied);
        if let Some(trace) = workspace_walk {
            trace.finish(source.len(), self.structure_only);
        }
        Self::trace_analysis_phase(phase_start, source.len(), "walk-complete");
        if crate::analysis_control::interrupted() {
            return AnalysisResult::default();
        }
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
        // value a literal call-site argument proves.  A
        // *structural* fact, so it runs on the item-tree path too — that is
        // the path the workspace class-factory index is computed from, and a
        // metaclass missing there is invisible to every other document.
        self.record_literal_parameter_definitions();
        self.retain_original_callback_signature_lookups(source);
        Self::trace_analysis_phase(phase_start, source.len(), "parameter-definitions");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return AnalysisResult::default();
        }
        if self.structure_only {
            if let Some(registry) = self.registry.as_deref() {
                self.result.retain_logical_class_source_factories(registry);
            }
        } else {
            self.run_diagnostic_emitters(source);
        }
        Self::trace_analysis_phase(phase_start, source.len(), "diagnostics-complete");
        if crate::analysis_control::interrupted() {
            return AnalysisResult::default();
        }

        let mut result = std::mem::take(&mut self.result);
        result.source_words_memo.clear();
        self.clear_run_state();
        Self::trace_analysis_phase(phase_start, source.len(), "complete");
        result
    }

    /// Hold `source` through its document image's buffer and return it.
    pub(super) fn retain_walk_source(&mut self, source: &str) -> Arc<str> {
        let text = tcl_lexer::SourceImage::document(source)
            .shared_text()
            .unwrap_or_else(|| Arc::from(source));
        self.source = Arc::clone(&text);
        text
    }

    fn trace_analysis_phase(
        phase_start: Option<std::time::Instant>,
        source_bytes: usize,
        stage: &str,
    ) {
        if let Some(start) = phase_start {
            eprintln!(
                "ANALYSER_PHASE bytes={source_bytes} stage={stage} ms={}",
                start.elapsed().as_millis()
            );
        }
    }

    fn prepare_source_walk_indexes(&mut self, source: &str) {
        self.compute_irules_file_profiles();
        self.line_offsets = Some(compute_line_offsets(source));
        self.cached_line_index = tcl_lexer::LineIndex::new(source);
        self.cached_line_index_source_len = source.len();
        self.recovery_known_commands = super::utils::recovery_known_commands(
            source,
            self.registry.as_deref().expect("registry just stashed"),
            &self.extra_commands,
            self.file_lexer_config(),
        );
    }

    fn seed_walk_namespace(&mut self, source: &str) {
        if let Some(point) = self
            .source_analysis_entry
            .as_ref()
            .and_then(|entry| entry.source_point_entry.as_ref())
        {
            let kind = match point.frame().layout() {
                crate::var_resolve::VariableExecutionFrame::Procedure { .. } => {
                    super::types::ScopeKind::Proc
                }
                crate::var_resolve::VariableExecutionFrame::Global => {
                    super::types::ScopeKind::Global
                }
                _ => super::types::ScopeKind::Namespace,
            };
            let mut scope = super::types::Scope::new(
                kind,
                point.namespace().display().as_deref().unwrap_or(""),
            );
            scope.naming_scope = point.namespace_scope();
            scope.body_span = Some(tcl_lexer::Span::new(
                0,
                u32::try_from(source.len()).unwrap_or(u32::MAX),
            ));
            self.result.global_scope.children.push(scope);
            self.seed_scope_path = vec![self.result.global_scope.children.len() - 1];
            return;
        }
        if let Some(seed) = self.seed_namespace_key.take() {
            self.path_constant_source_namespace = Some(seed.clone());
            let end = u32::try_from(source.len()).unwrap_or(u32::MAX);
            let mut base: Vec<usize> = Vec::new();
            let policy = self.declaration_name_policy();
            let mut naming_scope = self
                .selected_logical_declaration_advice()
                .then(|| crate::signature_scan::scope::SignatureNamespaceScope::root(policy));
            for segment in crate::naming::key_segments(&seed) {
                naming_scope = naming_scope.and_then(|scope| scope.child(&segment, policy));
                let mut child =
                    super::types::Scope::new(super::types::ScopeKind::Namespace, &segment);
                child.body_span = Some(tcl_lexer::Span::new(0, end));
                child.naming_scope.clone_from(&naming_scope);
                let parent = super::scope::scope_at_mut(&mut self.result.global_scope, &base)
                    .expect("seed scope path is self-built");
                parent.children.push(child);
                base.push(parent.children.len() - 1);
            }
            self.seed_scope_path = base;
        }
    }

    /// Next-line comment syntax is selected after retaining the actual
    /// complete input, declared contracts and original source realm.
    pub(super) fn retain_original_comment_suppressions(&mut self, source: &str) {
        // Every next-line marker spells `noqa`; a source without it carries
        // no marker, so the registry-shaped comment walk has nothing to find.
        if !source
            .as_bytes()
            .windows(4)
            .any(|w| w.eq_ignore_ascii_case(b"noqa"))
        {
            return;
        }
        if let Some(lines) =
            super::utils::parse_noqa_line_suppressions_from_analysis(source, &self.result)
        {
            merge_noqa_line_suppressions(&mut self.result.suppressed_lines, lines);
        }
    }

    fn retain_walk_directives(&mut self, source: &str, dialect: &str) {
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
        // Inline ``# tcl-lsp: stub …`` block scan.  After
        // capturing the parsed records, build the per-document
        // overlay so analyser / compiler queries see the
        // user-declared stubs as first-class commands (without
        // mutating the global registry).
        let (overlay_cmds, overlay_exprs, outside) =
            super::utils::document_stub_declarations_with_outside(
                source,
                self.file_path.as_deref(),
                dialect,
                self.file_lexer_config(),
            );
        self.result
            .diagnostics
            .extend(super::utils::outside_stub_diagnostics(outside));
        self.declared_commands = Some(
            self.with_workspace_declarations(super::types::build_declared_surface(&overlay_cmds)),
        );
        self.result.stub_commands = overlay_cmds;
        self.result.stub_expr_defs = overlay_exprs;
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
        // A source shell interprets nothing.
        if self.source_shell {
            return;
        }
        self.result.global_scope.naming_scope = self.declaration_namespace_scope(&[]);
        let total = commands.len();
        let mut cmd_idx: usize = 0;
        while cmd_idx < total {
            if crate::analysis_control::checkpoint(
                crate::analysis_control::AnalysisCheckpoint::Command,
            ) {
                break;
            }
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
        // After the walk, so the path inventory reads each command's words
        // under the same recorded head metadata as every later consumer.
        self.record_path_constant_candidates(commands);
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
            self.file_lexer_config(),
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
                self.lexer_config(),
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
    /// `${…}` variable name) use the lexer's retained `ParseIssueKind`.
    /// Missing delimiters that overlap E201 / E202 / E203 are skipped (the
    /// recovery detectors own those with better positions + fixes);
    /// every other typed issue maps to the catch-all E200. Messages remain
    /// unchanged presentation.
    fn emit_lexer_warning_diagnostics(&mut self) {
        let lexer = tcl_lexer::Lexer::with_source_map(
            Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            ),
            self.file_lexer_config(),
        );
        let Ok((tokens, warnings)) = lexer.tokenise_all_with_warnings() else {
            return;
        };
        for w in warnings {
            let code = match w.kind {
                // Owned by the E201/E202/E203 recovery heuristics.
                tcl_lexer::ParseIssueKind::MissingBracket
                | tcl_lexer::ParseIssueKind::MissingQuote
                | tcl_lexer::ParseIssueKind::MissingBrace => continue,
                tcl_lexer::ParseIssueKind::ExtraAfterBrace => DiagCode::E204,
                tcl_lexer::ParseIssueKind::ExtraAfterQuote => DiagCode::E205,
                tcl_lexer::ParseIssueKind::MissingVariableBrace => DiagCode::E206,
                // Any other typed lexer issue → catch-all E200.
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
        // Flat lexer warnings cannot see welded words inside a command
        // substitution. The shared parse-cut owner descends into those scripts
        // and keeps the original source coordinates.
        let config = self.file_lexer_config();
        let commands = tcl_lexer::group_commands(&tokens, &self.source, config);
        let Ok(cuts) = tcl_lexer::parse_cuts_in(&commands, &tokens, &self.source, config) else {
            return;
        };
        let mut emitted: HashSet<_> = self
            .result
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.span.start()))
            .collect();
        for cut in cuts {
            let code = match cut.kind {
                tcl_lexer::ParseIssueKind::ExtraAfterBrace => DiagCode::E204,
                tcl_lexer::ParseIssueKind::ExtraAfterQuote => DiagCode::E205,
                _ => continue,
            };
            if !emitted.insert((code, cut.offset)) {
                continue;
            }
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    code,
                    Span::new(cut.offset, cut.offset),
                    cut.message.to_owned(),
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
        let retained = self.retain_walk_source(source);
        let source: &str = &retained;
        self.original_static_source_names.clear();
        // Same-source memo, cleared with the source it was derived from.
        self.irules_event_bodies = None;
        self.irules_debug_flags = None;
        let tk_ambient = self.resolve_walk_environment(dialect);
        if let Some(result) = self.unavailable_analysis_result(dialect) {
            return (result, Vec::new());
        }
        self.result.dialect = dialect.to_string();
        self.result.body_lexer_config = Some(self.file_lexer_config());
        self.result.lexical_declaration_advice = self.selected_logical_declaration_advice();
        self.result.resolved_input = Some(self.resolved_analysis_input());
        self.result.library_versions = self.library_versions.clone();
        self.tk_accumulation_enabled =
            super::tk_checks::tk_checks_could_apply(source, tk_ambient, &self.package_provides);
        self.directive_provides = super::utils::parse_provides_directives(source);
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

        // Build + stash the dialect-aware registry so
        // ``process_command`` 's body-iteration loop has access
        // to it on every chunked / incremental call (the entry
        // point used by the LSP's incremental update path).
        // Without this, body recursion silently no-ops.  Same
        // for the ``line_offsets`` index used by
        // ``apply_preceding_noqa``.
        self.registry = Some(self.profile_registry());
        self.head_identities = self.document_command_realm(source);
        self.result.command_realm = Some(Arc::new(self.head_identities.clone()));
        self.result.original_command_world =
            self.head_identities.original_completed_command_world();
        self.retain_original_comment_suppressions(source);
        self.recovery_known_commands = super::utils::recovery_known_commands(
            source,
            self.registry.as_deref().expect("registry just stashed"),
            &self.extra_commands,
            self.file_lexer_config(),
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
        self.retain_original_callback_signature_lookups(source);
        self.run_diagnostic_emitters(source);

        let mut result = std::mem::take(&mut self.result);
        result.source_words_memo.clear();
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
        let retained = self.retain_walk_source(source);
        let source: &str = &retained;
        self.original_static_source_names.clear();
        // Same-source memo, cleared with the source it was derived from.
        self.irules_event_bodies = None;
        self.irules_debug_flags = None;
        let tk_ambient = self.resolve_walk_environment(dialect);
        if let Some(result) = self.unavailable_analysis_result(dialect) {
            return result;
        }
        self.result.dialect = dialect.to_string();
        self.result.body_lexer_config = Some(self.file_lexer_config());
        self.result.lexical_declaration_advice = self.selected_logical_declaration_advice();
        self.result.resolved_input = Some(self.resolved_analysis_input());
        self.result.library_versions = self.library_versions.clone();
        self.tk_accumulation_enabled =
            super::tk_checks::tk_checks_could_apply(source, tk_ambient, &self.package_provides);
        self.directive_provides = super::utils::parse_provides_directives(source);
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

        // Same registry + line-index prelude as
        // ``analyse_chunked`` — see that doc-comment.  Without
        // these the registry-driven body loop in
        // ``process_command`` silently skips body recursion on
        // the incremental path.
        self.registry = Some(self.profile_registry());
        self.head_identities = self.document_command_realm(source);
        self.result.command_realm = Some(Arc::new(self.head_identities.clone()));
        self.result.original_command_world =
            self.head_identities.original_completed_command_world();
        self.retain_original_comment_suppressions(source);
        self.recovery_known_commands = super::utils::recovery_known_commands(
            source,
            self.registry.as_deref().expect("registry just stashed"),
            &self.extra_commands,
            self.file_lexer_config(),
        );
        self.line_offsets = Some(compute_line_offsets(source));
        self.cached_line_index = tcl_lexer::LineIndex::new(source);
        self.cached_line_index_source_len = source.len();

        // Stub-directive pre-scan + overlay, matching ``analyse`` so command
        // resolution (W123 / W307 / param-trait inference) sees the same stub
        // surface and ``analyse_commands`` stays byte-identical to ``analyse``.
        let (overlay_cmds, overlay_exprs, outside) =
            super::utils::document_stub_declarations_with_outside(
                source,
                self.file_path.as_deref(),
                dialect,
                self.file_lexer_config(),
            );
        self.result
            .diagnostics
            .extend(super::utils::outside_stub_diagnostics(outside));
        self.declared_commands = Some(
            self.with_workspace_declarations(super::types::build_declared_surface(&overlay_cmds)),
        );
        self.result.stub_commands = overlay_cmds;
        self.result.stub_expr_defs = overlay_exprs;

        if self.source_region_override.is_some() {
            self.seed_walk_namespace(source);
            self.current_scope_path.clone_from(&self.seed_scope_path);
        }
        let file_env_pushed = self.seed_file_scope_env(source);
        self.analyse_commands_inner(commands);
        if file_env_pushed {
            self.body_scope_stack.pop();
        }

        if finalise {
            self.record_literal_parameter_definitions();
            self.retain_original_callback_signature_lookups(source);
            self.run_diagnostic_emitters(source);
        }

        let mut result = std::mem::take(&mut self.result);
        result.source_words_memo.clear();
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
        self.resolve_walk_environment(dialect);
        if let Some(result) = self.unavailable_analysis_result(dialect) {
            return result;
        }
        // Error recovery (ghost-token re-lex, stray-closer repair) and
        // inline `# tcl-lsp: stub` overlays are only applied on the full
        // `analyse` path; when either could be in play, re-analyse fully
        // so the incremental result can never diverge.
        if tcl_lexer::script_image_is_complete(
            &tcl_lexer::SourceImage::document(new_text),
            self.file_lexer_config(),
        ) != Ok(true)
            || new_text.contains("tcl-lsp: stub")
        {
            return self.analyse(new_text, dialect);
        }
        let cmds = crate::segmenter::segment_commands_incremental(
            prev_text,
            prev_commands,
            new_text,
            // The document's whole-file grammar, resolved once through the
            // ingress exactly as `analyse` resolves it for the full walk.
            self.file_lexer_config(),
        );
        // `analyse` segments with *error recovery*; the fast path uses plain
        // incremental segmentation. Configured completeness accepts terminal
        // syntax cuts as complete; that does not prove the recovery stream.
        // A locally-unbalanced brace can still make recovery segmentation
        // split off a partial command — emitting E200/E102/E202 that the plain
        // walk never sees.  Replicate the recovery segmentation `analyse` uses
        // and fall back to a full rebuild when it differs from what the fast
        // path would walk, or leaves any partial command.  (This also subsumes
        // the plain-segmentation-metadata check: the recovery segmenter is the
        // authority on the command stream + its attached comments.)
        let generation = self.analysis_context();
        let known: std::collections::HashSet<&str> =
            generation.commands().command_names().collect();
        let recovery_cmds = crate::segmenter::segment_commands_with_recovery_and_config(
            new_text,
            &known,
            self.file_lexer_config(),
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
    ///
    /// A supplied `cu_override` describes the same source, so it moves to the
    /// fresh analyser rather than paying for a second whole-file unit build.
    pub(super) fn fresh_full_analyse(&mut self, new_text: &str, dialect: &str) -> AnalysisResult {
        let mut fresh = Analyser::with_disabled_diagnostics(self.disabled_diagnostics.clone())
            .with_resolved_input(self.resolved_analysis_input())
            .with_non_ascii_mode(self.non_ascii_mode)
            .with_file_path(self.file_path.clone());
        // The same configured analysis the fast path ran under: dropping the
        // driver's workspace, target and version settings here would give the
        // fallback (taken on most edits) a different answer from the fast path.
        super::source_settings::OriginalAnalyserSourceSettings::capture(self).apply(&mut fresh);
        fresh.defer_proc_bodies = false;
        fresh
            .source_analysis_entry
            .clone_from(&self.source_analysis_entry);
        fresh.cu_override = self.cu_override.take();
        if let Some((source, inputs)) = self.per_item_realm_inputs.take()
            && source == new_text
        {
            fresh.prebuilt_realm = Some(Box::new(PrebuiltRealm {
                source,
                inputs,
                realm: std::mem::take(&mut self.head_identities),
                command_trust: self.command_trust.take(),
            }));
        }
        fresh.analyse(new_text, dialect)
    }

    /// Extend conditional path source advice through the same actual input,
    /// original realm and complete image as command analysis. Batches preserve
    /// prior lexical homes; namespace seeds remain constructed source scopes.
    fn record_path_constant_candidates(&mut self, commands: &[crate::segmenter::SegmentedCommand]) {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let mut inventory = std::mem::take(&mut self.result.path_constant_assignments);
        crate::auto_path_eval::extend_path_constant_assignments_from_analysis_commands(
            &self.source,
            &self.result,
            commands,
            self.path_constant_source_namespace.as_deref(),
            &mut inventory,
        );
        self.result.path_constant_assignments = inventory;
    }

    /// Inner dispatch loop shared by [`Self::analyse_chunked`]
    /// and [`Self::analyse_commands`].  Walks pre-segmented
    /// commands at the current scope path.  Covers the dispatch
    /// portion that's load-bearing for incremental analysis.
    fn analyse_commands_inner(&mut self, commands: &[crate::segmenter::SegmentedCommand]) {
        self.result.global_scope.naming_scope = self.declaration_namespace_scope(&[]);
        let scope_path = self.current_scope_path.clone();
        let total = commands.len();
        let mut cmd_idx: usize = 0;
        while cmd_idx < total {
            if crate::analysis_control::checkpoint(
                crate::analysis_control::AnalysisCheckpoint::Command,
            ) {
                break;
            }
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
        self.record_path_constant_candidates(commands);
    }

    /// Resolve (and cache) the set of built-in command names that
    /// **exist** under the active dialect — the registry tier of the one
    /// `exists` oracle: every store name
    /// the resolved context actually provides
    /// ([`tcl_registry::model::ResolvedContext::resolve_spec`]), plus the
    /// measured iRules §4b interpreter-present extension
    /// ([`Self::extend_with_irules_interpreter_present_names`]).
    ///
    /// This is the same set the W123 unresolved-command pass resolves
    /// registry names against, so settlement
    /// (`finalise_invocation_resolutions`), constant-dispatch, W113, and
    /// W123 cannot disagree about which registry commands exist.  Reading
    /// the *unfiltered* store name set at settlement instead would believe in
    /// commands W123 does not.
    ///
    /// The name set is held on ``self.builtin_names`` for subsequent
    /// proc / class registrations to consult without rebuilding.
    /// The dialect resolves through the profile catalogue; unknown dialect
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
    /// revokes an import alias on it, which would drop a
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

    /// Record the packages a declared edge says the document's own `package
    /// require`s also load.
    ///
    /// A binary extension whose `Init` calls `Tcl_PkgRequire` — or links Tk
    /// through `Tk_InitStubs` — makes a package available with nothing in any
    /// Tcl source to say so, so no amount of workspace scanning can find it
    /// (the `pkgIndex.tcl` route [`transitive_available_packages`] follows
    /// reads the *Tcl* files an `ifneeded` script sources, and a `load`-only
    /// extension has none). It can only be declared, and both places it can
    /// be declared state the same edge:
    ///
    /// - `# tcl-lsp: package myExtension provides Tk` in the file
    ///   ([`Self::directive_provides`]), which travels with it; or
    /// - a `[packages.provides]` / `tclLsp.packages.provides` entry
    ///   ([`Self::package_provides`]), which states it once for the project.
    ///
    /// Each package the edges reach becomes an ordinary
    /// [`SignaturePackageRequire`](crate::signature_scan::types::SignaturePackageRequire)
    /// carrying the span of the `package require` that loaded it — the point
    /// at which it really does become available, so the order-sensitive H301
    /// stays right — plus a
    /// [`PackageRequireOrigin`](crate::signature_scan::types::PackageRequireOrigin)
    /// naming the loader, for the consumers that want what the source
    /// literally says.
    ///
    /// Called from the head of [`Self::run_diagnostic_emitters`], the one
    /// tail both the whole-file and the per-item walk reach with their
    /// `package_requires` already merged — so the two strategies cannot
    /// disagree about what a declaration made available.
    fn expand_implied_package_requires(&mut self) {
        let implied = self.implied_package_requires();
        self.result.package_requires.extend(implied);
    }

    /// Every package the declared edges reach from this document's own
    /// `package require`s, minus everything it already requires, as
    /// ready-to-record synthetic requires.
    ///
    /// Breadth-first, so each package is anchored at the earliest require
    /// that reaches it and the result is independent of hash iteration order;
    /// each name is visited once, so a cyclic declaration terminates. The
    /// file's own directives are searched before the configured edges, and a
    /// package reached through either can carry the closure onward. Empty —
    /// the fast path for every document — whenever nothing is declared or
    /// nothing is required.
    pub(super) fn implied_package_requires(
        &self,
    ) -> Vec<crate::signature_scan::types::SignaturePackageRequire> {
        use crate::signature_scan::types::{PackageRequireOrigin, SignaturePackageRequire};

        if (self.directive_provides.is_empty() && self.package_provides.is_empty())
            || self.result.package_requires.is_empty()
        {
            return Vec::new();
        }
        // The earliest offset each package is already available from — not a
        // visited *set*. Seeding with the source requires keeps a redundant
        // entry out, but comparing offsets keeps the earlier anchor when a
        // declared edge makes a package available *above* the explicit
        // require for it. A set would discard the edge's entry outright and
        // leave only the later span, and H301 takes the **minimum** start per
        // package (`unresolved.rs`), so the command between the two would be
        // reported as used-before-available — a hint that appears only
        // because the file *also* requires the package explicitly.
        let mut earliest: HashMap<String, u32> = HashMap::new();
        for pr in &self.result.package_requires {
            earliest
                .entry(pr.name.clone())
                .and_modify(|at| *at = (*at).min(pr.range.start()))
                .or_insert_with(|| pr.range.start());
        }
        // `(package name, span, conditional, control_flow)` — the anchor an
        // edge's target inherits from whatever made its source available.
        let mut queue: Vec<(String, tcl_lexer::Span, bool, bool)> = self
            .result
            .package_requires
            .iter()
            .map(|pr| (pr.name.clone(), pr.range, pr.conditional, pr.control_flow))
            .collect();
        let mut out: Vec<SignaturePackageRequire> = Vec::new();
        let mut head = 0;
        while head < queue.len() {
            let (name, range, conditional, control_flow) = queue[head].clone();
            head += 1;
            let edges = self
                .directive_provides
                .iter()
                .map(|edge| (edge, true))
                .chain(self.package_provides.iter().map(|edge| (edge, false)));
            for ((from, to), from_directive) in edges.filter(|((from, _), _)| *from == name) {
                for loaded in to {
                    // `<=` rather than `<` is what keeps a cyclic declaration
                    // terminating: re-reaching a package with the identical
                    // anchor stops. Every anchor is one of the finitely many
                    // require starts, and a name is only re-enqueued when its
                    // anchor strictly decreases.
                    let start = range.start();
                    if earliest.get(loaded).is_some_and(|&at| at <= start) {
                        continue;
                    }
                    earliest.insert(loaded.clone(), start);
                    let origin = if from_directive {
                        PackageRequireOrigin::Directive(from.clone())
                    } else {
                        PackageRequireOrigin::Provides(from.clone())
                    };
                    out.push(SignaturePackageRequire {
                        original_name: None,
                        original_requirements: Vec::new(),
                        name: loaded.clone(),
                        version: None,
                        requirements: Vec::new(),
                        exact: false,
                        range,
                        conditional,
                        control_flow,
                        origin,
                    });
                    queue.push((loaded.clone(), range, conditional, control_flow));
                }
            }
        }
        out
    }

    pub(super) fn run_diagnostic_emitters(&mut self, source: &str) {
        use crate::analysis_control::{AnalysisCheckpoint, checkpoint};
        // A source shell interpreted nothing, so there is nothing to report on.
        if self.source_shell {
            return;
        }
        let phase_start = ((cfg!(debug_assertions)
            && tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some())
            || tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_WORKSPACE_SCAN").is_some())
        .then(std::time::Instant::now);
        let trace_phase = |stage: &str| {
            if let Some(start) = phase_start {
                eprintln!(
                    "DIAGNOSTIC_PHASE bytes={} stage={stage} ms={}",
                    source.len(),
                    start.elapsed().as_millis()
                );
            }
        };
        trace_phase("initial");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        // Declared package availability first: every emitter below that asks
        // "is this package available here?" — W120, H301, the W123 widening,
        // the Tk activation gate — reads `package_requires`, so the declared
        // entries have to be in it before any of them run.
        self.expand_implied_package_requires();
        // Whole-source security checks belong in the analyser result so direct
        // consumers receive the same verdict as the LSP and CLI adapters. The
        // pure producer is also reused by non-Tcl F5 document adapters. Like
        // every other finding it is emitted raw: an inline `# noqa` reaches
        // the policy step through `suppressed_lines`, which records the
        // suppression with its reason.
        self.result
            .diagnostics
            .extend(super::source_integrity::bidi_control_diagnostics(source));
        // Replay the `<ensemble> <subcommand>` call sites the shell pass met
        // before the deferred body that declares the ensemble was walked
        // — before `finalise_invocation_resolutions`, so
        // the replayed invocations go through exactly the same settlement
        // the walk-time ones do. A no-op on every other entry point.
        self.flush_pending_ensemble_subcommand_invocations();
        // Settle every invocation's `resolved_qualified_name` with Tcl's
        // existence-checked two-step rule now that the walk has recorded
        // every definition in the file (a local candidate defined later in
        // the file still wins; an absent one falls back to global).
        self.finalise_invocation_resolutions();
        self.retain_positioned_command_definitions();
        self.publish_load_level_destructions();
        trace_phase("invocations");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        // Attach every namespace-qualified occurrence to the cell it names,
        // now that the whole file's `namespace eval` bodies have been walked
        // — a qualified read can precede its declaring `namespace eval`
        // textually and still resolve at run time.
        self.attach_qualified_var_references();
        trace_phase("variable-references");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        let diag_registry = self.profile_registry();
        self.emit_unresolved_command_diagnostics(&diag_registry);
        self.flush_disabled_command_diagnostics();
        self.flush_w143_diagnostics();
        self.flush_w304_diagnostics();
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
        trace_phase("command-diagnostics");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        self.emit_variable_usage_diagnostics();
        trace_phase("variable-usage");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        #[cfg(test)]
        let before_cfg_ssa = self
            .cfg_ssa_capture_for_test
            .is_some()
            .then(|| self.result.diagnostics.clone());
        self.emit_cfg_ssa_diagnostics(source);
        #[cfg(test)]
        if let Some(before) = before_cfg_ssa {
            self.cfg_ssa_capture_for_test = Some(
                self.result
                    .diagnostics
                    .iter()
                    .filter(|diagnostic| !before.contains(diagnostic))
                    .cloned()
                    .collect(),
            );
        }
        trace_phase("cfg-ssa");
        if checkpoint(AnalysisCheckpoint::Diagnostics) {
            return;
        }
        self.flush_loop_terminations();
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
        trace_phase("complete");
    }

    pub(super) fn retain_original_callback_signature_lookups(&mut self, source: &str) {
        let retained = self.result.command_invocations.iter().map(|invocation| {
            let prefix = std::sync::Arc::clone(invocation.original_callback_prefix.as_ref()?);
            let offset = prefix.source_registration().map(|registration| registration.site().offset)
                .or_else(|| prefix.lookup().map(|lookup| lookup.site().offset))?;
            let lookup = crate::registry_invocation::source_structure::source_callback_procedure_target_at(source, &self.result, offset, &prefix)?;
            super::SourceCallbackSignatureLookup::from_original_lookup(&self.result, prefix, std::sync::Arc::new(lookup)).map(std::sync::Arc::new)
        }).collect::<Vec<_>>();
        for (invocation, lookup) in self.result.command_invocations.iter_mut().zip(retained) {
            invocation.original_callback_signature_lookup = lookup;
        }
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
                .then(a.lookup.cmp(&b.lookup))
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
    /// command-substitution folding, built lazily on first
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
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, self.file_lexer_config())
            .with_dialect(
                // `None`, not `Some(plain_tcl)`, when the analysis named no
                // dialect: an unstated dialect is not the same input as an
                // explicit plain-Tcl one, and `Lowerer::dialect` feeds numeral
                // source selection. `self.profile` is always populated (it
                // defaults to the plain fallback), so gate on the recorded
                // spelling the way the pre-refactor `&str` boundary did.
                (!self.result.dialect.is_empty()).then_some(self.profile),
            );
        if let Some(entry) = &self.source_analysis_entry {
            lowerer.set_source_analysis_options(entry.options());
        }
        lowerer.lower(&self.source);
        let module = lowerer.finish_module_for_command_trust(&self.source);
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
        // Command inventory is scoped to this resolved context, not a profile
        // display label shared by distinct pack generations.
        self.builtin_names = None;
        self.builtin_dialect = None;
        self.registry = None;
        self.context = None;
        self.environment = None;
        self.command_trust = None;
        self.objdefine_bindings.clear();
        self.objdefine_abort_candidates.clear();
        self.objdefine_unresolved_receiver = false;
        self.seed_namespace_key = None;
        self.path_constant_source_namespace = None;
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
        self.proven_sites.clear();
        self.interval_index_sites.clear();
        self.loop_candidates.clear();
        self.loop_unseen_writes = super::bounds_checks::ModuleUnseenWrites::default();
        self.pending_bareword_dispatch_sites = None;
        self.line_offsets = None;
        self.cached_line_index = tcl_lexer::LineIndex::new("");
        self.cached_line_index_source_len = 0;
        self.e207_emitted = false;
        self.defer_proc_bodies = false;
        self.deferred_bodies.clear();
        self.cu_override = None;
        self.per_item_realm_inputs = None;
        self.prebuilt_realm = None;
        self.source_region_override = None;
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
mod tests;

#[cfg(test)]
mod retained_configuration_consumer_tests;
