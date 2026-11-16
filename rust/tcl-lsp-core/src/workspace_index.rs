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
use tcl_registry::definer::DeclaredMemberVisibility;
use tcl_syntax::naming::{key_holder_and_tail, root_unrooted_key, unroot_rooted_key};

/// Readonly location of a conditional Logical workspace declaration candidate.
/// Reporting fields cannot recreate the source request that selected it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalWorkspaceCommandCandidate {
    revision: u64,
    image: tcl_lexer::SourceImage,
    /// Independently indexed declaring document.
    uri: String,
    /// Presentation name from the selected declaration.
    qualified_name: String,
    /// Original declaration Name span in that document.
    name_span: Span,
    /// Optional descriptive signature summary.
    detail: Option<String>,
    procedure: Option<tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration>,
    location: crate::original_indexed_location::LogicalIndexedDeclarationLocation,
}

impl LogicalWorkspaceCommandCandidate {
    /// The actual selected immutable source image, without loaded-provider claims.
    #[must_use]
    pub fn source_image(&self) -> &tcl_lexer::SourceImage {
        &self.image
    }

    /// Genuine selected source header; class/nonprocedure candidates remain
    /// distinct occupants and cannot become a procedure via reporting ranges.
    #[must_use]
    pub fn procedure_declaration(
        &self,
    ) -> Option<&tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration> {
        self.procedure.as_ref()
    }

    /// Independently indexed declaring document; display names cannot replace it.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Presentation name from the selected declaration.
    #[must_use]
    pub fn qualified_name(&self) -> &str {
        &self.qualified_name
    }

    /// Immutable original selected Name geometry in the declaring document.
    #[must_use]
    pub const fn name_span(&self) -> Span {
        self.name_span
    }

    /// Optional descriptive signature summary.
    #[must_use]
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Current declaring bytes must still match before converting the original span.
    #[must_use]
    pub fn matches_source(&self, source: &str) -> bool {
        self.image.bytes() == source.as_bytes()
    }
}

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
    /// Completed Logical source membership, independent of original Native ledgers.
    pub logical_declaration: Option<tcl_compiler::analyser::types::LogicalSourceClassDeclaration>,
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
    /// Descriptor family selected from this record's actual analysis input and
    /// Registry at publication. This is metadata, not native class identity.
    definer_family: Option<tcl_registry::definer::DefinerFamily>,
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
    pub const fn is_itcl(&self, _dialect: &'static tcl_dialect::DialectProfile) -> bool {
        matches!(
            self.definer_family,
            Some(tcl_registry::definer::DefinerFamily::Itcl)
        )
    }

    /// Retained descriptor family, independently of callable class or method
    /// identity. Unknown does not establish a different dispatch family.
    #[must_use]
    pub const fn definer_family(&self) -> Option<tcl_registry::definer::DefinerFamily> {
        self.definer_family
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
            .find(|m| m.name == name && m.kind != MethodKind::ClassMethod.as_str())
    }

    /// The typed record for the *class-receiver* member `name` — a
    /// `classmethod` / `self method` / snit `typemethod` — if this record
    /// defines one.  The counterpart of [`Self::instance_method`].
    #[must_use]
    pub fn class_method(&self, name: &str) -> Option<&WorkspaceMethod> {
        self.methods
            .iter()
            .find(|m| m.name == name && m.kind == MethodKind::ClassMethod.as_str())
    }
}

/// Which receiver side a [`WorkspaceMethod`] is declared on — the same split
/// [`WorkspaceClass::instance_method`] / [`WorkspaceClass::class_method`] make,
/// named once so the member fold and the tombstone lookup agree on it.
#[must_use]
fn method_side(m: &WorkspaceMethod) -> MemberSide {
    if m.kind == MethodKind::ClassMethod.as_str() {
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
    /// Selected source domain, independent of whether a lookup was available.
    /// Missing input does not authorize report-name settlement.
    logical_source_domain: Option<bool>,
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

/// Original namespace-variable declaration with a checked C compatibility key.
/// Local frames and unrepresentable/Jim keys retain their separate typed
/// original inventory; this row supplies no runtime cell or publication proof.
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

/// Original namespace-variable source occurrence with a checked outward C key.
/// Conditional local aliases remain possible source relationships; this row
/// supplies neither an entered link nor completed variable access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceVariableRef {
    /// Document the occurrence is in.
    pub uri: String,
    /// Checked outward C source coordinate, never reconstructed from a label.
    pub qualified_name: String,
    /// Byte span of the name token as written.
    pub span: Span,
}

/// A conditional original alias declaration retained for document candidacy.
/// Its typed source target and unresolved applicability remain distinct from
/// an entered link, current cell or complete runtime alias inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceVariableAlias {
    uri: String,
    source: tcl_compiler::signature_scan::variable_symbol::OriginalVariableAliasTemplate,
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

/// One `source FILE` reference recorded in the index.
///
/// Tracks where a document loads another file so a file rename can
/// rewrite the dependent's `source` literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSource {
    /// Actual current-interpreter source association and its retained schema.
    /// This remains separate from child-interpreter source visibility advice.
    pub original_project_source_load: Option<
        std::sync::Arc<
            tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad,
        >,
    >,
    /// Compiler-issued current cut, conditional association or unknown debt
    /// from this row's actual parent analysis. This retains the full finite
    /// plan/body issuer, rather than promoting a point alone to a frontier.
    /// The host independently rejoins the parent's physical publication.
    pub original_source_frontier: Option<
        std::sync::Arc<
            tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceFrontier,
        >,
    >,
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

/// A readonly Logical call recipe retained by an indexed source view.
/// Membership follows the actual candidate allocation, not a report label or
/// revision token. It establishes neither command existence nor a target binding.
#[derive(Debug, Clone)]
pub struct WorkspaceLogicalCommandCall {
    uri: String,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    source: WorkspaceDiagnosticSourceContext,
    candidates: Arc<tcl_compiler::command_binding::OriginalLogicalCommandCandidates>,
}

/// Conditional own class-object method cards selected from an actual caller
/// recipe and independent indexed original class headers. No dispatch follows.
#[derive(Debug, Clone)]
pub struct WorkspaceLogicalClassMemberTargets {
    /// Actual retained caller source allocation, rechecked after asynchronous reads.
    pub call: WorkspaceLogicalCommandCall,
    /// Genuine direct public source method declarations, possibly unavailable/empty.
    pub targets: Vec<crate::original_indexed_location::LogicalIndexedMemberLocation>,
}

/// Original member-body caller and independently selected same-side declarations.
/// The caller is the existing genuine member allocation, not a command namespace.
#[derive(Debug, Clone)]
pub struct WorkspaceLogicalSelfMemberTargets {
    /// Exact original enclosing source member, rechecked through existing membership.
    pub caller: crate::original_indexed_location::LogicalIndexedMemberLocation,
    /// Conditional original source declarations, with no dispatch entry claim.
    pub targets: Vec<crate::original_indexed_location::LogicalIndexedMemberLocation>,
}

impl WorkspaceLogicalCommandCall {
    /// Original scope and word geometry for declaration suggestions only.
    #[must_use]
    pub fn candidates(&self) -> &tcl_compiler::command_binding::OriginalLogicalCommandCandidates {
        &self.candidates
    }

    /// Whether this exact call receipt remains in the indexed source views.
    /// Accumulating another view keeps it; replacing or removing its document
    /// retracts it. External document currency remains the caller's obligation.
    #[must_use]
    pub fn is_retained_in(&self, index: &WorkspaceIndex) -> bool {
        index
            .slots
            .get(&self.uri)
            .and_then(|slot| index.docs.get(*slot))
            .is_some_and(|document| {
                document
                    .logical_command_calls
                    .iter()
                    .any(|call| Arc::ptr_eq(&call.candidates, &self.candidates))
                    && self
                        .candidates
                        .matches_source(self.source.image(), &self.input)
            })
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
    original_source_views: Vec<WorkspaceOriginalSourceView>,
    /// Index-wide mutation token for the indexed revision. The server uses it
    /// to prove that a closed file did not reindex while its source was being
    /// loaded for byte-span to LSP-range conversion.
    revision: u64,
    /// Descriptive analysed dialect label. Current Registry queries retain the
    /// actual input and complete source context independently of this label.
    dialect: String,
    diagnostic_source: Option<WorkspaceDiagnosticSourceContext>,
    /// Genuine source-point/body/input owners; no executed chronology.
    source_order_context: Option<crate::source_graph::SourceOrderContext>,
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
    logical_command_calls: Vec<WorkspaceLogicalCommandCall>,
    logical_namespace_locations:
        Vec<crate::original_indexed_location::LogicalIndexedNamespaceLocation>,
    logical_procedure_declarations:
        Vec<tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration>,
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
    alias_source_coverage_complete: bool,
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
    /// Command-store currency read by the import metadata conflict gates.
    registry: Option<tcl_registry::RegistrySemanticKey>,
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
    /// Actual immutable command store for this indexed source revision. The
    /// full source/configuration guard was captured at index ingress; its
    /// exact store must still match the retained input. No Native capability
    /// or loaded-package effect is supplied by this metadata borrow.
    fn registry(&self) -> Option<&tcl_registry::CommandRegistry> {
        let input = self.original_source_input.as_ref()?;
        let source = self.diagnostic_source.as_ref()?;
        let registry = input.borrowed_context_registry().commands().as_ref();
        (input.lexer_config() == source.config
            && registry.snapshot().semantic_key() == source.registry)
            .then_some(registry)
    }

    fn settlement_dependencies(&self) -> SettlementDependencies {
        SettlementDependencies {
            dialect: self.dialect.clone(),
            registry: self
                .registry()
                .map(|registry| registry.snapshot().semantic_key()),
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
            original_publications: self.original_publication_dependencies(),
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

    fn original_publication_dependencies(&self) -> Vec<OriginalPublicationDependency> {
        self.original_command_world
            .iter()
            .flat_map(tcl_compiler::command_binding::OriginalCompletedCommandWorld::declarations)
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
            .collect()
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
            original_source_views,
            revision: _,
            dialect,
            diagnostic_source,
            source_order_context,
            procs,
            classes,
            original_command_world,
            original_source_procedures,
            original_source_classes,
            original_source_input,
            logical_command_calls,
            logical_namespace_locations,
            logical_procedure_declarations,
            original_procs,
            original_classes,
            original_symbols,
            original_vendor_declarations,
            original_method_queries,
            original_variables,
            variables,
            variable_refs,
            variable_aliases,
            alias_source_coverage_complete,
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
        original_source_views.clear();
        dialect.clear();
        *diagnostic_source = None;
        *source_order_context = None;
        path_constant_assignments.clear();
        procs.clear();
        classes.clear();
        *original_command_world = None;
        *original_source_procedures = None;
        *original_source_classes = None;
        *original_source_input = None;
        logical_command_calls.clear();
        logical_namespace_locations.clear();
        logical_procedure_declarations.clear();
        original_procs.clear();
        original_classes.clear();
        original_symbols.clear();
        original_vendor_declarations.clear();
        original_method_queries.clear();
        original_variables.clear();
        variables.clear();
        variable_refs.clear();
        variable_aliases.clear();
        *alias_source_coverage_complete = false;
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
            self.collect_original_symbols_matching(lower_query, limit, out);
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
                    logical_source: None,
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
                    logical_source: None,
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
                        logical_source: None,
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
                        logical_source: None,
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
                    logical_source: None,
                });
            }
        }
    }

    fn collect_original_symbols_matching(
        &self,
        lower_query: &str,
        limit: usize,
        out: &mut Vec<IndexedWorkspaceSymbol>,
    ) {
        for symbol in &self.original_symbols {
            if out.len() >= limit {
                return;
            }
            if matches_query(&symbol.name, lower_query)
                || symbol.container_name.as_ref().is_some_and(|container| {
                    matches_query(
                        &tcl_syntax::naming::rooted_member_key(container, &symbol.name),
                        lower_query,
                    )
                })
            {
                out.push(symbol.clone());
            }
        }
    }

    /// Lift one document's analysis into this record set.
    ///
    /// The caller ([`WorkspaceIndex::add_document`]) has already cleared the
    /// slot, so this only ever appends.
    fn index_document(&mut self, uri: &str, analysis: &AnalysisResult) {
        let had_source_view = !self.uri.is_empty();
        let source_order_context = crate::source_graph::SourceOrderContext::capture(analysis);
        uri.clone_into(&mut self.uri);
        self.dialect.clone_from(&analysis.dialect);
        self.diagnostic_source = WorkspaceDiagnosticSourceContext::for_analysis(analysis);
        // Accumulated source-site views cannot replace an incompatible
        // original input/Realm with the latest reporting view.
        self.source_order_context =
            if !had_source_view || self.source_order_context == source_order_context {
                source_order_context
            } else {
                None
            };
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
        if let (Some(context), Some(input)) = (&self.diagnostic_source, &analysis.resolved_input)
            && input.has_logical_source_name_context()
            && let Ok(source) = context.image().try_text()
            && let Some(calls) = tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates(source, analysis)
        {
            self.logical_command_calls.extend(calls.into_iter().filter_map(|candidates| {
                candidates.matches_source(context.image(), input).then(|| WorkspaceLogicalCommandCall {
                    uri: uri.to_owned(), input: input.clone(), source: context.clone(), candidates: Arc::new(candidates),
                })
            }));
        }
        self.logical_namespace_locations
            .extend(analysis.namespace_refs.iter().filter_map(|row| {
                crate::original_indexed_location::LogicalIndexedNamespaceLocation::capture(
                    uri, analysis, row,
                )
            }));
        self.original_namespace_patterns
            .extend(analysis.original_namespace_patterns().cloned());
        if let Some(context) = self.diagnostic_source.as_ref()
            && let Ok(source) = std::str::from_utf8(context.image().bytes())
        {
            self.original_vendor_declarations.extend(
                crate::vendor_declaration::retained_declarations(source, analysis)
                    .unwrap_or_default(),
            );
            self.original_symbols
                .extend(crate::workspace_symbols::original_symbols(
                    uri, source, analysis, context,
                ));
        }
        self.original_method_queries
            .extend(crate::method_symbol::source_queries(analysis));
        self.original_procs
            .extend(analysis.original_procedure_declarations().cloned());
        self.original_classes
            .extend(analysis.original_class_declarations().cloned());
        self.index_document_declarations(uri, analysis);
        self.index_document_invocations(uri, analysis);
        self.index_sources(uri, analysis);
        self.index_package_declarations(uri, analysis);
        self.index_namespace_exports(uri, analysis);
        self.index_import_lifecycle(uri, analysis);
        self.index_command_links(uri, analysis);
    }

    fn index_document_declarations(&mut self, uri: &str, analysis: &AnalysisResult) {
        // `all_procs` / `all_classes` / a class's `methods` are `HashMap`s, so
        // iterating them directly would order this document's index entries by
        // the process's random hash seed — and consumers that answer with the
        // *first* matching record (go-to-definition's dispatch entry, most
        // visibly) then answer differently run to run.  Source
        // order is the stable, meaningful order: it is also the order the
        // records take effect in when the file is sourced.
        for proc_def in sorted_by_span(analysis.all_procs.values(), |p| p.name_span) {
            if let Some(declaration) = analysis.logical_source_procedure_declaration(proc_def) {
                self.logical_procedure_declarations
                    .push(declaration.clone());
            }
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
        if let Some(context) = self.diagnostic_source.as_ref()
            && let Ok(source) = context.image().try_text()
        {
            self.original_variables.extend(
                analysis
                    .original_variable_symbols
                    .iter()
                    .filter(|occurrence| {
                        occurrence.symbol().is_namespace()
                            && crate::original_name_edit::original_input_matches_source(
                                source,
                                analysis,
                                occurrence.original_name_input(),
                                occurrence.span(),
                            )
                    })
                    .cloned(),
            );
        }
        self.index_variables(uri, analysis);
        self.index_namespace_refs(uri, analysis);
    }

    fn index_document_invocations(&mut self, uri: &str, analysis: &AnalysisResult) {
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
                logical_source_domain: analysis.resolved_input.as_ref().map(
                    tcl_compiler::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
                ),
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
    }

    fn index_package_declarations(&mut self, uri: &str, analysis: &AnalysisResult) {
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
    }

    fn index_namespace_exports(&mut self, uri: &str, analysis: &AnalysisResult) {
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
    }

    /// The document's `source` rows, plus the raw path-constant candidates
    /// the edge resolver folds them with — recorded together because they
    /// are consumed together (a computed row without its constants is
    /// unresolvable, constants without rows are inert).
    fn index_sources(&mut self, uri: &str, analysis: &AnalysisResult) {
        for target in &analysis.source_targets {
            if let Some(original) = &target.original_project_source_load
                && !analysis.resolved_input.as_ref().is_some_and(|input| {
                    original.path_word().is_some_and(|word| {
                        analysis.matches_original_source_image(word.image(), input.lexer_config())
                            && original.matches_source(word.image(), input)
                    })
                })
            {
                continue;
            }
            if let Some(original) = &target.original_interpreter_source_load
                && !original.matches_analysis(analysis)
            {
                continue;
            }
            self.sources.push(WorkspaceSource {
                original_project_source_load: target.original_project_source_load.clone(),
                original_source_frontier: target
                    .original_project_source_load
                    .as_ref()
                    .map(|load| std::sync::Arc::new(load.frontier(analysis))),
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
                        exported: m.visibility == DeclaredMemberVisibility::Public.as_str(),
                        private: m.visibility == DeclaredMemberVisibility::Private.as_str(),
                        is_self_method: m.is_self_method,
                        name_span: m.name_span,
                    })
                    .chain(
                        sorted_by_span(class_def.class_methods.values(), |m| m.name_span)
                            .into_iter()
                            .map(|m| WorkspaceMethod {
                                name: m.name.clone(),
                                kind: MethodKind::ClassMethod.as_str().to_string(),
                                exported: m.visibility == DeclaredMemberVisibility::Public.as_str(),
                                private: m.visibility == DeclaredMemberVisibility::Private.as_str(),
                                is_self_method: m.is_self_method,
                                name_span: m.name_span,
                            }),
                    )
                    .collect();
            self.classes.push(WorkspaceClass {
                logical_declaration: analysis
                    .logical_source_class_declaration(class_def)
                    .cloned(),
                original_declaration: original
                    .map(|original| original.original_occurrence().clone()),
                original_members: original
                    .map(|original| original.metadata().original_members.clone())
                    .or_else(|| {
                        analysis
                            .allows_retained_logical_declaration_advice()
                            .then(|| class_def.original_members.clone())
                    }),
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
                definer_family: crate::definition::class_definer_family(analysis, class_def),
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

    /// Index current original source symbols and conditional alias declarations.
    /// Compatibility labels are checked outward C projections only; they never
    /// reconstruct a source symbol or claim actual cells or alias completion.
    fn index_variables(&mut self, uri: &str, analysis: &AnalysisResult) {
        let Some(context) = self.diagnostic_source.as_ref() else {
            return;
        };
        let Ok(source) = context.image().try_text() else {
            return;
        };
        self.alias_source_coverage_complete =
            analysis.original_variable_alias_source_is_complete(context.image(), context.config());
        for occurrence in &analysis.original_variable_symbols {
            if !crate::original_name_edit::original_input_matches_source(
                source,
                analysis,
                occurrence.original_name_input(),
                occurrence.span(),
            ) {
                continue;
            }
            let Some(qualified_name) = occurrence.symbol().checked_c_namespace_key() else {
                continue;
            };
            if occurrence.is_declaration() {
                let Some(name) = occurrence
                    .symbol()
                    .rename_tail()
                    .and_then(|name| std::str::from_utf8(name).ok())
                else {
                    continue;
                };
                self.variables.push(WorkspaceVariable {
                    uri: uri.to_owned(),
                    name: name.to_owned(),
                    qualified_name,
                    name_span: occurrence.span(),
                });
            } else {
                self.variable_refs.push(WorkspaceVariableRef {
                    uri: uri.to_owned(),
                    qualified_name,
                    span: occurrence.span(),
                });
            }
        }
        if let Some(templates) =
            analysis.original_variable_alias_templates_in_source(context.image(), context.config())
        {
            self.variable_aliases.extend(
                templates
                    .into_iter()
                    .filter(tcl_compiler::signature_scan::variable_symbol::OriginalVariableAliasTemplate::is_declaration)
                    .map(|source| WorkspaceVariableAlias {
                        uri: uri.to_owned(),
                        source,
                    }),
            );
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
            if !tcl_syntax::glob::is_literal(&source.tail_pattern) {
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
        self.index_alias_and_rename_links(uri, analysis);
    }

    fn index_alias_and_rename_links(&mut self, uri: &str, analysis: &AnalysisResult) {
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
    /// Provider-issued workspace roster captured for the current scan. A
    /// browser host must retain a current complete receipt before absence can
    /// authorize cross-document rename; positive readonly rows remain useful.
    source_inventory_provider: Option<crate::vfs::WorkspaceSourceInventoryProvider>,
    source_inventory_observations: Vec<crate::vfs::WorkspaceSourceInventoryObservation>,
    /// Completeness of the server's own bounded directory scan, independent
    /// of host census completeness.
    source_inventory_scan_complete: Option<bool>,
    /// Changes when the scan receipt itself is replaced or invalidated.
    source_inventory_revision: u64,
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

    /// Conditional source navigation order, retaining the exact original
    /// source/body/schema/input owners and no executed event completion.
    /// The existing forest is built at most once per
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
            crate::source_graph::RunOrder::for_source_navigation(
                &edges,
                self.docs
                    .iter()
                    .filter_map(|doc| Some((doc.uri.clone(), doc.source_order_context.clone()?))),
            )
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
        // Both carry whole current source-point owners, even when command
        // settlement can preserve other documents after a body-only edit.
        self.run_order = Derived::default();
        self.export_snapshot = Derived::default();
        if resolution_inputs_changed {
            self.command_names = Derived::default();
            self.live_links = Derived::default();
            self.defined_names = <[Derived<HashSet<String>>; 2]>::default();
            self.command_link_map = Derived::default();
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
                                || self.registry_for(&l.uri).is_some_and(|r| {
                                    r.declares_command_at_rooted_key(&l.linked_qname)
                                })
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

    /// Metadata store retained with the indexed document's complete source
    /// context. Missing or mismatched input does not recapture a profile store.
    fn registry_for(&self, uri: &str) -> Option<&tcl_registry::CommandRegistry> {
        self.docs[*self.slots.get(uri)?].registry()
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
        self.docs[slot].original_source_views.clear();
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

    /// Match one actual indexed provider to the selected Logical declaration.
    /// Complete source/input, canonical member and document ownership stay exact.
    #[must_use]
    pub fn logical_member_provider_matches_target(
        &self,
        declaring_uri: &str,
        provider: &WorkspaceClass,
        target: &crate::call_hierarchy::LogicalCallTarget<'_>,
    ) -> bool {
        let Some(document) = self
            .slots
            .get(&provider.uri)
            .and_then(|slot| self.docs.get(*slot))
        else {
            return false;
        };
        let (Some(context), Some(input)) =
            (&document.diagnostic_source, &document.original_source_input)
        else {
            return false;
        };
        document
            .classes
            .iter()
            .any(|current| std::ptr::eq(current, provider))
            && target.matches_indexed_member_provider(declaring_uri, provider, context, input)
    }

    /// Declaration-side Logical names from this document's retained source views.
    /// Complete original occurrences select records; missing views grant no fallback.
    #[must_use]
    pub fn logical_declaration_source_names_at(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
    ) -> Option<Vec<String>> {
        let config = analysis.body_lexer_config?;
        let image = tcl_lexer::SourceImage::document(source);
        if !analysis.allows_retained_logical_declaration_advice()
            || !analysis.matches_original_source_image(&image, config)
        {
            return Some(Vec::new());
        }
        let covers = |span: Span| span.start() <= offset && offset < span.end();
        let procedures = analysis
            .original_procedure_declarations()
            .filter(|record| covers(record.name_input().span()))
            .map(|record| (record.original_occurrence(), false, record.name().policy()));
        let classes = analysis
            .original_class_declarations()
            .filter(|record| covers(record.name_input().span()))
            .map(|record| (record.original_occurrence(), true, record.name().policy()));
        let mut selected = procedures.chain(classes);
        let (occurrence, class, policy) = selected.next()?;
        if selected.next().is_some() {
            return Some(Vec::new());
        }
        let Some(document) = self.slots.get(uri).and_then(|slot| self.docs.get(*slot)) else {
            return Some(Vec::new());
        };
        let Some(context) = document.diagnostic_source.as_ref() else {
            return Some(Vec::new());
        };
        if context.uses_original_names()
            || context.image() != &image
            || context.config() != config
            || document.original_source_input.as_ref() != analysis.resolved_input.as_ref()
        {
            return Some(Vec::new());
        }
        let records = document
            .original_procs
            .iter()
            .map(|record| (record.original_occurrence(), record.name(), false))
            .chain(
                document
                    .original_classes
                    .iter()
                    .map(|record| (record.original_occurrence(), record.name(), true)),
            );
        let mut slots = Vec::new();
        for (candidate, name, candidate_class) in records {
            if candidate == occurrence
                && candidate_class == class
                && name.policy() == policy
                && !slots.contains(name.slot())
            {
                slots.push(name.slot().clone());
            }
        }
        let names = slots
            .iter()
            .filter_map(|slot| {
                tcl_syntax::naming::native_command_source_spelling(policy.recipe(), slot)
            })
            .collect();
        Some(names)
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

    /// Install the source-store census captured for a workspace scan. This
    /// records the actual provider observations, not a detached completion flag.
    pub fn set_source_inventory_snapshot(
        &mut self,
        provider: crate::vfs::WorkspaceSourceInventoryProvider,
        observations: Vec<crate::vfs::WorkspaceSourceInventoryObservation>,
    ) {
        self.set_source_inventory_snapshot_with_scan_status(provider, observations, true);
    }

    /// Install provider observations together with the server's own file-scan
    /// result. A complete host census cannot certify a truncated server index.
    pub fn set_source_inventory_snapshot_with_scan_status(
        &mut self,
        provider: crate::vfs::WorkspaceSourceInventoryProvider,
        observations: Vec<crate::vfs::WorkspaceSourceInventoryObservation>,
        scan_complete: bool,
    ) {
        self.source_inventory_provider = Some(provider);
        self.source_inventory_observations = observations;
        self.source_inventory_scan_complete = Some(scan_complete);
        self.source_inventory_revision = self
            .source_inventory_revision
            .checked_add(1)
            .expect("source inventory revision exhausted");
    }

    /// Withdraw absence authority while a new workspace scan is in progress.
    pub fn invalidate_source_inventory_scan(&mut self) {
        self.source_inventory_scan_complete = Some(false);
        self.source_inventory_revision = self
            .source_inventory_revision
            .checked_add(1)
            .expect("source inventory revision exhausted");
    }

    /// Whether the supplied observations exactly match the stored receipt
    /// snapshot, without interpreting the server's scan-completeness result.
    /// This is only for deciding whether a known failed scan would be repeated;
    /// it grants no absence or edit authority.
    #[must_use]
    pub fn source_inventory_observations_match(
        &self,
        provider: crate::vfs::WorkspaceSourceInventoryProvider,
        observations: &[crate::vfs::WorkspaceSourceInventoryObservation],
    ) -> bool {
        self.source_inventory_provider == Some(provider)
            && observations == self.source_inventory_observations
    }

    /// Revision for the exact provider receipt used by a workspace query plan.
    #[must_use]
    pub const fn source_inventory_revision(&self) -> u64 {
        self.source_inventory_revision
    }

    /// Whether the server's own file scan was complete for this index.
    #[must_use]
    pub const fn source_inventory_scan_complete(&self) -> Option<bool> {
        self.source_inventory_scan_complete
    }

    /// Roots whose source inventory was captured with this workspace index.
    #[must_use]
    pub fn source_inventory_roots(&self) -> Vec<std::path::PathBuf> {
        self.source_inventory_observations
            .iter()
            .filter_map(|observation| match observation {
                crate::vfs::WorkspaceSourceInventoryObservation::NotTracked => None,
                crate::vfs::WorkspaceSourceInventoryObservation::Missing { root, .. } => {
                    Some(root.clone())
                }
                crate::vfs::WorkspaceSourceInventoryObservation::Stale { receipt, .. }
                | crate::vfs::WorkspaceSourceInventoryObservation::Current(receipt) => {
                    Some(receipt.root.clone())
                }
            })
            .collect()
    }

    /// Whether a new actual provider snapshot is the one this index used and
    /// still admits every selected root. Used at the final workspace edit gate.
    #[must_use]
    pub fn source_inventory_is_current(
        &self,
        provider: crate::vfs::WorkspaceSourceInventoryProvider,
        current: &[crate::vfs::WorkspaceSourceInventoryObservation],
    ) -> bool {
        if self.source_inventory_scan_complete == Some(false) {
            return false;
        }
        if provider == crate::vfs::WorkspaceSourceInventoryProvider::Filesystem {
            return self.source_inventory_provider.is_none()
                || self.source_inventory_provider == Some(provider);
        }
        if self.source_inventory_provider != Some(provider) {
            return false;
        }
        !current.is_empty()
            && current == self.source_inventory_observations
            && current
                .iter()
                .all(crate::vfs::WorkspaceSourceInventoryObservation::admits_complete_inventory)
    }

    /// Recheck the scan's receipt against its live source store. A workspace
    /// edit may use absence only while this exact census still matches.
    #[must_use]
    pub fn source_inventory_matches_store(&self, store: &dyn crate::vfs::SourceStore) -> bool {
        let provider = store.workspace_source_inventory_provider();
        let roots = self.source_inventory_roots();
        let current = roots
            .iter()
            .map(|root| store.workspace_source_inventory(root))
            .collect::<Vec<_>>();
        self.source_inventory_is_current(provider, &current)
    }

    /// Whether every retained provider independently selected lexical advice.
    /// A host-tracked source roster and the server's own scan must both be
    /// complete in this index snapshot; callers with a live store must also
    /// validate currentness.
    #[must_use]
    pub fn allows_lexical_rename_advice(&self) -> bool {
        let sources_admit_absence = match self.source_inventory_provider {
            Some(crate::vfs::WorkspaceSourceInventoryProvider::HostReported) => {
                self.source_inventory_scan_complete != Some(false)
                    && !self.source_inventory_observations.is_empty()
                    && self.source_inventory_observations.iter().all(
                        crate::vfs::WorkspaceSourceInventoryObservation::admits_complete_inventory,
                    )
            }
            Some(crate::vfs::WorkspaceSourceInventoryProvider::Filesystem) => {
                self.source_inventory_scan_complete != Some(false)
            }
            None => true,
            Some(crate::vfs::WorkspaceSourceInventoryProvider::Untracked) => false,
        };
        sources_admit_absence
            && !self.slots.is_empty()
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

    /// Current source-action input from an actual indexed row. Reporting path
    /// text/ranges cannot replace the retained FILE operand and source owner.
    #[must_use]
    pub fn source_file_reference(
        &self,
        source: &WorkspaceSource,
    ) -> Option<(
        u64,
        &tcl_compiler::auto_path_eval::OriginalSourcePathExpression,
    )> {
        let document = self.docs.get(*self.slots.get(&source.uri)?)?;
        let source = document
            .sources
            .iter()
            .find(|row| std::ptr::eq(*row, source))?;
        let original = source.original_path_expression.as_ref()?;
        original.file_reference_value()?;
        original
            .matches_assignments(&document.path_constant_assignments)
            .then_some(())?;
        Some((document.revision, original))
    }

    /// Join a proposed host file action to the same indexed source generation
    /// and independently obtained current analysis before publishing edits.
    #[must_use]
    pub fn source_file_reference_is_current(
        &self,
        uri: &str,
        revision: u64,
        original: &tcl_compiler::auto_path_eval::OriginalSourcePathExpression,
        analysis: &AnalysisResult,
    ) -> bool {
        self.document_revision(uri) == Some(revision)
            && original.matches_analysis(analysis)
            && self.sources().any(|row| {
                row.uri == uri
                    && row.original_path_expression.as_ref() == Some(original)
                    && self
                        .source_file_reference(row)
                        .is_some_and(|(held, _)| held == revision)
            })
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
        // Logical compatibility rows are report candidates, not source facts.
        // Reattach the actual target document's retained source/config/Registry
        // context so the server can map their original spans only against that
        // exact image. Cross-document effective-member rows use the arrival
        // document's own context, not the class record that contributed them.
        for symbol in &mut out {
            if symbol.original_location.is_some() {
                continue;
            }
            let Some(slot) = self.slots.get(&symbol.uri) else {
                continue;
            };
            let Some(document) = self.docs.get(*slot) else {
                continue;
            };
            symbol.logical_source = document.diagnostic_source.as_ref().and_then(|context| {
                crate::workspace_symbols::LogicalWorkspaceSymbolSource::capture(
                    symbol.uri.clone(),
                    context,
                    symbol.name_span,
                    document.revision,
                )
            });
        }
        out
    }

    /// Full analysis input retained by the actual indexed source issuer.
    /// Borrowing it supplies no runtime lookup, member entry or edit authority.
    #[must_use]
    pub fn source_analysis_input(
        &self,
        uri: &str,
    ) -> Option<&tcl_compiler::analyser::ResolvedAnalysisInput> {
        self.docs
            .get(*self.slots.get(uri)?)?
            .original_source_input
            .as_ref()
    }

    /// One represented Logical call in an exact indexed source/input view.
    /// No represented call, ambiguous views or unavailable source yields no
    /// receipt; none of those results proves an empty command census.
    #[must_use]
    pub fn logical_command_call_at(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
    ) -> Option<WorkspaceLogicalCommandCall> {
        let input = analysis.resolved_input.as_ref()?;
        if !input.has_logical_source_name_context() {
            return None;
        }
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        if context.image().bytes() != source.as_bytes() {
            return None;
        }
        let document = self.docs.get(*self.slots.get(uri)?)?;
        let mut matches = document.logical_command_calls.iter().filter(|call| {
            call.input == *input
                && call.source == context
                && call.candidates.original_head().is_some_and(|head| {
                    let span = head.span();
                    span.start() <= offset && offset < span.end()
                })
        });
        let selected = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        Some(selected.clone())
    }

    /// Conditional external declaration cards selected by an actual retained
    /// Logical call. Occupied, deleted or unavailable source slots cannot be
    /// renewed from report names. An empty result is not a workspace census.
    #[must_use]
    pub fn logical_declaration_targets_for_call(
        &self,
        call: &WorkspaceLogicalCommandCall,
    ) -> Option<Vec<crate::original_indexed_location::LogicalIndexedDeclarationLocation>> {
        call.is_retained_in(self).then_some(())?;
        let coordinates = call.candidates().external_header_coordinates()?;
        let mut targets = Vec::new();
        for document in &self.docs {
            let (Some(context), Some(input)) =
                (&document.diagnostic_source, &document.original_source_input)
            else {
                continue;
            };
            if input != &call.input {
                continue;
            }
            for declaration in &document.logical_procedure_declarations {
                let Some(coordinate) = declaration.source_candidate_coordinate() else {
                    continue;
                };
                if coordinates.contains(&coordinate)
                    && let Some(target) = crate::original_indexed_location::LogicalIndexedDeclarationLocation::procedure(&document.uri, context, input, declaration) {
                        targets.push(target);
                    }
            }
            for declaration in document
                .classes
                .iter()
                .filter_map(|class| class.logical_declaration.as_ref())
            {
                if declaration
                    .source_candidate_coordinate()
                    .is_some_and(|coordinate| coordinates.contains(coordinate))
                    && let Some(target) =
                        crate::original_indexed_location::LogicalIndexedDeclarationLocation::class(
                            &document.uri,
                            context,
                            input,
                            declaration,
                        )
                {
                    targets.push(target);
                }
            }
        }
        Some(targets)
    }

    /// Actual immutable declaration allocation membership, independent of equal
    /// names or images. Adding a view keeps it; replacement/removal retracts it.
    #[must_use]
    pub fn logical_declaration_location_is_retained(
        &self,
        location: &crate::original_indexed_location::LogicalIndexedDeclarationLocation,
    ) -> bool {
        self.slots
            .get(location.uri())
            .and_then(|slot| self.docs.get(*slot))
            .is_some_and(|document| {
                document
                    .logical_procedure_declarations
                    .iter()
                    .any(|current| location.retains_procedure(current))
                    || document
                        .classes
                        .iter()
                        .filter_map(|class| class.logical_declaration.as_ref())
                        .any(|current| location.retains_class(current))
            })
    }

    /// Retain the actual indexed source recipe for this current Logical
    /// variable occurrence. Equal report keys cannot supply membership.
    #[must_use]
    pub fn logical_variable_location_for_occurrence(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        occurrence: &tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
    ) -> Option<crate::original_indexed_location::LogicalIndexedVariableLocation> {
        let input = analysis.resolved_input.as_ref()?;
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        (context.image().bytes() == source.as_bytes()).then_some(())?;
        let document = self.docs.get(*self.slots.get(uri)?)?;
        document
            .original_variables
            .iter()
            .any(|current| occurrence.shares_logical_source_allocation(current))
            .then_some(())?;
        crate::original_indexed_location::LogicalIndexedVariableLocation::capture(
            uri, &context, input, occurrence,
        )
    }

    /// Conditional same-symbol source occurrences retain actual Logical naming
    /// recipes and complete input correspondence. This is no cell census.
    pub fn logical_variable_source_locations(
        &self,
        request: &crate::original_indexed_location::LogicalIndexedVariableLocation,
    ) -> Vec<crate::original_indexed_location::LogicalIndexedVariableLocation> {
        if !self.logical_variable_location_is_retained(request) {
            return Vec::new();
        }
        self.docs
            .iter()
            .flat_map(|document| {
                document.original_variables.iter().filter_map(|occurrence| {
                    (document.original_source_input.as_ref() == Some(request.input())
                        && occurrence.symbol() == request.symbol())
                    .then_some(())?;
                    crate::original_indexed_location::LogicalIndexedVariableLocation::capture(
                        &document.uri,
                        document.diagnostic_source.as_ref()?,
                        document.original_source_input.as_ref()?,
                        occurrence,
                    )
                })
            })
            .collect()
    }

    /// Actual immutable variable recipe allocation retained by this document.
    #[must_use]
    pub fn logical_variable_location_is_retained(
        &self,
        location: &crate::original_indexed_location::LogicalIndexedVariableLocation,
    ) -> bool {
        self.slots
            .get(location.uri())
            .and_then(|slot| self.docs.get(*slot))
            .is_some_and(|document| {
                document
                    .original_variables
                    .iter()
                    .any(|occurrence| location.retains(occurrence))
            })
    }

    /// One exact represented namespace-role occurrence in the caller's actual
    /// indexed source view. Missing scope/role/word remains unavailable.
    #[must_use]
    pub fn logical_namespace_location_at(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
    ) -> Option<crate::original_indexed_location::LogicalIndexedNamespaceLocation> {
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        (context.image().bytes() == source.as_bytes()).then_some(())?;
        let document = self.docs.get(*self.slots.get(uri)?)?;
        let mut locations = analysis
            .namespace_refs
            .iter()
            .filter(|row| row.span.start() <= offset && offset < row.span.end())
            .filter_map(|row| {
                crate::original_indexed_location::LogicalIndexedNamespaceLocation::capture(
                    uri, analysis, row,
                )
            })
            .filter(|location| {
                document
                    .logical_namespace_locations
                    .iter()
                    .any(|current| location.retains(current))
            });
        let selected = locations.next()?;
        locations.next().is_none().then_some(selected)
    }

    /// Same typed source namespace under the actual request's complete input.
    /// Parent locations are independent positive geometry, never an absence proof.
    #[must_use]
    pub fn logical_namespace_source_locations(
        &self,
        request: &crate::original_indexed_location::LogicalIndexedNamespaceLocation,
        symbol: &crate::namespace_symbol::OriginalNamespaceSymbol,
    ) -> Vec<crate::original_indexed_location::LogicalIndexedNamespaceLocation> {
        if !request.names(symbol) || !self.logical_namespace_location_is_retained(request) {
            return Vec::new();
        }
        self.docs
            .iter()
            .flat_map(|document| document.logical_namespace_locations.iter())
            .filter(|location| location.input() == request.input() && location.names(symbol))
            .cloned()
            .collect()
    }

    /// Original declaring descendants may project a written implicit parent.
    #[must_use]
    pub fn logical_namespace_parent_locations(
        &self,
        request: &crate::original_indexed_location::LogicalIndexedNamespaceLocation,
        symbol: &crate::namespace_symbol::OriginalNamespaceSymbol,
    ) -> Vec<crate::original_indexed_location::LogicalIndexedNamespaceLocation> {
        if !request.names(symbol) || !self.logical_namespace_location_is_retained(request) {
            return Vec::new();
        }
        self.docs
            .iter()
            .flat_map(|document| document.logical_namespace_locations.iter())
            .filter(|location| location.input() == request.input())
            .filter_map(|location| location.for_implicit_parent(symbol))
            .collect()
    }

    /// Actual retained original namespace source allocation, not a report key.
    #[must_use]
    pub fn logical_namespace_location_is_retained(
        &self,
        location: &crate::original_indexed_location::LogicalIndexedNamespaceLocation,
    ) -> bool {
        self.slots
            .get(location.uri())
            .and_then(|slot| self.docs.get(*slot))
            .is_some_and(|document| {
                document
                    .logical_namespace_locations
                    .iter()
                    .any(|current| location.retains(current))
            })
    }

    fn logical_self_relation_classes(
        &self,
        analysis: &AnalysisResult,
        request: &tcl_compiler::analyser::types::LogicalSourceSelfSelector,
        receiver: &tcl_compiler::analyser::ClassDef,
    ) -> Option<Vec<tcl_compiler::analyser::types::LogicalSourceClassDeclaration>> {
        let input = analysis.resolved_input.as_ref()?;
        let mut supers = Vec::new();
        let mut mixins = Vec::new();
        for statement in request.relations() {
            let current = match statement.relation() {
                tcl_registry::definer::RelationSlot::Superclass => &mut supers,
                tcl_registry::definer::RelationSlot::Mixin => &mut mixins,
                tcl_registry::definer::RelationSlot::Filter => continue,
            };
            let mut selected = Vec::new();
            for operand in statement.operands() {
                match operand.target(analysis, receiver)? {
                    tcl_compiler::command_binding::LogicalSourceClassRelationTarget::Source(
                        class,
                    ) => {
                        selected.push(analysis.logical_source_class_declaration(class)?.clone());
                    }
                    tcl_compiler::command_binding::LogicalSourceClassRelationTarget::External(
                        coordinates,
                    ) => {
                        for document in &self.docs {
                            if document.original_source_input.as_ref() != Some(input) {
                                continue;
                            }
                            for class in document
                                .classes
                                .iter()
                                .filter_map(|class| class.logical_declaration.as_ref())
                            {
                                if class
                                    .source_candidate_coordinate()
                                    .is_some_and(|coordinate| coordinates.contains(coordinate))
                                {
                                    selected.push(class.clone());
                                }
                            }
                        }
                    }
                    tcl_compiler::command_binding::LogicalSourceClassRelationTarget::Registry => {}
                }
            }
            statement.apply_values(current, &selected);
        }
        let mut classes = vec![request.caller().class().clone()];
        classes.extend(supers);
        classes.extend(mixins);
        Some(classes)
    }

    /// Original self-selector advice from the actual member body and written
    /// class-relation operands. Results are source alternatives, never an MRO.
    #[must_use]
    pub fn logical_self_member_targets_at(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        cursor: u32,
    ) -> Option<WorkspaceLogicalSelfMemberTargets> {
        let request = analysis.logical_source_self_selector_at(source, cursor)?;
        let input = analysis.resolved_input.as_ref()?;
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        (context.image().bytes() == source.as_bytes()).then_some(())?;
        let caller =
            crate::original_indexed_location::LogicalIndexedMemberLocation::capture_self_source(
                uri,
                &context,
                input,
                request.caller().clone(),
                &request,
            )?;
        self.logical_member_location_is_retained(&caller)
            .then_some(())?;
        let receiver = analysis.logical_source_member_class(request.caller())?;
        let name = tcl_syntax::word_rules::original_static_word_unicode_value(
            request.original_selector(),
        )?;
        let classes = self.logical_self_relation_classes(analysis, &request, receiver)?;
        let mut targets = Vec::new();
        for document in &self.docs {
            let (Some(context), Some(input)) =
                (&document.diagnostic_source, &document.original_source_input)
            else {
                continue;
            };
            if input != analysis.resolved_input.as_ref()? {
                continue;
            }
            for class in document
                .classes
                .iter()
                .filter_map(|class| class.logical_declaration.as_ref())
            {
                if !class.matches_source(context.image(), input)
                    || !classes
                        .iter()
                        .any(|selected| std::ptr::eq(selected.metadata(), class.metadata()))
                {
                    continue;
                }
                let members = match request.caller().side() {
                    tcl_compiler::analyser::types::MemberSide::Instance => {
                        &class.metadata().methods
                    }
                    tcl_compiler::analyser::types::MemberSide::ClassObject => {
                        &class.metadata().class_methods
                    }
                };
                for method in members.values() {
                    let Some(member) = class.member(request.caller().side(), method) else {
                        continue;
                    };
                    if member
                        .original_name_word()
                        .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)
                        .as_deref()
                        != Some(name.as_slice())
                    {
                        continue;
                    }
                    if let Some(location) = crate::original_indexed_location::LogicalIndexedMemberLocation::capture_self_source(
                        &document.uri, context, input, member, &request,
                    ) { targets.push(location); }
                }
            }
        }
        Some(WorkspaceLogicalSelfMemberTargets { caller, targets })
    }

    /// Literal selector source advice for an independently indexed class header.
    /// Only direct public class-object methods are represented; these cards are
    /// not a method dispatch chain, instance receiver or negative member census.
    #[must_use]
    pub fn logical_class_member_targets_at(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        cursor: u32,
    ) -> Option<WorkspaceLogicalClassMemberTargets> {
        let input = analysis.resolved_input.as_ref()?;
        input.has_logical_source_name_context().then_some(())?;
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        (context.image().bytes() == source.as_bytes()).then_some(())?;
        let candidates =
            tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates(
                source, analysis,
            )?;
        let mut selectors = candidates
            .iter()
            .filter_map(|call| call.external_class_member_selector_at(cursor));
        let selector = selectors.next()?;
        selectors.next().is_none().then_some(())?;
        let document = self.docs.get(*self.slots.get(uri)?)?;
        let mut calls = document.logical_command_calls.iter().filter(|call| {
            call.input == *input
                && call.source == context
                && selector.matches_call(call.candidates())
        });
        let call = calls.next()?;
        calls.next().is_none().then_some(())?;
        self.logical_class_member_targets(call, &selector)
    }

    fn logical_class_member_targets(
        &self,
        call: &WorkspaceLogicalCommandCall,
        selector: &tcl_compiler::command_binding::LogicalExternalClassMemberSelector,
    ) -> Option<WorkspaceLogicalClassMemberTargets> {
        (call.is_retained_in(self) && selector.matches_call(call.candidates())).then_some(())?;
        let coordinates = call.candidates().external_header_coordinates()?;
        let mut represented_class = false;
        let mut targets = Vec::new();
        for document in &self.docs {
            let (Some(context), Some(input)) =
                (&document.diagnostic_source, &document.original_source_input)
            else {
                continue;
            };
            if input != &call.input {
                continue;
            }
            for declaration in document
                .classes
                .iter()
                .filter_map(|class| class.logical_declaration.as_ref())
            {
                if !declaration.matches_source(context.image(), input)
                    || !declaration
                        .source_candidate_coordinate()
                        .is_some_and(|coordinate| coordinates.contains(coordinate))
                {
                    continue;
                }
                represented_class = true;
                for method in declaration.metadata().class_methods.values() {
                    let Some(member) = declaration.member(
                        tcl_compiler::analyser::types::MemberSide::ClassObject,
                        method,
                    ) else {
                        continue;
                    };
                    if member
                        .original_name_word()
                        .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)
                        .as_deref()
                        != Some(selector.name())
                    {
                        continue;
                    }
                    if let Some(location) =
                        crate::original_indexed_location::LogicalIndexedMemberLocation::capture(
                            &document.uri,
                            context,
                            input,
                            member,
                        )
                    {
                        targets.push(location);
                    }
                }
            }
        }
        represented_class.then(|| WorkspaceLogicalClassMemberTargets {
            call: call.clone(),
            targets,
        })
    }

    /// Actual immutable declaring class/member allocation, independent of labels.
    #[must_use]
    pub fn logical_member_location_is_retained(
        &self,
        location: &crate::original_indexed_location::LogicalIndexedMemberLocation,
    ) -> bool {
        self.slots
            .get(location.uri())
            .and_then(|slot| self.docs.get(*slot))
            .is_some_and(|document| {
                document
                    .classes
                    .iter()
                    .filter_map(|class| class.logical_declaration.as_ref())
                    .any(|class| location.retains_class(class))
            })
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

    /// Original Logical headers from current indexed source and input owners.
    /// Suggestions provide insertion data without loaded or callable commands.
    pub fn logical_procedure_source_candidates(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration,
        ),
    > {
        self.docs.iter().flat_map(|document| {
            document
                .logical_procedure_declarations
                .iter()
                .filter_map(move |declaration| {
                    let context = document.diagnostic_source.as_ref()?;
                    let input = document.original_source_input.as_ref()?;
                    declaration
                        .matches_source(context.image(), input)
                        .then_some((document.uri.as_str(), declaration))
                })
        })
    }

    /// Current Logical procedure source cards for one exact original
    /// math-function occurrence. The occurrence must reborrow from the caller's
    /// actual `AnalysisResult`; candidate labels and report names cannot create
    /// this query. Positive cards grant no loaded function or dispatch result.
    #[must_use]
    pub fn logical_math_source_candidates(
        &self,
        occurrence: &tcl_compiler::command_binding::OriginalLogicalMathFunctionOccurrence,
        caller_analysis: &tcl_compiler::analyser::AnalysisResult,
        exclude_uri: &str,
    ) -> Option<
        Vec<(
            String,
            tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration,
        )>,
    > {
        Some(
            self.logical_math_source_cards(occurrence, caller_analysis, exclude_uri)?
                .into_iter()
                .filter_map(|candidate| {
                    Some((
                        candidate.uri.clone(),
                        candidate.procedure_declaration()?.clone(),
                    ))
                })
                .collect(),
        )
    }

    /// Retain the same actual selected declaration owner for diagnostic provider dependencies.
    #[must_use]
    pub fn logical_math_source_cards(
        &self,
        occurrence: &tcl_compiler::command_binding::OriginalLogicalMathFunctionOccurrence,
        caller_analysis: &tcl_compiler::analyser::AnalysisResult,
        exclude_uri: &str,
    ) -> Option<Vec<LogicalWorkspaceCommandCandidate>> {
        let image = occurrence.original_word().image();
        let source = image.try_text().ok()?;
        if caller_analysis
            .logical_math_function_at(source, occurrence.span().start())
            .as_ref()
            != Some(occurrence)
        {
            return None;
        }
        let coordinates = occurrence.external_header_coordinates()?;
        for coordinate in coordinates {
            let candidates = self
                .logical_procedure_source_candidates()
                .filter(|(uri, declaration)| {
                    *uri != exclude_uri
                        && self
                            .slots
                            .get(*uri)
                            .and_then(|slot| self.docs.get(*slot))
                            .is_some_and(|document| {
                                document.original_source_input.as_ref()
                                    == caller_analysis.resolved_input.as_ref()
                            })
                        && declaration.source_candidate_coordinate().as_ref() == Some(&coordinate)
                })
                .filter_map(|(uri, declaration)| self.logical_procedure_candidate(uri, declaration))
                .collect::<Vec<_>>();
            if !candidates.is_empty() {
                return Some(candidates);
            }
        }
        Some(Vec::new())
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
                        .filter(|m| {
                            m.method.kind != MethodKind::ClassMethod.as_str() && !m.method.private
                        })
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

    fn logical_procedure_candidate(
        &self,
        uri: &str,
        declaration: &tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration,
    ) -> Option<LogicalWorkspaceCommandCandidate> {
        let document = self.docs.get(*self.slots.get(uri)?)?;
        let context = document.diagnostic_source.as_ref()?;
        let input = document.original_source_input.as_ref()?;
        let location =
            crate::original_indexed_location::LogicalIndexedDeclarationLocation::procedure(
                uri,
                context,
                input,
                declaration,
            )?;
        let metadata = declaration.metadata();
        Some(LogicalWorkspaceCommandCandidate {
            revision: self.document_revision(uri)?,
            image: context.image().clone(),
            uri: uri.to_owned(),
            qualified_name: metadata.qualified_name.clone(),
            name_span: metadata.name_span,
            detail: Some(format!("({} params)", metadata.params.len())),
            procedure: Some(declaration.clone()),
            location,
        })
    }

    /// Conditional Logical declarations at the first matching original source coordinate.
    /// Every duplicate remains a candidate; source cards grant no loaded command,
    /// reached implementation or writable reference. Native query owners are separate.
    #[must_use]
    pub fn logical_command_candidates(
        &self,
        request: &tcl_compiler::command_binding::OriginalLogicalCommandCandidates,
        exclude_uri: &str,
    ) -> Vec<LogicalWorkspaceCommandCandidate> {
        let Some(coordinates) = request.external_header_coordinates() else {
            return Vec::new();
        };
        for coordinate in coordinates {
            let mut candidates = Vec::new();
            for (uri, declaration) in self.logical_procedure_source_candidates() {
                if uri != exclude_uri
                    && self.logical_request_matches_document_input(request, uri)
                    && declaration.source_candidate_coordinate().as_ref() == Some(coordinate)
                    && let Some(candidate) = self.logical_procedure_candidate(uri, declaration)
                {
                    candidates.push(candidate);
                }
            }
            for class in self
                .classes()
                .filter(|class| class.uri != exclude_uri && !class.via_define)
            {
                if self.logical_request_matches_document_input(request, &class.uri)
                    && class
                        .logical_declaration
                        .as_ref()
                        .and_then(|declaration| declaration.source_candidate_coordinate())
                        == Some(coordinate)
                {
                    let Some(context) = self.diagnostic_source_context(&class.uri) else {
                        continue;
                    };
                    candidates.push(LogicalWorkspaceCommandCandidate {
                        location: match class.logical_declaration.as_ref().and_then(|declaration| {
                            let document = self.docs.get(*self.slots.get(&class.uri)?)?;
                            crate::original_indexed_location::LogicalIndexedDeclarationLocation::class(
                                &class.uri, context, document.original_source_input.as_ref()?, declaration,
                            )
                        }) { Some(location) => location, None => continue },
                        revision: self
                            .document_revision(&class.uri)
                            .expect("indexed declaration revision"),
                        image: context.image().clone(),
                        uri: class.uri.clone(),
                        qualified_name: class.qualified_name.clone(),
                        name_span: class.name_span,
                        detail: None,
                        procedure: None,
                    });
                }
            }
            if !candidates.is_empty() {
                return candidates;
            }
        }
        Vec::new()
    }

    fn logical_request_matches_document_input(
        &self,
        request: &tcl_compiler::command_binding::OriginalLogicalCommandCandidates,
        uri: &str,
    ) -> bool {
        let Some(input) = self
            .slots
            .get(uri)
            .and_then(|slot| self.docs.get(*slot))
            .and_then(|document| document.original_source_input.as_ref())
        else {
            return false;
        };
        request
            .original_head()
            .is_some_and(|head| request.matches_source(head.image(), input))
    }

    /// The independently selected declaring revision still owns this candidate.
    #[must_use]
    pub fn logical_command_candidate_is_current(
        &self,
        candidate: &LogicalWorkspaceCommandCandidate,
    ) -> bool {
        self.logical_declaration_location_is_retained(&candidate.location)
            && self.document_revision(&candidate.uri) == Some(candidate.revision)
            && self
                .diagnostic_source_context(&candidate.uri)
                .is_some_and(|context| context.image() == &candidate.image)
    }

    /// Proc definitions matching `name` (simple, qualified, or
    /// `::`-prefixed simple form), excluding `exclude_uri`.
    /// Used by cross-document go-to-definition: when the
    /// current document has no matching proc, the index
    /// resolves one defined elsewhere.
    #[must_use]
    pub fn proc_definitions<'a>(&'a self, name: &str, exclude_uri: &str) -> Vec<&'a WorkspaceProc> {
        let qualified = root_unrooted_key(name);
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
        let qualified = root_unrooted_key(name);
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

    /// Original declaration candidates whose checked outward C key matches
    /// this compatibility query, excluding `exclude_uri`. Source coordinates
    /// supply no runtime namespace incarnation or complete target census.
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

    /// Documents with an original conditional alias target matching one checked
    /// C compatibility coordinate. This selects source candidates only, never
    /// an entered link or a complete runtime target inventory.
    #[must_use]
    pub fn documents_aliasing_variable(&self, qualified_name: &str) -> Vec<String> {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        let mut uris: Vec<String> = self
            .variable_aliases()
            .filter(|alias| {
                alias
                    .source
                    .target_symbol()
                    .checked_c_namespace_key()
                    .is_some_and(|key| unroot_rooted_key(&key).unwrap_or(&key) == target)
            })
            .map(|alias| alias.uri.clone())
            .collect();
        uris.sort();
        uris.dedup();
        uris
    }

    /// Documents without complete retained alias-source coverage remain a
    /// barrier for every target. Missing typed receipts cannot establish that
    /// a computed or unavailable alias does not affect a proposed rename.
    #[must_use]
    pub fn documents_with_ambiguous_alias_of(&self, _qualified_name: &str) -> Vec<String> {
        let mut uris: Vec<String> = self
            .slots
            .values()
            .filter_map(|slot| self.docs.get(*slot))
            .filter(|document| !document.alias_source_coverage_complete)
            .map(|document| document.uri.clone())
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

    /// The qualified names of the workspace declarations the call written at
    /// `range` in `uri` settles to in the link-following view. A listing aid
    /// for find-references on a release profile; rename does not read it.
    #[must_use]
    pub fn linked_declaration_names_of_call(&self, uri: &str, range: Span) -> Vec<String> {
        let Some(document) = self.slots.get(uri).and_then(|slot| self.docs.get(*slot)) else {
            return Vec::new();
        };
        let Some(invocation) = document.invocations.iter().find(|inv| inv.range == range) else {
            return Vec::new();
        };
        let defined = self.defined_command_names(true);
        let links = self.command_link_map();
        let wci = WildcardImportIndex::build(self);
        match self.settle_invocation(invocation, &defined, Some(&links), &wci) {
            Some(SettledCommandTarget::Authored(name)) => {
                vec![tcl_syntax::naming::root_unrooted_key(&name)]
            }
            Some(SettledCommandTarget::OriginalSlot(slot, policy)) => {
                let names = self
                    .original_procedure_declarations()
                    .filter(|(_, declaration)| {
                        declaration.name().slot() == &slot && declaration.name().policy() == policy
                    })
                    .map(|(_, declaration)| declaration.metadata().qualified_name.clone())
                    .chain(
                        self.original_class_declarations()
                            .filter(|(_, declaration)| {
                                declaration.name().slot() == &slot
                                    && declaration.name().policy() == policy
                            })
                            .map(|(_, declaration)| declaration.metadata().qualified_name.clone()),
                    )
                    .collect::<Vec<_>>();
                // Two documents publishing the slot leave the call's owner
                // unknown; the listing abstains rather than pick one.
                if names.len() == 1 { names } else { Vec::new() }
            }
            None => Vec::new(),
        }
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

    /// A release-profile call the document's own analysis did not bind
    /// settles to the workspace definition its lookup selects: the current
    /// namespace, then the global one, or the exact qualified name. The
    /// link-following view lists references and answers whenever that
    /// selection is unanimous. The direct view feeds rename, so it answers
    /// only for a single definition nothing visible can rebind.
    fn workspace_assistance_target(
        &self,
        lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
        follow_links: bool,
    ) -> Option<SettledCommandTarget> {
        let target = self.original_assistance_target(lookup, follow_links)?;
        (follow_links || self.original_target_is_unrebound(&target)).then_some(target)
    }

    /// Whether exactly one workspace declaration publishes this slot and no
    /// `rename`, `interp alias` or `namespace import` in any document names it
    /// as its link or its target.
    fn original_target_is_unrebound(&self, target: &SettledCommandTarget) -> bool {
        let SettledCommandTarget::OriginalSlot(slot, policy) = target else {
            return false;
        };
        let mut publications = 0usize;
        for document in &self.docs {
            if let Some(world) = &document.original_command_world {
                publications += world
                    .declarations()
                    .filter(|publication| {
                        publication.slot() == slot && publication.policy() == *policy
                    })
                    .count();
            } else {
                publications += document
                    .original_procedure_source_candidates()
                    .into_iter()
                    .filter(|(_, candidate, candidate_policy)| {
                        *candidate == slot && candidate_policy == policy
                    })
                    .count();
                publications += document
                    .original_class_source_candidates()
                    .into_iter()
                    .filter(|(_, candidate, candidate_policy, _)| {
                        *candidate == slot && candidate_policy == policy
                    })
                    .count();
            }
        }
        if publications != 1 {
            return false;
        }
        let Ok(name) = String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(slot))
        else {
            return false;
        };
        let name = unroot_rooted_key(&name).unwrap_or(&name);
        !self.command_links().any(|link| {
            unroot_rooted_key(&link.linked_qname).unwrap_or(&link.linked_qname) == name
                || unroot_rooted_key(&link.target_qname).unwrap_or(&link.target_qname) == name
        })
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
        if let Some(lookup) = inv.original_lookup.as_ref()
            && lookup.policy().authority() == tcl_syntax::naming::NamePolicyAuthority::Native
        {
            return self.original_assistance_target(lookup, links.is_some());
        }
        if let Some(lookup) = inv.original_lookup.as_ref()
            && inv.logical_source_domain == Some(false)
        {
            return self.workspace_assistance_target(lookup, links.is_some());
        }
        if inv.original_name_input.is_some() || inv.logical_source_domain != Some(false) {
            return None;
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

    /// Project readonly conditional namespace-import source advice for a
    /// bare call. The legacy qualified-name string is presentation only: a
    /// positive establishes no live alias or Native command identity, and
    /// None establishes no complete candidate inventory or namespace absence.
    /// Unknown entry/completion/observer/lifecycle alternatives remain retained
    /// in [`Self::conditional_namespace_import_advice`]. Edit/binding consumers
    /// must use the independently guarded [`Self::resolve_wildcard_import_for_binding`].
    ///
    /// `resolution_candidates` preserves the caller's authored order; `call`
    /// supplies an actual written point or the explicit legacy report horizon.
    /// Qualified words decline, matching the existing bareword query purpose.
    #[must_use]
    pub fn resolve_wildcard_import(
        &self,
        word: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
    ) -> Option<String> {
        self.conditional_namespace_import_advice(word, resolution_candidates, call)?
            .compatibility_report()
            .map(str::to_owned)
    }

    /// Existing binding/edit interpretation, independently retaining modeled
    /// exclusion guards. Conditional report advice cannot enter this path.
    #[must_use]
    pub fn resolve_wildcard_import_for_binding(
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

    /// Source-only namespace relationship under its retained statement horizon.
    /// No candidate is a live-slot absence or a completed namespace operation.
    #[must_use]
    pub fn conditional_namespace_import_advice(
        &self,
        word: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
    ) -> Option<ConditionalNamespaceImportAdvice> {
        let documents = self
            .docs
            .iter()
            .map(|document| {
                Some(NamespaceAdviceDocument {
                    uri: document.uri.clone(),
                    context: document.source_order_context.clone()?,
                    revision: document.revision,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let wci = WildcardImportIndex::build_for_purpose(
            self,
            crate::namespace_import::NamespaceStatementPurpose::SourceHorizon,
        );
        let selected =
            self.resolve_wildcard_import_target_indexed(word, resolution_candidates, call, &wci);
        let subjects = selected
            .as_ref()
            .and_then(|target| target.pattern)
            .map_or_default(|pattern| {
                wci.original_subjects
                    .iter()
                    .copied()
                    .filter(|command| {
                        pattern.matched_imported_command_tail(command) == Some(word.as_bytes())
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            });
        let target = subjects.first().and_then(|first| {
            subjects
                .iter()
                .all(|subject| subject.slot() == first.slot() && subject.policy() == first.policy())
                .then(|| first.source_spelling())
                .flatten()
        });
        Some(ConditionalNamespaceImportAdvice {
            target,
            subjects,
            documents,
            order: self.run_order(),
            generation: self.generation(),
            patterns: self
                .docs
                .iter()
                .flat_map(|document| document.original_namespace_patterns.iter().cloned())
                .collect(),
            exports: self
                .namespace_exports()
                .filter_map(|event| event.original.clone())
                .collect(),
        })
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
        self.resolve_wildcard_import_target_indexed(word, resolution_candidates, call, wci)
            .map(|target| target.report)
    }

    fn resolve_wildcard_import_target_indexed<'i>(
        &self,
        word: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
        wci: &WildcardImportIndex<'i>,
    ) -> Option<ImportChainTarget<'i>> {
        if tcl_syntax::naming::is_qualified(word.as_bytes()) {
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
    fn follow_import_chain<'i>(
        &self,
        ns: &str,
        word: &str,
        call: CallSite<'_>,
        wci: &WildcardImportIndex<'i>,
    ) -> Option<ImportChainTarget<'i>> {
        let mut current = ns.to_owned();
        for _ in 0..tcl_compiler::analyser::indirection::MAX_COMMAND_NAME_HOPS {
            let row = self.import_hop(&current, word, call, wci)?;
            let import: &'i WorkspaceGlobImport = row.imp;
            let hop = &import.source_ns;
            let target = tcl_syntax::naming::qualify(hop, word);
            if self.workspace_command_exists(&target) {
                return Some(ImportChainTarget {
                    report: target,
                    pattern: import.original_pattern.as_ref(),
                });
            }
            current = hop.clone();
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
    fn import_hop<'w, 'i>(
        &self,
        ns: &str,
        word: &str,
        call: CallSite<'_>,
        wci: &'w WildcardImportIndex<'i>,
    ) -> Option<&'w GlobImportRow<'i>> {
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
                    || !((wci.purpose
                        == crate::namespace_import::NamespaceStatementPurpose::Modeled
                        && (self.defines_command(&target) || row.declares_builtin_at(&target)))
                        || wci.conflicting_alias_at(ns, &row.imp.source_ns, word, row.site()))
            })
            .filter(|row| wci.alias_live_at(ns, &row.imp.source_ns, word, row.site(), call))
            .enumerate()
            .max_by_key(|(seq, row)| (row.imp.uri == call.uri, row.imp.at, *seq))
            .map(|(_, row)| row)
    }
}

struct ImportChainTarget<'a> {
    report: String,
    pattern: Option<&'a tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
}

/// One actual document input consumed by conditional namespace source advice.
#[derive(Debug, Clone)]
pub struct NamespaceAdviceDocument {
    uri: String,
    context: crate::source_graph::SourceOrderContext,
    revision: u64,
}
impl NamespaceAdviceDocument {
    /// The independently retained indexed document URI.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }
    /// Full original source text; unavailable text never becomes an empty image.
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.context.source_text()
    }
    /// Match the exact retained full analysis input and its original Realm.
    #[must_use]
    pub fn matches_input(&self, input: &tcl_compiler::analyser::ResolvedAnalysisInput) -> bool {
        self.context.matches_input(input)
    }
}

/// Remaining obligations of a conditional namespace relationship report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalNamespaceDebt {
    /// Source geometry does not establish entry or successful completion.
    EntryAndCompletion,
    /// Namespace observers and effects have not been discharged.
    NamespaceObservers,
    /// Unmodeled alternative command/namespace outcomes remain possible.
    UnknownAlternatives,
    /// Raw lifecycle/body-replacement rows supply no operation receipt.
    UnrepresentedLifecycle,
}

/// A readonly conditional relationship, not a command identity or edit proof.
/// Unknown entry/completion/observer and unmodeled alternatives always remain.
#[derive(Debug, Clone)]
pub struct ConditionalNamespaceImportAdvice {
    target: Option<String>,
    subjects: Vec<tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
    documents: Vec<NamespaceAdviceDocument>,
    order: Arc<crate::source_graph::RunOrder>,
    generation: u64,
    patterns: Vec<tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
    exports: Vec<tcl_compiler::signature_scan::original_name::SourceNamespaceExport>,
}
impl ConditionalNamespaceImportAdvice {
    /// Explicit legacy presentation only. None is not live namespace absence.
    #[must_use]
    pub fn compatibility_report(&self) -> Option<&str> {
        self.target.as_deref()
    }
    /// Actual source contexts consumed by this conditional relationship.
    #[must_use]
    pub fn documents(&self) -> &[NamespaceAdviceDocument] {
        &self.documents
    }
    /// Always conditional, including an empty modeled candidate set.
    #[must_use]
    pub const fn has_unresolved_operation_debt(&self) -> bool {
        true
    }
    /// These debts survive both positive and empty compatibility projections.
    #[must_use]
    pub const fn debts(&self) -> &[ConditionalNamespaceDebt] {
        &[
            ConditionalNamespaceDebt::EntryAndCompletion,
            ConditionalNamespaceDebt::NamespaceObservers,
            ConditionalNamespaceDebt::UnknownAlternatives,
            ConditionalNamespaceDebt::UnrepresentedLifecycle,
        ]
    }

    /// Authenticate the original subjects, contexts, revisions and order allocation.
    #[must_use]
    pub fn matches_index(&self, index: &WorkspaceIndex) -> bool {
        self.subjects.iter().all(|subject| {
            index.docs.iter().any(|document| {
                document
                    .original_procs
                    .iter()
                    .any(|declaration| declaration.name() == subject)
                    || document
                        .original_classes
                        .iter()
                        .any(|declaration| declaration.name() == subject)
            })
        }) && self.generation == index.generation()
            && Arc::ptr_eq(&self.order, &index.run_order())
            && self.documents.iter().all(|document| {
                index
                    .slots
                    .get(&document.uri)
                    .and_then(|slot| index.docs.get(*slot))
                    .is_some_and(|current| {
                        current.revision == document.revision
                            && current.source_order_context.as_ref() == Some(&document.context)
                    })
            })
            && self.patterns.iter().all(|pattern| {
                index
                    .docs
                    .iter()
                    .any(|document| document.original_namespace_patterns.contains(pattern))
            })
            && self.exports.iter().all(|export| {
                index
                    .namespace_exports()
                    .any(|event| event.original.as_ref() == Some(export))
            })
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

#[derive(Clone, Copy)]
struct ExportQuery<'p, 's, 'n> {
    pattern: Option<&'p tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
    subjects: &'s [&'n tcl_compiler::signature_scan::scope::SignatureSourceCommand],
    source_known: bool,
}

impl<'a> ExportGate<'a> {
    /// The gate for `source_ns` as seen from the import at `site`, or an empty
    /// gate when the namespace has no export rows at all.
    fn decide(
        exports_by_ns: &std::collections::HashMap<&'a str, Vec<&'a WorkspaceNamespaceExport>>,
        order: &crate::source_graph::RunOrder,
        source_ns: &str,
        site: ImportSite<'_>,
        query: ExportQuery<'_, '_, 'a>,
        purpose: crate::namespace_import::NamespaceStatementPurpose,
    ) -> Self {
        let ExportQuery {
            pattern: original,
            subjects,
            source_known: native_source_known,
        } = query;
        if let Some(original) = original {
            if tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(
                original.name_input().policy(),
            )
            .export_filter_applies()
                && exports_by_ns.values().flatten().any(|event| {
                    event.unknown
                        && purpose
                            .precedence(
                                order,
                                RunPoint {
                                    uri: &event.uri,
                                    at: event.at,
                                    enclosing_body: event.enclosing_body,
                                },
                                site.point(),
                            )
                            .unwrap_or(true)
                })
            {
                return Self::default();
            }
            let literal = subjects
                .iter()
                .copied()
                .filter(|name| {
                    original.matches_imported_command(name.slot(), name.policy()) == Some(true)
                        && crate::namespace_import::exported_original_for_purpose(
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
                            purpose,
                        ) == ExportVerdict::Exported
                })
                .filter_map(|name| {
                    std::str::from_utf8(original.matched_imported_command_tail(name)?).ok()
                })
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
        let surviving = crate::namespace_import::export_patterns_for_purpose(
            &mut events,
            order,
            site.point(),
            purpose,
        );
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
    /// Declared-command metadata from this import document's retained store.
    /// Original import/export/source capabilities remain independently guarded.
    /// `None` when the actual source input cannot supply that store.
    registry: Option<&'a tcl_registry::CommandRegistry>,
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
            .map_or_default(Vec::as_slice)
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
            .is_some_and(|r| r.declares_command_at_rooted_key(qualified_name))
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
    source_patterns: Vec<(
        &'a str,
        &'a tcl_compiler::signature_scan::original_name::SourceNamespacePattern,
    )>,
    purpose: crate::namespace_import::NamespaceStatementPurpose,
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

type CommandDeletionsByName<'a> =
    std::collections::HashMap<&'a str, Vec<&'a WorkspaceCommandDeletion>>;
type DeclarationSitesByName<'a> = std::collections::HashMap<&'a str, Vec<(&'a str, u32)>>;

impl<'a> WildcardImportIndex<'a> {
    fn build(index: &'a WorkspaceIndex) -> Self {
        Self::build_for_purpose(
            index,
            crate::namespace_import::NamespaceStatementPurpose::Modeled,
        )
    }

    fn build_for_purpose(
        index: &'a WorkspaceIndex,
        purpose: crate::namespace_import::NamespaceStatementPurpose,
    ) -> Self {
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
                    .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name)
                    .chain(doc.original_classes.iter().map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name))
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
                        ExportQuery {
                            pattern: imp.original_pattern.as_ref(),
                            subjects: &original_subjects,
                            source_known: imp.native_source.is_some(),
                        },
                        purpose,
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
                            ExportQuery {
                                pattern: gate.original_pattern.as_ref(),
                                subjects: &original_subjects,
                                source_known: gate.native_source.is_some(),
                            },
                            purpose,
                        ),
                    });
            }
        }
        let mut forgets_by_ns: std::collections::HashMap<&str, Vec<&WorkspaceNamespaceForget>> =
            std::collections::HashMap::new();
        for fgt in index.namespace_forgets() {
            forgets_by_ns.entry(fgt.ns.as_str()).or_default().push(fgt);
        }
        let deletions_by_name = Self::deletions_by_name(index);
        let declarations_by_qname = Self::declaration_sites_by_name(index);
        let imports_by_ns = rows_by_ns
            .into_iter()
            .map(|(ns, rows)| (ns, NamespaceImports::index(rows)))
            .collect();
        Self {
            source_patterns: index
                .docs
                .iter()
                .flat_map(|document| {
                    document
                        .original_namespace_patterns
                        .iter()
                        .map(|pattern| (document.uri.as_str(), pattern))
                })
                .collect(),
            purpose,
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

    fn deletions_by_name(index: &'a WorkspaceIndex) -> CommandDeletionsByName<'a> {
        let mut deletions_by_name: std::collections::HashMap<&str, Vec<&WorkspaceCommandDeletion>> =
            std::collections::HashMap::new();
        for del in index.command_deletions() {
            deletions_by_name
                .entry(del.qualified_name.as_str())
                .or_default()
                .push(del);
        }
        deletions_by_name
    }

    fn declaration_sites_by_name(index: &'a WorkspaceIndex) -> DeclarationSitesByName<'a> {
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
        declarations_by_qname
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
        if tcl_syntax::naming::is_qualified(name.as_bytes()) {
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
        install_at: ImportSite<'e>,
        query: Option<RunPoint<'e>>,
    ) -> impl Iterator<Item = crate::namespace_import::AliasEvent<'e>> + 'e {
        use crate::namespace_import::{AliasEvent, AliasEventKind};
        let forgets = self
            .forgets_by_ns
            .get(importing_ns)
            .map_or_default(Vec::as_slice)
            .iter()
            .filter(move |f| {
                if self.purpose == crate::namespace_import::NamespaceStatementPurpose::SourceHorizon
                {
                    return self.source_patterns.iter().any(|(uri, forget)| {
                        *uri == f.uri
                            && forget.span().start() == f.at
                            && forget.purpose()
                                == tcl_syntax::naming::NativeNamePurpose::NamespaceForgetPattern
                            && self.source_patterns.iter().any(|(uri, import)| {
                                *uri == install_at.uri
                                    && import.span().start() == install_at.at
                                    && self.original_subjects.iter().any(|command| {
                                        command.slot().simple.try_utf8().ok() == Some(name)
                                            && forget.forgets_source_import(import, command)
                                                == Some(true)
                                    })
                            })
                    });
                }
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
            .map_or_default(Vec::as_slice)
            .iter()
            .filter(move |_| {
                self.purpose == crate::namespace_import::NamespaceStatementPurpose::Modeled
            })
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
            .map_or_default(Vec::as_slice)
            .iter()
            .map(|d| RunPoint {
                uri: d.uri.as_str(),
                at: d.at,
                enclosing_body: None,
            })
            .filter(move |&at| match self.purpose {
                crate::namespace_import::NamespaceStatementPurpose::Modeled => true,
                // Source advice drops a destruction unless the call's own
                // document runs it, in straight-line order, before the call.
                crate::namespace_import::NamespaceStatementPurpose::SourceHorizon => query
                    .is_some_and(|query| {
                        query.uri == at.uri
                            && self.order.source_advice_statement_precedence(at, query)
                                == Some(true)
                    }),
            })
            .map(|at| AliasEvent {
                kind: AliasEventKind::Remove,
                at,
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
        let mut events = install.chain(self.removal_events(
            importing_ns,
            source_ns,
            name,
            install_at,
            Some(call.point()),
        ));
        crate::namespace_import::alias_for_purpose(
            &mut events,
            &self.order,
            Some(call.point()),
            self.purpose,
        )
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
            .map_or_default(|imports| imports.admitting(name).collect::<Vec<_>>())
            .into_iter()
            .map(|other| (other.imp.source_ns.as_str(), other.site(), &other.exported))
            .chain(
                self.exact_by_ns
                    .get(ns)
                    .map_or_default(Vec::as_slice)
                    .iter()
                    .filter(|other| other.gate.name == name)
                    .map(|other| (other.gate.source_ns.as_str(), other.site(), &other.exported)),
            );
        earlier.any(|(other_source, other_site, other_exported)| {
            self.purpose
                .comparison(&self.order, other_site.point(), site.point())
                == Some(std::cmp::Ordering::Less)
                && !ns_eq(other_source, source_ns)
                && other_exported.covers(name)
                && self.alias_certainly_live_at(ns, other_source, name, other_site, call)
        })
    }

    /// [`Self::alias_live_at`] for the conflict premise: the earlier alias
    /// must be certainly live at the later import, so a removal whose position
    /// is unknown (another document, a deferred body, a conditional branch)
    /// leaves it possibly dead and the later install stands
    /// ([`crate::namespace_import::alias_certainly_live`]).
    fn alias_certainly_live_at(
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
        let mut events = install.chain(self.removal_events(
            importing_ns,
            source_ns,
            name,
            install_at,
            Some(call.point()),
        ));
        crate::namespace_import::alias_certainly_live(
            &mut events,
            &self.order,
            call.point(),
            self.purpose,
        )
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
    /// which has no query point of its own, so it reads the state after the
    /// document completes normally, as tclsh leaves it after `source`.
    ///
    /// - On workspace navigation a forget or redefinition applies only when
    ///   its document's end is admitted and it is a sequence position before
    ///   that end; it then revokes the link when written *after* the import
    ///   in the same document. Where the end is withheld (a possible early
    ///   exit, an unretained script, a function call, unresolved words), or
    ///   the removal sits in a deferred body or a branch, the order stays
    ///   conditional and the link stays live.
    /// - An explicitly constructed order treats every recorded removal as
    ///   having run.
    /// - Either way the removal is ordered against the **import**: one
    ///   written before it is undone by the import itself (a re-import after
    ///   a forget reinstalls — oracle), and one in another document revokes
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
        let mut events = install.chain(self.removal_events(
            importing_ns,
            &gate.source_ns,
            &gate.name,
            gate.site(uri),
            None,
        ));
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
    /// (`definition::exported_original_slot_at_import_in`) calls, so the two tiers cannot
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
                        .navigation_precedence(
                            RunPoint {
                                uri: &event.uri,
                                at: event.at,
                                enclosing_body: event.enclosing_body,
                            },
                            site.point(),
                        )
                        .and_then(
                            crate::source_graph::SourceNavigationOrder::modeled_source_exclusion,
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
        if tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy)
            .export_filter_applies()
            && self.original_exports.iter().any(|event| {
                event.unknown
                    && self
                        .order
                        .navigation_precedence(
                            RunPoint {
                                uri: &event.uri,
                                at: event.at,
                                enclosing_body: event.enclosing_body,
                            },
                            import_site,
                        )
                        .and_then(
                            crate::source_graph::SourceNavigationOrder::modeled_source_exclusion,
                        )
                        .unwrap_or(true)
            })
        {
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
mod tests;

#[cfg(test)]
mod original_settlement_transport_tests;

#[cfg(test)]
mod original_namespace_export_index_tests;

#[cfg(test)]
mod retained_store_consumer_tests;

#[cfg(test)]
mod logical_navigation_candidate_tests;

#[cfg(test)]
mod logical_class_navigation_candidate_tests;

#[cfg(test)]
mod shared_import_pattern_classification_tests;

#[cfg(test)]
mod selected_import_export_policy_tests;

#[cfg(test)]
mod typed_variable_source_index_tests;

#[cfg(test)]
mod original_source_order_transport_tests;

#[cfg(test)]
mod retained_logical_call_membership_tests;

#[cfg(test)]
mod logical_indexed_declaration_membership_tests;

#[cfg(test)]
mod logical_indexed_variable_location_tests;

#[cfg(test)]
mod logical_variable_input_correspondence_tests;

#[cfg(test)]
mod logical_indexed_namespace_locations_tests;

#[cfg(test)]
mod logical_namespace_request_symbol_tests;

#[cfg(test)]
mod logical_cross_document_class_member_tests;

#[cfg(test)]
mod logical_library_input_correspondence_tests;

#[cfg(test)]
mod logical_self_member_source_tests;

mod source_views;
pub use source_views::WorkspaceOriginalSourceView;
