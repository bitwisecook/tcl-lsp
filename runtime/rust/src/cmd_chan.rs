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

//! Channels (M2 / L2) — `open`/`close`/`read`/`gets`/`puts`/`flush`/`eof`/
//! `seek`/`tell`/`fconfigure`/`fblocked`. C refs `tclIO.c`/`tclIOCmd.c`.
//!
//! `stdout`/`stderr` go through the host's [`StdIo`](tcl_platform::StdIo)
//! capability (so the browser routes them to a console import). File channels
//! (`open` → `fileN` ids backed by a buffered reader or a write/append handle)
//! still use `std::fs` directly — the streaming channel layer over the host
//! (handle table + buffering + encoding/EOL) is the deferred net-new piece
//! that needs both the per-interp channel state and a streaming host I/O seam.
//! `fconfigure` accepts and ignores the translation/encoding/buffering options
//! (UTF-8 internal; no CRLF translation on Unix) so library channel setup
//! succeeds.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Cursor, Read, Seek, Write};

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// A seekable read source for a channel: a real file (native) or an in-memory
/// buffer (the host-filesystem read path used where there is no real filesystem,
/// e.g. the embedded-stdlib VFS on `wasm32-wasip1`). `BufReader<File>` and
/// `Cursor<Vec<u8>>` both satisfy it, so `read`/`gets`/`seek`/`tell` are uniform.
pub trait ReadSeek: BufRead + Seek {}
impl<T: BufRead + Seek> ReadSeek for T {}

/// One open channel: a buffered reader (read modes) and/or a writable handle.
pub struct ChanState {
    reader: Option<Box<dyn ReadSeek>>,
    writer: Option<File>,
    eof: bool,
}

/// The interpreter's channel table (`fileN` → state) + id counter.
#[derive(Default)]
pub struct ChannelTable {
    map: BTreeMap<Vec<u8>, ChanState>,
    next: usize,
}

impl ChannelTable {
    pub(crate) fn names(&self) -> Vec<Vec<u8>> {
        self.map.keys().cloned().collect()
    }

    /// Open `path` for `mode` (`r`/`w`/`a`/`r+`/`w+`/`a+`), returning the id.
    pub(crate) fn open(&mut self, path: &str, mode: &[u8]) -> std::io::Result<Vec<u8>> {
        let read = mode.first() == Some(&b'r') || mode.contains(&b'+');
        let write =
            mode.first() == Some(&b'w') || mode.first() == Some(&b'a') || mode.contains(&b'+');
        let append = mode.first() == Some(&b'a');
        let truncate = mode.first() == Some(&b'w');
        let create = write;
        let mut opts = OpenOptions::new();
        opts.read(read || !write)
            .write(write)
            .append(append)
            .truncate(truncate && !append)
            .create(create);
        let file = opts.open(path)?;
        let state = if write {
            ChanState {
                reader: None,
                writer: Some(file),
                eof: false,
            }
        } else {
            ChanState {
                reader: Some(Box::new(BufReader::new(file))),
                writer: None,
                eof: false,
            }
        };
        Ok(self.insert(state))
    }

    /// Register an already-built channel state, returning its `fileN` id.
    fn insert(&mut self, state: ChanState) -> Vec<u8> {
        self.next += 1;
        let id = format!("file{}", self.next).into_bytes();
        self.map.insert(id.clone(), state);
        id
    }

    /// Register a read-only channel over an in-memory buffer (a file read whole
    /// from the host filesystem capability — the VFS read path).
    fn open_mem(&mut self, bytes: Vec<u8>) -> Vec<u8> {
        self.insert(ChanState {
            reader: Some(Box::new(Cursor::new(bytes))),
            writer: None,
            eof: false,
        })
    }
}

/// Register the channel commands.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"open", open_cmd);
    interp.register_builtin(b"close", close_cmd);
    interp.register_builtin(b"read", read_cmd);
    interp.register_builtin(b"gets", gets_cmd);
    interp.register_builtin(b"puts", puts_cmd);
    interp.register_builtin(b"flush", flush_cmd);
    interp.register_builtin(b"eof", eof_cmd);
    interp.register_builtin(b"fconfigure", fconfigure_cmd);
    interp.register_builtin(b"fblocked", fblocked_cmd);
    interp.register_builtin(b"seek", seek_cmd);
    interp.register_builtin(b"tell", tell_cmd);
    interp.register_builtin(b"chan", chan_cmd);
}

/// `chan subcommand ?arg ...?` — the core channel ensemble. Its subcommand set
/// (and their order) is C's, so the "unknown or ambiguous subcommand" error and
/// unique-prefix resolution match (`chan pu` is ambiguous between `push`/`puts`).
/// The subcommands the embedded runtime backs forward to the corresponding
/// channel command (the resolved subcommand word sits in `argv[0]`, which those
/// handlers ignore); the event-driven / reflected / stacked-channel ones it does
/// not provide report that.
fn chan_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    const SUBS: &[&[u8]] = &[
        b"blocked",
        b"close",
        b"configure",
        b"copy",
        b"create",
        b"eof",
        b"event",
        b"flush",
        b"gets",
        b"isbinary",
        b"names",
        b"pending",
        b"pipe",
        b"pop",
        b"postevent",
        b"push",
        b"puts",
        b"read",
        b"seek",
        b"tell",
        b"truncate",
    ];
    if argv.len() < 2 {
        return interp.wrong_args(b"chan subcommand ?arg ...?");
    }
    let sub = obj_bytes(argv[1]);
    let names: Vec<Vec<u8>> = SUBS.iter().map(|s| s.to_vec()).collect();
    let Some(idx) = tcl_cmd_core::ensemble::resolve_subcommand(&names, &sub, true) else {
        let mut m = b"unknown or ambiguous subcommand \"".to_vec();
        m.extend_from_slice(&sub);
        m.extend_from_slice(b"\": must be ");
        m.extend_from_slice(&tcl_cmd_core::ensemble::subcommand_choices(&names));
        return interp.set_error(&m);
    };
    // `argv[1..]` is `subcommand args…`; each target reads its arguments from
    // `argv[1..]` and uses `argv[0]` (the subcommand word) only for error text.
    let rest = &argv[1..];
    match SUBS[idx] {
        b"blocked" => fblocked_cmd(interp, rest),
        b"close" => close_cmd(interp, rest),
        b"configure" => fconfigure_cmd(interp, rest),
        b"eof" => eof_cmd(interp, rest),
        b"flush" => flush_cmd(interp, rest),
        b"gets" => gets_cmd(interp, rest),
        b"puts" => puts_cmd(interp, rest),
        b"read" => read_cmd(interp, rest),
        b"seek" => seek_cmd(interp, rest),
        b"tell" => tell_cmd(interp, rest),
        other => {
            let mut m = b"chan ".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b" is not supported under the WASM runtime");
            interp.set_error(&m)
        }
    }
}

fn no_channel(interp: &mut Interp, id: &[u8]) -> Code {
    let mut m = b"can not find channel named \"".to_vec();
    m.extend_from_slice(id);
    m.push(b'"');
    interp.set_error(&m)
}

/// A channel operation's outcome, produced *inside* the `channels` borrow scope
/// and translated to a result/error *after* the borrow drops — the `RefMut`
/// guard holds a shared borrow of `interp`, so its methods can't be called while
/// the channel state is borrowed.
enum ChanOp {
    Bytes(Vec<u8>),
    NoChannel,
    WriteOnly,
    Io(std::io::Error),
}

/// Translate a [`ChanOp`] into a completion code (the channel was addressed by
/// `id`), after the `channels` borrow has been released.
fn finish_chan(interp: &mut Interp, id: &[u8], op: ChanOp) -> Code {
    match op {
        ChanOp::Bytes(b) => {
            interp.set_result_bytes(&b);
            Code::Ok
        }
        ChanOp::NoChannel => no_channel(interp, id),
        ChanOp::WriteOnly => interp.set_error(b"channel is write only"),
        ChanOp::Io(e) => io_error(interp, &e),
    }
}

fn io_error(interp: &mut Interp, e: &std::io::Error) -> Code {
    use std::io::ErrorKind;
    interp.set_error(match e.kind() {
        ErrorKind::NotFound => b"no such file or directory",
        ErrorKind::PermissionDenied => b"permission denied",
        _ => b"I/O error",
    })
}

fn open_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 4 {
        return interp.wrong_args(b"open fileName ?access? ?permissions?");
    }
    let path = obj_bytes(argv[1]);
    let mode = argv
        .get(2)
        .map(|&a| obj_bytes(a))
        .unwrap_or_else(|| b"r".to_vec());
    let Ok(path_s) = core::str::from_utf8(&path) else {
        return interp.set_error(b"invalid file name");
    };
    let opened = interp.channels.borrow_mut().open(path_s, &mode);
    match opened {
        Ok(id) => {
            interp.set_result_bytes(&id);
            Code::Ok
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // No real file. On a host with no native filesystem (wasm), the file
            // may live in the host filesystem capability (the embedded-stdlib
            // VFS). For read modes, read it whole and back the channel with that
            // buffer. Native is unaffected — `std::fs` succeeded there, or the
            // file genuinely does not exist (the VFS read then fails too).
            let read_only =
                mode.first() != Some(&b'w') && mode.first() != Some(&b'a') && !mode.contains(&b'+');
            let vfs_bytes = if read_only {
                interp
                    .host()
                    .filesystem()
                    .and_then(|fs| fs.read(path_s).ok())
            } else {
                None
            };
            match vfs_bytes {
                Some(bytes) => {
                    let id = interp.channels.borrow_mut().open_mem(bytes);
                    interp.set_result_bytes(&id);
                    Code::Ok
                }
                None => {
                    let mut m = b"couldn't open \"".to_vec();
                    m.extend_from_slice(&path);
                    m.extend_from_slice(b"\": no such file or directory");
                    interp.set_error(&m)
                }
            }
        }
        Err(e) => io_error(interp, &e),
    }
}

fn close_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"close channelId");
    }
    let id = obj_bytes(argv[1]);
    let removed = interp.channels.borrow_mut().map.remove(&id);
    if let Some(mut st) = removed {
        if let Some(w) = st.writer.as_mut() {
            let _ = w.flush();
        }
        interp.set_result_bytes(b"");
        Code::Ok
    } else {
        no_channel(interp, &id)
    }
}

fn read_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // read ?-nonewline? channelId   |   read channelId ?numChars?
    let (nonewline, id_idx) = if argv.get(1).map(|&a| obj_bytes(a)) == Some(b"-nonewline".to_vec())
    {
        (true, 2)
    } else {
        (false, 1)
    };
    let Some(&id_obj) = argv.get(id_idx) else {
        return interp.wrong_args(b"read ?-nonewline? channelId");
    };
    let id = obj_bytes(id_obj);
    let nchars = argv.get(id_idx + 1).and_then(|&a| {
        core::str::from_utf8(&obj_bytes(a))
            .ok()
            .and_then(|s| s.trim().parse::<usize>().ok())
    });
    let op = {
        let mut channels = interp.channels.borrow_mut();
        match channels.map.get_mut(&id) {
            None => ChanOp::NoChannel,
            Some(st) => match st.reader.as_mut() {
                None => ChanOp::WriteOnly,
                Some(reader) => {
                    let mut buf = Vec::new();
                    let res = match nchars {
                        Some(n) => reader.take(n as u64).read_to_end(&mut buf),
                        None => reader.read_to_end(&mut buf),
                    };
                    match res {
                        Err(e) => ChanOp::Io(e),
                        Ok(_) => {
                            st.eof = nchars.is_none() || buf.is_empty();
                            if nonewline && buf.last() == Some(&b'\n') {
                                buf.pop();
                            }
                            ChanOp::Bytes(buf)
                        }
                    }
                }
            },
        }
    };
    finish_chan(interp, &id, op)
}

fn gets_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return interp.wrong_args(b"gets channelId ?varName?");
    }
    let id = obj_bytes(argv[1]);
    // Read the line inside the borrow scope; `n == usize::MAX` flags EOF/no-line.
    enum Gets {
        Line(Vec<u8>, usize),
        NoChannel,
        WriteOnly,
        Io(std::io::Error),
    }
    let g = {
        let mut channels = interp.channels.borrow_mut();
        match channels.map.get_mut(&id) {
            None => Gets::NoChannel,
            Some(st) => match st.reader.as_mut() {
                None => Gets::WriteOnly,
                Some(reader) => {
                    let mut line = Vec::new();
                    match reader.read_until(b'\n', &mut line) {
                        Err(e) => Gets::Io(e),
                        Ok(n) => {
                            if n == 0 {
                                st.eof = true;
                            }
                            if line.last() == Some(&b'\n') {
                                line.pop();
                                if line.last() == Some(&b'\r') {
                                    line.pop();
                                }
                            }
                            Gets::Line(line, n)
                        }
                    }
                }
            },
        }
    };
    let (line, n) = match g {
        Gets::NoChannel => return no_channel(interp, &id),
        Gets::WriteOnly => return interp.set_error(b"channel is write only"),
        Gets::Io(e) => return io_error(interp, &e),
        Gets::Line(line, n) => (line, n),
    };
    if argv.len() == 3 {
        // gets chan var → set var to the line, return its length (or -1 at EOF).
        let var = obj_bytes(argv[2]);
        if let Some(c) = interp.const_write_check(&var) {
            return c;
        }
        let o = crate::interp::new_string(&line);
        if let Err(e) = interp.var_set(&var, o) {
            crate::interp::drop_fresh(o);
            return crate::builtins::var_error(interp, &var, e);
        }
        let len = if n == 0 { -1 } else { line.len() as i64 };
        interp.set_result_bytes(len.to_string().as_bytes());
    } else {
        interp.set_result_bytes(&line);
    }
    Code::Ok
}

/// `puts ?-nonewline? ?channelId? string` — to stdout/stderr or a file channel.
pub(crate) fn puts_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let usage = b"puts ?-nonewline? ?channelId? string";
    let mut rest = &argv[1..];
    let mut newline = true;
    if rest.first().map(|&a| obj_bytes(a)) == Some(b"-nonewline".to_vec()) {
        newline = false;
        rest = &rest[1..];
    }
    let (chan, string) = match rest {
        [s] => (b"stdout".to_vec(), obj_bytes(*s)),
        [ch, s] => (obj_bytes(*ch), obj_bytes(*s)),
        _ => return interp.wrong_args(usage),
    };
    // `None` ⇒ no such channel; `Some(Err)` ⇒ write failed. The standard sinks
    // go through the host's StdIo capability (the browser routes them to a
    // console import); file channels write their `File` directly (the streaming
    // channel layer over the host is the deferred net-new piece).
    let result: Option<std::io::Result<()>> = match chan.as_slice() {
        b"stdout" => {
            write_std(interp, &string, newline, false);
            Some(Ok(()))
        }
        b"stderr" => {
            write_std(interp, &string, newline, true);
            Some(Ok(()))
        }
        _ => {
            let mut channels = interp.channels.borrow_mut();
            channels
                .map
                .get_mut(&chan)
                .and_then(|s| s.writer.as_mut())
                .map(|w| write_to(w, &string, newline))
        }
    };
    match result {
        None => no_channel(interp, &chan),
        Some(Err(_)) => interp.set_error(b"error writing to channel"),
        Some(Ok(())) => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
    }
}

fn write_to(w: &mut impl Write, bytes: &[u8], newline: bool) -> std::io::Result<()> {
    w.write_all(bytes)?;
    if newline {
        w.write_all(b"\n")?;
    }
    Ok(())
}

/// `puts` to a standard sink (`stdout`/`stderr`) via the host's StdIo capability.
/// Infallible at this layer — a console sink swallows write errors (the prior
/// `std::io::stdout()` path's `EPIPE` was likewise effectively unobserved).
fn write_std(interp: &Interp, bytes: &[u8], newline: bool, err: bool) {
    let mut buf = bytes.to_vec();
    if newline {
        buf.push(b'\n');
    }
    let host = interp.host();
    let stdio = host.stdio();
    if err {
        stdio.write_stderr(&buf);
    } else {
        stdio.write_stdout(&buf);
    }
}

fn flush_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"flush channelId");
    }
    let id = obj_bytes(argv[1]);
    match id.as_slice() {
        b"stdout" => interp.host().stdio().flush_stdout(),
        b"stderr" => interp.host().stdio().flush_stderr(),
        _ => {
            let found = {
                let mut channels = interp.channels.borrow_mut();
                match channels.map.get_mut(&id) {
                    Some(st) => {
                        if let Some(w) = st.writer.as_mut() {
                            let _ = w.flush();
                        }
                        true
                    }
                    None => false,
                }
            };
            if !found {
                return no_channel(interp, &id);
            }
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn eof_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"eof channelId");
    }
    let id = obj_bytes(argv[1]);
    let eof = interp.channels.borrow().map.get(&id).map(|st| st.eof);
    match eof {
        Some(eof) => {
            interp.set_result_bytes(if eof { b"1" } else { b"0" });
            Code::Ok
        }
        None => no_channel(interp, &id),
    }
}

/// `fconfigure channelId ?option ?value? ...?` — accept + ignore the standard
/// options (no CRLF translation, UTF-8 encoding); report queried options blank.
fn fconfigure_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"fconfigure channelId ?-option value ...?");
    }
    let id = obj_bytes(argv[1]);
    if id != b"stdout"
        && id != b"stderr"
        && id != b"stdin"
        && !interp.channels.borrow().map.contains_key(&id)
    {
        return no_channel(interp, &id);
    }
    // A single-option query returns a (blank) value; sets are accepted silently.
    interp.set_result_bytes(b"");
    Code::Ok
}

fn fblocked_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"fblocked channelId");
    }
    interp.set_result_bytes(b"0");
    Code::Ok
}

/// `seek`'s origin word, in C table order (`originOptions[]`, `tclIOCmd.c`):
/// `Tcl_GetIndexFromObj(…, "origin", 0)`, so `s`/`c`/`e` abbreviate, the empty
/// word — a prefix of all three — is `ambiguous origin ""`, and the offending
/// word is quoted in the message.
const SEEK_ORIGINS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("origin", &[b"start", b"current", b"end"]);

fn seek_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use std::io::SeekFrom;
    if argv.len() < 3 || argv.len() > 4 {
        return interp.wrong_args(b"seek channelId offset ?origin?");
    }
    let id = obj_bytes(argv[1]);
    let offset: i64 = core::str::from_utf8(&obj_bytes(argv[2]))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let origin = match argv.get(3) {
        None => 0,
        Some(&a) => match SEEK_ORIGINS.index_of(&obj_bytes(a)) {
            Ok(i) => i,
            Err(m) => return interp.set_error(&m),
        },
    };
    let from = match origin {
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => SeekFrom::Start(offset.max(0) as u64),
    };
    let op = {
        let mut channels = interp.channels.borrow_mut();
        match channels.map.get_mut(&id) {
            None => ChanOp::NoChannel,
            Some(st) => {
                let r = if let Some(rd) = st.reader.as_mut() {
                    rd.seek(from)
                } else if let Some(w) = st.writer.as_mut() {
                    w.seek(from)
                } else {
                    Ok(0)
                };
                match r {
                    Ok(_) => {
                        st.eof = false;
                        ChanOp::Bytes(Vec::new())
                    }
                    Err(e) => ChanOp::Io(e),
                }
            }
        }
    };
    finish_chan(interp, &id, op)
}

fn tell_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"tell channelId");
    }
    let id = obj_bytes(argv[1]);
    let pos = {
        let mut channels = interp.channels.borrow_mut();
        match channels.map.get_mut(&id) {
            None => None,
            Some(st) => Some(if let Some(rd) = st.reader.as_mut() {
                rd.stream_position().unwrap_or(0)
            } else if let Some(w) = st.writer.as_mut() {
                w.stream_position().unwrap_or(0)
            } else {
                0
            }),
        }
    };
    match pos {
        Some(pos) => {
            interp.set_result_bytes(pos.to_string().as_bytes());
            Code::Ok
        }
        None => no_channel(interp, &id),
    }
}

#[cfg(test)]
mod tests {
    use crate::interp::{Code, Interp};

    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    #[test]
    fn chan_ensemble_forwards_and_reports() {
        let mut i = Interp::new();
        let path = std::env::temp_dir().join(format!("tclrt_chan_ens_{}.txt", std::process::id()));
        let p = path.display();
        // Supported subcommands forward to the channel commands.
        ok(&mut i, format!("set f [open {p} w]").as_bytes());
        ok(&mut i, b"chan puts $f {via chan}");
        ok(&mut i, b"chan close $f");
        ok(&mut i, format!("set f [open {p} r]").as_bytes());
        assert_eq!(ok(&mut i, b"chan gets $f"), b"via chan");
        assert_eq!(ok(&mut i, b"chan eof $f"), b"0");
        ok(&mut i, b"chan close $f");
        // Unknown subcommand → the full C error + subcommand list.
        assert_eq!(i.eval_str(b"chan badcmd"), Code::Error);
        assert!(
            i.result_bytes().starts_with(
                b"unknown or ambiguous subcommand \"badcmd\": must be blocked, close,"
            ),
            "{:?}",
            String::from_utf8_lossy(&i.result_bytes())
        );
        // `pu` is an ambiguous prefix (push / puts).
        assert_eq!(i.eval_str(b"chan pu"), Code::Error);
        assert!(i
            .result_bytes()
            .starts_with(b"unknown or ambiguous subcommand \"pu\""));
        // A recognised-but-unbacked subcommand reports rather than mis-dispatching.
        assert_eq!(i.eval_str(b"chan pipe"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"chan pipe is not supported under the WASM runtime"
        );
        assert_eq!(i.eval_str(b"chan"), Code::Error);
        let _ = std::fs::remove_file(&path);
    }

    /// Issue #1607: `seek`'s origin word is a `Tcl_GetIndexFromObj(…,
    /// "origin", 0)` table (`originOptions[]`, `tclIOCmd.c`). This matched
    /// exactly and left the offending word out of the message entirely
    /// (`bad origin: must be …`); C quotes it, abbreviates `s`/`c`/`e`, and
    /// words the empty origin — a prefix of all three — `ambiguous`.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   seek $f 0 x  -> bad origin "x": must be start, current, or end
    ///   seek $f 0 {} -> ambiguous origin "": must be start, current, or end
    ///   seek $f 0 e; tell $f -> the file size
    ///   chan seek $f 0 x -> bad origin "x": must be start, current, or end
    #[test]
    fn seek_origin_resolves_like_tcl_get_index_from_obj() {
        const MUST: &str = "must be start, current, or end";
        let mut i = Interp::new();
        let path = std::env::temp_dir().join(format!("tclrt_seek_{}.txt", std::process::id()));
        let p = path.display();
        ok(&mut i, format!("set f [open {p} w]").as_bytes());
        ok(&mut i, b"puts -nonewline $f abcdef");
        ok(&mut i, b"close $f");
        ok(&mut i, format!("set f [open {p} r]").as_bytes());
        assert_eq!(i.eval_str(b"seek $f 0 x"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("bad origin \"x\": {MUST}").as_bytes()
        );
        assert_eq!(i.eval_str(b"seek $f 0 {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("ambiguous origin \"\": {MUST}").as_bytes()
        );
        assert_eq!(i.eval_str(b"chan seek $f 0 x"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("bad origin \"x\": {MUST}").as_bytes()
        );
        // Abbreviations resolve.
        ok(&mut i, b"seek $f 0 e");
        assert_eq!(ok(&mut i, b"tell $f"), b"6");
        ok(&mut i, b"seek $f 2 s");
        assert_eq!(ok(&mut i, b"tell $f"), b"2");
        ok(&mut i, b"seek $f 1 c");
        assert_eq!(ok(&mut i, b"tell $f"), b"3");
        ok(&mut i, b"close $f");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn open_write_read_gets_eof() {
        let mut i = Interp::new();
        let path = std::env::temp_dir().join(format!("tclrt_chan_{}.txt", std::process::id()));
        let p = path.display();
        ok(&mut i, format!("set f [open {p} w]").as_bytes());
        ok(&mut i, b"puts $f {line one}");
        ok(&mut i, b"puts -nonewline $f partial");
        ok(&mut i, b"close $f");
        // read back: gets the first line, then read the rest.
        ok(&mut i, format!("set f [open {p} r]").as_bytes());
        assert_eq!(ok(&mut i, b"gets $f"), b"line one");
        assert_eq!(ok(&mut i, b"eof $f"), b"0");
        assert_eq!(ok(&mut i, b"read $f"), b"partial");
        assert_eq!(ok(&mut i, b"eof $f"), b"1");
        ok(&mut i, b"close $f");
        // gets with a var returns the line length (or -1 at eof).
        ok(&mut i, format!("set f [open {p} r]").as_bytes());
        assert_eq!(ok(&mut i, b"gets $f line"), b"8");
        assert_eq!(ok(&mut i, b"set line"), b"line one");
        assert_eq!(ok(&mut i, b"gets $f line"), b"7"); // "partial"
        assert_eq!(ok(&mut i, b"gets $f line"), b"-1"); // eof
        ok(&mut i, b"close $f");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn unknown_channel_errors() {
        let mut i = Interp::new();
        assert_eq!(i.eval_str(b"gets nosuchchan"), Code::Error);
        assert_eq!(i.eval_str(b"close bogus"), Code::Error);
    }
}
