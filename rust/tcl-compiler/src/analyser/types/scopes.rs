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

//! The scope tree: `ScopeKind`, `Scope` and the `VarDef` records it holds.

use super::class_def::ClassDef;
use super::original_member_context::OriginalLexicalMemberContext;
use super::original_receiver_body::OriginalSourceReceiverBodyDeclaration;
use super::procedures::{DefinedSymbol, ProcDef};
use std::collections::HashMap;
use tcl_lexer::Span;

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

/// Variable definition record.
///
/// Populated by [`Analyser`](super::super::Analyser) every time it
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

/// Whether this body belongs to the surrounding interpreter's source walk.
/// This is independent of lambda context and `TclOO` lookup/callability axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum InterpreterSourceBody {
    Enclosing,
    Independent,
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
        Option<std::sync::Arc<super::super::OriginalInterpreterSourceDomain>>,
    /// Conditional lambda namespace source owner. Whole input, fields and
    /// applicability remain retained independently of this scope's report name.
    /// This supplies no lambda activation, formal binding or physical frame.
    pub(in super::super) original_logical_lambda_namespace:
        Option<std::sync::Arc<super::super::interp_visibility::OriginalInterpreterLambdaBody>>,
    /// An actual lambda body cannot borrow an enclosing source frame when
    /// its own call/formal/source context is unavailable.
    pub(in super::super) requires_original_lambda_source_context: bool,
    /// An independently walked interpreter script has its own source root.
    pub(in super::super) interpreter_source_body: InterpreterSourceBody,
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
    /// Producer-owned first declaration extents, coupled to actual variable
    /// map entries. These storage keys supply no name or scope interpretation.
    pub(in super::super) original_variable_declaration_spans: HashMap<String, Span>,
    pub(crate) readonly_dictionary_keys:
        HashMap<String, super::super::scope::ReadonlyDictionaryKeyVariable>,
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
    pub(in super::super) const fn is_independent_interpreter_source_body(&self) -> bool {
        matches!(
            self.interpreter_source_body,
            InterpreterSourceBody::Independent
        )
    }

    /// Construct a fresh empty scope.
    #[must_use]
    pub fn new(kind: ScopeKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            original_interpreter_source_domain: None,
            original_logical_lambda_namespace: None,
            requires_original_lambda_source_context: false,
            interpreter_source_body: InterpreterSourceBody::Enclosing,
            naming_scope: None,
            original_member_context: None,
            original_receiver_body_declaration: None,
            name: name.into(),
            body_span: None,
            name_span: None,
            variables: HashMap::new(),
            original_variable_declaration_spans: HashMap::new(),
            readonly_dictionary_keys: HashMap::new(),
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

impl Default for Scope {
    fn default() -> Self {
        Self::global()
    }
}
