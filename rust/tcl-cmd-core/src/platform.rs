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

//! Platform-backed command helpers (`exec`, `pwd`, `cd`, …).
//!
//! The portable Tcl semantics (argument parsing, the error catalogue, result
//! building) live here once, while the actual host syscalls are reached through
//! the [`tcl_platform`] capability traits and live per-target. A
//! **platform-conditional** capability (subprocess, sockets) that
//! a host lacks — every WASM target lacks subprocess — surfaces as the faithful
//! Tcl "unsupported" error rather than a panic, exactly as the capability model
//! intends.

use tcl_dialect::TclVersion;
use tcl_platform::{Host, ProcessExit, StderrMode};
use tcl_syntax::list::join_list;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;
use crate::path::PathReading;

/// Installation directories appended to `auto_path`, in Tcl lookup order,
/// with paths read as `reading`'s release reads them.
#[must_use]
pub fn auto_path_installation(
    library: Option<&[u8]>,
    executable: Option<&[u8]>,
    reading: PathReading,
) -> Vec<Vec<u8>> {
    let mut paths = Vec::new();
    if let Some(library) = library {
        paths.push(library.to_vec());
        paths.push(crate::path::dirname(library, reading).into_owned());
    }
    if let Some(executable) = executable {
        let directory = crate::path::dirname(executable, reading);
        let parent = crate::path::dirname(&directory, reading);
        paths.push(crate::path::join(&[parent.as_ref(), b"lib"], reading));
    }
    paths
}

/// Expand the supplied host home directory without guessing named user homes;
/// joined paths are read as `reading`'s release reads them.
pub fn auto_path_environment(
    paths: Vec<Vec<u8>>,
    home: Option<&[u8]>,
    reading: PathReading,
) -> Result<Vec<Vec<u8>>, CmdError> {
    let mut expanded = Vec::new();
    for path in paths {
        let selected = {
            if path == b"~" {
                home.map(<[u8]>::to_vec)
            } else if path.starts_with(b"~/") {
                home.map(|home| crate::path::join(&[home, &path[2..]], reading))
            } else if path.starts_with(b"~") {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "named home directory lookup",
                )
                .into());
            } else {
                Some(path)
            }
        };
        if let Some(path) = selected {
            expanded.push(path);
        }
    }
    Ok(expanded)
}

/// A leading `exec` switch, as the command's registry spec resolves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecSwitch {
    /// `-ignorestderr`: a child's standard error passes through instead of
    /// raising.
    IgnoreStderr,
    /// `-keepnewline`: keep the result's trailing newline.
    KeepNewline,
    /// `-encoding name` (Tcl 9.0+).
    Encoding,
    /// `--`: the next word starts the pipeline.
    EndOfSwitches,
}

/// `exec ?switches? arg ?arg ...?` — run a command and capture its standard
/// output, with one trailing newline removed unless `-keepnewline` is given.
///
/// `resolve_switch` maps each leading `-` word to its [`ExecSwitch`] under the
/// active release (the registry spec owns the table). Words after the switches
/// are passed through as the command line: pipelines, redirections and a
/// trailing `&` are not interpreted, and `-encoding`'s value is consumed but
/// not applied.
///
/// A child that exits non-zero, is killed by a signal, or (without
/// `-ignorestderr`) writes to standard error makes this raise, as
/// `TclCleanupChildren` does: the message is the captured standard output
/// followed by the standard error text, or failing that `child killed: …` /
/// `child process exited abnormally`, and the error code is
/// `CHILDSTATUS pid status` or `CHILDKILLED pid SIGNAME message`.
///
/// Subprocess execution is a *platform-conditional* capability: when the host
/// provides no [`Process`](tcl_platform::Process) (every WASM target, a sandbox),
/// this yields the faithful unsupported error instead of running — the same
/// shared body, a different host.
pub fn exec<O: ValueOps>(
    ops: &mut O,
    host: &dyn Host,
    release: TclVersion,
    resolve_switch: impl Fn(&str) -> Result<ExecSwitch, CmdError>,
    args: &[O::Value],
) -> Result<O::Value, CmdError> {
    let Some(process) = host.process() else {
        return Err(CmdError::new(
            "exec is not available: this platform provides no subprocess support",
        ));
    };
    let words: Vec<String> = args
        .iter()
        .map(|a| ops.try_as_str(a).map(|text| text.to_string()))
        .collect::<Result<_, _>>()?;
    let mut keep_newline = false;
    let mut stderr = StderrMode::Capture;
    let mut skip = 0;
    while let Some(word) = words.get(skip).filter(|word| word.starts_with('-')) {
        skip += 1;
        match resolve_switch(word)? {
            ExecSwitch::IgnoreStderr => stderr = StderrMode::Inherit,
            ExecSwitch::KeepNewline => keep_newline = true,
            ExecSwitch::Encoding if skip < words.len() => skip += 1,
            ExecSwitch::Encoding => {
                return Err(CmdError::new(if release == TclVersion::V9_0 {
                    "No value given for option -encoding."
                } else {
                    "No value given for option -encoding"
                }));
            }
            ExecSwitch::EndOfSwitches => break,
        }
    }
    if skip >= words.len() {
        return Err(CmdError::wrong_args(if release < TclVersion::V8_6 {
            "exec ?switches? arg ?arg ...?"
        } else {
            "exec ?-option ...? arg ?arg ...?"
        }));
    }
    let part_refs: Vec<&str> = words[skip..].iter().map(String::as_str).collect();
    let output = process.run(&part_refs, stderr)?;
    let (failure, error_code) = child_failure(output.pid, output.exit);
    let mut result = output.stdout;
    let raised = if !output.stderr.is_empty() {
        result.extend_from_slice(&output.stderr);
        true
    } else if let Some(failure) = &failure {
        result.extend_from_slice(failure.as_bytes());
        true
    } else {
        false
    };
    if !keep_newline && result.last() == Some(&b'\n') {
        result.pop();
    }
    if !raised {
        return Ok(ops.new_bytes(&result));
    }
    let message = String::from_utf8_lossy(&result).into_owned();
    Err(match error_code {
        Some(code) => CmdError::with_error_code(message, code),
        None => CmdError::new(message),
    })
}

/// The message and `-errorcode` `TclProcessWait` reports for a child's exit,
/// or `(None, None)` for a clean exit.
fn child_failure(pid: u32, exit: ProcessExit) -> (Option<String>, Option<String>) {
    let pid = pid.to_string();
    match exit {
        ProcessExit::Exited(0) => (None, None),
        ProcessExit::Exited(status) => (
            Some("child process exited abnormally".to_owned()),
            Some(join_list(["CHILDSTATUS", &pid, &status.to_string()])),
        ),
        ProcessExit::Signaled(name) => {
            let message = signal_message(name);
            (
                Some(format!("child killed: {message}")),
                Some(join_list(["CHILDKILLED", &pid, name, message])),
            )
        }
    }
}

/// `Tcl_SignalMsg`'s description of the signal named `name` (`SIGKILL`).
#[must_use]
pub fn signal_message(name: &str) -> &'static str {
    match name {
        "SIGABRT" => "SIGABRT",
        "SIGALRM" => "alarm clock",
        "SIGBUS" => "bus error",
        "SIGCHLD" | "SIGCLD" => "child status changed",
        "SIGCONT" => "continue after stop",
        "SIGEMT" => "EMT instruction",
        "SIGFPE" => "floating-point exception",
        "SIGHUP" => "hangup",
        "SIGILL" => "illegal instruction",
        "SIGINT" => "interrupt",
        "SIGIO" | "SIGPOLL" => "input/output possible on file",
        "SIGIOT" => "IOT instruction",
        "SIGKILL" => "kill signal",
        "SIGLOST" => "resource lost",
        "SIGPIPE" => "write on pipe with no readers",
        "SIGPROF" => "profiling alarm",
        "SIGPWR" => "power-fail restart",
        "SIGQUIT" => "quit signal",
        "SIGSEGV" => "segmentation violation",
        "SIGSTOP" => "stop",
        "SIGSYS" => "bad argument to system call",
        "SIGTERM" => "software termination signal",
        "SIGTRAP" => "trace trap",
        "SIGTSTP" => "stop signal from tty",
        "SIGTTIN" => "background tty read",
        "SIGTTOU" => "background tty write",
        "SIGURG" => "urgent I/O condition",
        "SIGUSR1" => "user-defined signal 1",
        "SIGUSR2" => "user-defined signal 2",
        "SIGVTALRM" => "virtual time alarm",
        "SIGWINCH" => "window changed",
        "SIGXCPU" => "exceeded CPU time limit",
        "SIGXFSZ" => "exceeded file size limit",
        "SIGINFO" => "information request",
        _ => "unknown signal",
    }
}

/// `pwd` — the current working directory. The working directory is a *mandatory*
/// capability ([`Env`](tcl_platform::Env)), virtualised on hosts without a real
/// one, so this never reports "unsupported".
pub fn pwd<O: ValueOps>(ops: &mut O, host: &dyn Host) -> Result<O::Value, CmdError> {
    let dir = host.env().cwd()?;
    Ok(ops.new_string(dir))
}
