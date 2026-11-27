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

//! Seeding the facts: constructor calls and naming factories found in each unit and in the original source.

use super::FactSink;
use crate::compilation_unit::FunctionUnit;
use crate::ir::Statement;
use crate::value_shapes::parse_command_substitution_with_config;
use tcl_core_types::BodyFrameId;
use tcl_registry::CommandRegistry;

pub(super) fn harvest_unit(
    fu: &FunctionUnit,
    frame: Option<BodyFrameId>,
    registry: &CommandRegistry,
    sink: &mut FactSink,
) {
    // Syntactic constructor assignments (`set VAR [Class new|create …]`) and
    // naming object-factories (`struct::graph myG` — the created object command
    // is `myG`, not a `set` target the SSA lattice can carry).
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignValue { name, value, .. } => {
                    if let Some((_head, args)) = parse_command_substitution_with_config(
                        value.trim(),
                        tcl_lexer::LexerConfig::for_profile(registry.profile()),
                    ) {
                        if let Some(class) = constructor_class(
                            value,
                            registry,
                            stmt.tokens(),
                            sink.metadata_context,
                            sink.source_analysis,
                        ) {
                            sink.bind(frame, name, &class);
                        }
                        // The nested-constructor half of the fast-path gate:
                        // `set w [Wrap new [listbox .l]]` reaches `arg_classes`
                        // through `emit_ctor_param_bindings`.
                        sink.note_constructor_args(&args, registry);
                    }
                }
                // Source advice uses the selected descriptor's naming layout;
                // normal handler candidates retain their independent proof.
                Statement::Call { args, .. } => {
                    if let Some(tokens) = stmt.tokens() {
                        harvest_source_command_candidates(tokens, frame, registry, sink);
                    }
                    // Both parameter edges read a call's arguments through
                    // `arg_classes`, whose `[Factory new]` branch needs no
                    // seeded handle — so the fast-path gate must see them.
                    sink.note_constructor_args(args, registry);
                }
                _ => {}
            }
        }
    }
    // SSA values typed `OBJECT(class)` — includes collection retrievals
    // (`set p [dict get $pins $k]`) the syntactic scan above cannot see.
    for ((sym, _ver), t) in fu.types.iter() {
        if t.tcl_type() == Some(tcl_registry::TclType::Object)
            && let Some(class) = t.class_name()
        {
            let name = fu.ssa.var_name(*sym).to_owned();
            sink.bind(frame, &name, class);
        }
    }
}

pub(super) fn harvest_script_factories(
    script: &crate::ir::Script,
    owner: Option<BodyFrameId>,
    registry: &CommandRegistry,
    sink: &mut FactSink,
) {
    let mut pending = vec![script];
    while let Some(script) = pending.pop() {
        for statement in &script.statements {
            if let Some(tokens) = script.retained_source_tokens_for_statement(statement) {
                harvest_source_command_candidates(tokens, owner, registry, sink);
            }
            pending.extend(statement.child_scripts());
        }
    }
}

fn harvest_source_command_candidates(
    tokens: &crate::ir::CommandTokens,
    owner: Option<BodyFrameId>,
    registry: &CommandRegistry,
    sink: &mut FactSink,
) {
    for assignment in source_assignment_candidates(registry, sink.metadata_context, tokens) {
        if let Some(class) =
            sole_substitution_tokens(&assignment.value, tokens, registry).and_then(|child| {
                constructor_class_candidate(
                    &child,
                    registry,
                    sink.metadata_context,
                    sink.source_analysis,
                )
            })
        {
            sink.bind(owner, &assignment.name, &class);
        }
    }
    for factory in source_object_factory_candidates(registry, sink.metadata_context, tokens) {
        if let Some((name, class)) = factory.publication {
            sink.bind(owner, &name, class);
        }
    }
}

pub(super) fn source_assignment_candidates(
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    tokens: &crate::ir::CommandTokens,
) -> Vec<crate::registry_invocation::AdvisoryValueAssignment> {
    let Some(context) = context else {
        return Vec::new();
    };
    if context.permits_logical_source_names() {
        return crate::registry_invocation::original_advisory_value_assignments_with_metadata_context(
            registry,
            Some(context),
            tokens,
        );
    }
    source_normal_representation(registry, Some(context), tokens)
        .and_then(|normal| normal.value_assignment())
        .map(
            |assignment| crate::registry_invocation::AdvisoryValueAssignment {
                name: assignment.name,
                value: assignment.value,
            },
        )
        .into_iter()
        .collect()
}

fn source_object_factory_candidates(
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    tokens: &crate::ir::CommandTokens,
) -> Vec<crate::registry_invocation::AdvisoryObjectFactory> {
    let Some(context) = context else {
        return Vec::new();
    };
    if context.permits_logical_source_names() {
        return crate::registry_invocation::advisory_object_factories_with_metadata_context(
            registry, context, tokens,
        );
    }
    let Some(normal) = source_normal_representation(registry, Some(context), tokens) else {
        return Vec::new();
    };
    vec![crate::registry_invocation::AdvisoryObjectFactory {
        publication: normal
            .naming_factory_candidate()
            .filter(|(name, _)| is_plain_object_name(name)),
        result_class: normal.callable_result_class_candidate(),
    }]
}

/// Query original normal metadata only under its supplied source generation.
/// Independent construction-result receipts remain separate from this query.
fn source_normal_representation(
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    tokens: &crate::ir::CommandTokens,
) -> Option<crate::registry_invocation::NormalRepresentationInvocation> {
    crate::registry_invocation::normal_representation_invocation_with_metadata_context(
        registry,
        Some(context?),
        tokens,
    )
}

/// The registry class named by a class-command manufacturer value, or `None`
/// when the value is not such a call. A `TclOO` class command may be
/// written with or without the leading `::` global qualifier; the registry's
/// [`CommandRegistry::object_class`] strips it as [`CommandRegistry::get`] does.
pub(super) fn constructor_class(
    value: &str,
    registry: &CommandRegistry,
    parent: Option<&crate::ir::CommandTokens>,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    source_analysis: Option<&crate::analyser::AnalysisResult>,
) -> Option<String> {
    let parent = parent?;
    let mut result = None;
    for word in parent
        .words()
        .iter()
        .filter(|word| word.legacy_text() == value)
    {
        let tokens = sole_substitution_tokens(word, parent, registry)?;
        let class = constructor_class_candidate(&tokens, registry, context, source_analysis)?;
        if result.as_ref().is_some_and(|previous| previous != &class) {
            return None;
        }
        result = Some(class);
    }
    result
}

pub(super) fn sole_substitution_tokens(
    word: &tcl_source::words::WordExpr,
    parent: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
) -> Option<crate::ir::CommandTokens> {
    word.sole_command_substitution()?;
    let mut commands = crate::value_shapes::command_substitution_tokens(
        word,
        Some(parent),
        parent.native_lexer_config(tcl_lexer::LexerConfig::for_profile(registry.profile())),
    )?;
    (commands.len() == 1).then(|| commands.remove(0))
}

pub(super) fn constructor_class_candidate(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    source_analysis: Option<&crate::analyser::AnalysisResult>,
) -> Option<String> {
    if let Some(class) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.proved_construction_result(registry))
    {
        return Some(class);
    }
    let context = context?;
    if context.permits_logical_source_names()
        && let Some(analysis) = source_analysis
        && let Some(class) = original_logical_constructor_class(analysis, context, tokens)
    {
        return Some(class);
    }
    if let Some(binding) = tokens.source_binding.as_ref() {
        let candidates = binding.class_factory_candidates(registry);
        if let Some(first) = candidates.first()
            && candidates
                .iter()
                .all(|candidate| candidate.command == first.command)
        {
            return Some(first.command.clone());
        }
    }
    let factories = source_object_factory_candidates(registry, Some(context), tokens);
    let mut classes = factories.iter().filter_map(|factory| factory.result_class);
    if let Some(class) = classes.next() {
        return classes
            .all(|other| other == class)
            .then(|| class.to_owned());
    }
    // A factory that publishes an instance command returns that command's
    // name (`set t [ttk::treeview .tree]` leaves `.tree` in `t`), so the
    // word holding the result is a handle of the published class.
    let mut published = factories
        .iter()
        .filter_map(|factory| factory.publication.as_ref().map(|(_, class)| *class));
    if let Some(class) = published.next() {
        return published
            .all(|other| other == class)
            .then(|| class.to_owned());
    }
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry,
        Some(context),
        tokens,
    )?;
    let head = &invocation.facts.canonical_command;
    if let Some(method) = invocation.argument_literal(0)
        && registry
            .exported_manufacturer_method(head, &method)
            .is_some()
    {
        return registry
            .object_class(head)
            .map(|class| class.class_name.to_owned());
    }
    None
}

pub(super) fn original_logical_constructor_class(
    analysis: &crate::analyser::AnalysisResult,
    context: crate::registry_invocation::InvocationMetadataContext<'_>,
    tokens: &crate::ir::CommandTokens,
) -> Option<String> {
    let input = context.source_analysis_input()?;
    if !context.permits_logical_source_names() || analysis.resolved_input.as_ref() != Some(input) {
        return None;
    }
    let image = analysis.retained_command_realm()?.original_source_image()?;
    let offset = tokens.words().first()?.source().span.start();
    let original = crate::registry_invocation::original_native_compiler_words(
        image,
        tokens.words(),
        offset,
        input.lexer_config(),
    )?;
    let call = crate::registry_invocation::source_structure::source_constructor_call_at(
        image.try_text().ok()?,
        analysis,
        offset,
    )?;
    (call.original_words() == original).then_some(())?;
    call.logical_constructor_shape(analysis)?;
    let class = call.class_declaration().logical_source_class(analysis)?;
    Some(class.qualified_name.clone())
}

/// Could `word` be the bracketed registry-constructor call that makes
/// [`arg_classes`] yield a class with **no** seeded handle in play
/// (`take [listbox .l]`)?
///
/// Deliberately over-approximate and deliberately cheap: it reads the head word
/// with a string split instead of lexing, because it runs on every argument of
/// every call during the harvest.  A false "yes" only costs the propagation
/// walk that would have run anyway; a false "no" would drop a binding, so a
/// head this cannot read as a bareword (a braced, quoted, substituted, or
/// expanded head) answers "yes".
pub(super) fn may_be_constructor_word(word: &str, registry: &CommandRegistry) -> bool {
    let trimmed = word.trim();
    if !trimmed.starts_with('[') {
        return false;
    }
    let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|w| w.strip_suffix(']'))
        .map(str::trim_start)
    else {
        return false;
    };
    let head = inner
        .find(char::is_whitespace)
        .map_or(inner, |end| &inner[..end]);
    if head.is_empty() || head.starts_with(['{', '"', '$', '[', '\\']) {
        return true;
    }
    registry.object_class(head).is_some()
        || registry
            .get(head)
            .is_some_and(|s| s.creates_instance_at.is_some())
}

/// Whether `name` is a plain object-command name a naming factory binds — a
/// bareword, not a `$var` / `[subst]` and not one of the `struct` deserialise
/// operator words (`= | := | as | deserialize`) that occupy the name slot when
/// the object is built from a source instead of freshly named.
fn is_plain_object_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(['$', '[', '{'])
        && !matches!(name, "=" | ":=" | "as" | "deserialize")
}
