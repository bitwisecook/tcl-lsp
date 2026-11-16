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

//! Array operations on a retained array binding: keys, searches, element reads and removals across trace callbacks.

use super::super::{
    error_code_list, native_operation_currency, ArrayOperationTarget, Code, Interp,
};
use crate::obj::TclObj;

impl Interp {
    /// Locate one array binding, retain its opaque identity across the array
    /// operation trace, and expose it to the shared command core.
    pub(crate) fn with_array_trace_target(
        &mut self,
        name: &[u8],
        operation: impl FnOnce(&mut Self, &tcl_runtime_api::ArrayTarget) -> Code,
    ) -> Code {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => return self.refuse_native_execution(first),
        };
        let reached = self.with_array_trace_target_entered(name, operation);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            return self.refuse_native_execution(first);
        }
        reached
    }

    fn with_array_trace_target_entered(
        &mut self,
        name: &[u8],
        operation: impl FnOnce(&mut Self, &tcl_runtime_api::ArrayTarget) -> Code,
    ) -> Code {
        if self.observed_name_policy_selected() {
            let level = self.frames.borrow().current_level();
            return match self.observed_array_target(name, level) {
                Ok(target) => operation(self, &target),
                Err(error) => self.report_cmd_error(error.into()),
            };
        }
        if self.require_variable_name_protocol().is_err() {
            return Code::Error;
        }

        let frame = tcl_runtime_api::FrameId(self.frames.borrow().current_level());
        let located = crate::vars::array_target_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            name,
            frame.0,
        );
        let target = located.as_ref().map_or_else(
            || tcl_runtime_api::ArrayTarget::named_bytes(frame, name),
            |record| tcl_runtime_api::ArrayTarget::cell_bytes(frame, name, record.id()),
        );
        if let Some(record) = located {
            let retained = crate::vars::retain_array_target(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                &record,
            );
            debug_assert!(retained);
            self.array_operation_targets
                .borrow_mut()
                .push(ArrayOperationTarget { target: record });
        }
        let result = match self.fire_array_trace(name) {
            Some(code) => code,
            None => operation(self, &target),
        };
        if let Some(id) = target.cell_id() {
            let popped = self.array_operation_targets.borrow_mut().pop();
            debug_assert_eq!(popped.as_ref().map(|record| record.target.id()), Some(id));
            if let Some(record) = popped {
                crate::vars::release_array_target(&record.target);
                drop(record);
            }
        }
        result
    }

    pub(crate) fn array_operation_target(
        &self,
        target: &tcl_runtime_api::ArrayTarget,
    ) -> Option<crate::vars::ArrayCellTarget> {
        let id = target.cell_id()?;
        self.array_operation_targets
            .borrow()
            .iter()
            .rev()
            .find(|record| record.target.id() == id)
            .map(|record| record.target.clone())
    }

    pub(crate) fn array_keys_at_target(
        &self,
        target: &tcl_runtime_api::ArrayTarget,
    ) -> Option<Vec<Vec<u8>>> {
        let record = self.array_operation_target(target)?;
        crate::vars::array_names_at_target(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            &record,
        )
    }

    pub(crate) fn name_policy_protocol(&self) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        let protocols = self.invocation_protocols();
        if protocols.name.is_some() {
            return protocols.name_policy;
        }
        let dialect = self.native_invocation_dialect();
        let names = dialect.authored_logical_name_simulation(self.logical_name_provider.get()?)?;
        self.variable_storage_matches(names.variable_lookup_policy())
            .then_some(names)
    }

    pub(crate) fn array_search_keys_at_target(
        &self,
        target: &tcl_runtime_api::ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        if target.cell_id().is_none() {
            // LocateArray observed no original cell; a later same-name creation
            // cannot become this operation's target.
            return Ok(None);
        }
        let record = self.array_operation_target(target).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("retained array search"),
        )?;
        Ok(crate::vars::array_search_keys_at_target(&record))
    }

    pub(crate) fn array_search_element_exists_at_target(
        &self,
        target: &tcl_runtime_api::ArrayTarget,
        key: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let record = self.array_operation_target(target).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("retained array search"),
        )?;
        Ok(crate::vars::array_search_element_exists_at_target(
            &record, key,
        ))
    }

    pub(crate) fn array_unset_elem_at_target(
        &mut self,
        target: &tcl_runtime_api::ArrayTarget,
        key: &[u8],
    ) -> bool {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return false;
            }
        };
        let Some(record) = self.array_operation_target(target) else {
            return false;
        };
        let reached = crate::vars::unset_element_at_target(&record, key, scope.currency());
        if scope.currency().ensure_current_or_refuse().is_err() {
            return false;
        }
        let selected = match reached {
            Ok(selected) => selected,
            Err(error) => {
                self.byte_store_failure(error);
                return false;
            }
        };
        let removed = if let Some((removed, home)) = selected {
            if removed && self.has_variable_traces() {
                self.fire_selected_unset_callbacks(
                    &home,
                    target.name_bytes(),
                    Some(key),
                    target.name_bytes(),
                    true,
                );
            }
            removed
        } else {
            false
        };
        drop(record);
        scope.currency().ensure_current_or_refuse().is_ok() && removed
    }

    /// Tcl's `TclPtrGetVarIdx` read used by `array get`: select the live
    /// element before callbacks, fire containing-array then element traces,
    /// and read that same element afterwards. Trace errors are swallowed but
    /// published to `errorInfo`/`errorCode`; destruction of the array selected
    /// for the enclosing operation remains a hard read error.
    pub(crate) fn array_read_elem_at_target(
        &mut self,
        target: &tcl_runtime_api::ArrayTarget,
        key: &[u8],
    ) -> tcl_runtime_api::ArrayElementRead<*mut TclObj> {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => return tcl_runtime_api::ArrayElementRead::HostRefusal(first),
        };
        let reached = self.array_read_elem_at_target_entered(target, key);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            return tcl_runtime_api::ArrayElementRead::HostRefusal(first);
        }
        reached
    }

    fn array_read_elem_at_target_entered(
        &mut self,
        target: &tcl_runtime_api::ArrayTarget,
        key: &[u8],
    ) -> tcl_runtime_api::ArrayElementRead<*mut TclObj> {
        if let Some(error) = self.native_execution_refusal() {
            return tcl_runtime_api::ArrayElementRead::HostRefusal(error);
        }
        let live_array = crate::vars::array_target_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            target.name_bytes(),
            target.frame().0,
        );
        let live_was_array = live_array.as_ref().is_some_and(|live| {
            crate::vars::array_names_at_target(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                live,
            )
            .is_some()
        });
        let selected = crate::vars::array_element_target_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            target.name_bytes(),
            key,
            target.frame().0,
        );
        let selected_retained = selected.as_ref().is_some_and(|(array, element)| {
            crate::vars::retain_array_element_target(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                array,
                key,
                *element,
            )
        });
        let trace_errored = self
            .fire_read_trace(target.name_bytes(), Some(key))
            .is_some();
        let trace_failure = if trace_errored {
            self.error_code_bytes_checked().and_then(|code| {
                let message = self.completion_result_bytes_checked()?;
                let info = self.completion_error_info_bytes_checked()?;
                Ok(Some((message, code, info, i64::from(self.error_line()))))
            })
        } else if let Some(error) = self.native_execution_refusal() {
            Err(error)
        } else {
            Ok(None)
        };
        let outcome = match trace_failure {
            Err(error) => tcl_runtime_api::ArrayElementRead::HostRefusal(error),
            Ok(trace_failure) => match self.array_operation_target(target) {
                Some(operation)
                    if crate::vars::array_names_at_target(
                        &self.frames.borrow(),
                        &self.namespaces.borrow(),
                        &operation,
                    )
                    .is_some() =>
                {
                    if let Some((_, _, info, line)) = trace_failure {
                        self.publish_and_reset_error();
                        if let Some(error) = self.native_execution_refusal() {
                            tcl_runtime_api::ArrayElementRead::HostRefusal(error)
                        } else {
                            tcl_runtime_api::ArrayElementRead::Missing(
                                tcl_runtime_api::ArrayReadMiss::trace_error(Some(info), line),
                            )
                        }
                    } else {
                        let value = selected.as_ref().map_or_else(
                            || self.var_get_elem(target.name_bytes(), key),
                            |(array, element)| {
                                crate::vars::get_element_at_target(
                                    &self.frames.borrow(),
                                    &self.namespaces.borrow(),
                                    array,
                                    key,
                                    *element,
                                )
                            },
                        );
                        value.map_or_else(
                            || {
                                let miss = if live_was_array {
                                    tcl_runtime_api::ArrayReadMiss::missing()
                                } else {
                                    tcl_runtime_api::ArrayReadMiss::lookup(error_code_list(&[
                                        b"TCL",
                                        b"LOOKUP",
                                        b"VARNAME",
                                        target.name_bytes(),
                                    ]))
                                };
                                tcl_runtime_api::ArrayElementRead::Missing(miss)
                            },
                            tcl_runtime_api::ArrayElementRead::Value,
                        )
                    }
                }
                Some(operation) => {
                    let invalidation = if crate::vars::array_target_is_set(
                        &self.frames.borrow(),
                        &self.namespaces.borrow(),
                        &operation,
                    ) {
                        tcl_runtime_api::ArrayInvalidation::Retyped
                    } else {
                        tcl_runtime_api::ArrayInvalidation::Unset
                    };
                    trace_failure.map_or_else(
                        || tcl_runtime_api::ArrayElementRead::ArrayInvalidated(invalidation),
                        |(message, code, info, line)| {
                            tcl_runtime_api::ArrayElementRead::TraceError(
                                tcl_runtime_api::ArrayReadFailure::new_bytes(
                                    message,
                                    code,
                                    Some(info),
                                    Some(line),
                                ),
                            )
                        },
                    )
                }
                None => trace_failure.map_or_else(
                    || {
                        tcl_runtime_api::ArrayElementRead::ArrayInvalidated(
                            tcl_runtime_api::ArrayInvalidation::Unset,
                        )
                    },
                    |(message, code, info, line)| {
                        tcl_runtime_api::ArrayElementRead::TraceError(
                            tcl_runtime_api::ArrayReadFailure::new_bytes(
                                message,
                                code,
                                Some(info),
                                Some(line),
                            ),
                        )
                    },
                ),
            },
        };
        if let tcl_runtime_api::ArrayElementRead::Value(value) = &outcome {
            // The selected element's retained cell is released immediately
            // below. Transfer one hold to the shared array core first so no
            // raw pointer can die between this return and list materialisation.
            tcl_syntax::value::ValueOps::pin_value(self, value);
        }
        if selected_retained {
            if let Some((array, element)) = selected {
                crate::vars::release_array_element_target(&array, key, element);
            }
        }
        outcome
    }
}
