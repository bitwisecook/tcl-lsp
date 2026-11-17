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

//! How much a facts domain of a command is known (issue #1712).
//!
//! A declared facts field used to mean two things at once: an empty side-effect
//! list was both "verified to have none" and "nobody looked". [`Knowledge`]
//! separates them, so a consumer cannot read a domain without facing how much
//! of it is known:
//!
//! - [`Knowledge::Unknown`]: nothing is claimed. A consumer must assume the
//!   worst the domain allows.
//! - [`Knowledge::Partial`]: the value is a lower bound. Every fact in it holds,
//!   and more may hold that it does not state.
//! - [`Knowledge::Complete`]: the value is the whole domain, evidenced from the
//!   implementation for the releases its [`SpecSurface`] rows cover. One
//!   release never fills another: outside those rows it is only a lower bound.

use tcl_dialect::model::{SpecProvider, SpecSurface, SurfaceQuery};

/// The completeness of one facts domain, with the facts known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Knowledge<T> {
    /// Nothing is known about the domain.
    #[default]
    Unknown,
    /// The facts hold, and the domain may hold more.
    Partial(T),
    /// The facts are the whole domain at every release `surfaces` covers.
    Complete(T, &'static [SpecSurface]),
}

impl<T> Knowledge<T> {
    /// The facts known, whatever their completeness.
    #[must_use]
    pub const fn known(&self) -> Option<&T> {
        match self {
            Self::Unknown => None,
            Self::Partial(value) | Self::Complete(value, _) => Some(value),
        }
    }

    /// Whether nothing is known.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// The surfaces a [`Knowledge::Complete`] value is evidenced for.
    #[must_use]
    pub const fn complete_surfaces(&self) -> Option<&'static [SpecSurface]> {
        match self {
            Self::Complete(_, surfaces) => Some(surfaces),
            _ => None,
        }
    }

    /// The facts, when they are the whole domain at every point `query` can
    /// resolve to.
    ///
    /// A point with a stated release is complete when some surface covers it.
    /// A point with no release, which is any release of its family, is complete
    /// only when every release on the family's ladder is covered. A package
    /// with no floor is complete only when a surface states the package with no
    /// window. No query, or a query with no point at all, is never complete.
    #[must_use]
    pub fn complete_at(&self, query: Option<&SurfaceQuery<'_>>) -> Option<&T> {
        let Self::Complete(value, surfaces) = self else {
            return None;
        };
        complete_over(surfaces, query?).then_some(value)
    }

    /// Map the known facts.
    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Knowledge<U> {
        match self {
            Self::Unknown => Knowledge::Unknown,
            Self::Partial(value) => Knowledge::Partial(f(value)),
            Self::Complete(value, surfaces) => Knowledge::Complete(f(value), surfaces),
        }
    }

    /// The completeness word the `SpecTcl` DSL and Spec Studio spell.
    #[must_use]
    pub const fn level(&self) -> KnowledgeLevel {
        match self {
            Self::Unknown => KnowledgeLevel::Unknown,
            Self::Partial(_) => KnowledgeLevel::Partial,
            Self::Complete(..) => KnowledgeLevel::Complete,
        }
    }
}

impl<T: Copy> Knowledge<T> {
    /// The facts stated at this layer, whatever their completeness: what the
    /// `Option` the domain replaced held.
    #[must_use]
    pub const fn stated(&self) -> Option<T> {
        match self {
            Self::Unknown => None,
            Self::Partial(value) | Self::Complete(value, _) => Some(*value),
        }
    }

    /// The knowledge an optional authored value states before any evidence: a
    /// value is a lower bound, no value says nothing.
    #[must_use]
    pub const fn from_stated(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Partial(value),
            None => Self::Unknown,
        }
    }
}

impl<T: 'static> Knowledge<&'static [T]> {
    /// The facts known to hold: the stated slice, or none when unknown.
    #[must_use]
    pub const fn lower_bound(&self) -> &'static [T] {
        match self {
            Self::Unknown => &[],
            Self::Partial(value) | Self::Complete(value, _) => value,
        }
    }

    /// The knowledge an authored list states before any evidence: a
    /// non-empty list is a lower bound, an empty one says nothing.
    #[must_use]
    pub const fn declared(value: &'static [T]) -> Self {
        if value.is_empty() {
            Self::Unknown
        } else {
            Self::Partial(value)
        }
    }

    /// Whether the domain is known to be empty at every point `query` can
    /// resolve to.
    #[must_use]
    pub fn complete_and_empty_at(&self, query: Option<&SurfaceQuery<'_>>) -> bool {
        self.complete_at(query)
            .is_some_and(|value| value.is_empty())
    }
}

/// The completeness of a domain without its facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KnowledgeLevel {
    /// [`Knowledge::Unknown`].
    Unknown,
    /// [`Knowledge::Partial`].
    Partial,
    /// [`Knowledge::Complete`].
    Complete,
}

impl KnowledgeLevel {
    /// The lower-case spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Partial => "partial",
            Self::Complete => "complete",
        }
    }
}

/// The words an `effects -complete` flag spells one surface with: `FROM
/// UNTIL` on a family ladder, both inclusive; `package NAME`; or `package
/// NAME FROM UNTIL`, half-open on the package's own axis. `None` for a
/// surface no flag can spell.
#[must_use]
pub fn complete_window_words(surface: &SpecSurface) -> Option<Vec<&'static str>> {
    match (surface.provider, surface.windows) {
        (SpecProvider::Core(family), [_]) => {
            let admitted: Vec<&'static str> = family
                .releases()
                .iter()
                .map(|step| step.as_str())
                .filter(|release| surface.admits(&SurfaceQuery::core(family, release)))
                .collect();
            Some(vec![admitted.first()?, admitted.last()?])
        }
        (SpecProvider::Package(name), []) => Some(vec!["package", name]),
        (SpecProvider::Package(name), [(from, Some(until))]) => {
            Some(vec!["package", name, from, until])
        }
        _ => None,
    }
}

fn complete_over(surfaces: &[SpecSurface], query: &SurfaceQuery<'_>) -> bool {
    if query.core.is_empty() && query.packages.is_empty() {
        return false;
    }
    let states_core = surfaces
        .iter()
        .any(|surface| matches!(surface.provider, SpecProvider::Core(_)));
    let core_complete = !states_core
        || query.core.iter().all(|(family, release)| {
            let covered = |release: &str| {
                surfaces.iter().any(|surface| {
                    matches!(surface.provider, SpecProvider::Core(stated) if stated == family)
                        && surface.admits(&SurfaceQuery::core(family, release))
                })
            };
            match release {
                Some(release) => covered(release),
                None => family.releases().iter().all(|step| covered(step.as_str())),
            }
        });
    // A package surface speaks for the package's own implementation: where
    // the query carries that package, its floor must be covered, and a
    // floor-less package only by a surface with no window.
    let packages_complete = surfaces.iter().all(|surface| match surface.provider {
        SpecProvider::Package(package) => query.package(package).is_none_or(|carried| {
            surface.windows.is_empty() || carried.version.is_some() && surface.admits(query)
        }),
        SpecProvider::Core(_) => true,
    });
    let states_package = surfaces
        .iter()
        .any(|surface| matches!(surface.provider, SpecProvider::Package(_)));
    (states_core || states_package) && core_complete && packages_complete
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::Family;

    const EVIDENCED_90: &[SpecSurface] =
        &[SpecSurface::core_in(Family::Tcl, &[("9.0", Some("9.1"))])];

    #[test]
    fn complete_knowledge_holds_only_where_its_surface_is_evidenced() {
        let empty: Knowledge<&'static [u8]> = Knowledge::Complete(&[], EVIDENCED_90);
        let at = |release| SurfaceQuery::core(Family::Tcl, release);
        assert!(empty.complete_and_empty_at(Some(&at("9.0"))));
        assert!(!empty.complete_and_empty_at(Some(&at("8.4"))));
        assert!(!empty.complete_and_empty_at(Some(&at("9.1"))));
        assert!(!empty.complete_and_empty_at(Some(&SurfaceQuery::any_release(Family::Tcl))));
        assert!(!empty.complete_and_empty_at(None));
        assert_eq!(empty.lower_bound(), &[] as &[u8]);
    }

    #[test]
    fn an_unresolved_release_is_complete_only_over_the_whole_ladder() {
        let everywhere: Knowledge<&'static [u8]> = Knowledge::Complete(&[], SpecSurface::ALL_TCL);
        assert!(everywhere.complete_and_empty_at(Some(&SurfaceQuery::any_release(Family::Tcl))));
        for release in ["8.4", "8.5", "8.6", "9.0", "9.1"] {
            assert!(
                everywhere.complete_and_empty_at(Some(&SurfaceQuery::core(Family::Tcl, release)))
            );
        }
        assert!(!everywhere.complete_and_empty_at(Some(&SurfaceQuery::core(Family::Jim, "0.84"))));
    }

    #[test]
    fn complete_windows_spell_back_as_their_inclusive_releases() {
        let core = SpecSurface::core_in(Family::Tcl, &[("8.6", Some("9.1"))]);
        assert_eq!(complete_window_words(&core), Some(vec!["8.6", "9.0"]));
        assert_eq!(
            complete_window_words(&SpecSurface::ALL_TCL[0]),
            Some(vec!["8.4", "9.1"])
        );
        let open = SpecSurface::core_in(Family::Tcl, &[("9.0", None)]);
        assert_eq!(complete_window_words(&open), Some(vec!["9.0", "9.1"]));
        assert_eq!(
            complete_window_words(&SpecSurface::package("struct::list")),
            Some(vec!["package", "struct::list"])
        );
    }

    #[test]
    fn authored_lists_declare_a_lower_bound_or_nothing() {
        assert_eq!(
            Knowledge::<&'static [u8]>::declared(&[]),
            Knowledge::Unknown
        );
        assert_eq!(
            Knowledge::declared(&[1u8][..]),
            Knowledge::Partial(&[1u8][..])
        );
        let partial = Knowledge::Partial(&[1u8][..]);
        assert_eq!(partial.lower_bound(), &[1]);
        assert!(!partial.complete_and_empty_at(Some(&SurfaceQuery::core(Family::Tcl, "9.0"))));
        assert_eq!(partial.level(), KnowledgeLevel::Partial);
    }
}
