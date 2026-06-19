//! General proc inliner (v0 + verbatim).
//!
//! Ported from `compiler/inlining/` — the catalogue (`decision.py`) plus
//! the IR-level splice transform (`inline_pass.py`). The inliner rewrites
//! statement-position calls to *inlinable* procedures so the call boundary
//! disappears before WASM codegen consumes the IR.
//!
//! # Scope
//!
//! Two of the four Python shapes are ported here, the ones whose soundness
//! needs no α-renaming or interprocedural escape proof:
//!
//! * **v0 — empty body.** A call to a zero-statement proc vanishes.
//! * **v1 / v2 — verbatim wrapper.** A zero-parameter proc whose every
//!   body statement is *splice-eligible* (a frame-independent, def-free,
//!   substitution-free builtin call) has its body spliced verbatim into
//!   the call site.
//!
//! The **v3 parameterised** shape (α-renamed body + parameter bindings) and
//! its `_rename` machinery, plus dead-proc elimination, are the documented
//! residual — see [`docs/rust-rewrite.md`] (FE-OPT) / the **RT-WASM** track.
//! The inliner's only consumer is the WASM codegen (RT-WASM, unported), so
//! it is exposed but not yet wired into a pipeline.
//!
//! # Eligibility
//!
//! Two gates combine in [`classify_proc`] / `build_inlinable_map`: the
//! precise interprocedural [`var_escape::pure_leaf`](crate::var_escape)
//! proof (`safe_to_inline` — FE-VARESCAPE's S4.1 soundness predicate) and
//! the structural shape. The per-statement [`stmt_is_splice_eligible`] check
//! then guards the verbatim body itself.
//!
//! # Soundness
//!
//! The verbatim shape leans entirely on [`stmt_is_splice_eligible`]: a body
//! statement may be lifted out of the proc frame only when it is an
//! [`Statement::Call`] with no `defs` (it cannot mutate the caller's
//! scope), whose command is in the frame-independent allow-list (no
//! `info` / `uplevel` / `upvar` / `return` / `break` / `continue`), and
//! whose arguments carry no `[cmd]` substitution. That makes the splice
//! observationally equivalent to the call regardless of which frame it
//! runs in. Empty bodies are trivially safe. Redefined procs are never
//! inlined (their body is ambiguous). Recursion cannot occur: a call to a
//! user proc is not in the splice-safe allow-list, so a self-calling body
//! is never verbatim-eligible.

use std::collections::{HashMap, HashSet};

use crate::ir::{Module, Procedure, Script, Statement};
use crate::var_escape::ProcEscapeSummary;

/// Per-proc inlining eligibility — the catalogue tag. Mirrors Python's
/// `InlineDecision`, restricted to the cases the v0 / verbatim mechanism
/// acts on. `IfSingleCall` is intentionally inert here (its profitability
/// depends on post-inline proc pruning, not yet ported).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineDecision {
    /// Always inline a call to this proc.
    Always,
    /// Inline only when the proc has exactly one static caller (not yet
    /// acted on — recorded for parity / future pruning).
    IfSingleCall,
    /// Never inline.
    Never,
}

/// A pure-leaf proc whose body has at most this many flat statements is
/// unconditionally inlinable. Mirrors `decision.SMALL_BODY_THRESHOLD`.
pub const SMALL_BODY_THRESHOLD: usize = 5;

/// What an inlinable proc splices in at a call site.
#[derive(Debug, Clone)]
enum InlineSpec {
    /// v0 — empty body; the call vanishes.
    Empty,
    /// v1 / v2 — splice these body statements verbatim.
    Verbatim(Vec<Statement>),
}

/// Total number of leaf statements reachable in `script`, walking nested
/// control-flow bodies transitively (the inlined code-size cost). Mirrors
/// `decision.count_statements`.
#[must_use]
pub fn count_statements(script: &Script) -> usize {
    script.statements.iter().map(count_one).sum()
}

fn count_one(stmt: &Statement) -> usize {
    match stmt {
        Statement::Block { body, .. } | Statement::UpFrame { body, .. } => count_statements(body),
        Statement::If {
            clauses, else_body, ..
        } => {
            let mut n = 0;
            for c in clauses {
                n += 1 + count_statements(&c.body);
            }
            if let Some(b) = else_body {
                n += count_statements(b);
            }
            n
        }
        Statement::For {
            init, next, body, ..
        } => count_statements(init) + 1 + count_statements(next) + count_statements(body),
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. } => 1 + count_statements(body),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            let mut n = count_statements(body);
            for h in handlers {
                n += count_statements(&h.body);
            }
            if let Some(fb) = finally_body {
                n += count_statements(fb);
            }
            n
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            let mut n = 0;
            for a in arms {
                if let Some(b) = &a.body {
                    n += count_statements(b);
                }
            }
            if let Some(b) = default_body {
                n += count_statements(b);
            }
            n
        }
        _ => 1,
    }
}

/// Commands whose semantics are independent of the calling frame — a
/// wrapped call to one of these can be spliced into any caller's frame.
/// Frame-observing (`info` / `uplevel` / `upvar`) and frame-affecting
/// control flow (`return` / `break` / `continue`) are deliberately
/// excluded. Mirrors `_SPLICE_SAFE_COMMANDS`.
const SPLICE_SAFE_COMMANDS: &[&str] = &[
    // List primitives — pure value computation.
    "list", "lindex", "lrange", "linsert", "llength", "lsort", "lsearch", "lreverse", "lreplace",
    "lrepeat", "concat", // String primitives.
    "split", "join", "string", // Arithmetic.
    "expr", // I/O — observable but frame-independent.
    "puts",
];

/// Whether `command` (the head word as written) is a frame-independent,
/// splice-safe builtin. Mirrors `_command_is_splice_safe`.
fn command_is_splice_safe(command: &str) -> bool {
    // Accept a `::`-qualified spelling of a builtin too (`::puts`).
    let bare = command.strip_prefix("::").unwrap_or(command);
    SPLICE_SAFE_COMMANDS.contains(&bare)
}

/// Whether a statement can be lifted out of a wrapper proc and spliced
/// into a caller without changing observable behaviour. Mirrors
/// `_stmt_is_splice_eligible`.
fn stmt_is_splice_eligible(stmt: &Statement) -> bool {
    let Statement::Call {
        command, args, defs, ..
    } = stmt
    else {
        return false;
    };
    if !defs.is_empty() {
        return false;
    }
    if !command_is_splice_safe(command) {
        return false;
    }
    // No `[cmd]` substitution in any argument — its result would depend on
    // evaluation in the original frame's command-resolution context.
    if args.iter().any(|a| a.contains('[')) {
        return false;
    }
    true
}

/// Classify a procedure's *structural shape* for inlining. A proc is
/// [`InlineDecision::Always`] when it is not redefined and is either
/// empty-bodied (v0) or a small zero-parameter wrapper whose every body
/// statement is splice-eligible (v1 / v2). Everything else is
/// [`InlineDecision::Never`] in this port (v3 parameterised inlining is the
/// residual). The caller (`build_inlinable_map`) additionally gates on the
/// interprocedural `var_escape::safe_to_inline` (`pure_leaf`) soundness
/// proof — this function only judges the shape.
#[must_use]
pub fn classify_proc<S: std::hash::BuildHasher>(
    proc: &Procedure,
    redefined: &HashSet<String, S>,
) -> InlineDecision {
    if redefined.contains(&proc.qualified_name) {
        return InlineDecision::Never;
    }
    if proc.body.statements.is_empty() {
        return InlineDecision::Always;
    }
    if proc.params.is_empty()
        && count_statements(&proc.body) <= SMALL_BODY_THRESHOLD
        && proc.body.statements.iter().all(stmt_is_splice_eligible)
    {
        return InlineDecision::Always;
    }
    InlineDecision::Never
}

/// Build the qname → [`InlineSpec`] map of inlinable procedures.
///
/// Two gates combine: the precise `var_escape::pure_leaf` predicate
/// (`safe_to_inline`, the Python S4.1 soundness proof — FE-VARESCAPE) and
/// the structural shape classification ([`classify_proc`]). A proc must
/// pass both. The escape summaries are computed from the IR module, the
/// path that populates `pure_leaf`.
fn build_inlinable_map(module: &Module) -> HashMap<String, InlineSpec> {
    let summaries = crate::var_escape::analyse_var_escape(module, true);
    let mut map = HashMap::new();
    for (qname, proc) in &module.procedures {
        // Soundness gate: the interprocedural pure-leaf proof. An absent
        // summary (shouldn't happen for a module proc) is treated as opaque.
        if !summaries.get(qname).is_some_and(ProcEscapeSummary::safe_to_inline) {
            continue;
        }
        if classify_proc(proc, &module.redefined_procedures) != InlineDecision::Always {
            continue;
        }
        let spec = if proc.body.statements.is_empty() {
            InlineSpec::Empty
        } else {
            InlineSpec::Verbatim(proc.body.statements.clone())
        };
        map.insert(qname.clone(), spec);
    }
    map
}

/// Resolve a call's head word to a procedure qname, using the same naive
/// rule the optimiser folds use: an absolute name as-is, else rooted at
/// `::`. (Namespace-chain resolution is the shared O103 / inliner residual.)
fn resolve_qname(command: &str) -> String {
    if command.starts_with("::") {
        command.to_owned()
    } else {
        format!("::{command}")
    }
}

/// Inline eligible statement-position calls throughout `module`, returning
/// the rewritten module. Idempotent: a module with no remaining inlinable
/// sites is returned structurally unchanged. Mirrors `inline_module`'s v0 /
/// verbatim behaviour (v3 and dead-proc elimination are the residual).
#[must_use]
pub fn inline_module(mut module: Module) -> Module {
    let inlinable = build_inlinable_map(&module);
    if inlinable.is_empty() {
        return module;
    }
    rewrite_script(&mut module.top_level, &inlinable);
    let mut procedures = std::mem::take(&mut module.procedures);
    for proc in procedures.values_mut() {
        rewrite_script(&mut proc.body, &inlinable);
    }
    module.procedures = procedures;
    module
}

/// Rewrite a script in place: replace each inlinable statement-position
/// call with the proc's spliced statements (or nothing, for empty bodies),
/// then recurse into control-flow bodies.
fn rewrite_script(script: &mut Script, inlinable: &HashMap<String, InlineSpec>) {
    let mut out: Vec<Statement> = Vec::with_capacity(script.statements.len());
    for mut stmt in std::mem::take(&mut script.statements) {
        if let Statement::Call { command, .. } = &stmt
            && let Some(spec) = inlinable.get(&resolve_qname(command))
        {
            match spec {
                InlineSpec::Empty => {} // call vanishes
                InlineSpec::Verbatim(body) => out.extend(body.iter().cloned()),
            }
            continue;
        }
        recurse_into_bodies(&mut stmt, inlinable);
        out.push(stmt);
    }
    script.statements = out;
}

/// Recurse the inliner into a compound statement's nested bodies.
fn recurse_into_bodies(stmt: &mut Statement, inlinable: &HashMap<String, InlineSpec>) {
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                rewrite_script(&mut c.body, inlinable);
            }
            if let Some(b) = else_body {
                rewrite_script(b, inlinable);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            rewrite_script(init, inlinable);
            rewrite_script(next, inlinable);
            rewrite_script(body, inlinable);
        }
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => rewrite_script(body, inlinable),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            rewrite_script(body, inlinable);
            for h in handlers {
                rewrite_script(&mut h.body, inlinable);
            }
            if let Some(fb) = finally_body {
                rewrite_script(fb, inlinable);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &mut a.body {
                    rewrite_script(b, inlinable);
                }
            }
            if let Some(b) = default_body {
                rewrite_script(b, inlinable);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn module_for(source: &str) -> Module {
        CompilationUnit::build_for(source, &CommandRegistry::build_default(), false).ir_module
    }

    /// Count statement-position calls to `command` across the top level.
    fn top_calls_to(module: &Module, command: &str) -> usize {
        module
            .top_level
            .statements
            .iter()
            .filter(|s| matches!(s, Statement::Call { command: c, .. } if c == command))
            .count()
    }

    #[test]
    fn empty_body_call_vanishes() {
        let module = module_for("proc ::noop {} {}\nnoop\nputs done");
        assert_eq!(top_calls_to(&module, "noop"), 1);
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "noop"), 0, "empty-body call should vanish");
        // The unrelated `puts done` survives.
        assert_eq!(top_calls_to(&inlined, "puts"), 1);
    }

    #[test]
    fn verbatim_wrapper_body_is_spliced() {
        let module = module_for("proc ::greet {} { puts hello }\ngreet");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "greet"), 0, "wrapper call replaced");
        assert_eq!(
            top_calls_to(&inlined, "puts"),
            1,
            "wrapper body spliced verbatim",
        );
    }

    #[test]
    fn multi_statement_verbatim_wrapper() {
        let module = module_for("proc ::two {} { puts a\n puts b }\ntwo");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "two"), 0);
        assert_eq!(top_calls_to(&inlined, "puts"), 2, "both body stmts spliced");
    }

    #[test]
    fn wrapper_with_defs_is_not_inlined() {
        // `set x 1` mutates the proc frame — not splice-eligible.
        let module = module_for("proc ::w {} { set x 1 }\nw");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "w"), 1, "frame-mutating body kept");
    }

    #[test]
    fn parameterised_proc_is_not_inlined() {
        // v3 (parameters) is the residual — left intact.
        let module = module_for("proc ::id {x} { puts $x }\nid 1");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "id"), 1, "parameterised proc kept");
    }

    #[test]
    fn redefined_proc_is_not_inlined() {
        let module = module_for("proc ::r {} { puts a }\nproc ::r {} { puts b }\nr");
        assert!(module.redefined_procedures.contains("::r"));
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "r"), 1, "redefined proc kept");
    }

    #[test]
    fn arg_command_subst_blocks_verbatim() {
        // `puts [clock seconds]` depends on frame command resolution.
        let module = module_for("proc ::w {} { puts [clock seconds] }\nw");
        let inlined = inline_module(module);
        assert_eq!(top_calls_to(&inlined, "w"), 1);
    }

    #[test]
    fn inline_inside_control_flow_body() {
        let module = module_for("proc ::greet {} { puts hi }\nif {1} { greet }");
        let inlined = inline_module(module);
        // The `greet` call inside the `if` body is inlined to `puts hi`.
        let if_has_puts = inlined.top_level.statements.iter().any(|s| {
            matches!(s, Statement::If { clauses, .. }
                if clauses.iter().any(|c| c.body.statements.iter().any(|b|
                    matches!(b, Statement::Call { command, .. } if command == "puts"))))
        });
        assert!(if_has_puts, "call inside if-body should be inlined");
    }

    #[test]
    fn count_statements_walks_nested_bodies() {
        let module = module_for("proc ::f {} { puts a\n if {1} { puts b\n puts c } }");
        let proc = &module.procedures["::f"];
        // `puts a` + the `if` clause (1) + its two `puts` = 4.
        assert_eq!(count_statements(&proc.body), 4);
    }
}
