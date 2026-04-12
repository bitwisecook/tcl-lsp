//! Constant / copy propagation optimiser pass (C30f, partial).
//!
//! Ported from `core/compiler/optimiser/_propagation.py`. The
//! Python module exposes seven distinct entry points; this
//! Rust port currently lands three:
//!
//! - **`optimise_constant_var_refs`** (`O100`) — replace a
//!   single-token `$var` call argument with its SCCP-proved
//!   literal, when the value is safe to inline as a bare word.
//! - **`optimise_static_proc_calls`** (`O103`) — fold calls to
//!   pure procs whose return is a proven constant
//!   (`can_fold_static_calls` from C28x-return).
//! - **`optimise_return_terminator`** (`O104`) — rewrite
//!   `return $v` as `return K` when `v` is SCCP-constant.
//!
//! Still deferred (each needs token-level source machinery not
//! yet exposed by the pipeline):
//!
//! - `optimise_expression_args` — constants into `expr {…}`
//!   command arguments outside branch contexts.
//! - `optimise_expr_substitutions` — transform inside `expr`
//!   bodies of non-branch commands.
//! - `optimise_string_interpolation_var_refs` — inline
//!   constants into `"…"` interpolations.
//! - `optimise_load_forwarding` — most-recent-def forwarding.

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
    run_function(ctx, cu, &cu.top_level, &cu.ir_module.top_level);
    for (qname, fu) in &cu.procedures {
        let Some(proc) = cu.ir_module.procedures.get(qname) else {
            continue;
        };
        run_function(ctx, cu, fu, &proc.body);
    }
}

fn run_function(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    fu: &FunctionUnit,
    script: &Script,
) {
    // Project the per-function SCCP lattice into a name → literal
    // map that survives only when every tracked version of the
    // variable collapses to the same single constant value.
    let constants = sccp_constants_for(fu);
    walk_script(ctx, cu, script, &constants);
}

fn walk_script(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    script: &Script,
    constants: &std::collections::HashMap<String, String>,
) {
    for stmt in &script.statements {
        walk_statement(ctx, cu, stmt, constants);
    }
}

fn walk_statement(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    stmt: &Statement,
    constants: &std::collections::HashMap<String, String>,
) {
    match stmt {
        Statement::Call {
            span,
            command,
            args,
            tokens,
            ..
        } => {
            if let Some(t) = tokens {
                visit_call_tokens(ctx, t, constants);
            }
            try_fold_static_proc_call(ctx, cu, *span, command, args);
        }
        Statement::Return { span, value, expr, braced, .. } => {
            try_fold_return_terminator(ctx, *span, value.as_deref(), expr.as_ref(), *braced, constants);
        }
        Statement::If { clauses, else_body, .. } => {
            for c in clauses {
                walk_script(ctx, cu, &c.body, constants);
            }
            if let Some(b) = else_body {
                walk_script(ctx, cu, b, constants);
            }
        }
        Statement::For { init, next, body, .. } => {
            walk_script(ctx, cu, init, constants);
            walk_script(ctx, cu, next, constants);
            walk_script(ctx, cu, body, constants);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => walk_script(ctx, cu, body, constants),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, cu, body, constants);
            for h in handlers {
                walk_script(ctx, cu, &h.body, constants);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, cu, fb, constants);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, cu, b, constants);
                }
            }
            if let Some(b) = default_body {
                walk_script(ctx, cu, b, constants);
            }
        }
        _ => {}
    }
}

/// O103: if `command` resolves to a proc with `can_fold_static_calls`
/// and a `constant_return`, emit a rewrite replacing the call
/// with the literal return value.
fn try_fold_static_proc_call(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    span: tcl_lexer::Span,
    command: &str,
    _args: &[String],
) {
    use crate::interprocedural::ConstantReturn;

    let Some(ia) = cu.interproc.as_ref() else {
        return;
    };
    // Naive resolution: treat `command` as a qualified name when
    // it starts with `::`, else try `::command`. The Python side
    // does full namespace walking via `_resolve_summary_proc_name`;
    // this scaled-down version catches the common case of calls
    // written with their absolute names or at namespace root.
    let qname = if command.starts_with("::") {
        command.to_owned()
    } else {
        format!("::{command}")
    };
    let Some(summary) = ia.procedures.get(&qname) else {
        return;
    };
    if !summary.can_fold_static_calls {
        return;
    }
    let Some(cr) = &summary.constant_return else {
        return;
    };
    let replacement = match cr {
        ConstantReturn::Int(i) => i.to_string(),
        ConstantReturn::Float(f) => f.to_string(),
        ConstantReturn::Bool(true) => "1".to_owned(),
        ConstantReturn::Bool(false) => "0".to_owned(),
        ConstantReturn::Str(s) => {
            if is_value_safe_bare_word(s) {
                s.clone()
            } else {
                return;
            }
        }
    };
    ctx.report(Optimisation::new(
        "O103",
        format!("Fold pure-proc call to '{}' to its constant return", summary.qualified_name),
        span,
        replacement,
    ));
}

/// O104: rewrite `return $v` to `return K` when the SCCP
/// environment proves `v` is a constant. Works on the `value`
/// text since `Statement::Return::expr` is populated only when
/// the original source was `return [expr …]`.
fn try_fold_return_terminator(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    value: Option<&str>,
    expr: Option<&crate::expr_ast::ExprNode>,
    _braced: bool,
    constants: &std::collections::HashMap<String, String>,
) {
    use crate::naming::normalise_var_name;

    // Only fold `return $v` — numeric/bare literals and complex
    // values are left to richer passes.
    if expr.is_some() {
        return;
    }
    let Some(raw) = value else {
        return;
    };
    let v = raw.trim();
    let name = if let Some(n) = v.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        n.to_owned()
    } else if let Some(n) = v.strip_prefix('$') {
        n.to_owned()
    } else {
        return;
    };
    let normalised = normalise_var_name(&format!("${name}")).to_owned();
    let Some(resolved) = constants.get(&name).or_else(|| constants.get(&normalised)) else {
        return;
    };
    if !is_value_safe_bare_word(resolved) {
        return;
    }
    ctx.report(Optimisation::new(
        "O104",
        "Fold return of constant variable",
        span,
        format!("return {resolved}"),
    ));
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
    fn return_terminator_folds_constant_variable() {
        let opts = run_pass("proc ::f {} { set x 42; return $x }");
        assert!(
            opts.iter()
                .any(|o| o.code == "O104" && o.replacement.contains("42")),
            "expected O104 folding return $x to return 42, got {opts:?}",
        );
    }

    #[test]
    fn static_proc_call_folds_to_constant_return() {
        // ::answer returns 42 unconditionally and is pure — a
        // call to ::answer can be folded.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc ::answer {} { return 42 }\n::answer",
            &registry,
            false,
        )
        .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == "O103"),
            "expected O103 static-proc fold, got {:?}",
            ctx.optimisations,
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
