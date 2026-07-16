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

//! Result and record types for the analyser.
//!
//! These are the subset of the semantic model that the analyser
//! actually populates. Each handler fills in the variant fields of
//! the records it owns as it walks.

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;

use crate::signature_scan::types::{
    ParamDef, SignatureCommandAlias, SignatureCommandInvocation, SignatureNamespaceImport,
    SignaturePackageRequire, SignatureSource,
};

pub use tcl_core_types::DiagCode;
/// Severity of a diagnostic — the shared [`tcl_core_types::Severity`] so the
/// analyser, compiler-checks, and LSP/CLI layers speak one type.
pub use tcl_core_types::Severity;

/// Lexical scope kind — the scope kinds the analyser ever creates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    /// The top-level ``::`` global scope.
    Global,
    /// A ``namespace eval`` scope.
    Namespace,
    /// A ``proc`` body scope.
    Proc,
    /// An ``uplevel #0 { … }`` body scope.  The script runs in the
    /// global frame, so this scope's locals belong to a global-rooted
    /// frame rather than the enclosing proc — completion / definition
    /// see globals + this scope's locals, not the proc's locals.
    Uplevel,
    /// A `TclOO` `method` / `constructor` / `destructor` body scope.
    /// Like [`Self::Proc`] but the body runs as an object method: its
    /// formal parameters and the class's instance ``variable``s are
    /// pre-bound, and ``$self`` / ``my`` self-dispatch is recognised.
    Method,
}

impl ScopeKind {
    /// Stable lower-case wire form (`"global"`, `"namespace"`,
    /// `"proc"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Namespace => "namespace",
            Self::Proc => "proc",
            Self::Uplevel => "uplevel",
            Self::Method => "method",
        }
    }
}

/// A suggested fix for a [`Diagnostic`] — maps to an LSP `TextEdit`.
///
/// Re-exported from [`crate::irules_checks`] (the canonical, lower-level
/// definition shared with the iRules-flow / compiler-checks layer) so the
/// analyser and those passes speak one `CodeFix` type.  Populated by
/// emitters that know exactly *what* the user should change (E101
/// inserts a missing ``{``, E103
/// inserts a missing ``}``, W123 may suggest a similarly-named command, etc.).
pub use crate::irules_checks::CodeFix;

/// Diagnostic emitted by the analyser.
///
/// Carries a stable ``code`` (e.g. ``"W210"``), the source
/// [`Span`] the diagnostic anchors to, a one-line ``message``, a
/// [`Severity`], and optional [`CodeFix`] suggestions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable W-/IRULE-coded identifier.
    pub code: DiagCode,
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

/// One same-file user-call arity candidate, buffered during the command
/// walk for post-walk resolution against same-file procs / `TclOO`
/// forwards / `interp alias` / static `rename` targets — the set the
/// registry-only [`super::diagnostics::validity`] arity check can't see
/// (see `Analyser::resolve_indirect_call_target`).  Distinct from
/// [`super::state::Analyser::pending_arity`] (the registry-command
/// candidate queue): this one is queued for *every* call, independent of
/// whether `cmd_name` also resolves to a registry signature, since a
/// user proc/alias/rename can shadow a builtin name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUserCallArity {
    /// Command name as written at the call site — also the diagnostic's
    /// display name (a same-file call has no subcommand-style split).
    pub cmd_name: String,
    /// Call-site resolution namespace
    /// (`Analyser::command_resolution_namespace`).
    pub ns: String,
    /// `false` inside a proc/method body (definitions there are visible
    /// regardless of textual order, since bodies only run after the
    /// whole file has loaded); `true` at top level (order-gated —
    /// mirrors `pending_arity`'s identical field).
    pub enforce_order: bool,
    /// Offset of the command-name token, for the top-level order gate.
    pub call_off: u32,
    /// Full diagnostic span (command head through the last argument).
    pub full_span: Span,
    /// Lower-bound positional argument count (exact when
    /// `positional_any_expand` is `false`).
    pub nargs_min: usize,
    /// Whether any positional word is `{*}`-expanded — when true,
    /// `nargs_min` is a lower bound only, so E002 ("too few") can never
    /// fire (expansion may still supply the missing arguments at run
    /// time), but E003 ("too many") can still fire when even that lower
    /// bound already exceeds the max; matches the identical convention
    /// in the registry-command arity check.
    pub positional_any_expand: bool,
    /// Widened source span of each argument word (closer included), so the
    /// flush — which only then knows the resolved arity's `max` — can
    /// anchor E003 on the surplus run and target the removal fix.  Empty
    /// when the call has a `{*}` expansion (the surplus run is ambiguous).
    pub arg_spans: Vec<Span>,
    /// Widened end of the command-head word: the removal fix's deletion
    /// start when every positional argument is surplus (`max == 0`).
    pub head_end: u32,
}

/// Which `TclOO` instantiation form introduced a [`PendingCtorArity`] —
/// `oo::class` (and every other `IS_OO_METACLASS` command) inherits all
/// three from its own metaclass protocol, and an ordinary class inherits
/// them from `oo::class` in turn, so `ClassName new/create/createWithNamespace
/// ?args?` all reach the same constructor. Each form has a different count
/// of mandatory leading words that are *not* part of the constructor's own
/// argument list — confirmed against tclsh 9.0.4 and matching
/// `oo_class_arg_roles`'s identical word layout for the sibling
/// class-*definition* shapes (`oo::class create Name body` / `new body` /
/// `createWithNamespace Name ::ns body`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtorForm {
    /// `ClassName new ?args?` — every word after `new` is a constructor arg.
    New,
    /// `ClassName create name ?args?` — the mandatory object-name word is
    /// not part of the constructor's own arguments.
    Create,
    /// `ClassName createWithNamespace name ::ns ?args?` — the mandatory
    /// object-name and namespace words are not part of the constructor's
    /// own arguments.
    CreateWithNamespace,
}

impl CtorForm {
    /// Count of mandatory leading words to fold into the constructor's
    /// arity bound before comparing it against the call site — see
    /// [`Self`]'s own doc comment.
    #[must_use]
    pub fn extra_leading_words(self) -> u16 {
        match self {
            CtorForm::New => 0,
            CtorForm::Create => 1,
            CtorForm::CreateWithNamespace => 2,
        }
    }

    /// The keyword as written at the call site, for the diagnostic's
    /// display name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CtorForm::New => "new",
            CtorForm::Create => "create",
            CtorForm::CreateWithNamespace => "createWithNamespace",
        }
    }
}

/// A queued `TclOO` constructor-call arity candidate — `ClassName new
/// ?args?` / `ClassName create name ?args?` / `ClassName createWithNamespace
/// name ::ns ?args?` — mirroring [`PendingUserCallArity`]'s architecture
/// exactly: queued unconditionally whenever a call's first word is
/// literally one of those three keywords (regardless of whether the head
/// even resolves to a class), and resolved post-walk
/// ([`super::state::Analyser::flush_ctor_arity_diagnostics`]) once
/// `all_classes` — and thus the class hierarchy a constructor may be
/// inherited through — is fully populated.  A candidate that doesn't
/// resolve to a locally-known class, or resolves to one with no explicit
/// constructor anywhere in its MRO (`TclOO`'s default constructor accepts
/// any argument count), is silently dropped at flush time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCtorArity {
    /// Class name as written at the call site (`Dog`, `::ns::Dog`).
    pub class_name: String,
    /// Call-site resolution namespace
    /// (`Analyser::command_resolution_namespace`).
    pub ns: String,
    /// Same order-gating convention as [`PendingUserCallArity::enforce_order`]
    /// — a top-level `Dog new` must have `Dog`'s definition lexically
    /// precede it; a call inside a proc/method body is not order-gated.
    pub enforce_order: bool,
    /// Which of `new`/`create`/`createWithNamespace` this call used.
    pub form: CtorForm,
    /// Offset of the class-name token, for the top-level order gate.
    pub call_off: u32,
    /// Full diagnostic span (class-name head through the last argument).
    pub full_span: Span,
    /// Lower-bound positional count of the words *after* the keyword (for
    /// `create`/`createWithNamespace`, this includes their mandatory
    /// leading words).
    pub nargs_min: usize,
    /// Whether any positional word is `{*}`-expanded — same convention as
    /// [`PendingUserCallArity::positional_any_expand`].
    pub positional_any_expand: bool,
}

/// A queued `TclOO` `next` / `nextto` call-site arity candidate.
///
/// Unlike [`PendingUserCallArity`] / [`PendingCtorArity`] (queued
/// unconditionally and resolved by *name*), this is queued only when the
/// call site is lexically inside a method body — the callee is never
/// named at the call site at all; it is derived entirely from *where*
/// the call sits (`Analyser::current_method_context`). Resolved post-walk
/// ([`super::state::Analyser::flush_next_arity_diagnostics`]) once
/// `all_classes` is fully populated, via
/// [`super::class_hierarchy::ClassHierarchy::next_provider`]. A candidate
/// whose enclosing method has no further provider along the MRO (`next`
/// past the end of the chain, or a `nextto` target that isn't a locally
/// known class) is silently dropped — see the same abstention convention
/// documented on [`PendingCtorArity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingNextArity {
    /// Qualified name of the class whose method body the call sits in —
    /// always the class that *textually declares* this method, since a
    /// method's `Method` scope is only ever created while walking that
    /// class's own body.
    pub class_qualified: String,
    /// Simple name of the enclosing method (`next`/`nextto` always
    /// re-invoke the *same* method name, never a different one).
    pub method_name: String,
    /// `nextto`'s explicit target class as written at the call site
    /// (`None` for bare `next`, which starts the MRO search one past
    /// `class_qualified`).
    pub target_class: Option<String>,
    /// Call-site resolution namespace, for resolving `target_class`.
    pub ns: String,
    /// Command name as written (`"next"` / `"nextto"`) — the
    /// diagnostic's display name.
    pub display_name: String,
    /// Full diagnostic span (command head through the last argument).
    pub full_span: Span,
    /// Lower-bound positional argument count, *excluding* `nextto`'s own
    /// target-class word.
    pub nargs_min: usize,
    /// Whether any positional word is `{*}`-expanded — same convention as
    /// [`PendingUserCallArity::positional_any_expand`].
    pub positional_any_expand: bool,
}

/// Variable definition record.
///
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
    /// Array element indices observed for this variable (`set arr(name) …`
    /// / `$arr(name)`).  Used by completion to offer `$arr(name)`.
    pub array_indices: std::collections::BTreeSet<String>,
    /// For a local that aliases a namespace/global cell (`global v`,
    /// `variable v`, `namespace upvar ns v local`), the qualified name of that
    /// cell (`::v`, `::ns::v`, …).  Every alias of the same cell — across
    /// procs, and the namespace-level declaration itself — carries the same
    /// target, so Find-References / Rename can unify them into one variable
    /// (the analyser analogue of Tcl's `VAR_LINK`).  `None` for an ordinary
    /// local or a directly-defined variable.
    pub link_target: Option<String>,
}

/// How a proc parameter is used inside the proc body.
///
/// Drives optimisation, shimmer analysis, taint propagation, and
/// diagnostics — tells downstream passes how a parameter value
/// flows through the proc.
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
    /// The parameter's **value** is used as a variable *name* in the
    /// proc's **own** (callee-local) scope — e.g. ``set $p 1``,
    /// ``scan $s %d $p``, ``lassign $l $p``, ``regsub … $p``, or a
    /// registry ``VarWrite`` / ``VarRead`` role landing on a bare
    /// ``$param`` substitution.
    ///
    /// Distinct from [`VarWrite`](Self::VarWrite) /
    /// [`VarRead`](Self::VarRead): those imply the param *aliases* a
    /// caller-frame variable via ``upvar`` (so passing a literal name
    /// at the call site consumes the caller's variable).  This trait is
    /// callee-local only — ``f x`` does **not** consume the caller's
    /// ``x``; the callee merely uses the string ``x`` to name one of
    /// its own locals.  It is always emitted alongside `VarRead` (the
    /// param's string value *is* read), so consumers querying
    /// `VarRead` alone for "is the param used at all" still see it;
    /// the refinement only matters for caller-side dead-store /
    /// unused-variable suppression, which must skip a param that is
    /// `DynamicNameLocal` without also being a genuine `VarWrite`.
    /// See PR #498 / #499 (deep-review finding 10 / 6).
    DynamicNameLocal,
    /// The parameter's **value** is used as a **command name** — either the
    /// command word of an invocation (``$cmd arg1 arg2``) or a registry / stub
    /// ``CommandPrefix`` callback argument (a command prefix such as
    /// ``tcltest::customMatch``'s matcher, ``selection handle``'s handler, or a
    /// stub ``:command_prefix`` argument).  Passing a literal at the call site
    /// therefore names a command, which a consumer can resolve (call graph) or
    /// highlight as a command.
    Command,
}

impl ProcArgTrait {
    /// Stable lower-case name suitable for serialisation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ProcArgTrait::Eval => "eval",
            ProcArgTrait::Body => "body",
            ProcArgTrait::VarWrite => "var_write",
            ProcArgTrait::VarRead => "var_read",
            ProcArgTrait::Expr => "expr",
            ProcArgTrait::LoopList => "loop_list",
            ProcArgTrait::DynamicNameLocal => "dynamic_name_local",
            ProcArgTrait::Command => "command",
        }
    }
}

/// Proc definition record.
///
/// Reuses the [`ParamDef`] type the signature scanner uses — same
/// param shape, same parser.
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

/// A lightweight *named definition* introduced by a registry
/// symbol-definer command (a `tcltest::test NAME …` case, …).
///
/// Unlike [`ProcDef`] / [`ClassDef`] these carry no parameter list or member
/// table — just enough to list the name in the document / workspace outline and
/// jump to it.  The analyser records one per call to a command whose registry
/// spec declares a [`tcl_registry::SymbolDef`]; the argument index and category
/// come from that descriptor, so no command name is hardcoded here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinedSymbol {
    /// Definition name as resolved (constant-propagated from the name
    /// argument).  For a test this is the test-case label (`foo-1.1`).
    pub name: String,
    /// Fully-qualified name with leading ``::`` (the enclosing namespace
    /// applied), for workspace-symbol container grouping.
    pub qualified_name: String,
    /// The outline category, straight from the registry descriptor.
    pub kind: tcl_registry::DefinedSymbolKind,
    /// Source span of the name argument's token — the outline selection range.
    pub name_span: Span,
    /// Source span covering the whole call (name token through the last
    /// argument), used as the outline entry's fold range.
    pub full_span: Span,
    /// Short description harvested from the descriptor's detail argument when
    /// it resolves to a constant, else `None`.
    pub detail: Option<String>,
}

/// Method definition inside a `TclOO` class.
///
/// Populated by the class-body walker; the shape is shared so the
/// class-hierarchy / MRO algorithms have a stable target.
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
    /// For a ``"forward"`` method (``forward NAME TARGET ?ARG…?``), the
    /// forwarded ``(target command, prepended args)`` — `TclOO`'s
    /// version of `interp alias` partial application (confirmed against
    /// tclsh 9.0.4: a forwarded method call binds the prepended args
    /// first, then the caller's own arguments, against `TARGET`'s own
    /// arity). `None` for every other kind, and for a `forward` whose
    /// target couldn't be parsed.
    pub forward_target: Option<(String, Vec<String>)>,
}

/// `TclOO` property definition.
///
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
    /// is omitted.
    pub kind: String,
    /// True when ``-get BODY`` was supplied.
    pub has_getter: bool,
    /// True when ``-set BODY`` was supplied.
    pub has_setter: bool,
}

/// Class definition record.
///
/// The structural fields (`superclasses`, `mixins`, `methods`,
/// `class_methods`) feed the class-hierarchy / MRO algorithms; the
/// body walker populates them.  The remaining fields (``metaclass``,
/// ``properties``, ``variables``, ``filters``, ``exports``,
/// ``unexports``, ``constructors``, ``destructor``, ``doc``) carry
/// the full record.
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
    /// ``"oo::class"``.
    pub metaclass: String,
    /// Direct superclasses in declaration order.  Each entry
    /// is a fully-qualified class name with leading ``::``.
    pub superclasses: Vec<String>,
    /// Class-level mixins in declaration order.  Same naming
    /// convention as `superclasses`.
    pub mixins: Vec<String>,
    /// `(name, span)` for each superclass usage in a `superclass …`
    /// declaration — drives find-references on the referenced class.
    pub superclass_refs: Vec<(String, Span)>,
    /// `(name, span)` for each mixin usage in a `mixin …` declaration.
    pub mixin_refs: Vec<(String, Span)>,
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
    /// `true` when this record originates from an ``oo::define`` on a class
    /// **not** created in this file — a cross-file extension "stub" (it adds
    /// members / a `superclass` to a class defined elsewhere) rather than the
    /// class's own ``oo::class create`` definition.  Go-to-definition prefers a
    /// real creation site over such a stub; `false` for `oo::class create` and
    /// for an ``oo::define`` that extends a class created earlier in the same
    /// file.
    pub via_define: bool,
}

impl Default for ClassDef {
    /// Default-construct a [`ClassDef`].
    ///
    /// All names default to empty strings, both spans default to
    /// ``Span::new(0, 0)``, and the metaclass defaults to
    /// ``"oo::class"``.
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
            superclass_refs: Vec::new(),
            mixin_refs: Vec::new(),
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
            via_define: false,
        }
    }
}

/// A lexical scope (global, namespace, or proc body).
///
/// The analyser builds a tree of these as it walks; the root is
/// ``AnalysisResult.global_scope``.
///
/// Children are stored inline as ``Vec<Scope>``, so the tree is
/// a strict ownership graph.  The parent link is implicit, held
/// by the analyser's traversal stack
/// (``Analyser::current_scope_path``) rather than embedded as a
/// back-pointer.  Snapshot / restore only needs to copy the result
/// tree, not rewrite back-pointers.
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
    /// Lightweight named definitions (tcltest tests, …) declared directly in
    /// this scope by a registry symbol-definer command, in declaration order.
    pub defined_symbols: Vec<DefinedSymbol>,
    /// Child scopes (in declaration order).
    pub children: Vec<Scope>,
    /// True for a **`TclOO`** method scope: the body executes with the
    /// *object's* run-time namespace current (`::oo::ObjN`, path
    /// `::oo::Helpers`), so bare command calls are statically approximated
    /// as resolving from the GLOBAL namespace — the class's defining
    /// namespace is NEVER searched (tclsh 8.6.16 / 9.0.4-pinned: a helper
    /// proc in the class's defining namespace is unreachable unqualified
    /// from a method body; a global one is found). snit / itcl method
    /// scopes stay `false` — their members genuinely resolve in the
    /// type / class namespace.
    pub oo_global_resolution: bool,
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
            defined_symbols: Vec::new(),
            children: Vec::new(),
            oo_global_resolution: false,
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
/// Populated when the analyser encounters ``proc unknown {cmd args}
/// { ... }``: the body is lowered to IR and inspected for
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
// logic, so they stay as separate bools rather than a bitflags set.
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

/// Lazily-built, equality-transparent cache of the [`ClassHierarchy`]
/// derived from [`AnalysisResult::all_classes`].
///
/// The hierarchy is a pure function of `all_classes`, so it is excluded
/// from equality (two results with equal classes are equal regardless of
/// cache state) and reset on clone (rebuilt on demand).  This lets the LSP
/// providers (hover / completion / definition / rename / type-hierarchy)
/// share one MRO/method-provider computation per analysis instead of
/// rebuilding it on every request.
#[derive(Debug, Default)]
pub struct HierarchyCache(std::sync::OnceLock<super::class_hierarchy::ClassHierarchy>);

impl Clone for HierarchyCache {
    fn clone(&self) -> Self {
        // Fresh cache; the clone rebuilds identically on first access.
        Self(std::sync::OnceLock::new())
    }
}

impl PartialEq for HierarchyCache {
    fn eq(&self, _: &Self) -> bool {
        // A derived cache never affects analysis-result equality.
        true
    }
}

/// A lexical region whose body runs in a scoped command environment.
///
/// Recorded by the analyser when it recurses into the
/// [`ArgRole::Body`](tcl_registry::ArgRole::Body) argument of a command whose
/// spec carries a [`body_scope`](tcl_registry::CommandSpec::body_scope) (e.g. a
/// `report::defstyle` style script).  `span` covers the body's brace-delimited
/// region; the post-walk W123 pass and the LSP hover / completion providers
/// resolve a command head against `env` when its position falls inside `span`.
#[derive(Debug, Clone)]
pub struct ScopedBodyRegion {
    /// Byte span of the scoped body (the brace-delimited word).
    pub span: Span,
    /// The command environment ambient inside the body.
    pub env: &'static tcl_registry::scoped::ScopedCommandEnv,
}

impl PartialEq for ScopedBodyRegion {
    fn eq(&self, other: &Self) -> bool {
        // Environments are `&'static` singletons; pointer identity is the
        // cheapest sound comparison and avoids requiring `PartialEq` on the
        // registry-side hover/subcommand descriptors.
        self.span == other.span && std::ptr::eq(self.env, other.env)
    }
}

impl ScopedBodyRegion {
    /// Whether `offset` falls strictly inside this region's body.
    #[must_use]
    pub fn contains(&self, offset: u32) -> bool {
        self.span.start() <= offset && offset < self.span.end()
    }
}

/// Complete analysis result for a single document.
///
/// Holds the full field set the analyser can produce. Fields that
/// no emitter populates default to empty / `None` — they're carried
/// in the shape so the `PyO3` binding can serialise the complete
/// result dict.
#[derive(Debug, Clone, Default, PartialEq)]
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
    /// Every lightweight named definition (tcltest tests, …) in the document,
    /// in source order — the flat companion to the per-scope
    /// [`Scope::defined_symbols`] the workspace-symbol provider walks.
    pub all_defined_symbols: Vec<DefinedSymbol>,
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
    /// Static `rename OLD NEW` records: `new_qname → old_qname`. `NEW`
    /// resolves to whatever `OLD` denoted (unchanged) — see
    /// [`super::state::Analyser::renamed_commands`] for why a dynamic
    /// rename is deliberately absent here.
    pub renamed_commands: HashMap<String, String>,
    /// Source span of the `OLD` word of each static `rename OLD NEW`, keyed
    /// by `new_qname` (the same key as [`Self::renamed_commands`]). `OLD`
    /// names the command being moved, so it is a reference to it that rename
    /// rewrites; kept beside the name map rather than in it so the existing
    /// consumers of `renamed_commands` are untouched.
    pub rename_target_spans: HashMap<String, Span>,
    /// Namespace import records.
    pub namespace_imports: Vec<SignatureNamespaceImport>,
    /// Recorded `namespace path {…}` declarations, keyed by the declaring
    /// namespace's fully-qualified name (`::` for global).  Each entry is the
    /// path list *as written*; a relative entry roots against the declaring
    /// namespace.  Consumed by command resolution so a bare call reaches a proc
    /// on the namespace path before falling through to global.
    pub namespace_paths: HashMap<String, Vec<String>>,
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
    /// Instance-variable → class qualified-name map for `TclOO`
    /// objects.  Populated by a syntactic scan for
    /// ``set VAR [CLASS new …]`` / ``set VAR [CLASS create …]``
    /// and ``CLASS create VAR …`` patterns where ``CLASS`` is a
    /// user-defined class in [`Self::all_classes`].  Lets the
    /// LSP providers resolve ``$obj method`` call sites to the
    /// object's class.  Best-effort and not flow-sensitive — the
    /// last assignment wins, matching the global-by-var-name
    /// shape the W308 emitter already uses.
    pub instance_classes: HashMap<String, String>,
    /// Simple names of instance commands created by a `CLASS create NAME …`
    /// construct — both when `CLASS` is a known user class *and* when it is an
    /// unresolved (external-package) command whose `create NAME` idiom clearly
    /// binds a new object command.  A `create NAME` call names a command, so
    /// later `NAME method` dispatch — and `$var method` where `var` provably
    /// holds one of these names — must not be flagged as an unknown command
    /// (W123) or a stray non-literal command word (W307).  Issue #777.
    pub created_instance_commands: std::collections::HashSet<String>,
    /// Names dropped from [`Self::instance_classes`] because a *registry*
    /// object-factory binding (Tk widget path, tcllib naming factory) saw
    /// the same name bound to two different classes somewhere in the file
    /// — e.g. `.t` created as both a `ttk::treeview` and a `listbox` in two
    /// different procs. `instance_classes` itself stays last-write-wins for
    /// every other producer (its long-documented, best-effort contract);
    /// this set exists only so a consumer that needs a *sound* answer
    /// (`widget_command.rs`'s W001/E002/E003 — issue #927) can tell "no
    /// binding" apart from "binding, but two different ones, so
    /// unknowable" and abstain on the latter rather than trust whichever
    /// write happened to run last.  Populated only by
    /// `Analyser::record_registry_factory_instance`'s two registry-driven
    /// binding sites, not by the `TclOO` user-class paths in
    /// `record_instance_creation`.
    pub ambiguous_instance_names: std::collections::HashSet<String>,
    /// Per-object method declarations added by
    /// `oo::objdefine $obj { method … }` (or its inline form), keyed by the
    /// object variable's simple name (`$obj` / `${obj}` / bare `obj` all key
    /// as `obj`).  `TclOO` layers a per-object method *ahead* of the object's
    /// class methods, so `$obj m` navigation resolves the per-object override
    /// recorded here before falling back to the class.  The method **bodies**
    /// are walked into the scope tree at analysis time (so in-body diagnostics
    /// and variable/command resolution work); this map carries only each
    /// declaration's `name_span` for the receiver-dispatch name lookup.
    pub object_methods: HashMap<String, Vec<MethodDef>>,
    /// Call sites of unresolved (unknown) commands — `(span, bare name)`, the
    /// same set the W123 diagnostic is emitted for, but recorded **regardless of
    /// whether W123 is disabled** (only the *diagnostic* honours the toggle).
    /// Cross-file resolution keys its arity check off this, so disabling W123 does
    /// not also silence the cross-file arity error.  Empty when the W123 emitter's
    /// knowability gates fire (e.g. a dynamic `package require` / `unknown` proc).
    pub unresolved_command_sites: Vec<(Span, String)>,
    /// Lexical regions whose body runs in a scoped command environment
    /// (`report::defstyle` style scripts, …).  The W123 unknown-command pass
    /// treats a bare head inside one of these regions as known when it resolves
    /// against the region's [`ScopedBodyRegion::env`]; the LSP hover /
    /// completion providers read them to surface the scoped command set.  Empty
    /// for documents with no scoped-body commands.
    pub scoped_command_regions: Vec<ScopedBodyRegion>,
    /// Names introduced by a scoped-body definer command whose environment sets
    /// `include_sibling_definitions` — keyed by the environment name.  A
    /// `report::defstyle simpletable …` records `"simpletable"` under
    /// `"report style definition"`, so a later style body calling `simpletable`
    /// resolves instead of drawing a W123.
    pub scoped_sibling_defs: HashMap<&'static str, std::collections::HashSet<String>>,
    /// Memoised class hierarchy — see [`HierarchyCache`].  Not part of the
    /// analysis output; built on first [`Self::class_hierarchy`] call.  The
    /// inner cache is opaque (its `OnceLock` is private), so this being
    /// `pub` only preserves functional-update construction
    /// (`..Default::default()`); it can't be populated from outside.
    pub hierarchy_cache: HierarchyCache,
    /// The dialect this document was analysed under (`"tcl9.0"`,
    /// `"tcl8.6"`, `"f5-irules"`, …) — whatever string the caller passed to
    /// [`super::Analyser::analyse`].  Carried on the result so downstream
    /// consumers (the LSP variable-resolution path, completion) can apply
    /// *version-dependent* semantics — e.g. the TIP 278 namespace-scope
    /// global fallback ([`Self::ns_var_global_fallback`]) — without
    /// re-detecting the dialect.  Empty for a default-constructed result.
    pub dialect: String,
}

impl AnalysisResult {
    /// The [`ClassHierarchy`](super::class_hierarchy::ClassHierarchy) for
    /// this result's classes, built once and cached.  Prefer this over
    /// calling [`build_class_hierarchy`](super::class_hierarchy::build_class_hierarchy)
    /// directly so hover / completion / definition / rename /
    /// type-hierarchy requests share one MRO computation rather than
    /// rebuilding (and re-cloning `all_classes`) each time.
    #[must_use]
    pub fn class_hierarchy(&self) -> &super::class_hierarchy::ClassHierarchy {
        self.hierarchy_cache
            .0
            .get_or_init(|| super::class_hierarchy::build_class_hierarchy(self.all_classes.clone()))
    }

    /// Whether this document's dialect keeps the TIP 278 namespace-scope
    /// global variable fallback (Tcl 8.x yes, 9.0+ no) — the registry's
    /// [`DialectSet::namespace_var_global_fallback`](tcl_registry::prelude::DialectSet::namespace_var_global_fallback)
    /// applied to [`Self::dialect`].  The default-constructed empty dialect
    /// answers `false` (the stricter 9.0 semantics).
    #[must_use]
    pub fn ns_var_global_fallback(&self) -> bool {
        tcl_registry::prelude::DialectSet::namespace_var_global_fallback(&self.dialect)
    }
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
/// brace block.
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
    /// Trailing ``?-flag…?`` flags on a ``# tcl-lsp: stub`` line:
    /// ``barrier`` / ``loop`` / ``pure`` / ``mutator`` / ``unsafe``
    /// / ``scope_alias``, packed into a single byte because they're
    /// an enum-set of orthogonal flags.
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

impl StubCommandDef {
    /// Convert to the overlay-side [`tcl_registry::stub_overlay
    /// ::StubSig`] shape.  Drops the source span (the overlay
    /// only cares about the semantic shape; the original
    /// `StubCommandDef` retains the span for diagnostic
    /// emitters) and canonicalises each argument role string
    /// through [`tcl_registry::stub_overlay::StubOverlay::parse_role`].
    ///
    /// The two `StubFlags` bitflag types share identical bit
    /// layout by design (see the doc comment on
    /// [`tcl_registry::stub_overlay::StubSigFlags`]), so the
    /// flag conversion is a 1-for-1 bit copy via `bits()` /
    /// `from_bits_truncate`.
    #[must_use]
    pub fn to_stub_sig(&self) -> tcl_registry::stub_overlay::StubSig {
        use tcl_registry::stub_overlay::{StubArg, StubOverlay, StubSig, StubSigFlags};
        StubSig {
            name: self.name.clone(),
            args: self
                .args
                .iter()
                .map(|a| StubArg {
                    name: a.name.clone(),
                    role: StubOverlay::parse_role(&a.role),
                    optional: a.optional,
                })
                .collect(),
            flags: StubSigFlags::from_bits_truncate(self.flags.bits()),
        }
    }
}

/// Build a [`tcl_registry::stub_overlay::StubOverlay`] from a
/// slice of [`StubCommandDef`] records.  The order of inserts
/// matches the order in `defs`; per
/// [`tcl_registry::stub_overlay::StubOverlay::insert`]'s
/// "last directive wins" semantics, a later directive for the
/// same name overrides an earlier one.
#[must_use]
pub fn build_stub_overlay(defs: &[StubCommandDef]) -> tcl_registry::stub_overlay::StubOverlay {
    let mut overlay = tcl_registry::stub_overlay::StubOverlay::new();
    for def in defs {
        overlay.insert(def.to_stub_sig());
    }
    overlay
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
        // ``metaclass`` is the only non-trivial default the rest of
        // the analyser depends on.
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
