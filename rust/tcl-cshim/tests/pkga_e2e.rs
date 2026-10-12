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

//! The real C extension (`tests/c/pkga.c`, compiled by `build.rs` against
//! `runtime/rust/include/tcl.h`) loaded into a `tcl-vm`-backed shim interpreter
//! and driven from Tcl.
//!
//! Every expected string below was captured by building the same `pkga.c`
//! against Tcl 9.0.4's own `tcl.h`, loading it into `tclsh9.0`, and
//! recording the result and `$errorCode` of each call — so these assertions
//! are byte-for-byte against C Tcl, not against Tcl's documentation.

#![cfg(cshim_c_tests)]

use std::ffi::{c_int, c_void};

use tcl_cshim::{InitProc, Interp, InterpState, StaticExtensions};
use tcl_engine_tclvm::TclVmEngine;

unsafe extern "C" {
    // Declared over `void *`: the interpreter is opaque to C, and an `extern`
    // block naming a Rust type would trip the FFI-safety lint.
    fn Pkga_Init(interp: *mut c_void) -> c_int;
}

unsafe extern "C" fn pkga_init(interp: *mut InterpState) -> c_int {
    // SAFETY: the shim passes its live interpreter pointer through.
    unsafe { Pkga_Init(interp.cast::<c_void>()) }
}

fn loaded() -> Interp<TclVmEngine> {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: `Pkga_Init` is the test extension built against the shim header.
    let loaded = unsafe { interp.load_static(pkga_init) }.expect("pkga loads");
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
    interp
}

/// The extension as a host links it: its prefix and entry point.
static TABLE: &[(&str, InitProc)] = &[("Pkga", pkga_init)];

/// A shim interpreter whose host has opted in to `load` over [`TABLE`]; nothing
/// is loaded until a script asks.
fn bridged() -> Interp<TclVmEngine> {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: `Pkga_Init` is the test extension built against the shim header.
    let extensions = unsafe { StaticExtensions::new(TABLE) };
    interp
        .enable_static_extensions(extensions)
        .expect("the engine takes the command");
    interp
}

/// Run `script` under `catch`, returning `(code, result, errorCode)` the way
/// the reference probe recorded them.
fn catching(interp: &mut Interp<TclVmEngine>, script: &str) -> (i64, String, String) {
    // Each `eval` is its own unit with its own locals, so the result goes
    // through a global.
    let code = interp
        .eval(&format!("catch {{{script}}} ::r"))
        .expect("catch runs");
    let result = interp.eval("set ::r").expect("result readable");
    let error_code = if code.as_str() == Some("0") {
        String::new()
    } else {
        interp
            .eval("set ::errorCode")
            .expect("errorCode readable")
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };
    (
        code.as_str().and_then(|c| c.parse().ok()).expect("a code"),
        result.as_str().unwrap_or_default().to_owned(),
        error_code,
    )
}

/// The loaded report bridges to the analyser's vocabulary: every command the
/// real extension registered is declared, at the conservative default for a
/// command native code registers and at no narrower fact, and the name of each
/// is the one the shell would answer to.
#[test]
fn the_loaded_report_bridges_to_the_default_fact() {
    use tcl_registry::{CommandSpec, Traits};

    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: `Pkga_Init` is the test extension built against the shim header.
    let loaded = unsafe { interp.load_static(pkga_init) }.expect("pkga loads");
    let declared = loaded.declared_surface();
    let names: Vec<&str> = declared.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        loaded
            .commands
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
    assert!(!declared.is_empty(), "pkga registers commands");
    let default = CommandSpec::extension_default("pkga_calc");
    for command in &declared {
        assert_eq!(command.traits, default.traits, "{}", command.name);
        assert_eq!(
            command.side_effects, default.side_effects,
            "{}",
            command.name
        );
        // What every consumer asks of the declared command: a taint sink, hidden
        // in a safe interpreter, never pure.
        assert!(
            command.traits.contains(Traits::TAINT_SINK),
            "{}",
            command.name
        );
        assert!(
            command.traits.contains(Traits::SAFE_INTERP_HIDDEN),
            "{}",
            command.name
        );
        assert!(!command.traits.contains(Traits::PURE), "{}", command.name);
    }
}

#[test]
fn smoke_pkga_round_trips_through_the_vm() {
    let mut interp = loaded();
    assert_eq!(
        catching(&mut interp, "pkga_eq abc abc"),
        (0, "1".into(), String::new())
    );
    assert_eq!(
        catching(&mut interp, "pkga_calc add 5 [pkga_calc add 1 2]"),
        (0, "8".into(), String::new())
    );
}

include!("vectors/pkga.rs");

fn check_cases(interp: &mut Interp<TclVmEngine>) {
    for &(script, code, result, error_code) in CASES {
        assert_eq!(
            catching(interp, script),
            (code, result.to_owned(), error_code.to_owned()),
            "{script}"
        );
    }
}

#[test]
fn results_and_errors_match_c_tcl_byte_for_byte() {
    check_cases(&mut loaded());
}

/// A script that loads the extension itself, through the host's `load`, gets
/// the same bytes: nothing about the extension depends on who called its entry
/// point. The file name is the label a `pkgIndex.tcl` writes, and the prefix
/// Tcl would guess from it is what reaches the table.
#[test]
fn the_same_vectors_run_through_the_host_load_bridge() {
    let mut interp = bridged();
    assert_eq!(
        catching(
            &mut interp,
            "load [file join /opt/pkga libpkga[info sharedlibextension]]"
        ),
        (0, String::new(), String::new())
    );
    check_cases(&mut interp);
}

/// `load` over the host's table defines the extension's commands in the
/// engine, in the script that loads them and after it; what the table does not
/// hold is `couldn't load`, and a prefix loads once.
#[test]
fn load_through_the_host_bridge_defines_the_commands() {
    // Without the host's opt-in there is no `load` for a script to call.
    let mut bare = Interp::new(TclVmEngine::new());
    assert_eq!(
        catching(&mut bare, "load {} Pkga"),
        (
            1,
            "invalid command name \"load\"".into(),
            "TCL LOOKUP COMMAND load".into()
        )
    );

    let mut interp = bridged();
    assert!(interp.commands().is_empty(), "opting in loads nothing");
    // One script loads and calls: the registration door published the commands
    // before the next statement ran.
    let answer = interp
        .eval("load {} Pkga\npkga_eq abc abc")
        .expect("loads and calls in one script");
    assert_eq!(answer.as_str(), Some("1"));
    assert_eq!(
        interp.commands(),
        [
            "pkga_calc",
            "pkga_count",
            "pkga_eq",
            "pkga_forget",
            "pkga_quote"
        ]
    );
    assert_eq!(
        interp.provided_packages(),
        [("pkga".to_owned(), "1.0".to_owned())]
    );

    // The negatives: a name the table does not hold, by file and by prefix.
    assert_eq!(
        catching(&mut interp, "load nosuch"),
        (
            1,
            "couldn't load file \"nosuch\": no extension with the prefix \"Nosuch\" is linked \
             into this program"
                .into(),
            "NONE".into()
        )
    );
    assert_eq!(
        catching(&mut interp, "load {} Nosuch"),
        (
            1,
            "no library with prefix \"Nosuch\" is loaded statically".into(),
            "TCL OPERATION LOAD NOTSTATIC".into()
        )
    );

    // A prefix loads once per interpreter: the entry point does not run again,
    // so a command the extension deleted stays deleted.
    assert_eq!(
        catching(&mut interp, "pkga_forget"),
        (0, "1".into(), String::new())
    );
    assert_eq!(
        catching(&mut interp, "load ./libpkga.so Pkga"),
        (0, String::new(), String::new())
    );
    assert_eq!(
        catching(&mut interp, "pkga_count"),
        (
            1,
            "invalid command name \"pkga_count\"".into(),
            "TCL LOOKUP COMMAND pkga_count".into()
        )
    );
}

/// The shim's own bundle is the same extension a host would link, and loading
/// it through the engine's `define_command` directly, without an `Interp`, is
/// the other way a host opts in.
#[test]
fn the_bundled_extension_loads_through_a_command_a_host_registers_itself() {
    use std::rc::Rc;

    use tcl_engine_api::{CompileUnit, Engine};

    let mut engine = TclVmEngine::new();
    engine
        .define_command("load", Rc::new(StaticExtensions::bundled()))
        .expect("registers");
    let handle = engine
        .compile(CompileUnit {
            name: "script",
            parameters: &[],
            body: "load {} Pkga\npkga_calc add 2 3",
        })
        .expect("compiles");
    let answer = engine.invoke(&handle, &[]).expect("loads and calls");
    assert_eq!(answer.as_str(), Some("5"));
}

/// The unchanged `ifneeded` script a `pkgIndex.tcl` writes for a library with a
/// plain `load`: the entry point provides the package, so `package require`
/// finds it, and a second `package require` is satisfied from the package
/// database without the entry point running again. The expected answers are
/// `tclsh9.0`'s for the same `pkga.c` built against Tcl 9.0.4's own `tcl.h`: both
/// requires answer `1.0`, `pkga_forget` answers 1 and deletes `pkga_count`, and
/// that command stays deleted because a second run of the entry point would have
/// created it again; `info loaded` lists the library under the file `load` was
/// given.
#[test]
fn a_package_whose_ifneeded_script_is_a_plain_load_is_required_twice() {
    let mut interp = bridged();
    let extension = interp
        .eval("info sharedlibextension")
        .expect("the extension")
        .as_str()
        .unwrap_or_default()
        .to_owned();
    interp
        .eval(
            "package ifneeded pkga 1.0 \
             [list load [file join /opt/pkga libpkga[info sharedlibextension]] Pkga]",
        )
        .expect("registers the ifneeded script");
    assert_eq!(
        catching(&mut interp, "package provide pkga"),
        (0, String::new(), String::new()),
        "nothing provides it before the first require"
    );

    let listed = format!("{{/opt/pkga/libpkga{extension} Pkga}}");
    assert_eq!(
        catching(&mut interp, "package require pkga"),
        (0, "1.0".into(), String::new()),
        "the first require runs the load, and the entry point provides the package"
    );
    assert_eq!(
        catching(&mut interp, "package provide pkga"),
        (0, "1.0".into(), String::new())
    );
    assert_eq!(
        catching(&mut interp, "info loaded"),
        (0, listed.clone(), String::new())
    );

    let gone = (
        1,
        "invalid command name \"pkga_count\"".to_owned(),
        "TCL LOOKUP COMMAND pkga_count".to_owned(),
    );
    assert_eq!(
        catching(&mut interp, "pkga_forget"),
        (0, "1".into(), String::new())
    );
    assert_eq!(catching(&mut interp, "pkga_count"), gone);

    assert_eq!(
        catching(&mut interp, "package require pkga"),
        (0, "1.0".into(), String::new()),
        "the second is satisfied from the package database"
    );
    assert_eq!(
        catching(&mut interp, "pkga_count"),
        gone,
        "the entry point did not run again, or it would have created the command"
    );
    assert_eq!(
        catching(&mut interp, "info loaded"),
        (0, listed, String::new()),
        "and the library is listed once"
    );
}

/// The negative of the require above: an `ifneeded` script whose `load` the
/// host's table refuses leaves `package require` an error that carries the
/// load's, as it does in C Tcl, where the error is the file's `couldn't load
/// file "…": …`; nothing is provided and nothing is listed as loaded.
#[test]
fn an_ifneeded_script_whose_load_the_table_refuses_leaves_the_require_an_error() {
    let mut interp = bridged();
    interp
        .eval("package ifneeded pkgz 1.0 {load /opt/pkgz/libpkgz.so Pkgz}")
        .expect("registers the ifneeded script");
    let (code, message, _) = catching(&mut interp, "package require pkgz");
    assert_eq!(code, 1);
    assert!(
        message.starts_with("couldn't load file \"/opt/pkgz/libpkgz.so\": "),
        "{message}"
    );
    assert_eq!(
        catching(&mut interp, "package provide pkgz"),
        (0, String::new(), String::new())
    );
    assert_eq!(
        catching(&mut interp, "info loaded"),
        (0, String::new(), String::new())
    );
}

/// What `tclsh9.0` does when the package is already provided at another version:
/// the entry point's `Tcl_PkgProvide` fails with the package command's own error,
/// the entry point returns it before it creates a command, and `load` fails with
/// it, so nothing the extension would have defined exists.
#[test]
fn a_package_provided_at_another_version_fails_the_entry_point_before_it_registers_anything() {
    let mut interp = bridged();
    interp
        .eval("package provide pkga 2.0")
        .expect("provides another version");
    let conflict = (
        1,
        "conflicting versions provided for package \"pkga\": 2.0, then 1.0".to_owned(),
        "TCL PACKAGE VERSIONCONFLICT".to_owned(),
    );
    assert_eq!(
        catching(&mut interp, "load /opt/pkga/libpkga.so Pkga"),
        conflict
    );
    assert_eq!(
        catching(&mut interp, "pkga_eq a a"),
        (
            1,
            "invalid command name \"pkga_eq\"".into(),
            "TCL LOOKUP COMMAND pkga_eq".into()
        ),
        "the entry point returned before it created a command"
    );
    assert_eq!(
        catching(&mut interp, "package provide pkga"),
        (0, "2.0".into(), String::new())
    );
    assert_eq!(
        catching(&mut interp, "load /opt/pkga/libpkga.so Pkga"),
        conflict,
        "the load was not marked done, so a later one runs the entry point again"
    );
}

#[test]
fn client_data_and_delete_procs_work_across_calls() {
    let mut interp = loaded();
    // The counter is a C static shared by every interpreter in the process,
    // so only the increment between two calls is this test's to assert.
    let first: i64 = catching(&mut interp, "pkga_count")
        .1
        .parse()
        .expect("a count");
    assert_eq!(
        catching(&mut interp, "pkga_count"),
        (0, (first + 1).to_string(), String::new())
    );
    assert_eq!(
        catching(&mut interp, "pkga_count extra"),
        (
            1,
            "wrong # args: should be \"pkga_count\"".into(),
            "TCL WRONGARGS".into()
        )
    );
    assert_eq!(
        catching(&mut interp, "pkga_forget"),
        (0, "1".into(), String::new())
    );
    assert_eq!(
        catching(&mut interp, "pkga_count"),
        (
            1,
            "invalid command name \"pkga_count\"".into(),
            "TCL LOOKUP COMMAND pkga_count".into()
        )
    );
    assert_eq!(
        catching(&mut interp, "pkga_forget"),
        (0, "0".into(), String::new())
    );
    assert_eq!(
        interp.commands(),
        ["pkga_calc", "pkga_eq", "pkga_forget", "pkga_quote"].map(tcl_core_types::NameBytes::from)
    );
}

#[test]
fn provided_packages_are_recorded_shim_side() {
    let interp = loaded();
    assert_eq!(
        interp.provided_packages(),
        [(tcl_core_types::NameBytes::from("pkga"), b"1.0".to_vec())]
    );
}
