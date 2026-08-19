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

//! Argument count constraints.

/// Argument count range for a command or subcommand.
///
/// `min` and `max` are counts of arguments *after* the command name
/// (and after the subcommand word, for subcommands).
///
/// A flat `(min, max)` range can't express Tcl's "N, or N+2, or N+4, …"
/// key/value-pair shapes — `dict create ?key value ...?` (an even count),
/// `foreach varList list ?varList list ...? body` (an odd count), `dict
/// update dictVar key varName ?key varName ...? body` (an even count from
/// 4) — or `switch`'s two-shape union (its single-braced-body shorthand
/// alongside its flat pattern/body-pair form). [`Self::step`] and
/// [`Self::also_exact`] extend the range with exactly those two shapes;
/// both default to "no additional constraint" so every pre-existing
/// [`Arity`] call site ([`Self::new`] / [`Self::exact`] /
/// [`Self::at_least`] / [`Self::any`]) is unaffected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Arity {
    /// Minimum number of arguments.
    pub min: u16,
    /// Maximum number of arguments (`u16::MAX` = unlimited).
    pub max: u16,
    /// `0` (the default) means no parity constraint — every count in
    /// `min..=max` is valid. A nonzero value `S` further restricts valid
    /// counts to the arithmetic progression `min, min+S, min+S*2, …` —
    /// see [`Self::stepped`].
    pub step: u16,
    /// An additional single exact count that is valid regardless of
    /// [`Self::step`] — see [`Self::with_also_exact`]. `None` (the
    /// default) adds no such exception.
    pub also_exact: Option<u16>,
}

impl Arity {
    /// Unlimited upper bound sentinel.
    pub const UNLIMITED: u16 = u16::MAX;

    /// Create an arity with explicit min and max — every count in
    /// `min..=max` is valid.
    #[must_use]
    pub const fn new(min: u16, max: u16) -> Self {
        Self {
            min,
            max,
            step: 0,
            also_exact: None,
        }
    }

    /// Exactly `n` arguments required.
    #[must_use]
    pub const fn exact(n: u16) -> Self {
        Self::new(n, n)
    }

    /// At least `min` arguments, no upper bound.
    #[must_use]
    pub const fn at_least(min: u16) -> Self {
        Self::new(min, Self::UNLIMITED)
    }

    /// Zero or more arguments (no constraint).
    #[must_use]
    pub const fn any() -> Self {
        Self::new(0, Self::UNLIMITED)
    }

    /// An arity whose valid counts are the arithmetic progression `min,
    /// min+step, min+step*2, …` (capped at `max`) — Tcl's even/odd
    /// key/value-pair command shapes. `step == 0` is equivalent to
    /// [`Self::new`] (every count in range, no parity constraint).
    #[must_use]
    pub const fn stepped(min: u16, max: u16, step: u16) -> Self {
        Self {
            min,
            max,
            step,
            also_exact: None,
        }
    }

    /// Add a single exact-count exception valid regardless of
    /// [`Self::step`] or the `min..=max` range — `switch`'s
    /// single-braced-body shorthand (`switch $s {p1 b1 p2 b2 ...}`, one
    /// word after the subject) alongside its flat, odd-count-from-3
    /// pattern/body-pair form.
    #[must_use]
    pub const fn with_also_exact(self, n: u16) -> Self {
        Self {
            also_exact: Some(n),
            ..self
        }
    }

    /// Whether `n` arguments satisfy this constraint: within `min..=max`
    /// and on the [`Self::step`] progression, or equal to
    /// [`Self::also_exact`].
    #[must_use]
    pub const fn accepts(self, n: u16) -> bool {
        if let Some(exact) = self.also_exact
            && n == exact
        {
            return true;
        }
        n >= self.min && n <= self.max && self.matches_step(n)
    }

    /// Whether `n` lands on the [`Self::step`] progression from `min` —
    /// `true` unconditionally when `step == 0` (no parity constraint).
    /// Does **not** check the `min..=max` range or [`Self::also_exact`]
    /// on its own; combine with those separately (see
    /// [`Self::accepts`], and `tcl_compiler`'s `arity_verdict`, which
    /// reports a count that's in-range but off the progression as its
    /// own diagnostic shape distinct from "too few"/"too many").
    #[must_use]
    pub const fn matches_step(self, n: u16) -> bool {
        self.step == 0 || (n >= self.min && (n - self.min).is_multiple_of(self.step))
    }

    /// Whether the upper bound is unlimited.
    #[must_use]
    pub const fn is_unlimited(self) -> bool {
        self.max == Self::UNLIMITED
    }
}

impl Default for Arity {
    fn default() -> Self {
        Self::any()
    }
}

/// One version window of a signature that changed across its owning package's
/// releases (issue #1627).
///
/// A command whose signature grew an argument in 3.0 has two shapes, and which
/// one applies is a fact about the *resolved package floor*, not about the call
/// site. Before this existed the registry could hold only one [`Arity`], so a
/// library with a versioned signature either warned wrongly on old-pinned code
/// or stayed silent on new-pinned code.
///
/// Windows live beside the single `arity` field rather than replacing it: an
/// empty window list means "this signature never changed", which is the case
/// for almost every shipped spec, and the single `arity` remains the unbounded
/// default. See [`select`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArityWindow {
    /// The releases of the owning package this shape applies to.
    pub lifecycle: crate::lifecycle::Lifecycle,
    /// The signature shape in that window — the whole shape, including
    /// [`Arity::step`] and [`Arity::also_exact`], not just `min`/`max`.
    pub arity: Arity,
}

impl ArityWindow {
    /// Select the window that applies at `target`, or `None` when no window
    /// covers it — including the common case of an empty `windows` list.
    ///
    /// Deliberately the *first* covering window in declaration order rather
    /// than the narrowest or the newest: overlapping windows are a declaration
    /// error the loader notices and the shipped-spec sweep rejects outright, so
    /// by the time selection runs at most one window can legitimately cover any
    /// release. Taking the first keeps the behaviour of a malformed pack
    /// predictable (it degrades to whichever shape the author wrote first)
    /// instead of depending on a comparison between shapes that were never
    /// meant to overlap.
    ///
    /// An unresolved `target` selects nothing: with no floor there is no basis
    /// to prefer one shape over another, and the caller falls back to the
    /// unversioned `arity` — the same "no version known ⇒ do not gate" rule the
    /// rest of the lifecycle machinery follows.
    #[must_use]
    pub fn select(windows: &[Self], target: Option<&str>) -> Option<Self> {
        target?;
        windows
            .iter()
            .find(|window| window.lifecycle.available_at(target))
            .copied()
    }

    /// Whether two windows can both apply to some release.
    ///
    /// Used by the loader notice and the shipped-spec sweep. Two windows
    /// overlap when neither is wholly retired before the other is introduced;
    /// a window with no declared bounds overlaps everything, which is why a
    /// second unbounded window is always an error.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        fn ends_before(a: crate::lifecycle::Lifecycle, b: crate::lifecycle::Lifecycle) -> bool {
            match (a.retired, b.introduced) {
                (Some(retired), Some(introduced)) => {
                    crate::version::compare(retired, introduced).is_le()
                }
                _ => false,
            }
        }
        !ends_before(self.lifecycle, other.lifecycle)
            && !ends_before(other.lifecycle, self.lifecycle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_accepts_only_n() {
        let a = Arity::exact(4);
        assert!(!a.accepts(3));
        assert!(a.accepts(4));
        assert!(!a.accepts(5));
    }

    #[test]
    fn at_least_accepts_min_and_above() {
        let a = Arity::at_least(2);
        assert!(!a.accepts(1));
        assert!(a.accepts(2));
        assert!(a.accepts(100));
        assert!(a.is_unlimited());
    }

    #[test]
    fn range_accepts_within() {
        let a = Arity::new(1, 3);
        assert!(!a.accepts(0));
        assert!(a.accepts(1));
        assert!(a.accepts(3));
        assert!(!a.accepts(4));
        assert!(!a.is_unlimited());
    }

    #[test]
    fn any_accepts_everything() {
        let a = Arity::any();
        assert!(a.accepts(0));
        assert!(a.accepts(u16::MAX));
    }

    #[test]
    fn stepped_accepts_only_the_progression() {
        // `dict create ?key value ...?` — an even count from 0.
        let a = Arity::stepped(0, Arity::UNLIMITED, 2);
        assert!(a.accepts(0));
        assert!(!a.accepts(1));
        assert!(a.accepts(2));
        assert!(!a.accepts(3));
        assert!(a.accepts(100));
        assert!(!a.accepts(101));
    }

    #[test]
    fn stepped_respects_min_and_max_bounds() {
        // `dict update dictVar key varName ?...? body` — an even count
        // from 4, unbounded above.
        let a = Arity::stepped(4, Arity::UNLIMITED, 2);
        assert!(!a.accepts(2), "below min, even though on the progression");
        assert!(!a.accepts(3));
        assert!(a.accepts(4));
        assert!(!a.accepts(5));
        assert!(a.accepts(6));
    }

    #[test]
    fn stepped_zero_step_matches_plain_new() {
        assert_eq!(Arity::stepped(1, 3, 0), Arity::new(1, 3));
        let a = Arity::stepped(1, 3, 0);
        assert!(a.accepts(1));
        assert!(a.accepts(2));
        assert!(a.accepts(3));
    }

    #[test]
    fn with_also_exact_adds_a_single_exception() {
        // `switch`'s two-shape union: the single-braced-body shorthand
        // (exactly 2 args after `switch` — string + one braced blob) or
        // the flat pattern/body-pair form (an odd count `>= 3`).
        let a = Arity::stepped(3, Arity::UNLIMITED, 2).with_also_exact(2);
        assert!(!a.accepts(0));
        assert!(!a.accepts(1));
        assert!(a.accepts(2), "the also_exact shorthand count");
        assert!(a.accepts(3));
        assert!(
            !a.accepts(4),
            "4 is neither the shorthand nor on the odd progression"
        );
        assert!(a.accepts(5));
    }

    /// Build a window introduced at `from`, retired at `to` (exclusive).
    fn window(from: Option<&'static str>, to: Option<&'static str>, arity: Arity) -> ArityWindow {
        ArityWindow {
            lifecycle: crate::lifecycle::Lifecycle {
                introduced: from,
                deprecated: None,
                retired: to,
                deprecation_fix: None,
            },
            arity,
        }
    }

    #[test]
    fn no_windows_selects_nothing() {
        assert_eq!(ArityWindow::select(&[], Some("3.0")), None);
    }

    #[test]
    fn an_unresolved_floor_selects_nothing() {
        // No floor means no basis to prefer a shape; the caller falls back to
        // the unversioned `arity` rather than guessing a window.
        let windows = [window(None, None, Arity::exact(2))];
        assert_eq!(ArityWindow::select(&windows, None), None);
    }

    #[test]
    fn the_window_covering_the_floor_is_selected() {
        let windows = [
            window(None, Some("3.0"), Arity::exact(2)),
            window(Some("3.0"), None, Arity::exact(3)),
        ];
        // Below the change: the old shape.
        assert_eq!(
            ArityWindow::select(&windows, Some("2.4")).map(|w| w.arity),
            Some(Arity::exact(2))
        );
        // At the change: retirement is exclusive, so 3.0 is the new shape.
        assert_eq!(
            ArityWindow::select(&windows, Some("3.0")).map(|w| w.arity),
            Some(Arity::exact(3))
        );
        // Above: still the new shape.
        assert_eq!(
            ArityWindow::select(&windows, Some("4.1")).map(|w| w.arity),
            Some(Arity::exact(3))
        );
    }

    #[test]
    fn a_deprecated_window_still_applies() {
        // Deprecated is available — a signature can be discouraged without
        // having changed yet, and gating on it would report the wrong shape.
        let mut w = window(Some("1.0"), None, Arity::exact(2));
        w.lifecycle.deprecated = Some("2.0");
        assert_eq!(
            ArityWindow::select(&[w], Some("2.5")).map(|w| w.arity),
            Some(Arity::exact(2))
        );
    }

    #[test]
    fn a_floor_below_every_window_selects_nothing() {
        let windows = [window(Some("3.0"), None, Arity::exact(3))];
        assert_eq!(ArityWindow::select(&windows, Some("1.0")), None);
    }

    #[test]
    fn adjacent_windows_do_not_overlap_but_straddling_ones_do() {
        let old = window(None, Some("3.0"), Arity::exact(2));
        let new = window(Some("3.0"), None, Arity::exact(3));
        assert!(
            !old.overlaps(new),
            "retirement is exclusive, so [.., 3.0) and [3.0, ..) are adjacent"
        );
        assert!(!new.overlaps(old), "overlap is symmetric");

        let straddling = window(Some("2.0"), None, Arity::exact(4));
        assert!(
            old.overlaps(straddling),
            "2.0 falls inside [.., 3.0), so both windows claim it"
        );

        // Two unbounded windows always overlap — the shape a pack hits by
        // declaring a second `arity` row and forgetting its lifecycle.
        let a = window(None, None, Arity::exact(1));
        let b = window(None, None, Arity::exact(2));
        assert!(a.overlaps(b));
    }

    #[test]
    fn matches_step_ignores_range_and_also_exact() {
        // `matches_step` alone only judges the progression — callers combine
        // it with their own range / also_exact handling (see `accepts` and
        // `tcl_compiler`'s `arity_verdict`).
        let a = Arity::stepped(4, 4, 2).with_also_exact(99);
        assert!(a.matches_step(6), "6 is on the progression, though > max");
        assert!(
            !a.matches_step(99),
            "matches_step doesn't consult also_exact"
        );
    }
}
