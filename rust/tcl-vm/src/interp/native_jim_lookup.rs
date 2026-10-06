// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim lookup owners, independent of C caches and compiler generations.

use super::{
    Command, CommandSidecarKey, CommandSlot, Completion, Local, NameBytes, NsId, ROOT_NS, Rc,
    RefCell, Value, VarId, Vm, native_name_world,
};
use std::cell::Cell;
use std::rc::Weak;
use tcl_syntax::value::ValueError;

pub(super) struct JimCommandNode {
    pub(super) token: u64,
    pub(super) command: RefCell<Command>,
    pub(super) storage_key: RefCell<String>,
    pub(super) name: RefCell<NameBytes>,
    pub(super) published: Cell<bool>,
    pub(super) active: Cell<usize>,
    pub(super) previous: RefCell<Option<Rc<Self>>>,
    pub(super) previous_owners: Cell<usize>,
    pub(super) upcalls: Cell<usize>,
    pub(super) builtin_identity: RefCell<Option<String>>,
}
impl JimCommandNode {
    pub(super) fn unpublish(&self) {
        self.published.set(false);
    }
    pub(super) fn is_live(&self) -> bool {
        self.published.get() || self.active.get() != 0 || self.previous_owners.get() != 0
    }
}

/// An actual invocation owns its node until its synchronous or NRE continuation ends.
pub(crate) struct JimCommandLease {
    node: Rc<JimCommandNode>,
    world: Weak<RefCell<native_name_world::NativeNameWorld>>,
}
impl JimCommandLease {
    pub(crate) fn is_active(&self) -> bool {
        self.node.active.get() != 0
    }
}
impl Drop for JimCommandLease {
    fn drop(&mut self) {
        let active = self.node.active.get();
        assert!(active != 0, "Jim actual command invocation lease");
        self.node.active.set(active - 1);
        if !self.node.is_live()
            && let Some(world) = self.world.upgrade()
        {
            world.borrow_mut().retire_jim_node(&self.node);
        }
    }
}

impl native_name_world::NativeNameWorld {
    pub(super) fn retire_jim_node(&mut self, node: &Rc<JimCommandNode>) {
        self.note_jim_retirement(node.token);
        if let Some(previous) = node.previous.borrow_mut().take() {
            let owners = previous.previous_owners.get();
            assert!(owners != 0, "actual previous Jim command owner");
            previous.previous_owners.set(owners - 1);
            if !previous.is_live() {
                self.retire_jim_node(&previous);
            }
        }
    }
    pub(super) fn advance_jim_procedure_epoch(&mut self) {
        self.jim_procedure_epoch = self
            .jim_procedure_epoch
            .checked_add(1)
            .expect("Jim procedure epoch exhausted");
        self.jim_retired_commands = 0;
        self.jim_retired_tokens.clear();
    }
    fn note_jim_retirement(&mut self, token: u64) {
        if !self.jim_retired_tokens.insert(token) {
            return;
        }
        self.jim_retired_commands += 1;
        if self.jim_retired_commands >= 1000 {
            self.advance_jim_procedure_epoch();
            self.jim_command_receipts
                .retain(|_, node| node.strong_count() != 0);
        }
    }
}

impl Vm {
    pub(crate) fn uses_native_jim_lookup(&self) -> bool {
        if self.observed_name_policy_selected() {
            return false;
        }
        self.name_policy_protocol().is_some_and(|policy| {
            policy.authority() == tcl_syntax::naming::NamePolicyAuthority::Native
                && policy.recipe().is_jim084()
        }) && self
            .actual_native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
    }

    pub(crate) fn new_jim_declaration_namespace(&self, namespace: NsId) -> Value {
        let context = self
            .native_jim_object_context()
            .expect("actual Jim declaration issuer");
        if namespace == ROOT_NS {
            return context.empty_object().clone();
        }
        // JimUpdateProcNamespace creates this new original object from the
        // counted command namespace; it does not copy the current frame object.
        let original = Value::from_native_string_bytes(self.ns_name_bytes(namespace).as_bytes());
        original
            .bind_native_jim_context(&context)
            .expect("same Jim declaration context");
        original
    }

    fn with_jim_current_namespace<R>(
        &self,
        read: impl FnOnce(&Value) -> Result<R, ValueError>,
    ) -> Result<R, ValueError> {
        let context = self.native_jim_object_context()?;
        let namespace = self.current_ns_id();
        if namespace == ROOT_NS {
            return read(&context.empty_object());
        }
        let frame = self
            .frames
            .last()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim current original namespace frame",
            ))?;
        if !frame
            .jim_namespace
            .borrow()
            .as_ref()
            .is_some_and(|(token, _)| *token == namespace)
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim current original namespace owner",
            ));
        }
        let holder = frame.jim_namespace.borrow();
        read(&holder.as_ref().expect("checked original namespace owner").1)
    }

    pub(super) fn note_jim_command_publication(
        &self,
        token: u64,
        command: &Command,
        key: &str,
        name: &[u8],
    ) {
        if !self.uses_native_jim_lookup() {
            return;
        }
        let Some(recipe) = self
            .actual_native_invocation_dialect()
            .native_jim_lookup_protocol()
        else {
            return;
        };
        let shadows_root = matches!(command, Command::Proc(_))
            && recipe.procedure_shadow_tail(name).is_some_and(|tail| {
                let world = self.name_world.borrow();
                let slot = CommandSlot {
                    namespace: ROOT_NS,
                    simple: NameBytes::from(tail),
                };
                world
                    .command_identity
                    .by_slot
                    .get(&slot)
                    .is_some_and(|key| world.command_identity.generations.contains_key(key))
            });
        let node = Rc::new(JimCommandNode {
            token,
            command: RefCell::new(command.clone()),
            storage_key: RefCell::new(key.to_owned()),
            name: RefCell::new(NameBytes::from(name)),
            published: Cell::new(true),
            active: Cell::new(0),
            previous: RefCell::new(None),
            previous_owners: Cell::new(0),
            upcalls: Cell::new(0),
            builtin_identity: RefCell::new(self.builtin_identity_for_key(key)),
        });
        let mut world = self.name_world.borrow_mut();
        if shadows_root {
            world.advance_jim_procedure_epoch();
        }
        world
            .jim_command_receipts
            .insert(token, Rc::downgrade(&node));
        world.jim_command_nodes.insert(token, node);
    }

    pub(super) fn relocate_jim_command_node(&self, token: u64, command: &Command, key: &str) {
        if let Some(node) = self
            .name_world
            .borrow()
            .jim_command_receipts
            .get(&token)
            .and_then(Weak::upgrade)
        {
            *node.command.borrow_mut() = command.clone();
            key.clone_into(&mut node.storage_key.borrow_mut());
            *node.name.borrow_mut() = self.command_display_key_bytes(key);
        }
    }

    pub(super) fn note_jim_command_unpublished(&self, token: u64) {
        let mut world = self.name_world.borrow_mut();
        if let Some(node) = world.jim_command_nodes.remove(&token) {
            node.unpublish();
            if !node.is_live() {
                world.retire_jim_node(&node);
            }
        }
    }

    pub(crate) fn retain_active_jim_command(&self, token: u64) -> Option<JimCommandLease> {
        if !self.uses_native_jim_lookup() {
            return None;
        }
        self.actual_native_invocation_dialect()
            .native_jim_lookup_protocol()?;
        let node = self
            .name_world
            .borrow()
            .jim_command_receipts
            .get(&token)?
            .upgrade()?;
        if !node.is_live() {
            return None;
        }
        node.active.set(
            node.active
                .get()
                .checked_add(1)
                .expect("Jim inUse overflow"),
        );
        Some(JimCommandLease {
            node,
            world: Rc::downgrade(&self.name_world),
        })
    }

    pub(crate) fn retain_original_jim_command(
        &self,
        original: &Value,
        selected: Option<&str>,
    ) -> Option<JimCommandLease> {
        let token = self
            .selected_original_jim_node(original)
            .map(|node| node.token)
            .or_else(|| selected.and_then(|key| self.visible_command_generation(key)))?;
        self.retain_active_jim_command(token)
    }

    pub(crate) fn jim_original_command_reporting_name(
        &self,
        original: &Value,
    ) -> Option<NameBytes> {
        let token = original.with_jim_command_cache(|cache| cache.token)?;
        let node = self
            .name_world
            .borrow()
            .jim_command_receipts
            .get(&token)?
            .upgrade()?;
        let name = node.name.borrow().clone();
        Some(name)
    }

    fn cached_original_jim_command(
        &self,
        original: &Value,
        recipe: tcl_syntax::native_jim_lookup::NativeJimLookupProtocol,
    ) -> Result<Option<(String, Command)>, ValueError> {
        Ok(original
            .with_jim_command_cache(|cache| {
                if cache.interpreter != self.native_interpreter_identity() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "foreign Jim command cache",
                    ));
                }
                let epoch = self.name_world.borrow().jim_procedure_epoch;
                if cache.epoch != epoch {
                    return Ok(None);
                }
                let same_namespace = self.with_jim_current_namespace(|namespace| {
                    let current = namespace
                        .native_string_bytes(
                            tcl_syntax::native_string::NativeStringProtocol::Jim084,
                        )
                        .map_err(|_| {
                            ValueError::CommandProtocolUnavailable(
                                "Jim original current namespace spelling",
                            )
                        })?;
                    let cached = cache
                        .namespace
                        .native_string_bytes(
                            tcl_syntax::native_string::NativeStringProtocol::Jim084,
                        )
                        .map_err(|_| {
                            ValueError::CommandProtocolUnavailable(
                                "Jim original cached namespace spelling",
                            )
                        })?;
                    Ok(current == cached)
                })?;
                let node = self
                    .name_world
                    .borrow()
                    .jim_command_receipts
                    .get(&cache.token)
                    .and_then(Weak::upgrade);
                let live = node.as_ref().is_some_and(|node| node.is_live());
                if !recipe.command_is_current(cache.epoch, epoch, same_namespace, live) {
                    return Ok(None);
                }
                let node =
                    self.select_jim_previous_node(node.expect("checked original command node"))?;
                let key = self
                    .command_token_at_generation(node.token)
                    .and_then(|identity| match identity.key {
                        CommandSidecarKey::Visible(key) => Some(key),
                        CommandSidecarKey::Hidden(_) => None,
                    })
                    .unwrap_or_else(|| node.storage_key.borrow().clone());
                let worker = node.command.borrow().clone();
                Ok(Some((key, worker)))
            })
            .transpose()?
            .flatten())
    }

    pub(super) fn native_jim_command_from_original_at(
        &self,
        current: NsId,
        original: &Value,
    ) -> Result<Option<(String, Command)>, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        let recipe =
            dialect
                .native_jim_lookup_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim original command lookup issuer",
                ))?;
        let hit = self.cached_original_jim_command(original, recipe)?;
        if hit.is_some() {
            return Ok(hit);
        }
        let bytes = self
            .native_name_operand_bytes(original)
            .map_err(|_| ValueError::CommandProtocolUnavailable("Jim original command spelling"))?;
        let Some(key) = self
            .resolve_command_bytes_checked(current, &bytes, true)
            .map_err(|_| ValueError::CommandProtocolUnavailable("Jim original command lookup"))?
        else {
            return Ok(None);
        };
        let command =
            self.visible_command_at_key(&key)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim selected command implementation",
                ))?;
        let token =
            self.visible_command_generation(&key)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim selected command node",
                ))?;
        if !self
            .name_world
            .borrow()
            .jim_command_receipts
            .contains_key(&token)
        {
            self.note_jim_command_publication(token, &command, &key, &bytes);
        }
        let namespace = self.with_jim_current_namespace(|original| Ok(original.clone()))?;
        let epoch = self.name_world.borrow().jim_procedure_epoch;
        original.install_jim_command_cache(
            crate::value::JimCommandCache {
                interpreter: self.native_interpreter_identity(),
                epoch,
                token,
                namespace,
            },
            dialect,
        )?;
        let selected = self.selected_original_jim_node(original).ok_or(
            ValueError::CommandProtocolUnavailable("selected Jim previous-command node"),
        )?;
        let selected_key = selected.storage_key.borrow().clone();
        let selected_command = selected.command.borrow().clone();
        Ok(Some((selected_key, selected_command)))
    }

    fn jim_variable_frame_id(&self, global: bool) -> Result<u64, ValueError> {
        self.frames
            .get(if global { 0 } else { self.current_level() })
            .map(|frame| frame.jim_lookup_id.get())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim actual variable frame",
            ))
    }

    pub(super) fn current_jim_variable_cell(
        &self,
        original: &Value,
    ) -> Result<Option<VarId>, ValueError> {
        let recipe = self
            .actual_native_invocation_dialect()
            .native_jim_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim original variable lookup issuer",
            ))?;
        original
            .with_jim_variable_cache(|cache| {
                if cache.interpreter != self.native_interpreter_identity() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "foreign Jim variable cache",
                    ));
                }
                if !recipe
                    .variable_is_current(cache.frame, self.jim_variable_frame_id(cache.global)?)
                {
                    return Ok(None);
                }
                Ok(self.var_arena.validate_jim_cell(&cache.cell))
            })
            .transpose()
            .map(Option::flatten)
    }

    pub(super) fn install_original_jim_variable(
        &self,
        original: &Value,
        bytes: &[u8],
    ) -> Result<(), ValueError> {
        let projection = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim original variable naming",
            ))?
            .recipe()
            .combined_variable_input(bytes);
        if projection.element().is_some() {
            return Ok(());
        }
        let Some(binding) = self.var_binding_from_bytes(bytes, self.current_level()) else {
            return Ok(());
        };
        let Some(raw) = self.raw_variable_binding(&binding) else {
            return Ok(());
        };
        let Some(cell) = self.var_arena.weak_jim_cell(raw) else {
            return Ok(());
        };
        let global = bytes.starts_with(b"::");
        original.install_jim_variable_cache(
            crate::value::JimVariableCache {
                interpreter: self.native_interpreter_identity(),
                frame: self.jim_variable_frame_id(global)?,
                global,
                cell,
            },
            self.actual_native_invocation_dialect(),
        )
    }

    pub(crate) fn read_original_named_variable(
        &mut self,
        original: &Value,
    ) -> Result<Value, Completion<Value>> {
        if self.native_c_variable_name_protocol().is_some() {
            return self.read_original_c_variable(original);
        }
        if self.uses_native_jim_lookup() {
            let raw = self
                .current_jim_variable_cell(original)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            if let Some(raw) = raw
                && let Some(Local::Scalar(value)) =
                    self.var_arena.get(raw).map(crate::vars::VarCell::state)
            {
                return Ok(value.clone());
            }
            if let Some((target, level)) = self.original_jim_alias_target(original) {
                return self.read_original_jim_alias(original, target.value(), level);
            }
        }
        let bytes = self
            .native_name_operand_bytes(original)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        if self.uses_native_jim_lookup() {
            if Self::is_original_jim_dictionary_name(original, &bytes) {
                let context = self
                    .native_jim_object_context()
                    .map_err(|error| self.refuse_host_command(error.to_string()))?;
                original
                    .ensure_native_jim_dictionary_substitution(&context)
                    .map_err(|error| self.refuse_host_command(error.to_string()))?;
                let (name, key) = original
                    .with_native_jim_dictionary_substitution(|name, key| {
                        (name.native_lifetime_lease(), key.native_lifetime_lease())
                    })
                    .expect("installed original tuple");
                return self.read_native_jim_dictionary_member(name.value(), key.value());
            }
            self.install_original_jim_variable(original, &bytes)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            if let Some((target, level)) = self.original_jim_alias_target(original) {
                return self.read_original_jim_alias(original, target.value(), level);
            }
        }
        self.read_variable_result_bytes(&bytes, None)
    }

    pub(crate) fn store_original_named_variable(
        &mut self,
        original: &Value,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        if self.native_c_variable_name_protocol().is_some() {
            return self.store_original_c_variable(original, value);
        }
        if self.uses_native_jim_lookup() {
            let bytes = self
                .native_name_operand_bytes(original)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            if Self::is_original_jim_dictionary_name(original, &bytes) {
                return self.store_original_jim_dictionary_name(original, value);
            }
            self.install_original_jim_variable(original, &bytes)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            if let Some((target, level)) = self.original_jim_alias_target(original) {
                return self.store_original_jim_alias(target.value(), level, value);
            }
            let raw = self
                .current_jim_variable_cell(original)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            if let Some(raw) = raw
                && self
                    .var_arena
                    .get(raw)
                    .is_some_and(|cell| matches!(cell.state(), Local::Scalar(_) | Local::Undefined))
            {
                if !self
                    .var_arena
                    .replace_state(raw, Local::Scalar(value.clone()))
                {
                    return Err(
                        self.refuse_host_command("Jim original variable receiver expired".into())
                    );
                }
                return Ok(value);
            }
        }
        let bytes = self
            .native_name_operand_bytes(original)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let result = self.store_var_result_bytes(&bytes, value)?;
        if self.uses_native_jim_lookup() {
            let projection = self
                .name_policy_protocol()
                .expect("selected Jim naming")
                .recipe()
                .combined_variable_input(&bytes);
            if projection.element().is_none()
                && let Some(binding) = self.var_binding_from_bytes(&bytes, self.current_level())
                && self.raw_variable_binding(&binding).is_some()
            {
                // Jim strips a global marker into a distinct original key;
                // an unqualified first birth retains the very same operand.
                let key = if bytes.starts_with(b"::") {
                    let key = Value::from_native_string_bytes(binding.name.as_bytes());
                    key.bind_native_jim_context(
                        &self
                            .native_jim_object_context()
                            .map_err(|error| self.refuse_host_command(error.to_string()))?,
                    )
                    .map_err(|error| self.refuse_host_command(error.to_string()))?;
                    key
                } else {
                    original.clone()
                };
                self.var_table_mut(binding.owner)
                    .expect("actual Jim variable name table")
                    .retain_original_jim_key(&binding.name, key);
            }
            self.install_original_jim_variable(original, &bytes)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::ok;
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;

    fn actual_jim() -> Vm {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = Vm::new();
        assert!(vm.set_native_engine_profile(profile));
        vm.set_dialect_profile(profile);
        vm
    }
    struct ResultCommand(i64);
    impl crate::NativeCommand for ResultCommand {
        fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
            ok(Value::int(self.0))
        }
    }

    #[test]
    fn original_command_replacement_uses_native_node_liveness_without_epoch_drift() {
        let mut vm = actual_jim();
        vm.register_written_command("opaque", Command::Native(Rc::new(ResultCommand(1))));
        let original = Value::from_native_string_bytes(b"opaque".as_slice());
        let (_, first_command) = vm
            .native_jim_command_from_original_at(ROOT_NS, &original)
            .unwrap()
            .unwrap();
        let first_token = original
            .with_jim_command_cache(|cache| cache.token)
            .unwrap();
        let epoch = vm.name_world.borrow().jim_procedure_epoch;
        let ns_refs = vm
            .native_jim_object_context()
            .unwrap()
            .empty_object()
            .native_object_reference_count();
        let duplicate = original
            .duplicate_native_object_in(tcl_syntax::native_string::NativeStringProtocol::Jim084);
        assert_eq!(
            duplicate.native_object_snapshot().cache,
            Cache::JimCommand {
                procedure_epoch: epoch
            }
        );
        assert_eq!(
            vm.native_jim_object_context()
                .unwrap()
                .empty_object()
                .native_object_reference_count(),
            ns_refs + 1
        );
        drop(duplicate);
        vm.register_written_command("opaque", Command::Native(Rc::new(ResultCommand(2))));
        assert_eq!(vm.name_world.borrow().jim_procedure_epoch, epoch);
        let (_, second_command) = vm
            .native_jim_command_from_original_at(ROOT_NS, &original)
            .unwrap()
            .unwrap();
        assert_ne!(
            original
                .with_jim_command_cache(|cache| cache.token)
                .unwrap(),
            first_token
        );
        match (first_command, second_command) {
            (Command::Native(a), Command::Native(b)) => {
                assert_eq!(a.invoke(&mut vm, &[]).result.as_int().unwrap(), 1);
                assert_eq!(b.invoke(&mut vm, &[]).result.as_int().unwrap(), 2);
            }
            _ => panic!("actual selected native workers"),
        }
    }

    #[test]
    fn original_deleted_active_command_survives_only_its_actual_invocation_lease() {
        let mut vm = actual_jim();
        let key = vm.register_written_command("active", Command::Native(Rc::new(ResultCommand(1))));
        let original = Value::from_native_string_bytes(b"active".as_slice());
        vm.native_jim_command_from_original_at(ROOT_NS, &original)
            .unwrap()
            .unwrap();
        let token = original
            .with_jim_command_cache(|cache| cache.token)
            .unwrap();
        let lease = vm.retain_active_jim_command(token).unwrap();
        let epoch = vm.name_world.borrow().jim_procedure_epoch;
        vm.take_command_unchecked_key(&key).unwrap();
        assert!(
            vm.native_jim_command_from_original_at(ROOT_NS, &original)
                .unwrap()
                .is_some()
        );
        let fresh = Value::from_native_string_bytes(b"active".as_slice());
        assert!(
            vm.native_jim_command_from_original_at(ROOT_NS, &fresh)
                .unwrap()
                .is_none()
        );
        assert_eq!(fresh.native_object_snapshot().cache, Cache::None);
        drop(lease);
        assert!(
            vm.native_jim_command_from_original_at(ROOT_NS, &original)
                .unwrap()
                .is_none()
        );
        assert_eq!(vm.name_world.borrow().jim_procedure_epoch, epoch);
        assert_eq!(
            original.native_object_snapshot().cache,
            Cache::JimCommand {
                procedure_epoch: epoch
            }
        );
    }

    #[test]
    fn original_variable_cache_owns_no_cell_and_unset_changes_only_native_lookup_id() {
        let mut vm = actual_jim();
        let original = Value::from_native_string_bytes(b"k\0\xff".as_slice());
        assert_eq!(original.native_object_reference_count(), 1);
        vm.store_original_named_variable(&original, Value::int(7))
            .unwrap();
        assert_eq!(original.native_object_reference_count(), 2);
        let old_frame = vm.frames[0].jim_lookup_id.get();
        let activation = vm.frames[0].activation.serial;
        assert_eq!(
            original.native_object_snapshot().cache,
            Cache::JimVariable {
                frame: old_frame,
                global: false
            }
        );
        let duplicate = original
            .duplicate_native_object_in(tcl_syntax::native_string::NativeStringProtocol::Jim084);
        let count = vm.var_arena.len();
        assert_eq!(
            vm.read_original_named_variable(&duplicate)
                .unwrap()
                .as_int()
                .unwrap(),
            7
        );
        assert_eq!(vm.var_arena.len(), count);
        vm.store_original_named_variable(
            &Value::from_native_string_bytes(b"other".as_slice()),
            Value::int(9),
        )
        .unwrap();
        assert!(vm.unset_var_bytes(b"other"));
        assert_ne!(vm.frames[0].jim_lookup_id.get(), old_frame);
        assert_eq!(vm.frames[0].activation.serial, activation);
        assert_eq!(
            vm.read_original_named_variable(&original)
                .unwrap()
                .as_int()
                .unwrap(),
            7
        );
        assert!(vm.unset_var_bytes(b"k\0\xff"));
        assert_eq!(original.native_object_reference_count(), 1);
        let before = original.native_object_snapshot().cache;
        assert!(vm.read_original_named_variable(&original).is_err());
        assert_eq!(original.native_object_snapshot().cache, before);
    }

    #[test]
    fn global_original_uses_separate_stripped_key_and_foreign_caches_refuse() {
        let mut vm = actual_jim();
        let original = Value::from_native_string_bytes(b"::global\xff".as_slice());
        vm.store_original_named_variable(&original, Value::int(4))
            .unwrap();
        assert_eq!(original.native_object_reference_count(), 1);
        assert!(matches!(
            original.native_object_snapshot().cache,
            Cache::JimVariable { global: true, .. }
        ));
        let mut other = actual_jim();
        assert!(other.read_original_named_variable(&original).is_err());
        assert!(other.execution_refusal.is_some());
        assert_eq!(
            vm.read_original_named_variable(&original)
                .unwrap()
                .as_int()
                .unwrap(),
            4
        );
    }
}
