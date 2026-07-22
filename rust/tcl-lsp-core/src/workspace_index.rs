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
    /// Methods this record explicitly `export`s (an `oo::define` extension
    /// stub can flip visibility on a class defined elsewhere).
    pub exports: Vec<String>,
    /// Methods this record explicitly `unexport`s.
    pub unexports: Vec<String>,
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

    /// The typed record for the *instance-receiver* method `name`
    /// (an ordinary `method` or a `forward`), if this record defines one.
    #[must_use]
    pub fn instance_method(&self, name: &str) -> Option<&WorkspaceMethod> {
        self.methods
            .iter()
            .find(|m| m.name == name && m.kind != "classmethod")
    }
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
}

/// One `namespace export` declaration recorded in the index.
///
/// Aggregated workspace-wide (unlike
/// [`tcl_compiler::analyser::AnalysisResult::namespace_exports`], which is
/// per-document) so a wildcard import in one file can be checked against an
/// export declared in *another* file — the cross-document half of issue
/// #923 idx 18.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceNamespaceExport {
    /// Document the `namespace export` is in.
    pub uri: String,
    /// Exporting namespace, with leading `::`.
    pub ns: String,
    /// Exported pattern text, exactly as written (relative to `ns`).
    pub pattern: String,
}

/// Cross-document aggregate of proc / class definitions,
/// command-invocation sites, `source` references, command
/// name-links, and `package require` declarations.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceIndex {
    procs: Vec<WorkspaceProc>,
    classes: Vec<WorkspaceClass>,
    invocations: Vec<WorkspaceInvocation>,
    sources: Vec<WorkspaceSource>,
    package_requires: Vec<WorkspacePackageRequire>,
    command_links: Vec<WorkspaceCommandLink>,
    glob_imports: Vec<WorkspaceGlobImport>,
    namespace_exports: Vec<WorkspaceNamespaceExport>,
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
                nested: analysis.offset_is_inside_any_definition_body(proc_def.name_span.start()),
            });
        }
        for class_def in analysis.all_classes.values() {
            let methods: Vec<WorkspaceMethod> = class_def
                .methods
                .values()
                .map(|m| WorkspaceMethod {
                    name: m.name.clone(),
                    kind: m.kind.clone(),
                    exported: m.visibility == "public",
                    private: m.visibility == "private",
                })
                .chain(class_def.class_methods.values().map(|m| WorkspaceMethod {
                    name: m.name.clone(),
                    kind: "classmethod".to_string(),
                    exported: m.visibility == "public",
                    private: m.visibility == "private",
                }))
                .collect();
            self.classes.push(WorkspaceClass {
                uri: uri.to_owned(),
                name: class_def.name.clone(),
                qualified_name: class_def.qualified_name.clone(),
                name_span: class_def.name_span,
                superclasses: class_def.superclasses.clone(),
                mixins: class_def.mixins.clone(),
                methods,
                exports: class_def.exports.iter().cloned().collect(),
                unexports: class_def.unexports.iter().cloned().collect(),
                via_define: class_def.via_define,
                metaclass: class_def.metaclass.clone(),
            });
        }
        for inv in &analysis.command_invocations {
            self.invocations.push(WorkspaceInvocation {
                uri: uri.to_owned(),
                name: inv.name.clone(),
                resolution_candidates: inv.resolution_candidates.clone(),
                range: inv.range,
                indirect: inv.indirect,
                rename_safe: inv.rename_safe,
            });
        }
        for target in &analysis.source_targets {
            self.sources.push(WorkspaceSource {
                uri: uri.to_owned(),
                raw_path: target.raw_path.clone(),
                site_namespace: target.site_namespace.clone(),
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
        for exp in &analysis.namespace_exports {
            self.namespace_exports.push(WorkspaceNamespaceExport {
                uri: uri.to_owned(),
                ns: exp.ns.clone(),
                pattern: exp.pattern.clone(),
            });
        }
        self.index_command_links(uri, analysis);
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
                if let Some((source_ns, tail_pattern)) = imp.pattern.rsplit_once("::")
                    && !source_ns.is_empty()
                {
                    self.glob_imports.push(WorkspaceGlobImport {
                        uri: uri.to_owned(),
                        ns: imp.ns.clone(),
                        source_ns: source_ns.to_owned(),
                        tail_pattern: tail_pattern.to_owned(),
                    });
                }
                continue;
            }
            let Some(tail) = imp.pattern.rsplit("::").find(|s| !s.is_empty()) else {
                continue;
            };
            self.command_links.push(WorkspaceCommandLink {
                uri: uri.to_owned(),
                linked_qname: tcl_syntax::naming::qualify(&imp.ns, tail),
                target_qname: normalise_qualified_name(&imp.pattern),
                target_span: Some(imp.range),
                nested: analysis.offset_is_inside_any_definition_body(imp.range.start()),
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
        self.command_links.retain(|l| l.uri != uri);
        self.glob_imports.retain(|g| g.uri != uri);
        self.namespace_exports.retain(|e| e.uri != uri);
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
        for src in &self.sources {
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
        !self.sources.is_empty()
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

    /// Every indexed `namespace import` / `interp alias` / `rename` link.
    #[must_use]
    pub fn command_links(&self) -> &[WorkspaceCommandLink] {
        &self.command_links
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
        for c in self.classes.iter().filter(|c| c.qualified_name == qname) {
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
        for c in &self.classes {
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
        let mut out: Vec<&WorkspaceClass> = Vec::new();
        for class_q in self.class_linearisation(receiver_class) {
            let records: Vec<&WorkspaceClass> = self
                .classes
                .iter()
                .filter(|c| c.qualified_name == class_q)
                .collect();
            // The class-level effective export union: any record exporting
            // the name keeps it callable; explicit unexports matter only
            // when no record exports it.
            let any_exports = records
                .iter()
                .any(|c| c.exports.iter().any(|e| e == method));
            let any_unexports = records
                .iter()
                .any(|c| c.unexports.iter().any(|e| e == method));
            for record in records {
                let Some(m) = record.instance_method(method) else {
                    continue;
                };
                // Effective export across the class's records: an explicit
                // `export` anywhere wins, else an explicit `unexport`
                // anywhere, else the definer's own effective state.  (True
                // cross-file order is load-order; explicit-export-wins is
                // the navigation-permissive reading.)
                let exported = if any_exports {
                    true
                } else if any_unexports {
                    false
                } else {
                    m.exported
                };
                let visible = match access {
                    MethodAccess::External => exported && !m.private,
                    MethodAccess::Internal => !m.private || class_q == receiver_class,
                };
                if visible {
                    out.push(record);
                }
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
        let parents = |qname: &str| self.resolved_parents_of(qname, &known, &tail_index);
        let defines = |qname: &str| {
            self.classes
                .iter()
                .any(|c| c.qualified_name == qname && c.defines_method(method))
        };
        self.classes
            .iter()
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
        let class_defines = |qname: &str| {
            self.classes
                .iter()
                .any(|c| c.qualified_name == qname && c.defines_method(method))
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
                .filter(|c| c.defines_method(method))
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
    fn invocations_settling_to<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
        follow_links: bool,
    ) -> Vec<&'a WorkspaceInvocation> {
        let target = qualified_name.trim_start_matches("::");
        // Build the workspace command set once (normalised qualified names of
        // every proc and class, plus linked names when following), so each
        // candidate existence check is O(1). Same reasoning for the
        // wildcard-import index — see `WildcardImportIndex`'s own doc.
        let defined = self.defined_command_names(follow_links);
        let links = follow_links.then(|| self.command_link_map());
        let wci = WildcardImportIndex::build(self);
        self.invocations
            .iter()
            .filter(|i| i.uri != exclude_uri)
            .filter(|i| self.invocation_resolves_to(i, &defined, links.as_ref(), &wci, target))
            .collect()
    }

    /// Whether call site `inv` resolves to the command whose `::`-stripped
    /// qualified name is `target`: the first of its candidates defined anywhere
    /// in the workspace is the call's true target, chased along `links` (when
    /// supplied) to the command it ultimately names.
    fn invocation_resolves_to(
        &self,
        inv: &WorkspaceInvocation,
        defined: &std::collections::HashSet<&str>,
        links: Option<&std::collections::HashMap<&str, &str>>,
        wci: &WildcardImportIndex<'_>,
        target: &str,
    ) -> bool {
        if let Some(winner) = inv
            .resolution_candidates
            .iter()
            .find(|c| defined.contains(c.trim_start_matches("::")))
        {
            let winner = winner.trim_start_matches("::");
            let settled = links.map_or(winner, |m| Self::follow_links(m, winner));
            return settled == target;
        }
        // No real command or name-link settled this call — try a wildcard
        // `namespace import NS::*` in scope for the call's own namespace
        // (issue #923 idx 18). Only when following links: this mirrors an
        // exact import's `WorkspaceCommandLink`, which likewise only
        // participates in the *linked* view (`linked_invocations_of`, used
        // by find-references) and never the direct-only view rename relies
        // on — a call spelling the local imported name is not text-rewritten
        // just because its ultimate source is renamed.
        links.is_some()
            && self
                .resolve_wildcard_import_indexed(&inv.name, &inv.resolution_candidates, wci)
                .is_some_and(|resolved| resolved.trim_start_matches("::") == target)
    }

    /// The command name-link map (`::`-stripped `linked → immediate target`)
    /// used to chase an import / alias / rename to the command it names.
    fn command_link_map(&self) -> std::collections::HashMap<&str, &str> {
        self.command_links
            .iter()
            .map(|l| {
                (
                    l.linked_qname.trim_start_matches("::"),
                    l.target_qname.trim_start_matches("::"),
                )
            })
            .collect()
    }

    /// Chase `start` along the link map to its ultimate target, stopping at a
    /// name that is not itself a linked name.  Bounded by cycle detection (an
    /// alias-of-an-alias loop) so a malformed chain cannot spin.
    fn follow_links<'a>(
        links: &std::collections::HashMap<&'a str, &'a str>,
        start: &'a str,
    ) -> &'a str {
        let mut cur = start;
        let mut seen = std::collections::HashSet::new();
        while let Some(&next) = links.get(cur) {
            if !seen.insert(cur) {
                break;
            }
            cur = next;
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
        self.command_links
            .iter()
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
        self.classes
            .iter()
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
        self.invocations
            .iter()
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
        self.procs
            .iter()
            .any(|p| !p.nested && p.qualified_name.trim_start_matches("::") == target)
            || self
                .classes
                .iter()
                .any(|c| c.qualified_name.trim_start_matches("::") == target)
            || self
                .command_links
                .iter()
                .any(|l| !l.nested && l.linked_qname.trim_start_matches("::") == target)
    }

    /// The set of `::`-stripped qualified names of every indexed proc and
    /// class, for O(1) membership in the candidate-resolution loop of
    /// [`Self::invocations_of`].  With `include_links`, the names an import /
    /// alias / rename introduces join the set, so a call reaching one of them
    /// settles (and is then chased to its ultimate target).
    fn defined_command_names(&self, include_links: bool) -> std::collections::HashSet<&str> {
        let mut names: std::collections::HashSet<&str> = self
            .procs
            .iter()
            .map(|p| p.qualified_name.trim_start_matches("::"))
            .chain(
                self.classes
                    .iter()
                    .map(|c| c.qualified_name.trim_start_matches("::")),
            )
            .collect();
        if include_links {
            names.extend(
                self.command_links
                    .iter()
                    .map(|l| l.linked_qname.trim_start_matches("::")),
            );
        }
        names
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
    ) -> Option<String> {
        self.resolve_wildcard_import_indexed(
            word,
            resolution_candidates,
            &WildcardImportIndex::build(self),
        )
    }

    /// The indexed core of [`Self::resolve_wildcard_import`], taking a
    /// precomputed [`WildcardImportIndex`] instead of scanning
    /// `glob_imports`/`namespace_exports` in full — the fast path
    /// [`Self::invocation_resolves_to`] uses so a workspace-wide
    /// invocation-settling pass builds the index once (O(every glob import
    /// / export in the workspace)) rather than once per invocation (which
    /// would be O(invocation count × workspace-wide glob-import count) —
    /// measurably regressed find-references latency on a codebase using
    /// `namespace import NS::*` in more than a handful of files).
    fn resolve_wildcard_import_indexed(
        &self,
        word: &str,
        resolution_candidates: &[String],
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
            let Some(imports) = wci.imports_by_ns.get(candidate_ns) else {
                continue;
            };
            for imp in imports {
                if !tcl_syntax::glob::string_match(&imp.tail_pattern, word) {
                    continue;
                }
                if !wci.exports_name(&imp.source_ns, word) {
                    continue;
                }
                let target = format!("{}::{word}", imp.source_ns);
                if self.workspace_command_exists(&target) {
                    return Some(target);
                }
            }
        }
        None
    }
}

/// Precomputed per-namespace grouping of [`WorkspaceIndex::glob_imports`]
/// and [`WorkspaceIndex::namespace_exports`], built once per query rather
/// than re-scanned per call site — see
/// [`WorkspaceIndex::resolve_wildcard_import_indexed`]'s doc for why.
struct WildcardImportIndex<'a> {
    imports_by_ns: std::collections::HashMap<&'a str, Vec<&'a WorkspaceGlobImport>>,
    exports_by_ns: std::collections::HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> WildcardImportIndex<'a> {
    fn build(index: &'a WorkspaceIndex) -> Self {
        let mut imports_by_ns: std::collections::HashMap<&str, Vec<&WorkspaceGlobImport>> =
            std::collections::HashMap::new();
        for imp in &index.glob_imports {
            imports_by_ns.entry(imp.ns.as_str()).or_default().push(imp);
        }
        let mut exports_by_ns: std::collections::HashMap<&str, Vec<&str>> =
            std::collections::HashMap::new();
        for exp in &index.namespace_exports {
            exports_by_ns
                .entry(exp.ns.as_str())
                .or_default()
                .push(exp.pattern.as_str());
        }
        Self {
            imports_by_ns,
            exports_by_ns,
        }
    }

    /// Whether `ns` (`::`-rooted) has recorded a `namespace export` pattern
    /// that covers the unqualified `name` — the cross-document half of the
    /// wildcard-import gate (issue #923 idx 18): real Tcl only imports names
    /// a source namespace has actually exported (`Tcl_Export`,
    /// `tclNamesp.c`).
    fn exports_name(&self, ns: &str, name: &str) -> bool {
        self.exports_by_ns.get(ns).is_some_and(|patterns| {
            patterns
                .iter()
                .any(|p| tcl_syntax::glob::string_match(p, name))
        })
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
        let resolved = index.resolve_wildcard_import("Widget", &["::Widget".to_string()]);
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
        );
        assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
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
            .iter()
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
            .iter()
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
            .iter()
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
            .iter()
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
