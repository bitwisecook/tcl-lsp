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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Genuine software command-table/callback controls; no Native chronology.
use super::*;
use crate::{Code, Completion, NativeCommand, Value};
use std::cell::Cell;
use tcl_runtime_api::NsId;

fn native(environment: &str) -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect(environment))
}
struct Log {
    events: Rc<RefCell<Vec<&'static str>>>,
    tag: &'static str,
}
impl NativeCommand for Log {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.events.borrow_mut().push(self.tag);
        crate::interp::ok(Value::empty())
    }
}
struct Retire {
    events: Rc<RefCell<Vec<&'static str>>>,
    calls: Rc<Cell<usize>>,
    mode: u8,
    child: Option<InterpId>,
}
impl NativeCommand for Retire {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        crate::interp::ok(Value::int(1))
    }
    fn retire(&self, vm: &mut Vm) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        assert!(vm.name_world.try_borrow_mut().is_ok());
        self.events.borrow_mut().push("implementation");
        match self.mode {
            0 => {}
            1 => {
                let _ = vm.refuse_host_command("FIRST_COMMAND_RETIREMENT".into());
            }
            2 => {
                let actual = vm.host_rc();
                vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
                vm.set_host(actual);
            }
            3 => vm.switch_to(self.child.unwrap()),
            4 => vm.name_world.borrow_mut().retire(),
            _ => unreachable!(),
        }
        crate::interp::ok(Value::empty())
    }
}
type InstalledCommandCleanup = (String, Rc<RefCell<Vec<&'static str>>>, Rc<Cell<usize>>);

fn installed(vm: &mut Vm, mode: u8, child: Option<InterpId>) -> InstalledCommandCleanup {
    assert_eq!(
        vm.eval_source("namespace eval src {namespace export *}; namespace eval dst {}")
            .unwrap()
            .code,
        Code::Ok
    );
    let events = Rc::new(RefCell::new(Vec::new()));
    let calls = Rc::new(Cell::new(0));
    let key = vm
        .register_written_command(
            "::src::victim",
            Command::Native(Rc::new(Retire {
                events: Rc::clone(&events),
                calls: Rc::clone(&calls),
                mode,
                child,
            })),
        )
        .unwrap();
    for (name, tag) in [("source_trace", "trace"), ("import_trace", "imports")] {
        vm.register_written_command(
            name,
            Command::Native(Rc::new(Log {
                events: Rc::clone(&events),
                tag,
            })),
        )
        .unwrap();
    }
    assert_eq!(
        vm.eval_source("namespace eval dst {namespace import ::src::victim}")
            .unwrap()
            .code,
        Code::Ok
    );
    assert_eq!(
        vm.add_cmd_trace(
            false,
            "::src::victim",
            vec!["delete".into()],
            "source_trace".into()
        )
        .code,
        Code::Ok
    );
    assert_eq!(
        vm.add_cmd_trace(
            false,
            "::dst::victim",
            vec!["delete".into()],
            "import_trace".into()
        )
        .code,
        Code::Ok
    );
    (key, events, calls)
}

#[test]
fn original_true_command_deletion_keeps_release_order_and_visible_source_imports() {
    // Actual software NativeCommand callbacks and actual public namespace imports;
    // original C source orders are descriptive, not executed Native callbacks.
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = native(environment);
        let (key, events, calls) = installed(&mut vm, 0, None);
        vm.remove_registered_command(&key);
        assert_eq!(calls.get(), 1);
        assert_eq!(
            &*events.borrow(),
            if matches!(environment, "tcl8.4" | "tcl8.5") {
                &["trace", "implementation", "imports"]
            } else {
                &["trace", "imports", "implementation"]
            }
        );
        assert!(vm.visible_command_at_key(&key).is_none());
        assert!(vm.imported_commands.is_empty());
        assert!(vm.name_world.borrow().command_deletion_active.is_empty());
        assert!(vm.firing_cmd_traces.is_empty());
        assert!(vm.execution_refusal.is_none());
    }
}
struct Nested {
    key: String,
    calls: Rc<Cell<usize>>,
    recreate: bool,
}
impl NativeCommand for Nested {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        if self.recreate {
            vm.register_written_command(
                "::src::victim",
                Command::Builtin(|_, _| crate::interp::ok(Value::int(99))),
            )
            .unwrap();
        } else {
            vm.remove_registered_command(&self.key);
        }
        crate::interp::ok(Value::empty())
    }
}

#[test]
fn original_recursive_delete_unlinks_once_and_outer_owner_retires_captured_implementation() {
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for recreate in [false, true] {
            let mut vm = native(environment);
            let (key, _, retired) = installed(&mut vm, 0, None);
            let old = vm
                .command_generation_for_cleanup(&CommandSidecarKey::visible(&key))
                .unwrap();
            let calls = Rc::new(Cell::new(0));
            vm.register_written_command(
                "nested_delete",
                Command::Native(Rc::new(Nested {
                    key: key.clone(),
                    calls: Rc::clone(&calls),
                    recreate,
                })),
            )
            .unwrap();
            assert_eq!(
                vm.add_cmd_trace(
                    false,
                    "::src::victim",
                    vec!["delete".into()],
                    "nested_delete".into()
                )
                .code,
                Code::Ok
            );
            vm.remove_registered_command(&key);
            assert_eq!(calls.get(), 1);
            assert_eq!(retired.get(), 1);
            assert!(vm.command_token_at_generation(old).is_none());
            if recreate {
                assert_ne!(
                    vm.command_generation_for_cleanup(&CommandSidecarKey::visible(&key)),
                    Some(old)
                );
                assert!(vm.visible_command_at_key(&key).is_some());
            } else {
                assert!(vm.visible_command_at_key(&key).is_none());
            }
            assert!(vm.imported_commands.is_empty());
            assert!(vm.name_world.borrow().command_deletion_active.is_empty());
        }
    }
}

#[test]
fn original_command_cleanup_keeps_first_cause_and_retires_only_original_child_table() {
    for mode in [1, 2, 3, 4] {
        let mut vm = native("tcl9.1");
        let parent = vm.cur;
        let child_name = vm.create_child(Some("retirement_child".into()), false);
        let child = vm.child_id(&child_name).unwrap();
        let (child_key, child_generation) = vm.in_interp(child, |vm| {
            assert_eq!(
                vm.eval_source("namespace eval src {}").unwrap().code,
                Code::Ok
            );
            let key = vm
                .register_written_command(
                    "::src::victim",
                    Command::Builtin(|_, _| crate::interp::ok(Value::int(77))),
                )
                .unwrap();
            let generation = vm
                .command_generation_for_cleanup(&CommandSidecarKey::visible(&key))
                .unwrap();
            (key, generation)
        });
        let (key, _, calls) = installed(&mut vm, mode, Some(child));
        let original = OriginalCommandCleanup::enter(&mut vm);
        vm.remove_registered_command(&key);
        assert_eq!(calls.get(), 1);
        let error = original.check(&mut vm).unwrap_err();
        assert_eq!(
            error.native_execution_refusal(),
            vm.execution_refusal.as_ref()
        );
        let actual = vm.st_of(parent).unwrap();
        assert!(!actual.commands.contains_key(&key));
        assert!(actual.imported_commands.is_empty());
        assert!(actual.firing_cmd_traces.is_empty());
        assert!(
            actual
                .name_world
                .borrow()
                .command_deletion_active
                .is_empty()
        );
        let chosen = vm.cur;
        assert_eq!(chosen, if mode == 3 { child } else { parent });
        vm.in_interp(child, |vm| {
            assert_eq!(
                vm.command_generation_for_cleanup(&CommandSidecarKey::visible(&child_key)),
                Some(child_generation)
            );
            assert!(vm.commands.contains_key(&child_key));
        });
        assert_eq!(vm.cur, chosen);
        assert_eq!(
            original
                .check(&mut vm)
                .unwrap_err()
                .native_execution_refusal(),
            error.native_execution_refusal()
        );
        vm.switch_to(parent);
    }
}

struct MoveSource {
    calls: Rc<Cell<usize>>,
}
impl NativeCommand for MoveSource {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        vm.eval_source("rename ::src::victim ::src::moved").unwrap()
    }
}
#[test]
fn original_command_retirement_follows_actual_rename_without_repeating_delete_trace() {
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = native(environment);
        let (key, events, retired) = installed(&mut vm, 0, None);
        let generation = vm
            .command_generation_for_cleanup(&CommandSidecarKey::visible(&key))
            .unwrap();
        let calls = Rc::new(Cell::new(0));
        vm.register_written_command(
            "move_source",
            Command::Native(Rc::new(MoveSource {
                calls: Rc::clone(&calls),
            })),
        )
        .unwrap();
        assert_eq!(
            vm.add_cmd_trace(
                false,
                "::src::victim",
                vec!["delete".into()],
                "move_source".into()
            )
            .code,
            Code::Ok
        );
        vm.remove_registered_command(&key);
        assert_eq!(calls.get(), 1);
        assert_eq!(retired.get(), 1);
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|tag| **tag == "trace")
                .count(),
            1
        );
        assert!(vm.command_token_at_generation(generation).is_none());
        assert!(vm.imported_commands.is_empty());
        assert!(vm.firing_cmd_traces.is_empty());
        assert!(vm.name_world.borrow().command_deletion_active.is_empty());
        assert!(vm.execution_refusal.is_none());
    }
}
struct EnterChild {
    child: InterpId,
    calls: Rc<Cell<usize>>,
}

struct NamespaceMove {
    destination: NsId,
    after_move: Rc<Cell<u64>>,
}
impl NativeCommand for NamespaceMove {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        let result = vm.eval_source("rename ::src::victim ::dst::moved").unwrap();
        assert_eq!(result.code, Code::Ok);
        self.after_move.set(
            vm.name_world
                .borrow()
                .command_reference_epochs
                .get(&self.destination)
                .copied()
                .unwrap_or(0),
        );
        result
    }
}
struct NamespaceRetire {
    destination: NsId,
    after_move: Rc<Cell<u64>>,
    calls: Rc<Cell<usize>>,
}
impl NativeCommand for NamespaceRetire {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        crate::interp::ok(Value::empty())
    }
    fn retire(&self, vm: &mut Vm) -> Completion<Value> {
        assert_eq!(
            vm.name_world
                .borrow()
                .command_reference_epochs
                .get(&self.destination)
                .copied()
                .unwrap_or(0),
            self.after_move.get() + 1
        );
        self.calls.set(self.calls.get() + 1);
        crate::interp::ok(Value::empty())
    }
}
#[test]
fn original_delete_namespace_effect_follows_same_generation_cross_namespace_rename() {
    // These are actual software namespace/path/rename producers. The observed
    // cache effect grants no Native namespace temporary or callback chronology.
    for environment in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = native(environment);
        assert_eq!(
            vm.eval_source("namespace eval src {}; namespace eval dst {namespace path ::src}")
                .unwrap()
                .code,
            Code::Ok
        );
        let destination = vm.definition_namespace_token("dst");
        let after_move = Rc::new(Cell::new(0));
        let calls = Rc::new(Cell::new(0));
        let key = vm
            .register_written_command(
                "::src::victim",
                Command::Native(Rc::new(NamespaceRetire {
                    destination,
                    after_move: Rc::clone(&after_move),
                    calls: Rc::clone(&calls),
                })),
            )
            .unwrap();
        let generation = vm
            .command_generation_for_cleanup(&CommandSidecarKey::visible(&key))
            .unwrap();
        vm.register_written_command(
            "move_namespace",
            Command::Native(Rc::new(NamespaceMove {
                destination,
                after_move: Rc::clone(&after_move),
            })),
        )
        .unwrap();
        assert_eq!(
            vm.add_cmd_trace(
                false,
                "::src::victim",
                vec!["delete".into()],
                "move_namespace".into()
            )
            .code,
            Code::Ok
        );
        vm.remove_registered_command(&key);
        assert_eq!(calls.get(), 1);
        assert!(vm.command_token_at_generation(generation).is_none());
        assert!(vm.execution_refusal.is_none());
        assert!(vm.name_world.borrow().command_deletion_active.is_empty());
    }
}
impl NativeCommand for EnterChild {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        vm.switch_to(self.child);
        crate::interp::ok(Value::empty())
    }
}
#[test]
fn original_delete_trace_epilogue_preserves_child_stack_and_original_trace_gate() {
    let mut vm = native("tcl9.1");
    let parent = vm.cur;
    let name = vm.create_child(Some("trace_child".into()), false);
    let child = vm.child_id(&name).unwrap();
    vm.in_interp(child, |vm| {
        vm.trace_in_progress.set(true);
        vm.firing_cmd_traces
            .push(CommandSidecarKey::visible("CHILD_STACK"));
    });
    let (key, _, retired) = installed(&mut vm, 0, None);
    let calls = Rc::new(Cell::new(0));
    vm.register_written_command(
        "enter_child",
        Command::Native(Rc::new(EnterChild {
            child,
            calls: Rc::clone(&calls),
        })),
    )
    .unwrap();
    assert_eq!(
        vm.add_cmd_trace(
            false,
            "::src::victim",
            vec!["delete".into()],
            "enter_child".into()
        )
        .code,
        Code::Ok
    );
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    vm.remove_registered_command(&key);
    assert_eq!(calls.get(), 1);
    assert_eq!(retired.get(), 1);
    assert_eq!(vm.cur, child);
    assert_eq!(
        vm.firing_cmd_traces,
        vec![CommandSidecarKey::visible("CHILD_STACK")]
    );
    assert!(vm.trace_in_progress.get());
    let original = vm.st_of(parent).unwrap();
    assert!(original.firing_cmd_traces.is_empty());
    assert!(!original.trace_in_progress.get());
    assert!(!original.commands.contains_key(&key));
    assert!(original.imported_commands.is_empty());
    assert_eq!(
        scope.check(&mut vm).unwrap_err().native_execution_refusal(),
        vm.execution_refusal.as_ref()
    );
    vm.switch_to(parent);
}

struct EpochRetire {
    generation: Rc<Cell<u64>>,
    events: Rc<RefCell<Vec<u64>>>,
}
impl NativeCommand for EpochRetire {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        crate::interp::ok(Value::int(1))
    }
    fn retire(&self, vm: &mut Vm) -> Completion<Value> {
        let epoch = vm
            .name_world
            .borrow()
            .command_name_epochs
            .get(&self.generation.get())
            .copied()
            .unwrap_or(0);
        self.events.borrow_mut().push(epoch);
        crate::interp::ok(Value::empty())
    }
}
struct EpochTrace {
    key: String,
    generation: Rc<Cell<u64>>,
    events: Rc<RefCell<Vec<u64>>>,
    recursive: bool,
}
impl NativeCommand for EpochTrace {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        let epoch = vm
            .name_world
            .borrow()
            .command_name_epochs
            .get(&self.generation.get())
            .copied()
            .unwrap_or(0);
        self.events.borrow_mut().push(epoch);
        if self.recursive {
            vm.remove_registered_command(&self.key);
            let epoch = vm
                .name_world
                .borrow()
                .command_name_epochs
                .get(&self.generation.get())
                .copied()
                .unwrap_or(0);
            self.events.borrow_mut().push(epoch);
        }
        crate::interp::ok(Value::empty())
    }
}

#[test]
fn original_command_deletion_cache_epoch_stages_follow_actual_outer_trace_and_recursive_unlink() {
    // Actual software callbacks observe the selected original command epoch;
    // C source predicates do not constitute Native cache/callback observations.
    for (environment, entry, recursive_entry, final_unlink) in [
        ("tcl8.4", 1, 0, 0),
        ("tcl8.5", 1, 1, 0),
        ("tcl8.6", 1, 1, 1),
        ("tcl9.0", 0, 1, 1),
        ("tcl9.1", 0, 1, 1),
    ] {
        for recursive in [false, true] {
            let mut vm = native(environment);
            let generation = Rc::new(Cell::new(0));
            let events = Rc::new(RefCell::new(Vec::new()));
            let key = vm
                .register_written_command(
                    "epoch_victim",
                    Command::Native(Rc::new(EpochRetire {
                        generation: Rc::clone(&generation),
                        events: Rc::clone(&events),
                    })),
                )
                .unwrap();
            let actual = vm
                .command_generation_for_cleanup(&CommandSidecarKey::visible(&key))
                .unwrap();
            generation.set(actual);
            let before = vm
                .name_world
                .borrow()
                .command_name_epochs
                .get(&actual)
                .copied()
                .unwrap_or(0);
            vm.register_written_command(
                "epoch_trace",
                Command::Native(Rc::new(EpochTrace {
                    key: key.clone(),
                    generation: Rc::clone(&generation),
                    events: Rc::clone(&events),
                    recursive,
                })),
            )
            .unwrap();
            assert_eq!(
                vm.add_cmd_trace(
                    false,
                    "epoch_victim",
                    vec!["delete".into()],
                    "epoch_trace".into()
                )
                .code,
                Code::Ok
            );
            vm.remove_registered_command(&key);
            let expected = if recursive {
                vec![
                    before + entry,
                    before + entry + recursive_entry,
                    before + entry + recursive_entry,
                ]
            } else {
                vec![before + entry, before + entry]
            };
            assert_eq!(
                *events.borrow(),
                expected,
                "{environment} recursive={recursive}"
            );
            assert_eq!(
                vm.name_world
                    .borrow()
                    .command_name_epochs
                    .get(&actual)
                    .copied()
                    .unwrap_or(0),
                before
                    + entry
                    + if recursive {
                        recursive_entry
                    } else {
                        final_unlink
                    }
            );
            assert!(vm.visible_command_at_key(&key).is_none());
            assert!(vm.name_world.borrow().command_deletion_active.is_empty());
            assert!(vm.execution_refusal.is_none());
        }
    }
}

struct RecreateCoroutine;
impl NativeCommand for RecreateCoroutine {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        vm.try_eval_source("coroutine paused apply {{} {yield NEW; return NEW_DONE}}")
            .unwrap()
    }
}
#[test]
fn original_coroutine_retirement_leaves_a_genuine_same_key_recreated_state() {
    // Actual coroutine producer/parked state and real trace callback; no Native
    // coroutine cache/refcount or external callback chronology is inferred.
    let mut vm = native("tcl9.1");
    assert_eq!(
        vm.try_eval_source("coroutine paused apply {{} {yield OLD; return OLD_DONE}}")
            .unwrap()
            .code,
        Code::Ok
    );
    let key = vm
        .resolve_command_fqn_raw(vm.current_ns(), "paused")
        .unwrap();
    let old = vm
        .command_generation_for_cleanup(&CommandSidecarKey::visible(&key))
        .unwrap();
    vm.register_written_command(
        "recreate_coroutine",
        Command::Native(Rc::new(RecreateCoroutine)),
    )
    .unwrap();
    assert_eq!(
        vm.add_cmd_trace(
            false,
            "paused",
            vec!["delete".into()],
            "recreate_coroutine".into()
        )
        .code,
        Code::Ok
    );
    vm.remove_registered_command(&key);
    assert_ne!(
        vm.command_generation_for_cleanup(&CommandSidecarKey::visible(&key)),
        Some(old)
    );
    assert!(vm.visible_command_at_key(&key).is_some());
    let resumed = vm.try_invoke_command("paused", &[]).unwrap();
    assert_eq!(resumed.code, Code::Ok);
    assert_eq!(
        resumed.result.resident_string_bytes().unwrap().as_ref(),
        b"NEW_DONE"
    );
    assert!(vm.name_world.borrow().command_deletion_active.is_empty());
    assert!(vm.execution_refusal.is_none());
}

struct RepeatedEpochTrace {
    original: CommandTokenIdentity,
    events: Rc<RefCell<Vec<u64>>>,
}
impl NativeCommand for RepeatedEpochTrace {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        let epoch = |vm: &Vm| {
            vm.name_world
                .borrow()
                .command_name_epochs
                .get(&self.original.generation)
                .copied()
                .unwrap_or(0)
        };
        self.events.borrow_mut().push(epoch(vm));
        // The outer real deletion issued the active generation. Reuse its
        // retained token after unlink; a report lookup could not recover it.
        let cleanup = OriginalCommandCleanup::enter(vm);
        assert!(cleanup.unlink_recursive_c_delete(vm, &self.original));
        assert!(
            vm.command_token_at_generation(self.original.generation)
                .is_none()
        );
        self.events.borrow_mut().push(epoch(vm));
        assert!(cleanup.unlink_recursive_c_delete(vm, &self.original));
        assert!(
            vm.command_token_at_generation(self.original.generation)
                .is_none()
        );
        self.events.borrow_mut().push(epoch(vm));
        cleanup.check(vm).unwrap();
        crate::interp::ok(Value::empty())
    }
}

#[test]
fn original_repeated_recursive_unlinked_command_advances_only_selected_epoch_stages() {
    // Actual software trace and retained original token; pinned source stages
    // do not constitute executed Native cache/callback observations.
    for (environment, entry, each_recursive) in [
        ("tcl8.4", 1, 0),
        ("tcl8.5", 1, 1),
        ("tcl8.6", 1, 1),
        ("tcl9.0", 0, 1),
        ("tcl9.1", 0, 1),
    ] {
        let mut vm = native(environment);
        let generation = Rc::new(Cell::new(0));
        let events = Rc::new(RefCell::new(Vec::new()));
        let key = vm
            .register_written_command(
                "repeated_epoch_victim",
                Command::Native(Rc::new(EpochRetire {
                    generation: Rc::clone(&generation),
                    events: Rc::clone(&events),
                })),
            )
            .unwrap();
        let original = vm
            .command_token_identity(&CommandSidecarKey::visible(&key))
            .unwrap();
        generation.set(original.generation);
        let before = vm
            .name_world
            .borrow()
            .command_name_epochs
            .get(&original.generation)
            .copied()
            .unwrap_or(0);
        vm.register_written_command(
            "repeated_epoch_trace",
            Command::Native(Rc::new(RepeatedEpochTrace {
                original: original.clone(),
                events: Rc::clone(&events),
            })),
        )
        .unwrap();
        assert_eq!(
            vm.add_cmd_trace(
                false,
                "repeated_epoch_victim",
                vec!["delete".into()],
                "repeated_epoch_trace".into()
            )
            .code,
            Code::Ok
        );
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        vm.remove_registered_command(&key);
        let final_epoch = before + entry + 2 * each_recursive;
        assert_eq!(
            *events.borrow(),
            vec![
                before + entry,
                before + entry + each_recursive,
                final_epoch,
                final_epoch,
            ],
            "{environment}"
        );
        assert_eq!(
            vm.name_world
                .borrow()
                .command_name_epochs
                .get(&original.generation)
                .copied(),
            Some(final_epoch)
        );
        assert!(
            vm.command_token_at_generation(original.generation)
                .is_none()
        );
        assert!(vm.visible_command_at_key(&key).is_none());
        assert!(vm.name_world.borrow().command_deletion_active.is_empty());
        assert!(vm.execution_refusal.is_none());
        scope.check(&mut vm).unwrap();
    }
}
