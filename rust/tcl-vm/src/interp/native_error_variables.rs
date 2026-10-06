//! C error globals observe separately retained private interpreter objects.

use super::{Local, Value, VarId, Vm};
use tcl_registry::special_vars::{NativeErrorStorageVariable as Variable, NativeErrorVariableRead};
use tcl_syntax::value::ValueOps;

pub(super) struct NativeErrorTraceState {
    info: Option<Value>,
    info_len: usize,
    result: Option<crate::value::WeakNativeObject>,
    code: Option<Value>,
    options: Option<Value>,
    bytes: Option<Vec<u8>>,
    logged: bool,
}

impl Vm {
    pub(super) fn uses_c84_global_error_info(&self) -> bool {
        let dialect = self.actual_native_invocation_dialect();
        dialect.tcl_version == Some(tcl_dialect::TclVersion::V8_4)
            && dialect.native_error_log_protocol().is_some()
    }

    /// Publish a reached C9.1 compiler error through its actual ResetResult
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
            .is_some_and(|recipe| recipe.syntax_options_share_message())
        {
            return Err(Error::unavailable(
                "original compiler Syntax publication issuer",
            ));
        }
        let bytes = message
            .resident_string_bytes()
            .ok_or_else(|| Error::unavailable("original compiler Syntax message resident bytes"))?;
        let code = options
            .with_cached_dictionary_member(b"-errorcode", |member| member.cloned())
            .flatten()
            .ok_or_else(|| Error::unavailable("original compiler Syntax error code"))?;
        self.adopt_native_interp_result(message.clone())
            .map_err(|_| Error::unavailable("original compiler Syntax result owner"))?;
        self.native_error_info = Some(message.clone());
        self.native_error_info_len = bytes.len();
        self.primitive_error_code = Some(code);
        self.native_error_legacy_copy = true;
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
            info: self.native_error_info.clone(),
            info_len: self.native_error_info_len,
            result: self.native_error_result.clone(),
            code: self.primitive_error_code.clone(),
            options: self.native_return_options.clone(),
            bytes: self.error_info.clone(),
            logged: self.error_logged,
        })
    }
    pub(super) fn restore_native_error_trace_state(&mut self, saved: NativeErrorTraceState) {
        self.native_error_info = saved.info;
        self.native_error_info_len = saved.info_len;
        self.native_error_result = saved.result;
        self.primitive_error_code = saved.code;
        self.native_return_options = saved.options;
        self.error_info = saved.bytes;
        self.error_logged = saved.logged;
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
            Variable::ErrorInfo => &self.native_error_info,
            Variable::ErrorCode => &self.primitive_error_code,
        };
        let value = match protocol.read(self.native_error_legacy_copy, private.is_some(), defined) {
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
            && self.native_error_info.is_none()
        {
            self.native_error_result = Some(original.downgrade_native_object());
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
            || self.native_error_info.is_some()
        {
            return;
        }
        let original = match self.native_error_result.as_ref() {
            Some(receipt) => match receipt.upgrade() {
                Some(original) => original,
                None => {
                    let _ = self.refuse_host_command("native error result lifetime expired".into());
                    return;
                }
            },
            None => Value::new_native_string_bytes(bytes),
        };
        self.native_error_info = Some(original);
        self.native_error_info_len = bytes.len();
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
        let Some(bytes) = self.error_info.as_ref() else {
            return;
        };
        let bytes = bytes.clone();
        let outcome = bytes
            .get(self.native_error_info_len..)
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
                let receiver = self.native_error_info.as_ref().ok_or(
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
                    self.native_error_info = Some(original);
                }
                self.native_error_info_len = bytes.len();
                if self.primitive_error_code.is_none() {
                    self.primitive_error_code = Some(self.native_default_error_code(b"NONE"));
                }
                self.native_error_legacy_copy = true;
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
        if traced && let Some(original) = self.native_error_info.clone() {
            // The setter owns the SAME object. Its ordinary trace walk keeps
            // the interpreter's private error state live across callbacks.
            let _ = self.set_var_bytes(b"::errorInfo", original);
        }
    }

    fn seed_c84_global_error_info(&mut self, bytes: &[u8]) {
        if self.native_error_legacy_copy {
            return;
        }
        let original = match self.native_error_result.as_ref() {
            Some(receipt) => match receipt.upgrade() {
                Some(original) => original,
                None => {
                    let _ = self.refuse_host_command("native error result lifetime expired".into());
                    return;
                }
            },
            None => Value::new_native_string_bytes(bytes),
        };
        // Tcl_AddObjErrorInfo owns no private errorInfo header in C8.4;
        // its first real setter stores the current original result globally.
        let _ = self.set_var_bytes(b"::errorInfo", original);
        if self.primitive_error_code.is_none() {
            let _ = self.set_var_bytes(
                b"::errorCode",
                Value::new_native_string_bytes(b"NONE".as_slice()),
            );
        }
        self.native_error_info_len = bytes.len();
        self.native_error_legacy_copy = true;
    }

    fn append_c84_global_error_info(&mut self) {
        let Some(bytes) = self.error_info.as_ref() else {
            return;
        };
        let Some(suffix) = bytes.get(self.native_error_info_len..) else {
            let _ = self.refuse_host_command("native errorInfo append extent".into());
            return;
        };
        let length = bytes.len();
        let logged = self.error_logged;
        if !suffix.is_empty() {
            let source = Value::new_native_string_bytes(suffix);
            // TCL_APPEND_VALUE reaches the original variable receiver and its
            // actual getter/write traces; guest setter failures are ignored.
            let _ = self.append_variable_bytes(b"::errorInfo", None, source);
        }
        self.native_error_info_len = length;
        self.native_error_legacy_copy = true;
        self.error_logged = logged;
        if let Some(original) = self.get_var_from(0, "::errorInfo") {
            match self.native_string_bytes(&original) {
                Ok(bytes) => {
                    self.native_error_info_len = bytes.len();
                    self.error_info = Some(bytes.to_vec());
                }
                Err(error) => {
                    let _ = crate::command::completion_from_tcl_error(self, error.into());
                }
            }
        }
    }

    pub(crate) fn clear_return_error_info(&mut self) {
        self.native_error_info = None;
        self.native_error_info_len = 0;
        self.native_error_result = None;
        self.error_info = None;
    }
    pub(crate) fn native_return_error_info(&self) -> Option<&Value> {
        self.native_error_info.as_ref()
    }
    pub(crate) fn native_return_error_code(&self) -> Option<&Value> {
        self.primitive_error_code.as_ref()
    }

    pub(crate) fn native_merged_return_options(&self) -> Option<&Value> {
        self.native_return_options
            .as_ref()
            .filter(|value| value.cached_dictionary_bucket_count().is_some())
    }
    pub(crate) fn native_return_controls(&self, completion: tcl_core_types::Code) -> (i32, i64) {
        if completion == tcl_core_types::Code::Return {
            (
                self.native_c_return_state.code,
                self.native_c_return_state.level,
            )
        } else {
            (completion.as_int() as i32, 0)
        }
    }

    pub(crate) fn retain_native_return_options(&mut self, original: &Value) {
        if self
            .actual_native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
        {
            self.native_return_options = Some(original.clone());
        }
    }

    pub(crate) fn mark_native_error_copy(&mut self) {
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
        {
            self.native_error_legacy_copy = true;
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
            self.native_error_info = Some(original.clone());
            self.native_error_info_len = bytes.len();
            self.native_error_legacy_copy = true;
        }
    }

    pub(super) fn publish_native_error_objects(&mut self, info: &[u8], code: &Value) {
        if self.native_error_info.is_none() {
            self.seed_native_error_result_object(info);
        }
        if self.primitive_error_code.is_none() {
            self.primitive_error_code = Some(code.clone());
        }
        self.native_error_legacy_copy = true;
        self.reset_native_error_objects();
    }

    pub(super) fn reset_native_error_objects_before_trace_script(&mut self) {
        if self.native_error_legacy_copy
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
                .native_error_legacy_copy
                .then(|| match variable {
                    Variable::ErrorCode => self.primitive_error_code.clone(),
                    Variable::ErrorInfo => self.native_error_info.clone(),
                })
                .flatten();
            if let Some(argument) = argument {
                let _ = self.set_var_bytes(variable.global_name().as_bytes(), argument);
            }
            match variable {
                Variable::ErrorCode => self.primitive_error_code = None,
                Variable::ErrorInfo => self.native_error_info = None,
            }
        }
        self.native_error_legacy_copy = false;
        self.native_return_options = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use tcl_registry::special_vars::NativeBootstrapInputs;
    use tcl_runtime_api::{Code, Completion};

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
            assert!(vm.native_error_info.is_none());
            assert!(vm.primitive_error_code.is_none());
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
                Value::new_native_string_bytes(b"list".as_slice()),
                false,
            );
            vm.log_command_info_only(b"second command", b"second error", 1);
            assert!(vm.execution_refusal.is_none(), "{profile}");
            let private = vm
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
            assert!(vm.error_logged, "{profile} retains the reached log flag");
        }
    }

    thread_local! {
        static RESET_CALLBACKS: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    fn observe_reset(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        assert!(vm.native_error_info.is_none());
        assert!(vm.primitive_error_code.is_none());
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
                Value::string("observe_native_reset"),
                false,
            );
            vm.add_var_trace_bytes(
                b"::errorInfo",
                vec!["write".into()],
                Value::string("observe_native_reset"),
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
                vm.native_error_info
                    .as_ref()
                    .unwrap()
                    .is_same_object(&argv[1])
            );
            assert!(
                vm.primitive_error_code
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
                )
            });
            assert!(vm.native_error_info.is_none());
            assert!(vm.primitive_error_code.is_none());
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
            vm.native_error_info = Some(original.clone());
            vm.native_error_info_len = b"PRIVATE INFO".len();
            vm.native_error_legacy_copy = false;
            assert_eq!(
                vm.read_var_traced_bytes(b"::errorInfo")
                    .unwrap()
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                b"GUEST INFO"
            );
            vm.native_error_legacy_copy = true;
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
                vm.primitive_error_code = None;
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
                    Value::string("list"),
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
            vm.native_error_info
                .as_ref()
                .unwrap()
                .is_same_object(&original)
        );
        vm.error_info = Some(b"seed".to_vec());
        vm.update_native_error_info();
        assert!(
            vm.native_error_info
                .as_ref()
                .unwrap()
                .is_same_object(&original)
        );
        assert!(original.byte_array_representation().is_some());
        vm.seed_error_info_frame(b"seed", b" context");
        let private = vm.native_error_info.as_ref().unwrap();
        assert!(!private.is_same_object(&original));
        assert!(matches!(
            private.native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
        ));
        assert_eq!(private.string_bytes().as_ref(), b"seed context");
        assert!(original.byte_array_representation().is_some());
    }
}
