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
//! What *is* tier-specific is the primitive
//! "had this event already run when the import executed?", which the caller
//! supplies as `visible`:
//!
//! - Same document: `analyser::indirection::in_effect`, so a declaration
//!   inside a proc body is judged by the same load-order rule the rename /
//!   alias timeline uses.
//! - Cross-document, same file as the import:
//!   `analyser::indirection::in_effect_within` — the *identical* rule, stated
//!   over the two facts the index stores per row (the import's offset and the
//!   innermost proc/class body containing it). A plain offset comparison is
//!   **not** good enough and was a real tier divergence (PR #1102 review): an
//!   import written inside a body genuinely observes a top-level export
//!   written later in the same file, because the whole file loads before any
//!   body runs — oracle (tclsh 8.6.14 / 9.0.4), `namespace eval ::app {proc
//!   setup {} {namespace import ::mymod::*}; proc run {} {helper}}` followed
//!   by `namespace eval ::mymod {namespace export helper}`, then
//!   `::app::setup; ::app::run` → `HELP`.
//! - **Different file from the import: not ordered at all.** Which file loads
//!   first is not a static fact, so such an event is passed with
//!   [`ExportEvent::at`] `None`, and this module abstains toward the safer
//!   side *for navigation*: an unordered pattern still counts (keep answering
//!   go-to-definition / find-references, the pre-#1027 behaviour), while an
//!   unordered `-clear` does **not** revoke anything (revoking on a guess
//!   would silently drop real references).
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

use tcl_syntax::glob::string_match;

/// One `namespace export` event as either tier sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportEvent<'a> {
    /// The exported glob pattern exactly as written, relative to the
    /// exporting namespace. Empty (and ignored) when [`Self::clears`].
    pub pattern: &'a str,
    /// `true` for a `namespace export -clear` tombstone.
    pub clears: bool,
    /// Byte offset of the event *when it is ordered relative to the import
    /// site* — i.e. when both are in the same document. `None` when the two
    /// sit in different files, whose relative load order is not a static
    /// fact.
    pub at: Option<u32>,
}

/// Whether the source namespace exported `name` **at the point the import
/// ran** — the per-import-site snapshot (issue #1027).
///
/// `events` are that namespace's export events, in any order. `visible`
/// decides, for an ordered event at the given offset, whether it had already
/// run when the import executed; see the module docs for why that primitive is
/// the caller's and everything else is not.
///
/// The rule, applied to the events `visible` admits:
///
/// 1. Take the latest visible `-clear` tombstone, if any.
/// 2. `name` is exported when some visible pattern event *after* that
///    tombstone glob-matches it (Tcl's own `Tcl_StringMatch` semantics, via
///    [`tcl_syntax::glob::string_match`] — export patterns are glob patterns:
///    `namespace export get*` exports `getX` and not `setX`, and
///    `namespace export {p[ab]}` exports `pa`/`pb` and not `pc`,
///    oracle-verified).
/// 3. Failing that, an *unordered* pattern event (a different file from the
///    import) may still match — unordered `-clear`s revoke nothing.
///
/// An export pattern is a name pattern, not a reference to a command, so
/// nothing here requires the command to exist: `namespace export p` written
/// before `proc p` still exports `p` (oracle-verified). Whether the command
/// exists is the caller's own lookup, which it does afterwards.
///
/// Single pass, no sort, no allocation — "some matching pattern sits after the
/// latest `-clear`" is decided by comparing the *latest matching* pattern's
/// offset with the latest tombstone's, which needs only two running maxima.
/// This runs inside the cross-document find-references loop, once per
/// candidate import per invocation, where the workspace tier already had to
/// hoist an index out (see
/// `WorkspaceIndex::resolve_wildcard_import_indexed`) to keep that path off
/// the profiler.
#[must_use]
pub fn exported_at_import_site(
    events: &mut dyn Iterator<Item = ExportEvent<'_>>,
    name: &str,
    visible: &dyn Fn(u32) -> bool,
) -> bool {
    // Latest visible tombstone, and the latest visible pattern event that
    // matches `name`.
    let mut cleared_through: Option<u32> = None;
    let mut latest_match: Option<u32> = None;
    let mut unordered_match = false;
    for ev in events {
        let Some(at) = ev.at else {
            unordered_match |= !ev.clears && string_match(ev.pattern, name);
            continue;
        };
        if !visible(at) {
            continue;
        }
        if ev.clears {
            cleared_through = cleared_through.max(Some(at));
        } else if string_match(ev.pattern, name) {
            latest_match = latest_match.max(Some(at));
        }
    }
    let ordered_match =
        latest_match.is_some_and(|m| cleared_through.is_none_or(|cleared| m > cleared));
    ordered_match || unordered_match
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
pub struct AliasEvent {
    /// What the event does.
    pub kind: AliasEventKind,
    /// Byte offset of the event *when it is ordered relative to the query
    /// point* — i.e. when both are in the same document. `None` when the two
    /// sit in different files, whose relative load order is not a static
    /// fact.
    pub at: Option<u32>,
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
/// Symmetric with [`exported_at_import_site`], and for the same reason: the
/// latest in-effect [`AliasEventKind::Remove`] wins over every install before
/// it, and an install *after* that removal reinstates the alias (a re-import
/// after a forget genuinely does — oracle: re-running `namespace import
/// ::src::*` after a `namespace forget` makes the bare call work again).
/// Unordered events — from a different file, where no static load order
/// exists — abstain toward *answering*: an unordered install counts, an
/// unordered removal revokes nothing, exactly as an unordered `-clear` does
/// not revoke an export.
///
/// # What `removal_in_effect` does and does not gate
///
/// `removal_in_effect` decides, for a **removal** at the given offset,
/// whether it had already run at the query point. Both tiers pass the
/// order-gating primitive they already use for the export snapshot
/// ([`tcl_compiler::analyser::indirection::in_effect`] /
/// [`tcl_compiler::analyser::indirection::in_effect_within`]), so "has run by
/// here" cannot mean two things.
///
/// It deliberately does **not** filter installs. Whether a bare call written
/// *before* its own `namespace import` should stop resolving is a separate,
/// still-open leniency (issue #1104 item 1: the workspace tier has no
/// call-site body span to apply the identical rule with, so gating only
/// in-document would make the two tiers disagree — the divergence the shared
/// decision function exists to prevent). Installs still carry their offsets,
/// because a removal revokes only the installs *before* it; they are simply
/// not dropped for being later than the call.
///
/// With no install at all the answer is `false`: nothing put the alias there.
/// Callers that have already proven an import matches (pattern, export
/// snapshot, conflict rule) pass it as an [`AliasEventKind::Install`].
///
/// Single pass, two running maxima, no allocation — the same shape and the
/// same cost as [`exported_at_import_site`], which it runs beside.
#[must_use]
pub fn alias_live_at(
    events: &mut dyn Iterator<Item = AliasEvent>,
    removal_in_effect: &dyn Fn(u32) -> bool,
) -> bool {
    let mut installed: Option<u32> = None;
    let mut removed: Option<u32> = None;
    let mut unordered_install = false;
    for ev in events {
        let Some(at) = ev.at else {
            unordered_install |= ev.kind == AliasEventKind::Install;
            continue;
        };
        match ev.kind {
            AliasEventKind::Install => installed = installed.max(Some(at)),
            AliasEventKind::Remove => {
                if removal_in_effect(at) {
                    removed = removed.max(Some(at));
                }
            }
        }
    }
    let ordered_live = installed.is_some_and(|i| removed.is_none_or(|r| i > r));
    ordered_live || unordered_install
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(pattern: &str, at: u32) -> ExportEvent<'_> {
        ExportEvent {
            pattern,
            clears: false,
            at: Some(at),
        }
    }

    fn clear(at: u32) -> ExportEvent<'static> {
        ExportEvent {
            pattern: "",
            clears: true,
            at: Some(at),
        }
    }

    fn unordered(pattern: &str) -> ExportEvent<'_> {
        ExportEvent {
            pattern,
            clears: false,
            at: None,
        }
    }

    /// `visible` for an import at `import_at`: plain textual order.
    fn before(import_at: u32) -> impl Fn(u32) -> bool {
        move |at| at <= import_at
    }

    #[test]
    fn export_before_import_is_visible() {
        let mut evs = [ev("p", 10)].into_iter();
        assert!(exported_at_import_site(&mut evs, "p", &before(20)));
    }

    #[test]
    fn direction_b_export_after_import_is_not_retroactive() {
        // `namespace export p` at 30, import at 20 → not yet exported.
        let mut evs = [ev("p", 30)].into_iter();
        assert!(!exported_at_import_site(&mut evs, "p", &before(20)));
    }

    #[test]
    fn direction_a_clear_after_import_does_not_revoke() {
        // export p @10, import @20, `-clear` @30.
        let mut evs = [ev("p", 10), clear(30)].into_iter();
        assert!(exported_at_import_site(&mut evs, "p", &before(20)));
    }

    #[test]
    fn clear_before_import_does_revoke() {
        let mut evs = [ev("p", 10), clear(15)].into_iter();
        assert!(!exported_at_import_site(&mut evs, "p", &before(20)));
    }

    #[test]
    fn clear_then_add_on_the_same_call_keeps_the_new_pattern() {
        // `namespace export a b; namespace export -clear p` → exactly `p`.
        let mut evs = [ev("a", 5), ev("b", 7), clear(20), ev("p", 27)].into_iter();
        assert!(exported_at_import_site(&mut evs.clone(), "p", &before(40)));
        assert!(!exported_at_import_site(&mut evs, "a", &before(40)));
    }

    #[test]
    fn exports_are_additive_across_calls() {
        let mut evs = [ev("a", 5), ev("b", 20)].into_iter();
        assert!(exported_at_import_site(&mut evs.clone(), "a", &before(40)));
        assert!(exported_at_import_site(&mut evs, "b", &before(40)));
    }

    #[test]
    fn a_second_import_sees_the_later_export() {
        let events = [ev("p", 10), clear(30), ev("p", 37), ev("q", 39)];
        // First import at 20: `p` yes, `q` no.
        assert!(exported_at_import_site(
            &mut events.into_iter(),
            "p",
            &before(20)
        ));
        assert!(!exported_at_import_site(
            &mut events.into_iter(),
            "q",
            &before(20)
        ));
        // Second import at 50: both.
        assert!(exported_at_import_site(
            &mut events.into_iter(),
            "p",
            &before(50)
        ));
        assert!(exported_at_import_site(
            &mut events.into_iter(),
            "q",
            &before(50)
        ));
    }

    #[test]
    fn glob_patterns_use_tcl_semantics() {
        let events = [ev("get*", 10)];
        assert!(exported_at_import_site(
            &mut events.into_iter(),
            "getX",
            &before(20)
        ));
        assert!(!exported_at_import_site(
            &mut events.into_iter(),
            "setX",
            &before(20)
        ));
        let cc = [ev("p[ab]", 10)];
        assert!(exported_at_import_site(
            &mut cc.into_iter(),
            "pa",
            &before(20)
        ));
        assert!(!exported_at_import_site(
            &mut cc.into_iter(),
            "pc",
            &before(20)
        ));
    }

    #[test]
    fn unordered_pattern_counts_and_unordered_clear_does_not_revoke() {
        // Export declared in another file: ordering unknown, so it still
        // resolves…
        let mut evs = [unordered("p")].into_iter();
        assert!(exported_at_import_site(&mut evs, "p", &before(0)));
        // …and a `-clear` from another file cannot silently drop it.
        let mut evs = [
            unordered("p"),
            ExportEvent {
                pattern: "",
                clears: true,
                at: None,
            },
        ]
        .into_iter();
        assert!(exported_at_import_site(&mut evs, "p", &before(0)));
    }

    #[test]
    fn an_ordered_clear_does_not_revoke_an_unordered_export() {
        // The `-clear` is ordered before the import, but the surviving
        // pattern comes from a file whose load order is unknown — it may well
        // have run after. Navigation keeps answering.
        let mut evs = [clear(5), unordered("p")].into_iter();
        assert!(exported_at_import_site(&mut evs, "p", &before(20)));
    }

    #[test]
    fn no_events_means_not_exported() {
        let mut evs = [].into_iter();
        assert!(!exported_at_import_site(&mut evs, "p", &before(20)));
    }

    // ---- the import edge's own lifecycle (issue #1103) -------------------

    fn install(at: u32) -> AliasEvent {
        AliasEvent {
            kind: AliasEventKind::Install,
            at: Some(at),
        }
    }

    fn remove(at: u32) -> AliasEvent {
        AliasEvent {
            kind: AliasEventKind::Remove,
            at: Some(at),
        }
    }

    fn unordered_event(kind: AliasEventKind) -> AliasEvent {
        AliasEvent { kind, at: None }
    }

    /// `removal_in_effect` for a call at `call_at`: plain textual order.
    fn ran_before(call_at: u32) -> impl Fn(u32) -> bool {
        move |at| at < call_at
    }

    #[test]
    fn an_import_with_no_removal_is_live() {
        let mut evs = [install(10)].into_iter();
        assert!(alias_live_at(&mut evs, &ran_before(20)));
    }

    #[test]
    fn nothing_installed_means_no_alias() {
        let mut evs = [remove(10)].into_iter();
        assert!(!alias_live_at(&mut evs, &ran_before(20)));
    }

    #[test]
    fn a_forget_after_the_import_kills_the_alias() {
        // import @10, `namespace forget` @15, call @20.
        let mut evs = [install(10), remove(15)].into_iter();
        assert!(!alias_live_at(&mut evs, &ran_before(20)));
    }

    #[test]
    fn a_forget_after_the_call_leaves_it_alive() {
        // import @10, call @20, `namespace forget` @30 — the call runs first.
        let mut evs = [install(10), remove(30)].into_iter();
        assert!(alias_live_at(&mut evs, &ran_before(20)));
    }

    #[test]
    fn a_re_import_after_a_forget_reinstates_the_alias() {
        let mut evs = [install(10), remove(15), install(17)].into_iter();
        assert!(alias_live_at(&mut evs, &ran_before(20)));
    }

    #[test]
    fn the_latest_removal_wins_over_an_earlier_install() {
        // Two forgets; the alias is dead from the first one that follows the
        // last install.
        let mut evs = [install(10), remove(12), remove(15)].into_iter();
        assert!(!alias_live_at(&mut evs, &ran_before(20)));
    }

    #[test]
    fn an_install_later_than_the_call_still_counts() {
        // Whether a call written *before* its own import resolves is #1104
        // item 1's still-open leniency: this function must not decide it, so
        // an install at 30 with the call at 20 still installs.
        let mut evs = [install(30)].into_iter();
        assert!(alias_live_at(&mut evs, &ran_before(20)));
        // …but a removal between them is still a removal.
        let mut evs = [install(30), remove(10)].into_iter();
        assert!(alias_live_at(&mut evs, &ran_before(20)));
    }

    #[test]
    fn an_unordered_removal_revokes_nothing() {
        // A `namespace forget` in another file: no static load order, so
        // navigation keeps answering rather than dropping a real alias.
        let mut evs = [install(10), unordered_event(AliasEventKind::Remove)].into_iter();
        assert!(alias_live_at(&mut evs, &ran_before(20)));
    }

    #[test]
    fn an_unordered_install_counts() {
        let mut evs = [unordered_event(AliasEventKind::Install)].into_iter();
        assert!(alias_live_at(&mut evs, &ran_before(20)));
        // …and survives an ordered removal, for the same reason an ordered
        // `-clear` cannot revoke an unordered export.
        let mut evs = [unordered_event(AliasEventKind::Install), remove(5)].into_iter();
        assert!(alias_live_at(&mut evs, &ran_before(20)));
    }

    #[test]
    fn no_events_at_all_means_no_alias() {
        let mut evs = [].into_iter();
        assert!(!alias_live_at(&mut evs, &ran_before(20)));
    }
}
