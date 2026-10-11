// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim previous-command owners and frame-owned original cleanup names.

use super::native_jim_lookup::{JimCommandLease, JimCommandNode};
use super::{
    Command, CommandSlot, Completion, NameBytes, ROOT_NS, Rc, RefCell, Value, Vm, err,
    native_name_world,
};
use tcl_syntax::value::ValueError;

/// The frame owns its original header and an already selected cleanup binding.
/// A weak node carries identity only; this adds no Native or node ownership.
pub(crate) struct OriginalJimLocalCommand {
    original: Value,
    interpreter: super::InterpId,
    world: Rc<RefCell<native_name_world::NativeNameWorld>>,
    slot: CommandSlot,
    generation: u64,
    node: std::rc::Weak<JimCommandNode>,
    currency: super::native_operation_currency::NativeOperationCurrency,
}

struct OriginalJimLocalContinuation {
    interpreter: super::InterpId,
    world: Rc<RefCell<native_name_world::NativeNameWorld>>,
    slot: CommandSlot,
    table_epoch: u64,
}

struct LocalScope(Rc<RefCell<native_name_world::NativeNameWorld>>);
impl Drop for LocalScope {
    fn drop(&mut self) {
        let mut world = self.0.borrow_mut();
        world.jim_local_depth = world
            .jim_local_depth
            .checked_sub(1)
            .expect("Jim local scope");
    }
}

struct UpcallScope {
    node: Rc<JimCommandNode>,
    _lease: JimCommandLease,
}
impl Drop for UpcallScope {
    fn drop(&mut self) {
        self.node.upcalls.set(
            self.node
                .upcalls
                .get()
                .checked_sub(1)
                .expect("Jim upcall scope"),
        );
    }
}

pub(crate) fn refresh(vm: &mut Vm) {
    let selected = vm.uses_native_jim_lookup()
        && vm
            .native_scalar_carrier_dialect()
            .native_jim_local_protocol()
            .is_some();
    for (name, handler) in [
        ("local", local as crate::command::BuiltinFn),
        ("upcall", upcall as crate::command::BuiltinFn),
    ] {
        let stock = vm.stock_native_identity(name).as_deref() == Some(name);
        if selected {
            if stock || vm.lookup_command(name).is_none() {
                vm.register_stock_builtin(name, handler);
            }
        } else if stock {
            vm.remove_registered_command(name);
        }
    }
}

impl Vm {
    pub(super) fn select_jim_previous_node(
        &self,
        mut node: Rc<JimCommandNode>,
    ) -> Result<Rc<JimCommandNode>, ValueError> {
        let recipe = self
            .native_scalar_carrier_dialect()
            .native_jim_local_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim previous-command selection",
            ))?;
        loop {
            let procedure = matches!(&*node.command.borrow(), Command::Proc(_));
            if !recipe.follows_previous(procedure, node.upcalls.get()) {
                return Ok(node);
            }
            let previous =
                node.previous
                    .borrow()
                    .clone()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "actual Jim previous-command edge",
                    ))?;
            node = previous;
        }
    }

    pub(super) fn selected_original_jim_node(
        &self,
        original: &Value,
    ) -> Option<Rc<JimCommandNode>> {
        let token = original.with_jim_command_cache(|cache| cache.token)?;
        let node = self
            .name_world
            .borrow()
            .jim_command_receipts
            .get(&token)?
            .upgrade()?;
        self.select_jim_previous_node(node).ok()
    }

    pub(crate) fn jim_original_builtin_identity(&self, original: &Value) -> Option<String> {
        let node = self.selected_original_jim_node(original)?;

        node.builtin_identity.borrow().clone()
    }

    pub(super) fn register_jim_local_command(
        &mut self,
        slot: CommandSlot,
        command: Command,
        table_key: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
    ) -> String {
        let previous = self.command_storage_key_at_slot(&slot).and_then(|key| {
            let command = self.visible_command_at_key(&key)?;
            let token = self.visible_command_generation(&key)?;
            if !self
                .name_world
                .borrow()
                .jim_command_receipts
                .contains_key(&token)
            {
                let name = self.command_display_key_bytes(&key);
                self.note_jim_command_publication(token, &command, &key, name.as_bytes());
            }
            let node = self
                .name_world
                .borrow()
                .jim_command_receipts
                .get(&token)?
                .upgrade()?;
            *node.builtin_identity.borrow_mut() = self.builtin_identity_for_key(&key);
            node.previous_owners.set(node.previous_owners.get() + 1);
            self.take_command_unchecked_key(&key)?;
            Some(node)
        });
        let depth = self.name_world.borrow().jim_local_depth;
        self.name_world.borrow_mut().jim_local_depth = 0;
        let key = self.register_command_in_slot_with_jim_key(slot, command, table_key);
        self.name_world.borrow_mut().jim_local_depth = depth;
        if let Some(previous) = previous {
            let token = self
                .visible_command_generation(&key)
                .expect("published local node");
            let mut world = self.name_world.borrow_mut();
            let node = world
                .jim_command_nodes
                .get(&token)
                .expect("actual published Jim node");
            *node.previous.borrow_mut() = Some(previous);
            let recipe = tcl_syntax::native_jim_local::NativeJimLocalProtocol::jim084();
            if recipe.changes_epoch(true) {
                world.advance_jim_procedure_epoch();
            }
        }
        key
    }

    fn capture_original_jim_local_command(
        &mut self,
        original: Value,
        key: &str,
        scope: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<OriginalJimLocalCommand, tcl_cmd_core::CmdError> {
        scope.check(self)?;
        let selected = self
            .command_slot(key)
            .zip(self.visible_command_generation(key));
        let node = selected.as_ref().and_then(|(_, generation)| {
            self.name_world
                .borrow()
                .jim_command_receipts
                .get(generation)
                .cloned()
        });
        let valid = selected
            .as_ref()
            .zip(node.as_ref())
            .is_some_and(|((_, generation), weak)| {
                original.with_jim_command_cache(|cache| cache.token) == Some(*generation)
                    && weak
                        .upgrade()
                        .is_some_and(|node| node.published.get() && node.token == *generation)
            });
        scope.require(self, valid, "original Jim local producer binding")?;
        let (slot, generation) = selected.expect("checked selected local binding");
        Ok(OriginalJimLocalCommand {
            original,
            interpreter: self.cur,
            world: Rc::clone(&self.name_world),
            slot,
            generation,
            node: node.expect("checked selected local node"),
            currency: scope.currency().clone(),
        })
    }

    pub(super) fn clean_jim_local_commands(
        &mut self,
        records: Vec<OriginalJimLocalCommand>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let names_scope =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self);
        let mut first = names_scope.as_ref().err().cloned();
        if let Some(error) = &first {
            let _ = crate::command::completion_from_cmd_error(self, error.clone());
        }
        for record in records.into_iter().rev() {
            if first.is_none()
                && let Ok(original_names) = &names_scope
            {
                if let Err(error) = original_names.read(self, &record.original) {
                    let _ = crate::command::completion_from_cmd_error(self, error.clone());
                    first = Some(error);
                }
            }
            // This mandatory stage uses only the producer's existing receipt.
            // Refusal never grants another String getter or a fresh name lookup.
            if let Err(error) = self.clean_captured_jim_local_command(record) {
                let _ = crate::command::completion_from_cmd_error(self, error.clone());
                first.get_or_insert(error);
            }
        }
        if let Ok(original_names) = names_scope
            && let Err(error) = original_names.finish(self)
        {
            first.get_or_insert(error);
        }
        first.map_or(Ok(()), Err)
    }

    fn refuse_original_jim_local_cleanup(
        &mut self,
        purpose: &'static str,
    ) -> tcl_cmd_core::CmdError {
        let cause = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(purpose),
        );
        self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(cause));
        tcl_cmd_core::CmdError::from_execution_refusal(
            self.execution_refusal
                .clone()
                .expect("retained original cleanup cause"),
        )
    }

    fn clean_captured_jim_local_command(
        &mut self,
        record: OriginalJimLocalCommand,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        if self.st_of(record.interpreter).is_none() {
            let error =
                self.refuse_original_jim_local_cleanup("original Jim local cleanup interpreter");
            drop(record);
            return Err(error);
        }
        self.in_interp(record.interpreter, |vm| {
            let operation =
                super::native_operation_currency::NativeOperationScope::enter_saved_cleanup(
                    vm,
                    &record.currency,
                    record.interpreter,
                );
            let reached = (|| {
                let key = vm.command_storage_key_at_slot(&record.slot);
                let node = record.node.upgrade();
                let table_current = operation.is_ok()
                    && Rc::ptr_eq(&record.world, &vm.name_world)
                    && vm.name_world.borrow().is_live()
                    && vm.native_namespace_token_has_owner(record.slot.namespace)
                    && key.as_ref().is_some_and(|key| {
                        vm.visible_command_generation(key) == Some(record.generation)
                    })
                    && node.as_ref().is_some_and(|node| {
                        node.published.get()
                            && vm
                                .name_world
                                .borrow()
                                .jim_command_receipts
                                .get(&record.generation)
                                .and_then(std::rc::Weak::upgrade)
                                .is_some_and(|current| Rc::ptr_eq(node, &current))
                    });
                if !table_current {
                    return Err(vm.refuse_original_jim_local_cleanup(
                        "original Jim local cleanup selected binding",
                    ));
                }
                drop(node);
                if let Some((continuation, previous)) =
                    vm.unlink_original_jim_local_command(record.slot)
                {
                    vm.restore_captured_jim_previous(continuation, previous, &record.currency)?;
                }
                Ok(())
            })();
            drop(record.original);
            let checked = operation
                .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
                .and_then(|scope| scope.check(vm));
            reached.and(checked)
        })
    }

    fn unlink_original_jim_local_command(
        &mut self,
        slot: CommandSlot,
    ) -> Option<(OriginalJimLocalContinuation, Rc<JimCommandNode>)> {
        let key = self.command_storage_key_at_slot(&slot)?;
        let token = self.visible_command_generation(&key)?;
        let top = self
            .name_world
            .borrow()
            .jim_command_receipts
            .get(&token)
            .and_then(std::rc::Weak::upgrade);
        let previous = top
            .as_ref()
            .and_then(|node| node.previous.borrow_mut().take());
        if let Some(previous) = &previous {
            previous.published.set(true);
            previous.previous_owners.set(
                previous
                    .previous_owners
                    .get()
                    .checked_sub(1)
                    .expect("owning previous-command edge"),
            );
        }
        let interpreter = self.cur;
        let world = Rc::clone(&self.name_world);
        let retired = self.take_command_unchecked_key(&key);
        // Capture after the genuine unlink, before retiring owned payloads.
        // This conservative software stamp grants no Native table identity.
        let table_epoch = self.compilation_epochs.cmd_epoch.get();
        let continuation = OriginalJimLocalContinuation {
            interpreter,
            world,
            slot,
            table_epoch,
        };
        drop(retired);
        drop(top);
        previous.map(|previous| (continuation, previous))
    }

    #[cfg(test)]
    fn restore_original_jim_previous(
        &mut self,
        continuation: OriginalJimLocalContinuation,
        previous: Rc<JimCommandNode>,
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.restore_captured_jim_previous(continuation, previous, original_names.currency())
    }

    fn restore_captured_jim_previous(
        &mut self,
        continuation: OriginalJimLocalContinuation,
        previous: Rc<JimCommandNode>,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        if self.st_of(continuation.interpreter).is_none() {
            super::native_jim_lookup::retire_detached_original_jim_node_in(
                &continuation.world,
                previous,
            );
            return Err(
                self.refuse_original_jim_local_cleanup("original Jim local cleanup interpreter")
            );
        }
        self.in_interp(continuation.interpreter, |vm| {
            let cleanup =
                super::native_operation_currency::NativeOperationScope::enter_saved_cleanup(
                    vm,
                    currency,
                    continuation.interpreter,
                );
            let table_current = cleanup.is_ok()
                && Rc::ptr_eq(&continuation.world, &vm.name_world)
                && continuation.world.borrow().is_live()
                && continuation.table_epoch != u64::MAX
                && vm.compilation_epochs.cmd_epoch.get() == continuation.table_epoch
                && vm.native_namespace_token_has_owner(continuation.slot.namespace)
                && vm.command_storage_key_at_slot(&continuation.slot).is_none();
            if !table_current {
                super::native_jim_lookup::retire_detached_original_jim_node_in(
                    &continuation.world,
                    previous,
                );
                return Err(vm.refuse_original_jim_local_cleanup(
                    "original Jim local cleanup table continuation",
                ));
            }
            // Restore this already selected previous role even after a first Host
            // refusal. Currency prevents later semantics, not mandatory cleanup.
            let display = previous.name.borrow().clone();
            let restored_key =
                vm.storage_key_for_command_slot(continuation.slot, display.as_bytes());
            vm.commands
                .insert(restored_key.clone(), previous.command.borrow().clone());
            {
                let mut world = vm.name_world.borrow_mut();
                world
                    .command_identity
                    .generations
                    .insert(restored_key.clone(), previous.token);
                world
                    .jim_command_nodes
                    .insert(previous.token, Rc::clone(&previous));
                world
                    .jim_command_table_keys
                    .insert(restored_key.clone(), previous.table_key.borrow().clone());
            }
            (*previous.storage_key.borrow_mut()).clone_from(&restored_key);
            if let Some(identity) = previous.builtin_identity.borrow().clone() {
                vm.builtin_identities.insert(restored_key.clone(), identity);
            }
            vm.note_command_bound(&restored_key, false);
            vm.invalidate_compiled_command_semantics_for_key(&restored_key);
            cleanup
                .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?
                .check(vm)
        })
    }

    pub(crate) fn jim_local_rename_error(
        &mut self,
        written: &[u8],
        delete: bool,
    ) -> Option<Completion<Value>> {
        if !self.uses_native_jim_lookup() {
            return None;
        }
        let recipe = self
            .native_scalar_carrier_dialect()
            .native_jim_local_protocol()?;
        let key = match self.native_rename_source_key(written) {
            Ok(Some(key)) => key,
            Ok(None) => return None,
            Err(error) => return Some(self.refuse_host_command(error.to_string())),
        };
        let token = self.visible_command_generation(&key)?;
        let previous = self
            .name_world
            .borrow()
            .jim_command_receipts
            .get(&token)
            .and_then(std::rc::Weak::upgrade)
            .is_some_and(|node| node.previous.borrow().is_some());
        (!recipe.accepts_rename(previous, delete)).then(|| err(recipe.local_rename_error(written)))
    }
}

fn local(vm: &mut Vm, arguments: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        let Some((head, tail)) = arguments.split_first() else {
            return crate::command::native_wrong_args(vm, "local cmd ?args ...?");
        };
        if !vm.uses_native_jim_lookup() {
            return vm.refuse_host_command("Jim local command issuer is unavailable".into());
        }
        let world = Rc::clone(&vm.name_world);
        world.borrow_mut().jim_local_depth += 1;
        let scope = LocalScope(world);
        let completion = vm.invoke_command_value_at(
            vm.current_ns_id(),
            head,
            tail,
            &[],
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        );
        drop(scope);
        if let Err(error) = original_names.check(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if !completion.code.is_ok() {
            return completion;
        }
        match vm.native_jim_command_from_original_at(vm.current_ns_id(), &completion.result) {
            Ok(Some((key, _))) => {
                let record = match vm.capture_original_jim_local_command(
                    completion.result.clone(),
                    &key,
                    &original_names,
                ) {
                    Ok(record) => record,
                    Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                };
                vm.frames
                    .last_mut()
                    .expect("actual Jim frame")
                    .jim_local_commands
                    .push(record);
                completion
            }
            Ok(None) => match original_names.read(vm, &completion.result) {
                Ok(bytes) => err(
                    tcl_syntax::native_jim_local::NativeJimLocalProtocol::jim084()
                        .missing_cleanup_command(&bytes),
                ),
                Err(error) => crate::command::completion_from_cmd_error(vm, error),
            },
            Err(error) => vm.refuse_host_command(error.to_string()),
        }
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

fn upcall(vm: &mut Vm, arguments: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        let Some((head, tail)) = arguments.split_first() else {
            return crate::command::native_wrong_args(vm, "upcall cmd ?args ...?");
        };
        let Some(recipe) = vm
            .native_scalar_carrier_dialect()
            .native_jim_local_protocol()
            .filter(|_| vm.uses_native_jim_lookup())
        else {
            return vm.refuse_host_command("Jim upcall command issuer is unavailable".into());
        };
        if let Err(error) = vm.native_jim_command_from_original_at(vm.current_ns_id(), head) {
            return vm.refuse_host_command(error.to_string());
        }
        if let Err(error) = original_names.check(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if let Some(node) = vm.selected_original_jim_node(head) {
            let procedure = matches!(&*node.command.borrow(), Command::Proc(_));
            if recipe.accepts_upcall(procedure, node.previous.borrow().is_some()) {
                let Some(lease) = vm.retain_active_jim_command(node.token) else {
                    return vm
                        .refuse_host_command("Jim upcall selected worker is unavailable".into());
                };
                node.upcalls.set(node.upcalls.get() + 1);
                let scope = UpcallScope {
                    node,
                    _lease: lease,
                };
                let completion = vm.invoke_command_value_at(
                    vm.current_ns_id(),
                    head,
                    tail,
                    &[],
                    tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
                );
                drop(scope);
                return completion;
            }
        }
        match original_names.read(vm, head) {
            Ok(bytes) => err(recipe.missing_previous_command(&bytes)),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        }
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

#[cfg(test)]
mod tests {
    #[test]
    fn local_frames_and_nested_upcalls_match_original_native_lineage() {
        let fixture = include_str!("../../../tcl-syntax/tests/data/native_jim_local/behaviour.tsv");
        let mut checked = 0;
        for row in fixture.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let expected: Vec<u8> = fields[1]
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
            let mut vm = crate::Vm::new();
            assert!(vm.set_native_engine_profile(profile));
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let result = vm.eval_source(fields[2]).unwrap();
            assert!(
                result.code.is_ok(),
                "{}: {:?}",
                fields[2],
                result.result.native_object_snapshot()
            );
            assert_eq!(
                result.result.string_bytes().as_ref(),
                expected.as_slice(),
                "{}",
                fields[2]
            );
            checked += 1;
        }
        assert_eq!(checked, 5);
    }

    struct PreviousOwnerObserver {
        world: std::rc::Weak<std::cell::RefCell<super::native_name_world::NativeNameWorld>>,
        released: std::rc::Rc<std::cell::RefCell<Vec<(u8, bool)>>>,
        label: u8,
    }
    impl crate::NativeCommand for PreviousOwnerObserver {
        fn invoke(&self, _: &mut super::Vm, _: &[super::Value]) -> super::Completion<super::Value> {
            super::super::ok(super::Value::empty())
        }
    }
    impl Drop for PreviousOwnerObserver {
        fn drop(&mut self) {
            let outside_borrow = self
                .world
                .upgrade()
                .is_some_and(|world| world.try_borrow_mut().is_ok());
            self.released
                .borrow_mut()
                .push((self.label, outside_borrow));
        }
    }
    fn selected_local() -> (
        super::Vm,
        super::CommandSlot,
        u64,
        std::rc::Rc<std::cell::RefCell<Vec<(u8, bool)>>>,
    ) {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = super::Vm::new();
        assert!(vm.set_native_engine_profile(profile));
        vm.set_dialect_profile(profile);
        let released = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let command = |label| {
            super::Command::Native(std::rc::Rc::new(PreviousOwnerObserver {
                world: std::rc::Rc::downgrade(&vm.name_world),
                released: std::rc::Rc::clone(&released),
                label,
            }))
        };
        let previous = command(b'A');
        let current = command(b'B');
        let key = vm
            .register_written_command("original_local", previous)
            .unwrap();
        let original_token = vm.visible_command_generation(&key).unwrap();
        let slot = super::CommandSlot {
            namespace: super::ROOT_NS,
            simple: super::NameBytes::from(b"original_local".as_slice()),
        };
        vm.register_jim_local_command(slot.clone(), current, None);
        assert_ne!(vm.visible_command_generation(&key), Some(original_token));
        (vm, slot, original_token, released)
    }

    #[test]
    fn original_jim_local_cleanup_restores_its_captured_previous_after_the_first_host_cause() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let (mut vm, slot, original_token, released) = selected_local();
        let scope =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let (continuation, previous) = vm.unlink_original_jim_local_command(slot.clone()).unwrap();
        let original = std::rc::Rc::downgrade(&previous);
        assert_eq!(released.borrow().as_slice(), [(b'B', true)]);
        let _ = vm.refuse_host_command("FIRST_SELECTED_LOCAL_CAUSE".into());
        let cause = vm.execution_refusal.clone().unwrap();
        let error = vm
            .restore_original_jim_previous(continuation, previous, &scope)
            .unwrap_err();
        assert_eq!(error.native_execution_refusal(), Some(&cause));
        let restored = vm.command_storage_key_at_slot(&slot).unwrap();
        assert_eq!(
            vm.visible_command_generation(&restored),
            Some(original_token)
        );
        let current = vm
            .name_world
            .borrow()
            .jim_command_nodes
            .get(&original_token)
            .cloned()
            .unwrap();
        assert!(std::rc::Rc::ptr_eq(&current, &original.upgrade().unwrap()));
        assert!(current.published.get());
        assert_eq!(current.previous_owners.get(), 0);
        assert_eq!(released.borrow().as_slice(), [(b'B', true)]);
        assert_eq!(
            scope
                .finish(&mut vm)
                .unwrap_err()
                .native_execution_refusal(),
            Some(&cause)
        );
    }

    #[test]
    fn original_jim_local_cleanup_refuses_recreate_delete_aba_and_retires_the_detached_previous() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let (mut vm, slot, original_token, released) = selected_local();
        let scope =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let (continuation, previous) = vm.unlink_original_jim_local_command(slot.clone()).unwrap();
        let original = std::rc::Rc::downgrade(&previous);
        let before = continuation.table_epoch;
        let replacement = vm.register_command_in_slot(
            slot.clone(),
            super::Command::Native(std::rc::Rc::new(PreviousOwnerObserver {
                world: std::rc::Rc::downgrade(&vm.name_world),
                released: std::rc::Rc::clone(&released),
                label: b'C',
            })),
        );
        assert_ne!(
            vm.visible_command_generation(&replacement),
            Some(original_token)
        );
        vm.remove_registered_command(&replacement);
        assert!(vm.command_storage_key_at_slot(&slot).is_none());
        assert_ne!(vm.compilation_epochs.cmd_epoch.get(), before);
        let error = vm
            .restore_original_jim_previous(continuation, previous, &scope)
            .unwrap_err();
        let cause = vm.execution_refusal.clone().unwrap();
        assert_eq!(error.native_execution_refusal(), Some(&cause));
        assert!(vm.command_storage_key_at_slot(&slot).is_none());
        assert!(original.upgrade().is_none());
        assert_eq!(
            released.borrow().as_slice(),
            [(b'B', true), (b'C', true), (b'A', true)]
        );
        assert_eq!(
            scope
                .finish(&mut vm)
                .unwrap_err()
                .native_execution_refusal(),
            Some(&cause)
        );
    }

    #[test]
    fn original_jim_local_cleanup_permits_an_independent_child_table_mutation() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let (mut vm, slot, original_token, released) = selected_local();
        vm.create_child(Some("independent".into()), false);
        let child = vm.child_id("independent").unwrap();
        let scope =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let (continuation, previous) = vm.unlink_original_jim_local_command(slot.clone()).unwrap();
        let epoch = continuation.table_epoch;
        let world = std::rc::Rc::clone(&continuation.world);
        vm.in_interp(child, |vm| {
            assert!(!std::rc::Rc::ptr_eq(&world, &vm.name_world));
            let key = vm
                .register_written_command(
                    "other_table",
                    super::Command::Native(std::rc::Rc::new(PreviousOwnerObserver {
                        world: std::rc::Rc::downgrade(&vm.name_world),
                        released: std::rc::Rc::clone(&released),
                        label: b'C',
                    })),
                )
                .unwrap();
            vm.remove_registered_command(&key);
        });
        assert_eq!(vm.compilation_epochs.cmd_epoch.get(), epoch);
        vm.restore_original_jim_previous(continuation, previous, &scope)
            .unwrap();
        let restored = vm.command_storage_key_at_slot(&slot).unwrap();
        assert_eq!(
            vm.visible_command_generation(&restored),
            Some(original_token)
        );
        assert_eq!(released.borrow().as_slice(), [(b'B', true), (b'C', true)]);
        scope.finish(&mut vm).unwrap();
    }

    #[test]
    fn original_jim_local_cleanup_refuses_an_exhausted_table_stamp() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let (mut vm, slot, _, released) = selected_local();
        let scope =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
        vm.compilation_epochs.cmd_epoch.set(u64::MAX);
        let (continuation, previous) = vm.unlink_original_jim_local_command(slot.clone()).unwrap();
        assert_eq!(continuation.table_epoch, u64::MAX);
        let error = vm
            .restore_original_jim_previous(continuation, previous, &scope)
            .unwrap_err();
        let cause = vm.execution_refusal.clone().unwrap();
        assert_eq!(error.native_execution_refusal(), Some(&cause));
        assert!(vm.command_storage_key_at_slot(&slot).is_none());
        assert_eq!(released.borrow().as_slice(), [(b'B', true), (b'A', true)]);
        assert_eq!(
            scope
                .finish(&mut vm)
                .unwrap_err()
                .native_execution_refusal(),
            Some(&cause)
        );
    }

    fn captured_local_record(
        vm: &mut super::Vm,
        slot: &super::CommandSlot,
    ) -> (super::Value, super::OriginalJimLocalCommand) {
        let scope =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter(vm).unwrap();
        let original = super::Value::new_native_string_bytes(slot.simple.as_bytes());
        let (selected, _) = vm
            .native_jim_command_from_original_at(super::ROOT_NS, &original)
            .unwrap()
            .unwrap();
        let record = vm
            .capture_original_jim_local_command(original.clone(), &selected, &scope)
            .unwrap();
        scope.finish(vm).unwrap();
        (original, record)
    }

    #[test]
    fn original_jim_local_frame_receipt_completes_cleanup_after_pre_entry_host_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let (mut vm, slot, original_token, released) = selected_local();
        let (original, record) = captured_local_record(&mut vm, &slot);
        let captured_node = record.node.clone();
        assert_eq!(original.native_object_reference_count(), 2);
        vm.frames
            .last_mut()
            .unwrap()
            .jim_local_commands
            .push(record);
        let records = std::mem::take(&mut vm.frames.last_mut().unwrap().jim_local_commands);
        let _ = vm.refuse_host_command("FIRST_BEFORE_LOCAL_CLEANUP_ENTRY".into());
        let first = vm.execution_refusal.clone().unwrap();
        let error = vm.clean_jim_local_commands(records).unwrap_err();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
        let restored = vm.command_storage_key_at_slot(&slot).unwrap();
        assert_eq!(
            vm.visible_command_generation(&restored),
            Some(original_token)
        );
        assert!(captured_node.upgrade().is_none());
        assert_eq!(released.borrow().as_slice(), [(b'B', true)]);
        assert_eq!(original.native_object_reference_count(), 1);
        assert_eq!(
            original.resident_string_bytes().unwrap().as_ref(),
            b"original_local"
        );
        assert!(vm.frames.last().unwrap().jim_local_commands.is_empty());
    }

    #[test]
    fn original_jim_local_frame_receipt_keeps_a_recreated_binding_unreached_after_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let (mut vm, slot, _, released) = selected_local();
        let (original, record) = captured_local_record(&mut vm, &slot);
        let previous_key = vm.command_storage_key_at_slot(&slot).unwrap();
        vm.remove_registered_command(&previous_key);
        let recreated = vm.register_command_in_slot(
            slot.clone(),
            super::Command::Native(std::rc::Rc::new(PreviousOwnerObserver {
                world: std::rc::Rc::downgrade(&vm.name_world),
                released: std::rc::Rc::clone(&released),
                label: b'C',
            })),
        );
        let recreated_token = vm.visible_command_generation(&recreated).unwrap();
        assert_ne!(recreated_token, record.generation);
        let before = released.borrow().clone();
        let _ = vm.refuse_host_command("FIRST_BEFORE_RECREATED_LOCAL_CLEANUP".into());
        let first = vm.execution_refusal.clone().unwrap();
        let error = vm.clean_jim_local_commands(vec![record]).unwrap_err();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
        assert_eq!(
            vm.command_storage_key_at_slot(&slot).as_deref(),
            Some(recreated.as_str())
        );
        assert_eq!(
            vm.visible_command_generation(&recreated),
            Some(recreated_token)
        );
        assert_eq!(*released.borrow(), before);
        assert!(released.borrow().iter().all(|(label, _)| *label != b'C'));
        assert_eq!(original.native_object_reference_count(), 1);
        assert_eq!(
            original.resident_string_bytes().unwrap().as_ref(),
            b"original_local"
        );
    }
}
