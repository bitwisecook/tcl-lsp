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

//! The `SpecTcl` 2.0 `environment NAME { … }` pack-level block (§6.2).
//!
//! An environment is the **selectable, aliasable identity** of §3.3 — what
//! a `# tcl-dialect:` directive, a settings string, or a file-extension
//! detection resolves to. Before 2.0 the only way to add one was to
//! compile it into `tcl-dialect`; this block lets a pack declare one:
//!
//! ```text
//! environment vivado-tcl {
//!     display_name    {Xilinx Vivado}
//!     core            tcl 8.6
//!     ambient         Vivado keyed ToolVersion
//!     hosted          Tk 8.5-
//!     alias           vivado
//!     editor_identity tcl
//!     file_extension  xdc -name {Xilinx Design Constraints}
//!     filename        vivado.jou
//!     signature       {create_project}
//!     policy          ambient-plus-require
//!     version_ceiling 8.6
//!     help_terms      {vivado xilinx}
//! }
//! ```
//!
//! ## What this module does, and does not, do
//!
//! It **parses, validates, and carries**: a [`PackEnvironment`] on the
//! pack plus the two total conversions the registration seam consumes —
//! [`PackEnvironment::to_definition`] (a declaration into an
//! [`EnvironmentDefinition`] at the declaring tier's [`Provenance`]) and
//! [`PackEnvironment::to_extension`] (an `-extend` block into an
//! additive [`tcl_registry::model::EnvironmentExtension`]). Registering
//! either into the live [`EnvironmentRegistry`] is
//! [`crate::registration`]'s call, under the §6.4 trust lattice.
//!
//! ## Reserved names (§3.3)
//!
//! Every compiled canonical id and every compiled alias is reserved. A
//! block claiming one is **rejected** — a notice, and the block is not
//! carried — because a workspace pack silently redefining `tcl8.6` or
//! `f5-irules` is the §6.4 trust boundary, not an editing convenience.
//! The names a **bundled** pack declares (the six EDA shells, seeded into
//! the compiled registry from the packs themselves — D17) are reserved
//! one step lower: the bundled tier may restate them, every other tier
//! is refused, and that refusal lives where the tier is known (the E-R2
//! gate and the registration seam). Editor identities are deliberately
//! *not* reserved: selecting one is their whole B7 purpose.

use std::sync::Arc;

use tcl_dialect::model::{BuildProfileId, Family, Release};
use tcl_dialect::model::{
    CoreProfileSelector, DetectionFacts, EditorLanguageIdentityId, EnvironmentDefinition,
    EnvironmentId, EnvironmentPolicy, FileExtensionClaim, KeyedAxis, PackagePlacement, Placement,
    Provenance, VersionAxisId, VersionSet, WorldPolicy, release_line, reserved_against,
};

use super::{Log, Stmt, block, next_text};
use crate::discovery::Tier;

/// The trust tier a pack-declared environment carries into the model.
///
/// A thin, total map onto [`Provenance`]: the loader knows the tier a pack
/// came from, and §6.4 says the tier *is* the trust class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackEnvironmentTier {
    /// Shipped with the distribution.
    Bundled,
    /// A per-user config directory pack.
    User,
    /// A workspace pack (`.tcl-lsp/`), trusted or not by the editor.
    Workspace,
    /// A live Spec Studio override.
    StudioOverride,
}

impl PackEnvironmentTier {
    /// The tier of a discovered pack.
    #[must_use]
    pub fn of(tier: Tier) -> Self {
        match tier {
            Tier::Bundled => Self::Bundled,
            Tier::User => Self::User,
            Tier::Workspace => Self::Workspace,
            Tier::StudioOverride => Self::StudioOverride,
        }
    }

    /// The §6.4 trust class this tier maps to.
    #[must_use]
    pub fn provenance(self) -> Provenance {
        match self {
            Self::Bundled => Provenance::BundledPack,
            Self::User => Provenance::User,
            Self::Workspace => Provenance::WorkspaceTrusted,
            Self::StudioOverride => Provenance::StudioOverride,
        }
    }
}

/// A `core` row naming a dialect the pack itself declares, before it is
/// resolved against the pack's `dialect` blocks.
///
/// The stand-in for a [`CoreProfileSelector`] that cannot exist: a
/// [`Family`] is a closed enum, so a pack-declared family has no variant
/// to select. The binding it becomes lives in
/// [`tcl_dialect::model::DynamicCore`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackCore {
    /// The dialect name the row named, as written.
    pub dialect: String,
    /// The release on that dialect's declared ladder.
    pub release: String,
    /// The build profile the row names.
    pub build: BuildProfileId,
    /// The declaring line.
    pub line: u32,
}

/// One `ambient` / `hosted` row, before it becomes a
/// [`PackagePlacement`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackPlacementRow {
    /// The package name as `package require` spells it.
    pub package: String,
    /// How the version is determined.
    pub version: Placement,
    /// The version word as the row spelt it (`3.1`, `8.5-`, `keyed`,
    /// `tracks-base`) — what a projection of the block writes back, since a
    /// parsed requirement set no longer carries its own spelling.
    pub version_word: String,
    /// Ambient (no `package require` needed) vs hosted.
    pub ambient: bool,
    /// The declaring line.
    pub line: u32,
}

/// A parsed `environment NAME { … }` block, or its additive
/// `environment NAME -extend { … }` form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackEnvironment {
    /// The canonical id the block declares — or, for an `-extend` block,
    /// the id (or alias) of the environment being extended.
    pub id: String,
    /// Whether this is the `-extend` form: an **additive** contribution
    /// of detection facts and placements to an environment declared
    /// elsewhere (a compiled one, or another pack's). An extend block
    /// may not restate identity facts (`core`, `policy`, `alias`,
    /// `editor_identity`, `display_name`) — those belong to the owner —
    /// and, unlike a declaration, it may name a compiled environment:
    /// the §6.4 trust gate on that lives at registration (and in the
    /// E-R2 evaluation gate), where the tier is known.
    pub extends: bool,
    /// `alias NAME` rows, in declaration order.
    pub aliases: Vec<String>,
    /// `display_name TEXT`, defaulting to the id.
    pub display_name: Option<String>,
    /// The validated `editor_identity ID`, when one resolved. An unknown
    /// id keeps the row (a notice) but drops the routing — §6.1's
    /// presentation rule, since an editor identity only decides which
    /// contributed language a document opens under.
    pub editor_identity: Option<EditorLanguageIdentityId>,
    /// The `core FAMILY RELEASE ?-build P?` selector, when it names a
    /// **compiled** family.
    pub core: Option<CoreProfileSelector>,
    /// The same row when it names a **pack-declared** dialect (§6.2's
    /// `dialect` block), carried unresolved because the block it names
    /// may be declared later in the file.
    ///
    /// [`crate::loader::finish_pack_cores`] resolves it against the
    /// pack's own `dialect` blocks once the whole file is read, and
    /// rejects the environment when it names nothing. A block that gets
    /// this far therefore always names a dialect the pack declares.
    pub pack_core: Option<PackCore>,
    /// `ambient` and `hosted` rows, in declaration order.
    pub placements: Vec<PackPlacementRow>,
    /// The `policy` word, defaulting to [`WorldPolicy::Open`].
    pub world_policy: WorldPolicy,
    /// `file_extension EXT ?-name NAME?` rows.
    pub file_extensions: Vec<FileExtensionClaim>,
    /// `filename NAME` rows.
    pub filenames: Vec<String>,
    /// `signature TEXT` rows.
    pub signatures: Vec<String>,
    /// `help_terms {WORD …}` rows, flattened in declaration order: the
    /// lower-case help-index filter terms `tcl help --dialect` narrows by.
    pub help_terms: Vec<String>,
    /// `version_ceiling RELEASE` — the upper-bound release for option
    /// gating (§5.2), on the `core` family's ladder. Resolved once the
    /// whole block is read, so the row may precede its `core`.
    pub version_ceiling: Option<Release>,
    /// The declaring line, for notices and editors.
    pub line: u32,
}

impl PackEnvironment {
    /// The [`EnvironmentDefinition`] this block describes, at `tier`.
    ///
    /// Total: every field of the block has a home, and the fields the
    /// block cannot state take the model's own defaults — an environment
    /// with no `core` row is the §3.3 identity-only case (`f5-bigip`),
    /// and its target set is empty rather than "everything".
    #[must_use]
    pub fn to_definition(&self, tier: PackEnvironmentTier) -> EnvironmentDefinition {
        debug_assert!(
            !self.extends,
            "an extend block converts through `to_extension`, never to a definition"
        );
        let targets = self.core.map_or_else(
            || VersionSet::empty(VersionAxisId::core(Family::Tcl)),
            |core| release_line(core.family, core.default_release),
        );
        EnvironmentDefinition {
            id: EnvironmentId::new(&self.id),
            aliases: self.aliases.iter().map(|a| Arc::from(a.as_str())).collect(),
            display_name: Arc::from(self.display_name.as_deref().unwrap_or(&self.id)),
            editor_identity: self.editor_identity,
            core: self.core,
            targets,
            expected_packages: self
                .placements
                .iter()
                .map(|row| PackagePlacement {
                    package: Arc::from(row.package.as_str()),
                    version: row.version.clone(),
                    ambient: row.ambient,
                })
                .collect(),
            policy_defaults: EnvironmentPolicy {
                closed_world: self.world_policy,
                fixed_ensembles: false,
                strict_ascii: false,
                version_ceiling: self.version_ceiling,
            },
            server_detection: DetectionFacts {
                file_extensions: self.file_extensions.clone(),
                filenames: self
                    .filenames
                    .iter()
                    .map(|f| Arc::from(f.as_str()))
                    .collect(),
                content_signatures: self
                    .signatures
                    .iter()
                    .map(|s| Arc::from(s.as_str()))
                    .collect(),
                shebang_words: Vec::new(),
                directive_names: Vec::new(),
            },
            help_terms: self
                .help_terms
                .iter()
                .map(|term| Arc::from(term.as_str()))
                .collect(),
            provenance: tier.provenance(),
        }
    }
}

impl PackEnvironment {
    /// The [`tcl_registry::model::EnvironmentExtension`] an `-extend`
    /// block describes, at `tier` — the registration seam's input.
    #[must_use]
    pub fn to_extension(
        &self,
        tier: PackEnvironmentTier,
    ) -> tcl_registry::model::EnvironmentExtension {
        debug_assert!(
            self.extends,
            "a declaration block converts through `to_definition`, never to an extension"
        );
        tcl_registry::model::EnvironmentExtension {
            base: self.id.clone(),
            file_extensions: self.file_extensions.clone(),
            filenames: self
                .filenames
                .iter()
                .map(|f| Arc::from(f.as_str()))
                .collect(),
            content_signatures: self
                .signatures
                .iter()
                .map(|s| Arc::from(s.as_str()))
                .collect(),
            placements: self
                .placements
                .iter()
                .map(|row| PackagePlacement {
                    package: Arc::from(row.package.as_str()),
                    version: row.version.clone(),
                    ambient: row.ambient,
                })
                .collect(),
            provenance: tier.provenance(),
        }
    }
}

/// Parse one `environment NAME { … }` block (or
/// `environment NAME -extend { … }`) written **literally**, or reject it.
///
/// The thin half of the reader: it splits the braced body into rows and
/// hands them to [`parse_rows`], which owns every validation and notice.
/// The evaluation loader runs the body as a script instead and calls
/// [`parse_rows`] with the rows the script registered, so a block written
/// with a variable, a `foreach`, or an `if` is validated by exactly the
/// same code as one written out longhand.
pub(super) fn parse(stmt: &Stmt, log: &mut Log) -> Option<PackEnvironment> {
    let body_index = if stmt.word_text(2) == "-extend" { 3 } else { 2 };
    let rows = stmt.arg(body_index).map(block);
    parse_rows(stmt, rows.as_deref(), log)
}

/// Parse one `environment` declaration from its header and its already-read
/// rows, or reject it.
///
/// `rows` is `None` when the declaration had no `{ … }` body word at all —
/// the brace-on-the-next-line mistake — which is a rejection with its own
/// notice rather than an empty block.
pub(super) fn parse_rows(
    stmt: &Stmt,
    rows: Option<&[Stmt]>,
    log: &mut Log,
) -> Option<PackEnvironment> {
    let name = stmt.word_text(1);
    if name.is_empty() || stmt.words.get(1).is_some_and(|word| word.braced) {
        log.say(stmt.line, "`environment` needs a name and a `{ … }` block");
        return None;
    }
    let extends = stmt.word_text(2) == "-extend";
    if extends {
        log.since(stmt.line, "environment -extend", "2.0");
    }
    let Some(rows) = rows else {
        log.say(
            stmt.line,
            format!("`environment {name}` has no `{{ … }}` block; the block is rejected"),
        );
        return None;
    };
    // A declaration claiming a compiled name is the §3.3 reservation and
    // is rejected outright. An `-extend` block *must* name an existing
    // environment — compiled included — so the reservation does not apply
    // to it; the §6.4 trust gate on extending a compiled base lives where
    // the tier is known (registration, and the E-R2 evaluation gate) —
    // as does the gate on a name only the bundled tier may restate.
    if !extends && let Some(reserved) = reserved_name(name) {
        log.say(
            stmt.line,
            format!(
                "`environment {name}` claims `{reserved}`, a compiled environment name \
                 (design §3.3 reserves every compiled id and alias); the block is rejected"
            ),
        );
        return None;
    }
    let mut environment = PackEnvironment {
        id: name.to_owned(),
        extends,
        aliases: Vec::new(),
        display_name: None,
        editor_identity: None,
        core: None,
        pack_core: None,
        placements: Vec::new(),
        world_policy: WorldPolicy::Open,
        file_extensions: Vec::new(),
        filenames: Vec::new(),
        signatures: Vec::new(),
        help_terms: Vec::new(),
        version_ceiling: None,
        line: stmt.line,
    };
    let mut rejected = false;
    let mut pending = Pending::default();
    log.scoped(format!("environment {name}"), |log| {
        for row in rows {
            if !read_row(&mut environment, row, &mut pending, log) {
                rejected = true;
            }
        }
        if !settle_ceiling(&mut environment, &pending, log) {
            rejected = true;
        }
    });
    if rejected {
        return None;
    }
    for alias in &environment.aliases {
        if let Some(reserved) = reserved_name(alias) {
            log.say(
                stmt.line,
                format!(
                    "`environment {name}` aliases `{reserved}`, a compiled environment name \
                     (design §3.3); the block is rejected"
                ),
            );
            return None;
        }
    }
    Some(environment)
}

/// Rows that resolve only once the whole block is read.
#[derive(Default)]
struct Pending {
    /// The `version_ceiling` word and its line, resolved against the
    /// `core` row's family by [`settle_ceiling`].
    ceiling: Option<(String, u32)>,
}

/// Resolve a pending `version_ceiling` against the block's `core` family.
/// `false` rejects the block: a ceiling is a point on a compiled ladder,
/// so it needs a compiled `core` to name one.
fn settle_ceiling(environment: &mut PackEnvironment, pending: &Pending, log: &mut Log) -> bool {
    let Some((word, line)) = &pending.ceiling else {
        return true;
    };
    let Some(core) = environment.core else {
        log.say(
            *line,
            format!(
                "`version_ceiling {word}` needs a `core` row naming a compiled family \
                 whose ladder carries the release; the environment block is rejected"
            ),
        );
        return false;
    };
    let found = core
        .family
        .releases()
        .iter()
        .copied()
        .find(|release| release.as_str() == word);
    let Some(release) = found else {
        log.say(
            *line,
            format!(
                "`version_ceiling {word}` names no release on the {} ladder; the \
                 environment block is rejected",
                core.family.name()
            ),
        );
        return false;
    };
    environment.version_ceiling = Some(release);
    true
}

/// Read one row. `false` means the whole block is rejected — the §6.1
/// semantic class, which is what every unknown word in an environment
/// block is: this block says which world is closed and what is ambient in
/// it, so there is no decorative word here to drop safely.
fn read_row(
    environment: &mut PackEnvironment,
    stmt: &Stmt,
    pending: &mut Pending,
    log: &mut Log,
) -> bool {
    // An `-extend` block is additive by construction: identity rows
    // belong to the environment's owner, and reading one here would let
    // an extension restate what §6.4 says only the owner may state.
    if environment.extends
        && matches!(
            stmt.word_text(0),
            "core"
                | "policy"
                | "alias"
                | "editor_identity"
                | "display_name"
                | "help_terms"
                | "version_ceiling"
        )
    {
        log.say(
            stmt.line,
            format!(
                "`{}` is an identity row and an `environment -extend` block is \
                 additive (detection rows and placements only); the block is rejected",
                stmt.word_text(0)
            ),
        );
        return false;
    }
    match stmt.word_text(0) {
        "display_name" => environment.display_name = Some(stmt.word_text(1).to_owned()),
        "alias" => match stmt.word_text(1) {
            "" => log.say(stmt.line, "`alias` needs a name"),
            alias => environment.aliases.push(alias.to_owned()),
        },
        "core" => return core_row(environment, stmt, log),
        "ambient" => return placement_row(environment, stmt, true, log),
        "hosted" => return placement_row(environment, stmt, false, log),
        "editor_identity" => editor_identity_row(environment, stmt, log),
        "file_extension" => file_extension_row(environment, stmt, log),
        "filename" => match stmt.word_text(1) {
            "" => log.say(stmt.line, "`filename` needs a basename"),
            name => environment.filenames.push(name.to_owned()),
        },
        "signature" => match stmt.word_text(1) {
            "" => log.say(stmt.line, "`signature` needs the text to look for"),
            text => environment.signatures.push(text.to_owned()),
        },
        "help_terms" => {
            let terms = super::list_words(stmt.word_text(1));
            if terms.is_empty() {
                log.say(stmt.line, "`help_terms` needs at least one term");
            }
            environment
                .help_terms
                .extend(terms.into_iter().map(|term| term.to_ascii_lowercase()));
        }
        "version_ceiling" => match stmt.word_text(1) {
            "" => {
                log.say(
                    stmt.line,
                    "`version_ceiling` needs a release; the environment block is rejected",
                );
                return false;
            }
            release => pending.ceiling = Some((release.to_owned(), stmt.line)),
        },
        "policy" => {
            let word = stmt.word_text(1);
            let Some(policy) = world_policy(word) else {
                log.say(
                    stmt.line,
                    format!(
                        "`policy {word}` is not a world policy (`open`, `closed`, \
                         `ambient-plus-require`); the environment block is rejected"
                    ),
                );
                return false;
            };
            environment.world_policy = policy;
        }
        other => {
            log.say(
                stmt.line,
                format!(
                    "unknown `environment` row `{}` is semantic-class vocabulary \
                     (design §6.1); the environment block is rejected",
                    super::quotable(other)
                ),
            );
            return false;
        }
    }
    true
}

/// `editor_identity ID`: an unknown id keeps the row without routing
/// (§6.1's presentation rule).
fn editor_identity_row(environment: &mut PackEnvironment, stmt: &Stmt, log: &mut Log) {
    let id = stmt.word_text(1);
    match EditorLanguageIdentityId::new(id) {
        Some(identity) => environment.editor_identity = Some(identity),
        None => log.say(
            stmt.line,
            format!(
                "`editor_identity {id}` is not a contributed editor language id \
                 (review B7 — an environment selects one, never mints one); the row \
                 is kept without routing"
            ),
        ),
    }
}

/// `file_extension EXT ?-name NAME?`.
fn file_extension_row(environment: &mut PackEnvironment, stmt: &Stmt, log: &mut Log) {
    let words = &stmt.words;
    let raw = stmt.word_text(1);
    let extension = raw.trim_start_matches('.').to_ascii_lowercase();
    if extension.is_empty() || extension.contains('.') {
        log.say(
            stmt.line,
            format!("`file_extension {raw}` is not a single extension"),
        );
        return;
    }
    let mut display = extension.clone();
    let mut index = 2;
    while index < words.len() {
        match words[index].text.as_str() {
            "-name" => display = next_text(words, &mut index),
            other => log.unknown_flag("file_extension", stmt.line, other),
        }
        index += 1;
    }
    environment.file_extensions.push(FileExtensionClaim {
        extension: Arc::from(extension.as_str()),
        display_name: Arc::from(display.as_str()),
    });
}

fn world_policy(word: &str) -> Option<WorldPolicy> {
    match word {
        "open" => Some(WorldPolicy::Open),
        "closed" => Some(WorldPolicy::Closed),
        "ambient-plus-require" => Some(WorldPolicy::AmbientPlusRequire),
        _ => None,
    }
}

/// `core FAMILY RELEASE ?-build P?`.
fn core_row(environment: &mut PackEnvironment, stmt: &Stmt, log: &mut Log) -> bool {
    let family_word = stmt.word_text(1);
    let compiled = Family::ALL
        .iter()
        .copied()
        .find(|family| family.name() == family_word);
    let release_word = stmt.word_text(2);
    // A name no compiled family carries may still be a dialect this pack
    // declares (§6.2's `dialect` block). The block naming it can come
    // *later* in the file, so the row is carried unresolved and
    // `finish_pack_cores` settles it — rejecting the environment then if
    // nothing declares the name, which is the same answer this row used
    // to give immediately, only correct for a forward reference.
    let release = if let Some(family) = compiled {
        let found = family
            .releases()
            .iter()
            .copied()
            .find(|release| release.as_str() == release_word);
        let Some(release) = found else {
            log.say(
                stmt.line,
                format!(
                    "`core {family_word} {release_word}` names no release on the \
                     {family_word} ladder; the environment block is rejected"
                ),
            );
            return false;
        };
        Some(release)
    } else {
        if release_word.is_empty() {
            log.say(
                stmt.line,
                format!(
                    "`core {family_word}` needs a release on that dialect's ladder; \
                     the environment block is rejected"
                ),
            );
            return false;
        }
        None
    };
    let mut build = BuildProfileId::Canonical;
    let words = &stmt.words;
    let mut index = 3;
    while index < words.len() {
        match words[index].text.as_str() {
            "-build" => {
                let named = next_text(words, &mut index);
                match named.as_str() {
                    "Canonical" => build = BuildProfileId::Canonical,
                    "Unknown" => build = BuildProfileId::Unknown,
                    other => {
                        log.say(
                            stmt.line,
                            format!(
                                "`-build {other}` is not a build profile (`Canonical`, \
                                 `Unknown`); the environment block is rejected"
                            ),
                        );
                        return false;
                    }
                }
            }
            other => {
                log.unknown_flag("core", stmt.line, other);
            }
        }
        index += 1;
    }
    match (compiled, release) {
        (Some(family), Some(default_release)) => {
            environment.core = Some(CoreProfileSelector {
                family,
                default_release,
                build,
            });
        }
        _ => {
            environment.pack_core = Some(PackCore {
                dialect: family_word.to_owned(),
                release: release_word.to_owned(),
                build,
                line: stmt.line,
            });
        }
    }
    true
}

/// `ambient PACKAGE VERSION|tracks-base|keyed KEY` and
/// `hosted PACKAGE REQUIREMENT`.
fn placement_row(
    environment: &mut PackEnvironment,
    stmt: &Stmt,
    ambient: bool,
    log: &mut Log,
) -> bool {
    let row_word = if ambient { "ambient" } else { "hosted" };
    let package = stmt.word_text(1);
    if package.is_empty() {
        log.say(
            stmt.line,
            format!("`{row_word}` needs a package name; the environment block is rejected"),
        );
        return false;
    }
    let placement = match stmt.word_text(2) {
        "" => {
            log.say(
                stmt.line,
                format!(
                    "`{row_word} {package}` needs a version, `tracks-base`, or \
                     `keyed KEY`; the environment block is rejected"
                ),
            );
            return false;
        }
        "tracks-base" => Placement::TracksBase,
        "keyed" => {
            let key = stmt.word_text(3);
            let Some(axis) = keyed_axis(key) else {
                log.say(
                    stmt.line,
                    format!(
                        "`keyed {key}` names no external version key (`BigipVersion`, \
                         `ToolVersion`, `SdcVersion`, `UpfVersion`); the environment \
                         block is rejected"
                    ),
                );
                return false;
            };
            Placement::Keyed(axis)
        }
        requirement if ambient && !requirement.contains('-') => {
            match tcl_dialect::model::Version::parse(requirement) {
                Ok(version) => Placement::Pinned(version),
                Err(err) => {
                    log.say(
                        stmt.line,
                        format!(
                            "`{row_word} {package} {requirement}` is not a version ({err}); \
                             the environment block is rejected"
                        ),
                    );
                    return false;
                }
            }
        }
        requirement => {
            let axis = VersionAxisId::package(package);
            match VersionSet::from_requirements(axis, &[requirement]) {
                Ok(set) => Placement::Requirement(set),
                Err(err) => {
                    log.say(
                        stmt.line,
                        format!(
                            "`{row_word} {package} {requirement}` is not a requirement \
                             ({err}); the environment block is rejected"
                        ),
                    );
                    return false;
                }
            }
        }
    };
    environment.placements.push(PackPlacementRow {
        package: package.to_owned(),
        version: placement,
        version_word: stmt.word_text(2).to_owned(),
        ambient,
        line: stmt.line,
    });
    true
}

fn keyed_axis(key: &str) -> Option<KeyedAxis> {
    match key {
        "BigipVersion" => Some(KeyedAxis::BigipVersion),
        "ToolVersion" => Some(KeyedAxis::ToolVersion),
        "SdcVersion" => Some(KeyedAxis::SdcVersion),
        "UpfVersion" => Some(KeyedAxis::UpfVersion),
        _ => None,
    }
}

/// The compiled canonical id or alias `name` collides with for **every**
/// pack tier, if any — the built-in rows. A name a bundled pack's own
/// block seeded is not in this set: the bundled tier restates it.
pub(crate) fn reserved_name(name: &str) -> Option<String> {
    reserved_against(name, Provenance::BundledPack)
}

/// The compiled or bundled spelling `name` collides with when claimed
/// from `tier`, if any — the tier-aware form the evaluation loader's E-R2
/// gate and the registration seam ([`crate::registration`]) ask, so a
/// workspace pack cannot claim `xilinx-eda-tcl` any more than `tcl8.6`.
pub(crate) fn reserved_name_for(name: &str, tier: PackEnvironmentTier) -> Option<String> {
    reserved_against(name, tier.provenance())
}
