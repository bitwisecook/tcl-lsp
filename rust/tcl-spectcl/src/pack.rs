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

//! A pack is a **logical unit, not a file**.
//!
//! `docs/design/spec-packs.md` is explicit about it: authors group however
//! they like — one big `.tclspec`, one per namespace, one per command — and
//! every file whose `speclib` names the same pack merges into one pack model
//! at load. This module is that merge, and the precedence rules around it:
//!
//! - **Merge order is sorted path order**, so the result does not depend on
//!   directory-iteration order, which no filesystem promises.
//! - **A command defined twice within one pack is a load-time diagnostic with
//!   the first definition winning**, never a silent overwrite.
//! - **Nearest tier wins**: a live Spec Studio override, then a workspace,
//!   user, or bundled pack. A pack name declared at more than one tier loads
//!   only from its nearest one, and the shadowed files are reported. This is
//!   whole-pack, not per-command — a user-tier pack is not a base a workspace
//!   pack patches, it is a different copy of the same pack, and merging them
//!   would produce a pack that exists on no one's disk.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tcl_registry::registry::CommandRegistry;

use crate::discovery::{Origin, PackFile, Tier};
use crate::loader::{Notice, Pack, PackCommand};

/// How loudly a notice should be shown. Every notice is a *degradation*, never
/// a failure — the pack still loads — so nothing here is an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Something the author wrote was dropped: an unknown word, a duplicate
    /// definition, an unreadable file. Worth fixing.
    Warning,
    /// Something the loader decided, correctly, that the author may not have
    /// expected: a shipped name left alone, a shadowed tier.
    Information,
}

/// One notice, carrying the file it belongs to so the server can publish it as
/// a diagnostic on that file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackNotice {
    /// The `.tclspec` the notice is about.
    pub path: PathBuf,
    /// 1-based line within that file. Line 1 for a whole-file notice.
    pub line: u32,
    /// Where in the pack the notice arose (`"command lsort"`, `"pack"`).
    pub context: String,
    /// What happened.
    pub message: String,
    /// How loudly to show it.
    pub severity: Severity,
}

impl PackNotice {
    fn from_loader(path: &Path, notice: &Notice) -> Self {
        Self {
            path: path.to_path_buf(),
            line: notice.line,
            context: notice.context.clone(),
            message: notice.message.clone(),
            severity: Severity::Warning,
        }
    }

    fn whole_file(path: &Path, message: impl Into<String>, severity: Severity) -> Self {
        Self {
            path: path.to_path_buf(),
            line: 1,
            context: "pack".to_owned(),
            message: message.into(),
            severity,
        }
    }
}

/// One pack, merged from every file that named it.
#[derive(Debug, Clone)]
pub struct MergedPack {
    /// The `speclib` name every contributing file agreed on.
    pub name: String,
    /// The DSL vocabulary version, from the first file in merge order.
    pub dsl_version: String,
    /// The tier the pack loaded from.
    pub tier: Tier,
    /// The files that contributed, in merge (sorted path) order.
    pub files: Vec<PathBuf>,
    /// The pack's human-readable name, from the first file that declares
    /// one (`display_name {IEEE 1801 UPF}`).
    pub display_name: Option<String>,
    /// The file extensions the pack's language is written under, merged
    /// first-declaration-wins across the pack's files.
    pub file_extensions: Vec<crate::loader::FileExtension>,
    /// The packages the pack declares ambient, merged across its files.
    ///
    /// Every row is kept, including two files naming the same package: they
    /// are floors, and [`CommandRegistry::ambient_package_floor`] takes the
    /// highest. Dropping one here would silently lower the floor instead.
    pub ambient_packages: Vec<crate::loader::AmbientPackage>,
    /// The merged commands: first definition of a name wins.
    pub commands: Vec<PackCommand>,
}

impl MergedPack {
    /// The command of that name, if the pack declares one.
    #[must_use]
    pub fn command(&self, name: &str) -> Option<&PackCommand> {
        self.commands.iter().find(|c| c.spec.name == name)
    }
}

/// Every pack a workspace loads, plus everything the load wanted to say.
#[derive(Debug, Clone, Default)]
pub struct PackSet {
    /// The merged packs, in name order.
    pub packs: Vec<MergedPack>,
    /// Every notice, from every file, in file-then-line order.
    pub notices: Vec<PackNotice>,
    /// Content key over every contributing file plus the vocabulary version
    /// and loader build — the identity this pack set installs under in the
    /// per-profile registry cache. `0` for an empty set, which is the
    /// "no packs" identity [`crate::install::registry_with_packs`] short-
    /// circuits on.
    pub key: u64,
}

impl PackSet {
    /// `true` when no pack contributed anything the registry would carry.
    ///
    /// Commands are not the only payload: a pack whose whole content is
    /// `ambient_package` rows still floors those packages for every document
    /// the pack is active in, so it is *not* empty. Counting only commands
    /// made [`crate::install::registry_with_packs`] short-circuit on such a
    /// pack and drop the floor with no notice — the silent-drop class this
    /// loader exists to make impossible.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.packs
            .iter()
            .all(|p| p.commands.is_empty() && p.ambient_packages.is_empty())
    }

    /// Every file that contributed to, or produced a notice about, this set —
    /// the files a reload must watch.
    #[must_use]
    pub fn files(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = self
            .packs
            .iter()
            .flat_map(|p| p.files.iter().cloned())
            .chain(self.notices.iter().map(|n| n.path.clone()))
            .collect();
        files.sort();
        files.dedup();
        files
    }

    /// The notices that belong to `path`.
    pub fn notices_for<'a>(&'a self, path: &Path) -> impl Iterator<Item = &'a PackNotice> {
        let path = path.to_path_buf();
        self.notices.iter().filter(move |n| n.path == path)
    }

    /// Every `(extension, dialect)` routing pair the set's packs declare —
    /// the rows with a `-dialect`, deduplicated first-pack-wins in the
    /// set's (name-sorted) pack order.
    #[must_use]
    pub fn extension_dialects(&self) -> Vec<(String, &'static str)> {
        let mut out: Vec<(String, &'static str)> = Vec::new();
        for pack in &self.packs {
            for row in &pack.file_extensions {
                if let Some(dialect) = row.dialect
                    && !out.iter().any(|(ext, _)| *ext == row.extension)
                {
                    out.push((row.extension.clone(), dialect));
                }
            }
        }
        out
    }
}

/// Load and merge every discovered file.
///
/// Reads each file, parses it through [`crate::cache::load_pack_cached`] (so
/// an unchanged pack costs a hash check), then groups by `speclib` name.
#[must_use]
pub fn load(files: &[PackFile]) -> PackSet {
    let (sources, notices) = read_sources(&tcl_lsp_core::vfs::NativeStore, files);
    load_sources(sources, notices)
}

/// Read every discovered file, turning an unreadable one into a notice rather
/// than dropping it silently.
///
/// Split out of [`load`] so [`crate::bundled::load_discovered`] can add the
/// embedded bundled sources to the same vec before the merge, instead of
/// merging twice and reconciling two content keys.
pub(crate) fn read_sources(
    store: &dyn tcl_lsp_core::vfs::SourceStore,
    files: &[PackFile],
) -> (Vec<(PackFile, String)>, Vec<PackNotice>) {
    let mut sources: Vec<(PackFile, String)> = Vec::with_capacity(files.len());
    let mut notices: Vec<PackNotice> = Vec::new();

    for file in files {
        match store.read_to_string(&file.path) {
            Ok(source) => sources.push((file.clone(), source)),
            Err(err) => notices.push(PackNotice::whole_file(
                &file.path,
                format!("cannot read pack file: {err}"),
                Severity::Warning,
            )),
        }
    }

    (sources, notices)
}

/// [`load`]'s merge, taken with the source text already in hand rather than
/// read from `file.path` on disk.
///
/// [`crate::bundled`]'s embedded-pack fallback is the one other caller: a
/// standalone binary with no `specs/` directory beside it (every
/// `tcl-lsp-server-<triple>` / `tcl-mcp-<triple>` / `tcl-<triple>` /
/// `f5-query-<triple>` release asset, until something stages one there —
/// the VSIX/JetBrains/Sublime bundles, a dev checkout, or
/// `TCL_LSP_SPEC_PACK_DIR`) has the eight shipped `.tclspec` sources compiled
/// in via `include_str!` but nowhere on disk to point a [`PackFile`] at.
/// Everything past "read the bytes" — cache lookup, `speclib` grouping,
/// tier-shadowing, duplicate-command detection, the content key — is
/// identical either way, so this is the one seam that seam splits on, not a
/// second implementation of the merge.
///
/// `notices` carries whatever the caller already collected before sources
/// were in hand (e.g. [`load`]'s unreadable-file notices); embedded sources
/// never fail to "read", so [`crate::bundled`] always passes an empty vec.
#[must_use]
pub(crate) fn load_sources(
    sources: Vec<(PackFile, String)>,
    mut notices: Vec<PackNotice>,
) -> PackSet {
    // Keyed from the bytes, before anything is parsed: the key must describe
    // the input, not what the loader made of it.
    let key = set_key(&sources);

    // Group by declared pack name. `BTreeMap` so the resulting pack order is
    // by name, deterministically. Each file is parsed exactly once here — the
    // merge below consumes these `Pack`s rather than re-reading the source.
    let mut by_name: BTreeMap<String, Vec<(PackFile, Pack)>> = BTreeMap::new();
    for (file, source) in sources {
        let pack = crate::cache::load_pack_cached(&source);
        if pack.name.is_empty() {
            // No `speclib` wrapper: nothing to merge, but the loader's
            // explanation of why still belongs on the file.
            for notice in &pack.notices {
                notices.push(PackNotice::from_loader(&file.path, notice));
            }
            continue;
        }
        by_name
            .entry(pack.name.clone())
            .or_default()
            .push((file, pack));
    }

    let mut packs = Vec::new();
    for (name, mut group) in by_name {
        // `PackFile` orders by (tier, path), which is exactly precedence then
        // merge order — one sort settles both questions.
        group.sort_by(|a, b| a.0.cmp(&b.0));
        let winning_tier = group[0].0.tier;
        let (winners, shadowed): (Vec<_>, Vec<_>) =
            group.into_iter().partition(|(f, _)| f.tier == winning_tier);

        for (file, _) in &shadowed {
            notices.push(PackNotice::whole_file(
                &file.path,
                format!(
                    "pack `{name}` also loads from the {} tier, which is nearer; \
                     this {} copy is not loaded",
                    winning_tier.label(),
                    file.tier.label()
                ),
                Severity::Information,
            ));
        }

        packs.push(merge_group(&name, winning_tier, winners, &mut notices));
    }

    // Cross-pack collisions, once every pack is merged — the only point where
    // all of them are known at once (issue #1637).
    notices.extend(cross_pack_command_notices(&packs));
    notices.extend(cross_pack_extension_notices(&packs));

    notices.sort_by(|a, b| {
        (&a.path, a.line, &a.context, &a.message).cmp(&(&b.path, b.line, &b.context, &b.message))
    });
    notices.dedup();

    let set = PackSet {
        packs,
        notices,
        key,
    };
    // Publish the packs' declared extension routing so dialect detection's
    // extension tier sees it — every consumer funnels through this merge
    // (bundled, discovered, server reloads), which is what makes a pack the
    // source of truth for its own extensions.
    tcl_registry::dialects::register_pack_extension_dialects(set.extension_dialects());
    set
}

/// Report every command name two *different* packs both claim.
///
/// The in-pack duplicate is caught by [`merge_group`] and the shipped-command
/// clash by [`collision_notices`]; between two packs there was nothing at all
/// before issue #1637 — the loser was dropped by [`installs_over`] and the
/// winner decided by pack-name sort order, in silence.
///
/// This walks the packs in exactly the order [`crate::install::install_into`]
/// does and applies the same rule, so the notice can never disagree with what
/// actually reached the registry: a later claim wins only if it says
/// `-override`, and either way the losing declaration is named.
///
/// # The vendor gate applies here too
///
/// [`crate::install::install_into`] skips a command whose `required_package`
/// names a closed-world vendor package the profile does not ship, which is
/// what lets the bundled tier carry all six EDA libraries at once: four of
/// them declare a `report_timing`, and they never meet because no profile
/// admits two vendors. A collision check that ignored that gate would report
/// every one of those as a clash — a dozen warnings on the *shipped* packs, in
/// every user's Problems panel, for something that cannot happen.
///
/// So two claims collide only when some profile would install both, which
/// [`could_collide`] asks exactly.
///
/// # Two shipped packs layering on purpose is not a collision either
///
/// `sdc_base` declares the SDC commands every EDA vendor implements, and each
/// vendor pack then declares its own richer spec for the same name. On a
/// Cadence profile both are admitted and `eda_cadence` wins on pack-name
/// order — which is the intended layering, not an accident. Reporting it would
/// put six warnings on the shipped packs that no user can act on.
///
/// So a collision between two **bundled** packs is left alone. Every other
/// combination is reported, including a workspace pack shadowing a bundled
/// one, which is exactly the case a user can do something about.
fn cross_pack_command_notices(packs: &[MergedPack]) -> Vec<PackNotice> {
    /// The claim currently holding a name.
    struct Claim {
        pack: String,
        file: PathBuf,
        line: u32,
        package: Option<&'static str>,
        tier: Tier,
    }

    let mut out = Vec::new();
    // Every claim that still stands for a name, not just the most recent one.
    //
    // One entry per name is not enough, because claims can be *mutually
    // exclusive*: several stand at once, one per vendor that could be live.
    // Three packs claiming `report_timing` — the first for Synopsys, the next
    // two for Cadence — leave the Synopsys claim standing while the two
    // Cadence claims genuinely collide with each other. A single-entry map
    // compares the third claim only against the first, finds them
    // vendor-disjoint, and reports nothing; the real collision goes unreported
    // (found reviewing #1637). So each claim is checked against every standing
    // claim and settles against the first it *could* collide with — which is
    // the one `install_into` would have let win, since both walk packs in the
    // same order.
    let mut claimed: BTreeMap<&'static str, Vec<Claim>> = BTreeMap::new();

    for pack in packs {
        for command in &pack.commands {
            let claim = Claim {
                pack: pack.name.clone(),
                file: command.file.clone(),
                line: command.line,
                package: command.spec.required_package,
                tier: pack.tier,
            };
            let standing = claimed.entry(command.spec.name).or_default();
            let Some(idx) = standing
                .iter()
                .position(|prior| could_collide(prior.package, command.spec.required_package))
            else {
                // Nothing standing is ever live in the same registry as this
                // one, so it shadows nothing and nothing shadows it: it joins
                // the standing set rather than replacing it.
                standing.push(claim);
                continue;
            };
            // Copied out before the standing set is mutated below.
            let prior_pack = standing[idx].pack.clone();
            let prior_file = standing[idx].file.clone();
            let prior_line = standing[idx].line;
            let prior_tier = standing[idx].tier;
            if prior_tier == Tier::Bundled && pack.tier == Tier::Bundled {
                // Shipped layering — see the note above.
                continue;
            }
            if command.overrides_shipped {
                // This one replaces the standing claim; the notice belongs on
                // the declaration that just lost its place.
                out.push(PackNotice {
                    path: prior_file.clone(),
                    line: prior_line,
                    context: format!("command {}", command.spec.name),
                    message: format!(
                        "`{}` is also declared by pack `{}` ({}:{}) with `-override`, \
                         which replaces this one",
                        command.spec.name,
                        pack.name,
                        command.file.display(),
                        command.line
                    ),
                    severity: Severity::Warning,
                });
                // Replaces the claim it displaced, in place — the other
                // standing claims for this name are for other vendors and are
                // untouched by an override aimed at this one.
                standing[idx] = claim;
            } else {
                out.push(PackNotice {
                    path: command.file.clone(),
                    line: command.line,
                    context: format!("command {}", command.spec.name),
                    message: format!(
                        "`{}` is already declared by pack `{prior_pack}` ({}:{prior_line}); \
                         that declaration wins and this one is not installed \
                         (declare it `-override` to replace it)",
                        command.spec.name,
                        prior_file.display()
                    ),
                    severity: Severity::Warning,
                });
            }
        }
    }
    out
}

/// Whether two commands' `required_package`s can ever be live in one registry.
///
/// The exact question [`crate::install::install_into`]'s vendor gate answers,
/// asked across every profile rather than for one: two declarations collide
/// only if some profile would admit both. Two different closed-world EDA
/// vendors never do, which is why the six shipped loadables can all declare a
/// `report_timing` without shadowing each other.
///
/// Identical requirements short-circuit to `true` — the common case, and one
/// no profile sweep could disagree with.
fn could_collide(a: Option<&'static str>, b: Option<&'static str>) -> bool {
    use tcl_registry::profile_queries::ProfileQueries as _;

    if a == b {
        return true;
    }
    tcl_dialect::DialectProfile::all()
        .iter()
        .any(|profile| profile.package_available(a) && profile.package_available(b))
}

/// Report every `file_extension` two different packs both claim.
///
/// One owner per extension is the invariant, and [`PackSet::extension_dialects`]
/// enforces it by dropping all but the first — silently, before issue #1637.
/// The loser matters more here than for a command: an extension routed to the
/// wrong dialect mis-lexes every file of that type.
///
/// Only rows carrying a `-dialect` can collide in the sense that matters, since
/// those are the ones `extension_dialects` publishes for routing.
fn cross_pack_extension_notices(packs: &[MergedPack]) -> Vec<PackNotice> {
    let mut out = Vec::new();
    // extension -> (pack name, dialect) of the row that owns the routing.
    let mut owner: BTreeMap<String, (String, &'static str)> = BTreeMap::new();

    for pack in packs {
        for row in &pack.file_extensions {
            let Some(dialect) = row.dialect else {
                continue;
            };
            match owner.get(&row.extension) {
                Some((prior_pack, prior_dialect)) => out.push(PackNotice {
                    // The row's *own* file, not the merged pack's first one:
                    // a logical pack can span several files, and `row.line` is
                    // a line in this one (issue #1637 review).
                    path: row.file.clone(),
                    line: row.line,
                    context: format!("file_extension {}", row.extension),
                    message: format!(
                        "`.{}` is already claimed by pack `{prior_pack}` (routing to \
                         `{prior_dialect}`); one extension has one owner, so this row's \
                         `-dialect {dialect}` is not used",
                        row.extension
                    ),
                    severity: Severity::Warning,
                }),
                None => {
                    owner.insert(row.extension.clone(), (pack.name.clone(), dialect));
                }
            }
        }
    }
    out
}

/// Merge one tier's files for one pack name, first-definition-wins.
fn merge_group(
    name: &str,
    tier: Tier,
    files: Vec<(PackFile, Pack)>,
    notices: &mut Vec<PackNotice>,
) -> MergedPack {
    let mut merged = MergedPack {
        name: name.to_owned(),
        dsl_version: String::new(),
        tier,
        files: files.iter().map(|(f, _)| f.path.clone()).collect(),
        display_name: None,
        file_extensions: Vec::new(),
        ambient_packages: Vec::new(),
        commands: Vec::new(),
    };
    // Where each command name was first defined, so the duplicate notice can
    // name the file that won rather than just saying "somewhere else".
    let mut first_seen: BTreeMap<&'static str, (PathBuf, u32)> = BTreeMap::new();
    let first_file = merged.files.first().cloned().unwrap_or_default();

    for (file, pack) in files {
        if merged.dsl_version.is_empty() {
            merged.dsl_version.clone_from(&pack.dsl_version);
        } else if !pack.dsl_version.is_empty() && pack.dsl_version != merged.dsl_version {
            notices.push(PackNotice::whole_file(
                &file.path,
                format!(
                    "pack `{name}` declares DSL version {} here and {} in {}; \
                     the first in merge order is used",
                    pack.dsl_version,
                    merged.dsl_version,
                    first_file.display()
                ),
                Severity::Warning,
            ));
        }
        for notice in &pack.notices {
            notices.push(PackNotice::from_loader(&file.path, notice));
        }
        if merged.display_name.is_none() {
            merged.display_name.clone_from(&pack.display_name);
        }
        for mut row in pack.file_extensions {
            // Same reason as the command loop below: the merge is the only
            // layer that knows which file of a multi-file pack a row came from
            // (found reviewing #1637).
            row.file.clone_from(&file.path);
            if !merged
                .file_extensions
                .iter()
                .any(|prior| prior.extension == row.extension)
            {
                merged.file_extensions.push(row);
            }
        }
        merged.ambient_packages.extend(pack.ambient_packages);
        for mut command in pack.commands {
            // The merge is the only layer that knows which file a command came
            // from, so this is where that gets recorded (issues #1637, #1638).
            command.file.clone_from(&file.path);
            if let Some((first_path, first_line)) = first_seen.get(command.spec.name) {
                notices.push(PackNotice {
                    path: file.path.clone(),
                    // The *ignored* declaration's own line, not line 1. The
                    // squiggle belongs on the duplicate the author can delete,
                    // not on the `speclib` header (issue #1638).
                    line: command.line,
                    context: format!("command {}", command.spec.name),
                    message: if *first_path == file.path {
                        // Same file: naming the path the reader already has
                        // open tells them nothing — the line does.
                        format!(
                            "`{}` is already defined on line {first_line} of this file; \
                             this definition is ignored",
                            command.spec.name
                        )
                    } else {
                        format!(
                            "`{}` is already defined in pack `{name}` by {}:{first_line}; \
                             this definition is ignored",
                            command.spec.name,
                            first_path.display()
                        )
                    },
                    severity: Severity::Warning,
                });
                continue;
            }
            first_seen.insert(command.spec.name, (file.path.clone(), command.line));
            merged.commands.push(command);
        }
    }
    merged
}

/// The content key for a whole pack set.
///
/// Covers, in order, every file's tier, path and byte content, plus the
/// vocabulary version and loader build — so moving a pack between tiers, or
/// upgrading the server, is as much a change as editing it.
fn set_key(sources: &[(PackFile, String)]) -> u64 {
    if sources.is_empty() {
        return 0;
    }
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    crate::cache::stamp_build(&mut hasher);
    for (file, source) in sources {
        hasher.update(&[file.tier as u8]);
        hasher.update(file.path.to_string_lossy().as_bytes());
        hasher.update(&[0]);
        hasher.update(source.as_bytes());
        hasher.update(&[0]);
    }
    // A pack set never keys as 0 — that value means "no packs" to the
    // registry-cache layer, and a real set must not be able to collide with it.
    match hasher.digest() {
        0 => 1,
        digest => digest,
    }
}

/// Every command in this set that collides with a name `registry` already
/// declares, and what the collision policy did about it.
///
/// **Shipped wins unless the pack says `-override`** — so a collision without
/// `-override` reports and drops, and one with it reports and replaces. Both
/// are reported: a silent override is exactly as surprising as a silent drop.
#[must_use]
pub fn collision_notices(packs: &PackSet, registry: &CommandRegistry) -> Vec<PackNotice> {
    let mut out = Vec::new();
    for pack in &packs.packs {
        for command in &pack.commands {
            if registry.get(command.spec.name).is_none() {
                continue;
            }
            let file = pack.files.first().cloned().unwrap_or_default();
            out.push(PackNotice {
                path: file,
                line: 1,
                context: format!("command {}", command.spec.name),
                message: if command.overrides_shipped {
                    format!(
                        "`{}` replaces the shipped command of that name (`-override`)",
                        command.spec.name
                    )
                } else {
                    format!(
                        "`{}` is already a shipped command; the shipped spec wins \
                         (declare it `-override` to replace it)",
                        command.spec.name
                    )
                },
                severity: Severity::Information,
            });
        }
    }
    out
}

/// `true` when this command should be installed over `registry`'s existing
/// entry — the one place the collision policy is decided, shared by
/// [`collision_notices`] and [`crate::install`] so the report and the
/// behaviour cannot drift.
pub(crate) fn installs_over(command: &PackCommand, registry: &CommandRegistry) -> bool {
    command.overrides_shipped || registry.get(command.spec.name).is_none()
}

/// The tier/origin pair as it reads in a log line: `workspace (.tcl-lsp/)`.
#[must_use]
pub fn describe_source(tier: Tier, origin: Origin) -> String {
    format!("{} ({})", tier.label(), origin.label())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tcl-spectcl-pack-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(&path, body).expect("write");
        path
    }

    /// Loading a pack writes a compiled-cache entry, and `cache`'s own tests
    /// count the entries under a redirected directory — so every test in this
    /// crate that loads a pack holds the same lock they do.
    fn cache_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::cache::REDIRECT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn workspace_file(path: PathBuf) -> PackFile {
        PackFile {
            tier: Tier::Workspace,
            path,
            origin: Origin::DotDir,
        }
    }

    #[test]
    fn files_naming_one_speclib_merge_into_one_pack() {
        let _cache = cache_guard();
        let dir = tmpdir("merge");
        let a = write(
            &dir,
            "a.tclspec",
            "speclib mylib 1 {\n  command mylib::alpha { arity 1 }\n}\n",
        );
        let b = write(
            &dir,
            "b.tclspec",
            "speclib mylib 1 {\n  command mylib::beta { arity 2 }\n}\n",
        );

        let set = load(&[workspace_file(b), workspace_file(a)]);
        assert_eq!(set.packs.len(), 1, "{:#?}", set.packs);
        let pack = &set.packs[0];
        assert_eq!(pack.name, "mylib");
        assert_eq!(pack.files.len(), 2);
        // Merge order is sorted path order regardless of the order in.
        assert!(pack.files[0].ends_with("a.tclspec"));
        let names: Vec<&str> = pack.commands.iter().map(|c| c.spec.name).collect();
        assert_eq!(names, vec!["mylib::alpha", "mylib::beta"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_command_defined_twice_in_one_pack_is_a_notice_and_the_first_wins() {
        let _cache = cache_guard();
        let dir = tmpdir("dupe");
        let a = write(
            &dir,
            "a.tclspec",
            "speclib mylib 1 {\n  command mylib::dup { arity 1 }\n}\n",
        );
        let b = write(
            &dir,
            "b.tclspec",
            "speclib mylib 1 {\n  command mylib::dup { arity 9 }\n}\n",
        );

        let set = load(&[workspace_file(a), workspace_file(b.clone())]);
        let pack = &set.packs[0];
        assert_eq!(pack.commands.len(), 1, "first definition wins");
        assert_eq!(pack.commands[0].spec.arity.min, 1);
        let dupe: Vec<&PackNotice> = set
            .notices
            .iter()
            .filter(|n| n.message.contains("already defined"))
            .collect();
        assert_eq!(dupe.len(), 1, "{:#?}", set.notices);
        assert_eq!(dupe[0].path, b, "the notice lands on the losing file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn distinct_speclib_names_stay_distinct_packs() {
        let _cache = cache_guard();
        let dir = tmpdir("distinct");
        let a = write(
            &dir,
            "a.tclspec",
            "speclib alpha 1 {\n  command a::one { arity 1 }\n}\n",
        );
        let b = write(
            &dir,
            "b.tclspec",
            "speclib beta 1 {\n  command b::one { arity 1 }\n}\n",
        );
        let set = load(&[workspace_file(a), workspace_file(b)]);
        let names: Vec<&str> = set.packs.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_nearer_tier_shadows_the_whole_pack_and_says_so() {
        let _cache = cache_guard();
        let dir = tmpdir("tiers");
        let ws = write(
            &dir,
            "ws/mylib.tclspec",
            "speclib mylib 1 {\n  command mylib::ws { arity 1 }\n}\n",
        );
        let studio = write(
            &dir,
            "studio/mylib.tclspec",
            "speclib mylib 1 {\n  command mylib::live { arity 2 }\n}\n",
        );
        let user = write(
            &dir,
            "user/mylib.tclspec",
            "speclib mylib 1 {\n  command mylib::user { arity 1 }\n}\n",
        );
        let set = load(&[
            PackFile {
                tier: Tier::StudioOverride,
                path: studio,
                origin: Origin::StudioOverride,
            },
            workspace_file(ws.clone()),
            PackFile {
                tier: Tier::User,
                path: user.clone(),
                origin: Origin::UserDir,
            },
        ]);
        assert_eq!(set.packs.len(), 1);
        let names: Vec<&str> = set.packs[0].commands.iter().map(|c| c.spec.name).collect();
        assert_eq!(names, vec!["mylib::live"]);
        assert!(
            set.notices
                .iter()
                .any(|n| n.path == user && n.message.contains("is not loaded"))
                && set
                    .notices
                    .iter()
                    .any(|n| n.path == ws && n.message.contains("Spec Studio override")),
            "{:#?}",
            set.notices
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn loader_notices_carry_their_file_and_line() {
        let _cache = cache_guard();
        let dir = tmpdir("notices");
        let a = write(
            &dir,
            "a.tclspec",
            "speclib mylib 1 {\n  command mylib::x {\n    nonsense 1\n  }\n}\n",
        );
        let set = load(&[workspace_file(a.clone())]);
        let notice = set
            .notices
            .iter()
            .find(|n| n.message.contains("nonsense"))
            .unwrap_or_else(|| panic!("{:#?}", set.notices));
        assert_eq!(notice.path, a);
        assert_eq!(notice.line, 3);
        assert_eq!(notice.severity, Severity::Warning);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unreadable_file_is_a_notice_not_a_panic() {
        let _cache = cache_guard();
        let dir = tmpdir("unreadable");
        let missing = dir.join("gone.tclspec");
        let set = load(&[workspace_file(missing.clone())]);
        assert!(set.packs.is_empty());
        assert_eq!(set.notices.len(), 1);
        assert_eq!(set.notices[0].path, missing);
        assert!(set.notices[0].message.contains("cannot read"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_set_key_tracks_content_and_never_collides_with_empty() {
        let _cache = cache_guard();
        let dir = tmpdir("key");
        let a = write(
            &dir,
            "a.tclspec",
            "speclib mylib 1 {\n  command mylib::x { arity 1 }\n}\n",
        );
        let before = load(&[workspace_file(a.clone())]).key;
        assert_ne!(
            before, 0,
            "a non-empty set never keys as the empty identity"
        );
        assert_eq!(
            before,
            load(&[workspace_file(a.clone())]).key,
            "the key is stable for unchanged content"
        );
        write(
            &dir,
            "a.tclspec",
            "speclib mylib 1 {\n  command mylib::x { arity 2 }\n}\n",
        );
        assert_ne!(before, load(&[workspace_file(a)]).key, "an edit rekeys");
        assert_eq!(load(&[]).key, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
