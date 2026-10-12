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

//! Versioned codegen-axis stamps.
//!
//! A command's codegen hook, inline codegen hook, semantic operation and native
//! lowering say which implementation emitted code rests on, and which one is
//! right can depend on the Tcl release the code is compiled for: a bytecode
//! emitter written against 9.0's instruction set is not 8.6's. A stamp is
//! versioned the way arity already is ([`ArityWindow`](crate::arity::ArityWindow)):
//! ordered windows beside the unversioned field, which stays the stamp for every
//! release no window covers, selected at the release the query asks about.
//!
//! The difference from arity is what a doubt costs. A signature that cannot be
//! chosen falls back to the plain `arity`, which only gates less; a stamp that
//! cannot be chosen must not be guessed, because a specialisation applied at a
//! release that does not have it emits wrong code. So selection has three
//! answers rather than two ([`StampSelection`]): the level states nothing at the
//! point and the level above answers; the level's stamp; or the level states
//! windows the point does not settle — a query with no release, or one over the
//! whole ladder that straddles a window edge — and the call is dispatched plain,
//! whatever a level above says.

use tcl_dialect::model::{Family, SurfaceQuery};

use crate::lifecycle::Lifecycle;

/// One version window of a codegen-axis stamp: the releases it applies to, and
/// the stamp in force there.
///
/// Windows live beside the unversioned field they version. An empty list means
/// the stamp never changed, which is every shipped spec; the unversioned field
/// then answers as it always has. Windows must not overlap — the loader reports
/// the second of two that do and `registry_sweep` rejects it for shipped specs —
/// so at most one window covers any release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StampWindow<T> {
    /// The releases of the Tcl core this stamp applies to.
    pub lifecycle: Lifecycle,
    /// The stamp in force in that window.
    pub value: T,
}

impl<T> StampWindow<T> {
    /// Whether two windows can both apply to some release.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.lifecycle.overlaps(other.lifecycle)
    }
}

impl<T: Copy> StampWindow<T> {
    /// The first window covering `target`, or `None` when none does — including
    /// every `target` of `None`, which selects nothing for want of a release to
    /// prefer one window over another.
    #[must_use]
    pub fn select(windows: &[Self], target: Option<&str>) -> Option<Self> {
        target?;
        windows
            .iter()
            .find(|window| window.lifecycle.available_at(target))
            .copied()
    }
}

/// Every stamp a level can carry at some point: the unversioned one, which may
/// be none, then each window's.
///
/// For the surface-blind questions — "which commands could be this operation" —
/// that must hold for whichever release a command is compiled for.
pub fn candidates<T: Copy>(
    unversioned: Option<T>,
    windows: &[StampWindow<T>],
) -> impl Iterator<Item = Option<T>> + '_ {
    std::iter::once(unversioned).chain(windows.iter().map(|window| Some(window.value)))
}

/// The stamps a level carries, windowed or not, without the absent one.
pub fn stamps<T: Copy>(
    unversioned: Option<T>,
    windows: &[StampWindow<T>],
) -> impl Iterator<Item = T> + '_ {
    candidates(unversioned, windows).flatten()
}

/// What one level of a descriptor — a command or one of its subcommands — says
/// about a stamp at the point a query asks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StampSelection<T> {
    /// The level states no stamp at this point, so the level above answers: a
    /// subcommand with none inherits its command's.
    Inherit,
    /// The stamp this level carries at this point.
    Stamp(T),
    /// The level states windows the point does not settle: a query with no
    /// release to select by, or over a whole ladder the windows divide. The call
    /// is dispatched plain, and a level above does not answer in its place.
    Decline,
}

impl<T: Copy + PartialEq> StampSelection<T> {
    /// What a level with this `unversioned` stamp and these `windows` says at
    /// the point `query` asks about.
    ///
    /// With no windows it is the unversioned field and nothing else, whatever
    /// the query. With windows the query's nearest core point must be a Tcl
    /// release or the whole Tcl ladder: a windowed stamp is a fact about a
    /// release of Tcl, so a query whose own family is another, with no core, or
    /// with no query at all cannot select one.
    /// At a release, the first window covering it wins and the unversioned stamp
    /// stands where none does. Over the whole ladder, the stamp must be the same
    /// at every release, or the level declines.
    #[must_use]
    pub fn of(
        unversioned: Option<T>,
        windows: &[StampWindow<T>],
        query: Option<&SurfaceQuery<'_>>,
    ) -> Self {
        if windows.is_empty() {
            return Self::stated(unversioned);
        }
        let Some((Family::Tcl, release)) = query.and_then(|query| query.core.nearest()) else {
            return Self::Decline;
        };
        if let Some(release) = release {
            return Self::at(unversioned, windows, release);
        }
        let mut ladder = Family::Tcl
            .releases()
            .iter()
            .map(|release| Self::at(unversioned, windows, release.as_str()));
        let first = ladder.next().unwrap_or(Self::Inherit);
        if ladder.all(|stamp| stamp == first) {
            first
        } else {
            Self::Decline
        }
    }

    /// A level with no windows: `stamp` if it carries one, and nothing to say
    /// otherwise.
    #[must_use]
    pub fn stated(stamp: Option<T>) -> Self {
        stamp.map_or(Self::Inherit, Self::Stamp)
    }

    fn at(unversioned: Option<T>, windows: &[StampWindow<T>], release: &str) -> Self {
        StampWindow::select(windows, Some(release))
            .map(|window| window.value)
            .map_or_else(|| Self::stated(unversioned), Self::Stamp)
    }

    /// This level's answer, or `parent`'s where this level states nothing. A
    /// declined level stays declined.
    #[must_use]
    pub fn or(self, parent: Self) -> Self {
        match self {
            Self::Inherit => parent,
            stated => stated,
        }
    }

    /// The stamp to act on: the selected one, and nothing for a level that
    /// states none or declines.
    #[must_use]
    pub const fn stamp(self) -> Option<T> {
        match self {
            Self::Stamp(stamp) => Some(stamp),
            Self::Inherit | Self::Decline => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(
        introduced: Option<&'static str>,
        retired: Option<&'static str>,
        value: u8,
    ) -> StampWindow<u8> {
        StampWindow {
            lifecycle: Lifecycle {
                introduced,
                deprecated: None,
                retired,
                deprecation_fix: None,
            },
            value,
        }
    }

    fn at_release(release: &'static str) -> SurfaceQuery<'static> {
        SurfaceQuery::core(Family::Tcl, release)
    }

    fn pick(
        unversioned: Option<u8>,
        windows: &[StampWindow<u8>],
        query: Option<&SurfaceQuery<'_>>,
    ) -> StampSelection<u8> {
        StampSelection::of(unversioned, windows, query)
    }

    #[test]
    fn with_no_windows_the_unversioned_stamp_answers_at_every_point() {
        for query in [
            None,
            Some(at_release("8.6")),
            Some(SurfaceQuery::any_release(Family::Tcl)),
            Some(SurfaceQuery::any_release(Family::F5Irules)),
        ] {
            assert_eq!(pick(Some(3), &[], query.as_ref()), StampSelection::Stamp(3));
            assert_eq!(pick(None, &[], query.as_ref()), StampSelection::Inherit);
        }
    }

    #[test]
    fn a_window_is_selected_at_the_release_it_covers() {
        let windows = [window(Some("9.0"), None, 7)];
        assert_eq!(
            pick(None, &windows, Some(&at_release("8.6"))),
            StampSelection::Inherit,
            "below the window the level states nothing, and the level above answers"
        );
        assert_eq!(
            pick(None, &windows, Some(&at_release("9.0"))),
            StampSelection::Stamp(7),
            "introduction is inclusive"
        );
        assert_eq!(
            pick(None, &windows, Some(&at_release("9.1"))),
            StampSelection::Stamp(7)
        );

        let closed = [window(None, Some("9.0"), 5)];
        assert_eq!(
            pick(None, &closed, Some(&at_release("8.6"))),
            StampSelection::Stamp(5)
        );
        assert_eq!(
            pick(None, &closed, Some(&at_release("9.0"))),
            StampSelection::Inherit,
            "retirement is exclusive"
        );
    }

    #[test]
    fn the_unversioned_stamp_stands_where_no_window_covers() {
        let windows = [window(Some("9.0"), None, 7)];
        assert_eq!(
            pick(Some(2), &windows, Some(&at_release("8.6"))),
            StampSelection::Stamp(2)
        );
        assert_eq!(
            pick(Some(2), &windows, Some(&at_release("9.0"))),
            StampSelection::Stamp(7),
            "a window wins over the unversioned stamp"
        );
    }

    #[test]
    fn a_whole_ladder_that_a_window_divides_declines() {
        let any = SurfaceQuery::any_release(Family::Tcl);
        let from_9 = [window(Some("9.0"), None, 7)];
        assert_eq!(
            pick(None, &from_9, Some(&any)),
            StampSelection::Decline,
            "8.x has no stamp and 9.x has one"
        );
        assert_eq!(
            pick(Some(2), &from_9, Some(&any)),
            StampSelection::Decline,
            "8.x has one stamp and 9.x another"
        );
        let two = [window(None, Some("9.0"), 5), window(Some("9.0"), None, 7)];
        assert_eq!(
            pick(None, &two, Some(&any)),
            StampSelection::Decline,
            "two windows cover the ladder between them, with two stamps"
        );
    }

    #[test]
    fn a_whole_ladder_the_windows_agree_on_selects() {
        let any = SurfaceQuery::any_release(Family::Tcl);
        assert_eq!(
            pick(None, &[window(Some("8.4"), None, 7)], Some(&any)),
            StampSelection::Stamp(7),
            "one window covers the whole ladder"
        );
        assert_eq!(
            pick(
                None,
                &[window(None, Some("9.0"), 7), window(Some("9.0"), None, 7)],
                Some(&any)
            ),
            StampSelection::Stamp(7),
            "two windows with one stamp are one stamp"
        );
        assert_eq!(
            pick(Some(2), &[window(Some("10.0"), None, 7)], Some(&any)),
            StampSelection::Stamp(2),
            "a window past the ladder moves nothing"
        );
        assert_eq!(
            pick(None, &[window(Some("10.0"), None, 7)], Some(&any)),
            StampSelection::Inherit
        );
    }

    #[test]
    fn a_point_that_is_not_a_tcl_release_declines_a_windowed_level() {
        let windows = [window(Some("9.0"), None, 7)];
        for query in [
            None,
            Some(SurfaceQuery::any_release(Family::F5Irules)),
            Some(SurfaceQuery::core(Family::Jim, "0.81")),
            Some(SurfaceQuery {
                realm: tcl_dialect::model::InvocationRealm::RuleLoader,
                core: tcl_dialect::model::CorePoints::NONE,
                packages: &[],
            }),
        ] {
            assert_eq!(
                pick(Some(2), &windows, query.as_ref()),
                StampSelection::Decline,
                "{query:?}"
            );
        }
    }

    #[test]
    fn a_declined_level_does_not_inherit_and_a_silent_one_does() {
        use StampSelection::{Decline, Inherit, Stamp};
        assert_eq!(Decline.or(Stamp(1)), Decline);
        assert_eq!(Inherit.or(Stamp(1)), Stamp(1));
        assert_eq!(Stamp(2).or(Stamp(1)), Stamp(2));
        assert_eq!(Inherit.or(Inherit::<u8>), Inherit);
        assert_eq!(Decline.stamp(), None::<u8>);
        assert_eq!(Inherit.stamp(), None::<u8>);
        assert_eq!(Stamp(4).stamp(), Some(4));
    }

    #[test]
    fn the_first_covering_window_is_selected_and_only_a_release_selects() {
        let windows = [window(None, Some("9.0"), 5), window(Some("9.0"), None, 7)];
        assert_eq!(
            StampWindow::select(&windows, Some("8.6")).map(|w| w.value),
            Some(5)
        );
        assert_eq!(
            StampWindow::select(&windows, Some("9.0")).map(|w| w.value),
            Some(7)
        );
        assert_eq!(StampWindow::select(&windows, None), None);
        assert_eq!(StampWindow::<u8>::select(&[], Some("9.0")), None);
    }

    #[test]
    fn candidates_are_the_unversioned_stamp_and_every_window() {
        let windows = [window(None, Some("9.0"), 5), window(Some("9.0"), None, 7)];
        assert_eq!(
            candidates(None, &windows).collect::<Vec<_>>(),
            [None, Some(5), Some(7)]
        );
        assert_eq!(candidates(Some(2), &[]).collect::<Vec<_>>(), [Some(2)]);
    }

    #[test]
    fn windows_overlap_as_lifecycles_do() {
        let old = window(None, Some("9.0"), 5);
        let new = window(Some("9.0"), None, 7);
        assert!(!old.overlaps(&new));
        assert!(!new.overlaps(&old));
        assert!(old.overlaps(&window(Some("8.6"), None, 9)));
        assert!(window(None, None, 1).overlaps(&window(None, None, 2)));
    }
}
