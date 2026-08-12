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
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The whole stack — discovery, merge, cache, install — over the **real**
//! ported packs in `docs/design/spec-dsl-examples/`.
//!
//! The unit tests in each module use small hand-written fixtures, which is
//! right for pinning one rule at a time and wrong for catching the thing that
//! actually breaks a layered design: a rule that holds on a two-line pack and
//! not on a thousand-line one. These packs are the corpus the DSL was designed
//! against — `lsort`, `switch`, `string`, the `TclOO` and snit definers, `upvar`,
//! an iRules taint command, plus four third-party libraries — so running the
//! runtime path over them is the closest thing to a production load this
//! repository can stage.

use std::path::{Path, PathBuf};

use tcl_spectcl::discovery::{DiscoveryOptions, Tier, discover};
use tcl_spectcl::pack::{self, PackSet};

/// The ported packs, from the repository rather than a fixture directory:
/// a port that stops loading should fail *here*, not silently drift from a
/// copy nobody updates.
fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/design/spec-dsl-examples")
        .canonicalize()
        .expect("the ported spec-DSL examples")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tcl-spectcl-workspace-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Copy every `.tclspec` under `examples_dir()` into `<root>/.tcl-lsp/`,
/// flattened, and return how many landed.
fn stage_examples(root: &Path) -> usize {
    let target = root.join(".tcl-lsp");
    std::fs::create_dir_all(&target).expect("pack dir");
    let mut staged = 0;
    let mut stack = vec![examples_dir()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read examples").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "tclspec") {
                let name = path.file_name().expect("file name");
                std::fs::copy(&path, target.join(name)).expect("copy pack");
                staged += 1;
            }
        }
    }
    assert!(staged >= 10, "expected the ported corpus, found {staged}");
    staged
}

fn load_workspace(root: &Path) -> PackSet {
    let files = discover(&DiscoveryOptions {
        workspace_roots: vec![root.to_path_buf()],
        // Pin both non-workspace tiers at directories that do not exist: the
        // developer's own `~/.config/tcl-lsp/specs` must not be able to change
        // this test's answer.
        user_dir: Some(root.join("no-user-tier")),
        bundled_dir: Some(root.join("no-bundled-tier")),
        ..DiscoveryOptions::default()
    });
    assert!(files.iter().all(|f| f.tier == Tier::Workspace));
    pack::load(&files)
}

/// Every ported pack loads, and the files sharing a `speclib` name merge.
#[test]
fn the_ported_corpus_loads_and_merges_by_speclib_name() {
    let root = scratch("corpus");
    let staged = stage_examples(&root);
    let set = load_workspace(&root);

    let names: Vec<&str> = set.packs.iter().map(|p| p.name.as_str()).collect();
    assert!(
        names.contains(&"tcl"),
        "the core ports merge into one `tcl` pack: {names:?}"
    );
    let core = set
        .packs
        .iter()
        .find(|p| p.name == "tcl")
        .expect("the `tcl` pack");
    assert!(
        core.files.len() > 1,
        "`tcl` is a multi-file pack: {:?}",
        core.files
    );
    // Merge order is sorted path order, deterministically.
    let mut sorted = core.files.clone();
    sorted.sort();
    assert_eq!(core.files, sorted);

    let total: usize = set.packs.iter().map(|p| p.commands.len()).sum();
    assert!(
        total >= staged,
        "{staged} files produced only {total} commands"
    );
    // Two loads of the same tree are the same load.
    assert_eq!(set.key, load_workspace(&root).key);

    let _ = std::fs::remove_dir_all(&root);
}

/// The corpus is *mostly* ports of shipped commands, so installing it is a
/// large-scale test of the collision policy: shipped wins, every collision is
/// reported, and the shipped specs come out unchanged.
#[test]
fn installing_the_corpus_leaves_every_shipped_command_alone() {
    let root = scratch("collisions");
    stage_examples(&root);
    let set = load_workspace(&root);

    let plain = tcl_registry::registry_for_dialect("tcl9.1");
    let with_packs = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.1", &set);

    let collisions = pack::collision_notices(&set, with_packs);
    assert!(
        !collisions.is_empty(),
        "the ports redeclare shipped commands by construction"
    );
    // None of the ports claims `-override`, so every collision must have been
    // resolved in the shipped spec's favour.
    for notice in &collisions {
        assert!(
            notice.message.contains("shipped spec wins"),
            "unexpected override: {notice:?}"
        );
    }
    for name in ["lsort", "switch", "string", "upvar", "foreach", "return"] {
        let Some(shipped) = plain.get(name) else {
            continue;
        };
        let after = with_packs.get(name).expect("still present");
        assert_eq!(
            shipped.arity, after.arity,
            "`{name}` was changed by a pack that did not claim it"
        );
        assert_eq!(shipped.traits, after.traits, "`{name}` traits changed");
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// The commands the ports declare that the registry does *not* ship — the
/// third-party libraries — are the ones a pack is for, and they land.
#[test]
fn commands_the_registry_does_not_ship_are_installed() {
    let root = scratch("newcomers");
    stage_examples(&root);
    let set = load_workspace(&root);

    let plain = tcl_registry::registry_for_dialect("tcl9.1");
    let with_packs = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.1", &set);

    let newcomers: Vec<&str> = set
        .packs
        .iter()
        .flat_map(|p| p.commands.iter())
        .map(|c| c.spec.name)
        .filter(|name| plain.get(name).is_none())
        .collect();
    assert!(
        !newcomers.is_empty(),
        "the third-party ports declare commands the registry has never seen"
    );
    for name in &newcomers {
        assert!(
            with_packs.get(name).is_some(),
            "`{name}` did not reach the registry"
        );
        assert!(
            with_packs.command_names().any(|n| n == *name),
            "`{name}` is not enumerable, so completion would miss it"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// The cache changes nothing. Loading the whole corpus twice — cold then warm —
/// must produce the same packs, the same commands, and the same notices.
#[test]
fn a_warm_cache_produces_the_identical_load() {
    let root = scratch("cache");
    stage_examples(&root);
    let cache = root.join("cache");
    tcl_spectcl::cache::redirect_for_test(Some((cache.clone(), false)));

    let cold = load_workspace(&root);
    assert!(
        std::fs::read_dir(&cache).is_ok(),
        "a cold load populated the cache directory"
    );
    let warm = load_workspace(&root);

    let shape = |set: &PackSet| {
        use std::fmt::Write as _;
        let mut out = String::new();
        for pack in &set.packs {
            let _ = writeln!(out, "{} v{}", pack.name, pack.dsl_version);
            for command in &pack.commands {
                let _ = writeln!(out, "  {}", command.spec.name);
            }
        }
        for notice in &set.notices {
            let _ = writeln!(out, "  {} {}", notice.line, notice.message);
        }
        out
    };
    assert_eq!(shape(&cold), shape(&warm));
    assert_eq!(cold.key, warm.key);

    // And with the cache thrown away mid-flight, the load is still the same —
    // the disposability contract, at corpus scale.
    let _ = std::fs::remove_dir_all(&cache);
    assert_eq!(shape(&cold), shape(&load_workspace(&root)));

    tcl_spectcl::cache::redirect_for_test(None);
    let _ = std::fs::remove_dir_all(&root);
}

/// A workspace copy of a pack shadows the user-tier copy of the same name, as
/// a whole pack — the "nearest wins" rule, with the real corpus on both sides.
#[test]
fn a_workspace_pack_shadows_the_user_tier_copy_of_the_same_name() {
    let root = scratch("tiers");
    stage_examples(&root);
    let user = root.join("user-tier");
    std::fs::create_dir_all(&user).expect("user tier");
    std::fs::write(
        user.join("tcl.tclspec"),
        "speclib tcl 1.0 {\n  command user_tier_only { arity 1 }\n}\n",
    )
    .expect("write user pack");

    let files = discover(&DiscoveryOptions {
        workspace_roots: vec![root.clone()],
        user_dir: Some(user),
        bundled_dir: Some(root.join("no-bundled-tier")),
        ..DiscoveryOptions::default()
    });
    let set = pack::load(&files);

    let core = set
        .packs
        .iter()
        .find(|p| p.name == "tcl")
        .expect("the `tcl` pack");
    assert_eq!(core.tier, Tier::Workspace);
    assert!(
        core.command("user_tier_only").is_none(),
        "the shadowed tier contributes nothing, not even a command the winner lacks"
    );
    assert!(
        set.notices
            .iter()
            .any(|n| n.message.contains("is not loaded")),
        "and the shadowing is reported: {:#?}",
        set.notices
    );

    let _ = std::fs::remove_dir_all(&root);
}
