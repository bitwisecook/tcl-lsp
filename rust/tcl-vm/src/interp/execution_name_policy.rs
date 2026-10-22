// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent measured name selection and actual authored event-frame storage.

use super::{Local, ResolvedVar, VarBinding, VarTableOwner, Vm};
use std::rc::{Rc, Weak};
use tcl_runtime_api::{Frames, authored_tmm::AuthoredObservedFrameStorageContext};
use tcl_syntax::{
    naming::{
        ExecutionNamePolicy, ExecutionVariableNameProjection, NativeVariableInputForm,
        ObservedBigIpNamePolicy, ObservedVariableNamePurpose,
    },
    value::ValueError,
};

/// Exact stages of the existing observed-input publication and cache cleanup.
#[derive(Clone, Copy)]
pub(super) enum ObservedInputReplacementStage {
    Published { changed: bool },
    CacheRetired,
}

type ObservedInputStage<'a> =
    dyn FnMut(&mut Vm, ObservedInputReplacementStage) -> Result<(), tcl_cmd_core::CmdError> + 'a;

#[derive(Clone)]
pub(super) struct ObservedNameSelection {
    policy: ObservedBigIpNamePolicy,
    build: tcl_registry::f5::evidence::BigIpBuild,
    context: tcl_registry::f5::BigIpExecutionContext,
    event: tcl_registry::f5::naming::BigIpNameEvent,
    profile: &'static tcl_dialect::DialectProfile,
    engine: Option<&'static tcl_dialect::DialectProfile>,
}

#[derive(Clone)]
pub(super) struct ObservedFrameStorage {
    context: AuthoredObservedFrameStorageContext,
    owner: Weak<crate::frame::ActivationIdentity>,
    level: usize,
    interpreter: super::InterpId,
}

fn unavailable() -> ValueError {
    ValueError::CommandProtocolUnavailable("observed event frame storage")
}

impl Vm {
    pub(crate) fn observed_name_policy_selected(&self) -> bool {
        self.observed_names.is_some()
    }

    /// Evaluate the retained original source object on the current actual frame.
    /// Original source home, object cache and body activation remain owned by
    /// the existing source-object evaluation entry.
    ///
    /// # Errors
    /// Returns the reached native execution or compilation refusal.
    pub fn try_eval_original_source_value(
        &mut self,
        source: &crate::value::Value,
    ) -> Result<
        tcl_runtime_api::Completion<crate::value::Value>,
        tcl_runtime_api::NativeExecutionError,
    > {
        self.host_execution_depth += 1;
        let completion = self.eval_value_at_level(self.current_level(), source);
        self.host_execution_depth -= 1;
        self.finish_host_execution(completion)
    }

    /// Install a caller-owned native event hook in the actual direct child.
    /// Registration executes in that child's command world and never aliases
    /// execution into the parent interpreter.
    pub fn register_child_native_command(
        &mut self,
        child: &str,
        name: &str,
        command: Rc<dyn crate::command::NativeCommand>,
    ) -> bool {
        match self.try_register_child_native_command(child, name, command) {
            Ok(registered) => registered,
            Err(error) => {
                let _ = crate::command::completion_from_tcl_error(self, error);
                false
            }
        }
    }

    /// Install through the same actual child and command-publication worker,
    /// retaining its full failure after mandatory interpreter restoration.
    ///
    /// # Errors
    /// The actual publication refusal remains outside guest Tcl completion.
    pub fn try_register_child_native_command(
        &mut self,
        child: &str,
        name: &str,
        command: Rc<dyn crate::command::NativeCommand>,
    ) -> Result<bool, crate::TclError> {
        let Some(id) = self.child_id(child) else {
            return Ok(false);
        };
        self.in_interp(id, |vm| {
            vm.try_register_native_command(name, command)
                .map(|()| true)
                .map_err(|error| {
                    crate::TclError::from_execution_failure(
                        vm.retain_host_command_refusal(error.to_string()),
                    )
                })
        })
    }

    pub(crate) fn with_original_observed_event_inputs_using(
        &mut self,
        original: &super::original_authored_operands::OriginalAuthoredOperandScope,
        build: tcl_registry::f5::evidence::BigIpBuild,
        context: tcl_registry::f5::BigIpExecutionContext,
        event: tcl_registry::f5::naming::BigIpNameEvent,
        storage: AuthoredObservedFrameStorageContext,
        body: impl FnOnce(
            &mut Self,
        ) -> Result<
            crate::Completion<crate::Value>,
            tcl_runtime_api::NativeExecutionError,
        >,
    ) -> Result<crate::Completion<crate::Value>, tcl_cmd_core::CmdError> {
        use super::native_operation_currency::SavedExecutionTarget;
        use tcl_cmd_core::CmdError;
        original.check(self)?;
        let saved = SavedExecutionTarget::capture(self);
        let previous_names = self.observed_names.clone();
        let previous_storage = self.observed_frame_storage.clone();
        let policy = tcl_registry::f5::naming::observed_name_policy(build, context, event)
            .ok_or_else(|| CmdError::from(unavailable()))?;
        let selected = ObservedNameSelection {
            policy,
            build,
            context,
            event,
            profile: self.dialect_profile(),
            engine: self.actual_engine_profile,
        };
        let mut names_reached = false;
        let mut storage_reached = false;
        let reached = (|| {
            original.check(self)?;
            // No callback occurs between this guard and the actual replacement.
            // Even a refused post-advance publication owes its prior restoration.
            names_reached = true;
            original.replace_observed_inputs(self, Some(selected), None, false)?;
            if self.execution_name_policy()
                != Some(ExecutionNamePolicy::ObservedBigIp(storage.policy))
                || storage.policy != policy
                || self.current_level() == 0
            {
                return Err(CmdError::from(unavailable()));
            }
            let level = self.current_level();
            let storage = ObservedFrameStorage {
                context: storage,
                owner: Rc::downgrade(&self.frames[level].activation),
                level,
                interpreter: self.cur,
            };
            original.check(self)?;
            storage_reached = true;
            let names = self.observed_names.clone();
            original.replace_observed_inputs(self, names, Some(storage), false)?;
            body(self).map_err(CmdError::from_execution_refusal)
        })();
        if let Err(first) = &reached {
            original.retain(self, first);
        }
        // Retain callback-caused current/physical failure before parking its
        // selection; mandatory restorations always run on the original target.
        let before_cleanup = original.check(self);
        if let Err(first) = &before_cleanup {
            original.retain(self, first);
        }
        let reached = saved
            .in_original(self, |vm| {
                if storage_reached {
                    let names = vm.observed_names.clone();
                    let _ = original.replace_observed_inputs(vm, names, None, true);
                }
                if names_reached {
                    let _ = original.replace_observed_inputs(
                        vm,
                        previous_names,
                        previous_storage,
                        true,
                    );
                }
                let _ = original.check(vm);
                // SavedExecutionTarget disposes unpublished completion ownership
                // inside this authentic attachment when its first cause is pending.
                reached
            })
            .map_err(CmdError::from_execution_refusal)?;
        before_cleanup?;
        original.check(self)?;
        reached
    }

    /// Retain a caller-owned exact event issuer for the duration of its body.
    /// Previous naming and frame scopes are restored after every returned completion.
    ///
    /// # Errors
    /// Refuses an unmeasured build, feature or event before executing the body.
    pub fn with_observed_name_policy<R>(
        &mut self,
        build: tcl_registry::f5::evidence::BigIpBuild,
        context: tcl_registry::f5::BigIpExecutionContext,
        event: tcl_registry::f5::naming::BigIpNameEvent,
        body: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, ValueError> {
        let previous_names = self.observed_names.clone();
        let previous_storage = self.observed_frame_storage.clone();
        if !self.set_observed_bigip_name_policy(build, context, event) {
            self.replace_observed_operation_inputs(previous_names, previous_storage);
            return Err(unavailable());
        }
        let result = body(self);
        self.replace_observed_operation_inputs(previous_names, previous_storage);
        Ok(result)
    }
    /// Retain an exact independently supplied appliance build, feature and event.
    /// This selects bounded counted name projections and supplies no physical storage.
    pub fn set_observed_bigip_name_policy(
        &mut self,
        build: tcl_registry::f5::evidence::BigIpBuild,
        context: tcl_registry::f5::BigIpExecutionContext,
        event: tcl_registry::f5::naming::BigIpNameEvent,
    ) -> bool {
        let policy = tcl_registry::f5::naming::observed_name_policy(build, context, event);
        let selected = policy.map(|policy| ObservedNameSelection {
            policy,
            build,
            context,
            event,
            profile: self.dialect_profile(),
            engine: self.actual_engine_profile,
        });
        self.replace_observed_operation_inputs(selected, None);
        policy.is_some()
    }

    #[must_use]
    pub fn execution_name_policy(&self) -> Option<ExecutionNamePolicy> {
        if let Some(selected) = &self.observed_names {
            return (tcl_registry::f5::naming::observed_name_policy(
                selected.build,
                selected.context,
                selected.event,
            ) == Some(selected.policy)
                && std::ptr::eq(selected.profile, self.dialect_profile())
                && selected.engine.map(std::ptr::from_ref)
                    == self.actual_engine_profile.map(std::ptr::from_ref))
            .then_some(ExecutionNamePolicy::ObservedBigIp(selected.policy));
        }
        self.name_policy_protocol()
            .map(ExecutionNamePolicy::NativeRecipe)
    }

    /// Install an independently authored counted-key provider on the current frame.
    ///
    /// # Errors
    /// Refuses an absent or stale issuer, global frame, or mismatched policy.
    pub fn with_observed_frame_storage<R>(
        &mut self,
        context: AuthoredObservedFrameStorageContext,
        body: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, ValueError> {
        if self.execution_name_policy() != Some(ExecutionNamePolicy::ObservedBigIp(context.policy))
            || self.current_level() == 0
        {
            return Err(unavailable());
        }
        let level = self.current_level();
        let storage = ObservedFrameStorage {
            context,
            owner: Rc::downgrade(&self.frames[level].activation),
            level,
            interpreter: self.cur,
        };
        let previous = self.observed_frame_storage.clone();
        self.replace_observed_operation_inputs(self.observed_names.clone(), Some(storage));
        let result = body(self);
        self.replace_observed_operation_inputs(self.observed_names.clone(), previous);
        Ok(result)
    }

    /// Enter a real event frame and retire its authored storage on completion.
    ///
    /// # Errors
    /// Refuses a missing exact policy issuer before entering the event.
    pub fn with_observed_event_frame<R>(
        &mut self,
        context: AuthoredObservedFrameStorageContext,
        body: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, ValueError> {
        if self.execution_name_policy() != Some(ExecutionNamePolicy::ObservedBigIp(context.policy))
        {
            return Err(unavailable());
        }
        let namespace = self.current_ns_id();
        let frame = Frames::push(self, namespace);
        let owner = Rc::downgrade(&self.frames[frame.0].activation);
        let result = self.with_observed_frame_storage(context, body);
        if self.current_level() == frame.0
            && owner.ptr_eq(&Rc::downgrade(&self.frames[frame.0].activation))
        {
            Frames::pop(self);
        }
        result
    }

    pub(super) fn replace_observed_operation_inputs(
        &mut self,
        names: Option<ObservedNameSelection>,
        storage: Option<ObservedFrameStorage>,
    ) -> bool {
        self.replace_observed_operation_inputs_using(names, storage, None)
            .expect("neutral observed-input replacement supplies no refusal hook")
    }

    pub(super) fn replace_observed_operation_inputs_using(
        &mut self,
        names: Option<ObservedNameSelection>,
        storage: Option<ObservedFrameStorage>,
        mut stage: Option<&mut ObservedInputStage<'_>>,
    ) -> Result<bool, tcl_cmd_core::CmdError> {
        let target = stage
            .as_ref()
            .map(|_| super::native_operation_currency::SavedExecutionTarget::capture(self));
        let same_names = match (&self.observed_names, &names) {
            (None, None) => true,
            (Some(old), Some(new)) => {
                old.policy == new.policy
                    && old.build == new.build
                    && old.context == new.context
                    && old.event == new.event
                    && std::ptr::eq(old.profile, new.profile)
                    && old.engine.map(std::ptr::from_ref) == new.engine.map(std::ptr::from_ref)
            }
            _ => false,
        };
        let same_storage = match (&self.observed_frame_storage, &storage) {
            (None, None) => true,
            (Some(old), Some(new)) => {
                old.context == new.context
                    && old.level == new.level
                    && old.interpreter == new.interpreter
                    && Weak::ptr_eq(&old.owner, &new.owner)
            }
            _ => false,
        };
        if !same_names || !same_storage {
            self.advance_native_operation_context();
        }
        self.observed_names = names;
        self.observed_frame_storage = storage;
        let changed = !same_names || !same_storage;
        self.invalidate_execution_name_policy_using(changed, &mut stage, target.as_ref())?;
        Ok(changed)
    }

    fn invalidate_execution_name_policy_using(
        &mut self,
        changed: bool,
        stage: &mut Option<&mut ObservedInputStage<'_>>,
        target: Option<&super::native_operation_currency::SavedExecutionTarget>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.name_world.borrow_mut().execution_name_policy = self.execution_name_policy();
        self.bump_cmd_epoch();
        self.bump_trace_deopt_epoch();
        self.invalidate_guard_domain(super::GuardDomain::Interpreter);
        self.profile_generation = self.profile_generation.wrapping_add(1);
        let mut first = self
            .check_observed_input_stage(stage, ObservedInputReplacementStage::Published { changed })
            .err();
        // These are the same reached mandatory cache clear workers. A retained
        // refusal never skips a later clear, and each actual clear settles its
        // stage before the next owner is retired.
        if let Err(error) = self.retire_observed_input_cache(target, |vm| vm.eval_cache.clear()) {
            if first.is_none() {
                first = Some(error);
            }
        }
        if let Err(error) =
            self.check_observed_input_stage(stage, ObservedInputReplacementStage::CacheRetired)
        {
            if first.is_none() {
                first = Some(error);
            }
        }
        if let Err(error) =
            self.retire_observed_input_cache(target, |vm| vm.eval_cache_plain.clear())
        {
            if first.is_none() {
                first = Some(error);
            }
        }
        if let Err(error) =
            self.check_observed_input_stage(stage, ObservedInputReplacementStage::CacheRetired)
        {
            if first.is_none() {
                first = Some(error);
            }
        }
        if let Err(error) = self.retire_observed_input_cache(target, |vm| vm.module_procs.clear()) {
            if first.is_none() {
                first = Some(error);
            }
        }
        if let Err(error) =
            self.check_observed_input_stage(stage, ObservedInputReplacementStage::CacheRetired)
        {
            if first.is_none() {
                first = Some(error);
            }
        }
        first.map_or(Ok(()), Err)
    }

    fn retire_observed_input_cache(
        &mut self,
        target: Option<&super::native_operation_currency::SavedExecutionTarget>,
        work: impl FnOnce(&mut Self),
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let Some(target) = target else {
            work(self);
            return Ok(());
        };
        let reached = target
            .retire_in_original(
                self,
                super::native_operation_currency::CleanupRestoration::PreserveCallbackSelection,
                work,
            )
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
        // The reached unit metadata is mandatory cleanup transport, including
        // after a first cause. No later clear follows Vm Deref into a child.
        let () = reached.metadata;
        if let Some(first) = reached.first_cause {
            let first = self.retain_tcl_host_failure(crate::TclHostFailure::Execution(first));
            return Err(tcl_cmd_core::CmdError::from_execution_refusal(first));
        }
        Ok(())
    }

    fn check_observed_input_stage(
        &mut self,
        stage: &mut Option<&mut ObservedInputStage<'_>>,
        reached: ObservedInputReplacementStage,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let result = stage
            .as_deref_mut()
            .map_or(Ok(()), |check| check(self, reached));
        if let Err(error) = &result {
            if let Some(first) = error.native_execution_refusal() {
                self.retain_tcl_host_failure(crate::TclHostFailure::Execution(first.clone()));
            }
        }
        result
    }

    fn observed_projection<'a>(
        &self,
        original: &'a [u8],
        level: usize,
        purpose: ObservedVariableNamePurpose,
    ) -> Result<ExecutionVariableNameProjection<'a>, ValueError> {
        let policy = self.execution_name_policy().ok_or_else(unavailable)?;
        let storage = self
            .observed_frame_storage
            .as_ref()
            .ok_or_else(unavailable)?;
        let frame = self.frames.get(level).ok_or_else(unavailable)?;
        if level != self.current_level()
            || storage.level != level
            || storage.interpreter != self.cur
            || !storage.owner.ptr_eq(&Rc::downgrade(&frame.activation))
            || storage.owner.upgrade().is_none()
            || policy != ExecutionNamePolicy::ObservedBigIp(storage.context.policy)
        {
            return Err(unavailable());
        }
        policy
            .variable_input(NativeVariableInputForm::Combined(original), purpose)
            .map_err(|_| unavailable())
    }

    fn counted_cell_is_owned(&self, id: tcl_runtime_api::VarId) -> bool {
        !self.variable_observers.script_traces.contains_key(&id)
            && !self.var_arena.has_link_refs(id)
            && !self.var_arena.has_static_refs(id)
            && self.var_arena.get(id).is_some_and(|cell| {
                cell.namespace_owner().is_none()
                    && !matches!(cell.state(), Local::Link(_) | Local::NameLink(_))
            })
    }

    fn observed_resolved_for(
        &self,
        original: &[u8],
        level: usize,
        purpose: ObservedVariableNamePurpose,
    ) -> Result<ResolvedVar, ValueError> {
        let projection = self.observed_projection(original, level, purpose)?;
        let frame = &self.frames[level];
        if frame
            .compiled_locals
            .iter()
            .any(|(name, _)| name.as_bytes() == projection.root())
            || frame
                .statics
                .as_ref()
                .is_some_and(|statics| statics.table.contains_key(projection.root()))
        {
            return Err(unavailable());
        }
        let binding = VarBinding {
            owner: VarTableOwner::Frame(level),
            name: projection.root().into(),
        };
        let base_id = frame.locals.get(projection.root()).copied();
        if base_id.is_some_and(|id| !self.counted_cell_is_owned(id)) {
            return Err(unavailable());
        }
        let id = match (base_id, projection.element()) {
            (Some(base), Some(key)) => {
                match self.var_arena.get(base).map(crate::vars::VarCell::state) {
                    Some(Local::Array(elements)) => elements.get(key).copied(),
                    _ => None,
                }
            }
            (id, None) => id,
            (None, Some(_)) => None,
        };
        if id.is_some_and(|id| !self.counted_cell_is_owned(id)) {
            return Err(unavailable());
        }
        Ok(ResolvedVar {
            binding,
            base_id,
            id,
            elem: projection.element().map(Into::into),
        })
    }

    pub(super) fn observed_resolved_variable(
        &self,
        name: &[u8],
        level: usize,
    ) -> Result<ResolvedVar, ValueError> {
        let purpose = if name.ends_with(b")") {
            ObservedVariableNamePurpose::CombinedElement
        } else {
            ObservedVariableNamePurpose::ScalarReceiver
        };
        self.observed_resolved_for(name, level, purpose)
    }

    fn allocate_counted_root(
        &mut self,
        resolved: &ResolvedVar,
        level: usize,
    ) -> Result<tcl_runtime_api::VarId, ValueError> {
        if let Some(id) = resolved.base_id {
            return Ok(id);
        }
        let id = self
            .var_arena
            .alloc(Local::Undefined)
            .ok_or_else(unavailable)?;
        self.frames[level]
            .locals
            .insert(resolved.binding.name.clone(), id);
        self.var_arena.bind(id);
        Ok(id)
    }

    pub(super) fn observed_variable_get(
        &self,
        name: &[u8],
        level: usize,
    ) -> Result<Option<crate::value::Value>, ValueError> {
        let resolved = self.observed_resolved_variable(name, level)?;
        match resolved
            .id
            .and_then(|id| self.var_arena.get(id))
            .map(crate::vars::VarCell::state)
        {
            Some(Local::Scalar(value)) => Ok(Some(value.clone())),
            Some(Local::Array(_) | Local::Link(_) | Local::NameLink(_)) => Err(unavailable()),
            Some(Local::Undefined) | None => Ok(None),
        }
    }

    pub(crate) fn observed_variable_exists(
        &self,
        name: &[u8],
        level: usize,
    ) -> Result<bool, ValueError> {
        let resolved = self.observed_resolved_variable(name, level)?;
        Ok(resolved
            .id
            .and_then(|id| self.var_arena.get(id))
            .is_some_and(|cell| matches!(cell.state(), Local::Scalar(_) | Local::Array(_))))
    }

    pub(super) fn observed_variable_set(
        &mut self,
        name: &[u8],
        level: usize,
        value: crate::value::Value,
    ) -> Result<(), ValueError> {
        let resolved = self.observed_resolved_variable(name, level)?;
        if resolved
            .base_id
            .into_iter()
            .chain(resolved.id)
            .any(|id| self.const_vars.contains(&id))
        {
            return Err(unavailable());
        }
        let base = self.allocate_counted_root(&resolved, level)?;
        let id = if let Some(key) = resolved.elem.as_deref() {
            match self.var_arena.get(base).map(crate::vars::VarCell::state) {
                Some(Local::Undefined) => {
                    self.var_arena
                        .replace_state(base, Local::Array(super::VarTable::new()));
                }
                Some(Local::Array(_)) => {}
                _ => return Err(unavailable()),
            }
            if let Some(id) = resolved.id {
                id
            } else {
                let id = self
                    .var_arena
                    .alloc_element(Local::Undefined, base, key)
                    .ok_or_else(unavailable)?;
                if !self.var_arena.array_insert(base, key, id) {
                    self.var_arena.discard_unbound(id);
                    return Err(unavailable());
                }
                self.var_arena.bind(id);
                id
            }
        } else {
            if self
                .var_arena
                .get(base)
                .is_some_and(|cell| matches!(cell.state(), Local::Array(_)))
            {
                return Err(unavailable());
            }
            base
        };
        self.var_arena
            .replace_state(id, Local::Scalar(value))
            .then_some(())
            .ok_or_else(unavailable)
    }

    pub(super) fn observed_variable_unset(
        &mut self,
        name: &[u8],
        level: usize,
    ) -> Result<bool, ValueError> {
        let resolved = self.observed_resolved_variable(name, level)?;
        let Some(id) = resolved.id else {
            return Ok(false);
        };
        if self.const_vars.contains(&id) {
            return Err(unavailable());
        }
        let existed = self
            .var_arena
            .get(id)
            .is_some_and(|cell| matches!(cell.state(), Local::Scalar(_) | Local::Array(_)));
        self.var_arena.replace_state(id, Local::Undefined);
        let raw = if let Some(key) = &resolved.elem {
            self.var_arena
                .array_remove(resolved.base_id.ok_or_else(unavailable)?, key)
        } else {
            self.frames[level].locals.remove(&resolved.binding.name)
        };
        if let Some(raw) = raw {
            self.var_arena.unbind(raw);
        }
        Ok(existed)
    }

    pub(super) fn observed_array_target(
        &self,
        name: &[u8],
        level: usize,
    ) -> Result<tcl_runtime_api::ArrayTarget, ValueError> {
        let resolved =
            self.observed_resolved_for(name, level, ObservedVariableNamePurpose::ArrayRoot)?;
        Ok(resolved.id.map_or_else(
            || tcl_runtime_api::ArrayTarget::named_bytes(tcl_runtime_api::FrameId(level), name),
            |id| {
                tcl_runtime_api::ArrayTarget::cell_bytes(tcl_runtime_api::FrameId(level), name, id)
            },
        ))
    }

    pub(super) fn observed_array_keys(
        &self,
        target: &tcl_runtime_api::ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, ValueError> {
        let resolved = self.observed_resolved_for(
            target.name_bytes(),
            target.frame().0,
            ObservedVariableNamePurpose::ArrayRoot,
        )?;
        if resolved.id != target.cell_id() {
            return Err(unavailable());
        }
        let Some(Local::Array(elements)) = resolved
            .id
            .and_then(|id| self.var_arena.get(id))
            .map(crate::vars::VarCell::state)
        else {
            return Ok(None);
        };
        if elements.values().any(|id| !self.counted_cell_is_owned(*id)) {
            return Err(unavailable());
        }
        Ok(Some(
            elements
                .iter()
                .filter(|(_, id)| {
                    self.var_arena
                        .get(**id)
                        .is_some_and(|cell| matches!(cell.state(), Local::Scalar(_)))
                })
                .map(|(key, _)| key.as_bytes().to_vec())
                .collect(),
        ))
    }

    pub(super) fn observed_ensure_array(
        &mut self,
        name: &[u8],
        level: usize,
    ) -> Result<(), ValueError> {
        let resolved =
            self.observed_resolved_for(name, level, ObservedVariableNamePurpose::ArrayRoot)?;
        let id = self.allocate_counted_root(&resolved, level)?;
        match self.var_arena.get(id).map(crate::vars::VarCell::state) {
            Some(Local::Array(_)) => Ok(()),
            Some(Local::Undefined) => self
                .var_arena
                .replace_state(id, Local::Array(super::VarTable::new()))
                .then_some(())
                .ok_or_else(unavailable),
            _ => Err(unavailable()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::f5::{BigIpExecutionContext, evidence::BigIpBuild, naming::BigIpNameEvent};
    use tcl_runtime_api::{FrameId, VarStore};

    fn select(vm: &mut Vm) -> AuthoredObservedFrameStorageContext {
        assert!(vm.set_observed_bigip_name_policy(
            BigIpBuild::MEASURED_21_1_0_1,
            BigIpExecutionContext::TmmIRule,
            BigIpNameEvent::HttpRequest
        ));
        let Some(ExecutionNamePolicy::ObservedBigIp(policy)) = vm.execution_name_policy() else {
            panic!("exact measured issuer");
        };
        AuthoredObservedFrameStorageContext {
            policy,
            tmm: 2,
            domain: 7,
        }
    }

    #[test]
    fn measured_names_require_actual_storage_and_retire_with_event_frame() {
        let mut vm = Vm::new();
        let context = select(&mut vm);
        let name = b"__tcl_lsp_2286_r2286m_nul_A\0B";
        assert!(VarStore::get_bytes(&vm, FrameId(0), name).is_err());
        vm.with_observed_event_frame(context, |vm| {
            let frame = Frames::current(vm);
            VarStore::set_bytes(vm, frame, name, crate::value::Value::string("counted")).unwrap();
            assert_eq!(
                VarStore::get_bytes(vm, frame, name)
                    .unwrap()
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                b"counted"
            );
            assert!(VarStore::get_bytes(vm, frame, b"::arbitrary").is_err());
            assert!(VarStore::get_bytes(vm, FrameId(0), name).is_err());
            assert!(vm.native_c_variable_name_protocol().is_none());
            let array = b"__tcl_lsp_2286_r2286m_arrayindex_root(A\0B)";
            VarStore::set_bytes(vm, frame, array, crate::value::Value::string("element")).unwrap();
            assert_eq!(
                VarStore::get_bytes(vm, frame, array)
                    .unwrap()
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                b"element"
            );
            assert!(VarStore::unset_bytes(vm, frame, array).unwrap());
            assert!(VarStore::get_bytes(vm, frame, array).unwrap().is_none());
        })
        .unwrap();
        assert_eq!(Frames::current(&vm), FrameId(0));
        assert!(VarStore::get_bytes(&vm, FrameId(0), name).is_err());
    }

    #[test]
    fn observed_counted_set_dispatch_uses_independent_command_and_source_issuers() {
        let profile = crate::environment::profile_for_dialect("tcl9.0");
        let mut vm = crate::native_fixture::interpreter(profile);
        let context = select(&mut vm);
        vm.with_observed_event_frame(context, |vm| {
            let result = vm.eval_source("set {__tcl_lsp_2286_r2286m_nul_A\0B} counted; set {__tcl_lsp_2286_r2286m_nul_A\0B}").unwrap();
            assert_eq!(result.code, tcl_runtime_api::Code::Ok);
            assert_eq!(result.result.string_bytes().as_ref(), b"counted");
            let result = vm.eval_source("set {__tcl_lsp_2286_r2286m_arrayindex_root(A\0B)} element; set {__tcl_lsp_2286_r2286m_arrayindex_root(A\0B)}").unwrap();
            assert_eq!(result.code, tcl_runtime_api::Code::Ok);
            assert_eq!(result.result.string_bytes().as_ref(), b"element");
        }).unwrap();
    }

    #[test]
    fn counted_array_root_and_scalar_queries_use_normal_source_dispatch() {
        let profile = crate::environment::profile_for_dialect("tcl9.0");
        let mut vm = crate::native_fixture::interpreter(profile);
        let context = select(&mut vm);
        vm.with_observed_event_frame(context, |vm| {
            let result = vm.eval_source("set {R\0T(k)} element; list [array exists {R\0T}] [array size {R\0T}] [array names {R\0T}] [info exists {R\0T(k)}] [info exists {R\0T(missing)}]").unwrap();
            assert_eq!(result.code, tcl_runtime_api::Code::Ok);
            assert_eq!(result.result.string_bytes().as_ref(), b"1 1 k 1 0");
            let result = vm.eval_source("set {S\0T} scalar; set copied ${S\0T}; unset {S\0T}; list $copied [info exists {S\0T}]").unwrap();
            assert_eq!(result.code, tcl_runtime_api::Code::Ok);
            assert_eq!(result.result.string_bytes().as_ref(), b"scalar 0");
        }).unwrap();
    }

    #[test]
    fn counted_dynamic_cell_does_not_collide_with_declared_cstring_prefix() {
        let mut vm = Vm::new();
        let context = select(&mut vm);
        vm.with_observed_event_frame(context, |vm| {
            let level = vm.current_level();
            let declared = vm
                .var_arena
                .alloc(Local::Scalar(crate::value::Value::string("formal")))
                .expect("allocate declared formal cell");
            vm.var_arena.bind(declared);
            vm.frames[level]
                .compiled_locals
                .push((b"A".as_slice().into(), declared));
            vm.observed_variable_set(b"A\0B", level, crate::value::Value::string("counted"))
                .unwrap();
            assert!(
                vm.observed_variable_set(b"A", level, crate::value::Value::string("replacement"))
                    .is_err()
            );
            assert_ne!(
                vm.frames[level].locals.get(b"A\0B".as_slice()),
                Some(&declared)
            );
            let Some(Local::Scalar(formal)) =
                vm.var_arena.get(declared).map(crate::vars::VarCell::state)
            else {
                panic!("retained formal cell")
            };
            assert_eq!(formal.string_bytes().as_ref(), b"formal");
        })
        .unwrap();
    }

    #[test]
    fn observed_trace_and_upvar_purposes_are_unavailable_in_source_dispatch() {
        for source in [
            "trace add variable A write {set callback}",
            "upvar 0 A alias",
        ] {
            let profile = crate::environment::profile_for_dialect("tcl9.0");
            let mut vm = crate::native_fixture::interpreter(profile);
            let context = select(&mut vm);
            vm.with_observed_event_frame(context, |vm| {
                assert!(vm.eval_source(source).is_err(), "{source}");
                assert!(vm.variable_observers.script_traces.is_empty());
            })
            .unwrap();
        }
    }

    #[test]
    fn scoped_selection_invalidates_entered_and_restored_compilation_world() {
        let mut vm = Vm::new();
        let before = vm.execution_name_policy();
        let generation = vm.profile_generation;
        let result: Result<(), ()> = vm
            .with_observed_name_policy(
                BigIpBuild::MEASURED_21_1_0_1,
                BigIpExecutionContext::TmmIRule,
                BigIpNameEvent::HttpRequest,
                |vm| {
                    assert_ne!(vm.execution_name_policy(), before);
                    assert!(vm.profile_generation > generation);
                    Err(())
                },
            )
            .unwrap();
        assert_eq!(result, Err(()));
        assert_eq!(vm.execution_name_policy(), before);
        assert!(vm.profile_generation > generation + 1);
    }

    #[test]
    fn frame_replacement_and_profile_change_withdraw_observed_storage() {
        let mut vm = Vm::new();
        let context = select(&mut vm);
        Frames::push(&mut vm, tcl_runtime_api::ROOT_NS);
        vm.with_observed_frame_storage(context, |vm| {
            Frames::pop(vm);
            Frames::push(vm, tcl_runtime_api::ROOT_NS);
            assert!(
                VarStore::get_bytes(vm, Frames::current(vm), b"__tcl_lsp_2286_r2286m_nul_A")
                    .is_err()
            );
        })
        .unwrap();
        Frames::pop(&mut vm);
        vm.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        assert!(vm.execution_name_policy().is_none());
    }
}

#[cfg(test)]
#[path = "execution_name_policy/original_event_transition_tests.rs"]
mod original_event_transition_tests;
