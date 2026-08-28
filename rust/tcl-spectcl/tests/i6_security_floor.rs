// SPDX-License-Identifier: MIT
//! Invariant I6 — a pack override cannot weaken a shipped command's security
//! facts.
//!
//! The probe in these tests is the one that *found* the hole: before R12 it
//! loaded without error and produced an `exec` carrying neither `TAINT_SINK`
//! nor `TAINT_SOURCE`.

use tcl_spectcl::discovery::{Origin, PackFile, Tier};
use tcl_spectcl::install::registry_for_dialect_with_packs;
use tcl_spectcl::pack::load;

const OVERRIDE_EXEC: &str = "speclib probe 2.0 {\n  \
                             command exec -override {\n    \
                               arity 1..\n  \
                             }\n}\n";

fn packs_from(dir: &std::path::Path, source: &str) -> tcl_spectcl::pack::PackSet {
    let path = dir.join("probe.tclspec");
    std::fs::write(&path, source).expect("write pack");
    load(&[PackFile {
        tier: Tier::Workspace,
        path,
        origin: Origin::DotDir,
    }])
}

fn tmpdir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tcl-spectcl-i6-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[test]
fn a_workspace_override_cannot_strip_exec_taint() {
    let dir = tmpdir("exec");
    let packs = packs_from(&dir, OVERRIDE_EXEC);
    let registry = registry_for_dialect_with_packs("tcl8.6", &packs);
    let spec = registry.get("exec").expect("exec is still registered");

    // The override took effect — the pack declared no options, and the
    // shipped `exec` has several (`-ignorestderr`, `-keepnewline`, `--`).
    assert!(
        spec.options.is_empty(),
        "the pack's spec is what took effect, not the shipped one"
    );
    // … and the security floor survived it.
    assert!(
        spec.traits.contains(tcl_registry::traits::Traits::TAINT_SINK),
        "I6: an override must not drop TAINT_SINK"
    );
    assert!(
        spec.traits
            .contains(tcl_registry::traits::Traits::TAINT_SOURCE),
        "I6: an override must not drop TAINT_SOURCE"
    );
}

#[test]
fn the_floor_does_not_invent_facts_for_a_new_command() {
    let dir = tmpdir("new");
    let packs = packs_from(
        &dir,
        "speclib probe 2.0 {\n  command probe::fresh {\n    arity 1..\n  }\n}\n",
    );
    let registry = registry_for_dialect_with_packs("tcl8.6", &packs);
    let spec = registry.get("probe::fresh").expect("the pack command loads");
    assert!(
        !spec.traits.contains(tcl_registry::traits::Traits::TAINT_SINK),
        "a command that overrides nothing inherits no floor"
    );
}
