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

//! The resolved context and the **assistance** availability view
//! (centralisation contract §1.2, redesign §5.2).
//!
//! A [`ResolvedContext`] is an environment plus its [`FloorMap`]: per-axis
//! target sets and a per-axis `primary` version, built from the
//! environment's core selector and expected placements (with
//! [`KeyedVersions`] resolving the externally-keyed axes, mirroring
//! today's `LibraryVersionOverrides`). [`ContextQueries`] answers the
//! assistance questions — `is_available` and the §5.4 range-mode
//! `available_at_targets` — over §4.1 [`SurfaceDeclaration`] sets.
//!
//! **The I3 type split**: this is deliberately a *different type and
//! different names* from the semantic view. Completion, hover,
//! annotations, and W120 take a `(environment, floors)` context; a
//! semantic pass (taint, lowering, codegen) takes realm
//! [`crate::model::binding::BindingKnowledge`] at a program point and
//! cannot call these APIs by accident.
//!
//! The old profile-level operator-head exclusion
//! (`operators_as_commands`) has **no analog here**: every compiled
//! `OPERATOR_COMMAND` spec is 8.5+-gated, so the profiles that disable
//! operator heads (f5-irules, cadence, f5-bigip — all with 8.4-or-no-Tcl
//! cores) already exclude them through their gates alone. The equivalence
//! sweeps in [`crate::model::assembly`] prove the rule is a no-op at
//! command level; if a future spec ever needs it, it arrives as a
//! [`CapabilityPredicate`] variant, not a policy side channel.

use std::sync::Arc;

use tcl_dialect::LibraryVersionOverrides;
use tcl_dialect::model::{
    CapabilityAnswer, CoreProfileId, EnvironmentDefinition, KeyedAxis, PackagePlacement, Placement,
    Version, VersionAxisId, VersionSet, VersionSetError, WorldPolicy,
};

use crate::model::surface::{
    BuildCapability, CapabilityPredicate, Provider, SurfaceDeclaration, VENDOR_BIT_PACKAGES,
    is_closed_world_package, vendor_surface_package,
};

/// The resolved externally-keyed axis versions — the new-model mirror of
/// today's `LibraryVersionOverrides`. An unset key falls back to the D5
/// default (**oldest supported**: BIG-IP `16.1.0`; the tool/SDC/UPF axes
/// stay permissive until their first data backfill).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyedVersions {
    /// Pinned BIG-IP TMOS release.
    pub bigip: Option<Version>,
    /// Pinned EDA tool release.
    pub tool: Option<Version>,
    /// Pinned SDC standard revision.
    pub sdc: Option<Version>,
    /// Pinned UPF (IEEE 1801) standard revision.
    pub upf: Option<Version>,
}

impl KeyedVersions {
    /// The resolved version for `axis`: the explicit pin, or the D5
    /// oldest-supported default.
    #[must_use]
    pub fn resolve(&self, axis: KeyedAxis) -> Option<Version> {
        let pinned = match axis {
            KeyedAxis::BigipVersion => &self.bigip,
            KeyedAxis::ToolVersion => &self.tool,
            KeyedAxis::SdcVersion => &self.sdc,
            KeyedAxis::UpfVersion => &self.upf,
        };
        pinned.clone().or_else(|| match axis {
            KeyedAxis::BigipVersion => {
                Some(Version::parse("16.1.0").expect("the compiled BIG-IP default version parses"))
            }
            KeyedAxis::ToolVersion | KeyedAxis::SdcVersion | KeyedAxis::UpfVersion => None,
        })
    }

    /// The keyed pins of today's [`LibraryVersionOverrides`], validated.
    ///
    /// # Errors
    /// [`VersionSetError::InvalidVersion`] when an override string is not a
    /// well-formed version.
    pub fn from_overrides(overrides: &LibraryVersionOverrides) -> Result<Self, VersionSetError> {
        let parse = |text: Option<&str>| text.map(Version::parse).transpose();
        Ok(Self {
            bigip: parse(overrides.bigip_version.as_deref())?,
            tool: parse(overrides.tool_version.as_deref())?,
            sdc: parse(overrides.sdc_version.as_deref())?,
            upf: parse(overrides.upf_version.as_deref())?,
        })
    }

    /// A stable content hash for registry-generation cache keys. Hashes the
    /// pins' spellings, so comparator-equal spellings (`1.2` / `1.2.0`)
    /// conservatively miss rather than alias.
    #[must_use]
    pub fn content_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::new();
        for pin in [&self.bigip, &self.tool, &self.sdc, &self.upf] {
            pin.as_ref().map(Version::as_str).hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// One axis of a [`FloorMap`]: the declared target set and, when the
/// context can name one, the single `primary` version assistance answers
/// under (§5.4 — required for multi-target work, defaulting here to the
/// environment's own selection, never silently to "newest").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisFloor {
    /// The axis the floor lives on.
    pub axis: VersionAxisId,
    /// The declared target set on that axis.
    pub targets: VersionSet,
    /// The primary point target, when the context resolves one (a pinned
    /// or keyed placement, the core selector's release). `None` — e.g. a
    /// pure requirement floor — answers permissively, mirroring the
    /// unknown-target rule of [`tcl_dialect::model::ItemHistory`].
    pub primary: Option<Version>,
}

/// Per-axis targets and primaries for one resolved context.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FloorMap {
    entries: Vec<AxisFloor>,
}

impl FloorMap {
    /// The floor on `axis`, if the context established one.
    #[must_use]
    pub fn floor(&self, axis: &VersionAxisId) -> Option<&AxisFloor> {
        self.entries.iter().find(|entry| entry.axis == *axis)
    }

    /// The target set on `axis`.
    #[must_use]
    pub fn targets(&self, axis: &VersionAxisId) -> Option<&VersionSet> {
        self.floor(axis).map(|entry| &entry.targets)
    }

    /// The primary point target on `axis`.
    #[must_use]
    pub fn primary(&self, axis: &VersionAxisId) -> Option<&Version> {
        self.floor(axis).and_then(|entry| entry.primary.as_ref())
    }

    /// Insert `floor`, replacing any existing entry on its axis.
    pub fn set(&mut self, floor: AxisFloor) {
        self.entries.retain(|entry| entry.axis != floor.axis);
        self.entries.push(floor);
    }

    /// Every axis floor, in insertion order.
    #[must_use]
    pub fn entries(&self) -> &[AxisFloor] {
        &self.entries
    }
}

/// A resolved context: the environment a document works against plus the
/// per-axis floors derived from it (§5.2 step 1; steps 2–3 — workspace and
/// document facts — join in P2 through [`ResolvedContext::require_package`]).
#[derive(Debug, Clone)]
pub struct ResolvedContext {
    /// The environment definition.
    pub environment: Arc<EnvironmentDefinition>,
    /// Per-axis targets and primaries.
    pub floors: FloorMap,
    /// Packages explicitly required by the document/workspace — the
    /// "explicitly-floored" half of the `AmbientPlusRequire` world policy.
    /// Empty for a bare environment context; the §5.2 `package require`
    /// scan feeds it in P2.
    required_packages: Vec<Arc<str>>,
}

impl ResolvedContext {
    /// Resolve `environment` into a context: the core selector's release
    /// becomes the core axis's single-point primary (with the
    /// environment's declared target set), and each expected placement
    /// becomes a package-axis floor — pinned and tracks-base placements as
    /// points, keyed placements through `keyed`, requirement placements as
    /// target sets with no point primary.
    #[must_use]
    pub fn resolve(environment: Arc<EnvironmentDefinition>, keyed: &KeyedVersions) -> Self {
        let mut floors = FloorMap::default();
        // The environment's own declared target set, on whatever axis it
        // names (the core ladder for the Tcl environments; a package axis
        // for the identity-only `f5-bigip`).
        floors.set(AxisFloor {
            axis: environment.targets.axis().clone(),
            targets: environment.targets.clone(),
            primary: None,
        });
        let core_release = environment
            .core
            .and_then(|core| Version::parse(core.default_release.as_str()).ok());
        if let Some(core) = environment.core {
            let axis = VersionAxisId::core(core.family);
            let targets = if environment.targets.axis() == &axis {
                environment.targets.clone()
            } else {
                // A definition whose declared targets live elsewhere still
                // has a core: give it the full axis rather than nothing.
                full_axis(&axis)
            };
            floors.set(AxisFloor {
                axis,
                // The primary stays permissive for a ladder whose
                // spellings are not versions (the iRules `tmos` line).
                primary: core_release.clone(),
                targets,
            });
        }
        for placement in &environment.expected_packages {
            let axis = VersionAxisId::package(&placement.package);
            let (targets, primary) = match &placement.version {
                Placement::Pinned(version) => (point_set(&axis, version), Some(version.clone())),
                Placement::TracksBase => match &core_release {
                    Some(version) => (point_set(&axis, version), Some(version.clone())),
                    None => (full_axis(&axis), None),
                },
                Placement::Keyed(key) => match keyed.resolve(*key) {
                    Some(version) => (point_set(&axis, &version), Some(version)),
                    None => (full_axis(&axis), None),
                },
                Placement::Requirement(set) => (set.clone(), None),
            };
            floors.set(AxisFloor {
                axis,
                targets,
                primary,
            });
        }
        Self {
            environment,
            floors,
            required_packages: Vec::new(),
        }
    }

    /// Record an explicit `package require` fact: the package joins the
    /// context's world (the `AmbientPlusRequire` policy's second half) and
    /// an optional requirement set becomes its axis floor.
    pub fn require_package(&mut self, package: &str, requirement: Option<VersionSet>) {
        if !self
            .required_packages
            .iter()
            .any(|required| required.as_ref() == package)
        {
            self.required_packages.push(Arc::from(package));
        }
        if let Some(targets) = requirement {
            self.floors.set(AxisFloor {
                axis: VersionAxisId::package(package),
                targets,
                primary: None,
            });
        }
    }

    /// The environment's expected placement for `package`, if any.
    #[must_use]
    pub fn placement(&self, package: &str) -> Option<&PackagePlacement> {
        self.environment
            .expected_packages
            .iter()
            .find(|placement| placement.package.as_ref() == package)
    }

    /// Whether `package` was explicitly required into this context.
    #[must_use]
    pub fn is_required(&self, package: &str) -> bool {
        self.required_packages
            .iter()
            .any(|required| required.as_ref() == package)
    }

    /// Whether the package provider `package` is active in this context
    /// under the environment's world policy (§5.3):
    ///
    /// - the environment's own vendor surface (the interim
    ///   [`vendor_surface_package`] bridge) and its **ambient** placements
    ///   are always active — they *are* the modelled runtime, under every
    ///   policy (a closed world is exactly its ambient closure);
    /// - a **hosted** placement grants no availability by itself — it only
    ///   supplies the axis floor, exactly as the old hosted `LibraryPin`s
    ///   (Tk, Itcl) never granted visibility — so hosted packages, placed
    ///   or not, activate through an explicit require under the open
    ///   policies and never under `Closed`;
    /// - an unrequired **closed-world** package (another environment's
    ///   surface) never resolves — the old `vendor_ambient_packages` rule.
    ///
    /// (`Open`'s "hosted packs resolve everywhere" is a statement about
    /// the workspace/document tiers that join in P2; a bare environment
    /// context has no hosted world beyond its explicit requires, which is
    /// exactly the old registry-level behaviour the equivalence sweeps
    /// pin.)
    #[must_use]
    pub fn package_active(&self, package: &str) -> bool {
        if vendor_surface_package(self.environment.id.as_str()) == Some(package) {
            return true;
        }
        if self
            .placement(package)
            .is_some_and(|placement| placement.ambient)
        {
            return true;
        }
        if is_closed_world_package(package) {
            return false;
        }
        match self.environment.policy_defaults.closed_world {
            WorldPolicy::Closed => false,
            WorldPolicy::AmbientPlusRequire | WorldPolicy::Open => self.is_required(package),
        }
    }

    /// Whether `provider` is active here: a core provider iff it is the
    /// environment's core family, a package provider per
    /// [`Self::package_active`].
    #[must_use]
    pub fn provider_active(&self, provider: &Provider) -> bool {
        match provider {
            Provider::Core(family) => self
                .environment
                .core
                .is_some_and(|core| core.family == *family),
            Provider::Package(package) => self.package_active(package.as_str()),
        }
    }

    /// Whether the axis primary admits `declaration`'s applicability: the
    /// primary point is in the set, or — when the axis has no point
    /// primary — the set is non-empty (the permissive unknown-target rule).
    #[must_use]
    pub fn primary_admits(&self, declaration: &SurfaceDeclaration) -> bool {
        match self.floors.primary(declaration.applicable.axis()) {
            Some(primary) => declaration.applicable.contains(primary),
            None => !declaration.applicable.is_empty(),
        }
    }

    /// Whether `predicate` holds in this context. The capability half
    /// resolves through the environment's core build (an environment with
    /// no core, or an unknown build, answers `Unknown` and fails — B1).
    #[must_use]
    pub fn predicate_passes(&self, predicate: &CapabilityPredicate) -> bool {
        match predicate {
            CapabilityPredicate::None => true,
            CapabilityPredicate::RequiresCapability(capability) => {
                let Some(core) = self.environment.core else {
                    return false;
                };
                let resolved = CoreProfileId::new(core.default_release, core.build).resolve();
                let answer = match capability {
                    BuildCapability::Utf8CharacterModel => {
                        resolved.capabilities.utf8_character_model
                    }
                    BuildCapability::MathExtension => resolved.capabilities.math_extension,
                };
                answer == CapabilityAnswer::Yes
            }
            CapabilityPredicate::RequiresPackage(package) => self.package_active(package.as_str()),
        }
    }

    /// Whether `declaration` holds here apart from any
    /// [`CapabilityPredicate::RequiresPackage`] conjunct — the analog of
    /// the old `spec_visible` (mask + profile exclusions, no package
    /// gate), used as the selection stage of most-specific-wins so the
    /// package conjunct filters the *winner*, exactly as the old
    /// `get_for_dialect → is_available` layering did.
    #[must_use]
    pub fn admits_for_selection(&self, declaration: &SurfaceDeclaration) -> bool {
        self.provider_active(&declaration.provider)
            && self.primary_admits(declaration)
            && match &declaration.predicate {
                CapabilityPredicate::RequiresPackage(_) => true,
                predicate => self.predicate_passes(predicate),
            }
    }
}

/// The whole of `axis`.
fn full_axis(axis: &VersionAxisId) -> VersionSet {
    VersionSet::from_requirements(axis.clone(), &["0-"])
        .expect("the full-axis requirement is well-formed")
}

/// The single point `{version}` on `axis`.
fn point_set(axis: &VersionAxisId, version: &Version) -> VersionSet {
    VersionSet::from_ranges(
        axis.clone(),
        vec![tcl_dialect::model::HalfOpenRange::Exact(version.clone())],
    )
}

/// The **assistance view** over declaration sets (§1.2's R-c/R-d
/// assistance column; invariant I3 keeps it a distinct type from the
/// semantic realm queries).
pub trait ContextQueries {
    /// Whether some declaration holds: its provider is active under the
    /// environment's world policy, the axis primary is in its
    /// applicability, and its predicate passes.
    ///
    /// [`ItemHistory`](tcl_dialect::model::ItemHistory) is deliberately
    /// **not** consulted — it is per-item metadata answered by state
    /// queries, exactly as the old `is_available` never read `lifecycle`.
    fn is_available(&self, declarations: &[SurfaceDeclaration]) -> bool;

    /// The §5.4 range mode: the subset of the context's targets on `axis`
    /// at which some declaration on that axis holds (provider active,
    /// predicate passing). `targets ⊆` the result is the "compatible
    /// across the whole declared range" check; the failing remainder names
    /// the diagnostics' targets. Declarations on other axes do not
    /// contribute — targeting is per provider (§5.4).
    fn available_at_targets(
        &self,
        declarations: &[SurfaceDeclaration],
        axis: &VersionAxisId,
    ) -> VersionSet;
}

impl ContextQueries for ResolvedContext {
    fn is_available(&self, declarations: &[SurfaceDeclaration]) -> bool {
        declarations.iter().any(|declaration| {
            self.provider_active(&declaration.provider)
                && self.primary_admits(declaration)
                && self.predicate_passes(&declaration.predicate)
        })
    }

    fn available_at_targets(
        &self,
        declarations: &[SurfaceDeclaration],
        axis: &VersionAxisId,
    ) -> VersionSet {
        let Some(targets) = self.floors.targets(axis) else {
            return VersionSet::empty(axis.clone());
        };
        let mut covered = VersionSet::empty(axis.clone());
        for declaration in declarations {
            if declaration.applicable.axis() != axis
                || !self.provider_active(&declaration.provider)
                || !self.predicate_passes(&declaration.predicate)
            {
                continue;
            }
            covered = covered
                .union(&declaration.applicable)
                .expect("axis equality was just checked");
        }
        covered
            .intersect(targets)
            .expect("targets were looked up on the same axis")
    }
}

/// The **total applicability breadth** of a declaration set — the
/// generalised most-specific-wins measure (§4.1: narrowest total
/// applicability beats widest; authoring precedence, never binding
/// resolution — B4).
///
/// Counted so that a mechanically translated spec's breadth equals the old
/// mask's popcount exactly, which is what lets
/// [`crate::model::assembly::ContextRegistry::resolve_command`] reproduce
/// `best_visible`'s answers:
///
/// - a core row counts the ladder releases its set covers (one per old
///   version bit; the iRules ladder's non-version spellings count one for
///   a non-empty set);
/// - a package row counts **one** when it is part of the vendor-bit
///   authoring vocabulary ([`VENDOR_BIT_PACKAGES`]) and zero otherwise —
///   a hosted owning-package attribution row mirrors the old
///   `required_package`, which never participated in specificity;
/// - the `dialects: None` translation therefore counts 22 (5 Tcl + 1
///   iRules + 9 Jim + 7 vendor packages), strictly wider than any
///   explicit gate's maximum of 13, reproducing the old rule that a
///   catch-all loses to every scoped spec.
#[must_use]
pub fn specificity_breadth(declarations: &[SurfaceDeclaration]) -> u32 {
    let mut breadth = 0u32;
    for declaration in declarations {
        match &declaration.provider {
            Provider::Core(family) => {
                let mut covered = 0u32;
                let mut parsed_any = false;
                for release in family.releases() {
                    if let Ok(version) = Version::parse(release.as_str()) {
                        parsed_any = true;
                        if declaration.applicable.contains(&version) {
                            covered += 1;
                        }
                    }
                }
                if !parsed_any && !declaration.applicable.is_empty() {
                    covered = 1;
                }
                breadth += covered;
            }
            Provider::Package(package) => {
                if VENDOR_BIT_PACKAGES.contains(&package.as_str()) {
                    breadth += 1;
                }
            }
        }
    }
    breadth
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::surface::declarations_for_spec;
    use crate::spec::CommandSpec;
    use tcl_dialect::DialectSet;
    use tcl_dialect::model::EnvironmentRegistry;

    fn context(environment: &str) -> ResolvedContext {
        let registry = EnvironmentRegistry::compiled();
        let definition = registry.resolve(environment).expect(environment);
        ResolvedContext::resolve(definition, &KeyedVersions::default())
    }

    fn rows(dialects: Option<DialectSet>) -> Vec<SurfaceDeclaration> {
        declarations_for_spec(&CommandSpec {
            name: "context-test",
            dialects,
            ..CommandSpec::DEFAULT
        })
        .into_vec()
    }

    #[test]
    fn core_floors_carry_the_environment_selection() {
        let ctx = context("tcl8.6");
        let axis = VersionAxisId::core(tcl_dialect::model::Family::Tcl);
        assert_eq!(
            ctx.floors.primary(&axis),
            Some(&Version::parse("8.6").expect("version"))
        );
        let targets = ctx.floors.targets(&axis).expect("core targets");
        assert!(targets.contains(&Version::parse("8.6.16").expect("version")));
        assert!(!targets.contains(&Version::parse("9.0").expect("version")));
    }

    #[test]
    fn keyed_placements_resolve_through_keyed_versions() {
        let ctx = context("f5-irules");
        let axis = VersionAxisId::package("f5-irules-cmds");
        // The D5 default: oldest supported TMOS.
        assert_eq!(
            ctx.floors.primary(&axis),
            Some(&Version::parse("16.1.0").expect("version"))
        );
        let registry = EnvironmentRegistry::compiled();
        let pinned = ResolvedContext::resolve(
            registry.resolve("f5-irules").expect("irules"),
            &KeyedVersions {
                bigip: Some(Version::parse("17.1.0").expect("version")),
                ..KeyedVersions::default()
            },
        );
        assert_eq!(
            pinned.floors.primary(&axis),
            Some(&Version::parse("17.1.0").expect("version"))
        );
    }

    #[test]
    fn keyed_versions_mirror_library_version_overrides() {
        let overrides = LibraryVersionOverrides {
            bigip_version: Some("17.1.0".to_owned()),
            ..LibraryVersionOverrides::default()
        };
        let keyed = KeyedVersions::from_overrides(&overrides).expect("valid overrides");
        assert_eq!(
            keyed.resolve(KeyedAxis::BigipVersion),
            Some(Version::parse("17.1.0").expect("version"))
        );
        assert_eq!(keyed.resolve(KeyedAxis::ToolVersion), None);
        let malformed = LibraryVersionOverrides {
            bigip_version: Some("not a version".to_owned()),
            ..LibraryVersionOverrides::default()
        };
        assert!(KeyedVersions::from_overrides(&malformed).is_err());
        // Distinct pins hash distinctly; the default is stable.
        assert_eq!(
            KeyedVersions::default().content_hash(),
            KeyedVersions::default().content_hash()
        );
        assert_ne!(
            keyed.content_hash(),
            KeyedVersions::default().content_hash()
        );
    }

    #[test]
    fn core_rows_follow_the_environment_family_and_primary() {
        let tcl85_plus = rows(Some(DialectSet::TCL85_PLUS));
        assert!(context("tcl8.6").is_available(&tcl85_plus));
        assert!(context("tcl9.1").is_available(&tcl85_plus));
        assert!(!context("tcl8.4").is_available(&tcl85_plus));
        // A core row never fires under another family or a coreless
        // environment.
        assert!(!context("f5-irules").is_available(&tcl85_plus));
        assert!(!context("f5-bigip").is_available(&tcl85_plus));
        let irules_only = rows(Some(DialectSet::IRULES));
        assert!(context("f5-irules").is_available(&irules_only));
        assert!(!context("tcl8.4").is_available(&irules_only));
    }

    #[test]
    fn world_policy_gates_package_rows() {
        let expect_only = rows(Some(DialectSet::EXPECT));
        // The vendor surface is active in its own environment only.
        assert!(context("expect").is_available(&expect_only));
        for other in ["tcl8.6", "f5-irules", "f5-iapps", "spectcl", "f5-bigip"] {
            assert!(!context(other).is_available(&expect_only), "{other}");
        }
        // The bridge covers surfaces with no compiled placement at all.
        let spectcl_only = rows(Some(DialectSet::SPECTCL));
        assert!(context("spectcl").is_available(&spectcl_only));
        assert!(!context("tcl9.0").is_available(&spectcl_only));
        let shared = rows(Some(DialectSet::IAPPS.union(DialectSet::TMSH)));
        assert!(context("f5-iapps").is_available(&shared));
        assert!(context("f5-tmsh").is_available(&shared));
        assert!(!context("tcl8.5").is_available(&shared));
    }

    #[test]
    fn the_none_translation_is_available_everywhere() {
        let universal = rows(None);
        for environment in [
            "tcl8.4",
            "tcl9.1",
            "tcl",
            "f5-irules",
            "f5-iapps",
            "f5-tmsh",
            "f5-bigip",
            "expect",
            "spectcl",
            "bpf",
            "cadence-eda-tcl",
        ] {
            assert!(
                context(environment).is_available(&universal),
                "{environment}"
            );
        }
    }

    #[test]
    fn an_unplaced_hosted_package_row_needs_an_explicit_require() {
        // The `tcltest::bytestring` shape: an 8.x-gated spec whose hosted
        // package must not resurrect it under 9.x through the package row.
        let spec = CommandSpec {
            name: "context-test",
            dialects: Some(DialectSet::TCL8X),
            required_package: Some("tcltest"),
            tcllib_package: None,
            ..CommandSpec::DEFAULT
        };
        let declarations = declarations_for_spec(&spec);
        assert!(!context("tcl9.0").is_available(&declarations));
        assert!(context("tcl8.6").is_available(&declarations));
        // An explicit require puts the hosted package in the world.
        let mut required = context("tcl9.0");
        required.require_package("tcltest", None);
        assert!(required.is_available(&declarations));
        // …but not under a closed world.
        let mut closed = context("f5-irules");
        closed.require_package("tcltest", None);
        assert!(!closed.is_available(&declarations));
    }

    #[test]
    fn a_closed_world_require_constrains_across_environments() {
        let spec = CommandSpec {
            name: "context-test",
            dialects: Some(DialectSet::IRULES),
            required_package: Some("f5-irules-cmds"),
            ..CommandSpec::DEFAULT
        };
        let declarations = declarations_for_spec(&spec);
        assert!(context("f5-irules").is_available(&declarations));
        assert!(!context("tcl8.4").is_available(&declarations));
        // Selection admits it (mask analog); the package conjunct is the
        // winner filter, mirroring `get_for_dialect → is_available`.
        assert!(
            declarations
                .iter()
                .any(|row| context("f5-irules").admits_for_selection(row))
        );
    }

    #[test]
    fn available_at_targets_names_the_covered_subset() {
        let ctx = context("tcl");
        let axis = VersionAxisId::core(tcl_dialect::model::Family::Tcl);
        let tcl86_plus = rows(Some(DialectSet::TCL86_PLUS));
        let covered = ctx.available_at_targets(&tcl86_plus, &axis);
        assert!(covered.contains(&Version::parse("8.6").expect("version")));
        assert!(covered.contains(&Version::parse("9.1").expect("version")));
        assert!(!covered.contains(&Version::parse("8.5.19").expect("version")));
        // The full-set check: targets ⊆ covered fails for a gated spec…
        let targets = ctx.floors.targets(&axis).expect("targets");
        assert!(!targets.subset(&covered).expect("same axis"));
        // …and holds for an everywhere spec.
        let everywhere = ctx.available_at_targets(&rows(Some(DialectSet::ALL_TCL)), &axis);
        assert!(targets.subset(&everywhere).expect("same axis"));
        // Another provider's rows never contribute to this axis.
        let vendor = ctx.available_at_targets(&rows(Some(DialectSet::EXPECT)), &axis);
        assert!(vendor.is_empty());
    }

    #[test]
    fn breadth_reproduces_the_old_mask_popcount() {
        let cases: &[(DialectSet, u32)] = &[
            (DialectSet::TCL84, 1),
            (DialectSet::TCL8X, 3),
            (DialectSet::TCL85_PLUS, 4),
            (DialectSet::ALL_TCL, 5),
            (DialectSet::IRULES, 1),
            (DialectSet::ALL_TCL.union(DialectSet::IRULES), 6),
            (DialectSet::TK_AND_TCL, 6),
            (DialectSet::IAPPS.union(DialectSet::TMSH), 2),
            (DialectSet::EXPECT, 1),
        ];
        for &(bits, expected) in cases {
            assert_eq!(specificity_breadth(&rows(Some(bits))), expected, "{bits:?}");
            assert_eq!(expected, bits.bits().count_ones(), "{bits:?} popcount");
        }
        // The universal translation is strictly wider than any explicit
        // gate (old rule: a catch-all loses to every scoped spec).
        assert_eq!(specificity_breadth(&rows(None)), 22);
        // A hosted attribution row adds no specificity, mirroring the old
        // popcount which never counted `required_package`.
        let hosted = declarations_for_spec(&CommandSpec {
            name: "context-test",
            dialects: Some(DialectSet::ALL_TCL),
            required_package: Some("http"),
            ..CommandSpec::DEFAULT
        });
        assert_eq!(specificity_breadth(&hosted), 5);
    }

    #[test]
    fn capability_predicates_resolve_through_the_core_build() {
        let mut declaration = rows(Some(DialectSet::ALL_TCL)).remove(0);
        declaration.predicate =
            CapabilityPredicate::RequiresCapability(BuildCapability::MathExtension);
        let ctx = context("tcl8.6");
        assert!(ctx.predicate_passes(&declaration.predicate));
        assert!(ctx.is_available(std::slice::from_ref(&declaration)));
        // No core ⇒ no build to answer ⇒ fail, never silently pass.
        assert!(!context("f5-bigip").predicate_passes(&declaration.predicate));
    }
}
