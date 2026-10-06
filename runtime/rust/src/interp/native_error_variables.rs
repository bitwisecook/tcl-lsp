//! C hidden error-variable traces retain private objects independently of globals.

use super::{new_string, Interp, TraceAccess, GLOBAL};
use crate::obj;
use tcl_registry::special_vars::{NativeErrorStorageVariable as Variable, NativeErrorVariableRead};

#[derive(Default, Clone)]
pub(super) struct NativeErrorObjects {
    pub(super) info: Option<obj::Owned>,
    pub(super) info_len: usize,
    pub(super) code: Option<obj::Owned>,
    pub(super) legacy_copy: bool,
}

pub(super) struct NativeErrorTraceState {
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
        let dialect = self.native_invocation_dialect();
        dialect.tcl_version == Some(tcl_dialect::TclVersion::V8_4)
            && dialect.native_error_log_protocol().is_some()
    }

    pub(super) fn save_native_error_trace_state(&self) -> Option<NativeErrorTraceState> {
        self.native_invocation_dialect()
            .native_error_variable_protocol()?;
        let exc = self.exc.borrow();
        Some(NativeErrorTraceState {
            originals: exc.native.clone(),
            info: exc.info.clone(),
            code: exc.code.clone(),
            explicit: exc.code_explicit,
            logged: exc.already_logged,
            primitive_getter: exc.primitive_getter.clone(),
            expression_error_stage: exc.expression_error_stage.clone(),
        })
    }

    pub(super) fn restore_native_error_trace_state(&self, saved: NativeErrorTraceState) {
        let mut exc = self.exc.borrow_mut();
        // Tcl_RestoreInterpState restores ERR_ALREADY_LOGGED, not ERR_LEGACY_COPY.
        let copy = exc.native.legacy_copy;
        exc.native = saved.originals;
        exc.native.legacy_copy = copy;
        exc.info = saved.info;
        exc.code = saved.code;
        exc.code_explicit = saved.explicit;
        exc.already_logged = saved.logged;
        exc.primitive_getter = saved.primitive_getter;
        exc.expression_error_stage = saved.expression_error_stage;
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
        let name = variable.global_name().as_bytes();
        self.ensure_trace_variable(name)
            .expect("native hidden error shell");
        let home = self.trace_identity(name);
        if let Some(id) = home.binding_id {
            let mut cells = self.native_error_cells.borrow_mut();
            cells.retain(|(kind, _)| *kind != variable);
            cells.push((variable, id));
        }
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
        let defined = crate::vars::get(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            GLOBAL,
            name,
        )
        .is_some();
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
            let _ = crate::vars::set(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                GLOBAL,
                name,
                value.as_ptr(),
            );
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

    pub(crate) fn retain_native_error_option(&self, info: bool, original: *mut obj::TclObj) {
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_none()
        {
            return;
        }
        let mut exc = self.exc.borrow_mut();
        if info {
            exc.native.info = Some(obj::Owned::retain(original));
            exc.native.info_len = obj::bytes_of(original).len();
        } else {
            exc.native.code = Some(obj::Owned::retain(original));
        }
    }

    pub(super) fn replace_native_error_code(&self, bytes: &[u8]) {
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
        {
            let original = self.native_default_error_code(bytes);
            self.exc.borrow_mut().native.code = Some(original);
        }
    }

    pub(super) fn capture_native_error_objects(&self) {
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_none()
        {
            return;
        }
        let mut exc = self.exc.borrow_mut();
        exc.native.code = Some(self.native_default_error_code(
            if exc.code.is_empty() && !exc.code_explicit {
                b"NONE"
            } else {
                &exc.code
            },
        ));
        exc.native.info_len = exc.info.as_ref().map_or(0, Vec::len);
        exc.native.info = exc
            .info
            .as_deref()
            .map(|bytes| obj::Owned::fresh(new_string(bytes)));
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
        let bytes = self
            .exc
            .borrow()
            .info
            .clone()
            .unwrap_or_else(|| self.result_bytes());
        if self.exc.borrow().native.info.is_none() {
            let mut exc = self.exc.borrow_mut();
            exc.native.info = Some(obj::Owned::retain(self.result.get()));
            exc.native.info_len = self.result_bytes().len();
        }
        let outcome = {
            let exc = self.exc.borrow();
            let suffix = bytes.get(exc.native.info_len..).ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native errorInfo append extent",
                ),
            );
            suffix.and_then(|suffix| {
                if suffix.is_empty() {
                    return Ok(None);
                }
                let protocol = dialect.native_object_append_protocol(None).ok_or(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native errorInfo append",
                    ),
                )?;
                let objects = crate::value_ops::RuntimeAppendObjects {
                    dialect,
                    binary_recipe: dialect.byte_array_string_recipe(None),
                };
                let receiver = crate::value_ops::RuntimeAppendValue::borrowed(
                    exc.native
                        .info
                        .as_ref()
                        .expect("private errorInfo owner")
                        .as_ptr(),
                );
                tcl_cmd_core::native_append::append_counted_bytes(
                    &objects,
                    protocol.recipe(),
                    &receiver,
                    suffix,
                )
                .map(|working| Some(obj::Owned::retain(working.as_ptr())))
            })
        };
        match outcome {
            Ok(original) => {
                let mut exc = self.exc.borrow_mut();
                if let Some(original) = original {
                    exc.native.info = Some(original);
                }
                exc.native.info_len = bytes.len();
                if exc.native.code.is_none() {
                    exc.native.code = Some(self.native_default_error_code(b"NONE"));
                }
                exc.native.legacy_copy = true;
            }
            Err(error) => {
                self.refuse_native_access(
                    error
                        .native_access_refusal()
                        .expect("native append capability failure"),
                );
            }
        }
    }

    fn append_c84_global_error_info(&mut self) {
        let Some(bytes) = self.exc.borrow().info.clone() else {
            return;
        };
        if !self.exc.borrow().native.legacy_copy {
            let original = self.result.get();
            let length = self.result_bytes().len();
            // C8.4's first Tcl_AddObjErrorInfo setter lends the current result
            // to the global cell, with no separately retained private header.
            let _ = self.var_set(b"::errorInfo", original);
            if self.exc.borrow().code.is_empty() {
                let code = obj::Owned::fresh(new_string(b"NONE"));
                let _ = self.var_set(b"::errorCode", code.as_ptr());
            }
            let mut exc = self.exc.borrow_mut();
            exc.native.info_len = length;
            exc.native.legacy_copy = true;
        }
        let length = self.exc.borrow().native.info_len;
        let Some(suffix) = bytes.get(length..) else {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native errorInfo append extent",
                ),
            );
            return;
        };
        let logged = self.exc.borrow().already_logged;
        if !suffix.is_empty() {
            // TCL_APPEND_VALUE consumes counted String bytes, rather than the
            // generic object-append binary/list conversion shortcuts.
            let _ = self.append_native_counted_variable_bytes(b"::errorInfo", suffix);
        }
        {
            let mut exc = self.exc.borrow_mut();
            exc.native.info_len = bytes.len();
            exc.native.legacy_copy = true;
            exc.already_logged = logged;
        }
        if let Some(original) = self.var_get_at(b"::errorInfo", 0) {
            use tcl_syntax::value::ValueOps;
            match self.native_string_bytes(&original) {
                Ok(bytes) => {
                    let mut exc = self.exc.borrow_mut();
                    exc.native.info_len = bytes.len();
                    exc.info = Some(bytes.to_vec());
                }
                Err(error) => {
                    self.report_cmd_error(error.into());
                }
            }
        }
    }

    /// Publish the SAME private header at `Tcl_LogCommandInfo`'s reached
    /// compatibility setter without creating an additional object owner.
    pub(super) fn publish_traced_native_error_info(&mut self) {
        let original = self
            .exc
            .borrow()
            .native
            .info
            .as_ref()
            .map(obj::Owned::as_ptr);
        if let Some(original) = original {
            // The private interpreter field lends this pointer. The variable
            // setter and its trace-state snapshot retain their actual roles.
            let _ = self.var_set(b"::errorInfo", original);
        } else if !self.host_refusal_pending() {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "private errorInfo logging publication",
                ),
            );
        }
    }

    pub(crate) fn native_private_error_object(&self, info: bool) -> Option<obj::Owned> {
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
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_error_variable_protocol()
        else {
            return;
        };
        for variable in protocol.reset_order() {
            let pointer = {
                let exc = self.exc.borrow();
                if !exc.native.legacy_copy {
                    None
                } else {
                    match variable {
                        Variable::ErrorInfo => &exc.native.info,
                        Variable::ErrorCode => &exc.native.code,
                    }
                    .as_ref()
                    .map(obj::Owned::as_ptr)
                }
            };
            if let Some(pointer) = pointer {
                // The private field lends its object to ObjSetVar2. The variable
                // and TclCallVarTraces saved state own their actual references.
                let _ = self.var_set(variable.global_name().as_bytes(), pointer);
            }
            // Tcl_ResetResult releases each current private field immediately
            // after its setter, so info callbacks already see code == NULL.
            match variable {
                Variable::ErrorCode => self.exc.borrow_mut().native.code = None,
                Variable::ErrorInfo => self.exc.borrow_mut().native.info = None,
            }
        }
        self.exc.borrow_mut().native.legacy_copy = false;
        self.clear_return_options();
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
    }

    fn observe_c84_log(interp: &mut Interp, _arguments: &[*mut obj::TclObj]) -> super::super::Code {
        let original = interp.var_get_at(b"::errorInfo", 0).unwrap();
        C84_LOG_WRITES.with(|writes| writes.borrow_mut().push(obj_bytes(original)));
        super::super::Code::Ok
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
            assert!(interp.exc.borrow().already_logged);
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
                interp.set_global_raw(b"::errorInfo", b"GUEST INFO");
                interp.set_global_raw(b"::errorCode", b"GUEST CODE");
                interp.exc.borrow_mut().native = NativeErrorObjects {
                    info: Some(original.clone()),
                    info_len: b"PRIVATE INFO".len(),
                    code: Some(code.clone()),
                    legacy_copy: false,
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
