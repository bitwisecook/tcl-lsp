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

//! Item tree — offset-stable structural view of a file's declarations.
//!
//! The foundation of per-item incremental analysis
//! (`docs/design/rust/incremental-analysis.md`). Splits a file into *items*
//! (procs, classes, methods, namespaces, aliases, ensembles) whose identity is
//! a **stable name + kind**, never a source position — so inserting blank lines
//! above a proc leaves its [`ItemId`] unchanged and a later memoised
//! `item_analysis` is a cache hit.
//!
//! ## Firewall shape
//!
//! - [`ItemSig`] is an item's *signature* (name, namespace, params, name span):
//!   the cross-item-relevant header. [`ItemTree`] carries one [`Item`] per
//!   declaration with its signature plus its body span.
//! - [`FileDecls`] is the aggregate the cross-item passes read: the set of
//!   declared procs / classes / aliases / ensembles + the namespace tree. A
//!   body-only edit leaves every signature equal, so `FileDecls` is unchanged
//!   and salsa early-cutoff fires on the dependent cross-item queries.
//!
//! ## Correctness anchor
//!
//! Reproducing the analyser's namespace-aware item detection with a *separate*
//! walker is exactly the work the design backs with the differential fuzzer +
//! full-rebuild fallback (see the experiments doc: `signature_scan` already
//! diverges from `analyse` on dynamic `${…}` names and body-nested procs). So
//! the item tree is built from the **authoritative [`AnalysisResult`]** the
//! analyser already produces from the CST — it therefore *cannot* diverge from
//! `analyse`. The [`FileDecls`] corpus gate (`tcl-lsp-db`'s
//! `file_decls_corpus` test) is the permanent guard that would protect a swap
//! to a cheap, independent CST extractor, where it becomes load-bearing
//! alongside the fuzzer.

use std::collections::{BTreeSet, HashSet};

use tcl_lexer::Span;

use crate::signature_scan::formal_count::{SourceFormalCount, SourceFormalCountProjection};
use crate::signature_scan::types::ParamDef;

use super::types::{AnalysisResult, Scope, ScopeKind};

/// What kind of declaration an [`Item`] is.
///
/// The discriminant participates in [`ItemId`] so a proc and a namespace that
/// happen to share a qualified name (Tcl allows it) stay distinct items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemKind {
    /// A `proc` definition.
    Proc,
    /// A `TclOO` / itcl class definition.
    Class,
    /// A method / constructor / destructor inside a class body.
    Method,
    /// A `namespace eval` scope.
    Namespace,
    /// An `interp alias` recorded for command resolution.
    Alias,
    /// A namespace marked by `namespace ensemble create`.
    Ensemble,
}

/// Offset-stable identity of an [`Item`].
///
/// `key` is the fully-qualified name (procs / classes / namespaces / aliases /
/// ensembles) or, for methods, a `class-qualified-name + kind + name`
/// composite so the identity survives offset shifts and method reordering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId {
    /// Declaration kind.
    pub kind: ItemKind,
    /// Stable name key (see type docs).
    pub key: String,
}

impl ItemId {
    fn new(kind: ItemKind, key: impl Into<String>) -> Self {
        Self {
            kind,
            key: key.into(),
        }
    }
}

/// Body-free projection of an independently retained source declaration.
/// Exact publication geometry remains distinct from the UI item identifier.
/// This header supplies no command existence or temporal installation proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDeclarationSignature {
    kind: ItemKind,
    name: crate::signature_scan::scope::SignatureSourceCommand,
    declaration_offset: u32,
    name_span: Span,
    params: Vec<ParamDef>,
    params_computed: bool,
    formal_count: SourceFormalCountProjection,
}

impl SourceDeclarationSignature {
    /// Project the independently retained original procedure declaration.
    /// The header grants neither installed identity nor entered arguments.
    #[must_use]
    pub fn from_original_procedure(
        record: &crate::signature_scan::original_name::SourceDeclarationMetadata<
            super::types::ProcDef,
        >,
    ) -> Self {
        let metadata = record.metadata();
        Self {
            kind: ItemKind::Proc,
            name: record.name().clone(),
            declaration_offset: record.declaration_site().offset,
            name_span: metadata.name_span,
            params: metadata.params.clone(),
            params_computed: metadata.params_computed,
            formal_count: metadata.formal_count_projection(),
        }
    }

    fn class(
        record: &crate::signature_scan::original_name::SourceDeclarationMetadata<
            super::types::ClassDef,
        >,
    ) -> Self {
        Self {
            kind: ItemKind::Class,
            name: record.name().clone(),
            declaration_offset: record.declaration_site().offset,
            name_span: record.metadata().name_span,
            params: Vec::new(),
            params_computed: false,
            formal_count: SourceFormalCount::Unknown.projection(&[], false),
        }
    }

    /// Source declaration category, independently of a displayed label.
    #[must_use]
    pub fn kind(&self) -> ItemKind {
        self.kind
    }
    /// Retained selected publication bytes and complete policy.
    #[must_use]
    pub fn name(&self) -> &crate::signature_scan::scope::SignatureSourceCommand {
        &self.name
    }
    /// Original declaration offset, without an identity or edit grant.
    #[must_use]
    pub fn declaration_offset(&self) -> u32 {
        self.declaration_offset
    }
    /// Original reported name extent, without an edit grant.
    #[must_use]
    pub fn name_span(&self) -> Span {
        self.name_span
    }
    /// Parameter advice; native formal binding has its separate typed owner.
    #[must_use]
    pub fn params(&self) -> &[ParamDef] {
        &self.params
    }
    /// Whether the parameter advice is unknown rather than empty.
    #[must_use]
    pub fn params_computed(&self) -> bool {
        self.params_computed
    }
    /// Frozen count metadata from the declaration's actual formal owner.
    /// It carries no source image, activation or argument-binding capability.
    #[must_use]
    pub const fn formal_count_projection(&self) -> SourceFormalCountProjection {
        // naming.database.original-formal-count-header
        // docs/design/analysis/name-resolution-proofs/database-original-formal-count-header.md
        self.formal_count
    }
}

/// An item's signature — the cross-item-relevant header, with no body.
///
/// A body-only edit leaves every `ItemSig` byte-identical, which is what lets
/// the cross-item aggregate queries early-cutoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSig {
    /// Authentic body-free source header; advisory UI records have none.
    pub original_declaration: Option<SourceDeclarationSignature>,
    /// Retained authored command geometry; report keys supply no native identity.
    pub source_name: Option<crate::signature_scan::scope::SignatureSourceCommand>,
    /// Stable identity.
    pub id: ItemId,
    /// Enclosing namespace (`"::"` for the global namespace), or the owning
    /// class qualified name for methods.
    pub namespace: String,
    /// Declared parameters (procs / methods; empty otherwise).
    ///
    /// Empty *and* [`Self::params_computed`] set means "unknown", not "none".
    pub params: Vec<ParamDef>,
    /// The declaration's parameter-list word was **computed**, so its formals
    /// are unmodelled ([`crate::analyser::ProcDef::params_computed`]).
    ///
    /// Carried on the signature — not just on the analyser record — because
    /// the *cross-file* arity table is built from `ItemSig` alone. Without it,
    /// `proc p [makeargs] {…}` would reach the cross-file check as a proc with
    /// an empty parameter list, i.e. "takes no arguments", and every call to it
    /// from another file would draw a false `E003`.
    pub params_computed: bool,
    /// Body-free count and grammar projected by the genuine formal owner.
    pub formal_count: SourceFormalCountProjection,
    /// Source span of the name token (`Span::new(0, 0)` when the analyser
    /// record carries no name span — e.g. aliases / ensembles).
    pub name_span: Span,
}

/// One declaration: its signature plus the span of its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Signature (header).
    pub sig: ItemSig,
    /// Body span (braces excluded), `None` for bodyless items (aliases /
    /// ensembles).
    pub body_span: Option<Span>,
}

/// The offset-stable item tree for a file: one [`Item`] per declaration,
/// sorted by [`ItemId`] for deterministic equality (the source `HashMap`s the
/// analyser populates have non-deterministic iteration order, which would
/// otherwise defeat salsa value-equality early-cutoff).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemTree {
    /// Items in stable [`ItemId`] order.
    pub items: Vec<Item>,
    /// The user-defined `TclOO` **class factories** this file declares, keyed
    /// by qualified name — what the workspace merges so *another* file's walk
    /// can classify `Meta create Name …`.
    ///
    /// It rides on the item tree because that is already a structure-level,
    /// per-file, memoised product of the same walk: publishing the factories
    /// here costs no extra analysis pass, and a body edit that leaves the
    /// metaclass's `create` override alone leaves this equal, so the
    /// cross-file query early-cutoffs. Behind an `Arc` because the consumer
    /// hands it straight to the analyser.
    pub class_factories: std::sync::Arc<super::types::ClassFactoryIndex>,
}

/// The aggregate declaration sets the cross-item passes read: the firewall's
/// "signature side". Built from [`ItemSig`]s; deterministic via `BTreeSet`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDecls {
    /// Every independently retained original procedure/class header. Equal
    /// display names and repeated publications remain distinct records.
    pub original_declarations: Vec<SourceDeclarationSignature>,
    /// Qualified names of every `proc`.
    pub procs: BTreeSet<String>,
    /// Qualified names of every class.
    pub classes: BTreeSet<String>,
    /// Qualified names of every recorded command alias.
    pub aliases: BTreeSet<String>,
    /// Namespaces marked as ensembles.
    pub ensembles: BTreeSet<String>,
    /// Every namespace in the file's namespace tree.
    pub namespaces: BTreeSet<String>,
}

/// Enclosing namespace of a fully-qualified name (`"::ns::foo"` → `"::ns"`,
/// `"::foo"` → `"::"`).
fn enclosing_namespace(qualified: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qualified);
    if holder.is_empty() {
        "::".to_string()
    } else {
        holder.to_string()
    }
}

/// Join a namespace prefix with a (possibly absolute) child name, mirroring the
/// absolute-reset rule the analyser's `command_resolution_namespace` uses.
fn join_ns(prefix: &str, name: &str) -> String {
    tcl_syntax::naming::qualify(prefix, name)
}

/// Walk the scope tree collecting every namespace's qualified name.
fn collect_namespaces(scope: &Scope, prefix: &str, out: &mut BTreeSet<String>) {
    for child in &scope.children {
        match child.kind {
            ScopeKind::Namespace => {
                let q = join_ns(prefix, &child.name);
                collect_namespaces(child, &q, out);
                out.insert(q);
            }
            // Proc / method / uplevel bodies can still contain `namespace
            // eval`; keep the enclosing prefix and keep descending.
            _ => collect_namespaces(child, prefix, out),
        }
    }
}

impl ItemTree {
    /// Build the item tree from the authoritative [`AnalysisResult`] plus the
    /// analyser's `ensemble_namespaces` set (which lives on the `Analyser`, not
    /// the result). See the module docs for why the tree anchors to `analyse`.
    #[must_use]
    pub fn from_analysis(result: &AnalysisResult, ensembles: &HashSet<String>) -> Self {
        // naming.database.original-formal-count-header
        // docs/design/analysis/name-resolution-proofs/database-original-formal-count-header.md
        let mut items: Vec<Item> = Vec::new();

        let original_procs = result.original_procedure_declarations().map(|record| {
            (
                &record.metadata().qualified_name,
                record.metadata(),
                Some(SourceDeclarationSignature::from_original_procedure(record)),
            )
        });
        // Full metadata equality only suppresses the duplicate UI projection.
        // It does not issue original provenance for an advisory record.
        let advisory_procs = result
            .all_procs
            .iter()
            .filter(|(_, metadata)| {
                !result
                    .original_procedure_declarations()
                    .any(|record| record.metadata() == *metadata)
            })
            .map(|(qualified, metadata)| (qualified, metadata, None));
        for (qualified, proc, original_declaration) in original_procs.chain(advisory_procs) {
            items.push(Item {
                sig: ItemSig {
                    original_declaration,
                    source_name: proc.source_name.clone(),
                    id: ItemId::new(ItemKind::Proc, qualified.clone()),
                    namespace: enclosing_namespace(qualified),
                    params: proc.params.clone(),
                    params_computed: proc.params_computed,
                    formal_count: proc.formal_count_projection(),
                    name_span: proc.name_span,
                },
                body_span: Some(proc.body_span),
            });
        }

        Self::push_class_items(result, &mut items);

        for qualified in result.command_aliases.keys() {
            items.push(Item {
                sig: ItemSig {
                    original_declaration: None,
                    source_name: None,
                    id: ItemId::new(ItemKind::Alias, qualified.clone()),
                    namespace: enclosing_namespace(qualified),
                    params: Vec::new(),
                    params_computed: false,
                    formal_count: SourceFormalCount::Unknown.projection(&[], false),
                    name_span: Span::new(0, 0),
                },
                body_span: None,
            });
        }

        for ns in ensembles {
            items.push(Item {
                sig: ItemSig {
                    original_declaration: None,
                    source_name: None,
                    id: ItemId::new(ItemKind::Ensemble, ns.clone()),
                    namespace: ns.clone(),
                    params: Vec::new(),
                    params_computed: false,
                    formal_count: SourceFormalCount::Unknown.projection(&[], false),
                    name_span: Span::new(0, 0),
                },
                body_span: None,
            });
        }

        Self::push_namespace_items(result, &mut items);

        // Deterministic order so `ItemTree` value-equality is stable across the
        // analyser's non-deterministic `HashMap` iteration order.
        items.sort_by(|a, b| {
            a.sig
                .id
                .cmp(&b.sig.id)
                .then(a.sig.name_span.start().cmp(&b.sig.name_span.start()))
                .then(a.sig.name_span.end().cmp(&b.sig.name_span.end()))
        });
        Self {
            items,
            class_factories: std::sync::Arc::new(result.class_factories()),
        }
    }

    fn push_class_items(result: &AnalysisResult, items: &mut Vec<Item>) {
        let original_classes = result.original_class_declarations().map(|record| {
            (
                &record.metadata().qualified_name,
                record.metadata(),
                Some(SourceDeclarationSignature::class(record)),
            )
        });
        let advisory_classes = result
            .all_classes
            .iter()
            .filter(|(_, metadata)| {
                !result
                    .original_class_declarations()
                    .any(|record| record.metadata() == *metadata)
            })
            .map(|(qualified, metadata)| (qualified, metadata, None));
        for (qualified, class, original_declaration) in original_classes.chain(advisory_classes) {
            items.push(Item {
                sig: ItemSig {
                    source_name: original_declaration
                        .as_ref()
                        .map(|header| header.name().clone()),
                    original_declaration,
                    id: ItemId::new(ItemKind::Class, qualified.clone()),
                    namespace: enclosing_namespace(qualified),
                    params: Vec::new(),
                    params_computed: false,
                    formal_count: SourceFormalCount::Unknown.projection(&[], false),
                    name_span: class.name_span,
                },
                body_span: Some(class.body_span),
            });
            for method in result_methods(class) {
                items.push(Item {
                    sig: ItemSig {
                        original_declaration: None,
                        source_name: None,
                        id: ItemId::new(ItemKind::Method, method.key),
                        namespace: qualified.clone(),
                        params: method.params,
                        params_computed: method.params_computed,
                        formal_count: method.formal_count,
                        name_span: method.name_span,
                    },
                    body_span: Some(method.body_span),
                });
            }
        }
    }

    fn push_namespace_items(result: &AnalysisResult, items: &mut Vec<Item>) {
        let mut namespaces = BTreeSet::new();
        collect_namespaces(&result.global_scope, "::", &mut namespaces);
        for ns in &namespaces {
            items.push(Item {
                sig: ItemSig {
                    original_declaration: None,
                    source_name: None,
                    id: ItemId::new(ItemKind::Namespace, ns.clone()),
                    namespace: enclosing_namespace(ns),
                    params: Vec::new(),
                    params_computed: false,
                    formal_count: SourceFormalCount::Unknown.projection(&[], false),
                    name_span: Span::new(0, 0),
                },
                body_span: None,
            });
        }
    }

    /// The signatures of every item, in stable [`ItemId`] order. This is the
    /// `item_sig*` projection of the design's query graph — the firewall's
    /// cross-item-relevant input.
    #[must_use]
    pub fn sigs(&self) -> Vec<ItemSig> {
        self.items.iter().map(|it| it.sig.clone()).collect()
    }

    /// Aggregate the declaration sets the cross-item passes read.
    #[must_use]
    pub fn file_decls(&self) -> FileDecls {
        FileDecls::from_sigs(self.items.iter().map(|it| &it.sig))
    }
}

impl FileDecls {
    /// Aggregate declaration sets from item signatures — the design's
    /// `file_decls ← item_sig*` edge. Body-independent: a body-only edit leaves
    /// every `ItemSig` equal, so `FileDecls` is unchanged.
    pub fn from_sigs<'a>(sigs: impl IntoIterator<Item = &'a ItemSig>) -> Self {
        let mut decls = FileDecls::default();
        for sig in sigs {
            if let Some(header) = &sig.original_declaration {
                decls.original_declarations.push(header.clone());
            }
            match sig.id.kind {
                ItemKind::Proc => {
                    decls.procs.insert(sig.id.key.clone());
                }
                ItemKind::Class => {
                    decls.classes.insert(sig.id.key.clone());
                }
                ItemKind::Alias => {
                    decls.aliases.insert(sig.id.key.clone());
                }
                ItemKind::Ensemble => {
                    decls.ensembles.insert(sig.id.key.clone());
                }
                ItemKind::Namespace => {
                    decls.namespaces.insert(sig.id.key.clone());
                }
                ItemKind::Method => {}
            }
        }
        decls
    }
}

/// A flattened method record (one per method / class-method / constructor /
/// destructor) with a stable composite key.
struct FlatMethod {
    key: String,
    name_span: Span,
    body_span: Span,
    params: Vec<ParamDef>,
    params_computed: bool,
    formal_count: SourceFormalCountProjection,
}

/// Flatten a class's methods into stable-keyed [`FlatMethod`]s. Constructors
/// are ordinal-keyed because `oo::configurable` allows several.
fn result_methods(class: &super::types::ClassDef) -> Vec<FlatMethod> {
    let cq = &class.qualified_name;
    let mut out = Vec::new();
    for (name, m) in &class.methods {
        out.push(FlatMethod {
            key: super::types::class_member_key(cq, name, false),
            name_span: m.name_span,
            body_span: m.body_span,
            params: m.params.clone(),
            params_computed: m.params_computed,
            formal_count: m.formal_count_projection(),
        });
    }
    for (name, m) in &class.class_methods {
        out.push(FlatMethod {
            key: super::types::class_member_key(cq, name, true),
            name_span: m.name_span,
            body_span: m.body_span,
            params: m.params.clone(),
            params_computed: m.params_computed,
            formal_count: m.formal_count_projection(),
        });
    }
    for (i, m) in class.constructors.iter().enumerate() {
        out.push(FlatMethod {
            key: format!("{cq}::constructor::{i}"),
            name_span: m.name_span,
            body_span: m.body_span,
            params: m.params.clone(),
            params_computed: m.params_computed,
            formal_count: m.formal_count_projection(),
        });
    }
    if let Some(m) = &class.destructor {
        out.push(FlatMethod {
            key: format!("{cq}::destructor"),
            name_span: m.name_span,
            body_span: m.body_span,
            params: m.params.clone(),
            params_computed: m.params_computed,
            formal_count: m.formal_count_projection(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    fn build(src: &str) -> (ItemTree, FileDecls) {
        let mut a = Analyser::new();
        let result = a.analyse(src, "tcl8.6");
        let tree = ItemTree::from_analysis(&result, &a.ensemble_namespaces);
        let decls = tree.file_decls();
        (tree, decls)
    }

    #[test]
    fn enclosing_namespace_cases() {
        assert_eq!(enclosing_namespace("::foo"), "::");
        assert_eq!(enclosing_namespace("::ns::foo"), "::ns");
        assert_eq!(enclosing_namespace("::a::b::foo"), "::a::b");
        assert_eq!(enclosing_namespace("::::::"), ":::");
        assert_eq!(join_ns("::", ":"), ":::");
        assert_eq!(join_ns(":::", ":"), "::::::");
        assert_eq!(join_ns("::", "foo:::"), "::foo::");
    }

    #[test]
    fn top_level_and_nested_procs_are_items() {
        let (_, decls) = build("proc foo {a b} { set x 1 }\nnamespace eval ns { proc bar {} {} }");
        assert!(decls.procs.contains("::foo"));
        assert!(decls.procs.contains("::ns::bar"));
        assert!(decls.namespaces.contains("::ns"));
    }

    #[test]
    fn body_nested_proc_is_an_item() {
        // The analyser records procs defined inside proc bodies; the item tree
        // must too: a body edit adding or removing a nested definition is a
        // signature change.
        let (_, decls) = build("proc outer {} { proc ::inner {} {} }");
        assert!(decls.procs.contains("::outer"));
        assert!(decls.procs.contains("::inner"));
    }

    #[test]
    fn classes_methods_and_ensembles() {
        let src = "oo::class create Shape {\n  method area {} { return 0 }\n  constructor {} {}\n}\nnamespace eval ::e { namespace ensemble create }";
        let (tree, decls) = build(src);
        assert!(decls.classes.contains("::Shape"));
        assert!(decls.ensembles.contains("::e"));
        assert!(
            tree.items
                .iter()
                .any(|it| it.sig.id.kind == ItemKind::Method && it.sig.id.key.contains("area"))
        );
    }

    #[test]
    fn item_tree_is_deterministic() {
        let src = "proc a {} {}\nproc b {} {}\nproc c {} {}\nnamespace eval n { proc d {} {} }";
        let (t1, _) = build(src);
        let (t2, _) = build(src);
        assert_eq!(t1, t2, "item tree must be order-stable across runs");
    }

    #[test]
    fn original_item_headers_survive_display_collision_and_cleared_ui_maps() {
        let mut analyser = Analyser::new();
        let mut result = analyser.analyse(
            r"proc p\uD800 {} {}; proc p\uD801 {} {}; oo::class create C\uD800 {}; oo::class create C\uD801 {}",
            "tcl8.6",
        );
        assert_eq!(result.original_procedure_declarations().count(), 2);
        assert_eq!(result.original_class_declarations().count(), 2);
        result.all_procs.clear();
        result.all_classes.clear();
        let tree = ItemTree::from_analysis(&result, &analyser.ensemble_namespaces);
        let decls = FileDecls::from_sigs(tree.sigs().iter());
        assert_eq!(decls.original_declarations.len(), 4);
        let names = decls
            .original_declarations
            .iter()
            .map(|header| {
                (
                    header.kind(),
                    header.name().slot().simple.as_bytes().to_vec(),
                )
            })
            .collect::<HashSet<_>>();
        assert!(names.contains(&(ItemKind::Proc, b"p\xed\xa0\x80".to_vec())));
        assert!(names.contains(&(ItemKind::Proc, b"p\xed\xa0\x81".to_vec())));
        assert!(names.contains(&(ItemKind::Class, b"C\xed\xa0\x80".to_vec())));
        assert!(names.contains(&(ItemKind::Class, b"C\xed\xa0\x81".to_vec())));
    }

    #[test]
    fn original_item_header_excludes_body_source_and_length() {
        let (before, before_decls) = build("proc p {a} { return 1 }");
        let (after, after_decls) = build("proc p {a} { return LONGER }");
        assert_eq!(before.sigs(), after.sigs());
        assert_eq!(before_decls, after_decls);
        assert_eq!(before_decls.original_declarations.len(), 1);
        assert_ne!(before.items[0].body_span, after.items[0].body_span);
    }

    #[test]
    fn file_decls_match_analysis_decl_sets() {
        // The contract the corpus gate enforces at scale: file_decls equals the
        // analyser's own decl maps. True by construction while the tree is built
        // from `AnalysisResult`; the guard bites if an independent extractor is
        // swapped in.
        let src = "proc p {} {}\noo::class create K {}\nnamespace eval z { namespace ensemble create\n proc q {} {} }";
        let mut a = Analyser::new();
        let result = a.analyse(src, "tcl8.6");
        let decls = ItemTree::from_analysis(&result, &a.ensemble_namespaces).file_decls();
        let want_procs: BTreeSet<String> = result.all_procs.keys().cloned().collect();
        let want_classes: BTreeSet<String> = result.all_classes.keys().cloned().collect();
        let want_aliases: BTreeSet<String> = result.command_aliases.keys().cloned().collect();
        let want_ensembles: BTreeSet<String> = a.ensemble_namespaces.iter().cloned().collect();
        assert_eq!(decls.procs, want_procs);
        assert_eq!(decls.classes, want_classes);
        assert_eq!(decls.aliases, want_aliases);
        assert_eq!(decls.ensembles, want_ensembles);
    }

    #[test]
    fn formal_count_headers_retain_c_and_jim_grammar_without_body_bytes() {
        // naming.database.original-formal-count-header
        // docs/design/analysis/name-resolution-proofs/database-original-formal-count-header.md
        use crate::signature_scan::formal_count::SourceFormalCountOrigin;
        for (dialect, minimum, grammar) in [
            ("tcl8.4", 3, tcl_dialect::ParameterGrammar::Tcl),
            ("tcl8.5", 3, tcl_dialect::ParameterGrammar::Tcl),
            ("tcl8.6", 3, tcl_dialect::ParameterGrammar::Tcl),
            ("tcl9.0", 3, tcl_dialect::ParameterGrammar::Tcl),
            ("tcl9.1", 3, tcl_dialect::ParameterGrammar::Tcl),
            ("jim", 2, tcl_dialect::ParameterGrammar::Jim),
        ] {
            let analyse = |source: &str| {
                let mut analyser = Analyser::new();
                let result = analyser.analyse(source, dialect);
                ItemTree::from_analysis(&result, &analyser.ensemble_namespaces)
            };
            let before = analyse("proc mixed {a {b B} c} {return FIRST}");
            let after = analyse("proc mixed {a {b B} c} {return A_LONGER_BODY}");
            let count = before.items[0].sig.formal_count;
            assert_eq!(
                count.arity(),
                tcl_registry::Arity::new(minimum, 3),
                "{dialect}"
            );
            assert_eq!(
                count.origin(),
                SourceFormalCountOrigin::OriginalSource,
                "{dialect}"
            );
            assert_eq!(count.parameter_grammar(), Some(grammar), "{dialect}");
            assert_eq!(before.sigs(), after.sigs(), "{dialect}");
            assert_eq!(before.file_decls(), after.file_decls(), "{dialect}");
            assert_eq!(
                before.items[0]
                    .sig
                    .original_declaration
                    .as_ref()
                    .unwrap()
                    .formal_count_projection(),
                count
            );
        }
    }

    #[test]
    fn original_formal_count_header_ignores_reported_parameter_labels() {
        // naming.database.original-formal-count-header
        // docs/design/analysis/name-resolution-proofs/database-original-formal-count-header.md
        let mut analyser = Analyser::new();
        let mut result = analyser.analyse("proc p {a args} {}", "tcl8.6");
        let original = result.all_procs["::p"].formal_count_projection();
        let metadata = result.all_procs.get_mut("::p").unwrap();
        metadata.params.clear();
        assert_eq!(metadata.formal_count_projection(), original);
        result.all_procs.clear();
        let tree = ItemTree::from_analysis(&result, &analyser.ensemble_namespaces);
        assert_eq!(tree.items.len(), 1);
        assert_eq!(tree.items[0].sig.formal_count, original);
        assert_eq!(
            original.arity(),
            tcl_registry::Arity::new(1, tcl_registry::Arity::UNLIMITED)
        );
        let unknown = SourceFormalCount::Unknown.projection(&[], false);
        assert_eq!(
            unknown.origin(),
            crate::signature_scan::formal_count::SourceFormalCountOrigin::Unknown
        );
        assert_eq!(unknown.arity(), tcl_registry::Arity::any());
        assert_eq!(unknown.parameter_grammar(), None);
    }
}
