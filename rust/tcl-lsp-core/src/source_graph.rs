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

//! Workspace `source` graph helpers.
//!
//! A multi-file Tcl project commonly has an *entry* file that runs the
//! `package require`s and then `source`s its individual modules.  Those
//! modules use the required packages' commands without a `package require`
//! of their own, which the single-file W120 check would flag as missing (see
//! [`crate::package_resolver`] and the analyser's
//! `emit_missing_package_require_diagnostics`).
//!
//! To resolve that, the server builds a workspace-wide `source` graph from the
//! `source FILE` statements the analyser records per document, then lets a
//! module inherit the `package require`s of every file that (transitively)
//! `source`s it.  This module owns the two pure, filesystem-free pieces of that
//! resolution: the lexical path resolution ([`resolve_source_target`]) and the
//! reverse-reachability walk ([`ancestor_requires`]).  The server supplies the
//! URI ↔ path conversion, keeping this logic testable without any real files.
//!
//! # The two directions, and why both exist
//!
//! `source` is a two-way channel of facts, and the two directions answer
//! different questions:
//!
//! * **Down** ([`ancestor_requires`]) — *what did my callers
//!   already load before they ran me?*  A module `source`d by an entry file
//!   that required `Tk` may use `winfo` with no `package require` of its own.
//! * **Up** ([`descendant_requires`]) — *what did the files I
//!   `source` load on my behalf?*  `source tkFile.tcl` where `tkFile.tcl` does
//!   `package require Tk` makes `Tk` present in the sourcing file from that
//!   statement onward, exactly as if the `package require` had been written
//!   inline.  Verified against C Tcl 9.0.4: a `package present` before the
//!   `source` statement fails and after it succeeds, which is why the up
//!   direction is *placed* (carries the statement offset) while the down
//!   direction is not — an ancestor ran its whole prologue before entering us,
//!   so there is no position in *our* text to gate on.

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// Resolve a literal `source` path argument written in `parent_file` to the
/// absolute path it refers to.
///
/// This project association resolves a relative `raw_path` against the
/// directory of `parent_file`; an absolute path passes through unchanged.
/// Tcl itself resolves relative `source` paths against its runtime working
/// directory, which this advisory graph does not establish.
/// The result is lexically normalised —
/// `.` and `..` segments are folded without touching the filesystem, so a
/// symlink is never followed and the call is cheap and deterministic.
#[must_use]
pub fn resolve_source_target(parent_file: &Path, raw_path: &str) -> PathBuf {
    resolve_under(
        parent_file.parent().unwrap_or_else(|| Path::new("")),
        raw_path,
    )
}

/// Associate a genuine original `SourceLoad` path under the declared project
/// script-directory policy. This is neither a reached load nor runtime cwd.
/// Dynamic paths and unsupported host-path forms preserve unknown topology.
#[must_use]
pub fn conditional_project_source_target(
    document: &Path,
    load: &tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad,
) -> Option<PathBuf> {
    let literal = load.literal_path()?;
    if literal.is_empty()
        || literal.starts_with('~')
        || literal.contains('\0')
        || !project_source_action_path(document)
    {
        return None;
    }
    let raw = Path::new(&literal);
    if host_absolute(raw) && !project_source_action_path(raw)
        || !host_absolute(raw) && matches!(raw.components().next(), Some(Component::Prefix(_)))
    {
        return None;
    }
    let target = resolve_source_target(document, &literal);
    project_source_action_path(&target).then_some(target)
}

/// Resolve `raw_path` relative to the directory `base_dir` (absolute paths pass
/// through), lexically normalised.  Used both for a `source` target (relative
/// to the sourcing file's directory) and for a configured project entry-point
/// path (relative to the workspace folder root).
#[must_use]
pub fn resolve_under(base_dir: &Path, raw_path: &str) -> PathBuf {
    let raw = Path::new(raw_path);
    let joined = if host_absolute(raw) {
        raw.to_path_buf()
    } else {
        base_dir.join(raw)
    };
    lexically_normalise(&joined)
}

/// Fold `.` and `..` segments lexically (no filesystem access). A leading `..`
/// with nothing to pop is preserved so a path that escapes its root still
/// resolves to a stable, comparable form.
fn lexically_normalise(path: &Path) -> PathBuf {
    let mut out: Vec<Component<'_>> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => match out.last() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                _ => out.push(comp),
            },
            other => out.push(other),
        }
    }
    if out.is_empty() {
        return PathBuf::from(".");
    }
    out.iter().collect()
}

/// The unique declared project base associating a literal source reference
/// with a host rename. This is an intentional source-action policy, not an
/// observation of Tcl's working directory, file loading or normal completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSourceAssociation {
    base: Option<PathBuf>,
}

impl ProjectSourceAssociation {
    /// Preserve absolute/relative style under this same declared project base.
    #[must_use]
    pub fn renamed_literal(&self, new_path: &Path) -> Option<String> {
        if !project_source_action_path(new_path) {
            return None;
        }
        let target = lexically_normalise(new_path);
        let value = match &self.base {
            None => target,
            Some(base) => relative_source_path(&target, base)?,
        };
        let text = value.to_str()?;
        #[cfg(windows)]
        let text = text.replace('\\', "/");
        Some(text.to_owned())
    }
}

/// Associate a whole literal FILE operand under the explicitly declared
/// script-directory/workspace-root project policy. Multiple distinct matching
/// bases refuse; candidate filesystem existence is never inferred here.
#[must_use]
pub fn project_source_association(
    document: &Path,
    workspace_roots: &[PathBuf],
    literal: &str,
    old_path: &Path,
) -> Option<ProjectSourceAssociation> {
    // Home expansion and drive-relative bases have no project-path issuer.
    if literal.is_empty()
        || literal.starts_with('~')
        || literal.contains('\0')
        || !project_source_action_path(document)
        || !project_source_action_path(old_path)
    {
        return None;
    }
    let old = lexically_normalise(old_path);
    let raw = Path::new(literal);
    if host_absolute(raw) {
        project_source_action_path(raw).then_some(())?;
        return (lexically_normalise(raw) == old)
            .then_some(ProjectSourceAssociation { base: None });
    }
    if matches!(raw.components().next(), Some(Component::Prefix(_))) {
        return None;
    }
    let mut bases = Vec::new();
    for base in
        std::iter::once(document.parent()?).chain(workspace_roots.iter().map(PathBuf::as_path))
    {
        if !project_source_action_path(base) {
            return None;
        }
        let base = lexically_normalise(base);
        if resolve_under(&base, literal) == old && !bases.contains(&base) {
            bases.push(base);
        }
    }
    let [base] = bases.as_slice() else {
        return None;
    };
    Some(ProjectSourceAssociation {
        base: Some(base.clone()),
    })
}

/// `Path::is_absolute`, plus a rooted path in a browser worker
/// (wasm32-unknown-unknown), where std reports no path as absolute and every
/// source-store path is a rooted virtual path.
fn host_absolute(path: &Path) -> bool {
    path.is_absolute()
        || cfg!(all(target_family = "wasm", target_os = "unknown")) && path.has_root()
}

fn project_source_action_path(path: &Path) -> bool {
    // POSIX double-leading slash may select an implementation-defined root.
    host_absolute(path)
        && path
            .to_str()
            .is_some_and(|text| !(text.contains('\0') || cfg!(unix) && text.starts_with("//")))
}

/// Relative spelling in the same host path domain; different roots decline.
#[must_use]
pub fn relative_source_path(target: &Path, base: &Path) -> Option<PathBuf> {
    if !host_absolute(target) || !host_absolute(base) {
        return None;
    }
    let target = lexically_normalise(target);
    let base = lexically_normalise(base);
    let target: Vec<_> = target.components().collect();
    let base: Vec<_> = base.components().collect();
    if target.first() != base.first() {
        return None;
    }
    let common = target
        .iter()
        .zip(&base)
        .take_while(|(left, right)| left == right)
        .count();
    if base[common..]
        .iter()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let mut result = PathBuf::new();
    for _ in &base[common..] {
        result.push("..");
    }
    for part in &target[common..] {
        result.push(part.as_os_str());
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    Some(result)
}

/// The union of `package require` names from every node that transitively
/// reaches `target` through the `source` graph.
///
/// `edges` are `(parent, child)` pairs meaning *`parent` `source`s `child`*;
/// `requires` maps a node to the package names it `package require`s.  The walk
/// follows edges in reverse (child → its parents), so `target` inherits the
/// requires of every ancestor that sources it, directly or transitively.  Node
/// identity is the caller's string key (the server uses document URIs); cycles
/// are handled by a visited set.  The result is sorted and de-duplicated.
#[must_use]
pub fn ancestor_requires<S: BuildHasher>(
    target: &str,
    edges: &[(String, String)],
    requires: &HashMap<String, Vec<String>, S>,
) -> Vec<String> {
    let mut requirements = ancestor_requirements(target, edges, requires);
    requirements.sort();
    requirements
}

/// Traverse source ancestors while preserving each caller-owned requirement
/// key. Graph reachability supplies advisory inheritance, not package loading
/// or source execution. Document identifiers are distinct from Tcl names.
#[must_use]
pub fn ancestor_requirements<K: Clone + Eq + std::hash::Hash, S: BuildHasher>(
    target: &str,
    edges: &[(String, String)],
    requires: &HashMap<String, Vec<K>, S>,
) -> Vec<K> {
    // child -> parents that source it.
    let mut parents_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for (parent, child) in edges {
        parents_of
            .entry(child.as_str())
            .or_default()
            .push(parent.as_str());
    }
    let mut ancestors: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = vec![target];
    let mut visited: HashSet<&str> = HashSet::new();
    while let Some(node) = stack.pop() {
        if !visited.insert(node) {
            continue;
        }
        if let Some(parents) = parents_of.get(node) {
            for &parent in parents {
                ancestors.insert(parent);
                stack.push(parent);
            }
        }
    }
    let mut ancestors = ancestors.into_iter().collect::<Vec<_>>();
    ancestors.sort_unstable();
    let mut seen = HashSet::new();
    let out: Vec<K> = ancestors
        .iter()
        .filter_map(|a| requires.get(*a))
        .flatten()
        .filter(|key| seen.insert((*key).clone()))
        .cloned()
        .collect();
    out
}

/// A package a document acquires by `source`ing another file, together with
/// **where in this document** it becomes available.
///
/// The position is what separates this from a plain package name: `source`
/// runs the child inline at that statement, so a command needing the package
/// is satisfied *after* the `source` and unsatisfied *before* it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedRequirement<K> {
    /// The `package require`d name, as written in the sourced file.
    pub name: K,
    /// Byte offset, **in the document that asked**, of the `source` statement
    /// that brings the package in.
    pub at: u32,
    /// Span of the innermost proc/class body containing that `source`
    /// statement; `None` at load level.  Feeds
    /// [`tcl_compiler::analyser::indirection::in_effect_within`], so a
    /// load-level `source` counts for a call written anywhere inside a proc
    /// body (the whole file loads before any body runs).
    pub enclosing_body: Option<tcl_lexer::Span>,
}

/// Positioned package-name reporting for compatibility consumers.
pub type PlacedRequire = PlacedRequirement<String>;

/// The `package require`s a document acquires from the files it (transitively)
/// `source`s — the **up** direction of the graph.
///
/// `edges` are the workspace's resolved `source` edges; only
/// [`RunEdgeKind::Source`] edges are followed, because only a `source` inlines
/// the child's body into the parent's run at a known position (a
/// `package require` edge merely bounds when the provider ran, which says
/// nothing about *this* document acquiring it).
///
/// Every package required anywhere below a direct edge `target -> child` is
/// attributed to that edge's position in `target`.  Depth beyond the first hop
/// is **not** re-gated: proving that `child`'s own `source` really executes
/// would need conditional-execution analysis this graph does not have, so the
/// walk takes the whole subtree and the caller treats the result as
/// suppression evidence rather than proof.  That is the deliberate direction
/// to be wrong in here — a missed suppression is a false "requires
/// `package require X`" on code that runs fine.
///
/// Cycles (`a` sources `b` sources `a`) terminate on the visited set, and a
/// file sourced twice simply contributes its packages at both positions.
/// `target`'s *own* `package require`s are not included — those are the
/// single-document question the analyser already answers.
#[must_use]
pub fn descendant_requires<S: BuildHasher>(
    target: &str,
    edges: &[RunEdge],
    requires: &HashMap<String, Vec<String>, S>,
) -> Vec<PlacedRequire> {
    let mut requirements = descendant_requirements(target, edges, requires);
    requirements.sort_by(|a, b| a.name.cmp(&b.name).then(a.at.cmp(&b.at)));
    requirements
}

/// Traverse positioned source descendants without converting the requirement
/// keys. Original byte policies and document-owned metadata remain separate
/// from this graph's advisory execution bounds.
#[must_use]
pub fn descendant_requirements<K: Clone + Eq + std::hash::Hash, S: BuildHasher>(
    target: &str,
    edges: &[RunEdge],
    requires: &HashMap<String, Vec<K>, S>,
) -> Vec<PlacedRequirement<K>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for edge in edges
        .iter()
        .filter(|e| e.parent == target && e.kind == RunEdgeKind::Source && e.child != target)
    {
        // Everything reachable from this direct child, attributed to the
        // position of *this* `source` statement in `target`.
        let mut stack: Vec<&str> = vec![edge.child.as_str()];
        let mut visited: HashSet<&str> = HashSet::new();
        visited.insert(target);
        while let Some(node) = stack.pop() {
            if !visited.insert(node) {
                continue;
            }
            if let Some(names) = requires.get(node) {
                out.extend(
                    names
                        .iter()
                        .filter(|name| seen.insert(((*name).clone(), edge.at, edge.enclosing_body)))
                        .map(|name| PlacedRequirement {
                            name: name.clone(),
                            at: edge.at,
                            enclosing_body: edge.enclosing_body,
                        }),
                );
            }
            for next in edges
                .iter()
                .filter(|e| e.parent == node && e.kind == RunEdgeKind::Source)
            {
                stack.push(next.child.as_str());
            }
        }
    }
    out
}

/// Coordinate rule of a constructed source-order hypothesis. The kind alone
/// authenticates neither original source, file entry nor event completion.
/// Workspace navigation retains those missing premises as Conditional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunEdgeKind {
    /// Candidate source route: the hypothetical child position is this
    /// statement's coordinate. Actual file entry and successful completion
    /// require separate owners; no such premise follows from this variant.
    #[default]
    Source,
    /// `package require P`, with `CHILD` the document that `package provide`s
    /// `P`: the child had loaded **by** this statement, but not necessarily
    /// *at* it.
    ///
    /// `package require` is a load event only for whichever require runs
    /// first; every later one finds the package already provided, returns its
    /// version, and evaluates nothing.  Oracle (byte-identical on tclsh 8.6.14
    /// and 9.0.4) — `a.tcl` and `b.tcl` each hold the same two lines, an
    /// "is it loaded?" probe followed by `package require mylib`, and a driver
    /// sources `a.tcl` then `b.tcl`:
    ///
    /// ```text
    ///   a.tcl: before its own require, is lib loaded? NO
    ///   lib.tcl body running
    ///   b.tcl: before its own require, is lib loaded? YES
    /// ```
    ///
    /// Identical source text, opposite answers, decided by something neither
    /// file says.  So this edge places the child at *at most* its `at`, and
    /// [`RunOrder`] answers only the half that survives the bound: "the child
    /// has already run" is a fact from the require onwards; "the child has not
    /// run yet" is never a fact at all.
    PackageRequire,
}

impl RunEdgeKind {
    /// Whether the coordinate is exact within the constructed order model.
    /// This says nothing about source entry, execution or Normal completion.
    #[must_use]
    pub fn is_exact(self) -> bool {
        matches!(self, Self::Source)
    }
}

/// One constructed source-order edge. Its public geometry carries no original
/// source issuer or executed timeline. The workspace constructor validates its
/// actual source operation/body/input and retains only conditional navigation.
///
/// [`ancestor_requires`] only asks *whether* one file reaches another, so a
/// bare `(parent, child)` pair is enough for it.  An **interpreter-state**
/// question — "had this statement already run by the time the child loaded?"
/// — additionally needs where in the parent the entering statement sits,
/// because a statement written after it has not run yet, and [`Self::kind`],
/// because only a `source` pins the child to that position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunEdge {
    /// The entering document's key.
    pub parent: String,
    /// The entered document's key.
    pub child: String,
    /// Byte offset of the entering statement in `parent`.
    pub at: u32,
    /// Span of the innermost proc/class body containing the entering
    /// statement in `parent`; `None` at load level.
    pub enclosing_body: Option<tcl_lexer::Span>,
    /// Whether `at` is the child's exact position or an upper bound on it.
    pub kind: RunEdgeKind,
}

/// Whether the **interpreter-global** `package prefer latest` latch is already
/// raised by the time `target` is loaded, given the raises recorded per
/// document.
///
/// `package prefer` is a monotone latch on interpreter state, so a raise in a
/// file that runs first really does change a later file's version selection.
/// Which file runs first is not knowable in general — but along the `source`
/// graph it is a static fact: `source CHILD` loads the whole child *at that
/// statement*, so everything the parent already ran is in effect for it, and
/// nothing the parent runs afterwards is.
///
/// `raises[node]` holds the offsets of that document's **unconditional**
/// `package prefer latest` statements (a conditional one may not run, and the
/// caller abstains toward the interpreter default by omitting it).
///
/// Two ways the latch is up when a node is entered:
///
/// * a parent raised it **before** the `source` statement — order-gated with
///   [`tcl_compiler::analyser::indirection::in_effect_within`], so a raise at
///   the parent's load level still counts for a `source` written inside one of
///   its proc bodies (the whole file loads before any body runs);
/// * a parent was **itself** entered with the latch up, in which case it was
///   up for the parent's whole execution and so for every file it sources.
///
/// Cycles terminate on the visited set.  `target`'s *own* raises are not
/// consulted — those are the single-document question
/// (`package_resolver::package_prefer_at`), which is position-sensitive within
/// the document.
#[must_use]
pub fn ancestor_prefer_latest_raised<S: BuildHasher>(
    target: &str,
    edges: &[RunEdge],
    raises: &HashMap<String, Vec<u32>, S>,
) -> bool {
    use tcl_compiler::analyser::indirection::in_effect_within;

    let no_raises: Vec<u32> = Vec::new();
    let raised_before = |node: &str, at: u32, body: Option<tcl_lexer::Span>| {
        raises
            .get(node)
            .unwrap_or(&no_raises)
            .iter()
            .any(|&established| in_effect_within(established, at, body))
    };
    // Seed: every child whose parent had already raised the latch when it
    // sourced them.
    let mut entered_raised: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = Vec::new();
    for edge in edges {
        if raised_before(&edge.parent, edge.at, edge.enclosing_body)
            && entered_raised.insert(edge.child.as_str())
        {
            stack.push(edge.child.as_str());
        }
    }
    // Propagate: a node entered with the latch up passes it to *every* file it
    // sources, wherever the `source` sits — the latch was already up for the
    // node's whole execution.
    while let Some(node) = stack.pop() {
        if node == target {
            return true;
        }
        for edge in edges.iter().filter(|e| e.parent == node) {
            if entered_raised.insert(edge.child.as_str()) {
                stack.push(edge.child.as_str());
            }
        }
    }
    entered_raised.contains(target)
}

/// Supplied source position. Workspace navigation reborrows its original
/// configured command/body owner instead of trusting the supplied body span.
/// This is not an entered execution point or event-completion receipt.
///
/// The three facts every import-lifecycle event and every query point already
/// carries, named once so [`RunOrder`] can take a pair of them rather than six
/// loose arguments.  Within one document the triple is exactly what
/// [`tcl_compiler::analyser::indirection::in_effect_within`] and
/// [`crate::namespace_import::load_order`] read; across documents it is what
/// [`RunOrder`] lifts onto the `source` forest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunPoint<'a> {
    /// Document the statement is written in.
    pub uri: &'a str,
    /// Byte offset of the statement within `uri`.
    pub at: u32,
    /// Span of the innermost proc/class body containing it; `None` at load
    /// level.
    pub enclosing_body: Option<tcl_lexer::Span>,
}

/// A position inside **one** document — what [`RunOrder`] reduces a pair of
/// [`RunPoint`]s to once it has found their deepest common document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    /// Byte offset within the common document.
    pub at: u32,
    /// Enclosing proc/class body of that offset, in the same document.
    pub enclosing_body: Option<tcl_lexer::Span>,
    /// Whether [`Self::at`] is where the point **is**, or only a bound on how
    /// late it can be.
    ///
    /// `true` for a point in its own document and for one reached entirely
    /// through `source` statements; `false` once a
    /// [`RunEdgeKind::PackageRequire`] edge is on the path, where the true
    /// position is *at most* `at`.  A bounded position still
    /// answers half the questions — see [`RunOrder::trusted`].
    pub exact: bool,
}

/// Purpose of a source navigation relation, never executed chronology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceNavigationOrderPurpose {
    /// Explicit constructed-input compatibility model; no source issuer implied.
    Constructed,
    /// Both points are straight-line positions of one indexed document: the
    /// document root, or a body entered once, immediately and in the current
    /// frame from such a position, outside every definition body, before any
    /// possible early exit and with no unrepresented script operand around
    /// the point. The document's own source sequence orders them: when the
    /// later point is reached, the earlier one has run.
    DocumentSequence,
    /// Genuine source geometry with unresolved entry/completion/event effects.
    Conditional,
}

/// A sealed purpose-specific source relation. A conditional relation alone
/// cannot revoke an export/import candidate; only a constructed model or one
/// document's straight-line sequence can.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceNavigationOrder<T> {
    relation: T,
    purpose: SourceNavigationOrderPurpose,
}

impl<T> SourceNavigationOrder<T> {
    /// The relation's original purpose, without completion or execution facts.
    #[must_use]
    pub const fn purpose(&self) -> SourceNavigationOrderPurpose {
        self.purpose
    }

    /// The source relation, without asserting any handler or body executed.
    #[must_use]
    pub const fn conditional_relation(&self) -> &T {
        &self.relation
    }

    /// An explicit constructed-input model, or the straight-line sequence of
    /// one document, supports its bounded exclusion. Cross-document,
    /// deferred-body and conditional-branch geometry, and completed
    /// command-publication worlds, cannot donate namespace event, tombstone or
    /// conflict completion.
    #[must_use]
    pub fn modeled_source_exclusion(self) -> Option<T> {
        matches!(
            self.purpose,
            SourceNavigationOrderPurpose::Constructed
                | SourceNavigationOrderPurpose::DocumentSequence
        )
        .then_some(self.relation)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalSourceOrderSite {
    span: tcl_lexer::Span,
    body: Option<tcl_lexer::Span>,
    source_load:
        Option<tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad>,
    package_require:
        Option<tcl_compiler::registry_invocation::source_structure::OriginalSourcePackageReference>,
    words: tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
}

/// Straight-line source sequence of one document.
///
/// Built from the same readonly command/body inventory and original selected
/// Registry words as the sites. Relations between two positions are
/// reach-conditioned: when the later position is reached, every earlier
/// position of the sequence has run. A position is admitted only when its
/// innermost retained script region is straight and it is not inside an
/// operand that may hold an unrepresented script or inside a definition body.
/// The root region is straight; a body is straight only when a command of a
/// straight region outside every caller-context-unavailable body, with
/// `Selected` words, enters it once and immediately in the current frame
/// (`ScriptBodyFlow::{Sequence, ConcatenatedScript}` without a frame level
/// word) and no document declaration shadows its command word. Deferred
/// bodies, conditional, looping or caught branches, substitutions and frame
/// shifts are never straight.
///
/// Possible early exits: a command whose Registry traits terminate the block
/// or the process, or a straight command containing one outside definition
/// bodies (`if {..} return`), ends the known sequence. Every position after
/// it, and the document end, is withheld.
///
/// The document end means "after the document completes normally"; an error
/// path never reaches it. It is admitted only when no command outside
/// definition bodies and callbacks may end that completion early: none
/// transfers control (`break`, `continue`, `tailcall`), evaluates another
/// file (`source`), runs a script now that is not retained (`apply $l`,
/// `expr $e`), an expression calling a math function (`expr {f(1)}`) or a
/// command prefix (`lsort -command`), or lacks selected
/// Registry words (a user procedure or a dynamic head may
/// `return -code return`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct DocumentSequence {
    end: u32,
    /// First offset a possible early exit may leave unreached.
    cutoff: Option<u32>,
    /// Whether some command outside definition bodies and callbacks may end
    /// the document's normal completion early without making any later
    /// position unreachable unnoticed (see [`Self::record_exits`]); the
    /// document end, read as "after normal completion", is then withheld.
    end_withheld: bool,
    /// Retained script regions, ordered by start and then outermost first.
    regions: Vec<SequenceRegion>,
    /// Definition bodies by start, each with the running maximum end.
    definition_bodies: Vec<(tcl_lexer::Span, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SequenceRegion {
    span: tcl_lexer::Span,
    parent: Option<usize>,
    straight: bool,
    /// A body a command stores for later (a callback), not run during load.
    deferred: bool,
    /// Direct commands of a straight region, by start (they are disjoint).
    commands: Vec<SequenceCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SequenceCommand {
    span: tcl_lexer::Span,
    /// The command word: the command's own position, whatever its arguments.
    head: Option<tcl_lexer::Span>,
    /// Operands whose role may carry a script; `None` admits only the command
    /// word (unselected words, unknown roles, an unrepresented operand or a
    /// command word the document itself declares).
    script_operands: Option<Vec<tcl_lexer::Span>>,
}

#[derive(Debug, Clone, Copy)]
enum ScriptScheduling {
    None,
    Deferred,
    Immediate { once: bool },
}
impl ScriptScheduling {
    fn runs_now(self) -> bool {
        matches!(self, Self::Immediate { .. })
    }
    fn defers(self) -> bool {
        matches!(self, Self::Deferred)
    }
    fn enters_once(self) -> bool {
        matches!(self, Self::Immediate { once: true })
    }
}

/// Registry control facts of one command, for the sequence.
#[derive(Debug, Clone, Copy)]
struct CommandFlow {
    /// Traits end the block or the process (`return`, `error`, `exit`).
    terminates: bool,
    /// Traits transfer control elsewhere (`break`, `continue`, `tailcall`),
    /// or the command evaluates another file (`source`), whose own `return`
    /// ends this document's normal completion.
    may_end_load: bool,
    /// Mutually exclusive immediate/deferred/no-script scheduling.
    scheduling: ScriptScheduling,
    /// The Registry may give it a command prefix to invoke (`lsort -command`,
    /// `regsub -command`), or cannot rule one out.
    invokes_prefix: bool,
}

/// One original Registry selection per command, shared by the sequence and
/// the site inventory.
#[derive(Debug, Clone)]
enum RegistryWordsSlot {
    Pending,
    Resolved(
        Box<Option<tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords>>,
    ),
}

impl RegistryWordsSlot {
    fn get(
        &mut self,
        resolve: impl FnOnce() -> Option<
            tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        >,
    ) -> Option<&tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords> {
        if matches!(self, Self::Pending) {
            *self = Self::Resolved(Box::new(resolve()));
        }
        match self {
            Self::Resolved(words) => (**words).as_ref(),
            Self::Pending => None,
        }
    }

    fn take(
        &mut self,
        resolve: impl FnOnce() -> Option<
            tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        >,
    ) -> Option<tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords> {
        match std::mem::replace(self, Self::Pending) {
            Self::Resolved(words) => *words,
            Self::Pending => resolve(),
        }
    }
}

/// Inputs shared by every step of [`DocumentSequence::capture`].
struct SequenceCapture<'a> {
    analysis: &'a tcl_compiler::analyser::AnalysisResult,
    input: &'a tcl_compiler::analyser::ResolvedAnalysisInput,
    image: &'a tcl_lexer::SourceImage,
    structure: &'a crate::source_structure::SourceStructure,
    declared: rustc_hash::FxHashSet<&'a [u8]>,
}

impl SequenceCapture<'_> {
    fn validated<'w>(
        &self,
        words: Option<
            &'w tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        >,
    ) -> Option<&'w tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords>
    {
        words.filter(|words| {
            words.matches_source(self.image, self.input.lexer_config())
                && words.matches_registry(self.input.borrowed_context_registry().commands())
                && words.context() == Some(self.input.availability_context())
        })
    }

    /// The command's Registry control facts, selected once per command.
    fn flow(
        &self,
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> CommandFlow {
        use tcl_registry::script_body_flow::{ScriptBodyFlow, script_body_flow_for_invocation};
        words
            .with_source_schema(self.input.borrowed_context_registry(), |schema| {
                let flow =
                    script_body_flow_for_invocation(&schema.facts(), schema.words.arguments());
                CommandFlow {
                    terminates: schema.semantics.traits.intersects(
                        tcl_registry::Traits::TERMINATES_BLOCK
                            | tcl_registry::Traits::TERMINATES_PROCESS,
                    ),
                    may_end_load: schema
                        .semantics
                        .traits
                        .contains(tcl_registry::Traits::TRANSFERS_CONTROL)
                        || schema.semantics.body_execution
                            == Some(tcl_registry::body_execution::BodyExecutionSpec::SourceFile),
                    scheduling: match flow {
                        ScriptBodyFlow::Deferred => ScriptScheduling::Deferred,
                        ScriptBodyFlow::None => ScriptScheduling::None,
                        _ => ScriptScheduling::Immediate {
                            once: schema.semantics.frame_effect.is_none_or(|effect| {
                                effect.level_word
                                    == tcl_registry::frame_effect::FrameLevelWord::None
                            }) && matches!(
                                flow,
                                ScriptBodyFlow::Sequence(_)
                                    | ScriptBodyFlow::ConcatenatedScript { .. }
                            ),
                        },
                    },
                    invokes_prefix: schema.authored_source_retains_callback() != Some(false),
                }
            })
            .unwrap_or(CommandFlow {
                terminates: false,
                may_end_load: true,
                scheduling: ScriptScheduling::Immediate { once: false },
                invokes_prefix: true,
            })
    }

    /// Operands whose role may carry a script, for `Selected` words with
    /// complete roles; the written token and the whole native word are both
    /// covered.
    fn script_operands(
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> Option<Vec<tcl_lexer::Span>> {
        let mut spans = Vec::new();
        for &(argument, role) in words.roles()? {
            if role.carries_script()
                || matches!(
                    role,
                    tcl_registry::ArgRole::OpaqueScript | tcl_registry::ArgRole::CommandPrefix
                )
            {
                let operand = words.operands().get(argument)?.as_ref()?;
                let written = operand.span();
                spans.push(operand.word().map_or(written, |word| {
                    let whole = word.span();
                    tcl_lexer::Span::new(
                        whole.start().min(written.start()),
                        whole.end().max(written.end()),
                    )
                }));
            }
        }
        Some(spans)
    }

    /// Bodies this command enters once, immediately and in the current frame.
    fn entered_bodies(
        &self,
        command: &tcl_compiler::segmenter::SegmentedCommand,
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        flow: CommandFlow,
    ) -> Vec<tcl_lexer::Span> {
        let at = command.span.start();
        if !flow.scheduling.enters_once()
            || self
                .structure
                .caller_context_unavailable_bodies
                .iter()
                .any(|body| body.start() <= at && at < body.end())
        {
            return Vec::new();
        }
        words
            .source_script_bodies(self.input.borrowed_context_registry())
            .iter()
            .map(tcl_compiler::registry_invocation::OriginalSourceScriptBody::content_span)
            .filter(|content| {
                self.analysis
                    .original_definition_body_span(content.start())
                    .is_none()
            })
            .collect()
    }

    /// Whether a straight command may now run a script operand this inventory
    /// does not retain as a region, which could hold an early exit. Deferred
    /// bodies run later and are no exit of the document.
    fn may_run_unretained_script(
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        flow: CommandFlow,
        regions: &[SequenceRegion],
    ) -> bool {
        if !flow.scheduling.runs_now() {
            return false;
        }
        let Some(roles) = words.roles() else {
            return words
                .written_argument_roles()
                .iter()
                .any(|(_, role)| *role == tcl_registry::ArgRole::Body);
        };
        roles
            .iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::Body)
            .any(|&(argument, _)| {
                let Some(operand) = words.operands().get(argument).and_then(Option::as_ref) else {
                    return true;
                };
                !Self::retains(regions, operand.span())
            })
    }

    /// End-only: whether a command that does not defer its scripts may run a
    /// script now that the inventory cannot see — a script-carrying operand
    /// (`Body`, `Expr`, `LambdaLiteral`, `OpaqueScript`, `CommandPrefix`) not
    /// retained as a region, an expression calling a math function (a
    /// command), or any command prefix it invokes. Such a script may
    /// `return -code return` and end the document's normal completion.
    fn may_complete_early(
        &self,
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        flow: CommandFlow,
        regions: &[SequenceRegion],
    ) -> bool {
        use tcl_registry::ArgRole;
        let carries = |role: ArgRole| {
            role.carries_script() || matches!(role, ArgRole::OpaqueScript | ArgRole::CommandPrefix)
        };
        if flow.scheduling.defers() {
            return false;
        }
        if flow.invokes_prefix {
            return true;
        }
        let Some(roles) = words.roles() else {
            return words
                .written_argument_roles()
                .iter()
                .any(|&(_, role)| carries(role));
        };
        roles
            .iter()
            .filter(|&&(_, role)| carries(role))
            .any(|&(argument, role)| {
                let Some(operand) = words.operands().get(argument).and_then(Option::as_ref) else {
                    return true;
                };
                if role == ArgRole::CommandPrefix {
                    return true;
                }
                let literal = operand
                    .word()
                    .and_then(tcl_syntax::word_rules::original_static_word_unicode_value);
                if role == ArgRole::Expr {
                    return self.expression_may_complete_early(literal.as_deref())
                        || (literal.as_ref().is_some_and(|value| value.contains(&b'['))
                            && !Self::retains(regions, operand.span()));
                }
                // An empty literal body runs nothing.
                if literal.is_some_and(|value| value.iter().all(u8::is_ascii_whitespace)) {
                    return false;
                }
                !Self::retains(regions, operand.span())
            })
    }

    /// Whether an expression may run a command the inventory cannot see. The
    /// shared expression owner parses it under the input's dialect; any math
    /// function call (`f(1)`, a user `::tcl::mathfunc` procedure or an
    /// undefined name reaching `unknown`) or a parse failure counts. A word
    /// that is not a literal is substituted twice and counts too. Command
    /// substitutions are judged by their retained regions.
    fn expression_may_complete_early(&self, literal: Option<&[u8]>) -> bool {
        let Some(text) = literal.and_then(|value| std::str::from_utf8(value).ok()) else {
            return true;
        };
        let tree = tcl_syntax::expr::parser::parse_expr_for_profile(
            text,
            Some(self.input.analyser_profile()),
        );
        tree.evaluation_nodes().iter().any(|node| {
            matches!(
                node,
                tcl_syntax::expr::ast::ExprNode::Call { .. }
                    | tcl_syntax::expr::ast::ExprNode::Raw { .. }
            )
        })
    }

    /// Whether a retained script region lies inside `span`.
    fn retains(regions: &[SequenceRegion], span: tcl_lexer::Span) -> bool {
        let first = regions.partition_point(|region| region.span.start() < span.start());
        regions.get(first).is_some_and(|region| {
            region.span.start() < span.end() && region.span.end() <= span.end()
        })
    }

    fn selected(
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> bool {
        matches!(
            words.source(),
            tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::Selected
        )
    }

    /// Bodies a deferring command stores for a later invocation.
    fn deferred_bodies(
        &self,
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> Vec<tcl_lexer::Span> {
        words
            .source_script_bodies(self.input.borrowed_context_registry())
            .iter()
            .map(tcl_compiler::registry_invocation::OriginalSourceScriptBody::content_span)
            .collect()
    }

    fn declares_head(&self, command: &tcl_compiler::segmenter::SegmentedCommand) -> bool {
        command.texts.first().is_some_and(|head| {
            self.declared
                .contains(tcl_syntax::naming::key_holder_and_tail(head).1.as_bytes())
        })
    }
}

impl DocumentSequence {
    fn capture(
        source: &str,
        analysis: &tcl_compiler::analyser::AnalysisResult,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        structure: &crate::source_structure::SourceStructure,
        slots: &mut [RegistryWordsSlot],
    ) -> Option<Self> {
        let end = u32::try_from(source.len()).ok()?;
        let capture = SequenceCapture {
            analysis,
            input,
            image,
            structure,
            declared: analysis
                .original_procedure_declarations()
                .map(|record| record.name().slot().simple.as_bytes())
                .chain(
                    analysis
                        .original_class_declarations()
                        .map(|record| record.name().slot().simple.as_bytes()),
                )
                .collect(),
        };
        let mut sequence = Self {
            end,
            cutoff: None,
            end_withheld: false,
            regions: Self::regions(structure, end),
            definition_bodies: Self::definition_bodies(analysis),
        };
        // Single start-ordered sweep: the innermost region of each command
        // is reached from the last region starting at or before it through
        // parent links, and the open straight commands form a stack.
        let mut next_region = 0;
        let mut current: Option<usize> = None;
        let mut open: Vec<tcl_lexer::Span> = Vec::new();
        for (index, command) in structure.commands.iter().enumerate() {
            let at = command.span.start();
            while sequence
                .regions
                .get(next_region)
                .is_some_and(|region| region.span.start() <= at)
            {
                current = Some(next_region);
                next_region += 1;
            }
            current = sequence.innermost_from(current, at);
            if Self::in_bodies(&sequence.definition_bodies, at) {
                continue;
            }
            while open.last().is_some_and(|span| span.end() <= at) {
                open.pop();
            }
            let straight = current.is_some_and(|region| sequence.regions[region].straight);
            if straight {
                open.push(command.span);
            }
            let words = capture.validated(slots[index].get(|| {
                tcl_compiler::registry_invocation::source_structure::source_registry_words(
                    source, analysis, command,
                )
            }));
            let flow = words.map(|words| capture.flow(words));
            let completes_early = words.zip(flow).is_some_and(|(words, flow)| {
                capture.may_complete_early(words, flow, &sequence.regions)
            });
            sequence.record_exits(
                completes_early,
                current,
                words,
                flow,
                open.last().copied(),
                at,
            );
            if let (Some(words), Some(flow)) = (words, flow)
                && flow.scheduling.defers()
            {
                sequence.mark(&capture.deferred_bodies(words), |region| {
                    region.deferred = true;
                });
            }
            if let Some(region) = current.filter(|_| straight) {
                sequence.record_straight(&capture, region, command, words, flow);
            }
        }
        Some(sequence)
    }

    fn definition_bodies(
        analysis: &tcl_compiler::analyser::AnalysisResult,
    ) -> Vec<(tcl_lexer::Span, u32)> {
        let mut bodies = analysis
            .original_procedure_declarations()
            .map(|record| record.metadata().body_span)
            .chain(
                analysis
                    .original_class_declarations()
                    .map(|record| record.metadata().body_span),
            )
            .map(|span| (span, span.end()))
            .collect::<Vec<_>>();
        bodies.sort_by_key(|(span, _)| (span.start(), span.end()));
        let mut reach = 0;
        for (span, running) in &mut bodies {
            reach = reach.max(span.end());
            *running = reach;
        }
        bodies
    }

    /// The innermost region containing `at`, from a region starting at or
    /// before it, through parent links.
    fn innermost_from(&self, mut current: Option<usize>, at: u32) -> Option<usize> {
        while let Some(index) = current
            && !(self.regions[index].span.start() <= at && at < self.regions[index].span.end())
        {
            current = self.regions[index].parent;
        }
        current
    }

    fn in_deferred(&self, mut current: Option<usize>) -> bool {
        while let Some(index) = current {
            if self.regions[index].deferred {
                return true;
            }
            current = self.regions[index].parent;
        }
        false
    }

    /// Apply `update` to every retained region with exactly one of `spans`.
    fn mark(&mut self, spans: &[tcl_lexer::Span], update: impl Fn(&mut SequenceRegion)) {
        for content in spans {
            if let Ok(body) = self.regions.binary_search_by(|region| {
                (region.span.start(), std::cmp::Reverse(region.span.end()))
                    .cmp(&(content.start(), std::cmp::Reverse(content.end())))
            }) {
                update(&mut self.regions[body]);
            }
        }
    }

    /// Record the two kinds of early exit a command outside definition bodies
    /// may take. A known terminator (or an unretained script that runs now)
    /// cuts every later position and the end; anything else that may end the
    /// document's normal completion early — control transfer, a sourced
    /// file, a script-carrying operand that is not retained or a command
    /// prefix, words that are not selected Registry words (user procedures,
    /// dynamic heads) — withholds only the end, since a later position is
    /// reached only when control did pass.
    fn record_exits(
        &mut self,
        completes_early: bool,
        current: Option<usize>,
        words: Option<&tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords>,
        flow: Option<CommandFlow>,
        straight_ancestor: Option<tcl_lexer::Span>,
        at: u32,
    ) {
        if self.in_deferred(current) {
            return;
        }
        let straight = current.is_some_and(|region| self.regions[region].straight);
        let unretained = words.zip(flow).is_some_and(|(words, flow)| {
            SequenceCapture::may_run_unretained_script(words, flow, &self.regions)
        });
        if flow.is_some_and(|flow| flow.terminates) || (straight && unretained) {
            let exit = straight_ancestor.map_or(at, tcl_lexer::Span::end);
            self.cutoff = Some(self.cutoff.map_or(exit, |cutoff| cutoff.min(exit)));
        }
        self.end_withheld |= unretained
            || completes_early
            || flow.is_none_or(|flow| flow.may_end_load)
            || !words.is_some_and(SequenceCapture::selected);
    }

    /// Admit one direct command of a straight region and mark the bodies it
    /// enters once, immediately and in the current frame.
    fn record_straight(
        &mut self,
        capture: &SequenceCapture<'_>,
        region: usize,
        command: &tcl_compiler::segmenter::SegmentedCommand,
        words: Option<&tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords>,
        flow: Option<CommandFlow>,
    ) {
        let selected = words
            .filter(|words| SequenceCapture::selected(words) && !capture.declares_head(command));
        let script_operands = selected.and_then(SequenceCapture::script_operands);
        if script_operands.is_some()
            && let Some(words) = selected
            && let Some(flow) = flow
        {
            self.mark(&capture.entered_bodies(command, words, flow), |region| {
                region.straight = true;
            });
        }
        self.regions[region].commands.push(SequenceCommand {
            span: command.span,
            head: command.argv.first().map(|word| word.span),
            script_operands,
        });
    }

    /// Retained script regions ordered by start, outermost first, with their
    /// nesting parents; only the document root starts straight.
    fn regions(
        structure: &crate::source_structure::SourceStructure,
        end: u32,
    ) -> Vec<SequenceRegion> {
        let mut spans = structure
            .scripts
            .iter()
            .map(|(span, _)| *span)
            .collect::<Vec<_>>();
        spans.sort_by_key(|span| (span.start(), std::cmp::Reverse(span.end())));
        spans.dedup();
        let root = tcl_lexer::Span::new(0, end);
        let mut regions = Vec::with_capacity(spans.len());
        let mut stack: Vec<usize> = Vec::new();
        for span in spans {
            while let Some(&open) = stack.last() {
                let outer: &SequenceRegion = &regions[open];
                if outer.span.start() <= span.start() && span.end() <= outer.span.end() {
                    break;
                }
                stack.pop();
            }
            regions.push(SequenceRegion {
                span,
                parent: stack.last().copied(),
                straight: span == root,
                deferred: false,
                commands: Vec::new(),
            });
            stack.push(regions.len() - 1);
        }
        regions
    }

    fn in_bodies(bodies: &[(tcl_lexer::Span, u32)], at: u32) -> bool {
        let index = bodies.partition_point(|(span, _)| span.start() <= at);
        index > 0 && bodies[index - 1].1 > at
    }

    fn contains(&self, at: u32) -> bool {
        if at >= self.end {
            return self.cutoff.is_none() && !self.end_withheld;
        }
        if self.cutoff.is_some_and(|cutoff| at >= cutoff) {
            return false;
        }
        if Self::in_bodies(&self.definition_bodies, at) {
            return false;
        }
        let mut current = self
            .regions
            .partition_point(|region| region.span.start() <= at)
            .checked_sub(1);
        while let Some(index) = current
            && !(self.regions[index].span.start() <= at && at < self.regions[index].span.end())
        {
            current = self.regions[index].parent;
        }
        let Some(region) = current.map(|index| &self.regions[index]) else {
            return false;
        };
        if !region.straight {
            return false;
        }
        let next = region
            .commands
            .partition_point(|command| command.span.start() <= at);
        let Some(command) = next.checked_sub(1).map(|index| &region.commands[index]) else {
            return true;
        };
        at >= command.span.end()
            || command
                .head
                .is_some_and(|head| head.start() <= at && at < head.end())
            || command.script_operands.as_ref().is_some_and(|operands| {
                !operands
                    .iter()
                    .any(|operand| operand.start() <= at && at < operand.end())
            })
    }
}

/// Original source-point transport retained at the actual indexing ingress.
/// Captured source/body/schema owners stay separate from runtime completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceOrderContext {
    image: tcl_lexer::SourceImage,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    realm: tcl_compiler::realm::CommandBindingRealm,
    sites: Vec<OriginalSourceOrderSite>,
    // Complete only for the retained readonly syntax inventory, never execution.
    command_spans: Vec<tcl_lexer::Span>,
    // Commands whose head is a literal word: order needs no admitted site.
    literal_head_commands: Vec<tcl_lexer::Span>,
    // Bodies whose commands issue no site, and those run during their command.
    unavailable_bodies: Vec<tcl_lexer::Span>,
    invocation_scoped_bodies: Vec<tcl_lexer::Span>,
    // Same-document straight-line sequence; None keeps every point Conditional.
    sequence: Option<DocumentSequence>,
}

impl SourceOrderContext {
    pub(crate) fn capture(analysis: &tcl_compiler::analyser::AnalysisResult) -> Option<Self> {
        let input = analysis.resolved_input.as_ref()?;
        let realm = analysis.retained_command_realm()?;
        let image = realm.original_source_image()?;
        let config = input.lexer_config();
        if image.channel() != tcl_lexer::SourceChannel::Document
            || !analysis.matches_original_source_image(image, config)
            || !realm.matches_resolved_analysis_input(input)
        {
            return None;
        }
        let source = image.try_text().ok()?;
        // Reuse the original readonly body/command issuer. Missing geometry
        // remains unrepresented; this partial inventory supplies no closure.
        let structure =
            crate::source_structure::SourceStructure::capture(source, Some(analysis), config)?;
        let command_spans = structure
            .commands
            .iter()
            .map(|command| command.span)
            .collect();
        let literal_head_commands = structure
            .commands
            .iter()
            .filter(|command| {
                !command.is_partial
                    && command
                        .expand_word
                        .as_ref()
                        .is_none_or(|expand| !expand.first().copied().unwrap_or(false))
                    && command.word_fragments.first().is_some_and(|head| {
                        !head.is_empty()
                            && head.iter().all(|fragment| {
                                matches!(
                                    fragment.token.kind,
                                    tcl_lexer::TokenType::Esc | tcl_lexer::TokenType::Str
                                )
                            })
                    })
            })
            .map(|command| command.span)
            .collect();
        // One original Registry selection per command, shared by both owners.
        let mut slots = vec![RegistryWordsSlot::Pending; structure.commands.len()];
        let sequence =
            DocumentSequence::capture(source, analysis, input, image, &structure, &mut slots);
        let mut sites = Vec::new();
        for (index, command) in structure.commands.iter().enumerate() {
            let at = command.argv.first()?.span.start();
            let body = analysis.original_definition_body_span(at);
            if body.is_none()
                && structure
                    .caller_context_unavailable_bodies
                    .iter()
                    .any(|span| span.start() <= at && at < span.end())
            {
                continue;
            }
            let Some(words) = slots[index].take(|| {
                tcl_compiler::registry_invocation::source_structure::source_registry_words(
                    source, analysis, command,
                )
            }) else {
                continue;
            };
            if !words.matches_source(image, config)
                || !words.matches_registry(input.borrowed_context_registry().commands())
                || words.context() != Some(input.availability_context())
            {
                continue;
            }
            let source_load = tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(source, analysis, at);
            let package_require = words
                .package_reference(input.borrowed_context_registry())
                .filter(|reference| {
                    reference.kind()
                        == tcl_registry::source_navigation::SourcePackageReferenceKind::Require
                });
            sites.push(OriginalSourceOrderSite {
                span: command.span,
                body,
                source_load,
                package_require,
                words,
            });
        }
        Some(Self {
            image: image.clone(),
            input: input.clone(),
            realm: realm.clone(),
            sites,
            command_spans,
            literal_head_commands,
            unavailable_bodies: structure.caller_context_unavailable_bodies.clone(),
            invocation_scoped_bodies: structure.invocation_scoped_bodies.clone(),
            sequence,
        })
    }

    pub(crate) fn source_text(&self) -> Option<&str> {
        self.image.try_text().ok()
    }
    pub(crate) fn matches_input(
        &self,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    ) -> bool {
        &self.input == input && self.realm.matches_resolved_analysis_input(input)
    }

    fn site(&self, at: u32) -> Option<&OriginalSourceOrderSite> {
        if !self.realm.matches_resolved_analysis_input(&self.input)
            || !self
                .realm
                .matches_original_source_image(&self.image, self.input.lexer_config())
        {
            return None;
        }
        // Select syntax before admission: an unissued inner command must not
        // borrow a wider enclosing declaration command's source/body owner.
        let command_span = self
            .command_spans
            .iter()
            .filter(|span| span.start() <= at && at < span.end())
            .min_by_key(|span| span.end() - span.start())?;
        self.sites
            .iter()
            .filter(|site| {
                site.span == *command_span
                    && site.span.start() <= at
                    && at < site.span.end()
                    && site
                        .words
                        .matches_source(&self.image, self.input.lexer_config())
                    && site
                        .words
                        .matches_registry(self.input.borrowed_context_registry().commands())
            })
            .min_by_key(|site| site.span.end() - site.span.start())
    }

    /// `point` under the definition body ordering it: its own site's, or none
    /// for a literal-headed command no site admits that runs at the position
    /// of the innermost enclosing straight-line command, every region between
    /// them being straight or run during its command (`namespace eval`). A
    /// dynamic head, a branch, or a deferred or unretained body stays unordered.
    fn ordered_point<'a>(&self, point: RunPoint<'a>) -> Option<RunPoint<'a>> {
        let at = point.at;
        if let Some(site) = self.site(at) {
            return Some(RunPoint {
                enclosing_body: site.body,
                ..point
            });
        }
        let sequence = self.sequence.as_ref()?;
        let contains = |span: &tcl_lexer::Span| span.start() <= at && at < span.end();
        let innermost = |spans: &mut dyn Iterator<Item = &tcl_lexer::Span>| {
            spans.min_by_key(|span| span.end() - span.start()).copied()
        };
        let head = innermost(&mut self.command_spans.iter().filter(|span| contains(span)))?;
        let anchor = innermost(
            &mut self
                .command_spans
                .iter()
                .filter(|span| contains(span) && self.sequence_position(span.start())),
        )?;
        let within_anchor = |span: &tcl_lexer::Span| {
            contains(span) && anchor.start() < span.start() && span.end() <= anchor.end()
        };
        (self.literal_head_commands.contains(&head)
            && !self
                .sites
                .iter()
                .any(|site| site.body.as_ref().is_some_and(contains))
            && sequence
                .regions
                .iter()
                .filter(|region| within_anchor(&region.span))
                .all(|region| {
                    region.straight || self.invocation_scoped_bodies.contains(&region.span)
                })
            && self
                .unavailable_bodies
                .iter()
                .filter(|body| contains(body))
                .all(|body| self.invocation_scoped_bodies.contains(body)))
        .then_some(RunPoint {
            enclosing_body: None,
            ..point
        })
    }

    /// The document end when it is a sequence position: the document is
    /// read after it completes normally.
    fn sequence_end(&self) -> Option<u32> {
        let end = self.sequence.as_ref()?.end;
        self.sequence_position(end).then_some(end)
    }

    /// Whether `at` is a straight-line sequence position of this document
    /// under the same retained image/input/Realm currency as [`Self::site`].
    fn sequence_position(&self, at: u32) -> bool {
        self.realm.matches_resolved_analysis_input(&self.input)
            && self
                .realm
                .matches_original_source_image(&self.image, self.input.lexer_config())
            && self
                .sequence
                .as_ref()
                .is_some_and(|sequence| sequence.contains(at))
    }

    fn edge_site(&self, edge: &RunEdge) -> Option<&OriginalSourceOrderSite> {
        let site = self.site(edge.at)?;
        match edge.kind {
            RunEdgeKind::Source => site
                .source_load
                .as_ref()?
                .matches_source(&self.image, &self.input)
                .then_some(())?,
            RunEdgeKind::PackageRequire => {
                site.package_require.as_ref()?;
            }
        }
        Some(site)
    }
}

/// A `String`-keyed map on the [`RunOrder`] hot path.
///
/// Every one of these is probed from inside the cross-document import walk,
/// which runs per (invocation × in-scope import), so the default `SipHash`
/// over an owned URI string is itself a measurable share of a 26 s
/// `textDocument/references` (338 of 439 samples under
/// [`RunOrder::alternatives`] were in `sip::Hasher::write`).  These keys are
/// workspace-local document URIs, never attacker-chosen, so the `HashDoS`
/// resistance `SipHash` buys is not worth its cost here.
type UriMap<V> = rustc_hash::FxHashMap<String, V>;
/// The [`UriMap`] of sets — same keys, same reasoning.
type UriSet = rustc_hash::FxHashSet<String>;

/// A constructed source-order model, or purpose-bound conditional workspace
/// navigation when built with authentic indexed source contexts. Neither mode
/// is an actual interpreter chronology. The relation both
/// wildcard-import tiers rank cross-document events with (design in
/// `docs/design/analysis/import-order-source-graph.md` §6).
///
/// The coordinate explanation below describes the constructed hypothesis.
/// Actual `WorkspaceIndex` results keep Conditional purpose even when its
/// relative coordinates are unique; they do not authorize event suppression.
/// The one exception is two straight-line positions of the same indexed
/// document, which that document's own sequence orders
/// ([`SourceNavigationOrderPurpose::DocumentSequence`]).
///
/// # Why an order exists at all
///
/// Nothing in a Tcl source tree says which of two files runs first, which is
/// why every cross-document import-lifecycle event was passed *unordered* and
/// the shared decision functions abstained toward answering: a foreign
/// `namespace export` counted, a foreign `namespace export -clear` revoked
/// nothing, a foreign `namespace forget` revoked nothing.  Two statements are
/// the exception:
///
/// - a **`source`** is an ordinary statement of the sourcing file that runs
///   the whole sourced file *at that point*, so the DFS of the `source` forest
///   **is** the run order ([`RunEdgeKind::Source`]);
/// - a **`package require`** guarantees the providing file has run by the time
///   it returns — but not that it ran *there*, since a package already loaded
///   makes the require evaluate nothing ([`RunEdgeKind::PackageRequire`]).
///
/// # The relation
///
/// Each point is lifted to its root-ward key: the offsets of the statements
/// that entered each document down the path, then the point's own offset.  Two
/// keys are compared element-wise; the first index where they differ is their
/// **deepest common document**, and there the existing single-document rules
/// apply unchanged — [`tcl_compiler::analyser::indirection::in_effect_within`]
/// for [`Self::has_run`], [`crate::namespace_import::load_order`] for
/// [`Self::cmp_run`].  One relation, both questions, so the gate and the
/// conflict rule cannot drift (design §6.1: a `has_run` predicate alone is not
/// enough — the folds rank events against *each other*, and a partial order
/// has no `max`).
///
/// A `package require` edge changes nothing about that projection; it only
/// marks the position it contributes as a **bound** rather than a fact
/// ([`Placed::exact`]), and [`Self::trusted`] then reads off the half of the
/// comparison the bound survives.
///
/// # Where it abstains, deliberately
///
/// `None` — incomparable — everywhere the order is not a *fact*, which the
/// callers fold to the same "abstain toward answering" direction `at: None`
/// took before:
///
/// - the two documents sit in **different trees** (no path joins them) — the
///   overwhelmingly common case, and exactly today's behaviour;
/// - either document is **ambiguous**: entered from two different statements,
///   or on a cycle.  Tcl tolerates re-sourcing a file, so a document with two
///   entry points has no unique position and must not be given one;
/// - either document is unknown to the graph;
/// - the answer would rest on the *late* side of a `package require` bound —
///   see [`Self::trusted`].
///
/// Two points in the **same** document never consult the graph at all: their
/// order is already fixed by the same rule.  A workspace with no resolvable
/// edge therefore orders nothing across documents.
///
/// Only *literal*, resolvable `source` targets become `source` edges — the
/// caller filters those — because `source $dir/x.tcl` sequences nothing; the
/// caller's soundness gates on `package require` edges are on
/// [`crate::workspace_index::WorkspaceIndex::package_run_edges`].  A statement
/// written inside a proc body is kept and judged by `in_effect_within` like
/// any other body-local statement, the same leniency
/// [`ancestor_prefer_latest_raised`] already applies to the same edges.
#[derive(Debug, Clone, Default)]
pub struct RunOrder {
    /// None is the explicitly constructed compatibility model. Workspace
    /// navigation always retains Some, including unavailable source owners.
    source_contexts: Option<UriMap<SourceOrderContext>>,
    /// Root-ward path of every document the graph knows: root-first positions
    /// of the `source` statements that enter the next document down.  Empty
    /// for a root.
    paths: UriMap<Vec<Placed>>,
    /// Root document of every keyed document (itself, for a root).
    root_of: UriMap<String>,
    /// Documents with no unique position — re-sourced from two sites, or on a
    /// cycle.  Every cross-document comparison touching one abstains.
    ambiguous: UriSet,
    /// [`Self::alternatives`]'s answer, per **document** rather than per root.
    ///
    /// The `package require` sites that load a document's tree are a fact about
    /// its root, and whether the document may use them is a fact about its own
    /// ambiguity — both fixed at build time.  Resolving that per call meant up
    /// to three hash probes on owned URI strings inside the import walk, and
    /// the `package_entries.is_empty()` fast path that was supposed to make it
    /// free vanished the moment *any* document in the workspace contributed a
    /// package edge.  Folding it into one probe of one map costs
    /// nothing extra: the site vectors are shared with `Arc`, so a root's
    /// entry is stored once however many documents it covers, and a workspace
    /// with no package edge leaves the map empty and every probe a miss.
    package_alternatives: UriMap<Arc<[OwnedRunPoint]>>,
}

/// A [`RunPoint`] the order has to keep past the borrow it was built from —
/// the `package require` sites in [`RunOrder::package_entries`].
#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedRunPoint {
    uri: String,
    at: u32,
    enclosing_body: Option<tcl_lexer::Span>,
}

impl OwnedRunPoint {
    fn as_point(&self) -> RunPoint<'_> {
        RunPoint {
            uri: &self.uri,
            at: self.at,
            enclosing_body: self.enclosing_body,
        }
    }
}

impl RunOrder {
    /// Build an explicitly constructed source-order hypothesis. This pure
    /// compatibility ingress authenticates no execution or original source.
    /// Workspace navigation uses `for_source_navigation` and typed answers.
    ///
    /// `edges` are `(parent, child)` with the parent-side position of the
    /// statement that brings `child` into the run; the caller has already
    /// dropped non-literal and unresolvable `source` targets and applied the
    /// `package require` soundness gates.  A child `source`d from two distinct
    /// sites — or from itself — is marked ambiguous, as is everything below it
    /// and every document on a cycle.
    ///
    /// [`RunEdgeKind::PackageRequire`] edges are kept **out** of the tree: a
    /// package is loaded once, by whichever require runs first, so a provider
    /// required from twenty files has twenty upper bounds and no unique
    /// position.  They are recorded as [`Self::alternatives`] instead, which is
    /// all the bound can support.  A document that is both `source`d and
    /// `package require`d **is** ambiguous — it would then have a tree position
    /// claiming it ran exactly there and a bound saying it had already run
    /// earlier, and those can contradict.
    #[must_use]
    pub fn build(edges: &[RunEdge]) -> Self {
        Self::build_with_source_refusals(edges, UriSet::default())
    }

    fn build_with_source_refusals(edges: &[RunEdge], mut ambiguous: UriSet) -> Self {
        let mut parent_of: HashMap<&str, (&str, Placed)> = HashMap::new();
        let mut documents: HashSet<&str> = HashSet::new();
        let mut package_entries: HashMap<String, Vec<OwnedRunPoint>> = HashMap::new();
        for edge in edges {
            documents.insert(edge.parent.as_str());
            documents.insert(edge.child.as_str());
            let entry = Placed {
                at: edge.at,
                enclosing_body: edge.enclosing_body,
                exact: edge.kind.is_exact(),
            };
            if edge.parent == edge.child {
                ambiguous.insert(edge.child.clone());
                continue;
            }
            if edge.kind == RunEdgeKind::PackageRequire {
                package_entries
                    .entry(edge.child.clone())
                    .or_default()
                    .push(OwnedRunPoint {
                        uri: edge.parent.clone(),
                        at: edge.at,
                        enclosing_body: edge.enclosing_body,
                    });
                continue;
            }
            match parent_of.entry(edge.child.as_str()) {
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert((edge.parent.as_str(), entry));
                }
                std::collections::hash_map::Entry::Occupied(slot) => {
                    // A second, different entry site: no unique position.
                    if *slot.get() != (edge.parent.as_str(), entry) {
                        ambiguous.insert(edge.child.clone());
                    }
                }
            }
        }
        // A document that is both `source`d and `package require`d has two
        // incompatible stories about when it ran; keep neither.
        for child in package_entries.keys() {
            if parent_of.contains_key(child.as_str()) {
                ambiguous.insert(child.clone());
            }
        }
        let mut paths: UriMap<Vec<Placed>> = UriMap::default();
        let mut root_of: UriMap<String> = UriMap::default();
        for &doc in &documents {
            let mut chain: Vec<Placed> = Vec::new();
            let mut seen: HashSet<&str> = HashSet::new();
            let mut cursor = doc;
            let root = loop {
                if !seen.insert(cursor) {
                    // A `source` cycle: no document on it has a position.
                    for &node in &seen {
                        ambiguous.insert(node.to_owned());
                    }
                    break None;
                }
                match parent_of.get(cursor) {
                    Some(&(parent, entry)) => {
                        chain.push(entry);
                        cursor = parent;
                    }
                    None => break Some(cursor),
                }
            };
            let Some(root) = root else { continue };
            // Built child-first; the key is read root-first.
            chain.reverse();
            paths.insert(doc.to_owned(), chain);
            root_of.insert(doc.to_owned(), root.to_owned());
        }
        // Ambiguity is inherited: a document below an ambiguous one has no
        // unique position either.
        let inherited: Vec<String> = paths
            .keys()
            .filter(|doc| {
                let mut cursor = doc.as_str();
                while let Some(&(parent, _)) = parent_of.get(cursor) {
                    if ambiguous.contains(parent) {
                        return true;
                    }
                    cursor = parent;
                }
                false
            })
            .cloned()
            .collect();
        ambiguous.extend(inherited);
        let package_alternatives =
            Self::fold_package_alternatives(&documents, &root_of, &ambiguous, &package_entries);
        Self {
            source_contexts: None,
            paths,
            root_of,
            ambiguous,
            package_alternatives,
        }
    }

    /// Reuse the same source forest/cycle/re-source/package-bound algorithm,
    /// but retain the actual indexed source owners and conditional purpose.
    pub(crate) fn for_source_navigation(
        edges: &[RunEdge],
        contexts: impl Iterator<Item = (String, SourceOrderContext)>,
    ) -> Self {
        let source_contexts: UriMap<_> = contexts.collect();
        let mut unknown = UriSet::default();
        let edges = edges
            .iter()
            .filter_map(|edge| {
                let Some(site) = source_contexts
                    .get(&edge.parent)
                    .and_then(|context| SourceOrderContext::edge_site(context, edge))
                else {
                    unknown.insert(edge.child.clone());
                    return None;
                };
                Some(RunEdge {
                    enclosing_body: site.body,
                    ..edge.clone()
                })
            })
            .collect::<Vec<_>>();
        let mut order = Self::build_with_source_refusals(&edges, unknown);
        order.source_contexts = Some(source_contexts);
        order
    }

    /// Source relation for navigation, retaining its own purpose. Two
    /// straight-line positions of one indexed document are ordered by that
    /// document's sequence ([`SourceNavigationOrderPurpose::DocumentSequence`]);
    /// every other source/package path stays Conditional and establishes no
    /// executed event.
    #[must_use]
    pub fn navigation_precedence(
        &self,
        event: RunPoint<'_>,
        query: RunPoint<'_>,
    ) -> Option<SourceNavigationOrder<bool>> {
        if let Some(known) = self.document_sequence(event, query, |event, query| {
            tcl_compiler::analyser::indirection::in_effect_within(event, query, None)
        }) {
            return Some(known);
        }
        let answer = self.precedence_relation(event, query)?;
        Some(self.source_navigation_answer(answer))
    }

    /// Source-model ranking; only a constructed model or one document's
    /// straight-line sequence can become a conflict.
    #[must_use]
    pub fn navigation_comparison(
        &self,
        a: RunPoint<'_>,
        b: RunPoint<'_>,
    ) -> Option<SourceNavigationOrder<std::cmp::Ordering>> {
        use crate::namespace_import::load_order;
        if let Some(known) =
            self.document_sequence(a, b, |a, b| load_order(a, false).cmp(&load_order(b, false)))
        {
            return Some(known);
        }
        self.site_point(a)?;
        self.site_point(b)?;
        let answer = self.comparison_relation(a, b)?;
        Some(self.source_navigation_answer(answer))
    }

    /// The end of `uri` — after that document completes normally — for a
    /// consumer with no query point of its own, when the document's
    /// sequence admits it. None keeps such a consumer conditional.
    #[must_use]
    pub(crate) fn document_end<'a>(&self, uri: &'a str) -> Option<RunPoint<'a>> {
        let at = self.source_contexts.as_ref()?.get(uri)?.sequence_end()?;
        Some(RunPoint {
            uri,
            at,
            enclosing_body: None,
        })
    }

    /// Whether this is purpose-bound workspace navigation rather than an
    /// explicitly constructed model. Such an order withholds a removal whose
    /// own position against the query point is not known.
    #[must_use]
    pub(crate) const fn is_source_navigation(&self) -> bool {
        self.source_contexts.is_some()
    }

    /// The same-document straight-line relation:
    /// both points in one indexed document and both sequence positions of
    /// it. Supplied body spans are ignored; the retained inventory decides.
    fn document_sequence<T>(
        &self,
        a: RunPoint<'_>,
        b: RunPoint<'_>,
        relation: impl FnOnce(u32, u32) -> T,
    ) -> Option<SourceNavigationOrder<T>> {
        if a.uri != b.uri {
            return None;
        }
        let context = self.source_contexts.as_ref()?.get(a.uri)?;
        (context.sequence_position(a.at) && context.sequence_position(b.at)).then(|| {
            SourceNavigationOrder {
                relation: relation(a.at, b.at),
                purpose: SourceNavigationOrderPurpose::DocumentSequence,
            }
        })
    }

    fn source_navigation_answer<T>(&self, answer: T) -> SourceNavigationOrder<T> {
        SourceNavigationOrder {
            relation: answer,
            purpose: if self.source_contexts.is_some() {
                SourceNavigationOrderPurpose::Conditional
            } else {
                SourceNavigationOrderPurpose::Constructed
            },
        }
    }

    /// Conditional compatibility within an actually represented statement
    /// horizon. An unentered sibling/deferred body is never a root event.
    pub(crate) fn source_advice_statement_precedence(
        &self,
        event: RunPoint<'_>,
        query: RunPoint<'_>,
    ) -> Option<bool> {
        let event = self.original_point(event)?;
        let query = self.original_point(query)?;
        if event.enclosing_body.is_some()
            && (event.uri != query.uri || event.enclosing_body != query.enclosing_body)
        {
            return None;
        }
        let (event, query) = self.common_original_frame(event, query)?;
        let relation = tcl_compiler::analyser::indirection::in_effect_within(
            event.at,
            query.at,
            query.enclosing_body,
        );
        Self::trusted(relation, event, query).then_some(relation)
    }

    pub(crate) fn source_advice_statement_comparison(
        &self,
        a: RunPoint<'_>,
        b: RunPoint<'_>,
    ) -> Option<std::cmp::Ordering> {
        let a = self.site_point(a)?;
        let b = self.site_point(b)?;
        if (a.enclosing_body.is_some() || b.enclosing_body.is_some())
            && (a.uri != b.uri || a.enclosing_body != b.enclosing_body)
        {
            return None;
        }
        self.comparison_relation(a, b)
    }

    /// A declared readonly end-of-document query, not an original command or
    /// executed completion. Unknown tails/entry remain debt at the caller.
    pub(crate) fn source_advice_after_document(
        &self,
        event: RunPoint<'_>,
        uri: &str,
    ) -> Option<bool> {
        let contexts = self.source_contexts.as_ref()?;
        let context = contexts.get(uri)?;
        let event = self.original_point(event)?;
        // Root text ending does not enter a deferred procedure/method body.
        if event.enclosing_body.is_some() {
            return None;
        }
        let query = RunPoint {
            uri,
            at: u32::try_from(context.image.bytes().len()).ok()?,
            enclosing_body: None,
        };
        let (event, query) = self.common_original_frame(event, query)?;
        let relation = tcl_compiler::analyser::indirection::in_effect_within(
            event.at,
            query.at,
            query.enclosing_body,
        );
        Self::trusted(relation, event, query).then_some(relation)
    }

    /// Resolve [`Self::alternatives`] for every document up front: the root
    /// walk and the ambiguity test do not depend on the point being asked
    /// about, so they belong here rather than on the per-call path.
    ///
    /// A root's site vector is shared by every document under it, so a tree is
    /// stored once however deep it is.  A document the graph knows only as a
    /// `package require` target is its own root, so it is keyed too.  An
    /// ambiguous document is left out entirely — that is the abstention the
    /// old per-call `self.ambiguous.contains` performed, moved rather than
    /// dropped.
    fn fold_package_alternatives(
        documents: &HashSet<&str>,
        root_of: &UriMap<String>,
        ambiguous: &UriSet,
        package_entries: &HashMap<String, Vec<OwnedRunPoint>>,
    ) -> UriMap<Arc<[OwnedRunPoint]>> {
        let mut out: UriMap<Arc<[OwnedRunPoint]>> = UriMap::default();
        if package_entries.is_empty() {
            return out;
        }
        let shared: HashMap<&str, Arc<[OwnedRunPoint]>> = package_entries
            .iter()
            .map(|(root, sites)| (root.as_str(), Arc::from(sites.as_slice())))
            .collect();
        for &doc in documents {
            if ambiguous.contains(doc) {
                continue;
            }
            let root = root_of.get(doc).map_or(doc, String::as_str);
            if let Some(sites) = shared.get(root) {
                out.insert(doc.to_owned(), Arc::clone(sites));
            }
        }
        out
    }

    /// Legacy explicitly constructed model comparison, without an execution
    /// receipt. Always None for purpose-bound workspace source navigation;
    /// use `navigation_precedence` to retain its conditional relation.
    ///
    /// The gating question — the leniency is
    /// [`tcl_compiler::analyser::indirection::in_effect_within`]'s, applied in
    /// the two points' deepest common document: a statement at the load level
    /// of some document counts as having run for a query inside one of that
    /// document's proc bodies, however far below it is written, because the
    /// whole file loads before any body runs.
    #[must_use]
    pub fn has_run(&self, event: RunPoint<'_>, query: RunPoint<'_>) -> Option<bool> {
        if self.source_contexts.is_some() {
            return None;
        }
        self.precedence_relation(event, query)
    }

    fn precedence_relation(&self, event: RunPoint<'_>, query: RunPoint<'_>) -> Option<bool> {
        self.decide(event, query, |e, q| {
            let ran =
                tcl_compiler::analyser::indirection::in_effect_within(e.at, q.at, q.enclosing_body);
            Self::trusted(ran, e, q).then_some(ran)
        })
    }

    /// Legacy explicitly constructed model ranking. Always None for
    /// purpose-bound workspace source navigation; `navigation_comparison`
    /// retains conditional coordinates without conflict/completion authority.
    ///
    /// The ranking question the latest-wins folds and the import-conflict rule
    /// need.  Deliberately *not* [`Self::has_run`]'s leniency: a conflict
    /// cancels an install, so counting a maybe-never-run body statement as
    /// having run would invent conflicts and drop aliases the program really
    /// has — the reasoning on [`crate::namespace_import::load_order`].
    #[must_use]
    pub fn cmp_run(&self, a: RunPoint<'_>, b: RunPoint<'_>) -> Option<std::cmp::Ordering> {
        if self.source_contexts.is_some() {
            return None;
        }
        self.comparison_relation(a, b)
    }

    fn comparison_relation(&self, a: RunPoint<'_>, b: RunPoint<'_>) -> Option<std::cmp::Ordering> {
        use crate::namespace_import::load_order;
        use std::cmp::Ordering;
        self.decide(a, b, |pa, pb| {
            let ord = load_order(pa.at, pa.enclosing_body.is_some())
                .cmp(&load_order(pb.at, pb.enclosing_body.is_some()));
            let usable = match ord {
                // `Equal` claims both directions at once, so it needs both.
                Ordering::Equal => pa.exact && pb.exact,
                other => Self::trusted(other == Ordering::Less, pa, pb),
            };
            usable.then_some(ord)
        })
    }

    /// Whether a comparison that came out as `a_first` is a **fact** given how
    /// each side was placed.
    ///
    /// A [`Placed`] that is not [`exact`](Placed::exact) is an *upper bound*:
    /// the statement ran at or before that offset, never after it (a
    /// `package require` finds an already-loaded package and evaluates
    /// nothing).  So each answer needs one side pinned:
    ///
    /// - "`a` ran first" survives `a` sliding earlier, but not `b` sliding
    ///   earlier — it needs `b` exact;
    /// - "`a` did not run first" survives `b` sliding earlier, but not `a`
    ///   sliding earlier — it needs `a` exact.
    ///
    /// Two bounded sides settle nothing, which is why two documents that a
    /// `package require` each brought in are not ranked against each other at
    /// all.
    #[must_use]
    pub fn trusted(a_first: bool, a: Placed, b: Placed) -> bool {
        if a_first { b.exact } else { a.exact }
    }

    /// Put the two points in one document and let `judge` read the pair, over
    /// every placement that could settle the question — the shared body of
    /// [`Self::has_run`] and [`Self::cmp_run`], so both see the same
    /// placements and can only differ in the rule they apply to them.
    ///
    /// The **direct** frame — both points lifted onto the `source` forest and
    /// met at their deepest common document — is the whole answer whenever it
    /// exists, and it is exact on both sides.  It is also the only placement
    /// considered then: a document reachable directly is not also reached
    /// through a `package require` bound (`build` marks such a document
    /// ambiguous), so there is nothing to disagree with.
    ///
    /// Otherwise, a point whose tree the workspace enters by `package require`
    /// is **substituted** by each require site that loads it: the site is a
    /// position by which that whole tree had certainly run, so it stands in for
    /// the point with [`Placed::exact`] cleared.  Only one side is ever
    /// substituted — two bounded sides can settle nothing anyway
    /// ([`Self::trusted`]), and two trees that each require the other would let
    /// one placement's `Less` sit beside another's `Greater`.
    ///
    /// The first placement `judge` accepts wins, and that is not a shortcut:
    /// with one side bounded, `trusted` admits exactly one of the two answers,
    /// so no two placements can accept opposite ones.
    fn decide<T>(
        &self,
        a: RunPoint<'_>,
        b: RunPoint<'_>,
        judge: impl Fn(Placed, Placed) -> Option<T>,
    ) -> Option<T> {
        // A URI-level package bound cannot replace a refused original point.
        // Constructed inputs retain original_point's identity behavior.
        let a = self.original_point(a)?;
        let b = self.original_point(b)?;
        if let Some((x, y)) = self.common_frame(a, b) {
            return judge(x, y);
        }
        let a_alts = self.alternatives(a);
        let b_alts = self.alternatives(b);
        if !a_alts.is_empty() && !b_alts.is_empty() {
            return None;
        }
        let blur = |p: Placed| Placed { exact: false, ..p };
        for site in a_alts {
            if let Some((x, y)) = self.common_frame(site.as_point(), b)
                && let Some(answer) = judge(blur(x), y)
            {
                return Some(answer);
            }
        }
        for site in b_alts {
            if let Some((x, y)) = self.common_frame(a, site.as_point())
                && let Some(answer) = judge(x, blur(y))
            {
                return Some(answer);
            }
        }
        None
    }

    /// The `package require` sites that load the tree `p` sits in, if the
    /// workspace enters that tree by `package require` at all.
    ///
    /// A point's tree is identified by its root: a file the provider `source`s
    /// runs when the provider does, so it is bounded by the same require
    /// sites.
    /// One probe: which document a point sits in is the only thing left to
    /// look up, because the root walk and the ambiguity test do not depend on
    /// the point's offset and are folded into
    /// [`Self::package_alternatives`] at build time.
    fn alternatives(&self, p: RunPoint<'_>) -> &[OwnedRunPoint] {
        self.package_alternatives
            .get(p.uri)
            .map_or(&[], |sites| &**sites)
    }

    fn original_point<'a>(&self, point: RunPoint<'a>) -> Option<RunPoint<'a>> {
        let Some(contexts) = &self.source_contexts else {
            return Some(point);
        };
        contexts.get(point.uri)?.ordered_point(point)
    }

    /// [`Self::original_point`] admitting only a point an issued site owns: a
    /// comparison cancels an install Tcl performs, so it needs that site.
    fn site_point<'a>(&self, point: RunPoint<'a>) -> Option<RunPoint<'a>> {
        let Some(contexts) = &self.source_contexts else {
            return Some(point);
        };
        let site = contexts.get(point.uri)?.site(point.at)?;
        Some(RunPoint {
            enclosing_body: site.body,
            ..point
        })
    }

    /// Both points as positions in their deepest common document — the single
    /// projection [`Self::has_run`] and [`Self::cmp_run`] share, so the gate
    /// and the conflict rule read one relation and differ only in which
    /// single-document rule they then apply.
    fn common_frame(&self, a: RunPoint<'_>, b: RunPoint<'_>) -> Option<(Placed, Placed)> {
        let a = self.original_point(a)?;
        let b = self.original_point(b)?;
        self.common_original_frame(a, b)
    }

    fn common_original_frame(&self, a: RunPoint<'_>, b: RunPoint<'_>) -> Option<(Placed, Placed)> {
        let own = |p: RunPoint<'_>| Placed {
            at: p.at,
            enclosing_body: p.enclosing_body,
            exact: true,
        };
        // Same document: already ordered, and ordered without the graph — a
        // re-sourced or cyclic file still sequences its own statements.
        if a.uri == b.uri {
            return Some((own(a), own(b)));
        }
        if self.ambiguous.contains(a.uri) || self.ambiguous.contains(b.uri) {
            return None;
        }
        // Different trees have no order at all — the common case, and exactly
        // the pre-source-graph behaviour.
        if self.root_of.get(a.uri)? != self.root_of.get(b.uri)? {
            return None;
        }
        let path_a = self.paths.get(a.uri)?;
        let path_b = self.paths.get(b.uri)?;
        for depth in 0..path_a.len().min(path_b.len()) {
            if path_a[depth] != path_b[depth] {
                return Some((path_a[depth], path_b[depth]));
            }
        }
        // One path is a prefix of the other: the shallower point's own
        // document holds the `source` statement that entered the deeper one's
        // branch, so that statement is the deeper point's position there.
        Some(match path_a.len().cmp(&path_b.len()) {
            std::cmp::Ordering::Less => (own(a), path_b[path_a.len()]),
            std::cmp::Ordering::Greater => (path_a[path_b.len()], own(b)),
            // Equal-length shared paths with different URIs cannot happen: a
            // path entry identifies the `source` statement, which identifies
            // the document it entered.
            std::cmp::Ordering::Equal => return None,
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_package_alternative_owner_tests;
#[cfg(test)]
mod original_source_navigation_order_tests;
#[cfg(test)]
mod project_source_action_tests;
