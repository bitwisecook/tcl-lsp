//! Channels (M2 / L2) — `open`/`close`/`read`/`gets`/`puts`/`flush`/`eof`/
//! `seek`/`tell`/`fconfigure`/`fblocked`. C refs `tclIO.c`/`tclIOCmd.c`.
//!
//! Host-backed via `std::fs` (native / `wasm32-wasip1`). `stdout`/`stderr` are
//! the process streams; `open` returns `fileN` ids backed by a buffered reader
//! (read modes) or a file handle (write/append). `fconfigure` accepts and
//! ignores the translation/encoding/buffering options (UTF-8 internal; no
//! CRLF translation on Unix) so the library's channel setup succeeds.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// One open channel: a buffered reader (read modes) and/or a writable handle.
pub struct ChanState {
    reader: Option<BufReader<File>>,
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
    /// Open `path` for `mode` (`r`/`w`/`a`/`r+`/`w+`/`a+`), returning the id.
    fn open(&mut self, path: &str, mode: &[u8]) -> std::io::Result<Vec<u8>> {
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
                reader: Some(BufReader::new(file)),
                writer: None,
                eof: false,
            }
        };
        self.next += 1;
        let id = format!("file{}", self.next).into_bytes();
        self.map.insert(id.clone(), state);
        Ok(id)
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
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
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
        return wrong_args(interp, b"open fileName ?access? ?permissions?");
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
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                let mut m = b"couldn't open \"".to_vec();
                m.extend_from_slice(&path);
                m.extend_from_slice(b"\": no such file or directory");
                interp.set_error(&m)
            }
            _ => io_error(interp, &e),
        },
    }
}

fn close_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return wrong_args(interp, b"close channelId");
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
        return wrong_args(interp, b"read ?-nonewline? channelId");
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
        return wrong_args(interp, b"gets channelId ?varName?");
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
        let o = crate::interp::new_string(&line);
        if interp.var_set(&var, o).is_err() {
            crate::interp::drop_fresh(o);
        }
        let len = if n == 0 { -1 } else { line.len() as i64 };
        interp.set_result_bytes(len.to_string().as_bytes());
    } else {
        interp.set_result_bytes(&line);
    }
    Code::Ok
}

/// `puts ?-nonewline? ?channelId? string` — to stdout/stderr or a file channel.
fn puts_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
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
        _ => return wrong_args(interp, usage),
    };
    // `None` ⇒ no such channel; `Some(Err)` ⇒ write failed.
    let result: Option<std::io::Result<()>> = match chan.as_slice() {
        b"stdout" => Some(write_to(&mut std::io::stdout(), &string, newline)),
        b"stderr" => Some(write_to(&mut std::io::stderr(), &string, newline)),
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

fn flush_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return wrong_args(interp, b"flush channelId");
    }
    let id = obj_bytes(argv[1]);
    match id.as_slice() {
        b"stdout" => {
            let _ = std::io::stdout().flush();
        }
        b"stderr" => {
            let _ = std::io::stderr().flush();
        }
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
        return wrong_args(interp, b"eof channelId");
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
        return wrong_args(interp, b"fconfigure channelId ?-option value ...?");
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
        return wrong_args(interp, b"fblocked channelId");
    }
    interp.set_result_bytes(b"0");
    Code::Ok
}

fn seek_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use std::io::{Seek, SeekFrom};
    if argv.len() < 3 || argv.len() > 4 {
        return wrong_args(interp, b"seek channelId offset ?origin?");
    }
    let id = obj_bytes(argv[1]);
    let offset: i64 = core::str::from_utf8(&obj_bytes(argv[2]))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let origin = argv
        .get(3)
        .map(|&a| obj_bytes(a))
        .unwrap_or_else(|| b"start".to_vec());
    let from = match origin.as_slice() {
        b"start" => SeekFrom::Start(offset.max(0) as u64),
        b"current" => SeekFrom::Current(offset),
        b"end" => SeekFrom::End(offset),
        _ => return interp.set_error(b"bad origin: must be start, current, or end"),
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
    use std::io::Seek;
    if argv.len() != 2 {
        return wrong_args(interp, b"tell channelId");
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
