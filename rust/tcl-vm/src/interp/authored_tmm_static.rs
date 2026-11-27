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

//! Authored static publication over enrolled, live worker namespace cells.

use super::{
    Completion, GuardDomain, HashSet, InterpId, Local, NameBytes, Namespaces, NsId, ROOT_INTERP,
    ROOT_NS, Value, VarBinding, VarId, VarTable, VarTableOwner, Vm, err, ok,
};
use tcl_runtime_api::authored_tmm::{
    AuthoredTmmStaticPolicy, AuthoredTmmWorkerTopology, TmmStaticExecutionContext,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StaticAddress {
    interpreter: InterpId,
    namespace: NsId,
    name: NameBytes,
    element: Option<NameBytes>,
}

#[derive(Clone)]
enum Mutation {
    Store(Value),
    EnsureArray,
    Unset,
}

struct Publication {
    sequence: u64,
    name: NameBytes,
    element: Option<NameBytes>,
    mutation: Mutation,
}

#[derive(Clone, Copy)]
struct Worker {
    interpreter: InterpId,
    namespace: NsId,
}

#[derive(Default)]
pub(super) struct AuthoredTmmStaticDomain {
    pub(super) policy: Option<AuthoredTmmStaticPolicy>,
    workers: Vec<Worker>,
    topology: AuthoredTmmWorkerTopology,
    // These are real authored storage roles, independently of worker cells.
    // Superseded values are released rather than retained as history.
    journal: Vec<Publication>,
    sequence: u64,
    replay: Option<StaticAddress>,
    recipient_depth: usize,
    configuration: Option<StaticAddress>,
}

impl Vm {
    /// Install the explicit logical publication contract. This does not select
    /// an F5 appliance, source grammar, numeric policy or native object provider.
    pub fn install_irules_static_simulation(&mut self) {
        self.authored_tmm_static.policy = Some(AuthoredTmmStaticPolicy::RuleInitPublication);
        register_provider(self);
    }

    fn static_namespace_context(
        &self,
        worker: Worker,
    ) -> Option<tcl_runtime_api::native_compilation::NativeNamespaceContext> {
        if !self.static_worker_is_live(worker) {
            return None;
        }
        let state = self.st_of(worker.interpreter)?;
        let path = state
            .name_world
            .borrow()
            .ns_intern
            .iter()
            .find_map(|(path, token)| (*token == worker.namespace).then(|| path.clone()))?;
        Some(
            tcl_runtime_api::native_compilation::NativeNamespaceContext {
                interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                    owner: self.owner_nonce,
                    interpreter: u64::try_from(worker.interpreter.0)
                        .expect("actual interpreter slot"),
                },
                token: u64::from(worker.namespace.0),
                path,
            },
        )
    }

    pub(super) fn authored_static_compilation_context(
        &self,
    ) -> Option<tcl_runtime_api::authored_tmm::AuthoredTmmStaticCompilationContext> {
        use tcl_runtime_api::authored_tmm::{
            AuthoredTmmStaticCompilationContext, AuthoredTmmStaticRecipient,
        };
        use tcl_runtime_api::native_compilation::NativeVariableObserverPresence as Presence;
        let policy = self.authored_tmm_static.policy?;
        let workers = self.static_workers();
        let worker = workers
            .iter()
            .copied()
            .find(|worker| worker.interpreter == self.cur)?;
        let namespace = self.static_namespace_context(worker)?;
        let context = self.static_publication_context();
        let mut recipients = Vec::new();
        let mut observed = false;
        for recipient in workers
            .into_iter()
            .filter(|recipient| recipient.interpreter != self.cur)
        {
            let selected = self.static_namespace_context(recipient)?;
            let state = self.st_of(recipient.interpreter)?;
            recipients.push(AuthoredTmmStaticRecipient {
                namespace: selected,
                // Every variable observer change in the recipient advances
                // this domain; a poisoned domain supplies no receipt.
                observer_epoch: state
                    .guards
                    .borrow()
                    .domain_epoch(GuardDomain::VariableTrace)?,
            });
            // Table attachment, not a name prefix or an alias spelling, selects
            // the actual callback roots in the enrolled recipient namespace.
            if let Some(table) = state.ns_vars.get(&recipient.namespace) {
                observed |= state
                    .variable_observers
                    .script_traces
                    .iter()
                    .any(|(id, traces)| {
                        let root = state
                            .var_arena
                            .element_parent(*id)
                            .map_or(*id, |(root, _)| root);
                        state.var_arena.element_is_attached(*id)
                            && state
                                .var_arena
                                .get(root)
                                .and_then(crate::vars::VarCell::namespace_owner)
                                == Some(recipient.namespace)
                            && table.values().any(|raw| *raw == root)
                            && traces.iter().any(|trace| {
                                trace.ops.iter().any(|op| op == "write" || op == "unset")
                            })
                    });
            }
        }
        let outward_observers = match context {
            Some(TmmStaticExecutionContext::ExecutingWorker) => Presence::Absent,
            Some(TmmStaticExecutionContext::InitialisationBroadcast) if !observed => {
                Presence::Absent
            }
            Some(TmmStaticExecutionContext::InitialisationBroadcast) => Presence::Present,
            None => Presence::Unknown,
        };
        Some(AuthoredTmmStaticCompilationContext {
            policy,
            context,
            namespace,
            recipients,
            outward_observers,
        })
    }

    fn static_publication_context(&self) -> Option<TmmStaticExecutionContext> {
        if let Some(context) = self.timer_contexts.iter().rev().find(|context| {
            context
                .scope
                .as_ref()
                .is_none_or(|scope| scope.strong_count() != 0)
        }) {
            return context.static_context;
        }
        (self.authored_tmm_static.recipient_depth != 0)
            .then_some(TmmStaticExecutionContext::InitialisationBroadcast)
    }

    fn static_worker_is_live(&self, worker: Worker) -> bool {
        if !self.interp_alive(worker.interpreter) {
            return false;
        }
        let Some(state) = self.st_of(worker.interpreter) else {
            return false;
        };
        let world = state.name_world.borrow();
        !world.dead_namespaces.contains(&worker.namespace)
            && !world.dying_namespaces.contains(&worker.namespace)
    }

    fn static_workers(&self) -> Vec<Worker> {
        let live: Vec<_> = self
            .authored_tmm_static
            .workers
            .iter()
            .copied()
            .filter(|worker| self.static_worker_is_live(*worker))
            .collect();
        live.into_iter()
            .filter(|worker| match self.authored_tmm_static.topology {
                AuthoredTmmWorkerTopology::RootInterpreter => worker.interpreter == ROOT_INTERP,
                AuthoredTmmWorkerTopology::EnrolledChildren => worker.interpreter != ROOT_INTERP,
            })
            .collect()
    }

    /// Attest the attached namespace root after all name/cell links have been
    /// followed. A stale element alias cannot name a recreated array.
    fn static_address(&self, id: VarId) -> Option<StaticAddress> {
        self.authored_tmm_static.policy?;
        let (root, element) = match self.var_arena.element_parent(id) {
            Some((root, element)) => {
                if !self.var_arena.element_is_attached(id) {
                    return None;
                }
                (root, Some(element.to_owned()))
            }
            None => (id, None),
        };
        let namespace = self.var_arena.get(root)?.namespace_owner()?;
        let worker = Worker {
            interpreter: self.cur,
            namespace,
        };
        if !self.static_worker_is_live(worker)
            || !self
                .authored_tmm_static
                .workers
                .iter()
                .any(|member| member.interpreter == self.cur && member.namespace == namespace)
        {
            return None;
        }
        let table = self.var_table(VarTableOwner::Namespace(namespace))?;
        let name = table
            .iter()
            .find_map(|(name, raw)| (*raw == root).then(|| name.clone()))?;
        Some(StaticAddress {
            interpreter: self.cur,
            namespace,
            name,
            element,
        })
    }

    pub(super) fn publish_authored_static_write(
        &mut self,
        id: VarId,
    ) -> Result<(), Completion<Value>> {
        let Some(address) = self.static_address(id) else {
            return Ok(());
        };
        let Some(Local::Scalar(value)) = self.var_arena.get(id).map(crate::vars::VarCell::state)
        else {
            return Ok(());
        };
        // A journal entry is an explicit owning role; no materialisation or
        // serialisation supplies this original value.
        let value = value.clone().into_native_reference();
        self.publish_authored_static(&address, &Mutation::Store(value))
    }

    pub(super) fn publish_authored_static_array(
        &mut self,
        id: VarId,
    ) -> Result<(), Completion<Value>> {
        let Some(address) = self.static_address(id) else {
            return Ok(());
        };
        self.publish_authored_static(&address, &Mutation::EnsureArray)
    }

    pub(super) fn authored_static_unset_address(&self, id: VarId) -> Option<StaticAddress> {
        self.static_address(id)
    }

    pub(super) fn publish_authored_static_unset(
        &mut self,
        address: Option<StaticAddress>,
    ) -> Result<(), Completion<Value>> {
        match address {
            Some(address) => self.publish_authored_static(&address, &Mutation::Unset),
            None => Ok(()),
        }
    }

    fn publish_authored_static(
        &mut self,
        address: &StaticAddress,
        mutation: &Mutation,
    ) -> Result<(), Completion<Value>> {
        if self.authored_tmm_static.replay.as_ref() == Some(address) {
            // Only this replicated primitive is suppressed. Its trace callbacks
            // run with ordinary publication semantics restored.
            self.authored_tmm_static.replay = None;
            return Ok(());
        }
        let configured = self.authored_tmm_static.configuration.as_ref() == Some(address);
        if configured {
            self.authored_tmm_static.configuration = None;
        }
        let context = if configured {
            Some(TmmStaticExecutionContext::InitialisationBroadcast)
        } else {
            self.static_publication_context()
        };
        let Some(context) = context else {
            return Ok(());
        };
        if !self
            .authored_tmm_static
            .policy
            .is_some_and(|policy| policy.publishes(context))
        {
            return Ok(());
        }
        let workers = self.static_workers();
        if !configured
            && !workers.iter().any(|worker| {
                worker.interpreter == address.interpreter && worker.namespace == address.namespace
            })
        {
            return Ok(());
        }
        let domain = &mut self.authored_tmm_static;
        if address.element.is_some() && matches!(mutation, Mutation::Store(_)) {
            // The reached attached parent is an array even if its last member
            // is later removed. Late enrollment must preserve that empty root.
            let needs_root = !domain.journal.iter().any(|entry| {
                entry.name == address.name
                    && entry.element.is_none()
                    && matches!(entry.mutation, Mutation::EnsureArray)
            });
            if needs_root {
                domain
                    .journal
                    .retain(|entry| entry.name != address.name || entry.element.is_some());
                domain.sequence += 1;
                domain.journal.push(Publication {
                    sequence: domain.sequence,
                    name: address.name.clone(),
                    element: None,
                    mutation: Mutation::EnsureArray,
                });
            }
        }
        domain.sequence += 1;
        domain.journal.retain(|entry| {
            entry.name != address.name
                || ((address.element.is_some() || matches!(mutation, Mutation::EnsureArray))
                    && entry.element != address.element)
        });
        domain.journal.push(Publication {
            sequence: domain.sequence,
            name: address.name.clone(),
            element: address.element.clone(),
            mutation: mutation.clone(),
        });
        for worker in workers {
            if worker.interpreter == address.interpreter {
                continue;
            }
            self.apply_static_publication(
                worker,
                &address.name,
                address.element.as_ref(),
                mutation,
                !configured,
            )?;
        }
        Ok(())
    }

    fn apply_static_publication(
        &mut self,
        worker: Worker,
        name: &NameBytes,
        element: Option<&NameBytes>,
        mutation: &Mutation,
        callbacks_broadcast: bool,
    ) -> Result<(), Completion<Value>> {
        if !self.static_worker_is_live(worker) {
            return Ok(());
        }
        self.in_interp(worker.interpreter, |vm| {
            if !vm.namespace_is_live(worker.namespace) {
                return Ok(());
            }
            let address = StaticAddress {
                interpreter: vm.cur,
                namespace: worker.namespace,
                name: name.clone(),
                element: element.cloned(),
            };
            let previous = vm.authored_tmm_static.replay.replace(address);
            if callbacks_broadcast {
                vm.authored_tmm_static.recipient_depth += 1;
            }
            let result = vm.apply_static_primitive(worker.namespace, name, element, mutation);
            if callbacks_broadcast {
                vm.authored_tmm_static.recipient_depth -= 1;
            }
            vm.authored_tmm_static.replay = previous;
            result
        })
    }

    fn apply_static_primitive(
        &mut self,
        namespace: NsId,
        name: &NameBytes,
        element: Option<&NameBytes>,
        mutation: &Mutation,
    ) -> Result<(), Completion<Value>> {
        let binding = VarBinding {
            owner: VarTableOwner::Namespace(namespace),
            name: name.clone(),
        };
        let resolved = self
            .resolve_binding_var(binding, element.cloned(), &mut HashSet::new())
            .ok_or_else(|| {
                self.refuse_host_command("static recipient binding is unavailable".into())
            })?;
        // A recipient may have installed an alias. Follow it before creation or
        // mutation, and require its actual root to remain in this live domain.
        // An ordinary global reached through a static spelling is still local.
        if let Some(id) = resolved.base_id {
            let root = self
                .var_arena
                .element_parent(id)
                .map_or(id, |(root, _)| root);
            if self
                .var_arena
                .get(root)
                .and_then(crate::vars::VarCell::namespace_owner)
                != Some(namespace)
            {
                return Ok(());
            }
            if self.static_address(id).is_none() {
                return Err(self
                    .refuse_host_command("static recipient alias targets a detached cell".into()));
            }
        } else if resolved.binding.owner != VarTableOwner::Namespace(namespace) {
            return Ok(());
        }
        let reported = tcl_syntax::naming::rooted_member_key_bytes(
            &Namespaces::name_bytes(self, namespace),
            name.as_bytes(),
        );
        match mutation {
            Mutation::Store(value) => {
                let captured = self.capture_selected_update(
                    &reported,
                    element.map(NameBytes::as_bytes),
                    &resolved,
                )?;
                self.select_static_primitive_address(
                    namespace,
                    captured.cell.id.expect("captured cell"),
                )?;
                self.store_captured_update_with_original(
                    &reported,
                    element.map(NameBytes::as_bytes),
                    &captured,
                    value.clone().into_native_reference(),
                    None,
                )?;
            }
            Mutation::EnsureArray => {
                let id = resolved
                    .id
                    .or_else(|| self.ensure_target_var_at_binding(&resolved.binding, None))
                    .ok_or_else(|| {
                        self.refuse_host_command("static array allocation is unavailable".into())
                    })?;
                self.select_static_primitive_address(namespace, id)?;
                match self.var_arena.get(id).map(crate::vars::VarCell::state) {
                    Some(Local::Array(_)) => {}
                    Some(Local::Undefined) => {
                        self.var_arena
                            .replace_state(id, Local::Array(VarTable::new()));
                    }
                    _ => {
                        return Err(self.variable_access_error_bytes(
                            "array set",
                            &reported,
                            "variable isn't array",
                        ));
                    }
                }
                self.publish_authored_static_array(id)?;
            }
            Mutation::Unset => {
                if let Some(id) = resolved.id {
                    self.select_static_primitive_address(namespace, id)?;
                }
                self.unset_resolved_checked(&reported, &resolved)?;
            }
        }
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        Ok(())
    }

    fn select_static_primitive_address(
        &mut self,
        namespace: NsId,
        id: VarId,
    ) -> Result<(), Completion<Value>> {
        let address = self
            .static_address(id)
            .filter(|address| address.namespace == namespace)
            .ok_or_else(|| {
                self.refuse_host_command("static recipient cell is detached or unavailable".into())
            })?;
        // Suppression belongs to the resolved primitive, not the source name.
        // The publication hook consumes it before recipient callbacks begin.
        if self.authored_tmm_static.replay.is_some() {
            self.authored_tmm_static.replay = Some(address.clone());
        }
        if self.authored_tmm_static.configuration.is_some() {
            self.authored_tmm_static.configuration = Some(address);
        }
        Ok(())
    }

    pub(super) fn retire_authored_static_worker(&mut self, interpreter: InterpId) {
        self.authored_tmm_static
            .workers
            .retain(|worker| worker.interpreter != interpreter);
    }

    fn enroll_static_worker(&mut self) -> Result<(), Completion<Value>> {
        // Cross-interpreter original object sharing is selected explicitly.
        // Jim original objects carry interpreter contexts and are not donated.
        if !self
            .actual_native_string_protocol()
            .is_some_and(|protocol| {
                matches!(
                    protocol,
                    tcl_syntax::native_string::NativeStringProtocol::C(_)
                )
            })
        {
            return Err(self.refuse_host_command(
                "TMM static original-object sharing requires an actual C provider".into(),
            ));
        }
        let namespace = self
            .namespace_id_from_written(ROOT_NS, "::static")
            .filter(|namespace| self.namespace_is_live(*namespace))
            .ok_or_else(|| {
                self.refuse_host_command(
                    "static enrollment requires a live original namespace".into(),
                )
            })?;
        let worker = Worker {
            interpreter: self.cur,
            namespace,
        };
        if self
            .authored_tmm_static
            .workers
            .iter()
            .any(|old| old.interpreter == worker.interpreter && old.namespace == namespace)
        {
            return Ok(());
        }
        self.authored_tmm_static
            .workers
            .retain(|old| old.interpreter != worker.interpreter);
        self.authored_tmm_static.workers.push(worker);
        let mut replay: Vec<_> = self
            .authored_tmm_static
            .journal
            .iter()
            .map(|entry| {
                (
                    entry.sequence,
                    entry.name.clone(),
                    entry.element.clone(),
                    entry.mutation.clone(),
                )
            })
            .collect();
        replay.sort_by_key(|entry| entry.0);
        for (_, name, element, mutation) in replay {
            self.apply_static_publication(worker, &name, element.as_ref(), &mutation, false)?;
        }
        Ok(())
    }
}

pub(super) fn register_provider(vm: &mut Vm) {
    vm.register("::tmm::_static_enroll", cmd_enroll);
    vm.register("::tmm::_static_domain", cmd_domain);
    vm.register("::tmm::_static_configure", cmd_configure);
    vm.register("::tmm::_static_reset_domain", cmd_reset);
}

fn cmd_domain(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    super::original_authored_operands::with_original_authored_operands(vm, |vm, operands| {
        Ok(cmd_domain_in_original(vm, args, operands))
    })
}

fn cmd_domain_in_original(
    vm: &mut Vm,
    args: &[Value],
    operands: &super::original_authored_operands::OriginalAuthoredOperandScope,
) -> Completion<Value> {
    let [mode] = args else {
        return err("static domain requires root or children");
    };
    let mode = match operands.string(vm, mode) {
        Ok(mode) => mode,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    vm.authored_tmm_static.topology = match mode.as_ref() {
        b"root" => AuthoredTmmWorkerTopology::RootInterpreter,
        b"children" => AuthoredTmmWorkerTopology::EnrolledChildren,
        _ => return err("static domain requires root or children"),
    };
    ok(Value::empty())
}

fn cmd_enroll(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return err("static enrollment takes no arguments");
    }
    match vm.enroll_static_worker() {
        Ok(()) => ok(Value::empty()),
        Err(error) => error,
    }
}

fn cmd_configure(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    super::original_authored_operands::with_original_authored_operands(vm, |vm, operands| {
        Ok(cmd_configure_in_original(vm, args, operands))
    })
}

fn cmd_configure_in_original(
    vm: &mut Vm,
    args: &[Value],
    operands: &super::original_authored_operands::OriginalAuthoredOperandScope,
) -> Completion<Value> {
    let [name, value] = args else {
        return err("static configuration requires a name and value");
    };
    let original = match operands.string(vm, name) {
        Ok(bytes) => bytes,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let Some(namespace) = vm.namespace_id_from_written(ROOT_NS, "::static") else {
        return vm.refuse_host_command("static configuration namespace is unavailable".into());
    };
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("static configuration name policy is unavailable".into());
    };
    let projected = policy.recipe().combined_variable_input(&original);
    let element = projected
        .element()
        .map(|element| NameBytes::from(element.selected()));
    let Some(binding) =
        vm.namespace_var_binding_from_token_bytes(namespace, projected.root().selected())
    else {
        return vm.refuse_host_command("static configuration name policy is unavailable".into());
    };
    // Explicit configuration requires the enrolled static domain, not a name
    // that escaped into another namespace.
    if binding.owner != VarTableOwner::Namespace(namespace) {
        return vm
            .refuse_host_command("static configuration name leaves the enrolled namespace".into());
    }
    let Some(worker) = vm.static_workers().first().copied() else {
        return vm.refuse_host_command("static configuration has no live enrolled worker".into());
    };
    let result = vm.in_interp(worker.interpreter, |vm| {
        let address = StaticAddress {
            interpreter: vm.cur,
            namespace: worker.namespace,
            name: binding.name.clone(),
            element: element.clone(),
        };
        let previous = vm.authored_tmm_static.configuration.replace(address);
        let result = vm.apply_static_primitive(
            worker.namespace,
            &binding.name,
            element.as_ref(),
            &Mutation::Store(value.clone()),
        );
        vm.authored_tmm_static.configuration = previous;
        result
    });
    match result {
        Ok(()) => ok(value.clone()),
        Err(error) => error,
    }
}

fn cmd_reset(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return err("static domain reset takes no arguments");
    }
    vm.authored_tmm_static.journal.clear();
    ok(Value::empty())
}
