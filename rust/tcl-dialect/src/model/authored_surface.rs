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
//! This replaces the retired `SpecSurface` bitmask (Q13). The bits could name
//! only whole Tcl lines and a fixed vendor list, which is why Jim's own
//! commands were unexpressible: there was no Jim bit, and one bit could not
//! have carried `{jim 0.81-}` anyway. A row names its provider and its window
//! on *that provider's* axis, so both fall out.

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
/// Each names exactly the rows the retired `SpecSurface` bit or union
/// lowered to, so a spec that said `surface: Some(SpecSurface::TCL85_PLUS)`
/// now says `surface: Some(SpecSurface::TCL85_PLUS)` and gets the same
/// answer. They exist because ~1,800 compiled specs spell one of a dozen
/// windows, and naming each once keeps the data readable.
///
/// The upper bounds are the ladder's, not infinity: `TCL85_PLUS` is
/// `8.5-9.2`, because the bitmask it replaces was a union of the five
/// *known* line bits and could not mean "and every line added later". A
/// spec that genuinely wants open-ended availability writes
/// [`SpecSurface::core_in`] with a `None` upper bound; the migration did
/// not widen any spec on its own.
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
    pub const TK_AND_TCL: &'static [Self] = &[
        Self::core_in(Family::Tcl, &W_ALL_TCL),
        Self::package("Tk"),
    ];
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

/// The point a surface question is asked at — the replacement for the
/// retired availability mask (Q13).
///
/// A mask conflated two different facts in one word: which Tcl *line* a
/// context is, and which vendor surface it carries. A point states both
/// separately, so a question can be asked about a release the bitmask had
/// no bit for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceQuery<'a> {
    /// The core family the context resolves to, and the release on its
    /// ladder. The outer `None` is a context with no core runtime of its
    /// own — the BIG-IP config surface, the permissive fallback. The inner
    /// `None` is *any* release of that family: what the mask said by
    /// setting every ladder bit, which a context with no resolved primary
    /// still needs to say.
    pub core: Option<(Family, Option<&'a str>)>,
    /// The packages active in the context.
    pub packages: &'a [&'a str],
}

impl<'a> SurfaceQuery<'a> {
    /// A query at `family`'s `release`, with no packages.
    #[must_use]
    pub const fn core(family: Family, release: &'a str) -> Self {
        Self {
            core: Some((family, Some(release))),
            packages: &[],
        }
    }

    /// A query at any release of `family`, with no packages.
    #[must_use]
    pub const fn any_release(family: Family) -> Self {
        Self {
            core: Some((family, None)),
            packages: &[],
        }
    }

    /// A query carrying `packages` as well as this one's core.
    #[must_use]
    pub const fn with_packages(self, packages: &'a [&'a str]) -> Self {
        Self { packages, ..self }
    }
}

impl SpecSurface {
    /// Whether this row admits `query`.
    #[must_use]
    pub fn admits(&self, query: &SurfaceQuery<'_>) -> bool {
        match self.provider {
            SpecProvider::Core(family) => match query.core {
                Some((asked, release)) => asked == family && self.covers(release),
                None => false,
            },
            // A package row's window is on the package's own axis, which a
            // query carries no point on: the resolved context narrows by the
            // placement floor instead.
            SpecProvider::Package(package) => query.packages.contains(&package),
        }
    }

    /// Whether `release` falls in one of this row's windows. An unstated
    /// release asks about the whole ladder, which any window meets.
    fn covers(&self, release: Option<&str>) -> bool {
        if self.windows.is_empty() {
            return true;
        }
        let Some(release) = release else {
            return true;
        };
        self.windows
            .iter()
            .any(|&(from, until)| crate::version::version_satisfies(release, &requirement(from, until)))
    }

}

/// The requirement spelling for a half-open window.
fn requirement(from: &str, until: Option<&str>) -> String {
    match until {
        Some(until) => format!("{from}-{until}"),
        None => format!("{from}-"),
    }
}

/// Whether any row admits `query` — the replacement for the retired
/// `the retired availability mask::intersects`.
///
/// An empty row list admits nothing, exactly as the empty mask matched
/// nothing. "Available everywhere" is the *absent* gate — a `None` on an
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
                    // A package row carries no point on the package's axis
                    // here, so naming the same package is the whole answer.
                    SpecProvider::Package(_) => true,
                }
        })
    })
}
