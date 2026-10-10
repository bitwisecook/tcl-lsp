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
mod hook_operands;
mod native_compiler_pass;
mod native_failure;
pub(crate) mod native_substitution;
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

// Emission context.

/// Reentrant emitter scopes may share a range only for the same selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NativeOperationOrigin {
    start: usize,
    source: crate::command_binding::CommandAllocationSite,
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
        let key = tcl_syntax::naming::checked_namespace_path_utf8(&path)
            .ok()
            .and_then(|segments| {
                let key = if segments.is_empty() {
                    "::".to_owned()
                } else {
                    format!("::{}", segments.join("::"))
                };
                (tcl_syntax::naming::key_segments(&key) == segments).then_some(key)
            });
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
    /// Without it the two decoders hard-code *opposite* rules —
    /// `values::parse_simple_var_ref` the 9.x one and
    /// `helpers::parse_subst_template` the 8.x one — leaving the compiled-word
    /// path wrong in both directions at once. Defaults to 9.0 for
    /// the hand-built contexts in tests.
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
    /// Instruction stream (append-only during emission).
    pub instructions: Vec<Instruction>,
    /// Label name → instruction index (populated by [`place_label`]).
    pub(crate) label_positions: HashMap<String, usize>,
    /// Monotonic counter for generating unique label names.
    label_counter: u32,
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
    pub command_bindings: Option<&'r crate::command_binding::ModuleCommandMutations>,
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
    math_invocations: Vec<crate::command_binding::SourceMathInvocation>,
    /// Whole-expression entry proofs from the same exact source inventory.
    expression_preparations: Vec<crate::command_binding::SourceExpressionPreparation>,
    /// Preparation selected for the expression currently emitted.
    active_expression_preparation:
        Option<std::sync::Arc<tcl_registry::runtime_expr_validation::PreparedExpressionWitness>>,
    /// One-shot source ownership supplied by the CFG statement emitter.
    pending_math_source: Option<std::sync::Arc<crate::command_binding::ExecutedScriptSource>>,
    /// Source/base scoped to the expression currently being emitted.
    math_source: Option<std::sync::Arc<crate::command_binding::ExecutedScriptSource>>,
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
    /// on ([`crate::site_claims`]). The artefact carries these beside the
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
    invocation_tokens: Option<Box<crate::ir::CommandTokens>>,
    /// Actual authored carriers for inline script commands, shared by the module.
    source_proofs: Option<std::sync::Arc<crate::command_binding::BodySourceProofs>>,
    /// First instruction and exact source/selection premises to its range label.
    native_operation_origins: HashMap<NativeOperationOrigin, String>,
    /// Authored body coordinate, available only when the original literal survives.
    inline_body_source_base: Option<u32>,
    /// Native bytecode entry supplied by the module driver.
    native_compilation: tcl_registry::native_compilation::NativeCompilationContext,
}

impl<'r> CodegenCtx<'r> {
    /// Parse an `expr` body the way this compile reads expressions: under
    /// [`Self::expr_grammar`] for a named dialect, under the ambient runtime
    /// syntax for a dialect-less compile. The one entry point codegen uses,
    /// so its re-parsed numerals cannot diverge from the ones it emits.
    #[must_use]
    pub fn parse_compile_expr(&self, source: &str) -> crate::expr_ast::ExprNode {
        match (&self.expr_grammar, self.dialect) {
            (Some(grammar), _) => crate::expr_parser::parse_expr_with_grammar(source, grammar),
            // A context built from a profile alone (tests, and hosts that do
            // not come through `codegen_module`) parses under that profile.
            (None, Some(profile)) => {
                crate::expr_parser::parse_expr_for_profile(source, Some(profile))
            }
            // The dialect-less compile: the ambient runtime syntax.
            (None, None) => crate::expr_parser::parse_expr_for_profile(source, None),
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
            instructions: Vec::new(),
            label_positions: HashMap::new(),
            label_counter: 0,
            is_proc,
            same_frame_eval_spans: Vec::new(),
            cmd_index: 0,
            start_cmd_end_label: None,
            break_target: None,
            continue_target: None,
            inline_loop_regions: Vec::new(),
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
            current_command_namespace: tcl_runtime_api::ByteNamespacePath::root(),
            current_command_namespace_context: None,
            cmd_arg_braced: Vec::new(),
            native_hook_layout: None,
            invocation_tokens: None,
            source_proofs: None,
            native_operation_origins: HashMap::new(),
            inline_body_source_base: None,
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: if is_proc {
                    tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode
                } else {
                    tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode
                },
                loop_depth: 0,
                catch_depth: Some(0),
            },
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

    /// Whether a variable named here may be addressed as a *compiled local* of
    /// this function.
    ///
    /// A proc body's variables are its frame's locals — except inside a
    /// same-frame script body this compiler folded in
    /// ([`Self::same_frame_eval_spans`]), which C compiles as a separate unit
    /// with no access to the enclosing proc's local table. Every emitter that
    /// chooses between a slot form and its dispatched `*Stk` sibling asks this,
    /// not [`Self::is_proc`].
    #[must_use]
    pub fn compiles_locals(&self) -> bool {
        self.is_proc
            && self
                .compiled_variable_protocol
                .is_some_and(tcl_syntax::naming::NativeCompiledVariableProtocol::has_indexed_locals)
            && !self.inside_same_frame_eval()
    }

    fn source_variable_environment(&self) -> tcl_syntax::naming::NativeCompiledVariableEnvironment {
        use tcl_syntax::naming::NativeCompiledVariableEnvironment;
        if self.compiles_locals() {
            NativeCompiledVariableEnvironment::DeclareProcedure
        } else if self.borrowed_local_layout.is_some() {
            NativeCompiledVariableEnvironment::BorrowFrameSlots
        } else {
            NativeCompiledVariableEnvironment::None
        }
    }

    /// Whether the statement being emitted lies inside a folded same-frame
    /// script body. Matched by source containment, exactly as the executor
    /// matches an instruction to its [`tcl_bytecode::ErrorRegion`].
    fn inside_same_frame_eval(&self) -> bool {
        self.current_span.is_some_and(|span| {
            self.same_frame_eval_spans
                .iter()
                .any(|(start, end)| span.start() >= *start && span.end() <= *end)
        })
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

    /// Set the rooted constructed command-resolution namespace for direct
    /// codegen specialisations in this function.
    /// Set an exact native namespace without reparsing a written name.
    pub(crate) fn set_resolution_namespace_path(
        &mut self,
        namespace: tcl_runtime_api::ByteNamespacePath,
    ) {
        self.resolution_namespace = EmissionNamespace::native(namespace);
        self.current_command_namespace = self.resolution_namespace.path.clone();
        self.current_command_namespace_context = self.resolution_namespace_context();
    }

    fn resolution_namespace_context(&self) -> Option<tcl_runtime_api::CompiledNamespaceContext> {
        let context =
            crate::command_binding::SourceNamespaceKey::from_native_entry(self.native_entry?)
                .ok()?
                .to_compiled_context()?;
        (context.path() == &self.resolution_namespace.path).then_some(context)
    }

    /// Checked constructed analysis key for this exact native namespace.
    pub(crate) fn resolution_namespace(&self) -> Option<&str> {
        self.resolution_namespace.key.as_deref()
    }

    /// Construct the complete source-site identity for a direct codegen
    /// specialisation.
    pub(crate) fn command_binding_identity(
        &self,
        name: impl Into<String>,
        identity: impl Into<String>,
    ) -> tcl_runtime_api::CommandBindingIdentity {
        let name = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| {
                crate::registry_invocation::static_command_word(
                    tokens,
                    self.escapes,
                    self.word_rules,
                )
            })
            .unwrap_or_else(|| name.into());
        let identity = identity.into();
        let namespace_context = self
            .invocation_tokens
            .as_deref()
            .and_then(crate::registry_invocation::compiled_namespace_context);
        let namespace = self.resolution_namespace().unwrap_or_else(|| {
            assert!(
                namespace_context.is_some(),
                "native specialisation requires an exact namespace context"
            );
            ""
        });
        let binding = tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
            namespace,
            name,
            identity.strip_prefix("::").unwrap_or(&identity),
        )
        .with_namespace_context(namespace_context);
        if let Some(tokens) = self.invocation_tokens.as_deref()
            && tokens
                .source_binding
                .as_ref()
                .and_then(|proof| proof.admitted_inline_invocation())
                .is_some_and(|proof| {
                    proof
                        .target
                        .command
                        .strip_prefix("::")
                        .unwrap_or(&proof.target.command)
                        == binding
                            .identity
                            .strip_prefix("::")
                            .unwrap_or(&binding.identity)
                })
        {
            return binding.with_guard(crate::registry_invocation::command_binding_guard(tokens));
        }
        binding
    }

    #[track_caller]
    fn refuse_native_dependency(&mut self) {
        if std::env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
            eprintln!(
                "NATIVE_CODEGEN_REFUSAL caller={} compilation={:?} invocation={:?} span={:?} source={:?}",
                std::panic::Location::caller(),
                self.native_compilation,
                self.invocation_dialect,
                self.current_span,
                self.exact_command_source
                    .as_ref()
                    .map(tcl_lexer::SourceImage::bytes),
            );
        }
        self.native_dependency_refusal = true;
    }

    /// Scope exact source proof to one emitter call; nested emitters restore it.
    fn with_invocation_tokens<T>(
        &mut self,
        tokens: Option<&crate::ir::CommandTokens>,
        emit: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous =
            std::mem::replace(&mut self.invocation_tokens, tokens.cloned().map(Box::new));
        let previous_layout = self.native_hook_layout.take();
        let operation = if self.plain_command_dispatch {
            None
        } else if let Some(tokens) = self
            .invocation_tokens
            .as_deref()
            .filter(|tokens| tokens.synthetic.is_none())
        {
            if let Ok(plan) = crate::registry_invocation::native_operation_selection_plan(
                tokens,
                self.escapes,
                self.word_rules,
            ) {
                plan
            } else {
                self.refuse_native_dependency();
                None
            }
        } else {
            None
        };
        let operation_start = self.instructions.len();
        if !self.plain_command_dispatch
            && let Some(tokens) = self.invocation_tokens.as_deref()
        {
            let refused = tokens.source_binding.as_ref().is_some_and(|binding| {
                matches!(
                    binding.native_compilation_admission_selection(),
                    tcl_registry::native_compilation::NativeCompilationSelection::Unknown
                ) && binding.original_named_compiler_admission(tokens).is_none()
            });
            // Auxiliary compiler lookups are guard dependencies, not source
            // commands. Recording one must not switch the enclosing command's
            // replay namespace to a private implementation's namespace.
            self.command_binding_requirements
                .extend(crate::registry_invocation::native_implementation_dependencies(tokens));
            if refused {
                if std::env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                    eprintln!(
                        "NATIVE_CODEGEN_ADMISSION argv={:?} selection={:?} original_named={}",
                        tokens.argv_texts,
                        tokens
                            .source_binding
                            .as_ref()
                            .map(crate::command_binding::SourceInvocationBinding::native_compilation_admission_selection),
                        tokens.source_binding.as_ref().is_some_and(|binding| binding
                            .original_named_compiler_admission(tokens)
                            .is_some()),
                    );
                }
                self.refuse_native_dependency();
            }
        }
        let result = emit(self);
        if let Some(operation) = operation {
            self.retain_native_operation_selection(operation_start, operation);
        }
        self.invocation_tokens = previous;
        self.native_hook_layout = previous_layout;
        result
    }

    /// Retain selection at the first operation instruction, before its argv.
    /// Nested operations can share that instruction but have distinct ranges.
    fn retain_native_operation_selection(
        &mut self,
        start: usize,
        plan: crate::registry_invocation::NativeOperationSelectionPlan,
    ) {
        if start == self.instructions.len() {
            return;
        }
        // The function inventory summarizes every retained operation premise,
        // including constant-result paths that never call a typed opcode hook.
        self.command_binding_requirements
            .extend(plan.requirements.iter().cloned());
        let end = self.fresh_label("native_operation_end");
        self.place_label(&end);
        let previous = self.native_operation_origins.insert(
            NativeOperationOrigin {
                start,
                source: plan.compilation_site,
                operation: plan.operation,
                guard: plan.guard,
                compiler_prerequisite: plan.compiler_prerequisite.clone(),
                requirements: plan.requirements.clone(),
            },
            end.clone(),
        );
        let site = tcl_bytecode::NativeOperationSelectionSite {
            compiler_prerequisite: plan.compiler_prerequisite,
            requirements: plan.requirements,
            guard: plan.guard,
            end,
            source: tcl_lexer::SourceImage::from_bytes(
                plan.source.into_bytes(),
                self.source.channel(),
            ),
            span: plan.span,
            namespace: plan.namespace_context.as_ref().map_or_else(
                || namespace_path_from_constructed_key(&plan.namespace),
                |context| context.path().clone(),
            ),
            namespace_context: plan.namespace_context,
        };
        let first = &mut self.instructions[start];
        // Reentrant bridges can scope the same invocation twice. Keep the
        // enclosing range rather than validating its shorter duplicate.
        if let Some(previous) = previous {
            first
                .native_operation_selections
                .retain(|existing| existing.end != previous);
        }
        first.native_operation_selections.insert(0, site);
        first.no_fold = true;
    }

    /// Native engine snapshot for a reached hook, independent of its catalogue.
    fn native_hook_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        if let Some(binding) = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.source_binding.as_ref())
        {
            return binding.native_compiler_dialect();
        }
        self.invocation_dialect.or_else(|| {
            Some(crate::environment_ingress::authoring_invocation_dialect(
                self.registry,
                self.dialect,
                tcl_lexer::LexerConfig::for_profile(self.dialect),
            ))
        })
    }

    /// Select backend metadata under the actual reached native dialect. A
    /// permissive or assisting catalogue cannot replace this engine snapshot.
    fn invocation_surface_query(&self) -> Option<tcl_dialect::model::SurfaceQuery<'static>> {
        match self.native_hook_dialect() {
            Some(dialect) => dialect.authoring_query(),
            None => self.registry.own_surface_query(),
        }
    }

    /// A retained uncertain execution cannot acquire a hook from its spelling.
    fn invocation_specialisation_proved(&self) -> bool {
        (self.resolution_namespace().is_some()
            || self
                .invocation_tokens
                .as_deref()
                .and_then(crate::registry_invocation::compiled_namespace_context)
                .is_some())
            && self.invocation_tokens.as_deref().is_none_or(|tokens| {
                tokens.source_binding.as_ref().is_none_or(|binding| {
                    crate::registry_invocation::proved_native_admitted_inline_operation(tokens)
                        .is_some()
                        && binding.native_inline_rejection
                            == crate::command_binding::NativeInlineRejection::None
                })
            })
    }

    /// Set the module source text (see [`Self::source`]) so emitted instructions
    /// carry their command's surface text for `errorInfo`.
    pub fn set_source(&mut self, source: &str) {
        self.exact_command_source = None;
        self.source = tcl_lexer::SourceImage::document(source);
        self.line_index = (!source.is_empty()).then(|| tcl_lexer::LineIndex::new(source));
    }

    /// Retain original native source bytes and channel before emitting spans.
    pub fn set_source_image(&mut self, source: tcl_lexer::SourceImage) {
        self.exact_command_source = None;
        self.line_index =
            (!source.is_empty()).then(|| tcl_lexer::LineIndex::from_bytes(source.bytes()));
        self.source = source;
    }

    /// Borrow the original source image for byte word and nested-body emission.
    #[must_use]
    pub fn source_image(&self) -> &tcl_lexer::SourceImage {
        &self.source
    }

    /// Check an unchanged Unicode advisory view without substituting a buffer.
    pub(crate) fn source_unicode(&self) -> Result<&str, std::str::Utf8Error> {
        self.source.try_text()
    }

    /// Install the module source and its already-built line index. The module
    /// emitter uses this path so procedure contexts share both allocations.
    pub(super) fn set_indexed_source(
        &mut self,
        source: tcl_lexer::SourceImage,
        line_index: tcl_lexer::LineIndex,
    ) {
        self.exact_command_source = None;
        self.line_index = (!source.is_empty()).then_some(line_index);
        self.source = source;
    }

    /// Select the executable Tcl command whose source metadata subsequent
    /// instructions inherit.
    ///
    /// A source span and its command/control ownership are one piece of
    /// emitter state: updating only the span would retain the right line while
    /// silently dropping the command text needed by `errorInfo` and runtime
    /// command-table revalidation. Keep that invariant behind this method
    /// rather than exposing the two fields independently.
    pub fn set_command_source_span(&mut self, span: impl Into<Option<Span>>) {
        self.exact_command_source = None;
        self.current_span = span.into();
        self.current_span_is_command = true;
        self.current_command_namespace = self.resolution_namespace.path.clone();
        self.current_command_namespace_context = self.resolution_namespace_context();
    }

    /// Select compiler control which has a useful diagnostic span but is not
    /// itself a replayable Tcl command (for example, a CFG branch condition).
    pub(crate) fn set_control_source_span(&mut self, span: Option<Span>) {
        self.exact_command_source = None;
        self.current_span = span;
        self.current_span_is_command = false;
        self.current_command_namespace = tcl_runtime_api::ByteNamespacePath::root();
        self.current_command_namespace_context = None;
    }

    /// Whether `name` is free of whole-unit mutation, for transforms that have
    /// no independently replayable command boundary.
    ///
    /// Answers from the whole-module [`Self::command_bindings`] summary — a
    /// flow-**insensitive** scan on purpose: a `rename` buried in a proc body
    /// can fire before a call earlier in the file runs, so "no rename seen so
    /// far" is not a sound answer. Without a module view the answer is
    /// `true`; see [`Self::command_bindings`].
    #[must_use]
    pub fn trusts_builtin(&self, name: &str) -> bool {
        self.command_bindings.is_none_or(|m| m.trusts(name))
    }

    /// Record one source binding relied on by specialised emission.
    pub fn require_command_binding(&mut self, binding: &tcl_runtime_api::CommandBindingIdentity) {
        self.command_binding_requirements.insert(binding.clone());
        self.current_command_namespace = namespace_path_for_binding(binding);
        self.current_command_namespace_context
            .clone_from(&binding.namespace_context);
    }

    /// Unit cache validation can retain only the compiler world present at
    /// this artifact's original entry. Later child worlds keep their own
    /// instruction selection and validation boundary.
    fn retain_entry_named_compiler_prerequisite(
        &mut self,
        required: &std::sync::Arc<
            tcl_runtime_api::native_compilation::NativeEnsembleCompilerPrerequisite,
        >,
    ) {
        let Some(entry) = self.native_entry else {
            return;
        };
        if entry.interpreter != required.interpreter {
            return;
        }
        if required.matches_registration_with(|namespace, word| {
            entry.lookup_command_bytes(namespace, word.as_bytes()).map(
                Option::<&tcl_runtime_api::native_compilation::NativeCompilationBinding>::cloned,
            )
        }) != Ok(true)
        {
            return;
        }
        // Cache validity is checked at this same immutable entry; the
        // instruction retains its independent before-arguments selection.
        let mut entry_required = required.as_ref().clone();
        entry_required.guard = tcl_runtime_api::CommandBindingGuard::ChunkEntry;
        self.retain_native_compilation_dependency(
            crate::registry_invocation::NativeCompilationDependency::Compiler(
                tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::from_command_registration(
                    std::sync::Arc::new(entry_required),
                ),
            ),
        );
    }

    fn retain_native_compilation_dependency(
        &mut self,
        dependency: crate::registry_invocation::NativeCompilationDependency,
    ) {
        match dependency {
            crate::registry_invocation::NativeCompilationDependency::Compiler(required) => {
                if !self.native_compiler_prerequisites.contains(&required) {
                    self.native_compiler_prerequisites.push(required);
                }
            }
            crate::registry_invocation::NativeCompilationDependency::Implementation(binding) => {
                self.command_binding_requirements.insert(binding);
            }
        }
    }

    /// Atomically retain the exact dependencies consumed by successful folds.
    /// Implicit lookup guards must not overwrite the enclosing replay namespace.
    pub(crate) fn retain_math_invocations(
        &mut self,
        proofs: &[crate::command_binding::SourceMathInvocation],
    ) -> bool {
        use crate::math_function_binding::{NativeMathFoldDependency, native_fold_dependency};
        let Some(dependencies) = proofs
            .iter()
            .map(native_fold_dependency)
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        let mut table = self.math_table_prerequisite.as_ref();
        for dependency in &dependencies {
            if let NativeMathFoldDependency::Fixed(required) = dependency {
                if table.is_some_and(|existing| existing != required) {
                    return false;
                }
                table = Some(required);
            }
        }
        for dependency in dependencies {
            match dependency {
                NativeMathFoldDependency::Command(binding) => {
                    self.command_binding_requirements.insert(binding);
                }
                NativeMathFoldDependency::Fixed(required) => {
                    self.math_table_prerequisite = Some(required);
                }
            }
        }
        true
    }

    /// Retain actual preparation owners even when no function call was reached.
    pub(crate) fn retain_expression_preparations(
        &mut self,
        proofs: &[crate::command_binding::SourceExpressionPreparation],
    ) -> bool {
        if proofs.is_empty() {
            return true;
        }
        let Some(mut context) = self.fold_policy().preparation_context() else {
            return false;
        };
        context.lexer_grammar = self.lexer_config().grammar_over(context.lexer_grammar);
        let mut table = self.math_table_prerequisite.as_ref();
        for proof in proofs {
            if proof.witness.context() != &context
                || proof.witness.source().as_bytes() != proof.source.text.bytes()
                || !proof.has_closed_script_compilation()
            {
                return false;
            }
            if let Some(required) = proof.witness.fixed_functions() {
                if table.is_some_and(|existing| existing != required) {
                    return false;
                }
                table = Some(required);
            }
        }
        let Some(dependencies) = proofs
            .iter()
            .flat_map(crate::command_binding::SourceExpressionPreparation::script_compilation_dependencies)
            .map(expression_script_dependency)
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        if let Some(required) = table {
            self.math_table_prerequisite = Some(required.clone());
        }
        for dependency in dependencies {
            self.retain_native_compilation_dependency(dependency);
        }
        true
    }

    /// Record one spec-pack claim a specialised operation rests on.
    pub fn require_site_claim(&mut self, claim: &tcl_runtime_api::SiteClaim) {
        self.site_claim_requirements.insert(claim.clone());
    }

    /// Record a site binding: the binding, and its claim when it has one.
    pub(crate) fn require_site_binding(&mut self, site: &SiteBinding) {
        self.require_command_binding(&site.binding);
        if let Some(claim) = &site.claim {
            self.require_site_claim(claim);
        }
    }

    /// The binding a call of `cmd` specialised on `stamp` relies on: the
    /// identity the registry answers for it
    /// ([`ResolvedCall::stamp_identity`](tcl_registry::registry::ResolvedCall::stamp_identity)),
    /// and the rung-2 claim when that identity reached a builtin through a
    /// pack command's `alias_of`.
    pub(crate) fn stamped_binding(
        &self,
        cmd: &str,
        resolved: &tcl_registry::registry::ResolvedCall<'_>,
        stamp: tcl_registry::codegen_stamp::CodegenStamp,
    ) -> SiteBinding {
        let binding =
            self.command_binding_identity(cmd, resolved.stamp_identity(self.registry, stamp));
        let claim = crate::site_claims::builtin_alias_claim(self.registry, resolved, &binding);
        SiteBinding { binding, claim }
    }

    /// Resolve a registry-described lowering specialisation for an inline
    /// emitter which operates below the normal IR lowering boundary.
    ///
    /// The returned identity is the dependency the caller must retain if it
    /// consumes the command head. Keeping the proof and its identity together
    /// prevents ad-hoc nested-body emitters from specialising on a raw command
    /// name without participating in runtime command-table revalidation.
    pub(crate) fn inline_lowering_hook(
        &self,
        command: &str,
        args: &[&str],
    ) -> Option<(
        tcl_registry::hooks::LoweringHookId,
        tcl_runtime_api::CommandBindingIdentity,
    )> {
        if self.plain_command_dispatch || !self.trusts_builtin(command) {
            return None;
        }
        let resolved =
            self.registry
                .resolve_call(command, args, self.invocation_surface_query())?;
        if resolved.spec.name != command {
            return None;
        }
        Some((
            resolved.lowering_hook?,
            self.command_binding_identity(command, resolved.spec.name),
        ))
    }

    /// Clear source ownership before emitting compiler-generated block
    /// machinery. A later `START_CMD` remains a non-boundary until its exact
    /// owning Tcl command is supplied explicitly.
    pub(crate) fn clear_source_site(&mut self) {
        self.set_control_source_span(None);
    }

    /// Select the explicit structured-command owner for a synthetic runtime
    /// boundary. No site means the marker is compiler control only and must not
    /// be used for plain-dispatch replay.
    pub(crate) fn set_command_boundary_site(
        &mut self,
        site: Option<&crate::ir::CommandBindingSite>,
    ) {
        self.clear_source_site();
        if let Some(site) = site {
            self.set_command_source_span(site.span);
            self.current_command_namespace = namespace_path_for_binding(&site.binding);
            self.current_command_namespace_context
                .clone_from(&site.binding.namespace_context);
            if !self.plain_command_dispatch
                && let Some(binding) =
                    crate::registry_invocation::native_site_binding_requirement(site)
            {
                self.require_command_binding(binding);
            }
        }
    }

    /// Restamp an already-emitted synthetic `START_CMD` with its explicit
    /// structured owner without changing the source metadata of the wrapped
    /// clause instructions.
    pub(crate) fn stamp_command_boundary(
        &mut self,
        instruction: usize,
        site: Option<&crate::ir::CommandBindingSite>,
    ) {
        let (span, text, line) =
            site.map_or((None, tcl_lexer::SourceImage::default(), 0), |site| {
                if !self.plain_command_dispatch
                    && let Some(binding) =
                        crate::registry_invocation::native_site_binding_requirement(site)
                {
                    self.require_command_binding(binding);
                }
                (
                    Some(site.span),
                    self.source_text(site.span),
                    self.source_line(site.span),
                )
            });
        if let Some(instr) = self.instructions.get_mut(instruction) {
            instr.source_span = span;
            instr.source_cmd_text = text;
            instr.source_line = line;
            instr.source_command_namespace = site
                .map(|site| namespace_path_for_binding(&site.binding))
                .unwrap_or_default();
            instr.source_command_namespace_context =
                site.and_then(|site| site.binding.namespace_context.clone());
            instr.source_command_boundary = site.is_some().into();
        }
    }

    /// Scope one inline command's diagnostic and replay source together.
    /// An absent module coordinate stays absent; its exact command bytes still
    /// belong to every emitted instruction. Nested commands restore this owner.
    fn with_inline_command_source<T>(
        &mut self,
        span: Option<Span>,
        text: &str,
        emit: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous_span = self.current_span;
        let previous_command = self.current_span_is_command;
        let previous_exact = self.exact_command_source.take();
        let previous_namespace = self.current_command_namespace.clone();
        let previous_namespace_context = self.current_command_namespace_context.clone();
        self.set_command_source_span(span);
        self.exact_command_source = Some(tcl_lexer::SourceImage::from_bytes(
            text.as_bytes(),
            self.source.channel(),
        ));
        let result = emit(self);
        self.current_span = previous_span;
        self.current_span_is_command = previous_command;
        self.exact_command_source = previous_exact;
        self.current_command_namespace = previous_namespace;
        self.current_command_namespace_context = previous_namespace_context;
        result
    }

    /// Give every synthetic `START_CMD` just emitted for an inline body command
    /// its exact replay text, without overwriting a more deeply nested command
    /// boundary that was already restamped while those instructions were
    /// produced.
    ///
    /// The scoped command source supplies its module span and diagnostic text.
    /// This helper marks only its own replay points; deeper command owners keep
    /// their separate boundaries and source coordinates.
    pub(crate) fn restamp_emitted_inline_command_boundaries(
        &mut self,
        start: usize,
        text: &str,
        line: u32,
    ) {
        let enclosing_span = self.current_span;
        let enclosing_text = self.command_span_text();
        let enclosing_line = self.span_line();
        for instruction in self.instructions.iter_mut().skip(start) {
            if instruction.op == Op::START_CMD
                && instruction.source_span == enclosing_span
                && instruction.source_cmd_text == enclosing_text
                && instruction.source_line == enclosing_line
            {
                instruction.source_cmd_text =
                    tcl_lexer::SourceImage::from_bytes(text.as_bytes(), self.source.channel());
                instruction.source_line = line;
                // This START_CMD is a nested inline replay point, not the
                // boundary of the enclosing IR/source command used to locate
                // that outer command's continuation.
                instruction.source_command_boundary = SourceCommandBoundary::InlineReplay;
            }
        }
    }

    /// Begin an executable boundary for an inline command whose specialised
    /// instructions consume the invocation completely.
    ///
    /// Most inline command substitutions pass through `emit_cmd_word`, which
    /// already retains a nested `START_CMD`. A compile-time constant fold is
    /// different: it replaces the entire invocation with a literal push, so
    /// without this boundary a command-table mutation in an earlier argument
    /// of the same active command can leave the later fold executing stale
    /// semantics. The explicit source text lets the VM replay ordinary Tcl
    /// dispatch and resume after the stale folded instructions.
    ///
    /// This is a nested boundary, not a new source-command owner. Its absolute
    /// span/line continue to identify the enclosing command for diagnostics,
    /// while `source_cmd_text` is the exact bracket interior to replay.
    pub(crate) fn begin_consumed_inline_command(&mut self, text: &str) -> String {
        let end = self.fresh_label("inline_cmd_end");
        let start = self.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(end.clone()), Operand::Imm(1)],
            "",
        );
        if let Some(instruction) = self.instructions.get_mut(start) {
            instruction.source_cmd_text =
                tcl_lexer::SourceImage::from_bytes(text.as_bytes(), self.source.channel());
            instruction.source_command_boundary = SourceCommandBoundary::InlineReplay;
        }
        self.cmd_index += 1;
        end
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
    fn span_text(&self) -> tcl_lexer::SourceImage {
        match self.current_span {
            Some(sp) => {
                let (s, mut e) = (sp.start() as usize, sp.end() as usize);
                if self.source.bytes().get(e) == Some(&b'"') {
                    e += 1;
                }
                tcl_lexer::SourceImage::from_bytes(
                    self.source.get(s..e).unwrap_or_default(),
                    self.source.channel(),
                )
            }
            None => tcl_lexer::SourceImage::default(),
        }
    }

    /// The whole Tcl command represented by [`Self::current_span`], or empty
    /// when the span belongs only to compiler-generated control machinery.
    fn command_span_text(&self) -> tcl_lexer::SourceImage {
        if self.current_span_is_command {
            self.exact_command_source
                .clone()
                .unwrap_or_else(|| self.span_text())
        } else {
            tcl_lexer::SourceImage::default()
        }
    }

    /// The surface text of an explicit `span` within the module source — for
    /// inline-body error regions, whose enclosing command's span differs from the
    /// per-instruction `current_span`. Empty when no source was supplied.
    pub(crate) fn source_text(&self, span: Span) -> tcl_lexer::SourceImage {
        let (s, e) = (span.start() as usize, span.end() as usize);
        tcl_lexer::SourceImage::from_bytes(
            self.source.get(s..e).unwrap_or_default(),
            self.source.channel(),
        )
    }

    /// The 1-based source line of an explicit `span`'s start (its first byte).
    /// `0` when no source was supplied (the span can't be located).
    pub(crate) fn source_line(&self, span: Span) -> u32 {
        if self.source.is_empty() {
            return 0;
        }
        self.line_at(span.start())
    }

    /// The indexed 1-based line containing `offset`.
    fn line_at(&self, offset: u32) -> u32 {
        let Some(line_index) = &self.line_index else {
            return 1;
        };
        if usize::try_from(offset).map_or(true, |offset| offset > self.source.len()) {
            return 1;
        }
        line_index.position_at(offset).line.saturating_add(1)
    }

    /// The 1-based line of `current_span` within the module source — the line a
    /// command reports in `errorInfo` (`(procedure … line N)` / `("while" body
    /// line N)`). `0` when no span is available. A hand-built context with a
    /// span but no source falls back to line one.
    fn span_line(&self) -> u32 {
        match self.current_span {
            Some(sp) => self.line_at(sp.start()),
            None => 0,
        }
    }

    /// Append an instruction, returning its index in the stream.
    pub fn emit(&mut self, op: Op, operands: Vec<Operand>) -> usize {
        let idx = self.instructions.len();
        let mut instr = Instruction::new(op, operands);
        instr.source_span = self.current_span;
        instr.source_cmd_text = self.command_span_text();
        instr.source_line = self.span_line();
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
        instr.source_cmd_text = self.command_span_text();
        instr.source_line = self.span_line();
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
    dependency: &crate::command_binding::SourceNativeCompilationDependency,
) -> Option<crate::registry_invocation::NativeCompilationDependency> {
    (dependency.guard == tcl_registry::native_compilation::NativeCompilationGuard::ChunkEntry
        && (dependency.compiler_prerequisite.is_some()
            || (dependency.target.registry_backed && dependency.target.prepended.is_empty())))
    .then(|| crate::registry_invocation::native_compilation_dependency(dependency))
}

/// A command binding a specialised site relies on, and the rung-2 claim that
/// must hold beside it when the binding reached a builtin through a pack
/// command's `alias_of` ([`crate::site_claims::builtin_alias_claim`]).
#[derive(Debug, Clone)]
pub(crate) struct SiteBinding {
    pub(crate) binding: tcl_runtime_api::CommandBindingIdentity,
    pub(crate) claim: Option<tcl_runtime_api::SiteClaim>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_preparation(generation: u64) -> crate::command_binding::SourceExpressionPreparation {
        use std::sync::Arc;
        use tcl_runtime_api::native_compilation::{
            NativeInterpreterIdentity, NativeMathFunctionBinding, NativeMathFunctionPrerequisite,
            NativeMathFunctionTable,
        };
        let source = "0 && abs(1)";
        let origin = Arc::new(crate::command_binding::SourceOriginId::authored(
            &Arc::from(source),
        ));
        let source = Arc::new(
            crate::command_binding::ExecutedScriptSource::contiguous(
                Arc::clone(&origin),
                source,
                0,
            )
            .unwrap(),
        );
        let table = NativeMathFunctionPrerequisite {
            interpreter: NativeInterpreterIdentity {
                owner: 17,
                interpreter: 3,
            },
            table: NativeMathFunctionTable {
                closed: true,
                generation,
                functions: vec![NativeMathFunctionBinding {
                    name: "abs".into(),
                    token: 1,
                    implementation_generation: 1,
                    registry_identity: Some("abs".into()),
                    arity: Some(1),
                }],
            },
        };
        let profile = tcl_registry::model::ingress::static_context_for("jim")
            .commands()
            .profile()
            .unwrap();
        let context = tcl_registry::InvocationDialect::of_profile(profile)
            .expression_parse_context(Some(profile));
        let tcl_registry::runtime_expr_validation::ExpressionPreparationProof::Prepared(witness) =
            tcl_registry::runtime_expr_validation::prepare_expression_witness(
                source.try_text().unwrap(),
                &context,
                Some(&table),
            )
        else {
            panic!("actual closed fixed table must prepare expression");
        };
        crate::command_binding::SourceExpressionPreparation {
            namespace_key: crate::command_binding::SourceNamespaceKey::authored("::"),
            invocation: crate::command_binding::CommandAllocationSite {
                source: origin,
                offset: 0,
            },
            source,
            witness: Arc::from(witness),
            script_compilation: None,
            executed_expression: None,
        }
    }

    #[test]
    fn preparation_guard_survives_without_reached_math_calls() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut ctx = CodegenCtx::new(false, &[], registry);
        ctx.dialect = registry.profile();
        ctx.numbers = registry.numbers();
        let proof = fixed_preparation(4);
        assert!(ctx.retain_expression_preparations(std::slice::from_ref(&proof)));
        assert_eq!(
            ctx.math_table_prerequisite.as_ref(),
            proof.witness.fixed_functions()
        );
        assert!(ctx.command_binding_requirements.is_empty());
        let incompatible = fixed_preparation(5);
        assert!(!ctx.retain_expression_preparations(&[incompatible]));
        assert_eq!(
            ctx.math_table_prerequisite.as_ref(),
            proof.witness.fixed_functions()
        );
    }

    #[test]
    fn preparation_guard_declines_a_different_native_engine() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut ctx = CodegenCtx::new(false, &[], registry);
        ctx.dialect = registry.profile();
        ctx.numbers = registry.numbers();
        assert!(!ctx.retain_expression_preparations(&[fixed_preparation(4)]));
        assert!(ctx.math_table_prerequisite.is_none());
        assert!(ctx.command_binding_requirements.is_empty());
    }

    #[test]
    fn script_preparation_retains_chunk_entry_guards_atomically() {
        use crate::command_binding::{ExecutedScriptSource, SourceCommandBindings};
        use tcl_runtime_api::CommandBindingGuard;

        let registry = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
        let source = "set x 4; expr {[set x] + 0}";
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry,
            crate::command_binding::SourceAnalysisOptions {
                invocation_dialect: registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let script = ExecutedScriptSource::contiguous(
            std::sync::Arc::clone(bindings.source_origin().unwrap()),
            source,
            0,
        )
        .unwrap();
        let proofs = bindings.expression_preparations_for_script(&script);
        assert_eq!(proofs.len(), 1);
        assert_ne!(proofs[0].witness.compiled_scripts(), []);
        let mut ctx = CodegenCtx::new(false, &[], registry);
        ctx.dialect = registry.profile();
        ctx.numbers = registry.numbers();
        assert!(ctx.retain_expression_preparations(&proofs));
        assert!(ctx.command_binding_requirements.iter().any(|binding| {
            binding.identity == "set" && binding.guard == CommandBindingGuard::ChunkEntry
        }));
        let retained = ctx.command_binding_requirements.clone();
        let mut missing = proofs[0].clone();
        missing.script_compilation = None;
        assert!(!ctx.retain_expression_preparations(&[missing]));
        assert_eq!(ctx.command_binding_requirements, retained);
    }

    #[test]
    fn native_source_boundaries_keep_opaque_bytes_and_constructed_namespace() {
        let registry = CommandRegistry::build_default();
        let namespace = tcl_runtime_api::ByteNamespacePath::from_segments([
            b"\xff".as_slice(),
            b":".as_slice(),
        ]);
        let source = tcl_lexer::SourceImage::native(b"set \xfe V".as_slice());
        let mut context = CodegenCtx::with_native_parameters(
            true,
            &[tcl_runtime_api::NameBytes::from(b"\xfe")],
            &registry,
        );
        context.set_source_image(source.clone());
        context.set_resolution_namespace_path(namespace.clone());
        assert!(context.resolution_namespace().is_none());
        assert_eq!(context.lvt.entries()[0].as_bytes(), b"\xfe");
        context.set_command_source_span(Span::new(0, 7));
        let index = context.emit(Op::NOP, Vec::new());
        assert_eq!(context.instructions[index].source_cmd_text, source);
        assert_eq!(
            context.instructions[index].source_command_namespace,
            namespace
        );
    }

    #[test]
    fn source_lines_are_indexed_at_byte_boundaries() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.set_source("alpha\nβeta\n\ngamma");

        for (offset, expected) in [(0, 1), (5, 1), (6, 2), (11, 2), (12, 3), (13, 4)] {
            assert_eq!(ctx.source_line(Span::empty(offset)), expected, "{offset}");
        }

        assert_eq!(ctx.source_line(Span::empty(99)), 1);

        ctx.current_span = Some(Span::empty(13));
        let instruction = ctx.emit(Op::NOP, Vec::new());
        assert_eq!(ctx.instructions[instruction].source_line, 4);

        let mut empty = CodegenCtx::new(false, &[], &registry);
        assert_eq!(empty.source_line(Span::empty(0)), 0);
        empty.current_span = Some(Span::empty(0));
        let instruction = empty.emit(Op::NOP, Vec::new());
        assert_eq!(empty.instructions[instruction].source_line, 1);

        let source = tcl_lexer::SourceImage::document("shared\nsource");
        let source_clone = source.shared_bytes();
        let line_index = tcl_lexer::LineIndex::from_bytes(source.bytes());
        let line_index_clone = line_index.clone();
        let mut shared = CodegenCtx::new(false, &[], &registry);
        shared.set_indexed_source(source, line_index);
        assert!(std::sync::Arc::ptr_eq(
            &shared.source.shared_bytes(),
            &source_clone
        ));
        assert!(
            shared
                .line_index
                .as_ref()
                .is_some_and(|index| index.shares_storage_with(&line_index_clone)),
        );
    }
}
