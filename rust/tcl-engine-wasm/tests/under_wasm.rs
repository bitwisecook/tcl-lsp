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

//! The runtime compiled to `wasm32` as an engine, under wasmtime: the cases
//! every runtime engine is held to (`runtime/rust/tests/common/engine_cases.rs`,
//! included by path, one copy), the test extension `pkga.c` built as a side
//! module against the header's WASM leg and evaluated with fuel, and the
//! registry's extension seam bound to the WASM host.
//!
//! It needs the real-link toolchain, and shares its gate and its reserved
//! runtime with `tcl-compiler`'s real-link suites (`tests/common/wasm_link.rs`,
//! included by path, one copy): without the toolchain every test here is
//! skipped loudly, and with `TCL_REQUIRE_WASM_LINK=1` a missing piece is a
//! failure.

#[path = "../../tcl-compiler/tests/common/wasm_link.rs"]
mod wasm_link;

#[path = "../../../runtime/rust/tests/common/engine_cases.rs"]
mod engine_cases;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tcl_engine_api::{
    Budget, BudgetKind, CompileUnit, Engine, EngineError, HostCommand, HostOutcome, Value,
};
use tcl_engine_wasm::{WasmEngine, WasmExtensionHost, WasmRuntime};
use tcl_registry::extension_host::{
    ExtensionHost, artefact_hash, clear_extension_host, evaluate_extension, install_extension_host,
    load_extension,
};
use tcl_registry::value_transfer::{BudgetLimit, DeclineReason, ImplementationBudget};

include!("../../tcl-cshim/tests/vectors/pkga.rs");

/// The two vectors the runtime answers differently from C Tcl, under WASM as
/// natively (`runtime/rust/tests/pkga_extension.rs` says why): the script, and
/// the code, result and `errorCode` the runtime gives.
const DIVERGENCES: &[(&str, i64, &str, &str)] = &[
    ("pkga_calc join a\\u0000b c", 0, "a+c", ""),
    ("pkga_calc fail x\\u0000y", 1, "x", "PKGA FAIL x"),
];

/// An extension that reads the clock, randomness and the environment through
/// WASI itself, ends the process, and fills most of its stack: what it answers
/// is what the host hands every module.
const PROBE: &str = r#"
#include <tcl.h>

__attribute__((import_module("wasi_snapshot_preview1"), import_name("clock_time_get")))
int wasi_clock_time_get(int id, long long precision, long long *time);
__attribute__((import_module("wasi_snapshot_preview1"), import_name("random_get")))
int wasi_random_get(unsigned char *buffer, int length);
__attribute__((import_module("wasi_snapshot_preview1"), import_name("environ_sizes_get")))
int wasi_environ_sizes_get(int *count, int *size);
__attribute__((import_module("wasi_snapshot_preview1"), import_name("proc_exit")))
void wasi_proc_exit(int code);

static int
Ambient(void *clientData, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    long long now = -1;
    unsigned char bytes[8] = {1, 1, 1, 1, 1, 1, 1, 1};
    int count = -1, size = -1, i;
    long long random = 0;
    Tcl_Obj *list;
    wasi_clock_time_get(0, 0, &now);
    wasi_random_get(bytes, 8);
    wasi_environ_sizes_get(&count, &size);
    for (i = 0; i < 8; i++) {
        random = (random << 8) | bytes[i];
    }
    list = Tcl_NewListObj(0, NULL);
    Tcl_ListObjAppendElement(interp, list, Tcl_NewWideIntObj(now));
    Tcl_ListObjAppendElement(interp, list, Tcl_NewWideIntObj(random));
    Tcl_ListObjAppendElement(interp, list, Tcl_NewIntObj(count));
    Tcl_SetObjResult(interp, list);
    return TCL_OK;
}

static int
Exit(void *clientData, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    wasi_proc_exit(3);
    return TCL_OK;
}

static char marker[64] = "the probe's data, intact";

static int
Stack(void *clientData, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[])
{
    volatile char frame[49152];
    int i;
    for (i = 0; i < (int) sizeof(frame); i++) {
        frame[i] = (char) i;
    }
    Tcl_SetObjResult(interp, Tcl_NewStringObj(marker, -1));
    return frame[0] == 0 ? TCL_OK : TCL_ERROR;
}

int
Probe_Init(Tcl_Interp *interp)
{
    Tcl_CreateObjCommand(interp, "probe_ambient", Ambient, NULL, NULL);
    Tcl_CreateObjCommand(interp, "probe_exit", Exit, NULL, NULL);
    Tcl_CreateObjCommand(interp, "probe_stack", Stack, NULL, NULL);
    return TCL_OK;
}
"#;

/// An extension that calls what the runtime does not export.
const STRAY: &str = r"
#include <tcl.h>

extern int Tcl_NoSuchFunction(Tcl_Interp *interp);

int
Stray_Init(Tcl_Interp *interp)
{
    return Tcl_NoSuchFunction(interp);
}
";

/// The runtime and the extensions, built once for every test.
struct Built {
    runtime: WasmRuntime,
    pkga: Vec<u8>,
    probe: Vec<u8>,
    stray: Vec<u8>,
}

/// The built toolchain artefacts, or `None` (said loudly) without the
/// toolchain.
fn built() -> Option<&'static Built> {
    static BUILT: OnceLock<Option<Built>> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let runtime = wasm_link::real_link_runtime()?;
            let bytes = std::fs::read(&runtime).expect("read the reserved runtime");
            let runtime = WasmRuntime::new(&bytes).expect("the runtime compiles under wasmtime");
            let pkga = side_module(&repo().join("rust/tcl-cshim/tests/c/pkga.c"), "Pkga");
            let probe = side_module(&written("probe.c", PROBE), "Probe");
            let stray = side_module(&written("stray.c", STRAY), "Stray");
            Some(Built {
                runtime,
                pkga,
                probe,
                stray,
            })
        })
        .as_ref()
}

fn repo() -> PathBuf {
    wasm_link::workspace_root()
}

/// `source` written to a scratch file named `name`.
fn written(name: &str, source: &str) -> PathBuf {
    let path = wasm_link::scratch(name);
    std::fs::write(&path, source).expect("write the extension's source");
    path
}

/// `source` built for `wasm32` against the header's WASM leg, as a side module
/// exporting `PREFIX_Init`.
fn side_module(source: &Path, prefix: &str) -> Vec<u8> {
    let sdk = wasm_link::wasi_sdk_root().expect("the gate checked wasi-sdk");
    let name = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("a file name");
    let object = wasm_link::scratch(&format!("{name}-side.o"));
    let module = wasm_link::scratch(&format!("{name}-side.wasm"));
    let compiled = Command::new(sdk.join("bin/clang"))
        .args([
            "--target=wasm32-wasip1",
            "-fPIC",
            "-O2",
            "-DTCL_HOST_WASM",
            "-I",
        ])
        .arg(repo().join("runtime/rust/include"))
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(&object)
        .output()
        .expect("run clang");
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let linked = Command::new(sdk.join("bin/wasm-ld"))
        .args([
            "--experimental-pic",
            "-shared",
            "--no-entry",
            "--import-memory",
            "--import-table",
            "--allow-undefined",
        ])
        .arg(format!("--export={prefix}_Init"))
        .arg("-o")
        .arg(&module)
        .arg(&object)
        .arg(sdk.join("share/wasi-sysroot/lib/wasm32-wasip1/libc.a"))
        .output()
        .expect("run wasm-ld");
    assert!(
        linked.status.success(),
        "{}",
        String::from_utf8_lossy(&linked.stderr)
    );
    let bytes = std::fs::read(&module).expect("read the side module");
    let _ = std::fs::remove_file(object);
    let _ = std::fs::remove_file(module);
    bytes
}

/// The engine the shared cases run on: the runtime's, under wasmtime.
fn wasm_engine() -> Option<impl Fn() -> WasmEngine> {
    let Some(built) = built() else {
        eprintln!("SKIPPING the WASM engine's cases: no real-link toolchain");
        return None;
    };
    Some(|| WasmEngine::new(&built.runtime).expect("an instance"))
}

engine_cases::engine_cases!(wasm_engine);

fn unit<'a>(parameters: &'a [&'a str], body: &'a str) -> CompileUnit<'a> {
    CompileUnit {
        name: "test",
        parameters,
        body,
    }
}

/// A WASM engine with `pkga` loaded.
fn engine_with_pkga(built: &Built) -> WasmEngine {
    let mut engine = WasmEngine::new(&built.runtime).expect("an instance");
    let pkga = built
        .runtime
        .extension(&built.pkga, "Pkga")
        .expect("pkga is a side module");
    engine.load_extension(&pkga).expect("Pkga_Init runs");
    engine
}

/// Evaluate `body` once on `engine`, answering its value's text.
fn run(engine: &mut WasmEngine, body: &str) -> Result<String, EngineError> {
    let handle = engine.compile(unit(&[], body)).expect("compiles");
    engine
        .invoke(&handle, &[])
        .map(|value| value.as_str().expect("a string").to_owned())
}

/// The extension evaluated under WASM with fuel: its commands answer, a body
/// over its command budget is `BudgetExceeded` whether it dispatches commands
/// or none (the fuel stands in for the count), never a hang, and the engine is
/// rebuilt as it was after the trap that stopped it.
#[test]
fn pkga_evaluates_under_wasm_with_fuel() {
    let Some(built) = built() else {
        eprintln!("SKIPPING pkga_evaluates_under_wasm_with_fuel");
        return;
    };
    let mut engine = engine_with_pkga(built);
    engine
        .set_budget(Budget::of_commands(500).with_wall_clock(Duration::from_secs(20)))
        .expect("the WASM engine enforces both");
    assert_eq!(
        run(&mut engine, "return [pkga_calc add 1 2]"),
        Ok("3".to_owned())
    );
    assert_eq!(
        run(&mut engine, "pkga_count; pkga_count; return [pkga_count]"),
        Ok("3".to_owned())
    );
    let started = Instant::now();
    assert_eq!(
        run(&mut engine, "while 1 {pkga_calc add 1 2}"),
        Err(EngineError::BudgetExceeded(BudgetKind::Commands)),
        "the interpreter's count stops a body that dispatches"
    );
    assert_eq!(
        run(&mut engine, "pkga_calc range 2000000000"),
        Err(EngineError::BudgetExceeded(BudgetKind::Commands)),
        "fuel stops a C command's own loop, which dispatches nothing"
    );
    assert!(
        started.elapsed() < Duration::from_secs(60),
        "never a hang: {:?}",
        started.elapsed()
    );
    // The trap left the instance unusable; the next call has a fresh one with
    // the extension loaded again, so its commands answer and its count restarts.
    assert_eq!(
        run(&mut engine, "return [pkga_calc add 40 2]"),
        Ok("42".to_owned())
    );
    assert_eq!(run(&mut engine, "return [pkga_count]"), Ok("1".to_owned()));
}

/// Memory an evaluation grows past what its value-size budget allows traps as
/// that budget, and so does a C command's: no count sees either allocation.
#[test]
fn growing_the_memory_past_the_value_size_budget_is_that_budget() {
    let Some(built) = built() else {
        eprintln!("SKIPPING growing_the_memory_past_the_value_size_budget_is_that_budget");
        return;
    };
    let mut engine = engine_with_pkga(built);
    engine
        .set_budget(Budget::of_commands(1_000_000_000).with_max_value_bytes(1024))
        .expect("the WASM engine enforces both");
    for body in ["lrepeat 10000000 x", "pkga_calc range 2000000000"] {
        assert_eq!(
            run(&mut engine, body),
            Err(EngineError::BudgetExceeded(BudgetKind::ValueSize)),
            "{body}"
        );
    }
    assert_eq!(
        run(&mut engine, "llength [lrepeat 1000 x]"),
        Ok("1000".to_owned())
    );
}

/// Every vector `tclsh9.0` answered for `pkga.c` is answered the same under
/// WASM, but the two the runtime answers differently natively too, which it
/// answers as it does natively.
#[test]
fn every_vector_answers_under_wasm_as_natively() {
    let Some(built) = built() else {
        eprintln!("SKIPPING every_vector_answers_under_wasm_as_natively");
        return;
    };
    let mut engine = engine_with_pkga(built);
    let handle = engine
        .compile(unit(
            &["script"],
            "set code [uplevel #0 [list catch $script ::r]]\n\
             if {$code == 0} { return [list $code $::r {}] }\n\
             return [list $code $::r $::errorCode]",
        ))
        .expect("compiles");
    let diverges = |script: &str| DIVERGENCES.iter().any(|row| row.0 == script);
    let rows = CASES
        .iter()
        .filter(|row| !diverges(row.0))
        .chain(DIVERGENCES);
    let mut differ = Vec::new();
    for &(script, code, result, error_code) in rows {
        let answer = engine
            .invoke(&handle, &[Value::string(script)])
            .expect("the vector runs");
        let answer =
            tcl_syntax::list::split_list(answer.as_str().expect("a list")).expect("a list");
        let got = (
            answer[0].parse::<i64>().expect("a code"),
            answer[1].to_string(),
            answer[2].to_string(),
        );
        let wanted = (code, result.to_owned(), error_code.to_owned());
        if got != wanted {
            differ.push(format!(
                "{script:?}\n    wanted {wanted:?}\n    here   {got:?}"
            ));
        }
    }
    assert!(differ.is_empty(), "{}", differ.join("\n"));
}

/// A host command that defines `made` through its door.
struct Maker;

impl HostCommand for Maker {
    fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::Unsupported("a maker needs the door"))
    }

    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn tcl_engine_api::CommandRegistrar,
        _arguments: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        registrar.define_command("made", Rc::new(Echo))?;
        registrar.remove_command("doomed")?;
        Ok(Value::Empty.into())
    }
}

/// A host command that defines `late` through its door.
struct LateMaker;

impl HostCommand for LateMaker {
    fn invoke(&self, _arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::Unsupported("a maker needs the door"))
    }

    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn tcl_engine_api::CommandRegistrar,
        _arguments: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        registrar.define_command("late", Rc::new(Echo))?;
        Ok(Value::Empty.into())
    }
}

/// A host command that answers its first word.
struct Echo;

impl HostCommand for Echo {
    fn invoke(&self, arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        Ok(arguments.first().cloned().unwrap_or(Value::Empty).into())
    }
}

/// What was set up on the engine survives a trap: the instance it left
/// unusable is rebuilt with the commands defined and removed, through the
/// engine and through a door — in the evaluation the trap ended too — the
/// package provided, the whitelist and the confinement, as they were.
#[test]
fn what_was_set_up_survives_a_trap() {
    let Some(built) = built() else {
        eprintln!("SKIPPING what_was_set_up_survives_a_trap");
        return;
    };
    let mut engine = WasmEngine::new(&built.runtime).expect("an instance");
    engine
        .define_command("maker", Rc::new(Maker))
        .expect("defines");
    engine
        .define_command("doomed", Rc::new(Echo))
        .expect("defines");
    engine
        .define_command("latemaker", Rc::new(LateMaker))
        .expect("defines");
    run(&mut engine, "maker").expect("the door defines and removes");
    engine.provide_package("kept", "1.0").expect("provides");
    engine
        .restrict_commands(&[
            "return", "while", "set", "info", "llength", "package",
            // Allowed, so only the removal replayed can keep it gone.
            "doomed",
        ])
        .expect("restricts");
    engine.confine_stores().expect("confines");
    engine
        .set_budget(Budget::of_commands(1_000_000).with_wall_clock(Duration::from_millis(50)))
        .expect("budgets");
    assert_eq!(
        run(&mut engine, "latemaker\nwhile 1 {}"),
        Err(EngineError::BudgetExceeded(BudgetKind::WallClock))
    );
    engine.set_budget(Budget::default()).expect("no budget");
    assert_eq!(
        run(&mut engine, "return [late too]"),
        Ok("too".to_owned()),
        "defined through a door in the evaluation the trap ended"
    );
    assert_eq!(
        run(&mut engine, "return [made again]"),
        Ok("again".to_owned())
    );
    let refused = |answer: Result<String, EngineError>, wanted: &str| {
        answer.err().is_some_and(|error| {
            error.script_message_bytes().is_some_and(|bytes| {
                bytes
                    .windows(wanted.len())
                    .any(|part| part == wanted.as_bytes())
            })
        })
    };
    assert!(
        refused(
            run(&mut engine, "doomed x"),
            "invalid command name \"doomed\""
        ),
        "removed through the door"
    );
    assert!(
        refused(
            run(&mut engine, "lindex {a b} 0"),
            "invalid command name \"lindex\""
        ),
        "off the whitelist"
    );
    assert!(
        refused(
            run(&mut engine, "set ::g 1"),
            "stores are confined to the activation"
        ),
        "confined"
    );
    assert_eq!(
        run(&mut engine, "return [info exists ::env]"),
        Ok("0".to_owned())
    );
    assert_eq!(
        run(&mut engine, "return [package present kept]"),
        Ok("1.0".to_owned())
    );
}

/// The engine names itself and the release it pinned, a module that calls
/// `proc_exit` ends the evaluation as a crash the engine survives, and what
/// is not an extension this host can load is refused with the reason.
#[test]
fn the_wasm_engine_refuses_what_it_cannot_load() {
    let Some(built) = built() else {
        eprintln!("SKIPPING the_wasm_engine_refuses_what_it_cannot_load");
        return;
    };
    let mut engine = WasmEngine::new(&built.runtime).expect("an instance");
    assert_eq!(engine.name(), "wasm");
    assert_eq!(engine.release(), None);
    engine.set_release("tcl8.6").expect("pins");
    assert_eq!(engine.release(), Some("tcl8.6"));

    let probe = built
        .runtime
        .extension(&built.probe, "Probe")
        .expect("a side module");
    engine.load_extension(&probe).expect("Probe_Init runs");
    assert!(
        matches!(run(&mut engine, "probe_exit"), Err(EngineError::Crashed(why))
            if why.contains("proc_exit(3)")),
        "proc_exit ends the evaluation"
    );
    assert_eq!(
        run(&mut engine, "return [expr {010 + 0}]"),
        Ok("8".to_owned())
    );

    let not_side = built.runtime.extension(b"\0asm\x01\0\0\0", "Pkga");
    assert!(
        matches!(&not_side, Err(EngineError::Crashed(why)) if why.contains("not a side module")),
        "an ordinary module"
    );
    let stray = built
        .runtime
        .extension(&built.stray, "Stray")
        .expect("a side module");
    assert!(
        matches!(engine.load_extension(&stray), Err(EngineError::Crashed(why))
            if why.contains("Tcl_NoSuchFunction")),
        "a call the runtime does not export"
    );
    let misnamed = built
        .runtime
        .extension(&built.pkga, "Nope")
        .expect("a side module");
    assert!(
        matches!(engine.load_extension(&misnamed), Err(EngineError::Crashed(why))
            if why.contains("Nope_Init")),
        "an entry point it does not define"
    );
    assert_eq!(
        run(&mut engine, "return [expr {010 + 0}]"),
        Ok("8".to_owned())
    );
}

/// A side module runs on a stack of its own: a command whose frame fills most
/// of it leaves the module's data, which the host placed just below it, and the
/// runtime's heap as they were.
#[test]
fn a_side_module_runs_on_a_stack_of_its_own() {
    let Some(built) = built() else {
        eprintln!("SKIPPING a_side_module_runs_on_a_stack_of_its_own");
        return;
    };
    let mut engine = WasmEngine::new(&built.runtime).expect("an instance");
    let probe = built
        .runtime
        .extension(&built.probe, "Probe")
        .expect("a side module");
    engine.load_extension(&probe).expect("Probe_Init runs");
    for _ in 0..2 {
        assert_eq!(
            run(&mut engine, "return [probe_stack]"),
            Ok("the probe's data, intact".to_owned())
        );
    }
    assert_eq!(
        run(&mut engine, "return [string repeat ab 3]"),
        Ok("ababab".to_owned())
    );
}

/// An extension reaches the interpreter only through the runtime's C API: an
/// import of any other runtime export (the compiled code's ABI, `tcl_eval`, the
/// engine's own `tcl_engine_*`) is refused, and the C API has no eval or
/// variable door to import, so an extension's command can neither run a script
/// nor store a variable, nor lift its own budget.
#[test]
fn an_extension_reaches_the_runtime_only_through_its_c_api() {
    let Some(built) = built() else {
        eprintln!("SKIPPING an_extension_reaches_the_runtime_only_through_its_c_api");
        return;
    };
    let mut engine = WasmEngine::new(&built.runtime).expect("an instance");
    for (index, (name, why)) in [
        ("tcl_eval", "which is not the runtime's C API"),
        ("tcl_engine_set_limits", "which is not the runtime's C API"),
        ("tcl_codegen_var_set", "which is not the runtime's C API"),
        ("Tcl_EvalObjEx", "which the runtime does not export"),
        ("Tcl_GetVar2Ex", "which the runtime does not export"),
        ("Tcl_ObjSetVar2", "which the runtime does not export"),
        ("Tcl_UnsetVar2", "which the runtime does not export"),
    ]
    .into_iter()
    .enumerate()
    {
        let source = format!(
            "extern int {name}(void);\nint Door_Init(void *interp) {{ return {name}(); }}\n"
        );
        let module = side_module(&written(&format!("door{index}.c"), &source), "Door");
        let door = built
            .runtime
            .extension(&module, "Door")
            .expect("a side module");
        let refused = engine.load_extension(&door);
        assert!(
            matches!(&refused, Err(EngineError::Crashed(message))
                if message.contains(name) && message.contains(why)),
            "{name}: {refused:?}"
        );
    }
}

/// The first-use fuel is an instance's first evaluation's: a later evaluation
/// that is the first to build the release's command tables pays for them from
/// its own fuel, and an instance rebuilt after a trap grants it again.
#[test]
fn the_first_use_fuel_is_an_instance_s_first_evaluation_s() {
    let Some(built) = built() else {
        eprintln!("SKIPPING the_first_use_fuel_is_an_instance_s_first_evaluation_s");
        return;
    };
    let budget = Budget::of_commands(5);
    let mut first = WasmEngine::new(&built.runtime).expect("an instance");
    first.set_budget(budget).expect("budgets");
    assert_eq!(run(&mut first, "string length abc"), Ok("3".to_owned()));

    let mut later = WasmEngine::new(&built.runtime).expect("an instance");
    later.set_budget(budget).expect("budgets");
    assert_eq!(run(&mut later, "return 1"), Ok("1".to_owned()));
    assert_eq!(
        run(&mut later, "string length abc"),
        Err(EngineError::BudgetExceeded(BudgetKind::Commands)),
        "the tables are built on the second evaluation's own fuel"
    );
    assert_eq!(
        run(&mut later, "string length abc"),
        Ok("3".to_owned()),
        "the instance rebuilt after the trap grants the first-use fuel again"
    );
}

/// What is set up between evaluations runs under none of their limits: a
/// deadline the last evaluation armed has long passed when the next command is
/// defined and the next extension loaded.
#[test]
fn what_is_set_up_between_evaluations_is_under_no_budget() {
    let Some(built) = built() else {
        eprintln!("SKIPPING what_is_set_up_between_evaluations_is_under_no_budget");
        return;
    };
    let mut engine = WasmEngine::new(&built.runtime).expect("an instance");
    engine
        .set_budget(Budget::of_commands(100_000).with_wall_clock(Duration::from_millis(30)))
        .expect("budgets");
    assert_eq!(run(&mut engine, "return 1"), Ok("1".to_owned()));
    std::thread::sleep(Duration::from_millis(100));
    engine
        .define_command("echo", Rc::new(Echo))
        .expect("defines");
    let pkga = built
        .runtime
        .extension(&built.pkga, "Pkga")
        .expect("pkga is a side module");
    engine.load_extension(&pkga).expect("Pkga_Init runs");
    engine.set_budget(Budget::default()).expect("no budget");
    assert_eq!(
        run(&mut engine, "return [echo [pkga_calc add 1 2]]"),
        Ok("3".to_owned())
    );
}

/// `text` split into its words at each space.
fn words(text: &str) -> Vec<String> {
    text.split(' ').map(str::to_owned).collect()
}

/// The registry's extension seam bound to the WASM host: the artefact is named
/// by its content hash, its commands are what its entry point registered, an
/// evaluation answers on a fresh instance each time (so two runs agree), an
/// error is never a value, what the host cannot load is refused, and without a
/// host installed every load and evaluation is unavailable.
#[test]
fn the_extension_host_evaluates_on_a_fresh_instance_each_time() {
    let Some(built) = built() else {
        eprintln!("SKIPPING the_extension_host_evaluates_on_a_fresh_instance_each_time");
        return;
    };
    clear_extension_host();
    let budget = ImplementationBudget::default();
    assert_eq!(
        load_extension(&built.pkga, "Pkga"),
        Err(DeclineReason::Transient)
    );
    assert_eq!(
        evaluate_extension(
            artefact_hash(&built.pkga),
            &words("pkga_calc add 1 2"),
            &budget
        ),
        Err(DeclineReason::Transient),
        "no host installed"
    );

    install_extension_host(Rc::new(WasmExtensionHost::new(&built.runtime)));
    let loaded = load_extension(&built.pkga, "Pkga").expect("pkga loads");
    assert_eq!(loaded.hash, artefact_hash(&built.pkga));
    assert_eq!(loaded.prefix, "Pkga");
    assert_eq!(
        loaded.commands,
        [
            "pkga_calc",
            "pkga_count",
            "pkga_eq",
            "pkga_forget",
            "pkga_quote"
        ]
    );
    let evaluate = |text: &str| evaluate_extension(loaded.hash, &words(text), &budget);
    assert_eq!(evaluate("pkga_calc add 1 2"), Ok("3".to_owned()));
    assert_eq!(evaluate("pkga_count"), Ok("1".to_owned()));
    assert_eq!(
        evaluate("pkga_count"),
        Ok("1".to_owned()),
        "a fresh instance"
    );
    assert_eq!(
        evaluate("pkga_calc nosuch 1"),
        Err(DeclineReason::Unsupported),
        "an error is never a value"
    );
    assert_eq!(
        evaluate_extension(artefact_hash(b"not loaded"), &words("pkga_count"), &budget),
        Err(DeclineReason::Transient),
        "an artefact the host has not loaded"
    );
    assert_eq!(
        load_extension(b"\0asm\x01\0\0\0", "Pkga"),
        Err(DeclineReason::Unsupported),
        "an artefact that is not a side module"
    );
    assert_eq!(
        load_extension(&built.stray, "Stray"),
        Err(DeclineReason::Unsupported),
        "an extension that calls what the runtime does not export"
    );
    clear_extension_host();
}

/// Each budget an evaluation outruns is its own decline: the command count and
/// the fuel that stands in for it, the wall clock, the memory's growth, and a
/// result over the value size; under them all, the same command answers.
#[test]
fn each_budget_an_evaluation_outruns_is_its_own_decline() {
    let Some(built) = built() else {
        eprintln!("SKIPPING each_budget_an_evaluation_outruns_is_its_own_decline");
        return;
    };
    // A host whose own budget stops nothing these evaluations reach: a wall
    // clock long enough that only an evaluation's own stops a loop, and no
    // value size, so a C loop that allocates meets its fuel before any cap.
    let host = WasmExtensionHost::with_budget(
        &built.runtime,
        Budget::of_commands(100_000).with_wall_clock(Duration::from_secs(120)),
    );
    let loaded = host.load(&built.pkga, "Pkga").expect("pkga loads");
    let budget = ImplementationBudget::default();
    for (limited, text, limit) in [
        (
            ImplementationBudget {
                commands: Some(0),
                ..budget
            },
            "pkga_calc add 1 2",
            BudgetLimit::Fuel,
        ),
        (
            ImplementationBudget {
                commands: Some(10),
                ..budget
            },
            "pkga_calc range 2000000000",
            BudgetLimit::Fuel,
        ),
        (
            ImplementationBudget {
                wall_clock_ms: Some(20),
                ..budget
            },
            "pkga_calc range 2000000000",
            BudgetLimit::Request,
        ),
        (
            ImplementationBudget {
                value_bytes: Some(1024),
                ..budget
            },
            "lrepeat 10000000 x",
            BudgetLimit::AllocationBytes,
        ),
        (
            ImplementationBudget {
                value_bytes: Some(64),
                ..budget
            },
            "pkga_calc range 1000",
            BudgetLimit::ResultBytes,
        ),
    ] {
        assert_eq!(
            host.evaluate(loaded.hash, &words(text), &limited),
            Err(DeclineReason::Budget(limit)),
            "{text}"
        );
    }
    let every = ImplementationBudget {
        commands: Some(10),
        wall_clock_ms: Some(1000),
        value_bytes: Some(64),
    };
    assert_eq!(
        host.evaluate(loaded.hash, &words("pkga_calc range 10"), &every),
        Ok("0 1 2 3 4 5 6 7 8 9".to_owned())
    );
}

/// What a module reads of the clock, randomness and the environment is what
/// the host's stubs hand every module: nothing of the machine, the same on
/// every run.
#[test]
fn an_extension_reads_nothing_of_the_machine() {
    let Some(built) = built() else {
        eprintln!("SKIPPING an_extension_reads_nothing_of_the_machine");
        return;
    };
    let host = WasmExtensionHost::new(&built.runtime);
    let loaded = host.load(&built.probe, "Probe").expect("the probe loads");
    let budget = ImplementationBudget::default();
    let first = host.evaluate(loaded.hash, &["probe_ambient".to_owned()], &budget);
    let second = host.evaluate(loaded.hash, &["probe_ambient".to_owned()], &budget);
    assert_eq!(
        first,
        Ok("0 0 0".to_owned()),
        "a zero clock, zero randomness, no environment"
    );
    assert_eq!(first, second, "two runs agree");
}

#[test]
fn installed_receipts_keep_renamed_hosts_and_remove_old_name_replacements() {
    // Software integration: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let Some(built) = built() else {
        return;
    };
    let mut engine = WasmEngine::new(&built.runtime).unwrap();
    engine.define_command("keeper", Rc::new(Echo)).unwrap();
    run(
        &mut engine,
        "rename ::keeper ::moved; proc ::keeper {} {set ::replacement ran}; return READY",
    )
    .unwrap();
    engine
        .restrict_commands(&["set", "info", "list", "return"])
        .unwrap();
    assert_eq!(
        run(
            &mut engine,
            "list [::moved ORIGINAL] [info commands ::keeper] [info exists ::replacement]"
        ),
        Ok("ORIGINAL {} 0".to_owned())
    );
}

#[test]
fn compiled_receipts_refuse_foreign_and_replaced_handles_before_effects() {
    // Software integration: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let Some(built) = built() else {
        return;
    };
    let mut engine = WasmEngine::new(&built.runtime).unwrap();
    let handle = engine
        .compile(CompileUnit {
            name: "receipt",
            parameters: &[],
            body: "set ::original ran",
        })
        .unwrap();
    let mut foreign = WasmEngine::new(&built.runtime).unwrap();
    assert!(matches!(
        foreign.invoke(&handle, &[]),
        Err(EngineError::ExecutionRefusal(_))
    ));
    assert_eq!(
        run(&mut foreign, "info exists ::original"),
        Ok("0".to_owned())
    );
    run(&mut engine, "rename ::spectcl::unit::1 ::original_body; proc ::spectcl::unit::1 {} {set ::replacement ran}").unwrap();
    assert!(matches!(
        engine.invoke(&handle, &[]),
        Err(EngineError::ExecutionRefusal(_))
    ));
    assert_eq!(
        run(
            &mut engine,
            "list [info exists ::original] [info exists ::replacement]"
        ),
        Ok("0 0".to_owned())
    );
}

struct OpaqueCompletion;
impl HostCommand for OpaqueCompletion {
    fn invoke(&self, arguments: &[Value]) -> Result<HostOutcome, EngineError> {
        if arguments.first().and_then(Value::as_str) == Some("error") {
            return Err(EngineError::ScriptBytes {
                message: b"ERR\0\xff".to_vec(),
                code: Some(b"HOST OPAQUE".to_vec()),
                options: Some(
                    b"-code 1 -level 0 -errorcode {HOST OPAQUE} -errorinfo {opaque\0\xff}".to_vec(),
                ),
            });
        }
        Ok(HostOutcome::ok(Value::string_bytes(
            b"RESULT\0\xff".as_slice(),
        )))
    }
}

#[test]
fn counted_publication_and_guest_completions_preserve_opaque_bytes() {
    // Software integration: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let Some(built) = built() else {
        return;
    };
    let mut engine = WasmEngine::new(&built.runtime).unwrap();
    engine
        .define_command_bytes("café\0unused".as_bytes(), Rc::new(OpaqueCompletion))
        .unwrap();
    let ok = engine
        .compile(CompileUnit {
            name: "opaque",
            parameters: &[],
            body: "::café",
        })
        .unwrap();
    assert_eq!(
        engine.invoke(&ok, &[]).unwrap().as_bytes(),
        Some(b"RESULT\0\xff".as_slice())
    );
    let error = engine
        .compile(CompileUnit {
            name: "opaque error",
            parameters: &[],
            body: "::café error",
        })
        .unwrap();
    let error = engine.invoke(&error, &[]).unwrap_err();
    assert_eq!(error.script_message_bytes(), Some(b"ERR\0\xff".as_slice()));
    assert_eq!(error.script_code_bytes(), Some(b"HOST OPAQUE".as_slice()));
    assert!(
        error
            .script_options_bytes()
            .unwrap()
            .windows(8)
            .any(|bytes| bytes == b"opaque\0\xff")
    );
}

struct RefusingHost;
impl HostCommand for RefusingHost {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::ExecutionRefusal(
            "actual host contract refusal".into(),
        ))
    }
}

#[test]
fn host_refusal_bypasses_guest_catch_and_keeps_prior_effects() {
    // docs/design/analysis/name-resolution-proofs/interpreter-original-child-host-refusal-transport.md

    // Software integration: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // naming.interpreter.original-child-host-refusal-transport
    let Some(built) = built() else {
        return;
    };
    let mut engine = WasmEngine::new(&built.runtime).unwrap();
    engine
        .define_command("refuse", Rc::new(RefusingHost))
        .unwrap();
    assert!(matches!(
        run(
            &mut engine,
            "set ::before reached; catch {refuse} result; set ::after caught"
        ),
        Err(EngineError::ExecutionRefusal(_))
    ));
    assert_eq!(
        run(&mut engine, "list $::before [info exists ::after]"),
        Ok("reached 0".to_owned())
    );
}

#[test]
fn structured_binary_and_scalar_inputs_use_selected_original_producers() {
    // Software transport/integration: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let Some(built) = built() else {
        return;
    };
    let mut engine = WasmEngine::new(&built.runtime).unwrap();
    engine.set_release("tcl8.6").unwrap();
    let binary = engine
        .compile(CompileUnit {
            name: "binary",
            parameters: &["x"],
            body: "binary encode hex $x",
        })
        .unwrap();
    assert_eq!(
        engine
            .invoke(&binary, &[Value::byte_array(b"\xff\0A".as_slice())])
            .unwrap()
            .as_str(),
        Some("ff0041")
    );
    let resident = engine
        .compile(CompileUnit {
            name: "resident",
            parameters: &["x"],
            body: "set x",
        })
        .unwrap();
    let original = Value::NativeScalar(tcl_engine_api::NativeScalarCache::Integer(16))
        .with_resident_string_storage(
            b"0x10".as_slice(),
            tcl_engine_api::NativeStringStorageIdentity::Allocated,
        );
    assert_eq!(
        engine.invoke(&resident, &[original]).unwrap().as_bytes(),
        Some(b"0x10".as_slice())
    );
    let foreign = Value::NativeScalar(tcl_engine_api::NativeScalarCache::WordBoolean {
        value: true,
        origin: tcl_engine_api::NativeCVersion::V8_5,
    });
    assert!(matches!(
        engine.invoke(&resident, &[foreign]),
        Err(EngineError::ExecutionRefusal(_))
    ));
    assert_eq!(
        engine
            .invoke(&resident, &[Value::string("NEXT")])
            .unwrap()
            .as_str(),
        Some("NEXT")
    );
}

struct OriginalView(std::rc::Rc<std::cell::Cell<usize>>);
impl HostCommand for OriginalView {
    fn argument_view(&self) -> tcl_engine_api::HostArgumentView {
        tcl_engine_api::HostArgumentView::OriginalObjects
    }
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        self.0.set(self.0.get() + 1);
        Ok(HostOutcome::ok(Value::string("UNREACHED")))
    }
}

#[test]
fn original_object_callbacks_refuse_before_host_invocation_without_fake_snapshots() {
    // Software limitation: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let Some(built) = built() else {
        return;
    };
    let mut engine = WasmEngine::new(&built.runtime).unwrap();
    let reached = Rc::new(std::cell::Cell::new(0));
    engine
        .define_command("original_view", Rc::new(OriginalView(Rc::clone(&reached))))
        .unwrap();
    assert!(matches!(
        run(
            &mut engine,
            "set ::prior reached; catch {original_view x}; set ::later ran"
        ),
        Err(EngineError::ExecutionRefusal(_))
    ));
    assert_eq!(reached.get(), 0);
    assert_eq!(
        run(&mut engine, "list $::prior [info exists ::later]"),
        Ok("reached 0".to_owned())
    );
}

#[test]
fn unicode_extension_inventory_refuses_an_opaque_counted_command_name() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let Some(built) = built() else {
        return;
    };
    // The C source contains an explicit \xff byte escape, not U+00FF.
    let source = written(
        "opaque-inventory426.c",
        r#"
#include "tcl.h"
static int Opaque(ClientData data, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
    (void)data; (void)interp; (void)objc; (void)objv;
    return TCL_OK;
}
int OpaqueInventory_Init(Tcl_Interp *interp) {
    Tcl_CreateObjCommand(interp, "opaque_\xff", Opaque, NULL, NULL);
    return TCL_OK;
}
"#,
    );
    let module = side_module(&source, "OpaqueInventory");
    let host = WasmExtensionHost::new(&built.runtime);
    let result = host.load(&module, "OpaqueInventory");
    assert!(
        matches!(result, Err(DeclineReason::NotText)),
        "the Unicode catalogue must not publish a replacement identity: {result:?}"
    );
}

#[test]
fn restriction_keeps_renamed_stock_roots_without_namespace_prefix_donation() {
    // Software integration: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let Some(built) = built() else { return };
    let mut engine = WasmEngine::new(&built.runtime).unwrap();
    engine.set_release("tcl8.6").unwrap();
    run(&mut engine, "rename ::dict ::renamed_dict; proc ::tcl::dict::impostor {} {set ::impostor_ran 1}; return READY").unwrap();
    engine
        .restrict_commands(&["set", "info", "list", "return", "renamed_dict"])
        .unwrap();
    assert_eq!(
        run(
            &mut engine,
            "list [::renamed_dict size {a 1 b 2}] [info commands ::tcl::dict::impostor] [info exists ::impostor_ran]"
        ),
        Ok("2 {} 0".to_owned())
    );
}

#[test]
fn restriction_refuses_a_retired_original_host_receipt_before_table_effects() {
    // Software integration: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let Some(built) = built() else { return };
    let mut engine = WasmEngine::new(&built.runtime).unwrap();
    engine.define_command("keeper", Rc::new(Echo)).unwrap();
    run(
        &mut engine,
        "rename ::keeper {}; set ::before RETAINED; return READY",
    )
    .unwrap();
    assert!(matches!(
        engine.restrict_commands(&["return"]),
        Err(EngineError::ExecutionRefusal(_))
    ));
    // This is a fresh public entry: earlier host refusal does not remove `set`.
    assert_eq!(
        run(&mut engine, "list [set ::before] [info commands ::set]"),
        Ok("RETAINED ::set".to_owned())
    );
}
