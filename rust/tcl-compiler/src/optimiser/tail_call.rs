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
//! - **O122** (hint-only) — "Convert self-recursion to a
//!   `while` loop". Fires when every self-call in the proc body
//!   is in tail position (the total count of self-calls equals
//!   the number of tail-position calls).
//! - **O123** (hint-only) — "Accumulator-eligible non-tail
//!   self-recursion". Fires when there is at least one non-tail
//!   self-call inside an expression body (e.g., `return [expr
//!   {$n * [f [expr {$n - 1}]]}]`) — a common pattern worth
//!   converting to an accumulator recurrence.

use std::collections::HashSet;
use tcl_core_types::DiagCode;

use crate::compilation_unit::CompilationUnit;
use crate::ir::{Procedure, Script, Statement};
use crate::naming::normalise_qualified_name;

use super::helpers::spans::full_rewrite_span;
use super::{Optimisation, PassContext};

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
    dialect: Option<&tcl_dialect::DialectProfile>,
    floor: tcl_dialect::TclVersion,
) -> bool {
    // `None` (no dialect info on the context — only set by the public-API
    // entry points that don't carry one) defaults to **enabled**.
    dialect.is_none_or(|profile| profile.runtime_base.is_some_and(|base| base >= floor))
}

fn tailcall_supported(dialect: Option<&tcl_dialect::DialectProfile>) -> bool {
    runtime_at_least(dialect, tcl_dialect::TclVersion::V8_6)
}

/// Whether `lassign` is available in `dialect`.  Same `None`-means-
/// enabled fallback as [`tailcall_supported`].
fn lassign_supported(dialect: Option<&tcl_dialect::DialectProfile>) -> bool {
    runtime_at_least(dialect, tcl_dialect::TclVersion::V8_5)
}

/// Run the tail-call detection pass. Emits `O121` for every
/// self-call in tail position (bare-call + return-subst variants),
/// plus the hint-only `O122` loop-conversion and `O123`
/// accumulator-candidate diagnostics described in the module docs.
///
/// O121 is gated on `tailcall`-supporting dialects (Tcl 8.6+ per TIP
/// 327).  Pre-8.6 dialects keep the O122 recursion-to-loop hint but
/// not the O121 `tailcall` suggestion.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    let emit_o121 = tailcall_supported(ctx.dialect);
    for (qname, proc) in &cu.ir_module.procedures {
        let self_names = self_name_variants(qname);
        let mut sites: Vec<TailSite> = Vec::new();
        collect_tail_sites(ctx, &proc.body, &self_names, proc, &mut sites, emit_o121, 0);

        let total_self_calls = count_self_calls_in_script(&proc.body, &self_names);
        if !sites.is_empty() && sites.len() == total_self_calls {
            // O122: every self-call is in tail position. Emit a
            // real source rewrite — restructure the proc body as
            // a `while {1}` loop, replacing each tail call with
            // a parameter reassignment (`set p v` for single
            // param, `lassign` for multiple).  Multi-param
            // bodies need `lassign` (Tcl 8.5+).
            if proc.params.len() <= 1 || lassign_supported(ctx.dialect) {
                emit_loop_conversion(ctx, proc, &sites);
            }
        }

        // O123: any non-tail self-call embedded in an expression
        // → accumulator candidate (hint-only).
        if non_tail_self_call_in_expression(&proc.body, &self_names, ctx.registry, 0) {
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
    /// Raw argument texts passed to the recursive call.
    args: Vec<String>,
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

/// Emit the O122 while-loop conversion rewrite on top of the
/// full proc span. Falls back silently when the proc's
/// `body_source` is not available (synthetic procs) or when
/// argument counts don't line up with parameter counts.
fn emit_loop_conversion(
    ctx: &mut PassContext<'_>,
    proc: &crate::ir::Procedure,
    sites: &[TailSite],
) {
    let Some(body_source) = &proc.body_source else {
        return;
    };
    if proc.params.is_empty() {
        return;
    }
    // Every tail-call site must pass exactly `params.len()` args
    // — otherwise the loop conversion would lose information.
    for site in sites {
        if site.args.len() != proc.params.len() {
            return;
        }
    }
    // Find the body_source within the outer source so we can
    // translate absolute call-site spans to body-local offsets.
    let proc_range = proc.span.as_range();
    if proc_range.end > ctx.source.len() {
        return;
    }
    let proc_text = &ctx.source[proc_range.clone()];
    let Some(body_offset_in_proc) = proc_text.find(body_source.as_str()) else {
        return;
    };
    let body_start_abs = proc_range.start + body_offset_in_proc;

    // Replace every tail-call site with the reassignment — in
    // reverse order so earlier substitutions don't shift later
    // offsets.
    let mut modified = body_source.clone();
    let mut ordered = sites.to_vec();
    ordered.sort_by_key(|s| std::cmp::Reverse(s.span.start()));
    for site in ordered {
        let site_range = site.span.as_range();
        if site_range.start < body_start_abs || site_range.end > body_start_abs + modified.len() {
            return;
        }
        let rel_start = site_range.start - body_start_abs;
        let rel_end = site_range.end - body_start_abs;
        let reassign = make_reassignment(&proc.params, &site.args);
        modified.replace_range(rel_start..rel_end, &reassign);
    }

    // Re-indent the body for the `while {1}` nesting (add 4
    // spaces to every non-empty line).
    let trimmed = modified.trim_end();
    let reindented: String = trimmed
        .split('\n')
        .map(|line| {
            if line.trim().is_empty() {
                line.to_owned()
            } else {
                format!("    {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Pull the short name + params_raw from the proc so the
    // replacement matches the original shape.
    let short_name = &proc.name;
    let params_raw = if proc.params_raw.is_empty() {
        proc.params.join(" ")
    } else {
        proc.params_raw.clone()
    };
    let replacement = format!(
        "proc {short_name} {{{params_raw}}} {{\n    while {{1}} {{{reindented}\n    }}\n}}"
    );

    ctx.report(Optimisation::new(
        DiagCode::O122,
        format!("Convert tail-recursive '{short_name}' to iterative loop"),
        full_rewrite_span(ctx.source, proc.span),
        replacement,
    ));
}

/// Count every textual reference to a self-name across the
/// script (tail, non-tail, inside conditions, inside argument
/// substitutions). Uses the source-level argument text to catch
/// `[self …]` substitutions the IR does not parse into a Call.
fn count_self_calls_in_script(script: &Script, self_names: &HashSet<String>) -> usize {
    let mut count = 0;
    count_self_calls_in_script_impl(script, self_names, &mut count);
    count
}

fn count_self_calls_in_script_impl(
    script: &Script,
    self_names: &HashSet<String>,
    count: &mut usize,
) {
    for stmt in &script.statements {
        count_self_calls_in_stmt(stmt, self_names, count);
    }
}

fn count_self_calls_in_stmt(stmt: &Statement, self_names: &HashSet<String>, count: &mut usize) {
    match stmt {
        Statement::Call { command, args, .. } => {
            if self_names.contains(command) {
                *count += 1;
            }
            for arg in args {
                *count += count_bracket_self_calls(arg, self_names);
            }
        }
        Statement::Return {
            value: Some(v),
            braced,
            ..
        } if !*braced => {
            *count += count_bracket_self_calls(v, self_names);
        }
        Statement::AssignValue { value, .. } => {
            *count += count_bracket_self_calls(value, self_names);
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                count_self_calls_in_script_impl(&c.body, self_names, count);
            }
            if let Some(eb) = else_body {
                count_self_calls_in_script_impl(eb, self_names, count);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            count_self_calls_in_script_impl(init, self_names, count);
            count_self_calls_in_script_impl(next, self_names, count);
            count_self_calls_in_script_impl(body, self_names, count);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => {
            count_self_calls_in_script_impl(body, self_names, count);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            count_self_calls_in_script_impl(body, self_names, count);
            for h in handlers {
                count_self_calls_in_script_impl(&h.body, self_names, count);
            }
            if let Some(fb) = finally_body {
                count_self_calls_in_script_impl(fb, self_names, count);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    count_self_calls_in_script_impl(b, self_names, count);
                }
            }
            if let Some(db) = default_body {
                count_self_calls_in_script_impl(db, self_names, count);
            }
        }
        _ => {}
    }
}

/// Count `[selfname args…]` command-substitution occurrences
/// inside `text`. Uses a simple bracket scanner — enough to
/// catch the common accumulator-pattern shapes.
fn count_bracket_self_calls(text: &str, self_names: &HashSet<String>) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        i += 1;
        // Skip whitespace.
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        // Extract the head word.
        let start = i;
        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_alphanumeric() || b == b'_' {
                i += 1;
            } else if b == b':' && i + 1 < bytes.len() && bytes[i + 1] == b':' {
                i += 2;
            } else {
                break;
            }
        }
        if let Ok(head) = std::str::from_utf8(&bytes[start..i])
            && self_names.contains(head)
        {
            count += 1;
        }
    }
    count
}

/// Whether `value` (a `return` argument) is an accumulator-eligible
/// non-tail self-recursion, gated on the argument being an `[expr {…}]`
/// wrapper:
///
/// 1. the argument is an `[expr {…}]` command substitution (not a plain
///    `[self …]` tail call, which O121 already handles) — the head is
///    recognised via the registry's `EXPR_CONCATENATES_ARGS` trait, not a
///    name match;
/// 2. it embeds **exactly one** self-call — tree recursion like
///    `fib` (`[fib …] + [fib …]`, two calls) is *not* a simple
///    accumulator and must not fire;
/// 3. it contains an associative operator (`+` / `*`) so introducing an
///    accumulator parameter is meaningful.
fn is_accumulator_pattern(
    value: &str,
    self_names: &HashSet<String>,
    registry: Option<&tcl_registry::CommandRegistry>,
) -> bool {
    let Some((head, _)) = parse_return_subst(value) else {
        return false;
    };
    let head_is_expr = registry.and_then(|r| r.get(&head)).is_some_and(|s| {
        s.traits
            .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
    });
    if !head_is_expr {
        return false;
    }
    if count_bracket_self_calls(value, self_names) != 1 {
        return false;
    }
    value.contains('+') || value.contains('*')
}

/// Detect a non-tail self-call embedded in an expression body
/// or a return's command substitution — the accumulator pattern.
/// `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn non_tail_self_call_in_expression(
    script: &Script,
    self_names: &HashSet<String>,
    registry: Option<&tcl_registry::CommandRegistry>,
    depth: u32,
) -> bool {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        return false;
    }
    for stmt in &script.statements {
        if non_tail_in_stmt(stmt, self_names, registry, depth) {
            return true;
        }
    }
    false
}

fn non_tail_in_stmt(
    stmt: &Statement,
    self_names: &HashSet<String>,
    registry: Option<&tcl_registry::CommandRegistry>,
    depth: u32,
) -> bool {
    match stmt {
        Statement::Return {
            value: Some(v),
            braced,
            ..
        } => {
            // Braced `return {[f $n]}` is literal text — never
            // executed as a call.
            if *braced {
                return false;
            }
            is_accumulator_pattern(v, self_names, registry)
        }
        // Accumulator sites come from `return` statements only, so
        // an assignment never contributes an O123 candidate.
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses
                .iter()
                .any(|c| non_tail_self_call_in_expression(&c.body, self_names, registry, depth + 1))
                || else_body.as_ref().is_some_and(|b| {
                    non_tail_self_call_in_expression(b, self_names, registry, depth + 1)
                })
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            arms.iter().any(|a| {
                a.body.as_ref().is_some_and(|b| {
                    non_tail_self_call_in_expression(b, self_names, registry, depth + 1)
                })
            }) || default_body.as_ref().is_some_and(|b| {
                non_tail_self_call_in_expression(b, self_names, registry, depth + 1)
            })
        }
        Statement::For {
            init, body, next, ..
        } => {
            non_tail_self_call_in_expression(init, self_names, registry, depth + 1)
                || non_tail_self_call_in_expression(body, self_names, registry, depth + 1)
                || non_tail_self_call_in_expression(next, self_names, registry, depth + 1)
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => {
            non_tail_self_call_in_expression(body, self_names, registry, depth + 1)
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            non_tail_self_call_in_expression(body, self_names, registry, depth + 1)
                || handlers.iter().any(|h| {
                    non_tail_self_call_in_expression(&h.body, self_names, registry, depth + 1)
                })
                || finally_body.as_ref().is_some_and(|fb| {
                    non_tail_self_call_in_expression(fb, self_names, registry, depth + 1)
                })
        }
        _ => false,
    }
}

/// Return the set of command names that refer to `qname` — the
/// normalised qualified name, its short (final) segment, and the
/// global form without the leading `::`.
fn self_name_variants(qname: &str) -> HashSet<String> {
    let mut names: HashSet<String> = HashSet::new();
    let normalised = normalise_qualified_name(qname);
    names.insert(normalised.clone());
    if let Some(short) = normalised.rsplit("::").next()
        && !short.is_empty()
    {
        names.insert(short.to_owned());
    }
    if let Some(stripped) = normalised.strip_prefix("::") {
        names.insert(stripped.to_owned());
    }
    names
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
    self_names: &HashSet<String>,
    proc: &Procedure,
    sites: &mut Vec<TailSite>,
    emit_o121: bool,
    depth: u32,
) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        return;
    }
    let Some(last) = script.statements.last() else {
        return;
    };
    match last {
        Statement::Call {
            span,
            command,
            args,
            ..
        } if self_names.contains(command) => {
            let rewrite_span = full_rewrite_span(ctx.source, *span);
            if emit_o121 {
                ctx.report(Optimisation::new(
                    DiagCode::O121,
                    format!("Use tailcall for self-recursion in proc '{}'", proc.name),
                    rewrite_span,
                    format!("tailcall {command}"),
                ));
            }
            sites.push(TailSite {
                span: rewrite_span,
                args: args.clone(),
            });
        }
        Statement::Return {
            span,
            value: Some(v),
            braced,
            ..
        } => {
            // `return {[f $n]}` is a braced literal — the
            // substitution is never executed — so neither O121
            // (tailcall rewrite) nor the site count toward O122
            // (loop conversion) should fire.
            if *braced {
                return;
            }
            if let Some((call_head, call_args)) = parse_return_subst(v)
                && self_names.contains(&call_head)
            {
                let rewrite_span = full_rewrite_span(ctx.source, *span);
                if emit_o121 {
                    let replacement = if call_args.is_empty() {
                        format!("tailcall {call_head}")
                    } else {
                        format!("tailcall {call_head} {call_args}")
                    };
                    ctx.report(Optimisation::new(
                        DiagCode::O121,
                        format!("Use tailcall for self-recursion in proc '{}'", proc.name),
                        rewrite_span,
                        replacement,
                    ));
                }
                let split_args: Vec<String> = if call_args.is_empty() {
                    Vec::new()
                } else {
                    call_args.split_whitespace().map(str::to_owned).collect()
                };
                sites.push(TailSite {
                    span: rewrite_span,
                    args: split_args,
                });
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                collect_tail_sites(ctx, &c.body, self_names, proc, sites, emit_o121, depth + 1);
            }
            if let Some(eb) = else_body {
                collect_tail_sites(ctx, eb, self_names, proc, sites, emit_o121, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    collect_tail_sites(ctx, b, self_names, proc, sites, emit_o121, depth + 1);
                }
            }
            if let Some(db) = default_body {
                collect_tail_sites(ctx, db, self_names, proc, sites, emit_o121, depth + 1);
            }
        }
        _ => {}
    }
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
/// (`[a]] [b`), and trailing-text shapes a naive strip would accept (issue
/// 152).
fn parse_return_subst(value: &str) -> Option<(String, String)> {
    let v = value.trim();
    let sm = tcl_lexer::SourceMap::new(v);
    let toks = tcl_lexer::Lexer::new(v).tokenise_all().ok()?;
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
    // Split on the first whitespace run.
    if let Some(pos) = inner.find(char::is_whitespace) {
        let head = inner[..pos].to_owned();
        let rest = inner[pos..].trim().to_owned();
        return Some((head, rest));
    }
    Some((inner.to_owned(), String::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::CommandRegistry;

    use crate::interprocedural::InterproceduralAnalysis;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.registry = Some(&reg);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    fn run_pass_with_dialect(
        source: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> Vec<Optimisation> {
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let mut ctx = PassContext::with_dialect(
            &cu.source,
            InterproceduralAnalysis::default(),
            Some(dialect),
        );
        ctx.registry = Some(&reg);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    /// Regression coverage for issue #996: `collect_tail_sites` and the
    /// mutually-recursive `non_tail_self_call_in_expression`/
    /// `non_tail_in_stmt` pair recurse once per nested `if`/`for`/`while`/
    /// `foreach`/`catch`/`try`/`switch` body, with no depth cap of their
    /// own before this fix. Transitively bounded to `MAX_LOWER_NEST_DEPTH`
    /// (256) by the lowering pass today, so this is defence-in-depth /
    /// consistency with every other full-tree walker in this crate, not a
    /// currently-reproducible crash. 1000 levels of source nesting is
    /// comfortably past this new cap; the assertion is that `run_pass`
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
            parse_return_subst("[a $x]"),
            Some(("a".to_owned(), "$x".to_owned()))
        );
        assert_eq!(
            parse_return_subst("[foo]"),
            Some(("foo".to_owned(), String::new()))
        );
        // A concatenation of two substitutions is NOT a single subst — a naive
        // strip would yield the invalid `a $x][b $y` (issue 152).
        assert_eq!(parse_return_subst("[a $x][b $y]"), None);
        // Trailing text after the substitution is likewise rejected.
        assert_eq!(parse_return_subst("[a] tail"), None);
        // Not a substitution at all.
        assert_eq!(parse_return_subst("plain"), None);
        assert_eq!(parse_return_subst("[]"), None);
    }

    #[test]
    fn self_name_variants_cover_short_absolute_bare() {
        let v = self_name_variants("::ns::foo");
        assert!(v.contains("::ns::foo"));
        assert!(v.contains("foo"));
        assert!(v.contains("ns::foo"));
    }

    #[test]
    fn tail_call_bare_variant_fires() {
        let opts =
            run_pass("proc ::f {n} {\n    if {$n <= 0} { return 1 }\n    f [expr {$n - 1}]\n}");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O121 && o.replacement.contains("tailcall")),
            "expected O121, got {opts:?}",
        );
    }

    #[test]
    fn o121_suppressed_on_pre_8_6_dialects() {
        // `tailcall` is TIP 327 (Tcl 8.6+); pre-8.6 dialects must NOT
        // emit O121.  The body is a single-recursive self-call —
        // O121 would normally fire — but on tcl8.4 / tcl8.5 / f5-irules
        // / cadence-eda-tcl (8.4-based) the suggestion is incorrect
        // (the dialect can't run `tailcall`).
        let src = "proc ::f {n} {\n    if {$n <= 0} { return 1 }\n    f [expr {$n - 1}]\n}";
        for dialect in [
            "tcl8.4",
            "tcl8.5",
            "f5-irules",
            "f5-iapps",
            "cadence-eda-tcl",
        ] {
            let opts = run_pass_with_dialect(src, tcl_dialect::DialectProfile::by_name(dialect));
            assert!(
                opts.iter().all(|o| o.code != DiagCode::O121),
                "O121 must not fire on {dialect}, got {opts:?}",
            );
        }
    }

    #[test]
    fn o121_fires_on_8_6_plus_dialects() {
        let src = "proc ::f {n} {\n    if {$n <= 0} { return 1 }\n    f [expr {$n - 1}]\n}";
        for dialect in [
            "tcl8.6",
            "tcl9.0",
            "synopsys-eda-tcl",
            "mentor-eda-tcl",
            "expect",
        ] {
            let opts = run_pass_with_dialect(src, tcl_dialect::DialectProfile::by_name(dialect));
            assert!(
                opts.iter()
                    .any(|o| o.code == DiagCode::O121 && o.replacement.contains("tailcall")),
                "O121 expected on {dialect}, got {opts:?}",
            );
        }
    }

    #[test]
    fn o122_loop_conversion_still_fires_pre_8_6_for_single_param() {
        // O122 is dialect-agnostic for single-param bodies (it emits
        // a bare `set`, no `lassign`).  A pre-8.6 dialect should
        // still see the loop-conversion suggestion.
        let src = "proc ::f {n} {\n    if {$n <= 0} { return 1 }\n    f [expr {$n - 1}]\n}";
        let opts = run_pass_with_dialect(src, tcl_dialect::DialectProfile::by_name("tcl8.4"));
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O122),
            "O122 expected on tcl8.4 single-param body, got {opts:?}",
        );
    }

    #[test]
    fn o122_loop_conversion_suppressed_on_tcl8_4_multi_param() {
        // tcl8.4 doesn't have `lassign` (TIP 57, 8.5+); a multi-param
        // body's O122 rewrite would need `lassign` so it must be
        // suppressed on tcl8.4.
        let src = "proc ::f {a b} {\n    if {$a <= 0} { return 1 }\n    f [expr {$a - 1}] $b\n}";
        let opts = run_pass_with_dialect(src, tcl_dialect::DialectProfile::by_name("tcl8.4"));
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O122),
            "O122 must not fire on tcl8.4 multi-param body, got {opts:?}",
        );
    }

    #[test]
    fn o122_multi_param_rewrite_uses_list_not_braces() {
        // The multi-param reassignment must be
        // `lassign [list …] a b`, never the braced `lassign {…} a b`
        // form (which breaks `[expr {…}]` args with a hard tclsh error).
        let src = "proc ::f {a b} {\n    if {$a <= 0} { return $b }\n    f [expr {$a - 1}] [expr {$b + $a}]\n}";
        let opts = run_pass_with_dialect(src, tcl_dialect::DialectProfile::by_name("tcl8.6"));
        let opt = opts
            .iter()
            .find(|o| o.code == DiagCode::O122)
            .expect("O122 should fire on multi-param tail recursion");
        assert!(
            opt.replacement.contains("lassign [list "),
            "O122 must use `lassign [list …]`, got {:?}",
            opt.replacement,
        );
        assert!(
            !opt.replacement.contains("lassign {"),
            "O122 must not emit a braced `lassign {{…}}`, got {:?}",
            opt.replacement,
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
    fn tail_call_inside_if_branch_fires() {
        let opts = run_pass(
            "proc ::fact {n} {\n\
                 if {$n <= 1} { return 1 } else { fact [expr {$n - 1}] }\n\
             }",
        );
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O121),
            "expected O121 inside else branch, got {opts:?}",
        );
    }

    #[test]
    fn return_substitution_variant_fires() {
        let opts = run_pass(
            "proc ::fact {n} { if {$n <= 1} { return 1 } else { return [fact [expr {$n - 1}]] } }",
        );
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O121 && o.replacement.contains("tailcall")),
            "expected O121 for return [self …] variant, got {opts:?}",
        );
    }

    #[test]
    fn parse_return_subst_extracts_head_and_args() {
        assert_eq!(
            parse_return_subst("[f $n]"),
            Some(("f".to_string(), "$n".to_string()))
        );
        assert_eq!(
            parse_return_subst("[g]"),
            Some(("g".to_string(), String::new()))
        );
        assert!(parse_return_subst("$x").is_none());
        assert!(parse_return_subst("[]").is_none());
    }

    #[test]
    fn o122_loop_conversion_rewrite_when_all_self_calls_are_tail() {
        // Every self-call is in a tail position → emit a
        // real source rewrite (not hint-only).
        let opts =
            run_pass("proc ::fact {n} { if {$n <= 1} { return 1 } else { fact [expr {$n - 1}] } }");
        let opt = opts
            .iter()
            .find(|o| o.code == DiagCode::O122)
            .expect("O122 should fire");
        assert!(!opt.hint_only, "O122 should now be a real rewrite");
        assert!(
            opt.replacement.contains("while {1}"),
            "expected while-loop replacement, got {:?}",
            opt.replacement,
        );
        assert!(
            opt.replacement.contains("set n") || opt.replacement.contains("lassign"),
            "expected parameter reassignment in loop body, got {:?}",
            opt.replacement,
        );
    }

    #[test]
    fn o122_multi_param_uses_lassign() {
        // Two-param tail-recursive proc → lassign for simultaneous
        // reassignment.
        let opts = run_pass(
            "proc ::f {a b} { if {$a <= 0} { return $b } else { f [expr {$a - 1}] [expr {$b + 1}] } }",
        );
        let opt = opts
            .iter()
            .find(|o| o.code == DiagCode::O122)
            .expect("O122 should fire");
        assert!(
            opt.replacement.contains("lassign"),
            "expected lassign for multi-param reassignment, got {:?}",
            opt.replacement,
        );
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
        let cu = CompilationUnit::build_for("proc ::f {} { f }", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::TailCall]);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == DiagCode::O121),
            "expected O121 via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
