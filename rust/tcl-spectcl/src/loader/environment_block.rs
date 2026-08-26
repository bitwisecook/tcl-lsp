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
//! }
//! ```
//!
//! ## What this module does, and does not, do
//!
//! It **parses, validates, and carries**. Registering a pack-declared
//! environment into the live [`EnvironmentRegistry`] is P3 wire-up; what
//! lands here is a [`PackEnvironment`] on the pack plus
//! [`PackEnvironment::to_definition`], the total conversion into an
//! [`EnvironmentDefinition`] with the declaring tier's [`Provenance`].
//! Unit tests cover the conversion, so the wire-up is a call, not a
//! design.
//!
//! ## Reserved names (§3.3)
//!
//! Every compiled canonical id and every compiled alias is reserved. A
//! block claiming one is **rejected** — a notice, and the block is not
//! carried — because a workspace pack silently redefining `tcl8.6` or
//! `f5-irules` is the §6.4 trust boundary, not an editing convenience.
//! Editor identities are deliberately *not* reserved: selecting one is
//! their whole B7 purpose.

use std::sync::Arc;

use tcl_dialect::model::{BuildProfileId, Family, Release};
use tcl_dialect::model::{
    CoreProfileSelector, DetectionFacts, EditorLanguageIdentityId, EnvironmentDefinition,
    EnvironmentId, EnvironmentPolicy, FileExtensionClaim, KeyedAxis, PackagePlacement, Placement,
    Provenance, VersionAxisId, VersionSet, WorldPolicy, compiled_definitions,
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

/// One `ambient` / `hosted` row, before it becomes a
/// [`PackagePlacement`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackPlacementRow {
    /// The package name as `package require` spells it.
    pub package: String,
    /// How the version is determined.
    pub version: Placement,
    /// Ambient (no `package require` needed) vs hosted.
    pub ambient: bool,
    /// The declaring line.
    pub line: u32,
}

/// A parsed `environment NAME { … }` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackEnvironment {
    /// The canonical id the block declares.
    pub id: String,
    /// `alias NAME` rows, in declaration order.
    pub aliases: Vec<String>,
    /// `display_name TEXT`, defaulting to the id.
    pub display_name: Option<String>,
    /// The validated `editor_identity ID`, when one resolved. An unknown
    /// id keeps the row (a notice) but drops the routing — §6.1's
    /// presentation rule, since an editor identity only decides which
    /// contributed language a document opens under.
    pub editor_identity: Option<EditorLanguageIdentityId>,
    /// The `core FAMILY RELEASE ?-build P?` selector, when declared.
    pub core: Option<CoreProfileSelector>,
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
                version_ceiling: None,
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
            help_terms: Vec::new(),
            provenance: tier.provenance(),
        }
    }
}

/// The single-release-line target set of one ladder release — the same
/// `[R·a0, next·a0)` window the compiled definitions use.
fn release_line(family: Family, release: Release) -> VersionSet {
    let axis = VersionAxisId::core(family);
    let requirement = match family
        .releases()
        .iter()
        .position(|candidate| *candidate == release)
        .and_then(|index| family.releases().get(index + 1))
    {
        Some(next) => format!("{}-{}", release.as_str(), next.as_str()),
        None => format!("{}-", release.as_str()),
    };
    VersionSet::from_requirements(axis, &[requirement])
        .unwrap_or_else(|_| VersionSet::empty(VersionAxisId::core(family)))
}

/// Parse one `environment NAME { … }` block, or reject it.
pub(super) fn parse(stmt: &Stmt, log: &mut Log) -> Option<PackEnvironment> {
    let name = stmt.word_text(1);
    if name.is_empty() || stmt.words.get(1).is_some_and(|word| word.braced) {
        log.say(stmt.line, "`environment` needs a name and a `{ … }` block");
        return None;
    }
    let Some(body) = stmt.arg(2) else {
        log.say(
            stmt.line,
            format!("`environment {name}` has no `{{ … }}` block; the block is rejected"),
        );
        return None;
    };
    if let Some(reserved) = reserved_name(name) {
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
        aliases: Vec::new(),
        display_name: None,
        editor_identity: None,
        core: None,
        placements: Vec::new(),
        world_policy: WorldPolicy::Open,
        file_extensions: Vec::new(),
        filenames: Vec::new(),
        signatures: Vec::new(),
        line: stmt.line,
    };
    let mut rejected = false;
    log.scoped(format!("environment {name}"), |log| {
        for row in block(body) {
            if !read_row(&mut environment, &row, log) {
                rejected = true;
            }
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

/// Read one row. `false` means the whole block is rejected — the §6.1
/// semantic class, which is what every unknown word in an environment
/// block is: this block says which world is closed and what is ambient in
/// it, so there is no decorative word here to drop safely.
fn read_row(environment: &mut PackEnvironment, stmt: &Stmt, log: &mut Log) -> bool {
    let words = &stmt.words;
    match stmt.word_text(0) {
        "display_name" => environment.display_name = Some(stmt.word_text(1).to_owned()),
        "alias" => match stmt.word_text(1) {
            "" => log.say(stmt.line, "`alias` needs a name"),
            alias => environment.aliases.push(alias.to_owned()),
        },
        "core" => return core_row(environment, stmt, log),
        "ambient" => return placement_row(environment, stmt, true, log),
        "hosted" => return placement_row(environment, stmt, false, log),
        "editor_identity" => {
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
        "file_extension" => {
            let raw = stmt.word_text(1);
            let extension = raw.trim_start_matches('.').to_ascii_lowercase();
            if extension.is_empty() || extension.contains('.') {
                log.say(
                    stmt.line,
                    format!("`file_extension {raw}` is not a single extension"),
                );
            } else {
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
        }
        "filename" => match stmt.word_text(1) {
            "" => log.say(stmt.line, "`filename` needs a basename"),
            name => environment.filenames.push(name.to_owned()),
        },
        "signature" => match stmt.word_text(1) {
            "" => log.say(stmt.line, "`signature` needs the text to look for"),
            text => environment.signatures.push(text.to_owned()),
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
    let Some(family) = Family::ALL
        .iter()
        .copied()
        .find(|family| family.name() == family_word)
    else {
        log.say(
            stmt.line,
            format!(
                "`core {family_word}` names no core family (`tcl`, `f5-irules`, `jim`); \
                 the environment block is rejected"
            ),
        );
        return false;
    };
    let release_word = stmt.word_text(2);
    let Some(release) = family
        .releases()
        .iter()
        .copied()
        .find(|release| release.as_str() == release_word)
    else {
        log.say(
            stmt.line,
            format!(
                "`core {family_word} {release_word}` names no release on the {family_word} \
                 ladder; the environment block is rejected"
            ),
        );
        return false;
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
    environment.core = Some(CoreProfileSelector {
        family,
        default_release: release,
        build,
    });
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

/// The compiled canonical id or alias `name` collides with, if any.
///
/// `pub(super)` so the evaluation loader's E-R2 provenance gate asks the
/// same question this block's own rejection does.
pub(super) fn reserved_name(name: &str) -> Option<String> {
    for definition in compiled_definitions() {
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
