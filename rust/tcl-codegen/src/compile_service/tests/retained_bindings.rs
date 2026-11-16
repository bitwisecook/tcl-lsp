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

//! The command bindings and native selections a compiled unit retains: escaped heads, inline bodies, hooks and inlined callees.

use super::*;

#[test]
fn retained_string_selection_captures_a_private_name_without_a_public_token() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let module = service
        .compile("string equal -nocase a A")
        .expect("known native ensemble");
    assert!(module.top_level.literals.entries().iter().any(|literal| {
        literal
            .unicode()
            .is_ok_and(|text| text.strip_prefix("::").unwrap_or(text) == "tcl::string::equal")
    }));
    assert!(module.top_level.instructions.iter().any(|instruction| {
        instruction.op == tcl_bytecode::Op::INVOKE_REPLACE
            && instruction.operands
                == [tcl_bytecode::Operand::Imm(5), tcl_bytecode::Operand::Imm(2)]
    }));
    assert!(
        module
            .top_level
            .instructions
            .iter()
            .all(|instruction| instruction.entered_command.is_none())
    );
}

#[test]
fn escaped_heads_retain_decoded_bindings_at_the_native_release_boundary() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let direct = service.compile_for_profile("set x OLD", profile).unwrap();
        assert!(
            direct
                .top_level
                .command_bindings
                .iter()
                .any(|binding| { binding.name == "set" && binding.identity == "set" }),
            "{dialect} direct binding"
        );
        let escaped = service
            .compile_for_profile(r"se\x74 x OLD", profile)
            .unwrap();
        if matches!(dialect, "tcl8.4" | "tcl8.5") {
            assert!(escaped.top_level.command_bindings.is_empty(), "{dialect}");
            assert!(
                escaped.top_level.instructions.iter().any(|instruction| {
                    matches!(
                        instruction.op,
                        tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                    )
                }),
                "{dialect}"
            );
        } else {
            assert!(
                escaped
                    .top_level
                    .command_bindings
                    .iter()
                    .any(|binding| { binding.name == "set" && binding.identity == "set" }),
                "{dialect}: {:?}",
                escaped.top_level.command_bindings
            );
            assert!(
                escaped
                    .top_level
                    .command_bindings
                    .iter()
                    .all(|binding| binding.name != r"se\x74")
            );
        }
    }
}

#[test]
fn lowering_fallback_keeps_the_native_compiler_selection() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let specialised = service
        .compile_for_profile("return value", profile)
        .unwrap();
    assert!(
        specialised
            .top_level
            .command_bindings
            .iter()
            .any(|binding| binding.name == "return" && binding.identity == "return")
    );

    let fallback = service
        .compile_for_profile("return -code ok value", profile)
        .unwrap();
    assert!(
        fallback
            .top_level
            .command_bindings
            .iter()
            .any(|binding| binding.name == "return"),
        "a backend surrogate must retain the native compiled return token: {:?}",
        fallback.top_level.command_bindings,
    );
    assert!(
        fallback
            .top_level
            .instructions
            .iter()
            .any(|instruction| matches!(
                instruction.op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            ))
    );

    // `llength` reaches bytecode as a Call, then its bytecode hook
    // specialises that exact shape. The later consumer must retain the
    // binding itself rather than inheriting one from lowering.
    let codegen_specialised = service
        .compile_for_profile("llength {a b}", profile)
        .unwrap();
    assert!(
        codegen_specialised
            .top_level
            .command_bindings
            .iter()
            .any(|binding| binding.name == "llength" && binding.identity == "llength"),
        "a codegen specialisation must retain its own exact binding: {:?}",
        codegen_specialised.top_level.command_bindings,
    );
}

#[test]
fn catch_keeps_its_binding_and_exact_replay_boundary_after_cfg_lowering() {
    let source = "proc p {} {mutate; catch {error x} msg; return $msg}";
    let catch_source = "catch {error x} msg";
    let module = BytecodeCompileService::default().compile(source).unwrap();
    let procedure = module
        .procedures
        .get(&fixture_id(&module.procedures, "::p"))
        .expect("authored fixture body");

    assert!(
        procedure
            .command_bindings
            .iter()
            .any(|binding| { binding.name == "catch" && binding.identity == "catch" })
    );
    assert!(procedure.instructions.iter().any(|instruction| {
        instruction.op == tcl_bytecode::Op::START_CMD && instruction.source_cmd_text == catch_source
    }));
}

#[test]
fn nested_inline_bodies_retain_every_consumed_command_binding() {
    let module = BytecodeCompileService::default()
            .compile(
                "proc ret {} {return [expr {1 + 2}]}\n\
                 proc caught {} {set rc [catch {expr {1 + 2}} value]; list $rc $value}\n\
                 proc tried {} {set rc [catch {try {expr {1 + 2}} on error {m} {set m handled}} value]; list $rc $value}\n\
                 proc returning {} {set rc [catch {return value} result]; list $rc $result}\n\
                 proc erroring {} {set rc [catch {error boom} result]; list $rc $result}\n\
                 proc breaking {} {set rc [catch {break} result]; list $rc $result}\n\
                 proc continuing {} {set rc [catch {continue} result]; list $rc $result}",
            )
            .unwrap();

    for procedure in ["::ret", "::caught", "::tried"] {
        let bindings = &module
            .procedures
            .get(&fixture_id(&module.procedures, procedure))
            .expect("authored fixture body")
            .command_bindings;
        assert!(
            bindings
                .iter()
                .any(|binding| binding.name == "expr" && binding.identity == "expr"),
            "{procedure} lost its nested expr dependency: {bindings:?}",
        );
    }
    assert!(
        module
            .procedures
            .get(&fixture_id(&module.procedures, "::tried"))
            .expect("authored fixture body")
            .command_bindings
            .iter()
            .any(|binding| binding.name == "try" && binding.identity == "try"),
        "the catch-body try specialisation must retain its own dependency",
    );
    assert!(
        module
            .procedures
            .get(&fixture_id(&module.procedures, "::tried"))
            .expect("authored fixture body")
            .command_bindings
            .iter()
            .any(|binding| binding.name == "set" && binding.identity == "set"),
        "the directly-emitted try handler must retain its lowering dependency",
    );
    for (procedure, dependency) in [
        ("::returning", "return"),
        ("::erroring", "error"),
        ("::breaking", "break"),
        ("::continuing", "continue"),
    ] {
        let bindings = &module
            .procedures
            .get(&fixture_id(&module.procedures, procedure))
            .expect("authored fixture body")
            .command_bindings;
        assert!(
            bindings
                .iter()
                .any(|binding| { binding.name == dependency && binding.identity == dependency }),
            "{procedure} lost its {dependency} inline dependency: {bindings:?}",
        );
    }
}

#[test]
fn native_operation_metadata_is_specialisation_scoped() {
    let service = BytecodeCompileService::default();
    let eligible = service.compile("set result [llength [pid]]").unwrap();
    let generic = service.compile("set result [pid]").unwrap();
    let wrong_arity = service.compile("set result [llength [pid] extra]").unwrap();
    let traced = service
        .compile_traced("set result [llength [pid]]")
        .unwrap();

    let selected_lengths = |module: &tcl_bytecode::ModuleAsm| {
        module
            .top_level
            .instructions
            .iter()
            .flat_map(|instruction| &instruction.native_operation_selections)
            .filter(|operation| {
                operation
                    .requirements
                    .iter()
                    .any(|binding| binding.name == "llength" && binding.identity == "llength")
            })
            .count()
    };
    assert_eq!(selected_lengths(&eligible), 1);
    assert_eq!(selected_lengths(&generic), 0);
    assert_eq!(selected_lengths(&wrong_arity), 0);
    assert_eq!(selected_lengths(&traced), 0);
}

#[test]
fn every_applicable_typed_codegen_hook_retains_operation_selection() {
    let cases = [
        ("lassign", "lassign {a b} a b"),
        // Keep const-foldable hooks dynamic: this test exercises the
        // entered-token codegen path, while fold provenance is covered by
        // the constant-substitution tests.
        ("llength", "llength $value"),
        ("lset", "lset value 0 x"),
        ("dict", "dict set value key x"),
        ("append", "append value x"),
        ("lappend", "lappend value x"),
        ("unset", "unset value"),
        ("tailcall", "tailcall target"),
        ("concat", "concat $value b"),
        ("global", "global value"),
        ("upvar", "upvar 1 remote local"),
    ];
    let service = BytecodeCompileService::default();
    for (name, command) in cases {
        let module = service
            .compile(&format!(
                "proc target {{}} {{}}; proc p {{}} {{set result [{command}]}}"
            ))
            .unwrap();
        let operation = module
            .procedures
            .get(&fixture_id(&module.procedures, "::p"))
            .expect("authored fixture body")
            .instructions
            .iter()
            .flat_map(|instruction| &instruction.native_operation_selections)
            .find(|operation| {
                operation
                    .requirements
                    .iter()
                    .any(|binding| binding.name == name && binding.identity == name)
            });
        let operation = operation.unwrap_or_else(|| {
            panic!(
                "{name}: proved inline hook retains its operation range; requirements={:?}",
                module
                    .procedures
                    .get(&fixture_id(&module.procedures, "::p"))
                    .expect("authored fixture body")
                    .command_bindings
            )
        });
        assert_eq!(operation.source, command);
        assert!(
            module
                .procedures
                .get(&fixture_id(&module.procedures, "::p"))
                .expect("authored fixture body")
                .labels
                .contains_key(&operation.end)
        );
        assert!(
            module
                .procedures
                .get(&fixture_id(&module.procedures, "::p"))
                .expect("authored fixture body")
                .command_bindings
                .iter()
                .any(|binding| binding.name == name && binding.identity == name),
            "{name} metadata was not retained as a function dependency"
        );
    }
}

#[test]
fn inline_hooks_with_whole_unit_mutation_specialise_and_retain_binding() {
    let cases = [
        (
            "lrange",
            "lrange {a b c} 0 1",
            tcl_bytecode::Op::LIST_RANGE_IMM,
        ),
        ("linsert", "linsert {a b} 1 x", tcl_bytecode::Op::LREPLACE4),
    ];
    let service = BytecodeCompileService::default();
    for (name, command, expected_op) in cases {
        let module = service
            .compile(&format!(
                "proc mutate {{}} {{rename {name} saved_{name}}}; \
                     proc p {{}} {{set result [{command}]}}"
            ))
            .unwrap();
        assert!(
            module
                .procedures
                .get(&fixture_id(&module.procedures, "::p"))
                .expect("authored fixture body")
                .instructions
                .iter()
                .any(|instruction| instruction.op == expected_op),
            "{name} lost its inline specialisation"
        );
        assert!(
            module
                .procedures
                .get(&fixture_id(&module.procedures, "::p"))
                .expect("authored fixture body")
                .command_bindings
                .iter()
                .any(|binding| binding.name == name && binding.identity == name),
            "{name} lost its runtime-validated command binding"
        );
        assert!(
            module
                .procedures
                .get(&fixture_id(&module.procedures, "::p"))
                .expect("authored fixture body")
                .instructions
                .iter()
                .all(|instruction| {
                    !matches!(
                        instruction.op,
                        tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                    )
                }),
            "{name} unexpectedly fell back to generic dispatch"
        );
    }
}

#[test]
fn return_expr_alias_retains_the_source_binding_identity() {
    let module = BytecodeCompileService::default()
        .compile("interp alias {} e {} expr; proc p {} {return [e {1 + 2}]}")
        .unwrap();
    assert!(
        module
            .procedures
            .get(&fixture_id(&module.procedures, "::p"))
            .expect("authored fixture body")
            .command_bindings
            .iter()
            .any(|binding| binding.name == "e" && binding.identity == "expr"),
        "a fused return dependency belongs to the alias source spelling: {:?}",
        module
            .procedures
            .get(&fixture_id(&module.procedures, "::p"))
            .expect("authored fixture body")
            .command_bindings,
    );
}

#[test]
fn foreach_line_binding_dependency_follows_the_resolved_release_profile() {
    let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let tcl86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let source = "foreachLine line file.txt {set seen $line}";

    let current = BytecodeCompileService::for_profile(tcl90)
        .compile(source)
        .unwrap();
    assert_eq!(
        current.top_level.command_bindings,
        [] as [tcl_runtime_api::CommandBindingIdentity; 0]
    );
    assert!(std::ptr::eq(current.profile, tcl90));
    assert!(current.top_level.instructions.iter().any(|instruction| {
        matches!(
            instruction.op,
            tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
        ) && instruction.source_cmd_text == source
    }));

    let legacy = BytecodeCompileService::for_profile(tcl86)
        .compile(source)
        .unwrap();
    assert!(
        legacy
            .top_level
            .command_bindings
            .iter()
            .all(|binding| binding.name != "foreachLine"),
        "an unavailable Tcl 9 command must not acquire an 8.6 binding: {:?}",
        legacy.top_level.command_bindings,
    );
}

#[test]
fn inlining_preserves_callee_structured_bindings_through_cfg_and_codegen() {
    let source = "proc direct {enabled} {while {$enabled} {}}\n\
                      proc wrapped {enabled} {if {$enabled} {set seen 1}; return $enabled}\n\
                      direct 0\n\
                      wrapped 1\n\
                      set done 1";
    let registry = CommandRegistry::build_default();
    let lowered = tcl_compiler::lowering::lower_to_ir_for_bytecode(source, &registry);
    let inlined = crate::inlining::inline_module(lowered, &registry);

    assert!(
        inlined.top_level.statements.iter().all(|statement| {
            !matches!(
                statement,
                tcl_compiler::ir::Statement::Call { command, .. }
                    if command == "direct" || command == "wrapped"
            )
        }),
        "the regression must exercise both actual inline splices: {:?}",
        inlined.top_level.statements,
    );

    let cfg = build_cfg_codegen_with_registry(&inlined, false, &registry);
    let module = codegen_module(&cfg, &inlined, &registry).expect("original source namespace");
    for name in ["while", "if"] {
        assert!(
            module
                .top_level
                .command_bindings
                .iter()
                .any(|binding| binding.name == name && binding.identity == name),
            "inlining dropped {name:?} before codegen: {:?}",
            module.top_level.command_bindings,
        );
    }
}
