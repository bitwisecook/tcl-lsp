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

//! O103: folding a call to a pure procedure to its constant return, from its summary or by evaluating it with constant arguments.

use super::super::{Optimisation, PassContext};
use super::{is_value_safe_bare_word, render_propagation_word};
use crate::analyses::{ConstValue, LatticeValue};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::interprocedural::ConstantReturn;
use crate::ir::CommandTokens;
use crate::tcl_expr_eval::FoldPolicy;
use tcl_core_types::DiagCode;

/// Evaluate a pure procedure with its parameters bound to the call's
/// constant arguments, returning the constant return value when every
/// reachable `return` agrees on it.
///
/// Seeds the parameters as version-0 lattice constants, re-runs SCCP over
/// the callee's CFG, then reads the one exact value every reachable exit
/// gives ([`crate::interprocedural::exit_value`], the reading the summary's
/// seedless run shares). Used for the argument-sensitive O103 fold
/// (`[::math::add 2 4]` → `6`) that the summary's argument-independent
/// `constant_return` cannot express.
///
/// This re-runs SCCP fresh (not `callee.sccp`), so it must feed the same
/// registry + whole-module trace facts the original build did — otherwise
/// this specific re-run path would be independently trace-blind (the same
/// silent-miscompile class `run_function`/`run_load_forwarding` guard
/// against). The callee and unit must retain the same supplied owner;
/// unavailable callee metadata cannot borrow the caller's grammar or folds.
pub(super) fn evaluate_proc_with_constants(
    (ctx, cu): (&PassContext<'_>, &CompilationUnit),
    callee: &FunctionUnit,
    params: &[tcl_syntax::formal_params::FormalParameter],
    args: &[ConstValue],
    grammar: tcl_dialect::ParameterGrammar,
    policy: FoldPolicy,
) -> Option<tcl_registry::value_transfer::ExactValue> {
    let seed = seed_params_from_args(params, args, grammar, policy)?;
    let registry = ctx.registry?;
    let metadata = callee.invocation_metadata_context_for_module(registry, &cu.ir_module)?;
    let source_metadata_input = metadata.source_analysis_input()?;
    let folds = crate::sccp::BuiltinFoldInputs {
        source_metadata_input: Some(source_metadata_input),
        dialect: Some(source_metadata_input.unit_profile()),
        proven_pure_parameters: true,
        ..ctx.rewrite_folds()
    };
    // A call the callee makes to another procedure of the module is applied
    // as that procedure's transfer summary says, its result too: the
    // re-run's module is the unit's.
    let module = crate::interprocedural::ModuleProcedures::of_unit(cu, registry);
    let no_escaping = std::collections::HashSet::new();
    let result = crate::sccp::sccp_in_module(&crate::sccp::SolveInputs {
        cfg: &callee.cfg,
        ssa: &callee.ssa,
        param_constants: Some(&seed),
        policy,
        extra_escaping: &no_escaping,
        trace: crate::sccp::TraceInputs {
            registry,
            source_metadata_input: Some(source_metadata_input),
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: &cu.ir_module.deferred_writes,
            analysis_context: None,
            existence: None,
        },
        // This re-run feeds an O103 *rewrite*, so it takes the whole-module
        // stance. Without any trust fact — what it used before — it folded
        // `[llength …]` with builtin semantics even where the module shadows
        // `llength`, handing O103 a value the rest of the pipeline disagrees
        // with (#2164).
        folds: Some(folds),
        module: crate::sccp::ModuleRun {
            analysis_body: callee.body_id.map(crate::sccp::AnalysisBody::Procedure),
            procedures: Some(&module),
            owned: None,
            level: crate::sccp::ModuleLevel::Results,
            reads_exits: true,
        },
    });
    crate::interprocedural::exit_value(
        crate::interprocedural::ExitBody::of(callee),
        &result,
        crate::interprocedural::ExitReading {
            policy,
            grammar: callee
                .source_lexer_config()
                .grammar_over(source_metadata_input.unit_profile().grammar),
            folds,
            module: Some((&module, &callee.body_id?)),
        },
    )
}

/// Seed ordinary incoming formal values using the shared native activation
/// plan. Defaults and rest names retain their selected Tcl/Jim semantics;
/// caller links cannot supply ordinary constant local slots.
pub(super) fn seed_params_from_args(
    params: &[tcl_syntax::formal_params::FormalParameter],
    args: &[ConstValue],
    grammar: tcl_dialect::ParameterGrammar,
    policy: FoldPolicy,
) -> Option<std::collections::HashMap<(String, crate::ssa::Version), LatticeValue>> {
    use tcl_syntax::formal_params::FormalArgumentBinding;
    let plan =
        tcl_syntax::formal_params::bind_formal_arguments(params, args.len(), grammar).ok()?;
    let mut seed = std::collections::HashMap::new();
    for formal in plan {
        let (name, value) = match formal {
            FormalArgumentBinding::Value {
                parameter,
                argument,
            } => (
                params.get(parameter)?.name.clone(),
                args.get(argument)?.clone(),
            ),
            FormalArgumentBinding::Default { parameter } => {
                let formal = params.get(parameter)?;
                (
                    formal.name.clone(),
                    crate::sccp::parse_literal_value(formal.default.as_deref()?),
                )
            }
            FormalArgumentBinding::Rest {
                name, start, len, ..
            } => {
                let end = start.checked_add(len)?;
                let tail = args
                    .get(start..end)?
                    .iter()
                    .map(|value| const_value_text(value, policy))
                    .collect::<Option<Vec<_>>>()?;
                (
                    name,
                    // A leading `#` is braced from 8.5 and bare in 8.4.
                    ConstValue::String(
                        tcl_registry::value_transfer::TargetSemantics::of(policy.dialect)
                            .render_list(&tail)?
                            .into(),
                    ),
                )
            }
            FormalArgumentBinding::CallerLink { .. } => return None,
        };
        seed.insert((name, 0), LatticeValue::Const(value));
    }
    Some(seed)
}

/// Render a [`ConstValue`] back to its plain text form — the inverse of
/// [`crate::sccp::parse_literal_value`], used to re-serialise a call
/// argument's already-typed constant into one element of the canonical
/// list text [`seed_params_from_args`] builds for a variadic `args`
/// parameter.
fn const_value_text(cv: &ConstValue, policy: FoldPolicy) -> Option<String> {
    super::super::helpers::literals::format_constant_with_policy(cv, policy)
}

/// Resolve a call's head word to a procedure qname that has an
/// interprocedural summary, following Tcl's real bareword command
/// resolution via [`crate::naming::bareword_resolution_candidates`] —
/// current namespace first, then global, exactly two levels, never every
/// enclosing ancestor namespace (Tcl's own command lookup does not walk
/// intermediate namespaces absent an explicit `namespace path`, which this
/// analysis does not model — a body using it is more conservatively left
/// unresolved here rather than risk folding to the wrong proc).
///
/// Shared with the analyser's identical same-file resolution chase
/// (`Analyser::resolve_indirect_call_target`) so the two can't diverge on
/// the same rule: a relative dotted word (`inner::p`) must not resolve
/// straight to `::inner::p`, rooted at global, because real Tcl (and the
/// analyser side) tries the *current* namespace first
/// (`{namespace}::inner::p`); two procs of that shape in different
/// namespaces could fold a call to the wrong one's constant return.
///
/// Returns the first candidate that has a summary, else `None`.
#[cfg(test)]
pub(super) fn resolve_proc_qname(
    command: &str,
    namespace: &str,
    ia: &crate::interprocedural::InterproceduralAnalysis,
) -> Option<String> {
    if command.is_empty() {
        return None;
    }
    crate::naming::resolve_command_with::<&str, _>(namespace, &[], command, |qname| {
        ia.procedures
            .values()
            .filter(|summary| summary.qualified_name == qname)
            .count()
            == 1
    })
}

/// O103: if `command` resolves to a proc with `can_fold_static_calls`
/// and a `constant_return`, and the call is one [`summary_answers`],
/// emit a rewrite replacing the call with the literal return value.
pub(super) fn try_fold_static_proc_call(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    span: tcl_lexer::Span,
    command: &str,
    tokens: Option<&CommandTokens>,
    _namespace: &str,
) {
    let Some(ia) = cu.interproc.as_ref() else {
        return;
    };
    let Some(registry) = ctx.registry else {
        return;
    };
    let Some(calls) = tokens.and_then(|tokens| {
        crate::registry_invocation::original_source_procedure_calls_for_word_for_module(
            tokens,
            command,
            &cu.ir_module,
            registry,
        )
    }) else {
        return;
    };
    let [call] = calls.as_slice() else {
        return;
    };
    let Some(summary) = ia.procedures.get(&call.id()) else {
        return;
    };
    let qname = &call.procedure().qualified_name;
    // A redefined proc has an ambiguous body — never fold its calls
    // (the flow-sensitive rename gate, mirroring the cmd-subst form and
    // `redefined_procedures` check).
    if cu.ir_module.redefined_procedures.contains(qname) {
        return;
    }
    // Nor a proc whose bare name is later `rename`d over or `interp
    // alias`ed elsewhere in the module — folding this call site to the
    // *originally-declared* proc's constant return would miscompile a
    // script where `command` no longer denotes that proc by the time this
    // call runs (e.g. `proc a {…} …; proc b {…} …; rename b a` moves `b`'s
    // body onto the name `a`; a later `[a …]` call must not fold to `a`'s
    // original body).
    if !ctx.command_mutations.trusts_proc_binding(qname) {
        return;
    }
    if !summary.can_fold_static_calls {
        return;
    }
    let literal_words = call
        .effective_arguments()
        .words
        .iter()
        .all(|word| {
            matches!(
                word,
                tcl_source::words::WordExpr::Literal { .. }
                    | tcl_source::words::WordExpr::BracedLiteral { .. }
            )
        })
        .then(|| call.argument_count())
        .flatten();
    if !summary_answers(&summary.params, literal_words) {
        return;
    }
    let Some(cr) = &summary.constant_return else {
        return;
    };
    let Some(replacement) =
        exact_proc_return_text(ctx, cr).filter(|text| is_value_safe_bare_word(text))
    else {
        return;
    };
    // The span here covers the whole Statement::Call — applying
    // the literal replacement would turn `::answer` into a
    // command named `42`, which is invalid Tcl. Targeting
    // `[procName …]` command substitutions with their token span
    // would avoid this. Until CMD-subst spans are tracked at
    // the call argument level, emit as a hint so
    // editors surface the fold without proposing an applicable
    // quick-fix.
    let mut opt = Optimisation::new(
        DiagCode::O103,
        format!(
            "Fold pure-proc call to '{}' to its constant return",
            summary.qualified_name
        ),
        span,
        replacement.to_owned(),
    );
    opt.hint_only = true;
    ctx.report(opt);
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
/// Fold a constant `[expr {…}]` command substitution in an argument position,
/// returning the rewritten word (`[expr {1 + 2}]` → `3`). A *braced* body folds
/// under the known `constants` (value substitution); a quoted / bare `expr "…"`
/// folds only as a literal (textual substitution is conservative). `None` when
/// the inner command is not `expr` or does not fold to a substitution-free
/// constant. Extracted from [`visit_call_cmd_subst_folds`].
pub(super) fn try_o101_expr_arg_fold(
    ctx: &PassContext<'_>,
    cu: &CompilationUnit,
    tokens: &CommandTokens,
    call: &crate::word_subst::LiftedCall,
    constants: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let registry = ctx.registry?;

    let input = cu.ir_module.source_metadata_input.as_ref()?;

    let context = PassContext {
        source: ctx.source,
        dialect: ctx.dialect,
        registry: Some(registry),
        ir_module: Some(&cu.ir_module),
        ..PassContext::default()
    };
    let actual = context.retained_metadata_context()?;

    let mut parser = tcl_syntax::expr::parser::ExprParseContext::for_profile(input.unit_profile());
    parser.lexer_grammar = input.lexer_config().grammar_over(parser.lexer_grammar);
    let expression = crate::word_subst::lifted_source_expressions_with_metadata_context(
        Some(tokens),
        registry,
        Some(actual.as_ref().into()),
        input.lexer_config(),
        parser,
    )
    .into_iter()
    .find(|expression| expression.span == call.span)?;

    let env = constants
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                crate::tcl_expr_eval::EnvValue::text(value.as_str()),
            )
        })
        .collect();
    let value = context.eval_expression_at(&expression.expression, &env, call.span)?;

    crate::tcl_expr_eval::format_tcl_value_with_policy(&value, context.fold_policy())
        .and_then(|value| render_propagation_word(&value, input.lexer_config()))
}

/// Fold a pure-proc command substitution to its constant return (O103),
/// returning `(qualified_name, replacement_word)`. Uses the interprocedural
/// summary's argument-independent constant return when present and the call
/// is one it answers ([`summary_answers`]), else re-runs the pure callee
/// under the call's constant arguments. `None` when the head
/// is not a foldable internal proc. Extracted from [`visit_call_cmd_subst_folds`].
pub(super) fn try_o103_proc_fold(
    ctx: &PassContext<'_>,
    cu: &CompilationUnit,
    call: &crate::word_subst::LiftedCall,
    calls: &[crate::word_subst::LiftedCall],
) -> Option<(String, String)> {
    let ia = cu.interproc.as_ref()?;
    let tokens = call.tokens.as_ref()?;
    let binding = tokens.source_binding.as_ref()?;
    // A conditional declaration can supply a result when entered under its
    // captured command world. A source edit needs the actual selected target.
    let target = binding.proved_execution_target()?;
    if target.kind != crate::command_binding::BindingKind::Proc || target.registry_backed {
        return None;
    }
    let qname = target.command.clone();
    let (declaration_id, declaration) = cu.ir_module.source_procedure_for_target(target)?;
    if !target.matches_authored_implementation_image(&cu.ir_module.source, declaration.span.start())
    {
        return None;
    }
    let summary = ia.procedures.get(&declaration_id)?;
    // A redefined proc has an ambiguous body — never fold its calls.
    if cu.ir_module.redefined_procedures.contains(&qname) {
        return None;
    }
    let grammar = binding
        .variable_context
        .invocation_dialect?
        .parameter_grammar()?;
    let parameters =
        tcl_syntax::formal_params::parse_formal_parameters_in(&declaration.params_raw, grammar)
            .ok()?;
    let registry = ctx.registry?;
    let module = &cu.ir_module;
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?;
    if crate::registry_invocation::source_procedure_fold_has_execution_trace(
        registry, metadata, tokens, target, module,
    ) {
        return None;
    }
    let original = crate::const_subst::ConstSubstCtx {
        registry,
        resolution_namespace: "::",
        namespace_context: binding.lookup_namespace_key.to_compiled_context(),
        version: None,
        defining_class: None,
        trusts: &|_| false,
        lookup_var: &|_| None,
        script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
    }
    .retained_arguments_with_metadata_context(
        call,
        crate::const_subst::RetainedCalls::in_module(calls, &cu.ir_module),
        metadata,
    )?;
    let plan =
        tcl_syntax::formal_params::bind_formal_arguments(&parameters, original.len(), grammar)
            .ok()?;
    if plan.iter().any(|formal| {
        matches!(
            formal,
            tcl_syntax::formal_params::FormalArgumentBinding::CallerLink { .. }
        )
    }) {
        return None;
    }
    if summary.pure
        && binding.original_invocation_completes_normally(tokens)
        && let Some(result) = binding.original_normal_result(tokens)
    {
        return Some((
            qname,
            render_propagation_word(result.text(), ctx.lexer_config())?,
        ));
    }
    let args: Vec<_> = original
        .iter()
        .map(|value| crate::sccp::parse_literal_value(value))
        .collect();
    let replacement = if summary.can_fold_static_calls
        && let Some(cr) = &summary.constant_return
        && tcl_syntax::formal_params::bind_formal_arguments(&parameters, args.len(), grammar)
            .is_ok()
    {
        render_proc_constant_return(ctx, cr)?
    } else if summary.pure
        && let Some(callee) = cu.procedures.get(&declaration_id)
        && let Some(value) = evaluate_proc_with_constants(
            (ctx, cu),
            callee,
            &parameters,
            &args,
            grammar,
            ctx.fold_policy(),
        )
    {
        // Argument-sensitive: re-run SCCP on the pure callee with the call's
        // constant arguments bound and fold the constant return.
        render_propagation_word(value.as_str().ok()?, ctx.lexer_config())?
    } else {
        return None;
    };
    Some((summary.qualified_name.clone(), replacement))
}

fn render_proc_constant_return(ctx: &PassContext<'_>, constant: &ConstantReturn) -> Option<String> {
    render_propagation_word(exact_proc_return_text(ctx, constant)?, ctx.lexer_config())
}

fn exact_proc_return_text<'a>(
    ctx: &PassContext<'_>,
    constant: &'a ConstantReturn,
) -> Option<&'a str> {
    let exact = constant.exact();
    let text = exact.bytes.as_str();
    if let Some(tcl_registry::value_transfer::NumericValue::Float(value)) = exact.numeric
        && crate::tcl_expr_eval::format_tcl_value_with_policy(
            &crate::tcl_expr_eval::TclValue::Float(value),
            ctx.fold_policy(),
        )
        .as_deref()
            != Some(text)
    {
        return None;
    }
    Some(text)
}

/// Whether a procedure's argument-independent constant return answers a
/// call whose words after the head are `literal_words` many, all literal
/// (`None` when one substitutes): only a call the argument-sensitive re-run
/// could make — every word literal, as many as the parameters accept. A
/// word that substitutes runs before the call and may raise or write, and a
/// count the parameters do not accept raises; the constant says neither
/// (#2389).
fn summary_answers(params: &[String], literal_words: Option<usize>) -> bool {
    literal_words
        .and_then(|count| u16::try_from(count).ok())
        .is_some_and(|count| crate::interprocedural::arity_from_names(params).accepts(count))
}

/// Parse the head word out of a CMD-subst interior. Returns
/// `None` when the head word is empty or contains metacharacters
/// that would change the parsed command name under substitution.
#[cfg(test)]
pub(super) fn parse_cmd_subst_head(inner: &str) -> Option<&str> {
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
