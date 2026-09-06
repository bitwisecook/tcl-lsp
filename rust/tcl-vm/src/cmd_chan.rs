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
//! preload the file contents (test inputs are small); write/append channels
//! hold an `std::fs::File`. The predefined `stdin`/`stdout`/`stderr` are not
//! stored in the channel table — they are handled by name: `stdout` follows the
//! VM's captured output, `stderr` the process stderr, and `stdin` is always at
//! end-of-file (non-interactive).

use std::fs::OpenOptions;
use std::io::Write as _;

use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// An open file channel.
pub(crate) enum Channel {
    /// A readable channel: whole-file contents with a byte cursor.
    Read { data: Vec<u8>, pos: usize },
    /// A writable (create/truncate or append) channel.
    Write(std::fs::File),
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
    // Normalise an access-flag list (`{RDWR CREAT}`) down to the leading mode
    // character set; the simple string forms map directly.
    let m = mode.trim();
    let read = m.starts_with('r') || m.contains('+') || m.contains("RD");
    let truncate = m.starts_with('w') || m.contains("TRUNC");
    let append = m.starts_with('a') || m.contains("APPEND");
    let writing = truncate || append || m.contains('+') || m.contains("WR");

    if read && !writing {
        match std::fs::read(&name) {
            Ok(data) => {
                let id = vm.add_channel(Channel::Read { data, pos: 0 });
                ok(Value::string(id))
            }
            Err(e) => err(format!("couldn't open \"{name}\": {}", io_reason(&e))),
        }
    } else {
        let mut opts = OpenOptions::new();
        opts.write(true).create(true);
        if append {
            opts.append(true);
        } else if truncate {
            opts.truncate(true);
        }
        if read {
            opts.read(true);
        }
        match opts.open(&name) {
            Ok(file) => {
                let id = vm.add_channel(Channel::Write(file));
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

/// `gets channelId ?varName?` — read one line (without the trailing newline).
/// With `varName`, store the line and return its length (`-1` at end-of-file);
/// without, return the line.
fn cmd_gets(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (id, var) = match args {
        [c] => (c.to_str().to_string(), None),
        [c, v] => (c.to_str().to_string(), Some(v.to_str().to_string())),
        _ => return err("wrong # args: should be \"gets channelId ?varName?\""),
    };
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
        Some(Channel::Read { data, pos }) => {
            if *pos >= data.len() {
                None
            } else {
                let start = *pos;
                let mut end = start;
                while end < data.len() && data[end] != b'\n' {
                    end += 1;
                }
                let mut line_end = end;
                // Strip a trailing CR for CRLF inputs.
                if line_end > start && data[line_end - 1] == b'\r' {
                    line_end -= 1;
                }
                let s = String::from_utf8_lossy(&data[start..line_end]).into_owned();
                *pos = if end < data.len() { end + 1 } else { end };
                Some(s)
            }
        }
        Some(Channel::Write(_)) => {
            return err(format!("channel \"{id}\" wasn't opened for reading"));
        }
        None => return err(format!("can not find channel named \"{id}\"")),
    };
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
        [c, n] => {
            let cnt = n.as_int().unwrap_or(-1);
            (c.to_str().to_string(), Some(cnt))
        }
        _ => return err("wrong # args: should be \"read channelId ?numChars?\""),
    };
    if is_std(&id) {
        return ok(Value::string(""));
    }
    match vm.channel_mut(&id) {
        Some(Channel::Read { data, pos }) => {
            let start = *pos;
            let end = match count {
                Some(n) if n >= 0 => (start + usize::try_from(n).unwrap_or(0)).min(data.len()),
                _ => data.len(),
            };
            let mut bytes = &data[start..end];
            if nonewline && bytes.last() == Some(&b'\n') {
                bytes = &bytes[..bytes.len() - 1];
            }
            let s = String::from_utf8_lossy(bytes).into_owned();
            *pos = end;
            ok(Value::string(s))
        }
        Some(Channel::Write(_)) => err(format!("channel \"{id}\" wasn't opened for reading")),
        None => err(format!("can not find channel named \"{id}\"")),
    }
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
        Some(Channel::Read { data, pos }) => ok(Value::bool(*pos >= data.len())),
        Some(Channel::Write(_)) => ok(Value::bool(false)),
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
        Some(Channel::Write(f)) => {
            let _ = f.flush();
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
        _ => ok(Value::int(-1)),
    }
}

/// `seek`'s origin word, in C table order (`originOptions[]`, `tclIOCmd.c`):
/// `Tcl_GetIndexFromObj(…, "origin", 0)`, so `s`/`c`/`e` abbreviate and the
/// empty word — a prefix of all three — is `ambiguous origin ""`.
const SEEK_ORIGINS: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("origin", &["start", "current", "end"]);

fn cmd_seek(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (id, offset, origin) = match args {
        [c, o] => (c.to_str().to_string(), o.as_int().unwrap_or(0), 0),
        [c, o, w] => {
            let index = match SEEK_ORIGINS.index_of_str(&w.to_str()) {
                Ok(i) => i,
                Err(e) => return err(e.into_message()),
            };
            (c.to_str().to_string(), o.as_int().unwrap_or(0), index)
        }
        _ => return err("wrong # args: should be \"seek channelId offset ?origin?\""),
    };
    if let Some(Channel::Read { data, pos }) = vm.channel_mut(&id) {
        let base = match origin {
            1 => i64::try_from(*pos).unwrap_or(0),
            2 => i64::try_from(data.len()).unwrap_or(0),
            _ => 0,
        };
        let np = (base + offset).clamp(0, i64::try_from(data.len()).unwrap_or(i64::MAX));
        *pos = usize::try_from(np).unwrap_or(0);
    }
    ok(Value::empty())
}

/// `fconfigure channelId ?option ?value?? ...` — channel options are accepted
/// but not modelled; getters return conventional defaults.
fn cmd_fconfigure(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((_chan, rest)) = args.split_first() else {
        return err("wrong # args: should be \"fconfigure channelId ?-option value ...?\"");
    };
    match rest {
        // Query all options: return a small conventional set.
        [] => ok(Value::string(
            "-blocking 1 -buffering full -encoding utf-8 -translation lf",
        )),
        // Query one option.
        [opt] => ok(Value::string(match &*opt.to_str() {
            "-blocking" => "1",
            "-buffering" => "full",
            "-buffersize" => "4096",
            "-encoding" => "utf-8",
            "-translation" => "lf",
            // `-eofchar` and any unmodelled option report empty.
            _ => "",
        })),
        // Setting one or more options: accepted, no-op.
        _ => ok(Value::empty()),
    }
}

/// True for a predefined standard channel name.
pub(crate) fn is_std(id: &str) -> bool {
    matches!(id, "stdin" | "stdout" | "stderr")
}

/// Write `text` (optionally with a trailing newline) to channel `id`. Used by
/// `puts`. Standard channels follow the VM's output / process stderr.
pub(crate) fn chan_puts(vm: &mut Vm, id: &str, text: &str, newline: bool) -> Result<(), String> {
    match id {
        "stdout" => {
            vm.write_output(text, newline);
            Ok(())
        }
        "stderr" => {
            eprint!("{text}");
            if newline {
                eprintln!();
            }
            Ok(())
        }
        "stdin" => Err("channel \"stdin\" wasn't opened for writing".to_string()),
        _ => match vm.channel_mut(id) {
            Some(Channel::Write(f)) => {
                let _ = f.write_all(text.as_bytes());
                if newline {
                    let _ = f.write_all(b"\n");
                }
                Ok(())
            }
            Some(Channel::Read { .. }) => {
                Err(format!("channel \"{id}\" wasn't opened for writing"))
            }
            None => Err(format!("can not find channel named \"{id}\"")),
        },
    }
}
