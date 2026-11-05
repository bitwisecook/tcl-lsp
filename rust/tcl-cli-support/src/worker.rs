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

//! Reusable CLI processes for test suites.
//!
//! A fresh `tcl` process spends seconds discovering and loading `SpecTcl` packs
//! and building registries before it reads its first argument; the verb itself
//! is usually milliseconds. Integration tests that assert on hundreds of
//! invocations therefore spend their time starting processes.
//!
//! A *worker* is the CLI binary started as `<bin> __tcl-cli-worker <dir> <slot>`.
//! It loads nothing up front, listens on loopback, and answers framed
//! `(argv, cwd) → (exit code, stdout, stderr)` requests one connection at a
//! time, so its process-wide caches warm once and serve every later request.
//!
//! [`output`] is the client: a drop-in for [`Command::output`]. Workers live in
//! numbered *slots* under a directory keyed by the binary's identity and the
//! command's environment. A slot is owned through an OS file lock, so nextest's
//! one-process-per-test runs share workers across test processes, and a
//! multi-threaded `cargo test` gets one worker per concurrently busy thread.
//! The number of CLI starts is bounded by test parallelism per distinct
//! environment, not by the number of invocations or tests.
//!
//! Fidelity is the contract. Anything a worker cannot reproduce byte-for-byte
//! runs as a real process instead:
//! - a request whose output escaped the capture (a library writing to the
//!   process streams, a panic message) — the worker reports it and the client
//!   re-runs the invocation;
//! - a worker that dies mid-request (`process::exit`, abort);
//! - `TCL_CLI_TEST_SPAWN=1`, which disables workers for bisecting.
//!
//! A reused process still continues process-wide counters a fresh one
//! restarts (the scope in an opaque `bodyKey`), so a test comparing such
//! output across invocations runs real processes.
//!
//! Each worker appends a line to `<dir>/starts.log` when it starts, and exits
//! after 15 idle seconds.

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The argv sentinel that turns the CLI binary into a worker.
pub const WORKER_ARG: &str = "__tcl-cli-worker";

/// Set to `1` to make [`output`] always start a real process.
pub const SPAWN_ENV: &str = "TCL_CLI_TEST_SPAWN";

const MAGIC: &[u8; 4] = b"TCW1";
const DEFAULT_IDLE: Duration = Duration::from_secs(15);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(300);

// ---------------------------------------------------------------- protocol

/// One invocation: argv after the program name, and the directory to run in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
}

/// How a worker answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The invocation ran in the worker with this exit code and output.
    Completed {
        code: i32,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
    /// The worker could not reproduce a fresh process; run one instead.
    Respawn(String),
}

#[cfg_attr(unix, expect(clippy::unnecessary_wraps, reason = "fallible off Unix"))]
fn os_bytes(s: &OsStr) -> io::Result<Vec<u8>> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        Ok(s.as_bytes().to_vec())
    }
    #[cfg(not(unix))]
    {
        s.to_str()
            .map(|s| s.as_bytes().to_vec())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "argument is not UTF-8"))
    }
}

#[cfg_attr(unix, expect(clippy::unnecessary_wraps, reason = "fallible off Unix"))]
fn bytes_os(b: Vec<u8>) -> io::Result<OsString> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt as _;
        Ok(OsString::from_vec(b))
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(b)
            .map(OsString::from)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "argument is not UTF-8"))
    }
}

fn put_bytes(out: &mut Vec<u8>, b: &[u8]) {
    let len = u32::try_from(b.len()).expect("frame field under 4 GiB");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(b);
}

struct Cursor<'a>(&'a [u8]);

impl Cursor<'_> {
    fn bytes(&mut self) -> io::Result<Vec<u8>> {
        let len = self.u32()? as usize;
        if self.0.len() < len {
            return Err(truncated());
        }
        let (head, tail) = self.0.split_at(len);
        self.0 = tail;
        Ok(head.to_vec())
    }

    fn u32(&mut self) -> io::Result<u32> {
        let Some((head, tail)) = self.0.split_first_chunk::<4>() else {
            return Err(truncated());
        };
        self.0 = tail;
        Ok(u32::from_le_bytes(*head))
    }
}

fn truncated() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "truncated worker frame")
}

fn write_frame(w: &mut impl Write, payload: &[u8]) -> io::Result<()> {
    let len = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame over 4 GiB"))?;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(payload)?;
    w.flush()
}

/// `Ok(None)` on a clean end of stream before a frame starts.
fn read_frame(r: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut len = [0u8; 4];
    match r.read_exact(&mut len) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let mut payload = vec![0u8; u32::from_le_bytes(len) as usize];
    r.read_exact(&mut payload)?;
    Ok(Some(payload))
}

impl Request {
    fn encode(&self) -> io::Result<Vec<u8>> {
        let mut out = Vec::new();
        put_bytes(&mut out, &os_bytes(self.cwd.as_os_str())?);
        out.extend_from_slice(
            &u32::try_from(self.args.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        for arg in &self.args {
            put_bytes(&mut out, &os_bytes(arg)?);
        }
        Ok(out)
    }

    fn decode(payload: &[u8]) -> io::Result<Self> {
        let mut c = Cursor(payload);
        let cwd = PathBuf::from(bytes_os(c.bytes()?)?);
        let n = c.u32()?;
        let args = (0..n)
            .map(|_| c.bytes().and_then(bytes_os))
            .collect::<io::Result<_>>()?;
        Ok(Self { args, cwd })
    }
}

impl Reply {
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Reply::Completed {
                code,
                stdout,
                stderr,
            } => {
                out.push(0);
                out.extend_from_slice(&code.to_le_bytes());
                put_bytes(&mut out, stdout);
                put_bytes(&mut out, stderr);
            }
            Reply::Respawn(reason) => {
                out.push(1);
                put_bytes(&mut out, reason.as_bytes());
            }
        }
        out
    }

    fn decode(payload: &[u8]) -> io::Result<Self> {
        let (&tag, rest) = payload.split_first().ok_or_else(truncated)?;
        let mut c = Cursor(rest);
        match tag {
            0 => {
                let code = c.u32()?.cast_signed();
                Ok(Reply::Completed {
                    code,
                    stdout: c.bytes()?,
                    stderr: c.bytes()?,
                })
            }
            1 => Ok(Reply::Respawn(
                String::from_utf8_lossy(&c.bytes()?).into_owned(),
            )),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unknown reply tag",
            )),
        }
    }
}

// ---------------------------------------------------------------- server

/// What a worker needs from the binary it lives in: run one invocation with
/// its streams captured.
pub trait Handler {
    /// Run `args` (argv without the program name) and return its exit code
    /// and captured output, or `None` when the run cannot be reproduced
    /// in-process (it panicked).
    fn run(&mut self, args: &[OsString]) -> Option<(i32, Vec<u8>, Vec<u8>)>;
}

impl<F: FnMut(&[OsString]) -> Option<(i32, Vec<u8>, Vec<u8>)>> Handler for F {
    fn run(&mut self, args: &[OsString]) -> Option<(i32, Vec<u8>, Vec<u8>)> {
        self(args)
    }
}

fn slot_path(dir: &Path, slot: usize, ext: &str) -> PathBuf {
    dir.join(format!("slot-{slot}.{ext}"))
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map_or(0, |m| m.len())
}

/// Serve requests forever (until idle). `args` is argv after [`WORKER_ARG`]:
/// `<dir> <slot> [idle-seconds]`. The access token is the first line of
/// stdin. Returns the process exit code.
pub fn serve(args: &[OsString], handler: &mut dyn Handler) -> u8 {
    match serve_inner(args, handler) {
        Ok(()) => 0,
        Err(e) => {
            let _ = writeln!(io::stderr(), "tcl-cli worker: {e}");
            1
        }
    }
}

fn serve_inner(args: &[OsString], handler: &mut dyn Handler) -> io::Result<()> {
    let bad = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: <dir> <slot> [idle-secs]",
        )
    };
    let dir = PathBuf::from(args.first().ok_or_else(bad)?);
    let slot: usize = args
        .get(1)
        .and_then(|s| s.to_str())
        .and_then(|s| s.parse().ok())
        .ok_or_else(bad)?;
    let idle = args
        .get(2)
        .and_then(|s| s.to_str())
        .and_then(|s| s.parse().ok())
        .map_or(DEFAULT_IDLE, Duration::from_secs);

    let mut token = String::new();
    io::stdin().lock().read_line(&mut token)?;
    let token = token.trim_end().to_owned();
    if token.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no token on stdin",
        ));
    }

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    // One line per CLI start-up in this pool, so a suite's start count can be
    // read back (`wc -l <dir>/starts.log`).
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("starts.log"))?
        .write_all(format!("{} slot {slot}\n", std::process::id()).as_bytes())?;
    let tmp = slot_path(&dir, slot, "port.tmp");
    fs::write(&tmp, format!("{port}\n"))?;
    fs::rename(&tmp, slot_path(&dir, slot, "port"))?;

    let busy = Arc::new(AtomicBool::new(false));
    let last = Arc::new(AtomicU64::new(now_secs()));
    {
        let (busy, last) = (Arc::clone(&busy), Arc::clone(&last));
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let idle_for = now_secs().saturating_sub(last.load(Ordering::Relaxed));
                if !busy.load(Ordering::Relaxed) && idle_for >= idle.as_secs() {
                    std::process::exit(0);
                }
            }
        });
    }

    let logs = [slot_path(&dir, slot, "out"), slot_path(&dir, slot, "err")];
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        busy.store(true, Ordering::Relaxed);
        let _ = session(stream, &token, &logs, handler, &last);
        last.store(now_secs(), Ordering::Relaxed);
        busy.store(false, Ordering::Relaxed);
    }
    Ok(())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn session(
    stream: TcpStream,
    token: &str,
    logs: &[PathBuf; 2],
    handler: &mut dyn Handler,
    last: &AtomicU64,
) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;
    let Some(hello) = read_frame(&mut reader)? else {
        return Ok(());
    };
    let mut expected = MAGIC.to_vec();
    expected.extend_from_slice(token.as_bytes());
    if hello != expected {
        return Ok(());
    }
    write_frame(&mut writer, MAGIC)?;
    while let Some(payload) = read_frame(&mut reader)? {
        last.store(now_secs(), Ordering::Relaxed);
        let reply = match Request::decode(&payload) {
            Ok(request) => answer(&request, logs, handler),
            Err(e) => Reply::Respawn(format!("undecodable request: {e}")),
        };
        write_frame(&mut writer, &reply.encode())?;
    }
    Ok(())
}

fn answer(request: &Request, logs: &[PathBuf; 2], handler: &mut dyn Handler) -> Reply {
    if let Err(e) = std::env::set_current_dir(&request.cwd) {
        return Reply::Respawn(format!("cannot enter {}: {e}", request.cwd.display()));
    }
    let before = logs.each_ref().map(|p| file_len(p));
    let ran = handler.run(&request.args);
    let _ = io::stdout().flush();
    let _ = io::stderr().flush();
    let after = logs.each_ref().map(|p| file_len(p));
    match ran {
        _ if before != after => Reply::Respawn("output escaped the capture".to_owned()),
        None => Reply::Respawn("the invocation panicked".to_owned()),
        Some((code, stdout, stderr)) => Reply::Completed {
            code,
            stdout,
            stderr,
        },
    }
}

// ---------------------------------------------------------------- client

/// Run `command` as [`Command::output`] would, through a pooled worker of the
/// same binary when possible.
///
/// `state_dir` holds the slot files; integration tests pass
/// `env!("CARGO_TARGET_TMPDIR")`. The command's stdin is not consulted: like
/// `output()`, the invocation sees an empty stdin.
pub fn output(command: &mut Command, state_dir: impl AsRef<Path>) -> io::Result<Output> {
    if std::env::var_os(SPAWN_ENV).is_some_and(|v| v == "1") {
        return command.output();
    }
    let Some(pool) = Pool::for_command(command, state_dir.as_ref()) else {
        return command.output();
    };
    let request = Request {
        args: command.get_args().map(OsStr::to_os_string).collect(),
        cwd: match command.get_current_dir() {
            Some(dir) => std::env::current_dir()?.join(dir),
            None => std::env::current_dir()?,
        },
    };
    let Ok(mut session) = pool.acquire() else {
        return command.output();
    };
    match session.call(&request) {
        Ok(Reply::Completed {
            code,
            stdout,
            stderr,
        }) => {
            pool.release(session);
            Ok(Output {
                status: exit_status(code),
                stdout,
                stderr,
            })
        }
        Ok(Reply::Respawn(_)) => {
            pool.release(session);
            command.output()
        }
        Err(_) => {
            pool.discard(session);
            command.output()
        }
    }
}

#[cfg(unix)]
fn exit_status(code: i32) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt as _;
    ExitStatus::from_raw((code & 0xff) << 8)
}

#[cfg(windows)]
fn exit_status(code: i32) -> ExitStatus {
    use std::os::windows::process::ExitStatusExt as _;
    ExitStatus::from_raw(code.cast_unsigned())
}

struct Session {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    // Held for the life of the session: the slot's ownership.
    _lock: File,
}

impl Session {
    fn call(&mut self, request: &Request) -> io::Result<Reply> {
        write_frame(&mut self.stream, &request.encode()?)?;
        let payload = read_frame(&mut self.reader)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "worker closed the connection")
        })?;
        Reply::decode(&payload)
    }
}

struct Pool {
    program: PathBuf,
    envs: Vec<(OsString, Option<OsString>)>,
    dir: PathBuf,
    slots: usize,
    idle: Mutex<Vec<Session>>,
    held: Mutex<usize>,
    returned: Condvar,
}

impl Pool {
    fn for_command(command: &Command, state_dir: &Path) -> Option<&'static Pool> {
        static POOLS: OnceLock<Mutex<Vec<&'static Pool>>> = OnceLock::new();
        let program = which(command.get_program())?;
        let mut envs: Vec<(OsString, Option<OsString>)> = command
            .get_envs()
            .map(|(k, v)| (k.to_os_string(), v.map(OsStr::to_os_string)))
            .collect();
        envs.sort();
        let dir = state_dir
            .join("tcl-cli-workers")
            .join(format!("{:016x}", identity(&program, &envs)?));
        let pools = POOLS.get_or_init(Mutex::default);
        let mut pools = pools.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(pool) = pools.iter().find(|p| p.dir == dir) {
            return Some(pool);
        }
        create_private_dir(&dir).ok()?;
        let slots = std::env::var("TCL_CLI_WORKER_SLOTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, usize::from))
            .max(1);
        let pool: &'static Pool = Box::leak(Box::new(Pool {
            program,
            envs,
            dir,
            slots,
            idle: Mutex::default(),
            held: Mutex::new(0),
            returned: Condvar::new(),
        }));
        pools.push(pool);
        Some(pool)
    }

    fn acquire(&self) -> io::Result<Session> {
        loop {
            if let Some(session) = self
                .idle
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .pop()
            {
                return Ok(session);
            }
            let start = std::process::id() as usize;
            for i in 0..self.slots {
                let slot = (start + i) % self.slots;
                let lock = OpenOptions::new()
                    .create(true)
                    .truncate(false)
                    .write(true)
                    .open(slot_path(&self.dir, slot, "lock"))?;
                if lock.try_lock().is_ok() {
                    return self.claimed(slot, lock);
                }
            }
            let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
            if *held > 0 {
                // A session of ours is busy on another thread; wait for it
                // rather than queueing on a lock this process already holds.
                held = self
                    .returned
                    .wait_timeout(held, Duration::from_millis(200))
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
                drop(held);
                continue;
            }
            drop(held);
            let slot = start % self.slots;
            let lock = OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(slot_path(&self.dir, slot, "lock"))?;
            lock.lock()?;
            return self.claimed(slot, lock);
        }
    }

    fn claimed(&self, slot: usize, lock: File) -> io::Result<Session> {
        let session = self.connect(slot, lock)?;
        *self.held.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        Ok(session)
    }

    fn release(&self, session: Session) {
        self.idle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(session);
        self.returned.notify_all();
    }

    fn discard(&self, session: Session) {
        drop(session);
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        *held = held.saturating_sub(1);
        self.returned.notify_all();
    }

    /// Connect to the slot's worker, starting one if none answers. The caller
    /// holds the slot lock, so nothing else touches these files meanwhile.
    fn connect(&self, slot: usize, lock: File) -> io::Result<Session> {
        let token_path = slot_path(&self.dir, slot, "token");
        if let Ok(token) = fs::read_to_string(&token_path)
            && let Some((stream, reader)) = self.handshake(slot, token.trim())
        {
            return Ok(Session {
                stream,
                reader,
                _lock: lock,
            });
        }
        let token = new_token();
        let port_path = slot_path(&self.dir, slot, "port");
        let _ = fs::remove_file(&port_path);
        write_private(&token_path, token.as_bytes())?;
        self.start_worker(slot, &token)?;
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        while Instant::now() < deadline {
            if port_path.exists()
                && let Some((stream, reader)) = self.handshake(slot, &token)
            {
                return Ok(Session {
                    stream,
                    reader,
                    _lock: lock,
                });
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "worker did not start",
        ))
    }

    fn handshake(&self, slot: usize, token: &str) -> Option<(TcpStream, BufReader<TcpStream>)> {
        let port: u16 = fs::read_to_string(slot_path(&self.dir, slot, "port"))
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).ok()?;
        stream.set_nodelay(true).ok()?;
        let mut hello = MAGIC.to_vec();
        hello.extend_from_slice(token.as_bytes());
        write_frame(&mut stream, &hello).ok()?;
        let mut reader = BufReader::new(stream.try_clone().ok()?);
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
        let ack = read_frame(&mut reader).ok()??;
        stream.set_read_timeout(None).ok()?;
        (ack == MAGIC).then_some((stream, reader))
    }

    fn start_worker(&self, slot: usize, token: &str) -> io::Result<()> {
        let out = File::create(slot_path(&self.dir, slot, "out"))?;
        let err = File::create(slot_path(&self.dir, slot, "err"))?;
        let mut command = Command::new(&self.program);
        command
            .arg(WORKER_ARG)
            .arg(&self.dir)
            .arg(slot.to_string())
            .current_dir(&self.dir)
            .stdin(Stdio::piped())
            .stdout(out)
            .stderr(err);
        for (key, value) in &self.envs {
            match value {
                Some(value) => command.env(key, value),
                None => command.env_remove(key),
            };
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            // Outlive the test process that started it: test runners kill a
            // finished test's process group.
            command.process_group(0);
        }
        let mut child = command.spawn()?;
        let mut stdin = child.stdin.take().expect("piped stdin");
        writeln!(stdin, "{token}")?;
        drop(stdin);
        // Reap it if it exits while we are still running.
        std::thread::spawn(move || child.wait());
        Ok(())
    }
}

/// Resolve the program as `Command` would, so the identity is the file run.
fn which(program: &OsStr) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.components().count() > 1 {
        return Some(std::env::current_dir().ok()?.join(path));
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(path))
        .find(|candidate| candidate.is_file())
}

/// The worker directory's key: the binary as built, the command's overrides,
/// and the inherited environment a worker would see.
fn identity(program: &Path, envs: &[(OsString, Option<OsString>)]) -> Option<u64> {
    let meta = fs::metadata(program).ok()?;
    let mut h = std::hash::DefaultHasher::new();
    program.hash(&mut h);
    meta.len().hash(&mut h);
    meta.modified().ok()?.hash(&mut h);
    envs.hash(&mut h);
    let mut inherited: Vec<(OsString, OsString)> = std::env::vars_os()
        .filter(|(k, _)| !volatile_env(k))
        .collect();
    inherited.sort();
    inherited.hash(&mut h);
    Some(h.finish())
}

/// Variables a test runner sets per test that a CLI run never reads.
fn volatile_env(key: &OsStr) -> bool {
    let key = key.to_string_lossy();
    key.starts_with("NEXTEST_") || key == "RUST_TEST_THREADS" || key == "__NEXTEST_ATTEMPT"
}

fn new_token() -> String {
    use std::hash::BuildHasher as _;
    let a = std::hash::RandomState::new().hash_one(std::process::id());
    let b = std::hash::RandomState::new().hash_one(Instant::now());
    format!("{a:016x}{b:016x}")
}

fn create_private_dir(dir: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        builder.mode(0o700);
    }
    builder.create(dir)
}

fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path)?.write_all(bytes)
}

#[cfg(test)]
mod tests {
    use super::{Reply, Request};
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn request_round_trips() {
        let request = Request {
            args: vec![
                OsString::from("diag"),
                OsString::from(""),
                OsString::from("ü {x}"),
            ],
            cwd: PathBuf::from("/tmp/a dir"),
        };
        assert_eq!(
            Request::decode(&request.encode().unwrap()).unwrap(),
            request
        );
    }

    #[test]
    fn replies_round_trip() {
        for reply in [
            Reply::Completed {
                code: 2,
                stdout: b"out\n".to_vec(),
                stderr: vec![0, 255],
            },
            Reply::Completed {
                code: -1,
                stdout: Vec::new(),
                stderr: Vec::new(),
            },
            Reply::Respawn("why".to_owned()),
        ] {
            assert_eq!(Reply::decode(&reply.encode()).unwrap(), reply);
        }
    }

    #[test]
    fn truncated_frames_are_errors() {
        let bytes = Request {
            args: vec![OsString::from("x")],
            cwd: PathBuf::from("/"),
        }
        .encode()
        .unwrap();
        assert!(Request::decode(&bytes[..bytes.len() - 1]).is_err());
        assert!(Reply::decode(&[]).is_err());
        assert!(Reply::decode(&[9]).is_err());
    }
}
