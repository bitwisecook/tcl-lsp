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

//! The option dictionary a completion carries (`-code`, `-level`, `-errorcode`, ...): checked reads, replacement, `return`, and the snapshot of the current completion's options.

use super::error_completion::{completion_from_cmd_error, completion_from_tcl_error, options_dict};
use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;
use tcl_runtime_api::completion_options::{ErrorOptions, OptionValue};
use tcl_runtime_api::{Code, Completion, completion_options as shared_options};

/// Retain a checked failure at its actual interpreter owner before transport.
pub(crate) fn completion_option_failure(
    vm: &mut Vm,
    error: impl Into<tcl_cmd_core::CmdError>,
) -> TclError {
    let completion = completion_from_cmd_error(vm, error.into());
    match vm.execution_refusal.clone() {
        Some(cause) => TclError::from_execution_failure(cause),
        None => TclError::from_completion(completion),
    }
}

/// Exact retained option objects through the original cached Dictionary or
/// checked List owner. Reading an option does not reconstruct its value.
pub(crate) fn completion_option_rows_checked(
    vm: &mut Vm,
    options: &Value,
) -> Result<Vec<(Value, Value)>, TclError> {
    completion_option_rows_using(vm, options, &mut None)
}

pub(crate) trait OriginalCompletionOptions {
    fn current(&self, vm: &mut Vm, value: &Value) -> Result<(), TclError>;
    fn list(&self, vm: &mut Vm, value: &Value) -> Result<Vec<Value>, TclError>;
    fn string(&self, vm: &mut Vm, value: &Value) -> Result<std::rc::Rc<[u8]>, TclError>;
}

fn completion_option_rows_using(
    vm: &mut Vm,
    options: &Value,
    original: &mut Option<&dyn OriginalCompletionOptions>,
) -> Result<Vec<(Value, Value)>, TclError> {
    if let Some(original) = original {
        original.current(vm, options)?;
    }
    if let Some(cause) = vm.execution_refusal.clone() {
        return Err(TclError::from_execution_failure(cause));
    }
    options
        .check_native_header()
        .map_err(|error| completion_option_failure(vm, error))?;
    if let Some(pairs) = options.with_cached_dictionary_representation(|pairs, _| pairs.to_vec()) {
        if let Some(original) = original {
            original.current(vm, options)?;
        }
        return Ok(pairs);
    }
    let items = match original {
        Some(original) => original.list(vm, options)?,
        None => tcl_syntax::value::ValueOps::list_elements(vm, options)
            .map_err(|error| completion_option_failure(vm, error))?,
    };
    if items.len() % 2 != 0 {
        return Err(completion_option_failure(
            vm,
            tcl_syntax::value::ValueError::MissingDictionaryValue,
        ));
    }
    let pairs = items
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| (pair[0].clone(), pair[1].clone()))
        .collect();
    drop(items);
    if let Some(original) = original {
        original.current(vm, options)?;
    }
    Ok(pairs)
}

/// Checked exact-byte lookup. An absent member is independent of a getter or
/// conversion failure; no diagnostic message contributes option identity.
pub(crate) fn opt_get_checked(
    vm: &mut Vm,
    options: &Value,
    key: &[u8],
) -> Result<Option<Value>, TclError> {
    opt_get_checked_using(vm, options, key, None)
}

pub(crate) fn opt_get_checked_using(
    vm: &mut Vm,
    options: &Value,
    key: &[u8],
    mut original: Option<&dyn OriginalCompletionOptions>,
) -> Result<Option<Value>, TclError> {
    if original.is_none() {
        for (name, value) in completion_option_rows_using(vm, options, &mut None)? {
            let bytes = tcl_syntax::value::ValueOps::native_string_bytes(vm, &name)
                .map_err(|error| completion_option_failure(vm, error))?;
            if bytes.as_ref() == key {
                return Ok(Some(value));
            }
        }
        return Ok(None);
    }
    let mut rows = completion_option_rows_using(vm, options, &mut original)?.into_iter();
    let mut found = None;
    for (name, value) in rows.by_ref() {
        let bytes = match original {
            Some(original) => original.string(vm, &name)?,
            None => tcl_syntax::value::ValueOps::native_string_bytes(vm, &name)
                .map_err(|error| completion_option_failure(vm, error))?,
        };
        let matches = bytes.as_ref() == key;
        drop(bytes);
        if matches {
            drop(name);
            if let Some(original) = original {
                original.current(vm, options)?;
            }
            found = Some(value);
            break;
        }
        // Preserve the original reverse local-binding retirement order.
        drop(value);
        if let Some(original) = original {
            original.current(vm, options)?;
        }
        drop(name);
        if let Some(original) = original {
            original.current(vm, options)?;
        }
    }
    drop(rows);
    if let Some(original) = original {
        original.current(vm, options)?;
    }
    Ok(found)
}

/// Read a carried integer at the actual current primitive getter owner.
pub(crate) fn option_integer_checked(
    vm: &mut Vm,
    options: &Value,
    key: &[u8],
    absent: i64,
) -> Result<i64, TclError> {
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(vm)
        .map_err(|error| completion_option_failure(vm, error))?;
    let Some(original) = opt_get_checked(vm, options, key)? else {
        scope
            .check(vm)
            .map_err(|error| completion_option_failure(vm, error))?;
        return Ok(absent);
    };
    let reached = if vm.authored_numeric_context().is_some() {
        tcl_syntax::value::ValueOps::as_int(vm, &original).map_err(tcl_cmd_core::CmdError::from)
    } else {
        crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(vm, &scope)
            .and_then(|ops| {
                ops.getter(
                    &original,
                    tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
                )
            })
            .and_then(|value| match value {
                tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(integer) => Ok(integer),
                _ => Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into()),
            })
    };
    scope
        .check(vm)
        .map_err(|error| completion_option_failure(vm, error))?;
    reached.map_err(|error| completion_option_failure(vm, error))
}

/// Use the shared original completion-code grammar and cache producer.
pub(crate) fn option_code_checked(
    vm: &mut Vm,
    options: &Value,
    absent: Code,
) -> Result<Code, TclError> {
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(vm)
        .map_err(|error| completion_option_failure(vm, error))?;
    let Some(original) = opt_get_checked(vm, options, b"-code")? else {
        scope
            .check(vm)
            .map_err(|error| completion_option_failure(vm, error))?;
        return Ok(absent);
    };
    let (mut ops, protocol) = crate::return_options::selected_in_original_scope(vm, &scope)
        .map_err(|error| completion_option_failure(vm, error))?;
    crate::return_options::with_original_return_ops(vm, &scope, &mut ops, |checked| {
        tcl_cmd_core::return_options::parse_completion_code(checked, protocol, &original)
    })
    .map(Code::from_int)
    .map_err(|error| completion_option_failure(vm, error))
}

/// Carried options or the ordinary standard options when genuinely empty.
fn is_error_metadata_key(key: &[u8]) -> bool {
    matches!(
        key,
        b"-errorcode" | b"-errorinfo" | b"-errorline" | b"-errorstack"
    )
}

pub(crate) fn completion_options(vm: &mut Vm, comp: &Completion<Value>) -> Result<Value, TclError> {
    if completion_option_rows_checked(vm, &comp.options)?.is_empty() {
        Ok(options_dict(comp.code, 0, &[]))
    } else {
        Ok(comp.options.clone())
    }
}

/// Settle a control owner without hiding failure in the carried option value.
pub(crate) fn settle_control_options(
    vm: &mut Vm,
    mut completion: Completion<Value>,
    policy: tcl_runtime_api::completion_options::ControlOptionPolicy,
) -> Completion<Value> {
    if let Some(refused) = vm.refused_completion() {
        return refused;
    }
    if completion.code != Code::Ok {
        return completion;
    }
    let empty = match completion_option_rows_checked(vm, &completion.options) {
        Ok(rows) => rows.is_empty(),
        Err(error) => return completion_from_tcl_error(vm, error),
    };
    if policy.settles_success() || (policy.begins_fresh() && empty) {
        completion.options = options_dict(Code::Ok, 0, &[]);
    }
    completion
}

/// Explicit metadata wins; a genuinely absent code uses NONE.
pub(crate) fn resolved_error_code(
    vm: &mut Vm,
    comp: &Completion<Value>,
) -> Result<Value, TclError> {
    Ok(opt_get_checked(vm, &comp.options, b"-errorcode")?.unwrap_or_else(|| Value::string("NONE")))
}

/// Replace an option while retaining unrelated key/value objects and the
/// original Dictionary versus List storage purpose.
pub(crate) fn with_return_option(
    vm: &mut Vm,
    options: &Value,
    key: &str,
    value: Value,
) -> Result<Value, TclError> {
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(vm)
        .map_err(|error| completion_option_failure(vm, error))?;
    let pairs = completion_option_rows_checked(vm, options)?;
    if options
        .with_cached_dictionary_representation(|_, _| ())
        .is_some()
    {
        let (ops, _) = crate::return_options::selected_in_original_scope(vm, &scope)
            .map_err(|error| completion_option_failure(vm, error))?;
        let original = options.duplicate_native_object_in(ops.string);
        let mut prepared = original
            .prepare_native_dictionary(ops.string)
            .map_err(|error| completion_option_failure(vm, error))?;
        drop(original);
        prepared
            .set_member(Value::new_native_string_bytes(key.as_bytes()), value)
            .map_err(|error| completion_option_failure(vm, error))?;
        let value = prepared.into_value();
        scope
            .check(vm)
            .map_err(|error| completion_option_failure(vm, error))?;
        return Ok(value);
    }
    let mut items = Vec::with_capacity(pairs.len() * 2 + 2);
    let mut replaced = false;
    for (name, original) in pairs {
        let bytes = tcl_syntax::value::ValueOps::native_string_bytes(vm, &name)
            .map_err(|error| completion_option_failure(vm, error))?;
        let matched = bytes.as_ref() == key.as_bytes();
        items.push(name);
        items.push(if matched {
            replaced = true;
            value.clone()
        } else {
            original
        });
    }
    if !replaced {
        items.extend([Value::string(key), value]);
    }
    let value = Value::list(items);
    scope
        .check(vm)
        .map_err(|error| completion_option_failure(vm, error))?;
    Ok(value)
}

/// Procedure-boundary level replacement uses the same checked option owner.
pub(crate) fn with_return_level(
    vm: &mut Vm,
    options: &Value,
    new_level: i64,
) -> Result<Value, TclError> {
    with_return_option(vm, options, "-level", Value::int(new_level))
}

/// Borrow only already resident option storage. Conversion requires the
/// separately supplied interpreter-aware owner; it cannot turn into absence.
pub(crate) fn completion_option_rows_resident(
    options: &Value,
) -> Result<Vec<(Value, Value)>, tcl_syntax::value::ValueError> {
    options.check_native_header()?;
    if let Some(pairs) = options.with_cached_dictionary_representation(|pairs, _| pairs.to_vec()) {
        return Ok(pairs);
    }
    if let Some((items, _)) = options.cached_list_representation() {
        let items = items.lifetime_view();
        if items.len() % 2 != 0 {
            return Err(tcl_syntax::value::ValueError::MissingDictionaryValue);
        }
        return Ok(items
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| (pair[0].clone(), pair[1].clone()))
            .collect());
    }
    if options
        .resident_string_bytes()
        .is_some_and(|bytes| bytes.is_empty())
    {
        return Ok(Vec::new());
    }
    Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
        "borrowed completion options",
    ))
}

/// Borrowed exact-key lookup grants no native conversion or getter capability.
pub(crate) fn opt_get_resident_checked(
    options: &Value,
    key: &[u8],
) -> Result<Option<Value>, tcl_syntax::value::ValueError> {
    for (name, value) in completion_option_rows_resident(options)? {
        name.check_native_header()?;
        let bytes = name.resident_string_bytes().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("borrowed option key"),
        )?;
        if bytes.as_ref() == key {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

/// Test inspection of known compatible resident fixtures.
#[cfg(test)]
pub(crate) fn opt_get(options: &Value, key: &str) -> Option<Value> {
    opt_get_resident_checked(options, key.as_bytes())
        .ok()
        .flatten()
}

/// `return ?-code c? ?-level l? ?-errorcode ec? ?-errorinfo ei? ?value?`.
/// Shared with the `returnStk` opcode (C `INST_RETURN_STK` →
/// `Tcl_SetReturnOptions`), which behaves exactly like
/// `return -options $opts $result`.
pub(crate) fn cmd_return(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    crate::return_options::apply(
        vm,
        args,
        tcl_cmd_core::return_options::ReturnOptionsPurpose::User,
    )
}

/// Apply an internal original-object options dictionary, independently of the
/// exposed return command's argv grammar.
pub(crate) fn apply_return_options(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    crate::return_options::apply(
        vm,
        args,
        tcl_cmd_core::return_options::ReturnOptionsPurpose::InternalDictionary,
    )
}

impl Vm {
    /// Snapshot a completion through the shared standard-options planner.
    /// `catch`, compiled catch ranges, and `try` all use this adapter, so live
    /// error metadata and carried return options cannot drift between them.
    pub(crate) fn completion_options_snapshot(
        &mut self,
        comp: &Completion<Value>,
    ) -> Result<Value, TclError> {
        if let Some(cause) = self.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(cause));
        }
        let merged = match comp.option_origin {
            tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { code, level } => {
                Some((comp.options.clone(), code, level))
            }
            _ => self
                .native_merged_return_options()
                .cloned()
                .map(|original| {
                    let (code, level) = self.native_return_controls(comp.code);
                    (original, code, level)
                }),
        };
        if let Some((options, code, level)) = merged {
            return self.merged_completion_options_snapshot(comp, &options, code, level);
        }
        if self.uses_jim_error_stack() {
            return self
                .jim_return_options(comp.code)
                .map_err(|error| completion_option_failure(self, error));
        }
        let mut carried = Vec::new();
        for (key, value) in completion_option_rows_checked(self, &comp.options)? {
            let bytes = tcl_syntax::value::ValueOps::native_string_bytes(self, &key)
                .map_err(|error| completion_option_failure(self, error))?;
            carried.push((bytes.to_vec(), value));
        }
        let jim_quote = self.native_invocation_dialect().expression_quote_control()
            == Some(tcl_native_model::ExpressionQuoteControl::Jim084);
        if jim_quote && comp.code == Code::Ok {
            carried.retain(|(key, _)| !is_error_metadata_key(key));
        }
        let live = if jim_quote {
            None
        } else {
            self.live_success_error_options(comp)?
        };
        if live.is_some() {
            carried.retain(|(key, _)| !is_error_metadata_key(key));
        }
        let (code, level) = if comp.code == Code::Return {
            (
                option_code_checked(self, &comp.options, Code::Ok)?,
                option_integer_checked(self, &comp.options, b"-level", 1)?,
            )
        } else {
            (
                comp.code,
                if jim_quote && comp.code == Code::Ok {
                    option_integer_checked(self, &comp.options, b"-level", 0)?
                } else {
                    0
                },
            )
        };
        let error = match live {
            Some(live) => Some(live),
            None => self.completion_error_options(comp, code, level)?,
        };
        let rows = shared_options::plan_with_origin(
            self.runtime_version(),
            code,
            level,
            comp.option_origin,
            &carried,
            error.as_ref(),
        );
        Ok(Value::list(
            rows.into_iter()
                .flat_map(|(key, value)| {
                    let value = match value {
                        OptionValue::Integer(value) => Value::int(value),
                        OptionValue::Value(value) => value,
                    };
                    [Value::from_string_bytes(key), value]
                })
                .collect(),
        ))
    }

    /// The interpreter error code, info and line a successful completion
    /// carries, such as a swallowed read's. `Tcl_GetReturnOptions` reports
    /// them after `-code` and `-level`; only a `return` carries its own.
    fn live_success_error_options(
        &mut self,
        comp: &Completion<Value>,
    ) -> Result<Option<ErrorOptions<Value>>, TclError> {
        if matches!(comp.code, Code::Error | Code::Return)
            || matches!(
                comp.option_origin,
                tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { .. }
            )
        {
            return Ok(None);
        }
        let error_code = opt_get_checked(self, &comp.options, b"-errorcode")?;
        let error_info = opt_get_checked(self, &comp.options, b"-errorinfo")?;
        let error_line = if opt_get_checked(self, &comp.options, b"-errorline")?.is_some() {
            Some(option_integer_checked(
                self,
                &comp.options,
                b"-errorline",
                0,
            )?)
        } else {
            None
        };
        Ok(
            (error_code.is_some() || error_info.is_some() || error_line.is_some()).then_some(
                ErrorOptions {
                    error_code,
                    error_info,
                    error_line,
                    ..ErrorOptions::default()
                },
            ),
        )
    }

    fn completion_error_options(
        &mut self,
        comp: &Completion<Value>,
        code: Code,
        level: i64,
    ) -> Result<Option<ErrorOptions<Value>>, TclError> {
        let active_error = code == Code::Error && comp.code == Code::Error && level == 0;
        let error = if code == Code::Error {
            let error_code = Some(resolved_error_code(self, comp)?);
            let error_info = if active_error {
                Some(match self.error_info_value() {
                    Some(bytes) => Value::from_string_bytes(bytes),
                    None => opt_get_checked(self, &comp.options, b"-errorinfo")?
                        .unwrap_or_else(|| comp.result.clone()),
                })
            } else {
                None
            };
            let error_stack = if active_error && self.supports_error_stack() {
                let original = opt_get_checked(self, &comp.options, b"-errorstack")?;
                Some(self.error_stack_for_completion(original))
            } else {
                None
            };
            Some(ErrorOptions {
                error_code,
                error_info,
                error_stack,
                error_line: (active_error && !self.uses_jim_error_stack())
                    .then(|| i64::from(self.error_line())),
                // TIP 329 chaining (`try` handler/finally/bind failures) is a
                // completion option whatever the dictionary's provenance.
                during: opt_get_checked(self, &comp.options, b"-during")?,
            })
        } else {
            None
        };
        Ok(error)
    }

    fn merged_completion_options_snapshot(
        &mut self,
        comp: &Completion<Value>,
        options: &Value,
        code: i32,
        level: i64,
    ) -> Result<Value, TclError> {
        options
            .check_native_header()
            .map_err(|error| completion_option_failure(self, error))?;
        let strings = self
            .native_invocation_dialect()
            .native_string_protocol()
            .ok_or_else(|| {
                completion_option_failure(
                    self,
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "merged return options",
                    ),
                )
            })?;
        let original = options.duplicate_native_object_in(strings);
        let mut copied = original
            .prepare_native_dictionary(strings)
            .map_err(|error| completion_option_failure(self, error))?;
        drop(original);
        let active_error = comp.code == Code::Error && level == 0;
        let live = self.live_success_error_options(comp)?;
        let error = if let Some(live) = live {
            Some(live)
        } else if code == 1 {
            let error_code = match self.native_return_error_code().cloned() {
                Some(value) => value,
                None => resolved_error_code(self, comp)?,
            };
            let error_stack = if active_error && self.supports_error_stack() {
                let original = opt_get_checked(self, &comp.options, b"-errorstack")?;
                Some(self.error_stack_for_completion(original))
            } else {
                None
            };
            // The interpreter's trace is the error's `-errorinfo`: the
            // `return -errorinfo` object while nothing was logged after it,
            // else the trace logging extended (a top-level `return -code
            // error` logs its command, C's `TclEvalEx`).
            let error_info = active_error.then(|| {
                let current = self.error_info_value().map(<[u8]>::to_vec);
                match (self.native_return_error_info().cloned(), current) {
                    (Some(original), Some(current))
                        if original.string_bytes().as_ref() == current.as_slice() =>
                    {
                        original
                    }
                    (_, Some(current)) => Value::new_native_string_bytes(current),
                    (Some(original), None) => original,
                    (None, None) => comp.result.clone(),
                }
            });
            Some(ErrorOptions {
                error_code: Some(error_code),
                error_stack,
                error_info,
                error_line: active_error.then(|| i64::from(self.error_line())),
                during: None,
            })
        } else {
            None
        };
        let overlay = shared_options::plan_with_origin(
            self.runtime_version(),
            Code::from_int(code),
            level,
            tcl_core_types::CompletionOptionOrigin::ErrorMetadata,
            &[],
            error.as_ref(),
        );
        for (key, value) in overlay {
            let value = match value {
                OptionValue::Integer(value) => Value::int(value),
                OptionValue::Value(value) => value,
            };
            copied
                .set_member(Value::new_native_string_bytes(key), value)
                .map_err(|error| completion_option_failure(self, error))?;
        }
        Ok(copied.into_value())
    }

    /// Restore only after checked original metadata access succeeds.
    pub(crate) fn restore_completion_error_state(
        &mut self,
        comp: &Completion<Value>,
    ) -> Result<(), TclError> {
        if comp.code != Code::Error {
            return Ok(());
        }
        let code = opt_get_checked(self, &comp.options, b"-errorcode")?;
        let original = opt_get_checked(self, &comp.options, b"-errorinfo")?
            .unwrap_or_else(|| comp.result.clone());
        let info = tcl_syntax::value::ValueOps::native_string_bytes(self, &original)
            .map_err(|error| completion_option_failure(self, error))?;
        let stack = opt_get_checked(self, &comp.options, b"-errorstack")?;
        let line = match opt_get_checked(self, &comp.options, b"-errorline")? {
            Some(value) => Some(
                tcl_syntax::value::ValueOps::as_int(self, &value)
                    .map_err(|error| completion_option_failure(self, error))?,
            ),
            None => None,
        };
        if let Some(code) = code {
            self.restore_guest_error_code(code);
            if let Some(cause) = self.execution_refusal.clone() {
                return Err(TclError::from_execution_failure(cause));
            }
        }
        self.seed_error_info_original(&original, &info);
        if let Some(stack) = stack {
            self.seed_error_stack(&stack)?;
        }
        if let Some(line) = line.and_then(|value| u32::try_from(value).ok()) {
            self.set_error_line(line);
        }
        Ok(())
    }
}
