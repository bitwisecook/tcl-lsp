// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim lookup owners, independent of C caches and compiler generations.

use super::native_original_lookup::OriginalLookup;
use super::{
    Command, CommandSidecarKey, CommandSlot, Completion, Local, NameBytes,
    NamespaceExecutionContext, NsId, ROOT_NS, Rc, RefCell, Value, VarId, Vm, native_name_world,
};
use std::cell::Cell;
use std::rc::Weak;
use tcl_cmd_core::CmdError;
use tcl_syntax::value::ValueError;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OriginalJimVariableResult {
    Retained,
    Borrowed,
}
impl OriginalJimVariableResult {
    pub(crate) fn select(self, value: &Value) -> Value {
        match self {
            Self::Retained => value.clone(),
            Self::Borrowed => value.native_lifetime_lease().into_value(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OriginalJimVariableReport {
    Missing,
    Silent,
}

/// The earlier quiet getter's exact header, without an additional native hold.
pub(crate) struct OriginalJimErrorCodeCapture {
    caller: super::native_name_consumers::ParkedOriginalNameConsumerScope,
    original: Option<crate::value::NativeObjectLifetimeLease>,
}

impl OriginalJimErrorCodeCapture {
    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.caller.check(vm)
    }

    pub(crate) fn original(&self, vm: &mut Vm) -> Result<Option<Value>, CmdError> {
        self.check(vm)?;
        self.original
            .as_ref()
            .map(|original| {
                self.caller.current_for_value(vm, original.value())?;
                Ok(original.value().native_lifetime_lease().into_value())
            })
            .transpose()
    }
}

impl Vm {
    pub(crate) fn capture_original_jim_error_code(
        &mut self,
    ) -> Result<OriginalJimErrorCodeCapture, CmdError> {
        let caller = super::native_name_consumers::OriginalNameConsumerScope::enter(self)?;
        let reached = self.read_original_jim_global_error_code().map_err(|_| {
            self.execution_refusal.clone().map_or_else(
                || ValueError::CommandProtocolUnavailable("original Jim error-code capture").into(),
                CmdError::from_execution_refusal,
            )
        })?;
        caller.check(self)?;
        if let Some(value) = &reached {
            caller.current_for_value(self, value)?;
        }
        let original = reached.as_ref().map(Value::native_lifetime_lease);
        drop(reached);
        caller.check(self)?;
        Ok(OriginalJimErrorCodeCapture {
            caller: caller.park(),
            original,
        })
    }
}

#[cfg(test)]
mod global_error_code_tests;

/// Mutable name metadata owns only the original key's allocation, never a
/// command worker or a native object reference.
#[derive(Clone, Default)]
pub(crate) struct OriginalJimCommandName {
    original: Rc<RefCell<Option<crate::value::JimCommandNameObservation>>>,
}

impl OriginalJimCommandName {
    pub(crate) fn is_observed(&self) -> bool {
        self.original.borrow().is_some()
    }

    pub(crate) fn original(&self) -> Result<Option<Value>, ValueError> {
        let original = self.original.borrow();
        let Some(original) = original.as_ref() else {
            return Ok(None);
        };
        original.current().map(Some)
    }
}

pub(super) struct JimCommandNode {
    pub(super) token: u64,
    pub(super) command: RefCell<Command>,
    pub(super) storage_key: RefCell<String>,
    pub(super) observed_name: OriginalJimCommandName,
    pub(super) table_key: RefCell<tcl_syntax::naming::NativeJimCommandTableKey>,
    pub(super) published: Cell<bool>,
    pub(super) retired: Rc<Cell<bool>>,
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
            let retired = world.borrow_mut().retire_jim_node(&self.node);
            drop(retired);
        }
    }
}

impl native_name_world::NativeNameWorld {
    pub(super) fn retire_jim_node(&mut self, node: &Rc<JimCommandNode>) -> Vec<Rc<JimCommandNode>> {
        node.retired.set(true);
        self.note_jim_retirement(node.token);
        let previous = node.previous.borrow_mut().take();
        let Some(previous) = previous else {
            return Vec::new();
        };
        let owners = previous.previous_owners.get();
        assert!(owners != 0, "actual previous Jim command owner");
        previous.previous_owners.set(owners - 1);
        let mut retired = if previous.is_live() {
            Vec::new()
        } else {
            self.retire_jim_node(&previous)
        };
        // Retain the extracted edge until the caller has released the world
        // borrow. Deeper previous edges retain their existing release order.
        retired.push(previous);
        retired
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

/// Retire the captured detached node in its actual retained interpreter world.
/// The caller preserves original execution currency independently of this owner.
pub(super) fn retire_detached_original_jim_node_in(
    world: &Rc<RefCell<native_name_world::NativeNameWorld>>,
    node: Rc<JimCommandNode>,
) {
    node.unpublish();
    let retired = if node.is_live() {
        Vec::new()
    } else {
        world.borrow_mut().retire_jim_node(&node)
    };
    drop(retired);
    drop(node);
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

    pub(crate) fn with_jim_current_namespace<R>(
        &self,
        read: impl FnOnce(&Value) -> Result<R, ValueError>,
    ) -> Result<R, ValueError> {
        match self.current_namespace_execution_context() {
            NamespaceExecutionContext::Resolved(namespace) => {
                self.with_jim_namespace_at(namespace, read)
            }
            NamespaceExecutionContext::OriginalJim(context) => {
                if !context.owner_is_current(self) {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "Jim original current namespace owner",
                    ));
                }
                read(context.original())
            }
        }
    }

    /// Borrow the actual original namespace object for this explicit token.
    /// Metadata/display bytes cannot replace a missing frame or root owner.
    pub(super) fn with_jim_namespace_at<R>(
        &self,
        namespace: NsId,
        read: impl FnOnce(&Value) -> Result<R, ValueError>,
    ) -> Result<R, ValueError> {
        if !self.native_namespace_token_has_owner(namespace) {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim explicit original namespace token",
            ));
        }
        let context = self.native_jim_object_context()?;
        if namespace == ROOT_NS {
            return read(&context.empty_object());
        }
        let original = self
            .frames
            .iter()
            .rev()
            .find_map(|frame| {
                frame
                    .jim_namespace
                    .borrow()
                    .as_ref()
                    .filter(|(token, _)| *token == Some(namespace))
                    .map(|(_, original)| original.native_lifetime_lease())
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim explicit original namespace owner",
            ))?;
        // All owner borrows have ended before a reached physical getter.
        read(original.value())
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
        let table_key = self
            .name_world
            .borrow()
            .jim_command_table_keys
            .get(key)
            .cloned()
            .or_else(|| {
                self.command_slot(key).and_then(|slot| {
                    tcl_syntax::naming::NativeJimCommandTableKey::from_comparison_key(
                        tcl_syntax::naming::NativeNameProtocol::Jim084,
                        slot.simple.as_bytes(),
                    )
                })
            })
            .expect("actual Jim command table comparison key");
        self.name_world
            .borrow_mut()
            .jim_command_table_keys
            .insert(key.to_owned(), table_key.clone());
        let node = Rc::new(JimCommandNode {
            token,
            command: RefCell::new(command.clone()),
            storage_key: RefCell::new(key.to_owned()),
            observed_name: OriginalJimCommandName::default(),
            table_key: RefCell::new(table_key),
            published: Cell::new(true),
            retired: Rc::new(Cell::new(false)),
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
            if let Some(table_key) = self.name_world.borrow().jim_command_table_keys.get(key) {
                node.table_key.borrow_mut().clone_from(table_key);
            }
        }
    }

    pub(super) fn note_jim_command_unpublished(&self, token: u64) {
        let node = self
            .name_world
            .borrow_mut()
            .jim_command_nodes
            .remove(&token);
        if let Some(node) = node {
            node.unpublish();
            let retired = if node.is_live() {
                Vec::new()
            } else {
                self.name_world.borrow_mut().retire_jim_node(&node)
            };
            drop(retired);
            drop(node);
        }
    }

    /// Release an already detached previous node after a refused restoration.
    /// Active invocations and owning previous edges retain their existing roles.
    #[cfg(test)]
    pub(super) fn retire_detached_original_jim_node(&mut self, node: Rc<JimCommandNode>) {
        retire_detached_original_jim_node_in(&self.name_world, node);
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

    pub(crate) fn original_jim_command_name_reference(
        &self,
        original: &Value,
    ) -> Option<OriginalJimCommandName> {
        Some(
            self.selected_original_jim_node(original)?
                .observed_name
                .clone(),
        )
    }

    fn cached_original_jim_command(
        &mut self,
        entered: &OriginalLookup,
        original: &Value,
        recipe: tcl_syntax::native_jim_lookup::NativeJimLookupProtocol,
    ) -> Result<Option<(String, Command)>, CmdError> {
        let Some((interpreter, cached_epoch, token, namespace)) =
            original.with_jim_command_cache(|cache| {
                (
                    cache.interpreter,
                    cache.epoch,
                    cache.token,
                    cache.namespace.native_lifetime_lease(),
                )
            })
        else {
            return Ok(None);
        };
        if interpreter != self.native_interpreter_identity() {
            return Err(ValueError::CommandProtocolUnavailable("foreign Jim command cache").into());
        }
        let epoch = self.name_world.borrow().jim_procedure_epoch;
        if cached_epoch != epoch {
            return Ok(None);
        }
        let current =
            entered
                .jim_namespace_original()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim original current namespace header",
                ))?;
        let same_namespace = if namespace.value().is_same_object(current) {
            true
        } else {
            let cached = self.native_name_operand_bytes_checked(namespace.value())?;
            entered.check(self)?;
            let current = entered.read_jim_namespace(self)?;
            cached == current
        };
        entered.check(self)?;
        let node = self
            .name_world
            .borrow()
            .jim_command_receipts
            .get(&token)
            .and_then(Weak::upgrade);
        let live = node.as_ref().is_some_and(|node| node.is_live());
        let epoch = self.name_world.borrow().jim_procedure_epoch;
        if !recipe.command_is_current(cached_epoch, epoch, same_namespace, live) {
            return Ok(None);
        }
        let node = self.select_jim_previous_node(node.expect("checked original command node"))?;
        let key = self
            .command_token_at_generation(node.token)
            .and_then(|identity| match identity.key {
                CommandSidecarKey::Visible(key) => Some(key),
                CommandSidecarKey::Hidden(_) => None,
            })
            .unwrap_or_else(|| node.storage_key.borrow().clone());
        let worker = node.command.borrow().clone();
        Ok(Some((key, worker)))
    }

    pub(super) fn native_jim_command_from_original_at(
        &mut self,
        current: NsId,
        original: &Value,
    ) -> Result<Option<(String, Command)>, CmdError> {
        self.native_jim_command_from_original_in_context(
            &NamespaceExecutionContext::Resolved(current),
            original,
        )
    }

    pub(super) fn native_jim_command_from_original_in_context(
        &mut self,
        context: &NamespaceExecutionContext,
        original: &Value,
    ) -> Result<Option<(String, Command)>, CmdError> {
        let entered = OriginalLookup::enter_in_context(self, context, original)?;
        let reached: Result<Option<(String, Command)>, CmdError> = (|| {
            let dialect = self.actual_native_invocation_dialect();
            let recipe = dialect.native_jim_lookup_protocol().ok_or(
                ValueError::CommandProtocolUnavailable("Jim original command lookup issuer"),
            )?;
            let hit = self.cached_original_jim_command(&entered, original, recipe)?;
            entered.check(self)?;
            if hit.is_some() {
                return Ok(hit);
            }
            let current = entered.materialize_namespace(self)?;
            let bytes = entered.read(self)?;
            let Some(key) = self
                .resolve_command_bytes_checked(current, &bytes, true)
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("Jim original command lookup")
                })?
            else {
                return Ok(None);
            };
            let selected = entered.command(self, &key)?;
            let token = selected.generation();
            if !self
                .name_world
                .borrow()
                .jim_command_receipts
                .contains_key(&token)
            {
                let command = self.visible_command_at_key(&key).ok_or(
                    ValueError::CommandProtocolUnavailable("Jim selected command implementation"),
                )?;
                self.note_jim_command_publication(token, &command, &key, &bytes);
                drop(command);
            }
            let node = self
                .name_world
                .borrow()
                .jim_command_receipts
                .get(&token)
                .and_then(Weak::upgrade)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original found Jim command node",
                ))?;
            let slot = self
                .command_slot(&key)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original found Jim command slot",
                ))?;
            let observed = {
                let world = self.name_world.borrow();
                let original = world.jim_command_original_keys.get(&slot).ok_or(
                    ValueError::CommandProtocolUnavailable("original found Jim command hash key"),
                )?;
                original.original.check_native_header()?;
                crate::value::JimCommandNameObservation::capture(&original.original)?
            };
            *node.observed_name.original.borrow_mut() = Some(observed);
            let namespace = entered
                .jim_namespace_original()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim original lookup namespace header",
                ))?
                .native_lifetime_lease();
            let epoch = self.name_world.borrow().jim_procedure_epoch;
            let interpreter = self.native_interpreter_identity();
            // The original allocation is observed before retirement. The actual
            // incoming cache's namespace reference is acquired at its adoption stage.
            let reached = original.install_jim_command_cache_checked(
                dialect,
                || {
                    Ok(crate::value::JimCommandCache {
                        interpreter,
                        epoch,
                        token,
                        namespace: namespace.value().clone(),
                    })
                },
                &mut || {
                    selected.check(self, &entered)?;
                    let same_epoch = self.name_world.borrow().jim_procedure_epoch == epoch;
                    entered.require(self, same_epoch, "original Jim command lookup epoch")
                },
            );
            selected.check(self, &entered)?;
            reached?;
            drop(namespace);
            selected.check(self, &entered)?;
            let selected_node = self.selected_original_jim_node(original).ok_or(
                ValueError::CommandProtocolUnavailable("selected Jim previous-command node"),
            )?;
            let selected_key = selected_node.storage_key.borrow().clone();
            let selected_command = selected_node.command.borrow().clone();
            Ok(Some((selected_key, selected_command)))
        })();
        entered.finish(self, reached)
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
        &mut self,
        original: &Value,
        bytes: &[u8],
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        original_names.current_for_value(self, original)?;
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
        let global = projection.root().qualification()
            == tcl_syntax::naming::NativeNameQualification::Absolute;
        let frame = self.jim_variable_frame_id(global)?;
        let cache = crate::value::JimVariableCache {
            interpreter: self.native_interpreter_identity(),
            frame,
            global,
            cell: cell.clone(),
        };
        original.install_jim_variable_cache_checked(
            self.actual_native_invocation_dialect(),
            || Ok(cache),
            &mut || {
                original_names.current_for_value(self, original)?;
                let current = self.jim_variable_frame_id(global)? == frame
                    && self.raw_variable_binding(&binding) == Some(raw)
                    && self.var_arena.validate_jim_cell(&cell) == Some(raw);
                original_names.require(self, current, "original Jim variable cache frame and cell")
            },
        )
    }

    /// Jim `ArraySet`'s nullable original-variable read is independent of C's
    /// `LocateArray` receiver. This borrows its real selected scalar before COW.
    pub(crate) fn original_jim_array_root(
        &mut self,
        original: &Value,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<Option<crate::value::NativeObjectLifetimeLease>, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            use crate::command::completion_from_cmd_error;
            currency
                .check(self)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            original
                .check_native_header()
                .map_err(|error| completion_from_cmd_error(self, error.into()))?;
            if !self.uses_native_jim_lookup() {
                return Err(completion_from_cmd_error(
                    self,
                    ValueError::CommandProtocolUnavailable("original Jim array variable read")
                        .into(),
                ));
            }
            let reached = self.current_jim_variable_cell(original);
            currency
                .check(self)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            if let Some(raw) =
                reached.map_err(|error| completion_from_cmd_error(self, error.into()))?
                && let Some(Local::Scalar(value)) =
                    self.var_arena.get(raw).map(crate::vars::VarCell::state)
            {
                let root = value.native_lifetime_lease();
                root.value()
                    .check_native_header()
                    .map_err(|error| completion_from_cmd_error(self, error.into()))?;
                return Ok(Some(root));
            }
            if let Some((target, level)) = self.original_jim_alias_target(original) {
                let frame = self.select_execution_frame(level).map_err(super::err)?;
                let reached = self.original_jim_array_root(target.value(), currency);
                self.restore_execution_frame(frame);
                currency
                    .check(self)
                    .map_err(|error| completion_from_cmd_error(self, error))?;
                return reached;
            }
            let reached = original_names.read(self, original);
            currency
                .check(self)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            let bytes = reached.map_err(|error| completion_from_cmd_error(self, error))?;
            if Self::is_original_jim_dictionary_name(original, &bytes) {
                // A dictionary-sugar receiver needs its original path owner; this
                // scalar adapter cannot synthesise that tuple from a byte spelling.
                return Err(completion_from_cmd_error(
                    self,
                    ValueError::CommandProtocolUnavailable(
                        "original Jim array dictionary-sugar receiver",
                    )
                    .into(),
                ));
            }
            let reached = self.install_original_jim_variable(original, &bytes, &original_names);
            currency
                .check(self)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            reached.map_err(|error| completion_from_cmd_error(self, error))?;
            if let Some((target, level)) = self.original_jim_alias_target(original) {
                let frame = self.select_execution_frame(level).map_err(super::err)?;
                let reached = self.original_jim_array_root(target.value(), currency);
                self.restore_execution_frame(frame);
                currency
                    .check(self)
                    .map_err(|error| completion_from_cmd_error(self, error))?;
                return reached;
            }
            let reached = self.current_jim_variable_cell(original);
            currency
                .check(self)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            let Some(raw) =
                reached.map_err(|error| completion_from_cmd_error(self, error.into()))?
            else {
                return Ok(None);
            };
            let selected = match self.var_arena.get(raw).map(crate::vars::VarCell::state) {
                Some(Local::Scalar(value)) => Ok(Some(value.native_lifetime_lease())),
                Some(Local::Undefined) | None => Ok(None),
                _ => Err(ValueError::CommandProtocolUnavailable(
                    "original Jim array scalar-root cell",
                )),
            };
            let root = selected.map_err(|error| completion_from_cmd_error(self, error.into()))?;
            if let Some(root) = &root {
                root.value()
                    .check_native_header()
                    .map_err(|error| completion_from_cmd_error(self, error.into()))?;
            }
            Ok(root)
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    pub(crate) fn read_original_named_variable(
        &mut self,
        original: &Value,
    ) -> Result<Value, Completion<Value>> {
        self.read_original_named_variable_reported(original, OriginalJimVariableReport::Missing)
    }

    pub(crate) fn read_original_named_variable_reported(
        &mut self,
        original: &Value,
        report: OriginalJimVariableReport,
    ) -> Result<Value, Completion<Value>> {
        self.read_original_named_variable_using(
            original,
            report,
            OriginalJimVariableResult::Retained,
        )
    }

    fn missing_original_named_variable(
        &mut self,
        report: OriginalJimVariableReport,
        bytes: &[u8],
        original_names: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<Value, Completion<Value>> {
        if report == OriginalJimVariableReport::Silent {
            let context = self
                .native_jim_object_context()
                .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
            let empty = context.empty_object().native_lifetime_lease();
            original_names
                .current_for_value(self, empty.value())
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            return Err(Completion::new(
                super::Code::Error,
                empty.value().native_lifetime_lease().into_value(),
                empty.into_value(),
            ));
        }
        self.read_variable_result_bytes(bytes, None)
    }

    pub(crate) fn read_original_named_variable_using(
        &mut self,
        original: &Value,
        report: OriginalJimVariableReport,
        result: OriginalJimVariableResult,
    ) -> Result<Value, Completion<Value>> {
        if (report == OriginalJimVariableReport::Silent
            || result == OriginalJimVariableResult::Borrowed)
            && !self.uses_native_jim_lookup()
        {
            return Err(crate::command::completion_from_cmd_error(
                self,
                ValueError::CommandProtocolUnavailable("original Jim quiet variable getter").into(),
            ));
        }
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            if self.native_c_variable_name_protocol().is_some() {
                return self.read_original_c_variable(original);
            }
            if self.uses_native_jim_lookup() {
                let raw = self.current_jim_variable_cell(original).map_err(|error| {
                    crate::command::completion_from_cmd_error(self, error.into())
                })?;
                if let Some(raw) = raw
                    && let Some(Local::Scalar(value)) =
                        self.var_arena.get(raw).map(crate::vars::VarCell::state)
                {
                    return Ok(result.select(value));
                }
                if let Some((target, level)) = self
                    .original_jim_alias_target_checked(original)
                    .map_err(|error| {
                        crate::command::completion_from_cmd_error(self, error.into())
                    })?
                {
                    return self.read_original_jim_alias(
                        original,
                        target.value(),
                        level,
                        report,
                        result,
                    );
                }
            }
            let bytes = original_names
                .read(self, original)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            if self.uses_native_jim_lookup() {
                if Self::is_original_jim_dictionary_name(original, &bytes) {
                    let (name, key) = self
                        .original_jim_tuple_children_in_context(original, &original_names)
                        .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                    return self.read_native_jim_dictionary_member_reported(
                        name.value(),
                        key.value(),
                        &original_names,
                        report,
                        result,
                    );
                }
                self.install_original_jim_variable(original, &bytes, &original_names)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                if let Some((target, level)) = self
                    .original_jim_alias_target_checked(original)
                    .map_err(|error| {
                        crate::command::completion_from_cmd_error(self, error.into())
                    })?
                {
                    return self.read_original_jim_alias(
                        original,
                        target.value(),
                        level,
                        report,
                        result,
                    );
                }
            }
            if self.uses_native_jim_lookup()
                && let Some(raw) = self.current_jim_variable_cell(original).map_err(|error| {
                    crate::command::completion_from_cmd_error(self, error.into())
                })?
                && let Some(Local::Scalar(value)) =
                    self.var_arena.get(raw).map(crate::vars::VarCell::state)
            {
                return Ok(result.select(value));
            }
            self.missing_original_named_variable(report, &bytes, &original_names)
        })();
        if let Ok(value) = &reached {
            original_names
                .current_for_value(self, value)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        }
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    /// The original global API constructs its bare name in the actual top
    /// frame. A quiet miss proves no variable absence and preserves the result;
    /// a reached dictionary failure retains its actual published diagnostic.
    pub(crate) fn read_original_jim_global_error_code(
        &mut self,
    ) -> Result<Option<Value>, Completion<Value>> {
        if !self.uses_native_jim_lookup() {
            return Err(crate::command::completion_from_cmd_error(
                self,
                ValueError::CommandProtocolUnavailable("original Jim global error-code getter")
                    .into(),
            ));
        }
        let operation = super::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = self.with_event_global_frame(|vm| {
            if vm.frames.len() != 1 || vm.current_ns_id() != ROOT_NS {
                return Err(
                    vm.refuse_host_command("original Jim top variable frame unavailable".into())
                );
            }
            let name = Value::from_native_string_bytes(
                tcl_registry::special_vars::NativeBootstrapVariable::ErrorCode
                    .name()
                    .as_bytes(),
            );
            match vm.read_original_named_variable_using(
                &name,
                OriginalJimVariableReport::Silent,
                OriginalJimVariableResult::Borrowed,
            ) {
                Ok(value) => Ok(Some(value)),
                Err(error) if vm.refused_completion().is_some() => Err(error),
                Err(_) => Ok(None),
            }
        });
        operation
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    pub(crate) fn reset_original_jim_global_error_code(&mut self) -> Result<(), Completion<Value>> {
        self.store_original_jim_global_error_code(Value::from_native_string_bytes(
            b"NONE".as_slice(),
        ))
    }

    pub(crate) fn store_original_jim_global_error_code(
        &mut self,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        if !self.uses_native_jim_lookup() {
            return Err(self.refuse_host_command("original Jim global error-code setter".into()));
        }
        let caller = super::native_name_consumers::OriginalNameConsumerScope::enter(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        caller
            .current_for_value(self, &value)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = self.with_event_global_frame(|vm| {
            if vm.frames.len() != 1 || vm.current_ns_id() != ROOT_NS {
                return Err(
                    vm.refuse_host_command("original Jim top variable frame unavailable".into())
                );
            }
            let name = Value::from_native_string_bytes(
                tcl_registry::special_vars::NativeBootstrapVariable::ErrorCode
                    .name()
                    .as_bytes(),
            );
            vm.store_original_named_variable(&name, value).map(|_| ())
        });
        caller
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    pub(crate) fn store_original_named_variable(
        &mut self,
        original: &Value,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            if self.native_c_variable_name_protocol().is_some() {
                return self.store_original_c_variable(original, value);
            }
            if self.uses_native_jim_lookup() {
                let bytes = original_names
                    .read(self, original)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                if Self::is_original_jim_dictionary_name(original, &bytes) {
                    return self.store_original_jim_dictionary_name(original, value);
                }
                self.install_original_jim_variable(original, &bytes, &original_names)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                if let Some((target, level)) = self
                    .original_jim_alias_target_checked(original)
                    .map_err(|error| {
                        crate::command::completion_from_cmd_error(self, error.into())
                    })?
                {
                    return self.store_original_jim_alias(target.value(), level, value);
                }
                let raw = self.current_jim_variable_cell(original).map_err(|error| {
                    crate::command::completion_from_cmd_error(self, error.into())
                })?;
                if let Some(raw) = raw
                    && self.var_arena.get(raw).is_some_and(|cell| {
                        matches!(cell.state(), Local::Scalar(_) | Local::Undefined)
                    })
                {
                    if !self
                        .var_arena
                        .replace_state(raw, Local::Scalar(value.clone()))
                    {
                        return Err(self
                            .refuse_host_command("Jim original variable receiver expired".into()));
                    }
                    return Ok(value);
                }
            }
            let bytes = original_names
                .read(self, original)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
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
                    let key = if projection.root().qualification()
                        == tcl_syntax::naming::NativeNameQualification::Absolute
                    {
                        let key = Value::from_native_string_bytes(binding.name.as_bytes());
                        key.bind_native_jim_context(&self.native_jim_object_context().map_err(
                            |error| crate::command::completion_from_cmd_error(self, error.into()),
                        )?)
                        .map_err(|error| {
                            crate::command::completion_from_cmd_error(self, error.into())
                        })?;
                        key
                    } else {
                        original.clone()
                    };
                    self.var_table_mut(binding.owner)
                        .expect("actual Jim variable name table")
                        .retain_original_jim_key(&binding.name, key);
                }
                self.install_original_jim_variable(original, &bytes, &original_names)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            }
            Ok(result)
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
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
    fn original_publication_header_survives_replacement_without_lookup_references() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = crate::native_fixture::core(profile);
        let published = Value::from_native_string_bytes(b"P".as_slice());
        let key = vm
            .original_jim_publication_key(
                &published,
                tcl_syntax::naming::NativeNamePurpose::CommandPublication,
            )
            .unwrap()
            .unwrap();
        assert!(key.original.is_same_object(&published));
        let (_, slot, _) = vm.native_procedure_publication(b"P").unwrap();
        vm.register_command_in_slot_with_jim_key(
            slot.clone(),
            Command::Native(Rc::new(ResultCommand(1))),
            Some(key),
        );
        let refs = published.native_object_reference_count();
        let head = Value::from_native_string_bytes(b"P".as_slice());
        vm.native_jim_command_from_original_at(ROOT_NS, &head)
            .unwrap()
            .unwrap();
        assert_eq!(published.native_object_reference_count(), refs);
        let observed = vm.original_jim_command_name_reference(&head).unwrap();
        vm.native_jim_command_from_original_at(ROOT_NS, &head)
            .unwrap()
            .unwrap();
        assert_eq!(published.native_object_reference_count(), refs);
        let replacement = Value::from_native_string_bytes(b"::P".as_slice());
        let key = vm
            .original_jim_publication_key(
                &replacement,
                tcl_syntax::naming::NativeNamePurpose::CommandPublication,
            )
            .unwrap()
            .unwrap();
        vm.register_command_in_slot_with_jim_key(
            slot.clone(),
            Command::Native(Rc::new(ResultCommand(2))),
            Some(key),
        );
        let world = vm.name_world.borrow();
        assert!(
            world.jim_command_original_keys[&slot]
                .original
                .is_same_object(&published)
        );
        assert_eq!(published.native_object_reference_count(), refs);
        assert_eq!(replacement.native_object_reference_count(), 1);
        drop(world);
        assert!(
            observed
                .original()
                .unwrap()
                .unwrap()
                .is_same_object(&published)
        );
    }

    #[test]
    fn original_key_retirement_is_not_reversed_by_a_same_named_publication() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = crate::native_fixture::core(profile);
        let published = Value::from_native_string_bytes(b"P".as_slice());
        let lifetime = published.native_lifetime_lease();
        let key = vm
            .original_jim_publication_key(
                &published,
                tcl_syntax::naming::NativeNamePurpose::CommandPublication,
            )
            .unwrap()
            .unwrap();
        let (_, slot, _) = vm.native_procedure_publication(b"P").unwrap();
        let storage = vm.register_command_in_slot_with_jim_key(
            slot,
            Command::Native(Rc::new(ResultCommand(1))),
            Some(key),
        );
        let head = Value::from_native_string_bytes(b"P".as_slice());
        vm.native_jim_command_from_original_at(ROOT_NS, &head)
            .unwrap()
            .unwrap();
        let observed = vm.original_jim_command_name_reference(&head).unwrap();
        drop(published);
        vm.remove_registered_command(&storage);
        assert!(!lifetime.value().native_object_is_live());
        assert!(observed.original().is_err());
        vm.register_written_command("P", Command::Native(Rc::new(ResultCommand(2))))
            .unwrap();
        let new_head = Value::from_native_string_bytes(b"P".as_slice());
        vm.native_jim_command_from_original_at(ROOT_NS, &new_head)
            .unwrap()
            .unwrap();
        assert!(
            vm.original_jim_command_name_reference(&new_head)
                .unwrap()
                .original()
                .unwrap()
                .is_some()
        );
        assert!(!lifetime.value().native_object_is_live());
        assert!(observed.original().is_err());
    }

    #[test]
    fn host_registration_keys_are_issued_at_actual_jim_selection() {
        let mut vm = Vm::new();
        vm.register_command("original-host", Command::Native(Rc::new(ResultCommand(1))));
        assert!(vm.name_world.borrow().jim_command_original_keys.is_empty());
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        assert!(vm.set_native_engine_profile(profile));
        let head = Value::from_native_string_bytes(b"original-host".as_slice());
        vm.native_jim_command_from_original_at(ROOT_NS, &head)
            .unwrap()
            .unwrap();
        let original = vm
            .original_jim_command_name_reference(&head)
            .unwrap()
            .original()
            .unwrap()
            .unwrap();
        assert_eq!(original.string_bytes().as_ref(), b"original-host");
        let error = Value::from_native_string_bytes(b"error".as_slice());
        vm.native_jim_command_from_original_at(ROOT_NS, &error)
            .unwrap()
            .unwrap();
        let name = vm
            .original_jim_command_name_reference(&error)
            .unwrap()
            .original()
            .unwrap()
            .unwrap();
        assert_eq!(name.string_bytes().as_ref(), b"error");
    }

    #[test]
    fn selected_jim_host_publication_retains_its_original_rooted_cstring() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = crate::native_fixture::core(profile);
        vm.register_command(
            "::original-host",
            Command::Native(Rc::new(ResultCommand(1))),
        );
        let head = Value::from_native_string_bytes(b"original-host".as_slice());
        vm.native_jim_command_from_original_at(ROOT_NS, &head)
            .unwrap()
            .unwrap();
        let observed = vm
            .original_jim_command_name_reference(&head)
            .unwrap()
            .original()
            .unwrap()
            .unwrap();
        assert_eq!(observed.string_bytes().as_ref(), b"::original-host");
        let slot = CommandSlot {
            namespace: ROOT_NS,
            simple: NameBytes::from(b"original-host"),
        };
        assert_eq!(
            vm.name_world.borrow().jim_command_table_keys
                [&vm.command_storage_key_at_slot(&slot).unwrap()]
                .report_bytes(),
            b"::original-host"
        );
        vm.register("::original-builtin", |_, _| ok(Value::empty()));
        let head = Value::from_native_string_bytes(b"original-builtin".as_slice());
        vm.native_jim_command_from_original_at(ROOT_NS, &head)
            .unwrap()
            .unwrap();
        let observed = vm
            .original_jim_command_name_reference(&head)
            .unwrap()
            .original()
            .unwrap()
            .unwrap();
        assert_eq!(observed.string_bytes().as_ref(), b"::original-builtin");
    }

    #[test]
    fn a_previous_c_namespace_slot_is_not_promoted_to_a_jim_flat_key() {
        let mut vm = Vm::new();
        vm.declare_namespace_key("original-ns");
        let key = vm
            .register_command(
                "original-ns::host",
                Command::Native(Rc::new(ResultCommand(1))),
            )
            .into_key()
            .expect("actual original C publication");
        let slot = vm.command_slot(&key).unwrap();
        let token = vm.visible_command_generation(&key).unwrap();
        assert_ne!(slot.namespace, ROOT_NS);
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        assert!(vm.set_native_engine_profile(profile));
        assert_eq!(vm.command_slot(&key), Some(slot.clone()));
        assert_eq!(vm.visible_command_generation(&key), Some(token));
        assert!(
            !vm.name_world
                .borrow()
                .jim_command_original_keys
                .contains_key(&slot)
        );
        let head = Value::from_native_string_bytes(b"error".as_slice());
        vm.native_jim_command_from_original_at(ROOT_NS, &head)
            .unwrap()
            .unwrap();
        assert_eq!(
            vm.original_jim_command_name_reference(&head)
                .unwrap()
                .original()
                .unwrap()
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"error"
        );
    }

    #[test]
    fn unavailable_host_publication_is_distinct_from_a_legal_empty_key() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = crate::native_fixture::core(profile);
        let key = vm
            .register_command("", Command::Native(Rc::new(ResultCommand(1))))
            .into_key()
            .expect("genuine empty CString publication");
        let slot = vm.command_slot(&key).unwrap();
        let token = vm.visible_command_generation(&key).unwrap();
        assert_eq!(slot.simple.as_bytes(), b"");
        let _ = vm.retain_name_query_refusal("withdrawn original host publication");
        assert!(
            vm.register_command("", Command::Native(Rc::new(ResultCommand(2))))
                .into_key()
                .is_none()
        );
        assert_eq!(vm.command_slot(&key), Some(slot));
        assert_eq!(vm.visible_command_generation(&key), Some(token));
    }

    #[test]
    fn selected_error_frames_ignore_an_unselected_retired_key() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = crate::native_fixture::core(profile);
        let stale = Value::from_native_string_bytes(b"unselected".as_slice());
        let observed = OriginalJimCommandName::default();
        *observed.original.borrow_mut() =
            Some(crate::value::JimCommandNameObservation::capture(&stale).unwrap());
        drop(stale);
        assert!(observed.original().is_err());
        vm.jim_errors
            .frames
            .push(super::super::OriginalJimEvaluationFrame {
                evaluation_frame_entered: true,
                procedure_level: 0,
                command: Some(observed),
                is_procedure: false,
                script: None,
                invocation: Value::empty(),
            });
        vm.jim_errors
            .frames
            .push(super::super::OriginalJimEvaluationFrame {
                evaluation_frame_entered: true,
                procedure_level: 0,
                command: None,
                is_procedure: false,
                script: None,
                invocation: Value::list(vec![Value::string("error"), Value::string("fail")]),
            });
        let frames = vm
            .materialize_original_jim_error_frames(0, None)
            .unwrap()
            .unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].procedure.string_bytes().as_ref(), b"");
        assert_eq!(frames[0].invocation.string_bytes().as_ref(), b"error fail");
    }

    #[test]
    fn automatic_error_frames_follow_original_cold_rename_and_selected_key_lifetimes() {
        struct Output(Rc<RefCell<Vec<u8>>>);
        impl std::io::Write for Output {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.borrow_mut().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let cases: &[(&str, &[u8])] = &[
            (
                "proc P {n} {if {$n} {rename P Q; Q 0; error fail}; return ok}\nset c [catch {P 1} result options]\nputs [list $c $result [dict get $options -errorinfo]]\n",
                b"1 fail {Q stdin 1 {error fail} {} stdin 2 {P 1}}\n",
            ),
            (
                "proc P {} {rename eval {}; error fail}\nset c [catch {eval {P}} result options]\nputs [list $c $result [dict get $options -errorinfo]]\n",
                b"1 fail {P stdin 1 {error fail} {} stdin 2 P}\n",
            ),
        ];
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for &(source, expected) in cases {
            let output = Rc::new(RefCell::new(Vec::new()));
            let mut vm = crate::native_fixture::interpreter_with_output(
                profile,
                Box::new(Output(Rc::clone(&output))),
            );
            let completion = vm
                .try_eval_source_at(
                    source,
                    tcl_runtime_api::script_source_location::ScriptSourceLocation {
                        file: "stdin".into(),
                        line: 1,
                    },
                )
                .expect("original actual Jim source entry");
            assert!(
                completion.code.is_ok(),
                "{source}: {}",
                completion.result.to_str()
            );
            assert_eq!(output.borrow().as_slice(), expected, "{source}");
            assert!(vm.refused_completion().is_none(), "{source}");
        }
    }

    #[test]
    fn original_command_replacement_uses_native_node_liveness_without_epoch_drift() {
        let mut vm = actual_jim();
        vm.register_written_command("opaque", Command::Native(Rc::new(ResultCommand(1))))
            .unwrap();
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
        vm.register_written_command("opaque", Command::Native(Rc::new(ResultCommand(2))))
            .unwrap();
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
        let key = vm
            .register_written_command("active", Command::Native(Rc::new(ResultCommand(1))))
            .unwrap();
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

    struct RetirementObserver {
        world: Weak<RefCell<native_name_world::NativeNameWorld>>,
        releases: Rc<RefCell<Vec<(u8, bool)>>>,
        label: u8,
    }
    impl crate::NativeCommand for RetirementObserver {
        fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
            ok(Value::empty())
        }
    }
    impl Drop for RetirementObserver {
        fn drop(&mut self) {
            let released = self
                .world
                .upgrade()
                .is_some_and(|world| world.try_borrow_mut().is_ok());
            self.releases.borrow_mut().push((self.label, released));
        }
    }
    fn observed_command(vm: &Vm, releases: &Rc<RefCell<Vec<(u8, bool)>>>, label: u8) -> Command {
        Command::Native(Rc::new(RetirementObserver {
            world: Rc::downgrade(&vm.name_world),
            releases: Rc::clone(releases),
            label,
        }))
    }
    fn detach_selected_previous(vm: &mut Vm, key: &str) -> Rc<JimCommandNode> {
        let token = vm.visible_command_generation(key).unwrap();
        let top = vm
            .name_world
            .borrow()
            .jim_command_receipts
            .get(&token)
            .and_then(Weak::upgrade)
            .unwrap();
        let previous = top.previous.borrow_mut().take().unwrap();
        // The existing local restoration stage promotes this same node before
        // retiring the top binding; the detached cleanup owns that real role.
        previous.published.set(true);
        previous
            .previous_owners
            .set(previous.previous_owners.get().checked_sub(1).unwrap());
        drop(vm.take_command_unchecked_key(key).unwrap());
        drop(top);
        previous
    }

    #[test]
    fn detached_previous_retirement_releases_the_actual_chain_outside_world_borrows() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Software owner cleanup; no Native pointer or reference-count parity.
        let mut vm = actual_jim();
        let releases = Rc::new(RefCell::new(Vec::new()));
        vm.register_written_command("chain", observed_command(&vm, &releases, b'A'))
            .unwrap();
        vm.register_jim_local_command(
            CommandSlot {
                namespace: ROOT_NS,
                simple: NameBytes::from(b"chain".as_slice()),
            },
            observed_command(&vm, &releases, b'B'),
            None,
        );
        let key = vm.register_jim_local_command(
            CommandSlot {
                namespace: ROOT_NS,
                simple: NameBytes::from(b"chain".as_slice()),
            },
            observed_command(&vm, &releases, b'C'),
            None,
        );
        let previous = detach_selected_previous(&mut vm, &key);
        assert_eq!(releases.borrow().as_slice(), [(b'C', true)]);
        assert_eq!(previous.previous_owners.get(), 0);
        assert!(previous.published.get());
        vm.retire_detached_original_jim_node(previous);
        assert_eq!(
            releases.borrow().as_slice(),
            [(b'C', true), (b'A', true), (b'B', true)]
        );
        assert!(vm.visible_command_generation(&key).is_none());
    }

    #[test]
    fn detached_previous_retirement_preserves_its_real_active_invocation_owner() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Software entered-role cleanup, independent of idle table ownership.
        let mut vm = actual_jim();
        let releases = Rc::new(RefCell::new(Vec::new()));
        let key = vm
            .register_written_command("active", observed_command(&vm, &releases, b'A'))
            .unwrap();
        let token = vm.visible_command_generation(&key).unwrap();
        let active = vm.retain_active_jim_command(token).unwrap();
        let key = vm.register_jim_local_command(
            CommandSlot {
                namespace: ROOT_NS,
                simple: NameBytes::from(b"active".as_slice()),
            },
            observed_command(&vm, &releases, b'B'),
            None,
        );
        let previous = detach_selected_previous(&mut vm, &key);
        let original = Rc::downgrade(&previous);
        vm.retire_detached_original_jim_node(previous);
        assert_eq!(releases.borrow().as_slice(), [(b'B', true)]);
        let original = original.upgrade().unwrap();
        assert!(!original.published.get());
        assert_eq!(original.active.get(), 1);
        drop(original);
        drop(active);
        assert_eq!(releases.borrow().as_slice(), [(b'B', true), (b'A', true)]);
    }

    #[test]
    fn detached_previous_retirement_uses_its_original_world_when_another_vm_is_current() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Explicit original world custody grants no authority to another interpreter.
        let mut original_vm = actual_jim();
        let original_world = Rc::clone(&original_vm.name_world);
        let releases = Rc::new(RefCell::new(Vec::new()));
        original_vm
            .register_written_command("original", observed_command(&original_vm, &releases, b'A'))
            .unwrap();
        let key = original_vm.register_jim_local_command(
            CommandSlot {
                namespace: ROOT_NS,
                simple: NameBytes::from(b"original".as_slice()),
            },
            observed_command(&original_vm, &releases, b'B'),
            None,
        );
        let previous = detach_selected_previous(&mut original_vm, &key);
        let token = previous.token;
        let other_vm = actual_jim();
        let other_epoch = other_vm.name_world.borrow().jim_procedure_epoch;
        let other_retired = other_vm.name_world.borrow().jim_retired_tokens.clone();
        retire_detached_original_jim_node_in(&original_world, previous);
        assert!(original_world.borrow().jim_retired_tokens.contains(&token));
        assert_eq!(releases.borrow().as_slice(), [(b'B', true), (b'A', true)]);
        assert_eq!(
            other_vm.name_world.borrow().jim_procedure_epoch,
            other_epoch
        );
        assert_eq!(
            other_vm.name_world.borrow().jim_retired_tokens,
            other_retired
        );
    }
}
