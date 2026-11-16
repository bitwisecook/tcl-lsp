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

//! The argument roles a command's registry row, procedure definition or class gives its words.

use super::argument_override::{ArgOverride, head_is_computed};
use super::command_tokens::document_member;
use super::legend::{MOD_DECLARATION, MOD_DEFAULT_LIBRARY, TokenKind};
use super::object_dispatch::{insert_object_method_overrides, retag_original_variable_argument};
use super::script_walk::ScriptCtx;
use super::variable_name_roles::{
    OriginalProcArgRoles, proc_command_indices, proc_var_read_indices, proc_var_write_indices,
};
use super::{original, receiver_source};
use rustc_hash::{FxHashMap, FxHashSet};
use tcl_lexer::{Span, TokenType};
use tcl_registry::definer::DefinitionBodyGrammar;

/// Head colour of a command with no selected invocation: its descriptor's
/// kind when the spelling still names the document's own command.
pub(super) fn unrecorded_head_kind(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
) -> (TokenKind, u32) {
    let Some(traits) = unrecorded_command_spec(ctx, seg).map(|spec| spec.traits) else {
        return (TokenKind::Function, 0);
    };
    if traits.contains(tcl_registry::Traits::LANGUAGE_KEYWORD) {
        (TokenKind::Keyword, 0)
    } else if traits.contains(tcl_registry::Traits::OPERATOR_COMMAND) {
        (TokenKind::Operator, 0)
    } else {
        (TokenKind::Function, MOD_DEFAULT_LIBRARY)
    }
}

/// The document's own command for a written bareword head the analysis kept
/// no invocation for (a body it did not enter). Display only: the spelling
/// selects the descriptor, so it grants no binding.
pub(super) fn unrecorded_command_spec(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
) -> Option<&'static tcl_registry::CommandSpec> {
    use tcl_compiler::realm::RealmBindingFact;
    let head = seg.argv.first()?;
    (head.kind == TokenType::Esc && seg.single_token_word.first() == Some(&true)).then_some(())?;
    let text = seg.texts.first()?;
    let Some(analysis) = ctx.analysis else {
        // A source shell knows no rebinding: the spelling names the command.
        ctx.walk.shell?;
        return ctx.generation.resolve_command(text);
    };
    let realm = analysis.retained_command_realm()?;
    let name = match realm.binding_at(text, head.span.start()) {
        RealmBindingFact::Unchanged => text.as_str(),
        RealmBindingFact::Command(name) => name,
        RealmBindingFact::Rebound => {
            // A site the interpretation dispatched and could not prove stays
            // unknown. One it never reached has no fact at all: the spelling
            // still names the document's own command for display, unless the
            // document itself rebinds that name.
            let tail = tcl_syntax::naming::written_command_tail(text.as_bytes());
            if realm.source_bindings_ref().reached_site(head.span.start())
                || ctx.walk.rebound_tails.contains(tail)
            {
                return None;
            }
            text.as_str()
        }
    };
    ctx.generation.resolve_command(name)
}

/// Registry roles of an unrecorded command, from its written spellings.
fn insert_unrecorded_command_roles(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if let Some(spec) = unrecorded_command_spec(ctx, seg) {
        original::insert_written_overrides(ctx, seg, spec, overrides);
    }
}

pub(super) fn command_role_overrides(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    original: Option<&crate::original_invocation::OriginalRegistryWords>,
    member_bodies: Option<
        &[tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody],
    >,
) -> FxHashMap<u32, ArgOverride> {
    let mut overrides = original.map_or_else(FxHashMap::default, |words| {
        original::registry_overrides(ctx, seg, words)
    });
    for body in member_bodies.unwrap_or_default() {
        if let [token] = body.original_container().tokens() {
            overrides.insert(token.span.start(), ArgOverride::BodyScript);
        }
    }
    if ctx.lexical {
        insert_lexical_user_overrides(ctx, seg, &mut overrides);
    } else if let Some(providers) = ctx.project_classes {
        // Native lexical/report overlays remain guarded by has_head. This
        // independent source-display branch requires the selected original
        // class/instance and preserves stronger role overrides on insertion.
        receiver_source::insert_project_method(ctx, seg, providers, &mut overrides);
    } else {
        insert_object_method_overrides(ctx, seg, ctx.classes, &mut overrides);
    }
    original::insert_definition_member_overrides(ctx, seg, &mut overrides);
    if let Some(member) = document_member(ctx, seg) {
        use tcl_registry::ArgRole;
        for &(ordinal, role) in member.arg_roles {
            let word = usize::from(ordinal) + 1;
            let Some(token) = seg.argv.get(word) else {
                continue;
            };
            let single = seg.single_token_word.get(word) == Some(&true);
            let kind = match role {
                ArgRole::Body if token.kind == TokenType::Str => ArgOverride::BodyScript,
                ArgRole::ParamList if single => ArgOverride::ParamList,
                ArgRole::Name if single => ArgOverride::MemberName,
                _ => continue,
            };
            overrides.entry(token.span.start()).or_insert(kind);
        }
    } else if original.is_none() {
        insert_generic_option_overrides(seg, &mut overrides);
        insert_unrecorded_command_roles(ctx, seg, &mut overrides);
    }
    overrides
}

/// Generic `-option` / option-value highlighting for a command with a
/// *computed* head not resolved by the registry — a `$obj method …` object
/// dispatch, a `[Class new] method …`, or a multi-fragment `chartV$node …`
/// head.
///
/// A *registered* command's declared option set is authoritative — `puts -foo`
/// stays a string because `puts` declares no `-foo`
/// ([`insert_option_and_subcommand_overrides`]).  A plain bareword head is a
/// (possibly user-defined) command name and is left to the registry too, so
/// `mycmd -foo` stays a string.  Only a computed head — where the real option
/// set lives on a method / ensemble the registry does not model — is treated as
/// the overwhelmingly-common `-switch value` shape: those pairs are coloured
/// like any built-in's.
///
/// A "clean option" is a single-token [`TokenType::Esc`] word for which
/// [`is_generic_option_word`] holds — `-<letter>…`, excluding substitution
/// forms, negative numbers (including the `-inf` / `-nan` special-float
/// literals), a bare `-`, and `--`.  The single-token check is what excludes
/// `-$var` / `-{$var}` / `-[cmd]`, which keep their variable / command
/// highlight.
///
/// Tcl's `--` end-of-options marker is honoured: `--` itself is coloured as an
/// option marker, and scanning stops there — every following word is a
/// positional operand, even if it reads like `-foo` (`$obj cfg -- -literal`
/// leaves `-literal` a plain string).
///
/// The word immediately following an option is recoloured [`TokenKind::OptionValue`]
/// when it is a literal (`Esc`/`Str`) that is not itself an option and not
/// `--` — a `$var` / `[cmd]` value keeps its own highlight, and a following
/// option stays an option.  Arity is unknown for an undeclared command, so at
/// most the single adjacent value word is claimed; a boolean option followed by
/// another option therefore claims no value.
fn insert_generic_option_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // Only a *computed* head — a `$var` / `[cmd]` substitution or a
    // multi-fragment word (`chartV$node`) — is treated as a runtime dispatch
    // (object handle, ensemble) whose `-switch value` pairs are options.  A
    // plain single-token bareword head is a (possibly user-defined) command
    // name; deferring to the registry there keeps a bareword call conservative
    // — `mycmd -foo` stays a string, a user command's `test … -body …` is not
    // mistaken for tcltest's, and an OO-body member (`property … -get {…}`)
    // keeps its recursed bodies — exactly as `puts -foo` stays a string.
    if !head_is_computed(seg) {
        return;
    }
    // The single-token literal (`Esc`) text of word `i`, or `None` for a
    // substitution / braced / multi-fragment word.
    let literal_word = |i: usize| -> Option<&str> {
        (seg.single_token_word.get(i).copied().unwrap_or(false)
            && seg
                .argv
                .get(i)
                .is_some_and(|t| matches!(t.kind, TokenType::Esc)))
        .then(|| seg.texts.get(i).map(String::as_str))
        .flatten()
    };
    let mut i = 1;
    while i < seg.texts.len() {
        let Some(text) = literal_word(i) else {
            i += 1;
            continue;
        };
        // `--` ends option processing (Tcl convention). Colour the marker, then
        // stop — nothing after it is an option.
        // registry-axis-ok: irreducible — this function only runs for a
        // computed head with no selected invocation, so there is no
        // per-command `EndsOptions` row here to read; until never
        if text == "--" {
            if let Some(tok) = seg.argv.get(i) {
                overrides
                    .entry(tok.span.start())
                    .or_insert(ArgOverride::Decorator);
            }
            break;
        }
        if !is_generic_option_word(text) {
            i += 1;
            continue;
        }
        if let Some(tok) = seg.argv.get(i) {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Decorator);
        }
        // The immediately-following literal word is this option's value when it
        // is not itself an option and not the `--` marker.  A `$var` / `[cmd]`
        // value falls through the `Esc | Str` check and keeps its own highlight.
        let vi = i + 1;
        if let Some(val_tok) = seg.argv.get(vi)
            && matches!(val_tok.kind, TokenType::Esc | TokenType::Str)
            // registry-axis-ok: irreducible — same reason as this function's
            // other "--" check above: no `CommandSpec` exists for this head;
            // until never
            && literal_word(vi).is_none_or(|w| w != "--" && !is_generic_option_word(w))
        {
            overrides
                .entry(val_tok.span.start())
                .or_insert(ArgOverride::Kind(TokenKind::OptionValue));
        }
        i += 1;
    }
}

/// Whether a literal word reads as a generic `-option` switch on an unknown
/// command head: a leading `-` then an ASCII letter, excluding the negative
/// special-float literals Tcl's parser accepts as numbers (`-inf`, `-Inf`,
/// `-infinity`, `-nan`, …).  A bare `-`, `--`, and ordinary negative numbers
/// (`-5`, `-1.6`) are already excluded by the "letter after the dash" rule.
fn is_generic_option_word(text: &str) -> bool {
    let Some(rest) = text.strip_prefix('-') else {
        return false;
    };
    if !rest.as_bytes().first().is_some_and(u8::is_ascii_alphabetic) {
        return false;
    }
    // `-inf` / `-infinity` / `-nan` are negative floating-point values, not
    // options — Tcl's `expr` and numeric commands parse them as numbers.
    !(rest.eq_ignore_ascii_case("inf")
        || rest.eq_ignore_ascii_case("infinity")
        || rest.eq_ignore_ascii_case("nan"))
}

/// User-procedure traits and entered OO source grammar are independent of
/// builtin source descriptors. They cannot recover a refused builtin head.
fn insert_lexical_user_overrides(
    ctx: ScriptCtx<'_>,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    insert_object_method_overrides(
        ctx,
        command,
        if ctx.original_roles.has_head(command.argv[0].span.start()) {
            None
        } else {
            ctx.classes
        },
        overrides,
    );
}

/// The analysed class one of this command's words is a body of. A method body
/// the analysis never entered still belongs to its class for display, so a
/// self-dispatch inside it reads that class's declared methods. This names no
/// receiver and grants no edit.
pub(super) fn declared_class_for_body<'a>(
    ctx: ScriptCtx<'a>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
) -> Option<&'a str> {
    ctx.analysis?
        .class_body_spans
        .iter()
        .find(|(_, span)| {
            seg.argv
                .iter()
                .skip(1)
                .any(|token| token.span.start().abs_diff(span.start()) <= 1)
        })
        .map(|(name, _)| name.as_str())
}

/// Retain only the actual selected outer source grammar and the independent
/// enclosing member vocabulary. Member bodies leave definition context;
/// the descriptor's wrapper-block form keeps the enclosing vocabulary.
pub(super) fn next_original_definition_grammar(
    ctx: ScriptCtx<'_>,
    head: &str,
    arguments: &[&str],
    outer: Option<&'static DefinitionBodyGrammar>,
) -> Option<&'static DefinitionBodyGrammar> {
    if let Some(grammar) = ctx
        .oo_grammar
        .and_then(|current| ctx.registry.authored_document_member_grammar(current, head))
    {
        return Some(grammar);
    }
    if outer.is_some() {
        return outer;
    }
    if let Some(grammar) = ctx.oo_grammar
        && let Some(member) = grammar.member(head)
    {
        return (member.wrapper_block_body
            && arguments
                .first()
                .is_some_and(|inner| !grammar.is_member(inner)))
        .then_some(grammar);
    }
    ctx.oo_grammar
}

pub(super) fn insert_original_proc_role_overrides(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) -> Vec<(tcl_lexer::Span, TokenKind, u32)> {
    let Some(analysis) = ctx.analysis else {
        return Vec::new();
    };
    if let Some(index) = ctx.proc_roles
        && !index.project_source.is_empty()
        && let Some(prototype) = tcl_compiler::registry_invocation::source_structure::original_project_procedure_arguments(
            ctx.full_source, analysis, seg, &index.project_source,
        )
        && analysis.resolved_input.as_ref().is_some_and(|input| {
            prototype.matches_source(&tcl_lexer::SourceImage::document(ctx.full_source), input)
        })
    {
        let metadata = prototype.declaration().metadata();
        return insert_original_procedure_argument_role_geometry(
            seg, prototype.bindings(), prototype.arguments(), prototype.operands(),
            [&proc_var_write_indices(metadata), &proc_var_read_indices(metadata), &proc_command_indices(metadata)],
            overrides,
        );
    }
    if ctx.lexical {
        let Some(prototype) = tcl_compiler::registry_invocation::source_structure::original_logical_procedure_arguments(ctx.full_source, analysis, seg) else {
            return Vec::new();
        };
        if !prototype.matches_source(
            &tcl_lexer::SourceImage::document(ctx.full_source),
            ctx.config,
        ) || !prototype.matches_registry(ctx.registry)
            || analysis.resolved_input.as_ref() != Some(prototype.resolved_input())
        {
            return Vec::new();
        }
        let metadata = prototype.declaration().metadata();
        return insert_original_procedure_argument_role_geometry(
            seg,
            prototype.bindings(),
            prototype.arguments(),
            prototype.operands(),
            [
                &proc_var_write_indices(metadata),
                &proc_var_read_indices(metadata),
                &proc_command_indices(metadata),
            ],
            overrides,
        );
    }
    let Some(prototype) =
        tcl_compiler::registry_invocation::source_structure::original_procedure_arguments(
            ctx.full_source,
            analysis,
            seg,
        )
    else {
        return Vec::new();
    };
    if !prototype.matches_source(
        &tcl_lexer::SourceImage::document(ctx.full_source),
        ctx.config,
    ) || !prototype.matches_registry(ctx.registry)
    {
        return Vec::new();
    }
    let declaration = prototype.declaration();
    let roles = ctx
        .proc_roles
        .and_then(|index| {
            index.original_roles_at(analysis, ctx.full_source, seg.argv.first()?.span.start())
        })
        .filter(|roles| {
            roles.site.as_ref() == Some(declaration.declaration_site())
                && roles.name == *declaration.name()
        })
        .unwrap_or_else(|| {
            OriginalProcArgRoles::new(
                declaration.name(),
                Some(declaration.declaration_site()),
                declaration.metadata(),
            )
        });
    insert_original_procedure_argument_role_geometry(
        seg,
        prototype.bindings(),
        prototype.arguments(),
        prototype.operands(),
        [&roles.write, &roles.read, &roles.command],
        overrides,
    )
}

fn insert_original_procedure_argument_role_geometry(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    bindings: &[tcl_syntax::formal_params::FormalByteArgumentBinding],
    arguments: &[tcl_compiler::registry_invocation::EffectiveInvocationWord],
    operands: &[Option<
        tcl_compiler::registry_invocation::source_structure::OriginalOperandSource,
    >],
    roles: [&[u32]; 3],
    overrides: &mut FxHashMap<u32, ArgOverride>,
) -> Vec<(Span, TokenKind, u32)> {
    use tcl_syntax::formal_params::FormalByteArgumentBinding;
    let mut extents = Vec::new();
    for (parameters, kind, modifier, override_kind) in [
        (
            roles[0],
            TokenKind::Variable,
            MOD_DECLARATION,
            ArgOverride::VarDecl,
        ),
        (roles[1], TokenKind::Variable, 0, ArgOverride::VarRef),
        (roles[2], TokenKind::Function, 0, ArgOverride::CommandRef),
    ] {
        for binding in bindings {
            let (parameter, argument) = match binding {
                FormalByteArgumentBinding::Value {
                    parameter,
                    argument,
                }
                | FormalByteArgumentBinding::CallerLink {
                    parameter,
                    argument,
                    ..
                } => (*parameter, *argument),
                FormalByteArgumentBinding::Default { .. }
                | FormalByteArgumentBinding::Rest { .. } => continue,
            };
            if !u32::try_from(parameter).is_ok_and(|index| parameters.contains(&index)) {
                continue;
            }
            let Some(operand) = operands.get(argument).and_then(Option::as_ref) else {
                continue;
            };
            if let Some(word) = operand.word() {
                if !seg.argv.iter().any(|token| token.span == operand.span()) {
                    continue;
                }
                if matches!(override_kind, ArgOverride::VarDecl | ArgOverride::VarRef) {
                    retag_original_variable_argument(seg, word, override_kind.clone(), overrides);
                } else if tcl_syntax::word_rules::original_static_word_unicode_value(word).is_some()
                    && word
                        .tokens()
                        .iter()
                        .all(|token| seg.all_tokens.contains(token))
                {
                    for token in word
                        .tokens()
                        .iter()
                        .filter(|token| matches!(token.kind, TokenType::Esc | TokenType::Str))
                    {
                        overrides
                            .entry(token.span.start())
                            .or_insert_with(|| override_kind.clone());
                    }
                }
            } else if let Some(input) = operand.input()
                && arguments
                    .get(argument)
                    .and_then(|value| value.literal_bytes())
                    == Some(input.bytes())
            {
                extents.push((operand.span(), kind, modifier));
            }
        }
    }
    extents
}

/// Project the shared original produced prefix onto its own written operands.
/// Captured values have no current anchor; the builder head stays its own call.
pub(super) fn merge_list_quoted_command_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    ctx: ScriptCtx<'_>,
    produced: &crate::original_invocation::OriginalRegistryWords,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // naming.source.original-produced-command-prefix
    // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
    let overlay = original::registry_overrides(ctx, seg, produced);
    for (span_start, override_kind) in overlay {
        overrides.entry(span_start).or_insert(override_kind);
    }
    if let Some(head) = produced.head_source().and_then(|head| head.word())
        && let Some(token) = head.tokens().first()
    {
        overrides
            .entry(token.span.start())
            .or_insert(ArgOverride::CommandRef);
    }
}

/// This command's own argument slots whose registry role means "the value
/// here is later invoked/sourced as a command" — `Body` / `LambdaLiteral` /
/// `CommandPrefix` — as the set of their representative tokens' start
/// offsets. A `[…]` substitution occupying one of these slots recurses with
/// `deferred_role = true` (see [`collect_script`]) so list-quoted-lambda
/// source projection only fires for a
/// genuinely deferred invocation (`package ifneeded … [list apply {…}
/// $dir]`), never for inert data (`set x [list apply {…} value]`).
pub(super) fn deferred_role_arg_starts(
    words: Option<&crate::original_invocation::OriginalRegistryWords>,
) -> FxHashSet<u32> {
    words
        .into_iter()
        .flat_map(|words| {
            words
                .roles
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .filter(|(_, role)| {
                    matches!(
                        role,
                        tcl_registry::ArgRole::Body
                            | tcl_registry::ArgRole::CommandPrefix
                            | tcl_registry::ArgRole::LambdaLiteral
                    )
                })
                .filter_map(|(ordinal, _)| {
                    words
                        .operands
                        .get(*ordinal)?
                        .as_ref()
                        .map(|operand| operand.span.start())
                })
        })
        .collect()
}
