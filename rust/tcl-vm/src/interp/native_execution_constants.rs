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

//! Actual interpreter execution constants, independent of artifacts and literal arrays.

use super::Vm;
use crate::Value;
use tcl_registry::native_tcloo_compilation::NativeTclOoObjectInfo;
use tcl_syntax::value::ValueError;

impl Vm {
    pub(crate) fn native_oo_predicate_result(
        &mut self,
        present: bool,
    ) -> Result<Value, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native object predicate execution constants",
            ));
        }
        let version = dialect
            .tcl_version
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native object predicate execution constants",
            ))?;
        if !NativeTclOoObjectInfo::IsObject.uses_execution_constant(version) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native object predicate execution constants",
            ));
        }
        self.native_c_execution_boolean(present, version)
    }

    pub(crate) fn native_c_execution_boolean(
        &mut self,
        present: bool,
        version: tcl_dialect::TclVersion,
    ) -> Result<Value, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version != Some(version)
            || !tcl_registry::native_string_compilation::uses_execution_constant(version)
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "native C execution constants",
            ));
        }
        let constants = self
            .native_execution_booleans
            .get_or_insert_with(|| [Value::int(0), Value::int(1)]);
        Ok(constants[usize::from(present)].clone())
    }
    pub(crate) fn native_compiled_match_result(
        &mut self,
        pattern: Value,
        matched: bool,
        version: tcl_dialect::TclVersion,
        operation: tcl_registry::native_string_compilation::NativeStringMatchOperation,
    ) -> Result<Value, ValueError> {
        if tcl_registry::native_string_compilation::uses_execution_constant(version) {
            return self.native_c_execution_boolean(matched, version);
        }
        let dialect = self.actual_native_invocation_dialect();
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version != Some(version)
            || version != tcl_dialect::TclVersion::V8_4
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "native C84 match result",
            ));
        }
        if !operation.reuses_unshared_pattern(version) || pattern.native_object_is_shared() {
            let result = Value::int(i64::from(matched));
            result.set_native_unshared_integer(i64::from(matched), version)?;
            Ok(result)
        } else {
            pattern.set_native_unshared_integer(i64::from(matched), version)?;
            Ok(pattern)
        }
    }
}
