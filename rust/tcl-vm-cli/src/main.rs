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
//! `--static-extensions` opts the VM into a `load` command over the C Tcl
//! extensions the build links in (`tcl-cshim`; the `static-extensions`
//! feature), the way a `tclsh` test build links `Tcltest`: without the flag
//! there is no `load`, and no script can ask for one.
//!
//! Like the `run_test` example, work runs on a large-stack worker thread so deep
//! recursion surfaces as a catchable Tcl error rather than a native stack
//! overflow.

use std::io::{IsTerminal, Read, Write};
use std::sync::{Arc, Mutex};

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_lexer::script_is_complete;
use tcl_vm::{CompileService, Value, Vm};

/// The `CompileService` the VM uses for runtime `eval` / command substitution
/// (and for the top-level script the driver runs): the real Rust compiler
/// pipeline (lower → CFG → bytecode). Matches the `eval` / `run_test` examples.
///
/// Built from one resolved [`DialectProfile`]: the registry,
/// the lexer grammar (`{*}` expansion, the `${…}` delimiting rule), and the
/// expression dialect all come from the emulated release, so `--tcl-version
/// 8.4` rejects `{*}` exactly as `tclsh8.4` does.
type Svc = BytecodeCompileService;

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
#[derive(Debug, PartialEq, Eq)]
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

/// What the command line asks for besides the mode.
#[derive(Debug, Default)]
struct Options {
    /// The Tcl release to emulate (`--tcl-version`); `None` keeps the default.
    version: Option<TclVersion>,
    /// Give the VM a `load` over the linked-in extensions
    /// (`--static-extensions`).
    static_extensions: bool,
}

/// Parse `std::env::args` into a [`Mode`] plus the [`Options`]. Recognises
/// `--tcl-version <x.y>` (select 8.4–9.1 variable / resolution semantics, like
/// picking a `tclsh<x.y>` binary), `--static-extensions`, `-c <script>`, `-i`
/// (force REPL), `-h` / `--help`, and otherwise treats the first non-flag
/// argument as a script file (everything after it is the script's `argv`,
/// `tclsh`-style).
fn parse_args() -> (Mode, Options) {
    parse_args_from(std::env::args().skip(1))
}

/// [`parse_args`] over an explicit argument list (without the program name).
fn parse_args_from(mut args: impl Iterator<Item = String>) -> (Mode, Options) {
    let mut options = Options::default();
    loop {
        return match args.next() {
            None => (Mode::Stdin, options),
            Some(flag) if flag == "-h" || flag == "--help" => (Mode::Usage(0), options),
            Some(flag) if flag == "-i" => (Mode::Repl, options),
            Some(flag) if flag == "--static-extensions" => {
                options.static_extensions = true;
                continue;
            }
            Some(flag) if flag == "--tcl-version" => {
                match args
                    .next()
                    .as_deref()
                    .and_then(TclVersion::from_package_version)
                {
                    Some(v) => {
                        options.version = Some(v);
                        continue;
                    }
                    None => (Mode::Usage(2), options),
                }
            }
            Some(flag) if flag == "-c" => match args.next() {
                Some(script) => (Mode::Command(script), options),
                None => (Mode::Usage(2), options),
            },
            Some(path) => (
                Mode::File {
                    path,
                    argv: args.collect(),
                },
                options,
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
                                   semantics (default 9.0)\n  \
         --static-extensions       give scripts a `load` over the extensions\n  \
                                   linked into this build (the\n  \
                                   `static-extensions` feature)"
    );
}

/// Build a VM with the compiler-backed `CompileService` and stdout host
/// output, as `options` ask.
fn new_vm(options: &Options) -> Vm {
    build_vm(Box::new(Stdout), options)
}

/// [`configure_vm`] over a fresh VM writing to `output`, with the `load` the
/// options ask for.
fn build_vm(output: Box<dyn Write>, options: &Options) -> Vm {
    let mut vm = configure_vm(Vm::with_output(output), options.version);
    if options.static_extensions {
        link_static_extensions(&mut vm);
    }
    vm
}

/// Give `vm` a `load` over the extensions the build links in: the host's
/// opt-in, which is why nothing else registers it.
#[cfg(feature = "static-extensions")]
fn link_static_extensions(vm: &mut Vm) {
    tcl_engine_tclvm::register_host_command(
        vm,
        "load",
        std::rc::Rc::new(tcl_cshim::StaticExtensions::bundled()),
    );
}

/// A build without the feature links nothing, and `run` has refused the flag.
#[cfg(not(feature = "static-extensions"))]
fn link_static_extensions(_vm: &mut Vm) {}

/// Attach the compiler/runtime services to an already-created VM. Tests pass
/// a byte capture here so Tcl-originated REPL results still travel through the
/// VM's configured standard channel.
fn configure_vm(mut vm: Vm, version: Option<TclVersion>) -> Vm {
    // Default to the plain-Tcl 9.0 profile; `--tcl-version <x.y>` selects
    // another release's profile. The profile is resolved once and drives
    // BOTH halves: the runtime semantics (below) and the compiler's
    // grammar/registry (`Svc::for_profile`). Both dialect *names* resolve
    // through the one ingress seam.
    //
    // Letting `--tcl-version` name a non-plain-Tcl environment would need a
    // new flag spelling and a wider acceptance set, so the ingress stays
    // release-only here and only its resolution moves.
    let release = version.unwrap_or_else(|| {
        tcl_registry::model::resolve_environment("tcl9.0")
            .unit_profile()
            .vm_runtime_version
    });
    let profile = tcl_registry::model::resolve_environment(release.dialect_name()).unit_profile();
    vm.set_dialect_profile(profile);
    // Codegen targets the same release: the dialect is a *compile* target, not
    // an execution-time switch, so a numeric literal is resolved for 8.6 while
    // compiling rather than re-read under 8.6 rules at run time — and a
    // grammar fact (`{*}` is 8.5+) is enforced while compiling too.
    vm.set_compiler(Box::new(Svc::for_profile(profile)));
    // Enable the real `thread` package: a Send factory each worker calls to
    // build its own compiler, and a thread-safe shared stdout for `puts`.
    vm.enable_threads(
        Arc::new(move || {
            Box::new(Svc::for_profile(profile))
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
            vm.report_stderr_text(&comp.result.to_str());
            1
        }
        Err(e) => {
            report_eval_error(vm, &e);
            1
        }
    }
}

fn report_eval_error(vm: &mut Vm, error: &tcl_vm::TclError) {
    match error.message_unicode() {
        Ok(message) => vm.report_stderr_text(&message),
        Err(host) => vm.report_stderr_text(&host.to_string()),
    }
}

fn run() -> i32 {
    let (mode, options) = parse_args();
    #[cfg(not(feature = "static-extensions"))]
    if options.static_extensions && !matches!(mode, Mode::Usage(_)) {
        eprintln!("tclvm: --static-extensions needs a build with the `static-extensions` feature");
        return 2;
    }
    match mode {
        Mode::Usage(code) => {
            usage();
            code
        }
        Mode::Command(script) => {
            let mut vm = new_vm(&options);
            run_script(&mut vm, &script)
        }
        Mode::Repl => {
            let stdin = std::io::stdin();
            let mut out = std::io::stdout();
            repl_loop(&mut new_vm(&options), &mut stdin.lock(), &mut out)
        }
        Mode::File { path, argv } => {
            let src = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("tclvm: cannot read {path}: {e}");
                    return 1;
                }
            };
            let mut vm = new_vm(&options);
            set_argv(&mut vm, &path, &argv);
            run_script(&mut vm, &src)
        }
        Mode::Stdin => {
            if std::io::stdin().is_terminal() {
                let stdin = std::io::stdin();
                let mut out = std::io::stdout();
                repl_loop(&mut new_vm(&options), &mut stdin.lock(), &mut out)
            } else {
                let mut src = String::new();
                if let Err(e) = std::io::stdin().read_to_string(&mut src) {
                    eprintln!("tclvm: cannot read stdin: {e}");
                    return 1;
                }
                let mut vm = new_vm(&options);
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
                    let _ = vm.write_stdout_text(&result, true);
                }
            }
            Ok(comp) => {
                vm.report_stderr_text(&comp.result.to_str());
            }
            Err(e) => {
                report_eval_error(vm, &e);
            }
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
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct Capture(Rc<RefCell<Vec<u8>>>);

    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Drive `repl_loop` over a canned input script and return what it wrote to
    /// `out` (prompts + results), with a fresh compiler-backed VM.
    fn drive(input: &str) -> String {
        drive_at(input, None)
    }

    /// [`drive`] at an explicit runtime version (the `--tcl-version` path).
    fn drive_at(input: &str, version: Option<TclVersion>) -> String {
        drive_with(
            input,
            &Options {
                version,
                ..Options::default()
            },
        )
    }

    /// [`drive`] under the options a command line would have set.
    fn drive_with(input: &str, options: &Options) -> String {
        let bytes = Rc::new(RefCell::new(Vec::new()));
        let capture = Capture(Rc::clone(&bytes));
        let mut vm = build_vm(Box::new(capture.clone()), options);
        let mut reader = std::io::Cursor::new(input.as_bytes().to_vec());
        let mut out = capture;
        repl_loop(&mut vm, &mut reader, &mut out);
        let output = bytes.borrow().clone();
        String::from_utf8(output).expect("utf-8")
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

    #[test]
    fn tcl_version_flag_gates_grammar_and_command_surface() {
        // `{*}` expansion is TIP 157 (8.5+). The braced catch
        // body recompiles at run time through the CLI's compile service, so
        // under an 8.4 pin the 8.4 grammar rejects it with the genuine
        // tclsh8.4 message — as a *catchable* error, matching C Tcl's
        // deferral of compile-time parse errors. (The REPL echoes command
        // *results*; `$m` on its own line surfaces the caught message.)
        let expand = "catch {llength [list {*}{a b}]} m\nset m\n";
        let out84 = drive_at(expand, Some(TclVersion::V8_4));
        assert!(
            out84.contains("extra characters after close-brace"),
            "8.4 rejects {{*}}: {out84:?}"
        );
        let out90 = drive_at(expand, Some(TclVersion::V9_0));
        assert!(out90.contains("% 2"), "9.0 expands {{*}}: {out90:?}");

        // `lassign` is 8.5+, so an 8.4-pinned VM resolves it
        // like tclsh8.4 — to an invalid command name.
        let lassign = "catch {lassign {a b} x} m\nset m\n";
        let out84 = drive_at(lassign, Some(TclVersion::V8_4));
        assert!(
            out84.contains("invalid command name \"lassign\""),
            "8.4 has no lassign: {out84:?}"
        );
        let out90 = drive_at(lassign, Some(TclVersion::V9_0));
        assert!(out90.contains("% b"), "9.0 has lassign: {out90:?}");
    }

    #[test]
    fn without_the_flag_a_script_has_no_load() {
        let out = drive("catch {load {} Pkga} m\nset m\n");
        assert!(
            out.contains("invalid command name \"load\""),
            "a script cannot ask for a load: {out:?}"
        );
    }

    #[cfg(feature = "static-extensions")]
    fn with_extensions() -> Options {
        Options {
            static_extensions: true,
            ..Options::default()
        }
    }

    #[cfg(all(feature = "static-extensions", not(windows)))]
    #[test]
    fn with_the_flag_a_script_loads_the_extension_the_build_links() {
        let out = drive_with(
            "load {} Pkga\npkga_eq abc abc\npkga_quote {a b c}\n",
            &with_extensions(),
        );
        assert!(out.contains("% 1"), "pkga_eq answered: {out:?}");
        assert!(out.contains("% a b c"), "pkga_quote answered: {out:?}");
        let out = drive_with(
            "load ./libpkga.so\ncatch {pkga_eq a} m\nset m\n",
            &with_extensions(),
        );
        assert!(
            out.contains("wrong # args: should be \"pkga_eq string1 string2\""),
            "the file name stood for the prefix: {out:?}"
        );
    }

    #[cfg(feature = "static-extensions")]
    #[test]
    fn with_the_flag_a_library_the_build_does_not_link_is_still_refused() {
        let out = drive_with(
            "catch {load libnosuch.so} m\nset m\ncatch {load {} Nosuch} m\nset m\n",
            &with_extensions(),
        );
        assert!(
            out.contains("couldn't load file \"libnosuch.so\""),
            "{out:?}"
        );
        assert!(
            out.contains("no library with prefix \"Nosuch\" is loaded statically"),
            "{out:?}"
        );
    }

    fn parse(args: &[&str]) -> (Mode, Options) {
        parse_args_from(args.iter().map(|arg| (*arg).to_owned()))
    }

    #[test]
    fn the_static_extensions_flag_is_an_option_and_not_a_script_argument() {
        let (mode, options) = parse(&["--static-extensions", "-c", "puts hi"]);
        assert_eq!(mode, Mode::Command("puts hi".to_owned()));
        assert!(options.static_extensions);

        // `tclsh`-style: after the script file, everything is the script's argv.
        let (mode, options) = parse(&["script.tcl", "--static-extensions"]);
        assert_eq!(
            mode,
            Mode::File {
                path: "script.tcl".to_owned(),
                argv: vec!["--static-extensions".to_owned()],
            }
        );
        assert!(!options.static_extensions);

        let (mode, options) = parse(&["--tcl-version", "8.6", "--static-extensions", "-i"]);
        assert_eq!(mode, Mode::Repl);
        assert!(options.static_extensions);
        assert_eq!(options.version, Some(TclVersion::V8_6));

        let (_, options) = parse(&["-c", "puts hi"]);
        assert!(!options.static_extensions, "no flag, no load");
    }
}
