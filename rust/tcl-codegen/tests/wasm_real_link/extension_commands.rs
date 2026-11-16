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

//! A compiled script calling a command a C extension registered.

use super::*;

/// A C extension in the shape the ABI describes (`c-extension-abi.md` §4.5 and
/// §12), written in WAT so the seam is exercised without a C toolchain: a module
/// that shares the runtime's memory and function table, installs its command
/// procedure in that table, and registers it from `Foo_Init` through the
/// runtime's own `Tcl_CreateObjCommand` export — which is all a compiled C
/// extension's `Foo_Init` is, once `wasm-ld` has laid it out.
///
/// `foo` answers three ways, so each half of the contract is observable: with
/// no argument, its `clientData` (the `7` it was registered with); with one,
/// that argument itself; with two or more, the error `too many`.
fn foo_extension_wat() -> String {
    r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "__indirect_function_table" (table $fns 0 funcref))
  (import "tcl" "Tcl_CreateObjCommand" (func $create (param i32 i32 i32 i32 i32) (result i32)))
  (import "tcl" "Tcl_NewStringObj" (func $newstr (param i32 i32) (result i32)))
  (import "tcl" "Tcl_NewWideIntObj" (func $newint (param i64) (result i32)))
  (import "tcl" "Tcl_SetObjResult" (func $setres (param i32 i32)))
  (type $objcmd_t (func (param i32 i32 i32 i32) (result i32)))
  (elem declare func $foo)
  (func $foo (type $objcmd_t)
    (param $client i32) (param $interp i32) (param $objc i32) (param $objv i32) (result i32)
    (if (i32.gt_s (local.get $objc) (i32.const 2))
      (then
        (call $setres (local.get $interp) (call $newstr (i32.const 0x1a0010) (i32.const 8)))
        (return (i32.const 1))))
    (if (i32.eq (local.get $objc) (i32.const 2))
      (then
        (call $setres (local.get $interp) (i32.load offset=4 (local.get $objv)))
        (return (i32.const 0))))
    (call $setres (local.get $interp) (call $newint (i64.extend_i32_u (local.get $client))))
    (i32.const 0))
  (func (export "Foo_Init") (param $interp i32) (result i32)
    (local $slot i32)
    (local.set $slot (table.grow $fns (ref.func $foo) (i32.const 1)))
    (if (i32.lt_s (local.get $slot) (i32.const 0)) (then unreachable))
    (if (i32.eqz (call $create (local.get $interp) (i32.const 0x1a0000)
          (local.get $slot) (i32.const 7) (i32.const 0)))
      (then unreachable))
    (i32.const 0))
  (data (i32.const 0x1a0000) "foo\00")
  (data (i32.const 0x1a0010) "too many"))
"#
    .to_owned()
}

/// The generic-argv bootstrap of [`semantic_invoke_bootstrap_wat`], with the
/// extension's `Foo_Init` called between creating the interpreter and running
/// `::top` when `register` is set: the order a host that loads an extension
/// follows, and the one that makes the command's absence before it observable.
fn extension_bootstrap_wat(expected_code: i32, register: bool) -> String {
    let create = CodegenAbiImportId::RuntimeCreateInterp.descriptor().name;
    let set_current = CodegenAbiImportId::RuntimeSetCurrentInterp
        .descriptor()
        .name;
    let object_release = CodegenAbiImportId::ObjectRelease.descriptor().name;
    let init = if register {
        "    (if (i32.ne (call $init (local.get $interp)) (i32.const 0)) (then unreachable))\n"
    } else {
        ""
    };
    format!(
        r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "{create}" (func $create (result i32)))
  (import "tcl" "{set_current}" (func $setcur (param i32)))
  (import "tcl" "{object_release}" (func $rel (param i32)))
  (import "tcl" "Tcl_GetStringFromObj" (func $getstr (param i32 i32) (result i32)))
  (import "ext" "Foo_Init" (func $init (param i32) (result i32)))
  (import "user" "::top" (func $top (result i32 i32 i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $interp i32) (local $code i32) (local $result i32) (local $options i32)
    (local $strptr i32) (local $len i32)
    (local.set $interp (call $create))
    (call $setcur (local.get $interp))
{init}    (call $top)
    (local.set $options)
    (local.set $result)
    (local.set $code)
    (if (i32.ne (local.get $code) (i32.const {expected_code})) (then unreachable))
    (local.set $strptr (call $getstr (local.get $result) (i32.const 0x190000)))
    (local.set $len (i32.load (i32.const 0x190000)))
    (i32.store (i32.const 0x190008) (local.get $strptr))
    (i32.store (i32.const 0x19000C) (local.get $len))
    (drop (call $fd_write (i32.const 1) (i32.const 0x190008) (i32.const 1) (i32.const 0x190010)))
    (call $rel (local.get $result))
    (call $rel (local.get $options))))
"#,
    )
}

/// Compile `program` (one literal command, so it takes the canonical generic
/// argv), link it with the real runtime and [`foo_extension_wat`], run it, and
/// return what `::top` completed with, having demanded `expected_code`.
fn run_real_link_with_extension(
    runtime: &Path,
    tag: &str,
    program: &str,
    expected_code: i32,
    register: bool,
) -> String {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(program, &registry, false, "tcl9.0");
    let mut output = compile_wasm(&unit, &registry, WasmCompileOptions::runtime_linked());
    assert!(
        matches!(output.plan, WasmCodegenPlan::GenericInvoke { .. }),
        "a literal command should select the canonical generic argv, got {:?}",
        output.plan
    );
    let user = scratch(&format!("tcl_real_link_ext_user_{tag}.wasm"));
    let ext = scratch(&format!("tcl_real_link_ext_{tag}.wat"));
    let boot = scratch(&format!("tcl_real_link_ext_boot_{tag}.wat"));
    write_linked(runtime, &user, &output.to_bytes(), program);
    std::fs::write(&ext, foo_extension_wat()).expect("write the extension");
    std::fs::write(&boot, extension_bootstrap_wat(expected_code, register))
        .expect("write the bootstrap");

    let out = Command::new("wasmtime")
        .arg("run")
        .arg("--preload")
        .arg(format!("tcl={}", runtime.display()))
        .arg("--preload")
        .arg(format!("ext={}", ext.display()))
        .arg("--preload")
        .arg(format!("user={}", user.display()))
        .arg(&boot)
        .output()
        .expect("run wasmtime");
    for path in [&user, &ext, &boot] {
        let _ = std::fs::remove_file(path);
    }
    assert!(
        out.status.success(),
        "the extension link trapped for {program:?}:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("stdout is utf-8")
}

/// The seam `c-extension-abi.md` §12 names: a script compiled by
/// `tcl_codegen::codegen::wasm` calls a command an *extension* registered, and
/// the call dispatches into that extension's procedure through the shared
/// function table. `Foo_Init` registers `foo` through `Tcl_CreateObjCommand`;
/// the compiled script's argv reaches it through `tcl_invoke_argv` and the live
/// command table, with the `clientData` the extension registered it with.
///
/// The negative is the same compiled module before registration: the call is
/// `invalid command name "foo"`, so the dispatch that finds `foo` afterwards is
/// the registration and not something the compiled code carried.
#[test]
fn a_compiled_script_calls_an_extension_registered_command() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    assert_eq!(
        run_real_link_with_extension(&runtime, "before", "foo a\n", 1, false),
        "invalid command name \"foo\"",
        "before registration there is no such command"
    );
    // (program, completion code, what `::top` completed with)
    for (index, (program, code, expected)) in [
        ("foo\n", 0, "7"),
        ("foo hello\n", 0, "hello"),
        ("foo a b\n", 1, "too many"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            run_real_link_with_extension(&runtime, &format!("after{index}"), program, code, true),
            expected,
            "program {program:?}"
        );
    }
}
