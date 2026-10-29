// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Owned application preparation: a private owner's candidate, compiled once
//! by this VM's installed service and admitted by one closure check before it
//! runs. Nothing here issues a closed-consumer scope.

use std::rc::Rc;

use tcl_bytecode::{FunctionAsm, Instruction, LiteralRep, ModuleAsm, Op, Operand};
use tcl_runtime_api::closed_native_consumer::ClosedSourceRefusal;
use tcl_runtime_api::native_compilation::NativeCompilationEntry;

use super::{ClosedModuleDomain, check_module};
use crate::interp::{Code, Vm};

/// The returned module of one installed-service compilation, with the entry,
/// compiler generation and service it was produced under. Callers cannot
/// construct one.
pub struct PreparedOwnedApplicationSource {
    module: ModuleAsm,
    entry: NativeCompilationEntry,
    compiler_generation: u64,
    service: *const (),
}

impl PreparedOwnedApplicationSource {
    /// The admitted module, for inspection only.
    #[must_use]
    pub fn module(&self) -> &ModuleAsm {
        &self.module
    }
}

impl Vm {
    /// Whether the compiled-module caches a preparation could be served from
    /// are empty: `eval_cache`, `eval_cache_plain` and `module_procs`. The
    /// value-attached native bytecode cache cannot be enumerated; a fresh VM
    /// whose prepared module runs through
    /// [`Self::execute_owned_application_preparation`] never consults it.
    #[must_use]
    pub fn owned_application_cache_scope_is_empty(&self) -> bool {
        self.eval_cache.is_empty()
            && self.eval_cache_plain.is_empty()
            && self.module_procs.is_empty()
    }

    fn installed_service(&self) -> Option<*const ()> {
        self.compiler
            .as_ref()
            .map(|service| Rc::as_ptr(service).cast::<()>())
    }

    /// Compile `source` once through this interpreter's installed service and
    /// admit the returned module under the application closure. The result
    /// carries no closed-consumer permission.
    ///
    /// # Errors
    /// Refuses a non-empty cache scope, a service refusal, any module outside
    /// the application domain, and any state, generation or service change.
    pub fn prepare_owned_application_source(
        &self,
        source: &str,
        commands: &[String],
        scalars: &[String],
        observation: &NativeCompilationEntry,
    ) -> Result<PreparedOwnedApplicationSource, ClosedSourceRefusal> {
        if !self.owned_application_cache_scope_is_empty() {
            return Err(ClosedSourceRefusal::Currency);
        }
        let compiler_generation = self.compiler_generation;
        let service = self
            .installed_service()
            .ok_or(ClosedSourceRefusal::NativeContext)?;
        let before = self.observe_closed_native_state(commands, scalars, observation)?;
        let image = tcl_runtime_api::SourceImage::native(source.as_bytes());
        let namespace = self.source_namespace_path();
        let module = self
            .compiler
            .as_ref()
            .ok_or(ClosedSourceRefusal::NativeContext)?
            .compile_plain_script_bytes_with_entry(
                tcl_runtime_api::ScriptCompileTargetBytes {
                    source: &image,
                    namespace: &namespace,
                },
                self.source_profile(),
                &before,
            )
            .map_err(|_| ClosedSourceRefusal::Source)?;
        check_module(
            &module,
            &image,
            &namespace,
            self.source_profile(),
            ClosedModuleDomain::Application(self.lexer_config()),
        )?;
        self.validate_module_profile(&module)
            .map_err(|_| ClosedSourceRefusal::NativeContext)?;
        for function in [&module.top_level, &module.top_level_body] {
            function
                .validate_native_compilation_entry()
                .map_err(|_| ClosedSourceRefusal::NativeContext)?;
            if !self.native_compiler_prerequisites_match(function)
                || !self.native_math_table_prerequisite_matches(function)
            {
                return Err(ClosedSourceRefusal::Currency);
            }
        }
        let after = self.observe_closed_native_state(commands, scalars, observation)?;
        if before != after
            || self.compiler_generation != compiler_generation
            || self.installed_service() != Some(service)
            || !self.owned_application_cache_scope_is_empty()
        {
            return Err(ClosedSourceRefusal::Currency);
        }
        Ok(PreparedOwnedApplicationSource {
            module,
            entry: after,
            compiler_generation,
            service,
        })
    }

    /// Run exactly the prepared module once. No completion value escapes.
    ///
    /// # Errors
    /// Refuses a changed entry, compiler generation, service or cache scope,
    /// and any completion other than a normal one: an error, a top-level
    /// `return`, a host refusal, or a limit breach even when the script
    /// caught it.
    pub fn execute_owned_application_preparation(
        &mut self,
        prepared: PreparedOwnedApplicationSource,
        commands: &[String],
        scalars: &[String],
    ) -> Result<(), ClosedSourceRefusal> {
        let current = self.observe_closed_native_state(commands, scalars, &prepared.entry)?;
        if current != prepared.entry
            || self.compiler_generation != prepared.compiler_generation
            || self.installed_service() != Some(prepared.service)
            || !self.owned_application_cache_scope_is_empty()
        {
            return Err(ClosedSourceRefusal::Currency);
        }
        let completion = self.run_current_module_at(&prepared.module, None);
        if self.limit_exceeded().is_some()
            || completion.code != Code::Ok
            || self.execution_refusal.is_some()
        {
            return Err(ClosedSourceRefusal::Execution);
        }
        // The preparation is single use: it retires with this execution.
        drop(completion);
        drop(prepared);
        Ok(())
    }
}

/// The application closure over one prepared function: plain dispatch of
/// literal words and scalar reads, with every auxiliary field empty. Only
/// the top-level body has local slots, each a plain source scalar.
pub(super) fn check_function(
    function: &FunctionAsm,
    body: bool,
    config: tcl_lexer::LexerConfig,
) -> Result<(), ClosedSourceRefusal> {
    let FunctionAsm {
        name: _,
        native_compilation_failure,
        native_compilation_preflight: _,
        native_math_table_prerequisite,
        native_compiler_prerequisites,
        required_compiled_local_layout,
        literals,
        lvt,
        instructions,
        labels,
        loop_targets,
        body_base_line: _,
        proc_body_src,
        error_regions,
        plain_command_dispatch,
        command_bindings,
        procedure_bindings,
        site_claims,
    } = function;
    // Only the emitter's structural entry/command-end/exit labels; they are
    // inert because no jump opcode is admitted below.
    if !labels.keys().all(|label| structural_label(label))
        || native_compilation_failure.is_some()
        || native_math_table_prerequisite.is_some()
        || !native_compiler_prerequisites.is_empty()
        || required_compiled_local_layout.is_some()
        || !slots_are_plain_scalars(lvt, body)
        || !loop_targets.is_empty()
        || proc_body_src.is_some()
        || !error_regions.is_empty()
        || !*plain_command_dispatch
        || !command_bindings.is_empty()
        || !procedure_bindings.is_empty()
        || !site_claims.is_empty()
        || instructions.len() > 4096
    {
        return Err(ClosedSourceRefusal::Source);
    }
    for instruction in instructions {
        if !instruction_is_closed(instruction, body)
            || (matches!(instruction.op, Op::PUSH1 | Op::PUSH4)
                && !push_is_literal(instruction, literals, config))
            || (matches!(instruction.op, Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4)
                && !slot_operand_is_bound(instruction, lvt.len()))
        {
            return Err(ClosedSourceRefusal::Source);
        }
    }
    Ok(())
}

fn slots_are_plain_scalars(lvt: &tcl_bytecode::LocalVarTable, body: bool) -> bool {
    (body || lvt.is_empty())
        && lvt.native_slot_names().iter().all(|name| {
            name.as_ref().is_some_and(|name| {
                let bytes = name.as_bytes();
                !bytes.is_empty()
                    && !bytes.contains(&b'(')
                    && !bytes.windows(2).any(|pair| pair == b"::")
            })
        })
}

fn slot_operand_is_bound(instruction: &Instruction, slots: usize) -> bool {
    matches!(
        instruction.operands.as_slice(),
        [Operand::Imm(slot)] if usize::try_from(*slot).is_ok_and(|slot| slot < slots)
    )
}

fn structural_label(label: &str) -> bool {
    ["entry_", "cmd_end_", "exit_"].iter().any(|prefix| {
        label.strip_prefix(prefix).is_some_and(|ordinal| {
            !ordinal.is_empty() && ordinal.bytes().all(|b| b.is_ascii_digit())
        })
    })
}

fn instruction_is_closed(instruction: &Instruction, body: bool) -> bool {
    // Exhaustive: a new executable field must be classified here.
    let Instruction {
        op,
        operands: _,
        comment: _,
        offset: _,
        jump_table,
        native_switch_bytes,
        native_switch_integers,
        native_switch_version,
        native_string_index_purpose,
        native_fixed_math_call,
        no_fold: _,
        source_line: _,
        source_value_line: _,
        source_cmd_text: _,
        error_stack_context: _,
        error_info_command: _,
        error_origin: _,
        source_command_namespace: _,
        source_command_namespace_context: _,
        source_command_boundary: _,
        completion_option_scope,
        source_span: _,
        foreach_vars,
        native_each,
        native_list_index,
        native_list_range,
        native_list_insertion,
        foreach_collect,
        dict_vars,
        push_verbatim: _,
        literal_rep: _,
        evaluation: _,
        release_binding,
        release_hold,
        script_command,
        release_body,
        release_body_entry,
        entered_command,
        native_compiler_selection,
        native_operation_selections,
        catch_target,
        catch_start,
        catch_end,
        release_compiled,
    } = instruction;
    let allowed = matches!(
        op,
        Op::NOP
            | Op::PUSH1
            | Op::PUSH4
            | Op::POP
            | Op::DUP
            | Op::LOAD_STK
            | Op::LOAD_SCALAR_STK
            | Op::INVOKE_STK1
            | Op::INVOKE_STK4
            | Op::DONE
            | Op::START_CMD
    ) || (body && matches!(op, Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4));
    allowed
        && jump_table.is_none()
        && native_switch_bytes.is_none()
        && native_switch_integers.is_none()
        && native_switch_version.is_none()
        && native_string_index_purpose.is_none()
        && native_fixed_math_call.is_none()
        && completion_option_scope.is_none()
        && foreach_vars.is_none()
        && native_each.is_none()
        && native_list_index.is_none()
        && native_list_range.is_none()
        && native_list_insertion.is_none()
        && !*foreach_collect
        && dict_vars.is_none()
        && release_binding.is_none()
        && release_hold.is_none()
        && !*script_command
        && *release_body == 0
        && release_body_entry.is_none()
        && entered_command.is_none()
        && native_compiler_selection.is_none()
        && native_operation_selections.is_empty()
        && catch_target.is_none()
        && catch_start.is_none()
        && catch_end.is_none()
        && release_compiled.is_none()
}

/// A verbatim PUSH must carry a verbatim literal; any other PUSH must hold
/// bytes the executor's substitution path returns unchanged, so no getter or
/// nested command can run.
fn push_is_literal(
    instruction: &Instruction,
    literals: &tcl_bytecode::LiteralTable,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let Some(Operand::Imm(index)) = instruction.operands.first() else {
        return false;
    };
    let Some(literal) = usize::try_from(*index)
        .ok()
        .and_then(|index| literals.entries().get(index))
    else {
        return false;
    };
    if instruction.push_verbatim {
        instruction.literal_rep == LiteralRep::Verbatim
    } else {
        tcl_syntax::word_rules::compiled_word_substitution(literal.bytes(), config)
            == tcl_syntax::word_rules::CompiledWordSubstitution::LiteralValue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function_with(literal: &[u8], verbatim: bool, rep: LiteralRep) -> FunctionAsm {
        let mut function = FunctionAsm {
            plain_command_dispatch: true,
            ..FunctionAsm::default()
        };
        let index = function.literals.intern_bytes(literal);
        let mut push =
            Instruction::new(Op::PUSH1, vec![Operand::Imm(i32::try_from(index).unwrap())]);
        push.push_verbatim = verbatim;
        push.literal_rep = rep;
        function.instructions.push(push);
        function
    }

    #[test]
    fn a_push_is_admitted_only_when_its_bytes_cannot_substitute() {
        let config = tcl_lexer::LexerConfig::default();
        for (literal, verbatim, rep, admitted) in [
            (b"abc".as_slice(), false, LiteralRep::Word, true),
            (b"$name".as_slice(), false, LiteralRep::Word, true),
            (b"[set x]".as_slice(), false, LiteralRep::Word, false),
            (b"a${x}b".as_slice(), false, LiteralRep::Word, false),
            (b"[set x]".as_slice(), true, LiteralRep::Verbatim, true),
            (b"abc".as_slice(), true, LiteralRep::Word, false),
        ] {
            assert_eq!(
                check_function(&function_with(literal, verbatim, rep), true, config).is_ok(),
                admitted,
                "{:?} verbatim={verbatim} {rep:?}",
                String::from_utf8_lossy(literal)
            );
        }
    }

    #[test]
    fn control_and_auxiliary_instructions_are_refused() {
        let config = tcl_lexer::LexerConfig::default();
        for op in [
            Op::BEGIN_CATCH4,
            Op::END_CATCH,
            Op::RETURN_IMM,
            Op::STR_CONCAT1,
        ] {
            let mut function = FunctionAsm {
                plain_command_dispatch: true,
                ..FunctionAsm::default()
            };
            function.instructions.push(Instruction::new(op, vec![]));
            assert!(check_function(&function, true, config).is_err(), "{op:?}");
        }
        let mut function = FunctionAsm {
            plain_command_dispatch: true,
            ..FunctionAsm::default()
        };
        let mut invoke = Instruction::new(Op::INVOKE_STK1, vec![Operand::Imm(1)]);
        invoke.script_command = true;
        function.instructions.push(invoke);
        assert!(check_function(&function, false, config).is_err());
        let plain = FunctionAsm::default();
        assert!(check_function(&plain, false, config).is_err());
    }

    #[test]
    fn only_plain_source_slots_are_admitted_and_read() {
        let config = tcl_lexer::LexerConfig::default();
        let load = |names: &[&str], synthetic: bool, slot: i32, body: bool| {
            let mut function = FunctionAsm {
                plain_command_dispatch: true,
                ..FunctionAsm::default()
            };
            for name in names {
                function.lvt.intern(name);
            }
            if synthetic {
                function.lvt.intern_synthetic("#tmp");
            }
            function
                .instructions
                .push(Instruction::new(Op::LOAD_SCALAR1, vec![Operand::Imm(slot)]));
            check_function(&function, body, config).is_ok()
        };
        assert!(load(&["y"], false, 0, true));
        assert!(
            !load(&["y"], false, 0, false),
            "top-level function has no slots"
        );
        assert!(!load(&["y"], false, 1, true), "unbound slot");
        assert!(!load(&["y"], false, -1, true), "negative slot");
        assert!(!load(&["y"], true, 0, true), "compiler temporary");
        assert!(!load(&["::y"], false, 0, true), "qualified name");
        assert!(!load(&["a(1)"], false, 0, true), "element name");
        assert!(!load(&[""], false, 0, true), "empty name");
        let mut slotted = FunctionAsm {
            plain_command_dispatch: true,
            ..FunctionAsm::default()
        };
        slotted.lvt.intern("y");
        assert!(check_function(&slotted, true, config).is_ok());
        assert!(check_function(&slotted, false, config).is_err());
    }

    #[test]
    fn only_structural_labels_without_jumps_are_admitted() {
        let config = tcl_lexer::LexerConfig::default();
        let labelled = |labels: &[&str], op: Op| {
            let mut function = FunctionAsm {
                plain_command_dispatch: true,
                ..FunctionAsm::default()
            };
            for (index, label) in labels.iter().enumerate() {
                function.labels.insert((*label).to_owned(), index);
            }
            function.instructions.push(Instruction::new(op, vec![]));
            check_function(&function, false, config).is_ok()
        };
        assert!(labelled(&["entry_1", "cmd_end_0", "exit_2"], Op::DONE));
        for labels in [&["loop_1"][..], &["cmd_end_"], &["entry_x"], &["catch_0"]] {
            assert!(!labelled(labels, Op::DONE), "{labels:?}");
        }
        assert!(!labelled(&["cmd_end_0"], Op::JUMP4));
        assert!(!labelled(&["cmd_end_0"], Op::JUMP_TRUE4));
    }

    #[test]
    fn a_filled_plain_module_cache_is_outside_the_owned_scope() {
        let mut vm = crate::test_support::native_vm();
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::default(),
        ));
        assert!(vm.owned_application_cache_scope_is_empty());
        vm.compile_plain_cached_module(tcl_runtime_api::ScriptCompileTarget {
            source: "set y 1",
            namespace: "",
            evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
        })
        .expect("plain module");
        assert!(!vm.owned_application_cache_scope_is_empty());
        let entry = vm.native_compilation_entry_for_namespace_token(crate::interp::ROOT_NS, false);
        assert_eq!(
            vm.prepare_owned_application_source("set y 1", &[], &[], &entry)
                .err(),
            Some(ClosedSourceRefusal::Currency)
        );
    }
}
