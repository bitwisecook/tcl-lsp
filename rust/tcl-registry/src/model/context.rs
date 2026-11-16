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

use std::sync::{Arc, OnceLock};

use tcl_dialect::model::Family;
use tcl_dialect::model::{
    CapabilityAnswer, CoreProfileId, EnvironmentDefinition, KeyedAxis, PackagePlacement, Placement,
    Version, VersionAxisId, VersionSet, VersionSetError, WorldPolicy,
};
use tcl_dialect::{LibraryVersionOverrides, TclVersion};

use crate::hover::OptionSpec;
use crate::model::surface::{
    BuildCapability, CapabilityPredicate, Provider, SurfaceDeclaration, VENDOR_SURFACE_PACKAGES,
    is_closed_world_package, is_placement_gated_package, package_hosting_families,
    vendor_surface_package,
};
use crate::registry::NameProviders;
#[cfg(test)]
use tcl_dialect::model::PackageFloor;
use tcl_dialect::model::{CorePoints, SurfaceQuery, surface_admits};
// Shared registry-derived payload for vendor-surface queries.
use crate::profile_queries::VendorSurface;
use crate::registry::CommandRegistry;
use crate::spec::{CommandSpec, SubCommand, SubSubCommand};
use crate::traits::Traits;
use tcl_dialect::model::SpecProvider;
use tcl_dialect::model::SpecSurface;

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

    /// The raw override *spelling* for `axis`, if the session pinned one —
    /// the borrow-preserving half of [`Self::from_overrides`], for the
    /// floor readers whose answer must keep the caller's lifetime rather
    /// than borrow from a resolved context (the LSP's `DocumentFloor`;
    /// the old `DialectProfile::library_floor`'s keyed arm).
    #[must_use]
    pub fn override_spelling(overrides: &LibraryVersionOverrides, axis: KeyedAxis) -> Option<&str> {
        match axis {
            KeyedAxis::BigipVersion => overrides.bigip_version.as_deref(),
            KeyedAxis::ToolVersion => overrides.tool_version.as_deref(),
            KeyedAxis::SdcVersion => overrides.sdc_version.as_deref(),
            KeyedAxis::UpfVersion => overrides.upf_version.as_deref(),
        }
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
/// document facts — join through [`ResolvedContext::require_package`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedContext {
    /// The environment definition.
    pub environment: Arc<EnvironmentDefinition>,
    /// Per-axis targets and primaries.
    pub floors: FloorMap,
    /// Packages explicitly required by the document/workspace — the
    /// "explicitly-floored" half of the `AmbientPlusRequire` world policy.
    /// Empty for a bare environment context; the §5.2 `package require`
    /// scan feeds it.
    required_packages: Vec<Arc<str>>,
    /// Packages a loaded `SpecTcl` pack declared ambient for this context's
    /// generation, with each declared floor verbatim — recorded by the
    /// assembly layer from the generation's command store
    /// ([`crate::model::assembly::ContextRegistry`]), mirroring the old
    /// overlay registry's `ambient_packages` rows.
    pack_ambient: Vec<(Arc<str>, &'static str)>,
    /// **Declared** target sets (§5.4 range targeting): per-axis version
    /// sets the document or project explicitly declared it supports —
    /// `tclLsp.targets` and the `# tcl-lsp: supports NAME RANGE`
    /// directive (centralisation ruling R6). Deliberately separate from
    /// [`Self::floors`]: the environment's own targets (the lenient
    /// `tcl` sink targets the whole ladder) must not switch range mode
    /// on, and the assistance `primary` stays exactly where the
    /// environment put it — only the range-compatibility queries below
    /// read this. Empty for every undeclared document, which is what
    /// pins the no-range behaviour byte-identical.
    declared_targets: Vec<VersionSet>,
    /// The point this context asks surface questions at: the core family
    /// and release the environment resolves to, plus its active packages.
    /// Cached at resolution — every availability answer reads it.
    authoring_scope: AuthoringScope,
    inherited_surfaces: tcl_dialect::model::inherited_surface::InheritedSurfaceSnapshot,
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
        Self::resolve_with_inherited_surfaces(
            environment,
            keyed,
            tcl_dialect::model::inherited_surface::InheritedSurfaceSnapshot::capture(),
        )
    }

    pub(crate) fn resolve_with_inherited_surfaces(
        environment: Arc<EnvironmentDefinition>,
        keyed: &KeyedVersions,
        inherited_surfaces: tcl_dialect::model::inherited_surface::InheritedSurfaceSnapshot,
    ) -> Self {
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
            // The default release is the axis's *point primary* only when
            // the environment genuinely targets that single release line
            // (`tcl8.6` → primary 8.6). An environment whose targets span
            // more of the ladder — the lenient `tcl` fallback and the `tk`
            // environment target the whole of it — answers under the
            // permissive no-point rule instead (§5.4's "no primary" case),
            // which is exactly the old model's `ALL_TCL`-permissive
            // fallback behaviour. The primary also stays permissive for a
            // ladder whose spellings are not versions (the iRules `tmos`
            // line).
            let single_line = core_release.as_ref().is_some_and(|release| {
                release_line(core.family, core.default_release, release)
                    .is_some_and(|line| targets.subset(&line).unwrap_or(false))
            });
            floors.set(AxisFloor {
                axis,
                primary: single_line.then(|| core_release.clone()).flatten(),
                targets,
            });
            // Ancestry lineage: a core derived from another family
            // embeds that family's surface, so each ancestor axis gets a
            // point primary at the edge's own **anchor**
            // ([`Ancestry::anchor`]) — the F5 trunk at its measured fork
            // patchlevel 8.4.6 (measurements §4/§4a), Jim at the Tcl 8.6
            // command set `jim_tcl.txt` says it implements a significant
            // subset of. Reading the anchor off the edge is what lets one
            // walk serve a fork and a reimplementation without a
            // family special case. An anchor that
            // is not a version (the iRules `tmos` line) yields no floor,
            // which is the intent. The `f5-irules` offshoot's closed
            // load-time resolution keeps its ancestor surface explicit
            // per spec instead (`provider_active`), so it takes no
            // lineage floors either.
            if !core.family.closed_load_time_resolution() {
                let mut edge = core.family.ancestry();
                while let Some(ancestry) = edge {
                    let axis = VersionAxisId::core(ancestry.parent);
                    if floors.floor(&axis).is_none()
                        && let Ok(point) = Version::parse(ancestry.anchor)
                    {
                        floors.set(AxisFloor {
                            axis: axis.clone(),
                            targets: point_set(&axis, &point),
                            primary: Some(point),
                        });
                    }
                    edge = ancestry.parent.ancestry();
                }
            }
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
        let mut context = Self {
            environment,
            floors,
            required_packages: Vec::new(),
            pack_ambient: Vec::new(),
            declared_targets: Vec::new(),
            authoring_scope: AuthoringScope::default(),
            inherited_surfaces,
        };
        context.authoring_scope = compute_authoring_scope(&context);
        context
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
        self.authoring_scope = compute_authoring_scope(self);
    }

    /// The package floors in force: each package axis whose floor names a
    /// point, as `(package, version)`, by package name.
    #[must_use]
    pub fn package_floors(&self) -> Vec<(String, String)> {
        let mut floors: Vec<(String, String)> = self
            .floors
            .entries()
            .iter()
            .filter_map(|floor| {
                Some((
                    floor.axis.package_name()?.to_owned(),
                    floor.primary.as_ref()?.as_str().to_owned(),
                ))
            })
            .collect();
        floors.sort();
        floors
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

    /// Whether the package **provider** `package` supplies declarations in
    /// this context — the carrier question, asked of a
    /// [`Provider::Package`] row:
    ///
    /// - the environment's own vendor surface (the interim
    ///   [`vendor_surface_package`] bridge) and its **ambient** placements
    ///   are always active — they *are* the modelled runtime, under every
    ///   policy (a closed world is exactly its ambient closure);
    /// - a **hosted** placement carries no declarations by itself — it only
    ///   supplies the axis floor, exactly as the old hosted `LibraryPin`s
    ///   (Tk, Itcl) never granted visibility — so a hosted package carries
    ///   its rows only through an explicit require, and never under
    ///   `Closed`;
    /// - an unrequired **closed-world** package (another environment's
    ///   surface) never carries — the old `vendor_ambient_packages` rule.
    ///
    /// Deliberately **narrower** than [`Self::package_active`]: the
    /// translated `Provider::Package` row is a full-axis fallback the
    /// mechanical translation adds for every owning package, so treating it
    /// as leniently active would resurrect a spec its Tcl-core row already
    /// excluded (`tcltest::bytestring` under 9.x — pinned by
    /// `an_unplaced_hosted_package_row_needs_an_explicit_require`).
    #[must_use]
    pub(crate) fn package_provider_active(&self, package: &str) -> bool {
        if vendor_surface_package(self.environment.id.as_str()) == Some(package) {
            return true;
        }
        if self.placement_is_ambient(package) {
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

    /// Whether `package` is **in this document's world** under the
    /// environment's strictness policy (§5.3) — the availability question,
    /// as opposed to [`Self::package_provider_active`]'s carrier question:
    ///
    /// - everything [`Self::package_provider_active`] admits (the ambient
    ///   closure, the vendor surface, an explicit require);
    /// - plus, under an **open** world only, any package no environment
    ///   owns as its closed-world runtime — §5.3's lenient `open` default,
    ///   "hosted packs visible, W120 advisory";
    /// - nothing further under `AmbientPlusRequire`, where a hosted pack
    ///   joins the world only by being required (Q7): an iApp that never
    ///   says `package require Tk` cannot call `wm`;
    /// - and nothing at all under `Closed`, where `package require` is not
    ///   part of the language.
    ///
    /// **`Tk`'s placement.** This is the canonical query for "is Tk in
    /// this document's world?": `Tk` is ambient under the `tk` environment
    /// and hosted under plain Tcl, so one function answers it for both, and
    /// the two answers differ for the right reason — the placement, not the
    /// name. Callers
    /// wanting "…without a `package require`" ask
    /// [`Self::ambient_package`]; callers wanting "could this environment
    /// host it at all?" ask [`Self::can_host_package`].
    #[must_use]
    pub fn package_active(&self, package: &str) -> bool {
        self.package_provider_active(package)
            || (self.environment.policy_defaults.closed_world == WorldPolicy::Open
                && !is_closed_world_package(package))
    }

    /// Whether this environment can **host** `package` as a `package
    /// require` — it declares a placement for it (§3.2's placement
    /// claims).
    ///
    /// Closed-world F5, EDA, BPF and `SpecTcl` environments declare no Tk
    /// placement. Plain Tcl, its lenient sink and Tk declare one.
    ///
    /// Distinct from [`Self::package_active`]: hosting is what the
    /// *environment* offers, activation is what this *document* got.
    #[must_use]
    pub fn can_host_package(&self, package: &str) -> bool {
        self.placement(package).is_some()
    }

    /// Whether a command offered by `offered` could be *meant for* this
    /// document's world: some provider of the name stands in a relation to
    /// the environment's core family.
    ///
    /// The distinction W002 needs. A name offered to a related world — a
    /// `tcl8.4` document writing a `tcl8.6` command, a `jim` document writing
    /// a Tcl command Jim's roster omits, a `tcl8.6` document writing an iRules
    /// command — is *disabled here*. A name only an unrelated world offers
    /// (`system`, Expect's, in a `jim` document) is simply unknown here.
    ///
    /// - a core provider is related when its family is on one derivation line
    ///   with the document's ([`Family::on_one_line_with`]);
    /// - a package provider is related when this environment can host the
    ///   package, or the document's family shares packages with a family that
    ///   ships it ([`Family::shares_packages_with`] — a fork does, a
    ///   reimplementation does not). A package no compiled environment ships
    ///   relates to everything, as a spec that states no surface does.
    ///
    /// A context with no core runtime of its own relates to everything.
    #[must_use]
    pub fn is_related_to_a_provider_of(&self, offered: &NameProviders) -> bool {
        let Some(core) = self.environment.core else {
            return true;
        };
        if offered.unrestricted || offered.providers.is_empty() {
            return true;
        }
        offered.providers.iter().any(|provider| match provider {
            SpecProvider::Core(family) => core.family.on_one_line_with(*family),
            SpecProvider::Package(package) => {
                let hosts = package_hosting_families(package);
                hosts.is_empty()
                    || self.can_host_package(package)
                    || hosts
                        .iter()
                        .any(|host| core.family.shares_packages_with(*host))
            }
        })
    }

    /// Whether `provider` is active here: a core provider iff it is the
    /// environment's core family **or a fork ancestor of it** (a
    /// fork-of-Tcl core embeds the fork point's Tcl core, measurements
    /// §4/§4a — so a `Core(Tcl)` declaration reaches an `f5-tcl`-core
    /// environment through the fork edge, admitted at the fork-point
    /// release by [`Self::primary_admits`]); a package provider per
    /// [`Self::package_active`].
    ///
    /// The one exception is a core family with **closed load-time
    /// resolution** (`f5-irules`, measurements §4a/§4b): its embedded
    /// ancestor surface is explicit per spec — a command exists there iff
    /// its own declaration says so — so the fork-lineage channel is
    /// deliberately not open.
    #[must_use]
    pub fn provider_active(&self, provider: &Provider) -> bool {
        match provider {
            Provider::Core(family) => self.environment.core.is_some_and(|core| {
                if core.family == *family {
                    return true;
                }
                if core.family.closed_load_time_resolution() {
                    return false;
                }
                let mut edge = core.family.ancestry();
                while let Some(ancestry) = edge {
                    if ancestry.parent == *family {
                        return true;
                    }
                    edge = ancestry.parent.ancestry();
                }
                false
            }),
            Provider::Package(package) => self.package_provider_active(package.as_str()),
            // A document provider is the analysed buffer's own declaration
            // (R1). It is active wherever it is consulted — the declaration
            // set is per-document, so possession of the row *is* the scope
            // check — and it never reaches a shared generation.
            Provider::Document => true,
        }
    }

    /// Whether `name`, offered by `provider`, survives the **enumerated**
    /// half of inherit-then-override — design **Q6**.
    ///
    /// [`Self::provider_active`] answers the ancestor channel's *carrier*
    /// question: is a `Core(Tcl)` declaration reachable from a `jim`
    /// document at all? Yes — that edge is what makes deleting the 76
    /// re-authored specs sound. But a
    /// [`Lineage::Reimplementation`](tcl_dialect::model::family::Lineage)
    /// implements a *subset* of its ancestor, so the carrier alone
    /// over-admits: measured against a built `jimsh`, the inherited Tcl
    /// 8.6 surface offered `coroutine`, `trace`, `yield`, `yieldto` and
    /// thirteen more heads Jim has never had.
    ///
    /// This is the second half of the question, and the only one that
    /// needs the command's *name*: the roster the family's surface pack
    /// enumerated (`include from tcl {…}`) says which ancestor names
    /// actually reach it, over which window of its own ladder. A fork
    /// edge, the family's own provider, and a build with no roster
    /// loaded all answer `true` — see
    /// [`inherited_surface::admits`](tcl_dialect::model::inherited_surface::admits)
    /// for why fail-open is the honest degradation.
    #[must_use]
    pub fn inherited_surface_admits(&self, name: &str, provider: &Provider) -> bool {
        let (Some(core), Provider::Core(source)) = (self.environment.core, provider) else {
            return true;
        };
        if core.family == *source {
            return true;
        }
        let at = self.floors.primary(&VersionAxisId::core(core.family));
        self.inherited_surfaces
            .admits(core.family, *source, name, at)
    }

    /// The generation of the exact inherited roster held by this context.
    /// This is source catalogue observation, not current execution authority.
    #[must_use]
    pub fn inherited_surface_generation(&self) -> u64 {
        self.inherited_surfaces.generation()
    }

    pub(crate) fn inherited_surface_snapshot(
        &self,
    ) -> &tcl_dialect::model::inherited_surface::InheritedSurfaceSnapshot {
        &self.inherited_surfaces
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

    /// Configured core catalogue membership for lexical source presentation.
    /// Missing cores and unknown builds abstain. Compatibility targets do not
    /// replace the environment's assistance point.
    /// This does not resolve a current math command or establish execution.
    #[must_use]
    pub fn source_math_function(&self, name: &str) -> CapabilityAnswer {
        let Some(core) = self.environment.core else {
            return CapabilityAnswer::Unknown;
        };
        CoreProfileId::new(core.default_release, core.build)
            .resolve()
            .mathfunc(name)
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
            CapabilityPredicate::RequiresPackage(package) => {
                self.required_package_available(Some(package.as_str()))
            }
        }
    }

    /// Whether `declaration` holds here apart from any
    /// [`CapabilityPredicate::RequiresPackage`] conjunct.
    ///
    /// This is the selection stage of most-specific-wins, so the package
    /// conjunct filters the *winner* rather than the candidates, which is
    /// the layering `get_for_surface` → `is_available` performs.
    #[must_use]
    pub fn admits_for_selection(&self, declaration: &SurfaceDeclaration) -> bool {
        self.provider_active(&declaration.provider)
            && self.primary_admits(declaration)
            && match &declaration.predicate {
                CapabilityPredicate::RequiresPackage(_) => true,
                predicate => self.predicate_passes(predicate),
            }
    }

    // --- the spec-query surface -----------------------------------------
    //
    // Availability questions the compiler and analyser ask, answered from
    // the environment's own derived facts (the authoring point, the version
    // ceiling, the placements). The parity sweeps in this module's tests
    // pin each answer to the catalogue profile's, which is what makes the
    // centralised inputs a refactor rather than a behaviour change.

    /// The point this context asks surface questions at.
    ///
    /// It has exactly one source — [`compute_authoring_scope`] — for every
    /// environment, including `tk`, whose Tk package the ambient placement
    /// derives rather than an injected override.
    #[must_use]
    pub fn authoring_query(&self) -> SurfaceQuery<'_> {
        self.authoring_scope.query()
    }

    /// The environment's authoring surface, owned — for a caller that
    /// cannot hold the context alive while it asks.
    #[must_use]
    pub fn authoring_scope(&self) -> AuthoringScope {
        self.authoring_scope.clone()
    }

    /// The environment's option-gating version ceiling — the old
    /// `version_ceiling` (§5.2 upper bound), from environment policy.
    #[must_use]
    pub fn tcl_version_ceiling(&self) -> Option<TclVersion> {
        tcl_version_of_release(self.environment.policy_defaults.version_ceiling?)
    }

    /// Whether the math-operator command heads exist as callable commands
    /// here — the old `operators_as_commands`, derived: TIP 174 heads
    /// exist exactly on a Tcl core at 8.5 or newer (sweep-pinned).
    #[must_use]
    pub fn operator_heads_are_commands(&self) -> bool {
        self.environment.core.is_some_and(|core| {
            core.family == Family::Tcl
                && core.default_release.ordinal() >= tcl_dialect::model::Release::TCL_8_5.ordinal()
        })
    }

    /// Whether a `required_package` gate is satisfied here — the old
    /// `ProfileQueries::package_available` rule, restated over the
    /// placement model.
    ///
    /// Two classes, and the split is data (§3.2's placement claims), not a
    /// name list:
    ///
    /// - a package **no** environment runs as part of its own runtime
    ///   ([`is_placement_gated_package`] is false — `Itcl`, every tcllib
    ///   module) is always satisfied: the model does not know where it is
    ///   installed, so W120 owns the nag;
    /// - a **placement-gated** package answers [`Self::package_active`] —
    ///   the environment's ambient closure, plus (outside a closed world)
    ///   the lenient hosted rule and this document's own requires.
    ///
    /// `Tk` is the concrete case for the second class, because `wish` runs
    /// it ambiently. The single enumerated consequence is that a
    /// **closed** world does not resolve Tk: a `.bpf` or `.tclspec`
    /// document cannot call `wm` (`package require` is not part of either
    /// language, so it never could), while every open world resolves it as
    /// usual.
    ///
    /// [`is_placement_gated_package`]: crate::model::surface::is_placement_gated_package
    #[must_use]
    pub fn required_package_available(&self, required: Option<&str>) -> bool {
        let Some(package) = required else {
            return true;
        };
        // An ambient-plus-require world answers the gate outright (Q7):
        // its whole point is that a hosted pack is absent until the source
        // requires it, so the lenient "unplaced libraries are assumed
        // present, W120 nags" rule does not apply there.
        if self.environment.policy_defaults.closed_world == WorldPolicy::AmbientPlusRequire {
            return self.package_active(package);
        }
        !is_placement_gated_package(package) || self.package_active(package)
    }

    /// Whether the environment ships `package` ambient (a placement or a
    /// pack's `ambient_package` row) — the old
    /// `CommandRegistry::is_ambient_package` union, context-keyed.
    #[must_use]
    pub fn ambient_package(&self, package: &str) -> bool {
        self.placement_is_ambient(package)
            || self
                .pack_ambient
                .iter()
                .any(|(name, _)| name.as_ref() == package)
    }

    /// Whether the environment's own placements ship `package` ambient —
    /// the old `DialectProfile::is_ambient_package`.
    #[must_use]
    pub fn placement_is_ambient(&self, package: &str) -> bool {
        self.placement(package)
            .is_some_and(|placement| placement.ambient)
    }

    /// The version floor the environment guarantees for `package` before
    /// any `package require` — the old `DialectProfile::library_floor`
    /// with the keyed axes already resolved into the context's floor map
    /// (pinned → the pin, tracks-base → the core release, keyed → the
    /// session pin or the oldest-supported default).
    #[must_use]
    pub fn placement_floor(&self, package: &str) -> Option<&Version> {
        self.placement(package)?;
        self.floors.primary(&VersionAxisId::package(package))
    }

    /// The externally-keyed axis `package`'s placement sits on, if any —
    /// the old `LibraryVersion::Keyed(key)` arm of
    /// `DialectProfile::library_floor`, exposed so a floor reader can apply
    /// its own session override spelling
    /// ([`KeyedVersions::override_spelling`]) without rebuilding the
    /// context for every distinct pin.
    #[must_use]
    pub fn placement_keyed_axis(&self, package: &str) -> Option<KeyedAxis> {
        match self.placement(package)?.version {
            Placement::Keyed(axis) => Some(axis),
            _ => None,
        }
    }

    /// The highest floor loaded packs declared for `package` as ambient —
    /// the old `CommandRegistry::ambient_package_floor`.
    #[must_use]
    pub fn pack_ambient_floor(&self, package: &str) -> Option<&'static str> {
        self.pack_ambient
            .iter()
            .filter(|(name, _)| name.as_ref() == package)
            .map(|&(_, version)| version)
            .max_by(|a, b| crate::version::compare(a, b))
    }

    /// Record one pack-declared ambient package row (assembly-layer use;
    /// see [`Self::pack_ambient_floor`]).
    pub(crate) fn record_pack_ambient(&mut self, package: &str, version: &'static str) {
        self.pack_ambient.push((Arc::from(package), version));
        self.authoring_scope = compute_authoring_scope(self);
    }

    /// Whether `spec` is available in this context: its surface admits
    /// the point, its head is not an operator this environment keeps inside
    /// `expr`, and its required package is in this document's world.
    #[must_use]
    pub fn spec_available(&self, spec: &CommandSpec) -> bool {
        self.spec_available_under(spec, self.authoring_query())
    }

    /// [`Self::spec_available`] for a caller that already holds the
    /// authoring query.
    fn spec_available_under(&self, spec: &CommandSpec, query: SurfaceQuery<'_>) -> bool {
        crate::irules_policy::runtime_surface_admits(spec, query)
            && self.spec_available_after_surface(spec)
    }

    /// The remaining context filters after this exact spec and query have
    /// passed `CommandRegistry::get_for_surface`'s surface admission.
    fn spec_available_after_surface(&self, spec: &CommandSpec) -> bool {
        (self.operator_heads_are_commands() || !spec.traits.contains(Traits::OPERATOR_COMMAND))
            && self.required_package_available(spec.required_package)
    }

    /// Resolve `name` to its command spec in this context over `registry`
    /// — the point query plus the full availability filter, for call sites
    /// holding a command store that is not this context's own generation.
    #[must_use]
    pub fn resolve_spec(
        &self,
        registry: &CommandRegistry,
        name: &str,
    ) -> Option<&'static CommandSpec> {
        self.resolve_spec_in_realm(
            registry,
            name,
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
    }

    /// Exact catalogue lookup first, then case-insensitive advice for one
    /// admitted descriptor. Ambiguity refuses; this grants no runtime binding.
    #[must_use]
    pub fn resolve_spec_case_advice(
        &self,
        registry: &CommandRegistry,
        name: &str,
    ) -> Option<&'static CommandSpec> {
        if let Some(spec) = self.resolve_spec(registry, name) {
            return Some(spec);
        }
        let folded = name.to_lowercase();
        let mut matches = registry
            .command_names()
            .filter(|candidate| candidate.to_lowercase() == folded)
            .filter_map(|candidate| self.resolve_spec(registry, candidate));
        let selected = matches.next()?;
        matches
            .all(|candidate| std::ptr::eq(candidate, selected))
            .then_some(selected)
    }

    /// Resolve command availability in an explicitly selected execution phase.
    /// This never establishes native command/compiler implementation identity.
    #[must_use]
    pub fn resolve_spec_in_realm(
        &self,
        registry: &CommandRegistry,
        name: &str,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Option<&'static CommandSpec> {
        let query = self.authoring_query().with_realm(realm);
        let selected = registry.get_for_surface(name, Some(query));
        if selected.is_none() && crate::trace_deferred_writes_any() {
            eprintln!(
                "TCL_LSP_TRACE_METADATA_AVAILABILITY stage=get_for_surface head={name:?} context={self:p} query={query:?}"
            );
        }
        selected.filter(|spec| {
            if !self.spec_available_after_surface(spec) {
                if crate::trace_deferred_writes_any() {
                    eprintln!(
                        "TCL_LSP_TRACE_METADATA_AVAILABILITY stage=spec_available head={name:?} canonical={:?} context={self:p} query={query:?}",
                        spec.name,
                    );
                }
                return false;
            }
            let admitted = self.roster_admits(spec, &query);
            if !admitted && crate::trace_deferred_writes_any() {
                eprintln!(
                    "TCL_LSP_TRACE_METADATA_AVAILABILITY stage=roster head={name:?} canonical={:?} context={self:p} query={query:?}",
                    spec.name,
                );
            }
            admitted
        })
    }

    /// The enumerated half of inherit-then-override for one spec: a spec
    /// that reaches this document only through an ancestor's core row
    /// survives when that ancestor's roster lists the name.
    ///
    /// The assembled generation applies the roster when it admits a spec
    /// (`ContextRegistry::assemble`); a query over a store that is not that
    /// generation, which is how the analyser resolves a written head, applies
    /// it here, so both answer alike: a `jim` document has no `coroutine`.
    /// A spec with a row of the document's own family, or a package row, or
    /// no row at all, is not the roster's to filter.
    // Membership belongs to the selected registry descriptor. A rooted
    // fallback query retains its spelling independently of that roster key.
    fn roster_admits(&self, spec: &CommandSpec, query: &SurfaceQuery<'_>) -> bool {
        if query.realm == tcl_dialect::model::InvocationRealm::InterpreterRuntime
            && query
                .core
                .nearest()
                .is_some_and(|(family, _)| family == Family::F5Irules)
            && crate::irules_policy::irules_disabled_class(spec.name)
                == Some(crate::irules_policy::IrulesDisabledClass::CompilerRefused)
            && spec.owning_package().is_none()
        {
            return crate::irules_policy::runtime_surface_admits(spec, *query);
        }
        let Some(rows) = spec.surface else {
            return true;
        };
        let mut any_row_reaches_here = false;
        for row in rows {
            if !surface_admits(std::slice::from_ref(row), Some(query)) {
                continue;
            }
            any_row_reaches_here = true;
            let passes = match row.provider {
                SpecProvider::Core(source) => {
                    self.inherited_surface_admits(spec.name, &Provider::Core(source))
                }
                SpecProvider::Package(_) => true,
            };
            if passes {
                return true;
            }
        }
        !any_row_reaches_here
    }

    /// Whether `sub` (of `spec`) is available here — the old
    /// `is_subcommand_available` gate-inheritance rule.
    #[must_use]
    pub fn subcommand_available(&self, spec: &CommandSpec, sub: &SubCommand) -> bool {
        sub.surface
            .or(spec.surface)
            .is_none_or(|gate| surface_admits(gate, Some(&self.authoring_query())))
    }

    /// The subcommands of `spec` available here, in declaration order.
    #[must_use]
    pub fn available_subcommands<'r>(&self, spec: &'r CommandSpec) -> Vec<&'r SubCommand> {
        spec.subcommands
            .iter()
            .filter(|sub| self.subcommand_available(spec, sub))
            .collect()
    }

    /// Whether `sub_sub` is available here at `package_version` — the old
    /// `is_sub_subcommand_available` (two-level gate inheritance plus the
    /// owning package's lifecycle window).
    #[must_use]
    pub fn sub_subcommand_available(
        &self,
        spec: &CommandSpec,
        sub: &SubCommand,
        sub_sub: &SubSubCommand,
        package_version: Option<&str>,
    ) -> bool {
        sub_sub
            .surface
            .or(sub.surface)
            .or(spec.surface)
            .is_none_or(|gate| surface_admits(gate, Some(&self.authoring_query())))
            && sub_sub.available_for_version(package_version)
    }

    /// The second-level operations of `sub` available here at
    /// `package_version`, in declaration order.
    #[must_use]
    pub fn available_sub_subcommands(
        &self,
        spec: &CommandSpec,
        sub: &SubCommand,
        package_version: Option<&str>,
    ) -> Vec<&'static SubSubCommand> {
        sub.sub_subcommands
            .iter()
            .filter(|sub_sub| self.sub_subcommand_available(spec, sub, sub_sub, package_version))
            .collect()
    }

    /// Whether `opt` is available here given its inherited `parent_gate`
    /// (§5.2): the gate admits the point, and its version floor is at or
    /// below the environment's ceiling.
    #[must_use]
    pub fn option_available(
        &self,
        opt: &OptionSpec,
        parent_gate: Option<&'static [SpecSurface]>,
    ) -> bool {
        let Some(gate) = opt.surface.or(parent_gate) else {
            // No restriction on the option or its parent.
            return true;
        };
        if !surface_admits(gate, Some(&self.authoring_query())) {
            return false;
        }
        match (core_tcl_floor(gate), self.tcl_version_ceiling()) {
            (Some(min), Some(ceiling)) => min <= ceiling,
            // A pure vendor gate has no version floor; an environment
            // without a ceiling accepts every version.
            _ => true,
        }
    }

    /// Declared option / switch names of `spec` available here, in
    /// declaration order with duplicates removed.
    #[must_use]
    pub fn available_option_names(&self, spec: &CommandSpec) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        let mut consider = |opt: &OptionSpec| {
            if self.option_available(opt, spec.surface) && !names.contains(&opt.name) {
                names.push(opt.name);
            }
        };
        for opt in spec.options {
            consider(opt);
        }
        for form in spec.command_forms {
            for opt in form.options {
                consider(opt);
            }
        }
        names
    }

    /// The [`OptionSpec`]s of `spec` available here, in declaration order.
    #[must_use]
    pub fn available_option_specs(&self, spec: &CommandSpec) -> Vec<&'static OptionSpec> {
        let mut out: Vec<&'static OptionSpec> = Vec::new();
        let mut consider = |opt: &'static OptionSpec| {
            if self.option_available(opt, spec.surface) && !out.iter().any(|o| o.name == opt.name) {
                out.push(opt);
            }
        };
        for opt in spec.options {
            consider(opt);
        }
        for form in spec.command_forms {
            for opt in form.options {
                consider(opt);
            }
        }
        out
    }

    /// Option / switch names of subcommand `sub` (inheriting
    /// `sub.surface.or(spec.surface)` as the parent gate).
    #[must_use]
    pub fn available_sub_option_names(
        &self,
        spec: &CommandSpec,
        sub: &SubCommand,
    ) -> Vec<&'static str> {
        let parent = sub.surface.or(spec.surface);
        let mut names: Vec<&'static str> = Vec::new();
        for opt in sub.options {
            if self.option_available(opt, parent) && !names.contains(&opt.name) {
                names.push(opt.name);
            }
        }
        names
    }

    /// The [`OptionSpec`]s of subcommand `sub` available here (same
    /// inheritance as [`Self::available_sub_option_names`]).
    #[must_use]
    pub fn available_sub_option_specs(
        &self,
        spec: &CommandSpec,
        sub: &SubCommand,
    ) -> Vec<&'static OptionSpec> {
        let parent = sub.surface.or(spec.surface);
        sub.options
            .iter()
            .filter(|opt| self.option_available(opt, parent))
            .collect()
    }

    /// Look up an option of `spec` by canonical name or alias, honouring
    /// this context's gate and the resolved `package_version` — the old
    /// `find_option`.
    #[must_use]
    pub fn find_option<'r>(
        &self,
        spec: &'r CommandSpec,
        option_name: &str,
        package_version: Option<&str>,
    ) -> Option<&'r OptionSpec> {
        let matches = |opt: &&'r OptionSpec| {
            opt.matches(option_name)
                && self.option_available(opt, spec.surface)
                && opt.available_for_version(package_version)
        };
        spec.options.iter().find(matches).or_else(|| {
            spec.command_forms
                .iter()
                .flat_map(|f| f.options.iter())
                .find(matches)
        })
    }

    /// The provider this environment's own commands are authored under,
    /// when it has a vendor vocabulary of its own (F5, expect, spectcl,
    /// bpf). The iRules core authors under its family; every other vendor
    /// surface authors under its package.
    #[must_use]
    pub fn vendor_authoring_provider(&self) -> Option<SpecProvider> {
        if self
            .environment
            .core
            .is_some_and(|core| core.family == Family::F5Irules)
        {
            return Some(SpecProvider::Core(Family::F5Irules));
        }
        vendor_surface_package(self.environment.id.as_str()).map(SpecProvider::Package)
    }

    /// This environment's **own** command surface over `registry`: the
    /// commands available here authored under its vendor provider, grouped
    /// by `NS::` namespace prefix (bare names group under `""`), sorted by
    /// descending size then name.
    ///
    /// `None` for an environment with no vendor surface of its own, and for
    /// one whose surface resolves to nothing in `registry`. Feeds the
    /// generated consumers (the AI prompt's F5-surface summary) so prose can
    /// never drift from the data.
    #[must_use]
    pub fn vendor_command_surface(&self, registry: &CommandRegistry) -> Option<VendorSurface> {
        let vendor = self.vendor_authoring_provider()?;
        let mut by_ns: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        let mut command_count = 0usize;
        for name in registry.command_names() {
            let Some(spec) = self.resolve_spec(registry, name) else {
                continue;
            };
            if !is_vendor_own(spec, vendor) {
                continue;
            }
            command_count += 1;
            let (holder, _) = tcl_syntax::naming::key_holder_and_tail(name);
            let namespaces = tcl_syntax::naming::key_segments(holder);
            let namespace = namespaces.first().map_or("", String::as_str);
            *by_ns.entry(namespace.to_owned()).or_insert(0) += 1;
        }
        if command_count == 0 {
            return None;
        }
        let mut namespaces: Vec<(String, usize)> = by_ns.into_iter().collect();
        namespaces.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Some(VendorSurface {
            command_count,
            namespaces,
        })
    }

    /// The ambient **keyed** placement `spec` sits on here, if any: the
    /// owning package's placement when that is keyed and ambient, or — for
    /// a spec the environment's own vendor provider carries — the
    /// environment's single ambient keyed placement.
    #[must_use]
    pub fn keyed_ambient_placement(&self, spec: &CommandSpec) -> Option<&PackagePlacement> {
        let keyed_ambient = |placement: &&PackagePlacement| {
            placement.ambient && matches!(placement.version, Placement::Keyed(_))
        };
        if let Some(package) = spec.owning_package()
            && let Some(placement) = self.placement(package)
        {
            return keyed_ambient(&placement).then_some(placement);
        }
        let vendor = self.vendor_authoring_provider()?;
        if !is_vendor_own(spec, vendor) {
            return None;
        }
        self.environment
            .expected_packages
            .iter()
            .find(keyed_ambient)
    }

    /// The declared version range of `spec` on this context's keyed
    /// library axis — the old `keyed_version_range`: the explicit
    /// introduction release or the axis baseline, plus the removal
    /// release.
    #[must_use]
    pub fn keyed_version_range(
        &self,
        spec: &CommandSpec,
    ) -> Option<(Option<&'static str>, Option<&'static str>)> {
        let placement = self.keyed_ambient_placement(spec)?;
        let Placement::Keyed(axis) = placement.version else {
            return None;
        };
        let lifecycle = spec.lifecycle.with_baseline(keyed_axis_baseline(axis));
        Some((lifecycle.introduced, lifecycle.retired))
    }

    // §5.4 range targeting: declared target sets.
    //
    // The additive range-mode queries. A document/project that
    // *declares* a multi-version target set — `tclLsp.targets`, the
    // `# tcl-lsp: supports NAME RANGE` directive (ruling R6) — records
    // it here, and the compatibility checks ask whether an item holds at
    // **every** declared target, naming the failing remainder. Nothing
    // else reads the declared sets: `primary` (and with it every
    // assistance answer) stays the environment's own, per §5.4's
    // assistance/semantic split, and a document that declares nothing
    // takes none of these paths.

    /// Declare a target set on its axis, replacing any earlier
    /// declaration on the same axis (most specific wins per provider —
    /// §5.4's source ordering is resolved by the caller).
    pub fn declare_targets(&mut self, targets: VersionSet) {
        self.declared_targets
            .retain(|existing| existing.axis() != targets.axis());
        self.declared_targets.push(targets);
    }

    /// The declared target set on `axis`, if the document declared one.
    #[must_use]
    pub fn declared_targets(&self, axis: &VersionAxisId) -> Option<&VersionSet> {
        self.declared_targets
            .iter()
            .find(|targets| targets.axis() == axis)
    }

    /// Every declared target set, in declaration order.
    #[must_use]
    pub fn declared_target_sets(&self) -> &[VersionSet] {
        &self.declared_targets
    }

    /// The subset of the declared targets on `axis` **outside** the item
    /// window `[introduced, retired)` — the §5.4 lifecycle range check.
    /// `None` when the axis has no declared targets, the window covers
    /// them all, or a bound does not parse (permissive, like every other
    /// unparseable-version path). Bounds take the `a0` pad of the
    /// requirement algebra, so "below the introduction" and "at or past
    /// the retirement" agree with [`Lifecycle`]'s own comparisons at
    /// ladder-release granularity.
    ///
    /// [`Lifecycle`]: crate::lifecycle::Lifecycle
    #[must_use]
    pub fn targets_outside_window(
        &self,
        axis: &VersionAxisId,
        introduced: Option<&str>,
        retired: Option<&str>,
    ) -> Option<VersionSet> {
        let targets = self.declared_targets(axis)?;
        let mut excluded: Vec<String> = Vec::new();
        if let Some(introduced) = introduced {
            excluded.push(format!("0-{introduced}"));
        }
        if let Some(retired) = retired {
            excluded.push(format!("{retired}-"));
        }
        if excluded.is_empty() {
            return None;
        }
        let complement = VersionSet::from_requirements(axis.clone(), &excluded).ok()?;
        let outside = targets.intersect(&complement).ok()?;
        (!outside.is_empty()).then_some(outside)
    }

    /// The subset of the declared **core-Tcl** targets a surface gate does
    /// not cover — the §5.4 range check for items whose introduction is
    /// spelled as ladder rows rather than a lifecycle. `None` when no core
    /// targets are declared, the gate is absent (unrestricted), the gate
    /// names no plain-Tcl line at all (the item is another provider's — its
    /// own axis governs it), or every declared target is covered.
    #[must_use]
    pub fn targets_uncovered_by_gate(
        &self,
        gate: Option<&'static [SpecSurface]>,
    ) -> Option<VersionSet> {
        let axis = VersionAxisId::core(Family::Tcl);
        let targets = self.declared_targets(&axis)?;
        let covered = crate::model::surface::core_tcl_set(gate?)?;
        let uncovered = targets.intersect(&complement_of(&covered)).ok()?;
        (!uncovered.is_empty()).then_some(uncovered)
    }
}

/// The declared data baseline of a keyed axis — the modelled F5 surfaces
/// are declared against BIG-IP 15.0 (`VersionKey::baseline_version`,
/// restated for the model's [`KeyedAxis`]); the tool/SDC/UPF axes carry no
/// baseline until their first data backfill.
fn keyed_axis_baseline(axis: KeyedAxis) -> Option<&'static str> {
    match axis {
        KeyedAxis::BigipVersion => Some("15.0.0"),
        KeyedAxis::ToolVersion | KeyedAxis::SdcVersion | KeyedAxis::UpfVersion => None,
    }
}

/// The old `TclVersion` a ladder [`tcl_dialect::model::Release`] names,
/// `None` off the Tcl family ladder — transitional plumbing for the
/// ceiling comparison, retired with `TclVersion` itself.
fn tcl_version_of_release(release: tcl_dialect::model::Release) -> Option<TclVersion> {
    use tcl_dialect::model::Release;
    match release {
        Release::TCL_8_4 => Some(TclVersion::V8_4),
        Release::TCL_8_5 => Some(TclVersion::V8_5),
        Release::TCL_8_6 => Some(TclVersion::V8_6),
        Release::TCL_9_0 => Some(TclVersion::V9_0),
        Release::TCL_9_1 => Some(TclVersion::V9_1),
        _ => None,
    }
}

/// The release **line** of `release` on `family`'s ladder as a version
/// set: `[release, next-ladder-release)`, unbounded for the newest. `None`
/// when the ladder's spellings are not versions (the iRules `tmos` line).
fn release_line(
    family: Family,
    release: tcl_dialect::model::Release,
    start: &Version,
) -> Option<VersionSet> {
    let end = family
        .next_release(release)
        .and_then(|next| Version::parse(next.as_str()).ok());
    let requirement = match end {
        Some(end) => format!("{}-{}", start.as_str(), end.as_str()),
        None => format!("{}-", start.as_str()),
    };
    VersionSet::from_requirements(VersionAxisId::core(family), &[requirement]).ok()
}

/// Derive the point this context asks at (see
/// [`ResolvedContext::authoring_query`]): the core family at the core
/// axis's point primary — no primary, in the lenient environments, means
/// the family's whole ladder — plus every package this environment's own
/// runtime carries.
///
/// "Carries" is deliberately narrower than
/// [`ResolvedContext::package_active`]: only the environment's vendor
/// surface and its **ambient** placements. A *hosted* library that merely
/// resolves leniently (Tk under `tclsh`, every tcllib package) is not part
/// of the point, or every plain-Tcl document would author against Tk.
/// `tk` earns Tk here because it places it ambient.
fn compute_authoring_scope(context: &ResolvedContext) -> AuthoringScope {
    let mut scope = AuthoringScope::default();
    if let Some(core) = context.environment.core {
        // The Tcl-facing point a family derives from its ancestry anchor,
        // so the release it authors against is that anchor — derived, not
        // per family.
        let anchor_point = |family: Family| {
            (
                Family::Tcl,
                family
                    .ancestry()
                    .filter(|ancestry| ancestry.parent == Family::Tcl)
                    .and_then(|ancestry| Version::parse(ancestry.anchor).ok()),
            )
        };
        match core.family {
            // The primary pins the release; without one the question is
            // about the family's whole ladder.
            Family::Tcl => scope.core.push((
                Family::Tcl,
                context
                    .floors
                    .primary(&VersionAxisId::core(Family::Tcl))
                    .cloned(),
            )),
            Family::F5Irules => scope.core.push((Family::F5Irules, None)),
            // `f5-tcl`: measurements §4a (F5 reclassification,
            // `docs/design/f5/bigip-irule-parser-measurements.md`) — the
            // trunk-riding environments (`f5-iapps`, `f5-tmsh`) embed the
            // fork of Tcl at 8.4.6, and every 8.4/8.5 discriminator behaves
            // as 8.4.
            Family::F5Tcl => scope.core.push(anchor_point(Family::F5Tcl)),
            // `jim`: its own family first, then the 8.6 command-set anchor
            // (`jim_tcl.txt`). The anchor is how a `jim` document resolves
            // `set`, `if`, `proc`, `lassign`, `dict` and `lmap` from the
            // shared core specs; the own-family point is how a row that
            // says `available {jim 0.81-}` is admitted, and it is asked
            // first, so such a row shadows an inherited Tcl row for the
            // same command.
            Family::Jim => {
                scope.core.push((
                    Family::Jim,
                    context
                        .floors
                        .primary(&VersionAxisId::core(Family::Jim))
                        .cloned(),
                ));
                scope.core.push(anchor_point(Family::Jim));
            }
        }
    }
    // Preserve historical vendor order and floor recipe, then project genuine
    // arbitrary required/ambient provider facts. Ambient pack rows alone are
    // floor metadata, not activation; package_provider_active remains the gate.
    let mut names = Vec::<&str>::new();
    for &package in VENDOR_SURFACE_PACKAGES {
        if vendor_surface_package(context.environment.id.as_str()) == Some(package)
            || context.placement_is_ambient(package)
        {
            names.push(package);
        }
    }
    for package in context.required_packages.iter().map(AsRef::as_ref).chain(
        context
            .environment
            .expected_packages
            .iter()
            .filter(|row| row.ambient)
            .map(|row| row.package.as_ref()),
    ) {
        if context.package_provider_active(package) && !names.contains(&package) {
            names.push(package);
        }
    }
    for package in names {
        scope
            .packages
            .push(tcl_dialect::model::OwnedPackageFloor::new(
                package,
                context.pack_ambient_floor(package),
            ));
    }
    scope
}

/// The environment's own authoring surface, owned so the context can lend it
/// as a [`SurfaceQuery`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthoringScope {
    /// The core families this environment authors against, nearest first,
    /// each with the release when one is pinned. A family with an ancestry
    /// anchor lists itself, then the anchor.
    core: Vec<(Family, Option<Version>)>,
    /// Retained vendor and genuine required/ambient source package facts.
    /// Pack-declared floors preserve the historical floor-only recipe;
    /// they do not independently activate a package or prove installation.
    packages: Vec<tcl_dialect::model::OwnedPackageFloor>,
}

impl AuthoringScope {
    /// This scope as a query.
    #[must_use]
    pub fn query(&self) -> SurfaceQuery<'_> {
        SurfaceQuery {
            realm: tcl_dialect::model::InvocationRealm::RuleLoader,
            core: CorePoints::from_ordered(
                self.core
                    .iter()
                    .map(|(family, release)| (*family, release.as_ref().map(Version::as_str))),
            ),
            packages: tcl_dialect::model::PackageFloors::Owned(&self.packages),
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

/// The complement of `set` on its axis, over span ranges.
///
/// An [`HalfOpenRange::Exact`] point contributes nothing here — "everything
/// except exactly `v`" has no half-open spelling — so a point conservatively
/// stays *inside* the complement. That direction can only widen a reported
/// remainder, never hide one, and no gate-derived coverage set carries
/// points.
///
/// [`HalfOpenRange::Exact`]: tcl_dialect::model::HalfOpenRange::Exact
fn complement_of(set: &VersionSet) -> VersionSet {
    use tcl_dialect::model::HalfOpenRange;
    let mut ranges: Vec<HalfOpenRange> = Vec::new();
    let mut cursor = Some(Version::parse("0").expect("the zero version parses"));
    for range in set.ranges() {
        let HalfOpenRange::Span { min, max } = range else {
            continue;
        };
        if let Some(start) = cursor.take() {
            if start < *min {
                ranges.push(HalfOpenRange::Span {
                    min: start,
                    max: Some(min.clone()),
                });
            }
            cursor.clone_from(max);
        }
        if cursor.is_none() {
            break;
        }
    }
    if let Some(start) = cursor {
        ranges.push(HalfOpenRange::Span {
            min: start,
            max: None,
        });
    }
    VersionSet::from_ranges(set.axis().clone(), ranges)
}

/// Requirement-style spelling of `set` for diagnostics and status
/// surfaces: each range as `min-max` / `min-` (the `a0` bound pads
/// stripped), joined by spaces. The empty set spells `""`.
#[must_use]
pub fn requirement_spelling(set: &VersionSet) -> String {
    use tcl_dialect::model::HalfOpenRange;
    fn strip(version: &Version) -> &str {
        let text = version.as_str();
        text.strip_suffix("a0").unwrap_or(text)
    }
    set.ranges()
        .iter()
        .map(|range| match range {
            HalfOpenRange::Exact(version) => strip(version).to_owned(),
            HalfOpenRange::Span { min, max: None } => format!("{}-", strip(min)),
            HalfOpenRange::Span {
                min,
                max: Some(max),
            } => format!("{}-{}", strip(min), strip(max)),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The ladder releases of `set`'s core family whose release lines the set
/// touches — the human names a range diagnostic lists ("the declared
/// targets include 8.5"). Empty off a core axis, or where the ladder's
/// spellings are not versions (the iRules `tmm` line).
#[must_use]
pub fn ladder_releases_in(set: &VersionSet) -> Vec<&'static str> {
    let Some(family) = set.axis().core_family() else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for release in family.releases() {
        if Version::parse(release.as_str()).is_err() {
            continue;
        }
        // The **narrow** line `[R, R+ε)` — one minor — so the unshipped
        // ladder interior (8.7, 8.8) a coverage span deliberately includes
        // never makes a release's name appear in a remainder it is not
        // actually part of.
        let requirement = match bumped_spelling(release.as_str()) {
            Some(bound) => format!("{}-{bound}", release.as_str()),
            None => continue,
        };
        let Ok(line) = VersionSet::from_requirements(set.axis().clone(), &[requirement]) else {
            continue;
        };
        if line.intersect(set).is_ok_and(|overlap| !overlap.is_empty()) {
            names.push(release.as_str());
        }
    }
    names
}

/// The **targets grammar** (§5.4 / ruling R6) — how a `supports NAME
/// RANGE` directive or a `tclLsp.targets` entry spells a target set.
/// Space-separated clauses union; each clause is:
///
/// - `V` — the release **line** of `V` alone (`8.5` targets 8.5.x, never
///   8.6 — the §6.2 `available` grammar's bare-release rule, not the
///   `vsatisfies` next-major window);
/// - `MIN-` — everything from `MIN` (clamped to the modelled ladder on a
///   core axis);
/// - `MIN-MAX` — from `MIN` through the **whole release line of `MAX`**
///   (`tcl 8.5-9.0` includes 9.0.x — the §5.4 canonical spelling; the
///   strict `vsatisfies` exclusive-max reading would silently drop the
///   very release the declaration names).
///
/// On a core axis line bounds come from the family ladder; on a package
/// axis (no ladder) the line of `V` is `[V, V+ε)` with the last dotted
/// component bumped (`Tk 8.6` → `[8.6, 8.7)`).
///
/// # Errors
/// [`VersionSetError`] when a clause is not a well-formed version or
/// range — the ingress treats a malformed declaration as absent rather
/// than guessing.
pub fn targets_from_clauses<S: AsRef<str>>(
    axis: &VersionAxisId,
    clauses: &[S],
) -> Result<VersionSet, VersionSetError> {
    let mut requirements: Vec<String> = Vec::new();
    for clause in clauses {
        let clause = clause.as_ref();
        let requirement = match clause.split_once('-') {
            None => {
                Version::parse(clause)?;
                match next_line_bound(axis, clause) {
                    Some(next) => format!("{clause}-{next}"),
                    None => format!("{clause}-"),
                }
            }
            Some((min, "")) => {
                Version::parse(min)?;
                format!("{min}-")
            }
            Some((min, max)) => {
                Version::parse(min)?;
                Version::parse(max)?;
                match next_line_bound(axis, max) {
                    Some(next) => format!("{min}-{next}"),
                    None => format!("{min}-"),
                }
            }
        };
        requirements.push(requirement);
    }
    let declared = VersionSet::from_requirements(axis.clone(), &requirements)?;
    // A core axis clamps to the modelled ladder, so an open-ended `8.5-`
    // does not read past the newest modelled line and report every item
    // "missing" at releases that do not exist yet.
    if let Some(coverage) = ladder_coverage(axis) {
        return declared.intersect(&coverage);
    }
    Ok(declared)
}

/// The exclusive upper bound of the release line starting at `version` on
/// `axis`: the next ladder release above it (core axes), or the last
/// dotted component bumped by one (package axes and the ladder's top
/// line). `None` when no bound can be spelled (a non-numeric component).
fn next_line_bound(axis: &VersionAxisId, version: &str) -> Option<String> {
    let parsed = Version::parse(version).ok()?;
    if let Some(family) = axis.core_family() {
        for release in family.releases() {
            if let Ok(step) = Version::parse(release.as_str())
                && step > parsed
            {
                return Some(release.as_str().to_owned());
            }
        }
    }
    bumped_spelling(version)
}

/// `version` with its last dotted component bumped by one (`8.6` → `8.7`,
/// `2` → `3`) — the one-minor line width. `None` when the last component
/// is not a plain number.
fn bumped_spelling(version: &str) -> Option<String> {
    let mut parts: Vec<&str> = version.split('.').collect();
    let last = parts.pop()?;
    let bumped = last.parse::<u64>().ok()?.checked_add(1)?;
    let mut spelling = parts.join(".");
    if !spelling.is_empty() {
        spelling.push('.');
    }
    spelling.push_str(&bumped.to_string());
    Some(spelling)
}

/// The full modelled coverage of a core axis's ladder — first line start
/// to one past the newest line (`tcl` → `8.4-9.2`, matching
/// [`TCL_LINES`]' own bounds). `None` off a core axis or where the ladder
/// spells no versions.
fn ladder_coverage(axis: &VersionAxisId) -> Option<VersionSet> {
    let family = axis.core_family()?;
    let releases = family.releases();
    let first = releases.first()?;
    let last = releases.last()?;
    Version::parse(first.as_str()).ok()?;
    let requirement = match next_line_bound(axis, last.as_str()) {
        Some(bound) => format!("{}-{}", first.as_str(), bound),
        None => format!("{}-", first.as_str()),
    };
    VersionSet::from_requirements(axis.clone(), &[requirement]).ok()
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

/// Whether `spec` is one of an environment's **own vendor** commands: its
/// surface names that environment's vendor provider and no plain-Tcl row,
/// so the provider is what identifies it rather than shared library data
/// (tcllib's "everywhere but the closed sandboxes" rows name Tcl too). The
/// membership rule [`ResolvedContext::vendor_command_surface`] and
/// [`ResolvedContext::keyed_ambient_placement`] share.
fn is_vendor_own(spec: &CommandSpec, vendor: SpecProvider) -> bool {
    spec.surface.is_some_and(|rows| {
        rows.iter().any(|row| row.provider == vendor)
            && !rows
                .iter()
                .any(|row| row.provider == SpecProvider::Core(Family::Tcl))
    })
}

// Release ladders are compile-time family data. Parse them once without
// caching any context, selected surface, or declaration applicability.
fn parsed_family_ladder(family: Family) -> &'static [Version] {
    static LADDERS: OnceLock<[Vec<Version>; 4]> = OnceLock::new();
    let ladders = LADDERS.get_or_init(|| {
        [Family::Tcl, Family::F5Tcl, Family::F5Irules, Family::Jim].map(|family| {
            family
                .releases()
                .iter()
                .filter_map(|release| Version::parse(release.as_str()).ok())
                .collect()
        })
    });
    &ladders[match family {
        Family::Tcl => 0,
        Family::F5Tcl => 1,
        Family::F5Irules => 2,
        Family::Jim => 3,
    }]
}

/// The **total applicability breadth** of a declaration set — the
/// generalised most-specific-wins measure (§4.1: narrowest total
/// applicability beats widest; authoring precedence, never binding
/// resolution — B4).
///
/// Counted per row so that narrower authoring beats wider:
///
/// - a core row counts the ladder releases its set covers (a family whose
///   ladder carries no version spellings counts one for a non-empty set);
/// - a package row counts **one** when it is part of the vendor authoring
///   vocabulary ([`VENDOR_SURFACE_PACKAGES`]) and zero otherwise — a hosted
///   owning-package attribution row states where a command comes from, not
///   how narrowly it is authored, so it must not tighten specificity;
/// - a spec that states no surface at all therefore counts 22 (5 Tcl + 1
///   iRules + 9 Jim + 7 vendor packages), strictly wider than any explicit
///   gate's maximum of 13, so a catch-all loses to every scoped spec.
#[must_use]
pub fn specificity_breadth(declarations: &[SurfaceDeclaration]) -> u32 {
    let mut breadth = 0u32;
    for declaration in declarations {
        match &declaration.provider {
            Provider::Core(family) => {
                let mut covered = 0u32;
                let parsed = parsed_family_ladder(*family);
                for version in parsed {
                    if declaration.applicable.contains(version) {
                        covered += 1;
                    }
                }
                if parsed.is_empty() && !declaration.applicable.is_empty() {
                    covered = 1;
                }
                breadth += covered;
            }
            Provider::Package(package) => {
                if VENDOR_SURFACE_PACKAGES.contains(&package.as_str()) {
                    breadth += 1;
                }
            }
            // A document declaration is the narrowest surface there is —
            // one buffer — so it contributes no breadth and wins every
            // most-specific tiebreak against a catalogue row.
            Provider::Document => {}
        }
    }
    breadth
}

/// The lowest Tcl release a gate's core-Tcl rows name — the floor the §5.2
/// option-gating ceiling compares against.
///
/// `None` for a gate that names no plain-Tcl line at all: a pure vendor gate
/// has no version floor, so no ceiling can exclude it.
#[must_use]
pub fn core_tcl_floor(gate: &[SpecSurface]) -> Option<TclVersion> {
    gate.iter()
        .filter(|row| row.provider == SpecProvider::Core(Family::Tcl))
        .flat_map(|row| row.windows.iter())
        .filter_map(|&(from, _)| TclVersion::from_version_string(from))
        .min()
}

#[cfg(test)]
mod tests;
