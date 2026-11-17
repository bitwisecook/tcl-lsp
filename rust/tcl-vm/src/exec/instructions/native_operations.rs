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

//! Capturing the live command of a literal command head and entering the native compiler operations selected for it.

use super::super::{
    CapturedNativeCommand, EnteredCommand, EnteredNativeOperation, Frame, ScriptNamespace,
    SelectedNativeExecutionTraces, Tick,
};
use super::operands::{label_to_idx, label0};
use crate::command::Command;
use crate::interp::{CommandSidecarKey, Vm, err};
use crate::value::Value;
use tcl_bytecode::{FunctionAsm, Instruction, Op};
use tcl_runtime_api::Completion;

impl Vm {
    /// Select a compiler-hook-capable live callable, excluding execution traces.
    pub(in super::super) fn capture_native_command(
        &mut self,
        binding: &tcl_runtime_api::CommandBindingIdentity,
    ) -> Option<CapturedNativeCommand> {
        if !self.command_binding_matches(binding) {
            return None;
        }
        let (key, command) = self.lookup_compiled_command_binding(binding)?;
        let command = match command {
            Command::Builtin(_) => command,
            Command::Ensemble(ensemble) => {
                ensemble.config().native_implementation()?;
                // Retain the public token, whose generic handler selects the
                // live map and worker after arguments. Capturing its compiler
                // adapter would bypass a replaced private member.
                Command::Ensemble(ensemble)
            }
            _ => return None,
        };
        let builtin_identity = self.stock_native_identity(&key);
        let sidecar_key = CommandSidecarKey::visible(key);
        if self.command_has_execution_trace(&sidecar_key) {
            return None;
        }
        Some(CapturedNativeCommand {
            command,
            builtin_identity,
            sidecar: self.active_sidecar(sidecar_key),
            execution_traces: SelectedNativeExecutionTraces::Omitted,
        })
    }

    /// Capture the callable token for a typed generic surrogate at its literal
    /// head push. This is deliberately before every argument substitution: a
    /// trace, rename, or replacement performed by an earlier word belongs to
    /// the next invocation, not the command Tcl has already entered.
    pub(super) fn capture_entered_command(
        &mut self,
        f: &mut Frame,
        asm: &FunctionAsm,
        instr: &Instruction,
        resume: usize,
    ) -> Result<(), Completion<Value>> {
        let Some(entered) = instr.entered_command.as_ref() else {
            return Ok(());
        };
        let captured = match entered.binding.guard {
            tcl_runtime_api::CommandBindingGuard::ChunkEntry => {
                f.chunk_commands.get(&entered.binding).cloned()
            }
            tcl_runtime_api::CommandBindingGuard::BeforeArguments => {
                self.capture_native_command(&entered.binding)
            }
        };
        let Some(captured) = captured else {
            return Ok(());
        };
        let Some(continuation) = label_to_idx(asm, &f.off2idx, &entered.end) else {
            return Err(err("entered command has no valid continuation"));
        };
        f.entered_commands.push(EnteredCommand {
            resume,
            continuation,
            command: captured.command,
            builtin_identity: captured.builtin_identity,
            sidecar: captured.sidecar,
            execution_traces: captured.execution_traces,
        });
        Ok(())
    }

    /// Whether the release keeps running `instr`'s command as `f` compiled it
    /// across the command-table change that outdated `f`'s bindings, where
    /// this VM would otherwise evaluate the command afresh: the release
    /// compiled every command the command's specialised instructions rely on
    /// into a unit `f` entered before the change ([`crate::release_units`]).
    pub(super) fn release_keeps_running(
        &self,
        f: &Frame,
        asm: &FunctionAsm,
        instr: &Instruction,
    ) -> bool {
        !self.runtime_version().revalidates_running_bytecode()
            && instr.evaluation == tcl_runtime_api::CommandEvaluation::Compiled
            && !self.step_trace_active()
            && f.release_units.current(instr).is_some_and(|unit| {
                !unit.entry.stale && instr.release_hold.is_some_and(|hold| hold.binds(unit.kind))
            })
            && self.function_inlined_facts_hold(asm, f.manifest.as_deref())
    }

    pub(in super::super) fn active_native_bindings_match(
        &self,
        frame: &Frame,
        asm: &FunctionAsm,
    ) -> bool {
        let selected = frame
            .entered_native_operations
            .iter()
            .flat_map(|operation| &operation.requirements)
            .collect::<Vec<_>>();
        self.function_live_command_bindings_match(asm, &selected, frame.manifest.as_deref())
    }

    pub(super) fn enter_native_operations(
        &mut self,
        frame: &mut Frame,
        asm: &FunctionAsm,
        instruction: &Instruction,
    ) -> Option<Tick> {
        for (site_index, site) in instruction.native_operation_selections.iter().enumerate() {
            let continuation = label_to_idx(asm, &frame.off2idx, &site.end)
                .expect("validated native operation continuation");
            let identity = (frame.pc, site_index);
            let selected = frame.chunk_native_operations.contains(&identity)
                && site
                    .compiler_selection_prerequisite()
                    .as_ref()
                    .is_none_or(|required| {
                        required.guard() != tcl_runtime_api::CommandBindingGuard::BeforeArguments
                            || self.native_compiler_selection_prerequisite_matches_at(required)
                    })
                && site
                    .requirements
                    .iter()
                    .filter(|binding| {
                        binding.guard == tcl_runtime_api::CommandBindingGuard::BeforeArguments
                    })
                    .all(|binding| self.command_binding_matches(binding));
            if selected {
                frame
                    .entered_native_operations
                    .retain(|entered| entered.site != identity);
                frame
                    .entered_native_operations
                    .push(EnteredNativeOperation {
                        site: identity,
                        start: frame.pc,
                        end: continuation,
                        requirements: site.requirements.clone(),
                    });
                continue;
            }
            // The neutral operation site retains a rooted source namespace;
            // VM compilation and resolution stacks use its unrooted key.
            let context = site.replay_namespace_context();
            if self.resolve_compiled_namespace_context(&context).is_none() {
                return Some(Tick::Return(err(
                    "native replay namespace context is unavailable",
                )));
            }
            let namespace = context.path();
            let child = match self.compile_plain_function_cached_bytes(
                tcl_runtime_api::ScriptCompileTargetBytes {
                    source: &site.source,
                    namespace,
                },
            ) {
                Ok(child) => child,
                Err(error) => {
                    return Some(Tick::Return(crate::command::completion_from_tcl_error(
                        self, error,
                    )));
                }
            };
            frame.pc = continuation;
            return Some(Tick::PushScript {
                script: Box::new(self.compiled_unit(child, namespace.to_owned())),
                label: None,
                cleanup_proc: None,
                fatal_tail: None,
                namespace: ScriptNamespace::CommandBoundary(context),
            });
        }
        None
    }

    /// Select an independently audited native compiler operation.
    /// The range records selection, rather than repeatedly consulting a live
    /// callable during substitutions that may replace that very callable.
    /// Selections joined to the instruction are checked with its own, as C's
    /// one `INST_START_CMD` checks every command starting there; when any
    /// fails the instruction's own, outermost command is replayed.
    pub(super) fn enter_native_compiler_selection(
        &mut self,
        frame: &mut Frame,
        asm: &FunctionAsm,
        instruction: &Instruction,
    ) -> Option<Tick> {
        if !instruction.selects_native_compiler() {
            return None;
        }
        let mut entered = Vec::new();
        for site in instruction.native_compiler_selections() {
            let Some(continuation) = label_to_idx(asm, &frame.off2idx, &site.end) else {
                return Some(Tick::Return(err(
                    "native compiler selection has no valid continuation",
                )));
            };
            let selected = match site.prerequisite.guard() {
                tcl_runtime_api::CommandBindingGuard::ChunkEntry => frame
                    .chunk_native_compiler_selections
                    .contains(&site.prerequisite),
                tcl_runtime_api::CommandBindingGuard::BeforeArguments => {
                    self.native_compiler_selection_prerequisite_matches_at(&site.prerequisite)
                }
            };
            if !selected {
                entered.clear();
                break;
            }
            entered.push((frame.pc, continuation));
        }
        if !entered.is_empty() {
            frame.entered_native_compiler_selections.extend(entered);
            return None;
        }
        let own_end = match &instruction.native_compiler_selection {
            Some(site) => Some(site.end.as_str()),
            None if instruction.op == Op::START_CMD => label0(instruction),
            None => None,
        };
        let Some(continuation) = own_end.and_then(|end| label_to_idx(asm, &frame.off2idx, end))
        else {
            return Some(Tick::Return(err(
                "native compiler selection has no valid continuation",
            )));
        };
        if self
            .resolve_compiled_namespace_context(&instruction.source_namespace_context())
            .is_none()
        {
            return Some(Tick::Return(err(
                "native replay namespace context is unavailable",
            )));
        }
        let child = match self.compile_plain_function_cached_bytes(
            tcl_runtime_api::ScriptCompileTargetBytes {
                source: &instruction.source_cmd_text,
                namespace: &instruction.source_command_namespace,
            },
        ) {
            Ok(child) => child,
            Err(error) => {
                return Some(Tick::Return(crate::command::completion_from_tcl_error(
                    self, error,
                )));
            }
        };
        frame.pc = continuation;
        Some(Tick::PushScript {
            script: Box::new(
                self.compiled_unit(child, instruction.source_command_namespace.clone()),
            ),
            label: None,
            cleanup_proc: None,
            fatal_tail: None,
            namespace: ScriptNamespace::CommandBoundary(instruction.source_namespace_context()),
        })
    }
}
