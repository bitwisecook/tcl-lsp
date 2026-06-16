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
pub mod emitter;
pub mod expressions;
pub mod helpers;
pub mod peephole;
pub mod statements;
pub mod values;

pub use backend::{Backend, BytecodeBackend};
pub use emitter::{codegen_function, codegen_module};
// Bytecode artifact types moved to the `tcl-bytecode` crate; re-export them (and
// the `layout`/`format` modules) so `crate::codegen::{Op, FunctionAsm, …}`,
// `codegen::layout::*`, and `codegen::format::*` keep resolving for the emitter
// submodules, tests, and external consumers.
pub use tcl_bytecode::*;
pub use tcl_bytecode::{format, layout};

use std::collections::HashMap;

use tcl_registry::CommandRegistry;

// -- Emission context --

/// Mutable context for bytecode emission.
///
/// Replaces the Python `_Emitter` class-level state (`self.asm`,
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
    /// Command registry consulted by registry-driven codegen hooks.
    ///
    /// Threaded in by the caller so dialect-loaded specs (iRules,
    /// Tk, EDA) drive codegen-hook resolution. Borrowed for the
    /// lifetime of the context — codegen runs synchronously and the
    /// caller already holds the registry that lowering used.
    pub registry: &'r CommandRegistry,
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
            registry,
        }
    }

    /// Append an instruction, returning its index in the stream.
    pub fn emit(&mut self, op: Op, operands: Vec<Operand>) -> usize {
        let idx = self.instructions.len();
        self.instructions.push(Instruction::new(op, operands));
        idx
    }

    /// Append an instruction with a comment, returning its index.
    pub fn emit_comment(&mut self, op: Op, operands: Vec<Operand>, comment: &str) -> usize {
        let idx = self.instructions.len();
        let mut instr = Instruction::new(op, operands);
        comment.clone_into(&mut instr.comment);
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
        }
    }
}
