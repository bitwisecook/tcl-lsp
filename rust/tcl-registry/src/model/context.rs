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

use tcl_dialect::model::Family;
use tcl_dialect::model::{
    CapabilityAnswer, CoreProfileId, EnvironmentDefinition, KeyedAxis, PackagePlacement, Placement,
    Version, VersionAxisId, VersionSet, VersionSetError, WorldPolicy,
};
use tcl_dialect::{DialectSet, LibraryVersionOverrides, TclVersion};

use crate::hover::OptionSpec;
use crate::model::surface::{
    BuildCapability, CapabilityPredicate, Provider, SurfaceDeclaration, TCL_LINES,
    VENDOR_BIT_PACKAGES, VENDOR_BITS, is_closed_world_package, is_placement_gated_package,
    vendor_surface_package,
};
// The vendor-surface summary payload: plain registry-derived data, not
// part of the retiring profile trait, so both faces answer with the one
// type and the parity pin can compare them directly. P1-G removed the
// trait from the public surface (it survives crate-internally, plus a
// cfg(test) oracle for the sweeps); the type moves here when the trait
// goes entirely under ledger C1/F1.
use crate::profile_queries::VendorSurface;
use crate::registry::CommandRegistry;
use crate::spec::{CommandSpec, SubCommand, SubSubCommand};
use crate::traits::Traits;

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
    /// The **authoring mask** this context admits — the old model's
    /// `availability_mask`, re-derived from the environment (core primary ×
    /// ladder lines, active vendor surfaces). Cached at resolution; the
    /// parity sweep pins it to the old profile's mask for every catalogue
    /// environment.
    authoring_mask: DialectSet,
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
            // walk serve a fork and a reimplementation without the
            // family special case this loop used to carry. An anchor that
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
            authoring_mask: DialectSet::empty(),
        };
        context.authoring_mask = compute_authoring_mask(&context);
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
    fn package_provider_active(&self, package: &str) -> bool {
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
    /// - plus, outside a **closed** world, any package no environment owns
    ///   as its closed-world runtime — §5.3's lenient `open` default,
    ///   "hosted packs visible, W120 advisory";
    /// - and nothing at all under `Closed`, where `package require` is not
    ///   part of the language.
    ///
    /// **P3 (the Tk pilot).** This is the query ledger row F4 retires
    /// `tk_loaded` / `hosts_tk` / the `TK_PACKAGE` substring scan onto, and
    /// the pilot is what makes it load-bearing: `Tk` is ambient under the
    /// `tk` environment and hosted under plain Tcl, so one function answers
    /// "is Tk in this document's world?" for both, and the two answers
    /// differ for the right reason — the placement, not the name. Callers
    /// wanting "…without a `package require`" ask
    /// [`Self::ambient_package`]; callers wanting "could this environment
    /// host it at all?" ask [`Self::can_host_package`].
    #[must_use]
    pub fn package_active(&self, package: &str) -> bool {
        self.package_provider_active(package)
            || (self.environment.policy_defaults.closed_world != WorldPolicy::Closed
                && !is_closed_world_package(package))
    }

    /// Whether this environment can **host** `package` as a `package
    /// require` — it declares a placement for it (§3.2's placement
    /// claims).
    ///
    /// The placement-model face of the retired `DialectProfile::hosts_tk`
    /// and of `DocumentEnvironment::can_host_package`'s lenient-sink
    /// special case (ledger F4): a closed-world vendor shell — the F5
    /// surfaces, the EDA shells, `bpf`, `spectcl` — declares no Tk
    /// placement and therefore cannot host it however the source spells
    /// its `package require`, while every plain-Tcl environment (the
    /// lenient `tcl` sink included) and `tk` itself declares one.
    ///
    /// Distinct from [`Self::package_active`]: hosting is what the
    /// *environment* offers, activation is what this *document* got.
    #[must_use]
    pub fn can_host_package(&self, package: &str) -> bool {
        self.placement(package).is_some()
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
    /// deliberately not open, exactly as the old bare-`IRULES` mask never
    /// admitted a plain Tcl-version gate.
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

    // --- the transitional spec-query surface (P1-F, compiler port) -----
    //
    // The methods below are the context-keyed replacements for the old
    // `ProfileQueries` availability surface: same registry data types, the
    // context's derived facts (authoring mask, ceiling, placements) in
    // place of the `DialectProfile` fields. Each derived fact is pinned to
    // the old profile's value by the parity sweeps in this module's tests,
    // so the bodies can mirror the old rules verbatim — behaviour is held
    // exactly while the *inputs* come from the centralised environment
    // model. The `DialectSet`-typed vocabulary they still speak retires
    // with the rest of the mask model under ledger C1 (post-P1-G).

    /// The authoring mask this context admits — the environment-derived
    /// mirror of the old `availability_mask` (sweep-pinned per catalogue
    /// environment). The lenient `tcl` fallback derives the permissive
    /// full-ladder mask, exactly as the old fallback profile answered;
    /// `tk` derives that ladder **plus** the `TK` bit, off its ambient
    /// placement.
    ///
    /// **P3 (the Tk pilot).** Until the pilot this field had a second,
    /// injected value: `DocumentEnvironment::document_context` overrode
    /// the derivation with the threaded profile's mask, because a `tk`
    /// document has always been answered under the additive `TK` bit and
    /// the derivation could not produce it. That door
    /// (`with_authoring_mask`) is deleted — the ambient Tk placement
    /// derives the bit — so the field now has exactly one source,
    /// [`compute_authoring_mask`], for every environment, and the
    /// analyser-vs-unit `tk` mask asymmetry waves 1-2 carried is gone.
    #[must_use]
    pub fn authoring_mask(&self) -> DialectSet {
        self.authoring_mask
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
    ///   installed, so W120 owns the nag, exactly as before;
    /// - a **placement-gated** package answers [`Self::package_active`] —
    ///   the environment's ambient closure, plus (outside a closed world)
    ///   the lenient hosted rule and this document's own requires.
    ///
    /// **P3 (the Tk pilot)**: `Tk` moves from the first class into the
    /// second, because `wish` runs it ambiently. The single enumerated
    /// consequence is that a **closed** world stops resolving Tk: a `.bpf`
    /// or `.tclspec` document can no longer call `wm` (`package require`
    /// is not part of either language, so it never could), while every
    /// open world answers exactly as before.
    ///
    /// [`is_placement_gated_package`]: crate::model::surface::is_placement_gated_package
    #[must_use]
    pub fn required_package_available(&self, required: Option<&str>) -> bool {
        match required {
            None => true,
            Some(package) => !is_placement_gated_package(package) || self.package_active(package),
        }
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
    /// session pin or the D5 oldest-supported default).
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
    }

    /// Whether `spec` is available in this context — the old
    /// `ProfileQueries::is_available` trio (mask membership, operator-head
    /// exclusion, required-package gate) over context-derived facts.
    #[must_use]
    pub fn spec_available(&self, spec: &CommandSpec) -> bool {
        spec.supports_dialect(self.authoring_mask)
            && (self.operator_heads_are_commands()
                || !spec.traits.contains(Traits::OPERATOR_COMMAND))
            && self.required_package_available(spec.required_package)
    }

    /// Resolve `name` to its command spec in this context over `registry`
    /// — the old `ProfileQueries::resolve_command` (mask query + the full
    /// availability filter), for call sites holding a command store that
    /// is not this context's own generation.
    #[must_use]
    pub fn resolve_spec(
        &self,
        registry: &CommandRegistry,
        name: &str,
    ) -> Option<&'static CommandSpec> {
        registry
            .get_for_dialect(name, self.authoring_mask)
            .filter(|spec| self.spec_available(spec))
    }

    /// Whether `sub` (of `spec`) is available here — the old
    /// `is_subcommand_available` gate-inheritance rule.
    #[must_use]
    pub fn subcommand_available(&self, spec: &CommandSpec, sub: &SubCommand) -> bool {
        sub.dialects
            .or(spec.dialects)
            .is_none_or(|gate| gate.intersects(self.authoring_mask))
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
            .dialects
            .or(sub.dialects)
            .or(spec.dialects)
            .is_none_or(|gate| gate.intersects(self.authoring_mask))
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

    /// Whether `opt` is available here given its inherited `parent_gate` —
    /// the old `is_option_available` §5.2 semantics: mask membership plus
    /// the gate's version floor against the environment's ceiling.
    #[must_use]
    pub fn option_available(&self, opt: &OptionSpec, parent_gate: Option<DialectSet>) -> bool {
        let Some(gate) = opt.dialects.or(parent_gate) else {
            // No restriction on the option or its parent.
            return true;
        };
        if !gate.intersects(self.authoring_mask) {
            return false;
        }
        match (gate.min_version(), self.tcl_version_ceiling()) {
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
            if self.option_available(opt, spec.dialects) && !names.contains(&opt.name) {
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
            if self.option_available(opt, spec.dialects) && !out.iter().any(|o| o.name == opt.name)
            {
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
    /// `sub.dialects.or(spec.dialects)` as the parent gate).
    #[must_use]
    pub fn available_sub_option_names(
        &self,
        spec: &CommandSpec,
        sub: &SubCommand,
    ) -> Vec<&'static str> {
        let parent = sub.dialects.or(spec.dialects);
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
        let parent = sub.dialects.or(spec.dialects);
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
                && self.option_available(opt, spec.dialects)
                && opt.available_for_version(package_version)
        };
        spec.options.iter().find(matches).or_else(|| {
            spec.command_forms
                .iter()
                .flat_map(|f| f.options.iter())
                .find(matches)
        })
    }

    /// The environment's own vendor **authoring bit**, when its surface is
    /// authored under one (the F5/expect/spectcl/bpf vendor vocabularies) —
    /// the old `DialectProfile::vendor_bit`, derived from the environment:
    /// the iRules core family authors under `IRULES`; the bridge surfaces
    /// author under their bridge package's bit.
    #[must_use]
    pub fn vendor_authoring_bit(&self) -> Option<DialectSet> {
        if self
            .environment
            .core
            .is_some_and(|core| core.family == Family::F5Irules)
        {
            return Some(DialectSet::IRULES);
        }
        let surface = vendor_surface_package(self.environment.id.as_str())?;
        VENDOR_BITS
            .iter()
            .find(|&&(_, package)| package == surface)
            .map(|&(bit, _)| bit)
    }

    /// This environment's **own vendor command surface** over `registry`,
    /// summarised from registry data — the old
    /// `ProfileQueries::vendor_surface`, derived from the resolved context:
    /// the commands available here that carry the environment's vendor
    /// authoring bit, grouped by `NS::` namespace prefix (bare names group
    /// under `""`), sorted by descending size then name.
    ///
    /// `None` for an environment with no vendor surface of its own, and for
    /// one whose surface resolves to nothing in `registry`. Feeds the
    /// generated consumers (the AI prompt's F5-surface summary) so prose can
    /// never drift from the data.
    ///
    /// Pinned equal to the retired profile query for every catalogue profile
    /// over its own generation (`vendor_surface_matches_the_profile_query`).
    #[must_use]
    pub fn vendor_command_surface(&self, registry: &CommandRegistry) -> Option<VendorSurface> {
        let vendor = self.vendor_authoring_bit()?;
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
            let ns = name.split_once("::").map_or("", |(ns, _)| ns);
            *by_ns.entry(ns.to_owned()).or_insert(0) += 1;
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

    /// The ambient **keyed** placement `spec` sits on here, if any — the
    /// old `keyed_pin_for`: the owning package's placement (only when
    /// keyed and ambient), or — for a vendor-own spec (gate carries the
    /// vendor authoring bit and no plain-Tcl version) — the environment's
    /// single ambient keyed placement.
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
        let vendor = self.vendor_authoring_bit()?;
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

    // --- §5.4 range targeting: declared target sets --------------------
    //
    // The additive range-mode queries (P1b). A document/project that
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

    /// The subset of the declared **core-Tcl** targets a `DialectSet`
    /// availability gate does not cover — the §5.4 range check for the
    /// mask-gated items (commands, subcommands, options) whose
    /// introduction is spelled as ladder-line bits rather than a
    /// lifecycle. `None` when no core targets are declared, the gate is
    /// absent (unrestricted), the gate names no plain-Tcl line at all
    /// (the item is another provider's — its own axis governs it), or
    /// every declared target is covered.
    #[must_use]
    pub fn targets_uncovered_by_gate(&self, gate: Option<DialectSet>) -> Option<VersionSet> {
        let axis = VersionAxisId::core(Family::Tcl);
        let targets = self.declared_targets(&axis)?;
        let bits = gate?;
        let covered = crate::model::surface::tcl_core_set(bits)?;
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
    let ladder = family.releases();
    let position = ladder.iter().position(|step| *step == release)?;
    let end = ladder
        .get(position + 1)
        .and_then(|next| Version::parse(next.as_str()).ok());
    let requirement = match end {
        Some(end) => format!("{}-{}", start.as_str(), end.as_str()),
        None => format!("{}-", start.as_str()),
    };
    VersionSet::from_requirements(VersionAxisId::core(family), &[requirement]).ok()
}

/// Derive the context's authoring mask (see
/// [`ResolvedContext::authoring_mask`]): each Tcl ladder line bit is
/// admitted exactly when the core axis's point primary sits inside the
/// line (no point primary — the lenient environments — admits the whole
/// ladder); the iRules core admits the bare `IRULES` bit; each vendor bit
/// is admitted when this context's own runtime carries its surface
/// package — the environment's vendor surface, or a package it places
/// **ambient**.
///
/// The vendor conjunct is deliberately narrower than
/// [`ResolvedContext::package_active`]: the mask is the old model's
/// "which profile does this document thread?" vocabulary, and a *hosted*
/// library that merely resolves leniently (Tk under `tclsh`, every
/// tcllib package) never set a bit. Reading `package_active` here would
/// hand `tcl8.6` the `TK` bit the moment P3 routed Tk through the
/// placement model. The pinned consequence is the one the pilot wants:
/// `tk` earns the `TK` bit from its **ambient** placement, which is
/// exactly the promotion `DocumentEnvironment::document_context` used to
/// apply by hand.
fn compute_authoring_mask(context: &ResolvedContext) -> DialectSet {
    let mut mask = DialectSet::empty();
    if let Some(core) = context.environment.core {
        match core.family {
            Family::Tcl => {
                let axis = VersionAxisId::core(Family::Tcl);
                let primary = context.floors.primary(&axis);
                for (bit, start, end) in TCL_LINES {
                    let admitted = primary.is_none_or(|point| {
                        let start = Version::parse(start).expect("compiled line start parses");
                        let end = Version::parse(end).expect("compiled line end parses");
                        *point >= start && *point < end
                    });
                    if admitted {
                        mask = mask.union(bit);
                    }
                }
            }
            Family::F5Irules => {
                mask = mask.union(DialectSet::IRULES);
            }
            // Every other family derives its Tcl-facing surface from an
            // ancestry anchor, so the admitted ladder bit is the line
            // that anchor sits in — derived, not per family.
            //
            // `f5-tcl`: measurements §4a (F5 reclassification,
            // `docs/design/bigip-irule-parser-measurements.md`) — the
            // trunk-riding environments (`f5-iapps`, `f5-tmsh`) embed the
            // fork of Tcl at 8.4.6, every 8.4/8.5 discriminator behaves
            // as 8.4, so the embedded core admits the 8.4 line and not
            // the falsified 8.5 one the old profiles claimed.
            //
            // `jim`: the 8.6 command-set anchor (`jim_tcl.txt`), which is
            // the whole of P6's inherit-then-override — a `jim` document
            // resolves `set`, `if`, `proc`, `lassign`, `dict` and `lmap`
            // from the shared core specs instead of from 76
            // hand-re-authored copies.
            Family::F5Tcl | Family::Jim => {
                if let Some(ancestry) = core.family.ancestry()
                    && ancestry.parent == Family::Tcl
                    && let Ok(anchor) = Version::parse(ancestry.anchor)
                {
                    for (bit, start, end) in TCL_LINES {
                        let start = Version::parse(start).expect("compiled line start parses");
                        let end = Version::parse(end).expect("compiled line end parses");
                        if anchor >= start && anchor < end {
                            mask = mask.union(bit);
                        }
                    }
                }
            }
        }
    }
    for (bit, package) in VENDOR_BITS {
        if vendor_surface_package(context.environment.id.as_str()) == Some(package)
            || context.placement_is_ambient(package)
        {
            mask = mask.union(bit);
        }
    }
    mask
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
        // The **narrow** line `[R, R+ε)` — one minor, the same width
        // [`crate::model::surface::TCL_LINES`] gives each availability
        // bit — so the unshipped ladder interior (8.7, 8.8) a coverage
        // span deliberately includes never makes a release's name appear
        // in a remainder it is not actually part of.
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
/// gate carries that environment's vendor authoring bit and no plain-Tcl
/// version, so the vendor bit is the discriminating tag rather than shared
/// library data (tcllib's "everywhere but the closed sandboxes" complement
/// gates). The membership rule
/// [`ResolvedContext::vendor_command_surface`] and
/// [`ResolvedContext::keyed_ambient_placement`] share.
fn is_vendor_own(spec: &CommandSpec, vendor: DialectSet) -> bool {
    spec.dialects
        .is_some_and(|d| d.intersects(vendor) && !d.intersects(DialectSet::ALL_TCL))
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

    // --- P3: the Tk pilot's placement model ---------------------------

    /// The pilot's central claim, stated as one table: `Tk` is one
    /// package whose availability is decided by **placement plus policy**,
    /// and the three answers a caller can want are three distinct queries
    /// that disagree in exactly the right places.
    #[test]
    fn tk_is_one_package_with_three_placement_answers() {
        // (environment, in the world, ambient — no require needed, hostable)
        for (environment, active, ambient, hostable) in [
            // `wish`: ambient. Everything true, and W120 owns the silence.
            ("tk", true, true, true),
            // A release-pinned `tclsh`: hosted. Visible under the open
            // world (§5.3's lenient default), but not shipped, so W120 nags
            // and the `TK` authoring bit stays off.
            ("tcl8.6", true, false, true),
            ("tcl9.0", true, false, true),
            // The lenient sink declares the same hosted placement.
            ("tcl", true, false, true),
            // A closed world is exactly its ambient closure: `package
            // require` is not part of the language, so Tk is neither
            // active nor hostable.
            ("f5-irules", false, false, false),
            ("bpf", false, false, false),
            ("spectcl", false, false, false),
            // Ambient-plus-require shells host nothing they did not place.
            ("f5-iapps", true, false, false),
            ("f5-tmsh", true, false, false),
            // An EDA shell is open, so the lenient hosted rule applies —
            // but it declares no Tk placement, so it cannot host one.
            ("xilinx-eda-tcl", true, false, false),
        ] {
            let ctx = context(environment);
            assert_eq!(ctx.package_active("Tk"), active, "{environment} active");
            assert_eq!(ctx.ambient_package("Tk"), ambient, "{environment} ambient");
            assert_eq!(
                ctx.can_host_package("Tk"),
                hostable,
                "{environment} hostable"
            );
        }
    }

    /// `Tk` is ambient somewhere **and** hosted elsewhere, which is what
    /// keeps it out of the closed-world vocabulary: reading ambience alone
    /// would classify it as a vendor runtime the moment the pilot placed
    /// it, and every Tk command would vanish from plain Tcl.
    #[test]
    fn tk_is_a_library_with_an_ambient_host_not_a_vendor_surface() {
        use crate::model::surface::{is_closed_world_package, is_placement_gated_package};
        assert!(!is_closed_world_package("Tk"));
        assert!(is_placement_gated_package("Tk"));
        assert!(context("tk").placement_is_ambient("Tk"));
        assert!(context("tcl8.6").can_host_package("Tk"));
        assert!(!context("tcl8.6").placement_is_ambient("Tk"));
        // The vendor runtimes stay closed-world; the unplaced libraries
        // stay ungated (W120 owns their nag, as before).
        for vendor in ["f5-irules-cmds", "f5-iapps-cmds", "Expect"] {
            assert!(is_closed_world_package(vendor), "{vendor}");
        }
        for library in ["Itcl", "csv", "struct::graph", "tcltest"] {
            assert!(!is_closed_world_package(library), "{library}");
            assert!(!is_placement_gated_package(library), "{library}");
        }
    }

    /// Every `required_package: Some("Tk")` spec now carries the package
    /// conjunct, so the Tk surface resolves through
    /// [`ResolvedContext::package_active`] rather than off its
    /// `Core(Tcl)` row. The surviving core row is specificity data — the
    /// breadth the coexisting `get_for_dialect` popcount ordering needs —
    /// which is why it is asserted here rather than deleted.
    #[test]
    fn tk_declarations_are_gated_on_the_package_provider() {
        use crate::model::surface::{CapabilityPredicate, PackageId, Provider};
        let spec = CommandSpec {
            name: "context-test",
            dialects: Some(DialectSet::TK_AND_TCL),
            required_package: Some("Tk"),
            ..CommandSpec::DEFAULT
        };
        let declarations = declarations_for_spec(&spec);
        assert!(
            declarations
                .iter()
                .all(|row| row.predicate
                    == CapabilityPredicate::RequiresPackage(PackageId::new("Tk"))),
            "{declarations:?}"
        );
        assert!(
            declarations
                .iter()
                .any(|row| row.provider == Provider::Package(PackageId::new("Tk")))
        );
        // Specificity is unchanged: five Tcl ladder releases + the Tk
        // vendor-bit package = the old `TK_AND_TCL` mask popcount of 6.
        assert_eq!(specificity_breadth(&declarations), 6);
        // …and availability follows the placement query exactly.
        for environment in ["tk", "tcl8.6", "tcl", "f5-iapps"] {
            assert!(
                context(environment).is_available(&declarations),
                "{environment}"
            );
        }
    }

    /// The **one enumerated P3 delta** from the old model, pinned
    /// directly rather than only as an allowlist in the P1-E sweeps: a
    /// closed world stops resolving the Tk surface. `package require` is
    /// not part of the `bpf` or `spectcl` language, so `wm` was never
    /// callable there; the old profile mask admitted it only because
    /// `TK_AND_TCL` unions the whole Tcl ladder.
    #[test]
    fn tk_is_closed_out_of_closed_worlds() {
        let spec = CommandSpec {
            name: "context-test",
            dialects: Some(DialectSet::TK_AND_TCL),
            required_package: Some("Tk"),
            ..CommandSpec::DEFAULT
        };
        let declarations = declarations_for_spec(&spec);
        for closed in ["bpf", "spectcl", "f5-irules"] {
            assert!(!context(closed).is_available(&declarations), "{closed}");
            assert!(
                !context(closed).required_package_available(Some("Tk")),
                "{closed}"
            );
        }
        // An explicit require cannot open a closed world either.
        let mut required = context("spectcl");
        required.require_package("Tk", None);
        assert!(!required.is_available(&declarations));
        // Nothing else moves: an unplaced library keeps the old lenient
        // answer under the same closed worlds.
        for closed in ["bpf", "spectcl"] {
            assert!(
                context(closed).required_package_available(Some("Itcl")),
                "{closed}"
            );
            assert!(
                context(closed).required_package_available(Some("csv")),
                "{closed}"
            );
        }
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

    /// **P6.** A `jim` context resolves the shared core surface through
    /// its ancestry edge instead of through 76 re-authored specs: the
    /// `Core(Tcl)` provider is active, the Tcl-axis primary is the 8.6
    /// anchor, and the derived authoring mask is the 8.6 line — so
    /// `lassign` (8.5+) and `lmap` (8.6+) both resolve while an
    /// 8.4-only shape does not.
    #[test]
    fn a_jim_context_inherits_the_tcl_core_surface() {
        let ctx = context("jim");
        let jim_axis = VersionAxisId::core(tcl_dialect::model::Family::Jim);
        let tcl_axis = VersionAxisId::core(tcl_dialect::model::Family::Tcl);

        assert!(ctx.provider_active(&Provider::Core(tcl_dialect::model::Family::Jim)));
        assert!(
            ctx.provider_active(&Provider::Core(tcl_dialect::model::Family::Tcl)),
            "the ancestry edge is what admits the shared core specs"
        );
        assert!(!ctx.provider_active(&Provider::Core(tcl_dialect::model::Family::F5Irules)));

        // The lineage floor is the edge's anchor, on the *Tcl* axis.
        assert_eq!(
            ctx.floors.primary(&tcl_axis).map(Version::as_str),
            Some(tcl_dialect::model::Family::JIM_SURFACE_ANCHOR)
        );
        // Jim's own axis spans the ladder and takes no point primary.
        assert!(ctx.floors.primary(&jim_axis).is_none());
        assert!(
            ctx.floors
                .targets(&jim_axis)
                .expect("jim targets")
                .contains(&Version::parse("0.76").expect("version"))
        );

        // The derived mask, and what it admits.
        assert_eq!(ctx.authoring_mask(), DialectSet::TCL86);
        for gate in [
            DialectSet::ALL_TCL,
            DialectSet::TCL85_PLUS,
            DialectSet::TCL86_PLUS,
        ] {
            assert!(
                ctx.spec_available(&CommandSpec {
                    name: "context-test",
                    dialects: Some(gate),
                    ..CommandSpec::DEFAULT
                }),
                "{gate:?}"
            );
        }
        assert!(
            !ctx.spec_available(&CommandSpec {
                name: "context-test",
                dialects: Some(DialectSet::TCL84),
                ..CommandSpec::DEFAULT
            }),
            "an 8.4-only shape is not part of the 8.6 command set"
        );
        // A real command, resolved from the shared catalogue: this is
        // the 76-commands-by-hand line item, gone.
        let registry = crate::model::assembly::universe();
        for name in ["set", "if", "proc", "lassign", "lmap", "dict"] {
            assert!(ctx.resolve_spec(registry, name).is_some(), "{name}");
        }
    }

    /// **P6, invariant I2.** A declared `jim` range gates on the jim
    /// axis and says nothing on the Tcl axis — and vice versa. The
    /// axis machinery needed no jim-specific code: `targets_from_clauses`,
    /// `ladder_releases_in` and `ladder_coverage` all read the family's
    /// own ladder.
    #[test]
    fn a_declared_jim_range_gates_on_the_jim_axis_only() {
        let jim_axis = VersionAxisId::core(tcl_dialect::model::Family::Jim);
        let tcl_axis = VersionAxisId::core(tcl_dialect::model::Family::Tcl);
        let mut ctx = context("jim");

        // The targets grammar spells jim ranges off the jim ladder: a
        // bare release is that release's line, an inclusive max covers
        // the whole line of the named release, and an open end clamps to
        // the modelled ladder.
        let declared = targets_from_clauses(&jim_axis, &["0.76-0.79"]).expect("well-formed");
        assert_eq!(
            ladder_releases_in(&declared),
            vec!["0.76", "0.77", "0.78", "0.79"]
        );
        assert!(!declared.contains(&Version::parse("0.80").expect("version")));
        let open = targets_from_clauses(&jim_axis, &["0.81-"]).expect("well-formed");
        assert_eq!(
            ladder_releases_in(&open),
            vec!["0.81", "0.82", "0.83", "0.84"]
        );
        assert!(
            !open.contains(&Version::parse("0.85").expect("version")),
            "an open end clamps to the modelled ladder"
        );

        ctx.declare_targets(declared);
        assert!(ctx.declared_targets(&jim_axis).is_some());
        assert!(
            ctx.declared_targets(&tcl_axis).is_none(),
            "I2: a jim declaration is not a Tcl declaration"
        );

        // The window query answers on the jim axis. `lt`/`ge` arrive at
        // Jim 0.80, so a 0.76-0.79 project is told so.
        let outside = ctx
            .targets_outside_window(&jim_axis, Some("0.80"), None)
            .expect("the whole declaration is below 0.80");
        assert_eq!(
            ladder_releases_in(&outside),
            vec!["0.76", "0.77", "0.78", "0.79"]
        );
        assert_eq!(requirement_spelling(&outside), "0.76-0.80");
        // A window the declaration sits inside abstains.
        assert!(
            ctx.targets_outside_window(&jim_axis, Some("0.76"), None)
                .is_none()
        );
        // …and the same question on the Tcl axis abstains entirely: the
        // jim declaration is not evidence about Tcl releases.
        assert!(
            ctx.targets_outside_window(&tcl_axis, Some("8.6"), None)
                .is_none(),
            "I2: the jim range must not leak onto the Tcl axis"
        );

        // The leak is unrepresentable, not merely untested: the two sets
        // cannot be compared at all.
        let tcl_targets = targets_from_clauses(&tcl_axis, &["8.5-9.0"]).expect("well-formed");
        let jim_targets = targets_from_clauses(&jim_axis, &["0.81-"]).expect("well-formed");
        assert!(
            jim_targets.intersect(&tcl_targets).is_err(),
            "I2: cross-axis operations are typed errors"
        );

        // A declared *Tcl* range on a jim context gates the Tcl axis and
        // leaves jim's alone — the mirror direction.
        let mut mirror = context("jim");
        mirror.declare_targets(tcl_targets);
        assert!(mirror.declared_targets(&tcl_axis).is_some());
        assert!(mirror.declared_targets(&jim_axis).is_none());
        assert!(
            mirror
                .targets_outside_window(&jim_axis, Some("0.80"), None)
                .is_none()
        );
    }

    /// **P6.** A `Core(Jim)` declaration restricted to part of the jim
    /// ladder reports exactly the covered subset — the same
    /// `available_at_targets` machinery Tk and the tcllib modules use,
    /// on a core family's own axis.
    #[test]
    fn available_at_targets_answers_on_the_jim_axis() {
        let ctx = context("jim");
        let jim_axis = VersionAxisId::core(tcl_dialect::model::Family::Jim);
        let modern = vec![SurfaceDeclaration {
            provider: Provider::Core(tcl_dialect::model::Family::Jim),
            applicable: targets_from_clauses(&jim_axis, &["0.80-"]).expect("well-formed"),
            predicate: crate::model::surface::CapabilityPredicate::None,
            history: tcl_dialect::model::ItemHistory::default(),
            provenance: tcl_dialect::model::Provenance::BuiltIn,
        }];
        let covered = ctx.available_at_targets(&modern, &jim_axis);
        assert!(covered.contains(&Version::parse("0.84").expect("version")));
        assert!(!covered.contains(&Version::parse("0.79").expect("version")));
        // The declaration does not cover the environment's whole ladder.
        let targets = ctx.floors.targets(&jim_axis).expect("targets");
        assert!(!targets.subset(&covered).expect("same axis"));
        // A Tcl-provider row contributes nothing to the jim axis.
        assert!(
            ctx.available_at_targets(&rows(Some(DialectSet::ALL_TCL)), &jim_axis)
                .is_empty()
        );
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
        // gate (old rule: a catch-all loses to every scoped spec). 23 =
        // 5 Tcl releases + 1 f5-tcl + 1 f5-irules + 9 jim + 7 vendor
        // packages — the `f5-tcl` trunk family (measurements §4a) added
        // its row in the F5 reclassification.
        assert_eq!(specificity_breadth(&rows(None)), 23);
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

    /// **P1-F parity sweep 1**: every context-derived authoring fact —
    /// the mask, the option ceiling, the operator-head rule, the vendor
    /// authoring bit, and every placement's ambience and static floor —
    /// reproduces the old profile's value for every catalogue profile,
    /// and the lenient `tcl`/`tk` environments reproduce the permissive
    /// fallback profile the analyser resolved those names to.
    #[test]
    fn authoring_facts_reproduce_every_catalogue_profile() {
        use tcl_dialect::DialectProfile;
        for profile in DialectProfile::all() {
            let ctx = context(profile.name);
            // documented deltas (mask, ceiling, operator heads).
            assert_eq!(
                ctx.authoring_mask(),
                profile.availability_mask,
                "{} mask",
                profile.name
            );
            assert_eq!(
                ctx.tcl_version_ceiling(),
                profile.version_ceiling,
                "{} ceiling",
                profile.name
            );
            assert_eq!(
                ctx.operator_heads_are_commands(),
                profile.operators_as_commands,
                "{} operator heads",
                profile.name
            );
            assert_eq!(
                ctx.vendor_authoring_bit(),
                profile.vendor_bit,
                "{} vendor bit",
                profile.name
            );
            for pin in profile.libraries {
                assert_eq!(
                    ctx.placement_is_ambient(pin.package),
                    profile.is_ambient_package(pin.package),
                    "{} ambience of {}",
                    profile.name,
                    pin.package
                );
                assert_eq!(
                    ctx.placement_floor(pin.package).map(Version::as_str),
                    profile.library_floor_default(pin.package),
                    "{} floor of {}",
                    profile.name,
                    pin.package
                );
            }
            // The keyed axes honour a session pin exactly as
            // `library_floor` with overrides does.
            let overrides = LibraryVersionOverrides {
                bigip_version: Some("17.1.0".to_owned()),
                ..LibraryVersionOverrides::default()
            };
            let keyed = KeyedVersions::from_overrides(&overrides).expect("valid overrides");
            let registry = EnvironmentRegistry::compiled();
            let pinned = ResolvedContext::resolve(
                registry.resolve(profile.name).expect(profile.name),
                &keyed,
            );
            for pin in profile.libraries {
                assert_eq!(
                    pinned.placement_floor(pin.package).map(Version::as_str),
                    profile.library_floor(pin.package, &overrides),
                    "{} pinned floor of {}",
                    profile.name,
                    pin.package
                );
            }
        }
        // The analyser resolved `tcl` (and every unknown name) to the
        // permissive fallback profile; its environment derives the same
        // permissive facts.
        let plain = DialectProfile::plain_tcl();
        let ctx = context("tcl");
        assert_eq!(ctx.authoring_mask(), plain.availability_mask);
        assert_eq!(ctx.tcl_version_ceiling(), None);
        assert!(ctx.operator_heads_are_commands());
        assert_eq!(ctx.vendor_authoring_bit(), None);
        // `tk` derives the same permissive facts **plus** the `TK` bit —
        // P3: its ambient Tk placement produces the bit the ingress used
        // to inject over this derivation (`with_authoring_mask`, deleted).
        // It is still not a *vendor* environment: `TK` is a library bit,
        // so no vendor authoring bit and no ceiling.
        let tk = context("tk");
        assert_eq!(
            tk.authoring_mask(),
            plain.availability_mask.union(DialectSet::TK)
        );
        assert_eq!(tk.tcl_version_ceiling(), None);
        assert!(tk.operator_heads_are_commands());
        assert_eq!(tk.vendor_authoring_bit(), None);
        assert!(tk.placement_is_ambient("Tk"));
    }

    /// **P1-F parity sweep 2**: the spec/subcommand/option availability
    /// queries answer exactly as the old `ProfileQueries` for every spec
    /// in the compiled universe under every catalogue profile — commands,
    /// each subcommand, each sub-subcommand (at no version and at a pinned
    /// one), and every option at its inherited parent gate.
    #[test]
    fn spec_queries_reproduce_profile_queries_for_every_profile() {
        use crate::profile_queries::{LegacyProfileOracle, ProfileQueries};
        use tcl_dialect::DialectProfile;
        let universe = crate::model::assembly::universe();
        let mut checks = 0usize;
        for profile in DialectProfile::all() {
            let ctx = context(profile.name);
            for name in universe.command_names() {
                for spec in universe.specs(name) {
                    assert_eq!(
                        ctx.spec_available(spec),
                        crate::model::assembly::tests::old_available_after_p3(profile, spec),
                        "`{name}` under `{}`",
                        profile.name
                    );
                    assert_eq!(
                        ctx.keyed_ambient_placement(spec)
                            .map(|placement| placement.package.as_ref().to_owned()),
                        profile
                            .keyed_pin_for(spec)
                            .map(|pin| pin.package.to_owned()),
                        "`{name}` keyed pin under `{}`",
                        profile.name
                    );
                    assert_eq!(
                        ctx.keyed_version_range(spec),
                        profile.keyed_version_range(spec),
                        "`{name}` keyed range under `{}`",
                        profile.name
                    );
                    for sub in spec.subcommands {
                        assert_eq!(
                            ctx.subcommand_available(spec, sub),
                            profile.is_subcommand_available(spec, sub),
                            "`{name} {}` under `{}`",
                            sub.name,
                            profile.name
                        );
                        for sub_sub in sub.sub_subcommands {
                            for version in [None, Some("1.0"), Some("99.0")] {
                                assert_eq!(
                                    ctx.sub_subcommand_available(spec, sub, sub_sub, version),
                                    profile
                                        .is_sub_subcommand_available(spec, sub, sub_sub, version),
                                    "`{name} {} {}` under `{}` at {version:?}",
                                    sub.name,
                                    sub_sub.name,
                                    profile.name
                                );
                            }
                        }
                        assert_eq!(
                            ctx.available_sub_option_names(spec, sub),
                            profile.available_sub_option_names(spec, sub),
                            "`{name} {}` options under `{}`",
                            sub.name,
                            profile.name
                        );
                        checks += 1;
                    }
                    let parent = spec.dialects;
                    for opt in spec
                        .options
                        .iter()
                        .chain(spec.command_forms.iter().flat_map(|form| form.options))
                    {
                        assert_eq!(
                            ctx.option_available(opt, parent),
                            profile.is_option_available(opt, parent),
                            "`{name} {}` under `{}`",
                            opt.name,
                            profile.name
                        );
                        checks += 1;
                    }
                    assert_eq!(
                        ctx.available_option_names(spec),
                        profile.available_option_names(spec),
                        "`{name}` option names under `{}`",
                        profile.name
                    );
                    checks += 1;
                }
            }
        }
        println!("profile-query parity sweep: {checks} item checks, 0 divergences");
    }

    /// **P1-F wave-4 parity pin**: the context's vendor-surface summary
    /// reproduces `ProfileQueries::vendor_surface` for every catalogue
    /// profile, over that profile's own registry generation — the store
    /// the generated AI prompt has always read it from. At least one
    /// profile must actually have a surface, or the sweep would pass
    /// vacuously on a pair of `None`s.
    #[test]
    fn vendor_surface_matches_the_profile_query() {
        use crate::profile_queries::LegacyProfileOracle;
        use tcl_dialect::DialectProfile;
        let mut with_surface = 0usize;
        for profile in DialectProfile::all() {
            let ctx = context(profile.name);
            let store = crate::model::ingress::static_context_for(profile.name).commands();
            let ported = ctx.vendor_command_surface(store);
            assert_eq!(
                ported,
                profile.vendor_surface(store),
                "{} vendor surface",
                profile.name
            );
            if ported.is_some() {
                with_surface += 1;
            }
        }
        assert!(
            with_surface > 0,
            "no catalogue profile has a vendor surface — the sweep is vacuous"
        );
        // The lenient sink and the additive `tk` ingress author under no
        // vendor bit, so neither has a surface of its own.
        for lenient in ["tcl", "tk"] {
            let store = crate::model::ingress::static_context_for(lenient).commands();
            assert!(
                context(lenient).vendor_command_surface(store).is_none(),
                "{lenient} has no vendor surface"
            );
        }
    }

    /// §5.4 range targeting: the targets grammar, the window/gate
    /// remainder queries, and the diagnostic spellings.
    #[test]
    fn declared_targets_answer_the_range_queries() {
        let axis = VersionAxisId::core(tcl_dialect::model::Family::Tcl);
        let mut ctx = context("tcl8.6");
        // No declaration ⇒ every range query abstains (the no-range pin).
        assert!(ctx.declared_targets(&axis).is_none());
        assert!(
            ctx.targets_outside_window(&axis, Some("8.6"), None)
                .is_none()
        );
        assert!(
            ctx.targets_uncovered_by_gate(Some(DialectSet::TCL86_PLUS))
                .is_none()
        );

        // The §5.4 canonical declaration: `tcl 8.5-9.0` covers the whole
        // 9.0 line (inclusive-max reading), not the vsatisfies exclusive
        // one.
        let declared = targets_from_clauses(&axis, &["8.5-9.0"]).expect("well-formed");
        assert!(declared.contains(&Version::parse("9.0.1").expect("version")));
        assert!(declared.contains(&Version::parse("8.5").expect("version")));
        assert!(!declared.contains(&Version::parse("9.1").expect("version")));
        assert_eq!(ladder_releases_in(&declared), vec!["8.5", "8.6", "9.0"]);
        ctx.declare_targets(declared.clone());
        assert_eq!(ctx.declared_targets(&axis), Some(&declared));

        // An item introduced at 8.6: the remainder is exactly the 8.5
        // line, spelled and named for the message.
        let outside = ctx
            .targets_outside_window(&axis, Some("8.6"), None)
            .expect("8.5 is outside");
        assert_eq!(ladder_releases_in(&outside), vec!["8.5"]);
        assert_eq!(requirement_spelling(&outside), "8.5-8.6");
        // An item retired at 9.0: the remainder is the 9.0 line.
        let retired = ctx
            .targets_outside_window(&axis, None, Some("9.0"))
            .expect("9.0 is outside");
        assert_eq!(ladder_releases_in(&retired), vec!["9.0"]);
        // A window covering the whole declaration abstains.
        assert!(
            ctx.targets_outside_window(&axis, Some("8.5"), None)
                .is_none()
        );

        // Gate coverage: an 8.6+ gate misses the 8.5 target; ALL_TCL and
        // an absent gate cover everything; a vendor-only gate is another
        // provider's item and abstains.
        let uncovered = ctx
            .targets_uncovered_by_gate(Some(DialectSet::TCL86_PLUS))
            .expect("8.5 uncovered");
        assert_eq!(ladder_releases_in(&uncovered), vec!["8.5"]);
        assert!(
            ctx.targets_uncovered_by_gate(Some(DialectSet::ALL_TCL))
                .is_none()
        );
        assert!(ctx.targets_uncovered_by_gate(None).is_none());
        assert!(
            ctx.targets_uncovered_by_gate(Some(DialectSet::TK))
                .is_none()
        );
        // An 8.x-only gate misses the 9.0 end.
        let gone = ctx
            .targets_uncovered_by_gate(Some(DialectSet::TCL8X))
            .expect("9.0 uncovered");
        assert_eq!(ladder_releases_in(&gone), vec!["9.0"]);

        // Redeclaring an axis replaces, never accumulates.
        ctx.declare_targets(targets_from_clauses(&axis, &["9.0"]).expect("well-formed"));
        assert!(
            ctx.targets_uncovered_by_gate(Some(DialectSet::TCL86_PLUS))
                .is_none(),
            "a single 9.0 target is covered by an 8.6+ gate"
        );
        assert_eq!(ctx.declared_target_sets().len(), 1);
    }

    /// The targets grammar's clause forms, including the package-axis
    /// line rule and the ladder clamp.
    #[test]
    fn the_targets_grammar_spells_lines_and_ranges() {
        let core = VersionAxisId::core(tcl_dialect::model::Family::Tcl);
        // Bare release: the line alone, not the vsatisfies next-major
        // window.
        let line = targets_from_clauses(&core, &["8.5"]).expect("well-formed");
        assert!(line.contains(&Version::parse("8.5.19").expect("version")));
        assert!(!line.contains(&Version::parse("8.6").expect("version")));
        // Open-ended: clamped to the modelled ladder.
        let open = targets_from_clauses(&core, &["8.5-"]).expect("well-formed");
        assert!(open.contains(&Version::parse("9.1").expect("version")));
        assert!(!open.contains(&Version::parse("9.2").expect("version")));
        // Multi-clause union (the `8.5 9.0` spelling).
        let union = targets_from_clauses(&core, &["8.5", "9.0"]).expect("well-formed");
        assert!(union.contains(&Version::parse("8.5").expect("version")));
        assert!(union.contains(&Version::parse("9.0.1").expect("version")));
        assert!(!union.contains(&Version::parse("8.6").expect("version")));
        // Package axis: bare release bumps the last component.
        let tk = VersionAxisId::package("Tk");
        let tk_line = targets_from_clauses(&tk, &["8.6"]).expect("well-formed");
        assert!(tk_line.contains(&Version::parse("8.6.12").expect("version")));
        assert!(!tk_line.contains(&Version::parse("8.7").expect("version")));
        let tk_range = targets_from_clauses(&tk, &["8.5-8.6"]).expect("well-formed");
        assert!(tk_range.contains(&Version::parse("8.6.12").expect("version")));
        assert!(!tk_range.contains(&Version::parse("8.7").expect("version")));
        // Malformed clauses error rather than guess.
        assert!(targets_from_clauses(&core, &["not-a-version"]).is_err());
        assert!(targets_from_clauses(&core, &["8.5-9.0-9.1"]).is_err());
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
