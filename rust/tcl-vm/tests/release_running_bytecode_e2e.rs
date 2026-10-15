// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A command-table change while bytecode runs, as each release sees it.
//!
//! Tcl 8.4 checks its compile epoch only when bytecode is entered, so a unit
//! already running keeps the commands it compiled until its next entry; 8.5
//! and later revalidate every command (`INST_START_CMD`). Which commands a
//! unit compiled is 8.4's choice: its `compileProc`s, some only in a
//! procedure body or for literal words, and `eval`/`uplevel` scripts it
//! evaluates directly. The expectations are tclsh 8.4.20 and 9.0.4 running
//! each script as a file, and are checked against those releases when they
//! are installed.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::{Code, Vm};

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
        "{version:?}: {source}: {}",
        completion.result.to_str()
    );
    String::from_utf8(output.0.borrow().clone()).expect("UTF-8 output")
}

/// What `version`'s tclsh prints running `source` as a file, when it is
/// installed.
fn oracle(version: TclVersion, index: usize, source: &str) -> Option<String> {
    let tclsh = tcl_test_support::locate_tclsh(version).expect("the oracle is valid")?;
    let path = std::env::temp_dir().join(format!(
        "release-running-bytecode-{}-{index}-{}.tcl",
        std::process::id(),
        version.dialect_name()
    ));
    std::fs::write(&path, source).expect("writes the script");
    let output = std::process::Command::new(&tclsh.path)
        .arg(&path)
        .output()
        .expect("runs tclsh");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success() && output.stderr.is_empty(),
        "{version:?}: {source}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(String::from_utf8(output.stdout).expect("UTF-8 output"))
}

const REDEFINE_EXPR: &str = "proc r {} {rename expr _o; proc expr args {return R}}\n";

/// A script with what 8.4 and 9.0 print; `None` where 9.0's own library
/// cannot run the script.
type Case = (String, &'static str, Option<&'static str>);

fn cases() -> Vec<Case> {
    let mut cases = expr_cases();
    cases.extend(command_cases());
    cases.extend(shape_cases());
    cases
}

/// [`Case`]s whose word shapes decide whether 8.4 compiles the command, and
/// one that names the command its unit was entered with.
fn shape_cases() -> Vec<Case> {
    let case = |redefine: &str, body: &str, v84: &'static str, v90: &'static str| {
        (format!("proc r {{}} {{{redefine}}}; {body}"), v84, Some(v90))
    };
    vec![
        // `append` compiles one value at most, `lappend` one value.
        case(
            "rename append _a; proc append args {puts AP}",
            "proc p {} {set s {}; r; append s x; append s y z; puts $s}; p",
            "AP\nx\n",
            "AP\nAP\n\n",
        ),
        case(
            "rename lappend _l; proc lappend args {puts LA}",
            "proc p {} {set L {}; r; lappend L x; lappend L y z; puts $L}; p",
            "LA\nx\n",
            "LA\nLA\n\n",
        ),
        // `regexp` compiles a plain pattern without match variables.
        case(
            "rename regexp _re; proc regexp args {return RE}",
            "proc p {} {r; puts [regexp abc xabcx]; puts [regexp a+ aa]; \
             puts [regexp ^ab.* abc]; puts [regexp -nocase -- AB ab]; puts [regexp b abc m]}; p",
            "1\nRE\n1\n1\nRE\n",
            "RE\nRE\nRE\nRE\nRE\n",
        ),
        // `string` reads its subcommand as a unique prefix, and counts words.
        case(
            "rename string _s; proc string args {return S}",
            "proc p {} {r; puts [string len abc]; puts [string match -nocase A* abc]; \
             puts [string equal -nocase a A]; puts [string index abc 1]; \
             puts [string compare a b]}; p",
            "3\n1\nS\nb\n-1\n",
            "S\nS\nS\nS\nS\n",
        ),
        // A result or loop variable must be a local scalar.
        case(
            "rename catch _c; proc catch args {puts CATCH}",
            "proc p {} {r; catch {puts b1} m; catch {puts b2} ::m2; catch {puts b3} a(x)}; p",
            "b1\nCATCH\nCATCH\n",
            "CATCH\nCATCH\nCATCH\n",
        ),
        case(
            "rename foreach _f; proc foreach args {puts FE}",
            "proc p {} {r; foreach x {1} {puts x$x}; foreach ::y {1} {puts y}; \
             foreach {a b} {1 2} {puts $a$b}}; p",
            "x1\nFE\n12\n",
            "FE\nFE\nFE\n",
        ),
        // `return` is not compiled inside a `catch`.
        case(
            "rename return _r; proc return args {puts RET}",
            "proc p {} {r; catch {return x}; puts after}; puts [p]",
            "RET\nafter\n\n",
            "RET\nafter\n\n",
        ),
        case(
            "rename lindex _i; proc lindex args {return IX}",
            "proc p {} {r; puts [lindex {a b} 1]; puts [lindex {a b}]}; p",
            "b\na b\n",
            "IX\nIX\n",
        ),
        // A command defined in the procedure's namespace after its unit was
        // entered does not displace the global builtin the unit compiled.
        (
            "proc r {} {proc ::ns::expr args {return NSR}}; \
             namespace eval ns {proc p {} {puts [expr {1+1}]; ::r; puts [expr {1+1}]}}; ns::p; ns::p"
                .to_owned(),
            "2\n2\nNSR\nNSR\n",
            Some("2\nNSR\nNSR\nNSR\n"),
        ),
    ]
}

/// [`Case`]s that replace `expr` while bytecode runs.
fn expr_cases() -> Vec<Case> {
    let expr = |body: &str| format!("{REDEFINE_EXPR}{body}");
    vec![
        // A running procedure keeps its compiled `expr`; its next entry and
        // the top level see the replacement.
        (
            expr("proc p {} {puts [expr {1+1}]; r; puts [expr {1+1}]}; p; puts [expr {1+1}]"),
            "2\n2\nR\n",
            Some("2\nR\nR\n"),
        ),
        // A top-level loop body is a unit entered on each iteration.
        (
            expr("foreach i {1 2} {if {$i == 1} {r}; puts [expr {1+1}]}"),
            "2\nR\n",
            Some("R\nR\n"),
        ),
        (
            expr("set i 0; while {$i < 2} {if {$i == 0} {r}; incr i; puts [expr {1+1}]}"),
            "2\nR\n",
            Some("R\nR\n"),
        ),
        // In a procedure the loop is compiled into its body.
        (
            expr("proc p {} {foreach i {1 2} {if {$i == 1} {r}; puts [expr {1+1}]}}; p"),
            "2\n2\n",
            Some("R\nR\n"),
        ),
        (
            expr(
                "proc p {} {for {set i 0} {$i < 2} {incr i} {if {$i == 0} {r}; puts [expr {1+1}]}}; p",
            ),
            "2\n2\n",
            Some("R\nR\n"),
        ),
        // `switch` is not compiled, so its arm is a unit of its own, outside
        // a procedure body, where `foreach` is not compiled either.
        (
            expr(
                "proc p {} {switch a {a {foreach i {1 2} {if {$i==1} {r}; puts [expr {1+1}]}}}}; p",
            ),
            "2\nR\n",
            Some("R\nR\n"),
        ),
        // `eval` and `uplevel` evaluate their scripts directly.
        (
            expr(
                "proc p {} {r; eval {puts [expr {1+1}]}; uplevel 0 {puts [expr {1+1}]}; puts [expr {1+1}]}; p",
            ),
            "R\nR\n2\n",
            Some("R\nR\nR\n"),
        ),
        (
            expr("proc p {} {set s {puts [expr {1+1}]}; r; eval $s; puts [expr {1+1}]}; p"),
            "R\n2\n",
            Some("R\nR\n"),
        ),
    ]
}

/// [`Case`]s that replace or trace other commands while bytecode runs.
fn command_cases() -> Vec<Case> {
    vec![
        // A compiled command survives its deletion.
        (
            "proc r {} {rename expr {}}; proc p {} {r; puts [expr {1+1}]}; p".to_owned(),
            "2\n",
            None,
        ),
        (
            "proc r {} {rename set _s; proc set args {puts SET}}; proc p {} {r; set y 1; puts ok}; p"
                .to_owned(),
            "ok\n",
            Some("SET\nok\n"),
        ),
        // Only some `string` subcommands are compiled.
        (
            "proc r {} {rename string _s; proc string args {return S}}; \
             proc p {} {r; puts [string length abc]; puts [string toupper abc]}; p"
                .to_owned(),
            "3\nS\n",
            Some("S\nS\n"),
        ),
        // `lappend` is compiled only in a procedure body.
        (
            "proc r {} {rename lappend _la; proc lappend args {puts LA}}; \
             proc p {} {set L {}; r; lappend L x; puts $L}; p"
                .to_owned(),
            "x\n",
            Some("LA\n\n"),
        ),
        (
            "proc r {} {rename switch _s; proc switch args {puts SW}}; \
             proc p {} {r; switch a {a {puts A}}}; p"
                .to_owned(),
            "SW\n",
            Some("SW\n"),
        ),
        // A compiled command runs without the execution trace added to it.
        (
            "proc t args {puts \"trace [lindex $args 0]\"}; \
             proc p {} {trace add execution set enter t; set y 1; puts ok}; p"
                .to_owned(),
            "ok\n",
            Some("trace set y 1\nok\n"),
        ),
        (
            "proc r {} {rename catch _c; proc catch args {puts CATCH}}; \
             proc p {} {r; catch {puts body}; catch {puts b2} m; puts done}; p"
                .to_owned(),
            "body\nb2\ndone\n",
            Some("CATCH\nCATCH\ndone\n"),
        ),
        // `if` is compiled only when every word is literal.
        (
            "proc r {} {rename if _if; proc if args {puts IF}}; \
             proc p {} {r; if 1 {puts T}; set c 1; if $c {puts T2}}; p; rename if {}; rename _if if"
                .to_owned(),
            "T\nIF\n",
            Some("IF\nIF\n"),
        ),
        // A body the VM runs through the builtin it invokes still belongs to
        // the procedure that compiled the command.
        (
            "proc r {} {rename foreach _o; proc foreach args {puts F}}; set l {a b}; \
             proc p {} {global l; foreach _x {1} {r; foreach x $l {puts $x}}}; p"
                .to_owned(),
            "a\nb\n",
            Some("F\n"),
        ),
    ]
}

#[test]
fn running_bytecode_sees_a_command_table_change_as_its_release_does() {
    for (index, (source, v84, v90)) in cases().iter().enumerate() {
        assert_eq!(run_file(TclVersion::V8_4, source), *v84, "8.4: {source}");
        if let Some(expected) = oracle(TclVersion::V8_4, index, source) {
            assert_eq!(expected, *v84, "tclsh 8.4: {source}");
        }
        let Some(v90) = v90 else {
            continue;
        };
        assert_eq!(run_file(TclVersion::V9_0, source), *v90, "9.0: {source}");
        if let Some(expected) = oracle(TclVersion::V9_0, index, source) {
            assert_eq!(expected, *v90, "tclsh 9.0: {source}");
        }
    }
}
