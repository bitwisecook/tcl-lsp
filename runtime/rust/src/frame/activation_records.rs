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

//! What an activation retains: the running procedure and its native execution role, Jim frame objects, and the TclOO variable resolver.

use super::{FrameStack, OriginalJimVariadic, RetiredJimFrame, VarError};
use crate::namespace::NsId;
use crate::obj;
use std::rc::Rc;

impl FrameStack {
    pub(crate) fn recycle_jim_storage(
        &mut self,
        storage: tcl_runtime_api::jim_call_frame::JimCallFrameStorageSlot,
    ) {
        if self.variable_lookup_policy == tcl_dialect::VariableLookupPolicy::Jim {
            self.jim_storage.recycle(storage);
        }
    }

    pub(crate) fn jim_link_storage(
        &self,
        level: usize,
    ) -> Option<tcl_runtime_api::jim_call_frame::JimCallFrameStorageReference> {
        Some(
            self.frames
                .get(self.index_of_level(level)?)?
                .jim_storage
                .reference(),
        )
    }

    pub(crate) fn jim_link_level(
        &self,
        retained: &tcl_runtime_api::jim_call_frame::JimCallFrameStorageReference,
    ) -> Option<usize> {
        let frame = self
            .frames
            .iter()
            .find(|frame| frame.jim_storage.matches(retained))?;
        self.frames
            .get(self.index_of_level(frame.level)?)
            .filter(|selected| selected.jim_storage.matches(retained))
            .map(|_| frame.level)
    }

    pub(crate) fn retain_c_procedure(&mut self, procedure: Rc<crate::interp::ProcDef>) {
        self.frames
            .last_mut()
            .expect("actual C activation")
            .c_procedure = Some(procedure);
    }

    pub(crate) fn retain_native_procedure_execution(
        &mut self,
        role: tcl_runtime_api::native_procedure_roles::NativeProcedureReference<
            crate::interp::ProcDef,
        >,
    ) {
        let frame = self.frames.last_mut().expect("actual procedure activation");
        assert!(frame.native_procedure_execution.is_none());
        frame.native_procedure_execution = Some(role);
    }

    pub(crate) fn take_native_procedure_execution(
        &mut self,
    ) -> Option<
        tcl_runtime_api::native_procedure_roles::NativeProcedureReference<crate::interp::ProcDef>,
    > {
        self.frames.last_mut()?.native_procedure_execution.take()
    }

    /// WORK-only copied live owner identities; no retained execution authority.
    pub(crate) fn codegen_depth_owner_ids(&self) -> Option<(usize, Option<usize>, Option<bool>)> {
        let frame = self.frames.get(self.current_frame_index())?;
        let procedure = frame.c_procedure.as_ref();
        let role_matches = procedure
            .zip(frame.native_procedure_execution.as_ref())
            .map(|(procedure, role)| Rc::ptr_eq(procedure, role.owner()));
        Some((
            Rc::as_ptr(&frame.jim_link_birth) as usize,
            procedure.map(|procedure| Rc::as_ptr(procedure) as usize),
            role_matches,
        ))
    }

    pub(crate) fn current_c_procedure(&self) -> Option<&Rc<crate::interp::ProcDef>> {
        self.frames
            .get(self.current_frame_index())?
            .c_procedure
            .as_ref()
    }

    pub(crate) fn observe_original_jim_procedure(
        &mut self,
        procedure: &Rc<crate::interp::ProcDef>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        let frame = self
            .frames
            .last_mut()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "actual Jim procedure activation",
            ))?;
        let body = frame
            .jim_body
            .as_ref()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "actual Jim procedure body role",
            ))?;
        obj::check_native_liveness(body.as_ptr())?;
        if procedure.body.checked_ptr()? != body.as_ptr() {
            return Err(ValueError::CommandProtocolUnavailable(
                "actual Jim procedure body owner",
            ));
        }
        frame.original_jim_procedure = Some(Rc::downgrade(procedure));
        Ok(())
    }

    pub(crate) fn current_original_jim_procedure(
        &self,
    ) -> Result<Option<Rc<crate::interp::ProcDef>>, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        let Some(frame) = self.frames.get(self.current_frame_index()) else {
            return Ok(None);
        };
        let Some(owner) = frame.original_jim_procedure.as_ref() else {
            return Ok(None);
        };
        let owner = owner
            .upgrade()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "retired actual Jim procedure owner",
            ))?;
        let body = frame
            .jim_body
            .as_ref()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "actual Jim procedure body role",
            ))?;
        obj::check_native_liveness(body.as_ptr())?;
        if owner.body.checked_ptr()? != body.as_ptr() {
            return Err(ValueError::CommandProtocolUnavailable(
                "actual Jim procedure body owner",
            ));
        }
        Ok(Some(owner))
    }

    pub(crate) fn observe_original_jim_variadic(
        &mut self,
        original: OriginalJimVariadic,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        original.original()?;
        let frame = self
            .frames
            .last_mut()
            .expect("actual Jim formal binding frame");
        if frame.original_jim_procedure.is_some() {
            frame.original_jim_variadic = Some(original);
        }
        Ok(())
    }

    pub(crate) fn current_original_jim_variadic(
        &self,
    ) -> Result<Option<*mut obj::TclObj>, tcl_syntax::value::ValueError> {
        if self.current_original_jim_procedure()?.is_none() {
            return Ok(None);
        }
        self.frames
            .get(self.current_frame_index())
            .and_then(|frame| frame.original_jim_variadic.as_ref())
            .map(OriginalJimVariadic::original)
            .transpose()
    }

    pub(crate) fn observe_original_jim_import_traversal(
        &mut self,
        traversal: Rc<crate::interp::OriginalJimImportTraversal>,
    ) {
        self.frames
            .last_mut()
            .expect("actual Jim activation")
            .original_jim_import = Some(traversal);
    }

    pub(crate) fn observe_original_namespace_info_frame(
        &mut self,
        frame: Rc<crate::interp::OriginalNamespaceInfoFrame>,
    ) {
        self.frames
            .last_mut()
            .expect("actual Jim procedure activation")
            .original_namespace_info = Some(frame);
    }

    pub(crate) fn current_original_namespace_info_frame(
        &self,
    ) -> Result<Option<Rc<crate::interp::OriginalNamespaceInfoFrame>>, tcl_syntax::value::ValueError>
    {
        let frame = self
            .frames
            .get(self.current_frame_index())
            .and_then(|frame| frame.original_namespace_info.as_ref())
            .cloned();
        if frame.is_some() && self.current_original_jim_procedure()?.is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original NamespaceInfo selected procedure frame",
            ));
        }
        Ok(frame)
    }

    pub(crate) fn current_original_jim_import_traversal(
        &self,
    ) -> Result<Option<Rc<crate::interp::OriginalJimImportTraversal>>, tcl_syntax::value::ValueError>
    {
        let traversal = self
            .frames
            .get(self.current_frame_index())
            .and_then(|frame| frame.original_jim_import.as_ref())
            .cloned();
        if traversal.is_some() && self.current_original_jim_procedure()?.is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Import selected procedure frame",
            ));
        }
        Ok(traversal)
    }

    #[cfg(test)]
    pub(crate) fn jim_activation_objects(
        &self,
    ) -> Option<(
        *mut crate::obj::TclObj,
        *mut crate::obj::TclObj,
        *mut crate::obj::TclObj,
    )> {
        let frame = self.frames.last()?;
        Some((
            frame.jim_parameters.as_ref()?.as_ptr(),
            frame.jim_body.as_ref()?.as_ptr(),
            frame.jim_namespace.as_ref()?.as_ptr(),
        ))
    }

    pub(crate) fn retain_jim_frame_body(&mut self, body: crate::obj::Owned) {
        self.frames
            .last_mut()
            .expect("native namespace frame")
            .jim_body = Some(body);
    }

    pub(crate) fn retain_jim_root_namespace(&mut self, namespace: Rc<crate::obj::Owned>) {
        self.frames.first_mut().expect("global frame").jim_namespace = Some(namespace);
    }

    pub(crate) fn retain_jim_activation_objects(
        &mut self,
        parameters: Option<crate::obj::Owned>,
        body: Option<crate::obj::Owned>,
        namespace: Rc<crate::obj::Owned>,
    ) {
        let frame = self.frames.last_mut().expect("actual activation frame");
        frame.jim_parameters = parameters;
        frame.jim_body = body;
        frame.jim_namespace = Some(namespace);
    }

    pub(crate) fn take_frame_for_pop(&mut self) -> Option<(NsId, RetiredJimFrame)> {
        if self.frames.len() <= 1 {
            return None;
        }
        let frame = self.frames.pop().expect("non-global frame");
        self.active_level = frame.saved_active;
        Some((frame.ns, RetiredJimFrame { _frame: frame }))
    }

    pub(crate) fn select_top_for_jim_retirement(&mut self) -> Option<usize> {
        let level = self.frames.last()?.level;
        self.active_level = level;
        Some(level)
    }

    pub(crate) fn take_top_for_jim_retirement(&mut self) -> Option<RetiredJimFrame> {
        let frame = self.frames.pop()?;
        self.active_level = self.frames.last().map_or(0, |frame| frame.level);
        Some(RetiredJimFrame { _frame: frame })
    }

    pub(crate) fn retain_jim_local_command(&mut self, name: crate::obj::Owned) {
        let index = self.current_frame_index();
        self.frames[index].jim_local_commands.push(name);
    }

    pub(crate) fn take_jim_local_commands(&mut self) -> Vec<crate::obj::Owned> {
        let index = self.current_frame_index();
        std::mem::take(&mut self.frames[index].jim_local_commands)
    }

    /// Install the resolver only in the actual reached method activation.
    pub(crate) fn install_tcloo_variable_resolver(
        &mut self,
        resolver: crate::cmd_oo::native_variables::NativeOoVariableResolver,
    ) -> Result<(), VarError> {
        let frame = self
            .frames
            .last_mut()
            .ok_or(VarError::NameProtocolUnavailable)?;
        if !frame.is_proc || frame.ns != resolver.namespace() {
            return Err(VarError::NameProtocolUnavailable);
        }
        frame.oo_variable_resolver = Some(resolver);
        Ok(())
    }

    /// Runtime CString resolution is independent of the installed local table.
    pub(crate) fn tcloo_variable_target(
        &self,
        level: usize,
        protocol: tcl_syntax::naming::NativeNameProtocol,
        supplied: &[u8],
    ) -> Result<Option<(NsId, Vec<u8>)>, VarError> {
        let Some(frame) = self
            .index_of_level(level)
            .and_then(|index| self.frames.get(index))
        else {
            return Err(VarError::NameProtocolUnavailable);
        };
        let Some(resolver) = &frame.oo_variable_resolver else {
            return Ok(None);
        };
        if !frame.is_proc || frame.ns != resolver.namespace() {
            return Err(VarError::NameProtocolUnavailable);
        }
        resolver
            .runtime_target(protocol, supplied)
            .map_err(|_| VarError::NameProtocolUnavailable)
    }

    pub(crate) fn declared_tcloo_variable_bindings(
        &self,
    ) -> Result<Vec<crate::cmd_oo::native_variables::DeclaredVariableBinding>, VarError> {
        let Some(frame) = self
            .index_of_level(self.active_level)
            .and_then(|index| self.frames.get(index))
        else {
            return Err(VarError::NameProtocolUnavailable);
        };
        frame.oo_variable_resolver.as_ref().map_or_else(
            || Ok(Vec::new()),
            |resolver| {
                if !frame.is_proc || frame.ns != resolver.namespace() {
                    return Err(VarError::NameProtocolUnavailable);
                }
                resolver
                    .reported_bindings()
                    .map_err(|_| VarError::NameProtocolUnavailable)
            },
        )
    }

    pub(crate) fn declared_tcloo_variable_names(&self) -> Result<Vec<Vec<u8>>, VarError> {
        self.declared_tcloo_variable_bindings()
            .map(|bindings| bindings.into_iter().map(|(_, name, _)| name).collect())
    }
}

impl FrameStack {
    #[cfg(have_tommath)]
    pub(crate) fn native_i64_execution_matches(
        &self,
        procedure: &Rc<crate::interp::ProcDef>,
        original: &std::rc::Weak<()>,
    ) -> bool {
        let index = self.current_frame_index();
        let Some(frame) = self.frames.get(index) else {
            return false;
        };
        index + 1 == self.frames.len()
            && std::rc::Weak::ptr_eq(original, &Rc::downgrade(&frame.jim_link_birth))
            && frame
                .c_procedure
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, procedure))
            && frame
                .native_procedure_execution
                .as_ref()
                .is_some_and(|role| Rc::ptr_eq(role.owner(), procedure))
    }

    #[cfg(have_tommath)]
    pub(crate) fn quiet_native_i64_frame(
        &self,
        procedure: &Rc<crate::interp::ProcDef>,
        slots: [usize; 2],
        protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
    ) -> Option<std::rc::Weak<()>> {
        let frame = self.current_activation_owner();
        if !self.native_i64_execution_matches(procedure, &frame) {
            crate::interp::native_i64_body::observe("frame-owner", None);
            return None;
        }
        let mut integers = [0; 2];
        for (index, slot) in slots.into_iter().enumerate() {
            if self.compiled_slot_name(slot)? != procedure.params[index].name.as_slice() {
                return None;
            }
            let (frame, cell) = self.compiled_cell(slot)?;
            let contents = self.frames[frame]
                .table
                .cells
                .get(cell)?
                .contents
                .try_borrow()
                .ok()?;
            contents.check_owned_value_role().ok()?;
            let Var::Scalar(value) = contents.var.as_ref()? else {
                return None;
            };
            if !contents.compiled_declaration
                || contents.operation_refs != 0
                || contents.native_alias_refs != 0
                || contents.rmw_refs != 0
            {
                crate::interp::native_i64_body::observe("frame-quiet-cell", None);
                return None;
            }
            crate::obj::check_native_liveness(*value).ok()?;
            let descriptor = crate::obj::obj_type_ptr(*value);
            integers[index] = if core::ptr::eq(descriptor, &crate::obj::TCL_INT_TYPE) {
                crate::obj::wide_of(*value)
            } else if descriptor.is_null() && crate::obj::has_string_rep(*value) {
                // The actual scalar cell retains this plain resident header;
                // no getter/updater runs while testing candidate eligibility.
                // SAFETY: the checked scalar cell owns this live allocation.
                let length = usize::try_from(unsafe { (**value).length }).ok()?;
                if length > 64 {
                    return None;
                }
                // SAFETY: the checked live scalar owns this non-null resident
                // buffer for its bounded cached length; no callback runs here.
                let bytes =
                    unsafe { core::slice::from_raw_parts((**value).bytes.cast::<u8>(), length) };
                let digits = bytes
                    .strip_prefix(b"+")
                    .or_else(|| bytes.strip_prefix(b"-"))
                    .unwrap_or(bytes);
                if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
                    return None;
                }
                let conversion = protocol.fresh_number_conversion(
                    tcl_syntax::scalar_getter::NativeNumberGetterKind::Number,
                    bytes,
                )?;
                let Ok(tcl_syntax::number::Number::Int(integer)) = conversion.outcome() else {
                    return None;
                };
                *integer
            } else {
                crate::interp::native_i64_body::observe("frame-operand-type", None);
                return None;
            };
        }
        integers[0].checked_add(integers[1])?;
        Some(frame)
    }
}
