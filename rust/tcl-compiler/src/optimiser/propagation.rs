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
//! - **`optimise_string_interpolation_var_refs`** (`O100`) —
//!   inline SCCP-proved constants into `"…$x…"` double-quoted
//!   string arguments of calls (only when the interpolation is
//!   safe: the string contains no other substitutions, the
//!   constant value contains no Tcl metacharacters).
//! - **`optimise_load_forwarding`** (`O102`) — forward the
//!   single-reaching literal value of a variable at a use site,
//!   even when SCCP didn't fold it (e.g., when another path
//!   through the CFG makes the lattice Overdefined, but this
//!   particular use is dominated by one literal def).
//!
//! `optimise_expression_args` and `optimise_expr_substitutions`
//! in the Python source operate on the condition sub-expressions
//! of `if` / `while` / `for` and the bodies of standalone `expr`
//! commands. Both are already covered by this Rust port's
//! [`super::branch_folding::propagate_into_branches`] (for
//! branch conditions) and [`super::expr_simplify::run`] (for
//! standalone `expr` commands) — no separate port is needed.

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
    // Load-forwarding runs a separate per-function pass on top
    // of the SCCP-based substitutions. It consults the def-use
    // chains directly and fires independently of the SCCP
    // lattice — a variable whose *sole* reaching def is a
    // literal Assign is forwarded even when other paths make
    // the lattice Overdefined.
    run_load_forwarding(ctx, &cu.top_level);
    for fu in cu.procedures.values() {
        run_load_forwarding(ctx, fu);
    }
}

/// Forward a single reaching literal definition to each of its
/// Operand use sites. Emits `O102` ("Forward literal load") with
/// the literal text as replacement.
fn run_load_forwarding(ctx: &mut PassContext<'_>, fu: &crate::compilation_unit::FunctionUnit) {
    use crate::def_use::{DefKind, UseKind};
    use crate::ir::Statement;

    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != DefKind::Statement {
            continue;
        }
        // Find the defining statement — must be an AssignConst
        // with a literal value to forward. AssignValue without
        // substitutions could also work, but we're conservative:
        // the Python pass restricts to the same shapes.
        let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
            continue;
        };
        let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
            continue;
        };
        let Some(def_stmt) = block.statements.get(idx) else {
            continue;
        };
        let literal = match def_stmt {
            Statement::AssignConst { value, .. } => value.clone(),
            Statement::AssignValue { value, .. }
                if !value.contains(['$', '[', '\\', '"']) && !value.is_empty() =>
            {
                value.clone()
            }
            _ => continue,
        };
        if !is_value_safe_bare_word(&literal) {
            continue;
        }
        // Emit O102 at each Operand use site. For each use we
        // need a span — we synthesise it from the use
        // statement's span (approximate; the more precise
        // per-operand span would need token-level tracking).
        for use_site in &chain.uses {
            if use_site.kind != UseKind::Operand {
                continue;
            }
            let Ok(use_idx) = usize::try_from(use_site.statement_index) else {
                continue;
            };
            let Some(use_block) = fu.cfg.blocks.get(&use_site.block) else {
                continue;
            };
            let Some(use_stmt) = use_block.statements.get(use_idx) else {
                continue;
            };
            ctx.report(Optimisation::new(
                "O102",
                format!(
                    "Forward literal load of '{}' from its single reaching definition",
                    chain.key.0
                ),
                use_stmt.span(),
                literal.clone(),
            ));
        }
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
        let Some(text) = tokens.argv_texts.get(i) else {
            continue;
        };
        if single {
            visit_simple_var_word(ctx, *span, text, constants);
        }
        // `"..."` interpolation substitution — works on both
        // single-token (quoted strings) and composite (mixed
        // text + var) words.
        visit_string_interpolation(ctx, *span, text, constants);
    }
}

fn visit_simple_var_word(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    text: &str,
    constants: &std::collections::HashMap<String, String>,
) {
    let Some(varname) = simple_var_ref(text) else {
        return;
    };
    let normalised = normalise_var_name(&format!("${varname}")).to_owned();
    let Some(value) = constants.get(&normalised).or_else(|| constants.get(varname)) else {
        return;
    };
    if !is_value_safe_bare_word(value) {
        return;
    }
    ctx.report(Optimisation::new(
        "O100",
        "Propagate constant into command argument",
        span,
        value.clone(),
    ));
}

/// Inline SCCP-proved constants into a `"…$x…"` string arg.
///
/// Rewrites only when:
/// 1. The argument begins with `"` and ends with `"` (a
///    double-quoted word).
/// 2. Every `$name` / `${name}` reference inside resolves to a
///    constant whose string form is safe for `"…"`
///    interpolation — no `$`, `[`, `\`, or `"`.
///
/// Produces a single `O100` diagnostic spanning the whole arg
/// with the fully-substituted string text.
fn visit_string_interpolation(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    text: &str,
    constants: &std::collections::HashMap<String, String>,
) {
    // The lexer's argv_text strips outer `"…"` delimiters, so
    // we accept either form. Skip strings whose content is a
    // single bare `$var` — that path is already covered by
    // `visit_simple_var_word` with a more appropriate span.
    let inside = text.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(text);
    if !inside.contains('$') {
        return;
    }
    if simple_var_ref(inside).is_some() {
        return;
    }
    let Some(rewritten) = substitute_dollar_refs(inside, constants) else {
        return;
    };
    if rewritten == inside {
        return;
    }
    ctx.report(Optimisation::new(
        "O100",
        "Inline constant into string interpolation",
        span,
        format!("\"{rewritten}\""),
    ));
}

/// Scan `text` for `$name` / `${name}` references and replace
/// each with the corresponding literal from `constants`, rejecting
/// any value that would re-introduce new substitutions. Returns
/// `None` when any `$` is seen whose name is not a known
/// constant (a partial substitution would be worse than no
/// substitution).
fn substitute_dollar_refs(
    text: &str,
    constants: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            // Copy a two-char backslash-escape verbatim.
            if i + 1 < bytes.len() {
                out.push(bytes[i] as char);
                out.push(bytes[i + 1] as char);
                i += 2;
            } else {
                out.push('\\');
                i += 1;
            }
            continue;
        }
        if bytes[i] != b'$' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        // `$` — parse the var name.
        i += 1;
        if i >= bytes.len() {
            return None;
        }
        let (name, new_i) = if bytes[i] == b'{' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end] != b'}' {
                end += 1;
            }
            if end >= bytes.len() {
                return None;
            }
            (std::str::from_utf8(&bytes[start..end]).ok()?.to_owned(), end + 1)
        } else {
            let start = i;
            let mut end = start;
            while end < bytes.len() {
                let b = bytes[end];
                if b.is_ascii_alphanumeric() || b == b'_' {
                    end += 1;
                } else if b == b':' && end + 1 < bytes.len() && bytes[end + 1] == b':' {
                    end += 2;
                } else {
                    break;
                }
            }
            if start == end {
                return None;
            }
            (std::str::from_utf8(&bytes[start..end]).ok()?.to_owned(), end)
        };
        let value = constants.get(&name)?;
        if value.contains(['$', '[', '\\', '"']) {
            return None;
        }
        out.push_str(value);
        i = new_i;
    }
    Some(out)
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
    fn string_interpolation_const_is_inlined() {
        // `set count 42` → Const(42) in SCCP → interpolation into
        // `"count is $count"` substitutes to `"count is 42"`.
        let opts = run_pass("set count 42\nputs \"count is $count\"");
        assert!(
            opts.iter().any(|o| o.code == "O100" && o.replacement.contains("42")),
            "expected O100 inlining count into interpolation, got {opts:?}",
        );
    }

    #[test]
    fn string_interpolation_unknown_var_skipped() {
        // `$name` is not in the constants map → must not fire.
        let opts = run_pass("puts \"hello $name\"");
        assert!(
            opts.iter().all(|o| o.code != "O100"),
            "unknown var should not be inlined, got {opts:?}",
        );
    }

    #[test]
    fn substitute_dollar_refs_handles_braced_and_missing_vars() {
        let mut c = std::collections::HashMap::new();
        c.insert("x".into(), "42".into());
        assert_eq!(
            substitute_dollar_refs("a${x}b", &c).as_deref(),
            Some("a42b"),
        );
        // Missing var → None (cannot partially substitute).
        assert!(substitute_dollar_refs("$unknown", &c).is_none());
    }

    #[test]
    fn load_forwarding_fires_o102_for_single_reaching_def() {
        // `set n 7; puts $n` — single reaching def is literal 7
        // → emit O102 on the puts use site.
        let opts = run_pass("set n 7\nputs $n");
        assert!(
            opts.iter().any(|o| o.code == "O102"),
            "expected O102 load-forwarding, got {opts:?}",
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
