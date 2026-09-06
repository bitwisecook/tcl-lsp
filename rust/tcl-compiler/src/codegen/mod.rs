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
    /// The backslash-escape grammar of the release being compiled *for*.
    ///
    /// Threaded from `IrModule::dialect` beside [`Self::numbers`], so a literal
    /// word's escapes are decoded the way the target release reads them —
    /// `\x4142` is `B` when compiling for 8.5 and `A42` from 8.6 (issue #1479).
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
    /// Before this was threaded, the two decoders hard-coded *opposite* rules —
    /// `values::parse_simple_var_ref` the 9.x one and
    /// `helpers::parse_subst_template` the 8.x one — so the compiled-word path
    /// was wrong in both directions at once (issue #1568). Defaults to 9.0 for
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
    /// operators the target release's `expr` actually has (issue #1435).
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
    resolution_namespace: String,
    /// Whole-module command-mutation summary — which command *names* may stop
    /// denoting their original builtin anywhere in this compilation unit
    /// (issue #1585).
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
    /// seam, which is handed one CFG and never sees the module. Those keep
    /// the historical trust-everything behaviour;
    /// [`codegen_module`](crate::codegen::codegen_module), the whole-unit
    /// entry point every production pipeline uses, always supplies the scan.
    pub command_bindings: Option<&'r crate::command_binding::ModuleCommandMutations>,
    /// Registry identities assumed by specialised operations emitted into this
    /// function. The bytecode artifact carries these to the runtime.
    pub command_binding_requirements: BTreeSet<tcl_runtime_api::CommandBindingIdentity>,
    /// Suppress registry codegen hooks as well as lowering hooks.
    pub plain_command_dispatch: bool,
    /// The module's original source text, indexed by `current_span` to recover
    /// each command's surface text for `errorInfo` (`while executing "…"`).
    /// Empty when the caller did not supply it (hand-built test contexts).
    source: std::rc::Rc<str>,
    /// The lexer-owned line index for [`Self::source`]. Production module
    /// emission shares one Arc-backed index across every function context.
    line_index: Option<tcl_lexer::LineIndex>,
    /// Whether [`Self::current_span`] denotes an executable Tcl command rather
    /// than CFG control machinery such as a condition or body-edge jump.
    /// Both need source ranges for diagnostics, but only a command supplies a
    /// safe whole script for runtime command-table revalidation.
    current_span_is_command: bool,
    /// Unrooted constructed namespace paired with the current executable
    /// command source site. This follows explicit IR binding sites across
    /// inlining rather than inheriting the surrounding function's namespace.
    current_command_namespace: String,
    /// Per-argument "is a braced (`{…}`) word" flags for the command currently
    /// dispatching to a codegen hook (`try_bytecoded`). Set by [`Self::emit_call`]
    /// from the command's tokens and consulted by [`Self::emit_word_arg`] so a
    /// hook collapses a non-braced literal's backslashes exactly like the generic
    /// per-word path. Empty for hand-built test contexts (treated as non-braced).
    cmd_arg_braced: Vec<bool>,
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
        Self {
            numbers: tcl_dialect::NumberSyntax::default(),
            escapes: tcl_dialect::EscapeSyntax::default(),
            word_rules: tcl_syntax::word_rules::WordValueRules::default(),
            braced_var: tcl_dialect::BracedVarStyle::default(),
            dialect: None,
            expr_grammar: None,
            literals: LiteralTable::new(),
            lvt: LocalVarTable::new(params),
            instructions: Vec::new(),
            label_positions: HashMap::new(),
            label_counter: 0,
            is_proc,
            same_frame_eval_spans: Vec::new(),
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
            resolution_namespace: "::".to_owned(),
            command_bindings: None,
            command_binding_requirements: BTreeSet::new(),
            plain_command_dispatch: false,
            source: "".into(),
            line_index: None,
            current_span_is_command: false,
            current_command_namespace: String::new(),
            cmd_arg_braced: Vec::new(),
        }
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
        self.is_proc && !self.inside_same_frame_eval()
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
        tcl_lexer::LexerConfig::for_profile(self.registry.profile())
    }

    /// Set the rooted constructed command-resolution namespace for direct
    /// codegen specialisations in this function.
    pub(crate) fn set_resolution_namespace(&mut self, namespace: &str) {
        namespace.clone_into(&mut self.resolution_namespace);
        Self::unrooted_namespace(namespace).clone_into(&mut self.current_command_namespace);
    }

    fn unrooted_namespace(namespace: &str) -> &str {
        namespace.strip_prefix("::").unwrap_or(namespace)
    }

    /// Rooted constructed command-resolution namespace for direct codegen
    /// specialisations in this function.
    pub(crate) fn resolution_namespace(&self) -> &str {
        &self.resolution_namespace
    }

    /// Construct the complete source-site identity for a direct codegen
    /// specialisation.
    pub(crate) fn command_binding_identity(
        &self,
        name: impl Into<String>,
        identity: impl Into<String>,
    ) -> tcl_runtime_api::CommandBindingIdentity {
        tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
            &self.resolution_namespace,
            name,
            identity,
        )
    }

    /// Set the module source text (see [`Self::source`]) so emitted instructions
    /// carry their command's surface text for `errorInfo`.
    pub fn set_source(&mut self, source: &str) {
        self.source = source.into();
        self.line_index = (!source.is_empty()).then(|| tcl_lexer::LineIndex::new(source));
    }

    /// Install the module source and its already-built line index. The module
    /// emitter uses this path so procedure contexts share both allocations.
    pub(super) fn set_indexed_source(
        &mut self,
        source: std::rc::Rc<str>,
        line_index: tcl_lexer::LineIndex,
    ) {
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
        self.current_span = span.into();
        self.current_span_is_command = true;
        self.current_command_namespace =
            Self::unrooted_namespace(&self.resolution_namespace).to_owned();
    }

    /// Select compiler control which has a useful diagnostic span but is not
    /// itself a replayable Tcl command (for example, a CFG branch condition).
    pub(crate) fn set_control_source_span(&mut self, span: Option<Span>) {
        self.current_span = span;
        self.current_span_is_command = false;
        self.current_command_namespace.clear();
    }

    /// Whether `name` is free of whole-unit mutation, for transforms that have
    /// no independently replayable command boundary (issue #1585).
    ///
    /// Answers from the whole-module [`Self::command_bindings`] summary — a
    /// flow-**insensitive** scan on purpose: a `rename` buried in a proc body
    /// can fire before a call earlier in the file runs, so "no rename seen so
    /// far" is not a sound answer. Without a module view the answer is the
    /// historical `true`; see [`Self::command_bindings`].
    #[must_use]
    pub fn trusts_builtin(&self, name: &str) -> bool {
        self.command_bindings.is_none_or(|m| m.trusts(name))
    }

    /// Record one source binding relied on by specialised emission.
    pub fn require_command_binding(&mut self, binding: &tcl_runtime_api::CommandBindingIdentity) {
        self.command_binding_requirements.insert(binding.clone());
        self.current_command_namespace
            .clone_from(&binding.resolution_namespace);
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
                .resolve_call(command, args, self.registry.own_surface_query())?;
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
            self.current_command_namespace
                .clone_from(&site.binding.resolution_namespace);
            if !self.plain_command_dispatch {
                self.require_command_binding(&site.binding);
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
        let (span, text, line) = site.map_or((None, String::new(), 0), |site| {
            if !self.plain_command_dispatch {
                self.require_command_binding(&site.binding);
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
                .map(|site| site.binding.resolution_namespace.clone())
                .unwrap_or_default();
            instr.source_command_boundary = site.is_some().into();
        }
    }

    /// Give every synthetic `START_CMD` just emitted for an inline body command
    /// its exact replay text, without overwriting a more deeply nested command
    /// boundary that was already restamped while those instructions were
    /// produced.
    ///
    /// The absolute module span stays inherited from the enclosing construct:
    /// debugger and explorer consumers index the module source with it. Runtime
    /// replay reads the explicit command text and local continuation carried by
    /// `START_CMD`, so this helper updates only those replay-owned fields.
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
                text.clone_into(&mut instruction.source_cmd_text);
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
            text.clone_into(&mut instruction.source_cmd_text);
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

    /// The whole Tcl command represented by [`Self::current_span`], or empty
    /// when the span belongs only to compiler-generated control machinery.
    fn command_span_text(&self) -> String {
        if self.current_span_is_command {
            self.span_text()
        } else {
            String::new()
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
    /// span but no source retains the historical line-one fallback.
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
        }
        instr.source_command_boundary =
            (op == Op::START_CMD && !instr.source_cmd_text.is_empty()).into();
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
            proc_body_src: None,
            error_regions: Vec::new(),
            plain_command_dispatch: self.plain_command_dispatch,
            command_bindings: self.command_binding_requirements.into_iter().collect(),
            procedure_bindings: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let source: std::rc::Rc<str> = "shared\nsource".into();
        let source_clone = std::rc::Rc::clone(&source);
        let line_index = tcl_lexer::LineIndex::new(&source);
        let line_index_clone = line_index.clone();
        let mut shared = CodegenCtx::new(false, &[], &registry);
        shared.set_indexed_source(source, line_index);
        assert!(std::rc::Rc::ptr_eq(&shared.source, &source_clone));
        assert!(
            shared
                .line_index
                .as_ref()
                .is_some_and(|index| index.shares_storage_with(&line_index_clone)),
        );
    }
}
