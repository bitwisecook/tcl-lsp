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

//! The level operand of `uplevel` and `upvar` (`#N`, `N`), frame selection from it, and `uplevel`.

use super::error_completion::{completion_from_cmd_error, completion_from_tcl_error};
use super::{native_frame_numeric, native_wrong_args, original_script_list};
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use tcl_runtime_api::Completion;

#[cfg(test)]
pub(super) struct OriginalFrameLevel<'a> {
    pub(super) value: &'a Value,
    dialect: tcl_registry::InvocationDialect,
    host: std::rc::Rc<dyn tcl_platform::Host>,
}

#[cfg(test)]
impl<'a> OriginalFrameLevel<'a> {
    pub(super) fn selected(
        vm: &Vm,
        value: &'a Value,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        value.check_native_header()?;
        let dialect = vm.native_scalar_carrier_dialect();
        if dialect.native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            value.bind_native_jim_context(
                &crate::interp::InterpState::native_jim_object_context(vm)?,
            )?;
        }
        value.check_native_header()?;
        Ok(Self {
            value,
            dialect,
            host: vm.host_rc(),
        })
    }

    // The existing VM value adapter retains the selected Host and original
    // header. Runtime's entered-interpreter currency is a separate owner.
    pub(super) fn after<T>(
        &self,
        result: Result<T, tcl_syntax::value::ValueError>,
    ) -> Result<T, tcl_syntax::value::ValueError> {
        self.value.check_native_header()?;
        result
    }

    pub(super) fn environment(
        &self,
    ) -> Result<&dyn tcl_platform::NumericEnvironment, tcl_syntax::value::ValueError> {
        self.value.check_native_header()?;
        let actual = self.host.numeric_environment();
        self.after(actual.ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable))
    }
}

#[cfg(test)]
impl tcl_registry::frame_effect::NativeFrameLevelObject for OriginalFrameLevel<'_> {
    type Error = tcl_syntax::value::ValueError;

    fn native_frame_string(&mut self) -> Result<Vec<u8>, Self::Error> {
        self.value.check_native_header()?;
        let protocol = self.dialect.native_string_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("native frame string"),
        )?;
        let formatter = self.host.native_integer_formatter();
        self.value.check_native_header()?;
        let result = self
            .value
            .native_string_bytes_with_integer_formatter(protocol, formatter)
            .map(|bytes| bytes.to_vec())
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
            .map_err(Self::Error::from);
        self.after(result)
    }

    fn native_frame_probe(
        &mut self,
        kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    ) -> Result<Result<i64, tcl_syntax::scalar_getter::NativeScalarGetterFailure>, Self::Error>
    {
        use tcl_syntax::scalar_getter::NativeScalarGetterValue;
        let environment = self.environment()?;
        let result =
            self.value
                .native_scalar_probe_with_environment(self.dialect, kind, Some(environment));
        self.after(result).map(|outcome| {
            outcome.map(|value| match value {
                NativeScalarGetterValue::Wide(integer) => integer,
                _ => unreachable!("integer frame getter"),
            })
        })
    }

    fn native_frame_temporary_int(
        &mut self,
        protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
        bytes: &[u8],
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterConversion, Self::Error> {
        use tcl_syntax::scalar_getter::{NativeScalarGetterKind, NativeScalarGetterStage};
        self.value.check_native_header()?;
        if self.dialect.native_scalar_getter_protocol() != Some(protocol) {
            return Err(Self::Error::ScalarNumericInputUnavailable);
        }
        let environment = self.environment()?;
        self.after(tcl_cmd_core::native_numeric::scalar_getter_recipe_abi(
            protocol,
            NativeScalarGetterKind::Int,
            NativeScalarGetterStage::Fresh,
            environment,
        ))?;
        let result = if protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4) {
            tcl_cmd_core::native_numeric::fresh_c84_conversion(
                protocol,
                NativeScalarGetterKind::Int,
                bytes,
                environment,
            )
        } else {
            protocol
                .fresh_conversion(NativeScalarGetterKind::Int, bytes)
                .ok_or(Self::Error::ScalarNumericInputUnavailable)
        };
        self.after(result)
    }

    fn native_frame_is_integer(&self) -> Result<bool, Self::Error> {
        self.value.check_native_header()?;
        self.after(Ok(matches!(
            self.value.native_scalar_cache(),
            Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. }
            ))
        )))
    }

    fn native_frame_is_machine_integer(&self) -> Result<bool, Self::Error> {
        self.value.check_native_header()?;
        self.after(Ok(matches!(
            self.value.native_scalar_cache(),
            Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Int(_)
            ))
        )))
    }

    fn native_frame_cache(
        &self,
    ) -> Result<Option<tcl_registry::NativeFrameLevelCache>, Self::Error> {
        self.value.check_native_header()?;
        self.after(self.value.native_frame_level_cache_in(self.dialect))
    }

    fn native_frame_set_cache(
        &mut self,
        cache: tcl_registry::NativeFrameLevelCache,
    ) -> Result<(), Self::Error> {
        self.value.check_native_header()?;
        self.after(
            self.value
                .install_native_frame_level_cache(cache, self.dialect),
        )
    }
}

fn native_frame_failure(
    vm: &mut Vm,
    failure: tcl_registry::frame_effect::NativeFrameLevelFailure,
) -> Completion<Value> {
    completion_from_cmd_error(vm, tcl_cmd_core::native_frame_error::present(failure))
}

pub(crate) fn runtime_frame_selection(
    vm: &mut Vm,
    effect: tcl_registry::FrameEffectSpec,
    args: &[Value],
) -> Result<(usize, usize), Completion<Value>> {
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(vm)
        .map_err(|error| completion_from_cmd_error(vm, error))?;
    let dialect = vm.native_scalar_carrier_dialect();
    let Some(protocol) = dialect.native_frame_level_protocol() else {
        return Err(
            vm.refuse_host_command("native original-object frame protocol is unavailable".into())
        );
    };
    let current = vm.current_level();
    let count_width = effect.level_word_len_for_native_bytes(args.len(), None, dialect);
    let default_target = |vm: &mut Vm| {
        current
            .checked_sub(1)
            .map(|target| (0, target))
            .ok_or_else(|| {
                native_frame_failure(
                    vm,
                    tcl_registry::frame_effect::NativeFrameLevelFailure::BadLevel {
                        name: b"1".to_vec(),
                        string_result: protocol.bad_level_string_result(),
                        lookup_code: protocol
                            .tcl_version()
                            .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_6),
                    },
                )
            })
    };
    if count_width == Some(0) {
        return default_target(vm);
    }
    let original = args.first().expect("frame command arity checked");
    if count_width == Some(1) {
        return runtime_frame_selection_for_required_operand(vm, original)
            .map(|target| (1, target));
    }
    if effect == tcl_registry::FrameEffectSpec::UPLEVEL
        && args.len() == 1
        && protocol.probes_single_script_list_first()
        && original.resident_string_bytes().is_none()
    {
        let reached =
            crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(vm, &scope)
                .and_then(|ops| {
                    ops.with_vm_for_value(original, |vm| {
                        Ok(tcl_syntax::value::ValueOps::list_len(vm, original))
                    })
                });
        scope
            .check(vm)
            .map_err(|error| completion_from_cmd_error(vm, error))?;
        let reached = reached.map_err(|error| completion_from_cmd_error(vm, error))?;
        match reached {
            Ok(length) if length > 1 => return default_target(vm),
            Err(error) if error.native_access_refusal().is_some() => {
                return Err(completion_from_cmd_error(vm, error.into()));
            }
            _ => {}
        }
    }
    let resolution = (|| {
        let mut operand = native_frame_numeric::CheckedFrameLevel::selected(vm, original, &scope)?;
        let reached = protocol.resolve_leading_object(current, &mut operand);
        operand.current()?;
        reached
    })();
    scope
        .check(vm)
        .map_err(|error| completion_from_cmd_error(vm, error))?;
    match resolution {
        Ok(Ok(result)) => Ok((
            count_width.unwrap_or(usize::from(result.explicit)),
            result.target,
        )),
        Ok(Err(failure)) => Err(native_frame_failure(vm, failure)),
        Err(error) => Err(completion_from_cmd_error(vm, error)),
    }
}

/// Resolve an already-explicit original level through the selected native getter.
/// This does not decide whether the operand belongs to the script word vector.
pub(crate) fn runtime_explicit_frame_selection(
    vm: &mut Vm,
    original: &Value,
) -> Result<usize, Completion<Value>> {
    runtime_frame_selection_with_requirement(vm, original, false)
}

fn runtime_frame_selection_for_required_operand(
    vm: &mut Vm,
    original: &Value,
) -> Result<usize, Completion<Value>> {
    runtime_frame_selection_with_requirement(vm, original, true)
}

fn runtime_frame_selection_with_requirement(
    vm: &mut Vm,
    original: &Value,
    required: bool,
) -> Result<usize, Completion<Value>> {
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(vm)
        .map_err(|error| completion_from_cmd_error(vm, error))?;
    let dialect = vm.native_scalar_carrier_dialect();
    let protocol = dialect.native_frame_level_protocol().ok_or_else(|| {
        vm.refuse_host_command("native original-object frame protocol is unavailable".into())
    })?;
    let current = vm.current_level();
    let result = (|| {
        let mut operand = native_frame_numeric::CheckedFrameLevel::selected(vm, original, &scope)?;
        let reached = if required {
            protocol.resolve_required_object(current, &mut operand)
        } else {
            protocol.resolve_object(current, &mut operand)
        };
        operand.current()?;
        reached
    })();
    scope
        .check(vm)
        .map_err(|error| completion_from_cmd_error(vm, error))?;
    match result {
        Ok(Ok(selected)) => Ok(selected.target),
        Ok(Err(failure)) => Err(native_frame_failure(vm, failure)),
        Err(error) => Err(completion_from_cmd_error(vm, error)),
    }
}

/// The instruction already consumed an original level and one original script.
/// Preserve that operand boundary even when the level getter chooses its default.
pub(crate) fn compiled_uplevel(vm: &mut Vm, level: &Value, script: Value) -> Completion<Value> {
    let target = match runtime_explicit_frame_selection(vm, level) {
        Ok(target) => target,
        Err(error) => return error,
    };
    let location = script.source_location();
    eval_original_uplevel(vm, target, script, location)
}

/// `uplevel ?level? arg ?arg ...?` — evaluate the concatenated args as a script
/// in the call frame `level` up (default 1, the caller). `#N` selects an
/// absolute level.
pub(crate) fn cmd_uplevel(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return native_wrong_args(vm, "uplevel ?level? command ?arg ...?");
    }
    let (width, target) =
        match runtime_frame_selection(vm, tcl_registry::FrameEffectSpec::UPLEVEL, args) {
            Ok(selected) => selected,
            Err(error) => return error,
        };
    let rest = &args[width..];
    if rest.is_empty() {
        return native_wrong_args(vm, "uplevel ?level? command ?arg ...?");
    }
    let script = if rest.len() == 1 {
        rest[0].clone()
    } else {
        match tcl_cmd_core::list::concat_selected(vm, rest) {
            Ok(value) => value,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    };
    let location = rest
        .first()
        .filter(|_| rest.len() == 1)
        .and_then(Value::source_location);
    eval_original_uplevel(vm, target, script, location)
}

/// Enter the selected variable frame before inspecting or preparing the source.
/// The actual evaluation activation owns restoration across suspension/unwind.
fn eval_original_uplevel(
    vm: &mut Vm,
    target: usize,
    script: Value,
    location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
) -> Completion<Value> {
    let selected = match vm.select_execution_frame(target) {
        Ok(selected) => selected,
        Err(message) => return err(message),
    };
    let direct = match original_script_list(
        vm,
        &script,
        tcl_registry::native_eval_object::EvalObjectPurpose::UpLevel,
    ) {
        Ok(direct) => direct,
        Err(error) => {
            vm.restore_execution_frame(selected);
            return error;
        }
    };
    if let Some(words) = direct {
        vm.pending.control = Some(crate::cmd_control::ControlState::object_eval(
            script, words, "uplevel", selected,
        ));
        return ok(Value::empty());
    }
    // Compile against the selected namespace and physical frame. The deferred
    // activation retains the hidden caller owners until it completes, so every
    // level uses the same yieldable execution protocol.
    match vm.prepare_script_commands_value_for(
        &script,
        tcl_registry::native_eval_object::EvalObjectPurpose::UpLevel,
    ) {
        Ok(prepared) => {
            if let Some(unit) = prepared.prefix {
                let unit = match unit.with_deferred_script_original(script) {
                    Ok(unit) => unit,
                    Err(error) => {
                        vm.restore_execution_frame(selected);
                        return completion_from_cmd_error(vm, error.into());
                    }
                };
                vm.pending.eval = Some(crate::exec::EvalReq {
                    script: unit.with_source_location(location.clone()),
                    label: Some("uplevel"),
                    cleanup_proc: None,
                    fatal_tail: prepared.fatal_tail,
                    selected_frame_restore: selected,
                    package: None,
                });
                ok(Value::empty())
            } else {
                let completion = prepared.fatal_tail.map_or_else(
                    || ok(Value::empty()),
                    |tail| vm.raise_script_parse_failure(tail, location),
                );
                vm.restore_execution_frame(selected);
                completion
            }
        }
        Err(error) => {
            vm.restore_execution_frame(selected);
            completion_from_tcl_error(vm, error)
        }
    }
}
