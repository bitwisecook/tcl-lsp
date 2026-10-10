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

use std::collections::{BTreeMap, HashMap, HashSet};

mod original_configurations;
mod original_constructor_call;
mod original_member_context;
mod original_members;
mod original_properties;
mod original_receiver_body;
mod original_relations;
mod original_special_members;
pub use original_configurations::{
    OriginalSourceClassConfiguration, OriginalSourceObjectConfiguration,
};
pub use original_constructor_call::{OriginalConstructorArityAdvice, OriginalConstructorArityCall};
pub use original_member_context::{OriginalLexicalMemberContext, OriginalLexicalNextCall};
pub use original_members::{
    OriginalSourceMemberDeclaration, OriginalSourceMemberEffect, OriginalSourceMemberEffectKind,
    OriginalSourceMemberLedger, OriginalSourceMethodMetadata,
};
pub use original_properties::{OriginalSourcePropertyLedger, OriginalSourcePropertyMetadata};
pub use original_receiver_body::OriginalSourceReceiverBodyDeclaration;
pub use original_relations::{
    OriginalSourceClassRelation, OriginalSourceClassRelationEffect,
    OriginalSourceClassRelationKind, OriginalSourceClassRelationLedger,
};
pub use original_special_members::{
    OriginalSourceSpecialMemberLedger, OriginalSourceSpecialMemberMetadata,
};

use tcl_lexer::{Span, Token};

use crate::signature_scan::types::{
    ParamDef, SignatureCommandAlias, SignatureCommandInvocation, SignatureNamespaceExport,
    SignatureNamespaceForget, SignatureNamespaceImport, SignaturePackagePrefer,
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
/// Each fix carries a [`FixSafety`] classification recording how much the
/// rewrite changes behaviour; "Fix All Safe Issues" applies only the
/// provably equivalent ones.
pub use crate::irules_checks::{CodeFix, FixSafety};

/// Diagnostic emitted by the analyser.
///
/// Carries a stable ``code`` (e.g. ``"W210"``), the source
/// [`Span`] the diagnostic anchors to, a one-line ``message``, a
/// [`Severity`], and optional [`CodeFix`] suggestions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Optional typed subject from the emitting semantic owner.
    pub subject: Option<super::DiagnosticSubject>,
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

impl Diagnostic {
    /// Construct a diagnostic without a suggested fix.
    ///
    /// Emitters that can offer a source edit add it with
    /// [`Self::with_fix`] or [`Self::with_fixes`]. Keeping the empty list
    /// here makes the common, fix-less case explicit without repeating the
    /// representation detail at every emission site.
    #[must_use]
    pub fn new(code: DiagCode, span: Span, message: impl Into<String>, severity: Severity) -> Self {
        Self {
            code,
            span,
            message: message.into(),
            severity,
            fixes: Vec::new(),
            subject: None,
        }
    }

    /// Retain the genuine semantic subject independently of reporting text.
    #[must_use]
    pub fn with_subject(mut self, subject: super::DiagnosticSubject) -> Self {
        self.subject = Some(subject);
        self
    }

    /// Borrow the retained semantic subject, if the emitter could authenticate it.
    #[must_use]
    pub fn subject(&self) -> Option<&super::DiagnosticSubject> {
        self.subject.as_ref()
    }

    /// Selected package database key, without parsing the diagnostic message.
    #[must_use]
    pub fn required_package_key(
        &self,
    ) -> Option<&tcl_registry::native_package::NativePackageNameKey> {
        match self.subject()? {
            super::DiagnosticSubject::RequiredPackage(key) => Some(key),
            super::DiagnosticSubject::RegistrySource(source) => source.required_package_key(),
            super::DiagnosticSubject::CallbackSourceArity(_)
            | super::DiagnosticSubject::ObjectSourceArity(_)
            | super::DiagnosticSubject::CommandAvailability(_)
            | super::DiagnosticSubject::RegisteredInstanceSource(_)
            | super::DiagnosticSubject::DeclaredSource(_)
            | super::DiagnosticSubject::UnresolvedCommand(_)
            | super::DiagnosticSubject::UnresolvedMathFunction(_)
            | super::DiagnosticSubject::ConditionalInterpreterVisibility(_) => None,
        }
    }

    /// Original unresolved name and source invocation, without message parsing.
    #[must_use]
    pub fn unresolved_command(&self) -> Option<&super::SourceUnresolvedCommandSubject> {
        match self.subject()? {
            super::DiagnosticSubject::UnresolvedCommand(subject) => Some(subject),
            super::DiagnosticSubject::CallbackSourceArity(_)
            | super::DiagnosticSubject::ObjectSourceArity(_)
            | super::DiagnosticSubject::CommandAvailability(_)
            | super::DiagnosticSubject::RegisteredInstanceSource(_)
            | super::DiagnosticSubject::DeclaredSource(_)
            | super::DiagnosticSubject::RegistrySource(_)
            | super::DiagnosticSubject::RequiredPackage(_)
            | super::DiagnosticSubject::UnresolvedMathFunction(_)
            | super::DiagnosticSubject::ConditionalInterpreterVisibility(_) => None,
        }
    }

    /// Original unresolved expression function, without a command-head issuer.
    #[must_use]
    pub fn unresolved_math_function(&self) -> Option<&super::SourceUnresolvedMathFunctionSubject> {
        match self.subject()? {
            super::DiagnosticSubject::UnresolvedMathFunction(subject) => Some(subject),
            super::DiagnosticSubject::CallbackSourceArity(_)
            | super::DiagnosticSubject::ObjectSourceArity(_)
            | super::DiagnosticSubject::CommandAvailability(_)
            | super::DiagnosticSubject::RegisteredInstanceSource(_)
            | super::DiagnosticSubject::DeclaredSource(_)
            | super::DiagnosticSubject::RegistrySource(_)
            | super::DiagnosticSubject::RequiredPackage(_)
            | super::DiagnosticSubject::UnresolvedCommand(_)
            | super::DiagnosticSubject::ConditionalInterpreterVisibility(_) => None,
        }
    }

    /// Conditional child source visibility and its explicit assumptions.
    /// This is not a selected hidden callable or an effects exclusion.
    #[must_use]
    pub fn conditional_interpreter_visibility(
        &self,
    ) -> Option<&super::ConditionalInterpreterVisibilitySubject> {
        match self.subject()? {
            super::DiagnosticSubject::ConditionalInterpreterVisibility(subject) => Some(subject),
            super::DiagnosticSubject::CallbackSourceArity(_)
            | super::DiagnosticSubject::ObjectSourceArity(_)
            | super::DiagnosticSubject::CommandAvailability(_)
            | super::DiagnosticSubject::RegisteredInstanceSource(_)
            | super::DiagnosticSubject::DeclaredSource(_)
            | super::DiagnosticSubject::RegistrySource(_)
            | super::DiagnosticSubject::RequiredPackage(_)
            | super::DiagnosticSubject::UnresolvedCommand(_)
            | super::DiagnosticSubject::UnresolvedMathFunction(_) => None,
        }
    }

    /// Original whole-command exclusion metadata, independent of reporting text.
    #[must_use]
    pub fn command_availability(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceCommandAvailability> {
        match self.subject.as_ref()? {
            super::DiagnosticSubject::CommandAvailability(subject) => Some(subject),
            _ => None,
        }
    }

    /// Original Registry source syntax retained independently of presentation.
    #[must_use]
    pub fn registry_source(&self) -> Option<&super::RegistrySourceDiagnosticSubject> {
        match self.subject()? {
            super::DiagnosticSubject::RegistrySource(subject) => Some(subject),
            _ => None,
        }
    }

    /// Authored declaration purpose and original operands, without message parsing.
    #[must_use]
    pub fn declared_source(&self) -> Option<&super::DeclaredSourceDiagnosticSubject> {
        match self.subject()? {
            super::DiagnosticSubject::DeclaredSource(subject) => Some(subject),
            _ => None,
        }
    }

    /// Original lexical object signature ownership, without receiver dispatch.
    #[must_use]
    pub fn object_source_arity(&self) -> Option<&super::ObjectSourceAritySubject> {
        match self.subject()? {
            super::DiagnosticSubject::ObjectSourceArity(subject) => Some(subject),
            _ => None,
        }
    }

    /// Conditional callback signature advice, without future dispatch proof.
    #[must_use]
    pub fn callback_source_arity(&self) -> Option<&super::SourceCallbackAritySubject> {
        match self.subject()? {
            super::DiagnosticSubject::CallbackSourceArity(subject) => Some(subject),
            _ => None,
        }
    }

    /// Genuine registered-instance source metadata and its diagnostic purpose.
    #[must_use]
    pub fn registered_instance_source(
        &self,
    ) -> Option<&super::RegisteredInstanceSourceDiagnosticSubject> {
        match self.subject()? {
            super::DiagnosticSubject::RegisteredInstanceSource(subject) => Some(subject),
            _ => None,
        }
    }

    /// Attach one suggested edit.
    #[must_use]
    pub fn with_fix(mut self, fix: CodeFix) -> Self {
        self.fixes.push(fix);
        self
    }

    /// Attach the complete set of suggested edits computed by an emitter.
    #[must_use]
    pub fn with_fixes(mut self, fixes: Vec<CodeFix>) -> Self {
        self.fixes = fixes;
        self
    }
}

/// One statically recorded `namespace ensemble` subcommand: the command it
/// dispatches to, plus **how** the ensemble bound the two together.
///
/// The value half of
/// [`AnalysisResult::ensemble_subcommand_targets`]'s inner map. Navigation
/// (`definition` / `hover` / `references`) only wants [`Self::target`]; a
/// source-rewriting consumer (rename) has to read [`Self::provenance`] as
/// well, because the subcommand word is the target's own tail under
/// `-subcommands` and an arbitrary key under `-map`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnsembleSubcommandTarget {
    /// The resolved, qualified command name the subcommand dispatches to
    /// (`::app::widget::Show`). For a `-map` target that is a command
    /// *prefix* (`-map {go {string length}}`) this is the prefix's head —
    /// the command actually invoked.
    pub target: String,
    /// Which option declared the mapping.
    pub provenance: crate::signature_scan::types::EnsembleSubcommandProvenance,
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

/// Queued genuine source factory/call signature, with no runtime receiver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCtorArity {
    /// Complete source-call issuer and independent source-order requirement.
    pub original: std::sync::Arc<OriginalConstructorArityCall>,
    /// Reporting label, never a class/provider selection key.
    pub display_name: String,
}

/// A queued readonly next-chain signature question with exact source identity.
/// Declaration context and Registry source shape supply no entered receiver,
/// installed method, current dispatch or successful completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingNextArity {
    /// Retained declaration/body and complete selected source invocation.
    pub original: std::sync::Arc<OriginalLexicalNextCall>,
    /// Reporting label used only in the eventual diagnostic.
    pub display_name: String,
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

impl VarDef {
    /// Record `span` as a read site, ignoring a span already present.
    ///
    /// A byte span is one source location, so the same location can never be
    /// two reads of the same cell: a repeat means two recorders saw the same
    /// word.  That is expected now that the registry `VarRead` role records
    /// name words generically *and* a handful of commands still
    /// record their own read through a dedicated handler (`set`'s one-argument
    /// form).  Deduping at the sink keeps both recorders honest instead of
    /// making one of them conditional on the other.
    pub fn push_reference(&mut self, span: Span) {
        if !self.references.contains(&span) {
            self.references.push(span);
        }
    }
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
    /// Exact authored publication geometry, independent of this declaration's report.
    pub source_name: Option<crate::signature_scan::scope::SignatureSourceCommand>,
    /// Proc name as written (no namespace qualifiers).
    pub name: String,
    /// Fully-qualified proc name with leading ``::``.
    pub qualified_name: String,
    /// Parameter list in declaration order.
    ///
    /// Empty *and* [`Self::params_computed`] set means "unknown", not
    /// "none" — see that field.
    pub params: Vec<ParamDef>,
    /// Original formal count owner or explicit authored metadata. Display
    /// parameter labels cannot recreate an original binding/count recipe.
    pub formal_count: crate::signature_scan::formal_count::SourceFormalCount,
    /// The parameter-list word was **computed**, so the proc's formals are
    /// unmodelled.
    ///
    /// `proc p [makeargs] {…}` / `proc q $params {…}` build the formal list at
    /// definition time from a value; which names it declares, and how many,
    /// are run-time facts (tclsh 9.0.4 / 8.6.16: with
    /// `proc makeargs {} {return {a b}}`, `proc p [makeargs] {…}` then
    /// `info args p` → `a b`, and `p 1 2` runs).  Reading the unresolved word
    /// as a one-parameter literal would register a `VarDef` whose *name is the
    /// source text* (`"[makeargs]"`) and make the call-site arity checker
    /// demand exactly one argument.  Nothing is claimed instead: no
    /// per-parameter `VarDef`, and the arity resolvers abstain
    /// (`0..unlimited`).
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
    /// **Literal** caller-frame names this proc binds in its immediate
    /// caller's frame — spelled in the proc's *own* body (`upvar 1 name
    /// name`), so no call-site argument word carries them.
    /// `name → written-through-alias`: `true` means
    /// the conditional callee template writes through the alias, `false` that
    /// it only reads through it. This proves no actual caller cell or completed
    /// write. Navigation requires a separate original call/allocation/frame
    /// receipt. Populated by
    /// [`super::param_traits::caller_frame_literal_targets`]; empty when
    /// the body binds none.
    pub caller_frame_literals: std::collections::HashMap<String, bool>,
}

impl ProcDef {
    /// Callable source spelling only when both publication and lookup round-trip.
    #[must_use]
    pub fn source_spelling(&self) -> Option<String> {
        self.source_name.as_ref()?.source_spelling()
    }
    /// The proc's declared argument arity — or the **abstaining**
    /// `0..unlimited` when its parameter list was computed
    /// ([`Self::params_computed`]).
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
        self.formal_count_projection().arity()
    }

    /// Stable descriptive header count, without source bytes or runtime grants.
    #[must_use]
    pub fn formal_count_projection(
        &self,
    ) -> crate::signature_scan::formal_count::SourceFormalCountProjection {
        self.formal_count
            .projection(&self.params, self.params_computed)
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
    /// Independently produced original name under positioned Registry advice.
    /// None belongs only to explicitly logical declaration reporting; this is
    /// neither a command publication nor an editable computed name.
    pub original_name_input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
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
    ///
    /// Empty *and* [`Self::params_computed`] set means "unknown", not
    /// "none" — see that field.
    pub params: Vec<ParamDef>,
    /// Original formal count owner or explicit authored metadata. Display
    /// parameter labels cannot recreate an original binding/count recipe.
    pub formal_count: crate::signature_scan::formal_count::SourceFormalCount,
    /// The method's formals are unmodelled: either the parameter-list word
    /// was itself computed, or — the case this field was added for
    /// — the *method* itself was installed by a literal
    /// loop (`foreach m {alpha beta gamma} { method $m {args} {…} }`)
    /// whose per-iteration name this walk can read off the loop's own
    /// literal list, but whose per-iteration signature it deliberately
    /// does not attempt to re-derive (a reflective installer's parameter
    /// list can itself be computed per iteration — `method $m {*}[classDef
    /// $m]` — so treating one iteration's literal `{args}` as
    /// representative of every other would be a fabrication for the
    /// general shape this exists to cover, even on the rare iteration
    /// where it happens to be right).
    ///
    /// Mirrors [`crate::analyser::ProcDef::params_computed`] exactly: every
    /// arity-shaped consumer must ask [`Self::arity`] rather than compute
    /// one from [`Self::params`] directly, so the abstention cannot be
    /// forgotten at one call site.
    pub params_computed: bool,
    /// Source span of the name token.
    pub name_span: Span,
    /// Source span of the method body (braces excluded).
    pub body_span: Span,
    /// Method kind: ``"method"`` / ``"classmethod"`` /
    /// ``"forward"`` / ``"constructor"`` / ``"destructor"``.
    pub kind: String,
    /// `true` only for a `classmethod`-kind entry declared via `TclOO`'s
    /// `self` wrapper (`self method NAME …`) directly on this class.
    /// Unlike `ooutil`'s `classmethod` keyword —
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

impl MethodDef {
    /// The method's declared argument arity — or the **abstaining**
    /// `0..unlimited` when its parameter list is unmodelled
    /// ([`Self::params_computed`]).
    ///
    /// Exactly [`ProcDef::arity`]'s contract, extended to `MethodDef`: every
    /// arity consumer (`$obj method` call-site checking, `next`/`nextto`
    /// dispatch) asks here rather than calling
    /// [`crate::signature_scan::arity::arity_of`] on [`Self::params`]
    /// directly, so a member whose name is all this walk could honestly
    /// record never has its absent parameter list mistaken for a
    /// zero-argument one.
    #[must_use]
    pub fn arity(&self) -> tcl_registry::Arity {
        self.formal_count_projection().arity()
    }

    /// Stable descriptive header count, without source bytes or runtime grants.
    #[must_use]
    pub fn formal_count_projection(
        &self,
    ) -> crate::signature_scan::formal_count::SourceFormalCountProjection {
        self.formal_count
            .projection(&self.params, self.params_computed)
    }
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

/// One per-object method added by an `oo::objdefine`: the method declaration
/// plus the **objdefine site's**
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

/// Effective **per-object member state** for one receiver binding — the
/// durable home for `oo::objdefine` effects, folded in source
/// order across every block on the same binding (the same binding identity
/// [`ObjectMethodDef::objdefine_offset`] anchors: the innermost proc / method
/// body declaring the receiver variable, or the top level).
///
/// [`AnalysisResult::object_methods`] stays the per-block *declaration*
/// record (navigation to each declared member, reassignment abstention);
/// this is the *folded* record — what the object's own table and visibility
/// look like after the last walked block — which is what dispatch-order
/// consumers (definition, completion) and the `oo::objdefine` W315 need.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectMemberState {
    /// The object's own member table after every walked block: declared
    /// methods, minus retractions, with renames applied — each entry
    /// carrying its effective visibility.
    pub methods: HashMap<String, MethodDef>,
    /// Names this binding's blocks explicitly `export` — including names the
    /// object's *class* provides, which is exactly the flip that had nowhere
    /// to live.  Kept mutually exclusive with
    /// [`Self::unexports`], last writer wins, the same contract as the
    /// class-side pairs.
    pub exports: std::collections::HashSet<String>,
    /// Names explicitly `unexport`ed — see [`Self::exports`].
    pub unexports: std::collections::HashSet<String>,
    /// Byte offset of the binding's first `oo::objdefine` receiver word —
    /// the anchor a consumer resolves to a variable binding, exactly as
    /// [`ObjectMethodDef::objdefine_offset`] is resolved.
    pub anchor_offset: u32,
    /// True when any contributing block ran under a conditional: the
    /// cross-block table is then not order-provable, and the per-object
    /// W315 abstains rather than judge retractions against it.
    pub conditional: bool,
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
/// from also un-exporting an identically-named instance method,
/// and what lets a cross-document retraction record which table it removes from.
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

    /// This side's `(exports, unexports)` visibility sets — the pair
    /// `export` / `unexport` writes when scoped to this side.
    ///
    /// The sided counterpart of [`Self::table`], and the reason
    /// `apply_visibility_member` has no `if side == Instance` branch: both
    /// sides take the identical last-writer-exclusive update, they just land in
    /// a different pair.  [`ClassDef::exports`] / [`ClassDef::unexports`] are
    /// the **instance**-side pair by contract and
    /// [`ClassDef::class_exports`] / [`ClassDef::class_unexports`] the
    /// class-object-side one.
    #[must_use]
    pub fn visibility_sets(
        self,
        class_def: &mut ClassDef,
    ) -> (&mut HashSet<String>, &mut HashSet<String>) {
        match self {
            Self::Instance => (&mut class_def.exports, &mut class_def.unexports),
            Self::ClassObject => (&mut class_def.class_exports, &mut class_def.class_unexports),
        }
    }

    /// This side's declared method-filter list — what `filter` (instance) and
    /// `self filter` (class object) each assign.
    ///
    /// The two are genuinely separate slots in `TclOO`, not one list read
    /// twice, and they intercept different dispatches (see
    /// [`ClassDef::filters`] / [`ClassDef::class_filters`] for the oracle).
    #[must_use]
    pub fn filter_list(self, class_def: &mut ClassDef) -> &mut Vec<String> {
        match self {
            Self::Instance => &mut class_def.filters,
            Self::ClassObject => &mut class_def.class_filters,
        }
    }
}

/// A reason one `TclOO` definition body **cannot run at all** — real Tcl
/// aborts the whole `oo::class create` / `oo::define` and creates no class.
///
/// Recorded by the member walker where the retracting word is applied (the one
/// site that knows the side's table state at that point in the body) and
/// drained by the class handlers, which turn each into a `W315`.  Every arm is
/// oracle-pinned byte-identical on tclsh 9.0.4 and 8.6.14, and each carries the
/// interpreter's own error text so the diagnostic reads as the failure the user
/// will actually hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionAbortKind {
    /// A retracting word (`deletemethod m`, `renamemethod m x`) named a member
    /// that does not exist **on the side the word is scoped to** — either never
    /// declared, or declared only on the other side.
    ///
    /// ```tcl
    /// oo::class create ::E1 { deletemethod ghost ; method ghost {} {} }
    /// ;# -> method ghost does not exist        (and ::E1 is never created)
    /// oo::class create ::E2 { self { method cm {} {} } ; deletemethod cm }
    /// ;# -> method cm does not exist
    /// ```
    MissingMember,
    /// `renamemethod a b` where `b` is already a member of the same side.
    ///
    /// ```tcl
    /// oo::class create ::E3 { method a {} {} ; method b {} {} ; renamemethod a b }
    /// ;# -> method called b already exists
    /// ```
    ///
    /// Side-scoped like everything else: `method a` + `self method b` +
    /// `renamemethod a b` is legal and leaves the instance side with `b`.
    DestinationExists,
    /// `renamemethod a a` — the source and destination are the same name.
    ///
    /// ```tcl
    /// oo::class create ::A4 { method a {} {} ; renamemethod a a }
    /// ;# -> cannot rename method to itself
    /// ```
    RenameToItself,
}

/// One member a record **retracts without declaring** — the cross-document
/// tombstone described on
/// [`ClassDef::retracted_members`](crate::analyser::ClassDef::retracted_members).
///
/// Carries a *destination*: a `renamemethod old new` in a
/// cross-file `oo::define` stub does not merely delete `old`, it moves the
/// member to `new`, and the stub has no [`MethodDef`] of its own to move — the
/// member's params / body / visibility live in the defining file's record.
/// Recording the arrival name here is what lets the workspace join re-key that
/// other record's member, so `new` appears instead of the member vanishing.
///
/// `arrival` is `None` for a plain `deletemethod`, and also for a
/// `renamemethod old $new` whose destination is computed — that names nothing
/// statically, so the retraction stands alone and the move abstains, the
/// direction this campaign abstains toward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRetractionRecord {
    /// The member name retracted.
    pub member: String,
    /// The side the retracting word was scoped to.
    pub side: MemberSide,
    /// The name the member arrives under, when the retracting word was a
    /// **move** rather than a deletion.
    pub arrival: Option<String>,
    /// Span of the arrival word — the synthetic declaration site the moved
    /// member gets, and therefore what go-to-definition on the new name
    /// answers with.  `None` whenever [`Self::arrival`] is.
    pub arrival_span: Option<Span>,
}

impl MemberRetractionRecord {
    /// A plain retraction with no destination.
    #[must_use]
    pub fn deletion(member: String, side: MemberSide) -> Self {
        Self {
            member,
            side,
            arrival: None,
            arrival_span: None,
        }
    }
}

/// One `renamemethod` that successfully **moved** a member, with the member
/// state the move ran against.
///
/// Recording the move is what lets a *rename* of the arrived member be checked:
/// its declaration site is the `renamemethod`'s destination word, so renaming
/// it rewrites that word and nothing else — turning `renamemethod old new` into
/// `renamemethod old X`. For some `X` that is a body real Tcl refuses to run,
/// and the request has to be refused rather than answered with edits that make
/// the class invalid.
///
/// [`Self::abort_if_renamed_to`] is the single fold both consumers use: the
/// member walker asks it about the destination actually written (that is how
/// `W315` is decided), and the rename gate asks it about the name the user
/// typed. Neither re-derives the rule, so the diagnostic and the gate cannot
/// drift apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamedMember {
    /// The name the member was declared under, retracted by this move.
    pub source: String,
    /// The name it arrived under — the key it now occupies in `side`'s table.
    pub destination: String,
    /// The side the move happened on. A collision is side-local: `method a` +
    /// `self method b` + `renamemethod a b` is legal and leaves the instance
    /// side with `b` (tclsh 9.0.4 / 8.6.14).
    pub side: MemberSide,
    /// Span of the destination word — the moved member's synthetic name span,
    /// and therefore the one token a rename of it rewrites.
    pub destination_span: Span,
    /// Names live on `side` **at the moment the move ran**: the source already
    /// removed, the destination not yet inserted.
    ///
    /// A snapshot rather than the final table because the interpreter reads the
    /// table at that point too, and both directions of that matter:
    /// `deletemethod sib ; renamemethod old sib` is legal (`sib` is gone by
    /// then) and `renamemethod old sib ; method sib {} {}` is legal as well
    /// (`sib` does not exist yet) — both pinned identical on 9.0.4 and 8.6.14.
    pub blocked: Vec<String>,
}

impl RenamedMember {
    /// Why naming this moved member `candidate` would abort the whole class
    /// definition, or `None` when that name is legal here.
    ///
    /// The two shapes, oracle-pinned byte-identical on tclsh 9.0.4 and 8.6.14
    /// (no class is created in either case):
    ///
    /// ```tcl
    /// oo::class create ::A1 { method old {} {…} ; renamemethod old old }
    /// ;# -> cannot rename method to itself
    /// oo::class create ::B1 { method old {} {…} ; method sib {} {…}
    ///                         renamemethod old sib }
    /// ;# -> method called sib already exists
    /// ```
    #[must_use]
    pub fn abort_if_renamed_to(&self, candidate: &str) -> Option<DefinitionAbortKind> {
        if candidate == self.source {
            Some(DefinitionAbortKind::RenameToItself)
        } else if self.blocked.iter().any(|n| n == candidate) {
            Some(DefinitionAbortKind::DestinationExists)
        } else {
            None
        }
    }

    /// Whether renaming the **ordinary member** `member` to `candidate` would
    /// collide at *this* move's site — the mirror direction.
    ///
    /// `method old {} {…} ; method sib {} {…} ; renamemethod old new` is legal,
    /// but renaming `sib` to `new` rewrites the sibling's declaration and
    /// produces `method new {} {…} ; renamemethod old new`, which aborts with
    /// `method called new already exists`. The same recorded snapshot answers
    /// it: `member` was live here, and `candidate` is the name this move needs
    /// free.
    #[must_use]
    pub fn collides_with_renaming(&self, member: &str, candidate: &str) -> bool {
        candidate == self.destination && self.blocked.iter().any(|n| n == member)
    }
}

/// One recorded [`DefinitionAbortKind`] with the member name it names and the
/// span of the offending word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionAbort {
    /// Why the definition cannot run.
    pub kind: DefinitionAbortKind,
    /// The member name the offending word named.
    pub member: String,
    /// Span of the offending argument word — the name that does not exist, or
    /// the destination that is already taken.
    pub span: Span,
}

impl DefinitionAbort {
    /// The diagnostic message for this abort, mirroring the interpreter's own
    /// error text after a fixed "cannot run" preamble.
    #[must_use]
    pub fn message(&self) -> String {
        format!("this class definition cannot run: {}", self.reason())
    }

    /// [`Self::message`] for a **per-object** definition (`oo::objdefine`),
    /// whose failing script is an object definition, not a class one — the
    /// interpreter's own reason text is identical on both paths.
    #[must_use]
    pub fn object_message(&self) -> String {
        format!("this object definition cannot run: {}", self.reason())
    }

    /// The interpreter-mirroring reason text shared by both message forms.
    fn reason(&self) -> String {
        match self.kind {
            DefinitionAbortKind::MissingMember => {
                format!("method \"{}\" does not exist", self.member)
            }
            DefinitionAbortKind::DestinationExists => {
                format!("method called \"{}\" already exists", self.member)
            }
            DefinitionAbortKind::RenameToItself => "cannot rename method to itself".to_string(),
        }
    }
}

/// One word of a definition-body member a class factory splices into every
/// class it manufactures, as a **template** over the creation call's own
/// arguments.
///
/// The template is call-site independent, which is the whole point: it is
/// derived once, where the metaclass is written, and then resolved against
/// each `Meta create …` call — including one in a different file, which is
/// the only way the per-file walk can classify such a call at all.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FactoryWord {
    /// A word the manufacturer writes literally in its own body
    /// (`superclass ::tk::MegawidgetClass`), with the token it occupies
    /// **in the metaclass's own document**.
    Literal {
        /// The word's text.
        text: String,
        /// Its token in the defining document — usable only while resolving
        /// a call in that same document (see
        /// [`ClassFactory::resolve_in_other_document`]).
        token: Token,
    },
    /// A `{*}$param` splice of the creation call's argument at this index
    /// (argument 0 being the manufacturer subcommand itself), so the words
    /// it contributes are the caller's own and carry the caller's tokens.
    CallerSplice(usize),
}

/// One definition-body member a class factory injects, as a template.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FactoryMember {
    /// Member keyword followed by its argument words.
    pub words: Vec<FactoryWord>,
}

/// The word layout one manufacturer subcommand of a class factory imposes,
/// plus what it splices into every body it makes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManufacturerSpec {
    /// Argument index of the new class's name.
    pub name_arg: usize,
    /// Argument index of the new class's definition body.
    pub body_arg: usize,
    /// The members the manufacturer always injects, or `None` when its
    /// prologue could not be read — the
    /// [`ClassDef::inheritance_unknown`] case.
    pub injected: Option<Vec<FactoryMember>>,
}

/// Workspace-wide class factories, keyed by fully-qualified metaclass name —
/// what a host hands the analyser so a `Meta create …` call can be classified
/// when `Meta` is written in another document.
pub type ClassFactoryIndex = BTreeMap<String, ClassFactory>;

/// A creation call the walk could not classify, kept for a **second verdict**
/// once the parameterised-class observation join has settled.
///
/// A metaclass proved only by that post-pass does not exist while the walk
/// runs: `::T::D::class create ::T::W { … }` in the same document reads as a
/// call to an unknown command, so the class it makes is recorded nowhere.
/// Which verdict the head deserves is therefore not answerable during the
/// walk — the same situation version-floor arity resolution meets, and the
/// same answer: buffer the inputs, decide afterwards.
///
/// Only calls whose head names **nothing this document knows** are kept, which
/// is the one shape a later proof can turn into a creation; a head that
/// already resolves has had its verdict, right or wrong, and is not revisited.
#[derive(Debug, Clone)]
pub struct DeferredClassCreation {
    /// The call's head word, exactly as `handle_oo_class_command` received it.
    pub cmd_name: String,
    /// Its argument words.
    pub args: Vec<String>,
    /// Their tokens, for the name and body spans the replay records.
    pub arg_tokens: Vec<Token>,
    /// The scope the call was written in, so the replayed class homes where
    /// the walk would have homed it.
    pub scope_path: Vec<usize>,
    /// The head's own token — the call offset a rename chain resolves at.
    pub cmd_tok: Token,
}

/// Instance methods dispatchable on some workspace **descendant** of each
/// class, keyed by the ancestor's fully-qualified name — what a host hands
/// the analyser so the template-method abstention can see a
/// subclass written in another document.
///
/// `map["::Formatter"]` holds every instance-side member name reachable on
/// any workspace class that lists `::Formatter` anywhere in its
/// linearisation — including names the descendant inherits from a mixin,
/// and excluding `private` members, which `TclOO` hides even from a base
/// class's own `my` dispatch.  `BTreeMap`/`BTreeSet` so equal contents
/// compare equal regardless of build order (the host's compare-then-set
/// depends on it).
pub type SubclassProvidedMethods = BTreeMap<String, std::collections::BTreeSet<String>>;

/// How a user-defined `TclOO` metaclass manufactures classes.
///
/// Recorded on the metaclass's own [`ClassDef`] when it is written, so a
/// `Meta create Name …` call anywhere — same file or, through the workspace
/// factory index, another one — is classified from a fact that was *proved*
/// at the definition rather than guessed from the call's shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ClassFactory {
    /// The registry metaclass command at the root of this factory's
    /// superclass chain (`oo::class`), whose definition-body grammar governs
    /// every body the factory makes.
    pub root_metaclass: String,
    /// Per manufacturer subcommand (`create` / `new` / …) word layout, for
    /// the subcommands the factory **overrides**.  A subcommand with no entry
    /// runs the inherited manufacturer, i.e. the builtin `create Name Body`
    /// layout with nothing injected.
    pub overrides: BTreeMap<String, ManufacturerSpec>,
    /// Manufacturer methods reachable through ordinary class-command
    /// dispatch after applying the metaclass object's `self export` and
    /// `self unexport` changes to the registry defaults.
    pub exported_manufacturers: std::collections::BTreeSet<String>,
    /// Whether calling one of the classes this factory makes with a **bare
    /// unrecognised word** both constructs an object and returns that word —
    /// Tk's `::tk::IconList .il` idiom.
    ///
    /// Proved where the metaclass is written, from its unrecognised-word
    /// fallback member (`TclOO`'s `unknown`), and `false` whenever the proof
    /// does not go through: no such member, a member that constructs but
    /// returns something else, or one that returns the word without
    /// constructing anything. See
    /// `Analyser::unknown_dispatch_binds_instance` for the exact rule and its
    /// residuals.
    ///
    /// A consumer reads this to decide whether `set w [Widget .w]` binds `w`
    /// to an instance of `Widget`; `false` means "not proved", so the handle
    /// stays untyped rather than being guessed.
    pub unknown_binds_instance: bool,
}

impl ClassFactory {
    /// The same factory with every literal word's token replaced by
    /// `elsewhere` — what a **cross-document** consumer must use, because the
    /// stored tokens index the metaclass's own document, not the caller's.
    ///
    /// A member whose registry spec *retracts* (`deletemethod` /
    /// `renamemethod`) reads its argument tokens for real, so an injection
    /// containing one cannot be resolved against a foreign document at all:
    /// the whole injection collapses to `None` (inheritance unknown) rather
    /// than being applied against a substituted span.  `retracts` is supplied
    /// by the caller, which owns the registry lookup.
    #[must_use]
    pub fn resolve_in_other_document(
        &self,
        elsewhere: Token,
        retracts: &dyn Fn(&str) -> bool,
    ) -> Self {
        let overrides = self
            .overrides
            .iter()
            .map(|(sub, spec)| {
                let injected = spec.injected.as_ref().and_then(|members| {
                    members
                        .iter()
                        .map(|m| rehome_member(m, elsewhere, retracts))
                        .collect::<Option<Vec<_>>>()
                });
                (sub.clone(), ManufacturerSpec { injected, ..*spec })
            })
            .collect();
        Self {
            root_metaclass: self.root_metaclass.clone(),
            overrides,
            exported_manufacturers: self.exported_manufacturers.clone(),
            // A property of the metaclass's own body, carrying no token, so
            // it crosses a document boundary unchanged.
            unknown_binds_instance: self.unknown_binds_instance,
        }
    }
}

/// One injected member's words re-homed onto `elsewhere`, or `None` when the
/// member's own effect reads the tokens it would lose.
fn rehome_member(
    member: &FactoryMember,
    elsewhere: Token,
    retracts: &dyn Fn(&str) -> bool,
) -> Option<FactoryMember> {
    let keyword = match member.words.first() {
        Some(FactoryWord::Literal { text, .. }) => text.as_str(),
        // A member whose *keyword* is spliced from the caller is not a shape
        // the prologue reader produces, and cannot be classified here.
        _ => return None,
    };
    if retracts(keyword) {
        return None;
    }
    Some(FactoryMember {
        words: member
            .words
            .iter()
            .map(|w| match w {
                FactoryWord::Literal { text, .. } => FactoryWord::Literal {
                    text: text.clone(),
                    token: elsewhere,
                },
                splice @ FactoryWord::CallerSplice(_) => splice.clone(),
            })
            .collect(),
    })
}

/// Where a [`ClassDef::metaclass`] value came from.
///
/// An enum rather than a bare flag because the interesting half is what the
/// *default* means: `"oo::class"` sitting in a record no creation command
/// ever touched is a placeholder, and a join that reads it as evidence
/// invents a disagreement out of nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetaclassProvenance {
    /// Left at [`ClassDef::default()`]'s `"oo::class"` — no walk read it.
    ///
    /// The abstaining direction, and therefore the default: a record that
    /// cannot say where its metaclass came from constrains nothing in a
    /// join, while every consumer that simply reads `metaclass` keeps the
    /// stand-in it always had.
    #[default]
    StandIn,
    /// Read from a creation command's own head word — `oo::class create`
    /// and the other `TclOO` metaclasses, `snit::type` / `snit::widget`,
    /// `itcl::class`. The only spelling that is evidence.
    Observed,
}

/// Whether distinct original source slots have collided in one report key.
/// An unobserved collision is not proof of a unique or live native class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SourceNameAmbiguity {
    /// No collision was retained by the source publication join.
    #[default]
    Unobserved,
    /// A distinct original slot or policy was retained under the report key.
    Observed,
}

impl SourceNameAmbiguity {
    /// Reports only the retained collision observation.
    #[must_use]
    pub const fn is_observed(self) -> bool {
        matches!(self, Self::Observed)
    }

    /// Accumulate an observation without withdrawing an earlier collision.
    pub fn observe_if(&mut self, observed: bool) {
        if observed {
            *self = Self::Observed;
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
    /// Authoritative original counted member declarations and ordered effects.
    /// Reporting-name maps do not supply this ledger's identity or coverage.
    pub original_members: OriginalSourceMemberLedger,
    /// Canonical nameless lifecycle declarations, without dispatch or absence.
    pub original_special_members: OriginalSourceSpecialMemberLedger,
    /// Original property source names, kinds and accessor-body producers.
    /// This ledger supplies declaration advice, not native property admission.
    pub original_properties: OriginalSourcePropertyLedger,
    /// Original relation operands and own receiver effects, without report keys.
    pub original_relations: OriginalSourceClassRelationLedger,
    /// Retained original object publication geometry; no runtime class token.
    pub source_name: Option<crate::signature_scan::scope::SignatureSourceCommand>,
    /// Distinct component slots share this record's rendered map key.
    pub source_name_ambiguous: SourceNameAmbiguity,
    /// Original metaclass-head lookup; supplies no reached provider identity.
    pub metaclass_lookup: Option<crate::signature_scan::scope::SignatureSourceLookup>,
    /// Original caller contexts of relation words; ambiguity remains unavailable.
    pub relation_lookups:
        HashMap<String, Option<crate::signature_scan::scope::SignatureSourceLookup>>,
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
    ///
    /// The default is a *stand-in*, not a reading of the source: a record
    /// built by anything other than a creation command — an
    /// ``oo::define`` stub, the ``oo::objdefine`` holder — carries it
    /// having observed nothing. [`Self::metaclass_provenance`] is what
    /// tells the two apart.
    pub metaclass: String,
    /// Where [`Self::metaclass`] came from — the observed-fields mask this
    /// record needs, and the only entry it needs.
    ///
    /// Every other field defaults to a value that *is* its own "nothing
    /// observed" reading — empty tables, empty superclass list, `None`,
    /// `false`, a zero span — so a join can treat the default as a lower
    /// bound and take the other side's. `metaclass` alone defaults to a
    /// fabricated concrete name, which a join comparing it as evidence
    /// reads as a walk having *proved* `oo::class`; a `via_define` stub for
    /// a computed-name class made by `::T::Mother` then looked like two
    /// walks disagreeing about the metaclass, and the record abstained down
    /// to no factory and no superclasses.
    pub metaclass_provenance: MetaclassProvenance,
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
    /// **Instance-side** method filters declared via an unwrapped (or
    /// `private`-wrapped) ``filter`` — the class's `info class filters` slot.
    /// They intercept dispatches on *instances* of the class.
    ///
    /// Sided because `TclOO` keeps two independent filter slots, exactly as it
    /// keeps two method tables. Oracle, byte-identical on tclsh
    /// 9.0.4 and 8.6.14:
    ///
    /// ```tcl
    /// oo::class create ::A { method real {} {…} ; method logit {args} {…} ; filter logit }
    /// info class filters ::A    ;# -> logit
    /// info object filters ::A   ;# -> (empty)      the class object is unfiltered
    /// [::A new] real            ;# -> logit fires, `self target` is `::A real`
    /// oo::class create ::P { private { method s {} {} ; filter s } }
    /// info class filters ::P    ;# -> s            `private` is instance-side too
    /// ```
    pub filters: Vec<String>,
    /// **Class-object-side** method filters declared via ``self filter`` — the
    /// class object's own `info object filters` slot.
    ///
    /// A class-side filter intercepts dispatches on the *class command itself*,
    /// including the constructor path, and never touches instance dispatch.
    /// Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
    ///
    /// ```tcl
    /// oo::class create ::B { method inst {} {…}
    ///                        self { method cls {} {…} ; method logit {args} {…} ; filter logit } }
    /// info object filters ::B   ;# -> logit
    /// info class filters ::B    ;# -> (empty)      instances are unfiltered
    /// ::B cls                   ;# -> logit fires, `self target` is `::B cls`
    /// ::B new                   ;# -> logit fires, `self target` is `::oo::class new`
    /// [::B new] inst            ;# -> logit does NOT fire
    /// ```
    pub class_filters: Vec<String>,
    /// Methods explicitly exported via an **instance-side** ``export``.
    pub exports: HashSet<String>,
    /// Methods explicitly unexported via an **instance-side** ``unexport``.
    ///
    /// Kept **mutually exclusive** with [`Self::exports`] — each set records the
    /// last explicit writer for a name, and the cross-file consumer
    /// (`workspace_index::method_dispatch_chain`) reads any `exports` entry as
    /// decisive, so a name left in both would advertise a method the interpreter
    /// rejects.
    pub unexports: HashSet<String>,
    /// Methods explicitly exported via ``self export`` — the **class-object**
    /// side's counterpart of [`Self::exports`], maintained under the identical
    /// last-writer-exclusive rule ([`MemberSide::visibility_sets`]).
    ///
    /// A separate pair rather than a side tag on the existing one because
    /// `exports`/`unexports` are the instance-side record *by contract*: the
    /// workspace effective-export union and `rename_safety` both read them that
    /// way, so a class-side flip landing there would silently re-state an
    /// unrelated instance method's export bit. Without this pair a
    /// `self unexport m` in one file never reached another file's class-command
    /// dispatch, which went on advertising a member `::C m` rejects with
    /// `unknown method "m"`.
    ///
    /// Rides the same cross-file channel as [`Self::retracted_members`] — a
    /// `via_define` stub carries it — and shares that channel's documented
    /// unordered caveat: cross-file load order is not knowable from the index,
    /// so the workspace takes the union rather than a last writer.
    pub class_exports: HashSet<String>,
    /// Methods explicitly unexported via ``self unexport`` — see
    /// [`Self::class_exports`].
    pub class_unexports: HashSet<String>,
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
    /// extension deletes it. Cross-file *order* is
    /// unprovable, so the tombstone is unordered — exactly as a cross-file
    /// `oo::define ::C { method extra … }` is an unordered addition today.
    pub retracted_members: Vec<MemberRetractionRecord>,
    /// Reasons this record's definition body **cannot run at all** — a
    /// retraction of a member absent from its side's table, or a
    /// `renamemethod` onto a name already taken.
    ///
    /// **Transient.** Recorded by the member walker, which is the only place
    /// that knows a side's table state at the point the offending word runs,
    /// and *drained* by the class handlers the moment the body walk returns —
    /// so the record that reaches `all_classes` (and the workspace index)
    /// always has this empty. It is not part of the class's published shape;
    /// it is the walker's channel to the diagnostic emitter.
    ///
    /// The partial class is still recorded, deliberately: a body that cannot
    /// run has no outline at all in real Tcl, but degrading navigation to
    /// nothing is worse than describing what the author clearly meant —
    /// the same judgement parse errors already get.
    pub definition_aborts: Vec<DefinitionAbort>,
    /// Every `renamemethod` in this record's body that **moved** a member, with
    /// the member state the move ran against.
    ///
    /// The moved member's declaration site is the `renamemethod`'s destination
    /// word, so a rename of it rewrites that word — and some new names turn the
    /// body into one real Tcl refuses to run. Unlike
    /// [`Self::definition_aborts`] this is **not** transient: the rename gate
    /// re-analyses the document and reads it from the recorded class, so it has
    /// to survive into `all_classes`.
    pub renamed_members: Vec<RenamedMember>,
    /// Names genuinely bareword-callable from any method body of this
    /// class, because ``link`` (``oo::Helpers::link``) installed a
    /// per-object-namespace alias — keyed by the alias name, valued by
    /// the real member name it dispatches to via ``my TARGET`` (alias ==
    /// target for the plain ``link NAME`` form). In contrast, a bareword
    /// matching an un-linked sibling method/classmethod/property name is
    /// **not** reachable that way and errors "invalid command name" at
    /// runtime.
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
    /// a method it inherits is not one it is missing.
    pub inheritance_unknown: bool,
    /// `true` when calling **this class's own command** with a bare,
    /// unrecognised first word constructs an instance and yields its name —
    /// Tk's `::tk::IconList .il`.
    ///
    /// Proved where the class is written, from its metaclass's
    /// unrecognised-word fallback member
    /// ([`ClassFactory::unknown_binds_instance`]), so a consumer in another
    /// document reads a settled fact instead of needing the metaclass's own
    /// body. `false` is "not proved", never "proved not to" — the abstaining
    /// direction, which leaves a handle untyped rather than mistyped.
    ///
    /// Distinct from the *family-wide*
    /// [`DefinitionBodyGrammar::bare_word_construction`](tcl_registry::definer::DefinitionBodyGrammar::bare_word_construction)
    /// flag snit carries: that one is true of every snit type by definition,
    /// this one is true only of the classes a particular metaclass makes.
    pub class_command_fallback: ClassCommandFallback,
    /// `true` when the class's definition body installs members the analyser
    /// could not read, so the recorded member tables are a **lower bound**
    /// on what the class really has.
    ///
    /// Two shapes set it, both registry-driven rather than keyword-matched:
    ///
    /// * a recognised member word whose declaration is supplied through an
    ///   unresolvable `{*}` expansion or a computed name —
    ///   `constructor {*}[info class constructor ::Base]`,
    ///   `method $m {*}[info class definition ::Base $m]` (the ticklecharts
    ///   `chart3D` reflection idiom). Tcl expands these at definition time,
    ///   so the members are entirely real; only their spelling is opaque.
    /// * a body command that is **not** a member word and either has no
    ///   registry spec at all (a helper proc that installs members) or takes
    ///   a script argument the member walk does not descend into
    ///   (`foreach m {…} { method $m … }`, `if {…} { method … }`).
    ///
    /// The class itself, and every member that *was* readable, stay fully
    /// modelled — this only says "there may be more". Method-existence
    /// checks (W308) must therefore abstain on such a class, exactly as they
    /// do for [`Self::inheritance_unknown`]: a method installed by
    /// reflection is not a method that is missing. tclsh 9.0.4 and 8.6.16
    /// agree that `oo::class create C3 { constructor {*}[info class
    /// constructor ::C] ; foreach m {options} { method $m {*}[info class
    /// definition ::C $m] } }` yields `info class methods ::C3` → `options`,
    /// and `[C3 new] options` really runs.
    pub member_set_incomplete: bool,
    /// Present when this class is itself a **class factory** — a user-defined
    /// `TclOO` metaclass whose superclass chain reaches `oo::class`.
    ///
    /// It describes how the factory manufactures classes (which creation
    /// argument is the name, which is the body, what the manufacturer splices
    /// into every body), derived once here rather than re-derived at each
    /// `Meta create …` call.  Publishing it on the `ClassDef` is what lets a
    /// *different file*'s walk classify `::tk::Megawidget create IconList …`
    /// at all: without it, that call is indistinguishable from
    /// `interp create` or `image create`.
    pub factory: Option<ClassFactory>,
}

/// Concatenate an **appended** ordered slot (`variable`, `filter`) from a
/// record whose words ran `from_ran_second` relative to `into`'s.
///
/// These slots accumulate rather than replace: on 8.6.16 and 9.0.4,
/// `oo::class create C { filter f; variable x }` then
/// `oo::define C { filter g; variable y }` reports filters `f g` and
/// variables `x y`. A name the slot already carries is not repeated —
/// declaring `variable x` twice leaves one `x` — and the surviving order
/// is run order, which is why the earlier record's entries lead.
///
/// See [`ClassDef::absorb_declarations`] for the slots that replace instead.
fn append_ordered_slot(into: &mut Vec<String>, from: &[String], from_ran_second: bool) {
    let (mut merged, tail) = if from_ran_second {
        (std::mem::take(into), from.to_vec())
    } else {
        (from.to_vec(), std::mem::take(into))
    };
    for name in tail {
        if !merged.contains(&name) {
            merged.push(name);
        }
    }
    *into = merged;
}

/// Add `key`'s declaration to `table`, replacing an existing entry only
/// when the contributing record is the one that ran second — see
/// [`ClassDef::absorb_declarations`].
fn insert_declaration<V: Clone>(
    table: &mut HashMap<String, V>,
    key: &str,
    value: &V,
    replace: bool,
) {
    match table.entry(key.to_owned()) {
        std::collections::hash_map::Entry::Occupied(mut slot) => {
            if replace {
                slot.insert(value.clone());
            }
        }
        std::collections::hash_map::Entry::Vacant(slot) => {
            slot.insert(value.clone());
        }
    }
}

impl ClassDef {
    /// Take every declaration `other` makes that this record does not
    /// already carry.
    ///
    /// Two records can describe one class: the computed-name creation
    /// observed inside a procedure body, and whatever the ordinary walk
    /// already built under the proved name — in particular an
    /// [`oo::define` stub](Self::via_define), whose members are a real
    /// additional contribution to the same class rather than a second
    /// reading of the same body. A join that simply picked the more
    /// complete record therefore silently dropped the other's members,
    /// which is how a stub's `method m` disappeared the moment the join
    /// stopped abstaining.
    ///
    /// **This join knows the order.** An `oo::define` extending a class
    /// necessarily runs after the creation it extends, so
    /// [`via_define`](Self::via_define) tells the two records apart *and*
    /// tells us which ran second. Every rule below is that ordering applied
    /// to one kind of field, and each kind takes it differently — so the
    /// rules are per-slot facts read off a real interpreter, not one
    /// pattern assumed to generalise. Measured identically on tclsh 8.6.16
    /// and 9.0.4, `oo::class create C {…}` followed by `oo::define C {…}`:
    ///
    /// | field | second definition supplies | result |
    /// |---|---|---|
    /// | `method m {a}` → `method m {a b}` | a new body | `{a b} …` — replace |
    /// | `export m` → `unexport m` | the opposite bit | unexported; `m` leaves the public table |
    /// | `mixin ::A` → `mixin ::B` | a new list | `::B` — replace (`mixin -append` is the additive form) |
    /// | `superclass ::S0` → `superclass ::S9` | a new list | `::S9` — replace |
    /// | `filter f` → `filter g` | another filter | `f g` — **append** |
    /// | `variable x` → `variable y` | another name | `x y` — **append**, de-duplicated |
    /// | `method m` → `deletemethod m` | a retraction | `m` is gone; dispatch raises `unknown method "m"` |
    ///
    /// So:
    ///
    /// * **Keyed tables** (`methods`, `class_methods`, `properties`,
    ///   `linked_members`) union, and a key both records declare goes to
    ///   whichever ran second. Where neither side is a stub the choice
    ///   cannot matter: a second creation under a name already taken never
    ///   runs (`can't create object "::D::class": command already exists
    ///   with that name`, same tclsh), so two non-stub observations are two
    ///   readings of one body and a colliding key holds the same
    ///   declaration either way. This record then keeps its own, being the
    ///   observation the caller judged more complete.
    /// * **Visibility** (`exports`/`unexports` and the class-side pair) is
    ///   one bit per member, not two independent sets, so a union can leave
    ///   a member in both — and a workspace consumer reads an `exports`
    ///   entry as decisive, which would advertise a method the later
    ///   definition took away. The later record's bit wins and clears its
    ///   opposite, the same last-writer-exclusive update
    ///   `oo::apply_visibility_member` performs while walking one body.
    /// * **Replaced ordered slots** (`mixins`) take the later record's list
    ///   whole when it has one, and the earlier record's only when the
    ///   later never set the slot.
    /// * **Appended ordered slots** (`variables`, `filters`,
    ///   `class_filters`) concatenate in run order, dropping a name the
    ///   list already carries — `variable x` twice leaves one `x`.
    /// * **Whole-body singletons** (`constructors`, `destructor`) follow the
    ///   replace rule, taking the earlier record's only when the later
    ///   declares none.
    /// * **Retractions** are *applied*, not merely carried. A `deletemethod`
    ///   in the later record removes the member from the joined table and a
    ///   `renamemethod` moves it, because this join is the point where the
    ///   ordering is known. The tombstones ride along as well, for the
    ///   cross-file consumer that still needs them, but compiler-side
    ///   consumers of `all_classes` never apply the workspace retraction
    ///   fold and would otherwise treat a deleted member as live.
    ///
    /// Two observations of one identical body — the case this join was
    /// written for, where neither record is a stub — remain a no-op under
    /// every rule above.
    ///
    /// Identity, spans, provenance flags and `definition_aborts` are not
    /// declarations and are left to the caller;
    /// [`definition_aborts`](Self::definition_aborts) in particular is
    /// transient and already consumed by W315 before either record reaches
    /// a join.
    pub(crate) fn absorb_declarations(&mut self, other: &Self) {
        // Destructured exhaustively on purpose: a field added to
        // `ClassDef` fails to compile here until it has been classified,
        // rather than being quietly dropped by every join.
        let Self {
            original_members: _,
            original_special_members: _,
            original_properties: _,
            original_relations: _,
            source_name: _,
            source_name_ambiguous: _,
            metaclass_lookup: _,
            relation_lookups: _,
            name: _,
            qualified_name: _,
            name_span: _,
            body_span: _,
            metaclass: _,
            metaclass_provenance: _,
            superclasses: _,
            mixins,
            methods,
            class_methods,
            constructors,
            destructor,
            variables,
            properties,
            filters,
            class_filters,
            exports,
            unexports,
            class_exports,
            class_unexports,
            retracted_members,
            definition_aborts: _,
            renamed_members,
            linked_members,
            doc,
            via_define,
            inheritance_unknown: _,
            class_command_fallback: _,
            member_set_incomplete: _,
            factory: _,
        } = other;

        // `other` is the `oo::define` that extends what this record created,
        // so it ran second and its declarations replace rather than add.
        // When *this* record is the stub the ordering is simply reversed:
        // `other` ran first and only fills what this one never mentions.
        let other_ran_second = *via_define && !self.via_define;
        self.absorb_original_declarations(other, other_ran_second);
        for (key, def) in methods {
            insert_declaration(&mut self.methods, key, def, other_ran_second);
        }
        for (key, def) in class_methods {
            insert_declaration(&mut self.class_methods, key, def, other_ran_second);
        }
        for (key, def) in properties {
            insert_declaration(&mut self.properties, key, def, other_ran_second);
        }
        for (key, target) in linked_members {
            insert_declaration(&mut self.linked_members, key, target, other_ran_second);
        }

        for side in [MemberSide::Instance, MemberSide::ClassObject] {
            let (other_exports, other_unexports) = match side {
                MemberSide::Instance => (exports, unexports),
                MemberSide::ClassObject => (class_exports, class_unexports),
            };
            self.absorb_visibility(side, other_exports, other_unexports, other_ran_second);
        }

        // `mixin` (and the whole-body singletons) replace the slot;
        // `variable` / `filter` append to it. Both confirmed on 8.6.16 and
        // 9.0.4 — see this function's table.
        if other_ran_second && !mixins.is_empty() || self.mixins.is_empty() {
            self.mixins.clone_from(mixins);
        }
        for (into, from) in [
            (&mut self.variables, variables),
            (&mut self.filters, filters),
            (&mut self.class_filters, class_filters),
        ] {
            append_ordered_slot(into, from, other_ran_second);
        }
        if other_ran_second && !constructors.is_empty() || self.constructors.is_empty() {
            self.constructors.clone_from(constructors);
        }
        if other_ran_second && destructor.is_some() || self.destructor.is_none() {
            self.destructor.clone_from(destructor);
        }

        for record in retracted_members {
            if !self.retracted_members.contains(record) {
                self.retracted_members.push(record.clone());
            }
        }
        for record in renamed_members {
            if !self.renamed_members.contains(record) {
                self.renamed_members.push(record.clone());
            }
        }
        if other_ran_second {
            self.apply_retractions(retracted_members);
        }
        if self.doc.is_empty() {
            self.doc.clone_from(doc);
        }
    }

    fn absorb_original_declarations(&mut self, other: &Self, other_ran_second: bool) {
        self.original_members
            .absorb(&other.original_members, other_ran_second);
        self.original_special_members
            .absorb(&other.original_special_members, other_ran_second);
        self.original_relations
            .absorb(&other.original_relations, other_ran_second);
        self.original_properties
            .absorb(&other.original_properties, other_ran_second);
        if self.source_name.is_none() {
            self.source_name.clone_from(&other.source_name);
        }
        self.source_name_ambiguous
            .observe_if(other.source_name_ambiguous.is_observed());
        if self.metaclass_lookup.is_none() {
            self.metaclass_lookup.clone_from(&other.metaclass_lookup);
        }
        for (written, lookup) in &other.relation_lookups {
            match self.relation_lookups.entry(written.clone()) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(lookup.clone());
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    if entry.get() != lookup {
                        entry.insert(None);
                    }
                }
            }
        }
    }

    /// Merge one side's `export` / `unexport` pair from a record whose
    /// words ran `other_ran_second` relative to this one's.
    ///
    /// `TclOO` keeps one visibility *bit* per member, not two independent
    /// sets: `export m` sets it and `unexport m` clears it, so the later
    /// word wins outright and the earlier one leaves no trace. Confirmed on
    /// 8.6.16 and 9.0.4 — after `oo::class create E1 { method m {} {};
    /// export m }` then `oo::define E1 { unexport m }`,
    /// `info class methods E1` is empty while `-private` still lists `m`,
    /// and `[E1 new] m` raises `unknown method "m"`; the reverse order
    /// re-exports it.
    ///
    /// The same last-writer-exclusive update
    /// [`oo::apply_visibility_member`](crate::analyser::oo) performs while
    /// walking a single body, applied across the two records this join
    /// holds. A plain union would leave a member in both sets, and the
    /// workspace reads an `exports` entry as decisive — so it would go on
    /// advertising a method the later definition took away.
    fn absorb_visibility(
        &mut self,
        side: MemberSide,
        other_exports: &HashSet<String>,
        other_unexports: &HashSet<String>,
        other_ran_second: bool,
    ) {
        for (from, exported) in [(other_exports, true), (other_unexports, false)] {
            for name in from {
                let (exports, unexports) = side.visibility_sets(self);
                let (mine, opposite) = if exported {
                    (exports, unexports)
                } else {
                    (unexports, exports)
                };
                // Skip only a name this record already decided *and*
                // decided later; otherwise `other`'s bit is the live one.
                if !other_ran_second && (mine.contains(name) || opposite.contains(name)) {
                    continue;
                }
                mine.insert(name.clone());
                opposite.remove(name);
            }
        }
    }

    /// Apply retractions recorded by a definition that ran **after** this
    /// record's members were declared.
    ///
    /// A `via_define` stub for a class created elsewhere finds nothing to
    /// remove while it is walked, so `deletemethod m` / `renamemethod m n`
    /// can only be written down as a tombstone. Across files that is all
    /// anyone can say — load order is unknowable, which is why
    /// [`Self::retracted_members`] is documented as unordered and the
    /// workspace applies it as a fold. Here the order *is* known, and the
    /// tombstone alone is not enough: compiler-side consumers of
    /// `AnalysisResult::all_classes` never run that fold, so a deleted
    /// member would stay dispatchable, keep answering method-existence
    /// checks, and feed derived factory facts.
    ///
    /// tclsh 8.6.16 and 9.0.4 agree on both shapes: after
    /// `oo::class create D1 { method m …; method keep … }` then
    /// `oo::define D1 { deletemethod m }`, `info class methods D1` is
    /// `keep` and `[D1 new] m` raises `unknown method "m"`; after
    /// `renamemethod m n` the table holds `n` alone.
    fn apply_retractions(&mut self, retractions: &[MemberRetractionRecord]) {
        for record in retractions {
            let table = record.side.table(self);
            let Some(mut moved) = table.remove(&record.member) else {
                continue;
            };
            // A move re-homes the member under the arrival name, whose
            // declaration site is the `renamemethod` destination word — the
            // token go-to-definition must answer with, and the only one a
            // rename of the arrived member may rewrite.
            let (Some(arrival), Some(span)) = (&record.arrival, record.arrival_span) else {
                continue;
            };
            arrival.clone_into(&mut moved.name);
            moved.name_span = span;
            table.insert(arrival.clone(), moved);
        }
    }
}

impl Default for ClassDef {
    /// Default-construct a [`ClassDef`].
    ///
    /// All names default to empty strings, both spans default to
    /// ``Span::new(0, 0)``, and the metaclass defaults to
    /// ``"oo::class"`` — with
    /// [`metaclass_provenance`](ClassDef::metaclass_provenance)
    /// [`StandIn`](MetaclassProvenance::StandIn), so
    /// that stand-in is never mistaken for a reading of the source.
    /// Used by handlers that build a class record incrementally
    /// (most common shape — only `name` / `qualified_name` /
    /// `name_span` / `body_span` need explicit values).
    fn default() -> Self {
        let zero = Span::new(0, 0);
        Self {
            original_members: OriginalSourceMemberLedger::default(),
            original_special_members: OriginalSourceSpecialMemberLedger::default(),
            original_properties: OriginalSourcePropertyLedger::default(),
            original_relations: OriginalSourceClassRelationLedger::default(),
            source_name: None,
            source_name_ambiguous: SourceNameAmbiguity::Unobserved,
            metaclass_lookup: None,
            relation_lookups: HashMap::new(),
            name: String::new(),
            qualified_name: String::new(),
            name_span: zero,
            body_span: zero,
            metaclass: "oo::class".to_string(),
            metaclass_provenance: MetaclassProvenance::StandIn,
            superclasses: Vec::new(),
            mixins: Vec::new(),
            methods: HashMap::new(),
            class_methods: HashMap::new(),
            constructors: Vec::new(),
            destructor: None,
            variables: Vec::new(),
            properties: HashMap::new(),
            filters: Vec::new(),
            class_filters: Vec::new(),
            exports: HashSet::new(),
            unexports: HashSet::new(),
            class_exports: HashSet::new(),
            class_unexports: HashSet::new(),
            retracted_members: Vec::new(),
            definition_aborts: Vec::new(),
            renamed_members: Vec::new(),
            linked_members: HashMap::new(),
            doc: String::new(),
            via_define: false,
            inheritance_unknown: false,
            class_command_fallback: ClassCommandFallback::None,
            member_set_incomplete: false,
            factory: None,
        }
    }
}

/// Statically proved behaviour of an unrecognised word dispatched through a
/// class command. An enum keeps the proof state explicit and leaves room for
/// future fallback effects without adding another boolean to [`ClassDef`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ClassCommandFallback {
    /// No construction-and-return proof is available.
    #[default]
    None,
    /// The fallback constructs an instance named by the word and returns it.
    ConstructsNamedInstance,
}

impl ClassCommandFallback {
    /// Whether the fallback provides the named-instance contract.
    #[must_use]
    pub fn constructs_named_instance(self) -> bool {
        self == Self::ConstructsNamedInstance
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
    /// Genuine readonly class/member/body declaration context. No runtime
    /// receiver, activation, dispatch or Normal authority follows from it.
    pub original_member_context: Option<Box<OriginalLexicalMemberContext>>,
    /// Independent original method body declaration. Its source ownership does
    /// not establish a native factory, receiver frame or executing namespace.
    pub original_receiver_body_declaration:
        Option<std::sync::Arc<OriginalSourceReceiverBodyDeclaration>>,
    /// Original child-interpreter reporting domain. Its source namespace
    /// geometry remains separate from parent-file reporting identity.
    pub original_interpreter_source_domain:
        Option<std::sync::Arc<super::OriginalInterpreterSourceDomain>>,
    /// Retained authored namespace geometry; supplies no actual activation or token.
    pub naming_scope: Option<crate::signature_scan::scope::SignatureNamespaceScope>,
    /// Scope kind (global, namespace, proc).
    pub kind: ScopeKind,
    /// Scope identifier — namespace name for namespace/global
    /// scopes, proc qualified name for proc scopes.
    pub name: String,
    /// Body span (braces excluded), `None` for the global scope.
    pub body_span: Option<Span>,
    /// Span of the word that *names* this scope, when the scope was opened by
    /// a command that spells its name out — the `NAME` word of `namespace
    /// eval NAME { … }`.
    ///
    /// `None` for scopes with no written name word: the global scope, an
    /// `interp eval` domain, a synthesised source-seed frame, and proc /
    /// method scopes (whose name span already travels on the richer
    /// [`ProcDef::name_span`] / [`MethodDef`] records).
    ///
    /// Consumers that must point *at the name* rather than at the body read
    /// this: the outline's `selectionRange` ("the range that should be
    /// selected and revealed when this symbol is picked") is the name token
    /// for every other symbol kind, and a namespace that answered its whole
    /// body there selected the entire block when clicked.
    pub name_span: Option<Span>,
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
    /// them is a live defect. A Tcl 9 class
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
    /// The fully-qualified class defining this **instance-side `TclOO`
    /// method** scope's implementation — the value `[self class]` answers in
    /// this frame — or `None` everywhere else.
    ///
    /// Deliberately `None` for the class-side forms funnelled into the same
    /// [`ScopeKind::Method`]: in a `self method` body `self class` *raises*
    /// "method not defined by a class", and in a `classmethod` body it
    /// answers the internal delegate class (`::oo::ObjN:: oo ::delegate`),
    /// never the written class — both pinned on tclsh 9.0.4, so neither has
    /// a statically-foldable value. Also `None` for snit / itcl members
    /// (not `TclOO`; `self` has different meaning),
    /// for class-level `initialise` scripts, and for `oo::objdefine`'s
    /// synthetic per-object records. Consumed by the analyser's constant
    /// command-substitution fold so `set ns [namespace
    /// qualifiers [self class]]` folds only where real Tcl produces that
    /// value.
    pub oo_defining_class: Option<String>,
}

impl Scope {
    /// Construct a fresh empty scope.
    #[must_use]
    pub fn new(kind: ScopeKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            original_interpreter_source_domain: None,
            naming_scope: None,
            original_member_context: None,
            original_receiver_body_declaration: None,
            name: name.into(),
            body_span: None,
            name_span: None,
            variables: HashMap::new(),
            procs: HashMap::new(),
            classes: HashMap::new(),
            defined_symbols: Vec::new(),
            children: Vec::new(),
            oo_global_resolution: false,
            oo_method_frame: false,
            oo_defining_class: None,
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
/// in the shape so a consumer can serialise the complete result.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnalysisResult {
    pub(super) command_realm: Option<std::sync::Arc<crate::realm::CommandBindingRealm>>,
    pub(super) original_command_world:
        Option<crate::command_binding::OriginalCompletedCommandWorld>,
    pub(super) lexical_declaration_advice: bool,
    /// Actual editing inputs retained independently of the dialect label.
    /// This is not a native execution or implementation-identity proof.
    pub resolved_input: Option<super::ResolvedAnalysisInput>,
    pub(super) original_vendor_source_names: HashMap<
        tcl_lexer::Span,
        Option<crate::signature_scan::vendor_name::VendorSourceNameOccurrence>,
    >,
    pub(super) original_vendor_variable_advice:
        Vec<crate::signature_scan::vendor_variable::VendorSourceVariableAdvice>,
    pub(super) original_vendor_variable_bodies:
        Vec<crate::signature_scan::vendor_variable::VendorSourceVariableBody>,
    /// Root scope tree (`::`).
    pub global_scope: Scope,
    /// Procs keyed by qualified name.
    pub all_procs: HashMap<String, ProcDef>,
    /// Authentic original declaration records, including byte-distinct names
    /// whose Unicode presentation collides. UI maps are projections only.
    pub original_procedure_metadata:
        Vec<crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>>,
    /// Independently selected hosted procedure source cards, without native slots.
    pub(super) original_vendor_procedure_metadata:
        Vec<crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata<ProcDef>>,
    /// Independently selected hosted class source cards, without native object grants.
    pub(super) original_vendor_class_metadata:
        Vec<crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata<ClassDef>>,
    /// Immutable original Registry-defined source cards, independent of UI maps.
    pub(super) original_symbol_metadata:
        Vec<crate::signature_scan::symbol_name::OriginalSourceSymbolDeclaration>,
    /// Independently selected hosted Registry symbol source cards.
    pub(super) original_vendor_symbol_metadata:
        Vec<crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata<DefinedSymbol>>,
    /// Every `proc` declaration's own name-token span, in source order —
    /// unlike `all_procs` (a map that, for a qualified name declared more
    /// than once in the same document, retains only the *last* processed
    /// declaration — plain Tcl's own "last redefinition wins" semantics),
    /// this keeps one entry per declaration, including an earlier, shadowed
    /// one. Exists purely so a query landing on a shadowed declaration's own
    /// name token can still be recognised as declaring that qualified name
    /// and re-resolved to whichever `ProcDef` currently wins in `all_procs`
    /// — `all_procs`' own span alone can never satisfy that lookup, since it
    /// only ever holds the winner's.
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
    /// [`Self::proc_def_in_effect_at`] is how consumers ask it.
    ///
    /// Distinct from [`Self::proc_declaration_sites`], which records only
    /// *spans* (enough to recognise a shadowed declaration's own name token)
    /// and so cannot answer what the displaced definition's parameters were.
    pub superseded_procs: HashMap<String, Vec<ProcDef>>,
    /// Classes keyed by qualified name.
    pub all_classes: HashMap<String, ClassDef>,
    /// Authentic original class declarations, separate from UI `QName` maps.
    pub original_class_metadata:
        Vec<crate::signature_scan::original_name::SourceDeclarationMetadata<ClassDef>>,
    /// Independently targeted original configuration deltas, in analyser walk order.
    pub original_class_configuration_metadata: Vec<OriginalSourceClassConfiguration>,
    /// Own-object configuration deltas keyed by their actual bounded allocations.
    /// This source inventory never selects a later dispatch or an offset epoch.
    pub original_object_configuration_metadata: Vec<OriginalSourceObjectConfiguration>,
    /// Original class declarations displaced by a later declaration of the
    /// same name. Positioned navigation uses these records; the final class
    /// inventory alone cannot identify an earlier selected implementation.
    pub superseded_classes: HashMap<String, Vec<ClassDef>>,
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
    /// just `ClassDef::body_span`.
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
    /// ``package ifneeded NAME VERSION ?SCRIPT?`` records — the load
    /// scripts this document registers, in source order.  Only the *name*
    /// end matters to consumers: an
    /// `ifneeded` body is an arbitrary script evaluated later in the
    /// global namespace, so its presence marks the package's loading
    /// as not statically known.
    pub package_ifneededs: Vec<PackageIfneeded>,
    /// ``package prefer latest`` records — the interpreter-global
    /// version-selection mode raises, in source order.  See
    /// [`SignaturePackagePrefer`] for why only the
    /// raise to `latest` is a record.
    pub package_prefer_latest: Vec<SignaturePackagePrefer>,
    /// True when a non-literal ``package require`` / ``load`` /
    /// ``auto_path`` mutation has been seen — downstream W123
    /// emission suppresses unknown-command diagnostics under this
    /// flag because the dynamic provider may register the missing
    /// command at runtime.
    pub has_dynamic_providers: bool,
    /// Source-target records.
    pub source_targets: Vec<SignatureSource>,
    /// Every path-constant fact the document's load-time surface records —
    /// top-level `set`s, and `variable`/`set` writes inside literal
    /// `namespace eval` bodies — in document order, values **unfolded**
    /// ([`crate::auto_path_eval::constant_path_assignments`]).
    ///
    /// Recorded raw rather than folded because folding the
    /// `[file dirname [info script]]` idiom needs the document's own
    /// filesystem path, which the analyser deliberately does not know — the
    /// same text must analyse identically wherever the file lives.  Consumers
    /// that know the path (the source-graph edge resolver, `auto_path`
    /// folding) chain-fold these with
    /// [`crate::auto_path_eval::fold_constant_assignments`], which also owns
    /// the multi-write poisoning rule.
    pub path_constant_assignments: crate::auto_path_eval::PathConstantAssignments,
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
    /// {}` too.
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
    /// the same way they already resolve an alias name
    /// to its target. Only ever populated from a *literal* `-map`/
    /// `-subcommands` list — a dynamic value (`-map $var`) leaves the
    /// ensemble's entry absent entirely, so a lookup against it correctly
    /// abstains rather than guessing.
    ///
    /// Each entry carries the **provenance** of its mapping
    /// ([`EnsembleSubcommandTarget::provenance`]) — `-map` binds
    /// an arbitrary key to a target, `-subcommands` derives the target from
    /// the name — because a consumer that rewrites the subcommand word
    /// (rename) is only correct for one of the two.
    pub ensemble_subcommand_targets: HashMap<String, HashMap<String, EnsembleSubcommandTarget>>,
    /// Resolved command names of ensembles this file configured with
    /// `namespace ensemble … -prefixes 0`, which turns off the
    /// `Tcl_GetIndexFromObj` prefix matching every other ensemble has.
    ///
    /// Consulted by the abbreviation machinery (W145 and the formatter's
    /// expansion) so an abbreviation on such an ensemble is never reported
    /// ambiguous or rewritten, and by
    /// [`Self::resolve_ensemble_subcommand`] so navigation does not follow
    /// an abbreviation the ensemble would refuse.
    pub prefixless_ensembles: std::collections::HashSet<String>,
    /// Namespace import records.
    pub namespace_imports: Vec<SignatureNamespaceImport>,
    /// Namespace `forget` records — the removal half of the import edge's
    /// ordered lifecycle log. See
    /// [`SignatureNamespaceForget`] for the oracle: an import installs an
    /// alias, `namespace forget` takes it away again, and a bare call after
    /// the forget is `invalid command name`. Consumed together with
    /// [`Self::namespace_imports`] by
    /// `tcl_lsp_core::namespace_import::alias_live_at`, under the same
    /// order gate ([`super::indirection::in_effect`]) the export snapshot
    /// and the `rename` / `interp alias` timeline already use.
    pub namespace_forgets: Vec<SignatureNamespaceForget>,
    /// Authentic original import/forget operands, including opaque byte units.
    /// Display inventories are independent projections and cannot recover these.
    pub(super) original_namespace_patterns:
        Vec<crate::signature_scan::original_name::SourceNamespacePattern>,
    /// Byte offset of the statement that **destroyed** each qualified command
    /// — `rename OLD {}` and `interp alias {} NAME {}`, the two forms that
    /// delete the command *object* rather than move it.
    ///
    /// A destruction is a lifecycle event on every `namespace import` edge
    /// pointing at the command: the alias holds the object, so destroying it
    /// kills the alias too (oracle tclsh 8.6.14 / 9.0.4 — with `::dst::p`
    /// imported from `::src::p`, `rename ::src::p {}` makes `::dst::p` an
    /// `invalid command name` and empties `info commands ::dst::*`).
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
    /// they exist (gating wildcard-import bareword resolution).
    pub namespace_exports: Vec<SignatureNamespaceExport>,
    /// Original counted export patterns and clear operands with caller geometry.
    pub(super) original_namespace_exports:
        Vec<crate::signature_scan::original_name::SourceNamespaceExport>,
    /// Unrepresented export operands are May obligations, not evaluated names.
    pub(super) original_namespace_export_unknowns: Vec<Span>,
    /// Recorded `namespace path {…}` declarations, keyed by the declaring
    /// namespace's fully-qualified name (`::` for global).  Each entry is the
    /// path list *as written*; a relative entry roots against the declaring
    /// namespace.  Consumed by command resolution so a bare call reaches a proc
    /// on the namespace path before falling through to global.
    pub namespace_paths: HashMap<String, Vec<String>>,
    /// Spans of `namespace path` list words the analyser **could not** read
    /// statically — `namespace path $paths`, `namespace path "$ns ::a"` — in
    /// source order.
    ///
    /// [`Self::namespace_paths`] records only the resolvable declarations, so
    /// its absence cannot tell "this document declares no path" from "this
    /// document declares one whose entries are unknowable". A post-analysis
    /// consumer that must be exhaustive about namespace occurrences — the
    /// namespace rename tier, which rewrites each literal entry through the
    /// per-element [`NamespaceRef`] rows — needs the difference: an entry it
    /// cannot see is an entry it cannot rewrite.
    ///
    /// A *braced* word is not dynamic: braces suppress substitution, so
    /// `namespace path {::$ns ::a}` names a namespace literally called
    /// `::$ns` and is recorded as an ordinary literal path.
    pub namespace_path_computed: Vec<Span>,
    /// `auto_path` mutations (``lappend auto_path …`` / ``set auto_path …``).
    pub auto_path_entries: Vec<AutoPathEntry>,
    /// Namespace-qualified variable occurrences, in source order — see
    /// [`QualifiedVarRef`].  The cross-document variable reference set is
    /// built from these; an unqualified occurrence is never recorded.
    pub qualified_var_refs: Vec<QualifiedVarRef>,
    /// Original variable naming occurrences with typed byte geometry. This
    /// metadata supplies no successful store, allocation or live cell receipt.
    pub original_variable_symbols:
        Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence>,
    /// Conditional original write-name source cards. These retain selected
    /// naming grammar and optional authentic frame/home geometry independently
    /// of executed stores, variable-cell selection and rename coverage.
    pub original_variable_write_advice:
        Vec<crate::signature_scan::variable_symbol::OriginalVariableWriteAdvice>,
    /// Same-site producer/namespace disagreement withdraws navigation advice.
    pub original_variable_symbol_conflicts: Vec<Span>,
    pub(super) original_variable_roots:
        Vec<crate::signature_scan::variable_name::SignatureSourceVariableRoot>,
    pub(super) original_variable_alias_sites: Vec<Span>,
    pub(super) original_variable_name_unknowns: Vec<Span>,
    pub(super) original_variable_alias_obligations: Vec<(u32, usize)>,
    pub(super) original_variable_alias_operands: Vec<(
        u32,
        Span,
        Span,
        crate::registry_invocation::DeclarationVariableAliasPurpose,
    )>,
    pub(super) original_variable_alias_source_operands:
        Vec<crate::registry_invocation::OriginalSourceVariableAliasOperands>,
    pub(super) original_variable_alias_receipts:
        Vec<crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt>,
    /// Words naming a **namespace**, in source order — see [`NamespaceRef`].
    /// Both the declaring `namespace eval` name tokens (`declares: true`) and
    /// every other spelling of the same namespace, so go-to-definition /
    /// hover / find-references treat a namespace as a first-class symbol.
    pub namespace_refs: Vec<NamespaceRef>,
    /// Namespace-role assistance words whose original value could not be
    /// retained. These spans are May obligations for refactoring coverage;
    /// they establish no namespace selection, successful call or edit grant.
    pub namespace_name_unknowns: Vec<Span>,
    /// Variable-name argument words computed at run time, in source order —
    /// see [`DynamicVariableNameSite`].  The per-site provenance a
    /// post-analysis consumer needs to ask what a `$n` in a name position can
    /// actually spell.
    pub dynamic_variable_names: Vec<DynamicVariableNameSite>,
    /// Byte spans where command resolution is pinned to a namespace by
    /// runtime context rather than lexical nesting:
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
    /// Owner-attributed object-handle provenance from the compilation unit's
    /// VTA-lite lattice ([`crate::object_types::object_handle_facts`]) — the
    /// carrier the object-handle unification is built on.
    ///
    /// **Contract.** Best-effort, exactly like [`Self::instance_classes`]: an
    /// absent key means *no evidence was found in this document*, never *proof
    /// that the name holds no object*.  An empty value (the default) is what
    /// every CU-less analysis path produces, so a runtime-proof consumer must
    /// abstain rather than concluding anything from emptiness.
    ///
    /// **Which map to read.** `proven_reads` retains the original substitution
    /// and reaching SSA contents. Edits, references and refusal gates require
    /// that positioned proof; a singleton candidate is insufficient.
    /// `by_scope` and `any_scope` union contents versions and supply assistance
    /// only. Empty proof means abstention, never a fallback to a nominal class.
    ///
    /// Populated once per analysis, from the same `CompilationUnit` the
    /// CFG/SSA diagnostics ride on (`analyser/diagnostics.rs`); no other
    /// producer writes it and the per-item incremental path inherits it from
    /// the shell, so there is nothing to merge per grafted body.
    pub object_handle_facts: crate::object_types::ObjectHandleFacts,
    /// Simple names of instance commands created by a `CLASS create NAME …`
    /// construct — both when `CLASS` is a known user class *and* when it is an
    /// unresolved (external-package) command whose `create NAME` idiom clearly
    /// binds a new object command.  A `create NAME` call names a command, so
    /// later `NAME method` dispatch — and `$var method` where `var` provably
    /// holds one of these names — must not be flagged as an unknown command
    /// (W123) or a stray non-literal command word (W307).
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
    /// two, and a rename of one class's method would rewrite the other's call
    /// site.
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
    /// (`widget_command.rs`'s W001/E002/E003) can tell "no
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
    /// objects).
    pub object_methods: HashMap<String, Vec<ObjectMethodDef>>,
    /// Folded per-object member state, keyed like [`Self::object_methods`]
    /// (receiver variable simple name, plus the resolved object name when the
    /// receiver word folds to a constant), one entry per distinct receiver
    /// **binding** — see [`ObjectMemberState`].
    pub object_member_state: HashMap<String, Vec<ObjectMemberState>>,
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
    pub(crate) original_scoped_bodies: Vec<crate::registry_invocation::OriginalSourceScopedBody>,
    pub(crate) original_conditional_registry_metadata:
        HashMap<u32, Option<crate::registry_invocation::OriginalConditionalRegistryMetadata>>,
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
    /// Actual body grammar selected at the analysis ingress. Navigation
    /// reparses use this retained context rather than interpreting a profile
    /// name again. A default-constructed assistance result carries no grammar
    /// authority for matching a positioned source definition.
    pub body_lexer_config: Option<tcl_lexer::LexerConfig>,
    /// Session library-version overrides used for this analysis. Downstream
    /// providers use the same keyed BIG-IP/tool floors as diagnostics.
    pub library_versions: tcl_dialect::LibraryVersionOverrides,
}

impl AnalysisResult {
    /// Source declaration cards from genuine original operands and selected
    /// Registry `SymbolDef` roles. No publication, entered handler, cell or edit
    /// capability is supplied by this immutable inventory.
    pub fn original_symbol_declarations(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::symbol_name::OriginalSourceSymbolDeclaration>
    {
        self.original_symbol_metadata.iter()
    }

    /// Original export/clear events in source traversal order. These are readonly
    /// source naming advice, not export completion or live-import receipts.
    pub fn original_namespace_exports(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::original_name::SourceNamespaceExport> {
        self.original_namespace_exports.iter()
    }

    /// May obligations for export operands without an original evaluated input.
    /// These spans provide no role, namespace, runtime or editable value proof.
    #[must_use]
    pub fn original_namespace_export_unknowns(&self) -> &[Span] {
        &self.original_namespace_export_unknowns
    }

    /// Every retained original import/forget pattern, in source traversal order.
    /// This is source assistance without import completion or namespace lifetime.
    pub fn original_namespace_patterns(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::original_name::SourceNamespacePattern> {
        self.original_namespace_patterns.iter()
    }

    /// The analyser independently selected lexical-only declaration advice
    /// because no native name recipe was supplied. Byte-policy failures do not
    /// enable this compatibility surface; default-constructed results abstain.
    /// An independently retained hosted input remains outside this domain even
    /// when no complete source name occurrence can be retained.
    #[must_use]
    pub fn allows_lexical_declaration_advice(&self) -> bool {
        self.lexical_declaration_advice
            && !self.has_original_vendor_source_names()
            && self
                .resolved_input
                .as_ref()
                .is_some_and(super::ResolvedAnalysisInput::has_logical_source_name_context)
    }

    /// Positively retained Logical declaration advice for reporting consumers.
    /// Missing input, Native recipes and hosted source cannot borrow this domain
    /// from a copied flag or display profile. This grants no executed operation,
    /// current method-table closure, successful effect or writable identity.
    #[must_use]
    pub fn allows_retained_logical_declaration_advice(&self) -> bool {
        self.allows_lexical_declaration_advice()
    }

    /// Original hosted source producers remain present even when interpreted
    /// name units are unsupported. Consumers must not use reporting maps in
    /// place of these inputs or derive a C/Jim recipe from compatibility.
    pub fn original_vendor_source_names(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::vendor_name::VendorSourceNameOccurrence> {
        self.original_vendor_source_names
            .values()
            .filter_map(Option::as_ref)
    }

    /// Whether this analysis owns hosted source names, including conflicting
    /// producers whose selected input has been withdrawn.
    #[must_use]
    pub fn has_original_vendor_source_names(&self) -> bool {
        !self.original_vendor_source_names.is_empty()
    }

    /// Exact original word at this consumer's source point. Registry name
    /// roles and purpose-specific materialisation are independent queries.
    #[must_use]
    pub fn original_vendor_source_name_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        span: tcl_lexer::Span,
    ) -> Option<&crate::signature_scan::vendor_name::VendorSourceNameOccurrence> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let occurrence = self.original_vendor_source_names.get(&span)?.as_ref()?;
        occurrence
            .name_input()
            .matches_source(image, config)
            .then_some(occurrence)
    }

    /// Registry-selected hosted variable and formal source advice. These rows
    /// supply neither selected cells nor entered frames or runtime name units.
    pub fn original_vendor_variable_advice(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::vendor_variable::VendorSourceVariableAdvice>
    {
        self.original_vendor_variable_advice.iter()
    }

    /// Authentic hosted source body at a cursor, independently of runtime
    /// activation and execution-domain state. Conflicting body owners abstain.
    #[must_use]
    pub fn original_vendor_variable_body_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<&crate::signature_scan::vendor_variable::VendorSourceVariableBody> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let contains =
            |body: &&crate::signature_scan::vendor_variable::VendorSourceVariableBody| {
                body.matches_source(image, config) && body.contains_cursor(offset)
            };
        let length = self
            .original_vendor_variable_bodies
            .iter()
            .filter(contains)
            .map(|body| body.span().end() - body.span().start())
            .min()?;
        let mut bodies = self
            .original_vendor_variable_bodies
            .iter()
            .filter(contains)
            .filter(|body| body.span().end() - body.span().start() == length);
        let first = bodies.next()?;
        bodies.all(|body| body == first).then_some(first)
    }

    /// Hosted procedure declaration advice from authentic source words and roles.
    pub fn original_vendor_procedure_declarations(
        &self,
    ) -> impl Iterator<
        Item = &crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata<ProcDef>,
    > {
        self.original_vendor_procedure_metadata.iter()
    }

    /// Hosted class declaration advice, separate from runtime class allocations.
    pub fn original_vendor_class_declarations(
        &self,
    ) -> impl Iterator<
        Item = &crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata<ClassDef>,
    > {
        self.original_vendor_class_metadata.iter()
    }

    /// Hosted Registry symbol declarations, including owned unsupported names.
    pub fn original_vendor_symbol_declarations(
        &self,
    ) -> impl Iterator<
        Item = &crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata<DefinedSymbol>,
    > {
        self.original_vendor_symbol_metadata.iter()
    }

    /// Every independently retained original procedure declaration.
    pub fn original_procedure_declarations(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>>
    {
        self.original_procedure_metadata.iter()
    }

    /// Original byte formal topology from this exact retained procedure
    /// declaration and unanimous Registry-selected declaration recipe.
    /// Reporting parameter names supply no fallback or activation authority.
    #[must_use]
    pub fn original_procedure_formals(
        &self,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::signature_scan::formal_parameters::SignatureSourceFormalParameters> {
        if !self.original_procedure_metadata.contains(declaration)
            || !self.matches_original_source_image(
                declaration.name_input().source_image(),
                declaration.name_input().lexer_config(),
            )
        {
            return None;
        }
        self.command_realm
            .as_ref()?
            .source_bindings_ref()
            .original_procedure_formals_at(declaration.declaration_site(), registry)
    }

    /// Semantic alpha equivalence of a closed source containing one procedure
    /// whose sole required scalar formal is returned as the same whole object.
    /// Actual frame/read/handler effects are proved separately from source
    /// rename coverage. The receipt supplies neither an edit nor a reached call.
    #[must_use]
    pub fn original_scalar_body_alpha_rename(
        &self,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>,
        new_native_tail: &[u8],
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::command_binding::OriginalScalarBodyAlphaRename> {
        use crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver;
        if !self.original_procedure_metadata.contains(declaration)
            || !self.matches_original_source_image(
                declaration.name_input().source_image(),
                declaration.name_input().lexer_config(),
            )
        {
            return None;
        }
        let bindings = self.command_realm.as_ref()?.source_bindings_ref();
        if std::env::var_os("TCL_LSP_TRACE_SCALAR_ALPHA").is_some() {
            eprintln!(
                "SCALAR_ALPHA offset={} symbols={} formals={} completed-world={}",
                declaration.declaration_site().offset,
                self.original_variable_symbols.len(),
                self.original_variable_symbols
                    .iter()
                    .filter(|occurrence| occurrence.receiver()
                        == OriginalVariableSymbolReceiver::FormalDeclaration)
                    .count(),
                bindings.original_completed_command_world().is_some()
            );
        }
        let mut agreed = None;
        for occurrence in &self.original_variable_symbols {
            if std::env::var_os("TCL_LSP_TRACE_SCALAR_ALPHA").is_some()
                && occurrence.receiver() == OriginalVariableSymbolReceiver::FormalDeclaration
            {
                eprintln!(
                    "SCALAR_ALPHA formal-span={}..{} declaration={} coverage={}",
                    occurrence.span().start(),
                    occurrence.span().end(),
                    occurrence.is_declaration(),
                    self.original_variable_rename_is_complete(occurrence.symbol())
                );
            }
            if occurrence.receiver() != OriginalVariableSymbolReceiver::FormalDeclaration
                || !occurrence.is_declaration()
                || !self.original_variable_rename_is_complete(occurrence.symbol())
            {
                continue;
            }
            let Some(receipt) = bindings.original_scalar_body_alpha_rename(
                declaration,
                occurrence.symbol(),
                new_native_tail,
                registry,
            ) else {
                continue;
            };
            if agreed.as_ref().is_some_and(|previous| previous != &receipt) {
                return None;
            }
            agreed = Some(receipt);
        }
        agreed
    }

    /// Every independently retained original class declaration.
    pub fn original_class_declarations(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::original_name::SourceDeclarationMetadata<ClassDef>>
    {
        self.original_class_metadata.iter()
    }

    /// Original class configurations retain their own target lookup obligations.
    /// These deltas do not inherit a class identity from reporting maps.
    pub fn original_class_configurations(
        &self,
    ) -> impl Iterator<Item = &OriginalSourceClassConfiguration> {
        self.original_class_configuration_metadata.iter()
    }

    /// Own-object source deltas retain their independently selected instance.
    /// Consumers must prove a current allocation join; walk order is not a clock.
    pub fn original_object_configurations(
        &self,
    ) -> impl Iterator<Item = &OriginalSourceObjectConfiguration> {
        self.original_object_configuration_metadata.iter()
    }

    /// Byte-slot declaration candidates in local-before-root lookup order.
    /// Every declaration at the first matching slot is retained; this metadata
    /// query does not select a temporal installation or reached implementation.
    #[must_use]
    pub fn procedures_for_original_name<'a>(
        &'a self,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
    ) -> Vec<&'a ProcDef> {
        let Some(lookup) = crate::signature_scan::scope::SignatureSourceLookup::from_input(
            namespace.clone(),
            input,
        ) else {
            return Vec::new();
        };
        lookup.first_matching_publications(
            self.original_procedure_metadata
                .iter()
                .map(|record| (record.name(), record.metadata())),
        )
    }

    /// Actual retained naming geometry of the innermost source scope. Missing
    /// geometry cannot be reconstructed from its UI namespace label.
    #[must_use]
    pub fn original_namespace_scope_at(
        &self,
        offset: u32,
    ) -> Option<&crate::signature_scan::scope::SignatureNamespaceScope> {
        super::scope::original_namespace_scope_at(&self.global_scope, offset)
    }

    /// Exact temporal source world used by this analysis. Missing evidence
    /// remains unknown; rebuilding a realm from the dialect label cannot
    /// replace its native entry, provider or source-instance obligations.
    #[must_use]
    pub fn retained_command_realm(&self) -> Option<&crate::realm::CommandBindingRealm> {
        self.command_realm.as_deref()
    }

    /// Independently completed Normal source publication world. This is final
    /// source liveness advice, separate from positioned invocation lookup and
    /// physical interpreter existence. Unknown or abrupt source stays absent.
    #[must_use]
    pub const fn original_completed_command_world(
        &self,
    ) -> Option<&crate::command_binding::OriginalCompletedCommandWorld> {
        self.original_command_world.as_ref()
    }

    /// Actual compilation profile retained at analysis ingress. Assistance
    /// records without retained inputs leave this unknown.
    #[must_use]
    pub fn resolved_profile(&self) -> Option<&'static tcl_dialect::DialectProfile> {
        self.resolved_input.as_ref().map(|input| input.unit_profile)
    }

    /// Select the smallest original naming occurrence at a byte cursor. Equal
    /// source geometry must unanimously identify the same typed byte symbol.
    /// Display strings and current runtime cells are not consulted.
    #[must_use]
    pub fn original_variable_symbol_at_offset(
        &self,
        offset: u32,
    ) -> Option<&crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        let contains = |span: Span| span.start() <= offset && offset < span.end();
        if self
            .original_variable_symbol_conflicts
            .iter()
            .copied()
            .any(contains)
        {
            return None;
        }
        let length = self
            .original_variable_symbols
            .iter()
            .filter(|site| contains(site.span()))
            .map(|site| site.span().end() - site.span().start())
            .min()?;
        let mut sites = self.original_variable_symbols.iter().filter(|site| {
            contains(site.span()) && site.span().end() - site.span().start() == length
        });
        let first = sites.next()?;
        sites
            .all(|site| site.symbol() == first.symbol())
            .then_some(first)
    }

    /// Whether every selected alias declaration has its original byte/frame
    /// receipt. Unknown computed variable names remain a rename barrier. This
    /// is source coverage only, never executed alias or current-cell authority.
    /// A document need not contain the symbol; selection and edit existence
    /// remain independent caller obligations.
    #[must_use]
    pub fn original_variable_rename_is_complete(
        &self,
        symbol: &crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    ) -> bool {
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_COVERAGE policy={:?} local={} dynamic={:?} unknowns={:?} aliases={:?} receipt_sites={:?} conflicts={:?}",
                symbol.policy(),
                !symbol.is_namespace(),
                self.dynamic_variable_names,
                self.original_variable_name_unknowns,
                self.original_variable_alias_obligations,
                self.original_variable_alias_receipts
                    .iter()
                    .map(crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt::invocation_offset)
                    .collect::<Vec<_>>(),
                self.original_variable_symbol_conflicts
            );
        }
        if self.retained_command_realm().is_none_or(|realm| {
            realm
                .source_bindings_ref()
                .original_variable_formal_name_is_unrepresented(symbol)
        }) {
            return false;
        }
        self.dynamic_variable_names.is_empty()
            && self.original_variable_name_unknowns.is_empty()
            && self
                .original_variable_alias_obligations
                .iter()
                .all(|&(site, count)| {
                    self.original_variable_alias_receipts
                        .iter()
                        .filter(|receipt| receipt.invocation_offset() == site)
                        .count()
                        == count
                })
            && self.original_variable_symbol_conflicts.is_empty()
    }

    /// Original naming advice belongs to the exact complete consumer image,
    /// channel and scanner configuration. Copied producer images may differ;
    /// they do not replace this independently retained consumer correspondence.
    #[must_use]
    pub fn original_variable_symbol_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<&crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        self.original_variable_symbol_at_offset(offset)
    }

    /// Authentic lexical substitution at this cursor, even when its selected
    /// variable symbol is unknown. Membership, complete source and grammar
    /// agree; no namespace-role operand or computed value becomes a root.
    #[must_use]
    pub fn original_variable_root_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<&crate::signature_scan::variable_name::SignatureSourceVariableRoot> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let contains =
            |root: &&crate::signature_scan::variable_name::SignatureSourceVariableRoot| {
                let span = root.part_span();
                root.source_image() == image
                    && root.lexer_config() == config
                    && span.start() <= offset
                    && offset < span.end()
            };
        let length = self
            .original_variable_roots
            .iter()
            .filter(contains)
            .map(|root| root.part_span().end() - root.part_span().start())
            .min()?;
        let mut roots = self
            .original_variable_roots
            .iter()
            .filter(contains)
            .filter(|root| root.part_span().end() - root.part_span().start() == length);
        let first = roots.next()?;
        roots.all(|root| root == first).then_some(first)
    }

    /// Source-only local alias candidates retained by their original
    /// declarations, even when the body has no complete read of that alias.
    /// The whole consumer image/configuration must still match. Target cells,
    /// entered links and editable reference coverage remain independent.
    #[must_use]
    pub fn original_variable_alias_advice_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Vec<crate::signature_scan::variable_symbol::OriginalVariableAliasAdvice<'_>>> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        Some(self.original_variable_alias_receipts.iter()
            .filter(|receipt| receipt.frame().lexer_config() == config)
            .map(crate::signature_scan::variable_symbol::OriginalVariableAliasAdvice::from_receipt)
            .collect())
    }

    /// Original local naming frame at this cursor, including whitespace inside
    /// an authenticated body. This is lexical visibility only, without an
    /// entered activation, current contents or native frame capability.
    #[must_use]
    pub fn original_variable_frame_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<crate::command_binding::SourceOriginalVariableFrame> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        self.retained_command_realm()?
            .source_bindings_ref()
            .original_variable_frame_at_offset(offset, config)
    }

    /// Complete original analysis/source correspondence, including input
    /// channel and every lexer override. This supplies no current value,
    /// completed execution, namespace existence or native capability.
    #[must_use]
    pub fn matches_original_source_image(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        let Some(input) = self.resolved_input.as_ref() else {
            return false;
        };
        self.retained_command_realm().is_some_and(|realm| {
            realm.matches_resolved_analysis_input(input)
                && input.lexer_config() == config
                && realm
                    .source_bindings_ref()
                    .matches_original_source_image(image, config)
        })
    }

    /// Checked original expression function at this source position. Fixed
    /// native tables and command-table navigation retain separate purposes;
    /// this supplies no command-head word, evaluation or compiler admission.
    #[must_use]
    pub fn original_math_function_at_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<crate::command_binding::OriginalMathFunctionOccurrence> {
        let registry = self.resolved_registry()?;
        let end = offset.checked_add(1)?;
        if usize::try_from(end).ok()? > image.bytes().len() {
            return None;
        }
        let matches = self.original_math_functions_in_source(
            image,
            config,
            tcl_lexer::Span::new(offset, end),
        )?;
        let mut selected = matches.into_iter().filter(|function| {
            let span = function.span();
            offset >= span.start()
                && offset < span.end()
                && function.matches_source(image, config, registry)
        });
        let first = selected.next()?;
        selected.all(|other| other == first).then_some(first)
    }

    /// Readonly checked function occurrences in an original source region.
    /// The complete image, channel, lexer configuration and retained Registry
    /// must agree. Missing ownership returns `None`, independently of an empty
    /// occurrence list; no body entry or runtime function roster is issued.
    #[must_use]
    pub fn original_math_functions_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        region: tcl_lexer::Span,
    ) -> Option<Vec<crate::command_binding::OriginalMathFunctionOccurrence>> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        if region.start() > region.end() || usize::try_from(region.end()).ok()? > image.len() {
            return None;
        }
        let registry = self.resolved_registry()?;
        let rows = self
            .retained_command_realm()?
            .source_bindings_ref()
            .original_math_functions_in_source(registry, image, config, region);
        rows.iter()
            .all(|row| row.matches_source(image, config, registry))
            .then_some(rows)
    }

    /// Exact editing command store retained with its availability generation.
    /// It supplies metadata, independently of reached command identities.
    #[must_use]
    pub fn resolved_registry(&self) -> Option<&tcl_registry::CommandRegistry> {
        self.resolved_input
            .as_ref()
            .map(|input| input.context.commands().as_ref())
    }

    /// Publish a class assistance record while retaining original declarations
    /// for navigation. Updating members of the same declaration does not mint
    /// a second declaration or a positioned execution fact.
    pub(crate) fn retain_class_declaration(&mut self, qualified: String, mut class: ClassDef) {
        if let Some(previous) = self.all_classes.get(&qualified) {
            class.source_name_ambiguous.observe_if(previous.source_name_ambiguous.is_observed()
                || matches!((&class.source_name, &previous.source_name), (Some(left), Some(right)) if left.slot() != right.slot() || left.policy() != right.policy()));
        }
        if let Some(previous) = self.all_classes.insert(qualified.clone(), class)
            && self.all_classes[&qualified].name_span != previous.name_span
        {
            self.superseded_classes
                .entry(qualified)
                .or_default()
                .push(previous);
        }
    }

    /// Does this document configure the ensemble resolving to `ensemble`
    /// with `namespace ensemble … -prefixes 0`?
    ///
    /// [`Self::prefixless_ensembles`] is keyed by the ensemble's resolved
    /// command name, which callers hold in either spelling depending on how
    /// they got it, so both are tried here rather than at each call site.
    #[must_use]
    pub fn ensemble_refuses_prefixes(&self, ensemble: &str) -> bool {
        self.prefixless_ensembles.contains(ensemble)
            || self
                .prefixless_ensembles
                .contains(&crate::naming::root_unrooted_key(
                    crate::naming::unroot_rooted_key(ensemble).unwrap_or(ensemble),
                ))
    }

    /// Resolve `sub` against the subcommands recorded for the ensemble
    /// command `ensemble`, the way the ensemble itself would
    /// (`NsEnsembleImplementationCmd`): an exact name first, then — unless
    /// the ensemble was configured `-prefixes 0` — a **unique** prefix.
    ///
    /// The match rule is not re-derived here: it is
    /// [`tcl_cmd_core::ensemble::resolve_subcommand`], the owner the engines
    /// dispatch through, so navigation follows exactly the subcommand real
    /// Tcl would run. An *ambiguous* prefix resolves to nothing and the
    /// caller keeps abstaining — the ensemble would error there, so there is
    /// no target to point at.
    ///
    /// `None` when the ensemble is unknown to this document, its `-map` was
    /// dynamic (nothing is ever recorded for it), or `sub` matches nothing.
    #[must_use]
    pub fn resolve_ensemble_subcommand(
        &self,
        ensemble: &str,
        sub: &str,
    ) -> Option<&EnsembleSubcommandTarget> {
        let subs = self.ensemble_subcommand_targets.get(ensemble)?;
        if let Some(entry) = subs.get(sub) {
            return Some(entry); // the exact-name probe C does first
        }
        // The recorded map is unordered; the scan needs a stable candidate
        // order to call a prefix unique, and C's own table is sorted.
        let mut names: Vec<&str> = subs.keys().map(String::as_str).collect();
        names.sort_unstable();
        let index = tcl_cmd_core::ensemble::resolve_subcommand(
            &names,
            sub.as_bytes(),
            !self.ensemble_refuses_prefixes(ensemble),
        )?;
        subs.get(names[index])
    }

    /// Every **class factory** this document declares, keyed by qualified
    /// name — the slice a host merges into the workspace factory index it
    /// feeds back through
    /// [`Analyser::with_workspace_class_factories`](super::Analyser::with_workspace_class_factories).
    ///
    /// A document that declares no user metaclass — nearly all of them —
    /// contributes an empty map, so the merged index stays empty and every
    /// consuming walk behaves exactly as it did before.
    #[must_use]
    pub fn class_factories(&self) -> ClassFactoryIndex {
        self.all_classes
            .iter()
            .filter_map(|(qname, class)| Some((qname.clone(), class.factory.clone()?)))
            .collect()
    }

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
    /// [`DialectProfile::namespace_var_global_fallback`](tcl_dialect::DialectProfile::namespace_var_global_fallback)
    /// applied to [`Self::dialect`]. An unresolved dialect answers `false`
    /// (the stricter 9.0 semantics).
    #[must_use]
    pub fn ns_var_global_fallback(&self) -> bool {
        self.resolved_profile()
            .or_else(|| tcl_dialect::DialectProfile::find(&self.dialect))
            .is_some_and(tcl_dialect::DialectProfile::namespace_var_global_fallback)
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

    /// Match a retained implementation allocation to its original authored
    /// declaration. This is navigation only; no lexical-order or final-name
    /// reconstruction substitutes for the selected allocation. A materialised
    /// or loaded script has no editable declaration in this document.
    #[must_use]
    pub fn proc_for_definition(
        &self,
        definition: &crate::command_binding::SourceCommandDefinition,
        source: &str,
    ) -> Option<&ProcDef> {
        if definition.kind() != crate::command_binding::SourceCommandDefinitionKind::Procedure {
            return None;
        }
        let allocation = definition.allocation();
        if !matches!(allocation.site.source.kind(), crate::command_binding::SourceOriginKind::Authored(authored) if authored.bytes() == source.as_bytes())
        {
            return None;
        }
        let mut original = self
            .original_procedure_metadata
            .iter()
            .filter(|record| record.declaration_site() == &allocation.site);
        if let Some(first) = original.next() {
            return original
                .all(|record| record == first)
                .then_some(first.metadata());
        }
        let tokens = self.definition_tokens(definition, source)?;
        self.proc_declarations(&definition.allocation().command)
            .find(|declaration| tokens.contains(&declaration.name_span))
    }

    /// Navigate to the original authored class declaration matching a retained
    /// class receipt. Instance and procedure allocations cannot borrow a class
    /// record; overwritten or foreign records cannot supply its source span.
    #[must_use]
    pub fn class_for_definition(
        &self,
        definition: &crate::command_binding::SourceCommandDefinition,
        source: &str,
    ) -> Option<&ClassDef> {
        if definition.kind() != crate::command_binding::SourceCommandDefinitionKind::Class {
            return None;
        }
        let allocation = definition.allocation();
        if !matches!(allocation.site.source.kind(), crate::command_binding::SourceOriginKind::Authored(authored) if authored.bytes() == source.as_bytes())
        {
            return None;
        }
        let mut original = self
            .original_class_metadata
            .iter()
            .filter(|record| record.declaration_site() == &allocation.site);
        if let Some(first) = original.next() {
            return original
                .all(|record| record == first)
                .then_some(first.metadata());
        }
        let tokens = self.definition_tokens(definition, source)?;
        let qualified = &definition.allocation().command;
        self.superseded_classes
            .get(qualified)
            .into_iter()
            .flatten()
            .chain(self.all_classes.get(qualified))
            .find(|declaration| tokens.contains(&declaration.name_span))
    }

    fn definition_tokens(
        &self,
        definition: &crate::command_binding::SourceCommandDefinition,
        source: &str,
    ) -> Option<Vec<Span>> {
        let allocation = definition.allocation();
        if !matches!(allocation.site.source.kind(),
            crate::command_binding::SourceOriginKind::Authored(authored) if authored.bytes() == source.as_bytes())
        {
            return None;
        }
        let config = self.body_lexer_config?;
        let start = usize::try_from(allocation.site.offset).ok()?;
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            source.get(start..)?,
            allocation.site.offset,
            config,
        );
        Some(
            commands
                .first()?
                .argv
                .iter()
                .map(|word| word.span)
                .collect(),
        )
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
        let mut retained = self.command_invocations.iter().filter_map(|invocation| {
            (invocation.range.start() == call_off
                && (invocation.resolved_qualified_name.as_deref() == Some(qualified)
                    || invocation
                        .resolved_definition
                        .as_ref()
                        .is_some_and(|definition| definition.allocation().command == qualified)))
            .then_some(invocation.resolved_definition.as_ref())
            .flatten()
        });
        if let Some(definition) = retained.next() {
            if !retained.all(|alternative| alternative == definition) {
                return None;
            }
            let crate::command_binding::SourceOriginKind::Authored(source) =
                definition.allocation().site.source.kind()
            else {
                return None;
            };
            return self.proc_for_definition(definition, source.try_text().ok()?);
        }
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

    /// Innermost declaration body extent retained by genuine original source
    /// metadata. It supplies source ordering geometry without entered execution.
    #[must_use]
    pub fn original_definition_body_span(&self, off: u32) -> Option<Span> {
        self.original_procedure_declarations()
            .map(|record| record.metadata().body_span)
            .chain(
                self.original_class_declarations()
                    .map(|record| record.metadata().body_span),
            )
            .filter(|span| span.start() <= off && off < span.end())
            .min_by_key(|span| span.end() - span.start())
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
    /// Authentic original package operand and selected native package key.
    pub original_name: Option<crate::signature_scan::original_name::SourcePackageName>,
    /// Provided package name.
    pub name: String,
    /// Version string when present.
    pub version: Option<String>,
    /// Span of the originating ``package`` token.
    pub range: Span,
    /// ``true`` when the call is inside a guarded branch (an
    /// ``if``/``elseif``/``else`` body, a ``catch`` script, or a
    /// ``try``/``on``/``trap``/``finally`` clause), matching
    /// [`SignaturePackageRequire::conditional`].
    ///
    /// The shim idiom makes this load-bearing: tcllib's
    /// ``doctools2idx/import_json.tcl`` writes ``package provide dict 1``
    /// inside ``if {[package vcompare …] < 0} { if {[catch {package
    /// require dict}]} { … } }`` — a fake ``dict`` package supplied only
    /// on an old interpreter.  Reading that as "this document provides
    /// ``dict``" would name the wrong file as a package's provider on
    /// every other interpreter.
    pub conditional: bool,
}

/// `package ifneeded NAME VERSION ?SCRIPT?` record.
///
/// The script argument is deliberately **not** kept.  It is evaluated later,
/// in the global namespace, by whichever `package require` first needs this
/// version, and it may do anything at all — `source` one file, `load` a shared
/// library, branch on the platform, or provide the package inline.  A consumer
/// asking "which statements does a `package require NAME` run?" therefore
/// cannot answer it from here; what this record supplies is the *fact that the
/// question is open*, which is exactly the abstention a package-derived load
/// order needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageIfneeded {
    /// Authentic original package operand and selected native package key.
    pub original_name: Option<crate::signature_scan::original_name::SourcePackageName>,
    /// Package the load script is registered for.
    pub name: String,
    /// Version the script is registered for.
    pub version: String,
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
/// reference set.
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
    /// Original lexical root or retained evaluated variable-name operand. The
    /// producer kind selects neither a compiler primary nor an executed cell.
    pub original_name_input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    /// Independently retained namespace geometry at this occurrence. Missing
    /// geometry cannot be reconstructed from `qualified_name`.
    pub original_namespace: Option<crate::signature_scan::scope::SignatureNamespaceScope>,
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
/// referenceable symbol.
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
    /// Original namespace operand. List children and evaluated values remain
    /// readonly; their source lineage cannot acquire complete-word geometry.
    pub original_name_input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    /// Exact namespace context before interpreting the original operand.
    pub source_context: Option<crate::signature_scan::scope::SignatureNamespaceScope>,
    /// Original selected word value, retained before any report rendering.
    pub original_name: String,
    /// Candidate literal value range. Consumers verify these exact original bytes
    /// against source before projecting editable subranges; escaped/computed words decline.
    pub source_span: Option<Span>,
    /// Exact original namespace selection, independent of its rendered report.
    pub source_namespace: Option<crate::signature_scan::scope::SignatureNamespaceScope>,
    /// Independently selected naming policy for the original namespace operand.
    pub name_policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
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

impl NamespaceRef {
    /// Select the original namespace operand using independently retained context.
    /// The report fallback supplies presentation only when that context is absent.
    #[must_use]
    pub(crate) fn from_original(
        source_context: Option<crate::signature_scan::scope::SignatureNamespaceScope>,
        name_policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
        original_name: &str,
        span: Span,
        content_offset: u16,
        declares: bool,
        report_fallback: String,
    ) -> Self {
        // A native policy cannot interpret the reporting String as original
        // native units. The genuine input producer is joined separately below.
        let source_namespace = name_policy
            .is_none()
            .then(|| {
                source_context
                    .as_ref()
                    .and_then(|current| current.child(original_name, None))
            })
            .flatten();
        let qualified_name = source_namespace
            .as_ref()
            .and_then(crate::signature_scan::scope::SignatureNamespaceScope::display)
            .unwrap_or(report_fallback);
        let source_span = span
            .start()
            .checked_add(u32::from(content_offset))
            .and_then(|start| {
                Some(Span::new(
                    start,
                    start.checked_add(u32::try_from(original_name.len()).ok()?)?,
                ))
            });
        Self {
            original_name_input: None,
            source_context,
            original_name: original_name.to_owned(),
            source_span,
            source_namespace,
            name_policy,
            qualified_name,
            span,
            declares,
        }
    }

    pub(crate) fn with_original_input(
        mut self,
        input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    ) -> Self {
        if let Some(input) = &input {
            self.name_policy = Some(input.policy());
            self.source_namespace = self
                .source_context
                .as_ref()
                .and_then(|scope| scope.child_from_input(input));
            if let Some(report) = self
                .source_namespace
                .as_ref()
                .and_then(crate::signature_scan::scope::SignatureNamespaceScope::display)
            {
                self.qualified_name = report;
            }
        }
        self.original_name_input = input;
        self
    }

    fn original_extent_span(
        &self,
        source: &str,
        wanted: &crate::signature_scan::scope::SignatureNamespaceScope,
        member: bool,
    ) -> Option<Span> {
        let input = self.original_name_input.as_ref()?;
        let key = input.original_word_key()?;
        let word = key.original_word();
        if word.image() != &tcl_lexer::SourceImage::document(source) {
            return None;
        }
        let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            std::slice::from_ref(word),
            input.policy().string_protocol(),
        )
        .ok()?;
        if captured.literal(0)? != input.bytes() {
            return None;
        }
        let protocol = input.policy().recipe();
        let current = self.source_context.as_ref()?.context()?;
        let wanted = wanted.context()?;
        let extent = if member {
            tcl_syntax::naming::native_written_namespace_member_extent(
                protocol,
                current,
                input.bytes(),
                wanted,
            )?
        } else {
            tcl_syntax::naming::native_written_namespace_prefix_extent(
                protocol,
                current,
                input.bytes(),
                wanted,
            )?
        };
        captured.original_literal_extent(0, extent)
    }

    /// Covering original prefix for an exact retained ancestor namespace.
    #[must_use]
    pub fn written_ancestor_span(
        &self,
        source: &str,
        wanted: &crate::signature_scan::scope::SignatureNamespaceScope,
    ) -> Option<Span> {
        self.original_extent_span(source, wanted, false)
    }
    /// Original component range for an exact retained namespace.
    #[must_use]
    pub fn written_member_span(
        &self,
        source: &str,
        wanted: &crate::signature_scan::scope::SignatureNamespaceScope,
    ) -> Option<Span> {
        self.original_extent_span(source, wanted, true)
    }
}

/// One variable-name argument whose word is **computed at run time** — a
/// registry [`tcl_registry::ArgRole::VarWrite`] / [`tcl_registry::ArgRole::VarRead`]
/// position holding a `$`/`[…]` substitution (`set $n 2`, `variable $n`,
/// `namespace upvar ::ns $n local`) — together with what the analyser could
/// prove about the value.
///
/// The proof is analyser *state*: `Scope::lookup_dominating_const_string`
/// knows, while the walk is in progress, that the `$n` in `set n other; set
/// $n 2` is `other` and can never be anything else at that site.  Nothing
/// per-site survived into [`AnalysisResult`], so a post-analysis consumer —
/// the namespace-variable rename gate, which has to be exhaustive about which
/// cells a document can name — had only the word's own text to reason from
/// and refused on every bare `$n`, which is a lone wildcard.
///
/// Recording the site is what lets that gate ask the *value* question as well
/// as the textual one.  Abstain-toward-refuse is unchanged: a site whose
/// resolution is [`None`] proves nothing, and a site the walk never reached
/// leaves no row at all, so a consumer that only *narrows* on a
/// [`Some`] resolution can never widen what it accepts.
///
/// A **brace-quoted** word is not here: `set {$n} 1` names the variable
/// literally called `$n` and substitutes nothing (tclsh 8.6.14: `set {$n} v;
/// info exists {$n}` -> 1 while `info exists n` -> 0), so it is not a dynamic
/// name at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicVariableNameSite {
    /// Byte span of the name word as written.
    pub span: Span,
    /// The name the word evaluates to when every substitution in it has a
    /// value that **dominates** this site — the straight-line constant
    /// `Scope::lookup_dominating_const_string` proves.
    ///
    /// [`None`] when it does not: a branch-dependent binding, a parameter, a
    /// command substitution, or any variable the constant lattice does not
    /// track.  A consumer must treat `None` as "could be anything".
    pub resolved: Option<String>,
    /// `true` when the position **writes** the named variable
    /// ([`tcl_registry::ArgRole::VarWrite`]), `false` when it reads it.
    pub writes: bool,
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
    /// `true` when this declaration came from a workspace sidecar rather than
    /// the analysed document. Such declarations participate in resolution but
    /// cannot produce source-positioned shadow diagnostics.
    pub from_sidecar: bool,
}

impl StubCommandDef {
    /// Ingest this directive as a provenance-tagged
    /// [`tcl_registry::model::DeclaredCommand`] (gap ruling R1).
    ///
    /// The source span stays here — the directive keeps it for diagnostic
    /// emitters — and each argument's role word is canonicalised through
    /// the registry's own [`tcl_registry::model::role_for_word`], so a
    /// declared argument and a catalogue argument are the same fact.
    ///
    /// [`Self::from_sidecar`] chooses the §6.4 trust class: a workspace
    /// `.tcl.stubs` file is [`Provenance::WorkspaceUntrusted`], an inline
    /// block in the analysed buffer is [`Provenance::Document`]. The
    /// directive's trailing flag set is deliberately **not** carried: it
    /// has never had a consumer, and R1's principle P-C says a fact comes
    /// back with the consumer that needs it.
    ///
    /// [`Provenance::WorkspaceUntrusted`]: tcl_dialect::model::Provenance::WorkspaceUntrusted
    /// [`Provenance::Document`]: tcl_dialect::model::Provenance::Document
    #[must_use]
    pub fn to_declared_command(&self) -> tcl_registry::model::DeclaredCommand {
        use tcl_dialect::model::Provenance;
        use tcl_registry::model::{DeclaredArgument, DeclaredCommand, role_for_word};
        DeclaredCommand::new(
            self.name.clone(),
            self.args
                .iter()
                .map(|a| DeclaredArgument {
                    name: a.name.clone(),
                    role: role_for_word(&a.role),
                    optional: a.optional,
                })
                .collect(),
            if self.from_sidecar {
                Provenance::WorkspaceUntrusted
            } else {
                Provenance::Document
            },
        )
    }
}

/// Ingest a slice of [`StubCommandDef`] records as the document's
/// [`tcl_registry::model::DeclaredSurface`].
///
/// Declarations are ingested in slice order and a later one of the same
/// name replaces an earlier one, which is both the directive's documented
/// "last one wins" rule and what makes an inline block override a
/// workspace sidecar (the caller ingests sidecar declarations first).
#[must_use]
pub fn build_declared_surface(defs: &[StubCommandDef]) -> tcl_registry::model::DeclaredSurface {
    let mut surface = tcl_registry::model::DeclaredSurface::new();
    for def in defs {
        surface.declare(def.to_declared_command());
    }
    surface
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
    /// See [`StubCommandDef::from_sidecar`].
    pub from_sidecar: bool,
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
        assert_eq!(s.variables.len(), 0);
        assert_eq!(s.procs.len(), 0);
        assert_eq!(s.classes.len(), 0);
        assert_eq!(s.children, [] as [crate::analyser::types::Scope; 0]);
    }

    #[test]
    fn analysis_result_default_is_empty() {
        let r = AnalysisResult::default();
        assert_eq!(r.global_scope.kind, ScopeKind::Global);
        assert_eq!(r.all_procs.len(), 0);
        assert_eq!(r.all_classes.len(), 0);
        assert_eq!(r.all_variables.len(), 0);
        assert_eq!(r.diagnostics, [] as [crate::analyser::types::Diagnostic; 0]);
        assert_eq!(
            r.command_invocations,
            [] as [crate::signature_scan::types::SignatureCommandInvocation; 0]
        );
        assert_eq!(
            r.package_requires,
            [] as [crate::signature_scan::types::SignaturePackageRequire; 0]
        );
        assert_eq!(
            r.source_targets,
            [] as [crate::signature_scan::types::SignatureSource; 0]
        );
        assert_eq!(r.command_aliases.len(), 0);
        assert_eq!(
            r.namespace_imports,
            [] as [crate::signature_scan::types::SignatureNamespaceImport; 0]
        );
        assert!(r.unknown_proc_info.is_none());
    }

    #[test]
    fn class_def_default_has_oo_class_metaclass() {
        // ``metaclass`` is the only non-trivial default the rest of
        // the analyser depends on.
        let c = ClassDef::default();
        assert_eq!(c.metaclass, "oo::class");
        // …and it is a stand-in, not an observation.
        assert_eq!(c.metaclass_provenance, MetaclassProvenance::StandIn);
        assert_eq!(c.constructors, [] as [crate::analyser::types::MethodDef; 0]);
        assert!(c.destructor.is_none());
        assert_eq!(c.variables, [] as [std::string::String; 0]);
        assert_eq!(c.properties.len(), 0);
        assert_eq!(c.filters, [] as [std::string::String; 0]);
        assert_eq!(c.exports.len(), 0);
        assert_eq!(c.unexports.len(), 0);
        assert_eq!(c.doc, "");
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    fn string_set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    fn method(name: &str, params: &[&str]) -> MethodDef {
        MethodDef {
            name: name.to_owned(),
            params: params
                .iter()
                .map(|p| crate::signature_scan::types::ParamDef {
                    name: (*p).to_owned(),
                    has_default: false,
                    default_value: None,
                })
                .collect(),
            params_computed: false,
            formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                tcl_dialect::ParameterGrammar::Tcl,
            ),
            name_span: Span::new(0, 0),
            body_span: Span::new(0, 0),
            kind: "method".to_owned(),
            is_self_method: false,
            visibility: "public".to_owned(),
            doc: String::new(),
            forward_target: None,
        }
    }

    /// The pair this join sees: a computed-name creation and the
    /// `oo::define` stub that extends it, the stub having run second.
    fn creation_and_stub(creation: ClassDef, stub: ClassDef) -> (ClassDef, ClassDef) {
        (
            creation,
            ClassDef {
                via_define: true,
                ..stub
            },
        )
    }

    #[test]
    fn absorb_declarations_lets_the_later_definition_set_visibility() {
        // `export` / `unexport`
        // are one bit per member, not two sets: on 8.6.16 and 9.0.4,
        // `oo::class create E1 { method m {} {}; export m }` then
        // `oo::define E1 { unexport m }` leaves `info class methods E1`
        // empty while `-private` still lists `m`, and `[E1 new] m` raises
        // `unknown method "m"`. The reverse order re-exports it. A union
        // would leave `m` in both sets, and workspace dispatch reads an
        // `exports` entry as decisive.
        let (mut creation, stub) = creation_and_stub(
            ClassDef {
                exports: string_set(&["m"]),
                class_exports: string_set(&["cm"]),
                ..ClassDef::default()
            },
            ClassDef {
                unexports: string_set(&["m"]),
                class_unexports: string_set(&["cm"]),
                ..ClassDef::default()
            },
        );
        creation.absorb_declarations(&stub);
        assert!(
            creation.exports.is_empty() && creation.unexports == string_set(&["m"]),
            "the later `unexport m` must clear the earlier `export m`; \
             exports={:?} unexports={:?}",
            creation.exports,
            creation.unexports
        );
        assert!(
            creation.class_exports.is_empty() && creation.class_unexports == string_set(&["cm"]),
            "the class-object side takes the identical update"
        );

        // Reverse: the creation unexports, the later define re-exports.
        let (mut creation, stub) = creation_and_stub(
            ClassDef {
                unexports: string_set(&["m"]),
                ..ClassDef::default()
            },
            ClassDef {
                exports: string_set(&["m"]),
                ..ClassDef::default()
            },
        );
        creation.absorb_declarations(&stub);
        assert!(
            creation.unexports.is_empty() && creation.exports == string_set(&["m"]),
            "the later `export m` must clear the earlier `unexport m`"
        );

        // …and with the stub as the *winner* of the join, the creation it
        // absorbs ran first, so the stub's own bit stands.
        let mut stub = ClassDef {
            via_define: true,
            unexports: string_set(&["m"]),
            ..ClassDef::default()
        };
        stub.absorb_declarations(&ClassDef {
            exports: string_set(&["m", "untouched"]),
            ..ClassDef::default()
        });
        assert!(
            stub.unexports == string_set(&["m"]) && stub.exports == string_set(&["untouched"]),
            "the stub ran second whichever side won the join; \
             exports={:?} unexports={:?}",
            stub.exports,
            stub.unexports
        );
    }

    #[test]
    fn absorb_declarations_replaces_mixins_but_appends_variables_and_filters() {
        // The ordered slots are
        // *not* uniform, which is why each was measured rather than
        // assumed. On 8.6.16 and 9.0.4, a class created with
        // `mixin ::FH ::A; filter f; variable x` and then
        // `oo::define … { mixin ::FH ::B; filter g; variable y }` reports
        // mixins `::FH ::B` (replaced — `mixin -append` is the additive
        // form), filters `f g` and variables `x y` (both appended).
        let (mut creation, stub) = creation_and_stub(
            ClassDef {
                mixins: strings(&["::FH", "::A"]),
                filters: strings(&["f"]),
                class_filters: strings(&["cf"]),
                variables: strings(&["x"]),
                ..ClassDef::default()
            },
            ClassDef {
                mixins: strings(&["::FH", "::B"]),
                filters: strings(&["g"]),
                class_filters: strings(&["cg"]),
                variables: strings(&["y"]),
                ..ClassDef::default()
            },
        );
        creation.absorb_declarations(&stub);
        assert_eq!(creation.mixins, strings(&["::FH", "::B"]), "mixin replaces");
        assert_eq!(creation.filters, strings(&["f", "g"]), "filter appends");
        assert_eq!(creation.class_filters, strings(&["cf", "cg"]));
        assert_eq!(creation.variables, strings(&["x", "y"]), "variable appends");

        // Appends keep run order when the *stub* won the join, and never
        // duplicate a name the slot already carries (`variable x` twice
        // leaves one `x` on both interpreters).
        let mut stub = ClassDef {
            via_define: true,
            filters: strings(&["g"]),
            variables: strings(&["x", "y"]),
            ..ClassDef::default()
        };
        stub.absorb_declarations(&ClassDef {
            filters: strings(&["f"]),
            variables: strings(&["x"]),
            ..ClassDef::default()
        });
        assert_eq!(stub.filters, strings(&["f", "g"]), "earlier filter first");
        assert_eq!(
            stub.variables,
            strings(&["x", "y"]),
            "declared twice, kept once"
        );

        // A slot the later definition never sets keeps the earlier list.
        let (mut creation, stub) = creation_and_stub(
            ClassDef {
                mixins: strings(&["::A"]),
                ..ClassDef::default()
            },
            ClassDef::default(),
        );
        creation.absorb_declarations(&stub);
        assert_eq!(creation.mixins, strings(&["::A"]));
    }

    #[test]
    fn absorb_declarations_applies_a_later_retraction_to_the_member_table() {
        // A stub records
        // `deletemethod m` as a tombstone because it has no table to remove
        // from; the join is where the ordering is known, and compiler-side
        // consumers of `all_classes` never run the workspace fold. On
        // 8.6.16 and 9.0.4, `oo::class create D1 { method m …; method keep … }`
        // then `oo::define D1 { deletemethod m }` reports `info class
        // methods D1` = `keep`, and `[D1 new] m` raises `unknown method`.
        let (mut creation, stub) = creation_and_stub(
            ClassDef {
                methods: [
                    ("m".to_owned(), method("m", &[])),
                    ("keep".to_owned(), method("keep", &[])),
                ]
                .into_iter()
                .collect(),
                ..ClassDef::default()
            },
            ClassDef {
                retracted_members: vec![MemberRetractionRecord::deletion(
                    "m".to_owned(),
                    MemberSide::Instance,
                )],
                ..ClassDef::default()
            },
        );
        creation.absorb_declarations(&stub);
        assert_eq!(
            creation
                .methods
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["keep"],
            "the retracted member must leave the joined table"
        );
        assert_eq!(
            creation.retracted_members.len(),
            1,
            "the tombstone still rides along for the cross-file consumer"
        );

        // A move re-homes the member under its arrival name, taking the
        // destination word as its declaration site — `renamemethod m n`
        // leaves `n` alone in the table on both interpreters.
        let (mut creation, stub) = creation_and_stub(
            ClassDef {
                methods: [("m".to_owned(), method("m", &["a"]))]
                    .into_iter()
                    .collect(),
                ..ClassDef::default()
            },
            ClassDef {
                retracted_members: vec![MemberRetractionRecord {
                    member: "m".to_owned(),
                    side: MemberSide::Instance,
                    arrival: Some("n".to_owned()),
                    arrival_span: Some(Span::new(40, 41)),
                }],
                ..ClassDef::default()
            },
        );
        creation.absorb_declarations(&stub);
        let moved = creation
            .methods
            .get("n")
            .expect("the moved member arrives under its new name");
        assert!(
            !creation.methods.contains_key("m"),
            "the source name is gone"
        );
        assert_eq!(moved.params.len(), 1, "the moved member keeps its body");
        assert_eq!(moved.name, "n");
        assert_eq!(moved.name_span, Span::new(40, 41));

        // The reverse order retracts nothing: a `deletemethod` the creation
        // body itself ran already emptied its own table, and the stub's
        // members were declared afterwards.
        let mut stub = ClassDef {
            via_define: true,
            methods: [("m".to_owned(), method("m", &[]))].into_iter().collect(),
            ..ClassDef::default()
        };
        stub.absorb_declarations(&ClassDef {
            retracted_members: vec![MemberRetractionRecord::deletion(
                "m".to_owned(),
                MemberSide::Instance,
            )],
            ..ClassDef::default()
        });
        assert!(
            stub.methods.contains_key("m"),
            "an earlier retraction must not remove a later declaration"
        );
    }

    #[test]
    fn absorb_declarations_unions_the_remaining_tables() {
        // The rules with no ordering component: keyed tables union with the
        // later record winning a collision, whole-body singletons follow
        // the replace rule, and `doc` fills only where absent.
        let mut into = ClassDef {
            linked_members: [("shared".to_owned(), "::mine".to_owned())]
                .into_iter()
                .collect(),
            doc: "mine".to_owned(),
            ..ClassDef::default()
        };
        let other = ClassDef {
            linked_members: [
                ("shared".to_owned(), "::theirs".to_owned()),
                ("only-theirs".to_owned(), "::theirs".to_owned()),
            ]
            .into_iter()
            .collect(),
            doc: "theirs".to_owned(),
            ..ClassDef::default()
        };
        into.absorb_declarations(&other);
        // Neither side is a stub, so this record keeps the colliding key.
        assert_eq!(
            into.linked_members.get("shared").map(String::as_str),
            Some("::mine")
        );
        assert_eq!(
            into.linked_members.get("only-theirs").map(String::as_str),
            Some("::theirs")
        );
        assert_eq!(into.doc, "mine");

        // …and with `other` the stub, the colliding key goes to it.
        let mut into = ClassDef {
            linked_members: [("shared".to_owned(), "::mine".to_owned())]
                .into_iter()
                .collect(),
            ..ClassDef::default()
        };
        into.absorb_declarations(&ClassDef {
            via_define: true,
            ..other
        });
        assert_eq!(
            into.linked_members.get("shared").map(String::as_str),
            Some("::theirs")
        );
        // A record with no doc of its own takes the other's.
        assert_eq!(into.doc, "theirs");
    }
}

#[cfg(test)]
mod original_declaration_inventory_tests {
    use super::*;

    #[test]
    fn original_procedure_byte_slots_survive_display_collision_and_ui_removal() {
        let source = r"proc p\uD800 {} {return FIRST}
proc p\uD801 {} {return SECOND}
p\uD800
p\uD801";
        let mut result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let declarations = result.original_procedure_declarations().collect::<Vec<_>>();
        assert_eq!(declarations.len(), 2);
        assert_ne!(declarations[0].name().slot(), declarations[1].name().slot());
        assert_eq!(declarations[0].name_input().bytes(), b"p\xed\xa0\x80");
        assert_eq!(declarations[1].name_input().bytes(), b"p\xed\xa0\x81");
        let first = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
            declarations[0].name_input().clone(),
        );
        let second = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
            declarations[1].name_input().clone(),
        );
        result.all_procs.clear();
        result.superseded_procs.clear();
        let scope = result.original_namespace_scope_at(0).unwrap();
        let first = result.procedures_for_original_name(&first, scope);
        let second = result.procedures_for_original_name(&second, scope);
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert_ne!(first[0].name_span, second[0].name_span);
        let call_offsets = [
            source.rfind(r"p\uD800").unwrap(),
            source.rfind(r"p\uD801").unwrap(),
        ];
        let calls = result
            .command_invocations
            .iter()
            .filter(|invocation| call_offsets.contains(&(invocation.range.start() as usize)))
            .filter_map(|invocation| invocation.original_name_input.as_ref())
            .collect::<Vec<_>>();
        assert!(calls.iter().any(|input| input.bytes() == b"p\xed\xa0\x80"));
        assert!(calls.iter().any(|input| input.bytes() == b"p\xed\xa0\x81"));
    }

    #[test]
    fn original_scope_lookup_uses_native_namespace_and_local_priority() {
        let source = r"proc p\u0000tail {} {return ROOT}
namespace eval N {proc p\u0000tail {} {return LOCAL}; p\u0000tail}";
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let call = result
            .command_invocations
            .iter()
            .find(|invocation| {
                invocation.range.start()
                    == u32::try_from(source.rfind(r"p\u0000tail").unwrap()).unwrap()
            })
            .unwrap();
        let input = call.original_name_input.as_ref().unwrap();
        assert_eq!(input.bytes(), b"p\xc0\x80tail");
        let matches = result.procedures_for_original_name(
            input,
            result
                .original_namespace_scope_at(call.range.start())
                .unwrap(),
        );
        assert_eq!(matches.len(), 1);
        let local_declaration = result
            .original_procedure_declarations()
            .find(|declaration| {
                declaration.declaration_site().offset
                    == u32::try_from(source.rfind(r"proc p\u0000tail").unwrap()).unwrap()
            })
            .unwrap();
        assert_eq!(matches[0].name_span, local_declaration.metadata().name_span);
    }

    #[test]
    fn invocation_transports_opaque_original_lookup_without_reporting_name() {
        let source = r"missing\uD800";
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let invocation = result
            .command_invocations
            .iter()
            .find(|invocation| invocation.range.start() == 0)
            .unwrap();
        let input = invocation.original_name_input.as_ref().unwrap();
        let lookup = invocation.original_lookup.as_ref().unwrap();
        assert_eq!(input.bytes(), b"missing\xed\xa0\x80");
        assert_eq!(lookup.name_input(), input);
        assert_eq!(lookup.site().offset, 0);
        assert_eq!(lookup.candidates()[0][0].simple.as_bytes(), input.bytes());
        assert!(invocation.resolved_definition.is_none());
    }

    #[test]
    fn computed_head_retains_readonly_input_and_positioned_lookup() {
        let source = "proc p {} {}; set cmd p; $cmd";
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.rfind("$cmd").unwrap()).unwrap();
        let invocation = result
            .command_invocations
            .iter()
            .find(|invocation| invocation.range.start() == offset && invocation.indirect)
            .unwrap();
        let input = invocation.original_name_input.as_ref().unwrap();
        assert!(input.original_word_key().is_none());
        assert_eq!(input.bytes(), b"p");
        let lookup = invocation.original_lookup.as_ref().unwrap();
        assert_eq!(lookup.name_input(), input);
        assert_eq!(lookup.site().offset, offset);
        assert_eq!(lookup.candidates()[0][0].simple.as_bytes(), b"p");
    }

    #[test]
    fn advisory_invocation_and_unknown_body_scope_cannot_issue_source_names() {
        let invocation =
            SignatureCommandInvocation::written("p".to_owned(), Span::new(0, 1), Some(0));
        assert!(invocation.original_name_input.is_none());
        let mut result = crate::analyser::Analyser::new().analyse("set x 1", "tcl8.6");
        let mut child = Scope::new(ScopeKind::Method, "opaque".to_owned());
        child.body_span = Some(Span::new(0, 7));
        result.global_scope.children.push(child);
        assert!(result.original_namespace_scope_at(2).is_none());
    }
}

#[cfg(test)]
mod original_definition_metadata_tests {
    #[test]
    fn positioned_procedure_reference_navigates_without_reported_maps() {
        let source = "proc p {} {return BODY}\np";
        let mut result = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
        let definition = result
            .command_invocations
            .iter()
            .find(|invocation| {
                invocation.range.start() == u32::try_from(source.rfind('p').unwrap()).unwrap()
            })
            .unwrap()
            .resolved_definition
            .clone()
            .unwrap();
        let expected = result
            .original_procedure_declarations()
            .next()
            .unwrap()
            .metadata()
            .name_span;
        result.all_procs.clear();
        result.superseded_procs.clear();
        assert_eq!(
            result
                .proc_for_definition(&definition, source)
                .unwrap()
                .name_span,
            expected
        );
        assert!(
            result
                .proc_for_definition(&definition, "proc p {} {return OTHER}\np")
                .is_none()
        );
    }
}

#[cfg(test)]
mod original_completed_world_transport_tests {
    #[test]
    fn completed_world_survives_ui_clear_and_retains_moved_implementation() {
        let source = "proc P {} {}; rename P Q";
        let mut result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let declaration = result
            .original_procedure_declarations()
            .next()
            .unwrap()
            .clone();
        let policy = declaration.name().policy();
        let former_slot = declaration.name().slot().clone();
        result.all_procs.clear();
        result.superseded_procs.clear();
        let world = result
            .original_completed_command_world()
            .expect("independently completed Normal root");
        assert_eq!(world.source_image().bytes(), source.as_bytes());
        assert!(world.declaration_at(&former_slot, policy).is_none());
        let moved = world
            .declarations()
            .find(|publication| publication.slot().simple.as_bytes() == b"Q")
            .unwrap();
        assert_eq!(moved.declaration_site(), declaration.declaration_site());
        assert!(moved.definition().is_some());
    }

    #[test]
    fn abrupt_or_unknown_roots_cannot_transport_completed_world() {
        for source in [
            "proc P {} {}; error BOOM",
            "proc P {} {}; eval $unknown",
            "proc P {} {}; rename $unknown Q",
        ] {
            let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            assert!(
                result.original_completed_command_world().is_none(),
                "{source}"
            );
        }
        assert!(
            super::AnalysisResult::default()
                .original_completed_command_world()
                .is_none()
        );
    }
}

#[cfg(test)]
mod original_namespace_operand_tests {
    use crate::analyser::Analyser;

    #[test]
    fn original_namespace_names_keep_surrogate_and_nul_units_in_their_receipts() {
        let source =
            r"namespace eval n\uD800 {}; namespace eval n\uD801 {}; namespace eval n\u0000tail {}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let declarations = analysis
            .namespace_refs
            .iter()
            .filter(|reference| reference.declares)
            .collect::<Vec<_>>();
        assert_eq!(declarations.len(), 3);
        let names = declarations
            .iter()
            .map(|reference| {
                reference
                    .original_name_input
                    .as_ref()
                    .unwrap()
                    .bytes()
                    .to_vec()
            })
            .collect::<Vec<_>>();
        assert_eq!(names[0], b"n\xed\xa0\x80");
        assert_eq!(names[1], b"n\xed\xa0\x81");
        assert_eq!(names[2], b"n\xc0\x80tail");
        assert_ne!(
            declarations[0].source_namespace,
            declarations[1].source_namespace
        );
        assert!(
            declarations
                .iter()
                .all(|reference| reference.source_namespace.is_some())
        );
    }

    #[test]
    fn computed_namespace_values_have_identity_without_word_edit_geometry() {
        let source = r"set target ::n; namespace eval $target {}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("$target").unwrap()).unwrap();
        let references = analysis
            .namespace_refs
            .iter()
            .filter(|reference| reference.span.start() == offset)
            .collect::<Vec<_>>();
        assert_eq!(references.len(), 1, "one authentic namespace occurrence");
        let reference = references[0];
        let input = reference
            .original_name_input
            .as_ref()
            .expect("retained evaluated bytes");
        assert_eq!(input.bytes(), b"::n");
        assert!(input.original_word_key().is_none());
        assert!(reference.source_namespace.is_some());
        assert!(
            reference
                .original_extent_span(source, reference.source_namespace.as_ref().unwrap(), false)
                .is_none()
        );
        let child = analysis
            .global_scope
            .children
            .iter()
            .find(|scope| scope.name_span == Some(reference.span))
            .unwrap();
        assert_eq!(child.naming_scope, reference.source_namespace);
    }

    #[test]
    fn namespace_path_values_retain_parent_list_lineage_without_word_geometry() {
        let source = r"namespace eval n {}; namespace path {::n ::n\u0000tail}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let values = analysis
            .namespace_refs
            .iter()
            .filter(|reference| !reference.declares)
            .collect::<Vec<_>>();
        assert_eq!(values.len(), 2);
        let second = values[1].original_name_input.as_ref().unwrap();
        assert_eq!(second.bytes(), b"::n\xc0\x80tail");
        assert!(second.original_word_key().is_none());
        assert!(
            values
                .iter()
                .all(|reference| reference.source_namespace.is_some())
        );
    }
}

#[cfg(test)]
mod original_namespace_pattern_tests {
    use crate::analyser::Analyser;

    #[test]
    fn original_namespace_patterns_retain_opaque_source_and_tail_without_ui_rows() {
        let source = r"namespace import ::n\uD800::*; namespace forget ::n\uD801::p\uD802";
        let mut result = Analyser::new().analyse(source, "tcl8.6");
        let patterns = result
            .original_namespace_patterns()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].parts().tail.as_bytes(), b"*");
        assert_eq!(patterns[1].parts().tail.as_bytes(), b"p\xed\xa0\x82");
        assert_ne!(patterns[0].parts().source, patterns[1].parts().source);
        assert_eq!(
            patterns[0].purpose(),
            tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern
        );
        assert_eq!(
            patterns[1].purpose(),
            tcl_syntax::naming::NativeNamePurpose::NamespaceForgetPattern
        );
        assert_eq!(
            patterns[0]
                .name_input()
                .original_word_key()
                .unwrap()
                .source_image(),
            &tcl_lexer::SourceImage::document(source)
        );
        result.namespace_imports.clear();
        result.namespace_forgets.clear();
        assert_eq!(
            result
                .original_namespace_patterns()
                .cloned()
                .collect::<Vec<_>>(),
            patterns
        );
    }
}

#[cfg(test)]
mod original_namespace_unknown_tests {
    #[test]
    fn original_namespace_unknown_operands_remain_coverage_obligations() {
        let source = "namespace eval ::old {}\nproc mk {ns} {namespace eval $ns {}; namespace eval ::other::$ns {}}";
        let mut analyser = crate::analyser::Analyser::new();
        let analysis = analyser.analyse(source, "tcl8.6");
        let spellings = analysis
            .namespace_name_unknowns
            .iter()
            .map(|span| &source[span.as_range()])
            .collect::<Vec<_>>();
        assert_eq!(spellings, ["$ns", "::other::$ns"]);
        assert!(
            analysis
                .namespace_refs
                .iter()
                .all(|row| row.original_name != "$ns")
        );
    }
}

#[cfg(test)]
mod original_namespace_export_inventory_tests {
    use crate::analyser::Analyser;

    #[test]
    fn original_namespace_export_inventory_keeps_opaque_patterns_clear_and_prefix_abort() {
        // Implementation contract: naming.namespace.original-export-source-advice
        // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = r"namespace export {p\uD800}; namespace export -clear p\uD801; namespace export -clear -clear; namespace export first ::invalid never";
            let mut result = Analyser::new().analyse(source, version.dialect_name());
            let events = result
                .original_namespace_exports()
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(
                events.iter().filter(|event| event.clears()).count(),
                2,
                "{version:?}"
            );
            assert!(
                events
                    .iter()
                    .any(|event| event.pattern() == Some(b"p\\uD800".as_slice())),
                "braced word keeps its literal backslash"
            );
            assert!(
                events
                    .iter()
                    .any(|event| event.pattern() == Some(b"p\xed\xa0\x81".as_slice())),
                "{version:?}"
            );
            assert!(
                events
                    .iter()
                    .any(|event| event.pattern() == Some(b"-clear".as_slice()))
            );
            assert!(
                events
                    .iter()
                    .any(|event| event.pattern() == Some(b"first".as_slice()))
            );
            assert!(
                !events
                    .iter()
                    .any(|event| event.pattern() == Some(b"never".as_slice()))
            );
            assert!(events.iter().all(|event| event.context().is_root()
                && event.site().source.source_image()
                    == &tcl_lexer::SourceImage::document(source)));
            result.namespace_exports.clear();
            assert_eq!(
                result
                    .original_namespace_exports()
                    .cloned()
                    .collect::<Vec<_>>(),
                events
            );
        }
    }

    #[test]
    fn original_namespace_export_inventory_retains_unknown_operand_as_may_only() {
        // Implementation contract: naming.namespace.original-export-source-advice
        // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
        let source = "namespace export $unavailable";
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert!(result.original_namespace_exports().next().is_none());
        assert_eq!(result.original_namespace_export_unknowns().len(), 1);
        let span = result.original_namespace_export_unknowns()[0];
        assert!(span.start() >= u32::try_from(source.find('$').unwrap()).unwrap());
    }

    #[test]
    fn original_namespace_inventory_keeps_conditional_source_scopes_after_unknown_import() {
        // Implementation contract: naming.namespace.original-export-source-advice
        // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
        let source = "unavailable_provider\nnamespace eval C {proc p {} {}; namespace export p}\nnamespace eval B {namespace import ::C::*; namespace export p}\nnamespace eval A {namespace import ::B::*}\n";
        let result = Analyser::new().analyse(source, "tcl8.6");
        let exports = result.original_namespace_exports().collect::<Vec<_>>();
        let imports = result.original_namespace_patterns().collect::<Vec<_>>();
        assert_eq!(exports.len(), 2);
        assert_eq!(imports.len(), 2);
        assert!(result.original_namespace_export_unknowns().is_empty());
        assert!(
            exports
                .iter()
                .any(|event| event.conditional_declaration().is_some())
        );
        assert!(
            imports
                .iter()
                .any(|event| event.conditional_declaration().is_some())
        );
        for event in &exports {
            assert_eq!(
                event
                    .name_input()
                    .original_word_key()
                    .unwrap()
                    .source_image(),
                &tcl_lexer::SourceImage::document(source)
            );
        }
        assert_eq!(
            exports
                .iter()
                .map(|event| event.context().display().unwrap())
                .collect::<Vec<_>>(),
            ["::C", "::B"]
        );
        assert_eq!(
            imports
                .iter()
                .map(|event| event.context().display().unwrap())
                .collect::<Vec<_>>(),
            ["::B", "::A"]
        );
        assert!(
            result.original_completed_command_world().is_none(),
            "source declaration geometry cannot close the actual import prelude or source mode"
        );
    }

    #[test]
    fn original_namespace_inventory_does_not_borrow_a_parent_for_unknown_source_scope() {
        // Implementation contract: naming.namespace.original-export-source-advice
        // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
        let source = "namespace eval $unknown {namespace export p; namespace import ::C::*}";
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert!(result.original_namespace_exports().next().is_none());
        assert!(result.original_namespace_patterns().next().is_none());
        assert!(!result.original_namespace_export_unknowns().is_empty());
        assert!(result.original_completed_command_world().is_none());
    }

    #[test]
    fn original_namespace_inventory_does_not_extend_static_scope_through_an_unentered_callable() {
        // Implementation contract: naming.namespace.original-export-source-advice
        // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
        let source =
            "proc unused {} {namespace eval B {namespace export p; namespace import ::C::*}}";
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            result
                .original_namespace_exports()
                .all(|event| event.conditional_declaration().is_none())
        );
        assert!(
            result
                .original_namespace_patterns()
                .all(|event| event.conditional_declaration().is_none())
        );
    }
}

#[cfg(test)]
mod lexical_source_advice_domain_tests;

#[cfg(test)]
mod original_source_context_tests;
