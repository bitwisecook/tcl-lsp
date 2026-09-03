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

//! From `.tclspec` source to a folded call site, through the real loader and
//! the real install path.
//!
//! `tcl-spec-hooks/tests/const_fold_e2e.rs` proves the host and the optimiser
//! agree; this proves the *seam* between the loader and the host — that a hook
//! written in a pack file as a `const_fold {words ctx} { … }` statement,
//! carried by the loader as a `HookDecl`, becomes the function pointer the
//! installed `CommandSpec` holds.
//!
//! The adapter is no longer written out here: it is
//! [`tcl_spectcl::hooks`](tcl_spectcl::hooks), which the workspace install path
//! runs for real. What this file still owns is the claim that the two halves
//! line up end to end.

use std::path::PathBuf;
use std::rc::Rc;

use tcl_compiler::optimiser::manager::optimise_raw;
use tcl_registry::CommandRegistry;
use tcl_registry::hover::ScriptTiming;
use tcl_registry::pack_hooks::{self, HookInputs};
use tcl_spec_hooks::tclvm_host;
use tcl_spectcl::discovery::{Origin, PackFile, Tier};
use tcl_spectcl::hooks;
use tcl_spectcl::loader::{self, HookSource};
use tcl_spectcl::pack::PackSet;

/// The body shared by the legacy and 2.0 spellings of the live-hook pack.
/// Keeping one body makes the DSL release the only axis in the end-to-end
/// comparison.
const FOLD_SOURCE_BODY: &str = r"
    command mylib::strlen {
        arity 1
        arg 0 -role Value
        const_fold -inputs {words} {words ctx} {
            set subject [lindex $words 0]
            if {![string is ascii $subject]} { return }
            fold [string length $subject]
        }
    }
";

fn fold_source(dsl_version: &str) -> String {
    [
        "\nspeclib mylib ",
        dsl_version,
        " {\n",
        FOLD_SOURCE_BODY,
        "}\n",
    ]
    .concat()
}

/// A command-prefix position whose timing depends on another argument. This
/// exercises the new family through the loader, binder, host, thunk and the
/// registry's exact current-position query rather than testing any seam in
/// isolation.
const TIMING_SOURCE: &str = r#"
speclib mylib 1.2 {
    command mylib::callback {
        arity 2
        arg 1 -appends {Exactly 0}
        script_timing_resolver {words ctx} {
            if {[lindex $words 0] eq "later"} {
                timing 1 Deferred
            } else {
                timing 1 SameInvocation
            }
        }
    }
}
"#;

/// The same source as a discovered, merged pack set — the shape the install
/// path takes.
fn pack_set_from(name: &str, source: &str) -> PackSet {
    let dir = std::env::temp_dir().join(format!(
        "tcl-spectcl-pack-source-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path: PathBuf = dir.join("mylib.tclspec");
    std::fs::write(&path, source).expect("write pack");
    tcl_spectcl::pack::load(&[PackFile {
        tier: Tier::Workspace,
        path,
        origin: Origin::DotDir,
    }])
}

fn pack_set(name: &str) -> PackSet {
    pack_set_from(name, &fold_source("1"))
}

fn folds(registry: &CommandRegistry) -> Vec<String> {
    optimise_raw(
        "proc ::f {} {\n    set y [mylib::strlen abcde]\n}\n",
        registry,
        None,
    )
    .into_iter()
    .filter(|optimisation| optimisation.code.as_str() == "O129")
    .map(|optimisation| optimisation.replacement)
    .collect()
}

#[test]
fn the_loader_carries_the_declared_inputs() {
    let source = fold_source("1");
    let pack = loader::evaluate_pack(&source);
    let command = pack.command("mylib::strlen").expect("the pack declares it");
    let [hook] = &command.hooks[..] else {
        panic!("one hook, got {:?}", command.hooks.len());
    };
    let HookSource::Body { params, inputs, .. } = &hook.source else {
        panic!("a Tcl body");
    };
    assert_eq!(params, &["words".to_owned(), "ctx".to_owned()]);
    assert_eq!(inputs, &HookInputs::parse(&["words"]));
    // `words` is the one input that makes a hook uncacheable.
    assert!(!inputs.shape_only());
}

#[test]
fn a_pack_file_folds_a_call_site_in_the_optimiser() {
    let mut results = Vec::new();
    for dsl_version in ["1", "2.0"] {
        let source = fold_source(dsl_version);
        let packs = pack_set_from(&format!("folds-{dsl_version}"), &source);
        assert!(
            packs.notices.is_empty(),
            "DSL {dsl_version} pack loads cleanly: {:?}",
            packs.notices
        );
        assert_eq!(
            packs.packs.first().map(|pack| pack.dsl_version.as_str()),
            Some(dsl_version),
            "the loader preserves the DSL release presented to the hook adapter"
        );

        // The production adapter, and the production slot assignment.
        let plan = hooks::plan_for(&packs);
        assert!(!plan.is_empty(), "the pack declares one hook body");

        let host = Rc::new(tclvm_host());
        for programs in plan.packs() {
            let installed = host.install_pack_hooks(programs.clone());
            for (installation, program) in installed.iter().zip(&programs.programs) {
                assert_eq!(
                    installation.slot, program.slot,
                    "DSL {dsl_version}: the host binds the slot the plan assigned, never a fresh one: {:?}",
                    installation.declined
                );
            }
        }
        pack_hooks::install_host(host);

        // The registry the workspace would install — same call the LSP makes.
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
        let result = folds(&registry);
        assert_eq!(result, vec!["5".to_string()], "DSL {dsl_version}");
        results.push(result);
        pack_hooks::clear_host();
    }
    assert_eq!(
        results[0], results[1],
        "legacy and 2.0 hook programs have identical live semantics"
    );
}

#[test]
fn without_a_host_the_installed_pack_simply_does_not_fold() {
    // The thunk is in place, but a thread with no host installed gets the
    // family's silence — the same answer the loader's abstaining placeholder
    // gave before hooks could run.
    let packs = pack_set("hostless");
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
    pack_hooks::clear_host();
    assert!(folds(&registry).is_empty());
}

#[test]
fn a_pack_timing_hook_controls_a_live_command_prefix_position() {
    let packs = pack_set_from("timing", TIMING_SOURCE);
    assert!(
        packs.notices.is_empty(),
        "the timing pack loads cleanly: {:?}",
        packs.notices
    );

    let plan = hooks::plan_for(&packs);
    assert_eq!(
        plan.packs()
            .iter()
            .map(|programs| programs.programs.len())
            .sum::<usize>(),
        1,
        "the pack declares one timing hook"
    );
    let host = Rc::new(tclvm_host());
    for programs in plan.packs() {
        let installed = host.install_pack_hooks(programs.clone());
        assert!(
            installed.iter().all(|entry| entry.declined.is_none()),
            "the timing hook installs: {installed:?}"
        );
    }
    pack_hooks::install_host(host);

    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
    assert_eq!(
        registry.script_timing("mylib::callback", &["now", "callback"], 1, None,),
        Some(ScriptTiming::SameInvocation),
    );
    assert_eq!(
        registry.script_timing("mylib::callback", &["later", "callback"], 1, None,),
        Some(ScriptTiming::Deferred),
    );
    pack_hooks::clear_host();
}
