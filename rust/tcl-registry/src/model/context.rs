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
    VENDOR_BIT_PACKAGES, VENDOR_BITS, ambient_placement_packages, is_closed_world_package,
    vendor_surface_package,
};
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
    // with the rest of the mask model in P1-G.

    /// The authoring mask this context admits — the environment-derived
    /// mirror of the old `availability_mask` (sweep-pinned per catalogue
    /// environment). The lenient `tcl` fallback and `tk` environments
    /// derive the permissive full-ladder mask, exactly as the old
    /// fallback profile answered.
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
    /// `ProfileQueries::package_available` rule with context-derived
    /// inputs: a hosted library (never ambient in any environment) is
    /// always satisfied (W120 owns the nag), a closed-world vendor
    /// package only where this context ships it ambient.
    #[must_use]
    pub fn required_package_available(&self, required: Option<&str>) -> bool {
        match required {
            None => true,
            Some(package) => {
                self.placement_is_ambient(package)
                    || !ambient_placement_packages().contains(package)
            }
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
    pub fn resolve_spec<'r>(
        &self,
        registry: &'r CommandRegistry,
        name: &str,
    ) -> Option<&'r CommandSpec> {
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
        let vendor_own = spec
            .dialects
            .is_some_and(|d| d.intersects(vendor) && !d.intersects(DialectSet::ALL_TCL));
        if !vendor_own {
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
/// is admitted when its surface package is active in this context.
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
            Family::Jim => {}
        }
    }
    for (bit, package) in VENDOR_BITS {
        if context.package_active(package) {
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
        // The analyser resolved both `tcl` (and every unknown name) and
        // the set-only `tk` ingress to the permissive fallback profile;
        // their environments derive the same permissive facts.
        let plain = DialectProfile::plain_tcl();
        for lenient in ["tcl", "tk"] {
            let ctx = context(lenient);
            assert_eq!(ctx.authoring_mask(), plain.availability_mask, "{lenient}");
            assert_eq!(ctx.tcl_version_ceiling(), None, "{lenient}");
            assert!(ctx.operator_heads_are_commands(), "{lenient}");
            assert_eq!(ctx.vendor_authoring_bit(), None, "{lenient}");
        }
    }

    /// **P1-F parity sweep 2**: the spec/subcommand/option availability
    /// queries answer exactly as the old `ProfileQueries` for every spec
    /// in the compiled universe under every catalogue profile — commands,
    /// each subcommand, each sub-subcommand (at no version and at a pinned
    /// one), and every option at its inherited parent gate.
    #[test]
    fn spec_queries_reproduce_profile_queries_for_every_profile() {
        use crate::profile_queries::ProfileQueries;
        use tcl_dialect::DialectProfile;
        let universe = crate::model::assembly::universe();
        let mut checks = 0usize;
        for profile in DialectProfile::all() {
            let ctx = context(profile.name);
            for name in universe.command_names() {
                for spec in universe.specs(name) {
                    assert_eq!(
                        ctx.spec_available(spec),
                        profile.is_available(spec),
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
