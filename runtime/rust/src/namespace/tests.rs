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

use super::*;
use crate::interp::Code;

fn dummy(_i: &mut crate::interp::Interp, _a: &[*mut crate::obj::TclObj]) -> Code {
    Code::Ok
}
fn cmd() -> Command {
    Command::Builtin(dummy)
}
fn is_some(c: Option<Command>) -> bool {
    c.is_some()
}

#[test]
fn command_lookup_currency_keeps_unrelated_topology_but_rejects_foreign_worlds() {
    let mut world = Namespaces::new();
    world.bind(GLOBAL, b"kept", cmd());
    let generation = world.command_generation(GLOBAL, b"kept").unwrap();
    let lookup = world.command_guard_lookup(GLOBAL, generation).unwrap();
    let unrelated = world.ensure_namespace(GLOBAL, b"other");
    assert!(world.command_guard_lookup_is_current(&lookup, GLOBAL, generation));
    world.delete_namespace_by_id(unrelated);
    assert!(world.command_guard_lookup_is_current(&lookup, GLOBAL, generation));
    let mut foreign = Namespaces::new();
    foreign.bind(GLOBAL, b"kept", cmd());
    let other = foreign.command_generation(GLOBAL, b"kept").unwrap();
    assert_eq!(generation, other);
    assert!(!foreign.command_guard_lookup_is_current(&lookup, GLOBAL, other));
    drop(world);
    assert!(!foreign.command_guard_lookup_is_current(&lookup, GLOBAL, other));
    assert!(foreign.command_guard_lookup(GLOBAL, other).is_some());
    let actual = foreign.command_guard_lookup(GLOBAL, other).unwrap();
    foreign.bind(GLOBAL, b"kept", cmd());
    let replacement = foreign.command_generation(GLOBAL, b"kept").unwrap();
    assert_ne!(other, replacement);
    assert!(!foreign.command_guard_lookup_is_current(&actual, GLOBAL, replacement));
    assert!(!foreign.command_guard_lookup_is_current(&actual, GLOBAL, other));
}

#[test]
fn command_lookup_currency_checks_actual_path_holder_lifetime_and_policy_exhaustion() {
    let mut world = Namespaces::new();
    let holder = world.ensure_namespace(GLOBAL, b"holder");
    let caller = world.ensure_namespace(GLOBAL, b"caller");
    world.bind(holder, b"kept", cmd());
    let generation = world.command_generation(holder, b"kept").unwrap();
    world.set_path(caller, vec![holder]);
    let lookup = world.command_guard_lookup(caller, generation).unwrap();
    assert!(world.command_guard_lookup_is_current(&lookup, caller, generation));
    assert!(!world.command_guard_lookup_is_current(&lookup, GLOBAL, generation));
    world.set_path(caller, vec![GLOBAL]);
    assert!(!world.command_guard_lookup_is_current(&lookup, caller, generation));
    world.set_path(caller, vec![holder]);
    assert!(!world.command_guard_lookup_is_current(&lookup, caller, generation));
    let current = world.command_guard_lookup(caller, generation).unwrap();
    world.begin_namespace_variable_teardown(holder);
    assert!(!world.command_guard_lookup_is_current(&current, caller, generation));
    assert!(world.command_guard_lookup(caller, generation).is_none());
    world.bind(GLOBAL, b"root", cmd());
    let root = world.command_generation(GLOBAL, b"root").unwrap();
    world.guard_command_lookup_policy_revision = Some(u64::MAX);
    world.set_path(GLOBAL, Vec::new());
    assert!(world.guard_command_lookup_policy_revision().is_none());
    assert!(world.command_guard_lookup(GLOBAL, root).is_none());
}

#[test]
fn command_lookup_currency_keeps_absent_unknown_retirement_but_rejects_real_changes() {
    let mut world = Namespaces::new();
    world.bind(GLOBAL, b"kept", cmd());
    let generation = world.command_generation(GLOBAL, b"kept").unwrap();
    let original = world.command_guard_lookup(GLOBAL, generation).unwrap();
    let unrelated = world.ensure_namespace(GLOBAL, b"unrelated");
    let broad = world.guard_lookup_environment_revision();
    let policy = world.guard_command_lookup_policy_revision();
    assert!(world.set_unknown_handler(unrelated, None).is_none());
    assert_ne!(world.guard_lookup_environment_revision(), broad);
    assert_eq!(world.guard_command_lookup_policy_revision(), policy);
    assert!(world.command_guard_lookup_is_current(&original, GLOBAL, generation));

    let handler = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"::unknown"));
    assert!(!handler.as_ptr().is_null());
    assert!(world
        .set_unknown_handler(unrelated, Some(handler))
        .is_none());
    assert!(!world.command_guard_lookup_is_current(&original, GLOBAL, generation));
    let installed = world.command_guard_lookup(GLOBAL, generation).unwrap();
    let retired = world.set_unknown_handler(unrelated, None);
    assert!(retired.is_some());
    assert!(!world.command_guard_lookup_is_current(&installed, GLOBAL, generation));
    drop(retired);

    let cleared = world.command_guard_lookup(GLOBAL, generation).unwrap();
    assert!(world.set_unknown_handler(unrelated, None).is_none());
    assert!(world.command_guard_lookup_is_current(&cleared, GLOBAL, generation));
    world.set_path(unrelated, vec![GLOBAL]);
    assert!(!world.command_guard_lookup_is_current(&cleared, GLOBAL, generation));
}

#[test]
fn publication_revision_keeps_fresh_births_but_rejects_slot_retirement() {
    let mut world = Namespaces::new();
    world.bind(GLOBAL, b"kept", cmd());
    let kept = world.command_generation(GLOBAL, b"kept").unwrap();
    let before = world.original_publication_revision().unwrap();
    world.bind(GLOBAL, b"fresh", cmd());
    assert!(before.matches(&world.original_publication_revision().unwrap()));
    assert!(world.original_publication_is_current(GLOBAL, b"kept", kept));
    world.bind(GLOBAL, b"kept", cmd());
    assert!(!before.matches(&world.original_publication_revision().unwrap()));
    assert!(!world.original_publication_is_current(GLOBAL, b"kept", kept));
}

#[test]
fn publication_revision_rejects_all_raw_withdrawal_paths() {
    for mutation in [
        "unlink", "take", "hide", "retire", "clear", "subtree", "owners",
    ] {
        let mut world = Namespaces::new();
        world.bind(GLOBAL, b"kept", cmd());
        let kept = world.command_generation(GLOBAL, b"kept").unwrap();
        let before = world.original_publication_revision().unwrap();
        match mutation {
            "unlink" => {
                drop(world.unlink_original_command_generation(kept));
            }
            "take" => {
                drop(world.take(GLOBAL, b"kept"));
            }
            "hide" => world.note_native_command_hidden(kept),
            "retire" => world.retire_native_command_node(kept),
            "clear" => {
                drop(world.clear_namespace_ids(&[GLOBAL]));
            }
            "subtree" => world.delete_subtree(GLOBAL),
            "owners" => {
                drop(world.take_jim_command_owners());
            }
            _ => unreachable!(),
        }
        assert!(
            !before.matches(&world.original_publication_revision().unwrap()),
            "{mutation}"
        );
        assert!(
            !world.original_publication_is_current(GLOBAL, b"kept", kept),
            "{mutation}"
        );
    }
}

#[test]
fn publication_revision_rejects_tentative_alias_slots_and_restore() {
    let mut world = Namespaces::new();
    world.bind(GLOBAL, b"target", cmd());
    let target = world.command_generation(GLOBAL, b"target").unwrap();
    world.bind(
        GLOBAL,
        b"alias",
        Command::Alias {
            original_prefix: std::rc::Rc::new(crate::interp::NativeAliasHeader::new(
                crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"worker")),
            )),
            registration_name: None,
            publication_name: b"alias".to_vec(),
            identity: std::rc::Rc::new(()),
        },
    );
    let before = world.original_publication_revision().unwrap();
    let probe = world
        .begin_alias_rename_probe(GLOBAL, b"alias", b"target")
        .unwrap();
    assert!(!world.original_publication_is_current(GLOBAL, b"target", target));
    assert!(!before.matches(&world.original_publication_revision().unwrap()));
    let tentative = world.original_publication_revision().unwrap();
    drop(world.restore_alias_rename_probe(probe));
    assert!(world.original_publication_is_current(GLOBAL, b"target", target));
    assert!(!tentative.matches(&world.original_publication_revision().unwrap()));
}

#[test]
fn publication_revision_rejects_jim_local_shadow_and_restoration() {
    let mut world = Namespaces::new();
    world.bind(GLOBAL, b"kept", cmd());
    let original = world.command_generation(GLOBAL, b"kept").unwrap();
    let before = world.original_publication_revision().unwrap();
    let entry = world.native_command_creation_entry(GLOBAL, b"kept");
    let top = world.bind_jim_local(entry, b"kept", cmd());
    assert!(!world.original_publication_is_current(GLOBAL, b"kept", original));
    assert!(!before.matches(&world.original_publication_revision().unwrap()));
    let shadowed = world.original_publication_revision().unwrap();
    world.clean_jim_local_key(b"kept");
    assert!(world.original_publication_is_current(GLOBAL, b"kept", original));
    assert!(!world.original_publication_is_current(GLOBAL, b"kept", top));
    assert!(!shadowed.matches(&world.original_publication_revision().unwrap()));
}

#[test]
fn publication_revision_refuses_foreign_dead_and_exhausted_worlds() {
    let before = Namespaces::new().original_publication_revision().unwrap();
    assert!(!before.matches(&before));
    let mut world = Namespaces::new();
    let current = world.original_publication_revision().unwrap();
    assert!(!before.matches(&current));
    let foreign = Namespaces::new();
    assert!(!current.matches(&foreign.original_publication_revision().unwrap()));
    world.original_publication_revision = Some(u64::MAX);
    world.invalidate_original_publications();
    assert!(world.original_publication_revision().is_none());
    world.bind(GLOBAL, b"fresh", cmd());
    world.invalidate_original_publications();
    assert!(world.original_publication_revision().is_none());
}

#[test]
fn publication_revision_tracks_moves_and_callback_rebound_rename_sources() {
    let mut world = Namespaces::new();
    world.bind(GLOBAL, b"old", cmd());
    let moved = world.command_generation(GLOBAL, b"old").unwrap();
    let before = world.original_publication_revision().unwrap();
    let publication = world
        .publish_rename_destination(GLOBAL, b"old", b"new")
        .unwrap();
    assert!(!before.matches(&world.original_publication_revision().unwrap()));
    assert!(world.original_publication_is_current(GLOBAL, b"new", moved));
    assert!(!world.original_publication_is_current(GLOBAL, b"old", moved));
    world.bind(GLOBAL, b"old", cmd());
    let rebound = world.command_generation(GLOBAL, b"old").unwrap();
    let rebound_revision = world.original_publication_revision().unwrap();
    world.retire_rename_source(&publication);
    assert!(!rebound_revision.matches(&world.original_publication_revision().unwrap()));
    assert!(!world.original_publication_is_current(GLOBAL, b"old", rebound));
    assert!(world.original_publication_is_current(GLOBAL, b"new", moved));
}

#[test]
fn publication_revision_tracks_occupied_jim_value_replacement() {
    let mut world = Namespaces::new();
    world.variable_name_protocol = Some(tcl_syntax::naming::NativeNameProtocol::Jim084);
    world.bind(GLOBAL, b"kept", cmd());
    let old = world.command_generation(GLOBAL, b"kept").unwrap();
    let before = world.original_publication_revision().unwrap();
    world.bind(GLOBAL, b"kept", cmd());
    assert!(!before.matches(&world.original_publication_revision().unwrap()));
    assert!(!world.original_publication_is_current(GLOBAL, b"kept", old));
    assert_ne!(world.command_generation(GLOBAL, b"kept"), Some(old));
}

#[test]
fn single_unlink_witness_requires_actual_c_and_exact_previous_revision() {
    use tcl_syntax::naming::NativeNameProtocol;
    for protocol in [
        Some(NativeNameProtocol::C(tcl_dialect::TclVersion::V9_0)),
        Some(NativeNameProtocol::Jim084),
        None,
    ] {
        let mut world = Namespaces::new();
        world.variable_name_protocol = protocol;
        world.bind(GLOBAL, b"first", cmd());
        world.bind(GLOBAL, b"second", cmd());
        let first = world.command_generation(GLOBAL, b"first").unwrap();
        let second = world.command_generation(GLOBAL, b"second").unwrap();
        let before = world.original_publication_revision().unwrap();
        drop(world.unlink_original_command_generation(first));
        let once = world.original_publication_revision().unwrap();
        let expected = matches!(protocol, Some(NativeNameProtocol::C(_))).then_some(first);
        assert_eq!(once.single_unlink_since(&before), expected);
        assert!(!world.original_publication_is_current(GLOBAL, b"first", first));
        assert!(world.original_publication_is_current(GLOBAL, b"second", second));
        drop(world.unlink_original_command_generation(second));
        let twice = world.original_publication_revision().unwrap();
        assert_eq!(twice.single_unlink_since(&before), None);
        assert_eq!(twice.single_unlink_since(&once), expected.map(|_| second));
    }
}

#[test]
fn single_unlink_witness_refuses_broad_foreign_dead_and_exhausted_history() {
    let mut world = Namespaces::new();
    world.variable_name_protocol = Some(tcl_syntax::naming::NativeNameProtocol::C(
        tcl_dialect::TclVersion::V9_0,
    ));
    world.bind(GLOBAL, b"kept", cmd());
    let generation = world.command_generation(GLOBAL, b"kept").unwrap();
    let before = world.original_publication_revision().unwrap();
    let foreign = Namespaces::new();
    let foreign_before = foreign.original_publication_revision().unwrap();
    drop(world.unlink_original_command_generation(generation));
    let after = world.original_publication_revision().unwrap();
    assert_eq!(after.single_unlink_since(&before), Some(generation));
    assert_eq!(after.single_unlink_since(&foreign_before), None);
    drop(world);
    assert_eq!(after.single_unlink_since(&before), None);
    drop(foreign);

    for broad_first in [false, true] {
        let mut world = Namespaces::new();
        world.variable_name_protocol = Some(tcl_syntax::naming::NativeNameProtocol::C(
            tcl_dialect::TclVersion::V9_0,
        ));
        world.bind(GLOBAL, b"first", cmd());
        let generation = world.command_generation(GLOBAL, b"first").unwrap();
        let before = world.original_publication_revision().unwrap();
        if broad_first {
            world.invalidate_original_publications();
        }
        drop(world.unlink_original_command_generation(generation));
        if !broad_first {
            world.invalidate_original_publications();
        }
        let after = world.original_publication_revision().unwrap();
        assert_eq!(after.single_unlink_since(&before), None);
        let foreign = Namespaces::new().original_publication_revision().unwrap();
        assert_eq!(after.single_unlink_since(&foreign), None);
        drop(world);
        assert_eq!(after.single_unlink_since(&before), None);
    }
    let mut world = Namespaces::new();
    world.original_publication_revision = Some(u64::MAX);
    world.invalidate_single_original_unlink(42);
    assert!(world.original_publication_revision().is_none());
    assert!(world.original_publication_single_unlink.is_none());
    world.invalidate_single_original_unlink(43);
    assert!(world.original_publication_revision().is_none());
    assert!(world.original_publication_single_unlink.is_none());
}

#[test]
fn support_publication_keeps_actual_relative_holders_and_counted_empty_tails() {
    let mut namespaces = Namespaces::new();
    let current = namespaces.ensure_child(GLOBAL, b":");
    let child = namespaces.ensure_child(current, b"child");
    assert_eq!(
        namespaces.support_command_publication_at(current, b"p"),
        (current, b"p".to_vec())
    );
    assert_eq!(
        namespaces.support_command_publication_at(current, b"::p"),
        (GLOBAL, b"p".to_vec())
    );
    assert_eq!(
        namespaces.support_command_publication_at(current, b"child::\xff\0tail"),
        (child, b"\xff\0tail".to_vec())
    );
    assert_eq!(
        namespaces.support_command_publication_at(current, b"child::"),
        (child, Vec::new())
    );
}

#[test]
fn bootstrap_registration_and_restore_keep_the_last_nonempty_component() {
    let mut namespaces = Namespaces::new();
    assert!(namespaces.register_at(b"", cmd()).is_none());
    assert!(namespaces.register_at(b"::::", cmd()).is_none());
    for name in [b"::A:::b::".as_slice(), b"\xff::k\0tail::"] {
        let (namespace, simple) = namespaces.register_at(name, cmd()).unwrap();
        assert_eq!(
            simple,
            if name.starts_with(b"::A") {
                b"b".as_slice()
            } else {
                b"k\0tail".as_slice()
            },
        );
        let generation = namespaces.command_generation(namespace, &simple).unwrap();
        let binding = namespaces.arena[namespace]
            .commands
            .remove_binding(&simple)
            .unwrap();
        assert!(namespaces.restore(b"::D:::moved::", binding));
        let destination = namespaces.find_namespace(GLOBAL, b"D").unwrap();
        assert_eq!(
            namespaces.command_generation(destination, b"moved"),
            Some(generation)
        );
    }
}

#[test]
fn generation_location_projects_actual_constructed_slots_and_declines_replacement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Fixed table-owner control; no Native namespace or callback timing is asserted.
    let mut namespaces = Namespaces::new();
    let holder = namespaces.ensure_namespace(GLOBAL, b":");
    namespaces.bind(holder, b":run", cmd());
    let original = namespaces.command_generation(holder, b":run").unwrap();
    assert_eq!(
        namespaces.command_fqn_by_generation(original).as_deref(),
        Some(b"::::::run".as_slice()),
    );
    let moved = namespaces.take(holder, b":run").unwrap();
    assert!(namespaces.command_fqn_by_generation(original).is_none());
    assert!(namespaces.restore(b"other::moved", moved));
    let owned = namespaces.command_by_generation(original).unwrap();
    assert_eq!(owned.0, b"::other::moved");
    assert_eq!(
        namespaces.command_fqn_by_generation(original),
        Some(owned.0)
    );
    namespaces.register(b"other::moved", cmd());
    assert!(namespaces.command_fqn_by_generation(original).is_none());
    assert!(namespaces.command_by_generation(original).is_none());
    let replacement = namespaces
        .resolve_generation(GLOBAL, b"other::moved")
        .unwrap();
    assert_ne!(replacement, original);
    assert_eq!(
        namespaces.command_fqn_by_generation(replacement).as_deref(),
        Some(b"::other::moved".as_slice()),
    );
}

#[test]
fn generation_publication_projection_excludes_the_actual_saved_jim_previous_worker() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Real previous-edge and restoration workers; this is a software ownership control.
    let mut namespaces = Namespaces::new();
    namespaces.bind(GLOBAL, b"kept", cmd());
    let original = namespaces.command_generation(GLOBAL, b"kept").unwrap();
    let entry = namespaces.native_command_creation_entry(GLOBAL, b"kept");
    let top = namespaces.bind_jim_local(entry, b"kept", cmd());
    assert_ne!(original, top);
    assert_eq!(
        namespaces.native_command_at_node(original).unwrap().1,
        b"::kept"
    );
    assert_eq!(
        namespaces.command_by_generation(original).unwrap().0,
        b"::kept"
    );
    assert!(namespaces.command_fqn_by_generation(original).is_none());
    assert_eq!(
        namespaces.command_fqn_by_generation(top).as_deref(),
        Some(b"::kept".as_slice()),
    );
    namespaces.clean_jim_local_key(b"kept");
    assert_eq!(
        namespaces.command_fqn_by_generation(original).as_deref(),
        Some(b"::kept".as_slice()),
    );
    assert!(namespaces.command_fqn_by_generation(top).is_none());
}

#[test]
fn compiler_and_resolver_epochs_match_original_native_mutation_rows() {
    use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
    let cases = [
        (
            tcl_dialect::TclVersion::V8_4,
            include_str!("../../tests/data/native_compiler_epochs/8.4.20.tsv"),
        ),
        (
            tcl_dialect::TclVersion::V8_5,
            include_str!("../../tests/data/native_compiler_epochs/8.5.19.tsv"),
        ),
        (
            tcl_dialect::TclVersion::V8_6,
            include_str!("../../tests/data/native_compiler_epochs/8.6.18.tsv"),
        ),
        (
            tcl_dialect::TclVersion::V9_0,
            include_str!("../../tests/data/native_compiler_epochs/9.0.4.tsv"),
        ),
        (
            tcl_dialect::TclVersion::V9_1,
            include_str!("../../tests/data/native_compiler_epochs/9.1.0.tsv"),
        ),
    ];
    for (version, rows) in cases {
        let mut namespaces = Namespaces::new();
        namespaces.set_native_command_version(Some(version));
        namespaces.bind(GLOBAL, b"set", cmd());
        let stock = namespaces.resolve_generation(GLOBAL, b"set").unwrap();
        namespaces.set_native_compiler_hook(stock, Hook::Present);
        let baseline = rows
            .lines()
            .nth(1)
            .unwrap()
            .split('\t')
            .nth(2)
            .unwrap()
            .parse::<u64>()
            .unwrap();
        let mut local = None;
        let mut hidden = None;
        for row in rows.lines().skip(1) {
            let fields: Vec<_> = row.split('\t').collect();
            let event = fields[0];
            match event {
                "initial" => {}
                "namespace_create" | "namespace_recreate" => {
                    local = Some(namespaces.ensure_namespace(GLOBAL, b"N"));
                }
                "plain_create" => namespaces.bind(GLOBAL, b"p", cmd()),
                "plain_rename" | "compiled_rename" | "compiled_restore" => {
                    let (from, to) = match event {
                        "plain_rename" => (b"p".as_slice(), b"q".as_slice()),
                        "compiled_rename" => (b"set".as_slice(), b"stamp_saved_set".as_slice()),
                        _ => (b"stamp_saved_set".as_slice(), b"set".as_slice()),
                    };
                    let publication = namespaces
                        .publish_rename_destination(GLOBAL, from, to)
                        .unwrap();
                    namespaces.retire_rename_source(&publication);
                }
                "plain_delete" => assert!(namespaces.delete(GLOBAL, b"q")),
                "compiled_shadow" => namespaces.bind(local.unwrap(), b"set", cmd()),
                "shadow_delete" => assert!(namespaces.delete(local.unwrap(), b"set")),
                "replacement_plain" => namespaces.bind(GLOBAL, b"set", cmd()),
                "replacement_delete" => assert!(namespaces.delete(GLOBAL, b"set")),
                "compiled_hide" => {
                    // The hidden table owns this exact token between operations.
                    let binding = namespaces.take(GLOBAL, b"set").unwrap();
                    namespaces.note_native_command_hidden(binding.generation);
                    assert_eq!(binding.generation, stock);
                    hidden = Some(binding);
                }
                "compiled_expose" => {
                    assert!(namespaces.restore(b"set", hidden.take().unwrap()));
                }
                "namespace_path" | "same_path" => {
                    let target = namespaces.ensure_namespace(GLOBAL, b"M");
                    namespaces.set_path(local.unwrap(), vec![target]);
                }
                "clear_path" => namespaces.set_path(local.unwrap(), Vec::new()),
                "namespace_delete" => {
                    assert!(namespaces.delete_namespace(GLOBAL, b"N"));
                    local = None;
                }
                _ => panic!("unhandled original native event {event}"),
            }
            assert_eq!(fields[1], "0", "{version:?} {event}: native completion");
            let expected_compiler = fields[2].parse::<u64>().unwrap() - baseline;
            assert_eq!(
                namespaces.native_compiler_cache_epochs(GLOBAL),
                Some((expected_compiler, fields[3].parse().unwrap())),
                "{version:?} {event}: global stamp"
            );
            match local {
                Some(local) => assert_eq!(
                    namespaces.native_compiler_cache_epochs(local),
                    Some((expected_compiler, fields[4].parse().unwrap())),
                    "{version:?} {event}: original namespace stamp"
                ),
                None => assert_eq!(fields[4], "-1", "{version:?} {event}"),
            }
        }
    }
}

#[test]
fn imported_raw_compiler_recipe_is_independent_of_future_origin_header() {
    let mut namespaces = Namespaces::new();
    namespaces.bind(GLOBAL, b"origin", cmd());
    let origin = namespaces.resolve_generation(GLOBAL, b"origin").unwrap();
    namespaces.set_native_compiler_recipe(origin, NativeCompilerRecipe::ProcedureNoOp);
    namespaces.bind(
        GLOBAL,
        b"copied",
        Command::Imported {
            source: b"::origin".to_vec(),
            source_generation: origin,
            ensemble: None,
            identity: std::rc::Rc::new(crate::interp::ImportToken),
        },
    );
    let imported = namespaces.resolve_generation(GLOBAL, b"copied").unwrap();
    namespaces.set_native_compiler_recipe(origin, NativeCompilerRecipe::Absent);
    assert_eq!(
        namespaces.native_compiler_recipe(imported),
        Some(NativeCompilerRecipe::ProcedureNoOp)
    );
    namespaces.set_native_compiler_recipe(origin, NativeCompilerRecipe::Unknown);
    assert_eq!(
        namespaces.native_compiler_recipe(imported),
        Some(NativeCompilerRecipe::ProcedureNoOp)
    );
}

#[test]
fn imported_raw_compiler_attachment_is_copied_once() {
    use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
    let mut namespaces = Namespaces::new();
    namespaces.bind(GLOBAL, b"origin", cmd());
    let origin = namespaces.resolve_generation(GLOBAL, b"origin").unwrap();
    namespaces.bind(
        GLOBAL,
        b"old",
        Command::Imported {
            source: b"::origin".to_vec(),
            source_generation: origin,
            ensemble: None,
            identity: std::rc::Rc::new(crate::interp::ImportToken),
        },
    );
    let old = namespaces.resolve_generation(GLOBAL, b"old").unwrap();
    namespaces.set_native_compiler_hook(origin, Hook::Present);
    assert_eq!(namespaces.native_compiler_hook(old), Some(Hook::Absent));
    namespaces.bind(
        GLOBAL,
        b"new",
        Command::Imported {
            source: b"::origin".to_vec(),
            source_generation: origin,
            ensemble: None,
            identity: std::rc::Rc::new(crate::interp::ImportToken),
        },
    );
    let new = namespaces.resolve_generation(GLOBAL, b"new").unwrap();
    assert_eq!(namespaces.native_compiler_hook(new), Some(Hook::Present));
    assert!(namespaces.delete(GLOBAL, b"old"));
    assert_eq!(
        namespaces.native_compiler_cache_epochs(GLOBAL),
        Some((0, 0))
    );
    assert!(namespaces.delete(GLOBAL, b"new"));
    assert_eq!(
        namespaces.native_compiler_cache_epochs(GLOBAL),
        Some((1, 0))
    );
}

#[test]
fn command_reference_epochs_follow_shadow_trails_and_reverse_path_entries() {
    let mut namespaces = Namespaces::new();
    let local = namespaces.ensure_namespace(GLOBAL, b"local");
    let watcher = namespaces.ensure_namespace(GLOBAL, b"watcher");
    namespaces.set_path(watcher, vec![local, local]);
    namespaces.bind(GLOBAL, b"p", cmd());
    let watcher_before = namespaces
        .native_command_reference(watcher)
        .command_reference_epoch;
    let local_before = namespaces
        .native_command_reference(local)
        .command_reference_epoch;
    namespaces.bind(local, b"p", cmd());
    // Fresh publication invalidates two path entries, then shadowing the
    // global p invalidates the local reference and the same two entries.
    assert_eq!(
        namespaces
            .native_command_reference(watcher)
            .command_reference_epoch,
        watcher_before + 4
    );
    assert_eq!(
        namespaces
            .native_command_reference(local)
            .command_reference_epoch,
        local_before + 1
    );
    let global_child = namespaces.ensure_namespace(GLOBAL, b"child");
    let local_child = namespaces.ensure_namespace(local, b"child");
    namespaces.bind(global_child, b"q", cmd());
    let before = namespaces
        .native_command_reference(local)
        .command_reference_epoch;
    namespaces.bind(local_child, b"q", cmd());
    assert_eq!(
        namespaces
            .native_command_reference(local)
            .command_reference_epoch,
        before + 1
    );
    assert_eq!(
        namespaces
            .native_command_reference(local_child)
            .command_reference_epoch,
        0
    );
}

#[test]
fn c85_obj_creation_invalidates_reverse_paths_before_delete_callback_window() {
    let mut namespaces = Namespaces::new();
    namespaces.set_native_command_version(Some(tcl_dialect::TclVersion::V8_5));
    let source = namespaces.ensure_namespace(GLOBAL, b"source");
    let watcher = namespaces.ensure_namespace(GLOBAL, b"watcher");
    namespaces.set_path(watcher, vec![source, source]);
    namespaces.bind(source, b"p", cmd());
    let before = namespaces
        .native_command_reference(watcher)
        .command_reference_epoch;
    let entry = namespaces.native_command_creation_entry(source, b"p");
    assert_eq!(
        namespaces
            .native_command_reference(watcher)
            .command_reference_epoch,
        before + 2
    );
    namespaces.bind_after_native_creation_entry(entry, b"p", cmd());
    assert_eq!(
        namespaces
            .native_command_reference(watcher)
            .command_reference_epoch,
        before + 2
    );
    namespaces.set_native_command_version(Some(tcl_dialect::TclVersion::V8_4));
    let before = namespaces
        .native_command_reference(watcher)
        .command_reference_epoch;
    namespaces.bind(source, b"q", cmd());
    assert_eq!(
        namespaces
            .native_command_reference(watcher)
            .command_reference_epoch,
        before
    );
}

#[test]
fn constructed_path_and_lookup_stay_on_detached_original_token() {
    let mut namespaces = Namespaces::new();
    let original = namespaces.ensure_namespace(GLOBAL, b"a:");
    let child = namespaces.ensure_child(original, b"x\xff");
    namespaces.bind(original, b"p", cmd());
    let path = namespaces.native_context_path(original).unwrap();
    assert_eq!(
        path.as_segments(),
        &[tcl_core_types::NameBytes::from(b"a:".as_slice())]
    );
    namespaces.activation_enter(original);
    drop(namespaces.defer_namespace(original));
    let replacement = namespaces.ensure_namespace(GLOBAL, b"a:");
    assert_ne!(replacement, original);
    assert_eq!(namespaces.native_context_path(original), Some(path.clone()));
    assert_eq!(namespaces.native_context_path(replacement), Some(path));
    assert_eq!(
        namespaces.native_context_path(child).unwrap().as_segments(),
        &[
            tcl_core_types::NameBytes::from(b"a:".as_slice()),
            tcl_core_types::NameBytes::from(b"x\xff".as_slice()),
        ]
    );
    assert!(namespaces.resolve(original, b"p").is_some());
    assert!(namespaces.resolve(replacement, b"p").is_none());
    assert!(namespaces.activation_leave(original));
}

#[test]
fn unqualified_resolves_in_global() {
    let mut ns = Namespaces::new();
    ns.register(b"set", cmd());
    assert!(is_some(ns.resolve(GLOBAL, b"set")));
    assert!(!is_some(ns.resolve(GLOBAL, b"nope")));
}

#[test]
fn qualified_registration_and_resolution() {
    let mut ns = Namespaces::new();
    ns.register(b"::tcl::mathfunc::sin", cmd());
    ns.register(b"set", cmd());
    let mf = ns.find_namespace(GLOBAL, b"::tcl::mathfunc").unwrap();
    // qualified lookup hits the namespace directly
    assert!(is_some(ns.resolve(GLOBAL, b"::tcl::mathfunc::sin")));
    // `sin` is NOT visible unqualified from global
    assert!(!is_some(ns.resolve(GLOBAL, b"sin")));
    // an absolute reference to a global command works
    assert!(is_some(ns.resolve(mf, b"::set")));
    assert_eq!(ns.qualified_name(mf), b"::tcl::mathfunc");
}

#[test]
fn namespace_path_fallback() {
    let mut ns = Namespaces::new();
    ns.register(b"::tcl::mathop::+", cmd());
    let mathop = ns.find_namespace(GLOBAL, b"::tcl::mathop").unwrap();
    let foo = ns.ensure_namespace(GLOBAL, b"::foo");
    // bare `+` is not resolvable from ::foo …
    assert!(!is_some(ns.resolve(foo, b"+")));
    // … until ::tcl::mathop is on ::foo's namespace path.
    ns.set_path(foo, vec![mathop]);
    assert!(is_some(ns.resolve(foo, b"+")));
}
