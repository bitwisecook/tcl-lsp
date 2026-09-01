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

//! The **golden snapshot** of a loaded pack — what a shipped `.tclspec`
//! means, in a form that can be checked into the repository and compared on
//! every run.
//!
//! ## Why this exists
//!
//! Until the `one-loader` lane there were two implementations of "load a
//! pack", and the gate that a loader change could not silently alter a pack's
//! meaning was *the other loader*: `tests/eval_loader.rs` loaded all 24
//! shipped packs through both and demanded byte-identical snapshots. Deleting
//! the CST loader removes that oracle, so the proof has to be kept some other
//! way. It is kept here: the snapshots themselves are checked in, and every
//! test run recomputes and compares them. A change that alters what a shipped
//! pack means now has to be *written down* in the same commit
//! (`cargo xtask pack-goldens` regenerates), where a reviewer sees it.
//!
//! Where the two-loader gate was stronger: it compared two independent
//! readings *of the same build*, so a bug present in both readings was
//! invisible to it, but a divergence was caught without anyone updating a
//! file. Where the golden gate is stronger: it compares against a reading
//! from a *previous* build, which is the direction real regressions travel,
//! and it catches a change both halves of one build would agree on. The
//! same-build duality is not lost either — the loader's static fast path and
//! its interpreter route are two readings of one file, and
//! `tests/eval_loader.rs` still holds them byte-identical over the same 24
//! packs.
//!
//! ## The shape
//!
//! Pack-level facts and every notice appear in full. Each command appears as
//! one line: its name, the loader-level facts, and **digests** of two
//! exhaustive renderings — the whole [`CommandSpec`] debug form (the same
//! rendering `upgrade.rs`'s U9 round-trip and the fast-path gate compare) and
//! the pack's declared hooks. Digests rather than the text itself because the
//! text is 8.6 MB for the shipped corpus, in single lines of several
//! kilobytes: unreviewable, and a diff of it says nothing a human can read.
//! A digest costs the *reader* nothing — the comparison recomputes the full
//! rendering in-process, so a failure prints the offending command's complete
//! before/after, which is more than a checked-in blob would have shown.
//!
//! Function pointers are normalised out ([`normalise`]): a `CommandSpec`
//! carries resolver `fn` pointers whose addresses move with every build, and
//! *which* function a pack installed is covered by the `hooks` digest beside
//! them, which is stable text.

use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

use crate::discovery::{Origin, Tier};
use crate::loader::Pack;

/// The language-neutral owner for the repository-relative directories that
/// hold every shipped `.tclspec`. CI's changed-path classifier reads this
/// same manifest, so adding a pack directory cannot silently omit the real
/// `SpecTcl` execution lane.
pub const PACK_DIRS_MANIFEST: &str = include_str!("../data/shipped-pack-dirs.txt");

/// Every shipped-pack directory in manifest order.
pub fn shipped_pack_dirs() -> impl Iterator<Item = &'static str> {
    PACK_DIRS_MANIFEST
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// One repository-shipped pack file and the production-shaped metadata the
/// execution corpus uses to discover and resolve it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShippedPackFile {
    /// Absolute path under the supplied repository root.
    pub path: PathBuf,
    /// Discovery precedence used when this file is loaded.
    pub tier: Tier,
    /// Discovery rule reported in load notices.
    pub origin: Origin,
    /// Registry profile under which its commands are executable.
    pub dialect: &'static str,
}

/// Tier and origin are properties of the manifest directory, owned here with
/// the directory scan. A new manifest directory must choose its semantics
/// explicitly instead of inheriting a test-local default.
fn shipped_pack_location(directory: &str) -> io::Result<(Tier, Origin)> {
    match directory {
        "specs" => Ok((Tier::Bundled, Origin::Bundled)),
        "docs/design/spec-dsl-examples" | "docs/design/spec-dsl-examples/external" => {
            Ok((Tier::Workspace, Origin::DotDir))
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "shipped-pack directory `{other}` has no tier/origin metadata; add it to \
                 tcl_spectcl::golden::shipped_pack_location"
            ),
        )),
    }
}

/// The dialect under which a shipped pack's commands are executable. This is
/// part of the inventory because both registry comparison and live corpus
/// execution must resolve the same vendor surface.
fn shipped_pack_dialect(path: &Path) -> &'static str {
    match path.file_stem().and_then(|stem| stem.to_str()) {
        Some("eda_xilinx" | "sdc_base") => "xilinx-eda-tcl",
        Some("upf" | "eda_synopsys") => "synopsys-eda-tcl",
        Some("eda_microchip") => "microchip-libero-eda-tcl",
        Some("eda_cadence") => "cadence-eda-tcl",
        Some("eda_quartus") => "intel-quartus-eda-tcl",
        Some("eda_mentor") => "mentor-eda-tcl",
        Some("irules-http-header") => "f5-irules",
        _ => "tcl9.1",
    }
}

fn inventory_io_error(action: &str, path: &Path, error: &io::Error) -> io::Error {
    io::Error::new(
        error.kind(),
        format!("{action} `{}`: {error}", path.display()),
    )
}

/// Fallible inventory construction. No manifest directory or entry may
/// disappear silently: callers either receive the complete inventory or one
/// contextual error naming the directory whose scan failed.
fn try_shipped_pack_inventory(repo_root: &Path) -> io::Result<Vec<ShippedPackFile>> {
    let mut files = Vec::new();
    for directory in shipped_pack_dirs() {
        let (tier, origin) = shipped_pack_location(directory)?;
        let directory_path = repo_root.join(directory);
        let entries = std::fs::read_dir(&directory_path).map_err(|error| {
            inventory_io_error(
                "failed to read shipped-pack directory",
                &directory_path,
                &error,
            )
        })?;
        let mut here = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| {
                inventory_io_error(
                    "failed to read an entry in shipped-pack directory",
                    &directory_path,
                    &error,
                )
            })?;
            let path = entry.path();
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == crate::PACK_EXTENSION)
            {
                here.push(path);
            }
        }
        here.sort();
        files.extend(here.into_iter().map(|path| ShippedPackFile {
            dialect: shipped_pack_dialect(&path),
            path,
            tier,
            origin,
        }));
    }
    Ok(files)
}

/// Every `.tclspec` under [`PACK_DIRS_MANIFEST`], sorted per directory, with
/// its tier, origin, and executable dialect from this single owner.
///
/// # Panics
///
/// Panics with the repository root and failing directory when any manifest
/// directory cannot be opened or any directory entry cannot be read. A
/// partial inventory would make the execution gates vacuous, so there is no
/// best-effort form of this API.
#[must_use]
pub fn shipped_pack_inventory(repo_root: &Path) -> Vec<ShippedPackFile> {
    try_shipped_pack_inventory(repo_root).unwrap_or_else(|error| {
        panic!(
            "failed to build shipped-pack inventory under `{}`: {error}",
            repo_root.display()
        )
    })
}

/// The path-only projection retained for golden generation and the real-Tcl
/// syntax oracle. All discovery still flows through [`shipped_pack_inventory`].
#[must_use]
pub fn shipped_packs(repo_root: &Path) -> Vec<PathBuf> {
    shipped_pack_inventory(repo_root)
        .into_iter()
        .map(|pack| pack.path)
        .collect()
}

/// Where a pack's checked-in golden lives, given the repository root.
#[must_use]
pub fn golden_path(repo_root: &Path, pack: &Path) -> PathBuf {
    let stem = pack
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    repo_root
        .join("rust/tcl-spectcl/tests/golden")
        .join(format!("{stem}.snap"))
}

/// The rendering version. Bump when [`render`] changes shape, so a stale
/// golden fails loudly on the header line rather than confusingly deep in a
/// command row.
const FORMAT: u32 = 1;

/// The golden snapshot of one loaded pack.
#[must_use]
pub fn render(pack: &Pack) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# spectcl golden snapshot v{FORMAT}");
    let _ = writeln!(
        out,
        "# regenerate with `cargo xtask pack-goldens`; do not edit by hand"
    );
    let _ = writeln!(
        out,
        "pack {} dsl {} display {:?} load_error {:?}",
        pack.name, pack.dsl_version, pack.display_name, pack.load_error
    );
    let _ = writeln!(out, "target_dependent {}", pack.target_dependent);
    let _ = writeln!(out, "file_extensions {:?}", pack.file_extensions);
    let _ = writeln!(out, "provides {:?}", pack.provides);
    let _ = writeln!(out, "co_provides {:?}", pack.co_provides);
    let _ = writeln!(out, "ambient_packages {:?}", pack.ambient_packages);
    let _ = writeln!(
        out,
        "environments {}",
        normalise(&format!("{:?}", pack.environments))
    );
    let _ = writeln!(
        out,
        "dialects {}",
        normalise(&format!("{:?}", pack.dialects))
    );
    let _ = writeln!(out, "registrations {}", pack.registrations.len());
    let _ = writeln!(out, "notices {}", pack.notices.len());
    for notice in &pack.notices {
        let _ = writeln!(
            out,
            "notice {} {} {} {}",
            notice.context,
            notice.line,
            notice.class.name(),
            one_line(&notice.message)
        );
    }
    let _ = writeln!(out, "commands {}", pack.commands.len());
    for command in &pack.commands {
        let _ = writeln!(
            out,
            "command {} line {} overrides {} degraded {} spec {} hooks {} grammar {}",
            command.spec.name,
            command.line,
            command.overrides_shipped,
            command.degraded,
            digest(&spec_rendering(pack, command.spec.name).unwrap_or_default()),
            digest(&format!("{:?}", command.hooks)),
            digest(&format!("{:?}", command.clause_grammar)),
        );
    }
    out
}

/// The exhaustive rendering one command's `spec` digest is taken over — the
/// text a failing comparison prints so a reviewer can see *what* changed.
#[must_use]
pub fn spec_rendering(pack: &Pack, name: &str) -> Option<String> {
    let command = pack.commands.iter().find(|c| c.spec.name == name)?;
    Some(normalise(&format!("{:?}", command.spec)))
}

/// Replace every `0x…` run with a placeholder.
///
/// A `CommandSpec` holds resolver function pointers, and `{:?}` prints their
/// addresses, which move with the binary. `Some(fn)` versus `None` — whether
/// a resolver was installed at all — survives, and *which* resolver a pack
/// asked for is the `hooks` digest's job, since a `HookDecl` is stable text.
#[must_use]
pub fn normalise(rendering: &str) -> String {
    let mut out = String::with_capacity(rendering.len());
    let mut rest = rendering;
    while let Some(at) = rest.find("0x") {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 2..];
        let end = tail
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(tail.len());
        if end == 0 {
            out.push_str("0x");
            rest = tail;
            continue;
        }
        out.push_str("fn");
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// A notice message as one golden line: real newlines and tabs escaped, so a
/// multi-line message cannot forge extra rows.
fn one_line(message: &str) -> String {
    message
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// The digest form used for every rendering in a golden line.
fn digest(text: &str) -> String {
    format!("{:016x}", xxhash_rust::xxh3::xxh3_64(text.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointers_normalise_and_ordinary_hex_survives() {
        assert_eq!(normalise("Some(0x55f984912f50)"), "Some(fn)");
        assert_eq!(normalise("None"), "None");
        // A `0x` that is not followed by hex digits is left alone.
        assert_eq!(normalise("prefix 0xz"), "prefix 0xz");
        // Two pointers in one rendering.
        assert_eq!(normalise("a 0xdeadbeef b 0x1"), "a fn b fn");
    }

    #[test]
    fn a_multi_line_notice_stays_one_golden_line() {
        assert_eq!(one_line("a\nb\tc"), "a\\nb\\tc");
    }

    #[test]
    fn shipped_inventory_owns_location_and_dialect_metadata() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .to_path_buf();
        let inventory = shipped_pack_inventory(&root);
        assert!(inventory.len() >= 24, "{inventory:#?}");

        for pack in &inventory {
            let relative = pack.path.strip_prefix(&root).expect("repository path");
            if relative.starts_with("specs") {
                assert_eq!((pack.tier, pack.origin), (Tier::Bundled, Origin::Bundled));
            } else {
                assert_eq!((pack.tier, pack.origin), (Tier::Workspace, Origin::DotDir));
            }
        }

        let dialect = |stem: &str| {
            inventory
                .iter()
                .find(|pack| pack.path.file_stem().is_some_and(|value| value == stem))
                .map(|pack| pack.dialect)
        };
        assert_eq!(dialect("upf"), Some("synopsys-eda-tcl"));
        assert_eq!(dialect("eda_xilinx"), Some("xilinx-eda-tcl"));
        assert_eq!(dialect("irules-http-header"), Some("f5-irules"));
        assert_eq!(dialect("geturl"), Some("tcl9.1"));
    }

    #[test]
    fn shipped_inventory_fails_closed_when_a_manifest_directory_is_missing() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "tcl-spectcl-missing-inventory-root-{}-{unique}",
            std::process::id()
        ));
        assert!(
            !root.exists(),
            "test root unexpectedly exists: {}",
            root.display()
        );

        let failure = std::panic::catch_unwind(|| shipped_pack_inventory(&root))
            .expect_err("a missing manifest directory must fail closed");
        let message = failure
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| failure.downcast_ref::<&str>().copied())
            .unwrap_or("non-string panic");
        assert!(
            message.contains("failed to build shipped-pack inventory"),
            "{message}"
        );
        assert!(
            message.contains(&root.join("specs").display().to_string()),
            "{message}"
        );
        assert!(
            message.contains("failed to read shipped-pack directory"),
            "{message}"
        );
    }

    #[test]
    fn a_rendered_snapshot_is_stable_and_names_its_commands() {
        let pack = crate::loader::evaluate_pack(
            "speclib golden 2.0 {\n  command golden::a { arity 1 }\n  \
             command golden::b { arity 2 }\n}\n",
        );
        let first = render(&pack);
        assert_eq!(
            first,
            render(&crate::loader::evaluate_pack(
                "speclib golden 2.0 {\n  command golden::a { arity 1 }\n  \
             command golden::b { arity 2 }\n}\n",
            ))
        );
        assert!(first.contains("command golden::a line 2"), "{first}");
        assert!(first.contains("commands 2"), "{first}");
    }
}
