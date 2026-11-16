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

//! The native observers of the global `env` array and of `tcl_precision`: installation and firing.

use super::super::variable_trace_outcomes::VariableTraceOutcome;
use super::super::{VarTraceCell, Vm, err, native_numeric_ops, native_operation_currency};
use crate::command::err_with_code;
use crate::value::Value;
use crate::vars::VarState as Local;
use tcl_runtime_api::guard::GuardDomain;
use tcl_runtime_api::{Completion, VarId};

impl Vm {
    /// Hook the global `env` array to the host environment, as C's
    /// `TclSetupEnv` does for every interpreter it creates: one array-level
    /// read/write/unset/array observer ([`Self::fire_native_env_trace`]).
    /// Jim keeps no such hook.
    pub(in super::super) fn install_native_env_trace(&mut self) {
        self.variable_observers.env_cell = None;
        if self.uses_jim_error_stack() {
            return;
        }
        self.variable_observers.env_cell = self.resolve_var_from("::env", 0).and_then(|r| r.id);
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
    }

    /// C's `EnvTraceProc` (`tclEnv.c`) for the hooked `env` array: an `array`
    /// operation re-syncs the whole array from the host; an element write sets
    /// the host variable, an element unset removes it, and an element read
    /// refreshes the element from the host or fails with C's `no such
    /// variable` when the host lacks it. A whole-array unset leaves the host
    /// alone. Every store into the array here is raw: nothing is written
    /// back. A name or value reaches the host cut at its first NUL, as C's
    /// `environ` keeps it.
    pub(super) fn fire_native_env_trace(
        &mut self,
        name: &[u8],
        op: &str,
        cell: &VarTraceCell,
        scope: &native_operation_currency::NativeOperationScope,
    ) -> VariableTraceOutcome {
        scope.check_native(self)?;
        let Some(array) = self.variable_observers.env_cell else {
            return Ok(Ok(()));
        };
        let host = self.host_rc();
        let env = host.env();
        let before_nul = |text: &str| text.split('\0').next().unwrap_or_default().to_owned();
        if op == "array" {
            let mut vars = env.vars();
            vars.sort_unstable();
            let stale: Vec<String> =
                match self.var_arena.get(array).map(crate::vars::VarCell::state) {
                    Some(Local::Array(elements)) => elements
                        .keys()
                        .map(|key| String::from_utf8_lossy(key.as_ref()).into_owned())
                        .filter(|key| {
                            vars.binary_search_by(|(host_key, _)| {
                                host_key.as_str().cmp(key.as_str())
                            })
                            .is_err()
                        })
                        .collect(),
                    _ => return Ok(Ok(())),
                };
            // `TclSetupEnv` unsets each stale element globally as the parts
            // `env` and `NAME` (`Tcl_UnsetVar2`).
            for key in stale {
                if let Some(base) = self
                    .resolve_var_parts_from_bytes(b"env", Some(key.as_bytes()), 0)
                    .and_then(|resolved| resolved.base_id)
                {
                    let _ = self.array_unset_elem_reporting_bytes_at(
                        base,
                        b"env",
                        key.as_bytes(),
                        None,
                    );
                }
                scope.check_native(self)?;
            }
            for (key, value) in vars {
                let _ = self.write_array_raw("::env", &key, Value::string(value));
            }
            scope.check_native(self)?;
            return Ok(Ok(()));
        }
        // Only an element access names a host variable.
        let Some(key) = cell
            .elem
            .as_ref()
            .filter(|_| cell.array_id() == Some(array))
            .map(|key| String::from_utf8_lossy(key.as_ref()).into_owned())
        else {
            return Ok(Ok(()));
        };
        let host_key = before_nul(&key);
        match op {
            "write" => {
                let value = cell.id.and_then(|id| {
                    match self.var_arena.get(id).map(crate::vars::VarCell::state) {
                        Some(Local::Scalar(value)) => Some(value.to_str().to_string()),
                        _ => None,
                    }
                });
                if let Some(value) = value {
                    env.set(&host_key, &before_nul(&value));
                }
            }
            "read" => {
                let Some(value) = env.get(&host_key) else {
                    let failure = self.present_variable_trace_failure(
                        err("no such variable"),
                        None,
                        op,
                        name,
                        scope,
                    )?;
                    return Ok(Err(failure));
                };
                let current = cell.id.and_then(|id| {
                    match self.var_arena.get(id).map(crate::vars::VarCell::state) {
                        Some(Local::Scalar(value)) => Some(value.to_str().to_string()),
                        _ => None,
                    }
                });
                if current.as_deref() != Some(value.as_str()) {
                    let _ = self.write_array_raw("::env", &key, Value::string(value));
                }
            }
            // An element unset reaches the host; the elements a whole-array
            // unset tears down, the array no longer bound as `::env`, do not.
            "unset" if self.resolve_var_from("::env", 0).and_then(|r| r.id) == Some(array) => {
                env.unset(&host_key);
            }
            _ => {}
        }
        scope.check_native(self)?;
        Ok(Ok(()))
    }

    pub(in super::super) fn install_native_precision_trace(&mut self) {
        use tcl_registry::special_vars::SpecialVariableHook;
        self.variable_observers.precision_cell = None;
        let Some(name) = self
            .authored_math_provider()
            .map(|_| "tcl_precision")
            .or_else(|| {
                SpecialVariableHook::DoublePrecision.name_in(self.native_invocation_dialect())
            })
        else {
            return;
        };
        self.install_native_precision_trace_at(name.as_bytes());
    }

    pub(in super::super) fn install_native_precision_trace_at(&mut self, name: &[u8]) {
        let _ = self.install_original_precision_trace_at(name);
    }

    fn install_original_precision_trace_at(
        &mut self,
        name: &[u8],
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter_native(self)?;
        self.variable_observers.precision_cell = None;
        let selected = self.ensure_base_var_from_bytes(name, 0).map(|(_, id)| id);
        scope.check_native(self)?;
        self.variable_observers.precision_cell = selected;
        scope.check_native(self)
    }

    fn read_trace_precision(
        &mut self,
        id: VarId,
        cell: &VarTraceCell,
        authored: bool,
        policy: tcl_dialect::DoubleStringPolicy,
        scope: &native_operation_currency::NativeOperationScope,
    ) -> Result<Result<Option<u8>, Completion<Value>>, tcl_runtime_api::NativeExecutionError> {
        let original = self
            .initial_value_at_cell(id, cell.array)
            .map(Value::native_lifetime_lease);
        scope.check_native(self)?;
        let reached = if authored {
            original.as_ref().map_or(Ok(None), |lease| {
                let value = lease.value();
                Ok(tcl_syntax::number::parse_double_precision(
                    policy,
                    &value.to_str(),
                ))
            })
        } else if policy == tcl_dialect::DoubleStringPolicy::Tcl84Precision {
            let bytes = original
                .as_ref()
                .map(|lease| {
                    lease
                        .value()
                        .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::C(
                            tcl_dialect::TclVersion::V8_4,
                        ))
                        .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
                        .map_err(tcl_syntax::value::ValueError::from)
                })
                .transpose();
            scope.check_native(self)?;
            match bytes {
                Ok(bytes) => native_numeric_ops::OriginalNumericOps::in_original_scope(self, scope)
                    .and_then(|ops| ops.c84_precision(bytes.as_deref().unwrap_or(&[]))),
                Err(error) => Err(error.into()),
            }
        } else {
            match original.as_ref() {
                Some(lease) => {
                    let value = lease.value();
                    native_numeric_ops::OriginalNumericOps::in_original_scope(self, scope)
                        .and_then(|ops| {
                            ops.probe(
                                value,
                                tcl_syntax::scalar_getter::NativeScalarGetterKind::Int,
                            )
                        })
                        .map(|outcome| match outcome {
                            Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(
                                integer,
                            )) => tcl_syntax::number::double_precision_from_native_integer(
                                policy, integer,
                            ),
                            _ => None,
                        })
                }
                None => Ok(None),
            }
        };
        drop(original);
        scope.check_native(self)?;
        let precision = match reached {
            Ok(precision) => precision,
            Err(error) => return Ok(Err(self.variable_trace_command_error(error)?)),
        };
        Ok(Ok(precision))
    }

    pub(in super::super) fn fire_native_precision_trace_bytes(
        &mut self,
        name: &[u8],
        op: &str,
        cell: &VarTraceCell,
        scope: &native_operation_currency::NativeOperationScope,
    ) -> VariableTraceOutcome {
        scope.check_native(self)?;
        let Some(registered) = self.variable_observers.precision_cell else {
            return Ok(Ok(()));
        };
        if cell.array_id() != Some(registered) && cell.id != Some(registered) {
            return Ok(Ok(()));
        }
        let Some(id) = cell.id else {
            return Ok(Ok(()));
        };
        let dialect = self.native_invocation_dialect();
        if self.authored_math_provider().is_none()
            && tcl_registry::special_vars::SpecialVariableHook::DoublePrecision
                .name_in(dialect)
                .is_none()
        {
            return Ok(Ok(()));
        }
        let Some(context) = self
            .authored_math_format()
            .or_else(|| crate::value::DoubleFormatContext::for_dialect(dialect))
        else {
            return Ok(Ok(()));
        };
        let authored = self.authored_math_provider().is_some();
        let policy = if authored {
            tcl_dialect::DoubleStringPolicy::Tcl84Precision
        } else {
            let Some(policy) = dialect.double_string_policy() else {
                return Ok(Ok(()));
            };
            policy
        };
        if op == "read" {
            if matches!(
                self.var_arena.get(id).map(crate::vars::VarCell::state),
                Some(Local::Array(_))
            ) {
                return Ok(Ok(()));
            }
            self.var_arena.replace_state(
                id,
                Local::Scalar(Value::int(i64::from(context.precision()))),
            );
            scope.check_native(self)?;
            return Ok(Ok(()));
        }
        if op != "write" {
            return Ok(Ok(()));
        }
        // Safe mode is selected before any original getter or Host conversion.
        let error = if self.is_safe {
            Some("can't modify precision from a safe interpreter")
        } else {
            let precision = match self.read_trace_precision(id, cell, authored, policy, scope)? {
                Ok(precision) => precision,
                Err(error) => return Ok(Err(error)),
            };
            if let Some(precision) = precision {
                context.set_precision(precision);
                None
            } else {
                Some("improper value for precision")
            }
        };
        if let Some(error) = error {
            if policy == tcl_dialect::DoubleStringPolicy::Tcl84Precision {
                self.var_arena.replace_state(
                    id,
                    Local::Scalar(Value::int(i64::from(context.precision()))),
                );
                scope.check_native(self)?;
            }
            scope.check_native(self)?;
            // From 8.5 `TclCallVarTraces` starts errorInfo from the trace
            // procedure's own message and logs its `(write trace on "…")`
            // frame; 8.4 starts it again from the access error.
            if !self.uses_jim_error_stack() && self.runtime_version.has_var_trace_error_frames() {
                self.native_errors.error_info = None;
                self.seed_error_info_frame(error.as_bytes(), b"");
                self.append_var_trace_frame_bytes("write", name);
            }
            let mut message = b"can't set \"".to_vec();
            message.extend_from_slice(name);
            message.extend_from_slice(b"\": ");
            message.extend_from_slice(error.as_bytes());
            let code = Value::list(vec![
                Value::string("TCL"),
                Value::string("WRITE"),
                Value::string("VARNAME"),
                Value::from_string_bytes(name),
            ]);
            return Ok(Err(err_with_code(message, code.string_bytes())));
        }
        scope.check_native(self)?;
        Ok(Ok(()))
    }
}
