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

    let plain = tcl_registry::model::static_context_for("tcl9.1").commands();
    let with_packs = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.1", &set);

    let collisions = pack::collision_notices(&set, &with_packs);
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

    let plain = tcl_registry::model::static_context_for("tcl9.1").commands();
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

// ─────────────────────── the stamp rejection rule ───────────────────────

/// A pack command stamped with `lassign`'s own codegen hook: `vendor::unpack
/// LIST VAR…` is the builtin by another name, if `alias_of` says so.
fn unpack_pack(alias_of: Option<&str>) -> String {
    let alias = alias_of.map_or_else(String::new, |target| format!("        alias_of {target}\n"));
    format!(
        "speclib vendor 2.0 {{\n    \
             command vendor::unpack {{\n        \
                 arity 2..\n        \
                 arg 0 -role Value\n        \
                 arg 1 -role VarWrite\n\
         {alias}        \
                 codegen_hook -native Lassign\n    \
             }}\n\
         }}\n"
    )
}

/// Load `source` as the one pack file of `tier` under `trust`.
fn load_one(
    name: &str,
    source: &str,
    tier: Tier,
    trust: tcl_dialect::model::WorkspaceTrust,
) -> PackSet {
    let root = scratch(name);
    let path = root.join("vendor.tclspec");
    std::fs::write(&path, source).expect("write pack");
    let origin = match tier {
        Tier::Bundled => tcl_spectcl::discovery::Origin::Bundled,
        Tier::User => tcl_spectcl::discovery::Origin::UserDir,
        Tier::StudioOverride => tcl_spectcl::discovery::Origin::StudioOverride,
        Tier::Workspace => tcl_spectcl::discovery::Origin::DotDir,
    };
    pack::load_under(
        &[tcl_spectcl::PackFile {
            tier,
            path,
            origin,
            dependency_tier: None,
        }],
        trust,
    )
}

/// The notices the stamp rule raised: the warnings on `command vendor::unpack`.
fn stamp_notices(set: &PackSet) -> Vec<&tcl_spectcl::PackNotice> {
    set.notices
        .iter()
        .filter(|n| n.context == "command vendor::unpack" && n.message.contains(" refused for "))
        .collect()
}

fn unpack(set: &PackSet) -> &tcl_spectcl::PackCommand {
    set.packs[0]
        .command("vendor::unpack")
        .expect("the command loads whatever happens to its stamp")
}

/// Rule 1 and rule 2 at once: a trusted workspace pack stamps a pack command
/// that names no target. The stamp is dropped with one warning on the
/// command's row, and the warning names the provenance and the `alias_of`
/// target the stamp would have had to sit on — the shipped command whose own
/// spec carries `CodegenHookId::Lassign`.
#[test]
fn a_workspace_stamp_without_alias_of_is_refused_and_names_the_target() {
    use tcl_dialect::model::WorkspaceTrust;
    let set = load_one(
        "stamp-no-alias",
        &unpack_pack(None),
        Tier::Workspace,
        WorkspaceTrust::Trusted,
    );
    let notices = stamp_notices(&set);
    assert_eq!(notices.len(), 1, "{:#?}", set.notices);
    assert_eq!(notices[0].severity, tcl_spectcl::pack::Severity::Warning);
    assert_eq!(
        notices[0].line,
        unpack(&set).line,
        "on the command's own row"
    );
    assert_eq!(
        notices[0].message,
        "`codegen_hook Lassign` refused for `vendor::unpack`: a trusted workspace pack may not \
         name a codegen catalogue member; the stamp would have to sit on `alias_of lassign`"
    );
    assert_eq!(unpack(&set).spec.codegen_hook, None, "the stamp is dropped");
}

/// Rule 2 alone: naming the right target does not let a non-bundled pack
/// stamp. Every tier the gate refuses says so with its own provenance, and
/// the command keeps its `alias_of` — only the stamp goes.
#[test]
fn a_workspace_stamp_with_alias_of_is_refused_by_the_tier_gate() {
    use tcl_dialect::model::WorkspaceTrust;
    let source = unpack_pack(Some("lassign"));
    for (tier, trust, label) in [
        (
            Tier::Workspace,
            WorkspaceTrust::Trusted,
            "a trusted workspace",
        ),
        (
            Tier::Workspace,
            WorkspaceTrust::Untrusted,
            "an untrusted workspace",
        ),
        (Tier::User, WorkspaceTrust::Trusted, "a user"),
        (
            Tier::StudioOverride,
            WorkspaceTrust::Trusted,
            "a Spec Studio override",
        ),
    ] {
        let set = load_one("stamp-tier-gate", &source, tier, trust);
        let notices = stamp_notices(&set);
        assert_eq!(notices.len(), 1, "{label}: {:#?}", set.notices);
        assert_eq!(
            notices[0].message,
            format!(
                "`codegen_hook Lassign` refused for `vendor::unpack`: {label} pack may not name \
                 a codegen catalogue member; only a bundled pack may carry `alias_of lassign`'s \
                 own stamp"
            )
        );
        let command = unpack(&set);
        assert_eq!(command.spec.codegen_hook, None, "{label}");
        assert_eq!(command.spec.alias_of, Some("lassign"), "{label}");
    }
}

/// Rule 3: a refusal costs the author no analysis fact. The refused
/// command's arity, argument roles and target survive the load and the
/// install, and the stripped spec differs from the loader's only in the
/// dropped stamp.
#[test]
fn a_refused_stamp_costs_no_analysis_fact() {
    use tcl_dialect::model::WorkspaceTrust;
    let source = unpack_pack(Some("lassign"));
    let set = load_one(
        "stamp-costs-nothing",
        &source,
        Tier::Workspace,
        WorkspaceTrust::Trusted,
    );
    assert_eq!(stamp_notices(&set).len(), 1, "{:#?}", set.notices);

    let declared = tcl_spectcl::evaluate_pack(&source);
    let as_written = declared
        .command("vendor::unpack")
        .expect("the pack declares it")
        .spec;
    assert_eq!(
        as_written.codegen_hook,
        Some(tcl_registry::hooks::CodegenHookId::Lassign),
        "the loader reads the stamp as written"
    );
    let stripped = unpack(&set).spec;
    let without_the_stamp = tcl_registry::CommandSpec {
        codegen_hook: None,
        ..as_written.clone()
    };
    assert_eq!(
        format!("{stripped:?}"),
        format!("{without_the_stamp:?}"),
        "only the stamp differs"
    );

    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &set);
    let installed = registry.get("vendor::unpack").expect("installed");
    assert_eq!(installed.codegen_hook, None);
    assert_eq!(installed.arity.min, 2);
    assert_eq!(installed.alias_of, Some("lassign"));
    let words = ["$pair", "first"];
    assert_eq!(
        registry.arg_indices_for_role("vendor::unpack", &words, tcl_registry::ArgRole::Value),
        vec![0]
    );
    assert_eq!(
        registry.arg_indices_for_role("vendor::unpack", &words, tcl_registry::ArgRole::VarWrite),
        vec![1]
    );
}

/// The one admitted shape: a bundled pack whose command declares `alias_of
/// lassign` may carry `lassign`'s own stamp, loaded through the bundled
/// door on a `specs/` directory and installed as written. The negative: the
/// same pack naming `alias_of lsort` — a shipped command that carries no
/// such stamp — is refused by rule 1 even at the bundled tier, and the
/// refusal still names `lassign`.
#[test]
fn a_bundled_stamp_on_an_alias_of_target_is_admitted() {
    let root = scratch("stamp-bundled");
    let specs = root.join("specs");
    std::fs::create_dir_all(&specs).expect("specs dir");
    std::fs::write(specs.join("vendor.tclspec"), unpack_pack(Some("lassign"))).expect("write");

    let set = tcl_spectcl::bundled::load_from(&specs);
    assert_eq!(set.packs[0].tier, Tier::Bundled);
    assert!(stamp_notices(&set).is_empty(), "{:#?}", set.notices);
    assert_eq!(
        unpack(&set).spec.codegen_hook,
        Some(tcl_registry::hooks::CodegenHookId::Lassign),
        "admitted: the stamp is lassign's own, and the pack ships with the server"
    );
    let registry = tcl_spectcl::bundled::registry_for_dialect_from("tcl8.6", &set);
    assert_eq!(
        registry
            .get("vendor::unpack")
            .expect("installed")
            .codegen_hook,
        Some(tcl_registry::hooks::CodegenHookId::Lassign)
    );

    std::fs::write(specs.join("vendor.tclspec"), unpack_pack(Some("lsort"))).expect("rewrite");
    let wrong = tcl_spectcl::bundled::load_from(&specs);
    let notices = stamp_notices(&wrong);
    assert_eq!(notices.len(), 1, "{:#?}", wrong.notices);
    assert_eq!(
        notices[0].message,
        "`codegen_hook Lassign` refused for `vendor::unpack`: `alias_of lsort` names a shipped \
         command that does not carry it; the stamp would have to sit on `alias_of lassign`"
    );
    assert_eq!(unpack(&wrong).spec.codegen_hook, None);

    let _ = std::fs::remove_dir_all(&root);
}

/// A `runtime_backing tcl-body {-pack-text …}` is reported at load — once,
/// as information, on its command's row — because the body travels with the
/// pack and goes stale without anyone touching it. The other backings, a body
/// read from the library's own source among them, draw nothing.
#[test]
fn a_pack_text_backing_is_reported_at_load() {
    let root = scratch("pack-text");
    let dir = root.join(".tcl-lsp");
    std::fs::create_dir_all(&dir).expect("pack dir");
    std::fs::write(
        dir.join("probe.tclspec"),
        "speclib probe 2.1 {\n\
         command probe::file { arity 0; runtime_backing tcl-body {-package-source init.tcl} }\n\
         command probe::host { arity 0; runtime_backing host-native }\n\
         command probe::text { arity 0; runtime_backing tcl-body {-pack-text {return 1}} }\n\
         }\n",
    )
    .expect("write pack");
    let set = load_workspace(&root);

    assert_eq!(set.notices.len(), 1, "{:#?}", set.notices);
    let notice = &set.notices[0];
    assert!(notice.message.contains("-pack-text"), "{notice:?}");
    assert_eq!(notice.severity, pack::Severity::Information);
    assert_eq!(notice.context, "command probe::text");
    assert_eq!(notice.line, 4, "the command's own row");

    let _ = std::fs::remove_dir_all(&root);
}

// ─────────────────────── the capability gate ───────────────────────

/// A pack command that says everything the capability gate polices: it names
/// `lassign`'s own stamp, says it is `alias_of lassign`, and declares the
/// builtin as its backing.
fn packaged_pack(speclib: &str) -> String {
    format!(
        "speclib {speclib} 1.0 {{\n    \
             command {speclib}::unpack {{\n        \
                 arity 2..\n        \
                 arg 0 -role Value\n        \
                 arg 1 -role VarWrite\n        \
                 alias_of lassign\n        \
                 runtime_backing shipped-builtin lassign\n        \
                 codegen_hook -native Lassign\n    \
             }}\n\
         }}\n"
    )
}

/// A workspace as `tcl pkg install` leaves one. The root package `myapp`
/// requires `direct` and, for development, `devdep`; `direct` requires `deep`.
/// The root and every dependency ship a pack under a `speclib` of their own
/// name, with a manifest beside it, and `myapp_requires` is the root
/// manifest's `require` lines.
fn stage_packages(name: &str, myapp_requires: &str) -> PathBuf {
    use tcl_pkg_model::lockfile::{LockFile, LockedPackage, SourceSpec, serialise};
    let root = scratch(name);
    let write = |path: PathBuf, text: &str| {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        std::fs::write(path, text).expect("write");
    };
    write(
        root.join("tclpkg.tcl"),
        &format!("package myapp\nversion 1.0.0\n{myapp_requires}dev-require devdep 1.0.0\n"),
    );
    write(root.join("myapp.tclspec"), &packaged_pack("myapp"));
    let mut lock = LockFile::new("myapp", ">=8.6");
    for (package, requires, dev) in [
        ("direct", vec!["deep@1.0.0"], false),
        ("deep", vec![], false),
        ("devdep", vec![], true),
    ] {
        let dir = root.join(format!("lib/{package}-1.0.0"));
        write(
            dir.join("tclpkg.tcl"),
            &format!("package {package}\nversion 1.0.0\n"),
        );
        write(
            dir.join(format!("{package}.tclspec")),
            &packaged_pack(package),
        );
        lock.packages.push(LockedPackage {
            name: package.to_owned(),
            version: "1.0.0".to_owned(),
            source: SourceSpec::new("tarball", ""),
            integrity: String::new(),
            size: 0,
            requires: requires.into_iter().map(str::to_owned).collect(),
            provides: Vec::new(),
            license: String::new(),
            dev,
            spec_integrity: None,
        });
    }
    write(root.join("tclpkg.lock"), &serialise(&lock));
    root
}

/// `speclib::unpack` as the load left it, and the messages the load raised
/// on its row, in message order.
fn loaded<'a>(set: &'a PackSet, speclib: &str) -> (&'a tcl_spectcl::PackCommand, Vec<&'a str>) {
    let name = format!("{speclib}::unpack");
    let command = set
        .packs
        .iter()
        .find(|pack| pack.name == speclib)
        .and_then(|pack| pack.command(&name))
        .unwrap_or_else(|| panic!("`{name}` loads: {:#?}", set.notices));
    let context = format!("command {name}");
    let mut messages: Vec<&str> = set
        .notices
        .iter()
        .filter(|notice| notice.context == context)
        .map(|notice| notice.message.as_str())
        .collect();
    messages.sort_unstable();
    (command, messages)
}

const LASSIGN_STAMP_REFUSED: &str = "`codegen_hook Lassign` refused for `{}::unpack`: a trusted workspace pack may not name a \
     codegen catalogue member; only a bundled pack may carry `alias_of lassign`'s own stamp";

fn stamp_refused_for(speclib: &str) -> String {
    LASSIGN_STAMP_REFUSED.replace("{}", speclib)
}

/// A package reached only through another's requirements may not say `alias_of`
/// or a backing: both are dropped, each with a warning on the command's row
/// that names the tier, and the command keeps every analysis fact it
/// declared. A development dependency is no nearer. The stamp goes too — by
/// the provenance gate, which every workspace pack meets first, so the notice
/// names that gate.
#[test]
fn a_transitive_dependencys_alias_of_is_dropped() {
    let root = stage_packages("capability-transitive", "require direct 1.0.0\n");
    let set = load_workspace(&root);

    for (speclib, article) in [("deep", "a transitive"), ("devdep", "a development")] {
        let (command, messages) = loaded(&set, speclib);
        assert_eq!(
            messages,
            vec![
                format!(
                    "`alias_of lassign` refused for `{speclib}::unpack`: {article} dependency's \
                     pack may not declare `alias_of`; only the workspace's own package and its \
                     direct dependencies may"
                ),
                stamp_refused_for(speclib),
                format!(
                    "`runtime_backing shipped-builtin lassign` refused for `{speclib}::unpack`: \
                     {article} dependency's pack may not declare a `runtime_backing`; only the \
                     workspace's own package and its direct dependencies may"
                ),
            ],
            "{speclib}"
        );
        assert_eq!(command.spec.alias_of, None, "{speclib}");
        assert_eq!(
            command.spec.runtime_backing,
            tcl_registry::RuntimeBacking::None
        );
        assert_eq!(command.spec.codegen_hook, None, "{speclib}");
        assert_eq!(
            command.spec.arity.min, 2,
            "{speclib}: the arity is untouched"
        );
        assert!(
            set.notices
                .iter()
                .filter(|n| n.context == format!("command {speclib}::unpack"))
                .all(|n| n.severity == pack::Severity::Warning),
            "{speclib}"
        );

        let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &set);
        let installed = registry
            .get(&format!("{speclib}::unpack"))
            .expect("installed");
        assert_eq!(installed.alias_of, None, "{speclib}: nothing installed");
        assert_eq!(
            installed.runtime_backing,
            tcl_registry::RuntimeBacking::None
        );
        assert_eq!(installed.arity.min, 2, "{speclib}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// A direct dependency keeps `alias_of` and a backing but no stamp. The
/// negative is the workspace's own package, the one tier the capability
/// matrix gives everything: it keeps both declarations, and its stamp is
/// refused only by the provenance gate, which the capability cannot lift.
#[test]
fn a_direct_dependency_keeps_alias_of_but_not_a_stamp() {
    let root = stage_packages("capability-direct", "require direct 1.0.0\n");
    let set = load_workspace(&root);

    for speclib in ["direct", "myapp"] {
        let (command, messages) = loaded(&set, speclib);
        assert_eq!(command.spec.alias_of, Some("lassign"), "{speclib}");
        assert_eq!(
            command.spec.runtime_backing,
            tcl_registry::RuntimeBacking::shipped("lassign"),
            "{speclib}"
        );
        assert_eq!(command.spec.codegen_hook, None, "{speclib}: no stamp");
        assert_eq!(
            messages,
            vec![stamp_refused_for(speclib)],
            "{speclib}: one notice, and it is the provenance gate's"
        );
    }

    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &set);
    let installed = registry.get("direct::unpack").expect("installed");
    assert_eq!(installed.alias_of, Some("lassign"));
    assert_eq!(
        installed.runtime_backing,
        tcl_registry::RuntimeBacking::shipped("lassign")
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The gate reads the graph, so moving a package in it moves what its pack
/// may say — and the pack set's key with it, or the cached registry would
/// keep serving the old answer: `deep` loses its `alias_of` while only
/// `direct` requires it, and keeps it once the workspace requires it too.
#[test]
fn a_package_moving_in_the_graph_changes_what_its_pack_loads() {
    let root = stage_packages("capability-moves", "require direct 1.0.0\n");
    let far = load_workspace(&root);
    assert_eq!(loaded(&far, "deep").0.spec.alias_of, None);

    std::fs::write(
        root.join("tclpkg.tcl"),
        "package myapp\nversion 1.0.0\nrequire direct 1.0.0\nrequire deep 1.0.0\n\
         dev-require devdep 1.0.0\n",
    )
    .expect("rewrite the manifest");
    let near = load_workspace(&root);
    assert_eq!(loaded(&near, "deep").0.spec.alias_of, Some("lassign"));
    assert_ne!(
        far.key, near.key,
        "a tier is part of what a pack set is, as an edit is"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A manifest that places nothing does not lift its pack: one that does not
/// read, and one that names a package the lockfile does not list, leave the
/// dependency at the floor of a listed package, transitive, so its `alias_of`
/// and its backing are dropped as a transitive package's are. A dependency
/// cannot reach the workspace's own rights by writing `package anything`.
#[test]
fn a_manifest_that_places_nothing_does_not_lift_its_pack() {
    use tcl_dialect::model::DependencyTier;
    let root = stage_packages("capability-unplaced", "require direct 1.0.0\n");
    for manifest in ["not a manifest %%\n", "package anything\nversion 1.0.0\n"] {
        std::fs::write(root.join("lib/direct-1.0.0/tclpkg.tcl"), manifest).expect("rewrite");
        let set = load_workspace(&root);
        let (command, messages) = loaded(&set, "direct");
        assert_eq!(
            command.dependency_tier,
            Some(DependencyTier::Transitive),
            "{manifest:?}"
        );
        assert_eq!(command.spec.alias_of, None, "{manifest:?}");
        assert_eq!(
            command.spec.runtime_backing,
            tcl_registry::RuntimeBacking::None,
            "{manifest:?}"
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("a transitive dependency's pack may not declare")),
            "{manifest:?}: {messages:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// A pack no package places has no tier, so the matrix does not narrow it: it
/// keeps its backing as well as its `alias_of`. That is a pack under
/// `.tcl-lsp/`, which no package ships, and — since nothing is known of a
/// graph with no lockfile — one beside a dependency's manifest in a workspace
/// that has none.
#[test]
fn a_pack_no_package_ships_is_not_narrowed() {
    use tcl_dialect::model::WorkspaceTrust;
    let set = load_one(
        "capability-none",
        &packaged_pack("vendor"),
        Tier::Workspace,
        WorkspaceTrust::Trusted,
    );
    let (command, messages) = loaded(&set, "vendor");
    assert_eq!(command.dependency_tier, None);
    assert_eq!(command.spec.alias_of, Some("lassign"));
    assert_eq!(
        command.spec.runtime_backing,
        tcl_registry::RuntimeBacking::shipped("lassign")
    );
    assert_eq!(messages, vec![stamp_refused_for("vendor")]);

    let root = stage_packages("capability-no-lockfile", "require direct 1.0.0\n");
    std::fs::remove_file(root.join("tclpkg.lock")).expect("remove the lockfile");
    let set = load_workspace(&root);
    for speclib in ["direct", "deep", "devdep", "myapp"] {
        let (command, _) = loaded(&set, speclib);
        assert_eq!(command.dependency_tier, None, "{speclib}");
        assert_eq!(command.spec.alias_of, Some("lassign"), "{speclib}");
    }
    let _ = std::fs::remove_dir_all(&root);
}
