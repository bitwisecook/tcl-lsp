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

//! I/O channel commands: `open`, `close`, `gets`, `read`, `eof`, `flush`,
//! `seek`, `tell`, `fconfigure`, and channel-aware `puts`.
//!
//! Channels back onto the real filesystem (the test host). Read channels
//! preload the file contents (test inputs are small); writable channels,
//! read-write ones included, hold an `std::fs::File`. The predefined `stdin`/`stdout`/`stderr` are not
//! stored in the channel table — they are handled by name: `stdout` follows the
//! VM's captured output, `stderr` the process stderr, and `stdin` is always at
//! end-of-file (non-interactive).

use std::fs::OpenOptions;
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};

use tcl_cmd_core::CmdError;
use tcl_cmd_core::channel::{
    ChannelConfig, channel_output_error, config_list, config_value, encode_output,
    is_standard_channel, resolve_open_access_mode, set_config_value,
};
use tcl_runtime_api::Completion;

use crate::error::TclError;
use crate::expr::{
    OriginalNumericCommandContext, original_numeric_command_ops, with_original_numeric_command,
};
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use tcl_registry::native_numeric_error::{
    NativeNumericCommandProtocol, NativeNumericCommandPurpose, NativeReadCountProtocol,
};

/// An open file channel.
pub(crate) enum Channel {
    /// A readable channel: whole-file contents with a byte cursor.
    Read {
        data: Vec<u8>,
        pos: usize,
        config: ChannelConfig,
        path: Option<std::rc::Rc<[u8]>>,
    },
    /// A writable channel over the file itself; `readable` for the read-write
    /// modes (`r+`, `w+`, `a+`, `RDWR`), whose reads and writes share the
    /// file offset.
    Write {
        file: std::fs::File,
        readable: bool,
        eof: bool,
        config: ChannelConfig,
        path: Option<std::rc::Rc<[u8]>>,
    },
}

impl Channel {
    fn config(&self) -> ChannelConfig {
        match self {
            Self::Read { config, .. } | Self::Write { config, .. } => *config,
        }
    }

    fn path(&self) -> Option<&std::rc::Rc<[u8]>> {
        match self {
            Self::Read { path, .. } | Self::Write { path, .. } => path.as_ref(),
        }
    }

    fn seek(&mut self, offset: i64, origin: usize) -> Result<(), std::io::Error> {
        match self {
            Self::Read { data, pos, .. } => {
                let base = match origin {
                    1 => *pos as i128,
                    2 => data.len() as i128,
                    _ => 0,
                };
                let target = base + i128::from(offset);
                let target = usize::try_from(target)
                    .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
                *pos = target;
                Ok(())
            }
            Self::Write { file, eof, .. } => {
                let target = match origin {
                    1 => std::io::SeekFrom::Current(offset),
                    2 => std::io::SeekFrom::End(offset),
                    _ => std::io::SeekFrom::Start(
                        u64::try_from(offset)
                            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?,
                    ),
                };
                file.seek(target).map(|_| *eof = false)
            }
        }
    }

    fn set_config(&mut self, replacement: ChannelConfig) {
        match self {
            Self::Read { config, .. } | Self::Write { config, .. } => *config = replacement,
        }
    }
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register_stock_builtin("open", cmd_open);
    vm.register_stock_builtin("close", cmd_close);
    vm.register_stock_builtin("gets", cmd_gets);
    vm.register_stock_builtin("read", cmd_read);
    vm.register_stock_builtin("eof", cmd_eof);
    vm.register_stock_builtin("flush", cmd_flush);
    vm.register_stock_builtin("seek", cmd_seek);
    vm.register_stock_builtin("tell", cmd_tell);
    vm.register_stock_builtin("fconfigure", cmd_fconfigure);
    vm.register_stock_builtin("fblocked", |_vm, _args| ok(Value::int(0)));
}

/// `open fileName ?access?` — supports the common access strings
/// (`r r+ w w+ a a+`). Pipes (`|cmd`) and serial ports are unsupported.
fn cmd_open(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match with_original_numeric_command(vm, TclError::from_execution_failure, |vm, context| {
        cmd_open_in_original(vm, args, context)
    }) {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    }
}

fn cmd_open_in_original(
    vm: &mut Vm,
    args: &[Value],
    context: OriginalNumericCommandContext<'_>,
) -> Result<Completion<Value>, TclError> {
    let (name, mode) = match args {
        [n] => (
            channel_operand(vm, n, context)?.to_string(),
            "r".to_string(),
        ),
        [n, m] => (
            channel_operand(vm, n, context)?.to_string(),
            channel_operand(vm, m, context)?.to_string(),
        ),
        // The permissions argument is accepted but ignored (umask applies).
        [n, m, _perm] => (
            channel_operand(vm, n, context)?.to_string(),
            channel_operand(vm, m, context)?.to_string(),
        ),
        _ => {
            return Ok(crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"open fileName ?access? ?permissions?\"",
            ));
        }
    };
    if name.starts_with('|') {
        return Ok(err("command pipelines are not supported"));
    }
    let m = mode.trim();
    let version = vm.runtime_version();
    let access = match resolve_open_access_mode(version, m) {
        Ok(access) => access,
        Err(error) => return Ok(vm.cmd_error(error)),
    };
    let config = ChannelConfig::for_open_access(version, vm.system_encoding(), access);

    context
        .current(vm)
        .map_err(TclError::from_execution_failure)?;
    let path: std::rc::Rc<[u8]> = std::rc::Rc::from(name.as_bytes());
    Ok(if access.is_readable() && !access.is_writable() {
        match std::fs::read(&name) {
            Ok(data) => {
                context
                    .current(vm)
                    .map_err(TclError::from_execution_failure)?;
                let id = vm.add_channel(Channel::Read {
                    data,
                    pos: 0,
                    config,
                    path: Some(std::rc::Rc::clone(&path)),
                });
                ok(Value::string(id))
            }
            Err(e) => err(format!("couldn't open \"{name}\": {}", io_reason(&e))),
        }
    } else {
        let mut opts = OpenOptions::new();
        opts.read(access.is_readable())
            .write(access.is_writable())
            .append(access.appends())
            .truncate(access.truncates())
            .create(access.creates())
            .create_new(access.creates() && access.is_exclusive());
        // `Tcl_FSOpenFileChannel` starts an appending channel at the end.
        let opened = opts.open(&name).and_then(|mut file| {
            if access.appends() {
                file.seek(SeekFrom::End(0))?;
            }
            Ok(file)
        });
        match opened {
            Ok(file) => {
                context
                    .current(vm)
                    .map_err(TclError::from_execution_failure)?;
                let id = vm.add_channel(Channel::Write {
                    file,
                    readable: access.is_readable(),
                    eof: false,
                    config,
                    path: Some(path),
                });
                ok(Value::string(id))
            }
            Err(e) => err(format!("couldn't open \"{name}\": {}", io_reason(&e))),
        }
    })
}

fn io_reason(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => "no such file or directory".to_string(),
        std::io::ErrorKind::PermissionDenied => "permission denied".to_string(),
        _ => e.to_string(),
    }
}

fn cmd_close(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [chan] = args else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"close channelId\"",
        );
    };
    let id = chan.to_str();
    if is_std(&id) {
        return ok(Value::empty());
    }
    if vm.remove_channel(&id) {
        ok(Value::empty())
    } else {
        err(format!("can not find channel named \"{id}\""))
    }
}

/// `channel "…" wasn't opened for reading` — C's refusal (no `-errorcode`).
fn not_readable(id: &str) -> Completion<Value> {
    err(format!("channel \"{id}\" wasn't opened for reading"))
}

/// Read up to `limit` bytes (all of them for `None`) from a read-write
/// channel's file, noting when the read met the end.
fn read_file(
    file: &mut std::fs::File,
    eof: &mut bool,
    limit: Option<usize>,
) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    if let Some(n) = limit {
        let read = (&mut *file).take(n as u64).read_to_end(&mut bytes)?;
        *eof = read < n;
    } else {
        file.read_to_end(&mut bytes)?;
        *eof = true;
    }
    Ok(bytes)
}

/// Read one line (its newline consumed, not returned) from a read-write
/// channel's file; `None` when the file was already at its end.
fn read_file_line(file: &mut std::fs::File, eof: &mut bool) -> std::io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if file.read(&mut byte)? == 0 {
            *eof = true;
            return Ok((!line.is_empty()).then_some(line));
        }
        if byte[0] == b'\n' {
            return Ok(Some(line));
        }
        line.push(byte[0]);
    }
}

/// `gets channelId ?varName?` — read one line (without the trailing newline).
/// With `varName`, store the line and return its length (`-1` at end-of-file);
/// without, return the line.
fn cmd_gets(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (id, var) = match args {
        [c] => (c.to_str().to_string(), None),
        [c, v] => (c.to_str().to_string(), Some(v.to_str().to_string())),
        _ => {
            return crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"gets channelId ?varName?\"",
            );
        }
    };
    if id == "stdout" || id == "stderr" {
        return not_readable(&id);
    }
    if is_std(&id) {
        // stdin is non-interactive: immediate EOF.
        return match var {
            Some(name) => {
                if let Err(c) = vm.set_var(&name, Value::string("")) {
                    return c;
                }
                ok(Value::int(-1))
            }
            None => ok(Value::string("")),
        };
    }
    let line = match vm.channel_mut(&id) {
        Some(Channel::Read { data, pos, .. }) => {
            if *pos >= data.len() {
                None
            } else {
                let start = *pos;
                let mut end = start;
                while end < data.len() && data[end] != b'\n' {
                    end += 1;
                }
                *pos = if end < data.len() { end + 1 } else { end };
                Some(data[start..end].to_vec())
            }
        }
        Some(Channel::Write {
            file,
            readable: true,
            eof,
            ..
        }) => match read_file_line(file, eof) {
            Ok(line) => line,
            Err(e) => return err(format!("error reading \"{id}\": {}", io_reason(&e))),
        },
        Some(Channel::Write { .. }) => return not_readable(&id),
        None => return err(format!("can not find channel named \"{id}\"")),
    };
    // Strip a trailing CR for CRLF inputs.
    let line = line.map(|mut bytes| {
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
        String::from_utf8_lossy(&bytes).into_owned()
    });
    match (var, line) {
        (Some(name), Some(s)) => {
            let n = s.chars().count();
            if let Err(c) = vm.set_var(&name, Value::string(s)) {
                return c;
            }
            ok(Value::int(i64::try_from(n).unwrap_or(i64::MAX)))
        }
        (Some(name), None) => {
            if let Err(c) = vm.set_var(&name, Value::string("")) {
                return c;
            }
            ok(Value::int(-1))
        }
        (None, Some(s)) => ok(Value::string(s)),
        (None, None) => ok(Value::string("")),
    }
}

/// `read ?-nonewline? channelId` (or `read channelId ?numChars?`) — read the
/// remaining contents (or `numChars` bytes).
fn cmd_read(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match with_original_numeric_command(vm, TclError::from_execution_failure, |vm, original| {
        cmd_read_in_original(vm, args, original)
    }) {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    }
}

fn channel_operand(
    vm: &mut Vm,
    value: &Value,
    context: OriginalNumericCommandContext<'_>,
) -> Result<std::rc::Rc<str>, TclError> {
    context
        .current(vm)
        .map_err(TclError::from_execution_failure)?;
    let reached = if context.authored.is_some() {
        value.try_to_str().map_err(TclError::from)
    } else {
        vm.native_name_operand_bytes_checked(value)
            .and_then(|bytes| {
                tcl_syntax::raw_string::RawString::from_bytes(bytes)
                    .unicode()
                    .map_err(Into::into)
            })
            .map_err(|error| crate::command::completion_option_failure(vm, error))
    };
    context
        .current(vm)
        .map_err(TclError::from_execution_failure)?;
    reached
}

fn native_read_leading_option(
    original: &crate::interp::native_numeric_ops::OriginalNumericOps<'_>,
    selected: NativeNumericCommandProtocol,
    value: &Value,
    policy: NativeReadCountProtocol,
    string: &impl Fn() -> Result<std::rc::Rc<[u8]>, CmdError>,
) -> Result<Option<(Option<i64>, bool)>, CmdError> {
    if matches!(
        policy,
        NativeReadCountProtocol::LeadingDigitOrSuffix | NativeReadCountProtocol::JimAio
    ) {
        let spelling = string()?;
        let spelling = tcl_core_types::c_string_extent(&spelling);
        if policy == NativeReadCountProtocol::LeadingDigitOrSuffix
            && !spelling.first().is_some_and(u8::is_ascii_digit)
        {
            if spelling == b"nonewline" {
                return Ok(Some((None, true)));
            }
            let mut message = b"bad argument \"".to_vec();
            message.extend_from_slice(spelling);
            message.extend_from_slice(b"\": should be \"nonewline\"");
            return Err(CmdError::with_error_code_bytes(message, b"NONE".to_vec()));
        }
        if policy == NativeReadCountProtocol::JimAio && spelling.first() == Some(&b'-') {
            let options = selected.read_leading_options().ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "read leading option purpose",
                ),
            )?;
            let index = original.with_vm_for_value(value, |vm| {
                vm.native_static_option_index_checked(value, options, true, "option")
            })?;
            return Ok(Some((
                None,
                selected.read_option_suppresses_newline(index).ok_or(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "read leading option selection",
                    ),
                )?,
            )));
        }
    }
    Ok(None)
}

fn quiet_native_read_count(
    original: &crate::interp::native_numeric_ops::OriginalNumericOps<'_>,
    selected: NativeNumericCommandProtocol,
    value: &Value,
    string: &impl Fn() -> Result<std::rc::Rc<[u8]>, CmdError>,
) -> Result<(Option<i64>, bool), CmdError> {
    match original.probe(value, selected.getter_kind())? {
        Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(count)) if count >= 0 => {
            Ok((Some(count), false))
        }
        Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(_)) | Err(_) => {
            let spelling = string()?;
            let error = selected.negative_read_error(&spelling).ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "nonnegative read diagnostic",
                ),
            )?;
            Err(CmdError::with_error_code_bytes(
                error.message,
                error.error_code,
            ))
        }
        Ok(_) => Err(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("read Wide output").into(),
        ),
    }
}

fn native_read_count_operand(
    original: &crate::interp::native_numeric_ops::OriginalNumericOps<'_>,
    selected: NativeNumericCommandProtocol,
    value: &Value,
) -> Result<(Option<i64>, bool), CmdError> {
    let policy =
        selected
            .read_count()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "read count purpose",
            ))?;
    let string = || {
        original.string(
            value,
            original
                .dialect()
                .native_string_protocol()
                .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?,
        )
    };
    if let Some(option) = native_read_leading_option(original, selected, value, policy, &string)? {
        return Ok(option);
    }
    if policy == NativeReadCountProtocol::QuietNonnegativeWide {
        return quiet_native_read_count(original, selected, value, &string);
    }
    let reached = original.getter(value, selected.getter_kind());
    original.current_for_value(value)?;
    match reached {
        Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(count))
            if count >= 0 || policy == NativeReadCountProtocol::LeadingDigitOrSuffix =>
        {
            Ok((Some(count), false))
        }
        Err(error)
            if error.native_access_refusal().is_some()
                || error.native_execution_refusal().is_some() =>
        {
            Err(error)
        }
        Err(error) if policy == NativeReadCountProtocol::IntOrSuffix => {
            if tcl_core_types::c_string_extent(&string()?) == b"nonewline" {
                drop(error);
                original.current_for_value(value)?;
                Ok((None, true))
            } else {
                Err(error)
            }
        }
        Err(error) if policy == NativeReadCountProtocol::NonnegativeIntOrSuffix => {
            drop(error);
            original.current_for_value(value)?;
            let spelling = string()?;
            if tcl_core_types::c_string_extent(&spelling) == b"nonewline" {
                return Ok((None, true));
            }
            let error = selected.negative_read_error(&spelling).ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "nonnegative read diagnostic",
                ),
            )?;
            Err(CmdError::with_error_code_bytes(
                error.message,
                error.error_code,
            ))
        }
        Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(_)) => {
            let spelling = string()?;
            if policy == NativeReadCountProtocol::NonnegativeIntOrSuffix
                && tcl_core_types::c_string_extent(&spelling) == b"nonewline"
            {
                return Ok((None, true));
            }
            let error = selected.negative_read_error(&spelling).ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "nonnegative read diagnostic",
                ),
            )?;
            Err(CmdError::with_error_code_bytes(
                error.message,
                error.error_code,
            ))
        }
        Err(error) => Err(error),
        Ok(_) => Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "read integer output",
        )
        .into()),
    }
}

fn read_count_operand(
    vm: &mut Vm,
    value: &Value,
    context: OriginalNumericCommandContext<'_>,
) -> Result<(Option<i64>, bool), TclError> {
    if let Some(authored) = context.authored {
        let reached = value.as_int_with_stages(Some(authored));
        context
            .current(vm)
            .map_err(TclError::from_execution_failure)?;
        let count = reached?;
        if count < 0 {
            return Err(TclError::new("expected non-negative read count"));
        }
        return Ok((Some(count), false));
    }
    let reached = original_numeric_command_ops(
        vm,
        context,
        NativeNumericCommandPurpose::ReadCount,
        |original, selected| native_read_count_operand(original, selected, value),
    );
    context
        .current(vm)
        .map_err(TclError::from_execution_failure)?;
    reached.map_err(|error| crate::command::completion_option_failure(vm, error))
}

fn cmd_read_in_original(
    vm: &mut Vm,
    args: &[Value],
    context: OriginalNumericCommandContext<'_>,
) -> Result<Completion<Value>, TclError> {
    let mut nonewline = false;
    let mut rest = args;
    if let Some(first) = rest.first()
        && channel_operand(vm, first, context)?.as_ref() == "-nonewline"
    {
        nonewline = true;
        rest = &rest[1..];
    }
    let (channel, count) = match rest {
        [channel] => (channel, None),
        [channel, count] if !nonewline => (channel, Some(count)),
        _ => {
            return Ok(crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"read channelId ?numChars?\"",
            ));
        }
    };
    let id = channel_operand(vm, channel, context)?.to_string();
    // C finds a readable channel before reading the count.
    if !is_std(&id) {
        match vm.channel(&id) {
            Some(Channel::Read { .. } | Channel::Write { readable: true, .. }) => {}
            Some(Channel::Write { .. }) => return Ok(not_readable(&id)),
            None => return Ok(err(format!("can not find channel named \"{id}\""))),
        }
    } else if id != "stdin" {
        return Ok(not_readable(&id));
    }
    let count = if let Some(value) = count {
        let (count, suffix) = read_count_operand(vm, value, context)?;
        nonewline |= suffix;
        count
    } else {
        None
    };
    context
        .current(vm)
        .map_err(TclError::from_execution_failure)?;
    if is_std(&id) {
        return Ok(ok(Value::string("")));
    }
    Ok(match vm.channel_mut(&id) {
        Some(Channel::Read { data, pos, .. }) => {
            let start = (*pos).min(data.len());
            let end = match count {
                Some(count) if count >= 0 => start
                    .saturating_add(usize::try_from(count).unwrap_or(usize::MAX))
                    .min(data.len()),
                _ => data.len(),
            };
            let mut bytes = &data[start..end];
            if nonewline && bytes.last() == Some(&b'\n') {
                bytes = &bytes[..bytes.len() - 1];
            }
            let string = String::from_utf8_lossy(bytes).into_owned();
            *pos = pos.saturating_add(end - start);
            ok(Value::string(string))
        }
        Some(Channel::Write { file, eof, .. }) => {
            // A read-write channel reads the file at its shared offset.
            let limit = count.and_then(|count| usize::try_from(count).ok());
            match read_file(file, eof, limit) {
                Ok(bytes) => {
                    let mut bytes = &bytes[..];
                    if nonewline && bytes.last() == Some(&b'\n') {
                        bytes = &bytes[..bytes.len() - 1];
                    }
                    ok(Value::string(String::from_utf8_lossy(bytes).into_owned()))
                }
                Err(e) => err(format!("error reading \"{id}\": {}", io_reason(&e))),
            }
        }
        None => err(format!("can not find channel named \"{id}\"")),
    })
}
fn cmd_eof(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [chan] = args else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"eof channelId\"",
        );
    };
    let id = chan.to_str();
    if is_std(&id) {
        return ok(Value::bool(&*id == "stdin"));
    }
    match vm.channel_mut(&id) {
        Some(Channel::Read { data, pos, .. }) => ok(Value::bool(*pos >= data.len())),
        Some(Channel::Write { eof, .. }) => ok(Value::bool(*eof)),
        None => err(format!("can not find channel named \"{id}\"")),
    }
}

fn cmd_flush(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [chan] = args else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"flush channelId\"",
        );
    };
    let id = chan.to_str();
    if is_std(&id) {
        return ok(Value::empty());
    }
    match vm.channel_mut(&id) {
        Some(Channel::Write { file, .. }) => {
            let _ = file.flush();
            ok(Value::empty())
        }
        Some(Channel::Read { .. }) => ok(Value::empty()),
        None => err(format!("can not find channel named \"{id}\"")),
    }
}

fn cmd_tell(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [chan] = args else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"tell channelId\"",
        );
    };
    let id = chan.to_str();
    match vm.channel_mut(&id) {
        Some(Channel::Read { pos, .. }) => ok(Value::int(i64::try_from(*pos).unwrap_or(i64::MAX))),
        Some(Channel::Write { file, .. }) => ok(Value::int(
            file.stream_position()
                .map_or(-1, |pos| i64::try_from(pos).unwrap_or(i64::MAX)),
        )),
        None => ok(Value::int(-1)),
    }
}

/// `seek`'s origin word, in C table order (`originOptions[]`, `tclIOCmd.c`):
/// `Tcl_GetIndexFromObj(…, "origin", 0)`, so `s`/`c`/`e` abbreviate and the
/// empty word — a prefix of all three — is `ambiguous origin ""`.
const SEEK_ORIGINS: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("origin", &["start", "current", "end"]);

fn cmd_seek(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match with_original_numeric_command(vm, TclError::from_execution_failure, |vm, original| {
        cmd_seek_in_original(vm, args, original)
    }) {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    }
}

fn seek_offset_operand(
    vm: &mut Vm,
    value: &Value,
    context: OriginalNumericCommandContext<'_>,
) -> Result<i64, TclError> {
    if let Some(authored) = context.authored {
        let reached = value.as_int_with_stages(Some(authored));
        context
            .current(vm)
            .map_err(TclError::from_execution_failure)?;
        return reached;
    }
    let reached = original_numeric_command_ops(
        vm,
        context,
        NativeNumericCommandPurpose::SeekOffset,
        |original, selected| match original.getter(value, selected.getter_kind())? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(offset) => Ok(offset),
            _ => Err(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable("seek Wide output")
                    .into(),
            ),
        },
    );
    context
        .current(vm)
        .map_err(TclError::from_execution_failure)?;
    reached.map_err(|error| crate::command::completion_option_failure(vm, error))
}

fn seek_origin_operand(
    vm: &mut Vm,
    value: &Value,
    context: OriginalNumericCommandContext<'_>,
    id: &str,
    selected: Option<NativeNumericCommandProtocol>,
) -> Result<usize, TclError> {
    let reached = if selected.is_some_and(NativeNumericCommandProtocol::seek_origin_is_exact) {
        let spelling = channel_operand(vm, value, context)?;
        match spelling.as_ref() {
            "start" => Ok(0),
            "current" => Ok(1),
            "end" => Ok(2),
            _ => Err(CmdError::wrong_args_bytes(
                format!("{id} seek offset ?start|current|end").as_bytes(),
            )),
        }
    } else {
        vm.native_index_operand_checked(
            value,
            &tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(
                SEEK_ORIGINS.names(),
            ),
            false,
            "origin",
        )
    };
    context
        .current(vm)
        .map_err(TclError::from_execution_failure)?;
    reached.map_err(|error| crate::command::completion_option_failure(vm, error))
}

fn cmd_seek_in_original(
    vm: &mut Vm,
    args: &[Value],
    context: OriginalNumericCommandContext<'_>,
) -> Result<Completion<Value>, TclError> {
    let (channel, offset, origin) = match args {
        [channel, offset] => (channel, offset, None),
        [channel, offset, origin] => (channel, offset, Some(origin)),
        _ => {
            return Ok(crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"seek channelId offset ?origin?\"",
            ));
        }
    };
    let id = channel_operand(vm, channel, context)?.to_string();
    if vm.channel(&id).is_none() {
        return Ok(err(format!("can not find channel named \"{id}\"")));
    }
    let selected = if context.authored.is_none() {
        Some(
            vm.native_invocation_dialect()
                .native_numeric_command_protocol(NativeNumericCommandPurpose::SeekOffset)
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "seek offset purpose",
                ))?,
        )
    } else {
        None
    };
    let origin_first = selected.is_some_and(NativeNumericCommandProtocol::seek_origin_first);
    let early_origin = if origin_first {
        origin
            .map(|value| seek_origin_operand(vm, value, context, &id, selected))
            .transpose()?
            .unwrap_or(0)
    } else {
        0
    };
    let offset = seek_offset_operand(vm, offset, context)?;
    let origin = if origin_first {
        early_origin
    } else {
        origin
            .map(|value| seek_origin_operand(vm, value, context, &id, selected))
            .transpose()?
            .unwrap_or(0)
    };
    context
        .current(vm)
        .map_err(TclError::from_execution_failure)?;
    let channel =
        vm.channel_mut(&id)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "selected seek channel",
            ))?;
    match channel.seek(offset, origin) {
        Ok(()) => Ok(ok(Value::empty())),
        Err(error) => {
            let path = channel.path().map(std::rc::Rc::clone);
            let selected =
                selected.ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "authored seek backend error",
                ))?;
            let error = (error.kind() == std::io::ErrorKind::InvalidInput)
                .then(|| selected.negative_seek_error(id.as_bytes(), path.as_deref()))
                .flatten()
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "seek backend diagnostic",
                ))?;
            context
                .current(vm)
                .map_err(TclError::from_execution_failure)?;
            Err(crate::command::completion_option_failure(
                vm,
                CmdError::with_error_code_bytes(error.message, error.error_code),
            ))
        }
    }
}
/// `fconfigure channelId ?option ?value?? ...` — retain the options that
/// affect byte output; conventional non-output options remain accepted.
fn cmd_fconfigure(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((chan, rest)) = args.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"fconfigure channelId ?-option value ...?\"",
        );
    };
    let id = chan.to_str().to_string();
    let Some(mut config) = channel_config(vm, &id) else {
        return err(format!("can not find channel named \"{id}\""));
    };
    let version = vm.runtime_version();
    match rest {
        [] => ok(Value::string(config_list(version, &id, config))),
        [opt] => {
            let option = match tcl_registry::commands::tcl::resolve_fconfigure_option(
                vm.dialect_profile(),
                &opt.to_str(),
            ) {
                Ok(option) => option,
                Err(error) => return vm.cmd_error(error),
            };
            ok(Value::string(config_value(option, &id, config)))
        }
        values if values.len() % 2 == 0 => {
            for pair in values.as_chunks::<2>().0 {
                let option = match tcl_registry::commands::tcl::resolve_fconfigure_option(
                    vm.dialect_profile(),
                    &pair[0].to_str(),
                ) {
                    Ok(option) => option,
                    Err(error) => return vm.cmd_error(error),
                };
                let configured = set_config_value(version, &mut config, option, &pair[1].to_str());
                // Tcl preserves any direction changed before a later
                // direction of the same translation value reports an error.
                set_channel_config(vm, &id, config);
                if let Err(error) = configured {
                    return vm.cmd_error(error);
                }
            }
            ok(Value::empty())
        }
        _ => crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"fconfigure channelId ?-option value ...?\"",
        ),
    }
}

fn channel_config(vm: &Vm, id: &str) -> Option<ChannelConfig> {
    if is_std(id) {
        vm.standard_channels.borrow().get(id)
    } else {
        vm.channel(id).map(Channel::config)
    }
}

fn set_channel_config(vm: &mut Vm, id: &str, config: ChannelConfig) {
    if is_std(id) {
        let _ = vm.standard_channels.borrow_mut().set(id, config);
    } else if let Some(channel) = vm.channel_mut(id) {
        channel.set_config(config);
    }
}

/// True for a predefined standard channel name.
pub(crate) fn is_std(id: &str) -> bool {
    is_standard_channel(id)
}

/// Write `text` (optionally with a trailing newline) to channel `id`. Used by
/// `puts`. Standard channels follow the VM's output / process stderr.
pub(crate) fn chan_puts(vm: &mut Vm, id: &str, text: &str, newline: bool) -> Result<(), CmdError> {
    if id == "stdin" {
        return Err(CmdError::new("channel \"stdin\" wasn't opened for writing"));
    }
    let Some(config) = channel_config(vm, id) else {
        return Err(CmdError::new(format!(
            "can not find channel named \"{id}\""
        )));
    };
    let encoded = encode_output(text, newline, config);
    match id {
        "stdout" => vm
            .write_output_bytes(&encoded.bytes)
            .map_err(|error| write_error(id, &error))?,
        "stderr" => {
            vm.host().stdio().write_stderr(&encoded.bytes);
            vm.host().stdio().flush_stderr();
        }
        _ => match vm.channel_mut(id) {
            Some(Channel::Write { file, .. }) => file
                .write_all(&encoded.bytes)
                .map_err(|error| write_error(id, &error))?,
            Some(Channel::Read { .. }) => {
                return Err(CmdError::new(format!(
                    "channel \"{id}\" wasn't opened for writing"
                )));
            }
            None => {
                return Err(CmdError::new(format!(
                    "can not find channel named \"{id}\""
                )));
            }
        },
    }
    if let Some(error) = encoded.error {
        return Err(channel_output_error(id, error));
    }
    Ok(())
}

fn write_error(id: &str, error: &std::io::Error) -> CmdError {
    CmdError::new(format!("error writing \"{id}\": {error}"))
}

#[cfg(test)]
mod original_channel_position_tests {
    use super::*;

    #[test]
    fn original_channel_negative_target_refuses_before_position_and_retains_opened_path() {
        // naming.numeric.io-debug-public-conversion-and-side-effect-order
        // docs/design/analysis/name-resolution-proofs/numeric-io-debug-public-conversion-and-side-effect-order.md
        // Read backend state only; no external worker or platform error chronology.
        let version = tcl_dialect::TclVersion::V8_6;
        let access = resolve_open_access_mode(version, "r").unwrap();
        let config =
            ChannelConfig::for_open_access(version, tcl_platform::SystemEncoding::Utf8, access);
        let mut channel = Channel::Read {
            data: b"abcdef".to_vec(),
            pos: 3,
            config,
            path: Some(std::rc::Rc::from(b"original-opened-path".as_slice())),
        };
        let rejected = channel.seek(-1, 0).unwrap_err();
        assert_eq!(rejected.kind(), std::io::ErrorKind::InvalidInput);
        let Channel::Read { pos, .. } = &channel else {
            unreachable!()
        };
        assert_eq!(*pos, 3);
        assert_eq!(channel.path().unwrap().as_ref(), b"original-opened-path");
        channel.seek(-1, 1).unwrap();
        let Channel::Read { pos, .. } = &channel else {
            unreachable!()
        };
        assert_eq!(*pos, 2);
        channel.seek(2, 2).unwrap();
        let Channel::Read { pos, .. } = &channel else {
            unreachable!()
        };
        assert_eq!(*pos, 8);
    }
}
