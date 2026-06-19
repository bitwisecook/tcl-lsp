//! Platform-backed command helpers (`exec`, `pwd`, `cd`, …).
//!
//! These are the P1/P2 bodies: the portable Tcl semantics (argument parsing,
//! the error catalogue, result building) live here once, while the actual host
//! syscalls are reached through the [`tcl_platform`] capability traits and live
//! per-target. A **platform-conditional** capability (subprocess, sockets) that
//! a host lacks — every WASM target lacks subprocess — surfaces as the faithful
//! Tcl "unsupported" error rather than a panic, exactly as the capability model
//! intends.

use tcl_platform::Host;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// `exec arg ?arg ...?` — run a command and capture its standard output (with
/// trailing newlines trimmed), the minimal form.
///
/// Subprocess execution is a *platform-conditional* capability: when the host
/// provides no [`Process`](tcl_platform::Process) (every WASM target, a sandbox),
/// this yields the faithful unsupported error instead of running — the same
/// shared body, a different host. Redirections / pipelines / `-keepnewline`
/// land as the command fills in.
pub fn exec<O: ValueOps>(
    ops: &mut O,
    host: &dyn Host,
    args: &[O::Value],
) -> Result<O::Value, CmdError> {
    let Some(process) = host.process() else {
        return Err(CmdError::new(
            "exec is not available: this platform provides no subprocess support",
        ));
    };
    if args.is_empty() {
        return Err(CmdError::wrong_args("exec ?-option ...? arg ?arg ...?"));
    }
    let parts: Vec<String> = args.iter().map(|a| ops.as_str(a).to_string()).collect();
    let part_refs: Vec<&str> = parts.iter().map(String::as_str).collect();
    let output = process.run(&part_refs)?;
    let mut out = String::from_utf8_lossy(&output.stdout).into_owned();
    while out.ends_with('\n') {
        out.pop();
    }
    Ok(ops.new_string(out))
}

/// `pwd` — the current working directory. The working directory is a *mandatory*
/// capability ([`Env`](tcl_platform::Env)), virtualised on hosts without a real
/// one, so this never reports "unsupported".
pub fn pwd<O: ValueOps>(ops: &mut O, host: &dyn Host) -> Result<O::Value, CmdError> {
    let dir = host.env().cwd()?;
    Ok(ops.new_string(dir))
}
