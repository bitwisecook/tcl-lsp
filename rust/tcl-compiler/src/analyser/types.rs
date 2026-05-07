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

use std::collections::{HashMap, HashSet};

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

impl Severity {
    /// Stable lower-case wire form (`"hint"`, `"suggestion"`,
    /// `"warning"`, `"error"`). Same vocabulary as the
    /// `compiler_checks::Severity` wire form — both LSP layers
    /// consume the same strings.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hint => "hint",
            Self::Suggestion => "suggestion",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
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

impl ScopeKind {
    /// Stable lower-case wire form (`"global"`, `"namespace"`,
    /// `"proc"`). Same vocabulary as the Python `Scope.kind`
    /// string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Namespace => "namespace",
            Self::Proc => "proc",
        }
    }
}

/// A suggested fix for a [`Diagnostic`] — maps to an LSP
/// `TextEdit`.
///
/// Mirrors ``CodeFix`` in ``core/analysis/semantic_model.py``.
/// Populated by emitters that know exactly *what* the user
/// should change (E101 inserts a missing ``{``, E103 inserts a
/// missing ``}``, W123 may suggest a similarly-named command,
/// etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeFix {
    /// Source span the replacement applies to.  An insertion
    /// is a zero-width span anchored at the insertion point.
    pub span: Span,
    /// Text to replace ``span`` with.
    pub new_text: String,
    /// Human-readable description of the fix
    /// (e.g. ``"Insert missing '{'"``).  Empty when omitted.
    pub description: String,
}

/// Diagnostic emitted by the analyser.
///
/// Carries a stable ``code`` (e.g. ``"W210"``), the source
/// [`Span`] the diagnostic anchors to, a one-line ``message``, a
/// [`Severity`], and optional [`CodeFix`] suggestions.
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
    /// Suggested fixes (zero or more).  Empty when no
    /// emitter-supplied fix is available.
    pub fixes: Vec<CodeFix>,
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

/// How a proc parameter is used inside the proc body.
///
/// Mirrors ``ProcArgTrait`` in
/// ``core/analysis/semantic_model.py``.  Drives optimisation,
/// shimmer analysis, taint propagation, and diagnostics — tells
/// downstream passes how a parameter value flows through the
/// proc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcArgTrait {
    /// Argument is eval'd as a script (``eval`` / ``uplevel`` /
    /// ``subst``).
    Eval,
    /// Argument is used as a loop / control body.
    Body,
    /// Argument names a variable that the proc writes (upvar +
    /// set, or a registry-marked write site).
    VarWrite,
    /// Argument names a variable that the proc reads via
    /// ``upvar`` (read-only alias).
    VarRead,
    /// Argument is evaluated as an expression.
    Expr,
    /// Argument is used as the list in a ``foreach`` / ``lmap``.
    LoopList,
}

impl ProcArgTrait {
    /// Stable lower-case name suitable for serialisation.
    /// Mirrors the Python enum's ``.name`` (uppercase) for the
    /// API but lowercases for nicer dict keys.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ProcArgTrait::Eval => "eval",
            ProcArgTrait::Body => "body",
            ProcArgTrait::VarWrite => "var_write",
            ProcArgTrait::VarRead => "var_read",
            ProcArgTrait::Expr => "expr",
            ProcArgTrait::LoopList => "loop_list",
        }
    }
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
    /// Inferred parameter usage traits, keyed by parameter
    /// name.  Populated by ``infer_param_traits`` after the
    /// body walk.  Empty when no traits inferred (parameter
    /// unused, or proc body wasn't statically scannable).
    pub param_traits: HashMap<String, std::collections::HashSet<ProcArgTrait>>,
}

/// Method definition inside a `TclOO` class.
///
/// Mirrors ``MethodDef`` in ``core/analysis/semantic_model.py``.
/// Populated by **C41e1** when the class-body walker lands; the
/// shape lives here from **C41e0** so the class-hierarchy /
/// MRO algorithms have a stable target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodDef {
    /// Method name as written.
    pub name: String,
    /// Parameters parsed from the method's parameter list.
    /// Empty for ``destructor`` (no parameter slot in the syntax).
    pub params: Vec<ParamDef>,
    /// Source span of the name token.
    pub name_span: Span,
    /// Source span of the method body (braces excluded).
    pub body_span: Span,
    /// Method kind: ``"method"`` / ``"classmethod"`` /
    /// ``"forward"`` / ``"constructor"`` / ``"destructor"``.
    pub kind: String,
    /// Visibility: ``"public"`` / ``"private"`` /
    /// ``"unexported"``.
    pub visibility: String,
    /// Doc-comment text harvested from preceding lines.
    pub doc: String,
}

/// `TclOO` property definition.
///
/// Mirrors ``PropertyDef`` in ``core/analysis/semantic_model.py``.
/// Recorded by the OO body walker for ``property`` subcommands
/// inside an ``oo::class create`` / ``oo::define`` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDef {
    /// Property name as written.
    pub name: String,
    /// Source span of the property-name token.
    pub name_span: Span,
    /// Property kind: ``"readable"`` / ``"writable"`` /
    /// ``"readwrite"``.  Defaults to ``"readwrite"`` when ``-kind``
    /// is omitted (matches Python).
    pub kind: String,
    /// True when ``-get BODY`` was supplied.
    pub has_getter: bool,
    /// True when ``-set BODY`` was supplied.
    pub has_setter: bool,
}

/// Class definition record.
///
/// Mirrors ``ClassDef`` in ``core/analysis/semantic_model.py``.
/// **C41e0** lands the structural fields (`superclasses`,
/// `mixins`, `methods`, `class_methods`) needed by the
/// class-hierarchy / MRO algorithms; **C41e1** wires the body
/// walker that populates them.  **C41e3** extends the record
/// with the remaining Python fields (``metaclass``, ``properties``,
/// ``variables``, ``filters``, ``exports``, ``unexports``,
/// ``constructors``, ``destructor``, ``doc``) so the
/// ``_materialise_rust_analysis`` helper can populate the full
/// dataclass shape.
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
    /// Metaclass — one of ``"oo::class"`` / ``"oo::configurable"``
    /// / ``"oo::abstract"`` / ``"oo::singleton"``.  Defaults to
    /// ``"oo::class"`` (matches Python's dataclass default).
    pub metaclass: String,
    /// Direct superclasses in declaration order.  Each entry
    /// is a fully-qualified class name with leading ``::``.
    pub superclasses: Vec<String>,
    /// Class-level mixins in declaration order.  Same naming
    /// convention as `superclasses`.
    pub mixins: Vec<String>,
    /// Instance methods keyed by simple name.
    pub methods: HashMap<String, MethodDef>,
    /// Class methods keyed by simple name.
    pub class_methods: HashMap<String, MethodDef>,
    /// Constructor methods (multiple constructors allowed under
    /// ``oo::configurable``).  Stored in declaration order.
    pub constructors: Vec<MethodDef>,
    /// Destructor method, when one was defined.
    pub destructor: Option<MethodDef>,
    /// Class-level instance variables declared via ``variable``.
    pub variables: Vec<String>,
    /// Configurable properties keyed by name.
    pub properties: HashMap<String, PropertyDef>,
    /// Method filters declared via ``filter``.
    pub filters: Vec<String>,
    /// Methods explicitly exported via ``export``.
    pub exports: HashSet<String>,
    /// Methods explicitly unexported via ``unexport``.
    pub unexports: HashSet<String>,
    /// Doc-comment text harvested from the line(s) above the
    /// ``oo::class create`` / ``oo::define`` statement.
    pub doc: String,
}

impl Default for ClassDef {
    /// Default-construct a [`ClassDef`].
    ///
    /// All names default to empty strings, both spans default to
    /// ``Span::new(0, 0)``, and the metaclass defaults to
    /// ``"oo::class"`` to mirror the Python dataclass default.
    /// Used by handlers that build a class record incrementally
    /// (most common shape — only `name` / `qualified_name` /
    /// `name_span` / `body_span` need explicit values).
    fn default() -> Self {
        let zero = Span::new(0, 0);
        Self {
            name: String::new(),
            qualified_name: String::new(),
            name_span: zero,
            body_span: zero,
            metaclass: "oo::class".to_string(),
            superclasses: Vec::new(),
            mixins: Vec::new(),
            methods: HashMap::new(),
            class_methods: HashMap::new(),
            constructors: Vec::new(),
            destructor: None,
            variables: Vec::new(),
            properties: HashMap::new(),
            filters: Vec::new(),
            exports: HashSet::new(),
            unexports: HashSet::new(),
            doc: String::new(),
        }
    }
}

/// A lexical scope (global, namespace, or proc body).
///
/// Mirrors ``Scope`` in ``core/analysis/semantic_model.py``.
/// The analyser builds a tree of these as it walks; the root is
/// ``AnalysisResult.global_scope``.
///
/// Children are stored inline as ``Vec<Scope>``, so the tree is
/// a strict ownership graph.  The parent link is implicit, held
/// by the analyser's traversal stack
/// (``Analyser::current_scope_path``) rather than embedded as a
/// back-pointer the way Python's ``Scope.parent`` is.  Snapshot /
/// restore (**C41a3**) only needs to copy the result tree, not
/// rewrite back-pointers.
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

/// Analysis result from a user-defined ``unknown`` proc.
///
/// Mirrors ``UnknownProcInfo`` in
/// ``core/analysis/semantic_model.py``.  Populated by **C41e3**
/// when the analyser encounters ``proc unknown {cmd args} {
/// ... }``: the body is lowered to IR and inspected for
/// dispatch shapes (switch arms, ``exec``, ``auto_load``,
/// chains to a saved ``_original_unknown``, case-folding
/// dispatch).  The result gates the W123 (unresolved command)
/// emitter so commands handled by ``unknown`` aren't false-
/// positived.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
// `chains_original` / `empty_stub` / `case_insensitive` /
// `has_pattern_dispatch` / `has_exec` / `has_auto_load` are
// orthogonal facts detected about the body of an ``unknown``
// proc, each consumed independently by the W123 suppression
// logic. The type also crosses the PyO3 serialisation boundary
// (the materialiser unpacks each bool by name), so a bitflags
// migration needs to rewrite the Python-side reader in lockstep
// — deferred to its own chunk.
#[allow(clippy::struct_excessive_bools)]
pub struct UnknownProcInfo {
    /// Command names explicitly dispatched (e.g. switch arm
    /// labels).
    pub dispatch_targets: std::collections::BTreeSet<String>,
    /// Calls a renamed original ``unknown`` (e.g.
    /// ``_original_unknown``).
    pub chains_original: bool,
    /// Body is empty — nothing resolves at all.
    pub empty_stub: bool,
    /// Normalises case before dispatch (all known commands are
    /// valid).
    pub case_insensitive: bool,
    /// Uses glob or regexp switch dispatch — opaque match
    /// semantics.
    pub has_pattern_dispatch: bool,
    /// Calls ``exec`` — opaque external dispatch.
    pub has_exec: bool,
    /// Calls ``auto_load`` — dynamic package loading.
    pub has_auto_load: bool,
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
    /// Package provide records (``package provide NAME ?VERSION?``).
    pub package_provides: Vec<PackageProvide>,
    /// True when a non-literal ``package require`` / ``load`` /
    /// ``auto_path`` mutation has been seen — downstream W123
    /// emission suppresses unknown-command diagnostics under this
    /// flag because the dynamic provider may register the missing
    /// command at runtime.
    pub has_dynamic_providers: bool,
    /// Source-target records.
    pub source_targets: Vec<SignatureSource>,
    /// Command-alias records keyed by qualified alias name.
    pub command_aliases: HashMap<String, SignatureCommandAlias>,
    /// Namespace import records.
    pub namespace_imports: Vec<SignatureNamespaceImport>,
    /// `auto_path` mutations (``lappend auto_path …`` / ``set auto_path …``).
    pub auto_path_entries: Vec<AutoPathEntry>,
    /// Inline ``# stub: NAME ARGS BODY`` directive captures.
    pub stub_commands: Vec<StubCommandDef>,
    /// Inline ``# stub-expr: NAME ARGS`` directive captures.
    pub stub_expr_defs: Vec<StubExprDef>,
    /// `regexp` / `regsub` / `switch -regexp` literal patterns.
    pub regex_patterns: Vec<RegexPattern>,
    /// Per-line ``# noqa: CODE`` suppression map; the ``-1``
    /// sentinel carries top-of-file ``# tcl-lsp: disable=CODE``
    /// directives applying file-wide.
    pub suppressed_lines: HashMap<i32, std::collections::HashSet<String>>,
    /// Analysis result from a user-defined ``unknown`` proc, when
    /// one was seen.  ``None`` when the document didn't define
    /// one (the W123 emitter then runs unconditionally).
    pub unknown_proc_info: Option<UnknownProcInfo>,
}

/// `package provide NAME ?VERSION?` record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageProvide {
    /// Provided package name.
    pub name: String,
    /// Version string when present.
    pub version: Option<String>,
    /// Span of the originating ``package`` token.
    pub range: Span,
}

/// `auto_path` mutation record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoPathEntry {
    /// Raw path text as written.
    pub raw_path: String,
    /// Span of the path argument.
    pub range: Span,
}

/// One parameter declared inside a ``# tcl-lsp: stub NAME {ARGS}``
/// brace block.  Mirrors Python's ``StubArgDef`` in
/// ``core/analysis/semantic_model.py``.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubArgDef {
    /// Argument name as written.  Optional markers (``?…?``) are
    /// stripped before storage.
    pub name: String,
    /// Argument role — one of ``body`` / ``expr`` / ``var`` /
    /// ``var_read`` / ``name`` / ``pattern`` / ``channel`` /
    /// ``value`` (the default when no ``:role`` annotation is
    /// supplied).
    pub role: String,
    /// ``true`` when the source token is wrapped in ``?…?``.
    pub optional: bool,
}

bitflags::bitflags! {
    /// Trailing ``?-flag…?`` flags on a ``# tcl-lsp: stub`` line.
    /// Mirrors the ``barrier`` / ``loop`` / ``pure`` / ``mutator``
    /// / ``unsafe`` / ``scope_alias`` boolean fields on Python's
    /// ``StubCommandDef`` dataclass — packed into a single byte
    /// here because they're an enum-set of orthogonal flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct StubFlags: u8 {
        /// ``-barrier`` — command creates a dynamic barrier.
        const BARRIER     = 1 << 0;
        /// ``-loop`` — command has a loop body.
        const LOOP        = 1 << 1;
        /// ``-pure`` — command is side-effect-free.
        const PURE        = 1 << 2;
        /// ``-mutator`` — command mutates state.
        const MUTATOR     = 1 << 3;
        /// ``-unsafe`` — command is unsafe.
        const UNSAFE      = 1 << 4;
        /// ``-scope_alias`` — command creates a scope alias
        /// (``upvar``-like).
        const SCOPE_ALIAS = 1 << 5;
    }
}

/// Inline `# stub: NAME ARGS BODY` directive capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubCommandDef {
    /// Stub command name.
    pub name: String,
    /// Parsed parameter list from the ``{ARGS}`` brace block
    /// (empty when the block is ``{}``).
    pub args: Vec<StubArgDef>,
    /// Span of the comment line carrying the directive.
    pub range: Span,
    /// Trailing flag set (``-barrier`` / ``-loop`` / ``-pure``
    /// / ``-mutator`` / ``-unsafe`` / ``-scope_alias``).
    pub flags: StubFlags,
}

/// Inline `# stub-expr: NAME ARGS` directive capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubExprDef {
    /// Stub expression-function name.
    pub name: String,
    /// Either ``"function"`` (``stub expr-func``) or
    /// ``"operator"`` (``stub expr-op``).
    pub kind: String,
    /// Number of arguments (functions) or operands (operators).
    /// Defaults to 1 for functions and 2 for operators when the
    /// trailing arity word is absent.
    pub arity: u32,
    /// Span of the comment line carrying the directive.
    pub range: Span,
}

/// `regexp` / `regsub` / `switch -regexp` literal-pattern record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexPattern {
    /// Span of the literal pattern token in source.
    pub range: Span,
    /// Raw pattern text.
    pub pattern: String,
    /// Originating command name (``"regexp"`` / ``"regsub"`` /
    /// ``"switch"``).
    pub command: String,
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
        assert!(r.unknown_proc_info.is_none());
    }

    #[test]
    fn class_def_default_has_oo_class_metaclass() {
        // Mirrors the Python dataclass default — ``metaclass``
        // is the only non-trivial default the rest of the
        // analyser depends on.
        let c = ClassDef::default();
        assert_eq!(c.metaclass, "oo::class");
        assert!(c.constructors.is_empty());
        assert!(c.destructor.is_none());
        assert!(c.variables.is_empty());
        assert!(c.properties.is_empty());
        assert!(c.filters.is_empty());
        assert!(c.exports.is_empty());
        assert!(c.unexports.is_empty());
        assert!(c.doc.is_empty());
    }
}
