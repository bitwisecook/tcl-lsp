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

//! The import edge: the per-import-site export snapshot that decides whether
//! it is installed, and the lifecycle log that decides whether it is still
//! there — the two decisions both wildcard-import tiers ask.
//!
//! `namespace import ::src::*` does not create a standing subscription to
//! `::src`'s export list — it creates *aliases*, once, for the names exported
//! at the moment the import runs. Everything after that is history:
//!
//! - **`namespace export -clear` after an import does not revoke the alias.**
//!   Oracle (tclsh 8.6.14 / 9.0.4):
//!   ```tcl
//!   namespace eval ::src { proc p {} {return P}; namespace export p }
//!   namespace eval ::dst { namespace import ::src::* }
//!   namespace eval ::src { namespace export -clear }
//!   ::dst::p     ;# → P   (and `info commands ::dst::*` still lists ::dst::p)
//!   ```
//! - **Exporting a name after an import does not add it retroactively.**
//!   Oracle:
//!   ```tcl
//!   namespace eval ::src { proc p {} {return P} }
//!   namespace eval ::dst { namespace import ::src::* }
//!   namespace eval ::src { namespace export p }
//!   ::dst::p     ;# → invalid command name "::dst::p"
//!   ```
//!   A *second* `namespace import ::src::*` written after the export does
//!   pick `p` up — each import site takes its own snapshot.
//!
//! Joining every import against a namespace's *final* export set gets both
//! directions wrong (issue #1027): it drops an alias the program still has,
//! and invents one the program never had.
//!
//! # The model
//!
//! `namespace export` records are an ordered, append-only event log —
//! [`tcl_compiler::signature_scan::types::SignatureNamespaceExport`], one
//! entry per pattern word plus a `clears` tombstone per `-clear`. The
//! question this module answers is *"was NAME exported by this namespace at
//! the point import I ran?"*, which is the export half of the same
//! "what does this name hold at this point?" timeline
//! `tcl_compiler::analyser::indirection` answers for `rename` / `interp
//! alias`, and under the same order-gating discipline.
//!
//! # One entry point, two tiers
//!
//! [`exported_at_import_site`] is the single decision function. The
//! same-document resolver (`definition.rs` / `references.rs`) and the
//! cross-document one (`workspace_index.rs`) both call it, so they cannot
//! disagree about the semantics — the same "one entry point" discipline
//! `qualified_variable_cell_at` uses. It gates **every** `namespace import`,
//! glob or exact: an exact `namespace import ::src::p` of a name `::src` has
//! not exported silently installs nothing in real Tcl (oracle: `info commands
//! ::dst::*` stays empty and no error is raised), so the cross-document
//! `WorkspaceCommandLink` an exact import produces is only live while this
//! function admits it (`WorkspaceIndex::live_command_links`).
//!
//! The primitive "had this event already run when the import executed?" is no
//! longer tier-specific either: both tiers hand these functions the workspace's
//! [`crate::source_graph::RunOrder`] and the point being asked about, and the
//! relation answers all three cases from the same rule:
//!
//! - Same document: `analyser::indirection::in_effect_within`, so a declaration
//!   inside a proc body is judged by the same load-order rule the rename /
//!   alias timeline uses. (The in-document tier reaches it by giving every
//!   event the document's own key; `in_effect(analysis, …)` is that same rule
//!   with the body span read out of the analysis.) A plain offset comparison is
//!   **not** good enough and was a real tier divergence (PR #1102 review): an
//!   import written inside a body genuinely observes a top-level export
//!   written later in the same file, because the whole file loads before any
//!   body runs — oracle (tclsh 8.6.14 / 9.0.4), `namespace eval ::app {proc
//!   setup {} {namespace import ::mymod::*}; proc run {} {helper}}` followed
//!   by `namespace eval ::mymod {namespace export helper}`, then
//!   `::app::setup; ::app::run` → `HELP`.
//! - **Different file from the import: ordered only where the `source` graph
//!   proves an order** ([`crate::source_graph::RunOrder`], issue #1104 item
//!   3). Sourcing a file inlines its whole body at the `source` statement's
//!   position, so the DFS of the `source` forest *is* the run order and two
//!   events in one tree are genuinely comparable. Everywhere else — different
//!   trees, a re-sourced file, a `source` cycle — nothing is ordered and this
//!   module abstains toward the safer side *for navigation*: an unrankable
//!   pattern still counts (keep answering go-to-definition / find-references,
//!   the pre-#1027 behaviour), while an unrankable `-clear` does **not**
//!   revoke anything (revoking on a guess would silently drop real
//!   references).
//!
//!   Two events written in the **same** foreign file were always ordered
//!   against each other even before the graph existed — a file's statements
//!   run consecutively — so a `namespace export p` followed by a `namespace
//!   export -clear` in one other file now revokes, where the old
//!   one-`Option<u32>`-per-event encoding could only say "unordered" about
//!   both and kept the export. What stays unknown is where *this* import sits
//!   relative to that file, and the pattern-versus-tombstone question does not
//!   depend on it.
//!
//! # The edge has a lifetime, not just a birth
//!
//! Installing the alias is only the first event on it (issue #1103). The same
//! ordered-log discipline answers the second question — *does this namespace
//! still hold the alias here?* — in [`alias_live_at`]:
//!
//! - **`namespace forget` removes it.** `namespace forget ::src::p` (or the
//!   simple form `namespace forget p`) makes a later bare call `invalid
//!   command name`.
//! - **Deleting the source command removes it.** The alias holds the command
//!   *object*, so `rename ::src::p {}` kills `::dst::p` as well — while a
//!   plain `rename ::src::p ::src::pp` leaves it working (only `namespace
//!   origin` moves) and a redefinition of the source is seen straight through
//!   the link. That asymmetry is exactly
//!   `tcl_compiler::analyser::indirection`'s
//!   rename-captures-object-identity rule, which is why this module models
//!   the deletion and not the rename.
//! - **A conflict means it was never installed at all.** Without `-force`,
//!   importing onto a name the target namespace already holds raises `can't
//!   import command "p": already exists`; with `-force` it silently replaces
//!   the local command, and a later bare call reaches the source
//!   (`namespace origin` → `::src::p`). The install/no-install half is
//!   modelled; the error's control-flow consequence (the rest of that script
//!   never running) deliberately is not.
//! - **Chains are edges of edges.** `::A` importing `::B::*` where `::B`
//!   imported `::C::*` makes `::A::p` run `::C`'s body, and a forget anywhere
//!   along the chain kills the whole thing. The tiers follow the chain while
//!   every hop is provable, bounded by the same
//!   `indirection::MAX_COMMAND_NAME_HOPS` cap the rename / alias walk uses.
//!
//! Every one of those rows is oracle-confirmed byte-identically on tclsh
//! 9.0.4 and 8.6.14; the transcripts sit on the individual items.

use crate::source_graph::{RunOrder, RunPoint};
use tcl_syntax::glob::string_match;

/// What is known about "had `source_ns` exported this name when the import
/// ran?" — three-valued, because *absence of an export record* is only
/// evidence where the export records themselves are visible.
///
/// The distinction the single-document tier could not draw before issue #1116
/// item 1: it saw one file, so "no export here" and "the export is in another
/// file" were the same observation. Both consumers of a verdict abstain, but
/// in opposite directions, because the questions are opposite:
///
/// | verdict | "what does this call reach?" | "did `-force` delete the local command?" |
/// |---|---|---|
/// | [`Exported`](Self::Exported) | the import installed — resolve through it | yes |
/// | [`NotExported`](Self::NotExported) | it installed nothing — do not | no |
/// | [`Unknown`](Self::Unknown) | do not resolve (no evidence to resolve *on*) | yes — answering with a command the import may have deleted is worse than answering nothing |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportVerdict {
    /// A `namespace export` covering the name had run at the import site.
    Exported,
    /// The exporting namespace's export list is visible **and** nothing in it
    /// covers the name at that point — a fact, not a gap.
    NotExported,
    /// Nothing can be said: the namespace's exports are somewhere this
    /// answerer cannot see (another file, an installed package, a document
    /// outside the workspace).
    Unknown,
}

/// Whole-program export knowledge, as the *single-document* tier needs it.
///
/// # Why the in-document tier needs an oracle at all (issue #1116 item 1)
///
/// `namespace import -force` deletes the importing namespace's own command of
/// that name — but only for names the source namespace actually exports. A
/// document can hold the source namespace's procs, its importing namespace,
/// the `-force` import and the call, and still not hold the one `namespace
/// export` that decides the question. Two whole programs whose *single
/// document* is byte-identical then disagree (tclsh 8.6.14 / 9.0.4, verbatim):
///
/// ```text
/// # shared_main.tcl — identical in both programs
/// namespace eval ::src { proc helper {} {return SRC}
///                        proc other {} {return OTHER}
///                        namespace export other }
/// namespace eval ::app { proc helper {} {return LOCAL} }
/// namespace eval ::app { namespace import -force ::src::* }
/// namespace eval ::app { puts [helper] }
///
/// with `namespace eval ::src {namespace export helper}` sourced first:
///     call -> SRC     origin -> ::src::helper
/// without it:
///     call -> LOCAL   origin -> ::app::helper
/// ```
///
/// No rule reading only that document can separate those, so the decision has
/// to import whole-program evidence. The oracle is that evidence and nothing
/// else: it answers one question, at one point in the timeline, and is free to
/// answer [`ExportVerdict::Unknown`].
///
/// Optional everywhere it is threaded — a caller with no workspace index (a
/// single-document unit test, the `tcl` CLI, an unindexed buffer) supplies
/// none and keeps the document-only rule, which is this trait's whole reason
/// for returning a verdict rather than a `bool`.
///
/// Implemented by `crate::workspace_index::NamespaceExportSnapshot`.
pub trait NamespaceExportOracle {
    /// Had `source_ns` exported `name` by the time the `namespace import` at
    /// `import_site` ran, as the whole program sees it?
    ///
    /// `import_site` carries the importing document's real URI, so the
    /// implementation can rank the workspace's export events against it with
    /// the `source`-graph run order — an export the graph proves runs *after*
    /// this import is not retroactive, exactly as within one document.
    fn exported_at(&self, source_ns: &str, name: &str, import_site: RunPoint<'_>) -> ExportVerdict;
}

/// One `namespace export` event as either tier sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportEvent<'a> {
    /// The exported glob pattern exactly as written, relative to the
    /// exporting namespace. Empty (and ignored) when [`Self::clears`].
    pub pattern: &'a str,
    /// `true` for a `namespace export -clear` tombstone.
    pub clears: bool,
    /// Where the event sits in the workspace's execution timeline. Ordering it
    /// against anything else is [`RunOrder`]'s job — within one document
    /// always, and across documents wherever the `source` graph proves a load
    /// order (issue #1104 item 3).
    pub at: RunPoint<'a>,
}

/// Whether `later` is **known** to have run strictly after `earlier`.
///
/// The one direction both folds below need, stated once over
/// [`RunOrder::has_run`] so the gate ("had this run by the query point?") and
/// the ranking ("which of these two ran later?") cannot mean different things.
/// `has_run(later, earlier) == Some(false)` reads "at the moment `earlier`
/// ran, `later` had not" — which is precisely "`later` came after".
///
/// Incomparable answers `false`: a removal that cannot be placed revokes
/// nothing, the abstain-toward-answering direction this whole family takes.
fn ran_after(order: &RunOrder, later: RunPoint<'_>, earlier: RunPoint<'_>) -> bool {
    order.has_run(later, earlier) == Some(false)
}

/// Whether an event had already run at the query point, abstaining toward
/// *yes* when the two have no static order.
fn in_effect_at(order: &RunOrder, event: RunPoint<'_>, query: Option<RunPoint<'_>>) -> bool {
    query.is_none_or(|q| order.has_run(event, q).unwrap_or(true))
}

/// Whether the source namespace exported `name` **at the point the import
/// ran** — the per-import-site snapshot (issue #1027).
///
/// `events` are that namespace's export events, in any order; `order` is the
/// workspace's [`RunOrder`] and `import_site` the position of the import being
/// judged.
///
/// The rule, applied to the events in effect at `import_site`:
///
/// `name` is exported when some visible pattern event glob-matches it (Tcl's
/// own `Tcl_StringMatch` semantics, via [`tcl_syntax::glob::string_match`] —
/// export patterns are glob patterns: `namespace export get*` exports `getX`
/// and not `setX`, and `namespace export {p[ab]}` exports `pa`/`pb` and not
/// `pc`, oracle-verified) and **no visible `-clear` tombstone is known to have
/// run after it**.
///
/// That is a *latest-wins* rule stated over a partial order rather than a
/// total one, which is what lets cross-document events participate at all
/// (design §6.1): two events the gate admits still have to be ranked against
/// each other, and a partial order has no `max`. A pattern survives unless
/// some tombstone provably follows it, so an unrankable tombstone revokes
/// nothing — the same abstention the old unordered encoding expressed, now
/// stated once instead of as a special case.
///
/// An export pattern is a name pattern, not a reference to a command, so
/// nothing here requires the command to exist: `namespace export p` written
/// before `proc p` still exports `p` (oracle-verified). Whether the command
/// exists is the caller's own lookup, which it does afterwards.
///
/// One pass to collect the visible events, then an `O(patterns × clears)`
/// check over them — both sets are a namespace's own `namespace export`
/// statements, so in practice one or two of each, and the `Vec`s stay
/// unallocated when nothing is visible.
///
/// Stated over [`exports_in_effect`], which is the whole rule *except* the
/// name: `name` enters only at the final glob match. Callers that ask about
/// many names at one import site should hoist that half themselves rather
/// than call this in a loop — see [`exports_in_effect`] and issue #1297.
#[must_use]
pub fn exported_at_import_site(
    events: &mut dyn Iterator<Item = ExportEvent<'_>>,
    name: &str,
    order: &RunOrder,
    import_site: RunPoint<'_>,
) -> bool {
    exports_in_effect(events, order, import_site)
        .iter()
        .any(|pattern| string_match(pattern, name))
}

/// The export **patterns** in force at `import_site` — [`exported_at_import_site`]
/// with the name left out, so a caller asking about many names at one site
/// pays for the timeline once instead of once per name.
///
/// Every visible non-`-clear` pattern that no visible tombstone is *known* to
/// follow, in recorded order. That is exactly the set
/// [`exported_at_import_site`] would glob-match `name` against: the name only
/// ever filtered which patterns were collected, never which tombstones applied
/// (a `-clear` clears the slot, not one name), so pulling it out of the walk
/// cannot change an answer.
///
/// # Why this exists
///
/// The workspace tier ranks every export row against the import with
/// [`RunOrder`], which is a `HashMap` walk over document URIs. Asking per
/// *call site* made that walk run once per (invocation × in-scope import ×
/// export row) — 26 s to answer one `textDocument/references` on the #1181
/// corpus, where 88 load-level `namespace import`s meet 38 `namespace export`
/// rows (issue #1297). An import's site does not move between edits, so the
/// timeline half is decided once per recorded import at index-build time and
/// only the glob match stays on the per-call path.
#[must_use]
pub fn exports_in_effect<'e>(
    events: &mut dyn Iterator<Item = ExportEvent<'e>>,
    order: &RunOrder,
    import_site: RunPoint<'_>,
) -> Vec<&'e str> {
    let mut patterns: Vec<(&'e str, RunPoint<'e>)> = Vec::new();
    let mut clears: Vec<RunPoint<'e>> = Vec::new();
    for ev in events {
        if !in_effect_at(order, ev.at, Some(import_site)) {
            continue;
        }
        if ev.clears {
            clears.push(ev.at);
        } else {
            patterns.push((ev.pattern, ev.at));
        }
    }
    patterns
        .into_iter()
        .filter(|&(_, p)| !clears.iter().any(|&c| ran_after(order, c, p)))
        .map(|(pattern, _)| pattern)
        .collect()
}

/// The **load-order** position of a statement in its own document: `false`
/// (load level) sorts before `true` (inside a proc/class body), and within
/// each tier by byte offset. `load_order(a) < load_order(b)` reads "a had
/// definitely already run when b ran".
///
/// # Why the import-conflict rule needs its own order
///
/// [`alias_live_at`]'s query point is a *call*, and its ordering primitive is
/// [`tcl_compiler::analyser::indirection::in_effect_within`], which is
/// deliberately **lenient**: an event in some other proc's body counts as
/// having run, because whether that proc was called is not statically
/// decidable. For a removal that is the safe direction — the conservative
/// answer is that the alias may be gone.
///
/// The conflict question inverts the sign. A conflict *cancels an install*, so
/// the same leniency invents conflicts and drops aliases the program really
/// has: applied to
/// `namespace eval ::dst {proc p {} {namespace import ::B::x}}` followed by a
/// top-level `namespace import ::A::*`, it makes each import cancel the other
/// and the name resolves nowhere, where real Tcl binds `::A::x`. What the
/// conflict rule needs is the *strict* reading — "did this definitely already
/// run" — which is exactly load order:
///
/// | event | query | ran before? |
/// |---|---|---|
/// | load level | load level | by offset |
/// | load level | inside a body | **yes**, wherever written — the file loads first |
/// | inside a body | load level | **no** — that body may never run |
/// | same body | same body | by offset |
///
/// Two statements in *different* bodies have no static order at all; they keep
/// their offset order, which is what both tiers did before and what the
/// enclosing abstentions already assume.
///
/// Both tiers order conflicts through this one function — the same-document
/// resolver by sorting its slot log with it (`definition::slot_events`), the
/// cross-document one by comparing two sites with it
/// (`WildcardImportIndex::conflicting_alias_at`) — so neither can drift from
/// the other, exactly as they share [`exported_at_import_site`] and
/// [`alias_live_at`].
///
/// Oracle for every row, byte-identical on tclsh 8.6.14 and 9.0.4:
///
/// ```tcl
/// namespace eval ::A { proc x {} {return A}; namespace export x }
/// namespace eval ::B { proc x {} {return B}; namespace export x }
/// namespace eval ::dst { proc p {} { namespace import ::B::x } }
/// namespace eval ::dst { namespace import ::A::* }
/// namespace origin ::dst::x   ;# → ::A::x
/// ::dst::p                    ;# → can't import command "x": already exists
/// namespace origin ::dst::x   ;# → ::A::x   (unchanged)
/// ```
#[must_use]
pub fn load_order(at: u32, body_local: bool) -> (bool, u32) {
    (body_local, at)
}

/// What one lifecycle event does to an import edge — see [`AliasEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasEventKind {
    /// A `namespace import` **installed** the alias. The caller has already
    /// applied the export snapshot ([`exported_at_import_site`]) and the
    /// pattern match, so this event says only "an import ran here".
    Install,
    /// The alias was **removed**: a `namespace forget`, or the deletion of
    /// the source command the alias holds (`rename ::src::p {}`).
    Remove,
}

/// One event on an import edge's own lifecycle, as either tier sees it.
///
/// The installing/removing counterpart of [`ExportEvent`], on the same kind of
/// ordered, append-only log and read by the same "latest visible event wins"
/// rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AliasEvent<'a> {
    /// What the event does.
    pub kind: AliasEventKind,
    /// Where the event sits in the workspace's execution timeline; ordered
    /// against the query point and against the other events by [`RunOrder`].
    pub at: RunPoint<'a>,
}

/// Whether the importing namespace still holds a live imported alias at the
/// query point, given every lifecycle event that bears on it (issue #1103).
///
/// `namespace import` does not create a permanent name. `namespace forget`
/// takes the alias away again, and so does deleting the source command the
/// alias holds — both oracle-confirmed byte-identically on tclsh 9.0.4 and
/// 8.6.14:
///
/// ```tcl
/// namespace eval ::src { proc p {} {return P}; namespace export p }
/// namespace eval ::dst { namespace import ::src::* }
/// namespace eval ::dst { p }                        ;# → P
/// namespace eval ::dst { namespace forget ::src::p }
/// namespace eval ::dst { p }                        ;# → invalid command name "p"
/// ```
///
/// ```tcl
/// namespace eval ::src { proc p {} {return P}; namespace export p }
/// namespace eval ::dst { namespace import ::src::* }
/// rename ::src::p {}
/// ::dst::p                                          ;# → invalid command name "::dst::p"
/// ```
///
/// A *re*name is deliberately **not** a removal: the alias holds the command
/// object, not the name, so `rename ::src::p ::src::pp` leaves `::dst::p`
/// working (only `namespace origin ::dst::p` moves, to `::src::pp`), and a
/// redefinition of the source is seen straight through the link. That is the
/// same rename-captures-object-identity rule
/// [`tcl_compiler::analyser::indirection`] already applies to `rename` /
/// `interp alias`, which is why this module carries no rename event kind.
///
/// # The rule
///
/// Symmetric with [`exported_at_import_site`], and for the same reason: an
/// install survives unless some in-effect [`AliasEventKind::Remove`] is *known*
/// to have run after it, and an install after a removal reinstates the alias (a
/// re-import after a forget genuinely does — oracle: re-running `namespace
/// import ::src::*` after a `namespace forget` makes the bare call work again).
/// Events the [`RunOrder`] cannot rank abstain toward *answering*: an
/// unrankable install counts, an unrankable removal revokes nothing, exactly as
/// an unrankable `-clear` does not revoke an export.
///
/// # What the order gates
///
/// [`RunOrder::has_run`] decides, for each event, whether it had already run at
/// `query` — and it is applied to **every** event, install and removal alike.
/// Both tiers pass the same relation, so "has run by here" cannot mean two
/// things; within one document it *is*
/// [`tcl_compiler::analyser::indirection::in_effect_within`], the primitive the
/// `rename` / `interp alias` timeline uses.
///
/// `query` is `None` for a caller with no query point of its own — the
/// whole-index liveness mask over exact import links
/// (`WildcardImportIndex::link_alias_live`), which asks "does this alias exist
/// for navigation at all". Every event then counts as having run, and the only
/// ordering left is each removal's position relative to the install.
///
/// Gating the *install* is what makes a bare call written **before** its own
/// `namespace import` stop resolving through it (issue #1104 item 1). Oracle
/// (tclsh 8.6.14 / 9.0.4, byte-identical):
///
/// ```tcl
/// namespace eval ::src { proc p {} {return P}; namespace export p }
/// namespace eval ::dst { p }                    ;# → invalid command name "p"
/// namespace eval ::dst { namespace import ::src::* }
/// namespace eval ::dst { p }                    ;# → P
/// ```
///
/// That gate is *not* a plain offset comparison, and must not be reduced to
/// one: a call inside a proc body is not ordered against a top-level import of
/// the same file at all, because the whole file loads — running every
/// top-level statement, imports included — before any body runs.
/// [`RunOrder::has_run`] states exactly that rule, which is why both tiers pass
/// it rather than `at < call_off`. A tier that compared offsets alone would
/// drop every proc body calling a name imported further down its own file —
/// the overwhelmingly common shape in real code.
///
/// With no install in effect the answer is `false`: nothing had put the alias
/// there yet. Callers that have already proven an import matches (pattern,
/// export snapshot, conflict rule) pass it as an [`AliasEventKind::Install`].
///
/// The same shape and the same cost as [`exported_at_import_site`], which it
/// runs beside: one pass to collect the visible events, then an
/// `O(installs × removals)` check over two sets that hold a handful of
/// elements each.
#[must_use]
pub fn alias_live_at(
    events: &mut dyn Iterator<Item = AliasEvent<'_>>,
    order: &RunOrder,
    query: Option<RunPoint<'_>>,
) -> bool {
    let mut installs: Vec<RunPoint<'_>> = Vec::new();
    let mut removals: Vec<RunPoint<'_>> = Vec::new();
    for ev in events {
        if !in_effect_at(order, ev.at, query) {
            continue;
        }
        match ev.kind {
            AliasEventKind::Install => installs.push(ev.at),
            AliasEventKind::Remove => removals.push(ev.at),
        }
    }
    installs
        .iter()
        .any(|&i| !removals.iter().any(|&r| ran_after(order, r, i)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_graph::RunEdge;

    /// The document every ordered event in these tests lives in — the
    /// single-document case both tiers spend almost all their time in.
    const DOC: &str = "file:///dst.tcl";
    /// A second document with no `source` path to [`DOC`]: unrankable.
    const OTHER: &str = "file:///other.tcl";

    fn at(uri: &str, off: u32) -> RunPoint<'_> {
        RunPoint {
            uri,
            at: off,
            enclosing_body: None,
        }
    }

    fn ev(pattern: &str, off: u32) -> ExportEvent<'_> {
        ExportEvent {
            pattern,
            clears: false,
            at: at(DOC, off),
        }
    }

    fn clear(off: u32) -> ExportEvent<'static> {
        ExportEvent {
            pattern: "",
            clears: true,
            at: at(DOC, off),
        }
    }

    fn foreign(pattern: &str) -> ExportEvent<'_> {
        ExportEvent {
            pattern,
            clears: false,
            at: at(OTHER, 0),
        }
    }

    /// No `source` statement anywhere — the workspace shape that must behave
    /// exactly as the pre-#1104-item-3 tiers did.
    fn unlinked() -> RunOrder {
        RunOrder::default()
    }

    #[test]
    fn export_before_import_is_visible() {
        let mut evs = [ev("p", 10)].into_iter();
        assert!(exported_at_import_site(
            &mut evs,
            "p",
            &unlinked(),
            at(DOC, 20)
        ));
    }

    #[test]
    fn direction_b_export_after_import_is_not_retroactive() {
        // `namespace export p` at 30, import at 20 → not yet exported.
        let mut evs = [ev("p", 30)].into_iter();
        assert!(!exported_at_import_site(
            &mut evs,
            "p",
            &unlinked(),
            at(DOC, 20)
        ));
    }

    #[test]
    fn direction_a_clear_after_import_does_not_revoke() {
        // export p @10, import @20, `-clear` @30.
        let mut evs = [ev("p", 10), clear(30)].into_iter();
        assert!(exported_at_import_site(
            &mut evs,
            "p",
            &unlinked(),
            at(DOC, 20)
        ));
    }

    #[test]
    fn clear_before_import_does_revoke() {
        let mut evs = [ev("p", 10), clear(15)].into_iter();
        assert!(!exported_at_import_site(
            &mut evs,
            "p",
            &unlinked(),
            at(DOC, 20)
        ));
    }

    #[test]
    fn clear_then_add_on_the_same_call_keeps_the_new_pattern() {
        // `namespace export a b; namespace export -clear p` → exactly `p`.
        let evs = [ev("a", 5), ev("b", 7), clear(20), ev("p", 27)];
        assert!(exported_at_import_site(
            &mut evs.into_iter(),
            "p",
            &unlinked(),
            at(DOC, 40)
        ));
        assert!(!exported_at_import_site(
            &mut evs.into_iter(),
            "a",
            &unlinked(),
            at(DOC, 40)
        ));
    }

    #[test]
    fn exports_are_additive_across_calls() {
        let evs = [ev("a", 5), ev("b", 20)];
        assert!(exported_at_import_site(
            &mut evs.into_iter(),
            "a",
            &unlinked(),
            at(DOC, 40)
        ));
        assert!(exported_at_import_site(
            &mut evs.into_iter(),
            "b",
            &unlinked(),
            at(DOC, 40)
        ));
    }

    #[test]
    fn a_second_import_sees_the_later_export() {
        let events = [ev("p", 10), clear(30), ev("p", 37), ev("q", 39)];
        // First import at 20: `p` yes, `q` no.
        assert!(exported_at_import_site(
            &mut events.into_iter(),
            "p",
            &unlinked(),
            at(DOC, 20)
        ));
        assert!(!exported_at_import_site(
            &mut events.into_iter(),
            "q",
            &unlinked(),
            at(DOC, 20)
        ));
        // Second import at 50: both.
        assert!(exported_at_import_site(
            &mut events.into_iter(),
            "p",
            &unlinked(),
            at(DOC, 50)
        ));
        assert!(exported_at_import_site(
            &mut events.into_iter(),
            "q",
            &unlinked(),
            at(DOC, 50)
        ));
    }

    #[test]
    fn glob_patterns_use_tcl_semantics() {
        let events = [ev("get*", 10)];
        assert!(exported_at_import_site(
            &mut events.into_iter(),
            "getX",
            &unlinked(),
            at(DOC, 20)
        ));
        assert!(!exported_at_import_site(
            &mut events.into_iter(),
            "setX",
            &unlinked(),
            at(DOC, 20)
        ));
        let cc = [ev("p[ab]", 10)];
        assert!(exported_at_import_site(
            &mut cc.into_iter(),
            "pa",
            &unlinked(),
            at(DOC, 20)
        ));
        assert!(!exported_at_import_site(
            &mut cc.into_iter(),
            "pc",
            &unlinked(),
            at(DOC, 20)
        ));
    }

    #[test]
    fn an_unrankable_pattern_counts_and_an_unrankable_clear_does_not_revoke() {
        // Export declared in another file with no `source` path to this one:
        // ordering unknown, so it still resolves…
        let mut evs = [foreign("p")].into_iter();
        assert!(exported_at_import_site(
            &mut evs,
            "p",
            &unlinked(),
            at(DOC, 0)
        ));
        // …and a `-clear` from a *third* file cannot silently drop it.
        let mut evs = [
            foreign("p"),
            ExportEvent {
                pattern: "",
                clears: true,
                at: at("file:///third.tcl", 0),
            },
        ]
        .into_iter();
        assert!(exported_at_import_site(
            &mut evs,
            "p",
            &unlinked(),
            at(DOC, 0)
        ));
    }

    #[test]
    fn an_ordered_clear_does_not_revoke_an_unrankable_export() {
        // The `-clear` is ordered before the import, but the surviving
        // pattern comes from a file whose load order is unknown — it may well
        // have run after. Navigation keeps answering.
        let mut evs = [clear(5), foreign("p")].into_iter();
        assert!(exported_at_import_site(
            &mut evs,
            "p",
            &unlinked(),
            at(DOC, 20)
        ));
    }

    /// TN, issue #1104 item 3. Two export events written in **one** other file
    /// were always ordered against each other — a file's statements run
    /// consecutively — even though where that file sits relative to this
    /// import is unknown. The old one-`Option<u32>`-per-event encoding could
    /// only call both "unordered" and kept the export.
    #[test]
    fn a_clear_revokes_a_pattern_written_above_it_in_the_same_foreign_file() {
        let evs = [
            ExportEvent {
                pattern: "p",
                clears: false,
                at: at(OTHER, 10),
            },
            ExportEvent {
                pattern: "",
                clears: true,
                at: at(OTHER, 30),
            },
        ];
        assert!(!exported_at_import_site(
            &mut evs.into_iter(),
            "p",
            &unlinked(),
            at(DOC, 0)
        ));
        // …and the other way round the pattern survives, as it must.
        let evs = [
            ExportEvent {
                pattern: "",
                clears: true,
                at: at(OTHER, 10),
            },
            ExportEvent {
                pattern: "p",
                clears: false,
                at: at(OTHER, 30),
            },
        ];
        assert!(exported_at_import_site(
            &mut evs.into_iter(),
            "p",
            &unlinked(),
            at(DOC, 0)
        ));
    }

    #[test]
    fn no_events_means_not_exported() {
        let mut evs = [].into_iter();
        assert!(!exported_at_import_site(
            &mut evs,
            "p",
            &unlinked(),
            at(DOC, 20)
        ));
    }

    // ---- the `source` graph orders what the file boundary did not ---------

    /// The two documents the sourced-order tests use, and the order that
    /// sequences them: `app.tcl` sources `exp.tcl` and then `imp.tcl`.
    fn sourced_order() -> RunOrder {
        RunOrder::build(&[
            RunEdge {
                parent: "file:///app.tcl".to_owned(),
                child: "file:///exp.tcl".to_owned(),
                at: 10,
                enclosing_body: None,
                kind: crate::source_graph::RunEdgeKind::Source,
            },
            RunEdge {
                parent: "file:///app.tcl".to_owned(),
                child: DOC.to_owned(),
                at: 20,
                enclosing_body: None,
                kind: crate::source_graph::RunEdgeKind::Source,
            },
        ])
    }

    /// TP (issue #1104 item 3): a `namespace export` in a file the entry point
    /// sources **before** the importing file counts, as it always did — but
    /// now as a *fact* rather than an abstention.
    #[test]
    fn an_export_in_an_earlier_sourced_file_counts() {
        let mut evs = [ExportEvent {
            pattern: "p",
            clears: false,
            at: at("file:///exp.tcl", 5),
        }]
        .into_iter();
        assert!(exported_at_import_site(
            &mut evs,
            "p",
            &sourced_order(),
            at(DOC, 0)
        ));
    }

    /// TN (CRITICAL, issue #1104 item 3): the same export in a file sourced
    /// **after** the importing one has not run when the import executes, so it
    /// exports nothing to it. This is the leniency the source graph removes.
    #[test]
    fn an_export_in_a_later_sourced_file_is_not_retroactive() {
        // Reverse the order: imp.tcl at 10, exp.tcl at 20.
        let order = RunOrder::build(&[
            RunEdge {
                parent: "file:///app.tcl".to_owned(),
                child: DOC.to_owned(),
                at: 10,
                enclosing_body: None,
                kind: crate::source_graph::RunEdgeKind::Source,
            },
            RunEdge {
                parent: "file:///app.tcl".to_owned(),
                child: "file:///exp.tcl".to_owned(),
                at: 20,
                enclosing_body: None,
                kind: crate::source_graph::RunEdgeKind::Source,
            },
        ]);
        let mut evs = [ExportEvent {
            pattern: "p",
            clears: false,
            at: at("file:///exp.tcl", 5),
        }]
        .into_iter();
        assert!(!exported_at_import_site(&mut evs, "p", &order, at(DOC, 0)));
    }

    /// TN: a `-clear` in an earlier-sourced file now revokes an export from a
    /// file sourced before *it* — the second row of #1104 item 3's table.
    #[test]
    fn a_clear_in_a_later_sourced_file_revokes_an_earlier_export() {
        let order = RunOrder::build(&[
            RunEdge {
                parent: "file:///app.tcl".to_owned(),
                child: "file:///exp.tcl".to_owned(),
                at: 10,
                enclosing_body: None,
                kind: crate::source_graph::RunEdgeKind::Source,
            },
            RunEdge {
                parent: "file:///app.tcl".to_owned(),
                child: "file:///clr.tcl".to_owned(),
                at: 20,
                enclosing_body: None,
                kind: crate::source_graph::RunEdgeKind::Source,
            },
            RunEdge {
                parent: "file:///app.tcl".to_owned(),
                child: DOC.to_owned(),
                at: 30,
                enclosing_body: None,
                kind: crate::source_graph::RunEdgeKind::Source,
            },
        ]);
        let evs = [
            ExportEvent {
                pattern: "p",
                clears: false,
                at: at("file:///exp.tcl", 5),
            },
            ExportEvent {
                pattern: "",
                clears: true,
                at: at("file:///clr.tcl", 5),
            },
        ];
        assert!(!exported_at_import_site(
            &mut evs.into_iter(),
            "p",
            &order,
            at(DOC, 0)
        ));
        // …and with the import sourced *between* the two, the snapshot it took
        // still holds — the `-clear` had not run yet.
        let order = RunOrder::build(&[
            RunEdge {
                parent: "file:///app.tcl".to_owned(),
                child: "file:///exp.tcl".to_owned(),
                at: 10,
                enclosing_body: None,
                kind: crate::source_graph::RunEdgeKind::Source,
            },
            RunEdge {
                parent: "file:///app.tcl".to_owned(),
                child: DOC.to_owned(),
                at: 20,
                enclosing_body: None,
                kind: crate::source_graph::RunEdgeKind::Source,
            },
            RunEdge {
                parent: "file:///app.tcl".to_owned(),
                child: "file:///clr.tcl".to_owned(),
                at: 30,
                enclosing_body: None,
                kind: crate::source_graph::RunEdgeKind::Source,
            },
        ]);
        assert!(exported_at_import_site(
            &mut evs.into_iter(),
            "p",
            &order,
            at(DOC, 0)
        ));
    }

    // ---- load order, for the conflict rule --------------------------------

    #[test]
    fn load_order_is_the_strict_did_this_already_run_relation() {
        const LOAD: bool = false;
        const BODY: bool = true;
        let ran_before =
            |e: (u32, bool), q: (u32, bool)| load_order(e.0, e.1) < load_order(q.0, q.1);
        // Two load-level statements: plain offset order.
        assert!(ran_before((10, LOAD), (20, LOAD)));
        assert!(!ran_before((20, LOAD), (10, LOAD)));
        // A load-level statement runs before *any* body, however far below it
        // is written — the whole file loads before any body runs.
        assert!(ran_before((900, LOAD), (10, BODY)));
        assert!(ran_before((10, LOAD), (900, BODY)));
        // …and a body-local statement never counts as having run at load
        // level, because that body may never be entered. This is where the
        // lenient `in_effect_within` differs, and why the conflict rule
        // cannot use it: it would invent a conflict and drop a real alias.
        assert!(!ran_before((10, BODY), (900, LOAD)));
        assert!(tcl_compiler::analyser::indirection::in_effect_within(
            10, 900, None
        ));
        // Inside one body the offsets are genuine execution order again.
        assert!(ran_before((10, BODY), (20, BODY)));
        assert!(!ran_before((20, BODY), (10, BODY)));
        // Nothing runs before itself — which is what excludes an import from
        // conflicting with itself, at either level.
        assert!(!ran_before((10, LOAD), (10, LOAD)));
        assert!(!ran_before((10, BODY), (10, BODY)));
    }

    // ---- the import edge's own lifecycle (issue #1103) -------------------

    fn install(off: u32) -> AliasEvent<'static> {
        AliasEvent {
            kind: AliasEventKind::Install,
            at: at(DOC, off),
        }
    }

    fn remove(off: u32) -> AliasEvent<'static> {
        AliasEvent {
            kind: AliasEventKind::Remove,
            at: at(DOC, off),
        }
    }

    fn unrankable(kind: AliasEventKind) -> AliasEvent<'static> {
        AliasEvent {
            kind,
            at: at(OTHER, 0),
        }
    }

    #[test]
    fn an_import_with_no_removal_is_live() {
        let mut evs = [install(10)].into_iter();
        assert!(alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
    }

    #[test]
    fn nothing_installed_means_no_alias() {
        let mut evs = [remove(10)].into_iter();
        assert!(!alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
    }

    #[test]
    fn a_forget_after_the_import_kills_the_alias() {
        // import @10, `namespace forget` @15, call @20.
        let mut evs = [install(10), remove(15)].into_iter();
        assert!(!alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
    }

    #[test]
    fn a_forget_after_the_call_leaves_it_alive() {
        // import @10, call @20, `namespace forget` @30 — the call runs first.
        let mut evs = [install(10), remove(30)].into_iter();
        assert!(alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
    }

    #[test]
    fn a_re_import_after_a_forget_reinstates_the_alias() {
        let mut evs = [install(10), remove(15), install(17)].into_iter();
        assert!(alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
    }

    #[test]
    fn the_latest_removal_wins_over_an_earlier_install() {
        // Two forgets; the alias is dead from the first one that follows the
        // last install.
        let mut evs = [install(10), remove(12), remove(15)].into_iter();
        assert!(!alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
    }

    #[test]
    fn an_install_later_than_the_call_has_not_run_yet() {
        // #1104 item 1: a bare call written *before* its own `namespace
        // import` reaches nothing (oracle in the function's docs), so an
        // install at 30 with the call at 20 does not install here.
        let mut evs = [install(30)].into_iter();
        assert!(!alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
        // The earlier install is the one that counts, and a removal between
        // the two still kills it.
        let mut evs = [install(5), install(30)].into_iter();
        assert!(alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
        let mut evs = [install(5), remove(10), install(30)].into_iter();
        assert!(!alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
    }

    #[test]
    fn a_body_local_call_observes_a_later_top_level_install() {
        // The install gate is the load-order rule, not `at < call_off`: a call
        // inside a proc body at 20 sees a top-level import at 30, because the
        // whole file loads before any body runs.
        let call = RunPoint {
            uri: DOC,
            at: 20,
            enclosing_body: Some(tcl_lexer::Span::new(15, 25)),
        };
        let mut evs = [install(30)].into_iter();
        assert!(alias_live_at(&mut evs, &unlinked(), Some(call)));
        // …but an install written inside that same body after the call is an
        // ordinary later statement of the running script.
        let mut evs = [install(22)].into_iter();
        assert!(!alias_live_at(&mut evs, &unlinked(), Some(call)));
    }

    #[test]
    fn an_unrankable_removal_revokes_nothing() {
        // A `namespace forget` in another file with no `source` path to this
        // one: no static load order, so navigation keeps answering rather than
        // dropping a real alias.
        let mut evs = [install(10), unrankable(AliasEventKind::Remove)].into_iter();
        assert!(alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
    }

    #[test]
    fn an_unrankable_install_counts() {
        let mut evs = [unrankable(AliasEventKind::Install)].into_iter();
        assert!(alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
        // …and survives an ordered removal, for the same reason an ordered
        // `-clear` cannot revoke an unrankable export.
        let mut evs = [unrankable(AliasEventKind::Install), remove(5)].into_iter();
        assert!(alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
    }

    #[test]
    fn no_events_at_all_means_no_alias() {
        let mut evs = [].into_iter();
        assert!(!alias_live_at(&mut evs, &unlinked(), Some(at(DOC, 20))));
    }

    /// TP: with no query point every event counts as having run, and the only
    /// ordering left is the removal's position relative to the install — the
    /// whole-index liveness mask over exact import links.
    #[test]
    fn without_a_query_point_only_the_install_removal_order_matters() {
        let mut evs = [install(10), remove(30)].into_iter();
        assert!(!alias_live_at(&mut evs, &unlinked(), None));
        let mut evs = [install(30), remove(10)].into_iter();
        assert!(alias_live_at(&mut evs, &unlinked(), None));
        // A removal that cannot be ranked against the install revokes nothing.
        let mut evs = [install(10), unrankable(AliasEventKind::Remove)].into_iter();
        assert!(alias_live_at(&mut evs, &unlinked(), None));
    }

    /// TN (CRITICAL, #1104 item 3 / #1116 item 6): `source lib.tcl` followed
    /// by `namespace forget ::lib::p` beside it — the genuinely-sequenced
    /// idiom that got *both* halves of the abstention wrong. The install from
    /// `lib.tcl` counts (it always did), and now the forget written after the
    /// `source` revokes it.
    #[test]
    fn a_forget_after_the_source_statement_revokes_the_sourced_install() {
        let order = RunOrder::build(&[RunEdge {
            parent: "file:///app.tcl".to_owned(),
            child: "file:///lib.tcl".to_owned(),
            at: 10,
            enclosing_body: None,
            kind: crate::source_graph::RunEdgeKind::Source,
        }]);
        let install_in_lib = AliasEvent {
            kind: AliasEventKind::Install,
            at: at("file:///lib.tcl", 5),
        };
        let forget_in_app = AliasEvent {
            kind: AliasEventKind::Remove,
            at: at("file:///app.tcl", 20),
        };
        let call = at("file:///app.tcl", 30);
        let mut evs = [install_in_lib, forget_in_app].into_iter();
        assert!(!alias_live_at(&mut evs, &order, Some(call)));
        // …and a call written *between* the two still resolves.
        let mut evs = [install_in_lib, forget_in_app].into_iter();
        assert!(alias_live_at(
            &mut evs,
            &order,
            Some(at("file:///app.tcl", 15))
        ));
    }

    // Issue #1297: `exported_at_import_site` is now stated over
    // `exports_in_effect`, which is the whole rule minus the name.  The
    // workspace tier decides that half once per recorded import instead of
    // once per (invocation x import x export row).  These pin that the split
    // is exact: whatever `exports_in_effect` keeps must glob-match to the
    // same verdict the one-shot function gives, over every shape the
    // timeline can produce.

    /// The two must agree for every (event set, name) pair, including the
    /// cases where the *timeline* is what decides — a `-clear` that provably
    /// follows, one that does not, and a foreign event that cannot be ranked
    /// at all.  A disagreement here would change what a `namespace import`
    /// covers, silently.
    #[test]
    fn exports_in_effect_is_exported_at_import_site_without_the_name() {
        let order = unlinked();
        let site = at(DOC, 100);
        let cases: Vec<Vec<ExportEvent<'_>>> = vec![
            vec![],
            vec![ev("p", 10)],
            vec![ev("*", 10)],
            vec![ev("get*", 10)],
            vec![ev("p[ab]", 10)],
            // A tombstone that provably follows the pattern revokes it.
            vec![ev("p", 10), clear(20)],
            // …one written before it does not.
            vec![clear(10), ev("p", 20)],
            // An event after the import site is not visible at all.
            vec![ev("p", 200)],
            vec![ev("p", 10), ev("q", 20)],
            // A foreign event cannot be ranked, so it revokes nothing.
            vec![ev("p", 10), foreign("q")],
            vec![foreign("p"), clear(20)],
            vec![ev("p", 10), clear(20), ev("p", 30)],
        ];
        for events in cases {
            for name in ["p", "q", "pa", "pc", "getX", "setX", ""] {
                let mut for_one = events.clone().into_iter();
                let one_shot = exported_at_import_site(&mut for_one, name, &order, site);
                let mut for_split = events.clone().into_iter();
                let split = exports_in_effect(&mut for_split, &order, site)
                    .iter()
                    .any(|pattern| string_match(pattern, name));
                assert_eq!(
                    one_shot, split,
                    "split disagreed for name={name:?} over {events:?}",
                );
            }
        }
    }

    /// TP/TN on the surviving set itself, so a regression shows up as the
    /// wrong *patterns* rather than only as a wrong verdict for some name.
    #[test]
    fn exports_in_effect_keeps_exactly_the_untombstoned_visible_patterns() {
        let order = unlinked();
        // TP: visible, no tombstone after it.
        let mut evs = [ev("a", 10), ev("b", 20)].into_iter();
        assert_eq!(
            exports_in_effect(&mut evs, &order, at(DOC, 100)),
            vec!["a", "b"],
        );
        // TN: a `-clear` that provably follows takes both away.
        let mut evs = [ev("a", 10), ev("b", 20), clear(30)].into_iter();
        assert!(exports_in_effect(&mut evs, &order, at(DOC, 100)).is_empty());
        // FN guard: a re-export after the tombstone survives it.
        let mut evs = [ev("a", 10), clear(20), ev("b", 30)].into_iter();
        assert_eq!(exports_in_effect(&mut evs, &order, at(DOC, 100)), vec!["b"],);
        // FP guard: nothing written after the import site is visible.
        let mut evs = [ev("a", 200)].into_iter();
        assert!(exports_in_effect(&mut evs, &order, at(DOC, 100)).is_empty());
    }
}
