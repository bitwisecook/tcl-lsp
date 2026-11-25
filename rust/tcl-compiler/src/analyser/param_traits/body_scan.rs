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

//! The walk over a procedure body: the shallow command scan, the bounded descent into substitutions and the deep scan through bodies and lambda literals.

use super::super::types::ProcArgTrait;
use super::caller_frame::record_direct_caller_effects;
use super::command_traits::scan_command;
use super::{
    Aliases, ArgumentValues, MAX_DEPTH, ScanCtx, TraitScanEnv, infer_param_traits_deep_at_depth,
};
use crate::cfg_builder::frame_effects::DirectCallerEffects;
use crate::segmenter::segment_commands_with_offset_and_config;
use std::collections::{HashMap, HashSet};
use tcl_lexer::{LexerConfig, TokenType};
use tcl_registry::arg_role::ArgRole;

/// Single-level command scan shared by both passes.  Extracts
/// segmented commands from `source` and dispatches each through
/// [`scan_command`].
///
/// `source` has a lifetime independent of `'p` so callers can
/// re-enter with body-arg slices that don't outlive the
/// enclosing source.
pub(super) fn scan_commands<'p>(
    source: &str,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
    direct: Option<&mut DirectCallerEffects>,
) {
    scan_commands_bounded(source, ctx, traits, aliases, 0, direct);
}

/// [`scan_commands`] plus descent into `[...]` command substitutions, bounded
/// by [`MAX_DEPTH`].  A param used inside a substitution (`return [set $v]`,
/// `set x [foo $v]`, `set ${v}($k)` wrapped in `[...]`) is a first-order use of
/// that param, not a "deep" one, so both the shallow and deep passes see it —
/// the substitution is where much ordinary Tcl computation happens.  Nested
/// substitutions are reached by the recursion.
fn scan_commands_bounded<'p>(
    source: &str,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
    depth: u8,
    mut direct: Option<&mut DirectCallerEffects>,
) {
    let segments = segment_commands_with_offset_and_config(source, 0, ctx.config.nested());
    for seg in &segments {
        if seg.texts.is_empty() {
            continue;
        }
        let view = if ctx.parameters.is_empty() {
            ctx.body_assistance(source, seg)
        } else {
            scan_segment(source, seg, ctx, traits, aliases, depth).0
        };
        if let Some((direct, view)) = direct.as_deref_mut().zip(view.as_deref()) {
            record_direct_caller_effects(direct, view, source, seg, ctx);
        }
    }
}

fn scan_segment<'p>(
    source: &str,
    seg: &crate::segmenter::SegmentedCommand,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
    depth: u8,
) -> (
    Option<Box<crate::registry_invocation::InvocationBodyAssistance>>,
    Aliases<'p>,
    ArgumentValues<'p>,
) {
    let cmd_args: Vec<String> = seg.texts[1..].to_vec();
    // Per-arg "braced literal" flags: a single `Str` word (`{…}`) suppresses
    // substitution, so a `$k` inside it is literal text, not a param ref.
    let braced: Vec<bool> = (1..seg.texts.len())
        .map(|i| {
            seg.argv.get(i).is_some_and(|t| t.kind == TokenType::Str)
                && seg.single_token_word.get(i).copied().unwrap_or(false)
        })
        .collect();
    let values = ctx.argument_values(source, seg, aliases);
    if depth < MAX_DEPTH {
        let config = ctx.config.nested();
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            seg,
        );
        if let Some(words) = tokens
            .original_words_match_argv_text()
            .then(|| {
                crate::registry_invocation::original_native_compiler_words(
                    &tcl_lexer::SourceImage::document(source),
                    tokens.words(),
                    seg.span.start(),
                    config,
                )
            })
            .flatten()
        {
            for word in &words {
                let arena = word.executable_parts();
                let mut bodies: Vec<_> = arena
                    .all_parts()
                    .filter_map(|part| match part.part {
                        tcl_lexer::ExecutablePart::Command { body } => Some(body),
                        _ => None,
                    })
                    .collect();
                bodies.sort_by_key(|body| body.start());
                for body in bodies {
                    let Some(inner) = arena
                        .bytes(body)
                        .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    else {
                        continue;
                    };
                    if !inner.trim().is_empty() {
                        let nested = ctx.nested_at(Some(body.start()), inner);
                        scan_commands_bounded(inner, &nested, traits, aliases, depth + 1, None);
                    }
                }
            }
        }
    }
    let body_aliases = aliases.clone();
    let view = ctx.body_assistance(source, seg);
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_PARAM_ROLES").is_some() {
        eprintln!(
            "ORIGINAL_PARAM_ROLES command local_offset={} base={:?} executed_source={} parameters={} full_values={} components={} assistance={} roles={}",
            seg.span.start(),
            ctx.source_base,
            ctx.executed_source.is_some(),
            ctx.parameters.len(),
            values.full.iter().filter(|value| value.is_some()).count(),
            values.components.iter().map(Vec::len).sum::<usize>(),
            view.is_some(),
            view.as_ref()
                .map_or(0, |view| view.parameter_role_advice().written_roles().len())
        );
    }
    scan_command(
        view.as_deref(),
        &cmd_args,
        &braced,
        &values,
        ctx,
        traits,
        aliases,
    );
    (view, body_aliases, values)
}

/// Recursive scan with depth tracking.  Walks every command at
/// the current level, then descends into the braced body
/// arguments each command declares via the registry's
/// ``ArgRole::Body`` role assignments.
///
/// `$var` / `[cmd]` body args are skipped — they aren't
/// braced bodies, and any `Eval` trait they carry is recorded
/// at the top level by the same call-site's role scan.
//
pub(super) fn scan_deep<'p>(
    source: &str,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
    depth: u8,
) {
    if depth > MAX_DEPTH {
        return;
    }

    // Each nested body receives its original pre-dispatch copy state. A
    // conditional body cannot publish its assignments into a sibling path.
    let segments = segment_commands_with_offset_and_config(source, 0, ctx.config.nested());
    for seg in segments {
        if seg.texts.is_empty() {
            continue;
        }
        let (view, body_aliases, values) = scan_segment(source, &seg, ctx, traits, aliases, depth);
        let cmd_args: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();
        // One surface answers for both a catalogue command and a
        // document-declared `# tcl-lsp: stub` one, so a stub-defined body
        // arg recurses just like a shipped one.
        let roles = view.as_ref().map_or_else(Vec::new, |view| {
            if ctx.original_logical_lambda.is_some() {
                return view.conditional_logical_lambda_roles().unwrap_or_default();
            }
            let mut roles = view.parameter_role_advice().written_roles();
            if let Some(shape) = view.conditional_parameter_source_shape(ctx.parameters) {
                roles.extend(shape.written_argument_roles());
            }
            roles.extend(view.declared_roles.iter().copied());
            roles
        });
        let body_indices: HashSet<usize> = roles
            .iter()
            .filter_map(|&(index, role)| (role == ArgRole::Body).then_some(index))
            .collect();
        for idx in body_indices {
            let original_body = view
                .as_ref()
                .and_then(|view| view.logical_trait_caller_body(idx));
            if (ctx.original_logical_body.is_some() || ctx.original_logical_lambda.is_some())
                && original_body.is_none()
            {
                continue;
            }
            let Some(body_text) = cmd_args.get(idx) else {
                continue;
            };
            if let Some(token) = seg.argv.get(idx + 1) {
                scan_deep_body(
                    body_text,
                    token,
                    original_body.as_ref(),
                    ctx,
                    traits,
                    &body_aliases,
                    depth,
                );
            }
        }

        // `apply {argList body ?ns?} …` — an apply body runs in a *fresh*
        // call frame with its own parameters; recursing into it with the
        // enclosing proc's own `ctx`/`traits` (as a plain `Body` arg would)
        // conflates the two frames. A lambda-local variable that happens to
        // share a name with an enclosing param (`proc f {body} { apply {x
        // {eval $body}} 1 }`) would wrongly mark `f`'s `body` param as
        // evaluated, while the real forwarding case (`apply {x {eval $x}}
        // $body`) would be missed entirely. Instead: infer the lambda's own
        // traits in complete isolation — as if it were its own tiny proc —
        // then propagate a
        // lambda param's trait back onto an enclosing param only when the
        // corresponding actual argument is a bare, unadorned reference to
        // that enclosing param, i.e. only when the value genuinely flows
        // from the caller's frame into the lambda's.
        for idx in roles
            .iter()
            .filter_map(|&(index, role)| (role == ArgRole::LambdaLiteral).then_some(index))
        {
            scan_deep_lambda(
                source,
                &seg,
                view.as_deref(),
                ctx,
                traits,
                (idx, depth),
                &values,
            );
        }
    }
}

fn deep_lambda_parts(
    source: &str,
    seg: &crate::segmenter::SegmentedCommand,
    ctx: &ScanCtx<'_, '_>,
    idx: usize,
    selected_lambda: Option<crate::registry_invocation::OriginalSourceScriptBody>,
) -> Option<(Vec<crate::signature_scan::types::ParamDef>, String, u32)> {
    let parts = if let Some(body) = selected_lambda {
        let (params_text, body_text) =
            selected_logical_lambda_texts(source, ctx.source_base, &body, ctx.config)?;
        let parameter_defs = crate::signature_scan::params::parse_param_list(
            params_text,
            tcl_syntax::word_rules::WordValueRules::from_config(&ctx.config),
        );
        let body_offset = ctx
            .source_base
            .and_then(|base| body.content_span().start().checked_sub(base))?;
        (parameter_defs, body_text.to_owned(), body_offset)
    } else if let Some((document, analysis, _)) = ctx.document {
        let (params, body, offset) = selected_lambda_source(
            document,
            analysis,
            seg.argv
                .first()
                .and_then(|head| head.span.start().checked_add(ctx.source_base?)),
            idx,
            ctx.config,
        )?;
        (params, body, offset)
    } else {
        let &tok = seg.argv.get(idx + 1)?;
        if tok.kind != tcl_lexer::TokenType::Str {
            return None;
        }
        let elems = crate::lambda_literal::split_lambda_literal(source, tok)?;
        let body_span = elems.body?;
        let body_text = source.get(body_span.start() as usize..body_span.end() as usize)?;
        let params_text = source.get(elems.params.start() as usize..elems.params.end() as usize)?;
        (
            crate::signature_scan::params::parse_param_list(
                params_text,
                tcl_syntax::word_rules::WordValueRules::from_config(&ctx.config),
            ),
            body_text.to_owned(),
            ctx.source_base
                .unwrap_or(0)
                .saturating_add(body_span.start()),
        )
    };
    Some(parts)
}

fn scan_deep_lambda<'p>(
    source: &str,
    seg: &crate::segmenter::SegmentedCommand,
    view: Option<&crate::registry_invocation::InvocationBodyAssistance>,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    (idx, depth): (usize, u8),
    values: &ArgumentValues<'p>,
) {
    if let Some((document, _, base)) = ctx.document {
        let Some(current_base) = ctx.source_base.or(base) else {
            return;
        };
        let Ok(start) = usize::try_from(current_base) else {
            return;
        };
        let Some(end) = start.checked_add(source.len()) else {
            return;
        };
        let Some(current_image) = document.get(start..end) else {
            return;
        };
        if current_image.as_bytes() != source.as_bytes() {
            return;
        }
    }
    let selected_lambda = view.and_then(|view| view.logical_trait_caller_lambda(idx));
    let selected_lambda_owner = view.and_then(|view| view.logical_trait_caller_lambda_owner(idx));
    if selected_lambda_owner.is_some() && ctx.executed_source.is_some() {
        // A retained Logical lambda is source-role advice, not an
        // entered Native body or a replacement for that owner.
        return;
    }
    if (ctx.original_logical_body.is_some() || ctx.original_logical_lambda.is_some())
        && (selected_lambda.is_none() || selected_lambda_owner.is_none())
    {
        return;
    }
    let Some((lambda_param_defs, body_text, body_offset)) =
        deep_lambda_parts(source, seg, ctx, idx, selected_lambda)
    else {
        return;
    };
    if body_text.trim().is_empty() {
        return;
    }
    let lambda_param_names: Vec<&str> = lambda_param_defs.iter().map(|p| p.name.as_str()).collect();
    if lambda_param_names.is_empty() {
        return;
    }
    let lambda_source = if ctx.executed_source.is_some() {
        let Some(source_base) = ctx.source_base else {
            return;
        };
        let Some(relative_body_offset) = body_offset.checked_sub(source_base) else {
            return;
        };
        ctx.source_slice(&body_text, relative_body_offset)
    } else {
        None
    };
    if ctx.executed_source.is_some() && lambda_source.is_none() {
        return;
    }
    let lambda_traits = infer_param_traits_deep_at_depth(
        &lambda_param_names,
        &body_text,
        TraitScanEnv {
            surface: ctx.surface,
            config: ctx.config,
            identities: ctx.identities,
            executed_source: lambda_source.as_ref(),
            original_logical_body: None,
        },
        depth + 1,
        ctx.document
            .map(|(document, analysis, _)| (document, analysis, Some(body_offset))),
        selected_lambda_owner.as_ref(),
    );
    if lambda_traits.is_empty() {
        return;
    }
    for (param_name, outer_param) in lambda_input_bindings(view, idx, &lambda_param_defs, values) {
        if let Some(lambda_param_traits) = lambda_traits.get(&param_name) {
            traits
                .entry(outer_param)
                .or_default()
                .extend(lambda_param_traits.iter().copied());
        }
    }
}

/// Recover a lambda's formal list and body only through the complete document's
/// retained source invocation. This joins the original selected `LambdaLiteral`
/// ordinal, whole word, source-list children, lexer configuration and body
/// geometry; cooked token text is not used as an alternate source identity.
pub(super) fn selected_lambda_source(
    document: &str,
    analysis: &crate::analyser::AnalysisResult,
    command_offset: Option<u32>,
    argument: usize,
    config: LexerConfig,
) -> Option<(Vec<crate::signature_scan::types::ParamDef>, String, u32)> {
    let command_offset = command_offset?;
    if analysis.body_lexer_config != Some(config) {
        return None;
    }
    let image = tcl_lexer::SourceImage::document(document);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let input = analysis.resolved_input.as_ref()?;
    let context = input.context_registry();
    let words = crate::registry_invocation::source_structure::source_registry_words_at(
        document,
        analysis,
        command_offset,
    )?;
    let selected = words.with_source_schema(
        &context,
        crate::registry_invocation::source_schema_queries::lambda_call,
    )??;
    if selected.lambda_argument != argument {
        return None;
    }
    let lambda = words.original_argument_word(argument)?;
    let fields = tcl_syntax::word_rules::original_static_word_list_elements(lambda)?;
    if !(2..=3).contains(&fields.len()) {
        return None;
    }
    let parameters = crate::signature_scan::params::parse_param_list(
        fields.first()?.value(),
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
    );
    let body = words.original_lambda_body_geometry(&context)?;
    if body.original_container() != lambda || !body.matches_source(&image, config) {
        return None;
    }
    let span = body.content_span();
    let body_text = document.get(span.as_range())?.to_owned();
    (body_text.as_bytes()
        == body
            .original_container()
            .image()
            .bytes()
            .get(span.as_range())?)
    .then_some((parameters, body_text, span.start()))
}

/// Only an ordinary value binding forwards the original outer input. Defaults,
/// variadic list construction and Jim caller links have different value owners.
fn lambda_input_bindings<'p>(
    view: Option<&crate::registry_invocation::InvocationBodyAssistance>,
    lambda: usize,
    parameters: &[crate::signature_scan::types::ParamDef],
    values: &ArgumentValues<'p>,
) -> Vec<(String, &'p str)> {
    use tcl_syntax::formal_params::{
        FormalArgumentBinding, FormalParameter, bind_formal_arguments,
    };
    let Some(selected) = view.and_then(|view| view.caller_name_template_invocation()) else {
        return Vec::new();
    };
    let Some(grammar) = selected
        .dialect
        .and_then(tcl_registry::InvocationDialect::parameter_grammar)
    else {
        return Vec::new();
    };
    let Some(lambda) = (0..selected.arguments.len())
        .find(|index| selected.effective.written_argument(*index) == Some(lambda))
    else {
        return Vec::new();
    };
    let formals: Vec<_> = parameters
        .iter()
        .map(|parameter| FormalParameter {
            name: parameter.name.clone(),
            default: parameter.default_value.clone(),
        })
        .collect();
    let Ok(plan) = bind_formal_arguments(
        &formals,
        selected.arguments.len().saturating_sub(lambda + 1),
        grammar,
    ) else {
        return Vec::new();
    };
    plan.into_iter()
        .filter_map(|binding| {
            let FormalArgumentBinding::Value {
                parameter,
                argument,
            } = binding
            else {
                return None;
            };
            let original = selected.effective.written_argument(lambda + 1 + argument)?;
            Some((formals[parameter].name.clone(), values.argument(original)?))
        })
        .collect()
}

fn scan_deep_body<'p>(
    body_text: &str,
    token: &tcl_lexer::Token,
    original_body: Option<&crate::registry_invocation::OriginalSourceScriptBody>,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &Aliases<'p>,
    depth: u8,
) {
    if body_text.trim().is_empty() {
        return;
    }
    let mut nested = ctx.nested(token, body_text);
    if let Some(body) = original_body {
        // The actual selected same-frame script owns these bytes. A leading
        // substitution is Tcl code inside this source body, not a computed
        // parent operand. No entered script or runtime value is supplied.
        if nested.source_base != Some(body.content_span().start())
            || !body.matches_original_content(body_text, body.content_span().start(), ctx.config)
            || u32::try_from(body_text.len()).ok()
                != Some(body.content_span().end() - body.content_span().start())
        {
            return;
        }
        if ctx.original_logical_lambda.is_some() {
            nested.selected_source_child_body = Some(body.clone());
        }
    } else {
        let head = body_text.as_bytes();
        if head
            .first()
            .is_some_and(|&byte| byte == b'$' || byte == b'[')
            || head
                .get(1)
                .is_some_and(|&byte| byte == b'$' || byte == b'[')
        {
            return;
        }
    }
    scan_deep(body_text, &nested, traits, &mut aliases.clone(), depth + 1);
}

fn selected_logical_lambda_texts<'a>(
    source: &'a str,
    source_base: Option<u32>,
    body: &crate::registry_invocation::OriginalSourceScriptBody,
    config: LexerConfig,
) -> Option<(&'a str, &'a str)> {
    let container = body.original_container();
    let fields = crate::lambda_literal::split_original_lambda_literal(container)?;
    if container.config() != config || fields.braced_body() != Some(body.content_span()) {
        return None;
    }
    let image = container.image().try_text().ok()?;
    let local_slice = |span: tcl_lexer::Span| {
        let base = source_base?;
        let start = usize::try_from(span.start().checked_sub(base)?).ok()?;
        let end = usize::try_from(span.end().checked_sub(base)?).ok()?;
        let text = source.get(start..end)?;
        (image.get(span.as_range())? == text).then_some(text)
    };
    Some((
        local_slice(fields.params)?,
        local_slice(body.content_span())?,
    ))
}
