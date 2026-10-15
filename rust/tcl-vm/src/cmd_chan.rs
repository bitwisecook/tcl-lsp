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

use crate::command::{completion_from_tcl_error, err_with_code};
use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// An open file channel.
pub(crate) enum Channel {
    /// A readable channel: whole-file contents with a byte cursor.
    Read {
        data: Vec<u8>,
        pos: usize,
        config: ChannelConfig,
    },
    /// A writable channel over the file itself; `readable` for the read-write
    /// modes (`r+`, `w+`, `a+`, `RDWR`), whose reads and writes share the
    /// file offset.
    Write {
        file: std::fs::File,
        readable: bool,
        eof: bool,
        config: ChannelConfig,
    },
}

impl Channel {
    fn config(&self) -> ChannelConfig {
        match self {
            Self::Read { config, .. } | Self::Write { config, .. } => *config,
        }
    }

    fn set_config(&mut self, replacement: ChannelConfig) {
        match self {
            Self::Read { config, .. } | Self::Write { config, .. } => *config = replacement,
        }
    }
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register("open", cmd_open);
    vm.register("close", cmd_close);
    vm.register("gets", cmd_gets);
    vm.register("read", cmd_read);
    vm.register("eof", cmd_eof);
    vm.register("flush", cmd_flush);
    vm.register("seek", cmd_seek);
    vm.register("tell", cmd_tell);
    vm.register("fconfigure", cmd_fconfigure);
    vm.register("fblocked", |_vm, _args| ok(Value::int(0)));
}

/// `open fileName ?access?` — supports the common access strings
/// (`r r+ w w+ a a+`). Pipes (`|cmd`) and serial ports are unsupported.
fn cmd_open(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (name, mode) = match args {
        [n] => (n.to_str().to_string(), "r".to_string()),
        [n, m] => (n.to_str().to_string(), m.to_str().to_string()),
        // The permissions argument is accepted but ignored (umask applies).
        [n, m, _perm] => (n.to_str().to_string(), m.to_str().to_string()),
        _ => return err("wrong # args: should be \"open fileName ?access? ?permissions?\""),
    };
    if name.starts_with('|') {
        return err("command pipelines are not supported");
    }
    let m = mode.trim();
    let version = vm.runtime_version();
    let access = match resolve_open_access_mode(version, m) {
        Ok(access) => access,
        Err(error) => return vm.cmd_error(error),
    };
    let config = ChannelConfig::for_open_access(version, vm.system_encoding(), access);

    if access.is_readable() && !access.is_writable() {
        match std::fs::read(&name) {
            Ok(data) => {
                let id = vm.add_channel(Channel::Read {
                    data,
                    pos: 0,
                    config,
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
                let id = vm.add_channel(Channel::Write {
                    file,
                    readable: access.is_readable(),
                    eof: false,
                    config,
                });
                ok(Value::string(id))
            }
            Err(e) => err(format!("couldn't open \"{name}\": {}", io_reason(&e))),
        }
    }
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
        return err("wrong # args: should be \"close channelId\"");
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
        _ => return err("wrong # args: should be \"gets channelId ?varName?\""),
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
    let mut nonewline = false;
    let mut rest = args;
    if let Some(first) = rest.first()
        && &*first.to_str() == "-nonewline"
    {
        nonewline = true;
        rest = &rest[1..];
    }
    let (id, count) = match rest {
        [c] => (c.to_str().to_string(), None),
        [c, n] => (c.to_str().to_string(), Some(n)),
        _ => return err("wrong # args: should be \"read channelId ?numChars?\""),
    };
    // C finds a readable channel before reading the count.
    let readable = match vm.channel(&id) {
        Some(Channel::Read { .. } | Channel::Write { readable: true, .. }) => true,
        Some(Channel::Write { .. }) => false,
        None if id == "stdin" => true,
        None if is_std(&id) => false,
        None => return err(format!("can not find channel named \"{id}\"")),
    };
    if !readable {
        return not_readable(&id);
    }
    let count = match count {
        None => None,
        Some(n) => match n.as_int().ok().and_then(|n| usize::try_from(n).ok()) {
            Some(n) => Some(n),
            None => {
                return err_with_code(
                    format!("expected non-negative integer but got \"{}\"", n.to_str()),
                    "TCL VALUE NUMBER",
                );
            }
        },
    };
    let bytes = match vm.channel_mut(&id) {
        Some(Channel::Read { data, pos, .. }) => {
            let start = *pos;
            let end = count.map_or(data.len(), |n| start.saturating_add(n).min(data.len()));
            *pos = end;
            data[start..end].to_vec()
        }
        Some(Channel::Write { file, eof, .. }) => match read_file(file, eof, count) {
            Ok(bytes) => bytes,
            Err(e) => return err(format!("error reading \"{id}\": {}", io_reason(&e))),
        },
        None => Vec::new(),
    };
    let mut bytes = &bytes[..];
    if nonewline && bytes.last() == Some(&b'\n') {
        bytes = &bytes[..bytes.len() - 1];
    }
    ok(Value::string(String::from_utf8_lossy(bytes).into_owned()))
}

fn cmd_eof(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [chan] = args else {
        return err("wrong # args: should be \"eof channelId\"");
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
        return err("wrong # args: should be \"flush channelId\"");
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
        return err("wrong # args: should be \"tell channelId\"");
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
    let (id, offset, origin) = match args {
        [c, o] => (c.to_str().to_string(), o, None),
        [c, o, w] => (c.to_str().to_string(), o, Some(w)),
        _ => return err("wrong # args: should be \"seek channelId offset ?origin?\""),
    };
    // C finds the channel, then reads the offset wide, then the origin.
    if vm.channel(&id).is_none() && !is_std(&id) {
        return err(format!("can not find channel named \"{id}\""));
    }
    let offset = match offset.as_int() {
        Ok(offset) => offset,
        Err(error) => return completion_from_tcl_error(error),
    };
    let origin = match origin.map(|w| SEEK_ORIGINS.index_of_str(&w.to_str())) {
        None => 0,
        Some(Ok(index)) => index,
        Some(Err(error)) => return vm.cmd_error(error),
    };
    match vm.channel_mut(&id) {
        Some(Channel::Read { data, pos, .. }) => {
            let base = match origin {
                1 => i64::try_from(*pos).unwrap_or(0),
                2 => i64::try_from(data.len()).unwrap_or(0),
                _ => 0,
            };
            let np = base
                .saturating_add(offset)
                .clamp(0, i64::try_from(data.len()).unwrap_or(i64::MAX));
            *pos = usize::try_from(np).unwrap_or(0);
        }
        Some(Channel::Write { file, eof, .. }) => {
            let from = match origin {
                1 => SeekFrom::Current(offset),
                2 => SeekFrom::End(offset),
                _ => SeekFrom::Start(u64::try_from(offset).unwrap_or(0)),
            };
            if let Err(e) = file.seek(from) {
                return err(format!("error during seek on \"{id}\": {}", io_reason(&e)));
            }
            *eof = false;
        }
        None => {}
    }
    ok(Value::empty())
}

/// `fconfigure channelId ?option ?value?? ...` — retain the options that
/// affect byte output; conventional non-output options remain accepted.
fn cmd_fconfigure(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((chan, rest)) = args.split_first() else {
        return err("wrong # args: should be \"fconfigure channelId ?-option value ...?\"");
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
        _ => err("wrong # args: should be \"fconfigure channelId ?-option value ...?\""),
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
