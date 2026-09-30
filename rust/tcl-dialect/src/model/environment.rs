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
//! `docs/design/registry/dialect-and-package-registry-redesign.md` §3.3): the
//! named, selectable definitions of what a project works against, the
//! overlay mechanism that adjusts them without mutation, and the one
//! resolver every user-facing ingress goes through.
//!
//! An [`EnvironmentDefinition`] is a core-profile selector plus per-axis
//! version-set targets, expected/ambient package placements, server-side
//! detection facts, policy defaults, and a reference to a *fixed,
//! contributed* editor language identity — a server can never mint a new
//! editor language id. Environments are dynamic data: `Arc`-held, equality
//! by `(id, generation, overlay hash)`, never interned statics with
//! pointer identity. Workspace/user adjustments are
//! [`EnvironmentOverlay`]s whose content hash and origin are part of the
//! resolved identity — the canonical definition is never redefined in
//! place.
//!
//! The collision contract (§3.3): compiled canonical names are reserved,
//! alias cycles are unrepresentable (an alias may never equal any
//! canonical id, which is the only way a flat alias table could cycle),
//! and same-precedence collisions are typed construction errors, not
//! nearest-wins picks.
//!
//! [`EnvironmentRegistry::resolve`] is the one name validator: every name a
//! user can write — a canonical id, an alias, an editor language id — resolves
//! as data, not through a per-surface list.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

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

/// A member of the FIXED, contributed language identity set: the language ids
/// a client sends. Dynamic server environments *select among* these; they can
/// never mint a new one.
///
/// Most are the language ids the editor extensions contribute, each the
/// [`EnvironmentDefinition::editor_identity`] of the environment whose
/// documents open under it. The rest (`tcl-apl`, `tcl-bpf`, `tcl-libero`,
/// `tcl-spec`) are spellings a client may send that name an environment
/// without being the identity a generator emits a language mode for; an
/// environment lists those in
/// [`EnvironmentDefinition::selecting_identities`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EditorLanguageIdentityId(&'static str);

impl EditorLanguageIdentityId {
    /// The contributed set.
    pub const CONTRIBUTED: &'static [&'static str] = &[
        "tcl",
        "tcl-cadence",
        "tcl-expect",
        "tcl-bigip",
        "tcl-iapp",
        "tcl-irule",
        "tcl-jim",
        "tcl-tmsh",
        "tcl-quartus",
        "tcl-mentor",
        "tcl-microchip",
        "tclspec",
        "sslictcl",
        "tcl-synopsys",
        "tcl84",
        "tcl85",
        "tcl86",
        "tcl90",
        "tcl91",
        "tcl-xilinx",
        "tcl-apl",
        "tcl-bpf",
        "tcl-libero",
        "tcl-spec",
    ];

    /// The identity for `id`, or `None` when no editor contributes it —
    /// this constructor is the only way to obtain one, which is the whole
    /// enforcement of "a server can never mint a new editor language id".
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
    /// The build profile.
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
    /// for hosts that genuinely guarantee matched versions — never the
    /// default for Tk.
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

impl Provenance {
    /// Whether a definition of this provenance is gated by §6.4's
    /// **untrusted** rules: its registrations may add assistance but may
    /// never touch a reserved compiled name or a compiled dialect axis.
    ///
    /// This is the one place the class is decided. It was two: the
    /// `SpecTcl` loader and the registration layer each matched the same
    /// three variants, agreeing by maintenance rather than by
    /// construction (#2139). Both now ask here.
    ///
    /// The class is a property of the **provenance**, never of the tier a
    /// pack was discovered from. [`Provenance::WorkspaceTrusted`] and
    /// [`Provenance::WorkspaceUntrusted`] are both reachable from a
    /// workspace, and §6.4 keys the difference on the editor's Workspace
    /// Trust state rather than on where the file was found — a *trusted*
    /// workspace pack may `-override` a shipped command. Asking a tier
    /// directly is what produced the contradiction #2139 records.
    #[must_use]
    pub fn is_untrusted(self) -> bool {
        matches!(
            self,
            Provenance::WorkspaceUntrusted | Provenance::StudioOverride | Provenance::Document
        )
    }

    /// The machine-readable spelling status payloads carry. A workspace
    /// pack reads the same whether or not the editor trusts the workspace:
    /// trust is a separate fact from where the environment came from.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Provenance::BuiltIn => "built-in",
            Provenance::BundledPack => "bundled-pack",
            Provenance::User => "user-pack",
            Provenance::WorkspaceTrusted | Provenance::WorkspaceUntrusted => "workspace-pack",
            Provenance::StudioOverride => "studio-override",
            Provenance::Document => "document",
        }
    }
}

/// What an environment is, for presentation.
///
/// Every definition states one; no rule over the other fields separates
/// `bpf` (a language whose surface is a package over a Tcl 9.0 core) from
/// `tk` (a package over a Tcl 8.x core), so the classification is data.
///
/// Kind is display metadata only: it shapes
/// [`EnvironmentDefinition::description`] and is read by presentation
/// surfaces. It never influences resolution, grammar or availability.
/// Kinds order as declared: languages sort before tool shells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnvironmentKind {
    /// The thing being written is this language: its grammar, or its core
    /// command vocabulary, is the identity.
    Language,
    /// A stock Tcl release with library packages loaded — a tool shell.
    Packages,
}

impl EnvironmentKind {
    /// The pack-vocabulary spelling (`kind language|packages`).
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Language => "language",
            Self::Packages => "packages",
        }
    }

    /// The kind a pack-vocabulary word names.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        [Self::Language, Self::Packages]
            .into_iter()
            .find(|kind| kind.word() == word)
    }
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
    /// The compact name for tight UI (`Vivado`, `Jim`).
    pub short_name: Arc<str>,
    /// Whether the environment is a language or a Tcl release with
    /// packages loaded — presentation only, see [`EnvironmentKind`].
    pub kind: EnvironmentKind,
    /// The contributed editor identity this environment's documents open
    /// under, when one is dedicated.
    pub editor_identity: Option<EditorLanguageIdentityId>,
    /// Further contributed language ids that select this environment
    /// without being its [`Self::editor_identity`] (an APL file is an iApp
    /// presentation sublanguage; `tcl-bpf` names `bpf`).
    pub selecting_identities: Vec<EditorLanguageIdentityId>,
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

    /// The one-line description shown beside the environment's name in
    /// every picker, generated enum description and status tooltip.
    ///
    /// Derived from the definition, never authored: a `Language` reads as
    /// its display name; a `Packages` environment reads as its display name,
    /// the core release, and the ambient packages in declaration order
    /// (`Xilinx Vivado — Tcl 8.5 + vivado, sdc, upf`).
    #[must_use]
    pub fn description(&self) -> String {
        if self.kind == EnvironmentKind::Language {
            return self.display_name.to_string();
        }
        let base = self.core_label();
        let packages: Vec<&str> = self.ambient_packages().collect();
        let detail = match (base, packages.is_empty()) {
            (Some(base), true) => base,
            (Some(base), false) => format!("{base} + {}", packages.join(", ")),
            (None, false) => packages.join(", "),
            (None, true) => return self.display_name.to_string(),
        };
        format!("{} — {detail}", self.display_name)
    }

    /// The core release as a label (`Tcl 8.5`): the family's name and the
    /// core's default release. `None` for an environment with no ladder core.
    ///
    /// The one spelling of "which release is this environment built on" that
    /// [`Self::description`] and the tool-environment notice both read.
    #[must_use]
    pub fn core_label(&self) -> Option<String> {
        self.core
            .map(|core| format!("{} {}", core.family.display_name(), core.default_release))
    }

    /// The packages loaded without a `package require`, in the pack's
    /// declaration order. Hosted packages, which an environment only places
    /// when they are required, are left out.
    pub fn ambient_packages(&self) -> impl Iterator<Item = &str> {
        self.expected_packages
            .iter()
            .filter(|placement| placement.ambient)
            .map(|placement| placement.package.as_ref())
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

/// A workspace/user adjustment to a named environment: the canonical
/// definition is never redefined in place — the overlay
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
    /// A language id selects one environment while another environment
    /// already owns the spelling as its id, alias or editor identity, or
    /// selects it too.
    DuplicateSelectingIdentity(String),
    /// An alias equals a package another definition places, so a name read
    /// as either would resolve two ways.
    AliasSpellsPackage {
        /// The offending alias.
        alias: String,
        /// The canonical id of the definition the alias belongs to.
        claimed_by: String,
        /// The canonical id of the other definition that places the package.
        placed_by: String,
    },
    /// Two definitions claim one shebang interpreter word, which would make
    /// the shebang tier's answer depend on registration order.
    DuplicateShebangWord(String),
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
            Self::DuplicateSelectingIdentity(id) => {
                write!(
                    f,
                    "the language id `{id}` selects one environment but is already \
                     another's name, alias or identity"
                )
            }
            Self::AliasSpellsPackage {
                alias,
                claimed_by,
                placed_by,
            } => {
                write!(
                    f,
                    "alias `{alias}` of `{claimed_by}` is a package `{placed_by}` places"
                )
            }
            Self::DuplicateShebangWord(word) => {
                write!(f, "two environments claim the shebang word `{word}`")
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
    /// aliases, an alias spelling a package another definition places,
    /// duplicate editor identities, a language id selecting two
    /// environments, a shebang word claimed twice, or a non-built-in
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

    /// The environments a user can pick: every definition except the
    /// lenient sink, [`EnvironmentKind::Language`] first and then
    /// [`EnvironmentKind::Packages`], canonical id ascending within a kind.
    ///
    /// The one list every picker, `--dialect` value, tool schema and
    /// generated enumeration reads. The sink is where an unknown or
    /// unstated name lands, so it is a fallback rather than a choice.
    #[must_use]
    pub fn selectable(&self) -> Vec<Arc<EnvironmentDefinition>> {
        let mut selectable: Vec<Arc<EnvironmentDefinition>> = self
            .definitions
            .iter()
            .filter(|definition| definition.id.as_str() != LENIENT_ENVIRONMENT_ID)
            .cloned()
            .collect();
        selectable.sort_by(|left, right| {
            (left.kind, left.id.as_str()).cmp(&(right.kind, right.id.as_str()))
        });
        selectable
    }

    /// [`Self::selectable`] over the compiled registry (generation 0): the
    /// list for consumers that run before, or without, any pack
    /// registration — command-line parsing and the generators.
    #[must_use]
    pub fn compiled_selectable() -> &'static [Arc<EnvironmentDefinition>] {
        static COMPILED: OnceLock<Vec<Arc<EnvironmentDefinition>>> = OnceLock::new();
        COMPILED.get_or_init(|| Self::compiled().selectable())
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

/// Reject definitions claiming names reserved against their provenance
/// (§3.3: **all** compiled canonical names are reserved, and so are the
/// compiled aliases; a bundled pack's seeded names are reserved below the
/// bundled tier; editor identities are selectable by anyone, never minted,
/// which is the whole point of keeping them a fixed contributed set). See
/// [`reserved_against`].
fn check_reserved(definitions: &[EnvironmentDefinition]) -> Result<(), EnvironmentRegistryError> {
    let compiled = compiled_definitions();
    for definition in definitions {
        let claimed = std::iter::once(definition.id.as_str())
            .chain(definition.aliases.iter().map(AsRef::as_ref));
        for name in claimed {
            if let Some(reserved) = reserved_in(&compiled, name, definition.provenance) {
                return Err(EnvironmentRegistryError::ReservedName {
                    name: reserved,
                    claimed_by: definition.id.as_str().to_owned(),
                });
            }
        }
    }
    Ok(())
}

/// Build the four-tier name index: canonical ids, then aliases, then editor
/// identities, then selecting identities. Within a tier a collision is a
/// typed error; across tiers the higher tier wins, fixed here at
/// construction, except that a selecting identity another environment already
/// owns is itself a collision.
///
/// Two claims that are not names are checked here too, because the registry
/// is where every definition meets: an alias never spells a package another
/// definition places, and a shebang word selects one environment.
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
    check_alias_packages(definitions)?;
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
    for (position, definition) in definitions.iter().enumerate() {
        for identity in &definition.selecting_identities {
            match index.get(identity.as_str()) {
                // The definition's own alias or editor identity: `tcl-spec`
                // is both an alias and a selecting identity of `spectcl`.
                Some(&owner) if owner == position => {}
                Some(_) => {
                    return Err(EnvironmentRegistryError::DuplicateSelectingIdentity(
                        identity.as_str().to_owned(),
                    ));
                }
                None => {
                    index.insert(Arc::from(identity.as_str()), position);
                }
            }
        }
    }
    check_shebang_words(definitions)?;
    Ok(index)
}

/// Reject an alias that spells a package another definition places, ambient or
/// hosted, compared ASCII case-insensitively. A definition may alias a package
/// it places itself (`vivado` is both the Vivado shell's alias and the package
/// it loads).
fn check_alias_packages(
    definitions: &[Arc<EnvironmentDefinition>],
) -> Result<(), EnvironmentRegistryError> {
    let mut placed: HashMap<String, Vec<usize>> = HashMap::new();
    for (position, definition) in definitions.iter().enumerate() {
        for placement in &definition.expected_packages {
            placed
                .entry(placement.package.to_ascii_lowercase())
                .or_default()
                .push(position);
        }
    }
    for (position, definition) in definitions.iter().enumerate() {
        for alias in &definition.aliases {
            let other = placed
                .get(&alias.to_ascii_lowercase())
                .and_then(|owners| owners.iter().find(|&&owner| owner != position));
            if let Some(&other) = other {
                return Err(EnvironmentRegistryError::AliasSpellsPackage {
                    alias: alias.as_ref().to_owned(),
                    claimed_by: definition.id.as_str().to_owned(),
                    placed_by: definitions[other].id.as_str().to_owned(),
                });
            }
        }
    }
    Ok(())
}

/// Reject a shebang word two definitions claim, compared ASCII
/// case-insensitively as the shebang tier compares them.
fn check_shebang_words(
    definitions: &[Arc<EnvironmentDefinition>],
) -> Result<(), EnvironmentRegistryError> {
    let mut claimed: HashMap<String, usize> = HashMap::new();
    for (position, definition) in definitions.iter().enumerate() {
        for word in &definition.server_detection.shebang_words {
            let previous = claimed.insert(word.to_ascii_lowercase(), position);
            if previous.is_some_and(|owner| owner != position) {
                return Err(EnvironmentRegistryError::DuplicateShebangWord(
                    word.as_ref().to_owned(),
                ));
            }
        }
    }
    Ok(())
}

// The compiled seed set.

fn arc(text: &str) -> Arc<str> {
    Arc::from(text)
}

fn arcs(items: &[&str]) -> Vec<Arc<str>> {
    items.iter().map(|&item| arc(item)).collect()
}

fn identities(ids: &[&str]) -> Vec<EditorLanguageIdentityId> {
    ids.iter()
        .filter_map(|&id| EditorLanguageIdentityId::new(id))
        .collect()
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

/// The single-release-line target of a Tcl ladder release — see
/// [`release_line`].
fn tcl_line(release: Release) -> VersionSet {
    release_line(Family::Tcl, release)
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
        short_name: arc(&format!("Tcl {release}")),
        kind: EnvironmentKind::Language,
        editor_identity: EditorLanguageIdentityId::new(editor_id),
        selecting_identities: Vec::new(),
        core: Some(tcl_core(release)),
        targets: tcl_line(release),
        expected_packages: vec![
            // Tk is **hosted** here: a `tclsh8.6` document must
            // `package require Tk` (W120 nags when it does not), and the
            // floor rides Tk's **own** package axis — never the Tcl core
            // axis (invariant I2). `TracksBase` is the one named
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
            shebang_words: vec![
                arc(&format!("tclsh{release}")),
                arc(&format!("wish{release}")),
            ],
            ..DetectionFacts::default()
        },
        help_terms: arcs(&["tcl", "tk"]),
        provenance: Provenance::BuiltIn,
    })
    .collect()
}

/// The id of the lenient environment: the sink every unknown, unstated or
/// plain `tcl` name resolves to.
pub const LENIENT_ENVIRONMENT_ID: &str = "tcl";

/// The id of the environment a session analyses under when no dialect is
/// configured or detected. Every editor's `dialect` default is generated from
/// this constant, so a manifest cannot name a different starting point from
/// the one the server uses.
pub const DEFAULT_ENVIRONMENT_ID: &str = "tcl8.6";

/// The plain-`tcl` fallback: the full-ladder lenient environment every
/// unversioned document lands on.
fn plain_tcl_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new(LENIENT_ENVIRONMENT_ID),
        aliases: Vec::new(),
        display_name: arc("Tcl"),
        short_name: arc("Tcl"),
        kind: EnvironmentKind::Language,
        editor_identity: EditorLanguageIdentityId::new("tcl"),
        selecting_identities: Vec::new(),
        core: Some(tcl_core(Release::TCL_9_0)),
        targets: tcl_full_ladder(),
        // The lenient sink declares the same **hosted** Tk placement the
        // ladder rows carry, so "can this environment host Tk?" is a
        // placement query everywhere instead of a lenient special case.
        // No release is implied — an unversioned document names no Tcl
        // release either — so Tk sits on a requirement over its own axis,
        // which is also why this row grants no floor.
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
/// Tk's **own** version axis — never `tracks-base`. Erases the tk
/// triangle.
///
/// The placement is `ambient` because that is what a `wish` document *is*:
/// the interpreter has already loaded Tk before the first byte of the
/// script runs, so there is no `package require Tk` to write and none to
/// nag about. `package_active("Tk")`, the context's `TK` authoring bit,
/// the Tk-checks activation fact, and W120's silence all fall out of this
/// one placement fact, rather than being spelled three separate ways. The
/// version stays a **requirement** on `Tk`'s own axis rather than a point:
/// `wish` reports its own Tk patchlevel, which the document text does not
/// carry, so the honest answer is "some Tk ≥ 8.4" and the permissive
/// no-primary rule applies.
fn tk_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("tk"),
        aliases: arcs(&["wish"]),
        display_name: arc("Tk"),
        short_name: arc("Tk"),
        kind: EnvironmentKind::Packages,
        editor_identity: None,
        selecting_identities: Vec::new(),
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
/// the whole nine-release ladder.
///
/// A catalogue profile carries exactly one resolved `LexerGrammar`, which
/// would need a row per release. Here the grammar is a function of
/// `(family, release, build)` ([`crate::model::family::grammar`]), so the
/// environment names the family and the ladder, and a project picks its
/// point on the ladder with `# tcl-lsp: supports jim 0.81-0.84` — the §5.4
/// range machinery, on the `jim` core axis.
///
/// It claims the contributed `tcl-jim` editor identity, and has two
/// deliberate absences:
///
/// - **No release-pinned siblings.** `jim0.84` is not an environment
///   name; it is a target on this environment's axis.
/// - **No expected packages.** Jim's command surface rides its ancestry
///   edge from Tcl 8.6 ([`Family::ancestry`]) — inherit-then-override.
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
        short_name: arc("Jim"),
        kind: EnvironmentKind::Language,
        editor_identity: EditorLanguageIdentityId::new("tcl-jim"),
        selecting_identities: Vec::new(),
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
        short_name: arc("iRules"),
        kind: EnvironmentKind::Language,
        editor_identity: EditorLanguageIdentityId::new("tcl-irule"),
        selecting_identities: Vec::new(),
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
        short_name: arc("iApps"),
        kind: EnvironmentKind::Language,
        editor_identity: EditorLanguageIdentityId::new("tcl-iapp"),
        selecting_identities: identities(&["tcl-apl"]),
        // Per measurement (`docs/design/f5/bigip-irule-parser-measurements.md`
        // §4a): the 8.5 baseline hypothesis is falsified — `IAppImplementation`
        // reports patchlevel 8.4.6, fails every 8.5 discriminator, and
        // carries the full `f5-tcl` trunk grammar. The core rides the
        // trunk under the 32-bit `scriptd` build profile (`wordSize 4`,
        // measurements §4).
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
            // The W108 strict-ASCII rule.
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
        short_name: arc("tmsh"),
        kind: EnvironmentKind::Language,
        editor_identity: EditorLanguageIdentityId::new("tcl-tmsh"),
        selecting_identities: Vec::new(),
        // CORRECTED by measurement
        // (`docs/design/f5/bigip-irule-parser-measurements.md` §4a): the
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
        short_name: arc("BIG-IP"),
        kind: EnvironmentKind::Language,
        editor_identity: EditorLanguageIdentityId::new("tcl-bigip"),
        selecting_identities: Vec::new(),
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
        short_name: arc("Expect"),
        kind: EnvironmentKind::Language,
        editor_identity: EditorLanguageIdentityId::new("tcl-expect"),
        selecting_identities: Vec::new(),
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
        short_name: arc("SpecTcl"),
        kind: EnvironmentKind::Language,
        editor_identity: EditorLanguageIdentityId::new("tclspec"),
        selecting_identities: identities(&["tcl-spec"]),
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

fn sslictcl_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("sslictcl"),
        aliases: arcs(&["sslic-tcl", "tls-sslictcl"]),
        display_name: arc("SslicTcl"),
        short_name: arc("SslicTcl"),
        kind: EnvironmentKind::Language,
        editor_identity: EditorLanguageIdentityId::new("sslictcl"),
        selecting_identities: Vec::new(),
        core: Some(tcl_core(Release::TCL_9_0)),
        targets: tcl_line(Release::TCL_9_0),
        expected_packages: Vec::new(),
        policy_defaults: EnvironmentPolicy {
            // A `.sslictcl` document is declarative and is never evaluated —
            // not even its `predicate` bodies — so nothing is
            // `package require`-able into it.
            closed_world: WorldPolicy::Closed,
            fixed_ensembles: false,
            strict_ascii: false,
            version_ceiling: Some(Release::TCL_9_0),
        },
        server_detection: DetectionFacts {
            file_extensions: vec![ext("sslictcl", "SslicTcl TLS Declaration")],
            ..DetectionFacts::default()
        },
        help_terms: arcs(&["sslictcl", "tls", "certificate", "endpoint"]),
        provenance: Provenance::BuiltIn,
    }
}

fn bpf_environment() -> EnvironmentDefinition {
    EnvironmentDefinition {
        id: EnvironmentId::new("bpf"),
        aliases: Vec::new(),
        display_name: arc("BPF"),
        short_name: arc("BPF"),
        kind: EnvironmentKind::Language,
        editor_identity: None,
        selecting_identities: identities(&["tcl-bpf"]),
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

/// The compiled environment definitions: every current
/// `DialectProfile` catalogue entry translated, plus the `tk` and
/// plain-`tcl` environments that erase the off-catalogue profiles, plus
/// the environments the bundled packs declare
/// ([`bundled_pack_definitions`]), seeded at `Provenance::BundledPack`.
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
    definitions.push(sslictcl_environment());
    definitions.push(bpf_environment());
    definitions.extend(bundled_pack_definitions());
    definitions
}

/// The single-release-line target set of one ladder release: the release
/// line itself, `[R·a0, next-minor·a0)` — `8.6-8.7`, not a floor and not
/// "up to the next ladder release" (`8.6-9.0` would take in 8.7, which is
/// no release of anything). A release whose spelling is not `major.minor`
/// (the F5 trunk's `tmos`) takes the next ladder release as its bound, or
/// stays unbounded at the top of the ladder.
///
/// The one spelling of "this release line" the compiled definitions, a
/// pack-declared `core` row and the bundled seed share, so the forms of
/// one environment target the same set.
#[must_use]
pub fn release_line(family: Family, release: Release) -> VersionSet {
    let axis = VersionAxisId::core(family);
    let spelling = release.as_str();
    let next_minor = spelling.split_once('.').and_then(|(major, minor)| {
        let minor: u32 = minor.parse().ok()?;
        Some(format!("{major}.{}", minor + 1))
    });
    let requirement = match (next_minor, family.next_release(release)) {
        (Some(next), _) => format!("{spelling}-{next}"),
        (None, Some(next)) => format!("{spelling}-{}", next.as_str()),
        (None, None) => format!("{spelling}-"),
    };
    VersionSet::from_requirements(axis, &[requirement])
        .unwrap_or_else(|_| VersionSet::empty(VersionAxisId::core(family)))
}

/// The compiled spelling `name` collides with when a definition of
/// `provenance` claims it, if any.
///
/// Two reservation strengths (§3.3, §6.4): a **built-in** id or alias is
/// reserved against every other provenance, and a spelling the **bundled
/// packs** seed ([`bundled_pack_definitions`]) is reserved against every
/// provenance below the bundled tier — the bundled pack that declared it
/// restates it on every publish, and nothing else may claim it.
#[must_use]
pub fn reserved_against(name: &str, provenance: Provenance) -> Option<String> {
    reserved_in(&compiled_definitions(), name, provenance)
}

/// [`reserved_against`] over an already-built compiled set, so a whole
/// registry can be checked without rebuilding the seed per claimed name.
fn reserved_in(
    compiled: &[EnvironmentDefinition],
    name: &str,
    provenance: Provenance,
) -> Option<String> {
    for definition in compiled {
        let claimable = match definition.provenance {
            Provenance::BuiltIn => provenance == Provenance::BuiltIn,
            Provenance::BundledPack => {
                matches!(provenance, Provenance::BuiltIn | Provenance::BundledPack)
            }
            _ => true,
        };
        if claimable {
            continue;
        }
        if definition.id.as_str() == name {
            return Some(definition.id.as_str().to_owned());
        }
        if let Some(alias) = definition
            .aliases
            .iter()
            .find(|alias| alias.as_ref() == name)
        {
            return Some(alias.to_string());
        }
    }
    None
}

/// One `environment` block of a bundled pack, as `cargo xtask
/// gen-bundled-environments` projects it into
/// [`bundled_environments::BUNDLED_ENVIRONMENTS`].
///
/// The packs under `specs/` are the source of truth for these
/// environments (redesign D17); this table is their compiled projection,
/// held equal to them by the generator's `--check` gate and by
/// `tcl-spectcl`'s pack parity test. It exists so the environments resolve
/// at generation 0 — before, or without, a pack publish — everywhere a name
/// is resolved: the acceptance gates in `tcl-registry`, the MCP server, the
/// `xtask` generators, and the closed-world package derivation that runs
/// while the packs themselves are still being merged.
#[derive(Debug, Clone, Copy)]
pub struct BundledEnvironmentRow {
    /// The block's canonical id.
    pub id: &'static str,
    /// `display_name`, defaulting to the id.
    pub display_name: &'static str,
    /// `short_name`, defaulting to the display name.
    pub short_name: &'static str,
    /// The `kind` word, defaulting to `packages`.
    pub kind: EnvironmentKind,
    /// `alias` rows.
    pub aliases: &'static [&'static str],
    /// `editor_identity`, a contributed id.
    pub editor_identity: Option<&'static str>,
    /// `selecting_identity` rows, contributed ids.
    pub selecting_identities: &'static [&'static str],
    /// The `core` row.
    pub core: Option<BundledCore>,
    /// `ambient` / `hosted` rows, in declaration order.
    pub placements: &'static [BundledPlacement],
    /// The `policy` word.
    pub world_policy: WorldPolicy,
    /// `version_ceiling`, a release on the core's ladder.
    pub version_ceiling: Option<&'static str>,
    /// `file_extension EXT -name NAME` rows.
    pub file_extensions: &'static [(&'static str, &'static str)],
    /// `filename` rows.
    pub filenames: &'static [&'static str],
    /// `signature` rows.
    pub signatures: &'static [&'static str],
    /// `help_terms` words.
    pub help_terms: &'static [&'static str],
}

/// A bundled block's `core FAMILY RELEASE ?-build P?` row.
#[derive(Debug, Clone, Copy)]
pub struct BundledCore {
    /// The compiled family.
    pub family: Family,
    /// The release spelling on that family's ladder.
    pub release: &'static str,
    /// The build profile.
    pub build: BuildProfileId,
}

/// A bundled block's `ambient` / `hosted` row.
#[derive(Debug, Clone, Copy)]
pub struct BundledPlacement {
    /// The package name.
    pub package: &'static str,
    /// The version word.
    pub version: BundledVersion,
    /// Ambient (no `package require`) vs hosted.
    pub ambient: bool,
}

/// The version word of a bundled placement row, as the block spelt it.
#[derive(Debug, Clone, Copy)]
pub enum BundledVersion {
    /// A fixed version (`ambient Expect 5.45.4`).
    Pinned(&'static str),
    /// `tracks-base`.
    TracksBase,
    /// `keyed KEY`.
    Keyed(KeyedAxis),
    /// A requirement on the package's own axis (`hosted Tk 8.5-`).
    Requirement(&'static str),
}

fn ladder_release(family: Family, spelling: &str) -> Release {
    family
        .releases()
        .iter()
        .copied()
        .find(|release| release.as_str() == spelling)
        .unwrap_or_else(|| {
            panic!("generated bundled row names `{spelling}` on the {family:?} ladder")
        })
}

impl BundledEnvironmentRow {
    /// The definition this row seeds — the same total conversion a loaded
    /// block makes, at `Provenance::BundledPack`.
    fn definition(&self) -> EnvironmentDefinition {
        let core = self.core.map(|core| CoreProfileSelector {
            family: core.family,
            default_release: ladder_release(core.family, core.release),
            build: core.build,
        });
        let targets = core.map_or_else(
            || VersionSet::empty(VersionAxisId::core(Family::Tcl)),
            |core| release_line(core.family, core.default_release),
        );
        let version_ceiling = self.version_ceiling.map(|spelling| {
            let family = core
                .expect("a generated ceiling always sits on its core's ladder")
                .family;
            ladder_release(family, spelling)
        });
        EnvironmentDefinition {
            id: EnvironmentId::new(self.id),
            aliases: arcs(self.aliases),
            display_name: arc(self.display_name),
            short_name: arc(self.short_name),
            kind: self.kind,
            editor_identity: self.editor_identity.and_then(EditorLanguageIdentityId::new),
            selecting_identities: identities(self.selecting_identities),
            core,
            targets,
            expected_packages: self
                .placements
                .iter()
                .map(|placement| PackagePlacement {
                    package: arc(placement.package),
                    version: match placement.version {
                        BundledVersion::Pinned(version) => Placement::Pinned(ver(version)),
                        BundledVersion::TracksBase => Placement::TracksBase,
                        BundledVersion::Keyed(axis) => Placement::Keyed(axis),
                        BundledVersion::Requirement(requirement) => Placement::Requirement(reqs(
                            VersionAxisId::package(placement.package),
                            &[requirement],
                        )),
                    },
                    ambient: placement.ambient,
                })
                .collect(),
            policy_defaults: EnvironmentPolicy {
                closed_world: self.world_policy,
                fixed_ensembles: false,
                strict_ascii: false,
                version_ceiling,
            },
            server_detection: DetectionFacts {
                file_extensions: self
                    .file_extensions
                    .iter()
                    .map(|&(extension, display_name)| ext(extension, display_name))
                    .collect(),
                filenames: arcs(self.filenames),
                content_signatures: arcs(self.signatures),
                shebang_words: Vec::new(),
                directive_names: Vec::new(),
            },
            help_terms: arcs(self.help_terms),
            provenance: Provenance::BundledPack,
        }
    }
}

/// The environments the bundled packs declare, from the generated table.
#[must_use]
pub fn bundled_pack_definitions() -> Vec<EnvironmentDefinition> {
    super::bundled_environments::BUNDLED_ENVIRONMENTS
        .iter()
        .map(BundledEnvironmentRow::definition)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DialectProfile;
    use crate::TclVersion;

    /// A stand-in for a pack-declared environment: `bpf`'s definition under
    /// `id`, with none of `bpf`'s language-id claims.
    fn pack_environment(id: &str) -> EnvironmentDefinition {
        let mut definition = bpf_environment();
        definition.id = EnvironmentId::new(id);
        definition.selecting_identities = Vec::new();
        definition
    }

    /// #2139: one predicate owns the untrusted class, and every variant has
    /// a stated answer.
    ///
    /// The inner `match` is the point: it is exhaustive, so a new
    /// [`Provenance`] variant does not compile here until someone decides
    /// its trust class. (The array still has to be extended by hand — the
    /// match forces the decision, it cannot force the coverage.)
    #[test]
    fn provenance_states_a_trust_class_for_every_variant_issue_2139() {
        for provenance in [
            Provenance::BuiltIn,
            Provenance::BundledPack,
            Provenance::User,
            Provenance::WorkspaceTrusted,
            Provenance::WorkspaceUntrusted,
            Provenance::StudioOverride,
            Provenance::Document,
        ] {
            let expected = match provenance {
                Provenance::BuiltIn
                | Provenance::BundledPack
                | Provenance::User
                | Provenance::WorkspaceTrusted => false,
                Provenance::WorkspaceUntrusted
                | Provenance::StudioOverride
                | Provenance::Document => true,
            };
            assert_eq!(
                provenance.is_untrusted(),
                expected,
                "{provenance:?} changed trust class"
            );
        }
    }

    /// A workspace is trusted until the editor's trust state is plumbed
    /// (ledger item O9) — the fact the `EvalOptions::tier` doc comment used
    /// to deny.
    #[test]
    fn a_workspace_is_trusted_until_the_editor_says_otherwise_issue_2139() {
        assert!(!Provenance::WorkspaceTrusted.is_untrusted());
        assert!(Provenance::WorkspaceUntrusted.is_untrusted());
    }

    #[test]
    fn every_old_name_and_alias_resolves() {
        let registry = EnvironmentRegistry::compiled();
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

    /// One `jim` environment for a nine-release ladder: the releases are
    /// targets on the family's own axis, not nine catalogue rows, and no
    /// editor identity is minted for it.
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
        assert_eq!(
            jim.editor_identity.map(EditorLanguageIdentityId::as_str),
            Some("tcl-jim"),
            "jim opens under its own contributed language id"
        );
        assert_eq!(jim.short_name.as_ref(), "Jim");
        assert!(
            jim.expected_packages.is_empty(),
            "the jim surface rides the ancestry edge, not a placement"
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

        for id in ["spectcl", "sslictcl", "bpf"] {
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

    /// The `tk` environment places Tk **ambient** (a `wish` shell has
    /// already loaded it — no `package
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
    /// release (the one named exemption) while the unpinned ones do not.
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
        for id in ["f5-irules", "bpf", "spectcl", "sslictcl", "xilinx-eda-tcl"] {
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

    /// A selecting language id is contributed, resolves to the environment
    /// that lists it, and is no other environment's id or editor identity: an
    /// environment's own identity and canonical id resolve as themselves.
    #[test]
    fn selecting_language_ids_resolve_to_the_environment_that_lists_them() {
        let registry = EnvironmentRegistry::compiled();
        let mut selecting: Vec<(&str, &str)> = Vec::new();
        for definition in registry.definitions() {
            for identity in &definition.selecting_identities {
                selecting.push((identity.as_str(), definition.id.as_str()));
            }
        }
        selecting.sort_unstable();
        assert_eq!(
            selecting,
            [
                ("tcl-apl", "f5-iapps"),
                ("tcl-bpf", "bpf"),
                ("tcl-libero", "microchip-libero-eda-tcl"),
                ("tcl-spec", "spectcl"),
            ]
        );
        for (spelling, owner) in selecting {
            assert_eq!(
                registry.resolve(spelling).map(|found| found.id.to_string()),
                Some(owner.to_owned()),
                "`{spelling}` selects `{owner}`"
            );
            assert!(
                !registry.definitions().iter().any(|definition| {
                    definition.id.as_str() == spelling
                        || definition
                            .editor_identity
                            .is_some_and(|identity| identity.as_str() == spelling)
                }),
                "`{spelling}` is no environment's own id or identity"
            );
        }
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
            ("jim", Some("tcl-jim")),
            ("spectcl", Some("tclspec")),
            ("sslictcl", Some("sslictcl")),
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

    /// The kind of every compiled environment is a stated judgement:
    /// `bpf` is a language over a Tcl 9.0 core, `tk` is a package over a
    /// Tcl 8.x core, and nothing derives one from the other fields.
    #[test]
    fn every_compiled_environment_states_its_kind() {
        const LANGUAGES: &[&str] = &[
            "tcl",
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "f5-irules",
            "f5-iapps",
            "f5-tmsh",
            "f5-bigip",
            "jim",
            "bpf",
            "expect",
            "spectcl",
            "sslictcl",
        ];
        const PACKAGES: &[&str] = &[
            "tk",
            "xilinx-eda-tcl",
            "intel-quartus-eda-tcl",
            "mentor-eda-tcl",
            "microchip-libero-eda-tcl",
            "synopsys-eda-tcl",
            "cadence-eda-tcl",
        ];
        let registry = EnvironmentRegistry::compiled();
        for definition in registry.definitions() {
            let id = definition.id.as_str();
            let expected = match (LANGUAGES.contains(&id), PACKAGES.contains(&id)) {
                (true, false) => EnvironmentKind::Language,
                (false, true) => EnvironmentKind::Packages,
                other => panic!("{id}: not stated as exactly one kind ({other:?})"),
            };
            assert_eq!(definition.kind, expected, "{id}");
        }
        assert_eq!(
            registry.definitions().len(),
            LANGUAGES.len() + PACKAGES.len()
        );
    }

    #[test]
    fn the_kind_words_round_trip() {
        for kind in [EnvironmentKind::Language, EnvironmentKind::Packages] {
            assert_eq!(EnvironmentKind::from_word(kind.word()), Some(kind));
        }
        assert_eq!(EnvironmentKind::from_word("Packages"), None);
        assert_eq!(EnvironmentKind::from_word(""), None);
    }

    /// A `Language` describes itself by its display name alone.
    #[test]
    fn a_language_describes_itself_by_name() {
        let registry = EnvironmentRegistry::compiled();
        for (id, description) in [
            ("jim", "Jim Tcl"),
            ("f5-irules", "F5 iRules"),
            ("tcl8.6", "Tcl 8.6"),
            ("expect", "Expect"),
        ] {
            assert_eq!(registry.resolve(id).expect(id).description(), description);
        }
    }

    /// A `Packages` environment names its core release and its ambient
    /// packages in declaration order; hosted packages are not ambient and
    /// are left out.
    #[test]
    fn a_tool_shell_describes_its_core_and_ambient_packages() {
        let registry = EnvironmentRegistry::compiled();
        assert_eq!(
            registry
                .resolve("xilinx-eda-tcl")
                .expect("xilinx")
                .description(),
            "Xilinx Vivado — Tcl 8.5 + vivado, sdc, upf"
        );
        let tk = registry.resolve("tk").expect("tk");
        assert_eq!(tk.description(), "Tk — Tcl 8.6 + Tk");
        assert!(
            tk.expected_packages
                .iter()
                .any(|placement| !placement.ambient && &*placement.package == "Itcl"),
            "Itcl is hosted on tk, so the description omits it"
        );

        let mut bare = (*tk).clone();
        bare.expected_packages.clear();
        assert_eq!(bare.description(), "Tk — Tcl 8.6");
        bare.core = None;
        assert_eq!(bare.description(), "Tk");
    }

    /// The core label and the ambient packages are what the description is
    /// made of, so anything else that quotes them agrees with it.
    #[test]
    fn the_description_is_the_core_label_and_the_ambient_packages() {
        let registry = EnvironmentRegistry::compiled();
        let vivado = registry.resolve("xilinx-eda-tcl").expect("xilinx");
        assert_eq!(vivado.core_label().as_deref(), Some("Tcl 8.5"));
        assert_eq!(
            vivado.ambient_packages().collect::<Vec<_>>(),
            ["vivado", "sdc", "upf"]
        );
        assert_eq!(
            vivado.description(),
            "Xilinx Vivado — Tcl 8.5 + vivado, sdc, upf"
        );
        let tk = registry.resolve("tk").expect("tk");
        assert_eq!(tk.ambient_packages().collect::<Vec<_>>(), ["Tk"]);

        let mut no_core = (*vivado).clone();
        no_core.core = None;
        assert_eq!(no_core.core_label(), None);
    }

    /// Kind is presentation: two definitions that differ only in kind
    /// resolve, and describe their core, identically.
    #[test]
    fn kind_does_not_change_what_an_environment_resolves_to() {
        let registry = EnvironmentRegistry::compiled();
        let tk = registry.resolve("tk").expect("tk");
        let mut as_language = (*tk).clone();
        as_language.kind = EnvironmentKind::Language;
        assert_eq!(as_language.core, tk.core);
        assert_eq!(as_language.point(), tk.point());
        assert_eq!(as_language.expected_packages, tk.expected_packages);
        assert_eq!(as_language.policy_defaults, tk.policy_defaults);
    }

    /// An alias selects an environment; it never spells a package another
    /// environment places, ambient or hosted, in any case, so a name read as
    /// either resolves one way. An alias may equal a package its own
    /// environment places: the tool's shell and the tool's package share the
    /// tool's name.
    #[test]
    fn an_alias_never_spells_another_environments_package() {
        let placed_by_other = |alias: &str| {
            let mut definitions = compiled_definitions();
            let mut extra = pack_environment("runtime-pack-env");
            extra.provenance = Provenance::WorkspaceTrusted;
            extra.aliases = arcs(&[alias]);
            definitions.push(extra);
            EnvironmentRegistry::new(definitions, 1).err()
        };
        // `sdc` is ambient in the Vivado shell; `TK` is hosted by the ladder.
        for (alias, package) in [("sdc", "sdc"), ("TK", "Tk")] {
            match placed_by_other(alias) {
                Some(EnvironmentRegistryError::AliasSpellsPackage {
                    alias: named,
                    claimed_by,
                    placed_by,
                }) => {
                    assert_eq!(named, alias);
                    assert_eq!(claimed_by, "runtime-pack-env");
                    assert!(
                        registry_places(&placed_by, package),
                        "`{placed_by}` places `{package}`"
                    );
                }
                other => panic!("`{alias}` must be rejected as a package, got {other:?}"),
            }
        }
        assert_eq!(placed_by_other("not-a-package"), None);
        // The owner is exempt: `vivado` is the Vivado shell's alias and package.
        let registry = EnvironmentRegistry::compiled();
        let vivado = registry.resolve("vivado").expect("the Vivado alias");
        assert!(
            vivado
                .expected_packages
                .iter()
                .any(|placement| placement.package.eq_ignore_ascii_case("vivado"))
        );
    }

    fn registry_places(environment: &str, package: &str) -> bool {
        EnvironmentRegistry::compiled()
            .resolve(environment)
            .is_some_and(|definition| {
                definition
                    .expected_packages
                    .iter()
                    .any(|placement| placement.package.as_ref() == package)
            })
    }

    /// The compiled environments select by these interpreter words.
    #[test]
    fn the_compiled_shebang_words_name_their_environments() {
        let registry = EnvironmentRegistry::compiled();
        for (word, owner) in [
            ("jimsh", "jim"),
            ("wish", "tk"),
            ("expect", "expect"),
            ("tclsh8.5", "tcl8.5"),
            ("wish9.0", "tcl9.0"),
        ] {
            let claimants: Vec<&str> = registry
                .definitions()
                .iter()
                .filter(|definition| {
                    definition
                        .server_detection
                        .shebang_words
                        .iter()
                        .any(|claimed| claimed.as_ref() == word)
                })
                .map(|definition| definition.id.as_str())
                .collect();
            assert_eq!(claimants, [owner], "{word}");
        }
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
        let mut extra = pack_environment("my-env");
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
        let mut a = pack_environment("env-a");
        a.aliases = arcs(&["shared-alias"]);
        let mut b = pack_environment("env-b");
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
        let mut c = pack_environment("env-c");
        c.editor_identity = EditorLanguageIdentityId::new("tcl-apl");
        let mut d = pack_environment("env-d");
        d.editor_identity = EditorLanguageIdentityId::new("tcl-apl");
        dup_editor.push(c);
        dup_editor.push(d);
        assert_eq!(
            EnvironmentRegistry::new(dup_editor, 1).err(),
            Some(EnvironmentRegistryError::DuplicateEditorIdentity(
                "tcl-apl".to_owned()
            ))
        );
        // A language id another environment already selects, or owns as an
        // identity, alias or id.
        // (`tcl-bpf` is selected by `bpf`, `tcl-jim` is Jim's editor identity,
        // `tcl-spec` is an alias of `spectcl`, `sslictcl` is a canonical id.)
        for taken in ["tcl-bpf", "tcl-jim", "tcl-spec", "sslictcl"] {
            let mut selecting = base.clone();
            let mut claimant = pack_environment("selecting-claimant");
            claimant.selecting_identities = identities(&[taken]);
            selecting.push(claimant);
            assert_eq!(
                EnvironmentRegistry::new(selecting, 1).err(),
                Some(EnvironmentRegistryError::DuplicateSelectingIdentity(
                    taken.to_owned()
                )),
                "{taken}"
            );
        }
        // Two environments claiming one shebang word, in any case.
        let mut dup_shebang = base;
        let mut claimant = pack_environment("shebang-claimant");
        claimant.server_detection.shebang_words = arcs(&["JimSH"]);
        dup_shebang.push(claimant);
        assert_eq!(
            EnvironmentRegistry::new(dup_shebang, 1).err(),
            Some(EnvironmentRegistryError::DuplicateShebangWord(
                "JimSH".to_owned()
            ))
        );
    }

    /// The environments the bundled packs declare seed the compiled
    /// registry at `Provenance::BundledPack` (redesign D17), and their
    /// names are reserved one step below the built-in ones — restatable by
    /// the bundled tier, refused from every other.
    #[test]
    fn bundled_pack_names_are_reserved_below_the_bundled_tier() {
        let seeded = bundled_pack_definitions();
        assert!(!seeded.is_empty(), "the bundled packs declare environments");
        for definition in &seeded {
            assert_eq!(definition.provenance, Provenance::BundledPack);
            let id = definition.id.as_str();
            assert_eq!(
                reserved_against(id, Provenance::BundledPack),
                None,
                "{id}: the bundled tier restates its own environment"
            );
            for provenance in [
                Provenance::User,
                Provenance::WorkspaceTrusted,
                Provenance::WorkspaceUntrusted,
                Provenance::StudioOverride,
                Provenance::Document,
            ] {
                assert_eq!(
                    reserved_against(id, provenance).as_deref(),
                    Some(id),
                    "{id}: reserved against {provenance:?}"
                );
            }
        }
        // A built-in name stays reserved against the bundled tier too.
        assert_eq!(
            reserved_against("tcl8.6", Provenance::BundledPack).as_deref(),
            Some("tcl8.6")
        );
        assert_eq!(reserved_against("tcl8.6", Provenance::BuiltIn), None);

        // The registry enforces the same rule at construction: a workspace
        // definition claiming a bundled name is a typed error, while a
        // bundled restatement of a *different* id is fine.
        let bundled_id = seeded[0].id.as_str().to_owned();
        let mut hijack = compiled_definitions();
        hijack.retain(|definition| definition.id.as_str() != bundled_id);
        let mut intruder = pack_environment(&bundled_id);
        intruder.provenance = Provenance::WorkspaceTrusted;
        hijack.push(intruder);
        assert_eq!(
            EnvironmentRegistry::new(hijack, 1).err(),
            Some(EnvironmentRegistryError::ReservedName {
                name: bundled_id.clone(),
                claimed_by: bundled_id,
            })
        );
    }

    /// The six vendor `DialectProfile` rows stay compiled as the lexer's
    /// grammar key and the editor catalogues' key, so each must agree with
    /// the pack-declared environment on every identity fact both carry.
    #[test]
    fn every_bundled_environment_agrees_with_its_catalogue_profile() {
        let seeded = bundled_pack_definitions();
        for definition in &seeded {
            let id = definition.id.as_str();
            let profile = DialectProfile::find(id)
                .unwrap_or_else(|| panic!("{id}: a bundled environment keeps its profile row"));
            assert_eq!(
                definition.display_name.as_ref(),
                profile.display_name,
                "{id}"
            );
            assert_eq!(definition.short_name.as_ref(), profile.short_name, "{id}");
            let aliases: Vec<&str> = definition.aliases.iter().map(AsRef::as_ref).collect();
            assert_eq!(aliases, profile.aliases, "{id}");
            assert_eq!(
                definition.kind,
                EnvironmentKind::Packages,
                "{id}: a vendor shell is a Tcl release with packages"
            );
            assert_eq!(
                definition
                    .editor_identity
                    .map(EditorLanguageIdentityId::as_str),
                profile.editor_language_id,
                "{id}"
            );
            let extensions: Vec<(&str, &str)> = definition
                .server_detection
                .file_extensions
                .iter()
                .map(|claim| (claim.extension.as_ref(), claim.display_name.as_ref()))
                .collect();
            let profile_extensions: Vec<(&str, &str)> = profile
                .file_extensions
                .iter()
                .map(|row| (row.extension, row.display_name))
                .collect();
            assert_eq!(extensions, profile_extensions, "{id}");
            let terms: Vec<&str> = definition.help_terms.iter().map(AsRef::as_ref).collect();
            assert_eq!(terms, profile.help_terms, "{id}");
            let core = definition.core.expect("a vendor shell selects a Tcl core");
            assert_eq!(core.family, Family::Tcl, "{id}");
            assert_eq!(
                Some(core.default_release.as_str()),
                profile.runtime_base.map(TclVersion::version_string),
                "{id}: base release"
            );
            assert_eq!(
                definition
                    .policy_defaults
                    .version_ceiling
                    .map(Release::as_str),
                profile.version_ceiling.map(TclVersion::version_string),
                "{id}: ceiling"
            );
            for pin in profile.libraries {
                let placement = definition
                    .expected_packages
                    .iter()
                    .find(|placement| placement.package.as_ref() == pin.package)
                    .unwrap_or_else(|| panic!("{id}: the block places `{}`", pin.package));
                assert_eq!(placement.ambient, pin.ambient, "{id}: `{}`", pin.package);
            }
            assert_eq!(
                definition.expected_packages.len(),
                profile.libraries.len(),
                "{id}: the block places exactly the profile's libraries"
            );
        }
        let profiles: Vec<&str> = DialectProfile::all()
            .iter()
            .filter(|profile| profile.name.ends_with("-eda-tcl"))
            .map(|profile| profile.name)
            .collect();
        let mut seeded_ids: Vec<&str> = seeded.iter().map(|d| d.id.as_str()).collect();
        seeded_ids.sort_unstable();
        assert_eq!(seeded_ids, profiles, "every vendor shell is pack-declared");
    }

    /// The selectable set is the registry without the lenient sink,
    /// languages before tool shells, canonical id ascending within a kind.
    #[test]
    fn selectable_lists_languages_then_packages_by_id_without_the_sink() {
        let selectable = EnvironmentRegistry::compiled().selectable();
        let ids: Vec<&str> = selectable
            .iter()
            .map(|definition| definition.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "bpf",
                "expect",
                "f5-bigip",
                "f5-iapps",
                "f5-irules",
                "f5-tmsh",
                "jim",
                "spectcl",
                "sslictcl",
                "tcl8.4",
                "tcl8.5",
                "tcl8.6",
                "tcl9.0",
                "tcl9.1",
                "cadence-eda-tcl",
                "intel-quartus-eda-tcl",
                "mentor-eda-tcl",
                "microchip-libero-eda-tcl",
                "synopsys-eda-tcl",
                "tk",
                "xilinx-eda-tcl",
            ]
        );
        assert!(!ids.contains(&LENIENT_ENVIRONMENT_ID));
        let languages = selectable
            .iter()
            .take_while(|definition| definition.kind == EnvironmentKind::Language)
            .count();
        assert!(
            selectable[languages..]
                .iter()
                .all(|definition| definition.kind == EnvironmentKind::Packages),
            "no language follows a tool shell"
        );
    }

    /// A pack-declared environment joins the selectable set of the registry
    /// it was built into, among the tool shells.
    #[test]
    fn a_pack_declared_environment_is_selectable_beside_the_tool_shells() {
        let mut declared = pack_environment("spicegentcl/ngspice");
        declared.kind = EnvironmentKind::Packages;
        declared.provenance = Provenance::WorkspaceTrusted;
        let mut definitions = compiled_definitions();
        definitions.push(declared);
        let registry = EnvironmentRegistry::new(definitions, 1).expect("collision-free");
        let ids: Vec<String> = registry
            .selectable()
            .iter()
            .map(|definition| definition.id.to_string())
            .collect();
        let acme = ids.iter().position(|id| id == "spicegentcl/ngspice");
        let cadence = ids.iter().position(|id| id == "cadence-eda-tcl");
        let xilinx = ids.iter().position(|id| id == "xilinx-eda-tcl");
        assert!(acme.is_some(), "{ids:?}");
        assert!(
            cadence < acme && acme < xilinx,
            "`spicegentcl/ngspice` sorts among the tool shells: {ids:?}"
        );
    }

    /// The starting environment is a choice a user can also make: it is
    /// selectable and is not the lenient sink.
    #[test]
    fn the_default_environment_is_selectable() {
        assert_ne!(DEFAULT_ENVIRONMENT_ID, LENIENT_ENVIRONMENT_ID);
        assert!(
            EnvironmentRegistry::compiled_selectable()
                .iter()
                .any(|definition| definition.id.as_str() == DEFAULT_ENVIRONMENT_ID)
        );
    }

    /// The compiled form is the generation-0 registry's selectable set.
    #[test]
    fn compiled_selectable_is_the_compiled_registrys_selectable_set() {
        let compiled = EnvironmentRegistry::compiled().selectable();
        assert_eq!(EnvironmentRegistry::compiled_selectable(), compiled);
    }

    /// The machine-readable provenance words a status payload carries.
    #[test]
    fn provenance_words_name_where_an_environment_came_from() {
        let registry = EnvironmentRegistry::compiled();
        let word = |id: &str| registry.resolve(id).expect(id).provenance.word();
        assert_eq!(word("jim"), "built-in");
        assert_eq!(word("tk"), "built-in");
        assert_eq!(word("xilinx-eda-tcl"), "bundled-pack");
        assert_eq!(Provenance::User.word(), "user-pack");
        assert_eq!(Provenance::WorkspaceTrusted.word(), "workspace-pack");
        assert_eq!(Provenance::WorkspaceUntrusted.word(), "workspace-pack");
    }

    #[test]
    fn compiled_names_are_reserved_for_non_builtins() {
        let mut definitions = compiled_definitions();
        let mut intruder = pack_environment("workspace-env");
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
        let mut third_party = pack_environment("mypack/mytool");
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
