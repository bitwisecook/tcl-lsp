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

//! Folding `[cmd …]` substitutions in call words and inside interpolated strings: the retained calls, builtin folds and O115.

use super::super::helpers::spans::original_quoted_word_rewrite_span;
use super::super::{Optimisation, PassContext};
use super::proc_folds::{try_o101_expr_arg_fold, try_o103_proc_fold};
use super::render_propagation_word;
use crate::compilation_unit::CompilationUnit;
use crate::ir::CommandTokens;
use tcl_core_types::DiagCode;
use tcl_registry::CommandRegistry;

pub(crate) fn retained_substitution_calls(
    tokens: &CommandTokens,
    module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> Option<Vec<crate::word_subst::LiftedCall>> {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    if module.source_metadata_input.is_none()
        && module.source_entry.metadata_context.is_standalone()
    {
        return crate::word_subst::checked_lifted_calls(
            tokens,
            tokens.native_lexer_config(module.lexer_config.nested()),
        );
    }
    if !module
        .retained_source_bindings
        .as_deref()?
        .matches_module(module, registry)
    {
        return None;
    }
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?;
    crate::word_subst::checked_original_lifted_calls_with_metadata_context(
        tokens,
        module.lexer_config.nested(),
        registry,
        metadata,
    )
}

pub(crate) fn retained_call_at(
    calls: &[crate::word_subst::LiftedCall],
    start: u32,
) -> Option<&crate::word_subst::LiftedCall> {
    let mut matching = calls.iter().filter(|call| call.span.start() == start);
    let call = matching.next()?;
    matching.next().is_none().then_some(call)
}

/// A result whose literal spelling a script evaluation would read apart from
/// the result itself ([`crate::const_subst::folded_literal_reads_apart`])
/// abstains: `[list string range $s 0 end-2147483649]` evaluates directly,
/// its braced literal compiles. `dialect` names the release that reads it.
/// The declared script encoding of the unit's retained source input.
pub(super) fn module_script_encoding(
    cu: &CompilationUnit,
) -> tcl_registry::value_transfer::ScriptEncoding {
    cu.ir_module
        .source_metadata_input
        .as_ref()
        .map_or_default(crate::analyser::ResolvedAnalysisInput::script_encoding)
}

pub(super) fn fold_retained_builtin(
    module: &crate::ir::Module,
    registry: &CommandRegistry,
    call: &crate::word_subst::LiftedCall,
    calls: &[crate::word_subst::LiftedCall],
    defining_class: Option<&str>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    (dialect, constants, script_encoding): (
        Option<&'static tcl_dialect::DialectProfile>,
        Option<&std::collections::HashMap<String, String>>,
        tcl_registry::value_transfer::ScriptEncoding,
    ),
) -> Option<String> {
    let metadata = metadata?;
    let release = dialect.map_or(tcl_dialect::TclVersion::V9_0, |profile| {
        profile.vm_runtime_version
    });
    let lookup = |name: &str| constants.and_then(|constants| constants.get(name).cloned());
    crate::const_subst::ConstSubstCtx {
        registry,
        resolution_namespace: "::",
        namespace_context: call
            .tokens
            .as_ref()
            .and_then(crate::registry_invocation::compiled_namespace_context),
        version: None,
        defining_class,
        trusts: &|_| false,
        lookup_var: &lookup,
        script_encoding,
    }
    .fold_retained_call_with_metadata_context(
        call,
        crate::const_subst::RetainedCalls::in_module(calls, module),
        metadata,
    )
    .filter(|value| !crate::const_subst::folded_literal_reads_apart(value, release))
}

fn retained_expr_invocation(
    module: &crate::ir::Module,
    call: &crate::word_subst::LiftedCall,
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<crate::registry_invocation::ResolvedStatementInvocation> {
    let metadata = metadata?;
    let tokens = call.tokens.as_ref()?;
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry,
        Some(metadata),
        tokens,
    )?;
    (invocation.facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        )
        && !module.has_dynamic_trace
        && !crate::registry_invocation::source_fold_has_execution_trace(
            registry,
            metadata,
            tokens,
            invocation.facts.descriptor_key(),
            &module.traced_commands,
        ))
    .then_some(invocation)
}

pub(super) fn retained_o115(
    ctx: &PassContext<'_>,
    module: &crate::ir::Module,
    _tokens: &CommandTokens,
    call: &crate::word_subst::LiftedCall,
    word: &str,
    registry: &CommandRegistry,
) -> Option<String> {
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, module);
    let invocation = retained_expr_invocation(module, call, registry, metadata)?;
    if !ctx
        .command_mutations
        .trusts(invocation.facts.descriptor_key().label())
    {
        return None;
    }
    let tokens = call.tokens.as_ref()?;
    let (_, command) = tokens
        .source_binding
        .as_ref()?
        .nested_expression_normalisation_source(
            registry,
            tokens,
            &ctx.fold_policy().preparation_context()?,
            None,
        )?;
    // The call was selected from the exact original whole substitution. The
    // inner command source comes from its retained outer expression owner.
    (ctx.source.get(call.span.as_range()) == Some(word)).then_some(command)
}

pub(super) fn visit_call_cmd_subst_folds(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    tokens: &CommandTokens,
    constants: &std::collections::HashMap<String, String>,
    _namespace: &str,
    statement_span: tcl_lexer::Span,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &cu.ir_module);
    let Some(calls) = retained_substitution_calls(tokens, &cu.ir_module, registry) else {
        return;
    };

    for word in tokens.words() {
        let tcl_source::words::WordExpr::CommandSubstitution { spelling, source } = word else {
            continue;
        };
        let Some(call) = retained_call_at(&calls, source.span.start()) else {
            continue;
        };
        if let Some(collapsed) = retained_o115(ctx, &cu.ir_module, tokens, call, spelling, registry)
        {
            let wrapper =
                crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                    registry, metadata, tokens,
                )
                .filter(|invocation| {
                    invocation.facts.operation
                        == tcl_registry::SemanticOperationId::StructuredLowering(
                            tcl_registry::hooks::LoweringHookId::Return,
                        )
                        && tokens.words().len() == 2
                })
                .and_then(|_| {
                    let original = ctx.source.get(statement_span.as_range())?;
                    let start = call.span.start().checked_sub(statement_span.start())? as usize;
                    let end = call.span.end().checked_sub(statement_span.start())? as usize;
                    let mut replacement = original.to_owned();
                    replacement.replace_range(start..end, &collapsed);
                    Some((statement_span, replacement))
                });
            let (span, replacement) = wrapper.unwrap_or((call.span, collapsed));
            ctx.report(Optimisation::new(
                DiagCode::O115,
                "Remove redundant nested expr",
                span,
                replacement,
            ));
            continue;
        }
        let is_expr = retained_expr_invocation(&cu.ir_module, call, registry, metadata).is_some();

        if is_expr && let Some(folded) = try_o101_expr_arg_fold(ctx, cu, tokens, call, constants) {
            ctx.report(Optimisation::new(
                DiagCode::O101,
                "Fold constant expression",
                call.span,
                folded,
            ));
            continue;
        }
        if let Some(value) = fold_retained_builtin(
            &cu.ir_module,
            registry,
            call,
            &calls,
            None,
            metadata,
            (ctx.dialect, Some(constants), module_script_encoding(cu)),
        ) {
            if let Some(rewrite) = builtin_fold_rewrite(ctx, call, &value, metadata) {
                ctx.report(rewrite);
            }
            continue;
        }
        if let Some((name, replacement)) = try_o103_proc_fold(ctx, cu, call, &calls) {
            ctx.report(Optimisation::new(
                DiagCode::O103,
                format!("Fold pure-proc call to '{name}' to its constant return"),
                call.span,
                replacement,
            ));
        }
    }
}

fn builtin_fold_rewrite(
    ctx: &PassContext<'_>,
    call: &crate::word_subst::LiftedCall,
    value: &str,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<Optimisation> {
    let registry = ctx.registry?;
    let code = call
        .tokens
        .as_ref()
        .and_then(|tokens| {
            crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                registry, metadata, tokens,
            )
        })
        .map_or(DiagCode::O129, |invocation| {
            // value-transfer-ok: irreducible — names the reported
            // rewrite family of a fold already made; no value depends on it.
            match invocation.facts.canonical_command.as_str() {
                "list" => DiagCode::O116,
                "lindex" => DiagCode::O118,
                _ => DiagCode::O129,
            }
        });
    let config = ctx.lexer_config();
    let replacement = if code == DiagCode::O116
        && !value.is_ascii()
        && ctx
            .source
            .get(call.span.as_range())
            .is_some_and(|text| text.contains("\\x"))
    {
        tcl_syntax::word_rules::lexical_ascii_escaped_source_word(value, config)
    } else {
        render_propagation_word(value, config)
    };
    Some(Optimisation::new(
        code,
        "Fold constant builtin command substitution",
        call.span,
        replacement?,
    ))
}

/// True when `inside` is exactly one bracket-balanced `[…]` covering the
/// whole word (a free-standing command substitution), as opposed to a
/// sub embedded in surrounding interpolation text.
pub(super) fn is_whole_word_cmd_subst(inside: &str) -> bool {
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

fn outermost_calls_in(
    calls: &[crate::word_subst::LiftedCall],
    span: tcl_lexer::Span,
) -> Vec<&crate::word_subst::LiftedCall> {
    let contained: Vec<_> = calls
        .iter()
        .filter(|call| call.span.start() >= span.start() && call.span.end() <= span.end())
        .collect();
    let mut roots: Vec<_> = contained
        .iter()
        .copied()
        .filter(|call| {
            !contained.iter().any(|outer| {
                outer.span.start() < call.span.start() && outer.span.end() >= call.span.end()
            })
        })
        .collect();
    roots.sort_by_key(|call| call.span.start());
    roots
}

/// Fold a pure-builtin command substitution
/// embedded *inside* an interpolation string, splicing the raw result
/// into the surrounding `"…"` (O129): `puts "v=[string length abc]"` →
/// `puts "v=5"`.
///
/// Each original substitution with a proved executable registry target and
/// closed original arguments is folded through the shared retained-call owner.  Soundness guards: the result must not
/// reintroduce a substitution into the `"…"` context — a result
/// carrying `$`, `[`, `]`, `\`, or `"` is left unfolded.  `$var`
/// substitutions and any non-foldable `[cmd]` are kept verbatim; at
/// least one successful fold is required to emit.
pub(super) fn visit_string_interpolation_cmd_subs(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    tokens: &CommandTokens,
    span: tcl_lexer::Span,
    text: &str,
    constants: &std::collections::HashMap<String, String>,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &cu.ir_module);
    let Some(calls) = retained_substitution_calls(tokens, &cu.ir_module, registry) else {
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
    // `visit_call_cmd_subst_folds`; this path only handles a sub *embedded* in
    // surrounding interpolation text (else we'd emit a duplicate O129).
    if is_whole_word_cmd_subst(inside) {
        return;
    }
    let Some(rewrite_span) = original_quoted_word_rewrite_span(ctx, span, inside) else {
        return;
    };
    let content_base = if ctx.source.as_bytes().get(rewrite_span.start() as usize) == Some(&b'"') {
        rewrite_span.start().checked_add(1)
    } else {
        Some(rewrite_span.start())
    };
    let Some(content_base) = content_base else {
        return;
    };
    let Some(content_end) = u32::try_from(inside.len())
        .ok()
        .and_then(|length| content_base.checked_add(length))
    else {
        return;
    };
    if ctx.source.get(content_base as usize..content_end as usize) != Some(inside) {
        return;
    }
    let roots = outermost_calls_in(&calls, tcl_lexer::Span::new(content_base, content_end));
    let mut out = String::with_capacity(inside.len());
    let mut last = 0;
    let mut folded_any = false;
    let lattice = (ctx.dialect, Some(constants), module_script_encoding(cu));
    for call in roots {
        let Some(result) = fold_retained_builtin(
            &cu.ir_module,
            registry,
            call,
            &calls,
            None,
            metadata,
            lattice,
        ) else {
            continue;
        };
        if result
            .bytes()
            .any(|byte| matches!(byte, b'$' | b'[' | b']' | b'\\' | b'"'))
        {
            continue;
        }
        let start = (call.span.start() - content_base) as usize;
        let end = (call.span.end() - content_base) as usize;
        let Some(prefix) = inside.get(last..start) else {
            return;
        };
        let mut next = out.clone();
        next.push_str(prefix);
        if !interpolation_splice_preserves_escapes(
            &next,
            &result,
            &inside[end..],
            ctx.lexer_config(),
            ctx.dialect,
        ) {
            continue;
        }
        next.push_str(&result);
        out = next;
        last = end;
        folded_any = true;
    }
    if !folded_any {
        return;
    }
    out.push_str(&inside[last..]);
    ctx.report(Optimisation::new(
        DiagCode::O129,
        "Fold constant builtin command substitution in interpolation",
        rewrite_span,
        format!("\"{out}\""),
    ));
}

/// Substitution boundaries terminate variable-width escapes. No preceding
/// escape may consume bytes across a boundary removed by this splice.
fn interpolation_splice_preserves_escapes(
    prefix: &str,
    result: &str,
    suffix: &str,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> bool {
    let selected_escapes = [config.escapes];
    let escapes = if dialect
        .and_then(tcl_dialect::DialectProfile::runtime_version)
        .is_some()
    {
        selected_escapes.as_slice()
    } else {
        tcl_dialect::EscapeSyntax::ALL
    };
    let joined = format!("{prefix}{result}{suffix}");
    escapes.iter().all(|escapes| {
        prefix.match_indices('\\').all(|(start, _)| {
            tcl_syntax::backslash::escape_end_in(&joined, start, *escapes) <= prefix.len()
        })
    })
}
