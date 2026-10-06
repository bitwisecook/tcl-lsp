// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native return preparation over original VM object storage.

use crate::{Value, Vm};
use tcl_cmd_core::{
    CmdError,
    return_options::{self, ReturnOptionsOps, ReturnOptionsProtocol, ReturnOptionsPurpose},
};
use tcl_registry::InvocationDialect;
use tcl_runtime_api::{Code, Completion};
use tcl_syntax::{
    native_string::NativeStringProtocol,
    scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue},
    value::ValueError,
};

pub(crate) struct NativeReturnOps {
    pub(crate) dialect: InvocationDialect,
    pub(crate) string: NativeStringProtocol,
    protocol: ReturnOptionsProtocol,
    jim_context: Option<std::rc::Rc<crate::value::NativeJimObjectContext>>,
}

impl NativeReturnOps {
    pub(crate) fn for_c(version: tcl_dialect::TclVersion) -> Result<Self, CmdError> {
        let dialect = InvocationDialect::for_version(version);
        let protocol =
            dialect
                .return_options_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native C return merger",
                ))?;
        Ok(Self {
            dialect,
            string: NativeStringProtocol::C(version),
            protocol,
            jim_context: None,
        })
    }

    pub(crate) fn selected(vm: &Vm) -> Result<(Self, ReturnOptionsProtocol), CmdError> {
        use tcl_registry::native_return_options::LogicalReturnOptionsProvider;
        let dialect = vm.native_invocation_dialect();
        let actual = dialect.return_options_protocol();
        let protocol = actual
            .or_else(|| {
                dialect.logical_return_options_protocol(
                    LogicalReturnOptionsProvider::Tcl84CoreSimulation,
                )
            })
            .ok_or(ValueError::CommandProtocolUnavailable("return options"))?;
        let string = dialect
            .native_string_protocol()
            .or_else(|| {
                dialect
                    .logical_return_options_protocol(
                        LogicalReturnOptionsProvider::Tcl84CoreSimulation,
                    )
                    .map(|_| NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4))
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "return option strings",
            ))?;
        Ok((
            Self {
                dialect,
                string,
                protocol,
                jim_context: if string.is_jim084() {
                    Some(vm.native_jim_object_context()?)
                } else {
                    None
                },
            },
            protocol,
        ))
    }
}

impl ReturnOptionsOps for NativeReturnOps {
    type Value = Value;
    fn bytes(&mut self, value: &Value) -> Result<Vec<u8>, CmdError> {
        value
            .native_string_bytes(self.string)
            .map(|bytes| bytes.to_vec())
            .map_err(|error| {
                CmdError::from(ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                ))
            })
    }
    fn list(&mut self, value: &Value) -> Result<Vec<Value>, CmdError> {
        if let Some(context) = &self.jim_context {
            value.bind_native_jim_context(context)?;
        }
        value
            .native_object_list_elements(self.string)
            .map(|items| items.as_ref().clone())
            .map_err(CmdError::from)
    }
    fn new_string(&mut self, bytes: &[u8]) -> Self::Value {
        Value::new_native_string_bytes(bytes.to_vec())
    }
    fn integer_probe(&mut self, value: &Value, wide: bool) -> Result<Option<i64>, CmdError> {
        if self.dialect.native_scalar_getter_protocol().is_none() {
            use tcl_syntax::logical_numeric_simulation::{
                AuthoredLogicalNumericSimulation, LogicalNumericInputStage,
            };
            let provider = self
                .dialect
                .authored_logical_numeric_simulation(AuthoredLogicalNumericSimulation::Tcl84Core)
                .ok_or(ValueError::ScalarNumericInputUnavailable)?;
            let bytes = self.bytes(value)?;
            return Ok(
                match provider.parse_number(&bytes, LogicalNumericInputStage::Integer) {
                    Ok(tcl_syntax::number::Number::Int(value))
                        if wide || i32::try_from(value).is_ok() =>
                    {
                        Some(value)
                    }
                    _ => None,
                },
            );
        }
        match value.native_scalar_probe(
            self.dialect,
            if wide {
                NativeScalarGetterKind::Wide
            } else {
                NativeScalarGetterKind::Int
            },
        ) {
            Ok(Ok(NativeScalarGetterValue::Wide(integer))) => Ok(Some(integer)),
            Ok(Ok(_)) => Err(ValueError::ScalarNumericInputUnavailable.into()),
            Ok(Err(_)) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
    fn completion_code_cache(&self, value: &Value) -> Option<i32> {
        if let Some(protocol) = self.dialect.native_index_lookup_protocol() {
            let table = tcl_registry::native_return_options::completion_code_table();
            if let Ok(Some(index)) = protocol.cached_index(value.native_index_cache(), &table) {
                return i32::try_from(index).ok();
            }
        }
        match (value.completion_code_cache()?, self.string) {
            (return_options::CompletionCodeCache::Jim(code), NativeStringProtocol::Jim084)
            | (return_options::CompletionCodeCache::TclKeyword(code), NativeStringProtocol::C(_)) => {
                Some(code)
            }
            _ => None,
        }
    }
    fn adopt_completion_code_cache(
        &mut self,
        value: &Value,
        cache: return_options::CompletionCodeCache,
    ) -> Result<(), CmdError> {
        if let return_options::CompletionCodeCache::TclKeyword(code) = cache {
            let protocol = self.dialect.native_index_lookup_protocol().ok_or(
                ValueError::CommandProtocolUnavailable("native completion Index origin"),
            )?;
            let table = tcl_registry::native_return_options::completion_code_table();
            let index = usize::try_from(code)
                .ok()
                .filter(|index| *index < table.entry_count())
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native completion Index entry",
                ))?;
            value.install_native_index_cache(table.cache(index), protocol)?;
            return Ok(());
        }

        value
            .adopt_completion_code_cache(cache)
            .map_err(CmdError::from)
    }
}

pub(crate) fn apply(
    vm: &mut Vm,
    args: &[Value],
    purpose: ReturnOptionsPurpose,
) -> Completion<Value> {
    let (mut ops, protocol) = match NativeReturnOps::selected(vm) {
        Ok(selected) => selected,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let prepared = match return_options::prepare_return(&mut ops, protocol, args, purpose) {
        Ok(prepared) => prepared,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    publish(vm, &mut ops, prepared)
}

pub(crate) fn publish(
    vm: &mut Vm,
    ops: &mut NativeReturnOps,
    prepared: return_options::PreparedReturn<Value>,
) -> Completion<Value> {
    let code = Code::from_int(prepared.code);
    if code == Code::Error
        && let Some(original) = prepared.result.as_ref()
    {
        vm.observe_native_error_result(original);
    }
    if code == Code::Error && ops.protocol == ReturnOptionsProtocol::Jim084 {
        for pair in &prepared.options {
            match pair.name_in(ops.protocol) {
                b"-errorinfo" => vm.jim_errors.stack.adopt_explicit(pair.value.clone()),
                b"-errorcode" => {
                    let _ = vm.set_var_bytes(b"::errorCode", pair.value.clone());
                    if let Some(refusal) = vm.refused_completion() {
                        return refusal;
                    }
                }
                _ => {}
            }
        }
    } else if code == Code::Error {
        for pair in &prepared.options {
            match pair.name_in(ops.protocol) {
                b"-errorinfo" => match ops.bytes(&pair.value) {
                    Ok(bytes) if !bytes.is_empty() => {
                        vm.seed_error_info_original(&pair.value, &bytes);
                    }
                    Ok(_) => {}
                    Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                },
                b"-errorstack" if vm.supports_error_stack() => match ops.list(&pair.value) {
                    Ok(parts) => vm.seed_error_stack_parts(&parts),
                    Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                },
                _ => {}
            }
        }
    }
    if code == Code::Error {
        let original = prepared
            .options
            .iter()
            .find(|pair| pair.name_in(ops.protocol) == b"-errorcode");
        let _ = vm.retain_return_error_code(
            original.map(|pair| &pair.value),
            ops.protocol != ReturnOptionsProtocol::Jim084,
        );
    }
    if code == Code::Error
        && vm
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
    {
        vm.mark_native_error_copy();
    }
    let mut options = Vec::with_capacity(prepared.options.len() * 2 + 4);
    options.extend([
        Value::string("-code"),
        Value::int(i64::from(prepared.code)),
        Value::string("-level"),
        Value::int(prepared.level),
    ]);
    for pair in prepared.options {
        // Legacy command grammar recognizes CString names; its completion
        // metadata uses the canonical standard key rather than a custom alias.
        let name = pair.name_in(ops.protocol);
        let key = if name == pair.key_bytes.as_slice() {
            pair.key
        } else {
            Value::from_string_bytes(name)
        };
        options.extend([key, pair.value]);
    }
    if ops.protocol == ReturnOptionsProtocol::Jim084 {
        vm.set_jim_return_state(prepared.code, prepared.level);
    } else {
        vm.set_native_c_return_state(prepared.code, prepared.level);
    }
    let options = Value::list(options);
    vm.retain_native_return_options(&options);
    Completion::new(
        if prepared.level == 0 {
            code
        } else {
            Code::Return
        },
        prepared.result.unwrap_or_else(Value::empty),
        options,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jim_private_state_retains_original_error_objects_after_ordinary_completion() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let trace = Value::list(vec![Value::from_string_bytes(&b"TRACE\xff"[..])]);
        let error_code = Value::list(vec![Value::string("CUSTOM")]);
        let result = apply(
            &mut vm,
            &[
                Value::string("-code"),
                Value::string("error"),
                Value::string("-level"),
                Value::int(3),
                Value::string("-errorinfo"),
                trace.clone(),
                Value::string("-errorcode"),
                error_code.clone(),
                Value::string("BODY"),
            ],
            ReturnOptionsPurpose::User,
        );
        assert_eq!(result.code, Code::Return);
        let receipt = vm.jim_return_receipt();
        assert_eq!(receipt.pending.level, 3);
        assert!(receipt.stack_trace.is_same_object(&trace));
        assert!(receipt.error_code.unwrap().is_same_object(&error_code));
        let options = vm.current_jim_options_for_exit_code(Code::Ok).unwrap();
        assert_eq!(
            crate::command::opt_get(&options, "-level")
                .unwrap()
                .as_int()
                .unwrap(),
            3
        );
        assert!(crate::command::opt_get(&options, "-errorinfo").is_none());
    }

    #[test]
    fn original_options_keep_custom_byte_key_and_value_identity() {
        let mut vm = Vm::new();
        let key = Value::from_string_bytes(&b"-custom\xff\0tail"[..]);
        let value = Value::list(vec![Value::string("VALUE")]);
        let options = Value::list(vec![
            Value::string("-level"),
            Value::int(0),
            key.clone(),
            value.clone(),
        ]);
        let result = apply(
            &mut vm,
            &[Value::string("-options"), options, Value::string("RESULT")],
            ReturnOptionsPurpose::User,
        );
        assert_eq!(result.code, Code::Ok);
        let pairs = result.options.as_list().unwrap();
        let index = pairs
            .iter()
            .position(|item| item.string_bytes().as_ref() == b"-custom\xff\0tail")
            .unwrap();
        assert!(pairs[index].is_same_object(&key));
        assert!(pairs[index + 1].is_same_object(&value));
    }

    #[test]
    fn opcode_dictionary_ingestion_uses_checked_internal_purpose() {
        let mut vm = Vm::new();
        let result = apply(
            &mut vm,
            &[
                Value::string("-options"),
                Value::string("{"),
                Value::string("RESULT"),
            ],
            ReturnOptionsPurpose::InternalDictionary,
        );
        assert_eq!(result.code, Code::Error);
        assert_eq!(
            result.result.string_bytes().as_ref(),
            b"expected dict but got \"{\""
        );
        assert_eq!(
            crate::command::opt_get(&result.options, "-errorcode")
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"TCL RESULT ILLEGAL_OPTIONS"
        );
    }
}
