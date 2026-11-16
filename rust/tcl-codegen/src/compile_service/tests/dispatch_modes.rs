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

//! Plain and specialising dispatch: what plain dispatch leaves generic and what a projected profile keeps in both modes.

use super::*;

#[test]
fn service_marks_plain_dispatch_and_preserves_profile() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let fast = service.compile("expr {1 + 2}").unwrap();
    let plain = service.compile_traced("expr {1 + 2}").unwrap();

    assert!(std::ptr::eq(fast.profile, profile));
    assert!(std::ptr::eq(plain.profile, profile));
    assert!(!fast.plain_command_dispatch);
    assert!(plain.plain_command_dispatch);
    assert!(plain.top_level.plain_command_dispatch);
    assert_eq!(
        plain.top_level.command_bindings,
        [] as [tcl_runtime_api::CommandBindingIdentity; 0]
    );
}

#[test]
fn plain_dispatch_does_not_append_builtin_error_semantics() {
    let service = BytecodeCompileService::default();
    let plain = service.compile_traced("error \"boom\"").unwrap();
    let ops: Vec<_> = plain
        .top_level
        .instructions
        .iter()
        .map(|instruction| instruction.op)
        .collect();
    assert_eq!(
        ops.iter()
            .filter(|op| matches!(
                op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            ))
            .count(),
        1,
        "plain compilation must contain one runtime dispatch: {ops:?}"
    );
    assert!(
        !ops.contains(&tcl_bytecode::Op::RETURN_IMM),
        "plain compilation must not append the registry builtin's completion: {ops:?}"
    );
}

#[test]
fn owned_registry_tcl84_projection_keeps_user_throw_fallthrough() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let service = BytecodeCompileService::new(CommandRegistry::build_default());
    let module = service
        .compile_procedure_for_profile(
            ProcedureCompileTarget {
                // Tcl 8.4 has no builtin `throw`, so both spellings can be
                // user procedures and neither may terminate CFG lowering.
                source: "throw; after_throw",
                parameters: &[],
                namespace: "",
            },
            profile,
            ProcedureDispatch::Optimised,
        )
        .unwrap();
    let invoke_count = module
        .top_level
        .instructions
        .iter()
        .filter(|instruction| {
            matches!(
                instruction.op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            )
        })
        .count();
    assert_eq!(
        invoke_count, 2,
        "the call after Tcl 8.4's user-bindable `throw` must remain reachable: {:?}",
        module.top_level.instructions
    );
}

#[test]
fn plain_procedure_dispatch_does_not_specialise_internal_dict_loops() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    for command in ["::tcl::dict::for", "::tcl::dict::map"] {
        let source = format!("{command} {{k v}} {{a 1}} {{set seen $k}}");
        let module = service
            .compile_procedure_for_profile(
                ProcedureCompileTarget {
                    source: &source,
                    parameters: &[],
                    namespace: "",
                },
                profile,
                ProcedureDispatch::Plain,
            )
            .unwrap();
        let ops: Vec<_> = module
            .top_level
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect();
        assert!(
            ops.iter().any(|op| matches!(
                op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            )),
            "{command} must retain runtime command dispatch: {ops:?}"
        );
        assert!(
            !ops.contains(&tcl_bytecode::Op::DICT_FIRST)
                && !ops.contains(&tcl_bytecode::Op::DICT_NEXT),
            "{command} must not use builtin dict-loop opcodes: {ops:?}"
        );
    }
}

#[test]
fn tcl84_profile_projection_preserves_owned_hooks_in_fast_and_plain_modes() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let service = BytecodeCompileService::new(registry_with_custom_list_expr_hook());
    let fast = service
        .compile_for_profile("list {1 + 2}", profile)
        .unwrap();
    let plain = service
        .compile_traced_for_profile("list {1 + 2}", profile)
        .unwrap();

    assert!(std::ptr::eq(fast.profile, profile));
    assert!(std::ptr::eq(plain.profile, profile));
    assert!(
        fast.top_level
            .command_bindings
            .iter()
            .any(|binding| { binding.name == "list" && binding.identity == "list" })
    );
    assert!(
        fast.top_level
            .instructions
            .iter()
            .all(|instruction| instruction.op != tcl_bytecode::Op::INVOKE_STK1),
        "the owned Expr hook must lower the custom list spec: {:?}",
        fast.top_level.instructions,
    );
    assert!(plain.plain_command_dispatch);
    assert!(plain.top_level.plain_command_dispatch);
    assert_eq!(
        plain.top_level.command_bindings,
        [] as [tcl_runtime_api::CommandBindingIdentity; 0]
    );
    assert!(
        plain
            .top_level
            .instructions
            .iter()
            .any(|instruction| instruction.op == tcl_bytecode::Op::INVOKE_STK1)
    );
}
