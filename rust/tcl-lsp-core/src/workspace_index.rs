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
//! `namespace eval ns { variable v }` in a sibling document.  An **unqualified**
//! `$v` names whichever cell the local scope chain supplies, which is a
//! per-document question with no statically-sound cross-file answer, so
//! proc locals and bare occurrences are still not indexed.
//!
//! **Namespaces** are indexed as first-class symbols too
//! ([`WorkspaceNamespaceRef`]): every word the registry marks
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

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use crate::namespace_import::ExportVerdict;
use crate::source_graph::RunPoint;
use crate::workspace_symbols::{
    IndexedWorkspaceSymbol, WorkspaceSymbolKind, matches_query, namespace_of,
};
use tcl_compiler::analyser::{AnalysisResult, MemberRetractionRecord, MemberSide};
use tcl_compiler::ir::MethodKind;
use tcl_lexer::Span;
use tcl_syntax::naming::{key_holder_and_tail, root_unrooted_key, unroot_rooted_key};

/// One proc definition recorded in the workspace index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceProc {
    /// Exact original command publication receipt; report keys grant no identity.
    pub source_name: Option<tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
    /// Document the proc is defined in (the `analyses` map key).
    pub uri: String,
    /// Simple (tail) name, e.g. `greet`.
    pub name: String,
    /// Fully-qualified name, e.g. `::myns::greet`.
    pub qualified_name: String,
    /// Declared parameter count (for completion detail).
    pub param_count: usize,
    /// The `(min, max)` argument arity this proc accepts, straight from
    /// [`tcl_compiler::analyser::ProcDef::arity`] — so a trailing `args` is
    /// unbounded, a defaulted parameter lowers the minimum, and a **computed**
    /// parameter list (`proc p $params {…}`) abstains to the fully-open
    /// `0..UNLIMITED` rather than reading as "takes no arguments".
    ///
    /// [`Self::param_count`] cannot answer an arity question: it is the raw
    /// formal count, which says nothing about defaults or `args`. Cross-file
    /// arity checking needs the real envelope, and it needs it
    /// from the same index that settles the call, so a caller is checked
    /// against the command navigation says it actually reaches.
    pub arity: tcl_registry::Arity,
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
    /// Genuine original class publication owner; metadata reports supply none.
    pub original_declaration:
        Option<tcl_compiler::signature_scan::original_name::SourceOriginalNameOccurrence>,
    /// Exact original own-table member declarations and effects from that owner.
    pub original_members: Option<tcl_compiler::analyser::types::OriginalSourceMemberLedger>,
    /// Original relation operands and receiver-specific effects from that owner.
    pub original_relations:
        Option<tcl_compiler::analyser::types::OriginalSourceClassRelationLedger>,
    /// Original object-command publication and relation lookup receipts.
    pub source_name: Option<tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
    /// Whether declaration reports collide across distinct retained source slots.
    pub source_name_ambiguous: bool,
    /// Original caller contexts for superclass and mixin name lookups.
    pub relation_lookups: std::collections::HashMap<
        String,
        Option<tcl_compiler::signature_scan::scope::SignatureSourceLookup>,
    >,
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
    /// The methods this record *directly defines* — the typed method table:
    /// each entry carries its receiver kind and its
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
    /// touched.  Without a channel of its own the flip would not travel at
    /// all, leaving `b.tcl`'s class-command dispatch advertising a member
    /// that `::C m` rejects with `unknown method "m"`.
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
    /// sourcing the extension deletes. Applied as an
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
    /// Whether this class's **own command** constructs an instance from a
    /// bare unrecognised word and yields its name — see
    /// [`tcl_compiler::analyser::ClassDef::class_command_fallback`] (Tk's
    /// `::tk::IconList .il`).
    ///
    /// Carried across the document boundary because the proof lives on the
    /// class's *metaclass*, which a consuming document need never see.
    pub bare_word_construction: bool,
    /// Byte spans of this record's `constructor` name tokens, in declaration
    /// order (`oo::configurable` admits more than one).  Constructors are not
    /// dispatchable members, so they stay out of [`Self::methods`]; they are
    /// carried only so `workspace/symbol` can offer them from an unopened file
    /// the way the per-document outline does.
    pub constructor_spans: Vec<Span>,
}

impl WorkspaceClass {
    /// Whether this record's definer dispatches its class-scoped members as
    /// a single `::`-qualified identifier (`Factory::make`) rather than the
    /// two-word `Factory make` shape — true for [incr Tcl] only.  Registry
    /// data (`DefinerFamily`), not a hardcoded command-name check.
    #[must_use]
    pub fn is_itcl(&self, dialect: &'static tcl_dialect::DialectProfile) -> bool {
        crate::registry_for_dialect_profile(dialect)
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
    /// of the class — the **arrival** half of the tombstone channel.
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
            .find(|m| m.name == name && m.kind != CLASS_METHOD)
    }

    /// The typed record for the *class-receiver* member `name` — a
    /// `classmethod` / `self method` / snit `typemethod` — if this record
    /// defines one.  The counterpart of [`Self::instance_method`].
    #[must_use]
    pub fn class_method(&self, name: &str) -> Option<&WorkspaceMethod> {
        self.methods
            .iter()
            .find(|m| m.name == name && m.kind == CLASS_METHOD)
    }
}

/// Which receiver side a [`WorkspaceMethod`] is declared on — the same split
/// [`WorkspaceClass::instance_method`] / [`WorkspaceClass::class_method`] make,
/// named once so the member fold and the tombstone lookup agree on it.
#[must_use]
fn method_side(m: &WorkspaceMethod) -> MemberSide {
    if m.kind == CLASS_METHOD {
        MemberSide::ClassObject
    } else {
        MemberSide::Instance
    }
}

/// One member of a class as the **workspace** sees it: the declaring record's
/// entry, keyed under the name the class actually dispatches it by once every
/// cross-document retraction and arrival has been applied.
///
/// The raw [`WorkspaceClass::methods`] table is per-record and additive — it
/// records what that record's own body declared and nothing else — so a member
/// moved by a cross-file `oo::define ::C { renamemethod old new }` is still
/// listed as `old` on the defining record and not listed at all on the stub.
/// Resolving one name against the table joins the two halves
/// ([`WorkspaceIndex::dispatch_chain`]); *listing* the table must join them
/// too, or anything that enumerates a class's members (`workspace/symbol`, an
/// outline, a member completion universe) reports the pre-rename name.
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
/// a cross-document `deletemethod` removed it outright.
///
/// The one place that decision is made; [`WorkspaceIndex::effective_members`]
/// (and through it [`WorkspaceIndex::dispatch_chain`]) and the
/// `workspace/symbol` member walk all route through it, so resolving a name
/// and listing the member set cannot disagree.
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
    // stub really removes the member. The union is taken
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
/// dispatch.
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
    /// two apart.
    pub is_self_method: bool,
    /// Byte span of the method's name token in the declaring record's
    /// document.  Carried so `workspace/symbol` can locate a method in a file
    /// the editor has never opened; cross-file *dispatch* uses
    /// the name and the visibility flags and does not need it.
    pub name_span: Span,
}

/// The access context of a method call site — `TclOO` dispatches an
/// external `$obj m` through exported methods only, while an internal
/// `my m` reaches unexported ones too.
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
    /// Original naming producer. Reporting names cannot recreate its bytes,
    /// source channel or independently selected dialect policy.
    pub original_name_input: Option<tcl_compiler::signature_scan::scope::SignatureSourceNameInput>,
    /// Positioned original lookup geometry, separate from declaration advice
    /// and from a successfully selected command implementation.
    pub original_lookup: Option<tcl_compiler::command_binding::OriginalCommandLookup>,
    /// Lookup purpose retained independently of the source edit span.
    pub lookup: tcl_compiler::signature_scan::types::SignatureCommandLookup,
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
    /// The analyser proved that [`Self::resolution_candidates`]'s selected
    /// target is a proc/class definition in this same document and live for
    /// this call's execution position.  Unlike the workspace's final
    /// live-name set, this remains true for a call which provably ran before
    /// a later load-level deletion.
    pub resolved_user_definition: Option<String>,
    /// Actual source implementation selected at this call, independent of its
    /// current slot and the document-final declaration inventory.
    pub resolved_definition: Option<tcl_compiler::command_binding::SourceCommandDefinition>,
    /// Complete called-slot navigation receipt. A known non-definition cannot
    /// borrow a same-named procedure or class from workspace assistance.
    pub resolved_command_reference: Option<tcl_compiler::command_binding::SourceCommandReference>,
    /// Byte span of the command-head token in `uri`'s source.
    pub range: Span,
    /// The span does not carry the written command name (an indirect site —
    /// a constant `$cmd` head): references may report it, but the
    /// cross-document rename path must not rewrite it.
    pub indirect: bool,
    /// `false` when renaming this invocation's resolved command cannot be
    /// completed soundly from source edits (an indirect site with at least
    /// one contributing constant that has no exact writable source span) —
    /// the rename providers must abstain for the whole symbol rather than
    /// leave the site dispatching the old name.
    pub rename_safe: bool,
    /// `Some(provenance)` when this site is the **subcommand word** of an
    /// `<ensemble> <sub> …` dispatch, mirroring
    /// [`tcl_compiler::signature_scan::types::SignatureCommandInvocation::ensemble_dispatch`]
    /// so the cross-document rename path applies the same gate
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
    /// [`CallSite`].
    ///
    /// The naive per-row lookup would be `O(procs × invocations)`;
    /// [`WorkspaceIndex::enclosing_body_spans`] computes the whole column in
    /// one stack sweep instead, `O((P + I) log (P + I))` per document.
    pub enclosing_body: Option<Span>,
}

/// One **registry symbol-definer** definition recorded in the index — a
/// `tcltest::test` case, a `testConstraint`, a `customMatch` mode, or an
/// iRules `when EVENT` handler.
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
/// the cross-document half of the namespace tier, lifted verbatim from
/// [`tcl_compiler::analyser::types::NamespaceRef`].
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
    /// Original source operand and independently retained geometry.
    pub source: tcl_compiler::analyser::types::NamespaceRef,
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
    /// Sealed conditional expression selected from genuine original words.
    /// It retains the full analysis/context and supplies no actual file load.
    pub original_path_expression:
        Option<tcl_compiler::auto_path_eval::OriginalSourcePathExpression>,
    /// Retained possible child source edge and its full source/input lineage.
    /// No actual file evaluation or completed execution is established.
    pub original_interpreter_source_load:
        Option<std::sync::Arc<tcl_compiler::analyser::OriginalInterpreterSourceLoad>>,
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
    /// namespace, so the sourced document's definitions re-home under
    /// this namespace — see [`WorkspaceIndex::source_seed_map`].
    pub site_namespace: String,
    /// Span of the innermost proc/class body containing the `source`
    /// statement; `None` at load level.  See
    /// [`WorkspaceGlobImport::enclosing_body`] — the same execution-order
    /// fact, needed here so a cross-document interpreter-state question
    /// ("had this statement already run when the child was loaded?") applies
    /// the identical [`tcl_compiler::analyser::indirection::in_effect_within`]
    /// rule the single-document tier does.
    pub enclosing_body: Option<Span>,
}

/// One unconditional `package prefer latest` recorded in the index.
///
/// `package prefer` latches **interpreter-global** state, so a raise in a file
/// that runs before this one really does change this one's version selection.
/// Which file runs first is not knowable in general — but along the `source`
/// graph it is: a file that `source`s another runs the sourcing statement
/// before the sourced file loads at all.  See
/// [`WorkspaceIndex::source_ancestor_prefers_latest`].
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
///
/// It is also an **ordered statement**: by the time it returns, the package's
/// files have run.  [`WorkspaceIndex::package_run_edges`] turns that into the
/// package half of the load order, which is why the position
/// fields are here and not only on the `source` record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackageRequire {
    /// Exact package-key projection; full original source ownership remains
    /// in the analyser and does not enter the body-free settlement summary.
    pub original_name: Option<tcl_registry::native_package::NativePackageNameKey>,
    /// Each alternative version word retained at this requirement.
    pub requirements: Vec<String>,
    /// Whether this requirement selected exact version matching.
    pub exact: bool,
    /// Local source preference at this requirement. The host independently
    /// supplies any interpreter or ancestor preference on entry.
    pub prefer: crate::package_resolver::PackagePrefer,
    /// Document containing the `package require` statement.
    pub uri: String,
    /// Required package name (the `NAME` argument).
    pub name: String,
    /// Byte offset of the `package` command word in `uri`'s source.
    pub at: u32,
    /// Span of the innermost proc/class body containing the require; `None`
    /// at load level.  Read by
    /// [`tcl_compiler::analyser::indirection::in_effect_within`] exactly as
    /// [`WorkspaceSource::enclosing_body`] is.
    pub enclosing_body: Option<Span>,
    /// `true` when the require sits inside a guarded branch (`if` / `catch` /
    /// `try`) and so may never run — the standard optional-dependency idiom
    /// `if {[catch {package require Tk}]} { … }`.  It still counts as a
    /// *declared dependency* for W120, which is why the record is kept rather
    /// than dropped; it establishes no order.
    pub conditional: bool,
}

impl WorkspacePackageRequire {
    /// Project advice without rebuilding a package key from its report label.
    /// Interpreter preference is a caller fact; local `latest` is a latch.
    #[must_use]
    pub fn original_advice(
        &self,
        default: crate::package_resolver::PackagePrefer,
    ) -> crate::package_resolver::PackageRequirementAdvice {
        use crate::package_resolver::{
            PackagePrefer, PackageRequirementAdvice, PackageRequirementAdviceKey,
        };
        let key = self
            .original_name
            .as_ref()
            .map_or(PackageRequirementAdviceKey::Unknown, |name| {
                PackageRequirementAdviceKey::Original(name.clone())
            });
        let prefer = if default == PackagePrefer::Latest || self.prefer == PackagePrefer::Latest {
            PackagePrefer::Latest
        } else {
            PackagePrefer::Stable
        };
        PackageRequirementAdvice::new(key, self.requirements.clone(), self.exact, prefer)
    }
}

/// One `package provide NAME` declaration recorded in the index.
///
/// The other end of a [`WorkspacePackageRequire`]: the document that, when it
/// runs, makes the package available.  A `package require` whose name this
/// resolves to a **single** indexed document is the load order's evidence that
/// that document had run by the require.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackageProvide {
    /// Exact original package-key projection, independent of its reporting label.
    pub original_name: Option<tcl_registry::native_package::NativePackageNameKey>,
    /// Document containing the `package provide` statement.
    pub uri: String,
    /// Provided package name (the `NAME` argument).
    pub name: String,
    /// Byte offset of the `package` command word in `uri`'s source.
    pub at: u32,
    /// `true` when the provide sits inside a guarded branch and so may never
    /// run — the shim idiom
    /// [`tcl_compiler::analyser::types::PackageProvide::conditional`]
    /// documents.  Such a document is not this package's provider.
    pub conditional: bool,
}

/// One `package ifneeded NAME VERSION SCRIPT` registration recorded in the
/// index.
///
/// A registration is a statement that the package's loading is *this script's*
/// business — and the script is arbitrary, runs later, and runs in the global
/// namespace.  The load order reads the mere existence of one as "which
/// statements a `package require NAME` runs is not static here" and builds no
/// package edge for that name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackageIfneeded {
    /// Original registered package key; registration does not prove loading.
    pub original_name: Option<tcl_registry::native_package::NativePackageNameKey>,
    /// Document containing the `package ifneeded` statement.
    pub uri: String,
    /// Package the load script is registered for.
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
    /// Exact authored alias publication slot, when supplied by its original
    /// declaration. Its reported full name is not new lookup input.
    pub linked_source_name: Option<tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
    /// Fully-qualified name (`::`-rooted) the link resolves *to*: the import
    /// pattern's source, the alias `TARGET`, or the `rename` `OLD` — the
    /// command whose references a call through `linked_qname` joins.
    pub target_qname: String,
    /// Exact authored global alias target slot. Its report need not be a
    /// globally callable spelling; positioned source references resolve it.
    /// Imports and renames retain their existing declaration carriers.
    pub target_source_name: Option<tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
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
    /// at creation time would go stale the moment that file changes.
    /// [`WorkspaceIndex::live_command_links`] applies it against the
    /// current index, cached per [`WorkspaceIndex::generation`].
    pub import_gate: Option<WorkspaceImportGate>,
}

/// The export-snapshot condition an exact `namespace import` link must satisfy
/// to be installed at all — see [`WorkspaceCommandLink::import_gate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceImportGate {
    /// Genuine whole original pattern and caller scope; optional display fields
    /// do not recover this receipt when unavailable.
    pub original_pattern:
        Option<tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
    /// Exact original selected namespace geometry; never parsed from its display.
    pub native_source: Option<tcl_syntax::naming::NativeNamespacePatternSource>,
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
    /// Genuine whole original pattern and caller scope; optional display fields
    /// do not recover this receipt when unavailable.
    pub original_pattern:
        Option<tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
    /// Exact original selected namespace geometry; never parsed from its display.
    pub native_source: Option<tcl_syntax::naming::NativeNamespacePatternSource>,
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
    /// export declared in the same file is judged against this offset.
    /// Meaningless against another file's offsets — which document
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
    /// weaker `at <= import_at`, which would reject such an export and lose a
    /// real imported alias.
    pub enclosing_body: Option<Span>,
    /// `true` when the import carried its declared leading option word —
    /// `namespace import -force`.
    ///
    /// Decides what happens when the importing namespace already holds a
    /// command of the imported name (oracle tclsh 8.6.14 / 9.0.4): without
    /// `-force` the import raises `can't import command "p":
    /// already exists` and installs **nothing**, so a bare call still reaches
    /// the local definition; with `-force` it silently replaces it and the
    /// call reaches the source (`namespace origin` → `::src::p`). See
    /// [`tcl_compiler::signature_scan::types::SignatureNamespaceImport::forced`]
    /// for why this is registry data rather than a `-force` name match.
    pub forced: bool,
}

/// One `namespace forget` **event** recorded in the index — the removal half
/// of the import edge's lifecycle log.
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
/// source kills the alias too (oracle on
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

/// One straight-line command-name retirement recorded in the index.
///
/// `rename OLD NEW` preserves the command object (and therefore any import
/// aliases which hold it), but it stops `OLD` being a callable name.  A
/// destruction has the same consequence for the name as well as ending the
/// command object.  Keeping that narrower name-liveness fact apart from
/// [`WorkspaceCommandDeletion`] lets the import lifecycle retain Tcl's
/// object-identity rule while all direct command lookups consistently hide a
/// definition after its name has been moved or destroyed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceCommandRetirement {
    /// Document containing the lifecycle event.
    pub uri: String,
    /// The `::`-normalised qualified name which stopped being callable.
    pub qualified_name: String,
    /// Byte offset of the lifecycle statement within [`Self::uri`].
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
/// export declared in *another* file.
///
/// An *event*, not a member of a set: `-clear` tombstones and ordering are
/// carried through so the cross-document tier applies the same per-import-site
/// snapshot the same-document one does (see
/// [`crate::namespace_import`]). Ordering only means something *within* a
/// document, which is why [`Self::uri`] and [`Self::at`] are always read
/// together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceNamespaceExport {
    /// Original counted event, retained independently of the reporting label.
    pub original: Option<tcl_compiler::signature_scan::original_name::SourceNamespaceExport>,
    /// Unrepresented source operand; this is a May barrier, never a name value.
    pub unknown: bool,
    /// Document the `namespace export` is in.
    pub uri: String,
    /// Exporting namespace, with leading `::`.
    pub ns: String,
    /// Exported pattern text, exactly as written (relative to `ns`). Empty
    /// for a [`Self::clears`] tombstone.
    pub pattern: String,
    /// Byte offset of the event within [`Self::uri`].
    pub at: u32,
    /// Genuine original declaration-body geometry for source ordering.
    pub enclosing_body: Option<Span>,
    /// `true` for a `namespace export -clear` tombstone.
    pub clears: bool,
}

/// Collect `items` into source order by the span `key` reports, so a
/// `HashMap`-valued analyser table lands in the index deterministically
/// instead of in the process's random hash order.
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

/// One class record's `(exports, unexports)` pair **for `side`** — the sided
/// visibility lookup [`WorkspaceIndex::dispatch_chain`] reads.
///
/// `exports`/`unexports` are the instance-side record by contract and
/// `class_exports`/`class_unexports` the class-object-side one; naming the
/// choice once is what keeps a `self unexport m` from ever silencing an
/// identically-named instance method.
fn visibility_sets_for(c: &WorkspaceClass, side: MemberSide) -> (&[String], &[String]) {
    match side {
        MemberSide::Instance => (&c.exports, &c.unexports),
        MemberSide::ClassObject => (&c.class_exports, &c.class_unexports),
    }
}

/// The names of a `HashSet`-valued analyser table, in a stable order.  See
/// [`sorted_by_span`] for why the source order matters.
fn sorted_names(names: &std::collections::HashSet<String>) -> Vec<String> {
    let mut out: Vec<String> = names.iter().cloned().collect();
    out.sort();
    out
}

fn resolved_user_definition(
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
) -> Option<String> {
    if inv.resolved_user_definition {
        inv.resolved_qualified_name.clone()
    } else {
        None
    }
}

/// Exact source/configuration/Registry currency and selected declaration advice.
/// This readonly invalidation context supplies no lookup, publication, Normal
/// completion, execution frame or native command-table capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDiagnosticSourceContext {
    image: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    registry: tcl_registry::RegistrySemanticKey,
    original_names: bool,
}

impl WorkspaceDiagnosticSourceContext {
    /// Retain complete source/configuration, Registry identity and advice mode.
    /// This readonly currency does not issue a lookup or native capability.
    #[must_use]
    pub fn for_analysis(analysis: &AnalysisResult) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let registry = analysis.resolved_registry()?.snapshot().semantic_key();
        let image = analysis
            .retained_command_realm()?
            .original_source_image()?
            .clone();
        analysis
            .matches_original_source_image(&image, config)
            .then_some(Self {
                image,
                config,
                registry,
                original_names: !analysis.allows_lexical_declaration_advice(),
            })
    }
    /// Complete actual source bytes and input channel retained at ingress.
    #[must_use]
    pub fn image(&self) -> &tcl_lexer::SourceImage {
        &self.image
    }
    /// Full scanner configuration, without profile-name reconstruction.
    #[must_use]
    pub const fn config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
    /// Independently selected original versus logical compatibility advice.
    #[must_use]
    pub const fn uses_original_names(&self) -> bool {
        self.original_names
    }
}

/// The fourteen record tables **one document** contributes to the index.
///
/// Grouping the tables per document is what makes
/// [`WorkspaceIndex::remove_document`] cost the document's own records rather
/// than the workspace's.  Flat workspace-wide vectors would make a
/// removal fourteen `Vec::retain` passes with a `String` compare per element
/// over tables that hold one row per call site and per qualified variable
/// occurrence — 10⁵–10⁶ rows on a tcllib-sized workspace — and the server runs
/// a removal on every diagnostics publish.
///
/// Consumers never see this type: [`WorkspaceIndex`] exposes each table as a
/// workspace-wide iterator that chains the per-document vectors in slot order,
/// which is the order the records were added in.
#[derive(Debug, Clone, Default)]
struct DocumentRecords {
    /// Workspace document ownership, independent of equal source images.
    uri: String,
    /// Index-wide mutation token for the indexed revision. The server uses it
    /// to prove that a closed file did not reindex while its source was being
    /// loaded for byte-span to LSP-range conversion.
    revision: u64,
    /// The dialect this document was analysed under — carried straight from
    /// [`AnalysisResult::dialect`], so the registry consulted for a decision
    /// about this document is the one it was actually analysed with.
    ///
    /// A workspace is not one dialect: an iRule and a plain Tcl script can sit
    /// in one folder, and which commands a fresh interpreter holds differs
    /// between them (`HTTP::uri` exists for `f5-irules` and nowhere else).
    /// Empty for a default-constructed record, which
    /// [`WorkspaceIndex::registry_for`] reads as "no dialect known".
    dialect: String,
    diagnostic_source: Option<WorkspaceDiagnosticSourceContext>,
    procs: Vec<WorkspaceProc>,
    classes: Vec<WorkspaceClass>,
    /// Complete original declarations, including opaque names and records
    /// whose presentation collides. These are declaration inventories, not
    /// claims that a command is installed or callable at a particular time.
    original_command_world: Option<tcl_compiler::command_binding::OriginalCompletedCommandWorld>,
    original_source_procedures:
        Option<tcl_compiler::command_binding::OriginalSourceProcedurePublications>,
    original_source_classes: Option<tcl_compiler::command_binding::OriginalSourceClassPublications>,
    original_source_input: Option<tcl_compiler::analyser::ResolvedAnalysisInput>,
    original_procs: Vec<
        tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ProcDef,
        >,
    >,
    original_classes: Vec<
        tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ClassDef,
        >,
    >,
    original_symbols: Vec<IndexedWorkspaceSymbol>,
    original_vendor_declarations: Vec<crate::vendor_declaration::RetainedOriginalVendorDeclaration>,
    original_method_queries: Vec<crate::method_symbol::OriginalMethodQuery>,
    original_variables:
        Vec<tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence>,
    variables: Vec<WorkspaceVariable>,
    variable_refs: Vec<WorkspaceVariableRef>,
    variable_aliases: Vec<WorkspaceVariableAlias>,
    namespace_refs: Vec<WorkspaceNamespaceRef>,
    invocations: Vec<WorkspaceInvocation>,
    sources: Vec<WorkspaceSource>,
    /// Raw path-constant facts, straight from
    /// [`AnalysisResult::path_constant_assignments`] — unfolded write facts
    /// the source-edge resolver chain-folds per parent (with the parent's
    /// own path standing in for `[info script]`) before resolving a computed
    /// `source` argument.  Raw here for the same reason as there: the fold
    /// depends on where the document lives, the index must not.
    path_constant_assignments: tcl_compiler::auto_path_eval::PathConstantAssignments,
    package_requires: Vec<WorkspacePackageRequire>,
    package_provides: Vec<WorkspacePackageProvide>,
    package_ifneededs: Vec<WorkspacePackageIfneeded>,
    package_prefers: Vec<WorkspacePackagePrefer>,
    command_links: Vec<WorkspaceCommandLink>,
    glob_imports: Vec<WorkspaceGlobImport>,
    original_namespace_patterns:
        Vec<tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
    namespace_exports: Vec<WorkspaceNamespaceExport>,
    namespace_forgets: Vec<WorkspaceNamespaceForget>,
    command_deletions: Vec<WorkspaceCommandDeletion>,
    command_retirements: Vec<WorkspaceCommandRetirement>,
    defined_symbols: Vec<WorkspaceDefinedSymbol>,
}

#[derive(Clone, PartialEq, Eq)]
struct OriginalPublicationDependency {
    slot: tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    kind: tcl_compiler::command_binding::OriginalCommandPublicationKind,
    definition: Option<(
        tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
    )>,
}

/// The complete per-document input surface the command-settlement walk reads,
/// directly or through its three shared tables.  Keeping this as an exact
/// value, rather than a lossy fingerprint, makes the incremental boundary a
/// correctness property: a body edit may retain other documents' answers only
/// when every resolution-relevant fact is byte-for-byte unchanged.
///
/// Invocation sites intentionally do not appear here.  Replacing them changes
/// the one document which must be re-settled, but cannot change how another
/// document's call resolves.
#[derive(Clone, PartialEq, Eq)]
struct SettlementDependencies {
    dialect: String,
    /// Only the facts `defined_command_names` and the nested-builtin gate
    /// read.  Parameter edits and a shifted declaration span cannot change a
    /// call's target, so retaining the whole `WorkspaceProc` would turn those
    /// ordinary body edits into false full invalidations.
    procs: Vec<(String, bool)>,
    /// Classes contribute command existence by qualified name; their member
    /// table is used by a different cross-file query, not settlement.
    classes: Vec<String>,
    /// `observable_namespaces` reads declaration identity only.  References
    /// to a namespace and their spans do not affect an import gate.
    declared_namespaces: Vec<String>,
    original_declarations: Vec<(
        tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
    )>,
    original_publications: Vec<OriginalPublicationDependency>,
    original_world_closed: bool,
    original_source_procedure_candidates: Vec<(
        tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
        tcl_core_types::ByteCommandSlot,
    )>,
    original_source_class_candidates: Vec<(
        tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
        tcl_core_types::ByteCommandSlot,
        Vec<tcl_compiler::command_binding::SourceCommandTransitionObligation>,
    )>,
    sources: Vec<WorkspaceSource>,
    /// A changed constant can change which child a computed source row
    /// resolves to, so the raw assignments are resolution-relevant exactly
    /// as the source rows themselves are.
    path_constant_assignments: tcl_compiler::auto_path_eval::PathConstantAssignments,
    package_requires: Vec<WorkspacePackageRequire>,
    package_provides: Vec<WorkspacePackageProvide>,
    package_ifneededs: Vec<WorkspacePackageIfneeded>,
    package_prefers: Vec<WorkspacePackagePrefer>,
    command_links: Vec<WorkspaceCommandLink>,
    glob_imports: Vec<WorkspaceGlobImport>,
    original_namespace_patterns:
        Vec<tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
    namespace_exports: Vec<WorkspaceNamespaceExport>,
    namespace_forgets: Vec<WorkspaceNamespaceForget>,
    command_deletions: Vec<WorkspaceCommandDeletion>,
    command_retirements: Vec<WorkspaceCommandRetirement>,
}

impl DocumentRecords {
    fn settlement_dependencies(&self) -> SettlementDependencies {
        SettlementDependencies {
            dialect: self.dialect.clone(),
            procs: self
                .procs
                .iter()
                .map(|proc_def| (proc_def.qualified_name.clone(), proc_def.nested))
                .collect(),
            classes: self
                .classes
                .iter()
                .map(|class_def| class_def.qualified_name.clone())
                .collect(),
            declared_namespaces: self
                .namespace_refs
                .iter()
                .filter(|namespace| namespace.declares)
                .map(|namespace| namespace.qualified_name.clone())
                .collect(),
            original_declarations: self
                .original_procs
                .iter()
                .map(|declaration| {
                    (
                        declaration.name().slot().clone(),
                        declaration.name().policy(),
                    )
                })
                .chain(self.original_classes.iter().map(|declaration| {
                    (
                        declaration.name().slot().clone(),
                        declaration.name().policy(),
                    )
                }))
                .collect(),
            original_publications: self
                .original_command_world
                .iter()
                .flat_map(|world| world.declarations())
                .map(|publication| {
                    let site = publication
                        .definition()
                        .map(|definition| &definition.allocation().site);
                    let origin = self
                        .original_procs
                        .iter()
                        .find(|declaration| Some(declaration.declaration_site()) == site)
                        .map(|declaration| {
                            (
                                declaration.name().slot().clone(),
                                declaration.name().policy(),
                            )
                        })
                        .or_else(|| {
                            self.original_classes
                                .iter()
                                .find(|declaration| Some(declaration.declaration_site()) == site)
                                .map(|declaration| {
                                    (
                                        declaration.name().slot().clone(),
                                        declaration.name().policy(),
                                    )
                                })
                        });
                    OriginalPublicationDependency {
                        slot: publication.slot().clone(),
                        policy: publication.policy(),
                        kind: publication.kind(),
                        definition: origin,
                    }
                })
                .collect(),
            original_world_closed: self.original_command_world.is_some(),
            original_source_procedure_candidates: self
                .original_procedure_source_candidates()
                .into_iter()
                .map(|(declaration, slot, policy)| {
                    (slot.clone(), policy, declaration.name().slot().clone())
                })
                .collect(),
            original_source_class_candidates: self
                .original_class_source_candidates()
                .into_iter()
                .map(|(declaration, slot, policy, obligations)| {
                    (
                        slot.clone(),
                        policy,
                        declaration.name().slot().clone(),
                        obligations.to_vec(),
                    )
                })
                .collect(),
            sources: self.sources.clone(),
            path_constant_assignments: self.path_constant_assignments.clone(),
            package_requires: self.package_requires.clone(),
            package_provides: self.package_provides.clone(),
            package_ifneededs: self.package_ifneededs.clone(),
            package_prefers: self.package_prefers.clone(),
            command_links: self.command_links.clone(),
            glob_imports: self.glob_imports.clone(),
            original_namespace_patterns: self.original_namespace_patterns.clone(),
            namespace_exports: self.namespace_exports.clone(),
            namespace_forgets: self.namespace_forgets.clone(),
            command_deletions: self.command_deletions.clone(),
            command_retirements: self.command_retirements.clone(),
        }
    }

    fn original_procedure_source_candidates(
        &self,
    ) -> Vec<(
        &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ProcDef,
        >,
        &tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
    )> {
        let Some(context) = self.diagnostic_source.as_ref() else {
            return Vec::new();
        };
        if let Some(world) = self.original_command_world.as_ref() {
            world
                .declarations()
                .flat_map(|publication| {
                    self.original_procs.iter().filter_map(move |declaration| {
                        (publication.declaration_site() == declaration.declaration_site()
                            && publication.policy() == declaration.name().policy())
                        .then_some((declaration, publication.slot(), publication.policy()))
                    })
                })
                .collect()
        } else if let Some(inventory) = self.original_source_procedures.as_ref() {
            inventory
                .candidates(self.original_procs.iter())
                .into_iter()
                .map(|candidate| {
                    (
                        candidate.declaration(),
                        candidate.source_slot(),
                        candidate.policy(),
                    )
                })
                .collect()
        } else {
            self.original_procs
                .iter()
                .filter(|declaration| {
                    declaration.name_input().source_image() == context.image()
                        && declaration.name_input().lexer_config() == context.config()
                })
                .map(|declaration| {
                    (
                        declaration,
                        declaration.name().slot(),
                        declaration.name().policy(),
                    )
                })
                .collect()
        }
    }

    fn original_class_source_candidates(
        &self,
    ) -> Vec<(
        &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ClassDef,
        >,
        &tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
        &[tcl_compiler::command_binding::SourceCommandTransitionObligation],
    )> {
        if self.diagnostic_source.is_none() {
            return Vec::new();
        }
        if let Some(world) = self.original_command_world.as_ref() {
            world
                .declarations()
                .flat_map(|publication| {
                    self.original_classes.iter().filter_map(move |declaration| {
                        (publication.declaration_site() == declaration.declaration_site()
                            && publication.policy() == declaration.name().policy())
                        .then_some((
                            declaration,
                            publication.slot(),
                            publication.policy(),
                            &[][..],
                        ))
                    })
                })
                .collect()
        } else {
            self.original_source_classes
                .as_ref()
                .map_or_else(Vec::new, |inventory| {
                    inventory
                        .candidates_from_retained_records(
                            self.original_source_input.as_ref(),
                            self.original_classes.iter(),
                        )
                        .into_iter()
                        .map(|candidate| {
                            (
                                candidate.declaration(),
                                candidate.source_slot(),
                                candidate.policy(),
                                candidate.obligations(),
                            )
                        })
                        .collect()
                })
        }
    }

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
            uri,
            revision: _,
            dialect,
            diagnostic_source,
            procs,
            classes,
            original_command_world,
            original_source_procedures,
            original_source_classes,
            original_source_input,
            original_procs,
            original_classes,
            original_symbols,
            original_vendor_declarations,
            original_method_queries,
            original_variables,
            variables,
            variable_refs,
            variable_aliases,
            namespace_refs,
            invocations,
            sources,
            path_constant_assignments,
            package_requires,
            package_provides,
            package_ifneededs,
            package_prefers,
            command_links,
            glob_imports,
            original_namespace_patterns,
            namespace_exports,
            namespace_forgets,
            command_deletions,
            command_retirements,
            defined_symbols,
        } = self;
        uri.clear();
        dialect.clear();
        *diagnostic_source = None;
        path_constant_assignments.clear();
        procs.clear();
        classes.clear();
        *original_command_world = None;
        *original_source_procedures = None;
        *original_source_classes = None;
        *original_source_input = None;
        original_procs.clear();
        original_classes.clear();
        original_symbols.clear();
        original_vendor_declarations.clear();
        original_method_queries.clear();
        original_variables.clear();
        variables.clear();
        variable_refs.clear();
        variable_aliases.clear();
        namespace_refs.clear();
        invocations.clear();
        sources.clear();
        package_requires.clear();
        package_provides.clear();
        package_ifneededs.clear();
        package_prefers.clear();
        command_links.clear();
        glob_imports.clear();
        original_namespace_patterns.clear();
        namespace_exports.clear();
        namespace_forgets.clear();
        command_deletions.clear();
        command_retirements.clear();
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
        let Some(context) = self.diagnostic_source.as_ref() else {
            return;
        };
        if context.uses_original_names() {
            for symbol in &self.original_symbols {
                if out.len() >= limit {
                    return;
                }
                if matches_query(&symbol.name, lower_query)
                    || symbol.container_name.as_ref().is_some_and(|container| {
                        matches_query(&format!("{container}::{}", symbol.name), lower_query)
                    })
                {
                    out.push(symbol.clone());
                }
            }
            return;
        }
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
                    original_location: None,
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
                    original_location: None,
                });
            }
            // Members carry the class's qualified name as their container, so
            // an editor renders them as `ClassName::methodName`.
            //
            // Enumerated through the workspace's member fold, not straight off
            // this record's own table: a member a cross-file `oo::define ::C {
            // renamemethod old new }` moved is declared here under `old` and
            // dispatches as `new`, and one deleted the same way is not a
            // member at all.  An arrived member's location is
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
                        original_location: None,
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
                        original_location: None,
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
                    original_location: None,
                });
            }
        }
    }

    /// Lift one document's analysis into this record set.
    ///
    /// The caller ([`WorkspaceIndex::add_document`]) has already cleared the
    /// slot, so this only ever appends.
    fn index_document(&mut self, uri: &str, analysis: &AnalysisResult) {
        self.uri = uri.to_owned();
        self.dialect.clone_from(&analysis.dialect);
        self.diagnostic_source = WorkspaceDiagnosticSourceContext::for_analysis(analysis);
        self.original_command_world = analysis.original_completed_command_world().cloned();
        self.original_source_input
            .clone_from(&analysis.resolved_input);
        self.original_source_classes = self
            .diagnostic_source
            .as_ref()
            .and_then(|context| context.image().try_text().ok())
            .and_then(|source| {
                tcl_compiler::registry_invocation::source_structure::source_class_publications(
                    source, analysis,
                )
            });
        self.original_source_procedures = self
            .diagnostic_source
            .as_ref()
            .and_then(|context| context.image().try_text().ok())
            .and_then(|source| {
                tcl_compiler::registry_invocation::source_structure::source_procedure_publications(
                    source, analysis,
                )
            });
        self.original_namespace_patterns
            .extend(analysis.original_namespace_patterns().cloned());
        if let Some(context) = self.diagnostic_source.as_ref() {
            if let Ok(source) = std::str::from_utf8(context.image().bytes()) {
                self.original_vendor_declarations.extend(
                    crate::vendor_declaration::retained_declarations(source, analysis)
                        .unwrap_or_default(),
                );
                self.original_symbols
                    .extend(crate::workspace_symbols::original_symbols(
                        uri, source, analysis, context,
                    ));
            }
        }
        self.original_method_queries
            .extend(crate::method_symbol::source_queries(analysis));
        self.original_procs
            .extend(analysis.original_procedure_declarations().cloned());
        self.original_classes
            .extend(analysis.original_class_declarations().cloned());
        // `all_procs` / `all_classes` / a class's `methods` are `HashMap`s, so
        // iterating them directly would order this document's index entries by
        // the process's random hash seed — and consumers that answer with the
        // *first* matching record (go-to-definition's dispatch entry, most
        // visibly) then answer differently run to run.  Source
        // order is the stable, meaningful order: it is also the order the
        // records take effect in when the file is sourced.
        for proc_def in sorted_by_span(analysis.all_procs.values(), |p| p.name_span) {
            self.procs.push(WorkspaceProc {
                source_name: proc_def.source_name.clone(),
                uri: uri.to_owned(),
                name: proc_def.name.clone(),
                qualified_name: proc_def.qualified_name.clone(),
                param_count: proc_def.params.len(),
                arity: proc_def.arity(),
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
        self.original_variables.extend(
            analysis
                .original_variable_symbols
                .iter()
                .filter(|occurrence| occurrence.symbol().is_namespace())
                .cloned(),
        );
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
                original_name_input: inv.original_name_input.clone(),
                original_lookup: inv.original_lookup.clone(),
                lookup: inv.lookup,
                uri: uri.to_owned(),
                name: inv.name.clone(),
                resolution_candidates: inv.resolution_candidates.clone(),
                resolved_user_definition: resolved_user_definition(inv),
                resolved_definition: inv.resolved_definition.clone(),
                resolved_command_reference: inv.resolved_command_reference.clone(),
                range: inv.range,
                indirect: inv.indirect,
                rename_safe: inv.rename_safe,
                ensemble_dispatch: inv.ensemble_dispatch,
                enclosing_body,
            });
        }
        self.index_sources(uri, analysis);
        for pr in &analysis.package_requires {
            self.package_requires.push(WorkspacePackageRequire {
                original_name: pr.original_name.as_ref().map(|name| name.key().clone()),
                requirements: pr.requirements.clone(),
                exact: pr.exact,
                prefer: crate::package_resolver::package_prefer_at(
                    analysis,
                    pr.range.start(),
                    crate::package_resolver::PackagePrefer::default(),
                ),
                uri: uri.to_owned(),
                name: pr.name.clone(),
                at: pr.range.start(),
                enclosing_body: analysis.innermost_definition_body_span(pr.range.start()),
                conditional: pr.conditional,
            });
        }
        for pp in &analysis.package_provides {
            self.package_provides.push(WorkspacePackageProvide {
                original_name: pp.original_name.as_ref().map(|name| name.key().clone()),
                uri: uri.to_owned(),
                name: pp.name.clone(),
                at: pp.range.start(),
                conditional: pp.conditional,
            });
        }
        for pi in &analysis.package_ifneededs {
            self.package_ifneededs.push(WorkspacePackageIfneeded {
                original_name: pi.original_name.as_ref().map(|name| name.key().clone()),
                uri: uri.to_owned(),
                name: pi.name.clone(),
            });
        }
        // Only the unconditional raises: a `package prefer latest` inside an
        // `if` / `catch` / `try` may not run, and the cross-document tier
        // abstains toward the interpreter default exactly as the
        // single-document one does.
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
        for exp in analysis.original_namespace_exports() {
            self.namespace_exports.push(WorkspaceNamespaceExport {
                original: Some(exp.clone()),
                unknown: false,
                uri: uri.to_owned(),
                ns: exp.context().reporting_label(),
                pattern: exp
                    .pattern()
                    .map_or_else(String::new, tcl_syntax::native_string::resident_name_label),
                at: exp.span().start(),
                enclosing_body: analysis.original_definition_body_span(exp.span().start()),
                clears: exp.clears(),
            });
        }
        for span in analysis.original_namespace_export_unknowns() {
            self.namespace_exports.push(WorkspaceNamespaceExport {
                original: None,
                unknown: true,
                uri: uri.to_owned(),
                ns: String::new(),
                pattern: String::new(),
                at: span.start(),
                enclosing_body: analysis.original_definition_body_span(span.start()),
                clears: false,
            });
        }
        if analysis.allows_lexical_declaration_advice() {
            for exp in &analysis.namespace_exports {
                self.namespace_exports.push(WorkspaceNamespaceExport {
                    original: None,
                    unknown: false,
                    uri: uri.to_owned(),
                    ns: exp.ns.clone(),
                    pattern: exp.pattern.clone(),
                    at: exp.range.start(),
                    enclosing_body: analysis.innermost_definition_body_span(exp.range.start()),
                    clears: exp.clears,
                });
            }
        }
        self.index_import_lifecycle(uri, analysis);
        self.index_command_links(uri, analysis);
    }

    /// The document's `source` rows, plus the raw path-constant candidates
    /// the edge resolver folds them with — recorded together because they
    /// are consumed together (a computed row without its constants is
    /// unresolvable, constants without rows are inert).
    fn index_sources(&mut self, uri: &str, analysis: &AnalysisResult) {
        for target in &analysis.source_targets {
            if let Some(original) = &target.original_interpreter_source_load
                && !original.matches_analysis(analysis)
            {
                continue;
            }
            self.sources.push(WorkspaceSource {
                original_path_expression:
                    tcl_compiler::auto_path_eval::capture_source_target_path_expression(
                        analysis, target,
                    ),
                original_interpreter_source_load: target.original_interpreter_source_load.clone(),
                uri: uri.to_owned(),
                raw_path: target.raw_path.clone(),
                site_namespace: target.site_namespace.clone(),
                range: target.range,
                is_literal: target.is_literal,
                enclosing_body: analysis.innermost_definition_body_span(target.range.start()),
            });
        }
        self.path_constant_assignments
            .clone_from(&analysis.path_constant_assignments);
    }

    /// Lift one document's class definitions, each with its members flattened
    /// into one source-ordered [`WorkspaceMethod`] list (instance methods
    /// first, then the class-side ones).
    ///
    /// Split out of [`Self::index_document`] only for size; the source-order
    /// rule that walk applies here unchanged.
    fn index_classes(&mut self, uri: &str, analysis: &AnalysisResult) {
        let mut classes: Vec<_> = analysis
            .all_classes
            .values()
            .map(|class_def| {
                let mut originals = analysis.original_class_declarations().filter(|original| {
                    !class_def.source_name_ambiguous.is_observed()
                        && class_def.source_name.as_ref() == Some(original.name())
                        && class_def.name_span == original.metadata().name_span
                });
                let original = originals.next().filter(|_| originals.next().is_none());
                (class_def, original)
            })
            .collect();
        for original in analysis.original_class_declarations() {
            if !classes.iter().any(|(_, candidate)| {
                candidate.is_some_and(|candidate| {
                    candidate.original_occurrence() == original.original_occurrence()
                })
            }) {
                classes.push((original.metadata(), Some(original)));
            }
        }
        classes
            .sort_by_key(|(class_def, _)| (class_def.name_span.start(), class_def.name_span.end()));
        for (class_def, original) in classes {
            let methods: Vec<WorkspaceMethod> =
                sorted_by_span(class_def.methods.values(), |m| m.name_span)
                    .into_iter()
                    .map(|m| WorkspaceMethod {
                        name: m.name.clone(),
                        kind: m.kind.clone(),
                        exported: m.visibility == PUBLIC,
                        private: m.visibility == PRIVATE,
                        is_self_method: m.is_self_method,
                        name_span: m.name_span,
                    })
                    .chain(
                        sorted_by_span(class_def.class_methods.values(), |m| m.name_span)
                            .into_iter()
                            .map(|m| WorkspaceMethod {
                                name: m.name.clone(),
                                kind: CLASS_METHOD.to_string(),
                                exported: m.visibility == PUBLIC,
                                private: m.visibility == PRIVATE,
                                is_self_method: m.is_self_method,
                                name_span: m.name_span,
                            }),
                    )
                    .collect();
            self.classes.push(WorkspaceClass {
                original_declaration: original
                    .map(|original| original.original_occurrence().clone()),
                original_members: original
                    .map(|original| original.metadata().original_members.clone()),
                original_relations: original
                    .map(|original| original.metadata().original_relations.clone()),
                source_name: class_def.source_name.clone(),
                source_name_ambiguous: class_def.source_name_ambiguous.is_observed(),
                relation_lookups: class_def.relation_lookups.clone(),
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
                bare_word_construction: class_def
                    .class_command_fallback
                    .constructs_named_instance(),
                constructor_spans: class_def.constructors.iter().map(|c| c.name_span).collect(),
            });
        }
    }

    /// Lift a document's `namespace forget` events and command **destructions**
    /// — the removal half of the import edge's lifecycle.
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
        // `sorted_by_span` exists: a consumer answering with the
        // first matching record must not answer differently run to run.
        let mut destroyed: Vec<(&String, &u32)> = analysis.destroyed_commands.iter().collect();
        destroyed.sort_unstable_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0)));
        for (name, at) in destroyed {
            let qualified_name = name.clone();
            self.command_deletions.push(WorkspaceCommandDeletion {
                uri: uri.to_owned(),
                qualified_name: qualified_name.clone(),
                at: *at,
            });
            self.command_retirements.push(WorkspaceCommandRetirement {
                uri: uri.to_owned(),
                qualified_name,
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
            aliased.insert(link.cell.to_owned());
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
                qualified_name: vref.qualified_name.clone(),
                span: vref.span,
            });
        }
    }

    /// Lift a document's namespace-name occurrences
    /// ([`tcl_compiler::analyser::AnalysisResult::namespace_refs`]) into the
    /// workspace table, so `namespace children ::tomato` in one file reaches
    /// the `namespace eval ::tomato { … }` in another.
    ///
    /// A straight copy: the analyser has already rooted relative spellings
    /// and dropped computed ones, so there is no second resolution rule here
    /// that could disagree with the in-document providers.
    fn index_namespace_refs(&mut self, uri: &str, analysis: &AnalysisResult) {
        for nref in &analysis.namespace_refs {
            self.namespace_refs.push(WorkspaceNamespaceRef {
                source: nref.clone(),
                uri: uri.to_owned(),
                qualified_name: nref.qualified_name.clone(),
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
        // `namespace import ::mod::helper` inside `::app` binds `::app::helper`
        // to the exporting `::mod::helper`.  A glob pattern names no single
        // command, so it introduces no fixed `WorkspaceCommandLink` — instead
        // it is indexed as a [`WorkspaceGlobImport`], consulted per-call by
        // [`Self::resolve_wildcard_import`] against whichever bare name the
        // invocation actually writes.
        for imp in &analysis.namespace_imports {
            let Some(source) = &imp.source else {
                continue;
            };
            if source.tail_pattern.contains(['*', '?', '[']) {
                self.glob_imports.push(WorkspaceGlobImport {
                    original_pattern: analysis
                        .original_namespace_patterns()
                        .find(|pattern| {
                            pattern.span() == imp.range
                                && pattern.purpose()
                                    == tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern
                        })
                        .cloned(),
                    native_source: source.native_source.clone(),
                    uri: uri.to_owned(),
                    ns: imp.ns.clone(),
                    source_ns: source.namespace.clone(),
                    tail_pattern: source.tail_pattern.clone(),
                    at: imp.range.start(),
                    enclosing_body: analysis.innermost_definition_body_span(imp.range.start()),
                    forced: imp.forced,
                });
                continue;
            }
            let source_ns = &source.namespace;
            let tail = &source.tail_pattern;
            // An exact import is export-gated exactly like a glob one — real
            // Tcl silently installs nothing when the name is not exported at
            // the import's own position (oracle on
            // `WorkspaceCommandLink::import_gate`). One shape stays ungated:
            // a *conjectured* import, which is inferred from a tcllib
            // `<NS>::import <ALIAS>` wrapper rather than read off a real
            // `namespace import` word, so there is no export declaration for
            // it to have passed and gating it would drop the idiom entirely.
            //
            // A pattern rooted at the global namespace (`namespace import
            // ::p`) is **not** one of them, though it reads as an empty
            // source namespace, which makes it easy to skip on that basis.
            // Real Tcl treats it like any other unexported import: a silent
            // no-op leaving `info commands ::dst::*` empty (oracle 8.6.14 /
            // 9.0.4). `::` is its source namespace, the same spelling every
            // global-level `namespace export` record already carries.
            let import_gate =
                (!imp.conjectured)
                    .then_some(source_ns)
                    .map(|source_ns| WorkspaceImportGate {
                        original_pattern: analysis
                            .original_namespace_patterns()
                            .find(|pattern| {
                                pattern.span() == imp.range
                                && pattern.purpose()
                                    == tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern
                            })
                            .cloned(),
                        native_source: source.native_source.clone(),
                        source_ns: global_rooted(source_ns).to_owned(),
                        name: tail.clone(),
                        at: imp.range.start(),
                        enclosing_body: analysis.innermost_definition_body_span(imp.range.start()),
                        forced: imp.forced,
                    });
            self.command_links.push(WorkspaceCommandLink {
                uri: uri.to_owned(),
                linked_qname: tcl_syntax::naming::qualify(&imp.ns, tail),
                linked_source_name: None,
                target_qname: source.constructed_pattern(),
                target_source_name: None,
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
            if alias.target.as_str().is_empty() {
                continue;
            }
            let nested = analysis
                .alias_offsets
                .get(&alias.qualified_name)
                .is_some_and(|&off| analysis.offset_is_inside_any_definition_body(off));
            let policy = analysis.resolved_profile().and_then(|profile| {
                tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy()
            });
            let Some(target) = alias.target.reported_global_key(policy) else {
                continue;
            };
            self.command_links.push(WorkspaceCommandLink {
                uri: uri.to_owned(),
                linked_qname: alias.qualified_name.clone(),
                linked_source_name: alias.source_name.clone(),
                target_qname: target.into_owned(),
                target_source_name: alias.target.selected_global_name(policy),
                target_span: None,
                nested,
                import_gate: None,
            });
        }
        // `rename OLD NEW` makes `NEW` run what `OLD` denoted.  The recorded
        // map is `NEW → OLD`, both already `::`-normalised.  `OLD`'s own
        // word is already a first-class command invocation — the ordinary
        // reference/rename path covers it — so, like
        // `interp alias`'s `TARGET` word above, it needs no `target_span`
        // here.
        for (new, old) in &analysis.renamed_commands {
            let Some(&at) = analysis.rename_offsets.get(new) else {
                continue;
            };
            let nested = analysis.offset_is_inside_any_definition_body(at);
            self.command_links.push(WorkspaceCommandLink {
                uri: uri.to_owned(),
                linked_qname: new.clone(),
                linked_source_name: None,
                target_qname: old.clone(),
                target_source_name: None,
                target_span: None,
                nested,
                import_gate: None,
            });
            // A rename written in a definition body runs only if that body is
            // called.  It introduces the same conditional link above but is
            // not evidence that the load-level name has gone away.
            if !nested {
                self.command_retirements.push(WorkspaceCommandRetirement {
                    uri: uri.to_owned(),
                    qualified_name: old.clone(),
                    at,
                });
            }
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
    /// Last index-wide document revision allocated. Tokens must not live on a
    /// recyclable slot: remove A, reuse its slot for B, then re-add A in a new
    /// slot would otherwise let A's old and new records share a token.
    document_revision_clock: u64,
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
    /// without link-following.  Unlike the other derived views this one is
    /// updated one document at a time after a body-only edit; see
    /// [`WorkspaceIndex::settled_sites`].
    settled_invocations: [IncrementalSettledTargets; 2],
    /// The whole-program export oracle the single-document tier borrows — see
    /// [`WorkspaceIndex::export_snapshot`].
    export_snapshot: Derived<NamespaceExportSnapshot>,
    /// The `source`-path → document-URI resolver, when the host has installed
    /// one — see [`WorkspaceIndex::set_source_resolver`].
    source_resolver: Option<SourceResolver>,
    /// The host's constant folder, installed alongside the resolver — folds
    /// one document's raw write facts (plus imports) into a `name → value`
    /// map, with the document's own filesystem path standing in for
    /// `[info script]`.
    constant_folder: Option<ConstantFolder>,
    /// The `source`-graph load order derived from it — see
    /// [`WorkspaceIndex::run_order`].
    run_order: Derived<crate::source_graph::RunOrder>,
    /// Per-document **imported** path constants — values source-graph
    /// ancestors establish before the document runs — see
    /// [`WorkspaceIndex::imported_constants`].
    imported_constants: Derived<HashMap<String, tcl_compiler::auto_path_eval::FoldedPathConstants>>,
}

/// The host's `source`-path → document-URI resolver: `(sourcing document's
/// URI, the path word as written, whether that word is a plain literal)` to
/// the sourced document's URI, or `None` for a path the host cannot place.
///
/// `WorkspaceIndex` holds no URI ↔ filesystem-path mapping of its own, so this
/// is how the `source`-graph load order gets its edges — see
/// [`WorkspaceIndex::set_source_resolver`].  The signature is
/// [`WorkspaceIndex::source_seed_map`]'s, so one host resolver serves both.
///
/// The third argument is the sealed original source expression. Missing or
/// foreign lineage declines; reported path text is not a selection input.
/// The fourth argument is the original source-site byte offset. The fifth
/// argument is the parent's retained path inventory; the sixth contains
/// position-gated source-ancestor constants. The host supplies only document
/// filenames and URI/path mapping, without reconstructing Tcl operations.
pub type SourceResolver = fn(
    &str,
    &str,
    Option<&tcl_compiler::auto_path_eval::OriginalSourcePathExpression>,
    u32,
    &tcl_compiler::auto_path_eval::PathConstantAssignments,
    &tcl_compiler::auto_path_eval::FoldedPathConstants,
) -> Option<String>;

/// The host's constant folder: `(document URI, raw write facts, imported
/// constants)` to the document's folded `name → value` view — the
/// [`tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports`]
/// fold, with the URI's filesystem path (host knowledge) standing in for
/// `[info script]`.  Installed with the resolver so the index can compute
/// what one document *provides* to the documents it sources.
pub type ConstantFolder = fn(
    &str,
    &tcl_compiler::auto_path_eval::PathConstantAssignments,
    &tcl_compiler::auto_path_eval::FoldedPathConstants,
) -> tcl_compiler::auto_path_eval::FoldedPathConstants;

/// A whole-index **derived view**: a value that is a pure function of the
/// index's contents, built at most once per [`WorkspaceIndex::generation`] and
/// dropped by the mutation hook that bumps it.
///
/// The workspace-indexing contract's rule 5 in one type, so the reset/build
/// boilerplate is written once rather than per view. Excluded
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

/// One document's contribution to the settled-target index: target name and
/// the invocation indices in that document which reach it.
///
/// [`WorkspaceIndex::invocations_of`] backs code-lens's per-proc / per-class
/// reference count, which calls it once per proc and once per class, so
/// re-settling *every* invocation in the workspace against the candidate
/// target on each call would cost an N-proc document an O(N × invocations)
/// walk, each rebuilding [`WildcardImportIndex`] (five `HashMap`s over the
/// whole index) from scratch. Settling every invocation once per generation
/// and grouping the results turns each `invocations_of` call into a hash
/// lookup plus a direct index into the owning document's slot — the indices
/// are stable because a document's `Vec<WorkspaceInvocation>` is only ever
/// cleared and refilled wholesale (`DocumentRecords::clear` /
/// `index_document`), never reordered in place, and any mutation that could
/// invalidate a stored `(slot, index)` pair also bumps the generation and
/// drops the [`Derived`] view holding it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SettledCommandTarget {
    Authored(String),
    OriginalSlot(
        tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
    ),
}

type DocumentSettledTargets = Vec<(SettledCommandTarget, usize)>;

/// The workspace-wide reverse index used by find-references and code lenses:
/// every invocation's settled target, grouped by that target's `::`-stripped
/// qualified name (`target -> [(doc slot, index within that slot's
/// invocations)]`).
///
/// A `BTreeSet` permits removal of the old contribution from one changed
/// document without scanning every caller of a popular command, while keeping
/// a stable document-slot / source-order answer.
type SettledTargets = std::collections::HashMap<SettledCommandTarget, BTreeSet<(usize, usize)>>;

/// Incremental storage for one of the direct or link-following settlement
/// modes.  It is deliberately separate from [`Derived`]: an edit in a proc
/// body changes that document's invocation rows but not the command tables
/// those rows resolve against, so throwing the complete reverse index away is
/// needless work.
#[derive(Debug, Default)]
struct IncrementalSettledTargets {
    state: std::sync::Mutex<IncrementalSettledState>,
}

#[derive(Debug, Default)]
struct IncrementalSettledState {
    /// `None` means this slot has never been settled (or has been removed).
    by_document: Vec<Option<DocumentSettledTargets>>,
    by_target: SettledTargets,
    /// A resolution-input change invalidates every document.  Otherwise this
    /// holds exactly the document slots whose own invocation rows changed.
    rebuild_all: bool,
    dirty_slots: BTreeSet<usize>,
    /// Test-visible accounting for the incremental firewall.
    settled_documents: usize,
}

impl Clone for IncrementalSettledTargets {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl IncrementalSettledTargets {
    fn invalidate_all(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.by_document.clear();
        state.by_target.clear();
        state.rebuild_all = true;
        state.dirty_slots.clear();
    }

    fn mark_document_dirty(&self, slot: usize) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !state.rebuild_all {
            state.dirty_slots.insert(slot);
        }
    }

    #[cfg(test)]
    fn settled_documents(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .settled_documents
    }
}

/// The owner-resolved class edges [`WorkspaceIndex::class_edges`] builds once
/// and [`Self::linearise`] answers any number of receivers from.
struct ClassEdges {
    supers_map: std::collections::HashMap<String, Vec<String>>,
    mixins_map: std::collections::HashMap<String, Vec<String>>,
}

impl ClassEdges {
    /// `class_q`'s full linearisation and its **mixin-free** twin — the
    /// `superclass` spine alone.
    ///
    /// The two are needed together because C's call-chain builder treats the
    /// paths differently: each mixin is entered with a *fresh copy* of the
    /// dispatch flags, so a mixin's `unexport` empties only its own branch
    /// while the same word on the spine decides the whole dispatch.  Both
    /// come from the one canonical
    /// [`tcl_syntax::mro::tcloo_linearise`] over the same resolved edges, so
    /// they cannot disagree about which qualified name a bare `superclass
    /// Device` meant.  Empty when the hierarchy is cyclic or too complex to
    /// linearise (the shared budget guard) — consumers abstain rather than
    /// guess.
    fn linearise(&self, class_q: &str) -> (Vec<String>, Vec<String>) {
        (
            tcl_syntax::mro::tcloo_linearise(class_q, &self.supers_map, &self.mixins_map)
                .unwrap_or_default(),
            self.spine(class_q),
        )
    }

    /// `class_q`'s mixin-free linearisation — the `superclass` spine alone.
    fn spine(&self, class_q: &str) -> Vec<String> {
        let no_mixins: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        tcl_syntax::mro::tcloo_linearise(class_q, &self.supers_map, &no_mixins).unwrap_or_default()
    }

    /// The spines `TclOO`'s call-chain builder walks for a receiver of class
    /// `class_q`, in the order it walks them: every reachable `mixin` branch,
    /// then the receiver's own spine. C enters each mixin with a fresh copy
    /// of the dispatch
    /// flags and so lets a branch's `export` / `unexport` govern that branch
    /// alone.
    fn branch_spines(&self, class_q: &str, linearisation: &[String]) -> Vec<Vec<String>> {
        let mut roots: Vec<String> = Vec::new();
        let mut queue: std::collections::VecDeque<String> =
            std::collections::VecDeque::from([class_q.to_owned()]);
        let mut seen: std::collections::HashSet<String> =
            std::collections::HashSet::from([class_q.to_owned()]);
        while let Some(root) = queue.pop_front() {
            for spine_q in self.spine(&root) {
                for mixin in self.mixins_map.get(&spine_q).into_iter().flatten() {
                    if seen.insert(mixin.clone()) {
                        roots.push(mixin.clone());
                        queue.push_back(mixin.clone());
                    }
                }
            }
        }
        // The linearisation is the order the chain builder reaches these
        // classes; a root it does not list sorts last but keeps its discovery
        // order.
        roots.sort_by_key(|root| {
            linearisation
                .iter()
                .position(|c| c == root)
                .unwrap_or(usize::MAX)
        });
        roots
            .into_iter()
            .map(|root| self.spine(&root))
            .chain(std::iter::once(self.spine(class_q)))
            .collect()
    }
}

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

    /// All genuine original namespace patterns, including opaque operands with
    /// no reporting import row. This retains source geometry, not live aliases.
    pub fn original_namespace_patterns(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::signature_scan::original_name::SourceNamespacePattern,
        ),
    > {
        self.docs.iter().flat_map(|doc| {
            doc.original_namespace_patterns
                .iter()
                .map(move |pattern| (doc.uri.as_str(), pattern))
        })
    }

    /// Source candidate export advice for a genuine import pattern and genuine
    /// source publication name. It grants no import completion or table entry.
    #[must_use]
    pub fn original_import_export_verdict(
        &self,
        pattern: &tcl_compiler::signature_scan::original_name::SourceNamespacePattern,
        name: &tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        import_uri: &str,
    ) -> ExportVerdict {
        if !self
            .original_namespace_patterns()
            .any(|(uri, retained)| uri == import_uri && retained == pattern)
            || pattern.matches_imported_command(name.slot(), name.policy()) != Some(true)
        {
            return ExportVerdict::Unknown;
        }
        let body = self
            .docs
            .iter()
            .find(|doc| doc.uri == import_uri)
            .and_then(|doc| {
                doc.original_procs
                    .iter()
                    .map(|record| record.metadata().body_span)
                    .chain(
                        doc.original_classes
                            .iter()
                            .map(|record| record.metadata().body_span),
                    )
                    .filter(|span| {
                        span.start() <= pattern.span().start()
                            && pattern.span().start() < span.end()
                    })
                    .min_by_key(|span| span.len())
            });
        crate::namespace_import::NamespaceExportOracle::exported_at_original(
            self.export_snapshot().as_ref(),
            name.slot(),
            name.policy(),
            RunPoint {
                uri: import_uri,
                at: pattern.span().start(),
                enclosing_body: body,
            },
        )
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

    /// Whether the definition named by `qualified_name` remains callable in
    /// its document's final command table.
    ///
    /// A document can define a name again after a `rename`/destruction, so
    /// retirement is compared with the declaration's own offset rather than
    /// being a workspace-wide tombstone.  Ordering across documents is not a
    /// static Tcl fact; a lifecycle record therefore only retires a
    /// definition from the same URI.
    fn definition_name_is_live(&self, uri: &str, qualified_name: &str, at: u32) -> bool {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.slots.get(uri).is_none_or(|&slot| {
            !self.docs[slot]
                .command_retirements
                .iter()
                .any(|retirement| {
                    unroot_rooted_key(&retirement.qualified_name)
                        .unwrap_or(&retirement.qualified_name)
                        == target
                        && retirement.at > at
                })
        })
    }

    /// [`tcl_compiler::analyser::AnalysisResult::innermost_definition_body_span`]
    /// for a whole column of offsets at once, in the order they were given.
    ///
    /// That single-offset lookup scans every recorded proc and class body, so
    /// calling it once per invocation is `O(procs × invocations)` — far too
    /// dear to run once per indexed call site.
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
            for p in self.live_procs() {
                tcl_compiler::analyser::utils::insert_qualified_and_tail(
                    &mut names,
                    &p.qualified_name,
                );
            }
            for c in self.live_classes() {
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
    /// importing namespace's own command.
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
                    .entry(unroot_rooted_key(&exp.ns).unwrap_or(&exp.ns).to_owned())
                    .or_default()
                    .push(exp.clone());
            }
            NamespaceExportSnapshot {
                original_exports: self.namespace_exports().cloned().collect(),
                original_declarations: self
                    .docs
                    .iter()
                    .flat_map(|doc| {
                        doc.original_procs
                            .iter()
                            .map(|record| record.name().clone())
                            .chain(
                                doc.original_classes
                                    .iter()
                                    .map(|record| record.name().clone()),
                            )
                    })
                    .collect(),
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

    /// Install the host's retained-source-expression → document-URI resolver, so
    /// the index can build the [`crate::source_graph::RunOrder`] the
    /// import-lifecycle gates rank cross-document events with.
    ///
    /// The index deliberately holds no URI ↔ filesystem-path mapping of its
    /// own — that is the host's knowledge, and every other `source`-graph
    /// consumer reads the same selected candidate edges
    /// ([`Self::source_ancestor_package_requires_from_resolved`],
    /// [`Self::source_ancestor_prefers_latest_from_resolved`], [`Self::source_seed_map`]).
    /// The order, though, is consulted *inside* the per-call import walk,
    /// where a per-call resolver argument would have to be threaded through
    /// every caller of `resolve_wildcard_import` and every derived view that
    /// builds a `WildcardImportIndex`. Holding it here instead lets the order
    /// be a [`Derived`] view like the rest, built at most once per
    /// [`Self::generation`].
    ///
    /// A plain `fn` pointer rather than a boxed closure: the resolver is a
    /// pure function of the explicit filename and retained source inputs, and
    /// a `fn` keeps the index `Debug + Clone + Default` with no manual impls.
    /// The signature is [`Self::source_seed_map`]'s, so one host resolver
    /// serves both — including its statically-foldable computed-path tier
    /// (`[file join [file dirname [info script]] x.tcl]`), which is as provable
    /// as a literal and is the idiom real multi-file projects actually write.
    /// A path the host cannot prove returns `None` and sequences nothing.
    ///
    /// **Without a resolver the order is empty**: every cross-document event
    /// is then unrankable and every same-document one ordered.
    pub fn set_source_resolver(&mut self, resolve: SourceResolver, fold: ConstantFolder) {
        self.source_resolver = Some(resolve);
        self.constant_folder = Some(fold);
        self.run_order = Derived::default();
        self.imported_constants = Derived::default();
        // The export snapshot captures the order, so it goes with it.
        self.export_snapshot = Derived::default();
    }

    /// Candidate source edges using the same genuine expression plans and
    /// retained inventories as import folding, namespace seeds and run order.
    /// These are conditional source routes, not completed file evaluation.
    #[must_use]
    pub fn resolved_source_edges(&self) -> Vec<crate::source_graph::RunEdge> {
        let imports = self.imported_constants();
        let empty = tcl_compiler::auto_path_eval::FoldedPathConstants::default();
        self.source_resolver
            .into_iter()
            .flat_map(|resolve| {
                self.sources()
                    .filter_map(|source| {
                        resolve(
                            &source.uri,
                            &source.raw_path,
                            source.original_path_expression.as_ref(),
                            source.range.start(),
                            self.path_constant_assignments(&source.uri),
                            imports.get(&source.uri).unwrap_or(&empty),
                        )
                        .map(|child| crate::source_graph::RunEdge {
                            parent: source.uri.clone(),
                            child,
                            at: source.range.start(),
                            enclosing_body: source.enclosing_body,
                            kind: crate::source_graph::RunEdgeKind::Source,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Package names available through the installed conditional source edge
    /// resolver. No raw reported path or dialect label selects an operation.
    #[must_use]
    pub fn source_ancestor_package_requires_from_resolved(&self, target_uri: &str) -> Vec<String> {
        let edges = self
            .resolved_source_edges()
            .into_iter()
            .map(|edge| (edge.parent, edge.child))
            .collect::<Vec<_>>();
        let mut requires = std::collections::HashMap::<String, Vec<String>>::new();
        for required in self.package_requires() {
            requires
                .entry(required.uri.clone())
                .or_default()
                .push(required.name.clone());
        }
        crate::source_graph::ancestor_requires(target_uri, &edges, &requires)
    }

    /// Source-order package preference advice over the installed, selected
    /// source edges, independently of actual interpreter package state.
    #[must_use]
    pub fn source_ancestor_prefers_latest_from_resolved(&self, target_uri: &str) -> bool {
        if self.package_prefers().next().is_none() {
            return false;
        }
        self.source_ancestor_prefers_latest_for_edges(target_uri, &self.resolved_source_edges())
    }

    fn source_ancestor_prefers_latest_for_edges(
        &self,
        target_uri: &str,
        edges: &[crate::source_graph::RunEdge],
    ) -> bool {
        let mut raises = std::collections::HashMap::<String, Vec<u32>>::new();
        for prefer in self.package_prefers() {
            raises
                .entry(prefer.uri.clone())
                .or_default()
                .push(prefer.at);
        }
        crate::source_graph::ancestor_prefer_latest_raised(target_uri, edges, &raises)
    }

    /// The load order over this workspace's documents, built at most once per
    /// [`Self::generation`] from the `source` graph
    /// ([`Self::set_source_resolver`]) **and** the `package require` graph
    /// ([`Self::package_run_edges`]).
    ///
    /// The `source` half is empty when no resolver is installed or when no
    /// `source` statement resolves: a target the resolver cannot place —
    /// `source $dir/x.tcl` with `$dir` unknown — names no document statically
    /// and sequences nothing.  The package half needs **no** resolver: a
    /// `package require NAME` names its provider through the index's own
    /// `package provide` records, so it works on a host that installs none.
    fn run_order(&self) -> Arc<crate::source_graph::RunOrder> {
        self.run_order.get_or_build(|| {
            let source_edges = self.resolved_source_edges().into_iter();
            let edges: Vec<crate::source_graph::RunEdge> =
                source_edges.chain(self.package_run_edges()).collect();
            crate::source_graph::RunOrder::build(&edges)
        })
    }

    /// Per-document **imported** path constants: the values a
    /// document's source-graph ancestors establish before it runs, agreed
    /// across every route that reaches it.
    ///
    /// Built as a joint fixpoint of edges and imports, in whole rounds so a
    /// shipped answer is always consistent with the shipped map:
    ///
    /// 1. Resolve **every** `source` row against the current imports (round
    ///    one: none) — no per-row caching across rounds, because a later
    ///    round can legitimately *shrink* an import (a newly resolved
    ///    in-edge whose parent does not supply a name drops that name from
    ///    the agreement), and a cached resolution from a richer map would
    ///    then be an answer the final map cannot reproduce.
    /// 2. From the resolved edges, compute what each parent **provides** to
    ///    each child: the parent's own writes *positioned before the
    ///    `source` statement* (a later assignment has not happened yet when
    ///    the child runs), folded by the host with the parent's own imports
    ///    — which precede the parent's whole execution, so they carry
    ///    unconditionally.
    /// 3. A child imports a name only when **every** in-edge supplies it
    ///    with an identical value — the agreement rule that carries OSVVM,
    ///    whose `StartUpShared.tcl` is sourced by eleven simulator scripts
    ///    each assigning `::osvvm::OsvvmScriptDirectory` the same directory.
    ///    One dissenting or silent route kills the name: whichever parent
    ///    actually ran, a kept value is the value.  Step 2-then-3 repeats to
    ///    its own fixpoint over the round's **fixed** edge set, where
    ///    provided maps only grow and agreement over a fixed edge set is
    ///    monotone.
    ///
    /// Rounds repeat until the map is stable (edges resolved under map M
    /// reproduce M), almost always round two.  A workspace that has not
    /// converged by the cap — constructible only from pathological
    /// import-unlocked cycles — ships **no** imports rather than a map its
    /// own edges disagree with.
    fn imported_constants(
        &self,
    ) -> Arc<HashMap<String, tcl_compiler::auto_path_eval::FoldedPathConstants>> {
        const ROUND_CAP: usize = 5;
        self.imported_constants.get_or_build(|| {
            let (Some(resolve), Some(fold)) = (self.source_resolver, self.constant_folder) else {
                return HashMap::new();
            };
            let empty = tcl_compiler::auto_path_eval::FoldedPathConstants::default();
            let rows: Vec<&WorkspaceSource> = self.sources().collect();
            let mut imports: HashMap<String, tcl_compiler::auto_path_eval::FoldedPathConstants> =
                HashMap::new();
            for _round in 0..ROUND_CAP {
                // Step 1: the round's edge set, from scratch.
                let edges: Vec<(&WorkspaceSource, String)> = rows
                    .iter()
                    .filter_map(|s| {
                        let child = resolve(
                            &s.uri,
                            &s.raw_path,
                            s.original_path_expression.as_ref(),
                            s.range.start(),
                            self.path_constant_assignments(&s.uri),
                            imports.get(&s.uri).unwrap_or(&empty),
                        )?;
                        // Self-sourcing provides nothing new.
                        (child != s.uri).then_some((*s, child))
                    })
                    .collect();
                // Steps 2 + 3 to their own fixpoint over this fixed edge set.
                let mut next: HashMap<String, tcl_compiler::auto_path_eval::FoldedPathConstants> =
                    HashMap::new();
                for _inner in 0..ROUND_CAP {
                    let mut candidates: HashMap<
                        &str,
                        Vec<tcl_compiler::auto_path_eval::FoldedPathConstants>,
                    > = HashMap::new();
                    for (s, child) in &edges {
                        // A `source` inside a proc or method body runs when
                        // that body is *called*, not at its lexical position
                        // — `set dir /a; proc load {} {source child.tcl};
                        // set dir /b; load` reaches the child with `/b`.
                        // The position gate below reads lexical order, so a
                        // body-local route instead contributes an **empty**
                        // candidate: it still reaches the child, and
                        // intersecting the agreement with an empty map
                        // correctly drops every name for it.
                        if s.enclosing_body.is_some() {
                            candidates.entry(child.as_str()).or_default().push(
                                tcl_compiler::auto_path_eval::FoldedPathConstants::empty(
                                    self.path_constant_assignments(&s.uri).naming_policy(),
                                ),
                            );
                            continue;
                        }
                        let before = self
                            .path_constant_assignments(&s.uri)
                            .before(s.range.start());
                        // From `next` only — never the previous round's map:
                        // the inner iteration is a fixpoint **from below**
                        // over this round's fixed edge set, and seeding it
                        // with values the current agreement may drop would
                        // break exactly the monotonicity that makes it
                        // terminate.
                        let parent_imports = next.get(&s.uri).unwrap_or(&empty);
                        candidates.entry(child.as_str()).or_default().push(fold(
                            &s.uri,
                            &before,
                            parent_imports,
                        ));
                    }
                    let stepped: HashMap<
                        String,
                        tcl_compiler::auto_path_eval::FoldedPathConstants,
                    > = candidates
                        .into_iter()
                        .map(|(child, maps)| {
                            (
                                child.to_owned(),
                                tcl_compiler::auto_path_eval::FoldedPathConstants::agreement(&maps),
                            )
                        })
                        .collect();
                    if stepped == next {
                        break;
                    }
                    next = stepped;
                }
                if next == imports {
                    return imports;
                }
                imports = next;
            }
            // Cap hit without convergence: conservative empty map.
            HashMap::new()
        })
    }

    /// The imported constants for one document — the map
    /// [`SourceResolver`] receives as its sixth argument, exposed so hosts
    /// can hand the very same view to their other per-document consumers
    /// (document links, a fresh analysis's own edges).
    #[must_use]
    pub fn imported_path_constants_for(
        &self,
        uri: &str,
    ) -> tcl_compiler::auto_path_eval::FoldedPathConstants {
        self.imported_constants()
            .get(uri)
            .cloned()
            .unwrap_or_default()
    }

    /// The `package require` half of the load order: one
    /// [`crate::source_graph::RunEdgeKind::PackageRequire`] edge per require
    /// site whose package this workspace provably provides (design §3.4).
    ///
    /// A `package require NAME` that returns has left `NAME` loaded, so the
    /// providing document's statements have all run — even though the require
    /// itself may have evaluated nothing.  That is a genuine ordering fact and
    /// the only one available on package-structured code, where the `source`
    /// statements that would sequence the same files are written
    /// `source [file join $dir x.tcl]` inside a `pkgIndex.tcl` and fold to
    /// nothing.
    ///
    /// # The soundness gates
    ///
    /// Each drops a require site rather than guessing, and each corresponds to
    /// one of the three uncertainties the design records:
    ///
    /// 1. **`auto_path` is mutable** (design §3.4 / uncertainty 1) — the
    ///    provider is identified by `package provide`, and the search path
    ///    that decides *which* copy of a package wins is a runtime value.  So
    ///    a package **two** indexed documents provide yields no edge at all:
    ///    with two directories on `auto_path`, their order alone decides which
    ///    file runs (oracle: tclsh 8.6.14 / 9.0.4, two directories each
    ///    holding a `mylib2 1.0`, `::lib::p` answers `from A` or `from B`
    ///    purely by `lappend auto_path` order — and only one of them exports).
    /// 2. **A package may already be loaded** (uncertainty 2) — handled by the
    ///    *edge kind*, not by dropping the edge: see
    ///    [`crate::source_graph::RunEdgeKind::PackageRequire`].
    /// 3. **`ifneeded` bodies are arbitrary** (uncertainty 3) — a package this
    ///    workspace registers a `package ifneeded` script for yields no edge.
    ///    The script is what actually runs, it runs later and in the global
    ///    namespace, and it may load something else entirely (oracle: one
    ///    `pkgIndex.tcl` whose body branches on the environment gives
    ///    `::c::p` → `real` or `stub`, with and without the provider's
    ///    `namespace export`, for the same `package require`).
    ///
    /// Plus the gates the `source` half already applies in spirit: a
    /// **conditional** require *or provide* may never run (the shim idiom
    /// [`tcl_compiler::analyser::types::PackageProvide::conditional`]
    /// documents); a **non-literal** package name
    /// (`package require $pkg`) names nothing statically; and a document that
    /// requires the package it itself provides is a self-edge, which
    /// [`crate::source_graph::RunOrder::build`] discards.
    fn package_run_edges(&self) -> Vec<crate::source_graph::RunEdge> {
        use std::collections::hash_map::Entry;
        // name -> the single indexed provider, or `None` once a second one
        // has been seen (gate 1).
        let mut provider: std::collections::HashMap<
            &tcl_registry::native_package::NativePackageNameKey,
            Option<&str>,
        > = std::collections::HashMap::new();
        if self
            .package_provides()
            .any(|provide| !provide.conditional && provide.original_name.is_none())
        {
            return Vec::new();
        }
        for pp in self.package_provides().filter(|pp| !pp.conditional) {
            let Some(name) = pp.original_name.as_ref() else {
                continue;
            };
            match provider.entry(name) {
                Entry::Vacant(slot) => {
                    slot.insert(Some(pp.uri.as_str()));
                }
                Entry::Occupied(mut slot) => {
                    if *slot.get() != Some(pp.uri.as_str()) {
                        slot.insert(None);
                    }
                }
            }
        }
        // Gate 3: a registered load script owns the package's loading.
        for pi in self.package_ifneededs() {
            if let Some(name) = pi.original_name.as_ref() {
                provider.insert(name, None);
            } else {
                return Vec::new();
            }
        }
        self.package_requires()
            .filter(|pr| !pr.conditional)
            .filter_map(|pr| {
                let child = (*provider.get(pr.original_name.as_ref()?)?)?;
                Some(crate::source_graph::RunEdge {
                    parent: pr.uri.clone(),
                    child: child.to_owned(),
                    at: pr.at,
                    enclosing_body: pr.enclosing_body,
                    kind: crate::source_graph::RunEdgeKind::PackageRequire,
                })
            })
            .collect()
    }

    /// Note a mutation, preserving the settled-target contributions which are
    /// still valid.  A command-resolution input change (a definition, link,
    /// import/export lifecycle event, source/package ordering edge, or
    /// dialect) can change any document's answer, so it deliberately requests
    /// a full re-settlement.  An invocation-only edit dirties only its own
    /// document contribution.
    fn invalidate(&mut self, resolution_inputs_changed: bool, changed_slot: usize) {
        self.generation = self.generation.wrapping_add(1);
        if resolution_inputs_changed {
            self.command_names = Derived::default();
            self.live_links = Derived::default();
            self.defined_names = <[Derived<HashSet<String>>; 2]>::default();
            self.command_link_map = Derived::default();
            self.export_snapshot = Derived::default();
            self.run_order = Derived::default();
            self.imported_constants = Derived::default();
            for settled in &self.settled_invocations {
                settled.invalidate_all();
            }
        } else {
            for settled in &self.settled_invocations {
                settled.mark_document_dirty(changed_slot);
            }
        }
    }

    fn import_source_matches(
        &self,
        report: &str,
        source: Option<&tcl_syntax::naming::NativeNamespacePatternSource>,
        name: &str,
    ) -> bool {
        use tcl_syntax::naming::{NativeNameProtocol, NativeNamespacePatternSource};
        let Some(source) = source else { return false };
        let declarations: Vec<_> = self
            .procs()
            .filter(|p| p.qualified_name == report)
            .map(|p| p.source_name.as_ref())
            .chain(
                self.classes()
                    .filter(|c| c.qualified_name == report)
                    .map(|c| {
                        if c.source_name_ambiguous {
                            None
                        } else {
                            c.source_name.as_ref()
                        }
                    }),
            )
            .collect();
        !declarations.is_empty()
            && declarations.into_iter().all(|receipt| {
                receipt.is_some_and(|receipt| {
                    let slot = receipt.slot();
                    match (source, receipt.policy().recipe()) {
                        (NativeNamespacePatternSource::C(namespace), NativeNameProtocol::C(_)) => {
                            &slot.namespace == namespace
                                && slot.simple.as_bytes() == name.as_bytes()
                        }
                        (
                            NativeNamespacePatternSource::Jim(namespace),
                            NativeNameProtocol::Jim084,
                        ) => {
                            let recipe = receipt.policy().recipe();
                            recipe.namespace_qualifier_bytes(slot.simple.as_bytes())
                                == namespace.as_bytes()
                                && recipe.namespace_tail_bytes(slot.simple.as_bytes())
                                    == name.as_bytes()
                        }
                        _ => false,
                    }
                })
            })
    }

    /// The command name-links that are actually installed — every `interp
    /// alias` / `rename` link, plus each exact `namespace import` link whose
    /// [`WorkspaceCommandLink::import_gate`] its source namespace's export
    /// timeline admits.
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
                        if self.defines_command(&l.target_qname)
                            && !self.import_source_matches(
                                &l.target_qname,
                                g.native_source.as_ref(),
                                &g.name,
                            )
                        {
                            return false;
                        }
                        if !observable
                            .contains(unroot_rooted_key(&g.source_ns).unwrap_or(&g.source_ns))
                        {
                            return true;
                        }
                        if !wci.exports_name_at(&g.source_ns, &g.name, g.site(&l.uri)) {
                            return false;
                        }
                        // A non-`-force` import onto a name the target
                        // namespace already holds installs nothing at all
                        // (oracle on `WorkspaceGlobImport::forced`), so
                        // the link it would introduce is not live either.
                        // "Already holds" is a real *definition*, a command
                        // a fresh interpreter of this document's dialect
                        // already has (the exact-import twin of
                        // `GlobImportRow::declares_builtin_at`), or an
                        // earlier live alias from a **different** source
                        // in the same document;
                        // a same-source re-import is a silent no-op, and
                        // two links cancelling each other out is what the
                        // different-source test rules out.
                        let importing_ns = importing_namespace_of(&l.linked_qname);
                        if !g.forced
                            && (self.defines_command(&l.linked_qname)
                                || self
                                    .registry_for(&l.uri)
                                    .is_some_and(|r| r.declares_command_at(&l.linked_qname))
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
                        // command.
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

    /// The registry a decision about `uri`'s content must be made against —
    /// the one for the dialect that document was analysed under.
    ///
    /// `None` for a document the index does not hold, or one whose record
    /// carries no dialect (a default-constructed [`AnalysisResult`], which
    /// only unit tests produce).  Callers abstain rather than guessing a
    /// dialect: answering an iRule's question out of the plain-Tcl command
    /// table would be worse than not answering it.
    fn registry_for(&self, uri: &str) -> Option<&'static tcl_registry::CommandRegistry> {
        let slot = *self.slots.get(uri)?;
        let dialect = self.docs[slot].dialect.as_str();
        (!dialect.is_empty()).then(|| crate::registry_for_dialect(dialect))
    }

    /// Whether the workspace holds a real proc or class definition at
    /// `qualified_name` — the "already exists" side of a non-`-force`
    /// `namespace import` conflict.
    ///
    /// Deliberately *not* [`Self::workspace_command_exists`], which also
    /// admits linked names: a link is what this question is being asked
    /// about, so counting one would make an import conflict with itself.
    ///
    /// Answered from the link-free reading of [`Self::defined_command_names`]
    /// — which is that same proc + class name set, already derived once per
    /// generation — rather than by re-walking every indexed proc and class.
    /// This sits inside `import_hop`'s per-call filter chain, where a re-walk
    /// would cost O(procs) *per call site per in-scope import*.
    fn defines_command(&self, qualified_name: &str) -> bool {
        self.defined_command_names(false)
            .contains(unroot_rooted_key(qualified_name).unwrap_or(qualified_name))
    }

    /// The `::`-stripped namespaces the workspace can say anything about: one
    /// that owns an indexed proc or class, that declares a `namespace export`
    /// somewhere, or that an indexed `namespace eval` block declares.
    ///
    /// The discriminator between "this namespace does not export the name"
    /// (a fact) and "this namespace is not in the workspace at all" (no
    /// information) — see [`Self::live_command_links`].
    ///
    /// The declaring-block source closes a hole the first two leave: a
    /// `namespace eval ::ns { namespace import ::other::* }` block that
    /// declares no proc, class, or export of its own *is* a namespace the
    /// workspace can see, and treating it as unknown would make the import
    /// gate abstain where it has the evidence to decide.
    fn observable_namespaces(&self) -> HashSet<&str> {
        fn owning_ns(qualified: &str) -> &str {
            {
                let (holder, _) = key_holder_and_tail(qualified);
                unroot_rooted_key(holder).unwrap_or(holder)
            }
        }
        self.procs()
            .map(|p| owning_ns(&p.qualified_name))
            .chain(self.classes().map(|c| owning_ns(&c.qualified_name)))
            .chain(
                self.namespace_exports()
                    .map(|e| unroot_rooted_key(&e.ns).unwrap_or(&e.ns)),
            )
            .chain(
                self.namespace_refs()
                    .filter(|n| n.declares)
                    .map(|n| unroot_rooted_key(&n.qualified_name).unwrap_or(&n.qualified_name)),
            )
            .collect()
    }

    /// Add (or refresh) one document's records.
    ///
    /// Call [`Self::remove_document`] first when re-indexing a changed document
    /// to avoid stale duplicates.  Adding the **same** URI twice without an
    /// intervening removal deliberately accumulates: the source-rehoming
    /// pass indexes one analysis per source-site namespace, and those views are
    /// several runtime identities of one physical file, not a replacement of
    /// each other.
    pub fn add_document(&mut self, uri: &str, analysis: &AnalysisResult) {
        let revision = self.allocate_document_revision();
        let slot = self.slot_for(uri);
        // `add_document` intentionally accumulates multiple source-site views
        // for one URI.  That is a resolution-input change unless this is an
        // empty analysis, so favour the conservative full invalidation here;
        // editor replacements use [`Self::replace_document`] below.
        self.docs[slot].index_document(uri, analysis);
        self.docs[slot].revision = revision;
        self.invalidate(true, slot);
    }

    /// Replace one document's records as one atomic incremental update.
    ///
    /// The server publishes ordinary edits through this entry point rather
    /// than spelling them as `remove_document` then `add_document`: doing the
    /// latter creates a transient missing-definition state and loses the fact
    /// that a body-only edit left command-resolution inputs untouched.
    pub fn replace_document(&mut self, uri: &str, analysis: &AnalysisResult) {
        self.replace_document_with_revision(uri, analysis);
    }

    /// Replace one document and return the revision assigned to its records.
    ///
    /// A publisher that releases the index before its final source-currency
    /// check can use this token to roll back only its own obsolete replacement,
    /// without deleting a newer writer's records for the same URI.
    pub fn replace_document_with_revision(&mut self, uri: &str, analysis: &AnalysisResult) -> u64 {
        let revision = self.allocate_document_revision();
        let slot = self.slot_for(uri);
        let old = self.docs[slot].settlement_dependencies();
        self.docs[slot].clear();
        self.docs[slot].index_document(uri, analysis);
        self.docs[slot].revision = revision;
        let changed = old != self.docs[slot].settlement_dependencies();
        self.invalidate(changed, slot);
        revision
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

    /// Allocate a token which no earlier document revision in this index has
    /// used, independently of slot ownership or reuse.
    fn allocate_document_revision(&mut self) -> u64 {
        self.document_revision_clock = self
            .document_revision_clock
            .checked_add(1)
            .expect("workspace document revision counter exhausted");
        self.document_revision_clock
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

    /// Retained document source/configuration for diagnostic dependency checks.
    /// Missing context is unknown; report names cannot recreate it.
    #[must_use]
    pub fn diagnostic_source_context(
        &self,
        uri: &str,
    ) -> Option<&WorkspaceDiagnosticSourceContext> {
        self.docs
            .get(*self.slots.get(uri)?)?
            .diagnostic_source
            .as_ref()
    }

    /// Whether every retained provider independently selected lexical advice.
    /// This compatibility eligibility supplies no lookup or rename permission;
    /// callers still require complete current source and an independent edit plan.
    #[must_use]
    pub fn allows_lexical_rename_advice(&self) -> bool {
        !self.slots.is_empty()
            && self.slots.iter().all(|(uri, &slot)| {
                self.docs.get(slot).is_some_and(|document| {
                    document.uri == *uri
                        && document
                            .diagnostic_source
                            .as_ref()
                            .is_some_and(|context| !context.uses_original_names())
                })
            })
    }

    /// Revision token for `uri`'s current records.
    ///
    /// Unlike [`Self::generation`], a mutation of an unrelated document does
    /// not change this value. A consumer can therefore load `uri`'s source
    /// outside the index lock and revalidate that the byte spans it captured
    /// still describe the same indexed revision.
    #[must_use]
    pub fn document_revision(&self, uri: &str) -> Option<u64> {
        self.slots.get(uri).map(|&slot| self.docs[slot].revision)
    }

    /// Drop every entry that came from `uri` (used before
    /// re-indexing a changed document, or on `did_close`).
    ///
    /// Costs that document's own records, not the workspace's: its slot is
    /// cleared and handed back to the free list.
    pub fn remove_document(&mut self, uri: &str) {
        if let Some(slot) = self.slots.remove(uri) {
            self.docs[slot].clear();
            self.free_slots.push(slot);
            self.invalidate(true, slot);
        }
    }

    /// Remove `uri` only when its records still have `revision`.
    ///
    /// This is the compare-and-remove counterpart to
    /// [`Self::replace_document_with_revision`].
    pub fn remove_document_if_revision(&mut self, uri: &str, revision: u64) -> bool {
        if self.document_revision(uri) != Some(revision) {
            return false;
        }
        self.remove_document(uri);
        true
    }

    /// Every indexed `source FILE` reference.
    pub fn sources(&self) -> impl Iterator<Item = &WorkspaceSource> {
        self.docs.iter().flat_map(|doc| doc.sources.iter())
    }

    /// `uri`'s raw single-assignment path-constant candidates
    /// ([`AnalysisResult::path_constant_assignments`], carried per document);
    /// empty for an unindexed URI.  The source-edge resolver chain-folds
    /// these with the parent's own path before resolving a computed `source`
    /// argument.
    #[must_use]
    pub fn path_constant_assignments(
        &self,
        uri: &str,
    ) -> &tcl_compiler::auto_path_eval::PathConstantAssignments {
        self.slots.get(uri).map_or(
            tcl_compiler::auto_path_eval::PathConstantAssignments::unknown(),
            |&slot| &self.docs[slot].path_constant_assignments,
        )
    }

    /// Every indexed `package require NAME` declaration.
    pub fn package_requires(&self) -> impl Iterator<Item = &WorkspacePackageRequire> {
        self.docs.iter().flat_map(|doc| doc.package_requires.iter())
    }

    /// Every indexed `package provide NAME` declaration.
    pub fn package_provides(&self) -> impl Iterator<Item = &WorkspacePackageProvide> {
        self.docs.iter().flat_map(|doc| doc.package_provides.iter())
    }

    /// Every indexed `package ifneeded NAME VERSION SCRIPT` registration.
    pub fn package_ifneededs(&self) -> impl Iterator<Item = &WorkspacePackageIfneeded> {
        self.docs
            .iter()
            .flat_map(|doc| doc.package_ifneededs.iter())
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

    /// Original package keys retained in a document's source inventory.
    /// This metadata grants no completed require or package loading.
    #[must_use]
    pub fn original_package_requires_for(
        &self,
        uri: &str,
    ) -> Vec<tcl_registry::native_package::NativePackageNameKey> {
        let mut keys = Vec::new();
        for required in self
            .package_requires()
            .filter(|required| required.uri == uri)
        {
            if let Some(key) = required.original_name.as_ref()
                && !keys.contains(key)
            {
                keys.push(key.clone());
            }
        }
        keys
    }

    /// Complete requirement advice retaining independently supplied source
    /// identity, constraints and preference. This proves no loader executed.
    #[must_use]
    pub fn original_package_requirement_advice_for(
        &self,
        uri: &str,
        default: crate::package_resolver::PackagePrefer,
    ) -> Vec<crate::package_resolver::PackageRequirementAdvice> {
        self.package_requires()
            .filter(|required| required.uri == uri)
            .map(|required| required.original_advice(default))
            .collect()
    }

    /// Advisory source ancestors preserving their original package keys.
    /// Graph identity uses document URIs; Tcl name geometry stays in the key.
    #[must_use]
    pub fn source_ancestor_original_package_requires(
        &self,
        target_uri: &str,
        resolve: impl Fn(&str, &str) -> Option<String>,
    ) -> Vec<tcl_registry::native_package::NativePackageNameKey> {
        let edges = self
            .sources()
            .filter(|source| source.is_literal)
            .filter_map(|source| {
                resolve(&source.uri, &source.raw_path).map(|child| (source.uri.clone(), child))
            })
            .collect::<Vec<_>>();
        let mut requirements: std::collections::HashMap<String, Vec<_>> =
            std::collections::HashMap::new();
        for required in self.package_requires() {
            if let Some(key) = required.original_name.as_ref() {
                requirements
                    .entry(required.uri.clone())
                    .or_default()
                    .push(key.clone());
            }
        }
        crate::source_graph::ancestor_requirements(target_uri, &edges, &requirements)
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
    /// (transitively) `source`s it raised it first.
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
        let edges: Vec<crate::source_graph::RunEdge> = self
            .sources()
            .filter(|s| s.is_literal)
            .filter_map(|s| {
                resolve(&s.uri, &s.raw_path).map(|child| crate::source_graph::RunEdge {
                    parent: s.uri.clone(),
                    child,
                    at: s.range.start(),
                    enclosing_body: s.enclosing_body,
                    kind: crate::source_graph::RunEdgeKind::Source,
                })
            })
            .collect();
        self.source_ancestor_prefers_latest_for_edges(target_uri, &edges)
    }

    /// The **source-site namespace seeds** per sourced document: for
    /// every `source` statement `resolve` can place (the closure maps
    /// `(parent-uri, reported-path, original-expression)` to the child's URI — handling
    /// relative literals and statically-foldable computed
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
        resolve: impl Fn(
            &str,
            &str,
            Option<&tcl_compiler::auto_path_eval::OriginalSourcePathExpression>,
            u32,
            &tcl_compiler::auto_path_eval::PathConstantAssignments,
            &tcl_compiler::auto_path_eval::FoldedPathConstants,
        ) -> Option<String>,
    ) -> std::collections::HashMap<String, std::collections::BTreeSet<String>> {
        let imports = self.imported_constants();
        let empty = tcl_compiler::auto_path_eval::FoldedPathConstants::default();
        let mut out: std::collections::HashMap<String, std::collections::BTreeSet<String>> =
            std::collections::HashMap::new();
        for src in self.sources() {
            let Some(child) = resolve(
                &src.uri,
                &src.raw_path,
                src.original_path_expression.as_ref(),
                src.range.start(),
                self.path_constant_assignments(&src.uri),
                imports.get(&src.uri).unwrap_or(&empty),
            ) else {
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
    /// that lets the server skip the re-homing pass entirely for
    /// workspaces that never `source`.
    #[must_use]
    pub fn has_source_edges(&self) -> bool {
        self.sources().next().is_some()
    }

    /// The workspace's symbols matching `query`, at most `limit` of them —
    /// the whole `workspace/symbol` answer.
    ///
    /// Every indexed document is searched, not only the ones the editor has
    /// open, so a symbol in a file the user has never opened is reachable from
    /// the picker.  The index is refreshed on each document's diagnostics
    /// publish (~50 ms after an edit), which is the freshness a symbol picker
    /// gets: a name typed a moment ago appears once that publish lands.  It is
    /// the same staleness every other cross-document feature answers with, and
    /// the alternative — re-analysing every open buffer per keystroke in the
    /// Ctrl+T box — costs far more than the freshness is worth.
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
        self.symbols_matching_excluding(query, limit, std::iter::empty::<&str>())
    }

    /// The workspace's symbols matching `query`, excluding every document URI
    /// in `excluded_uris` before applying `limit`.
    ///
    /// Filtering at the document scan keeps the result bounded without letting
    /// excluded documents consume the caller's capacity. The server uses this
    /// while a watched deletion has marked an open buffer orphaned but its
    /// atomic index-removal transaction is still waiting.
    #[must_use]
    pub fn symbols_matching_excluding<'a>(
        &self,
        query: &str,
        limit: usize,
        excluded_uris: impl IntoIterator<Item = &'a str>,
    ) -> Vec<IndexedWorkspaceSymbol> {
        let lower_query = query.to_lowercase();
        let mut out: Vec<IndexedWorkspaceSymbol> = Vec::new();
        let excluded_slots: HashSet<usize> = excluded_uris
            .into_iter()
            .filter_map(|uri| self.slots.get(uri).copied())
            .collect();
        // Built once for the whole scan — the class-member walk needs the
        // workspace's cross-document retractions, which no single document's
        // records can answer for themselves.
        let retractions = self.retraction_index_excluding(&excluded_slots);
        for (slot, doc) in self.docs.iter().enumerate() {
            if out.len() >= limit {
                break;
            }
            if excluded_slots.contains(&slot) {
                continue;
            }
            doc.collect_symbols_matching(&lower_query, limit, &retractions, &mut out);
        }
        out
    }

    /// Every indexed proc.
    pub fn procs(&self) -> impl Iterator<Item = &WorkspaceProc> {
        self.docs.iter().flat_map(|doc| doc.procs.iter())
    }

    /// Original source declarations without presentation-key collapse.
    /// These records grant declaration assistance. Positioned references and
    /// command lifetime must be supplied independently by the source owner.
    pub fn original_procedure_declarations(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
                tcl_compiler::analyser::ProcDef,
            >,
        ),
    > {
        self.docs.iter().flat_map(|doc| {
            doc.original_procs
                .iter()
                .map(|declaration| (doc.uri.as_str(), declaration))
        })
    }

    /// Current readonly procedure source suggestions. Actual completed publications
    /// and conditional authored publications retain independent owners; unrepresented
    /// original declarations remain source cards. No loading, dispatch or edit follows.
    pub fn original_procedure_source_candidates(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
                tcl_compiler::analyser::ProcDef,
            >,
            &tcl_core_types::ByteCommandSlot,
            tcl_syntax::naming::NamePolicyProtocol,
        ),
    > {
        // naming.source.original-procedure-publications
        // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
        self.docs.iter().flat_map(|document| {
            document
                .original_procedure_source_candidates()
                .into_iter()
                .map(move |(declaration, slot, policy)| {
                    (document.uri.as_str(), declaration, slot, policy)
                })
        })
    }

    /// Current readonly class suggestions with independently selected slots.
    /// Canonical declaration identity remains separate from a moved source slot;
    /// neither conditional nor completed source inventories prove workspace loading.
    pub fn original_class_source_candidates(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
                tcl_compiler::analyser::ClassDef,
            >,
            &tcl_core_types::ByteCommandSlot,
            tcl_syntax::naming::NamePolicyProtocol,
        ),
    > {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        self.docs.iter().flat_map(|document| {
            document.original_class_source_candidates().into_iter().map(
                move |(declaration, slot, policy, _)| {
                    (document.uri.as_str(), declaration, slot, policy)
                },
            )
        })
    }

    /// Current retained hosted source headers and their independent owner context.
    /// This readonly inventory grants no native recipe, command publication or lookup.
    pub fn original_vendor_declarations(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &WorkspaceDiagnosticSourceContext,
            &crate::vendor_declaration::RetainedOriginalVendorDeclaration,
        ),
    > {
        self.docs.iter().flat_map(|document| {
            document
                .original_vendor_declarations
                .iter()
                .filter_map(move |record| {
                    let context = document.diagnostic_source.as_ref()?;
                    let declaration = record.declaration();
                    (context.uses_original_names()
                        && declaration
                            .input()
                            .matches_source(context.image(), context.config()))
                    .then_some((document.uri.as_str(), context, record))
                })
        })
    }

    /// Original object declarations, independent of optional displayed names.
    pub fn original_class_declarations(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
                tcl_compiler::analyser::ClassDef,
            >,
        ),
    > {
        self.docs.iter().flat_map(|doc| {
            doc.original_classes
                .iter()
                .map(|declaration| (doc.uri.as_str(), declaration))
        })
    }

    /// Publications surviving an independently completed modeled source root.
    /// Document ownership is retained; no workspace load order or physical
    /// interpreter existence follows from this source inventory.
    pub fn original_command_publications(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::command_binding::OriginalCommandPublication,
        ),
    > {
        self.docs.iter().flat_map(|doc| {
            doc.original_command_world.iter().flat_map(move |world| {
                world
                    .declarations()
                    .map(move |publication| (doc.uri.as_str(), publication))
            })
        })
    }

    /// Select source publications through the call's original ordered paths.
    /// Unknown or disagreeing alternatives remain unknown. An empty selection
    /// concerns only this inventory, not absence from a running interpreter.
    #[must_use]
    pub fn original_command_candidates(
        &self,
        lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
    ) -> Option<
        Vec<(
            &str,
            &tcl_compiler::command_binding::OriginalCommandPublication,
        )>,
    > {
        lookup.matching_slot_publications(self.original_command_publications().map(
            |(uri, publication)| (publication.slot(), publication.policy(), (uri, publication)),
        ))
    }

    /// Procedure declaration candidates using the shared publication matcher.
    /// This provides workspace source assistance, without proving loading,
    /// normal completion, current command lifetime or a writable reference.
    pub fn original_procedure_candidates(
        &self,
        lookup: &tcl_compiler::signature_scan::scope::SignatureSourceLookup,
    ) -> Vec<(
        &str,
        &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ProcDef,
        >,
    )> {
        let Some(candidates) = lookup.candidates() else {
            return Vec::new();
        };
        tcl_compiler::signature_scan::scope::first_matching_byte_slots(
            lookup.policy(),
            &candidates,
            self.original_procedure_source_candidates()
                .map(|(uri, declaration, slot, policy)| (slot, policy, (uri, declaration))),
        )
    }

    /// Object declaration candidates under the same shared lookup purpose.
    pub fn original_class_candidates(
        &self,
        lookup: &tcl_compiler::signature_scan::scope::SignatureSourceLookup,
    ) -> Vec<(
        &str,
        &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ClassDef,
        >,
    )> {
        let Some(candidates) = lookup.candidates() else {
            return Vec::new();
        };
        tcl_compiler::signature_scan::scope::first_matching_byte_slots(
            lookup.policy(),
            &candidates,
            self.original_class_source_candidates()
                .map(|(uri, declaration, slot, policy)| (slot, policy, (uri, declaration))),
        )
    }

    /// Every proc whose name was not later retired at load level in its own
    /// document.  Cross-document command consumers should use this view;
    /// [`Self::procs`] remains the source-record view for declaration tools.
    pub fn live_procs(&self) -> impl Iterator<Item = &WorkspaceProc> {
        self.procs().filter(|proc_def| {
            self.definition_name_is_live(
                &proc_def.uri,
                &proc_def.qualified_name,
                proc_def.name_span.start(),
            )
        })
    }

    /// Live procs contributed by one document, without walking every other
    /// document's definition table.
    #[must_use]
    pub fn live_procs_in(&self, uri: &str) -> Vec<&WorkspaceProc> {
        let Some(&slot) = self.slots.get(uri) else {
            return Vec::new();
        };
        self.docs[slot]
            .procs
            .iter()
            .filter(|proc_def| {
                self.definition_name_is_live(
                    &proc_def.uri,
                    &proc_def.qualified_name,
                    proc_def.name_span.start(),
                )
            })
            .collect()
    }

    /// Every indexed class.
    pub fn classes(&self) -> impl Iterator<Item = &WorkspaceClass> {
        self.docs.iter().flat_map(|doc| doc.classes.iter())
    }

    /// Every class whose command name was not later retired at load level in
    /// its own document.  `TclOO`'s class command is a command like any other:
    /// `rename Dog {}` makes `Dog new` unknown, not an object construction.
    pub fn live_classes(&self) -> impl Iterator<Item = &WorkspaceClass> {
        self.classes().filter(|class_def| {
            self.definition_name_is_live(
                &class_def.uri,
                &class_def.qualified_name,
                class_def.name_span.start(),
            )
        })
    }

    /// Live classes contributed by one document, without walking every other
    /// document's definition table.
    #[must_use]
    pub fn live_classes_in(&self, uri: &str) -> Vec<&WorkspaceClass> {
        let Some(&slot) = self.slots.get(uri) else {
            return Vec::new();
        };
        self.docs[slot]
            .classes
            .iter()
            .filter(|class_def| {
                self.definition_name_is_live(
                    &class_def.uri,
                    &class_def.qualified_name,
                    class_def.name_span.start(),
                )
            })
            .collect()
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
        let q = root_unrooted_key(unroot_rooted_key(name).unwrap_or(name));
        self.classes()
            .filter(|c| c.qualified_name == name || c.qualified_name == q)
            .collect()
    }

    fn resolve_class_relation(&self, owner: &WorkspaceClass, written: &str) -> Option<String> {
        let lookup = owner.relation_lookups.get(written)?.as_ref()?;
        for candidate in lookup.candidates()? {
            let mut reports = self
                .classes()
                .filter(|c| {
                    !c.source_name_ambiguous
                        && c.source_name.as_ref().is_some_and(|source| {
                            source.policy() == lookup.policy() && source.slot() == &candidate
                        })
                })
                .map(|c| c.qualified_name.as_str());
            let Some(report) = reports.next() else {
                continue;
            };
            if reports.any(|other| other != report) {
                return None;
            }
            if self
                .classes()
                .filter(|c| c.qualified_name == report)
                .any(|c| {
                    c.source_name_ambiguous
                        || c.source_name.as_ref().is_none_or(|source| {
                            source.policy() != lookup.policy() || source.slot() != &candidate
                        })
                })
            {
                return None;
            }
            return Some(report.to_owned());
        }
        None
    }

    /// The owner-aware direct parents (superclasses + mixins) of `qname`,
    /// unioned across **every** indexed definition of the class.  A cross-file
    /// `oo::define ::C { ... }` records a second `::C` entry that names no
    /// `superclass`; unioning here keeps the real class's parent edges from
    /// being hidden when such a stub happens to be the first match (without
    /// the union the parent walk picks an arbitrary duplicate and silently
    /// drops the hierarchy).
    fn resolved_parents_of(&self, qname: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for c in self.classes().filter(|c| c.qualified_name == qname) {
            for s in c.superclasses.iter().chain(c.mixins.iter()) {
                if let Some(p) = self.resolve_class_relation(c, s)
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
        let mut out: Vec<&WorkspaceClass> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for name in wc.superclasses.iter().chain(wc.mixins.iter()) {
            let Some(q) = self.resolve_class_relation(wc, name) else {
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
        self.classes()
            .filter(|c| {
                c.superclasses
                    .iter()
                    .chain(c.mixins.iter())
                    .any(|s| self.resolve_class_relation(c, s).as_deref() == Some(class_qname))
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
        self.class_linearisation_and_spine(class_q).0
    }

    /// [`Self::class_linearisation`] and its **mixin-free** twin — the
    /// `superclass` spine alone — from one edge-map build.
    ///
    /// The two are needed together because C's call-chain builder treats the
    /// paths differently: each mixin is entered with a *fresh copy* of the
    /// dispatch flags, so a mixin's `unexport` empties only its own branch
    /// while the same word on the spine decides the whole dispatch.  Resolving
    /// the edge maps is the expensive half and is O(classes) on its own, so it
    /// is deliberately not paid twice — see
    /// [`Self::subclass_provided_methods`] for what per-call rebuilding costs
    /// the diagnostics worker.
    fn class_linearisation_and_spine(&self, class_q: &str) -> (Vec<String>, Vec<String>) {
        self.class_edges().linearise(class_q)
    }

    /// The owner-resolved `superclass` / `mixin` edge maps over every indexed
    /// class — the expensive half of [`Self::class_linearisation_and_spine`],
    /// built once so a caller with many receivers to linearise does not pay
    /// O(classes) name resolution per receiver (see
    /// [`Self::subclass_provided_methods`] for what per-call rebuilding costs
    /// the diagnostics worker).
    fn class_edges(&self) -> ClassEdges {
        // Build the resolved edge maps over the classes reachable from
        // `class_q` (bounded: every indexed class at worst).
        let mut supers_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut mixins_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for c in self.classes() {
            let owner = c.qualified_name.as_str();
            let resolve = |name: &str| self.resolve_class_relation(c, name);
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
        ClassEdges {
            supers_map,
            mixins_map,
        }
    }

    /// The project-wide **subclass-provided method** view behind the
    /// template-method W308 abstention: for every class, the
    /// instance-side member names dispatchable on some **other** class whose
    /// linearisation contains it, so a base's `my Render` can be refuted by
    /// the concrete subclass that writes `Render` in a sibling document.
    ///
    /// One pass, deliberately: the class-name universe and the owner-resolved
    /// super/mixin edge maps are built **once** and every class is linearised
    /// against them.  Calling [`Self::class_linearisation`] per class instead
    /// rebuilds those maps each time — O(classes²) resolution work — and doing
    /// that on the diagnostics worker after every publish is enough to take
    /// the Performance suite from its ~5-minute baseline to a 60-minute
    /// timeout.  Per-class member names are memoised across linearisations for
    /// the same reason.
    ///
    /// `private` members are excluded — `TclOO` hides them even from an
    /// ancestor's own `my` dispatch — as are class-object-side members
    /// (`classmethod` / `self method`): `my` inside an instance method
    /// dispatches instance-side.  A class whose hierarchy cannot be
    /// linearised (cyclic, or over the shared complexity budget) contributes
    /// nothing: the abstention needs evidence, and none is available for it.
    #[must_use]
    pub fn subclass_provided_methods(
        &self,
    ) -> std::collections::BTreeMap<String, std::collections::BTreeSet<String>> {
        let mut supers_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut mixins_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for c in self.classes() {
            let owner = c.qualified_name.as_str();
            let resolve = |name: &str| self.resolve_class_relation(c, name);
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
        let qnames: std::collections::BTreeSet<String> = self
            .live_classes()
            .map(|c| c.qualified_name.clone())
            .collect();
        let mut member_names: std::collections::HashMap<
            String,
            std::collections::BTreeSet<String>,
        > = std::collections::HashMap::new();
        let mut map: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
            std::collections::BTreeMap::new();
        for d in &qnames {
            let lin =
                tcl_syntax::mro::tcloo_linearise(d, &supers_map, &mixins_map).unwrap_or_default();
            if lin.iter().all(|c| c == d) {
                continue;
            }
            let mut dispatchable: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();
            for x in &lin {
                if !member_names.contains_key(x) {
                    let names = self
                        .effective_members(x)
                        .into_iter()
                        .filter(|m| m.method.kind != CLASS_METHOD && !m.method.private)
                        .map(|m| m.name.to_owned())
                        .collect();
                    member_names.insert(x.clone(), names);
                }
                dispatchable.extend(member_names[x].iter().cloned());
            }
            if dispatchable.is_empty() {
                continue;
            }
            for ancestor in lin.iter().filter(|c| c.as_str() != d.as_str()) {
                map.entry(ancestor.clone())
                    .or_default()
                    .extend(dispatchable.iter().cloned());
            }
        }
        map
    }

    /// The C-Tcl-faithful **method dispatch chain** for an instance of
    /// `receiver_class` calling `method` under `access`: the linearisation's
    /// classes that define an instance-receiver implementation of `method`,
    /// in dispatch order,
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
    /// [`Self::method_dispatch_chain`].
    ///
    /// Same rules, read against the other side's tables: `class_method`
    /// declarations instead of `instance_method` ones,
    /// [`WorkspaceClass::class_exports`] / [`WorkspaceClass::class_unexports`]
    /// instead of the instance pair, and [`MemberSide::ClassObject`]
    /// tombstones.  Without it a `self unexport m` would be invisible across
    /// files: the flip would have no channel, and the workspace would keep
    /// resolving a `::C m` the interpreter answers with `unknown method "m"`
    /// (tclsh 9.0.4 / 8.6.14).
    ///
    /// One `TclOO` rule is class-side only: a stock `self method` lives on the
    /// class object that declared it and a **subclass's** class command never
    /// reaches it (`Gadget make` against a parent's `self method make` errors
    /// `unknown method "make"` on 8.6 and 9.0.4), whereas an `ooutil`
    /// `classmethod` does propagate.  Both share the `"classmethod"` receiver
    /// kind, so a `self method` is kept only when the providing record *is* the
    /// receiver class.
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
    /// own document declares.
    ///
    /// The revival direction already crosses files: a `self export` written
    /// next door reaches every query through
    /// [`Self::class_method_dispatch_chain`].  Suppression does not travel the
    /// same way, because the declaring document's in-document provider
    /// resolves the member locally and returns before the workspace chain
    /// runs — without this consultation a cross-file `self unexport Cm` /
    /// `self deletemethod Cm` would suppress `C Cm` for every document except
    /// the one the author is editing.  It is deliberately a thin reading of
    /// the same chain fold the cross-file tier resolves through — one decision
    /// function, so the two tiers cannot diverge (the established
    /// `exported_at_import_site` pattern).
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
        let edges = self.class_edges();
        self.dispatch_chain_over(&edges, receiver_class, method, access, side)
    }

    /// [`Self::dispatch_chain`] over edge maps the caller already has — the
    /// form a bulk query uses so the O(classes) edge resolution behind
    /// [`Self::class_edges`] is paid once for many receivers.
    fn dispatch_chain_over<'a>(
        &'a self,
        edges: &ClassEdges,
        receiver_class: &str,
        method: &str,
        access: MethodAccess,
        side: MemberSide,
    ) -> Vec<&'a WorkspaceClass> {
        let mut out: Vec<&WorkspaceClass> = Vec::new();
        let linearisation = edges.linearise(receiver_class).0;
        // The receiver's **effective** export flag, read once for the whole
        // walk.  A subclass can `export` / `unexport` a name it
        // inherits without redeclaring it: `export` / `unexport` accept a name
        // their class does not define and create a body-less table entry whose
        // only content is the flag, so the flag comes from the most specific
        // spine class that *mentions* the member while the implementation
        // still comes from the first class that declares a body.  Oracle,
        // byte-identical on tclsh 8.6.16 and 9.0.4:
        //
        //     oo::class create Base   { method tick {} { return base } }
        //     oo::class create Child  { superclass Base }
        //     oo::define Child { unexport tick }
        //     [Child new] tick    ;# -> unknown method "tick"
        //     oo::class create Base3  { method tock {} { return b3 } ; unexport tock }
        //     oo::class create Child3 { superclass Base3 ; export tock }
        //     [Child3 new] tock   ;# -> b3
        let branches: Vec<(Vec<String>, Option<bool>)> = if access == MethodAccess::External {
            edges
                .branch_spines(receiver_class, &linearisation)
                .into_iter()
                .map(|spine| {
                    let state = self.spine_export_state(&spine, method, side);
                    (spine, state)
                })
                .collect()
        } else {
            Vec::new()
        };
        for class_q in linearisation {
            // Several records of one class (its creation site plus every
            // `oo::define` stub, possibly spread over files) are all kept, and
            // the *first* is what go-to-definition answers with — so the order
            // has to be a property of the workspace, not of when each document
            // happened to be indexed.  Document URI then source position is
            // that stable order.
            let records = self.class_records(&class_q);
            // The class-level effective export union **for this side**: any
            // record exporting the name keeps it callable; explicit unexports
            // matter only when no record exports it.  The two sides never share
            // a set — a `self unexport m` must not silence an identically-named
            // instance method, and vice versa.
            let any_exports = records
                .iter()
                .any(|c| visibility_sets_for(c, side).0.iter().any(|e| e == method));
            let any_unexports = records
                .iter()
                .any(|c| visibility_sets_for(c, side).1.iter().any(|e| e == method));
            // The class's member set — retractions applied, arrivals
            // re-keyed — is decided once by [`Self::effective_members`]
            // rather than here.  A `deletemethod`ed member is
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
                //
                // …and a provider answers to the effective flag of the
                // **branch** that reaches it, because C enters each mixin with
                // a fresh copy of the dispatch flags and so lets a branch's
                // `export` / `unexport` govern that branch alone.
                let branch_state = branches
                    .iter()
                    .find(|(spine, _)| spine.contains(&class_q))
                    .and_then(|(_, state)| *state);
                let exported = match branch_state {
                    Some(exported) => exported,
                    None if any_exports => true,
                    None if any_unexports => false,
                    None => em.method.exported,
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

    /// Which of `receivers` dispatch `method` **externally** into `family` —
    /// the bulk form of "does [`Self::method_dispatch_chain`] land on one of
    /// these classes", answering every receiver from one
    /// [`Self::class_edges`] build.
    ///
    /// The caller is the cross-file method-family pass, which needs the answer
    /// per inheriting class before it scans each document: a
    /// captured `[self]` object command in a subclass body is a call site of
    /// the family's declaration only when that subclass can actually dispatch
    /// the name.  Asking [`Self::method_dispatch_chain`] once per inheritor
    /// would re-resolve every class edge each time — the O(classes²) shape
    /// [`Self::subclass_provided_methods`] documents the cost of.
    #[must_use]
    pub fn external_dispatch_receivers<'a>(
        &self,
        receivers: impl IntoIterator<Item = &'a str>,
        family: &[&str],
        method: &str,
        is_classmethod: bool,
    ) -> std::collections::HashSet<String> {
        let side = if is_classmethod {
            MemberSide::ClassObject
        } else {
            MemberSide::Instance
        };
        let edges = self.class_edges();
        receivers
            .into_iter()
            .filter(|cq| {
                // Non-empty is not enough: the chain must land *in this
                // family*.  A receiver whose external dispatch reaches some
                // other class's implementation (a live `mixin` branch ahead of
                // the spine, say) is dispatching that other member, and its
                // capture belongs to that family instead.
                self.dispatch_chain_over(&edges, cq, method, MethodAccess::External, side)
                    .first()
                    .is_some_and(|entry| family.contains(&entry.qualified_name.as_str()))
            })
            .map(ToOwned::to_owned)
            .collect()
    }

    /// The export flag an external dispatch reads for `method` along `spine`
    /// — the mixin-free linearisation from
    /// [`Self::class_linearisation_and_spine`] — or `None` when no class on it
    /// mentions the name at all, so there is no state to read and the
    /// per-record union in [`Self::dispatch_chain`] stands.
    ///
    /// "Mentions" is deliberately wider than "declares": `export` / `unexport`
    /// record a name their class need not define, and that body-less entry is
    /// the whole point of this walk.  Within one class the
    /// records are unordered across files, so the same
    /// explicit-export-wins precedence [`Self::dispatch_chain`] applies to a
    /// declaring class applies here too.
    fn spine_export_state(&self, spine: &[String], method: &str, side: MemberSide) -> Option<bool> {
        for class_q in spine {
            let records = self.class_records(class_q);
            let any_exports = records
                .iter()
                .any(|c| visibility_sets_for(c, side).0.iter().any(|e| e == method));
            let any_unexports = records
                .iter()
                .any(|c| visibility_sets_for(c, side).1.iter().any(|e| e == method));
            if any_exports {
                return Some(true);
            }
            if any_unexports {
                return Some(false);
            }
            if let Some(em) = self
                .effective_members(class_q)
                .into_iter()
                .find(|em| em.name == method && method_side(em.method) == side)
            {
                return Some(em.method.exported);
            }
        }
        None
    }

    /// Every record of the class `class_q`, in the workspace's stable order.
    ///
    /// Several records of one class (its creation site plus every
    /// `oo::define` stub, possibly spread over files) are all kept, and the
    /// *first* is what go-to-definition answers with — so the order has to be
    /// a property of the workspace, not of when each document happened to be
    /// indexed.  Document URI then source position is that order.
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
    /// its arrivals re-keyed.  Both receiver sides, in
    /// [`Self::class_records`] order then declaration order — filter on
    /// [`EffectiveMember::method`]'s `kind` (via `WorkspaceClass`'s own
    /// instance/class split) for one side.
    ///
    /// This is the single rule for "which members does this class have":
    /// [`Self::dispatch_chain`] resolves one name against it and every
    /// *enumeration* (`workspace/symbol`, an outline, a member completion
    /// universe) lists it, so the two cannot disagree: a member moved by a
    /// cross-file `renamemethod` is enumerated under the name it dispatches
    /// as, not the one its declaring record spells.
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
    /// walk then costs no more than a plain member scan.
    ///
    /// Ties are broken by the workspace's stable record order
    /// ([`Self::class_records`]), so which stub an arrival is attributed to
    /// does not depend on indexing order.
    fn retraction_index(&self) -> RetractionIndex<'_> {
        self.retraction_index_excluding(&HashSet::new())
    }

    /// Build the cross-document member fold without records contributed by
    /// excluded document slots. Filtering after the map is folded is unsound:
    /// a stable first retraction from an excluded document may have displaced
    /// a later live retraction for the same member.
    fn retraction_index_excluding(&self, excluded_slots: &HashSet<usize>) -> RetractionIndex<'_> {
        let mut records: Vec<&WorkspaceClass> = self
            .docs
            .iter()
            .enumerate()
            .filter(|(slot, _)| !excluded_slots.contains(slot))
            .flat_map(|(_, doc)| doc.classes.iter())
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
        let parents = |qname: &str| self.resolved_parents_of(qname);
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
        // Owner-aware direct parents (superclasses + mixins) of each class,
        // unioned across every indexed definition (a cross-file `oo::define`
        // stub must not hide the real class's parents).
        let parents = |qname: &str| self.resolved_parents_of(qname);
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
        // declaring record just spells it under the source name.
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
    /// and must not be reused here.
    #[must_use]
    pub fn proc_definitions_qualified<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceProc> {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.procs()
            .filter(|p| p.uri != exclude_uri)
            .filter(|p| unroot_rooted_key(&p.qualified_name).unwrap_or(&p.qualified_name) == target)
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
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.classes()
            .filter(|c| c.uri != exclude_uri)
            .filter(|c| unroot_rooted_key(&c.qualified_name).unwrap_or(&c.qualified_name) == target)
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
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.variables()
            .filter(|v| v.uri != exclude_uri)
            .filter(|v| unroot_rooted_key(&v.qualified_name).unwrap_or(&v.qualified_name) == target)
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
        let target = unroot_rooted_key(namespace).unwrap_or(namespace);
        let parent_matches = |qualified: &str| -> bool {
            let (holder, _) = key_holder_and_tail(qualified);
            unroot_rooted_key(holder).unwrap_or(holder) == target
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
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        let mut uris: Vec<String> = self
            .variable_aliases()
            .filter(|a| !alias_cell_is_computed(&a.qualified_name))
            .filter(|a| unroot_rooted_key(&a.qualified_name).unwrap_or(&a.qualified_name) == target)
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
        let (cell_holder, cell_tail) = key_holder_and_tail(qualified_name);
        let cell_ns = unroot_rooted_key(cell_holder).unwrap_or(cell_holder);
        let mut uris: Vec<String> = self
            .variable_aliases()
            .filter(|a| alias_cell_is_computed(&a.qualified_name))
            .filter(|a| {
                let (holder, tail) = key_holder_and_tail(&a.qualified_name);
                let ns = unroot_rooted_key(holder).unwrap_or(holder);
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

    /// Original selectors targeting one canonical document-owned method.
    /// Declaration advice and temporal receiver entries retain their separate
    /// purpose; no reporting-name family or native allocation is reconstructed.
    #[must_use]
    pub fn original_method_reference_rows(
        &self,
        target: &crate::method_symbol::OriginalMethodCandidate,
        include_declaration: bool,
    ) -> Vec<(String, Span, tcl_lexer::SourceImage, tcl_lexer::LexerConfig)> {
        let mut spans = Vec::new();
        if include_declaration {
            spans.push((
                target.uri().to_owned(),
                target.declaration_span(),
                target.declaration_image().clone(),
                target.declaration_config(),
            ));
        }
        for document in &self.docs {
            for query in &document.original_method_queries {
                if let std::ops::ControlFlow::Break(Some(candidate)) =
                    crate::method_symbol::candidate(self, &document.uri, query)
                    && candidate.same_declaration(target)
                {
                    spans.push((
                        document.uri.clone(),
                        query.span(),
                        query.consumer_image().clone(),
                        query.consumer_config(),
                    ));
                }
            }
        }
        spans.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| (left.1.start(), left.1.end()).cmp(&(right.1.start(), right.1.end())))
        });
        spans.dedup();
        spans
    }

    /// Every indexed namespace-name occurrence.
    pub fn namespace_refs(&self) -> impl Iterator<Item = &WorkspaceNamespaceRef> {
        self.docs.iter().flat_map(|doc| doc.namespace_refs.iter())
    }

    /// Original variable naming rows joined by the shared byte-table symbol.
    /// These are source occurrences; loading, alias lifetime and native frame
    /// entry remain independent. Display maps supply no lookup fallback.
    pub fn original_variable_occurrences<'a>(
        &'a self,
        symbol: &'a tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
        exclude_uri: &'a str,
    ) -> impl Iterator<
        Item = (
            &'a str,
            &'a tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
        ),
    > + 'a {
        let mut seen = std::collections::HashSet::new();
        self.docs
            .iter()
            .filter(move |document| document.uri != exclude_uri)
            .flat_map(move |document| {
                document
                    .original_variables
                    .iter()
                    .filter(move |occurrence| {
                        symbol.is_namespace() && occurrence.symbol() == symbol
                    })
                    .map(move |occurrence| (document.uri.as_str(), occurrence))
            })
            .filter(move |(owner, occurrence)| {
                seen.insert((*owner, occurrence.span(), occurrence.is_declaration()))
            })
    }

    /// Exact namespace occurrences from independently retained scope and name
    /// policy. Document ownership and source words remain in each row.
    pub fn original_namespace_occurrences<'a>(
        &'a self,
        symbol: &'a crate::namespace_symbol::OriginalNamespaceSymbol,
        exclude_uri: &'a str,
    ) -> impl Iterator<Item = &'a WorkspaceNamespaceRef> {
        let mut seen = std::collections::HashSet::new();
        self.namespace_refs().filter(move |row| {
            row.uri != exclude_uri
                && row.source.source_namespace.as_ref() == Some(symbol.scope())
                && row.source.name_policy == Some(symbol.policy())
                && row
                    .source
                    .original_name_input
                    .as_ref()
                    .is_some_and(|input| input.policy() == symbol.policy())
                && seen.insert((row.uri.as_str(), row.span, row.declares))
        })
    }

    /// Actual declaring rows whose retained components include this namespace
    /// as a strict parent. This grants no substring source edit.
    pub fn original_namespace_descendant_declarations<'a>(
        &'a self,
        symbol: &'a crate::namespace_symbol::OriginalNamespaceSymbol,
        exclude_uri: &'a str,
    ) -> impl Iterator<Item = &'a WorkspaceNamespaceRef> {
        let mut seen = std::collections::HashSet::new();
        self.namespace_refs().filter(move |row| {
            row.declares
                && row.uri != exclude_uri
                && row.source.name_policy == Some(symbol.policy())
                && row.source.source_namespace.as_ref().is_some_and(|scope| {
                    symbol.scope().is_strict_ancestor_of(scope, symbol.policy()) == Some(true)
                })
                && seen.insert((row.uri.as_str(), row.span))
        })
    }

    /// Exact retained namespace selected from all indexed source receipts.
    #[must_use]
    pub fn retained_namespace(
        &self,
        report: &str,
    ) -> Option<(
        tcl_compiler::signature_scan::scope::SignatureNamespaceScope,
        tcl_syntax::naming::NamePolicyProtocol,
    )> {
        crate::namespace_symbol::retained_namespace_from_refs(
            self.namespace_refs().map(|row| &row.source),
            report,
        )
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
        let Some((scope, policy)) = self.retained_namespace(qualified_name) else {
            return Vec::new();
        };
        self.namespace_refs()
            .filter(|n| n.declares && n.uri != exclude_uri)
            .filter(|n| {
                n.source.source_namespace.as_ref() == Some(&scope)
                    && n.source.name_policy == Some(policy)
            })
            .collect()
    }

    /// Declaring sites whose namespace is a **strict descendant** of
    /// `qualified_name` — the rows that create it *implicitly*, as a parent
    /// (`namespace eval ::p::q::r {}` really creates `::p::q`), excluding any
    /// in `exclude_uri`.
    ///
    /// The cross-document half of the implicit-parent rule: the in-document
    /// tier answers implicit parents from its own `namespace_refs`, and
    /// without this query a namespace whose only creating block lives in a
    /// sibling file has no answer at all.
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
        let Some((scope, policy)) = self.retained_namespace(qualified_name) else {
            return Vec::new();
        };
        self.namespace_refs()
            .filter(|n| n.declares && n.uri != exclude_uri)
            .filter(|n| {
                n.source.name_policy == Some(policy)
                    && n.source.source_namespace.as_ref() != Some(&scope)
            })
            .filter(|n| {
                crate::namespace_symbol::retained_namespace_from_refs(
                    std::iter::once(&n.source),
                    qualified_name,
                )
                .as_ref()
                    == Some(&(scope.clone(), policy))
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
        let Some((scope, policy)) = self.retained_namespace(qualified_name) else {
            return Vec::new();
        };
        self.namespace_refs()
            .filter(|n| !n.declares && n.uri != exclude_uri)
            .filter(|n| {
                n.source.source_namespace.as_ref() == Some(&scope)
                    && n.source.name_policy == Some(policy)
            })
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
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.variable_refs()
            .filter(|v| v.uri != exclude_uri)
            .filter(|v| unroot_rooted_key(&v.qualified_name).unwrap_or(&v.qualified_name) == target)
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

    /// References to an independently retained original declaration slot.
    /// Linked references keep their selected implementation's declaration slot;
    /// direct references keep the actual called slot. Source assistance grants
    /// neither interpreter loading nor writable edits.
    #[must_use]
    pub fn original_invocations_of<'a>(
        &'a self,
        declaration: &tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        exclude_uri: &str,
        follow_links: bool,
    ) -> Vec<&'a WorkspaceInvocation> {
        let target =
            SettledCommandTarget::OriginalSlot(declaration.slot().clone(), declaration.policy());
        self.settled_sites(&[target], follow_links)
            .into_iter()
            .map(|(slot, index)| &self.docs[slot].invocations[index])
            .filter(|invocation| invocation.uri != exclude_uri)
            .collect()
    }

    /// Whether renaming `qualified_name` must be refused outright: some
    /// invocation of it, anywhere in the workspace, is marked
    /// `rename_safe: false` — an indirect dispatch at least one of whose
    /// contributing constants has no exact writable source span, so no
    /// edit set can keep that dispatch running the renamed command.
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
    /// A hash lookup into [`Self::settled_sites`] plus a
    /// direct index into each hit's owning document slot — the settlement
    /// walk itself (which needs [`Self::defined_command_names`],
    /// [`Self::command_link_map`] and a [`WildcardImportIndex`]) runs at most
    /// once per generation, not once per call (`code_lenses` calls this once
    /// per proc *and* once per class in the document).
    fn invocations_settling_to<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
        follow_links: bool,
    ) -> Vec<&'a WorkspaceInvocation> {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        let mut targets = vec![SettledCommandTarget::Authored(target.to_owned())];
        for declaration in self
            .original_procedure_declarations()
            .filter(|(_, declaration)| {
                unroot_rooted_key(&declaration.metadata().qualified_name)
                    .unwrap_or(&declaration.metadata().qualified_name)
                    == target
            })
        {
            targets.push(SettledCommandTarget::OriginalSlot(
                declaration.1.name().slot().clone(),
                declaration.1.name().policy(),
            ));
        }
        for declaration in self
            .original_class_declarations()
            .filter(|(_, declaration)| {
                unroot_rooted_key(&declaration.metadata().qualified_name)
                    .unwrap_or(&declaration.metadata().qualified_name)
                    == target
            })
        {
            targets.push(SettledCommandTarget::OriginalSlot(
                declaration.1.name().slot().clone(),
                declaration.1.name().policy(),
            ));
        }
        let sites = self.settled_sites(&targets, follow_links);
        sites
            .iter()
            .map(|(slot, idx)| &self.docs[*slot].invocations[*idx])
            .filter(|i| i.uri != exclude_uri)
            .collect()
    }

    /// Return the sites settled to `target`, incrementally repairing the
    /// changed documents first.  The cold path settles every document once.
    /// Thereafter a replacement which did not change the command-resolution
    /// inputs removes and re-inserts only that document's contribution in the
    /// reverse index; querying a common target never scans its callers.
    fn settled_sites(
        &self,
        targets: &[SettledCommandTarget],
        follow_links: bool,
    ) -> Vec<(usize, usize)> {
        let cache = &self.settled_invocations[usize::from(follow_links)];
        let mut state = cache
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let slots: Vec<usize> = if state.rebuild_all || state.by_document.len() != self.docs.len() {
            state.by_document.clear();
            state.by_document.resize_with(self.docs.len(), || None);
            state.by_target.clear();
            state.rebuild_all = false;
            state.dirty_slots.clear();
            (0..self.docs.len()).collect()
        } else {
            std::mem::take(&mut state.dirty_slots).into_iter().collect()
        };

        if !slots.is_empty() {
            // These are exactly the three workspace-wide inputs
            // `settle_invocation` reads.  They were retained across an
            // invocation-only replacement by `invalidate`, and are rebuilt
            // only when `SettlementDependencies` proved they changed.
            let defined = self.defined_command_names(follow_links);
            let links = follow_links.then(|| self.command_link_map());
            let wci = WildcardImportIndex::build(self);
            for slot in slots {
                if let Some(previous) = state.by_document[slot].take() {
                    for (old_target, idx) in previous {
                        let remove_key = (slot, idx);
                        let empty = state.by_target.get_mut(&old_target).is_some_and(|sites| {
                            sites.remove(&remove_key);
                            sites.is_empty()
                        });
                        if empty {
                            state.by_target.remove(&old_target);
                        }
                    }
                }
                let mut contribution = Vec::new();
                for (idx, inv) in self.docs[slot].invocations.iter().enumerate() {
                    if let Some(resolved) =
                        self.settle_invocation(inv, &defined, links.as_deref(), &wci)
                    {
                        state
                            .by_target
                            .entry(resolved.clone())
                            .or_default()
                            .insert((slot, idx));
                        contribution.push((resolved, idx));
                    }
                }
                state.by_document[slot] = Some(contribution);
                state.settled_documents = state.settled_documents.saturating_add(1);
            }
        }
        targets
            .iter()
            .filter_map(|target| state.by_target.get(target))
            .flat_map(|sites| sites.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn original_definition_target(
        &self,
        definition: &tcl_compiler::command_binding::SourceCommandDefinition,
    ) -> Option<SettledCommandTarget> {
        let site = &definition.allocation().site;
        self.original_procedure_declarations()
            .find(|(_, declaration)| declaration.declaration_site() == site)
            .map(|(_, declaration)| {
                SettledCommandTarget::OriginalSlot(
                    declaration.name().slot().clone(),
                    declaration.name().policy(),
                )
            })
            .or_else(|| {
                self.original_class_declarations()
                    .find(|(_, declaration)| declaration.declaration_site() == site)
                    .map(|(_, declaration)| {
                        SettledCommandTarget::OriginalSlot(
                            declaration.name().slot().clone(),
                            declaration.name().policy(),
                        )
                    })
            })
    }

    fn original_assistance_target(
        &self,
        lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
        follow_links: bool,
    ) -> Option<SettledCommandTarget> {
        let mut candidates = Vec::new();
        for document in &self.docs {
            if let Some(world) = &document.original_command_world {
                for publication in world.declarations() {
                    let target = if follow_links {
                        publication
                            .definition()
                            .and_then(|definition| self.original_definition_target(definition))
                    } else {
                        matches!(publication.kind(),
                            tcl_compiler::command_binding::OriginalCommandPublicationKind::Procedure
                            | tcl_compiler::command_binding::OriginalCommandPublicationKind::Object)
                        .then(|| {
                            SettledCommandTarget::OriginalSlot(
                                publication.slot().clone(),
                                publication.policy(),
                            )
                        })
                    };
                    candidates.push((publication.slot(), publication.policy(), target));
                }
            } else {
                // Header-only source assistance has a separate purpose. It
                // grants no completed world, current implementation or edit.
                candidates.extend(
                    document
                        .original_procedure_source_candidates()
                        .into_iter()
                        .map(|(declaration, slot, policy)| {
                            (
                                slot,
                                policy,
                                Some(SettledCommandTarget::OriginalSlot(
                                    declaration.name().slot().clone(),
                                    declaration.name().policy(),
                                )),
                            )
                        }),
                );
                candidates.extend(document.original_class_source_candidates().into_iter().map(
                    |(declaration, slot, policy, _)| {
                        (
                            slot,
                            policy,
                            Some(SettledCommandTarget::OriginalSlot(
                                declaration.name().slot().clone(),
                                declaration.name().policy(),
                            )),
                        )
                    },
                ));
            }
        }
        let selected = lookup.matching_slot_publications(candidates)?;
        let target = selected.first()?.as_ref()?;
        selected
            .iter()
            .all(|candidate| candidate.as_ref() == Some(target))
            .then(|| target.clone())
    }

    /// The command `inv` settles to, `::`-stripped, or `None` when nothing in
    /// the workspace resolves it: the first of its candidates defined
    /// anywhere in the workspace, chased along `links` (when supplied) to its
    /// ultimate target — falling back, when following links, to a wildcard
    /// `namespace import NS::*` in scope.  The question asked is "what does
    /// this call settle to" (grouped by the answer) rather than "does it
    /// settle to the one target the caller named", which would be re-checked
    /// once per candidate target.
    fn settle_invocation(
        &self,
        inv: &WorkspaceInvocation,
        defined: &HashSet<String>,
        links: Option<&std::collections::HashMap<String, String>>,
        wci: &WildcardImportIndex<'_>,
    ) -> Option<SettledCommandTarget> {
        if let Some(input) = &inv.original_name_input {
            let lookup = inv.original_lookup.as_ref()?;
            if lookup.name_input() != input {
                return None;
            }
        }
        if let Some(reference) = &inv.resolved_command_reference
            && (inv.original_name_input.is_some() || reference.original_name_policy().is_some())
        {
            return if links.is_some() {
                reference
                    .linked_definition()
                    .or_else(|| reference.definition())
                    .and_then(|definition| self.original_definition_target(definition))
            } else {
                reference.is_direct_definition().then_some(())?;
                Some(SettledCommandTarget::OriginalSlot(
                    reference.original_slot()?.clone(),
                    reference.original_name_policy()?,
                ))
            };
        }
        if inv.original_name_input.is_some() {
            return self.original_assistance_target(inv.original_lookup.as_ref()?, links.is_some());
        }
        // A positioned source receipt precedes the workspace's assistance
        // inventory. Later imports, renames and unrelated same-named records
        // cannot change the implementation this invocation already selected.
        // The direct view edits a called definition slot; imported spellings
        // belong only to the linked view, which retains the actual declaration.
        if let Some(reference) = &inv.resolved_command_reference {
            return if links.is_some() {
                reference
                    .linked_definition()
                    .or_else(|| reference.definition())
                    .map(|definition| {
                        unroot_rooted_key(&definition.allocation().command)
                            .unwrap_or(&definition.allocation().command)
                            .to_owned()
                    })
                    .map(SettledCommandTarget::Authored)
            } else {
                reference.is_direct_definition().then_some(())?;
                reference
                    .slot()
                    .map(|slot| unroot_rooted_key(slot).unwrap_or(slot).to_owned())
                    .map(SettledCommandTarget::Authored)
            };
        }
        if let Some(definition) = &inv.resolved_definition {
            return if links.is_some() {
                Some(SettledCommandTarget::Authored(
                    unroot_rooted_key(&definition.allocation().command)
                        .unwrap_or(&definition.allocation().command)
                        .to_owned(),
                ))
            } else {
                inv.resolved_user_definition.as_deref().map(|slot| {
                    SettledCommandTarget::Authored(
                        unroot_rooted_key(slot).unwrap_or(slot).to_owned(),
                    )
                })
            };
        }
        let call = CallSite {
            uri: &inv.uri,
            at: inv.range.start(),
            enclosing_body: inv.enclosing_body,
        };
        // A live `namespace import -force` has *replaced* the importing
        // namespace's own command of this name, so no candidate naming that
        // command may settle the call — it reaches the import's source, which
        // the wildcard tier below resolves. The same rule
        // `definition::resolve_called_proc` applies in-document, so without it
        // find-references files the call under the definition the import
        // deleted while go-to-definition jumps to the source.
        //
        // Only in the link-following view, matching the rule below: a glob
        // import introduces no fixed link, so the direct-only view rename
        // relies on does not see it in either direction.
        let forced_shadow = links.is_some()
            && wci.forced_shadow_over_candidates(&inv.name, &inv.resolution_candidates, call);
        if !forced_shadow && let Some(winner) = inv.resolved_user_definition.as_deref() {
            return Some(SettledCommandTarget::Authored(
                unroot_rooted_key(winner).unwrap_or(winner).to_owned(),
            ));
        }
        if !forced_shadow
            && let Some(winner) = inv
                .resolution_candidates
                .iter()
                .find(|c| defined.contains(unroot_rooted_key(c).unwrap_or(c)))
        {
            let winner = unroot_rooted_key(winner).unwrap_or(winner);
            return Some(SettledCommandTarget::Authored(links.map_or_else(
                || winner.to_owned(),
                |m| Self::follow_links(m, winner),
            )));
        }
        // No real command or name-link settled this call — try a wildcard
        // `namespace import NS::*` in scope for the call's own namespace
        // Only when following links: this mirrors an
        // exact import's `WorkspaceCommandLink`, which likewise only
        // participates in the *linked* view (`linked_invocations_of`, used
        // by find-references) and never the direct-only view rename relies
        // on — a call spelling the local imported name is not text-rewritten
        // just because its ultimate source is renamed.
        links?;
        self.resolve_wildcard_import_indexed(&inv.name, &inv.resolution_candidates, call, wci)
            .map(|resolved| {
                SettledCommandTarget::Authored(
                    unroot_rooted_key(&resolved).unwrap_or(&resolved).to_owned(),
                )
            })
    }

    /// The command name-link map (`::`-stripped `linked → immediate target`)
    /// used to chase an import / alias / rename to the command it names.
    /// A [`Derived`] view rather than a fresh walk of
    /// [`Self::live_command_links`] per call; owned (`String`,
    /// not `&str`) so the view is independent of any one call's borrow.
    fn command_link_map(&self) -> Arc<std::collections::HashMap<String, String>> {
        self.command_link_map.get_or_build(|| {
            self.live_command_links()
                .into_iter()
                .filter(|link| {
                    link.linked_source_name
                        .as_ref()
                        .is_none_or(|name| name.source_spelling().is_some())
                        && link
                            .target_source_name
                            .as_ref()
                            .is_none_or(|name| name.source_spelling().is_some())
                })
                .map(|l| {
                    (
                        unroot_rooted_key(&l.linked_qname)
                            .unwrap_or(&l.linked_qname)
                            .to_owned(),
                        unroot_rooted_key(&l.target_qname)
                            .unwrap_or(&l.target_qname)
                            .to_owned(),
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
        let settled = Self::follow_links(&links, unroot_rooted_key(name).unwrap_or(name));
        root_unrooted_key(&settled)
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
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.live_command_links()
            .into_iter()
            .filter(|l| l.uri != exclude_uri)
            .filter(|l| unroot_rooted_key(&l.target_qname).unwrap_or(&l.target_qname) == target)
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

    /// The qualified names of indexed classes whose own command constructs an
    /// instance from a bare unrecognised word — the workspace half of Tk's
    /// `::tk::IconList .il` idiom.
    ///
    /// Empty for every workspace with no such metaclass, which is nearly all
    /// of them, so a consumer given it pays nothing.
    #[must_use]
    pub fn bare_word_construction_class_qnames(&self) -> std::collections::HashSet<String> {
        self.classes()
            .filter(|c| c.bare_word_construction)
            .map(|c| c.qualified_name.clone())
            .collect()
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
                    .any(|c| class_qnames.contains(unroot_rooted_key(c).unwrap_or(c)))
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
            .contains(unroot_rooted_key(qualified_name).unwrap_or(qualified_name))
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
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.live_procs().any(|p| {
            (!has_builtin || !p.nested)
                && unroot_rooted_key(&p.qualified_name).unwrap_or(&p.qualified_name) == target
        }) || self
            .live_classes()
            .any(|c| unroot_rooted_key(&c.qualified_name).unwrap_or(&c.qualified_name) == target)
            || self.live_command_links().into_iter().any(|l| {
                (!has_builtin || !l.nested)
                    && unroot_rooted_key(&l.linked_qname).unwrap_or(&l.linked_qname) == target
            })
    }

    /// The set of `::`-stripped qualified names of every indexed proc and
    /// class, for O(1) membership in the candidate-resolution loop of
    /// [`Self::invocations_of`].  With `include_links`, the names an import /
    /// alias / rename introduces join the set, so a call reaching one of them
    /// settles (and is then chased to its ultimate target).
    ///
    /// A [`Derived`] view rather than a fresh walk: it
    /// is `O(procs + classes + links)` to build, and
    /// [`Self::workspace_command_exists`] asks for it *per candidate* inside
    /// [`Self::follow_import_chain`]'s loop — and
    /// [`Self::settled_sites`] once per settling pass — so rebuilding it per
    /// ask would turn an existence test into a workspace-wide scan. Owned (`String`,
    /// not `&str`) so the view is independent of any one call's borrow. The
    /// two `include_links` readings are two separate views because a consumer
    /// wants exactly one of them: the direct-only set is what rename relies
    /// on, and folding the link names into it would let a call spelling an
    /// imported name be text-rewritten.
    fn defined_command_names(&self, include_links: bool) -> Arc<HashSet<String>> {
        self.defined_names[usize::from(include_links)].get_or_build(|| {
            let mut names: HashSet<String> = self
                .live_procs()
                .map(|p| {
                    unroot_rooted_key(&p.qualified_name)
                        .unwrap_or(&p.qualified_name)
                        .to_owned()
                })
                .chain(self.live_classes().map(|c| {
                    unroot_rooted_key(&c.qualified_name)
                        .unwrap_or(&c.qualified_name)
                        .to_owned()
                }))
                .collect();
            if include_links {
                names.extend(self.live_command_links().into_iter().map(|l| {
                    unroot_rooted_key(&l.linked_qname)
                        .unwrap_or(&l.linked_qname)
                        .to_owned()
                }));
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
    /// the in-document resolver.
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
    /// would be O(invocation count × workspace-wide glob-import count), and
    /// measurably slows find-references on a codebase using
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
            let (prefix, tail) = tcl_syntax::naming::key_holder_and_tail(cand);
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
    /// origin ::A::p` answers `::C::p` (oracle tclsh 8.6.14 / 9.0.4). The
    /// middle hop is in no workspace proc/class table, so a single-hop walk
    /// finds nothing and go-to-definition silently abstains.
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
    ///    position** ([`WildcardImportIndex::exports_name_at`]);
    /// 3. without `-force`, the importing namespace must not already hold a
    ///    command of that name — such an import raises `can't import command
    ///    "p": already exists` and installs nothing, so a bare call still
    ///    reaches the local definition. With `-force` it
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
            .admitting(word)
            .filter(|row| {
                let target = tcl_syntax::naming::qualify(&row.imp.source_ns, word);
                !self.workspace_command_exists(&target)
                    || self.import_source_matches(&target, row.imp.native_source.as_ref(), word)
            })
            .filter(|row| {
                let target = tcl_syntax::naming::qualify(ns, word);
                row.imp.forced
                    || !(self.defines_command(&target)
                        || row.declares_builtin_at(&target)
                        || wci.conflicting_alias_at(ns, &row.imp.source_ns, word, row.site()))
            })
            .filter(|row| wci.alias_live_at(ns, &row.imp.source_ns, word, row.site(), call))
            .enumerate()
            .max_by_key(|(seq, row)| (row.imp.uri == call.uri, row.imp.at, *seq))
            .map(|(_, row)| row.imp.source_ns.clone())
    }
}

/// The names a source namespace had exported by the time one recorded import
/// ran — the word-independent half of
/// [`crate::namespace_import::exported_at_import_site`], decided once per
/// import at index-build time.
///
/// An import's position is fixed by the source text, so which export rows are
/// visible from it, and which of those a `-clear` tombstone revokes, cannot
/// depend on the call being resolved. Only the final glob match can. Asking
/// the whole question per call would run the [`crate::source_graph::RunOrder`]
/// walk once per (invocation × in-scope import × export row); with the
/// timeline half hoisted here it runs once per import, and the per-call cost is
/// a hash probe plus a glob match against however few non-literal patterns
/// remain.
///
/// Splitting literal patterns out is the same reasoning one level down:
/// `namespace export Resistor R Capacitor C …` is the common shape, and a
/// pattern with no metacharacter matches exactly its own text
/// ([`tcl_syntax::glob::is_literal`]), so it belongs in a set rather than in a
/// linear glob scan.
#[derive(Debug, Default)]
struct ExportGate<'a> {
    /// Surviving patterns with no glob metacharacter: an exact-name set.
    literal: HashSet<&'a str>,
    /// Original import and export purposes were already checked against the
    /// retained native declaration slot; display membership needs no parser.
    original_tail_matched: bool,
    /// Surviving patterns that need `Tcl_StringMatch` semantics (`*`, `b*`,
    /// `{p[ab]}`, …), in recorded order.
    globs: Vec<&'a str>,
}

impl<'a> ExportGate<'a> {
    /// The gate for `source_ns` as seen from the import at `site`, or an empty
    /// gate when the namespace has no export rows at all.
    fn decide(
        exports_by_ns: &std::collections::HashMap<&'a str, Vec<&'a WorkspaceNamespaceExport>>,
        order: &crate::source_graph::RunOrder,
        source_ns: &str,
        site: ImportSite<'_>,
        original: Option<&tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
        subjects: &[&'a tcl_compiler::signature_scan::scope::SignatureSourceCommand],
        native_source_known: bool,
    ) -> Self {
        if let Some(original) = original {
            if exports_by_ns.values().flatten().any(|event| {
                event.unknown
                    && order
                        .has_run(
                            RunPoint {
                                uri: &event.uri,
                                at: event.at,
                                enclosing_body: event.enclosing_body,
                            },
                            site.point(),
                        )
                        .unwrap_or(true)
            }) {
                return Self::default();
            }
            let literal = subjects
                .iter()
                .copied()
                .filter(|name| {
                    original.matches_imported_command(name.slot(), name.policy()) == Some(true)
                        && crate::namespace_import::exported_original_at_import_site(
                            exports_by_ns.values().flatten().filter_map(|event| {
                                Some((
                                    event.original.as_ref()?,
                                    RunPoint {
                                        uri: &event.uri,
                                        at: event.at,
                                        enclosing_body: event.enclosing_body,
                                    },
                                ))
                            }),
                            name.slot(),
                            name.policy(),
                            order,
                            site.point(),
                        ) == ExportVerdict::Exported
                })
                .filter_map(|name| name.slot().simple.try_utf8().ok())
                .collect();
            return Self {
                literal,
                globs: Vec::new(),
                original_tail_matched: true,
            };
        }
        if native_source_known {
            return Self::default();
        }
        let Some(exports) = exports_by_ns.get(source_ns) else {
            return Self::default();
        };
        let mut events = export_events(
            exports
                .iter()
                .copied()
                .filter(|event| event.original.is_none() && !event.unknown),
        );
        let surviving =
            crate::namespace_import::exports_in_effect(&mut events, order, site.point());
        let (literal, globs) = surviving
            .into_iter()
            .partition::<Vec<_>, _>(|p| tcl_syntax::glob::is_literal(p));
        Self {
            literal: literal.into_iter().collect(),
            original_tail_matched: false,
            globs,
        }
    }

    /// Whether the gate lets `name` through — the answer
    /// [`crate::namespace_import::exported_at_import_site`] would give for the
    /// same name at the same site.
    fn covers(&self, name: &str) -> bool {
        self.literal.contains(name)
            || self
                .globs
                .iter()
                .any(|pattern| tcl_syntax::glob::string_match(pattern, name))
    }
}

/// One recorded wildcard `namespace import NS::*`, with its export gate
/// already decided — see [`ExportGate`].
#[derive(Debug)]
struct GlobImportRow<'a> {
    imp: &'a WorkspaceGlobImport,
    exported: ExportGate<'a>,
    /// The command table a fresh interpreter running *this import's document*
    /// holds — the other half of the import-conflict rule, and dialect-correct
    /// because it is resolved from that document's own analysed dialect.
    /// `None` when the index cannot say.
    registry: Option<&'static tcl_registry::CommandRegistry>,
}

/// Every wildcard import recorded for one importing namespace, indexed by the
/// names its imports can actually admit.
///
/// Scanning every row in the namespace on every call, applying
/// `string_match(tail_pattern, word)` and `ExportGate::covers(word)` to each,
/// costs ~140 rows per call on a large corpus — 28 369 calls scanning
/// 3 987 739 rows, and ~390 ms of the ~420 ms a `textDocument/references`
/// takes after an edit.
///
/// Both of those filters depend only on the *word*, never on the call, so the
/// admissible set per word is a build-time fact. A row whose gate exports
/// nothing can admit no word at all and disappears from every lookup — which
/// is most of them in practice, since a workspace typically imports several
/// namespaces it has no `namespace export` records for (`::tcl::mathop`,
/// `::tcltest`).
#[derive(Debug, Default)]
struct NamespaceImports<'a> {
    /// The namespace's rows, in recorded order. [`Self::admitting`] yields
    /// indices into this, ascending, so callers see exactly the subsequence a
    /// full scan would have produced.
    rows: Vec<GlobImportRow<'a>>,
    /// Row indices keyed by a **literal** name that both the row's tail
    /// pattern and its export gate admit.
    by_name: rustc_hash::FxHashMap<&'a str, Vec<u32>>,
    /// Rows whose export gate holds a glob pattern: any word may pass, so
    /// these keep the per-word check.
    glob_gated: Vec<u32>,
    /// `-force` rows, for [`WildcardImportIndex::forced_shadow_at`] — which
    /// can bypass the export gate entirely for an unobservable source
    /// namespace, so it cannot use [`Self::by_name`].
    forced: Vec<u32>,
}

impl<'a> NamespaceImports<'a> {
    /// Index `rows` by the names they admit. Called once per namespace per
    /// index build.
    fn index(rows: Vec<GlobImportRow<'a>>) -> Self {
        let mut by_name: rustc_hash::FxHashMap<&'a str, Vec<u32>> =
            rustc_hash::FxHashMap::default();
        let mut glob_gated = Vec::new();
        let mut forced = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            let idx = u32::try_from(i).unwrap_or(u32::MAX);
            if row.imp.forced {
                forced.push(idx);
            }
            if !row.exported.globs.is_empty() {
                glob_gated.push(idx);
            }
            for name in &row.exported.literal {
                // The tail pattern is the other half of the admission test,
                // and it is decidable here because the name is known.
                if row.exported.original_tail_matched
                    || tcl_syntax::glob::string_match(&row.imp.tail_pattern, name)
                {
                    by_name.entry(name).or_default().push(idx);
                }
            }
        }
        Self {
            rows,
            by_name,
            glob_gated,
            forced,
        }
    }

    /// The rows whose tail pattern **and** export gate both admit `word`, in
    /// recorded order — the exact subsequence
    /// `rows.iter().filter(tail matches).filter(gate covers)` would yield.
    ///
    /// Order matters: `import_hop` breaks ties on the position within this
    /// sequence, so producing it out of order would change which import wins.
    fn admitting(&self, word: &str) -> impl Iterator<Item = &GlobImportRow<'a>> {
        let mut idx: Vec<u32> = self
            .by_name
            .get(word)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .to_vec();
        idx.extend(self.glob_gated.iter().copied().filter(|&i| {
            let row = &self.rows[i as usize];
            tcl_syntax::glob::string_match(&row.imp.tail_pattern, word) && row.exported.covers(word)
        }));
        // A row carrying both literal and glob patterns can arrive twice.
        idx.sort_unstable();
        idx.dedup();
        idx.into_iter().map(|i| &self.rows[i as usize])
    }

    /// The `-force` rows, in recorded order.
    fn forced_rows(&self) -> impl Iterator<Item = &GlobImportRow<'a>> {
        self.forced.iter().map(|&i| &self.rows[i as usize])
    }
}

impl<'a> GlobImportRow<'a> {
    fn site(&self) -> ImportSite<'a> {
        self.imp.site()
    }

    /// Whether a fresh interpreter of this import's dialect already holds a
    /// command at `qualified_name` — the registry half of the non-`-force`
    /// "already exists" conflict.
    ///
    /// The workspace half ([`WorkspaceIndex::defines_command`]) sees only
    /// procs and classes the workspace itself declares, so without this an
    /// unforced `namespace import` of a namespace exporting `set` reads as
    /// installed and every bare `set` in the workspace is filed as a
    /// reference to it — where real Tcl raises `can't import command "set":
    /// already exists` and installs nothing (oracle 9.0.4 / 8.6.14). The
    /// single-document tier applies the same gate
    /// (`definition::resolve_called_proc`'s `has_builtin`), from the same
    /// source of truth.
    ///
    /// Abstains toward *not* conflicting when the dialect is unknown:
    /// inventing a conflict drops an alias the program really has.
    fn declares_builtin_at(&self, qualified_name: &str) -> bool {
        self.registry
            .is_some_and(|r| r.declares_command_at(qualified_name))
    }
}

/// One recorded **exact** `namespace import ::src::p` link, with its export
/// gate already decided — the exact-pattern twin of [`GlobImportRow`].
#[derive(Debug)]
struct ExactImportRow<'a> {
    /// The document the link's import is written in — [`WorkspaceImportGate`]
    /// does not duplicate it.
    uri: &'a str,
    gate: &'a WorkspaceImportGate,
    exported: ExportGate<'a>,
}

impl<'a> ExactImportRow<'a> {
    fn site(&self) -> ImportSite<'a> {
        self.gate.site(self.uri)
    }
}

/// Precomputed per-namespace grouping of [`WorkspaceIndex::glob_imports`]
/// and [`WorkspaceIndex::namespace_exports`], built once per query rather
/// than re-scanned per call site — see
/// [`WorkspaceIndex::resolve_wildcard_import_indexed`]'s doc for why.
struct WildcardImportIndex<'a> {
    original_subjects: Vec<&'a tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
    imports_by_ns: std::collections::HashMap<&'a str, NamespaceImports<'a>>,
    /// Every **exact** `namespace import` link, by importing namespace. The
    /// exact-pattern twin of [`Self::imports_by_ns`]: the two tables are one
    /// command table as far as the import-conflict rule is concerned, which is
    /// why [`Self::conflicting_alias_at`] reads both.
    exact_by_ns: std::collections::HashMap<&'a str, Vec<ExactImportRow<'a>>>,
    exports_by_ns: std::collections::HashMap<&'a str, Vec<&'a WorkspaceNamespaceExport>>,
    forgets_by_ns: std::collections::HashMap<&'a str, Vec<&'a WorkspaceNamespaceForget>>,
    deletions_by_name: std::collections::HashMap<&'a str, Vec<&'a WorkspaceCommandDeletion>>,
    /// Every proc / class declaration site, by `::`-rooted qualified name —
    /// a redefinition of an *imported* name ends the alias, which is a
    /// question about where the declaration sits, not
    /// merely whether one exists.
    declarations_by_qname: std::collections::HashMap<&'a str, Vec<(&'a str, u32)>>,
    /// The workspace's `source`-graph load order — the relation every
    /// decision below ranks its events with, so a cross-document event is
    /// ordered wherever the graph proves an order and unrankable everywhere
    /// else.
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
        // Exports first: every import row's gate is decided against them here,
        // once, instead of once per call site.
        let mut exports_by_ns: std::collections::HashMap<&str, Vec<&WorkspaceNamespaceExport>> =
            std::collections::HashMap::new();
        for exp in index.namespace_exports() {
            exports_by_ns.entry(exp.ns.as_str()).or_default().push(exp);
        }
        let order = index.run_order();
        let original_subjects = index
            .docs
            .iter()
            .flat_map(|doc| {
                doc.original_procs
                    .iter()
                    .map(|record| record.name())
                    .chain(doc.original_classes.iter().map(|record| record.name()))
            })
            .collect::<Vec<_>>();
        let mut rows_by_ns: std::collections::HashMap<&str, Vec<GlobImportRow<'a>>> =
            std::collections::HashMap::new();
        for imp in index.glob_imports() {
            rows_by_ns
                .entry(imp.ns.as_str())
                .or_default()
                .push(GlobImportRow {
                    imp,
                    exported: ExportGate::decide(
                        &exports_by_ns,
                        &order,
                        &imp.source_ns,
                        imp.site(),
                        imp.original_pattern.as_ref(),
                        &original_subjects,
                        imp.native_source.is_some(),
                    ),
                    registry: index.registry_for(&imp.uri),
                });
        }
        let mut exact_by_ns: std::collections::HashMap<&str, Vec<ExactImportRow<'a>>> =
            std::collections::HashMap::new();
        for link in index.command_links() {
            if let Some(gate) = link.import_gate.as_ref() {
                let uri = link.uri.as_str();
                exact_by_ns
                    .entry(importing_namespace_of(&link.linked_qname))
                    .or_default()
                    .push(ExactImportRow {
                        uri,
                        gate,
                        exported: ExportGate::decide(
                            &exports_by_ns,
                            &order,
                            &gate.source_ns,
                            gate.site(uri),
                            gate.original_pattern.as_ref(),
                            &original_subjects,
                            gate.native_source.is_some(),
                        ),
                    });
            }
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
        let imports_by_ns = rows_by_ns
            .into_iter()
            .map(|(ns, rows)| (ns, NamespaceImports::index(rows)))
            .collect();
        Self {
            original_subjects,
            imports_by_ns,
            exact_by_ns,
            exports_by_ns,
            forgets_by_ns,
            deletions_by_name,
            declarations_by_qname,
            order,
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
            // Only the `-force` rows: this is the one path whose export gate
            // can be bypassed (an unobservable source namespace), so it cannot
            // read `NamespaceImports::by_name`.
            imports.forced_rows().any(|row| {
                tcl_syntax::glob::string_match(&row.imp.tail_pattern, name)
                    && (!self.observable.contains(
                        unroot_rooted_key(&row.imp.source_ns).unwrap_or(&row.imp.source_ns),
                    ) || row.exported.covers(name))
                    && self.alias_live_at(ns, &row.imp.source_ns, name, row.site(), call)
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
            let (prefix, tail) = tcl_syntax::naming::key_holder_and_tail(cand);
            tail == name
                && self.forced_shadow_at(if prefix.is_empty() { "::" } else { prefix }, name, call)
        })
    }

    /// Every removal event bearing on the alias `importing_ns` took from
    /// `source_ns` for `name`, as seen from `query` — the cross-document
    /// half of [`crate::namespace_import::alias_live_at`]'s event log.
    ///
    /// Three kinds, and the ordering rule differs per kind because the
    /// underlying facts differ:
    ///
    /// - **`namespace forget`** and **a redefinition of the imported name**
    ///   are events on *this namespace's* slot. They are ordered only inside
    ///   one document: which of two files loads first is not a static fact
    ///   so an event in another document is passed unordered
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
                    unroot_rooted_key(src).unwrap_or(src)
                        == unroot_rooted_key(source_ns).unwrap_or(source_ns)
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
    /// [`crate::namespace_import::alias_live_at`].
    ///
    /// `install_at` is the import's own site. It is ordered against the call
    /// **only when the two share a document**: a byte offset in the importing
    /// file and a byte offset in the calling file are unrelated numbers, and
    /// comparing them lets a `namespace forget` in the caller revoke a
    /// cross-file import purely because its local offset happens to be the
    /// larger one. Unordered, the shared function keeps the
    /// alias — the same direction every other cross-file event takes here.
    ///
    /// Within one document the comparison is
    /// [`tcl_compiler::analyser::indirection::in_effect_within`] — the
    /// *identical* rule the same-document tier applies — stated over the
    /// call's own offset and enclosing body span ([`CallSite`]). A plain
    /// offset test is not good enough in either direction: it would leave a
    /// body-local call resolving through a top-level `namespace forget`
    /// written before it (the lenient direction), and, since installs are
    /// order-gated too, it would drop the alias of every proc body calling a
    /// name its own file imports further down (the *un*safe direction, which
    /// is why the span is a required [`CallSite`] field rather than an
    /// optional refinement).
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
    /// non-`-force` `namespace import` aborts on.
    ///
    /// Oracle (9.0.4 / 8.6.14): with `::dst` already importing `p` from `::A`,
    /// a later unforced import of `p` from `::B` raises `can't import command
    /// "p": already exists` and leaves `namespace origin ::dst::p` → `::A::p`.
    ///
    /// **Both spellings of an import count on both sides**. Tcl installs one
    /// alias per name; whether the import that installed
    /// it was written as a glob or as an exact pattern is a fact about the
    /// *source text*, not about the command table. The index splits them —
    /// a glob pattern names no single command, so it becomes a
    /// [`WorkspaceGlobImport`] consulted per call, while an exact one becomes
    /// a fixed [`WorkspaceCommandLink`] — and asking each side only about its
    /// own kind would make the conflict rule directional: a glob import that
    /// had already bound the name would not conflict with a later exact
    /// import of it. One function, both tables, so neither caller can drift.
    ///
    /// Ordered within the import's own document only: two imports in
    /// different files have no static load order, and conflicting on a guess
    /// would drop an alias Tcl really installed. A **same-source** re-import
    /// is a silent no-op (oracle), never a conflict — which is also what stops
    /// an import from conflicting with itself.
    ///
    /// Within that document the order is
    /// [`crate::namespace_import::load_order`], shared with the same-document
    /// resolver's slot-log fold. A raw `other.at < site.at` is wrong for
    /// exactly the shape this whole family exists to model: a **body-local**
    /// import is not ordered against a top-level import of its own file by
    /// offset at all, because the file loads — running every top-level
    /// statement, imports included — before any body runs, so the top-level
    /// one owns the name however far below it is written (oracle transcript on
    /// `load_order`). An offset comparison sees `::A` written *later* and lets
    /// the body-local `::B` import install, so navigation answers a source
    /// the program never reaches.
    ///
    /// It is deliberately **not**
    /// [`tcl_compiler::analyser::indirection::in_effect_within`], the
    /// primitive the lifecycle check below runs on: that one is lenient about
    /// events in bodies that may never run, which is the safe direction for a
    /// *removal* and the unsafe one for a conflict — applied here it makes the
    /// two imports above cancel each other and the name resolve nowhere.
    /// [`crate::namespace_import::load_order`] carries the reasoning.
    ///
    /// Self-exclusion comes for free: an import's own key is never less than
    /// itself, so it cannot conflict with itself.
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
        // `admitting` applies exactly the two tests this arm needs — the tail
        // pattern covers `name`, and the export gate admits it — as a probe
        // rather than a scan of the namespace. A scan here would multiply with
        // `import_hop`'s own admission walk, since this runs once per row
        // `import_hop` admits.
        //
        // `other_exported.covers(name)` stays in the conjunction below: it is
        // redundant for the glob arm, but the exact arm chained onto it is
        // *not* filtered by the gate, so the test still has work to do there.
        let mut earlier = self
            .imports_by_ns
            .get(ns)
            .map(|imports| imports.admitting(name).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .map(|other| (other.imp.source_ns.as_str(), other.site(), &other.exported))
            .chain(
                self.exact_by_ns
                    .get(ns)
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                    .iter()
                    .filter(|other| other.gate.name == name)
                    .map(|other| (other.gate.source_ns.as_str(), other.site(), &other.exported)),
            );
        earlier.any(|(other_source, other_site, other_exported)| {
            self.order.cmp_run(other_site.point(), site.point()) == Some(std::cmp::Ordering::Less)
                && !ns_eq(other_source, source_ns)
                && other_exported.covers(name)
                && self.alias_live_at(ns, other_source, name, other_site, call)
        })
    }

    /// Whether an **exact** import link is still installed, with no call site
    /// to order against — the gate [`WorkspaceIndex::live_command_links`]
    /// applies.
    ///
    /// `namespace import ::src::p` produces a fixed `WorkspaceCommandLink`
    /// rather than a per-call glob lookup, so without this gate the export
    /// snapshot and the `-force` conflict would be the only things gating it:
    /// after `namespace forget ::src::p` — or `rename ::src::p {}` — the link
    /// would stay live and cross-document definition / references would still
    /// resolve `::dst::p`.
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
    /// half of the wildcard-import gate (real Tcl only imports names a source
    /// namespace has actually exported — `Tcl_Export`, `tclNamesp.c`), taken
    /// per import site rather than against the workspace's final export state.
    ///
    /// Delegates to [`crate::namespace_import::exported_at_import_site`] — the
    /// same function the same-document resolver
    /// (`definition::exported_original_slot_at_import`) calls, so the two tiers cannot
    /// disagree about what an import site sees. The only tier-specific part
    /// is which events are *ordered* against the import: those in the
    /// import's own document, compared by offset. An export in another
    /// document has no static order relative to this import (nothing fixes
    /// which file loads first), so it is passed unordered and the shared
    /// function abstains toward continuing to resolve.
    fn exports_name_at(&self, ns: &str, name: &str, site: ImportSite<'_>) -> bool {
        let mut found = None;
        for subject in &self.original_subjects {
            let scope = tcl_compiler::signature_scan::scope::SignatureNamespaceScope::C(
                subject.slot().namespace.clone(),
            );
            if subject.slot().simple.try_utf8().ok() != Some(name)
                || !scope.display().is_some_and(|label| ns_eq(&label, ns))
            {
                continue;
            }
            if found.is_some_and(
                |previous: &tcl_compiler::signature_scan::scope::SignatureSourceCommand| {
                    previous.slot() != subject.slot() || previous.policy() != subject.policy()
                },
            ) {
                return false;
            }
            found = Some(*subject);
        }
        if let Some(subject) = found {
            if self.exports_by_ns.values().flatten().any(|event| {
                event.unknown
                    && self
                        .order
                        .has_run(
                            RunPoint {
                                uri: &event.uri,
                                at: event.at,
                                enclosing_body: event.enclosing_body,
                            },
                            site.point(),
                        )
                        .unwrap_or(true)
            }) {
                return false;
            }
            return crate::namespace_import::exported_original_at_import_site(
                self.exports_by_ns.values().flatten().filter_map(|event| {
                    Some((
                        event.original.as_ref()?,
                        RunPoint {
                            uri: &event.uri,
                            at: event.at,
                            enclosing_body: event.enclosing_body,
                        },
                    ))
                }),
                subject.slot(),
                subject.policy(),
                &self.order,
                site.point(),
            ) == ExportVerdict::Exported;
        }
        self.exports_by_ns.get(ns).is_some_and(|exports| {
            exported_at_site(
                exports
                    .iter()
                    .copied()
                    .filter(|event| event.original.is_none() && !event.unknown),
                name,
                &self.order,
                site.point(),
            )
        })
    }
}

/// A namespace's indexed export rows as [`crate::namespace_import::ExportEvent`]s
/// — the one place `WorkspaceNamespaceExport` is turned into one.
///
/// No enclosing-body span: the index stores none per export row, so every
/// consumer ranks a `-clear` against a pattern by position alone. Symmetric,
/// and the safe direction — a body-local export the tombstone cannot be proven
/// to follow keeps the name exported.
fn export_events<'e>(
    exports: impl Iterator<Item = &'e WorkspaceNamespaceExport>,
) -> impl Iterator<Item = crate::namespace_import::ExportEvent<'e>> {
    exports.map(|e| crate::namespace_import::ExportEvent {
        pattern: e.pattern.as_str(),
        clears: e.clears,
        at: RunPoint {
            uri: e.uri.as_str(),
            at: e.at,
            enclosing_body: e.enclosing_body,
        },
    })
}

/// [`crate::namespace_import::exported_at_import_site`] over a namespace's
/// indexed export rows.
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
    let mut events = export_events(exports);
    crate::namespace_import::exported_at_import_site(&mut events, name, order, site)
}

/// The workspace's `namespace export` records, plus the run order and the
/// observable-namespace set needed to read them — an **owned**, self-contained
/// [`crate::namespace_import::NamespaceExportOracle`] the single-document tier
/// can borrow.
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
    original_exports: Vec<WorkspaceNamespaceExport>,
    original_declarations: Vec<tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
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
        let mut subject = None;
        for declaration in &self.original_declarations {
            let Ok(tail) = declaration.slot().simple.try_utf8() else {
                continue;
            };
            let scope = tcl_compiler::signature_scan::scope::SignatureNamespaceScope::C(
                declaration.slot().namespace.clone(),
            );
            if tail != name
                || !scope
                    .display()
                    .is_some_and(|label| ns_eq(&label, source_ns))
            {
                continue;
            }
            if subject.is_some_and(
                |previous: &tcl_compiler::signature_scan::scope::SignatureSourceCommand| {
                    previous.slot() != declaration.slot()
                        || previous.policy() != declaration.policy()
                },
            ) {
                return ExportVerdict::Unknown;
            }
            subject = Some(declaration);
        }
        if let Some(subject) = subject {
            return self.exported_at_original(subject.slot(), subject.policy(), import_site);
        }
        // Explicit lexical-only authored inventories remain advice. Native
        // source records without an original publication do not enter this path.
        let ns = unroot_rooted_key(source_ns).unwrap_or(source_ns);
        if !self.observable.contains(ns) {
            return ExportVerdict::Unknown;
        }
        let exports = self
            .exports_by_ns
            .get(ns)
            .into_iter()
            .flatten()
            .filter(|event| event.original.is_none() && !event.unknown);
        if exported_at_site(exports, name, &self.order, import_site) {
            ExportVerdict::Exported
        } else {
            ExportVerdict::Unknown
        }
    }

    fn exported_at_original(
        &self,
        slot: &tcl_core_types::ByteCommandSlot,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        import_site: RunPoint<'_>,
    ) -> ExportVerdict {
        if self.original_exports.iter().any(|event| {
            event.unknown
                && self
                    .order
                    .has_run(
                        RunPoint {
                            uri: &event.uri,
                            at: event.at,
                            enclosing_body: event.enclosing_body,
                        },
                        import_site,
                    )
                    .unwrap_or(true)
        }) {
            return ExportVerdict::Unknown;
        }
        crate::namespace_import::exported_original_at_import_site(
            self.original_exports.iter().filter_map(|event| {
                Some((
                    event.original.as_ref()?,
                    RunPoint {
                        uri: &event.uri,
                        at: event.at,
                        enclosing_body: event.enclosing_body,
                    },
                ))
            }),
            slot,
            policy,
            &self.order,
            import_site,
        )
    }
}

/// The namespace an imported name is installed *into*, read off the link's
/// `linked_qname` (`::app::helper` → `::app`, a global-level import → `::`).
fn importing_namespace_of(linked_qname: &str) -> &str {
    let (ns, _) = tcl_syntax::naming::key_holder_and_tail(linked_qname);
    if ns.is_empty() { "::" } else { ns }
}

/// The source namespace an import pattern names, reading the empty prefix a
/// global-rooted pattern (`::p`, `::*`) splits to as the global namespace it
/// actually is.
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
    unroot_rooted_key(a).unwrap_or(a) == unroot_rooted_key(b).unwrap_or(b)
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
/// it and leaves calls before it alone — so the resolver needs the call's own
/// position.
///
/// Carries the call's own **enclosing body span** as well, for the same reason
/// [`WorkspaceGlobImport::enclosing_body`] exists on the import side: ordering
/// an import against a call is not a plain offset compare. A call inside a
/// proc body observes every top-level statement of its own file, wherever
/// written, because the whole file loads before any body runs — so a body-local
/// call of a name imported further down the same file still resolves, and a
/// top-level `namespace forget` written after such a body does revoke it
/// too. Comparing offsets alone gets the first of those wrong once installs
/// are order-gated, which is why the span is required rather than optional.
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
        analyse_as(
            source,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        )
    }

    fn analyse_as(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, dialect.name).clone()
    }

    #[test]
    fn original_declaration_inventory_retains_bytes_and_document_ownership() {
        // Proof naming.workspace.original-declaration-transport:
        // docs/design/analysis/name-resolution-proofs/workspace-original-declaration-transport.md
        // Implementation contract: opaque naming units, declaration metadata
        // and workspace document ownership survive removal of display maps.
        // Native naming outcomes are established by the separate byte probes.
        let source = r"proc p\uD800 {} {return FIRST}
proc p\uD801 {} {return SECOND}
p\uD800
p\uD801";
        let mut analysis = analyse(source);
        assert_eq!(analysis.original_procedure_declarations().count(), 2);
        analysis.all_procs.clear();
        analysis.superseded_procs.clear();
        let mut index = WorkspaceIndex::from_documents([
            ("file:///first.tcl", &analysis),
            ("file:///second.tcl", &analysis),
        ]);
        let declarations = index.original_procedure_declarations().collect::<Vec<_>>();
        assert_eq!(declarations.len(), 4);
        let input = tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
            declarations[0].1.name_input().clone(),
        );
        let lookup = tcl_compiler::signature_scan::scope::SignatureSourceLookup::from_input(
            analysis.original_namespace_scope_at(0).unwrap().clone(),
            &input,
        )
        .unwrap();
        let candidates = index.original_procedure_candidates(&lookup);
        assert_eq!(candidates.len(), 2);
        assert!(
            candidates
                .iter()
                .all(|(_, declaration)| { declaration.name_input().bytes() == b"p\xed\xa0\x80" })
        );
        for uri in ["file:///first.tcl", "file:///second.tcl"] {
            let entries = declarations
                .iter()
                .filter(|(owner, _)| *owner == uri)
                .map(|(_, declaration)| *declaration)
                .collect::<Vec<_>>();
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].name_input().bytes(), b"p\xed\xa0\x80");
            assert_eq!(entries[1].name_input().bytes(), b"p\xed\xa0\x81");
            assert_ne!(entries[0].name().slot(), entries[1].name().slot());
        }
        assert_eq!(
            index
                .invocations()
                .filter(|invocation| {
                    invocation
                        .original_name_input
                        .as_ref()
                        .is_some_and(|input| input.bytes() == b"p\xed\xa0\x80")
                })
                .count(),
            2
        );
        index.remove_document("file:///first.tcl");
        assert!(
            index
                .original_procedure_declarations()
                .all(|(uri, _)| uri == "file:///second.tcl")
        );
        index.add_document("file:///first.tcl", &analyse("proc plain {} {}"));
        assert_eq!(index.original_procedure_declarations().count(), 3);
        assert!(
            index
                .original_procedure_declarations()
                .filter(|(uri, _)| *uri == "file:///first.tcl")
                .all(|(_, declaration)| declaration.name_input().bytes() == b"plain")
        );
    }

    #[test]
    fn original_requirement_graph_keeps_constraints_preference_and_unknowns() {
        // Implementation contract: naming.workspace.original-package-requirement-transport
        // docs/design/analysis/name-resolution-proofs/workspace-original-package-requirement-transport.md

        use crate::package_resolver::{PackagePrefer, PackageRequirementAdviceKey};
        let source = "package require -exact p\\uD800 1.2\npackage prefer latest\npackage require p\\uD800 1.0 2.0\npackage require $computed\n";
        let analysis = analyse(source);
        let mut index = WorkspaceIndex::from_documents([("file:///library.tcl", &analysis)]);
        let requirements = index
            .original_package_requirement_advice_for("file:///library.tcl", PackagePrefer::Stable);
        assert_eq!(requirements.len(), 3);
        assert_eq!(
            requirements[0].key().original().unwrap().bytes(),
            b"p\xed\xa0\x80"
        );
        assert_eq!(requirements[0].requirements(), &["1.2"]);
        assert!(requirements[0].exact());
        assert_eq!(requirements[0].prefer(), PackagePrefer::Stable);
        assert_eq!(requirements[1].key(), requirements[0].key());
        assert_eq!(requirements[1].requirements(), &["1.0", "2.0"]);
        assert!(!requirements[1].exact());
        assert_eq!(requirements[1].prefer(), PackagePrefer::Latest);
        assert_eq!(requirements[2].key(), &PackageRequirementAdviceKey::Unknown);
        let edges = vec![
            (
                "file:///library.tcl".to_owned(),
                "file:///consumer.tcl".to_owned(),
            ),
            (
                "file:///consumer.tcl".to_owned(),
                "file:///library.tcl".to_owned(),
            ),
        ];
        let records = std::collections::HashMap::from([(
            "file:///library.tcl".to_owned(),
            requirements.clone(),
        )]);
        assert_eq!(
            crate::source_graph::ancestor_requirements("file:///consumer.tcl", &edges, &records),
            requirements
        );
        assert!(
            index
                .original_package_requirement_advice_for(
                    "file:///library.tcl",
                    PackagePrefer::Latest
                )
                .iter()
                .all(|required| required.prefer() == PackagePrefer::Latest)
        );
        index.remove_document("file:///library.tcl");
        assert!(
            index
                .original_package_requirement_advice_for(
                    "file:///library.tcl",
                    PackagePrefer::Stable
                )
                .is_empty()
        );
    }

    #[test]
    fn literal_colon_namespace_keys_keep_definitions_and_variables_distinct() {
        let nested = analyse("namespace eval : {proc p {} {}; variable v 1}\n");
        let global = analyse("proc :p {} {}\nvariable :v 2\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///nested.tcl", &nested),
            ("file:///global.tcl", &global),
        ]);
        let nested_proc = index.proc_definitions_qualified(":::::p", "");
        let global_proc = index.proc_definitions_qualified(":::p", "");
        assert_eq!(nested_proc.len(), 1);
        assert_eq!(global_proc.len(), 1);
        assert_eq!(nested_proc[0].uri, "file:///nested.tcl");
        assert_eq!(global_proc[0].uri, "file:///global.tcl");
        let nested_var = index.variable_definitions_qualified(":::::v", "");
        assert_eq!(nested_var.len(), 1);
        assert_eq!(nested_var[0].uri, "file:///nested.tcl");
        let global_var = index.variable_definitions_qualified(":::v", "");
        assert_eq!(global_var.len(), 1);
        assert_eq!(global_var[0].uri, "file:///global.tcl");
        assert_eq!(index.documents_in_namespace(":::"), ["file:///nested.tcl"]);
        assert_eq!(index.documents_in_namespace("::"), ["file:///global.tcl"]);
        let namespaces = index.observable_namespaces();
        assert!(namespaces.contains(":"));
        assert!(namespaces.contains(""));
    }

    #[test]
    fn literal_colon_import_sources_survive_workspace_ingestion() {
        let source = "namespace eval : {namespace eval src {proc p {} {}; namespace export p}; namespace import src::p; p}\n";
        let analysis = analyse(source);
        let index = WorkspaceIndex::from_documents([("file:///colon-import.tcl", &analysis)]);
        let link = index
            .command_links()
            .find(|link| link.linked_qname == ":::::p")
            .unwrap();
        assert_eq!(link.target_qname, ":::::src::p");
        assert_eq!(link.import_gate.as_ref().unwrap().source_ns, ":::::src");
        assert!(
            index
                .command_links()
                .all(|link| link.target_qname != "::src::p")
        );
        assert!(
            index
                .linked_invocations_of(":::::src::p", "")
                .iter()
                .any(|invocation| invocation.name == "p")
        );
    }

    #[test]
    fn literal_colon_alias_targets_survive_workspace_and_indirection() {
        let source = "proc :target {} {}; interp alias {} :alias {} :target; :alias\n";
        let analysis = analyse(source);
        let index = WorkspaceIndex::from_documents([("file:///colon-alias.tcl", &analysis)]);
        let link = index
            .command_links()
            .find(|link| link.linked_qname == ":::alias")
            .unwrap();
        assert_eq!(link.target_qname, ":::target");
        assert_eq!(
            link.linked_source_name
                .as_ref()
                .unwrap()
                .slot()
                .simple
                .as_bytes(),
            b":alias"
        );
        let selected = link.target_source_name.as_ref().unwrap();
        assert!(selected.slot().namespace.is_root());
        assert_eq!(selected.slot().simple.as_bytes(), b":target");
        assert_eq!(selected.source_spelling(), None);
        assert!(!index.command_link_map().contains_key(":alias"));
        let hop = tcl_compiler::analyser::indirection::walk(
            &analysis,
            ":alias",
            u32::MAX,
            &tcl_syntax::naming::normalise_qualified_name,
        )
        .unwrap();
        assert_eq!(hop.target, ":::target");
        assert_eq!(hop.target_source_name.as_ref(), Some(selected));
        assert_eq!(hop.lookup_spelling(), None);
        assert!(
            index
                .linked_invocations_of(":::target", "")
                .iter()
                .any(|invocation| invocation.name == ":alias")
        );
    }

    #[test]
    fn alias_target_navigation_uses_selected_global_or_caller_context() {
        // native_alias_target_context.tcl: C aliases select ROOT and Jim
        // aliases select LOCAL from the same original relative target operand.
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let alias = if engine == "jimtcl" {
                "alias contextual target"
            } else {
                "interp alias {} contextual {} target"
            };
            let source = format!(
                "proc target {{}} {{return ROOT}}; {alias}; namespace eval N {{proc target {{}} {{return LOCAL}}; contextual}}\n"
            );
            let analysis = Analyser::new().analyse(&source, engine).clone();
            let call = analysis
                .command_invocations
                .iter()
                .find(|call| call.name == "contextual" && call.lookup.is_execution_site())
                .unwrap_or_else(|| panic!("{engine}: missing original caller"));
            let reference = call
                .resolved_command_reference
                .as_ref()
                .unwrap_or_else(|| panic!("{engine}: missing positioned caller receipt"));
            let selected = reference
                .linked_definition()
                .or_else(|| reference.definition())
                .unwrap_or_else(|| panic!("{engine}: missing selected target allocation"));
            let target = if engine == "jimtcl" {
                "::N::target"
            } else {
                "::target"
            };
            assert_eq!(selected.allocation().command, target, "{engine}");
            let index = WorkspaceIndex::from_documents([("file:///alias-context.tcl", &analysis)]);
            assert!(
                index
                    .linked_invocations_of(target, "")
                    .iter()
                    .any(|call| call.name == "contextual"),
                "{engine}"
            );
        }
    }

    #[test]
    fn positioned_import_definition_keeps_linked_and_editable_views_separate() {
        let source = "namespace eval origin {proc helper {} {return ORIGIN}; namespace export helper}\nnamespace eval app {namespace import ::origin::helper; helper}\n";
        let analysis = analyse(source);
        let index = WorkspaceIndex::from_documents([("file:///imports.tcl", &analysis)]);
        let call = analysis
            .command_invocations
            .iter()
            .find(|invocation| invocation.name == "helper")
            .unwrap();
        assert!(call.resolved_definition.is_some());
        assert!(!call.resolved_user_definition);
        assert_eq!(index.linked_invocations_of("::origin::helper", "").len(), 1);
        assert!(index.invocations_of("::origin::helper", "").is_empty());
        assert!(index.invocations_of("::app::helper", "").is_empty());
    }

    #[test]
    fn positioned_definition_precedes_later_forced_import_assistance() {
        let source = "namespace eval origin {proc helper {} {return ORIGIN}; namespace export helper}\nnamespace eval app {proc helper {} {return LOCAL}; helper; namespace import -force ::origin::helper; helper}\n";
        let analysis = analyse(source);
        let index = WorkspaceIndex::from_documents([("file:///timeline.tcl", &analysis)]);
        assert_eq!(index.invocations_of("::app::helper", "").len(), 1);
        assert_eq!(index.linked_invocations_of("::app::helper", "").len(), 1);
        assert_eq!(index.linked_invocations_of("::origin::helper", "").len(), 1);
        assert!(index.invocations_of("::origin::helper", "").is_empty());
    }

    #[test]
    fn subclass_provided_methods_matches_the_per_class_linearisation() {
        // The bulk fold must agree with the per-class `class_linearisation`
        // walk (the per-class form rebuilds the edge maps every call —
        // O(classes²) — which is what times out the Performance suite).  The
        // fixture
        // covers the shapes the fold has to get right: a mixin-provided
        // template method, an unresolved base, and a private member that
        // must not travel.
        let base = analyse(
            "oo::class create ::Fmt { method run {} { my Render } }\n\
             oo::class create ::Mix { method Render {} { return m } }\n",
        );
        let sub = analyse(
            "oo::class create ::Html { superclass ::Fmt\n    method Render {} { return h } }\n\
             oo::class create ::Md { superclass ::Fmt\n    mixin ::Mix }\n\
             oo::class create ::Secretive { superclass ::Fmt\n\
             \x20   private method Hidden {} { return no } }\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///base.tcl", &base),
            ("file:///sub.tcl", &sub),
        ]);
        let map = index.subclass_provided_methods();
        let fmt = map.get("::Fmt").expect("::Fmt has descendants");
        assert!(fmt.contains("Render"), "subclass-written: {fmt:?}");
        assert!(
            fmt.contains("run"),
            "the base's own method rides along harmlessly: {fmt:?}"
        );
        assert!(
            !fmt.contains("Hidden"),
            "private members never travel: {fmt:?}"
        );
        // The mixin is an ancestor on ::Md's linearisation, so it gains
        // ::Md's dispatchable surface exactly as a superclass would.
        let mix = map.get("::Mix").expect("::Mix hosts a mixin consumer");
        assert!(mix.contains("run"), "{mix:?}");
        // Agreement with the per-class walk: every key's entry is what the
        // slow form computes for the same class set.
        for (ancestor, methods) in &map {
            let mut slow: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            for d in index.live_classes().map(|c| c.qualified_name.clone()) {
                let lin = index.class_linearisation(&d);
                if lin.iter().all(|c| c == &d)
                    || !lin.iter().any(|c| c == ancestor)
                    || &d == ancestor
                {
                    continue;
                }
                for x in &lin {
                    slow.extend(
                        index
                            .effective_members(x)
                            .into_iter()
                            .filter(|m| m.method.kind != CLASS_METHOD && !m.method.private)
                            .map(|m| m.name.to_owned()),
                    );
                }
            }
            assert_eq!(methods, &slow, "bulk and per-class disagree on {ancestor}");
        }
    }

    #[test]
    fn a_receivers_export_stub_decides_the_inherited_members_visibility() {
        // `export` / `unexport` accept a name their class does
        // not define, creating a body-less table entry whose only content is
        // the flag; the flag comes from the most specific spine class that
        // *mentions* the member while the implementation still comes from the
        // first class that declares a body.  Oracle, byte-identical on tclsh
        // 8.6.16 and 9.0.4:
        //   oo::class create ::Base  { method tick {} { return base } }
        //   oo::class create ::Child { superclass ::Base }
        //   oo::define ::Child { unexport tick }
        //   [::Child new] tick  ->  unknown method "tick": must be destroy
        //   [::Base  new] tick  ->  base
        let base = analyse("oo::class create ::Base { method tick {} { return 1 } }\n");
        let child = analyse(
            "oo::class create ::Child { superclass ::Base }\noo::define ::Child { unexport tick }\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///base.tcl", &base),
            ("file:///child.tcl", &child),
        ]);
        assert!(
            index
                .method_dispatch_chain("::Child", "tick", MethodAccess::External)
                .is_empty(),
            "the receiver's own unexport suppresses the inherited implementation",
        );
        // `my` ignores the export flag entirely — it still reaches `Base`.
        assert_eq!(
            index
                .method_dispatch_chain("::Child", "tick", MethodAccess::Internal)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///base.tcl"],
            "internal dispatch is not gated on the export flag",
        );
        // TN: the provider's own receiver is untouched by the subclass's flip.
        assert_eq!(
            index
                .method_dispatch_chain("::Base", "tick", MethodAccess::External)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///base.tcl"],
        );
    }

    #[test]
    fn a_receivers_export_stub_revives_an_unexported_inherited_member() {
        // The mirror direction, which a provider-only reading gets wrong the
        // other way.  Oracle, byte-identical on tclsh 8.6.16 and 9.0.4:
        //   oo::class create ::Base  { method tock {} { return b3 } ; unexport tock }
        //   oo::class create ::Child { superclass ::Base ; export tock }
        //   [::Child new] tock  ->  b3
        //   [::Base  new] tock  ->  unknown method "tock": must be destroy
        let base = analyse(
            "oo::class create ::Base { method tock {} { return 1 }\n    unexport tock\n}\n",
        );
        let child = analyse("oo::class create ::Child { superclass ::Base\n    export tock\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///base.tcl", &base),
            ("file:///child.tcl", &child),
        ]);
        assert_eq!(
            index
                .method_dispatch_chain("::Child", "tock", MethodAccess::External)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///base.tcl"],
            "the receiver's export revives the inherited implementation",
        );
        assert!(
            index
                .method_dispatch_chain("::Base", "tock", MethodAccess::External)
                .is_empty(),
            "the provider itself stays unexported",
        );
    }

    #[test]
    fn a_mixins_unexport_does_not_suppress_the_spine_provider() {
        // C enters each mixin with a fresh copy of the dispatch flags, so a
        // mixin's `unexport` empties only its own branch.  Oracle,
        // byte-identical on tclsh 8.6.16 and 9.0.4:
        //   oo::class create ::Mix { method tick {} { return mix } ; unexport tick }
        //   oo::class create ::Child { superclass ::Base ; mixin ::Mix }
        //   [::Child new] tick  ->  base
        let base = analyse("oo::class create ::Base { method tick {} { return 1 } }\n");
        let mix =
            analyse("oo::class create ::Mix { method tick {} { return 2 }\n    unexport tick\n}\n");
        let child = analyse("oo::class create ::Child { superclass ::Base\n    mixin ::Mix\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///base.tcl", &base),
            ("file:///mix.tcl", &mix),
            ("file:///child.tcl", &child),
        ]);
        assert_eq!(
            index
                .method_dispatch_chain("::Child", "tick", MethodAccess::External)
                .iter()
                .map(|c| c.uri.as_str())
                .collect::<Vec<_>>(),
            ["file:///base.tcl"],
            "the mixin's unexport must not suppress the superclass spine",
        );
    }

    #[test]
    fn a_mixin_branch_decides_its_own_providers_visibility_across_files() {
        // A mixin that inherits the member from its
        // own superclass and unexports the name empties *that branch* only —
        // C enters each mixin with a fresh copy of the dispatch flags — so the
        // receiver's spine still answers.  Oracle, byte-identical on tclsh
        // 8.6.16 and 9.0.4:
        //   oo::class create ::MChild { superclass ::MBase } ; unexport m
        //   oo::class create ::D { superclass ::A ; mixin ::MChild }
        //   [::D new] m  ->  A-m   (not MBase-m)
        let a = analyse("oo::class create ::A { method m {} { return 1 } }\n");
        let mbase = analyse("oo::class create ::MBase { method m {} { return 2 } }\n");
        let mchild = analyse("oo::class create ::MChild { superclass ::MBase\n    unexport m\n}\n");
        let d = analyse("oo::class create ::D { superclass ::A\n    mixin ::MChild\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///mbase.tcl", &mbase),
            ("file:///mchild.tcl", &mchild),
            ("file:///d.tcl", &d),
        ]);
        assert_eq!(
            index
                .method_dispatch_chain("::D", "m", MethodAccess::External)
                .first()
                .map(|c| c.uri.as_str()),
            Some("file:///a.tcl"),
            "the suppressed mixin branch must not answer; the spine must",
        );
        // TN for the permission the family pass reads: `::D` dispatches into
        // `::A`'s family, not `::MBase`'s.
        assert!(
            index
                .external_dispatch_receivers(["::D"], &["::A"], "m", false)
                .contains("::D"),
        );
        assert!(
            index
                .external_dispatch_receivers(["::D"], &["::MBase"], "m", false)
                .is_empty(),
        );
    }

    #[test]
    fn a_live_mixin_branch_outranks_the_spine_across_files() {
        // The control: with the branch left public the mixin's inherited
        // `::MBase::m` is what `[::D new] m` enters (tclsh 8.6.16 / 9.0.4 ->
        // MBase-m), so the capture belongs to *its* family instead.
        let a = analyse("oo::class create ::A { method m {} { return 1 } }\n");
        let mbase = analyse("oo::class create ::MBase { method m {} { return 2 } }\n");
        let mpub = analyse("oo::class create ::MPub { superclass ::MBase }\n");
        let d = analyse("oo::class create ::D { superclass ::A\n    mixin ::MPub\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///a.tcl", &a),
            ("file:///mbase.tcl", &mbase),
            ("file:///mpub.tcl", &mpub),
            ("file:///d.tcl", &d),
        ]);
        assert_eq!(
            index
                .method_dispatch_chain("::D", "m", MethodAccess::External)
                .first()
                .map(|c| c.uri.as_str()),
            Some("file:///mbase.tcl"),
        );
        assert!(
            index
                .external_dispatch_receivers(["::D"], &["::A"], "m", false)
                .is_empty(),
            "a receiver dispatching into another family must not join this one",
        );
    }

    #[test]
    fn a_mixin_branch_revives_its_own_bases_unexported_member_across_files() {
        // The mirror direction inside a branch: `::NBase` unexports its own
        // `n`, the mixin `::NChild` exports the inherited name, and tclsh
        // 8.6.16 / 9.0.4 run `::NBase`'s body ahead of the spine's public
        // `::A2::n`.
        let a2 = analyse("oo::class create ::A2 { method n {} { return 1 } }\n");
        let nbase =
            analyse("oo::class create ::NBase { method n {} { return 2 }\n    unexport n\n}\n");
        let nchild = analyse("oo::class create ::NChild { superclass ::NBase\n    export n\n}\n");
        let d = analyse("oo::class create ::D { superclass ::A2\n    mixin ::NChild\n}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///a2.tcl", &a2),
            ("file:///nbase.tcl", &nbase),
            ("file:///nchild.tcl", &nchild),
            ("file:///d.tcl", &d),
        ]);
        assert_eq!(
            index
                .method_dispatch_chain("::D", "n", MethodAccess::External)
                .first()
                .map(|c| c.uri.as_str()),
            Some("file:///nbase.tcl"),
            "the branch's own export revives its base's body",
        );
    }

    #[test]
    fn cross_file_define_stub_retraction_removes_the_method_from_dispatch() {
        // A cross-file `oo::define` stub is already an
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
        // The predicate the in-document tier consults before
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
        // TP — the whole point of the class-side channel. Oracle,
        // byte-identical on tclsh 9.0.4 and 8.6.14:
        //   a.tcl  oo::class create ::C { self { method cm {} { return cm } } }
        //          ::C cm  ->  cm
        //   b.tcl  oo::define ::C { self unexport cm }
        //          ::C cm  ->  unknown method "cm": must be create, destroy or new
        //          info object methods ::C -all -private  ->  … cm …
        // Without a class-side channel the flip has nowhere to travel, b.tcl's
        // `self unexport` is invisible and `::C cm` still resolves.
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
        // TN (CRITICAL FP guard). A class that defines the
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
        // TP — a same-file rename reaching the workspace. The rename happens inside
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

    /// TP: the `renamemethod` sits in a **cross-file**
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

    /// TP: **enumeration** joins the arrival channel too.  The defining record
    /// still declares the member as `old`, so reading the raw
    /// `WorkspaceClass::methods` table directly advertises the pre-rename name
    /// even where the resolution join is applied.
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

    #[test]
    fn symbol_exclusions_apply_before_the_limit_and_member_fold() {
        let orphan =
            analyse("proc orphan_first {} {}\noo::define ::C { renamemethod old arrived }\n");
        let live = analyse("proc live_second {} {}\noo::class create ::C { method old {} {} }\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///orphan.tcl", &orphan),
            ("file:///live.tcl", &live),
        ]);

        let capped = index.symbols_matching_excluding("", 1, ["file:///orphan.tcl"]);
        assert_eq!(
            capped.first().map(|symbol| symbol.name.as_str()),
            Some("live_second"),
            "an excluded document must not consume the result capacity",
        );
        let methods = index.symbols_matching_excluding("old", 10, ["file:///orphan.tcl"]);
        assert!(methods.iter().any(|symbol| {
            symbol.kind == WorkspaceSymbolKind::Method
                && symbol.name == "old"
                && symbol.uri == "file:///live.tcl"
        }));
        assert!(
            methods
                .iter()
                .all(|symbol| symbol.uri != "file:///orphan.tcl"),
            "an excluded arrival must neither re-key nor locate a live member",
        );
    }

    #[test]
    fn symbol_exclusions_rebuild_duplicate_retractions_from_live_documents() {
        let class = analyse("oo::class create ::C { method old {} {} }\n");
        let orphan = analyse("oo::define ::C { renamemethod old orphan_arrival }\n");
        let live = analyse("oo::define ::C { renamemethod old live_arrival }\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///base.tcl", &class),
            ("file:///a-orphan.tcl", &orphan),
            ("file:///b-live.tcl", &live),
        ]);

        let symbols = index.symbols_matching_excluding("arrival", 10, ["file:///a-orphan.tcl"]);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "live_arrival");
        assert_eq!(symbols[0].uri, "file:///b-live.tcl");
        assert!(
            index
                .symbols_matching_excluding("old", 10, ["file:///a-orphan.tcl"])
                .iter()
                .all(|symbol| symbol.kind != WorkspaceSymbolKind::Method),
            "the remaining live retraction must still retire the old member name",
        );
    }

    /// TN: a cross-file `deletemethod` removes the member from
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

    /// TN: with no retraction anywhere, enumeration is exactly
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

    /// TN: a `self renamemethod` moves the **class-object** side only.  An
    /// identically-named instance method keeps its own name, which is the same
    /// side-scoping the tombstone channel already enforces for resolution.
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
        // `method m {} {}; export m; unexport m` leaves `m` unexported in real
        // Tcl ([L1 new] m -> unknown method, 9.0.4 / 8.6.14) — but this chain
        // reads *any* `exports` entry as decisive, so recording the name in
        // both sets would make cross-file go-to-definition treat a
        // runtime-inaccessible method as public.  Covers the unwrapped
        // spelling and the `private` one, in both writer orders.
        for (body, dialect, callable) in [
            ("export m\nunexport m", "tcl8.6", false),
            ("unexport m\nexport m", "tcl8.6", true),
            // `private` is a 9.0-only member word.
            ("private export m\nprivate unexport m", "tcl9.0", false),
            ("private unexport m\nprivate export m", "tcl9.0", true),
        ] {
            let src = format!("oo::class create ::C {{ method m {{}} {{ return 1 }}\n{body} }}\n");
            let a = analyse_as(
                &src,
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
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

    /// A removal is scoped to the removed document.  Every other
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

    /// The remove-then-add every diagnostics publish performs must
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

    /// Removing a URI the index never held changes nothing.
    #[test]
    fn removing_an_unindexed_document_changes_nothing() {
        let a = analyse("proc a {} {}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        index.remove_document("file:///never-indexed.tcl");
        assert_eq!(index.procs().count(), 1);
    }

    /// Several analyses of one URI — one re-homed view per source-site
    /// namespace — accumulate under that URI, and one removal drops the whole
    /// set.
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
    fn settled_invocations_keep_a_definition_called_before_its_later_deletion() {
        // TP: the final command table has retired
        // `::foo::bar`, but `foo::caller` demonstrably ran before that
        // retirement.  The analyser's per-site resolution must outrank the
        // final-state name set for this one historical invocation.
        let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n}\nfoo::caller\nrename foo::bar {}\n";
        let call = u32::try_from(src.find("[bar]").expect("call") + 1).expect("offset");
        let analysis = analyse(src);
        let index = WorkspaceIndex::from_documents([("file:///life.tcl", &analysis)]);

        assert!(
            !index.defined_command_names(false).contains("foo::bar"),
            "the final-state cache must still hide the retired name",
        );
        assert!(
            index
                .invocations_of("::foo::bar", "")
                .iter()
                .any(|inv| inv.range.start() == call),
            "the earlier call must remain attached to the local definition",
        );
        assert!(
            index
                .invocations_of("::bar", "")
                .iter()
                .all(|inv| inv.range.start() != call),
            "the earlier call must not also settle to the global fallback",
        );
    }

    #[test]
    fn settled_invocations_fall_back_after_a_local_definition_is_deleted() {
        // FN guard: moving the execution point past the deletion changes the
        // same candidate list's winner to the global definition.
        let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    rename foo::bar {}\n    proc caller {} { return [bar] }\n}\nfoo::caller\n";
        let call = u32::try_from(src.find("[bar]").expect("call") + 1).expect("offset");
        let analysis = analyse(src);
        let index = WorkspaceIndex::from_documents([("file:///life.tcl", &analysis)]);

        assert!(
            index
                .invocations_of("::foo::bar", "")
                .iter()
                .all(|inv| inv.range.start() != call),
            "the deleted local definition must not retain the later call",
        );
        assert!(
            index
                .invocations_of("::bar", "")
                .iter()
                .any(|inv| inv.range.start() == call),
            "the later call must settle to the live global definition",
        );
    }

    #[test]
    fn a_later_alias_does_not_retarget_an_earlier_definition_call() {
        // FP/TN guard for the link-following reference view: final state has
        // re-established `target` as an alias of `replacement`, but the first
        // call ran while `target` still named its original proc.  A final-state
        // link must not rewrite that historical target.
        let src = "proc target {} {}\nproc replacement {} {}\ntarget\nrename target {}\ninterp alias {} target {} replacement\ntarget\n";
        let first = u32::try_from(src.find("\ntarget\n").expect("first call") + 1).expect("offset");
        let second =
            u32::try_from(src.rfind("\ntarget\n").expect("second call") + 1).expect("offset");
        let analysis = analyse(src);
        let index = WorkspaceIndex::from_documents([("file:///life.tcl", &analysis)]);

        let original = index.linked_invocations_of("::target", "");
        assert!(original.iter().any(|inv| inv.range.start() == first));
        assert!(original.iter().all(|inv| inv.range.start() != second));
        let replacement = index.linked_invocations_of("::replacement", "");
        assert!(replacement.iter().all(|inv| inv.range.start() != first));
        assert!(replacement.iter().any(|inv| inv.range.start() == second));
    }

    #[test]
    fn settled_class_invocations_are_position_sensitive_across_destruction() {
        // Type guard: classes install ordinary commands and use the same
        // generic per-site definition fact as procs.
        let before_src = "oo::class create Dog {}\nDog new\nrename Dog {}\n";
        let before_call = u32::try_from(before_src.find("Dog new").expect("call")).expect("offset");
        let before = analyse(before_src);
        let before_index = WorkspaceIndex::from_documents([("file:///before.tcl", &before)]);
        assert!(!before_index.defined_command_names(false).contains("Dog"));
        assert!(
            before_index
                .invocations_of("::Dog", "")
                .iter()
                .any(|inv| inv.range.start() == before_call),
            "a class call before destruction still reaches that class command",
        );

        let after_src = "oo::class create Dog {}\nrename Dog {}\nDog new\n";
        let after_call = u32::try_from(after_src.find("Dog new").expect("call")).expect("offset");
        let after = analyse(after_src);
        let after_index = WorkspaceIndex::from_documents([("file:///after.tcl", &after)]);
        assert!(
            after_index
                .invocations_of("::Dog", "")
                .iter()
                .all(|inv| inv.range.start() != after_call),
            "a destroyed class command must not regain the later call",
        );
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
        // A bare call reaches a namespaced proc in
        // *another* file via `namespace path`, while an unrelated file defines
        // the same simple name (which disables the bare-name fallback).
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
        // was. Same shape as the `interp alias` case above: the `OLD` word is
        // itself a first-class invocation, so
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

    // Cross-document wildcard-import resolution:
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
    fn colon_run_glob_import_drives_cross_file_navigation_references_and_w123() {
        // Mutation guard: `rsplit_once("::")` sees the final two
        // colons in `::src:::im*`, while the Tcl owner sees source `::src`
        // and tail glob `im*`.  The same fixture proves the indexed target,
        // linked cross-file reference, and analyser W123 suppression.
        let src = analyse(
            "namespace eval ::src {\n    proc image {} {}\n    namespace export image\n}\n",
        );
        let app_src = "namespace eval ::app {\n    namespace import ::src:::im*\n    proc run {} { image }\n}\n";
        let app = analyse(app_src);
        assert!(
            app.diagnostics
                .iter()
                .all(|d| d.code != tcl_core_types::DiagCode::W123),
            "imported image must suppress W123: {:?}",
            app.diagnostics
        );
        let index =
            WorkspaceIndex::from_documents([("file:///src.tcl", &src), ("file:///app.tcl", &app)]);
        let call = index.linked_invocations_of("::src::image", "file:///src.tcl");
        assert!(
            call.iter()
                .any(|inv| inv.uri == "file:///app.tcl" && inv.name == "image"),
            "glob import must link the cross-file call: {call:?}"
        );
        let direct = index.invocations_of("::src::image", "file:///src.tcl");
        assert!(
            direct.is_empty(),
            "direct view must retain the local imported name"
        );
        assert_eq!(
            index.resolve_wildcard_import(
                "helper",
                &["::app::helper".to_owned(), "::helper".to_owned()],
                call_from("file:///caller.tcl"),
            ),
            None
        );
    }

    #[test]
    fn non_suffix_glob_import_resolves_cross_file_reference() {
        let src = analyse(
            "namespace eval ::src {\n    proc helper {} {}\n    namespace export helper\n}\n",
        );
        let app = analyse(
            "namespace eval ::app {\n    namespace import ::src::he*r\n    proc run {} { helper }\n}\n",
        );
        let index =
            WorkspaceIndex::from_documents([("file:///src.tcl", &src), ("file:///app.tcl", &app)]);
        let resolved = index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_owned(), "::helper".to_owned()],
            call_from("file:///caller.tcl"),
        );
        assert_eq!(resolved.as_deref(), Some("::src::helper"));
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
        // TP — the georgtree_tclopt corpus shape: `tclopt.tcl` exports a
        // TclOO class and `examples/*.tcl` wildcard-imports and instantiates
        // it bare.  The shared cross-document mechanism
        // (`workspace_command_exists` / `defined_command_names`) covers
        // classes exactly like procs; pinned here as class-side coverage.
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
        // TP — `namespace import` binds to the
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
        // document, so filtering candidate glob imports by `g.uri == uri`
        // (the calling document) would never admit this shape.
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

    // Per-import-site export snapshots, cross-document tier.
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

    // The `source` graph orders what the file boundary cannot.
    //
    // Sourcing a file inlines its whole
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

    /// The installed server resolver's source-plan and inventory contract,
    /// with the file URI mapped to an explicit document filename.
    fn test_resolve(
        parent_uri: &str,
        _raw_path: &str,
        expression: Option<&tcl_compiler::auto_path_eval::OriginalSourcePathExpression>,
        source_offset: u32,
        raw_constants: &tcl_compiler::auto_path_eval::PathConstantAssignments,
        imported: &tcl_compiler::auto_path_eval::FoldedPathConstants,
    ) -> Option<String> {
        let parent = parent_uri.strip_prefix("file://")?;
        let dir = std::path::Path::new(parent).parent()?;
        let expression = expression?;
        if !expression.matches_assignments(raw_constants)
            || expression.source_span().start() > source_offset
            || source_offset >= expression.source_span().end()
        {
            return None;
        }
        let constants = tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports(
            raw_constants,
            Some(parent),
            imported,
        );
        let raw = expression.evaluate(Some(parent), &|name| {
            tcl_compiler::auto_path_eval::PathConstantLookup::path_constant(
                &constants.at(source_offset),
                name,
            )
        })?;
        let child = crate::source_graph::resolve_under(dir, &raw);
        Some(format!("file://{}", child.display()))
    }

    /// The server's constant folder, in miniature — the counterpart of
    /// [`test_resolve`], same URI-to-path convention.
    fn test_fold(
        uri: &str,
        writes: &tcl_compiler::auto_path_eval::PathConstantAssignments,
        imported: &tcl_compiler::auto_path_eval::FoldedPathConstants,
    ) -> tcl_compiler::auto_path_eval::FoldedPathConstants {
        let path = uri.strip_prefix("file://");
        tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports(writes, path, imported)
    }

    /// An index over `documents` with the `source`-path resolver installed —
    /// the shape the real server builds ([`WorkspaceIndex::set_source_resolver`]).
    fn sourced_index<'a>(
        documents: impl IntoIterator<Item = (&'a str, &'a AnalysisResult)>,
    ) -> WorkspaceIndex {
        let mut index = WorkspaceIndex::new();
        index.set_source_resolver(test_resolve, test_fold);
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
    fn retained_source_routes_share_original_plans_across_graph_consumers() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let app = analyse(
            "interp alias {} build {} file join /proj; package require Tk; package prefer latest; source [build lib.tcl]",
        );
        let child = analyse("proc leaf {} {}");
        let index = sourced_index([
            ("file:///proj/app.tcl", &app),
            ("file:///proj/lib.tcl", &child),
        ]);
        let edges = index.resolved_source_edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].child, "file:///proj/lib.tcl");
        assert_eq!(
            index.source_ancestor_package_requires_from_resolved("file:///proj/lib.tcl"),
            vec!["Tk".to_owned()]
        );
        assert!(index.source_ancestor_prefers_latest_from_resolved("file:///proj/lib.tcl"));
        assert_eq!(
            index
                .source_seed_map(test_resolve)
                .get("file:///proj/lib.tcl"),
            Some(&std::collections::BTreeSet::from(["::".to_owned()]))
        );
        let shadow =
            analyse("proc file {args} {}; package require Tk; source [file join /proj lib.tcl]");
        let index = sourced_index([
            ("file:///proj/app.tcl", &shadow),
            ("file:///proj/lib.tcl", &child),
        ]);
        assert!(!index.sources().collect::<Vec<_>>().is_empty());
        assert!(index.resolved_source_edges().is_empty());
        assert!(index.source_seed_map(test_resolve).is_empty());
        assert!(
            index
                .source_ancestor_package_requires_from_resolved("file:///proj/lib.tcl")
                .is_empty()
        );
    }

    #[test]
    fn an_export_sourced_before_the_import_resolves() {
        // TP: `app.tcl` sources the export before the import, so the import
        // really did see it — resolution here rests on a proved order rather
        // than an abstention.
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
        // FP guard (CRITICAL) — the whole point of the
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
        // installed ranks no cross-document event — the foreign export counts
        // and the import resolves, whichever way the `source` statements are
        // written.
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

    // The `package require` half of the load order.
    //
    // Every test below builds the index with `WorkspaceIndex::from_documents`
    // — no `source` resolver at all — because the package half needs none: a
    // `package require NAME` names its provider through the index's own
    // `package provide` records.
    //
    // Oracle, byte-identical on tclsh 8.6.14 and 9.0.4, with `pkg/lib.tcl`
    // holding the provider and `pkg/pkgIndex.tcl` the usual
    // `[list source [file join $dir lib.tcl]]`:
    //
    //   package require mymod
    //   namespace eval ::mymod { namespace export -clear }
    //   namespace eval ::app { namespace import ::mymod::* }
    //     -> NO ALIAS (invalid command name "::app::helper")
    //
    //   namespace eval ::mymod { namespace export -clear }
    //   package require mymod
    //   namespace eval ::app { namespace import ::mymod::* }
    //     -> HELP
    //
    // …and the second script, byte-identical, answers NO ALIAS when some
    // other file required `mymod` before it ran.  That flip is the whole
    // reason the second shape abstains.

    /// The provider document every `package require` order test shares: it
    /// defines and exports `helper`, and provides the package.
    fn package_provider() -> AnalysisResult {
        analyse(
            "namespace eval ::mymod { proc helper {} { return HELP }\n namespace export helper }\npackage provide mymod 1.0\n",
        )
    }

    /// Whether `helper` still resolves through `::app`'s wildcard import in
    /// `index` — the question every test below asks.
    fn helper_resolves(index: &WorkspaceIndex) -> Option<String> {
        index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_string(), "::helper".to_string()],
            call_from("file:///p/caller.tcl"),
        )
    }

    #[test]
    fn a_clear_after_a_package_require_revokes_the_providers_export() {
        // TP (CRITICAL) — the movement the package half exists for. `app.tcl`
        // requires the package (so `lib.tcl` has run), clears `::mymod`'s
        // exports, and only then imports. Without the package order the
        // foreign export would count and the local `-clear` would revoke
        // nothing, so `helper` would resolve; real Tcl installs no alias at
        // all.
        let lib = package_provider();
        let app = analyse(
            "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///p/lib.tcl", &lib),
            ("file:///p/app.tcl", &app),
        ]);
        let resolved = helper_resolves(&index);
        assert!(
            resolved.is_none(),
            "the `-clear` provably ran after the provider's export and before the import: {resolved:?}",
        );
        // Non-vacuity: with the `-clear` removed, the same workspace resolves,
        // so the assertion above is the ordering's doing.
        let app_no_clear = analyse(
            "package require mymod\nnamespace eval ::app { namespace import ::mymod::* }\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///p/lib.tcl", &lib),
            ("file:///p/app.tcl", &app_no_clear),
        ]);
        assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
    }

    #[test]
    fn abstention_2_a_statement_above_the_require_stays_unordered() {
        // TN (CRITICAL) — uncertainty 2, the package may already be loaded,
        // in which case the `package require` runs nothing at all and the
        // provider's statements are *older* than everything in this file.
        //
        // A `source` in this shape settles it — "an export sourced after the
        // import is not retroactive", the test above. A `package require`
        // must not, and the oracle is a file whose answer flips with nothing
        // in it changed. `imp_body.tcl`, byte for byte:
        //
        //   namespace eval ::mymod {}
        //   namespace eval ::app { namespace import ::mymod::* }
        //   package require mymod
        //
        //   sourced on its own            -> NO ALIAS
        //   sourced after `package require mymod` elsewhere -> HELP
        //
        // So the order says nothing and the pre-existing "abstain toward
        // answering" stands: the foreign export counts, `helper` resolves.
        let lib = package_provider();
        let app = analyse(
            "namespace eval ::mymod {}\nnamespace eval ::app { namespace import ::mymod::* }\npackage require mymod\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///p/lib.tcl", &lib),
            ("file:///p/app.tcl", &app),
        ]);
        assert_eq!(
            helper_resolves(&index).as_deref(),
            Some("::mymod::helper"),
            "a require never proves the provider had *not* yet run",
        );
        // …and the same shape with a `-clear` above the require is equally
        // unordered: run alone the clear hits an empty `::mymod` and the
        // require then loads the exporting file (oracle: HELP), run after
        // any other file required `mymod` the very same text revokes
        // (oracle: NO ALIAS).
        let app = analyse(
            "namespace eval ::mymod { namespace export -clear }\npackage require mymod\nnamespace eval ::app { namespace import ::mymod::* }\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///p/lib.tcl", &lib),
            ("file:///p/app.tcl", &app),
        ]);
        assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
    }

    #[test]
    fn abstention_1_two_documents_providing_one_package_order_nothing() {
        // TN — uncertainty 1, `auto_path` is mutable. Two indexed files
        // provide `mymod`; which one a `package require` runs is decided by
        // the runtime search-path order, and only one of them exports.
        // Oracle (8.6.14 / 9.0.4), two directories each holding a `mylib2
        // 1.0`: `lappend auto_path pkgA pkgB` gives `from A` with `p`
        // exported, `pkgB pkgA` gives `from B` with nothing exported. So no
        // edge, and the `-clear` that would otherwise revoke does not.
        let lib = package_provider();
        let other = analyse(
            "namespace eval ::mymod { proc helper {} { return OTHER } }\npackage provide mymod 1.0\n",
        );
        let app = analyse(
            "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///p/lib.tcl", &lib),
            ("file:///p/other/lib.tcl", &other),
            ("file:///p/app.tcl", &app),
        ]);
        assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
    }

    #[test]
    fn abstention_3_an_indexed_ifneeded_script_orders_nothing() {
        // TN — uncertainty 3, `ifneeded` bodies are arbitrary. Once the
        // workspace holds the package's own index script, the statements a
        // `package require mymod` runs are that script's business, and it may
        // load something else entirely. Oracle (8.6.14 / 9.0.4): one
        // `pkgIndex.tcl` whose body branches on the environment answers
        // `::c::p` -> `real` (with `namespace export p` run) or -> `stub`
        // (with nothing exported), for the same `package require`.
        let lib = package_provider();
        let pkg_index =
            analyse("package ifneeded mymod 1.0 [list source [file join $dir lib.tcl]]\n");
        let app = analyse(
            "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///p/lib.tcl", &lib),
            ("file:///p/pkgIndex.tcl", &pkg_index),
            ("file:///p/app.tcl", &app),
        ]);
        assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
    }

    #[test]
    fn a_conditional_or_dynamic_require_orders_nothing() {
        // TN: the optional-dependency idiom `if {[catch {package require …}]}`
        // may never run, and `package require $pkg` names nothing statically.
        // Both mirror the gates `package prefer latest` already applies.
        let lib = package_provider();
        for app_src in [
            "catch {package require mymod}\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
            "set pkg mymod\npackage require $pkg\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
        ] {
            let app = analyse(app_src);
            let index = WorkspaceIndex::from_documents([
                ("file:///p/lib.tcl", &lib),
                ("file:///p/app.tcl", &app),
            ]);
            assert_eq!(
                helper_resolves(&index).as_deref(),
                Some("::mymod::helper"),
                "{app_src}",
            );
        }
    }

    #[test]
    fn a_conditional_provide_does_not_make_a_document_the_provider() {
        // TN: the shim idiom. tcllib's `doctools2idx/import_json.tcl` writes
        // `package provide dict 1` inside `if {[package vcompare …] < 0} { if
        // {[catch {package require dict}]} { … } }` — a fake `dict` package
        // supplied only on an old interpreter. Reading that as "this document
        // provides `dict`" would name the wrong file as the provider on every
        // other interpreter, so a guarded provide establishes nothing.
        let shim = analyse(
            "if {[package vcompare [package present Tcl] 8.5] < 0} {\nnamespace eval ::mymod { proc helper {} { return SHIM } }\npackage provide mymod 1.0\n}\n",
        );
        let app = analyse(
            "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///p/lib.tcl", &package_provider()),
            ("file:///p/shim.tcl", &shim),
            ("file:///p/app.tcl", &app),
        ]);
        // Only `lib.tcl` provides `mymod` unconditionally, so the edge stands
        // and the `-clear` revokes…
        assert!(helper_resolves(&index).is_none());
        // …but with `lib.tcl`'s provide made conditional too, nothing in the
        // workspace provably provides `mymod` and the order abstains.
        let guarded_lib = analyse(
            "namespace eval ::mymod { proc helper {} { return HELP }\n namespace export helper }\nif {$::tcl_platform(platform) eq \"unix\"} { package provide mymod 1.0 }\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///p/lib.tcl", &guarded_lib),
            ("file:///p/app.tcl", &app),
        ]);
        assert_eq!(helper_resolves(&index).as_deref(), Some("::mymod::helper"));
    }

    #[test]
    fn one_provider_required_from_many_documents_orders_each_of_them() {
        // TP: a package required from three files is loaded once, by whichever
        // require runs first — so it has three upper bounds and no unique
        // position. Each requiring file is still ordered against it, which a
        // tree position (one entry site per document) could not express.
        let lib = package_provider();
        let clearing = analyse(
            "package require mymod\nnamespace eval ::mymod { namespace export -clear }\nnamespace eval ::app { namespace import ::mymod::* }\n",
        );
        let bystander = analyse("package require mymod\nputs [::mymod::helper]\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///p/lib.tcl", &lib),
            ("file:///p/app.tcl", &clearing),
            ("file:///p/one.tcl", &bystander),
            ("file:///p/two.tcl", &bystander),
        ]);
        assert!(helper_resolves(&index).is_none());
    }

    #[test]
    fn an_unfoldable_computed_source_path_orders_nothing() {
        // TN: `source $dir/exp.tcl` where `dir` comes from `[lindex $argv 0]`
        // names no document statically — `lindex` is outside the evaluator's
        // subset, so the constant never folds — the edge is dropped and the
        // export goes back to being unrankable: the deliberate abstention,
        // not an accident of path resolution.
        //
        // Spelling `dir` as `[file dirname [info script]]` instead would fold
        // through the chained single-assignment evaluator; that foldable
        // spelling has its own TP pin directly below.
        let (mod_doc, exp, imp) = import_order_modules();
        let app = analyse(
            "set dir [lindex $argv 0]\nsource mod.tcl\nsource imp.tcl\nsource $dir/exp.tcl\n",
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
    fn typed_source_imports_export_global_homes_and_keep_original_site_scope() {
        let parent = tcl_compiler::analyser::Analyser::new().analyse(
            "namespace eval N {set dir /EPHEMERAL; source /s/shared.tcl}; set globalDir /GLOBAL; source /s/shared.tcl", "jim");
        let reader = tcl_compiler::analyser::Analyser::new()
            .analyse("source $dir/missing.tcl; source $globalDir/core.tcl", "jim");
        let core = tcl_compiler::analyser::Analyser::new().analyse("", "jim");
        let index = sourced_index([
            ("file:///s/start.tcl", &parent),
            ("file:///s/shared.tcl", &reader),
            ("file:///GLOBAL/core.tcl", &core),
        ]);
        let imports = index.imported_path_constants_for("file:///s/shared.tcl");
        assert!(!imports.contains_key("dir"));
        // The earlier namespace route supplies no globalDir; agreement must
        // not use the later route's value for all executions.
        assert!(!imports.contains_key("globalDir"));
        assert_eq!(
            imports.naming_policy(),
            parent.path_constant_assignments.naming_policy()
        );
        let seeds = index.source_seed_map(test_resolve);
        assert!(!seeds.contains_key("file:///s/missing.tcl"));
    }

    #[test]
    fn path_inventory_reset_and_snapshot_preserve_empty_policy_changes() {
        let c = tcl_compiler::analyser::Analyser::new().analyse("", "tcl8.6");
        let jim = tcl_compiler::analyser::Analyser::new().analyse("", "jim");
        let mut index = sourced_index([("file:///empty.tcl", &c)]);
        let before = index.path_constant_assignments("file:///empty.tcl").clone();
        assert!(before.is_empty());
        index.add_document("file:///empty.tcl", &jim);
        assert_ne!(
            index.path_constant_assignments("file:///empty.tcl"),
            &before
        );
        assert_eq!(
            index
                .path_constant_assignments("file:///missing.tcl")
                .naming_policy(),
            None
        );
    }

    #[test]
    fn a_cross_file_constant_resolves_a_readers_source_rows_1368() {
        // OSVVM's shape in miniature: two simulator entry scripts each
        // assign the same namespace constant and source the shared reader;
        // the reader sources a grandchild *through the imported value*.
        // Both parents live in one directory, so the agreement rule keeps
        // the name and the reader's own row resolves.
        let parent = analyse(
            "namespace eval ::osvvm {\n    variable dir [file dirname [file normalize [info script]]]\n}\nsource ${::osvvm::dir}/shared.tcl\n",
        );
        let reader = analyse("source ${::osvvm::dir}/core.tcl\n");
        let core = analyse("proc core_helper {} {}\n");
        let index = sourced_index([
            ("file:///s/start1.tcl", &parent),
            ("file:///s/start2.tcl", &parent),
            ("file:///s/shared.tcl", &reader),
            ("file:///s/core.tcl", &core),
        ]);
        assert_eq!(
            index
                .imported_path_constants_for("file:///s/shared.tcl")
                .get("::osvvm::dir")
                .map(String::as_str),
            Some("/s"),
        );
        let seeds = index.source_seed_map(test_resolve);
        assert!(
            seeds.contains_key("file:///s/core.tcl"),
            "the reader's own source row must resolve through the import: {seeds:?}",
        );
    }

    #[test]
    fn disagreeing_parents_import_nothing_1368() {
        // TN: the same reader sourced from two directories — the constant
        // folds to a different value per parent, so no route-independent
        // value exists and the name must not import.  Whichever parent runs
        // decides, and this tier never guesses.
        let parent = analyse(
            "namespace eval ::osvvm {\n    variable dir [file dirname [file normalize [info script]]]\n}\nsource /s/shared.tcl\n",
        );
        let reader = analyse("source ${::osvvm::dir}/core.tcl\n");
        let core = analyse("proc core_helper {} {}\n");
        let index = sourced_index([
            ("file:///a/start1.tcl", &parent),
            ("file:///b/start2.tcl", &parent),
            ("file:///s/shared.tcl", &reader),
            ("file:///s/core.tcl", &core),
        ]);
        assert!(
            !index
                .imported_path_constants_for("file:///s/shared.tcl")
                .contains_key("::osvvm::dir"),
        );
        let seeds = index.source_seed_map(test_resolve);
        assert!(
            !seeds.contains_key("file:///s/core.tcl"),
            "no agreed value, no edge: {seeds:?}",
        );
    }

    #[test]
    fn a_body_local_source_route_imports_nothing_1368() {
        // A `source` inside a proc runs when the proc is called, not at its
        // lexical position: `set dir /a; proc load {} {source shared.tcl};
        // set dir /b; load` reaches the child with `/b`.  The lexical
        // position gate would say `/a`, so a body-local route must supply
        // nothing at all.
        let parent = analyse(
            "namespace eval ::cfg {\n    variable dir [file dirname [file normalize [info script]]]\n}\nproc load {} {\n    source /s/shared.tcl\n}\n",
        );
        let reader = analyse("source ${::cfg::dir}/core.tcl\n");
        let core = analyse("proc core_helper {} {}\n");
        let index = sourced_index([
            ("file:///s/start.tcl", &parent),
            ("file:///s/shared.tcl", &reader),
            ("file:///s/core.tcl", &core),
        ]);
        assert!(
            !index
                .imported_path_constants_for("file:///s/shared.tcl")
                .contains_key("::cfg::dir"),
            "a body-local source route must not provide load-time constants",
        );
    }

    #[test]
    fn an_assignment_after_the_source_does_not_import_1368() {
        // TN: the parent assigns the constant *after* sourcing the reader —
        // when the reader runs, the variable does not exist yet, and the
        // position gate must say so.
        let parent = analyse(
            "source /s/shared.tcl\nnamespace eval ::osvvm {\n    variable dir [file dirname [file normalize [info script]]]\n}\n",
        );
        let reader = analyse("source ${::osvvm::dir}/core.tcl\n");
        let core = analyse("proc core_helper {} {}\n");
        let index = sourced_index([
            ("file:///s/start1.tcl", &parent),
            ("file:///s/shared.tcl", &reader),
            ("file:///s/core.tcl", &core),
        ]);
        assert!(
            !index
                .imported_path_constants_for("file:///s/shared.tcl")
                .contains_key("::osvvm::dir"),
            "a write after the source has not happened when the child runs",
        );
    }

    #[test]
    fn a_foldable_computed_source_path_orders_its_export() {
        // TP: the same shape with `dir` from `[file dirname
        // [info script]]` *does* fold through the chained constant, so
        // `$dir/exp.tcl` names its document and the load order ranks it.
        // With the order known the answer sharpens: app.tcl runs the
        // wildcard import *before* sourcing exp.tcl's `namespace export`,
        // and `namespace import` copies only currently-exported commands
        // (a later export does not retro-import), so `helper` is genuinely
        // not imported and resolution answers nothing — more precise than
        // the abstention the unfoldable spelling above falls back to.
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
            None,
        );
    }

    #[test]
    fn a_forget_written_after_the_source_revokes_the_sourced_import() {
        // TP (CRITICAL), the `source lib.tcl ; namespace forget ::lib::p`
        // idiom: the install from the sourced file counts *and* the forget
        // beside the `source` revokes it. Oracle: `::app::helper` is an
        // `invalid command name` after the forget.
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
        // TP (CRITICAL) — two imports of one name from
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
        // TP — a plain `at <= import_at` predicate is *weaker* than the
        // same-document tier's `indirection::in_effect` and rejects this real
        // alias. The import
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
        // FN guard for the leniency above: the
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

    // The import edge's own lifecycle, cross-document.
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
        // alias. Same direction as an unordered `namespace export -clear`.
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
        // FP guard (CRITICAL) — the import lives in a short `imports.tcl`
        // (small byte offset); the forget lives in a long `caller.tcl` (large
        // byte offset) and names a namespace the caller never imported into
        // itself. Nothing orders the two files, so the forget must revoke
        // nothing — comparing the raw offsets would make the caller's larger
        // number "later" and drop a live alias.
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
        // TN, the other direction of the same-document rule — when the import, the forget
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

    // Call-site ordering of the install.
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
        // `innermost_definition_body_span` per row (which is
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

    // Global-rooted imports go through the gate too.
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
        // TN — the glob spelling of the same shape, whose empty source
        // namespace makes it easy to drop rather than record.
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

    // Glob and exact import conflicts are symmetric.
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
        // TN (CRITICAL) — an exact `namespace import ::mymod::helper`
        // produces a fixed link rather than a per-call glob lookup, so the
        // lifecycle events must reach it: otherwise the link stays live after
        // the forget and cross-document definition / references keep
        // resolving `::app::helper` to `::mymod::helper`.
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
        // FP guard — the same-document ordering rule on the link tier: a forget with
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
        // TN, cross-document — a `proc ::app::helper`
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
        // TN, cross-document — `::dst` imports `::A::*`
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
        // TN — the live-alias conflict on the exact-link tier.
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

    // Exact (non-glob) `namespace import` is export-gated too.
    // Real Tcl silently installs nothing when the name is
    // not exported at the import's own position (oracle: `namespace eval
    // ::m {proc helper {} {}}; namespace eval ::a {namespace import
    // ::m::helper}` leaves `info commands ::a::*` empty and raises no error).

    #[test]
    fn exact_import_of_an_unexported_name_installs_no_link() {
        // FP guard (CRITICAL) — `::mymod` never exports `helper`, so the
        // exact import binds nothing: no `::app::helper` exists, the bare
        // call is not a reference to the source, and the pattern token is not
        // a link span.
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
        // TP — the declaring `namespace eval` blocks live in
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

    /// Declaring rows whose name is a **strict descendant** of
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
        // TN — an *unnested* (top-level) proc named after
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
    fn command_retirement_hides_the_vacated_name_but_keeps_the_rename_target() {
        // TP/TN, Tcl 9.0.4: `rename helper moved` transfers one command
        // object.  The old direct name is gone, while the new link remains
        // callable.  The workspace view must not resurrect `helper` from its
        // historical proc declaration when a second document asks about it.
        let lifecycle = analyse("proc ::helper {} {}\nrename ::helper ::moved\n");
        let index = WorkspaceIndex::from_documents([("file:///life.tcl", &lifecycle)]);
        assert!(
            !index.workspace_command_exists_for_call("::helper", false),
            "the name vacated by rename is not callable",
        );
        assert!(
            index.workspace_command_exists_for_call("::moved", false),
            "the rename target keeps the command object",
        );
        assert!(
            index.live_procs().all(|proc_def| proc_def.name != "helper"),
            "the direct command view must not expose the vacated name",
        );
        assert!(
            !index.command_names().contains("helper"),
            "the W123 cache must be built from live definitions",
        );
        assert!(
            !index.defined_command_names(false).contains("helper"),
            "the settled-target cache must not retain the vacated source name",
        );
        assert!(
            index.defined_command_names(true).contains("moved"),
            "the link-aware settled-target cache keeps the rename destination",
        );
    }

    #[test]
    fn later_redefinition_revives_a_retired_command_name() {
        // FN guard: retirement is ordered within its document, rather than a
        // permanent workspace tombstone.  Tcl permits a fresh proc after a
        // rename, and both the moved command and the new definition live.
        let lifecycle = analyse(
            "proc ::helper {} {return old}\nrename ::helper ::moved\nproc ::helper {} {return new}\n",
        );
        let index = WorkspaceIndex::from_documents([("file:///life.tcl", &lifecycle)]);
        assert!(index.workspace_command_exists_for_call("::helper", false));
        assert!(index.workspace_command_exists_for_call("::moved", false));
        assert_eq!(index.live_procs().filter(|p| p.name == "helper").count(), 1);
    }

    #[test]
    fn class_command_retirement_is_shared_by_command_and_class_lookups() {
        // TP: TclOO class creation installs a command.  Deleting that command
        // makes `Dog new` unknown; it cannot remain a class provider merely
        // because the class body was recorded earlier in the document.
        let lifecycle = analyse("oo::class create ::Dog {}\nrename ::Dog {}\n");
        let index = WorkspaceIndex::from_documents([("file:///life.tcl", &lifecycle)]);
        assert!(!index.workspace_command_exists_for_call("::Dog", false));
        assert!(
            index
                .live_classes()
                .all(|class_def| class_def.name != "Dog")
        );
        assert!(!index.command_names().contains("Dog"));
        assert!(!index.defined_command_names(false).contains("Dog"));
    }

    #[test]
    fn top_level_alias_workspace_command_exists_for_call_regardless_of_builtin() {
        // TP — `workspace_command_exists_for_call`'s `has_builtin` branch drops
        // *every* `command_links` entry (aliases / renames / imports), not
        // just conditional ones, so a permanent top-level `interp alias {}
        // set {} ::my_set` — exactly like a top-level `proc set` — must keep
        // counting as "::set exists" even with a same-named builtin in play.
        let a = analyse("proc my_set {args} {}\ninterp alias {} set {} ::my_set\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        let link = index
            .command_links()
            .find(|l| unroot_rooted_key(&l.linked_qname).unwrap_or(&l.linked_qname) == "set")
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
            .find(|l| unroot_rooted_key(&l.linked_qname).unwrap_or(&l.linked_qname) == "set")
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

    /// TP — a document whose only stake
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

    /// TP — a namespace variable declared in one
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

    #[test]
    fn document_revision_changes_only_with_that_document() {
        let a = analyse("proc alpha {} {}\n");
        let b = analyse("proc beta {} {}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        let a_revision = index.document_revision("file:///a.tcl").unwrap();

        index.add_document("file:///b.tcl", &b);
        assert_eq!(
            index.document_revision("file:///a.tcl"),
            Some(a_revision),
            "an unrelated mutation must not invalidate a source snapshot",
        );
        index.replace_document("file:///a.tcl", &a);
        assert_ne!(
            index.document_revision("file:///a.tcl"),
            Some(a_revision),
            "replacing the document must invalidate its source snapshot",
        );
        index.remove_document("file:///a.tcl");
        assert_eq!(index.document_revision("file:///a.tcl"), None);
    }

    #[test]
    fn conditional_revision_removal_preserves_a_newer_replacement() {
        let old = analyse("proc old {} {}\n");
        let new = analyse("proc new {} {}\n");
        let mut index = WorkspaceIndex::new();
        let old_revision = index.replace_document_with_revision("file:///a.tcl", &old);
        let new_revision = index.replace_document_with_revision("file:///a.tcl", &new);

        assert!(
            !index.remove_document_if_revision("file:///a.tcl", old_revision),
            "an obsolete publisher must not remove a newer replacement",
        );
        assert_eq!(index.document_revision("file:///a.tcl"), Some(new_revision));
        assert!(index.workspace_command_exists("::new"));
        assert!(!index.workspace_command_exists("::old"));
        assert!(index.remove_document_if_revision("file:///a.tcl", new_revision));
        assert!(!index.contains_document("file:///a.tcl"));
    }

    #[test]
    fn document_revision_survives_slot_reuse_aba_1854() {
        let a = analyse("proc alpha {} {}\n");
        let b = analyse("proc beta {} {}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        let old_revision = index.document_revision("file:///a.tcl").unwrap();

        index.remove_document("file:///a.tcl");
        index.add_document("file:///b.tcl", &b);
        index.add_document("file:///a.tcl", &a);

        assert_ne!(
            index.document_revision("file:///a.tcl"),
            Some(old_revision),
            "re-adding a URI must not recycle its old token after slot churn",
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

    /// `defined_command_names` caches its `HashSet` — one slot per
    /// `include_links` value — and drops it only on a mutation, same
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

    /// `command_link_map` caches its `HashMap` directly rather than rebuilding
    /// it from `live_command_links` per call, same discipline as
    /// `command_names`.
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

    /// `invocations_of` (via `settled_sites`) must not re-settle every
    /// invocation in the workspace, nor rebuild `WildcardImportIndex`, per
    /// call — `code_lenses` calls it once per proc *and* once per class in the
    /// document. The settled-target grouping is cached per generation, so
    /// repeated `invocations_of` calls against an unchanged index share it.
    #[test]
    fn settled_target_index_is_cached_until_its_inputs_change() {
        let a = analyse("proc helper {} {}\nproc other {} {}\n");
        let b = analyse("helper\nhelper\nother\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        index.add_document("file:///b.tcl", &b);

        // Querying different targets against the same generation must hit
        // the same cached map, not re-settle per target.
        let helper_calls = index.invocations_of("::helper", "");
        let after_first = index.settled_invocations[0].settled_documents();
        let other_calls = index.invocations_of("::other", "");
        assert_eq!(helper_calls.len(), 2, "{helper_calls:?}");
        assert_eq!(other_calls.len(), 1, "{other_calls:?}");
        assert_eq!(
            index.settled_invocations[0].settled_documents(),
            after_first,
            "an unchanged index must serve the settled-target cache",
        );

        // A mutation invalidates the cache and the next call re-settles
        // against the new state.
        let c = analyse("helper\n");
        index.add_document("file:///c.tcl", &c);
        assert_eq!(index.invocations_of("::helper", "").len(), 3);
        assert_eq!(
            index.settled_invocations[0].settled_documents(),
            after_first + 3,
            "a command-set change must re-settle all three documents",
        );
    }

    #[test]
    fn body_only_replacement_re_settles_only_the_changed_document() {
        // TP + performance proof.  The replacement changes both
        // call sites in `a.tcl`, so a cache that merely kept the old whole
        // view would be wrong; it must remove that document's old contribution
        // and insert its new one.  `b.tcl` supplies the same command table as
        // before, so it must not be visited again.
        let a = analyse("proc ::caller {} { helper }\nhelper\n");
        let b = analyse("proc ::helper {} {}\nproc ::other {} {}\n");
        let mut index =
            WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(index.invocations_of("::helper", "").len(), 2);
        let cold = index.settled_invocations[0].settled_documents();
        assert_eq!(cold, 2, "the cold read settles both documents");

        let edited_a = analyse("proc ::caller {} { other }\nother\n");
        index.replace_document("file:///a.tcl", &edited_a);
        assert!(
            index.invocations_of("::helper", "").is_empty(),
            "the old contribution must be removed",
        );
        assert_eq!(index.invocations_of("::other", "").len(), 2);
        assert_eq!(
            index.settled_invocations[0].settled_documents(),
            cold + 1,
            "a body-only replacement must settle one document, not the workspace",
        );
        let wholesale =
            WorkspaceIndex::from_documents([("file:///a.tcl", &edited_a), ("file:///b.tcl", &b)]);
        for target in ["::helper", "::other"] {
            assert_eq!(
                index.invocations_of(target, "").len(),
                wholesale.invocations_of(target, "").len(),
                "incremental replacement must agree with a wholesale rebuild for {target}",
            );
        }
    }

    #[test]
    fn resolution_input_replacement_re_settles_cross_document_callers() {
        // FN guard: a naive document-local cache would leave `a.tcl` filed
        // under `::helper` after `b.tcl` removes that definition.  Definitions
        // are part of SettlementDependencies, so this intentionally takes the
        // full path and agrees with a fresh workspace build.
        let a = analyse("helper\n");
        let b = analyse("proc ::helper {} {}\n");
        let mut index =
            WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(index.invocations_of("::helper", "").len(), 1);
        let cold = index.settled_invocations[0].settled_documents();

        let edited_b = analyse("# helper was removed\n");
        index.replace_document("file:///b.tcl", &edited_b);
        assert!(
            index.invocations_of("::helper", "").is_empty(),
            "a caller in another document must be re-settled",
        );
        assert_eq!(
            index.settled_invocations[0].settled_documents(),
            cold + 2,
            "a command-table change must re-settle every document",
        );
        let wholesale =
            WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &edited_b)]);
        assert_eq!(
            index.invocations_of("::helper", "").len(),
            wholesale.invocations_of("::helper", "").len(),
            "the cross-document invalidation must agree with a wholesale rebuild",
        );
    }

    #[test]
    fn export_replacement_re_settles_wildcard_import_callers() {
        // TN/FP guard for the non-definition half of the dependency surface:
        // an export change can make a different document's wildcard-imported
        // call disappear, even though its own text did not change.
        let lib = analyse(
            "namespace eval ::lib {\n    proc helper {} {}\n    namespace export helper\n}\n",
        );
        let app = analyse("namespace import ::lib::*\nhelper\n");
        let mut index =
            WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
        assert_eq!(index.linked_invocations_of("::lib::helper", "").len(), 1);
        let cold = index.settled_invocations[1].settled_documents();

        let edited_lib = analyse("namespace eval ::lib {\n    proc helper {} {}\n}\n");
        index.replace_document("file:///lib.tcl", &edited_lib);
        assert!(
            index.linked_invocations_of("::lib::helper", "").is_empty(),
            "the caller must not retain an export which no longer exists",
        );
        assert_eq!(index.settled_invocations[1].settled_documents(), cold + 2);
    }

    #[test]
    fn defined_command_names_are_cached_per_reading_and_dropped_on_mutation() {
        // `workspace_command_exists` asks for this set once per
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

    /// `package prefer latest` is interpreter-global, and
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

    // Both tiers on the two-file `namespace import -force` shadow.
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
        // The same fact on the *settle* path find-references reads: a
        // candidate naming the command a `-force` import deleted must not
        // settle the call, or find-references files it under a definition
        // go-to-definition no longer answers with.
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

    // The settlement walk's cost must not scale with
    // (invocations x in-scope imports x export rows).
    //
    // Asked per call, that product is a 26 s `textDocument/references` on a
    // 113-file workspace.  The shape that produces it is entirely ordinary and
    // is reproduced below: many load-level `namespace import NS::*`, a source
    // namespace with many `namespace export` patterns, and a great many bare
    // calls that resolve to no workspace command at all (every `set`, `if`,
    // `expr` in the project), each of which falls through to the wildcard
    // import tier.  Each such call would ask, per in-scope import, per export
    // row, "had this export run when that import ran?" — a `RunOrder` walk
    // over `HashMap`s keyed by owned URI strings.
    //
    // Nothing about an import's position depends on the call being resolved,
    // so that whole question is decided once per recorded import at
    // index-build time (`ExportGate`), leaving a hash probe and a glob match
    // on the per-call path.

    /// A workspace of that shape, sized so a per-call gate costs seconds and
    /// the indexed one costs milliseconds.
    fn wildcard_import_heavy_workspace(
        importers: usize,
        exports: usize,
        calls_per_file: usize,
    ) -> Vec<(String, AnalysisResult)> {
        use std::fmt::Write as _;
        let patterns: Vec<String> = (0..exports).map(|i| format!("Exported{i}")).collect();
        let lib = format!(
            "namespace eval ::Lib {{\n    namespace export {}\n    proc helper {{}} {{}}\n}}\n",
            patterns.join(" "),
        );
        let mut docs = vec![("file:///lib.tcl".to_owned(), analyse(&lib))];
        for f in 0..importers {
            // A load-level glob import, then bare calls that settle to no
            // workspace command — the ones that reach the wildcard tier.
            let mut src = String::from("namespace import ::Lib::*\n");
            for c in 0..calls_per_file {
                writeln!(src, "noSuchCommand{c} a b").expect("writing to a String cannot fail");
            }
            docs.push((format!("file:///f{f}.tcl"), analyse(&src)));
        }
        docs
    }

    #[test]
    fn settling_invocations_does_not_scale_with_imports_times_exports() {
        let docs = wildcard_import_heavy_workspace(60, 40, 60);
        let index = WorkspaceIndex::from_documents(docs.iter().map(|(uri, a)| (uri.as_str(), a)));
        let started = std::time::Instant::now();
        // The first call builds the settled-target view for the whole
        // workspace, which is where the cost lived.
        let hits = index.linked_invocations_of("::Lib::helper", "");
        let elapsed = started.elapsed();
        assert!(
            hits.is_empty(),
            "no call names ::Lib::helper, so this is the empty answer the walk had to work for",
        );
        // Measured on this shape, debug build: 81 ms with the gate hoisted,
        // 7.59 s with `import_hop` restored to asking `exports_name_at` per
        // call.  The bound sits between them with a 25x margin below and a
        // 3.8x margin above, so it cannot flake on a loaded CI box and still
        // fails outright if the per-call export walk comes back.
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "settling {} documents took {elapsed:?} — the per-call export walk is back (#1297)",
            docs.len(),
        );
    }

    /// The same workspace, asserting the *memo* rather than the wall clock:
    /// a second reading of an unchanged index must serve the cached view, so
    /// the cost above is paid once per generation and not once per request.
    /// (`code_lenses` asks once per proc *and* once per class.)
    #[test]
    fn the_settled_target_view_is_served_from_cache_within_a_generation() {
        let docs = wildcard_import_heavy_workspace(8, 6, 4);
        let index = WorkspaceIndex::from_documents(docs.iter().map(|(uri, a)| (uri.as_str(), a)));
        let _ = index.linked_invocations_of("::Lib::missing", "");
        let first = index.settled_invocations[1].settled_documents();
        let _ = index.linked_invocations_of("::Lib::missing", "");
        assert!(
            first == index.settled_invocations[1].settled_documents(),
            "an unchanged index must serve the settled-target cache, not rebuild it",
        );
    }

    /// TP/TN on the hoisted export gate itself, through the public answer: an
    /// exported name really does resolve through a wildcard import, and an
    /// unexported sibling really does not.  This is the behaviour the
    /// `ExportGate` precomputation must preserve exactly — the perf tests
    /// above would happily pass on a gate that answered "no" to everything.
    #[test]
    fn the_precomputed_export_gate_still_admits_exactly_what_is_exported() {
        let lib = analyse(
            "namespace eval ::Lib {\n    proc shown {} {}\n    proc hidden {} {}\n    namespace export shown\n}\n",
        );
        let app = analyse("namespace import ::Lib::*\nshown\nhidden\n");
        let index =
            WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
        // TP: `shown` is exported, so the bare call reaches it through the import.
        assert_eq!(
            index.linked_invocations_of("::Lib::shown", "").len(),
            1,
            "an exported name must still resolve through the wildcard import",
        );
        // TN: `hidden` is not exported, so the bare call reaches nothing —
        // tclsh 9.0.4 answers `invalid command name` for it.
        assert!(
            index.linked_invocations_of("::Lib::hidden", "").is_empty(),
            "an unexported sibling must not be reachable through the import",
        );
    }

    /// FN guard for the literal/glob split inside [`ExportGate`]: a *glob*
    /// export pattern must keep working, since only metacharacter-free
    /// patterns may take the hash-set path.
    #[test]
    fn a_glob_export_pattern_still_covers_the_names_it_matches() {
        let lib = analyse(
            "namespace eval ::Lib {\n    proc getThing {} {}\n    proc setThing {} {}\n    namespace export get*\n}\n",
        );
        let app = analyse("namespace import ::Lib::*\ngetThing\nsetThing\n");
        let index =
            WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
        assert_eq!(index.linked_invocations_of("::Lib::getThing", "").len(), 1);
        assert!(
            index
                .linked_invocations_of("::Lib::setThing", "")
                .is_empty(),
            "`get*` does not cover `setThing`",
        );
    }

    // The import-conflict rule's "already exists" side must
    // include the commands the *registry* declares, not only the procs and
    // classes the workspace declares.
    //
    // Every expectation below is oracle-verified on tclsh 9.0.4 (built from
    // core-9-0-4) and 8.6.14, byte-identical:
    //
    //   namespace eval ::Foo { proc set {a b} {…}; namespace export set }
    //   namespace import ::Foo::*   -> can't import command "set": already exists
    //                                  namespace origin ::set  == ::set
    //   namespace import ::Foo::set -> same error (the exact-pattern twin)
    //   namespace import -force ::Foo::*  -> namespace origin ::set == ::Foo::set
    //   namespace eval ::Bar { namespace import ::Foo::* }
    //                               -> namespace origin ::Bar::set == ::Foo::set
    //   # …and `+` is NOT a global command, so it does not conflict:
    //   info commands ::+           -> {}      (+ 1 2 -> invalid command name)

    /// TP: the defect itself.  `::Foo` exports `set`; real Tcl refuses the
    /// import, so the bare `set x 1` reaches the builtin and is **not** a
    /// reference to `::Foo::set`.
    #[test]
    fn a_wildcard_import_does_not_bind_a_name_the_registry_already_declares() {
        let a = analyse_as(
            "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let b = analyse_as(
            "namespace import ::Foo::*\nset x 1\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(
            index.linked_invocations_of("::Foo::set", "").is_empty(),
            "the import raised `already exists` and installed nothing, so the \
             bare call reaches the builtin",
        );
    }

    /// TN: `-force` is the one import that *does* replace a command the
    /// target namespace already holds, so the call really is a reference.
    #[test]
    fn a_forced_wildcard_import_still_shadows_a_registry_builtin() {
        let a = analyse_as(
            "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let b = analyse_as(
            "namespace import -force ::Foo::*\nset x 1\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(
            index.linked_invocations_of("::Foo::set", "").len(),
            1,
            "`namespace origin ::set` is `::Foo::set` after a -force import",
        );
    }

    /// FP guard: the rule is about the **target** namespace's command table,
    /// and the builtins live in `::`.  Importing into `::Bar` binds
    /// `::Bar::set` with no conflict at all, so a call from `::Bar` must
    /// still resolve — the gate must not fire on the bare name alone.
    #[test]
    fn importing_a_builtin_name_into_a_sub_namespace_still_binds() {
        let a = analyse_as(
            "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let b = analyse_as(
            "namespace eval ::Bar {\n    namespace import ::Foo::*\n    proc go {} { set x 1 }\n}\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(
            index.linked_invocations_of("::Foo::set", "").len(),
            1,
            "`::Bar::set` is not a builtin, so the import installs and the \
             call from ::Bar reaches it",
        );
    }

    /// TN: an ordinary, non-builtin exported name is unaffected — the gate
    /// must not have made the whole wildcard tier answer "no".
    #[test]
    fn a_wildcard_import_of_a_non_builtin_name_is_unaffected() {
        let a = analyse_as(
            "namespace eval ::Foo {\n    proc mything {} {}\n    namespace export mything\n}\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let b = analyse_as(
            "namespace import ::Foo::*\nmything\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(index.linked_invocations_of("::Foo::mything", "").len(), 1);
    }

    /// FN guard for the operator spellings, which is where a naive
    /// "`registry.get(name).is_some()`" gate would break a *working* import:
    /// `+` is not a command in `::` at all (`info commands ::+` is empty in a
    /// fresh tclsh 9.0.4), so exporting `+` and importing it into `::`
    /// genuinely binds — `namespace origin ::+` answers `::Ops::+`.
    #[test]
    fn importing_an_operator_name_into_the_global_namespace_still_binds() {
        let a = analyse_as(
            "namespace eval ::Ops {\n    proc + {a b} { return OPS }\n    namespace export +\n}\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let b = analyse_as(
            "namespace import ::Ops::*\n+ 1 2\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(
            index.linked_invocations_of("::Ops::+", "").len(),
            1,
            "a bare operator spelling is the post-`namespace import \
             ::tcl::mathop::*` form, not a command a fresh interpreter holds",
        );
    }

    /// The exact-pattern twin: `namespace import ::Foo::set` is refused for
    /// the identical reason, so the link it would introduce is not live.
    #[test]
    fn an_exact_import_of_a_registry_builtin_installs_no_link() {
        let a = analyse_as(
            "namespace eval ::Foo {\n    proc set {a b} { return SHADOW }\n    namespace export set\n}\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let b = analyse_as(
            "namespace import ::Foo::set\nset x 1\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(
            index.linked_invocations_of("::Foo::set", "").is_empty(),
            "an exact import of an already-existing name installs nothing either",
        );
    }

    /// Dialect sensitivity: the gate must read the *document's own* dialect,
    /// not a workspace-wide guess.  `HTTP::uri` is a command only the
    /// `f5-irules` registry declares, so an import of it conflicts there and
    /// binds under plain Tcl.
    #[test]
    fn the_builtin_gate_reads_each_documents_own_dialect() {
        let lib = "namespace eval ::Mine {\n    proc uri {} {}\n    namespace export uri\n}\n";
        let importer =
            "namespace eval ::HTTP {\n    namespace import ::Mine::*\n    proc go {} { uri }\n}\n";
        // Plain Tcl knows no `::HTTP::uri`, so the import installs.
        let a = analyse_as(
            lib,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let b = analyse_as(
            importer,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let plain = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(
            plain.linked_invocations_of("::Mine::uri", "").len(),
            1,
            "plain Tcl has no ::HTTP::uri to conflict with",
        );
        // The iRules registry does, so the same source conflicts.
        let a = analyse_as(lib, tcl_dialect::DialectProfile::irules());
        let b = analyse_as(importer, tcl_dialect::DialectProfile::irules());
        let irules = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert!(
            irules.linked_invocations_of("::Mine::uri", "").is_empty(),
            "f5-irules declares ::HTTP::uri, so the unforced import installs nothing",
        );
    }

    // `import_hop` must not scan a namespace's whole import list per call.
    //
    // Both of its first two filters — the tail pattern covers the word, and
    // the export gate admits it — depend only on the word, so the admissible
    // set is a build-time fact (`NamespaceImports`). On a large corpus such a
    // scan is 28 369 calls over 3 987 739 rows, ~390 ms of the ~420 ms a
    // `textDocument/references` takes after an edit.
    //
    // The risk in indexing it is order: `import_hop` breaks ties on the
    // position *within the filtered sequence*, so a reordering — or a
    // duplicate from a row carrying both literal and glob export patterns —
    // would silently change which import wins. These pin that.

    /// The admitted sequence must be exactly what a full scan would yield,
    /// in the same order, over every shape the gate can take: literal-only,
    /// glob-only, both at once (the duplicate hazard), and a tail pattern
    /// that excludes a name the gate would otherwise admit.
    #[test]
    fn admitting_matches_a_full_scan_in_order() {
        let lib = analyse(
            "namespace eval ::Lib {\n    proc alpha {} {}\n    proc beta {} {}\n    proc gamma {} {}\n    namespace export alpha beta gamma\n}\n",
        );
        let globby = analyse(
            "namespace eval ::Glob {\n    proc alpha {} {}\n    proc beta {} {}\n    namespace export a*\n}\n",
        );
        let app = analyse(
            "namespace import ::Lib::*\nnamespace import ::Glob::*\nnamespace import ::Lib::b*\nalpha\nbeta\ngamma\n",
        );
        let index = WorkspaceIndex::from_documents([
            ("file:///lib.tcl", &lib),
            ("file:///glob.tcl", &globby),
            ("file:///app.tcl", &app),
        ]);
        let wci = WildcardImportIndex::build(&index);
        let imports = wci
            .imports_by_ns
            .get("::")
            .expect("the three imports are all at global level");
        for word in ["alpha", "beta", "gamma", "delta", "", "a", "*"] {
            let scanned: Vec<_> = imports
                .rows
                .iter()
                .filter(|row| tcl_syntax::glob::string_match(&row.imp.tail_pattern, word))
                .filter(|row| row.exported.covers(word))
                .map(|row| (row.imp.uri.as_str(), row.imp.at))
                .collect();
            let admitted: Vec<_> = imports
                .admitting(word)
                .map(|row| (row.imp.uri.as_str(), row.imp.at))
                .collect();
            assert_eq!(
                admitted, scanned,
                "admitting({word:?}) must equal the full scan, in order",
            );
        }
    }

    /// A row whose gate holds **both** a literal and a glob pattern is
    /// reachable through both halves of the index and must still appear once.
    /// A duplicate would shift every later row's tie-break position.
    #[test]
    fn a_row_indexed_twice_is_admitted_once() {
        let lib = analyse(
            "namespace eval ::Both {\n    proc alpha {} {}\n    namespace export alpha a*\n}\n",
        );
        let app = analyse("namespace import ::Both::*\nalpha\n");
        let index =
            WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
        let wci = WildcardImportIndex::build(&index);
        let imports = wci.imports_by_ns.get("::").expect("one global import");
        assert_eq!(
            imports.admitting("alpha").count(),
            1,
            "`alpha` matches both the literal and the `a*` pattern of one gate",
        );
    }

    /// TP through the public answer: the indexed path still resolves a call
    /// that only a wildcard import can reach. The perf guards below would be
    /// happy with an index that admitted nothing at all.
    #[test]
    fn the_admission_index_still_resolves_a_wildcard_imported_call() {
        let lib = analyse(
            "namespace eval ::Lib {\n    proc helper {} {}\n    namespace export helper\n}\n",
        );
        let app = analyse("namespace import ::Lib::*\nhelper\n");
        let index =
            WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
        assert_eq!(index.linked_invocations_of("::Lib::helper", "").len(), 1);
    }

    /// A gate that exports nothing admits nothing, so such rows drop out of
    /// every lookup — the case that makes the index worth having, since a
    /// workspace routinely imports namespaces it holds no exports for
    /// (`::tcl::mathop`, `::tcltest`).
    #[test]
    fn an_import_of_an_unexporting_namespace_admits_no_word() {
        let app = analyse("namespace import ::tcl::mathop::*\nset x 1\n+ 1 2\n");
        let index = WorkspaceIndex::from_documents([("file:///app.tcl", &app)]);
        let wci = WildcardImportIndex::build(&index);
        let imports = wci.imports_by_ns.get("::").expect("one global import");
        for word in ["set", "+", "puts", "anything"] {
            assert_eq!(
                imports.admitting(word).count(),
                0,
                "no export rows for ::tcl::mathop, so nothing is admissible",
            );
        }
    }
}

#[cfg(test)]
mod original_settlement_transport_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::signature_scan::scope::SignatureSourceCommand;

    fn original_analysis(source: &str) -> AnalysisResult {
        Analyser::new().analyse(source, "tcl8.6").clone()
    }

    fn clear_reporting(analysis: &mut AnalysisResult) {
        analysis.all_procs.clear();
        analysis.superseded_procs.clear();
        analysis.all_classes.clear();
        analysis.superseded_classes.clear();
        for invocation in &mut analysis.command_invocations {
            invocation.name = "reporting-only".to_owned();
            invocation.resolved_qualified_name = Some("::wrong".to_owned());
            invocation.resolution_candidates = vec!["::wrong".to_owned()];
        }
    }

    fn original_names(analysis: &AnalysisResult) -> Vec<SignatureSourceCommand> {
        analysis
            .original_procedure_declarations()
            .map(|declaration| declaration.name().clone())
            .collect()
    }

    #[test]
    fn original_source_procedure_candidates_share_slots_and_reindex_currency() {
        // naming.source.original-procedure-publications
        // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
        let source = "proc old {value} {}; proc deleted {} {}; rename old moved; rename deleted {}";
        let mut analysis = original_analysis(source);
        assert!(analysis.original_completed_command_world().is_none());
        clear_reporting(&mut analysis);
        let uri = "file:///source-procedures.tcl";
        let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
        let candidates = index
            .original_procedure_source_candidates()
            .collect::<Vec<_>>();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0, uri);
        assert_eq!(candidates[0].1.name_input().bytes(), b"old");
        assert_eq!(candidates[0].2.simple.as_bytes(), b"moved");
        let caller = original_analysis("moved; old; deleted");
        for invocation in &caller.command_invocations {
            let input = invocation.original_name_input.as_ref().unwrap();
            let lookup = tcl_compiler::signature_scan::scope::SignatureSourceLookup::from_input(
                tcl_compiler::signature_scan::scope::SignatureNamespaceScope::C(
                    tcl_core_types::ByteNamespacePath::root(),
                ),
                input,
            )
            .unwrap();
            let matching = index.original_procedure_candidates(&lookup);
            assert_eq!(matching.len(), usize::from(input.bytes() == b"moved"));
            if let Some((owner, declaration)) = matching.first() {
                assert_eq!(*owner, uri);
                assert_eq!(declaration.name_input().bytes(), b"old");
            }
        }
        let dependency = index.docs[0].settlement_dependencies();
        let replacement = original_analysis("proc old {value} {}; rename old other");
        index.replace_document(uri, &replacement);
        assert!(dependency != index.docs[0].settlement_dependencies());
        let names = index
            .original_procedure_source_candidates()
            .map(|(_, _, slot, _)| slot.simple.as_bytes().to_vec())
            .collect::<Vec<_>>();
        assert_eq!(names, [b"other".to_vec()]);
        index.remove_document(uri);
        index.add_document("file:///reused.tcl", &AnalysisResult::default());
        assert!(
            index
                .original_procedure_source_candidates()
                .next()
                .is_none()
        );
    }

    #[test]
    fn original_source_class_candidates_share_current_slots_and_canonical_navigation() {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        let source = "oo::class create Old {}; oo::class create Removed {}; rename Old New; rename Removed {}";
        let mut analysis = original_analysis(source);
        assert!(analysis.original_completed_command_world().is_none());
        clear_reporting(&mut analysis);
        let uri = "file:///source-classes.tcl";
        let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
        assert_eq!(index.original_class_declarations().count(), 2);
        let candidates = index.original_class_source_candidates().collect::<Vec<_>>();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0, uri);
        assert_eq!(candidates[0].1.name_input().bytes(), b"Old");
        assert_eq!(candidates[0].2.simple.as_bytes(), b"New");
        let canonical = candidates[0].1.name().clone();
        let caller = original_analysis("New; Old; Removed");
        for invocation in &caller.command_invocations {
            let input = invocation.original_name_input.as_ref().unwrap();
            let lookup = tcl_compiler::signature_scan::scope::SignatureSourceLookup::from_input(
                tcl_compiler::signature_scan::scope::SignatureNamespaceScope::C(
                    tcl_core_types::ByteNamespacePath::root(),
                ),
                input,
            )
            .unwrap();
            let selected = input.bytes() == b"New";
            let matching = index.original_class_candidates(&lookup);
            assert_eq!(matching.len(), usize::from(selected));
            for follow_links in [false, true] {
                assert_eq!(
                    index.original_assistance_target(
                        invocation.original_lookup.as_ref().unwrap(),
                        follow_links,
                    ),
                    selected.then(|| SettledCommandTarget::OriginalSlot(
                        canonical.slot().clone(),
                        canonical.policy(),
                    )),
                );
            }
        }
        let dependencies = index.docs[0].settlement_dependencies();
        let replacement = original_analysis("oo::class create Old {}; rename Old Other");
        index.replace_document(uri, &replacement);
        assert!(dependencies != index.docs[0].settlement_dependencies());
        assert_eq!(
            index
                .original_class_source_candidates()
                .next()
                .unwrap()
                .2
                .simple
                .as_bytes(),
            b"Other",
        );
        index.remove_document(uri);
        index.add_document("file:///reused-class.tcl", &AnalysisResult::default());
        assert!(index.original_class_source_candidates().next().is_none());
        assert!(index.docs[0].original_source_input.is_none());
        assert!(index.docs[0].original_source_classes.is_none());
    }

    #[test]
    fn original_source_class_dependencies_retain_mutation_premises_and_known_barriers() {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        let uri = "file:///source-classes.tcl";
        let source = "oo::class create C {}";
        let analysis = original_analysis(source);
        let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
        let dependencies = index.docs[0].settlement_dependencies();
        let uncertain = original_analysis("oo::class create C {}; unknown_effect");
        assert!(uncertain.original_completed_command_world().is_none());
        index.replace_document(uri, &uncertain);
        let changed = index.docs[0].settlement_dependencies();
        assert_eq!(changed.original_source_class_candidates.len(), 1);
        assert!(changed.original_source_class_candidates[0].3.contains(
            &tcl_compiler::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
        ));
        assert!(dependencies != changed);
        for source in [
            "oo::class create C {}; rename C {}",
            "oo::class create C {}; proc C {} {}",
        ] {
            let deleted = original_analysis(source);
            index.replace_document(uri, &deleted);
            assert_eq!(index.original_class_declarations().count(), 1);
            assert!(index.original_class_source_candidates().next().is_none());
            assert!(
                index.docs[0]
                    .settlement_dependencies()
                    .original_source_class_candidates
                    .is_empty()
            );
        }
    }

    #[test]
    fn original_source_class_candidates_require_the_retained_input_and_availability() {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        let source = "oo::class create C {}";
        let uri = "file:///source-classes.tcl";
        let analysis = original_analysis(source);
        let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
        assert_eq!(index.original_class_source_candidates().count(), 1);
        let input = analysis.resolved_input.as_ref().unwrap();
        let generation = input.context_registry();
        let foreign = Arc::new(
            generation.with_command_store(generation.commands().snapshot().shared_registry()),
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        let mut changed = analysis.clone();
        changed.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            foreign,
            input.lexer_config(),
        ));
        let mut wrong_config = analysis.clone();
        wrong_config
            .body_lexer_config
            .as_mut()
            .unwrap()
            .strict_quoting = !analysis.body_lexer_config.unwrap().strict_quoting;
        for refused in [missing, changed, wrong_config] {
            index.replace_document(uri, &refused);
            assert_eq!(index.original_class_declarations().count(), 1);
            assert!(index.original_class_source_candidates().next().is_none());
        }
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(generation.commands())),
        );
        let supplied = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        );
        let unavailable = Analyser::new()
            .with_resolved_input(supplied)
            .analyse(source, "tcl8.6");
        index.replace_document(uri, &unavailable);
        assert!(index.original_class_source_candidates().next().is_none());
        assert_eq!(index.original_class_declarations().count(), 0);
        assert!(index.docs[0].original_source_classes.is_some());
    }

    #[test]
    fn original_reverse_index_keeps_opaque_slots_after_reporting_is_cleared() {
        let mut library = original_analysis("proc p\\uD800 {} {}\nproc p\\uD801 {} {}\n");
        let mut caller = original_analysis("p\\uD800\np\\uD801\n");
        let names = original_names(&library);
        assert_eq!(names.len(), 2);
        assert_ne!(names[0].slot(), names[1].slot());
        clear_reporting(&mut library);
        clear_reporting(&mut caller);
        let index = WorkspaceIndex::from_documents([
            ("file:///library.tcl", &library),
            ("file:///caller.tcl", &caller),
        ]);
        for name in &names {
            let rows = index.original_invocations_of(name, "", false);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].uri, "file:///caller.tcl");
            assert_eq!(
                rows[0].original_name_input.as_ref().unwrap().bytes(),
                name.slot().simple.as_bytes()
            );
            assert!(
                index
                    .original_invocations_of(name, "file:///caller.tcl", false)
                    .is_empty()
            );
        }
        assert_eq!(index.settled_invocations[0].settled_documents(), 2);
    }

    #[test]
    fn lexical_rename_advice_requires_every_captured_provider_and_refuses_vendor_contexts() {
        // Implementation contract: naming.workspace.lexical-rename-advice-eligibility
        // docs/design/analysis/name-resolution-proofs/workspace-lexical-rename-advice-eligibility.md
        let point =
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79);
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "indexed-logical-jim079",
            &[],
            "Logical Jim source advice",
            point,
        )
        .intern();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let logical = Analyser::new()
            .with_resolved_input(input)
            .analyse("proc lexical {} {}", profile.name);
        assert!(logical.allows_lexical_declaration_advice());
        assert!(!WorkspaceIndex::new().allows_lexical_rename_advice());
        let mut index = WorkspaceIndex::from_documents([("file:///logical.tcl", &logical)]);
        assert!(index.allows_lexical_rename_advice());
        assert!(index.clone().allows_lexical_rename_advice());
        let mut native = Analyser::new().analyse(r"proc p\uD800 {} {}", "tcl8.6");
        native.dialect = profile.name.to_owned();
        index.add_document("file:///native.tcl", &native);
        assert!(!index.allows_lexical_rename_advice());
        index.remove_document("file:///native.tcl");
        assert!(index.allows_lexical_rename_advice());
        for source in ["proc vendor {} {}", ""] {
            let mut vendor = Analyser::new().analyse(source, "f5-irules");
            vendor.dialect = profile.name.to_owned();
            index.add_document("file:///vendor.tcl", &vendor);
            assert!(!index.allows_lexical_rename_advice(), "{source:?}");
            index.remove_document("file:///vendor.tcl");
            assert!(index.allows_lexical_rename_advice());
        }
        index.add_document("file:///unknown.tcl", &AnalysisResult::default());
        assert!(!index.allows_lexical_rename_advice());
        index.remove_document("file:///unknown.tcl");
        assert!(index.allows_lexical_rename_advice());
    }

    #[test]
    fn workspace_symbols_require_current_source_context_before_report_advice() {
        // naming.editor.original-indexed-source-location
        // docs/design/analysis/name-resolution-proofs/original-indexed-source-location.md
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let source = "proc visible {} {}";
        let uri = "file:///symbols.tcl";
        let logical = Analyser::new().analyse(source, "tcl");
        assert!(logical.allows_lexical_declaration_advice());
        let mut index = WorkspaceIndex::from_documents([(uri, &logical)]);
        let symbols = index.symbols_matching("visible", 100);
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].original_location.is_none());
        let native = Analyser::new().analyse(source, "tcl8.6");
        index.replace_document(uri, &native);
        let symbols = index.symbols_matching("visible", 100);
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].original_location.is_some());
        for mut refused in [logical, native] {
            assert!(!refused.all_procs.is_empty());
            refused.resolved_input = None;
            index.replace_document(uri, &refused);
            assert!(index.diagnostic_source_context(uri).is_none());
            assert!(index.symbols_matching("visible", 100).is_empty());
        }
        let mut stale_config = Analyser::new().analyse(source, "tcl8.6");
        let config = stale_config.body_lexer_config.as_mut().unwrap();
        config.strict_quoting = !config.strict_quoting;
        index.replace_document(uri, &stale_config);
        assert!(index.diagnostic_source_context(uri).is_none());
        assert!(index.symbols_matching("visible", 100).is_empty());
        index.remove_document(uri);
        index.add_document(uri, &AnalysisResult::default());
        assert!(index.symbols_matching("", 100).is_empty());
    }

    #[test]
    fn diagnostic_source_context_retains_currency_and_clears_reused_slots() {
        // Implementation contract: naming.editor.original-diagnostic-invalidation
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-invalidation.md
        let uri = "file:///provider.tcl";
        let source = "proc p\\uD800 {} {}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
        let context = index.diagnostic_source_context(uri).unwrap();
        assert_eq!(context.image(), &tcl_lexer::SourceImage::document(source));
        assert_eq!(context.config(), analysis.body_lexer_config.unwrap());
        assert!(context.uses_original_names());
        assert_eq!(index.clone().diagnostic_source_context(uri), Some(context));
        index.remove_document(uri);
        assert!(index.diagnostic_source_context(uri).is_none());
        index.add_document("file:///reused.tcl", &AnalysisResult::default());
        assert!(
            index
                .diagnostic_source_context("file:///reused.tcl")
                .is_none()
        );
        let mut stale_config = analysis.clone();
        let mut config = stale_config.body_lexer_config.unwrap();
        config.strict_quoting = !config.strict_quoting;
        stale_config.body_lexer_config = Some(config);
        index.replace_document(uri, &stale_config);
        assert!(index.diagnostic_source_context(uri).is_none());
    }

    #[test]
    fn original_reverse_index_does_not_replace_missing_lookup_with_reports() {
        let mut library = original_analysis("proc p\\uD800 {} {}\n");
        let mut caller = original_analysis("p\\uD800\n");
        let name = original_names(&library).remove(0);
        let invocation = caller
            .command_invocations
            .iter_mut()
            .find(|invocation| {
                invocation
                    .original_name_input
                    .as_ref()
                    .is_some_and(|input| input.bytes() == b"p\xed\xa0\x80")
            })
            .unwrap();
        assert!(invocation.original_lookup.is_some());
        invocation.original_lookup = None;
        invocation.resolved_command_reference = None;
        invocation.resolved_definition = None;
        invocation.resolved_qualified_name = Some("::p\u{fffd}".to_owned());
        invocation.resolution_candidates = vec!["::p\u{fffd}".to_owned()];
        clear_reporting(&mut library);
        let index = WorkspaceIndex::from_documents([
            ("file:///library.tcl", &library),
            ("file:///caller.tcl", &caller),
        ]);
        assert!(index.original_invocations_of(&name, "", false).is_empty());
        assert!(index.original_invocations_of(&name, "", true).is_empty());
    }

    #[test]
    fn original_reverse_index_rejects_input_from_another_retained_source() {
        let library = original_analysis("proc p\\uD800 {} {}\n");
        let mut caller = original_analysis("p\\uD800\n");
        let other = original_analysis("p\\uD801\n");
        let name = original_names(&library).remove(0);
        let other_input = other
            .command_invocations
            .iter()
            .find_map(|invocation| invocation.original_name_input.clone())
            .unwrap();
        let invocation = caller
            .command_invocations
            .iter_mut()
            .find(|invocation| invocation.original_lookup.is_some())
            .unwrap();
        assert_ne!(invocation.original_name_input.as_ref(), Some(&other_input));
        invocation.original_name_input = Some(other_input);
        invocation.resolved_command_reference = None;
        invocation.resolved_definition = None;
        let index = WorkspaceIndex::from_documents([
            ("file:///library.tcl", &library),
            ("file:///caller.tcl", &caller),
        ]);
        assert!(index.original_invocations_of(&name, "", false).is_empty());
    }

    #[test]
    fn original_body_only_replacement_keeps_minimal_settlement_dependencies() {
        let mut library =
            original_analysis("proc p\\uD800 {} {return FIRST}\nproc p\\uD801 {} {}\n");
        let mut caller = original_analysis("p\\uD800\np\\uD801\n");
        let names = original_names(&library);
        clear_reporting(&mut library);
        clear_reporting(&mut caller);
        let mut index = WorkspaceIndex::from_documents([
            ("file:///library.tcl", &library),
            ("file:///caller.tcl", &caller),
        ]);
        assert!(library.original_completed_command_world().is_some());
        for name in &names {
            assert_eq!(index.original_invocations_of(name, "", false).len(), 1);
        }
        let cold = index.settled_invocations[0].settled_documents();
        let old_dependencies = index.docs[0].settlement_dependencies();
        let mut replacement =
            original_analysis("proc p\\uD800 {} {return SECOND}\nproc p\\uD801 {} {}\n");
        clear_reporting(&mut replacement);
        index.replace_document("file:///library.tcl", &replacement);
        assert!(
            old_dependencies == index.docs[0].settlement_dependencies(),
            "body/source-image changes cannot enter the settlement dependency header"
        );
        for name in &names {
            assert_eq!(index.original_invocations_of(name, "", false).len(), 1);
        }
        assert_eq!(
            index.settled_invocations[0].settled_documents(),
            cold + 1,
            "only the changed document needs settlement when exact published slots are unchanged"
        );
        let rebuilt = WorkspaceIndex::from_documents([
            ("file:///library.tcl", &replacement),
            ("file:///caller.tcl", &caller),
        ]);
        for name in &names {
            assert_eq!(
                index.original_invocations_of(name, "", false).len(),
                rebuilt.original_invocations_of(name, "", false).len()
            );
        }
        let mut removed = original_analysis("proc p\\uD801 {} {}\n");
        clear_reporting(&mut removed);
        index.replace_document("file:///library.tcl", &removed);
        assert!(
            index
                .original_invocations_of(&names[0], "", false)
                .is_empty()
        );
        assert_eq!(index.original_invocations_of(&names[1], "", false).len(), 1);
        assert_eq!(
            index.settled_invocations[0].settled_documents(),
            cold + 3,
            "changing an original publication re-settles the caller too"
        );
    }
}

#[cfg(test)]
mod original_namespace_export_index_tests {
    use super::*;
    use crate::namespace_import::NamespaceExportOracle;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_export_snapshot_joins_opaque_slots_and_rejects_ui_and_policy_substitutes() {
        // Implementation contract: naming.namespace.original-export-source-advice
        // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
        let source = r"proc p\uD800 {} {}; proc p\uD801 {} {}; namespace export p\uD800; namespace export -clear p\uD801";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let declarations = analysis
            .original_procedure_declarations()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(declarations.len(), 2);
        analysis.namespace_exports.clear();
        analysis.all_procs.clear();
        let index = WorkspaceIndex::from_documents([("file:///exports.tcl", &analysis)]);
        let snapshot = index.export_snapshot();
        let ask = |at, name: &tcl_compiler::signature_scan::scope::SignatureSourceCommand| {
            snapshot.exported_at_original(
                name.slot(),
                name.policy(),
                RunPoint {
                    uri: "file:///exports.tcl",
                    at,
                    enclosing_body: None,
                },
            )
        };
        let first = declarations
            .iter()
            .find(|record| record.name().slot().simple.as_bytes() == b"p\xed\xa0\x80")
            .unwrap()
            .name();
        let second = declarations
            .iter()
            .find(|record| record.name().slot().simple.as_bytes() == b"p\xed\xa0\x81")
            .unwrap()
            .name();
        let at = u32::try_from(source.len()).unwrap();
        assert_eq!(ask(at, first), ExportVerdict::NotExported);
        assert_eq!(ask(at, second), ExportVerdict::Exported);
        assert_eq!(
            snapshot.exported_at(
                "::",
                "p\u{fffd}",
                RunPoint {
                    uri: "file:///exports.tcl",
                    at,
                    enclosing_body: None
                }
            ),
            ExportVerdict::Unknown
        );
        let foreign =
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V9_0);
        assert_eq!(
            snapshot.exported_at_original(
                second.slot(),
                foreign,
                RunPoint {
                    uri: "file:///exports.tcl",
                    at,
                    enclosing_body: None
                }
            ),
            ExportVerdict::Unknown
        );
    }

    #[test]
    fn original_import_index_retains_opaque_patterns_without_reporting_rows() {
        // Implementation contract: naming.namespace.original-export-source-advice
        // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
        let source = r"namespace import -force ::n\uD800::p\uD801";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let pattern = analysis
            .original_namespace_patterns()
            .next()
            .expect("genuine original import")
            .clone();
        assert!(pattern.forced());
        analysis.namespace_imports.clear();
        let index = WorkspaceIndex::from_documents([("file:///import.tcl", &analysis)]);
        let rows = index.original_namespace_patterns().collect::<Vec<_>>();
        assert_eq!(rows, vec![("file:///import.tcl", &pattern)]);
        assert_eq!(pattern.parts().tail.as_bytes(), b"p\xed\xa0\x81");
    }

    #[test]
    fn original_export_unknown_operand_blocks_source_absence_and_match_advice() {
        // Implementation contract: naming.namespace.original-export-source-advice
        // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
        let source = "proc p {} {}; namespace export p; namespace export $unavailable";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let name = analysis
            .original_procedure_declarations()
            .next()
            .unwrap()
            .name();
        assert!(!analysis.original_namespace_export_unknowns().is_empty());
        let index = WorkspaceIndex::from_documents([("file:///unknown.tcl", &analysis)]);
        assert_eq!(
            index.export_snapshot().exported_at_original(
                name.slot(),
                name.policy(),
                RunPoint {
                    uri: "file:///unknown.tcl",
                    at: u32::try_from(source.len()).unwrap(),
                    enclosing_body: None,
                }
            ),
            ExportVerdict::Unknown
        );
    }
}
