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

//! The editor's Workspace Trust state, from discovery to the registry
//! (`docs/design/compiler/registry-consumer-contracts.md` § *Ruling — trust
//! gates execution, not authority*).
//!
//! One input, [`WorkspaceTrust`], decides whether a workspace pack is
//! [`Provenance::WorkspaceTrusted`] or [`Provenance::WorkspaceUntrusted`].
//! Authority never reads it — an untrusted pack's declarative facts reach the
//! registry exactly as a trusted one's do — while the registration gates
//! (E-R2), the snapshot identity and the execution of hook bodies do.

use std::path::PathBuf;
use std::rc::Rc;

use tcl_dialect::model::{Provenance, WorkspaceTrust};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::pack_hooks;
use tcl_spectcl::discovery::{DiscoveryOptions, Origin, PackFile, Tier};
use tcl_spectcl::hooks;
use tcl_spectcl::pack::{self, PackSet, Severity};
use tcl_spectcl::{EvalOptions, eval_snapshot_key};

const D: &str = "tcl8.6";

/// A pack file at `tier`, as discovery would report one.
fn file(tier: Tier, name: &str) -> PackFile {
    PackFile {
        tier,
        path: PathBuf::from(format!("/workspace/.tcl-lsp/{name}.tclspec")),
        origin: Origin::DotDir,
    }
}

/// Load `source` as one pack at `tier`, under the editor's `trust`.
fn load(tier: Tier, name: &str, source: &str, trust: WorkspaceTrust) -> PackSet {
    pack::load_in_memory_under(vec![(file(tier, name), source.to_owned())], trust)
}

/// A private command stating arity, roles and a value-semantics declaration:
/// every kind of fact the authority ruling protects.
const FACTS: &str = "speclib trustfacts 2.2 {\n\
    \x20   command trustfacts::label {\n\
    \x20       arity 1..2\n\
    \x20       arg 0 -role Value\n\
    \x20       arg 1 -role Body\n\
    \x20       semantics {\n\
    \x20           effects {no_store_writes no_external_io}\n\
    \x20           result -semantic string\n\
    \x20       }\n\
    \x20   }\n\
    }\n";

/// A pack that shadows a compiled command name.
const OVERRIDE: &str = "speclib trustsneaky 2.0 {\n\
    \x20   command lsort -override {\n\
    \x20       arity 1..\n\
    \x20   }\n\
    }\n";

#[test]
fn an_untrusted_workspace_pack_is_workspace_untrusted_provenance() {
    let untrusted = load(
        Tier::Workspace,
        "untrusted",
        FACTS,
        WorkspaceTrust::Untrusted,
    );
    let pack = &untrusted.packs[0];
    assert_eq!(pack.trust, WorkspaceTrust::Untrusted);
    assert_eq!(pack.provenance(), Provenance::WorkspaceUntrusted);

    // The negative: the same pack in a workspace the editor trusts.
    let trusted = load(Tier::Workspace, "trusted", FACTS, WorkspaceTrust::Trusted);
    assert_eq!(trusted.packs[0].provenance(), Provenance::WorkspaceTrusted);

    // Only the workspace tier reads the state: a user-tier pack is the
    // user's whatever the editor thinks of the folder that is open.
    let user = load(Tier::User, "user", FACTS, WorkspaceTrust::Untrusted);
    assert_eq!(user.packs[0].trust, WorkspaceTrust::Trusted);
    assert_eq!(user.packs[0].provenance(), Provenance::User);
}

#[test]
fn an_untrusted_workspace_pack_still_declares_its_facts() {
    let facts = |trust: WorkspaceTrust| {
        let packs = load(Tier::Workspace, "facts", FACTS, trust);
        assert!(
            packs.notices.is_empty(),
            "{trust:?}: the pack loads cleanly: {:#?}",
            packs.notices
        );
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs(D, &packs);
        let spec = registry
            .get("trustfacts::label")
            .unwrap_or_else(|| panic!("{trust:?}: the command reaches the registry"));
        (
            (spec.arity.min, spec.arity.max),
            registry.arg_indices_for_role("trustfacts::label", &["x", "{body}"], ArgRole::Body),
            registry.arg_indices_for_role("trustfacts::label", &["x", "{body}"], ArgRole::Value),
            format!("{:?}", spec.semantics),
        )
    };
    let untrusted = facts(WorkspaceTrust::Untrusted);
    assert_eq!(untrusted.0, (1, 2), "the arity is the pack's");
    assert_eq!(untrusted.1, vec![1], "the body role is the pack's");
    assert_eq!(untrusted.2, vec![0], "the value role is the pack's");
    assert!(
        untrusted.3.starts_with("Declared"),
        "the semantics declaration is the pack's: {}",
        untrusted.3
    );
    assert_eq!(
        untrusted,
        facts(WorkspaceTrust::Trusted),
        "authority does not read the trust state"
    );
}

#[test]
fn an_untrusted_workspace_pack_cannot_override_a_compiled_name() {
    let untrusted = load(
        Tier::Workspace,
        "sneaky-untrusted",
        OVERRIDE,
        WorkspaceTrust::Untrusted,
    );
    assert!(
        untrusted.packs.iter().all(|p| p.commands.is_empty()),
        "registration is transactional: nothing loads"
    );
    let refusal = untrusted
        .notices
        .iter()
        .find(|n| n.message.contains("design E-R2"))
        .unwrap_or_else(|| panic!("the refusal is reported: {:#?}", untrusted.notices));
    assert!(
        refusal.message.contains("untrusted workspace") && refusal.message.contains("lsort"),
        "the refusal names the provenance and the command: {}",
        refusal.message
    );
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs(D, &untrusted);
    let shipped = tcl_spectcl::install::registry_for_dialect_with_packs(D, &PackSet::default());
    assert_eq!(
        registry.get("lsort").map(|s| s.arity),
        shipped.get("lsort").map(|s| s.arity),
        "the shipped `lsort` stands"
    );

    // The negative: a trusted workspace pack may override a shipped command.
    let trusted = load(
        Tier::Workspace,
        "sneaky-trusted",
        OVERRIDE,
        WorkspaceTrust::Trusted,
    );
    let command = trusted.packs[0]
        .command("lsort")
        .expect("a trusted workspace pack loads its override");
    assert!(command.overrides_shipped);
}

#[test]
fn a_client_that_reports_nothing_is_trusted() {
    assert_eq!(WorkspaceTrust::default(), WorkspaceTrust::Trusted);
    assert_eq!(
        DiscoveryOptions::default().workspace_trust,
        WorkspaceTrust::Trusted
    );
    assert_eq!(EvalOptions::default().trust, WorkspaceTrust::Trusted);

    // End to end through discovery: options a client that says nothing
    // produces load a `.tcl-lsp/` pack as a trusted workspace pack, override
    // and all.
    let root = std::env::temp_dir().join(format!(
        "tcl-spectcl-workspace-trust-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".tcl-lsp")).expect("pack dir");
    std::fs::write(root.join(".tcl-lsp/sneaky.tclspec"), OVERRIDE).expect("write pack");
    let options = DiscoveryOptions {
        workspace_roots: vec![root.clone()],
        user_dir: Some(root.join("no-user-tier")),
        bundled_dir: Some(root.join("no-bundled-tier")),
        ..DiscoveryOptions::default()
    };
    let files = tcl_spectcl::discover(&options);
    let set = tcl_spectcl::bundled::load_discovered_in(
        &tcl_lsp_core::vfs::NativeStore,
        &files,
        options.workspace_trust,
    );
    let sneaky = set
        .packs
        .iter()
        .find(|p| p.name == "trustsneaky")
        .expect("the workspace pack loads");
    assert_eq!(sneaky.provenance(), Provenance::WorkspaceTrusted);
    assert!(sneaky.command("lsort").is_some());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_snapshot_key_distinguishes_trust() {
    let at = |tier: Tier, trust: WorkspaceTrust| {
        eval_snapshot_key(
            FACTS,
            &EvalOptions {
                tier,
                trust,
                ..EvalOptions::default()
            },
        )
    };
    assert_ne!(
        at(Tier::Workspace, WorkspaceTrust::Trusted),
        at(Tier::Workspace, WorkspaceTrust::Untrusted),
        "a workspace pack's snapshot is keyed by the trust it evaluated under"
    );
    assert_ne!(
        tcl_spectcl::cache::key_for(FACTS, Tier::Workspace, WorkspaceTrust::Trusted),
        tcl_spectcl::cache::key_for(FACTS, Tier::Workspace, WorkspaceTrust::Untrusted),
        "and so is its on-disk entry"
    );
    // A tier that does not read the state keys the same under either.
    assert_eq!(
        at(Tier::Bundled, WorkspaceTrust::Trusted),
        at(Tier::Bundled, WorkspaceTrust::Untrusted)
    );
    // And the set key moves with the grant, which is what makes a reload
    // that only changed the trust state re-install.
    let set_key = |trust: WorkspaceTrust| load(Tier::Workspace, "keyed", FACTS, trust).key;
    assert_ne!(
        set_key(WorkspaceTrust::Trusted),
        set_key(WorkspaceTrust::Untrusted)
    );
}

/// A pack with three hook bodies — a `const_fold`, an option's `-arity-hook`
/// and a declared implementation (the `evaluate` statement, rebound by the
/// `facts` statement after it) — beside a `clause_grammar`, whose two
/// derived hooks are not bodies. Lines are 1-based from `speclib`.
const HOOKED: &str = "speclib trustfold 2.2 {\n\
    \x20   command trustfold::strlen {\n\
    \x20       arity 1\n\
    \x20       arg 0 -role Value\n\
    \x20       const_fold -inputs {words} {words ctx} {\n\
    \x20           fold [string length [lindex $words 0]]\n\
    \x20       }\n\
    \x20   }\n\
    \x20   command trustfold::pick {\n\
    \x20       arity 1..\n\
    \x20       option -width -arity-hook {words ctx} {\n\
    \x20           consume 2\n\
    \x20       }\n\
    \x20   }\n\
    \x20   command trustfold::label {\n\
    \x20       arity 1\n\
    \x20       semantics {\n\
    \x20           effects {no_store_writes no_external_io}\n\
    \x20           result -semantic string\n\
    \x20       }\n\
    \x20       evaluate -implementation trustfold.label.v1 -host bounded_tcl {\n\
    \x20           inputs {arg 0 exact}\n\
    \x20           depends {tcl_profile implementation_identity}\n\
    \x20           body {name} { fold [string cat label: $name] }\n\
    \x20       }\n\
    \x20       facts {\n\
    \x20           result -string_segments {{constant label:} {operand 0}}\n\
    \x20       }\n\
    \x20   }\n\
    \x20   command trustfold::guarded {\n\
    \x20       arity 1\n\
    \x20       clause_grammar {\n\
    \x20           head {Body} -timing protected\n\
    \x20       }\n\
    \x20   }\n\
    }\n";

/// The bodies of [`HOOKED`], as `(command, field, line)`.
const HOOKED_BODIES: [(&str, &str, u32); 3] = [
    ("trustfold::strlen", "const_fold", 5),
    ("trustfold::pick", "options.arity_hook", 11),
    ("trustfold::label", "evaluate", 21),
];

/// What `trustfold::strlen abcde` folds to in the registry `packs` install.
fn strlen_fold(packs: &PackSet) -> Option<String> {
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs(D, packs);
    let fold = registry
        .get("trustfold::strlen")
        .and_then(|spec| spec.const_fold)
        .expect("the command reaches the registry with a const_fold field");
    fold(&["abcde"])
}

/// In a workspace the editor has not trusted, no pack hook body runs: the
/// plan allocates none a slot, the installed field keeps the loader's
/// abstaining placeholder, and each body is reported once, on its own row,
/// as information. The derived hooks are not bodies and draw nothing. The
/// negative: the same pack trusted draws no notice, binds every body, and
/// folds.
#[test]
fn an_untrusted_pack_installs_no_hook_body_and_reports_each_as_dormant() {
    let untrusted = load(Tier::Workspace, "hooked", HOOKED, WorkspaceTrust::Untrusted);
    let notices: Vec<(u32, &str, &str)> = untrusted
        .notices
        .iter()
        .map(|notice| {
            assert_eq!(
                notice.severity,
                Severity::Information,
                "a dormant hook is information, not a warning: {notice:?}"
            );
            assert_eq!(notice.path, file(Tier::Workspace, "hooked").path);
            (
                notice.line,
                notice.context.as_str(),
                notice.message.as_str(),
            )
        })
        .collect();
    assert_eq!(
        notices.len(),
        HOOKED_BODIES.len(),
        "one notice per body, none for the derived hooks: {notices:#?}"
    );
    for ((line, context, message), (command, field, want_line)) in notices.iter().zip(HOOKED_BODIES)
    {
        assert_eq!(*line, want_line, "{field} is reported on its own row");
        assert_eq!(*context, format!("command {command}"));
        assert!(
            message.starts_with(&format!(
                "`{field}` is dormant: the workspace is not trusted"
            )),
            "{message}"
        );
    }

    let plan = hooks::plan_for(&untrusted);
    assert!(plan.is_empty(), "no body has a slot");
    assert!(plan.packs().iter().all(|pack| pack.programs.is_empty()));
    let dormant: Vec<(&str, &str, u32)> = plan
        .dormant()
        .iter()
        .map(|hook| (hook.command.as_str(), hook.field, hook.line))
        .collect();
    assert_eq!(
        dormant, HOOKED_BODIES,
        "the plan lists what the load reports"
    );
    assert!(plan.dormant().iter().all(|hook| hook.pack == "trustfold"));

    // The negative: trusted, the same pack says nothing and binds all three.
    let trusted = load(Tier::Workspace, "hooked", HOOKED, WorkspaceTrust::Trusted);
    assert!(
        trusted.notices.is_empty(),
        "the pack loads cleanly: {:#?}",
        trusted.notices
    );
    let plan = hooks::plan_for(&trusted);
    assert!(plan.dormant().is_empty());
    let slots: Vec<_> = plan
        .packs()
        .iter()
        .flat_map(|pack| pack.programs.iter().map(|program| program.slot))
        .collect();
    assert_eq!(slots.len(), HOOKED_BODIES.len());
    assert!(slots.iter().all(Option::is_some), "every body is bound");

    // With a host on this thread serving the trusted plan, the trusted
    // install folds and the untrusted one still abstains: its field is the
    // placeholder, which no host is ever asked about.
    let host = Rc::new(tcl_spec_hooks::tclvm_host());
    for programs in plan.packs() {
        let installed = host.install_pack_hooks(programs.clone());
        assert!(
            installed.iter().all(|entry| entry.declined.is_none()),
            "{installed:?}"
        );
    }
    pack_hooks::install_host(host);
    assert_eq!(strlen_fold(&trusted), Some("5".to_owned()));
    assert_eq!(strlen_fold(&untrusted), None);
    pack_hooks::clear_host();
}

/// Only an untrusted workspace holds its bodies dormant. A Spec Studio
/// override is untrusted for registration — it may not `-override` a
/// compiled name — and still runs: it is the author's own live edit.
#[test]
fn only_an_untrusted_workspace_holds_its_bodies_dormant() {
    for provenance in [
        Provenance::BuiltIn,
        Provenance::BundledPack,
        Provenance::User,
        Provenance::WorkspaceTrusted,
        Provenance::StudioOverride,
        Provenance::Document,
    ] {
        assert!(hooks::hook_bodies_run(provenance), "{provenance:?} runs");
    }
    assert!(!hooks::hook_bodies_run(Provenance::WorkspaceUntrusted));
    assert!(Provenance::StudioOverride.is_untrusted());

    // Through the load: every tier but the workspace ignores the state.
    for tier in [Tier::Bundled, Tier::User, Tier::StudioOverride] {
        let packs = load(tier, "hooked-tier", HOOKED, WorkspaceTrust::Untrusted);
        assert!(
            hooks::plan_for(&packs).dormant().is_empty(),
            "{tier:?} bodies run"
        );
        assert!(
            packs
                .notices
                .iter()
                .all(|notice| !notice.message.contains("is dormant")),
            "{tier:?}: {:#?}",
            packs.notices
        );
    }
}
