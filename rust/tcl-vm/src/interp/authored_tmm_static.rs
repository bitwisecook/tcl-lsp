//! Authored static publication over enrolled, live worker namespace cells.

use super::{
    Completion, HashSet, InterpId, Local, NameBytes, Namespaces, NsId, ROOT_INTERP, ROOT_NS, Value,
    VarBinding, VarId, VarTable, VarTableOwner, Vm, err, ok,
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
                observer_epoch: state.compilation_epochs.trace_deopt_epoch.get(),
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
        let mut reported = Namespaces::name_bytes(self, namespace);
        if reported.as_slice() != b"::" {
            reported.extend_from_slice(b"::");
        }
        reported.extend_from_slice(name.as_bytes());
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
            .actual_native_invocation_dialect()
            .native_string_protocol()
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
    let [mode] = args else {
        return err("static domain requires root or children");
    };
    let mode = match vm.native_name_operand_bytes(mode) {
        Ok(mode) => mode,
        Err(error) => return vm.refuse_host_command(error.to_string()),
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
    let [name, value] = args else {
        return err("static configuration requires a name and value");
    };
    let original = match vm.native_name_operand_bytes(name) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use tcl_runtime_api::VarStore;
    use tcl_runtime_api::{Code, GLOBAL_FRAME};

    fn vm() -> Vm {
        let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .expect("authentic C object sharing");
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm.install_irules_timer_simulation();
        vm.install_irules_static_simulation();
        vm.authored_tmm_static.topology = AuthoredTmmWorkerTopology::EnrolledChildren;
        vm
    }

    fn eval(vm: &mut Vm, source: &str) -> Completion<Value> {
        vm.try_eval_source(source)
            .expect("actual selected C source")
    }

    fn worker(vm: &mut Vm, name: &str) -> InterpId {
        vm.create_child(Some(name.into()), false);
        let id = vm.child_id(name).expect("original child");
        vm.in_interp(id, |vm| {
            assert!(eval(vm, "namespace eval ::static {}; ::tmm::_static_enroll").is_ok());
        });
        id
    }

    fn init(vm: &mut Vm, worker: InterpId, source: &str) -> Completion<Value> {
        vm.in_interp(worker, |vm| {
            assert!(eval(vm, "::tmm::_timer_context_enter RULE_INIT {}").is_ok());
            let mut result = eval(vm, source);
            // This test receiver retains the original result across a separate
            // evaluation, just as a native caller must retain Tcl_GetObjResult.
            result.result = result.result.into_native_reference();
            assert!(eval(vm, "::tmm::_timer_context_leave").is_ok());
            result
        })
    }

    fn query(vm: &mut Vm, worker: InterpId, source: &str) -> String {
        vm.in_interp(worker, |vm| {
            let result = eval(vm, source);
            assert!(result.is_ok(), "{source}: {:?}", result.result);
            result.result.to_str().to_string()
        })
    }

    #[test]
    fn rule_init_publishes_resolved_alias_rmw_and_array_cells_only() {
        let mut vm = vm();
        let a = worker(&mut vm, "a");
        let b = worker(&mut vm, "b");
        assert!(init(&mut vm, a, "set ::ordinary 11; set ::static::n 4; proc bump {} {upvar #0 ::static::n alias; incr alias; lappend ::static::items X}; bump; array set ::static::empty {}; set ::static::a(k) OLD").is_ok());
        assert_eq!(
            query(
                &mut vm,
                b,
                "list $::static::n $::static::items [array exists ::static::empty] $::static::a(k) [info exists ::ordinary]"
            ),
            "5 X 1 OLD 0"
        );
        assert!(
            vm.in_interp(a, |vm| eval(vm, "incr ::static::n; set ::ordinary 12"))
                .is_ok()
        );
        assert_eq!(query(&mut vm, b, "set ::static::n"), "5");
        assert!(
            init(
                &mut vm,
                b,
                "set ::ordinary 29; set ::static::n $::ordinary; unset ::static::a(k)"
            )
            .is_ok()
        );
        assert_eq!(
            query(
                &mut vm,
                a,
                "list $::static::n $::ordinary [info exists ::static::a(k)]"
            ),
            "29 12 0"
        );
    }

    #[test]
    fn recipient_traces_publish_nested_writes_and_preserve_partial_errors() {
        let mut vm = vm();
        let a = worker(&mut vm, "a");
        let b = worker(&mut vm, "b");
        assert!(vm.in_interp(b, |vm| eval(vm, "proc observe {name key op} {set ::static::nested FROM_TRACE; error recipient_failed}; trace add variable ::static::n write observe")).is_ok());
        let result = init(
            &mut vm,
            a,
            "set ::static::n FIRST; set ::static::unreached 1",
        );
        assert_eq!(result.code, Code::Error);
        assert!(result.result.to_str().contains("recipient_failed"));
        for id in [a, b] {
            assert_eq!(
                query(
                    &mut vm,
                    id,
                    "list $::static::n $::static::nested [info exists ::static::unreached]"
                ),
                "FIRST FROM_TRACE 0"
            );
        }
        assert!(
            vm.in_interp(b, |vm| eval(
                vm,
                "trace remove variable ::static::n write observe"
            ))
            .is_ok()
        );
        assert!(
            vm.in_interp(a, |vm| eval(vm, "set ::static::nested ONLY_A"))
                .is_ok()
        );
        assert_eq!(query(&mut vm, b, "set ::static::nested"), "FROM_TRACE");
    }

    #[test]
    fn enrollment_replays_owned_publications_and_retires_namespace_incarnations() {
        let mut vm = vm();
        let a = worker(&mut vm, "a");
        assert!(init(&mut vm, a, "set ::static::n SEED; array set ::static::empty {}; set ::static::a(k) OLD; unset ::static::a(k)").is_ok());
        let b = worker(&mut vm, "b");
        assert_eq!(
            query(
                &mut vm,
                b,
                "list $::static::n [array exists ::static::empty] [array exists ::static::a] [info exists ::static::a(k)]"
            ),
            "SEED 1 1 0"
        );
        assert!(
            vm.in_interp(b, |vm| eval(
                vm,
                "namespace delete ::static; namespace eval ::static {};"
            ))
            .is_ok()
        );
        assert!(init(&mut vm, a, "set ::static::n NEW").is_ok());
        assert_eq!(query(&mut vm, b, "info exists ::static::n"), "0");
        assert!(
            vm.in_interp(b, |vm| eval(vm, "::tmm::_static_enroll"))
                .is_ok()
        );
        assert_eq!(query(&mut vm, b, "set ::static::n"), "NEW");
        assert!(vm.delete_interp(b));
        let b2 = worker(&mut vm, "b");
        assert_ne!(b, b2);
        assert_eq!(query(&mut vm, b2, "set ::static::n"), "NEW");
    }

    #[test]
    fn noncurrent_storage_facades_publish_without_source_trace_callbacks() {
        let mut vm = vm();
        let a = worker(&mut vm, "a");
        let b = worker(&mut vm, "b");
        vm.in_interp(a, |vm| {
            assert!(eval(vm, "set ::source_hits 0; proc source_trace {args} {incr ::source_hits}; trace add variable ::static::n write source_trace; ::tmm::_timer_context_enter RULE_INIT {}").is_ok());
            vm.push_call_frame(None, Vec::new());
            VarStore::set(vm, GLOBAL_FRAME, "::static::n", Value::int(7));
            VarStore::set_elem(vm, GLOBAL_FRAME, "::static::a", "k", Value::string("MEMBER"));
            assert!(VarStore::unset_elem(vm, GLOBAL_FRAME, "::static::a", "k"));
            vm.pop_call_frame();
            assert!(eval(vm, "::tmm::_timer_context_leave").is_ok());
        });
        assert_eq!(query(&mut vm, a, "set ::source_hits"), "0");
        assert_eq!(
            query(
                &mut vm,
                b,
                "list $::static::n [array exists ::static::a] [info exists ::static::a(k)]"
            ),
            "7 1 0"
        );
    }

    #[test]
    fn checked_noncurrent_store_retains_recipient_failure_and_prior_mutations() {
        let mut vm = vm();
        let a = worker(&mut vm, "a");
        let b = worker(&mut vm, "b");
        assert!(vm.in_interp(b, |vm| eval(vm, "proc failure {args} {error original_recipient_error}; trace add variable ::static::n write failure")).is_ok());
        vm.in_interp(a, |vm| {
            assert!(eval(vm, "::tmm::_timer_context_enter RULE_INIT {}").is_ok());
            vm.push_call_frame(None, Vec::new());
            let error = vm
                .write_scalar_bytes_from_checked(0, b"::static::n", Value::int(9))
                .expect_err("normal recipient trace error");
            assert_eq!(error.code, Code::Error);
            assert!(error.result.to_str().contains("original_recipient_error"));
            assert!(vm.execution_refusal.is_none());
            vm.pop_call_frame();
            assert!(eval(vm, "::tmm::_timer_context_leave").is_ok());
        });
        for id in [a, b] {
            assert_eq!(query(&mut vm, id, "set ::static::n"), "9");
        }
    }

    #[test]
    fn recipient_aliases_follow_actual_static_cells_without_republishing_or_touching_globals() {
        let mut vm = vm();
        let a = worker(&mut vm, "a");
        let b = worker(&mut vm, "b");
        assert!(vm.in_interp(b, |vm| eval(vm,
            "set ::static::target OLD; upvar #0 ::static::target ::static::n; set ::ordinary LOCAL; upvar #0 ::ordinary ::static::foreign"
        )).is_ok());
        assert!(
            init(
                &mut vm,
                a,
                "set ::static::n NEW; set ::static::foreign SOURCE"
            )
            .is_ok()
        );
        assert_eq!(
            query(
                &mut vm,
                b,
                "list $::static::target $::static::n $::ordinary"
            ),
            "NEW NEW LOCAL"
        );
        assert_eq!(
            query(
                &mut vm,
                a,
                "list $::static::n $::static::foreign [info exists ::static::target] [info exists ::ordinary]"
            ),
            "NEW SOURCE 0 0"
        );
        assert!(vm.execution_refusal.is_none());
        let c = worker(&mut vm, "c");
        assert_eq!(
            query(
                &mut vm,
                c,
                "list $::static::n $::static::foreign [info exists ::static::target]"
            ),
            "NEW SOURCE 0"
        );
    }

    #[test]
    fn compilation_context_retains_actual_recipient_observers_and_event_phase() {
        use tcl_runtime_api::native_compilation::NativeVariableObserverPresence as Presence;
        let mut vm = vm();
        let a = worker(&mut vm, "a");
        let b = worker(&mut vm, "b");
        let unknown = vm.in_interp(a, |vm| vm.authored_static_compilation_context().unwrap());
        assert_eq!(unknown.context, None);
        assert_eq!(unknown.outward_observers, Presence::Unknown);
        assert_eq!(unknown.recipients.len(), 1);
        assert_ne!(
            unknown.namespace.interpreter,
            unknown.recipients[0].namespace.interpreter
        );
        assert!(
            vm.in_interp(a, |vm| eval(vm, "::tmm::_timer_context_enter RULE_INIT {}"))
                .is_ok()
        );
        let before = vm.in_interp(a, |vm| vm.authored_static_compilation_context().unwrap());
        assert_eq!(before.outward_observers, Presence::Absent);
        assert!(vm.in_interp(b, |vm| eval(vm,
            "proc observe {args} {}; trace add variable ::ordinary write observe; upvar #0 ::ordinary ::static::foreign"
        )).is_ok());
        let outside = vm.in_interp(a, |vm| vm.authored_static_compilation_context().unwrap());
        assert_eq!(outside.outward_observers, Presence::Absent);
        assert_ne!(
            before.recipients[0].observer_epoch,
            outside.recipients[0].observer_epoch
        );
        assert!(vm.in_interp(b, |vm| eval(vm,
            "trace add variable ::static::n write observe; trace add variable ::static::a(k) unset observe"
        )).is_ok());
        let observed = vm.in_interp(a, |vm| vm.authored_static_compilation_context().unwrap());
        assert_eq!(observed.outward_observers, Presence::Present);
        assert_ne!(outside, observed);
        assert!(
            vm.in_interp(a, |vm| eval(
                vm,
                "::tmm::_timer_context_leave; ::tmm::_timer_context_enter HTTP_REQUEST {}"
            ))
            .is_ok()
        );
        let event = vm.in_interp(a, |vm| vm.authored_static_compilation_context().unwrap());
        assert_eq!(
            event.context,
            Some(TmmStaticExecutionContext::ExecutingWorker)
        );
        assert_eq!(event.outward_observers, Presence::Absent);
        assert_eq!(event.recipients, observed.recipients);
        assert!(
            vm.in_interp(a, |vm| eval(vm, "::tmm::_timer_context_leave"))
                .is_ok()
        );
        assert!(vm.delete_interp(b));
        let retired = vm.in_interp(a, |vm| vm.authored_static_compilation_context().unwrap());
        assert!(retired.recipients.is_empty());
    }
    #[test]
    fn direct_native_recipient_observers_invalidate_original_static_receipts() {
        use std::cell::Cell;
        use tcl_runtime_api::{
            native_compilation::NativeVariableObserverPresence as Presence,
            native_variable_trace::{
                NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
            },
        };

        struct Observe {
            recipient: InterpId,
            calls: Rc<Cell<usize>>,
        }
        impl NativeVariableObserver<Vm> for Observe {
            type Error = tcl_cmd_core::CmdError;
            fn observe(
                &self,
                runtime: &mut Vm,
                access: NativeVariableTraceAccess<'_>,
            ) -> Result<(), Self::Error> {
                assert_eq!(runtime.cur_interp(), self.recipient);
                assert_eq!(access.operation, NativeVariableTraceOperation::Write);
                self.calls.set(self.calls.get() + 1);
                Ok(())
            }
        }

        let mut vm = vm();
        let source = worker(&mut vm, "source");
        let recipient = worker(&mut vm, "recipient");
        let unknown = vm.in_interp(source, |vm| {
            vm.authored_static_compilation_context().unwrap()
        });
        assert_eq!(unknown.outward_observers, Presence::Unknown);
        assert!(
            vm.in_interp(source, |vm| eval(
                vm,
                "::tmm::_timer_context_enter RULE_INIT {}"
            ))
            .is_ok()
        );
        let absent = vm.in_interp(source, |vm| {
            vm.authored_static_compilation_context().unwrap()
        });
        assert_eq!(absent.outward_observers, Presence::Absent);

        let name = Value::new_native_string_bytes(b"::static::n".as_slice());
        let calls = Rc::new(Cell::new(0));
        let observer = Rc::new(Observe {
            recipient,
            calls: Rc::clone(&calls),
        });
        let original = vm.in_interp(recipient, |vm| {
            vm.add_native_variable_observer(
                &name,
                &[NativeVariableTraceOperation::Write],
                observer.clone(),
            )
            .unwrap()
        });
        let observed = vm.in_interp(source, |vm| {
            vm.authored_static_compilation_context().unwrap()
        });
        assert_eq!(observed.outward_observers, Presence::Present);
        assert_eq!(
            query(&mut vm, recipient, "trace info variable ::static::n"),
            ""
        );
        assert_ne!(
            absent.recipients[0].observer_epoch,
            observed.recipients[0].observer_epoch
        );
        assert_eq!(
            observed,
            vm.in_interp(source, |vm| vm
                .authored_static_compilation_context()
                .unwrap())
        );
        assert!(
            vm.in_interp(source, |vm| eval(vm, "set ::static::n FIRST"))
                .is_ok()
        );
        assert_eq!(calls.get(), 1);

        assert!(
            vm.in_interp(recipient, |vm| eval(
                vm,
                "unset ::static::n; set ::static::n REPLACED"
            ))
            .is_ok()
        );
        assert!(!vm.in_interp(recipient, |vm| {
            vm.remove_native_variable_observer(&original)
        }));
        let retired = vm.in_interp(source, |vm| {
            vm.authored_static_compilation_context().unwrap()
        });
        assert_eq!(retired.outward_observers, Presence::Absent);
        assert_ne!(
            observed.recipients[0].observer_epoch,
            retired.recipients[0].observer_epoch
        );
        let replacement = vm.in_interp(recipient, |vm| {
            vm.add_native_variable_observer(&name, &[NativeVariableTraceOperation::Write], observer)
                .unwrap()
        });
        assert!(!original.same_registration(&replacement));
        let replaced = vm.in_interp(source, |vm| {
            vm.authored_static_compilation_context().unwrap()
        });
        assert_eq!(replaced.outward_observers, Presence::Present);
        assert_ne!(observed, replaced);
        assert!(
            vm.in_interp(source, |vm| eval(vm, "set ::static::n SECOND"))
                .is_ok()
        );
        assert_eq!(calls.get(), 2);
        assert!(vm.in_interp(recipient, |vm| {
            vm.remove_native_variable_observer(&replacement)
        }));
        let removed = vm.in_interp(source, |vm| {
            vm.authored_static_compilation_context().unwrap()
        });
        assert_eq!(removed.outward_observers, Presence::Absent);
        assert_ne!(
            removed.recipients[0].observer_epoch,
            replaced.recipients[0].observer_epoch
        );
        assert!(
            vm.in_interp(source, |vm| eval(vm, "::tmm::_timer_context_leave"))
                .is_ok()
        );
        let outside = vm.in_interp(source, |vm| {
            vm.authored_static_compilation_context().unwrap()
        });
        assert_eq!(outside.outward_observers, Presence::Unknown);
    }
}
