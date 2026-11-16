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

//! One instruction: its operand, error context, source command boundary and the command sites it enters.

use super::CompiledVariableTarget;
use super::op::Op;
use std::collections::HashMap;
use tcl_lexer::Span;

// Instruction operand

/// An instruction operand: either an immediate integer or a label reference.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// Integer immediate (literal index, LVT slot, jump offset, etc.).
    Imm(i32),
    /// Symbolic label reference (resolved during layout).
    Label(String),
}

/// Runtime ingredients for a TIP 348 `INNER` context that cannot be
/// reconstructed from source text after lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorStackContext {
    /// Record the actual returnImm operation from its result and original
    /// options stack values, while retaining authored errorInfo source text.
    ReturnImmediate {
        /// Exact source command responsible for the compiled error.
        error_info_command: Vec<u8>,
    },
    /// Build `{head <runtime-result>}` for TIP 348 while using the failing
    /// command itself, rather than its containing lowered instruction, for
    /// `errorInfo`.
    CommandResult {
        head: String,
        error_info_command: Vec<u8>,
    },
}

/// The command C's `TclLogCommandInfo` names: the command's source up to its
/// terminator, trailing whitespace included, and for an instruction lowered
/// from a command substitution the command inside the brackets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorInfoCommand {
    pub text: String,
    pub line: u32,
    /// Names a command inside a substitution, so an enclosing substitution
    /// keeps it.
    pub nested: bool,
    /// For a substitution in a statement the release evaluates without
    /// compiling it, the commands C's `TclEvalEx` logs after this one as the
    /// error leaves each enclosing substitution: innermost first, ending with
    /// the statement. Empty for compiled code, which logs only the innermost.
    pub enclosing: Vec<(String, u32)>,
}

/// Where an error an instruction raises comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorOrigin {
    #[default]
    Execution,
    /// An operation of a constant `expr` subexpression C folds at compile
    /// time: the error is the fold's failure, so `errorInfo` already holds
    /// the message when the command is logged.
    ConstantFold,
}

impl Instruction {
    /// The command text and line `errorInfo` logs for this instruction.
    #[must_use]
    pub fn error_info_text(&self) -> (&str, u32) {
        self.error_info_command.as_ref().map_or(
            // Work's command text is a byte `SourceImage`; a non-UTF-8 command
            // has no `&str` view, so it names no text here.
            (
                self.source_cmd_text.try_text().unwrap_or_default(),
                self.source_line,
            ),
            |command| (command.text.as_str(), command.line),
        )
    }

    /// The commands logged after [`Self::error_info_text`]'s
    /// ([`ErrorInfoCommand::enclosing`]).
    #[must_use]
    pub fn error_info_enclosing(&self) -> &[(String, u32)] {
        self.error_info_command
            .as_ref()
            .map_or(&[], |command| command.enclosing.as_slice())
    }
}

// Instruction

/// Whether an instruction begins an executable source command.
///
/// This is a typed execution fact rather than a diagnostic-text transition:
/// nested inline commands can retain an enclosing command's source metadata
/// without becoming a replay boundary for that enclosing command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceCommandBoundary {
    /// The instruction does not begin a new source command.
    #[default]
    None,
    /// The instruction begins a source command and may be replayed from here.
    Start,
    /// A nested `START_CMD` consumes an inline command and may replay it after
    /// command-table mutation, without becoming the owner of the enclosing
    /// source command's diagnostic span or continuation.
    InlineReplay,
}

impl SourceCommandBoundary {
    /// Whether this instruction starts a source command.
    #[must_use]
    pub const fn is_start(self) -> bool {
        matches!(self, Self::Start)
    }

    /// Whether this is a nested replay boundary that peephole passes must
    /// preserve even when the surrounding top-level function has no generic
    /// invoke.
    #[must_use]
    pub const fn is_inline_replay(self) -> bool {
        matches!(self, Self::InlineReplay)
    }
}

impl From<bool> for SourceCommandBoundary {
    fn from(is_start: bool) -> Self {
        if is_start { Self::Start } else { Self::None }
    }
}

/// Original producer purpose for a Native string-index instruction.
/// This tag distinguishes instruction recipes; it grants no Native authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStringIndexPurpose {
    /// The original registered String index compiler emits `INST_STR_INDEX`.
    RegisteredInstruction,
    /// Namespace qualifiers already reached Unicode through its preceding search.
    ReachedNamespaceUnicode,
}

/// A single bytecode instruction (labels unresolved until layout).
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // independent per-instruction flags, not a state machine
pub struct Instruction {
    /// Opcode.
    pub op: Op,
    /// Operands.
    pub operands: Vec<Operand>,
    /// Human-readable comment for disassembly.
    pub comment: String,
    /// Byte offset (filled by the layout pass; -1 before layout).
    pub offset: i32,
    /// Pattern → label map for `JUMP_TABLE` only.
    pub jump_table: Option<HashMap<String, String>>,
    /// Original counted native switch table. Only authenticated compiler output
    /// uses this; the runtime selects `CString` extent independently of Rust text.
    pub native_switch_bytes: Option<HashMap<Vec<u8>, String>>,
    /// C9.1 wide-integer jump table, independent of textual keys.
    pub native_switch_integers: Option<HashMap<i64, String>>,
    /// Physical opcode recipe for original switch comparison/materialization.
    pub native_switch_version: Option<tcl_dialect::TclVersion>,
    /// Exact original string-index producer purpose, without an entry grant.
    pub native_string_index_purpose: Option<NativeStringIndexPurpose>,
    /// Exact pre-TIP-232 function registration consumed by `CALL_FUNC1`.
    /// Its complete owning table is retained on [`FunctionAsm`]; this is
    /// independent of any same-spelled Tcl command or namespace binding.
    pub native_fixed_math_call:
        Option<tcl_runtime_api::native_compilation::NativeMathFunctionBinding>,
    /// Prevent push-pop folding (jump target result).
    pub no_fold: bool,
    /// 1-based source line for `errorInfo`.
    pub source_line: u32,
    /// Original literal word's line within this compiled source object.
    /// The runtime combines this with the actual source-file entry when
    /// retaining script values; command lines cannot supply word locations.
    pub source_value_line: Option<u32>,
    /// Original command text for `errorInfo`.
    pub source_cmd_text: tcl_lexer::SourceImage,
    /// Runtime-only recipe for structured error-stack logging.
    pub error_stack_context: Option<ErrorStackContext>,
    /// The command `errorInfo` names for this instruction where C's differs
    /// from [`Self::source_cmd_text`], which stays the replayable statement.
    pub error_info_command: Option<ErrorInfoCommand>,
    /// Whether an error raised here is a compile-time fold failure in C.
    pub error_origin: ErrorOrigin,
    /// Canonical unrooted constructed namespace in which
    /// [`Self::source_cmd_text`] resolves when this instruction is an
    /// executable command boundary. Empty denotes the global namespace.
    ///
    /// This is compiler provenance, not the VM activation's current
    /// namespace: executable inlining may copy a namespaced command into a
    /// caller whose frame resolves commands somewhere else.
    pub source_command_namespace: tcl_runtime_api::ByteNamespacePath,
    /// Exact original namespace owner, authoritative when present.
    pub source_command_namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
    /// This instruction begins an executable IR/source command. The VM uses
    /// this explicit compiler fact to find deoptimisation continuations;
    /// diagnostic span/text transitions are not execution boundaries because
    /// inline catch/try machinery may retain an enclosing command's metadata.
    /// A nested inline `START_CMD` uses [`SourceCommandBoundary::InlineReplay`]
    /// and its own opcode continuation; an outer source command's `START_CMD`
    /// carries [`SourceCommandBoundary::Start`].
    pub source_command_boundary: SourceCommandBoundary,
    /// Completion-option scope entered before this instruction executes.
    /// Structured control commands use this out-of-band semantic marker where
    /// their Tcl bytecode has no result-options opcode of its own.
    pub completion_option_scope: Option<tcl_runtime_api::completion_options::ActivationOptionScope>,
    /// Byte span of the source construct this instruction was lowered
    /// from, when known. `None` for synthetic instructions with no
    /// direct source (loop-result pushes, fallthrough jumps, padding
    /// NOPs). Stamped at emission time from `CodegenCtx::current_span`;
    /// the explorer maps it to a line:col `range` for click-to-source.
    pub source_span: Option<Span>,
    /// Loop-variable groups for `FOREACH_START`/`lmap` only — the analogue of
    /// C Tcl's `ForeachInfo.varLists` (`tclExecute.c` `INST_FOREACH_*`). One
    /// inner target vector per iterator group; the value lists are pushed on the
    /// stack before the opcode. Not rendered in disassembly (keeps identity
    /// stable).
    pub foreach_vars: Option<Vec<Vec<CompiledVariableTarget>>>,
    /// Authenticated original foreach auxiliary layout. These physical slots
    /// are compiler output, independent of textual loop-variable names.
    pub native_each: Option<std::sync::Arc<NativeEachAuxiliary>>,
    /// Original C list-index immediate encoding, independent of `INDEX_END`.
    pub native_list_index: Option<tcl_syntax::native_compiled_index::NativeCompiledListIndex>,
    /// Original C inclusive range coordinates, independent of portable `INDEX_END`.
    pub native_list_range: Option<tcl_syntax::native_compiled_index::NativeCompiledListRange>,
    /// Original C9 insertion worker, distinct from logical List replacement.
    pub native_list_insertion:
        Option<tcl_runtime_api::native_list_insertion::NativeListInsertionInstruction>,
    /// `FOREACH_START` only: this is a *collecting* loop (`lmap`), so the VM
    /// initialises a per-loop accumulator that `LMAP_COLLECT` appends to and the
    /// paired `FOREACH_END` materialises as `list(accum)`. Carried out-of-band
    /// alongside `foreach_vars` so the 5-byte operand form and disassembly stay
    /// byte-stable. `false` for a plain `foreach` and every other opcode.
    pub foreach_collect: bool,
    /// Target physical LVT indices for `DICT_UPDATE_START`/`DICT_UPDATE_END` —
    /// C Tcl's `DictUpdateInfo.varIndices`. One slot per original key object;
    /// execution reads and writes the retained cell, independent of same-name locals. Carried
    /// out-of-band (like `foreach_vars`) so the 9-byte on-disk operand form and
    /// disassembly stay byte-stable. `None` for every other opcode.
    pub dict_vars: Option<Vec<usize>>,
    /// `PUSH1`/`PUSH4` only: the literal is a *verbatim* (braced / constant)
    /// word and must be pushed exactly as-is, suppressing the runtime word
    /// substitution that the VM otherwise applies to `${…}` / `[…]` markers
    /// (see `tcl-vm::subst::subst_word`). This is the Rust analogue of the
    /// reference VM's brace/`_RAW_PREFIX` `PUSH` handling, carried out-of-band
    /// so the literal pool and disassembly stay byte-stable (keeps identity
    /// stable).
    pub push_verbatim: bool,
    /// `PUSH1`/`PUSH4` only: how the VM pushes the literal ([`LiteralRep`]).
    /// Carried out-of-band so the literal pool and disassembly stay
    /// byte-stable.
    pub literal_rep: LiteralRep,
    /// `PUSH1`/`PUSH4` only: how the emulated release evaluates the command
    /// the word belongs to, and so a command substitution the VM makes in it.
    pub evaluation: tcl_runtime_api::CommandEvaluation,
    /// `INVOKE_STK1`/`INVOKE_STK4` of a literal command only: how an emulated
    /// release that keeps the commands running bytecode compiled
    /// (`TclVersion::revalidates_running_bytecode`) binds the invoked command.
    /// Where it compiled the command into the running unit, a command-table
    /// change after the unit was entered does not reach this invocation.
    pub release_binding: Option<tcl_runtime_api::ReleaseBinding>,
    /// A command's boundary instruction only, under such a release: where the
    /// release compiled every command whose binding this command's
    /// specialised instructions rely on, so that running bytecode keeps them
    /// across a command-table change rather than evaluating the command
    /// afresh.
    pub release_hold: Option<tcl_runtime_api::ReleaseBinding>,
    /// A boundary of one of the commands of a script the release evaluates
    /// directly, as `tclsh` evaluates the script file it is given: the
    /// outermost command running, which C's level-0 evaluation names when it
    /// turns an unexpected completion code into an error.
    pub script_command: bool,
    /// Under such a release, the script body this compiler inlined that this
    /// instruction runs part of: `0` for none, otherwise one more than the
    /// body's index in the function.
    pub release_body: u32,
    /// The first instruction of [`Self::release_body`]'s body, where running
    /// it evaluates the body afresh.
    pub release_body_entry: Option<ReleaseBodyEntry>,
    /// Literal command-head `PUSH1`/`PUSH4` only: token provenance carried
    /// through argument substitution to its consuming invoke.
    pub entered_command: Option<EnteredCommandSite>,
    /// Actual native compiler selection made before this command's arguments run.
    /// It remains valid through its continuation even if an argument replaces
    /// or removes the callable that supplied the compiler hook.
    pub native_compiler_selection: Option<NativeCompilerSelectionSite>,
    /// Selected stock operations guard their actual prerequisites before argv.
    /// Several nested operations may share their first operand instruction.
    pub native_operation_selections: Vec<NativeOperationSelectionSite>,
    /// `BEGIN_CATCH4` only: the label of the range's handler — where the VM
    /// resumes (stack trimmed, caught completion recorded) when an exceptional
    /// completion unwinds into the range. This is the analogue of C Tcl's
    /// `ExceptionRange.catchOffset`, carried out-of-band (like `foreach_vars`)
    /// so the 4-byte operand keeps C's meaning (the range index) and the
    /// disassembly stays byte-stable. `None` marks a *decorative* range — the
    /// codegen emitted the C-faithful shape but relies on the VM's
    /// activation-stack unwinding instead, so `BEGIN_CATCH4` is inert.
    pub catch_target: Option<String>,
    /// First protected instruction for a catch whose stack entry is opened
    /// before unprotected argument substitution, as in Tcl 8.4 catch.
    /// Absent means the instruction following `BEGIN_CATCH4` is protected.
    pub catch_start: Option<String>,
    /// Exclusive end of the protected body, excluding a native epilogue that
    /// runs before `END_CATCH`. Absent uses the handler instruction as the end.
    pub catch_end: Option<String>,
    /// `INVOKE_STK1`/`INVOKE_STK4` only: the emulated release compiles the
    /// command this invoke leaves to the command at run time. Carried
    /// out-of-band so the disassembly stays byte-stable.
    pub release_compiled: Option<ReleaseCompiled>,
}

/// Where a script body this compiler inlined starts, under a release that
/// keeps the commands running bytecode compiled: a body whose owner the
/// release does not compile into the unit enclosing it runs as a unit of its
/// own, compiled afresh each time the owner evaluates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseBodyEntry {
    /// The body enclosing this one ([`Instruction::release_body`]).
    pub parent: u32,
    /// How the release binds the command that owns the body; `Invoked` for a
    /// command it evaluates directly.
    pub owner: tcl_runtime_api::ReleaseBinding,
}

/// A command the emulated release's compiler compiles with its words, which
/// the VM runs as the command does where it reads them as that compiled
/// command would.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseCompiled {
    /// The word, counting the command word, that the release compiles as a
    /// script with the command whatever it holds (C's `TclCompileCmdWord`,
    /// whose `INST_EVAL_STK` compiles the value): the VM compiles that value
    /// in place before it invokes, so a canonical list there is no longer
    /// one.
    pub script_word: Option<u8>,
    /// The words, one bit per index counting the command word, that must name
    /// compiled locals of the procedure frame the command runs in. A script
    /// the release compiles outside a procedure body creates no locals and
    /// reaches only that frame's (`TclFindCompiledLocal` without a
    /// `procPtr`), so it compiles the command only where every one is.
    pub frame_local_words: u8,
    /// Whether the release compiles the command only in a procedure body, so
    /// a script compiled apart from one compiles it only where it runs as the
    /// body of a command the release compiled into a procedure body.
    pub in_procedure_only: bool,
}

impl Instruction {
    /// Original command namespace for transparent stale-command replay.
    #[must_use]
    pub fn source_namespace_context(&self) -> tcl_runtime_api::CompiledNamespaceContext {
        self.source_command_namespace_context
            .clone()
            .unwrap_or_else(|| {
                tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(
                    self.source_command_namespace.clone(),
                )
            })
    }

    /// Create a new instruction with default metadata.
    #[must_use]
    pub fn new(op: Op, operands: Vec<Operand>) -> Self {
        Self {
            op,
            operands,
            comment: String::new(),
            offset: -1,
            jump_table: None,
            native_switch_bytes: None,
            native_switch_integers: None,
            native_switch_version: None,
            native_string_index_purpose: None,
            native_fixed_math_call: None,
            no_fold: false,
            source_line: 0,
            source_value_line: None,
            source_cmd_text: tcl_lexer::SourceImage::default(),
            error_stack_context: None,
            error_info_command: None,
            error_origin: ErrorOrigin::Execution,
            source_command_namespace: tcl_runtime_api::ByteNamespacePath::root(),
            source_command_namespace_context: None,
            source_command_boundary: SourceCommandBoundary::None,
            completion_option_scope: None,
            source_span: None,
            foreach_vars: None,
            native_each: None,
            native_list_index: None,
            native_list_range: None,
            native_list_insertion: None,
            foreach_collect: false,
            dict_vars: None,
            push_verbatim: false,
            literal_rep: LiteralRep::Word,
            evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
            release_binding: None,
            release_hold: None,
            script_command: false,
            release_body: 0,
            release_body_entry: None,
            entered_command: None,
            native_compiler_selection: None,
            native_operation_selections: Vec::new(),
            catch_target: None,
            catch_start: None,
            catch_end: None,
            release_compiled: None,
        }
    }
}

/// Original foreach auxiliary indices and release-selected storage owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEachAuxiliary {
    /// Compiler release, independently checked against the executing engine.
    pub version: tcl_dialect::TclVersion,
    /// Ordered variable slots for each original value-list group.
    pub variables: Vec<Vec<usize>>,
    /// C8.4/8.5 value-list locals followed by their counter local.
    /// Empty for stack-based C8.6 and later iteration.
    pub temporaries: Vec<usize>,
}

/// How the VM pushes a literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LiteralRep {
    /// A word the VM substitutes as it pushes it (see
    /// `tcl-vm::subst::subst_word`).
    #[default]
    Word,
    /// Its string exactly as it is, with no word substitution: a braced or
    /// constant word, or a value the compiler finished. The analogue of the
    /// reference VM's brace/`_RAW_PREFIX` `PUSH` handling.
    Verbatim,
    /// A list the compiler built from literal words, spelt canonically: C's
    /// `TclCompileListCmd` pushes such a list as its list object, a canonical
    /// list.
    List,
}

/// A statically literal command entered before its remaining words perform
/// substitutions. The VM validates and resolves `binding` at the command-head
/// push and carries the entry through `end`, which labels the instruction
/// immediately after the consuming invoke. Generic invokes dispatch that
/// retained token; specialised `INVOKE_REPLACE` uses it only to guard the
/// outer substitution range, then resolves the rewritten implementation word
/// at the invoke itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnteredCommandSite {
    pub binding: tcl_runtime_api::CommandBindingIdentity,
    pub end: String,
}

/// Actual native compiler operation entered before argument substitutions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCompilerSelectionSite {
    /// Exact runtime callable/header prerequisite; no catalogue substitution.
    pub prerequisite: tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite,
    /// Label immediately following the selected invocation's instructions.
    pub end: String,
}

/// Exact stock-operation selection and its original replay boundary.
/// These prerequisites are checked once; later argument mutations cannot
/// replace a selected native opcode with a newly looked-up public command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOperationSelectionSite {
    /// Exact raw command or ensemble compiler registration, independent of the callable.
    pub compiler_prerequisite: Option<
        std::sync::Arc<tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite>,
    >,
    /// Actual implementation lookups required by the selected native compiler.
    pub requirements: Vec<tcl_runtime_api::CommandBindingIdentity>,
    /// Native selection timing, independent of operand evaluation.
    pub guard: tcl_runtime_api::CommandBindingGuard,
    /// Label immediately after the selected operation's instruction range.
    pub end: String,
    /// Original Tcl command, replayed only when the initial premise fails.
    pub source: tcl_lexer::SourceImage,
    /// Exact authored command extent for diagnostics.
    pub span: Span,
    /// Rooted constructed command namespace of the original lookup.
    pub namespace: tcl_runtime_api::ByteNamespacePath,
    /// Exact original namespace owner for stale-operation replay.
    pub namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
}

impl NativeOperationSelectionSite {
    /// Namespace owner retained by the original command, including its lifetime.
    #[must_use]
    pub fn replay_namespace_context(&self) -> tcl_runtime_api::CompiledNamespaceContext {
        self.namespace_context.clone().unwrap_or_else(|| {
            tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(self.namespace.clone())
        })
    }

    /// Retain the actual compiler kind independently of late callable lookup.
    #[must_use]
    pub fn compiler_selection_prerequisite(
        &self,
    ) -> Option<tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite> {
        self.compiler_prerequisite.as_ref().map(|required| {
            tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::from_command_registration(
                std::sync::Arc::clone(required),
            )
        })
    }
}
