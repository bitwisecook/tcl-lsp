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

//! Variables that hold object handles: loop variables, snit components and constructions.

use super::object_dispatch::{ObjectClassMap, object_handle_name, resolve_class_in_hierarchy};
use super::script_walk::MAX_TOKEN_RECURSION;
use tcl_compiler::analyser::{AnalysisResult, ClassHierarchy};
use tcl_lexer::Span;
#[cfg(test)]
use tcl_lexer::TokenType;

/// Walk the segmenter + comment scan and return raw
/// [`Entry`] tuples sorted by position.  Shared by `full` and `range`.
/// Augment the object-handle map with loop variables that iterate an object
/// collection — `dict for {k v} $coll {…}`, `dict map …`, `foreach v $coll {…}`,
/// `lmap …` — so a `$v method …` dispatch in the loop body resolves like a
/// `[dict get $coll $k] method …` retrieval.
///
/// The shared original executable-source walker supplies potential regions;
/// the actual per-point schema owns roles and captured operands. Inert braced
/// data supplies no loop binding. No-op when no object collection is tracked.
pub(super) fn augment_loop_var_handles(
    source: &str,
    analysis: &AnalysisResult,
    object_collections: &ObjectClassMap,
    object_classes: &mut ObjectClassMap,
) {
    if object_collections.is_empty() {
        return;
    }
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return;
    };
    let context = current.context();
    let rules = tcl_syntax::word_rules::WordValueRules::from_grammar(
        &current.config().grammar_over(current.profile().grammar),
    );
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if let Some(words) =
                tcl_compiler::registry_invocation::source_structure::source_registry_words(
                    source, analysis, command,
                )
            {
                bind_loop_vars_for_call(
                    (source, current.config()),
                    command,
                    &words,
                    &context,
                    rules,
                    object_collections,
                    object_classes,
                );
            }
            false
        },
    );
}

/// Add a loop variable → element-class set entry to the handle map (skips an
/// empty name).
fn bind_loop_var(
    handles: &mut ObjectClassMap,
    var: &str,
    classes: &std::collections::HashSet<String>,
) {
    if !var.is_empty() {
        handles
            .entry(var.to_owned())
            .or_default()
            .extend(classes.iter().cloned());
    }
}

/// Syntactic scan for object-handle bindings that the compiler CFG does not
/// surface, binding the handle variable to its class so a `$NAME method …`
/// dispatch in a snit method body resolves.  snit method bodies are **not**
/// lowered into the compiler CFG (only token-walked, like the `$self` path), so
/// a source scan is how these classes reach the handle map — the same technique
/// the loop-var scan uses.
///
/// Which calls bind a handle, and at which argument indices, is registry data
/// ([`tcl_registry::HandleBindingSpec`]) — the walker names no command, so
/// `::set` and a provable alias of it bind exactly like `set`.  Two layouts
/// exist:
///
/// - `install NAME using TYPE …`, snit's component installer, declared on the
///   snit definition-body grammar's
///   [`member_body_commands`](tcl_registry::definer::DefinitionBodyGrammar::member_body_commands)
///   because the word exists only inside a snit member body; and
/// - `set NAME [TYPE inst …]`, whose value word may be a *bare-word*
///   construction (`$type $name` creates an instance) — gated on `TYPE` being a
///   visible class of a family whose grammar declares
///   [`bare_word_construction`](tcl_registry::definer::DefinitionBodyGrammar::bare_word_construction),
///   and whose first argument is **not** a typemethod (`info` / `destroy` / a
///   declared `typemethod`), which would be a type-command call rather than a
///   construction.
///
/// Highlight-only and sound by abstention: the bare-constructor form only fires
/// when `TYPE` is visible in the hierarchy (local, or workspace-merged in
/// project mode), so an unknown type is never guessed at.
pub(super) fn augment_snit_handles(
    source: &str,
    analysis: &AnalysisResult,
    classes: Option<&ClassHierarchy>,
    object_classes: &mut ObjectClassMap,
) {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return;
    };
    let context = current.context();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            bind_original_handle_construction(
                source,
                analysis,
                command,
                &context,
                classes,
                object_classes,
            );
            false
        },
    );
    let Some(walk) = crate::refactor::FrameWalk::new(source, analysis) else {
        return;
    };
    for (region, grammar) in selected_member_handle_regions(source, analysis, &context) {
        let Some(text) = source.get(region.as_range()) else {
            continue;
        };
        for root in walk.segment_nested(text, region.start()) {
            let mut commands = Vec::new();
            walk.nested_same_frame_commands(source, &root, &mut commands);
            commands.push(root);
            for command in commands {
                let Some(original) = walk.native_words(source, &command) else {
                    continue;
                };
                let values = original
                    .iter()
                    .map(tcl_syntax::word_rules::original_static_word_ascii_presentation)
                    .map(|value| value.and_then(|bytes| String::from_utf8(bytes).ok()))
                    .collect::<Vec<_>>();
                let Some(head) = values.first().and_then(Option::as_deref) else {
                    continue;
                };
                let arguments = original
                    .iter()
                    .zip(&values)
                    .skip(1)
                    .map(|(word, value)| {
                        if word.group().expand {
                            tcl_registry::InvocationWord::Expanded
                        } else if let Some(value) = value.as_deref() {
                            tcl_registry::InvocationWord::Literal(value)
                        } else {
                            tcl_registry::InvocationWord::Dynamic
                        }
                    })
                    .collect::<Vec<_>>();
                if let Some(binding) = grammar
                    .member_body_command(head)
                    .and_then(|command| command.binds_handle)
                    && let Some(bound) = binding
                        .resolve_words(tcl_registry::InvocationArguments::structured(&arguments))
                    && matches!(bound.class_source, tcl_registry::HandleClassSource::Word(_))
                {
                    bind_object_handle(&bound, classes, object_classes);
                }
            }
        }
    }
}

/// The class a declaration names in this analysis: its canonical record
/// under a pinned release, its retained logical record otherwise.
fn declared_source_class<'a>(
    declaration: &tcl_compiler::command_binding::OriginalSourceClassDeclaration,
    analysis: &'a AnalysisResult,
) -> Option<&'a tcl_compiler::analyser::ClassDef> {
    declaration
        .source_class(analysis)
        .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::metadata)
        .or_else(|| declaration.logical_source_class(analysis))
}

/// The actual selected factory and sealed member-script vectors own the
/// installer vocabulary. This is conditional source metadata only.
pub(super) fn selected_member_handle_regions(
    source: &str,
    analysis: &AnalysisResult,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<(Span, &'static tcl_registry::definer::DefinitionBodyGrammar)> {
    use tcl_compiler::registry_invocation::source_structure;
    let mut pending = Vec::new();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            let declaration = source_structure::source_class_declaration_at(
                source,
                analysis,
                command.span.start(),
            )
            .or_else(|| {
                source_structure::source_configured_class_at(source, analysis, command.span.start())
                    .map(|target| target.class_declaration().clone())
            });
            if let Some(declaration) = declaration
                && declared_source_class(&declaration, analysis).is_some()
                && let Some(grammar) = declaration.grammar(context)
                && !grammar.member_body_commands.is_empty()
                && let Some(words) =
                    source_structure::source_registry_words(source, analysis, command)
            {
                for body in words.source_script_bodies(context) {
                    if let Some(parent) = body.definition_parent_for(context, None) {
                        pending.push((parent, body.content_span(), grammar, 0));
                    }
                }
            }
            false
        },
    );
    let mut regions = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    while let Some((parent, region, grammar, depth)) = pending.pop() {
        if MAX_TOKEN_RECURSION.exceeded(depth) || !visited.insert((region.start(), region.end())) {
            continue;
        }
        let Some(members) = parent.definition_member_region(context, region) else {
            continue;
        };
        for original in members.original_commands() {
            for body in members.script_bodies(original).into_iter().flatten() {
                if let Some(parent) = body.definition_parent() {
                    pending.push((parent.clone(), body.content_span(), grammar, depth + 1));
                } else {
                    regions.push((body.content_span(), grammar));
                }
            }
        }
    }
    regions
}

/// A bare-word construction the analysis kept no call for because the
/// instance name is computed (`set eng [Engine ${selfns}::e]`). The written
/// type word must name a class of the hierarchy whose definer's grammar
/// constructs on an unknown first word; a literal first word is left to the
/// analysis, which knows whether it is a type method.
fn bind_written_bare_construction(
    setter: &tcl_compiler::registry_invocation::OriginalSourceHandleConstruction,
    context: &tcl_registry::model::ContextRegistry,
    classes: Option<&ClassHierarchy>,
    handles: &mut ObjectClassMap,
) {
    let words = &setter.construction().words;
    let (Some(hierarchy), [head, name, ..]) = (classes, words.as_slice()) else {
        return;
    };
    if tcl_syntax::word_rules::original_static_word_ascii_presentation(name).is_some() {
        return;
    }
    let Some(class) = tcl_syntax::word_rules::original_static_word_ascii_presentation(head)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|written| resolve_class_in_hierarchy(hierarchy, &written))
        .and_then(|qualified| hierarchy.classes.get(&qualified))
    else {
        return;
    };
    let bare_word = class.class_command_fallback.constructs_named_instance()
        || context
            .commands()
            .get(&class.metaclass)
            .and_then(|definer| definer.definition_body)
            .is_some_and(|grammar| grammar.bare_word_construction);
    if !bare_word {
        return;
    }
    if let Ok(variable) = std::str::from_utf8(setter.variable_bytes())
        && !variable.is_empty()
    {
        handles
            .entry(variable.to_owned())
            .or_default()
            .insert(class.qualified_name.clone());
    }
}

/// A selected setter and current constructor graph retain independent source
/// producers; no displayed bracket text supplies a receiver or stored value.
fn bind_original_handle_construction(
    source: &str,
    analysis: &AnalysisResult,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    context: &tcl_registry::model::ContextRegistry,
    classes: Option<&ClassHierarchy>,
    handles: &mut ObjectClassMap,
) {
    use tcl_compiler::registry_invocation::source_structure;
    let Some(setter) =
        source_structure::source_handle_construction_at(source, analysis, command.span.start())
    else {
        return;
    };
    let Some(head) = setter.construction().words.first() else {
        return;
    };
    let Some(call) =
        source_structure::source_constructor_call_at(source, analysis, head.span().start())
    else {
        bind_written_bare_construction(&setter, context, classes, handles);
        return;
    };
    let Some(class) = declared_source_class(call.class_declaration(), analysis) else {
        return;
    };
    let Some(grammar) = call.class_declaration().grammar(context) else {
        return;
    };
    if !grammar.bare_word_construction
        && !class
            .factory
            .as_ref()
            .is_some_and(|factory| factory.unknown_binds_instance)
    {
        return;
    }
    let arguments = call.arguments();
    let Some(selector) = arguments
        .first()
        .and_then(|word| word.literal_bytes())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return;
    };
    if grammar.manufacturer(selector).is_none()
        && (grammar.is_builtin_type_method(selector) || class.class_methods.contains_key(selector))
    {
        return;
    }
    let Ok(name) = std::str::from_utf8(setter.variable_bytes()) else {
        return;
    };
    if !name.is_empty() {
        handles
            .entry(name.to_owned())
            .or_default()
            .insert(class.qualified_name.clone());
    }
}

/// Bind one resolved [`BoundHandle`] into the handle map.
///
/// The two class sources are resolved differently and both abstain rather than
/// guess: a [`HandleClassSource::Word`] is literal type-name data selected
/// from the original grouped word, while a
/// [`HandleClassSource::ConstructionValue`] must parse as a command
/// substitution whose head is a visible class of a family that constructs by
/// bare word.
///
/// [`BoundHandle`]: tcl_registry::BoundHandle
/// [`HandleClassSource::Word`]: tcl_registry::HandleClassSource::Word
/// [`HandleClassSource::ConstructionValue`]: tcl_registry::HandleClassSource::ConstructionValue
fn bind_object_handle(
    bound: &tcl_registry::BoundHandle<'_>,
    classes: Option<&ClassHierarchy>,
    handles: &mut ObjectClassMap,
) {
    if bound.name.is_empty() {
        return;
    }
    match bound.class_source {
        tcl_registry::HandleClassSource::Word(_) => {
            let type_name = bound.class_word;
            if type_name.is_empty() {
                return;
            }
            let Some(hierarchy) = classes else { return };
            let Some(qualified) = resolve_class_in_hierarchy(hierarchy, type_name) else {
                return;
            };
            handles
                .entry(bound.name.to_owned())
                .or_default()
                .insert(qualified);
        }
        tcl_registry::HandleClassSource::ConstructionValue(_) => {}
    }
}

/// Bind every loop variable of one call that iterates a known object
/// collection.
///
/// Registry-driven, with no command spelling anywhere: the
/// [`ArgRole::LoopVarList`] indices come from the registry
/// (`foreach`/`lmap`'s repeated `(start 0, stride 2)` layout, `dict for` /
/// `dict map` / `array for`'s static role tables), and the iterated
/// collection is always the word after the variable list. `::`-qualified and
/// aliased spellings therefore classify exactly like bare ones.
///
/// The variable-list shape follows the collection's declared argument type:
/// a `Dict` collection binds a `{keyVar valueVar}` pair whose *value*
/// variable holds the element, and anything else binds every variable in the
/// group.
fn bind_loop_vars_for_call(
    written_source: (&str, tcl_lexer::LexerConfig),
    command: &tcl_compiler::segmenter::SegmentedCommand,
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
    rules: tcl_syntax::word_rules::WordValueRules,
    collections: &ObjectClassMap,
    handles: &mut ObjectClassMap,
) {
    let (source, config) = written_source;
    if command.is_partial {
        return;
    }
    let Some(argument_roles) = words.roles() else {
        return;
    };
    let original_words = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        command,
    );
    for &(ordinal, role) in argument_roles {
        if role != tcl_registry::ArgRole::LoopVarList {
            continue;
        }
        let Some(value) = words
            .arguments()
            .get(ordinal)
            .and_then(|word| word.literal_bytes())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            continue;
        };
        let Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)) =
            ordinal
                .checked_add(2)
                .and_then(|index| words.origins().get(index))
        else {
            continue;
        };
        if command.single_token_word.get(*written) != Some(&true) {
            continue;
        }
        let Some(classes) = original_words
            .word_exprs
            .get(*written)
            .and_then(tcl_compiler::ir::WordExpr::sole_variable_substitution)
            .and_then(|(spelling, _)| object_handle_name(spelling, Some(config)))
            .and_then(|name| collections.get(name))
        else {
            continue;
        };
        let Ok(names) = rules.split_list(value) else {
            continue;
        };
        let keyed = words.with_source_schema(context, |schema| {
            schema
                .facts()
                .argument_type_hint(ordinal + 1)
                .is_some_and(|hint| hint.expected == Some(tcl_registry::types::TclType::Dict))
        }) == Some(true);
        if keyed {
            if let Some(name) = names.get(1) {
                bind_loop_var(handles, name, classes);
            }
        } else {
            for name in &names {
                bind_loop_var(handles, name, classes);
            }
        }
    }
}

/// Decode a literal loop-variable-list word using Tcl's canonical list
/// grammar. Substitutions, expansions, compound words, incomplete commands,
/// and malformed list values all abstain.
#[cfg(test)]
pub(super) fn static_loop_var_names(
    source: &str,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    word_index: usize,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<Vec<String>> {
    if seg.is_partial || seg.single_token_word.get(word_index) != Some(&true) {
        return None;
    }
    let token = *seg.argv.get(word_index)?;
    if matches!(
        token.kind,
        TokenType::Var | TokenType::Cmd | TokenType::Expand
    ) {
        return None;
    }
    // The segmenter strips a braced/quoted word's delimiters. Validate the
    // original token too, so incomplete recovery text cannot masquerade as a
    // valid list value.
    let raw_start = token.span.start() as usize;
    let raw_end = token.span.end() as usize;
    let raw = source.get(raw_start..raw_end)?;
    if raw.starts_with(['{', '"']) {
        let closing = if raw.starts_with('{') { b'}' } else { b'"' };
        let raw = if source.as_bytes().get(raw_end) == Some(&closing) {
            source.get(raw_start..raw_end + 1)?
        } else {
            raw
        };
        tcl_syntax::list::find_element(raw, 0).ok()??;
    }
    let text = seg.texts.get(word_index)?;
    rules.split_list(text).ok().map(|elements| {
        elements
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .collect()
    })
}
