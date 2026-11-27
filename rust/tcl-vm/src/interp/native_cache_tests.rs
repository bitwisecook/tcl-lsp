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

//! Actual raw command attachments and native compiler epoch transitions.

use super::*;
use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;

fn native_vm(engine: &str) -> Vm {
    Vm::with_native_core(
        Box::new(Vec::<u8>::new()),
        Rc::new(crate::host_native::NativeHost::new()),
        tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("actual native core")
}

fn invoke(vm: &mut Vm, command: &str, arguments: &[&str]) -> Code {
    let words = arguments
        .iter()
        .map(|word| Value::string(*word))
        .collect::<Vec<_>>();
    vm.invoke_command(command, &words).code
}

fn native_epoch_baseline(reports: &str) -> u64 {
    tcl_test_support::fixture::tsv_fields(reports.lines().nth(1).unwrap())
        .nth(2)
        .unwrap()
        .parse::<u64>()
        .unwrap()
}

fn raw_hook(vm: &Vm, key: &str) -> i32 {
    let key = CommandSidecarKey::visible(key);
    if vm.command_at_sidecar_key(&key).is_none() {
        return -1;
    }
    match vm.native_hook_at_sidecar(&key) {
        Hook::Absent => 0,
        Hook::Present => 1,
        Hook::Unknown => panic!("raw native attachment unavailable at {key:?}"),
    }
}

#[test]
fn file_worker_runtime_headers_match_all_190_original_native_snapshots() {
    let fixtures = [
        (
            "tcl8.4",
            include_str!("../../../tcl-registry/tests/data/native_file_compilers/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tcl-registry/tests/data/native_file_compilers/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tcl-registry/tests/data/native_file_compilers/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tcl-registry/tests/data/native_file_compilers/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tcl-registry/tests/data/native_file_compilers/9.1.0.tsv"),
        ),
    ];
    let mut checked = 0;
    for (engine, fixture) in fixtures {
        let vm = native_vm(engine);
        for row in fixture.lines().skip(1) {
            let (identity, native_hook) = tcl_test_support::fixture::tsv_split_once(row).unwrap();
            assert_eq!(
                raw_hook(&vm, identity.trim_start_matches("::")),
                native_hook.parse::<i32>().unwrap(),
                "{engine}/{identity}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 190);
}

const IMPORTED_COMPILER_FIXTURES: [(&str, &str); 5] = [
    (
        "tcl8.4",
        include_str!("../../tests/data/native_import_hook_copy/8.4.20.tsv"),
    ),
    (
        "tcl8.5",
        include_str!("../../tests/data/native_import_hook_copy/8.5.19.tsv"),
    ),
    (
        "tcl8.6",
        include_str!("../../tests/data/native_import_hook_copy/8.6.18.tsv"),
    ),
    (
        "tcl9.0",
        include_str!("../../tests/data/native_import_hook_copy/9.0.4.tsv"),
    ),
    (
        "tcl9.1",
        include_str!("../../tests/data/native_import_hook_copy/9.1.0.tsv"),
    ),
];

fn check_imported_compiler_snapshot(vm: &Vm, engine: &str, fields: &[&str]) {
    let event = fields[0];
    let hooks = [
        raw_hook(vm, "N::e"),
        raw_hook(vm, "old"),
        raw_hook(vm, "fresh"),
    ];
    for (column, actual) in hooks.into_iter().enumerate() {
        assert_eq!(
            actual,
            fields[column + 3].parse::<i32>().unwrap(),
            "{engine} {event}: raw hook {column}"
        );
    }
    for name in ["old", "fresh"] {
        if raw_hook(vm, name) < 0 {
            continue;
        }
        let binding = vm
            .native_compiler_binding_at_lookup(u64::from(ROOT_NS.0), &NameBytes::from(name))
            .expect("actual imported compiler lookup row");
        assert_eq!(
            binding.token,
            vm.visible_command_generation(name).unwrap(),
            "{engine} {event}: original imported token"
        );
        assert_eq!(
            binding.slot.simple.as_bytes(),
            name.as_bytes(),
            "{engine} {event}: original imported slot"
        );
        assert_eq!(
            binding.compiler_hook,
            vm.native_hook_at_sidecar(&CommandSidecarKey::visible(name)),
            "{engine} {event}: copied compiler function"
        );
        assert!(
            matches!(
                binding.implementation,
                tcl_runtime_api::native_compilation::NativeCommandImplementation::Imported { .. }
            ),
            "{engine} {event}: separate callable origin"
        );
    }
    for (column, imported) in [hooks[1], hooks[2]].into_iter().enumerate() {
        let same = if hooks[0] < 0 || imported < 0 {
            -1
        } else {
            i32::from(hooks[0] == imported)
        };
        assert_eq!(
            same,
            fields[column + 6].parse::<i32>().unwrap(),
            "{engine} {event}: compiler function copy {column}"
        );
    }
}

type ImportedSourceEnsemble = Rc<tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>>;

fn imported_source_ensemble(
    vm: &mut Vm,
    engine: &str,
    namespace: NsId,
) -> Option<ImportedSourceEnsemble> {
    if engine == "tcl8.4" {
        assert_eq!(invoke(vm, "proc", &["N::e", "", "return SOURCE"]), Code::Ok);
        None
    } else {
        let token = Rc::new(tcl_cmd_core::ensemble::EnsembleToken::new(
            EnsembleDef {
                originals: crate::command::native_ensemble_objects::NativeEnsembleObjects::default(
                ),
                native: None,
                namespace,
                map: vec![(NameBytes::from("x"), vec![Some(NameBytes::from("::list"))])],
                subcommands: None,
                prefixes: true,
                parameters: Vec::new(),
                unknown: None,
            },
            NameBytes::from("::N::e"),
        ));
        vm.register_namespace_ensemble("N::e", &token);
        Some(token)
    }
}

#[test]
fn imported_compiler_attachments_match_all_57_original_native_snapshots() {
    // Native proof: naming.import.compiler-hook-copy
    // docs/design/analysis/name-resolution-proofs/import-compiler-hook-copy.md
    // Native proof: naming.import.compiler-hook-mutation-epoch
    // docs/design/analysis/name-resolution-proofs/import-compiler-hook-mutation-epoch.md

    let engines = IMPORTED_COMPILER_FIXTURES;
    let mut completed = 0;
    for (engine, reports) in engines {
        let mut vm = native_vm(engine);
        vm.declare_namespace_exports("N", &["e"]);
        let namespace = vm.namespace_id_from_written(ROOT_NS, "N").unwrap();
        let ensemble = imported_source_ensemble(&mut vm, engine, namespace);
        let mut baseline = None;
        let native_baseline = native_epoch_baseline(reports);
        for row in reports.lines().skip(1) {
            let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
            let event = fields[0];
            let code = match event {
                "import_absent" | "import_present" => {
                    vm.import_commands(b"::N::e", false).unwrap();
                    invoke(
                        &mut vm,
                        "rename",
                        &[
                            "e",
                            if event == "import_absent" {
                                "old"
                            } else {
                                "fresh"
                            },
                        ],
                    )
                }
                "attach_source" | "reattach_source" | "detach_source" => {
                    vm.set_native_ensemble_compiler_attachment(
                        ensemble.as_ref().unwrap(),
                        event != "detach_source",
                    )
                    .unwrap();
                    Code::Ok
                }
                "configure_unhooked" | "configure_hooked" => invoke(
                    &mut vm,
                    "namespace",
                    &["ensemble", "configure", "::N::e", "-prefixes", "0"],
                ),
                "rename_old" => {
                    assert_eq!(invoke(&mut vm, "rename", &["old", "moved"]), Code::Ok);
                    invoke(&mut vm, "rename", &["moved", "old"])
                }
                "hide_old" | "hide_fresh" => {
                    let (name, hidden) = if event == "hide_old" {
                        ("old", "hiddenOld")
                    } else {
                        ("fresh", "hiddenFresh")
                    };
                    vm.hide_command(name, hidden).unwrap();
                    Code::Ok
                }
                "expose_old" | "expose_fresh" => {
                    let (hidden, name) = if event == "expose_old" {
                        ("hiddenOld", "old")
                    } else {
                        ("hiddenFresh", "fresh")
                    };
                    vm.expose_own_command(hidden, name).unwrap();
                    Code::Ok
                }
                "delete_source" => invoke(&mut vm, "rename", &["::N::e", ""]),
                _ => panic!("unhandled native event {event}"),
            };
            assert_eq!(
                code,
                Code::from_int(fields[1].parse().unwrap()),
                "{engine} {event}: completion"
            );
            let epoch = vm.native_cache_stamp(ROOT_NS).interpreter_epoch.0;
            let baseline = *baseline.get_or_insert(epoch);
            assert_eq!(
                epoch - baseline,
                fields[2].parse::<u64>().unwrap() - native_baseline,
                "{engine} {event}: compiler epoch"
            );
            check_imported_compiler_snapshot(&vm, engine, &fields);
            completed += 1;
        }
    }
    assert_eq!(completed, 57);
}

#[test]
fn imported_builtin_keeps_its_raw_compiler_after_origin_handler_replacement() {
    fn replacement(_vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        crate::interp::ok(Value::string("REPLACED"))
    }

    let mut vm = native_vm("tcl8.6");
    vm.declare_namespace_exports("N", &["set"]);
    assert_eq!(invoke(&mut vm, "rename", &["set", "N::set"]), Code::Ok);
    vm.import_commands(b"::N::set", false).unwrap();
    let original = vm
        .native_compiler_binding_at_lookup(u64::from(ROOT_NS.0), &NameBytes::from("set"))
        .unwrap();
    assert_eq!(original.compiler_hook, Hook::Present);
    assert_eq!(original.compiler.as_ref().unwrap().registry_identity, "set");
    vm.register_command("N::set", Command::Builtin(replacement));
    let current = vm
        .native_compiler_binding_at_lookup(u64::from(ROOT_NS.0), &NameBytes::from("set"))
        .unwrap();
    assert_eq!(raw_hook(&vm, "N::set"), 0);
    assert_eq!(current.token, original.token);
    assert_eq!(current.compiler_hook, Hook::Present);
    assert_eq!(current.compiler, original.compiler);
    assert!(matches!(
        current.implementation,
        tcl_runtime_api::native_compilation::NativeCommandImplementation::Imported { .. }
    ));
}

const COMPILER_EPOCH_FIXTURES: [(&str, &str); 5] = [
    (
        "tcl8.4",
        include_str!("../../../../runtime/rust/tests/data/native_compiler_epochs/8.4.20.tsv"),
    ),
    (
        "tcl8.5",
        include_str!("../../../../runtime/rust/tests/data/native_compiler_epochs/8.5.19.tsv"),
    ),
    (
        "tcl8.6",
        include_str!("../../../../runtime/rust/tests/data/native_compiler_epochs/8.6.18.tsv"),
    ),
    (
        "tcl9.0",
        include_str!("../../../../runtime/rust/tests/data/native_compiler_epochs/9.0.4.tsv"),
    ),
    (
        "tcl9.1",
        include_str!("../../../../runtime/rust/tests/data/native_compiler_epochs/9.1.0.tsv"),
    ),
];

fn apply_compiler_epoch_event(vm: &mut Vm, namespace: &mut Option<NsId>, event: &str) -> Code {
    match event {
        "initial" => Code::Ok,
        "namespace_create" | "namespace_recreate" => {
            *namespace = Some(vm.activate_namespace_operand(b"N").unwrap());
            Code::Ok
        }
        "plain_create" => invoke(vm, "proc", &["p", "", "return P"]),
        "plain_rename" => invoke(vm, "rename", &["p", "q"]),
        "plain_delete" => invoke(vm, "rename", &["q", ""]),
        "compiled_shadow" => invoke(vm, "proc", &["N::set", "", "return LOCAL"]),
        "shadow_delete" => invoke(vm, "rename", &["N::set", ""]),
        "compiled_rename" => invoke(vm, "rename", &["set", "stamp_saved_set"]),
        "replacement_plain" => invoke(vm, "proc", &["set", "args", "return REPLACEMENT"]),
        "replacement_delete" => invoke(vm, "rename", &["set", ""]),
        "compiled_restore" => invoke(vm, "rename", &["stamp_saved_set", "set"]),
        "compiled_hide" => {
            vm.hide_command("set", "set").unwrap();
            Code::Ok
        }
        "compiled_expose" => {
            vm.expose_own_command("set", "set").unwrap();
            Code::Ok
        }
        "namespace_path" | "same_path" | "clear_path" => {
            let path = if event == "clear_path" {
                Vec::new()
            } else {
                vec![vm.activate_namespace_operand(b"M").unwrap()]
            };
            vm.push_ns_eval_token_frame(namespace.unwrap(), Vec::new());
            vm.ns_path_set(path);
            vm.pop_call_frame();
            vm.pop_ns();
            Code::Ok
        }
        "namespace_delete" => {
            assert!(vm.delete_namespace_operand(b"N").unwrap());
            *namespace = None;
            Code::Ok
        }
        _ => panic!("unhandled native event {event}"),
    }
}

#[test]
fn compiler_and_resolver_epochs_match_all_87_original_native_mutation_rows() {
    let engines = COMPILER_EPOCH_FIXTURES;
    let mut completed = 0;
    for (engine, reports) in engines {
        let mut vm = native_vm(engine);
        let baseline = vm.native_cache_stamp(ROOT_NS).interpreter_epoch.0;
        let native_baseline = native_epoch_baseline(reports);
        let mut namespace = None;
        for row in reports.lines().skip(1) {
            let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
            let event = fields[0];
            let code = apply_compiler_epoch_event(&mut vm, &mut namespace, event);
            assert_eq!(
                code,
                Code::from_int(fields[1].parse().unwrap()),
                "{engine} {event}: completion"
            );
            let global = vm.native_cache_stamp(ROOT_NS);
            assert_eq!(
                global.interpreter_epoch.0 - baseline,
                fields[2].parse::<u64>().unwrap() - native_baseline,
                "{engine} {event}: compiler epoch"
            );
            assert_eq!(
                global.resolver_epoch,
                fields[3].parse::<u64>().unwrap(),
                "{engine} {event}: global resolver"
            );
            if let Some(namespace) = namespace {
                let stamp = vm.native_cache_stamp(namespace);
                assert_eq!(
                    stamp.resolver_epoch,
                    fields[4].parse::<u64>().unwrap(),
                    "{engine} {event}: namespace resolver"
                );
                assert_eq!(
                    vm.name_world
                        .borrow()
                        .command_reference_epochs
                        .get(&namespace)
                        .copied()
                        .unwrap_or(0),
                    fields[5].parse::<u64>().unwrap(),
                    "{engine} {event}: namespace command references"
                );
            } else {
                assert_eq!(fields[4], "-1", "{engine} {event}: absent namespace");
                assert_eq!(
                    fields[5], "-1",
                    "{engine} {event}: absent namespace command references"
                );
            }
            assert_eq!(
                raw_hook(&vm, "set"),
                fields[6].parse::<i32>().unwrap(),
                "{engine} {event}: raw set compiler"
            );
            completed += 1;
        }
    }
    assert_eq!(completed, 87);
}

#[test]
fn empty_literal_world_entry_is_original_capture_and_registration_scoped() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let vm = native_vm(engine);
        let first = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
        let receipt = first.empty_literal_world.as_ref().expect(engine);
        assert!(receipt.is_current_for(first.interpreter, first.epoch));
        assert!(!receipt.is_current_for(first.interpreter, first.epoch + 1));
        let second = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
        assert!(!receipt.is_current());
        let current = second.empty_literal_world.as_ref().unwrap();
        assert!(current.is_current_for(second.interpreter, second.epoch));
        assert!(!first.same_compilation_world(&second));

        let mut table = tcl_bytecode::LiteralTable::new();
        let index = table.intern_bytes(b"original ordinary registration");
        let pool = crate::literal_pool::NativeLiteralPool::create(
            &vm.native_literal_world,
            &table,
            vm.actual_native_string_protocol(),
            ROOT_NS,
            &tcl_runtime_api::ByteNamespacePath::root(),
        )
        .unwrap();
        assert!(!current.is_current());
        let references = pool
            .with_original(index, Value::native_object_reference_count)
            .unwrap();
        let populated = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
        assert!(populated.empty_literal_world.is_none());
        assert_eq!(
            pool.with_original(index, Value::native_object_reference_count),
            Some(references)
        );
        drop(pool);
        let fresh = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
        let final_receipt = fresh.empty_literal_world.unwrap();
        assert!(final_receipt.is_current());
        drop(vm);
        assert!(!final_receipt.is_current());
    }
}

#[test]
fn compiler_pass_capture_uses_actual_parent_limits_and_state_lifetime() {
    let mut vm = native_vm("tcl8.6");
    let entry = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
    let environment = entry.compiler_pass_environment.as_ref().unwrap();
    assert!(environment.is_current_for(entry.interpreter));
    assert!(environment.is_root());
    assert!(!environment.has_enabled_limits());
    vm.set_command_limit_value(Some(10_000));
    assert!(
        vm.native_compilation_entry_for_namespace_token(ROOT_NS, false)
            .compiler_pass_environment
            .unwrap()
            .has_enabled_limits()
    );
    vm.set_command_limit_value(None);
    vm.set_time_limit_deadline(Some(i128::from(i64::MAX)));
    assert!(
        vm.native_compilation_entry_for_namespace_token(ROOT_NS, false)
            .compiler_pass_environment
            .unwrap()
            .has_enabled_limits()
    );
    vm.set_time_limit_deadline(None);
    let name = vm.create_child(Some("child".into()), false);
    let id = vm.children[name.as_bytes()];
    let child_entry = vm.in_interp(id, |vm| {
        vm.native_compilation_entry_for_namespace_token(ROOT_NS, false)
    });
    assert!(!child_entry.compiler_pass_environment.unwrap().is_root());
    drop(vm);
    assert!(!environment.is_current_for(entry.interpreter));
    let jim = native_vm("jimtcl");
    assert!(
        jim.native_compilation_entry_for_namespace_token(ROOT_NS, false)
            .compiler_pass_environment
            .is_none()
    );
}

#[test]
fn discarded_compiler_array_executes_cache_actions_then_releases_registrations() {
    use tcl_syntax::scalar_getter::NativeScalarCache;
    let mut vm = native_vm("tcl8.6");
    let mut resident = tcl_bytecode::FunctionAsm::default();
    resident.literals.intern("7");
    let owner = vm
        .create_native_literal_pool(&resident, &NamespacePath::root())
        .unwrap();
    let original = owner.value(0).unwrap();
    assert_eq!(original.native_scalar_cache(), None);
    let baseline = original.native_object_reference_count();
    let mut first = tcl_bytecode::LiteralTable::default();
    first.intern_expression_number(
        b"7",
        tcl_dialect::TclVersion::V8_6,
        tcl_bytecode::NativeExpressionNumberLiteral::Integer(7),
    );
    first.intern("discarded-only");
    let mut final_pass = tcl_bytecode::FunctionAsm::default();
    final_pass.literals.intern("7");
    final_pass.literals.retain_discarded_native_pass(first);
    let final_pool = vm
        .create_native_literal_pool(&final_pass, &NamespacePath::root())
        .unwrap();
    assert_eq!(
        original.native_scalar_cache(),
        Some(NativeScalarCache::Number(tcl_syntax::number::Number::Int(
            7
        )))
    );
    assert!(
        final_pool
            .with_original(0, |value| value.is_same_object(&original))
            .unwrap()
    );
    assert_eq!(original.native_object_reference_count(), baseline + 1);
    drop(final_pool);
    assert_eq!(original.native_object_reference_count(), baseline);
    drop(original);
    drop(owner);
    let interpreter = vm.native_interpreter_identity();
    assert!(
        vm.native_literal_world
            .borrow()
            .capture_empty_world(interpreter, vm.trace_deopt_epoch())
            .is_some()
    );
}

#[test]
fn planned_replay_requires_same_live_environment_owner() {
    let mut vm = native_vm("tcl9.1");
    let foreign = native_vm("tcl9.1");
    let environment = foreign
        .capture_native_compiler_pass_environment(None)
        .unwrap();
    let mut asm = tcl_bytecode::FunctionAsm::default();
    asm.literals.intern("final-array-only");
    asm.literals
        .retain_discarded_native_pass(tcl_bytecode::LiteralTable::default());
    asm.literals.retain_compiler_replay_environment(environment);
    assert!(
        vm.create_native_literal_pool(&asm, &NamespacePath::root())
            .is_err()
    );
    drop(foreign);
    assert!(
        vm.create_native_literal_pool(&asm, &NamespacePath::root())
            .is_err()
    );
    vm.set_command_limit_value(Some(100));
    asm.literals.retain_compiler_replay_environment(
        vm.capture_native_compiler_pass_environment(None).unwrap(),
    );
    assert!(
        vm.create_native_literal_pool(&asm, &NamespacePath::root())
            .is_err()
    );
}
