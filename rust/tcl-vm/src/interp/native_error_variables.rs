//! C error globals observe separately retained private interpreter objects.

use super::{InterpState, Local, Value, VarId, Vm};
use tcl_registry::special_vars::{NativeErrorStorageVariable as Variable, NativeErrorVariableRead};
use tcl_syntax::value::ValueOps;

/// The two original object names owned by C8.4's interpreter `ExecEnv`.
/// Field order retires errorInfo before errorCode, after the live birth ends.
pub(super) struct C84ExecutionEnvironmentErrorNames {
    birth: std::rc::Rc<()>,
    world: std::rc::Weak<std::cell::RefCell<super::NativeNameWorld>>,
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
        if op != "read" {
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
        let original = match self.native_errors.native_error_result.as_ref() {
            Some(receipt) => {
                if let Some(original) = receipt.upgrade() {
                    original
                } else {
                    let _ = self.refuse_host_command("native error result lifetime expired".into());
                    return;
                }
            }
            None => Value::new_native_string_bytes(bytes),
        };
        self.native_errors.native_error_info = Some(original);
        self.native_errors.native_error_info_len = bytes.len();
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
        let Some(bytes) = self.native_errors.error_info.as_ref() else {
            return;
        };
        let bytes = bytes.clone();
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
                let receiver = self.native_errors.native_error_info.as_ref().ok_or(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native private errorInfo result",
                    ),
                )?;
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
        let original = match self.native_errors.native_error_result.as_ref() {
            Some(receipt) => {
                if let Some(original) = receipt.upgrade() {
                    original
                } else {
                    let _ = self.refuse_host_command("native error result lifetime expired".into());
                    return;
                }
            }
            None => Value::new_native_string_bytes(bytes),
        };
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use tcl_registry::special_vars::NativeBootstrapInputs;
    use tcl_runtime_api::{Code, Completion};

    fn lower_hex(bytes: &[u8]) -> String {
        use std::fmt::Write;
        let mut text = String::new();
        for byte in bytes {
            write!(&mut text, "{byte:02x}").expect("writing to a String cannot fail");
        }
        text
    }

    fn check_repeated_error_episode(
        vm: &mut Vm,
        head: &Value,
        original: &Value,
        fields: &[&str],
        profile: &str,
    ) {
        let completion = vm.invoke_host_original_object_vector(head, &[]);
        assert_eq!(
            completion.code,
            Code::from_int(fields[8].parse().unwrap()),
            "{profile}"
        );
        assert_eq!(
            completion.result.string_bytes().as_ref() == b"X",
            fields[10] == "1",
            "{profile}"
        );
        drop(completion);
        assert!(
            vm.execution_refusal.is_none(),
            "{profile}: {:?}",
            vm.execution_refusal
        );
        let current = vm.read_var_traced_bytes(b"::errorInfo").unwrap().unwrap();
        assert_eq!(
            !current.is_same_object(original),
            fields[9] == "1",
            "{profile}"
        );
        assert_eq!(
            !current
                .string_bytes()
                .windows(b"FIRST_ERROR_LONG".len())
                .any(|bytes| bytes == b"FIRST_ERROR_LONG"),
            fields[11] == "1",
            "{profile}"
        );
        drop(current);
        assert_eq!(
            vm.invoke_host_original_object_vector(head, &[]).code,
            Code::from_int(fields[12].parse().unwrap()),
            "{profile}"
        );
        assert!(
            vm.execution_refusal.is_none(),
            "{profile}: {:?}",
            vm.execution_refusal
        );
        let current = vm.read_var_traced_bytes(b"::errorInfo").unwrap().unwrap();
        assert_eq!(
            !current
                .string_bytes()
                .windows(b"FIRST_ERROR_LONG".len())
                .any(|bytes| bytes == b"FIRST_ERROR_LONG"),
            fields[13] == "1",
            "{profile}"
        );
    }

    #[test]
    fn public_error_resets_preserve_global_headers_and_start_fresh_episodes() {
        let observations = include_str!(
            "../../../tcl-registry/tests/data/native_error_variables/reset-episodes.tsv"
        );
        for (version, profile) in [
            ("8.4.20", "tcl8.4"),
            ("8.5.19", "tcl8.5"),
            ("8.6.18", "tcl8.6"),
            ("9.0.4", "tcl9.0"),
            ("9.1.0", "tcl9.1"),
        ] {
            let row = observations
                .lines()
                .find(|row| row.starts_with(version))
                .unwrap();
            let fields = row.split('\t').collect::<Vec<_>>();
            let mut vm = native_vm(profile);
            assert_eq!(
                vm.eval_source("proc p {} {error FIRST_ERROR_LONG}")
                    .unwrap()
                    .code,
                Code::from_int(fields[1].parse().unwrap()),
                "{profile}"
            );
            let head = Value::new_native_string_bytes(b"p".as_slice());
            assert_eq!(
                vm.invoke_host_original_object_vector(&head, &[]).code,
                Code::from_int(fields[2].parse().unwrap()),
                "{profile}"
            );
            assert!(
                vm.execution_refusal.is_none(),
                "{profile}: {:?}",
                vm.execution_refusal
            );
            // Tcl_GetVar2Ex reaches the native read trace which lends the
            // private C8.5+ header to its public variable cell.
            let original = vm.read_var_traced_bytes(b"::errorInfo").unwrap().unwrap();
            if vm.uses_c84_global_error_info() {
                vm.reset_native_jim_result().unwrap();
                assert_eq!(
                    vm.read_var_traced_bytes(b"::errorInfo")
                        .unwrap()
                        .unwrap()
                        .is_same_object(&original),
                    fields[3] == "1"
                );
                assert_eq!(
                    original.native_object_reference_count(),
                    fields[4].parse::<usize>().unwrap()
                );
                assert!(!vm.native_errors.native_error_legacy_copy);
                assert_eq!(vm.native_errors.native_error_info_len, 0);
            }
            assert_eq!(
                vm.eval_source("proc p {} {error X}").unwrap().code,
                Code::from_int(fields[5].parse().unwrap()),
                "{profile}"
            );
            assert_eq!(
                vm.read_var_traced_bytes(b"::errorInfo")
                    .unwrap()
                    .unwrap()
                    .is_same_object(&original),
                fields[6] == "1",
                "{profile}"
            );
            assert_eq!(
                original.native_object_reference_count(),
                fields[7].parse::<usize>().unwrap(),
                "{profile}"
            );
            check_repeated_error_episode(&mut vm, &head, &original, &fields, profile);
        }
    }

    #[test]
    fn syntax_compiler_reset_publishes_original_without_a_global_literal_owner() {
        let native = include_str!(
            "../../../tcl-registry/tests/data/native_control_expression/syntax-lifecycle-9.1.0.tsv"
        )
        .lines()
        .find(|line| line.starts_with("0\tcompiled\t"))
        .unwrap()
        .split('\t')
        .collect::<Vec<_>>();
        let mut vm = native_vm("tcl9.1");
        let mut literals = tcl_bytecode::LiteralTable::new();
        let message = literals.register_unshared(b"divide by zero");
        let options = literals.register_private_return_options(
            tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral {
                protocol: tcl_syntax::native_string::NativeStringProtocol::C(
                    tcl_dialect::TclVersion::V9_1,
                ),
                words: vec![
                    tcl_runtime_api::native_return_literal::NativeKnownWordLiteral {
                        pieces: vec![b"-errorcode".to_vec()],
                        composite: false,
                    },
                    tcl_runtime_api::native_return_literal::NativeKnownWordLiteral {
                        pieces: vec![b"ARITH DIVZERO {divide by zero}".to_vec()],
                        composite: false,
                    },
                ],
                code: 0,
                level: 1,
                size: 1,
            },
        );
        literals.retain_syntax_error_info(options, message);
        let asm = tcl_bytecode::FunctionAsm {
            literals,
            ..Default::default()
        };
        let pool = vm
            .create_native_literal_pool(&asm, &tcl_runtime_api::ByteNamespacePath::root())
            .unwrap();
        pool.with_original(message, |original| {
            assert_eq!(pool.reference_owners(original), (1, 0));
            assert!(
                vm.get_var_bytes(b"::errorInfo")
                    .unwrap()
                    .is_same_object(original)
            );
            assert_eq!(
                original.native_object_reference_count(),
                native[2].parse::<usize>().unwrap()
            );
            assert!(vm.native_errors.native_error_info.is_none());
            assert!(vm.native_errors.primitive_error_code.is_none());
            vm.set_var_bytes(
                b"::errorInfo",
                Value::new_native_string_bytes(b"later error".as_slice()),
            )
            .unwrap();
            assert_eq!(original.native_object_reference_count(), 2);
            pool.with_original(options, |options| {
                assert!(
                    options
                        .with_cached_dictionary_member(b"-errorinfo", |value| {
                            value.is_some_and(|value| value.is_same_object(original))
                        })
                        .unwrap()
                );
            })
            .unwrap();
        })
        .unwrap();
    }

    fn native_vm(profile: &str) -> Vm {
        let profile = tcl_registry::model::resolve_environment(profile).unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(Vec::<u8>::new()),
            Rc::new(super::super::DefaultHost::new()),
            profile,
            NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm
    }

    fn c84_execution_names(
        vm: &Vm,
    ) -> (
        crate::NativeObjectLifetimeLease,
        crate::NativeObjectLifetimeLease,
    ) {
        let names = vm
            .native_errors
            .c84_execution_names
            .as_ref()
            .expect("actual constructor resource");
        (
            names.info.native_lifetime_lease(),
            names.code.native_lifetime_lease(),
        )
    }

    #[test]
    fn c84_execution_environment_constructor_retains_original_named_headers_and_independent_children()
     {
        let mut vm = native_vm("tcl8.4");
        let original = include_str!(
            "../../../../experiments/name-handling/results/c84-execution-environment-error-name-objects/capture02/execution.stdout"
        );
        let fields = original
            .lines()
            .find(|row| row.starts_with("ENV|birth-before-init|"))
            .unwrap()
            .split('|')
            .collect::<Vec<_>>();
        let (info, code) = c84_execution_names(&vm);
        for (value, offset) in [(info.value(), 6), (code.value(), 10)] {
            assert_eq!(
                value.native_object_reference_count().to_string(),
                fields[offset]
            );
            assert_eq!(value.native_object_type_name(), fields[offset + 1]);
            let hex = lower_hex(&value.resident_string_bytes().unwrap());
            assert_eq!(hex, fields[offset + 3]);
        }
        assert!(!info.value().is_same_object(code.value()));
        vm.create_child(Some("child".to_owned()), false);
        let child = vm.resolve_interp_path("child").unwrap();
        let (child_info, child_code) = vm.in_interp(child, |vm| c84_execution_names(vm));
        for parent in [info.value(), code.value()] {
            for child in [child_info.value(), child_code.value()] {
                assert!(!parent.is_same_object(child));
            }
        }
        assert!(vm.delete_interp(child));
        assert!(!child_info.value().native_object_is_live());
        assert!(!child_code.value().native_object_is_live());
        assert!(info.value().native_object_is_live());
        assert!(code.value().native_object_is_live());
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn c84_execution_environment_reuses_physical_provider_and_retires_its_actual_birth() {
        let mut vm = native_vm("tcl8.4");
        let (info, code) = c84_execution_names(&vm);
        let birth =
            std::rc::Rc::downgrade(&vm.native_errors.c84_execution_names.as_ref().unwrap().birth);
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        let (same_info, same_code) = c84_execution_names(&vm);
        assert!(info.value().is_same_object(same_info.value()));
        assert!(code.value().is_same_object(same_code.value()));
        assert!(birth.upgrade().is_some());
        let c85 = tcl_registry::model::resolve_environment("tcl8.5").unit_profile();
        assert!(vm.set_native_engine_profile(c85));
        assert!(birth.upgrade().is_none());
        assert!(!info.value().native_object_is_live());
        assert!(!code.value().native_object_is_live());
        let c84 = tcl_registry::model::resolve_environment("tcl8.4").unit_profile();
        assert!(vm.set_native_engine_profile(c84));
        let (replacement_info, replacement_code) = c84_execution_names(&vm);
        assert!(!info.value().is_same_object(replacement_info.value()));
        assert!(!code.value().is_same_object(replacement_code.value()));
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn c84_execution_environment_fallback_pin_and_temporary_override_do_not_share_births() {
        let mut vm = Vm::new();
        vm.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        let (info, code) = c84_execution_names(&vm);
        vm.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        let (same_info, same_code) = c84_execution_names(&vm);
        assert!(info.value().is_same_object(same_info.value()));
        assert!(code.value().is_same_object(same_code.value()));
        vm.set_runtime_version(tcl_dialect::TclVersion::V8_5);
        assert!(vm.native_errors.c84_execution_names.is_none());
        assert!(!info.value().native_object_is_live());
        assert!(!code.value().native_object_is_live());
        vm.active_native_profile =
            Some(tcl_registry::model::resolve_environment("tcl8.4").unit_profile());
        vm.configure_c84_execution_environment_error_names()
            .unwrap();
        assert!(vm.native_errors.c84_execution_names.is_none());
        vm.active_native_profile = None;
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn c84_execution_error_logging_uses_the_same_original_parsed_name_headers() {
        let mut vm = native_vm("tcl8.4");
        let (info, code) = c84_execution_names(&vm);
        vm.adopt_native_interp_result(Value::int(777)).unwrap();
        assert!(vm.publish_c84_error_command_info(b" INFO").unwrap());
        let original = include_str!(
            "../../../../experiments/name-handling/results/c84-execution-environment-error-name-objects/capture02/execution.stdout"
        );
        for phase in [
            "after-seed-default-append",
            "after-reset",
            "after-nested-error",
        ] {
            if phase == "after-reset" {
                vm.reset_native_jim_result().unwrap();
            } else if phase == "after-nested-error" {
                assert_eq!(
                    vm.eval_source("namespace eval N {proc p {} {error FAIL}}; N::p")
                        .unwrap()
                        .code,
                    Code::Error
                );
            }
            let row = original
                .lines()
                .find(|row| row.starts_with(&format!("ENV|{phase}|")))
                .unwrap()
                .split('|')
                .collect::<Vec<_>>();
            let (same_info, same_code) = c84_execution_names(&vm);
            assert!(info.value().is_same_object(same_info.value()));
            assert!(code.value().is_same_object(same_code.value()));
            for (value, offset) in [(info.value(), 6), (code.value(), 10)] {
                assert_eq!(
                    value.native_object_reference_count().to_string(),
                    row[offset]
                );
                assert_eq!(value.native_object_type_name(), row[offset + 1]);
                assert!(value.resident_string_bytes().is_some());
            }
            assert!(vm.execution_refusal.is_none());
        }
    }

    #[test]
    fn c84_execution_error_name_missing_or_retired_birth_cannot_be_reconstructed() {
        let mut vm = native_vm("tcl8.4");
        drop(vm.native_errors.c84_execution_names.take());
        assert!(
            vm.store_c84_execution_error_value(Variable::ErrorInfo, Value::int(777))
                .is_err()
        );
        assert!(vm.execution_refusal.is_some());
        assert!(vm.native_errors.c84_execution_names.is_none());

        let mut vm = native_vm("tcl8.4");
        let scope =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let name = vm
            .original_c84_execution_error_name(Variable::ErrorInfo, &scope)
            .unwrap();
        let held_original = name.header.value().clone();
        scope.finish(&mut vm).unwrap();
        assert!(vm.set_native_engine_profile(
            tcl_registry::model::resolve_environment("tcl8.5").unit_profile()
        ));
        assert!(held_original.native_object_is_live());
        assert!(vm.set_native_engine_profile(
            tcl_registry::model::resolve_environment("tcl8.4").unit_profile()
        ));
        let scope =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
        assert!(name.check(&mut vm, &scope).is_err());
        assert!(vm.execution_refusal.is_some());
        assert!(held_original.native_object_is_live());
    }

    thread_local! {
        static C84_CODE_EVENTS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
        static C84_CODE_HEADERS: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    fn observe_c84_code(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let name = vm.native_name_operand_bytes(&args[0]).unwrap().to_vec();
        let operation = vm.native_name_operand_bytes(&args[2]).unwrap().to_vec();
        let rooted = if name.starts_with(b"::") {
            name.clone()
        } else {
            [b"::".as_slice(), name.as_slice()].concat()
        };
        let value = vm
            .get_var_bytes(&rooted)
            .expect("actual traced global cell");
        let bytes = vm.native_string_bytes(&value).unwrap();
        let hex: String = lower_hex(&bytes);
        C84_CODE_EVENTS.with(|events| {
            events.borrow_mut().push(format!(
                "{}|{}|{hex}",
                String::from_utf8(name).unwrap(),
                String::from_utf8(operation).unwrap(),
            ));
        });
        if rooted == b"::errorCode" {
            C84_CODE_HEADERS.with(|headers| {
                headers.borrow_mut().push(value.native_object_identity());
            });
        }
        super::super::ok(Value::empty())
    }

    fn c84_code_vm() -> Vm {
        let mut vm = native_vm("tcl8.4");
        vm.register_command(
            "observe",
            crate::command::Command::Builtin(observe_c84_code),
        );
        assert_eq!(
            vm.eval_source(
                "trace variable ::errorInfo rw observe;trace variable ::errorCode rw observe"
            )
            .unwrap()
            .code,
            Code::Ok
        );
        C84_CODE_EVENTS.with(|events| events.borrow_mut().clear());
        C84_CODE_HEADERS.with(|headers| headers.borrow_mut().clear());
        vm
    }

    fn check_c84_code_controls(controls: &str, sources: &[(&str, &str)]) {
        for row in controls.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let source = sources
                .iter()
                .find(|(name, _)| *name == fields[0])
                .unwrap()
                .1;
            let mut vm = c84_code_vm();
            let completion = vm.eval_source(source).unwrap();
            assert_eq!(completion.code.as_int().to_string(), fields[1], "{row}");
            let result: String = lower_hex(&vm.native_string_bytes(&completion.result).unwrap());
            assert_eq!(result, fields[2], "{row}");
            C84_CODE_EVENTS
                .with(|events| assert_eq!(events.borrow().join(";"), fields[3], "{row}"));
            assert!(
                vm.execution_refusal.is_none(),
                "{row}: {:?}",
                vm.execution_refusal
            );
        }
    }

    #[test]
    fn c84_global_error_code_publication_matches_five_original_callback_controls() {
        check_c84_code_controls(
            include_str!("../../../../runtime/rust/tests/data/native_c84_error_code/controls.tsv"),
            &[
                (
                    "conflict",
                    "package provide probe 1;catch {package provide probe 2} m;list $m $::errorCode",
                ),
                ("no-code", "catch {error FAIL} m;list $m $::errorCode"),
                ("NONE", "catch {error FAIL {} NONE} m;list $m $::errorCode"),
                (
                    "structured",
                    "catch {error FAIL {} {CUSTOM DETAIL}} m;list $m $::errorCode",
                ),
                (
                    "structured-getter",
                    "catch {lindex {} BAD} m;list $m $::errorCode",
                ),
            ],
        );
    }

    thread_local! {
        static C84_STRING_SETTER_MODE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    fn c84_string_setter_probe(vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        let mode = C84_STRING_SETTER_MODE.with(std::cell::Cell::get);
        let rows = include_str!(
            "../../../../experiments/name-handling/results/c84-original-global-string-error-code/capture01/execution.stdout"
        );
        let row = rows
            .lines()
            .find(|row| row.starts_with(&format!("OBS|{mode}|")))
            .unwrap();
        let fields = row.split('|').collect::<Vec<_>>();
        let level = vm.current_level();
        let namespace = vm.current_namespace_execution_context();
        let local = vm
            .get_var_bytes(b"errorCode")
            .unwrap()
            .native_lifetime_lease();
        let supplied = Value::int(777);
        let marker = Value::new_native_string_bytes(b"MARKER".as_slice());
        vm.adopt_native_interp_result(marker).unwrap();
        assert_eq!(vm.native_errors.c84_global_code_set, fields[3] == "1");
        vm.restore_guest_error_code(supplied.clone());
        assert_eq!(vm.native_errors.c84_global_code_set, fields[4] == "1");
        assert_eq!(vm.current_level() == level, fields[5] == "1");
        assert_eq!(
            vm.current_namespace_execution_context()
                .same_context(&namespace),
            fields[6] == "1"
        );
        assert_eq!(
            vm.get_var_bytes(b"errorCode")
                .unwrap()
                .is_same_object(local.value()),
            fields[7] == "1"
        );
        assert_eq!(
            vm.get_var_bytes(b"::errorCode")
                .is_some_and(|global| global.is_same_object(&supplied)),
            fields[8] == "1"
        );
        assert_eq!(
            supplied.resident_string_bytes().is_some(),
            fields[10] == "1"
        );
        let result = vm
            .with_native_interp_result(|result| lower_hex(&result.resident_string_bytes().unwrap()))
            .unwrap();
        assert_eq!(result, fields[11]);
        assert!(vm.execution_refusal.is_none());
        super::super::ok(Value::empty())
    }

    #[test]
    fn c84_string_global_error_code_setter_matches_original_scope_and_quiet_failure() {
        for mode in [0, 3] {
            C84_STRING_SETTER_MODE.with(|selected| selected.set(mode));
            let mut vm = native_vm("tcl8.4");
            vm.register_command(
                "probe",
                crate::command::Command::Builtin(c84_string_setter_probe),
            );
            assert_eq!(vm.eval_source("unset -nocomplain ::errorCode; namespace eval N {variable errorCode NAMESPACE; proc p {} {set errorCode LOCAL; probe; return $errorCode}}").unwrap().code, Code::Ok);
            if mode == 3 {
                assert_eq!(
                    vm.eval_source("array set ::errorCode {key VALUE}")
                        .unwrap()
                        .code,
                    Code::Ok
                );
            }
            let completion = vm.eval_source("N::p").unwrap();
            let rows = include_str!(
                "../../../../experiments/name-handling/results/c84-original-global-string-error-code/capture01/execution.stdout"
            );
            let row = rows
                .lines()
                .find(|row| row.starts_with(&format!("CALL|{mode}|")))
                .unwrap();
            let fields = row.split('|').collect::<Vec<_>>();
            assert_eq!(completion.code.as_int().to_string(), fields[2]);
            let hex = lower_hex;
            assert_eq!(
                hex(&vm.native_string_bytes(&completion.result).unwrap()),
                fields[3]
            );
            let namespace_value = vm.get_var_bytes(b"::N::errorCode").unwrap();
            assert_eq!(
                hex(&vm.native_string_bytes(&namespace_value).unwrap()),
                fields[4]
            );
            assert!(vm.execution_refusal.is_none());
        }
    }

    #[test]
    fn c84_string_global_error_code_setter_refuses_foreign_provider_and_retired_header() {
        let mut vm = native_vm("tcl8.5");
        assert!(
            vm.store_original_c84_global_error_code(Value::int(777))
                .is_err()
        );
        assert!(vm.execution_refusal.is_some());
        let mut vm = native_vm("tcl8.4");
        let supplied = Value::int(777);
        let retired = supplied.native_lifetime_lease();
        drop(supplied);
        assert!(
            vm.store_original_c84_global_error_code(retired.into_value())
                .is_err()
        );
        assert!(vm.execution_refusal.is_some());
        assert!(vm.get_var_bytes(b"::errorCode").is_none());
    }

    #[test]
    fn c84_nonempty_error_info_publication_matches_original_callback_controls() {
        check_c84_code_controls(
            include_str!(
                "../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/controls.tsv"
            ),
            &[
                (
                    "info-no-code",
                    "catch {error FAIL INFO} m;list $m $::errorInfo $::errorCode",
                ),
                (
                    "info-NONE",
                    "catch {error FAIL INFO NONE} m;list $m $::errorInfo $::errorCode",
                ),
                (
                    "info-structured",
                    "catch {error FAIL INFO {CUSTOM DETAIL}} m;list $m $::errorInfo $::errorCode",
                ),
            ],
        );
    }

    #[test]
    fn c84_explicit_error_code_setter_retains_the_same_original_header() {
        let mut vm = c84_code_vm();
        let code = Value::new_native_string_bytes(b"CUSTOM DETAIL".as_slice());
        let arguments = [
            Value::new_native_string_bytes(b"FAIL".as_slice()),
            Value::new_native_string_bytes(b"".as_slice()),
            code.clone(),
        ];
        assert_eq!(
            vm.try_invoke_command("error", &arguments).unwrap().code,
            Code::Error
        );
        C84_CODE_HEADERS.with(|headers| {
            assert_eq!(
                headers.borrow().first().copied(),
                Some(code.native_object_identity())
            );
        });
        assert!(
            vm.get_var_bytes(b"::errorCode")
                .unwrap()
                .is_same_object(&code)
        );
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn command_logging_publishes_same_private_header_only_to_user_traced_cells() {
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = native_vm(profile);
            let public = Value::new_native_string_bytes(b"previous public info".as_slice());
            vm.set_var_bytes(b"::errorInfo", public.clone()).unwrap();
            vm.log_command_info_only(b"first command", b"first error", 1);
            assert!(
                vm.get_var_from(0, "::errorInfo")
                    .unwrap()
                    .is_same_object(&public)
            );

            vm.reset_error_state_for_eval();
            vm.add_var_trace_bytes(
                b"::errorInfo",
                vec!["write".into()],
                &Value::new_native_string_bytes(b"list".as_slice()),
                false,
            );
            vm.log_command_info_only(b"second command", b"second error", 1);
            assert!(vm.execution_refusal.is_none(), "{profile}");
            let private = vm
                .native_errors
                .native_error_info
                .as_ref()
                .expect("restored private error info");
            assert!(
                vm.get_var_from(0, "::errorInfo")
                    .unwrap()
                    .is_same_object(private)
            );
            assert_eq!(
                private.string_bytes().as_ref(),
                b"second error\n    while executing\n\"second command\""
            );
            assert!(
                vm.native_errors.error_logged.is_logged(),
                "{profile} retains the reached log flag"
            );
        }
    }

    thread_local! {
        static RESET_CALLBACKS: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    fn observe_reset(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        assert!(vm.native_errors.native_error_info.is_none());
        assert!(vm.native_errors.primitive_error_code.is_none());
        let name = vm.native_name_operand_bytes(&args[0]).unwrap();
        RESET_CALLBACKS.with(|seen| seen.borrow_mut().push(name.to_vec()));
        super::super::ok(Value::empty())
    }

    fn producer_reference_counts(profile: &str) -> (usize, usize) {
        let fixture = match profile {
            "tcl8.5" => include_str!(
                "../../../tcl-registry/tests/data/native_error_variables/8.5.19-producers.tsv"
            ),
            "tcl8.6" => include_str!(
                "../../../tcl-registry/tests/data/native_error_variables/8.6.18-producers.tsv"
            ),
            "tcl9.0" => include_str!(
                "../../../tcl-registry/tests/data/native_error_variables/9.0.4-producers.tsv"
            ),
            "tcl9.1" => include_str!(
                "../../../tcl-registry/tests/data/native_error_variables/9.1.0-producers.tsv"
            ),
            _ => panic!("native producer fixture"),
        };
        let count = |label: &str| {
            let line = fixture
                .lines()
                .find(|line| line.starts_with(label))
                .unwrap();
            line.split('\t')
                .find_map(|field| field.strip_prefix("info_refs="))
                .unwrap()
                .parse::<usize>()
                .unwrap()
        };
        (count("explicit-error\t"), count("after-reset\t"))
    }

    #[test]
    fn original_private_error_primaries_and_script_reset_match_native_producers() {
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let (before_reset, after_reset) = producer_reference_counts(profile);
            let mut vm = native_vm(profile);
            vm.register_command(
                "observe_native_reset",
                crate::command::Command::Builtin(observe_reset),
            );
            vm.add_var_trace_bytes(
                b"::errorCode",
                vec!["write".into()],
                &Value::string("observe_native_reset"),
                false,
            );
            vm.add_var_trace_bytes(
                b"::errorInfo",
                vec!["write".into()],
                &Value::string("observe_native_reset"),
                false,
            );
            RESET_CALLBACKS.with(|seen| seen.borrow_mut().clear());
            let dialect = vm.native_invocation_dialect();
            let argv = vec![
                Value::string("message"),
                Value::from_native_byte_array(Rc::from(&b"\xff\0a"[..]), dialect).unwrap(),
                Value::from_native_byte_array(Rc::from(&b"\xff\0a"[..]), dialect).unwrap(),
            ];
            let completion = vm.invoke_command_value_at(
                vm.current_ns_id(),
                &Value::string("error"),
                &argv,
                &[],
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            );
            assert_eq!(completion.code, Code::Error, "{profile}");
            assert!(
                vm.native_errors
                    .native_error_info
                    .as_ref()
                    .unwrap()
                    .is_same_object(&argv[1])
            );
            assert!(
                vm.native_errors
                    .primitive_error_code
                    .as_ref()
                    .unwrap()
                    .is_same_object(&argv[2])
            );
            assert!(argv[1].byte_array_representation().is_some());
            assert!(argv[2].cached_list_representation().is_some());
            assert_eq!(
                argv[1].native_object_reference_count(),
                before_reset,
                "{profile}"
            );
            assert_eq!(
                argv[2].native_object_reference_count(),
                before_reset,
                "{profile}"
            );
            drop(completion); // private returnOpts retains the same options header.
            vm.reset_native_error_objects();
            assert!(
                vm.execution_refusal.is_none(),
                "{profile}: {:?}",
                vm.execution_refusal
            );
            RESET_CALLBACKS.with(|seen| {
                assert_eq!(
                    *seen.borrow(),
                    [b"::errorInfo".to_vec(), b"::errorCode".to_vec()],
                    "{profile}"
                );
            });
            assert!(vm.native_errors.native_error_info.is_none());
            assert!(vm.native_errors.primitive_error_code.is_none());
            assert!(
                vm.get_var_bytes(b"::errorInfo")
                    .unwrap()
                    .is_same_object(&argv[1])
            );
            assert!(
                vm.get_var_bytes(b"::errorCode")
                    .unwrap()
                    .is_same_object(&argv[2])
            );
            assert_eq!(
                argv[1].native_object_reference_count(),
                after_reset,
                "{profile}"
            );
            assert_eq!(
                argv[2].native_object_reference_count(),
                after_reset,
                "{profile}"
            );
        }
    }

    #[test]
    fn hidden_error_read_unset_and_root_lifecycle_match_original_c_cells() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = native_vm(profile);
            let enabled = vm
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some();
            let original = Value::string("PRIVATE INFO");
            vm.set_var_bytes(b"::errorInfo", Value::string("GUEST INFO"))
                .unwrap();
            vm.set_var_bytes(b"::errorCode", Value::string("GUEST CODE"))
                .unwrap();
            vm.native_errors.native_error_info = Some(original.clone());
            vm.native_errors.native_error_info_len = b"PRIVATE INFO".len();
            vm.native_errors.native_error_legacy_copy = false;
            assert_eq!(
                vm.read_var_traced_bytes(b"::errorInfo")
                    .unwrap()
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                b"GUEST INFO"
            );
            vm.native_errors.native_error_legacy_copy = true;
            let read = vm.read_var_traced_bytes(b"::errorInfo").unwrap().unwrap();
            if enabled {
                assert!(read.is_same_object(&original));
            } else {
                assert_eq!(read.string_bytes().as_ref(), b"GUEST INFO");
            }
            drop(read);
            assert!(vm.unset_var_bytes(b"::errorInfo"));
            if enabled {
                assert!(
                    vm.read_var_traced_bytes(b"::errorInfo")
                        .unwrap()
                        .unwrap()
                        .is_same_object(&original)
                );
                assert!(vm.unset_var_bytes(b"::errorCode"));
                assert!(!vm.unset_var_bytes(b"::errorCode"));
                vm.native_errors.primitive_error_code = None;
                assert_eq!(
                    vm.read_var_traced_bytes(b"::errorCode")
                        .unwrap()
                        .unwrap()
                        .string_bytes()
                        .len(),
                    0
                );
                vm.add_var_trace_bytes(
                    b"::errorInfo",
                    vec!["read".into()],
                    &Value::string("list"),
                    false,
                );
                vm.remove_var_trace_bytes(b"::errorInfo", &["read".into()], b"list");
                assert_eq!(vm.variable_observers.native_error_cells.len(), 2);
                vm.delete_namespace_token(super::super::ROOT_NS, true);
                assert_eq!(vm.variable_observers.native_error_cells.len(), 2);
                for id in vm.variable_observers.native_error_cells.keys() {
                    assert!(matches!(
                        vm.var_arena.get(*id).unwrap().state(),
                        Local::Undefined
                    ));
                }
            }
        }
    }

    #[test]
    fn original_error_result_is_weak_until_nonempty_info_append() {
        let mut vm = native_vm("tcl9.0");
        let original =
            Value::from_native_byte_array(Rc::from(&b"seed"[..]), vm.native_invocation_dialect())
                .unwrap();
        vm.observe_native_error_result(&original);
        assert_eq!(original.native_object_reference_count(), 1);
        vm.seed_native_error_result_object(b"seed");
        assert!(
            vm.native_errors
                .native_error_info
                .as_ref()
                .unwrap()
                .is_same_object(&original)
        );
        vm.native_errors.error_info = Some(b"seed".to_vec());
        vm.update_native_error_info();
        assert!(
            vm.native_errors
                .native_error_info
                .as_ref()
                .unwrap()
                .is_same_object(&original)
        );
        assert!(original.byte_array_representation().is_some());
        vm.seed_error_info_frame(b"seed", b" context");
        let private = vm.native_errors.native_error_info.as_ref().unwrap();
        assert!(!private.is_same_object(&original));
        assert!(matches!(
            private.native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
        ));
        assert_eq!(private.string_bytes().as_ref(), b"seed context");
        assert!(original.byte_array_representation().is_some());
    }
}
