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
    /// `true` when no pack contributed a single command.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.packs.iter().all(|p| p.commands.is_empty())
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
}

/// Load and merge every discovered file.
///
/// Reads each file, parses it through [`crate::cache::load_pack_cached`] (so
/// an unchanged pack costs a hash check), then groups by `speclib` name.
#[must_use]
pub fn load(files: &[PackFile]) -> PackSet {
    let (sources, notices) = read_sources(files);
    load_sources(sources, notices)
}

/// Read every discovered file, turning an unreadable one into a notice rather
/// than dropping it silently.
///
/// Split out of [`load`] so [`crate::bundled::load_discovered`] can add the
/// embedded bundled sources to the same vec before the merge, instead of
/// merging twice and reconciling two content keys.
pub(crate) fn read_sources(files: &[PackFile]) -> (Vec<(PackFile, String)>, Vec<PackNotice>) {
    let mut sources: Vec<(PackFile, String)> = Vec::with_capacity(files.len());
    let mut notices: Vec<PackNotice> = Vec::new();

    for file in files {
        match std::fs::read_to_string(&file.path) {
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
/// `TCL_LSP_SPEC_PACK_DIR`) has the six shipped `.tclspec` sources compiled
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

    notices.sort_by(|a, b| {
        (&a.path, a.line, &a.context, &a.message).cmp(&(&b.path, b.line, &b.context, &b.message))
    });
    notices.dedup();

    PackSet {
        packs,
        notices,
        key,
    }
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
        commands: Vec::new(),
    };
    // Where each command name was first defined, so the duplicate notice can
    // name the file that won rather than just saying "somewhere else".
    let mut first_seen: BTreeMap<&'static str, PathBuf> = BTreeMap::new();
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
        for command in pack.commands {
            if let Some(first) = first_seen.get(command.spec.name) {
                notices.push(PackNotice {
                    path: file.path.clone(),
                    line: 1,
                    context: format!("command {}", command.spec.name),
                    message: format!(
                        "`{}` is already defined in pack `{name}` by {}; \
                         this definition is ignored",
                        command.spec.name,
                        first.display()
                    ),
                    severity: Severity::Warning,
                });
                continue;
            }
            first_seen.insert(command.spec.name, file.path.clone());
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
