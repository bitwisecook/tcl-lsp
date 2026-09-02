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
use tcl_spectcl::loader::{EvalOptions, Notice, Pack, evaluate_pack, evaluate_pack_in};

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

/// Gate A: `evaluate_pack(export(evaluate_pack(src)))` is `evaluate_pack(src)`.
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
        let pack = evaluate_pack(&source);
        let (exported, losses) = export_pack_reporting(&pack);
        assert!(
            losses.is_empty(),
            "{}: canonical export lost {losses:#?}",
            path.display()
        );
        let reloaded = evaluate_pack(&exported);

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
    [
        "fleet.tclspec",
        "rows.tclspec",
        "table.tclspec",
        "environments.tclspec",
    ]
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

        let reloaded = evaluate_pack(&exported);
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
    assert!(packs >= 4, "only {packs} templated packs exported");
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

/// The expansion of a pack that shares one version between its
/// environments carries the **version**, not the variable (issue #1643).
///
/// Gate B already proves the export reloads to the evaluated snapshot, but
/// two empty environment lists would satisfy that as happily as two
/// correct ones — an unsubstituted `$libver` rejects the block, and the
/// rejection round-trips. So the assertion here is on the text.
#[test]
fn a_shared_environment_version_expands_to_the_version() {
    let source = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/templated/environments.tclspec"),
    )
    .expect("the environments fixture");
    let pack = evaluate_pack(&source);
    assert!(pack.load_error.is_none(), "{:#?}", pack.notices);
    assert_eq!(pack.environments.len(), 2, "{:#?}", pack.environments);

    let exported = export_pack_reporting(&pack).0;
    let rows: Vec<String> = exported
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    assert!(
        rows.iter().any(|row| row == "ambient envlib 3.2"),
        "the ambient row carries the version: {exported}"
    );
    assert!(
        rows.iter().any(|row| row == "hosted envlib 3.2-"),
        "and so does the hosted one: {exported}"
    );
    assert!(
        !exported.contains("libver"),
        "the variable itself does not survive the expansion: {exported}"
    );
}

/// The 2.0 word batch (P2-H): `provides`, `co_provides`,
/// `dynamic_surface`/`unknown_members`, and the `environment -extend`
/// block all round-trip through gate A's machinery — export, reload,
/// identical snapshot, idempotent text.
#[test]
fn the_new_two_point_oh_words_round_trip_through_gate_a() {
    let source = "speclib probe 2.0 {\n\
                  provides upf 1.0 2.1\n\
                  co_provides Tk -requires-exact tk -when {without TK_NO_DEPRECATED}\n\
                  environment synopsys-eda-tcl -extend {\n\
                  \x20   file_extension upfx -name {Probe UPF Extension}\n\
                  \x20   ambient upf_extras 1.0\n\
                  }\n\
                  command demo {\n\
                  \x20   arity 1\n\
                  \x20   dynamic_surface\n\
                  }\n\
                  command duo {\n\
                  \x20   arity 1\n\
                  \x20   unknown_members\n\
                  }\n\
                  }\n";
    let pack = evaluate_pack(source);
    assert!(pack.notices.is_empty(), "{:#?}", pack.notices);
    let (exported, losses) = export_pack_reporting(&pack);
    assert!(losses.is_empty(), "{losses:#?}");
    let reloaded = evaluate_pack(&exported);
    assert_eq!(snapshot(&pack), snapshot(&reloaded));
    assert_eq!(notice_keys(&pack), notice_keys(&reloaded));
    assert_eq!(reloaded.provides.len(), 1);
    assert_eq!(reloaded.co_provides.len(), 1);
    assert_eq!(reloaded.environments.len(), 1);
    assert!(reloaded.environments[0].extends);
    let twice = export_pack_reporting(&reloaded).0;
    assert_eq!(twice, exported, "export is not idempotent");
}

/// An `include`-assembled pack exports as its expansion — the included
/// statements inline, no `include` row — and the export reloads
/// context-free to the same snapshot.
#[test]
fn an_included_fragment_exports_as_its_expansion() {
    let context = tcl_spectcl::IncludeContext::new(|name| match name {
        "extra.frag" => Ok("command extra {\n arity 2\n}\n".to_owned()),
        other => Err(format!("no such fragment `{other}`")),
    });
    let pack = evaluate_pack_in(
        "speclib probe 2.0 {\n include extra.frag\n command demo { arity 1 }\n}\n",
        &EvalOptions::default(),
        Some(std::rc::Rc::new(context)),
    );
    assert!(pack.notices.is_empty(), "{:#?}", pack.notices);
    let (exported, losses) = export_pack_reporting(&pack);
    assert!(losses.is_empty(), "{losses:#?}");
    assert!(!exported.contains("include"), "{exported}");
    assert!(exported.contains("command extra {"), "{exported}");
    // Context-free reload: the expansion needs no resolver.
    let reloaded = evaluate_pack(&exported);
    assert_eq!(snapshot(&pack), snapshot(&reloaded));
    assert_eq!(notice_keys(&pack), notice_keys(&reloaded));
}

/// The seven ratified words round-trip through gate A's machinery: export,
/// reload, identical snapshot, idempotent text. Their rows are captured
/// verbatim like every other, so this is the gate that proves a reader
/// added without an export arm cannot slip through.
#[test]
fn the_ratified_words_round_trip_through_gate_a() {
    let source = r"
speclib probe 2.0 {
    command probe::collect {
        arity 0
        result_stability Volatile
        data_collection -native HTTP_COLLECT
        side_switch_target Server
        event_handler_priority -default 500 -min 0 -max 1000 -warn-implicit
        event_requirement_form {append} -only-in {HTTP_REQUEST} {
            client_side yes
        }
        body_scope {
            name {probe body}
            command top {
                arity 1..2
                subcommand set { arity 1 }
            }
        }
        subcommand line {
            arity 0
            result_stability ReferentiallyTransparent
        }
    }
}
";
    let pack = evaluate_pack(source);
    assert!(pack.notices.is_empty(), "{:#?}", pack.notices);
    let (exported, losses) = export_pack_reporting(&pack);
    assert!(losses.is_empty(), "{losses:#?}");
    let reloaded = evaluate_pack(&exported);
    assert_eq!(snapshot(&pack), snapshot(&reloaded));
    assert_eq!(notice_keys(&pack), notice_keys(&reloaded));
    assert!(reloaded.commands[0].spec.body_scope.is_some());
    assert!(reloaded.commands[0].spec.result_stability.is_some());
    let twice = export_pack_reporting(&reloaded).0;
    assert_eq!(twice, exported, "export is not idempotent");
}
