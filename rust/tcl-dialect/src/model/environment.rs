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

//! The environment layer of the registry redesign (design doc
//! `docs/design/dialect-and-package-registry-redesign.md` §3.3): the
//! named, selectable definitions of what a project works against, the
//! overlay mechanism that adjusts them without mutation, and the one
//! resolver every user-facing ingress goes through.
//!
//! An [`EnvironmentDefinition`] is a core-profile selector plus per-axis
//! version-set targets, expected/ambient package placements, server-side
//! detection facts, policy defaults, and a reference to a *fixed,
//! contributed* editor language identity (review B7 — a server can never
//! mint a new editor language id). Environments are dynamic data:
//! `Arc`-held, equality by `(id, generation, overlay hash)`, never
//! interned statics with pointer identity. Workspace/user adjustments are
//! [`EnvironmentOverlay`]s whose content hash and origin are part of the
//! resolved identity — the canonical definition is never redefined in
//! place (review H1).
//!
//! The collision contract (§3.3): compiled canonical names are reserved,
//! alias cycles are unrepresentable (an alias may never equal any
//! canonical id, which is the only way a flat alias table could cycle),
//! and same-precedence collisions are typed construction errors, not
//! nearest-wins picks.
//!
//! [`EnvironmentRegistry::resolve`] replaces the divergent validators of
//! the old model (`available_dialects`, `is_known_dialect_name`, the
//! directive's `KNOWN_DIALECTS` match, `resolve_known`) — every name a
//! user can write today keeps resolving, as data, not as a shim.

use std::collections::HashMap;
use std::sync::Arc;

use crate::model::family::{BuildProfileId, Family, Release};
use crate::model::version_set::{Version, VersionAxisId, VersionSet, VersionSetError};

/// The interned canonical id of one environment (`"tcl8.6"`,
/// `"f5-irules"`, or a namespaced third-party id such as
/// `"spicegentcl/ngspice"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvironmentId(Arc<str>);

impl EnvironmentId {
    /// An id from its canonical spelling.
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(Arc::from(id))
    }

    /// The canonical spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EnvironmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A member of the FIXED, contributed editor language identity set
/// (review B7): the language ids the shipped editor extensions actually
/// contribute, seeded from `editors/vscode/src/languageIds.ts`'s
/// `TCL_LANGUAGE_IDS` block. Dynamic server environments *select among*
/// these; they can never mint a new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EditorLanguageIdentityId(&'static str);

impl EditorLanguageIdentityId {
    /// The contributed set, verbatim from the generated
    /// `TCL_LANGUAGE_IDS` block.
    pub const CONTRIBUTED: &'static [&'static str] = &[
        "tcl",
        "tcl-cadence",
        "tcl-expect",
        "tcl-bigip",
        "tcl-iapp",
        "tcl-irule",
        "tcl-tmsh",
        "tcl-quartus",
        "tcl-mentor",
        "tcl-microchip",
        "tclspec",
        "tcl-synopsys",
        "tcl84",
        "tcl85",
        "tcl86",
        "tcl90",
        "tcl91",
        "tcl-xilinx",
        "tcl-apl",
    ];

    /// The identity for `id`, or `None` when no editor contributes it —
    /// the constructor is the whole enforcement of B7.
    #[must_use]
    pub fn new(id: &str) -> Option<Self> {
        Self::CONTRIBUTED
            .iter()
            .find(|&&contributed| contributed == id)
            .map(|&contributed| Self(contributed))
    }

    /// The contributed language id string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// The core an environment selects: a family, the release used when the
/// document states nothing narrower, and the build profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreProfileSelector {
    /// The core family.
    pub family: Family,
    /// The default release on that family's ladder.
    pub default_release: Release,
    /// The build profile (review B1).
    pub build: BuildProfileId,
}

/// An externally-keyed version axis a [`Placement::Keyed`] placement
/// resolves through — the platform-implied versions of the old
/// catalogue's `VersionKey`, restated in the new model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyedAxis {
    /// The BIG-IP TMOS release (F5 iRules / iApps / tmsh / config
    /// schema).
    BigipVersion,
    /// The EDA tool release (Vivado, Quartus, Design Compiler, …).
    ToolVersion,
    /// The SDC (Synopsys Design Constraints) standard revision.
    SdcVersion,
    /// The UPF (IEEE 1801) standard revision.
    UpfVersion,
}

/// How an expected package's version is determined (§3.2's placement
/// claims).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// A fixed platform version (Expect `5.45.4`).
    Pinned(Version),
    /// The version follows the environment's core release. Survives only
    /// for hosts that genuinely guarantee matched versions (review B11 —
    /// never the default for Tk).
    TracksBase,
    /// Resolved through an external key (the BIG-IP release, the EDA
    /// tool release).
    Keyed(KeyedAxis),
    /// Floored by a requirement set on the package's **own** axis.
    Requirement(VersionSet),
}

/// One package an environment expects, with its placement and whether it
/// is ambient (present with no `package require` — the F5 surfaces, an
/// EDA shell's tool commands) or hosted (installable, requiring its
/// `package require`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagePlacement {
    /// The package name as spec data spells it.
    pub package: Arc<str>,
    /// How the version is determined.
    pub version: Placement,
    /// Ambient (no require needed) vs hosted.
    pub ambient: bool,
}

/// Resolution strictness (§5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldPolicy {
    /// Hosted packs resolve everywhere; W120 stays advisory (plain Tcl,
    /// the EDA shells).
    Open,
    /// Only the ambient closure exists; `package require` is not part of
    /// the language (`f5-irules`).
    Closed,
    /// The ambient surface plus explicitly required packages;
    /// hosted-but-unrequired packs are excluded (`f5-iapps`, `f5-tmsh`).
    AmbientPlusRequire,
}

/// An environment's policy defaults (§3.3) — the last profile
/// stragglers, absorbed as policy: closed-world resolution, fixed
/// ensembles, the iApps W108 strict-ASCII rule, and the version ceiling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentPolicy {
    /// Resolution strictness.
    pub closed_world: WorldPolicy,
    /// Whether ensembles ship a closed subcommand set with no
    /// user-extensible ensembles (the F5 family), so minifier prefix
    /// shortening is safe.
    pub fixed_ensembles: bool,
    /// The iApps W108 strict-ASCII rule.
    pub strict_ascii: bool,
    /// Upper-bound release for option gating, when the environment names
    /// one.
    pub version_ceiling: Option<Release>,
}

/// One filename extension an environment claims, with its human-facing
/// name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileExtensionClaim {
    /// Lower-case extension without the leading dot (`"xdc"`).
    pub extension: Arc<str>,
    /// What the file type is called (`"Xilinx Design Constraints"`).
    pub display_name: Arc<str>,
}

/// The server-side detection facts of one environment (§5.1's chain
/// reads these).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DetectionFacts {
    /// Extensions the environment owns.
    pub file_extensions: Vec<FileExtensionClaim>,
    /// Whole basenames the environment owns (`bigip.conf`).
    pub filenames: Vec<Arc<str>>,
    /// Content signatures selecting the environment.
    pub content_signatures: Vec<Arc<str>>,
    /// Shebang interpreter words selecting it (`wish`, `tclsh8.5`).
    pub shebang_words: Vec<Arc<str>>,
    /// Extra `# tcl-dialect:` directive spellings beyond the canonical
    /// id and aliases (which always resolve).
    pub directive_names: Vec<Arc<str>>,
}

/// Where a definition (or overlay) came from — its trust class (§6.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provenance {
    /// Compiled into the binary.
    BuiltIn,
    /// A pack bundled and signed with the distribution.
    BundledPack,
    /// User-level configuration or packs.
    User,
    /// A workspace the editor trusts.
    WorkspaceTrusted,
    /// A workspace the editor does not trust — additions may improve
    /// assistance, never weaken shipped analysis facts.
    WorkspaceUntrusted,
    /// A live Spec Studio override.
    StudioOverride,
    /// The document under analysis declared this itself — an inline
    /// `# tcl-lsp: stub` block (gap ruling R1). The lowest trust class
    /// there is: it is scoped to one buffer, it may improve assistance
    /// inside that buffer, and it can never weaken a shipped analysis
    /// fact or reach another document.
    Document,
}

/// One environment definition (§3.3) — dynamic data, held behind `Arc`,
/// identified by `(id, generation, overlay hash)`, never by pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentDefinition {
    /// Canonical, reserved or namespaced id — see the collision
    /// contract.
    pub id: EnvironmentId,
    /// Retired and legacy spellings that resolve to this environment.
    pub aliases: Vec<Arc<str>>,
    /// The human-facing name.
    pub display_name: Arc<str>,
    /// The contributed editor identity this environment's documents open
    /// under, when one is dedicated (review B7).
    pub editor_identity: Option<EditorLanguageIdentityId>,
    /// The core selector — `None` only for an identity-only environment
    /// that routes outside the Tcl language pipeline entirely
    /// (`f5-bigip`, which keeps its detection identity while leaving the
    /// Tcl axis, per the §2 table and Q3).
    pub core: Option<CoreProfileSelector>,
    /// The declared target set (§5.4); a single release line by default.
    pub targets: VersionSet,
    /// Expected/ambient packages at platform-implied versions.
    pub expected_packages: Vec<PackagePlacement>,
    /// Policy defaults.
    pub policy_defaults: EnvironmentPolicy,
    /// Server-side detection facts.
    pub server_detection: DetectionFacts,
    /// Lower-case help-index filter terms.
    pub help_terms: Vec<Arc<str>>,
    /// Trust class (§6.4).
    pub provenance: Provenance,
}

impl EnvironmentDefinition {
    /// The environment's resolved [`DialectPoint`](crate::model::DialectPoint)
    /// — its core's default release under the core's build — when it has a
    /// ladder core. `None` for a ladder-less environment (`f5-bigip`, the
    /// BIG-IP *config* surface). This is the one derivation of a point from
    /// an environment; the ingress, the semantic handle's runtime base and
    /// the projected profile all read it, so they cannot disagree about
    /// which release an environment is.
    #[must_use]
    pub fn point(&self) -> Option<crate::model::DialectPoint> {
        self.core
            .map(|core| crate::model::DialectPoint::new(core.default_release, core.build))
    }
}

/// Target adjustments an overlay applies.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TargetChanges {
    /// Replacement target set, when the overlay narrows or widens the
    /// base's; must live on the base's axis.
    pub targets: Option<VersionSet>,
}

/// Package adjustments an overlay applies.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageChanges {
    /// Placements added (or replacing same-named base placements).
    pub add: Vec<PackagePlacement>,
    /// Package names removed from the base's expectations.
    pub remove: Vec<Arc<str>>,
}

/// Where an overlay came from: its trust class plus the content hash
/// that becomes part of the resolved identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConfigurationOrigin {
    /// The overlay's trust class.
    pub provenance: Provenance,
    /// A hash of the overlay's source content.
    pub content_hash: u64,
}

/// A workspace/user adjustment to a named environment (review H1): the
/// canonical definition is never redefined in place — the overlay
/// derives a new value whose origin hash is part of the identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentOverlay {
    /// The environment being adjusted.
    pub base: EnvironmentId,
    /// Target adjustments.
    pub target_changes: TargetChanges,
    /// Package adjustments.
    pub package_changes: PackageChanges,
    /// Hash + origin — part of the resolved identity.
    pub origin: ConfigurationOrigin,
}

/// The resolved identity of an environment value: id, registry
/// generation, and the overlay hash when one applied — the key the salsa
/// layer caches on (§3.3).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvironmentIdentity {
    /// The canonical id.
    pub id: EnvironmentId,
    /// The registry generation the value came from.
    pub generation: u64,
    /// The overlay content hash, when an overlay applied.
    pub overlay: Option<u64>,
}

/// A typed construction diagnostic from the §3.3 collision contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentRegistryError {
    /// Two definitions claim one canonical id.
    DuplicateCanonicalId(String),
    /// An alias equals a canonical id — the shape that would let alias
    /// chains cycle, so it is rejected outright.
    AliasShadowsCanonical {
        /// The offending alias.
        alias: String,
        /// The canonical id it shadows.
        canonical: String,
    },
    /// Two definitions claim one alias (a same-precedence collision).
    DuplicateAlias(String),
    /// Two definitions select one editor identity (a same-precedence
    /// collision).
    DuplicateEditorIdentity(String),
    /// A non-built-in definition claims a compiled (reserved) name.
    ReservedName {
        /// The reserved spelling.
        name: String,
        /// The canonical id of the offending definition.
        claimed_by: String,
    },
}

impl std::fmt::Display for EnvironmentRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateCanonicalId(id) => {
                write!(f, "two environments claim the canonical id `{id}`")
            }
            Self::AliasShadowsCanonical { alias, canonical } => {
                write!(f, "alias `{alias}` shadows the canonical id `{canonical}`")
            }
            Self::DuplicateAlias(alias) => {
                write!(f, "two environments claim the alias `{alias}`")
            }
            Self::DuplicateEditorIdentity(id) => {
                write!(f, "two environments select the editor identity `{id}`")
            }
            Self::ReservedName { name, claimed_by } => {
                write!(
                    f,
                    "`{claimed_by}` claims the compiled reserved name `{name}`"
                )
            }
        }
    }
}

impl std::error::Error for EnvironmentRegistryError {}

/// A typed error from overlay application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentOverlayError {
    /// The overlay names an environment the registry does not hold.
    UnknownBase(EnvironmentId),
    /// The overlay's replacement targets live on a different axis than
    /// the base's (invariant I2).
    Targets(VersionSetError),
}

impl std::fmt::Display for EnvironmentOverlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownBase(id) => write!(f, "overlay base `{id}` is not a known environment"),
            Self::Targets(err) => write!(f, "overlay targets rejected: {err}"),
        }
    }
}

impl std::error::Error for EnvironmentOverlayError {}

/// The environment registry: `Arc`-held values with a generation, one
/// resolver over canonical ids + aliases + editor identities (§3.3,
/// centralisation contract R-a).
#[derive(Debug, Clone)]
pub struct EnvironmentRegistry {
    generation: u64,
    definitions: Vec<Arc<EnvironmentDefinition>>,
    index: HashMap<Arc<str>, usize>,
}

impl EnvironmentRegistry {
    /// The compiled registry: the core seed set at generation 0.
    ///
    /// # Panics
    /// Never in practice — the compiled seed set is collision-free by
    /// test.
    #[must_use]
    pub fn compiled() -> Self {
        Self::new(compiled_definitions(), 0).expect("the compiled catalogue is collision-free")
    }

    /// A registry over `definitions` at `generation`, enforcing the
    /// collision contract.
    ///
    /// # Errors
    /// A typed [`EnvironmentRegistryError`] naming the first collision:
    /// duplicate canonical ids, an alias shadowing any canonical id (the
    /// only shape a flat alias table could cycle through), duplicate
    /// aliases, duplicate editor identities, or a non-built-in
    /// definition claiming a compiled reserved name.
    pub fn new(
        definitions: Vec<EnvironmentDefinition>,
        generation: u64,
    ) -> Result<Self, EnvironmentRegistryError> {
        check_reserved(&definitions)?;
        let definitions: Vec<Arc<EnvironmentDefinition>> =
            definitions.into_iter().map(Arc::new).collect();
        let index = build_index(&definitions)?;
        Ok(Self {
            generation,
            definitions,
            index,
        })
    }

    /// The registry's generation.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Every definition, in registration order.
    #[must_use]
    pub fn definitions(&self) -> &[Arc<EnvironmentDefinition>] {
        &self.definitions
    }

    /// Resolve any user-written name — canonical id, alias, or editor
    /// language id — to its environment. The one ingress function
    /// (centralisation contract R-a); precedence between the three tiers
    /// is canonical > alias > editor identity, fixed at construction.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<Arc<EnvironmentDefinition>> {
        self.index
            .get(name)
            .map(|&position| Arc::clone(&self.definitions[position]))
    }

    /// The `(id, generation, overlay)` identity of a definition resolved
    /// from this registry with no overlay applied.
    #[must_use]
    pub fn identity_of(&self, definition: &EnvironmentDefinition) -> EnvironmentIdentity {
        EnvironmentIdentity {
            id: definition.id.clone(),
            generation: self.generation,
            overlay: None,
        }
    }

    /// Apply `overlay` to its base, deriving a new value — the base is
    /// never mutated — and the identity carrying the overlay hash.
    ///
    /// # Errors
    /// [`EnvironmentOverlayError::UnknownBase`] when the base is not in
    /// this registry; [`EnvironmentOverlayError::Targets`] when the
    /// replacement targets sit on a different axis than the base's.
    pub fn apply_overlay(
        &self,
        overlay: &EnvironmentOverlay,
    ) -> Result<(Arc<EnvironmentDefinition>, EnvironmentIdentity), EnvironmentOverlayError> {
        let base = self
            .resolve(overlay.base.as_str())
            .ok_or_else(|| EnvironmentOverlayError::UnknownBase(overlay.base.clone()))?;
        let mut derived = (*base).clone();
        if let Some(targets) = &overlay.target_changes.targets {
            if targets.axis() != derived.targets.axis() {
                return Err(EnvironmentOverlayError::Targets(
                    VersionSetError::AxisMismatch {
                        left: derived.targets.axis().clone(),
                        right: targets.axis().clone(),
                    },
                ));
            }
            derived.targets = targets.clone();
        }
        derived.expected_packages.retain(|placement| {
            !overlay
                .package_changes
                .remove
                .iter()
                .any(|removed| **removed == *placement.package)
        });
        derived.expected_packages.retain(|placement| {
            !overlay
                .package_changes
                .add
                .iter()
                .any(|added| added.package == placement.package)
        });
        derived
            .expected_packages
            .extend(overlay.package_changes.add.iter().cloned());
        derived.provenance = overlay.origin.provenance;
        let identity = EnvironmentIdentity {
            id: base.id.clone(),
            generation: self.generation,
            overlay: Some(overlay.origin.content_hash),
        };
        Ok((Arc::new(derived), identity))
    }
}

/// Reject non-built-in definitions claiming compiled reserved names
/// (§3.3: **all** compiled canonical names are reserved, and so are the
/// compiled aliases; editor identities are selectable by anyone — that
/// is their B7 purpose).
fn check_reserved(definitions: &[EnvironmentDefinition]) -> Result<(), EnvironmentRegistryError> {
    let reserved: Vec<String> = compiled_definitions()
        .iter()
        .flat_map(|definition| {
            std::iter::once(definition.id.as_str().to_owned()).chain(
                definition
                    .aliases
                    .iter()
                    .map(|alias| alias.as_ref().to_owned()),
            )
        })
        .collect();
    for definition in definitions {
        if matches!(definition.provenance, Provenance::BuiltIn) {
            continue;
        }
        let claimed = std::iter::once(definition.id.as_str())
            .chain(definition.aliases.iter().map(AsRef::as_ref));
        for name in claimed {
            if reserved.iter().any(|reserved| reserved == name) {
                return Err(EnvironmentRegistryError::ReservedName {
                    name: name.to_owned(),
                    claimed_by: definition.id.as_str().to_owned(),
                });
            }
        }
    }
    Ok(())
}

/// Build the three-tier name index: canonical ids, then aliases, then
/// editor identities. Within a tier a collision is a typed error; across
/// tiers the higher tier wins, fixed here at construction.
fn build_index(
    definitions: &[Arc<EnvironmentDefinition>],
) -> Result<HashMap<Arc<str>, usize>, EnvironmentRegistryError> {
    let mut index: HashMap<Arc<str>, usize> = HashMap::new();
    for (position, definition) in definitions.iter().enumerate() {
        let id: Arc<str> = Arc::from(definition.id.as_str());
        if index.insert(Arc::clone(&id), position).is_some() {
            return Err(EnvironmentRegistryError::DuplicateCanonicalId(
                id.as_ref().to_owned(),
            ));
        }
    }
    for (position, definition) in definitions.iter().enumerate() {
        for alias in &definition.aliases {
            if let Some(&existing) = index.get(alias.as_ref()) {
                let existing = &definitions[existing];
                if existing.id.as_str() == alias.as_ref() {
                    return Err(EnvironmentRegistryError::AliasShadowsCanonical {
                        alias: alias.as_ref().to_owned(),
                        canonical: existing.id.as_str().to_owned(),
                    });
                }
                return Err(EnvironmentRegistryError::DuplicateAlias(
                    alias.as_ref().to_owned(),
                ));
            }
            index.insert(Arc::clone(alias), position);
        }
    }
    let mut editor_claims: HashMap<&'static str, usize> = HashMap::new();
    for (position, definition) in definitions.iter().enumerate() {
        let Some(identity) = definition.editor_identity else {
            continue;
        };
        if editor_claims.insert(identity.as_str(), position).is_some() {
            return Err(EnvironmentRegistryError::DuplicateEditorIdentity(
                identity.as_str().to_owned(),
            ));
        }
        // A higher tier already owns the spelling (e.g. an environment
        // whose alias doubles as its editor id): the ladder stands.
        index
            .entry(Arc::from(identity.as_str()))
            .or_insert(position);
    }
    Ok(index)
}

// --- the compiled seed set -------------------------------------------

fn arc(text: &str) -> Arc<str> {
    Arc::from(text)
}

fn arcs(items: &[&str]) -> Vec<Arc<str>> {
    items.iter().map(|&item| arc(item)).collect()
}

fn ver(text: &str) -> Version {
    Version::parse(text).expect("compiled version literal")
}

fn reqs(axis: VersionAxisId, requirements: &[&str]) -> VersionSet {
    VersionSet::from_requirements(axis, requirements).expect("compiled requirement literal")
}

fn ext(extension: &str, display_name: &str) -> FileExtensionClaim {
    FileExtensionClaim {
        extension: arc(extension),
        display_name: arc(display_name),
    }
}

fn keyed(package: &str, axis: KeyedAxis) -> PackagePlacement {
    PackagePlacement {
        package: arc(package),
        version: Placement::Keyed(axis),
        ambient: true,
    }
}

fn hosted_pin(package: &str, version: &str) -> PackagePlacement {
    PackagePlacement {
        package: arc(package),
        version: Placement::Pinned(ver(version)),
        ambient: false,
    }
}

/// The single-release-line target of a Tcl ladder release: `[R·a0,
/// next-minor·a0)` — the release line itself, not a floor.
fn tcl_line(release: Release) -> VersionSet {
    let requirement = match release {
        Release::TCL_8_4 => "8.4-8.5",
        Release::TCL_8_5 => "8.5-8.6",
        Release::TCL_8_6 => "8.6-8.7",
        Release::TCL_9_0 => "9.0-9.1",
        _ => "9.1-9.2",
    };
    reqs(VersionAxisId::core(Family::Tcl), &[requirement])
}

/// The full Tcl ladder, for the lenient environments.
fn tcl_full_ladder() -> VersionSet {
    reqs(VersionAxisId::core(Family::Tcl), &["8.4-9.2"])
}

fn open_policy(version_ceiling: Option<Release>) -> EnvironmentPolicy {
    EnvironmentPolicy {
        closed_world: WorldPolicy::Open,
        fixed_ensembles: false,
        strict_ascii: false,
        version_ceiling,
    }
}

fn tcl_core(default_release: Release) -> CoreProfileSelector {
    CoreProfileSelector {
        family: Family::Tcl,
        default_release,
        build: BuildProfileId::Canonical,
    }
}

/// The five core-ladder environments (`tcl8.4` … `tcl9.1`) — the flat
/// per-release names stay the generated, stable spellings (Q4).
fn ladder_environments() -> Vec<EnvironmentDefinition> {
    [
        (Release::TCL_8_4, "tcl84", "3.4"),
        (Release::TCL_8_5, "tcl85", "3.4"),
        (Release::TCL_8_6, "tcl86", "4.2"),
        (Release::TCL_9_0, "tcl90", "4.2"),
        (Release::TCL_9_1, "tcl91", "4.2"),
    ]
    .into_iter()
    .map(|(release, editor_id, itcl)| EnvironmentDefinition {
        id: EnvironmentId::new(&format!("tcl{}", release.as_str())),
        aliases: Vec::new(),
        display_name: arc(&format!("Tcl {release}")),
        editor_identity: EditorLanguageIdentityId::new(editor_id),
        core: Some(tcl_core(release)),
        targets: tcl_line(release),
        expected_packages: vec![
            // Tk is **hosted** here: a `tclsh8.6` document must
            // `package require Tk` (W120 nags when it does not), and the
            // floor rides Tk's **own** package axis — never the Tcl core
            // axis (B11, invariant I2). `TracksBase` is B11's one named
            // exemption ("unless a specific host environment truly
            // guarantees matched versions"): a *release-pinned* Tcl
            // environment is exactly that host — the 8.6 distribution
            // ships Tk 8.6 — so the point on the Tk axis is derived from
            // the pinned core release. The unpinned environments (`tcl`,
            // `tk`) claim no such guarantee and carry a bare requirement
            // instead.
            PackagePlacement {
                package: arc("Tk"),
                version: Placement::TracksBase,
                ambient: false,
            },
            hosted_pin("Itcl", itcl),
        ],
        policy_defaults: open_policy(Some(release)),
        server_detection: DetectionFacts {
            shebang_words: vec![arc(&format!("tclsh{release}"))],
            ..DetectionFacts::default()
        },
        help_terms: arcs(&["tcl", "tk"]),
        provenance: Provenance::BuiltIn,
    })
    .collect()
}

/// The plain-`tcl` fallback: the full-ladder lenient environment every
/// unversioned document lands on.
fn plain_tcl_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("tcl"),
        aliases: Vec::new(),
        display_name: arc("Tcl"),
        editor_identity: EditorLanguageIdentityId::new("tcl"),
        core: Some(tcl_core(Release::TCL_9_0)),
        targets: tcl_full_ladder(),
        // P3: the lenient sink declares the same **hosted** Tk placement
        // the ladder rows carry, so "can this environment host Tk?" is a
        // placement query everywhere instead of a lenient special case.
        // No release is implied — an unversioned document names no Tcl
        // release either — so Tk sits on a requirement over its own axis
        // (B11), which is also why this row grants no floor.
        expected_packages: vec![PackagePlacement {
            package: arc("Tk"),
            version: Placement::Requirement(reqs(VersionAxisId::package("Tk"), &["8.4-"])),
            ambient: false,
        }],
        policy_defaults: open_policy(None),
        server_detection: DetectionFacts {
            // The generic Tcl source extensions the editors register for
            // the `tcl` language id (`editors/vscode/src/languageIds.ts`).
            file_extensions: vec![
                ext("tcl", "Tcl Script"),
                ext("tk", "Tcl/Tk Script"),
                ext("itcl", "Incr Tcl Script"),
                ext("tm", "Tcl Module"),
                ext("test", "Tcl Test Script"),
            ],
            shebang_words: arcs(&["tclsh"]),
            ..DetectionFacts::default()
        },
        help_terms: Vec::new(),
        provenance: Provenance::BuiltIn,
    }
}

/// The `tk` environment (alias `wish`): tcl at base + Tk **ambient** on
/// Tk's **own** version axis — never `tracks-base` (review B11). Erases
/// the tk triangle.
///
/// **P3 (the Tk pilot).** The placement is `ambient` because that is what
/// a `wish` document *is*: the interpreter has already loaded Tk before
/// the first byte of the script runs, so there is no `package require Tk`
/// to write and none to nag about. Everything the old triangle spelled
/// three ways now falls out of this one row — `package_active("Tk")`,
/// the context's `TK` authoring bit, the Tk-checks activation fact, and
/// W120's silence (ledger F4). The version stays a **requirement** on
/// `Tk`'s own axis rather than a point: `wish` reports its own Tk
/// patchlevel, which the document text does not carry, so the honest
/// answer is "some Tk ≥ 8.4" and the permissive no-primary rule applies.
fn tk_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("tk"),
        aliases: arcs(&["wish"]),
        display_name: arc("Tk"),
        editor_identity: None,
        core: Some(tcl_core(Release::TCL_8_6)),
        targets: tcl_full_ladder(),
        expected_packages: vec![
            PackagePlacement {
                package: arc("Tk"),
                version: Placement::Requirement(reqs(VersionAxisId::package("Tk"), &["8.4-"])),
                ambient: true,
            },
            hosted_pin("Itcl", "4.2"),
        ],
        policy_defaults: open_policy(None),
        server_detection: DetectionFacts {
            shebang_words: arcs(&["wish"]),
            ..DetectionFacts::default()
        },
        help_terms: arcs(&["tk"]),
        provenance: Provenance::BuiltIn,
    }
}

/// The `jim` environment (aliases `jimsh`, `jimtcl`) — **one** row for
/// the whole nine-release ladder, which is P6's headline collapse.
///
/// The old model needed nine `jim0.76`–`jim0.84` catalogue profiles for
/// one reason: a profile carries exactly one resolved `LexerGrammar`, so
/// a release that differed in a single axis needed its own row, and ten
/// user-facing surfaces each grew nine lines. Here the grammar is a
/// function of `(family, release, build)`
/// ([`crate::model::family::grammar`]), so the environment names the
/// family and the ladder, and a project picks its point on the ladder
/// with `# tcl-lsp: supports jim 0.81-0.84` — the §5.4 range machinery,
/// on the `jim` core axis, unchanged.
///
/// Three deliberate absences:
///
/// - **No editor identity.** Review B7: a server may select among the
///   identities the shipped extensions contribute and can never mint a
///   new one. No editor contributes a `tcl-jim` id today, so this
///   environment carries `None` exactly as `tk` and `bpf` do, and the
///   jim rows the branch added to ten user-facing catalogues are simply
///   not needed to make `# tcl-dialect: jim` resolve.
/// - **No release-pinned siblings.** `jim0.84` is not an environment
///   name; it is a target on this environment's axis.
/// - **No expected packages.** Jim's command surface rides its ancestry
///   edge from Tcl 8.6 ([`Family::ancestry`]) — inherit-then-override
///   rather than the 76 hand-re-authored core commands the branch paid
///   for. The override half is design **Q6**'s jim surface pack.
///
/// The targets span the whole ladder, so the core axis takes **no point
/// primary** and answers under §5.4's permissive no-primary rule —
/// exactly like the lenient `tcl` sink, and for the same reason: a
/// document that names no jim release should not be judged against one.
fn jim_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("jim"),
        aliases: arcs(&["jimsh", "jimtcl"]),
        display_name: arc("Jim Tcl"),
        editor_identity: None,
        core: Some(CoreProfileSelector {
            family: Family::Jim,
            default_release: Release::JIM_0_84,
            build: BuildProfileId::Canonical,
        }),
        targets: reqs(VersionAxisId::core(Family::Jim), &["0.76-0.85"]),
        expected_packages: Vec::new(),
        policy_defaults: open_policy(None),
        server_detection: DetectionFacts {
            shebang_words: arcs(&["jimsh"]),
            ..DetectionFacts::default()
        },
        help_terms: arcs(&["jim", "jimtcl", "jimsh"]),
        provenance: Provenance::BuiltIn,
    }
}

fn irules_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("f5-irules"),
        aliases: arcs(&["irules", "tcl-irule"]),
        display_name: arc("F5 iRules"),
        editor_identity: EditorLanguageIdentityId::new("tcl-irule"),
        core: Some(CoreProfileSelector {
            family: Family::F5Irules,
            default_release: Release::F5_IRULES_TMM,
            build: BuildProfileId::Canonical,
        }),
        targets: reqs(VersionAxisId::core(Family::F5Irules), &["0-"]),
        expected_packages: vec![keyed("f5-irules-cmds", KeyedAxis::BigipVersion)],
        policy_defaults: EnvironmentPolicy {
            closed_world: WorldPolicy::Closed,
            fixed_ensembles: true,
            strict_ascii: false,
            version_ceiling: Some(Release::TCL_8_4),
        },
        server_detection: DetectionFacts {
            file_extensions: vec![
                ext("irul", "F5 iRule"),
                ext("irule", "F5 iRule"),
                ext("irules", "F5 iRule"),
            ],
            ..DetectionFacts::default()
        },
        help_terms: arcs(&["irules", "irule", "f5", "big-ip", "tmm", "event"]),
        provenance: Provenance::BuiltIn,
    }
}

fn iapps_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("f5-iapps"),
        aliases: Vec::new(),
        display_name: arc("F5 iApps"),
        editor_identity: EditorLanguageIdentityId::new("tcl-iapp"),
        // CORRECTED by measurement
        // (`docs/design/bigip-irule-parser-measurements.md` §4a): the
        // 8.5 baseline hypothesis is falsified — `IAppImplementation`
        // reports patchlevel 8.4.6, fails every 8.5 discriminator, and
        // carries the full `f5-tcl` trunk grammar. The core rides the
        // trunk under the 32-bit `scriptd` build profile (`wordSize 4`,
        // measurements §4 — review B1's build axis).
        core: Some(CoreProfileSelector {
            family: Family::F5Tcl,
            default_release: Release::F5_TCL_TMOS,
            build: BuildProfileId::F5Scriptd32,
        }),
        targets: reqs(VersionAxisId::core(Family::F5Tcl), &["0-"]),
        expected_packages: vec![keyed("f5-iapps-cmds", KeyedAxis::BigipVersion)],
        policy_defaults: EnvironmentPolicy {
            closed_world: WorldPolicy::AmbientPlusRequire,
            fixed_ensembles: true,
            // The W108 strict-ASCII rule, formerly keyed on the IAPPS
            // vendor bit.
            strict_ascii: true,
            // The fork point caps Tcl-versioned surface claims: the
            // embedded core is 8.4.6, and all sixteen measured 8.4/8.5
            // discriminators behave as 8.4 (measurements §4).
            version_ceiling: Some(Release::TCL_8_4),
        },
        server_detection: DetectionFacts {
            file_extensions: vec![
                ext("iapp", "F5 iApp Template"),
                ext("iappimpl", "F5 iApp Implementation"),
                ext("impl", "F5 iApp Implementation"),
            ],
            ..DetectionFacts::default()
        },
        help_terms: arcs(&["iapps", "iapp", "f5", "big-ip"]),
        provenance: Provenance::BuiltIn,
    }
}

fn tmsh_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("f5-tmsh"),
        aliases: Vec::new(),
        display_name: arc("F5 tmsh Scripts"),
        editor_identity: EditorLanguageIdentityId::new("tcl-tmsh"),
        // CORRECTED by measurement
        // (`docs/design/bigip-irule-parser-measurements.md` §4a): the
        // 8.5/8.5.13 claims are falsified — `TmshCliScript` reports
        // 8.4.6 and reproduces the entire trunk grammar (R-rules,
        // N-rules, inert `{*}`, word operators) identically to TMM. The
        // core rides the `f5-tcl` trunk at its canonical build; the
        // environment deltas (working `exec`, empty `tcl_platform`, no
        // `tcl_patchLevel`, `info vartype`) are host facts, not grammar.
        core: Some(CoreProfileSelector {
            family: Family::F5Tcl,
            default_release: Release::F5_TCL_TMOS,
            build: BuildProfileId::Canonical,
        }),
        targets: reqs(VersionAxisId::core(Family::F5Tcl), &["0-"]),
        expected_packages: vec![keyed("f5-tmsh-cmds", KeyedAxis::BigipVersion)],
        policy_defaults: EnvironmentPolicy {
            closed_world: WorldPolicy::AmbientPlusRequire,
            fixed_ensembles: false,
            strict_ascii: false,
            // The fork point caps Tcl-versioned surface claims
            // (measurements §4 — every 8.5 discriminator behaves as 8.4).
            version_ceiling: Some(Release::TCL_8_4),
        },
        server_detection: DetectionFacts {
            file_extensions: vec![ext("tmsh", "F5 tmsh Script")],
            ..DetectionFacts::default()
        },
        help_terms: arcs(&["tmsh", "f5", "big-ip", "bigip"]),
        provenance: Provenance::BuiltIn,
    }
}

/// `f5-bigip` keeps its detection identity but leaves the Tcl dialect
/// axis entirely (Q3): no core selector, no Tcl surface — identity and
/// keyed schema only.
fn bigip_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("f5-bigip"),
        aliases: Vec::new(),
        display_name: arc("F5 BIG-IP"),
        editor_identity: EditorLanguageIdentityId::new("tcl-bigip"),
        core: None,
        targets: reqs(VersionAxisId::package("f5-bigip-schema"), &["0-"]),
        expected_packages: vec![keyed("f5-bigip-schema", KeyedAxis::BigipVersion)],
        policy_defaults: EnvironmentPolicy {
            closed_world: WorldPolicy::Closed,
            fixed_ensembles: true,
            strict_ascii: false,
            version_ceiling: None,
        },
        server_detection: DetectionFacts {
            file_extensions: vec![ext("scf", "BIG-IP Single Configuration File")],
            filenames: arcs(&[
                "bigip.conf",
                "bigip_base.conf",
                "bigip_gtm.conf",
                "bigip_script.conf",
                "bigip_user.conf",
            ]),
            ..DetectionFacts::default()
        },
        help_terms: arcs(&["bigip", "big-ip", "bigip.conf", "f5", "ltm", "gtm"]),
        provenance: Provenance::BuiltIn,
    }
}

fn expect_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("expect"),
        aliases: Vec::new(),
        display_name: arc("Expect"),
        editor_identity: EditorLanguageIdentityId::new("tcl-expect"),
        core: Some(tcl_core(Release::TCL_8_6)),
        targets: tcl_line(Release::TCL_8_6),
        expected_packages: vec![PackagePlacement {
            package: arc("Expect"),
            version: Placement::Pinned(ver("5.45.4")),
            ambient: true,
        }],
        policy_defaults: open_policy(Some(Release::TCL_8_6)),
        server_detection: DetectionFacts {
            file_extensions: vec![ext("exp", "Expect Script"), ext("expect", "Expect Script")],
            shebang_words: arcs(&["expect"]),
            ..DetectionFacts::default()
        },
        help_terms: arcs(&["expect", "spawn", "interact"]),
        provenance: Provenance::BuiltIn,
    }
}

fn spectcl_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("spectcl"),
        aliases: arcs(&["tcl-spec", "tclspec"]),
        display_name: arc("SpecTcl"),
        editor_identity: EditorLanguageIdentityId::new("tclspec"),
        core: Some(tcl_core(Release::TCL_9_0)),
        targets: tcl_line(Release::TCL_9_0),
        expected_packages: Vec::new(),
        policy_defaults: EnvironmentPolicy {
            // A pack is declarative and its hook bodies run on our own
            // sandboxed VM: nothing is `package require`-able into it.
            closed_world: WorldPolicy::Closed,
            fixed_ensembles: false,
            strict_ascii: false,
            version_ceiling: Some(Release::TCL_9_0),
        },
        server_detection: DetectionFacts {
            file_extensions: vec![ext("tclspec", "SpecTcl Command Pack")],
            ..DetectionFacts::default()
        },
        help_terms: arcs(&["spectcl", "speclib", "tclspec"]),
        provenance: Provenance::BuiltIn,
    }
}

fn bpf_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("bpf"),
        aliases: Vec::new(),
        display_name: arc("BPF"),
        editor_identity: None,
        core: Some(tcl_core(Release::TCL_9_0)),
        targets: tcl_line(Release::TCL_9_0),
        // The bpf command surface rides provider declarations in its own
        // phase; the environment seeds identity and policy only.
        expected_packages: Vec::new(),
        policy_defaults: EnvironmentPolicy {
            closed_world: WorldPolicy::Closed,
            fixed_ensembles: false,
            strict_ascii: false,
            version_ceiling: Some(Release::TCL_9_0),
        },
        server_detection: DetectionFacts::default(),
        help_terms: arcs(&["bpf", "ebpf"]),
        provenance: Provenance::BuiltIn,
    }
}

/// The six EDA shells: a base release plus keyed tool placements. These
/// move into `specs/eda_*.tclspec` environment blocks in a later phase
/// (Q2); they seed compiled for now so every current name keeps
/// resolving.
struct Eda {
    id: &'static str,
    display: &'static str,
    editor: &'static str,
    release: Release,
    extensions: &'static [(&'static str, &'static str)],
    tools: &'static [&'static str],
    help: &'static [&'static str],
}

const EDA_SHELLS: [Eda; 6] = [
    Eda {
        id: "cadence-eda-tcl",
        display: "Cadence EDA Tcl",
        editor: "tcl-cadence",
        release: Release::TCL_8_4,
        extensions: &[("globals", "Innovus/Genus Globals")],
        tools: &[
            "cadence-genus",
            "cadence-common",
            "cadence-innovus",
            "cadence-xcelium",
        ],
        help: &[
            "cadence",
            "genus",
            "innovus",
            "tempus",
            "xcelium",
            "encounter",
        ],
    },
    Eda {
        id: "intel-quartus-eda-tcl",
        display: "Intel Quartus EDA Tcl",
        editor: "tcl-quartus",
        release: Release::TCL_8_5,
        extensions: &[
            ("qsf", "Quartus Settings File"),
            ("qpf", "Quartus Project File"),
            ("qip", "Quartus IP File"),
        ],
        tools: &[
            "quartus-project",
            "quartus-flow",
            "quartus-sta",
            "quartus-sdc-ext",
            "quartus-report",
            "quartus-device",
            "quartus-misc",
        ],
        help: &["quartus", "intel", "altera", "fpga", "quartus_sh"],
    },
    Eda {
        id: "mentor-eda-tcl",
        display: "Mentor EDA Tcl",
        editor: "tcl-mentor",
        release: Release::TCL_8_6,
        extensions: &[("do", "ModelSim/Questa Do Script")],
        tools: &["questa", "questa-formal", "calibre"],
        help: &["mentor", "siemens", "modelsim", "questa", "calibre", "vsim"],
    },
    Eda {
        id: "microchip-libero-eda-tcl",
        display: "Microchip Libero EDA Tcl",
        editor: "tcl-microchip",
        release: Release::TCL_8_5,
        extensions: &[],
        tools: &["libero"],
        help: &[
            "microchip",
            "microsemi",
            "actel",
            "libero",
            "smartfusion",
            "igloo",
            "proasic",
        ],
    },
    Eda {
        id: "synopsys-eda-tcl",
        display: "Synopsys EDA Tcl",
        editor: "tcl-synopsys",
        release: Release::TCL_8_6,
        extensions: &[
            ("sdc", "Synopsys Design Constraints"),
            ("upf", "Unified Power Format"),
        ],
        tools: &[
            "synopsys-dc",
            "synopsys-pt",
            "synopsys-icc2",
            "synopsys-fm",
            "synopsys",
        ],
        help: &[
            "synopsys",
            "dc_shell",
            "design_compiler",
            "primetime",
            "icc2",
            "formality",
        ],
    },
    Eda {
        id: "xilinx-eda-tcl",
        display: "Xilinx EDA Tcl",
        editor: "tcl-xilinx",
        release: Release::TCL_8_5,
        extensions: &[("xdc", "Xilinx Design Constraints")],
        tools: &["vivado"],
        help: &["xilinx", "vivado", "vitis", "amd", "fpga", "ise"],
    },
];

fn eda_environments() -> Vec<EnvironmentDefinition> {
    EDA_SHELLS
        .into_iter()
        .map(|shell| {
            let mut placements = vec![
                keyed("sdc", KeyedAxis::SdcVersion),
                keyed("upf", KeyedAxis::UpfVersion),
            ];
            placements.extend(
                shell
                    .tools
                    .iter()
                    .map(|&tool| keyed(tool, KeyedAxis::ToolVersion)),
            );
            EnvironmentDefinition {
                id: EnvironmentId::new(shell.id),
                aliases: Vec::new(),
                display_name: arc(shell.display),
                editor_identity: EditorLanguageIdentityId::new(shell.editor),
                core: Some(tcl_core(shell.release)),
                targets: tcl_line(shell.release),
                expected_packages: placements,
                policy_defaults: open_policy(Some(shell.release)),
                server_detection: DetectionFacts {
                    file_extensions: shell
                        .extensions
                        .iter()
                        .map(|&(extension, display_name)| ext(extension, display_name))
                        .collect(),
                    ..DetectionFacts::default()
                },
                help_terms: arcs(shell.help),
                provenance: Provenance::BuiltIn,
            }
        })
        .collect()
}

/// The compiled environment definitions: every current
/// `DialectProfile` catalogue entry translated, plus the `tk` and
/// plain-`tcl` environments that erase the off-catalogue profiles.
#[must_use]
pub fn compiled_definitions() -> Vec<EnvironmentDefinition> {
    let mut definitions = Vec::new();
    definitions.push(plain_tcl_environment());
    definitions.extend(ladder_environments());
    definitions.push(tk_environment());
    definitions.push(jim_environment());
    definitions.push(irules_environment());
    definitions.push(iapps_environment());
    definitions.push(tmsh_environment());
    definitions.push(bigip_environment());
    definitions.push(expect_environment());
    definitions.push(spectcl_environment());
    definitions.push(bpf_environment());
    definitions.extend(eda_environments());
    definitions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DialectProfile;
    use crate::KNOWN_DIALECTS;

    #[test]
    fn every_old_name_and_alias_resolves() {
        let registry = EnvironmentRegistry::compiled();
        for &name in KNOWN_DIALECTS {
            assert!(registry.resolve(name).is_some(), "{name}");
        }
        for profile in DialectProfile::all() {
            let resolved = registry.resolve(profile.name).expect(profile.name);
            for &alias in profile.aliases {
                let via_alias = registry.resolve(alias).unwrap_or_else(|| {
                    panic!("alias `{alias}` of `{}` must resolve", profile.name)
                });
                assert_eq!(via_alias.id, resolved.id, "{alias}");
            }
        }
        // The two off-catalogue profiles become real environments.
        assert_eq!(registry.resolve("tk").expect("tk").id.as_str(), "tk");
        assert_eq!(registry.resolve("tcl").expect("tcl").id.as_str(), "tcl");
        assert_eq!(registry.resolve("wish").expect("wish").id.as_str(), "tk");
    }

    /// The single-resolver property (§7 gates): `resolve` accepts exactly
    /// {canonical ids} ∪ {aliases} ∪ {editor identities}, each input
    /// resolving deterministically to the one environment that declares
    /// it at the highest tier, and nothing else resolves.
    #[test]
    fn resolve_accepts_exactly_the_declared_names() {
        let registry = EnvironmentRegistry::compiled();
        let mut accepted: HashMap<String, String> = HashMap::new();
        for definition in registry.definitions() {
            let owner = definition.id.as_str().to_owned();
            accepted.insert(owner.clone(), owner.clone());
            for alias in &definition.aliases {
                accepted.insert(alias.as_ref().to_owned(), owner.clone());
            }
            if let Some(identity) = definition.editor_identity {
                accepted
                    .entry(identity.as_str().to_owned())
                    .or_insert_with(|| owner.clone());
            }
        }
        for (name, owner) in &accepted {
            let resolved = registry.resolve(name).unwrap_or_else(|| {
                panic!("declared name `{name}` must resolve");
            });
            assert_eq!(resolved.id.as_str(), owner, "{name}");
            // Deterministic: a second resolution gives the same value.
            assert_eq!(
                registry.resolve(name).expect("still resolves").id,
                resolved.id
            );
        }
        // Perturbations of every accepted name do not resolve, and
        // resolution is case-sensitive.
        for name in accepted.keys() {
            let padded = format!("{name}x");
            if !accepted.contains_key(&padded) {
                assert!(registry.resolve(&padded).is_none(), "{padded}");
            }
            let upper = name.to_uppercase();
            if upper != *name && !accepted.contains_key(&upper) {
                assert!(registry.resolve(&upper).is_none(), "{upper}");
            }
        }
        for unknown in ["", "nonsense", "tcl8.7", "jim0.85", "jim0.84"] {
            assert!(registry.resolve(unknown).is_none(), "{unknown}");
        }
    }

    /// **P6.** One `jim` environment for a nine-release ladder: the
    /// releases are targets on the family's own axis, not nine catalogue
    /// rows, and no editor identity is minted for it (review B7).
    #[test]
    fn one_jim_environment_covers_the_whole_ladder() {
        let registry = EnvironmentRegistry::compiled();
        let jim = registry.resolve("jim").expect("jim");
        for alias in ["jimsh", "jimtcl"] {
            assert_eq!(registry.resolve(alias).expect(alias).id, jim.id, "{alias}");
        }
        // Exactly one compiled environment names the jim family.
        let jim_rows: Vec<&str> = registry
            .definitions()
            .iter()
            .filter(|definition| {
                definition
                    .core
                    .is_some_and(|core| core.family == Family::Jim)
            })
            .map(|definition| definition.id.as_str())
            .collect();
        assert_eq!(jim_rows, ["jim"], "nine profiles became one environment");

        let core = jim.core.expect("core");
        assert_eq!(core.default_release, Release::JIM_0_84);
        assert_eq!(core.build, BuildProfileId::Canonical);
        assert_eq!(jim.targets.axis(), &VersionAxisId::core(Family::Jim));
        // The whole ladder, so no release is implied.
        for release in Family::Jim.releases() {
            let point = Version::parse(release.as_str()).expect("jim releases spell versions");
            assert!(jim.targets.contains(&point), "{release}");
        }
        assert!(
            jim.editor_identity.is_none(),
            "no editor contributes a jim language id (B7)"
        );
        assert!(
            jim.expected_packages.is_empty(),
            "the jim surface rides the ancestry edge, not a placement (Q6)"
        );
        assert_eq!(jim.policy_defaults.closed_world, WorldPolicy::Open);
        assert_eq!(
            jim.policy_defaults.version_ceiling, None,
            "the ceiling is a Tcl-ladder concept; jim has its own axis"
        );
    }

    #[test]
    fn seeded_policies_translate_the_catalogue() {
        let registry = EnvironmentRegistry::compiled();
        let irules = registry.resolve("f5-irules").expect("irules");
        assert_eq!(irules.policy_defaults.closed_world, WorldPolicy::Closed);
        assert!(irules.policy_defaults.fixed_ensembles);
        assert_eq!(irules.core.expect("core").family, Family::F5Irules);
        assert!(irules.expected_packages.iter().any(|p| p.ambient
            && *p.package == *"f5-irules-cmds"
            && p.version == Placement::Keyed(KeyedAxis::BigipVersion)));

        let iapps = registry.resolve("f5-iapps").expect("iapps");
        assert_eq!(
            iapps.policy_defaults.closed_world,
            WorldPolicy::AmbientPlusRequire
        );
        assert!(iapps.policy_defaults.strict_ascii, "the W108 rule");
        assert!(iapps.policy_defaults.fixed_ensembles);
        // F5 reclassification (measurements §4a): the iApps core rides
        // the `f5-tcl` trunk under the 32-bit scriptd build, not
        // tcl@8.5.
        let iapps_core = iapps.core.expect("core");
        assert_eq!(iapps_core.family, Family::F5Tcl);
        assert_eq!(iapps_core.default_release, Release::F5_TCL_TMOS);
        assert_eq!(iapps_core.build, BuildProfileId::F5Scriptd32);

        let tmsh = registry.resolve("f5-tmsh").expect("tmsh");
        assert!(!tmsh.policy_defaults.fixed_ensembles);
        // F5 reclassification (measurements §4a): the tmsh core rides
        // the `f5-tcl` trunk at its canonical build, not tcl@8.5.
        let tmsh_core = tmsh.core.expect("core");
        assert_eq!(tmsh_core.family, Family::F5Tcl);
        assert_eq!(tmsh_core.default_release, Release::F5_TCL_TMOS);
        assert_eq!(tmsh_core.build, BuildProfileId::Canonical);

        let expect_env = registry.resolve("expect").expect("expect");
        assert!(expect_env.expected_packages.iter().any(
            |p| p.ambient && p.version == Placement::Pinned(Version::parse("5.45.4").unwrap())
        ));

        let bigip = registry.resolve("f5-bigip").expect("bigip");
        assert!(bigip.core.is_none(), "identity-only: no Tcl core");

        for id in ["spectcl", "bpf"] {
            let env = registry.resolve(id).expect(id);
            assert_eq!(
                env.policy_defaults.closed_world,
                WorldPolicy::Closed,
                "{id}"
            );
            assert_eq!(
                env.core.expect("core").default_release,
                Release::TCL_9_0,
                "{id}"
            );
        }
    }

    /// Review B11 and the P3 pilot: the `tk` environment places Tk
    /// **ambient** (a `wish` shell has already loaded it — no `package
    /// require Tk` exists to write) on Tk's **own** version axis, never
    /// `tracks-base`. Every plain-Tcl environment places the same package
    /// **hosted**, which is what makes `Tk` a library with an ambient
    /// host rather than a closed-world vendor surface.
    #[test]
    fn tk_environment_uses_tks_own_axis() {
        let registry = EnvironmentRegistry::compiled();
        let tk = registry.resolve("tk").expect("tk");
        assert_eq!(tk.core.expect("core").default_release, Release::TCL_8_6);
        let placement = tk
            .expected_packages
            .iter()
            .find(|p| *p.package == *"Tk")
            .expect("Tk placement");
        assert!(placement.ambient, "wish ships Tk: no require to write");
        let Placement::Requirement(set) = &placement.version else {
            panic!("Tk must be floored on its own axis, got {placement:?}");
        };
        assert_eq!(set.axis().package_name(), Some("Tk"));
        // The alias is the shebang word too — one identity, two ingresses.
        assert_eq!(registry.resolve("wish").expect("wish").id.as_str(), "tk");
        assert!(
            tk.server_detection
                .shebang_words
                .iter()
                .any(|word| &**word == "wish")
        );
    }

    /// The hosted half of the same placement: every plain-Tcl environment
    /// declares that it *can* host Tk without shipping it, so "can this
    /// environment host Tk?" is a placement query with no lenient special
    /// case, and a release-pinned host derives the Tk point from its own
    /// release (B11's named exemption) while the unpinned ones do not.
    #[test]
    fn plain_tcl_environments_host_tk_without_shipping_it() {
        let registry = EnvironmentRegistry::compiled();
        for (id, expected) in [
            ("tcl", None),
            ("tcl8.4", Some("8.4")),
            ("tcl8.6", Some("8.6")),
            ("tcl9.0", Some("9.0")),
        ] {
            let definition = registry.resolve(id).expect(id);
            let placement = definition
                .expected_packages
                .iter()
                .find(|p| *p.package == *"Tk")
                .unwrap_or_else(|| panic!("{id} declares a Tk placement"));
            assert!(!placement.ambient, "{id}: hosted, so W120 still nags");
            match (expected, &placement.version) {
                (Some(release), Placement::TracksBase) => {
                    assert_eq!(
                        definition.core.expect("core").default_release.as_str(),
                        release,
                        "{id}"
                    );
                }
                (None, Placement::Requirement(set)) => {
                    assert_eq!(set.axis().package_name(), Some("Tk"), "{id}");
                }
                (_, other) => panic!("{id}: unexpected Tk placement {other:?}"),
            }
        }
        // A closed vendor shell declares none, so it cannot host Tk at all.
        for id in ["f5-irules", "bpf", "spectcl", "xilinx-eda-tcl"] {
            let definition = registry.resolve(id).expect(id);
            assert!(
                !definition
                    .expected_packages
                    .iter()
                    .any(|p| *p.package == *"Tk"),
                "{id}"
            );
        }
    }

    #[test]
    fn ladder_targets_are_single_release_lines() {
        let registry = EnvironmentRegistry::compiled();
        let tcl86 = registry.resolve("tcl8.6").expect("tcl8.6");
        let v = |text: &str| Version::parse(text).expect("version");
        assert!(tcl86.targets.contains(&v("8.6")));
        assert!(tcl86.targets.contains(&v("8.6.16")));
        assert!(!tcl86.targets.contains(&v("8.7")));
        assert!(!tcl86.targets.contains(&v("9.0")));
        assert_eq!(tcl86.targets.axis().core_family(), Some(Family::Tcl));
        // The lenient fallback spans the whole ladder.
        let plain = registry.resolve("tcl").expect("tcl");
        assert!(plain.targets.contains(&v("8.4")));
        assert!(plain.targets.contains(&v("9.1.2")));
        assert!(!plain.targets.contains(&v("9.2")));
    }

    #[test]
    fn every_seeded_editor_identity_is_contributed() {
        // I8's model-side half: the seeds can only reference the
        // contributed set — the newtype makes the violation
        // unrepresentable, so this pins the expected selections.
        let registry = EnvironmentRegistry::compiled();
        let expected: &[(&str, Option<&str>)] = &[
            ("tcl", Some("tcl")),
            ("tcl8.4", Some("tcl84")),
            ("tcl9.1", Some("tcl91")),
            ("f5-irules", Some("tcl-irule")),
            ("spectcl", Some("tclspec")),
            ("tk", None),
            ("bpf", None),
        ];
        for &(env, editor) in expected {
            let definition = registry.resolve(env).expect(env);
            assert_eq!(
                definition
                    .editor_identity
                    .map(EditorLanguageIdentityId::as_str),
                editor,
                "{env}"
            );
        }
        assert!(EditorLanguageIdentityId::new("tcl-apl").is_some());
        assert!(EditorLanguageIdentityId::new("not-a-language").is_none());
    }

    #[test]
    fn collisions_are_typed_construction_errors() {
        let base = compiled_definitions();
        // Duplicate canonical id.
        let mut dup = base.clone();
        dup.push(plain_tcl_environment());
        assert_eq!(
            EnvironmentRegistry::new(dup, 1).err(),
            Some(EnvironmentRegistryError::DuplicateCanonicalId(
                "tcl".to_owned()
            ))
        );
        // An alias shadowing a canonical id (the cycle shape).
        let mut shadowing = base.clone();
        let mut extra = bpf_environment();
        extra.id = EnvironmentId::new("my-env");
        extra.aliases = arcs(&["tcl8.6"]);
        shadowing.push(extra);
        assert_eq!(
            EnvironmentRegistry::new(shadowing, 1).err(),
            Some(EnvironmentRegistryError::AliasShadowsCanonical {
                alias: "tcl8.6".to_owned(),
                canonical: "tcl8.6".to_owned(),
            })
        );
        // Two environments claiming one alias.
        let mut dup_alias = base.clone();
        let mut a = bpf_environment();
        a.id = EnvironmentId::new("env-a");
        a.aliases = arcs(&["shared-alias"]);
        let mut b = bpf_environment();
        b.id = EnvironmentId::new("env-b");
        b.aliases = arcs(&["shared-alias"]);
        dup_alias.push(a);
        dup_alias.push(b);
        assert_eq!(
            EnvironmentRegistry::new(dup_alias, 1).err(),
            Some(EnvironmentRegistryError::DuplicateAlias(
                "shared-alias".to_owned()
            ))
        );
        // Two environments selecting one editor identity.
        let mut dup_editor = base.clone();
        let mut c = bpf_environment();
        c.id = EnvironmentId::new("env-c");
        c.editor_identity = EditorLanguageIdentityId::new("tcl-apl");
        let mut d = bpf_environment();
        d.id = EnvironmentId::new("env-d");
        d.editor_identity = EditorLanguageIdentityId::new("tcl-apl");
        dup_editor.push(c);
        dup_editor.push(d);
        assert_eq!(
            EnvironmentRegistry::new(dup_editor, 1).err(),
            Some(EnvironmentRegistryError::DuplicateEditorIdentity(
                "tcl-apl".to_owned()
            ))
        );
    }

    #[test]
    fn compiled_names_are_reserved_for_non_builtins() {
        let mut definitions = compiled_definitions();
        let mut intruder = bpf_environment();
        intruder.id = EnvironmentId::new("workspace-env");
        intruder.aliases = arcs(&["irules"]);
        intruder.provenance = Provenance::WorkspaceTrusted;
        definitions.retain(|d| d.id.as_str() != "f5-irules");
        definitions.push(intruder);
        // Even with the compiled irules definition absent from this
        // registry, its names stay reserved.
        assert_eq!(
            EnvironmentRegistry::new(definitions, 1).err(),
            Some(EnvironmentRegistryError::ReservedName {
                name: "irules".to_owned(),
                claimed_by: "workspace-env".to_owned(),
            })
        );
        // A namespaced third-party id passes.
        let mut fine = compiled_definitions();
        let mut third_party = bpf_environment();
        third_party.id = EnvironmentId::new("mypack/mytool");
        third_party.provenance = Provenance::User;
        fine.push(third_party);
        assert!(EnvironmentRegistry::new(fine, 1).is_ok());
    }

    #[test]
    fn overlays_derive_without_mutating_the_base() {
        let registry = EnvironmentRegistry::compiled();
        let base_before = registry.resolve("tcl8.6").expect("tcl8.6");
        let overlay = EnvironmentOverlay {
            base: EnvironmentId::new("tcl8.6"),
            target_changes: TargetChanges {
                targets: Some(
                    VersionSet::from_requirements(VersionAxisId::core(Family::Tcl), &["8.6-9.1"])
                        .expect("targets"),
                ),
            },
            package_changes: PackageChanges {
                add: vec![PackagePlacement {
                    package: arc("json"),
                    version: Placement::Requirement(
                        VersionSet::from_requirements(VersionAxisId::package("json"), &["1.0"])
                            .expect("requirement"),
                    ),
                    ambient: false,
                }],
                remove: arcs(&["Itcl"]),
            },
            origin: ConfigurationOrigin {
                provenance: Provenance::WorkspaceTrusted,
                content_hash: 0xDEAD_BEEF,
            },
        };
        let (derived, identity) = registry.apply_overlay(&overlay).expect("overlay applies");
        assert_eq!(identity.id.as_str(), "tcl8.6");
        assert_eq!(identity.generation, 0);
        assert_eq!(identity.overlay, Some(0xDEAD_BEEF));
        assert!(
            derived
                .targets
                .contains(&Version::parse("9.0").expect("version"))
        );
        assert!(
            derived
                .expected_packages
                .iter()
                .all(|p| *p.package != *"Itcl")
        );
        assert!(
            derived
                .expected_packages
                .iter()
                .any(|p| *p.package == *"json")
        );
        assert_eq!(derived.provenance, Provenance::WorkspaceTrusted);
        // The base is untouched: same value as before the overlay.
        let base_after = registry.resolve("tcl8.6").expect("tcl8.6");
        assert_eq!(*base_before, *base_after);
        assert_eq!(registry.identity_of(&base_after).overlay, None);

        // Overlay errors are typed.
        let unknown = EnvironmentOverlay {
            base: EnvironmentId::new("no-such-env"),
            target_changes: TargetChanges::default(),
            package_changes: PackageChanges::default(),
            origin: overlay.origin,
        };
        assert!(matches!(
            registry.apply_overlay(&unknown),
            Err(EnvironmentOverlayError::UnknownBase(_))
        ));
        let wrong_axis = EnvironmentOverlay {
            base: EnvironmentId::new("tcl8.6"),
            target_changes: TargetChanges {
                targets: Some(
                    VersionSet::from_requirements(VersionAxisId::package("Tk"), &["8.6"])
                        .expect("targets"),
                ),
            },
            package_changes: PackageChanges::default(),
            origin: overlay.origin,
        };
        assert!(matches!(
            registry.apply_overlay(&wrong_axis),
            Err(EnvironmentOverlayError::Targets(
                VersionSetError::AxisMismatch { .. }
            ))
        ));
    }
}
