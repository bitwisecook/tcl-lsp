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

//! Compiled procedure bodies through the runtime's function table: frames, recursion depth, table growth and published entries.

use super::*;

/// A bootstrap that installs a function of its own into the **runtime's**
/// indirect function table and calls it back through that table.
///
/// This is the whole of the table-install transport, minus the emitter: a wasm32
/// function pointer *is* an index into this table, so a module can only hand
/// the runtime a callable body by growing the table, `ref.func`-ing its
/// function into the new slot, and passing the index across the ABI. Every
/// step is a linker property of `tcl_runtime.wasm`, not of anything we emit,
/// which is why it is proved here before any codegen depends on it.
///
/// Each failure is distinct and loud:
///
/// - the runtime linked without `--export-table` ⇒ wasmtime refuses to
///   instantiate this module, naming the missing import;
/// - linked without `--growable-table` ⇒ the table keeps `wasm-ld`'s
///   `min == max`, `table.grow` returns `-1`, and the `unreachable` traps;
/// - a slot that does not hold the installed function ⇒ the `call_indirect`
///   result is not 42 and the second `unreachable` traps.
fn function_table_bootstrap_wat() -> String {
    format!(
        r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "{WASM32_FUNCTION_TABLE_IMPORT}" (table $fns 0 funcref))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (export "memory" (memory 0))
  (type $answer_t (func (result i32)))
  (elem declare func $answer)
  (func $answer (type $answer_t) (i32.const 42))
  (func (export "_start")
    (local $slot i32)
    (local.set $slot (table.grow $fns (ref.null func) (i32.const 1)))
    (if (i32.lt_s (local.get $slot) (i32.const 0)) (then unreachable))
    (table.set $fns (local.get $slot) (ref.func $answer))
    (if (i32.ne (call_indirect $fns (type $answer_t) (local.get $slot)) (i32.const 42))
      (then unreachable))
    (i32.store (i32.const 0x190008) (i32.const 0x180000))
    (i32.store (i32.const 0x19000C) (i32.const 3))
    (drop (call $fd_write (i32.const 1) (i32.const 0x190008) (i32.const 1) (i32.const 0x190010))))
  (data (i32.const 0x180000) "ok\n"))
"#
    )
}

fn interpreted_source_bootstrap_wat(source: &str) -> String {
    format!(
        r#"(module
  (import "tcl" "memory" (memory 1))
  (import "tcl" "tcl_runtime_create_interp" (func $create (result i32)))
  (import "tcl" "tcl_runtime_set_current_interp" (func $setcur (param i32)))
  (import "tcl" "tcl_obj_new_string" (func $box (param i32 i32) (result i32)))
  (import "tcl" "tcl_eval" (func $eval (param i32) (result i32)))
  (import "tcl" "tcl_obj_release" (func $rel (param i32)))
  (import "tcl" "Tcl_GetStringFromObj" (func $getstr (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (export "memory" (memory 0))
  (func (export "_start")
    (local $script i32) (local $result i32) (local $strptr i32) (local $len i32)
    (call $setcur (call $create))
    (local.set $script (call $box (i32.const 0x180000) (i32.const {length})))
    (if (i32.eqz (local.get $script)) (then unreachable))
    (local.set $result (call $eval (local.get $script)))
    (if (i32.eqz (local.get $result)) (then unreachable))
    (local.set $strptr (call $getstr (local.get $result) (i32.const 0x190000)))
    (if (i32.eqz (local.get $strptr)) (then unreachable))
    (local.set $len (i32.load (i32.const 0x190000)))
    (i32.store (i32.const 0x190008) (local.get $strptr))
    (i32.store (i32.const 0x19000C) (local.get $len))
    (drop (call $fd_write (i32.const 1) (i32.const 0x190008) (i32.const 1) (i32.const 0x190010)))
    (call $rel (local.get $result)))
  (data (i32.const 0x180000) "{source}"))
"#,
        length = source.len(),
        source = wat_escape(source),
    )
}

/// A compiled procedure body reaches the interpreter through the shared
/// function table, and answers what tclsh answers.
///
/// The `::top` prologue grows the runtime's table, keeps the base in its own
/// global and writes one `ref.func` per bound body; `tcl_codegen_proc_define_original_argv`
/// hands each index to the definition. Everything after that is the runtime's
/// (`run_proc` -> `run_native_body` -> `call_indirect`), and the counter in the
/// bootstrap is what proves the compiled body — not the source body sitting
/// beside it on the same `ProcDef` — is the one that ran.
///
/// Verified against `tmp/tcl9.0.4/unix/tclsh` and `tmp/tcl8.6.16/unix/tclsh`.
#[test]
fn a_compiled_proc_body_runs_through_the_runtimes_function_table() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    // Two bodies, one calling the other, so the table holds more than one
    // entry and a slot-arithmetic slip cannot pass.
    let program = "\
proc add {a b} { return [expr {$a + $b}] }
proc twice {x} { return [add $x $x] }
set answer [twice 21]
set level [add 1 1]
";
    assert_eq!(
        run_real_native_proc(
            &runtime,
            "native_proc_table",
            program,
            2,
            "list $answer $level",
            3,
        ),
        "42 2",
    );
}

/// `info level` from inside a compiled body reports the frame `run_proc`
/// pushed — the detector for the double-frame mistake.
///
/// A proc entry that emitted the module-function prologue would push a second,
/// nameless frame at the caller's namespace: `info level` would answer one
/// deeper and `info level 0` would have no words at all. Both are checked
/// here through the whole linked stack, not just against a Rust stub.
#[test]
fn a_compiled_proc_body_sees_the_frame_run_proc_pushed() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let program = "\
proc lvl {} { return [list [info level] [info level 0]] }
proc outer {x} { return [lvl] }
set answer [outer 7]
";
    assert_eq!(
        run_real_native_proc(&runtime, "native_proc_frame", program, 2, "set answer", 2,),
        "2 lvl",
    );
}

/// Recursion through the native path refuses at the same eval depth the
/// interpreted body does, with the eval loop's own message.
///
/// The accounting this pins: one Tcl level through a compiled body costs
/// exactly two eval-depth units — `run_native_body`'s activation and the
/// recursive call's own `tcl_invoke_argv`. An entry that took an activation of
/// its own would refuse a third sooner; one that dropped the runtime's would
/// run past the bound. Measured on this exact script: `runtime/rust`'s
/// `run_script` answers `ok` through depth 61 and refuses at 62, and so does
/// this. Both diverge from tclsh 9.0.4 and 8.6.16, which run every depth
/// here — that gap belongs to the native eval-depth limit, not to this
/// harness, and is deliberately left visible.
///
/// `probe` keeps the `catch` out of `::top`: a top-level `catch` declines the
/// entry point's own lowering, and nothing binds in a module whose `::top`
/// never runs the table install.
#[test]
fn recursion_through_a_compiled_body_refuses_at_the_interpreted_depth() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let program = "\
proc d {n} { if {$n} { d [expr {$n - 1}] } ; return ok }
proc probe {n} { if {[catch {d $n} m]} { return $m } ; return ok }
set r60 [probe 60]
set r61 [probe 61]
set r62 [probe 62]
set answer [list $r60 $r61 $r62]
";
    let printed = run_real_native_proc(&runtime, "native_proc_depth", program, 2, "set answer", 50);
    if printed != "ok ok {too many nested evaluations (infinite loop?)}" {
        let source = format!("{program}\nset answer");
        let interpreted = run_boot_only(
            &runtime,
            "interpreted_proc_depth_witness",
            &interpreted_source_bootstrap_wat(&source),
        );
        eprintln!("WASM_INTERPRETED_DEPTH_WITNESS result={interpreted:?}");
    }
    assert_eq!(
        printed, "ok ok {too many nested evaluations (infinite loop?)}",
        "the compiled body must refuse at the interpreted runtime's own depth, \
         with the eval loop's own message"
    );
}

/// The runtime exports its indirect function table, and that table grows.
///
/// `runtime/rust/build.rs` passes `--export-table --growable-table` for every
/// wasm target. Without the first there is no table to import; without the
/// second `wasm-ld` fixes `max == min` and nothing can ever be installed. Both
/// are inert for a module that does not touch the table, so no already-emitted
/// module changes — but a native proc entry is impossible without them.
#[test]
fn the_runtime_exports_a_growable_indirect_function_table() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    match run_boot_only(&runtime, "function_table", &function_table_bootstrap_wat()) {
        Ok(stdout) => assert_eq!(stdout, "ok\n"),
        Err(report) => panic!(
            "a module could not install a function into the runtime's table.\n             Check that runtime/rust/build.rs still passes --export-table and \
             --growable-table for wasm targets.\n{report}"
        ),
    }
}

#[test]
fn root_colon_publication_runs_its_own_compiled_table_entry() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source = "proc :audit {a b} {return [expr {$a + $b}]}\n\
                  proc ordinary {a b} {return [expr {$a - $b}]}\n\
                  set colon_result [:audit 40 2]\n\
                  set ordinary_result [ordinary 10 3]\n";
    // compile_native_bound requires two actual table entries and a call to the
    // additive definition import. The bootstrap requires two runtime native
    // procedure dispatches before evaluating this query. Different body results
    // make binding the colon declaration to the wrong table slot observable.
    assert_eq!(
        run_real_native_proc(
            &runtime,
            "original_argv_colon_table",
            source,
            2,
            "list $colon_result $ordinary_result [info commands audit] [info args :audit]",
            2,
        ),
        "42 7 {} {a b}",
    );
}

#[test]
fn original_argv_descriptor_requires_current_manifest_and_rejects_prior_fingerprint() {
    use tcl_runtime_api::ManifestField;
    // This is the source-derived fingerprint of the inspected pre-additive
    // descriptor/layout table, retained with its source hash in abi-layout.json.
    // It is not a claim of executing an old Runtime binary or old module.
    const PRIOR_TABLE_FINGERPRINT: u32 = 0x034b_ab19;
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    assert_ne!(
        tcl_runtime_api::codegen_abi::CODEGEN_ABI_VERSION,
        PRIOR_TABLE_FINGERPRINT,
    );
    let source = "proc :audit {} {return ORIGINAL}\nset answer [:audit]\n";
    let honest = compile_native_bound(source, 1);
    assert_eq!(check_manifest(&runtime, &honest), Ok(()));
    // A genuine newly compiled module with a foreign prior-table manifest is
    // passed to the actual pre-link identity comparator. Only that manifest
    // field changes; no fake Runtime identity or widened gate is supplied.
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    let mut foreign = compile_wasm(
        &unit,
        &registry,
        WasmCompileOptions::runtime_linked().native_tier(),
    );
    foreign
        .manifest
        .as_mut()
        .expect("actual emitter manifest")
        .abi_version = PRIOR_TABLE_FINGERPRINT;
    assert!(matches!(
        check_manifest(&runtime, &foreign.to_bytes()),
        Err(LinkRefusal::Disagrees {
            field: ManifestField::AbiVersion,
            ..
        }),
    ));
    // Actual linking and call_indirect execution also validate the new four-i32
    // import signature; descriptor arithmetic alone cannot establish it.
    assert_eq!(
        run_real_native_proc(
            &runtime,
            "original_argv_current_manifest",
            source,
            1,
            "list $answer [info commands audit]",
            1,
        ),
        "ORIGINAL {}",
    );
}
