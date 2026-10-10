// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite measured names over independently authored actual event-frame storage.

use super::Interp;
use crate::{frame::VarError, obj::TclObj};
use std::rc::Weak;
use tcl_runtime_api::authored_tmm::AuthoredObservedFrameStorageContext;
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
}

pub(super) struct ObservedFrameStorage {
    context: AuthoredObservedFrameStorageContext,
    owner: Weak<()>,
    index: usize,
    activation: u64,
    level: usize,
    interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
}

impl Interp {
    pub(crate) fn observed_name_policy_selected(&self) -> bool {
        self.observed_names.borrow().is_some()
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
        let previous_names = self.observed_names.borrow_mut().take();
        let previous_storage = self.observed_frame_storage.borrow_mut().take();
        if !self.set_observed_bigip_name_policy(build, context, event) {
            *self.observed_names.borrow_mut() = previous_names;
            *self.observed_frame_storage.borrow_mut() = previous_storage;
            self.invalidate_execution_name_policy();
            return Err(Self::observed_unavailable());
        }
        let result = body(self);
        *self.observed_names.borrow_mut() = previous_names;
        *self.observed_frame_storage.borrow_mut() = previous_storage;
        self.invalidate_execution_name_policy();
        Ok(result)
    }
    /// Retain an exact independently supplied build, feature and event issuer.
    pub fn set_observed_bigip_name_policy(
        &mut self,
        build: tcl_registry::f5::evidence::BigIpBuild,
        context: tcl_registry::f5::BigIpExecutionContext,
        event: tcl_registry::f5::naming::BigIpNameEvent,
    ) -> bool {
        let policy = tcl_registry::f5::naming::observed_name_policy(build, context, event);
        *self.observed_names.borrow_mut() = policy.map(|policy| ObservedNameSelection {
            policy,
            build,
            context,
            event,
            profile: self.dialect_profile.get(),
        });
        *self.observed_frame_storage.borrow_mut() = None;
        self.invalidate_execution_name_policy();
        policy.is_some()
    }

    #[must_use]
    pub fn execution_name_policy(&self) -> Option<ExecutionNamePolicy> {
        if let Some(selected) = self.observed_names.borrow().as_ref() {
            return (tcl_registry::f5::naming::observed_name_policy(
                selected.build,
                selected.context,
                selected.event,
            ) == Some(selected.policy)
                && std::ptr::eq(selected.profile, self.dialect_profile.get()))
            .then_some(ExecutionNamePolicy::ObservedBigIp(selected.policy));
        }
        self.name_policy_protocol()
            .map(ExecutionNamePolicy::NativeRecipe)
    }

    /// Select authored native simulation naming independently of the profile.
    pub fn set_logical_name_provider(
        &mut self,
        provider: tcl_syntax::naming::NamePolicyProtocol,
    ) -> bool {
        if self
            .native_invocation_dialect()
            .authored_logical_name_simulation(provider)
            .is_none()
        {
            return false;
        }
        self.logical_name_provider.set(Some(provider));
        self.namespaces.borrow_mut().variable_name_protocol =
            self.name_policy_protocol().map(|policy| policy.recipe());
        self.invalidate_execution_name_policy();
        true
    }

    /// Install counted-key storage on an independently entered actual frame.
    ///
    /// # Errors
    /// Refuses a global frame or absent, stale or mismatched measured issuer.
    pub fn with_observed_frame_storage<R>(
        &mut self,
        context: AuthoredObservedFrameStorageContext,
        body: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, ValueError> {
        if self.execution_name_policy() != Some(ExecutionNamePolicy::ObservedBigIp(context.policy))
        {
            return Err(Self::observed_unavailable());
        }
        let frames = self.frames.borrow();
        if frames.current_level() == 0 {
            return Err(Self::observed_unavailable());
        }
        let storage = ObservedFrameStorage {
            context,
            owner: frames.current_activation_owner(),
            index: frames.current_frame_index(),
            activation: frames.current_activation(),
            level: frames.current_level(),
            interpreter: self.native_command_interpreter,
        };
        drop(frames);
        let previous = self.observed_frame_storage.borrow_mut().replace(storage);
        self.invalidate_execution_name_policy();
        let result = body(self);
        *self.observed_frame_storage.borrow_mut() = previous;
        self.invalidate_execution_name_policy();
        Ok(result)
    }

    /// Enter a real event frame; retire its storage on every returned completion.
    ///
    /// # Errors
    /// Refuses an absent exact issuer before entering the event.
    pub fn with_observed_event_frame<R>(
        &mut self,
        context: AuthoredObservedFrameStorageContext,
        body: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, ValueError> {
        if self.execution_name_policy() != Some(ExecutionNamePolicy::ObservedBigIp(context.policy))
        {
            return Err(Self::observed_unavailable());
        }
        let level = self.frames.borrow_mut().push(self.current_ns.get());
        self.frames
            .borrow_mut()
            .table_mut(level)
            .expect("entered event frame")
            .set_hash_recipe(None);
        let owner = self.frames.borrow().current_activation_owner();
        let result = self.with_observed_frame_storage(context, body);
        let frame_is_current = {
            let frames = self.frames.borrow();
            frames.current_level() == level && owner.ptr_eq(&frames.current_activation_owner())
        };
        if frame_is_current {
            self.frames.borrow_mut().pop();
        }
        result
    }

    fn observed_unavailable() -> ValueError {
        ValueError::CommandProtocolUnavailable("observed event frame storage")
    }

    fn invalidate_execution_name_policy(&self) {
        self.namespaces.borrow_mut().execution_name_policy = self.execution_name_policy();
        self.invalidate_interpreter_policy();
        self.invalidate_command_environment();
    }

    fn observed_projection<'a>(
        &self,
        original: &'a [u8],
        level: usize,
        purpose: ObservedVariableNamePurpose,
    ) -> Result<ExecutionVariableNameProjection<'a>, VarError> {
        let policy = self.execution_name_policy();
        let storage = self.observed_frame_storage.borrow();
        let frames = self.frames.borrow();
        let admitted = storage.as_ref().is_some_and(|storage| {
            storage.interpreter == self.native_command_interpreter
                && storage.level == level
                && frames.current_level() == level
                && storage.index == frames.current_frame_index()
                && storage.activation == frames.current_activation()
                && storage.owner.ptr_eq(&frames.current_activation_owner())
                && storage.owner.upgrade().is_some()
                && policy == Some(ExecutionNamePolicy::ObservedBigIp(storage.context.policy))
        });
        if !admitted || self.has_variable_traces() {
            return self.observed_refusal();
        }
        policy
            .and_then(|policy| {
                policy
                    .variable_input(NativeVariableInputForm::Combined(original), purpose)
                    .ok()
            })
            .ok_or_else(|| {
                self.clone().refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "observed event variable purpose",
                    ),
                );
                VarError::NameProtocolUnavailable
            })
    }

    pub(crate) fn observed_refusal<T>(&self) -> Result<T, VarError> {
        self.clone().refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "observed event frame storage",
            ),
        );
        Err(VarError::NameProtocolUnavailable)
    }

    fn observed_receiver(
        &self,
        name: &[u8],
        level: usize,
        create: bool,
    ) -> Result<Option<crate::frame::VariableReceiver>, VarError> {
        let purpose = if name.ends_with(b")") {
            ObservedVariableNamePurpose::CombinedElement
        } else {
            ObservedVariableNamePurpose::ScalarReceiver
        };
        let projection = self.observed_projection(name, level, purpose)?;
        self.frames
            .borrow_mut()
            .table_mut(level)
            .ok_or(VarError::NameProtocolUnavailable)?
            .capture_authored_counted_receiver(
                projection.root(),
                projection.element().map(<[u8]>::to_vec),
                create,
            )
            .or_else(|_| self.observed_refusal())
    }

    pub(crate) fn observed_variable_get(
        &self,
        name: &[u8],
        level: usize,
    ) -> Result<Option<*mut TclObj>, VarError> {
        self.observed_receiver(name, level, false)?
            .map_or(Ok(None), |receiver| receiver.read())
            .or_else(|_| self.observed_refusal())
    }

    /// Complete original substitution names use the same measured current
    /// receiver. Separate root/index inputs and actual observers stay refused.
    pub(super) fn observed_substitution_receiver(
        &self,
        name: &[u8],
        index: Option<&[u8]>,
    ) -> Result<Option<*mut TclObj>, VarError> {
        // naming.variable.observed-original-substitution-receiver
        // docs/design/analysis/name-resolution-proofs/variable-observed-original-substitution-receiver.md
        if index.is_some() {
            return self.observed_refusal();
        }
        let level = self.frames.borrow().current_level();
        self.observed_variable_get(name, level)
    }

    pub(crate) fn observed_variable_exists(
        &self,
        name: &[u8],
        level: usize,
    ) -> Result<bool, VarError> {
        self.observed_receiver(name, level, false)?
            .map_or(Ok(false), |receiver| {
                if !name.ends_with(b")") && receiver.is_array() {
                    Ok(true)
                } else {
                    receiver.read().map(|value| value.is_some())
                }
            })
            .or_else(|_| self.observed_refusal())
    }

    pub(crate) fn observed_variable_set(
        &self,
        name: &[u8],
        level: usize,
        value: *mut TclObj,
    ) -> Result<(), VarError> {
        self.observed_receiver(name, level, true)?
            .ok_or(VarError::NameProtocolUnavailable)?
            .store(value)
            .or_else(|_| self.observed_refusal())
    }

    pub(crate) fn observed_variable_unset(
        &self,
        name: &[u8],
        level: usize,
    ) -> Result<bool, VarError> {
        self.observed_receiver(name, level, false)?
            .map_or(Ok(false), |receiver| receiver.unset())
            .or_else(|_| self.observed_refusal())
    }

    pub(crate) fn observed_array_target(
        &self,
        name: &[u8],
        level: usize,
    ) -> Result<tcl_runtime_api::ArrayTarget, ValueError> {
        let projection = self
            .observed_projection(name, level, ObservedVariableNamePurpose::ArrayRoot)
            .map_err(|_| Self::observed_unavailable())?;
        let frames = self.frames.borrow();
        let id = frames
            .table(level)
            .ok_or_else(Self::observed_unavailable)?
            .authored_counted_binding_id(projection.root())
            .map_err(|_| Self::observed_unavailable())?;
        Ok(id.map_or_else(
            || tcl_runtime_api::ArrayTarget::named_bytes(tcl_runtime_api::FrameId(level), name),
            |id| {
                tcl_runtime_api::ArrayTarget::cell_bytes(tcl_runtime_api::FrameId(level), name, id)
            },
        ))
    }

    pub(crate) fn observed_array_keys(
        &self,
        target: &tcl_runtime_api::ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, ValueError> {
        let projection = self
            .observed_projection(
                target.name_bytes(),
                target.frame().0,
                ObservedVariableNamePurpose::ArrayRoot,
            )
            .map_err(|_| Self::observed_unavailable())?;
        let frames = self.frames.borrow();
        let table = frames
            .table(target.frame().0)
            .ok_or_else(Self::observed_unavailable)?;
        if table
            .authored_counted_binding_id(projection.root())
            .map_err(|_| Self::observed_unavailable())?
            != target.cell_id()
        {
            return Err(Self::observed_unavailable());
        }
        table
            .authored_counted_array_keys(projection.root())
            .map_err(|_| Self::observed_unavailable())
    }

    pub(super) fn observed_ensure_array(&self, name: &[u8], level: usize) -> Result<(), VarError> {
        let projection =
            self.observed_projection(name, level, ObservedVariableNamePurpose::ArrayRoot)?;
        self.frames
            .borrow_mut()
            .table_mut(level)
            .ok_or(VarError::NameProtocolUnavailable)?
            .capture_authored_counted_receiver(projection.root(), None, true)?
            .ok_or(VarError::NameProtocolUnavailable)?
            .ensure_array()
            .or_else(|_| self.observed_refusal())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::f5::{evidence::BigIpBuild, naming::BigIpNameEvent, BigIpExecutionContext};

    fn select(interp: &mut Interp) -> AuthoredObservedFrameStorageContext {
        assert!(interp.set_observed_bigip_name_policy(
            BigIpBuild::MEASURED_21_1_0_1,
            BigIpExecutionContext::TmmIRule,
            BigIpNameEvent::HttpRequest
        ));
        let Some(ExecutionNamePolicy::ObservedBigIp(policy)) = interp.execution_name_policy()
        else {
            panic!("exact measured issuer");
        };
        AuthoredObservedFrameStorageContext {
            policy,
            tmm: 2,
            domain: 7,
        }
    }

    #[test]
    fn measured_names_use_actual_frame_and_retire_on_error_completion() {
        let mut interp = Interp::new();
        let context = select(&mut interp);
        let name = b"__tcl_lsp_2286_r2286m_nul_A\0B";
        assert!(interp.observed_variable_get(name, 0).is_err());
        let completion: Result<(), ()> = interp
            .with_observed_event_frame(context, |interp| {
                let level = interp.frames.borrow().current_level();
                let value = super::super::new_string(b"counted");
                interp.observed_variable_set(name, level, value).unwrap();
                assert_eq!(
                    interp.observed_variable_get(name, level).unwrap(),
                    Some(value)
                );
                assert!(interp.observed_variable_get(b"::arbitrary", level).is_err());
                assert!(interp.observed_variable_get(name, 0).is_err());
                assert!(interp.native_c_variable_name_protocol().is_none());
                let array = b"__tcl_lsp_2286_r2286m_arrayindex_root(A\0B)";
                interp.observed_variable_set(array, level, value).unwrap();
                assert_eq!(
                    interp.observed_variable_get(array, level).unwrap(),
                    Some(value)
                );
                assert!(interp.observed_variable_unset(array, level).unwrap());
                Err(())
            })
            .unwrap();
        assert_eq!(completion, Err(()));
        assert_eq!(interp.frames.borrow().current_level(), 0);
        assert!(interp.observed_variable_get(name, 0).is_err());
    }

    #[test]
    fn observed_counted_set_dispatch_uses_actual_entered_frame() {
        let mut interp = Interp::new();
        let context = select(&mut interp);
        interp.with_observed_event_frame(context, |interp| {
            assert_eq!(interp.eval_str(b"set {__tcl_lsp_2286_r2286m_nul_A\0B} counted; set {__tcl_lsp_2286_r2286m_nul_A\0B}"), super::super::Code::Ok);
            assert_eq!(interp.eval_str(b"set {__tcl_lsp_2286_r2286m_arrayindex_root(A\0B)} element; set {__tcl_lsp_2286_r2286m_arrayindex_root(A\0B)}"), super::super::Code::Ok);
        }).unwrap();
    }

    #[test]
    fn original_object_assignments_use_the_current_observed_receiver() {
        // naming.variable.observed-original-object-assignment
        // docs/design/analysis/name-resolution-proofs/variable-observed-original-object-assignment.md
        for (prefix, spelling) in [
            (b"A".as_slice(), b"A\0B".as_slice()),
            (b"R(k)".as_slice(), b"R\0T(k)".as_slice()),
        ] {
            let mut interp = Interp::new();
            let context = select(&mut interp);
            let plain = crate::obj::Owned::fresh(super::super::new_string(b"PLAIN"));
            let counted = crate::obj::Owned::fresh(super::super::new_string(b"COUNTED"));
            let prefix = crate::obj::Owned::fresh(super::super::new_string(prefix));
            let original = crate::obj::Owned::fresh(super::super::new_string(spelling));
            interp
                .with_observed_event_frame(context, |interp| {
                    interp.set_result_bytes(b"before");
                    interp
                        .assign_original_named_variable(prefix.as_ptr(), plain.as_ptr())
                        .unwrap();
                    interp
                        .assign_original_named_variable(original.as_ptr(), counted.as_ptr())
                        .unwrap();
                    assert_eq!(interp.result_bytes(), b"before");
                    assert_eq!(
                        interp
                            .read_original_named_variable(original.as_ptr())
                            .unwrap(),
                        counted.as_ptr(),
                    );
                    assert_eq!(
                        interp
                            .read_original_named_variable(prefix.as_ptr())
                            .unwrap(),
                        plain.as_ptr(),
                    );
                    interp
                        .store_original_named_variable(original.as_ptr(), counted.as_ptr())
                        .unwrap();
                    assert_eq!(interp.result_bytes(), b"COUNTED");
                    assert!(interp.native_c_variable_name_protocol().is_none());
                })
                .unwrap();
            assert!(
                interp
                    .assign_original_named_variable(original.as_ptr(), counted.as_ptr())
                    .is_err(),
                "the retired event frame cannot supply an original receiver",
            );
            assert_eq!(interp.frames.borrow().current_level(), 0);
        }
    }

    #[test]
    fn counted_array_root_and_scalar_queries_use_normal_source_dispatch() {
        let mut interp = Interp::new();
        let context = select(&mut interp);
        interp.with_observed_event_frame(context, |interp| {
            let source = b"set {R\0T(k)} element; list [array exists {R\0T}] [array size {R\0T}] [array names {R\0T}] [info exists {R\0T(k)}] [info exists {R\0T(missing)}]";
            let code = interp.eval_str(source);
            assert_eq!(code, super::super::Code::Ok, "source={source:?} result={:?} admission={:?} refusal={:?}", interp.result_bytes(), interp.native_compilation_admission_error(), interp.native_access_refusal());
            assert_eq!(interp.result_bytes(), b"1 1 k 1 0");
            let source = b"set {S\0T} scalar; set copied ${S\0T}; unset {S\0T}; list $copied [info exists {S\0T}]";
            let code = interp.eval_str(source);
            assert_eq!(code, super::super::Code::Ok, "source={source:?} result={:?} admission={:?} refusal={:?}", interp.result_bytes(), interp.native_compilation_admission_error(), interp.native_access_refusal());
            assert_eq!(interp.result_bytes(), b"scalar 0");
        }).unwrap();
    }

    #[test]
    fn original_observed_substitutions_keep_counted_cells_and_input_forms_separate() {
        // naming.variable.observed-original-substitution-receiver
        // docs/design/analysis/name-resolution-proofs/variable-observed-original-substitution-receiver.md
        let mut interp = Interp::new();
        let context = select(&mut interp);
        interp
            .with_observed_event_frame(context, |interp| {
                for source in [
                    b"set S prefix; set {S\0T} counted; list $S ${S\0T}".as_slice(),
                    b"set S prefix; set {S\0T} counted; list x${S\0T}x".as_slice(),
                ] {
                    assert_eq!(
                        interp.eval_str(source),
                        super::super::Code::Ok,
                        "source={source:?} result={:?} refusal={:?}",
                        interp.result_bytes(),
                        interp.native_access_refusal()
                    );
                    let expected = if source.ends_with(b"${S\0T}") {
                        b"prefix counted".as_slice()
                    } else {
                        b"xcountedx".as_slice()
                    };
                    assert_eq!(interp.result_bytes(), expected);
                    assert!(interp.native_c_variable_name_protocol().is_none());
                }
                assert_eq!(interp.fire_read_trace(b"S\0T", None), None);
                assert_eq!(interp.read_var(b"S\0T", None), Some(b"counted".to_vec()));
                assert_eq!(
                    interp.fire_read_trace(b"S\0T", Some(b"k")),
                    Some(super::super::Code::Error)
                );
                assert!(interp.native_access_refusal().is_some());
                assert!(interp.read_var(b"S\0T", Some(b"k")).is_none());
            })
            .unwrap();
        assert_eq!(
            interp.fire_read_trace(b"S\0T", None),
            Some(super::super::Code::Error)
        );
        assert!(interp.read_var(b"S\0T", None).is_none());
    }

    #[test]
    fn counted_dynamic_cell_does_not_collide_with_declared_cstring_prefix() {
        let mut interp = Interp::new();
        let context = select(&mut interp);
        interp
            .with_observed_event_frame(context, |interp| {
                let level = interp.frames.borrow().current_level();
                interp.frames.borrow_mut().install_formal_cells(
                    &[b"A".to_vec()],
                    tcl_syntax::naming::NativeCompiledVariableProtocol::authored_tcl(
                        tcl_dialect::TclVersion::V8_4,
                    ),
                );
                let formal = super::super::new_string(b"formal");
                interp
                    .frames
                    .borrow_mut()
                    .store_formal_cell(0, formal)
                    .unwrap();
                interp
                    .observed_variable_set(b"A\0B", level, super::super::new_string(b"counted"))
                    .unwrap();
                assert!(interp
                    .observed_variable_set(b"A", level, super::super::new_string(b"replacement"))
                    .is_err());
                let frames = interp.frames.borrow();
                let Some(slot) = frames.compiled_slot_var(0) else {
                    panic!("retained formal cell")
                };
                let crate::frame::Var::Scalar(value) = &*slot else {
                    panic!("retained formal value")
                };
                assert_eq!(super::super::obj_bytes(*value), b"formal");
            })
            .unwrap();
    }

    #[test]
    fn observed_trace_and_upvar_purposes_are_unavailable_in_source_dispatch() {
        for source in [
            b"trace add variable A write {set callback}".as_slice(),
            b"upvar 0 A alias".as_slice(),
        ] {
            let mut interp = Interp::new();
            let context = select(&mut interp);
            interp
                .with_observed_event_frame(context, |interp| {
                    assert_eq!(interp.eval_str(source), super::super::Code::Error);
                    assert!(interp.native_access_refusal().is_some());
                })
                .unwrap();
        }
    }

    #[test]
    fn scoped_policy_restores_after_error_completion() {
        let mut interp = Interp::new();
        let before = interp.execution_name_policy();
        let result: Result<(), ()> = interp
            .with_observed_name_policy(
                BigIpBuild::MEASURED_21_1_0_1,
                BigIpExecutionContext::TmmIRule,
                BigIpNameEvent::HttpRequest,
                |interp| {
                    assert_ne!(interp.execution_name_policy(), before);
                    Err(())
                },
            )
            .unwrap();
        assert_eq!(result, Err(()));
        assert_eq!(interp.execution_name_policy(), before);
    }

    #[test]
    fn replacement_activation_and_changed_profile_do_not_inherit_capability() {
        let mut interp = Interp::new();
        let context = select(&mut interp);
        interp.frames.borrow_mut().push(crate::namespace::GLOBAL);
        interp
            .with_observed_frame_storage(context, |interp| {
                interp.frames.borrow_mut().pop();
                let level = interp.frames.borrow_mut().push(crate::namespace::GLOBAL);
                assert!(interp
                    .observed_variable_get(b"__tcl_lsp_2286_r2286m_nul_A", level)
                    .is_err());
            })
            .unwrap();
        interp.frames.borrow_mut().pop();
        interp.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        assert!(interp.execution_name_policy().is_none());
    }
}
