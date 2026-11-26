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

//! Jim Parent consumes and publishes actual original object holders.

use super::{CmdError, Value, ValueError, Vm};
use tcl_syntax::naming::NativeJimNamespaceParentResult;

impl Vm {
    pub(crate) fn original_jim_namespace_parent_object(
        &mut self,
        original: &Value,
        check: &impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<Value, CmdError> {
        let original = original.native_lifetime_lease();
        let scope = super::super::native_operation_currency::NativeOperationScope::enter(self)?;
        let current = |vm: &mut Vm| {
            check(vm)?;
            super::super::native_numeric_ops::OriginalNumericOps::in_original_scope(vm, &scope)?
                .current_for_value(original.value())
        };
        current(self)?;
        let dialect = self.actual_native_invocation_dialect();
        let bytes = self.native_name_operand_bytes_checked_using(original.value(), &current);
        current(self)?;
        let bytes = bytes?;
        if !bytes.is_empty() {
            let table = dialect
                .native_jim_namespace_parent_root_comparison_table()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim parent root comparison declaration",
                ))?;
            let root = self.native_jim_compare_immediate_checked(original.value(), &table, 0);
            current(self)?;
            if root? {
                return self.original_jim_parent_result(&current);
            }
        }
        let protocol = dialect
            .native_name_protocol()
            .filter(|p| p.is_jim084())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim parent text issuer",
            ))?;
        match protocol
            .jim_namespace_parent_result(&bytes)
            .map_err(|_| ValueError::CommandProtocolUnavailable("Jim parent text result purpose"))?
        {
            NativeJimNamespaceParentResult::Unchanged => {}
            NativeJimNamespaceParentResult::CountedQualifier(bytes) => {
                self.adopt_native_interp_result(Value::new_native_string_bytes(bytes))?;
                current(self)?;
            }
            NativeJimNamespaceParentResult::RootedQualifier(bytes) => {
                self.append_original_jim_parent_qualifier(Some(bytes), &current)?;
            }
            NativeJimNamespaceParentResult::RootedSharedEmpty => {
                self.append_original_jim_parent_qualifier(None, &current)?;
            }
        }
        self.original_jim_parent_result(&current)
    }

    fn original_jim_parent_result(
        &mut self,
        check: &impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<Value, CmdError> {
        check(self)?;
        let value = self.with_native_interp_result(Value::native_lifetime_lease)?;
        value.value().check_native_header()?;
        check(self)?;
        Ok(value.into_value())
    }

    fn append_original_jim_parent_qualifier(
        &mut self,
        fresh: Option<&[u8]>,
        check: &impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<(), CmdError> {
        let context = self.native_jim_object_context()?;
        let qualifier = fresh.map_or_else(
            || context.empty_object().native_lifetime_lease().into_value(),
            |bytes| Value::new_native_string_bytes(bytes).into_native_unowned_lifetime(),
        );
        let reached = (|| {
            check(self)?;
            let append = self
                .actual_native_invocation_dialect()
                .native_object_append_protocol(None)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim parent qualifier append",
                ))?;
            let published =
                self.adopt_native_interp_result(Value::new_native_string_bytes(b"::".as_slice()))?;
            check(self)?;
            let mut first = None;
            let appended =
                tcl_cmd_core::native_append::append_published_unshared_jim_object_checked(
                    &crate::value::VmAppendObjects,
                    append.recipe(),
                    published.value(),
                    &qualifier,
                    &mut || {
                        if let Err(error) = check(self) {
                            first.get_or_insert(error);
                            return Err(ValueError::CommandProtocolUnavailable(
                                "Jim parent original operation",
                            ));
                        }
                        qualifier.check_native_header()?;
                        published.value().check_native_header()?;
                        if published.value().native_object_reference_count() != 1
                            || published.value().native_object_is_shared()
                            || !self.with_native_interp_result(|actual| {
                                actual.is_same_object(published.value())
                            })?
                        {
                            return Err(ValueError::CommandProtocolUnavailable(
                                "Jim parent actual published unshared receiver",
                            ));
                        }
                        Ok(())
                    },
                );
            if let Some(error) = first {
                return Err(error);
            }
            check(self)?;
            appended?;
            // Jim balances this temporary source reference only after append.
            drop(
                qualifier
                    .native_lifetime_lease()
                    .into_value()
                    .into_native_reference(),
            );
            check(self)
        })();
        if let Err(error) = &reached
            && let Some(cause) = error.native_execution_refusal()
        {
            self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(cause.clone()));
        }
        if reached.is_err()
            && fresh.is_some()
            && qualifier.native_object_is_live()
            && qualifier.native_object_reference_count() == 0
        {
            qualifier.retire_unowned_native_header();
        }
        let current = check(self);
        match reached {
            Err(first) => Err(first),
            Ok(()) => current,
        }
    }
}
