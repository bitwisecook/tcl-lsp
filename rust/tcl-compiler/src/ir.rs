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

//! Intermediate representation (IR) for Tcl analysis.
//!
//! This is a structured IR front-end that keeps source [`Span`]s on all
//! nodes. Later passes lower this to CFG + SSA.
//!
//! **Architectural note:** every IR node carries a [`Span`] (two `u32`
//! byte offsets) rather than inline `(line, character, offset)` pairs.
//! Text and positions are resolved on demand via a
//! [`SourceMap`](tcl_lexer::SourceMap). This mirrors the span-first
//! design established in the lexer crate.

use tcl_lexer::Span;

use crate::expr_ast::ExprNode;

// Command tokens

/// Original parsed tokens for a command invocation.
///
/// Carried on [`Statement::Call`] and [`Statement::Barrier`] so
/// downstream passes (optimiser, compiler checks) can inspect tokens
/// without re-lexing.
///
/// Token references use [`Span`]s into the
/// source buffer. The `argv_texts` and `single_token_word` fields
/// preserve the per-word metadata needed by analysis passes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandTokens {
    /// Per-word representative token spans.
    pub argv: Vec<Span>,
    /// Per-word text values.
    pub argv_texts: Vec<String>,
    /// Per-word representative token kind. Preserves the [`tcl_lexer::TokenType`]
    /// of each argv entry so analysis passes can distinguish a
    /// brace-string literal (`Str`) from a bareword (`Esc`), a
    /// variable reference (`Var`), or a command substitution
    /// (`Cmd`).
    pub argv_kinds: Vec<tcl_lexer::TokenType>,
    /// Whether each word consists of a single token.
    pub single_token_word: Vec<bool>,
    /// All tokens in the command (including separators).
    pub all_tokens: Vec<Span>,
    /// `{*}` expansion markers per word, if any word uses expansion.
    pub expand_word: Option<Vec<bool>>,
}

impl CommandTokens {
    /// Whether **argument** `idx` (0-based, so `argv[idx + 1]`) is a
    /// brace-quoted literal word — `{…}`, the one word form Tcl leaves
    /// completely unsubstituted.
    ///
    /// The IR stores argument *content*, which cannot tell `{$a}` (the
    /// literal three-character name `$a`) from `$a` (a substitution); the
    /// per-word token kind can.  A `$var` lexes to `Var`, a `[cmd]` to `Cmd`,
    /// a `"…"` (which *does* substitute) to `Esc`, and only a brace-quoted
    /// word to `Str`.
    ///
    /// The single-token conjunct is belt-and-braces: a word that opens with
    /// `{` must be entirely brace-quoted (`set x {a}{b}` is a tclsh parse
    /// error, "extra characters after close-brace"), so a `Str`
    /// representative token already implies a single-token word.
    ///
    /// The one definition of this question in the compiler — the dynamic-name
    /// barrier, the place bridge, the inliner's literal-argument binding and
    /// the def/use naming layer all ask it here rather than re-deriving it
    /// (issue #1078).
    #[must_use]
    pub fn arg_is_braced_literal(&self, idx: usize) -> bool {
        self.argv_kinds.get(idx + 1) == Some(&tcl_lexer::TokenType::Str)
            && self.single_token_word.get(idx + 1).copied().unwrap_or(true)
    }
}

// IR node types
//
// IR node kinds are modelled as a flat enum with struct variants.
// Each variant carries a `Span` for position tracking, plus
// variant-specific fields.

/// A script: a sequence of IR statements.
///
/// Using `Vec` rather than a tuple
/// because scripts are built incrementally during lowering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Script {
    /// Statements in execution order.
    pub statements: Vec<Statement>,
}

impl Script {
    /// Create an empty script.
    #[must_use]
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }

    /// Create a script from a list of statements.
    #[must_use]
    pub fn from_statements(statements: Vec<Statement>) -> Self {
        Self { statements }
    }
}

/// Depth cap for [`for_each_statement`]'s recursion over nested
/// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`/`uplevel` bodies —
/// issue #996. Transitively bounded today via `MAX_LOWER_NEST_DEPTH` (every
/// `Script` this crate hands to `for_each_statement` was built by
/// [`crate::lowering`], which already caps its own construction at 256 and
/// emits a `Statement::Barrier` past that point), capped here independently
/// for defence-in-depth and consistency with every other full-tree walker in
/// this crate (`optimiser::MAX_OPTIMISER_WALK_DEPTH`,
/// `codegen::structured::MAX_STRUCTURED_DEPTH`), since this is a `pub`
/// utility any future caller could hand a `Script` built some other way.
const MAX_FOR_EACH_STATEMENT_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Recursively visit every statement in `script`, including nested bodies
/// (`if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`), in source order.
/// Each statement is visited before its own nested bodies (pre-order): a
/// compound statement (`If`, `For`, …) is passed to `visit` once for itself,
/// then its nested scripts are walked in turn.
///
/// Shared traversal for whole-module / whole-body scans that need "every
/// statement anywhere in this script" without each re-deriving the same
/// nested-body match arms (e.g. [`crate::var_observability::scan_module_global_names`]).
///
/// Nesting past [`MAX_FOR_EACH_STATEMENT_DEPTH`] (256) is silently not
/// descended into — `visit` still runs for every statement up to and
/// including that depth, matching this function's existing "best-effort
/// scan" contract (callers already tolerate this visitor skipping statements
/// it cannot reach, e.g. inside opaque `Statement::Barrier`s).
pub fn for_each_statement(script: &Script, visit: &mut impl FnMut(&Statement)) {
    for_each_statement_inner(script, visit, 0);
}

fn for_each_statement_inner(script: &Script, visit: &mut impl FnMut(&Statement), depth: u32) {
    if MAX_FOR_EACH_STATEMENT_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        visit(stmt);
        match stmt {
            Statement::Block { body, .. }
            | Statement::UpFrame { body, .. }
            | Statement::While { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Foreach { body, .. } => {
                for_each_statement_inner(body, visit, depth + 1);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    for_each_statement_inner(&c.body, visit, depth + 1);
                }
                if let Some(b) = else_body {
                    for_each_statement_inner(b, visit, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                for_each_statement_inner(init, visit, depth + 1);
                for_each_statement_inner(next, visit, depth + 1);
                for_each_statement_inner(body, visit, depth + 1);
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                for_each_statement_inner(body, visit, depth + 1);
                for h in handlers {
                    for_each_statement_inner(&h.body, visit, depth + 1);
                }
                if let Some(fb) = finally_body {
                    for_each_statement_inner(fb, visit, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        for_each_statement_inner(b, visit, depth + 1);
                    }
                }
                if let Some(b) = default_body {
                    for_each_statement_inner(b, visit, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// An `if` clause: condition + body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IfClause {
    /// The parsed condition expression.
    pub condition: ExprNode,
    /// Source span of the condition text.
    pub condition_span: Span,
    /// Absolute source offset of the condition *text*'s first byte, when
    /// the text handed to `parse_expr` is a verbatim source slice (single
    /// braced / bare word).  Expression-AST leaf offsets are relative to
    /// that text, so `condition_base + leaf.start` recovers an absolute
    /// operand span.  `None` when the text was reconstructed (quoted /
    /// multi-token words) — consumers fall back to `condition_span`.
    pub condition_base: Option<u32>,
    /// Body script to execute when the condition is true.
    pub body: Script,
    /// Source span of the body text.
    pub body_span: Span,
}

/// A `try` handler clause (`on`/`trap`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TryHandler {
    /// Handler kind: `"on"` or `"trap"`.
    pub kind: String,
    /// Return code or error class pattern to match.
    pub match_arg: String,
    /// Variable bound to the result, if any.
    pub var_name: Option<String>,
    /// Variable bound to the options dict, if any.
    pub options_var: Option<String>,
    /// Handler body.
    pub body: Script,
    /// Source span of the handler body.
    pub body_span: Span,
    /// `true` when the handler body is the literal `-` fallthrough
    /// marker: the handler shares the next non-fallthrough handler's
    /// body (Tcl `try` semantics, mirroring `switch`). `body` is an
    /// empty script in that case — the `-` is *not* a command call.
    pub fallthrough: bool,
}

/// A `switch` arm: pattern + body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SwitchArm {
    /// Pattern text.
    pub pattern: String,
    /// Source span of the pattern.
    pub pattern_span: Span,
    /// Body script (`None` for fall-through arms).
    pub body: Option<Script>,
    /// Source span of the body (`None` for fall-through arms).
    pub body_span: Option<Span>,
    /// Whether this arm falls through to the next.
    pub fallthrough: bool,
}

/// An iterator group in a `foreach`/`lmap`/`dict for` loop.
///
/// Each group pairs a list of loop variable names with the list
/// argument text they iterate over.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForeachIterator {
    /// Variable names assigned each iteration.
    pub vars: Vec<String>,
    /// Source text of the list argument.
    pub list_arg: String,
}

/// An IR statement — the building block of the compiler pipeline.
///
/// Each variant
/// carries a [`Span`] for position tracking. The span is resolved
/// to text or `(line, character)` via a `SourceMap` on demand.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    /// Constant assignment: `set name value` where value is a literal.
    AssignConst {
        /// Source span of the full command.
        span: Span,
        /// Variable name.
        name: String,
        /// Whether the *name* word was a brace-string literal
        /// (`set {a($x)} v`). Braces suppress substitution, so an
        /// array-element target's key (`a($x)`) must be pushed
        /// LITERALLY (`$x` stays `$x`) rather than substituted.
        /// Mirrors the `braced` precedent on [`Self::Return`].
        /// Defaults to `false` for every construction site except
        /// the `set` lowering hook (only it knows the source token
        /// kind); honoured by codegen's `push_var_ref`.
        name_braced: bool,
        /// Constant value text.
        value: String,
        /// Content span of the value *word* when `value` is written
        /// verbatim in the source at this statement (the writable
        /// provenance a rename may splice a new command name over).
        /// `None` when the constant has no exact source
        /// representation — a synthesised constant (folding,
        /// desugaring, an optimiser rewrite) or a word whose content
        /// was reconstructed — in which case value-provenance
        /// consumers must abstain rather than guess a span.
        value_span: Option<Span>,
    },

    /// Expression assignment: `set name [expr ...]`.
    AssignExpr {
        /// Source span of the full command.
        span: Span,
        /// Variable name.
        name: String,
        /// Whether the *name* word was a brace-string literal — see
        /// [`Self::AssignConst::name_braced`].
        name_braced: bool,
        /// Parsed expression AST.
        expr: ExprNode,
        /// Absolute source offset of the expression text's first byte —
        /// see [`IfClause::condition_base`].  `None` when the expression
        /// text is not a verbatim source slice.
        expr_base: Option<u32>,
    },

    /// General value assignment: `set name value` where value may
    /// contain substitutions.
    AssignValue {
        /// Source span of the full command.
        span: Span,
        /// Variable name.
        name: String,
        /// Whether the *name* word was a brace-string literal — see
        /// [`Self::AssignConst::name_braced`].
        name_braced: bool,
        /// Value text (may contain backslash substitutions).
        value: String,
        /// Whether the value contains backslash substitutions.
        value_needs_backsubst: bool,
        /// Command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
    },

    /// Increment: `incr name ?amount?`.
    Incr {
        /// Source span.
        span: Span,
        /// Variable name.
        name: String,
        /// Whether the *name* word was a brace-string literal — see
        /// [`Self::AssignConst::name_braced`].
        name_braced: bool,
        /// Increment amount (None = 1).
        amount: Option<String>,
        /// Whether it is safe if the variable is uninitialised.
        safe_on_uninit: bool,
    },

    /// Standalone expression evaluation: `expr ...`.
    ExprEval {
        /// Source span.
        span: Span,
        /// Parsed expression AST.
        expr: ExprNode,
        /// Absolute source offset of the expression text's first byte —
        /// see [`IfClause::condition_base`].  `None` when the expression
        /// text is not a verbatim source slice.
        expr_base: Option<u32>,
    },

    /// A generic command invocation.
    ///
    /// Optionally annotated with variables it defines/reads.
    Call {
        /// Source span.
        span: Span,
        /// Command name as it appeared in source — preserves the
        /// original spelling for diagnostics ("unknown command
        /// `Foo`" reads better than "unknown command `::Foo`").
        command: String,
        /// Canonical command name resolved against the
        /// registry.  `None` when the lowerer hasn't (yet)
        /// populated it or when the command is unknown to the
        /// registry; downstream dispatch sites use
        /// [`Statement::canonical_command_or_source`] so a `None`
        /// canonical falls back to `command`.  The lowerer populates
        /// it when an alias resolves (`interp alias {} foo {} ::ns::bar; foo
        /// 1 2` lowers to a `Call` with `command="foo"` and
        /// `canonical_command=Some("::ns::bar")`).
        canonical_command: Option<String>,
        /// Argument texts.
        args: Vec<String>,
        /// Variables defined by this command.
        defs: Vec<String>,
        /// Variables read by this command (beyond `$`-references in args).
        reads: Vec<String>,
        /// Whether defined variables are also read (read-before-write).
        reads_own_defs: bool,
        /// Whether it is safe if defined variables are uninitialised.
        safe_on_uninit: bool,
        /// Command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
        /// Per-iterator-group sizes for synthetic `foreach` /
        /// `lmap` / `dict for` / `dict map` header calls produced
        /// by the CFG builder.  ``Some([n0, n1, …])`` encodes that
        /// `defs` is a flattened concatenation of `n0` + `n1` + …
        /// vars per iterator group, so the codegen can reconstruct
        /// the original `var-list` ↔ `list-arg` pairing.  ``None``
        /// for every other call shape.
        foreach_groups: Option<Vec<usize>>,
    },

    /// Return statement: `return ?value?`.
    Return {
        /// Source span.
        span: Span,
        /// Return value text, if any.
        value: Option<String>,
        /// Return expression, if any (for `return [expr ...]`).
        expr: Option<ExprNode>,
        /// Whether the value was braced.
        braced: bool,
    },

    /// A command whose side effects defeat static analysis.
    ///
    /// Commands like `eval`, `uplevel`, and `upvar` can modify
    /// arbitrary variables at runtime, so no constant propagation or
    /// dead-store reasoning can cross a barrier.
    Barrier {
        /// Source span.
        span: Span,
        /// Human-readable reason for the barrier.
        reason: String,
        /// Original command name (source-surface spelling).
        command: String,
        /// Canonical command name resolved against the
        /// registry.  See [`Statement::Call::canonical_command`].
        canonical_command: Option<String>,
        /// Original argument texts.
        args: Vec<String>,
        /// Command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
    },

    /// Inline group of statements to splice into the enclosing
    /// script *without* introducing a separate scope. Produced by
    /// [`crate::inline_uplevel`] when a passthrough callsite is
    /// rewritten — the callee's body runs in the caller's frame, so
    /// it can be flattened directly into the parent block stream.
    /// Also produced by const-propagation when an `eval` /
    /// `uplevel` body resolves to a brace-literal.
    Block {
        /// Source span of the original call that produced this block.
        span: Span,
        /// The pre-lowered body, evaluated in the enclosing scope.
        body: Script,
        /// Fully-qualified namespace the body was lowered in.
        namespace: String,
        /// Original command-tokens snapshot for downstream analysis
        /// (lets diagnostics still report the source surface form).
        tokens: Option<CommandTokens>,
    },

    /// Static-body uplevel — `uplevel ?level? {body}` where the body
    /// is a brace-string literal so the body's IR can be lowered
    /// inline. Models the "shift the active frame, evaluate body,
    /// restore frame" semantics that `uplevel` provides without the
    /// runtime [`Self::Barrier`] dispatch.
    ///
    /// Codegen emits `frame_depth_stash`
    /// / `frame_depth_restore` around the body.
    UpFrame {
        /// Source span of the original `uplevel` call.
        span: Span,
        /// The level argument's magnitude — `1` for `uplevel 1
        /// {body}` (the canonical form) and for a bare `uplevel
        /// {body}`, `0` for `uplevel #0 {body}` or `uplevel 0
        /// {body}`. Read it together with [`Self::UpFrame::absolute`]:
        /// the same magnitude means two different frames depending on
        /// that flag.
        frame_shift: i32,
        /// `true` when the level was written in the `#N` *absolute*
        /// form, counting down from the global frame (`uplevel #0`
        /// evaluates the body in the global frame). `false` for the
        /// *relative* form, counting up from the current frame
        /// (`uplevel 0` evaluates the body in the current frame,
        /// `uplevel 1` in the caller's).
        ///
        /// The distinction is load-bearing, not cosmetic: `uplevel #0`
        /// and `uplevel 0` are different frames whenever the current
        /// frame is not the global one, so bare command words in the
        /// body resolve against different namespaces
        /// (tclsh8.6/9.0-confirmed).
        absolute: bool,
        /// The pre-lowered body, evaluated at the shifted frame.
        body: Script,
        /// Original command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
    },

    /// Conditional: `if cond body ?elseif cond body ...? ?else body?`.
    If {
        /// Source span.
        span: Span,
        /// `if`/`elseif` clauses in order.
        clauses: Vec<IfClause>,
        /// Optional `else` body.
        else_body: Option<Script>,
        /// Source span of the `else` body.
        else_span: Option<Span>,
    },

    /// `for` loop: `for init cond next body`.
    For {
        /// Source span.
        span: Span,
        /// Initialisation script.
        init: Script,
        /// Source span of the init script.
        init_span: Span,
        /// Loop condition expression.
        condition: ExprNode,
        /// Source span of the condition.
        condition_span: Span,
        /// Absolute source offset of the condition text — see
        /// [`IfClause::condition_base`].
        condition_base: Option<u32>,
        /// Next-iteration script.
        next: Script,
        /// Source span of the next script.
        next_span: Span,
        /// Loop body.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// Per-word source token metadata for the generic ("frozen") runtime
        /// fallback, when the loop cannot be inlined (a command-substitution
        /// condition). Carries the original word kinds so the fallback pushes
        /// braced `{cond}` / `{body}` words verbatim and substituted words
        /// (`$body`) interpolated — exactly as written. `None` for
        /// synthetically-constructed loops that never take the fallback.
        raw_tokens: Option<CommandTokens>,
    },

    /// `while` loop: `while cond body`.
    While {
        /// Source span.
        span: Span,
        /// Loop condition expression.
        condition: ExprNode,
        /// Source span of the condition.
        condition_span: Span,
        /// Absolute source offset of the condition text — see
        /// [`IfClause::condition_base`].
        condition_base: Option<u32>,
        /// Loop body.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// Per-word source token metadata for the generic ("frozen") runtime
        /// fallback — see [`Statement::For::raw_tokens`].
        raw_tokens: Option<CommandTokens>,
    },

    /// `foreach`/`lmap`/`dict for`/`dict map` loop.
    Foreach {
        /// Source span.
        span: Span,
        /// Iterator groups: `(var_list, list_arg)` pairs.
        iterators: Vec<ForeachIterator>,
        /// Loop body.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Whether this is an `lmap` (returns a list).
        is_lmap: bool,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// Whether this is `dict for`/`dict map`.
        is_dict_iteration: bool,
        /// Whether this is `array for {k v} arr body` (Tcl 9.0). Unlike a plain
        /// `foreach`, the body runs in the caller's frame over an *array* (not a
        /// list value), so analysis inlines it into the caller unit (binding the
        /// loop vars) while codegen barriers it to the `::tcl::array::for`
        /// ensemble invoke — see `lower_foreach_dispatch`.
        is_array_iteration: bool,
        /// Per-word source token metadata for the generic runtime fallback
        /// (the `dict for`/`map` barrier and the runtime `foreach`/`lmap`
        /// call), so braced var-lists / bodies are pushed verbatim and
        /// `$list` arguments substituted — see [`Statement::For::raw_tokens`].
        raw_tokens: Option<CommandTokens>,
    },

    /// `catch script ?resultVar? ?optionsVar?`.
    Catch {
        /// Source span.
        span: Span,
        /// Body script to evaluate.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Variable for the result, if any.
        result_var: Option<String>,
        /// Variable for the options dict, if any.
        options_var: Option<String>,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// Original parsed tokens, including braced/quoted flags.
        /// Threaded through so the CFG's `Catch → Call` lowering
        /// (`emit_opaque_catch`) can preserve the `{…}` braces
        /// around the body when the codegen falls back to
        /// `tcl_eval`.  Without this, ``catch {$undef} msg`` would
        /// be reconstructed as ``catch $undef msg`` and the
        /// unset-var read would fire before catch could intercept
        /// it.
        tokens: Option<CommandTokens>,
    },

    /// `try body ?on/trap ...? ?finally body?`.
    Try {
        /// Source span.
        span: Span,
        /// Body script.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Handler clauses.
        handlers: Vec<TryHandler>,
        /// Optional `finally` body.
        finally_body: Option<Script>,
        /// Source span of the `finally` body.
        finally_span: Option<Span>,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
    },

    /// `switch` statement.
    Switch {
        /// Source span.
        span: Span,
        /// Subject text being matched.
        subject: String,
        /// Source span of the subject.
        subject_span: Span,
        /// Pattern/body arms.
        arms: Vec<SwitchArm>,
        /// Optional default body.
        default_body: Option<Script>,
        /// Source span of the default body.
        default_span: Option<Span>,
        /// Match mode: `"exact"`, `"glob"`, or `"regexp"`.
        mode: SwitchMode,
        /// Whether matching is case-insensitive.
        nocase: bool,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// `true` when the arms came from a single braced
        /// `{pat body …}` block — patterns are literal list elements
        /// with no substitution. `false` when supplied as separate
        /// words (`switch $s $pat {body}`), where each pattern
        /// undergoes normal `$var` / `[cmd]` runtime substitution
        /// before matching. Defaults to `true`, the canonical braced form.
        patterns_braced: bool,
    },
}

impl Statement {
    /// Return the source span of this statement.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::AssignConst { span, .. }
            | Self::AssignExpr { span, .. }
            | Self::AssignValue { span, .. }
            | Self::Incr { span, .. }
            | Self::ExprEval { span, .. }
            | Self::Call { span, .. }
            | Self::Return { span, .. }
            | Self::Barrier { span, .. }
            | Self::Block { span, .. }
            | Self::UpFrame { span, .. }
            | Self::If { span, .. }
            | Self::For { span, .. }
            | Self::While { span, .. }
            | Self::Foreach { span, .. }
            | Self::Catch { span, .. }
            | Self::Try { span, .. }
            | Self::Switch { span, .. } => *span,
        }
    }

    /// Dispatch helper that returns the canonical command name
    /// when populated, falling back to the source-surface `command`
    /// otherwise.  Use from downstream dispatch sites (codegen hook
    /// lookup, side-effect classification, GVN purity) so a single
    /// canonical key drives every consumer; use the bare `command`
    /// field directly when rendering source-surface text in
    /// diagnostics.
    ///
    /// Returns `""` for non-Call / non-Barrier statements.
    #[must_use]
    pub fn canonical_command_or_source(&self) -> &str {
        match self {
            Self::Call {
                command,
                canonical_command,
                ..
            }
            | Self::Barrier {
                command,
                canonical_command,
                ..
            } => canonical_command.as_deref().unwrap_or(command.as_str()),
            _ => "",
        }
    }
}

/// Switch matching mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SwitchMode {
    /// Exact string match (default).
    #[default]
    Exact,
    /// Glob pattern match.
    Glob,
    /// Regular expression match.
    Regexp,
}

impl SwitchMode {
    /// Parse a switch mode from its Tcl string form.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "glob" => Self::Glob,
            "regexp" => Self::Regexp,
            _ => Self::Exact,
        }
    }

    /// Return the Tcl string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Glob => "glob",
            Self::Regexp => "regexp",
        }
    }
}

// Procedure and module types

/// A procedure definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Procedure {
    /// Short procedure name.
    pub name: String,
    /// Fully qualified name (e.g. `::ns::proc`).
    pub qualified_name: String,
    /// Parameter names.
    pub params: Vec<String>,
    /// Source span of the definition.
    pub span: Span,
    /// Procedure body.
    pub body: Script,
    /// Raw parameter list text.
    pub params_raw: String,
    /// Source text of the body (`None` for synthetic procs like `when`).
    pub body_source: Option<String>,
    /// Whether defined inside `namespace eval`.
    pub namespace_scoped: bool,
    /// BIG-IP handler priority (0..2^32-1, default 500).
    pub base_priority: u32,
}

/// A method definition within a class body.
///
/// Compiles like [`Procedure`] but carries class context for
/// interprocedural analysis and devirtualisation.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    /// Enclosing class name.
    pub class_name: String,
    /// Method name.
    pub method_name: String,
    /// Parameter names.
    pub params: Vec<String>,
    /// Method body.
    pub body: Script,
    /// Method kind.
    pub kind: MethodKind,
    /// Source span (may be absent for synthetic methods).
    pub span: Option<Span>,
    /// Instance-variable names in scope for this method (class-level
    /// `variable` declarations + the method's own `variable` decls).
    /// A method that *writes* any of these mutates object state and is
    /// therefore impure for O126 purposes, even though the write looks
    /// like a plain local `set`. Used by interprocedural method-purity.
    pub instance_vars: std::collections::HashSet<String>,
}

/// The kind of a class method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MethodKind {
    /// Regular instance method.
    #[default]
    Method,
    /// Class-level method.
    ClassMethod,
    /// Constructor.
    Constructor,
    /// Destructor.
    Destructor,
}

impl MethodKind {
    /// Parse from the string representation.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "classmethod" => Self::ClassMethod,
            "constructor" => Self::Constructor,
            "destructor" => Self::Destructor,
            _ => Self::Method,
        }
    }

    /// Return the string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Method => "method",
            Self::ClassMethod => "classmethod",
            Self::Constructor => "constructor",
            Self::Destructor => "destructor",
        }
    }
}

/// A top-level module: procedures + top-level script.
///
/// This is the only mutable IR type — it accumulates procedures and
/// methods during lowering.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    /// The original source text this module was lowered from. Spans on the
    /// lowered statements index into it, so codegen can recover each command's
    /// surface text for `errorInfo` (`while executing "…"`). Empty when not
    /// supplied (older construction paths / hand-built test modules).
    pub source: String,
    /// Top-level script (code outside any procedure).
    pub top_level: Script,
    /// Named procedures.
    pub procedures: std::collections::HashMap<String, Procedure>,
    /// Named methods (keyed by `class::method`).
    pub methods: std::collections::HashMap<String, MethodDef>,
    /// Synthetic *body units* — the bodies of commands that run their script
    /// argument in a fresh frame but are **not** real named procedures:
    /// `apply` lambdas and `namespace eval` bodies (keyed by a synthetic
    /// qualified name like `::apply#0`, or `::NS::namespace-eval#0` — `NS`
    /// *prefixes* the marker, matching every other qname's "everything
    /// before the last `::` is the enclosing namespace" convention, so a
    /// bare call inside the body resolves against the namespace it actually
    /// targets rather than always the global namespace).
    ///
    /// These are lowered into [`Procedure`]s purely so the static-analysis
    /// pipeline (CFG → SSA → SCCP → taint) reaches *inside* the body — the
    /// same coverage a real `proc` gets. They are held in a **separate** map
    /// from [`Self::procedures`] so codegen (which emits `procedures`) never
    /// materialises them as callable procs, and from [`Self::methods`] so the
    /// TclOO-specific consumers (interproc method purity, the O126 optimiser
    /// gate) never mistake them for methods. The body still executes at
    /// runtime via its command's own `Statement::Barrier`, so bytecode is
    /// byte-identical whether or not a body unit is recorded.
    pub body_units: std::collections::HashMap<String, Procedure>,
    /// The subset of [`Self::body_units`] that are **lambda** frames — an
    /// `apply` literal's body, whose parameter list binds real locals exactly
    /// as a `proc`'s does.
    ///
    /// A `namespace eval` body unit is *not* one: its variables are the
    /// namespace's, shared with every other body that opens the same
    /// namespace, so a name it reads without writing is routinely defined
    /// elsewhere.  The read-before-set family (`W210`) therefore runs over
    /// this subset only — a lambda body is a closed frame and an unwritten
    /// read in it is a genuine error (issue #1070).
    pub lambda_body_units: std::collections::BTreeSet<String>,
    /// Procedure names that were defined more than once.
    pub redefined_procedures: std::collections::HashSet<String>,
    /// `TclOO` method qnames defined more than once (a later
    /// `oo::define` / in-body redefinition replaces the body at
    /// runtime). Method purity is forced impure for these — we can't
    /// prove which body a given dispatch runs, so the O126 `my
    /// <method>` deletion gate must stay conservative.
    pub redefined_methods: std::collections::HashSet<String>,
    /// `namespace import` directives captured at lowering time —
    /// `(context_namespace, absolute_pattern)` pairs. Future codegen
    /// passes pattern-match each against the final
    /// `Self::procedures` table to resolve unqualified calls
    /// directly instead of falling back to the runtime
    /// interpreter. Only absolute patterns (`::foo::*` /
    /// `::foo::bar`) are recorded; relative patterns require
    /// runtime namespace-path walking which compile-time
    /// resolution does not model.
    pub namespace_imports: Vec<(String, String)>,
    /// `namespace export` directives captured at lowering time —
    /// `(context_namespace, pattern)` pairs. Codegen consults this
    /// list to gate the import shortcut on actual exportedness so
    /// `namespace import ::foo::*` only resolves names that
    /// `::foo` actually exports.
    pub namespace_exports: Vec<(String, String)>,
    /// Literal command names that have an execution trace
    /// registered (`trace add execution NAME enter|leave HANDLER`).
    /// GVN consults this set to gate purity (a traced call is
    /// never pure because the trace handler composes side effects
    /// in).
    pub traced_commands: std::collections::BTreeSet<String>,
    /// `true` when a `trace add execution` was seen with a
    /// non-literal command target (`trace add execution $cmd ...`).
    /// Forces GVN / partial-redundancy / loop-invariant passes to
    /// treat *every* call as potentially traced.
    pub has_dynamic_trace: bool,
    /// Literal variable names targeted by an active *variable* trace
    /// (`trace add variable NAME ops HANDLER`, or the deprecated
    /// `trace variable`/`vdelete` spellings) anywhere in the module —
    /// top level or any procedure body, regardless of whether the
    /// installing call is reachable before or after a given use.  A
    /// read of a traced variable can run arbitrary handler code, and a
    /// write handler can rewrite the value being stored, so the
    /// propagation optimiser (`O102` load-forwarding) and dead-store
    /// elimination must never treat a use of one of these names as
    /// equivalent to its last literal assignment. Whole-module and
    /// position-independent by design — mirrors [`Self::traced_commands`]
    /// but for `ArgRole::VarWrite`-role variable-trace targets
    /// (`Traits::ESTABLISHES_VARIABLE_TRACE`) rather than execution-trace
    /// command names.
    pub traced_variables: std::collections::BTreeSet<String>,
    /// `true` when a variable-trace install/remove call was seen with a
    /// non-literal variable-name target (`trace add variable $name ...`).
    /// Forces the propagation optimiser to treat *every* variable as
    /// potentially traced.
    pub has_dynamic_variable_trace: bool,
}

/// Extract the event name from a `::when::` qualified name.
///
/// Handles both `::when::HTTP_REQUEST` and indexed forms like
/// `::when::HTTP_REQUEST#1`.
#[must_use]
pub fn when_event_name(qualified_name: &str) -> &str {
    let bare = qualified_name
        .strip_prefix("::when::")
        .unwrap_or(qualified_name);
    match bare.find('#') {
        Some(idx) => &bare[..idx],
        None => bare,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_script() {
        let script = Script::new();
        assert!(script.statements.is_empty());
    }

    #[test]
    fn script_from_statements() {
        let stmts = vec![Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }];
        let script = Script::from_statements(stmts);
        assert_eq!(script.statements.len(), 1);
    }

    #[test]
    fn for_each_statement_visits_nested_bodies_in_order() {
        // `if {c} { inner1 } else { inner2 }` — the visitor must see the
        // `If` itself plus both nested-body calls, in source order.
        let inner1 = Statement::Call {
            span: Span::new(0, 0),
            command: "inner1".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let inner2 = Statement::Call {
            span: Span::new(0, 0),
            command: "inner2".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let if_stmt = Statement::If {
            span: Span::new(0, 0),
            clauses: vec![IfClause {
                condition: ExprNode::Raw { text: "c".into() },
                condition_span: Span::new(0, 0),
                body: Script::from_statements(vec![inner1]),
                body_span: Span::new(0, 0),
                condition_base: None,
            }],
            else_body: Some(Script::from_statements(vec![inner2])),
            else_span: None,
        };
        let script = Script::from_statements(vec![if_stmt]);

        let mut seen: Vec<String> = Vec::new();
        for_each_statement(&script, &mut |stmt| {
            let name = match stmt {
                Statement::If { .. } => "if".to_owned(),
                Statement::Call { command, .. } => command.clone(),
                _ => "other".to_owned(),
            };
            seen.push(name);
        });
        assert_eq!(seen, vec!["if", "inner1", "inner2"]);
    }

    /// Regression coverage for issue #996: `for_each_statement` recurses
    /// once per nested `If`/`For`/`While`/`Foreach`/`Catch`/`Try`/`Switch`/
    /// `UpFrame` body, with no depth cap of its own before this fix.
    /// Transitively bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering
    /// pass today (every `Script` this crate hands to `for_each_statement`
    /// is built by `crate::lowering`), so this is defence-in-depth /
    /// consistency with every other full-tree walker in this crate, not a
    /// currently-reproducible crash. This test builds the nested `Script`
    /// directly (bypassing lowering) so it genuinely exercises
    /// `for_each_statement`'s own new cap rather than lowering's; 2000
    /// levels is comfortably past the new 256-level cap. The assertion is
    /// that the call returns at all, not what it returns.
    #[test]
    fn deeply_nested_if_survives_for_each_statement() {
        const DEPTH: usize = 2000;
        let leaf = Statement::Call {
            span: Span::new(0, 0),
            command: "leaf".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let mut script = Script::from_statements(vec![leaf]);
        for _ in 0..DEPTH {
            script = Script::from_statements(vec![Statement::If {
                span: Span::new(0, 0),
                clauses: vec![IfClause {
                    condition: ExprNode::Raw { text: "1".into() },
                    condition_span: Span::new(0, 0),
                    body: script,
                    body_span: Span::new(0, 0),
                    condition_base: None,
                }],
                else_body: None,
                else_span: None,
            }]);
        }

        let mut count = 0usize;
        for_each_statement(&script, &mut |_| count += 1);
        assert!(count > 0, "must still visit statements within the cap");
        assert!(count <= DEPTH + 1, "must not exceed the number of nodes");
    }

    #[test]
    fn for_each_statement_descends_into_upframe_body() {
        // FN guard (P1, code review): `UpFrame` (a static-body `uplevel
        // ?level? {...}`) has a nested `body: Script` just like `Block` /
        // `While` / `Catch` / `Foreach`, but was missing from the grouped
        // match arm — the visitor stopped at the `UpFrame` statement itself
        // and never walked its body, so a whole-module scan built on this
        // visitor (e.g. `var_observability::scan_module_global_names`)
        // silently missed anything declared inside a static `uplevel` body.
        let inner = Statement::Call {
            span: Span::new(0, 0),
            command: "inner".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let upframe = Statement::UpFrame {
            span: Span::new(0, 0),
            frame_shift: 0,
            absolute: true,
            body: Script::from_statements(vec![inner]),
            tokens: None,
        };
        let script = Script::from_statements(vec![upframe]);

        let mut seen: Vec<String> = Vec::new();
        for_each_statement(&script, &mut |stmt| {
            let name = match stmt {
                Statement::UpFrame { .. } => "upframe".to_owned(),
                Statement::Call { command, .. } => command.clone(),
                _ => "other".to_owned(),
            };
            seen.push(name);
        });
        assert_eq!(seen, vec!["upframe", "inner"]);
    }

    #[test]
    fn statement_span_accessor() {
        let stmt = Statement::Call {
            span: Span::new(5, 20),
            command: "puts".into(),
            canonical_command: None,
            args: vec!["hello".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        assert_eq!(stmt.span(), Span::new(5, 20));
    }

    #[test]
    fn assign_const_roundtrip() {
        let stmt = Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            name_braced: false,
            value: "42".into(),
            value_span: None,
        };
        if let Statement::AssignConst { name, value, .. } = &stmt {
            assert_eq!(name, "x");
            assert_eq!(value, "42");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn barrier_with_tokens() {
        let tokens = CommandTokens {
            argv: vec![Span::new(0, 4), Span::new(5, 10)],
            argv_texts: vec!["eval".into(), "body".into()],
            argv_kinds: vec![tcl_lexer::TokenType::Esc, tcl_lexer::TokenType::Str],
            single_token_word: vec![true, true],
            all_tokens: vec![Span::new(0, 4), Span::new(4, 5), Span::new(5, 10)],
            expand_word: None,
        };
        let stmt = Statement::Barrier {
            span: Span::new(0, 10),
            reason: "eval".into(),
            command: "eval".into(),
            canonical_command: None,
            args: vec!["body".into()],
            tokens: Some(tokens.clone()),
        };
        if let Statement::Barrier {
            tokens: Some(t), ..
        } = &stmt
        {
            assert_eq!(t.argv_texts, vec!["eval", "body"]);
        } else {
            panic!("expected tokens");
        }
    }

    #[test]
    fn if_statement_structure() {
        let stmt = Statement::If {
            span: Span::new(0, 50),
            clauses: vec![IfClause {
                condition: ExprNode::Literal {
                    text: "1".into(),
                    start: 3,
                    end: 4,
                },
                condition_span: Span::new(3, 4),
                body: Script::from_statements(vec![Statement::Call {
                    span: Span::new(6, 15),
                    command: "puts".into(),
                    canonical_command: None,
                    args: vec!["yes".into()],
                    defs: Vec::new(),
                    reads: Vec::new(),
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: None,
                    foreach_groups: None,
                }]),
                body_span: Span::new(5, 16),
                condition_base: None,
            }],
            else_body: None,
            else_span: None,
        };
        assert_eq!(stmt.span(), Span::new(0, 50));
        if let Statement::If { clauses, .. } = &stmt {
            assert_eq!(clauses.len(), 1);
        }
    }

    #[test]
    fn for_loop_structure() {
        let stmt = Statement::For {
            span: Span::new(0, 40),
            init: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(5, 12),
                name: "i".into(),
                name_braced: false,
                value: "0".into(),
                value_span: None,
            }]),
            init_span: Span::new(4, 13),
            condition: ExprNode::Binary {
                op: crate::expr_ast::BinOp::Lt,
                left: Box::new(ExprNode::Var {
                    text: "$i".into(),
                    name: "i".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Literal {
                    text: "10".into(),
                    start: 5,
                    end: 7,
                }),
            },
            condition_span: Span::new(14, 22),
            next: Script::from_statements(vec![Statement::Incr {
                span: Span::new(24, 30),
                name: "i".into(),
                name_braced: false,
                amount: None,
                safe_on_uninit: false,
            }]),
            next_span: Span::new(23, 31),
            body: Script::new(),
            body_span: Span::new(32, 34),
            raw_args: Vec::new(),
            raw_tokens: None,
            condition_base: None,
        };
        assert_eq!(stmt.span(), Span::new(0, 40));
    }

    #[test]
    fn switch_mode_parse() {
        assert_eq!(SwitchMode::from_str_lossy("exact"), SwitchMode::Exact);
        assert_eq!(SwitchMode::from_str_lossy("glob"), SwitchMode::Glob);
        assert_eq!(SwitchMode::from_str_lossy("regexp"), SwitchMode::Regexp);
        assert_eq!(SwitchMode::from_str_lossy("unknown"), SwitchMode::Exact);
    }

    #[test]
    fn switch_mode_str() {
        assert_eq!(SwitchMode::Exact.as_str(), "exact");
        assert_eq!(SwitchMode::Glob.as_str(), "glob");
        assert_eq!(SwitchMode::Regexp.as_str(), "regexp");
    }

    #[test]
    fn method_kind_roundtrip() {
        for kind in [
            MethodKind::Method,
            MethodKind::ClassMethod,
            MethodKind::Constructor,
            MethodKind::Destructor,
        ] {
            assert_eq!(MethodKind::from_str_lossy(kind.as_str()), kind);
        }
    }

    #[test]
    fn when_event_name_simple() {
        assert_eq!(when_event_name("::when::HTTP_REQUEST"), "HTTP_REQUEST");
    }

    #[test]
    fn when_event_name_indexed() {
        assert_eq!(when_event_name("::when::HTTP_REQUEST#1"), "HTTP_REQUEST");
    }

    #[test]
    fn when_event_name_no_prefix() {
        assert_eq!(when_event_name("HTTP_REQUEST"), "HTTP_REQUEST");
    }

    #[test]
    fn procedure_construction() {
        let proc = Procedure {
            name: "greet".into(),
            qualified_name: "::greet".into(),
            params: vec!["name".into()],
            span: Span::new(0, 50),
            body: Script::from_statements(vec![Statement::Call {
                span: Span::new(20, 40),
                command: "puts".into(),
                canonical_command: None,
                args: vec!["Hello $name".into()],
                defs: Vec::new(),
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            }]),
            params_raw: "name".into(),
            body_source: Some("puts \"Hello $name\"".into()),
            namespace_scoped: false,
            base_priority: 500,
        };
        assert_eq!(proc.name, "greet");
        assert_eq!(proc.params, vec!["name"]);
    }

    #[test]
    fn module_default() {
        let module = Module::default();
        assert!(module.top_level.statements.is_empty());
        assert!(module.procedures.is_empty());
        assert!(module.methods.is_empty());
        assert!(module.redefined_procedures.is_empty());
        assert!(module.redefined_methods.is_empty());
    }

    #[test]
    fn foreach_with_iterators() {
        let stmt = Statement::Foreach {
            span: Span::new(0, 30),
            iterators: vec![ForeachIterator {
                vars: vec!["k".into(), "v".into()],
                list_arg: "$dict".into(),
            }],
            body: Script::new(),
            body_span: Span::new(25, 28),
            is_lmap: false,
            raw_args: Vec::new(),
            is_dict_iteration: true,
            is_array_iteration: false,
            raw_tokens: None,
        };
        if let Statement::Foreach {
            iterators,
            is_dict_iteration,
            ..
        } = &stmt
        {
            assert_eq!(iterators.len(), 1);
            assert_eq!(iterators[0].vars, vec!["k", "v"]);
            assert!(is_dict_iteration);
        }
    }

    #[test]
    fn catch_statement() {
        let stmt = Statement::Catch {
            span: Span::new(0, 30),
            body: Script::new(),
            body_span: Span::new(6, 12),
            result_var: Some("result".into()),
            options_var: Some("opts".into()),
            raw_args: Vec::new(),
            tokens: None,
        };
        if let Statement::Catch {
            result_var,
            options_var,
            ..
        } = &stmt
        {
            assert_eq!(result_var.as_deref(), Some("result"));
            assert_eq!(options_var.as_deref(), Some("opts"));
        }
    }

    #[test]
    fn try_with_handlers() {
        let stmt = Statement::Try {
            span: Span::new(0, 60),
            body: Script::new(),
            body_span: Span::new(4, 10),
            handlers: vec![TryHandler {
                kind: "on".into(),
                match_arg: "error".into(),
                var_name: Some("e".into()),
                options_var: None,
                body: Script::new(),
                body_span: Span::new(30, 40),
                fallthrough: false,
            }],
            finally_body: Some(Script::new()),
            finally_span: Some(Span::new(50, 58)),
            raw_args: Vec::new(),
        };
        if let Statement::Try {
            handlers,
            finally_body,
            ..
        } = &stmt
        {
            assert_eq!(handlers.len(), 1);
            assert_eq!(handlers[0].kind, "on");
            assert!(finally_body.is_some());
        }
    }

    #[test]
    fn switch_statement() {
        let stmt = Statement::Switch {
            span: Span::new(0, 80),
            subject: "$cmd".into(),
            subject_span: Span::new(7, 11),
            arms: vec![
                SwitchArm {
                    pattern: "start".into(),
                    pattern_span: Span::new(13, 18),
                    body: Some(Script::new()),
                    body_span: Some(Span::new(19, 22)),
                    fallthrough: false,
                },
                SwitchArm {
                    pattern: "stop".into(),
                    pattern_span: Span::new(23, 27),
                    body: None,
                    body_span: None,
                    fallthrough: true,
                },
            ],
            default_body: Some(Script::new()),
            default_span: Some(Span::new(50, 60)),
            mode: SwitchMode::Exact,
            nocase: false,
            raw_args: Vec::new(),
            patterns_braced: true,
        };
        if let Statement::Switch { arms, mode, .. } = &stmt {
            assert_eq!(arms.len(), 2);
            assert!(arms[1].fallthrough);
            assert_eq!(*mode, SwitchMode::Exact);
        }
    }

    #[test]
    fn clone_preserves_equality() {
        let stmt = Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        let cloned = stmt.clone();
        assert_eq!(stmt, cloned);
    }

    #[test]
    fn return_with_expr() {
        let stmt = Statement::Return {
            span: Span::new(0, 20),
            value: None,
            expr: Some(ExprNode::Binary {
                op: crate::expr_ast::BinOp::Add,
                left: Box::new(ExprNode::Var {
                    text: "$a".into(),
                    name: "a".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Literal {
                    text: "1".into(),
                    start: 5,
                    end: 6,
                }),
            }),
            braced: false,
        };
        if let Statement::Return { expr: Some(e), .. } = &stmt {
            let vars = e.vars();
            assert!(vars.contains("a"));
        }
    }

    // canonical_command_or_source helper

    #[test]
    fn canonical_command_or_source_falls_back_to_source_when_none() {
        let stmt = Statement::Call {
            span: Span::new(0, 3),
            command: "foo".into(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        assert_eq!(stmt.canonical_command_or_source(), "foo");
    }

    #[test]
    fn canonical_command_or_source_returns_canonical_when_some() {
        let stmt = Statement::Call {
            span: Span::new(0, 3),
            command: "foo".into(),
            canonical_command: Some("::ns::bar".into()),
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        assert_eq!(stmt.canonical_command_or_source(), "::ns::bar");
    }

    #[test]
    fn canonical_command_or_source_works_on_barrier() {
        let stmt = Statement::Barrier {
            span: Span::new(0, 6),
            reason: "eval".into(),
            command: "eval".into(),
            canonical_command: Some("eval".into()),
            args: vec!["body".into()],
            tokens: None,
        };
        assert_eq!(stmt.canonical_command_or_source(), "eval");
    }

    #[test]
    fn canonical_command_or_source_returns_empty_for_non_call_non_barrier() {
        let stmt = Statement::AssignConst {
            span: Span::new(0, 5),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        assert_eq!(stmt.canonical_command_or_source(), "");
    }
}
