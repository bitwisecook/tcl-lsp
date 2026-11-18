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

//! Raw emitted opcode inspection and whole-unit compact compiler replay.

use tcl_registry::native_compiler_pass::{
    NativeCompilerPassHazard as Hazard, native_compiler_replays,
};

use super::statements::SC_GENERIC_TAG;
use super::{CodegenCtx, Op};
use tcl_bytecode::Operand;

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

/// A generically invoked command's boundary carrier, as
/// `schedule_native_generic` emits it: C compiles no instruction for an
/// invoked command's start (`tcl::unsupported::disassemble` shows its first
/// word's push first), so the next instruction takes over the boundary.
pub(super) fn omit_generic_boundary_carriers(ctx: &mut CodegenCtx<'_>) {
    let mut index = 0;
    while index + 1 < ctx.instructions.len() {
        let carrier = &ctx.instructions[index];
        let is_carrier = carrier.op == Op::NOP
            && carrier.operands.is_empty()
            && carrier.comment.is_empty()
            && carrier.no_fold
            && carrier.source_command_boundary.is_start()
            && carrier.native_compiler_selection.is_none()
            && carrier.native_operation_selections.is_empty()
            && carrier.entered_command.is_none()
            && carrier.completion_option_scope.is_none();
        if !is_carrier || ctx.instructions[index + 1].op == Op::START_CMD {
            index += 1;
            continue;
        }
        let carrier = ctx.instructions.remove(index);
        let next = &mut ctx.instructions[index];
        next.no_fold = true;
        next.source_command_boundary = carrier.source_command_boundary;
        if next.source_cmd_text.is_empty() {
            next.source_cmd_text = carrier.source_cmd_text;
            next.source_span = next.source_span.or(carrier.source_span);
            next.source_command_namespace = carrier.source_command_namespace;
            next.source_command_namespace_context = carrier.source_command_namespace_context;
        }
        for position in ctx.label_positions.values_mut() {
            if *position > index {
                *position -= 1;
            }
        }
    }
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

/// Place command markers where C's `CompileCmdCompileProc` puts
/// `INST_START_CMD` (tclCompile.c `atCmdStart`, 8.5.19 through 9.1.0): one
/// before each command it compiles, none for a command it invokes, none at
/// the start of the bytecode, and none straight after another marker, whose
/// count grows instead. The compact second pass (`atCmdStart == 2`) emits
/// none at all.
///
/// A marker that C omits gives its replay metadata to the next instruction,
/// which the VM checks as the same source boundary. Where that instruction
/// already owns another command's metadata the marker stays. A nested
/// command's selection that starts on a marker joins it, and the marker's
/// count grows, so a stale one replays the outermost command as C's does.
pub(super) fn place_command_starts(ctx: &mut CodegenCtx<'_>) {
    collapse_repeated_starts(ctx);
    let revalidates = ctx
        .native_entry
        .and_then(|entry| entry.execution_point?.tcl_version())
        .is_none_or(tcl_dialect::TclVersion::revalidates_running_bytecode);
    if !revalidates {
        omit_release_markers(ctx);
        return;
    }
    let mut index = 0;
    while index < ctx.instructions.len() {
        let instruction = &ctx.instructions[index];
        let selection =
            instruction.op == Op::NOP && instruction.native_compiler_selection.is_some();
        if instruction.op != Op::START_CMD && !selection {
            index += 1;
            continue;
        }
        let generic = instruction.op == Op::START_CMD
            && instruction.comment == SC_GENERIC_TAG
            && instruction.native_compiler_selection.is_none()
            && instruction.native_operation_selections.is_empty();
        let inline_replay = instruction.source_command_boundary.is_inline_replay();
        let follows_start = index > 0 && ctx.instructions[index - 1].op == Op::START_CMD;
        if !ctx.compact_compiler_pass && !generic && !follows_start && index > 0 {
            if selection {
                materialise_start(&mut ctx.instructions[index]);
            }
            index += 1;
            continue;
        }
        // The preceding marker's replay covers an inline command that starts
        // at the same instruction, so nothing is lost by dropping it there.
        let merged = !ctx.compact_compiler_pass && !generic && follows_start;
        let covered = inline_replay
            && merged
            && ctx.instructions[index]
                .native_operation_selections
                .is_empty();
        if !covered && !receives_marker(&ctx.instructions, index) {
            let opens_body =
                (ctx.compact_compiler_pass || index == 0) && opens_nested(&ctx.instructions, index);
            if (merged && joins_previous(&ctx.instructions, index)) || opens_body {
                let marker = ctx.instructions.remove(index);
                for position in ctx.label_positions.values_mut() {
                    if *position > index {
                        *position -= 1;
                    }
                }
                if opens_body {
                    open_with_host(marker, &mut ctx.instructions[index]);
                } else {
                    join_start(marker, &mut ctx.instructions[index - 1]);
                }
                continue;
            }
            if selection && !ctx.compact_compiler_pass && index > 0 && !follows_start {
                materialise_start(&mut ctx.instructions[index]);
            }
            index += 1;
            continue;
        }
        let marker = ctx.instructions.remove(index);
        for position in ctx.label_positions.values_mut() {
            if *position > index {
                *position -= 1;
            }
        }
        let same_command =
            ctx.instructions[index - usize::from(merged)].source_cmd_text == marker.source_cmd_text;
        if merged && !same_command {
            let count = start_count(&marker);
            if let Some(Operand::Imm(previous)) = ctx.instructions[index - 1].operands.get_mut(1) {
                *previous += count;
            }
        }
        if !covered {
            give_marker(marker, &mut ctx.instructions[index]);
        }
    }
}

/// A compiled statement's wrapper and the command's own marker, or its
/// selection, start the same source command at the same instruction; C has
/// one marker there, which carries the selection with the same replay text.
fn collapse_repeated_starts(ctx: &mut CodegenCtx<'_>) {
    let mut index = 1;
    while index < ctx.instructions.len() {
        let previous = &ctx.instructions[index - 1];
        let marker = &ctx.instructions[index];
        let own_selection = marker.op == Op::NOP
            && marker.native_compiler_selection.is_some()
            && previous.native_compiler_selection.is_none();
        let repeated = previous.op == Op::START_CMD
            && (marker.op == Op::START_CMD || own_selection)
            && marker.source_command_boundary == previous.source_command_boundary
            && marker.source_cmd_text == previous.source_cmd_text
            && marker.source_command_namespace == previous.source_command_namespace
            && marker.source_command_namespace_context == previous.source_command_namespace_context
            && (own_selection || marker.native_compiler_selection.is_none())
            && marker.entered_command.is_none()
            && marker.completion_option_scope.is_none();
        if !repeated {
            index += 1;
            continue;
        }
        let mut marker = ctx.instructions.remove(index);
        ctx.instructions[index - 1]
            .native_operation_selections
            .append(&mut marker.native_operation_selections);
        if marker.native_compiler_selection.is_some() {
            ctx.instructions[index - 1].native_compiler_selection =
                marker.native_compiler_selection;
        }
        if marker.comment == SC_GENERIC_TAG {
            ctx.instructions[index - 1].comment = marker.comment;
        }
        ctx.instructions[index - 1].no_fold |= marker.no_fold;
        for position in ctx.label_positions.values_mut() {
            if *position > index {
                *position -= 1;
            }
        }
    }
}

/// Tcl 8.4.20 has no `INST_START_CMD`: running bytecode never revalidates,
/// so no marker survives. One whose replay metadata the next instruction of
/// the same command cannot take stays as a `nop` holding it.
fn omit_release_markers(ctx: &mut CodegenCtx<'_>) {
    let mut index = 0;
    while index < ctx.instructions.len() {
        if ctx.instructions[index].op != Op::START_CMD {
            index += 1;
            continue;
        }
        if !receives_marker(&ctx.instructions, index) {
            let kept = &mut ctx.instructions[index];
            kept.op = Op::NOP;
            kept.operands.clear();
            index += 1;
            continue;
        }
        let marker = ctx.instructions.remove(index);
        for position in ctx.label_positions.values_mut() {
            if *position > index {
                *position -= 1;
            }
        }
        give_marker(marker, &mut ctx.instructions[index]);
    }
}

fn start_count(marker: &tcl_bytecode::Instruction) -> i32 {
    match marker.operands.get(1) {
        Some(Operand::Imm(count)) if marker.op == Op::START_CMD => *count,
        _ => 1,
    }
}

fn materialise_start(instruction: &mut tcl_bytecode::Instruction) {
    let end = instruction
        .native_compiler_selection
        .as_ref()
        .expect("selection marker")
        .end
        .clone();
    instruction.op = Op::START_CMD;
    instruction.operands = vec![Operand::Label(end), Operand::Imm(1)];
}

/// Whether the instruction after `index` can carry the marker's replay
/// metadata as the same source command.
fn receives_marker(instructions: &[tcl_bytecode::Instruction], index: usize) -> bool {
    let marker = &instructions[index];
    let Some(next) = instructions.get(index + 1) else {
        return false;
    };
    !marker.source_command_boundary.is_inline_replay()
        && next.op != Op::START_CMD
        && !(next.op == Op::NOP && next.native_compiler_selection.is_some())
        && (marker.native_compiler_selection.is_none() || next.native_compiler_selection.is_none())
        && (marker.entered_command.is_none() || next.entered_command.is_none())
        && (marker.completion_option_scope.is_none() || next.completion_option_scope.is_none())
        && (!marker.source_command_boundary.is_start()
            || (!next.source_command_boundary.is_start()
                && !next.source_command_boundary.is_inline_replay()))
        && next.source_cmd_text == marker.source_cmd_text
        && next.source_command_namespace == marker.source_command_namespace
        && next.source_command_namespace_context == marker.source_command_namespace_context
}

/// Whether the selection at `index` belongs to a command nested in the
/// command whose marker precedes it, so C's one `INST_START_CMD` counts both.
fn joins_previous(instructions: &[tcl_bytecode::Instruction], index: usize) -> bool {
    let marker = &instructions[index];
    let host = &instructions[index - 1];
    marker.native_compiler_selection.is_some()
        && marker.entered_command.is_none()
        && marker.completion_option_scope.is_none()
        && marker.source_cmd_text != host.source_cmd_text
        && nests_in(host, marker)
}

/// Whether a marker C does not emit (the body's first, or any in the compact
/// pass) starts the same instruction as a command nested in its own, which
/// can then carry it as a host.
fn opens_nested(instructions: &[tcl_bytecode::Instruction], index: usize) -> bool {
    let marker = &instructions[index];
    let selection = marker.op == Op::NOP && marker.native_compiler_selection.is_some();
    (marker.op == Op::START_CMD || selection)
        && host_end(marker).is_some()
        && marker.joined_compiler_selections.is_empty()
        && marker.entered_command.is_none()
        && marker.completion_option_scope.is_none()
        && !marker.source_command_boundary.is_inline_replay()
        && instructions.get(index + 1).is_some_and(|next| {
            next.source_command_boundary.is_start()
                && next.op != Op::START_CMD
                && next.start_hosts.is_empty()
                && next.source_cmd_text != marker.source_cmd_text
                && nests_in(marker, next)
        })
}

/// Where the marker's command ends: its selection's continuation, else the
/// marker's own.
fn host_end(marker: &tcl_bytecode::Instruction) -> Option<String> {
    if let Some(site) = &marker.native_compiler_selection {
        return Some(site.end.clone());
    }
    match marker.operands.first() {
        Some(Operand::Label(end)) if marker.op == Op::START_CMD => Some(end.clone()),
        _ => None,
    }
}

/// The nested start carries the omitted marker's command, and the marker's
/// own hosts, as its hosts; the outer operation selections stay first, so the
/// outermost is checked first.
fn open_with_host(mut marker: tcl_bytecode::Instruction, next: &mut tcl_bytecode::Instruction) {
    let end = host_end(&marker).expect("checked by opens_nested");
    marker
        .native_operation_selections
        .append(&mut next.native_operation_selections);
    next.native_operation_selections = marker.native_operation_selections;
    next.no_fold |= marker.no_fold;
    next.start_hosts = marker.start_hosts;
    next.start_hosts.push(tcl_bytecode::CommandStartHost {
        selection: marker.native_compiler_selection,
        source: marker.source_cmd_text,
        namespace: marker.source_command_namespace,
        namespace_context: marker.source_command_namespace_context,
        end,
    });
}

fn nests_in(outer: &tcl_bytecode::Instruction, inner: &tcl_bytecode::Instruction) -> bool {
    outer
        .source_span
        .zip(inner.source_span)
        .is_some_and(|(outer, inner)| outer.start() <= inner.start() && inner.end() <= outer.end())
}

/// Fold a nested command's selection into the marker it starts with. The
/// host's own operation selections stay first, so the outermost is checked
/// first.
fn join_start(mut marker: tcl_bytecode::Instruction, host: &mut tcl_bytecode::Instruction) {
    let count = start_count(&marker);
    if let Some(Operand::Imm(previous)) = host.operands.get_mut(1) {
        *previous += count;
    }
    host.native_operation_selections
        .append(&mut marker.native_operation_selections);
    host.joined_compiler_selections
        .extend(marker.native_compiler_selection);
    host.joined_compiler_selections
        .append(&mut marker.joined_compiler_selections);
    host.no_fold |= marker.no_fold;
}

fn give_marker(mut marker: tcl_bytecode::Instruction, next: &mut tcl_bytecode::Instruction) {
    marker
        .native_operation_selections
        .append(&mut next.native_operation_selections);
    next.native_operation_selections = marker.native_operation_selections;
    if marker.native_compiler_selection.is_some() {
        next.native_compiler_selection = marker.native_compiler_selection;
    }
    next.joined_compiler_selections
        .append(&mut marker.joined_compiler_selections);
    marker.start_hosts.append(&mut next.start_hosts);
    next.start_hosts = marker.start_hosts;
    if marker.entered_command.is_some() {
        next.entered_command = marker.entered_command;
    }
    if marker.completion_option_scope.is_some() {
        next.completion_option_scope = marker.completion_option_scope;
    }
    if marker.source_command_boundary.is_start() {
        next.source_command_boundary = marker.source_command_boundary;
    }
    next.no_fold |= marker.no_fold;
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
            let (_vm, entry) = crate::test_support::captured_native_entry_with_owner(profile);
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
                    assert_ne!(function.native_compiler_prerequisites, [] as [tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite; 0]);
                }
            }
        }
    }
}
