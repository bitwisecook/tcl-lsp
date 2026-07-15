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
//! Only procs, classes, and command invocations are indexed
//! today (the cross-document features that need them); variables
//! and namespaces are not.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::class_hierarchy::{build_tail_index, resolve_class_name};
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
    /// Names of methods the class *directly defines* (instance methods +
    /// class-side methods), for computing cross-file override families in
    /// method rename.  Spans aren't stored — the server re-analyses each
    /// family member's document to collect the precise decl / call sites.
    pub defined_methods: Vec<String>,
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
    /// Scope-resolved qualified name when the analyser computed
    /// one, else `None`.
    pub resolved_qualified_name: Option<String>,
    /// The full ordered command-resolution candidate list for this call
    /// (caller namespace, then each `namespace path` entry, then global).
    /// Run through the workspace-wide existence oracle to settle a call the
    /// single analysing file could not — see [`WorkspaceIndex::invocations_of`].
    pub resolution_candidates: Vec<String>,
    /// Byte span of the command-head token in `uri`'s source.
    pub range: Span,
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

/// Cross-document aggregate of proc / class definitions,
/// command-invocation sites, `source` references, and
/// `package require` declarations.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceIndex {
    procs: Vec<WorkspaceProc>,
    classes: Vec<WorkspaceClass>,
    invocations: Vec<WorkspaceInvocation>,
    sources: Vec<WorkspaceSource>,
    package_requires: Vec<WorkspacePackageRequire>,
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

    /// Add (or refresh) one document's proc / class definitions.
    ///
    /// Call [`Self::remove_document`] first when re-indexing a
    /// changed document to avoid stale duplicates.
    pub fn add_document(&mut self, uri: &str, analysis: &AnalysisResult) {
        for proc_def in analysis.all_procs.values() {
            self.procs.push(WorkspaceProc {
                uri: uri.to_owned(),
                name: proc_def.name.clone(),
                qualified_name: proc_def.qualified_name.clone(),
                param_count: proc_def.params.len(),
                name_span: proc_def.name_span,
            });
        }
        for class_def in analysis.all_classes.values() {
            self.classes.push(WorkspaceClass {
                uri: uri.to_owned(),
                name: class_def.name.clone(),
                qualified_name: class_def.qualified_name.clone(),
                name_span: class_def.name_span,
                superclasses: class_def.superclasses.clone(),
                mixins: class_def.mixins.clone(),
                defined_methods: class_def
                    .methods
                    .keys()
                    .chain(class_def.class_methods.keys())
                    .cloned()
                    .collect(),
            });
        }
        for inv in &analysis.command_invocations {
            self.invocations.push(WorkspaceInvocation {
                uri: uri.to_owned(),
                name: inv.name.clone(),
                resolved_qualified_name: inv.resolved_qualified_name.clone(),
                resolution_candidates: inv.resolution_candidates.clone(),
                range: inv.range,
            });
        }
        for target in &analysis.source_targets {
            self.sources.push(WorkspaceSource {
                uri: uri.to_owned(),
                raw_path: target.raw_path.clone(),
                range: target.range,
                is_literal: target.is_literal,
            });
        }
        for pr in &analysis.package_requires {
            self.package_requires.push(WorkspacePackageRequire {
                uri: uri.to_owned(),
                name: pr.name.clone(),
            });
        }
    }

    /// Drop every entry that came from `uri` (used before
    /// re-indexing a changed document, or on `did_close`).
    pub fn remove_document(&mut self, uri: &str) {
        self.procs.retain(|p| p.uri != uri);
        self.classes.retain(|c| c.uri != uri);
        self.invocations.retain(|i| i.uri != uri);
        self.sources.retain(|s| s.uri != uri);
        self.package_requires.retain(|pr| pr.uri != uri);
    }

    /// Every indexed `source FILE` reference.
    #[must_use]
    pub fn sources(&self) -> &[WorkspaceSource] {
        &self.sources
    }

    /// Every indexed `package require NAME` declaration.
    #[must_use]
    pub fn package_requires(&self) -> &[WorkspacePackageRequire] {
        &self.package_requires
    }

    /// The package names `uri` `package require`s, de-duplicated. Used to seed
    /// the workspace W120 refinement from an explicitly configured project
    /// entry file.
    #[must_use]
    pub fn package_requires_for(&self, uri: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .package_requires
            .iter()
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
            .sources
            .iter()
            .filter(|s| s.is_literal)
            .filter_map(|s| resolve(&s.uri, &s.raw_path).map(|child| (s.uri.clone(), child)))
            .collect();
        let mut requires: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for pr in &self.package_requires {
            requires
                .entry(pr.uri.clone())
                .or_default()
                .push(pr.name.clone());
        }
        crate::source_graph::ancestor_requires(target_uri, &edges, &requires)
    }

    /// Every indexed proc.
    #[must_use]
    pub fn procs(&self) -> &[WorkspaceProc] {
        &self.procs
    }

    /// Every indexed class.
    #[must_use]
    pub fn classes(&self) -> &[WorkspaceClass] {
        &self.classes
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
        self.classes
            .iter()
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
        let known: std::collections::HashSet<&str> = self
            .classes
            .iter()
            .map(|c| c.qualified_name.as_str())
            .collect();
        let tail_index = build_tail_index(self.classes.iter().map(|c| &c.qualified_name));
        (known, tail_index)
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
            out.extend(self.classes.iter().filter(|c| c.qualified_name == q));
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
        self.classes
            .iter()
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
        self.classes
            .iter()
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
        let parents = |qname: &str| -> Vec<String> {
            self.classes
                .iter()
                .find(|c| c.qualified_name == qname)
                .map(|c| {
                    c.superclasses
                        .iter()
                        .chain(c.mixins.iter())
                        .filter_map(|s| {
                            resolve_class_name(s, qname, |cand| known.contains(cand), &tail_index)
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let defines = |qname: &str| {
            self.classes
                .iter()
                .any(|c| c.qualified_name == qname && c.defined_methods.iter().any(|m| m == method))
        };
        self.classes
            .iter()
            .filter(|c| {
                // A definer is handled by the family itself, not here.
                if c.defined_methods.iter().any(|m| m == method)
                    || family_set.contains(c.qualified_name.as_str())
                {
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
        // Owner-aware direct parents (superclasses + mixins) of each class.
        let parents = |qname: &str| -> Vec<String> {
            self.classes
                .iter()
                .find(|c| c.qualified_name == qname)
                .map(|c| {
                    c.superclasses
                        .iter()
                        .chain(c.mixins.iter())
                        .filter_map(|s| {
                            resolve_class_name(s, qname, |cand| known.contains(cand), &tail_index)
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
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
        let class_defines = |qname: &str| {
            self.classes
                .iter()
                .any(|c| c.qualified_name == qname && c.defined_methods.iter().any(|m| m == method))
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
                .classes
                .iter()
                .filter(|c| c.defined_methods.iter().any(|m| m == method))
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
        self.procs
            .iter()
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
        self.procs
            .iter()
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
        self.classes
            .iter()
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
        self.procs
            .iter()
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
        self.classes
            .iter()
            .filter(|c| c.uri != exclude_uri)
            .filter(|c| c.qualified_name.trim_start_matches("::") == target)
            .collect()
    }

    /// Whether some proc or class *other than* the target `qualified_name`
    /// shares the `simple_name` under a different namespace anywhere in the
    /// workspace. When true, a bare simple-name call site is ambiguous and
    /// cross-document rename must not rewrite it by simple name alone.
    #[must_use]
    fn simple_name_defined_elsewhere(&self, simple_name: &str, qualified_name: &str) -> bool {
        let target = qualified_name.trim_start_matches("::");
        let differs =
            |name: &str, qn: &str| name == simple_name && qn.trim_start_matches("::") != target;
        self.procs
            .iter()
            .any(|p| differs(&p.name, &p.qualified_name))
            || self
                .classes
                .iter()
                .any(|c| differs(&c.name, &c.qualified_name))
    }

    /// Every indexed invocation site.
    #[must_use]
    pub fn invocations(&self) -> &[WorkspaceInvocation] {
        &self.invocations
    }

    /// The distinct set of document URIs the index currently holds
    /// (across procs, classes, and invocation sites).  Lets the
    /// server reach indexed-but-unopened files for cross-document
    /// passes that need each document's source (e.g. incoming call
    /// hierarchy).
    #[must_use]
    pub fn document_uris(&self) -> Vec<String> {
        let mut uris: Vec<String> = self
            .procs
            .iter()
            .map(|p| p.uri.clone())
            .chain(self.classes.iter().map(|c| c.uri.clone()))
            .chain(self.invocations.iter().map(|i| i.uri.clone()))
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
    /// oracle: its recorded [`resolution_candidates`](WorkspaceInvocation::resolution_candidates)
    /// (caller namespace, then each `namespace path` entry, then global — Tcl's
    /// real priority order) are walked, and the first that names a proc/class
    /// defined *anywhere in the workspace* is the call's true target.  A call is
    /// a reference iff that target is `qualified_name`.  This is the canonical
    /// resolver (`resolve_command_with`) widened from one file to the whole
    /// project, so a bare call reaching a namespaced proc in another file via
    /// `namespace path` resolves correctly — the case the file-local
    /// `resolved_qualified_name` guess (and the previous textual matcher) could
    /// not settle, and the collision on the simple name no longer disables a
    /// legitimate reference.
    ///
    /// Fallbacks preserve the earlier behaviour where the oracle can't help: an
    /// exact literal match (`::a::helper` written verbatim), and the recorded
    /// `resolved_qualified_name` when no candidate is defined anywhere in the
    /// workspace (an as-yet-undefined cross-file target — treated as a
    /// candidate, never ground truth).
    #[must_use]
    pub fn invocations_of<'a>(
        &'a self,
        simple_name: &str,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceInvocation> {
        let qname_no_prefix = qualified_name.strip_prefix("::").unwrap_or(qualified_name);
        let target_norm = qualified_name.trim_start_matches("::");
        // Build the workspace command set once (normalised qualified names of
        // every proc and class), so the per-candidate existence check below is
        // O(1) rather than a scan per candidate per call site.
        let defined = self.defined_command_names();
        // A bare simple-name call is safe to match only when no *other*
        // namespace defines the same simple name (the ambiguity gate); computed
        // once for the background-scan fallback below.
        let bare_is_safe = !self.simple_name_defined_elsewhere(simple_name, qualified_name);
        self.invocations
            .iter()
            .filter(|i| i.uri != exclude_uri)
            .filter(|i| {
                // 1. Exact literal spelling of the qualified name.
                if i.name == qualified_name || i.name == qname_no_prefix {
                    return true;
                }
                // 2. Resolve the call against the workspace: the first candidate
                //    defined anywhere in the project is its true target.
                if let Some(winner) = i
                    .resolution_candidates
                    .iter()
                    .find(|c| defined.contains(c.trim_start_matches("::")))
                {
                    return winner.trim_start_matches("::") == target_norm;
                }
                // 3. A candidate list exists but none is defined workspace-wide
                //    (target not yet indexed): fall back to the settled
                //    local-first guess, a candidate rather than ground truth.
                if !i.resolution_candidates.is_empty() {
                    return i
                        .resolved_qualified_name
                        .as_deref()
                        .is_some_and(|r| r.trim_start_matches("::") == target_norm);
                }
                // 4. No candidate list at all (a background-scanned file skips
                //    the scope walk): fall back to the resolved guess, then to a
                //    bare simple-name match gated on the simple name being
                //    unambiguous across the workspace.
                i.resolved_qualified_name
                    .as_deref()
                    .is_some_and(|r| r.trim_start_matches("::") == target_norm)
                    || (bare_is_safe && i.name == simple_name)
            })
            .collect()
    }

    /// Whether a proc or class with fully-qualified `qualified_name` (leading
    /// `::` ignored) is defined anywhere in the workspace — the existence oracle
    /// that widens the single-file command resolver to the whole project.
    #[must_use]
    pub fn workspace_command_exists(&self, qualified_name: &str) -> bool {
        let target = qualified_name.trim_start_matches("::");
        self.procs
            .iter()
            .any(|p| p.qualified_name.trim_start_matches("::") == target)
            || self
                .classes
                .iter()
                .any(|c| c.qualified_name.trim_start_matches("::") == target)
    }

    /// The set of `::`-stripped qualified names of every indexed proc and class,
    /// for O(1) membership in the candidate-resolution loop of
    /// [`Self::invocations_of`].
    fn defined_command_names(&self) -> std::collections::HashSet<&str> {
        self.procs
            .iter()
            .map(|p| p.qualified_name.trim_start_matches("::"))
            .chain(
                self.classes
                    .iter()
                    .map(|c| c.qualified_name.trim_start_matches("::")),
            )
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
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
            .classes
            .iter()
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
        assert_eq!(index.procs().len(), 3);
        // Param counts captured.
        let greet = index.procs().iter().find(|p| p.name == "greet").unwrap();
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
        assert_eq!(index.procs().len(), 2);
        index.remove_document("file:///a.tcl");
        assert_eq!(index.procs().len(), 1);
        assert_eq!(index.procs()[0].name, "b");
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
        let calls = index.invocations_of("helper", "::helper", "file:///a.tcl");
        assert_eq!(calls.len(), 2, "{calls:?}");
        assert!(calls.iter().all(|c| c.uri == "file:///b.tcl"));
    }

    #[test]
    fn invocations_of_excludes_current_doc() {
        let a = analyse("proc helper {} {}\nhelper\n");
        let b = analyse("helper\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        // Excluding a.tcl leaves only b.tcl's call.
        let calls = index.invocations_of("helper", "::helper", "file:///a.tcl");
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
        let calls = index.invocations_of("helper", "::ns::helper", "file:///a.tcl");
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
        let refs = index.invocations_of("helper", "::mymod::helper", "file:///mymod.tcl");
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
        let refs = index.invocations_of("helper", "::other::helper", "file:///other.tcl");
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
        let refs = index.invocations_of("helper", "::mymod::helper", "file:///mymod.tcl");
        assert!(refs.is_empty(), "{refs:?}");
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
    fn remove_document_drops_invocations_too() {
        let a = analyse("helper\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        assert!(!index.invocations().is_empty());
        index.remove_document("file:///a.tcl");
        assert!(index.invocations().is_empty());
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
        assert!(index.package_requires().is_empty());
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
}
