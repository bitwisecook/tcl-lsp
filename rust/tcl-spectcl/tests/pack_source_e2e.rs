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

//! From `.tclspec` source to a folded call site, through the real loader.
//!
//! [`const_fold_e2e`] proves the host and the optimiser agree; this proves the
//! *seam* between the loader and the host — that a hook written in a pack file
//! as a `const_fold {words ctx} { … }` statement, carried by the loader as a
//! `HookDecl`, becomes the function pointer a `CommandSpec` holds.
//!
//! The adapter below (`programs_of`) is the whole bridge, and it is
//! deliberately in a test rather than in either crate: the loader depends on no
//! host type, the host depends on no loader type, and the dozen lines that
//! join them belong to whoever owns both — today the pack-installation path,
//! tomorrow the LSP's workspace loader.
//!
//! [`const_fold_e2e`]: ../const_fold_e2e/index.html

use std::rc::Rc;

use tcl_compiler::optimiser::manager::optimise_raw;
use tcl_registry::CommandRegistry;
use tcl_registry::pack_hooks::{self, HookInputs};
use tcl_registry::spec::CommandSpec;
use tcl_spec_hooks::{HookOwner, HookProgram, PackPrograms, tclvm_host};
use tcl_spectcl::loader::{self, HookOwner as DeclOwner, HookSource, Pack};

/// A pack file: one command, whose folder is a Tcl body.
const SOURCE: &str = r#"
speclib mylib 1 {
    command mylib::strlen {
        arity 1
        arg 0 -role Value
        const_fold -inputs {words} {words ctx} {
            set subject [lindex $words 0]
            if {![string is ascii $subject]} { return }
            fold [string length $subject]
        }
    }
}
"#;

/// The bridge: the loader's `HookDecl`s for one command become the host's
/// `HookProgram`s. Only bodies cross — a `-native` id names engine code and a
/// derivation keyword is the loader's own business.
fn programs_of(pack: &Pack) -> PackPrograms {
    let mut programs = PackPrograms::new(pack.name.clone());
    programs.dsl_version = pack.dsl_version.clone();
    for command in &pack.commands {
        for hook in &command.hooks {
            let HookSource::Body {
                params,
                body,
                inputs,
            } = &hook.source
            else {
                continue;
            };
            let owner = match &hook.owner {
                DeclOwner::Command => HookOwner::Command,
                DeclOwner::Subcommand(name) => HookOwner::Subcommand(name.clone()),
                DeclOwner::Option { subcommand, option } => HookOwner::Option {
                    subcommand: subcommand.clone(),
                    option: option.clone(),
                },
            };
            programs.programs.push(HookProgram {
                command: command.spec.name.to_owned(),
                owner,
                family: hook.family,
                parameters: params.clone(),
                body: body.clone(),
                inputs: inputs.clone(),
            });
        }
    }
    programs
}

#[test]
fn the_loader_carries_the_declared_inputs() {
    let pack = loader::load_pack(SOURCE);
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
    let pack = loader::load_pack(SOURCE);
    assert!(
        pack.notices.is_empty(),
        "the pack loads cleanly: {:?}",
        pack.notices
    );

    let host = Rc::new(tclvm_host());
    let installed = host.load_pack(programs_of(&pack));
    pack_hooks::install_host(host);
    let [installation] = &installed[..] else {
        panic!("one installed hook");
    };
    let slot = installation.slot.expect("the hook claimed a slot");

    // Installation: the loaded spec, with the pack's folder in place of the
    // abstaining placeholder the loader installs.
    let mut spec: CommandSpec = pack
        .command("mylib::strlen")
        .expect("declared")
        .spec
        .clone();
    spec.const_fold = pack_hooks::const_fold_fn(slot);

    let mut registry = CommandRegistry::build_default();
    registry.insert(spec);

    let folded: Vec<String> = optimise_raw(
        "proc ::f {} {\n    set y [mylib::strlen abcde]\n}\n",
        &registry,
        None,
    )
    .into_iter()
    .filter(|optimisation| optimisation.code.as_str() == "O129")
    .map(|optimisation| optimisation.replacement)
    .collect();
    assert_eq!(folded, vec!["5".to_string()]);
    pack_hooks::clear_host();
}

#[test]
fn without_the_host_the_loaded_pack_simply_does_not_fold() {
    // The loader installs an abstaining placeholder, so a pack loaded with no
    // hook host behaves exactly as it did before hooks could run: the command
    // keeps its declarative facts and nothing folds.
    let pack = loader::load_pack(SOURCE);
    let mut registry = CommandRegistry::build_default();
    registry.insert(
        pack.command("mylib::strlen")
            .expect("declared")
            .spec
            .clone(),
    );
    let folded = optimise_raw(
        "proc ::f {} {\n    set y [mylib::strlen abcde]\n}\n",
        &registry,
        None,
    )
    .into_iter()
    .filter(|optimisation| optimisation.code.as_str() == "O129")
    .count();
    assert_eq!(folded, 0);
}
