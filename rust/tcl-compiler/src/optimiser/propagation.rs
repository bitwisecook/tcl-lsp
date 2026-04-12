//! Constant / copy propagation optimiser pass (C30f, partial).
//!
//! Ported from `core/compiler/optimiser/_propagation.py`. The
//! Python module exposes seven distinct entry points:
//!
//! - `optimise_expression_args` — propagate constants into
//!   `expr {…}` arguments of non-`expr` commands.
//! - `optimise_expr_substitutions` — transform inside `expr`
//!   bodies beyond the branch/conditional contexts.
//! - `optimise_static_proc_calls` — fold calls to pure procs
//!   with proven-constant returns.
//! - `optimise_constant_var_refs` — replace `$var` with its
//!   SCCP-proved literal in command arguments (**landed**).
//! - `optimise_string_interpolation_var_refs` — inline
//!   constants into `"…"` string interpolations.
//! - `optimise_return_terminator` — simplify `return $v` when
//!   `v` is constant.
//! - `optimise_load_forwarding` — forward the value of the most
//!   recent definition across contiguous reads.
//!
//! This strip lands the simplest of those — **`optimise_constant_var_refs`**
//! — producing **`O100`** ("Propagate constant into command
//! argument") for every `$var` reference at a call site that
//! SCCP proved to be a literal. The remaining six entry points
//! need either the deeper `expr_simplify` rewrites (deferred to
//! `C30e4`–`C30e7`) or the `C28` return-value inference
//! (deferred to the `C28` follow-up) — each will plug into
//! `run(ctx, cu)` without an API change when their prerequisites
//! land.

use crate::analyses::{ConstValue, LatticeValue};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{CommandTokens, Script, Statement};
use crate::naming::normalise_var_name;

use super::helpers::literals::{is_safe_word, is_static_var_word};
use super::{Optimisation, PassContext};

/// Run the propagation pass across every function.
///
/// Emits one `O100` per single-token `$var` argument that SCCP
/// proved to be a safe literal value. "Safe" means the value
/// parses as a decimal integer or as a bare identifier-shaped
/// word; string literals with Tcl metacharacters (`$`, `[`, `\`,
/// …) are skipped because substituting them as a bare word
/// would change the command's interpretation.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    run_function(ctx, &cu.top_level, &cu.ir_module.top_level);
    for (qname, fu) in &cu.procedures {
        let Some(proc) = cu.ir_module.procedures.get(qname) else {
            continue;
        };
        run_function(ctx, fu, &proc.body);
    }
}

fn run_function(ctx: &mut PassContext<'_>, fu: &FunctionUnit, script: &Script) {
    // Project the per-function SCCP lattice into a name → literal
    // map that survives only when every tracked version of the
    // variable collapses to the same single constant value.
    let constants = sccp_constants_for(fu);
    if constants.is_empty() {
        return;
    }
    walk_script(ctx, script, &constants);
}

fn walk_script(
    ctx: &mut PassContext<'_>,
    script: &Script,
    constants: &std::collections::HashMap<String, String>,
) {
    for stmt in &script.statements {
        walk_statement(ctx, stmt, constants);
    }
}

fn walk_statement(
    ctx: &mut PassContext<'_>,
    stmt: &Statement,
    constants: &std::collections::HashMap<String, String>,
) {
    match stmt {
        Statement::Call { tokens: Some(t), .. } => visit_call_tokens(ctx, t, constants),
        Statement::If { clauses, else_body, .. } => {
            for c in clauses {
                walk_script(ctx, &c.body, constants);
            }
            if let Some(b) = else_body {
                walk_script(ctx, b, constants);
            }
        }
        Statement::For { init, next, body, .. } => {
            walk_script(ctx, init, constants);
            walk_script(ctx, next, constants);
            walk_script(ctx, body, constants);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => walk_script(ctx, body, constants),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, body, constants);
            for h in handlers {
                walk_script(ctx, &h.body, constants);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, fb, constants);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, b, constants);
                }
            }
            if let Some(b) = default_body {
                walk_script(ctx, b, constants);
            }
        }
        _ => {}
    }
}

fn visit_call_tokens(
    ctx: &mut PassContext<'_>,
    tokens: &CommandTokens,
    constants: &std::collections::HashMap<String, String>,
) {
    for (i, span) in tokens.argv.iter().enumerate() {
        let single = tokens.single_token_word.get(i).copied().unwrap_or(false);
        if !single {
            continue;
        }
        let Some(text) = tokens.argv_texts.get(i) else {
            continue;
        };
        // Only target simple `$var` / `${var}` references.
        let Some(varname) = simple_var_ref(text) else {
            continue;
        };
        let normalised = normalise_var_name(&format!("${varname}")).to_owned();
        let Some(value) = constants.get(&normalised).or_else(|| constants.get(varname)) else {
            continue;
        };
        // Refuse to inline values that would re-introduce Tcl
        // substitutions as a bare word.
        if !is_value_safe_bare_word(value) {
            continue;
        }
        ctx.report(Optimisation::new(
            "O100",
            "Propagate constant into command argument",
            *span,
            value.clone(),
        ));
    }
}

/// Return the variable name inside a `$var` or `${var}` word, or
/// `None` if the text is not a simple var reference.
fn simple_var_ref(text: &str) -> Option<&str> {
    if let Some(rest) = text.strip_prefix("${") {
        return rest.strip_suffix('}').filter(|n| is_static_var_word(n));
    }
    text.strip_prefix('$').filter(|n| is_static_var_word(n))
}

/// Allow a literal to be inlined as a bare word if it is
/// decimal-integer-shaped or matches the safe-word grammar.
/// Rejects anything containing Tcl metacharacters that would
/// introduce new substitutions.
fn is_value_safe_bare_word(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.trim().parse::<i64>().is_ok() {
        return true;
    }
    is_safe_word(value)
}

fn sccp_constants_for(fu: &FunctionUnit) -> std::collections::HashMap<String, String> {
    use super::helpers::literals::format_constant;

    let mut per_var: std::collections::HashMap<String, Vec<&ConstValue>> =
        std::collections::HashMap::new();
    let mut dirty: std::collections::HashSet<String> = std::collections::HashSet::new();
    for ((name, _ver), lv) in &fu.sccp.values {
        if dirty.contains(name) {
            continue;
        }
        if let LatticeValue::Const(cv) = lv {
            per_var.entry(name.clone()).or_default().push(cv);
        } else {
            dirty.insert(name.clone());
            per_var.remove(name);
        }
    }
    let mut out = std::collections::HashMap::new();
    for (name, cvs) in per_var {
        let first = cvs[0];
        if !cvs.iter().all(|cv| *cv == first) {
            continue;
        }
        if let Some(text) = format_constant(first) {
            out.insert(name, text);
        }
    }
    out
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

    // -- internal helpers ---------------------------------------------------

    #[test]
    fn simple_var_ref_parses_bare_and_braced() {
        assert_eq!(simple_var_ref("$foo"), Some("foo"));
        assert_eq!(simple_var_ref("${bar}"), Some("bar"));
        assert!(simple_var_ref("$foo(idx)").is_none());
        assert!(simple_var_ref("plain").is_none());
    }

    #[test]
    fn safe_bare_word_accepts_int_and_safe_words() {
        assert!(is_value_safe_bare_word("42"));
        assert!(is_value_safe_bare_word("-7"));
        assert!(is_value_safe_bare_word("foo_bar"));
        assert!(!is_value_safe_bare_word(""));
        assert!(!is_value_safe_bare_word("has space"));
        assert!(!is_value_safe_bare_word("$dollar"));
    }

    // -- end-to-end tests --------------------------------------------------

    #[test]
    fn constant_int_propagates_into_call_arg() {
        let opts = run_pass("set x 42\nputs $x");
        assert!(
            opts.iter().any(|o| o.code == "O100" && o.replacement == "42"),
            "expected O100 propagating 42, got {opts:?}",
        );
    }

    #[test]
    fn unsafe_string_constant_is_not_propagated() {
        // Tcl metacharacters in the value → must not inline as
        // a bare word.
        let opts = run_pass("set x \"$other\"\nputs $x");
        assert!(
            opts.iter().all(|o| o.code != "O100"),
            "unsafe string should not be propagated, got {opts:?}",
        );
    }

    #[test]
    fn non_const_lattice_skipped() {
        // Two different writes to x → ConstSet or Overdefined,
        // not a single Const.
        let opts = run_pass("set x 1\nif {$cond} { set x 2 }\nputs $x");
        assert!(
            opts.iter().all(|o| o.code != "O100"),
            "non-const lattice should be skipped, got {opts:?}",
        );
    }

    #[test]
    fn braced_var_reference_also_propagated() {
        let opts = run_pass("set x 7\nputs ${x}");
        assert!(
            opts.iter().any(|o| o.code == "O100" && o.replacement == "7"),
            "expected O100 for braced var ref, got {opts:?}",
        );
    }

    #[test]
    fn run_passes_dispatches_propagation() {
        let cu = CompilationUnit::build_for("set x 9\nputs $x", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::Propagation]);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == "O100"),
            "expected O100 via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
