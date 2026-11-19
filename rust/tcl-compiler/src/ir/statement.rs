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

//! The IR statement, its clause types and `switch` modes.

use super::command_tokens::CommandTokens;
use super::script::Script;
use super::synthetic_marker::{OriginalArgvInventoryDisposition, SyntheticMarker, WordEffectsHost};
use super::word_expr::WordExpr;
use crate::expr_ast::ExprNode;
use tcl_lexer::Span;
use tcl_registry::value_transfer::HandlerMatch;

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

/// A `try` handler clause — a clause of the command's clause plan that is
/// selected by its pattern word.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TryHandler {
    /// The vocabulary the handler's pattern word selects it by — the clause
    /// row's [`HandlerMatch`]: a completion code (`try`'s `on`) or an
    /// `-errorcode` prefix (`trap`).
    pub kind: HandlerMatch,
    /// Return code or error class pattern to match.
    pub match_arg: String,
    /// Parsed error-code prefix for a statically literal `trap` selector.
    ///
    /// `None` means either an `on` handler, a selector containing Tcl word
    /// substitutions, or a malformed Tcl list. Literal `$`, `[`, and
    /// backslashes remain ordinary element data when protected by braces or
    /// escapes; lowering decides that from source token shape before the
    /// decoded word loses the distinction.
    pub trap_pattern: Option<Vec<String>>,
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
    /// Pattern text: an element of a braced arm list is its decoded value,
    /// and a pattern given as a separate word is its spelling with the
    /// delimiters removed, as the subject of a `Statement::Switch` is —
    /// `value_transfer::recorded_word_value` gives that word's value.
    pub pattern: String,
    /// Whether this arm's pattern word was braced, so its value is literal.
    ///
    /// Per-arm, because the two arm forms differ per *word*: a single braced
    /// `{pat body …}` block holds literal list elements, while the multi-word
    /// form (`switch $s $pat {body}`) is a word each, and `{${x}}` there is
    /// literal while a bare `$pat` substitutes. The whole-switch
    /// `Statement::Switch::patterns_braced` cannot say that, and using it
    /// substituted a braced pattern.
    pub pattern_braced: bool,
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
    /// Whether the *list* word was a brace-string literal
    /// (`foreach n {a $b c} …`). Braces suppress substitution, so a
    /// `$b` inside such a word is a literal three-character list
    /// element, not a read of `b` — tclsh 8.6.14: `foreach n {a $b c}
    /// {puts $n}` prints `a`, `$b`, `c` with `b` undefined throughout.
    ///
    /// [`list_arg`](Self::list_arg) stores the word's *content* (braces
    /// already stripped), which cannot tell `{$b}` from `$b`; this flag
    /// is the same distinction [`Statement::AssignConst::braced`],
    /// [`Statement::Return::braced`] and
    /// [`CommandTokens::arg_is_braced_literal`] carry for their own
    /// words.
    pub list_braced: bool,
}

/// An IR statement — the building block of the compiler pipeline.
///
/// Each variant
/// carries a [`Span`] for position tracking. The span is resolved
/// to text or `(line, character)` via a `SourceMap` on demand.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    /// Exact native words evaluated by the ordinary runtime dispatcher.
    /// Their byte/source identities grant no catalogue or inline authority.
    NativeCall {
        /// Original complete command extent.
        span: Span,
        /// Original operands, including expansion and substitution structure.
        words: Vec<tcl_lexer::word_parts::NativeWord>,
    },
    /// Constant assignment: `set name value` where value is a literal.
    AssignConst {
        /// Source span of the full command.
        span: Span,
        /// Variable name.
        name: String,
        /// Whether the *name* word was a brace-string literal
        /// (`set {a($x)} v`). Braces suppress substitution, so an
        /// array-element target's key (`a($x)`) must be pushed
        /// LITERALLY (`$x` stays `$x`) rather than substituted — and,
        /// conversely, a *non*-braced name word still has its backslash
        /// escapes to decode before it is a variable name.
        /// Mirrors the `braced` precedent on [`Self::Return`].
        /// Defaults to `false` for every construction site except
        /// the `set` lowering hook (only it knows the source token
        /// kind); resolved by codegen's `store_target`, which
        /// `push_var_ref` / `emit_incr` then consume.
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
        /// Live command binding whose registry identity justified fusing the
        /// nested command. This is generic binding metadata (source spelling
        /// plus canonical implementation), not an `expr`-name flag: aliases
        /// therefore carry their own binding identity into bytecode.
        command_binding: Option<tcl_runtime_api::CommandBindingIdentity>,
        /// Original value word (`[expr ...]`) before this statement was
        /// fused. Codegen replays this through the ordinary `set` value path
        /// when the whole-module command-binding summary says `expr` no
        /// longer denotes its registry builtin. Keeping the exact word here
        /// preserves quoting, substitutions, and an overridden command's
        /// argv instead of trying to reconstruct Tcl source from the AST.
        fallback_value: String,
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
        /// Whether the *amount* word was a brace-string literal
        /// (`incr x {$n}`): its text is the amount itself, never a
        /// substitution — the program raises `expected integer but got
        /// "$n"` — mirroring [`Self::AssignConst::name_braced`] for the name.
        amount_braced: bool,
        /// Whether it is safe if the variable is uninitialised.
        safe_on_uninit: bool,
    },

    /// Standalone expression evaluation: `expr ...`.
    ExprEval {
        /// Source span.
        span: Span,
        /// Source binding whose registry implementation this specialised
        /// expression evaluation assumes.
        command_binding: tcl_runtime_api::CommandBindingIdentity,
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
        /// populated it or when the command is unknown to the registry.
        /// This spelling is presentation metadata; semantic consumers resolve
        /// the retained point proof through the shared invocation adapter.
        /// The lowerer populates
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
        /// Original return invocation and nested substitution lookup proofs.
        tokens: Option<CommandTokens>,
        /// Return value text, if any.
        value: Option<String>,
        /// Canonical source word for [`Self::Return::value`], when the simple
        /// return form retained one. Codegen uses it only to decide whether a
        /// nested local-name opcode can consume a final literal value.
        value_word: Option<WordExpr>,
        /// Return expression, if any (for `return [expr ...]`).
        expr: Option<ExprNode>,
        /// Exact expression source base, absent when decoded bytes lack an affine source mapping.
        expr_base: Option<u32>,
        /// Live command binding whose registry identity justified compiling a
        /// nested command substitution directly into [`expr`](Self::Return::expr).
        ///
        /// This is separate from the outer `return` command's binding: the
        /// latter remains owned by the statement source boundary, while this
        /// dependency belongs to the consumed nested command (`expr`, or a
        /// registry-proved alias for it).
        command_binding: Option<tcl_runtime_api::CommandBindingIdentity>,
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
        /// Registry-owned Tcl error context contributed by this inlined body.
        /// `None` for transparent passthrough and synthetic blocks.
        error_context: Option<tcl_registry::InlineBodyErrorContext>,
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
        raw_tokens: Option<Box<CommandTokens>>,
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
        /// The subject word as the source spells it, its delimiters removed:
        /// a bare or quoted word's escapes are not decoded and a braced
        /// word's continuations are not collapsed, so this is a spelling, not
        /// a value. `value_transfer::recorded_word_value` gives the value the
        /// selection and the dispatch chain read it by.
        subject: String,
        /// `true` when the subject came from a braced word, so its value is
        /// literal and suppresses substitution.
        ///
        /// The sibling of [`Self::Switch::patterns_braced`], and asked for the
        /// same reason: codegen hands the subject to the expression emitter as
        /// a word, and a word that lost its braces on the way in is
        /// indistinguishable from a bare one. Without this,
        /// `switch -- {a[nosuchcmd]}` *ran* `nosuchcmd` where both oracles
        /// match the literal and take the default arm. Defaults to `false` —
        /// an unbraced subject substitutes, which is the common form.
        subject_braced: bool,
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
        /// Per-[`Self::Switch::raw_args`] "this word was braced" flags, for the
        /// generic-invoke fallback the opaque modes take.
        ///
        /// `raw_args` are *values*, and a value cannot say how it was written,
        /// so the emitter that pushes them has no way to tell a braced word's
        /// literal content from a bare word that still substitutes. Fabricating
        /// the flags — marking only a single braced arm list — sends everything
        /// else out substituting: `switch -glob -- {a[bc]d}` runs `bc`, and the
        /// pattern `{a\[bc\]d}` decodes to `a[bc]d` and matches as a character
        /// class, so it matches `abd` and not the literal both oracles match.
        ///
        /// Empty when the flags are unknown (a hand-built statement), which the
        /// emitter reads as "none braced" — the prior behaviour.
        raw_arg_braced: Vec<bool>,
        /// Per-[`Self::Switch::raw_args`] "this word was double-quoted" flags,
        /// beside [`Self::Switch::raw_arg_braced`]: a word neither braced nor
        /// quoted was spelled bare, which is how 9.1b0's byte-compiled
        /// `switch` recognises a fall-through body — so the selection record
        /// reads a quoted `-` apart from a bare one. Empty when unknown.
        raw_arg_quoted: Vec<bool>,
        /// The command as the source spells it: `switch`, or another
        /// case-list command lowered through the same hook (`case`). The
        /// generic invoke of an opaque form names it.
        command: String,
        /// `true` when the arms came from a single braced
        /// `{pat body …}` block — patterns are literal list elements
        /// with no substitution. `false` when supplied as separate
        /// words (`switch $s $pat {body}`), where each pattern
        /// undergoes normal `$var` / `[cmd]` runtime substitution
        /// before matching. Defaults to `true`, the canonical braced form.
        patterns_braced: bool,
    },
}

/// Traverse only retained child scripts; opaque operands are never reparsed.
pub(crate) fn statements_have_opaque_native_accesses<'a>(
    statements: impl IntoIterator<Item = &'a Statement>,
) -> bool {
    let mut nested = Vec::new();
    for statement in statements {
        if statement.has_opaque_native_accesses() {
            return true;
        }
        nested.extend(statement.child_scripts());
    }
    while let Some(script) = nested.pop() {
        for statement in &script.statements {
            if statement.has_opaque_native_accesses() {
                return true;
            }
            nested.extend(statement.child_scripts());
        }
    }
    false
}

impl Statement {
    /// The statement standing where code the module cannot see runs
    /// ([`SyntheticMarker::UnseenCall`]): it carries no name and is no command
    /// to run. The SSA gives the names live past it fresh versions and records
    /// the version each name holds where it stands.
    #[must_use]
    pub fn unseen_call_marker(span: Span) -> Self {
        Self::Call {
            span,
            command: "<unseen-call>".to_owned(),
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(CommandTokens::marker(SyntheticMarker::UnseenCall)),
            foreach_groups: None,
        }
    }

    /// The definition point of what the `[…]` substitutions in a statement's
    /// words do to the frame ([`SyntheticMarker::WordEffects`]): the names
    /// they write and those they read before writing, for the statement
    /// `host` places. It carries no command to run.
    #[must_use]
    pub fn word_effects(
        span: Span,
        (defs, reads): (Vec<String>, Vec<String>),
        host: WordEffectsHost,
    ) -> Self {
        Self::Call {
            span,
            command: "<word-effects>".to_owned(),
            canonical_command: None,
            args: Vec::new(),
            defs,
            reads,
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(CommandTokens::marker(SyntheticMarker::WordEffects(host))),
            foreach_groups: None,
        }
    }

    /// Where the statement whose words this definition point states the
    /// effects of stands, when this is one ([`SyntheticMarker::WordEffects`]).
    #[must_use]
    pub fn word_effects_host(&self) -> Option<WordEffectsHost> {
        match self.synthetic_marker() {
            Some(SyntheticMarker::WordEffects(host)) => Some(host),
            _ => None,
        }
    }

    /// Return the synthetic marker attached to this statement, when it has
    /// one. Markers carry analysis-only effects beside their source command.
    #[must_use]
    pub fn synthetic_marker(&self) -> Option<SyntheticMarker> {
        match self {
            Self::Call { tokens, .. } | Self::Barrier { tokens, .. } => {
                tokens.as_ref().and_then(|tokens| tokens.synthetic)
            }
            _ => None,
        }
    }

    /// Whether native byte operands lack a named-variable/effect projection.
    /// Empty SSA read or definition maps for this statement do not prove that
    /// it leaves variables, command bindings or other runtime state unchanged.
    #[must_use]
    pub const fn has_opaque_native_accesses(&self) -> bool {
        matches!(self, Self::NativeCall { .. })
    }

    /// Retained child scripts in source/phase order. Only the original generic
    /// invocation owns an evaluated region; synthetic phase boundaries carry
    /// provenance without owning another copy of its scripts.
    #[must_use]
    pub fn child_scripts(&self) -> Vec<&Script> {
        match self {
            Self::Block { body, .. }
            | Self::UpFrame { body, .. }
            | Self::While { body, .. }
            | Self::Catch { body, .. }
            | Self::Foreach { body, .. } => vec![body],
            Self::If {
                clauses, else_body, ..
            } => clauses
                .iter()
                .map(|clause| &clause.body)
                .chain(else_body.iter())
                .collect(),
            Self::For {
                init, next, body, ..
            } => vec![init, next, body],
            Self::Try {
                body,
                handlers,
                finally_body,
                ..
            } => std::iter::once(body)
                .chain(handlers.iter().map(|handler| &handler.body))
                .chain(finally_body.iter())
                .collect(),
            Self::Switch {
                arms, default_body, ..
            } => arms
                .iter()
                .filter_map(|arm| arm.body.as_ref())
                .chain(default_body.iter())
                .collect(),
            Self::Call {
                tokens: Some(tokens),
                ..
            }
            | Self::Barrier {
                tokens: Some(tokens),
                ..
            } if tokens.synthetic.is_none() => {
                tokens.evaluated_body().map_or_else(Vec::new, |region| {
                    region.phases.iter().map(|phase| &phase.script).collect()
                })
            }
            _ => Vec::new(),
        }
    }

    /// Mutable projection of the same owned scripts as [`Self::child_scripts`].
    /// Visitors may change metadata while preserving the retained phase graph.
    pub fn child_scripts_mut(&mut self) -> Vec<&mut Script> {
        match self {
            Self::Block { body, .. }
            | Self::UpFrame { body, .. }
            | Self::While { body, .. }
            | Self::Catch { body, .. }
            | Self::Foreach { body, .. } => vec![body],
            Self::If {
                clauses, else_body, ..
            } => clauses
                .iter_mut()
                .map(|clause| &mut clause.body)
                .chain(else_body.iter_mut())
                .collect(),
            Self::For {
                init, next, body, ..
            } => vec![init, next, body],
            Self::Try {
                body,
                handlers,
                finally_body,
                ..
            } => std::iter::once(body)
                .chain(handlers.iter_mut().map(|handler| &mut handler.body))
                .chain(finally_body.iter_mut())
                .collect(),
            Self::Switch {
                arms, default_body, ..
            } => arms
                .iter_mut()
                .filter_map(|arm| arm.body.as_mut())
                .chain(default_body.iter_mut())
                .collect(),
            Self::Call {
                tokens: Some(tokens),
                ..
            }
            | Self::Barrier {
                tokens: Some(tokens),
                ..
            } if tokens.synthetic.is_none() => tokens
                .evaluated_body
                .as_deref_mut()
                .filter(|region| region.valid())
                .map_or_else(Vec::new, |region| {
                    region
                        .phases
                        .iter_mut()
                        .map(|phase| &mut phase.script)
                        .collect()
                }),
            _ => Vec::new(),
        }
    }

    /// Original invocation words when retained by lowering. Typed operations
    /// without this carrier retain their consumed proof in `CommandBindingSite`.
    #[must_use]
    pub fn tokens(&self) -> Option<&CommandTokens> {
        match self {
            Self::AssignValue { tokens, .. }
            | Self::Return { tokens, .. }
            | Self::Call { tokens, .. }
            | Self::Barrier { tokens, .. }
            | Self::Block { tokens, .. }
            | Self::Catch { tokens, .. }
            | Self::UpFrame { tokens, .. }
            | Self::While {
                raw_tokens: tokens, ..
            }
            | Self::Foreach {
                raw_tokens: tokens, ..
            } => tokens.as_ref(),
            Self::For { raw_tokens, .. } => raw_tokens.as_deref(),
            _ => None,
        }
    }

    /// Whether this statement represents an invocation that code generation
    /// and command-effect analyses must execute.
    ///
    /// Unseen calls, registry barriers and upvar invalidations widen facts
    /// only; the original invocation beside them performs dispatch. Other
    /// synthetic statements retain their existing executable semantics.
    #[must_use]
    pub fn is_executable_invocation(&self) -> bool {
        !matches!(
            self.synthetic_marker(),
            Some(
                SyntheticMarker::UnseenCall
                    | SyntheticMarker::RegistryBarrier
                    | SyntheticMarker::UpvarInvalidate
            )
        )
    }

    /// Classify only canonical, source-free widening phases. A marker attached
    /// to actual argv or changed token metadata cannot erase their inventory.
    #[must_use]
    pub fn original_argv_inventory_disposition(&self) -> OriginalArgvInventoryDisposition {
        let (Self::Call {
            args,
            canonical_command,
            tokens: Some(tokens),
            ..
        }
        | Self::Barrier {
            args,
            canonical_command,
            tokens: Some(tokens),
            ..
        }) = self
        else {
            return OriginalArgvInventoryDisposition::SourceRequired;
        };
        let Some(
            marker @ (SyntheticMarker::UnseenCall
            | SyntheticMarker::RegistryBarrier
            | SyntheticMarker::UpvarInvalidate
            | SyntheticMarker::GlobalFrameScript
            | SyntheticMarker::CallerFrameOpaque),
        ) = tokens.synthetic
        else {
            return OriginalArgvInventoryDisposition::SourceRequired;
        };
        if args.is_empty()
            && canonical_command.is_none()
            && tokens == &CommandTokens::marker(marker)
        {
            OriginalArgvInventoryDisposition::EffectOnly(marker)
        } else {
            OriginalArgvInventoryDisposition::SourceRequired
        }
    }

    /// Lowering diagnosed a script operand that runs without a represented CFG.
    pub(crate) const OPAQUE_SCRIPT_BODY: &'static str = "unsupported body command";

    /// A selected command can run its script operand, regardless of whether
    /// the command's own frame is declared independent of its caller.
    pub(crate) fn is_opaque_script_body(&self) -> bool {
        matches!(self, Self::Barrier { reason, .. } if reason == Self::OPAQUE_SCRIPT_BODY)
    }

    /// Whether this is the barrier a `return` lowers to when it leaves the
    /// procedure or its completion is not known — `return` with options
    /// ([`crate::lowering::hooks::control::try_lower_return`]) or with an
    /// expanded word. The CFG takes it for a procedure exit, and it writes
    /// no variable.
    #[must_use]
    pub fn is_return_barrier(&self) -> bool {
        matches!(
            self,
            Self::Barrier { reason, .. }
                if matches!(reason.as_str(), "return with options" | "return with expansion")
        )
    }

    /// Mutable invocation carrier, for owner-controlled provenance transfer.
    pub fn tokens_mut(&mut self) -> Option<&mut CommandTokens> {
        match self {
            Self::AssignValue { tokens, .. }
            | Self::Return { tokens, .. }
            | Self::Call { tokens, .. }
            | Self::Barrier { tokens, .. }
            | Self::Block { tokens, .. }
            | Self::Catch { tokens, .. }
            | Self::UpFrame { tokens, .. }
            | Self::While {
                raw_tokens: tokens, ..
            }
            | Self::Foreach {
                raw_tokens: tokens, ..
            } => tokens.as_mut(),
            Self::For { raw_tokens, .. } => raw_tokens.as_deref_mut(),
            _ => None,
        }
    }
    /// A source command range eligible for command-level editing. Typed CFG
    /// boundaries may retain a diagnostic span on an enclosing operand, but
    /// they are not commands and must never delete that operand's delimiters.
    /// Consumers still need their usual reachability/effect and origin proof.
    #[must_use]
    pub fn source_edit_span(&self) -> Option<Span> {
        if self.has_opaque_native_accesses() {
            return None;
        }
        if self.tokens().is_some_and(|tokens| {
            tokens.synthetic.is_some()
                || tokens
                    .source_binding
                    .as_ref()
                    .and_then(|binding| binding.source_origin())
                    .is_some_and(|origin| {
                        matches!(
                            origin.kind(),
                            crate::command_binding::SourceOriginKind::Derived { .. }
                        )
                    })
        }) {
            return None;
        }
        let span = self.span();
        (!span.is_empty()).then_some(span)
    }

    /// Return the source span of this statement, including diagnostic spans
    /// retained by synthetic boundaries. Use `source_edit_span` for editing.
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
            | Self::NativeCall { span, .. }
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

    /// Return the recorded canonical spelling, or the written head, for display
    /// and compatibility metadata. This does not prove implementation identity.
    /// Semantic consumers must use `registry_invocation::resolved_statement_invocation`
    /// so an unresolved source binding cannot recover catalogue semantics.
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
