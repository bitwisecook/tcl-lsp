// SPDX-License-Identifier: MIT
//! Invariant I6 — a pack override cannot weaken a shipped command's security
//! facts.
//!
//! The probe in these tests is the one that *found* the hole: before R12 it
//! loaded without error and produced an `exec` carrying neither `TAINT_SINK`
//! nor `TAINT_SOURCE`.

use tcl_registry::CommandSpec;
use tcl_registry::hooks::LoweringHookId;
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
        dependency_tier: None,
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

/// One shipped fact a workspace override must leave alone.
struct FloorCase {
    fact: &'static str,
    dialect: &'static str,
    command: &'static str,
    /// What the override says; empty where a pack cannot spell the fact and
    /// can only lose it by replacing the command and saying nothing.
    body: &'static str,
    /// The fact as text, so one table carries facts of different types.
    read: fn(&CommandSpec) -> String,
    /// How the shipped command's fact starts: the probe needs a command that
    /// ships one.
    shipped_as: &'static str,
}

fn subcommand_fact(
    spec: &CommandSpec,
    name: &str,
    read: fn(&tcl_registry::spec::SubCommand) -> String,
) -> String {
    spec.subcommand(name)
        .map_or_else(|| "absent".to_owned(), read)
}

fn implicit_form_lowering(spec: &CommandSpec) -> String {
    spec.command_forms
        .iter()
        .find(|form| form.name == "implicit")
        .map_or_else(
            || "absent".to_owned(),
            |form| format!("{:?}", form.lowering_hook),
        )
}

const FLOOR_CASES: &[FloorCase] = &[
    FloorCase {
        fact: "lowering hook",
        dialect: "tcl8.6",
        command: "while",
        body: "lowering_hook -native If",
        read: |spec| format!("{:?}", spec.lowering_hook),
        shipped_as: "Some(While)",
    },
    FloorCase {
        fact: "analyser hook",
        dialect: "tcl8.6",
        command: "source",
        body: "analyser_hook -native Rename",
        read: |spec| format!("{:?}", spec.analyser_hook),
        shipped_as: "Some(Source)",
    },
    FloorCase {
        fact: "semantic operation",
        dialect: "tcl8.6",
        command: "puts",
        body: "semantic_operation Invoke",
        read: |spec| format!("{:?}", spec.semantic_operation),
        shipped_as: "Some(Intrinsic(",
    },
    FloorCase {
        fact: "state transitions",
        dialect: "tcl8.6",
        command: "join",
        body: "state_transitions {\n      composition Extend\n      argument_shape Positional\n    }",
        read: |spec| format!("{:?}", spec.state_transitions.stated()),
        shipped_as: "Some(",
    },
    FloorCase {
        fact: "native lowering",
        dialect: "tcl8.6",
        command: "break",
        body: "",
        read: |spec| format!("{:?}", spec.native_lowering),
        shipped_as: "Some(Completion(",
    },
    FloorCase {
        fact: "BPF op",
        dialect: "bpf",
        command: "pass",
        body: "",
        read: |spec| format!("{:?}", spec.bpf_op),
        shipped_as: "Some(",
    },
    // Claiming the command is the host's own, or that nothing executes
    // it, is the swap a compiled site's identity must not follow.
    FloorCase {
        fact: "runtime backing",
        dialect: "tcl8.6",
        command: "lindex",
        body: "runtime_backing host-native",
        read: |spec| {
            format!(
                "shipped lindex: {}",
                spec.runtime_backing == tcl_registry::RuntimeBacking::shipped("lindex")
            )
        },
        shipped_as: "shipped lindex: true",
    },
    FloorCase {
        fact: "runtime backing",
        dialect: "tcl8.6",
        command: "lindex",
        body: "runtime_backing none",
        read: |spec| {
            format!(
                "shipped lindex: {}",
                spec.runtime_backing == tcl_registry::RuntimeBacking::shipped("lindex")
            )
        },
        shipped_as: "shipped lindex: true",
    },
    FloorCase {
        fact: "subcommand lowering hook",
        dialect: "tcl8.6",
        command: "array",
        body: "subcommand for { lowering_hook -native If }",
        read: |spec| subcommand_fact(spec, "for", |sub| format!("{:?}", sub.lowering_hook)),
        shipped_as: "Some(ArrayFor)",
    },
    FloorCase {
        fact: "subcommand semantic operation",
        dialect: "tcl8.6",
        command: "string",
        body: "subcommand length { arity 1 }",
        read: |spec| {
            subcommand_fact(spec, "length", |sub| {
                format!("{:?}", sub.semantic_operation)
            })
        },
        shipped_as: "Some(Intrinsic(",
    },
    FloorCase {
        fact: "form lowering hook",
        dialect: "tcl8.6",
        command: "incr",
        body: "refine implicit { lowering_hook -native If }",
        read: implicit_form_lowering,
        shipped_as: "Some(Incr)",
    },
];

#[test]
fn a_workspace_override_keeps_the_shipped_floor_under_tcl_8_6_and_bpf() {
    for (row, case) in FLOOR_CASES.iter().enumerate() {
        let label = format!("{} of `{}` under {}", case.fact, case.command, case.dialect);
        let kept = (case.read)(shipped(case.dialect, case.command));
        assert!(kept.starts_with(case.shipped_as), "{label}: ships {kept}");
        let spec = overridden(
            &format!("floor-{row}"),
            case.dialect,
            case.command,
            case.body,
        );
        assert_eq!(
            (case.read)(spec),
            kept,
            "{label}: `{}` does not change what the shipped command carries",
            case.body
        );
    }
}

#[test]
fn a_renamed_form_on_the_same_route_keeps_the_shipped_lowering_hook() {
    let kept = shipped("tcl8.6", "incr")
        .command_forms
        .iter()
        .find(|form| form.name == "implicit")
        .expect("incr's implicit form is shipped")
        .lowering_hook;
    let spec = overridden(
        "form-renamed-route",
        "tcl8.6",
        "incr",
        "refine renamed { arity 1; lowering_hook -native If }",
    );
    let form = spec
        .command_forms
        .iter()
        .find(|form| form.name == "renamed")
        .expect("the pack's renamed form remains installed");
    assert_eq!(
        form.lowering_hook, kept,
        "a display-name change cannot replace the compiler identity for the same invocation route"
    );
}

#[test]
fn the_floor_leaves_removed_and_new_invocation_routes_as_declared() {
    let removed = overridden("form-removed", "tcl8.6", "incr", "");
    assert!(
        removed.command_forms.is_empty(),
        "the pack removed its forms"
    );

    let added = overridden(
        "form-added",
        "tcl8.6",
        "incr",
        "refine added { arity 3; lowering_hook -native If }",
    );
    let form = added
        .command_forms
        .iter()
        .find(|form| form.name == "added")
        .expect("the new route remains installed");
    assert_eq!(
        form.lowering_hook,
        Some(LoweringHookId::If),
        "a route with no shipped invocation counterpart keeps its declared hook"
    );

    let removed_subcommand = overridden("subcommand-removed", "tcl8.6", "array", "");
    assert!(
        removed_subcommand.subcommands.is_empty(),
        "the pack removed its subcommands"
    );
}

#[test]
fn a_new_command_keeps_the_backing_it_declares() {
    use tcl_registry::RuntimeBacking;

    let dir = tmpdir("new-backing");
    let packs = packs_from(
        &dir,
        "speclib probe 2.0 {\n  command probe::native {\n    arity 1..\n    runtime_backing host-native\n  }\n}\n",
    );
    let registry = registry_for_dialect_with_packs("tcl8.6", &packs);
    let spec = registry
        .get("probe::native")
        .expect("the pack command loads");
    assert_eq!(
        spec.runtime_backing,
        RuntimeBacking::HostNative,
        "a command that overrides nothing inherits no floor, so its own fact stands"
    );
}
