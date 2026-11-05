// SPDX-License-Identifier: AGPL-3.0-or-later
//! The selected C substitution compiler shares actual native word emission.

use super::{CodegenCtx, Op, Operand};
use std::{collections::HashMap, rc::Rc};
use tcl_runtime_api::{
    CompileError,
    native_substitution::{NativeSubstitutionCompilationEntry, NativeSubstitutionTarget},
};

pub(crate) fn compile(
    target: NativeSubstitutionTarget<'_>,
    profile: &'static tcl_dialect::DialectProfile,
    entry: NativeSubstitutionCompilationEntry<'_>,
    registry: &tcl_registry::CommandRegistry,
) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
    let snapshot = entry.snapshot();
    if target.source.channel() != tcl_lexer::SourceChannel::NativeValue
        || snapshot.profile != profile.cache_key()
        || snapshot
            .namespaces
            .iter()
            .filter(|row| row.token == snapshot.current_namespace && &row.path == target.namespace)
            .count()
            != 1
    {
        return Err(CompileError::Unsupported(
            "substitution target entry correspondence".into(),
        ));
    }
    let config = crate::compile_service::BytecodeCompileService::native_entry_config(
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        Some(snapshot),
    );
    let policy = tcl_registry::substitution::TemplateParseErrors::for_dialect(
        tcl_registry::InvocationDialect::of_point(
            snapshot
                .execution_point
                .ok_or_else(|| CompileError::Unsupported("substitution engine point".into()))?,
        ),
    )
    .ok_or_else(|| CompileError::Unsupported("substitution variable grammar".into()))?;
    let end = u32::try_from(target.source.len())
        .map_err(|_| CompileError::Unsupported("substitution source extent".into()))?;
    let arena = Rc::new(
        tcl_lexer::ExecutablePartArena::decompose_template(
            target.source.clone(),
            tcl_lexer::Span::new(0, end),
            entry.flags().lexer_flags(),
            config,
            policy.variable_syntax(),
        )
        .map_err(|error| {
            CompileError::Unsupported(format!("substitution template ownership: {error:?}"))
        })?,
    );
    let mut ctx = CodegenCtx::new(false, &[], registry);
    ctx.native_entry = Some(snapshot);
    ctx.ingress_lexer_config = Some(config);
    ctx.source_string_protocol = snapshot.source_string_protocol;
    ctx.compiled_variable_protocol = snapshot.compiled_variable_protocol;
    ctx.invocation_dialect = crate::compile_service::BytecodeCompileService::native_entry_options(
        snapshot,
        Some(profile),
    )
    .logical_invocation_dialect();
    ctx.dialect = Some(profile);
    ctx.expr_grammar = Some(config.grammar_over(profile.grammar));
    ctx.escapes = config.escapes;
    ctx.braced_var = config.braced_var;
    ctx.set_source_image(target.source.clone());
    ctx.set_resolution_namespace_path(target.namespace.clone());
    ctx.lvt
        .set_native_protocol(snapshot.compiled_variable_protocol);
    if let Some(layout) = &snapshot.compiled_local_layout {
        ctx.lvt = tcl_bytecode::LocalVarTable::from_native_slot_names(&layout.names);
        ctx.lvt
            .set_native_protocol(snapshot.compiled_variable_protocol);
        ctx.borrowed_local_layout = Some(layout.clone());
    }
    ctx.emit_substitution(&arena);
    ctx.emit(Op::DONE, vec![]);
    super::layout::optimise_jumps(&mut ctx.instructions, &ctx.label_positions, 5);
    let labels = super::layout::resolve_layout(&mut ctx.instructions, &ctx.label_positions);
    let mut function = ctx.into_function_asm("<subst>".into());
    function.labels = labels;
    function
        .validate_native_compilation_entry()
        .map_err(CompileError::NativeCompilationAdmission)?;
    let mut asm = tcl_bytecode::ModuleAsm {
        profile,
        source: target.source.clone(),
        source_namespace: target.namespace.clone(),
        plain_command_dispatch: false,
        top_level_body: tcl_bytecode::FunctionAsm::default(),
        top_level: function,
        procedures: HashMap::new(),
        procedure_provenance: HashMap::new(),
        manifest: None,
    };
    asm.manifest = Some(std::sync::Arc::new(super::emitter::module_manifest(&asm)));
    Ok(asm)
}

impl CodegenCtx<'_> {
    fn flush_substitution_fragments(&mut self, count: &mut usize) {
        while *count > 255 {
            self.emit(Op::STR_CONCAT1, vec![Operand::Imm(255)]);
            *count -= 254;
        }
        if *count > 1 {
            self.emit(
                Op::STR_CONCAT1,
                vec![Operand::Imm(
                    i32::try_from(*count).expect("bounded concatenation"),
                )],
            );
            *count = 1;
        }
    }

    fn emit_substitution(&mut self, arena: &Rc<tcl_lexer::ExecutablePartArena>) {
        use tcl_lexer::ExecutablePart;
        let parts = arena.list(arena.root());
        let mut count = 0;
        if parts
            .first()
            .is_none_or(|part| !matches!(part.part, ExecutablePart::Text(_)))
        {
            self.push_lit_bytes_exact(b"");
            count = 1;
        }
        let done = self.fresh_label("subst_done");
        for (position, part) in parts.iter().enumerate() {
            let exceptional = match &part.part {
                ExecutablePart::Command { .. } => true,
                ExecutablePart::Variable {
                    index: Some(index), ..
                } => arena
                    .list(*index)
                    .iter()
                    .any(|child| matches!(child.part, ExecutablePart::Command { .. })),
                _ => false,
            };
            if !exceptional {
                self.emit_substitution_component(arena, position);
                count += 1;
                continue;
            }
            self.flush_substitution_fragments(&mut count);
            let (begin, handler) = self.emit_catch_region_prologue(None, None);
            let saved_context = self.native_compilation;
            self.native_compilation = saved_context.with_inline_exception_range();
            self.emit_substitution_component(arena, position);
            self.native_compilation = saved_context;
            self.instructions[begin].catch_target = Some(handler.clone());
            self.catch_depth -= 1;
            self.emit(Op::END_CATCH, vec![]);
            let ok = self.fresh_label("subst_ok");
            let finish = self.fresh_label("subst_next");
            let returned = self.fresh_label("subst_return");
            let broken = self.fresh_label("subst_break");
            let continued = self.fresh_label("subst_continue");
            self.emit(Op::JUMP4, vec![Operand::Label(ok.clone())]);
            self.place_label(&handler);
            self.emit(Op::PUSH_RETURN_OPTS, vec![]);
            self.emit(Op::PUSH_RESULT, vec![]);
            self.emit(Op::PUSH_RETURN_CODE, vec![]);
            self.emit(Op::END_CATCH, vec![]);
            self.emit(Op::RETURN_CODE_BRANCH, vec![]);
            // Native branch slots have fixed two-byte width: error, return,
            // break, continue, other. Error's returnStk+nop is also two bytes.
            self.emit(Op::RETURN_STK, vec![]);
            self.emit(Op::NOP, vec![]);
            for target in [&returned, &broken, &continued, &returned] {
                self.emit(Op::JUMP1, vec![Operand::Label(target.clone())]);
            }
            self.place_label(&broken);
            self.emit(Op::POP, vec![]);
            self.emit(Op::POP, vec![]);
            self.emit(Op::JUMP4, vec![Operand::Label(done.clone())]);
            self.place_label(&continued);
            self.emit(Op::POP, vec![]);
            self.emit(Op::POP, vec![]);
            self.emit(Op::JUMP4, vec![Operand::Label(finish.clone())]);
            self.place_label(&returned);
            self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
            self.emit(Op::POP, vec![]);
            self.place_label(&ok);
            self.emit(Op::STR_CONCAT1, vec![Operand::Imm(2)]);
            self.place_label(&finish);
            count = 1;
        }
        self.flush_substitution_fragments(&mut count);
        self.place_label(&done);
    }
}
