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

//! Stores through an already selected cell: captured byte names, native formal cells and compiled temporaries.

use super::super::{native_operation_currency, Interp};
use crate::frame::VarError;
use crate::namespace::NsId;
use crate::obj;
use crate::obj::TclObj;

impl Interp {
    pub(in super::super) fn with_captured_byte_variable<T>(
        &self,
        current: NsId,
        level: Option<usize>,
        name: &[u8],
        element: Option<&[u8]>,
        create: bool,
        worker: impl FnOnce(
            &mut crate::vars::EnteredVariablePlace,
            &native_operation_currency::NativeOperationCurrency,
        ) -> Result<T, crate::frame::JimLinkStoreError>,
    ) -> Result<T, VarError> {
        let scope =
            native_operation_currency::NativeOperationScope::enter(self).map_err(|first| {
                self.clone().refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        self.require_variable_name_protocol()?;
        let captured = crate::vars::EnteredVariablePlace::capture(
            self,
            current,
            level,
            name,
            element,
            create,
            scope.currency(),
        );
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| {
                self.clone().refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        let mut captured = captured.map_err(|error| self.byte_store_failure(error))?;
        let reached = worker(&mut captured, scope.currency());
        let current = captured.check(self, scope.currency());
        drop(captured);
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| {
                self.clone().refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        current.map_err(|error| self.byte_store_failure(error))?;
        reached.map_err(|error| self.byte_store_failure(error))
    }

    pub(crate) fn read_captured_byte_variable(
        &self,
        current: NsId,
        level: Option<usize>,
        name: &[u8],
        element: Option<&[u8]>,
        default: bool,
    ) -> Result<Option<*mut TclObj>, VarError> {
        self.with_captured_byte_variable(current, level, name, element, false, |place, currency| {
            place.read(self, currency, default)
        })
    }

    pub(in super::super) fn byte_store_failure(
        &self,
        error: crate::frame::JimLinkStoreError,
    ) -> VarError {
        match error {
            crate::frame::JimLinkStoreError::Variable(error) => error,
            crate::frame::JimLinkStoreError::Command(error) => {
                if let Some(first) = error.native_execution_refusal() {
                    self.clone().refuse_native_execution(first.clone());
                }
                VarError::NameProtocolUnavailable
            }
        }
    }

    /// Storage-only port for an already selected byte-name context. It grants
    /// no original header, native lookup or Jim dictionary path purpose.
    pub(crate) fn store_captured_byte_variable(
        &mut self,
        current: NsId,
        level: Option<usize>,
        name: &[u8],
        element: Option<&[u8]>,
        value: *mut TclObj,
    ) -> Result<(), VarError> {
        let scope =
            native_operation_currency::NativeOperationScope::enter(self).map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        self.require_variable_name_protocol()?;
        obj::check_native_liveness(value).map_err(|error| self.byte_store_failure(error.into()))?;
        let allocation = obj::NativeObjectLifetime::retain(value);
        self.associate_native_jim_variable_value(value)?;
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        let captured = crate::vars::capture_byte_store_entered(
            self,
            current,
            level,
            name,
            element,
            scope.currency(),
        );
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        let mut captured = captured.map_err(|error| self.byte_store_failure(error))?;
        let reached = captured.store(value, scope.currency());
        let current = captured.check(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            scope.currency(),
        );
        // Receiver cleanup can retire a genuine native alias/shell owner.
        drop(captured);
        drop(allocation);
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        current.map_err(|error| self.byte_store_failure(error))?;
        reached.map_err(|error| self.byte_store_failure(error))
    }

    /// Publish the same indexed formal role outside frame/table borrows, under
    /// its actual original activation and cell through all reached retirement.
    pub(crate) fn store_native_formal_cell_checked(
        &mut self,
        slot: usize,
        value: *mut TclObj,
    ) -> Result<(), VarError> {
        let scope =
            native_operation_currency::NativeOperationScope::enter(self).map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        obj::check_native_liveness(value).map_err(|error| {
            self.refuse_completion_value_access(error);
            VarError::NameProtocolUnavailable
        })?;
        let (prepared, frame) = {
            let mut frames = self.frames.borrow_mut();
            let frame = frames.current_activation_owner();
            (frames.prepare_formal_cell_store(slot, value), frame)
        };
        // Preserve the full prepared-owner refusal after ending the frame
        // borrow; the legacy VarError port only unwinds the same first cause.
        let prepared = prepared.map_err(|error| {
            self.refuse_completion_value_access(error);
            VarError::NameProtocolUnavailable
        })?;
        let original = prepared.observation();
        let check = || {
            let reached = (|| {
                scope.currency().ensure_current_or_refuse().map_err(|_| {
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "entered original formal store",
                    )
                })?;
                let current = self.frames.borrow().current_activation_owner();
                if frame.strong_count() == 0 || !std::rc::Weak::ptr_eq(&frame, &current) {
                    return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original formal frame",
                    ));
                }
                if !self
                    .frames
                    .borrow()
                    .original_formal_cell_is_current(slot, &original)
                {
                    return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original formal cell publication",
                    ));
                }
                original.check_current()
            })();
            reached.inspect_err(|error| {
                self.clone().refuse_completion_value_access(error.clone());
            })
        };
        let reached = prepared.finish(check).and_then(|old| old.finish(check));
        if let Err(error) = reached {
            self.refuse_completion_value_access(error);
            return Err(VarError::NameProtocolUnavailable);
        }
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })
    }

    /// The admitted anonymous-cell worker keeps its original frame through
    /// displaced-value retirement. A reported name supplies no indexed grant.
    pub(crate) fn store_native_compiled_temporary_checked(
        &mut self,
        slot: usize,
        value: *mut TclObj,
    ) -> Result<(), VarError> {
        let scope =
            native_operation_currency::NativeOperationScope::enter(self).map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        obj::check_native_liveness(value).map_err(|error| self.byte_store_failure(error.into()))?;
        let allocation = obj::NativeObjectLifetime::retain(value);
        let (receiver, frame) = {
            let mut frames = self.frames.borrow_mut();
            let frame = frames.current_activation_owner();
            (
                frames.capture_native_compiled_temporary_receiver(slot)?,
                frame,
            )
        };
        let reached = receiver.store(value);
        let receiver_current = receiver.check_current();
        drop(receiver);
        drop(allocation);
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        let frame_current = {
            let frames = self.frames.borrow();
            std::rc::Weak::ptr_eq(&frame, &frames.current_activation_owner())
                && frame.strong_count() != 0
        };
        if !frame_current {
            let failure = tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "captured native anonymous frame",
            );
            return Err(self.byte_store_failure(failure.into()));
        }
        receiver_current?;
        reached
    }

    /// Clear the original anonymous cell, then retire its actual displaced
    /// owner outside the frame borrow before continuing the instruction.
    pub(crate) fn unset_native_compiled_temporary_checked(
        &mut self,
        slot: usize,
    ) -> Result<(), VarError> {
        let scope =
            native_operation_currency::NativeOperationScope::enter(self).map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        let (receiver, frame) = {
            let mut frames = self.frames.borrow_mut();
            let frame = frames.current_activation_owner();
            (
                frames.capture_native_compiled_temporary_receiver_for_unset(slot)?,
                frame,
            )
        };
        let reached = receiver.as_ref().map_or(Ok(()), |receiver| {
            receiver.unset_checked(scope.currency()).map(|_| ())
        });
        let receiver_current = receiver
            .as_ref()
            .map_or(Ok(()), |receiver| receiver.check_current());
        drop(receiver);
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        let frame_current = {
            let frames = self.frames.borrow();
            std::rc::Weak::ptr_eq(&frame, &frames.current_activation_owner())
                && frame.strong_count() != 0
        };
        if !frame_current {
            return Err(self.byte_store_failure(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "captured native anonymous frame",
                )
                .into(),
            ));
        }
        receiver_current?;
        reached.map_err(|error| self.byte_store_failure(error))
    }
}
