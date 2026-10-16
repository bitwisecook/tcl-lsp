// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reached name queries retain their selected command and namespace identities.

use super::native_operation_currency::NativeOperationScope;
use super::{Command, CommandId, NsId, Rc, Value, Vm};
use crate::command::ProcDef;
use tcl_cmd_core::CmdError;
use tcl_runtime_api::{ProcInfo, ProcParam};
use tcl_syntax::value::ValueError;

pub(super) struct QueryNamespace {
    id: NsId,
    dying: bool,
    dead: bool,
}
/// Evidence of this selected operation retaining the already entered birth.
/// It never changes the original query's lifecycle snapshot.
pub(super) struct SelectedNamespaceRetirement {
    namespace: NsId,
}

pub(super) struct PendingNamespaceRetirement {
    namespace: NsId,
    selected: NsId,
}

impl PendingNamespaceRetirement {
    pub(super) fn complete(
        self,
        vm: &Vm,
        original: &QueryNamespace,
        retired: NsId,
    ) -> Option<SelectedNamespaceRetirement> {
        if self.selected != retired {
            return None;
        }
        let receipt = SelectedNamespaceRetirement {
            namespace: self.namespace,
        };
        receipt.current(vm, original).then_some(receipt)
    }
}

impl SelectedNamespaceRetirement {
    pub(super) fn current(&self, vm: &Vm, original: &QueryNamespace) -> bool {
        if original.id != self.namespace || original.dying || original.dead {
            return false;
        }
        let world = vm.name_world.borrow();
        world.is_live()
            && !world.dying_namespaces.contains(&self.namespace)
            && world.dead_namespaces.contains(&self.namespace)
            && world
                .ns_deferral
                .counts
                .get(&self.namespace)
                .is_some_and(|count| *count > 0)
            && world.ns_deferral.owners.get(&self.namespace) == Some(&self.namespace)
            && world
                .ns_deferral
                .retained
                .get(&self.namespace)
                .is_some_and(|record| {
                    record.root == self.namespace
                        && record
                            .subtree
                            .values()
                            .any(|member| *member == self.namespace)
                })
    }
}

impl QueryNamespace {
    pub(super) fn capture(vm: &Vm, id: NsId) -> Result<Self, CmdError> {
        if !vm.native_namespace_token_has_owner(id) {
            return Err(
                ValueError::CommandProtocolUnavailable("original query namespace owner").into(),
            );
        }
        let world = vm.name_world.borrow();
        Ok(Self {
            id,
            dying: world.dying_namespaces.contains(&id),
            dead: world.dead_namespaces.contains(&id),
        })
    }
    pub(super) fn prepare_selected_retirement(
        &self,
        vm: &Vm,
        selected: NsId,
    ) -> Option<PendingNamespaceRetirement> {
        if self.dying || self.dead || !self.current(vm) {
            return None;
        }
        let world = vm.name_world.borrow();
        let mut member = self.id;
        for _ in 0..world.ns_parents.len() {
            if member == selected {
                return Some(PendingNamespaceRetirement {
                    namespace: self.id,
                    selected,
                });
            }
            member = world.ns_parents.get(member.0 as usize).copied().flatten()?;
        }
        None
    }

    pub(super) fn current(&self, vm: &Vm) -> bool {
        if !vm.native_namespace_token_has_owner(self.id) {
            return false;
        }
        let world = vm.name_world.borrow();
        world.dying_namespaces.contains(&self.id) == self.dying
            && world.dead_namespaces.contains(&self.id) == self.dead
    }
}

struct ProcedureQuery {
    generation: u64,
    namespace: QueryNamespace,
    declaration: Rc<ProcDef>,
}
impl ProcedureQuery {
    fn capture(vm: &Vm, name: &[u8]) -> Result<Option<Self>, CmdError> {
        let Some(key) = vm.resolve_command_from_token_bytes(vm.current_ns_id(), name, true) else {
            return Ok(None);
        };
        let Some(Command::Proc(procedure)) = vm.visible_command_at_key(&key) else {
            return Ok(None);
        };
        let generation =
            vm.visible_command_generation(&key)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original procedure query generation",
                ))?;
        let declaration = procedure.declaration();
        let namespace = QueryNamespace::capture(
            vm,
            declaration
                .actual_namespace_context()
                .context()
                .resolved_token()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original procedure query namespace coordinate",
                ))?,
        )?;
        Ok(Some(Self {
            generation,
            namespace,
            declaration,
        }))
    }
    fn check(&self, vm: &mut Vm, scope: &NativeOperationScope) -> Result<(), CmdError> {
        scope.check(vm)?;
        let same = vm.command_token_at_generation(self.generation)
            .and_then(|token| vm.command_at_sidecar_key(&token.key))
            .is_some_and(|command| matches!(command, Command::Proc(procedure) if Rc::ptr_eq(&procedure.declaration(), &self.declaration)));
        if !same
            || self.declaration.native_reference_count() == 0
            || self
                .declaration
                .actual_namespace_context()
                .context()
                .resolved_token()
                != Some(self.namespace.id)
            || !self.namespace.current(vm)
        {
            return Err(vm.retain_name_query_refusal("original procedure query owner retired"));
        }
        Ok(())
    }
    fn bytes(
        &self,
        vm: &mut Vm,
        scope: &NativeOperationScope,
        value: &Value,
    ) -> Result<Vec<u8>, CmdError> {
        self.check(vm, scope)?;
        let reached = vm.native_name_operand_bytes_checked(value);
        self.check(vm, scope)?;
        reached.map(|bytes| bytes.to_vec())
    }
}

struct OriginalJimBodyResult {
    caller: super::native_name_consumers::OriginalNameConsumerScope,
    original: crate::NativeObjectLifetimeLease,
    node: std::rc::Weak<super::native_jim_lookup::JimCommandNode>,
    token: u64,
    declaration: std::rc::Weak<ProcDef>,
    namespace: QueryNamespace,
    body: crate::NativeObjectLifetimeLease,
    context: std::rc::Weak<crate::value::NativeJimObjectContext>,
}

fn retain_body_result_error(vm: &mut Vm, error: CmdError) -> CmdError {
    if let Some(cause) = error.native_execution_refusal() {
        return CmdError::from_execution_refusal(
            vm.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(cause.clone())),
        );
    }
    error
}

impl OriginalJimBodyResult {
    fn current(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.caller.current_for_value(vm, self.original.value())
    }
    fn result_context(
        &self,
        vm: &mut Vm,
    ) -> Result<Rc<crate::value::NativeJimObjectContext>, CmdError> {
        self.current(vm)?;
        let context = self
            .context
            .upgrade()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original body-result Jim context retired",
            ))?;
        let actual = vm.native_jim_object_context()?;
        if !Rc::ptr_eq(&context, &actual) {
            return Err(vm.retain_name_query_refusal("foreign original body-result consumer"));
        }
        self.body.value().check_native_jim_context(&context)?;
        self.current(vm)?;
        Ok(context)
    }
}

impl tcl_runtime_api::OriginalProcedureBodyValue<Value, CmdError, Vm> for OriginalJimBodyResult {
    fn checked_value(&self, vm: &mut Vm) -> Result<Value, CmdError> {
        let reached = (|| {
            let _context = self.result_context(vm)?;
            let declaration =
                self.declaration
                    .upgrade()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original body-result declaration retired",
                    ))?;
            let node = self
                .node
                .upgrade()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original body-result command retired",
                ))?;
            let selected = {
                let command = node.command.borrow();
                matches!(&*command, Command::Proc(procedure) if Rc::ptr_eq(&procedure.declaration(), &declaration))
            };
            if node.token != self.token
                || !node.is_live()
                || !selected
                || declaration.native_reference_count() == 0
                || declaration
                    .actual_namespace_context()
                    .context()
                    .resolved_token()
                    != Some(self.namespace.id)
                || !self.namespace.current(vm)
                || !declaration.body_src.is_same_object(self.body.value())
            {
                return Err(
                    vm.retain_name_query_refusal("original body-result selected owner changed")
                );
            }
            self.caller.current_for_value(vm, self.body.value())?;
            drop(node);
            drop(declaration);
            self.current(vm)?;
            Ok(self.body.value().native_lifetime_lease().into_value())
        })();
        reached.map_err(|error| retain_body_result_error(vm, error))
    }

    fn finish(self: Box<Self>, vm: &mut Vm) -> Result<(), CmdError> {
        let reached = (|| {
            let context = self.result_context(vm)?;
            // A real old-result callback may retire the selected declaration.
            // The actual result role now owns the body; only caller currency
            // and this published original header remain required.
            self.caller.current_for_value(vm, self.body.value())?;
            if !context.with_result_object(|result| result.is_same_object(self.body.value())) {
                return Err(vm.retain_name_query_refusal("original body-result publication header"));
            }
            self.current(vm)
        })();
        reached.map_err(|error| retain_body_result_error(vm, error))
    }
}

impl Vm {
    pub(crate) fn original_proc_body_value_query(
        &mut self,
        original: &Value,
    ) -> Result<tcl_runtime_api::ProcBodyValueQuery<Value, CmdError, Vm>, CmdError> {
        let caller = super::native_name_consumers::OriginalNameConsumerScope::enter(self)?;
        let reached = (|| {
            let dialect = self.actual_native_invocation_dialect();
            if dialect.native_jim_lookup_protocol().is_none()
                || dialect.native_jim_info_protocol().is_none()
            {
                return Err(
                    self.retain_name_query_refusal("original Jim procedure body query issuer")
                );
            }
            caller.current_for_value(self, original)?;
            let context = self.native_jim_object_context()?;
            original.check_native_jim_context(&context)?;
            let namespace = self.current_namespace_execution_context();
            let selected =
                self.native_jim_command_from_original_in_context(&namespace, original)?;
            caller.current_for_value(self, original)?;
            let Some((_, Command::Proc(procedure))) = selected else {
                let name = caller.read(self, original)?.to_vec();
                caller.check(self)?;
                return Ok(tcl_runtime_api::ProcBodyValueQuery::NotProcedure(name));
            };
            let declaration = procedure.declaration();
            let node = self.selected_original_jim_node(original).ok_or(
                ValueError::CommandProtocolUnavailable("original body-result selected Jim node"),
            )?;
            let same = matches!(&*node.command.borrow(), Command::Proc(selected)
                if Rc::ptr_eq(&selected.declaration(), &declaration));
            if !same || !node.is_live() || declaration.native_reference_count() == 0 {
                return Err(
                    self.retain_name_query_refusal("original body-result selected procedure")
                );
            }
            let namespace = QueryNamespace::capture(
                self,
                declaration
                    .actual_namespace_context()
                    .context()
                    .resolved_token()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original body-result namespace owner",
                    ))?,
            )?;
            let body = declaration.body_src.native_lifetime_lease();
            let token = node.token;
            let selected_node = Rc::downgrade(&node);
            let selected_declaration = Rc::downgrade(&declaration);
            drop(node);
            drop(declaration);
            drop(procedure);
            caller.check(self)?;
            let result = OriginalJimBodyResult {
                caller,
                original: original.native_lifetime_lease(),
                node: selected_node,
                token,
                declaration: selected_declaration,
                namespace,
                body,
                context: Rc::downgrade(&context),
            };
            let value = tcl_runtime_api::OriginalProcedureBodyValue::checked_value(&result, self)?;
            drop(value);
            Ok(tcl_runtime_api::ProcBodyValueQuery::Body(Box::new(result)))
        })();
        reached.map_err(|error| retain_body_result_error(self, error))
    }
}

impl Vm {
    pub(super) fn retain_name_query_refusal(&mut self, reason: &'static str) -> CmdError {
        let error = CmdError::from(ValueError::CommandProtocolUnavailable(reason));
        if let Some(cause) = error.native_execution_refusal() {
            self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(cause.clone()));
        }
        error
    }
    pub(crate) fn original_proc_body_query(
        &mut self,
        name: &[u8],
    ) -> Result<Option<Vec<u8>>, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        let query = ProcedureQuery::capture(self, name)?;
        let reached: Result<Option<Vec<u8>>, CmdError> = query
            .as_ref()
            .map(|query| query.bytes(self, &scope, &query.declaration.body_src))
            .transpose();
        if let Err(error) = &reached {
            if let Some(cause) = error.native_execution_refusal() {
                self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(
                    cause.clone(),
                ));
            }
        }
        drop(query);
        scope.check(self)?;
        reached
    }
    pub(crate) fn original_proc_info_query(
        &mut self,
        name: &[u8],
    ) -> Result<Option<ProcInfo>, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        let query = ProcedureQuery::capture(self, name)?;
        let reached: Result<Option<ProcInfo>, CmdError> = (|| {
            let Some(query) = query.as_ref() else {
                return Ok(None);
            };
            let mut params = Vec::with_capacity(query.declaration.params.len());
            for parameter in &query.declaration.params {
                let default = parameter
                    .default
                    .as_ref()
                    .map(|value| query.bytes(self, &scope, value))
                    .transpose()?;
                params.push(ProcParam {
                    name: parameter.name.as_bytes().to_vec(),
                    default,
                });
            }
            let body = query.bytes(self, &scope, &query.declaration.body_src)?;
            Ok(Some(ProcInfo { body, params }))
        })();
        if let Err(error) = &reached {
            if let Some(cause) = error.native_execution_refusal() {
                self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(
                    cause.clone(),
                ));
            }
        }
        drop(query);
        scope.check(self)?;
        reached
    }

    pub(crate) fn original_alias_bytes_query(
        &mut self,
        id: CommandId,
    ) -> Result<Option<Vec<Vec<u8>>>, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        let key = self
            .command_sidecar_key(id.0)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "alias prefix command token",
            ))?;
        let token =
            self.command_token_identity(&key)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original alias query generation",
                ))?;
        let namespace_id = match &key {
            super::CommandSidecarKey::Visible(name) => {
                self.command_slot(name)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original alias query slot",
                    ))?
                    .namespace
            }
            super::CommandSidecarKey::Hidden(_) => super::ROOT_NS,
        };
        let namespace = QueryNamespace::capture(self, namespace_id)?;
        let check = |vm: &mut Vm| -> Result<(), CmdError> {
            scope.check(vm)?;
            let current = vm.command_token_at_generation(token.generation);
            if current.as_ref() != Some(&token) || !namespace.current(vm) {
                return Err(vm.retain_name_query_refusal("original alias query owner retired"));
            }
            Ok(())
        };
        let command = self.command_at_sidecar_key(&key);
        enum Words {
            Direct(Vec<crate::NativeObjectLifetimeLease>),
            List(crate::NativeObjectLifetimeLease),
        }
        let originals = match command.as_ref() {
            Some(Command::Alias { words, .. }) => Some(Words::Direct(
                words.iter().map(Value::native_lifetime_lease).collect(),
            )),
            Some(Command::CallerAlias(original)) => {
                Some(Words::List(original.native_lifetime_lease()))
            }
            _ => None,
        };
        drop(command);
        let reached: Result<Option<Vec<Vec<u8>>>, CmdError> = (|| {
            check(self)?;
            let mut result = Vec::new();
            match originals.as_ref() {
                None => return Ok(None),
                Some(Words::Direct(words)) => {
                    for word in words {
                        check(self)?;
                        let reached = self.native_name_operand_bytes_checked(word.value());
                        check(self)?;
                        result.push(reached?.to_vec());
                    }
                }
                Some(Words::List(original)) => {
                    let backing = {
                        let ops = super::native_numeric_ops::OriginalNumericOps::in_original_scope(
                            self, &scope,
                        )?;
                        let reached = ops.list_with_original_check(
                            original.value(),
                            tcl_syntax::native_string::NativeStringProtocol::Jim084,
                            || ops.with_vm_for_value(original.value(), |vm| check(vm)),
                        );
                        ops.current_for_value(original.value())?;
                        reached
                    };
                    check(self)?;
                    original.value().check_native_header()?;
                    let backing = backing?;
                    for word in &*backing {
                        check(self)?;
                        backing.check_generation()?;
                        original.value().check_native_header()?;
                        let reached = self.native_name_operand_bytes_checked(word);
                        check(self)?;
                        backing.check_generation()?;
                        original.value().check_native_header()?;
                        result.push(reached?.to_vec());
                    }
                }
            }
            Ok(Some(result))
        })();
        if let Err(error) = &reached {
            if let Some(cause) = error.native_execution_refusal() {
                self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(
                    cause.clone(),
                ));
            }
        }
        drop(originals);
        scope.check(self)?;
        reached
    }
}

#[cfg(test)]
mod tests;
