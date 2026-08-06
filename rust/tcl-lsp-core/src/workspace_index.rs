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

//! Workspace index — a cross-document symbol aggregate.
//!
//! The per-document providers (definition, references, rename,
//! completion, code-lens) answer queries against a single
//! document's [`AnalysisResult`].  The workspace index lifts
//! the proc / class *definitions* of every analysed document
//! into one searchable structure so cross-document features
//! can resolve a symbol that lives in a sibling file.
//!
//! The server owns one index, rebuilt (or incrementally
//! updated) as documents open / change / close from its cached
//! `AnalysisResult` map.  The index stores owned data (so it
//! can move into a `spawn_blocking` worker) and keeps the byte
//! [`Span`] of each definition; converting a span to an LSP
//! range needs the *target* document's source, which the
//! server resolves at query time.
//!
//! This is the foundation for:
//!
//! * workspace-wide proc enumeration in completion;
//! * cross-document go-to-definition;
//! * cross-document references / rename / call-hierarchy
//!   (these consume the per-document *invocation* sites the
//!   index also records).
//!
//! The server seeds the index from both editor-opened documents
//! (via the diagnostics path) and an on-disk scan of the
//! workspace folders on `initialized`, so unopened `.tcl` / `.tm`
//! files are covered too.
//!
//! Procs, classes, and command invocations are indexed in full.
//! Variables are indexed **only in their namespace-qualified form**
//! ([`WorkspaceVariable`] / [`WorkspaceVariableRef`]): a namespace- or
//! global-scope `variable` / `set` declaration, and every occurrence
//! written with a `::` qualifier.  That is the same bound proc / class
//! indexing already has — one cell, one namespace, one name, whatever
//! file it is written in — and it is what makes `$::ns::v` resolve to a
//! `namespace eval ns { variable v }` in a sibling document (issue #923
//! differential-audit findings idx 65 / 75 / 78).  An **unqualified**
//! `$v` names whichever cell the local scope chain supplies, which is a
//! per-document question with no statically-sound cross-file answer, so
//! proc locals and bare occurrences are still not indexed.
//!
//! **Namespaces** are indexed as first-class symbols too
//! ([`WorkspaceNamespaceRef`], issue #1088): every word the registry marks
//! [`tcl_registry::ArgRole::NamespaceName`] — the declaring `namespace eval`
//! name token and every other spelling (`namespace children ::tomato`,
//! `namespace exists ns`, `namespace delete ::a`, `namespace upvar ns v l`).
//! This tier needs no qualified-only bound the way variables do: the analyser
//! roots a relative namespace word against its own lexical namespace before
//! recording it, so every indexed row names one namespace absolutely.  A
//! **computed** target (`namespace eval $ns { … }`) is recorded nowhere — it
//! names no static namespace — and a namespace brought into being only as an
//! implicit parent (`namespace eval ::p::q::r {}` creates `::p` and `::p::q`
//! on both interpreters) has no declaring row of its own, because its name is
//! not written anywhere.

use std::collections::HashSet;
use std::sync::Arc;

use crate::namespace_import::ExportVerdict;
use crate::source_graph::RunPoint;
use crate::workspace_symbols::{
    IndexedWorkspaceSymbol, WorkspaceSymbolKind, matches_query, namespace_of,
};
use tcl_compiler::analyser::class_hierarchy::{build_tail_index, resolve_class_name};
use tcl_compiler::analyser::{AnalysisResult, MemberRetractionRecord, MemberSide};
use tcl_lexer::Span;

/// One proc definition recorded in the workspace index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceProc {
    /// Document the proc is defined in (the `analyses` map key).
    pub uri: String,
    /// Simple (tail) name, e.g. `greet`.
    pub name: String,
    /// Fully-qualified name, e.g. `::myns::greet`.
    pub qualified_name: String,
    /// Declared parameter count (for completion detail).
    pub param_count: usize,
    /// Byte span of the proc's name token in `uri`'s source.
    /// The server resolves this to an LSP range against the
    /// target document at query time.
    pub name_span: Span,
    /// Whether this proc's own declaration sits inside another proc's or
    /// class's body — i.e. it exists only conditionally, when and if that
    /// enclosing definition actually runs (the "rename a builtin away,
    /// install a same-named shadow proc, restore it" idiom). A nested
    /// definition must not permanently outrank a real registry builtin for
    /// workspace-wide command existence, mirroring the same judgement
    /// `tcl_compiler::analyser::AnalysisResult::offset_is_inside_any_definition_body`
    /// already applies same-file in `resolve_called_proc`.
    pub nested: bool,
}

/// One class definition recorded in the workspace index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceClass {
    /// Document the class is defined in.
    pub uri: String,
    /// Simple (tail) name.
    pub name: String,
    /// Fully-qualified name.
    pub qualified_name: String,
    /// Byte span of the class's name token.
    pub name_span: Span,
    /// Declared superclass names (as written), for cross-file type
    /// hierarchy (subtype resolution).
    pub superclasses: Vec<String>,
    /// Declared class-level mixin names (as written).
    pub mixins: Vec<String>,
    /// The methods this record *directly defines* — the typed method table
    /// (issue #945 fault 4): each entry carries its receiver kind and its
    /// **effective export state** at the end of this record's body, so
    /// cross-file dispatch can honour `TclOO` visibility instead of treating
    /// every name as callable.  Spans aren't stored — the server
    /// re-analyses each family member's document to collect the precise
    /// decl / call sites.
    pub methods: Vec<WorkspaceMethod>,
    /// Methods this record explicitly `export`s **on the instance side** (an
    /// `oo::define` extension stub can flip visibility on a class defined
    /// elsewhere).
    pub exports: Vec<String>,
    /// Methods this record explicitly `unexport`s on the instance side.
    pub unexports: Vec<String>,
    /// Members this record explicitly `self export`s — the **class-object**
    /// side's counterpart of [`Self::exports`], from
    /// [`tcl_compiler::analyser::ClassDef::class_exports`].
    ///
    /// Its own channel because the instance-side pair is the instance-side
    /// record by contract: a `self unexport m` folded into `exports`/`unexports`
    /// would flip an identically-named *instance* method the wrapper never
    /// touched (issue #1098).  Without a channel of its own the flip did not
    /// travel at all, so a `self unexport m` in `a.tcl` left `b.tcl`'s
    /// class-command dispatch advertising a member `::C m` rejects with
    /// `unknown method "m"` (issue #1119).
    ///
    /// Read by [`WorkspaceIndex::class_method_dispatch_chain`] under the same
    /// union rule, and carrying the same unordered-cross-file caveat, as the
    /// instance-side pair and [`Self::retracted_members`]: true load order is
    /// not knowable from the index, so any exporting record keeps the member
    /// dispatchable.
    pub class_exports: Vec<String>,
    /// Members this record explicitly `self unexport`s — see
    /// [`Self::class_exports`].
    pub class_unexports: Vec<String>,
    /// Members this record **retracts** (`deletemethod` / `renamemethod`)
    /// without declaring them itself — the cross-document tombstones of
    /// [`tcl_compiler::analyser::ClassDef::retracted_members`].
    ///
    /// A `via_define` stub in another file has no local method table to remove
    /// from, so without these the workspace keeps advertising a method that
    /// sourcing the extension deletes (issue #1101 review). Applied as an
    /// unordered fact, the mirror of the way a cross-file `oo::define ::C {
    /// method extra … }` is an unordered addition: cross-file load order is not
    /// knowable from the index, and a retraction of a member the *same*
    /// document declares never becomes a tombstone in the first place.
    pub retracted_members: Vec<MemberRetractionRecord>,
    /// `true` when this record is a cross-file `oo::define` extension stub
    /// rather than the class's own `oo::class create` site (see
    /// [`tcl_compiler::analyser::ClassDef::via_define`]).  Go-to-definition
    /// prefers a real creation site over a stub.
    pub via_define: bool,
    /// The definer command as written (`"oo::class"`, `"snit::type"`,
    /// `"itcl::class"`, …) — see
    /// [`tcl_compiler::analyser::ClassDef::metaclass`].  Lets a cross-file
    /// consumer tell [incr Tcl]'s class-scoped `proc` (dispatched as a
    /// single `::`-qualified identifier) apart from a `TclOO`
    /// `classmethod` / snit `typemethod` (dispatched as two bare words)
    /// without a local `ClassDef` to ask.
    pub metaclass: String,
    /// Byte spans of this record's `constructor` name tokens, in declaration
    /// order (`oo::configurable` admits more than one).  Constructors are not
    /// dispatchable members, so they stay out of [`Self::methods`]; they are
    /// carried only so `workspace/symbol` can offer them from an unopened file
    /// the way the per-document outline does (issue #1156).
    pub constructor_spans: Vec<Span>,
}

impl WorkspaceClass {
    /// Whether this record's definer dispatches its class-scoped members as
    /// a single `::`-qualified identifier (`Factory::make`) rather than the
    /// two-word `Factory make` shape — true for [incr Tcl] only.  Registry
    /// data (`DefinerFamily`), not a hardcoded command-name check.
    #[must_use]
    pub fn is_itcl(&self, dialect: &str) -> bool {
        tcl_registry::registry_for_dialect(dialect)
            .get(&self.metaclass)
            .and_then(|spec| spec.definition_body)
            .is_some_and(|g| g.family == tcl_registry::definer::DefinerFamily::Itcl)
    }

    /// Whether this record directly defines `name` (any receiver kind).
    #[must_use]
    pub fn defines_method(&self, name: &str) -> bool {
        self.methods.iter().any(|m| m.name == name)
    }

    /// Whether a `renamemethod` recorded on this record makes `name` a member
    /// of the class — the **arrival** half of the tombstone channel (issue
    /// #1167).
    ///
    /// A cross-file `oo::define ::C { renamemethod old new }` declares no
    /// member of its own (the params / body / visibility stay in the defining
    /// file's record), so `defines_method` is `false` for `new` on every
    /// record of the class.  Without this the member simply disappears at the
    /// workspace tier: `old` is correctly tombstoned and `new` is nowhere.
    #[must_use]
    pub fn arrives_method(&self, name: &str) -> bool {
        self.retracted_members
            .iter()
            .any(|r| r.arrival.as_deref() == Some(name))
    }

    /// The member name that arrives as `name` on `side`, per a `renamemethod`
    /// recorded on this record — the source whose `MethodDef` the workspace
    /// join re-keys.
    #[must_use]
    pub fn arrival_source(&self, name: &str, side: MemberSide) -> Option<&str> {
        self.retracted_members
            .iter()
            .find(|r| r.arrival.as_deref() == Some(name) && r.side == side)
            .map(|r| r.member.as_str())
    }

    /// The typed record for the *instance-receiver* method `name`
    /// (an ordinary `method` or a `forward`), if this record defines one.
    #[must_use]
    pub fn instance_method(&self, name: &str) -> Option<&WorkspaceMethod> {
        self.methods
            .iter()
            .find(|m| m.name == name && m.kind != "classmethod")
    }

    /// The typed record for the *class-receiver* member `name` — a
    /// `classmethod` / `self method` / snit `typemethod` — if this record
    /// defines one.  The counterpart of [`Self::instance_method`].
    #[must_use]
    pub fn class_method(&self, name: &str) -> Option<&WorkspaceMethod> {
        self.methods
            .iter()
            .find(|m| m.name == name && m.kind == "classmethod")
    }
}

/// Which receiver side a [`WorkspaceMethod`] is declared on — the same split
/// [`WorkspaceClass::instance_method`] / [`WorkspaceClass::class_method`] make,
/// named once so the member fold and the tombstone lookup agree on it.
#[must_use]
fn method_side(m: &WorkspaceMethod) -> MemberSide {
    if m.kind == "classmethod" {
        MemberSide::ClassObject
    } else {
        MemberSide::Instance
    }
}

/// One member of a class as the **workspace** sees it: the declaring record's
/// entry, keyed under the name the class actually dispatches it by once every
/// cross-document retraction and arrival has been applied (issue #1263).
///
/// The raw [`WorkspaceClass::methods`] table is per-record and additive — it
/// records what that record's own body declared and nothing else — so a member
/// moved by a cross-file `oo::define ::C { renamemethod old new }` is still
/// listed as `old` on the defining record and not listed at all on the stub.
/// Resolving one name against the table already joined the two halves
/// ([`WorkspaceIndex::dispatch_chain`], issue #1167); *listing* the table did
/// not, so anything that enumerates a class's members (`workspace/symbol`, an
/// outline, a member completion universe) showed the pre-rename name.
///
/// [`WorkspaceIndex::effective_members`] is the one place that fold happens,
/// and the dispatch chain reads it too, so a single rule decides what a class's
/// member set is.
#[derive(Debug, Clone, Copy)]
pub struct EffectiveMember<'a> {
    /// The name the class dispatches this member under.
    pub name: &'a str,
    /// The record whose body, parameters and visibility define the member —
    /// where a `renamemethod`'s *source* was declared, not where the rename is
    /// written.
    pub declaring: &'a WorkspaceClass,
    /// The declaring record's own entry, still keyed under its declared
    /// spelling.  Visibility travels with the body, so this is what a
    /// visibility test must read.
    pub method: &'a WorkspaceMethod,
    /// Document holding the token that spells [`Self::name`] — the declaring
    /// record's document normally, the retracting stub's when the member
    /// arrived through a cross-file `renamemethod`.
    pub name_uri: &'a str,
    /// Byte span of that token, in [`Self::name_uri`]'s source.
    pub name_span: Span,
}

/// Every cross-document member retraction in the workspace, keyed by the
/// `(class qualified name, member name, side)` it removes and valued with the
/// record that wrote it plus the retraction itself.  See
/// [`WorkspaceIndex::retraction_index`].
type RetractionIndex<'a> = std::collections::HashMap<
    (&'a str, &'a str, MemberSide),
    (&'a WorkspaceClass, &'a MemberRetractionRecord),
>;

/// Apply the workspace's retraction / arrival fold to one member `declaring`
/// declares, yielding the name the class dispatches it under — or `None` when
/// a cross-document `deletemethod` removed it outright (issue #1263).
///
/// The one place that decision is made; [`WorkspaceIndex::effective_members`]
/// (and through it [`WorkspaceIndex::dispatch_chain`]) and the
/// `workspace/symbol` member walk all route through it, so resolving a name
/// and listing the member set can no longer disagree.
///
/// tclsh-proof (8.6.14), for the two arms:
///
/// ```tcl
/// # a.tcl: oo::class create ::C { method old {} { return OLDBODY } }
/// # b.tcl: oo::define ::C { renamemethod old new }
/// info class methods ::C     ;# -> new         (arrival re-keys the member)
/// [::C new] old              ;# -> unknown method "old"
/// # with `deletemethod old` instead:
/// info class methods ::C     ;# -> (empty)     (the member is gone)
/// ```
fn effective_member<'a>(
    retractions: &RetractionIndex<'a>,
    declaring: &'a WorkspaceClass,
    method: &'a WorkspaceMethod,
) -> Option<EffectiveMember<'a>> {
    let side = method_side(method);
    // A `deletemethod` / `renamemethod` in *another* document's `oo::define`
    // stub really removes the member (issue #1101 review). The union is taken
    // per class — a subclass cannot retract an inherited member (real Tcl:
    // `method … does not exist`) — and the tombstone carries the side it
    // removed from, so a `self deletemethod m` never touches the
    // instance-side member.
    let Some((stub, retraction)) = retractions.get(&(
        declaring.qualified_name.as_str(),
        method.name.as_str(),
        side,
    )) else {
        return Some(EffectiveMember {
            name: &method.name,
            declaring,
            method,
            name_uri: &declaring.uri,
            name_span: method.name_span,
        });
    };
    // A `renamemethod old new` *moves* the member: the stub owns no
    // `MethodDef` (the params / body / visibility stay on the declaring
    // record), so the arrival re-keys this entry and the arrival word is the
    // moved member's declaration site.  A plain `deletemethod` — and a
    // `renamemethod old $new` whose destination is computed, which names
    // nothing statically — records no arrival, and the member is simply gone.
    Some(EffectiveMember {
        name: retraction.arrival.as_deref()?,
        declaring,
        method,
        name_uri: &stub.uri,
        name_span: retraction.arrival_span?,
    })
}

/// One method a class record directly defines, as indexed for cross-file
/// dispatch (issue #945 faults 4 and 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMethod {
    /// Simple method name.
    pub name: String,
    /// Declaration kind: `"method"`, `"classmethod"`, or `"forward"`.
    pub kind: String,
    /// Effective `TclOO` export state at the end of the defining record's
    /// body: the family's name default (`[a-z]*` for `TclOO`) plus any
    /// explicit `export` / `unexport`, last writer wins (tclsh
    /// 9.0.4-pinned).  Externally callable iff `true`.
    pub exported: bool,
    /// `true` for a `TclOO` `private` definition — invisible to external
    /// dispatch *and* to subclasses; callable only via `my` within the
    /// declaring class's own methods.
    pub private: bool,
    /// `true` for a stock-`TclOO` `self method` (as opposed to `ooutil`'s
    /// `classmethod` keyword).  Both land in the class-receiver bucket
    /// (`kind == "classmethod"`), but a `self method` is visible **only**
    /// on the exact class object that declared it: `Gadget make` on a
    /// subclass of a class declaring `self method make` errors `unknown
    /// method "make"` under tclsh 8.6 and 9.0.4, whereas `classmethod`
    /// propagates to the subclass's own bound command through its
    /// `Delegate`-mixin machinery.  The single-document scan reads
    /// [`tcl_compiler::analyser::MethodDef::is_self_method`] for this;
    /// carrying it here is what lets a *cross-file* consumer scan tell the
    /// two apart (Codex review on #1047).
    pub is_self_method: bool,
    /// Byte span of the method's name token in the declaring record's
    /// document.  Carried so `workspace/symbol` can locate a method in a file
    /// the editor has never opened (issue #1156); cross-file *dispatch* uses
    /// the name and the visibility flags and does not need it.
    pub name_span: Span,
}

/// The access context of a method call site — `TclOO` dispatches an
/// external `$obj m` through exported methods only, while an internal
/// `my m` reaches unexported ones too (issue #945 fault 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodAccess {
    /// `$obj m` / `objcmd m` — exported methods only.
    External,
    /// `my m` (or a declaration-side query from inside the class body) —
    /// exported and unexported methods; `private` methods only from the
    /// declaring class itself.
    Internal,
}

/// One command-invocation (call) site recorded in the index.
///
/// Tagged with the defining document so cross-document references
/// / rename / call-hierarchy can walk every call site of a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceInvocation {
    /// Document the call site is in.
    pub uri: String,
    /// Command head as written at the call site (no namespace
    /// resolution).
    pub name: String,
    /// The full ordered command-resolution candidate list for this call
    /// (caller namespace, then each `namespace path` entry, then global — Tcl's
    /// real priority order).  Run through the workspace-wide existence oracle to
    /// settle which definition the call names, wherever it lives — see
    /// [`WorkspaceIndex::invocations_of`].
    pub resolution_candidates: Vec<String>,
    /// Byte span of the command-head token in `uri`'s source.
    pub range: Span,
    /// The span does not carry the written command name (an indirect site —
    /// a constant `$cmd` head, M7): references may report it, but the
    /// cross-document rename path must not rewrite it.
    pub indirect: bool,
    /// `false` when renaming this invocation's resolved command cannot be
    /// completed soundly from source edits (an indirect site with at least
    /// one contributing constant that has no exact writable source span) —
    /// the rename providers must abstain for the whole symbol rather than
    /// leave the site dispatching the old name (issue #945 fault 1).
    pub rename_safe: bool,
    /// `Some(provenance)` when this site is the **subcommand word** of an
    /// `<ensemble> <sub> …` dispatch, mirroring
    /// [`tcl_compiler::signature_scan::types::SignatureCommandInvocation::ensemble_dispatch`]
    /// (issue #1281) so the cross-document rename path applies the same gate
    /// the in-document one does: a `-map` key is an arbitrary name, so it
    /// must survive a rename of its target unchanged; a `-subcommands` entry
    /// is the target's tail and must follow it.  References report the site
    /// either way.
    pub ensemble_dispatch:
        Option<tcl_compiler::signature_scan::types::EnsembleSubcommandProvenance>,
    /// Span of the innermost proc/class **body** containing this call site,
    /// within [`Self::uri`]; `None` when the call sits at load level.
    ///
    /// The call-side twin of [`WorkspaceGlobImport::enclosing_body`], and for
    /// the same reason: ordering an import edge's events against a call is
    /// [`tcl_compiler::analyser::indirection::in_effect_within`], not an
    /// offset compare, because the whole file loads before any body runs.
    /// Carried per row so [`WorkspaceIndex::invocation_resolves_to`] — which
    /// has no calling-document `AnalysisResult` in hand — can build a complete
    /// [`CallSite`] (issue #1116 item 3).
    ///
    /// The naive per-row lookup would be `O(procs × invocations)`;
    /// [`WorkspaceIndex::enclosing_body_spans`] computes the whole column in
    /// one stack sweep instead, `O((P + I) log (P + I))` per document.
    pub enclosing_body: Option<Span>,
}

/// One **registry symbol-definer** definition recorded in the index — a
/// `tcltest::test` case, a `testConstraint`, a `customMatch` mode, or an
/// iRules `when EVENT` handler (issue #790's outline tier, lifted to the
/// workspace for issue #1156).
///
/// Cross-document identity, as the index requires of every table: each of
/// these is a *named* definition carrying its enclosing namespace, spellable
/// from another file exactly as a proc's qualified name is.  They are not
/// callable commands, so they stay out of `procs` and out of the command
/// existence oracle; the workspace-symbol picker is their consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDefinedSymbol {
    /// Document the definition is in.
    pub uri: String,
    /// Definition name as resolved (a test's case label, an event's name).
    pub name: String,
    /// Fully-qualified name, with the enclosing namespace applied.
    pub qualified_name: String,
    /// The registry's outline category for it.
    pub kind: tcl_registry::DefinedSymbolKind,
    /// Byte span of the name token in `uri`'s source.
    pub name_span: Span,
}

/// One **namespace-qualified** variable declaration recorded in the index —
/// a `variable v` / `set v …` sitting directly in a `namespace eval` body or
/// at global scope, i.e. a cell a sibling document can name as `$::ns::v`.
///
/// Proc locals are deliberately absent: an unqualified name is resolved by
/// the local scope chain, which is a per-document question.  See the module
/// doc for the bound this table keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceVariable {
    /// Document the declaration is in.
    pub uri: String,
    /// Simple (tail) name, e.g. `version`.
    pub name: String,
    /// `::`-rooted qualified name, e.g. `::tomato::version`.
    pub qualified_name: String,
    /// Byte span of the declaring name token in `uri`'s source.
    pub name_span: Span,
}

/// One **namespace-qualified** variable occurrence (read or write) recorded
/// in the index — the reference-side companion to [`WorkspaceVariable`],
/// lifted from [`tcl_compiler::analyser::QualifiedVarRef`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceVariableRef {
    /// Document the occurrence is in.
    pub uri: String,
    /// `::`-rooted cell the occurrence names.
    pub qualified_name: String,
    /// Byte span of the name token as written.
    pub span: Span,
}

/// One local **alias** of a namespace-qualified cell recorded in the index —
/// a `variable v` / `global ::ns::v` / `namespace upvar ::ns v local` /
/// `upvar #0 ::ns::v local`, wherever it is written.
///
/// The third variable table, and the only one that is not about a document's
/// *own* namespace.  An alias binds a cell from an arbitrary scope in an
/// arbitrary namespace: a global `proc p {} { namespace upvar ::ns v local;
/// return $local }` binds `::ns::v` while declaring nothing in `::ns` and
/// writing no qualified occurrence, so it appears in neither
/// [`WorkspaceVariable`] nor [`WorkspaceVariableRef`] nor
/// [`WorkspaceIndex::documents_in_namespace`].  Renaming `::ns::v` without
/// visiting that document moves the declaration and leaves the alias bound to
/// a cell that no longer exists — `can't read "local": no such variable` on
/// tclsh 9.0.4 and 8.6.16 alike.
///
/// Unlike the other two tables this one *is* keyed on a proc-scope binding.
/// That is not a widening of the index's bound: the fact recorded is the
/// **qualified cell** the alias names, which is exactly as spellable from
/// another document as a declaration is.  The local spelling is not recorded
/// and stays a per-document question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceVariableAlias {
    /// Document the alias is written in.
    pub uri: String,
    /// `::`-rooted cell the alias binds to, e.g. `::ns::v`.
    pub qualified_name: String,
}

/// One occurrence of a word naming a **namespace**, recorded workspace-wide —
/// the cross-document half of issue #1088, lifted verbatim from
/// [`tcl_compiler::analyser::NamespaceRef`].
///
/// One table, not two, because a namespace's declaring site *is* one of its
/// spellings: the `::tomato` of `namespace eval ::tomato { … }` is both the
/// definition go-to-definition answers with and a word find-references
/// reports.  [`Self::declares`] is the discriminator, and it is registry data
/// ([`tcl_registry::Traits::DECLARES_NAMESPACE`]), not a spelling check.
///
/// Unlike [`WorkspaceVariable`] this table needs no qualified-only bound: the
/// analyser roots a relative namespace word against its own lexical namespace
/// before recording it, so every row already names one namespace absolutely,
/// spellable from any document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceNamespaceRef {
    /// Document the occurrence is in.
    pub uri: String,
    /// `::`-rooted namespace the occurrence names.
    pub qualified_name: String,
    /// Byte span of the name token as written.
    pub span: Span,
    /// `true` for a declaring `namespace eval` name word.
    pub declares: bool,
}

/// Whether a word carries a substitution marker, so its run-time text is not
/// the text as written.
fn is_computed_word(word: &str) -> bool {
    word.contains(['$', '['])
}

/// Whether an alias's recorded cell is **computed** rather than a fixed name.
///
/// `namespace upvar $ns v local` records the cell as written (`::$ns::v`) —
/// the analyser keeps the substitution marker rather than inventing a name —
/// so a marker anywhere in the cell is exactly the signal that this alias
/// binds no statically-known variable.
fn alias_cell_is_computed(cell: &str) -> bool {
    is_computed_word(cell)
}

/// One `source FILE` reference recorded in the index.
///
/// Tracks where a document loads another file so a file rename can
/// rewrite the dependent's `source` literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSource {
    /// Document containing the `source` statement.
    pub uri: String,
    /// Verbatim path text as written (with `${var}` / `[cmd]` markers
    /// preserved for substituted words).
    pub raw_path: String,
    /// Byte span of the path argument in `uri`'s source.
    pub range: Span,
    /// `true` when the path is a plain literal (no `$` / `[`).
    pub is_literal: bool,
    /// Command-resolution namespace at the `source` call site (a constructed
    /// `::`-rooted key).  `source` evaluates the file in the caller's current
    /// namespace (M9), so the sourced document's definitions re-home under
    /// this namespace — see [`WorkspaceIndex::source_seed_map`].
    pub site_namespace: String,
    /// Span of the innermost proc/class body containing the `source`
    /// statement; `None` at load level.  See
    /// [`WorkspaceGlobImport::enclosing_body`] — the same execution-order
    /// fact, needed here so a cross-document interpreter-state question
    /// ("had this statement already run when the child was loaded?") applies
    /// the identical [`tcl_compiler::analyser::indirection::in_effect_within`]
    /// rule the single-document tier does (issue #1253).
    pub enclosing_body: Option<Span>,
}

/// One unconditional `package prefer latest` recorded in the index.
///
/// `package prefer` latches **interpreter-global** state, so a raise in a file
/// that runs before this one really does change this one's version selection.
/// Which file runs first is not knowable in general — but along the `source`
/// graph it is: a file that `source`s another runs the sourcing statement
/// before the sourced file loads at all.  See
/// [`WorkspaceIndex::source_ancestor_prefers_latest`] (issue #1253).
///
/// Conditional raises (inside `if` / `catch` / `try`) are not recorded at all:
/// they may not run, and the abstention is toward the interpreter default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackagePrefer {
    /// Document containing the `package prefer latest` statement.
    pub uri: String,
    /// Byte offset of the `package` command word in `uri`'s source.
    pub at: u32,
    /// Span of the innermost proc/class body containing the raise; `None` at
    /// load level.
    pub enclosing_body: Option<Span>,
}

/// One `package require NAME` declaration recorded in the index.
///
/// Lets a module inherit the requires of the entry file(s) that `source` it,
/// so the workspace W120 refinement does not flag a command whose package is
/// required upstream (see [`crate::source_graph`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackageRequire {
    /// Document containing the `package require` statement.
    pub uri: String,
    /// Required package name (the `NAME` argument).
    pub name: String,
}

/// One command name-link recorded in the index.
///
/// A `namespace import`, `interp alias`, or `rename` introduces a *new*
/// callable name that resolves to another command: an imported `helper`
/// runs the exporting namespace's `helper`, an alias runs its target, a
/// `rename OLD NEW` makes `NEW` run what `OLD` denoted.  A call reaching the
/// new name is a reference to the ultimate target; the token that *names*
/// the target in the declaration (the import pattern, the alias `TARGET`
/// word, the `rename` `OLD` word) is itself a reference and a rename must
/// rewrite it.  Ground truth: the VM re-resolves an alias from `::` at call
/// time ([`tcl_vm::exec`]); a rename is a pure name move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCommandLink {
    /// Document the link declaration is in.
    pub uri: String,
    /// Fully-qualified name the link introduces (`::`-rooted): the imported
    /// `<ns>::<tail>`, the alias name, or the `rename` `NEW`.
    pub linked_qname: String,
    /// Fully-qualified name (`::`-rooted) the link resolves *to*: the import
    /// pattern's source, the alias `TARGET`, or the `rename` `OLD` — the
    /// command whose references a call through `linked_qname` joins.
    pub target_qname: String,
    /// Byte span of the token naming the target in the declaration (import
    /// pattern, alias `TARGET`, `rename` `OLD`).  A reference to the target;
    /// rename rewrites it.  `None` when the source scan did not record a span
    /// for this link kind.
    pub target_span: Option<Span>,
    /// Whether this link's own declaration (the `namespace import` /
    /// `interp alias` / `rename` command) sits inside another proc's or
    /// class's body — i.e. it takes effect only conditionally, when and if
    /// that enclosing definition actually runs (the same "rename a builtin
    /// away, install a same-named shadow, restore it" idiom
    /// [`WorkspaceProc::nested`] guards against, extended to the alias /
    /// rename / import forms of introducing a name). Mirrors
    /// [`WorkspaceProc::nested`] exactly, including its consumer:
    /// [`WorkspaceIndex::workspace_command_exists_for_call`] excludes only a
    /// *nested* link when a same-named builtin is in play, so an
    /// unconditional (top-level) alias/rename/import still counts as
    /// existing.
    pub nested: bool,
    /// For a link introduced by an **exact** `namespace import ::src::p`: the
    /// export snapshot the import must pass before it installs anything.
    /// `None` for an `interp alias` / `rename` link, and for a *conjectured*
    /// import — neither is gated by `namespace export`.
    ///
    /// An exact import is no less gated than a glob one: real Tcl installs
    /// **no** alias, silently, when `p` is not exported at the moment the
    /// import runs (oracle tclsh 8.6.14 / 9.0.4 — `namespace eval ::src {proc
    /// p {} {}}; namespace eval ::dst {namespace import ::src::p}` leaves
    /// `info commands ::dst::*` empty and raises no error; with `namespace
    /// export p` first it binds). Recording the site here, rather than
    /// resolving the gate when the link is created, is what keeps the answer
    /// correct across edits: the export usually lives in a *different*
    /// document, re-indexed independently of this one, so a decision frozen
    /// at creation time would go stale the moment that file changes (issue
    /// #1027). [`WorkspaceIndex::live_command_links`] applies it against the
    /// current index, cached per [`WorkspaceIndex::generation`].
    pub import_gate: Option<WorkspaceImportGate>,
}

/// The export-snapshot condition an exact `namespace import` link must satisfy
/// to be installed at all — see [`WorkspaceCommandLink::import_gate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceImportGate {
    /// The pattern's source namespace, with leading `::` (`::src` for
    /// `::src::p`).
    pub source_ns: String,
    /// The bare name the import binds (`p`).
    pub name: String,
    /// Byte offset of the import's pattern word within the link's own `uri`.
    pub at: u32,
    /// Span of the innermost proc/class body containing the import; `None` at
    /// load level. See [`WorkspaceGlobImport::enclosing_body`].
    pub enclosing_body: Option<Span>,
    /// `true` when the import carried its declared leading option word —
    /// `namespace import -force`. See [`WorkspaceGlobImport::forced`].
    pub forced: bool,
}

/// One wildcard `namespace import NS::*` recorded in the index.
///
/// Unlike [`WorkspaceCommandLink`], a glob pattern names no single command —
/// [`WorkspaceIndex::index_command_links`] deliberately skips it — so it
/// cannot resolve a bare call to a fixed `target_qname` on its own. Instead
/// each recorded entry is consulted per-call, against whichever bare `word`
/// the invocation actually writes:
/// [`WorkspaceIndex::resolve_wildcard_import`] glob-matches `word` against
/// [`Self::tail_pattern`] and requires [`Self::source_ns`] to have exported
/// a covering pattern (`WildcardImportIndex::exports_name`) before
/// resolving — see [`WorkspaceIndex::index_command_links`]'s doc comment for
/// why an exact pattern still takes the `WorkspaceCommandLink` path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceGlobImport {
    /// Document the `namespace import` is in.
    pub uri: String,
    /// Importing namespace, with leading `::` — the namespace a bare call
    /// must resolve *from* for this import to be in scope (the same
    /// candidate-namespace order ordinary command resolution already uses).
    pub ns: String,
    /// The pattern's source namespace, with leading `::` (`::Foo` for a
    /// `::Foo::*` / `::Foo::b*` pattern).
    pub source_ns: String,
    /// The pattern's final `::`-segment, exactly as written (`*`, `b*`,
    /// or a literal tail) — matched against a call's bare name with Tcl
    /// glob semantics ([`tcl_syntax::glob::string_match`]).
    pub tail_pattern: String,
    /// Byte offset of the import's pattern word within [`Self::uri`].
    ///
    /// The import's position on its own document's timeline: an import binds
    /// the names its source namespace exported *when the import ran*, so an
    /// export declared in the same file is judged against this offset (issue
    /// #1027). Meaningless against another file's offsets — which document
    /// loads first is not a static fact — so
    /// [`WildcardImportIndex::exports_name_at`] only compares within one URI.
    pub at: u32,
    /// Span of the innermost proc/class **body** containing this import,
    /// within [`Self::uri`]; `None` when the import is at load level.
    ///
    /// Ordering an event against this import is not a plain offset compare:
    /// an import written *inside a body* observes every top-level statement
    /// of its own file, wherever written, because the whole file loads before
    /// any body runs. This is the one fact
    /// [`tcl_compiler::analyser::indirection::in_effect`] reads out of an
    /// `AnalysisResult`, stored per row so the cross-document tier can apply
    /// that identical rule via
    /// [`tcl_compiler::analyser::indirection::in_effect_within`] instead of a
    /// weaker `at <= import_at` — which rejected such an export and lost a
    /// real imported alias.
    pub enclosing_body: Option<Span>,
    /// `true` when the import carried its declared leading option word —
    /// `namespace import -force`.
    ///
    /// Decides what happens when the importing namespace already holds a
    /// command of the imported name (oracle tclsh 8.6.14 / 9.0.4, issue
    /// #1103): without `-force` the import raises `can't import command "p":
    /// already exists` and installs **nothing**, so a bare call still reaches
    /// the local definition; with `-force` it silently replaces it and the
    /// call reaches the source (`namespace origin` → `::src::p`). See
    /// [`tcl_compiler::signature_scan::types::SignatureNamespaceImport::forced`]
    /// for why this is registry data rather than a `-force` name match.
    pub forced: bool,
}

/// One `namespace forget` **event** recorded in the index — the removal half
/// of the import edge's lifecycle log (issue #1103).
///
/// Aggregated workspace-wide for the same reason
/// [`WorkspaceNamespaceExport`] is: the forget and the import it undoes need
/// not live in the same file. Ordering only means something *within* a
/// document, which is why [`Self::uri`] and [`Self::at`] are always read
/// together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceNamespaceForget {
    /// Document the `namespace forget` is in.
    pub uri: String,
    /// The namespace losing the aliases, with leading `::`.
    pub ns: String,
    /// Source namespace named by a *qualified* pattern (`::src` for
    /// `namespace forget ::src::p`), `None` for a simple pattern — which
    /// matches this namespace's own imported command names whatever their
    /// origin. See
    /// [`tcl_compiler::signature_scan::types::SignatureNamespaceForget`].
    pub source_ns: Option<String>,
    /// The pattern's final `::`-segment, exactly as written, matched against
    /// a command's bare name with Tcl glob semantics.
    pub pattern: String,
    /// Byte offset of the event within [`Self::uri`].
    pub at: u32,
}

/// One straight-line command **deletion** recorded in the index — `rename
/// OLD {}` / `interp alias {} NAME {}`.
///
/// A deletion is a lifecycle event on every import edge pointing at the
/// deleted command: the alias holds the command object, so deleting the
/// source kills the alias too (issue #1103, oracle on
/// [`tcl_compiler::analyser::AnalysisResult::deleted_commands`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCommandDeletion {
    /// Document the deletion is in.
    pub uri: String,
    /// The `::`-normalised qualified name that was deleted.
    pub qualified_name: String,
    /// Byte offset of the deleting statement within [`Self::uri`].
    pub at: u32,
}

impl WorkspaceGlobImport {
    /// This import's site, for the export-snapshot gate.
    fn site(&self) -> ImportSite<'_> {
        ImportSite {
            uri: &self.uri,
            at: self.at,
            enclosing_body: self.enclosing_body,
        }
    }
}

impl WorkspaceImportGate {
    /// This gate's import site, for the export-snapshot query. `uri` is the
    /// owning link's document — the gate itself does not duplicate it.
    fn site<'a>(&self, uri: &'a str) -> ImportSite<'a> {
        ImportSite {
            uri,
            at: self.at,
            enclosing_body: self.enclosing_body,
        }
    }
}

/// One `namespace export` **event** recorded in the index.
///
/// Aggregated workspace-wide (unlike
/// [`tcl_compiler::analyser::AnalysisResult::namespace_exports`], which is
/// per-document) so a wildcard import in one file can be checked against an
/// export declared in *another* file — the cross-document half of issue
/// #923 idx 18.
///
/// An *event*, not a member of a set: `-clear` tombstones and ordering are
/// carried through so the cross-document tier applies the same per-import-site
/// snapshot the same-document one does (issue #1027 — see
/// [`crate::namespace_import`]). Ordering only means something *within* a
/// document, which is why [`Self::uri`] and [`Self::at`] are always read
/// together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceNamespaceExport {
    /// Document the `namespace export` is in.
    pub uri: String,
    /// Exporting namespace, with leading `::`.
    pub ns: String,
    /// Exported pattern text, exactly as written (relative to `ns`). Empty
    /// for a [`Self::clears`] tombstone.
    pub pattern: String,
    /// Byte offset of the event within [`Self::uri`].
    pub at: u32,
    /// `true` for a `namespace export -clear` tombstone.
    pub clears: bool,
}

/// Collect `items` into source order by the span `key` reports, so a
/// `HashMap`-valued analyser table lands in the index deterministically
/// instead of in the process's random hash order (issue #1028).
fn sorted_by_span<'a, T, I, F>(items: I, key: F) -> Vec<&'a T>
where
    I: IntoIterator<Item = &'a T>,
    F: Fn(&T) -> Span,
{
    let mut out: Vec<&T> = items.into_iter().collect();
    out.sort_by_key(|item| {
        let span = key(item);
        (span.start(), span.end())
    });
    out
}

/// The names of a `HashSet`-valued analyser table, in a stable order.  See
/// [`sorted_by_span`] for why the source order matters.
/// One class record's `(exports, unexports)` pair **for `side`** — the sided
/// visibility lookup [`WorkspaceIndex::dispatch_chain`] reads.
///
/// `exports`/`unexports` are the instance-side record by contract and
/// `class_exports`/`class_unexports` the class-object-side one; naming the
/// choice once is what keeps a `self unexport m` from ever silencing an
/// identically-named instance method (issues #1098 / #1119).
fn visibility_sets_for(c: &WorkspaceClass, side: MemberSide) -> (&[String], &[String]) {
    match side {
        MemberSide::Instance => (&c.exports, &c.unexports),
        MemberSide::ClassObject => (&c.class_exports, &c.class_unexports),
    }
}

fn sorted_names(names: &std::collections::HashSet<String>) -> Vec<String> {
    let mut out: Vec<String> = names.iter().cloned().collect();
    out.sort();
    out
}

/// The fourteen record tables **one document** contributes to the index.
///
/// Grouping the tables per document is what makes
/// [`WorkspaceIndex::remove_document`] cost the document's own records rather
/// than the workspace's (issue #1149).  Flat workspace-wide vectors made a
/// removal fourteen `Vec::retain` passes with a `String` compare per element
/// over tables that hold one row per call site and per qualified variable
/// occurrence — 10⁵–10⁶ rows on a tcllib-sized workspace — and the server runs
/// a removal on every diagnostics publish.
///
/// Consumers never see this type: [`WorkspaceIndex`] exposes each table as a
/// workspace-wide iterator that chains the per-document vectors in slot order,
/// which is the order the records were added in and therefore exactly the
/// order the previous flat vectors held them in.
#[derive(Debug, Clone, Default)]
struct DocumentRecords {
    procs: Vec<WorkspaceProc>,
    classes: Vec<WorkspaceClass>,
    variables: Vec<WorkspaceVariable>,
    variable_refs: Vec<WorkspaceVariableRef>,
    variable_aliases: Vec<WorkspaceVariableAlias>,
    namespace_refs: Vec<WorkspaceNamespaceRef>,
    invocations: Vec<WorkspaceInvocation>,
    sources: Vec<WorkspaceSource>,
    package_requires: Vec<WorkspacePackageRequire>,
    package_prefers: Vec<WorkspacePackagePrefer>,
    command_links: Vec<WorkspaceCommandLink>,
    glob_imports: Vec<WorkspaceGlobImport>,
    namespace_exports: Vec<WorkspaceNamespaceExport>,
    namespace_forgets: Vec<WorkspaceNamespaceForget>,
    command_deletions: Vec<WorkspaceCommandDeletion>,
    defined_symbols: Vec<WorkspaceDefinedSymbol>,
}

impl DocumentRecords {
    /// Drop every record, keeping each table's allocation.
    ///
    /// The capacity is deliberately retained: a re-index of the same document
    /// (the remove-then-add every publish performs) refills tables of very
    /// nearly the same size, so the slot's buffers are reused instead of being
    /// freed and regrown fourteen times per publish.  A slot whose document is
    /// gone for good keeps its capacity until another document reuses the slot
    /// — bounded by the workspace's peak document count, not by the number of
    /// removals.
    fn clear(&mut self) {
        let Self {
            procs,
            classes,
            variables,
            variable_refs,
            variable_aliases,
            namespace_refs,
            invocations,
            sources,
            package_requires,
            package_prefers,
            command_links,
            glob_imports,
            namespace_exports,
            namespace_forgets,
            command_deletions,
            defined_symbols,
        } = self;
        procs.clear();
        classes.clear();
        variables.clear();
        variable_refs.clear();
        variable_aliases.clear();
        namespace_refs.clear();
        invocations.clear();
        sources.clear();
        package_requires.clear();
        package_prefers.clear();
        command_links.clear();
        glob_imports.clear();
        namespace_exports.clear();
        namespace_forgets.clear();
        command_deletions.clear();
        defined_symbols.clear();
    }

    /// Append this document's [`IndexedWorkspaceSymbol`]s matching
    /// `lower_query` to `out`, stopping once `out` reaches `limit`.
    ///
    /// The order — procs, then classes with their members, then registry
    /// symbol-definer definitions — mirrors the per-document outline the
    /// document-symbol provider produces, and each table is already in source
    /// order (see [`sorted_by_span`]).
    fn collect_symbols_matching(
        &self,
        lower_query: &str,
        limit: usize,
        retractions: &RetractionIndex<'_>,
        out: &mut Vec<IndexedWorkspaceSymbol>,
    ) {
        for proc_def in &self.procs {
            if out.len() >= limit {
                return;
            }
            if matches_query(&proc_def.name, lower_query)
                || matches_query(&proc_def.qualified_name, lower_query)
            {
                out.push(IndexedWorkspaceSymbol {
                    uri: proc_def.uri.clone(),
                    name: proc_def.name.clone(),
                    container_name: namespace_of(&proc_def.qualified_name),
                    kind: WorkspaceSymbolKind::Function,
                    name_span: proc_def.name_span,
                });
            }
        }
        // A constructor's only name is the keyword, so whether the query
        // admits one is decided once rather than per class.
        let wants_constructors = matches_query("constructor", lower_query);
        for class_def in &self.classes {
            if out.len() >= limit {
                return;
            }
            if matches_query(&class_def.name, lower_query)
                || matches_query(&class_def.qualified_name, lower_query)
            {
                out.push(IndexedWorkspaceSymbol {
                    uri: class_def.uri.clone(),
                    name: class_def.name.clone(),
                    container_name: namespace_of(&class_def.qualified_name),
                    kind: WorkspaceSymbolKind::Class,
                    name_span: class_def.name_span,
                });
            }
            // Members carry the class's qualified name as their container, so
            // an editor renders them as `ClassName::methodName`.
            //
            // Enumerated through the workspace's member fold, not straight off
            // this record's own table: a member a cross-file `oo::define ::C {
            // renamemethod old new }` moved is declared here under `old` and
            // dispatches as `new`, and one deleted the same way is not a
            // member at all (issue #1263).  An arrived member's location is
            // the arrival word in the retracting document, which is also what
            // go-to-definition on the new name answers with.
            for method in &class_def.methods {
                if out.len() >= limit {
                    return;
                }
                let Some(em) = effective_member(retractions, class_def, method) else {
                    continue;
                };
                if matches_query(em.name, lower_query) {
                    out.push(IndexedWorkspaceSymbol {
                        uri: em.name_uri.to_owned(),
                        name: em.name.to_owned(),
                        container_name: Some(class_def.qualified_name.clone()),
                        kind: WorkspaceSymbolKind::Method,
                        name_span: em.name_span,
                    });
                }
            }
            if wants_constructors {
                for &name_span in &class_def.constructor_spans {
                    if out.len() >= limit {
                        return;
                    }
                    out.push(IndexedWorkspaceSymbol {
                        uri: class_def.uri.clone(),
                        name: "constructor".to_owned(),
                        container_name: Some(class_def.qualified_name.clone()),
                        kind: WorkspaceSymbolKind::Constructor,
                        name_span,
                    });
                }
            }
        }
        for sym in &self.defined_symbols {
            if out.len() >= limit {
                return;
            }
            if matches_query(&sym.name, lower_query)
                || matches_query(&sym.qualified_name, lower_query)
            {
                out.push(IndexedWorkspaceSymbol {
                    uri: sym.uri.clone(),
                    name: sym.name.clone(),
                    container_name: namespace_of(&sym.qualified_name),
                    kind: WorkspaceSymbolKind::from(sym.kind),
                    name_span: sym.name_span,
                });
            }
        }
    }

    /// Lift one document's analysis into this record set.
    ///
    /// The caller ([`WorkspaceIndex::add_document`]) has already cleared the
    /// slot, so this only ever appends.
    fn index_document(&mut self, uri: &str, analysis: &AnalysisResult) {
        // `all_procs` / `all_classes` / a class's `methods` are `HashMap`s, so
        // iterating them directly would order this document's index entries by
        // the process's random hash seed — and consumers that answer with the
        // *first* matching record (go-to-definition's dispatch entry, most
        // visibly) then answer differently run to run (issue #1028).  Source
        // order is the stable, meaningful order: it is also the order the
        // records take effect in when the file is sourced.
        for proc_def in sorted_by_span(analysis.all_procs.values(), |p| p.name_span) {
            self.procs.push(WorkspaceProc {
                uri: uri.to_owned(),
                name: proc_def.name.clone(),
                qualified_name: proc_def.qualified_name.clone(),
                param_count: proc_def.params.len(),
                name_span: proc_def.name_span,
                nested: analysis.offset_is_inside_any_definition_body(proc_def.name_span.start()),
            });
        }
        self.index_classes(uri, analysis);
        for sym in sorted_by_span(&analysis.all_defined_symbols, |s| s.name_span) {
            self.defined_symbols.push(WorkspaceDefinedSymbol {
                uri: uri.to_owned(),
                name: sym.name.clone(),
                qualified_name: sym.qualified_name.clone(),
                kind: sym.kind,
                name_span: sym.name_span,
            });
        }
        self.index_variables(uri, analysis);
        self.index_namespace_refs(uri, analysis);
        let bodies = WorkspaceIndex::enclosing_body_spans(
            analysis,
            &analysis
                .command_invocations
                .iter()
                .map(|inv| inv.range.start())
                .collect::<Vec<_>>(),
        );
        for (inv, enclosing_body) in analysis.command_invocations.iter().zip(bodies) {
            self.invocations.push(WorkspaceInvocation {
                uri: uri.to_owned(),
                name: inv.name.clone(),
                resolution_candidates: inv.resolution_candidates.clone(),
                range: inv.range,
                indirect: inv.indirect,
                rename_safe: inv.rename_safe,
                ensemble_dispatch: inv.ensemble_dispatch,
                enclosing_body,
            });
        }
        for target in &analysis.source_targets {
            self.sources.push(WorkspaceSource {
                uri: uri.to_owned(),
                raw_path: target.raw_path.clone(),
                site_namespace: target.site_namespace.clone(),
                range: target.range,
                is_literal: target.is_literal,
                enclosing_body: analysis.innermost_definition_body_span(target.range.start()),
            });
        }
        for pr in &analysis.package_requires {
            self.package_requires.push(WorkspacePackageRequire {
                uri: uri.to_owned(),
                name: pr.name.clone(),
            });
        }
        // Only the unconditional raises: a `package prefer latest` inside an
        // `if` / `catch` / `try` may not run, and the cross-document tier
        // abstains toward the interpreter default exactly as the
        // single-document one does (issue #1253).
        for prefer in analysis
            .package_prefer_latest
            .iter()
            .filter(|p| !p.conditional)
        {
            self.package_prefers.push(WorkspacePackagePrefer {
                uri: uri.to_owned(),
                at: prefer.range.start(),
                enclosing_body: analysis.innermost_definition_body_span(prefer.range.start()),
            });
        }
        for exp in &analysis.namespace_exports {
            self.namespace_exports.push(WorkspaceNamespaceExport {
                uri: uri.to_owned(),
                ns: exp.ns.clone(),
                pattern: exp.pattern.clone(),
                at: exp.range.start(),
                clears: exp.clears,
            });
        }
        self.index_import_lifecycle(uri, analysis);
        self.index_command_links(uri, analysis);
    }

    /// Lift one document's class definitions, each with its members flattened
    /// into one source-ordered [`WorkspaceMethod`] list (instance methods
    /// first, then the class-side ones).
    ///
    /// Split out of [`Self::index_document`] only for size; the source-order
    /// rule that walk documents applies here unchanged.
    fn index_classes(&mut self, uri: &str, analysis: &AnalysisResult) {
        for class_def in sorted_by_span(analysis.all_classes.values(), |c| c.name_span) {
            let methods: Vec<WorkspaceMethod> =
                sorted_by_span(class_def.methods.values(), |m| m.name_span)
                    .into_iter()
                    .map(|m| WorkspaceMethod {
                        name: m.name.clone(),
                        kind: m.kind.clone(),
                        exported: m.visibility == "public",
                        private: m.visibility == "private",
                        is_self_method: m.is_self_method,
                        name_span: m.name_span,
                    })
                    .chain(
                        sorted_by_span(class_def.class_methods.values(), |m| m.name_span)
                            .into_iter()
                            .map(|m| WorkspaceMethod {
                                name: m.name.clone(),
                                kind: "classmethod".to_string(),
                                exported: m.visibility == "public",
                                private: m.visibility == "private",
                                is_self_method: m.is_self_method,
                                name_span: m.name_span,
                            }),
                    )
                    .collect();
            self.classes.push(WorkspaceClass {
                uri: uri.to_owned(),
                name: class_def.name.clone(),
                qualified_name: class_def.qualified_name.clone(),
                name_span: class_def.name_span,
                superclasses: class_def.superclasses.clone(),
                mixins: class_def.mixins.clone(),
                methods,
                exports: sorted_names(&class_def.exports),
                unexports: sorted_names(&class_def.unexports),
                class_exports: sorted_names(&class_def.class_exports),
                class_unexports: sorted_names(&class_def.class_unexports),
                retracted_members: class_def.retracted_members.clone(),
                via_define: class_def.via_define,
                metaclass: class_def.metaclass.clone(),
                constructor_spans: class_def.constructors.iter().map(|c| c.name_span).collect(),
            });
        }
    }

    /// Lift a document's `namespace forget` events and command **destructions**
    /// — the removal half of the import edge's lifecycle (issue #1103).
    ///
    /// A destroying `rename OLD {}` / `interp alias {} NAME {}` kills every
    /// import alias pointing at `OLD`, because the alias holds the command
    /// object; a plain `rename OLD NEW` does not, which is why
    /// `AnalysisResult::destroyed_commands` (not `deleted_commands`) is the
    /// source here.
    fn index_import_lifecycle(&mut self, uri: &str, analysis: &AnalysisResult) {
        for fgt in &analysis.namespace_forgets {
            self.namespace_forgets.push(WorkspaceNamespaceForget {
                uri: uri.to_owned(),
                ns: fgt.ns.clone(),
                source_ns: fgt.source_ns.clone(),
                pattern: fgt.pattern.clone(),
                at: fgt.range.start(),
            });
        }
        // `HashMap`-valued, so sort into source order for the same reason
        // `sorted_by_span` exists (issue #1028): a consumer answering with the
        // first matching record must not answer differently run to run.
        let mut destroyed: Vec<(&String, &u32)> = analysis.destroyed_commands.iter().collect();
        destroyed.sort_unstable_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0)));
        for (name, at) in destroyed {
            self.command_deletions.push(WorkspaceCommandDeletion {
                uri: uri.to_owned(),
                qualified_name: tcl_syntax::naming::normalise_qualified_name(name),
                at: *at,
            });
        }
    }

    /// Lift a document's three **variable** tables: the namespace-scoped
    /// declarations it holds, the qualified cells it *aliases*, and the
    /// qualified occurrences it writes.
    ///
    /// The compiler owns both enumeration rules — `namespace_variables` (the
    /// enumerating twin of the single-name `lookup_var_in_namespace` the
    /// in-document providers use) and `variable_alias_links` (the enumerating
    /// twin of `VarDef::link_target`) — so the index cannot drift from what a
    /// single-document lookup would answer.
    fn index_variables(&mut self, uri: &str, analysis: &AnalysisResult) {
        for (qualified, var) in tcl_compiler::analyser::namespace_variables(&analysis.global_scope)
        {
            self.variables.push(WorkspaceVariable {
                uri: uri.to_owned(),
                name: var.name.clone(),
                qualified_name: qualified,
                name_span: var.definition_span,
            });
        }
        // Only the *cell* is recorded, so a document aliasing one cell twenty
        // times contributes one row: the question this table answers is "must
        // the rename visit this document", not "where in it".
        let mut aliased: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for link in tcl_compiler::analyser::variable_alias_links(&analysis.global_scope) {
            aliased.insert(tcl_syntax::naming::normalise_qualified_name(link.cell));
        }
        for qualified_name in aliased {
            self.variable_aliases.push(WorkspaceVariableAlias {
                uri: uri.to_owned(),
                qualified_name,
            });
        }
        for vref in &analysis.qualified_var_refs {
            self.variable_refs.push(WorkspaceVariableRef {
                uri: uri.to_owned(),
                qualified_name: tcl_syntax::naming::normalise_qualified_name(&vref.qualified_name),
                span: vref.span,
            });
        }
    }

    /// Lift a document's namespace-name occurrences
    /// ([`tcl_compiler::analyser::AnalysisResult::namespace_refs`]) into the
    /// workspace table, so `namespace children ::tomato` in one file reaches
    /// the `namespace eval ::tomato { … }` in another (issue #1088).
    ///
    /// A straight copy: the analyser has already rooted relative spellings
    /// and dropped computed ones, so there is no second resolution rule here
    /// that could disagree with the in-document providers.
    fn index_namespace_refs(&mut self, uri: &str, analysis: &AnalysisResult) {
        for nref in &analysis.namespace_refs {
            self.namespace_refs.push(WorkspaceNamespaceRef {
                uri: uri.to_owned(),
                qualified_name: tcl_syntax::naming::normalise_qualified_name(&nref.qualified_name),
                span: nref.span,
                declares: nref.declares,
            });
        }
    }

    /// Lift a document's `namespace import` / `interp alias` / `rename`
    /// records into flat [`WorkspaceCommandLink`] entries the cross-document
    /// reference walk can follow.  Each becomes `linked_qname → target_qname`:
    /// the new callable name and the command it ultimately runs.
    fn index_command_links(&mut self, uri: &str, analysis: &AnalysisResult) {
        use tcl_syntax::naming::normalise_qualified_name;
        // `namespace import ::mod::helper` inside `::app` binds `::app::helper`
        // to the exporting `::mod::helper`.  A glob pattern names no single
        // command, so it introduces no fixed `WorkspaceCommandLink` — instead
        // it is indexed as a [`WorkspaceGlobImport`], consulted per-call by
        // [`Self::resolve_wildcard_import`] against whichever bare name the
        // invocation actually writes (issue #923 idx 18).
        for imp in &analysis.namespace_imports {
            if imp.pattern.contains(['*', '?', '[']) {
                if let Some((source_ns, tail_pattern)) = imp.pattern.rsplit_once("::") {
                    self.glob_imports.push(WorkspaceGlobImport {
                        uri: uri.to_owned(),
                        ns: imp.ns.clone(),
                        source_ns: global_rooted(source_ns).to_owned(),
                        tail_pattern: tail_pattern.to_owned(),
                        at: imp.range.start(),
                        enclosing_body: analysis.innermost_definition_body_span(imp.range.start()),
                        forced: imp.forced,
                    });
                }
                continue;
            }
            let Some(tail) = imp.pattern.rsplit("::").find(|s| !s.is_empty()) else {
                continue;
            };
            // An exact import is export-gated exactly like a glob one — real
            // Tcl silently installs nothing when the name is not exported at
            // the import's own position (issue #1027; oracle on
            // `WorkspaceCommandLink::import_gate`). One shape stays ungated:
            // a *conjectured* import, which is inferred from a tcllib
            // `<NS>::import <ALIAS>` wrapper rather than read off a real
            // `namespace import` word, so there is no export declaration for
            // it to have passed and gating it would drop the idiom entirely.
            //
            // A pattern rooted at the global namespace (`namespace import
            // ::p`) is **not** one of them, though it reads as an empty
            // source namespace and both tiers used to skip it on that basis —
            // the last import shape bypassing the gate (#1104's review note).
            // Real Tcl treats it like any other unexported import: a silent
            // no-op leaving `info commands ::dst::*` empty (oracle 8.6.14 /
            // 9.0.4). `::` is its source namespace, the same spelling every
            // global-level `namespace export` record already carries.
            let import_gate = (!imp.conjectured)
                .then(|| imp.pattern.rsplit_once("::"))
                .flatten()
                .map(|(source_ns, _)| WorkspaceImportGate {
                    source_ns: global_rooted(source_ns).to_owned(),
                    name: tail.to_owned(),
                    at: imp.range.start(),
                    enclosing_body: analysis.innermost_definition_body_span(imp.range.start()),
                    forced: imp.forced,
                });
            self.command_links.push(WorkspaceCommandLink {
                uri: uri.to_owned(),
                linked_qname: tcl_syntax::naming::qualify(&imp.ns, tail),
                target_qname: normalise_qualified_name(&imp.pattern),
                target_span: Some(imp.range),
                nested: analysis.offset_is_inside_any_definition_body(imp.range.start()),
                import_gate,
            });
        }
        // `interp alias {} a {} ::mod::helper` binds `a` to `::mod::helper`;
        // the alias target resolves from `::` at call time, so root it there.
        // The `TARGET` word itself is already a first-class command invocation
        // (the registry marks it a command prefix), so it needs no
        // `target_span` here — the ordinary reference/rename path covers it;
        // this link only lets a call through the *alias name* resolve.
        for alias in analysis.command_aliases.values() {
            if alias.target.is_empty() {
                continue;
            }
            let nested = analysis
                .alias_offsets
                .get(&alias.qualified_name)
                .is_some_and(|&off| analysis.offset_is_inside_any_definition_body(off));
            self.command_links.push(WorkspaceCommandLink {
                uri: uri.to_owned(),
                linked_qname: normalise_qualified_name(&alias.qualified_name),
                target_qname: normalise_qualified_name(&alias.target),
                target_span: None,
                nested,
                import_gate: None,
            });
        }
        // `rename OLD NEW` makes `NEW` run what `OLD` denoted.  The recorded
        // map is `NEW → OLD`, both already `::`-normalised.  `OLD`'s own
        // word is already a first-class command invocation (issue #923 idx
        // 39) — the ordinary reference/rename path covers it — so, like
        // `interp alias`'s `TARGET` word above, it needs no `target_span`
        // here.
        for (new, old) in &analysis.renamed_commands {
            let nested = analysis
                .rename_offsets
                .get(new)
                .is_some_and(|&off| analysis.offset_is_inside_any_definition_body(off));
            self.command_links.push(WorkspaceCommandLink {
                uri: uri.to_owned(),
                linked_qname: normalise_qualified_name(new),
                target_qname: normalise_qualified_name(old),
                target_span: None,
                nested,
                import_gate: None,
            });
        }
    }
}

/// Cross-document aggregate of proc / class definitions,
/// command-invocation sites, `source` references, command
/// name-links, and `package require` declarations.
///
/// Records are stored per document ([`DocumentRecords`]) in a slot vector, with
/// `slots` mapping a document URI to its slot.  Removing a document clears its
/// slot and returns the index to `free_slots`; the next document to be added —
/// in practice the same one, since the server's re-index is a remove
/// immediately followed by an add — takes the most recently freed slot back, so
/// a document keeps its position across a re-index and the slot vector stays
/// bounded by the workspace's peak document count.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceIndex {
    docs: Vec<DocumentRecords>,
    slots: std::collections::HashMap<String, usize>,
    free_slots: Vec<usize>,
    generation: u64,
    /// Every command name the workspace defines, in each spelling a call site
    /// may write — see [`WorkspaceIndex::command_names`].
    command_names: Derived<HashSet<String>>,
    /// Parallel liveness mask over `command_links` — see
    /// [`WorkspaceIndex::live_command_links`].
    live_links: Derived<Vec<bool>>,
    /// `::`-stripped qualified names of every indexed proc and class, and the
    /// same set with the names links introduce — see
    /// [`WorkspaceIndex::defined_command_names`].
    defined_names: [Derived<HashSet<String>>; 2],
    /// `linked name -> target name` over the live links — see
    /// [`WorkspaceIndex::command_link_map`].
    command_link_map: Derived<std::collections::HashMap<String, String>>,
    /// Every invocation's settled target, grouped by that target, with and
    /// without link-following — see
    /// [`WorkspaceIndex::invocations_by_settled_target`].
    settled_invocations: [Derived<SettledTargets>; 2],
    /// The whole-program export oracle the single-document tier borrows — see
    /// [`WorkspaceIndex::export_snapshot`].
    export_snapshot: Derived<NamespaceExportSnapshot>,
    /// The `source`-path → document-URI resolver, when the host has installed
    /// one — see [`WorkspaceIndex::set_source_resolver`].
    source_resolver: Option<SourceResolver>,
    /// The `source`-graph load order derived from it — see
    /// [`WorkspaceIndex::run_order`].
    run_order: Derived<crate::source_graph::RunOrder>,
}

/// The host's `source`-path → document-URI resolver: `(sourcing document's
/// URI, the path word as written, whether that word is a plain literal)` to
/// the sourced document's URI, or `None` for a path the host cannot place.
///
/// `WorkspaceIndex` holds no URI ↔ filesystem-path mapping of its own, so this
/// is how the `source`-graph load order gets its edges — see
/// [`WorkspaceIndex::set_source_resolver`].  The signature is
/// [`WorkspaceIndex::source_seed_map`]'s, so one host resolver serves both.
pub type SourceResolver = fn(&str, &str, bool) -> Option<String>;

/// A whole-index **derived view**: a value that is a pure function of the
/// index's contents, built at most once per [`WorkspaceIndex::generation`] and
/// dropped by the mutation hook that bumps it.
///
/// The workspace-indexing contract's rule 5 in one type, so the reset/build
/// boilerplate is written once rather than per view (issue #1105). Excluded
/// from equality and dropped on clone — the same discipline
/// `tcl_compiler::analyser::HierarchyCache` uses for the class hierarchy it
/// derives from `all_classes`. `Arc` because a caller may hold the view while
/// the index it came from is dropped or re-derived.
#[derive(Debug)]
struct Derived<T>(std::sync::OnceLock<Arc<T>>);

impl<T> Default for Derived<T> {
    fn default() -> Self {
        Self(std::sync::OnceLock::new())
    }
}

impl<T> Clone for Derived<T> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<T> Derived<T> {
    /// The cached value, building it with `build` on first use.
    fn get_or_build(&self, build: impl FnOnce() -> T) -> Arc<T> {
        Arc::clone(self.0.get_or_init(|| Arc::new(build())))
    }
}

/// Every invocation's **settled target**, grouped by that target's
/// `::`-stripped qualified name: `target -> [(doc slot, index within that
/// slot's invocations)]`.
///
/// [`WorkspaceIndex::invocations_of`] (code-lens's per-proc / per-class
/// reference count, issue #1152) used to re-settle *every* invocation in the
/// workspace against the candidate target on every call — `code_lenses`
/// calls it once per proc and once per class, so an N-proc document paid an
/// O(N × invocations) walk, each rebuilding [`WildcardImportIndex`] (five
/// `HashMap`s over the whole index) from scratch. Settling every invocation
/// once per generation and grouping the results turns each subsequent
/// `invocations_of` call into a hash lookup plus a direct index into the
/// owning document's slot — the indices are stable because a document's
/// `Vec<WorkspaceInvocation>` is only ever cleared and refilled wholesale
/// (`DocumentRecords::clear` / `index_document`), never reordered in place,
/// and any mutation that could invalidate a stored `(slot, index)` pair also
/// bumps the generation and drops the [`Derived`] view holding it.
type SettledTargets = std::collections::HashMap<String, Vec<(usize, usize)>>;

impl WorkspaceIndex {
    /// Empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build an index from an iterator of `(uri, analysis)`
    /// pairs — typically the server's cached-analysis map.
    #[must_use]
    pub fn from_documents<'a, I>(documents: I) -> Self
    where
        I: IntoIterator<Item = (&'a str, &'a AnalysisResult)>,
    {
        let mut index = Self::new();
        for (uri, analysis) in documents {
            index.add_document(uri, analysis);
        }
        index
    }

    /// A counter bumped by every mutation ([`Self::add_document`] /
    /// [`Self::remove_document`]).  Lets a consumer tell whether the index has
    /// changed since it last looked without diffing its contents.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Every indexed qualified variable **occurrence**.
    fn variable_refs(&self) -> impl Iterator<Item = &WorkspaceVariableRef> {
        self.docs.iter().flat_map(|doc| doc.variable_refs.iter())
    }

    /// Every indexed **alias** of a qualified variable cell.
    fn variable_aliases(&self) -> impl Iterator<Item = &WorkspaceVariableAlias> {
        self.docs.iter().flat_map(|doc| doc.variable_aliases.iter())
    }

    /// Every indexed wildcard `namespace import NS::*`.
    fn glob_imports(&self) -> impl Iterator<Item = &WorkspaceGlobImport> {
        self.docs.iter().flat_map(|doc| doc.glob_imports.iter())
    }

    /// Every indexed `namespace export` declaration.
    fn namespace_exports(&self) -> impl Iterator<Item = &WorkspaceNamespaceExport> {
        self.docs
            .iter()
            .flat_map(|doc| doc.namespace_exports.iter())
    }

    /// Every indexed `namespace forget`.
    fn namespace_forgets(&self) -> impl Iterator<Item = &WorkspaceNamespaceForget> {
        self.docs
            .iter()
            .flat_map(|doc| doc.namespace_forgets.iter())
    }

    /// Every indexed command **destruction** (`rename OLD {}` / a destroying
    /// `interp alias`).
    fn command_deletions(&self) -> impl Iterator<Item = &WorkspaceCommandDeletion> {
        self.docs
            .iter()
            .flat_map(|doc| doc.command_deletions.iter())
    }

    /// [`tcl_compiler::analyser::AnalysisResult::innermost_definition_body_span`]
    /// for a whole column of offsets at once, in the order they were given.
    ///
    /// That single-offset lookup scans every recorded proc and class body, so
    /// calling it once per invocation is `O(procs × invocations)` — the cost
    /// that kept a per-call body span out of the index (issue #1116 item 3).
    /// It is not the cost the answer actually needs: definition bodies are
    /// syntactic, so they **nest properly** — no two of them partially overlap
    /// — which means one pass over the bodies in start order, keeping the
    /// currently-open chain on a stack, yields the innermost body of every
    /// offset in increasing order. Sorting both sides makes the whole column
    /// `O((P + I) log (P + I))` instead, small beside the analysis that
    /// produced them.
    ///
    /// Ties in `start` are ordered by *longer first*, so a body sharing its
    /// start with a nested one is pushed before its child and the stack top
    /// stays the innermost.
    fn enclosing_body_spans(analysis: &AnalysisResult, offsets: &[u32]) -> Vec<Option<Span>> {
        let mut bodies: Vec<Span> = analysis
            .all_procs
            .values()
            .map(|p| p.body_span)
            .chain(analysis.all_classes.values().map(|c| c.body_span))
            .collect();
        if bodies.is_empty() {
            return vec![None; offsets.len()];
        }
        bodies.sort_unstable_by_key(|s| (s.start(), std::cmp::Reverse(s.end())));
        let mut order: Vec<usize> = (0..offsets.len()).collect();
        order.sort_unstable_by_key(|&i| offsets[i]);
        let mut out = vec![None; offsets.len()];
        let mut open: Vec<Span> = Vec::new();
        let mut next = 0usize;
        for i in order {
            let off = offsets[i];
            while next < bodies.len() && bodies[next].start() <= off {
                open.push(bodies[next]);
                next += 1;
            }
            while open.last().is_some_and(|b| b.end() <= off) {
                open.pop();
            }
            out[i] = open.last().copied();
        }
        out
    }

    /// Every command name the workspace defines — each indexed proc and class
    /// in the three forms a call site may spell (`::ns::name`, `ns::name`,
    /// `name`).
    ///
    /// This is the cross-file "does this command exist anywhere in the
    /// project?" set the unknown-command (W123) refinement consults.  It is
    /// **derived**, so it is built once and cached until the next index
    /// mutation rather than walked per request: on a 400-file / 10 000-proc
    /// workspace it is ~20 000 names and ~7 ms to build, against ~120 ns to
    /// serve from the cache.
    #[must_use]
    pub fn command_names(&self) -> Arc<HashSet<String>> {
        self.command_names.get_or_build(|| {
            let mut names: HashSet<String> = HashSet::new();
            for p in self.procs() {
                tcl_compiler::analyser::utils::insert_qualified_and_tail(
                    &mut names,
                    &p.qualified_name,
                );
            }
            for c in self.classes() {
                tcl_compiler::analyser::utils::insert_qualified_and_tail(
                    &mut names,
                    &c.qualified_name,
                );
            }
            names
        })
    }

    /// The whole-program export oracle the *single-document* tier consults
    /// when deciding whether a `namespace import -force` really deleted the
    /// importing namespace's own command (issue #1116 item 1).
    ///
    /// One document cannot answer that on its own: the `namespace export` that
    /// decides it may live in another file, and two programs whose single
    /// document is byte-identical then disagree — the transcript is on
    /// [`crate::namespace_import::NamespaceExportOracle`]. This is the
    /// whole-program evidence that closes the gap, and nothing more: it
    /// answers one question and may answer
    /// [`ExportVerdict::Unknown`].
    ///
    /// A [`Derived`] view, so it is built at most once per
    /// [`Self::generation`]; the returned `Arc` is owned, so a caller may hold
    /// it after releasing the index lock (which is how the server hands it to
    /// its blocking providers).
    #[must_use]
    pub fn export_snapshot(&self) -> Arc<NamespaceExportSnapshot> {
        self.export_snapshot.get_or_build(|| {
            let mut exports_by_ns: std::collections::HashMap<
                String,
                Vec<WorkspaceNamespaceExport>,
            > = std::collections::HashMap::new();
            for exp in self.namespace_exports() {
                exports_by_ns
                    .entry(exp.ns.trim_start_matches("::").to_owned())
                    .or_default()
                    .push(exp.clone());
            }
            NamespaceExportSnapshot {
                exports_by_ns,
                observable: self
                    .observable_namespaces()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                order: self.run_order(),
            }
        })
    }

    /// Install the host's literal-`source`-path → document-URI resolver, so
    /// the index can build the [`crate::source_graph::RunOrder`] the
    /// import-lifecycle gates rank cross-document events with (issue #1104
    /// item 3).
    ///
    /// The index deliberately holds no URI ↔ filesystem-path mapping of its
    /// own — that is the host's knowledge, and every other `source`-graph
    /// consumer already takes the same closure per call
    /// ([`Self::source_ancestor_package_requires`],
    /// [`Self::source_ancestor_prefers_latest`], [`Self::source_seed_map`]).
    /// The order, though, is consulted *inside* the per-call import walk,
    /// where a per-call resolver argument would have to be threaded through
    /// every caller of `resolve_wildcard_import` and every derived view that
    /// builds a `WildcardImportIndex`. Holding it here instead lets the order
    /// be a [`Derived`] view like the rest, built at most once per
    /// [`Self::generation`].
    ///
    /// A plain `fn` pointer rather than a boxed closure: the resolver is a
    /// pure function of `(parent uri, raw path, is_literal)` in every host, and
    /// a `fn` keeps the index `Debug + Clone + Default` with no manual impls.
    /// The signature is [`Self::source_seed_map`]'s, so one host resolver
    /// serves both — including its statically-foldable computed-path tier
    /// (`[file join [file dirname [info script]] x.tcl]`), which is as provable
    /// as a literal and is the idiom real multi-file projects actually write.
    /// A path the host cannot prove returns `None` and sequences nothing.
    ///
    /// **Without a resolver the order is empty**, and both tiers behave
    /// exactly as they did before the `source` graph existed — every
    /// cross-document event unrankable, every same-document one ordered.
    pub fn set_source_resolver(&mut self, resolve: SourceResolver) {
        self.source_resolver = Some(resolve);
        self.run_order = Derived::default();
        // The export snapshot captures the order, so it goes with it.
        self.export_snapshot = Derived::default();
    }

    /// The `source`-graph load order over this workspace's documents, built at
    /// most once per [`Self::generation`].
    ///
    /// Empty when no resolver is installed (see [`Self::set_source_resolver`])
    /// or when no `source` statement resolves. A target the resolver cannot
    /// place — `source $dir/x.tcl` with `$dir` unknown — names no document
    /// statically and sequences nothing.
    fn run_order(&self) -> Arc<crate::source_graph::RunOrder> {
        self.run_order.get_or_build(|| {
            let Some(resolve) = self.source_resolver else {
                return crate::source_graph::RunOrder::default();
            };
            let edges: Vec<crate::source_graph::SourceEdge> = self
                .sources()
                .filter_map(|s| {
                    resolve(&s.uri, &s.raw_path, s.is_literal).map(|child| {
                        crate::source_graph::SourceEdge {
                            parent: s.uri.clone(),
                            child,
                            at: s.range.start(),
                            enclosing_body: s.enclosing_body,
                        }
                    })
                })
                .collect();
            crate::source_graph::RunOrder::build(&edges)
        })
    }

    /// Note a mutation: bump the generation and drop every derived cache.
    fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.command_names = Derived::default();
        self.live_links = Derived::default();
        self.defined_names = <[Derived<HashSet<String>>; 2]>::default();
        self.command_link_map = Derived::default();
        self.settled_invocations = <[Derived<SettledTargets>; 2]>::default();
        self.export_snapshot = Derived::default();
        self.run_order = Derived::default();
    }

    /// The command name-links that are actually installed — every `interp
    /// alias` / `rename` link, plus each exact `namespace import` link whose
    /// [`WorkspaceCommandLink::import_gate`] its source namespace's export
    /// timeline admits (issue #1027).
    ///
    /// Every consumer of `command_links` that answers "does this name exist /
    /// what does it reach" goes through here, so an import Tcl never
    /// installed cannot leak into definition, references, or the existence
    /// oracle through one of them. The mask is built once per
    /// [`Self::generation`] (a [`Derived`] view); this call is then a `Vec` of
    /// borrows, the same order of cost as the `HashMap`/`HashSet` each caller
    /// already builds.
    ///
    /// The gate only fires for a source namespace the workspace can actually
    /// *observe* ([`Self::observable_namespaces`]).  `namespace import
    /// ::msgcat::mc` names a namespace that lives in an installed package, not
    /// in any indexed document: the index holds no export declaration for it
    /// and never will, so treating that silence as "not exported" would revoke
    /// a real command and hand W123 a fresh false positive on every bare `mc`
    /// call.  Absence of an export is evidence only where the definitions are
    /// visible too — otherwise this abstains and keeps the link, the same
    /// abstain-toward-silence rule the unknown-command pass follows.
    fn live_command_links(&self) -> Vec<&WorkspaceCommandLink> {
        let mask = self.live_links.get_or_build(|| {
            let wci = WildcardImportIndex::build(self);
            let observable = self.observable_namespaces();
            self.command_links()
                .map(|l| {
                    l.import_gate.as_ref().is_none_or(|g| {
                        if !observable.contains(g.source_ns.trim_start_matches("::")) {
                            return true;
                        }
                        if !wci.exports_name_at(&g.source_ns, &g.name, g.site(&l.uri)) {
                            return false;
                        }
                        // A non-`-force` import onto a name the target
                        // namespace already holds installs nothing at all
                        // (oracle on `WorkspaceGlobImport::forced`), so
                        // the link it would introduce is not live either.
                        // "Already holds" is a real *definition* or an
                        // earlier live alias from a **different** source
                        // in the same document (issue #1116 finding 4);
                        // a same-source re-import is a silent no-op, and
                        // two links cancelling each other out is what the
                        // different-source test rules out.
                        let importing_ns = importing_namespace_of(&l.linked_qname);
                        if !g.forced
                            && (self.defines_command(&l.linked_qname)
                                || wci.conflicting_alias_at(
                                    importing_ns,
                                    &g.source_ns,
                                    &g.name,
                                    g.site(&l.uri),
                                ))
                        {
                            return false;
                        }
                        // …and the alias it installed can be taken away
                        // again: `namespace forget`, a redefinition of the
                        // imported name, or destruction of the source
                        // command (issue #1116 finding 2).
                        wci.link_alias_live(importing_ns, g, &l.uri)
                    })
                })
                .collect()
        });
        self.command_links()
            .zip(mask.iter())
            .filter_map(|(link, &live)| live.then_some(link))
            .collect()
    }

    /// Whether the workspace holds a real proc or class definition at
    /// `qualified_name` — the "already exists" side of a non-`-force`
    /// `namespace import` conflict.
    ///
    /// Deliberately *not* [`Self::workspace_command_exists`], which also
    /// admits linked names: a link is what this question is being asked
    /// about, so counting one would make an import conflict with itself.
    fn defines_command(&self, qualified_name: &str) -> bool {
        let target = qualified_name.trim_start_matches("::");
        self.procs()
            .any(|p| p.qualified_name.trim_start_matches("::") == target)
            || self
                .classes()
                .any(|c| c.qualified_name.trim_start_matches("::") == target)
    }

    /// The `::`-stripped namespaces the workspace can say anything about: one
    /// that owns an indexed proc or class, that declares a `namespace export`
    /// somewhere, or that an indexed `namespace eval` block declares.
    ///
    /// The discriminator between "this namespace does not export the name"
    /// (a fact) and "this namespace is not in the workspace at all" (no
    /// information) — see [`Self::live_command_links`].
    ///
    /// The declaring-block source (issue #1088) closes a hole the first two
    /// leave: a `namespace eval ::ns { namespace import ::other::* }` block
    /// that declares no proc, class, or export of its own *is* a namespace
    /// the workspace can see, and treating it as unknown made the import gate
    /// abstain where it had the evidence to decide.
    fn observable_namespaces(&self) -> HashSet<&str> {
        fn owning_ns(qualified: &str) -> &str {
            qualified
                .trim_start_matches("::")
                .rsplit_once("::")
                .map_or("", |(ns, _)| ns)
        }
        self.procs()
            .map(|p| owning_ns(&p.qualified_name))
            .chain(self.classes().map(|c| owning_ns(&c.qualified_name)))
            .chain(
                self.namespace_exports()
                    .map(|e| e.ns.trim_start_matches("::")),
            )
            .chain(
                self.namespace_refs()
                    .filter(|n| n.declares)
                    .map(|n| n.qualified_name.trim_start_matches("::")),
            )
            .collect()
    }

    /// Add (or refresh) one document's records.
    ///
    /// Call [`Self::remove_document`] first when re-indexing a changed document
    /// to avoid stale duplicates.  Adding the **same** URI twice without an
    /// intervening removal deliberately accumulates: the M9 source-rehoming
    /// pass indexes one analysis per source-site namespace, and those views are
    /// several runtime identities of one physical file, not a replacement of
    /// each other.
    pub fn add_document(&mut self, uri: &str, analysis: &AnalysisResult) {
        self.invalidate();
        let slot = self.slot_for(uri);
        self.docs[slot].index_document(uri, analysis);
    }

    /// `uri`'s slot, allocating one — the most recently freed, else a fresh
    /// one — if it has none.
    ///
    /// The free list is LIFO so that the server's remove-then-add re-index
    /// hands the document straight back the slot it just gave up, keeping its
    /// position (and its tables' allocations) across a publish.
    fn slot_for(&mut self, uri: &str) -> usize {
        if let Some(&slot) = self.slots.get(uri) {
            return slot;
        }
        let slot = if let Some(free) = self.free_slots.pop() {
            free
        } else {
            self.docs.push(DocumentRecords::default());
            self.docs.len() - 1
        };
        self.slots.insert(uri.to_owned(), slot);
        slot
    }

    /// Whether `uri` currently has a slot in the index.
    ///
    /// Lets the server spot an **open** document whose entry is momentarily
    /// absent — `did_open` drops it and the debounced diagnostics publish is
    /// what puts it back — so a workspace-wide query can fill the gap from
    /// that document's own analysis instead of silently omitting it.
    #[must_use]
    pub fn contains_document(&self, uri: &str) -> bool {
        self.slots.contains_key(uri)
    }

    /// Drop every entry that came from `uri` (used before
    /// re-indexing a changed document, or on `did_close`).
    ///
    /// Costs that document's own records, not the workspace's: its slot is
    /// cleared and handed back to the free list (issue #1149).
    pub fn remove_document(&mut self, uri: &str) {
        self.invalidate();
        if let Some(slot) = self.slots.remove(uri) {
            self.docs[slot].clear();
            self.free_slots.push(slot);
        }
    }

    /// Every indexed `source FILE` reference.
    pub fn sources(&self) -> impl Iterator<Item = &WorkspaceSource> {
        self.docs.iter().flat_map(|doc| doc.sources.iter())
    }

    /// Every indexed `package require NAME` declaration.
    pub fn package_requires(&self) -> impl Iterator<Item = &WorkspacePackageRequire> {
        self.docs.iter().flat_map(|doc| doc.package_requires.iter())
    }

    /// Every indexed unconditional `package prefer latest`.
    pub fn package_prefers(&self) -> impl Iterator<Item = &WorkspacePackagePrefer> {
        self.docs.iter().flat_map(|doc| doc.package_prefers.iter())
    }

    /// The package names `uri` `package require`s, de-duplicated. Used to seed
    /// the workspace W120 refinement from an explicitly configured project
    /// entry file.
    #[must_use]
    pub fn package_requires_for(&self, uri: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .package_requires()
            .filter(|pr| pr.uri == uri)
            .map(|pr| pr.name.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// The union of `package require` names from every document that
    /// transitively `source`s `target_uri`.
    ///
    /// `resolve(parent_uri, raw_path)` maps a literal `source` path written in
    /// `parent_uri` to the child document's URI (the server supplies the
    /// URI ↔ path conversion); a `None` return drops that unresolvable edge.
    /// Only literal `source` targets are followed — a `source $dir/x.tcl` whose
    /// path is computed at runtime cannot be resolved statically.  The
    /// reachability walk and requires union live in
    /// [`crate::source_graph::ancestor_requires`].
    #[must_use]
    pub fn source_ancestor_package_requires(
        &self,
        target_uri: &str,
        resolve: impl Fn(&str, &str) -> Option<String>,
    ) -> Vec<String> {
        let edges: Vec<(String, String)> = self
            .sources()
            .filter(|s| s.is_literal)
            .filter_map(|s| resolve(&s.uri, &s.raw_path).map(|child| (s.uri.clone(), child)))
            .collect();
        let mut requires: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for pr in self.package_requires() {
            requires
                .entry(pr.uri.clone())
                .or_default()
                .push(pr.name.clone());
        }
        crate::source_graph::ancestor_requires(target_uri, &edges, &requires)
    }

    /// Whether the interpreter-global `package prefer latest` latch is already
    /// raised by the time `target_uri` is loaded, because a document that
    /// (transitively) `source`s it raised it first (issue #1253).
    ///
    /// `resolve` maps a literal `source` path written in a document to the
    /// child document's URI, exactly as for
    /// [`Self::source_ancestor_package_requires`]; a `None` return drops that
    /// edge, and only literal `source` targets are followed.
    ///
    /// The state is interpreter-global, so this is genuinely *this* document's
    /// answer — it just is not a fact about this document's own text.  The
    /// order rule and the two ways the latch can already be up live in
    /// [`crate::source_graph::ancestor_prefer_latest_raised`].
    #[must_use]
    pub fn source_ancestor_prefers_latest(
        &self,
        target_uri: &str,
        resolve: impl Fn(&str, &str) -> Option<String>,
    ) -> bool {
        // Nothing raises the latch anywhere: skip building the graph. This is
        // the overwhelmingly common case (the default is `stable` and most
        // workspaces never write `package prefer` at all), and it keeps a
        // per-`package require` query free.
        if self.package_prefers().next().is_none() {
            return false;
        }
        let edges: Vec<crate::source_graph::SourceEdge> = self
            .sources()
            .filter(|s| s.is_literal)
            .filter_map(|s| {
                resolve(&s.uri, &s.raw_path).map(|child| crate::source_graph::SourceEdge {
                    parent: s.uri.clone(),
                    child,
                    at: s.range.start(),
                    enclosing_body: s.enclosing_body,
                })
            })
            .collect();
        let mut raises: std::collections::HashMap<String, Vec<u32>> =
            std::collections::HashMap::new();
        for p in self.package_prefers() {
            raises.entry(p.uri.clone()).or_default().push(p.at);
        }
        crate::source_graph::ancestor_prefer_latest_raised(target_uri, &edges, &raises)
    }

    /// The **source-site namespace seeds** per sourced document (M9): for
    /// every `source` statement `resolve` can place (the closure maps
    /// `(parent-uri, raw-path, is_literal)` to the child's URI — handling
    /// relative literals and, for stage 9.2, statically-foldable computed
    /// paths), the child URI maps to the set of namespaces it is sourced
    /// under.  `source` runs the file in the caller's namespace, so a child
    /// sourced from `namespace eval ::x` must be (re-)analysed seeded with
    /// `::x` for the index to hold its true runtime names.
    ///
    /// Transitivity needs no composition here: once a child's *seeded*
    /// analysis is merged into the index, its own recorded `source` sites
    /// already carry the composed namespace.
    #[must_use]
    pub fn source_seed_map(
        &self,
        resolve: impl Fn(&str, &str, bool) -> Option<String>,
    ) -> std::collections::HashMap<String, std::collections::BTreeSet<String>> {
        let mut out: std::collections::HashMap<String, std::collections::BTreeSet<String>> =
            std::collections::HashMap::new();
        for src in self.sources() {
            let Some(child) = resolve(&src.uri, &src.raw_path, src.is_literal) else {
                continue;
            };
            // Self-sourcing carries no new namespace view.
            if child == src.uri {
                continue;
            }
            let seed = if src.site_namespace.is_empty() {
                "::".to_owned()
            } else {
                src.site_namespace.clone()
            };
            out.entry(child).or_default().insert(seed);
        }
        out
    }

    /// Whether any `source` statement is indexed at all — the cheap guard
    /// that lets the server skip the M9 re-homing pass entirely for
    /// workspaces that never `source`.
    #[must_use]
    pub fn has_source_edges(&self) -> bool {
        self.sources().next().is_some()
    }

    /// The workspace's symbols matching `query`, at most `limit` of them —
    /// the whole `workspace/symbol` answer (issue #1156).
    ///
    /// Every indexed document is searched, not only the ones the editor has
    /// open, so a symbol in a file the user has never opened is reachable from
    /// the picker.  The index is refreshed on each document's diagnostics
    /// publish (~50 ms after an edit), which is the freshness a symbol picker
    /// gets: a name typed a moment ago appears once that publish lands.  It is
    /// the same staleness every other cross-document feature answers with, and
    /// the alternative — re-analysing every open buffer per keystroke in the
    /// Ctrl+T box — is what this replaces.
    ///
    /// The scan is **document-major**: each document contributes its procs,
    /// then its classes (with their methods and constructors), then its
    /// registry symbol-definer definitions, before the next document is
    /// looked at.  So a `limit`-truncated answer is a prefix of the workspace
    /// rather than a prefix of one symbol table — a capped result still holds
    /// classes and test cases, not only procs — and results arrive grouped by
    /// URI, which lets the caller resolve each document's source once.
    #[must_use]
    pub fn symbols_matching(&self, query: &str, limit: usize) -> Vec<IndexedWorkspaceSymbol> {
        let lower_query = query.to_lowercase();
        let mut out: Vec<IndexedWorkspaceSymbol> = Vec::new();
        // Built once for the whole scan — the class-member walk needs the
        // workspace's cross-document retractions, which no single document's
        // records can answer for themselves (issue #1263).
        let retractions = self.retraction_index();
        for doc in &self.docs {
            if out.len() >= limit {
                break;
            }
            doc.collect_symbols_matching(&lower_query, limit, &retractions, &mut out);
        }
        out
    }

    /// Every indexed proc.
    pub fn procs(&self) -> impl Iterator<Item = &WorkspaceProc> {
        self.docs.iter().flat_map(|doc| doc.procs.iter())
    }

    /// Every indexed class.
    pub fn classes(&self) -> impl Iterator<Item = &WorkspaceClass> {
        self.docs.iter().flat_map(|doc| doc.classes.iter())
    }

    /// Every indexed `namespace import` / `interp alias` / `rename` link.
    pub fn command_links(&self) -> impl Iterator<Item = &WorkspaceCommandLink> {
        self.docs.iter().flat_map(|doc| doc.command_links.iter())
    }

    /// Workspace classes whose qualified name matches `name` exactly or via
    /// the leading-`::` normalisation (`Animal` ↔ `::Animal`).  Used to
    /// resolve the class **at the cursor**, whose name arrives already
    /// qualified.
    ///
    /// Deliberately does *not* fall back to a bare simple-name (tail) match:
    /// superclass / mixin names are namespace-relative in Tcl, so an
    /// ownerless tail match (`Base` → `::Other::Base`) could manufacture a
    /// wrong cross-file link.  Owner-aware resolution of written super/mixin
    /// names is done by [`Self::supertype_classes`] / [`Self::subclasses_of`]
    /// via [`resolve_class_name`], which walks the defining class's
    /// namespace ancestry before considering a *unique* tail.
    #[must_use]
    pub fn classes_named<'a>(&'a self, name: &str) -> Vec<&'a WorkspaceClass> {
        let q = format!("::{}", name.trim_start_matches("::"));
        self.classes()
            .filter(|c| c.qualified_name == name || c.qualified_name == q)
            .collect()
    }

    /// The `(qualified-name set, tail index)` over every indexed class —
    /// the inputs [`resolve_class_name`] needs, built once per query so
    /// owner-aware resolution is O(1) membership rather than a linear scan
    /// per candidate.
    fn class_name_universe(
        &self,
    ) -> (
        std::collections::HashSet<&str>,
        std::collections::HashMap<String, Vec<String>>,
    ) {
        let known: std::collections::HashSet<&str> =
            self.classes().map(|c| c.qualified_name.as_str()).collect();
        let tail_index = build_tail_index(self.classes().map(|c| &c.qualified_name));
        (known, tail_index)
    }

    /// The owner-aware direct parents (superclasses + mixins) of `qname`,
    /// unioned across **every** indexed definition of the class.  A cross-file
    /// `oo::define ::C { ... }` records a second `::C` entry that names no
    /// `superclass`; unioning here keeps the real class's parent edges from
    /// being hidden when such a stub happens to be the first match (the parent
    /// walk otherwise picked an arbitrary duplicate and silently dropped the
    /// hierarchy).
    fn resolved_parents_of(
        &self,
        qname: &str,
        known: &std::collections::HashSet<&str>,
        tail_index: &std::collections::HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for c in self.classes().filter(|c| c.qualified_name == qname) {
            for s in c.superclasses.iter().chain(c.mixins.iter()) {
                if let Some(p) =
                    resolve_class_name(s, qname, |cand| known.contains(cand), tail_index)
                    && seen.insert(p.clone())
                {
                    out.push(p);
                }
            }
        }
        out
    }

    /// The workspace classes that `wc`'s written superclasses + mixins
    /// resolve to, **owner-aware** — each name is resolved relative to
    /// `wc.qualified_name`'s namespace (ancestry → global → unique tail) via
    /// [`resolve_class_name`], never by a bare global tail guess.  Used for
    /// cross-file **supertype** resolution.
    #[must_use]
    pub fn supertype_classes<'a>(&'a self, wc: &WorkspaceClass) -> Vec<&'a WorkspaceClass> {
        let (known, tail_index) = self.class_name_universe();
        let mut out: Vec<&WorkspaceClass> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for name in wc.superclasses.iter().chain(wc.mixins.iter()) {
            let Some(q) = resolve_class_name(
                name,
                &wc.qualified_name,
                |cand| known.contains(cand),
                &tail_index,
            ) else {
                continue;
            };
            if !seen.insert(q.clone()) {
                continue;
            }
            out.extend(self.classes().filter(|c| c.qualified_name == q));
        }
        out
    }

    /// Workspace classes that declare `class_qname` as a direct superclass
    /// or mixin, resolving each written super/mixin name **owner-aware**
    /// (relative to the declaring class) so an ambiguous bare name never
    /// manufactures a subtype edge.  Used for cross-file **subtype**
    /// resolution.
    #[must_use]
    pub fn subclasses_of<'a>(&'a self, class_qname: &str) -> Vec<&'a WorkspaceClass> {
        let (known, tail_index) = self.class_name_universe();
        self.classes()
            .filter(|c| {
                c.superclasses.iter().chain(c.mixins.iter()).any(|s| {
                    resolve_class_name(
                        s,
                        &c.qualified_name,
                        |cand| known.contains(cand),
                        &tail_index,
                    )
                    .as_deref()
                        == Some(class_qname)
                })
            })
            .collect()
    }

    /// The **class linearisation** of `class_q` — the order `TclOO` searches
    /// classes for a method implementation (mixins fully linearised first,
    /// then the class, then superclasses; diamond duplicates keep their
    /// late placement — tclsh 9.0.4-pinned via `info object call`).
    ///
    /// A thin workspace adapter over the canonical
    /// [`tcl_syntax::mro::tcloo_linearise`]: the super / mixin edges are
    /// resolved **owner-aware** ([`resolve_class_name`]) and unioned
    /// across every indexed record of each class (an `oo::define` stub
    /// must not hide the creation site's edges).  Empty when the
    /// hierarchy is cyclic or too complex to linearise (the shared
    /// budget guard) — consumers abstain rather than guess.
    #[must_use]
    pub fn class_linearisation(&self, class_q: &str) -> Vec<String> {
        let (known, tail_index) = self.class_name_universe();
        // Build the resolved edge maps over the classes reachable from
        // `class_q` (bounded: every indexed class at worst).
        let mut supers_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut mixins_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for c in self.classes() {
            let owner = c.qualified_name.as_str();
            let resolve = |name: &str| {
                resolve_class_name(name, owner, |cand| known.contains(cand), &tail_index)
            };
            let supers = supers_map.entry(owner.to_owned()).or_default();
            for s in c.superclasses.iter().filter_map(|s| resolve(s)) {
                if !supers.contains(&s) {
                    supers.push(s);
                }
            }
            let mixins = mixins_map.entry(owner.to_owned()).or_default();
            for m in c.mixins.iter().filter_map(|m| resolve(m)) {
                if !mixins.contains(&m) {
                    mixins.push(m);
                }
            }
        }
        tcl_syntax::mro::tcloo_linearise(class_q, &supers_map, &mixins_map).unwrap_or_default()
    }

    /// The C-Tcl-faithful **method dispatch chain** for an instance of
    /// `receiver_class` calling `method` under `access` (issue #945
    /// faults 4 and 6): the linearisation's classes that define an
    /// instance-receiver implementation of `method`, in dispatch order,
    /// visibility-filtered —
    ///
    /// * [`MethodAccess::External`] keeps only **exported**
    ///   implementations (an unexported method is not externally
    ///   callable: `unknown method`, tclsh 9.0.4-pinned);
    /// * [`MethodAccess::Internal`] keeps exported + unexported;
    ///   `private` definitions are visible only when the defining class
    ///   *is* the receiver's own class (`TclOO` private scoping).
    ///
    /// The **first** record is the implementation the call actually
    /// enters — go-to-definition's single target; the rest is the `next`
    /// chain.  Several records of one class (creation site + `oo::define`
    /// stubs) are all kept, adjacent, when each defines the method; a
    /// class exported/unexported by a *different* record than the definer
    /// honours the union of that class's records (last state is
    /// load-order-dependent across files, so any exporting record keeps
    /// the method dispatchable — the navigation-permissive reading).
    #[must_use]
    pub fn method_dispatch_chain<'a>(
        &'a self,
        receiver_class: &str,
        method: &str,
        access: MethodAccess,
    ) -> Vec<&'a WorkspaceClass> {
        self.dispatch_chain(receiver_class, method, access, MemberSide::Instance)
    }

    /// The dispatch chain for a call on the **class's own command**
    /// (`::C cm`) rather than on an instance — the class-object-side twin of
    /// [`Self::method_dispatch_chain`] (issue #1119).
    ///
    /// Same rules, read against the other side's tables: `class_method`
    /// declarations instead of `instance_method` ones,
    /// [`WorkspaceClass::class_exports`] / [`WorkspaceClass::class_unexports`]
    /// instead of the instance pair, and [`MemberSide::ClassObject`]
    /// tombstones.  Without it a `self unexport m` was invisible across files:
    /// the flip had no channel, so the workspace kept resolving a `::C m` the
    /// interpreter answers with `unknown method "m"` (tclsh 9.0.4 / 8.6.14).
    ///
    /// One `TclOO` rule is class-side only: a stock `self method` lives on the
    /// class object that declared it and a **subclass's** class command never
    /// reaches it (`Gadget make` against a parent's `self method make` errors
    /// `unknown method "make"` on 8.6 and 9.0.4), whereas an `ooutil`
    /// `classmethod` does propagate.  Both share the `"classmethod"` receiver
    /// kind, so a `self method` is kept only when the providing record *is* the
    /// receiver class — the same test
    /// `tcl_lsp_core::oo_dispatch::method_dispatch_provider` applies
    /// same-document.
    #[must_use]
    pub fn class_method_dispatch_chain<'a>(
        &'a self,
        receiver_class: &str,
        method: &str,
        access: MethodAccess,
    ) -> Vec<&'a WorkspaceClass> {
        self.dispatch_chain(receiver_class, method, access, MemberSide::ClassObject)
    }

    /// Whether the workspace's **class-side visibility union** leaves no
    /// dispatchable implementation of `method` on `class_q`'s own command
    /// under `access` — the *suppression* half of the class-side channel,
    /// consulted by the in-document tier before it answers for a class its
    /// own document declares (issue #1168).
    ///
    /// The revival direction already crossed files: a `self export` written
    /// next door reaches every query through
    /// [`Self::class_method_dispatch_chain`].  Suppression did not reach the
    /// *declaring* document, because its in-document provider resolves the
    /// member locally and returns before the workspace chain runs — so a
    /// cross-file `self unexport Cm` / `self deletemethod Cm` suppressed
    /// `C Cm` for every document except the one the author is editing.  This
    /// predicate is the missing consultation, and it is deliberately a thin
    /// reading of the same chain fold the cross-file tier resolves through —
    /// one decision function, so the two tiers cannot diverge (the
    /// established `exported_at_import_site` pattern).
    ///
    /// `false` is an abstention as well as a "not suppressed": a class the
    /// index holds no record of, or whose hierarchy the shared linearisation
    /// declines (cycle / budget), yields no suppression evidence, and the
    /// in-document answer stands.  The chain itself carries the standing
    /// unordered-cross-file caveat — any exporting record keeps the member
    /// dispatchable — so an unordered flip pair abstains toward answering.
    #[must_use]
    pub fn class_member_dispatch_suppressed(
        &self,
        class_q: &str,
        method: &str,
        access: MethodAccess,
    ) -> bool {
        if !self.classes().any(|c| c.qualified_name == class_q) {
            return false;
        }
        if self.class_linearisation(class_q).is_empty() {
            return false;
        }
        self.class_method_dispatch_chain(class_q, method, access)
            .is_empty()
    }

    /// The one dispatch-chain walk both sides go through — see
    /// [`Self::method_dispatch_chain`] for the rules and
    /// [`Self::class_method_dispatch_chain`] for what the class side changes.
    ///
    /// Sharing the walk is what keeps the two sides' answers consistent: the
    /// record ordering, the retraction gate, the effective-export union and the
    /// visibility filter are decided once, and `side` only selects *which*
    /// table each of them reads.
    fn dispatch_chain<'a>(
        &'a self,
        receiver_class: &str,
        method: &str,
        access: MethodAccess,
        side: MemberSide,
    ) -> Vec<&'a WorkspaceClass> {
        let mut out: Vec<&WorkspaceClass> = Vec::new();
        for class_q in self.class_linearisation(receiver_class) {
            // Several records of one class (its creation site plus every
            // `oo::define` stub, possibly spread over files) are all kept, and
            // the *first* is what go-to-definition answers with — so the order
            // has to be a property of the workspace, not of when each document
            // happened to be indexed.  Document URI then source position is
            // that stable order (issue #1028).
            let records = self.class_records(&class_q);
            // The class-level effective export union **for this side**: any
            // record exporting the name keeps it callable; explicit unexports
            // matter only when no record exports it.  The two sides never share
            // a set — a `self unexport m` must not silence an identically-named
            // instance method, and vice versa (issue #1098/#1119).
            let any_exports = records
                .iter()
                .any(|c| visibility_sets_for(c, side).0.iter().any(|e| e == method));
            let any_unexports = records
                .iter()
                .any(|c| visibility_sets_for(c, side).1.iter().any(|e| e == method));
            // The class's member set — retractions applied, arrivals
            // re-keyed — is decided once by [`Self::effective_members`]
            // rather than here (issue #1263).  A `deletemethod`ed member is
            // absent from it, so the chain for that name is empty exactly as
            // it is for a method no record defines; a `renamemethod`ed one is
            // present under its destination and absent under its source.
            for em in self.effective_members(&class_q) {
                if em.name != method || method_side(em.method) != side {
                    continue;
                }
                // A stock `self method` is not inherited: the class object
                // that declared it is the only one whose command reaches it
                // (`Gadget make` against a parent's `self method make` ->
                // `unknown method "make"`, 8.6 / 9.0.4). An `ooutil`
                // `classmethod` shares the receiver kind but does propagate.
                if side == MemberSide::ClassObject
                    && em.method.is_self_method
                    && em.declaring.qualified_name != receiver_class
                {
                    continue;
                }
                // Effective export across the class's records: an explicit
                // `export` anywhere wins, else an explicit `unexport`
                // anywhere, else the definer's own effective state.  (True
                // cross-file order is load-order; explicit-export-wins is the
                // navigation-permissive reading.)  Visibility travels with the
                // *body*, not an arrival name's leading-capital default
                // (oracle, tclsh 9.0.4 / 8.6.14: `oo::class create ::R4
                // {method Priv {} {…}; renamemethod Priv pub}` leaves `info
                // class methods ::R4` empty while `-private` lists `pub`), so
                // the source member's own record is what this reads.
                let exported = if any_exports {
                    true
                } else if any_unexports {
                    false
                } else {
                    em.method.exported
                };
                let visible = match access {
                    MethodAccess::External => exported && !em.method.private,
                    MethodAccess::Internal => !em.method.private || class_q == receiver_class,
                };
                if visible {
                    out.push(em.declaring);
                }
            }
        }
        out
    }

    /// Every record of the class `class_q`, in the workspace's stable order.
    ///
    /// Several records of one class (its creation site plus every
    /// `oo::define` stub, possibly spread over files) are all kept, and the
    /// *first* is what go-to-definition answers with — so the order has to be
    /// a property of the workspace, not of when each document happened to be
    /// indexed.  Document URI then source position is that order (issue
    /// #1028).
    #[must_use]
    pub fn class_records<'a>(&'a self, class_q: &str) -> Vec<&'a WorkspaceClass> {
        let mut records: Vec<&WorkspaceClass> = self
            .classes()
            .filter(|c| c.qualified_name == class_q)
            .collect();
        records.sort_by_key(|c| (c.uri.as_str(), c.name_span.start(), c.name_span.end()));
        records
    }

    /// The members of `class_q` as the workspace sees them: every record's own
    /// declarations, with the class's cross-document retractions applied and
    /// its arrivals re-keyed (issue #1263).  Both receiver sides, in
    /// [`Self::class_records`] order then declaration order — filter on
    /// [`EffectiveMember::method`]'s `kind` (via `WorkspaceClass`'s own
    /// instance/class split) for one side.
    ///
    /// This is the single rule for "which members does this class have":
    /// [`Self::dispatch_chain`] resolves one name against it and every
    /// *enumeration* (`workspace/symbol`, an outline, a member completion
    /// universe) lists it, so the two can no longer disagree.  Before this,
    /// resolution joined the arrival channel and listing did not, so a member
    /// moved by a cross-file `renamemethod` was still enumerated under its
    /// pre-rename name.
    ///
    /// Inheritance is **not** applied: these are the class's own members.
    /// Walk [`Self::class_linearisation`] for the inherited set.
    ///
    /// Carries the tombstone channel's unordered-cross-file caveat: true load
    /// order is not knowable from the index, so a retraction recorded by any
    /// record of the class applies to every record of it.
    #[must_use]
    pub fn effective_members<'a>(&'a self, class_q: &str) -> Vec<EffectiveMember<'a>> {
        let retractions = self.retraction_index();
        let mut out: Vec<EffectiveMember<'a>> = Vec::new();
        for declaring in self.class_records(class_q) {
            out.extend(
                declaring
                    .methods
                    .iter()
                    .filter_map(|method| effective_member(&retractions, declaring, method)),
            );
        }
        out
    }

    /// Every cross-document member retraction in the workspace, keyed by the
    /// `(class, member, side)` it removes — the lookup table the member fold
    /// ([`effective_member`]) applies.
    ///
    /// Built once per query rather than rescanned per member: a retraction
    /// recorded by *any* record of a class applies to every record of it, so
    /// the naive form is a scan of the class's records per declared method,
    /// which is quadratic in a workspace's class count on a path
    /// (`workspace/symbol`) that runs per keystroke.  The map is empty in the
    /// overwhelmingly common case — no document retracts anything — and the
    /// walk is then exactly what it was before the fold existed.
    ///
    /// Ties are broken by the workspace's stable record order
    /// ([`Self::class_records`]), so which stub an arrival is attributed to
    /// does not depend on indexing order.
    fn retraction_index(&self) -> RetractionIndex<'_> {
        let mut records: Vec<&WorkspaceClass> = self
            .classes()
            .filter(|c| !c.retracted_members.is_empty())
            .collect();
        if records.is_empty() {
            return RetractionIndex::default();
        }
        records.sort_by_key(|c| (c.uri.as_str(), c.name_span.start(), c.name_span.end()));
        let mut out = RetractionIndex::default();
        for c in records {
            for r in &c.retracted_members {
                out.entry((c.qualified_name.as_str(), r.member.as_str(), r.side))
                    .or_insert((c, r));
            }
        }
        out
    }

    /// The **cross-file override family** of `method` seeded at
    /// `seed_class`: every indexed class that directly defines `method` and
    /// sits in the same subtype-connected component as `seed_class` (or the
    /// ancestor that provides `method` to it).
    ///
    /// This is the workspace-wide analogue of the single-document override
    /// family used by method rename: a method (re)defined up or down the
    /// hierarchy is one polymorphic name, so renaming it must touch every
    /// class that defines it across the whole workspace.  Superclass/mixin
    /// edges are resolved **owner-aware** (via [`resolve_class_name`]), so an
    /// ambiguous bare parent name never fabricates a connection.  The
    /// returned set always includes the seed's provider and is empty only
    /// when `method` is neither defined nor inherited from any indexed
    /// class reachable from `seed_class`.
    #[must_use]
    pub fn method_override_family<'a>(
        &'a self,
        seed_class: &str,
        method: &str,
    ) -> Vec<&'a WorkspaceClass> {
        let family = self.method_family_qnames(seed_class, method);
        let family_set: std::collections::HashSet<&str> =
            family.iter().map(String::as_str).collect();
        self.classes()
            .filter(|c| family_set.contains(c.qualified_name.as_str()))
            .collect()
    }

    /// Indexed classes that **inherit** `method` from the override family of
    /// `(seed_class, method)` but do not define it themselves — the pure
    /// inheritors whose `my method` / `$obj method` sites a rename must also
    /// rewrite, even though they contribute no declaration.
    ///
    /// A class is included only when it inherits `method` (some ancestor
    /// defines it) **and every** method-defining ancestor it can reach is in
    /// the family.  That keeps the result sound under multiple inheritance:
    /// if a class could resolve `method` to a definer *outside* the family
    /// (a disjoint same-named method), it is abstained on rather than risk an
    /// over-rename.  The workspace index carries ancestry but not a full
    /// cross-file MRO, so this is deliberately conservative.
    #[must_use]
    pub fn method_inheritor_classes<'a>(
        &'a self,
        seed_class: &str,
        method: &str,
    ) -> Vec<&'a WorkspaceClass> {
        let family = self.method_family_qnames(seed_class, method);
        if family.is_empty() {
            return Vec::new();
        }
        let family_set: std::collections::HashSet<&str> =
            family.iter().map(String::as_str).collect();
        let (known, tail_index) = self.class_name_universe();
        let parents = |qname: &str| self.resolved_parents_of(qname, &known, &tail_index);
        let defines = |qname: &str| {
            self.classes()
                .any(|c| c.qualified_name == qname && c.defines_method(method))
        };
        self.classes()
            .filter(|c| {
                // A definer is handled by the family itself, not here.
                if c.defines_method(method) || family_set.contains(c.qualified_name.as_str()) {
                    return false;
                }
                // Every method-defining ancestor this class can reach.
                let mut stack = parents(&c.qualified_name);
                let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
                let mut defining_ancestors: Vec<String> = Vec::new();
                while let Some(p) = stack.pop() {
                    if seen.insert(p.clone()) {
                        if defines(&p) {
                            defining_ancestors.push(p.clone());
                        }
                        stack.extend(parents(&p));
                    }
                }
                // Inherits `method` (has a definer ancestor) and cannot resolve
                // it to a definer outside the family.
                !defining_ancestors.is_empty()
                    && defining_ancestors
                        .iter()
                        .all(|a| family_set.contains(a.as_str()))
            })
            .collect()
    }

    /// The qualified names of the override family of `(seed_class, method)`:
    /// every indexed class that directly defines `method` and sits in the
    /// same subtype-connected component as `seed_class` (or the ancestor that
    /// provides `method` to it).  Shared by [`Self::method_override_family`]
    /// and [`Self::method_inheritor_classes`].  Empty when `method` is neither
    /// defined nor inherited from any indexed class reachable from
    /// `seed_class`.
    fn method_family_qnames(&self, seed_class: &str, method: &str) -> Vec<String> {
        let (known, tail_index) = self.class_name_universe();
        // Owner-aware direct parents (superclasses + mixins) of each class,
        // unioned across every indexed definition (a cross-file `oo::define`
        // stub must not hide the real class's parents).
        let parents = |qname: &str| self.resolved_parents_of(qname, &known, &tail_index);
        // `parent` is a (transitive) ancestor of `child`.
        let is_ancestor = |child: &str, parent: &str| -> bool {
            let mut stack = parents(child);
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            while let Some(p) = stack.pop() {
                if p == parent {
                    return true;
                }
                if seen.insert(p.clone()) {
                    stack.extend(parents(&p));
                }
            }
            false
        };
        let connected = |a: &str, b: &str| a == b || is_ancestor(a, b) || is_ancestor(b, a);
        // A member that *arrived* through a cross-file `renamemethod` counts
        // as defined for family purposes: the class really has it, the
        // declaring record just spells it under the source name (issue #1167).
        let class_defines = |qname: &str| {
            self.classes().any(|c| {
                c.qualified_name == qname && (c.defines_method(method) || c.arrives_method(method))
            })
        };
        // Seed: the class under the cursor if it defines `method`, else the
        // nearest ancestor that does (any definer ancestor is in the same
        // family, so the first one found seeds it).
        let seed = if class_defines(seed_class) {
            seed_class.to_string()
        } else {
            let mut stack = parents(seed_class);
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut found = None;
            while let Some(p) = stack.pop() {
                if class_defines(&p) {
                    found = Some(p);
                    break;
                }
                if seen.insert(p.clone()) {
                    stack.extend(parents(&p));
                }
            }
            match found {
                Some(p) => p,
                None => return Vec::new(),
            }
        };
        // Every indexed definer of `method` (qualified names, de-duplicated).
        let definers: Vec<String> = {
            let mut ds: Vec<String> = self
                .classes()
                .filter(|c| c.defines_method(method) || c.arrives_method(method))
                .map(|c| c.qualified_name.clone())
                .collect();
            ds.sort();
            ds.dedup();
            ds
        };
        // Grow the weakly-connected component of definers containing `seed`.
        let mut family = vec![seed];
        let mut changed = true;
        while changed {
            changed = false;
            for d in &definers {
                if family.iter().any(|f| f == d) {
                    continue;
                }
                if family.iter().any(|f| connected(f, d)) {
                    family.push(d.clone());
                    changed = true;
                }
            }
        }
        family
    }

    /// Procs whose simple *or* qualified name starts with
    /// `prefix`, excluding any defined in `exclude_uri` (the
    /// caller's current document, whose procs the single-doc
    /// provider already surfaces).  Empty `prefix` matches all.
    #[must_use]
    pub fn procs_matching<'a>(&'a self, prefix: &str, exclude_uri: &str) -> Vec<&'a WorkspaceProc> {
        self.procs()
            .filter(|p| p.uri != exclude_uri)
            .filter(|p| {
                prefix.is_empty()
                    || p.name.starts_with(prefix)
                    || p.qualified_name.starts_with(prefix)
            })
            .collect()
    }

    /// Proc definitions matching `name` (simple, qualified, or
    /// `::`-prefixed simple form), excluding `exclude_uri`.
    /// Used by cross-document go-to-definition: when the
    /// current document has no matching proc, the index
    /// resolves one defined elsewhere.
    #[must_use]
    pub fn proc_definitions<'a>(&'a self, name: &str, exclude_uri: &str) -> Vec<&'a WorkspaceProc> {
        let qualified = format!("::{name}");
        self.procs()
            .filter(|p| p.uri != exclude_uri)
            .filter(|p| p.name == name || p.qualified_name == name || p.qualified_name == qualified)
            .collect()
    }

    /// Class definitions matching `name`, excluding
    /// `exclude_uri`.
    #[must_use]
    pub fn class_definitions<'a>(
        &'a self,
        name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceClass> {
        let qualified = format!("::{name}");
        self.classes()
            .filter(|c| c.uri != exclude_uri)
            .filter(|c| c.name == name || c.qualified_name == name || c.qualified_name == qualified)
            .collect()
    }

    /// Proc definitions whose **fully-qualified** name equals `qualified_name`
    /// (leading `::` ignored), excluding `exclude_uri`.
    ///
    /// This is the correct matcher for cross-document **rename**: a proc in
    /// another file is the *same* proc only when its qualified name matches, so
    /// renaming `::a::helper` must not touch a `proc helper` inside
    /// `namespace eval ::b` (whose qualified name is `::b::helper`). The looser
    /// [`Self::proc_definitions`] matches by simple name for go-to-definition
    /// and must not be reused here (`RUST_ISSUE_036`).
    #[must_use]
    pub fn proc_definitions_qualified<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceProc> {
        let target = qualified_name.trim_start_matches("::");
        self.procs()
            .filter(|p| p.uri != exclude_uri)
            .filter(|p| p.qualified_name.trim_start_matches("::") == target)
            .collect()
    }

    /// Class definitions whose fully-qualified name equals `qualified_name`
    /// (leading `::` ignored), excluding `exclude_uri`. The class analogue of
    /// [`Self::proc_definitions_qualified`] for cross-document rename.
    #[must_use]
    pub fn class_definitions_qualified<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceClass> {
        let target = qualified_name.trim_start_matches("::");
        self.classes()
            .filter(|c| c.uri != exclude_uri)
            .filter(|c| c.qualified_name.trim_start_matches("::") == target)
            .collect()
    }

    /// Every indexed invocation site.
    pub fn invocations(&self) -> impl Iterator<Item = &WorkspaceInvocation> {
        self.docs.iter().flat_map(|doc| doc.invocations.iter())
    }

    /// Every indexed namespace-qualified variable declaration.
    pub fn variables(&self) -> impl Iterator<Item = &WorkspaceVariable> {
        self.docs.iter().flat_map(|doc| doc.variables.iter())
    }

    /// Declaration sites of the namespace variable whose `::`-rooted
    /// qualified name is `qualified_name`, excluding any in `exclude_uri`
    /// (pass `""` to exclude nothing).
    ///
    /// Exact-name matching only — a namespace variable's qualified name
    /// names exactly one cell in real Tcl (`$other::v` never searches
    /// enclosing namespaces or falls back to global), so there is no
    /// candidate list to walk and no simple-name fallback to get wrong.
    /// One reopened namespace can be declared across several files, so the
    /// result is a set, not an `Option`.
    #[must_use]
    pub fn variable_definitions_qualified<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceVariable> {
        let target = qualified_name.trim_start_matches("::");
        self.variables()
            .filter(|v| v.uri != exclude_uri)
            .filter(|v| v.qualified_name.trim_start_matches("::") == target)
            .collect()
    }

    /// Every document holding an indexed symbol declared **directly in**
    /// `namespace` — a proc, a class, or a namespace variable whose parent
    /// namespace is exactly it.
    ///
    /// One of the three candidate sources a namespace-variable *rename* must
    /// visit, and the one that catches a document whose only stake in the
    /// cell is an unqualified alias written *inside* the namespace
    /// (`namespace eval ns { proc p {} { variable v; puts $v } }`): that
    /// binding is proc-scope and deliberately not in either variable table,
    /// but the enclosing `proc` is indexed as `::ns::p`.
    ///
    /// It is **not** sufficient on its own.  An alias can be written from any
    /// namespace — a global `proc p {} { namespace upvar ::ns v local; … }`
    /// declares nothing in `::ns` at all — which is what
    /// [`Self::documents_aliasing_variable`] answers.  Both are needed;
    /// neither subsumes the other.
    #[must_use]
    pub fn documents_in_namespace(&self, namespace: &str) -> Vec<String> {
        let target = namespace.trim_start_matches("::");
        let parent_matches = |qualified: &str| -> bool {
            let bare = qualified.trim_start_matches("::");
            match bare.rfind("::") {
                Some(idx) => &bare[..idx] == target,
                None => target.is_empty(),
            }
        };
        let mut uris: Vec<String> = self
            .procs()
            .filter(|p| parent_matches(&p.qualified_name))
            .map(|p| p.uri.clone())
            .chain(
                self.classes()
                    .filter(|c| parent_matches(&c.qualified_name))
                    .map(|c| c.uri.clone()),
            )
            .chain(
                self.variables()
                    .filter(|v| parent_matches(&v.qualified_name))
                    .map(|v| v.uri.clone()),
            )
            .collect();
        uris.sort();
        uris.dedup();
        uris
    }

    /// Every document holding a local **alias** of the cell `qualified_name`
    /// — a `variable v` / `global ::ns::v` / `namespace upvar ::ns v local` /
    /// `upvar #0 ::ns::v local`, written from any scope in any namespace.
    ///
    /// The candidate source that makes a namespace-variable rename's coverage
    /// provable rather than presumed.  A document can bind `::ns::v` without
    /// declaring anything in `::ns` and without writing a single qualified
    /// occurrence of it, so it appears in neither variable table and in no
    /// namespace listing; renaming the cell while leaving that document
    /// unvisited leaves the alias bound to a cell that no longer exists.
    ///
    /// Matched the same exact way as the other two — one cell, one name, no
    /// scope-chain search (see [`Self::variable_definitions_qualified`]).
    /// Aliases whose cell is *computed* name no fixed cell and so match
    /// nothing here; they are [`Self::documents_with_ambiguous_alias_of`]'s
    /// business instead.
    #[must_use]
    pub fn documents_aliasing_variable(&self, qualified_name: &str) -> Vec<String> {
        let target = qualified_name.trim_start_matches("::");
        let mut uris: Vec<String> = self
            .variable_aliases()
            .filter(|a| !alias_cell_is_computed(&a.qualified_name))
            .filter(|a| a.qualified_name.trim_start_matches("::") == target)
            .map(|a| a.uri.clone())
            .collect();
        uris.sort();
        uris.dedup();
        uris
    }

    /// Every document holding an alias whose cell is **computed** and could
    /// therefore be `qualified_name` — `namespace upvar $ns version local`,
    /// `namespace upvar ::ns $v local`.
    ///
    /// The completeness proof for [`Self::documents_aliasing_variable`].  A
    /// computed cell names no fixed variable, so no candidate scan can find
    /// the alias and no edit can keep it consistent; renaming the cell anyway
    /// leaves it bound to a variable that no longer exists (tclsh 9.0.4 /
    /// 8.6.16 alike: `namespace eval mypkg { variable version 1 }` +
    /// `namespace upvar $ns version local` prints `1`, and renaming
    /// `version` to `release` gives `can't read "local": no such variable`).
    /// A rename whose cell this could be is refused.
    ///
    /// The match is on whichever half is still written literally, so this
    /// stays narrow: a computed *namespace* with a literal tail can only be
    /// this cell if the tails agree, and a computed *tail* under a literal
    /// namespace only if the namespaces do.  Both computed matches anything
    /// in scope.
    #[must_use]
    pub fn documents_with_ambiguous_alias_of(&self, qualified_name: &str) -> Vec<String> {
        let bare = qualified_name.trim_start_matches("::");
        let (cell_ns, cell_tail) = bare.rsplit_once("::").unwrap_or(("", bare));
        let mut uris: Vec<String> = self
            .variable_aliases()
            .filter(|a| alias_cell_is_computed(&a.qualified_name))
            .filter(|a| {
                let written = a.qualified_name.trim_start_matches("::");
                let (ns, tail) = written.rsplit_once("::").unwrap_or(("", written));
                let ns_could_match = is_computed_word(ns) || ns == cell_ns;
                let tail_could_match = is_computed_word(tail) || tail == cell_tail;
                ns_could_match && tail_could_match
            })
            .map(|a| a.uri.clone())
            .collect();
        uris.sort();
        uris.dedup();
        uris
    }

    /// Every indexed namespace-name occurrence.
    pub fn namespace_refs(&self) -> impl Iterator<Item = &WorkspaceNamespaceRef> {
        self.docs.iter().flat_map(|doc| doc.namespace_refs.iter())
    }

    /// **Declaring** sites of the namespace `qualified_name` — the name word
    /// of each `namespace eval` block that creates or extends it — excluding
    /// any in `exclude_uri` (pass `""` to exclude nothing).
    ///
    /// A set, not an `Option`, and for a stronger reason than the variable
    /// tier's: reopening a namespace is the *normal* way to build one, and
    /// tclsh 9.0.4 / 8.6.16 agree byte-for-byte that two `namespace eval ::a
    /// {}` blocks are one namespace (`info vars ::a::*` shows both blocks'
    /// variables).  Every block is a real definition site.
    ///
    /// Exact-name matching only: the analyser already rooted every relative
    /// spelling, so there is no candidate list to walk.
    #[must_use]
    pub fn namespace_declarations_qualified<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceNamespaceRef> {
        let target = qualified_name.trim_start_matches("::");
        self.namespace_refs()
            .filter(|n| n.declares && n.uri != exclude_uri)
            .filter(|n| n.qualified_name.trim_start_matches("::") == target)
            .collect()
    }

    /// Declaring sites whose namespace is a **strict descendant** of
    /// `qualified_name` — the rows that create it *implicitly*, as a parent
    /// (`namespace eval ::p::q::r {}` really creates `::p::q`), excluding any
    /// in `exclude_uri`.
    ///
    /// The cross-document half of issue #1113 item 1 (issue #1246): the
    /// in-document tier answers implicit parents from its own
    /// `namespace_refs`, and without this query a namespace whose only
    /// creating block lives in a sibling file answered nothing at all.
    ///
    /// Rows only — the *span* an implicit answer reports is the covering
    /// prefix of the written word, a sub-range that needs the declaring
    /// document's text, so the caller pairs each row with its source through
    /// [`crate::namespace_symbol::namespace_implicit_parent_span_in`].
    #[must_use]
    pub fn namespace_declarations_under<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceNamespaceRef> {
        // The global namespace is created by the interpreter, not by any
        // block, so it has no implicit creator — the same bound the
        // in-document tier keeps.  Without it every declaring row in the
        // workspace would count as creating `::`.
        if qualified_name.trim_start_matches(':').is_empty() {
            return Vec::new();
        }
        self.namespace_refs()
            .filter(|n| n.declares && n.uri != exclude_uri)
            .filter(|n| {
                crate::namespace_symbol::namespace_strictly_contains(
                    qualified_name,
                    &n.qualified_name,
                )
            })
            .collect()
    }

    /// Non-declaring occurrences of the namespace `qualified_name`, excluding
    /// any in `exclude_uri` — the reference-side twin of
    /// [`Self::namespace_declarations_qualified`].
    #[must_use]
    pub fn namespace_refs_of<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceNamespaceRef> {
        let target = qualified_name.trim_start_matches("::");
        self.namespace_refs()
            .filter(|n| !n.declares && n.uri != exclude_uri)
            .filter(|n| n.qualified_name.trim_start_matches("::") == target)
            .collect()
    }

    /// Occurrence (read / write) sites naming the namespace variable
    /// `qualified_name`, excluding any in `exclude_uri`.  The reference-side
    /// twin of [`Self::variable_definitions_qualified`], matched the same
    /// exact way.
    #[must_use]
    pub fn variable_refs_of<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceVariableRef> {
        let target = qualified_name.trim_start_matches("::");
        self.variable_refs()
            .filter(|v| v.uri != exclude_uri)
            .filter(|v| v.qualified_name.trim_start_matches("::") == target)
            .collect()
    }

    /// The distinct set of document URIs the index currently holds
    /// (across procs, classes, and invocation sites).  Lets the
    /// server reach indexed-but-unopened files for cross-document
    /// passes that need each document's source (e.g. incoming call
    /// hierarchy).
    #[must_use]
    pub fn document_uris(&self) -> Vec<String> {
        let mut uris: Vec<String> = self
            .procs()
            .map(|p| p.uri.clone())
            .chain(self.classes().map(|c| c.uri.clone()))
            .chain(self.invocations().map(|i| i.uri.clone()))
            .collect();
        uris.sort();
        uris.dedup();
        uris
    }

    /// Invocation sites that target the proc identified by
    /// `simple_name` / `qualified_name`, excluding any in
    /// `exclude_uri` (the caller's own document, whose call
    /// sites the single-doc provider already surfaces).
    ///
    /// Each call site is settled against a **workspace-wide** command-existence
    /// oracle: its [`resolution_candidates`](WorkspaceInvocation::resolution_candidates)
    /// (caller namespace, then each `namespace path` entry, then global — Tcl's
    /// real priority order) are walked, and the first that names a proc/class
    /// defined *anywhere in the workspace* is the call's true target.  A call is
    /// a reference iff that target is `qualified_name`.
    ///
    /// This is the canonical resolver ([`tcl_syntax::naming::resolve_command_with`])
    /// widened from one file to the whole project: a bare call reaching a
    /// namespaced proc in another file via `namespace path` resolves correctly
    /// (the file-local guess could not settle it), and a call whose simple name
    /// collides with an unrelated proc resolves to the one it actually names —
    /// no textual heuristic, no ambiguity gate.
    #[must_use]
    pub fn invocations_of<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceInvocation> {
        self.invocations_settling_to(qualified_name, exclude_uri, false)
    }

    /// Whether renaming `qualified_name` must be refused outright: some
    /// invocation of it, anywhere in the workspace, is marked
    /// `rename_safe: false` — an indirect dispatch at least one of whose
    /// contributing constants has no exact writable source span, so no
    /// edit set can keep that dispatch running the renamed command
    /// (issue #945 fault 1's corruption, inverted into abstention).
    #[must_use]
    pub fn rename_blocked(&self, qualified_name: &str) -> bool {
        self.invocations_of(qualified_name, "")
            .iter()
            .any(|inv| !inv.rename_safe)
    }

    /// Invocation sites that reach `qualified_name` **through** a command
    /// name-link — an `interp alias`, a `rename`, or a `namespace import`.
    ///
    /// The same candidate settling as [`Self::invocations_of`], but the
    /// existence oracle also admits the linked names an import / alias /
    /// rename introduces, and the winning candidate is followed along those
    /// links to its ultimate target before matching.  So a bare `helper` call
    /// in a namespace that `namespace import`ed `::mod::helper` counts as a
    /// reference to `::mod::helper`, and a call through an alias counts as a
    /// reference to the aliased command.  Used by find-references, which shows
    /// every use; **not** by rename, which must not text-rewrite a call that
    /// names the local imported / aliased command (the token follows the
    /// source rename at runtime, it is not edited).
    #[must_use]
    pub fn linked_invocations_of<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceInvocation> {
        self.invocations_settling_to(qualified_name, exclude_uri, true)
    }

    /// Shared core of [`Self::invocations_of`] / [`Self::linked_invocations_of`]:
    /// call sites whose settled target is `qualified_name`, excluding
    /// `exclude_uri`.  With `follow_links`, the existence oracle admits linked
    /// names and the winning candidate is chased along the link map to its
    /// ultimate target; without it, only real proc/class definitions settle a
    /// call (the direct-reference behaviour rename relies on).
    ///
    /// A hash lookup into [`Self::invocations_by_settled_target`] plus a
    /// direct index into each hit's owning document slot — the settlement
    /// walk itself (which needs [`Self::defined_command_names`],
    /// [`Self::command_link_map`] and a [`WildcardImportIndex`]) runs at most
    /// once per generation, not once per call (issue #1152: `code_lenses`
    /// calls this once per proc *and* once per class in the document).
    fn invocations_settling_to<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
        follow_links: bool,
    ) -> Vec<&'a WorkspaceInvocation> {
        let target = qualified_name.trim_start_matches("::");
        let by_target = self.invocations_by_settled_target(follow_links);
        let Some(sites) = by_target.get(target) else {
            return Vec::new();
        };
        sites
            .iter()
            .map(|&(slot, idx)| &self.docs[slot].invocations[idx])
            .filter(|i| i.uri != exclude_uri)
            .collect()
    }

    /// Every indexed invocation's settled target, `::`-stripped and grouped
    /// into `target -> [(doc slot, index within that slot's invocations)]`
    /// — one entry per invocation that settles to *some* command, in the
    /// same relative order [`Self::invocations`] would visit them in.
    /// A [`Derived`] view, one reading per `follow_links` value; see
    /// [`SettledTargets`] for why the stored `(slot, index)` pairs stay valid
    /// for the view's whole lifetime.
    fn invocations_by_settled_target(&self, follow_links: bool) -> Arc<SettledTargets> {
        self.settled_invocations[usize::from(follow_links)].get_or_build(|| {
            // Built once per generation (both the command set and the
            // wildcard-import index — see `WildcardImportIndex`'s own doc),
            // then reused for every invocation in the workspace.
            let defined = self.defined_command_names(follow_links);
            let links = follow_links.then(|| self.command_link_map());
            let wci = WildcardImportIndex::build(self);
            let mut by_target: SettledTargets = std::collections::HashMap::new();
            for (slot, doc) in self.docs.iter().enumerate() {
                for (idx, inv) in doc.invocations.iter().enumerate() {
                    if let Some(target) =
                        self.settle_invocation(inv, &defined, links.as_deref(), &wci)
                    {
                        by_target.entry(target).or_default().push((slot, idx));
                    }
                }
            }
            by_target
        })
    }

    /// The command `inv` settles to, `::`-stripped, or `None` when nothing in
    /// the workspace resolves it: the first of its candidates defined
    /// anywhere in the workspace, chased along `links` (when supplied) to its
    /// ultimate target — falling back, when following links, to a wildcard
    /// `namespace import NS::*` in scope (issue #923 idx 18). The settlement
    /// logic itself is unchanged from the pre-#1152 `invocation_resolves_to`,
    /// restated as "what does this settle to" (grouped by the answer) rather
    /// than "does this settle to the one target the caller named" (checked
    /// once per candidate target).
    fn settle_invocation(
        &self,
        inv: &WorkspaceInvocation,
        defined: &HashSet<String>,
        links: Option<&std::collections::HashMap<String, String>>,
        wci: &WildcardImportIndex<'_>,
    ) -> Option<String> {
        let call = CallSite {
            uri: &inv.uri,
            at: inv.range.start(),
            enclosing_body: inv.enclosing_body,
        };
        // A live `namespace import -force` has *replaced* the importing
        // namespace's own command of this name, so no candidate naming that
        // command may settle the call — it reaches the import's source, which
        // the wildcard tier below resolves (issue #1116 item 1). The same rule
        // `definition::resolve_called_proc` applies in-document, so without it
        // find-references files the call under the definition the import
        // deleted while go-to-definition jumps to the source.
        //
        // Only in the link-following view, matching the rule below: a glob
        // import introduces no fixed link, so the direct-only view rename
        // relies on does not see it in either direction.
        let forced_shadow = links.is_some()
            && wci.forced_shadow_over_candidates(&inv.name, &inv.resolution_candidates, call);
        if !forced_shadow
            && let Some(winner) = inv
                .resolution_candidates
                .iter()
                .find(|c| defined.contains(c.trim_start_matches("::")))
        {
            let winner = winner.trim_start_matches("::");
            return Some(
                links.map_or_else(|| winner.to_owned(), |m| Self::follow_links(m, winner)),
            );
        }
        // No real command or name-link settled this call — try a wildcard
        // `namespace import NS::*` in scope for the call's own namespace
        // (issue #923 idx 18). Only when following links: this mirrors an
        // exact import's `WorkspaceCommandLink`, which likewise only
        // participates in the *linked* view (`linked_invocations_of`, used
        // by find-references) and never the direct-only view rename relies
        // on — a call spelling the local imported name is not text-rewritten
        // just because its ultimate source is renamed.
        links?;
        self.resolve_wildcard_import_indexed(&inv.name, &inv.resolution_candidates, call, wci)
            .map(|resolved| resolved.trim_start_matches("::").to_owned())
    }

    /// The command name-link map (`::`-stripped `linked → immediate target`)
    /// used to chase an import / alias / rename to the command it names.
    /// A [`Derived`] view rather than a fresh walk of
    /// [`Self::live_command_links`] per call (issue #1152); owned (`String`,
    /// not `&str`) so the view is independent of any one call's borrow.
    fn command_link_map(&self) -> Arc<std::collections::HashMap<String, String>> {
        self.command_link_map.get_or_build(|| {
            self.live_command_links()
                .into_iter()
                .map(|l| {
                    (
                        l.linked_qname.trim_start_matches("::").to_owned(),
                        l.target_qname.trim_start_matches("::").to_owned(),
                    )
                })
                .collect()
        })
    }

    /// Chase `start` along the link map to its ultimate target, stopping at a
    /// name that is not itself a linked name.  Bounded by cycle detection (an
    /// alias-of-an-alias loop) so a malformed chain cannot spin.
    fn follow_links(links: &std::collections::HashMap<String, String>, start: &str) -> String {
        let mut cur = start.to_owned();
        let mut seen = std::collections::HashSet::new();
        while let Some(next) = links.get(&cur) {
            if !seen.insert(cur.clone()) {
                break;
            }
            cur.clone_from(next);
        }
        cur
    }

    /// The ultimate command `name` denotes after following every
    /// import / alias / rename link, `::`-rooted.  A name that is not linked
    /// (an ordinary proc/class, or an unknown) returns unchanged.  Lets a
    /// cursor sitting on an imported / aliased call resolve to the command it
    /// really names, so its references gather with that command's.
    #[must_use]
    pub fn resolve_command_target(&self, name: &str) -> String {
        let links = self.command_link_map();
        let settled = Self::follow_links(&links, name.trim_start_matches("::"));
        format!("::{settled}")
    }

    /// The declaration spans that *name* the command `qualified_name` in an
    /// `interp alias` / `rename` / `namespace import` — the alias `TARGET`
    /// word, the `rename` `OLD` word, the import pattern.  Each is a reference
    /// to the command that a rename of it must rewrite.  Excludes
    /// `exclude_uri` (the caller's own document, whose spans the single-doc
    /// provider already surfaces) and any link whose source scan recorded no
    /// span.
    #[must_use]
    pub fn link_target_spans(
        &self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<(String, Span)> {
        let target = qualified_name.trim_start_matches("::");
        self.live_command_links()
            .into_iter()
            .filter(|l| l.uri != exclude_uri)
            .filter(|l| l.target_qname.trim_start_matches("::") == target)
            .filter_map(|l| l.target_span.map(|sp| (l.uri.clone(), sp)))
            .collect()
    }

    /// The fully-qualified names of every indexed class — the workspace class
    /// set the cross-file analysis feeds to
    /// [`tcl_compiler::analyser::Analyser::with_workspace_classes`] so a
    /// consumer document's `set d [::other::Cls new]` resolves cross-file.
    #[must_use]
    pub fn all_class_qnames(&self) -> std::collections::HashSet<String> {
        self.classes().map(|c| c.qualified_name.clone()).collect()
    }

    /// The URIs of documents that invoke (a constructor of) any class in
    /// `class_qnames` — the *candidate consumer* documents whose `$obj method`
    /// sites a cross-file method reference must scan.  A call qualifies when any
    /// of its resolution candidates names one of the classes (leading `::`
    /// ignored), which catches `Cls new` / `Cls create obj` however the class
    /// was spelled at the call site.  Bounds the consumer scan to documents that
    /// actually mention a family class rather than the whole workspace.
    #[must_use]
    pub fn documents_invoking_classes(
        &self,
        class_qnames: &std::collections::HashSet<&str>,
    ) -> std::collections::HashSet<String> {
        self.invocations()
            .filter(|i| {
                i.resolution_candidates
                    .iter()
                    .any(|c| class_qnames.contains(c.trim_start_matches("::")))
            })
            .map(|i| i.uri.clone())
            .collect()
    }

    /// Whether the command `qualified_name` (leading `::` ignored) resolves
    /// anywhere in the workspace — either a real proc/class definition, or a
    /// name an `interp alias` / `rename` / `namespace import` introduces.  The
    /// existence oracle that widens the single-file command resolver to the
    /// whole project; the linked names are admitted so a cursor on an
    /// imported / aliased call still finds a symbol to resolve.
    #[must_use]
    pub fn workspace_command_exists(&self, qualified_name: &str) -> bool {
        self.defined_command_names(true)
            .contains(qualified_name.trim_start_matches("::"))
    }

    /// [`Self::workspace_command_exists`], but a proc, or an `interp alias` /
    /// `rename` / `namespace import` link, whose own declaration is nested
    /// inside another proc's or class's body (so it exists only
    /// conditionally, when and if that enclosing definition actually runs)
    /// counts only when `has_builtin` is `false`. An unconditional
    /// (top-level) proc or link still counts regardless of `has_builtin`.
    ///
    /// A nested `proc ::set {...}` written to temporarily shadow the real
    /// `set` builtin — `rename` it away, install the shadow, `rename` it
    /// back — must not make `::set` permanently "exist in the workspace" for
    /// every cross-file call-site resolution the way a real top-level
    /// definition would; that call always reaches the builtin unless a
    /// caller can prove the shadow's narrow active window actually contains
    /// it, which this index does not attempt to prove. The same reasoning
    /// applies to a `rename`/alias/import written inside a body — e.g.
    /// `proc withRealSet {} { rename set ::real_set; ... }` locally
    /// redirecting `set` — while a *top-level* `interp alias {} set {}
    /// ::my_set` permanently overrides the builtin for the rest of the file,
    /// exactly like a top-level `proc set`, and must keep counting as
    /// existing. Mirrors the same judgement `resolve_called_proc` already
    /// applies same-file
    /// (`AnalysisResult::offset_is_inside_any_definition_body`), extended to
    /// the cross-file existence oracle so hover / definition / references
    /// agree instead of one abstaining and the other still finding the
    /// shadow.
    #[must_use]
    pub fn workspace_command_exists_for_call(
        &self,
        qualified_name: &str,
        has_builtin: bool,
    ) -> bool {
        if !has_builtin {
            return self.workspace_command_exists(qualified_name);
        }
        let target = qualified_name.trim_start_matches("::");
        self.procs()
            .any(|p| !p.nested && p.qualified_name.trim_start_matches("::") == target)
            || self
                .classes()
                .any(|c| c.qualified_name.trim_start_matches("::") == target)
            || self
                .live_command_links()
                .into_iter()
                .any(|l| !l.nested && l.linked_qname.trim_start_matches("::") == target)
    }

    /// The set of `::`-stripped qualified names of every indexed proc and
    /// class, for O(1) membership in the candidate-resolution loop of
    /// [`Self::invocations_of`].  With `include_links`, the names an import /
    /// alias / rename introduces join the set, so a call reaching one of them
    /// settles (and is then chased to its ultimate target).
    ///
    /// A [`Derived`] view rather than a fresh walk (issues #1105 / #1152): it
    /// is `O(procs + classes + links)` to build, and
    /// [`Self::workspace_command_exists`] asks for it *per candidate* inside
    /// [`Self::follow_import_chain`]'s loop — and
    /// [`Self::invocations_by_settled_target`] once per settling pass — which
    /// turned an existence test into a workspace-wide scan. Owned (`String`,
    /// not `&str`) so the view is independent of any one call's borrow. The
    /// two `include_links` readings are two separate views because a consumer
    /// wants exactly one of them: the direct-only set is what rename relies
    /// on, and folding the link names into it would let a call spelling an
    /// imported name be text-rewritten.
    fn defined_command_names(&self, include_links: bool) -> Arc<HashSet<String>> {
        self.defined_names[usize::from(include_links)].get_or_build(|| {
            let mut names: HashSet<String> = self
                .procs()
                .map(|p| p.qualified_name.trim_start_matches("::").to_owned())
                .chain(
                    self.classes()
                        .map(|c| c.qualified_name.trim_start_matches("::").to_owned()),
                )
                .collect();
            if include_links {
                names.extend(
                    self.live_command_links()
                        .into_iter()
                        .map(|l| l.linked_qname.trim_start_matches("::").to_owned()),
                );
            }
            names
        })
    }

    /// Resolve a bare call through a wildcard `namespace import NS::*` —
    /// the cross-document analogue of `tcl-lsp-core`'s in-document
    /// `definition::resolve_called_proc` / `resolve_class_target_at`
    /// wildcard-import fallback, for when `NS` is defined in a **different**
    /// file. `namespace import` binds to the *namespace*, not the file that
    /// wrote it (real Tcl: `namespace eval ::app { namespace import
    /// ::mymod::* }` in a shared "imports.tcl" makes `::mymod`'s exports
    /// visible to every `::app`-namespace proc regardless of which file its
    /// body lives in) — so every recorded [`WorkspaceGlobImport`] whose
    /// `ns` matches is in scope, not only ones recorded in the calling
    /// document.
    ///
    /// `resolution_candidates` is the call's own ordered candidate list
    /// (`word`'s caller namespace, then each `namespace path` entry, then
    /// global — [`tcl_syntax::naming::command_resolution_candidates`]); for
    /// each candidate, this checks whether *that* candidate's namespace has
    /// an in-scope glob import whose tail pattern glob-matches `word`. A
    /// wildcard import only ever imports names its source namespace has
    /// actually `namespace export`ed — an unexported sibling command is
    /// **not** reachable through it (tclsh9.0/8.6-verified: `invalid command
    /// name` calling it bare) — so this also requires the pattern's source
    /// namespace to have exported a covering pattern, and a real proc/class
    /// definition to exist anywhere in the workspace at the resolved
    /// qualified name. Returns that qualified name, `::`-rooted, or `None`.
    ///
    /// Restricted to a genuine bareword `word` (no embedded `::`), matching
    /// the in-document resolver and the bug's scope (issue #923 idx 18).
    #[must_use]
    pub fn resolve_wildcard_import(
        &self,
        word: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
    ) -> Option<String> {
        self.resolve_wildcard_import_indexed(
            word,
            resolution_candidates,
            call,
            &WildcardImportIndex::build(self),
        )
    }

    /// The indexed core of [`Self::resolve_wildcard_import`], taking a
    /// precomputed [`WildcardImportIndex`] instead of scanning
    /// `glob_imports`/`namespace_exports` in full — the fast path
    /// [`Self::settle_invocation`] uses so a workspace-wide
    /// invocation-settling pass builds the index once (O(every glob import
    /// / export in the workspace)) rather than once per invocation (which
    /// would be O(invocation count × workspace-wide glob-import count) —
    /// measurably regressed find-references latency on a codebase using
    /// `namespace import NS::*` in more than a handful of files).
    fn resolve_wildcard_import_indexed(
        &self,
        word: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
        wci: &WildcardImportIndex<'_>,
    ) -> Option<String> {
        if word.contains("::") {
            return None;
        }
        for cand in resolution_candidates {
            let Some((prefix, tail)) = cand.rsplit_once("::") else {
                continue;
            };
            if tail != word {
                continue;
            }
            let candidate_ns = if prefix.is_empty() { "::" } else { prefix };
            if let Some(target) = self.follow_import_chain(candidate_ns, word, call, wci) {
                return Some(target);
            }
        }
        None
    }

    /// Follow the import edges out of `ns` until they reach a command the
    /// workspace actually defines, and return that qualified name.
    ///
    /// An import edge may land on a name that is *itself* imported: with
    /// `::C` exporting `p`, `::B` importing `::C::*` and re-exporting, and
    /// `::A` importing `::B::*`, `::A::p` runs `::C`'s body and `namespace
    /// origin ::A::p` answers `::C::p` (oracle tclsh 8.6.14 / 9.0.4 — issue
    /// #1103). The middle hop is in no workspace proc/class table, so a
    /// single-hop walk found nothing and go-to-definition silently abstained.
    ///
    /// Bounded by [`tcl_compiler::analyser::indirection::MAX_COMMAND_NAME_HOPS`]
    /// — the same cap the `rename` / `interp alias` walk applies to the same
    /// kind of chain, so a mutually-importing pair cannot spin. The whole
    /// chain is judged at the *call's* offset: a forget anywhere along it
    /// kills the call (oracle: forgetting `::C::p` inside `::B` makes
    /// `::A::p` an `invalid command name` too, because deleting an imported
    /// command deletes the commands imported from it).
    fn follow_import_chain(
        &self,
        ns: &str,
        word: &str,
        call: CallSite<'_>,
        wci: &WildcardImportIndex<'_>,
    ) -> Option<String> {
        let mut current = ns.to_owned();
        for _ in 0..tcl_compiler::analyser::indirection::MAX_COMMAND_NAME_HOPS {
            let hop = self.import_hop(&current, word, call, wci)?;
            let target = tcl_syntax::naming::qualify(&hop, word);
            if self.workspace_command_exists(&target) {
                return Some(target);
            }
            current = hop;
        }
        None
    }

    /// One hop of [`Self::follow_import_chain`]: the source namespace of the
    /// live import that makes `word` callable from `ns` at `call`, or `None`.
    ///
    /// Three gates, in the order real Tcl applies them:
    ///
    /// 1. the pattern must cover `word`;
    /// 2. the source namespace must have exported it **at that import's own
    ///    position** ([`WildcardImportIndex::exports_name_at`], issue #1027);
    /// 3. without `-force`, the importing namespace must not already hold a
    ///    command of that name — such an import raises `can't import command
    ///    "p": already exists` and installs nothing, so a bare call still
    ///    reaches the local definition (issue #1103). With `-force` it
    ///    replaces the local one instead, which is why the check is skipped
    ///    there.
    ///
    /// …and then the edge must still be *there*
    /// ([`WildcardImportIndex::alias_live_at`]).
    fn import_hop(
        &self,
        ns: &str,
        word: &str,
        call: CallSite<'_>,
        wci: &WildcardImportIndex<'_>,
    ) -> Option<String> {
        let imports = wci.imports_by_ns.get(ns)?;
        // The **latest** live install wins, not the first match: a second
        // import of the same name — `-force`, or after a forget — replaces
        // the first alias (oracle on `conflicting_alias_at`), exactly as the
        // same-document tier's fold decides it. Offsets only order rows in
        // the calling document, so same-document imports rank above
        // unordered foreign ones, and among foreign ones the index's own
        // stable source order breaks the tie.
        imports
            .iter()
            .filter(|imp| tcl_syntax::glob::string_match(&imp.tail_pattern, word))
            .filter(|imp| wci.exports_name_at(&imp.source_ns, word, imp.site()))
            .filter(|imp| {
                imp.forced
                    || !(self.defines_command(&tcl_syntax::naming::qualify(ns, word))
                        || wci.conflicting_alias_at(ns, &imp.source_ns, word, imp.site()))
            })
            .filter(|imp| wci.alias_live_at(ns, &imp.source_ns, word, imp.site(), call))
            .enumerate()
            .max_by_key(|(seq, imp)| (imp.uri == call.uri, imp.at, *seq))
            .map(|(_, imp)| imp.source_ns.clone())
    }
}

/// Precomputed per-namespace grouping of [`WorkspaceIndex::glob_imports`]
/// and [`WorkspaceIndex::namespace_exports`], built once per query rather
/// than re-scanned per call site — see
/// [`WorkspaceIndex::resolve_wildcard_import_indexed`]'s doc for why.
struct WildcardImportIndex<'a> {
    imports_by_ns: std::collections::HashMap<&'a str, Vec<&'a WorkspaceGlobImport>>,
    /// Every **exact** `namespace import` link, by importing namespace, as
    /// `(document, gate)`. The exact-pattern twin of [`Self::imports_by_ns`]:
    /// the two tables are one command table as far as the import-conflict rule
    /// is concerned, which is why [`Self::conflicting_alias_at`] reads both
    /// (issue #1116 item 7).
    exact_by_ns: std::collections::HashMap<&'a str, Vec<(&'a str, &'a WorkspaceImportGate)>>,
    exports_by_ns: std::collections::HashMap<&'a str, Vec<&'a WorkspaceNamespaceExport>>,
    forgets_by_ns: std::collections::HashMap<&'a str, Vec<&'a WorkspaceNamespaceForget>>,
    deletions_by_name: std::collections::HashMap<&'a str, Vec<&'a WorkspaceCommandDeletion>>,
    /// Every proc / class declaration site, by `::`-rooted qualified name —
    /// a redefinition of an *imported* name ends the alias (issue #1116
    /// finding 3), which is a question about where the declaration sits, not
    /// merely whether one exists.
    declarations_by_qname: std::collections::HashMap<&'a str, Vec<(&'a str, u32)>>,
    /// The workspace's `source`-graph load order — the relation every
    /// decision below ranks its events with, so a cross-document event is
    /// ordered wherever the graph proves an order and unrankable everywhere
    /// else (issue #1104 item 3).
    order: Arc<crate::source_graph::RunOrder>,
    /// The `::`-stripped namespaces this workspace can say anything about —
    /// [`WorkspaceIndex::observable_namespaces`]. Only
    /// [`Self::forced_shadow_at`] reads it, and only to abstain the way the
    /// single-document tier does: a `-force` import of a namespace no indexed
    /// file declares deletes the local command whether or not the export can
    /// be proven here.
    observable: HashSet<&'a str>,
}

impl<'a> WildcardImportIndex<'a> {
    fn build(index: &'a WorkspaceIndex) -> Self {
        let mut imports_by_ns: std::collections::HashMap<&str, Vec<&WorkspaceGlobImport>> =
            std::collections::HashMap::new();
        for imp in index.glob_imports() {
            imports_by_ns.entry(imp.ns.as_str()).or_default().push(imp);
        }
        let mut exact_by_ns: std::collections::HashMap<&str, Vec<(&str, &WorkspaceImportGate)>> =
            std::collections::HashMap::new();
        for link in index.command_links() {
            if let Some(gate) = link.import_gate.as_ref() {
                exact_by_ns
                    .entry(importing_namespace_of(&link.linked_qname))
                    .or_default()
                    .push((link.uri.as_str(), gate));
            }
        }
        let mut exports_by_ns: std::collections::HashMap<&str, Vec<&WorkspaceNamespaceExport>> =
            std::collections::HashMap::new();
        for exp in index.namespace_exports() {
            exports_by_ns.entry(exp.ns.as_str()).or_default().push(exp);
        }
        let mut forgets_by_ns: std::collections::HashMap<&str, Vec<&WorkspaceNamespaceForget>> =
            std::collections::HashMap::new();
        for fgt in index.namespace_forgets() {
            forgets_by_ns.entry(fgt.ns.as_str()).or_default().push(fgt);
        }
        let mut deletions_by_name: std::collections::HashMap<&str, Vec<&WorkspaceCommandDeletion>> =
            std::collections::HashMap::new();
        for del in index.command_deletions() {
            deletions_by_name
                .entry(del.qualified_name.as_str())
                .or_default()
                .push(del);
        }
        let mut declarations_by_qname: std::collections::HashMap<&str, Vec<(&str, u32)>> =
            std::collections::HashMap::new();
        for (qname, uri, at) in index
            .procs()
            .map(|p| {
                (
                    p.qualified_name.as_str(),
                    p.uri.as_str(),
                    p.name_span.start(),
                )
            })
            .chain(index.classes().map(|c| {
                (
                    c.qualified_name.as_str(),
                    c.uri.as_str(),
                    c.name_span.start(),
                )
            }))
        {
            declarations_by_qname
                .entry(qname)
                .or_default()
                .push((uri, at));
        }
        Self {
            imports_by_ns,
            exact_by_ns,
            exports_by_ns,
            forgets_by_ns,
            deletions_by_name,
            declarations_by_qname,
            order: index.run_order(),
            observable: index.observable_namespaces(),
        }
    }

    /// Whether a live `namespace import -force` in `ns` has taken `name` away
    /// from `ns`'s own command table by the time the call at `call` runs — the
    /// cross-document twin of `definition::forced_import_shadows`.
    ///
    /// `-force` is the one import that outranks a command the importing
    /// namespace already holds, so it is the one case where a *defined*
    /// candidate must not settle a call ([`WorkspaceIndex::settle_invocation`]).
    /// No conflict check: a forced import never conflicts, which is what
    /// `-force` means.
    ///
    /// The export gate abstains toward the shadow for a source namespace no
    /// indexed file declares — the same direction the single-document tier
    /// takes and the same one [`WorkspaceIndex::live_command_links`] takes for
    /// exact imports. Answering with a command the import may have deleted is
    /// worse than answering nothing.
    fn forced_shadow_at(&self, ns: &str, name: &str, call: CallSite<'_>) -> bool {
        self.imports_by_ns.get(ns).is_some_and(|imports| {
            imports.iter().any(|imp| {
                imp.forced
                    && tcl_syntax::glob::string_match(&imp.tail_pattern, name)
                    && (!self
                        .observable
                        .contains(imp.source_ns.trim_start_matches("::"))
                        || self.exports_name_at(&imp.source_ns, name, imp.site()))
                    && self.alias_live_at(ns, &imp.source_ns, name, imp.site(), call)
            })
        })
    }

    /// [`Self::forced_shadow_at`] over a call's own candidate list, in Tcl's
    /// resolution order — the shape [`WorkspaceIndex::settle_invocation`] has
    /// in hand.
    fn forced_shadow_over_candidates(
        &self,
        name: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
    ) -> bool {
        if name.contains("::") {
            return false;
        }
        resolution_candidates.iter().any(|cand| {
            cand.rsplit_once("::").is_some_and(|(prefix, tail)| {
                tail == name
                    && self.forced_shadow_at(
                        if prefix.is_empty() { "::" } else { prefix },
                        name,
                        call,
                    )
            })
        })
    }

    /// Every removal event bearing on the alias `importing_ns` took from
    /// `source_ns` for `name`, as seen from `query` — the cross-document
    /// half of [`crate::namespace_import::alias_live_at`]'s event log
    /// (issue #1103).
    ///
    /// Three kinds, and the ordering rule differs per kind because the
    /// underlying facts differ:
    ///
    /// - **`namespace forget`** and **a redefinition of the imported name**
    ///   are events on *this namespace's* slot. They are ordered only inside
    ///   one document: which of two files loads first is not a static fact
    ///   (#1104 item 3), so an event in another document is passed unordered
    ///   and revokes nothing.
    /// - **Destroying the source command** (`rename ::src::p {}`) is not a
    ///   slot event at all — the command *object* the alias holds is gone,
    ///   workspace-wide, and no load order brings it back. When there is a
    ///   call site to order against it is ordered like the rest; with none
    ///   ([`Self::link_alias_live`]) it is passed as having already happened.
    fn removal_events<'e>(
        &'e self,
        importing_ns: &'e str,
        source_ns: &'e str,
        name: &'e str,
        destroy_point: impl Fn(&'e WorkspaceCommandDeletion) -> RunPoint<'e> + 'e,
    ) -> impl Iterator<Item = crate::namespace_import::AliasEvent<'e>> + 'e {
        use crate::namespace_import::{AliasEvent, AliasEventKind};
        let forgets = self
            .forgets_by_ns
            .get(importing_ns)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter(move |f| {
                f.source_ns.as_deref().is_none_or(|src| {
                    src.trim_start_matches("::") == source_ns.trim_start_matches("::")
                }) && tcl_syntax::glob::string_match(&f.pattern, name)
            })
            .map(move |f| AliasEvent {
                kind: AliasEventKind::Remove,
                at: RunPoint {
                    uri: f.uri.as_str(),
                    at: f.at,
                    enclosing_body: None,
                },
            });
        // A `proc` / class declaration of the *imported* name recreates it as
        // an ordinary command and ends the alias (oracle on
        // `definition::live_import_at` case 5).
        let redefinitions = self
            .declarations_by_qname
            .get(tcl_syntax::naming::qualify(importing_ns, name).as_str())
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(move |&(uri, at)| AliasEvent {
                kind: AliasEventKind::Remove,
                at: RunPoint {
                    uri,
                    at,
                    enclosing_body: None,
                },
            });
        let qualified = tcl_syntax::naming::qualify(source_ns, name);
        let deletions = self
            .deletions_by_name
            .get(qualified.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(move |d| AliasEvent {
                kind: AliasEventKind::Remove,
                at: destroy_point(d),
            });
        forgets.chain(redefinitions).chain(deletions)
    }

    /// Whether the alias `importing_ns` took from `source_ns` for `name` is
    /// still there when the call at `call` runs — the cross-document binding
    /// of the shared lifecycle decision
    /// [`crate::namespace_import::alias_live_at`] (issue #1103).
    ///
    /// `install_at` is the import's own site. It is ordered against the call
    /// **only when the two share a document** (issue #1116 finding 1): a byte
    /// offset in the importing file and a byte offset in the calling file are
    /// unrelated numbers, and comparing them let a `namespace forget` in the
    /// caller revoke a cross-file import purely because its local offset
    /// happened to be the larger one. Unordered, the shared function keeps the
    /// alias — the same direction every other cross-file event takes here.
    ///
    /// Within one document the comparison is
    /// [`tcl_compiler::analyser::indirection::in_effect_within`] — the
    /// *identical* rule the same-document tier applies — stated over the
    /// call's own offset and enclosing body span ([`CallSite`]). A plain
    /// offset test is not good enough in either direction: it would leave a
    /// body-local call resolving through a top-level `namespace forget`
    /// written before it (issue #1116 item 3, the lenient direction), and,
    /// now that installs are order-gated too (issue #1104 item 1), it would
    /// drop the alias of every proc body calling a name its own file imports
    /// further down (the *un*safe direction, which is why the span became a
    /// required [`CallSite`] field rather than an optional refinement).
    fn alias_live_at(
        &self,
        importing_ns: &str,
        source_ns: &str,
        name: &str,
        install_at: ImportSite<'_>,
        call: CallSite<'_>,
    ) -> bool {
        use crate::namespace_import::{AliasEvent, AliasEventKind};
        let install = std::iter::once(AliasEvent {
            kind: AliasEventKind::Install,
            at: install_at.point(),
        });
        let mut events =
            install.chain(
                self.removal_events(importing_ns, source_ns, name, |d| RunPoint {
                    uri: d.uri.as_str(),
                    at: d.at,
                    enclosing_body: None,
                }),
            );
        crate::namespace_import::alias_live_at(&mut events, &self.order, Some(call.point()))
    }

    /// Whether `ns` already holds a live alias for `name` from a namespace
    /// other than `source_ns` when the import at `site` runs — the conflict a
    /// non-`-force` `namespace import` aborts on (issue #1116 finding 4).
    ///
    /// Oracle (9.0.4 / 8.6.14): with `::dst` already importing `p` from `::A`,
    /// a later unforced import of `p` from `::B` raises `can't import command
    /// "p": already exists` and leaves `namespace origin ::dst::p` → `::A::p`.
    ///
    /// **Both spellings of an import count on both sides** (issue #1116 item
    /// 7). Tcl installs one alias per name; whether the import that installed
    /// it was written as a glob or as an exact pattern is a fact about the
    /// *source text*, not about the command table. The index splits them —
    /// a glob pattern names no single command, so it becomes a
    /// [`WorkspaceGlobImport`] consulted per call, while an exact one becomes
    /// a fixed [`WorkspaceCommandLink`] — and asking each side only about its
    /// own kind made the conflict rule directional: a glob import that had
    /// already bound the name did not conflict with a later exact import of
    /// it. One function, both tables, so neither caller can drift.
    ///
    /// Ordered within the import's own document only: two imports in
    /// different files have no static load order, and conflicting on a guess
    /// would drop an alias Tcl really installed. A **same-source** re-import
    /// is a silent no-op (oracle), never a conflict — which is also what stops
    /// an import from conflicting with itself.
    ///
    /// Within that document the order is
    /// [`crate::namespace_import::load_order`], shared with the same-document
    /// resolver's slot-log fold. A raw `other.at < site.at` was wrong for
    /// exactly the shape this whole family exists to model: a **body-local**
    /// import is not ordered against a top-level import of its own file by
    /// offset at all, because the file loads — running every top-level
    /// statement, imports included — before any body runs, so the top-level
    /// one owns the name however far below it is written (oracle transcript on
    /// `load_order`). The offset comparison saw `::A` written *later* and let
    /// the body-local `::B` import install, so navigation answered a source
    /// the program never reaches.
    ///
    /// It is deliberately **not**
    /// [`tcl_compiler::analyser::indirection::in_effect_within`], the
    /// primitive the lifecycle check below runs on: that one is lenient about
    /// events in bodies that may never run, which is the safe direction for a
    /// *removal* and the unsafe one for a conflict — applied here it made the
    /// two imports above cancel each other and the name resolve nowhere.
    /// [`crate::namespace_import::load_order`] carries the reasoning.
    ///
    /// Self-exclusion survives the swap for free: an import's own key is never
    /// less than itself, so it still cannot conflict with itself.
    fn conflicting_alias_at(
        &self,
        ns: &str,
        source_ns: &str,
        name: &str,
        site: ImportSite<'_>,
    ) -> bool {
        let call = CallSite {
            uri: site.uri,
            at: site.at,
            enclosing_body: site.enclosing_body,
        };
        let mut earlier = self
            .imports_by_ns
            .get(ns)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter(|other| tcl_syntax::glob::string_match(&other.tail_pattern, name))
            .map(|other| (other.source_ns.as_str(), other.site()))
            .chain(
                self.exact_by_ns
                    .get(ns)
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                    .iter()
                    .filter(|(_, gate)| gate.name == name)
                    .map(|(uri, gate)| (gate.source_ns.as_str(), gate.site(uri))),
            );
        earlier.any(|(other_source, other_site)| {
            self.order.cmp_run(other_site.point(), site.point()) == Some(std::cmp::Ordering::Less)
                && !ns_eq(other_source, source_ns)
                && self.exports_name_at(other_source, name, other_site)
                && self.alias_live_at(ns, other_source, name, other_site, call)
        })
    }

    /// Whether an **exact** import link is still installed, with no call site
    /// to order against — the gate [`WorkspaceIndex::live_command_links`]
    /// applies (issue #1116 finding 2).
    ///
    /// `namespace import ::src::p` produces a fixed `WorkspaceCommandLink`
    /// rather than a per-call glob lookup, so the export snapshot and the
    /// `-force` conflict were the only things gating it: after `namespace
    /// forget ::src::p` — or `rename ::src::p {}` — the link stayed live and
    /// cross-document definition / references still resolved `::dst::p`.
    ///
    /// The question a link answers is "does this alias exist for navigation",
    /// which has no query point of its own, so every recorded removal counts
    /// as having run (the order predicate is unconditionally true) and the
    /// ordering that remains is the removal's position relative to the
    /// **import**:
    ///
    /// - A forget or redefinition in the import's own document orders against
    ///   it: one written *after* the import revokes the link, one written
    ///   before is undone by the import itself (a re-import after a forget
    ///   reinstalls — oracle).
    /// - One in another document has no static load order and revokes
    ///   nothing, exactly as in [`Self::alias_live_at`].
    /// - A destroyed source command revokes regardless, and is **not** put on
    ///   the timeline at all: the command object is gone workspace-wide
    ///   (oracle: `rename ::src::p {}` makes `::dst::p` an `invalid command
    ///   name` and empties `info commands ::dst::*`) and no load order brings
    ///   it back, so with no query point to order anything against it is
    ///   decided before the fold runs.  Encoding it as an event at `u32::MAX`
    ///   instead would be wrong for a **body-local** import gate: the
    ///   load-order rule reads a removal written outside the import's own body
    ///   as having run *before* it — right for a `namespace forget`, which the
    ///   import then undoes, and exactly backwards for a destruction the
    ///   import cannot undo.
    fn link_alias_live(&self, importing_ns: &str, gate: &WorkspaceImportGate, uri: &str) -> bool {
        use crate::namespace_import::{AliasEvent, AliasEventKind};
        if self
            .deletions_by_name
            .contains_key(tcl_syntax::naming::qualify(&gate.source_ns, &gate.name).as_str())
        {
            return false;
        }
        let install = std::iter::once(AliasEvent {
            kind: AliasEventKind::Install,
            at: gate.site(uri).point(),
        });
        let mut events =
            install.chain(
                self.removal_events(importing_ns, &gate.source_ns, &gate.name, |d| RunPoint {
                    uri: d.uri.as_str(),
                    at: d.at,
                    enclosing_body: None,
                }),
            );
        crate::namespace_import::alias_live_at(&mut events, &self.order, None)
    }

    /// Whether `ns` (`::`-rooted) had exported the unqualified `name` **as of
    /// the import site** at `import_at` in `import_uri` — the cross-document
    /// half of the wildcard-import gate (issue #923 idx 18: real Tcl only
    /// imports names a source namespace has actually exported, `Tcl_Export`,
    /// `tclNamesp.c`), taken per import site rather than against the
    /// workspace's final export state (issue #1027).
    ///
    /// Delegates to [`crate::namespace_import::exported_at_import_site`] — the
    /// same function the same-document resolver
    /// (`definition::exported_at_import`) calls, so the two tiers cannot
    /// disagree about what an import site sees. The only tier-specific part
    /// is which events are *ordered* against the import: those in the
    /// import's own document, compared by offset. An export in another
    /// document has no static order relative to this import (nothing fixes
    /// which file loads first), so it is passed unordered and the shared
    /// function abstains toward continuing to resolve.
    fn exports_name_at(&self, ns: &str, name: &str, site: ImportSite<'_>) -> bool {
        self.exports_by_ns.get(ns).is_some_and(|exports| {
            exported_at_site(exports.iter().copied(), name, &self.order, site.point())
        })
    }
}

/// [`crate::namespace_import::exported_at_import_site`] over a namespace's
/// indexed export rows — the one place `WorkspaceNamespaceExport` is turned
/// into an [`crate::namespace_import::ExportEvent`].
///
/// Shared by the per-call wildcard walk ([`WildcardImportIndex::exports_name_at`])
/// and the standalone [`NamespaceExportSnapshot`] the single-document tier
/// borrows, so the two cannot answer the same question differently.
fn exported_at_site<'e>(
    exports: impl Iterator<Item = &'e WorkspaceNamespaceExport>,
    name: &str,
    order: &crate::source_graph::RunOrder,
    site: RunPoint<'_>,
) -> bool {
    let mut events = exports.map(|e| crate::namespace_import::ExportEvent {
        pattern: e.pattern.as_str(),
        clears: e.clears,
        at: RunPoint {
            uri: e.uri.as_str(),
            at: e.at,
            enclosing_body: None,
        },
    });
    crate::namespace_import::exported_at_import_site(&mut events, name, order, site)
}

/// The workspace's `namespace export` records, plus the run order and the
/// observable-namespace set needed to read them — an **owned**, self-contained
/// [`crate::namespace_import::NamespaceExportOracle`] the single-document tier
/// can borrow (issue #1116 item 1).
///
/// Owned rather than a view borrowing [`WorkspaceIndex`] because the server's
/// pure-CPU providers run on a blocking worker with the index lock released:
/// the snapshot is taken under the lock and moved into the worker. It is a
/// [`Derived`] view, so a whole generation's requests share one build.
///
/// It holds *only* what the export question needs. Everything else about an
/// import — the conflict rule, the alias lifecycle, chain following — stays
/// where it already is; this is not a second cross-document resolver.
#[derive(Debug, Default)]
pub struct NamespaceExportSnapshot {
    /// Export rows by exporting namespace, `::`-stripped so the two spellings
    /// an import pattern and an export record may carry still meet.
    exports_by_ns: std::collections::HashMap<String, Vec<WorkspaceNamespaceExport>>,
    /// The `::`-stripped namespaces the workspace can say anything about — the
    /// discriminator between [`ExportVerdict::NotExported`] and
    /// [`ExportVerdict::Unknown`], and the same set
    /// [`WorkspaceIndex::live_command_links`] gates its own abstention on.
    observable: HashSet<String>,
    /// The `source`-graph load order, so an export the graph proves runs
    /// *after* the import is not retroactive.
    order: Arc<crate::source_graph::RunOrder>,
}

impl crate::namespace_import::NamespaceExportOracle for NamespaceExportSnapshot {
    fn exported_at(&self, source_ns: &str, name: &str, import_site: RunPoint<'_>) -> ExportVerdict {
        let ns = source_ns.trim_start_matches("::");
        if !self.observable.contains(ns) {
            // The namespace lives somewhere the workspace cannot see — an
            // installed package, a file outside the project. Silence is not
            // evidence, exactly as in `live_command_links`.
            return ExportVerdict::Unknown;
        }
        let exported = self.exports_by_ns.get(ns).is_some_and(|exports| {
            exported_at_site(exports.iter(), name, &self.order, import_site)
        });
        if exported {
            ExportVerdict::Exported
        } else {
            ExportVerdict::NotExported
        }
    }
}

/// The namespace an imported name is installed *into*, read off the link's
/// `linked_qname` (`::app::helper` → `::app`, a global-level import → `::`).
fn importing_namespace_of(linked_qname: &str) -> &str {
    linked_qname
        .rsplit_once("::")
        .map_or("::", |(ns, _)| if ns.is_empty() { "::" } else { ns })
}

/// The source namespace an import pattern names, reading the empty prefix a
/// global-rooted pattern (`::p`, `::*`) splits to as the global namespace it
/// actually is (#1104's review note).
fn global_rooted(source_ns: &str) -> &str {
    if source_ns.is_empty() {
        "::"
    } else {
        source_ns
    }
}

/// Namespace-name equality, ignoring a leading `::` one spelling carries and
/// the other does not.
fn ns_eq(a: &str, b: &str) -> bool {
    a.trim_start_matches("::") == b.trim_start_matches("::")
}

/// Where a `namespace import` sits, as the export-snapshot gate needs to see
/// it: the document, the offset, and the innermost proc/class body containing
/// it (`None` at load level).
///
/// The body span is what makes the workspace tier's ordering rule the *same*
/// rule the same-document tier applies rather than a weaker offset compare —
/// see [`WorkspaceGlobImport::enclosing_body`] and
/// [`tcl_compiler::analyser::indirection::in_effect_within`]. Carried as one
/// value so no caller can pass two of the three and silently drop the third.
#[derive(Debug, Clone, Copy)]
struct ImportSite<'a> {
    uri: &'a str,
    at: u32,
    enclosing_body: Option<Span>,
}

impl<'a> ImportSite<'a> {
    /// This site as the workspace-wide execution-timeline point
    /// [`crate::source_graph::RunOrder`] ranks events by.
    fn point(self) -> RunPoint<'a> {
        RunPoint {
            uri: self.uri,
            at: self.at,
            enclosing_body: self.enclosing_body,
        }
    }
}

/// Where the **call** being resolved sits: the document and the offset of its
/// command-head token.
///
/// The import edge's lifecycle is a question about the call, not about the
/// import — a `namespace forget` written after the import kills calls after
/// it and leaves calls before it alone (issue #1103) — so the resolver needs
/// the call's own position, which
/// [`WorkspaceIndex::resolve_wildcard_import`] previously never received.
///
/// Carries the call's own **enclosing body span** as well, for the same reason
/// [`WorkspaceGlobImport::enclosing_body`] exists on the import side: ordering
/// an import against a call is not a plain offset compare. A call inside a
/// proc body observes every top-level statement of its own file, wherever
/// written, because the whole file loads before any body runs — so a body-local
/// call of a name imported further down the same file still resolves, and a
/// top-level `namespace forget` written after such a body does revoke it
/// (issue #1116 item 3). Comparing offsets alone got the first wrong the
/// moment installs became order-gated (issue #1104 item 1), which is why the
/// span is no longer optional to supply.
///
/// `None` means "top level" — the same encoding
/// [`tcl_compiler::analyser::AnalysisResult::innermost_definition_body_span`]
/// uses, which is where every producer gets it: directly for a caller holding
/// the calling document's analysis, and from
/// [`WorkspaceInvocation::enclosing_body`] for the index-driven sweep.
#[derive(Debug, Clone, Copy)]
pub struct CallSite<'a> {
    /// Document the call is in.
    pub uri: &'a str,
    /// Byte offset of the call's command-head token within `uri`.
    pub at: u32,
    /// Span of the innermost proc/class **body** containing the call, within
    /// `uri`; `None` when the call sits at load level.
    pub enclosing_body: Option<Span>,
}

impl<'a> CallSite<'a> {
    /// This call as the workspace-wide execution-timeline point
    /// [`crate::source_graph::RunOrder`] ranks events against.
    fn point(self) -> RunPoint<'a> {
        RunPoint {
            uri: self.uri,
            at: self.at,
            enclosing_body: self.enclosing_body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        analyse_as(source, "tcl8.6")
    }

    fn analyse_as(source: &str, dialect: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, dialect).clone()
    }

    #[test]
    fn cross_file_define_stub_retraction_removes_the_method_from_dispatch() {
        // Issue #1101 review. A cross-file `oo::define` stub is already an
        // additive channel — a `method extra` written in b.tcl dispatches on a
        // class created in a.tcl — so a `deletemethod` written the same way has
        // to travel too, or the workspace advertises a method that sourcing the
        // extension deletes. Oracle, byte-identical on tclsh 9.0.4 / 8.6.14:
        //   oo::class create ::C { method m {} {…} }
        //   oo::define ::C { method extra {} {…} }
        //   oo::define ::C { deletemethod m }
        //   info class methods ::C  ->  extra
        //   [::C new] m             ->  unknown method "m": must be destroy or extra
        //   [::C new] extra         ->  2
        let a = analyse("oo::class create ::C { method m {} { return 1 } }\n");
        let b = analyse("oo::define ::C { method extra {} { return 2 } }\n");
        let d = analyse("oo::define ::C { deletemethod m }\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///d.tcl", &d),
        ]);
        // TP: the additive half still resolves (the channel this rides on).
        let extra = index.method_dispatch_chain("::C", "extra", MethodAccess::External);
        assert_eq!(
            extra.iter().map(|c| c.uri.as_str()).collect::<Vec<_>>(),
            ["file:///b.tcl"],
            "the cross-file additive channel must keep working",
        );
        // TP: the retraction now travels the same way.
        assert!(
            index
                .method_dispatch_chain("::C", "m", MethodAccess::External)
                .is_empty(),
            "a cross-file `deletemethod` must remove the method from dispatch",
        );
        // …and from internal (`my`) dispatch too — the member is gone, not
        // merely unexported.
        assert!(
            index
                .method_dispatch_chain("::C", "m", MethodAccess::Internal)
                .is_empty(),
        );
    }

    #[test]
    fn a_same_document_retraction_does_not_cancel_another_document() {
        // TN for the tombstone's scope. A retraction of a member the *same*
        // document declares is applied locally and leaves no tombstone, so a
        // second document that declares the name keeps it — cross-file order is
        // not knowable, and suppressing it would be an unsupported guess.
        let a = analyse(
            "oo::class create ::C {}\noo::define ::C { method m {} { return 1 }\n deletemethod m }\n",
        );
        let b = analyse("oo::define ::C { method m {} { return 2 } }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(
            index
                .method_dispatch_chain("::C", "m", MethodAccess::External)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///b.tcl"],
        );
    }

    #[test]
    fn a_cross_file_retraction_does_not_reach_an_inherited_provider() {
        // TN. Deleting a subclass's own override falls back to the superclass's
        // method rather than erasing the name — and deleting a member the
        // subclass never declared is a hard error, so a retraction never
        // crosses a class boundary. Oracle (9.0.4 / 8.6.14, identical):
        //   oo::class create B { method m {} {return base-m} }
        //   oo::class create D { superclass B; method m {} {return derived-m} }
        //   oo::define D { deletemethod m } ; [D new] m  ->  base-m
        //   oo::define D2 { deletemethod m }             ->  method m does not exist
        let b = analyse("oo::class create ::B { method m {} { return 1 } }\n");
        let d = analyse("oo::class create ::D {\n superclass ::B\n method m {} { return 2 }\n}\n");
        let x = analyse("oo::define ::D { deletemethod m }\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///b.tcl", &b),
            ("file:///d.tcl", &d),
            ("file:///x.tcl", &x),
        ]);
        assert_eq!(
            index
                .method_dispatch_chain("::D", "m", MethodAccess::External)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///b.tcl"],
            "the superclass's own `m` must still provide the dispatch entry",
        );
    }

    #[test]
    fn class_member_suppression_reads_the_same_chain_as_resolution() {
        // Issue #1168 — the predicate the in-document tier consults before
        // answering for a class its own document declares.  Oracle for the
        // suppressing shape (tclsh 9.0.4 / 8.6.14, byte-identical):
        //   a.tcl  oo::class create ::C { self { method cm {} { return 1 } } }
        //   b.tcl  oo::define ::C { self unexport cm }
        //   ::C cm  ->  unknown method "cm": must be create, destroy or new
        let a = analyse("oo::class create ::C { self { method cm {} { return 1 } } }\n");
        let b = analyse("oo::define ::C { self unexport cm }\n");
        let e = analyse("oo::define ::C { self export cm }\n");
        let d = analyse("oo::define ::C { self deletemethod cm }\n");

        // TP: a cross-file `self unexport` suppresses the external dispatch.
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::External));
        // …but the member is only unexported: internal dispatch still lands.
        assert!(!index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::Internal));

        // TP: a cross-file `self deletemethod` suppresses both accesses — the
        // member is gone, not merely unexported.
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///d.tcl", &d)]);
        assert!(index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::External));
        assert!(index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::Internal));

        // TN (abstention): with only the declaring record in view there is no
        // suppressing evidence, and the in-document answer must stand.
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        assert!(!index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::External));

        // TN (export-wins union): an exporting record anywhere keeps the
        // member dispatchable — the standing unordered-cross-file caveat.
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///e.tcl", &e),
        ]);
        assert!(!index.class_member_dispatch_suppressed("::C", "cm", MethodAccess::External));

        // TN (abstention): a class the index holds no record of yields no
        // suppression evidence at all.
        assert!(!index.class_member_dispatch_suppressed("::Ghost", "cm", MethodAccess::External));
    }

    #[test]
    fn a_cross_file_self_unexport_removes_the_member_from_class_dispatch() {
        // TP, issue #1119 — the whole point of the class-side channel. Oracle,
        // byte-identical on tclsh 9.0.4 and 8.6.14:
        //   a.tcl  oo::class create ::C { self { method cm {} { return cm } } }
        //          ::C cm  ->  cm
        //   b.tcl  oo::define ::C { self unexport cm }
        //          ::C cm  ->  unknown method "cm": must be create, destroy or new
        //          info object methods ::C -all -private  ->  … cm …
        // Before the channel existed the flip had nowhere to travel, so b.tcl's
        // `self unexport` was invisible and `::C cm` still resolved.
        let a = analyse("oo::class create ::C { self { method cm {} { return 1 } } }\n");
        let b = analyse("oo::define ::C { self unexport cm }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(
            index
                .class_method_dispatch_chain("::C", "cm", MethodAccess::External)
                .is_empty(),
            "a cross-file `self unexport` must remove the member from class dispatch",
        );
        // …but the member is only *unexported*, not gone: an internal (`my`)
        // dispatch still reaches it, exactly as `info object methods -all
        // -private` still lists it.
        assert_eq!(
            index
                .class_method_dispatch_chain("::C", "cm", MethodAccess::Internal)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///a.tcl"],
        );
    }

    #[test]
    fn a_cross_file_self_export_revives_a_class_side_member() {
        // TP, the other direction. `self export` travels the same way, and the
        // union rule is the class side's too: any record exporting the name
        // keeps it dispatchable, because true cross-file load order is not
        // knowable from the index.
        let a =
            analyse("oo::class create ::C { self { method Cm {} { return 1 }\n unexport Cm } }\n");
        let b = analyse("oo::define ::C { self export Cm }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(
            index
                .class_method_dispatch_chain("::C", "Cm", MethodAccess::External)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///a.tcl"],
        );
    }

    #[test]
    fn the_two_sides_visibility_channels_never_cross() {
        // TN (CRITICAL FP guard), issue #1098 + #1119. A class that defines the
        // same name on both sides must have each side answer for itself:
        //   a.tcl  oo::class create ::C { method m {} {…}
        //                                 self { method m {} {…} } }
        //   b.tcl  oo::define ::C { self unexport m }
        //   ::C m        ->  unknown method "m"     (class side flipped)
        //   [::C new] m  ->  inst-m                 (instance side untouched)
        let a = analyse(
            "oo::class create ::C { method m {} { return 1 }\n\
             self { method m {} { return 2 } } }\n",
        );
        let b = analyse("oo::define ::C { self unexport m }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(
            index
                .class_method_dispatch_chain("::C", "m", MethodAccess::External)
                .is_empty(),
            "the class side is unexported",
        );
        assert_eq!(
            index
                .method_dispatch_chain("::C", "m", MethodAccess::External)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///a.tcl"],
            "the instance side must be untouched by a `self unexport`",
        );
    }

    #[test]
    fn an_unwrapped_cross_file_unexport_leaves_the_class_side_dispatchable() {
        // TN, the mirror. Oracle: `oo::class create E2 { self { method onlyclass
        // {} {…} } }` then `oo::define E2 { unexport onlyclass }` is a silent
        // no-op and `::E2 onlyclass` still answers (9.0.4 / 8.6.14).
        let a = analyse("oo::class create ::C { self { method m {} { return 1 } } }\n");
        let b = analyse("oo::define ::C { unexport m }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(
            index
                .class_method_dispatch_chain("::C", "m", MethodAccess::External)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///a.tcl"],
        );
    }

    #[test]
    fn a_cross_file_self_deletemethod_empties_only_the_class_chain() {
        // TP + TN for the sided tombstone, which the class-side chain now reads
        // as its own. `self deletemethod m` in b.tcl removes the class-object
        // side's `m` and leaves an identically-named instance method alone.
        let a = analyse(
            "oo::class create ::C { method m {} { return 1 }\n\
             self { method m {} { return 2 } } }\n",
        );
        let b = analyse("oo::define ::C { self deletemethod m }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(
            index
                .class_method_dispatch_chain("::C", "m", MethodAccess::Internal)
                .is_empty(),
            "the class-side member is gone, not merely unexported",
        );
        assert!(
            !index
                .method_dispatch_chain("::C", "m", MethodAccess::External)
                .is_empty(),
            "an unwrapped-side member must survive a `self deletemethod`",
        );
    }

    #[test]
    fn a_cross_file_renamed_member_dispatches_under_its_new_name() {
        // TP, issue #1121 reaching the workspace. The rename happens inside
        // a.tcl, so the moved member is an ordinary indexed declaration by the
        // time b.tcl consumes it — `[::C new] new` really runs the old body
        // (`info class definition ::C new` -> `{} { return 1 }`, 9.0.4/8.6.14).
        let a =
            analyse("oo::class create ::C { method old {} { return 1 }\n renamemethod old new }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        assert_eq!(
            index
                .method_dispatch_chain("::C", "new", MethodAccess::External)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///a.tcl"],
        );
        assert!(
            index
                .method_dispatch_chain("::C", "old", MethodAccess::External)
                .is_empty(),
            "the source name must not survive the move",
        );
    }

    /// TP, issue #1167: the `renamemethod` sits in a **cross-file**
    /// `oo::define` stub, which has no `MethodDef` of its own to move.  The
    /// stub tombstones the source and records the arrival; the workspace join
    /// re-keys the defining file's record, so the member dispatches under its
    /// new name instead of disappearing.
    ///
    /// tclsh-proof (8.6.14) that this is what sourcing both files does:
    ///
    /// ```tcl
    /// oo::class create ::C { method old {} { return OLDBODY } }
    /// oo::define ::C { renamemethod old new }
    /// info class methods ::C   ;# -> new
    /// [::C new] new            ;# -> OLDBODY
    /// [::C new] old            ;# -> unknown method "old"
    /// ```
    #[test]
    fn a_cross_file_stub_renamemethod_records_the_arrival_name() {
        let a = analyse("oo::class create ::C { method old {} { return 1 } }\n");
        let b = analyse("oo::define ::C { renamemethod old new }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(
            index
                .method_dispatch_chain("::C", "new", MethodAccess::External)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///a.tcl"],
            "the arrival resolves to the record holding the body",
        );
        assert!(
            index
                .method_dispatch_chain("::C", "old", MethodAccess::External)
                .is_empty(),
            "the source name must not survive the move",
        );
        // The family resolution sees it too, so rename / references seed from
        // the new name rather than finding nothing.
        assert_eq!(
            index
                .method_override_family("::C", "new")
                .iter()
                .map(|c| c.qualified_name.as_str())
                .collect::<Vec<_>>(),
            ["::C", "::C"],
            "both records of the class are in the family",
        );
    }

    /// TN: visibility travels with the **body**, not the arrival name's
    /// leading-capital default.
    ///
    /// tclsh-proof (8.6.14): `oo::class create ::R4 { method Priv {} {…} }` +
    /// `oo::define ::R4 { renamemethod Priv pub }` leaves `info class methods
    /// ::R4` empty (the member is still unexported) while `info class methods
    /// ::R4 -private` lists `pub`.
    #[test]
    fn a_cross_file_arrival_keeps_the_sources_visibility() {
        let a = analyse("oo::class create ::C { method Priv {} { return 1 } }\n");
        let b = analyse("oo::define ::C { renamemethod Priv pub }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(
            index
                .method_dispatch_chain("::C", "pub", MethodAccess::External)
                .is_empty(),
            "the moved member is still unexported — the new name's default does not apply",
        );
        assert!(
            !index
                .method_dispatch_chain("::C", "pub", MethodAccess::Internal)
                .is_empty(),
            "…but it is a real member, reachable internally",
        );
    }

    /// TN: a plain cross-file `deletemethod` records no arrival, so nothing
    /// arrives — the tombstone keeps its original meaning.
    #[test]
    fn a_cross_file_deletemethod_records_no_arrival() {
        let a = analyse("oo::class create ::C { method m {} { return 1 } }\n");
        let b = analyse("oo::define ::C { deletemethod m }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(
            index
                .method_dispatch_chain("::C", "m", MethodAccess::External)
                .is_empty(),
        );
        assert!(
            index.classes().all(|c| !c.arrives_method("m")),
            "a deletion has no destination",
        );
    }

    /// TN: a **computed** destination (`renamemethod old $new`) names nothing
    /// statically, so the move abstains and only the retraction stands.
    #[test]
    fn a_computed_cross_file_arrival_abstains() {
        let a = analyse("oo::class create ::C { method old {} { return 1 } }\n");
        let b = analyse("oo::define ::C { renamemethod old $target }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(
            index
                .classes()
                .all(|c| c.retracted_members.iter().all(|r| r.arrival.is_none())),
            "a computed destination records no arrival",
        );
    }

    /// TP, issue #1263: **enumeration** joins the arrival channel too.  The
    /// defining record still declares the member as `old`, so the raw
    /// `WorkspaceClass::methods` table (which `workspace/symbol` used to read
    /// directly) advertised the pre-rename name after the resolution join had
    /// already been fixed for `dispatch_chain` (#1167).
    ///
    /// tclsh-proof (8.6.14), sourcing both files:
    ///
    /// ```tcl
    /// oo::class create ::C { method old {} { return OLDBODY } }
    /// oo::define ::C { renamemethod old new }
    /// info class methods ::C   ;# -> new
    /// [::C new] old            ;# -> unknown method "old"
    /// ```
    #[test]
    fn a_cross_file_arrival_re_keys_member_enumeration() {
        let a = analyse("oo::class create ::C { method old {} { return 1 } }\n");
        let b = analyse("oo::define ::C { renamemethod old new }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(
            index
                .effective_members("::C")
                .iter()
                .map(|em| em.name)
                .collect::<Vec<_>>(),
            ["new"],
            "the fold lists the member under the name the class dispatches it by",
        );
        // The body still lives in a.tcl; only the *name* moved.
        let em = index.effective_members("::C");
        let moved = em.first().expect("one member");
        assert_eq!(moved.declaring.uri, "file:///a.tcl");
        assert_eq!(moved.method.name, "old", "the declaring entry is untouched");
        assert_eq!(
            moved.name_uri, "file:///b.tcl",
            "the arrival word is the moved member's declaration site",
        );
        // …and `workspace/symbol` offers the new name, not the old one.
        let names: Vec<String> = index
            .symbols_matching("", 100)
            .into_iter()
            .filter(|s| s.kind == WorkspaceSymbolKind::Method)
            .map(|s| s.name)
            .collect();
        assert_eq!(
            names,
            ["new"],
            "the picker must not show the pre-rename name"
        );
    }

    /// TN, issue #1263: a cross-file `deletemethod` removes the member from
    /// enumeration outright — nothing arrives, so nothing is listed.
    ///
    /// tclsh-proof (8.6.14): `oo::class create ::C { method m {} {…} }` +
    /// `oo::define ::C { deletemethod m }` leaves `info class methods ::C`
    /// empty and `[::C new] m` erroring `unknown method "m"`.
    #[test]
    fn a_cross_file_deletion_drops_the_member_from_enumeration() {
        let a = analyse("oo::class create ::C { method m {} { return 1 } }\n");
        let b = analyse("oo::define ::C { deletemethod m }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(
            index.effective_members("::C").is_empty(),
            "a deleted member is not a member",
        );
        assert!(
            index
                .symbols_matching("m", 100)
                .iter()
                .all(|s| s.kind != WorkspaceSymbolKind::Method),
            "the picker must not offer a deleted member",
        );
    }

    /// TN, issue #1263: with no retraction anywhere, enumeration is exactly
    /// each record's own table — the fold must not perturb the ordinary case,
    /// including a member declared by a cross-file `oo::define` stub and one
    /// on the class-object side.
    #[test]
    fn enumeration_without_a_retraction_is_the_declared_table() {
        let a = analyse("oo::class create ::C { method one {} {}\n method two {} {} }\n");
        let b = analyse("oo::define ::C { method three {} {}\n self method cm {} {} }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(
            index
                .effective_members("::C")
                .iter()
                .map(|em| em.name)
                .collect::<Vec<_>>(),
            ["one", "two", "three", "cm"],
            "record order then declaration order, unchanged",
        );
        assert!(
            index
                .effective_members("::C")
                .iter()
                .all(|em| em.name == em.method.name),
            "no rename, so no re-keying",
        );
    }

    /// TN, issue #1263: a `self renamemethod` moves the **class-object** side
    /// only.  An identically-named instance method keeps its own name, which
    /// is the same side-scoping the tombstone channel already enforces for
    /// resolution (#1098 / #1119).
    #[test]
    fn a_cross_file_arrival_is_side_scoped_in_enumeration() {
        let a = analyse(
            "oo::class create ::C { method m {} { return 1 }\n self method m {} { return 2 } }\n",
        );
        let b = analyse("oo::define ::C { self renamemethod m cm }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        let mut listed: Vec<(&str, &str)> = index
            .effective_members("::C")
            .iter()
            .map(|em| (em.name, em.method.kind.as_str()))
            .collect();
        listed.sort_unstable();
        assert_eq!(
            listed,
            [("cm", "classmethod"), ("m", "method")],
            "only the class-object member moved",
        );
    }

    #[test]
    fn a_superseded_export_does_not_outrank_the_last_unexport() {
        // Issue #1101 review finding 3. `method m {} {}; export m; unexport m`
        // leaves `m` unexported in real Tcl ([L1 new] m -> unknown method,
        // 9.0.4 / 8.6.14) — but this chain reads *any* `exports` entry as
        // decisive, so recording the name in both sets made cross-file
        // go-to-definition treat a runtime-inaccessible method as public.
        // Covers the unwrapped spelling and the `private` one Codex cited, in
        // both writer orders.
        for (body, dialect, callable) in [
            ("export m\nunexport m", "tcl8.6", false),
            ("unexport m\nexport m", "tcl8.6", true),
            // `private` is a 9.0-only member word.
            ("private export m\nprivate unexport m", "tcl9.0", false),
            ("private unexport m\nprivate export m", "tcl9.0", true),
        ] {
            let src = format!("oo::class create ::C {{ method m {{}} {{ return 1 }}\n{body} }}\n");
            let a = analyse_as(&src, dialect);
            let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
            let chain = index.method_dispatch_chain("::C", "m", MethodAccess::External);
            assert_eq!(!chain.is_empty(), callable, "{body}");
        }
    }

    /// A call site at the end of `uri` — the offset only matters to the
    /// import-lifecycle gate (a `namespace forget` / source deletion earlier
    /// in the *same* document), so tests with no such event may use it
    /// freely.
    /// Byte offset of `needle` in a test source.
    fn app_src_offset(src: &str, needle: &str) -> usize {
        src.find(needle).expect("needle present")
    }

    fn call_from(uri: &str) -> CallSite<'_> {
        call_at(uri, u32::MAX)
    }

    /// A **top-level** call site at `at` in `uri` — the shape every ordering
    /// test below uses, since a load-level call carries no enclosing body.
    fn call_at(uri: &str, at: u32) -> CallSite<'_> {
        CallSite {
            uri,
            at,
            enclosing_body: None,
        }
    }

    #[test]
    fn cross_file_supertypes_and_subtypes() {
        // Base in a.tcl; Dog (subclass) in b.tcl; Puppy (subclass of Dog) in c.tcl.
        let a = analyse("oo::class create Animal {}\n");
        let b = analyse("oo::class create Dog {\n    superclass Animal\n}\n");
        let c = analyse("oo::class create Puppy {\n    superclass Dog\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///c.tcl", &c),
        ]);
        // Dog's superclass Animal resolves cross-file (a.tcl).
        let sup = index.classes_named("Animal");
        assert_eq!(sup.len(), 1);
        assert_eq!(sup[0].uri, "file:///a.tcl");
        // Animal's subclasses: Dog (b.tcl).
        let subs = index.subclasses_of("::Animal");
        let names: Vec<&str> = subs.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["Dog"]);
        // Dog's subclasses: Puppy (c.tcl).
        let dog_subs = index.subclasses_of("::Dog");
        assert_eq!(
            dog_subs.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            vec!["Puppy"]
        );
    }

    #[test]
    fn owner_aware_super_resolution_picks_same_namespace_and_abstains_on_ambiguity() {
        // Two `Base` classes in disjoint namespaces.  A subclass in ::A that
        // writes a bare `superclass Base` must link to ::A::Base (its own
        // namespace), never ::B::Base — and a subclass in a *third*
        // namespace with no local Base must abstain (ambiguous tail), not
        // guess.
        let a = analyse(
            "oo::class create ::A::Base {}\noo::class create ::A::Derived {\n    superclass Base\n}\n",
        );
        let b = analyse("oo::class create ::B::Base {}\n");
        let c = analyse("oo::class create ::C::Widget {\n    superclass Base\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///c.tcl", &c),
        ]);
        // ::A::Base's subclasses: only ::A::Derived (owner-aware pick).
        let a_subs: Vec<&str> = index
            .subclasses_of("::A::Base")
            .iter()
            .map(|c| c.qualified_name.as_str())
            .collect();
        assert_eq!(
            a_subs,
            vec!["::A::Derived"],
            "owner-aware resolution mis-linked"
        );
        // ::B::Base gets no subclass from the ambiguous bare `Base` names.
        assert!(
            index.subclasses_of("::B::Base").is_empty(),
            "ownerless tail match manufactured a wrong subtype edge",
        );
        // ::C::Widget's supertypes abstain (Base is ambiguous, ::C has none).
        let widget = index
            .classes()
            .find(|c| c.qualified_name == "::C::Widget")
            .expect("Widget indexed");
        assert!(
            index.supertype_classes(widget).is_empty(),
            "ambiguous bare superclass should resolve to nothing",
        );
    }

    #[test]
    fn cross_file_method_override_family() {
        // Base `speak` in a.tcl; Dog overrides it in b.tcl; Cat overrides it
        // in c.tcl; unrelated Engine::speak in d.tcl must stay out.
        let animal = analyse("oo::class create Animal {\n    method speak {} {}\n}\n");
        let dog =
            analyse("oo::class create Dog {\n    superclass Animal\n    method speak {} {}\n}\n");
        let cat =
            analyse("oo::class create Cat {\n    superclass Animal\n    method speak {} {}\n}\n");
        let engine = analyse("oo::class create Engine {\n    method speak {} {}\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &animal),
            ("file:///b.tcl", &dog),
            ("file:///c.tcl", &cat),
            ("file:///d.tcl", &engine),
        ]);
        // Seed from Dog: family = Animal + Dog + Cat (across three files).
        let mut fam: Vec<&str> = index
            .method_override_family("::Dog", "speak")
            .iter()
            .map(|wc| wc.qualified_name.as_str())
            .collect();
        fam.sort_unstable();
        fam.dedup();
        assert_eq!(
            fam,
            vec!["::Animal", "::Cat", "::Dog"],
            "cross-file family wrong"
        );
        // Unrelated Engine::speak must not be pulled in.
        assert!(
            !index
                .method_override_family("::Dog", "speak")
                .iter()
                .any(|wc| wc.qualified_name == "::Engine"),
            "unrelated same-named method must stay out of the family",
        );
        // Seeding from a class that only *inherits* speak still finds the
        // family via the providing ancestor.
        let puppy = analyse("oo::class create Puppy {\n    superclass Dog\n}\n");
        let index2 = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &animal),
            ("file:///b.tcl", &dog),
            ("file:///e.tcl", &puppy),
        ]);
        let fam2: Vec<&str> = index2
            .method_override_family("::Puppy", "speak")
            .iter()
            .map(|wc| wc.qualified_name.as_str())
            .collect();
        assert!(
            fam2.contains(&"::Animal") && fam2.contains(&"::Dog"),
            "{fam2:?}"
        );
    }

    #[test]
    fn cross_file_method_inheritor_classes() {
        // Base `speak` in a.tcl; a purely-inheriting Dog (no override) in
        // b.tcl; an unrelated Engine::speak hierarchy with its own inheritor
        // Car in c.tcl/d.tcl.  Seeding from Animal, Dog is an inheritor and
        // Car (disjoint hierarchy) is not.
        let animal = analyse("oo::class create Animal {\n    method speak {} {}\n}\n");
        let dog = analyse(
            "oo::class create Dog {\n    superclass Animal\n    method describe {} { my speak }\n}\n",
        );
        let engine = analyse("oo::class create Engine {\n    method speak {} {}\n}\n");
        let car = analyse("oo::class create Car {\n    superclass Engine\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &animal),
            ("file:///b.tcl", &dog),
            ("file:///c.tcl", &engine),
            ("file:///d.tcl", &car),
        ]);
        let inheritors: Vec<&str> = index
            .method_inheritor_classes("::Animal", "speak")
            .iter()
            .map(|wc| wc.qualified_name.as_str())
            .collect();
        assert_eq!(inheritors, vec!["::Dog"], "{inheritors:?}");
        // A definer is never returned as an inheritor.
        assert!(
            !index
                .method_inheritor_classes("::Animal", "speak")
                .iter()
                .any(|wc| wc.qualified_name == "::Animal"),
        );
    }

    #[test]
    fn method_inheritor_abstains_on_disjoint_definer_ancestor() {
        // A class that multiply-inherits from two unrelated definers of the
        // same method could resolve to either; the family seeded from only one
        // must NOT claim it (sound abstention, no over-rename).
        let a = analyse("oo::class create A {\n    method run {} {}\n}\n");
        let b = analyse("oo::class create B {\n    method run {} {}\n}\n");
        let both = analyse("oo::class create Both {\n    superclass A B\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///both.tcl", &both),
        ]);
        // Family seeded from A does not include B, so `Both` (which can reach
        // B::run too) is abstained on.
        assert!(
            index.method_inheritor_classes("::A", "run").is_empty(),
            "must abstain when an out-of-family definer ancestor exists",
        );
    }

    #[test]
    fn indexes_procs_from_multiple_documents() {
        let a = analyse("proc greet {name} {}\n");
        let b = analyse("proc farewell {} {}\nproc greet2 {x y} {}\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(index.procs().count(), 3);
        // Param counts captured.
        let greet = index.procs().find(|p| p.name == "greet").unwrap();
        assert_eq!(greet.param_count, 1);
        assert_eq!(greet.uri, "file:///a.tcl");
    }

    #[test]
    fn procs_matching_excludes_current_doc_and_filters_prefix() {
        let a = analyse("proc alpha {} {}\n");
        let b = analyse("proc alphabet {} {}\nproc beta {} {}\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        // From a.tcl's perspective, only b.tcl procs with `alph`.
        let got = index.procs_matching("alph", "file:///a.tcl");
        let names: Vec<&str> = got.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["alphabet"]);
    }

    #[test]
    fn proc_definitions_resolves_cross_document() {
        let a = analyse("proc helper {} {}\n");
        let b = analyse("helper\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        // From b.tcl, `helper` resolves to a.tcl's definition.
        let defs = index.proc_definitions("helper", "file:///b.tcl");
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].uri, "file:///a.tcl");
        // The same-document exclusion drops it when querying
        // from a.tcl itself.
        assert!(index.proc_definitions("helper", "file:///a.tcl").is_empty());
    }

    #[test]
    fn remove_document_drops_its_entries() {
        let a = analyse("proc a {} {}\n");
        let b = analyse("proc b {} {}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        index.add_document("file:///b.tcl", &b);
        assert_eq!(index.procs().count(), 2);
        index.remove_document("file:///a.tcl");
        assert_eq!(index.procs().count(), 1);
        assert_eq!(index.procs().next().map(|p| p.name.as_str()), Some("b"));
    }

    /// Issue #1149: a removal is scoped to the removed document.  Every other
    /// document's records survive it untouched, in their original order —
    /// which is what lets the removal cost that one document's rows instead of
    /// a pass over every table in the workspace.
    #[test]
    fn remove_document_leaves_the_other_documents_alone() {
        let a = analyse("proc a {} {}\na\n");
        let b = analyse("proc b {} {}\nb\n");
        let c = analyse("proc c {} {}\nc\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        index.add_document("file:///b.tcl", &b);
        index.add_document("file:///c.tcl", &c);
        index.remove_document("file:///b.tcl");
        let names: Vec<&str> = index.procs().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["a", "c"]);
        let call_uris: std::collections::BTreeSet<&str> =
            index.invocations().map(|i| i.uri.as_str()).collect();
        assert_eq!(
            call_uris,
            ["file:///a.tcl", "file:///c.tcl"].into_iter().collect()
        );
        assert_eq!(
            index.document_uris(),
            vec!["file:///a.tcl", "file:///c.tcl"]
        );
    }

    /// Issue #1149: the remove-then-add every diagnostics publish performs must
    /// neither grow the index nor shuffle the workspace — the document goes
    /// back into the slot it just vacated.
    #[test]
    fn re_indexing_a_document_keeps_its_size_and_its_place() {
        let a = analyse("proc a {} {}\n");
        let b = analyse("proc b {} {}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        index.add_document("file:///b.tcl", &b);
        for _ in 0..5 {
            index.remove_document("file:///a.tcl");
            index.add_document("file:///a.tcl", &a);
        }
        let names: Vec<&str> = index.procs().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    /// Issue #1149: removing a URI the index never held changes nothing.
    #[test]
    fn removing_an_unindexed_document_changes_nothing() {
        let a = analyse("proc a {} {}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        index.remove_document("file:///never-indexed.tcl");
        assert_eq!(index.procs().count(), 1);
    }

    /// Several analyses of one URI — M9's one re-homed view per source-site
    /// namespace — still accumulate under that URI, and one removal drops the
    /// whole set.
    #[test]
    fn several_views_of_one_document_accumulate_and_drop_together() {
        let a = analyse("proc helper {} {}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        index.add_document("file:///a.tcl", &a);
        assert_eq!(index.procs().count(), 2);
        index.remove_document("file:///a.tcl");
        assert_eq!(index.procs().count(), 0);
    }

    #[test]
    fn indexes_classes() {
        let a = analyse("oo::class create Widget {}\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        let defs = index.class_definitions("Widget", "file:///other.tcl");
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].qualified_name, "::Widget");
    }

    #[test]
    fn indexes_invocation_sites_per_document() {
        // a.tcl defines `helper`; b.tcl calls it twice.
        let a = analyse("proc helper {} {}\n");
        let b = analyse("helper\nhelper\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        // From a.tcl's view, the two calls live in b.tcl.
        let calls = index.invocations_of("::helper", "file:///a.tcl");
        assert_eq!(calls.len(), 2, "{calls:?}");
        assert!(calls.iter().all(|c| c.uri == "file:///b.tcl"));
    }

    #[test]
    fn invocations_of_excludes_current_doc() {
        let a = analyse("proc helper {} {}\nhelper\n");
        let b = analyse("helper\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        // Excluding a.tcl leaves only b.tcl's call.
        let calls = index.invocations_of("::helper", "file:///a.tcl");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].uri, "file:///b.tcl");
    }

    #[test]
    fn invocations_of_finds_namespaced_call_all_spellings() {
        // A namespaced proc in a.tcl, called three ways from b.tcl: fully
        // qualified, relative-qualified, and bare from inside the namespace.
        let a = analyse("namespace eval ns {\n    proc helper {} {}\n}\n");
        let b = analyse("::ns::helper\nns::helper\nnamespace eval ns {\n    helper\n}\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        let calls = index.invocations_of("::ns::helper", "file:///a.tcl");
        assert_eq!(calls.len(), 3, "{calls:?}");
    }

    #[test]
    fn invocations_of_resolves_namespace_path_across_files() {
        // The confirmed #923 trigger: a bare call reaches a namespaced proc in
        // *another* file via `namespace path`, while an unrelated file defines
        // the same simple name (which used to disable the bare-name fallback).
        // The file-local guess settles to `::app::helper` (the caller's
        // namespace), so only the workspace-wide candidate resolution finds it.
        let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
        let other = analyse("namespace eval ::other { proc helper {} {} }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace path ::mymod\n    proc run {} { helper }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///other.tcl", &other),
            ("file:///app.tcl", &app),
        ]);
        // The call resolves to `::mymod::helper` via the namespace path.
        let refs = index.invocations_of("::mymod::helper", "file:///mymod.tcl");
        assert_eq!(refs.len(), 1, "{refs:?}");
        assert_eq!(refs[0].uri, "file:///app.tcl");
    }

    #[test]
    fn invocations_of_does_not_cross_link_the_colliding_namespace() {
        // The same call must NOT be reported as a reference of the *other*
        // same-named proc: `namespace path ::mymod` resolves it to
        // `::mymod::helper`, never `::other::helper`.
        let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
        let other = analyse("namespace eval ::other { proc helper {} {} }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace path ::mymod\n    proc run {} { helper }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///other.tcl", &other),
            ("file:///app.tcl", &app),
        ]);
        let refs = index.invocations_of("::other::helper", "file:///other.tcl");
        assert!(refs.is_empty(), "{refs:?}");
    }

    #[test]
    fn bare_call_without_path_does_not_reach_unrelated_namespace() {
        // A bare `helper` in `::app` with no `namespace path` and no local
        // `::app::helper` resolves to nothing (real tclsh: invalid command
        // name), so it is a reference of neither namespaced proc.
        let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
        let app = analyse("namespace eval ::app {\n    proc run {} { helper }\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let refs = index.invocations_of("::mymod::helper", "file:///mymod.tcl");
        assert!(refs.is_empty(), "{refs:?}");
    }

    #[test]
    fn namespace_import_call_site_references_the_source_command() {
        // `::app` imports `::mymod::helper`, then calls a bare `helper`.  The
        // call names the local imported `::app::helper`, which runs
        // `::mymod::helper` — so it is a reference to the source command.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        // Following the import link, the bare call resolves to the source.
        let refs = index.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
        assert_eq!(refs.len(), 1, "{refs:?}");
        assert_eq!(refs[0].uri, "file:///app.tcl");
        // The direct-only resolver (which rename uses) must NOT rewrite that
        // call: it names the local imported command, not the source.
        assert!(
            index
                .invocations_of("::mymod::helper", "file:///mymod.tcl")
                .is_empty(),
            "direct resolver must not claim the imported call site",
        );
        // The import pattern token is a defining-side reference rename rewrites.
        let spans = index.link_target_spans("::mymod::helper", "file:///mymod.tcl");
        assert_eq!(spans.len(), 1, "{spans:?}");
        assert_eq!(spans[0].0, "file:///app.tcl");
    }

    #[test]
    fn interp_alias_call_site_references_the_target_command() {
        // `a` aliases `::mymod::helper`; a bare `a` call runs the target.  The
        // alias `TARGET` word is itself a first-class invocation (a command
        // prefix), so references see two sites: the `TARGET` word and the `a`
        // call reaching the target through the alias link.
        let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
        let app = analyse("interp alias {} a {} ::mymod::helper\na\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let refs = index.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
        assert!(
            refs.iter().any(|r| r.name == "a"),
            "the aliased call should reference the target: {refs:?}",
        );
        // The direct-only resolver (rename) sees just the `TARGET` word, never
        // the `a` call — that call names the alias, which keeps its own name.
        let direct = index.invocations_of("::mymod::helper", "file:///mymod.tcl");
        assert!(
            direct.iter().all(|r| r.name != "a"),
            "rename must not rewrite the alias call site: {direct:?}",
        );
        // The alias `TARGET` word needs no separate link span — it is already
        // an invocation the ordinary reference/rename path covers.
        assert!(
            index
                .link_target_spans("::mymod::helper", "file:///mymod.tcl")
                .is_empty(),
        );
    }

    #[test]
    fn rename_new_name_call_site_references_the_old_command() {
        // `rename ::mymod::helper h` makes `h` run what `::mymod::helper`
        // was. Same shape as the `interp alias` case above (issue #923 idx
        // 39): the `OLD` word is itself a first-class invocation, so
        // references see two sites — the `OLD` word and the `h` call
        // reaching the target through the rename link.
        let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
        let app = analyse("rename ::mymod::helper h\nh\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let refs = index.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
        assert!(
            refs.iter().any(|r| r.name == "h"),
            "the renamed call should reference the target: {refs:?}",
        );
        assert!(
            refs.iter()
                .any(|r| r.name == "::mymod::helper" && r.uri == "file:///app.tcl"),
            "the rename's own OLD word is a reference too: {refs:?}",
        );
        // The direct-only resolver (rename) sees just the `OLD` word, never
        // the `h` call — that call names the local renamed alias, which
        // keeps its own name.
        let direct = index.invocations_of("::mymod::helper", "file:///mymod.tcl");
        assert!(
            direct.iter().all(|r| r.name != "h"),
            "rename must not rewrite the renamed-to call site: {direct:?}",
        );
        // The `OLD` word needs no separate link span — it is already an
        // invocation the ordinary reference/rename path covers.
        assert!(
            index
                .link_target_spans("::mymod::helper", "file:///mymod.tcl")
                .is_empty(),
        );
    }

    #[test]
    fn resolve_command_target_follows_a_chain_and_leaves_plain_names() {
        // `b` aliases `a`, `a` aliases `::mymod::helper`: `b` ultimately runs
        // the source.  A name with no link is returned unchanged.
        let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
        let app = analyse("interp alias {} a {} ::mymod::helper\ninterp alias {} b {} a\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert_eq!(index.resolve_command_target("::b"), "::mymod::helper");
        assert_eq!(index.resolve_command_target("::a"), "::mymod::helper");
        assert_eq!(
            index.resolve_command_target("::mymod::helper"),
            "::mymod::helper"
        );
        // A bare call through the two-hop alias still resolves to the source.
        let app2 = analyse("interp alias {} a {} ::mymod::helper\ninterp alias {} b {} a\nb\n");
        let index2 = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app2),
        ]);
        let refs = index2.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
        assert!(
            refs.iter().any(|r| r.name == "b"),
            "two-hop aliased call should reference the source: {refs:?}",
        );
    }

    #[test]
    fn glob_import_introduces_no_command_link() {
        // `namespace import ::mymod::*` names no single command, so it must
        // not manufacture a link that a bare call could resolve through.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        // A glob import records no `::app::helper` link, so the bare call does
        // not resolve to the source through a (non-existent) link.
        assert!(
            index
                .link_target_spans("::mymod::helper", "file:///mymod.tcl")
                .is_empty(),
            "glob import should record no link span",
        );
    }

    // Cross-document wildcard-import resolution (issue #923 idx 18):
    // `resolve_wildcard_import` is the mechanism that DOES resolve a bare
    // call through the glob import `glob_import_introduces_no_command_link`
    // (above) proves records no fixed link for.

    #[test]
    fn resolve_wildcard_import_resolves_exported_proc_cross_document() {
        // TP — `::mymod` (a.k.a. `mymod.tcl`) exports `helper`; `app.tcl`
        // wildcard-imports it and calls it bare from inside `run`, whose
        // own namespace is `::app` (a proc runs in the namespace it was
        // defined in, regardless of call site).
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
    }

    #[test]
    fn resolve_wildcard_import_does_not_resolve_unexported_sibling_cross_document() {
        // FP guard (CRITICAL) — `::mymod` also declares `other`, but never
        // exports it; real Tcl's `namespace import ::mymod::*` never binds
        // it, so the resolver must abstain (matching the in-document
        // `wildcard_namespace_import_unexported_sibling_stays_unresolved`
        // guard in `definition.rs`).
        let mymod = analyse(
            "namespace eval ::mymod {\n    proc helper {} {}\n    proc other {} {}\n    namespace export helper\n}\n",
        );
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { other }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let resolved = index.resolve_wildcard_import(
            "other",
            &["::app::other".to_string(), "::other".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert!(resolved.is_none(), "{resolved:?}");
    }

    #[test]
    fn resolve_wildcard_import_resolves_exported_class_cross_document() {
        // TP — differential-audit finding idx 29 (main audit wave):
        // the finding's own repro shape (analogous to the real
        // georgtree_tclopt corpus's `tclopt.tcl` exporting a TclOO class,
        // `examples/*.tcl` wildcard-importing and instantiating it bare).
        // Already fixed by idx 18's shared cross-document mechanism
        // (`workspace_command_exists`/`defined_command_names` cover
        // classes exactly like procs); pinned here as dedicated coverage
        // since the idx 18 diff itself only unit-tested the proc case.
        let mypkg = analyse(
            "namespace eval ::mypkg {\n    namespace export Widget\n    oo::class create Widget {\n        method run {} { return 42 }\n    }\n}\n",
        );
        let consumer = analyse("namespace import ::mypkg::*\nset w [Widget new]\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mypkg.tcl", &mypkg),
            ("file:///consumer.tcl", &consumer),
        ]);
        let resolved = index.resolve_wildcard_import(
            "Widget",
            &["::Widget".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert_eq!(resolved.as_deref(), Some("::mypkg::Widget"));
    }

    #[test]
    fn resolve_wildcard_import_is_not_restricted_to_the_calling_document() {
        // TP, regression guard — `namespace import` binds to the
        // *namespace*, not the file that wrote it: real Tcl reopening
        // `::app` in a later `namespace eval ::app { ... }` block sees
        // every import already recorded for `::app`, regardless of which
        // file recorded it. A common real-world shape is a shared
        // "imports.tcl" that does the import once, with sibling files
        // reopening the same namespace to call the bare name. Three
        // separate files: `mymod.tcl` exports `helper`; `imports.tcl` does
        // `namespace eval ::app { namespace import ::mymod::* }`;
        // `caller.tcl` does `namespace eval ::app { proc run {} { helper } }`
        // — the import statement and the call site are never in the same
        // document. An earlier version of this fix filtered candidate
        // glob imports by `g.uri == uri` (the calling document), which
        // this shape never satisfies — found by adversarial review before
        // being shipped.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let imports = analyse("namespace eval ::app {\n    namespace import ::mymod::*\n}\n");
        let caller = analyse("namespace eval ::app {\n    proc run {} { helper }\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///imports.tcl", &imports),
            ("file:///caller.tcl", &caller),
        ]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
    }

    // Per-import-site export snapshots, cross-document tier (issue #1027).
    // The workspace resolver applies the same shared decision function the
    // same-document one does (`namespace_import::exported_at_import_site`),
    // so the two tiers cannot disagree — but only events in the *import's own
    // document* are ordered against it; another file's load order is not a
    // static fact.

    #[test]
    fn resolve_wildcard_import_survives_a_later_export_clear_in_the_import_file() {
        // TP, direction A, cross-document — the import and the later
        // `namespace export -clear` are both in `app.tcl`, so they *are*
        // ordered: the `-clear` runs after the import and cannot revoke what
        // it bound (oracle tclsh 8.6.14/9.0.4). `::mymod`'s own export is in
        // the other file and unordered, which the shared function keeps.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export -clear\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
    }

    // ---- the `source` graph orders what the file boundary did not --------
    //
    // Issue #1104 item 3 / #1116 item 6. Sourcing a file inlines its whole
    // body at the `source` statement's position, so the DFS of the source
    // forest *is* the run order. Oracle for every case below, byte-identical
    // on tclsh 8.6.14 and 9.0.4 — with
    //
    //   # exp.tcl:  namespace eval ::mymod { namespace export helper }
    //   # imp.tcl:  namespace eval ::app   { namespace import ::mymod::* }
    //   # mod.tcl:  namespace eval ::mymod { proc helper {} {return HELP} }
    //
    //   # app.tcl:  source mod.tcl; source exp.tcl; source imp.tcl
    //   ::app::helper   ->  HELP
    //   # app.tcl:  source mod.tcl; source imp.tcl; source exp.tcl
    //   ::app::helper   ->  invalid command name "::app::helper"

    /// The server's URI resolver, in miniature: a literal path resolved
    /// against the sourcing document's directory.
    fn test_resolve(parent_uri: &str, raw_path: &str, is_literal: bool) -> Option<String> {
        let parent = parent_uri.strip_prefix("file://")?;
        let dir = std::path::Path::new(parent).parent()?;
        // The server's two tiers, in miniature: a literal resolves directly, a
        // computed path only when it folds statically — anything else names no
        // document and sequences nothing.
        let raw = if is_literal {
            raw_path.to_owned()
        } else {
            tcl_compiler::auto_path_eval::evaluate_auto_path_expr(raw_path, Some(parent))?
        };
        let child = crate::source_graph::resolve_under(dir, &raw);
        Some(format!("file://{}", child.display()))
    }

    /// An index over `documents` with the `source`-path resolver installed —
    /// the shape the real server builds ([`WorkspaceIndex::set_source_resolver`]).
    fn sourced_index<'a>(
        documents: impl IntoIterator<Item = (&'a str, &'a AnalysisResult)>,
    ) -> WorkspaceIndex {
        let mut index = WorkspaceIndex::new();
        index.set_source_resolver(test_resolve);
        for (uri, analysis) in documents {
            index.add_document(uri, analysis);
        }
        index
    }

    /// The three module documents every source-order test below shares.
    fn import_order_modules() -> (AnalysisResult, AnalysisResult, AnalysisResult) {
        (
            analyse("namespace eval ::mymod { proc helper {} { return HELP } }\n"),
            analyse("namespace eval ::mymod { namespace export helper }\n"),
            analyse("namespace eval ::app { namespace import ::mymod::* }\n"),
        )
    }

    #[test]
    fn an_export_sourced_before_the_import_resolves() {
        // TP: `app.tcl` sources the export before the import, so the import
        // really did see it — the same answer as before, but now as a proved
        // order rather than an abstention.
        let (mod_doc, exp, imp) = import_order_modules();
        let app = analyse("source mod.tcl\nsource exp.tcl\nsource imp.tcl\n");
        let index = sourced_index([
            ("file:///p/mod.tcl", &mod_doc),
            ("file:///p/exp.tcl", &exp),
            ("file:///p/imp.tcl", &imp),
            ("file:///p/app.tcl", &app),
        ]);
        assert_eq!(
            index
                .resolve_wildcard_import(
                    "helper",
                    &["::app::helper".to_string(), "::helper".to_string()],
                    call_from("file:///p/caller.tcl"),
                )
                .as_deref(),
            Some("::mymod::helper"),
        );
    }

    #[test]
    fn an_export_sourced_after_the_import_is_not_retroactive() {
        // FP guard (CRITICAL), issue #1104 item 3 — the whole point of the
        // order. The import runs first, `::mymod` has exported nothing yet,
        // so real Tcl installs no alias at all. Byte-identical documents to
        // the test above; only `app.tcl`'s two `source` lines swap.
        let (mod_doc, exp, imp) = import_order_modules();
        let app = analyse("source mod.tcl\nsource imp.tcl\nsource exp.tcl\n");
        let index = sourced_index([
            ("file:///p/mod.tcl", &mod_doc),
            ("file:///p/exp.tcl", &exp),
            ("file:///p/imp.tcl", &imp),
            ("file:///p/app.tcl", &app),
        ]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///p/caller.tcl"),
        );
        assert!(
            resolved.is_none(),
            "an export sourced after the import cannot apply retroactively: {resolved:?}",
        );
    }

    #[test]
    fn without_a_resolver_the_same_workspace_keeps_abstaining() {
        // TN for the deployment shape: an index with no `source` resolver
        // installed holds the pre-#1104-item-3 behaviour exactly — the
        // foreign export counts and the import resolves, whichever way the
        // `source` statements are written.
        let (mod_doc, exp, imp) = import_order_modules();
        let app = analyse("source mod.tcl\nsource imp.tcl\nsource exp.tcl\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///p/mod.tcl", &mod_doc),
            ("file:///p/exp.tcl", &exp),
            ("file:///p/imp.tcl", &imp),
            ("file:///p/app.tcl", &app),
        ]);
        assert_eq!(
            index
                .resolve_wildcard_import(
                    "helper",
                    &["::app::helper".to_string(), "::helper".to_string()],
                    call_from("file:///p/caller.tcl"),
                )
                .as_deref(),
            Some("::mymod::helper"),
        );
    }

    #[test]
    fn a_re_sourced_export_file_keeps_abstaining() {
        // TN (CRITICAL): `exp.tcl` is sourced twice, so it has no unique
        // position and the order must not invent one — Tcl tolerates
        // re-sourcing, and guessing would silently drop a real alias. Falls
        // back to the pre-graph abstention: the export counts.
        let (mod_doc, exp, imp) = import_order_modules();
        let app = analyse("source mod.tcl\nsource imp.tcl\nsource exp.tcl\nsource exp.tcl\n");
        let index = sourced_index([
            ("file:///p/mod.tcl", &mod_doc),
            ("file:///p/exp.tcl", &exp),
            ("file:///p/imp.tcl", &imp),
            ("file:///p/app.tcl", &app),
        ]);
        assert_eq!(
            index
                .resolve_wildcard_import(
                    "helper",
                    &["::app::helper".to_string(), "::helper".to_string()],
                    call_from("file:///p/caller.tcl"),
                )
                .as_deref(),
            Some("::mymod::helper"),
        );
    }

    #[test]
    fn a_computed_source_path_orders_nothing() {
        // TN: `source $dir/exp.tcl` names no document statically, so the edge
        // is dropped and the export goes back to being unrankable — the
        // deliberate abstention, not an accident of path resolution.
        let (mod_doc, exp, imp) = import_order_modules();
        let app = analyse(
            "set dir [file dirname [info script]]\nsource mod.tcl\nsource imp.tcl\nsource $dir/exp.tcl\n",
        );
        let index = sourced_index([
            ("file:///p/mod.tcl", &mod_doc),
            ("file:///p/exp.tcl", &exp),
            ("file:///p/imp.tcl", &imp),
            ("file:///p/app.tcl", &app),
        ]);
        assert_eq!(
            index
                .resolve_wildcard_import(
                    "helper",
                    &["::app::helper".to_string(), "::helper".to_string()],
                    call_from("file:///p/caller.tcl"),
                )
                .as_deref(),
            Some("::mymod::helper"),
        );
    }

    #[test]
    fn a_forget_written_after_the_source_revokes_the_sourced_import() {
        // TP (CRITICAL), the `source lib.tcl ; namespace forget ::lib::p`
        // idiom #1104 item 3 and #1116 called out by name: the install from
        // the sourced file counts (it always did) *and* the forget beside the
        // `source` now revokes it. Oracle: `::app::helper` is an `invalid
        // command name` after the forget.
        let (mod_doc, exp, imp) = import_order_modules();
        let app = analyse(
            "source mod.tcl\nsource exp.tcl\nsource imp.tcl\nnamespace eval ::app { namespace forget ::mymod::helper }\n",
        );
        let index = sourced_index([
            ("file:///p/mod.tcl", &mod_doc),
            ("file:///p/exp.tcl", &exp),
            ("file:///p/imp.tcl", &imp),
            ("file:///p/app.tcl", &app),
        ]);
        // A call written after the forget no longer resolves…
        let after = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_at("file:///p/app.tcl", u32::MAX),
        );
        assert!(
            after.is_none(),
            "a forget written after the `source` that installed the alias must revoke it: {after:?}",
        );
        // …and a call written *above* every `source` statement has no alias
        // yet either — the install has not run at that point.
        assert_eq!(
            index
                .resolve_wildcard_import(
                    "helper",
                    &["::app::helper".to_string(), "::helper".to_string()],
                    call_at("file:///p/app.tcl", 0),
                )
                .as_deref(),
            None,
        );
        // TP: with the forget removed, the same call site after the sources
        // does resolve — so the assertion above is the forget's doing and not
        // an accident of the graph.
        let app_no_forget = analyse("source mod.tcl\nsource exp.tcl\nsource imp.tcl\n");
        let index = sourced_index([
            ("file:///p/mod.tcl", &mod_doc),
            ("file:///p/exp.tcl", &exp),
            ("file:///p/imp.tcl", &imp),
            ("file:///p/app.tcl", &app_no_forget),
        ]);
        assert_eq!(
            index
                .resolve_wildcard_import(
                    "helper",
                    &["::app::helper".to_string(), "::helper".to_string()],
                    call_at("file:///p/app.tcl", u32::MAX),
                )
                .as_deref(),
            Some("::mymod::helper"),
        );
    }

    #[test]
    fn a_cross_file_import_conflict_is_decided_by_the_source_order() {
        // TP (CRITICAL), issue #1116 item 6 — two imports of one name from
        // different sources, in different files. Without an order neither
        // conflicts and the later one silently installs; with one, the file
        // sourced first owns the name and the second import raises `can't
        // import command "helper": already exists` and installs nothing.
        //
        // Oracle (8.6.14 / 9.0.4), with the two importers in separate files
        // sourced in this order:
        //   namespace origin ::app::helper  ->  ::first::helper
        let first = analyse(
            "namespace eval ::first { proc helper {} { return F }\n namespace export helper }\n",
        );
        let second = analyse(
            "namespace eval ::second { proc helper {} { return S }\n namespace export helper }\n",
        );
        let imp_a = analyse("namespace eval ::app { namespace import ::first::* }\n");
        let imp_b = analyse("namespace eval ::app { namespace import ::second::* }\n");
        let app =
            analyse("source first.tcl\nsource second.tcl\nsource impa.tcl\nsource impb.tcl\n");
        let docs = [
            ("file:///p/first.tcl", &first),
            ("file:///p/second.tcl", &second),
            ("file:///p/impa.tcl", &imp_a),
            ("file:///p/impb.tcl", &imp_b),
            ("file:///p/app.tcl", &app),
        ];
        assert_eq!(
            sourced_index(docs)
                .resolve_wildcard_import(
                    "helper",
                    &["::app::helper".to_string(), "::helper".to_string()],
                    call_from("file:///p/caller.tcl"),
                )
                .as_deref(),
            Some("::first::helper"),
            "the import sourced first owns the name; the second conflicts and installs nothing",
        );
        // Swap the two `source` lines and the winner swaps with them.
        let app =
            analyse("source first.tcl\nsource second.tcl\nsource impb.tcl\nsource impa.tcl\n");
        let docs = [
            ("file:///p/first.tcl", &first),
            ("file:///p/second.tcl", &second),
            ("file:///p/impa.tcl", &imp_a),
            ("file:///p/impb.tcl", &imp_b),
            ("file:///p/app.tcl", &app),
        ];
        assert_eq!(
            sourced_index(docs)
                .resolve_wildcard_import(
                    "helper",
                    &["::app::helper".to_string(), "::helper".to_string()],
                    call_from("file:///p/caller.tcl"),
                )
                .as_deref(),
            Some("::second::helper"),
        );
    }

    #[test]
    fn resolve_wildcard_import_ignores_a_same_file_export_written_after_the_import() {
        // FP guard (CRITICAL), direction B, cross-document — the import and
        // the export are both in `app.tcl` and the export comes *after*, so
        // real Tcl never binds `::app::helper` (oracle: `invalid command
        // name`). `mymod.tcl` exports nothing, so there is no unordered
        // event to fall back on and the resolver must abstain.
        let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export helper\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert!(
            resolved.is_none(),
            "an export written after the import in the same file must not \
             apply retroactively: {resolved:?}"
        );
    }

    #[test]
    fn resolve_wildcard_import_keeps_answering_when_the_export_is_in_another_file() {
        // TN-for-abstention — which of two files loads first is not a static
        // fact, so a `namespace export -clear` in a *third* document cannot
        // be ordered against this import and must not silently revoke it.
        // Navigation keeps answering; the residual is documented in
        // `namespace_import`'s module docs.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
        );
        let teardown = analyse("namespace eval ::mymod { namespace export -clear }\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
            ("file:///teardown.tcl", &teardown),
        ]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
    }

    #[test]
    fn wildcard_import_inside_a_body_sees_a_later_top_level_export() {
        // TP, PR #1102 review finding 1 — a plain `at <= import_at` predicate
        // is *weaker* than the same-document tier's
        // `indirection::in_effect`, and rejected this real alias. The import
        // sits in `::app::setup`'s body; the `namespace export` is written
        // further down the *same file* but at load level, so loading `app.tcl`
        // runs the export before `setup` can ever be called.
        //
        // Oracle (tclsh 8.6.14 / 9.0.4): with `::mymod::helper` defined
        // elsewhere, `namespace eval ::app {proc setup {} {namespace import
        // ::mymod::*}; proc run {} {helper}}` followed by `namespace eval
        // ::mymod {namespace export helper}`, then `::app::setup; ::app::run`
        // → `HELP`.
        let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
        let app = analyse(
            "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::* }\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export helper\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert_eq!(
            resolved.as_deref(),
            Some("::mymod::helper"),
            "an import inside a body observes every top-level statement of \
             its own file, wherever written",
        );
    }

    #[test]
    fn wildcard_import_inside_a_body_still_refuses_an_export_in_that_same_body() {
        // FN guard for the leniency above (PR #1102 review finding 1): the
        // "whole file loads first" exception does **not** extend to a
        // statement of the *same* body — there the offsets are in genuine
        // execution order. Oracle: `proc setup {} {namespace import ::m::*;
        // namespace eval ::m {namespace export helper}}` then `::a::setup`
        // leaves `::a::helper` an `invalid command name`.
        let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
        let app = analyse(
            "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::* ; namespace eval ::mymod { namespace export helper } }\n    proc run {} { helper }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert!(
            resolved.is_none(),
            "an export written after the import in the same body is a later \
             statement of the running script: {resolved:?}",
        );
    }

    // ---- the import edge's own lifecycle, cross-document (issue #1103) ---
    //
    // The workspace twin of `definition.rs`'s in-document block. Same oracle
    // rows (tclsh 9.0.4 + 8.6.14, byte-identical), same shared decision
    // function (`namespace_import::alias_live_at`), so the two tiers cannot
    // drift.

    #[test]
    fn cross_file_forget_after_the_import_stops_resolving() {
        // TN — the source lives in another file, so only this tier can
        // answer; the forget and the call are in one document, so they are
        // ordered.
        let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nnamespace eval ::app {\n    namespace forget ::mymod::helper\n}\nhelper\n";
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(src);
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_at("file:///app.tcl", call),
        );
        assert!(
            resolved.is_none(),
            "a call after the forget must not resolve through the alias: {resolved:?}"
        );
    }

    #[test]
    fn cross_file_call_before_the_forget_still_resolves() {
        // TP — the same document, a call written before the forget.
        let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nhelper\nnamespace eval ::app {\n    namespace forget ::mymod::helper\n}\n";
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(src);
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_at("file:///app.tcl", call),
        );
        assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
    }

    #[test]
    fn a_forget_in_another_file_revokes_nothing() {
        // TN-for-abstention — no static load order between two files, so a
        // foreign forget is passed unordered and cannot silently drop a real
        // alias. Same direction as an unordered `namespace export -clear`
        // (#1104 item 3).
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse("namespace eval ::app {\n    namespace import ::mymod::*\n}\n");
        let teardown = analyse("namespace eval ::app {\n    namespace forget ::mymod::helper\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
            ("file:///teardown.tcl", &teardown),
        ]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
    }

    #[test]
    fn cross_file_source_deletion_kills_the_alias() {
        // TN — `rename ::mymod::helper {}` destroys the command object, and
        // the alias holds the object. Ordered because the deletion and the
        // call share a document.
        let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nrename ::mymod::helper {}\nhelper\n";
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(src);
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_at("file:///app.tcl", call),
        );
        assert!(resolved.is_none(), "{resolved:?}");
    }

    #[test]
    fn cross_file_source_rename_leaves_the_alias_alive() {
        // TP — the asymmetry again: `rename ::mymod::helper ::mymod::h2`
        // moves the origin and keeps `::app::helper` working.
        let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nrename ::mymod::helper ::mymod::h2\nhelper\n";
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(src);
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_at("file:///app.tcl", call),
        );
        assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
    }

    #[test]
    fn an_unforced_import_onto_an_existing_workspace_command_installs_nothing() {
        // TN — `::app` already defines `helper`, so the non-`-force` import
        // errors and installs nothing; the call reaches the local definition,
        // not `::mymod::helper` (oracle: `namespace origin ::app::helper` →
        // `::app::helper`).
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    proc helper {} {}\n    namespace import ::mymod::*\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///app.tcl"),
        );
        assert!(resolved.is_none(), "{resolved:?}");
    }

    #[test]
    fn a_forced_import_onto_an_existing_workspace_command_installs() {
        // TP — the same program with `-force` replaces the local command, so
        // the call reaches `::mymod::helper` (oracle: `namespace origin
        // ::app::helper` → `::mymod::helper`).
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    proc helper {} {}\n    namespace import -force ::mymod::*\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///app.tcl"),
        );
        assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
    }

    #[test]
    fn an_exact_import_onto_an_existing_workspace_command_installs_no_link() {
        // TN — the same conflict rule on the exact-import link path
        // (`live_command_links`), so definition / references / the existence
        // oracle all agree the import bound nothing.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    proc helper {} {}\n    namespace import ::mymod::helper\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::app::helper",
            "a conflicting exact import installs no link"
        );
    }

    #[test]
    fn a_forced_exact_import_still_installs_its_link() {
        // FN guard for the rule above.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    proc helper {} {}\n    namespace import -force ::mymod::helper\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::mymod::helper"
        );
    }

    #[test]
    fn a_cross_file_forget_cannot_revoke_on_unrelated_offsets() {
        // FP guard (CRITICAL), issue #1116 finding 1 — the import lives in a
        // short `imports.tcl` (small byte offset); the forget lives in a long
        // `caller.tcl` (large byte offset) and names a namespace the caller
        // never imported into itself. Nothing orders the two files, so the
        // forget must revoke nothing — but comparing the raw offsets made the
        // caller's larger number "later" and dropped a live alias.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let imports = analyse("namespace eval ::app { namespace import ::mymod::* }\n");
        // Padding so the forget's offset is numerically far past the import's.
        let pad = "# ".to_string() + &"x".repeat(400) + "\n";
        let caller_src = format!(
            "{pad}namespace eval ::app {{\n    namespace forget ::mymod::helper\n}}\nhelper\n"
        );
        let caller = analyse(&caller_src);
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///imports.tcl", &imports),
            ("file:///caller.tcl", &caller),
        ]);
        let call =
            u32::try_from(app_src_offset(&caller_src, "\nhelper\n") + 1).expect("tiny source");
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_at("file:///caller.tcl", call),
        );
        assert_eq!(
            resolved.as_deref(),
            Some("::mymod::helper"),
            "a forget in another document has no order against the import and \
             must not revoke it: {resolved:?}"
        );
    }

    #[test]
    fn a_same_file_forget_after_the_import_still_revokes() {
        // TN, the other direction of finding 1 — when the import, the forget
        // and the call really do share a document the offsets mean something
        // and the ordering is unchanged.
        let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nnamespace eval ::app {\n    namespace forget ::mymod::helper\n}\nhelper\n";
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(src);
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_at("file:///app.tcl", call),
        );
        assert!(resolved.is_none(), "{resolved:?}");
    }

    // ---- call-site ordering of the install (issue #1104 item 1) ----------
    //
    // Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
    //
    //   namespace eval ::src { proc p {} {return P}; namespace export p }
    //   namespace eval ::dst { p }                  ;# invalid command name "p"
    //   namespace eval ::dst { namespace import ::src::* }
    //   namespace eval ::dst { p }                  ;# P
    //
    // …and the body-scope half, same transcript run:
    //
    //   namespace eval ::app { proc run {} { helper } }
    //   namespace eval ::app { namespace import ::src::* }
    //   ::app::run                                  ;# HELP
    //
    //   namespace eval ::app2 { proc run2 {} {
    //       catch {helper} e ; namespace import ::src::* ; return "$e [helper]" } }
    //   ::app2::run2   ;# {invalid command name "helper"} HELP

    #[test]
    fn a_top_level_call_before_its_own_import_does_not_resolve() {
        // TN. Both call sites are top level, so the offsets are in genuine
        // execution order and the earlier one reaches nothing.
        let src = "helper\nnamespace eval ::app {\n    namespace import ::mymod::*\n}\nhelper\n";
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(src);
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let candidates = ["::app::helper".to_string(), "::helper".to_string()];
        let before = u32::try_from(app_src_offset(src, "helper\n")).expect("tiny source");
        assert!(
            index
                .resolve_wildcard_import("helper", &candidates, call_at("file:///app.tcl", before))
                .is_none(),
            "a call written before its own import must not resolve through it",
        );
        // TP — the same call after the import still does.
        let after = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
        assert_eq!(
            index
                .resolve_wildcard_import("helper", &candidates, call_at("file:///app.tcl", after))
                .as_deref(),
            Some("::mymod::helper"),
        );
    }

    #[test]
    fn a_body_local_call_resolves_through_an_import_written_later_in_its_file() {
        // TP (CRITICAL) — the reason the install gate is `in_effect_within`
        // and not `at < call`: the whole file loads, imports included, before
        // any body runs, so this shape (procs first, `namespace import` at the
        // bottom) still resolves. `tcllib`'s `modules/uev/uevent.tcl` is
        // exactly it; a plain-offset gate broke all five of its call sites.
        let src = "namespace eval ::app {\n    proc run {} { helper }\n}\nnamespace eval ::app {\n    namespace import ::mymod::*\n}\n";
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(src);
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let call = u32::try_from(app_src_offset(src, "helper }")).expect("tiny source");
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            CallSite {
                uri: "file:///app.tcl",
                at: call,
                enclosing_body: app.innermost_definition_body_span(call),
            },
        );
        assert_eq!(
            resolved.as_deref(),
            Some("::mymod::helper"),
            "a body-local call observes its own file's later top-level import",
        );
    }

    #[test]
    fn a_body_local_call_before_an_import_in_that_same_body_does_not_resolve() {
        // TN — the FN guard for the leniency above: an import written in the
        // *same* body after the call is an ordinary later statement of the
        // running script, exactly as the export snapshot already treats one.
        let src =
            "namespace eval ::app {\n    proc run {} { helper ; namespace import ::mymod::* }\n}\n";
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(src);
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let call = u32::try_from(app_src_offset(src, "helper ;")).expect("tiny source");
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            CallSite {
                uri: "file:///app.tcl",
                at: call,
                enclosing_body: app.innermost_definition_body_span(call),
            },
        );
        assert!(resolved.is_none(), "{resolved:?}");
    }

    #[test]
    fn a_call_before_an_import_in_another_file_still_resolves() {
        // TP — cross-file installs have no static load order, so the gate
        // cannot fire on a foreign offset (the same abstention every other
        // cross-file event takes here). `at` is deliberately tiny.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let imports = analyse("namespace eval ::app { namespace import ::mymod::* }\n");
        let caller = analyse("helper\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///imports.tcl", &imports),
            ("file:///caller.tcl", &caller),
        ]);
        assert_eq!(
            index
                .resolve_wildcard_import(
                    "helper",
                    &["::app::helper".to_string(), "::helper".to_string()],
                    call_at("file:///caller.tcl", 0),
                )
                .as_deref(),
            Some("::mymod::helper"),
        );
    }

    #[test]
    fn the_body_span_column_matches_the_per_offset_lookup() {
        // `enclosing_body_spans` is a stack sweep standing in for one
        // `innermost_definition_body_span` per row (issue #1116 item 3's
        // O(procs × invocations)); nesting, shared starts and calls between
        // sibling bodies are where the two could part company.
        let a = analyse(
            "proc outer {} {\n  proc inner {} { set x 1 }\n  set y 2\n}\nset z 3\noo::class create C { method m {} { set w 4 } }\nset q 5\n",
        );
        let offsets: Vec<u32> = a
            .command_invocations
            .iter()
            .map(|i| i.range.start())
            .collect();
        assert!(offsets.len() > 5, "the fixture must exercise the sweep");
        let swept = WorkspaceIndex::enclosing_body_spans(&a, &offsets);
        let naive: Vec<Option<Span>> = offsets
            .iter()
            .map(|&o| a.innermost_definition_body_span(o))
            .collect();
        assert_eq!(swept, naive);
        // …and the column really is populated on the rows the resolver reads.
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        assert!(
            index.invocations().any(|i| i.enclosing_body.is_some()),
            "body-local invocations must carry their span",
        );
    }

    // ---- global-rooted imports go through the gate too (#1104 review note) -
    //
    // Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
    //
    //   proc p {} { return GLOBAL }
    //   namespace eval ::dst  { namespace import ::p }   ;# no error…
    //   info commands ::dst::*                           ;# …and nothing bound
    //   namespace export p
    //   namespace eval ::dst2 { namespace import ::p }
    //   info commands ::dst2::*                          ;# ::dst2::p

    #[test]
    fn a_global_rooted_exact_import_is_export_gated() {
        // TN — `::p` splits to an *empty* source namespace, which both tiers
        // read as "no source" and skipped, leaving the one import shape that
        // bypassed the gate entirely.
        let lib = analyse("proc p {} { return GLOBAL }\n");
        let dst = analyse("namespace eval ::dst { namespace import ::p }\n");
        let index =
            WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
        assert_eq!(
            index.resolve_command_target("::dst::p"),
            "::dst::p",
            "an unexported global command installs nothing",
        );
    }

    #[test]
    fn a_global_rooted_exact_import_of_an_exported_name_still_binds() {
        // TP — the export at global level is recorded with `ns` = `::`, the
        // same spelling the gate now asks with, so the working case works.
        let lib = analyse("proc p {} { return GLOBAL }\nnamespace export p\n");
        let dst = analyse("namespace eval ::dst { namespace import ::p }\n");
        let index =
            WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
        assert_eq!(index.resolve_command_target("::dst::p"), "::p");
    }

    #[test]
    fn a_global_rooted_glob_import_is_export_gated() {
        // TN — the glob spelling of the same shape, which the index used to
        // drop on the floor rather than record.
        let lib = analyse("proc p {} { return GLOBAL }\n");
        let dst = analyse("namespace eval ::dst { namespace import ::* }\np\n");
        let index =
            WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
        let candidates = ["::dst::p".to_string(), "::p".to_string()];
        assert!(
            index
                .resolve_wildcard_import("p", &candidates, call_from("file:///dst.tcl"))
                .is_none(),
            "an unexported global command is not reachable through `::*`",
        );
        // TP — with the export, it is.
        let lib = analyse("proc p {} { return GLOBAL }\nnamespace export p\n");
        let index =
            WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
        assert_eq!(
            index
                .resolve_wildcard_import("p", &candidates, call_from("file:///dst.tcl"))
                .as_deref(),
            Some("::p"),
        );
    }

    // ---- glob / exact import conflicts are symmetric (issue #1116 item 7) --
    //
    // Oracle, byte-identical on tclsh 8.6.14 and 9.0.4, both orders:
    //
    //   namespace eval ::A { proc p {} {return A}; namespace export p }
    //   namespace eval ::B { proc p {} {return B}; namespace export p }
    //   namespace eval ::dst  { namespace import ::A::* ; namespace import ::B::p }
    //   namespace eval ::dst2 { namespace import ::A::p ; namespace import ::B::* }
    //   → both second imports: can't import command "p": already exists
    //   → namespace origin ::dst::p  = ::A::p ; ::dst::p  → A
    //   → namespace origin ::dst2::p = ::A::p ; ::dst2::p → A

    /// The `::A` / `::B` sources both exporting `p`, used by the conflict
    /// tests below.
    fn two_exporting_sources() -> (AnalysisResult, AnalysisResult) {
        (
            analyse("namespace eval ::A { proc p {} { return A }\n namespace export p }\n"),
            analyse("namespace eval ::B { proc p {} { return B }\n namespace export p }\n"),
        )
    }

    #[test]
    fn a_glob_import_conflicts_with_a_later_exact_import_of_the_same_name() {
        // TN (the gap) — the exact link's conflict check only ever compared
        // other *exact* links, so the earlier glob import was invisible to it
        // and `::dst::p` resolved to `::B::p`, the one binding Tcl refuses.
        let (a, b) = two_exporting_sources();
        let dst = analyse(
            "namespace eval ::dst {\n    namespace import ::A::*\n    namespace import ::B::p\n}\np\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///dst.tcl", &dst),
        ]);
        assert_ne!(
            index.resolve_command_target("::dst::p"),
            "::B::p",
            "the exact import installs nothing over the live glob alias",
        );
        // …and the name the call really reaches is the glob import's source.
        assert_eq!(
            index
                .resolve_wildcard_import(
                    "p",
                    &["::dst::p".to_string(), "::p".to_string()],
                    call_from("file:///dst.tcl"),
                )
                .as_deref(),
            Some("::A::p"),
        );
    }

    #[test]
    fn an_exact_import_conflicts_with_a_later_glob_import_of_the_same_name() {
        // TN, the other direction of the same rule — the glob side asked only
        // about other glob imports.
        let (a, b) = two_exporting_sources();
        let dst = analyse(
            "namespace eval ::dst {\n    namespace import ::A::p\n    namespace import ::B::*\n}\np\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///dst.tcl", &dst),
        ]);
        let resolved = index.resolve_wildcard_import(
            "p",
            &["::dst::p".to_string(), "::p".to_string()],
            call_from("file:///dst.tcl"),
        );
        assert!(
            resolved.is_none_or(|r| r == "::A::p"),
            "the later glob import must not install over a live exact alias",
        );
        assert_eq!(index.resolve_command_target("::dst::p"), "::A::p");
    }

    #[test]
    fn a_forced_exact_import_still_replaces_a_live_glob_alias() {
        // TP / FN guard — `-force` is exactly the case that *does* install
        // over whatever was there (oracle on `WorkspaceGlobImport::forced`),
        // so widening the conflict check must not swallow it.
        let (a, b) = two_exporting_sources();
        let dst = analyse(
            "namespace eval ::dst {\n    namespace import ::A::*\n    namespace import -force ::B::p\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///dst.tcl", &dst),
        ]);
        assert_eq!(index.resolve_command_target("::dst::p"), "::B::p");
    }

    #[test]
    fn a_body_local_import_conflicts_with_a_later_top_level_import() {
        // TN (CRITICAL) — the conflict check must order the two imports the
        // way they *run*, not the way they are written. Oracle, byte-identical
        // on tclsh 8.6.14 and 9.0.4:
        //
        //   namespace eval ::A { proc x {} {return A}; namespace export x }
        //   namespace eval ::B { proc x {} {return B}; namespace export x }
        //   namespace eval ::dst { proc p {} { namespace import ::B::x } }
        //   namespace eval ::dst { namespace import ::A::* }
        //   namespace origin ::dst::x  -> ::A::x   (after load)
        //   ::dst::p                   -> can't import command "x": already exists
        //   namespace origin ::dst::x  -> ::A::x   (unchanged)
        //
        // The whole file loads before any body runs, so the top-level `::A`
        // glob import owns the name by the time `p`'s body-local `::B` import
        // executes — and that one installs nothing. A plain offset compare
        // sees `::A` written *later* and lets `::B` install.
        let (a, b) = two_exporting_sources();
        let dst = analyse(
            "namespace eval ::dst {\n    proc runner {} { namespace import ::B::p }\n}\nnamespace eval ::dst {\n    namespace import ::A::*\n}\np\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///dst.tcl", &dst),
        ]);
        assert_ne!(
            index.resolve_command_target("::dst::p"),
            "::B::p",
            "the body-local import runs after the top-level one and installs nothing",
        );
        assert_eq!(
            index
                .resolve_wildcard_import(
                    "p",
                    &["::dst::p".to_string(), "::p".to_string()],
                    call_from("file:///dst.tcl"),
                )
                .as_deref(),
            Some("::A::p"),
            "the top-level glob import is the edge the name really has",
        );
    }

    #[test]
    fn a_top_level_import_is_not_conflicted_by_a_later_top_level_one() {
        // TN guard — at load level the offsets *are* execution order, so the
        // first import still wins and the second is the one that installs
        // nothing. Oracle (8.6.14 / 9.0.4): after `namespace import ::A::*`
        // then `namespace import ::B::p`, the second raises `can't import
        // command "p": already exists` and `namespace origin ::dst::p` stays
        // `::A::p`. Making the check body-aware must not reorder these.
        let (a, b) = two_exporting_sources();
        let dst = analyse(
            "namespace eval ::dst {\n    namespace import ::A::*\n}\nnamespace eval ::dst {\n    namespace import ::B::p\n}\np\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///dst.tcl", &dst),
        ]);
        assert_ne!(index.resolve_command_target("::dst::p"), "::B::p");
        assert_eq!(
            index
                .resolve_wildcard_import(
                    "p",
                    &["::dst::p".to_string(), "::p".to_string()],
                    call_from("file:///dst.tcl"),
                )
                .as_deref(),
            Some("::A::p"),
        );
    }

    #[test]
    fn a_later_import_in_the_same_body_has_not_run_yet() {
        // TN guard, the other half — inside one body the offsets are genuine
        // execution order again. Oracle (8.6.14 / 9.0.4): `proc q {} {
        // namespace import ::A::* ; namespace import ::B::p }` → the first
        // succeeds, the second raises `already exists`, origin stays `::A::p`.
        // So the *first* must not be conflicted away by the second.
        let (a, b) = two_exporting_sources();
        let dst = analyse(
            "namespace eval ::dst {\n    proc q {} { namespace import ::A::* ; namespace import ::B::p }\n}\np\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///dst.tcl", &dst),
        ]);
        assert_ne!(index.resolve_command_target("::dst::p"), "::B::p");
        assert_eq!(
            index
                .resolve_wildcard_import(
                    "p",
                    &["::dst::p".to_string(), "::p".to_string()],
                    call_from("file:///dst.tcl"),
                )
                .as_deref(),
            Some("::A::p"),
        );
    }

    #[test]
    fn a_glob_import_in_another_file_does_not_conflict_with_an_exact_one() {
        // FP guard — two imports in different documents have no static load
        // order, so conflicting on a guess would drop a link Tcl installed.
        // The abstention is the same one every other cross-file event takes.
        let (a, b) = two_exporting_sources();
        let globbed = analyse("namespace eval ::dst { namespace import ::A::* }\n");
        let exact = analyse("namespace eval ::dst { namespace import ::B::p }\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///glob.tcl", &globbed),
            ("file:///exact.tcl", &exact),
        ]);
        assert_eq!(index.resolve_command_target("::dst::p"), "::B::p");
    }

    #[test]
    fn an_exact_import_link_dies_on_a_same_file_forget() {
        // TN (CRITICAL), issue #1116 finding 2 — an exact `namespace import
        // ::mymod::helper` produces a fixed link rather than a per-call glob
        // lookup, and the lifecycle events never reached it: after the forget
        // the link stayed live and cross-document definition / references
        // still resolved `::app::helper` to `::mymod::helper`.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::helper\n    namespace forget ::mymod::helper\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::app::helper",
            "a forgotten exact import installs no live link"
        );
        assert!(!index.workspace_command_exists("::app::helper"));
    }

    #[test]
    fn an_exact_import_link_survives_a_forget_written_before_it() {
        // FN guard for the row above — the forget is ordered against the
        // *import*, and one written before it is undone by the import itself
        // (a re-import after a forget reinstalls — oracle).
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace forget ::mymod::helper\n    namespace import ::mymod::helper\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::mymod::helper"
        );
    }

    #[test]
    fn a_destroyed_source_kills_a_body_local_exact_import_link() {
        // TN (CRITICAL): the destruction is not a timeline event — the command
        // object is gone workspace-wide and no load order brings it back — so
        // it must revoke however the import is written. A **body-local**
        // import gate is the case that separates the two encodings: the
        // load-order rule reads a removal written outside the import's own
        // body as having run before it, which is right for a `namespace
        // forget` (the import then undoes it) and backwards for a destruction
        // the import cannot undo. Oracle: `rename ::mymod::helper {}` makes
        // `::app::helper` an `invalid command name` and empties `info commands
        // ::app::*`.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::helper }\n}\nrename ::mymod::helper {}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::app::helper",
            "destroying the source command revokes the link wherever the import sits"
        );
        assert!(!index.workspace_command_exists("::app::helper"));
        // …and the top-level spelling of the same import, for the control.
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::helper\n}\nrename ::mymod::helper {}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::app::helper"
        );
    }

    #[test]
    fn a_body_local_exact_import_survives_a_top_level_forget() {
        // FN guard for the row above, and the reason the destruction cannot
        // simply be encoded as "a removal at `u32::MAX`": a `namespace forget`
        // at the file's load level runs *before* a body-local import, which
        // then reinstalls the alias. Oracle: `::app::setup` followed by
        // `namespace origin ::app::helper` answers `::mymod::helper`.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::helper }\n}\nnamespace eval ::app { namespace forget ::mymod::helper }\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::mymod::helper",
            "a load-level forget runs before the body-local import that reinstalls it"
        );
    }

    #[test]
    fn an_exact_import_link_ignores_a_forget_in_another_file() {
        // FP guard — finding 1's rule applied to the link tier: a forget with
        // no static order against the import revokes nothing.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse("namespace eval ::app {\n    namespace import ::mymod::helper\n}\n");
        let teardown = analyse("namespace eval ::app {\n    namespace forget ::mymod::helper\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
            ("file:///teardown.tcl", &teardown),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::mymod::helper"
        );
    }

    #[test]
    fn an_exact_import_link_dies_when_the_source_command_is_destroyed() {
        // TN — destroying the source is not order-ambiguous the way a forget
        // is: the command *object* the alias holds is gone workspace-wide
        // (oracle: `rename ::mymod::helper {}` makes `::app::helper` an
        // `invalid command name` and empties `info commands ::app::*`), so the
        // link dies even though the deletion sits in another document.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse("namespace eval ::app {\n    namespace import ::mymod::helper\n}\n");
        let teardown = analyse("rename ::mymod::helper {}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
            ("file:///teardown.tcl", &teardown),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::app::helper",
            "destroying the source destroys the link"
        );
    }

    #[test]
    fn an_exact_import_link_dies_when_the_imported_name_is_redefined() {
        // TN, issue #1116 finding 3 cross-document — a `proc ::app::helper`
        // after the import recreates the name as an ordinary command
        // (oracle: rc 0, `namespace origin` → `::app::helper`).
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::helper\n}\nproc ::app::helper {} {}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::app::helper"
        );
    }

    #[test]
    fn a_cross_file_live_alias_is_an_import_conflict_for_a_different_source() {
        // TN, issue #1116 finding 4 cross-document — `::dst` imports `::A::*`
        // and then, further down the same file, `::B::*` without `-force`.
        // Oracle: the second import raises `can't import command "p": already
        // exists` and `namespace origin ::dst::p` stays `::A::p`.
        let a = analyse("namespace eval ::A { proc p {} { return AP }\n namespace export p }\n");
        let b = analyse("namespace eval ::B { proc p {} { return BP }\n namespace export p }\n");
        let dst = analyse(
            "namespace eval ::dst {\n    namespace import ::A::*\n}\nnamespace eval ::dst {\n    namespace import ::B::*\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///dst.tcl", &dst),
        ]);
        let resolved = index.resolve_wildcard_import(
            "p",
            &["::dst::p".to_string(), "::p".to_string()],
            call_from("file:///dst.tcl"),
        );
        assert_eq!(
            resolved.as_deref(),
            Some("::A::p"),
            "the failed second import leaves the first alias in place: {resolved:?}"
        );
    }

    #[test]
    fn a_cross_file_forced_import_replaces_a_live_alias() {
        // TP, the other half — with `-force` the second import wins (oracle:
        // `namespace origin ::dst::p` → `::B::p`).
        let a = analyse("namespace eval ::A { proc p {} { return AP }\n namespace export p }\n");
        let b = analyse("namespace eval ::B { proc p {} { return BP }\n namespace export p }\n");
        let dst = analyse(
            "namespace eval ::dst {\n    namespace import ::A::*\n}\nnamespace eval ::dst {\n    namespace import -force ::B::*\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///dst.tcl", &dst),
        ]);
        let resolved = index.resolve_wildcard_import(
            "p",
            &["::dst::p".to_string(), "::p".to_string()],
            call_from("file:///dst.tcl"),
        );
        assert_eq!(resolved.as_deref(), Some("::B::p"));
    }

    #[test]
    fn a_cross_file_exact_link_conflicts_with_an_earlier_different_source() {
        // TN — finding 4 on the exact-link tier.
        let a = analyse("namespace eval ::A { proc p {} {}\n namespace export p }\n");
        let b = analyse("namespace eval ::B { proc p {} {}\n namespace export p }\n");
        let dst = analyse(
            "namespace eval ::dst {\n    namespace import ::A::p\n    namespace import ::B::p\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///b.tcl", &b),
            ("file:///dst.tcl", &dst),
        ]);
        assert_eq!(
            index.resolve_command_target("::dst::p"),
            "::A::p",
            "the second exact import fails and the first alias stays"
        );
    }

    #[test]
    fn a_cross_file_import_chain_follows_to_the_original_source() {
        // TP — `::A` imports `::B::*`, `::B` imported `::C::*`, each in its
        // own file. Oracle: `::A::p` runs `::C`'s body and `namespace origin
        // ::A::p` → `::C::p`. The middle hop is in no workspace proc table,
        // so a single-hop walk abstained.
        let c = analyse("namespace eval ::C { proc p {} { return CP }\n namespace export p }\n");
        let b = analyse("namespace eval ::B { namespace import ::C::*\n namespace export p }\n");
        let a = analyse("namespace eval ::A { namespace import ::B::* }\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///c.tcl", &c),
            ("file:///b.tcl", &b),
            ("file:///a.tcl", &a),
        ]);
        let resolved = index.resolve_wildcard_import(
            "p",
            &["::A::p".to_string(), "::p".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert_eq!(resolved.as_deref(), Some("::C::p"), "{resolved:?}");
    }

    #[test]
    fn a_cross_file_chain_hop_that_was_never_re_exported_abstains() {
        // FN guard — the middle hop keeps its own export gate.
        let c = analyse("namespace eval ::C { proc p {} { return CP }\n namespace export p }\n");
        let b = analyse("namespace eval ::B { namespace import ::C::* }\n");
        let a = analyse("namespace eval ::A { namespace import ::B::* }\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///c.tcl", &c),
            ("file:///b.tcl", &b),
            ("file:///a.tcl", &a),
        ]);
        let resolved = index.resolve_wildcard_import(
            "p",
            &["::A::p".to_string(), "::p".to_string()],
            call_from("file:///caller.tcl"),
        );
        assert!(resolved.is_none(), "{resolved:?}");
    }

    #[test]
    fn a_mutually_importing_pair_terminates_cross_file() {
        // FP/hang guard — bounded by `MAX_COMMAND_NAME_HOPS`.
        let a = analyse("namespace eval ::A { namespace import ::B::*\n namespace export * }\n");
        let b = analyse("namespace eval ::B { namespace import ::A::*\n namespace export * }\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(
            index
                .resolve_wildcard_import(
                    "p",
                    &["::A::p".to_string()],
                    call_from("file:///caller.tcl"),
                )
                .is_none()
        );
    }

    // Exact (non-glob) `namespace import` is export-gated too — PR #1102
    // review finding 2. Real Tcl silently installs nothing when the name is
    // not exported at the import's own position (oracle: `namespace eval
    // ::m {proc helper {} {}}; namespace eval ::a {namespace import
    // ::m::helper}` leaves `info commands ::a::*` empty and raises no error).

    #[test]
    fn exact_import_of_an_unexported_name_installs_no_link() {
        // FP guard (CRITICAL) — `::mymod` never exports `helper`, so the
        // exact import binds nothing: no `::app::helper` exists, the bare
        // call is not a reference to the source, and the pattern token is not
        // a link span. Before the gate, `index_command_links` created the
        // link unconditionally.
        let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert!(
            !index.workspace_command_exists("::app::helper"),
            "an unexported exact import installs no command",
        );
        assert!(
            index
                .linked_invocations_of("::mymod::helper", "file:///mymod.tcl")
                .is_empty(),
            "the bare call reaches nothing, so it is no reference",
        );
        assert!(
            index
                .link_target_spans("::mymod::helper", "file:///mymod.tcl")
                .is_empty(),
            "a link that was never installed has no target span",
        );
    }

    #[test]
    fn exact_import_ignores_an_export_written_after_it() {
        // FP guard, direction B for the exact form — the export lands after
        // the import in the same file, so real Tcl still binds nothing
        // (oracle: `info commands ::a5::*` empty).
        let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export helper\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert!(!index.workspace_command_exists("::app::helper"));
    }

    #[test]
    fn exact_import_survives_a_later_export_clear() {
        // TP, direction A for the exact form — the `-clear` runs after the
        // import, so the alias it already installed stays.
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export -clear\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert!(index.workspace_command_exists("::app::helper"));
        let refs = index.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
        assert_eq!(refs.len(), 1, "{refs:?}");
    }

    #[test]
    fn exact_import_from_an_unseen_namespace_keeps_its_link() {
        // Abstention guard (CRITICAL for W123 silence) — `::msgcat` lives in
        // an installed package, not in any indexed document, so the index
        // holds no export declaration for it and never will. Treating that
        // silence as "not exported" would revoke `::app::mc` and hand the
        // unknown-command pass a false positive on every bare `mc` call. The
        // gate only fires where the workspace can see the namespace's own
        // definitions or exports.
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::msgcat::mc\n    proc run {} { mc hello }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([("file:///app.tcl", &app)]);
        assert!(
            index.workspace_command_exists("::app::mc"),
            "an import from a namespace the workspace cannot observe must not \
             be gated away",
        );
    }

    #[test]
    fn alias_and_rename_links_are_never_export_gated() {
        // TN — only a `namespace import` link carries an export snapshot.
        // `interp alias` and `rename` introduce a name with no reference to
        // any export list, and must keep working unchanged.
        let lib = analyse("proc ::mymod::helper {} {}\n");
        let app = analyse(
            "interp alias {} ::app::a {} ::mymod::helper\nrename ::mymod::helper ::mymod::gone\n",
        );
        let index =
            WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
        assert!(
            index.workspace_command_exists("::app::a"),
            "an interp alias is not gated by any namespace export",
        );
        assert!(
            index.workspace_command_exists("::mymod::gone"),
            "a rename is not gated by any namespace export",
        );
    }

    #[test]
    fn wildcard_import_call_site_is_a_reference_to_the_source_command() {
        // TP — `linked_invocations_of` (find-references' cross-document
        // mechanism) must reach the bare call site through a wildcard
        // import exactly like it already does for an exact import
        // (`namespace_import_call_site_references_the_source_command`).
        let mymod =
            analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let refs = index.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
        assert_eq!(refs.len(), 1, "{refs:?}");
        assert_eq!(refs[0].uri, "file:///app.tcl");
        assert_eq!(refs[0].name, "helper");
    }

    #[test]
    fn wildcard_import_unexported_sibling_call_site_is_not_a_reference() {
        // FP guard — the call site for an unexported sibling must not
        // surface as a reference to it either.
        let mymod = analyse(
            "namespace eval ::mymod {\n    proc helper {} {}\n    proc other {} {}\n    namespace export helper\n}\n",
        );
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { other }\n}\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        let refs = index.linked_invocations_of("::mymod::other", "file:///mymod.tcl");
        assert!(refs.is_empty(), "{refs:?}");
    }

    #[test]
    fn oo_forward_target_is_a_reference_to_the_command() {
        // A `forward` method delegates to `::logger::write`; that `TARGET` word
        // is a reference to the command, so finding references (and rename) of
        // `::logger::write` must include it — like a direct call.
        let logger = analyse("namespace eval ::logger { proc write {} {} }\n");
        let widget = analyse("oo::class create ::Widget {\n    forward log ::logger::write\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///logger.tcl", &logger),
            ("file:///widget.tcl", &widget),
        ]);
        let refs = index.invocations_of("::logger::write", "file:///logger.tcl");
        assert_eq!(refs.len(), 1, "{refs:?}");
        assert_eq!(refs[0].uri, "file:///widget.tcl");
    }

    #[test]
    fn cross_file_oo_define_stub_does_not_hide_superclass() {
        // `::B` defines `greet`; `::C` (superclass `::B`) inherits it; a
        // cross-file `oo::define ::C` adds `extra` and names no superclass,
        // recording a second `::C` entry with empty parents.  The parent walk
        // must union both entries — otherwise the stub hides the `::B` edge and
        // `::C` is wrongly dropped from `greet`'s inheritor set.
        let b = analyse("oo::class create B {\n    method greet {} {}\n}\n");
        let c = analyse("oo::class create C {\n    superclass B\n}\n");
        let stub = analyse("oo::define C {\n    method extra {} {}\n}\n");
        // The stub is indexed *before* the real class, so a first-match parent
        // lookup would pick the stub's empty superclasses — the adversarial
        // ordering the union guards against.
        let index = WorkspaceIndex::from_documents([
            ("file:///b.tcl", &b),
            ("file:///ext.tcl", &stub),
            ("file:///c.tcl", &c),
        ]);
        let inheritors: Vec<&str> = index
            .method_inheritor_classes("::B", "greet")
            .iter()
            .map(|wc| wc.qualified_name.as_str())
            .collect();
        assert!(inheritors.contains(&"::C"), "{inheritors:?}");
    }

    #[test]
    fn workspace_command_exists_covers_procs_and_classes() {
        let a = analyse("namespace eval ns { proc p {} {} }\noo::class create ::C {}\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        assert!(index.workspace_command_exists("::ns::p"));
        assert!(index.workspace_command_exists("ns::p")); // leading `::` optional
        assert!(index.workspace_command_exists("::C"));
        assert!(!index.workspace_command_exists("::ns::missing"));
    }

    #[test]
    fn nested_proc_flagged_and_workspace_command_exists_for_call_excludes_it_from_a_builtin() {
        // TP — a `proc ::set {...}` written inside another proc's body (the
        // "rename the builtin away, install a shadow, restore it" idiom)
        // must be recorded as `nested`, and must not count as "::set exists
        // in the workspace" once a builtin is in play — the cross-file twin
        // of `resolve_called_proc`'s same-file gate.
        let a = analyse("proc outer {} {\n    proc ::set {v val} {}\n}\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        let set_proc = index
            .procs()
            .find(|p| p.qualified_name == "::set")
            .expect("nested ::set proc indexed");
        assert!(
            set_proc.nested,
            "the shadow proc must be recorded as nested"
        );
        assert!(
            index.workspace_command_exists("::set"),
            "the unconditional existence check (no builtin gate) still finds it",
        );
        assert!(
            !index.workspace_command_exists_for_call("::set", true),
            "a nested shadow of a real builtin must not count as existing \
             for call-target resolution",
        );
        assert!(
            index.workspace_command_exists_for_call("::set", false),
            "with no colliding builtin, the nested definition still counts \
             (e.g. `namespace which -command` probes, W120 existence checks)",
        );
    }

    #[test]
    fn namespace_declarations_and_refs_are_indexed_across_documents() {
        // TP (issue #1088) — the declaring `namespace eval` blocks live in
        // one document and the `namespace children ::mypkg` consumer in
        // another; both spellings name the one namespace.  Oracle (tclsh
        // 9.0.4 / 8.6.16, byte-identical): reopening `::mypkg` extends the
        // same namespace, and `namespace children ::mypkg` lists its
        // children rather than erroring.
        let decl = analyse("namespace eval mypkg {}\nnamespace eval mypkg {}\n");
        let user = analyse("set t [namespace children ::mypkg]\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///decl.tcl", &decl),
            ("file:///user.tcl", &user),
        ]);
        assert_eq!(
            index
                .namespace_declarations_qualified("::mypkg", "")
                .iter()
                .map(|n| n.uri.as_str())
                .collect::<Vec<_>>(),
            vec!["file:///decl.tcl", "file:///decl.tcl"],
            "both declaring blocks are definition sites",
        );
        assert_eq!(
            index
                .namespace_refs_of("::mypkg", "")
                .iter()
                .map(|n| n.uri.as_str())
                .collect::<Vec<_>>(),
            vec!["file:///user.tcl"],
        );
        // The declaring document is excluded on request, which is how the
        // cross-document definition tier avoids re-reporting local answers.
        assert!(
            index
                .namespace_declarations_qualified("::mypkg", "file:///decl.tcl")
                .is_empty(),
        );
    }

    /// Issue #1246 — declaring rows whose name is a **strict descendant** of
    /// the cell: the workspace half of the implicit-parent answer.
    ///
    /// tclsh-proof (9.0.4 / 8.6.16, byte-identical): `namespace eval
    /// ::p::q::r {}` leaves `namespace exists ::p::q` -> 1 and `namespace
    /// exists ::p` -> 1, while `namespace eval ::pq::r {}` leaves `namespace
    /// exists ::p` -> 0.
    #[test]
    fn namespace_declarations_under_finds_implicit_parent_rows() {
        let decl = analyse("namespace eval ::p::q::r {}\n");
        let other = analyse("namespace eval ::pq::r {}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///decl.tcl", &decl),
            ("file:///other.tcl", &other),
        ]);
        // TP — both ancestors are answered by the one deeper block.
        assert_eq!(
            index
                .namespace_declarations_under("::p::q", "")
                .iter()
                .map(|n| n.qualified_name.as_str())
                .collect::<Vec<_>>(),
            vec!["::p::q::r"],
        );
        assert_eq!(index.namespace_declarations_under("::p", "").len(), 1);
        // TN — an exact match is a real declaration, not an implicit one.
        assert!(
            index
                .namespace_declarations_under("::p::q::r", "")
                .is_empty(),
        );
        // TN — a segment prefix is a different namespace entirely.
        assert!(
            index
                .namespace_declarations_under("::p", "file:///decl.tcl")
                .is_empty(),
            "excluding the declaring document leaves only `::pq::r`, which is not under `::p`",
        );
        // TN — the global namespace is created by the interpreter, so nothing
        // implicitly creates it.
        assert!(index.namespace_declarations_under("::", "").is_empty());
        assert!(index.namespace_declarations_under("", "").is_empty());
    }

    #[test]
    fn namespace_rows_are_dropped_with_their_document() {
        // A re-index (`remove_document` then `add_document`) must not leave
        // stale namespace rows behind — the same discipline every other
        // table follows.
        let a = analyse("namespace eval gone {}\n");
        let mut index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        assert_eq!(
            index.namespace_declarations_qualified("::gone", "").len(),
            1
        );
        index.remove_document("file:///a.tcl");
        assert!(
            index
                .namespace_declarations_qualified("::gone", "")
                .is_empty()
        );
        assert!(index.namespace_refs().next().is_none());
    }

    #[test]
    fn a_relative_namespace_word_is_indexed_rooted() {
        // TP — the analyser roots a relative spelling against its own
        // namespace before the index sees it, so a sibling document can
        // match it by exact qualified name.  Oracle: inside `namespace eval
        // ::outer`, `namespace children inner` means `::outer::inner`; the
        // same words at global scope mean `::inner` (both interpreters).
        let a = analyse(
            "namespace eval ::outer {\n    namespace eval inner {}\n    namespace children inner\n}\n",
        );
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        assert_eq!(
            index
                .namespace_declarations_qualified("::outer::inner", "")
                .len(),
            1,
        );
        assert_eq!(index.namespace_refs_of("::outer::inner", "").len(), 1);
        assert!(index.namespace_refs_of("::inner", "").is_empty());
    }

    #[test]
    fn top_level_proc_workspace_command_exists_for_call_regardless_of_builtin() {
        // TN / regression guard — an *unnested* (top-level) proc named after
        // a builtin unconditionally overrides it for the rest of the file,
        // exactly like real Tcl's `proc puts {args} {...}`; it must keep
        // counting as existing even when a builtin of the same name is
        // known, unlike the nested case above.
        let a = analyse("proc ::puts {args} {}\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        let puts_proc = index
            .procs()
            .find(|p| p.qualified_name == "::puts")
            .expect("top-level ::puts proc indexed");
        assert!(!puts_proc.nested, "a top-level proc is never nested");
        assert!(index.workspace_command_exists_for_call("::puts", true));
    }

    #[test]
    fn top_level_alias_workspace_command_exists_for_call_regardless_of_builtin() {
        // TP — regression for a bug found by Codex review of PR #963:
        // `workspace_command_exists_for_call`'s `has_builtin` branch dropped
        // *every* `command_links` entry (aliases / renames / imports), not
        // just conditional ones, so a permanent top-level `interp alias {}
        // set {} ::my_set` — exactly like a top-level `proc set` — wrongly
        // stopped counting as "::set exists" the moment a same-named builtin
        // was in play.
        let a = analyse("proc my_set {args} {}\ninterp alias {} set {} ::my_set\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        let link = index
            .command_links()
            .find(|l| l.linked_qname.trim_start_matches("::") == "set")
            .expect("top-level alias link indexed");
        assert!(!link.nested, "a top-level alias is never nested");
        assert!(
            index.workspace_command_exists_for_call("::set", true),
            "an unconditional alias of a builtin name must still count as existing",
        );
    }

    #[test]
    fn nested_alias_workspace_command_exists_for_call_excludes_it_from_a_builtin() {
        // TP — the link-kind twin of
        // `nested_proc_flagged_and_workspace_command_exists_for_call_excludes_it_from_a_builtin`:
        // an `interp alias` written inside a proc body only takes effect
        // while that proc is running, so it must not permanently count as
        // "::set exists" once a same-named builtin is in play.
        let a = analyse(
            "proc my_set {args} {}\nproc withShadow {} {\n    interp alias {} set {} ::my_set\n}\n",
        );
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        let link = index
            .command_links()
            .find(|l| l.linked_qname.trim_start_matches("::") == "set")
            .expect("nested alias link indexed");
        assert!(
            link.nested,
            "the alias inside withShadow must be recorded as nested"
        );
        assert!(
            !index.workspace_command_exists_for_call("::set", true),
            "a nested alias of a real builtin must not count as existing \
             for call-target resolution",
        );
        assert!(
            index.workspace_command_exists_for_call("::set", false),
            "with no colliding builtin, the nested alias still counts",
        );
    }

    /// TP (Codex review of PR #1091, finding 2) — a document whose only stake
    /// in `::ns::v` is an alias written from *outside* `::ns` is found by the
    /// alias table, and by nothing else.
    ///
    /// A global `proc p {} { namespace upvar ::ns v local; … }` declares
    /// nothing in `::ns`, holds no namespace-scoped declaration of the cell,
    /// and writes no qualified occurrence of it — so all three of the older
    /// candidate sources miss it, while the alias it holds breaks the moment
    /// the declaration moves without it (tclsh 9.0.4 / 8.6.16: `can't read
    /// "local": no such variable`).
    #[test]
    fn indexes_a_cell_alias_written_from_another_namespace() {
        let decl = analyse("namespace eval ns {\n    variable v 1\n}\n");
        let aliaser =
            analyse("proc p {} {\n    namespace upvar ::ns v local\n    return $local\n}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///decl.tcl", &decl);
        index.add_document("file:///aliaser.tcl", &aliaser);

        assert_eq!(
            index.documents_aliasing_variable("::ns::v"),
            vec![
                "file:///aliaser.tcl".to_string(),
                "file:///decl.tcl".to_string()
            ],
            "both the `variable v` declaration and the out-of-namespace \
             `namespace upvar` are aliases of the one cell",
        );
        assert!(
            !index
                .documents_in_namespace("::ns")
                .contains(&"file:///aliaser.tcl".to_string()),
            "the aliasing document declares nothing in ::ns — which is why \
             the alias table is needed",
        );
        assert!(
            index
                .variable_refs_of("::ns::v", "")
                .iter()
                .all(|r| r.uri != "file:///aliaser.tcl"),
            "and it writes no qualified occurrence either",
        );
    }

    /// TN (same finding) — the alias table is keyed on the cell, so an alias
    /// of a *different* cell never drags its document into another cell's
    /// rename.
    #[test]
    fn cell_alias_index_does_not_match_a_different_cell() {
        let other = analyse("proc p {} {\n    namespace upvar ::ns other local\n}\n");
        let index = WorkspaceIndex::from_documents([("file:///other.tcl", &other)]);
        assert!(index.documents_aliasing_variable("::ns::v").is_empty());
        assert_eq!(
            index.documents_aliasing_variable("::ns::other"),
            vec!["file:///other.tcl".to_string()]
        );
    }

    /// TP (issue #923 idx 65 / 75 / 78) — a namespace variable declared in one
    /// document and read, qualified, from another is matched across the two by
    /// its one `::`-rooted cell name.
    #[test]
    fn indexes_namespace_variables_and_their_qualified_occurrences() {
        let decl = analyse("namespace eval app::colors {\n    variable palette red\n}\n");
        let user = analyse("puts $app::colors::palette\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///decl.tcl", &decl);
        index.add_document("file:///user.tcl", &user);

        let defs = index.variable_definitions_qualified("::app::colors::palette", "");
        assert_eq!(defs.len(), 1, "one declaration: {defs:?}");
        assert_eq!(defs[0].uri, "file:///decl.tcl");
        assert_eq!(defs[0].name, "palette");

        let refs = index.variable_refs_of("::app::colors::palette", "file:///decl.tcl");
        assert_eq!(
            refs.iter().map(|r| r.uri.as_str()).collect::<Vec<_>>(),
            vec!["file:///user.tcl"],
            "the sibling document's qualified read is a reference: {refs:?}",
        );

        // TN: an unrelated cell name matches nothing.
        assert!(
            index
                .variable_definitions_qualified("::app::colors::other", "")
                .is_empty(),
        );
        index.remove_document("file:///decl.tcl");
        assert!(
            index
                .variable_definitions_qualified("::app::colors::palette", "")
                .is_empty(),
            "removing the declaring document drops its variables",
        );
    }

    /// TN — a proc **local** never enters the variable table: it has no
    /// qualified name a sibling document could spell.
    #[test]
    fn proc_locals_are_not_indexed_as_namespace_variables() {
        let a = analyse("proc ns::f {} {\n    set localOnly 1\n}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        assert!(
            !index.variables().any(|v| v.name == "localOnly"),
            "indexed variables: {:?}",
            index.variables().collect::<Vec<_>>(),
        );
    }

    /// The generation counter must advance on every mutation, so a consumer
    /// can tell the index changed without diffing it.
    #[test]
    fn generation_advances_on_every_mutation() {
        let a = analyse("proc helper {} {}\n");
        let mut index = WorkspaceIndex::new();
        let start = index.generation();
        index.add_document("file:///a.tcl", &a);
        let after_add = index.generation();
        assert_ne!(start, after_add, "add_document must bump the generation");
        index.remove_document("file:///a.tcl");
        assert_ne!(
            after_add,
            index.generation(),
            "remove_document must bump the generation",
        );
    }

    /// The derived command-name set is cached between mutations and rebuilt
    /// after one — the bound that keeps the cross-file unknown-command check
    /// off the per-request cost curve.
    #[test]
    fn command_names_are_cached_until_the_index_changes() {
        let a = analyse("proc ::alpha {} {}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        let first = index.command_names();
        assert!(first.contains("alpha"), "{first:?}");
        assert!(
            Arc::ptr_eq(&first, &index.command_names()),
            "an unchanged index must serve the cache, not rebuild it",
        );

        let b = analyse("oo::class create ::Beta {}\n");
        index.add_document("file:///b.tcl", &b);
        let third = index.command_names();
        assert!(
            !Arc::ptr_eq(&first, &third),
            "indexing a document must drop the cache",
        );
        assert!(third.contains("Beta"), "{third:?}");

        // A clone starts with a fresh (empty) cache and rebuilds identically.
        let cloned = index.clone();
        assert_eq!(*cloned.command_names(), *third);
    }

    /// Issue #1152: `defined_command_names` used to rebuild its `HashSet`
    /// from scratch on every call. It is now cached — one slot per
    /// `include_links` value — and dropped only by a mutation, same
    /// `Arc::ptr_eq` proof as [`command_names_are_cached_until_the_index_changes`].
    #[test]
    fn defined_command_names_are_cached_per_generation() {
        let a = analyse("proc ::alpha {} {}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        let first = index.defined_command_names(false);
        assert!(first.contains("alpha"), "{first:?}");
        assert!(
            Arc::ptr_eq(&first, &index.defined_command_names(false)),
            "an unchanged index must serve the cache, not rebuild it",
        );
        // The `include_links` variants are cached independently: reading one
        // must not populate (or invalidate) the other's slot.
        let with_links_first = index.defined_command_names(true);
        assert!(
            !Arc::ptr_eq(&first, &with_links_first),
            "the two `include_links` slots are distinct caches",
        );
        assert!(
            Arc::ptr_eq(&with_links_first, &index.defined_command_names(true)),
            "the with-links slot must also serve from cache once built",
        );

        let b = analyse("proc ::beta {} {}\n");
        index.add_document("file:///b.tcl", &b);
        let after_mutation = index.defined_command_names(false);
        assert!(
            !Arc::ptr_eq(&first, &after_mutation),
            "indexing a document must drop both cached slots",
        );
        assert!(after_mutation.contains("beta"), "{after_mutation:?}");
    }

    /// Issue #1152: `command_link_map` used to rebuild its `HashMap` (from
    /// `live_command_links`, itself already cached) on every call. It is now
    /// cached directly, same discipline as `command_names`.
    #[test]
    fn command_link_map_is_cached_per_generation() {
        let a = analyse("proc ::real {} {}\ninterp alias {} ::aliased {} ::real\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        let first = index.command_link_map();
        assert_eq!(first.get("aliased").map(String::as_str), Some("real"));
        assert!(
            Arc::ptr_eq(&first, &index.command_link_map()),
            "an unchanged index must serve the cache, not rebuild it",
        );
        index.remove_document("file:///a.tcl");
        assert!(
            !Arc::ptr_eq(&first, &index.command_link_map()),
            "a mutation must drop the cache",
        );
    }

    /// Issue #1152: `invocations_of` (via `invocations_by_settled_target`)
    /// used to re-settle every invocation in the workspace, and rebuild
    /// `WildcardImportIndex` from scratch, on every call — the cost
    /// `code_lenses` multiplied by one call per proc *and* per class in the
    /// document. The settled-target grouping is now cached per generation;
    /// repeated `invocations_of` calls against an unchanged index must
    /// share it rather than re-settle.
    #[test]
    fn invocations_by_settled_target_is_cached_per_generation() {
        let a = analyse("proc helper {} {}\nproc other {} {}\n");
        let b = analyse("helper\nhelper\nother\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        index.add_document("file:///b.tcl", &b);

        let first = index.invocations_by_settled_target(false);
        assert_eq!(first.get("helper").map(Vec::len), Some(2), "{first:?}");
        assert!(
            Arc::ptr_eq(&first, &index.invocations_by_settled_target(false)),
            "an unchanged index must serve the cache, not re-settle",
        );
        // Querying different targets against the same generation must hit
        // the same cached map, not re-settle per target.
        let helper_calls = index.invocations_of("::helper", "");
        let other_calls = index.invocations_of("::other", "");
        assert_eq!(helper_calls.len(), 2, "{helper_calls:?}");
        assert_eq!(other_calls.len(), 1, "{other_calls:?}");

        // A mutation invalidates the cache and the next call re-settles
        // against the new state.
        let c = analyse("helper\n");
        index.add_document("file:///c.tcl", &c);
        let after_mutation = index.invocations_by_settled_target(false);
        assert!(
            !Arc::ptr_eq(&first, &after_mutation),
            "indexing a document must drop the settled-invocations cache",
        );
        assert_eq!(index.invocations_of("::helper", "").len(), 3);
    }

    #[test]
    fn defined_command_names_are_cached_per_reading_and_dropped_on_mutation() {
        // Issue #1105 — `workspace_command_exists` asks for this set once per
        // candidate inside the import-chain loop, so it has to be a derived
        // view like `command_names`, not a fresh O(procs + classes + links)
        // walk. The two `include_links` readings are separate views: folding
        // the link names into the direct one would let rename rewrite a call
        // that merely spells an imported name.
        let a = analyse(
            "namespace eval ::lib { proc alpha {} {}\n namespace export alpha }\nnamespace eval ::app { namespace import ::lib::alpha }\n",
        );
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);

        let direct = index.defined_command_names(false);
        let linked = index.defined_command_names(true);
        assert!(direct.contains("lib::alpha"));
        assert!(
            !direct.contains("app::alpha"),
            "the direct reading must not admit a link: {direct:?}",
        );
        assert!(
            linked.contains("app::alpha"),
            "the linked reading must: {linked:?}",
        );
        assert!(
            Arc::ptr_eq(&direct, &index.defined_command_names(false))
                && Arc::ptr_eq(&linked, &index.defined_command_names(true)),
            "an unchanged index must serve both caches, not rebuild them",
        );

        let b = analyse("proc ::beta {} {}\n");
        index.add_document("file:///b.tcl", &b);
        let after = index.defined_command_names(false);
        assert!(
            !Arc::ptr_eq(&direct, &after),
            "indexing a document must drop the cache",
        );
        assert!(after.contains("beta"), "{after:?}");
        // A clone starts with a fresh cache and rebuilds identically.
        assert_eq!(*index.clone().defined_command_names(false), *after);
    }

    #[test]
    fn remove_document_drops_invocations_too() {
        let a = analyse("helper\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        assert!(index.invocations().next().is_some());
        index.remove_document("file:///a.tcl");
        assert!(index.invocations().next().is_none());
    }

    #[test]
    fn indexes_and_removes_package_requires() {
        let a = analyse("package require Tk\npackage require http\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        assert_eq!(
            index.package_requires_for("file:///a.tcl"),
            vec!["Tk".to_owned(), "http".to_owned()]
        );
        index.remove_document("file:///a.tcl");
        assert!(index.package_requires().next().is_none());
    }

    #[test]
    fn source_ancestor_requires_walks_the_graph() {
        // app.tcl requires Tk and sources lib/util.tcl; util inherits Tk.
        let app = analyse("package require Tk\nsource lib/util.tcl\n");
        let util = analyse("proc u {} {}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///proj/app.tcl", &app),
            ("file:///proj/lib/util.tcl", &util),
        ]);
        // Resolver mirrors the server's: join the raw path onto the parent's
        // directory (path portion of the file URI).
        let resolve = |parent: &str, raw: &str| -> Option<String> {
            let dir = parent.rsplit_once('/').map(|(d, _)| d)?;
            Some(format!("{dir}/{raw}"))
        };
        let got = index.source_ancestor_package_requires("file:///proj/lib/util.tcl", resolve);
        assert_eq!(got, vec!["Tk".to_owned()]);
        // The entry file itself inherits nothing.
        assert!(
            index
                .source_ancestor_package_requires("file:///proj/app.tcl", resolve)
                .is_empty()
        );
    }

    #[test]
    fn source_ancestor_requires_ignores_nonliteral_sources() {
        // A computed `source $path` produces no resolvable edge.
        let app = analyse("package require Tk\nsource $dir/util.tcl\n");
        let util = analyse("proc u {} {}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///proj/app.tcl", &app),
            ("file:///proj/util.tcl", &util),
        ]);
        let resolve =
            |_p: &str, _r: &str| -> Option<String> { panic!("non-literal must not resolve") };
        assert!(
            index
                .source_ancestor_package_requires("file:///proj/util.tcl", resolve)
                .is_empty()
        );
    }

    /// Issue #1253 item 1 — `package prefer latest` is interpreter-global, and
    /// along the `source` graph "ran first" is a static fact.
    ///
    /// tclsh-proof (8.6.14), with `lib.tcl` holding `puts [package prefer]`:
    /// `app.tcl` written as `package prefer latest; source lib.tcl` prints
    /// `latest`, and as `source lib.tcl; package prefer latest` prints
    /// `stable`.
    #[test]
    fn source_ancestor_prefer_latest_crosses_documents() {
        let resolve = |parent: &str, raw: &str| -> Option<String> {
            let dir = parent.rsplit_once('/').map(|(d, _)| d)?;
            Some(format!("{dir}/{raw}"))
        };
        let lib = analyse("package require w\n");

        // TP — the raise runs before the `source`.
        let app = analyse("package prefer latest\nsource lib.tcl\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///proj/app.tcl", &app),
            ("file:///proj/lib.tcl", &lib),
        ]);
        assert!(index.source_ancestor_prefers_latest("file:///proj/lib.tcl", resolve));
        // …and the entry file itself inherits nothing (its own raise is the
        // position-sensitive single-document question).
        assert!(!index.source_ancestor_prefers_latest("file:///proj/app.tcl", resolve));

        // FP guard — the raise runs after the `source`.
        let app = analyse("source lib.tcl\npackage prefer latest\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///proj/app.tcl", &app),
            ("file:///proj/lib.tcl", &lib),
        ]);
        assert!(!index.source_ancestor_prefers_latest("file:///proj/lib.tcl", resolve));

        // FP guard — a conditional raise is never recorded at all.
        let app = analyse("if {$::c} { package prefer latest }\nsource lib.tcl\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///proj/app.tcl", &app),
            ("file:///proj/lib.tcl", &lib),
        ]);
        assert!(index.package_prefers().next().is_none());
        assert!(!index.source_ancestor_prefers_latest("file:///proj/lib.tcl", resolve));
    }

    /// The prefer rows drop with their document, like every other table.
    #[test]
    fn package_prefers_are_dropped_with_their_document() {
        let a = analyse("package prefer latest\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        assert_eq!(index.package_prefers().count(), 1);
        index.remove_document("file:///a.tcl");
        assert!(index.package_prefers().next().is_none());
    }

    // Both tiers on the two-file `namespace import -force` shadow
    // (issue #1116 item 1).
    //
    // `MAIN` below is one document, byte-for-byte identical in every test of
    // this group. What changes is the *rest of the program*, and with it the
    // correct answer — which is exactly why the single-document tier needed a
    // whole-program export oracle. Oracle transcript (tclsh 8.6.14 and 9.0.4,
    // byte-identical), running `MAIN` from a loader that does or does not also
    // source the `namespace export helper`:
    //
    //   with the export sourced first: call -> SRC    origin -> ::src::helper
    //   with nothing exporting it:     call -> LOCAL  origin -> ::app::helper

    /// The pinned two-file shape's main document.
    const MAIN: &str = "namespace eval src {\n    proc helper {} { return SRC }\n    proc other {} { return O }\n    namespace export other\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";

    /// What each tier says about the `helper` call at the end of [`MAIN`],
    /// given the rest of the program.
    ///
    /// Returns `(in-document target, cross-document target)`, where a tier's
    /// "target" is the qualified name the call reaches: the local definition
    /// when the `-force` import bound nothing, the import's source when it
    /// did, and `None` when the tier cannot say. The two are computed
    /// independently — the single-document resolver over `MAIN`'s own analysis
    /// plus the index's export oracle, and the workspace's own
    /// [`WorkspaceIndex::resolve_wildcard_import`] — so agreeing is a real
    /// result and not a tautology.
    fn both_tiers_on_main(others: &[(&str, &AnalysisResult)]) -> (Option<String>, Option<String>) {
        let main = analyse(MAIN);
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///main.tcl", &main);
        for (uri, analysis) in others {
            index.add_document(uri, analysis);
        }
        let call = u32::try_from(MAIN.rfind("    helper\n").expect("call present") + 4)
            .expect("tiny test source");
        let candidates =
            tcl_syntax::naming::command_resolution_candidates("::app", &[] as &[String], "helper");

        let exports = index.export_snapshot();
        let in_document = crate::definition::resolve_called_proc(
            &main,
            MAIN,
            "::app",
            "helper",
            call,
            crate::definition::CallResolution::document_only().in_program(
                crate::definition::ProgramExports {
                    uri: "file:///main.tcl",
                    oracle: exports.as_ref(),
                },
            ),
        )
        .map(|p| p.qualified_name.clone());

        // The cross-document tier answers the import question; when no import
        // is live the call reaches whatever the workspace defines under the
        // first candidate, which is the local proc.
        let cross_document = index
            .resolve_wildcard_import(
                "helper",
                &candidates,
                CallSite {
                    uri: "file:///main.tcl",
                    at: call,
                    enclosing_body: main.innermost_definition_body_span(call),
                },
            )
            .or_else(|| {
                candidates
                    .iter()
                    .find(|c| index.defines_command(c))
                    .cloned()
            });
        (in_document, cross_document)
    }

    #[test]
    fn both_tiers_shadow_when_another_file_holds_the_covering_export() {
        // TP (CRITICAL) — the export that decides it lives in `exports.tcl`.
        // Oracle: SRC / `::src::helper`.
        let exports = analyse("namespace eval src {\n    namespace export helper\n}\n");
        let (in_document, cross_document) =
            both_tiers_on_main(&[("file:///exports.tcl", &exports)]);
        assert_eq!(in_document.as_deref(), Some("::src::helper"));
        assert_eq!(
            in_document, cross_document,
            "the two tiers must reach the same command",
        );
    }

    #[test]
    fn both_tiers_keep_the_local_when_the_whole_program_exports_nothing() {
        // TN, byte-identical `MAIN` — nothing anywhere exports `helper`, so
        // the `-force` import binds only `other` and the local definition
        // survives. Oracle: LOCAL / `::app::helper`.
        let unrelated = analyse("namespace eval other {\n    proc q {} {}\n}\n");
        let (in_document, cross_document) =
            both_tiers_on_main(&[("file:///unrelated.tcl", &unrelated)]);
        assert_eq!(in_document.as_deref(), Some("::app::helper"));
        assert_eq!(
            in_document, cross_document,
            "the two tiers must reach the same command",
        );
    }

    #[test]
    fn both_tiers_shadow_when_the_source_namespace_is_wholly_in_another_file() {
        // TP — the third oracle shape: `::src`'s procs *and* its export are
        // elsewhere, so this document sees only the `-force` import and the
        // local proc it deletes. Oracle: SRC / `::src::helper`.
        let main = analyse(WHOLLY_FOREIGN);
        let lib = analyse(
            "namespace eval src {\n    proc helper {} { return SRC }\n    namespace export helper\n}\n",
        );
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///main.tcl", &main);
        index.add_document("file:///lib.tcl", &lib);
        let call = u32::try_from(WHOLLY_FOREIGN.rfind("    helper\n").expect("call present") + 4)
            .expect("tiny test source");
        let candidates =
            tcl_syntax::naming::command_resolution_candidates("::app", &[] as &[String], "helper");
        let exports = index.export_snapshot();
        // In-document: the source is in no local table, so the resolver cannot
        // *name* the target — but it must refuse to answer with the local
        // definition the import deleted, which is the abstention the shadow
        // gate exists for.
        let ctx = crate::definition::CallResolution::document_only().in_program(
            crate::definition::ProgramExports {
                uri: "file:///main.tcl",
                oracle: exports.as_ref(),
            },
        );
        assert!(crate::definition::forced_import_shadows_call(
            &main,
            ctx,
            "helper",
            &candidates,
            call,
        ));
        assert!(
            crate::definition::resolve_called_proc(
                &main,
                WHOLLY_FOREIGN,
                "::app",
                "helper",
                call,
                ctx
            )
            .is_none(),
            "the deleted local definition must not be the answer",
        );
        // …and the cross-document tier supplies the name.
        assert_eq!(
            index.resolve_wildcard_import(
                "helper",
                &candidates,
                CallSite {
                    uri: "file:///main.tcl",
                    at: call,
                    enclosing_body: main.innermost_definition_body_span(call),
                },
            ),
            Some("::src::helper".to_owned()),
        );
    }

    /// The `-force` shape whose source namespace is wholly in another file.
    const WHOLLY_FOREIGN: &str = "namespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";

    #[test]
    fn the_settled_call_moves_to_the_import_source_when_the_shadow_is_live() {
        // The same fact on the *settle* path find-references reads (issue
        // #1116 item 1): a candidate naming the command a `-force` import
        // deleted must not settle the call, or find-references files it under
        // a definition go-to-definition no longer answers with.
        let exports = analyse("namespace eval src {\n    namespace export helper\n}\n");
        let main = analyse(MAIN);
        let index = WorkspaceIndex::from_documents([
            ("file:///main.tcl", &main),
            ("file:///exports.tcl", &exports),
        ]);
        assert_eq!(
            index
                .linked_invocations_of("::src::helper", "file:///exports.tcl")
                .len(),
            1,
            "the shadowed call is a reference to the import source",
        );
        assert!(
            index
                .linked_invocations_of("::app::helper", "file:///exports.tcl")
                .is_empty(),
            "…and not to the command the import deleted",
        );
    }

    #[test]
    fn the_settled_call_stays_local_when_the_program_exports_nothing() {
        // TN, byte-identical `MAIN` — with nothing exporting `helper` the
        // import binds nothing and the call still belongs to the local proc.
        let unrelated = analyse("namespace eval other {\n    proc q {} {}\n}\n");
        let main = analyse(MAIN);
        let index = WorkspaceIndex::from_documents([
            ("file:///main.tcl", &main),
            ("file:///unrelated.tcl", &unrelated),
        ]);
        assert_eq!(
            index
                .linked_invocations_of("::app::helper", "file:///unrelated.tcl")
                .len(),
            1,
            "the surviving local definition owns the call",
        );
        assert!(
            index
                .linked_invocations_of("::src::helper", "file:///unrelated.tcl")
                .is_empty(),
            "…and an import that bound nothing creates no reference",
        );
    }

    #[test]
    fn the_export_snapshot_answers_unknown_for_a_namespace_no_file_declares() {
        // The abstention, stated directly on the oracle: `::msgcat` is an
        // installed package, in no indexed document. Silence about its exports
        // is not evidence that it exports nothing — the same rule
        // `live_command_links` already applies to exact imports.
        use crate::namespace_import::NamespaceExportOracle as _;
        let app = analyse("namespace eval app {\n    namespace import -force ::msgcat::*\n}\n");
        let index = WorkspaceIndex::from_documents([("file:///app.tcl", &app)]);
        let snapshot = index.export_snapshot();
        let site = RunPoint {
            uri: "file:///app.tcl",
            at: 0,
            enclosing_body: None,
        };
        assert_eq!(
            snapshot.exported_at("::msgcat", "mc", site),
            ExportVerdict::Unknown,
        );
        // …while a namespace the workspace *can* see gives a real negative.
        assert_eq!(
            snapshot.exported_at("::app", "mc", site),
            ExportVerdict::NotExported,
        );
    }
}
