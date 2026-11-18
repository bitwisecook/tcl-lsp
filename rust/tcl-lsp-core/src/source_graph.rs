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

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::path::{Path, PathBuf};

mod package_requirements;
mod run_order;
mod sequence_capture;
mod source_paths;

pub use package_requirements::{
    PlacedRequire, PlacedRequirement, ancestor_prefer_latest_raised, ancestor_requirements,
    ancestor_requires, descendant_requirements, descendant_requires,
};
pub use run_order::RunOrder;
use sequence_capture::RegistryWordsSlot;
pub use source_paths::{
    ProjectSourceAssociation, conditional_project_source_target, project_source_association,
    relative_source_path, resolve_source_target, resolve_under,
};

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
            // A body run during its own invocation (`namespace eval`) is
            // ordered in place when the command is a straight-line sequence
            // position, as `ordered_point` already treats it.
            let straight = sequence
                .as_ref()
                .is_some_and(|sequence| sequence.contains(at));
            if body.is_none()
                && structure
                    .caller_context_unavailable_bodies
                    .iter()
                    .any(|span| {
                        span.start() <= at
                            && at < span.end()
                            && !(straight && structure.invocation_scoped_bodies.contains(span))
                    })
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_package_alternative_owner_tests;
#[cfg(test)]
mod original_source_navigation_order_tests;
#[cfg(test)]
mod project_source_action_tests;
