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
//! The foundation of per-item incremental analysis (slice 1 of the plan in
//! `docs/design/rust/incremental-analysis.md`). Splits a file into *items*
//! (procs, classes, methods, namespaces, aliases, ensembles) whose identity is
//! a **stable name + kind**, never a source position — so inserting blank lines
//! above a proc leaves its [`ItemId`] unchanged and a later memoised
//! `item_analysis` (slice 3) is a cache hit.
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
//! ## Correctness anchor (slice 1)
//!
//! Reproducing the analyser's namespace-aware item detection with a *separate*
//! walker is exactly the work the design backs with the differential fuzzer +
//! full-rebuild fallback (see the experiments doc: `signature_scan` already
//! diverges from `analyse` on dynamic `${…}` names and body-nested procs). So
//! slice 1 builds the item tree from the **authoritative [`AnalysisResult`]**
//! the analyser already produces from the CST — it therefore *cannot* diverge
//! from `analyse`. The [`FileDecls`] corpus gate (`tcl-lsp-db`'s
//! `file_decls_corpus` test) is the permanent guard that protects the swap to a
//! cheap, independent CST extractor in slices 2–3, where it becomes
//! load-bearing alongside the fuzzer.

use std::collections::{BTreeSet, HashSet};

use tcl_lexer::Span;

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

/// An item's signature — the cross-item-relevant header, with no body.
///
/// A body-only edit leaves every `ItemSig` byte-identical, which is what lets
/// the cross-item aggregate queries early-cutoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSig {
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
    /// `proc p [makeargs] {…}` reached the cross-file check as a proc with an
    /// empty parameter list, i.e. "takes no arguments", and every call to it
    /// from another file drew a false `E003` (issue #1107).
    pub params_computed: bool,
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
}

/// The aggregate declaration sets the cross-item passes read: the firewall's
/// "signature side". Built from [`ItemSig`]s; deterministic via `BTreeSet`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDecls {
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
    match qualified.rsplit_once("::") {
        Some((prefix, _)) if !prefix.is_empty() => prefix.to_string(),
        _ => "::".to_string(),
    }
}

/// Join a namespace prefix with a (possibly absolute) child name, mirroring the
/// absolute-reset rule the analyser's `command_resolution_namespace` uses.
fn join_ns(prefix: &str, name: &str) -> String {
    if name.starts_with("::") {
        name.to_string()
    } else if prefix == "::" {
        format!("::{name}")
    } else {
        format!("{prefix}::{name}")
    }
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
    /// the result). See the module docs for why slice 1 anchors to `analyse`.
    #[must_use]
    pub fn from_analysis(result: &AnalysisResult, ensembles: &HashSet<String>) -> Self {
        let mut items: Vec<Item> = Vec::new();

        for (qualified, proc) in &result.all_procs {
            items.push(Item {
                sig: ItemSig {
                    id: ItemId::new(ItemKind::Proc, qualified.clone()),
                    namespace: enclosing_namespace(qualified),
                    params: proc.params.clone(),
                    params_computed: proc.params_computed,
                    name_span: proc.name_span,
                },
                body_span: Some(proc.body_span),
            });
        }

        for (qualified, class) in &result.all_classes {
            items.push(Item {
                sig: ItemSig {
                    id: ItemId::new(ItemKind::Class, qualified.clone()),
                    namespace: enclosing_namespace(qualified),
                    params: Vec::new(),
                    params_computed: false,
                    name_span: class.name_span,
                },
                body_span: Some(class.body_span),
            });
            let mut push_method =
                |key: String, name_span: Span, body_span: Span, params: Vec<ParamDef>| {
                    items.push(Item {
                        sig: ItemSig {
                            id: ItemId::new(ItemKind::Method, key),
                            namespace: qualified.clone(),
                            params,
                            params_computed: false,
                            name_span,
                        },
                        body_span: Some(body_span),
                    });
                };
            for m in result_methods(class) {
                push_method(m.key, m.name_span, m.body_span, m.params);
            }
        }

        for qualified in result.command_aliases.keys() {
            items.push(Item {
                sig: ItemSig {
                    id: ItemId::new(ItemKind::Alias, qualified.clone()),
                    namespace: enclosing_namespace(qualified),
                    params: Vec::new(),
                    params_computed: false,
                    name_span: Span::new(0, 0),
                },
                body_span: None,
            });
        }

        for ns in ensembles {
            items.push(Item {
                sig: ItemSig {
                    id: ItemId::new(ItemKind::Ensemble, ns.clone()),
                    namespace: ns.clone(),
                    params: Vec::new(),
                    params_computed: false,
                    name_span: Span::new(0, 0),
                },
                body_span: None,
            });
        }

        let mut namespaces = BTreeSet::new();
        collect_namespaces(&result.global_scope, "::", &mut namespaces);
        for ns in &namespaces {
            items.push(Item {
                sig: ItemSig {
                    id: ItemId::new(ItemKind::Namespace, ns.clone()),
                    namespace: enclosing_namespace(ns),
                    params: Vec::new(),
                    params_computed: false,
                    name_span: Span::new(0, 0),
                },
                body_span: None,
            });
        }

        // Deterministic order so `ItemTree` value-equality is stable across the
        // analyser's non-deterministic `HashMap` iteration order.
        items.sort_by(|a, b| a.sig.id.cmp(&b.sig.id));
        Self { items }
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
        });
    }
    for (name, m) in &class.class_methods {
        out.push(FlatMethod {
            key: super::types::class_member_key(cq, name, true),
            name_span: m.name_span,
            body_span: m.body_span,
            params: m.params.clone(),
        });
    }
    for (i, m) in class.constructors.iter().enumerate() {
        out.push(FlatMethod {
            key: format!("{cq}::constructor::{i}"),
            name_span: m.name_span,
            body_span: m.body_span,
            params: m.params.clone(),
        });
    }
    if let Some(m) = &class.destructor {
        out.push(FlatMethod {
            key: format!("{cq}::destructor"),
            name_span: m.name_span,
            body_span: m.body_span,
            params: m.params.clone(),
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
        // must too (E1: a body edit adding/removing a nested def is a signature
        // change).
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
    fn file_decls_match_analysis_decl_sets() {
        // The contract the corpus gate enforces at scale: file_decls equals the
        // analyser's own decl maps. True by construction in slice 1; the guard
        // bites when slices 2–3 swap in an independent extractor.
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
}
