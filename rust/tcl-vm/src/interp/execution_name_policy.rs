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

pub(super) struct ObservedNameSelection {
    policy: ObservedBigIpNamePolicy,
    build: tcl_registry::f5::evidence::BigIpBuild,
    context: tcl_registry::f5::BigIpExecutionContext,
    event: tcl_registry::f5::naming::BigIpNameEvent,
    profile: &'static tcl_dialect::DialectProfile,
    engine: Option<&'static tcl_dialect::DialectProfile>,
}

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
        let Some(id) = self.child_id(child) else {
            return false;
        };
        self.in_interp(id, |vm| vm.register_native_command(name, command));
        true
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
        let previous_names = self.observed_names.take();
        let previous_storage = self.observed_frame_storage.take();
        if !self.set_observed_bigip_name_policy(build, context, event) {
            self.observed_names = previous_names;
            self.observed_frame_storage = previous_storage;
            self.invalidate_execution_name_policy();
            return Err(unavailable());
        }
        let result = body(self);
        self.observed_names = previous_names;
        self.observed_frame_storage = previous_storage;
        self.invalidate_execution_name_policy();
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
        self.observed_names = policy.map(|policy| ObservedNameSelection {
            policy,
            build,
            context,
            event,
            profile: self.dialect_profile,
            engine: self.actual_engine_profile,
        });
        self.observed_frame_storage = None;
        self.invalidate_execution_name_policy();
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
                && std::ptr::eq(selected.profile, self.dialect_profile)
                && selected.engine.map(|p| p as *const _)
                    == self.actual_engine_profile.map(|p| p as *const _))
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
        let previous = self.observed_frame_storage.replace(storage);
        self.invalidate_execution_name_policy();
        let result = body(self);
        self.observed_frame_storage = previous;
        self.invalidate_execution_name_policy();
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

    fn invalidate_execution_name_policy(&mut self) {
        self.name_world.borrow_mut().execution_name_policy = self.execution_name_policy();
        self.bump_cmd_epoch();
        self.bump_trace_deopt_epoch();
        self.invalidate_guard_domain(super::GuardDomain::Interpreter);
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
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
