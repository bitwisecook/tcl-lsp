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

//! Tail-call detection pass.
//!
//! Emits:
//!
//! - **O121** — "Use `tailcall` for self-recursion". Two
//!   variants:
//!   - **bare call**: `proc f {…} { …; f $args }` — the
//!     self-call is the final statement of the body.
//!   - **return substitution**: `return [f $args]`.
//! - **O122** — "Convert self-recursion to a `while` loop". A
//!   real source rewrite, not a hint: the whole proc is replaced
//!   with a `while {1}` body whose recursive call becomes a
//!   parameter reassignment. Fires when every self-call in the
//!   proc body is in tail position (the total count of self-calls
//!   equals the number of tail-position calls) and every one of
//!   them passes exactly one argument per parameter.
//! - **O123** (hint-only) — "Accumulator-eligible non-tail
//!   self-recursion". Fires when there is at least one non-tail
//!   self-call inside an expression body (e.g., `return [expr
//!   {$n * [f [expr {$n - 1}]]}]`) — a common pattern worth
//!   converting to an accumulator recurrence.

use tcl_core_types::DiagCode;

use crate::compilation_unit::CompilationUnit;
use crate::ir::{Procedure, Script, Statement};

use super::helpers::spans::full_rewrite_span;
use super::{Optimisation, PassContext};

mod inventory;
use inventory::Inventory;

/// Whether `tailcall` is available in `dialect` — TIP 327, Tcl 8.6+,
/// derived from the profile's modelled runtime rather than a name list
/// (AGENTS.md: profile facts, never `match dialect_name` special-casing —
/// a list here silently excluded `tcl9.1` and every future 8.6+ shell).
/// A profile with no runtime at all (`f5-bigip`) has no Tcl surface to
/// rewrite and gates out naturally.
///
/// O122's `lassign`-based loop conversion needs a separate 8.5+ gate
/// (lassign is TIP 57, Tcl 8.5+), but a single-param body emits a bare
/// `set` and is dialect-agnostic.
fn runtime_at_least(
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    floor: tcl_dialect::TclVersion,
) -> bool {
    // `None` (no dialect info on the context — only set by the public-API
    // entry points that don't carry one) defaults to **enabled**.
    dialect.is_none_or(|profile| profile.runtime_base.is_some_and(|base| base >= floor))
}

fn tailcall_supported(dialect: Option<&'static tcl_dialect::DialectProfile>) -> bool {
    runtime_at_least(dialect, tcl_dialect::TclVersion::V8_6)
}

/// Whether `lassign` is available in `dialect`.  Same `None`-means-
/// enabled fallback as [`tailcall_supported`].
fn lassign_supported(dialect: Option<&'static tcl_dialect::DialectProfile>) -> bool {
    runtime_at_least(dialect, tcl_dialect::TclVersion::V8_5)
}

/// Run the tail-call detection pass. Emits `O121` for every
/// self-call in tail position (bare-call + return-subst variants),
/// plus the guarded `O122` loop-conversion rewrite and hint-only `O123`
/// accumulator-candidate diagnostic described in the module docs.
///
/// O121 is gated on `tailcall`-supporting dialects (Tcl 8.6+ per TIP
/// 327).  Pre-8.6 dialects keep the O122 recursion-to-loop hint but
/// not the O121 `tailcall` suggestion.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    let emit_o121 = tailcall_supported(ctx.dialect);
    for proc in cu.ir_module.procedures.values() {
        if !proc.body.is_authored_source() {
            continue;
        }
        let Some(inventory) = Inventory::capture(ctx, &cu.ir_module, proc) else {
            continue;
        };
        let mut sites: Vec<TailSite> = Vec::new();
        collect_tail_sites(
            ctx,
            &proc.body,
            &inventory,
            proc,
            &mut sites,
            emit_o121 && inventory.complete && inventory.frame_effects_closed,
            0,
        );

        let total_self_calls = inventory.calls.len() + inventory.readonly_calls;
        if inventory.complete
            && inventory.loop_commands_preserved
            && inventory.frame_effects_closed
            && inventory
                .calls
                .values()
                .all(|call| call.retains_formal_roots || call.assignments_preserve_effects)
            && !sites.is_empty()
            && sites.len() == total_self_calls
            && proc.body.is_fully_authored_source()
        {
            // O122: every self-call is in tail position. Emit a
            // real source rewrite — restructure the proc body as
            // a `while {1}` loop, replacing each tail call with
            // a parameter reassignment (`set p v` for single
            // param, `lassign` for multiple).  Multi-param
            // bodies need `lassign` (Tcl 8.5+).
            if sites.iter().all(|site| site.retains_formal_roots)
                || proc.params.len() <= 1
                || lassign_supported(ctx.dialect)
            {
                emit_loop_conversion(ctx, proc, &inventory, &sites);
            }
        }

        // O123: any non-tail self-call embedded in an expression
        // → accumulator candidate (hint-only).
        if inventory.complete && !inventory.accumulator_returns.is_empty() {
            let mut opt = Optimisation::new(
                DiagCode::O123,
                format!(
                    "Proc '{}' is a candidate for accumulator-style rewriting",
                    proc.name
                ),
                proc.span,
                "",
            );
            opt.hint_only = true;
            ctx.report(opt);
        }
    }
}

/// One tail-position self-call site — the span of the call
/// statement plus the argument texts (needed to build the loop
/// body's parameter reassignment).
#[derive(Debug, Clone)]
struct TailSite {
    /// Absolute source span of the tail-call statement.
    span: tcl_lexer::Span,
    /// The words passed to the recursive call, one entry per
    /// argument. `None` when the arguments could not be split into
    /// words with a fixed arity — a `{*}` expansion, or a value
    /// holding more than one command — which makes the O122 loop
    /// conversion unsafe.
    args: Option<Vec<String>>,
    /// Every original formal object is passed unchanged and fetched quietly.
    /// Redundant assignments can be omitted only under this source owner.
    retains_formal_roots: bool,
}

/// Produce the replacement parameter reassignment for a tail
/// call — `set p v` for a single param, `lassign [list v1 v2 …]
/// p1 p2 …` for multiple.
///
/// The multi-param form must use `[list …]`, **not** a braced
/// `{v1 v2 …}` word: a braced word suppresses all substitution, so
/// for plain `$var` args the params would be reassigned the literal
/// strings, and for the common `[expr {…}]` argument the braced list
/// is malformed (`list element in braces followed by "]"`) and Tcl
/// raises a hard runtime error.  `[list …]` evaluates each argument
/// and builds a proper list before `lassign` distributes it.
fn make_reassignment(params: &[String], args: &[String]) -> String {
    if params.len() == 1 {
        format!("set {} {}", params[0], args[0])
    } else {
        let arg_list = args.join(" ");
        let param_list = params.join(" ");
        format!("lassign [list {arg_list}] {param_list}")
    }
}

/// Render proved fixed scalar destinations under the original source channel.
/// Every tail site must select the same complete formal binding topology.
fn loop_parameter_source_words(
    ctx: &PassContext<'_>,
    inventory: &Inventory,
    original: &crate::command_binding::ExecutedScriptSource,
    sites: &[TailSite],
) -> Option<Vec<String>> {
    let topology = inventory.formal_topology.as_ref()?;
    let mut parameter_words = None;
    for site in sites {
        let arguments = site.args.as_ref()?;
        let names = topology.fixed_scalar_binding_names(arguments.len())?;
        let words = names
            .iter()
            .map(|name| {
                tcl_syntax::backslash::native_literal_source_word(
                    name.as_bytes(),
                    original.origin.source_image().channel(),
                    ctx.lexer_config(),
                    topology.source_string_protocol(),
                )
            })
            .collect::<Option<Vec<_>>>()?;
        if parameter_words
            .as_ref()
            .is_some_and(|previous| previous != &words)
        {
            return None;
        }
        parameter_words = Some(words);
    }
    parameter_words
}

/// Emit the O122 while-loop conversion rewrite on top of the
/// full proc span. Falls back silently when the proc's
/// `body_source` is not available (synthetic procs) or when
/// argument counts don't line up with parameter counts.
fn emit_loop_conversion(
    ctx: &mut PassContext<'_>,
    proc: &crate::ir::Procedure,
    inventory: &Inventory,
    sites: &[TailSite],
) {
    let Some(body_source) = &proc.body_source else {
        return;
    };
    let Some(original) = proc.body.executed_source.as_deref() else {
        return;
    };
    if !matches!(original.mapping, crate::command_binding::ExecutedScriptMapping::Contiguous { base } if base == proc.body_offset)
        || original.text.bytes() != body_source.as_bytes()
    {
        return;
    }
    let Some(parameter_words) = loop_parameter_source_words(ctx, inventory, original, sites) else {
        return;
    };
    if parameter_words.len() > 1
        && !lassign_supported(ctx.dialect)
        && sites.iter().any(|site| !site.retains_formal_roots)
    {
        return;
    }
    let proc_range = proc.span.as_range();
    let Some(proc_text) = ctx.source.get(proc_range.clone()) else {
        return;
    };
    let body_start = usize::try_from(proc.body_offset).ok();
    let Some(body_start) = body_start else {
        return;
    };
    let Some(body_end) = body_start.checked_add(body_source.len()) else {
        return;
    };
    // A loop body is inserted only inside this original, contiguous brace
    // operand. The unchanged header retains the selected definer, source name,
    // parameter-list spelling and surrounding namespace qualification.
    if body_start <= proc_range.start
        || body_end >= proc_range.end
        || ctx.source.as_bytes().get(body_start - 1) != Some(&b'{')
        || ctx.source.as_bytes().get(body_end) != Some(&b'}')
        || ctx.source.get(body_start..body_end) != Some(body_source.as_str())
    {
        return;
    }
    let mut modified = body_source.clone();
    let mut ordered = sites.to_vec();
    ordered.sort_by_key(|site| std::cmp::Reverse(site.span.start()));
    for site in ordered {
        let range = site.span.as_range();
        if range.start < body_start || range.end > body_end {
            return;
        }
        let Some(arguments) = &site.args else {
            return;
        };
        let reassignment = if parameter_words.is_empty() || site.retains_formal_roots {
            "continue".to_owned()
        } else {
            format!(
                "{}; continue",
                make_reassignment(&parameter_words, arguments)
            )
        };
        let local = range.start - body_start..range.end - body_start;
        if !modified.is_char_boundary(local.start) || !modified.is_char_boundary(local.end) {
            return;
        }
        modified.replace_range(local, &reassignment);
    }
    // Do not indent original lines: spaces in multiline literals are values.
    // A branch which falls through still completes the procedure normally;
    // rewritten recursive sites continue explicitly after argument evaluation.
    let loop_body = format!("\n    while {{1}} {{\n{modified}\nreturn\n    }}\n");
    let relative = body_start - proc_range.start..body_end - proc_range.start;
    let mut replacement = proc_text.to_owned();
    replacement.replace_range(relative, &loop_body);
    ctx.report(Optimisation::new(
        DiagCode::O122,
        format!("Convert tail-recursive '{}' to iterative loop", proc.name),
        proc.span,
        replacement,
    ));
}

/// Record the tail site for a bare self-call, `f $args`, and report its
/// O121 rewrite.
///
/// The rewrite prefixes the call as written: `full_rewrite_span` only
/// extends the statement span through trailing closers, so its text is the
/// call verbatim, with braces, quotes, `{*}` markers and spacing intact.
/// The IR's `args` hold each word's *value* with its delimiters stripped,
/// which cannot be reassembled into the source that produced it, so both
/// the `tailcall` text and the loop conversion's arguments read the source.
fn collect_bare_call_site(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    _command: &str,
    proc: &Procedure,
    sites: &mut Vec<TailSite>,
    emit_o121: bool,
) {
    let rewrite_span = full_rewrite_span(ctx.source, span);
    let call_text = ctx.source.get(rewrite_span.as_range());
    if emit_o121 {
        let Some(text) = call_text else {
            return;
        };
        let replacement = format!("tailcall {text}");
        ctx.report(Optimisation::new(
            DiagCode::O121,
            format!("Use tailcall for self-recursion in proc '{}'", proc.name),
            rewrite_span,
            replacement,
        ));
    }
    // `[list …]` must receive the words as written, so a braced or quoted
    // argument stays one element.
    let args = call_text.and_then(|text| split_call_arguments(text, ctx.lexer_config()));
    sites.push(TailSite {
        span: rewrite_span,
        args,
        retains_formal_roots: false,
    });
}

/// Record the tail site for a return substitution, `return [f $args]`, and
/// report its O121 rewrite. Does nothing when the returned value is not a single
/// command substitution of a self-name.
fn collect_return_subst_site(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    tokens: &crate::ir::CommandTokens,
    inventory: &Inventory,
    proc: &Procedure,
    sites: &mut Vec<TailSite>,
    emit_o121: bool,
) {
    let config = ctx.lexer_config();
    let Some((call_span, call_text)) = original_return_command(ctx, tokens) else {
        return;
    };
    let Some(call) = inventory.call_at(call_span) else {
        return;
    };
    let rewrite_span = full_rewrite_span(ctx.source, span);
    if emit_o121 && call.tailcall_command_preserved && call.retains_formal_roots {
        let replacement = format!("tailcall {call_text}");
        ctx.report(Optimisation::new(
            DiagCode::O121,
            format!("Use tailcall for self-recursion in proc '{}'", proc.name),
            rewrite_span,
            replacement,
        ));
    }
    let args = call
        .written_arguments()
        .then(|| split_call_arguments(&call_text, config))
        .flatten();
    sites.push(TailSite {
        span: rewrite_span,
        args,
        retains_formal_roots: call.retains_formal_roots,
    });
}

/// Recursively walk `script` collecting self-calls in tail
/// position. Only the last statement of each script (and the
/// tail position of each `if` / `switch` branch) is considered.
///
/// When `emit_o121` is false (pre-8.6 dialect), tail sites are
/// still collected (so O122 loop conversion can still fire if every
/// self-call is in tail position) but the O121 `tailcall`
/// rewrite suggestion is suppressed.
/// `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn collect_tail_sites(
    ctx: &mut PassContext<'_>,
    script: &Script,
    inventory: &Inventory,
    proc: &Procedure,
    sites: &mut Vec<TailSite>,
    emit_o121: bool,
    depth: u32,
) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) || !script.is_authored_source() {
        return;
    }
    let Some(last) = script.statements.last() else {
        return;
    };
    match last {
        Statement::Call { span, command, .. } if inventory.call_at(*span).is_some() => {
            collect_bare_call_site(
                ctx,
                *span,
                command,
                proc,
                sites,
                emit_o121
                    && inventory.call_at(*span).is_some_and(|call| {
                        call.tailcall_command_preserved && call.retains_formal_roots
                    }),
            );
            if let Some(site) = sites.last_mut() {
                site.retains_formal_roots = inventory
                    .call_at(*span)
                    .is_some_and(|call| call.retains_formal_roots);
            }
            if !inventory
                .call_at(*span)
                .is_some_and(inventory::SelfCall::written_arguments)
                && let Some(site) = sites.last_mut()
            {
                site.args = None;
            }
        }
        Statement::Return {
            span,
            value: Some(_),
            braced,
            ..
        } => {
            // `return {[f $n]}` is a braced literal — the substitution is
            // never executed — so neither O121 nor the site count toward
            // O122 should fire.
            if !*braced && let Some(tokens) = script.retained_source_tokens_for_statement(last) {
                collect_return_subst_site(ctx, *span, tokens, inventory, proc, sites, emit_o121);
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                collect_tail_sites(ctx, &c.body, inventory, proc, sites, emit_o121, depth + 1);
            }
            if let Some(eb) = else_body {
                collect_tail_sites(ctx, eb, inventory, proc, sites, emit_o121, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    collect_tail_sites(ctx, b, inventory, proc, sites, emit_o121, depth + 1);
                }
            }
            if let Some(db) = default_body {
                collect_tail_sites(ctx, db, inventory, proc, sites, emit_o121, depth + 1);
            }
        }
        _ => {}
    }
}

/// Split a whole command's text into its top-level Tcl words, the command
/// name first.
///
/// One entry per word, which is what O122's one-argument-per-parameter gate
/// counts: `f [expr {$n - 1}] [expr {$acc * $n}]` is three words, not nine.
/// Grouping the lexer's own tokens into words keeps a `{…}`, `[…]`, `"…"`
/// or `${…}` word intact where splitting on whitespace would count its
/// inner spaces.
///
/// The text must start at the command name, not at the first argument: a
/// word is lexed according to its position, so an argument such as `#stop`
/// is a comment at command position and an ordinary word after one.
///
/// `None` when the arity is not statically known — a `{*}` expansion, whose
/// runtime word count is unbounded, or more than one command, from a
/// newline inside the substitution.  The gate refuses a site it cannot
/// count rather than rewrite it against a wrong arity.
fn split_command_words(text: &str, config: tcl_lexer::LexerConfig) -> Option<Vec<String>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Some(Vec::new());
    }
    let tokens = tcl_lexer::Lexer::with_config(trimmed, config)
        .tokenise_all()
        .ok()?;
    let commands = tcl_lexer::group_commands(&tokens, trimmed, config);
    let [command] = commands.as_slice() else {
        return None;
    };
    if !command.expand_markers.is_empty() {
        return None;
    }
    command
        .words
        .iter()
        .map(|word| {
            // The lexer's inner-end convention leaves a delimited word's
            // closer outside the span. `word_span_at` puts it back for every
            // delimited form, `${name}` included — its token-based sibling
            // knows only `{`, `[` and `"` openers and would cut `${n}` back
            // to `${n`.
            let end = tcl_lexer::word_span_at(trimmed, word.span)
                .end()
                .max(word.span.end());
            trimmed
                .get(word.span.start() as usize..end as usize)
                .map(str::to_owned)
        })
        .collect()
}

/// The arguments of a self-call, one entry per argument, or `None` when
/// their arity is not statically known. Splits the whole command so each
/// word is lexed in the position it actually occupies, then drops the
/// command name.
fn split_call_arguments(command_text: &str, config: tcl_lexer::LexerConfig) -> Option<Vec<String>> {
    let mut words = split_command_words(command_text, config)?;
    if words.is_empty() {
        return None;
    }
    words.remove(0);
    Some(words)
}

/// Original sole command substitution of the retained Return value word.
/// The value's compatibility text cannot choose a nested implementation or
/// fabricate an editable source span.
fn original_return_command(
    ctx: &PassContext<'_>,
    tokens: &crate::ir::CommandTokens,
) -> Option<(tcl_lexer::Span, String)> {
    if tokens.words().len() != 2 {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    let words = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        ctx.lexer_config(),
    )?;
    let word = words.get(1)?;
    let arena = word.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    let tcl_lexer::ExecutablePart::Command { body } = part.part else {
        return None;
    };
    let bytes = arena.bytes(body)?;
    let image = tcl_lexer::SourceImage::from_bytes(bytes, arena.image().channel());
    let commands = crate::segmenter::segment_commands_image_with_offset_and_config(
        &image,
        body.start(),
        ctx.lexer_config(),
    )?;
    let [command] = commands.as_slice() else {
        return None;
    };
    if command.is_partial {
        return None;
    }
    Some((command.span, std::str::from_utf8(bytes).ok()?.to_owned()))
}

/// Parse a `return` value's text looking for a `[cmd args…]`
/// command substitution shape. Returns `(cmd, args_text)` or
/// `None` if the text is not a *single* command substitution.
///
/// The single-substitution requirement matters: `return [a $x][b $y]` is a
/// legal *concatenation* of two substitutions whose text starts with `[` and
/// ends with `]`, but stripping the outer brackets and splitting would build
/// the syntactically invalid `tailcall a $x][b $y`. Lexing the value and
/// requiring exactly one top-level `Cmd` word rejects the concat, nested-close
/// (`[a]] [b`), and trailing-text shapes a naive strip would accept.
#[cfg(test)]
fn parse_return_subst(value: &str, config: tcl_lexer::LexerConfig) -> Option<(String, String)> {
    let v = value.trim();
    let sm = tcl_lexer::SourceMap::new(v);
    let toks = tcl_lexer::Lexer::with_config(v, config)
        .tokenise_all()
        .ok()?;
    let mut words = toks.iter().filter(|t| {
        !matches!(
            t.kind,
            tcl_lexer::TokenType::Sep | tcl_lexer::TokenType::Eol | tcl_lexer::TokenType::Eof
        )
    });
    let cmd_tok = words.next()?;
    // Exactly one word, a command substitution starting at the very front.
    if words.next().is_some()
        || cmd_tok.kind != tcl_lexer::TokenType::Cmd
        || cmd_tok.span.start() != 0
    {
        return None;
    }
    // `token_text` strips the leading `[`; the trailing `]` is outside the span
    // (inner-end convention) so the inner body is exactly the command text.
    let inner = sm.token_text(*cmd_tok).trim();
    if inner.is_empty() {
        return None;
    }
    // Keep each grouped word as written: a quoted or braced command name may
    // itself contain whitespace, escapes or non-ASCII units.
    let mut words = split_command_words(inner, config)?;
    if words.is_empty() {
        return None;
    }
    let head = words.remove(0);
    Some((head, words.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::CommandRegistry;

    use crate::interprocedural::InterproceduralAnalysis;

    fn registry() -> &'static CommandRegistry {
        tcl_registry::model::ingress::static_context_for("tcl8.6")
            .commands()
            .as_ref()
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let reg = registry();
        let cu = CompilationUnit::build_for_dialect(source, reg, false, "tcl8.6");
        let mut ctx = PassContext::with_dialect(
            &cu.source,
            InterproceduralAnalysis::default(),
            reg.profile(),
        );
        ctx.registry = Some(reg);
        ctx.ir_module = Some(&cu.ir_module);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    fn run_pass_with_dialect(
        source: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> Vec<Optimisation> {
        let reg = tcl_registry::model::ingress::static_context_for(dialect.name)
            .commands()
            .clone();
        let cu = CompilationUnit::build_for_profile(source, &reg, false, dialect);
        let mut ctx = PassContext::with_dialect(
            &cu.source,
            InterproceduralAnalysis::default(),
            Some(dialect),
        );
        ctx.registry = Some(&reg);
        ctx.ir_module = Some(&cu.ir_module);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    /// `collect_tail_sites` and the
    /// mutually-recursive `non_tail_self_call_in_expression`/
    /// `non_tail_in_stmt` pair recurse once per nested `if`/`for`/`while`/
    /// `foreach`/`catch`/`try`/`switch` body, and carry their own depth
    /// cap. Transitively bounded to `MAX_LOWER_NEST_DEPTH`
    /// (256) by the lowering pass today, so this is defence-in-depth /
    /// consistency with every other full-tree walker in this crate, not a
    /// currently-reproducible crash. 1000 levels of source nesting is
    /// comfortably past that cap; the assertion is that `run_pass`
    /// returns at all, not what it returns. Spawns its own big-stack
    /// thread since the lexer/CST/segmenter stages upstream of the
    /// lowering cap still walk the full un-truncated source nesting before
    /// that cap trims it — same rationale as
    /// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_tail_call_scan() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = "proc f {n} {\n".to_owned();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("f $n\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        src.push_str("}\n");
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = run_pass(&src);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn parse_return_subst_accepts_single_and_rejects_concat() {
        // A single command substitution parses into (head, args).
        assert_eq!(
            parse_return_subst("[a $x]", tcl_lexer::LexerConfig::default()),
            Some(("a".to_owned(), "$x".to_owned()))
        );
        assert_eq!(
            parse_return_subst("[foo]", tcl_lexer::LexerConfig::default()),
            Some(("foo".to_owned(), String::new()))
        );
        // A concatenation of two substitutions is NOT a single subst — a naive
        // strip would yield the invalid `a $x][b $y`.
        assert_eq!(
            parse_return_subst("[a $x][b $y]", tcl_lexer::LexerConfig::default()),
            None
        );
        // Trailing text after the substitution is likewise rejected.
        assert_eq!(
            parse_return_subst("[a] tail", tcl_lexer::LexerConfig::default()),
            None
        );
        // Not a substitution at all.
        assert_eq!(
            parse_return_subst("plain", tcl_lexer::LexerConfig::default()),
            None
        );
        assert_eq!(
            parse_return_subst("[]", tcl_lexer::LexerConfig::default()),
            None
        );
    }

    #[test]
    fn unrelated_short_name_is_not_the_original_procedure() {
        let source = "namespace eval other {proc f {n} {return OTHER}}; namespace eval here {proc f {n} {::other::f $n}}";
        assert!(
            run_pass(source)
                .iter()
                .all(|item| !matches!(item.code, DiagCode::O121 | DiagCode::O122))
        );
    }

    #[test]
    fn original_allocation_and_grouped_names_control_tail_rewrites() {
        // Implementation contract: naming.optimiser.original-tail-call-inventory
        // docs/design/analysis/name-resolution-proofs/original-tail-call-inventory.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let profile = tcl_registry::model::ingress::static_context_for(dialect)
                .commands()
                .profile()
                .unwrap();
            for source in [
                "proc \"f name\" {n} {\"f name\" $n}",
                "proc {f😀} {n} {{f😀} $n}",
            ] {
                let options = run_pass_with_dialect(source, profile);
                let loop_rewrite = options
                    .iter()
                    .find(|option| option.code == DiagCode::O122)
                    .unwrap_or_else(|| {
                        panic!("{dialect}: exact original scalar self call {options:?}")
                    });
                assert!(
                    loop_rewrite
                        .replacement
                        .starts_with(source.split(" {n}").next().unwrap())
                );
                assert!(loop_rewrite.replacement.contains("continue"));
            }
            for source in [
                "proc f {n} {rename f g; proc f {n} {return REPLACED}; f $n}",
                "namespace eval other {proc f {n} {return OTHER}}; proc f {n} {::other::f $n}",
                "proc f {args} {f {*}$args}",
            ] {
                assert!(
                    run_pass_with_dialect(source, profile)
                        .iter()
                        .all(|option| option.code != DiagCode::O122),
                    "{dialect}: {source}"
                );
            }
        }
    }

    #[test]
    fn produced_expression_commands_retain_readonly_self_identity() {
        // Implementation contract: naming.optimiser.original-tail-call-inventory
        // docs/design/analysis/name-resolution-proofs/original-tail-call-inventory.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let profile = tcl_registry::model::ingress::static_context_for(dialect)
                .commands()
                .profile()
                .unwrap();
            for source in [
                r#"proc f {n} {expr "\[f 0\]"; f $n}"#,
                r#"interp alias {} a {} f; proc f {n} {expr "\[a 0\]"; f $n}"#,
            ] {
                assert!(
                    run_pass_with_dialect(source, profile)
                        .iter()
                        .all(|option| option.code != DiagCode::O122),
                    "{dialect}: readonly non-tail self call"
                );
            }
        }
    }

    #[test]
    fn tail_call_bare_variant_fires() {
        let opts = run_pass("proc ::f {n} {f $n}");
        assert!(
            opts.iter().any(
                |option| option.code == DiagCode::O121 && option.replacement == "tailcall f $n"
            ),
            "{opts:?}"
        );
    }

    #[test]
    fn o121_suppressed_on_pre_8_6_dialects() {
        // `tailcall` is TIP 327 (Tcl 8.6+); pre-8.6 dialects must NOT
        // emit O121.  The body is a single-recursive self-call —
        // O121 would normally fire — but on tcl8.4 / tcl8.5 / f5-irules
        // / cadence-eda-tcl (8.4-based) the suggestion is incorrect
        // (the dialect can't run `tailcall`).
        let src = "proc ::f {n} {f $n}";
        for dialect in [
            "tcl8.4",
            "tcl8.5",
            "f5-irules",
            "f5-iapps",
            "cadence-eda-tcl",
        ] {
            let opts = run_pass_with_dialect(
                src,
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert!(
                opts.iter().all(|o| o.code != DiagCode::O121),
                "O121 must not fire on {dialect}, got {opts:?}",
            );
        }
    }

    #[test]
    fn o121_fires_on_8_6_plus_dialects() {
        let src = "proc ::f {n} {f $n}";
        for dialect in [
            "tcl8.6",
            "tcl9.0",
            "synopsys-eda-tcl",
            "mentor-eda-tcl",
            "expect",
        ] {
            let opts = run_pass_with_dialect(
                src,
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert!(
                opts.iter()
                    .any(|o| o.code == DiagCode::O121 && o.replacement.contains("tailcall")),
                "O121 expected on {dialect}, got {opts:?}",
            );
        }
    }

    #[test]
    fn o122_loop_conversion_still_fires_pre_8_6_for_single_param() {
        let opts = run_pass_with_dialect(
            "proc ::f {n} {f $n}",
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        );
        let option = opts
            .iter()
            .find(|option| option.code == DiagCode::O122)
            .expect("unchanged scalar root needs no newer assignment worker");
        assert!(option.replacement.contains("continue"));
        assert!(!option.replacement.contains("set "));
    }

    #[test]
    fn o122_loop_conversion_suppressed_on_tcl8_4_multi_param() {
        // tcl8.4 doesn't have `lassign` (TIP 57, 8.5+); a multi-param
        // body's O122 rewrite would need `lassign` so it must be
        // suppressed on tcl8.4.
        let src = "proc ::f {a b} {\n    if {$a <= 0} { return 1 }\n    f [expr {$a - 1}] $b\n}";
        let opts = run_pass_with_dialect(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        );
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O122),
            "O122 must not fire on tcl8.4 multi-param body, got {opts:?}",
        );
    }

    #[test]
    fn o122_bracketed_word_geometry_does_not_close_changed_formal_release() {
        let source = "proc ::f {a b} {f [expr {$a - 1}] [expr {$b + $a}]}";
        assert!(
            run_pass(source)
                .iter()
                .all(|option| option.code != DiagCode::O122)
        );
        assert_eq!(
            split_call_arguments(
                "f [expr {$a - 1}] [expr {$b + $a}]",
                tcl_lexer::LexerConfig::default()
            ),
            Some(vec!["[expr {$a - 1}]".into(), "[expr {$b + $a}]".into()])
        );
    }

    #[test]
    fn make_reassignment_multi_param_emits_list() {
        let r = make_reassignment(
            &["a".to_string(), "b".to_string()],
            &["[expr {$a - 1}]".to_string(), "$b".to_string()],
        );
        assert_eq!(r, "lassign [list [expr {$a - 1}] $b] a b");
    }

    #[test]
    fn non_tail_call_is_not_reported() {
        // The self-call is NOT the last statement — puts follows.
        let opts = run_pass("proc ::f {n} {\n    f $n\n    puts \"done\"\n}");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O121),
            "non-tail call should not fire, got {opts:?}",
        );
    }

    #[test]
    fn tail_call_branch_requires_closed_condition_and_frame_release() {
        // Fixed arity and source spelling do not close an unknown incoming object's release.
        let opts = run_pass("proc fact {n} {if {$n <= 1} {return 1} else {fact [expr {$n - 1}]}}");
        assert!(
            opts.iter()
                .all(|option| !matches!(option.code, DiagCode::O121 | DiagCode::O122))
        );
    }

    #[test]
    fn return_substitution_requires_independent_old_frame_release() {
        // Fixed arity and source spelling do not close an unknown incoming object's release.
        let opts = run_pass("proc fact {n} {return [fact [expr {$n - 1}]]}");
        assert!(
            opts.iter()
                .all(|option| !matches!(option.code, DiagCode::O121 | DiagCode::O122))
        );
    }

    #[test]
    fn parse_return_subst_extracts_head_and_args() {
        assert_eq!(
            parse_return_subst("[f $n]", tcl_lexer::LexerConfig::default()),
            Some(("f".to_string(), "$n".to_string()))
        );
        assert_eq!(
            parse_return_subst("[g]", tcl_lexer::LexerConfig::default()),
            Some(("g".to_string(), String::new()))
        );
        assert!(parse_return_subst("$x", tcl_lexer::LexerConfig::default()).is_none());
        assert!(parse_return_subst("[]", tcl_lexer::LexerConfig::default()).is_none());
    }

    #[test]
    fn o122_loop_conversion_rewrite_when_all_self_calls_are_tail() {
        let opts = run_pass("proc ::fact {} {fact}");
        let opt = opts
            .iter()
            .find(|option| option.code == DiagCode::O122)
            .expect("empty frame has no incoming object release");
        assert!(!opt.hint_only);
        assert!(opt.replacement.contains("while {1}") && opt.replacement.contains("continue"));
    }

    #[test]
    fn o122_unchanged_multi_param_roots_omit_stores() {
        let opts = run_pass("proc ::f {a b} {f $a $b}");
        let opt = opts
            .iter()
            .find(|option| option.code == DiagCode::O122)
            .expect("unchanged roots require no stores");
        assert!(opt.replacement.contains("continue"));
        assert!(!opt.replacement.contains("lassign"));
    }

    #[test]
    fn o122_bracketed_return_arguments_require_original_release_owners() {
        // Fixed arity and source spelling do not close an unknown incoming object's release.
        let opts = run_pass("proc fact {n acc} {return [fact [expr {$n - 1}] [expr {$acc * $n}]]}");
        assert!(
            opts.iter()
                .all(|option| !matches!(option.code, DiagCode::O121 | DiagCode::O122))
        );
    }

    #[test]
    fn o122_gcd_arguments_do_not_prove_incoming_object_class() {
        // Fixed arity and source spelling do not close an unknown incoming object's release.
        let opts = run_pass(
            "proc gcd {a b} {if {$b == 0} {return $a} else {return [gcd $b [expr {$a % $b}]]}}",
        );
        assert!(
            opts.iter()
                .all(|option| !matches!(option.code, DiagCode::O121 | DiagCode::O122))
        );
    }

    #[test]
    fn split_call_arguments_keeps_each_word_whole() {
        let config = tcl_lexer::LexerConfig::default();
        let args = |text: &str| split_call_arguments(text, config);
        assert_eq!(
            args("f [expr {$n - 1}] [expr {$acc * $n}]"),
            Some(vec![
                "[expr {$n - 1}]".to_owned(),
                "[expr {$acc * $n}]".to_owned(),
            ]),
        );
        assert_eq!(
            args("f $b [expr {$a % $b}]"),
            Some(vec!["$b".to_owned(), "[expr {$a % $b}]".to_owned()]),
        );
        assert_eq!(
            args("f {a b} \"c d\" e"),
            Some(vec![
                "{a b}".to_owned(),
                "\"c d\"".to_owned(),
                "e".to_owned(),
            ]),
        );
        // A braced variable reference keeps its closing brace: `${n}` sliced
        // back to `${n` would make the reassignment unparseable.
        assert_eq!(
            args("f ${n} ${a b}"),
            Some(vec!["${n}".to_owned(), "${a b}".to_owned()]),
        );
        // `#` is a comment only at command position; after the command name
        // it is an ordinary word.
        assert_eq!(args("f #stop"), Some(vec!["#stop".to_owned()]));
        assert_eq!(args("f"), Some(Vec::new()));
        // A `{*}` expansion has no statically known arity.
        assert_eq!(args("f {*}$args $b"), None);
    }

    #[test]
    fn o122_suppressed_when_tail_call_expands_its_args() {
        // `{*}` makes the runtime word count unknown, so the loop
        // conversion and frame elimination require independent object capabilities.
        let opts = run_pass(
            "proc f {a b} {\n    if {$a == 0} { return $b }\n    return [f {*}[list [expr {$a - 1}] $b]]\n}",
        );
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O122),
            "O122 must not fire on a `{{*}}`-expanded tail call, got {opts:?}",
        );
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O121),
            "unknown expanded values cannot prove frame teardown, got {opts:?}",
        );
    }

    #[test]
    fn o122_suppressed_when_a_self_call_sits_in_a_control_condition() {
        // A condition is not a tail position: the loop body still
        // evaluates it, so the recursion inside it would survive the
        // conversion. Every one of these shapes must keep O122 out.
        for src in [
            "proc f {n} {\n    if {[f $n]} {\n        return [f [expr {$n - 1}]]\n    } else {\n        return $n\n    }\n}",
            "proc f {n} {\n    while {[f $n]} {\n        return [f [expr {$n - 1}]]\n    }\n    return $n\n}",
            "proc f {n} {\n    for {set i 0} {[f $n]} {incr i} {\n        return [f [expr {$n - 1}]]\n    }\n    return $n\n}",
            "proc f {n} {\n    switch [f [expr {$n - 1}]] {\n        0 { return 0 }\n        default { return [f [expr {$n - 2}]] }\n    }\n}",
        ] {
            let opts = run_pass(src);
            assert!(
                opts.iter().all(|o| o.code != DiagCode::O122),
                "O122 must not fire with a self-call in a control condition: {src}\ngot {opts:?}",
            );
        }
    }

    #[test]
    fn o122_braced_same_cell_argument_needs_no_reassignment() {
        let options = run_pass("proc f {n} {f ${n}}");
        let replacement = &options
            .iter()
            .find(|option| option.code == DiagCode::O122)
            .expect("exact unchanged braced variable root")
            .replacement;
        assert!(replacement.contains("continue") && !replacement.contains("set "));
    }

    #[test]
    fn o122_hash_leading_argument_does_not_close_old_object_release() {
        // Fixed arity and source spelling do not close an unknown incoming object's release.
        let opts = run_pass("proc h {tag} {h #stop}");
        assert!(
            opts.iter()
                .all(|option| !matches!(option.code, DiagCode::O121 | DiagCode::O122))
        );
    }

    #[test]
    fn o121_bare_call_keeps_its_arguments() {
        let opts = run_pass("proc h {a b} {h $a $b}");
        assert_eq!(
            opts.iter()
                .find(|option| option.code == DiagCode::O121)
                .expect("unchanged whole-object arguments")
                .replacement,
            "tailcall h $a $b"
        );
    }

    #[test]
    fn o121_static_argument_spelling_does_not_close_old_frame_release() {
        let opts = run_pass(r#"proc f {a b c} {f {x y} "q r" {*}{MORE}}"#);
        assert!(
            opts.iter().all(|option| option.code != DiagCode::O121),
            "quoted/static expansion values cannot classify released old formal objects: {opts:?}"
        );
    }

    #[test]
    fn original_frame_observation_and_release_obligations_withdraw_tail_rewrites() {
        // Native proof: naming.optimiser.recursive-frame-level
        // docs/design/analysis/name-resolution-proofs/recursive-frame-level.md
        // Native proof: naming.optimiser.recursive-parent-cell
        // docs/design/analysis/name-resolution-proofs/recursive-parent-cell.md
        // Native proof: naming.optimiser.recursive-unset-observer-frame
        // docs/design/analysis/name-resolution-proofs/recursive-unset-observer-frame.md
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let profile = tcl_registry::model::ingress::static_context_for(name)
                .commands()
                .profile()
                .unwrap();
            for source in [
                "proc f {n} {if {$n <= 0} {return [info level]}; f [expr {$n - 1}]}; f 2",
                "set n 99; proc f {n} {if {$n <= 0} {upvar 1 n parent; return $parent}; f [expr {$n - 1}]}; f 2",
                "proc watch args {}; proc f {} {trace add variable shell unset watch; f}",
                "proc f {} {set shell VALUE; f}",
                "proc watch args {}; proc f {n} {f $n}; trace add execution f enter watch; f 2",
            ] {
                let options = run_pass_with_dialect(source, profile);
                assert!(
                    options
                        .iter()
                        .all(|option| !matches!(option.code, DiagCode::O121 | DiagCode::O122)),
                    "{name}: frame reach, local shells and observers require independent closure: {source}: {options:?}"
                );
            }
        }
    }

    #[test]
    fn original_changed_formal_objects_require_complete_release_evidence() {
        // Native proof: naming.optimiser.recursive-argument-release-order
        // docs/design/analysis/name-resolution-proofs/recursive-argument-release-order.md
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let profile = tcl_registry::model::ingress::static_context_for(name)
                .commands()
                .profile()
                .unwrap();
            for source in [
                "proc f {n value} {if {$n <= 0} {return DONE}; f [expr {$n - 1}] [make]}; f 2 INITIAL",
                "proc f {value} {f CHANGED}",
                "proc f {n} {if {$n <= 0} {return DONE}; f [expr {$n - 1}]}",
            ] {
                let options = run_pass_with_dialect(source, profile);
                assert!(
                    options
                        .iter()
                        .all(|option| !matches!(option.code, DiagCode::O121 | DiagCode::O122)),
                    "{name}: known bytes and fixed arity do not prove ordinary release of an incoming object: {source}: {options:?}"
                );
            }
        }
    }

    #[test]
    fn unchanged_original_formal_objects_continue_without_releasing_or_rendering_them() {
        // Implementation contract: naming.optimiser.original-tail-call-inventory
        // docs/design/analysis/name-resolution-proofs/original-tail-call-inventory.md
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let profile = tcl_registry::model::ingress::static_context_for(name)
                .commands()
                .profile()
                .unwrap();
            for source in [
                "proc f {} {f}",
                "proc f {n} {f $n}",
                "proc f {a b} {f $a $b}",
            ] {
                let options = run_pass_with_dialect(source, profile);
                let replacement = &options
                    .iter()
                    .find(|option| option.code == DiagCode::O122)
                    .unwrap_or_else(|| {
                        panic!("{name}: complete unchanged-root transfer: {source}: {options:?}")
                    })
                    .replacement;
                assert!(replacement.contains("continue"));
                assert!(
                    !replacement.contains("lassign") && !replacement.contains("set \""),
                    "{name}: unchanged formal objects need no synthetic store: {replacement}"
                );
            }
        }
    }

    #[test]
    fn original_unknown_expansion_cannot_preserve_the_callee() {
        let opts = run_pass("proc f {rest} { f {*}$rest }");
        assert!(
            opts.iter()
                .all(|opt| !matches!(opt.code, DiagCode::O121 | DiagCode::O122))
        );
    }

    #[test]
    fn original_unknown_string_conversion_cannot_preserve_the_callee() {
        let opts = run_pass("proc h {tag} { if {$tag eq \"stop\"} { return $tag }; h #stop }");
        assert!(
            opts.iter()
                .all(|opt| !matches!(opt.code, DiagCode::O121 | DiagCode::O122))
        );
    }

    #[test]
    fn original_rewrite_helpers_require_their_positioned_registry_implementations() {
        for helper in ["while", "return", "continue"] {
            let source = format!(
                "namespace eval N {{ proc {helper} {{args}} {{ error SHADOW }}; proc f {{n}} {{ f $n }} }}"
            );
            let opts = run_pass(&source);
            assert!(
                opts.iter().all(|opt| opt.code != DiagCode::O122),
                "shadowed {helper}: {opts:?}"
            );
        }
        for helper in ["list", "lassign"] {
            let source = format!(
                "namespace eval N {{ proc {helper} {{args}} {{ error SHADOW }}; proc f {{a b}} {{ f $a $b }} }}"
            );
            let opts = run_pass(&source);
            assert!(
                opts.iter().any(|opt| opt.code == DiagCode::O122),
                "unused assignment helper {helper}: {opts:?}"
            );
        }
        let opts =
            run_pass("namespace eval N { proc tailcall {args} {error SHADOW}; proc f {n} {f $n} }");
        assert!(
            opts.iter().all(|opt| opt.code != DiagCode::O121),
            "shadowed tailcall: {opts:?}"
        );
    }

    #[test]
    fn o122_grouped_word_geometry_remains_separate_from_release() {
        let opts = run_pass(r#"proc f {a b c} {f {x y} "q r" $c}"#);
        assert!(opts.iter().all(|option| option.code != DiagCode::O122));
        assert_eq!(
            split_call_arguments(r#"f {x y} "q r" $c"#, tcl_lexer::LexerConfig::default()),
            Some(vec!["{x y}".into(), r#""q r""#.into(), "$c".into()])
        );
    }

    #[test]
    fn bare_and_return_subst_changed_arguments_need_the_same_release_obligation() {
        for source in [
            r#"proc f {a b c} {f {x y} "q r" $c}"#,
            r#"proc f {a b c} {return [f {x y} "q r" $c]}"#,
        ] {
            let opts = run_pass(source);
            assert!(opts.iter().all(|option| option.code != DiagCode::O122));
        }
    }

    #[test]
    fn o122_skipped_when_arity_mismatch() {
        // Tail-call passes wrong number of args → fold refused.
        let opts = run_pass("proc ::f {a b} { if {$a <= 0} { return 0 } else { f 1 } }");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O122),
            "arity mismatch should suppress O122, got {opts:?}",
        );
    }

    #[test]
    fn o123_accumulator_hint_when_self_call_embedded_in_expr() {
        // Classic accumulator pattern: `return [expr {$n * [fact
        // [expr {$n - 1}]]}]` — the recursive call is nested
        // inside an expression, not in the tail position.
        let opts = run_pass(
            "proc ::fact {n} { if {$n <= 1} { return 1 } else { return [expr {$n * [fact [expr {$n - 1}]]}] } }",
        );
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O123 && o.hint_only),
            "expected O123 accumulator hint, got {opts:?}",
        );
    }

    #[test]
    fn o123_does_not_fire_on_tree_recursion() {
        // `fib` has TWO self-calls in the expression, so it is not a
        // simple accumulator pattern — O123 must not fire.
        let opts = run_pass(
            "proc ::fib {n} { if {$n < 2} { return $n } else { return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}] } }",
        );
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O123),
            "tree recursion must not emit O123, got {opts:?}",
        );
    }

    #[test]
    fn o123_requires_associative_operator() {
        // A single embedded self-call with no `+`/`*` operator is not an
        // accumulator candidate (e.g. a wrapping `[expr {-[f $n]}]`).
        let opts = run_pass(
            "proc ::g {n} { if {$n <= 0} { return 0 } else { return [expr {-[g [expr {$n - 1}]]}] } }",
        );
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O123),
            "non-associative wrapper must not emit O123, got {opts:?}",
        );
    }

    #[test]
    fn run_passes_dispatches_tail_call() {
        let reg = registry();
        let cu = CompilationUnit::build_for_dialect("proc ::f {} { f }", reg, false, "tcl8.6");
        let mut ctx = PassContext::with_dialect(
            &cu.source,
            InterproceduralAnalysis::default(),
            reg.profile(),
        );
        ctx.registry = Some(reg);
        ctx.ir_module = Some(&cu.ir_module);
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::TailCall]);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == DiagCode::O121),
            "expected O121 via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
