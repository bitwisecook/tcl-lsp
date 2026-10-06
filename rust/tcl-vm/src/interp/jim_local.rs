// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim previous-command owners and frame-owned original cleanup names.

use super::native_jim_lookup::{JimCommandLease, JimCommandNode};
use super::{
    Command, CommandSlot, Completion, NameBytes, ROOT_NS, Rc, RefCell, Value, Vm, err,
    native_name_world,
};
use tcl_syntax::value::ValueError;

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
        let key = self.register_command_in_slot(slot, command);
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

    pub(super) fn clean_jim_local_commands(&mut self, names: Vec<Value>) {
        for original in names.into_iter().rev() {
            let Ok(name) = self.native_name_operand_bytes(&original) else {
                let _ = self
                    .refuse_host_command("Jim local cleanup original string is unavailable".into());
                break;
            };
            // JimDeleteLocalProcs uses the original counted hash comparison key.
            let slot = CommandSlot {
                namespace: ROOT_NS,
                simple: NameBytes::from(name.as_ref()),
            };
            let Some(key) = self.command_storage_key_at_slot(&slot) else {
                continue;
            };
            let Some(token) = self.visible_command_generation(&key) else {
                continue;
            };
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
            self.take_command_unchecked_key(&key);
            if let Some(previous) = previous {
                let display = previous.name.borrow().clone();
                let restored_key = self.storage_key_for_command_slot(slot, display.as_bytes());
                self.commands
                    .insert(restored_key.clone(), previous.command.borrow().clone());
                {
                    let mut world = self.name_world.borrow_mut();
                    world
                        .command_identity
                        .generations
                        .insert(restored_key.clone(), previous.token);
                    world
                        .jim_command_nodes
                        .insert(previous.token, Rc::clone(&previous));
                }
                (*previous.storage_key.borrow_mut()).clone_from(&restored_key);
                if let Some(identity) = previous.builtin_identity.borrow().clone() {
                    self.builtin_identities
                        .insert(restored_key.clone(), identity);
                }
                self.note_command_bound(&restored_key, false);
                self.invalidate_compiled_command_semantics_for_key(&restored_key);
            }
        }
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
    if !completion.code.is_ok() {
        return completion;
    }
    match vm.native_jim_command_from_original_at(vm.current_ns_id(), &completion.result) {
        Ok(Some(_)) => {
            vm.frames
                .last_mut()
                .expect("actual Jim frame")
                .jim_local_commands
                .push(completion.result.clone());
            completion
        }
        Ok(None) => match vm.native_name_operand_bytes(&completion.result) {
            Ok(bytes) => err(
                tcl_syntax::native_jim_local::NativeJimLocalProtocol::jim084()
                    .missing_cleanup_command(&bytes),
            ),
            Err(error) => vm.refuse_host_command(error.to_string()),
        },
        Err(error) => vm.refuse_host_command(error.to_string()),
    }
}

fn upcall(vm: &mut Vm, arguments: &[Value]) -> Completion<Value> {
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
    if let Some(node) = vm.selected_original_jim_node(head) {
        let procedure = matches!(&*node.command.borrow(), Command::Proc(_));
        if recipe.accepts_upcall(procedure, node.previous.borrow().is_some()) {
            let Some(lease) = vm.retain_active_jim_command(node.token) else {
                return vm.refuse_host_command("Jim upcall selected worker is unavailable".into());
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
    match vm.native_name_operand_bytes(head) {
        Ok(bytes) => err(recipe.missing_previous_command(&bytes)),
        Err(error) => vm.refuse_host_command(error.to_string()),
    }
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
}
