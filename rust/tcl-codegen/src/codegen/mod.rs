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
/// Stateless emitter helpers; they live in `tcl-compiler`, whose constant
/// folding shares them.
pub mod helpers {
    pub use tcl_compiler::codegen_helpers::*;
}
mod hook_operands;
mod native_compiler_pass;
mod native_failure;
pub(crate) mod native_substitution;
mod operand_width;
pub mod peephole;
pub mod statements;
pub mod structured;
pub mod values;
pub mod wasm;

pub use backend::{Backend, BytecodeBackend};
pub use emitter::{codegen_function, codegen_module, codegen_module_with_command_mutations};
// Bytecode artifact types moved to the `tcl-bytecode` crate; re-export them (and
// the `layout`/`format` modules) so `crate::codegen::{Op, FunctionAsm, …}`,
// `codegen::layout::*`, and `codegen::format::*` keep resolving for the emitter
// submodules, tests, and external consumers.
pub use tcl_bytecode::*;
pub use tcl_bytecode::{format, layout};

use std::collections::{BTreeSet, HashMap};

use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

mod error_info;
mod release_evaluation;
mod site_bindings;
mod source_sites;
mod splice_emission;

// Emission context.

/// Reentrant emitter scopes may share a range only for the same selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NativeOperationOrigin {
    start: usize,
    source: tcl_compiler::command_binding::CommandAllocationSite,
    operation: tcl_registry::SemanticOperationId,
    guard: tcl_runtime_api::CommandBindingGuard,
    compiler_prerequisite: Option<
        std::sync::Arc<tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite>,
    >,
    requirements: Vec<tcl_runtime_api::CommandBindingIdentity>,
}

/// One native namespace identity with an optional checked legacy analysis key.
/// The key is a projection of these exact segments, never a fallback namespace.
#[derive(Debug, Clone)]
struct EmissionNamespace {
    path: tcl_runtime_api::ByteNamespacePath,
    key: Option<String>,
}
impl EmissionNamespace {
    fn native(path: tcl_runtime_api::ByteNamespacePath) -> Self {
        let key = tcl_syntax::naming::checked_namespace_path_key(&path);
        Self { path, key }
    }
}

/// Adapt a symbolic compatibility key through its declared component convention.
/// Native paths never enter this function through display or UTF-8 replacement.
fn namespace_path_from_constructed_key(key: &str) -> tcl_runtime_api::ByteNamespacePath {
    tcl_runtime_api::ByteNamespacePath::from_segments(tcl_syntax::naming::key_segments(key))
}

/// Retained native geometry takes precedence over the presentation key.
fn namespace_path_for_binding(
    binding: &tcl_runtime_api::CommandBindingIdentity,
) -> tcl_runtime_api::ByteNamespacePath {
    binding.namespace_context.as_ref().map_or_else(
        || namespace_path_from_constructed_key(&binding.resolution_namespace),
        |context| context.path().clone(),
    )
}

/// Native body loop range. Labels are retained through peephole edits and
/// resolved only against the final instruction layout.
#[derive(Debug)]
struct InlineLoopRegion {
    start: String,
    end: String,
    continue_target: Option<String>,
    break_target: String,
}

/// The compiled locals of a script body's own compile, keyed by its text and
/// re-lex configuration ([`CodegenCtx::intern_script_locals`]). One compile
/// shares it with the nested compiles it makes for that purpose, so a body
/// nested `n` deep is compiled for its locals once rather than once per
/// enclosing level.
pub(crate) type ScriptLocalsMemo = std::rc::Rc<
    std::cell::RefCell<HashMap<(String, tcl_lexer::LexerConfig), std::rc::Rc<[String]>>>,
>;

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
// readability or perf gain.
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
    /// The backslash-escape grammar of the release being compiled *for*.
    ///
    /// Threaded from `IrModule::dialect` beside [`Self::numbers`], so a literal
    /// word's escapes are decoded the way the target release reads them —
    /// `\x4142` is `B` when compiling for 8.5 and `A42` from 8.6.
    /// Defaults to 9.0 for the hand-built contexts in tests.
    pub escapes: tcl_dialect::EscapeSyntax,
    /// The word-value rules of the release being compiled *for* — whether a
    /// braced word's `\<newline>` folds, and whether malformed list text
    /// raises.
    ///
    /// Threaded from `IrModule::dialect` beside [`Self::numbers`] and
    /// [`Self::escapes`], and for the same reason: a braced literal's bytes
    /// are the target dialect's, not whatever the emitting host would do.
    /// Before this, `push_lit_verbatim` collapsed unconditionally and a Jim
    /// `set x {a\<newline>b}` compiled to the Tcl value `a b`.
    pub word_rules: tcl_syntax::word_rules::WordValueRules,
    /// The `${…}` variable-name close rule of the release being compiled
    /// *for*.
    ///
    /// Threaded from `IrModule::dialect` beside [`Self::numbers`] and
    /// [`Self::escapes`], and for the same reason: `Tcl_ParseVarName` changed
    /// between 8.x and 9.x, so a `${…}` reference must be decoded the way the
    /// *target* release reads it. 9.x counts nested `{…}` and consumes `\X` as
    /// an inert pair, making `${a{b}c}` the variable `a{b}c`; the 8.x family
    /// ends the name at the first literal `}`, making it `a{b` followed by the
    /// ordinary word text `c}`.
    ///
    /// Original source references and templates use the complete selected
    /// [`Self::lexer_config`]. The retained close rule defaults to 9.0 for
    /// hand-built contexts.
    pub braced_var: tcl_dialect::BracedVarStyle,
    /// The resolved profile of the release being compiled *for*, from the
    /// name the lowering pass received (`IrModule::dialect`).  `None` means
    /// the compile named *no* dialect, and only that: a named-but-unknown
    /// dialect still resolves (through `by_name`, to the permissive
    /// fallback) and stays `Some`.
    ///
    /// The distinction matters because the readers branch on `is_some()`, not
    /// on the profile's identity — `parse_expr_for_profile` in
    /// [`codegen::control_flow`](crate::codegen::control_flow) and
    /// [`codegen::cmd_subst`](crate::codegen::cmd_subst) pick the target
    /// grammar when this is `Some` and the thread-ambient one when it is
    /// `None`. Resolving the name with `DialectProfile::find` here would
    /// answer `None` for `tk` and for any unrecognised name and silently move
    /// those compiles onto the ambient grammar.
    ///
    /// This is the `expr` half of the same fact [`Self::numbers`] and
    /// [`Self::escapes`] carry: it resolves the grammar a re-parsed `expr`
    /// body is read under and, through
    /// [`RuntimeExprSurface`](tcl_registry::expr_surface::RuntimeExprSurface),
    /// which
    /// operators the target release's `expr` actually has.
    /// A dialect-less compile stays distinguishable from one that named plain
    /// `tcl`: `parse_expr`'s numeral grammar follows the ambient runtime
    /// syntax for the former and the profile's for the latter.
    pub dialect: Option<&'static tcl_dialect::DialectProfile>,
    /// The grammar a *named* compile re-parses its `expr` bodies under —
    /// the same [`tcl_dialect::LexerGrammar`] [`Self::numbers`],
    /// [`Self::escapes`], [`Self::braced_var`] and [`Self::word_rules`] were
    /// taken from, so a numeral means one thing throughout a compile. `None`
    /// is the dialect-less compile, whose `expr` bodies follow the ambient
    /// runtime syntax (see [`Self::dialect`]); it is never the fallback for
    /// a named dialect. Before this, a named compile re-parsed under
    /// [`Self::dialect`]'s profile while emitting under a grammar resolved
    /// from the name — two currencies, and for `tk` two different answers
    /// to what `010` is inside one compile.
    pub expr_grammar: Option<tcl_dialect::LexerGrammar>,
    /// Literal constant pool.
    pub literals: LiteralTable,
    /// Local variable table.
    pub lvt: LocalVarTable,
    /// The live splices this function's statements stand in.
    pub(crate) splices: splice_emission::SpliceEmission,
    /// Instruction stream (append-only during emission).
    pub instructions: Vec<Instruction>,
    /// Label name → instruction index (populated by [`place_label`]).
    pub(crate) label_positions: HashMap<String, usize>,
    /// Monotonic counter for generating unique label names.
    label_counter: u32,
    /// [`ScriptLocalsMemo`] of this compile and the nested ones it makes.
    script_locals_memo: ScriptLocalsMemo,
    /// Whether we are compiling a proc body (affects LVT vs stack ops).
    ///
    /// This is the *function's* shape. It is not the same question as "may a
    /// variable here be addressed as a compiled local" — see
    /// [`Self::compiles_locals`].
    pub is_proc: bool,
    /// Source spans of the same-frame script bodies this function folded into
    /// its own instruction stream (`eval {…}`), from the CFG's
    /// `inline_body_error_sites`.
    ///
    /// The fold is this compiler's optimisation; C has no `eval` compiler, so
    /// the script becomes its own unit whose variables are *not* the enclosing
    /// proc's compiled locals. Codegen therefore keeps the dispatched variable
    /// forms inside these spans, which is what makes an in-proc
    /// `eval {lappend l z}` fire the `read` C fires.
    pub(crate) same_frame_eval_spans: Vec<(u32, u32)>,
    /// Command index for `startCommand` numbering.
    pub cmd_index: u32,
    /// End label for the current `startCommand` (paired by `end_command`).
    pub start_cmd_end_label: Option<String>,
    /// Loop break target label (set by the emitter loop).
    pub break_target: Option<String>,
    /// Loop continue target label (set by the emitter loop).
    pub continue_target: Option<String>,
    /// Actual instruction regions emitted inside a source command. Labels
    /// survive peephole edits and select the same native loop completion door.
    inline_loop_regions: Vec<InlineLoopRegion>,
    /// The break and continue labels each open native body can jump to;
    /// `None` where a body puts the enclosing loop out of reach.
    native_loop_targets: Vec<Option<(String, Option<String>)>>,
    /// Targets a loop emitter announced for the body script that follows.
    pending_native_loop: Option<(String, Option<String>)>,
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
    current_span: Option<Span>,
    /// Command registry consulted by registry-driven codegen hooks.
    ///
    /// Threaded in by the caller so dialect-loaded specs (iRules,
    /// Tk, EDA) drive codegen-hook resolution. Borrowed for the
    /// lifetime of the context — codegen runs synchronously and the
    /// caller already holds the registry that lowering used.
    pub registry: &'r CommandRegistry,
    /// Rooted constructed namespace in which command heads emitted directly
    /// by codegen resolve. IR-carried bindings retain their own source-site
    /// namespace and are never rewritten to this value.
    resolution_namespace: EmissionNamespace,
    /// Whole-module command-mutation summary — which command *names* may stop
    /// denoting their original builtin anywhere in this compilation unit.
    ///
    /// C Tcl inline-compiles a builtin unconditionally but guards every
    /// compiled command with `INST_START_CMD`, which re-dispatches the slow
    /// way once `iPtr->compileEpoch` moves — so `rename dict {}` earlier in
    /// the file makes the *compiled* `dict create` call fall back and raise
    /// `invalid command name "dict"` (tclExecute.c, `instStartCmdFailed`).
    /// Compiled artifacts now carry the equivalent typed binding requirements
    /// and source boundaries for runtime epoch revalidation. Fully consuming
    /// transforms that have no independently replayable command boundary still
    /// need this conservative static fact; entered-token specialisations retain
    /// their exact binding and are revalidated when execution reaches them.
    ///
    /// `None` means the caller supplied **no whole-module view** — the
    /// hand-built emitter contexts in unit tests and the per-function
    /// [`Backend::lower_function`](crate::codegen::backend::Backend::lower_function)
    /// seam, which is handed one CFG and never sees the module. Those trust
    /// every name;
    /// [`codegen_module`](crate::codegen::codegen_module), the whole-unit
    /// entry point every production pipeline uses, always supplies the scan.
    pub command_bindings: Option<&'r tcl_compiler::command_binding::ModuleCommandMutations>,
    /// Registry identities assumed by specialised operations emitted into this
    /// function. The bytecode artifact carries these to the runtime.
    pub command_binding_requirements: BTreeSet<tcl_runtime_api::CommandBindingIdentity>,
    native_compiler_prerequisites:
        Vec<tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite>,
    /// Actual runtime policies, independent of the retained authoring profile.
    invocation_dialect: Option<tcl_registry::InvocationDialect>,
    /// Independently selected source-local compiler, including logical providers.
    compiled_variable_protocol: Option<tcl_syntax::naming::NativeCompiledVariableProtocol>,
    borrowed_local_layout: Option<tcl_runtime_api::native_compilation::NativeCompiledLocalLayout>,
    required_compiled_local_layout:
        Option<tcl_runtime_api::native_compilation::NativeCompiledLocalLayout>,
    source_string_protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
    native_entry: Option<&'r tcl_runtime_api::NativeCompilationEntry>,
    /// Exact ingress lexical axes for all nested parsing and preparation.
    ingress_lexer_config: Option<tcl_lexer::LexerConfig>,
    /// Exact source-owned implicit calls retained by CFG lowering.
    math_invocations: Vec<tcl_compiler::command_binding::SourceMathInvocation>,
    /// Whole-expression entry proofs from the same exact source inventory.
    expression_preparations: Vec<tcl_compiler::command_binding::SourceExpressionPreparation>,
    /// Preparation selected for the expression currently emitted.
    active_expression_preparation:
        Option<std::sync::Arc<tcl_registry::runtime_expr_validation::PreparedExpressionWitness>>,
    /// One-shot source ownership supplied by the CFG statement emitter.
    pending_math_source:
        Option<std::sync::Arc<tcl_compiler::command_binding::ExecutedScriptSource>>,
    /// Source/base scoped to the expression currently being emitted.
    math_source: Option<std::sync::Arc<tcl_compiler::command_binding::ExecutedScriptSource>>,
    math_expression_base: Option<u32>,
    /// Actual fixed-function entry consumed by successful constant folding.
    math_table_prerequisite:
        Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
    /// A transformed CFG retained a dependency which this artifact cannot guard.
    native_dependency_refusal: bool,
    /// Hazards captured from actual emitted instructions before peepholes.
    native_compiler_pass_hazards: Vec<tcl_registry::native_compiler_pass::NativeCompilerPassHazard>,
    /// Actual second compiler pass omits executable `START_CMD` markers.
    compact_compiler_pass: bool,
    /// Active C8.4 `Catch` body compilation checkpoints, independently of
    /// exception ranges entered by the eventual runtime instructions.
    native_speculative_compilations: Vec<std::rc::Rc<std::cell::Cell<bool>>>,
    /// Spec-pack facts specialised operations emitted into this function rest
    /// on ([`tcl_compiler::site_claims`]). The artefact carries these beside the
    /// bindings, and admission requires each one's stamp.
    pub site_claim_requirements: BTreeSet<tcl_runtime_api::SiteClaim>,
    /// Suppress registry codegen hooks as well as lowering hooks.
    pub plain_command_dispatch: bool,
    /// The module's original source text, indexed by `current_span` to recover
    /// each command's surface text for `errorInfo` (`while executing "…"`).
    /// Empty when the caller did not supply it (hand-built test contexts).
    source: tcl_lexer::SourceImage,
    /// The lexer-owned line index for [`Self::source`]. Production module
    /// emission shares one Arc-backed index across every function context.
    line_index: Option<tcl_lexer::LineIndex>,
    /// Whether [`Self::current_span`] denotes an executable Tcl command rather
    /// than CFG control machinery such as a condition or body-edge jump.
    /// Both need source ranges for diagnostics, but only a command supplies a
    /// safe whole script for runtime command-table revalidation.
    current_span_is_command: bool,
    /// Complete original command extent supplied by the byte word owner.
    /// Its closing delimiters need no legacy representative-span widening.
    exact_command_source: Option<tcl_lexer::SourceImage>,
    /// The spans of the commands the release evaluates without compiling
    /// them: the top level's own commands where the module says so
    /// ([`tcl_compiler::ir::Module::top_level_evaluation`]), and none elsewhere.
    pub(crate) direct_commands: Vec<Span>,
    /// Whether the block being emitted runs a script body the release
    /// evaluates without compiling it (8.4's `eval` and `uplevel`).
    pub(crate) direct_body: bool,
    /// The commands of the script bodies the release evaluates without
    /// compiling them ([`Self::direct_body`]), as their blocks are emitted.
    pub(crate) direct_body_commands: Vec<Span>,
    /// The regions of the script bodies inlined here that the release
    /// evaluates directly rather than compiling where they stand.
    pub(crate) direct_regions: Vec<Span>,
    /// The directly evaluated statements a substitution's `errorInfo` levels
    /// were named under, so those levels survive the function's finish.
    pub(crate) direct_levels: Vec<String>,
    /// The directly evaluated statement being emitted, if any
    /// ([`Self::direct_statement_error_info`]).
    pub(crate) direct_statement: Option<(String, u32)>,
    /// How many expressions enclose the code being emitted. The release
    /// compiles an expression, and the commands in it, even in a command it
    /// evaluates without compiling.
    expr_depth: u32,
    /// Per command span, under a release that keeps the commands running
    /// bytecode compiled: where it compiled every command whose binding the
    /// command's specialised instructions retain
    /// ([`tcl_bytecode::Instruction::release_hold`]).
    release_holds: HashMap<Span, tcl_runtime_api::ReleaseBinding>,
    /// Unrooted constructed namespace paired with the current executable
    /// command source site. This follows explicit IR binding sites across
    /// inlining rather than inheriting the surrounding function's namespace.
    current_command_namespace: tcl_runtime_api::ByteNamespacePath,
    current_command_namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
    /// Per-argument "is a braced (`{…}`) word" flags for the command currently
    /// dispatching to a codegen hook (`try_bytecoded`). Set by [`Self::emit_call`]
    /// from the command's tokens and consulted by [`Self::emit_word_arg`] so a
    /// hook collapses a non-braced literal's backslashes exactly like the generic
    /// per-word path. Empty for hand-built test contexts (treated as non-braced).
    cmd_arg_braced: Vec<bool>,
    /// Inert logical selectors currently scoped to a private backend hook.
    native_hook_layout: Option<(String, usize)>,
    /// Original invocation proof scoped to the current emitter entry.
    invocation_tokens: Option<Box<tcl_compiler::ir::CommandTokens>>,
    /// Derived lexical words belong only to the immutable invocation scope.
    original_lexical_words: std::cell::RefCell<Option<values::OriginalLexicalWords>>,
    /// Actual authored carriers for inline script commands, shared by the module.
    source_proofs: Option<std::sync::Arc<tcl_compiler::command_binding::BodySourceProofs>>,
    /// First instruction and exact source/selection premises to its range label.
    native_operation_origins: HashMap<NativeOperationOrigin, String>,
    /// Authored body coordinate, available only when the original literal survives.
    inline_body_source_base: Option<u32>,
    /// Native bytecode entry supplied by the module driver.
    native_compilation: tcl_registry::native_compilation::NativeCompilationContext,
}

/// The native compilation a function starts in: a bytecode object for a
/// procedure's code or a script's, outside any loop or `catch`.
const fn initial_native_compilation(
    is_proc: bool,
) -> tcl_registry::native_compilation::NativeCompilationContext {
    tcl_registry::native_compilation::NativeCompilationContext {
        mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
        frame: if is_proc {
            tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode
        } else {
            tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode
        },
        loop_depth: 0,
        catch_depth: Some(0),
    }
}

impl<'r> CodegenCtx<'r> {
    /// Parse an `expr` body the way this compile reads expressions: under
    /// [`Self::expr_grammar`] for a named dialect, under the ambient runtime
    /// syntax for a dialect-less compile. The one entry point codegen uses,
    /// so its re-parsed numerals cannot diverge from the ones it emits.
    #[must_use]
    pub fn parse_compile_expr(&self, source: &str) -> tcl_compiler::expr_ast::ExprNode {
        match (&self.expr_grammar, self.dialect) {
            (Some(grammar), _) => {
                tcl_compiler::expr_parser::parse_expr_with_grammar(source, grammar)
            }
            // A context built from a profile alone (tests, and hosts that do
            // not come through `codegen_module`) parses under that profile.
            (None, Some(profile)) => {
                tcl_compiler::expr_parser::parse_expr_for_profile(source, Some(profile))
            }
            // The dialect-less compile: the ambient runtime syntax.
            (None, None) => tcl_compiler::expr_parser::parse_expr_for_profile(source, None),
        }
    }

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
        let compiled_variable_protocol = Some(
            tcl_syntax::naming::NativeCompiledVariableProtocol::authored_tcl(
                tcl_dialect::TclVersion::V9_0,
            ),
        );
        let mut lvt = LocalVarTable::new(params);
        lvt.set_native_protocol(compiled_variable_protocol);
        Self {
            numbers: tcl_dialect::NumberSyntax::default(),
            escapes: tcl_dialect::EscapeSyntax::default(),
            word_rules: tcl_syntax::word_rules::WordValueRules::default(),
            braced_var: tcl_dialect::BracedVarStyle::default(),
            dialect: None,
            expr_grammar: None,
            literals: LiteralTable::new(),
            lvt,
            splices: splice_emission::SpliceEmission::default(),
            instructions: Vec::new(),
            label_positions: HashMap::new(),
            label_counter: 0,
            script_locals_memo: ScriptLocalsMemo::default(),
            is_proc,
            same_frame_eval_spans: Vec::new(),
            cmd_index: 0,
            start_cmd_end_label: None,
            break_target: None,
            continue_target: None,
            inline_loop_regions: Vec::new(),
            native_loop_targets: Vec::new(),
            pending_native_loop: None,
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
            resolution_namespace: EmissionNamespace::native(
                tcl_runtime_api::ByteNamespacePath::root(),
            ),
            command_bindings: None,
            command_binding_requirements: BTreeSet::new(),
            native_compiler_prerequisites: Vec::new(),
            invocation_dialect: None,
            compiled_variable_protocol,
            borrowed_local_layout: None,
            required_compiled_local_layout: None,
            source_string_protocol: Some(tcl_syntax::native_string::NativeStringProtocol::C(
                tcl_dialect::TclVersion::V9_0,
            )),
            native_entry: None,
            ingress_lexer_config: None,
            math_invocations: Vec::new(),
            expression_preparations: Vec::new(),
            active_expression_preparation: None,
            pending_math_source: None,
            math_source: None,
            math_expression_base: None,
            math_table_prerequisite: None,
            native_dependency_refusal: false,
            native_compiler_pass_hazards: Vec::new(),
            compact_compiler_pass: false,
            native_speculative_compilations: Vec::new(),
            site_claim_requirements: BTreeSet::new(),
            plain_command_dispatch: false,
            source: tcl_lexer::SourceImage::default(),
            line_index: None,
            current_span_is_command: false,
            exact_command_source: None,
            direct_commands: Vec::new(),
            direct_body: false,
            direct_body_commands: Vec::new(),
            direct_regions: Vec::new(),
            direct_levels: Vec::new(),
            direct_statement: None,
            expr_depth: 0,
            release_holds: HashMap::new(),
            current_command_namespace: tcl_runtime_api::ByteNamespacePath::root(),
            current_command_namespace_context: None,
            cmd_arg_braced: Vec::new(),
            native_hook_layout: None,
            invocation_tokens: None,
            original_lexical_words: std::cell::RefCell::new(None),
            source_proofs: None,
            native_operation_origins: HashMap::new(),
            inline_body_source_base: None,
            native_compilation: initial_native_compilation(is_proc),
        }
    }

    /// Create a function emission context using actual native formal keys.
    /// This seeds local slots without interpreting or decoding names.
    #[must_use]
    pub fn with_native_parameters(
        is_proc: bool,
        params: &[tcl_runtime_api::NameBytes],
        registry: &'r CommandRegistry,
    ) -> Self {
        let mut context = Self::new(is_proc, &[], registry);
        context.lvt = LocalVarTable::from_native_names(params);
        context
            .lvt
            .set_native_protocol(context.compiled_variable_protocol);
        context
    }

    /// The compile's own lexer config — the dialect grammar every nested
    /// re-lex in codegen (a re-lowered body, a segmented catch body) must use,
    /// rather than the default grammar.
    #[must_use]
    pub fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.ingress_lexer_config.unwrap_or_else(|| {
            tcl_lexer::LexerConfig::for_profile(self.dialect.or_else(|| self.registry.profile()))
        })
    }

    /// Whether nested source reparsed by codegen recognises TIP 157 argument
    /// expansion. This delegates to the compile's central re-lex configuration
    /// so module-profile and profile-projected-registry consumers agree.
    #[must_use]
    pub(crate) fn recognises_expand_syntax(&self) -> bool {
        self.lexer_config().expand_syntax
    }

    /// Append an instruction, returning its index in the stream.
    pub fn emit(&mut self, op: Op, operands: Vec<Operand>) -> usize {
        let idx = self.instructions.len();
        let mut instr = Instruction::new(op, operands);
        instr.source_span = self.current_span;
        instr.evaluation = self.command_evaluation();
        instr.source_cmd_text = self.command_span_text();
        instr.source_line = self.span_line();
        // A non-UTF-8 command has no text view to stamp; it names none.
        instr.error_info_command =
            self.trailing_space_error_info(instr.source_cmd_text.try_text().unwrap_or_default());
        if !instr.source_cmd_text.is_empty() {
            instr
                .source_command_namespace
                .clone_from(&self.current_command_namespace);
            instr
                .source_command_namespace_context
                .clone_from(&self.current_command_namespace_context);
        }
        instr.source_command_boundary =
            (op == Op::START_CMD && !instr.source_cmd_text.is_empty()).into();
        self.instructions.push(instr);
        idx
    }

    /// Append an instruction with a comment, returning its index.
    pub fn emit_comment(&mut self, op: Op, operands: Vec<Operand>, comment: &str) -> usize {
        let idx = self.instructions.len();
        let mut instr = Instruction::new(op, operands);
        comment.clone_into(&mut instr.comment);
        instr.source_span = self.current_span;
        instr.evaluation = self.command_evaluation();
        instr.source_cmd_text = self.command_span_text();
        instr.source_line = self.span_line();
        // A non-UTF-8 command has no text view to stamp; it names none.
        instr.error_info_command =
            self.trailing_space_error_info(instr.source_cmd_text.try_text().unwrap_or_default());
        if !instr.source_cmd_text.is_empty() {
            instr
                .source_command_namespace
                .clone_from(&self.current_command_namespace);
            instr
                .source_command_namespace_context
                .clone_from(&self.current_command_namespace_context);
        }
        instr.source_command_boundary =
            (op == Op::START_CMD && !instr.source_cmd_text.is_empty()).into();
        self.instructions.push(instr);
        idx
    }

    /// Mark an emitted instruction as the start of a control completion's
    /// option scope. The marker stays out-of-band so Tcl bytecode layout and
    /// disassembly remain stable while every runtime consumer sees the same
    /// typed policy.
    pub(crate) fn mark_completion_option_scope(
        &mut self,
        instruction: usize,
        policy: tcl_runtime_api::completion_options::ControlOptionPolicy,
    ) {
        self.instructions[instruction].completion_option_scope = Some(policy.activation);
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
            required_compiled_local_layout: self.required_compiled_local_layout,
            native_compilation_failure: None,
            native_math_table_prerequisite: self.math_table_prerequisite,
            native_compiler_prerequisites: self.native_compiler_prerequisites,
            native_compilation_preflight: if self.native_dependency_refusal {
                tcl_runtime_api::NativeCompilationPreflight::ProviderRequired
            } else {
                tcl_runtime_api::NativeCompilationPreflight::NotRequired
            },
            literals: self.literals,
            lvt: self.lvt,
            instructions: self.instructions,
            labels,
            loop_targets: HashMap::new(),
            body_base_line: 0,
            proc_body_src: None,
            error_regions: Vec::new(),
            splice_ranges: Vec::new(),
            plain_command_dispatch: self.plain_command_dispatch,
            command_bindings: self.command_binding_requirements.into_iter().collect(),
            procedure_bindings: Vec::new(),
            site_claims: self.site_claim_requirements.into_iter().collect(),
        }
    }
}

/// The current binding ABI validates registry implementations at chunk entry.
/// A source procedure or prefixed alias needs its own compiler-header witness.
fn expression_script_dependency(
    dependency: &tcl_compiler::command_binding::SourceNativeCompilationDependency,
) -> Option<tcl_compiler::registry_invocation::NativeCompilationDependency> {
    (dependency.guard == tcl_registry::native_compilation::NativeCompilationGuard::ChunkEntry
        && (dependency.compiler_prerequisite.is_some()
            || (dependency.target.registry_backed && dependency.target.prepended.is_empty())))
    .then(|| tcl_compiler::registry_invocation::native_compilation_dependency(dependency))
}

/// A command binding a specialised site relies on, and the rung-2 claim that
/// must hold beside it when the binding reached a builtin through a pack
/// command's `alias_of` ([`tcl_compiler::site_claims::builtin_alias_claim`]).
#[derive(Debug, Clone)]
pub(crate) struct SiteBinding {
    pub(crate) binding: tcl_runtime_api::CommandBindingIdentity,
    pub(crate) claim: Option<tcl_runtime_api::SiteClaim>,
}

#[cfg(test)]
mod tests;
