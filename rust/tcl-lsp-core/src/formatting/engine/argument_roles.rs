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

//! Which arguments of a parsed command are bodies, expressions, keywords or parameter lists.

use super::super::config::FormatterConfig;
use super::{ArgKind, FormattingSourceLayout, ParsedCommand};
use tcl_lexer::SourceMap;
use tcl_registry::{ArgRole, CommandRegistry, Traits};

// Body / expr / param-list argument identification

/// Mark body / keyword / param-list arguments in place, entirely from
/// registry data.
///
/// Every classification here comes from the command's spec, so no command
/// name appears:
///
/// * **Bodies** are the [`ArgRole::Body`] positions the spec (or its dynamic
///   resolver) reports — for `if` and `try` that is the C-Tcl-shaped clause
///   walk, which knows where a body may legally sit and is the same walk that
///   drives the E004 shape diagnostic.
/// * **Block vs inline** comes from [`tcl_registry::ArgPresentation`]: a body
///   argument is expanded onto its own lines unless the spec declares it
///   `InlineScript`, which is how `for`'s `start` and
///   `next` scripts stay on the header line.
/// * **Structural keywords** are the [`ArgRole::Keyword`] positions the same
///   resolvers report. That is *structural*, not textual: a data word that
///   merely spells `else` is not a keyword unless the grammar puts a keyword
///   there, and `if {1} {a} else then` correctly treats the trailing `then`
///   as the else-branch body, not a keyword.
/// * **Parameter lists** and **lambda literals** are their own roles.
///
/// Because the lookup goes through the registry, the explicitly-global
/// spellings (`::if`, `::for`, `::try`) resolve to the same grammar as their
/// bare forms — a false negative the old literal name comparisons had.
pub(super) fn identify_body_args(
    cmd: &mut ParsedCommand,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
    source_offset: u32,
    config: &FormatterConfig,
    source: &str,
) {
    if identities.analysis.is_some()
        || identities.decoded_phase.is_some()
        || identities.conditional_phase.is_some()
    {
        reset_selected_command_roles(cmd);
        let Some(image) = selected_formatting_image(identities, config, registry) else {
            return;
        };
        let Ok(whole) = std::str::from_utf8(image.bytes()) else {
            return;
        };
        let Some(head) = source_offset.checked_add(cmd.head_start) else {
            return;
        };
        let aligned = if let Some(original) = &identities.original {
            let Some((segment, words)) = original.command(
                source,
                source_offset,
                head,
                cmd.args.len(),
                config.lexer_config(),
            ) else {
                return;
            };
            Some((segment.clone(), words))
        } else {
            let Ok(start) = usize::try_from(source_offset) else {
                return;
            };
            if whole.get(start..start.saturating_add(source.len())) != Some(source) {
                return;
            }
            None
        };
        let segments = if aligned.is_none() {
            tcl_source::segment::segment_commands_with_offset_and_config(
                source,
                source_offset,
                config.lexer_config(),
            )
        } else {
            Vec::new()
        };
        let Some(segment) = aligned.as_ref().map(|(segment, _)| segment).or_else(|| {
            segments.iter().find(|segment| {
                segment
                    .argv
                    .first()
                    .is_some_and(|token| token.span.start() == head)
            })
        }) else {
            return;
        };
        if let (Some(original), Some((_, words))) = (&identities.original, &aligned)
            && apply_selected_member_bodies(cmd, original, head, words)
        {
            return;
        }
        let selected = if let Some(phase) = &identities.conditional_phase {
            crate::original_invocation::conditional_lambda_registry_words(
                phase,
                segment,
                &identities.input,
            )
        } else if let Some(phase) = &identities.decoded_phase {
            crate::original_invocation::decoded_lambda_registry_words(
                phase,
                segment,
                &identities.input,
            )
        } else {
            identities.analysis.as_ref().and_then(|analysis| {
                crate::original_invocation::source_registry_words(whole, analysis, segment)
            })
        };
        let Some(selected) = selected else {
            // The shared original syntax inventory retains declaration-member
            // bodies independently of a global command schema. It donates no
            // formal value, expression rewrite or executed member identity.
            if let (Some(original), Some((_, words))) = (&identities.original, &aligned) {
                apply_original_fallback_bodies(cmd, original, words);
            }
            return;
        };
        let Some(_selected_role_owners) = apply_selected_command_roles(
            cmd,
            &selected,
            identities,
            config,
            registry,
            (whole, segment),
        ) else {
            return;
        };
        cmd.source_words = Some(selected);
        return;
    }
    // Missing whole-source advice is terminal; no nominal head reconstruction.
    cmd.source_roles = Some(Vec::new());
    cmd.logical_roles = None;
    cmd.logical_context = None;
    cmd.source_traits = Some(Traits::empty());
    cmd.resolved_name.clear();
}

/// Indices into `cmd.args` of braced expression arguments.
pub(super) fn identify_expr_args(cmd: &ParsedCommand, registry: &CommandRegistry) -> Vec<usize> {
    if let Some(roles) = cmd.source_roles.as_ref().or(cmd.logical_roles.as_ref()) {
        return roles
            .iter()
            .filter_map(|&(written, role)| (role == ArgRole::Expr).then_some(written))
            .collect();
    }
    let arg_texts: Vec<&str> = cmd.args.iter().skip(1).map(|a| a.text.as_str()).collect();
    registry
        .arg_indices_for_role(&cmd.resolved_name, &arg_texts, ArgRole::Expr)
        .into_iter()
        .map(|idx| idx + 1)
        .collect()
}

fn reset_selected_command_roles(cmd: &mut ParsedCommand) {
    cmd.source_roles = Some(Vec::new());
    cmd.logical_roles = None;
    cmd.logical_context = None;
    cmd.source_traits = Some(Traits::empty());
    cmd.source_words = None;
    cmd.resolved_name.clear();
}

fn selected_formatting_image<'a>(
    identities: &'a FormattingSourceLayout,
    config: &FormatterConfig,
    registry: &CommandRegistry,
) -> Option<&'a tcl_lexer::SourceImage> {
    if identities.input.lexer_config() != config.lexer_config()
        || identities
            .analysis
            .as_ref()
            .is_some_and(|analysis| analysis.body_lexer_config != Some(config.lexer_config()))
        || identities
            .input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            != registry.snapshot().semantic_key()
    {
        return None;
    }
    identities
        .conditional_phase
        .as_ref()
        .map(|phase| phase.image())
        .or_else(|| identities.realm.original_source_image())
}

fn apply_selected_member_bodies(
    cmd: &mut ParsedCommand,
    original: &super::super::analysis_layout::AnalysisFormattingLayout,
    head: u32,
    words: &[tcl_lexer::NativeWord],
) -> bool {
    let bodies = match original.member_body_arguments(head, words) {
        Ok(super::super::analysis_layout::MemberBodyArguments::CompleteNonMember) => None,
        Ok(super::super::analysis_layout::MemberBodyArguments::CompleteMember(bodies)) => {
            Some(bodies)
        }
        Err(_) => return true,
    };
    if let Some(bodies) = bodies {
        // The selected declaration vocabulary precedes global lookup even
        // when an unrelated global command has the same member spelling.
        for (arg, word) in cmd.args.iter_mut().zip(words) {
            arg.original_word = Some(word.clone());
        }
        for &(written, presentation) in &bodies {
            if let Some(arg) = cmd.args.get_mut(written)
                && arg.is_braced
            {
                arg.kind = ArgKind::Body;
                arg.member_body_presentation = Some(presentation);
            }
        }
        cmd.source_roles = Some(
            bodies
                .into_iter()
                .map(|(written, _)| (written, ArgRole::Body))
                .collect(),
        );
        return true;
    }
    false
}

fn apply_original_fallback_bodies(
    cmd: &mut ParsedCommand,
    original: &super::super::analysis_layout::AnalysisFormattingLayout,
    words: &[tcl_lexer::NativeWord],
) {
    let mut roles = Vec::new();
    for (written, (arg, word)) in cmd.args.iter_mut().zip(words).enumerate().skip(1) {
        arg.original_word = Some(word.clone());
        if arg.is_braced
            && word
                .content_span()
                .is_ok_and(|span| original.has_script(span))
        {
            arg.kind = ArgKind::Body;
            roles.push((written, ArgRole::Body));
        }
    }
    cmd.source_roles = Some(roles);
}

fn apply_expression_bracing(
    cmd: &mut ParsedCommand,
    whole: &str,
    segment: &tcl_source::segment::SegmentedCommand,
    identities: &FormattingSourceLayout,
    config: &FormatterConfig,
    registry: &CommandRegistry,
) {
    let mut tokens = tcl_compiler::ir::CommandTokens::from_segmented(
        &SourceMap::new(whole),
        config.lexer_config(),
        segment,
    );
    identities.realm.stamp_original_tokens(&mut tokens);
    cmd.expression_bracing = tokens.source_binding.as_ref().and_then(|binding| {
        let proof = binding.original_literal_expression_bracing(&tokens, registry)?;
        let original = proof.original_operand();
        let written = segment
            .argv
            .iter()
            .position(|token| token.span.start() == original.span().start())?;
        (written > 0).then(|| (written, proof.replacement_source_word().to_owned()))
    });
}

fn apply_selected_operand_geometry(
    cmd: &mut ParsedCommand,
    selected: &crate::original_invocation::OriginalRegistryWords,
) {
    for (ordinal, operand) in selected.operands.iter().enumerate() {
        let Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)) =
            selected.origins.get(ordinal.saturating_add(1))
        else {
            continue;
        };
        let (Some(arg), Some(operand)) = (cmd.args.get_mut(*written), operand.as_ref()) else {
            continue;
        };
        if *written == 0 {
            continue;
        }
        arg.original_word.clone_from(&operand.word);
        arg.original_input.clone_from(&operand.input);
    }
}

type SelectedCommandRoleOwners = (
    Vec<(usize, Option<tcl_registry::ArgPresentation>)>,
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
);

fn apply_selected_command_roles(
    cmd: &mut ParsedCommand,
    selected: &crate::original_invocation::OriginalRegistryWords,
    identities: &FormattingSourceLayout,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    position: (&str, &tcl_source::segment::SegmentedCommand),
) -> Option<SelectedCommandRoleOwners> {
    let (whole, segment) = position;
    let logical = match &selected.source {
        crate::original_invocation::OriginalRegistrySource::SourceTransitions(advice) => {
            advice.original_head().logical_input().is_some()
                && advice.logical_source_input() == Some(&identities.input)
        }
        _ => false,
    };
    let roles = selected.roles.as_deref().unwrap_or_default();
    let input = &identities.input;
    let context = input.context_registry();
    let (traits, presentations, case) = selected.with_source_schema(&context, |schema| {
        (
            schema.semantics.traits,
            roles
                .iter()
                .map(|&(ordinal, _)| {
                    (
                        ordinal,
                        schema.authored_source_argument_presentation(ordinal),
                    )
                })
                .collect::<Vec<_>>(),
            schema
                .authored_source_case_invocation()
                .and_then(|(descriptor, layout)| Some((descriptor, layout.clause_list_index?)))
                .or_else(|| schema.authored_source_case_presentation()),
        )
    })?;
    cmd.source_traits = Some(traits);
    cmd.source_case = case.and_then(|(descriptor, ordinal)| {
        match selected.origins.get(ordinal.checked_add(1)?)? {
            tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)
                if *written > 0 && *written < cmd.args.len() =>
            {
                Some((*written, descriptor))
            }
            _ => None,
        }
    });
    if config.enforce_braced_expr && identities.conditional_phase.is_none() {
        apply_expression_bracing(cmd, whole, segment, identities, config, registry);
    }
    // Geometry belongs to every genuine written operand, independently
    // of whether the selected schema assigns that operand a source role.
    apply_selected_operand_geometry(cmd, selected);
    let mut written_roles = Vec::new();
    for &(ordinal, role) in roles {
        let Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)) =
            selected.origins.get(ordinal.saturating_add(1))
        else {
            continue;
        };
        if *written == 0 || *written >= cmd.args.len() {
            continue;
        }
        written_roles.push((*written, role));
        let arg = &mut cmd.args[*written];
        arg.kind = match role {
            ArgRole::Body
                if arg.is_braced
                    && presentations.iter().any(|(index, presentation)| {
                        *index == ordinal
                            && presentation.is_some_and(tcl_registry::ArgPresentation::is_block)
                    }) =>
            {
                ArgKind::Body
            }
            ArgRole::LambdaLiteral if arg.is_braced && arg.original_word.is_some() => {
                ArgKind::LambdaLiteral
            }
            ArgRole::Keyword => ArgKind::Keyword,
            ArgRole::ParamList if arg.is_braced => ArgKind::ParamList,
            _ => ArgKind::Word,
        };
    }
    if logical {
        cmd.source_roles = None;
        cmd.logical_roles = Some(written_roles);
        cmd.logical_context = Some(std::sync::Arc::clone(&context));
    } else {
        cmd.source_roles = Some(written_roles);
    }
    let identity_arguments = selected
        .origins
        .iter()
        .skip(1)
        .enumerate()
        .all(|(index, origin)| {
            *origin == tcl_compiler::registry_invocation::InvocationWordOrigin::Written(index + 1)
        });
    if identity_arguments && selected.arguments.len() + 1 == cmd.args.len() {
        cmd.resolved_name.clone_from(&selected.command);
    }
    Some((presentations, context))
}
