//! Tail-call detection pass (C30h).
//!
//! Ported from `core/compiler/optimiser/_tail_call.py`. Emits:
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
//!   {$n * [f [expr {$n - 1}]]}]`) — a common pattern that the
//!   Python pass flags as worth converting to an accumulator
//!   recurrence.

use std::collections::HashSet;

use crate::compilation_unit::CompilationUnit;
use crate::ir::{Procedure, Script, Statement};
use crate::naming::normalise_qualified_name;

use super::{Optimisation, PassContext};

/// Run the tail-call detection pass. Emits `O121` for every
/// self-call in tail position (bare-call variant only; see
/// module docs for deferred variants).
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    for (qname, proc) in &cu.ir_module.procedures {
        let self_names = self_name_variants(qname);
        let tail_count_before = ctx.optimisations.iter().filter(|o| o.code == "O121").count();
        collect_tail_sites(ctx, &proc.body, &self_names, proc);
        let tail_count = ctx
            .optimisations
            .iter()
            .filter(|o| o.code == "O121")
            .count()
            - tail_count_before;

        let total_self_calls = count_self_calls_in_script(&proc.body, &self_names);
        if tail_count > 0 && tail_count == total_self_calls {
            let mut opt = Optimisation::new(
                "O122",
                format!(
                    "Convert tail-recursive proc '{}' to a while loop",
                    proc.name
                ),
                proc.span,
                "",
            );
            opt.hint_only = true;
            ctx.report(opt);
        }

        // O123: any non-tail self-call embedded in an expression
        // → accumulator candidate.
        if non_tail_self_call_in_expression(&proc.body, &self_names) {
            let mut opt = Optimisation::new(
                "O123",
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

fn count_self_calls_in_stmt(
    stmt: &Statement,
    self_names: &HashSet<String>,
    count: &mut usize,
) {
    match stmt {
        Statement::Call { command, args, .. } => {
            if self_names.contains(command) {
                *count += 1;
            }
            for arg in args {
                *count += count_bracket_self_calls(arg, self_names);
            }
        }
        Statement::Return { value: Some(v), .. } => {
            *count += count_bracket_self_calls(v, self_names);
        }
        Statement::AssignValue { value, .. } => {
            *count += count_bracket_self_calls(value, self_names);
        }
        Statement::If { clauses, else_body, .. } => {
            for c in clauses {
                count_self_calls_in_script_impl(&c.body, self_names, count);
            }
            if let Some(eb) = else_body {
                count_self_calls_in_script_impl(eb, self_names, count);
            }
        }
        Statement::For { init, next, body, .. } => {
            count_self_calls_in_script_impl(init, self_names, count);
            count_self_calls_in_script_impl(next, self_names, count);
            count_self_calls_in_script_impl(body, self_names, count);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => {
            count_self_calls_in_script_impl(body, self_names, count);
        }
        Statement::Try { body, handlers, finally_body, .. } => {
            count_self_calls_in_script_impl(body, self_names, count);
            for h in handlers {
                count_self_calls_in_script_impl(&h.body, self_names, count);
            }
            if let Some(fb) = finally_body {
                count_self_calls_in_script_impl(fb, self_names, count);
            }
        }
        Statement::Switch { arms, default_body, .. } => {
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
        if let Ok(head) = std::str::from_utf8(&bytes[start..i]) {
            if self_names.contains(head) {
                count += 1;
            }
        }
    }
    count
}

/// Detect a non-tail self-call embedded in an expression body
/// or a return's command substitution — the accumulator pattern.
fn non_tail_self_call_in_expression(
    script: &Script,
    self_names: &HashSet<String>,
) -> bool {
    for stmt in &script.statements {
        if non_tail_in_stmt(stmt, self_names) {
            return true;
        }
    }
    false
}

fn non_tail_in_stmt(stmt: &Statement, self_names: &HashSet<String>) -> bool {
    match stmt {
        Statement::Return { value: Some(v), .. } => {
            // Pure `return [self …]` — that's a tail return-
            // subst, already handled by O121, not accumulator.
            if let Some((head, _)) = parse_return_subst(v) {
                if self_names.contains(&head) && !v.trim_start().contains("[expr") {
                    return false;
                }
            }
            count_bracket_self_calls(v, self_names) > 0
                && !matches!(parse_return_subst(v), Some((h, _)) if self_names.contains(&h))
        }
        Statement::AssignValue { value, .. } => {
            count_bracket_self_calls(value, self_names) > 0
        }
        Statement::If { clauses, else_body, .. } => {
            clauses.iter().any(|c| non_tail_self_call_in_expression(&c.body, self_names))
                || else_body
                    .as_ref()
                    .is_some_and(|b| non_tail_self_call_in_expression(b, self_names))
        }
        Statement::Switch { arms, default_body, .. } => {
            arms.iter().any(|a| {
                a.body
                    .as_ref()
                    .is_some_and(|b| non_tail_self_call_in_expression(b, self_names))
            }) || default_body
                .as_ref()
                .is_some_and(|b| non_tail_self_call_in_expression(b, self_names))
        }
        Statement::For { init, body, next, .. } => {
            non_tail_self_call_in_expression(init, self_names)
                || non_tail_self_call_in_expression(body, self_names)
                || non_tail_self_call_in_expression(next, self_names)
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => non_tail_self_call_in_expression(body, self_names),
        Statement::Try { body, handlers, finally_body, .. } => {
            non_tail_self_call_in_expression(body, self_names)
                || handlers
                    .iter()
                    .any(|h| non_tail_self_call_in_expression(&h.body, self_names))
                || finally_body
                    .as_ref()
                    .is_some_and(|fb| non_tail_self_call_in_expression(fb, self_names))
        }
        _ => false,
    }
}

/// Return the set of command names that refer to `qname`.
/// Matches Python's `_self_name_variants` — the normalised
/// qualified name, its short (final) segment, and the global
/// form without the leading `::`.
fn self_name_variants(qname: &str) -> HashSet<String> {
    let mut names: HashSet<String> = HashSet::new();
    let normalised = normalise_qualified_name(qname);
    names.insert(normalised.clone());
    if let Some(short) = normalised.rsplit("::").next() {
        if !short.is_empty() {
            names.insert(short.to_owned());
        }
    }
    if let Some(stripped) = normalised.strip_prefix("::") {
        names.insert(stripped.to_owned());
    }
    names
}

/// Recursively walk `script` collecting self-calls in tail
/// position. Only the last statement of each script (and the
/// tail position of each `if` / `switch` branch) is considered.
fn collect_tail_sites(
    ctx: &mut PassContext<'_>,
    script: &Script,
    self_names: &HashSet<String>,
    proc: &Procedure,
) {
    let Some(last) = script.statements.last() else {
        return;
    };
    match last {
        Statement::Call { span, command, .. } if self_names.contains(command) => {
            ctx.report(Optimisation::new(
                "O121",
                format!(
                    "Use tailcall for self-recursion in proc '{}'",
                    proc.name,
                ),
                *span,
                format!("tailcall {command}"),
            ));
        }
        Statement::Return {
            span,
            value: Some(v),
            ..
        } => {
            if let Some((call_head, call_args)) = parse_return_subst(v) {
                if self_names.contains(&call_head) {
                    let replacement = if call_args.is_empty() {
                        format!("tailcall {call_head}")
                    } else {
                        format!("tailcall {call_head} {call_args}")
                    };
                    ctx.report(Optimisation::new(
                        "O121",
                        format!(
                            "Use tailcall for self-recursion in proc '{}'",
                            proc.name,
                        ),
                        *span,
                        replacement,
                    ));
                }
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                collect_tail_sites(ctx, &c.body, self_names, proc);
            }
            if let Some(eb) = else_body {
                collect_tail_sites(ctx, eb, self_names, proc);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    collect_tail_sites(ctx, b, self_names, proc);
                }
            }
            if let Some(db) = default_body {
                collect_tail_sites(ctx, db, self_names, proc);
            }
        }
        _ => {}
    }
}

/// Parse a `return` value's text looking for a `[cmd args…]`
/// command substitution shape. Returns `(cmd, args_text)` or
/// `None` if the text is not a single command substitution.
fn parse_return_subst(value: &str) -> Option<(String, String)> {
    let v = value.trim();
    let inner = v.strip_prefix('[').and_then(|s| s.strip_suffix(']'))?;
    let inner = inner.trim();
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
        let cu = CompilationUnit::build_for(source, &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run(&mut ctx, &cu);
        ctx.optimisations
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
        let opts = run_pass("proc ::f {n} {\n    if {$n <= 0} { return 1 }\n    f [expr {$n - 1}]\n}");
        assert!(
            opts.iter()
                .any(|o| o.code == "O121" && o.replacement.contains("tailcall")),
            "expected O121, got {opts:?}",
        );
    }

    #[test]
    fn non_tail_call_is_not_reported() {
        // The self-call is NOT the last statement — puts follows.
        let opts = run_pass(
            "proc ::f {n} {\n    f $n\n    puts \"done\"\n}",
        );
        assert!(
            opts.iter().all(|o| o.code != "O121"),
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
            opts.iter().any(|o| o.code == "O121"),
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
                .any(|o| o.code == "O121" && o.replacement.contains("tailcall")),
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
    fn o122_loop_conversion_hint_when_all_self_calls_are_tail() {
        // Every self-call is in a tail position → emit O122 hint.
        let opts = run_pass(
            "proc ::fact {n} { if {$n <= 1} { return 1 } else { fact [expr {$n - 1}] } }",
        );
        assert!(
            opts.iter().any(|o| o.code == "O122" && o.hint_only),
            "expected O122 loop-conversion hint, got {opts:?}",
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
            opts.iter().any(|o| o.code == "O123" && o.hint_only),
            "expected O123 accumulator hint, got {opts:?}",
        );
    }

    #[test]
    fn run_passes_dispatches_tail_call() {
        let cu = CompilationUnit::build_for(
            "proc ::f {} { f }",
            &registry(),
            false,
        );
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::TailCall]);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == "O121"),
            "expected O121 via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
