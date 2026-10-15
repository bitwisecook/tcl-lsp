//! C hidden error-variable traces retain private objects independently of globals.

use super::{new_string, Interp, TraceAccess, GLOBAL};
use crate::obj;
use tcl_registry::special_vars::{NativeErrorStorageVariable as Variable, NativeErrorVariableRead};
use tcl_syntax::value::ValueError;

#[derive(Default, Clone)]
pub(super) struct NativeErrorObjects {
    pub(super) info: Option<obj::Owned>,
    pub(super) info_len: usize,
    pub(super) code: Option<obj::Owned>,
    pub(super) legacy_copy: bool,
    pub(super) global_code_set: bool,
    info_retirement: Option<std::rc::Rc<NativeErrorObjectRetirement>>,
    code_retirement: Option<std::rc::Rc<NativeErrorObjectRetirement>>,
}

/// A raw role during the real decrement, without a native object reference.
/// Its allocation lease cannot revive a retired header or authorise a getter.
struct NativeErrorObjectRetirement {
    _original: obj::NativeObjectLifetime,
    active: std::cell::Cell<bool>,
}

/// These selected workers decrement the old header before clearing its role.
/// Generic byte-authored state replacement keeps its distinct publication order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NativeErrorObjectRetirementPurpose {
    OriginalSetter,
    SelectedReset,
}

impl NativeErrorObjects {
    fn retirement(&self, variable: Variable) -> &Option<std::rc::Rc<NativeErrorObjectRetirement>> {
        match variable {
            Variable::ErrorInfo => &self.info_retirement,
            Variable::ErrorCode => &self.code_retirement,
        }
    }

    fn retirement_mut(
        &mut self,
        variable: Variable,
    ) -> &mut Option<std::rc::Rc<NativeErrorObjectRetirement>> {
        match variable {
            Variable::ErrorInfo => &mut self.info_retirement,
            Variable::ErrorCode => &mut self.code_retirement,
        }
    }
}

pub(super) struct NativeGlobalErrorFlags {
    in_progress: bool,
    already_logged: bool,
    code_set: bool,
}

pub(super) struct NativeErrorTraceState {
    currency: super::native_operation_currency::SavedNativeOperationCurrency,
    originals: NativeErrorObjects,
    info: Option<Vec<u8>>,
    code: Vec<u8>,
    explicit: bool,
    logged: bool,
    primitive_getter: Option<Box<tcl_syntax::scalar_getter::NativeScalarGetterError>>,
    expression_error_stage:
        Option<Box<tcl_registry::native_expression_error::NativeExpressionErrorStage>>,
}

impl Interp {
    pub(super) fn uses_c84_global_error_info(&self) -> bool {
        self.native_invocation_dialect()
            .native_error_log_protocol()
            .is_some_and(
                tcl_registry::native_error_log::NativeErrorLogProtocol::resets_global_error_episode,
            )
    }

    /// Clear C8.4's error-in-progress, already-logged and error-code-set state.
    /// No global variable setter or header publication belongs to this reset.
    pub(super) fn reset_native_global_error_episode(&self) {
        let Ok(scope) = super::native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        if self.uses_c84_global_error_info() {
            let _ = self.replace_exception_state_checked(
                scope.currency(),
                super::ExceptionState::default(),
            );
        }
    }

    pub(super) fn reset_native_global_error_episode_using(
        &self,
        stage: super::native_variable_names::OriginalCNameStage<'_>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.check_original_c_name_stage(stage)?;
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        if self.uses_c84_global_error_info() {
            self.replace_exception_state_using(
                scope.currency(),
                super::ExceptionState::default(),
                stage,
            )?;
        }
        self.check_original_c_name_stage(stage)
    }

    /// Tcl 8.4 CallVarTraces saves only these error-episode flag bits.
    pub(super) fn capture_native_global_error_flags(&self) -> Option<NativeGlobalErrorFlags> {
        self.uses_c84_global_error_info().then(|| {
            let exc = self.exc.borrow();
            NativeGlobalErrorFlags {
                in_progress: exc.native.legacy_copy,
                already_logged: exc.already_logged,
                code_set: exc.native.global_code_set,
            }
        })
    }

    /// Successful C8.4 trace chains OR the original flags into current state;
    /// globals and projected completion data retain their independent owners.
    pub(super) fn restore_native_global_error_flags(&self, saved: Option<NativeGlobalErrorFlags>) {
        if let Some(saved) = saved {
            let mut exc = self.exc.borrow_mut();
            exc.native.legacy_copy |= saved.in_progress;
            exc.already_logged |= saved.already_logged;
            exc.native.global_code_set |= saved.code_set;
        }
    }

    pub(super) fn save_native_error_trace_state(&self) -> Option<NativeErrorTraceState> {
        self.native_invocation_dialect()
            .native_error_variable_protocol()?;
        let currency =
            super::native_operation_currency::SavedNativeOperationCurrency::capture(self);
        let exc = self.exc.borrow();
        Some(NativeErrorTraceState {
            currency,
            originals: exc.native.clone(),
            info: exc.info.clone(),
            code: exc.code.clone(),
            explicit: exc.code_explicit,
            logged: exc.already_logged,
            primitive_getter: exc.primitive_getter.clone(),
            expression_error_stage: exc.expression_error_stage.clone(),
        })
    }

    pub(super) fn restore_native_error_trace_state(
        &self,
        saved: NativeErrorTraceState,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        // Saved-state settlement is mandatory after the callback even when its
        // original entered operation has already refused. Preserve that cause
        // before retiring any current owner, then check again after cleanup.
        let cleanup = super::native_operation_currency::NativeOperationScope::enter_saved_cleanup(
            &saved.currency,
            self,
        )?;
        let retired = {
            let mut exc = self.exc.borrow_mut();
            // Tcl_RestoreInterpState restores ERR_ALREADY_LOGGED, not ERR_LEGACY_COPY.
            let copy = exc.native.legacy_copy;
            let retired = std::mem::replace(&mut exc.native, saved.originals);
            exc.native.legacy_copy = copy;
            exc.info = saved.info;
            exc.code = saved.code;
            exc.code_explicit = saved.explicit;
            exc.already_logged = saved.logged;
            exc.primitive_getter = saved.primitive_getter;
            exc.expression_error_stage = saved.expression_error_stage;
            retired
        };
        drop(retired);
        cleanup.check()
    }

    /// C84 SaveResult restoration must reset the reached current episode even
    /// when its callback has refused. This supplies no continuation permission.
    pub(super) fn settle_native_global_error_episode(&self) {
        if self.uses_c84_global_error_info() {
            let retired = self.exc.replace(super::ExceptionState::default());
            drop(retired);
        }
    }

    pub(super) fn install_native_error_variable_traces(&mut self) {
        let _ = self.install_native_error_variable_traces_checked();
    }

    pub(super) fn install_native_error_variable_traces_checked(
        &mut self,
    ) -> Result<(), super::Code> {
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_none()
        {
            return Ok(());
        }
        for variable in [Variable::ErrorInfo, Variable::ErrorCode] {
            self.install_native_error_variable_trace_checked(variable)?;
        }
        Ok(())
    }

    pub(super) fn install_native_error_variable_trace(&mut self, variable: Variable) {
        let _ = self.install_native_error_variable_trace_checked(variable);
    }

    fn install_native_error_variable_trace_checked(
        &mut self,
        variable: Variable,
    ) -> Result<(), super::Code> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        let name = variable.global_name().as_bytes();
        let allocated = self.prepare_trace_registration(name, None, None);
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| self.refuse_native_execution(first))?;
        let allocated = allocated.map_err(|error| crate::builtins::var_error(self, name, error));
        let home = self.checked_bootstrap_outcome(scope.currency(), allocated)?;
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| self.refuse_native_execution(first))?;
        if let Some(id) = home.binding_id {
            let mut cells = self.native_error_cells.borrow_mut();
            cells.retain(|(kind, _)| *kind != variable);
            cells.push((variable, id));
        }
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| self.refuse_native_execution(first))?;
        Ok(())
    }

    pub(super) fn native_error_variable_at(
        &self,
        home: &crate::vars::TraceHome,
    ) -> Option<Variable> {
        if home.ns != Some(GLOBAL) || home.level.is_some() || home.link_elem.is_some() {
            return None;
        }
        let id = home.binding_id?;
        self.native_error_cells
            .borrow()
            .iter()
            .find_map(|(kind, registered)| (*registered == id).then_some(*kind))
    }

    pub(super) fn fire_native_error_variable_trace(
        &mut self,
        home: &crate::vars::TraceHome,
        access: &TraceAccess,
        op: &[u8],
    ) {
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_error_variable_protocol()
        else {
            return;
        };
        let Some(variable) = self.native_error_variable_at(home) else {
            return;
        };
        if access.match_elem.is_some() || !access.whole_array {
            return;
        }
        if op == b"unset" {
            self.install_native_error_variable_trace(variable);
            return;
        }
        if op != b"read" {
            return;
        }
        let name = variable.global_name().as_bytes();
        if self.check_native_error_object_readable(variable).is_err() {
            return;
        }
        let defined = match self.read_captured_byte_variable(GLOBAL, None, name, None, false) {
            Ok(value) => value.is_some(),
            Err(_) => return,
        };
        let value = {
            let exc = self.exc.borrow();
            let private = match variable {
                Variable::ErrorCode => &exc.native.code,
                Variable::ErrorInfo => &exc.native.info,
            };
            match protocol.read(exc.native.legacy_copy, private.is_some(), defined) {
                NativeErrorVariableRead::CopyPrivate => private.clone(),
                NativeErrorVariableRead::DefineEmpty => Some(obj::Owned::fresh(new_string(b""))),
                NativeErrorVariableRead::Preserve => None,
            }
        };
        if let Some(value) = value {
            // The outer read trace marks this cell active: the intrinsic's
            // ObjSetVar2 cannot recursively run traces on the same cell.
            let _ = self.store_captured_byte_variable(GLOBAL, None, name, None, value.as_ptr());
        }
    }

    fn native_default_error_code(&self, bytes: &[u8]) -> obj::Owned {
        let protocol = self
            .native_invocation_dialect()
            .native_string_protocol()
            .expect("native private error code issuer");
        let words = tcl_syntax::list::split_native_list_bytes(bytes, protocol)
            .expect("structured native error classification list");
        let owners: Vec<_> = words
            .into_iter()
            .map(|word| obj::Owned::fresh(new_string(&word)))
            .collect();
        let pointers: Vec<_> = owners.iter().map(obj::Owned::as_ptr).collect();
        obj::Owned::fresh(crate::list::new_list_obj_native(&pointers, protocol))
    }

    /// Refuse semantic access to a header in its actual decr/free stage.
    /// The raw retirement role is not an absent guest value or an owned value.
    pub(super) fn check_native_error_object_readable(
        &self,
        variable: Variable,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let retiring = self
            .exc
            .borrow()
            .native
            .retirement(variable)
            .as_ref()
            .is_some_and(|role| role.active.get());
        if retiring {
            return Err(self.clone().refuse_completion_value_access(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "retiring private error header",
                ),
            ));
        }
        Ok(())
    }

    /// Complete one selected old-header decrement outside the exception borrow.
    /// OriginalSetter checks before retirement; SelectedReset also completes
    /// the reached setter's mandatory cleanup under its retained first cause.
    pub(super) fn retire_native_error_object_checked(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        variable: Variable,
        purpose: NativeErrorObjectRetirementPurpose,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let reached = currency.ensure_current_or_refuse();
        if purpose == NativeErrorObjectRetirementPurpose::OriginalSetter && reached.is_err() {
            return reached;
        }
        self.check_native_error_object_readable(variable)?;
        let original = {
            let exc = self.exc.borrow();
            match variable {
                Variable::ErrorInfo => &exc.native.info,
                Variable::ErrorCode => &exc.native.code,
            }
            .as_ref()
            .map(obj::Owned::as_ptr)
        };
        let Some(original) = original else {
            currency.ensure_current_or_refuse()?;
            return reached;
        };
        obj::check_native_liveness(original)
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        let role = std::rc::Rc::new(NativeErrorObjectRetirement {
            _original: obj::NativeObjectLifetime::retain(original),
            active: std::cell::Cell::new(true),
        });
        let retired = {
            let mut exc = self.exc.borrow_mut();
            let retired = match variable {
                Variable::ErrorInfo => exc.native.info.take(),
                Variable::ErrorCode => exc.native.code.take(),
            };
            *exc.native.retirement_mut(variable) = Some(role.clone());
            retired
        };
        // The actual native +1 is decremented here. During a free callback the
        // descriptor keeps only the allocation and the selected raw field role.
        drop(retired);
        role.active.set(false);
        let (same, displaced, replaced) = {
            let mut exc = self.exc.borrow_mut();
            let same = exc
                .native
                .retirement(variable)
                .as_ref()
                .is_some_and(|current| std::rc::Rc::ptr_eq(current, &role));
            let displaced = if same {
                exc.native.retirement_mut(variable).take()
            } else {
                None
            };
            let replaced = match variable {
                Variable::ErrorInfo => exc.native.info.is_some(),
                Variable::ErrorCode => exc.native.code.is_some(),
            };
            (same, displaced, replaced)
        };
        drop(displaced);
        drop(role);
        currency.ensure_current_or_refuse()?;
        reached?;
        if !same || replaced {
            return Err(self.clone().refuse_completion_value_access(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "private error retirement role replaced during callback",
                ),
            ));
        }
        Ok(())
    }

    /// Raw header-only observer for the selected field's actual decrement.
    /// The pointer grants no getter, retain, cache access or owned-value role.
    #[cfg(test)]
    pub(super) fn native_error_header_for_observation(
        &self,
        variable: Variable,
    ) -> Option<*mut obj::TclObj> {
        let exc = self.exc.borrow();
        if let Some(role) = exc
            .native
            .retirement(variable)
            .as_ref()
            .filter(|role| role.active.get())
        {
            return Some(role.original.as_ptr());
        }
        match variable {
            Variable::ErrorInfo => &exc.native.info,
            Variable::ErrorCode => &exc.native.code,
        }
        .as_ref()
        .map(obj::Owned::as_ptr)
    }

    /// Replace one genuine private owner and complete its retirement before
    /// the next getter or field publication. This supplies no object authority.
    fn check_native_error_stage<E>(
        &self,
        stage: Option<&dyn Fn() -> Result<(), E>>,
    ) -> Result<(), E> {
        match stage {
            Some(check) => check(),
            None => Ok(()),
        }
    }

    pub(super) fn replace_native_error_object_checked(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        variable: Variable,
        original: Option<obj::Owned>,
        info_len: Option<usize>,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.replace_native_error_object_using(currency, variable, original, info_len, None)
    }

    pub(super) fn replace_native_error_object_using<
        E: From<tcl_runtime_api::NativeExecutionError>,
    >(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        variable: Variable,
        original: Option<obj::Owned>,
        info_len: Option<usize>,
        stage: Option<&dyn Fn() -> Result<(), E>>,
    ) -> Result<(), E> {
        self.check_native_error_stage(stage)?;
        currency.ensure_current_or_refuse().map_err(E::from)?;
        self.check_native_error_object_readable(variable)
            .map_err(E::from)?;
        let published = original
            .as_ref()
            .map(|value| obj::NativeObjectLifetime::retain(value.as_ptr()));
        let retired = {
            let mut exc = self.exc.borrow_mut();
            match variable {
                Variable::ErrorInfo => {
                    if let Some(length) = info_len {
                        exc.native.info_len = length;
                    }
                    std::mem::replace(&mut exc.native.info, original)
                }
                Variable::ErrorCode => std::mem::replace(&mut exc.native.code, original),
            }
        };
        drop(retired);
        self.check_native_error_stage(stage)?;
        currency.ensure_current_or_refuse().map_err(E::from)?;
        if let Some(published) = published {
            self.check_native_error_object(currency, variable, &published)
                .map_err(E::from)?;
            drop(published);
        } else {
            let present = {
                let exc = self.exc.borrow();
                match variable {
                    Variable::ErrorInfo => exc.native.info.is_some(),
                    Variable::ErrorCode => exc.native.code.is_some(),
                }
            };
            if present {
                return Err(E::from(self.clone().refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "cleared native error field replaced during retirement",
                    ),
                )));
            }
        }
        self.check_native_error_stage(stage)?;
        currency.ensure_current_or_refuse().map_err(E::from)
    }

    /// Check the actual private role after an original getter's callback.
    /// Its allocation lease changes no native sharing or reference count.
    pub(super) fn check_native_error_object(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        variable: Variable,
        original: &obj::NativeObjectLifetime,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        self.check_native_error_object_readable(variable)?;
        obj::check_native_liveness(original.as_ptr())
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        let current = {
            let exc = self.exc.borrow();
            match variable {
                Variable::ErrorInfo => &exc.native.info,
                Variable::ErrorCode => &exc.native.code,
            }
            .as_ref()
            .map(obj::Owned::as_ptr)
        };
        if current != Some(original.as_ptr()) {
            let first =
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native error object replaced during access",
                );
            self.clone().refuse_native_access(first);
            return Err(self
                .native_execution_refusal()
                .expect("retained original cause"));
        }
        Ok(())
    }

    /// Retain the supplied operand at its real private-field stage, then reach
    /// the original info getter after displaced-owner cleanup, without a borrow.
    pub(crate) fn retain_native_error_option_checked(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        info: bool,
        original: *mut obj::TclObj,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.retain_native_error_option_using(
            currency,
            info,
            original,
            &|| Ok(()),
            &|value, current| {
                self.native_object_string_bytes_using(value, current, &|error| {
                    self.clone().refuse_completion_value_access(error)
                })
            },
            &|error| self.clone().refuse_completion_value_access(error),
        )
    }

    /// Same actual private-field reference/retirement/getter stages, retaining
    /// the original caller's full error channel and independent frame/backing.
    pub(super) fn retain_native_error_option_using<E>(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        info: bool,
        original: *mut obj::TclObj,
        extra: &dyn Fn() -> Result<(), E>,
        string: &dyn Fn(
            *mut obj::TclObj,
            &dyn Fn() -> Result<(), E>,
        ) -> Result<std::rc::Rc<[u8]>, E>,
        refuse: &dyn Fn(ValueError) -> E,
    ) -> Result<(), E>
    where
        E: From<tcl_runtime_api::NativeExecutionError>,
    {
        currency.ensure_current_or_refuse()?;
        extra()?;
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_none()
        {
            return Ok(());
        }
        obj::check_native_liveness(original).map_err(refuse)?;
        let lifetime = obj::NativeObjectLifetime::retain(original);
        let variable = if info {
            Variable::ErrorInfo
        } else {
            Variable::ErrorCode
        };
        self.replace_native_error_object_checked(
            currency,
            variable,
            Some(obj::Owned::retain(original)),
            None,
        )?;
        extra()?;
        if info {
            let current = || {
                self.check_native_error_object(currency, variable, &lifetime)?;
                extra()
            };
            let bytes = string(lifetime.as_ptr(), &current);
            current()?;
            let bytes = bytes?;
            self.exc.borrow_mut().native.info_len = bytes.len();
        }
        drop(lifetime);
        currency.ensure_current_or_refuse()?;
        extra()
    }

    #[cfg(test)]
    pub(crate) fn retain_native_error_option(&self, info: bool, original: *mut obj::TclObj) {
        let Ok(scope) = super::native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        let _ = self.retain_native_error_option_checked(scope.currency(), info, original);
    }

    /// C8.4's Tcl_SetObjErrorCode publishes the borrowed original globally
    /// before marking ERROR_CODE_SET. The global cell owns its actual role.
    pub(crate) fn publish_original_c84_error_code(&mut self, original: *mut obj::TclObj) {
        if !self.uses_c84_global_error_info() {
            return;
        }
        let stored =
            self.store_captured_byte_variable(GLOBAL, None, b"::errorCode", None, original);
        if stored.is_ok() && self.has_variable_traces() {
            let home = self.trace_identity(b"::errorCode");
            let access = self.trace_access(b"errorCode", b"errorCode", None, &home, false);
            if let Err(first) = self.fire_var_trace_resolved(&home, &access, b"write") {
                self.refuse_native_execution(first);
            }
        }
        // Tcl_SetObjErrorCode ignores setter failure and sets this flag after
        // the actual callback, independently of projected -errorcode bytes.
        self.exc.borrow_mut().native.global_code_set = true;
    }

    /// A nonempty C8.4 error info argument extends the current result through
    /// AddObjErrorInfo before the explicit code and final message setters.
    pub(crate) fn publish_c84_error_command_info(
        &mut self,
        info: &[u8],
        code: &[u8],
    ) -> Result<bool, tcl_runtime_api::NativeExecutionError> {
        if !self.uses_c84_global_error_info() {
            return Ok(false);
        }
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        self.set_error_state(code);
        scope.currency().ensure_current_or_refuse()?;
        let original = obj::NativeObjectLifetime::retain(self.result.get());
        let reached = self.completion_result_bytes_checked();
        self.check_c84_error_info_result(scope.currency(), &original)?;
        let mut bytes = reached?;
        bytes.extend_from_slice(info);
        self.exc.borrow_mut().info = Some(bytes);
        let updated = self.update_native_error_info_checked(scope.currency());
        drop(original);
        scope.currency().ensure_current_or_refuse()?;
        updated?;
        self.exc.borrow_mut().already_logged = true;
        Ok(true)
    }

    pub(super) fn replace_native_error_code(&mut self, bytes: &[u8]) {
        let Ok(scope) = super::native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        let _ = self.replace_native_error_code_checked(scope.currency(), bytes);
    }

    pub(super) fn replace_native_error_code_checked(
        &mut self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        bytes: &[u8],
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.replace_native_error_code_using(currency, bytes, None)
    }

    pub(super) fn replace_native_error_code_using<
        E: From<tcl_runtime_api::NativeExecutionError>,
    >(
        &mut self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        bytes: &[u8],
        stage: Option<&dyn Fn() -> Result<(), E>>,
    ) -> Result<(), E> {
        self.check_native_error_stage(stage)?;
        currency.ensure_current_or_refuse().map_err(E::from)?;
        if self.uses_c84_global_error_info() {
            let original = obj::Owned::fresh(new_string(bytes));
            self.publish_original_c84_error_code(original.as_ptr());
            // The selected C84 setter settles its flag even after a reached
            // callback refusal. Classify only after that mandatory settlement
            // and temporary retirement, retaining the original first cause.
            let first = self.check_native_error_stage(stage);
            drop(original);
            let after = self.check_native_error_stage(stage);
            first?;
            after?;
            return currency.ensure_current_or_refuse().map_err(E::from);
        }
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
        {
            let original = self.native_default_error_code(bytes);
            self.replace_native_error_object_using(
                currency,
                Variable::ErrorCode,
                Some(original),
                None,
                stage,
            )?;
        }
        self.check_native_error_stage(stage)?;
        currency.ensure_current_or_refuse().map_err(E::from)
    }

    /// Publish the complete new state before retiring its genuine old owners.
    /// No exception borrow survives a native header's destructor callback.
    pub(super) fn replace_exception_state_checked(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        state: super::ExceptionState,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.replace_exception_state_using(currency, state, None)
    }

    pub(super) fn replace_exception_state_using<E: From<tcl_runtime_api::NativeExecutionError>>(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        state: super::ExceptionState,
        stage: Option<&dyn Fn() -> Result<(), E>>,
    ) -> Result<(), E> {
        self.check_native_error_stage(stage)?;
        currency.ensure_current_or_refuse().map_err(E::from)?;
        let mut retired = self.exc.replace(state);
        if stage.is_some() {
            // Genuine info/code roles keep their existing field-drop order.
            // Both retire even when the first reached callback withdrew the
            // caller's frame. No exception borrow spans either callback.
            drop(retired.info.take());
            drop(retired.native.info.take());
            let first = self.check_native_error_stage(stage);
            drop(retired.native.code.take());
            let after_code = self.check_native_error_stage(stage);
            drop(retired);
            let after = self.check_native_error_stage(stage);
            first?;
            after_code?;
            after?;
        } else {
            drop(retired);
        }
        self.check_native_error_stage(stage)?;
        currency.ensure_current_or_refuse().map_err(E::from)
    }

    // Compatibility port for actual callers which already check their Scope.
    pub(super) fn capture_native_error_objects(&self) {
        let Ok(scope) = super::native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        let _ = self.capture_native_error_objects_checked(scope.currency());
    }

    /// Capture each byte-authored private role at its original reached stage.
    /// Code retirement finishes before reading and publishing the info role.
    pub(super) fn capture_native_error_objects_checked(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.capture_native_error_objects_using(currency, None)
    }

    pub(super) fn capture_native_error_objects_using<
        E: From<tcl_runtime_api::NativeExecutionError>,
    >(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        stage: Option<&dyn Fn() -> Result<(), E>>,
    ) -> Result<(), E> {
        self.check_native_error_stage(stage)?;
        currency.ensure_current_or_refuse().map_err(E::from)?;
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_none()
        {
            return Ok(());
        }
        let code = {
            let exc = self.exc.borrow();
            if exc.code.is_empty() && !exc.code_explicit {
                b"NONE".to_vec()
            } else {
                exc.code.clone()
            }
        };
        let original = self.native_default_error_code(&code);
        self.replace_native_error_object_using(
            currency,
            Variable::ErrorCode,
            Some(original),
            None,
            stage,
        )?;
        self.check_native_error_stage(stage)?;
        let info = self.exc.borrow().info.clone();
        let original = info
            .as_deref()
            .map(|bytes| obj::Owned::fresh(new_string(bytes)));
        self.replace_native_error_object_using(
            currency,
            Variable::ErrorInfo,
            original,
            Some(info.as_ref().map_or(0, Vec::len)),
            stage,
        )
    }

    /// Seed the byte accumulator and its selected original result role without
    /// borrowing exception state across the getter or displaced-owner retirement.
    pub(super) fn seed_error_info_checked(
        &mut self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        self.check_native_error_object_readable(Variable::ErrorInfo)?;
        self.check_native_error_object_readable(Variable::ErrorCode)?;
        if self.exc.borrow().info.is_some() {
            return Ok(());
        }
        let original = obj::NativeObjectLifetime::retain(self.result.get());
        let bytes = self.completion_result_bytes_checked()?;
        currency.ensure_current_or_refuse()?;
        obj::check_native_liveness(original.as_ptr())
            .map_err(|error| self.refuse_completion_value_access(error))?;
        if self.result.get() != original.as_ptr() {
            return Err(self.refuse_completion_value_access(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original error-info seed result replaced during getter",
                ),
            ));
        }
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
        {
            self.replace_native_error_object_checked(
                currency,
                Variable::ErrorInfo,
                Some(obj::Owned::retain(original.as_ptr())),
                Some(bytes.len()),
            )?;
        }
        self.exc.borrow_mut().info = Some(bytes);
        drop(original);
        currency.ensure_current_or_refuse()
    }

    #[cfg(test)]
    pub(crate) fn update_native_error_info(&mut self) {
        let Ok(scope) = super::native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        let _ = self.update_native_error_info_checked(scope.currency());
    }

    pub(crate) fn update_native_error_info_checked(
        &mut self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        self.check_native_error_object_readable(Variable::ErrorInfo)?;
        self.check_native_error_object_readable(Variable::ErrorCode)?;
        if self.uses_c84_global_error_info() {
            return self.append_c84_global_error_info_checked(currency);
        }
        let dialect = self.native_invocation_dialect();
        if dialect.native_error_variable_protocol().is_none() {
            return Ok(());
        }
        let accumulated = self.exc.borrow().info.clone();
        let original_result = accumulated
            .is_none()
            .then(|| obj::NativeObjectLifetime::retain(self.result.get()));
        let reached = match accumulated {
            Some(bytes) => Ok(bytes),
            None => self.completion_result_bytes_checked(),
        };
        currency.ensure_current_or_refuse()?;
        if let Some(original) = &original_result {
            obj::check_native_liveness(original.as_ptr())
                .map_err(|error| self.refuse_completion_value_access(error))?;
            if self.result.get() != original.as_ptr() {
                return Err(self.refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original error-info result replaced during getter",
                    ),
                ));
            }
        }
        let bytes = reached?;
        if self.exc.borrow().native.info.is_none() {
            self.retain_native_error_option_checked(currency, true, self.result.get())?;
        }
        let (original, length) = {
            let exc = self.exc.borrow();
            (
                obj::NativeObjectLifetime::retain(
                    exc.native
                        .info
                        .as_ref()
                        .expect("initialized private info")
                        .as_ptr(),
                ),
                exc.native.info_len,
            )
        };
        self.check_native_error_object(currency, Variable::ErrorInfo, &original)?;
        let suffix = bytes.get(length..).ok_or_else(|| {
            self.refuse_completion_value_access(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native errorInfo append extent",
                ),
            )
        })?;
        if !suffix.is_empty() {
            let protocol = dialect.native_object_append_protocol(None).ok_or_else(|| {
                self.refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native errorInfo append",
                    ),
                )
            })?;
            let objects = crate::value_ops::RuntimeAppendObjects {
                dialect,
                binary_recipe: dialect.byte_array_string_recipe(None),
            };
            let receiver = crate::value_ops::RuntimeAppendValue::borrowed(original.as_ptr());
            let mut check = || {
                self.check_native_error_object(currency, Variable::ErrorInfo, &original)
                    .map_err(|_| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "entered native errorInfo append",
                        )
                    })
            };
            let outcome = tcl_cmd_core::native_append::append_counted_bytes_checked(
                &objects,
                protocol.recipe(),
                &receiver,
                suffix,
                &mut check,
            );
            currency.ensure_current_or_refuse()?;
            self.check_native_error_object(currency, Variable::ErrorInfo, &original)?;
            let working = outcome.map_err(|error| self.refuse_completion_value_access(error))?;
            // This is the existing genuine working-to-private ownership transfer,
            // after its COW decision; the allocation lease adds no native +1.
            let published = obj::Owned::retain(working.as_ptr());
            drop(working);
            self.check_native_error_object(currency, Variable::ErrorInfo, &original)?;
            self.replace_native_error_object_checked(
                currency,
                Variable::ErrorInfo,
                Some(published),
                None,
            )?;
        }
        self.exc.borrow_mut().native.info_len = bytes.len();
        if self.exc.borrow().native.code.is_none() {
            self.replace_native_error_code_checked(currency, b"NONE")?;
        }
        self.exc.borrow_mut().native.legacy_copy = true;
        drop(original);
        currency.ensure_current_or_refuse()
    }

    fn append_c84_global_error_info_checked(
        &mut self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let Some(bytes) = self.exc.borrow().info.clone() else {
            return Ok(());
        };
        let original = obj::NativeObjectLifetime::retain(self.result.get());
        let reached = (|| {
            let starting = !self.exc.borrow().native.legacy_copy;
            let length = if starting {
                let length = self.completion_result_bytes_checked();
                self.check_c84_error_info_result(currency, &original)?;
                length?.len()
            } else {
                self.exc.borrow().native.info_len
            };
            let append = tcl_registry::native_error_log::NativeErrorInfoAppend::from_accumulated(
                &bytes, length,
            )
            .ok_or_else(|| {
                self.refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native errorInfo append extent",
                    ),
                )
            })?;
            if starting {
                // ERR_IN_PROGRESS is reached before the initial setter callback.
                {
                    let mut exc = self.exc.borrow_mut();
                    exc.native.info_len = length;
                    exc.native.legacy_copy = true;
                }
                let _ = self.var_set(b"::errorInfo", original.as_ptr());
                // Rearm the reached flags before classifying callback cleanup.
                {
                    let mut exc = self.exc.borrow_mut();
                    exc.native.info_len = length;
                    exc.native.legacy_copy = true;
                }
                self.check_c84_error_info_result(currency, &original)?;
                if !self.exc.borrow().native.global_code_set {
                    let code = obj::Owned::fresh(new_string(b"NONE"));
                    let _ = self.var_set(b"::errorCode", code.as_ptr());
                    drop(code);
                    self.check_c84_error_info_result(currency, &original)?;
                }
            }
            let suffix = append.suffix();
            let logged = self.exc.borrow().already_logged;
            let appended = if suffix.is_empty() {
                Ok(super::VariableTraceGuestOutcome::Complete)
            } else {
                // The generated static global name selects the existing C
                // variable owner; the captured physical cell survives the worker.
                let name = obj::Owned::fresh(new_string(b"::errorInfo"));
                let selected = self.capture_original_c_variable_for(
                    name.as_ptr(),
                    tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose::QuietWrite,
                );
                self.check_c84_error_info_result(currency, &original)?;
                let appended = match selected {
                    Ok(captured) => self
                        .append_native_counted_c_variable_bytes_checked(currency, captured, suffix),
                    Err(_) => Ok(super::VariableTraceGuestOutcome::GuestError),
                };
                drop(name);
                appended
            };
            // A reached append's flags settle before Host classification;
            // genuine Guest setter failure is ignored by Tcl_AddObjErrorInfo.
            {
                let mut exc = self.exc.borrow_mut();
                exc.native.info_len = append.accumulated_length();
                exc.native.legacy_copy = true;
                exc.already_logged = logged;
            }
            self.check_c84_error_info_result(currency, &original)?;
            let _ = appended?;
            // Tcl_AddObjErrorInfo ends at the counted setter. A subsequent
            // global getter belongs to its own caller, not this append purpose.
            currency.ensure_current_or_refuse()
        })();
        drop(original);
        currency.ensure_current_or_refuse()?;
        reached
    }

    fn check_c84_error_info_result(
        &mut self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        original: &obj::NativeObjectLifetime,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        obj::check_native_liveness(original.as_ptr())
            .map_err(|error| self.refuse_completion_value_access(error))?;
        if self.result.get() != original.as_ptr() {
            return Err(self.refuse_completion_value_access(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original C84 errorInfo result replaced during callback",
                ),
            ));
        }
        Ok(())
    }

    /// Publish the SAME private header at `Tcl_LogCommandInfo`'s reached
    /// compatibility setter without creating an additional object owner.
    pub(super) fn publish_traced_native_error_info(&mut self) {
        let Ok(scope) = super::native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        let _ = self.publish_traced_native_error_info_checked(scope.currency());
    }

    fn publish_traced_native_error_info_checked(
        &mut self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let original = {
            let exc = self.exc.borrow();
            exc.native
                .info
                .as_ref()
                .map(|value| obj::NativeObjectLifetime::retain(value.as_ptr()))
        }
        .ok_or_else(|| {
            self.refuse_completion_value_access(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "private errorInfo logging publication",
                ),
            )
        })?;
        self.check_native_error_object(currency, Variable::ErrorInfo, &original)?;
        // The current private field lends its actual pointer. The selected
        // variable setter and trace saved state own their independent roles.
        let _ = self.var_set(b"::errorInfo", original.as_ptr());
        self.check_native_error_object(currency, Variable::ErrorInfo, &original)?;
        drop(original);
        currency.ensure_current_or_refuse()
    }

    pub(crate) fn native_private_error_object_checked(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        info: bool,
    ) -> Result<Option<obj::Owned>, tcl_runtime_api::NativeExecutionError> {
        let variable = if info {
            Variable::ErrorInfo
        } else {
            Variable::ErrorCode
        };
        currency.ensure_current_or_refuse()?;
        self.check_native_error_object_readable(variable)?;
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_none()
        {
            return Ok(None);
        }
        let pointer = {
            let exc = self.exc.borrow();
            if info {
                &exc.native.info
            } else {
                &exc.native.code
            }
            .as_ref()
            .map(obj::Owned::as_ptr)
        };
        let Some(pointer) = pointer else {
            return Ok(None);
        };
        obj::check_native_liveness(pointer)
            .map_err(|error| self.clone().refuse_completion_value_access(error))?;
        let original = obj::NativeObjectLifetime::retain(pointer);
        self.check_native_error_object(currency, variable, &original)?;
        let held = obj::Owned::retain(pointer);
        drop(original);
        currency.ensure_current_or_refuse()?;
        Ok(Some(held))
    }

    #[cfg(test)]
    pub(crate) fn native_private_error_object(&self, info: bool) -> Option<obj::Owned> {
        self.check_native_error_object_readable(if info {
            Variable::ErrorInfo
        } else {
            Variable::ErrorCode
        })
        .ok()?;
        self.native_invocation_dialect()
            .native_error_variable_protocol()?;
        let exc = self.exc.borrow();
        if info {
            exc.native.info.clone()
        } else {
            exc.native.code.clone()
        }
    }

    pub(super) fn reset_native_error_objects_before_trace_script(&mut self) {
        if self.exc.borrow().native.legacy_copy {
            self.publish_native_error_objects();
        }
    }

    pub(super) fn publish_native_error_objects(&mut self) {
        let Ok(scope) = super::native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        let _ = self.publish_native_error_objects_checked(scope.currency());
    }

    pub(super) fn publish_native_error_objects_checked(
        &mut self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_error_variable_protocol()
        else {
            return Ok(());
        };
        for &variable in protocol.reset_order() {
            let original = {
                let exc = self.exc.borrow();
                if !exc.native.legacy_copy {
                    None
                } else {
                    match variable {
                        Variable::ErrorInfo => &exc.native.info,
                        Variable::ErrorCode => &exc.native.code,
                    }
                    .as_ref()
                    .map(|value| obj::NativeObjectLifetime::retain(value.as_ptr()))
                }
            };
            if let Some(original) = &original {
                self.check_native_error_object(currency, variable, original)?;
                // Preserve the genuine borrowed private role throughout this
                // reached setter. This allocation lease adds no native +1.
                let _ = self.var_set(variable.global_name().as_bytes(), original.as_ptr());
            }
            // The reached setter's field cleanup is mandatory, including when
            // its callback retained a first cause. The shared descriptor keeps
            // the old raw role until the actual native decrement completes.
            let reached = self.retire_native_error_object_checked(
                currency,
                variable,
                NativeErrorObjectRetirementPurpose::SelectedReset,
            );
            drop(original);
            currency.ensure_current_or_refuse()?;
            reached?;
        }
        self.exc.borrow_mut().native.legacy_copy = false;
        self.clear_return_options_checked()?;
        currency.ensure_current_or_refuse()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{default_host, obj_bytes, ExceptionState};
    use super::*;
    use crate::{counters, environment::profile_for_dialect};
    use tcl_registry::special_vars::NativeBootstrapInputs;

    thread_local! {
        static C84_LOG_WRITES: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
        static LOG_HEADERS: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    fn observe_c84_log(interp: &mut Interp, _arguments: &[*mut obj::TclObj]) -> super::super::Code {
        let original = interp.var_get_at(b"::errorInfo", 0).unwrap();
        C84_LOG_WRITES.with(|writes| writes.borrow_mut().push(obj_bytes(original)));
        LOG_HEADERS.with(|headers| headers.borrow_mut().push(original as usize));
        super::super::Code::Ok
    }

    fn check_repeated_error_episode(
        interp: &mut Interp,
        head: &obj::Owned,
        original: &obj::Owned,
        fields: &[&str],
        profile: &str,
    ) {
        use super::super::Code;
        assert_eq!(
            interp.eval_original_object_vector(&[head.as_ptr()]),
            Code::from_int(fields[8].parse().unwrap()),
            "{profile}"
        );
        assert!(
            !interp.host_refusal_pending(),
            "{profile}: {:?}",
            interp.native_access_refusal()
        );
        let current = interp.read_named_variable(b"::errorInfo").unwrap();
        assert_eq!(current != original.as_ptr(), fields[9] == "1", "{profile}");
        assert_eq!(
            interp.result_bytes() == b"X",
            fields[10] == "1",
            "{profile}"
        );
        assert_eq!(
            !obj_bytes(current)
                .windows(b"FIRST_ERROR_LONG".len())
                .any(|bytes| bytes == b"FIRST_ERROR_LONG"),
            fields[11] == "1",
            "{profile}"
        );
        assert_eq!(
            interp.eval_original_object_vector(&[head.as_ptr()]),
            Code::from_int(fields[12].parse().unwrap()),
            "{profile}"
        );
        assert!(
            !interp.host_refusal_pending(),
            "{profile}: {:?}",
            interp.native_access_refusal()
        );
        let current = interp.read_named_variable(b"::errorInfo").unwrap();
        assert_eq!(
            !obj_bytes(current)
                .windows(b"FIRST_ERROR_LONG".len())
                .any(|bytes| bytes == b"FIRST_ERROR_LONG"),
            fields[13] == "1",
            "{profile}"
        );
    }

    // Native proof: naming.variable.error-reset-distinct-episodes
    // docs/design/analysis/name-resolution-proofs/variable.error-reset-distinct-episodes.md
    #[test]
    fn public_error_resets_preserve_global_headers_and_start_fresh_episodes() {
        use super::super::Code;
        let observations = include_str!(
            "../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv"
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
            counters::reset();
            {
                let mut interp = Interp::with_native_core(
                    default_host(),
                    profile_for_dialect(profile),
                    NativeBootstrapInputs::default(),
                )
                .unwrap();
                assert_eq!(
                    interp.eval_str(b"proc p {} {error FIRST_ERROR_LONG}"),
                    Code::from_int(fields[1].parse().unwrap()),
                    "{profile}"
                );
                let head = obj::Owned::fresh(new_string(b"p"));
                assert_eq!(
                    interp.eval_original_object_vector(&[head.as_ptr()]),
                    Code::from_int(fields[2].parse().unwrap()),
                    "{profile}"
                );
                assert!(
                    !interp.host_refusal_pending(),
                    "{profile}: {:?}",
                    interp.native_access_refusal()
                );
                let original =
                    obj::Owned::retain(interp.read_named_variable(b"::errorInfo").unwrap());
                if interp.uses_c84_global_error_info() {
                    // C8.4 TclEvalObjvInternal resets even the empty vector.
                    assert_eq!(interp.eval_original_object_vector(&[]), Code::Ok);
                    assert_eq!(
                        interp.read_named_variable(b"::errorInfo").unwrap() == original.as_ptr(),
                        fields[3] == "1"
                    );
                    assert_eq!(
                        unsafe { (*original.as_ptr()).ref_count },
                        fields[4].parse().unwrap()
                    );
                    assert!(!interp.exc.borrow().native.legacy_copy);
                    assert_eq!(interp.exc.borrow().native.info_len, 0);
                }
                assert_eq!(
                    interp.eval_str(b"proc p {} {error X}"),
                    Code::from_int(fields[5].parse().unwrap()),
                    "{profile}"
                );
                assert_eq!(
                    interp.read_named_variable(b"::errorInfo").unwrap() == original.as_ptr(),
                    fields[6] == "1",
                    "{profile}"
                );
                assert_eq!(
                    unsafe { (*original.as_ptr()).ref_count },
                    fields[7].parse().unwrap(),
                    "{profile}"
                );
                check_repeated_error_episode(&mut interp, &head, &original, &fields, profile);
            }
            assert_eq!(counters::finalize(), 0, "{profile}");
        }
    }

    thread_local! {
        static C84_CODE_EVENTS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
        static C84_CODE_HEADERS: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    fn observe_c84_code(interp: &mut Interp, argv: &[*mut obj::TclObj]) -> super::super::Code {
        let name = obj_bytes(argv[1]);
        let operation = obj_bytes(argv[3]);
        let rooted = if name.starts_with(b"::") {
            name.clone()
        } else {
            [b"::".as_slice(), name.as_slice()].concat()
        };
        let value = interp.var_get(&rooted).expect("actual traced global cell");
        let bytes = obj_bytes(value);
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        C84_CODE_EVENTS.with(|events| {
            events.borrow_mut().push(format!(
                "{}|{}|{hex}",
                String::from_utf8(name).unwrap(),
                String::from_utf8(operation).unwrap(),
            ));
        });
        if rooted == b"::errorCode" {
            C84_CODE_HEADERS.with(|headers| headers.borrow_mut().push(value as usize));
        }
        super::super::Code::Ok
    }

    fn c84_code_interpreter() -> Interp {
        let mut interp = Interp::with_native_core(
            default_host(),
            profile_for_dialect("tcl8.4"),
            NativeBootstrapInputs::default(),
        )
        .unwrap();
        interp.register_builtin(b"observe", observe_c84_code);
        assert_eq!(
            interp.eval_str(
                b"trace variable ::errorInfo rw observe;trace variable ::errorCode rw observe"
            ),
            super::super::Code::Ok
        );
        C84_CODE_EVENTS.with(|events| events.borrow_mut().clear());
        C84_CODE_HEADERS.with(|headers| headers.borrow_mut().clear());
        interp
    }

    #[test]
    fn c84_global_error_code_publication_matches_five_original_callback_controls() {
        let controls = include_str!("../../tests/data/native_c84_error_code/controls.tsv");
        let sources: &[(&str, &[u8])] = &[
            (
                "conflict",
                b"package provide probe 1;catch {package provide probe 2} m;list $m $::errorCode",
            ),
            ("no-code", b"catch {error FAIL} m;list $m $::errorCode"),
            ("NONE", b"catch {error FAIL {} NONE} m;list $m $::errorCode"),
            (
                "structured",
                b"catch {error FAIL {} {CUSTOM DETAIL}} m;list $m $::errorCode",
            ),
            (
                "structured-getter",
                b"catch {lindex {} BAD} m;list $m $::errorCode",
            ),
        ];
        for row in controls.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let source = sources
                .iter()
                .find(|(name, _)| *name == fields[0])
                .unwrap()
                .1;
            let mut interp = c84_code_interpreter();
            assert_eq!(
                interp.eval_str(source).as_int().to_string(),
                fields[1],
                "{row}"
            );
            let result: String = interp
                .result_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            assert_eq!(result, fields[2], "{row}");
            C84_CODE_EVENTS
                .with(|events| assert_eq!(events.borrow().join(";"), fields[3], "{row}"));
            assert!(!interp.host_refusal_pending(), "{row}");
        }
    }

    #[test]
    fn c84_nonempty_error_info_publication_matches_original_callback_controls() {
        let controls = include_str!("../../tests/data/native_c84_error_code/nonempty/controls.tsv");
        let sources: &[(&str, &[u8])] = &[
            (
                "info-no-code",
                b"catch {error FAIL INFO} m;list $m $::errorInfo $::errorCode",
            ),
            (
                "info-NONE",
                b"catch {error FAIL INFO NONE} m;list $m $::errorInfo $::errorCode",
            ),
            (
                "info-structured",
                b"catch {error FAIL INFO {CUSTOM DETAIL}} m;list $m $::errorInfo $::errorCode",
            ),
        ];
        for row in controls.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let source = sources
                .iter()
                .find(|(name, _)| *name == fields[0])
                .unwrap()
                .1;
            let mut interp = c84_code_interpreter();
            assert_eq!(
                interp.eval_str(source).as_int().to_string(),
                fields[1],
                "{row}"
            );
            let result: String = interp
                .result_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            assert_eq!(result, fields[2], "{row}");
            C84_CODE_EVENTS
                .with(|events| assert_eq!(events.borrow().join(";"), fields[3], "{row}"));
            assert!(!interp.host_refusal_pending(), "{row}");
        }
    }

    #[test]
    fn c84_explicit_error_code_setter_retains_the_same_original_header() {
        let mut interp = c84_code_interpreter();
        let head = obj::Owned::fresh(new_string(b"error"));
        let message = obj::Owned::fresh(new_string(b"FAIL"));
        let info = obj::Owned::fresh(new_string(b""));
        let code = obj::Owned::fresh(new_string(b"CUSTOM DETAIL"));
        assert_eq!(
            interp.eval_original_object_vector(&[
                head.as_ptr(),
                message.as_ptr(),
                info.as_ptr(),
                code.as_ptr(),
            ]),
            super::super::Code::Error
        );
        C84_CODE_HEADERS.with(|headers| {
            assert_eq!(
                headers.borrow().first().copied(),
                Some(code.as_ptr() as usize)
            )
        });
        assert_eq!(interp.var_get(b"::errorCode"), Some(code.as_ptr()));
        assert!(!interp.host_refusal_pending());
    }

    #[test]
    fn c85_error_options_rearm_publication_of_the_restored_original_header() {
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                default_host(),
                profile_for_dialect("tcl8.5"),
                NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"observe_log", observe_c84_log);
            assert_eq!(
                interp.eval_str(b"trace variable ::errorInfo w observe_log"),
                super::super::Code::Ok
            );
            C84_LOG_WRITES.with(|writes| writes.borrow_mut().clear());
            LOG_HEADERS.with(|headers| headers.borrow_mut().clear());
            interp.set_result_bytes(b"SEED");
            interp.log_command_bytes(1, b"failing command");
            let original = interp.exc.borrow().native.info.as_ref().unwrap().as_ptr();
            assert!(!interp.exc.borrow().native.legacy_copy);
            let options = obj::Owned::fresh(
                crate::cmd_error::completion_options(&mut interp, super::super::Code::Error)
                    .unwrap(),
            );
            assert!(interp.exc.borrow().native.legacy_copy);
            assert_eq!(
                interp.exc.borrow().native.info.as_ref().unwrap().as_ptr(),
                original
            );
            interp.publish_and_reset_error();
            LOG_HEADERS
                .with(|headers| assert_eq!(headers.borrow().as_slice(), [original as usize; 2]));
            C84_LOG_WRITES.with(|writes| {
                assert_eq!(
                    writes.borrow().as_slice(),
                    [
                        b"SEED\n    while executing\n\"failing command\"".to_vec(),
                        b"SEED\n    while executing\n\"failing command\"".to_vec(),
                    ]
                )
            });
            assert!(interp.exc.borrow().native.info.is_none());
            drop(options);
        }
        assert_eq!(counters::finalize(), 0);
    }

    #[test]
    fn c84_command_logging_seeds_then_appends_the_original_global_receiver() {
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                default_host(),
                profile_for_dialect("tcl8.4"),
                NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"observe_c84_log", observe_c84_log);
            assert_eq!(
                interp.eval_str(b"trace variable ::errorInfo w observe_c84_log"),
                super::super::Code::Ok
            );
            C84_LOG_WRITES.with(|writes| writes.borrow_mut().clear());
            let original = obj::Owned::fresh(new_string(b"SEED"));
            interp.set_result(original.as_ptr());
            interp.log_command_bytes(1, b"failing command");
            assert!(!interp.host_refusal_pending());
            C84_LOG_WRITES.with(|writes| {
                assert_eq!(
                    writes.borrow().as_slice(),
                    [
                        b"SEED".to_vec(),
                        b"SEED\n    while executing\n\"failing command\"".to_vec()
                    ]
                );
            });
            assert_eq!(interp.result.get(), original.as_ptr());
            assert_eq!(obj_bytes(original.as_ptr()), b"SEED");
            assert!(interp.exc.borrow().native.info.is_none());
            // Direct Tcl_LogCommandInfo clears ERR_ALREADY_LOGGED in C8.4;
            // the evaluating command, rather than this logger, owns the bit.
            assert!(!interp.exc.borrow().already_logged);
            interp.publish_and_reset_error();
            C84_LOG_WRITES.with(|writes| assert_eq!(writes.borrow().len(), 2));
        }
        assert_eq!(counters::finalize(), 0);
    }

    #[test]
    fn command_logging_publishes_same_private_header_only_to_user_traced_cells() {
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            counters::reset();
            {
                let mut interp = Interp::with_native_core(
                    default_host(),
                    profile_for_dialect(profile),
                    NativeBootstrapInputs::default(),
                )
                .unwrap();
                let public = obj::Owned::fresh(new_string(b"previous public info"));
                interp.var_set(b"::errorInfo", public.as_ptr()).unwrap();
                interp.set_result_bytes(b"first error");
                interp.log_command_bytes(1, b"first command");
                assert_eq!(interp.var_get_at(b"::errorInfo", 0), Some(public.as_ptr()));

                *interp.exc.borrow_mut() = ExceptionState::default();
                assert_eq!(
                    interp.eval_str(b"trace add variable ::errorInfo write list"),
                    super::super::Code::Ok,
                    "{profile}"
                );
                interp.set_result_bytes(b"second error");
                interp.log_command_bytes(1, b"second command");
                assert!(!interp.host_refusal_pending(), "{profile}");
                let private = interp
                    .exc
                    .borrow()
                    .native
                    .info
                    .as_ref()
                    .expect("restored private error info")
                    .as_ptr();
                assert_eq!(interp.var_get_at(b"::errorInfo", 0), Some(private));
                assert_eq!(
                    obj_bytes(private),
                    b"second error\n    while executing\n\"second command\""
                );
                assert!(interp.exc.borrow().already_logged, "{profile}");
            }
            assert_eq!(counters::finalize(), 0, "{profile}");
        }
    }

    #[test]
    fn outermost_native_error_retains_private_objects_until_the_next_evaluation() {
        use super::super::Code;
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            counters::reset();
            {
                let mut interp = Interp::with_native_core(
                    default_host(),
                    profile_for_dialect(profile),
                    NativeBootstrapInputs::default(),
                )
                .unwrap();
                assert_eq!(
                    interp.eval_str(b"::oo::define::method outside {} {}"),
                    Code::Error,
                    "{profile}"
                );
                assert_eq!(interp.error_code(), b"TCL OO MONKEY_BUSINESS", "{profile}");
                let original = interp.native_private_error_object(false).unwrap();
                assert_eq!(
                    interp.native_private_error_object(false).unwrap().as_ptr(),
                    original.as_ptr()
                );
                // Tcl_GetReturnOptions(TCL_ERROR) observes the private error,
                // while a subsequent public Eval resets it before dispatch.
                assert_eq!(interp.eval_str(b"namespace current"), Code::Ok);
                assert_eq!(interp.error_code(), b"NONE", "{profile}");
                assert!(interp.native_private_error_object(false).is_none());
                assert_eq!(
                    interp.eval_str(b"catch {::oo::define::method outside {} {}}"),
                    Code::Ok
                );
                assert_eq!(interp.error_code(), b"NONE", "{profile}");
                assert!(interp.native_private_error_object(false).is_none());
            }
            assert_eq!(counters::finalize(), 0, "{profile}");
        }
    }

    // Native proof: naming.variable.error-globals-bootstrap-and-legacy-copy
    // docs/design/analysis/name-resolution-proofs/variable.error-globals-bootstrap-and-legacy-copy.md
    #[test]
    fn hidden_error_reads_and_unsets_match_original_native_objects() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            counters::reset();
            {
                let mut interp = Interp::with_native_core(
                    default_host(),
                    profile_for_dialect(profile),
                    NativeBootstrapInputs::default(),
                )
                .unwrap();
                let enabled = interp
                    .native_invocation_dialect()
                    .native_error_variable_protocol()
                    .is_some();
                let original = obj::Owned::fresh(new_string(b"PRIVATE INFO"));
                let code = obj::Owned::fresh(new_string(b"PRIVATE CODE"));
                let public_info = obj::Owned::fresh(new_string(b"GUEST INFO"));
                interp
                    .var_set(b"::errorInfo", public_info.as_ptr())
                    .unwrap();
                drop(public_info);
                let public_code = obj::Owned::fresh(new_string(b"GUEST CODE"));
                interp
                    .var_set(b"::errorCode", public_code.as_ptr())
                    .unwrap();
                drop(public_code);
                interp.exc.borrow_mut().native = NativeErrorObjects {
                    info: Some(original.clone()),
                    info_len: b"PRIVATE INFO".len(),
                    code: Some(code.clone()),
                    legacy_copy: false,
                    global_code_set: false,
                    ..NativeErrorObjects::default()
                };
                assert_eq!(
                    obj_bytes(interp.read_named_variable(b"::errorInfo").unwrap()),
                    b"GUEST INFO"
                );
                interp.exc.borrow_mut().native.legacy_copy = true;
                let read = interp.read_named_variable(b"::errorInfo").unwrap();
                if enabled {
                    assert_eq!(read, original.as_ptr());
                } else {
                    assert_eq!(obj_bytes(read), b"GUEST INFO");
                }
                assert!(interp.var_unset(b"::errorInfo"));
                if enabled {
                    let home = interp.trace_identity(b"::errorInfo");
                    assert!(interp.native_error_variable_at(&home).is_some());
                    assert_eq!(
                        interp.read_named_variable(b"::errorInfo").ok(),
                        Some(original.as_ptr())
                    );
                    assert!(interp.var_unset(b"::errorCode"));
                    assert!(!interp.var_unset(b"::errorCode"));
                    interp.exc.borrow_mut().native.code = None;
                    assert_eq!(
                        obj_bytes(interp.read_named_variable(b"::errorCode").unwrap()),
                        b""
                    );
                    assert!(interp.exc.borrow().native.code.is_none());
                    interp.exc.borrow_mut().native.code = Some(code.clone());
                    interp.publish_native_error_objects();
                    assert_eq!(interp.var_get_at(b"::errorCode", 0), Some(code.as_ptr()));
                } else {
                    assert!(interp.var_get(b"::errorInfo").is_none());
                }
                *interp.exc.borrow_mut() = ExceptionState::default();
            }
            assert_eq!(counters::finalize(), 0, "{profile}");
        }
    }
    thread_local! {
        static RESET_CALLBACKS: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    fn observe_reset(interp: &mut Interp, argv: &[*mut obj::TclObj]) -> super::super::Code {
        assert!(interp.exc.borrow().native.info.is_none());
        assert!(interp.exc.borrow().native.code.is_none());
        RESET_CALLBACKS.with(|seen| seen.borrow_mut().push(obj_bytes(argv[1])));
        super::super::Code::Ok
    }

    fn producer_reference_counts(profile: &str) -> (usize, usize) {
        let fixture = match profile {
            "tcl8.5" => include_str!(
                "../../../../rust/tcl-registry/tests/data/native_error_variables/8.5.19-producers.tsv"
            ),
            "tcl8.6" => include_str!(
                "../../../../rust/tcl-registry/tests/data/native_error_variables/8.6.18-producers.tsv"
            ),
            "tcl9.0" => include_str!(
                "../../../../rust/tcl-registry/tests/data/native_error_variables/9.0.4-producers.tsv"
            ),
            "tcl9.1" => include_str!(
                "../../../../rust/tcl-registry/tests/data/native_error_variables/9.1.0-producers.tsv"
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

    // Native proof: naming.variable.error-producer-reset-callback-order
    // docs/design/analysis/name-resolution-proofs/variable.error-producer-reset-callback-order.md
    // Native proof: naming.variable.error-producer-script-trace-reset-order
    // docs/design/analysis/name-resolution-proofs/variable.error-producer-script-trace-reset-order.md
    #[test]
    fn original_private_error_primaries_and_script_reset_match_native_producers() {
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let (before_reset, after_reset) = producer_reference_counts(profile);
            counters::reset();
            {
                let mut interp = Interp::with_native_core(
                    default_host(),
                    profile_for_dialect(profile),
                    NativeBootstrapInputs::default(),
                )
                .unwrap();
                interp.replace_native_error_code(b"NONE");
                assert!(std::ptr::eq(
                    obj::obj_type_ptr(interp.exc.borrow().native.code.as_ref().unwrap().as_ptr()),
                    &crate::list::TCL_LIST_TYPE
                ));
                interp.exc.borrow_mut().native = NativeErrorObjects::default();
                interp.register_builtin(b"observe_native_reset", observe_reset);
                assert_eq!(interp.eval_str(b"trace add variable ::errorCode write observe_native_reset; trace add variable ::errorInfo write observe_native_reset"), super::super::Code::Ok);
                RESET_CALLBACKS.with(|seen| seen.borrow_mut().clear());
                let recipe = interp
                    .native_invocation_dialect()
                    .byte_array_string_recipe(None)
                    .unwrap();
                let info =
                    obj::Owned::fresh(crate::bytearray::new_byte_array(&[0xff, 0, b'a'], recipe));
                let code =
                    obj::Owned::fresh(crate::bytearray::new_byte_array(&[0xff, 0, b'a'], recipe));
                let head = obj::Owned::fresh(new_string(b"error"));
                let message = obj::Owned::fresh(new_string(b"message"));
                let super::super::Command::Builtin(handler) =
                    interp.resolve_dispatchable(GLOBAL, b"error").unwrap()
                else {
                    panic!("native error handler");
                };
                assert_eq!(
                    handler(
                        &mut interp,
                        &[
                            head.as_ptr(),
                            message.as_ptr(),
                            info.as_ptr(),
                            code.as_ptr()
                        ]
                    ),
                    super::super::Code::Error
                );
                assert_eq!(
                    interp.exc.borrow().native.info.as_ref().unwrap().as_ptr(),
                    info.as_ptr()
                );
                assert_eq!(
                    interp.exc.borrow().native.code.as_ref().unwrap().as_ptr(),
                    code.as_ptr()
                );
                assert!(std::ptr::eq(
                    obj::obj_type_ptr(info.as_ptr()),
                    &crate::bytearray::TCL_BYTE_ARRAY_TYPE
                ));
                assert!(std::ptr::eq(
                    obj::obj_type_ptr(code.as_ptr()),
                    &crate::list::TCL_LIST_TYPE
                ));
                assert_eq!(
                    unsafe { (*info.as_ptr()).ref_count },
                    before_reset as obj::TclSize,
                    "{profile}"
                );
                assert_eq!(
                    unsafe { (*code.as_ptr()).ref_count },
                    before_reset as obj::TclSize,
                    "{profile}"
                );
                interp.publish_native_error_objects();
                RESET_CALLBACKS.with(|seen| {
                    assert_eq!(
                        *seen.borrow(),
                        [b"::errorInfo".to_vec(), b"::errorCode".to_vec()],
                        "{profile}"
                    )
                });
                assert!(interp.exc.borrow().native.info.is_none());
                assert!(interp.exc.borrow().native.code.is_none());
                assert_eq!(interp.var_get_at(b"::errorInfo", 0), Some(info.as_ptr()));
                assert_eq!(interp.var_get_at(b"::errorCode", 0), Some(code.as_ptr()));
                assert_eq!(
                    unsafe { (*info.as_ptr()).ref_count },
                    after_reset as obj::TclSize,
                    "{profile}"
                );
                assert_eq!(
                    unsafe { (*code.as_ptr()).ref_count },
                    after_reset as obj::TclSize,
                    "{profile}"
                );
            }
            assert_eq!(counters::finalize(), 0, "{profile}");
        }
    }

    #[test]
    fn hidden_trace_survives_script_removal_array_unset_and_root_teardown() {
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            counters::reset();
            {
                let mut interp = Interp::with_native_core(
                    default_host(),
                    profile_for_dialect(profile),
                    NativeBootstrapInputs::default(),
                )
                .unwrap();
                assert_eq!(interp.eval_str(b"proc hook {args} {}; trace add variable ::errorInfo read hook; trace remove variable ::errorInfo read hook"), super::super::Code::Ok);
                assert!(interp
                    .native_error_variable_at(&interp.trace_identity(b"::errorInfo"))
                    .is_some());
                assert_eq!(
                    interp.eval_str(b"array set ::errorInfo {k value}; unset ::errorInfo"),
                    super::super::Code::Ok
                );
                assert!(interp
                    .native_error_variable_at(&interp.trace_identity(b"::errorInfo"))
                    .is_some());
                assert!(!interp.var_unset(b"::errorInfo"));
                interp.delete_namespace_by_id(GLOBAL);
                assert!(interp
                    .native_error_variable_at(&interp.trace_identity(b"::errorInfo"))
                    .is_some());
                assert!(interp
                    .native_error_variable_at(&interp.trace_identity(b"::errorCode"))
                    .is_some());
            }
            assert_eq!(counters::finalize(), 0, "{profile}");
        }
    }

    // Native proof: naming.variable.error-info-append-original-object
    // docs/design/analysis/name-resolution-proofs/variable.error-info-append-original-object.md
    #[test]
    fn error_info_byte_append_preserves_seed_primary_until_nonempty_mutation() {
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                default_host(),
                profile_for_dialect("tcl9.0"),
                NativeBootstrapInputs::default(),
            )
            .unwrap();
            let recipe = interp
                .native_invocation_dialect()
                .byte_array_string_recipe(None)
                .unwrap();
            let original = obj::Owned::fresh(crate::bytearray::new_byte_array(b"seed", recipe));
            unsafe {
                interp.set_obj_result(original.as_ptr());
            }
            interp.exc.borrow_mut().info = Some(b"seed".to_vec());
            interp.exc.borrow_mut().native.info = Some(original.clone());
            interp.exc.borrow_mut().native.info_len = 4;
            interp.update_native_error_info();
            assert_eq!(
                interp.exc.borrow().native.info.as_ref().unwrap().as_ptr(),
                original.as_ptr()
            );
            assert!(std::ptr::eq(
                obj::obj_type_ptr(original.as_ptr()),
                &crate::bytearray::TCL_BYTE_ARRAY_TYPE
            ));
            interp.append_error_info_context(b"context");
            let pointer = interp.exc.borrow().native.info.as_ref().unwrap().as_ptr();
            assert_ne!(pointer, original.as_ptr());
            assert!(matches!(
                obj::native_object_snapshot(pointer).unwrap().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
            ));
            assert_eq!(obj_bytes(pointer), b"seed\n    context");
            assert!(std::ptr::eq(
                obj::obj_type_ptr(original.as_ptr()),
                &crate::bytearray::TCL_BYTE_ARRAY_TYPE
            ));
        }
        assert_eq!(counters::finalize(), 0);
    }
}
