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

//! Per-item incremental analysis
//! (`docs/design/rust/incremental-analysis.md`).
//!
//! [`Analyser::analyse_per_item`] reproduces [`Analyser::analyse`]
//! **byte-for-byte** but decomposed so each top-level **proc / method body** is
//! analysed as a separate unit — the granularity at which the expensive
//! per-line walk is memoised (a body edit then re-analyses only that body).
//!
//! ## How it stays byte-identical
//!
//! The walk is split into two passes over the *same* analyser:
//! 1. **Shell pass** — walk the top level with `defer_proc_bodies = true`, so
//!    `handle_proc_command` / OO method walks create each body's scope (params
//!    defined, `body_span` set) but record the body in `deferred_bodies` instead
//!    of recursing. The whole top level (incl. `namespace eval` / control-flow
//!    bodies and the procs they contain) is walked here, so top-level variable
//!    flow is identical to `analyse`.
//! 2. **Body pass** — for each deferred body (proc *and* method),
//!    `analyse_proc_body_isolated` analyses it as an offset-0 isolated unit and
//!    `graft_proc_body` rebases + merges the result into the shell.  Both are
//!    memoisable, so a body edit re-analyses only that body.
//!
//! Splitting top-level vs body work reorders the walk-populated collections
//! (`command_invocations`, `VarDef.references`, …) relative to `analyse`'s DFS,
//! but those are made order-independent by `canonicalize_result_order` in the
//! shared tail — so the merged result matches. The cross-item tail
//! (`run_diagnostic_emitters`: W123 / arity / whole-source CFG-SSA) runs once at
//! the end, exactly as in `analyse`.
//!
//! Correctness rests on the differential fuzzer + the `per_item == analyse`
//! corpus gate, with a **full-rebuild fallback** for anything not provably
//! equivalent (error recovery / partial commands / inline stub directives).

use tcl_lexer::Token;

use super::state::Analyser;
use super::types::AnalysisResult;

mod body_pass;
mod graft;
mod isolated_body;
mod rebase;

#[cfg(test)]
use body_pass::fragment_needs_original_source_owner;
pub use isolated_body::analyse_proc_body_isolated;
pub(crate) use isolated_body::reconstruct_proc_scope;

/// Largest proc / method body (in bytes of body text) the per-item
/// decomposition will take on.  Above this the document falls back to a whole-
/// file [`Analyser::analyse`] with [`PerItemFallback::OversizedBody`].
///
/// 256 KiB is deliberately far above any hand-written procedure — roughly
/// 6,000 lines of ordinary Tcl — so no real code base loses the incremental
/// path; it exists to catch *generated* single-body files, which are the only
/// place bodies this size occur and the only place the isolated-body path is a
/// dramatic pessimisation (see the call site in [`Analyser::fill_deferred_bodies`]).
/// Two of tcllib 2.0's 882 documents are above it, and both are generated:
/// `fumagic/filetypes.tcl` (one ~1.2 MB body) and the `i.map` variant of
/// `textutil/wcswidth.tcl` (two ~2.2 MB bodies).
const OVERSIZED_BODY_BYTES: usize = 256 * 1024;

/// Why [`Analyser::analyse_per_item_with`] abandoned the incremental path and
/// paid for a full whole-file walk.
///
/// Every fallback costs a complete re-analysis of the document on **every
/// keystroke** — the per-body memoisation above it is dead weight for a file
/// that trips one of these. Knowing the rate is a perf question; knowing the
/// *distribution* is what says which gate is worth engineering away, so the
/// variants are ordered by where they fire in the pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PerItemFallback {
    /// Source is not a complete script (unbalanced braces / quotes) — mid-edit
    /// state, so this one is expected to fire transiently while typing.
    IncompleteScript,
    /// The selected original source parser cannot answer completeness.
    CompletenessUnavailable,
    /// An inline `tcl-lsp: stub` directive; stub overlays are modelled only on
    /// the full path.
    StubDirective,
    /// A nearest `<dialect>.tcl.stubs` sidecar is present. Its signatures must
    /// reach every isolated proc body, so use the full analyser until the
    /// per-body memo key carries the complete overlay.
    SidecarStub,
    /// Tk checks are live, and their whole-file state (created widgets,
    /// per-parent geometry) is not visible to an isolated body.
    ///
    /// Fires in two places, because Tk activation has two inputs of very
    /// different cost: the `tk` dialect is known at entry and
    /// short-circuits before any work, whereas a `package require Tk` is a
    /// whole-file fact only the walk can establish, so it is checked *after*
    /// the shell + body passes have filled `result.package_requires` via the
    /// registry's `PackageRequire` hook.  The late check pays one wasted
    /// per-item pass on a genuinely-Tk document — the price of never paying a
    /// whole-file re-analysis, on every keystroke, for a document that merely
    /// *mentions* the word `Tk`.
    TkActive,
    /// Ghost-token error recovery engaged.
    GhostRecovery,
    /// A partial (unterminated) command survived segmentation.
    PartialCommand,
    /// An `E…` diagnostic was emitted — `analyse` would have run recovery
    /// machinery the per-item walk only partially reproduces.
    ErrorDiagnostic,
    /// A deferred body is too large for the isolated-body decomposition to be
    /// worth taking ([`OVERSIZED_BODY_BYTES`]).
    OversizedBody,
    /// The same method qualified name is defined more than once.
    DuplicateMethod,
    /// A body declares a link to a qualified sub-namespace variable, or
    /// `namespace import`/`export`s from inside a body
    /// ([`body_needs_enclosing_context`]).
    EnclosingContext,
    /// A body defines a proc whose name is already defined elsewhere.
    DuplicateProcInBody,
    /// A body defines/extends a class whose facts collide with existing ones.
    ClassFactsCollide,
    /// A method body's object-instance tracking could not be replayed.
    MethodInstanceReplay,
    /// Isolated naming facts need genuine full-document source ownership.
    /// Rebasing their spans cannot reissue original name or lookup receipts.
    OriginalSourceOwnership,
    /// §5.4 range targeting is declared (configuration pairs or a
    /// `# tcl-lsp: supports` directive): the range verdicts (W150/W151)
    /// read walk-level state the isolated-body memo key does not carry,
    /// so a declared document takes the full path rather than diverge.
    DeclaredTargets,
    /// The document invokes a definer only the workspace's packs declare.
    /// The shell reads the un-overlaid store (`per_item_setup`),
    /// where that command has no definition-body grammar, so the class it
    /// makes — and every member its body declares — would be invisible;
    /// the full path reads the overlaid store, until the per-body memo key
    /// carries the pack overlay.
    PackDefiner,
}

impl PerItemFallback {
    /// Stable short name for histograms / logs.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IncompleteScript => "incomplete-script",
            Self::CompletenessUnavailable => "completeness-unavailable",
            Self::StubDirective => "stub-directive",
            Self::SidecarStub => "sidecar-stub",
            Self::TkActive => "tk-active",
            Self::GhostRecovery => "ghost-recovery",
            Self::PartialCommand => "partial-command",
            Self::ErrorDiagnostic => "error-diagnostic",
            Self::OversizedBody => "oversized-body",
            Self::DuplicateMethod => "duplicate-method",
            Self::EnclosingContext => "enclosing-context",
            Self::DuplicateProcInBody => "duplicate-proc-in-body",
            Self::ClassFactsCollide => "class-facts-collide",
            Self::MethodInstanceReplay => "method-instance-replay",
            Self::OriginalSourceOwnership => "original-source-ownership",
            Self::DeclaredTargets => "declared-targets",
            Self::PackDefiner => "pack-definer",
        }
    }
}

/// A proc / method body deferred by the shell pass for a second analysis pass.
/// Its scope already exists at `scope_path` (params + instance vars defined);
/// the body pass fills it via an **isolated** analysis (memoisable) and grafts
/// the result back.
#[derive(Debug, Clone)]
pub struct DeferredBody {
    /// Actual parent input, independent of temporal command proofs restored
    /// by the document shell. Isolated analysis explicitly projects this
    /// whole-source configuration to nested body coordinates.
    pub resolved_input: Option<super::ResolvedAnalysisInput>,
    /// Body text (braces stripped), as `analyse_body` expects.
    ///
    /// Shared behind an `Arc` so the LSP query database can intern it into its
    /// per-body memo key without a second copy of every proc body's full source
    /// on every edit (`tcl-lsp-db`'s `ItemBodyKey`).
    pub body_text: std::sync::Arc<str>,
    /// The body's `Str` token (absolute span) — drives offset arithmetic.
    pub body_tok: Token,
    /// Path to the already-created (empty) proc / method scope to fill.
    pub scope_path: Vec<usize>,
    /// `true` for an OO method/constructor/destructor body (analysed in an
    /// isolated `Method` scope with instance variables pre-bound); `false` for a
    /// `proc` body.
    pub is_method: bool,
    /// Mirrors [`super::types::Scope::oo_global_resolution`] for the isolated
    /// rebuild: `true` for a `TclOO` method body (bare commands resolve
    /// globally — object-namespace semantics), `false` for procs and
    /// snit / itcl members.
    pub oo_global_resolution: bool,
    /// Lexical namespace of the proc (e.g. `"::ns"`), or — for a method — the
    /// class's **defining** namespace, used to reconstruct the isolated analysis
    /// context.
    pub namespace: String,
    /// The proc scope's name (as written) — or, for a method, its qualified name
    /// (`::Class::method`) — used for `all_variables` keys and the per-item
    /// duplicate detector.
    pub scope_name: String,
    /// The proc / method's declared parameters (locals in the body).
    pub params: Vec<crate::signature_scan::types::ParamDef>,
    /// Variables pre-bound in the body before it is walked: the class
    /// instance variables of every method body (`variable` declarations at
    /// class level), or the static variables a procedure's static-variable
    /// list declares. Empty for a proc body with no such list.
    ///
    /// The seeded var's real `definition_span` (the `variable v` declaration)
    /// is supplied by the *shell* walk (`oo::walk_method_body`), which the
    /// graft keeps for shell-owned keys (`merge_one_var`); the isolated body
    /// pass below only needs the names, so this stays `Vec<String>` (and salsa
    /// -interning-friendly — see `tcl-lsp-db`'s `ItemBodyKey`).
    pub seeded_variables: Vec<String>,
    /// The whole-file command-mutation trust snapshot, attached by
    /// [`Analyser::fill_deferred_bodies`] to every deferred body whose text
    /// could fold a command substitution
    /// ([`crate::const_subst::body_has_fold_candidate`]), and
    /// `None` for every other body. The isolated pass folds only under this
    /// snapshot — its own body-only source cannot see a `rename` elsewhere
    /// in the file, so an absent snapshot means *distrust everything*.
    /// Canonical/sorted so it keys `tcl-lsp-db`'s `ItemBodyKey`: a rename
    /// appearing (or disappearing) anywhere in the file re-keys exactly the
    /// bodies whose folds it could affect.
    pub command_trust: Option<std::sync::Arc<crate::command_binding::CommandTrustSnapshot>>,
    /// The ensemble `subcommand → resolved target` maps visible to this body
    /// — the entries of
    /// [`super::types::AnalysisResult::ensemble_subcommand_targets`] whose
    /// `namespace ensemble` command **precedes this body** in source order
    /// (matching the whole-file DFS, which walks the body at its definition
    /// point), filtered to ensembles whose tail name occurs in the body text
    /// (so an unrelated ensemble edit re-keys no body) and canonically
    /// sorted (outer by ensemble, inner by subcommand) so it can key
    /// `tcl-lsp-db`'s `ItemBodyKey`.  Attached by
    /// [`Analyser::fill_deferred_bodies`]; empty at construction.  The
    /// isolated pass seeds its own result map from this, so an
    /// `<ensemble> <sub> …` call inside the body records the same
    /// existence-probed subcommand invocation the whole-file walk does —
    /// provenance included, so the body's dispatch words carry the same
    /// `-map`/`-subcommands` tag rename gates on.
    pub ensemble_targets: Vec<(
        String,
        Vec<(String, super::types::EnsembleSubcommandTarget)>,
    )>,
    /// The `-prefixes 0` opt-out for the ensembles in
    /// [`Self::ensemble_targets`] — the visible slice of
    /// [`super::types::AnalysisResult::prefixless_ensembles`], under the same
    /// precedes-the-body and occurs-in-the-body-text filters, sorted so it can
    /// key `tcl-lsp-db`'s `ItemBodyKey`.
    ///
    /// **Travels with `ensemble_targets`, and must keep doing so.** The two
    /// are read together by
    /// [`super::types::AnalysisResult::resolve_ensemble_subcommand`]: the map
    /// says which subcommands exist, this says whether an abbreviation of one
    /// may match. Seeding only the map let an isolated body resolve `g fo`
    /// against a `-prefixes 0` ensemble and record a dispatch invocation the
    /// whole-file walk (and real Tcl) refuse — so the two are seeded and
    /// merged at one site each, never separately.
    pub prefixless_ensembles: Vec<String>,
    /// Mirrors [`super::types::Scope::oo_defining_class`] for the isolated
    /// rebuild: the fully-qualified defining class when this is an
    /// instance-side `TclOO` method body of a statically-named class,
    /// `None` otherwise (class-side members, snit / itcl, procs).
    pub oo_defining_class: Option<String>,
    /// The top child-source visibility receipt when this body was deferred.
    /// Its hash retains original full source/configuration, immutable editing
    /// input, scoped visible slots and separate hidden-token relationships.
    /// It supplies only conditional source warnings; runtime visibility and
    /// entered child frames are independent. A foreign body projection must
    /// decline this receipt rather than reconstruct it from reporting names.
    pub safe_interp_ctx: Option<super::SourceInterpreterVisibilitySnapshot>,
}

impl Analyser {
    /// Per-item analysis entry — byte-identical to [`Analyser::analyse`] for
    /// well-formed input, falling back to it otherwise. Analyses each proc body
    /// directly. See module docs.
    pub fn analyse_per_item(&mut self, source: &str, dialect: &str) -> AnalysisResult {
        let disabled = self.disabled_diagnostics.clone();
        let non_ascii = self.non_ascii_mode;
        let factories = self.workspace_class_factories.clone();
        let workspace = self.workspace_declared_commands.clone();
        let no_declarations = tcl_registry::model::DeclaredSurface::new();
        let mut body_fn = |db: &DeferredBody| {
            analyse_proc_body_isolated(
                db,
                dialect,
                &disabled,
                non_ascii,
                Some(no_declarations.clone()),
                factories.clone(),
                workspace.clone(),
            )
        };
        self.analyse_per_item_with(source, dialect, &mut body_fn)
    }

    /// As [`Self::analyse_per_item`], but each proc / method body's isolated
    /// analysis is produced by `body_fn` — letting a caller (the LSP query DB)
    /// memoise it in salsa so a body edit recomputes only that body.
    /// Byte-identical to `analyse` whenever `body_fn` returns what
    /// [`analyse_proc_body_isolated`] would.
    pub fn analyse_per_item_with(
        &mut self,
        source: &str,
        dialect: &str,
        body_fn: &mut dyn FnMut(&DeferredBody) -> BodyFragment,
    ) -> AnalysisResult {
        self.resolve_walk_environment(dialect);
        if let Some(result) = self.unavailable_analysis_result(dialect) {
            return result;
        }
        self.took_fast_path = false;
        self.per_item_fallback = None;
        // Recovery / stub overlays are only modelled on the full `analyse`
        // path; fall back so the per-item result can never diverge.  Tk's
        // TK100x checks accumulate whole-file state (created widgets, per-parent
        // geometry) that an isolated proc body cannot see, so a Tk document
        // also falls back to full analysis rather than diverge (a parent
        // created outside a proc would otherwise look missing inside it, and a
        // `pack`/`grid` conflict spanning a proc body would never flush).
        // Only the *ambient-placement* half of Tk activation is decidable
        // here; the `package require Tk` half is a whole-file fact
        // established by the walk, and is checked after the body pass below.
        //
        // Evaluated one gate at a time (rather than as one `||` chain) so the
        // telemetry can name which fired — these are checked in cheapest-first
        // order, so the split costs nothing.
        let environment = crate::environment_ingress::resolve_environment(dialect);
        let profile = environment.analyser_profile();
        let completeness = tcl_lexer::script_image_is_complete(
            &tcl_lexer::SourceImage::document(source),
            self.file_lexer_config(),
        );
        let entry_gate = if completeness == Ok(true) {
            if source.contains("tcl-lsp: stub") {
                Some(PerItemFallback::StubDirective)
            } else if super::utils::has_sidecar_stubs(self.file_path.as_deref(), profile) {
                Some(PerItemFallback::SidecarStub)
            } else if environment
                .document_context()
                .ambient_package(super::tk_checks::TK_PACKAGE)
            {
                Some(PerItemFallback::TkActive)
            } else if !self.declared_targets.is_empty()
                || !super::utils::parse_supports_directives(source).is_empty()
            {
                // §5.4 range mode: the range verdicts read whole-walk
                // state the isolated-body key does not carry.
                Some(PerItemFallback::DeclaredTargets)
            } else {
                None
            }
        } else {
            Some(if completeness.is_err() {
                PerItemFallback::CompletenessUnavailable
            } else {
                PerItemFallback::IncompleteScript
            })
        };
        if let Some(reason) = entry_gate {
            self.per_item_fallback = Some(reason);
            return self.analyse(source, dialect);
        }

        // Setup, mirroring `analyse` (gated by the corpus `per_item ==
        // analyse` test).
        let mut commands = self.per_item_setup(source, dialect);

        // Error recovery → fall back to full `analyse` (its ghost-recovery /
        // partial-command handling is not reproduced on the per-item path).
        let ghost = self.apply_ghost_recovery(source, &mut commands);
        if ghost || commands.iter().any(|c| c.is_partial) {
            self.per_item_fallback = Some(if ghost {
                PerItemFallback::GhostRecovery
            } else {
                PerItemFallback::PartialCommand
            });
            return self.fresh_full_analyse(source, dialect);
        }

        // Pass 1: shell (defer proc/method bodies).
        // Whole-file scoped environment (tclpkg manifests) — same seeding as
        // `analyse`, so the per-item path resolves file-scoped directives
        // identically (gated by the corpus `per_item == analyse` test).
        let file_env_pushed = self.seed_file_scope_env(source);
        // Capture the shell's own user-class instance creations too: a
        // top-level `Cls create x` may name a class defined inside a
        // deferred body (invisible to the shell walk), so all creations —
        // shell and body alike — replay post-graft in source order under
        // the definition-precedes-site gate
        // ([`Self::replay_deferred_instances`]).
        self.pending_instances = Some(Vec::new());
        self.pending_bareword_dispatch_sites = Some(Vec::new());
        self.defer_proc_bodies = true;
        self.pack_definer_seen = false;
        self.walk_commands_top_level(&commands, false);
        self.defer_proc_bodies = false;
        if file_env_pushed {
            self.body_scope_stack.pop();
        }
        if self.pack_definer_seen {
            self.per_item_fallback = Some(PerItemFallback::PackDefiner);
            return self.fresh_full_analyse(source, dialect);
        }

        // Tk activation, first opportunity.  A `package require
        // Tk` sits at the top level of essentially every real Tk script, and
        // the shell pass has just walked the whole top level — including
        // `namespace eval` and control-flow bodies — so checking here hands off
        // before the body pass is paid for.  Repeated after the body pass,
        // which is what makes the check *complete*; this one only makes the
        // common case cheap.
        if let Some(tk) = self.tk_activation_fallback(source, dialect) {
            return tk;
        }

        // Pass 2: fill each deferred body.
        // Returns `Err(fallback_result)` for the patterns the isolated-body
        // decomposition can't reproduce byte-for-byte.
        if let Err(fallback) = self.fill_deferred_bodies(source, dialect, body_fn) {
            return *fallback;
        }

        // Tk activation, completed.  `result.package_requires` is now the whole
        // document's: `graft_proc_body` merged the requires each proc/method
        // body contributed, so a `package require Tk` buried inside a body is
        // caught here even though the shell pass could not see it.
        if let Some(tk) = self.tk_activation_fallback(source, dialect) {
            return tk;
        }

        // Tail: cross-item passes, canonicalising order.
        self.record_literal_parameter_definitions();
        self.retain_original_callback_signature_lookups(source);
        self.run_diagnostic_emitters(source);

        // Correctness backstop: a syntax error
        // (`E…` diagnostic) means `analyse` engaged its error-*recovery* machinery
        // (ghost-token re-lexing, recovery segmentation, body-level partial
        // detection) that the per-item walk only partially reproduces — a
        // locally-unbalanced `}` that configured completeness accepts is the
        // classic case.  Re-analyse fully so the result is byte-identical to a
        // from-scratch `analyse`.  Well-formed edits (the common case) carry no
        // E-codes and stay on the fast path.
        if self.result.diagnostics.iter().any(|d| d.code.is_error()) {
            self.per_item_fallback = Some(PerItemFallback::ErrorDiagnostic);
            return self.fresh_full_analyse(source, dialect);
        }

        self.took_fast_path = true;
        let mut result = std::mem::take(&mut self.result);
        result.source_words_memo.clear();
        self.clear_run_state();
        result
    }

    /// Hand off to a full [`Analyser::analyse`] if the walk so far has proved
    /// the document is Tk — otherwise `None`, and the per-item path continues.
    ///
    /// The **exact** activation fact, read from
    /// `result.package_requires`, which the registry's `PackageRequire` hook
    /// populated during the walk — the very fact
    /// [`Analyser::flush_tk_geometry_diagnostics`] gates the TK diagnostics on,
    /// so the two can never disagree.  Tk's `TK100x` checks accumulate whole-file
    /// state (created widgets, per-parent geometry) that an isolated proc body
    /// cannot see, so a Tk document must be analysed whole.
    ///
    /// This replaced a pre-walk conjunction of three independent substring
    /// searches (`package`, `require`, `Tk` — anywhere, in any order, comments
    /// and generated data included).  That guess was ~78% false positives and
    /// forced a whole-file re-analysis on every keystroke for a quarter of the
    /// tcllib corpus's source lines.  Paying one discarded per-item pass on a
    /// *genuinely* Tk document is the trade; the `tk` dialect, the other half
    /// of activation, still short-circuits at entry for no cost at all.
    fn tk_activation_fallback(&mut self, source: &str, dialect: &str) -> Option<AnalysisResult> {
        if !self.has_tk_require() {
            return None;
        }
        self.per_item_fallback = Some(PerItemFallback::TkActive);
        Some(self.fresh_full_analyse(source, dialect))
    }

    /// Per-item setup, mirroring `analyse`: record source/dialect, apply file +
    /// `noqa` suppressions, build the stub overlay, stash a dialect-aware
    /// registry + line offsets, and segment the source with recovery.  Returns
    /// the segmented top-level commands.  (Gated by the corpus `per_item ==
    /// analyse` test.)
    fn per_item_setup(
        &mut self,
        source: &str,
        dialect: &str,
    ) -> Vec<crate::segmenter::SegmentedCommand> {
        use std::collections::HashSet;

        let retained = self.retain_walk_source(source);
        let source: &str = &retained;
        let tk_ambient = self.resolve_walk_environment(dialect);
        self.result.dialect = dialect.to_string();
        self.result.body_lexer_config = Some(self.file_lexer_config());
        self.result.lexical_declaration_advice = self.selected_logical_declaration_advice();
        self.result.resolved_input = Some(self.resolved_analysis_input());
        self.result.library_versions = self.library_versions.clone();
        self.tk_ambient = tk_ambient;
        // The per-item path deliberately accumulates **no** Tk state, unlike
        // `analyse` / `analyse_chunked` / `analyse_commands`.  Everything the
        // accumulation feeds is discarded unless Tk turns out to be active, and
        // when it is active this walk's whole result is thrown away for a full
        // re-analysis (see `analyse_per_item_with`) — so buffering here would
        // only be wasted work, and leaving a stale `true` behind on a reused
        // `Analyser` would let one document's widgets leak into the next one's
        // flush.  Pinned explicitly rather than left to the constructor for
        // exactly that reason.
        self.tk_accumulation_enabled = false;
        // The `# tcl-lsp: package NAME provides …` edges, scanned here for the
        // same reason the whole-file ingresses scan them: the
        // Tk hand-off below reads them *during* the walk, through
        // `has_tk_require`, to decide whether this document needs a full
        // analysis at all. Without this the two strategies disagree — the
        // whole-file walk sees a declared Tk and the per-item one does not.
        self.directive_provides = super::utils::parse_provides_directives(source);
        let file_codes = super::utils::parse_file_suppression(source);
        for code in &file_codes {
            self.disabled_diagnostics.insert(code.clone());
        }
        if !file_codes.is_empty() {
            self.result
                .suppressed_lines
                .insert(-1, file_codes.iter().cloned().collect());
        }

        let (stub_cmds, stub_exprs, outside) =
            super::utils::scan_source_for_stubs_with_outside(source, self.file_lexer_config());
        self.result
            .diagnostics
            .extend(super::utils::outside_stub_diagnostics(outside));
        self.declared_commands = Some(
            self.with_workspace_declarations(super::types::build_declared_surface(&stub_cmds)),
        );
        self.result.stub_commands = stub_cmds;
        self.result.stub_expr_defs = stub_exprs;

        // Full and isolated walks read the same retained generation.
        self.registry = Some(self.profile_registry());
        // The shell owns the complete document's temporal command proofs.
        // Isolated fragments cannot reconstruct this world from their bytes.
        self.head_identities = self.document_command_realm(source);
        self.per_item_realm_inputs = Some((source.to_owned(), self.realm_build_inputs()));
        self.result.command_realm = Some(std::sync::Arc::new(self.head_identities.clone()));
        self.result.original_command_world =
            self.head_identities.original_completed_command_world();
        self.retain_original_comment_suppressions(source);
        self.line_offsets = Some(super::state::compute_line_offsets(source));
        // Same recovery known-command universe as `Analyser::analyse` — see
        // `recovery_known_commands` — so per-item analysis matches the
        // full-file walk byte-for-byte (the corpus `per_item == analyse`
        // test gates this).
        self.recovery_known_commands = super::utils::recovery_known_commands(
            source,
            self.registry.as_deref().expect("registry just stashed"),
            &self.extra_commands,
            self.file_lexer_config(),
        );
        let known: HashSet<&str> = self.recovery_known_commands.iter().collect();
        crate::segmenter::segment_commands_with_recovery_and_config(
            source,
            &known,
            self.file_lexer_config(),
        )
    }
}

/// The original read owner selects name grammar before isolated-body replay.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CapturedBodyGlobalRead {
    pub(super) base_name: String,
    pub(super) element: Option<String>,
    pub(super) span: tcl_lexer::Span,
}

/// The facts produced by analysing one proc body in isolation.  Cloneable +
/// `PartialEq` so the LSP query DB can memoise it (salsa early-cutoff).
#[derive(Clone, PartialEq)]
pub struct BodyFragment {
    result: AnalysisResult,
    proc_scope: super::types::Scope,
    ensembles: std::collections::HashSet<String>,
    pending_arity: Vec<(String, String, bool, super::types::Diagnostic)>,
    /// Version-gated W147 option conflicts the body raised, carried so the
    /// shell decides them against the *whole document's* `package require`
    /// set rather than the fragment's.
    pending_option_conflicts: Vec<super::diagnostics::version_gate::GatedOptionConflict>,
    pending_user_call_arity: Vec<super::types::PendingUserCallArity>,
    pending_ctor_arity: Vec<super::types::PendingCtorArity>,
    pending_next_arity: Vec<super::types::PendingNextArity>,
    var_sites: Vec<super::state::VarCommandSite>,
    cmd_sites: Vec<super::state::CmdCommandSite>,
    /// Qualified (`::`/`static::`) reads that missed the isolated body's empty
    /// enclosing global scope; replayed on the shell's real globals at graft.
    global_reads: Vec<CapturedBodyGlobalRead>,
    /// Definitions the isolated body wrote **into its own root scope** — the
    /// fixed, frame-independent cells an `upvar` `otherVar` word names
    /// (`upvar ::tk::FocusGrab($i) data`, `upvar #0 counter c`).  The graft
    /// merges the fragment's *proc* scope only, so these are replayed onto
    /// the shell's real global scope; without that the cell never entered
    /// the scope tree the navigation providers and
    /// `attach_qualified_var_references` read.
    global_defs: Vec<(String, tcl_lexer::Token, tcl_lexer::Span)>,
    /// W002 (disabled-in-dialect command) sites — always deferred (see
    /// [`super::state::Analyser::pending_disabled_commands`]), so this is
    /// simply the isolated body's own queue: `(command name, call-site
    /// namespace, enforce_order, deferred diagnostic)`, the same shape as
    /// [`Self::pending_arity`]. `rebase_fragment` rebases the diagnostic span
    /// in place; the graft re-defers it onto the shell's queue and the tail
    /// re-checks the shadow against the merged `all_procs` /
    /// `command_aliases` / `renamed_commands` / `all_classes` /
    /// `ensemble_namespaces`.
    disabled_commands: Vec<(String, String, bool, super::types::Diagnostic)>,
    /// W143 (private `::tcl::` namespace) sites — likewise always deferred
    /// (see [`super::state::Analyser::pending_w143`]) and carried in the same
    /// shape, so the tail can apply the whole-file `proc` / `package require`
    /// suppressions an isolated body cannot see.
    private_ns_calls: Vec<super::diagnostics::PendingPrivateTclSourceAdvice>,
    /// Legacy W304 presentation buffer; its final owner clears it rather than
    /// infer substituted values from another scope or source text.
    w304: Vec<(
        tcl_lexer::Token,
        String,
        Vec<super::types::CodeFix>,
        tcl_lexer::Span,
    )>,
    /// Captured `TclOO` instance-creation candidates (`(command, args,
    /// creation namespace, site offset)`); the graft queues them (offset
    /// rebased) for the global source-ordered replay
    /// (`Analyser::replay_deferred_instances`).
    instances: Vec<(String, Vec<String>, String, u32)>,
    /// Captured bareword receiver syntax; graft rebases the sites for final
    /// source-instance selection against the complete original document.
    /// Construction reporting maps do not select a Native receiver.
    bareword_dispatch_sites: Vec<super::state::VarCommandSite>,
    /// The isolated pass's **analyser-side** alias / rename / deletion
    /// tables (`Analyser::command_aliases` / `renamed_commands` /
    /// `alias_offsets` / `rename_offsets` / `deleted_commands`) — parallel
    /// to, and consumed separately from, the `AnalysisResult` copies the
    /// graft already merges.  The tail's W123 resolution reads these
    /// analyser-side maps (`live_tail_names`, `fact_live_*`), and the
    /// whole-file walk populates them from *inside proc bodies* too (an
    /// `interp alias` in one body suppresses W123 on a call in a sibling
    /// body), so the per-item graft must merge them the same way.
    walk_aliases: std::collections::HashMap<String, (String, Vec<String>)>,
    walk_renames: std::collections::HashMap<String, String>,
    walk_alias_offsets: std::collections::HashMap<String, u32>,
    walk_rename_offsets: std::collections::HashMap<String, u32>,
    walk_deleted: std::collections::HashMap<String, u32>,
    /// `$cmd`-head dispatch sites pending settlement in the CFG/SSA
    /// phase — the settlement runs on the shell against the whole-file
    /// compilation unit, so an isolated body's sites are re-queued there
    /// (spans rebased) exactly like `var_sites` / `cmd_sites`.
    const_dispatches: Vec<super::state::ConstDispatchSite>,
    /// `$class`-headed instance-creation sites, same
    /// settle-late discipline as `const_dispatches`.
    instance_class_sites: Vec<super::state::PendingInstanceClassSite>,
    /// Buffered widget/instance dispatch syntax (`.w sub …` / `$w sub …`)
    /// whose head had no selected registry command. Graft rebases these sites
    /// for the shared final diagnostic owners; Native receiver identity and
    /// lookup currency require their own retained receipts.
    widget_sites: Vec<super::diagnostics::widget_command::WidgetDispatchSite>,
    /// Original dispatch syntax awaiting the complete CU word-value owner.
    /// Grafting only shifts recorded token coordinates, never value or lookup
    /// authority; finalisation reissues source selection from the whole image.
    proven_sites: Vec<super::diagnostics::ProvenSite>,
    /// Where inside this body each ensemble the body declares was recorded
    /// (`super::state::Analyser::ensemble_record_offsets`), body-relative.
    /// The graft rebases these to absolute and merges them into the shell so
    /// the tail's `flush_pending_ensemble_subcommand_invocations` can apply
    /// the whole-file DFS's "declaration precedes the call site" visibility
    /// rule to an ensemble created inside a proc body.
    ensemble_offsets: std::collections::HashMap<String, u32>,
    /// The conditional loops the body's walk examined
    /// ([`super::state::Analyser::loop_candidates`]), body-relative until the
    /// graft rebases them, so the shell's CFG/SSA pass resolves a loop inside
    /// a proc body against its unit's branch fact as the whole-file walk does.
    loop_candidates: Vec<super::bounds_checks::LoopTerminationCandidate>,
    /// Every offset-keyed synthetic identity the isolated pass minted
    /// (`@dynns@<off>` / `@dynclass@<off>` / `@autoname@<off>`), with
    /// **body-relative** offsets — the whole fragment is body-relative, so
    /// the memo stays offset-invariant.  [`rebase_fragment`] uses this set
    /// to shift exactly these names (wherever they are embedded — scope
    /// names, map keys, qualified names, resolution candidates) to the
    /// absolute offsets the whole-file walk mints, and never a look-alike
    /// literal from the source.
    minted_synthetics: std::collections::HashSet<String>,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_configured_entry_tests;

#[cfg(test)]
mod original_parent_purpose_tests;
