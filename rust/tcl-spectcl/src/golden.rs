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
use std::path::{Path, PathBuf};

use crate::loader::Pack;

/// The directories, relative to the repository root, that hold every
/// `.tclspec` the repository ships: the bundled packs and the design's
/// worked examples.
///
/// One inventory, shared by the golden gate, its regeneration verb
/// (`cargo xtask pack-goldens`) and the fast-path gate, so no two of them can
/// disagree about what "every shipped pack" means.
pub const PACK_DIRS: &[&str] = &[
    "specs",
    "docs/design/spec-dsl-examples",
    "docs/design/spec-dsl-examples/external",
];

/// Every `.tclspec` under [`PACK_DIRS`], sorted per directory.
#[must_use]
pub fn shipped_packs(repo_root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in PACK_DIRS {
        let Ok(entries) = std::fs::read_dir(repo_root.join(dir)) else {
            continue;
        };
        let mut here: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == crate::PACK_EXTENSION)
            })
            .collect();
        here.sort();
        files.extend(here);
    }
    files
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
    fn a_rendered_snapshot_is_stable_and_names_its_commands() {
        let pack = crate::loader::evaluate_pack(
            "speclib golden 2.0 {\n  command golden::a { arity 1 }\n  \
             command golden::b { arity 2 }\n}\n",
        );
        let first = render(&pack);
        assert_eq!(first, render(&crate::loader::evaluate_pack(
            "speclib golden 2.0 {\n  command golden::a { arity 1 }\n  \
             command golden::b { arity 2 }\n}\n",
        )));
        assert!(first.contains("command golden::a line 2"), "{first}");
        assert!(first.contains("commands 2"), "{first}");
    }
}
