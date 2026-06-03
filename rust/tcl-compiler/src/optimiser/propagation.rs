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
//!   (`can_fold_static_calls` from C28x-return). Fires
//!   applicable rewrites when the call appears as a `[proc …]`
//!   command substitution inside another call's argv (the argv
//!   span is the rewrite target); the bare statement form stays
//!   hint-only because the call result is discarded and folding
//!   `::answer` to `42` would leave an invalid command name.
//! - **`optimise_return_terminator`** (`O100`) — rewrite
//!   `return $v` as `return K` when `v` is SCCP-constant.
//!   (Earlier Rust commits emitted `O104`; that code is reserved
//!   by the canonical optimisation-codes table for the
//!   string-build chain fold and was reassigned in the C* close-out
//!   audit.)
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
//!   particular use is dominated by one literal def). Fires
//!   applicable rewrites with argv-level spans when the use is
//!   a bare `$var` / `${var}` word in a `Statement::Call`; falls
//!   back to hint-only on statements without `CommandTokens` or
//!   on uses inside interpolated strings (the latter covered by
//!   the O100 string-interpolation path).
//!
//! `optimise_expression_args` and `optimise_expr_substitutions`
//! in the Python source operate on the condition sub-expressions
//! of `if` / `while` / `for` and the bodies of standalone `expr`
//! commands. Both are already covered by this Rust port's
//! `branch_folding::propagate_into_branches` (for
//! branch conditions) and [`super::expr_simplify::run`] (for
//! standalone `expr` commands) — no separate port is needed.

use crate::analyses::{ConstValue, LatticeValue};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{CommandTokens, Script, Statement};
use crate::naming::normalise_var_name;

use super::helpers::expr_simplify::try_unwrap_expr_in_expr;
use super::helpers::literals::{is_safe_word, is_static_var_word};
use super::helpers::spans::{full_quoted_string_span, full_word_span};
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
///
/// When the consuming statement is a `Statement::Call` with
/// [`CommandTokens`] populated, fire one applicable `O102` per
/// argv entry whose text is `$var` / `${var}` matching the
/// defined variable — the argv span is the precise rewrite
/// target. Fall back to a hint-only diagnostic covering the whole
/// consuming statement when no `CommandTokens` are present or the
/// use is on a non-Call statement (where we still don't have
/// per-operand spans).
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
        let var_name = chain.key.0.as_str();
        let message =
            format!("Forward literal load of '{var_name}' from its single reaching definition");
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
            // Prefer a per-argv applicable rewrite when the use
            // lives inside a `Statement::Call` whose tokens we
            // tracked. Each matching `$var` / `${var}` word gets
            // its own O102 with the argv span as target.
            let mut emitted_applicable = false;
            if let Statement::Call {
                tokens: Some(tokens),
                ..
            } = use_stmt
            {
                for (i, argv_span) in tokens.argv.iter().enumerate() {
                    let Some(text) = tokens.argv_texts.get(i) else {
                        continue;
                    };
                    if !simple_var_ref_matches(text, var_name) {
                        continue;
                    }
                    ctx.report(Optimisation::new(
                        "O102",
                        message.clone(),
                        full_word_span(ctx.source, *argv_span),
                        literal.clone(),
                    ));
                    emitted_applicable = true;
                }
            }
            if emitted_applicable {
                continue;
            }
            // Fall back to a hint-only diagnostic on the whole
            // consuming statement when the use wasn't on a
            // Call, `CommandTokens` weren't captured, or no argv
            // entry matched as a simple `$var` word (e.g. the
            // read is inside an interpolated string — the O100
            // string-interpolation path handles that).
            let mut opt =
                Optimisation::new("O102", message.clone(), use_stmt.span(), literal.clone());
            opt.hint_only = true;
            ctx.report(opt);
        }
    }
}

/// True when `text` is the bare word `$name` / `${name}` and the
/// parsed name equals `var_name` (including namespace-qualified
/// comparison via [`normalise_var_name`]).
fn simple_var_ref_matches(text: &str, var_name: &str) -> bool {
    let Some(name) = simple_var_ref(text) else {
        return false;
    };
    if name == var_name {
        return true;
    }
    // Compare normalised names so `$::ns::x` matches chain key
    // `::ns::x` (and vice versa), and `$x(0)` is rejected by
    // simple_var_ref already.
    normalise_var_name(&format!("${name}")) == normalise_var_name(&format!("${var_name}"))
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
                visit_call_cmd_subst_folds(ctx, cu, t, constants);
            }
            try_fold_static_proc_call(ctx, cu, *span, command, args);
        }
        // `set TARGET [cmd-sub]` lowers to `AssignValue` carrying the
        // full command's tokens (`["set", TARGET, "[cmd-sub]"]`). Walk
        // its value words for the value-position cmd-sub folds — O115
        // redundant-nested-expr collapse and the O103 pure-proc
        // constant-return fold — so a `set` target gets the same folds a
        // command-argument position already gets. Only the cmd-sub fold
        // path is wired (not `visit_call_tokens`): a bare `set y $c` RHS
        // is handled by SCCP / load-forwarding, and folding it here would
        // change behaviour beyond the documented gap. SYNC-JUN02b-6 (#519).
        //
        // Note: the *canonical* `set x [expr {[expr {E}]}]` does not reach
        // this arm — `extract_single_expr_arg` strips the outer `expr` at
        // lowering time, producing an `AssignExpr` whose `ExprCommand`
        // holds the inner `[expr {E}]`. So O115 fires here only for the
        // rarer AssignValue forms that retain a nested-expr cmd-sub word;
        // the O103 pure-proc fold is the common reachable case.
        Statement::AssignValue {
            tokens: Some(t), ..
        } => {
            visit_call_cmd_subst_folds(ctx, cu, t, constants);
        }
        Statement::Return {
            span,
            value,
            expr,
            braced,
            ..
        } => {
            try_fold_return_terminator(
                ctx,
                *span,
                value.as_deref(),
                expr.as_ref(),
                *braced,
                constants,
            );
        }
        Statement::AssignExpr { span, name, expr } => {
            try_substitute_assign_expr(ctx, *span, name, expr, constants);
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(ctx, cu, &c.body, constants);
            }
            if let Some(b) = else_body {
                walk_script(ctx, cu, b, constants);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
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
    // The span here covers the whole Statement::Call — applying
    // the literal replacement would turn `::answer` into a
    // command named `42`, which is invalid Tcl. The Python pass
    // targets `[procName …]` command substitutions with their
    // token span instead. Until the Rust side tracks CMD-subst
    // spans at the call argument level, emit as a hint so
    // editors surface the fold without proposing an applicable
    // quick-fix.
    let mut opt = Optimisation::new(
        "O103",
        format!(
            "Fold pure-proc call to '{}' to its constant return",
            summary.qualified_name
        ),
        span,
        replacement,
    );
    opt.hint_only = true;
    ctx.report(opt);
}

/// O100: rewrite `return $v` to `return K` when the SCCP
/// environment proves `v` is a constant. Works on the `value`
/// text since `Statement::Return::expr` is populated only when
/// the original source was `return [expr …]`.
///
/// Uses `O100` (the canonical constant-propagation code) rather
/// than the `O104` that earlier Rust commits emitted — `O104` is
/// reserved by `docs/generated/optimisation_codes.md` for the
/// pattern-recognition string-build chain fold (matching the
/// Python optimiser's allocation).
fn try_fold_return_terminator(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    value: Option<&str>,
    expr: Option<&crate::expr_ast::ExprNode>,
    _braced: bool,
    constants: &std::collections::HashMap<String, String>,
) {
    use crate::naming::normalise_var_name;

    // O115: `return [expr {[expr {E}]}]` → `return [expr {E}]`. Checked
    // before the `expr`-gate below because the return value of a cmd-sub
    // also populates `expr`, yet the redundant-nested-expr collapse
    // operates on the raw value text.
    if let Some(collapsed) = value.and_then(|raw| o115_redundant_nested_expr(raw.trim())) {
        ctx.report(Optimisation::new(
            "O115",
            "Remove redundant nested expr",
            span,
            format!("return {collapsed}"),
        ));
        return;
    }

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
    // SYNC-JUN02b-1: render the constant as a single self-contained word
    // rather than bailing on metacharacters (`return {Hello World}`).
    let word = render_propagation_word(resolved);
    ctx.report(Optimisation::new(
        "O100",
        "Fold return of constant variable",
        span,
        format!("return {word}"),
    ));
}

/// O100 (`AssignExpr` form): substitute SCCP-proved constants
/// into ``set name [expr { … }]`` expressions.
///
/// Builds on ``substitute_expr_constants`` — the same helper the
/// branch-condition pass uses. Emits an O100 rewrite targeting
/// the whole ``set`` statement with the substituted expression
/// re-wrapped in ``[expr { … }]``. The rewrite span is extended
/// via ``full_rewrite_span`` so nested substitutions don't leave
/// orphan delimiters.
fn try_substitute_assign_expr(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    name: &str,
    expr: &crate::expr_ast::ExprNode,
    constants: &std::collections::HashMap<String, String>,
) {
    use super::helpers::expr_simplify::{
        expr_has_command_subst, instcombine_expr, substitute_expr_constants,
    };
    use super::helpers::spans::full_rewrite_span;
    use crate::expr_parser::parse_expr;
    use crate::tcl_expr_eval::{eval_tcl_expr, format_tcl_value, Env};

    if matches!(expr, crate::expr_ast::ExprNode::Raw { .. }) {
        return;
    }
    if expr_has_command_subst(expr) {
        return;
    }
    let expr_text = crate::expr_ast::render_expr(expr);
    let result = substitute_expr_constants(&expr_text, constants, ctx.dialect);
    if !result.changed {
        return;
    }
    // After substitution, try to fold the result to a constant
    // — matches the Python cascade where O100 enables O101. When
    // the substituted expression is fully constant we can emit
    // the unwrapped ``set name VALUE`` form directly. Otherwise
    // keep the expression wrapper around the substituted text.
    let parsed = parse_expr(&result.text, ctx.dialect);
    let env = Env::new();
    if let Some(val) = eval_tcl_expr(&parsed, &env) {
        let folded = format_tcl_value(val);
        let needs_quoting = folded.is_empty()
            || folded.contains([
                ' ', '\t', '\n', '\r', '$', '[', ']', '{', '}', '"', '\\', '\0', ';',
            ]);
        if !needs_quoting {
            ctx.report(Optimisation::new(
                "O100",
                "Propagate constant and fold",
                full_rewrite_span(ctx.source, span),
                format!("set {name} {folded}"),
            ));
            return;
        }
    }
    // Not fully constant; try one pass of instcombine on the
    // substituted expression to pick up identity simplifications
    // (e.g. ``$a + 0`` after ``$a`` folds to ``3``).
    let (simplified, _changed) = instcombine_expr(&result.text, false);
    let final_text = if simplified.trim().is_empty() {
        result.text.clone()
    } else {
        simplified
    };
    ctx.report(Optimisation::new(
        "O100",
        "Propagate constant into expr argument",
        full_rewrite_span(ctx.source, span),
        format!("set {name} [expr {{{final_text}}}]"),
    ));
}

/// O115 (value position): collapse a redundant double-`expr` command
/// substitution `[expr {[expr {E}]}]` to `[expr {E}]`, returning the
/// inner cmd-sub when *word* has that shape.
///
/// Sound because both forms evaluate `E` as an expression. The gate is
/// the double-unwrap: the outer `[expr {…}]`'s body must itself be an
/// `[expr {…}]` cmd-sub, so the collapsed form stays a valid value-
/// position word. A plain `[expr {$x + 1}]` (whose unwrap `$x + 1`
/// would be invalid as a bare value) — or an `[expr {[other]}]` whose
/// inner is not itself `expr` (not value-equivalent for non-numeric
/// results) — yields `None`. Mirrors the O115 value-position path added
/// to `optimise_expr_substitutions` (#519).
fn o115_redundant_nested_expr(word: &str) -> Option<String> {
    let expr_arg = try_unwrap_expr_in_expr(word)?;
    try_unwrap_expr_in_expr(&expr_arg)?;
    Some(expr_arg)
}

/// O103 (CMD-subst form): walk each argv word looking for a
/// command substitution `[cmd …]` whose head resolves to a proc
/// with `can_fold_static_calls` and a proven `constant_return`.
/// Emits an applicable rewrite with the argv span and the literal
/// return value as replacement.
///
/// This is the "word-level" companion to the bare-call form in
/// [`try_fold_static_proc_call`], which stays hint-only because
/// folding `::answer` as a statement would turn the discarded
/// call into a bare `42` (invalid as a command name).
fn visit_call_cmd_subst_folds(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    tokens: &CommandTokens,
    constants: &std::collections::HashMap<String, String>,
) {
    use crate::interprocedural::ConstantReturn;

    for (i, argv_span) in tokens.argv.iter().enumerate() {
        let single = tokens.single_token_word.get(i).copied().unwrap_or(false);
        if !single {
            continue;
        }
        let Some(text) = tokens.argv_texts.get(i) else {
            continue;
        };
        let Some(inner) = text.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
            continue;
        };
        // O115: collapse a redundant double-`expr` cmd-sub in this
        // argument value position (needs no interproc summary).
        if let Some(collapsed) = o115_redundant_nested_expr(text) {
            ctx.report(Optimisation::new(
                "O115",
                "Remove redundant nested expr",
                full_word_span(ctx.source, *argv_span),
                collapsed,
            ));
            continue;
        }
        // O129: fold a pure-builtin cmd-sub with constant (literal) args
        // through the registry `const_fold` callback (no interproc
        // needed). Checked before the O103 interproc bail so it fires
        // even when no interprocedural summary is available.
        if let Some(reg) = ctx.registry {
            if let Some(folded) = try_o129_fold(reg, &ctx.command_mutations, constants, inner) {
                ctx.report(Optimisation::new(
                    "O129",
                    "Fold constant builtin command substitution",
                    full_word_span(ctx.source, *argv_span),
                    folded,
                ));
                continue;
            }
        }
        // O103 (below) folds a pure-proc cmd-sub to its constant return.
        let Some(ia) = cu.interproc.as_ref() else {
            continue;
        };
        let Some(head) = parse_cmd_subst_head(inner) else {
            continue;
        };
        let qname = if head.starts_with("::") {
            head.to_owned()
        } else {
            format!("::{head}")
        };
        let Some(summary) = ia.procedures.get(&qname) else {
            continue;
        };
        if !summary.can_fold_static_calls {
            continue;
        }
        let Some(cr) = &summary.constant_return else {
            continue;
        };
        let replacement = match cr {
            ConstantReturn::Int(i) => i.to_string(),
            ConstantReturn::Float(f) => f.to_string(),
            ConstantReturn::Bool(true) => "1".to_owned(),
            ConstantReturn::Bool(false) => "0".to_owned(),
            // B3 (SYNC-JUN02d-2, #525): a multi-word string return folds
            // too, list-quoted as a single word via the canonical quoter
            // (the cmd-sub is one argument word) — `set msg {a b}; return
            // $msg` in the callee no longer blocks the fold.
            ConstantReturn::Str(s) => render_propagation_word(s),
        };
        ctx.report(Optimisation::new(
            "O103",
            format!(
                "Fold pure-proc call to '{}' to its constant return",
                summary.qualified_name
            ),
            full_word_span(ctx.source, *argv_span),
            replacement,
        ));
    }
}

/// Parse the head word out of a CMD-subst interior. Returns
/// `None` when the head word is empty or contains metacharacters
/// that would change the parsed command name under substitution.
fn parse_cmd_subst_head(inner: &str) -> Option<&str> {
    let trimmed = inner.trim_start();
    let end = trimmed
        .find(|c: char| c.is_whitespace())
        .unwrap_or(trimmed.len());
    let head = &trimmed[..end];
    if head.is_empty() {
        return None;
    }
    if head.contains(['$', '[', '\\', '"', '{', '(', '}', ']']) {
        return None;
    }
    Some(head)
}

/// O129 (SYNC-JUN02b-6, #519): fold a pure-builtin command substitution
/// `[cmd args…]` (or `[cmd sub args…]`) whose arguments are constant
/// literals, by consulting the registry `const_fold` callback for the
/// resolved command (or subcommand). Returns the rendered replacement
/// word, or `None` when the head isn't a const-foldable builtin, an
/// argument isn't a clean literal, or the fold itself declines.
///
/// The result is rendered through [`render_propagation_word`] so a
/// multi-word fold result (`string toupper {a b}` → `A B`) is emitted
/// as a single brace-quoted word.
///
/// SYNC-JUN02b-4: gated by `mutations.trusts(head)` — if the command was
/// renamed / redefined anywhere in the module, it is no longer its
/// original builtin and must not be folded with the builtin's semantics.
fn try_o129_fold(
    registry: &tcl_registry::CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
    constants: &std::collections::HashMap<String, String>,
    inner: &str,
) -> Option<String> {
    let folded = fold_builtin_cmd_subst_raw(registry, mutations, constants, inner)?;
    Some(render_propagation_word(&folded))
}

/// The shared core of the O129 fold: resolve the cmd-sub head to its
/// spec (or subcommand), check all args are clean literals, and run the
/// registry `const_fold`, returning the **raw** result (no
/// single-word quoting).  [`try_o129_fold`] wraps this with
/// [`render_propagation_word`] for free-standing argument positions; the
/// embedded-interpolation path splices the raw result directly into the
/// surrounding string.
fn fold_builtin_cmd_subst_raw(
    registry: &tcl_registry::CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
    constants: &std::collections::HashMap<String, String>,
    inner: &str,
) -> Option<String> {
    let words = literal_words(inner, constants)?;
    let (head, rest) = words.split_first()?;
    if !mutations.trusts(head) {
        return None;
    }
    let spec = registry.get(head)?;
    if spec.subcommands.is_empty() {
        let fold = spec.const_fold?;
        let arg_refs: Vec<&str> = rest.iter().map(String::as_str).collect();
        fold(&arg_refs)
    } else {
        // Subcommand-dispatched builtin (`string`, `dict`, …): the fold
        // lives on the matching subcommand and sees the args after it.
        let (sub, sub_rest) = rest.split_first()?;
        let fold = spec.subcommand(sub)?.const_fold?;
        let arg_refs: Vec<&str> = sub_rest.iter().map(String::as_str).collect();
        fold(&arg_refs)
    }
}

/// Re-lex a command-substitution interior into its literal words for the
/// O129 const-fold. Returns `None` (bail — do not fold) if any word is
/// not a single clean literal token: a `$var` / `[cmd]` substitution
/// (`Var` / `Cmd`), a multi-token word (`foo$bar`), or a word whose text
/// carries a backslash escape (decoding is out of scope here). A braced
/// literal (`{a b}`, `{a$b}`) yields its interior text — the contents
/// are literal, so they fold soundly.
fn literal_words(
    inner: &str,
    constants: &std::collections::HashMap<String, String>,
) -> Option<Vec<String>> {
    use tcl_lexer::{Lexer, SourceMap, TokenType};

    let sm = SourceMap::new(inner);
    let tokens = Lexer::new(inner).tokenise_all().ok()?;
    let mut words: Vec<String> = Vec::new();
    let mut prev_is_sep = true;
    for tok in &tokens {
        match tok.kind {
            TokenType::Sep | TokenType::Eol | TokenType::Eof | TokenType::Comment => {
                prev_is_sep = true;
            }
            TokenType::Esc | TokenType::Str => {
                if !prev_is_sep {
                    return None; // multi-token word — not a clean literal
                }
                let text = sm.token_text(*tok);
                if text.contains('\\') {
                    return None; // unhandled escape — bail conservatively
                }
                words.push(text.to_owned());
                prev_is_sep = false;
            }
            TokenType::Var => {
                // B2 (SYNC-JUN02d-2, #525): resolve a single-token `$var`
                // word to its constant value (kept as ONE argument so a
                // multi-word value isn't re-split).  The Rust SCCP
                // `constants` map is whole-function (a var is present only
                // if every reaching def agrees), so substituting it here
                // is sound without Python's same-block reaching-version
                // gating.  A composite word (`foo$bar`), an array element
                // (`$a(1)` — never a scalar constant), or a non-constant
                // var bails.
                if !prev_is_sep {
                    return None;
                }
                let name = sm.token_text(*tok);
                let normalised = normalise_var_name(&format!("${name}")).to_owned();
                let value = constants.get(&normalised).or_else(|| constants.get(name))?;
                words.push(value.clone());
                prev_is_sep = false;
            }
            // Cmd (and any future kind) → substitution-bearing.
            _ => return None,
        }
    }
    Some(words)
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
        // B1 (SYNC-JUN02d-2): fold a pure-builtin `[cmd …]` substitution
        // embedded *inside* an interpolation string.
        visit_string_interpolation_cmd_subs(ctx, *span, text, constants);
    }
}

/// True when `inside` is exactly one bracket-balanced `[…]` covering the
/// whole word (a free-standing command substitution), as opposed to a
/// sub embedded in surrounding interpolation text.
fn is_whole_word_cmd_subst(inside: &str) -> bool {
    let b = inside.as_bytes();
    if b.first() != Some(&b'[') {
        return false;
    }
    let mut depth = 0u32;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return i == b.len() - 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

/// B1 (SYNC-JUN02d-2, #525): fold a pure-builtin command substitution
/// embedded *inside* an interpolation string, splicing the raw result
/// into the surrounding `"…"` (O129): `puts "v=[string length abc]"` →
/// `puts "v=5"`.
///
/// Each `[cmd …]` whose head resolves to a const-foldable builtin with
/// clean literal args is folded via [`fold_builtin_cmd_subst_raw`] and
/// the raw result spliced in.  Soundness guards: the result must not
/// reintroduce a substitution into the `"…"` context — a result
/// carrying `$`, `[`, `]`, `\`, or `"` is left unfolded.  `$var`
/// substitutions and any non-foldable `[cmd]` are kept verbatim; at
/// least one successful fold is required to emit.
fn visit_string_interpolation_cmd_subs(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    text: &str,
    constants: &std::collections::HashMap<String, String>,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let inside = text
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(text);
    if !inside.contains('[') {
        return;
    }
    // A whole-word `[…]` cmd-sub is already folded by
    // `visit_call_cmd_subst_folds`; B1 only handles a sub *embedded* in
    // surrounding interpolation text (else we'd emit a duplicate O129).
    if is_whole_word_cmd_subst(inside) {
        return;
    }
    // Byte scan is UTF-8-safe: the structural bytes we match (`[`, `]`,
    // `\`, `"`) are all < 0x80, so they never occur inside a multi-byte
    // sequence.  Text runs are flushed as `&str` slices (char-boundary
    // safe) only when a fold actually happens; unfolded subs stay in the
    // trailing un-flushed region.
    let bytes = inside.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut last = 0; // start of the not-yet-flushed text
    let mut i = 0;
    let mut folded_any = false;
    while i < n {
        match bytes[i] {
            b'\\' => i += 2, // skip an escaped pair (so `\[` is not a sub)
            b'[' => {
                let start = i;
                let mut depth = 0u32;
                let mut j = i;
                let mut close = None;
                while j < n {
                    match bytes[j] {
                        b'\\' => j += 1,
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                close = Some(j);
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                let Some(close) = close else {
                    break; // unbalanced — leave the rest in `[last..]`
                };
                let cmd = &inside[start + 1..close];
                if let Some(result) =
                    fold_builtin_cmd_subst_raw(registry, &ctx.command_mutations, constants, cmd)
                {
                    // Reject a result that would re-introduce a
                    // substitution into the `"…"` context.
                    if !result
                        .bytes()
                        .any(|b| matches!(b, b'$' | b'[' | b']' | b'\\' | b'"'))
                    {
                        out.push_str(&inside[last..start]);
                        out.push_str(&result);
                        last = close + 1;
                        folded_any = true;
                    }
                }
                i = close + 1;
            }
            _ => i += 1,
        }
    }
    if !folded_any {
        return;
    }
    out.push_str(&inside[last..]);
    let rewrite_span = full_quoted_string_span(ctx.source, span);
    ctx.report(Optimisation::new(
        "O129",
        "Fold constant builtin command substitution in interpolation",
        rewrite_span,
        format!("\"{out}\""),
    ));
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
    let Some(value) = constants
        .get(&normalised)
        .or_else(|| constants.get(varname))
    else {
        return;
    };
    // SYNC-JUN02b-1: re-render a metacharacter-bearing constant as a
    // single self-contained word instead of bailing.
    let word = render_propagation_word(value);
    ctx.report(Optimisation::new(
        "O100",
        "Propagate constant into command argument",
        span,
        word,
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
    let inside = text
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(text);
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
    // The argv span for a composite `"$a $b $c"` word holds only
    // the opening-quote token (e.g. `"$` at 5..7). Extend to the
    // matching close quote so the rewrite target covers the full
    // string — otherwise we leave trailing `$b $c"` garbage.
    let rewrite_span = full_quoted_string_span(ctx.source, span);
    ctx.report(Optimisation::new(
        "O100",
        "Inline constant into string interpolation",
        rewrite_span,
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
            (
                std::str::from_utf8(&bytes[start..end]).ok()?.to_owned(),
                end + 1,
            )
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
            (
                std::str::from_utf8(&bytes[start..end]).ok()?.to_owned(),
                end,
            )
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

/// SYNC-JUN02b-1 (#519): render a constant `value` as a single,
/// self-contained Tcl word for O100 propagation into a command-argument
/// or `return` value position.
///
/// A safe bare word (integer or `[A-Za-z0-9_./:+-]`-only identifier) is
/// emitted verbatim — its existing parity-tested behaviour. Anything
/// else (whitespace, `$` / `[` / `;` / quotes, unbalanced braces, a
/// trailing backslash, …) is re-rendered through the canonical
/// `TclConvertElement`-style quoter [`crate::codegen::helpers::tcl_list_element`]
/// (bare / brace-quoted / backslash-escaped, verified against Tcl's
/// `list`), so `set msg {Hello World}; return $msg` collapses to
/// `return {Hello World}` instead of bailing. The quoter never fails —
/// it always yields a word that re-evaluates to the literal `value` —
/// so this is total.
fn render_propagation_word(value: &str) -> String {
    if is_value_safe_bare_word(value) {
        value.to_owned()
    } else {
        crate::codegen::helpers::tcl_list_element(value)
    }
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
            opts.iter()
                .any(|o| o.code == "O100" && o.replacement == "42"),
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
    fn o100_multi_word_constant_renders_via_quoter() {
        // SYNC-JUN02b-1 (#519): a constant containing whitespace or
        // metacharacters is re-rendered as a single self-contained word
        // via the canonical quoter instead of bailing.
        // `return` value position → `return {Hello World}`.
        let ret = run_pass("proc ::f {} { set msg {Hello World}\nreturn $msg }");
        assert!(
            ret.iter()
                .any(|o| o.code == "O100" && o.replacement == "return {Hello World}"),
            "expected O100 `return {{Hello World}}`, got {ret:?}",
        );
        // Command-argument position → `puts {Hello World}`.
        let arg = run_pass("proc ::f {} { set msg {Hello World}\nputs $msg }");
        assert!(
            arg.iter()
                .any(|o| o.code == "O100" && o.replacement == "{Hello World}"),
            "expected O100 `{{Hello World}}` arg, got {arg:?}",
        );
        // A `$`/`[`-bearing literal constant is brace-quoted (the value
        // is literal — braces suppress the would-be substitution).
        let meta = run_pass("proc ::f {} { set m {a$b}\nputs $m }");
        assert!(
            meta.iter()
                .any(|o| o.code == "O100" && o.replacement == "{a$b}"),
            "expected O100 `{{a$b}}` arg, got {meta:?}",
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
            opts.iter()
                .any(|o| o.code == "O100" && o.replacement == "7"),
            "expected O100 for braced var ref, got {opts:?}",
        );
    }

    #[test]
    fn return_terminator_folds_constant_variable() {
        let opts = run_pass("proc ::f {} { set x 42; return $x }");
        assert!(
            opts.iter()
                .any(|o| o.code == "O100" && o.replacement.contains("42")),
            "expected O100 folding return $x to return 42, got {opts:?}",
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
            opts.iter()
                .any(|o| o.code == "O100" && o.replacement.contains("42")),
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
    fn o102_operand_span_fires_applicable_on_call_argv() {
        // `set n 7; puts $n` — the O102 emitted on the puts use
        // must target the `$n` argv span (not the whole Call) and
        // be applicable (not hint-only). The argv span lands on
        // the last 2 chars of the source: `$n`.
        let source = "set n 7\nputs $n";
        let opts = run_pass(source);
        let o102s: Vec<_> = opts.iter().filter(|o| o.code == "O102").collect();
        assert!(
            !o102s.is_empty(),
            "expected at least one O102, got {opts:?}"
        );
        let found = o102s.iter().any(|o| {
            let start = o.span.start() as usize;
            let end = o.span.end() as usize;
            !o.hint_only
                && o.replacement == "7"
                && end <= source.len()
                && &source[start..end] == "$n"
        });
        assert!(
            found,
            "expected applicable O102 with argv span covering `$n`, got {o102s:?}"
        );
    }

    #[test]
    fn o102_operand_span_matches_braced_var_word() {
        // ${n} forms must also be recognised at the argv-text
        // level and produce an applicable rewrite.
        let source = "set n 7\nputs ${n}";
        let opts = run_pass(source);
        let o102s: Vec<_> = opts.iter().filter(|o| o.code == "O102").collect();
        assert!(
            o102s.iter().any(|o| !o.hint_only
                && o.replacement == "7"
                && &source[o.span.start() as usize..o.span.end() as usize] == "${n}"),
            "expected applicable O102 over ${{n}}, got {o102s:?}",
        );
    }

    #[test]
    fn o102_falls_back_to_hint_only_when_var_is_inside_interpolation() {
        // `$n` appears only inside a double-quoted composite word
        // — no argv text matches the bare `$n` form, so the
        // rewrite stays hint-only (the O100 string-interpolation
        // path handles the actual inlining).
        let source = "set n 7\nputs \"n=$n\"";
        let opts = run_pass(source);
        let o102s: Vec<_> = opts.iter().filter(|o| o.code == "O102").collect();
        assert!(!o102s.is_empty(), "expected O102 hint, got {opts:?}");
        assert!(
            o102s.iter().all(|o| o.hint_only),
            "expected all O102s to be hint-only for interpolated use, got {o102s:?}",
        );
    }

    #[test]
    fn o103_cmd_subst_fires_applicable_rewrite() {
        // `[::answer]` inside an argv → applicable O103 with the
        // argv span covering `[::answer]` and the constant return
        // as replacement.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let source = "proc ::answer {} { return 42 }\nputs [::answer]";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        let o103s: Vec<_> = ctx
            .optimisations
            .iter()
            .filter(|o| o.code == "O103")
            .collect();
        assert!(
            o103s.iter().any(|o| !o.hint_only
                && o.replacement == "42"
                && &source[o.span.start() as usize..o.span.end() as usize] == "[::answer]"),
            "expected applicable O103 spanning `[::answer]`, got {o103s:?}",
        );
    }

    #[test]
    fn o103_folds_multi_word_string_return_list_quoted() {
        // B3 (SYNC-JUN02d-2, #525): a pure proc returning a multi-word
        // string folds in a cmd-sub position, list-quoted as one word
        // (previously bailed because `a b c` is not a safe bare word).
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let source = "proc ::greet {} { return \"a b c\" }\nputs [::greet]";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        let o103s: Vec<_> = ctx
            .optimisations
            .iter()
            .filter(|o| o.code == "O103")
            .collect();
        assert!(
            o103s
                .iter()
                .any(|o| !o.hint_only && o.replacement == "{a b c}"),
            "expected applicable O103 folding [::greet] to {{a b c}}, got {o103s:?}",
        );
    }

    #[test]
    fn o103_cmd_subst_fires_on_set_value_target() {
        // SYNC-JUN02b-6 (#519): the value-position cmd-sub folds must also
        // fire on a `set TARGET [cmd-sub]` target. `set` lowers to
        // `AssignValue`, which `walk_statement` did not visit for cmd-sub
        // folds, so `set y [::answer]` missed the O103 fold that the same
        // `[::answer]` gets in a command-argument position (`puts
        // [::answer]`). With the AssignValue arm wired, the set value
        // folds identically.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let source = "proc ::answer {} { return 42 }\nproc ::f {} { set y [::answer]\nputs $y }";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        let o103s: Vec<_> = ctx
            .optimisations
            .iter()
            .filter(|o| o.code == "O103")
            .collect();
        assert!(
            o103s.iter().any(|o| !o.hint_only
                && o.replacement == "42"
                && &source[o.span.start() as usize..o.span.end() as usize] == "[::answer]"),
            "expected applicable O103 spanning the `[::answer]` set value, got {o103s:?}",
        );
    }

    #[test]
    fn o103_bare_call_stays_hint_only() {
        // Top-level `::answer` — the call result is discarded, so
        // folding it as a statement would leave an invalid `42`
        // command. Diagnostic must stay hint-only.
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
        let o103s: Vec<_> = ctx
            .optimisations
            .iter()
            .filter(|o| o.code == "O103")
            .collect();
        assert!(!o103s.is_empty(), "expected at least one O103");
        assert!(
            o103s.iter().all(|o| o.hint_only),
            "bare-call form must stay hint-only, got {o103s:?}",
        );
    }

    #[test]
    fn o103_no_fire_when_head_is_not_constant_return() {
        // `::not_const` has no constant_return → O103 must not
        // fire for either the bare call or the CMD-subst form.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc ::not_const {x} { return $x }\nputs [::not_const 1]",
            &registry,
            false,
        )
        .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations.iter().all(|o| o.code != "O103"),
            "no O103 expected for non-constant-return proc, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn parse_cmd_subst_head_rejects_metachars() {
        assert_eq!(parse_cmd_subst_head("::answer"), Some("::answer"));
        assert_eq!(parse_cmd_subst_head("::answer 1 2"), Some("::answer"));
        assert_eq!(parse_cmd_subst_head("  ::answer"), Some("::answer"));
        assert_eq!(parse_cmd_subst_head(""), None);
        // Leading `$` / `[` / `{` / `"` → would re-substitute.
        assert_eq!(parse_cmd_subst_head("$cmd"), None);
        assert_eq!(parse_cmd_subst_head("[cmd]"), None);
    }

    #[test]
    fn o115_collapses_redundant_nested_expr_in_value_positions() {
        // SYNC-JUN02b-6 (#519): a redundant double-`expr` cmd-sub
        // `[expr {[expr {E}]}]` collapses to `[expr {E}]` in command-arg
        // and return value positions (previously O115 only fired on a
        // standalone `expr` statement).
        let collapsed = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code == "O115")
                .map(|o| o.replacement)
                .collect()
        };
        assert_eq!(
            collapsed("proc ::f {x} { puts [expr {[expr {$x * 2}]}] }"),
            vec!["[expr {$x * 2}]".to_string()],
            "command-arg position should collapse",
        );
        assert_eq!(
            collapsed("proc ::f {x} { return [expr {[expr {$x * 2}]}] }"),
            vec!["return [expr {$x * 2}]".to_string()],
            "return position should collapse",
        );
    }

    #[test]
    fn o115_value_position_is_sound() {
        // A single `[expr {$x + 1}]` must NOT collapse (its unwrap
        // `$x + 1` would be invalid as a bare value), and an
        // `[expr {[other]}]` whose inner is not itself `expr` is not
        // value-equivalent for non-numeric results.
        let has_o115 = |src: &str| -> bool {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .iter()
                .any(|o| o.code == "O115")
        };
        assert!(!has_o115("proc ::f {x} { puts [expr {$x + 1}] }"));
        assert!(!has_o115("proc ::f {x} { return [expr {$x + 1}] }"));
        assert!(!has_o115("proc ::f {x} { puts [expr {[someproc]}] }"));
    }

    #[test]
    fn o129_folds_pure_builtin_cmd_subst() {
        // SYNC-JUN02b-6 (#519): a pure-builtin cmd-sub with constant
        // (literal) args folds via the registry `const_fold` callback
        // (O129). `optimise_raw` sets `ctx.registry`, which the O129
        // path requires.
        let fold = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code == "O129")
                .map(|o| o.replacement)
                .collect()
        };
        assert_eq!(fold("puts [string toupper foo]"), vec!["FOO".to_string()]);
        assert_eq!(fold("puts [string tolower BAR]"), vec!["bar".to_string()]);
        assert_eq!(fold("puts [string reverse abc]"), vec!["cba".to_string()]);
        // main's headline O129 example.
        assert_eq!(fold("puts [string length abcde]"), vec!["5".to_string()]);
        // SYNC-JUN02d-1 (#525): the cat / repeat / trim folds.
        assert_eq!(
            fold("puts [string cat foo bar]"),
            vec!["foobar".to_string()]
        );
        assert_eq!(
            fold("puts [string repeat ab 3]"),
            vec!["ababab".to_string()]
        );
        assert_eq!(fold("puts [string trim {  hi  }]"), vec!["hi".to_string()]);
        assert_eq!(
            fold("puts [string range abcde 1 3]"),
            vec!["bcd".to_string()]
        );
        assert_eq!(fold("puts [string index abc end]"), vec!["c".to_string()]);
        // SYNC-JUN02d-1 (#525): list + dict folds.
        assert_eq!(fold("puts [llength {a b c}]"), vec!["3".to_string()]);
        // The cmd-sub replacement is one word, so a spaced result is
        // brace-quoted by `render_propagation_word`.
        assert_eq!(fold("puts [concat a b c]"), vec!["{a b c}".to_string()]);
        assert_eq!(fold("puts [join {a b c} -]"), vec!["a-b-c".to_string()]);
        assert_eq!(fold("puts [lindex {a b c} 1]"), vec!["b".to_string()]);
        assert_eq!(fold("puts [dict get {a 1 b 2} b]"), vec!["2".to_string()]);
        assert_eq!(
            fold("puts [lreverse {a b c}]"),
            vec!["{c b a}".to_string()],
            "list result is brace-quoted as one word",
        );
        assert_eq!(fold("puts [string map {a b} aaa]"), vec!["bbb".to_string()]);
        assert_eq!(fold("puts [subst hello]"), vec!["hello".to_string()]);
        // `subst` with a substitution must NOT fold (no upstream resolution).
        assert!(fold("puts [subst {$x}]").is_empty());
        // SYNC-JUN02d B-tail: `string is` Tcl-faithful classes.
        assert_eq!(fold("puts [string is alpha abc]"), vec!["1".to_string()]);
        assert_eq!(fold("puts [string is lower abc1]"), vec!["0".to_string()]);
        assert_eq!(fold("puts [string is boolean yes]"), vec!["1".to_string()]);
        assert_eq!(fold("puts [string is list {a b c}]"), vec!["1".to_string()]);
        // SYNC-JUN02d: `format` %s / %d / %% subset.
        assert_eq!(fold("puts [format %d 42]"), vec!["42".to_string()]);
        assert_eq!(fold("puts [format {v=%s} hi]"), vec!["v=hi".to_string()]);
        // SYNC-JUN03 follow-up: `format` flag / width / precision now folds for
        // the decimal-integer + string conversions (dialect-invariant).
        assert_eq!(fold("puts [format %05d 7]"), vec!["00007".to_string()]);
        assert_eq!(fold("puts [format %.3d 5]"), vec!["005".to_string()]);
        // `%#d` stays unfolded (`0d5` on Tcl 9, `5` on 8.6 — divergent).
        assert!(fold("puts [format %#d 5]").is_empty());
        // SYNC-JUN03 follow-up: `string is integer` / `double` fold over their
        // dialect-invariant subsets.
        assert_eq!(fold("puts [string is integer 42]"), vec!["1".to_string()]);
        assert_eq!(fold("puts [string is double 1.5]"), vec!["1".to_string()]);
        assert_eq!(fold("puts [string is double abc]"), vec!["0".to_string()]);
        // `wideinteger` raises pre-8.6 → stays deferred (never folds).
        assert!(fold("puts [string is wideinteger 42]").is_empty());
        // A braced literal with a space → result rendered as one word.
        assert_eq!(
            fold("puts [string toupper {a b}]"),
            vec!["{A B}".to_string()],
        );
        // A `$var` arg is not a constant literal → no fold.
        assert!(fold("puts [string toupper $x]").is_empty());
        // The range form (`string toupper s first last`) does not fold.
        assert!(fold("puts [string toupper foo 0 0]").is_empty());
        // A non-builtin head (a user proc) is not an O129 candidate.
        assert!(fold("proc ::p {} { return 1 }\nputs [::p]").is_empty());
    }

    #[test]
    fn o129_resolves_constant_var_args_b2() {
        // B2 (SYNC-JUN02d-2, #525): a constant `$var` arg in a builtin
        // cmd-sub is resolved (whole-function-constant → sound) before
        // folding; a multi-word value stays a single argument.
        let fold = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code == "O129")
                .map(|o| o.replacement)
                .collect()
        };
        assert_eq!(
            fold("set s abcde\nputs [string length $s]"),
            vec!["5".to_string()],
        );
        // A multi-word constant value is kept as one arg.
        assert_eq!(
            fold("set s {a b}\nputs [llength $s]"),
            vec!["2".to_string()],
        );
        // Combined B1 + B2: resolved var inside an interpolation cmd-sub.
        assert_eq!(
            fold("set s abc\nputs \"len=[string length $s]\""),
            vec!["\"len=3\"".to_string()],
        );
        // A non-constant var does not resolve → no fold.
        assert!(fold("puts [string length $undefined]").is_empty());
    }

    #[test]
    fn o129_folds_embedded_cmd_subst_in_interpolation() {
        // B1 (SYNC-JUN02d-2, #525): a pure-builtin cmd-sub embedded inside
        // an interpolation string folds, splicing the raw result.
        let fold = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code == "O129")
                .map(|o| o.replacement)
                .collect()
        };
        assert_eq!(
            fold("puts \"v=[string length abc]\""),
            vec!["\"v=3\"".to_string()],
        );
        assert_eq!(
            fold("puts \"n=[llength {a b c}] items\""),
            vec!["\"n=3 items\"".to_string()],
        );
        // A non-foldable embedded sub leaves the string unfolded.
        assert!(fold("puts \"x=[someproc]\"").is_empty());
        // Soundness guard: a result that would re-introduce a `$`
        // substitution into the `\"…\"` is not spliced.
        assert!(fold("puts \"v=[string cat {$} x]\"").is_empty());
    }

    #[test]
    fn o129_trust_gate_suppresses_fold_for_rebound_builtin() {
        // SYNC-JUN02b-4 (#519): the builtin-fold trust gate. When the
        // module renames/redefines `string` anywhere, the whole-module
        // mutation scan distrusts it and O129 must not fold a `[string
        // …]` cmd-sub with the original builtin semantics.
        let fold = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code == "O129")
                .map(|o| o.replacement)
                .collect()
        };
        // Baseline: untouched `string` folds.
        assert_eq!(fold("puts [string toupper foo]"), vec!["FOO".to_string()]);
        // Rebound in a proc body → distrusted everywhere → no fold (the
        // scan over-approximates: any builtin a body may rebind is
        // untrusted, regardless of whether the proc is ever called).
        assert!(
            fold("proc clobber {} { rename string {} }\nputs [string toupper foo]").is_empty(),
            "rebound-in-proc-body builtin must not fold",
        );
        // Top-level rename before the call → also distrusted (the scan is
        // whole-module / flow-insensitive).
        assert!(
            fold("rename string {}\nputs [string toupper foo]").is_empty(),
            "renamed-away builtin must not fold",
        );
    }

    #[test]
    fn simple_var_ref_matches_recognises_forms() {
        assert!(simple_var_ref_matches("$x", "x"));
        assert!(simple_var_ref_matches("${x}", "x"));
        assert!(!simple_var_ref_matches("$y", "x"));
        assert!(!simple_var_ref_matches("plain", "x"));
        // Array subscript → not a simple ref.
        assert!(!simple_var_ref_matches("$x(0)", "x"));
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
