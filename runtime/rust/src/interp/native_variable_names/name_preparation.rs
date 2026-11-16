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

//! Preparing an original name object: parsing its parts once and selecting the cell it names, by purpose, namespace or byte parts.

use super::super::{Code, Interp};
use super::{OriginalCNameSelection, OriginalCNameStage};
use crate::obj;
use crate::obj::{Owned, TclObj};
use tcl_syntax::native_variable_name::{
    NativeParsedVariableElement, NativeParsedVariableName, NativeVariableNameLookupPurpose,
};
use tcl_syntax::value::ValueError;

impl Interp {
    pub(super) fn prepare_original_c_name(
        &mut self,
        original: *mut TclObj,
        create: bool,
    ) -> Result<OriginalCNameSelection, Code> {
        self.prepare_original_c_name_using(original, create, None)
    }

    pub(super) fn prepare_original_c_name_using(
        &mut self,
        original: *mut TclObj,
        create: bool,
        stage: OriginalCNameStage<'_>,
    ) -> Result<OriginalCNameSelection, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let purpose = if create {
            NativeVariableNameLookupPurpose::Write
        } else {
            NativeVariableNameLookupPurpose::Read
        };
        self.prepare_original_c_name_for_using(original, purpose, stage)?
            .ok_or_else(|| {
                let bytes = self.original_c_name_bytes_using(original, stage);
                match bytes {
                    Ok(bytes) => self.no_such_variable_using(&bytes, None, stage),
                    Err(error) => self.report_cmd_error_using(error, stage),
                }
            })
    }

    pub(super) fn prepare_original_c_name_for(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.prepare_original_c_name_for_using(original, purpose, None)
    }

    pub(super) fn prepare_original_c_name_for_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        self.prepare_original_c_name_in_using(original, purpose, None, stage)
    }

    pub(super) fn prepare_original_c_name_in(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<crate::namespace::NsId>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.prepare_original_c_name_in_using(original, purpose, namespace, None)
    }

    pub(super) fn prepare_original_c_name_in_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<crate::namespace::NsId>,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        self.prepare_original_c_name_parts_using(original, purpose, namespace, None, stage)
    }

    pub(super) fn prepare_original_c_name_parts(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<crate::namespace::NsId>,
        explicit_element: Option<&[u8]>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.prepare_original_c_name_parts_using(
            original,
            purpose,
            namespace,
            explicit_element,
            None,
        )
    }

    fn prepare_original_c_name_parts_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<crate::namespace::NsId>,
        explicit_element: Option<&[u8]>,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let operation = super::super::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        self.check_original_c_name_operation_using(&[original], stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let _original_lifetime = obj::NativeObjectLifetime::retain(original);
        let create = purpose.creates_entries();
        let dialect = self.native_invocation_dialect();
        let protocol = dialect
            .native_variable_name_protocol()
            .expect("C variable issuer");
        if !protocol.uses_original_name_cache(purpose) {
            return self.prepare_original_c_byte_name_parts_using(
                original,
                namespace,
                explicit_element,
                stage,
            );
        }
        if !protocol.cache_precedes_string_getter() {
            self.original_c_name_bytes_using(original, stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
        }
        // Project only allocation leases/bytes while the original Parsed owner
        // is borrowed. The reached element getter runs after that projection.
        let projected = explicit_element
            .is_none()
            .then(|| {
                obj::native_variable_name::with_parsed(original, |cache| {
                    if cache.protocol != protocol {
                        return None;
                    }
                    cache.array.as_ref().map(|(root, element)| {
                        let (bytes, object) = match element {
                            NativeParsedVariableElement::Bytes(bytes) => {
                                (Some(tcl_core_types::c_string_extent(bytes).to_vec()), None)
                            }
                            NativeParsedVariableElement::Object(object) => (
                                None,
                                Some(obj::NativeObjectLifetime::retain(object.as_ptr())),
                            ),
                        };
                        (
                            obj::NativeObjectLifetime::retain(root.as_ptr()),
                            bytes,
                            object,
                        )
                    })
                })
            })
            .flatten()
            .flatten();
        let parsed_primary = if stage.is_some() && projected.is_some() {
            Some(
                obj::NativePrimaryReceipt::capture(original).map_err(|error| {
                    self.report_cmd_error_using(
                        tcl_cmd_core::CmdError::from_execution_refusal(
                            self.clone().refuse_completion_value_access(error),
                        ),
                        stage,
                    )
                })?,
            )
        } else {
            None
        };
        let actual = self.clone();
        let original_stage = stage;
        let parsed_stage = || {
            actual.check_original_c_name_stage(original_stage)?;
            if let Some(primary) = &parsed_primary {
                primary
                    .check_for(original, "changed original Parsed variable primary")
                    .map_err(|error| {
                        tcl_cmd_core::CmdError::from_execution_refusal(
                            actual.clone().refuse_completion_value_access(error),
                        )
                    })?;
            }
            Ok(())
        };
        let stage = stage.map(|_| &parsed_stage as &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>);
        let mut parts = if let Some((root, bytes, element)) = &projected {
            let bytes = match bytes {
                Some(bytes) => bytes.clone(),
                None => self
                    .original_c_name_bytes_using(
                        element
                            .as_ref()
                            .expect("projected original element")
                            .as_ptr(),
                        stage,
                    )
                    .map_err(|error| self.report_cmd_error_using(error, stage))?
                    .to_vec(),
            };
            self.check_original_c_name_stage(stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            if let Some(primary) = &parsed_primary {
                primary
                    .check_for(original, "changed original Parsed variable primary")
                    .map_err(|error| {
                        let first = self.refuse_completion_value_access(error);
                        self.report_cmd_error_using(
                            tcl_cmd_core::CmdError::from_execution_refusal(first),
                            stage,
                        )
                    })?;
                self.check_original_c_name_operation_using(&[original, root.as_ptr()], stage)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
            }
            Some((root.as_ptr(), bytes))
        } else {
            None
        };
        let parsed_scalar = obj::native_variable_name::with_parsed(original, |cache| {
            cache.protocol == protocol && cache.array.is_none()
        })
        .unwrap_or(false);
        if explicit_element.is_none()
            && parts.is_none()
            && !parsed_scalar
            && obj::native_variable_name::with_local(original, |_| ()).is_none()
        {
            let original_bytes = self
                .original_c_name_bytes_using(original, stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            if let Some((root, element)) = protocol.parsed_array_parts(&original_bytes) {
                let root = Owned::fresh(obj::new_string_bytes(root));
                let element = match protocol.new_element_bytes(element) {
                    Some(bytes) => NativeParsedVariableElement::Bytes(bytes),
                    None => NativeParsedVariableElement::Object(Owned::fresh(
                        obj::new_string_bytes(element),
                    )),
                };
                let mut current = || {
                    operation
                        .currency()
                        .ensure_current_or_refuse()
                        .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
                        .and_then(|()| {
                            self.check_original_c_name_operation_using(&[original], stage)
                        })
                };
                let retain = |error: &tcl_cmd_core::CmdError| {
                    if let Some(first) = error.native_execution_refusal() {
                        self.clone().refuse_native_execution(first.clone());
                    }
                };
                let result = obj::native_variable_name::install_parsed_with_stage_check(
                    original,
                    NativeParsedVariableName {
                        protocol,
                        array: Some((root, element)),
                    },
                    dialect,
                    &mut current,
                    &retain,
                );
                self.check_original_c_name_operation_using(&[original], stage)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                result.map_err(|error| self.report_cmd_error_using(error, stage))?;
                return self.prepare_original_c_name_in_using(original, purpose, namespace, stage);
            }
        }
        let (root, element) = parts.take().map_or(
            (original, explicit_element.map(<[u8]>::to_vec)),
            |(root, element)| (root, Some(element)),
        );
        let cached = namespace
            .is_none()
            .then(|| self.original_c_local_index(root))
            .flatten();
        let root_bytes = if let Some(index) = cached {
            self.frames
                .borrow()
                .compiled_slot_name(index)
                .expect("authenticated actual local cell")
                .to_vec()
        } else {
            let root_bytes = self
                .original_c_name_bytes_using(root, stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?
                .to_vec();
            let kind = obj::obj_type_ptr(root);
            // SAFETY: a non-NULL type pointer names the live original descriptor.
            let free_hook = !kind.is_null() && unsafe { (*kind).free_int_rep_proc.is_some() };
            if protocol.retires_before_simple_lookup(free_hook) {
                let _root_lifetime = obj::NativeObjectLifetime::retain(root);
                let mut current = || {
                    operation
                        .currency()
                        .ensure_current_or_refuse()
                        .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
                        .and_then(|()| {
                            self.check_original_c_name_operation_using(&[original, root], stage)
                        })
                };
                let result = obj::discard_native_internal_representation_with_stage_check(
                    root,
                    &mut current,
                );
                self.check_original_c_name_operation_using(&[original, root], stage)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                result.map_err(|error| self.report_cmd_error_using(error, stage))?;
            }
            root_bytes
        };
        let selected = if let Some(index) = cached {
            crate::vars::prepare_original_c_indexed_name_cell(
                &mut self.frames.borrow_mut(),
                &self.namespaces.borrow(),
                index,
            )
            .map(Some)
        } else {
            crate::vars::prepare_original_c_variable_cell(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                crate::vars::NativeOriginalNameScope {
                    current: self.current_ns.get(),
                    namespace,
                },
                &root_bytes,
                root,
                create,
                protocol.version(),
            )
        };
        let selected = match selected {
            Ok(Some(selected)) => selected,
            Ok(None) => return Ok(None),
            Err(crate::frame::VarError::NameProtocolUnavailable) => {
                return Err(self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("actual original simple variable cell")
                        .into(),
                    stage,
                ));
            }
            Err(error) if create => {
                let reason = match error {
                    crate::frame::VarError::NoSuchNamespace => {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::MissingParentNamespace
                    }
                    crate::frame::VarError::IsScalar => {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::NotArray
                    }
                    _ => {
                        return Err(
                            self.original_c_lookup_var_error_using(original, purpose, error, stage)
                        );
                    }
                };
                return Err(self.original_c_variable_failure_for_object_using(
                    original,
                    purpose,
                    reason,
                    tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                    stage,
                ));
            }
            Err(_) => return Ok(None),
        };
        if cached.is_none() {
            self.install_original_c_name_cache_using(root, &selected, operation.currency(), stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
        }
        let mut materialised = None;
        if create
            && selected.compiled.is_none()
            && (protocol.version() >= tcl_dialect::TclVersion::V8_5
                || purpose == NativeVariableNameLookupPurpose::Define)
        {
            if let Some(element) = &element {
                let operation =
                    super::super::native_operation_currency::NativeOperationScope::enter(self)
                        .map_err(|error| {
                            self.report_cmd_error_using(
                                tcl_cmd_core::CmdError::from_execution_refusal(error),
                                stage,
                            )
                        })?;
                let retained_key = self.original_c_parsed_element_key(original);
                selected
                    .check(self, operation.currency())
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                let prepared = crate::vars::capture_original_c_selected_element_materialisation(
                    &mut self.frames.borrow_mut(),
                    &mut self.namespaces.borrow_mut(),
                    &selected,
                    element,
                    retained_key.as_ref().map(Owned::as_ptr),
                    purpose.creates_element_entries(),
                    protocol,
                );
                let reached = match prepared {
                    Ok(prepared) => prepared
                        .finish_using(self, operation.currency(), stage)
                        .map_err(Ok),
                    Err(crate::frame::JimLinkStoreError::Variable(error)) => Err(Err(error)),
                    Err(crate::frame::JimLinkStoreError::Command(error)) => Err(Ok(error)),
                };
                if let Err(Ok(error)) = &reached {
                    if let Some(first) = error.native_execution_refusal() {
                        self.refuse_native_execution(first.clone());
                    }
                }
                // The real parsed-key temporary retires after its selected
                // preparation, before any ordinary diagnostic continuation.
                drop(retained_key);
                operation
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(|first| self.refuse_native_execution(first))?;
                self.check_original_c_name_operation_using(&[original, root], stage)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                materialised = Some(reached.map_err(|error| {
                    let error = match error {
                        Ok(error) => return self.report_cmd_error_using(error, stage),
                        Err(error) => error,
                    };
                    let reason = match error {
                        crate::frame::VarError::IsScalar if purpose == NativeVariableNameLookupPurpose::Define => tcl_syntax::naming::NativeVariableDiagnosticReason::ArrayElement,
                        crate::frame::VarError::IsScalar => tcl_syntax::naming::NativeVariableDiagnosticReason::NotArray,
                        crate::frame::VarError::NoSuchNamespace => tcl_syntax::naming::NativeVariableDiagnosticReason::MissingParentNamespace,
                        _ => return self.original_c_lookup_var_error_using(original, purpose, error, stage),
                    };
                    self.original_c_variable_failure_for_object_using(original, purpose, reason,
                        tcl_syntax::naming::NativeVariableFailureSite::NameLookup, stage)
                })?);
            }
        }
        Ok(Some(OriginalCNameSelection {
            root: root_bytes,
            element,
            materialised,
            simple: selected,
            currency: operation.currency().clone(),
        }))
    }

    /// Original VARIABLE/NSUPVAR object lookup and exact indexed alias binding.
    /// Legacy C84 existence obtains only bytes from the original name. Its
    /// selected cell and read observers remain the same receiver owners; no
    /// parsed or local primary is inspected, discarded or installed here.
    ///
    fn prepare_original_c_byte_name_parts_using(
        &mut self,
        original: *mut TclObj,
        namespace: Option<crate::namespace::NsId>,
        explicit_element: Option<&[u8]>,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let operation = super::super::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        let bytes = self
            .original_c_name_bytes_using(original, stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?
            .to_vec();
        let protocol = self.native_c_variable_name_protocol().ok_or_else(|| {
            self.report_cmd_error_using(
                ValueError::CommandProtocolUnavailable("original C byte variable lookup").into(),
                stage,
            )
        })?;
        let input = tcl_syntax::naming::NativeNameProtocol::C(protocol.version())
            .combined_variable_input(tcl_core_types::c_string_extent(&bytes));
        if input.element().is_some() && explicit_element.is_some() {
            return Ok(None);
        }
        let root = input.root().selected().to_vec();
        let element = input
            .element()
            .map(|part| part.selected().to_vec())
            .or_else(|| {
                explicit_element.map(|part| tcl_core_types::c_string_extent(part).to_vec())
            });
        let selected = crate::vars::prepare_original_c_variable_cell(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            crate::vars::NativeOriginalNameScope {
                current: self.current_ns.get(),
                namespace,
            },
            &root,
            original,
            false,
            protocol.version(),
        );
        match selected {
            Ok(Some(selected)) => Ok(Some(OriginalCNameSelection {
                root,
                element,
                materialised: None,
                simple: selected,
                currency: operation.currency().clone(),
            })),
            Err(crate::frame::VarError::NameProtocolUnavailable) => Err(self
                .report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("actual original byte variable cell")
                        .into(),
                    stage,
                )),
            Ok(None) | Err(_) => Ok(None),
        }
    }
}
