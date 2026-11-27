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

//! The surfaces a registry is built from, and the set a process installs.
//!
//! A [`Surface`] is one authored group of command specs with the facts that
//! say how a registry places it. The core names no surface: an entry point
//! [`install`]s its list once, and every default build, layer load and
//! cross-dialect index reads that set
//! (`docs/design/registry/registry-split/01-surface-input.md`).

use std::sync::OnceLock;

use rustc_hash::FxHashMap;
use tcl_dialect::model::SurfaceLayer;

use super::lookup::AllDialectCommandNames;
use crate::definer::DefinitionBodyGrammar;
use crate::spec::CommandSpec;
use crate::taint::TaintColour;

/// One authored group of command specs and how a registry places it.
///
/// Declared as a `static`: its [`SpecCell`] builds the specs on first use and
/// leaks them once, so every registry and index shares one copy.
#[derive(Debug)]
pub struct Surface {
    /// The provenance key: the `commands/<id>/` directory, or the pack name.
    pub id: &'static str,
    /// Author-facing name.
    pub label: &'static str,
    /// One line saying what the surface holds.
    pub blurb: &'static str,
    /// The repository directory the specs are authored in.
    pub source_dir: &'static str,
    /// Where the specs come from.
    pub specs: SpecSource,
    /// Which registries load it.
    pub placement: Placement,
    /// Filtered layers over the same specs.
    pub views: &'static [SurfaceView],
    /// The surface's hook registrations.
    pub hooks: &'static HookTable,
    /// The member grammar of a document whose layer this surface is.
    pub document_grammar: Option<&'static DefinitionBodyGrammar>,
    /// Whether [`CommandRegistry::known_in_any_dialect`] counts its names.
    ///
    /// [`CommandRegistry::known_in_any_dialect`]: super::CommandRegistry::known_in_any_dialect
    pub in_universe: bool,
    /// The built specs.
    pub cell: SpecCell,
}

/// Where a surface's specs come from.
#[derive(Debug, Clone, Copy)]
pub enum SpecSource {
    /// Shipped Rust, built once and leaked.
    Built(fn() -> Vec<CommandSpec>),
    /// Specs a crate above the registry produced at run time.
    Registered(fn() -> Vec<&'static CommandSpec>),
}

/// Which registries load a surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Every default build.
    Base,
    /// Every default build, which also counts `layer` as loaded.
    BaseLayer(SurfaceLayer),
    /// A registry whose profile's base layers name a layer with this
    /// layer's provider.
    Layer(SurfaceLayer),
}

/// A layer over a subset of its surface's specs.
#[derive(Debug)]
pub struct SurfaceView {
    /// The layer the view answers.
    pub layer: SurfaceLayer,
    /// Which of the surface's specs the view keeps.
    pub filter: fn(&CommandSpec) -> bool,
    /// The kept specs.
    pub cell: SpecCell,
}

/// A surface's hook registrations. Empty until behaviour moves to its surface.
#[derive(Debug)]
pub struct HookTable {
    _reserved: (),
}

impl HookTable {
    /// The table with no registrations.
    pub const EMPTY: Self = Self { _reserved: () };
}

/// The once-built specs of a surface or view.
#[derive(Debug, Default)]
pub struct SpecCell(OnceLock<&'static [&'static CommandSpec]>);

impl SpecCell {
    /// An unbuilt cell.
    #[must_use]
    pub const fn new() -> Self {
        Self(OnceLock::new())
    }
}

impl Surface {
    /// The surface's specs, built on first use.
    #[must_use]
    pub fn specs(&'static self) -> &'static [&'static CommandSpec] {
        self.cell.0.get_or_init(|| match self.specs {
            SpecSource::Built(build) => {
                let specs: &'static [CommandSpec] = Vec::leak(build());
                Vec::leak(specs.iter().collect())
            }
            SpecSource::Registered(produce) => Vec::leak(
                produce()
                    .into_iter()
                    .map(crate::native_procedure::attach_builtin_core_definition)
                    .collect(),
            ),
        })
    }

    /// The specs this surface brings to `layer`, when it claims it.
    ///
    /// A layer is claimed by provider, so a core layer at any release of the
    /// family matches.
    #[must_use]
    pub fn specs_for_layer(
        &'static self,
        layer: SurfaceLayer,
    ) -> Option<&'static [&'static CommandSpec]> {
        let provider = layer.provider();
        match self.placement {
            Placement::Layer(own) | Placement::BaseLayer(own) if own.provider() == provider => {
                return Some(self.specs());
            }
            _ => {}
        }
        self.views
            .iter()
            .find(|view| view.layer.provider() == provider)
            .map(|view| {
                *view.cell.0.get_or_init(|| {
                    Vec::leak(
                        self.specs()
                            .iter()
                            .copied()
                            .filter(|spec| (view.filter)(spec))
                            .collect(),
                    )
                })
            })
    }

    /// Whether every default build loads this surface.
    #[must_use]
    pub const fn is_base(&self) -> bool {
        matches!(self.placement, Placement::Base | Placement::BaseLayer(_))
    }

    /// Whether the specs are authored as shipped Rust under [`Self::source_dir`].
    #[must_use]
    pub const fn is_built(&self) -> bool {
        matches!(self.specs, SpecSource::Built(_))
    }
}

/// The surfaces a process installed, with the indexes built once over them.
pub struct SurfaceSet {
    surfaces: &'static [&'static Surface],
    universe: OnceLock<AllDialectCommandNames>,
    pack_by_identity: OnceLock<FxHashMap<usize, &'static str>>,
    packs_by_name: OnceLock<FxHashMap<&'static str, Vec<&'static str>>>,
    taint_sources: OnceLock<FxHashMap<&'static str, TaintColour>>,
    safe_interp_hidden: OnceLock<Vec<&'static str>>,
}

impl std::fmt::Debug for SurfaceSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.surfaces.iter().map(|surface| surface.id))
            .finish()
    }
}

impl SurfaceSet {
    const fn new(surfaces: &'static [&'static Surface]) -> Self {
        Self {
            surfaces,
            universe: OnceLock::new(),
            pack_by_identity: OnceLock::new(),
            packs_by_name: OnceLock::new(),
            taint_sources: OnceLock::new(),
            safe_interp_hidden: OnceLock::new(),
        }
    }

    /// Every installed surface, in install order.
    #[must_use]
    pub const fn surfaces(&self) -> &'static [&'static Surface] {
        self.surfaces
    }

    /// The surfaces every default build loads, in install order.
    #[must_use]
    pub fn base(&self) -> Vec<&'static Surface> {
        self.surfaces
            .iter()
            .copied()
            .filter(|surface| surface.is_base())
            .collect()
    }

    /// The installed surface with this id.
    #[must_use]
    pub fn surface(&self, id: &str) -> Option<&'static Surface> {
        self.surfaces
            .iter()
            .copied()
            .find(|surface| surface.id == id)
    }

    /// The installed surface that claims `layer`, and the specs it brings.
    #[must_use]
    pub fn layer(
        &self,
        layer: SurfaceLayer,
    ) -> Option<(&'static Surface, &'static [&'static CommandSpec])> {
        self.surfaces
            .iter()
            .find_map(|&surface| Some((surface, surface.specs_for_layer(layer)?)))
    }

    pub(super) fn universe(&self) -> &AllDialectCommandNames {
        self.universe.get_or_init(|| {
            let mut names = AllDialectCommandNames::default();
            for surface in self.surfaces.iter().filter(|surface| surface.in_universe) {
                for spec in surface.specs() {
                    names.add(spec);
                }
            }
            names
        })
    }

    /// Which built surface declares each spec, keyed by spec identity: a
    /// name declared in several surfaces resolves to one of them per dialect.
    pub(super) fn pack_by_identity(&self) -> &FxHashMap<usize, &'static str> {
        self.pack_by_identity.get_or_init(|| {
            let mut index = FxHashMap::default();
            for (pack, specs) in self.authored_groups() {
                for spec in specs {
                    index.insert(std::ptr::from_ref(*spec) as usize, pack);
                }
            }
            index
        })
    }

    /// Which built surfaces declare each bare name, in install order.
    pub(super) fn packs_by_name(&self) -> &FxHashMap<&'static str, Vec<&'static str>> {
        self.packs_by_name.get_or_init(|| {
            let mut index: FxHashMap<&'static str, Vec<&'static str>> = FxHashMap::default();
            for (pack, specs) in self.authored_groups() {
                for spec in specs {
                    let name =
                        tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name);
                    let packs = index.entry(name).or_default();
                    if !packs.contains(&pack) {
                        packs.push(pack);
                    }
                }
            }
            index
        })
    }

    /// Every built surface's specs and each of its views', paired with the
    /// surface id: a view's specs are authored in its surface's directory.
    fn authored_groups(
        &self,
    ) -> impl Iterator<Item = (&'static str, &'static [&'static CommandSpec])> + '_ {
        self.surfaces
            .iter()
            .filter(|surface| surface.is_built())
            .flat_map(|&surface| {
                std::iter::once(surface.specs())
                    .chain(
                        surface
                            .views
                            .iter()
                            .filter_map(move |view| surface.specs_for_layer(view.layer)),
                    )
                    .map(move |specs| (surface.id, specs))
            })
    }

    /// The getter-form taint colour each source command declares, from every
    /// installed surface whatever a registry loaded; the first declaration of
    /// a name wins.
    pub(super) fn taint_sources(&self) -> &FxHashMap<&'static str, TaintColour> {
        self.taint_sources.get_or_init(|| {
            let mut index = FxHashMap::default();
            for surface in self.surfaces {
                for spec in surface.specs() {
                    if let Some(colour) = spec.taint_source {
                        index.entry(spec.name).or_insert(colour);
                    }
                }
            }
            index
        })
    }

    /// Every command a safe interpreter hides, sorted; see
    /// [`crate::safe_interp_hidden_commands`].
    pub(crate) fn safe_interp_hidden(&self) -> &[&'static str] {
        self.safe_interp_hidden.get_or_init(|| {
            let registry = super::CommandRegistry::from_surfaces(&self.base());
            let mut names: Vec<&'static str> = registry
                .commands_with_trait(crate::Traits::SAFE_INTERP_HIDDEN)
                .into_iter()
                .filter_map(|name| registry.get(name).map(|spec| spec.name))
                .collect();
            names.sort_unstable();
            names.dedup();
            names
        })
    }
}

/// A second [`install`] offered a different surface list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallConflict {
    /// The ids of the list already installed.
    pub installed: Vec<&'static str>,
    /// The ids of the list offered.
    pub offered: Vec<&'static str>,
}

impl std::fmt::Display for InstallConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a surface set is already installed ({:?}); a process installs one ({:?} offered)",
            self.installed, self.offered
        )
    }
}

impl std::error::Error for InstallConflict {}

static INSTALLED: OnceLock<SurfaceSet> = OnceLock::new();

fn ids(surfaces: &[&'static Surface]) -> Vec<&'static str> {
    surfaces.iter().map(|surface| surface.id).collect()
}

/// Install the process's surface set. Idempotent for the same list; a
/// different list is an [`InstallConflict`].
///
/// # Errors
///
/// [`InstallConflict`] when a different list is already installed.
pub fn install(
    surfaces: &'static [&'static Surface],
) -> Result<&'static SurfaceSet, InstallConflict> {
    let set = INSTALLED.get_or_init(|| SurfaceSet::new(surfaces));
    let same = set.surfaces.len() == surfaces.len()
        && set
            .surfaces
            .iter()
            .zip(surfaces)
            .all(|(held, offered)| std::ptr::eq(*held, *offered));
    if same {
        Ok(set)
    } else {
        Err(InstallConflict {
            installed: ids(set.surfaces),
            offered: ids(surfaces),
        })
    }
}

/// The installed surface set, when an entry point has installed one.
#[must_use]
pub fn try_installed() -> Option<&'static SurfaceSet> {
    INSTALLED.get()
}

/// What [`installed`] panics with before any install.
pub const NOT_INSTALLED: &str = "no command surface set is installed: an entry point must call \
     tcl_registry::install (a test harness, tcl_registry::testing::install_shipped!()) \
     before its first registry read; see docs/design/registry/registry-split/01-surface-input.md";

/// The installed surface set.
///
/// # Panics
///
/// When nothing is installed: the entry point has not done its duty.
#[must_use]
pub fn installed() -> &'static SurfaceSet {
    if let Some(set) = INSTALLED.get() {
        return set;
    }
    install(crate::commands::SHIPPED).unwrap_or_else(|conflict| panic!("{conflict}"))
}
