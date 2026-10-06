// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Raw emitted opcode inspection and whole-unit compact compiler replay.

use tcl_registry::native_compiler_pass::{
    NativeCompilerPassHazard as Hazard, native_compiler_replays,
};

use super::{CodegenCtx, Op};

fn hazard(op: Op) -> Option<Hazard> {
    Some(match op {
        Op::INVOKE_STK1 | Op::INVOKE_STK4 | Op::INVOKE_EXPANDED | Op::INVOKE_REPLACE => {
            Hazard::Invocation
        }
        Op::EVAL_STK => Hazard::ScriptEvaluation,
        Op::EXPR_STK => Hazard::ExpressionEvaluation,
        Op::YIELD => Hazard::Yield,
        Op::YIELD_TO_INVOKE => Hazard::YieldTo,
        Op::UPVAR => Hazard::Upvar,
        Op::NSUPVAR => Hazard::NamespaceUpvar,
        Op::VARIABLE => Hazard::Variable,
        _ => return None,
    })
}

pub(super) fn capture_first_pass(ctx: &mut CodegenCtx<'_>) {
    ctx.native_compiler_pass_hazards = ctx
        .instructions
        .iter()
        .filter_map(|instruction| hazard(instruction.op))
        .collect();
}

pub(super) fn replay_environment(
    ctx: &CodegenCtx<'_>,
    selected_procedure: bool,
) -> Option<tcl_runtime_api::native_compiler_pass::NativeCompilerPassEnvironment> {
    let entry = ctx.native_entry?;
    let version = entry.execution_point?.tcl_version()?;
    let captured = entry.compiler_pass_environment.as_ref()?;
    let environment = if selected_procedure {
        captured.clone()
    } else {
        captured.without_procedure()
    };
    native_compiler_replays(
        version,
        entry.interpreter,
        Some(&environment),
        ctx.native_compiler_pass_hazards.iter().copied(),
    )
    .then_some(environment)
}

/// Command markers are emission bookkeeping until this boundary. The actual
/// second-pass opcode stream omits them before any peephole/layout operation.
/// Independent original selection receipts stay on the next emitted operation.
pub(super) fn omit_command_markers(ctx: &mut CodegenCtx<'_>) {
    let mut index = 0;
    while index < ctx.instructions.len() {
        if ctx.instructions[index].op != Op::START_CMD {
            index += 1;
            continue;
        }
        let mut marker = ctx.instructions.remove(index);
        let next = ctx
            .instructions
            .get_mut(index)
            .expect("compiler command marker precedes an instruction");
        marker
            .native_operation_selections
            .append(&mut next.native_operation_selections);
        next.native_operation_selections = marker.native_operation_selections;
        if let Some(selection) = marker.native_compiler_selection {
            assert!(
                next.native_compiler_selection.is_none(),
                "distinct compiler selections need separate original operations"
            );
            next.native_compiler_selection = Some(selection);
        }
        if let Some(entered) = marker.entered_command {
            assert!(
                next.entered_command.is_none(),
                "distinct command entries need separate original operations"
            );
            next.entered_command = Some(entered);
        }
        next.no_fold |= marker.no_fold;
        for position in ctx.label_positions.values_mut() {
            if *position > index {
                *position -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_bytecode::{Instruction, Operand};

    #[test]
    fn opcode_hazards_are_captured_before_peepholes() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit(Op::EXPR_STK, vec![]);
        ctx.emit(Op::INVOKE_STK1, vec![Operand::Imm(1)]);
        capture_first_pass(&mut ctx);
        ctx.instructions.clear();
        assert_eq!(
            ctx.native_compiler_pass_hazards,
            [Hazard::ExpressionEvaluation, Hazard::Invocation]
        );
        assert_eq!(hazard(Op::TCLOO_NEXT), None);
    }

    #[test]
    fn compact_pass_omits_markers_without_losing_labels_or_original_selection() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.instructions
            .push(Instruction::new(Op::START_CMD, vec![]));
        ctx.instructions[0].no_fold = true;
        ctx.emit(Op::PUSH1, vec![Operand::Imm(0)]);
        ctx.emit(Op::DONE, vec![]);
        ctx.label_positions.insert("end".into(), 2);
        omit_command_markers(&mut ctx);
        assert_eq!(ctx.instructions.len(), 2);
        assert!(ctx.instructions[0].no_fold);
        assert_eq!(ctx.label_positions["end"], 1);
        assert!(
            !ctx.instructions
                .iter()
                .any(|instruction| instruction.op == Op::START_CMD)
        );
    }
    #[test]
    fn original_array_compiler_replays_with_same_local_table_and_discarded_literal_array() {
        use tcl_runtime_api::CompileService;
        for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            let (_vm, entry) =
                crate::environment_ingress::captured_native_entry_with_owner(profile);
            let compiler = crate::compile_service::BytecodeCompileService::for_profile(profile);
            let parameters = [
                tcl_runtime_api::NameBytes::from("left"),
                tcl_runtime_api::NameBytes::from("right"),
            ];
            let namespace = tcl_runtime_api::ByteNamespacePath::root();
            for (source, locals, passes) in [
                (b"array set a $right".as_slice(), 7, 1),
                (b"array set a $right; missing", 5, 0),
            ] {
                let source = tcl_runtime_api::SourceImage::native(source);
                let module = compiler
                    .compile_procedure_bytes_with_entry(
                        tcl_runtime_api::ProcedureCompileTargetBytes {
                            source: &source,
                            parameters: &parameters,
                            namespace: &namespace,
                        },
                        profile,
                        &entry,
                        tcl_runtime_api::ProcedureDispatch::Optimised,
                    )
                    .unwrap();
                let function = &module.top_level_body;
                assert_eq!(
                    function.lvt.native_slot_names().len(),
                    locals,
                    "{name}: {source:?}"
                );
                assert_eq!(
                    function.lvt.native_slot_names()[..3],
                    [
                        Some(parameters[0].clone()),
                        Some(parameters[1].clone()),
                        Some(tcl_runtime_api::NameBytes::from("a"))
                    ]
                );
                assert!(
                    function.lvt.native_slot_names()[3..]
                        .iter()
                        .all(Option::is_none)
                );
                assert_eq!(function.literals.discarded_native_passes().len(), passes);
                assert_eq!(
                    function.literals.compiler_replay_environment().is_some(),
                    passes != 0
                );
                if passes != 0 {
                    assert!(
                        !function
                            .instructions
                            .iter()
                            .any(|instruction| instruction.op == Op::START_CMD)
                    );
                    assert!(!function.native_compiler_prerequisites.is_empty());
                }
            }
        }
    }
}
