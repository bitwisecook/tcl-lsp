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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[cfg(test)]
use crate::namespace_import::ExportVerdict;
#[cfg(test)]
use crate::source_graph::RunPoint;
use crate::workspace_symbols::IndexedWorkspaceSymbol;
#[cfg(test)]
use crate::workspace_symbols::WorkspaceSymbolKind;
use tcl_compiler::analyser::{AnalysisResult, MemberRetractionRecord, MemberSide};
use tcl_compiler::ir::MethodKind;
use tcl_lexer::Span;
#[cfg(test)]
use tcl_syntax::naming::unroot_rooted_key;

mod class_dispatch;
mod command_settlement;
mod declarations;
mod document_records;
mod load_order;
mod logical_navigation;
mod namespace_imports;
mod records;
mod settlement_dependencies;
mod symbol_occurrences;
mod symbol_search;

pub use class_dispatch::EffectiveMember;
use command_settlement::IncrementalSettledTargets;
#[cfg(test)]
use command_settlement::SettledCommandTarget;
pub use logical_navigation::{
    LogicalWorkspaceCommandCandidate, WorkspaceLogicalClassMemberTargets,
    WorkspaceLogicalSelfMemberTargets,
};
#[cfg(test)]
use namespace_imports::WildcardImportIndex;
pub use namespace_imports::{
    CallSite, ConditionalNamespaceDebt, ConditionalNamespaceImportAdvice, NamespaceAdviceDocument,
    NamespaceExportSnapshot,
};
use records::WorkspaceCommandRetirement;
pub use records::{
    MethodAccess, WorkspaceCommandDeletion, WorkspaceCommandLink, WorkspaceDefinedSymbol,
    WorkspaceDiagnosticSourceContext, WorkspaceGlobImport, WorkspaceImportGate, WorkspaceMethod,
    WorkspaceNamespaceExport, WorkspaceNamespaceForget, WorkspaceNamespaceRef,
    WorkspacePackageIfneeded, WorkspacePackagePrefer, WorkspacePackageProvide,
    WorkspacePackageRequire, WorkspaceProc, WorkspaceSource, WorkspaceVariable,
    WorkspaceVariableRef,
};

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

/// A conditional original alias declaration retained for document candidacy.
/// Its typed source target and unresolved applicability remain distinct from
/// an entered link, current cell or complete runtime alias inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceVariableAlias {
    uri: String,
    source: tcl_compiler::signature_scan::variable_symbol::OriginalVariableAliasTemplate,
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

    /// Metadata store retained with the indexed document's complete source
    /// context. Missing or mismatched input does not recapture a profile store.
    fn registry_for(&self, uri: &str) -> Option<&tcl_registry::CommandRegistry> {
        self.docs[*self.slots.get(uri)?].registry()
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

    /// Every indexed invocation site.
    pub fn invocations(&self) -> impl Iterator<Item = &WorkspaceInvocation> {
        self.docs.iter().flat_map(|doc| doc.invocations.iter())
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
