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

//! Bytecode emission: the [`CodegenCtx`] context, the per-statement /
//! expression emitter submodules, and the agnostic [`Backend`] trait.
//!
//! The bytecode *artifact* types — [`Op`], [`Instruction`], [`FunctionAsm`],
//! [`ModuleAsm`], the interning tables, plus instruction [`layout`] and
//! disassembly [`format`] — live in the leaf `tcl-bytecode` crate and are
//! re-exported here so existing `codegen::*` paths keep resolving and the
//! bytecode VM can depend on them without pulling in the compiler.
//!
//! Submodules:
//! - [`helpers`] — pure utility functions for compile-time folding
//! - [`values`] — variable load/store and value emission
//! - [`expressions`] — expression AST compilation
//! - [`backend`] — the agnostic [`Backend`] trait + [`BytecodeBackend`]

pub mod backend;
pub mod cmd_subst;
pub mod control_flow;
pub mod emit;
pub mod emitter;
pub mod expressions;
pub mod helpers;
pub mod peephole;
pub mod statements;
pub mod structured;
pub mod values;
pub mod wasm;

pub use backend::{Backend, BytecodeBackend};
pub use emitter::{codegen_function, codegen_module};
// Bytecode artifact types moved to the `tcl-bytecode` crate; re-export them (and
// the `layout`/`format` modules) so `crate::codegen::{Op, FunctionAsm, …}`,
// `codegen::layout::*`, and `codegen::format::*` keep resolving for the emitter
// submodules, tests, and external consumers.
pub use tcl_bytecode::*;
pub use tcl_bytecode::{format, layout};

use std::collections::HashMap;

use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

// -- Emission context --

/// Mutable context for bytecode emission.
///
/// Replaces `_Emitter` class-level state (`self.asm`,
/// `self.current_block`, `self.local_vars`).  Each [`CodegenCtx`]
/// produces one [`FunctionAsm`] — create a separate context for each
/// procedure or top-level script.
#[derive(Debug)]
// `is_proc` is a constructor-time configuration flag; the others
// (`seen_generic_invoke`, `used_generic_invoke`,
// `used_inline_cmd_subst`) are emission-time tracking flags
// written and read at hot-path code-emission sites. They're
// genuinely orthogonal — folding into a bitflags type would just
// rename `ctx.is_proc` to `ctx.flags.contains(...)` without any
// readability or perf gain — and the emitter is a churn-sensitive
// area. Leaving the allow.
#[allow(clippy::struct_excessive_bools)]
pub struct CodegenCtx<'r> {
    /// The numeric-literal grammar of the release being compiled *for*.
    ///
    /// The dialect is a top-level property of the compile, threaded from the
    /// entry point (`IrModule::dialect`) to here, so a numeric literal is
    /// resolved for the target release while emitting rather than re-read under
    /// whatever rules happen to be installed at run time. Defaults to 9.0 for
    /// the hand-built contexts in tests.
    pub numbers: tcl_dialect::NumberSyntax,
    /// Literal constant pool.
    pub literals: LiteralTable,
    /// Local variable table.
    pub lvt: LocalVarTable,
    /// Instruction stream (append-only during emission).
    pub instructions: Vec<Instruction>,
    /// Label name → instruction index (populated by [`place_label`]).
    pub(crate) label_positions: HashMap<String, usize>,
    /// Monotonic counter for generating unique label names.
    label_counter: u32,
    /// Whether we are compiling a proc body (affects LVT vs stack ops).
    pub is_proc: bool,
    /// Command index for `startCommand` numbering.
    pub cmd_index: u32,
    /// End label for the current `startCommand` (paired by `end_command`).
    pub start_cmd_end_label: Option<String>,
    /// Loop break target label (set by the emitter loop).
    pub break_target: Option<String>,
    /// Loop continue target label (set by the emitter loop).
    pub continue_target: Option<String>,
    /// Catch nesting depth for `beginCatch4` operand.
    pub catch_depth: u32,
    /// Whether a generic invoke (`invokeStk1`) has been seen.
    pub seen_generic_invoke: bool,
    /// Whether a generic invoke was actually used (for peephole).
    pub used_generic_invoke: bool,
    /// Whether an inline command substitution was used.
    pub used_inline_cmd_subst: bool,
    /// Depth counter for nested math-function calls in expressions.
    pub expr_func_depth: u32,
    /// Deferred `startCommand` end label for `<cond>` synthetic statements.
    pub pending_cond_end_label: Option<String>,
    /// Label targeting the trailing proc `done` (dead-code jumps after return).
    pub proc_exit_label: Option<String>,
    /// Pending `startCommand` end labels for constant-folded branches.
    pub pending_join_labels: HashMap<String, String>,
    /// 1-based source line of the current statement (for `errorInfo`).
    pub current_source_line: u32,
    /// Byte span of the source construct currently being lowered, stamped
    /// onto every instruction [`Self::emit`] / [`Self::emit_comment`]
    /// appends. Set at the top of each statement / terminator emission and
    /// reset to `None` for synthetic per-block instructions, so each op's
    /// `source_span` reflects the construct it actually came from.
    pub current_span: Option<Span>,
    /// Command registry consulted by registry-driven codegen hooks.
    ///
    /// Threaded in by the caller so dialect-loaded specs (iRules,
    /// Tk, EDA) drive codegen-hook resolution. Borrowed for the
    /// lifetime of the context — codegen runs synchronously and the
    /// caller already holds the registry that lowering used.
    pub registry: &'r CommandRegistry,
    /// The module's original source text, indexed by `current_span` to recover
    /// each command's surface text for `errorInfo` (`while executing "…"`).
    /// Empty when the caller did not supply it (hand-built test contexts).
    source: std::rc::Rc<str>,
    /// Per-argument "is a braced (`{…}`) word" flags for the command currently
    /// dispatching to a codegen hook (`try_bytecoded`). Set by [`Self::emit_call`]
    /// from the command's tokens and consulted by [`Self::emit_word_arg`] so a
    /// hook collapses a non-braced literal's backslashes exactly like the generic
    /// per-word path. Empty for hand-built test contexts (treated as non-braced).
    cmd_arg_braced: Vec<bool>,
}

impl<'r> CodegenCtx<'r> {
    /// Create a new emission context.
    ///
    /// When `is_proc` is true, variable references use LVT-based
    /// instructions; when false, stack-based instructions are used.
    /// `params` pre-populates the LVT with procedure parameter names.
    /// `registry` is the [`CommandRegistry`] consulted by codegen
    /// hooks (`try_bytecoded`); pass the same instance the lowering
    /// pass used so dialect-loaded specs are visible.
    #[must_use]
    pub fn new(is_proc: bool, params: &[&str], registry: &'r CommandRegistry) -> Self {
        Self {
            numbers: tcl_dialect::NumberSyntax::Tcl90,
            literals: LiteralTable::new(),
            lvt: LocalVarTable::new(params),
            instructions: Vec::new(),
            label_positions: HashMap::new(),
            label_counter: 0,
            is_proc,
            cmd_index: 0,
            start_cmd_end_label: None,
            break_target: None,
            continue_target: None,
            catch_depth: 0,
            seen_generic_invoke: false,
            used_generic_invoke: false,
            used_inline_cmd_subst: false,
            expr_func_depth: 0,
            pending_cond_end_label: None,
            proc_exit_label: None,
            pending_join_labels: HashMap::new(),
            current_source_line: 0,
            current_span: None,
            registry,
            source: "".into(),
            cmd_arg_braced: Vec::new(),
        }
    }

    /// Set the module source text (see [`Self::source`]) so emitted instructions
    /// carry their command's surface text for `errorInfo`.
    pub fn set_source(&mut self, source: &str) {
        self.source = source.into();
    }

    /// The surface text of the construct at `current_span`, for `errorInfo`.
    /// Empty when no span is set or no source was supplied.
    ///
    /// A command ending in a quoted (`"…"`) word has its `current_span` end at
    /// the word's inner end — [`segmenter::widen_word_end`] deliberately does not
    /// widen quoted words (other `cmd.range` consumers rely on the inner end), so
    /// the closing `"` sits one byte past `span.end()`. The `errorInfo` frame must
    /// quote the *whole* command (`"error "test error""`, eval-2.5), so include a
    /// trailing `"` here — the analogue of `widen_word_end`'s brace/bracket widen,
    /// scoped to error reporting.
    fn span_text(&self) -> String {
        match self.current_span {
            Some(sp) => {
                let (s, mut e) = (sp.start() as usize, sp.end() as usize);
                if self.source.as_bytes().get(e) == Some(&b'"') {
                    e += 1;
                }
                self.source.get(s..e).unwrap_or("").to_string()
            }
            None => String::new(),
        }
    }

    /// The surface text of an explicit `span` within the module source — for
    /// inline-body error regions, whose enclosing command's span differs from the
    /// per-instruction `current_span`. Empty when no source was supplied.
    pub(crate) fn source_text(&self, span: Span) -> String {
        let (s, e) = (span.start() as usize, span.end() as usize);
        self.source.get(s..e).unwrap_or("").to_string()
    }

    /// The 1-based source line of an explicit `span`'s start (its first byte).
    /// `0` when no source was supplied (the span can't be located).
    pub(crate) fn source_line(&self, span: Span) -> u32 {
        if self.source.is_empty() {
            return 0;
        }
        let start = span.start() as usize;
        let prefix = self.source.get(..start).unwrap_or("");
        1 + u32::try_from(prefix.bytes().filter(|&b| b == b'\n').count()).unwrap_or(0)
    }

    /// The 1-based line of `current_span` within the module source — the line a
    /// command reports in `errorInfo` (`(procedure … line N)` / `("while" body
    /// line N)`). `0` when no span / source is available.
    fn span_line(&self) -> u32 {
        match self.current_span {
            Some(sp) => {
                let start = sp.start() as usize;
                let prefix = self.source.get(..start).unwrap_or("");
                1 + u32::try_from(prefix.bytes().filter(|&b| b == b'\n').count()).unwrap_or(0)
            }
            None => 0,
        }
    }

    /// Append an instruction, returning its index in the stream.
    pub fn emit(&mut self, op: Op, operands: Vec<Operand>) -> usize {
        let idx = self.instructions.len();
        let mut instr = Instruction::new(op, operands);
        instr.source_span = self.current_span;
        instr.source_cmd_text = self.span_text();
        instr.source_line = self.span_line();
        self.instructions.push(instr);
        idx
    }

    /// Append an instruction with a comment, returning its index.
    pub fn emit_comment(&mut self, op: Op, operands: Vec<Operand>, comment: &str) -> usize {
        let idx = self.instructions.len();
        let mut instr = Instruction::new(op, operands);
        comment.clone_into(&mut instr.comment);
        instr.source_span = self.current_span;
        instr.source_cmd_text = self.span_text();
        instr.source_line = self.span_line();
        self.instructions.push(instr);
        idx
    }

    /// Generate a unique label name with the given prefix.
    #[must_use]
    pub fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.label_counter;
        self.label_counter += 1;
        format!("{prefix}_{n}")
    }

    /// Record that a label points to the *next* instruction to be emitted.
    pub fn place_label(&mut self, label: &str) {
        self.label_positions
            .insert(label.to_owned(), self.instructions.len());
    }

    /// Consume the context and produce a [`FunctionAsm`].
    #[must_use]
    pub fn into_function_asm(self, name: String) -> FunctionAsm {
        // Convert label_positions (instruction indices) to byte offsets.
        // Before layout, labels map to instruction indices.
        let labels = self.label_positions.into_iter().collect();
        FunctionAsm {
            name,
            literals: self.literals,
            lvt: self.lvt,
            instructions: self.instructions,
            labels,
            loop_targets: HashMap::new(),
            body_base_line: 0,
            error_regions: Vec::new(),
        }
    }
}
