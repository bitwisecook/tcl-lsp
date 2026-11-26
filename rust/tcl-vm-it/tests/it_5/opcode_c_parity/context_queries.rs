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

//! Opcodes that read the execution context: `currentNamespace`, `infoLevel*`, `resolveCmd`, `originCmd` and `clockRead`.

use super::*;

// `currentNamespace` / `infoLevelNumber` / `infoLevelArgs`.

/// `currentNamespace` pushes the fully-qualified name (`::` at global scope)
/// and `infoLevelNumber` the call depth — the same values `namespace current`
/// and `info level` report.
#[test]
fn current_namespace_and_info_level_number() {
    let mut a = Asm::new();
    a.op(Op::CURRENT_NAMESPACE, &[]);
    a.op(Op::INFO_LEVEL_NUM, &[]);
    a.op(Op::LIST, &[2]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), ":: 0");
}

/// `infoLevelArgs` routes through `info level`'s own core, so its errors are
/// the command's: `bad level "N"` for a level with no call, and the integer
/// coercion error for a non-numeric level.
#[test]
fn info_level_args_matches_the_command() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    let cmd_bad_level = vm
        .eval_source("catch {info level 1} m; set m")
        .expect("compiles")
        .result
        .to_str()
        .to_string();
    let mut a = Asm::new();
    a.push("1").op(Op::INFO_LEVEL_ARGS, &[]);
    let c = run(&mut vm, a);
    // At global scope there is no frame 1, for the opcode as for the command.
    assert_eq!(err_str(&c), "bad level \"1\"");
    assert_eq!(cmd_bad_level, err_str(&c));

    let mut a = Asm::new();
    a.push("foo").op(Op::INFO_LEVEL_ARGS, &[]);
    let c = run(&mut vm, a);
    assert_eq!(err_str(&c), "expected integer but got \"foo\"");
}

// `resolveCmd` / `originCmd`.

/// `resolveCmd` pushes the fully-qualified command name, or the empty string
/// when the name resolves to nothing — it never errors (C
/// `INST_RESOLVE_COMMAND`).
#[test]
fn resolve_cmd_pushes_fqn_or_empty() {
    let mut a = Asm::new();
    a.push("set").op(Op::RESOLVE_CMD, &[]);
    a.push("no_such_cmd_xyz").op(Op::RESOLVE_CMD, &[]);
    a.op(Op::LIST, &[2]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "::set {}");
}

/// `originCmd` pushes the origin's fully-qualified name and errors on a name
/// that resolves to no command (C `INST_ORIGIN_COMMAND`).
#[test]
fn origin_cmd_pushes_origin_and_rejects_unknown() {
    let mut a = Asm::new();
    a.push("set").op(Op::ORIGIN_CMD, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "::set");

    let mut a = Asm::new();
    a.push("no_such_cmd_xyz").op(Op::ORIGIN_CMD, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "invalid command name \"no_such_cmd_xyz\"");
}

/// The origin follows the `namespace import` chain, as C's
/// `TclGetOriginalCommand` does.
#[test]
fn origin_cmd_follows_import_chain() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    let setup = "namespace eval src {proc p {} {return 1}; namespace export p}\n\
                 namespace import ::src::p";
    assert!(vm.eval_source(setup).expect("compiles").code.is_ok());
    let mut a = Asm::new();
    a.push("p").op(Op::ORIGIN_CMD, &[]);
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), "::src::p");
}

// `clockRead`.

/// `clockRead` reads the same host clock `clock clicks`/`clock seconds` do, so
/// the four readings agree with one another (0 = clicks = µs, 1 = µs, 2 = ms,
/// 3 = s).
#[test]
fn clock_read_units_agree() {
    let mut a = Asm::new();
    a.op(Op::CLOCK_READ, &[0]);
    a.op(Op::CLOCK_READ, &[1]);
    a.op(Op::CLOCK_READ, &[2]);
    a.op(Op::CLOCK_READ, &[3]);
    a.op(Op::LIST, &[4]);
    let (_vm, c) = run_fresh(a);
    let vals: Vec<i64> = ok_str(&c)
        .split_whitespace()
        .map(|s| s.parse::<i64>().expect("wide integer reading"))
        .collect();
    let &[clicks, micros, millis, secs] = vals.as_slice() else {
        panic!("four readings expected, got {vals:?}");
    };
    assert!(secs > 1_600_000_000, "seconds look like a Unix timestamp");
    // Same clock, different scales — allow one unit of drift between reads.
    assert!((micros / 1_000 - millis).abs() <= 1, "µs vs ms: {vals:?}");
    assert!((millis / 1_000 - secs).abs() <= 1, "ms vs s: {vals:?}");
    assert!(
        (clicks / 1_000 - millis).abs() <= 1,
        "clicks vs ms: {vals:?}"
    );
}
