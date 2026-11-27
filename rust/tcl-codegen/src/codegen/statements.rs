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

//! Statement-level bytecode emission.
//!
//! Extends [`CodegenCtx`] with methods for emitting IR statements
//! (assignments, calls, returns, barriers) with `startCommand`
//! wrapping.

use super::{CodegenCtx, Op, Operand};
use tcl_compiler::ir::Statement;
use tcl_compiler::registry_invocation;
use tcl_registry::hooks::InlineCodegenHookId;
#[path = "statements/native_array.rs"]
mod native_array;

mod assignments;
mod calls;
mod native_emission;
mod native_schedule;
mod registered_tasks;
mod value_tasks;
mod value_words;
mod variable_tasks;

fn is_op(code: Op) -> NativeEmissionTask {
    NativeEmissionTask::Operation(code, vec![])
}

fn is_jump(code: Op, label: &str) -> NativeEmissionTask {
    NativeEmissionTask::Operation(code, vec![Operand::Label(label.to_owned())])
}

#[cfg(test)]
#[path = "statements/native_clock_tests.rs"]
mod native_clock_tests;
#[path = "native_control.rs"]
mod native_control;
#[path = "native_coroutine.rs"]
mod native_coroutine;
#[path = "statements/native_dictionary.rs"]
mod native_dictionary;
#[path = "statements/native_dictionary_scope.rs"]
mod native_dictionary_scope;
#[path = "native_each.rs"]
mod native_each;
#[path = "native_error.rs"]
mod native_error;
#[path = "native_info_exists.rs"]
mod native_info_exists;
#[path = "statements/native_introspection.rs"]
mod native_introspection;
#[path = "native_list_operations.rs"]
mod native_list_operations;
#[path = "statements/native_mathop.rs"]
mod native_mathop;
#[path = "statements/native_namespace_string.rs"]
mod native_namespace_string;
#[path = "native_namespace_upvar.rs"]
mod native_namespace_upvar;
#[path = "statements/native_scalar.rs"]
mod native_scalar;
#[path = "statements/native_string.rs"]
mod native_string;
#[path = "native_switch.rs"]
mod native_switch;
#[path = "native_try.rs"]
mod native_try;
#[path = "native_unset.rs"]
mod native_unset;
#[path = "native_upvar.rs"]
mod native_upvar;

/// Work retained on the heap so array indices and bracket scripts share one
/// compilation unit without recursive emitter calls or depth-limited views.
enum NativeEmissionTask {
    /// `TclSubstCompile` over an original template.
    Substitution(std::rc::Rc<tcl_lexer::ExecutablePartArena>),
    NativePreparationLiteral(Vec<u8>),
    NativePrivateFormatLiteral(Vec<u8>),
    NativeDictionaryUpdateOperation(Op, Vec<u8>, Vec<Vec<u8>>),
    NativeArrayEachStart(
        tcl_dialect::TclVersion,
        [std::rc::Rc<std::cell::Cell<Option<usize>>>; 2],
    ),
    NativeEachStart(
        tcl_dialect::TclVersion,
        Vec<Vec<Vec<u8>>>,
        Vec<std::rc::Rc<std::cell::Cell<Option<usize>>>>,
        bool,
    ),
    ExpressionNode(
        tcl_registry::native_expression_program::NativeExpressionProgram,
        tcl_syntax::expr::NativeExprNode,
    ),
    ExpressionSyntax(Vec<u8>, Option<Vec<u8>>),
    PrivateExpressionNumber(
        tcl_dialect::TclVersion,
        tcl_bytecode::NativeExpressionNumberLiteral,
    ),
    PrivateLogicalBoolean85(bool),
    NativeCommandLiteral(
        Box<tcl_native_model::native_command_literal::NativeCompiledCommandLiteral>,
    ),
    RegisteredExpressionNumber(
        tcl_dialect::TclVersion,
        Vec<u8>,
        tcl_bytecode::NativeExpressionNumberLiteral,
    ),
    RegisteredExpressionBoolean84(Vec<u8>),
    FixedMathCall(
        tcl_runtime_api::native_compilation::NativeMathFunctionBinding,
        u8,
    ),
    ControlScript(
        tcl_lexer::SourceImage,
        tcl_registry::native_control_instructions::NativeControlBody,
        tcl_registry::native_compilation::NativeCompiledBodyContext,
        tcl_lexer::LexerConfig,
    ),
    ControlContext(tcl_registry::native_compilation::NativeCompilationContext),
    /// The break and continue labels of the loop whose body script follows.
    NativeLoopTargets(String, Option<String>),
    /// The innermost body's loop targets are no longer in reach.
    LeaveNativeLoop,
    /// `break` (true) or `continue`, resolved against the loop in reach.
    NativeLoopExit(bool, tcl_dialect::TclVersion),
    BeginNativeCatch(String),
    BeginNativeCatchAt(String, String, String),
    BeginSpeculativeNativeCompilation(std::rc::Rc<std::cell::Cell<bool>>),
    FinishSpeculativeNativeCatch(
        NativeNamespaceRollback,
        tcl_lexer::NativeScriptCommandWords,
        std::rc::Rc<std::cell::Cell<bool>>,
        tcl_registry::native_compilation::NativeCompilationContext,
        u32,
    ),
    EndNativeCatch,
    BeginNativeCatchBranch(String),
    EndNativeCatchBranch,
    DeclareNativeTemporary(std::rc::Rc<std::cell::Cell<Option<usize>>>),
    NativeTemporaryOperation(
        Op,
        std::rc::Rc<std::cell::Cell<Option<usize>>>,
        Vec<Operand>,
    ),
    PrivateInteger(i64),
    PrivateList(
        Vec<Vec<u8>>,
        tcl_syntax::native_string::NativeStringProtocol,
    ),
    Word(tcl_lexer::NativeWord),
    List(
        std::rc::Rc<tcl_lexer::ExecutablePartArena>,
        tcl_lexer::PartListId,
        usize,
    ),
    Part(
        std::rc::Rc<tcl_lexer::ExecutablePartArena>,
        tcl_lexer::PartListId,
        usize,
    ),
    Script(
        tcl_lexer::SourceImage,
        tcl_lexer::Span,
        tcl_lexer::LexerConfig,
    ),
    Command(tcl_lexer::NativeScriptCommandWords),
    /// Name the commands a bracket script emitted from this instruction on in
    /// `errorInfo`, once they are all emitted and the enclosing statement's
    /// source is restored.
    StampSubstitution(usize, String),
    RestoreSource(NativeEmissionSource),
    Operation(Op, Vec<Operand>),
    NativeListIndex(tcl_syntax::native_compiled_index::NativeCompiledListIndex),
    NativeListRange(tcl_syntax::native_compiled_index::NativeCompiledListRange),
    NativeListInsertion(
        tcl_registry::native_list_operations_compilation::NativeListInsertionInstruction,
        usize,
    ),
    ErrorReturn,
    NativeStringIndex(
        tcl_dialect::TclVersion,
        tcl_bytecode::NativeStringIndexPurpose,
    ),
    SwitchOperation(Op, Vec<Operand>, tcl_dialect::TclVersion),
    SwitchTable(
        tcl_dialect::TclVersion,
        bool,
        std::collections::HashMap<Vec<u8>, String>,
        std::collections::HashMap<i64, String>,
    ),
    DeclareNamespaceLocal(Vec<u8>),
    NamespaceLocalOperation(Op, Vec<u8>),
    NamespaceGenericRollback(NativeNamespaceRollback, tcl_lexer::NativeScriptCommandWords),
    Literal(Vec<u8>),
    PoolLiteral(usize),
    PrivateReturnOptions(tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral),
    Label(String),
    Syntax(&'static str),
}

struct PreparedNativeRegistered {
    version: tcl_dialect::TclVersion,
    instruction: tcl_registry::native_instruction_plan::NativeInstructionPlan,
    boundary: NativeRegisteredBoundary,
}

struct NativeRegisteredBoundary {
    namespace: tcl_core_types::ByteNamespacePath,
    prerequisite: tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
}

struct NativeNamespaceRollback {
    instructions: usize,
    labels: std::collections::HashMap<String, usize>,
    loop_regions: usize,
    command_index: u32,
}

struct NativeEmissionSource {
    image: tcl_lexer::SourceImage,
    lines: Option<tcl_lexer::LineIndex>,
    span: Option<tcl_lexer::Span>,
    command: bool,
    exact_command: Option<tcl_lexer::SourceImage>,
    namespace: tcl_runtime_api::ByteNamespacePath,
    namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
}

/// Tag used to identify `startCommand` instructions wrapping generic
/// invokes so the peephole pass can selectively remove them.
pub(crate) const SC_GENERIC_TAG: &str = "sc:generic";

/// Whether `s` contains a **backslash-escaped** substitution marker (`\$` or
/// `\[`).
///
/// The sibling of [`has_unescaped_subst`], and the question that decides
/// whether a word may be deferred to the VM's `subst_word`. It may not: the
/// VM reads a compiled literal under the convention that codegen already
/// decoded the word's escapes, so a surviving backslash is an ordinary
/// character and the `$` after it starts a live `${…}`. Handing over an
/// *undecoded* word therefore breaks that contract — `puts A:[string length
/// "\$\{x}"]` read the variable `x`, and with no closing brace it raised
/// `missing close-brace for variable name`, on scripts both oracles run.
pub(crate) fn has_escaped_subst_marker(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'\\' {
            if matches!(b[i + 1], b'$' | b'[') {
                return true;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    false
}

/// Tag appended to no-dedup literal comments.
pub(crate) const NO_DEDUP_TAG: &str = " #nodedup";

impl CodegenCtx<'_> {
    /// Emit a statement, wrapping with `startCommand` if needed.
    ///
    /// `count_override` overrides the default count of 1 (e.g. 2 when
    /// a compound command and its init share the same bytecode offset).
    /// `deferred_end_label` uses a caller-provided label instead of
    /// creating and placing one automatically.
    pub fn emit_stmt_with_start_cmd(
        &mut self,
        stmt: &Statement,
        count_override: Option<u32>,
        deferred_end_label: Option<&str>,
    ) {
        if !stmt.is_executable_invocation() {
            return;
        }
        // An empty `for` clause is three `nop`s inside the `for` command, not
        // a command of its own: Tcl gives it no startCommand. As the init
        // clause it still opens the `for`'s startCommand, which then starts
        // one command rather than two.
        let empty_clause =
            stmt.synthetic_marker() == Some(tcl_compiler::ir::SyntheticMarker::EmptyClause);
        // The wrapping startCommand shares the statement's source span.
        self.set_command_source_span(stmt.span());
        let count = if empty_clause {
            1
        } else {
            count_override.unwrap_or(1)
        };
        let emit_sc = self.cmd_index > 0 && (!empty_clause || deferred_end_label.is_some());
        let mut end_label: Option<String> = None;
        let mut sc_idx: Option<usize> = None;

        if emit_sc {
            let label = match deferred_end_label {
                Some(l) => l.to_owned(),
                None => self.fresh_label("cmd_end"),
            };
            sc_idx = Some(self.emit_comment(
                Op::START_CMD,
                vec![
                    Operand::Label(label.clone()),
                    Operand::Imm(i32::try_from(count).expect("startCommand count fits in i32")),
                ],
                "",
            ));
            end_label = Some(label);
        }

        let mut used_generic_invoke = false;
        // The statement a substitution inside it logs last when the release
        // evaluates it directly, wherever the emission's own source moves.
        let statement = self.direct_statement_error_info();
        let enclosing_direct = std::mem::replace(&mut self.direct_statement, statement);
        self.emit_stmt(stmt, &mut used_generic_invoke);
        self.direct_statement = enclosing_direct;

        // A typed emitter may have consumed a binding from an inlined source
        // site whose namespace differs from this function's. The wrapper was
        // emitted before that binding was known, so finish the boundary from
        // the same central namespace state the specialised instructions used.
        if let Some(idx) = sc_idx {
            self.instructions[idx]
                .source_command_namespace
                .clone_from(&self.current_command_namespace);
            self.instructions[idx]
                .source_command_namespace_context
                .clone_from(&self.current_command_namespace_context);
        }

        if used_generic_invoke {
            self.seen_generic_invoke = true;
            // Tag this startCommand as wrapping a generic invoke
            if let Some(idx) = sc_idx
                && (!self.is_proc || self.native_entry.is_some())
            {
                SC_GENERIC_TAG.clone_into(&mut self.instructions[idx].comment);
            }
        }

        if let Some(ref label) = end_label
            && deferred_end_label.is_none()
        {
            // Place end label before the trailing pop (or at end)
            let pos = if self.instructions.last().is_some_and(|i| i.op == Op::POP) {
                self.instructions.len() - 1
            } else {
                self.instructions.len()
            };
            self.label_positions.insert(label.clone(), pos);
        }

        self.cmd_index += 1;
    }

    /// Emit one source command whose bytecode position is already covered by
    /// an enclosing structured command's `START_CMD`.
    ///
    /// A constant-true `if` and the first command in its selected body begin at
    /// the same bytecode offset in Tcl 9.0. The enclosing marker therefore
    /// carries count two and the nested command must not grow a second marker
    /// at that position. Keep this beside the ordinary statement wrapper so
    /// command indexing and generic-invoke bookkeeping cannot diverge between
    /// the two paths.
    pub fn emit_stmt_under_start_cmd(&mut self, stmt: &Statement) {
        if !stmt.is_executable_invocation() {
            return;
        }
        self.set_command_source_span(stmt.span());
        let mut used_generic_invoke = false;
        self.emit_stmt(stmt, &mut used_generic_invoke);
        if used_generic_invoke {
            self.seen_generic_invoke = true;
        }
        self.cmd_index += 1;
    }

    /// Emit `Statement::Call` — handles the CFG placeholder names
    /// (`<cond>` / `<empty_clause>`), `{*}` expansion, and the
    /// generic call path.
    fn emit_call_stmt(
        &mut self,
        command: &str,
        args: &[String],
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
        used_generic_invoke: &mut bool,
    ) {
        // A statement the CFG builder synthesised carries an effect or a
        // placeholder, never a command to run. The identity is the *typed*
        // `CommandTokens::synthetic` discriminant, never the `command`
        // spelling: `<cond>`, `<empty_clause>` and the rest are all legal Tcl
        // command names a script may define and call (tclsh 8.4.20 / 8.5.19 /
        // 8.6.16 / 9.0.4 / 9.1 all run `proc <cond> {} { puts hit-cond };
        // <cond>`), so matching on the name silently dropped such a call.
        //
        // Dispatching a *marker*, conversely, either duplicates the callee's
        // side effects — a caller-frame barrier reusing the callee's own name
        // makes `proc p {} { upvar 1 {a b} v ; puts "u=$v" }; p` print twice —
        // or reaches the VM as an invalid command
        // name (`invalid command name "<global-frame-script>"`).
        if let Some(marker) = tokens.and_then(|t| t.synthetic) {
            if marker == tcl_compiler::ir::SyntheticMarker::EmptyClause {
                // tclsh's bytecode keeps three `nop`s where a `for` clause is
                // empty; match it so the instruction stream stays byte-true.
                // They are compiler control, never a replayable command.
                self.set_control_source_span(self.current_span);
                self.literals.intern("");
                self.emit(Op::NOP, vec![]);
                self.emit(Op::NOP, vec![]);
                self.emit(Op::NOP, vec![]);
            }
            return;
        }
        let has_expand = tokens
            .and_then(|t| t.expand_word.as_ref())
            .is_some_and(|ew| ew.iter().any(|&e| e));
        if has_expand {
            // The selected original compiler sees the parser's literal
            // expansion as ordinary words, so it owns the command first.
            if self.emit_native_original_preparation_invocation(tokens) {
                self.emit(Op::POP, vec![]);
                *used_generic_invoke = true;
                return;
            }
            let ew = tokens.and_then(|t| t.expand_word.as_ref()).unwrap();
            if let Some(words) = super::cmd_subst::parse_time_expansion(
                std::iter::once(command)
                    .chain(args.iter().map(String::as_str))
                    .enumerate()
                    .map(|(index, word)| {
                        let braced = tokens.is_some_and(|t| {
                            t.argv_kinds.get(index) == Some(&tcl_lexer::TokenType::Str)
                                && t.single_token_word.get(index).copied().unwrap_or(false)
                        });
                        (word, braced, ew.get(index).copied().unwrap_or(false))
                    }),
            ) && let Some(((command, _), args)) = words.split_first()
            {
                let args: Vec<String> = args.iter().map(|(word, _)| word.clone()).collect();
                self.emit_call(command, &args, None, used_generic_invoke);
                return;
            }
            self.emit_expanded_call(command, args, ew, tokens);
            *used_generic_invoke = true;
        } else {
            if self.emit_selected_try_statement(command, args, tokens) {
                *used_generic_invoke = true;
                return;
            }
            self.emit_call(command, args, tokens, used_generic_invoke);
        }
    }

    /// Dispatch a statement to the appropriate emission handler.
    pub fn emit_stmt(&mut self, stmt: &Statement, used_generic_invoke: &mut bool) {
        let source = self.pending_math_source.take();
        let previous_source = std::mem::replace(&mut self.math_source, source);
        let base = match stmt {
            Statement::AssignExpr { expr_base, .. }
            | Statement::ExprEval { expr_base, .. }
            | Statement::Return { expr_base, .. } => *expr_base,
            _ => None,
        };
        let previous_base = std::mem::replace(&mut self.math_expression_base, base);
        let source_proofs = self.source_proofs.clone();
        let tokens = stmt
            .tokens()
            .or_else(|| source_proofs.as_ref()?.tokens.get(&stmt.span().start()));
        self.with_invocation_tokens(tokens, |ctx| {
            ctx.emit_stmt_inner(stmt, used_generic_invoke);
        });
        self.math_source = previous_source;
        self.math_expression_base = previous_base;
    }

    fn emit_opaque_switch(
        &mut self,
        raw_args: &[String],
        raw_arg_braced: &[bool],
        used_generic_invoke: &mut bool,
    ) {
        // Each word goes out the way it was written. A braced word is
        // data — its `[…]` / `${…}` must not run — and that is true of
        // every position, not just the trailing arm list: otherwise
        // `switch -glob -- {a[bc]d} {a\[bc\]d} …` runs `bc` for the
        // subject, and decodes the pattern to `a[bc]d`, which then
        // matches as a character class rather than as the literal both
        // oracles match.
        //
        // `raw_arg_braced` is the lexer's answer, carried through the
        // IR because `raw_args` are values and a value cannot say how
        // it was written. Empty means unknown (a hand-built
        // statement), which keeps the old all-interpolated behaviour.
        if raw_arg_braced.iter().any(|braced| *braced) {
            let n = raw_args.len();
            let mut kinds = vec![tcl_lexer::TokenType::Esc; n + 1];
            for (i, braced) in raw_arg_braced.iter().enumerate() {
                if *braced && i + 1 < kinds.len() {
                    kinds[i + 1] = tcl_lexer::TokenType::Str;
                }
            }
            let toks = tcl_compiler::ir::CommandTokens::from_lossy_parts(
                Vec::new(),
                Vec::new(),
                kinds,
                vec![true; n + 1],
                Vec::new(),
                None,
            );
            self.emit_call_stmt("switch", raw_args, Some(&toks), used_generic_invoke);
        } else {
            self.emit_call_stmt("switch", raw_args, None, used_generic_invoke);
        }
    }

    fn emit_stmt_inner(&mut self, stmt: &Statement, used_generic_invoke: &mut bool) {
        // Stamp every instruction this statement lowers to with its source
        // span so the explorer can map each op back to source.
        self.set_command_source_span(stmt.span());
        if !stmt.is_executable_invocation() {
            return;
        }
        if !matches!(
            stmt,
            Statement::Call { .. } | Statement::Barrier { .. } | Statement::NativeCall { .. }
        ) && self.emit_retained_native_dispatch(used_generic_invoke)
        {
            return;
        }
        if self.emit_assign_or_incr(stmt) {
            return;
        }
        match stmt {
            Statement::NativeCall { words, .. } => {
                self.emit_native_words(words);
                self.emit(Op::POP, vec![]);
                *used_generic_invoke = true;
            }
            Statement::Call {
                command,
                args,
                tokens,
                ..
            } => self.emit_call_stmt(command, args, tokens.as_ref(), used_generic_invoke),

            Statement::Barrier {
                command,
                args,
                reason,
                tokens,
                ..
            } => {
                if command.is_empty() {
                    // Retained source with missing context/admission is not an
                    // executable empty body. A real native source provider must
                    // discharge the obligation before this unit can enter.
                    if reason == "original body namespace context is unavailable"
                        || reason == "original body source admission is unavailable"
                    {
                        self.refuse_native_dependency();
                    }
                    self.emit_comment(Op::NOP, vec![], &format!("barrier: {reason}"));
                } else {
                    self.emit_call_stmt(command, args, tokens.as_ref(), used_generic_invoke);
                }
            }

            // Opaque (glob/regexp/fall-through) switch: the CFG builder keeps
            // it as a single statement rather than expanding arm blocks, so
            // emit a generic invoke of the command it spells — tclsh 9.0's
            // un-compiled approach for these modes. A `case` list lowers
            // through the same statement and invokes `case`.
            Statement::Switch {
                raw_args,
                raw_arg_braced,
                ..
            } => {
                self.emit_opaque_switch(raw_args, raw_arg_braced, used_generic_invoke);
            }

            Statement::Return { value, .. } => {
                let val = value.as_deref().unwrap_or("");
                self.emit_value_interpolated(val);
                // A top-level `RETURN_IMM` pops the result *and* an options dict;
                // push the empty options so the pair sits at the top of the
                // stack regardless of any leftover from a preceding statement
                // (e.g. an `if` with no `else`). Proc bodies fold this to a
                // `DONE`, which returns the single top value, so no options push.
                if !self.is_proc {
                    self.push_lit("");
                }
                // A plain `return` is `(code 0, level 1)`: C's
                // `TclMergeReturnOptions` defaults `-level` to 1, and
                // `CompileReturnInternal` puts the merged pair on the operands
                // (`tclCompCmdsGR.c:2410`). `(0, 0)` would mean "push the result
                // and fall through", not "return". Proc bodies re-key on the same
                // pair in `fold_tail_return_to_done`.
                self.emit(Op::RETURN_IMM, vec![Operand::Imm(0), Operand::Imm(1)]);
            }

            // Static-body uplevel: `UpFrame` is the structured IR form the
            // analysers consume (interproc purity, code-sinking, var-escape, the
            // inline_uplevel pass). The whole-callee inline_uplevel splice
            // rewrites passthrough call-sites into `Statement::Block` (emitted
            // inline below); any `UpFrame` that reaches codegen unspliced is run
            // through the runtime `uplevel` builtin by re-emitting the original
            // `uplevel <level> {body}` invoke from its preserved command tokens.
            // This is byte-identical to the pre-static-lowering barrier path and
            // keeps correct frame semantics without needing frame-shift opcodes.
            Statement::UpFrame { tokens, .. } => {
                if let Some(ct) = tokens
                    && ct.argv_texts.len() >= 2
                {
                    let command = ct.argv_texts[0].clone();
                    let args: Vec<String> = ct.argv_texts[1..].to_vec();
                    self.emit_call_stmt(&command, &args, Some(ct), used_generic_invoke);
                } else {
                    // No preserved tokens (synthetic UpFrame) — nothing to
                    // dispatch; mark it rather than silently dropping a body.
                    self.emit_comment(Op::NOP, vec![], "unhandled: Statement::UpFrame (no tokens)");
                }
            }

            // Inline block: the inline_uplevel pass replaces
            // matching call-sites with ``Statement::Block { body, .. }``
            // — emit the body's statements as if they were inline at
            // this point. The block has no scope of its own at the
            // bytecode level (the body already ran in the caller's
            // frame in the original ``uplevel`` semantics).
            Statement::Block { body, .. } => {
                if registry_invocation::block_requires_runtime_script(stmt) {
                    if let Some(Statement::Barrier {
                        command,
                        args,
                        tokens,
                        ..
                    }) = registry_invocation::original_block_runtime_invocation(stmt)
                    {
                        self.emit_call_stmt(&command, &args, tokens.as_ref(), used_generic_invoke);
                    } else {
                        self.refuse_native_dependency();
                    }
                    return;
                }
                let begin = self.instructions.len();
                for inner in &body.statements {
                    self.emit_stmt(inner, used_generic_invoke);
                }
                if let Some(first) = (begin < self.instructions.len()).then_some(begin) {
                    self.mark_completion_option_scope(
                        first,
                        tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_FORWARDED,
                    );
                }
            }

            // Structured control flow is handled by the main emitter loop;
            // if we reach these here, emit a NOP placeholder.
            _ => {
                self.emit_comment(Op::NOP, vec![], "unhandled statement");
            }
        }
    }

    fn emit_retained_native_dispatch(&mut self, used_generic_invoke: &mut bool) -> bool {
        if self.invocation_specialisation_proved() {
            return false;
        }
        let Some(tokens) = self
            .invocation_tokens
            .as_deref()
            .filter(|tokens| tokens.synthetic.is_none() && tokens.source_binding.is_some())
            .cloned()
        else {
            return false;
        };
        let Some((head, arguments)) = tokens.argv_texts.split_first() else {
            self.refuse_native_dependency();
            return false;
        };
        self.emit_call_stmt(head, arguments, Some(&tokens), used_generic_invoke);
        true
    }

    /// Select a native loop opcode through the shared hook and retain its
    /// command requirement. Runtime replay must enter ordinary dispatch when
    /// a replacement command returns normally at this command boundary.
    fn emit_lexical_loop_jump(&mut self, command: &str, args: &[String]) -> bool {
        if !args.is_empty() {
            return false;
        }
        let hook = self.inline_cmd_subst_hook_candidate(command, &[]);
        let target = match hook {
            Some(InlineCodegenHookId::Break) => self.break_target.clone(),
            Some(InlineCodegenHookId::Continue) => self.continue_target.clone(),
            _ => None,
        };
        let Some(target) = target else { return false };
        if self.inline_codegen_hook(command, &[]) != hook {
            return false;
        }
        self.emit_comment(Op::JUMP4, vec![Operand::Label(target)], command);
        true
    }
}

#[cfg(test)]
#[path = "native_concat_tests.rs"]
mod native_concat_tests;
#[cfg(test)]
#[path = "native_list_index_tests.rs"]
mod native_list_index_tests;

#[cfg(test)]
#[path = "native_variable_preparation_tests.rs"]
mod native_variable_preparation_tests;

#[cfg(test)]
#[path = "native_authored_bracket_tests.rs"]
mod native_authored_bracket_tests;

/// Whether `entry` runs an authored naming simulation rather than its native
/// point's own policy.
fn authored_name_policy(entry: &tcl_runtime_api::NativeCompilationEntry) -> bool {
    entry.command_name_policy().is_some_and(|policy| {
        policy.authority() == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "native_format_tests.rs"]
mod native_format_tests;
