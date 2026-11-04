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

//! The shipped UPF pack's optional option values and its one executable hook
//! body, each held to the answers it is written to give.
//!
//! `specs/upf.tclspec` states sixteen optional option values as data
//! (`-arity {OptionalOneOf TRUE FALSE}`, `-arity OptionalNotOption`), which
//! the registry answers without a VM, and carries one body: the
//! `define_power_model` argument-role resolver, run here on the production
//! host (`tclvm_host`) through the production plan (`hooks::plan_for` over
//! the bundled `specs/` set). The corpus gate asks only for one successful
//! invocation per hooked pack, so a body that silently abstains would pass
//! it; here the declared slot is exercised and must answer.

use std::path::PathBuf;

use tcl_registry::InvocationWordKind;
use tcl_registry::arg_role::ArgRole;
use tcl_registry::pack_hooks::{
    HookAnswer, HookCall, HookFamily, HookWord, OptionCallCtx, PackHookHost,
};
use tcl_spec_hooks::{HookProgram, tclvm_host};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The UPF pack's hook programs from the production plan, each paired with
/// the slot the production host bound it to.
fn upf_hooks(
    host: &tcl_spec_hooks::HookHost<tcl_engine_tclvm::TclVmEngine>,
) -> Vec<(HookProgram, tcl_registry::pack_hooks::HookSlot)> {
    let set = tcl_spectcl::bundled::load_from(&repo_root().join("specs"));
    let plan = tcl_spectcl::hooks::plan_for(&set);
    let upf = plan
        .packs()
        .iter()
        .find(|programs| programs.pack == "upf")
        .expect("the bundled UPF pack declares hook bodies");
    let installed = host.install_pack_hooks(upf.clone());
    assert_eq!(installed.len(), upf.programs.len());
    upf.programs
        .iter()
        .zip(installed)
        .map(|(program, installation)| {
            assert!(
                installation.declined.is_none(),
                "{} {:?}: {:?}",
                program.command,
                program.owner,
                installation.declined
            );
            let slot = installation.slot.expect("an installed hook has a slot");
            (program.clone(), slot)
        })
        .collect()
}

fn literal(value: &str) -> HookWord<'_> {
    HookWord {
        value,
        kind: InvocationWordKind::Literal,
    }
}

/// Invoke `slot` for `words`, with the option at `option_index` when given.
fn invoke(
    host: &dyn PackHookHost,
    slot: tcl_registry::pack_hooks::HookSlot,
    words: &[&str],
    option_index: Option<usize>,
) -> HookAnswer {
    let words: Vec<HookWord<'_>> = words.iter().map(|word| literal(word)).collect();
    let option = option_index.map(|index| OptionCallCtx {
        option: words[index].value,
        index,
        value_start: index + 1,
    });
    host.invoke(
        slot,
        &HookCall {
            words: &words,
            version: Some(tcl_dialect::TclVersion::V8_6),
            in_event_body: false,
            option,
            constraints: None,
            dialect: Some("tcl8.6"),
            targets: &[],
            budget: tcl_registry::value_transfer::ImplementationBudget::default(),
            depends: &[],
        },
    )
}

/// Every optional option value is consumed exactly when it is there, and a
/// following option or the end of the call is left alone. Fifteen options
/// take an optional `TRUE|FALSE`; `-feedthrough` (`set_port_attributes`)
/// takes an optional name, i.e. any word that does not start with `-`.
#[test]
fn every_upf_optional_option_value_is_consumed_exactly_when_present() {
    use tcl_registry::hover::{OptionArity, OptionValue};

    let set = tcl_spectcl::bundled::load_from(&repo_root().join("specs"));
    let upf = set
        .packs
        .iter()
        .find(|pack| pack.name == "upf")
        .expect("the bundled UPF pack");
    let options: Vec<_> = upf
        .commands
        .iter()
        .flat_map(|command| {
            command
                .spec
                .options
                .iter()
                .map(move |option| (command.spec.name, option))
        })
        .filter(|(_, option)| {
            matches!(
                option.value,
                OptionValue::Takes(arg)
                    if matches!(arg.arity, OptionArity::OptionalOneOf(_) | OptionArity::OptionalNotOption)
            )
        })
        .collect();
    assert_eq!(options.len(), 16, "sixteen optional option values");

    for (command, spec) in options {
        let option = spec.name;
        let OptionValue::Takes(arg) = spec.value else {
            unreachable!("filtered to valued options");
        };
        let cases: &[(&[&str], usize)] = match arg.arity {
            OptionArity::OptionalNotOption => &[
                (&[option, "feed1"], 1),
                (&["obj", option, "feed1", "-unconnected"], 1),
                (&[option, "-unconnected"], 0),
                (&[option], 0),
            ],
            OptionArity::OptionalOneOf(values) => {
                assert_eq!(values, ["TRUE", "FALSE"], "{command} {option}");
                &[
                    (&[option, "TRUE"], 1),
                    (&[option, "FALSE"], 1),
                    (&["obj", option, "FALSE", "-x"], 1),
                    (&[option, "maybe"], 0),
                    (&[option, "-elements"], 0),
                    (&[option], 0),
                ]
            }
            _ => unreachable!("filtered to optional arities"),
        };
        for (words, expected) in cases {
            let index = words
                .iter()
                .position(|word| *word == option)
                .expect("option word");
            assert_eq!(
                spec.value_word_count(words, index),
                *expected,
                "{command} {option}: {words:?}"
            );
        }
    }
}

/// `define_power_model`'s resolver gives the Body role to the last word of
/// any call with at least two words, wherever the options leave it, and says
/// nothing for a shorter call.
#[test]
fn the_define_power_model_resolver_marks_the_trailing_body() {
    let host = tclvm_host();
    let hooks = upf_hooks(&host);
    let resolvers: Vec<_> = hooks
        .iter()
        .filter(|(program, _)| program.family == HookFamily::ArgRoleResolver)
        .collect();
    assert_eq!(resolvers.len(), 1, "one argument-role resolver body");
    let (program, slot) = resolvers[0];
    assert_eq!(program.command, "define_power_model");

    assert_eq!(
        invoke(&host, *slot, &["model", "{set x 1}"], None),
        HookAnswer::Roles(vec![(1, ArgRole::Body)])
    );
    assert_eq!(
        invoke(
            &host,
            *slot,
            &["model", "-for", "{m1 m2}", "{set x 1}"],
            None
        ),
        HookAnswer::Roles(vec![(3, ArgRole::Body)])
    );
    assert_eq!(invoke(&host, *slot, &["model"], None), HookAnswer::Abstain);
    assert_eq!(hooks.len(), 1, "the resolver is the pack's only body");
}

/// The shipped resolver's answers are precompiled into the binary
/// (`cargo xtask gen-bundled-manifests`): published, it answers through its
/// installed spec, and nothing starts a VM.
#[test]
fn the_shipped_resolver_answers_from_its_precompiled_table() {
    let set = tcl_spectcl::bundled::load_from(&repo_root().join("specs"));
    tcl_spectcl::hooks::publish(&set);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("xilinx-eda-tcl", &set);
    let resolver = registry
        .get("define_power_model")
        .and_then(|spec| spec.arg_role_resolver)
        .expect("define_power_model carries its resolver");
    assert_eq!(resolver(&["model", "{set x 1}"]), vec![(1, ArgRole::Body)]);
    assert_eq!(
        resolver(&["model", "-for", "{m1 m2}", "{set x 1}"]),
        vec![(3, ArgRole::Body)]
    );
    assert_eq!(resolver(&["model"]), Vec::new());
    assert_eq!(tcl_spectcl::hooks::hosts_built(), 0, "no VM was started");
}
