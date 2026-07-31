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
    ParamDef, SignatureCommandAlias, SignatureCommandInvocation, SignatureNamespaceExport,
    SignatureNamespaceForget, SignatureNamespaceImport, SignaturePackageRequire, SignatureSource,
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
    /// Span of the source word that **names** [`Self::link_target`] — the
    /// word a rename of that cell must rewrite.
    ///
    /// For `variable v` and `global ::ns::v` this is the declaration word
    /// itself, so it equals [`Self::definition_span`]: the local alias name
    /// *is* the cell's (tail) name, and renaming the cell renames the local
    /// spelling with it.
    ///
    /// For `namespace upvar ::ns v local` and `upvar #0 ::ns::v local` it is
    /// a **different** word from the declaration: the cell is named by
    /// `otherVar` (`v` / `::ns::v`), while `local` is an independent local
    /// spelling that the cell's name does not determine.  Renaming the cell
    /// must rewrite `otherVar` and leave `local` alone — rewriting `local`
    /// instead re-points the alias at a cell that no longer exists (tclsh
    /// 9.0.4 / 8.6.16 alike: `can't read "total": no such variable`).
    ///
    /// `None` whenever [`Self::link_target`] is `None`.
    pub link_target_span: Option<Span>,
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
    ///
    /// Empty *and* [`Self::params_computed`] set means "unknown", not
    /// "none" — see that field.
    pub params: Vec<ParamDef>,
    /// The parameter-list word was **computed**, so the proc's formals are
    /// unmodelled (issue #1079).
    ///
    /// `proc p [makeargs] {…}` / `proc q $params {…}` build the formal list at
    /// definition time from a value; which names it declares, and how many,
    /// are run-time facts (tclsh 9.0.4 / 8.6.16: with
    /// `proc makeargs {} {return {a b}}`, `proc p [makeargs] {…}` then
    /// `info args p` → `a b`, and `p 1 2` runs).  The analyser used to read
    /// the unresolved word as a one-parameter literal and register a
    /// `VarDef` whose *name is the source text* (`"[makeargs]"`) — a wrong
    /// fact that also made the call-site arity checker demand exactly one
    /// argument.  Now nothing is claimed: no per-parameter `VarDef`, and the
    /// arity resolvers abstain (`0..unlimited`).
    pub params_computed: bool,
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
    /// Parameters whose *value* names a variable in the **immediate caller's**
    /// frame — the `upvar 1 $param local` shape, and only that one.
    ///
    /// Strictly narrower than the [`ProcArgTrait::VarWrite`] /
    /// [`ProcArgTrait::VarRead`] entries in [`Self::param_traits`], which say
    /// a parameter's value is used as a variable *name* through an `upvar`
    /// but not which frame the alias lands in.  `upvar 0` aliases the
    /// callee's own frame, `upvar #0` the global one, `upvar 2` the caller's
    /// caller — none of them creates anything in the calling frame, so a
    /// call-site consumer (hover / go-to-definition / find-references on a
    /// caller-frame variable) must intersect the two rather than trust the
    /// trait alone.  Populated by
    /// [`super::param_traits::caller_frame_upvar_params`].
    pub caller_frame_params: std::collections::HashSet<String>,
}

impl ProcDef {
    /// The proc's declared argument arity — or the **abstaining**
    /// `0..unlimited` when its parameter list was computed
    /// ([`Self::params_computed`], issue #1079).
    ///
    /// A computed list declares an unknown number of formals with unknown
    /// names, so no call-site count can be wrong: `proc p [makeargs] {…}` with
    /// `makeargs` returning `{a b}` really does take two arguments (tclsh
    /// 9.0.4 / 8.6.16: `p 1 2` → `p got 1 2`), and reading the unresolved word
    /// as one literal parameter made `p 1 2` draw a false
    /// `E003 Too many arguments`.  Every arity consumer asks here rather than
    /// calling [`crate::signature_scan::arity::arity_of`] on `params`
    /// directly, so the abstention cannot be forgotten at one call site.
    #[must_use]
    pub fn arity(&self) -> tcl_registry::Arity {
        if self.params_computed {
            return tcl_registry::Arity::new(0, tcl_registry::Arity::UNLIMITED);
        }
        crate::signature_scan::arity::arity_of(&self.params)
    }
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
    /// `true` only for a `classmethod`-kind entry declared via `TclOO`'s
    /// `self` wrapper (`self method NAME …`) directly on this class
    /// (issue #923 idx 120). Unlike `ooutil`'s `classmethod` keyword —
    /// confirmed against tclsh 9.0.4/8.6 to propagate to a subclass's own
    /// bound command via its `Delegate`-mixin machinery, which walks
    /// `info class superclass` — a plain `self method` is visible ONLY on
    /// the exact class that declared it: `oo::class create Gadget {
    /// superclass Widget }` does NOT gain `Widget`'s `self method make`
    /// (real tclsh: `unknown method "make"`). The class-command MRO walk
    /// in `tcl-lsp-core`'s `method_dispatch_definition` accepts a
    /// `class_methods` entry from an ancestor provider only when this is
    /// `false`; a provider matching the receiver class itself is always
    /// accepted regardless.
    pub is_self_method: bool,
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

/// Stable composite key naming a class's instance method or class method:
/// `{class}::method::{name}` / `{class}::classmethod::{name}`.  The single
/// canonical form for re-identifying the *same* member across a round-trip —
/// the incremental item-tree diff ([`crate::analyser::item_tree`]) and any
/// LSP-facing consumer that hands a member identity to the client and gets
/// it back later (the code-lens `codeLens/resolve` handshake) — so the two
/// never drift onto different naming schemes for the same member.  A class
/// or proc's own qualified name (`::Bar`, `::helper`) never contains
/// `::method::`/`::classmethod::`, so this key cannot collide with theirs.
#[must_use]
pub fn class_member_key(class_qualified: &str, name: &str, is_classmethod: bool) -> String {
    if is_classmethod {
        format!("{class_qualified}::classmethod::{name}")
    } else {
        format!("{class_qualified}::method::{name}")
    }
}

/// Stable composite key naming a class's `property`: `{class}::property::{name}`.
/// Properties are a third, independent member table — never a `method` or
/// `classmethod` — so this never collides with [`class_member_key`]'s output;
/// same round-trip rationale (item-tree diff, code-lens `codeLens/resolve`).
#[must_use]
pub fn class_property_key(class_qualified: &str, name: &str) -> String {
    format!("{class_qualified}::property::{name}")
}

/// Stable composite key naming a class's own effective `constructor`:
/// `{class}::constructor`.  Unlike a method or property there is exactly one
/// dispatch-reachable constructor per class (`oo::configurable` allows
/// several to be *declared*, but only the last is ever effective — see
/// [`crate::analyser::class_hierarchy::ClassHierarchy::constructor_provider`]),
/// so this carries no `{name}` suffix; same round-trip rationale as
/// [`class_member_key`] / [`class_property_key`].
#[must_use]
pub fn class_constructor_key(class_qualified: &str) -> String {
    format!("{class_qualified}::constructor")
}

/// The `destructor` counterpart of [`class_constructor_key`]:
/// `{class}::destructor`.
#[must_use]
pub fn class_destructor_key(class_qualified: &str) -> String {
    format!("{class_qualified}::destructor")
}

/// One per-object method added by an `oo::objdefine` (issue #945
/// fault 5): the method declaration plus the **objdefine site's**
/// receiver offset, the anchor a consumer resolves to a variable
/// binding so same-named receivers in different scopes never collide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMethodDef {
    /// The declared method (name, spans, kind, visibility).
    pub def: MethodDef,
    /// Byte offset of the `oo::objdefine` receiver word — inside the
    /// scope whose binding of the receiver variable this record
    /// belongs to.
    pub objdefine_offset: u32,
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

/// Which of a class's two method tables a definition-body member word acts on.
///
/// `TclOO` keeps the members a class gives its *instances* ([`ClassDef::methods`])
/// and the members defined on the class *object* itself
/// ([`ClassDef::class_methods`], what `self …` targets) in separate tables, and
/// every member word is scoped to exactly one of them: an unwrapped
/// `deletemethod` / `unexport` in a class body acts on the instance side, the
/// same word under `self` acts on the class-object side, and neither reaches
/// across.  Naming the side explicitly is what stops a class-side `unexport m`
/// from also un-exporting an identically-named instance method (issue #1098),
/// and what lets a cross-document retraction record which table it removes from
/// (issue #1101).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberSide {
    /// [`ClassDef::methods`] — what instances of the class dispatch.
    Instance,
    /// [`ClassDef::class_methods`] — what the class object itself dispatches
    /// (`self method …`, `self { … }`).
    ClassObject,
}

impl MemberSide {
    /// The [`ClassDef`] method table this side names.
    #[must_use]
    pub fn table(self, class_def: &mut ClassDef) -> &mut HashMap<String, MethodDef> {
        match self {
            Self::Instance => &mut class_def.methods,
            Self::ClassObject => &mut class_def.class_methods,
        }
    }
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
    ///
    /// Kept **mutually exclusive** with [`Self::exports`] — each set records the
    /// last explicit writer for a name, and the cross-file consumer
    /// (`workspace_index::method_dispatch_chain`) reads any `exports` entry as
    /// decisive, so a name left in both would advertise a method the interpreter
    /// rejects.
    pub unexports: HashSet<String>,
    /// Members this record **retracts** (`deletemethod` / `renamemethod`)
    /// without itself declaring them — a *tombstone* for a cross-document
    /// consumer, keyed by member name and the side the retracting word was
    /// scoped to.
    ///
    /// A retraction of a member the same document declares needs no tombstone:
    /// it already removed the entry from that side's table, and the order within
    /// one body is known. What cannot be modelled locally is
    /// `oo::class create ::C { method m … }` in one file and
    /// `oo::define ::C { deletemethod m }` in another: the second file's
    /// [`Self::via_define`] stub finds nothing to remove, so without this the
    /// workspace goes on advertising and resolving `m` even though sourcing the
    /// extension deletes it (issue #1101 review). Cross-file *order* is
    /// unprovable, so the tombstone is unordered — exactly as a cross-file
    /// `oo::define ::C { method extra … }` is an unordered addition today.
    pub retracted_members: Vec<(String, MemberSide)>,
    /// Names genuinely bareword-callable from any method body of this
    /// class, because ``link`` (``oo::Helpers::link``) installed a
    /// per-object-namespace alias — keyed by the alias name, valued by
    /// the real member name it dispatches to via ``my TARGET`` (alias ==
    /// target for the plain ``link NAME`` form). In contrast, a bareword
    /// matching an un-linked sibling method/classmethod/property name is
    /// **not** reachable that way and errors "invalid command name" at
    /// runtime (issue #923 idx 113).
    pub linked_members: HashMap<String, String>,
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
    /// `true` when the class was manufactured by a **user-defined metaclass**
    /// whose `create` override the analyser could not read, so the superclass
    /// list it splices into the class is unknown.  The class itself is real
    /// and its own body is fully modelled; only its inheritance is opaque.
    /// Method-existence checks (W308) must abstain on such a class exactly as
    /// they do for one whose superclass lives outside the workspace index —
    /// a method it inherits is not one it is missing (issue #923 idx 96/97).
    pub inheritance_unknown: bool,
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
            methods: HashMap::new(),
            class_methods: HashMap::new(),
            constructors: Vec::new(),
            destructor: None,
            variables: Vec::new(),
            properties: HashMap::new(),
            filters: Vec::new(),
            exports: HashSet::new(),
            unexports: HashSet::new(),
            retracted_members: Vec::new(),
            linked_members: HashMap::new(),
            doc: String::new(),
            via_define: false,
            inheritance_unknown: false,
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
    /// True when this [`Self::oo_global_resolution`] frame is a real
    /// `TclOO` **method invocation** (`method` / `constructor` /
    /// `destructor` / class-side / `oo::objdefine method`) rather than
    /// merely a frame whose namespace path reaches `::oo::Helpers`.
    ///
    /// The pair encodes two facts real Tcl keeps separate, and conflating
    /// them is a live defect (Codex review of PR #1084). A Tcl 9 class
    /// `initialise` / `initialize` body runs in the class object's own
    /// namespace with `namespace path` = `::oo::Helpers ::oo`, so it sets
    /// `oo_global_resolution` — the helpers genuinely **resolve** there —
    /// but it is not a method context, so calling one raises `… may only
    /// be called from inside a method` (tclsh 9.0.4). Such a scope leaves
    /// this `false`.
    ///
    /// `W123` keys on resolution, so it must consult
    /// [`crate::analyser::scope::innermost_scope_reaches_oo_helpers`];
    /// completion and hover key on callability, so they consult
    /// [`crate::analyser::scope::innermost_scope_is_oo_method_frame`].
    /// `my` is callable in both kinds of frame — it lives in the object's
    /// own namespace, not `::oo::Helpers`, and a class is an object — which
    /// is why the per-command half of the rule is registry data
    /// (`Traits::TCLOO_REQUIRES_METHOD_FRAME`) rather than a second scope
    /// flag.
    pub oo_method_frame: bool,
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
            oo_method_frame: false,
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
    /// Every `proc` declaration's own name-token span, in source order —
    /// unlike `all_procs` (a map that, for a qualified name declared more
    /// than once in the same document, retains only the *last* processed
    /// declaration — plain Tcl's own "last redefinition wins" semantics),
    /// this keeps one entry per declaration, including an earlier, shadowed
    /// one. Exists purely so a query landing on a shadowed declaration's own
    /// name token can still be recognised as declaring that qualified name
    /// and re-resolved to whichever `ProcDef` currently wins in `all_procs`
    /// (issue #923 idx 31, main audit wave) — `all_procs`' own span alone
    /// can never satisfy that lookup, since it only ever holds the winner's.
    pub proc_declaration_sites: Vec<(String, Span)>,
    /// The `ProcDef`s a later same-named `proc` displaced from
    /// [`Self::all_procs`], keyed by qualified name, in source order —
    /// empty for every document that redefines nothing.
    ///
    /// `all_procs` keeps plain Tcl's "last redefinition wins", which is the
    /// right answer for a call site *after* the last declaration and the
    /// wrong one for a call between two of them: that call reaches the
    /// earlier definition, with the earlier definition's span and parameter
    /// list. This is the order-gated companion the map cannot express —
    /// the `proc` analogue of [`Self::rename_offsets`] /
    /// [`Self::alias_offsets`] — and
    /// [`Self::proc_def_in_effect_at`] is how consumers ask it (issue #923
    /// idx 45).
    ///
    /// Distinct from [`Self::proc_declaration_sites`], which records only
    /// *spans* (enough to recognise a shadowed declaration's own name token,
    /// issue #923 idx 31) and so cannot answer what the displaced definition's
    /// parameters were.
    pub superseded_procs: HashMap<String, Vec<ProcDef>>,
    /// Classes keyed by qualified name.
    pub all_classes: HashMap<String, ClassDef>,
    /// Every class-body span contributing member declarations to a
    /// qualified class name, in source order: the `oo::class create`
    /// block's own body span, plus one entry per later same-class
    /// `oo::define ClassName { ... }` extension (or inline-form call) in
    /// this file. The multi-span analogue of [`ClassDef::body_span`]
    /// (which stays pinned to the class's primary/creation site for
    /// hover / document-symbol / rename-target purposes): a class
    /// extended via a *separate* `oo::define` block has textually
    /// disjoint body spans, not one contiguous range, so any consumer
    /// asking "which class's body lexically contains this offset" for
    /// `my`-dispatch resolution must check every entry here rather than
    /// just `ClassDef::body_span` (issue #923 idx 52, main audit wave).
    pub class_body_spans: Vec<(String, Span)>,
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
    /// Byte offset of the `interp alias` command token that established each
    /// [`Self::command_aliases`] entry, keyed the same way (by qualified
    /// alias name) — lets a consumer such as
    /// [`Self::offset_is_inside_any_definition_body`] tell an alias declared
    /// inside a proc/class body (conditional — exists only while that
    /// enclosing definition is running) apart from a top-level one
    /// (unconditional).
    pub alias_offsets: HashMap<String, u32>,
    /// Static `rename OLD NEW` records: `new_qname → old_qname`. `NEW`
    /// resolves to whatever `OLD` denoted (unchanged) — see
    /// [`super::state::Analyser::renamed_commands`] for why a dynamic
    /// rename is deliberately absent here. `OLD`'s own token is recorded
    /// as an ordinary [`SignatureCommandInvocation`] (`command_invocations`)
    /// instead of a dedicated span map here — it is a first-class reference
    /// to the command it names, exactly like `info body PROC` — so
    /// find-references / go-to-definition / rename reach it through the
    /// same path as any other reference, covering a deleting `rename OLD
    /// {}` too (issue #923 idx 39, main audit wave).
    pub renamed_commands: HashMap<String, String>,
    /// Byte offset of the `rename` command token that established each
    /// [`Self::renamed_commands`] entry, keyed the same way (by qualified
    /// `NEW` name) — the `rename` analogue of [`Self::alias_offsets`], for
    /// the same "declared inside a proc/class body" nested-link check.
    pub rename_offsets: HashMap<String, u32>,
    /// Static `namespace ensemble create -map {sub target …}` /
    /// `-subcommands {a b …}` records: outer key is the ensemble's own
    /// resolved, qualified invocable name (`::widget`) — the same identity
    /// `ensemble_namespaces`/ `Analyser::ensemble_namespaces` already uses;
    /// inner key is the subcommand exactly as written (`make`); inner value
    /// is the target's resolved qualified command name (`::widget::Make`).
    /// A nested table, not a flat `command_aliases`-shaped one, because an
    /// ensemble subcommand is never independently callable the way an alias
    /// name is (only the pair `widget make` dispatches), and two different
    /// ensembles may share a subcommand spelling. Lets `definition`/`hover`/
    /// `references` in `tcl-lsp-core` resolve `widget make` to `::widget::Make`
    /// (issue #923 idx 106) the same way they already resolve an alias name
    /// to its target. Only ever populated from a *literal* `-map`/
    /// `-subcommands` list — a dynamic value (`-map $var`) leaves the
    /// ensemble's entry absent entirely, so a lookup against it correctly
    /// abstains rather than guessing.
    pub ensemble_subcommand_targets: HashMap<String, HashMap<String, String>>,
    /// Namespace import records.
    pub namespace_imports: Vec<SignatureNamespaceImport>,
    /// Namespace `forget` records — the removal half of the import edge's
    /// ordered lifecycle log (issue #1103). See
    /// [`SignatureNamespaceForget`] for the oracle: an import installs an
    /// alias, `namespace forget` takes it away again, and a bare call after
    /// the forget is `invalid command name`. Consumed together with
    /// [`Self::namespace_imports`] by
    /// `tcl_lsp_core::namespace_import::alias_live_at`, under the same
    /// order gate ([`super::indirection::in_effect`]) the export snapshot
    /// and the `rename` / `interp alias` timeline already use.
    pub namespace_forgets: Vec<SignatureNamespaceForget>,
    /// Byte offset of the statement that **destroyed** each qualified command
    /// — `rename OLD {}` and `interp alias {} NAME {}`, the two forms that
    /// delete the command *object* rather than move it.
    ///
    /// A destruction is a lifecycle event on every `namespace import` edge
    /// pointing at the command: the alias holds the object, so destroying it
    /// kills the alias too (oracle tclsh 8.6.14 / 9.0.4 — with `::dst::p`
    /// imported from `::src::p`, `rename ::src::p {}` makes `::dst::p` an
    /// `invalid command name` and empties `info commands ::dst::*`). Issue
    /// #1103.
    ///
    /// A **rename** is deliberately absent, which is why this is not
    /// [`super::state::Analyser::deleted_commands`] (whose "`OLD` is no
    /// longer callable under that name" meaning covers both forms, because
    /// that is what its W123 / arity consumers ask). `rename ::src::p
    /// ::src::pp` keeps `::dst::p` working and merely moves the origin
    /// (`namespace origin ::dst::p` → `::src::pp`) — the same
    /// rename-captures-object-identity rule
    /// [`super::indirection`] already models.
    ///
    /// Only straight-line (non-conditional) **load-level** destructions are
    /// recorded: one written inside a proc/class body runs at call time, when
    /// and if that definition is invoked, and a consumer that revoked an alias
    /// on it would drop a genuinely-live command
    /// ([`super::state::Analyser::publish_load_level_destructions`]).
    pub destroyed_commands: HashMap<String, u32>,
    /// Namespace `export` records — see [`SignatureNamespaceExport`] for why
    /// they exist (gating wildcard-import bareword resolution, issue #923
    /// idx 18).
    pub namespace_exports: Vec<SignatureNamespaceExport>,
    /// Recorded `namespace path {…}` declarations, keyed by the declaring
    /// namespace's fully-qualified name (`::` for global).  Each entry is the
    /// path list *as written*; a relative entry roots against the declaring
    /// namespace.  Consumed by command resolution so a bare call reaches a proc
    /// on the namespace path before falling through to global.
    pub namespace_paths: HashMap<String, Vec<String>>,
    /// `auto_path` mutations (``lappend auto_path …`` / ``set auto_path …``).
    pub auto_path_entries: Vec<AutoPathEntry>,
    /// Namespace-qualified variable occurrences, in source order — see
    /// [`QualifiedVarRef`].  The cross-document variable reference set is
    /// built from these; an unqualified occurrence is never recorded.
    pub qualified_var_refs: Vec<QualifiedVarRef>,
    /// Words naming a **namespace**, in source order — see [`NamespaceRef`].
    /// Both the declaring `namespace eval` name tokens (`declares: true`) and
    /// every other spelling of the same namespace, so go-to-definition /
    /// hover / find-references treat a namespace as a first-class symbol
    /// (issue #1088).
    pub namespace_refs: Vec<NamespaceRef>,
    /// Byte spans where command resolution is pinned to a namespace by
    /// runtime context rather than lexical nesting (issue #923 idx 116):
    /// `apply {{params} body ns}` runs `body` in `ns`, not the namespace
    /// the lambda is lexically written inside. Each entry is `(body_span,
    /// "::"-qualified namespace)`; consulted by `tcl-lsp-core`'s
    /// `innermost_namespace_at` *before* the ordinary lexical scope-chain
    /// walk, since the `Scope` subtree `handle_apply_command` builds for
    /// `ns` is rooted under freshly-constructed, `body_span`-less
    /// namespace wrapper nodes (`per_item::reconstruct_proc_scope`) that
    /// the walk's span-containment descent can never reach.
    pub namespace_overrides: Vec<(Span, String)>,
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
    /// **Namespace-qualified** object-command bindings — one record per
    /// `CLASS create NAME` site whose `CLASS` resolves to a user class.
    ///
    /// `created_instance_commands` above keeps only the bare name as written,
    /// which is namespace-blind: `::a::Factory create rex` and `::b::Widget
    /// create rex` are two *different* commands (tclsh 9.0.4 / 8.6.16 both run
    /// `rex make` inside `::a` against `::a::rex` and inside `::b` against
    /// `::b::rex`, and both object commands coexist), but a bare-name set
    /// cannot tell them apart — so find-references and rename cross-linked the
    /// two, and a rename of one class's method rewrote the other's call site
    /// (issue #981, object-command half).
    ///
    /// Each record carries the qualified command name the creation site's own
    /// namespace produces and the qualified name of the creating class, so the
    /// dispatch scanner can resolve a written head against the call site's
    /// lexical namespace instead of comparing text.  A `Vec`, not a map: two
    /// creations of the same bare name in different namespaces are both real,
    /// and two creations of the same *qualified* name by different classes are
    /// a genuine ambiguity the rename safety gate refuses on rather than
    /// silently resolving last-write-wins.
    pub instance_command_bindings: Vec<InstanceCommandBinding>,
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
    /// and variable/command resolution work); this map carries each
    /// declaration plus its `oo::objdefine` **site offset**, so a consumer
    /// keys the record by the receiver's *binding identity* — the scope
    /// declaring the variable — never by the textual tail alone (two
    /// unrelated locals both named `o` in different procs are different
    /// objects; issue #945 fault 5).
    pub object_methods: HashMap<String, Vec<ObjectMethodDef>>,
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

    /// Whether byte offset `off` falls inside any recorded proc or class
    /// definition body — code there runs at *call* time, not load time, so
    /// anything it declares (a nested `proc`, a `rename`, an alias) exists
    /// only conditionally, when and if the enclosing definition is actually
    /// invoked. Shared by the analyser's own W123 gate
    /// (`registry_name_deleted_before`) and `tcl-lsp-core`'s call-target
    /// resolver (`resolve_called_proc`), so both agree on what "load-time"
    /// means for the same underlying question: a `proc ::set {...}` written
    /// inside another proc's body must not permanently outrank the real
    /// `set` builtin for every call site in the workspace, the same way a
    /// `rename ::set {}` written there must not permanently un-resolve it.
    #[must_use]
    pub fn offset_is_inside_any_definition_body(&self, off: u32) -> bool {
        self.all_procs
            .values()
            .map(|p| p.body_span)
            .chain(self.all_classes.values().map(|c| c.body_span))
            .any(|span| span.start() <= off && off < span.end())
    }

    /// Every declaration of `qualified` in this document, in source order —
    /// the definitions a later same-named `proc` displaced
    /// ([`Self::superseded_procs`]) followed by the winner
    /// ([`Self::all_procs`]).  One element for the overwhelmingly common
    /// single-declaration case.
    pub fn proc_declarations<'a>(
        &'a self,
        qualified: &str,
    ) -> impl DoubleEndedIterator<Item = &'a ProcDef> + 'a {
        self.superseded_procs
            .get(qualified)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .chain(self.all_procs.get(qualified))
    }

    /// The definition of `qualified` a call at `call_off` actually reaches.
    ///
    /// Order-gated the same way every other command-table fact is
    /// ([`crate::analyser::indirection::in_effect`], which shares the rule):
    /// at load time the latest declaration written before the call wins, and
    /// a call *inside a definition body* additionally sees every declaration
    /// written outside that body, wherever it sits in the file, because the
    /// whole file loads — running every top-level `proc` — before any body
    /// runs.
    ///
    /// That load-before-body shortcut stops at the body's own edge.  A `proc`
    /// written as a statement *of the body now executing* is an ordinary
    /// statement of the running script, so it is gated by offset like any
    /// other, and — having run later than everything the file's load
    /// installed — it outranks them all.  Oracle (tclsh 8.6.14 and 9.0.4) for
    /// `proc outer {} { proc p {} {return one}; p; proc p {a} {…} }`: the bare
    /// `p` between the two returns `one`, and `p X` after them dispatches the
    /// one-parameter definition, while the counterpart
    /// `proc outer {} { later }` / `proc later {} {…}` — declared outside,
    /// after — resolves fine (`LATER`), which is what the shortcut exists for.
    ///
    /// A call that precedes *every* declaration reaches none of them, but the
    /// winner is still returned so a consumer that has its own order gate
    /// (go-to-definition's lenient "show me the declaration anyway") behaves
    /// exactly as it did before redefinitions were tracked.
    ///
    /// Oracle (tclsh 8.6.14 and 9.0.4) for `proc p {} {return first}` / `p` /
    /// `proc p {a} {…}` / `p x`: the first call returns `first`, the second
    /// dispatches the one-parameter definition, and a bare `p` after the
    /// redefinition fails `wrong # args: should be "p a"`.
    #[must_use]
    pub fn proc_def_in_effect_at(&self, qualified: &str, call_off: u32) -> Option<&ProcDef> {
        let winner = self.all_procs.get(qualified)?;
        let Some(earlier) = self.superseded_procs.get(qualified) else {
            return Some(winner);
        };
        let declarations = || earlier.iter().chain(std::iter::once(winner));
        let Some(body) = self.innermost_definition_body_span(call_off) else {
            // Load time: plain textual order.
            return declarations()
                .rfind(|def| def.name_span.start() < call_off)
                .or(Some(winner));
        };
        let in_this_body = |def: &&ProcDef| {
            let at = def.name_span.start();
            body.start() <= at && at < body.end()
        };
        // A declaration this body already executed beats everything the file's
        // load installed; failing that, the last declaration from outside.
        declarations()
            .rfind(|def| in_this_body(def) && def.name_span.start() < call_off)
            .or_else(|| declarations().rfind(|def| !in_this_body(def)))
            .or(Some(winner))
    }

    /// The body span of the *innermost* recorded proc or class definition
    /// containing `off` — the body that is actually executing when the
    /// statement at `off` runs — or `None` at load-time (top level).
    ///
    /// The span form of [`Self::enclosing_definition_qualified_name`], for the
    /// order-gating rules that need to ask "is this other statement part of
    /// the *same* running body, or did it already run at load time?" —
    /// [`Self::proc_def_in_effect_at`] and
    /// [`crate::analyser::indirection::in_effect`].
    #[must_use]
    pub fn innermost_definition_body_span(&self, off: u32) -> Option<Span> {
        self.all_procs
            .values()
            .map(|p| p.body_span)
            .chain(self.all_classes.values().map(|c| c.body_span))
            .filter(|span| span.start() <= off && off < span.end())
            .min_by_key(|span| span.end() - span.start())
    }

    /// The qualified name of the *innermost* recorded proc or class
    /// definition whose body contains offset `off`, or `None` when `off`
    /// isn't inside any recorded definition body.  Bodies can nest (a
    /// `proc` written inside another proc's or a class's body), so this
    /// picks the smallest enclosing span rather than an arbitrary match —
    /// the definition whose body most tightly wraps `off` is the one that
    /// actually runs when the call at `off` executes.
    #[must_use]
    pub fn enclosing_definition_qualified_name(&self, off: u32) -> Option<&str> {
        self.all_procs
            .values()
            .map(|p| (p.qualified_name.as_str(), p.body_span))
            .chain(
                self.all_classes
                    .values()
                    .map(|c| (c.qualified_name.as_str(), c.body_span)),
            )
            .filter(|(_, span)| span.start() <= off && off < span.end())
            .min_by_key(|(_, span)| span.end() - span.start())
            .map(|(qn, _)| qn)
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

/// One **namespace-qualified** variable occurrence — a read or a write whose
/// *written* name carries a `::` qualifier (`$::tomato::helper::TolEquals`,
/// `set app::colors::palette …`).
///
/// Recorded regardless of whether the named cell is declared in this document,
/// which is exactly what makes it useful: the declaring `namespace eval NS {
/// variable v }` routinely lives in a sibling file, so the occurrence resolves
/// to nothing locally and leaves no trace in the scope tree
/// ([`Scope::variables`] / [`VarDef::references`] only ever record a *resolved*
/// use).  The workspace index lifts these into the cross-document variable
/// reference set (issue #923 differential-audit findings idx 65 / 75 / 78).
///
/// Deliberately **qualified-only**: an unqualified `$v` names whichever cell
/// the local scope chain supplies, which is a per-document question, so
/// indexing those would be whole-program variable indexing with no
/// statically-sound cross-file meaning.  A qualified name, by contrast, names
/// one cell in one namespace no matter where it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedVarRef {
    /// The `::`-rooted cell the occurrence names (`::tomato::helper::TolEquals`),
    /// resolved against the occurrence's own namespace for a relative
    /// qualifier (`app::colors::palette` at global scope → `::app::colors::palette`).
    pub qualified_name: String,
    /// Byte span of the name token as written.
    pub span: Span,
}

/// One occurrence of a word that **names a namespace** — every argument the
/// registry marks [`tcl_registry::ArgRole::NamespaceName`] (`namespace
/// children ::tomato`, `namespace exists ns`, `namespace delete ::a ::b`,
/// `namespace eval NS body`, `namespace inscope NS script`, `namespace upvar
/// NS v local`).
///
/// The namespace analogue of [`QualifiedVarRef`], and it exists for the same
/// reason: the declaring `namespace eval` block routinely lives elsewhere —
/// another block in the same file, or a sibling document — so the occurrence
/// leaves no trace in the scope tree, which only records a namespace as a
/// *container* ([`Scope`] with [`ScopeKind::Namespace`]) and never as a
/// referenceable symbol (issue #1088).
///
/// Unlike the variable table this one records **relative** names too, because
/// there is nothing per-document about them: a namespace word roots against
/// the command-resolution namespace in effect at the occurrence, and that
/// namespace is a lexical fact this document already knows. Pinned on tclsh
/// 9.0.4 and 8.6.16, byte-identically: inside `namespace eval ::outer`,
/// `namespace exists inner` answers `1` for `::outer::inner` while the same
/// words at global scope answer `0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceRef {
    /// The `::`-rooted namespace the occurrence names, with a relative
    /// spelling already rooted against the occurrence's own namespace.
    pub qualified_name: String,
    /// Byte span of the name token as written.
    pub span: Span,
    /// `true` when this occurrence is also a **declaring** site — the name
    /// word of a [`tcl_registry::Traits::DECLARES_NAMESPACE`] command
    /// (`namespace eval`).  Go-to-definition answers with these; the
    /// reference set is everything else.
    pub declares: bool,
}

/// One `CLASS create NAME` object-command binding, namespace-qualified.
///
/// See [`AnalysisResult::instance_command_bindings`] for why the bare-name
/// set it accompanies is not enough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceCommandBinding {
    /// The object command's `::`-rooted qualified name — the creation site's
    /// own namespace applied to `NAME` as written (an already-qualified
    /// `NAME` is used as written).
    pub qualified_name: String,
    /// The `::`-rooted qualified name of the class whose `create` bound it.
    pub class_q: String,
}

/// Which `auto_path` mutation wrote an [`AutoPathEntry`].
///
/// The two forms differ in **arity**, so a consumer must not read them alike
/// (verified against `tclsh8.6` / `tclsh9.0`):
///
/// | Written | `llength $auto_path` gains |
/// |---------|----------------------------|
/// | `lappend auto_path a b` | 2 — one element per argument *word* |
/// | `lappend auto_path {p q}` | 1 — the word `p q`, spaces and all |
/// | `set auto_path {/o/p1 /o/p2}` | 2 — the right-hand side is a *list* |
/// | `set auto_path {/o/a {/o/w s} /o/b}` | 3 — the braced element stays one |
///
/// [`crate::auto_path_eval::evaluate_auto_path_entry`] is the consumer that
/// applies the distinction; it is the only supported way to turn one of these
/// records into directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoPathForm {
    /// `lappend auto_path WORD …` — one record per argument word, and that
    /// word is exactly one directory however many spaces it contains.
    Append,
    /// `set auto_path LIST` — one record holding the *whole* right-hand side,
    /// which is a Tcl list and may therefore name several directories.
    Assign,
}

/// `auto_path` mutation record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoPathEntry {
    /// Raw path text as written.  One directory for [`AutoPathForm::Append`];
    /// a Tcl *list* of them for [`AutoPathForm::Assign`].
    pub raw_path: String,
    /// Span of the path argument.
    pub range: Span,
    /// Which mutation wrote this record — see [`AutoPathForm`].
    pub form: AutoPathForm,
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
    fn class_member_key_disambiguates_method_and_classmethod() {
        assert_eq!(
            class_member_key("::Bar", "get", false),
            "::Bar::method::get"
        );
        assert_eq!(
            class_member_key("::Bar", "get", true),
            "::Bar::classmethod::get"
        );
        // Same class, same member name, different map — must not collide.
        assert_ne!(
            class_member_key("::Bar", "get", false),
            class_member_key("::Bar", "get", true)
        );
    }

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
