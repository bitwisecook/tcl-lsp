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

/// Associate a genuine original SourceLoad path under the declared project
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
    if raw.is_absolute() && !project_source_action_path(raw)
        || !raw.is_absolute() && matches!(raw.components().next(), Some(Component::Prefix(_)))
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
    let joined = if raw.is_absolute() {
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
    if raw.is_absolute() {
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

fn project_source_action_path(path: &Path) -> bool {
    // POSIX double-leading slash may select an implementation-defined root.
    path.is_absolute()
        && path
            .to_str()
            .is_some_and(|text| !text.contains('\0') && !(cfg!(unix) && text.starts_with("//")))
}

/// Relative spelling in the same host path domain; different roots decline.
#[must_use]
pub fn relative_source_path(target: &Path, base: &Path) -> Option<PathBuf> {
    if !target.is_absolute() || !base.is_absolute() {
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

/// Registry control facts of one command, for the sequence.
#[derive(Debug, Clone, Copy)]
struct CommandFlow {
    /// Traits end the block or the process (`return`, `error`, `exit`).
    terminates: bool,
    /// Traits transfer control elsewhere (`break`, `continue`, `tailcall`),
    /// or the command evaluates another file (`source`), whose own `return`
    /// ends this document's normal completion.
    may_end_load: bool,
    /// A script operand may run before the command returns.
    runs_now: bool,
    /// Its script operands are stored for a later invocation.
    defers: bool,
    /// The Registry may give it a command prefix to invoke (`lsort -command`,
    /// `regsub -command`), or cannot rule one out.
    invokes_prefix: bool,
    /// Its bodies run once, immediately and in the current frame.
    enters_once: bool,
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
                    runs_now: !matches!(flow, ScriptBodyFlow::Deferred | ScriptBodyFlow::None),
                    defers: matches!(flow, ScriptBodyFlow::Deferred),
                    invokes_prefix: schema.authored_source_retains_callback() != Some(false),
                    enters_once: schema.semantics.frame_effect.is_none_or(|effect| {
                        effect.level_word == tcl_registry::frame_effect::FrameLevelWord::None
                    }) && matches!(
                        flow,
                        ScriptBodyFlow::Sequence(_) | ScriptBodyFlow::ConcatenatedScript { .. }
                    ),
                }
            })
            .unwrap_or(CommandFlow {
                terminates: false,
                may_end_load: true,
                runs_now: true,
                defers: false,
                invokes_prefix: true,
                enters_once: false,
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
        if !flow.enters_once
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
        if !flow.runs_now {
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
        if flow.defers {
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
                && flow.defers
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
            let exit = straight_ancestor.map_or(at, |span| span.end());
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
/// Actual WorkspaceIndex results keep Conditional purpose even when its
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
        let answer = self.comparison_relation(a, b)?;
        Some(self.source_navigation_answer(answer))
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
        let a = self.original_point(a)?;
        let b = self.original_point(b)?;
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
        let site = SourceOrderContext::site(contexts.get(point.uri)?, point.at)?;
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
mod tests {
    use super::*;

    fn reqs(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(uri, pkgs)| {
                (
                    (*uri).to_owned(),
                    pkgs.iter().map(|p| (*p).to_owned()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn resolves_relative_source_against_parent_dir() {
        let got = resolve_source_target(Path::new("/proj/app.tcl"), "lib/util.tcl");
        assert_eq!(got, PathBuf::from("/proj/lib/util.tcl"));
    }

    #[test]
    fn resolves_dot_and_dotdot_segments() {
        let got = resolve_source_target(Path::new("/proj/sub/main.tcl"), "../lib/./util.tcl");
        assert_eq!(got, PathBuf::from("/proj/lib/util.tcl"));
    }

    #[test]
    fn absolute_source_passes_through() {
        let got = resolve_source_target(Path::new("/proj/app.tcl"), "/opt/x/y.tcl");
        assert_eq!(got, PathBuf::from("/opt/x/y.tcl"));
    }

    #[test]
    fn inherits_requires_from_direct_parent() {
        // app.tcl requires Tk and sources lib.tcl.
        let edges = vec![("app".to_owned(), "lib".to_owned())];
        let requires = reqs(&[("app", &["Tk", "http"]), ("lib", &[])]);
        let got = ancestor_requires("lib", &edges, &requires);
        assert_eq!(got, vec!["Tk".to_owned(), "http".to_owned()]);
    }

    #[test]
    fn inherits_transitively_through_a_chain() {
        // app -> mid -> leaf; leaf inherits from both app and mid.
        let edges = vec![
            ("app".to_owned(), "mid".to_owned()),
            ("mid".to_owned(), "leaf".to_owned()),
        ];
        let requires = reqs(&[("app", &["Tk"]), ("mid", &["json"]), ("leaf", &[])]);
        let mut got = ancestor_requires("leaf", &edges, &requires);
        got.sort();
        assert_eq!(got, vec!["Tk".to_owned(), "json".to_owned()]);
    }

    #[test]
    fn unions_requires_from_multiple_entry_points() {
        // Both a.tcl and b.tcl source shared.tcl with different requires.
        let edges = vec![
            ("a".to_owned(), "shared".to_owned()),
            ("b".to_owned(), "shared".to_owned()),
        ];
        let requires = reqs(&[("a", &["Tk"]), ("b", &["http"]), ("shared", &[])]);
        let got = ancestor_requires("shared", &edges, &requires);
        assert_eq!(got, vec!["Tk".to_owned(), "http".to_owned()]);
    }

    #[test]
    fn tolerates_cycles() {
        // a -> b -> a (pathological); must not loop forever.
        let edges = vec![
            ("a".to_owned(), "b".to_owned()),
            ("b".to_owned(), "a".to_owned()),
        ];
        let requires = reqs(&[("a", &["Tk"]), ("b", &["http"])]);
        let got = ancestor_requires("a", &edges, &requires);
        assert_eq!(got, vec!["Tk".to_owned(), "http".to_owned()]);
    }

    #[test]
    fn a_root_with_no_ancestors_inherits_nothing() {
        let edges = vec![("app".to_owned(), "lib".to_owned())];
        let requires = reqs(&[("app", &["Tk"])]);
        assert!(ancestor_requires("app", &edges, &requires).is_empty());
    }

    // `package prefer latest` across the source graph.
    //
    // tclsh-proof (8.6.14) that the latch really is interpreter-global and
    // crosses `source`:  with `lib.tcl` holding `puts [package prefer]`,
    //
    //   # app.tcl:  package prefer latest ; source lib.tcl
    //   -> latest
    //   # app.tcl:  source lib.tcl ; package prefer latest
    //   -> stable

    fn edge(parent: &str, child: &str, at: u32) -> RunEdge {
        RunEdge {
            parent: parent.to_owned(),
            child: child.to_owned(),
            at,
            enclosing_body: None,
            kind: RunEdgeKind::Source,
        }
    }

    fn raises(pairs: &[(&str, &[u32])]) -> HashMap<String, Vec<u32>> {
        pairs
            .iter()
            .map(|(uri, offs)| ((*uri).to_owned(), offs.to_vec()))
            .collect()
    }

    /// TP: a raise written **above** the `source` is in force in the child.
    #[test]
    fn a_raise_before_the_source_reaches_the_child() {
        let edges = vec![edge("app", "lib", 30)];
        assert!(ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("app", &[0])])
        ));
    }

    /// TN: a raise written **below** the `source` has not run when the child
    /// loads, so the child keeps the default.
    #[test]
    fn a_raise_after_the_source_does_not_reach_the_child() {
        let edges = vec![edge("app", "lib", 0)];
        assert!(!ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("app", &[30])])
        ));
    }

    /// TP: the latch is transitive — once up on entry to a node it is up for
    /// everything that node sources, wherever the `source` sits.
    #[test]
    fn the_latch_travels_the_whole_graph_once_raised() {
        let edges = vec![edge("app", "mid", 30), edge("mid", "leaf", 0)];
        assert!(ancestor_prefer_latest_raised(
            "leaf",
            &edges,
            &raises(&[("app", &[0])])
        ));
    }

    /// TN: a raise in a node the target is **not** reachable from changes
    /// nothing — the graph, not the workspace, decides.
    #[test]
    fn an_unrelated_raise_does_not_reach_the_target() {
        let edges = vec![edge("app", "lib", 30), edge("other", "sibling", 30)];
        assert!(!ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("other", &[0])])
        ));
    }

    /// TP: a `source` written inside a proc body still sees a raise written
    /// later at the parent's load level — the whole file loads before any
    /// body runs, the `in_effect_within` rule every other cross-document
    /// ordering question uses.
    #[test]
    fn a_body_source_sees_a_later_load_level_raise() {
        let edges = vec![RunEdge {
            parent: "app".to_owned(),
            child: "lib".to_owned(),
            at: 10,
            enclosing_body: Some(tcl_lexer::Span::new(0, 20)),
            kind: RunEdgeKind::Source,
        }];
        assert!(ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("app", &[30])])
        ));
        // …but a raise inside that same body, after the `source`, has not run.
        assert!(!ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("app", &[15])])
        ));
    }

    /// A cycle terminates, and the target is still answered.
    #[test]
    fn the_prefer_walk_tolerates_cycles() {
        let edges = vec![edge("a", "b", 30), edge("b", "a", 0)];
        assert!(ancestor_prefer_latest_raised(
            "b",
            &edges,
            &raises(&[("a", &[0])])
        ));
        assert!(!ancestor_prefer_latest_raised("b", &edges, &raises(&[])));
    }

    /// TN: the target's *own* raises are not this question — that is the
    /// position-sensitive single-document answer.
    #[test]
    fn the_targets_own_raise_is_not_an_ancestor_raise() {
        let edges = vec![edge("app", "lib", 30)];
        assert!(!ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("lib", &[0])])
        ));
    }

    // The `source`-graph load order.
    //
    // Oracle for the whole shape, byte-identical on tclsh 8.6.14 and 9.0.4 —
    // `source` inlines the sourced file's whole body at the `source`
    // statement's position, so the DFS of the source forest is the run order:
    //
    //   # src.tcl:  namespace eval ::src { proc p {} {return P} }
    //   # exp.tcl:  namespace eval ::src { namespace export p }
    //   # imp.tcl:  namespace eval ::dst { namespace import ::src::* }
    //
    //   # app.tcl:  source src.tcl ; source exp.tcl ; source imp.tcl
    //   ::dst::p   -> P                       (export ran before the import)
    //   # app.tcl:  source src.tcl ; source imp.tcl ; source exp.tcl
    //   ::dst::p   -> invalid command name "::dst::p"   (it did not)

    fn point(uri: &str, at: u32) -> RunPoint<'_> {
        RunPoint {
            uri,
            at,
            enclosing_body: None,
        }
    }

    fn body_point(uri: &str, at: u32, body: (u32, u32)) -> RunPoint<'_> {
        RunPoint {
            uri,
            at,
            enclosing_body: Some(tcl_lexer::Span::new(body.0, body.1)),
        }
    }

    /// TP: two files sourced by one parent are ordered by their `source`
    /// statements — everything in the first has run when the second loads.
    #[test]
    fn siblings_are_ordered_by_their_source_statements() {
        let order = RunOrder::build(&[edge("app", "exp", 10), edge("app", "imp", 20)]);
        assert_eq!(
            order.has_run(point("exp", 999), point("imp", 0)),
            Some(true),
            "exp.tcl is sourced first, so all of it has run when imp.tcl loads",
        );
        assert_eq!(
            order.has_run(point("imp", 0), point("exp", 999)),
            Some(false),
            "…and nothing in imp.tcl has run while exp.tcl is still loading",
        );
    }

    /// TP: a parent statement written above the `source` has run inside the
    /// child; one written below has not.
    #[test]
    fn a_parent_statement_is_ordered_against_the_child_body() {
        let order = RunOrder::build(&[edge("app", "lib", 50)]);
        assert_eq!(order.has_run(point("app", 10), point("lib", 5)), Some(true));
        assert_eq!(
            order.has_run(point("app", 90), point("lib", 5)),
            Some(false)
        );
        // …and the child's own statements have run for a parent statement
        // written below the `source`.
        assert_eq!(order.has_run(point("lib", 5), point("app", 90)), Some(true));
        assert_eq!(
            order.has_run(point("lib", 5), point("app", 10)),
            Some(false)
        );
    }

    /// TP: the leniency at the common document is `in_effect_within`'s — a
    /// child sourced from a parent's load level has run for a query inside
    /// one of that parent's proc bodies, however far below the `source` is
    /// written.
    #[test]
    fn a_body_local_query_observes_a_later_top_level_source() {
        let order = RunOrder::build(&[edge("app", "lib", 900)]);
        assert_eq!(
            order.has_run(point("lib", 5), body_point("app", 10, (0, 20))),
            Some(true),
            "the whole file loads — sourcing lib.tcl included — before any body runs",
        );
        // …but a `source` written inside that same body, after the query,
        // is an ordinary later statement of the running script.
        let order = RunOrder::build(&[RunEdge {
            parent: "app".to_owned(),
            child: "lib".to_owned(),
            at: 15,
            enclosing_body: Some(tcl_lexer::Span::new(0, 20)),
            kind: RunEdgeKind::Source,
        }]);
        assert_eq!(
            order.has_run(point("lib", 5), body_point("app", 10, (0, 20))),
            Some(false),
        );
    }

    /// TP: a grandchild is ordered against a sibling of its parent, through
    /// the deepest common document.
    #[test]
    fn a_grandchild_is_ordered_through_the_common_document() {
        // app sources exp @10, then mid @20; mid sources leaf @5.
        let order = RunOrder::build(&[
            edge("app", "exp", 10),
            edge("app", "mid", 20),
            edge("mid", "leaf", 5),
        ]);
        assert_eq!(
            order.has_run(point("exp", 999), point("leaf", 0)),
            Some(true)
        );
        assert_eq!(
            order.has_run(point("leaf", 0), point("exp", 999)),
            Some(false)
        );
    }

    /// TN: documents in different trees have no order — the pre-source-graph
    /// behaviour, and what an unrelated pair of files still gets.
    #[test]
    fn different_trees_are_incomparable() {
        let order = RunOrder::build(&[edge("app", "lib", 10), edge("other", "helper", 10)]);
        assert_eq!(order.has_run(point("lib", 0), point("helper", 0)), None);
        assert_eq!(order.cmp_run(point("lib", 0), point("helper", 0)), None);
    }

    /// TN: an empty graph orders nothing across documents…
    #[test]
    fn an_empty_graph_orders_nothing_across_documents() {
        let order = RunOrder::default();
        assert_eq!(order.has_run(point("a", 0), point("b", 0)), None);
        // …and a document the graph has never heard of is equally unordered.
        let order = RunOrder::build(&[edge("app", "lib", 10)]);
        assert_eq!(order.has_run(point("lib", 0), point("stray", 0)), None);
    }

    /// TP: two statements in the *same* document stay ordered regardless of
    /// the graph — the rule that predates this type, unchanged.
    #[test]
    fn one_document_is_ordered_without_the_graph() {
        let order = RunOrder::default();
        assert_eq!(order.has_run(point("a", 10), point("a", 20)), Some(true));
        assert_eq!(order.has_run(point("a", 20), point("a", 10)), Some(false));
        // Even for a re-sourced file, whose cross-document position abstains.
        let order = RunOrder::build(&[edge("app", "lib", 10), edge("other", "lib", 10)]);
        assert_eq!(
            order.has_run(point("lib", 10), point("lib", 20)),
            Some(true)
        );
    }

    /// TN (CRITICAL): a file reachable from two different `source` sites has
    /// no unique position, so every cross-document comparison touching it
    /// abstains — Tcl tolerates re-sourcing, and inventing an order would
    /// silently drop references.
    #[test]
    fn a_re_sourced_document_has_no_position() {
        let order = RunOrder::build(&[edge("app", "lib", 10), edge("app", "lib", 90)]);
        assert_eq!(order.has_run(point("lib", 0), point("app", 50)), None);
        // Two different parents, same story.
        let order = RunOrder::build(&[edge("app", "lib", 10), edge("tool", "lib", 10)]);
        assert_eq!(order.has_run(point("lib", 0), point("app", 50)), None);
    }

    /// TN: the ambiguity is inherited — a document below a re-sourced one has
    /// no unique position either.
    #[test]
    fn ambiguity_is_inherited_by_descendants() {
        let order = RunOrder::build(&[
            edge("app", "lib", 10),
            edge("app", "lib", 90),
            edge("lib", "deep", 5),
        ]);
        assert_eq!(order.has_run(point("deep", 0), point("app", 50)), None);
    }

    /// TN: a `source` cycle gives nothing on it a position, and terminates.
    #[test]
    fn a_source_cycle_abstains() {
        let order = RunOrder::build(&[edge("a", "b", 10), edge("b", "a", 10)]);
        assert_eq!(order.has_run(point("a", 0), point("b", 0)), None);
        // A self-`source` likewise.
        let order = RunOrder::build(&[edge("a", "a", 10), edge("a", "b", 20)]);
        assert_eq!(order.has_run(point("b", 0), point("a", 50)), None);
    }

    /// `cmp_run` is the strict order: a body-local statement never counts as
    /// having run at another document's load level, where `has_run`'s
    /// leniency says it may have.
    #[test]
    fn cmp_run_is_strict_where_has_run_is_lenient() {
        let order = RunOrder::build(&[edge("app", "lib", 900)]);
        let body_stmt = body_point("app", 10, (0, 20));
        // Lenient: the body statement is treated as having run for a query in
        // the sourced child (the body may have been entered).
        assert_eq!(order.has_run(body_stmt, point("lib", 5)), Some(true));
        // Strict: it is *not* known to have run before the child loaded.
        assert_eq!(
            order.cmp_run(body_stmt, point("lib", 5)),
            Some(std::cmp::Ordering::Greater),
            "a body-local statement sorts after every load-level one",
        );
        // Two load-level statements in different files rank by their `source`
        // statements, which is genuine execution order.
        let order = RunOrder::build(&[edge("app", "one", 10), edge("app", "two", 20)]);
        assert_eq!(
            order.cmp_run(point("one", 999), point("two", 0)),
            Some(std::cmp::Ordering::Less),
        );
    }

    /// Nothing runs before itself, at either level — what keeps an import
    /// from conflicting with itself once the order spans documents.
    #[test]
    fn a_point_does_not_precede_itself() {
        let order = RunOrder::build(&[edge("app", "lib", 10)]);
        assert_eq!(
            order.cmp_run(point("lib", 7), point("lib", 7)),
            Some(std::cmp::Ordering::Equal),
        );
        assert_eq!(order.has_run(point("lib", 7), point("lib", 7)), Some(false));
    }

    // The `package require` half of the load order.
    //
    // Oracle, byte-identical on tclsh 8.6.14 and 9.0.4.  `pkg/lib.tcl` holds
    //
    //   puts "lib.tcl body running"
    //   namespace eval ::lib { proc p {} {return P} ; namespace export p }
    //   package provide mylib 1.0
    //
    // (1) A require IS a load event, and the provider's whole body — export
    //     included — has run by the time it returns:
    //
    //       # app.tcl
    //       before require: lib not loaded
    //       lib.tcl body running
    //       after require:  lib loaded
    //       app::p -> P
    //
    // (2) …but only for whichever require runs first.  `a.tcl` and `b.tcl`
    //     hold the SAME two lines — a probe, then `package require mylib` —
    //     and a driver sources a.tcl then b.tcl:
    //
    //       a.tcl: before its own require, is lib loaded? NO
    //       lib.tcl body running
    //       b.tcl: before its own require, is lib loaded? YES
    //
    //     Identical text, opposite answers.  So a require bounds the
    //     provider's position from above and pins nothing.

    fn require_edge(parent: &str, child: &str, at: u32) -> RunEdge {
        RunEdge {
            kind: RunEdgeKind::PackageRequire,
            ..edge(parent, child, at)
        }
    }

    /// TP: the provider's statements have run for every query after the
    /// `package require` — oracle case (1), `app::p -> P`.
    #[test]
    fn a_require_proves_the_provider_has_already_run() {
        let order = RunOrder::build(&[require_edge("app", "lib", 10)]);
        assert_eq!(
            order.has_run(point("lib", 30), point("app", 50)),
            Some(true),
            "`package require` at 10 leaves the provider loaded for the import at 50",
        );
        // …and the mirror: the requiring document's later statement did not
        // run before the provider's, which the same bound settles.
        assert_eq!(
            order.has_run(point("app", 50), point("lib", 30)),
            Some(false),
        );
    }

    /// TP: the provider's *transitive* files ride the same bound — a file the
    /// provider `source`s runs when the provider does.
    #[test]
    fn the_bound_covers_what_the_provider_sources() {
        let order = RunOrder::build(&[require_edge("app", "lib", 10), edge("lib", "deep", 5)]);
        assert_eq!(
            order.has_run(point("deep", 0), point("app", 50)),
            Some(true)
        );
        assert_eq!(
            order.has_run(point("app", 50), point("deep", 0)),
            Some(false)
        );
    }

    /// TP: one provider required from many files still ranks — the whole
    /// point of a bound rather than a position.  A `source` child entered
    /// twice is ambiguous; a `package require`d provider is not, because no
    /// require claims the provider ran *there*.
    #[test]
    fn many_requires_of_one_package_all_bound_it() {
        let order = RunOrder::build(&[
            require_edge("a", "lib", 10),
            require_edge("b", "lib", 10),
            require_edge("c", "lib", 10),
        ]);
        for doc in ["a", "b", "c"] {
            assert_eq!(
                order.has_run(point("lib", 30), point(doc, 50)),
                Some(true),
                "{doc}'s own require bounds the provider for {doc}'s later statements",
            );
        }
    }

    /// ABSTENTION 1 (`auto_path` is mutable) is the caller's — it never emits
    /// an edge for a package two documents provide — but the order must not
    /// invent one from the *other* files' requires either: a document with no
    /// require of its own is not ordered against the provider.
    #[test]
    fn a_document_that_does_not_require_the_package_is_not_ordered_by_it() {
        let order = RunOrder::build(&[require_edge("a", "lib", 10)]);
        assert_eq!(
            order.has_run(point("lib", 30), point("bystander", 50)),
            None
        );
        assert_eq!(
            order.cmp_run(point("lib", 30), point("bystander", 50)),
            None
        );
    }

    /// ABSTENTION 2 (CRITICAL — the package may already be loaded): a query
    /// written **above** the `package require` gets `None`, never
    /// `Some(false)`.
    ///
    /// Oracle case (2) above: `b.tcl`'s probe, written above its own require,
    /// answers YES because `a.tcl` loaded the package first — while the
    /// byte-identical probe in `a.tcl` answers NO.  A static `Some(false)`
    /// here would revoke a `namespace forget` / `-clear` that really had run.
    #[test]
    fn a_require_never_proves_the_provider_has_not_run_yet() {
        let order = RunOrder::build(&[require_edge("app", "lib", 10)]);
        assert_eq!(
            order.has_run(point("lib", 30), point("app", 5)),
            None,
            "another file may have required the package before app.tcl started",
        );
        assert_eq!(
            order.has_run(point("app", 5), point("lib", 30)),
            None,
            "…so app.tcl's earlier statement is not known to precede the provider either",
        );
        // A `source` edge in the same shape *does* answer both, which is what
        // makes this an abstention rather than a limitation of the machinery.
        let sourced = RunOrder::build(&[edge("app", "lib", 10)]);
        assert_eq!(
            sourced.has_run(point("lib", 30), point("app", 5)),
            Some(false)
        );
        assert_eq!(
            sourced.has_run(point("app", 5), point("lib", 30)),
            Some(true)
        );
    }

    /// TN: two documents each brought in by a `package require` are ranked
    /// against each other exactly as before — not at all.  Both sides are
    /// bounds, and two bounds settle nothing.
    #[test]
    fn two_package_loaded_documents_are_incomparable() {
        let order = RunOrder::build(&[
            require_edge("app", "one", 10),
            require_edge("app", "two", 20),
        ]);
        assert_eq!(order.has_run(point("one", 0), point("two", 0)), None);
        assert_eq!(order.has_run(point("two", 0), point("one", 0)), None);
        assert_eq!(order.cmp_run(point("one", 0), point("two", 0)), None);
        // The `source` shape of the same workspace *is* ordered — the
        // difference is the load event, not the file layout.
        let sourced = RunOrder::build(&[edge("app", "one", 10), edge("app", "two", 20)]);
        assert_eq!(
            sourced.has_run(point("one", 999), point("two", 0)),
            Some(true)
        );
    }

    /// TN: a document that is both `source`d and `package require`d has two
    /// incompatible stories — the `source` says "it ran exactly there", the
    /// require says "it had already run" — so it gets neither.
    #[test]
    fn a_document_both_sourced_and_required_is_ambiguous() {
        let order = RunOrder::build(&[edge("app", "lib", 90), require_edge("app", "lib", 10)]);
        assert_eq!(order.has_run(point("lib", 0), point("app", 50)), None);
        assert_eq!(order.has_run(point("app", 50), point("lib", 0)), None);
    }

    /// TP: the bound is judged by `in_effect_within` like any other statement
    /// — a `package require` at a document's load level counts for a query
    /// inside one of that document's proc bodies, however far below it is
    /// written, because the whole file loads before any body runs.
    #[test]
    fn a_body_local_query_observes_a_later_top_level_require() {
        let order = RunOrder::build(&[require_edge("app", "lib", 900)]);
        assert_eq!(
            order.has_run(point("lib", 5), body_point("app", 10, (0, 20))),
            Some(true),
        );
        // …but a require written inside that same body, after the query, is
        // an ordinary later statement of the running script — and this time
        // the abstention applies, because "not yet" is what a bound cannot
        // say.
        let order = RunOrder::build(&[RunEdge {
            kind: RunEdgeKind::PackageRequire,
            ..RunEdge {
                parent: "app".to_owned(),
                child: "lib".to_owned(),
                at: 15,
                enclosing_body: Some(tcl_lexer::Span::new(0, 20)),
                kind: RunEdgeKind::Source,
            }
        }]);
        assert_eq!(
            order.has_run(point("lib", 5), body_point("app", 10, (0, 20))),
            None,
        );
    }

    /// `cmp_run` inherits the same one-sidedness: the conflict rule may learn
    /// that a provider's import ran *first*, never that it ran *second*.
    #[test]
    fn cmp_run_reads_only_the_half_the_bound_supports() {
        let order = RunOrder::build(&[require_edge("app", "lib", 10)]);
        assert_eq!(
            order.cmp_run(point("lib", 30), point("app", 50)),
            Some(std::cmp::Ordering::Less),
            "the provider's whole body precedes a statement below the require",
        );
        assert_eq!(
            order.cmp_run(point("app", 50), point("lib", 30)),
            Some(std::cmp::Ordering::Greater),
        );
        // Above the require, neither direction is a fact.
        assert_eq!(order.cmp_run(point("lib", 30), point("app", 5)), None);
        assert_eq!(order.cmp_run(point("app", 5), point("lib", 30)), None);
    }

    // Resolving, per call, "is there any package edge at all / is this
    // document ambiguous / what is its root / does that root have require
    // sites" costs three hash probes on owned URI strings, on the hottest
    // path in the cross-document import walk.  All
    // four questions are fixed at build time, so they are one probe of
    // `package_alternatives`.  These pin that the folded answer is the same
    // answer, case by case, because a silent change here would move which
    // cross-document events the import tiers can rank.

    /// TP: a document a `package require` loads, and everything under it,
    /// gets the require sites; the require's own document does not.
    #[test]
    fn package_alternatives_cover_the_required_tree_and_nothing_else() {
        let order = RunOrder::build(&[require_edge("app", "lib", 10), edge("lib", "deep", 5)]);
        assert_eq!(order.alternatives(point("lib", 0)).len(), 1);
        assert_eq!(
            order.alternatives(point("deep", 0)).len(),
            1,
            "a file the provider sources rides the provider's bound",
        );
        assert!(
            order.alternatives(point("app", 0)).is_empty(),
            "the requiring document is not itself bounded by its own require",
        );
    }

    /// TN: no package edge anywhere leaves every document without
    /// alternatives — the shape the old `package_entries.is_empty()` fast
    /// path existed for, which must stay free of any behaviour change.
    #[test]
    fn a_workspace_with_no_package_edge_has_no_alternatives() {
        let order = RunOrder::build(&[edge("a", "b", 10), edge("b", "c", 5)]);
        for doc in ["a", "b", "c"] {
            assert!(order.alternatives(point(doc, 0)).is_empty(), "{doc}");
        }
    }

    /// FP guard: an **ambiguous** document must get no alternatives even
    /// though its root has require sites.  This was the `self.ambiguous`
    /// test that stood in front of the old lookup; folding it into the build
    /// must not lose it.  A document both `source`d and `package require`d is
    /// exactly that case.
    #[test]
    fn an_ambiguous_document_gets_no_alternatives() {
        let order = RunOrder::build(&[require_edge("app", "lib", 10), edge("other", "lib", 3)]);
        assert!(
            order.alternatives(point("lib", 0)).is_empty(),
            "both sourced and required: two incompatible stories, so neither",
        );
        // …and ambiguity is inherited, so a file below it is barred too.
        let order = RunOrder::build(&[
            require_edge("app", "lib", 10),
            edge("other", "lib", 3),
            edge("lib", "deep", 1),
        ]);
        assert!(order.alternatives(point("deep", 0)).is_empty());
    }

    /// TN: a document the graph has never heard of has no alternatives, and
    /// asking does not panic — the `map_or(p.uri, …)` fallback the old code
    /// took, restated as a plain miss.
    #[test]
    fn an_unknown_document_has_no_alternatives() {
        let order = RunOrder::build(&[require_edge("app", "lib", 10)]);
        assert!(order.alternatives(point("never-indexed", 0)).is_empty());
        assert!(RunOrder::default().alternatives(point("a", 0)).is_empty());
    }

    /// TP: many requires of one provider all reach it — the per-document
    /// fold shares one site vector rather than dropping the extras.
    #[test]
    fn every_require_site_survives_the_per_document_fold() {
        let order = RunOrder::build(&[
            require_edge("a", "lib", 10),
            require_edge("b", "lib", 20),
            require_edge("c", "lib", 30),
        ]);
        let sites = order.alternatives(point("lib", 0));
        assert_eq!(sites.len(), 3);
        let mut ats: Vec<u32> = sites.iter().map(|s| s.at).collect();
        ats.sort_unstable();
        assert_eq!(ats, vec![10, 20, 30]);
    }

    /// `trusted` is the whole rule in one place: an answer needs the side it
    /// would break on to be pinned.
    #[test]
    fn trust_needs_the_side_the_answer_rests_on() {
        let exact = Placed {
            at: 0,
            enclosing_body: None,
            exact: true,
        };
        let bound = Placed {
            exact: false,
            ..exact
        };
        assert!(RunOrder::trusted(true, exact, exact));
        assert!(RunOrder::trusted(false, exact, exact));
        // "a first" survives `a` sliding earlier…
        assert!(RunOrder::trusted(true, bound, exact));
        assert!(!RunOrder::trusted(false, bound, exact));
        // …"a not first" survives `b` sliding earlier.
        assert!(RunOrder::trusted(false, exact, bound));
        assert!(!RunOrder::trusted(true, exact, bound));
        // Two bounds settle nothing.
        assert!(!RunOrder::trusted(true, bound, bound));
        assert!(!RunOrder::trusted(false, bound, bound));
    }

    // `descendant_requires` — the up direction
    //
    // Oracle for the whole group, C Tcl 9.0.4:
    //
    // ```
    // % cat child.tcl
    // package require msgcat
    // % cat t1.tcl
    // puts [expr {[lsearch -exact [package names] msgcat] >= 0}]  ;# 0
    // source child.tcl
    // puts [package present msgcat]                               ;# 1.7.1
    // ```
    //
    // i.e. the package is absent before the `source` statement and present
    // after it — which is why these results carry a position.

    // Load-level `source` edges are built with the module-test `edge` helper
    // defined above for the `ancestor_prefer_latest_raised` group — same
    // shape, same meaning, so it is shared rather than duplicated.

    /// The reported shape: `main.tcl` sources `tkFile.tcl`, which
    /// requires Tk. Tk becomes available to `main` *at the source statement*.
    #[test]
    fn descendant_requires_carries_a_sourced_files_package_up() {
        let edges = vec![edge("main", "tk", 7)];
        let requires = reqs(&[("tk", &["Tk"])]);
        let got = descendant_requires("main", &edges, &requires);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].name, "Tk");
        assert_eq!(
            got[0].at, 7,
            "the package arrives at the `source` statement, not at offset 0",
        );
    }

    /// A file that sources something requiring nothing contributes nothing —
    /// the TN that keeps `source`-following from silencing W120 wholesale.
    #[test]
    fn descendant_requires_is_empty_for_a_sourced_file_with_no_requires() {
        let edges = vec![edge("main", "plain", 7)];
        let requires = reqs(&[("plain", &[])]);
        assert!(descendant_requires("main", &edges, &requires).is_empty());
    }

    /// Transitive: `main` → `mid` → `leaf`. `leaf`'s package reaches `main`,
    /// attributed to the position of *`main`'s own* `source`, since that is
    /// where the whole subtree enters `main`'s run.
    #[test]
    fn descendant_requires_is_transitive_and_placed_at_the_first_hop() {
        let edges = vec![edge("main", "mid", 12), edge("mid", "leaf", 400)];
        let requires = reqs(&[("leaf", &["Tk"])]);
        let got = descendant_requires("main", &edges, &requires);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].name, "Tk");
        assert_eq!(
            got[0].at, 12,
            "a transitive package enters at the outermost hop's position, not \
             at an offset that belongs to a different document",
        );
    }

    /// A cycle terminates instead of spinning, and still reports what it found.
    #[test]
    fn descendant_requires_terminates_on_a_cycle() {
        let edges = vec![edge("a", "b", 5), edge("b", "a", 9)];
        let requires = reqs(&[("b", &["Tk"])]);
        let got = descendant_requires("a", &edges, &requires);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].name, "Tk");
    }

    /// The target's *own* requires are the single-document question and are
    /// never reported here — otherwise a file would inherit from itself and
    /// the caller would double-count.
    #[test]
    fn descendant_requires_excludes_the_targets_own_requires() {
        let edges = vec![edge("main", "child", 7)];
        let requires = reqs(&[("main", &["http"]), ("child", &["Tk"])]);
        let got = descendant_requires("main", &edges, &requires);
        assert_eq!(
            got.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["Tk"]
        );
    }

    /// Only `source` edges are followed. A `package require` edge bounds when
    /// a provider ran but says nothing about *this* document acquiring it, so
    /// it must not contribute — otherwise every document requiring a package
    /// would inherit every other requirer's packages.
    #[test]
    fn descendant_requires_ignores_package_require_edges() {
        let mut e = edge("main", "provider", 7);
        e.kind = RunEdgeKind::PackageRequire;
        let requires = reqs(&[("provider", &["Tk"])]);
        assert!(descendant_requires("main", &[e], &requires).is_empty());
    }

    /// A file sourced from two places contributes at both positions, so a
    /// caller gating on offset gets the right answer at each.
    #[test]
    fn descendant_requires_reports_a_twice_sourced_file_at_both_positions() {
        let edges = vec![edge("main", "tk", 7), edge("main", "tk", 90)];
        let requires = reqs(&[("tk", &["Tk"])]);
        let got = descendant_requires("main", &edges, &requires);
        assert_eq!(
            got.iter().map(|p| p.at).collect::<Vec<_>>(),
            [7, 90],
            "{got:?}",
        );
    }
}

#[cfg(test)]
mod project_source_action_tests {
    use super::*;

    #[test]
    fn rename_association_requires_a_unique_declared_base_and_same_host_root() {
        let document = Path::new("/project/sub/main.tcl");
        let old = Path::new("/project/old.tcl");
        let one = project_source_association(document, &[], "../old.tcl", old).unwrap();
        assert_eq!(
            one.renamed_literal(Path::new("/project/new.tcl"))
                .as_deref(),
            Some("../new.tcl")
        );
        assert!(
            project_source_association(
                document,
                &[PathBuf::from("/project/other")],
                "../old.tcl",
                old
            )
            .is_none()
        );
        let duplicate = project_source_association(
            document,
            &[PathBuf::from("/project/sub")],
            "../old.tcl",
            old,
        )
        .unwrap();
        assert_eq!(duplicate, one);
        assert!(project_source_association(document, &[], "other.tcl", old).is_none());
        assert!(
            project_source_association(document, &[PathBuf::from("relative")], "../old.tcl", old)
                .is_none()
        );
        assert!(one.renamed_literal(Path::new("relative/new.tcl")).is_none());
        assert!(
            relative_source_path(Path::new("relative/new.tcl"), Path::new("relative")).is_none()
        );
        assert!(project_source_association(document, &[], "~/old.tcl", old).is_none());
        assert!(project_source_association(document, &[], "~other/old.tcl", old).is_none());
        assert!(project_source_association(document, &[], "", old).is_none());
        #[cfg(unix)]
        {
            assert!(
                project_source_association(
                    Path::new("//project/sub/main.tcl"),
                    &[],
                    "../old.tcl",
                    old
                )
                .is_none()
            );
            assert!(project_source_association(document, &[], "//project/old.tcl", old).is_none());
            assert!(
                one.renamed_literal(Path::new("//project/new.tcl"))
                    .is_none()
            );
        }
    }
}
#[cfg(test)]
mod original_source_navigation_order_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn point(uri: &str, at: u32) -> RunPoint<'_> {
        RunPoint {
            uri,
            at,
            enclosing_body: None,
        }
    }

    #[test]
    fn original_workspace_order_retains_conditional_purpose_without_tombstone_authority() {
        let source = "set before 1\nset after 2\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.dialect = "display-only-changed".into();
        let context = SourceOrderContext::capture(&analysis).unwrap();
        let order = RunOrder::for_source_navigation(&[], [("doc".into(), context)].into_iter());
        let before = point("doc", 0);
        let after = point(
            "doc",
            u32::try_from(source.find("set after").unwrap()).unwrap(),
        );
        // Two straight-line points of one document are ordered by its own
        // sequence. The legacy `has_run` and `cmp_run` stay unavailable for
        // workspace navigation.
        let relation = order.navigation_precedence(before, after).unwrap();
        assert_eq!(
            relation.purpose(),
            SourceNavigationOrderPurpose::DocumentSequence
        );
        assert_eq!(*relation.conditional_relation(), true);
        assert_eq!(relation.modeled_source_exclusion(), Some(true));
        assert_eq!(order.has_run(before, after), None);
        assert_eq!(order.cmp_run(before, after), None);
        let ranking = order.navigation_comparison(before, after).unwrap();
        assert_eq!(*ranking.conditional_relation(), std::cmp::Ordering::Less);
        assert_eq!(
            ranking.modeled_source_exclusion(),
            Some(std::cmp::Ordering::Less)
        );

        let mut events = [
            crate::namespace_import::ExportEvent {
                pattern: "p",
                clears: false,
                at: before,
            },
            crate::namespace_import::ExportEvent {
                pattern: "",
                clears: true,
                at: after,
            },
        ]
        .into_iter();
        assert!(crate::namespace_import::exported_at_import_site(
            &mut events,
            "p",
            &order,
            after
        ));
        let mut aliases = [
            crate::namespace_import::AliasEvent {
                kind: crate::namespace_import::AliasEventKind::Install,
                at: before,
            },
            crate::namespace_import::AliasEvent {
                kind: crate::namespace_import::AliasEventKind::Remove,
                at: after,
            },
        ]
        .into_iter();
        assert!(crate::namespace_import::alias_live_at(
            &mut aliases,
            &order,
            Some(after)
        ));
    }

    /// `pattern p` / `-clear` and install / remove event pairs at `first`
    /// and `second`: whether `p` is exported and the alias live at `query`.
    fn exported_and_live(
        order: &RunOrder,
        first: RunPoint<'_>,
        second: RunPoint<'_>,
        query: RunPoint<'_>,
    ) -> (bool, bool) {
        use crate::namespace_import::{AliasEvent, AliasEventKind, ExportEvent};
        let exports = [
            ExportEvent {
                pattern: "p",
                clears: false,
                at: first,
            },
            ExportEvent {
                pattern: "",
                clears: true,
                at: second,
            },
        ];
        let aliases = [
            AliasEvent {
                kind: AliasEventKind::Install,
                at: first,
            },
            AliasEvent {
                kind: AliasEventKind::Remove,
                at: second,
            },
        ];
        (
            crate::namespace_import::exported_at_import_site(
                &mut exports.into_iter(),
                "p",
                order,
                query,
            ),
            crate::namespace_import::alias_live_at(&mut aliases.into_iter(), order, Some(query)),
        )
    }

    #[test]
    fn document_end_follows_every_straight_removal_but_no_foreign_one() {
        let source = "set before 1\nset after 2\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let mut other = Analyser::new().analyse(source, "tcl8.6");
        other.dialect = "display-only-other".into();
        let order = RunOrder::for_source_navigation(
            &[],
            [
                (
                    "doc".into(),
                    SourceOrderContext::capture(&analysis).unwrap(),
                ),
                ("other".into(), SourceOrderContext::capture(&other).unwrap()),
            ]
            .into_iter(),
        );
        let before = point("doc", 0);
        let after = point(
            "doc",
            u32::try_from(source.find("set after").unwrap()).unwrap(),
        );
        let end = point("doc", u32::try_from(source.len()).unwrap());
        // The same-document clear and removal have run at the document end.
        assert_eq!(
            exported_and_live(&order, before, after, end),
            (false, false)
        );
        // One in another document is never ordered after this one's events.
        let foreign = point("other", after.at);
        assert!(
            order
                .navigation_precedence(before, foreign)
                .is_none_or(|relation| {
                    relation.purpose() == SourceNavigationOrderPurpose::Conditional
                        && relation.modeled_source_exclusion().is_none()
                })
        );
        assert_eq!(
            exported_and_live(&order, before, foreign, end),
            (true, true)
        );
        // With no query point the order stays conditional, as at a link.
        let mut aliases = [
            crate::namespace_import::AliasEvent {
                kind: crate::namespace_import::AliasEventKind::Install,
                at: before,
            },
            crate::namespace_import::AliasEvent {
                kind: crate::namespace_import::AliasEventKind::Remove,
                at: after,
            },
        ]
        .into_iter();
        assert!(crate::namespace_import::alias_live_at(
            &mut aliases,
            &order,
            None
        ));
    }

    fn sequence_purposes(
        source: &str,
    ) -> impl Fn(&str, &str) -> Option<SourceNavigationOrderPurpose> + '_ {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let context = SourceOrderContext::capture(&analysis).unwrap();
        let order = RunOrder::for_source_navigation(&[], [("doc".into(), context)].into_iter());
        let at = move |needle: &str| {
            if needle == "END" {
                return u32::try_from(source.len()).unwrap();
            }
            u32::try_from(source.find(needle).unwrap()).unwrap()
        };
        move |a: &str, b: &str| {
            order
                .navigation_precedence(point("doc", at(a)), point("doc", at(b)))
                .map(|relation| relation.purpose())
        }
    }

    const KNOWN: Option<SourceNavigationOrderPurpose> =
        Some(SourceNavigationOrderPurpose::DocumentSequence);

    #[test]
    fn document_sequence_orders_straight_line_bodies_and_withholds_branches() {
        // Only top-level unconditional order is known. A `namespace eval`
        // body is entered once, immediately; an `if` branch and a procedure
        // body are not straight-line sequence. A `return` inside a procedure
        // body is no early exit of the document.
        let source = "set before 1\nnamespace eval ::n {\n    set inner 2\n}\nif {$x} {set branch 3}\nproc q {} {set body 4; return}\nset after 6\n";
        let purpose = sequence_purposes(source);
        assert_eq!(purpose("set before", "set inner"), KNOWN);
        assert_eq!(purpose("set inner", "set after"), KNOWN);
        assert_eq!(purpose("set after", "END"), KNOWN);
        for withheld in ["set branch", "set body"] {
            assert_ne!(purpose(withheld, "set after"), KNOWN, "{withheld}");
            assert_ne!(purpose("set before", withheld), KNOWN, "{withheld}");
        }
        // `uplevel` selects its frame from a level word: never entered.
        let shifted = sequence_purposes("set before 1\nuplevel #0 {set shifted 2}\n");
        assert_ne!(shifted("set before", "set shifted"), KNOWN);
        assert_ne!(shifted("set shifted", "END"), KNOWN);
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let context = SourceOrderContext::capture(&analysis).unwrap();
        let order = RunOrder::for_source_navigation(&[], [("doc".into(), context)].into_iter());
        assert!(
            order
                .navigation_precedence(point("doc", 0), point("elsewhere", 0))
                .is_none()
        );
    }

    #[test]
    fn document_sequence_ends_at_a_possible_early_exit() {
        // tclsh 8.6 / 9.0: a top-level `return`, unconditional or in a taken
        // branch, stops the document; nothing after it is a known position
        // and neither is the document end.
        for source in [
            "set before 1\nreturn\nset after 2\n",
            "set before 1\nif {$x} return\nset after 2\n",
            "set before 1\nif {$x} {exit 1}\nset after 2\n",
        ] {
            let purpose = sequence_purposes(source);
            assert_ne!(purpose("set before", "set after"), KNOWN, "{source}");
            assert_ne!(purpose("set before", "END"), KNOWN, "{source}");
        }
    }

    #[test]
    fn document_end_is_after_normal_completion_only() {
        // tclsh 8.6 / 9.0: a sourced file or a user procedure doing
        // `return -code return`, or a top-level `break` in a file sourced
        // inside a loop, completes the load early and the later statement
        // never runs. A later position is still ordered (it is reached only
        // when control did pass); the document end is withheld.
        for source in [
            "set before 1\nsource y.tcl\nset after 2\n",
            "proc myreturn {} {return -code return}\nset before 1\nmyreturn\nset after 2\n",
            "set before 1\nbreak\nset after 2\n",
            // A script the inventory cannot see runs now: a lambda in a
            // variable, a command prefix, a doubly substituted expression.
            "set l {{} {return -code return}}\nset before 1\napply $l\nset after 2\n",
            "proc cmp {a b} {return -code return}\nset before 1\nlsort -command cmp {b a}\nset after 2\n",
            "set e {[return -code return]}\nset before 1\nexpr $e\nset after 2\n",
            // Math functions are commands: a user `::tcl::mathfunc`
            // procedure, also from a braced `if` condition, or an undefined
            // name reaching a user `unknown`.
            "proc ::tcl::mathfunc::f {x} {return -code return}\nset before 1\nexpr {f(1)}\nset after 2\n",
            "proc ::tcl::mathfunc::f {x} {return -code return}\nset before 1\nif {f(1)} {set y 1}\nset after 2\n",
            "proc unknown {args} {return -code return}\nset before 1\nexpr {nosuchfn(1)}\nset after 2\n",
        ] {
            let purpose = sequence_purposes(source);
            assert_eq!(purpose("set before", "set after"), KNOWN, "{source}");
            assert_ne!(purpose("set before", "END"), KNOWN, "{source}");
            assert_ne!(purpose("set after", "END"), KNOWN, "{source}");
        }
    }

    #[test]
    fn document_sequence_refuses_shadowed_or_namespace_local_entering_commands() {
        // tclsh 8.6 / 9.0: a namespace-local `proc eval` or `proc namespace`
        // swallows the body, so the forget never runs; neither body is
        // straight-line sequence.
        for source in [
            "namespace eval dst {proc eval args {}}\nnamespace eval dst {eval {namespace forget ::src::p}}\nset after 1\n",
            "namespace eval dst {proc namespace args {}}\nnamespace eval dst {namespace forget ::src::p}\nset after 1\n",
            "proc eval args {}\neval {namespace forget ::src::p}\nset after 1\n",
            // A renamed procedure takes the built-in's name without a
            // declaration of that name; the Realm barrier must refuse it.
            "proc myeval args {}\nrename eval {}\nrename myeval eval\neval {namespace forget ::src::p}\nset after 1\n",
        ] {
            let purpose = sequence_purposes(source);
            assert_ne!(purpose("namespace forget", "set after"), KNOWN, "{source}");
            assert_ne!(purpose("namespace forget", "END"), KNOWN, "{source}");
        }
    }

    #[test]
    fn original_source_order_context_keeps_full_input_and_missing_body_owners_unknown() {
        let source = "set value 1\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(SourceOrderContext::capture(&analysis).is_some());
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
        assert!(SourceOrderContext::capture(&changed).is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(SourceOrderContext::capture(&missing).is_none());
        let input = analysis.resolved_input.as_ref().unwrap();
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry(),
            input.lexer_config(),
        ));
        assert!(SourceOrderContext::capture(&foreign).is_none());
        let dynamic = "$head arbitrary\n";
        let dynamic_analysis = Analyser::new().analyse(dynamic, "tcl8.6");
        let context = SourceOrderContext::capture(&dynamic_analysis).unwrap();
        assert!(context.site(0).is_none());
        let order = RunOrder::for_source_navigation(&[], std::iter::empty());
        assert!(
            order
                .navigation_precedence(point("doc", 0), point("doc", 1))
                .is_none()
        );
        assert_eq!(order.has_run(point("doc", 0), point("doc", 1)), None);

        let source = "proc pending {} {set body 1}\nset root 2\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let body_at = u32::try_from(source.find("set body").unwrap()).unwrap();
        assert!(analysis.original_definition_body_span(body_at).is_some());
        let context = SourceOrderContext::capture(&analysis).unwrap();
        if let Some(site) = context.site(body_at) {
            assert_eq!(site.body, analysis.original_definition_body_span(body_at));
        }
        let original_body = analysis.original_definition_body_span(body_at);
        analysis.all_procs.clear();
        // Reporting-map deletion neither erases the independent original body
        // nor manufactures a source-global point from it.
        assert_eq!(
            analysis.original_definition_body_span(body_at),
            original_body
        );
        let context = SourceOrderContext::capture(&analysis).unwrap();
        if let Some(site) = context.site(body_at) {
            assert_eq!(site.body, original_body);
        }
    }

    #[test]
    fn original_source_graph_uses_real_points_and_preserves_cycle_and_re_source_refusal() {
        let parent = "set before 1\nsource child.tcl\nsource child.tcl\nset after 2\n";
        let child = "set child 1\nsource parent.tcl\n";
        let parent_analysis = Analyser::new().analyse(parent, "tcl8.6");
        let child_analysis = Analyser::new().analyse(child, "tcl8.6");
        let at = u32::try_from(parent.find("source child").unwrap()).unwrap();
        let contexts = || {
            [
                (
                    "parent".into(),
                    SourceOrderContext::capture(&parent_analysis).unwrap(),
                ),
                (
                    "child".into(),
                    SourceOrderContext::capture(&child_analysis).unwrap(),
                ),
            ]
            .into_iter()
        };
        let edge = RunEdge {
            parent: "parent".into(),
            child: "child".into(),
            at,
            enclosing_body: Some(tcl_lexer::Span::new(
                0,
                u32::try_from(parent.len()).unwrap(),
            )),
            kind: RunEdgeKind::Source,
        };
        let order = RunOrder::for_source_navigation(&[edge.clone()], contexts());
        // Supplied report body geometry cannot replace the genuine root point.
        assert_eq!(order.paths.get("child").unwrap()[0].enclosing_body, None);
        let relation = order
            .navigation_precedence(point("parent", 0), point("child", 0))
            .unwrap();
        assert_eq!(*relation.conditional_relation(), true);
        assert_eq!(relation.modeled_source_exclusion(), None);
        let re_source = RunEdge {
            at: u32::try_from(parent.rfind("source child").unwrap()).unwrap(),
            ..edge.clone()
        };
        let order = RunOrder::for_source_navigation(&[edge.clone(), re_source], contexts());
        assert!(
            order
                .navigation_precedence(point("parent", 0), point("child", 0))
                .is_none()
        );
        let cycle = RunEdge {
            parent: "child".into(),
            child: "parent".into(),
            at: u32::try_from(child.find("source parent").unwrap()).unwrap(),
            enclosing_body: None,
            kind: RunEdgeKind::Source,
        };
        let order = RunOrder::for_source_navigation(&[edge, cycle], contexts());
        assert!(
            order
                .navigation_comparison(point("parent", 0), point("child", 0))
                .is_none()
        );
    }
}
#[cfg(test)]
mod original_package_alternative_owner_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn point(uri: &str, at: u32) -> RunPoint<'_> {
        RunPoint {
            uri,
            at,
            enclosing_body: None,
        }
    }

    #[test]
    fn package_alternatives_cannot_replace_unowned_dynamic_deferred_or_missing_input_points() {
        let parent_source = "package require P\nset after 1\n";
        let parent = Analyser::new().analyse(parent_source, "tcl8.6");
        let query_at = u32::try_from(parent_source.find("set after").unwrap()).unwrap();
        let edge = RunEdge {
            parent: "parent".into(),
            child: "child".into(),
            at: 0,
            enclosing_body: None,
            kind: RunEdgeKind::PackageRequire,
        };
        let parent_context = SourceOrderContext::capture(&parent).unwrap();
        assert!(parent_context.edge_site(&edge).is_some());
        assert!(parent_context.site(query_at).is_some());

        let child_source = "set owned 1\npackage provide P 1\n$head arbitrary\n";
        let child = Analyser::new().analyse(child_source, "tcl8.6");
        let child_context = SourceOrderContext::capture(&child).unwrap();
        assert!(child_context.site(0).is_some());
        let dynamic_at = u32::try_from(child_source.find("$head").unwrap()).unwrap();
        assert!(child_context.site(dynamic_at).is_none());
        let order = RunOrder::for_source_navigation(
            &[edge.clone()],
            [
                ("parent".into(), parent_context.clone()),
                ("child".into(), child_context),
            ]
            .into_iter(),
        );
        let parent_point = point("parent", query_at);
        let owned = point("child", 0);
        assert!(!order.alternatives(owned).is_empty());
        let positive = order.navigation_precedence(owned, parent_point).unwrap();
        assert_eq!(*positive.conditional_relation(), true);
        assert_eq!(
            positive.purpose(),
            SourceNavigationOrderPurpose::Conditional
        );
        assert_eq!(positive.modeled_source_exclusion(), None);
        let dynamic = point("child", dynamic_at);
        assert!(order.navigation_precedence(dynamic, parent_point).is_none());
        assert!(order.navigation_precedence(parent_point, dynamic).is_none());
        assert!(order.navigation_comparison(dynamic, parent_point).is_none());
        assert!(order.navigation_comparison(parent_point, dynamic).is_none());

        let deferred_source = "package provide P 1\nproc pending {} {$head arbitrary}\n";
        let deferred = Analyser::new().analyse(deferred_source, "tcl8.6");
        let deferred_at = u32::try_from(deferred_source.find("$head").unwrap()).unwrap();
        assert!(
            deferred
                .original_definition_body_span(deferred_at)
                .is_some()
        );
        let context = SourceOrderContext::capture(&deferred).unwrap();
        assert!(context.site(deferred_at).is_none());
        let order = RunOrder::for_source_navigation(
            &[edge.clone()],
            [
                ("parent".into(), parent_context.clone()),
                ("child".into(), context),
            ]
            .into_iter(),
        );
        let unowned_body = point("child", deferred_at);
        assert!(!order.alternatives(unowned_body).is_empty());
        assert!(
            order
                .navigation_precedence(unowned_body, parent_point)
                .is_none()
        );
        assert!(
            order
                .navigation_precedence(parent_point, unowned_body)
                .is_none()
        );
        assert!(
            order
                .navigation_comparison(unowned_body, parent_point)
                .is_none()
        );
        assert!(
            order
                .navigation_comparison(parent_point, unowned_body)
                .is_none()
        );

        let mut missing = child.clone();
        missing.resolved_input = None;
        assert!(SourceOrderContext::capture(&missing).is_none());
        let order = RunOrder::for_source_navigation(
            &[edge],
            [("parent".into(), parent_context)].into_iter(),
        );
        // The actual require route remains. Missing original source transport
        // cannot become a point merely because its document has that route.
        assert!(!order.alternatives(owned).is_empty());
        assert!(order.navigation_precedence(owned, parent_point).is_none());
        assert!(order.navigation_precedence(parent_point, owned).is_none());
        assert!(order.navigation_comparison(owned, parent_point).is_none());
        assert!(order.navigation_comparison(parent_point, owned).is_none());
        assert_eq!(order.has_run(owned, parent_point), None);
        assert_eq!(order.cmp_run(owned, parent_point), None);
    }
}
