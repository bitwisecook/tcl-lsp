//! Tail-call detection pass (C30h, partial).
//!
//! Ported from `core/compiler/optimiser/_tail_call.py`. The Python
//! module emits three distinct diagnostic codes:
//!
//! - **O121** — "Use `tailcall` for self-recursion". Fires on
//!   every tail-position self-call — either a bare call as the
//!   final statement of the procedure (or of each branch's tail
//!   position), or `return [self ...]` with a command
//!   substitution (**landed, bare-call variant only**).
//! - `O122` — "Convert self-recursion to a `while` loop when
//!   every self-call is a tail call". Requires a source-level
//!   loop synthesis — deferred.
//! - `O123` — "Accumulator-style eligible non-tail recursion".
//!   Shape-specific analysis — deferred.
//!
//! The bare-call variant of O121 — `proc f {} { ... ; f $args }`
//! — is landed here. The `return [f ...]` variant needs the
//! inner command-substitution span propagated to the IR (the
//! Rust lowering stores only the outer `Statement::Return`
//! span); it lands with a follow-up strip.

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
