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

//! C error globals observe separately retained private interpreter objects.

use super::{InterpState, Local, Value, VarId, Vm};
use tcl_registry::special_vars::{NativeErrorStorageVariable as Variable, NativeErrorVariableRead};
use tcl_syntax::value::ValueOps;

/// The two original object names owned by C8.4's interpreter `ExecEnv`.
/// Field order retires errorInfo before errorCode, after the live birth ends.
pub(super) struct C84ExecutionEnvironmentErrorNames {
    birth: std::rc::Rc<()>,
    world: std::rc::Weak<super::native_name_world::NameWorldCell>,
    info: Value,
    code: Value,
}

struct C84ExecutionEnvironmentErrorName {
    birth: std::rc::Weak<()>,
    variable: Variable,
    header: crate::NativeObjectLifetimeLease,
}

impl C84ExecutionEnvironmentErrorName {
    fn check(
        &self,
        vm: &mut Vm,
        scope: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        scope.check(vm)?;
        let current = vm.c84_execution_environment_selected()
            && vm.uses_c84_global_error_info()
            && vm
                .native_errors
                .c84_execution_names
                .as_ref()
                .is_some_and(|names| {
                    self.birth
                        .upgrade()
                        .is_some_and(|birth| std::rc::Rc::ptr_eq(&birth, &names.birth))
                        && names.world.upgrade().is_some_and(|world| {
                            std::rc::Rc::ptr_eq(&world, &vm.name_world) && world.borrow().is_live()
                        })
                        && match self.variable {
                            Variable::ErrorInfo => &names.info,
                            Variable::ErrorCode => &names.code,
                        }
                        .is_same_object(self.header.value())
                });
        scope.require(
            vm,
            current,
            "C8.4 original execution-environment name birth",
        )?;
        scope.current_for_value(vm, self.header.value())
    }
}

pub(super) struct NativeErrorTraceState {
    info: Option<Value>,
    info_len: usize,
    result: Option<crate::value::WeakNativeObject>,
    code: Option<Value>,
    options: Option<Value>,
    bytes: Option<Vec<u8>>,
    logged: crate::interp::ErrorLogged,
}

pub(super) struct NativeGlobalErrorFlags {
    in_progress: bool,
    already_logged: crate::interp::ErrorLogged,
    code_set: bool,
}

impl Vm {
    /// Physical resource provisioning ignores temporary invocation overrides.
    /// An explicitly installed engine survives changes to authored grammar.
    fn c84_execution_environment_selected(&self) -> bool {
        let profile = self.actual_engine_profile.unwrap_or(self.pin.profile);
        tcl_registry::InvocationDialect::of_profile(profile)
            .native_error_log_protocol()
            .is_some_and(
                tcl_registry::native_error_log::NativeErrorLogProtocol::resets_global_error_episode,
            )
    }

    /// Called only at real interpreter construction or provider reconfiguration,
    /// never by a logging consumer that discovers missing representation.
    pub(super) fn configure_c84_execution_environment_error_names(
        &mut self,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let selected = self.c84_execution_environment_selected();
        if !selected && self.native_errors.c84_execution_names.is_none() {
            return Ok(());
        }
        let operation = super::native_operation_currency::NativeOperationScope::enter(self);
        if !selected {
            let retired = self.native_errors.c84_execution_names.take();
            drop(retired);
            return operation?.check(self);
        }
        let operation = operation?;
        operation.check(self)?;
        if let Some(names) = self.native_errors.c84_execution_names.as_ref() {
            let current = names.world.upgrade().is_some_and(|world| {
                std::rc::Rc::ptr_eq(&world, &self.name_world) && world.borrow().is_live()
            });
            if !current {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "C8.4 execution environment interpreter birth",
                )
                .into());
            }
            names.info.check_native_header()?;
            names.code.check_native_header()?;
            return operation.check(self);
        }
        let original_names =
            super::native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
                self,
                Some(tcl_core_types::ROOT_NS),
            )?;
        let reached = (|| -> Result<(), tcl_cmd_core::CmdError> {
            if !self.uses_c84_global_error_info() {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "C8.4 execution environment actual construction provider",
                )
                .into());
            }
            self.with_native_interp_result(|_| ())?;
            original_names.check(self)?;
            let info = Value::new_native_string_bytes(Variable::ErrorInfo.global_name().as_bytes());
            original_names.current_for_value(self, &info)?;
            let code = Value::new_native_string_bytes(Variable::ErrorCode.global_name().as_bytes());
            original_names.current_for_value(self, &code)?;
            let names = C84ExecutionEnvironmentErrorNames {
                birth: std::rc::Rc::new(()),
                world: std::rc::Rc::downgrade(&self.name_world),
                info,
                code,
            };
            self.native_errors.c84_execution_names = Some(names);
            original_names.check(self)
        })();
        let reached = reached.inspect_err(|error| {
            if let Some(cause) = error.native_execution_refusal() {
                self.retain_native_execution_refusal(cause.clone());
            }
        });
        original_names.finish(self)?;
        reached
    }

    fn original_c84_execution_error_name(
        &mut self,
        variable: Variable,
        scope: &super::native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<C84ExecutionEnvironmentErrorName, tcl_cmd_core::CmdError> {
        scope.check(self)?;
        let names = self.native_errors.c84_execution_names.as_ref().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "missing original C8.4 execution-environment name owner",
            ),
        )?;
        let header = match variable {
            Variable::ErrorInfo => &names.info,
            Variable::ErrorCode => &names.code,
        }
        .native_lifetime_lease();
        let name = C84ExecutionEnvironmentErrorName {
            birth: std::rc::Rc::downgrade(&names.birth),
            variable,
            header,
        };
        name.check(self, scope)?;
        Ok(name)
    }

    fn store_c84_execution_error_value(
        &mut self,
        variable: Variable,
        original: Value,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let scope = super::native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
            self,
            Some(tcl_core_types::ROOT_NS),
        )?;
        let reached = (|| -> Result<(), tcl_cmd_core::CmdError> {
            let name = self.original_c84_execution_error_name(variable, &scope)?;
            scope.current_for_value(self, &original)?;
            let reached =
                self.store_original_c84_global_error_object(name.header.value(), original);
            drop(reached);
            name.check(self, &scope)
        })();
        let reached = reached.inspect_err(|error| {
            if let Some(cause) = error.native_execution_refusal() {
                self.retain_native_execution_refusal(cause.clone());
            }
        });
        scope.finish(self)?;
        reached
    }

    fn append_c84_execution_error_value(
        &mut self,
        source: Value,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let scope = super::native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
            self,
            Some(tcl_core_types::ROOT_NS),
        )?;
        let reached = (|| -> Result<(), tcl_cmd_core::CmdError> {
            let name = self.original_c84_execution_error_name(Variable::ErrorInfo, &scope)?;
            scope.current_for_value(self, &source)?;
            let reached = self.append_original_c84_global_error_object(name.header.value(), source);
            drop(reached);
            name.check(self, &scope)
        })();
        let reached = reached.inspect_err(|error| {
            if let Some(cause) = error.native_execution_refusal() {
                self.retain_native_execution_refusal(cause.clone());
            }
        });
        scope.finish(self)?;
        reached
    }

    /// C8.4 trace chains preserve only the three passive error flag bits.
    pub(super) fn capture_native_global_error_flags(&self) -> Option<NativeGlobalErrorFlags> {
        self.uses_c84_global_error_info()
            .then_some(NativeGlobalErrorFlags {
                in_progress: self.native_errors.native_error_legacy_copy,
                already_logged: self.native_errors.error_logged,
                code_set: self.native_errors.c84_global_code_set,
            })
    }

    pub(super) fn restore_native_global_error_flags(
        &mut self,
        saved: Option<NativeGlobalErrorFlags>,
    ) {
        if let Some(saved) = saved {
            self.native_errors.native_error_legacy_copy |= saved.in_progress;
            if !self.native_errors.error_logged.is_logged() {
                self.native_errors.error_logged = saved.already_logged;
            }
            self.native_errors.c84_global_code_set |= saved.code_set;
        }
    }

    /// Keep projected completion codes independent of C8.4's real setter flag.
    pub(crate) fn apply_cmd_error_code(
        &mut self,
        update: tcl_cmd_core::ResolvedCmdErrorCodeUpdate,
        explicit_code_store: bool,
    ) -> Value {
        let replaces = matches!(update, tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(_));
        let code = self.apply_primitive_error_code(update);
        if self.uses_c84_global_error_info() && replaces {
            if explicit_code_store {
                self.restore_guest_error_code(code.clone());
            } else {
                self.native_errors.c84_global_code_set = false;
            }
        }
        code
    }

    pub(crate) fn prepare_error_command_code(&mut self, original: Option<&Value>) -> Value {
        let code = original.cloned().unwrap_or_else(|| Value::string("NONE"));
        if self.uses_c84_global_error_info() {
            if original.is_some() {
                self.restore_guest_error_code(code.clone());
            } else {
                self.native_errors.primitive_error_code =
                    Some(code.native_lifetime_lease().into_value());
            }
            return code.native_lifetime_lease().into_value();
        }
        code
    }

    /// C8.4 error's nonempty info argument extends the current result before
    /// its explicit code setter and final message-result installation.
    pub(crate) fn publish_c84_error_command_info(
        &mut self,
        info: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        if !self.uses_c84_global_error_info() {
            return Ok(false);
        }
        let original = self.with_native_interp_result(Value::native_lifetime_lease)?;
        let bytes = self.native_string_bytes(original.value())?.to_vec();
        self.native_errors.native_error_result = Some(original.value().downgrade_native_object());
        self.seed_c84_global_error_info(&bytes);
        let mut bytes = bytes;
        bytes.extend_from_slice(info);
        self.native_errors.error_info = Some(bytes);
        self.append_c84_global_error_info();
        self.native_errors.error_logged = self.native_errors.error_logged.marked();
        Ok(true)
    }

    /// Publish a command completion through the callback-capable receiver.
    pub(crate) fn publish_native_interp_completion(
        &mut self,
        mut completion: tcl_core_types::Completion<Value>,
    ) -> Result<tcl_core_types::Completion<Value>, crate::TclError> {
        if let Some(cause) = self.execution_refusal.clone() {
            return Err(crate::TclError::from_execution_failure(cause));
        }
        completion.result = self
            .adopt_native_interp_result(completion.result)?
            .into_value();
        self.receive_guest_error_metadata(&completion)?;
        if let Some(cause) = self.execution_refusal.clone() {
            return Err(crate::TclError::from_execution_failure(cause));
        }
        Ok(completion)
    }

    /// Restore the carried original code at the selected native storage owner.
    pub(crate) fn restore_guest_error_code(&mut self, original: Value) {
        let original = if self.uses_c84_global_error_info() {
            // Tcl_SetObjErrorCode stores the SAME global header before setting
            // ERROR_CODE_SET; C8.4 has no separate private error-code object.
            let _ = self.store_original_c84_global_error_code(
                original.native_lifetime_lease().into_value(),
            );
            if self.execution_refusal.is_some() {
                return;
            }
            self.native_errors.c84_global_code_set = true;
            let metadata = original.native_lifetime_lease();
            drop(original);
            metadata.into_value()
        } else {
            original
        };
        self.native_errors.primitive_error_code = Some(original);
    }

    /// Receive transported error fields before command logging supplies defaults.
    /// A reached primitive's already retained original code remains authoritative.
    pub(super) fn receive_guest_error_metadata(
        &mut self,
        completion: &tcl_core_types::Completion<Value>,
    ) -> Result<(), crate::TclError> {
        if completion.code != tcl_core_types::Code::Error
            || completion.option_origin != tcl_core_types::CompletionOptionOrigin::ErrorMetadata
            || self.native_errors.primitive_error_code.is_some()
            || self
                .actual_native_invocation_dialect()
                .native_error_log_protocol()
                .is_none()
            // C8.4's logger already published this episode's code (NONE when
            // no setter ran); a transported option is not a later setter.
            || (self.uses_c84_global_error_info() && self.native_errors.error_logged.is_logged())
        {
            return Ok(());
        }
        let original =
            match crate::command::opt_get_checked(self, &completion.options, b"-errorcode") {
                Ok(original) => original,
                Err(error) => {
                    return Err(error);
                }
            };
        if let Some(original) = original {
            self.restore_guest_error_code(original);
        }
        Ok(())
    }

    /// Publish a reached C9.1 compiler error through its actual `ResetResult`
    /// variable setters after the local message/options array owners exist.
    pub(super) fn publish_original_compiler_syntax(
        &mut self,
        message: &Value,
        options: &Value,
    ) -> Result<(), crate::literal_pool::NativeLiteralUnavailable> {
        use crate::literal_pool::NativeLiteralUnavailable as Error;
        if !self
            .actual_native_invocation_dialect()
            .native_return_options_application(
                tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
            )
            .is_some_and(tcl_registry::native_return_options::NativeReturnOptionsApplicationProtocol::syntax_options_share_message)
        {
            return Err(Error::unavailable(
                "original compiler Syntax publication issuer",
            ));
        }
        let bytes = message
            .resident_string_bytes()
            .ok_or_else(|| Error::unavailable("original compiler Syntax message resident bytes"))?;
        let code = options
            .with_cached_dictionary_member(b"-errorcode", Value::retain_borrowed_member)
            .flatten()
            .ok_or_else(|| Error::unavailable("original compiler Syntax error code"))?;
        self.adopt_native_interp_result(message.clone())
            .map_err(|_| Error::unavailable("original compiler Syntax result owner"))?;
        self.native_errors.native_error_info = Some(message.clone());
        self.native_errors.native_error_info_len = bytes.len();
        self.native_errors.primitive_error_code = Some(code);
        self.native_errors.native_error_legacy_copy = true;
        self.reset_native_jim_result()
            .map_err(|_| Error::unavailable("original compiler Syntax result reset"))?;
        self.reset_native_error_objects();
        self.reset_error_state_for_eval();
        Ok(())
    }

    pub(super) fn save_native_error_trace_state(&self) -> Option<NativeErrorTraceState> {
        self.actual_native_invocation_dialect()
            .native_error_variable_protocol()?;
        Some(NativeErrorTraceState {
            info: self.native_errors.native_error_info.clone(),
            info_len: self.native_errors.native_error_info_len,
            result: self.native_errors.native_error_result.clone(),
            code: self.native_errors.primitive_error_code.clone(),
            options: self.native_errors.native_return_options.clone(),
            bytes: self.native_errors.error_info.clone(),
            logged: self.native_errors.error_logged,
        })
    }

    pub(super) fn install_native_error_variable_traces(&mut self) {
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_none()
        {
            return;
        }
        for variable in [Variable::ErrorInfo, Variable::ErrorCode] {
            self.install_native_error_variable_trace(variable);
        }
    }

    pub(super) fn install_native_error_variable_trace(&mut self, variable: Variable) {
        let old = self
            .variable_observers
            .native_error_cells
            .iter()
            .find_map(|(id, kind)| (*kind == variable).then_some(*id));
        if let Some(id) = old {
            self.take_native_error_variable_trace(id);
        }
        let (_, id) = self
            .ensure_base_var_from(variable.global_name(), 0)
            .expect("native hidden error root shell");
        self.var_arena.pin(id);
        self.variable_observers
            .native_error_cells
            .insert(id, variable);
    }

    pub(super) fn take_native_error_variable_trace(&mut self, id: VarId) -> Option<Variable> {
        let variable = self.variable_observers.native_error_cells.remove(&id)?;
        self.var_arena.unpin(id);
        Some(variable)
    }

    pub(super) fn suspend_native_error_variable_traces(&mut self) {
        let ids: Vec<_> = self
            .variable_observers
            .native_error_cells
            .keys()
            .copied()
            .collect();
        for id in ids {
            self.take_native_error_variable_trace(id);
        }
    }

    pub(super) fn fire_native_error_variable_trace(&mut self, id: VarId, op: &str) {
        let Some(variable) = self.variable_observers.native_error_cells.get(&id).copied() else {
            return;
        };
        if op != "read" || self.limits.confined_stores {
            return;
        }
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_error_variable_protocol()
        else {
            return;
        };
        let defined = self.read_resolved_cell(id).is_some();
        let private = match variable {
            Variable::ErrorInfo => &self.native_errors.native_error_info,
            Variable::ErrorCode => &self.native_errors.primitive_error_code,
        };
        let value = match protocol.read(
            self.native_errors.native_error_legacy_copy,
            private.is_some(),
            defined,
        ) {
            NativeErrorVariableRead::Preserve => None,
            NativeErrorVariableRead::CopyPrivate => private.clone(),
            NativeErrorVariableRead::DefineEmpty => {
                Some(Value::new_native_string_bytes(b"".as_slice()))
            }
        };
        if let Some(value) = value {
            let _ = self.var_arena.replace_state(id, Local::Scalar(value));
        }
    }

    pub(super) fn native_default_error_code(&self, bytes: &[u8]) -> Value {
        let dialect = self.native_invocation_dialect();
        if dialect.native_error_variable_protocol().is_none() {
            return Value::from_string_bytes(bytes);
        }
        let protocol = dialect
            .native_string_protocol()
            .expect("native private code issuer");
        let members = tcl_syntax::list::split_native_list_bytes(bytes, protocol)
            .expect("native structured classification list")
            .into_iter()
            .map(|member| Value::new_native_string_bytes(member.into_owned()))
            .collect();
        Value::native_list_constructor(members, protocol)
    }

    /// Remember the actual result without adding a native object reference.
    pub(crate) fn observe_native_error_result(&mut self, original: &Value) {
        if (self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
            || self.uses_c84_global_error_info())
            && self.native_errors.native_error_info.is_none()
        {
            self.native_errors.native_error_result = Some(original.downgrade_native_object());
        }
    }

    pub(super) fn seed_native_error_result_object(&mut self, bytes: &[u8]) {
        if self.uses_c84_global_error_info() {
            self.seed_c84_global_error_info(bytes);
            return;
        }
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_none()
            || self.native_errors.native_error_info.is_some()
        {
            return;
        }
        // The remembered result is only an identity to share. Once its object
        // is released (an error a host command re-raised from an evaluation
        // of its own) the message bytes are all that is left to start from.
        let original = self
            .native_errors
            .native_error_result
            .as_ref()
            .and_then(crate::value::WeakNativeObject::upgrade)
            .unwrap_or_else(|| Value::new_native_string_bytes(bytes));
        self.native_errors.native_error_info = Some(original);
        self.native_errors.native_error_info_len = bytes.len();
    }

    /// C's `Tcl_AddObjErrorInfo` starts an absent `errorInfo` from the
    /// interpreter result before appending, so a frame appended to an error no
    /// command logged extends the error message itself.
    fn start_native_error_info_from_result(&mut self) {
        if self.native_errors.native_error_info.is_some() {
            return;
        }
        let Some(result) = self
            .native_errors
            .native_error_result
            .as_ref()
            .and_then(crate::value::WeakNativeObject::upgrade)
        else {
            return;
        };
        let Some(message) = self
            .materialization_protocol(&result)
            .and_then(|protocol| result.native_string_bytes(protocol).ok())
        else {
            return;
        };
        let appended = self.native_errors.error_info.take().unwrap_or_default();
        let info = if appended.starts_with(&message) {
            appended
        } else {
            [message.as_ref(), appended.as_slice()].concat()
        };
        self.native_errors.error_info = Some(info);
        self.seed_native_error_result_object(&message);
    }

    pub(super) fn update_native_error_info(&mut self) {
        if self.uses_c84_global_error_info() {
            self.append_c84_global_error_info();
            return;
        }
        let dialect = self.native_invocation_dialect();
        if dialect.native_error_variable_protocol().is_none() {
            return;
        }
        if self.native_errors.error_info.is_none() {
            return;
        }
        self.start_native_error_info_from_result();
        let Some(bytes) = self.native_errors.error_info.clone() else {
            return;
        };
        let outcome = bytes
            .get(self.native_errors.native_error_info_len..)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native errorInfo append extent",
            ))
            .and_then(|suffix| {
                if suffix.is_empty() {
                    return Ok(None);
                }
                let protocol = dialect.native_object_append_protocol(None).ok_or(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native errorInfo append",
                    ),
                )?;
                // An error no result object reached, such as the one a stale
                // frame raises for itself, starts errorInfo from its own text.
                let Some(receiver) = self.native_errors.native_error_info.as_ref() else {
                    return Ok(Some(Value::new_native_string_bytes(bytes.as_slice())));
                };
                tcl_cmd_core::native_append::append_counted_bytes(
                    &crate::value::VmAppendObjects,
                    protocol.recipe(),
                    receiver,
                    suffix,
                )
                .map(Some)
            });
        match outcome {
            Ok(original) => {
                if let Some(original) = original {
                    self.native_errors.native_error_info = Some(original);
                }
                self.native_errors.native_error_info_len = bytes.len();
                if self.native_errors.primitive_error_code.is_none() {
                    self.native_errors.primitive_error_code =
                        Some(self.native_default_error_code(b"NONE"));
                }
                self.native_errors.native_error_legacy_copy = true;
            }
            Err(error) => {
                let _ = crate::command::completion_from_cmd_error(self, error.into());
            }
        }
    }

    /// `Tcl_LogCommandInfo` exposes the private header to non-core observers
    /// while the failing command's frame remains active.
    pub(super) fn publish_traced_native_error_info(&mut self) {
        if self
            .actual_native_invocation_dialect()
            .native_error_variable_protocol()
            .is_none()
        {
            return;
        }
        let traced = self
            .registered_trace_cell_bytes(b"::errorInfo")
            .and_then(|cell| cell.id)
            .is_some_and(|id| {
                self.variable_observers
                    .script_traces
                    .get(&id)
                    .is_some_and(|traces| !traces.is_empty())
            });
        if traced && let Some(original) = self.native_errors.native_error_info.clone() {
            // The setter owns the SAME object. Its ordinary trace walk keeps
            // the interpreter's private error state live across callbacks.
            let _ = self.set_var_bytes(b"::errorInfo", original);
        }
    }

    fn seed_c84_global_error_info(&mut self, bytes: &[u8]) {
        if self.native_errors.native_error_legacy_copy {
            return;
        }
        // The remembered result is only an identity to share. Once its object
        // is released (an error a host command re-raised from an evaluation
        // of its own) the message bytes are all that is left to start from.
        let original = self
            .native_errors
            .native_error_result
            .as_ref()
            .and_then(crate::value::WeakNativeObject::upgrade)
            .unwrap_or_else(|| Value::new_native_string_bytes(bytes));
        // Tcl_AddObjErrorInfo owns no private errorInfo header in C8.4;
        // its first real setter stores the current original result globally.
        self.native_errors.native_error_info_len = bytes.len();
        self.native_errors.native_error_legacy_copy = true;
        if let Err(error) = self.store_c84_execution_error_value(Variable::ErrorInfo, original) {
            let _ = crate::command::completion_from_cmd_error(self, error);
            return;
        }
        if !self.native_errors.c84_global_code_set {
            let default = Value::new_native_string_bytes(b"NONE".as_slice());
            if let Err(error) = self.store_c84_execution_error_value(Variable::ErrorCode, default) {
                let _ = crate::command::completion_from_cmd_error(self, error);
                return;
            }
        }
        self.native_errors.native_error_info_len = bytes.len();
        self.native_errors.native_error_legacy_copy = true;
    }

    fn append_c84_global_error_info(&mut self) {
        let Some(bytes) = self.native_errors.error_info.as_ref() else {
            return;
        };
        let Some(append) = tcl_registry::native_error_log::NativeErrorInfoAppend::from_accumulated(
            bytes,
            self.native_errors.native_error_info_len,
        ) else {
            let _ = self.refuse_host_command("native errorInfo append extent".into());
            return;
        };
        let suffix = append.suffix();
        let length = append.accumulated_length();
        let logged = self.native_errors.error_logged;
        if !suffix.is_empty() {
            let source = Value::new_native_string_bytes(suffix);
            // TCL_APPEND_VALUE reaches the original variable receiver and its
            // actual getter/write traces; guest setter failures are ignored.
            if let Err(error) = self.append_c84_execution_error_value(source) {
                let _ = crate::command::completion_from_cmd_error(self, error);
                return;
            }
        }
        self.native_errors.native_error_info_len = length;
        self.native_errors.native_error_legacy_copy = true;
        self.native_errors.error_logged = logged;
        // The selected native append performs no trailing global getter.
    }

    /// `Tcl_ResetResult`'s error-state half: forget the last error's
    /// return-options overlay and errorInfo so a new failure starts fresh.
    pub(crate) fn reset_native_error_result(&mut self) {
        self.clear_return_error_info();
        self.native_errors.native_return_options = None;
    }

    pub(crate) fn clear_return_error_info(&mut self) {
        self.native_errors.native_error_info = None;
        self.native_errors.native_error_info_len = 0;
        self.native_errors.native_error_result = None;
        self.native_errors.error_info = None;
    }
    pub(crate) fn native_return_error_info(&self) -> Option<&Value> {
        self.native_errors.native_error_info.as_ref()
    }
    pub(crate) fn native_return_error_code(&self) -> Option<&Value> {
        self.native_errors.primitive_error_code.as_ref()
    }

    pub(crate) fn native_merged_return_options(&self) -> Option<&Value> {
        self.native_errors
            .native_return_options
            .as_ref()
            .filter(|value| value.cached_dictionary_bucket_count().is_some())
    }
    pub(crate) fn native_return_controls(&self, completion: tcl_core_types::Code) -> (i32, i64) {
        if completion == tcl_core_types::Code::Return {
            (
                self.native_errors.native_c_return_state.code,
                self.native_errors.native_c_return_state.level,
            )
        } else {
            (
                tcl_syntax::number::native_int32_low_bits(completion.as_int()),
                0,
            )
        }
    }

    pub(crate) fn retain_native_return_options(&mut self, original: &Value) {
        if self
            .actual_native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
        {
            self.native_errors.native_return_options = Some(original.clone());
        }
    }

    pub(crate) fn mark_native_error_copy(&mut self) {
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
        {
            self.native_errors.native_error_legacy_copy = true;
        }
    }

    pub(crate) fn seed_error_info_original(&mut self, original: &Value, bytes: &[u8]) {
        self.seed_error_info(bytes);
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
            && !bytes.is_empty()
        {
            self.native_errors.native_error_info = Some(original.clone());
            self.native_errors.native_error_info_len = bytes.len();
            self.native_errors.native_error_legacy_copy = true;
        }
    }

    pub(super) fn publish_native_error_objects(&mut self, info: &[u8], code: &Value) {
        if self.native_errors.native_error_info.is_none() {
            self.seed_native_error_result_object(info);
        }
        if self.native_errors.primitive_error_code.is_none() {
            self.native_errors.primitive_error_code = Some(code.clone());
        }
        self.native_errors.native_error_legacy_copy = true;
        self.reset_native_error_objects();
    }

    pub(super) fn reset_native_error_objects_before_trace_script(&mut self) {
        if self.native_errors.native_error_legacy_copy
            && self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
        {
            self.reset_native_error_objects();
        }
    }

    fn reset_native_error_objects(&mut self) {
        let protocol = self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .expect("actual native reset");
        for variable in protocol.reset_order() {
            // Ownership moved into set_var is the variable owner. The trace
            // chain separately saves the actual private interpreter fields.
            let argument = self
                .native_errors
                .native_error_legacy_copy
                .then(|| match variable {
                    Variable::ErrorCode => self.native_errors.primitive_error_code.clone(),
                    Variable::ErrorInfo => self.native_errors.native_error_info.clone(),
                })
                .flatten();
            if let Some(argument) = argument {
                let _ = self.set_var_bytes(variable.global_name().as_bytes(), argument);
            }
            match variable {
                Variable::ErrorCode => self.native_errors.primitive_error_code = None,
                Variable::ErrorInfo => self.native_errors.native_error_info = None,
            }
        }
        self.native_errors.native_error_legacy_copy = false;
        self.native_errors.native_return_options = None;
    }
}

impl InterpState {
    pub(crate) fn uses_c84_global_error_info(&self) -> bool {
        self.actual_native_invocation_dialect()
            .native_error_log_protocol()
            .is_some_and(
                tcl_registry::native_error_log::NativeErrorLogProtocol::resets_global_error_episode,
            )
    }

    /// Reset C8.4's active error flags without mutating its global cells.
    pub(super) fn reset_native_global_error_episode(&mut self) {
        if self.uses_c84_global_error_info() {
            self.native_errors.error_info = None;
            self.native_errors.native_error_info_len = 0;
            self.native_errors.native_error_result = None;
            self.native_errors.native_error_legacy_copy = false;
            self.native_errors.c84_global_code_set = false;
            self.native_errors.primitive_error_code = None;
            self.native_errors.error_logged = crate::interp::ErrorLogged::No;
        }
    }
}

impl InterpState {
    pub(super) fn replace_native_error_trace_state(
        &mut self,
        saved: NativeErrorTraceState,
    ) -> NativeErrorTraceState {
        NativeErrorTraceState {
            info: std::mem::replace(&mut self.native_errors.native_error_info, saved.info),
            info_len: std::mem::replace(
                &mut self.native_errors.native_error_info_len,
                saved.info_len,
            ),
            result: std::mem::replace(&mut self.native_errors.native_error_result, saved.result),
            code: std::mem::replace(&mut self.native_errors.primitive_error_code, saved.code),
            options: std::mem::replace(
                &mut self.native_errors.native_return_options,
                saved.options,
            ),
            bytes: std::mem::replace(&mut self.native_errors.error_info, saved.bytes),
            logged: std::mem::replace(&mut self.native_errors.error_logged, saved.logged),
        }
    }
}
