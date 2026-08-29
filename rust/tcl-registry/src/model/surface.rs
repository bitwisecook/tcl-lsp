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

//! Surface declarations (redesign §4.1) and the mechanical translation
//! from today's [`CommandSpec`] availability fields.
//!
//! A [`SurfaceDeclaration`] says *who provides this shape and when*: a
//! [`Provider`] (a core family or a named package), an axis-typed
//! [`VersionSet`] of applicability, a [`CapabilityPredicate`] for
//! build/platform/feature conditions, the item's own [`ItemHistory`], and
//! its trust [`Provenance`]. A spec's availability facts are a small
//! **disjunction** of such rows — some row must hold for the spec to be
//! available in a context — with the one conjunctive residue of the old
//! model (`required_package` on a closed-world package) carried as a
//! per-row predicate.
//!
//! [`declarations_for_spec`] derives the rows mechanically from the old
//! fields (`dialects`, `required_package`, `tcllib_package`, `lifecycle`),
//! so the compiled catalogue needs no re-authoring in P1. The equivalence
//! sweeps in [`crate::model::assembly`] pin the translation to the old
//! `supports_dialect`/`ProfileQueries::is_available` semantics for every
//! compiled spec under every catalogue profile.

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use smallvec::SmallVec;
use tcl_dialect::model::{
    SpecProvider, SpecSurface, SpecWindow, SurfaceQuery, surface_admits,
    Family, ItemHistory, Provenance, Version, VersionAxisId, VersionSet, compiled_definitions,
};

use crate::lifecycle::Lifecycle;
use crate::spec::CommandSpec;

/// The interned identity of one package provider (`"Tk"`, `"csv"`,
/// `"iapps"`, `"f5-irules-cmds"`, …) — §4.1's `PackageId`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId(Arc<str>);

impl PackageId {
    /// The id for the package named `name`, as spec data spells it.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(Arc::from(name))
    }

    /// The package name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The package's own version axis.
    #[must_use]
    pub fn axis(&self) -> VersionAxisId {
        VersionAxisId::package(&self.0)
    }
}

impl std::fmt::Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Who provides a surface item (§4.1): a core family's own surface, or a
/// named package.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Provider {
    /// The family's core surface (`lmap` on [`Family::Tcl`]).
    Core(Family),
    /// A named package's surface (`"Tk"`, `"struct::graph"`, `"iapps"`).
    Package(PackageId),
    /// The analysed document (or its workspace sidecar) declaring a command
    /// for itself — gap ruling R1's `# tcl-lsp: stub` surface, carried by
    /// [`crate::model::declaration::DeclaredCommand`].
    ///
    /// A document provider is active exactly in the document that declared
    /// it and nowhere else, which is why such a declaration never joins a
    /// shared [`crate::model::ContextRegistry`] generation: the generation
    /// is keyed by environment, and this row's scope is one buffer.
    Document,
}

/// A typed build capability a predicate can require — the probe columns of
/// [`tcl_dialect::model::CapabilitySet`], named so a declaration can gate
/// on one (review B1/B5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildCapability {
    /// The build counts characters rather than bytes.
    Utf8CharacterModel,
    /// The expr math-function extension is compiled in.
    MathExtension,
}

/// A build/platform/feature condition on one declaration (§4.1's
/// predicate slot). Minimal on purpose and extensible by variant: each new
/// condition kind is a new variant, never a stringly side channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityPredicate {
    /// No condition — the declaration applies wherever its provider is
    /// active.
    None,
    /// The resolved core build must answer
    /// [`tcl_dialect::model::CapabilityAnswer::Yes`] for the named
    /// capability; `No` and `Unknown` both fail (an unmeasured build never
    /// silently passes a measured gate — B1).
    RequiresCapability(BuildCapability),
    /// The named package must be active in the resolved context.
    ///
    /// This is the translated form of the old model's **conjunctive**
    /// `required_package` gate for a closed-world package
    /// (`profile_queries::package_available`): the spec's dialect-derived
    /// rows stay the availability carriers, and this predicate keeps them
    /// from resolving where the closed-world package is not shipped. When
    /// packs migrate (P2) such specs become genuine
    /// [`Provider::Package`]-rowed declarations and the predicate form
    /// retires.
    RequiresPackage(PackageId),
}

/// One §4.1 surface declaration: provider, axis-typed applicability,
/// predicate, per-item history, and trust class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceDeclaration {
    /// Who provides the item.
    pub provider: Provider,
    /// When this shape exists, on the provider's axis (parallel trains are
    /// several rows).
    pub applicable: VersionSet,
    /// Build/platform/feature condition (B1/B5), plus the translated
    /// closed-world `required_package` gate.
    pub predicate: CapabilityPredicate,
    /// The item's own introduced/deprecated/retired story. For a spec with
    /// an owning package this lives on **that package's** version axis
    /// (the BIG-IP release for the F5 surfaces, the tcllib package train);
    /// otherwise on the provider's own axis. Metadata, not an availability
    /// gate: the old model's `is_available` never consulted it, and query
    /// consumers read it through state queries, exactly as before.
    pub history: ItemHistory,
    /// Where the declaration came from (§6.4). Every mechanically
    /// translated compiled spec is [`Provenance::BuiltIn`].
    pub provenance: Provenance,
}

/// The interim environment-id → vendor-surface-package bridge: which
/// [`PackageId`] carries each environment's **own** command surface,
/// mirroring the old `DialectProfile::vendor_surface`.
///
/// This is a **documented P2 seam**: today the environments for `spectcl`
/// and `bpf` place no packages at all and the F5/expect environments place
/// their keyed catalogue packs under other names, so these surface
/// packages need an activation home until the command packs
/// migrate to real pack-declared ambient placements. When they do, each
/// row here becomes an ordinary ambient [`PackagePlacement`] on its
/// environment and this table is deleted.
///
/// `f5-irules` is absent by design — its surface translates to
/// [`Provider::Core`]`(`[`Family::F5Irules`]`)`, not to a package. The
/// `tk` environment is also absent: `Tk` is a hosted library activated by
/// its placement, never a closed-world vendor surface (review B11).
///
/// [`PackagePlacement`]: tcl_dialect::model::PackagePlacement
pub const VENDOR_SURFACE_BRIDGE: &[(&str, &str)] = &[
    ("f5-iapps", "iapps"),
    ("f5-tmsh", "tmsh"),
    ("expect", "expect"),
    ("spectcl", "spectcl"),
    ("bpf", "bpf"),
    ("f5-bigip", "bigip"),
];

/// The packages that are part of the **authoring** vocabulary: a spec
/// naming one of these is scoped to a vendor surface, so
/// [`crate::model::context::specificity_breadth`] counts it.
///
/// `"Tk"` is in the set even though it is a hosted library rather than a
/// closed-world surface — a Tk command really is narrowly authored.
/// Whether such a package is *active* is [`is_closed_world_package`]'s
/// separate job.
pub const VENDOR_SURFACE_PACKAGES: &[&str] =
    &["iapps", "tmsh", "Tk", "expect", "spectcl", "bpf", "bigip"];

/// The vendor-surface package of the environment named
/// `environment_id`, if the interim bridge carries one.
#[must_use]
pub fn vendor_surface_package(environment_id: &str) -> Option<&'static str> {
    VENDOR_SURFACE_BRIDGE
        .iter()
        .find(|&&(environment, _)| environment == environment_id)
        .map(|&(_, package)| package)
}

/// The catalogue dialect names a surface row list covers.
///
/// The one projection from rows back onto the catalogue's own dialect ids,
/// shared by every consumer that names dialects rather than rows (the
/// command snapshot, the spec studio's drafts). A core row expands to one
/// name per ladder release it covers; a package row names the environment
/// that ships it.
#[must_use]
pub fn dialect_names_for_rows(rows: &[SpecSurface]) -> Vec<String> {
    let mut names: Vec<String> = rows.iter().flat_map(row_dialect_names).collect();
    names.sort_unstable();
    names.dedup();
    names
}

fn row_dialect_names(row: &SpecSurface) -> Vec<String> {
    match row.provider {
        SpecProvider::Core(Family::Tcl) => tcl_dialect::TclVersion::ALL
            .iter()
            .filter(|release| {
                surface_admits(
                    std::slice::from_ref(row),
                    Some(&SurfaceQuery::core(Family::Tcl, release.version_string())),
                )
            })
            .map(|release| format!("tcl{}", release.version_string()))
            .collect(),
        SpecProvider::Core(Family::F5Irules) => vec!["f5-irules".to_owned()],
        SpecProvider::Core(Family::F5Tcl) => vec!["f5-tcl".to_owned()],
        SpecProvider::Core(Family::Jim) => vec!["jim".to_owned()],
        SpecProvider::Package(package) => vendor_surface_environment(package)
            .map_or_else(|| vec![package.to_owned()], |id| vec![id.to_owned()]),
    }
}

/// Whether two points answer every availability question alike.
///
/// The context's authoring point carries the *measured* release where the
/// profile's carries the ladder line — the F5 trunk's fork patchlevel
/// `8.4.6` against `8.4` — so the two are not structurally equal and must
/// not be compared as if they were. What has to hold is that no shipped
/// surface tells them apart.
#[must_use]
pub fn points_answer_alike(left: &SurfaceQuery<'_>, right: &SurfaceQuery<'_>) -> bool {
    const PROBES: &[&[SpecSurface]] = &[
        SpecSurface::ALL_TCL,
        SpecSurface::TCL84,
        SpecSurface::TCL85,
        SpecSurface::TCL86,
        SpecSurface::TCL90,
        SpecSurface::TCL91,
        SpecSurface::TCL8X,
        SpecSurface::TCL85_PLUS,
        SpecSurface::TCL86_PLUS,
        SpecSurface::TCL90_PLUS,
        SpecSurface::IRULES,
        SpecSurface::JIM,
        SpecSurface::IAPPS,
        SpecSurface::TMSH,
        SpecSurface::TK,
        SpecSurface::EXPECT,
        SpecSurface::SPECTCL,
        SpecSurface::BPF,
        SpecSurface::BIGIP,
        SpecSurface::TK_AND_TCL,
    ];
    PROBES
        .iter()
        .all(|rows| surface_admits(rows, Some(left)) == surface_admits(rows, Some(right)))
}

/// The environment id a vendor `package` stands behind — the inverse of
/// [`vendor_surface_package`], for a projection that names dialects rather
/// than packages (the command snapshot).
#[must_use]
pub fn vendor_surface_environment(package: &str) -> Option<&'static str> {
    // `Tk` has no vendor-shell environment: the `tk` ingress profile is a
    // library pin, not a vendor surface, so it is named directly.
    if package == "Tk" {
        return Some("tk");
    }
    VENDOR_SURFACE_BRIDGE
        .iter()
        .find(|&&(_, bridged)| bridged == package)
        .map(|&(environment, _)| environment)
}

/// Every package some compiled environment ships **ambient** — its keyed
/// catalogue packs (`f5-irules-cmds`, the EDA tool surfaces, `Expect`).
/// `pub(crate)`: the closed-world classification below is built from it.
pub(crate) fn ambient_placement_packages() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| {
        compiled_definitions()
            .iter()
            .flat_map(|definition| definition.expected_packages.iter())
            .filter(|placement| placement.ambient)
            .map(|placement| placement.package.as_ref().to_owned())
            .collect()
    })
}

/// Every package some compiled environment offers **hosted** — the
/// installable libraries (`Tk`, `Itcl`). A package in this set is a
/// library, not a vendor runtime, however many environments also ship it
/// ambient.
pub(crate) fn hosted_placement_packages() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| {
        compiled_definitions()
            .iter()
            .flat_map(|definition| definition.expected_packages.iter())
            .filter(|placement| !placement.ambient)
            .map(|placement| placement.package.as_ref().to_owned())
            .collect()
    })
}

/// Whether `package` is a **closed-world** package: part of some
/// environment's modelled runtime and *only* that (an ambient placement no
/// environment also hosts, or a [`VENDOR_SURFACE_BRIDGE`] surface), and
/// therefore never resolvable where that environment's world does not ship
/// it.
///
/// The mirror of the old model's `vendor_ambient_packages` set, with the
/// classification stated positively: a hosted library (tcllib, the stdlib
/// packages) is **not** closed-world, and a `required_package` gate on one
/// never hides the command (W120 nags about the missing require instead).
///
/// **P3 (the Tk pilot).** `Tk` is the case that forces the "and hosted
/// nowhere" conjunct to be written down. It is ambient under the `tk`
/// environment (`wish` has already loaded it) *and* hosted under every
/// plain-Tcl environment (`tclsh` needs the `package require`) — a library
/// with an ambient host, not a vendor runtime. Reading ambience alone
/// would make Tk closed-world the moment the pilot placed it, and every Tk
/// command would vanish from plain Tcl. The rule is data-driven, so a
/// later pack that places a library ambient in its own environment gets
/// the same answer with no table to edit.
#[must_use]
pub fn is_closed_world_package(package: &str) -> bool {
    if hosted_placement_packages().contains(package) {
        return false;
    }
    VENDOR_SURFACE_BRIDGE
        .iter()
        .any(|&(_, bridged)| bridged == package)
        || ambient_placement_packages().contains(package)
}

/// Whether `package`'s availability is a **placement** question — some
/// compiled environment ships it as part of its own runtime (an ambient
/// placement, or a [`VENDOR_SURFACE_BRIDGE`] surface) — rather than the
/// old "a `required_package` gate never hides anything" leniency.
///
/// A package no environment runs ambiently is an ordinary installable
/// library (`Itcl`, every tcllib module): nothing in the model knows where
/// it is or is not present, so its declarations carry no package conjunct
/// and W120 owns the nag, exactly as before. **P3** brings `Tk` into this
/// set — `wish` runs it ambiently — which is what routes all 68 Tk specs
/// through [`ResolvedContext::package_active`].
///
/// [`ResolvedContext::package_active`]: crate::model::ResolvedContext::package_active
#[must_use]
pub fn is_placement_gated_package(package: &str) -> bool {
    ambient_placement_packages().contains(package)
        || VENDOR_SURFACE_BRIDGE
            .iter()
            .any(|&(_, bridged)| bridged == package)
}

/// The whole of `axis` — `0-`, every well-formed version.
fn full_axis(axis: VersionAxisId) -> VersionSet {
    VersionSet::from_requirements(axis, &["0-"]).expect("the full-axis requirement is well-formed")
}

/// The core-Tcl coverage of a gate's rows, as a version set.
///
/// `None` when the gate names no plain-Tcl line at all — the item is
/// another provider's, and that provider's own axis governs it.
pub(crate) fn core_tcl_set(gate: &[SpecSurface]) -> Option<VersionSet> {
    let axis = VersionAxisId::core(Family::Tcl);
    let mut requirements: Vec<String> = Vec::new();
    for row in gate {
        if row.provider != SpecProvider::Core(Family::Tcl) {
            continue;
        }
        if row.windows.is_empty() {
            requirements.push("0-".to_owned());
            continue;
        }
        for &(from, until) in row.windows {
            requirements.push(match until {
                Some(until) => format!("{from}-{until}"),
                None => format!("{from}-"),
            });
        }
    }
    if requirements.is_empty() {
        return None;
    }
    Some(
        VersionSet::from_requirements(axis, &requirements)
            .expect("compiled surface windows are well-formed"),
    )
}

/// [`Lifecycle`] restated as the new model's [`ItemHistory`]. The release
/// strings are compile-time curated plain dotted versions; the sweep in
/// this module's tests asserts every compiled one parses, so the lossy
/// `.ok()` here can only ever drop a string the old comparator would have
/// mis-ordered anyway.
fn item_history(lifecycle: &Lifecycle) -> ItemHistory {
    let parse = |text: Option<&'static str>| text.and_then(|text| Version::parse(text).ok());
    ItemHistory {
        introduced: parse(lifecycle.introduced),
        deprecated: parse(lifecycle.deprecated),
        retired: parse(lifecycle.retired),
    }
}

fn row(provider: Provider, applicable: VersionSet, history: ItemHistory) -> SurfaceDeclaration {
    SurfaceDeclaration {
        provider,
        applicable,
        predicate: CapabilityPredicate::None,
        history,
        provenance: Provenance::BuiltIn,
    }
}

/// A [`Provider::Package`] row for `name`, applicable over the package's
/// **declared** version set.
///
/// For a tcllib module (P5) that set is the union of the trains its
/// `pkgIndex.tcl` offers — `md5` is `[1.4.6, 2) ∪ [2.0.9, 3)`, not one
/// span and not one point — read from
/// [`crate::model::tcllib::module_version_set`]. For every other package
/// the model has no per-version evidence yet, so the row keeps the
/// full-axis fallback and the floor comes entirely from the document's
/// `package require`.
fn package_row(name: &str, history: ItemHistory) -> SurfaceDeclaration {
    let id = PackageId::new(name);
    let applicable =
        crate::model::tcllib::module_version_set(name).unwrap_or_else(|| full_axis(id.axis()));
    row(Provider::Package(id), applicable, history)
}

/// Lower one spec's authored availability into the runtime declarations
/// the context layer answers against (§4.1).
///
/// Each authored [`SpecSurface`] row becomes one declaration: a core row
/// carries its windows as a version set on that family's axis, a package
/// row covers its package's whole axis.
///
/// A spec that states **no** surface is available wherever it is asked
/// about, so it lowers to one row per provider there is — every core
/// family over its full axis, and every vendor package. That is wider than
/// any stated surface, which is what makes a catch-all lose to every
/// scoped spec under [`crate::model::context::specificity_breadth`].
///
/// `required_package`/`tcllib_package` **add** a full-axis
/// [`Provider::Package`] row for the owning package — where the command
/// comes from — and `required_package` also **constrains** every row with
/// [`CapabilityPredicate::RequiresPackage`], the conjunct that routes the
/// item's availability through [`ResolvedContext::package_active`].
/// `lifecycle` populates every row's [`ItemHistory`].
///
/// [`ResolvedContext::package_active`]: crate::model::ResolvedContext::package_active
#[must_use]
pub fn declarations_for_spec(spec: &CommandSpec) -> SmallVec<[SurfaceDeclaration; 2]> {
    let history = item_history(&spec.lifecycle);
    let mut rows: SmallVec<[SurfaceDeclaration; 2]> = SmallVec::new();
    match spec.surface {
        Some(authored) => {
            for authored_row in authored {
                rows.push(lower(authored_row, history.clone()));
            }
        }
        // A spec that names no provider is available wherever it is asked
        // about, which is one row per provider there is.
        None => {
            for family in Family::ALL {
                rows.push(row(
                    Provider::Core(family),
                    full_axis(VersionAxisId::core(family)),
                    history.clone(),
                ));
            }
            for package in VENDOR_SURFACE_PACKAGES {
                rows.push(package_row(package, history.clone()));
            }
        }
    }
    if let Some(package) = spec.owning_package() {
        let id = PackageId::new(package);
        if !rows
            .iter()
            .any(|existing| existing.provider == Provider::Package(id.clone()))
        {
            rows.push(package_row(package, history));
        }
    }
    // The require is stated on every row unconditionally; how strictly it
    // is read — leniently in an open world, literally in one where a pack
    // is absent until required — is the context's call, not the lowering's.
    if let Some(package) = spec.required_package {
        let id = PackageId::new(package);
        for declaration in &mut rows {
            declaration.predicate = CapabilityPredicate::RequiresPackage(id.clone());
        }
    }
    rows
}

/// Lower one authored row to the runtime declaration it stands for.
fn lower(authored: &SpecSurface, history: ItemHistory) -> SurfaceDeclaration {
    match authored.provider {
        SpecProvider::Core(family) => {
            let axis = VersionAxisId::core(family);
            let applicable = if authored.windows.is_empty() {
                full_axis(axis)
            } else {
                window_set(axis, authored.windows)
            };
            row(Provider::Core(family), applicable, history)
        }
        SpecProvider::Package(package) => package_row(package, history),
    }
}

/// The version set a row's windows describe, on `axis`.
fn window_set(axis: VersionAxisId, windows: &[SpecWindow]) -> VersionSet {
    let requirements: Vec<String> = windows
        .iter()
        .map(|&(from, until)| match until {
            Some(until) => format!("{from}-{until}"),
            None => format!("{from}-"),
        })
        .collect();
    VersionSet::from_requirements(axis, &requirements)
        .expect("compiled surface windows are well-formed")
}

#[cfg(test)]
mod tests {
    use tcl_dialect::surface;
    use tcl_dialect::model::{SpecSurface, Family};
    use super::*;
    use crate::spec::CommandSpec;

    fn spec_with(surface: Option<&'static [SpecSurface]>) -> CommandSpec {
        CommandSpec {
            name: "surface-test",
            surface,
            ..CommandSpec::DEFAULT
        }
    }

    fn v(text: &str) -> Version {
        Version::parse(text).expect("test version")
    }

    fn core_tcl_row(rows: &[SurfaceDeclaration]) -> &SurfaceDeclaration {
        rows.iter()
            .find(|row| row.provider == Provider::Core(Family::Tcl))
            .expect("a Core(Tcl) row")
    }

    #[test]
    fn contiguous_version_bits_fold_to_one_range() {
        let rows = declarations_for_spec(&spec_with(Some(SpecSurface::TCL85_PLUS)));
        assert_eq!(rows.len(), 1);
        let core = core_tcl_row(&rows);
        assert_eq!(core.applicable.ranges().len(), 1);
        for admitted in ["8.5", "8.6", "8.6.16", "9.0", "9.1", "9.1.2"] {
            assert!(core.applicable.contains(&v(admitted)), "{admitted}");
        }
        assert!(!core.applicable.contains(&v("8.4.19")));
        assert!(!core.applicable.contains(&v("9.2")));
        // The 8.x-only surface keeps its exclusive top.
        let tcl8x = declarations_for_spec(&spec_with(Some(SpecSurface::TCL8X)));
        let core = core_tcl_row(&tcl8x);
        assert!(core.applicable.contains(&v("8.6.16")));
        assert!(!core.applicable.contains(&v("9.0")));
    }

    #[test]
    fn a_ladder_gap_stays_two_ranges() {
        let rows =
            declarations_for_spec(&spec_with(Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.4", Some("8.5")), ("8.6", Some("8.7"))])])));
        let core = core_tcl_row(&rows);
        assert_eq!(core.applicable.ranges().len(), 2);
        assert!(core.applicable.contains(&v("8.4")));
        assert!(core.applicable.contains(&v("8.6")));
        assert!(!core.applicable.contains(&v("8.5.19")));
    }

    #[test]
    fn irules_translates_to_the_core_family_not_a_package() {
        let rows = declarations_for_spec(&spec_with(Some(SpecSurface::IRULES)));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].provider, Provider::Core(Family::F5Irules));
        // The iRules-enabled Tcl core carries both families.
        let both = declarations_for_spec(&spec_with(Some(SpecSurface::ALL_TCL_AND_IRULES)));
        assert_eq!(both.len(), 2);
        assert!(
            both.iter()
                .any(|row| row.provider == Provider::Core(Family::Tcl))
        );
        assert!(
            both.iter()
                .any(|row| row.provider == Provider::Core(Family::F5Irules))
        );
    }

    #[test]
    fn vendor_bits_translate_to_full_axis_package_rows() {
        let rows =
            declarations_for_spec(&spec_with(Some(surface![SpecSurface::package("iapps"), SpecSurface::package("tmsh")])));
        let packages: Vec<&str> = rows
            .iter()
            .filter_map(|row| match &row.provider {
                Provider::Package(id) => Some(id.as_str()),
                Provider::Core(_) | Provider::Document => None,
            })
            .collect();
        assert_eq!(packages, ["iapps", "tmsh"]);
        for row in &rows {
            assert!(row.applicable.contains(&v("1.0")));
            assert!(row.applicable.contains(&v("99.99")));
            assert_eq!(row.predicate, CapabilityPredicate::None);
            assert_eq!(row.provenance, Provenance::BuiltIn);
        }
    }

    #[test]
    fn none_dialects_translate_to_every_provider_the_old_mask_admitted() {
        let rows = declarations_for_spec(&spec_with(None));
        // Four core families (`f5-tcl` joined the tree in the F5
        // reclassification, measurements §4a) + seven vendor packages.
        assert_eq!(rows.len(), 11);
        for family in Family::ALL {
            assert!(
                rows.iter()
                    .any(|row| row.provider == Provider::Core(family)),
                "{family}"
            );
        }
        for package in VENDOR_SURFACE_PACKAGES {
            assert!(
                rows.iter()
                    .any(|row| row.provider == Provider::Package(PackageId::new(package))),
                "{package}"
            );
        }
    }

    #[test]
    fn a_hosted_owning_package_adds_a_row_and_states_the_require() {
        let spec = CommandSpec {
            name: "surface-test",
            surface: Some(SpecSurface::ALL_TCL),
            required_package: Some("csv"),
            tcllib_package: Some("csv"),
            ..CommandSpec::DEFAULT
        };
        let rows = declarations_for_spec(&spec);
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .any(|row| row.provider == Provider::Package(PackageId::new("csv")))
        );
        // Every row carries the require; a lenient world reads it
        // leniently, an ambient-plus-require world literally.
        assert!(rows.iter().all(|row| row.predicate
            == CapabilityPredicate::RequiresPackage(PackageId::new("csv"))));
    }

    /// **P5.** A tcllib module's package row is applicable over the
    /// module's own **trains**, not over the whole axis: the version
    /// range is a property of the declaration, on the module's own axis,
    /// and a parallel-train module contributes two disjoint ranges.
    #[test]
    fn a_tcllib_module_row_carries_its_declared_trains() {
        let row_for = |package: &'static str| {
            let spec = CommandSpec {
                name: "surface-test",
                surface: Some(SpecSurface::ALL_TCL),
                required_package: Some(package),
                tcllib_package: Some(package),
                ..CommandSpec::DEFAULT
            };
            declarations_for_spec(&spec)
                .into_iter()
                .find(|row| row.provider == Provider::Package(PackageId::new(package)))
                .expect("a package row")
        };
        // Single train: the whole major line, never the shipped point.
        let csv = row_for("csv");
        assert_eq!(csv.applicable.axis(), &VersionAxisId::package("csv"));
        assert_eq!(csv.applicable.ranges().len(), 1);
        assert!(csv.applicable.contains(&v("0.10")));
        assert!(!csv.applicable.contains(&v("1.0")));
        // Parallel trains: two disjoint ranges on one axis.
        let tree = row_for("struct::tree");
        assert_eq!(tree.applicable.ranges().len(), 2);
        assert!(tree.applicable.contains(&v("1.2.3")));
        assert!(tree.applicable.contains(&v("2.1.3")));
        assert!(!tree.applicable.contains(&v("2.0")));
        // A package the census does not carry keeps the full-axis
        // fallback — the model has no per-version evidence for it.
        let http = row_for("http");
        assert!(http.applicable.contains(&v("0.1")));
        assert!(http.applicable.contains(&v("99.0")));
    }

    #[test]
    fn a_closed_world_required_package_constrains_every_row() {
        // The `HTTP2::header` shape: an iRules spec gated on the keyed
        // catalogue pack.
        let spec = CommandSpec {
            name: "surface-test",
            surface: Some(SpecSurface::IRULES),
            required_package: Some("f5-irules-cmds"),
            ..CommandSpec::DEFAULT
        };
        let rows = declarations_for_spec(&spec);
        assert_eq!(rows.len(), 2, "core row + owning-package row");
        let expected = CapabilityPredicate::RequiresPackage(PackageId::new("f5-irules-cmds"));
        assert!(rows.iter().all(|row| row.predicate == expected));
    }

    #[test]
    fn the_owning_package_row_is_not_duplicated() {
        // `TK_AND_TCL` already yields a Package("Tk") row of its own;
        // `required_package: Some("Tk")` must merge with it, not double it.
        let spec = CommandSpec {
            name: "surface-test",
            surface: Some(SpecSurface::TK_AND_TCL),
            required_package: Some("Tk"),
            ..CommandSpec::DEFAULT
        };
        let rows = declarations_for_spec(&spec);
        let tk_rows = rows
            .iter()
            .filter(|row| row.provider == Provider::Package(PackageId::new("Tk")))
            .count();
        assert_eq!(tk_rows, 1);
        assert_eq!(rows.len(), 2, "Core(Tcl) + Package(Tk)");
    }

    #[test]
    fn lifecycle_populates_history_on_every_row() {
        let spec = CommandSpec {
            name: "surface-test",
            surface: Some(SpecSurface::IRULES),
            lifecycle: Lifecycle::introduced_in("16.1.0"),
            ..CommandSpec::DEFAULT
        };
        let rows = declarations_for_spec(&spec);
        for row in &rows {
            assert_eq!(row.history.introduced, Some(v("16.1.0")));
            assert!(row.history.available_at(Some(&v("17.0.0"))));
            assert!(!row.history.available_at(Some(&v("15.1.0"))));
        }
    }

    #[test]
    fn closed_world_classification_mirrors_the_old_ambient_set() {
        // Ambient keyed catalogue packs and bridge surfaces are closed.
        for package in [
            "f5-irules-cmds",
            "f5-iapps-cmds",
            "f5-tmsh-cmds",
            "f5-bigip-schema",
            "Expect",
            "sdc",
            "upf",
            "vivado",
            "iapps",
            "tmsh",
            "spectcl",
            "bpf",
        ] {
            assert!(is_closed_world_package(package), "{package}");
        }
        // Hosted libraries are not — a require on one never hides a spec.
        for package in ["Tk", "Itcl", "csv", "http", "tcltest", "struct::graph"] {
            assert!(!is_closed_world_package(package), "{package}");
        }
    }

    #[test]
    fn every_compiled_lifecycle_release_parses() {
        // `item_history` may drop only unparseable strings; prove there
        // are none in the compiled catalogue, so the conversion is
        // lossless in practice.
        let universe = crate::model::assembly::universe();
        for name in universe.command_names() {
            for spec in universe.specs(name) {
                for release in [
                    spec.lifecycle.introduced,
                    spec.lifecycle.deprecated,
                    spec.lifecycle.retired,
                ]
                .into_iter()
                .flatten()
                {
                    assert!(
                        Version::parse(release).is_ok(),
                        "{name}: lifecycle release `{release}` must parse"
                    );
                }
            }
        }
    }

    #[test]
    fn the_bridge_and_bit_vocabularies_agree() {
        // Every bridged surface package is also an authoring-vocabulary
        // name, and the one such name outside the bridge is the hosted Tk.
        for &(_, package) in VENDOR_SURFACE_BRIDGE {
            assert!(VENDOR_SURFACE_PACKAGES.contains(&package), "{package}");
        }
        let bridged: Vec<&str> = VENDOR_SURFACE_BRIDGE
            .iter()
            .map(|&(_, package)| package)
            .collect();
        let outside: Vec<&&str> = VENDOR_SURFACE_PACKAGES
            .iter()
            .filter(|package| !bridged.contains(*package))
            .collect();
        assert_eq!(outside, [&"Tk"]);
        // Bridge keys are real compiled environments.
        let registry = tcl_dialect::model::EnvironmentRegistry::compiled();
        for &(environment, _) in VENDOR_SURFACE_BRIDGE {
            assert!(registry.resolve(environment).is_some(), "{environment}");
        }
    }
}
