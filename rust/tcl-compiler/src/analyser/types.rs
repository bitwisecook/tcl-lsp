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

use std::collections::HashMap;
#[cfg(test)]
use std::collections::HashSet;

mod original_configurations;
mod original_constructor_call;
mod original_linked_member;
pub use original_linked_member::{
    OriginalSourceLinkedMember, original_source_linked_member_at,
    original_source_linked_member_reference_spans, original_source_linked_members_for_class,
};
mod logical_procedures;
pub use logical_procedures::LogicalSourceProcedureDeclaration;
mod logical_object_members;
pub use logical_object_members::{LogicalSourceObjectMemberState, LogicalSourceObjectMembers};
mod logical_members;
pub use logical_members::{
    LogicalSourceClassDeclaration, LogicalSourceMemberDeclaration, LogicalSourceMemberInsertion,
    LogicalSourceRelationStatement, LogicalSourceSelfSelector,
};
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
pub(crate) use original_members::OriginalSourceMethodNameBody;
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

use tcl_lexer::Span;

use crate::signature_scan::types::{
    SignatureCommandAlias, SignatureCommandInvocation, SignatureNamespaceExport,
    SignatureNamespaceForget, SignatureNamespaceImport, SignaturePackagePrefer,
    SignaturePackageRequire, SignatureSource,
};

pub use tcl_core_types::DiagCode;
/// Severity of a diagnostic — the shared [`tcl_core_types::Severity`] so the
/// analyser, compiler-checks, and LSP/CLI layers speak one type.
pub use tcl_core_types::Severity;

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
#[derive(Debug, Clone, Default)]
pub struct AnalysisResult {
    pub(super) optimiser_source_origin:
        Option<crate::compilation_unit::OriginalOptimiserSourceOrigin>,
    /// The requested command generation is unavailable. No source walk was
    /// performed; an empty diagnostic set therefore does not indicate success.
    pub analysis_context_unavailable: Option<tcl_registry::model::OverlayMiss>,
    pub(super) command_realm: Option<std::sync::Arc<crate::realm::CommandBindingRealm>>,
    pub(super) original_command_world:
        Option<crate::command_binding::OriginalCompletedCommandWorld>,
    pub(crate) original_source_region:
        Option<crate::command_binding::OriginalSourceConditionalRegion>,
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
    /// Members an `oo::define` adds to a class this document does not
    /// declare, keyed by the class's qualified name. Advisory and
    /// cross-document only: no class of this document, read by the
    /// workspace index when it merges them into the declaring document's
    /// class, never by a proof consumer.
    pub external_class_contributions: HashMap<String, ClassDef>,
    /// Genuine Logical headers retain insertion names separately from report labels.
    pub(super) logical_procedure_declarations: Vec<LogicalSourceProcedureDeclaration>,
    /// Completed Logical class walks retain source membership without Native identity.
    pub(super) logical_class_declarations: Vec<LogicalSourceClassDeclaration>,
    /// Authentic original class declarations, separate from UI `QName` maps.
    pub original_class_metadata:
        Vec<crate::signature_scan::original_name::SourceDeclarationMetadata<ClassDef>>,
    /// Counted class declarations without a UTF-8 reporting projection.
    pub original_counted_class_metadata:
        Vec<crate::signature_scan::original_name::SourceDeclarationMetadata<OriginalCountedClass>>,
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
    /// Whether dynamic source loading or command mutations can invalidate
    /// missing-package advice independently of unnameable package requirements.
    pub(crate) has_unresolved_provider_effects: bool,
    /// Source-target records.
    pub source_targets: Vec<SignatureSource>,
    /// Conditional source path writes from the same retained analysis input,
    /// immutable realm, whole source and grammar, in document order, values
    /// **unfolded** ([`crate::auto_path_eval::constant_path_assignments_from_analysis`]).
    /// Constructed namespace source scopes supply no entered frame or current value.
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
    /// Parentless original registry words memo; see [`SourceWordsMemo`].
    pub(crate) source_words_memo: SourceWordsMemo,
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

// Reports compare original source coordinates; retained consumer ownership stays exact.
// Exhaustive destructuring makes a newly added field a compile error until its
// reporting comparison is chosen. The object-handle report is the sole field
// with a purpose-specific comparison; every other field retains ordinary Eq.
macro_rules! analysis_result_report_eq {
    ($left:expr, $right:expr; [$($before:ident),*]; $report:ident; [$($after:ident),*]) => {{
        let AnalysisResult { $($before,)* $report, $($after,)* } = $left;
        $($before == &$right.$before &&)*
            $report.analysis_report_eq(&$right.$report)
            $(&& $after == &$right.$after)*
    }};
}

impl AnalysisResult {
    pub(crate) fn record_unresolved_provider_effect(&mut self) {
        self.has_dynamic_providers = true;
        self.has_unresolved_provider_effects = true;
    }
}

impl PartialEq for AnalysisResult {
    fn eq(&self, other: &Self) -> bool {
        analysis_result_report_eq!(self, other; [
            optimiser_source_origin,
            analysis_context_unavailable,
            command_realm,
            original_command_world,
            original_source_region,
            lexical_declaration_advice,
            resolved_input,
            original_vendor_source_names,
            original_vendor_variable_advice,
            original_vendor_variable_bodies,
            global_scope,
            all_procs,
            original_procedure_metadata,
            original_vendor_procedure_metadata,
            original_vendor_class_metadata,
            original_symbol_metadata,
            original_vendor_symbol_metadata,
            proc_declaration_sites,
            superseded_procs,
            all_classes,
            external_class_contributions,
            logical_procedure_declarations,
            logical_class_declarations,
            original_class_metadata,
            original_counted_class_metadata,
            original_class_configuration_metadata,
            original_object_configuration_metadata,
            superseded_classes,
            class_body_spans,
            all_variables,
            all_defined_symbols,
            diagnostics,
            command_invocations,
            package_requires,
            package_provides,
            package_ifneededs,
            package_prefer_latest,
            has_dynamic_providers,
            has_unresolved_provider_effects,
            source_targets,
            path_constant_assignments,
            command_aliases,
            alias_offsets,
            renamed_commands,
            rename_offsets,
            ensemble_subcommand_targets,
            prefixless_ensembles,
            namespace_imports,
            namespace_forgets,
            original_namespace_patterns,
            destroyed_commands,
            namespace_exports,
            original_namespace_exports,
            original_namespace_export_unknowns,
            namespace_paths,
            namespace_path_computed,
            auto_path_entries,
            qualified_var_refs,
            original_variable_symbols,
            original_variable_write_advice,
            original_variable_symbol_conflicts,
            original_variable_roots,
            original_variable_alias_sites,
            original_variable_name_unknowns,
            original_variable_alias_obligations,
            original_variable_alias_operands,
            original_variable_alias_source_operands,
            original_variable_alias_receipts,
            namespace_refs,
            namespace_name_unknowns,
            dynamic_variable_names,
            namespace_overrides,
            stub_commands,
            stub_expr_defs,
            regex_patterns,
            suppressed_lines,
            unknown_proc_info,
            instance_classes
        ]; object_handle_facts; [
            created_instance_commands,
            instance_command_bindings,
            ambiguous_instance_names,
            object_methods,
            object_member_state,
            unresolved_command_sites,
            scoped_command_regions,
            original_scoped_bodies,
            original_conditional_registry_metadata,
            scoped_sibling_defs,
            hierarchy_cache,
            source_words_memo,
            dialect,
            body_lexer_config,
            library_versions
        ])
    }
}

#[cfg(test)]
// One exhaustive inventory for the bounded diagnostic report. This does not
// participate in AnalysisResult equality, hashing or cache cutoffs.
macro_rules! analysis_result_mismatch_fields {
    ($callback:ident) => {
        $callback! {
            [
                optimiser_source_origin,
                analysis_context_unavailable,
                command_realm,
                original_command_world,
                original_source_region,
                lexical_declaration_advice,
                resolved_input,
                original_vendor_source_names,
                original_vendor_variable_advice,
                original_vendor_variable_bodies,
                global_scope,
                all_procs,
                original_procedure_metadata,
                original_vendor_procedure_metadata,
                original_vendor_class_metadata,
                original_symbol_metadata,
                original_vendor_symbol_metadata,
                proc_declaration_sites,
                superseded_procs,
                all_classes,
                external_class_contributions,
                logical_procedure_declarations,
                logical_class_declarations,
                original_class_metadata,
                original_counted_class_metadata,
                original_class_configuration_metadata,
                original_object_configuration_metadata,
                superseded_classes,
                class_body_spans,
                all_variables,
                all_defined_symbols,
                diagnostics,
                command_invocations,
                package_requires,
                package_provides,
                package_ifneededs,
                package_prefer_latest,
                has_dynamic_providers,
            has_unresolved_provider_effects,
                source_targets,
                path_constant_assignments,
                command_aliases,
                alias_offsets,
                renamed_commands,
                rename_offsets,
                ensemble_subcommand_targets,
                prefixless_ensembles,
                namespace_imports,
                namespace_forgets,
                original_namespace_patterns,
                destroyed_commands,
                namespace_exports,
                original_namespace_exports,
                original_namespace_export_unknowns,
                namespace_paths,
                namespace_path_computed,
                auto_path_entries,
                qualified_var_refs,
                original_variable_symbols,
                original_variable_write_advice,
                original_variable_symbol_conflicts,
                original_variable_roots,
                original_variable_alias_sites,
                original_variable_name_unknowns,
                original_variable_alias_obligations,
                original_variable_alias_operands,
                original_variable_alias_source_operands,
                original_variable_alias_receipts,
                namespace_refs,
                namespace_name_unknowns,
                dynamic_variable_names,
                namespace_overrides,
                stub_commands,
                stub_expr_defs,
                regex_patterns,
                suppressed_lines,
                unknown_proc_info,
                instance_classes,
            ]
            object_handle_facts
            [
                created_instance_commands,
                instance_command_bindings,
                ambiguous_instance_names,
                object_methods,
                object_member_state,
                unresolved_command_sites,
                scoped_command_regions,
                original_scoped_bodies,
                original_conditional_registry_metadata,
                scoped_sibling_defs,
                hierarchy_cache,
                source_words_memo,
                dialect,
                body_lexer_config,
                library_versions,
            ]
        }
    };
}

#[cfg(test)]
impl AnalysisResult {
    /// Bounded description of where two results differ, for comparison tests.
    /// Whole-result `Debug` output can reach gigabytes (it renders retained IR
    /// and source bindings), so each differing field is rendered through a
    /// capped writer instead; equality itself is unchanged ([`PartialEq`]).
    pub(crate) fn bounded_mismatch_summary(&self, other: &Self) -> String {
        fn bounded(value: &dyn std::fmt::Debug) -> String {
            struct Capped(String);
            const LIMIT: usize = 1500;
            impl std::fmt::Write for Capped {
                fn write_str(&mut self, text: &str) -> std::fmt::Result {
                    let room = LIMIT.saturating_sub(self.0.len());
                    if text.len() > room {
                        let mut cut = room;
                        while !text.is_char_boundary(cut) {
                            cut -= 1;
                        }
                        self.0.push_str(&text[..cut]);
                        self.0.push('…');
                        return Err(std::fmt::Error);
                    }
                    self.0.push_str(text);
                    Ok(())
                }
            }
            let mut out = Capped(String::new());
            let _ = std::fmt::write(&mut out, format_args!("{value:?}"));
            out.0
        }
        fn field(name: &str, left: &dyn std::fmt::Debug, right: &dyn std::fmt::Debug) -> String {
            format!(
                "  {name}:\n    left:  {}\n    right: {}",
                bounded(left),
                bounded(right)
            )
        }
        macro_rules! report_fields {
            ([$($before:ident,)*] $special:ident [$($after:ident,)*]) => {{
                let Self { $($before,)* $special, $($after,)* } = self;
                let mut differing = Vec::new();
                if !$special.analysis_report_eq(&other.$special) {
                    differing.push(field(
                        "object_handle_facts", $special, &other.$special,
                    ));
                }
                $(if $before != &other.$before {
                    differing.push(field(stringify!($before), $before, &other.$before));
                })*
                $(if $after != &other.$after {
                    differing.push(field(stringify!($after), $after, &other.$after));
                })*
                differing.join("\n")
            }};
        }
        analysis_result_mismatch_fields!(report_fields)
    }
}

/// Assert two analysis results are equal, reporting only a bounded summary
/// of the differing fields on failure.
#[cfg(test)]
#[track_caller]
pub(crate) fn assert_analysis_results_eq(
    left: &AnalysisResult,
    right: &AnalysisResult,
    context: &str,
) {
    assert!(
        left == right,
        "analysis results differ ({context}); differing fields:\n{}",
        left.bounded_mismatch_summary(right)
    );
}

bitflags::bitflags! {
    /// Trailing ``?-flag…?`` flags on a ``# tcl-lsp: stub`` line:
    /// ``barrier`` / ``loop`` / ``pure`` / ``mutator`` / ``unsafe``
    /// / ``scope_alias`` / ``extension``, packed into a single byte
    /// because they're an enum-set of orthogonal flags.
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
        /// ``-extension`` — a native extension registers the command, so
        /// nothing is known of it: the declaration starts at the registry's
        /// conservative default on every axis
        /// ([`tcl_registry::extension_default`]) and the other flags narrow it
        /// one axis each.
        const EXTENSION   = 1 << 6;
    }
}

mod channel_handlers;
mod class_def;
mod class_factories;
mod class_members;
mod definition_queries;
mod diagnostic;
mod occurrence_records;
mod original_declarations;
mod procedures;
mod result_caches;
mod scopes;
mod source_world;
mod stubs;
mod variable_renames;

pub use channel_handlers::ChannelHandlerUse;
pub use class_def::{
    ClassCommandFallback, ClassDef, MetaclassProvenance, OriginalCountedBodyMethod,
    OriginalCountedClass, SourceNameAmbiguity,
};
pub use class_factories::{
    ClassFactory, ClassFactoryIndex, DeferredClassCreation, FactoryMember, FactoryWord,
    ManufacturerSpec, SubclassProvidedMethods,
};
pub use class_members::{
    DefinitionAbort, DefinitionAbortKind, MemberRetractionRecord, MemberSide, MethodDef,
    ObjectMemberState, ObjectMethodDef, PropertyDef, RenamedMember, class_constructor_key,
    class_destructor_key, class_member_key, class_property_key,
};
pub use diagnostic::{Diagnostic, SignedDataGroupComparison};
pub use occurrence_records::{
    AutoPathEntry, AutoPathForm, DynamicVariableNameSite, InstanceCommandBinding, NamespaceRef,
    PackageIfneeded, PackageProvide, QualifiedVarRef, RegexPattern,
};
pub use procedures::{
    DefinedSymbol, EnsembleSubcommandTarget, PendingCtorArity, PendingNextArity,
    PendingUserCallArity, ProcArgTrait, ProcDef, ScriptParameter, ScriptParameterFrame,
    ScriptParameterTiming, UnknownProcInfo,
};
pub use result_caches::{HierarchyCache, SourceWordsMemo};
pub(super) use scopes::InterpreterSourceBody;
pub use scopes::{Scope, ScopeKind, VarDef};
pub use stubs::{StubArgDef, StubCommandDef, StubExprDef, build_declared_surface};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_declaration_inventory_tests;

#[cfg(test)]
mod original_definition_metadata_tests;

#[cfg(test)]
mod original_completed_world_transport_tests;

#[cfg(test)]
mod original_namespace_operand_tests;

#[cfg(test)]
mod original_namespace_pattern_tests;

#[cfg(test)]
mod original_namespace_unknown_tests;

#[cfg(test)]
mod original_namespace_export_inventory_tests;

#[cfg(test)]
mod lexical_source_advice_domain_tests;

#[cfg(test)]
mod original_source_context_tests;

#[cfg(test)]
mod original_definition_suffix_purpose_tests;

#[cfg(test)]
mod source_alias_coverage_tests;

#[cfg(test)]
mod original_body_syntax_request_tests;
