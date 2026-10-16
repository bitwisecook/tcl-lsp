// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `rename`, `return` and `source` as each release has them: which error
//! `rename` reports first and the `errorCode` it sets, the options `return`
//! takes, and how `source` settles a `return` and records an error's file.
//! The expectations are tclsh 8.4.20 through 9.1.0 running each script as a
//! file, and are checked against those releases when they are installed.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::{Code, Vm};

const RELEASES: [TclVersion; 5] = [
    TclVersion::V8_4,
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Run `source` as `tclsh` runs the script file it is given, under `version`.
fn run_file(version: TclVersion, source: &str) -> String {
    let output = Capture::default();
    let mut vm = Vm::with_output(Box::new(output.clone()));
    let profile = tcl_registry::model::resolve_environment(version.dialect_name()).unit_profile();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let completion = vm.eval_source_directly(source).expect("source compiles");
    assert_eq!(
        completion.code,
        Code::Ok,
        "{version:?}: {}",
        completion.result.to_str()
    );
    String::from_utf8(output.0.borrow().clone()).expect("UTF-8 output")
}

/// What `version`'s tclsh prints running the script at `path`, when it is
/// installed.
fn oracle(version: TclVersion, path: &std::path::Path) -> Option<String> {
    let tclsh = tcl_test_support::locate_tclsh(version).expect("the oracle is valid")?;
    let output = std::process::Command::new(&tclsh.path)
        .arg(path)
        .output()
        .expect("runs tclsh");
    Some(String::from_utf8(output.stdout).expect("UTF-8 output"))
}

/// A directory of its own for one test's scripts.
fn scratch(test: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rename-return-source-{}-{test}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("creates the scratch directory");
    dir
}

/// Check `source` (written to `main.tcl` in `dir`) against `expected` for
/// each release, and against each installed tclsh.
fn check(dir: &std::path::Path, source: &str, expected: impl Fn(TclVersion) -> String) {
    let path = dir.join("main.tcl");
    std::fs::write(&path, source).expect("writes the script");
    for version in RELEASES {
        let want = expected(version);
        assert_eq!(run_file(version, source), want, "{version:?}");
        if let Some(shell) = oracle(version, &path) {
            assert_eq!(shell, want, "tclsh {version:?}");
        }
    }
}

#[test]
fn rename_reports_a_missing_source_first_with_its_release_error_code() {
    let dir = scratch("rename");
    let source = "proc a {} {}\nproc b {} {}\n\
                  puts [catch {rename nope b} m]-$m-$errorCode\n\
                  puts [catch {rename nope {}} m]-$m-$errorCode\n\
                  puts [catch {rename a b} m]-$m-$errorCode\n";
    check(&dir, source, |version| {
        let (lookup, exists) = if version >= TclVersion::V8_6 {
            (
                "TCL LOOKUP COMMAND nope",
                "TCL OPERATION RENAME TARGET_EXISTS",
            )
        } else {
            ("NONE", "NONE")
        };
        format!(
            "1-can't rename \"nope\": command doesn't exist-{lookup}\n\
             1-can't delete \"nope\": command doesn't exist-{lookup}\n\
             1-can't rename to \"b\": command already exists-{exists}\n"
        )
    });
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn return_takes_the_options_its_release_has() {
    let dir = scratch("return");
    let source = "foreach s {{return a b} {return -level 2 x} {return -foo bar x} \
                  {return -code bogus x} {return -code 7 x} {return a b c}} {\n\
                  proc p {} $s\n    puts \"[catch p m] $m\"\n}\n";
    check(&dir, source, |version| {
        let bogus = "1 bad completion code \"bogus\": must be ok, error, return, break, \
                     continue, or an integer\n";
        if version == TclVersion::V8_4 {
            let bad = |option: &str| {
                format!("1 bad option \"{option}\": must be -code, -errorcode, or -errorinfo\n")
            };
            format!(
                "{}{}{}{bogus}7 x\n{}",
                bad("a"),
                bad("-level"),
                bad("-foo"),
                bad("a")
            )
        } else {
            format!("0 \n2 x\n0 x\n{bogus}7 x\n0 c\n")
        }
    });
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn source_settles_a_return_and_records_where_an_error_occurred() {
    let dir = scratch("source");
    let write = |name: &str, text: &str| {
        let path = dir.join(name);
        std::fs::write(&path, text).expect("writes the script");
        path.display().to_string()
    };
    let raises = write("raises.tcl", "return -code error boom\n");
    let early = write("early.tcl", "return -code break\n");
    let value = write("value.tcl", "return x\n");
    let errs = write("errs.tcl", "set a 1\nerror boom\n");
    let source = format!(
        "puts [catch {{source {raises}}} m]-$m\n\
         puts [catch {{source {early}}} m]\n\
         puts [source {value}]\n\
         catch {{source {errs}}}\n\
         puts [lindex [split $errorInfo \\n] 3]\n"
    );
    check(&dir, &source, |_| {
        format!("1-boom\n3\nx\n    (file \"{errs}\" line 2)\n")
    });
    let _ = std::fs::remove_dir_all(&dir);
}
