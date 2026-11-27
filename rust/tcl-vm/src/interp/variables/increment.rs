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

//! `incr` through a captured cell: the read phase, the amount, the native integer errors and the store.

use super::super::{CapturedVariableUpdate, PreparedIncrementAmount, ResolvedVar, Vm};
use crate::value::Value;
use crate::vars::VarState as Local;
use tcl_registry::native_rmw::LegacyIncrementDialect;
use tcl_registry::native_rmw::NativeRmwDialect;
use tcl_registry::native_rmw::{NativeRmwAmountError, NativeRmwOperation};
use tcl_runtime_api::Completion;

impl Vm {
    pub(crate) fn increment_captured_bytes(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        amount: &Value,
    ) -> Result<tcl_runtime_api::VariableUpdateResult<Value>, Completion<Value>> {
        self.increment_selected_bytes(name, key, amount, None)
    }

    fn prepare_increment_read_phase(
        &mut self,
        amount: &Value,
    ) -> Result<
        (
            tcl_registry::native_rmw::NativeRmwReadPolicy,
            Option<PreparedIncrementAmount>,
        ),
        Completion<Value>,
    > {
        use tcl_registry::native_rmw::{NativeRmwAmountValidation, NativeRmwOperation};
        let policy = self
            .native_invocation_dialect()
            .native_rmw_read_policy(NativeRmwOperation::Increment)
            .ok_or_else(|| {
                self.refuse_host_command("native increment read policy is not selected".into())
            })?;
        let validation = self
            .native_invocation_dialect()
            .native_rmw_amount_validation(NativeRmwOperation::Increment)
            .ok_or_else(|| {
                self.refuse_host_command("native increment amount phase is not selected".into())
            })?;
        let prepared = if validation == NativeRmwAmountValidation::BeforeRead {
            Some(self.prepare_increment_amount(amount)?)
        } else {
            None
        };
        Ok((policy, prepared))
    }

    pub(in super::super) fn increment_selected_bytes(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        amount: &Value,
        selected: Option<ResolvedVar>,
    ) -> Result<tcl_runtime_api::VariableUpdateResult<Value>, Completion<Value>> {
        use tcl_syntax::naming::{
            NativeVariableFailureSite as Site, NativeVariableInputForm as Input,
        };
        let (policy, prepared) = self.prepare_increment_read_phase(amount)?;
        if self.dictionary_variable_containers() {
            self.confine_store_bytes(name, self.current_level(), false)?;
        }
        let selected = selected.or_else(|| {
            if key.is_some() {
                self.resolve_var_parts_from_bytes(name, key, self.current_level())
            } else {
                self.resolve_var_from_bytes(name, self.current_level())
            }
        });
        if self.dictionary_variable_containers()
            && selected
                .as_ref()
                .is_some_and(|selected| selected.elem.is_some())
        {
            let read = if let Some(key) = key {
                self.read_elem_traced_bytes(name, key)
            } else {
                self.read_var_traced_bytes(name)
            };
            let current = match read {
                Ok(current) => current,
                Err(error) => {
                    if self.refused_completion().is_some() {
                        return Err(error);
                    }
                    self.publish_swallowed_trace_error();
                    None
                }
            };
            let Some(PreparedIncrementAmount::Authored(prepared)) = prepared.as_ref() else {
                return Err(self.refuse_host_command(
                    "native Jim increment dictionary receiver preparation is not selected".into(),
                ));
            };
            let next = tcl_syntax::value::ValueOps::int_add(self, current.as_ref(), prepared)
                .map_err(|error| self.native_integer_error(&error))?;
            let stored = if let Some(key) = key {
                self.store_elem_result_bytes(name, key, next)?
            } else {
                self.store_var_result_bytes(name, next)?
            };
            return self.variable_update_result(stored, &Value::empty());
        }
        let input = key.map_or(Input::Combined(name), |element| Input::Separate {
            root: name,
            element: Some(element),
        });
        let resolved = selected.ok_or_else(|| {
            self.variable_access_error_at(
                "read",
                input,
                if policy == tcl_registry::native_rmw::NativeRmwReadPolicy::RequireContents {
                    "no such variable"
                } else {
                    "parent namespace doesn't exist"
                },
                Site::NameLookup,
            )
        })?;
        if resolved.elem.is_some()
            && resolved
                .base_id
                .and_then(|id| self.var_arena.get(id))
                .is_some_and(|cell| matches!(cell.state(), Local::Scalar(_)))
        {
            return Err(self.variable_access_error_at(
                "read",
                input,
                "variable isn't array",
                Site::NameLookup,
            ));
        }
        let captured = self.capture_selected_update(name, key, &resolved)?;
        if captured
            .cell
            .id
            .is_some_and(|id| self.const_vars.contains(&id))
        {
            return Err(self.variable_access_error_at(
                "incr",
                input,
                "variable is a constant",
                Site::ValueWrite,
            ));
        }
        self.with_variable_operation(&captured.cell, |vm| {
            vm.increment_captured_cell(name, key, &captured, amount, prepared.as_ref(), policy)
        })
    }

    pub(in super::super) fn native_integer_error(
        &mut self,
        error: &tcl_syntax::value::ValueError,
    ) -> Completion<Value> {
        if let Some(first) = self.refused_completion() {
            return first;
        }
        if let Some(refusal) = error.native_access_refusal() {
            return self.refuse_host_command(refusal.to_string());
        }
        let dialect = self.native_invocation_dialect();
        let Some(presentation) = dialect.integer_error_presentation() else {
            return self.refuse_host_command("native integer diagnostic is not selected".into());
        };
        let message = presentation.message(error);
        if *error == tcl_syntax::value::ValueError::IntegerOverflow
            && dialect.arithmetic() == Some(tcl_dialect::NativeArithmetic::Tcl84Wide)
        {
            crate::command::completion_from_cmd_error(
                self,
                tcl_cmd_core::CmdError::with_byte_error_details(
                    message,
                    b"ARITH IOVERFLOW {integer value too large to represent}".to_vec(),
                    None,
                    None,
                ),
            )
        } else {
            crate::command::completion_from_cmd_error(
                self,
                tcl_cmd_core::CmdError::new_bytes(message),
            )
        }
    }

    pub(in super::super) fn prepare_increment_amount(
        &mut self,
        amount: &Value,
    ) -> Result<PreparedIncrementAmount, Completion<Value>> {
        let dialect = self.native_invocation_dialect();
        if let Some(protocol) = dialect.native_legacy_increment_protocol() {
            return tcl_cmd_core::native_increment::prepare_legacy_amount(
                &mut crate::value_ops::VmLegacyIncrementAmountOps(self),
                protocol.recipe(),
                amount,
            )
            .map(PreparedIncrementAmount::Native)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error));
        }
        if self.authored_numeric_context().is_none() {
            return Err(self
                .refuse_host_command("native increment amount conversion is not selected".into()));
        }
        let grammar = dialect
            .native_rmw_amount_grammar(NativeRmwOperation::Increment)
            .ok_or_else(|| {
                self.refuse_host_command("authored increment amount grammar is not selected".into())
            })?;
        grammar
            .prepare(self, amount)
            .map(PreparedIncrementAmount::Authored)
            .map_err(|error| match error {
                NativeRmwAmountError::Integer(error) => self.native_integer_error(&error),
                NativeRmwAmountError::SafeExpression => crate::command::completion_from_cmd_error(
                    self,
                    tcl_cmd_core::CmdError::new_bytes(
                        error
                            .safe_expression_message_bytes(&amount.string_bytes())
                            .expect("safe expression diagnostic"),
                    ),
                ),
            })
    }

    fn read_captured_increment(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        captured: &CapturedVariableUpdate,
        policy: tcl_registry::native_rmw::NativeRmwReadPolicy,
    ) -> Result<(bool, Value, bool), Completion<Value>> {
        use tcl_registry::native_rmw::NativeRmwReadPolicy;
        let cell = &captured.cell;
        let trace = if let Some(key) = key {
            self.variable_trace_guest_or_unwind(|vm| {
                vm.fire_elem_traces_from_cell_bytes(name, key, "read", Some(cell.clone()))
            })
        } else {
            self.variable_trace_guest_or_unwind(|vm| {
                vm.fire_var_traces_from_cell_bytes(name, "read", None, None, Some(cell.clone()))
            })
        };
        if let Some(refusal) = self.refused_completion() {
            return Err(refusal);
        }
        let modern = self
            .native_invocation_dialect()
            .native_scalar_getter_protocol()
            .is_some_and(
                tcl_syntax::scalar_getter::NativeScalarGetterProtocol::supports_number_getter,
            );
        let (has_current, read_options) = match trace {
            Ok(()) => {
                let has_current = self
                    .initial_value_at_cell(
                        cell.id.expect("captured increment cell"),
                        captured.explicit_array,
                    )
                    .is_some();
                if has_current {
                    (true, Value::empty())
                } else {
                    let (missing, sets_code) = self.captured_read_missing(name, key, cell);
                    if let Some(refusal) = self.refused_completion() {
                        return Err(refusal);
                    }
                    if policy == NativeRmwReadPolicy::RequireContents {
                        return Err(missing);
                    }
                    (
                        false,
                        if sets_code {
                            missing.options
                        } else {
                            Value::empty()
                        },
                    )
                }
            }
            Err(error) if policy == NativeRmwReadPolicy::RequireContents => return Err(error),
            Err(error) => {
                let options = self
                    .completion_options_snapshot(&error)
                    .map_err(|error| crate::command::completion_from_tcl_error(self, error))?;
                self.publish_swallowed_trace_error();
                (false, options)
            }
        };
        Ok((has_current, read_options, modern))
    }

    fn increment_captured_cell(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        captured: &CapturedVariableUpdate,
        amount: &Value,
        prepared_amount: Option<&PreparedIncrementAmount>,
        policy: tcl_registry::native_rmw::NativeRmwReadPolicy,
    ) -> Result<tcl_runtime_api::VariableUpdateResult<Value>, Completion<Value>> {
        let cell = &captured.cell;
        let (has_current, read_options, modern) =
            self.read_captured_increment(name, key, captured, policy)?;
        self.retain_variable_read_error_code(&read_options)?;
        let next = if modern {
            let current = if has_current {
                self.initial_value_at_cell(
                    cell.id.expect("captured increment cell"),
                    captured.explicit_array,
                )
                .map(Value::native_lifetime_lease)
            } else {
                None
            };
            let reached = (|| {
                let objects = crate::value_ops::VmOriginalIncrementObjects::entered(self)?;
                let result = tcl_cmd_core::native_increment::increment(
                    &objects,
                    current
                        .as_ref()
                        .map(crate::NativeObjectLifetimeLease::value),
                    amount,
                );
                objects.complete(result)
            })();
            reached.map_err(|error| crate::command::completion_from_cmd_error(self, error))?
        } else if let Some(PreparedIncrementAmount::Native(prepared)) = prepared_amount {
            let current = if has_current {
                self.initial_value_at_cell(
                    cell.id.expect("captured increment cell"),
                    captured.explicit_array,
                )
                .map(Value::native_lifetime_lease)
            } else {
                None
            };
            let reached = (|| {
                let objects = crate::value_ops::VmLegacyIncrementObjects::entered(self)?;
                let result = tcl_cmd_core::native_increment::increment_legacy(
                    &objects,
                    current
                        .as_ref()
                        .map(crate::NativeObjectLifetimeLease::value),
                    *prepared,
                );
                objects.complete(result)
            })();
            reached.map_err(|error| crate::command::completion_from_cmd_error(self, error))?
        } else if let Some(PreparedIncrementAmount::Authored(prepared)) = prepared_amount {
            let current = has_current
                .then(|| self.read_resolved_cell(cell.id.expect("captured increment cell")))
                .flatten();
            tcl_syntax::value::ValueOps::int_add(self, current.as_ref(), prepared)
                .map_err(|error| self.native_integer_error(&error))?
        } else {
            return Err(
                self.refuse_host_command("increment physical transaction is not selected".into())
            );
        };
        let stored = self.store_captured_update(name, key, captured, next)?;
        self.variable_update_result(stored, &read_options)
    }

    /// Publish the `errorInfo` a read trace left behind when the
    /// read-modify-write command that fired it swallowed the error. C mirrors
    /// `iPtr->errorInfo` into the global as each frame is added
    /// (`TclLogCommandInfo`), so the chain stays visible even though the
    /// command went on to succeed; the interp's own accumulator is then reset
    /// so the next error starts a fresh trace.
    pub(crate) fn publish_swallowed_trace_error(&mut self) {
        if let Some(info) = self.take_error_info() {
            self.publish_error_info(&info);
        }
    }
}
