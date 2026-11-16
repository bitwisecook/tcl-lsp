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

//! Capturing the selected cell of a name: element keys, namespace targets, materialised links and the receiver.

use super::super::{Code, Interp};
use super::{OriginalCLinkTarget, OriginalCNameSelection, OriginalCNameStage};
use crate::obj;
use crate::obj::{Owned, TclObj};
use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose;

impl Interp {
    pub(super) fn original_c_parsed_element_key(&self, original: *mut TclObj) -> Option<Owned> {
        obj::native_variable_name::with_parsed(original, |cache| {
            let (_, element) = cache.array.as_ref()?;
            match cache.protocol.parsed_element_table_key(element)? {
                tcl_syntax::native_variable_name::NativeElementTableKey::FreshString(bytes) => {
                    Some(Owned::fresh(obj::new_string_bytes(bytes)))
                }
                tcl_syntax::native_variable_name::NativeElementTableKey::Original(original) => {
                    Some(original.clone())
                }
            }
        })
        .flatten()
    }

    pub(super) fn capture_original_c_selection(
        &mut self,
        original: *mut TclObj,
        selected: &OriginalCNameSelection,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<(crate::frame::VariableReceiver, crate::vars::TraceHome)>, Code> {
        self.capture_original_c_selection_using(original, selected, purpose, None)
    }

    pub(super) fn capture_original_c_selection_using(
        &mut self,
        original: *mut TclObj,
        selected: &OriginalCNameSelection,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<(crate::frame::VariableReceiver, crate::vars::TraceHome)>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let reached = self.capture_original_c_selection_receiver(original, selected, purpose);
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        reached.map_err(|error| {
            if error == crate::frame::VarError::DeletedNamespace
                && selected.element.is_some()
                && matches!(
                    purpose,
                    NativeVariableNameLookupPurpose::Write
                        | NativeVariableNameLookupPurpose::QuietWrite
                )
            {
                // TclLookupArrayElement fails before the value write. This
                // original-object entry still supplies the root to its error
                // tuple even when that root selected a compiled local cell.
                return self.original_c_variable_failure_for_object_using(
                    original,
                    purpose,
                    tcl_syntax::naming::NativeVariableDiagnosticReason::RetiredNamespace,
                    tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                    stage,
                );
            }
            self.original_c_lookup_var_error_using(original, purpose, error, stage)
        })
    }

    pub(super) fn capture_original_c_parts_selection(
        &mut self,
        original: *mut TclObj,
        element: Option<*mut TclObj>,
        selected: &OriginalCNameSelection,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<(crate::frame::VariableReceiver, crate::vars::TraceHome)>, Code> {
        self.capture_original_c_parts_selection_using(original, element, selected, purpose, None)
    }

    pub(super) fn capture_original_c_parts_selection_using(
        &mut self,
        original: *mut TclObj,
        element: Option<*mut TclObj>,
        selected: &OriginalCNameSelection,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<(crate::frame::VariableReceiver, crate::vars::TraceHome)>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let captured = self.capture_original_c_selection_receiver(original, selected, purpose);
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        captured.map_err(|error| {
            if !purpose.leaves_error_message() || element.is_none() {
                return self.original_c_lookup_var_error_using(original, purpose, error, stage);
            }
            let input = match self.original_c_variable_reporting_input_using(
                (&selected.root, selected.element.as_deref()),
                (Some(original), element, false),
                stage,
            ) {
                Ok(input) => input,
                Err(code) => return code,
            };
            self.original_c_variable_receiver_error_using(
                input.input(),
                purpose,
                error,
                Some(tcl_syntax::naming::NativeVariableFailureSite::NameLookup),
                stage,
            )
        })
    }

    pub(super) fn prepare_selected_c_namespace_target(
        &mut self,
        original: *mut TclObj,
        selected: &OriginalCNameSelection,
    ) -> Result<OriginalCLinkTarget, Code> {
        self.prepare_selected_c_namespace_target_using(original, selected, None)
    }

    pub(super) fn prepare_selected_c_namespace_target_using(
        &mut self,
        original: *mut TclObj,
        selected: &OriginalCNameSelection,
        stage: OriginalCNameStage<'_>,
    ) -> Result<OriginalCLinkTarget, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        selected
            .simple
            .check(self, &selected.currency)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let operation = super::super::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        let mut target =
            if let Some(target) = self.capture_original_c_materialised_link(selected)? {
                target
            } else {
                let reached = crate::vars::selected_c_link_target(
                    &mut self.frames.borrow_mut(),
                    &mut self.namespaces.borrow_mut(),
                    &selected.simple,
                    selected.element.clone(),
                );
                selected
                    .simple
                    .check(self, &selected.currency)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                reached.map_err(|error| {
                    self.original_c_lookup_var_error_using(
                        original,
                        NativeVariableNameLookupPurpose::Link,
                        error,
                        stage,
                    )
                })?
            };
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let reached = crate::vars::prepare_selected_upvar_target(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            &mut target,
        );
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        selected
            .simple
            .check(self, &selected.currency)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        reached.map_err(|error| {
            self.original_c_lookup_var_error_using(
                original,
                NativeVariableNameLookupPurpose::Link,
                error,
                stage,
            )
        })?;
        let observed = crate::vars::OriginalAliasTargetObservation::capture(
            &self.frames.borrow(),
            &self.namespaces(),
            &target,
        );
        let observed = observed.map_err(|error| {
            self.original_c_lookup_var_error_using(
                original,
                NativeVariableNameLookupPurpose::Link,
                error,
                stage,
            )
        })?;
        let target = OriginalCLinkTarget {
            protocol: self
                .native_c_variable_name_protocol()
                .expect("selected C alias recipe"),
            link: target,
            source: Some(selected.simple.clone()),
            target: observed,
            operation,
        };
        target
            .check(self)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        Ok(target)
    }

    pub(super) fn capture_original_c_materialised_link(
        &mut self,
        selected: &OriginalCNameSelection,
    ) -> Result<Option<crate::frame::Link>, Code> {
        selected
            .simple
            .check(self, &selected.currency)
            .map_err(|error| self.report_cmd_error(error))?;
        let Some(original_target) = &selected.materialised else {
            return Ok(None);
        };
        let operation = super::super::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        let target = original_target.capture_link_target(self, operation.currency());
        operation
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| self.refuse_native_execution(first))?;
        target
            .map(Some)
            .map_err(|error| self.report_cmd_error(error))
    }

    fn capture_original_c_selection_receiver(
        &mut self,
        original: *mut TclObj,
        selected: &OriginalCNameSelection,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<
        Option<(crate::frame::VariableReceiver, crate::vars::TraceHome)>,
        crate::frame::VarError,
    > {
        selected
            .simple
            .check(self, &selected.currency)
            .map_err(|error| {
                self.report_cmd_error(error);
                crate::frame::VarError::NameProtocolUnavailable
            })?;
        if let Some(original_target) = &selected.materialised {
            let operation =
                super::super::native_operation_currency::NativeOperationScope::enter(self)
                    .map_err(|first| {
                        self.refuse_native_execution(first);
                        crate::frame::VarError::NameProtocolUnavailable
                    })?;
            let reached = original_target.capture_receiver(self, operation.currency());
            operation
                .currency()
                .ensure_current_or_refuse()
                .map_err(|first| {
                    self.refuse_native_execution(first);
                    crate::frame::VarError::NameProtocolUnavailable
                })?;
            let reached = reached.map_err(|error| {
                // The captured owner has already retained the full first cause;
                // VarError is only the existing outer diagnostic unwind port.
                self.report_cmd_error(error);
                crate::frame::VarError::NameProtocolUnavailable
            })?;
            self.refuse_confined_receiver(&reached.1, purpose)?;
            return Ok(Some(reached));
        }
        // Asked by name before the capture, which may create the entry.
        if Self::changes_a_variable(purpose) && self.store_escapes(&selected.root) {
            return Err(crate::frame::VarError::Confined);
        }
        let captured = crate::vars::capture_original_c_selected_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            &selected.simple,
            selected.element.clone(),
            purpose.creates_entries(),
        );
        selected
            .simple
            .check(self, &selected.currency)
            .map_err(|error| {
                self.report_cmd_error(error);
                crate::frame::VarError::NameProtocolUnavailable
            })?;
        let captured = captured?
            .map(|(receiver, home)| {
                let home = home
                    .for_selected_receiver(&receiver)
                    .ok_or(crate::frame::VarError::NameProtocolUnavailable)?;
                Ok((receiver, home))
            })
            .transpose()?;
        if purpose.creates_entries()
            && self
                .native_c_variable_name_protocol()
                .is_some_and(|protocol| protocol.version() >= tcl_dialect::TclVersion::V8_5)
        {
            if let (Some((receiver, _)), Some(_)) = (&captured, &selected.element) {
                let retained_key = self.original_c_parsed_element_key(original);
                receiver.retain_original_element_key(retained_key.as_ref().map(Owned::as_ptr));
            }
        }
        if let Some((_, home)) = &captured {
            self.refuse_confined_receiver(home, purpose)?;
        }
        Ok(captured)
    }
}
