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

//! **The canonical-export round-trip gates** — design E §15.1 (E-R11).
//!
//! Two claims, one per gate:
//!
//! - **Gate A — canonical source ↔ snapshot is a bijection.** Every
//!   `.tclspec` the repository ships loads, exports, and reloads to the same
//!   snapshot: same commands, same `CommandSpec` for each, same notices.
//! - **Gate B — expansion is total.** A *templated* pack (a `proc` driven by
//!   a `foreach` over a data table, the shape design E exists for) exports as
//!   canonical source whose reload equals the evaluated snapshot. That is the
//!   affordance `spectcl_expand` sells: read the expansion, not the loop.
//!
//! ## What "the same snapshot" excludes, and why
//!
//! Line numbers, and only line numbers. An export is not a source rewriter —
//! it writes registration calls in evaluation order with no comments and no
//! provenance markers — so a row's line is *derived on reload* and moves
//! (dramatically so for an expansion, where a loop body's single row becomes
//! one row per iteration). Every other loader-level fact is compared: the
//! declaration's `-override`, the §6.1 `degraded` flag, the hooks, the clause
//! grammar, the complete `CommandSpec`, and the notice set with its contexts
//! and classes.
//!
//! The rendering is the exhaustive one from `eval_loader.rs` — the same
//! `CommandSpec` debug form the upgrade tool's U9 round-trip compares — minus
//! the per-command line.

use std::path::{Path, PathBuf};

use tcl_spectcl::export::export_pack_reporting;
use tcl_spectcl::loader::{Notice, Pack, evaluate_pack, load_pack};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// Every `.tclspec` the repository ships — the same inventory the
/// equivalence gate scans, so the two cannot drift.
fn inventory() -> Vec<PathBuf> {
    let root = repo_root();
    let mut files = Vec::new();
    for dir in [
        root.join("specs"),
        root.join("docs/design/spec-dsl-examples"),
        root.join("docs/design/spec-dsl-examples/external"),
    ] {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut here: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == tcl_spectcl::PACK_EXTENSION)
            })
            .collect();
        here.sort();
        files.extend(here);
    }
    files
}

/// The exhaustive snapshot rendering, line-free: every command's complete
/// `CommandSpec` plus the loader-level per-command facts.
fn snapshot(pack: &Pack) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "pack {} dsl {} display {:?} load_error {:?} target_dependent {}",
        pack.name, pack.dsl_version, pack.display_name, pack.load_error, pack.target_dependent
    );
    let _ = writeln!(out, "file_extensions {:?}", pack.file_extensions);
    let _ = writeln!(out, "ambient_packages {:?}", pack.ambient_packages);
    let _ = writeln!(out, "environments {:?}", pack.environments);
    let _ = writeln!(out, "dialects {:?}", pack.dialects);
    for command in &pack.commands {
        let _ = writeln!(
            out,
            "command {} overrides {} degraded {}",
            command.spec.name, command.overrides_shipped, command.degraded
        );
        let _ = writeln!(out, "  hooks {:?}", command.hooks);
        let _ = writeln!(out, "  clause_grammar {:?}", command.clause_grammar);
        let _ = writeln!(out, "  spec {:?}", command.spec);
    }
    scrub_lines(&out)
}

/// Blind the rendering to every line number it carries.
///
/// The per-command line is left out above; the ones inside a `FileExtension`,
/// an `AmbientPackage`, or a `HookDecl` are inside a derived `Debug`, so they
/// are scrubbed from the text instead. Applied to both sides of every
/// comparison, so a line is invisible and nothing else is.
fn scrub_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("line: ") {
        out.push_str(&rest[..at]);
        out.push_str("line: _");
        rest = &rest[at + "line: ".len()..];
        let digits = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        rest = &rest[digits..];
    }
    out.push_str(rest);
    out
}

/// One notice as the comparison sees it — everything but its line.
fn notice_key(notice: &Notice) -> (String, String, String) {
    (
        notice.context.clone(),
        notice.class.name().to_owned(),
        notice.message.clone(),
    )
}

fn notice_keys(pack: &Pack) -> Vec<(String, String, String)> {
    let mut keys: Vec<_> = pack.notices.iter().map(notice_key).collect();
    keys.sort();
    keys
}

/// The first line at which two renderings differ, for a readable failure.
fn first_diff(a: &str, b: &str) -> String {
    for (index, (left, right)) in a.lines().zip(b.lines()).enumerate() {
        if left != right {
            let at = left
                .bytes()
                .zip(right.bytes())
                .position(|(l, r)| l != r)
                .unwrap_or(left.len().min(right.len()));
            let window = |s: &str| {
                let from = (0..=at.saturating_sub(120).min(s.len()))
                    .rev()
                    .find(|&i| s.is_char_boundary(i))
                    .unwrap_or(0);
                let to = ((at + 280).min(s.len())..=s.len())
                    .find(|&i| s.is_char_boundary(i))
                    .unwrap_or(s.len());
                s.get(from..to).unwrap_or("").to_owned()
            };
            return format!(
                "line {} (byte {at}):\n  source   : …{}…\n  exported : …{}…",
                index + 1,
                window(left),
                window(right)
            );
        }
    }
    format!(
        "line counts differ: source {} vs exported {}",
        a.lines().count(),
        b.lines().count()
    )
}

/// Gate A: `load_pack(export(load_pack(src)))` is `load_pack(src)`.
#[test]
fn round_trip_gate_a_every_shipped_pack_exports_and_reloads_to_the_same_snapshot() {
    let files = inventory();
    assert!(
        files.len() >= 24,
        "the inventory must cover the shipped packs; found {files:?}"
    );

    let mut packs = 0_usize;
    let mut commands = 0_usize;
    let mut registrations = 0_usize;
    for path in files {
        let source = std::fs::read_to_string(&path).expect("readable pack");
        let pack = load_pack(&source);
        let (exported, losses) = export_pack_reporting(&pack);
        assert!(
            losses.is_empty(),
            "{}: canonical export lost {losses:#?}",
            path.display()
        );
        let reloaded = load_pack(&exported);

        let before = snapshot(&pack);
        let after = snapshot(&reloaded);
        assert!(
            before == after,
            "{}: snapshot changed across export at {}",
            path.display(),
            first_diff(&before, &after)
        );
        assert!(
            notice_keys(&pack) == notice_keys(&reloaded),
            "{}: notices changed across export\n  source  : {:#?}\n  exported: {:#?}",
            path.display(),
            notice_keys(&pack),
            notice_keys(&reloaded),
        );

        // The export is itself canonical: a second round trip is a no-op on
        // the *text*, which is the byte-stability half of E-R11.
        let twice = export_pack_reporting(&reloaded).0;
        assert!(
            twice == exported,
            "{}: export is not idempotent at {}",
            path.display(),
            first_diff(&exported, &twice)
        );

        packs += 1;
        commands += pack.commands.len();
        registrations += count(&pack.registrations);
    }
    println!(
        "export gate A: {packs} packs, {commands} commands, \
         {registrations} registration calls round-tripped"
    );
    assert!(packs >= 24, "only {packs} packs round-tripped");
    assert!(commands >= 800, "only {commands} commands round-tripped");
}

/// Every registration call in a record, nested ones included.
fn count(registrations: &[tcl_spectcl::Registration]) -> usize {
    registrations
        .iter()
        .map(|reg| 1 + count(reg.body()))
        .sum::<usize>()
}

/// The fixture packs written as *programs* — the shape gate B exists for.
fn templated() -> Vec<(&'static str, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/templated");
    ["fleet.tclspec", "rows.tclspec", "table.tclspec"]
        .into_iter()
        .map(|name| {
            (
                name,
                std::fs::read_to_string(dir.join(name)).expect("a templated fixture"),
            )
        })
        .collect()
}

/// Gate B: `export(evaluate_pack(src))` is canonical source whose reload is
/// the evaluated snapshot.
#[test]
fn round_trip_gate_b_a_templated_pack_exports_as_its_expansion() {
    let mut packs = 0_usize;
    let mut commands = 0_usize;
    for (name, source) in templated() {
        let evaluated = evaluate_pack(&source);
        assert!(
            evaluated.load_error.is_none(),
            "{name}: {:#?}",
            evaluated.notices
        );
        assert!(!evaluated.commands.is_empty(), "{name}: registered nothing");

        let (exported, losses) = export_pack_reporting(&evaluated);
        assert!(losses.is_empty(), "{name}: expansion lost {losses:#?}");

        // The expansion is *canonical*: no statement in it is a general-Tcl
        // one. (A `$` or a `[` may legitimately survive *inside* a word — a
        // prose example, a synopsis — so the check is on statement heads, not
        // on the text.)
        for statement in exported.lines().map(str::trim_start) {
            let head = statement.split_whitespace().next().unwrap_or("");
            assert!(
                !matches!(
                    head,
                    "proc" | "foreach" | "set" | "if" | "while" | "for" | "lmap" | "eval"
                ),
                "{name}: the expansion still carries a `{head}` statement:\n{exported}"
            );
        }

        let reloaded = load_pack(&exported);
        let before = snapshot(&evaluated);
        let after = snapshot(&reloaded);
        assert!(
            before == after,
            "{name}: the expansion is not the evaluated snapshot at {}",
            first_diff(&before, &after)
        );
        assert!(
            notice_keys(&evaluated) == notice_keys(&reloaded),
            "{name}: notices changed across expansion\n  evaluated: {:#?}\n  exported : {:#?}",
            notice_keys(&evaluated),
            notice_keys(&reloaded),
        );

        // And the expansion evaluates to itself: exporting a canonical pack
        // through either loader is the same text.
        assert!(
            export_pack_reporting(&evaluate_pack(&exported)).0 == exported,
            "{name}: re-evaluating the expansion changed it"
        );

        packs += 1;
        commands += evaluated.commands.len();
    }
    println!("export gate B: {packs} templated packs, {commands} expanded commands");
    assert!(packs >= 3, "only {packs} templated packs exported");
    assert!(commands >= 9, "only {commands} expanded commands");
}

/// A programmed pack's expansion says what the loop registered, one
/// declaration per iteration, with the loop's own data in place.
#[test]
fn the_expansion_names_every_registration_the_loop_made() {
    let source = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/templated/fleet.tclspec"),
    )
    .expect("the fleet fixture");
    let exported = export_pack_reporting(&evaluate_pack(&source)).0;
    for name in ["alpha", "beta", "gamma", "delta"] {
        assert!(
            exported.contains(&format!("command math::fleet::{name} {{")),
            "{exported}"
        );
    }
    // The `speclib` header is the pack's own, not the newest vocabulary:
    // raising a declared vocabulary is `spec upgrade`'s job, not export's.
    assert!(exported.starts_with("speclib fleet 2.0 {\n"), "{exported}");
}
