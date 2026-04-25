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
    /// **C41a1 stub.** Returns the default empty result.
    /// Real analysis lands in **C41f1** (the orchestration layer)
    /// once C41a-C41e have wired up the per-handler logic.
    ///
    /// `source` is consumed by reference so the analyser can hold
    /// per-walk references back into it; `dialect` is one of
    /// `"tcl"`, `"f5-irules"`, `"irules"`, `"iapps"`, etc.
    pub fn analyse(&mut self, source: &str, dialect: &str) -> AnalysisResult {
        // C41a1: stub. The walker entry, dispatch table, and
        // diagnostic emitters land in subsequent strips.
        let _ = source;
        let _ = dialect;
        std::mem::take(&mut self.result)
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
    fn analyse_stub_returns_empty_result() {
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} {}", "tcl");
        // Stub returns the default empty result for every input.
        assert!(r.all_procs.is_empty());
        assert!(r.diagnostics.is_empty());
        assert!(r.command_invocations.is_empty());
    }

    #[test]
    fn default_constructs_via_new() {
        let a = Analyser::default();
        assert_eq!(a.current_scope_path.len(), 0);
        assert_eq!(a.body_depth, 0);
    }
}
