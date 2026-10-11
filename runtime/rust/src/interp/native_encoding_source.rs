// SPDX-License-Identifier: AGPL-3.0-or-later
//! Thin original argv stages for the shared byte encoding/source owner.

use super::{Interp, native_arguments::OriginalArguments};
use std::rc::Rc;
use tcl_cmd_core::{
    CmdError,
    channel::ChannelEncoding,
    encoding::{NativeEncodingNameProtocol, NativeEncodingNamePurpose, OriginalSourceFileOps},
};
use tcl_platform::{HostError, SystemEncoding};
use tcl_syntax::value::ValueError;

pub(crate) fn lookup(
    interpreter: &mut Interp,
    arguments: &OriginalArguments,
    index: usize,
    protocol: NativeEncodingNameProtocol,
    purpose: NativeEncodingNamePurpose,
    bytes: &[u8],
) -> Result<ChannelEncoding, CmdError> {
    let original = arguments.original(index)?;
    arguments.check_original_for(interpreter, index, original)?;
    let host = interpreter.host();
    let result =
        tcl_cmd_core::encoding::resolve_original_encoding_name(protocol, purpose, bytes, |name| {
            let catalogue = host.encoding_catalogue();
            arguments.check_original_for(interpreter, index, original)?;
            let result = catalogue
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original encoding catalogue",
                ))?
                .lookup(name);
            arguments.check_original_for(interpreter, index, original)?;
            result.map_err(|_| {
                ValueError::CommandProtocolUnavailable("original encoding catalogue lookup").into()
            })
        });
    let result = result.map_err(|error| arguments.retain_refusal(error));
    arguments.check_original_for(interpreter, index, original)?;
    result
}

pub(crate) struct SourceOps<'a> {
    pub(crate) interpreter: &'a mut Interp,
    pub(crate) arguments: &'a OriginalArguments,
    pub(crate) encoding_at: Option<usize>,
    pub(crate) reached: Vec<usize>,
}

impl OriginalSourceFileOps for SourceOps<'_> {
    fn original_bytes(&mut self, index: usize) -> Result<Rc<[u8]>, CmdError> {
        self.reached.push(index + 1);
        self.arguments.bytes(index + 1)
    }
    fn system_encoding(&self) -> SystemEncoding {
        self.interpreter.system_encoding()
    }
    fn open_file(
        &mut self,
        path: &[u8],
    ) -> Result<Result<Box<dyn tcl_platform::SourceFile>, HostError>, CmdError> {
        let host = self.interpreter.host();
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
            self.interpreter,
            self.arguments,
            index + 1,
            protocol,
            NativeEncodingNamePurpose::Channel,
            name,
        )
    }
    fn retain_failure(&mut self, error: &CmdError) {
        if let Some(cause) = error.native_execution_refusal() {
            self.interpreter.refuse_native_execution(cause.clone());
        }
    }
    fn check(&mut self) -> Result<(), CmdError> {
        self.arguments.check_for(self.interpreter)?;
        for &index in &self.reached {
            let original = self.arguments.original(index)?;
            self.arguments
                .check_original_for(self.interpreter, index, original)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
