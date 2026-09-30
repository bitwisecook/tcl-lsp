// SPDX-License-Identifier: MIT
//! Invariant I6 — a pack override cannot weaken a shipped command's security
//! facts.
//!
//! The probe in these tests is the one that *found* the hole: before R12 it
//! loaded without error and produced an `exec` carrying neither `TAINT_SINK`
//! nor `TAINT_SOURCE`.

use tcl_registry::CommandSpec;
use tcl_registry::hooks::{AnalyserHookId, LoweringHookId};
use tcl_registry::native_lowering::NativeLowering;
use tcl_registry::semantic_operation::SemanticOperationId;
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
        spec.traits
            .contains(tcl_registry::traits::Traits::TAINT_SINK),
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
    let spec = registry
        .get("probe::fresh")
        .expect("the pack command loads");
    assert!(
        !spec
            .traits
            .contains(tcl_registry::traits::Traits::TAINT_SINK),
        "a command that overrides nothing inherits no floor"
    );
}

/// The command as the dialect ships it, before any pack.
fn shipped(dialect: &str, command: &str) -> &'static CommandSpec {
    tcl_registry::model::static_context_for(dialect)
        .commands()
        .get(command)
        .expect("the dialect ships the command")
}

/// The command as it installs when a workspace pack overrides it with
/// `body`, and proof the override took effect: it declared an arity window no
/// shipped command has, so what survives of the shipped spec is the floor's
/// doing alone.
fn overridden(test: &str, dialect: &str, command: &str, body: &str) -> &'static CommandSpec {
    let dir = tmpdir(test);
    let source = format!(
        "speclib probe 2.0 {{\n  command {command} -override {{\n    arity 7..9\n    {body}\n  }}\n}}\n"
    );
    let packs = packs_from(&dir, &source);
    let registry = registry_for_dialect_with_packs(dialect, &packs);
    let spec = registry
        .get(command)
        .expect("the overridden command is still registered");
    assert_eq!(
        (spec.arity.min, spec.arity.max),
        (7, 9),
        "the override took effect: its arity window is what installed"
    );
    spec
}

#[test]
fn a_workspace_override_cannot_swap_the_lowering_hook() {
    let kept = shipped("tcl8.6", "while").lowering_hook;
    assert_eq!(kept, Some(LoweringHookId::While));
    let spec = overridden(
        "lowering-hook",
        "tcl8.6",
        "while",
        "lowering_hook -native If",
    );
    assert_eq!(
        spec.lowering_hook, kept,
        "an override does not choose how the compiler translates a shipped command"
    );
}

#[test]
fn a_workspace_override_cannot_swap_the_analyser_hook() {
    let kept = shipped("tcl8.6", "source").analyser_hook;
    assert_eq!(kept, Some(AnalyserHookId::Source));
    let spec = overridden(
        "analyser-hook",
        "tcl8.6",
        "source",
        "analyser_hook -native Rename",
    );
    assert_eq!(
        spec.analyser_hook, kept,
        "an override does not choose which analyser handler reads a shipped command"
    );
}

#[test]
fn a_workspace_override_cannot_swap_the_semantic_operation() {
    let kept = shipped("tcl8.6", "puts").semantic_operation;
    assert!(matches!(kept, Some(SemanticOperationId::Intrinsic(_))));
    let spec = overridden(
        "semantic-operation",
        "tcl8.6",
        "puts",
        "semantic_operation Invoke",
    );
    assert_eq!(
        spec.semantic_operation, kept,
        "an override does not rename the operation a shipped command performs"
    );
}

#[test]
fn a_workspace_override_cannot_swap_the_state_transitions() {
    let kept = format!("{:?}", shipped("tcl8.6", "join").state_transitions);
    assert!(
        kept.starts_with("Some("),
        "the probe needs a command that ships a descriptor"
    );
    let spec = overridden(
        "state-transitions",
        "tcl8.6",
        "join",
        "state_transitions {\n      composition Extend\n      argument_shape Positional\n    }",
    );
    assert_eq!(
        format!("{:?}", spec.state_transitions),
        kept,
        "an override does not restate the state a shipped command transitions"
    );
}

#[test]
fn a_workspace_override_cannot_drop_the_native_lowering() {
    let kept = shipped("tcl8.6", "break").native_lowering;
    assert!(matches!(kept, Some(NativeLowering::Completion(_))));
    // A pack cannot spell `native_lowering`; the way an override loses it is
    // by replacing the command wholesale and saying nothing.
    let spec = overridden("native-lowering", "tcl8.6", "break", "");
    assert_eq!(
        spec.native_lowering, kept,
        "an override that says nothing keeps the shipped native shape"
    );
}

#[test]
fn a_workspace_override_cannot_drop_the_bpf_op() {
    let kept = shipped("bpf", "pass").bpf_op;
    assert!(kept.is_some());
    // As above: a pack cannot spell `bpf_op`, only drop it.
    let spec = overridden("bpf-op", "bpf", "pass", "");
    assert_eq!(
        spec.bpf_op, kept,
        "an override that says nothing keeps the shipped BPF op"
    );
}
