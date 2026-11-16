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

//! The native interpreter result and C return state: borrowing, resetting and adopting the result object, the Jim object context, and native list and dictionary conversion at ingress.

use super::{InterpState, NativeCReturnState};
use crate::value::Value;
use std::rc::Rc;

impl InterpState {
    /// Retain the actual interpreter objects independently of source grammar.
    pub(crate) fn native_jim_object_context(
        &self,
    ) -> Result<Rc<crate::value::NativeJimObjectContext>, tcl_syntax::value::ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.native_string_protocol()
            != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original source context",
            ));
        }
        if let Some(context) = self.jim_object_context.borrow().as_ref() {
            if !context.is_live() {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "retired Jim interpreter",
                ));
            }
            context.select_numeric_host(Rc::clone(&self.host));
            return Ok(Rc::clone(context));
        }
        let context = crate::value::NativeJimObjectContext::new(dialect)?;
        context.select_numeric_host(Rc::clone(&self.host));
        self.name_world.borrow_mut().jim_root_namespace_object =
            Some(context.empty_object().clone());
        *self.jim_object_context.borrow_mut() = Some(Rc::clone(&context));
        Ok(context)
    }

    /// Consume the actual top-frame namespace object, independently of its name index.
    pub(super) fn native_jim_root_namespace_bytes(
        &self,
    ) -> Result<Rc<[u8]>, tcl_syntax::value::ValueError> {
        let _context = self.native_jim_object_context()?;
        self.name_world
            .borrow()
            .jim_root_namespace_object
            .as_ref()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original top namespace",
            ))?
            .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::Jim084)
            .map_err(|error| {
                tcl_syntax::value::ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                )
            })
    }

    /// Borrow the actual interpreter result without adding an object reference.
    pub(crate) fn with_native_interp_result<R>(
        &self,
        observer: impl FnOnce(&Value) -> R,
    ) -> Result<R, tcl_syntax::value::ValueError> {
        let protocol = self.actual_native_string_protocol();
        if protocol == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084) {
            return Ok(self
                .native_jim_object_context()?
                .with_result_object(observer));
        }
        if protocol
            .and_then(tcl_syntax::native_string::NativeStringProtocol::tcl_version)
            .is_none()
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual interpreter result ownership",
            ));
        }
        if self.native_c_result.borrow().is_none() {
            *self.native_c_result.borrow_mut() =
                Some(Value::new_native_string_bytes(b"".as_slice()));
        }
        let result = self.native_c_result.borrow();
        Ok(observer(result.as_ref().expect("actual C result owner")))
    }

    pub(crate) fn reset_native_jim_result(&mut self) -> Result<(), tcl_syntax::value::ValueError> {
        let protocol = self.actual_native_string_protocol();
        if protocol == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084) {
            let context = self.native_jim_object_context()?;
            context.publish_result(&context.empty_object());
        } else if protocol
            .and_then(tcl_syntax::native_string::NativeStringProtocol::tcl_version)
            .is_some()
        {
            let reuse = self.with_native_interp_result(|result| {
                (!result.native_object_is_shared()).then(|| result.native_lifetime_lease())
            })?;
            if let Some(original) = reuse {
                original.value().reset_native_c_result();
            } else {
                let retired = self
                    .native_c_result
                    .replace(Some(Value::new_native_string_bytes(b"".as_slice())));
                drop(retired);
            }
        }
        self.reset_native_global_error_episode();
        Ok(())
    }

    /// Move the real result owner into the interpreter and return a lifetime view.
    pub(crate) fn adopt_native_interp_result(
        &mut self,
        value: Value,
    ) -> Result<crate::NativeObjectLifetimeLease, tcl_syntax::value::ValueError> {
        let protocol = self.actual_native_string_protocol();
        let lease = value.native_lifetime_lease();
        if protocol == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084) {
            self.native_jim_object_context()?.publish_result(&value);
            drop(value);
        } else if protocol
            .and_then(tcl_syntax::native_string::NativeStringProtocol::tcl_version)
            .is_some()
        {
            let retired = self
                .native_c_result
                .replace(Some(value.into_native_reference()));
            drop(retired);
        } else {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual interpreter result ownership",
            ));
        }
        Ok(lease)
    }

    pub(crate) fn set_native_c_return_state(&mut self, code: i32, level: i64) {
        if self
            .actual_native_string_protocol()
            .and_then(tcl_syntax::native_string::NativeStringProtocol::tcl_version)
            .is_some()
        {
            self.native_errors.native_c_return_state = NativeCReturnState { code, level };
        }
    }

    pub(crate) fn settle_native_c_return_level(&mut self, level: i64) {
        let code = if level == 0 {
            0
        } else {
            self.native_errors.native_c_return_state.code
        };
        self.set_native_c_return_state(code, level);
    }

    /// Native List conversion binds original Jim source ownership at ingress.
    pub(crate) fn native_object_list_elements_in(
        &self,
        original: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<crate::NativeListItems, tcl_syntax::value::ValueError> {
        if protocol.is_jim084() {
            original.bind_native_jim_context(&self.native_jim_object_context()?)?;
        }
        original.native_object_list_elements(protocol)
    }

    pub(crate) fn native_object_dict_pairs_in(
        &self,
        original: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Vec<(Value, Value)>, tcl_syntax::value::ValueError> {
        if protocol.is_jim084() {
            original.bind_native_jim_context(&self.native_jim_object_context()?)?;
        }
        original.native_object_dict_pairs(protocol)
    }
}
