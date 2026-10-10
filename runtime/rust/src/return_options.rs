// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object return options and authentic primitive conversion.

use crate::{
    interp::{Code, Interp},
    obj::{self, Owned, TclObj},
};
use tcl_cmd_core::{
    return_options::{self, ReturnOptionsOps, ReturnOptionsProtocol, ReturnOptionsPurpose},
    CmdError,
};
use tcl_registry::InvocationDialect;
use tcl_syntax::{
    native_string::NativeStringProtocol,
    scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue},
    value::ValueError,
};

pub(crate) struct NativeReturnOps {
    pub(crate) dialect: InvocationDialect,
    pub(crate) string: NativeStringProtocol,
    protocol: ReturnOptionsProtocol,
    jim_context: Option<std::rc::Rc<crate::native_source::NativeJimObjectContext>>,
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

    pub(crate) fn selected(interp: &Interp) -> Result<(Self, ReturnOptionsProtocol), CmdError> {
        let dialect = interp.native_invocation_dialect();
        use tcl_registry::native_return_options::LogicalReturnOptionsProvider;
        let protocol = dialect
            .return_options_protocol()
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
                    Some(interp.native_jim_object_context()?)
                } else {
                    None
                },
            },
            protocol,
        ))
    }
}

impl ReturnOptionsOps for NativeReturnOps {
    type Value = Owned;
    fn bytes(&mut self, value: &Owned) -> Result<Vec<u8>, CmdError> {
        crate::typed_value::completion_code_string_bytes(value.as_ptr()).map_err(CmdError::from)
    }
    fn list(&mut self, value: &Owned) -> Result<Vec<Owned>, CmdError> {
        if matches!(
            crate::typed_value::completion_code_cache(value.as_ptr()),
            Some(return_options::CompletionCodeCache::Jim(_))
        ) && !obj::has_string_rep(value.as_ptr())
        {
            let _ = self.bytes(value)?;
        }
        if let Some(context) = &self.jim_context {
            crate::native_source::bind_context(value.as_ptr(), context)?;
        }
        crate::list::list_elements_native_checked(value.as_ptr(), self.string)
            .map(|items| items.into_iter().map(Owned::retain).collect())
            .map_err(CmdError::from)
    }
    fn new_string(&mut self, bytes: &[u8]) -> Self::Value {
        Owned::fresh(obj::new_string_bytes(bytes))
    }
    fn integer_probe(&mut self, value: &Owned, wide: bool) -> Result<Option<i64>, CmdError> {
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
        match crate::typed_value::native_scalar_probe(
            value.as_ptr(),
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
    fn completion_code_cache(&self, value: &Owned) -> Option<i32> {
        if let Some(protocol) = self.dialect.native_index_lookup_protocol() {
            let table = tcl_registry::native_return_options::completion_code_table();
            if let Ok(Some(index)) =
                protocol.cached_index(crate::obj::native_index::cache(value.as_ptr()), &table)
            {
                return i32::try_from(index).ok();
            }
        }
        match (
            crate::typed_value::completion_code_cache(value.as_ptr())?,
            self.string,
        ) {
            (return_options::CompletionCodeCache::Jim(code), NativeStringProtocol::Jim084)
            | (return_options::CompletionCodeCache::TclKeyword(code), NativeStringProtocol::C(_)) => {
                Some(code)
            }
            _ => None,
        }
    }
    fn adopt_completion_code_cache(
        &mut self,
        value: &Owned,
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
            crate::obj::native_index::install(value.as_ptr(), table.cache(index), protocol)?;
            return Ok(());
        }

        crate::typed_value::adopt_completion_code_cache(value.as_ptr(), cache)
            .map_err(CmdError::from)
    }
}

pub(crate) fn command(interp: &mut Interp, args: &[*mut TclObj]) -> Code {
    let (mut ops, protocol) = match NativeReturnOps::selected(interp) {
        Ok(selected) => selected,
        Err(error) => return interp.report_cmd_error(error),
    };
    let args: Vec<_> = args.iter().copied().map(Owned::retain).collect();
    let prepared =
        match return_options::prepare_return(&mut ops, protocol, &args, ReturnOptionsPurpose::User)
        {
            Ok(prepared) => prepared,
            Err(error) => return interp.report_cmd_error(error),
        };
    publish(interp, &mut ops, prepared)
}

pub(crate) fn publish(
    interp: &mut Interp,
    ops: &mut NativeReturnOps,
    prepared: return_options::PreparedReturn<Owned>,
) -> Code {
    let code = Code::from_int(prepared.code);
    let level = match usize::try_from(prepared.level) {
        Ok(level) => level,
        Err(_) => {
            return interp.report_cmd_error(
                ValueError::CommandProtocolUnavailable("return level storage").into(),
            );
        }
    };
    if code == Code::Error && ops.protocol == ReturnOptionsProtocol::Jim084 {
        for pair in &prepared.options {
            match pair.name_in(ops.protocol) {
                b"-errorinfo" => interp.adopt_jim_stacktrace(pair.value.clone()),
                b"-errorcode" => {
                    let _ = interp.var_set(b"::errorCode", pair.value.as_ptr());
                    if interp.host_refusal_pending() {
                        return Code::Error;
                    }
                }
                _ => {}
            }
        }
    } else if code == Code::Error {
        let mut info = None;
        let mut errorcode = None;
        let mut stack = None;
        for pair in &prepared.options {
            let destination = match pair.name_in(ops.protocol) {
                b"-errorinfo" => &mut info,
                b"-errorcode" => &mut errorcode,
                b"-errorstack" => &mut stack,
                _ => continue,
            };
            match ops.bytes(&pair.value) {
                Ok(bytes) => *destination = Some(bytes),
                Err(error) => return interp.report_cmd_error(error),
            }
        }
        if ops.protocol == ReturnOptionsProtocol::Tcl84 {
            for value in [&mut info, &mut errorcode].into_iter().flatten() {
                value.truncate(
                    value
                        .iter()
                        .position(|&byte| byte == 0)
                        .unwrap_or(value.len()),
                );
            }
        }
        interp.process_return_error(info.as_deref(), errorcode.as_deref(), stack.as_deref());
        for pair in &prepared.options {
            match pair.name_in(ops.protocol) {
                b"-errorinfo" if info.as_ref().is_some_and(|bytes| !bytes.is_empty()) => {
                    interp.retain_native_error_option(true, pair.value.as_ptr())
                }
                b"-errorcode" => interp.retain_native_error_option(false, pair.value.as_ptr()),
                b"-errorstack" if interp.runtime_version().has_error_stack() => {
                    if let Err(error) = interp.seed_original_error_stack(pair.value.as_ptr()) {
                        return interp.report_cmd_error(error.into());
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(result) = prepared.result {
        interp.set_result(result.as_ptr());
    }

    interp.set_return_option_objects(prepared.options);
    if ops.protocol == ReturnOptionsProtocol::Jim084 {
        interp.set_return_state(level, code);
    }
    if level == 0 {
        code
    } else {
        interp.set_return_state(level, code);
        Code::Return
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_and_catch_keep_original_custom_option_objects() {
        let mut interp = Interp::new();
        let key = Owned::fresh(obj::new_string_bytes(b"-custom\xff\0tail"));
        let value = Owned::fresh(crate::list::new_list_obj(&[obj::new_string_bytes(
            b"VALUE",
        )]));
        let argv = [
            Owned::fresh(obj::new_string_bytes(b"-level")),
            Owned::fresh(obj::new_wide_int_obj(0)),
            key.clone(),
            value.clone(),
            Owned::fresh(obj::new_string_bytes(b"RESULT")),
        ];
        let raw: Vec<_> = argv.iter().map(Owned::as_ptr).collect();
        assert_eq!(command(&mut interp, &raw), Code::Ok);
        let carried = interp.pending_return_option_objects();
        assert_eq!(carried[0].key.as_ptr(), key.as_ptr());
        assert_eq!(carried[0].value.as_ptr(), value.as_ptr());
        let options =
            Owned::fresh(crate::cmd_error::completion_options(&mut interp, Code::Ok).unwrap());
        let pairs = crate::dict::dict_pairs(options.as_ptr()).unwrap();
        assert!(
            pairs
                .iter()
                .any(|&(stored_key, stored_value)| stored_key == key.as_ptr()
                    && stored_value == value.as_ptr())
        );
    }

    #[test]
    fn duplicate_controls_validate_only_selected_value_on_modern_tcl() {
        let mut interp = Interp::new();
        assert_eq!(
            interp.eval_str(b"return -level 0 -code invalid -code ok VALUE"),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"VALUE");
        assert_eq!(
            interp.eval_str(b"return -level 0 -options {-custom A -custom B} VALUE"),
            Code::Ok
        );
        let carried = interp.pending_return_option_objects();
        assert_eq!(carried.len(), 1);
        assert_eq!(obj::bytes_of(carried[0].value.as_ptr()), b"B");
    }
}
