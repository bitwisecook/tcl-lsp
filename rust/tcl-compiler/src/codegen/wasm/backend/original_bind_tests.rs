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

//! Actual emitted prologue controls with a software ABI provider.

use super::*;
use crate::semantic_optimisation::SemanticOptimisationPassId;
use std::fmt::Write as _;
use std::sync::Arc;

fn original_module() -> WasmModule {
    let profile = tcl_dialect::DialectProfile::irules();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let context =
        tcl_registry::model::ingress::resolve_environment("f5-irules").default_context_registry();
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let mut entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input)
        .expect("complete original Logical source input");
    entry.compiled_variable_provider = Some(
        tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation,
    );
    let unit = CompilationUnit::build_with_context_registry(
        "proc sum {a b c} {return [expr {$a + $b + $c}]}\nsum 1 2 3\n",
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        Arc::clone(&context),
    );
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    assert!(matches!(
        crate::native_compilation_admission::script_admission_plan(
            &unit.ir_module.top_level,
            crate::native_compilation_admission::NativeCompilationAdmissionScope::Script,
        ),
        crate::native_compilation_admission::NativeCompilationAdmissionPlan::NoRetainedObligation
    ));
    let module = super::super::compile_wasm(
        &unit,
        context.commands(),
        WasmCompileOptions::hosted()
            .with_data_base(0)
            .with_semantic_optimisation(SemanticOptimisationPassId::LegacyAnalysisSpecialisation),
    )
    .into_module();
    let function = module
        .functions
        .iter()
        .find(|function| function.name == "::sum")
        .expect("actual original procedure is emitted");
    assert_eq!(function.params, vec![ValType::I32; 3]);
    assert_eq!(function.results, vec![ValType::I32]);
    assert!(
        module
            .imports
            .iter()
            .any(|import| import.name == "tcl_codegen_local_bind")
    );
    module
}

fn provider(module: &WasmModule, failed: i32, status: i32, host: bool, entry: bool) -> String {
    let mut source = format!(
        r#"(module
      (import "wasi_snapshot_preview1" "fd_write" (func $write (param i32 i32 i32 i32) (result i32)))
      (memory (export "memory") 1)
      (global $host (mut i32) (i32.const 0))
      (global $first (mut i32) (i32.const 0))
      (global $cells (mut i32) (i32.const 0))
      (func (export "first") (result i32) global.get $first)
      (func $mark (param $byte i32)
        (i32.store8 (i32.const 0xF100) (local.get $byte))
        (i32.store (i32.const 0xF000) (i32.const 0xF100))
        (i32.store (i32.const 0xF004) (i32.const 1))
        (drop (call $write (i32.const 1) (i32.const 0xF000) (i32.const 1) (i32.const 0xF010))))
      (func $refuse
        (global.set $host (i32.const 1))
        (if (i32.eqz (global.get $first)) (then (global.set $first (i32.const 11)))))
      (func $release (param $word i32) (call $mark (local.get $word)))
      (export "tcl_obj_release" (func $release))
      (func (export "tcl_codegen_frame_push")
        (if (i32.const {entry}) (then (call $refuse))))
      (func (export "tcl_codegen_frame_pop")
        (call $mark (i32.const 112))
        (if (i32.and (global.get $cells) (i32.const 1)) (then (call $release (i32.const 65))))
        (if (i32.and (global.get $cells) (i32.const 2)) (then (call $release (i32.const 66))))
        (if (i32.and (global.get $cells) (i32.const 4)) (then (call $release (i32.const 67))))
        (global.set $cells (i32.const 0)))
      (func (export "tcl_codegen_host_refusal_pending") (result i32) global.get $host)
      (func (export "tcl_codegen_local_bind") (param $slot i32) (param i32 i32) (param $word i32) (result i32)
        (call $mark (i32.add (i32.const 97) (local.get $slot)))
        (if (i32.eq (local.get $slot) (i32.const {failed}))
          (then
            (if (i32.const {host}) (then (call $refuse)))
            (call $release (local.get $word))
            (return (i32.const {status}))))
        (global.set $cells (i32.or (global.get $cells) (i32.shl (i32.const 1) (local.get $slot))))
        i32.const 0)
      (func (export "tcl_codegen_local_get") (param i32) (result i32)
        (call $mark (i32.const 103)) i32.const 999)
      (func (export "tcl_codegen_expr_add") (param i32 i32) (result i32)
        (call $mark (i32.const 120)) i32.const 999)
    "#,
        entry = i32::from(entry),
        host = i32::from(host)
    );
    let provided = [
        "tcl_obj_release",
        "tcl_codegen_frame_push",
        "tcl_codegen_frame_pop",
        "tcl_codegen_host_refusal_pending",
        "tcl_codegen_local_bind",
        "tcl_codegen_local_get",
        "tcl_codegen_expr_add",
    ];
    let types = module.type_section();
    for import in &module.imports {
        let (params, results) = &types[import.type_idx];
        if provided.contains(&import.name.as_str()) {
            continue;
        }
        let _ = write!(source, "(func (export \"{}\")", import.name);
        if !params.is_empty() {
            source.push_str(" (param ");
            source.push_str(
                &params
                    .iter()
                    .copied()
                    .map(ValType::wat_name)
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            source.push(')');
        }
        if !results.is_empty() {
            source.push_str(" (result ");
            source.push_str(
                &results
                    .iter()
                    .copied()
                    .map(ValType::wat_name)
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            source.push(')');
        }
        // Only the unreachable terminal empty-result factory is harmless.
        source.push_str(if import.name == "tcl_value_new_string" {
            " i32.const 999)"
        } else {
            " unreachable)"
        });
    }
    source.push(')');
    source
}

fn execute(
    module: &mut WasmModule,
    failed: i32,
    status: i32,
    host: bool,
    entry: bool,
    tag: &str,
) -> String {
    let available = std::process::Command::new("wasmtime")
        .arg("--version")
        .output()
        .expect("original emitted binding controls require wasmtime");
    assert!(
        available.status.success(),
        "original emitted binding controls require a working wasmtime: {}",
        String::from_utf8_lossy(&available.stderr)
    );
    let directory =
        std::env::temp_dir().join(format!("tcl_original_bind_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("create fixed control directory");
    let user = directory.join("user.wasm");
    let abi = directory.join("abi.wat");
    let driver = directory.join("driver.wat");
    std::fs::write(&user, module.to_bytes()).expect("write actual emitted module");
    std::fs::write(&abi, provider(module, failed, status, host, entry))
        .expect("write software ABI provider");
    let expected = if failed < 0 && !entry { 999 } else { 0 };
    let first = if host || entry { 11 } else { 0 };
    std::fs::write(&driver, format!(r#"(module
      (import "user" "::sum" (func $sum (param i32 i32 i32) (result i32)))
      (import "tcl" "first" (func $first (result i32)))
      (func (export "run")
        (if (i32.ne (call $sum (i32.const 65) (i32.const 66) (i32.const 67)) (i32.const {expected})) (then unreachable))
        (if (i32.ne (call $first) (i32.const {first})) (then unreachable))))"#)).expect("write control driver");
    let output = std::process::Command::new("wasmtime")
        .arg("run")
        .arg("--preload")
        .arg(format!("tcl={}", abi.display()))
        .arg("--preload")
        .arg(format!("user={}", user.display()))
        .arg("--invoke")
        .arg("run")
        .arg(&driver)
        .output()
        .expect("execute emitted prologue");
    assert!(
        output.status.success(),
        "{tag}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reached = String::from_utf8(output.stdout).expect("ASCII control markers");
    std::fs::remove_dir_all(directory).expect("remove closed control files");
    reached
}

#[test]
fn original_analysed_binding_nonzero_status_retires_unbound_parameters_before_body() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual production emission with a software ABI status/transfer observer; no Native callback or refcount claim.
    let mut module = original_module();
    for (failed, expected) in [(0, "aABCp"), (1, "abBCpA"), (2, "abcCpAB")] {
        let reached = execute(
            &mut module,
            failed,
            1,
            false,
            false,
            &format!("guest_{failed}"),
        );
        assert_eq!(reached, expected);
    }
}

#[test]
fn original_analysed_binding_host_and_entry_failures_finish_parameter_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport

    // The existing Host owner remains independent of the ABI status; software provider cause only.
    let mut module = original_module();
    for (failed, entry, expected) in [
        (0, false, "aABCp"),
        (1, false, "abBCpA"),
        (-1, true, "ABCp"),
    ] {
        let reached = execute(
            &mut module,
            failed,
            -6,
            true,
            entry,
            &format!("host_{failed}_{entry}"),
        );
        assert_eq!(reached, expected);
    }
}

#[test]
fn original_analysed_binding_success_reaches_expression_and_retires_bound_cells() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Positive original source/slot admission reaches the same actual expression emitter.
    let mut module = original_module();
    let reached = execute(&mut module, -1, 0, false, false, "success");
    assert_eq!(reached, "abcggxgxpABC");
}
