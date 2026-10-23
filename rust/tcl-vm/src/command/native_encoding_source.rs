// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original encoding-name and source stages under the actual Name operation.

use super::{Value, Vm, completion_from_cmd_error, completion_from_tcl_error, native_wrong_args};
use crate::interp::native_name_consumers::OriginalNameConsumerScope;
use std::rc::Rc;
use tcl_cmd_core::{
    CmdError,
    channel::ChannelEncoding,
    encoding::{NativeEncodingNameProtocol, NativeEncodingNamePurpose, OriginalSourceFileOps},
};
use tcl_platform::{HostError, SystemEncoding};
use tcl_runtime_api::Completion;
use tcl_syntax::value::ValueError;

fn retain(vm: &mut Vm, error: &CmdError) {
    if let Some(cause) = error.native_execution_refusal() {
        vm.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(cause.clone()));
    }
}

fn lookup(
    vm: &mut Vm,
    names: &OriginalNameConsumerScope,
    original: &Value,
    protocol: NativeEncodingNameProtocol,
    purpose: NativeEncodingNamePurpose,
    bytes: &[u8],
) -> Result<ChannelEncoding, CmdError> {
    names.current_for_value(vm, original)?;
    let result =
        tcl_cmd_core::encoding::resolve_original_encoding_name(protocol, purpose, bytes, |name| {
            let host = vm.host_rc();
            let catalogue = host.encoding_catalogue();
            names.current_for_value(vm, original)?;
            let result = catalogue
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original encoding catalogue",
                ))?
                .lookup(name);
            names.current_for_value(vm, original)?;
            result.map_err(|_| {
                ValueError::CommandProtocolUnavailable("original encoding catalogue lookup").into()
            })
        });
    if let Err(error) = &result {
        retain(vm, error);
    }
    names.current_for_value(vm, original)?;
    result
}

pub(super) fn set_system(vm: &mut Vm, original: &Value) -> Completion<Value> {
    let names = match OriginalNameConsumerScope::enter(vm) {
        Ok(names) => names,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let reached = (|| -> Result<Value, CmdError> {
        let protocol = vm
            .native_invocation_dialect()
            .native_encoding_name_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original encoding system protocol",
            ))?;
        let bytes = names.read(vm, original)?;
        let default = vm.default_system_encoding();
        let encoding = lookup(
            vm,
            &names,
            original,
            protocol,
            NativeEncodingNamePurpose::System(default),
            &bytes,
        )?
        .system_encoding()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "system encoding representation",
        ))?;
        names.check(vm)?;
        vm.set_system_encoding(encoding);
        names.check(vm)?;
        Ok(Value::empty())
    })();
    if let Err(error) = &reached {
        retain(vm, error);
    }
    if let Err(error) = names.finish(vm) {
        return completion_from_cmd_error(vm, error);
    }
    match reached {
        Ok(value) => super::ok(value),
        Err(error) => completion_from_cmd_error(vm, error),
    }
}

struct SourceOps<'a> {
    vm: &'a mut Vm,
    names: &'a OriginalNameConsumerScope,
    args: &'a [Value],
    encoding_at: Option<usize>,
    reached: Vec<usize>,
}
impl OriginalSourceFileOps for SourceOps<'_> {
    fn original_bytes(&mut self, index: usize) -> Result<Rc<[u8]>, CmdError> {
        self.reached.push(index);
        self.names.read(self.vm, &self.args[index])
    }
    fn external_filename_bytes(
        &mut self,
        units: &[u32],
        encoding: SystemEncoding,
    ) -> Result<Vec<u8>, CmdError> {
        let host = self.vm.host_rc();
        let filesystem = host.filesystem();
        self.check()?;
        let reached = filesystem.map_or_else(
            || {
                Err(
                    ValueError::CommandProtocolUnavailable("source filesystem filename conversion")
                        .into(),
                )
            },
            |filesystem| {
                filesystem
                    .source_filename_bytes(units, encoding)
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable(
                            "source filesystem filename conversion",
                        )
                        .into()
                    })
            },
        );
        if let Err(error) = &reached {
            retain(self.vm, error);
        }
        self.check()?;
        reached
    }
    fn system_encoding(&self) -> SystemEncoding {
        self.vm.system_encoding()
    }
    fn open_file(
        &mut self,
        path: &[u8],
    ) -> Result<Result<Box<dyn tcl_platform::SourceFile>, HostError>, CmdError> {
        let host = self.vm.host_rc();
        let filesystem = host.filesystem();
        self.check()?;
        Ok(filesystem.map_or(Err(HostError::Unsupported), |fs| fs.open_source_bytes(path)))
    }
    fn lookup_encoding(
        &mut self,
        protocol: NativeEncodingNameProtocol,
        name: &[u8],
    ) -> Result<ChannelEncoding, CmdError> {
        let index = self
            .encoding_at
            .ok_or(ValueError::CommandProtocolUnavailable(
                "source encoding operand",
            ))?;
        lookup(
            self.vm,
            self.names,
            &self.args[index],
            protocol,
            NativeEncodingNamePurpose::Channel,
            name,
        )
    }
    fn retain_failure(&mut self, error: &CmdError) {
        retain(self.vm, error);
    }
    fn check(&mut self) -> Result<(), CmdError> {
        self.names.check(self.vm)?;
        for &index in &self.reached {
            self.names.current_for_value(self.vm, &self.args[index])?;
        }
        Ok(())
    }
}

pub(super) fn source(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let names = match OriginalNameConsumerScope::enter(vm) {
        Ok(names) => names,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let operation = names.operation_scope().clone();
    let reached = source_in(vm, args, &names);
    if let Err(error) = names.finish(vm) {
        drop(reached);
        let first = operation.check(vm).err().unwrap_or(error);
        return completion_from_cmd_error(vm, first);
    }
    if let Err(error) = operation.check(vm) {
        drop(reached);
        let first = operation.check(vm).err().unwrap_or(error);
        return completion_from_cmd_error(vm, first);
    }
    reached
}

fn source_in(vm: &mut Vm, args: &[Value], names: &OriginalNameConsumerScope) -> Completion<Value> {
    let dialect = vm.native_invocation_dialect();
    let Some(grammar) = dialect.source_file_grammar() else {
        return completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("native source grammar").into(),
        );
    };
    let Some(protocol) = dialect.native_source_file_protocol() else {
        return completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("native source file protocol").into(),
        );
    };
    let first =
        if args.len() > 1 && grammar != tcl_registry::source_file::SourceFileGrammar::Filename {
            match names.read(vm, &args[0]) {
                Ok(bytes) => Some(bytes),
                Err(error) => return completion_from_cmd_error(vm, error),
            }
        } else {
            None
        };
    let tcl_registry::source_file::SourceFileSelection::Selected(selected) =
        grammar.select_original(args.len(), first.as_deref())
    else {
        return native_wrong_args(vm, grammar.synopsis());
    };
    let prepared = tcl_cmd_core::encoding::prepare_source_file(
        &mut SourceOps {
            vm,
            names,
            args,
            encoding_at: selected.encoding_at,
            reached: Vec::new(),
        },
        protocol,
        selected.path_at,
        selected.encoding_at,
    );
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let path = prepared.path.as_slice();
    let package_scope = selected.no_package.then(|| vm.take_package_file_scope());
    if let Err(error) = vm.record_package_source_file(path) {
        if let Some(scope) = package_scope {
            vm.restore_package_file_scope(scope);
        }
        return completion_from_cmd_error(vm, error.into());
    }
    vm.enter_package_source_path(path);
    let label = std::str::from_utf8(path).ok();
    if let Some(label) = label {
        vm.push_script(label.to_owned());
    }
    let image = tcl_lexer::SourceImage::native(std::sync::Arc::<[u8]>::from(prepared.script));
    let completion = match vm.eval_source_image_at_internal(&image, None) {
        Ok(completion) => completion,
        Err(error) => completion_from_tcl_error(vm, error),
    };
    // C's `Tcl_FSEvalFileEx`: an error leaving the file adds its
    // `(file "PATH" line N)` frame (the path cut at 150 bytes, `...` after a
    // cut from 8.5); an error a `return` settles names no file.
    if completion.code == tcl_core_types::Code::Error && !vm.uses_jim_error_stack() {
        let text = tcl_cmd_core::error::file_frame_text(path, vm.runtime_version());
        let mut frame = b"\n    (".to_vec();
        frame.extend_from_slice(&text);
        frame.extend_from_slice(format!(" line {})", vm.error_line()).as_bytes());
        let message = completion.result.string_bytes();
        vm.seed_error_info_frame(&*message, frame);
    }
    if label.is_some() {
        vm.pop_script();
    }
    vm.leave_package_source_path();
    if let Some(scope) = package_scope {
        vm.restore_package_file_scope(scope);
    }
    if let Err(error) = names.check(vm) {
        return completion_from_cmd_error(vm, error);
    }
    vm.settle_package_source_completion(completion)
}

#[cfg(test)]
mod tests;
