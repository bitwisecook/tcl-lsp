//! The [`Analyser`] struct and its per-walk state.
//!
//! Mirrors ``_AnalyserBase.__init__`` in
//! ``core/analysis/_analyser/_core.py:39-105``. Where Python uses
//! cooperative multiple inheritance to share state across mixin
//! handlers, the Rust analyser is a single struct whose methods
//! are grouped across modules (``commands.rs``, ``proc.rs``,
//! ``oo.rs``, ``diagnostics/``, …) but all operate on the same
//! ``&mut Analyser``. Each Python ``self._field`` becomes a Rust
//! struct field one-for-one.
//!
//! Per-walk handler logic is filled in by **C41b** onwards;
//! C41a1 lands the struct shape + constructor + a stub
//! [`Analyser::analyse`] entry point that returns an empty result.

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;

use super::types::AnalysisResult;

/// One entry in [`Analyser::var_command_sites`] —
/// `(var_name, method_name?, cmd_token_span, in_method)`.
///
/// Mirrors the Python ``self._var_command_sites`` tuple in
/// ``_AnalyserBase.__init__``. Used by the W307
/// (variable-as-command misuse) post-pass that lands in **C41d3**.
#[derive(Debug, Clone)]
pub struct VarCommandSite {
    /// Variable name used as a command head (no leading ``$``).
    pub var_name: String,
    /// Optional method name when the call shape is
    /// ``$obj method args…``.
    pub method_name: Option<String>,
    /// Span of the command-head token.
    pub cmd_span: Span,
    /// True when the call site is inside a class method body.
    pub in_method: bool,
}

/// One entry in [`Analyser::cmd_command_sites`] —
/// `([cmd] text, method_name?, cmd_token_span, in_method)`.
///
/// Mirrors ``self._cmd_command_sites`` — same shape as
/// [`VarCommandSite`] except the head is a command-substitution
/// rather than a variable.
#[derive(Debug, Clone)]
pub struct CmdCommandSite {
    /// Text of the bracketed command substitution (no brackets).
    pub cmd_text: String,
    /// Optional method name.
    pub method_name: Option<String>,
    /// Span of the command-head token.
    pub cmd_span: Span,
    /// True when inside a class method body.
    pub in_method: bool,
}

/// Single-pass Tcl analyser.
///
/// Mirrors the Python ``Analyser`` class composed in
/// ``core/analysis/_analyser/__init__.py``. Constructed once per
/// document, walked end-to-end, then dropped.
///
/// **Field documentation refers to the Python source field of the
/// same name** — see ``_AnalyserBase.__init__`` for the full
/// rationale per field. Comments here only call out where the
/// Rust shape diverges from Python (e.g. ``ns_cache`` keys).
#[derive(Debug)]
pub struct Analyser {
    /// Public accumulator returned by [`Analyser::analyse`].
    pub result: AnalysisResult,
    /// Path through ``result.global_scope`` to the currently-active
    /// scope. Each entry is the index into the parent's
    /// ``children`` list; an empty path means "currently in the
    /// global scope". Python uses a back-pointer (`Scope.parent`)
    /// for the same job; Rust prefers an index path so the scope
    /// tree stays a strict ownership graph.
    pub current_scope_path: Vec<usize>,
    /// Full source text being analysed.
    ///
    /// Set at the top of [`Self::analyse`] (and the chunked
    /// entries in **C41f2**) and read by handlers that need to
    /// re-slice the outer source — recovery (**C41e4** / **C41e5**)
    /// and CFG/SSA diagnostic emission (**C41d**). Mirrors
    /// ``self._source`` in
    /// ``core/analysis/_analyser/_core.py:43``.
    pub source: String,
    /// Active dialect name (``"tcl"``, ``"f5-irules"``,
    /// ``"f5-iapps"``, etc.).  Set at the top of
    /// [`Self::analyse`].  Mirrors ``active_dialect()`` in
    /// Python — handlers that need to compute dialect-specific
    /// command sets (W113 shadow check, dialect-only command
    /// gating in C41d) read this directly.  Empty string when
    /// dialect was not specified.
    pub dialect: String,
    /// Diagnostic codes that should not be emitted.
    pub disabled_diagnostics: HashSet<String>,
    /// Last seen comment text, for proc / class doc-comment
    /// harvesting.
    pub last_comment: String,
    /// Source-file path (for the LSP `Diagnostic.uri` field), or
    /// `None` when analysing in-memory text.
    pub file_path: Option<String>,
    /// Per-scope const-string tracker:
    /// ``scope_kind_path → { var_name → (value, span) }``.
    /// Python keys this on ``id(scope)``; Rust uses the path
    /// vector so snapshot/restore doesn't have to remap pointers.
    /// Filled in by **C41a4**.
    pub const_strings: HashMap<Vec<usize>, HashMap<String, (String, Span)>>,
    /// Variables known to contain regex patterns:
    /// ``(scope_path, var_name)``. Filled in by **C41a4**.
    pub regex_vars: HashSet<(Vec<usize>, String)>,
    /// iRules: enclosing ``when EVENT`` name.
    pub current_event: Option<String>,
    /// Cached set of built-in command names for redefined-builtin
    /// detection. `None` until first lookup; filled lazily.
    pub builtin_names: Option<HashSet<String>>,
    /// The dialect ``builtin_names`` was built for, for cache
    /// invalidation.
    pub builtin_dialect: Option<String>,
    /// Conditional-nesting depth — incremented on entry to
    /// `if` / `catch` / `try` arms, used to mark
    /// ``package require`` records as ``conditional=true``.
    pub conditional_depth: u32,
    /// Body-nesting depth — incremented on entry to a braced
    /// body. Used by **C41d** for top-level-only command checks.
    pub body_depth: u32,
    /// Command-alias records:
    /// ``alias_name → (target_cmd, prepended_args)``.
    pub command_aliases: HashMap<String, (String, Vec<String>)>,
    /// Variable-as-command call sites; resolved post-walk by W307
    /// (lands in **C41d3**).
    pub var_command_sites: Vec<VarCommandSite>,
    /// Command-substitution-as-command call sites; same dispatch
    /// as [`Self::var_command_sites`] but for ``[cmd] args``
    /// shapes.
    pub cmd_command_sites: Vec<CmdCommandSite>,
    /// Cache: ``scope_path → namespace string`` for
    /// ``namespace_from_scope``. Cleared on snapshot restore.
    pub ns_cache: HashMap<Vec<usize>, String>,
    /// Namespaces where ``namespace ensemble create`` was seen —
    /// their tail names become valid commands.
    pub ensemble_namespaces: HashSet<String>,
    /// Vars where ``oo::objdefine`` was applied — the per-instance
    /// method table may extend the class definition.
    pub objdefined_vars: HashSet<String>,
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
    pub registry: Option<tcl_registry::CommandRegistry>,
    /// When `true`, the proc handler runs the deep-recursive
    /// pass of [`super::param_traits::infer_param_traits_deep`]
    /// after the shallow pass and unions the results via
    /// [`super::param_traits::merge_traits`].  Off by default —
    /// the shallow pass is fast enough for synchronous analysis
    /// and catches the common patterns; the deep pass is
    /// intended for asynchronous use behind the `S*` call-graph
    /// / symbol-graph / dataflow-graph / semantic-graph builders.
    /// Mirrors Python's `deep_param_traits=True` opt-in surface.
    pub deep_param_traits: bool,
    /// Per-document stub-command overlay built at the top of
    /// [`Self::analyse`] from `result.stub_commands` via
    /// [`super::types::build_stub_overlay`].  Lets analyser /
    /// compiler queries see user-declared `# tcl-lsp: stub`
    /// commands as first-class members of the command surface
    /// without mutating the global [`tcl_registry::CommandRegistry`].
    /// `None` outside an active analysis run.  Mirrors the
    /// `_stub_signatures_var` `ContextVar` from Python's
    /// `core/commands/registry/runtime.py` but tied to the
    /// (single-threaded) analyser instead of a thread-local.
    pub stub_overlay: Option<tcl_registry::stub_overlay::StubOverlay>,
    /// Sorted byte offsets of every ``\n`` in [`Self::source`],
    /// precomputed at the top of [`Self::analyse`] /
    /// [`Self::analyse_chunked`] / [`Self::analyse_commands`] so
    /// per-command line-number lookups (notably
    /// [`super::utils::apply_preceding_noqa`] which runs once
    /// per command) cost ``O(log N)`` instead of ``O(N)`` per
    /// call.  ``None`` outside an active analysis run.
    pub line_offsets: Option<Vec<usize>>,
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

    /// Construct an analyser with a fixed set of diagnostic codes
    /// disabled (e.g. `"W210"`, `"W211"`).
    #[must_use]
    pub fn with_disabled_diagnostics(disabled: HashSet<String>) -> Self {
        Self {
            result: AnalysisResult::default(),
            current_scope_path: Vec::new(),
            source: String::new(),
            dialect: String::new(),
            disabled_diagnostics: disabled,
            last_comment: String::new(),
            file_path: None,
            const_strings: HashMap::new(),
            regex_vars: HashSet::new(),
            current_event: None,
            builtin_names: None,
            builtin_dialect: None,
            conditional_depth: 0,
            body_depth: 0,
            command_aliases: HashMap::new(),
            var_command_sites: Vec::new(),
            cmd_command_sites: Vec::new(),
            ns_cache: HashMap::new(),
            ensemble_namespaces: HashSet::new(),
            objdefined_vars: HashSet::new(),
            unresolved_commands_emitted: false,
            registry: None,
            deep_param_traits: false,
            stub_overlay: None,
            line_offsets: None,
        }
    }

    /// Analyse a Tcl source for the given dialect, returning a
    /// fully-populated [`AnalysisResult`].
    ///
    /// **C41f1 baseline.** Drives end-to-end:
    ///
    /// 1. Pre-scans the leading comment block for
    ///    `# tcl-lsp: disable=CODE` directives via
    ///    [`super::utils::parse_file_suppression`].
    /// 2. Segments `source` with the registry's known-commands
    ///    set (uses Seg2 recovery) so unclosed delimiters mid-file
    ///    don't drop later declarations.
    /// 3. Walks each segmented command through
    ///    [`Self::process_command`].
    ///
    /// Body recursion (proc bodies, namespace bodies, control-flow
    /// arm bodies) is **not** wired in this baseline — handlers in
    /// C41b1-b8 record the body span but don't recurse. Body walks
    /// land per-handler in **C41c** / **C41e**. Diagnostic emission
    /// (W210, W211, W120 etc.) lands in **C41d**.
    ///
    /// `source` is consumed by reference so the analyser can hold
    /// per-walk references back into it; `dialect` is one of
    /// `"tcl"`, `"f5-irules"`, `"irules"`, `"iapps"`, etc. (kept in
    /// the analyser's per-walk state via [`Self::current_event`]
    /// elsewhere; this entry just records it for future use).
    pub fn analyse(&mut self, source: &str, dialect: &str) -> AnalysisResult {
        use std::collections::HashSet;
        use tcl_registry::CommandRegistry;

        // Stash the source so handlers (recovery in C41e4/e5,
        // diagnostic emitters in C41d) can re-slice it.  Mirrors
        // ``self._source = source`` in
        // ``core/analysis/_analyser/_core.py``.
        self.source = source.to_string();
        self.dialect = dialect.to_string();
        // File-suppression pre-scan: merge codes from any
        // top-of-file ``# tcl-lsp: disable=CODE`` directives into
        // ``self.disabled_diagnostics`` so later emitter passes
        // honour them. The constructor-provided
        // ``disabled_diagnostics`` set (LSP user-config) and the
        // file-directive set are unioned — both sources should
        // take effect.
        //
        // Python keeps file-level suppression in
        // ``result.suppressed_lines[-1]`` (a per-line map keyed
        // by a sentinel ``-1`` for file-wide). Until the per-line
        // suppression machinery lands (alongside the diagnostic
        // emitters in C41d), the simpler "merge into
        // ``disabled_diagnostics``" route gives directives the
        // intended effect at the analyser-internal level. C41d
        // can revisit if a per-line distinction becomes load-bearing.
        let file_codes = super::utils::parse_file_suppression(source);
        for code in &file_codes {
            self.disabled_diagnostics.insert(code.clone());
        }
        if !file_codes.is_empty() {
            // Mirror Python's ``result.suppressed_lines[-1]``
            // sentinel so downstream consumers (the LSP
            // suppression filter, code-action UX) see the
            // file-wide directive set in one place.
            self.result
                .suppressed_lines
                .insert(-1, file_codes.iter().cloned().collect());
        }
        // Pre-scan for next-line ``# noqa`` suppressions (issue
        // #306, Python ``b66f8f9d`` / ``ceb190fc``).  Handles
        // orphaned noqa at the tail of a brace body and noqa
        // before a comment line that itself generates a
        // diagnostic.  Merges into ``suppressed_lines`` alongside
        // the command-attached ``apply_preceding_noqa`` pass that
        // runs per segmented command in the dispatch loop below.
        merge_noqa_line_suppressions(
            &mut self.result.suppressed_lines,
            super::utils::parse_noqa_line_suppressions(source),
        );
        // Inline ``# tcl-lsp: stub …`` block scan.  After
        // capturing the parsed records, build the per-document
        // overlay so analyser / compiler queries see the
        // user-declared stubs as first-class commands (without
        // mutating the global registry).  Mirrors the
        // `_stub_signatures_var` `ContextVar` wiring in Python's
        // `core/commands/registry/runtime.py`.
        let (stub_cmds, stub_exprs) = super::utils::scan_source_for_stubs(source);
        self.stub_overlay = Some(super::types::build_stub_overlay(&stub_cmds));
        self.result.stub_commands = stub_cmds;
        self.result.stub_expr_defs = stub_exprs;

        // Segment with Seg2 recovery so an unclosed delimiter
        // mid-file doesn't drop later top-level declarations.
        // Build the dialect-aware registry once and stash on
        // ``self`` so per-command handlers (registry-driven body
        // iteration in ``process_command``) reuse it.
        let mut registry = CommandRegistry::build_default();
        if let Some(d) = tcl_registry::prelude::DialectSet::parse(&self.dialect) {
            registry.load_dialect(d);
        }
        self.registry = Some(registry);
        // Precompute newline offsets once for ``O(log N)``
        // byte-offset → line-number lookup in
        // ``apply_preceding_noqa`` (which runs per command and
        // would otherwise be ``O(N)`` per call).
        self.line_offsets = Some(compute_line_offsets(source));
        let known_commands: HashSet<&str> = self
            .registry
            .as_ref()
            .expect("registry just stashed")
            .command_names()
            .collect();
        let commands = crate::segmenter::segment_commands_with_recovery(source, &known_commands);
        drop(known_commands);

        // Walk each command through the dispatcher. Body recursion
        // (proc bodies, namespace bodies, control-flow arms) is
        // C41c / C41e work; this baseline only walks the top level.
        // **C41e4** wires ``recover_stray_close_bracket``;
        // **C41e5** wires ``recover_missing_open_brace`` (for
        // switch with forgotten body brace), ``detect_stolen_close_brace``
        // (E103), and the generic E200 partial-command emitter.
        let total = commands.len();
        let mut cmd_idx: usize = 0;
        while cmd_idx < total {
            let cmd_ref = &commands[cmd_idx];
            if cmd_ref.argv.is_empty() {
                cmd_idx += 1;
                continue;
            }
            if cmd_ref.is_partial {
                if !self.detect_stolen_close_brace(cmd_ref) {
                    self.emit_partial_command_diagnostic(cmd_ref);
                }
                cmd_idx += 1;
                continue;
            }
            let mut cmd = cmd_ref.clone();
            self.recover_stray_close_bracket(&mut cmd);
            let consumed = self.recover_missing_open_brace(&mut cmd, &commands, cmd_idx);
            let single = cmd.single_token_word.clone();
            // ``# noqa[: CODE,...]`` directives in
            // ``cmd.preceding_comment`` suppress diagnostics on
            // the *following* command's line range.  Mirrors
            // ``core/analysis/_analyser/_core.py:285-303``.
            if let Some(line_offsets) = self.line_offsets.as_deref() {
                super::utils::apply_preceding_noqa(
                    &cmd,
                    line_offsets,
                    &mut self.result.suppressed_lines,
                );
            }
            // Mirrors ``self._last_comment = cmd.preceding_comment``
            // in ``core/analysis/_analyser/_core.py``: handlers
            // that consume a preceding comment (proc, oo::class)
            // ``std::mem::take`` it; everything else clears it on
            // the next command.
            self.last_comment = cmd.preceding_comment.clone().unwrap_or_default();
            self.process_command(&cmd.texts, &cmd.argv, &single, &[]);
            self.record_arg_var_reads(&cmd, &[]);
            cmd_idx += 1 + consumed;
        }

        // **C41d1.** Run the diagnostic-emission orchestrator
        // and the post-pass filters.  Mirrors the tail of
        // ``Analyser.analyse`` in
        // ``core/analysis/_analyser/_core.py:380-384``:
        //
        // 1. ``emit_unresolved_command_diagnostics`` — C41d4.
        // 2. ``emit_variable_usage_diagnostics`` — hook landed
        //    in C41d1 (currently no-op).
        // 3. ``emit_cfg_ssa_diagnostics(source)`` — orchestrator
        //    landed in C41d1 (currently inert; per-emitter
        //    dispatch lands in C41d2-d7).
        // 4. ``apply_disabled_diagnostics`` — filter codes the
        //    caller asked to silence (also covers the
        //    file-suppression directives merged at the top of
        //    ``analyse``).
        // 5. ``dedupe_diagnostics`` — drop exact duplicates and
        //    the line-based suppression pairs.
        let mut diag_registry = CommandRegistry::build_default();
        if let Some(d) = tcl_registry::prelude::DialectSet::parse(&self.dialect) {
            diag_registry.load_dialect(d);
        }
        self.emit_unresolved_command_diagnostics(&diag_registry);
        self.emit_variable_usage_diagnostics();
        self.emit_cfg_ssa_diagnostics(source);
        self.apply_disabled_diagnostics();
        self.dedupe_diagnostics();

        let result = std::mem::take(&mut self.result);
        self.clear_run_state();
        result
    }

    /// **C41f2** — Analyse pre-segmented commands chunk-by-chunk
    /// and capture per-chunk snapshots.
    ///
    /// Mirrors `Analyser.analyse_chunked` in
    /// `core/analysis/_analyser/_core.py:195-239`.  Used by the
    /// LSP for incremental document re-analysis: when the user
    /// types into the document, dirty chunks are re-segmented
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
    /// is one chunk's worth of commands.  `dialect` mirrors the
    /// argument to [`Self::analyse`].  Stub-pre-scan (Python's
    /// `scan_source_for_stubs`) is deferred — that path lands
    /// alongside the ``stub_commands`` field which the Rust
    /// `AnalysisResult` doesn't carry yet.
    pub fn analyse_chunked(
        &mut self,
        source: &str,
        chunk_commands: Vec<Vec<crate::segmenter::SegmentedCommand>>,
        dialect: &str,
    ) -> (AnalysisResult, Vec<super::snapshot::AnalyserSnapshot>) {
        use tcl_registry::CommandRegistry;
        self.source = source.to_string();
        self.dialect = dialect.to_string();
        self.unresolved_commands_emitted = false;
        self.ns_cache.clear();

        let file_codes = super::utils::parse_file_suppression(source);
        for code in &file_codes {
            self.disabled_diagnostics.insert(code.clone());
        }
        if !file_codes.is_empty() {
            // Mirror ``analyse``'s ``result.suppressed_lines[-1]``
            // sentinel so downstream consumers see file-wide
            // ``# tcl-lsp: disable=`` directives via the same
            // surface regardless of which entry point dispatched
            // the analyse (Copilot review on PR #371).
            self.result
                .suppressed_lines
                .insert(-1, file_codes.iter().cloned().collect());
        }
        // Next-line ``# noqa`` pre-scan — see ``analyse`` for
        // rationale (issue #306).
        merge_noqa_line_suppressions(
            &mut self.result.suppressed_lines,
            super::utils::parse_noqa_line_suppressions(source),
        );

        // Build + stash the dialect-aware registry so
        // ``process_command`` 's body-iteration loop has access
        // to it on every chunked / incremental call (the entry
        // point used by the LSP's incremental update path).
        // Without this, body recursion silently no-ops.  Same
        // for the ``line_offsets`` index used by
        // ``apply_preceding_noqa``.
        let mut registry = CommandRegistry::build_default();
        if let Some(d) = tcl_registry::prelude::DialectSet::parse(&self.dialect) {
            registry.load_dialect(d);
        }
        self.registry = Some(registry);
        self.line_offsets = Some(compute_line_offsets(source));

        let mut snapshots: Vec<super::snapshot::AnalyserSnapshot> =
            Vec::with_capacity(chunk_commands.len());
        for cmds in chunk_commands {
            self.analyse_commands_inner(&cmds);
            snapshots.push(self.snapshot());
        }

        // Same diagnostic-emission tail as ``analyse``.
        let mut diag_registry = CommandRegistry::build_default();
        if let Some(d) = tcl_registry::prelude::DialectSet::parse(&self.dialect) {
            diag_registry.load_dialect(d);
        }
        self.emit_unresolved_command_diagnostics(&diag_registry);
        self.emit_variable_usage_diagnostics();
        self.emit_cfg_ssa_diagnostics(source);
        self.apply_disabled_diagnostics();
        self.dedupe_diagnostics();

        let result = std::mem::take(&mut self.result);
        self.clear_run_state();
        (result, snapshots)
    }

    /// **C41f2** — Analyse pre-segmented commands without
    /// re-segmenting `source`.
    ///
    /// Mirrors `Analyser.analyse_commands` in
    /// `core/analysis/_analyser/_core.py:241-272`.  This is
    /// the single-chunk variant used by the LSP's incremental
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
        use tcl_registry::CommandRegistry;
        self.source = source.to_string();
        self.dialect = dialect.to_string();
        self.unresolved_commands_emitted = false;
        self.ns_cache.clear();

        let file_codes = super::utils::parse_file_suppression(source);
        for code in &file_codes {
            self.disabled_diagnostics.insert(code.clone());
        }
        if !file_codes.is_empty() {
            // ``-1`` sentinel parity with ``analyse`` — see the
            // matching block in ``analyse_chunked`` (Copilot
            // review on PR #371).
            self.result
                .suppressed_lines
                .insert(-1, file_codes.iter().cloned().collect());
        }
        // Next-line ``# noqa`` pre-scan — see ``analyse`` for
        // rationale (issue #306).
        merge_noqa_line_suppressions(
            &mut self.result.suppressed_lines,
            super::utils::parse_noqa_line_suppressions(source),
        );

        // Same registry + line-index prelude as
        // ``analyse_chunked`` — see that doc-comment.  Without
        // these the registry-driven body loop in
        // ``process_command`` silently skips body recursion on
        // the incremental path.
        let mut registry = CommandRegistry::build_default();
        if let Some(d) = tcl_registry::prelude::DialectSet::parse(&self.dialect) {
            registry.load_dialect(d);
        }
        self.registry = Some(registry);
        self.line_offsets = Some(compute_line_offsets(source));

        self.analyse_commands_inner(commands);

        if finalise {
            let mut diag_registry = CommandRegistry::build_default();
            if let Some(d) = tcl_registry::prelude::DialectSet::parse(&self.dialect) {
                diag_registry.load_dialect(d);
            }
            self.emit_unresolved_command_diagnostics(&diag_registry);
            self.emit_variable_usage_diagnostics();
            self.emit_cfg_ssa_diagnostics(source);
            self.apply_disabled_diagnostics();
            self.dedupe_diagnostics();
        }

        let result = std::mem::take(&mut self.result);
        self.clear_run_state();
        result
    }

    /// Inner dispatch loop shared by [`Self::analyse_chunked`]
    /// and [`Self::analyse_commands`].  Walks pre-segmented
    /// commands at the current scope path.
    ///
    /// Mirrors `_analyse_commands_inner` in
    /// `core/analysis/_analyser/_core.py:274-354`.  The Rust
    /// port is much smaller than Python's because the
    /// var-read recording, CMD-substitution recursion, and
    /// recovery hooks land per-strip in C41c / C41e.  This
    /// helper covers the dispatch portion that's load-bearing
    /// for incremental analysis.
    fn analyse_commands_inner(&mut self, commands: &[crate::segmenter::SegmentedCommand]) {
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
                // **C41e5** parity — partial commands surface
                // E103 / E200 in the chunked path too so the
                // LSP shows parse errors during incremental
                // analysis.
                if !self.detect_stolen_close_brace(cmd_ref) {
                    self.emit_partial_command_diagnostic(cmd_ref);
                }
                cmd_idx += 1;
                continue;
            }
            // **C41e4 + C41e5.** Repair stray ``]`` and missing
            // ``{`` in a clone of the segmented command before
            // dispatch — chunked analysis keeps the original
            // snapshot copies untouched so re-runs are
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
            self.process_command(&cmd.texts, &cmd.argv, &cmd.single_token_word, &scope_path);
            self.record_arg_var_reads(&cmd, &scope_path);
            cmd_idx += 1 + consumed;
        }
    }

    /// Resolve (and cache) the set of built-in command names for
    /// the active dialect.
    ///
    /// Mirrors the inline cache in
    /// ``_AnalyserProcMixin._handle_proc``
    /// (``core/analysis/_analyser/_proc.py:71-74``) — the
    /// registry is built once per dialect and the resulting name
    /// set is held on ``self.builtin_names`` for subsequent
    /// proc / class registrations to consult without rebuilding.
    /// Used by **W113** (proc shadows built-in) at proc-emit time
    /// and the **C41d** emitters that gate on built-in vs
    /// user-defined.
    ///
    /// The dialect string is parsed via ``DialectSet::parse``;
    /// unknown dialect names fall through to the core registry
    /// (TCL / stdlib / tcllib only) — same fallback Python uses
    /// implicitly when ``REGISTRY.command_names(dialect)``
    /// returns just the built-in set.
    pub fn builtin_command_names(&mut self) -> &std::collections::HashSet<String> {
        use tcl_registry::prelude::DialectSet;
        use tcl_registry::CommandRegistry;
        if self.builtin_dialect.as_deref() != Some(self.dialect.as_str())
            || self.builtin_names.is_none()
        {
            let mut registry = CommandRegistry::build_default();
            if let Some(d) = DialectSet::parse(&self.dialect) {
                registry.load_dialect(d);
            }
            let names: std::collections::HashSet<String> =
                registry.command_names().map(str::to_string).collect();
            self.builtin_names = Some(names);
            self.builtin_dialect = Some(self.dialect.clone());
        }
        // Safe: ``builtin_names`` was just set if it was missing.
        self.builtin_names
            .as_ref()
            .expect("builtin_names populated above")
    }

    /// Reset transient run state so the next ``analyse`` call
    /// starts from a clean slate.  Called at the end of every
    /// public entry point (``analyse`` / ``analyse_chunked`` /
    /// ``analyse_commands``).
    fn clear_run_state(&mut self) {
        self.registry = None;
        self.line_offsets = None;
    }
}

/// Precompute newline byte offsets for ``source``.  The returned
/// vector is sorted ascending — the byte offset of each ``\n``
/// in source order.  Callers (notably
/// ``apply_preceding_noqa``) use ``slice::partition_point`` /
/// ``binary_search`` on this vector to convert a byte offset to
/// a 0-based line number in ``O(log N)`` instead of a per-call
/// linear scan.
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
/// Mirrors the inline-merge block Python writes in
/// ``Analyser.analyse`` / ``analyse_chunked`` / ``analyse_commands``
/// after each call to ``parse_noqa_line_suppressions``.  Used by all
/// three Rust entry points so the merge logic stays in one place.
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
        assert!(a.ns_cache.is_empty());
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
    fn analyse_records_multiple_top_level_commands() {
        let mut a = Analyser::new();
        let r = a.analyse("set x 1\nproc foo {} {}\nglobal a b", "tcl");
        assert!(r.all_procs.contains_key("::foo"));
        assert!(r.global_scope.variables.contains_key("x"));
        assert!(r.global_scope.variables.contains_key("a"));
        assert!(r.global_scope.variables.contains_key("b"));
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
        // ``parse_noqa_line_suppressions`` pre-scan.  Mirrors the
        // Python merge block in ``_core.py`` post commit
        // ``ceb190fc`` (issue #306).  Line 0 carries the ``# noqa``
        // directive so line 1 should be in the map.
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
            !r.diagnostics.iter().any(|d| d.code == "W113"),
            "W113 should be silenced by file-suppression directive",
        );
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
        let w113s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W113").collect();
        assert_eq!(
            w113s.len(),
            2,
            "two distinct ``proc set`` definitions → two distinct W113s at different spans",
        );
    }

    #[test]
    fn analyse_records_source_text_for_handler_re_slicing() {
        // Handlers in C41c / C41d / C41e re-slice ``self.source``
        // via spans returned by the segmenter; the field must be
        // populated at the top of ``analyse``.
        let mut a = Analyser::new();
        let _ = a.analyse("set x 1", "tcl");
        assert_eq!(a.source, "set x 1");
    }

    #[test]
    fn analyse_records_dialect_for_w113_and_emitter_use() {
        // Handlers (W113 shadow check, dialect-only emitters in
        // C41d) read ``self.dialect`` directly.  The field must
        // be populated at the top of ``analyse``.
        let mut a = Analyser::new();
        let _ = a.analyse("", "f5-irules");
        assert_eq!(a.dialect, "f5-irules");
    }

    #[test]
    fn builtin_command_names_caches_per_dialect() {
        // First lookup populates the cache; subsequent lookups
        // with the same dialect return the same set.
        let mut a = Analyser::new();
        a.dialect = "tcl".to_string();
        let initial_len = a.builtin_command_names().len();
        // ``set`` is a core built-in across all dialects.
        assert!(a.builtin_command_names().contains("set"));
        // Cache invalidation: switching dialect rebuilds.
        a.dialect = "f5-irules".to_string();
        let irules_len = a.builtin_command_names().len();
        assert!(
            irules_len > initial_len,
            "f5-irules should add commands beyond core (got {irules_len} vs {initial_len})",
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
        a.dialect = "tcl".to_string();
        let r = a.analyse_commands(source, &commands, "tcl", false);
        // W113 was emitted by handle_proc but the tail didn't
        // run, so apply_disabled_diagnostics / dedupe didn't
        // touch the diag list.  The diag is still there.
        assert!(r.diagnostics.iter().any(|d| d.code == "W113"));
    }

    #[test]
    fn default_constructs_via_new() {
        let a = Analyser::default();
        assert_eq!(a.current_scope_path.len(), 0);
        assert_eq!(a.body_depth, 0);
    }

    // -- tcllib `<NS>::import <ALIAS>` wrapper detection
    //
    // Mirror the relevant cases in
    // ``tests/test_namespace_imports.py``
    // (``test_tcllib_import_wrapper_is_conjectured`` +
    // ``test_import_wrapper_alias_relative_to_current_namespace``)
    // against the Rust port so the ``namespace_imports`` supplement
    // guard can retire.

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
        // matches Python's ``"$" not in alias and "[" not in alias``
        // guard.
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

    // -- ``command_aliases`` parity tests
    //
    // Mirror the relevant cases in tests/test_analyser.py
    // (``test_alias_chain_not_resolved``, ``test_alias_redefinition_overwrites``)
    // to pin the no-transitive-chain behaviour the
    // ``alias-chains`` chunk relies on when retiring the
    // ``rust.command_aliases`` Python supplement merge.

    #[test]
    fn analyse_alias_chain_records_each_step_independently() {
        // Mirrors ``test_alias_chain_not_resolved``:
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
        // Mirrors ``test_alias_redefinition_overwrites``: the
        // second declaration wins.
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
    // Mirror the relevant cases in
    // ``tests/test_analyser.py``'s switch / regex tests.  The
    // pattern arms whose token is a literal are recorded as
    // ``RegexPattern { command = "switch" }``; ``default`` and
    // var / cmd-sub patterns are skipped (variable patterns are
    // the regex-vars chunk's territory).

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
        // available, so the arm is dropped (matches Python's
        // ``const_val is not None`` guard in
        // ``_proc.py:325-335``).
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
    // literal stored by a preceding ``set var "..."``.  Mirrors
    // Python's ``_lookup_const_string`` branch in
    // ``_commands.py:511-541`` (regexp / regsub) and
    // ``_proc.py:319-348`` (switch -regexp Form 2).

    #[test]
    fn analyse_regexp_resolves_var_pattern_to_const_string() {
        let mut a = Analyser::new();
        let r = a.analyse("set p {^foo}\nregexp $p $line\n", "tcl");
        // Two records: the use site (the ``$p`` token) and the
        // defining ``set`` value (mirrors
        // ``_record_defining_set_as_regex``).
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
        // pattern arg is dropped (matches Python).
        let mut a = Analyser::new();
        let r = a.analyse("regexp $p $line\n", "tcl");
        assert!(r.regex_patterns.is_empty());
    }

    // -- ``postpass`` chunk: W105 unbraced-body emitter
    //
    // Mirror the relevant cases from
    // ``tests/test_analyser.py`` / ``tests/test_w105*.py`` against
    // the Rust port so the W105 path retires from the Python
    // ``run_compiler_checks`` post-pass.

    #[test]
    fn analyse_emits_w105_for_unbraced_if_body_with_substitution() {
        let mut a = Analyser::new();
        let r = a.analyse("if {$cond} \"puts $x\"\n", "tcl");
        let w105: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W105").collect();
        assert!(!w105.is_empty(), "expected W105, got {:?}", r.diagnostics);
        // Substitution-bearing bodies are flagged at error severity.
        assert!(matches!(w105[0].severity, crate::analyser::Severity::Error));
    }

    #[test]
    fn analyse_skips_w105_for_braced_if_body() {
        // Braced ``{ ... }`` body — no W105.
        let mut a = Analyser::new();
        let r = a.analyse("if {$cond} { puts $x }\n", "tcl");
        assert!(!r.diagnostics.iter().any(|d| d.code == "W105"));
    }

    #[test]
    fn analyse_emits_w105_for_unbraced_while_body_var() {
        // ``while {$cond} $body`` — Var-token body is still an
        // unbraced body with substitution.  Mirrors Python's
        // ``_has_substitution(..., tok)`` which treats VAR / CMD
        // tokens as substitutions for the W105 check, so the
        // diagnostic fires at ERROR severity.
        let mut a = Analyser::new();
        let r = a.analyse("while {$cond} $body\n", "tcl");
        let w105: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W105").collect();
        assert!(!w105.is_empty(), "expected W105, got {:?}", r.diagnostics);
        assert!(matches!(w105[0].severity, crate::analyser::Severity::Error));
    }

    // -- ``postpass`` chunk: W110 string-compare-in-expr emitter
    //
    // Mirrors ``tests/test_checks.py::TestStringCompareInExpr``
    // against the Rust port so the W110 path retires from the
    // Python ``run_compiler_checks`` post-pass.

    #[test]
    fn analyse_emits_w110_for_string_eq_in_if_condition() {
        let mut a = Analyser::new();
        let r = a.analyse("if {$x == \"foo\"} {puts yes}\n", "tcl");
        let w110: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W110").collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
        assert!(w110[0].message.contains("eq"), "got {:?}", w110[0].message);
        assert!(matches!(w110[0].severity, crate::analyser::Severity::Hint));
    }

    #[test]
    fn analyse_emits_w110_for_string_ne_in_if_condition() {
        let mut a = Analyser::new();
        let r = a.analyse("if {$x != \"bar\"} {puts no}\n", "tcl");
        let w110: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W110").collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
        assert!(w110[0].message.contains("ne"), "got {:?}", w110[0].message);
    }

    #[test]
    fn analyse_no_w110_for_numeric_compare() {
        // ``$x == 42`` — numeric literal on the right, no string
        // operand, should not fire.
        let mut a = Analyser::new();
        let r = a.analyse("if {$x == 42} {puts yes}\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W110"),
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
            !r.diagnostics.iter().any(|d| d.code == "W110"),
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
        let w110: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W110").collect();
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
            !r.diagnostics.iter().any(|d| d.code == "W110"),
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
        let w110: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W110").collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    }

    #[test]
    fn analyse_w110_fires_for_boolean_string_literal() {
        // ``$x == "true"`` — boolean-spelled string literal still
        // counts as ExprString.
        let mut a = Analyser::new();
        let r = a.analyse("if {$x == \"true\"} {puts yes}\n", "tcl");
        let w110: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W110").collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    }

    #[test]
    fn analyse_w110_fires_through_unary_negation() {
        // ``!($x == "foo")`` — W110 walks through ExprUnary.
        let mut a = Analyser::new();
        let r = a.analyse("if {!($x == \"foo\")} {puts no}\n", "tcl");
        let w110: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W110").collect();
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
        let w110: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W110").collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
        assert_eq!(w110[0].fixes.len(), 0, "got {:?}", w110[0].fixes);
    }

    #[test]
    fn analyse_w110_fires_on_while_condition() {
        // ``while {EXPR} {body}`` — EXPR-role is at index 0.
        let mut a = Analyser::new();
        let r = a.analyse("while {$x == \"foo\"} { break }\n", "tcl");
        let w110: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W110").collect();
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
        let w110: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W110").collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    }

    #[test]
    fn analyse_w110_fires_on_multi_arg_expr_command() {
        // ``expr $x == "foo"`` (no braces, multiple argv slots) —
        // matches Python's ``expr_text = " ".join(args)`` special
        // case.
        let mut a = Analyser::new();
        let r = a.analyse("expr $x == \"foo\"\n", "tcl");
        let w110: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W110").collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    }

    #[test]
    fn analyse_w110_no_fire_on_for_clean_condition() {
        // ``for {set i 0} {$i < 10} {incr i} {body}`` — no ``==``
        // anywhere, but ensure the EXPR-role dispatch on ``for``
        // doesn't crash and produces no W110.
        let mut a = Analyser::new();
        let r = a.analyse("for {set i 0} {$i < 10} {incr i} { break }\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W110"),
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
        let w110: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W110").collect();
        assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    }

    // -- ``postpass`` chunk: W302 catch-without-result-var emitter
    //
    // Mirrors the IRCatch arm of ``_check_statement`` in
    // ``core/compiler/compiler_checks.py:491-504`` against the Rust
    // port so the W302 path retires from the Python
    // ``run_compiler_checks`` post-pass.

    #[test]
    fn analyse_emits_w302_for_catch_without_result_var() {
        let mut a = Analyser::new();
        let r = a.analyse("catch { puts hi }\n", "tcl");
        let w302: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W302").collect();
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
            !r.diagnostics.iter().any(|d| d.code == "W302"),
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
            !r.diagnostics.iter().any(|d| d.code == "W302"),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_no_w302_for_multi_token_catch_body() {
        // ``catch pre$x`` — multi-token-word body lands on
        // Python's ``IRBarrier`` "catch with dynamic body" path
        // in ``_lower_catch`` (``arg_single[0]`` is false), so no
        // IRCatch is built and W302 never fires.  The Rust
        // emitter mirrors that suppression by gating on
        // ``arg_single[0]``.
        let mut a = Analyser::new();
        let r = a.analyse("catch pre$x\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W302"),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_w302_anchors_at_command_range() {
        // The W302 span runs from the catch keyword through (at
        // least) the body argument's content, mirroring Python's
        // ``stmt.range`` (the IRCatch's full source range).  The
        // closing brace's inclusion depends on the lexer's
        // ``Str``-token end convention; what matters for the LSP
        // UX is that the span starts at ``catch`` and covers the
        // body text rather than just the catch keyword.
        let mut a = Analyser::new();
        let src = "catch { puts hi }\n";
        let r = a.analyse(src, "tcl");
        let w302: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W302").collect();
        assert_eq!(w302.len(), 1);
        let span = w302[0].span;
        let start = span.start() as usize;
        let end = span.end() as usize;
        let text = &src[start..end];
        assert!(text.starts_with("catch"), "span starts at {text:?}");
        assert!(text.contains("puts hi"), "span text {text:?}");
    }

    // -- ``postpass`` chunk: W001 unknown-subcommand emitter
    //
    // Mirrors the SubcommandSig branch of ``_check_arity`` in
    // ``core/compiler/compiler_checks.py:580-643`` against the
    // Rust port so the W001 path retires from the Python
    // ``run_compiler_checks`` post-pass.

    #[test]
    fn analyse_emits_w001_for_unknown_string_subcommand() {
        let mut a = Analyser::new();
        let r = a.analyse("string bogus $x\n", "tcl");
        let w001: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W001").collect();
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
        let w001: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W001").collect();
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
            !r.diagnostics.iter().any(|d| d.code == "W001"),
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
            !r.diagnostics.iter().any(|d| d.code == "W001"),
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
            !r.diagnostics.iter().any(|d| d.code == "W001"),
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
            !r.diagnostics.iter().any(|d| d.code == "W001"),
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
            !r.diagnostics.iter().any(|d| d.code == "W001"),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_w001_anchors_at_cmd_plus_subcommand_range() {
        let mut a = Analyser::new();
        let src = "string bogus $x\n";
        let r = a.analyse(src, "tcl");
        let w001: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W001").collect();
        assert_eq!(w001.len(), 1);
        let span = w001[0].span;
        let text = &src[span.start() as usize..span.end() as usize];
        assert_eq!(text, "string bogus", "got {text:?}");
    }

    #[test]
    fn analyse_emits_w001_for_unknown_dict_subcommand() {
        // ``dict`` is also a SubcommandSig command — confirm
        // dispatch isn't ``string``-specific.
        let mut a = Analyser::new();
        let r = a.analyse("dict froob $d $k\n", "tcl");
        let w001: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W001").collect();
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
            let w001: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W001").collect();
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

    // -- ``postpass`` chunk: E004 malformed-if emitter
    //
    // Mirrors the ``IRBarrier`` arm of ``_check_statement`` in
    // ``core/compiler/compiler_checks.py:506-525`` — fires
    // ``Severity::Error`` when an ``if`` invocation's structural
    // shape doesn't match
    // ``if COND BODY ?elseif COND BODY ...? ?else BODY?``.
    // Detection is analyser-side (mirrors W302 / W001 dispatch
    // pattern) rather than via IR-walk.

    #[test]
    fn analyse_emits_e004_for_extra_words_after_else() {
        // ``if {1} { a } else { b } extra`` — Python's
        // ``_lower_if`` produces an IRBarrier with reason
        // ``'extra words after "else" clause'``.
        let mut a = Analyser::new();
        let r = a.analyse("if {1} { a } else { b } extra\n", "tcl");
        let e004: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "E004").collect();
        assert_eq!(e004.len(), 1, "got {:?}", r.diagnostics);
        assert!(
            e004[0].message.contains("Extra words after \"else\""),
            "got {:?}",
            e004[0].message
        );
        assert!(matches!(e004[0].severity, crate::analyser::Severity::Error));
    }

    #[test]
    fn analyse_emits_e004_for_bare_else_without_body() {
        // ``if {1} { a } else`` — bare ``else`` keyword with no
        // body.  Python's ``_lower_if`` produces ``"malformed if
        // else clause"``.
        let mut a = Analyser::new();
        let r = a.analyse("if {1} { a } else\n", "tcl");
        let e004: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "E004").collect();
        assert_eq!(e004.len(), 1, "got {:?}", r.diagnostics);
        assert!(
            e004[0].message.contains("Malformed 'if'"),
            "got {:?}",
            e004[0].message
        );
    }

    #[test]
    fn analyse_emits_e004_for_condition_without_body() {
        // ``if {1}`` — condition with no body following.
        let mut a = Analyser::new();
        let r = a.analyse("if {1}\n", "tcl");
        let e004: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "E004").collect();
        assert_eq!(e004.len(), 1, "got {:?}", r.diagnostics);
        assert!(
            e004[0].message.contains("Malformed 'if'"),
            "got {:?}",
            e004[0].message
        );
    }

    #[test]
    fn analyse_emits_e004_for_then_keyword_without_body() {
        // ``if {1} then`` — condition + ``then`` keyword without
        // body.  Mirrors Python's ``"malformed if clause"``.
        let mut a = Analyser::new();
        let r = a.analyse("if {1} then\n", "tcl");
        let e004: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "E004").collect();
        assert_eq!(e004.len(), 1, "got {:?}", r.diagnostics);
        assert!(
            e004[0].message.contains("Malformed 'if'"),
            "got {:?}",
            e004[0].message
        );
    }

    #[test]
    fn analyse_emits_e004_for_if_with_only_else() {
        // ``if else { x }`` — no condition+body clause produced
        // before the else, so Python's post-walk
        // ``if not clauses`` check fires with ``"malformed if"``.
        let mut a = Analyser::new();
        let r = a.analyse("if else { x }\n", "tcl");
        let e004: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "E004").collect();
        assert_eq!(e004.len(), 1, "got {:?}", r.diagnostics);
        assert!(
            e004[0].message.contains("Malformed 'if'"),
            "got {:?}",
            e004[0].message
        );
    }

    #[test]
    fn analyse_no_e004_for_valid_if() {
        // ``if {1} { a }`` — single-clause without else.  No E004.
        let mut a = Analyser::new();
        let r = a.analyse("if {1} { a }\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "E004"),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_no_e004_for_valid_if_else() {
        // ``if {1} { a } else { b }`` — well-formed.  No E004.
        let mut a = Analyser::new();
        let r = a.analyse("if {1} { a } else { b }\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "E004"),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_no_e004_for_valid_if_elseif_chain() {
        // ``if {a} { x } elseif {b} { y } else { z }`` — full
        // shape.  No E004.
        let mut a = Analyser::new();
        let r = a.analyse("if {$a} { x } elseif {$b} { y } else { z }\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "E004"),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_no_e004_for_if_with_then_keyword() {
        // ``if {1} then { a }`` — explicit ``then`` keyword is
        // accepted by both Tcl and Python's lowerer.  No E004.
        let mut a = Analyser::new();
        let r = a.analyse("if {1} then { a }\n", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "E004"),
            "got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn analyse_e004_anchors_at_full_command_range() {
        // Span runs from the ``if`` keyword through the last arg
        // token's end, mirroring Python's ``cmd.range``.
        let mut a = Analyser::new();
        let src = "if {1} { a } else { b } extra\n";
        let r = a.analyse(src, "tcl");
        let e004: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "E004").collect();
        assert_eq!(e004.len(), 1);
        let span = e004[0].span;
        let text = &src[span.start() as usize..span.end() as usize];
        assert!(text.starts_with("if"), "span starts at {text:?}");
        assert!(text.contains("extra"), "span text {text:?}");
    }

    // -- ``postpass`` chunk: W304 missing-option-terminator emitter
    //
    // Mirrors `tests/test_checks.py::TestMissingOptionTerminator`
    // against the Rust port.  Resolution profile lives in
    // ``tcl-registry``; tristate severity / two-diagnostic origin /
    // code-fix logic lives in ``analyser/diagnostics.rs``.

    fn w304_diags(src: &str) -> Vec<crate::analyser::types::Diagnostic> {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        r.diagnostics
            .into_iter()
            .filter(|d| d.code == "W304")
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
        // `-`, so the OFF gate suppresses regardless of the
        // command's WARN_WITHOUT_TERMINATOR trait.  Mirrors Python.
        let diags = w304_diags("regexp {(a+)+$} $text\n");
        assert!(diags.is_empty(), "got {diags:?}");
    }

    #[test]
    fn analyse_emits_w304_for_regexp_literal_dash_pattern() {
        // ``regexp {-[0-9]+} $text`` — the literal pattern starts
        // with `-` so the positional scanner treats it as an
        // unknown option and lands on the next positional
        // (``$text``).  Diagnostic still fires (mirrors Python's
        // ``test_regexp_literal_dash_pattern_warns`` which only
        // asserts ``len(diags) == 1``); severity comes from the
        // dynamic-var INFO path because the diag anchors on
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
        // ``set X "datagroup"; switch $X { default { ... } }`` — the
        // variable resolves to a literal value that doesn't start
        // with `-`, so severity drops to INFO and a second "origin"
        // diagnostic anchors at the literal's range.
        let src = "set totp_key_storage \"datagroup\"\n\
                   switch $totp_key_storage {\n\
                   \x20\x20\x20\x20default { puts ok }\n\
                   }\n";
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

    // -- ``postpass`` chunk: W101 eval-string-concat emitter
    //
    // Mirrors `tests/test_checks.py::TestEvalStringConcat` against
    // the Rust port.  Canonical-list-idiom suppression lives in the
    // registry (``is_canonical_list_command``); substitution-detection
    // approximation lives in the analyser.

    fn w101_diags(src: &str) -> Vec<crate::analyser::types::Diagnostic> {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        r.diagnostics
            .into_iter()
            .filter(|d| d.code == "W101")
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
        // Regression for PR #290 review (Codex bot, P2):
        // ``eval foo{bar}`` is a multi-token word (Esc + Str joined,
        // ``single_token_word == false``) that contains no Var/Cmd
        // substitution — Python's reference check only fires on
        // actual VAR/CMD tokens, so this must not trigger W101.
        // The fix replaces the multi-token-word-implies-substitution
        // heuristic with a brace/backslash-aware source-byte scan
        // that looks for unescaped ``$`` / ``[`` outside ``{...}``.
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

    // -- ``recovery`` chunk: body-walk and nested-cmd improvements
    //
    // Verify ``when EVENT { body }`` recurses (registry
    // ``arg_role_resolver`` now records BODY at the last index)
    // and that braced expr args (``Str`` tokens) have their
    // outer braces unwrapped before the nested-``[cmd]`` scan
    // (otherwise the scanner skips the entire braced region
    // opaquely).

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
        // which entry point dispatched (Copilot review on PR #371).
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
        // Same parity assertion through ``analyse_commands`` — the
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
}
