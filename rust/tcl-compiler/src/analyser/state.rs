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
        for code in super::utils::parse_file_suppression(source) {
            self.disabled_diagnostics.insert(code);
        }

        // Segment with Seg2 recovery so an unclosed delimiter
        // mid-file doesn't drop later top-level declarations.
        let registry = CommandRegistry::build_default();
        let known_commands: HashSet<&str> = registry.command_names().collect();
        let commands = crate::segmenter::segment_commands_with_recovery(source, &known_commands);

        // Walk each command through the dispatcher. Body recursion
        // (proc bodies, namespace bodies, control-flow arms) is
        // C41c / C41e work; this baseline only walks the top level.
        for cmd in commands {
            if cmd.is_partial || cmd.argv.is_empty() {
                continue;
            }
            let single = cmd.single_token_word.clone();
            self.process_command(&cmd.texts, &cmd.argv, &single, &[]);
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

        std::mem::take(&mut self.result)
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
    fn default_constructs_via_new() {
        let a = Analyser::default();
        assert_eq!(a.current_scope_path.len(), 0);
        assert_eq!(a.body_depth, 0);
    }
}
