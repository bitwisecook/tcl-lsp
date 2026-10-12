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

//! What a compiled spec literal writes to say where it comes from.
//!
//! A command, option or form spec carries a list of these — the
//! const-constructible half of the registry's `SurfaceDeclaration`, which
//! needs a [`VersionSet`](crate::model::VersionSet) and interned ids no
//! `const` can build. Lowering happens once per spec, in the registry's
//! `declarations_for_spec`.
//!
//! This replaces the retired `DialectSet` bitmask (Q13), which could name only
//! whole Tcl lines and a fixed vendor list — which is why Jim's own commands
//! were unexpressible: there was no Jim bit, and one bit could not have
//! carried `{jim 0.81-}` anyway. A row names its provider and its window on
//! *that provider's* axis, so both fall out.

use crate::model::Family;

/// Who provides a shape, as a spec literal spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecProvider {
    /// A core family's own surface.
    Core(Family),
    /// A named package's surface, spelled as the registry's package data
    /// spells it (`"Tk"`, `"iapps"`, `"struct::graph"`).
    Package(&'static str),
}

/// One half-open `[start, end)` window on a provider's version axis.
///
/// `end` is `None` for "and everything after", which is the common case: a
/// command introduced in 8.5 and never removed is `("8.5", None)`.
pub type SpecWindow = (&'static str, Option<&'static str>);

/// One authored availability row: a provider, and when it offers the shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpecSurface {
    /// Who provides it.
    pub provider: SpecProvider,
    /// The windows on the provider's own axis. Empty means every release the
    /// provider has — the overwhelmingly common case, and why it is the
    /// spelling [`SpecSurface::core`] and [`SpecSurface::package`] produce.
    pub windows: &'static [SpecWindow],
}

impl SpecSurface {
    /// `family`'s core surface, at every release on its ladder.
    #[must_use]
    pub const fn core(family: Family) -> Self {
        Self {
            provider: SpecProvider::Core(family),
            windows: &[],
        }
    }

    /// `family`'s core surface, restricted to `windows` on its ladder.
    #[must_use]
    pub const fn core_in(family: Family, windows: &'static [SpecWindow]) -> Self {
        Self {
            provider: SpecProvider::Core(family),
            windows,
        }
    }

    /// `package`'s surface, at every release the package has.
    #[must_use]
    pub const fn package(package: &'static str) -> Self {
        Self {
            provider: SpecProvider::Package(package),
            windows: &[],
        }
    }

    /// `package`'s surface, restricted to `windows` on the package's axis.
    #[must_use]
    pub const fn package_in(package: &'static str, windows: &'static [SpecWindow]) -> Self {
        Self {
            provider: SpecProvider::Package(package),
            windows,
        }
    }
}

/// Ladder and vendor shorthands.
///
/// Each names exactly the rows the retired `DialectSet` bit or union lowered
/// to, so a migrated spec gets the same answer it always did. They exist
/// because ~1,800 compiled specs spell one of a dozen windows, and naming
/// each once keeps the data readable.
///
/// The upper bounds are the ladder's, not infinity: `TCL85_PLUS` is
/// `8.5-9.2`, because the bitmask it replaces was a union of the five
/// *known* line bits and could not mean "and every line added later". A
/// spec that genuinely wants open-ended availability writes
/// [`SpecSurface::core_in`] with a `None` upper bound.
impl SpecSurface {
    /// Every Tcl release the ladder has — 8.4 through 9.1.
    pub const ALL_TCL: &'static [Self] = &[Self::core_in(Family::Tcl, &W_ALL_TCL)];
    /// Tcl 8.4 only.
    pub const TCL84: &'static [Self] = &[Self::core_in(Family::Tcl, &W_TCL84)];
    /// Tcl 8.5 only.
    pub const TCL85: &'static [Self] = &[Self::core_in(Family::Tcl, &W_TCL85)];
    /// Tcl 8.6 only.
    pub const TCL86: &'static [Self] = &[Self::core_in(Family::Tcl, &W_TCL86)];
    /// Tcl 9.0 only.
    pub const TCL90: &'static [Self] = &[Self::core_in(Family::Tcl, &W_TCL90)];
    /// Tcl 9.1 only.
    pub const TCL91: &'static [Self] = &[Self::core_in(Family::Tcl, &W_TCL91)];
    /// The whole Tcl 8.x line — 8.4 through 8.6, not 9.x.
    pub const TCL8X: &'static [Self] = &[Self::core_in(Family::Tcl, &W_TCL8X)];
    /// Tcl 8.5 through the top of the ladder.
    pub const TCL85_PLUS: &'static [Self] = &[Self::core_in(Family::Tcl, &W_TCL85_PLUS)];
    /// Tcl 8.6 through the top of the ladder.
    pub const TCL86_PLUS: &'static [Self] = &[Self::core_in(Family::Tcl, &W_TCL86_PLUS)];
    /// Tcl 9.0 through the top of the ladder.
    pub const TCL90_PLUS: &'static [Self] = &[Self::core_in(Family::Tcl, &W_TCL90_PLUS)];

    /// The F5 iRules core surface.
    pub const IRULES: &'static [Self] = &[Self::core(Family::F5Irules)];
    /// The Jim Tcl core surface — Jim's own additions, which the retired
    /// bitmask had no bit for (ledger D17-J).
    pub const JIM: &'static [Self] = &[Self::core(Family::Jim)];

    /// The F5 iApps package surface.
    pub const IAPPS: &'static [Self] = &[Self::package("iapps")];
    /// The F5 tmsh package surface.
    pub const TMSH: &'static [Self] = &[Self::package("tmsh")];
    /// The Tk package surface.
    pub const TK: &'static [Self] = &[Self::package("Tk")];
    /// The Expect package surface.
    pub const EXPECT: &'static [Self] = &[Self::package("expect")];
    /// The `SpecTcl` authoring-DSL surface.
    pub const SPECTCL: &'static [Self] = &[Self::package("spectcl")];
    /// The `SslicTcl` TLS-assurance authoring-DSL surface.
    pub const SSLICTCL: &'static [Self] = &[Self::package("sslictcl")];
    /// The BPF-Tcl package surface.
    pub const BPF: &'static [Self] = &[Self::package("bpf")];
    /// The BIG-IP configuration surface.
    pub const BIGIP: &'static [Self] = &[Self::package("bigip")];

    /// The whole Tcl ladder plus the iRules surface — a core command that
    /// iRules also enables. The single most common composite in the
    /// compiled data.
    pub const ALL_TCL_AND_IRULES: &'static [Self] = &[
        Self::core_in(Family::Tcl, &W_ALL_TCL),
        Self::core(Family::F5Irules),
    ];

    /// Tk plus the whole Tcl ladder — a command `wish` has because Tcl has
    /// it, which also exists as a Tk-provided shape.
    pub const TK_AND_TCL: &'static [Self] =
        &[Self::core_in(Family::Tcl, &W_ALL_TCL), Self::package("Tk")];
}

const W_ALL_TCL: [SpecWindow; 1] = [("8.4", Some("9.2"))];
const W_TCL84: [SpecWindow; 1] = [("8.4", Some("8.5"))];
const W_TCL85: [SpecWindow; 1] = [("8.5", Some("8.6"))];
const W_TCL86: [SpecWindow; 1] = [("8.6", Some("8.7"))];
const W_TCL90: [SpecWindow; 1] = [("9.0", Some("9.1"))];
const W_TCL91: [SpecWindow; 1] = [("9.1", Some("9.2"))];
const W_TCL8X: [SpecWindow; 1] = [("8.4", Some("8.7"))];
const W_TCL85_PLUS: [SpecWindow; 1] = [("8.5", Some("9.2"))];
const W_TCL86_PLUS: [SpecWindow; 1] = [("8.6", Some("9.2"))];
const W_TCL90_PLUS: [SpecWindow; 1] = [("9.0", Some("9.2"))];

/// Build a `&'static [SpecSurface]` from rows, for a spec whose surface is
/// not one of the [`SpecSurface`] shorthands.
///
/// The replacement for the retired mask's `.union(…)`: rows compose by
/// listing, so `surface![SpecSurface::core(Family::Tcl), SpecSurface::package("iapps")]`
/// is what `ALL_TCL.union(IAPPS)` used to say.
#[macro_export]
macro_rules! surface {
    ($($row:expr),* $(,)?) => {{
        const ROWS: &[$crate::model::SpecSurface] = &[$($row),*];
        ROWS
    }};
}

/// A package a query's context carries, and the release of it the context
/// guarantees.
///
/// `version` is a **floor** on the package's own axis — the lowest release
/// the context promises, not the single release it runs. A row windowed on
/// the package's axis is asked about that floor, the way a lifecycle is asked
/// about a target release: `introduced <= floor < retired`. `None` is a
/// package the context carries and names no release of, which admits every
/// window rather than none — a floor nobody stated cannot rule a row out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackageFloor<'a> {
    /// The package, spelled as a [`SpecProvider::Package`] row spells it.
    pub name: &'a str,
    /// The lowest release of the package the context guarantees, if one is
    /// stated.
    pub version: Option<&'a str>,
}

impl<'a> PackageFloor<'a> {
    /// `name`, carried with no floor stated.
    #[must_use]
    pub const fn named(name: &'a str) -> Self {
        Self {
            name,
            version: None,
        }
    }

    /// `name`, guaranteed at `version` or later.
    #[must_use]
    pub const fn at(name: &'a str, version: &'a str) -> Self {
        Self {
            name,
            version: Some(version),
        }
    }
}

/// One core point of a [`SurfaceQuery`]: a family and, when one is pinned,
/// the release on its ladder. A release of `None` is *any* release of the
/// family.
pub type CorePoint<'a> = (Family, Option<&'a str>);

/// The most core points one query carries: a document's own family, then
/// the ancestry anchor its command surface is inherited from.
const MAX_CORE_POINTS: usize = 2;

/// The core points a query asks at, **nearest first**.
///
/// A family that inherits a command surface from an ancestor and adds to it
/// asks at its own family first and at the ancestor's anchor second. A row
/// naming either admits the query, and where rows of both offer the same
/// command the nearer one is the one selected ([`surface_nearness`]). Empty
/// for a context with no core runtime of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorePoints<'a> {
    points: [Option<CorePoint<'a>>; MAX_CORE_POINTS],
}

impl<'a> CorePoints<'a> {
    /// No core point.
    pub const NONE: Self = Self {
        points: [None; MAX_CORE_POINTS],
    };

    /// A single point: `family` at `release`, or at any release when `None`.
    #[must_use]
    pub const fn one(family: Family, release: Option<&'a str>) -> Self {
        Self {
            points: [Some((family, release)), None],
        }
    }

    /// Two points, `nearest` ahead of `next`.
    #[must_use]
    pub const fn two(nearest: CorePoint<'a>, next: CorePoint<'a>) -> Self {
        Self {
            points: [Some(nearest), Some(next)],
        }
    }

    /// The points of `ordered`, nearest first. Points past the second are
    /// dropped: a family asks at itself and at one ancestry anchor.
    #[must_use]
    pub fn from_ordered(ordered: impl IntoIterator<Item = CorePoint<'a>>) -> Self {
        let mut points = [None; MAX_CORE_POINTS];
        for (slot, point) in points.iter_mut().zip(ordered) {
            *slot = Some(point);
        }
        Self { points }
    }

    /// The nearest point: the query's own family.
    #[must_use]
    pub const fn nearest(&self) -> Option<CorePoint<'a>> {
        self.points[0]
    }

    /// The points, nearest first.
    pub fn iter(&self) -> impl Iterator<Item = CorePoint<'a>> + '_ {
        self.points.iter().flatten().copied()
    }

    /// How many points there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Whether there is no point.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.points[0].is_none()
    }
}

/// The host phase whose command availability is being queried.
/// This does not establish an engine/compiler identity or an entered body.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InvocationRealm {
    /// Authored source is subject to the host's rule-loader command policy.
    #[default]
    RuleLoader,
    /// An audited evaluated-script entry selects the interpreter command table.
    InterpreterRuntime,
}

/// The point a surface question is asked at — the replacement for the retired
/// availability point (Q13).
///
/// A mask conflated two different facts in one word: which Tcl *line* a
/// context is, and which vendor surface it carries. A point states both
/// separately, so a question can be asked about a release the bitmask had
/// no bit for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceQuery<'a> {
    /// Explicit availability phase, independent of source-origin kind.
    pub realm: InvocationRealm,
    /// The core points the context resolves to, nearest first, each a
    /// family and the release on its ladder. Empty is a context with no
    /// core runtime of its own — the BIG-IP config surface, the permissive
    /// fallback. A release of `None` is *any* release of that family: what
    /// the mask said by setting every ladder bit, which a context with no
    /// resolved primary still needs to say.
    pub core: CorePoints<'a>,
    /// The packages active in the context, each with the floor the context
    /// guarantees of it.
    pub packages: &'a [PackageFloor<'a>],
}

impl<'a> SurfaceQuery<'a> {
    /// A query at `family`'s `release`, with no packages.
    #[must_use]
    pub const fn core(family: Family, release: &'a str) -> Self {
        Self {
            realm: InvocationRealm::RuleLoader,
            core: CorePoints::one(family, Some(release)),
            packages: &[],
        }
    }

    /// A query at any release of `family`, with no packages.
    #[must_use]
    pub const fn any_release(family: Family) -> Self {
        Self {
            realm: InvocationRealm::RuleLoader,
            core: CorePoints::one(family, None),
            packages: &[],
        }
    }

    /// Select a host availability phase. iRules' runtime command table is
    /// anchored in the shared fork point; its closed roster is applied by the
    /// registry, independently of the native compiler protocol.
    #[must_use]
    pub fn with_realm(mut self, realm: InvocationRealm) -> Self {
        self.realm = realm;
        if let Some((Family::F5Irules, release)) = self.core.nearest() {
            self.core = match realm {
                InvocationRealm::RuleLoader => CorePoints::one(Family::F5Irules, release),
                InvocationRealm::InterpreterRuntime => CorePoints::two(
                    (Family::F5Irules, release),
                    (Family::Tcl, Some(Family::F5_FORK_POINT)),
                ),
            };
        }
        self
    }

    /// A query carrying `packages` as well as this one's core.
    #[must_use]
    pub const fn with_packages(self, packages: &'a [PackageFloor<'a>]) -> Self {
        Self { packages, ..self }
    }

    /// The floor this query carries for `name`, if the package is active.
    #[must_use]
    pub fn package(&self, name: &str) -> Option<&PackageFloor<'a>> {
        self.packages.iter().find(|carried| carried.name == name)
    }

    /// Whether the package `name` is active in this query's context,
    /// whatever floor it carries.
    #[must_use]
    pub fn carries(&self, name: &str) -> bool {
        self.package(name).is_some()
    }

    /// Whether `other` is the same point: the same core and the same
    /// packages, **whatever floors they carry**.
    ///
    /// A floor refines a package that is already part of the point, so a
    /// registry's query, which carries the floors its packs declared, is
    /// still its profile's own point; `==` would call the two different.
    #[must_use]
    pub fn same_point(&self, other: &SurfaceQuery<'_>) -> bool {
        self.core == other.core
            && self.packages.len() == other.packages.len()
            && self
                .packages
                .iter()
                .all(|carried| other.carries(carried.name))
    }
}

impl SpecSurface {
    /// Whether this row admits `query`.
    #[must_use]
    pub fn admits(&self, query: &SurfaceQuery<'_>) -> bool {
        self.nearness(query).is_some()
    }

    /// The position of the nearest point of `query` this row admits: a core
    /// row's is the index of the first core point it matches, and a package
    /// row's is `0`, because a package the context carries is part of the
    /// context itself. `None` when the row admits nothing.
    fn nearness(&self, query: &SurfaceQuery<'_>) -> Option<usize> {
        match self.provider {
            SpecProvider::Core(family) => query
                .core
                .iter()
                .position(|(asked, release)| asked == family && self.covers(release)),
            // A package row's window is on the package's own axis, and the
            // query's point on it is the floor the context guarantees of the
            // package: a row introduced after the floor is not there yet.
            SpecProvider::Package(package) => query
                .package(package)
                .is_some_and(|carried| self.covers(carried.version))
                .then_some(0),
        }
    }

    /// Whether `point` falls in one of this row's windows — a release on the
    /// provider's ladder for a core row, a package's floor for a package
    /// row. An unstated point asks about the whole axis, which any window
    /// meets.
    ///
    /// Asked on every registry lookup, once per authored row, so it hands the
    /// bounds to [`crate::version::version_in_any_window`] rather than
    /// `format!`-ing a `"from-until"` requirement per window and re-parsing
    /// `point` behind each one (issue #2021).
    fn covers(&self, point: Option<&str>) -> bool {
        if self.windows.is_empty() {
            return true;
        }
        let Some(point) = point else {
            return true;
        };
        crate::version::version_in_any_window(point, self.windows)
    }

    /// Whether some release of the provider falls in a window of both this
    /// row and `other` — the two rows name the same package and their
    /// windows meet. An empty list of windows is every release.
    fn windows_meet(&self, other: &Self) -> bool {
        if self.windows.is_empty() || other.windows.is_empty() {
            return true;
        }
        self.windows.iter().any(|&(from, until)| {
            other.windows.iter().any(|&(other_from, other_until)| {
                precedes(from, other_until) && precedes(other_from, until)
            })
        })
    }
}

/// Whether `start` is before `end`, where no `end` is the end of the axis.
fn precedes(start: &str, end: Option<&str>) -> bool {
    end.is_none_or(|end| crate::version::compare_versions(start, end).is_lt())
}

/// Whether any row admits `query`.
///
/// An empty row list admits nothing. "Available everywhere" is the
/// *absent* gate — a `None` on an
/// `Option<&[SpecSurface]>` field — not an empty one.
///
/// An absent `query` is the caller asking surface-blind, as the plain
/// `CommandRegistry::get` does: nothing is filtered out.
#[must_use]
pub fn surface_admits(rows: &[SpecSurface], query: Option<&SurfaceQuery<'_>>) -> bool {
    match query {
        None => true,
        Some(query) => rows.iter().any(|row| row.admits(query)),
    }
}

/// How near to the query's own family the closest admitting row sits: the
/// index of the first core point of `query` some row admits, `0` for a
/// package row, and `None` when no row admits.
///
/// Selection between two visible specs of one command prefers the lower
/// number, so a row from a document's own family shadows a row it merely
/// inherits. A query with one core point ranks every admitted row `0`.
#[must_use]
pub fn surface_nearness(rows: &[SpecSurface], query: &SurfaceQuery<'_>) -> Option<usize> {
    rows.iter().filter_map(|row| row.nearness(query)).min()
}

/// Whether any row admits `family` at `release` **or at any later release on
/// its ladder**.
///
/// A different question from [`surface_admits`], which asks about one exact
/// point. "Compatible with 9.0 or later" must not be spelled as "available at
/// 9.0": a surface introduced *in* 9.1 is not available at 9.0, and asking the
/// exact question silently drops it.
#[must_use]
pub fn surface_admits_from(rows: &[SpecSurface], family: Family, release: &str) -> bool {
    let ladder = family.releases();
    let Some(first) = ladder.iter().position(|r| r.as_str() == release) else {
        return false;
    };
    ladder[first..]
        .iter()
        .any(|later| surface_admits(rows, Some(&SurfaceQuery::core(family, later.as_str()))))
}

/// One command surface a registry can have loaded.
///
/// The replacement for the retired "dialect bit" a registry recorded in its
/// `loaded_dialects` mask. A bit conflated two unlike things: a core
/// release, which brings no spec pack and only records which language the
/// registry is, and a vendor package, which brings one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceLayer {
    /// A core family at a release on its ladder.
    Core(Family, &'static str),
    /// A vendor package's compiled spec pack.
    Package(&'static str),
}

impl SurfaceLayer {
    /// The provider this layer supplies.
    #[must_use]
    pub const fn provider(self) -> SpecProvider {
        match self {
            Self::Core(family, _) => SpecProvider::Core(family),
            Self::Package(package) => SpecProvider::Package(package),
        }
    }
}

/// Whether any row is provided by one of `providers` — the coarse,
/// version-blind test.
///
/// The static grammars (tree-sitter, tmLanguage) highlight a command if the
/// profile's language has it at *any* release, because first-paint
/// highlighting has no resolved version to ask about; precision is the LSP
/// semantic-token layer's job.
#[must_use]
pub fn surface_provided_by(rows: &[SpecSurface], providers: &[SpecProvider]) -> bool {
    rows.iter().any(|row| providers.contains(&row.provider))
}

/// How much of the surface these rows cover — the most-specific-wins
/// measure: fewer covered provider-releases beats more.
///
/// Reproduces the retired mask's bit popcount: one per Tcl ladder release a
/// core-Tcl row covers, one per other core row, one per package row.
#[must_use]
pub fn surface_breadth(rows: &[SpecSurface]) -> u32 {
    rows.iter()
        .map(|row| match row.provider {
            SpecProvider::Core(Family::Tcl) => crate::version::TclVersion::ALL
                .iter()
                .filter(|release| row.covers(Some(release.version_string())))
                .count()
                .try_into()
                .unwrap_or(u32::MAX),
            _ => 1,
        })
        .sum()
}

/// Whether two row lists could ever be satisfied together — some provider
/// they share offers a release both admit.
///
/// The question a pack's `available` guard asks against the surface the pack
/// declared: not "is it available *here*" (that is [`surface_admits`]) but
/// "could this row list ever hold where that one does".
#[must_use]
pub fn surfaces_overlap(left: &[SpecSurface], right: &[SpecSurface]) -> bool {
    left.iter().any(|a| {
        right.iter().any(|b| {
            a.provider == b.provider
                && match a.provider {
                    SpecProvider::Core(family) => family.releases().iter().any(|release| {
                        let release = Some(release.as_str());
                        a.covers(release) && b.covers(release)
                    }),
                    // A package has no ladder to walk, so the windows
                    // themselves are compared: two rows for one package
                    // meet where both name a release.
                    SpecProvider::Package(_) => a.windows_meet(b),
                }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FROM_8_6: &[SpecWindow] = &[("8.6", None)];
    const THROUGH_8_6: &[SpecWindow] = &[("8.5", Some("9.0"))];
    const TWO_TRAINS: &[SpecWindow] = &[("1.0", Some("2.0")), ("3.0", None)];

    const TK_ONLY: &[PackageFloor<'static>] = &[PackageFloor::named("Tk")];
    const TK_AT_9_0: &[PackageFloor<'static>] = &[PackageFloor::at("Tk", "9.0")];
    const TK_AND_EXPECT: &[PackageFloor<'static>] =
        &[PackageFloor::named("Tk"), PackageFloor::named("expect")];
    const TK_AND_IAPPS: &[PackageFloor<'static>] =
        &[PackageFloor::named("Tk"), PackageFloor::named("iapps")];
    const EXPECT_5_45_TK_8_6: &[PackageFloor<'static>] = &[
        PackageFloor::at("expect", "5.45"),
        PackageFloor::at("Tk", "8.6"),
    ];
    const ITCL_9_0_TK_8_5: &[PackageFloor<'static>] = &[
        PackageFloor::at("Itcl", "9.0"),
        PackageFloor::at("Tk", "8.5"),
    ];

    fn tk(windows: &'static [SpecWindow]) -> SpecSurface {
        SpecSurface::package_in("Tk", windows)
    }

    fn admitted_at(row: SpecSurface, floor: &[PackageFloor<'_>]) -> bool {
        row.admits(&SurfaceQuery::any_release(Family::Tcl).with_packages(floor))
    }

    #[test]
    fn a_package_row_is_asked_about_the_floor_its_query_carries() {
        let from_8_6 = tk(FROM_8_6);
        for (floor, admitted) in [
            ("8.5", false),
            ("8.5.19", false),
            ("8.6", true),
            ("8.6.13", true),
            ("9.0", true),
        ] {
            assert_eq!(
                admitted_at(from_8_6, &[PackageFloor::at("Tk", floor)]),
                admitted,
                "a row from 8.6, asked at a floor of {floor}"
            );
        }
        let through_8_6 = tk(THROUGH_8_6);
        for (floor, admitted) in [("8.4", false), ("8.5", true), ("8.6", true), ("9.0", false)] {
            assert_eq!(
                admitted_at(through_8_6, &[PackageFloor::at("Tk", floor)]),
                admitted,
                "a row retired at 9.0, asked at a floor of {floor}"
            );
        }
        let two_trains = tk(TWO_TRAINS);
        for (floor, admitted) in [("1.5", true), ("2.0", false), ("2.5", false), ("3.0", true)] {
            assert_eq!(
                admitted_at(two_trains, &[PackageFloor::at("Tk", floor)]),
                admitted,
                "a row with a gap, asked at a floor of {floor}"
            );
        }
    }

    #[test]
    fn a_package_the_query_carries_without_a_floor_admits_every_window() {
        assert!(admitted_at(tk(FROM_8_6), &[PackageFloor::named("Tk")]));
        assert!(admitted_at(tk(THROUGH_8_6), &[PackageFloor::named("Tk")]));
        assert!(admitted_at(tk(TWO_TRAINS), &[PackageFloor::named("Tk")]));
    }

    #[test]
    fn a_package_the_query_does_not_carry_admits_no_row_whatever_its_windows() {
        for row in [SpecSurface::package("Tk"), tk(FROM_8_6), tk(THROUGH_8_6)] {
            assert!(!admitted_at(row, &[]));
            assert!(!admitted_at(row, &[PackageFloor::at("Itcl", "4.0")]));
        }
    }

    #[test]
    fn a_row_with_no_window_admits_a_carried_package_at_any_floor() {
        for floor in ["0.1", "8.4", "99.0"] {
            assert!(admitted_at(
                SpecSurface::package("Tk"),
                &[PackageFloor::at("Tk", floor)]
            ));
        }
    }

    #[test]
    fn a_package_floor_does_not_move_a_core_row() {
        let query = SurfaceQuery::core(Family::Tcl, "8.5").with_packages(TK_AT_9_0);
        assert!(!SpecSurface::core_in(Family::Tcl, FROM_8_6).admits(&query));
        assert!(SpecSurface::core_in(Family::Tcl, THROUGH_8_6).admits(&query));
    }

    #[test]
    fn the_floor_a_row_is_asked_about_is_the_one_named_for_its_package() {
        let query = SurfaceQuery::any_release(Family::Tcl).with_packages(ITCL_9_0_TK_8_5);
        assert!(!tk(FROM_8_6).admits(&query));
        assert!(SpecSurface::package_in("Itcl", FROM_8_6).admits(&query));
    }

    #[test]
    fn a_query_is_the_same_point_whatever_floors_it_carries() {
        let named = SurfaceQuery::core(Family::Tcl, "8.6").with_packages(TK_AND_EXPECT);
        let floored = SurfaceQuery::core(Family::Tcl, "8.6").with_packages(EXPECT_5_45_TK_8_6);
        assert!(named.same_point(&floored));
        assert!(floored.same_point(&named));
        assert_ne!(named, floored, "equality still sees the floors");

        let other_release = SurfaceQuery::core(Family::Tcl, "9.0").with_packages(TK_AND_EXPECT);
        let fewer = SurfaceQuery::core(Family::Tcl, "8.6").with_packages(TK_ONLY);
        let other_package = SurfaceQuery::core(Family::Tcl, "8.6").with_packages(TK_AND_IAPPS);
        for different in [other_release, fewer, other_package] {
            assert!(!named.same_point(&different));
            assert!(!different.same_point(&named));
        }
    }

    #[test]
    fn two_rows_for_one_package_meet_where_their_windows_do() {
        let meets = |a: &'static [SpecWindow], b: &'static [SpecWindow]| {
            surfaces_overlap(&[tk(a)], &[tk(b)])
        };
        assert!(meets(&[], FROM_8_6));
        assert!(meets(FROM_8_6, THROUGH_8_6));
        assert!(meets(THROUGH_8_6, FROM_8_6));
        assert!(
            !meets(&[("8.4", Some("8.6"))], FROM_8_6),
            "ends where the other starts"
        );
        assert!(!meets(FROM_8_6, &[("8.4", Some("8.6"))]));
        assert!(meets(TWO_TRAINS, &[("1.5", Some("1.6"))]));
        assert!(
            !meets(TWO_TRAINS, &[("2.0", Some("3.0"))]),
            "inside the gap"
        );
        assert!(!surfaces_overlap(
            &[tk(FROM_8_6)],
            &[SpecSurface::package_in("Itcl", FROM_8_6)]
        ));
    }

    const JIM_FROM_080: &[SpecSurface] = &[SpecSurface::core_in(Family::Jim, &[("0.80", None)])];

    fn jim_then_tcl() -> SurfaceQuery<'static> {
        SurfaceQuery {
            realm: crate::model::InvocationRealm::RuleLoader,
            core: CorePoints::two((Family::Jim, None), (Family::Tcl, Some("8.6"))),
            packages: &[],
        }
    }

    #[test]
    fn a_row_from_either_core_point_admits() {
        let query = jim_then_tcl();
        assert!(surface_admits(JIM_FROM_080, Some(&query)));
        assert!(surface_admits(SpecSurface::TCL86, Some(&query)));
        assert!(!surface_admits(SpecSurface::TCL84, Some(&query)));
        assert!(!surface_admits(SpecSurface::IRULES, Some(&query)));
    }

    #[test]
    fn a_jim_row_is_admitted_by_no_tcl_point() {
        for query in [
            SurfaceQuery::core(Family::Tcl, "8.6"),
            SurfaceQuery::any_release(Family::Tcl),
            SurfaceQuery::any_release(Family::F5Tcl),
        ] {
            assert!(!surface_admits(JIM_FROM_080, Some(&query)), "{query:?}");
        }
    }

    #[test]
    fn a_window_on_the_own_family_is_met_only_within_it() {
        let pinned = |release| SurfaceQuery {
            realm: crate::model::InvocationRealm::RuleLoader,
            core: CorePoints::two((Family::Jim, Some(release)), (Family::Tcl, Some("8.6"))),
            packages: &[],
        };
        assert!(surface_admits(JIM_FROM_080, Some(&pinned("0.82"))));
        assert!(!surface_admits(JIM_FROM_080, Some(&pinned("0.76"))));
    }

    #[test]
    fn nearness_ranks_the_own_family_ahead_of_the_anchor() {
        let query = jim_then_tcl();
        assert_eq!(surface_nearness(JIM_FROM_080, &query), Some(0));
        assert_eq!(surface_nearness(SpecSurface::TCL86, &query), Some(1));
        assert_eq!(surface_nearness(SpecSurface::TCL84, &query), None);
        let both = surface![
            SpecSurface::core(Family::Tcl),
            SpecSurface::core(Family::Jim)
        ];
        assert_eq!(surface_nearness(both, &query), Some(0));
    }

    #[test]
    fn a_query_with_one_core_point_ranks_every_admitted_row_alike() {
        let query = SurfaceQuery::core(Family::Tcl, "8.6").with_packages(TK_ONLY);
        assert_eq!(surface_nearness(SpecSurface::TCL86, &query), Some(0));
        assert_eq!(surface_nearness(SpecSurface::ALL_TCL, &query), Some(0));
        assert_eq!(surface_nearness(SpecSurface::TK, &query), Some(0));
        assert_eq!(surface_nearness(SpecSurface::TCL90, &query), None);
    }

    #[test]
    fn core_points_keep_their_order_and_drop_the_excess() {
        let points = [
            (Family::Jim, None),
            (Family::Tcl, Some("8.6")),
            (Family::F5Tcl, None),
        ];
        let kept = CorePoints::from_ordered(points);
        assert_eq!(kept, CorePoints::two(points[0], points[1]));
        assert_eq!(kept.iter().collect::<Vec<_>>(), points[..2]);
        assert_eq!(kept.nearest(), Some(points[0]));
        assert_eq!(kept.len(), 2);

        assert!(CorePoints::NONE.is_empty());
        assert_eq!(CorePoints::NONE.nearest(), None);
        assert_eq!(CorePoints::from_ordered([]), CorePoints::NONE);
        assert_eq!(
            CorePoints::one(Family::Tcl, None),
            CorePoints::from_ordered([(Family::Tcl, None)])
        );
        assert_eq!(CorePoints::one(Family::Tcl, None).len(), 1);
    }
}
