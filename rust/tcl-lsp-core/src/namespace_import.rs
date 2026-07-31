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

//! The per-import-site export snapshot: the one decision both wildcard-import
//! tiers ask.
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
//! `qualified_variable_cell_at` uses. What *is* tier-specific is the primitive
//! "had this event already run when the import executed?", which the caller
//! supplies as `visible`:
//!
//! - Same document: `analyser::indirection::in_effect`, so a declaration
//!   inside a proc body is judged by the same load-order rule the rename /
//!   alias timeline uses.
//! - Cross-document, same file as the import: a plain offset comparison.
//! - **Different file from the import: not ordered at all.** Which file loads
//!   first is not a static fact, so such an event is passed with
//!   [`ExportEvent::at`] `None`, and this module abstains toward the safer
//!   side *for navigation*: an unordered pattern still counts (keep answering
//!   go-to-definition / find-references, the pre-#1027 behaviour), while an
//!   unordered `-clear` does **not** revoke anything (revoking on a guess
//!   would silently drop real references).

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
}
