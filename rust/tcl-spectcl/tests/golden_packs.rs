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

//! **The golden-snapshot gate** — a loader change cannot silently alter what
//! a shipped pack means.
//!
//! Every `.tclspec` the repository ships loads and is rendered by
//! [`tcl_spectcl::golden::render`]; the rendering must equal the snapshot
//! checked in beside this file. A deliberate change is recorded by running
//! `cargo xtask pack-goldens`, which writes the same rendering through the
//! same function — so the diff a reviewer reads *is* the change in meaning.
//!
//! This replaced the two-loader byte-identity gate when the CST loader was
//! deleted. `tcl_spectcl::golden`'s module documentation says precisely what
//! that trade gains and gives up; the short version is that the two-loader
//! gate compared two readings of one build (blind to a bug both shared) and
//! this one compares against a reading from an *earlier* build (the direction
//! regressions travel), while `eval_loader.rs`'s fast-path gate keeps a
//! same-build duality over the same 24 packs.

use std::path::{Path, PathBuf};

use tcl_spectcl::golden;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// The first differing line of two renderings, and — when the line names a
/// command — the full before/after of that command's exhaustive rendering,
/// which is the thing the golden holds only a digest of.
fn explain(golden: &str, fresh: &str, pack: &tcl_spectcl::Pack) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for (index, (was, now)) in golden.lines().zip(fresh.lines()).enumerate() {
        if was == now {
            continue;
        }
        let _ = writeln!(out, "line {}:\n  golden: {was}\n  now   : {now}", index + 1);
        if let Some(name) = now
            .strip_prefix("command ")
            .and_then(|r| r.split(' ').next())
            && let Some(rendering) = golden::spec_rendering(pack, name)
        {
            let _ = writeln!(
                out,
                "  `{name}` now renders as:\n    {rendering}\n  \
                 (the golden holds a digest of this; run `cargo xtask pack-goldens` \
                 to record the new value)"
            );
        }
        return out;
    }
    let _ = writeln!(
        out,
        "line counts differ: golden {} vs now {}",
        golden.lines().count(),
        fresh.lines().count()
    );
    out
}

#[test]
fn every_shipped_pack_still_loads_to_its_golden_snapshot() {
    let root = repo_root();
    let packs = golden::shipped_packs(&root);
    assert!(
        packs.len() >= 24,
        "the inventory must cover the shipped packs; found {packs:?}"
    );

    let mut commands = 0_usize;
    let mut notices = 0_usize;
    for path in &packs {
        let source = std::fs::read_to_string(path).expect("readable pack");
        let pack = tcl_spectcl::evaluate_pack(&source);
        let fresh = golden::render(&pack);
        let golden_path = golden::golden_path(&root, path);
        let recorded = std::fs::read_to_string(&golden_path).unwrap_or_else(|err| {
            panic!(
                "{}: no golden snapshot at {} ({err}) — run `cargo xtask pack-goldens`",
                path.display(),
                golden_path.display()
            )
        });
        assert!(
            recorded == fresh,
            "{} no longer loads to its golden snapshot.\n{}",
            path.display(),
            explain(&recorded, &fresh, &pack)
        );
        commands += pack.commands.len();
        notices += pack.notices.len();
    }

    println!(
        "golden gate: {} packs, {commands} commands, {notices} notices held to their \
         checked-in snapshots",
        packs.len()
    );
    assert!(commands >= 800, "only {commands} commands covered");
}

/// The rendering with the three facts the upgrade is *meant* to move masked
/// out, so everything else — every command's `spec`, `hooks` and `grammar`
/// digest included — is the comparison.
///
/// - `dsl 1.1` → `dsl 2.0` is the whole point of the rewrite.
/// - A `file_extension … -dialect NAME` row is pack-scope data in 1.x and an
///   `environment NAME -extend { … }` claim in 2.0, so the pack-level list
///   empties and the environment list gains a claim. The two are checked
///   against each other by [`the_extension_claims_moved`] rather than
///   ignored.
/// - Every declaration's source line moves when the rewrite spends more
///   lines saying the same thing. It is provenance, not meaning.
fn same_but_the_moved_facts(rendered: &str) -> String {
    rendered
        .lines()
        .filter(|line| !line.starts_with("file_extensions ") && !line.starts_with("environments "))
        .map(|line| mask_after(&mask_after(line, " dsl "), " line "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `line` with the word following `key` replaced by `<any>`.
fn mask_after(line: &str, key: &str) -> String {
    let Some((head, tail)) = line.split_once(key) else {
        return line.to_owned();
    };
    match tail.split_once(' ') {
        Some((_, rest)) => format!("{head}{key}<any> {rest}"),
        None => format!("{head}{key}<any>"),
    }
}

/// Every extension a 1.x pack claimed still reaches the same environment
/// after the rewrite — at pack scope when the row named no dialect, and as
/// that environment's own `-extend` claim when it did.
fn the_extension_claims_moved(legacy: &tcl_spectcl::Pack, modern: &tcl_spectcl::Pack) {
    for claim in &legacy.file_extensions {
        let Some(dialect) = claim.dialect.as_deref() else {
            assert!(
                modern
                    .file_extensions
                    .iter()
                    .any(|c| c.extension == claim.extension),
                "`{}` lost its pack-scope extension claim",
                claim.extension
            );
            continue;
        };
        let moved = modern.environments.iter().any(|environment| {
            environment.id == dialect
                && environment.extends
                && environment
                    .file_extensions
                    .iter()
                    .any(|c| *c.extension == *claim.extension)
        });
        assert!(
            moved,
            "`{}`'s claim on `{dialect}` did not become an `-extend` block",
            claim.extension
        );
    }
}

/// **The upgrade is a spelling change, and this is what says so.** Every
/// shipped pack, rewritten to the 2.0 vocabulary by `tcl spec upgrade`,
/// loads to the same registry as the 1.x source it came from — the same
/// exhaustive rendering the golden holds a digest of, not merely the same
/// command count. The facts the rewrite is *meant* to move are checked
/// as a translation instead of as equality; see
/// [`same_but_the_moved_facts`].
///
/// It runs the 2.0 source **through the interpreter** as well, with the
/// static fast path off, because "a pack is a Tcl program" is only true if
/// the VM can execute one to the same answer. Three readings, one result.
#[test]
fn every_shipped_pack_upgrades_to_2_0_and_loads_identically() {
    use tcl_spectcl::{EvalOptions, UpgradeOptions, UpgradeStatus, upgrade_source};

    let root = repo_root();
    // The interpreter route is what the fast path exists to avoid: 20k
    // statements of pure vocabulary blow the production wall clock by
    // design. This gate wants the comparison, not the budget behaviour
    // (`a_budget_blowing_loop_fails_closed_naming_the_axis` covers that),
    // so it buys the time.
    let interpreted = EvalOptions {
        static_fast_path: false,
        config: tcl_spec_hooks::pack_eval::PackEvalConfig {
            budget: tcl_engine_api::Budget::of_commands(2_000_000_000)
                .with_wall_clock(std::time::Duration::from_mins(30))
                .with_max_value_bytes(512 * 1024 * 1024),
        },
        ..EvalOptions::default()
    };
    let mut upgraded_packs = 0_usize;
    for path in &golden::shipped_packs(&root) {
        let legacy = std::fs::read_to_string(path).expect("readable pack");
        let outcome = upgrade_source(&legacy, &UpgradeOptions::default());
        assert!(
            matches!(
                outcome.status,
                UpgradeStatus::Upgraded | UpgradeStatus::AlreadyCurrent
            ),
            "{}: {:?} {:?}",
            path.display(),
            outcome.status,
            outcome.refusals
        );
        assert!(
            outcome.above_target.is_empty(),
            "{}: rows above the declared vocabulary: {:?}",
            path.display(),
            outcome.above_target
        );
        if matches!(outcome.status, UpgradeStatus::Upgraded) {
            upgraded_packs += 1;
        }

        let before = tcl_spectcl::evaluate_pack(&legacy);
        let was = same_but_the_moved_facts(&golden::render(&before));
        let now = tcl_spectcl::evaluate_pack(&outcome.source);
        the_extension_claims_moved(&before, &now);
        let rendered = same_but_the_moved_facts(&golden::render(&now));
        assert!(
            was == rendered,
            "{}: the 2.0 rewrite loads differently.\n{}",
            path.display(),
            explain(&was, &rendered, &now)
        );
        let run = tcl_spectcl::evaluate_pack_with(&outcome.source, &interpreted);
        the_extension_claims_moved(&before, &run);
        let rendered = same_but_the_moved_facts(&golden::render(&run));
        assert!(
            was == rendered,
            "{}: the 2.0 rewrite loads differently through the interpreter.\n{}",
            path.display(),
            explain(&was, &rendered, &run)
        );
    }
    assert!(
        upgraded_packs >= 8,
        "only {upgraded_packs} packs had 1.x rows to rewrite"
    );
}

/// A golden with no pack behind it would never be looked at again — a pack
/// renamed or deleted without its snapshot. The regeneration verb removes
/// them; this is the gate that notices.
#[test]
fn no_golden_snapshot_is_orphaned() {
    let root = repo_root();
    let expected: Vec<String> = golden::shipped_packs(&root)
        .iter()
        .filter_map(|pack| {
            golden::golden_path(&root, pack)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect();
    let dir = root.join("rust/tcl-spectcl/tests/golden");
    let mut orphans: Vec<String> = std::fs::read_dir(&dir)
        .expect("the golden directory")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("snap"))
                && !expected.contains(name)
        })
        .collect();
    orphans.sort();
    assert!(
        orphans.is_empty(),
        "golden snapshots with no shipped pack behind them: {orphans:?} — \
         run `cargo xtask pack-goldens`"
    );
}
