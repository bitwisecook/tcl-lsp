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

//! Per-command lowering specialisations.
//!
//! Each function takes a [`LoweringCommand`] (the parsed command context)
//! and returns `Some(Statement)` if the command is handled, or `None` to
//! fall through to the default [`Statement::Call`] path.
//!
//! The per-command logic lives in [`crate::lowering::hooks`] (one
//! file per command); this module retains the dispatcher, the
//! [`LoweringCommand`] context type, and the shared helpers
//! (`make_call`, `extract_single_expr_arg`, `parse_decimal_int`).

use std::collections::HashSet;

use tcl_lexer::Span;
use tcl_registry::CommandRegistry;
use tcl_registry::hooks::LoweringHookId;

use crate::alias::{CommandAliasMap, expr_alias_names};
use crate::expr_parser::parse_expr_for_profile;
use crate::ir::{CommandTokens, Statement};

/// Parsed command context passed to lowering hooks.
pub struct LoweringCommand<'a> {
    /// Source span of the full command.
    pub span: Span,
    /// Command name (first word).
    pub name: &'a str,
    /// Arguments (words after the command name).
    pub args: &'a [String],
    /// Whether each word is a single token.
    pub single_token_word: &'a [bool],
    /// `{*}` expansion markers, if any word uses expansion.
    pub expand_word: Option<&'a [bool]>,
    /// Snapshot of parsed tokens for downstream passes.
    pub tokens: Option<CommandTokens>,
    /// Per-arg token kinds.
    /// Uses a simplified enum since we only check STR/ESC/CMD.
    pub arg_kinds: &'a [ArgTokenKind],
    /// The document's analysis dialect, or `None` for plain Tcl.
    ///
    /// Threaded into [`crate::expr_parser::parse_expr`] by the hooks that
    /// parse an expression (`expr`, `return [expr …]`, `set x [expr …]`), so
    /// an iRules word operator (`contains`, `starts_with`, …) parses as the
    /// operator it is rather than falling back to
    /// [`crate::expr_ast::ExprNode::Raw`] — which no downstream fold can
    /// evaluate.
    pub dialect: Option<&'static tcl_dialect::DialectProfile>,
}

impl LoweringCommand<'_> {
    /// Whether **argument** `idx` (0-based) is a brace-quoted literal word.
    ///
    /// Braces suppress every substitution, so such a word's content is a
    /// *literal* variable name where the command's role says a name goes:
    /// `unset {$n}` destroys the variable called `$n`, not `n` (issue #1078).
    /// The de-braced `args` text cannot show that; the word's own token kind
    /// can — `arg_kinds` is arg-indexed, `single_token_word` word-indexed
    /// (index 0 is the command word).  Mirrors
    /// [`crate::ir::CommandTokens::arg_is_braced_literal`] for the hook view.
    #[must_use]
    pub fn arg_is_braced_literal(&self, idx: usize) -> bool {
        self.arg_kinds.get(idx) == Some(&ArgTokenKind::Str)
            && self.single_token_word.get(idx + 1).copied().unwrap_or(true)
    }

    /// Whether **argument** `idx` (0-based) is a single substitution-free
    /// literal word — a `{braced}` (`Str`) or a plain bareword (`Esc`), the
    /// two shapes whose value is the text exactly as spelled.
    ///
    /// The hook-view sibling of the lowering's own `seg_word_is_static_literal`
    /// body gate (issue #1375): a word built from several tokens (`x$y`,
    /// `a($i)`, or — under a grammar with no `{*}` expansion — `{*}$n`) still
    /// reports a literal *representative* kind while its value is computed at
    /// run time, so the single-token flag is the half that carries the answer.
    ///
    /// Backslash escapes stay literal: `\$x` is one `Esc` token naming the
    /// variable `$x`, which no substitution produced.
    #[must_use]
    pub fn arg_is_static_literal(&self, idx: usize) -> bool {
        matches!(
            self.arg_kinds.get(idx),
            Some(ArgTokenKind::Str | ArgTokenKind::Esc)
        ) && self.single_token_word.get(idx + 1).copied().unwrap_or(true)
    }
}

/// Simplified token kind for hook arg inspection.
///
/// We only need to distinguish braced strings, plain words, and
/// command substitutions in lowering hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgTokenKind {
    /// Braced string `{...}`.
    Str,
    /// Plain word (bare text, possibly with backslash escapes).
    Esc,
    /// Command substitution `[...]`.
    Cmd,
    /// Variable reference `$...`.
    Var,
    /// JimTcl's `$(…)` sugar. The segmenter spells the word `$(body)` and
    /// the body is an **expression**, not a script — `set x $($a*2)` is
    /// `set x [expr {$a*2}]` — so the hooks that lower a bracketed `expr`
    /// lower this the same way instead of leaving the word opaque.
    ExprSugar,
    /// Any other token type.
    Other,
}

/// Try to lower a command via a registry-described hook.
///
/// Resolves `cmd` against the registry, looks up the lowering hook
/// identifier on the matched [`tcl_registry::CommandSpec`] /
/// [`tcl_registry::SubCommand`], and dispatches to the per-hook
/// algorithm. Returns `Some(statement)` if a hook handled the
/// command; `None` to fall through to the default [`Statement::Call`] path.
#[must_use]
pub fn try_lower_hook(
    cmd: &LoweringCommand<'_>,
    aliases: &CommandAliasMap,
    registry: &CommandRegistry,
    context: Option<&tcl_registry::model::ResolvedContext>,
    safe_on_uninit: bool,
) -> Option<Statement> {
    let arg_refs: Vec<&str> = cmd.args.iter().map(String::as_str).collect();
    let resolved =
        tcl_registry::model::resolve_invocation_in_context(registry, context, cmd.name, &arg_refs)?;
    let hook = resolved.semantics.lowering_hook?;
    dispatch_lowering_hook(hook, cmd, aliases, safe_on_uninit)
}

/// Dispatch a typed [`LoweringHookId`] to its implementation.
///
/// Public so external dispatchers (tests, future LSP feature
/// providers, peephole experiments) can reuse the same per-hook
/// implementations without duplicating the table. Per-command
/// algorithms now live under [`crate::lowering::hooks`]; this
/// function is the single point that maps registry-typed hook IDs
/// to the implementation modules.
#[must_use]
pub fn dispatch_lowering_hook(
    hook: LoweringHookId,
    cmd: &LoweringCommand<'_>,
    aliases: &CommandAliasMap,
    safe_on_uninit: bool,
) -> Option<Statement> {
    match hook {
        LoweringHookId::Expr => crate::lowering::hooks::control::try_lower_expr(cmd),
        LoweringHookId::Return => Some(crate::lowering::hooks::control::try_lower_return(
            cmd, aliases,
        )),
        LoweringHookId::Set => Some(lower_set(cmd, aliases)),
        LoweringHookId::Incr => Some(crate::lowering::hooks::incr::try_lower_incr(
            cmd,
            safe_on_uninit,
        )),
        LoweringHookId::AppendOrLappend => lower_append_lappend(cmd, safe_on_uninit),
        LoweringHookId::Unset => Some(lower_unset(cmd)),
        LoweringHookId::Global => lower_global(cmd),
        LoweringHookId::Variable => Some(lower_variable(cmd)),
        LoweringHookId::Upvar => lower_upvar(cmd),
        // Structured-command hooks: the lowerer's per-command
        // methods (`lower_proc`, `lower_when`, `lower_if`,
        // `lower_switch`, `lower_for`, `lower_while`,
        // `lower_foreach`, `lower_catch`, `lower_try`, `lower_dict`,
        // `try_lower_eval_static`, `try_lower_uplevel_static`,
        // `lower_namespace_eval`) require `&mut self` to thread the
        // const-map / proc-depth / dead-code-depth / module state
        // through; the static dispatcher here can't call them.
        // Return `None` so `lower_command` dispatches to the
        // matching method directly.
        LoweringHookId::Proc
        | LoweringHookId::When
        | LoweringHookId::NamespaceEval
        | LoweringHookId::If
        | LoweringHookId::Switch
        | LoweringHookId::For
        | LoweringHookId::While
        | LoweringHookId::Foreach
        | LoweringHookId::Lmap
        | LoweringHookId::ForeachLine
        | LoweringHookId::Catch
        | LoweringHookId::Try
        | LoweringHookId::Dict
        | LoweringHookId::Eval
        | LoweringHookId::Uplevel
        | LoweringHookId::Apply
        | LoweringHookId::ArrayFor => None,
    }
}

/// Whether this command has `{*}` expansion on any argument.
///
/// `pub(crate)` so per-command hook modules under
/// [`crate::lowering::hooks`] can share the single canonical
/// expansion check rather than re-implementing it inline (which
/// would drift over time as the `LoweringCommand` shape evolves).
pub(crate) fn has_expansion(cmd: &LoweringCommand<'_>) -> bool {
    cmd.expand_word.is_some_and(|ew| ew.iter().any(|&e| e))
}

/// Absolute source offset of a word's *content* first byte, when the word is
/// a single token whose content `text` is a verbatim source slice — the base
/// that maps expression-AST leaf offsets (relative to the parsed text) back
/// to absolute source positions.
///
/// The lexer's inner-end span convention makes the opening-delimiter width
/// recoverable without a `content_offset`: `span_len - text_len` is `0` for a
/// bare word and `1` for a braced / quoted / bracketed one (whose span covers
/// the opener plus the content, ending exclusively at the closer).  Any other
/// difference means the text was reconstructed (`${x}` re-braced by the
/// segmenter, an empty `{}`, a multi-token word) and no verbatim mapping
/// exists, so the result is `None`.
pub(crate) fn word_content_base(span: Span, single: bool, text: &str) -> Option<u32> {
    if !single {
        return None;
    }
    let span_len = span.end().checked_sub(span.start())?;
    let text_len = u32::try_from(text.len()).ok()?;
    let delta = span_len.checked_sub(text_len)?;
    (delta <= 1).then(|| span.start() + delta)
}

// expr
//
// Moved to `crate::lowering::hooks::control::try_lower_expr`. The
// dispatcher above delegates the `"expr"` case to the per-hook
// module.

// return
//
// Moved to `crate::lowering::hooks::control::try_lower_return`. The
// dispatcher above delegates the `"return"` case to the per-hook
// module.

// set

fn lower_set(cmd: &LoweringCommand<'_>, aliases: &CommandAliasMap) -> Statement {
    if has_expansion(cmd) || cmd.args.len() != 2 {
        return make_call(cmd);
    }

    // The variable NAME must be a compile-time literal. A *substituted* name
    // (`set $x v`, `set [f] v`) is a dynamic store: folding it to a const/value
    // assignment would create a local named after the source text (e.g. `${x}`)
    // instead of storing through the runtime-resolved name. Fall back to the
    // general command path, which emits `STORE_STK` over the substituted name.
    if !matches!(
        cmd.arg_kinds.first(),
        Some(ArgTokenKind::Str | ArgTokenKind::Esc)
    ) {
        return make_call(cmd);
    }

    let name = &cmd.args[0];
    let value = &cmd.args[1];

    // The name word is a brace-string literal (`set {a($x)} v`) when its arg
    // token is `Str` and it is a single token. Braces suppress substitution, so
    // codegen must push an array-element key (`a($x)`) LITERALLY rather than
    // substitute it. Thread this through every assignment shape `set` lowers to.
    let name_braced = matches!(cmd.arg_kinds.first(), Some(ArgTokenKind::Str))
        && cmd.single_token_word.get(1).copied().unwrap_or(false);

    // A *compound* name word computes the name too, and the arg-kind check
    // above cannot see it: only the word's representative token is consulted,
    // so under a grammar with no `{*}` expansion (8.4, iRules) `set {*}$n 1`
    // reads as a `Str` word while its value is the literal `*` welded to
    // whatever `$n` holds (issue #1484). Every assignment shape below is a
    // *static* store — `AssignConst` / `AssignValue` / `AssignExpr` names are
    // static by contract and `dynamic_names::scan_statement` never inspects
    // them — so a computed name must stay a generic `Call`, the form
    // `scan_command` does inspect and which raises the dynamic-write barrier
    // the value-motion passes need.
    //
    // Only a word that really substitutes is examined, because
    // `names_a_dynamic_variable` reads text alone: `set \$x 1` is one literal
    // token naming the variable `$x`, and `set {$n} 1` names `$n`, neither of
    // which substitutes anything. Conversely a computed array *key*
    // (`set a($i) 1`) is not a computed name — the array is named statically
    // and codegen's `push_array_key` builds the key at run time — which is the
    // same line `names_a_dynamic_variable` already draws for `scan_command`.
    if !cmd.arg_is_static_literal(0) && crate::dynamic_names::names_a_dynamic_variable(name) {
        return make_call(cmd);
    }

    // Content span of the value word when it is a verbatim source slice —
    // the writable provenance for command-name-in-variable rename.
    let value_span = cmd.tokens.as_ref().and_then(|t| {
        let base = word_content_base(
            *t.argv.get(2)?,
            t.single_token_word.get(2).copied().unwrap_or(false),
            value,
        )?;
        Some(Span::new(base, base + u32::try_from(value.len()).ok()?))
    });

    // Check if value arg is a single token.
    if cmd.single_token_word.len() >= 3 && cmd.single_token_word[2] && cmd.arg_kinds.len() >= 2 {
        match cmd.arg_kinds[1] {
            ArgTokenKind::Str => {
                return Statement::AssignConst {
                    span: cmd.span,
                    name: name.clone(),
                    name_braced,
                    value: value.clone(),
                    value_span,
                };
            }
            ArgTokenKind::Esc => {
                // Only fold to the parsed decimal form when the
                // source spelling matches the canonical decimal
                // form **exactly** — including leading / trailing
                // whitespace.  ``set arg 0005`` must store
                // ``0005`` verbatim, and ``set x " 5"`` must store
                // ``" 5"`` verbatim — Tcl preserves the source
                // string repr; the value only shimmers to int 5
                // when used as an integer.  Folding eagerly here
                // would destroy the original spelling and break
                // ``puts $arg``, ``string length $arg``, and
                // ``expr "0005"`` which all care about it.
                // ``parse_decimal_int`` itself trims its input, so
                // comparing against ``value.trim()`` would still
                // accept leading-whitespace shapes.  Tighten to
                // ``value == int_val`` so any whitespace or sign
                // form different from the canonical produces no
                // fold.
                if let Some(int_val) = parse_decimal_int(value)
                    && value.as_str() == int_val
                {
                    return Statement::AssignConst {
                        span: cmd.span,
                        name: name.clone(),
                        name_braced,
                        value: int_val,
                        value_span,
                    };
                }
                let needs_backsubst = value.contains('\\');
                return Statement::AssignValue {
                    span: cmd.span,
                    name: name.clone(),
                    name_braced,
                    value: value.clone(),
                    value_needs_backsubst: needs_backsubst,
                    tokens: cmd.tokens.clone(),
                };
            }
            ArgTokenKind::Cmd => {
                // The segmenter wraps CMD tokens as [text]; strip the
                // brackets so extract_single_expr_arg sees "expr {arg}".
                let inner = value
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .unwrap_or(value);
                let alias_names = expr_alias_names(aliases);
                if let Some((expr_arg, rel_base)) = extract_single_expr_arg_with_config(
                    inner,
                    &alias_names,
                    tcl_lexer::LexerConfig::for_profile(cmd.dialect),
                ) {
                    let expr = parse_expr_for_profile(&expr_arg, cmd.dialect);
                    // Anchor the expression text absolutely when both the
                    // `[...]` value word's content and the expr word within
                    // it are verbatim slices: absolute = the bracketed
                    // word's content start + the expr word's offset in it.
                    let inner_base = cmd.tokens.as_ref().and_then(|t| {
                        word_content_base(
                            *t.argv.get(2)?,
                            t.single_token_word.get(2).copied().unwrap_or(false),
                            inner,
                        )
                    });
                    let expr_base = match (inner_base, rel_base) {
                        (Some(b), Some(r)) => Some(b + r),
                        _ => None,
                    };
                    return Statement::AssignExpr {
                        span: cmd.span,
                        name: name.clone(),
                        name_braced,
                        expr,
                        expr_base,
                    };
                }
            }
            ArgTokenKind::ExprSugar => {
                // JimTcl `$(…)`: the body is an expression, so this is the
                // `AssignExpr` a `[expr {…}]` value produces — dataflow,
                // taint and typing then see the operands instead of an
                // opaque word. The body is anchored absolutely when the
                // value word is a single verbatim token: the lexer's span
                // for the sugar covers the `$(` opener plus the body and
                // ends before the closer, so the body starts two bytes in.
                if let Some(body) = value.strip_prefix("$(").and_then(|s| s.strip_suffix(')')) {
                    let expr = parse_expr_for_profile(body, cmd.dialect);
                    let expr_base = cmd.tokens.as_ref().and_then(|t| {
                        let span = *t.argv.get(2)?;
                        if !t.single_token_word.get(2).copied().unwrap_or(false) {
                            return None;
                        }
                        let span_len = span.end().checked_sub(span.start())?;
                        let body_len = u32::try_from(body.len()).ok()?;
                        (span_len == body_len.checked_add(2)?).then(|| span.start() + 2)
                    });
                    return Statement::AssignExpr {
                        span: cmd.span,
                        name: name.clone(),
                        name_braced,
                        expr,
                        expr_base,
                    };
                }
            }
            _ => {}
        }
    }

    Statement::AssignValue {
        span: cmd.span,
        name: name.clone(),
        name_braced,
        value: value.clone(),
        value_needs_backsubst: false,
        tokens: cmd.tokens.clone(),
    }
}

// incr
//
// Moved to `crate::lowering::hooks::incr::try_lower_incr`. The
// dispatcher above delegates the `"incr"` case to the per-hook
// module.

// append / lappend

fn lower_append_lappend(cmd: &LoweringCommand<'_>, safe_on_uninit: bool) -> Option<Statement> {
    if cmd.args.is_empty() {
        return None;
    }
    // Record a concrete def of the target only when the variable NAME is a
    // compile-time literal and no `{*}` expansion shifts the argv — the same
    // gate `set`/`incr` apply. `append {*}$args` makes `args[0]` the *expanded
    // list being read*, not a write target; `append $x foo` writes through a
    // substituted name whose source text is not the variable. Recording
    // `defs=[normalise(args[0])]` in either case fabricates a def of the wrong
    // variable. Leaving `defs` empty keeps the write out of
    // SSA/def-use as a concrete name — the registry's arg-role / side-effect
    // model still resolves it, and `resolve_place` yields Unknown, matching
    // `incr $x`.
    let name_is_literal = !has_expansion(cmd)
        && matches!(
            cmd.arg_kinds.first(),
            Some(ArgTokenKind::Str | ArgTokenKind::Esc)
        );
    let defs = if name_is_literal {
        vec![
            crate::naming::normalise_var_name_braced(&cmd.args[0], cmd.arg_is_braced_literal(0))
                .to_owned(),
        ]
    } else {
        vec![]
    };
    Some(Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs,
        reads: vec![],
        reads_own_defs: true,
        safe_on_uninit,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
    })
}

// unset

fn lower_unset(cmd: &LoweringCommand<'_>) -> Statement {
    let mut i = 0;
    let mut nocomplain = false;
    // `unset` recognises only `-nocomplain` (skippable, repeatable) and `--`
    // (terminator).  Any other leading word — including a variable literally
    // named `-foo` (`unset -foo bar`) — ends option parsing and is a destroyed
    // variable, matching tclsh 8.6/9.0.
    while i < cmd.args.len() {
        match cmd.args[i].as_str() {
            "-nocomplain" => {
                nocomplain = true;
                i += 1;
            }
            "--" => {
                i += 1;
                break;
            }
            _ => break,
        }
    }
    let var_names: Vec<String> = cmd.args[i..]
        .iter()
        .enumerate()
        .map(|(k, a)| {
            crate::naming::normalise_var_name_braced(a, cmd.arg_is_braced_literal(i + k)).to_owned()
        })
        .collect();
    Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs: var_names,
        reads: vec![],
        reads_own_defs: !nocomplain,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
    }
}

// global

fn lower_global(cmd: &LoweringCommand<'_>) -> Option<Statement> {
    if cmd.args.is_empty() {
        return None;
    }
    let var_names: Vec<String> = cmd
        .args
        .iter()
        .enumerate()
        .map(|(i, a)| {
            crate::naming::normalise_var_name_braced(a, cmd.arg_is_braced_literal(i)).to_owned()
        })
        .collect();
    Some(Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs: var_names,
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
    })
}

// variable

fn lower_variable(cmd: &LoweringCommand<'_>) -> Statement {
    let var_names: Vec<String> = cmd
        .args
        .iter()
        .enumerate()
        .step_by(2)
        .map(|(i, a)| {
            crate::naming::normalise_var_name_braced(a, cmd.arg_is_braced_literal(i)).to_owned()
        })
        .collect();
    Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs: var_names,
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
    }
}

// upvar

fn lower_upvar(cmd: &LoweringCommand<'_>) -> Option<Statement> {
    if cmd.args.len() < 2 {
        return None;
    }
    let has_level = cmd.args[0]
        .trim_start_matches('-')
        .chars()
        .all(|c| c.is_ascii_digit())
        || cmd.args[0].starts_with('#');
    let start = usize::from(has_level);
    // `upvar ?level? caller local ?caller local ...?`.  A `local` is a clean
    // def only when its `caller` target is a *literal* name: a dynamic
    // `$name` / `[cmd]` target may resolve to a non-existent caller variable,
    // in which case the alias is a no-op and reading `$local` errors — so the
    // local is possibly-unset and must not be recorded as a def (read-before-set
    // fires on an unconditional read).
    let rest = &cmd.args[start..];
    let mut my_vars: Vec<String> = Vec::new();
    let mut i = 0;
    while i + 1 < rest.len() {
        let caller = &rest[i];
        if !caller.starts_with('$') && !caller.starts_with('[') {
            my_vars.push(
                crate::naming::normalise_var_name_braced(
                    &rest[i + 1],
                    cmd.arg_is_braced_literal(start + i + 1),
                )
                .to_owned(),
            );
        }
        i += 2;
    }
    Some(Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs: my_vars,
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
    })
}

// Helpers

/// Build a generic `Statement::Call` from a lowering command.
///
/// Shared with the per-command hook modules under
/// [`crate::lowering::hooks`] so every fallback site produces the
/// same `Call` shape and stays in sync if `Statement::Call` grows
/// new fields.
pub(crate) fn make_call(cmd: &LoweringCommand<'_>) -> Statement {
    Statement::Call {
        span: cmd.span,
        command: cmd.name.into(),
        canonical_command: None,
        args: cmd.args.to_vec(),
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: cmd.tokens.clone(),
        foreach_groups: None,
    }
}

/// Extract the single expression argument from a `[expr ...]` command.
///
/// Returns `Some((expr_text, content_base))` if the text is
/// `expr <one-word>` (or an expr alias), `None` otherwise.
/// `content_base` is the offset of the expression text's first byte
/// *within `text`* when the word content is a verbatim slice of it
/// (see [`word_content_base`]), letting callers anchor expression-AST
/// leaf offsets back to the source.
///
/// `pub(crate)` so per-command hook modules under
/// [`crate::lowering::hooks`] (e.g. `control::try_lower_return`) can
/// share the same single-arg-extraction logic as `lower_set` here.
pub(crate) fn extract_single_expr_arg(
    text: &str,
    expr_aliases: &HashSet<String>,
) -> Option<(String, Option<u32>)> {
    // dialect-drift-ok: compatibility shim for the two call sites outside this
    // lane (`lowering::hooks::control`, `ssa`); the dialect-aware form below is
    // what the hook path uses.
    extract_single_expr_arg_with_config(text, expr_aliases, tcl_lexer::LexerConfig::default())
}

/// [`extract_single_expr_arg`] under the document's own
/// [`tcl_lexer::LexerConfig`] — the form every caller that holds the
/// document's dialect must use, so the `[expr …]` interior is re-lexed under
/// the grammar the document was lexed with rather than the Tcl 9.x default.
pub(crate) fn extract_single_expr_arg_with_config(
    text: &str,
    expr_aliases: &HashSet<String>,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, Option<u32>)> {
    use tcl_lexer::{Lexer, SourceMap, TokenType};

    let sm = SourceMap::new(text);
    let lexer = Lexer::with_config(text, config);
    let Ok(tokens) = lexer.tokenise_all() else {
        return None;
    };

    let mut words = Vec::new();
    let mut single = Vec::new();
    // Representative (first) token span per word, for content anchoring.
    let mut word_spans: Vec<Span> = Vec::new();
    // Whether each word is built only from *literal* tokens (`Esc`/`Str`). A
    // `Var`/`Cmd` token has its sigil/brackets stripped in `token_text`, so the
    // reconstructed word would lose them (`$e` → `e`); such an arg also needs the
    // second round of evaluation the runtime `expr` performs, so it must not fuse.
    let mut literal = Vec::new();
    let mut prev_is_sep = true;
    for tok in &tokens {
        match tok.kind {
            TokenType::Sep | TokenType::Eol | TokenType::Eof | TokenType::Comment => {
                prev_is_sep = true;
            }
            _ => {
                let t = sm.token_text(*tok);
                let is_lit = matches!(tok.kind, TokenType::Esc | TokenType::Str);
                if prev_is_sep {
                    words.push(t.to_owned());
                    single.push(true);
                    literal.push(is_lit);
                    word_spans.push(tok.span);
                } else if let Some(last) = words.last_mut() {
                    last.push_str(t);
                    if let Some(s) = single.last_mut() {
                        *s = false;
                    }
                    if let Some(l) = literal.last_mut() {
                        *l = *l && is_lit;
                    }
                } else {
                    words.push(t.to_owned());
                    single.push(true);
                    literal.push(is_lit);
                    word_spans.push(tok.span);
                }
                prev_is_sep = false;
            }
        }
    }

    if words.len() != 2 {
        return None;
    }
    let cmd_word = &words[0];
    if cmd_word != "expr" && !expr_aliases.contains(cmd_word.as_str()) {
        return None;
    }
    if !single[1] {
        return None;
    }
    // A braced `{$e}` (a `Str` token) keeps its `$` and is a real expression
    // operand, so it fuses; a bare `$e`/`[f]` must defer to the runtime `expr`.
    if !literal[1] {
        return None;
    }
    let base = word_content_base(word_spans[1], single[1], &words[1]);
    Some((words[1].clone(), base))
}

/// Validate text as a decimal integer, returning the source text if valid.
///
/// Rejects non-digit content, hex/octal prefixes, and leading zeros
/// (except for `0` itself). Returns the trimmed source text unchanged
/// to preserve the original representation for `AssignConst`.
fn parse_decimal_int(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let digits = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Reject leading zeros (007 is not a plain decimal integer in Tcl).
    if digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    Some(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The statement `src` lowers to under `dialect` — the configuration a
    /// real host builds, so the `{*}` grammar matches the document's.
    fn first_stmt_for_dialect(
        src: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> Statement {
        let registry = tcl_registry::model::ingress::static_context_for_profile(dialect).commands();
        let m = crate::lowering::lower_to_ir_with_config(
            src,
            registry,
            tcl_lexer::LexerConfig::from_grammar(dialect.grammar),
        );
        m.top_level.statements[0].clone()
    }

    /// Issue #1484 — a computed name may not wear a static-assign shape.
    ///
    /// Under a grammar with no `{*}` expansion the word `{*}$n` is the literal
    /// `*` welded to `$n`, whose *representative* token is the braced `{*}` —
    /// literal enough to slip past the arg-kind check, computed enough that
    /// `AssignConst`'s static-name contract is a lie.  It must stay a `Call`,
    /// the form `dynamic_names::scan_command` inspects.
    #[test]
    fn lower_set_refuses_a_computed_name_under_an_expansionless_grammar() {
        for dialect in ["tcl8.4", "f5-irules"] {
            for src in ["set {*}$n 1", "set x$n 1", "set pre[f] 1"] {
                let stmt = first_stmt_for_dialect(
                    src,
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                );
                assert!(
                    matches!(&stmt, Statement::Call { command, .. } if command == "set"),
                    "{dialect}: {src:?} must stay a Call, got {stmt:?}",
                );
            }
        }
    }

    /// The gate reads the *word*, not just its text: these names are spelled
    /// out (a backslash-escaped `$`, a brace-quoted `$n`) or name a static
    /// array whose key alone is computed, so they keep their typed assignment.
    #[test]
    fn lower_set_keeps_the_static_assign_for_a_spelled_out_name() {
        for dialect in ["tcl9.0", "tcl8.6", "tcl8.4", "f5-irules"] {
            for src in ["set x 1", "set {$n} 1", "set \\$x 1", "set a($i) 1"] {
                let stmt = first_stmt_for_dialect(
                    src,
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                );
                assert!(
                    matches!(
                        &stmt,
                        Statement::AssignConst { .. }
                            | Statement::AssignValue { .. }
                            | Statement::AssignExpr { .. }
                    ),
                    "{dialect}: {src:?} must keep its typed assignment, got {stmt:?}",
                );
            }
        }
    }

    /// Under 9.0 `{*}$n` is a real expansion, which `has_expansion` has always
    /// rejected — the #1484 gate must not be what decides this case.
    #[test]
    fn lower_set_leaves_the_expanded_name_word_on_its_existing_path() {
        for dialect in ["tcl9.0", "tcl8.6"] {
            let stmt = first_stmt_for_dialect(
                "set {*}$n 1",
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            let Statement::Call {
                command, tokens, ..
            } = &stmt
            else {
                panic!("{dialect}: expected Call, got {stmt:?}");
            };
            assert_eq!(command, "set");
            assert_eq!(
                tokens.as_ref().and_then(|t| t.expand_word.clone()),
                Some(vec![false, true, false]),
                "{dialect}: the word must still be marked as expanded",
            );
        }
    }

    #[test]
    fn lower_set_preserves_leading_zeros() {
        // ``set arg 0005`` must store ``0005`` verbatim — folding
        // to ``5`` would destroy the source-level repr that
        // ``puts $arg`` / ``string length $arg`` / ``expr "0005"``
        // depend on.
        // The leading-zero rejection in `parse_decimal_int` already
        // kicks the value to the `AssignValue` path; the
        // `int_val == value.trim()` guard is a redundant safety
        // check that catches any future loosening of
        // `parse_decimal_int`.
        let m = crate::lowering::lower_to_ir("set arg 0005", &CommandRegistry::build_default());
        let value = match &m.top_level.statements[0] {
            Statement::AssignConst { value, .. } | Statement::AssignValue { value, .. } => {
                value.clone()
            }
            other => panic!("expected AssignConst or AssignValue, got {other:?}"),
        };
        assert_eq!(value, "0005");
    }

    #[test]
    fn lower_set_preserves_leading_whitespace_in_value() {
        // ``parse_decimal_int`` trims its input, so the
        // ``int_val == value.trim()`` check in ``lower_set`` would
        // still accept leading-whitespace shapes; the tightened
        // ``value == int_val`` check rejects them and sends ``" 5"``
        // through the ``AssignValue`` path with the spelling
        // preserved.  Use the brace form so the segmenter passes the
        // literal text through without trimming on its end.
        let m = crate::lowering::lower_to_ir("set arg { 5}", &CommandRegistry::build_default());
        let value = match &m.top_level.statements[0] {
            Statement::AssignConst { value, .. } | Statement::AssignValue { value, .. } => {
                value.clone()
            }
            other => panic!("expected AssignConst or AssignValue, got {other:?}"),
        };
        assert!(
            value.starts_with(' '),
            "expected leading whitespace preserved, got {value:?}",
        );
    }

    #[test]
    fn lower_set_folds_canonical_decimal_form() {
        // Canonical decimal spelling — ``5`` parses to ``5`` and
        // can fold without losing repr fidelity.
        let m = crate::lowering::lower_to_ir("set arg 5", &CommandRegistry::build_default());
        if let Statement::AssignConst { value, .. } = &m.top_level.statements[0] {
            assert_eq!(value, "5");
        } else {
            panic!("expected AssignConst");
        }
    }

    /// The concrete def names of the first lowered statement.
    fn first_stmt_defs(src: &str) -> Vec<String> {
        let m = crate::lowering::lower_to_ir(src, &CommandRegistry::build_default());
        match &m.top_level.statements[0] {
            Statement::Call { defs, .. } => defs.clone(),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn append_literal_name_records_def() {
        // FP-guard for: a literal target still records a def.
        assert_eq!(first_stmt_defs("append x foo"), vec!["x".to_string()]);
        assert_eq!(
            first_stmt_defs("lappend items a b"),
            vec!["items".to_string()]
        );
    }

    #[test]
    fn append_substituted_or_expanded_name_records_no_def() {
        // `append $x foo` writes through a substituted name —
        // recording defs=["x"] fabricates a def of the wrong variable. And
        // `append {*}$args` makes args[0] the expanded *read* list. Both must
        // record no concrete def (the registry side-effect model resolves the
        // real write, resolve_place → Unknown).
        assert!(
            first_stmt_defs("append $x foo").is_empty(),
            "substituted name must not record a concrete def",
        );
        assert!(
            first_stmt_defs("lappend {*}$args").is_empty(),
            "{{*}} expansion must not record a concrete def",
        );
        assert!(
            first_stmt_defs("append [pick] foo").is_empty(),
            "command-substituted name must not record a concrete def",
        );
    }

    #[test]
    fn parse_decimal_int_simple() {
        assert_eq!(parse_decimal_int("42"), Some("42".into()));
        assert_eq!(parse_decimal_int("-7"), Some("-7".into()));
        assert_eq!(parse_decimal_int("+3"), Some("+3".into())); // preserves source
        assert_eq!(parse_decimal_int("0"), Some("0".into()));
    }

    #[test]
    fn parse_decimal_int_invalid() {
        assert_eq!(parse_decimal_int("abc"), None);
        assert_eq!(parse_decimal_int("3.14"), None);
        assert_eq!(parse_decimal_int(""), None);
        assert_eq!(parse_decimal_int("0x1f"), None);
        assert_eq!(parse_decimal_int("007"), None); // leading zeros rejected
    }

    #[test]
    fn extract_expr_arg_basic() {
        let aliases = HashSet::new();
        // token_text strips braces from Str tokens; the content base is the
        // offset just past the `{` (the expr text's first byte in `text`).
        assert_eq!(
            extract_single_expr_arg("expr {$a + $b}", &aliases),
            Some(("$a + $b".into(), Some(6)))
        );
    }

    #[test]
    fn extract_expr_arg_too_many_words() {
        let aliases = HashSet::new();
        assert_eq!(extract_single_expr_arg("expr $a + $b", &aliases), None);
    }

    #[test]
    fn extract_expr_arg_alias() {
        let mut aliases = HashSet::new();
        aliases.insert("=".into());
        assert_eq!(
            extract_single_expr_arg("= {1+2}", &aliases),
            Some(("1+2".into(), Some(3)))
        );
    }

    /// JimTcl `set x $($a*2)` is `set x [expr {$a*2}]`: the sugar lowers to
    /// the same `AssignExpr`, with the body parsed as an expression whose
    /// operand is the variable `a` — not an opaque `AssignValue`.
    #[test]
    fn lower_set_jim_expr_sugar_is_an_expression() {
        let args = vec!["x".to_string(), "$($a*2)".to_string()];
        let single = vec![true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::ExprSugar];
        let aliases = CommandAliasMap::new();
        let jim = tcl_registry::model::resolve_environment("jim").unit_profile();
        let cmd = LoweringCommand {
            span: Span::new(0, 13),
            name: "set",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
            dialect: Some(jim),
        };
        match lower_set(&cmd, &aliases) {
            Statement::AssignExpr { name, expr, .. } => {
                assert_eq!(name, "x");
                let rendered = format!("{expr:?}");
                assert!(
                    rendered.contains('a'),
                    "operand `a` missing from {rendered}"
                );
            }
            other => panic!("expected AssignExpr; got {other:?}"),
        }
    }

    #[test]
    fn lower_set_const() {
        let args = vec!["x".to_string(), "hello".to_string()];
        let single = vec![true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::Str];
        let aliases = CommandAliasMap::new();
        let cmd = LoweringCommand {
            span: Span::new(0, 13),
            name: "set",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
            dialect: None,
        };
        let result = lower_set(&cmd, &aliases);
        assert!(
            matches!(result, Statement::AssignConst { .. }),
            "expected AssignConst; got {result:?}"
        );
    }

    #[test]
    fn lower_set_int() {
        let args = vec!["x".to_string(), "42".to_string()];
        let single = vec![true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::Esc];
        let aliases = CommandAliasMap::new();
        let cmd = LoweringCommand {
            span: Span::new(0, 8),
            name: "set",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
            dialect: None,
        };
        let result = lower_set(&cmd, &aliases);
        assert!(
            matches!(result, Statement::AssignConst { .. }),
            "expected AssignConst for integer; got {result:?}"
        );
    }

    #[test]
    fn lower_upvar_with_level() {
        let args = vec!["1".to_string(), "other".to_string(), "local".to_string()];
        let single = vec![true, true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::Esc, ArgTokenKind::Esc];
        let cmd = LoweringCommand {
            span: Span::new(0, 20),
            name: "upvar",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
            dialect: None,
        };
        let result = lower_upvar(&cmd);
        assert!(result.is_some());
        if let Some(Statement::Call { defs, .. }) = result {
            assert_eq!(defs, vec!["local"]);
        }
    }

    /// Registry must drive the lowering-hook dispatch — `set`'s
    /// hook ID must come from [`tcl_registry::CommandSpec`], not a
    /// compiler-side name table.
    #[test]
    fn registry_set_spec_carries_lowering_hook() {
        let registry = CommandRegistry::build_default();
        let spec = registry.get("set").expect("set is registered");
        assert_eq!(spec.lowering_hook, Some(LoweringHookId::Set));
    }

    /// Same check for `expr` — the registry-side hook ID must be
    /// the canonical source of truth, not a compiler dispatch table.
    #[test]
    fn registry_expr_spec_carries_lowering_hook() {
        let registry = CommandRegistry::build_default();
        let spec = registry.get("expr").expect("expr is registered");
        assert_eq!(spec.lowering_hook, Some(LoweringHookId::Expr));
    }

    /// `incr`'s hook is also stamped on the registry side.
    #[test]
    fn registry_incr_spec_carries_lowering_hook() {
        let registry = CommandRegistry::build_default();
        let spec = registry.get("incr").expect("incr is registered");
        assert_eq!(spec.lowering_hook, Some(LoweringHookId::Incr));
    }

    /// Structured-command specs declare their canonical
    /// `LoweringHookId` so downstream consumers (LSP / compiler
    /// explorer / coverage audit) can dispatch through the
    /// registry-typed identifier rather than re-parsing names.
    /// The dispatcher returns `None` for these hooks (the lowerer's
    /// per-command methods need `&mut self`); runtime dispatch flows
    /// through `lower_command`'s dispatch instead.
    #[test]
    fn structured_specs_declare_lowering_hook_ids() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let pairs: &[(&str, LoweringHookId)] = &[
            ("proc", LoweringHookId::Proc),
            ("when", LoweringHookId::When),
            ("if", LoweringHookId::If),
            ("switch", LoweringHookId::Switch),
            ("for", LoweringHookId::For),
            ("while", LoweringHookId::While),
            ("foreach", LoweringHookId::Foreach),
            ("lmap", LoweringHookId::Lmap),
            ("catch", LoweringHookId::Catch),
            ("try", LoweringHookId::Try),
            ("dict", LoweringHookId::Dict),
            ("eval", LoweringHookId::Eval),
            ("uplevel", LoweringHookId::Uplevel),
        ];
        for (name, hook) in pairs {
            let spec = registry.get(name).expect("registered");
            assert_eq!(
                spec.lowering_hook,
                Some(*hook),
                "{name} should declare {hook:?}",
            );
        }
        // `namespace eval` is subcommand-scoped.
        let ns = registry.get("namespace").expect("namespace registered");
        let eval_sub = ns.resolve_subcommand("eval").expect("namespace eval");
        assert_eq!(eval_sub.lowering_hook, Some(LoweringHookId::NamespaceEval),);
    }

    /// End-to-end: routing `set` through `try_lower_hook` returns
    /// the typed `AssignConst` form via the registry-resolved hook.
    #[test]
    fn try_lower_hook_routes_set_via_registry() {
        let registry = CommandRegistry::build_default();
        let aliases = CommandAliasMap::new();
        let args = vec!["x".to_string(), "hello".to_string()];
        let single = vec![true, true, true];
        let kinds = vec![ArgTokenKind::Esc, ArgTokenKind::Str];
        let cmd = LoweringCommand {
            span: Span::new(0, 13),
            name: "set",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
            dialect: None,
        };
        let result = try_lower_hook(&cmd, &aliases, &registry, None, false);
        assert!(
            matches!(result, Some(Statement::AssignConst { .. })),
            "expected AssignConst from registry-driven dispatch; got {result:?}",
        );
    }

    /// A command with no `lowering_hook` must return `None` from
    /// `try_lower_hook` so the caller falls through to the generic
    /// `Statement::Call` path.
    #[test]
    fn try_lower_hook_returns_none_for_uncovered_command() {
        let registry = CommandRegistry::build_default();
        let aliases = CommandAliasMap::new();
        let args = vec!["greetings".to_string()];
        let single = vec![true, true];
        let kinds = vec![ArgTokenKind::Esc];
        let cmd = LoweringCommand {
            span: Span::new(0, 4),
            name: "puts",
            args: &args,
            single_token_word: &single,
            expand_word: None,
            tokens: None,
            arg_kinds: &kinds,
            dialect: None,
        };
        assert!(try_lower_hook(&cmd, &aliases, &registry, None, false).is_none());
    }
}
