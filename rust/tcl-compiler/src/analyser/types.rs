//! Result and record types for the analyser.
//!
//! Mirrors the subset of ``core/analysis/semantic_model.py`` that
//! the analyser actually populates. Per-strip plan: this module
//! starts with the structural shells the rest of C41a needs;
//! later strips fill in the variant fields as their owning handlers
//! land.
//!
//! Field naming follows the Python source 1:1 — UK-spelt
//! identifiers stay UK-spelt, ``snake_case`` Python field names
//! stay ``snake_case`` in Rust.

use std::collections::HashMap;

use tcl_lexer::Span;

use crate::signature_scan::types::{
    ParamDef, SignatureCommandAlias, SignatureCommandInvocation, SignatureNamespaceImport,
    SignaturePackageRequire, SignatureSource,
};

/// Severity of a diagnostic.
///
/// Mirrors ``Severity`` in
/// ``core/analysis/semantic_model.py``; the Rust
/// ``compiler_checks::Severity`` is a similar enum but lives at the
/// compiler-checks layer (taint / GVN / shimmer) rather than the
/// analyser layer. Kept separate to avoid coupling the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// Hint — non-actionable suggestion.
    Hint,
    /// Suggestion — minor improvement opportunity.
    Suggestion,
    /// Warning — likely-incorrect code that still compiles.
    Warning,
    /// Error — definitely-incorrect code.
    Error,
}

/// Lexical scope kind.
///
/// Mirrors the ``Scope.kind`` string in
/// ``core/analysis/semantic_model.py`` — the three scope kinds the
/// analyser ever creates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    /// The top-level ``::`` global scope.
    Global,
    /// A ``namespace eval`` scope.
    Namespace,
    /// A ``proc`` body scope.
    Proc,
}

/// Diagnostic emitted by the analyser.
///
/// Carries a stable ``code`` (e.g. ``"W210"``), the source
/// [`Span`] the diagnostic anchors to, a one-line ``message``, and
/// a [`Severity`]. Replacement / fix-it suggestions land later via
/// a sibling ``CodeFix`` type (filled in **C41d**).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable W-/IRULE-coded identifier.
    pub code: String,
    /// Source span the diagnostic anchors to.
    pub span: Span,
    /// One-line user-facing message.
    pub message: String,
    /// Severity classifier.
    pub severity: Severity,
}

/// Variable definition record.
///
/// Mirrors ``VarDef`` in ``core/analysis/semantic_model.py``.
/// Populated by [`Analyser`](super::Analyser) every time it
/// processes a ``set`` / ``variable`` / ``upvar`` / loop binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarDef {
    /// Variable name (no leading ``$``).
    pub name: String,
    /// Source span of the defining occurrence.
    pub definition_span: Span,
    /// Spans of every read site that resolves to this definition.
    pub references: Vec<Span>,
    /// True when an unused-var warning should still fire even if
    /// the var is exported via a known mechanism (e.g. ``upvar``).
    pub warn_if_unused: bool,
}

/// Proc definition record.
///
/// Mirrors ``ProcDef`` in ``core/analysis/semantic_model.py``.
/// Reuses the [`ParamDef`] type the signature scanner already
/// landed in C40a2 — same param shape, same parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcDef {
    /// Proc name as written (no namespace qualifiers).
    pub name: String,
    /// Fully-qualified proc name with leading ``::``.
    pub qualified_name: String,
    /// Parameter list in declaration order.
    pub params: Vec<ParamDef>,
    /// Source span of the proc-name token.
    pub name_span: Span,
    /// Source span of the proc body (braces excluded).
    pub body_span: Span,
    /// Doc-comment text harvested from the line(s) above the
    /// ``proc`` statement, or empty when none was found.
    pub doc: String,
}

/// Class definition record.
///
/// Mirrors ``ClassDef`` in ``core/analysis/semantic_model.py``.
/// Methods + properties land in **C41e**; this strip seeds the
/// shape so ``handle_oo_class`` has a target to populate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDef {
    /// Class name as written.
    pub name: String,
    /// Fully-qualified class name with leading ``::``.
    pub qualified_name: String,
    /// Source span of the name token.
    pub name_span: Span,
    /// Source span of the class body (braces excluded).
    pub body_span: Span,
}

/// A lexical scope (global, namespace, or proc body).
///
/// Mirrors ``Scope`` in ``core/analysis/semantic_model.py``.
/// The analyser builds a tree of these as it walks; the root is
/// ``AnalysisResult.global_scope``.
///
/// Children are stored as ``Box<Scope>`` so the tree is a
/// strict ownership graph; the parent link is implicit (held by
/// the analyser's traversal stack rather than embedded as a back
/// pointer the way Python's [`Scope.parent`] is). Snapshot /
/// restore (**C41a3**) only needs to copy the result tree, not
/// rewrite back-pointers.
///
/// [`Scope.parent`]: https://example.com (intentionally placeholder)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// Scope kind (global, namespace, proc).
    pub kind: ScopeKind,
    /// Scope identifier — namespace name for namespace/global
    /// scopes, proc qualified name for proc scopes.
    pub name: String,
    /// Body span (braces excluded), `None` for the global scope.
    pub body_span: Option<Span>,
    /// Variables defined directly in this scope.
    pub variables: HashMap<String, VarDef>,
    /// Procs defined directly in this scope.
    pub procs: HashMap<String, ProcDef>,
    /// Classes defined directly in this scope.
    pub classes: HashMap<String, ClassDef>,
    /// Child scopes (in declaration order).
    pub children: Vec<Scope>,
}

impl Scope {
    /// Construct a fresh empty scope.
    #[must_use]
    pub fn new(kind: ScopeKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            body_span: None,
            variables: HashMap::new(),
            procs: HashMap::new(),
            classes: HashMap::new(),
            children: Vec::new(),
        }
    }

    /// Construct the canonical top-level ``::`` global scope.
    #[must_use]
    pub fn global() -> Self {
        Self::new(ScopeKind::Global, "::")
    }
}

/// Complete analysis result for a single document.
///
/// Mirrors ``AnalysisResult`` in
/// ``core/analysis/semantic_model.py``, restricted to the field
/// set the Rust analyser populates. Fields not yet emitted by any
/// strip default to empty / `None` — they're carried in the shape
/// so the `PyO3` binding (**C41f3**) can serialise the full result
/// dict from day one without follow-up plumbing.
#[derive(Debug, Clone, Default)]
pub struct AnalysisResult {
    /// Root scope tree (`::`).
    pub global_scope: Scope,
    /// Procs keyed by qualified name.
    pub all_procs: HashMap<String, ProcDef>,
    /// Classes keyed by qualified name.
    pub all_classes: HashMap<String, ClassDef>,
    /// Free variables (vars defined outside any proc scope) keyed
    /// by qualified name.
    pub all_variables: HashMap<String, VarDef>,
    /// Diagnostics emitted during analysis, in source order.
    pub diagnostics: Vec<Diagnostic>,
    /// Command invocations (lightweight `name + span` records,
    /// matches the [`SignatureCommandInvocation`] shape from
    /// `signature_scan` so cross-feature consumers see one type).
    pub command_invocations: Vec<SignatureCommandInvocation>,
    /// Package require records.
    pub package_requires: Vec<SignaturePackageRequire>,
    /// Source-target records.
    pub source_targets: Vec<SignatureSource>,
    /// Command-alias records keyed by qualified alias name.
    pub command_aliases: HashMap<String, SignatureCommandAlias>,
    /// Namespace import records.
    pub namespace_imports: Vec<SignatureNamespaceImport>,
}

impl Default for Scope {
    fn default() -> Self {
        Self::global()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_default_is_global() {
        let s = Scope::default();
        assert_eq!(s.kind, ScopeKind::Global);
        assert_eq!(s.name, "::");
        assert!(s.variables.is_empty());
        assert!(s.procs.is_empty());
        assert!(s.classes.is_empty());
        assert!(s.children.is_empty());
    }

    #[test]
    fn analysis_result_default_is_empty() {
        let r = AnalysisResult::default();
        assert_eq!(r.global_scope.kind, ScopeKind::Global);
        assert!(r.all_procs.is_empty());
        assert!(r.all_classes.is_empty());
        assert!(r.all_variables.is_empty());
        assert!(r.diagnostics.is_empty());
        assert!(r.command_invocations.is_empty());
        assert!(r.package_requires.is_empty());
        assert!(r.source_targets.is_empty());
        assert!(r.command_aliases.is_empty());
        assert!(r.namespace_imports.is_empty());
    }
}
