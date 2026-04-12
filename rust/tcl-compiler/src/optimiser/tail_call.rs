//! Tail-call detection pass (C30h).
//!
//! Ported from `core/compiler/optimiser/_tail_call.py`. Emits:
//!
//! - **O121** — "Use `tailcall` for self-recursion". Two
//!   variants:
//!   - **bare call**: `proc f {…} { …; f $args }` — the
//!     self-call is the final statement of the body (or of each
//!     branch's tail position). The pass emits O121 targeting
//!     the call site with a `tailcall …` replacement.
//!   - **return substitution**: `return [f $args]` — a return
//!     whose value is a command substitution whose head is a
//!     self-name. Detected from the source text slice of the
//!     `Statement::Return` span; same diagnostic, replacement
//!     `tailcall f $args`.
//!
//! **O122** ("Convert self-recursion to a `while` loop when
//! every self-call is a tail call") and **O123**
//! ("Accumulator-style eligible non-tail recursion") remain
//! deferred — each needs source-level body re-synthesis that is
//! beyond this strip's scope.

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
        collect_tail_sites(ctx, &proc.body, &self_names, proc);
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
