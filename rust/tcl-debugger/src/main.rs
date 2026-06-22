//! `tcl-debug` — an interactive CLI front-end for the native step debugger.
//!
//! Drives a [`tcl_debugger::VmBackend`] over a script: launch, set
//! breakpoints, step, and inspect the stack and variables. The backend is
//! record-and-replay (the script runs once up front to capture the trace), so
//! stepping moves a cursor over that trace — fast and side-effect-free to
//! re-inspect.
//!
//! Commands (gdb-ish): `break <line>` (`b`), `step` (`s`), `next` (`n`),
//! `finish` (`f`), `continue` (`c`), `print <var>` (`p`), `stack` (`bt`),
//! `vars`, `list` (`l`), `quit` (`q`). Reads commands from stdin so it scripts
//! and tests cleanly.

#![forbid(unsafe_code)]

use std::io::{BufRead, Write};

use tcl_debugger::backend::{DebugBackend, DebugError, VmBackend};

fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: tcl-debug <script.tcl>   (interactive CLI)");
        eprintln!("       tcl-debug --dap          (Debug Adapter Protocol over stdio)");
        return std::process::ExitCode::from(2);
    };

    // DAP mode: speak the Debug Adapter Protocol over stdin/stdout for an editor.
    if path == "--dap" {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut reader = stdin.lock();
        let mut writer = stdout.lock();
        return match tcl_debugger::dap::run_server(&mut reader, &mut writer) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("dap: {e}");
                std::process::ExitCode::from(1)
            }
        };
    }

    let mut backend = VmBackend::new();
    if let Err(e) = backend.launch(&path, None) {
        eprintln!("error: {e}");
        return std::process::ExitCode::from(1);
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    announce_stop(&backend, &mut out);

    let mut breakpoints: Vec<u32> = Vec::new();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines().map_while(Result::ok) {
        let mut parts = line.split_whitespace();
        let Some(cmd) = parts.next() else {
            continue;
        };
        let arg = parts.next();
        match cmd {
            "q" | "quit" => break,
            "b" | "break" => match arg.and_then(|a| a.parse::<u32>().ok()) {
                Some(l) => {
                    if !breakpoints.contains(&l) {
                        breakpoints.push(l);
                    }
                    let accepted = backend.set_breakpoints(&breakpoints).unwrap_or_default();
                    let _ = writeln!(out, "breakpoints: {accepted:?}");
                }
                None => {
                    let _ = writeln!(out, "usage: break <line>");
                }
            },
            "s" | "step" => control(&mut backend, &mut out, VmBackend::step_in),
            "n" | "next" => control(&mut backend, &mut out, VmBackend::step_over),
            "f" | "finish" => control(&mut backend, &mut out, VmBackend::step_out),
            "c" | "continue" => control(&mut backend, &mut out, VmBackend::continue_execution),
            "p" | "print" => match arg {
                Some(name) => {
                    let _ = match backend.evaluate(name) {
                        Ok(v) => writeln!(out, "{name} = {v}"),
                        Err(e) => writeln!(out, "{e}"),
                    };
                }
                None => {
                    let _ = writeln!(out, "usage: print <var>");
                }
            },
            "bt" | "stack" => match backend.stack_trace() {
                Ok(frames) => {
                    for fr in frames {
                        let _ = writeln!(out, "  #{} {} (line {})", fr.id, fr.name, fr.line);
                    }
                }
                Err(e) => {
                    let _ = writeln!(out, "{e}");
                }
            },
            "vars" | "locals" => match backend.variables(0) {
                Ok(vars) => {
                    for v in vars {
                        let _ = writeln!(out, "  {} = {}", v.name, v.value);
                    }
                }
                Err(e) => {
                    let _ = writeln!(out, "{e}");
                }
            },
            "l" | "list" => announce_stop(&backend, &mut out),
            other => {
                let _ = writeln!(out, "unknown command: {other}");
            }
        }
        let _ = out.flush();
    }
    std::process::ExitCode::SUCCESS
}



/// Run a control operation and report the resulting stop (or end of program).
fn control(
    backend: &mut VmBackend,
    out: &mut impl Write,
    op: fn(&mut VmBackend) -> Result<(), DebugError>,
) {
    match op(backend) {
        Ok(()) => announce_stop(backend, out),
        Err(DebugError::Finished) => {
            let _ = writeln!(out, "program finished");
        }
        Err(e) => {
            let _ = writeln!(out, "{e}");
        }
    }
}

/// Print the current stop location.
fn announce_stop(backend: &VmBackend, out: &mut impl Write) {
    if let Some(stop) = backend.last_stop() {
        let _ = writeln!(
            out,
            "stopped at line {} [{:?}]: {}",
            stop.line, stop.reason, stop.command_text
        );
    } else {
        let _ = writeln!(out, "not running");
    }
}