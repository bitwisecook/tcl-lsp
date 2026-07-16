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

//! `tclvm` — a CLI/REPL driver for the native Tcl bytecode VM (`tcl-vm`).
//!
//! Promotes the `tcl-vm` examples (`eval` / `run_test`) to a shipping binary.
//! The VM library is compiler-optional by design (the [`CompileService`] seam
//! keeps it off `tcl-compiler`), so this crate supplies the concrete
//! compiler-backed service and the host wiring.
//!
//! Modes (resolved like `tclsh`):
//!
//! - `tclvm <file.tcl> [args…]` — compile and run a script file; `args` become
//!   the script's `argv` (`argv0` is the file, `argc` the count).
//! - `tclvm -c "<script>"` — evaluate an inline script string.
//! - `tclvm` with piped stdin — run the piped script.
//! - `tclvm` on a TTY — an interactive REPL (`info complete`-aware line
//!   accumulation via `tcl_lexer::script_is_complete`).
//!
//! Like the `run_test` example, work runs on a large-stack worker thread so deep
//! recursion surfaces as a catchable Tcl error rather than a native stack
//! overflow.

use std::io::{IsTerminal, Read, Write};
use std::sync::{Arc, Mutex};

use tcl_compiler::cfg_builder::build_cfg_codegen as build_cfg;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode as lower_to_ir;
use tcl_dialect::TclVersion;
use tcl_lexer::script_is_complete;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Value, Vm};

/// The `CompileService` the VM uses for runtime `eval` / command substitution
/// (and for the top-level script the driver runs): the real Rust compiler
/// pipeline (lower → CFG → bytecode). Matches the `eval` / `run_test` examples.
struct Svc(CommandRegistry);

impl CompileService for Svc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, &self.0);
        let cfg = build_cfg(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.0))
    }
}

/// Forward the VM's `puts` output straight through to the process stdout.
struct Stdout;
impl Write for Stdout {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        std::io::stdout().write(b)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

const STACK_BYTES: usize = 512 * 1024 * 1024;

fn main() {
    let code = std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(run)
        .expect("spawn worker thread")
        .join()
        .expect("worker thread panicked");
    std::process::exit(code);
}

/// What the parsed command line asks the driver to do.
enum Mode {
    /// Run a script file with the given trailing `argv`.
    File { path: String, argv: Vec<String> },
    /// Evaluate an inline script string.
    Command(String),
    /// No script given: REPL if stdin is a TTY, else run piped stdin.
    Stdin,
    /// Force the interactive REPL even when stdin is not a TTY (`-i`).
    Repl,
    /// Print usage to stderr and exit with the given code.
    Usage(i32),
}

/// Parse `std::env::args` into a [`Mode`] plus an optional Tcl runtime
/// version. Recognises `--tcl-version <x.y>` (select 8.4–9.1 variable /
/// resolution semantics, like picking a `tclsh<x.y>` binary), `-c <script>`,
/// `-i` (force REPL), `-h` / `--help`, and otherwise treats the first
/// non-flag argument as a script file (everything after it is the script's
/// `argv`, `tclsh`-style).
fn parse_args() -> (Mode, Option<TclVersion>) {
    let mut version = None;
    let mut args = std::env::args().skip(1);
    loop {
        return match args.next() {
            None => (Mode::Stdin, version),
            Some(flag) if flag == "-h" || flag == "--help" => (Mode::Usage(0), version),
            Some(flag) if flag == "-i" => (Mode::Repl, version),
            Some(flag) if flag == "--tcl-version" => {
                match args
                    .next()
                    .as_deref()
                    .and_then(TclVersion::from_package_version)
                {
                    Some(v) => {
                        version = Some(v);
                        continue;
                    }
                    None => (Mode::Usage(2), version),
                }
            }
            Some(flag) if flag == "-c" => match args.next() {
                Some(script) => (Mode::Command(script), version),
                None => (Mode::Usage(2), version),
            },
            Some(path) => (
                Mode::File {
                    path,
                    argv: args.collect(),
                },
                version,
            ),
        };
    }
}

fn usage() {
    eprintln!(
        "usage:\n  \
         tclvm <file.tcl> [args…]   run a script file\n  \
         tclvm -c <script>         evaluate an inline script\n  \
         tclvm -i                  force the interactive REPL\n  \
         tclvm                     REPL (TTY) or run piped stdin\n\n\
         options:\n  \
         --tcl-version <x.y>       select 8.4|8.5|8.6|9.0|9.1 runtime\n  \
                                   semantics (default 9.0)"
    );
}

/// Build a VM with the compiler-backed `CompileService` and stdout host
/// output, at the requested runtime version (`None` keeps the VM default).
fn new_vm(version: Option<TclVersion>) -> Vm {
    let mut vm = Vm::with_output(Box::new(Stdout));
    // Default to the plain-Tcl profile's pinned VM release (dialect-profile
    // model §5.4); `--tcl-version <x.y>` overrides it.
    vm.set_runtime_version(
        version
            .unwrap_or_else(|| tcl_dialect::DialectProfile::by_name("tcl9.0").vm_runtime_version),
    );
    vm.set_compiler(Box::new(Svc(CommandRegistry::build_default())));
    // Enable the real `thread` package: a Send factory each worker calls to
    // build its own compiler, and a thread-safe shared stdout for `puts`.
    vm.enable_threads(
        Arc::new(|| {
            Box::new(Svc(CommandRegistry::build_default()))
                as Box<dyn CompileService<Module = tcl_bytecode::ModuleAsm>>
        }),
        Arc::new(Mutex::new(Box::new(Stdout) as Box<dyn Write + Send>)),
    );
    // Install the on-demand autoloader so library procs (word.tcl, …) resolve.
    let _ = vm.init_auto_load();
    vm
}

/// Seed `argv0` / `argv` / `argc` the way `tclsh` does before running a file.
fn set_argv(vm: &mut Vm, argv0: &str, argv: &[String]) {
    let _ = vm.set_var("argv0", Value::string(argv0));
    let _ = vm.set_var(
        "argv",
        Value::list(argv.iter().map(|s| Value::string(s.as_str())).collect()),
    );
    let _ = vm.set_var(
        "argc",
        Value::int(i64::try_from(argv.len()).unwrap_or(i64::MAX)),
    );
}

/// Compile and run a whole script once, reporting any error to stderr. Returns
/// the process exit code.
fn run_script(vm: &mut Vm, src: &str) -> i32 {
    let result = vm.eval_source(src);
    // `exit` records a pending code on the VM rather than killing the process
    // (so embedders survive); the standalone CLI performs the real termination.
    if let Some(code) = vm.take_exit() {
        return code;
    }
    match result {
        Ok(comp) if comp.code.is_ok() => 0,
        Ok(comp) => {
            eprintln!("{}", comp.result.to_str());
            1
        }
        Err(e) => {
            eprintln!("{}", e.message);
            1
        }
    }
}

fn run() -> i32 {
    let (mode, version) = parse_args();
    match mode {
        Mode::Usage(code) => {
            usage();
            code
        }
        Mode::Command(script) => {
            let mut vm = new_vm(version);
            run_script(&mut vm, &script)
        }
        Mode::Repl => {
            let stdin = std::io::stdin();
            let mut out = std::io::stdout();
            repl_loop(&mut new_vm(version), &mut stdin.lock(), &mut out)
        }
        Mode::File { path, argv } => {
            let src = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("tclvm: cannot read {path}: {e}");
                    return 1;
                }
            };
            let mut vm = new_vm(version);
            set_argv(&mut vm, &path, &argv);
            run_script(&mut vm, &src)
        }
        Mode::Stdin => {
            if std::io::stdin().is_terminal() {
                let stdin = std::io::stdin();
                let mut out = std::io::stdout();
                repl_loop(&mut new_vm(version), &mut stdin.lock(), &mut out)
            } else {
                let mut src = String::new();
                if let Err(e) = std::io::stdin().read_to_string(&mut src) {
                    eprintln!("tclvm: cannot read stdin: {e}");
                    return 1;
                }
                let mut vm = new_vm(version);
                run_script(&mut vm, &src)
            }
        }
    }
}

/// Interactive REPL loop over an arbitrary reader/writer (so it is testable
/// without a real terminal). Accumulates input lines until they form a complete
/// command (`tcl_lexer::script_is_complete`, the `info complete` oracle),
/// evaluates it in the persistent global frame, and writes a non-empty result.
/// Variables and procs persist across evaluations because each line runs in the
/// same VM. Prompts (`% ` primary, `> ` continuation) and results go to `out`;
/// errors go to stderr. Always returns 0 (EOF is a clean exit).
fn repl_loop<R: std::io::BufRead, W: Write>(vm: &mut Vm, reader: &mut R, out: &mut W) -> i32 {
    let mut buffer = String::new();
    let _ = write!(out, "% ");
    let _ = out.flush();

    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break, // EOF (Ctrl-D) or read error
            Ok(_) => {}
        }
        buffer.push_str(&line);

        // A blank buffer on its own just re-prompts; otherwise keep reading
        // (continuation prompt) until the command is complete.
        if buffer.trim().is_empty() {
            buffer.clear();
            let _ = write!(out, "% ");
            let _ = out.flush();
            continue;
        }
        if !script_is_complete(&buffer) {
            let _ = write!(out, "> ");
            let _ = out.flush();
            continue;
        }

        let result = vm.eval_source(&buffer);
        // `exit` in the REPL ends the session with the requested code.
        if let Some(code) = vm.take_exit() {
            let _ = writeln!(out);
            return code;
        }
        match result {
            Ok(comp) if comp.code.is_ok() => {
                let result = comp.result.to_str();
                if !result.is_empty() {
                    let _ = writeln!(out, "{result}");
                }
            }
            Ok(comp) => eprintln!("{}", comp.result.to_str()),
            Err(e) => eprintln!("{}", e.message),
        }
        buffer.clear();
        let _ = write!(out, "% ");
        let _ = out.flush();
    }
    // A trailing newline so the shell prompt starts on its own line after EOF.
    let _ = writeln!(out);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive `repl_loop` over a canned input script and return what it wrote to
    /// `out` (prompts + results), with a fresh compiler-backed VM.
    fn drive(input: &str) -> String {
        drive_at(input, None)
    }

    /// [`drive`] at an explicit runtime version (the `--tcl-version` path).
    fn drive_at(input: &str, version: Option<TclVersion>) -> String {
        let mut vm = new_vm(version);
        let mut reader = std::io::Cursor::new(input.as_bytes().to_vec());
        let mut out: Vec<u8> = Vec::new();
        repl_loop(&mut vm, &mut reader, &mut out);
        String::from_utf8(out).expect("utf-8")
    }

    #[test]
    fn repl_persists_state_and_prints_results() {
        // `set x 5` echoes its result; `$x` survives into the next command.
        let out = drive("set x 5\nexpr {$x + 1}\n");
        assert!(out.contains("% 5"), "set result not echoed: {out:?}");
        assert!(out.contains('6'), "var did not persist: {out:?}");
    }

    #[test]
    fn repl_continuation_prompt_for_incomplete_command() {
        // An open brace is incomplete: a continuation `> ` prompt is shown, and
        // the command only evaluates once the brace closes on the next line.
        let out = drive("set y {a\nb}\nllength $y\n");
        assert!(out.contains("> "), "no continuation prompt: {out:?}");
        // `llength {a\nb}` is 2 (two whitespace-separated words).
        assert!(out.contains('2'), "joined value not evaluated: {out:?}");
    }

    #[test]
    fn repl_blank_line_reprompts() {
        let out = drive("\n\nset z 9\n");
        assert!(out.contains("% 9"), "blank lines broke the loop: {out:?}");
    }

    #[test]
    fn tcl_version_flag_selects_cross_version_var_semantics() {
        // M11 pinned vector (tclsh8.6 vs tclsh9.0): `incr` on a bare name at
        // namespace scope reaches the global under 8.x (41 → 42), but creates
        // a fresh namespace variable under 9.0 (→ 1).
        let script = "set g 41\nnamespace eval foo { incr g }\n";
        let out86 = drive_at(script, Some(TclVersion::V8_6));
        assert!(
            out86.contains("42"),
            "8.6 falls back to the global: {out86:?}"
        );
        let out90 = drive_at(script, Some(TclVersion::V9_0));
        assert!(
            out90.contains("% 1"),
            "9.0 creates in the namespace: {out90:?}"
        );
    }
}
