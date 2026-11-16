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

//! Readable script bodies a runtime barrier evaluates: replay, constructed argv, no-change proofs.

use super::super::namespace_context::{NamespaceKeyQuery, SourceNamespaceKey};
use super::super::{InvocationBindingContext, ModuleCommandBindings};
use super::binding_walk::{collect_binding_states, observe_binding_state};
use super::{
    BindingWalkContext, ResolvedBindingInvocation, RetainedBindingRoots, interpret_binding_source,
};
use crate::naming::normalise_qualified_name as nqn;
use std::collections::BTreeSet;
use tcl_registry::CommandRegistry;

/// Apply command-binding transitions from a statically readable script body
/// evaluated by a runtime barrier. Structural commands whose body location or
/// namespace cannot be expressed by the generic frame-effect grammar are
/// selected by their registry-owned semantic operation, never by spelling.
// Registry frame/body layouts require one ordered state transition.
#[allow(clippy::too_many_lines)]
pub(super) fn apply_readable_evaluated_body(
    invocation: &ResolvedBindingInvocation,
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    observed: &mut Option<ModuleCommandBindings>,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    context: &mut InvocationBindingContext<'_>,
) -> bool {
    use tcl_registry::SemanticOperationId;
    use tcl_registry::frame_effect::{FrameArgLayout, FrameLevel};
    use tcl_registry::hooks::LoweringHookId;

    let arg_refs: Vec<&str> = invocation.arguments.iter().map(String::as_str).collect();
    let body_indices =
        registry.arg_indices_for_role(&invocation.command, &arg_refs, tcl_registry::ArgRole::Body);
    let body_interpreter = invocation.facts.body_interpreter.resolve_with(|index| {
        invocation
            .literal_arguments
            .get(index)
            .and_then(Option::as_deref)
    });
    let (sources, body_namespace, procedure_target) = match invocation.facts.operation {
        SemanticOperationId::StructuredLowering(LoweringHookId::Apply) => {
            let Some(argument_count) = invocation.exact_argument_count else {
                return false;
            };
            let Some(lambda) = invocation
                .literal_arguments
                .first()
                .and_then(Option::as_deref)
            else {
                return false;
            };
            let Some(dialect) = bindings.baseline.dialect else {
                return false;
            };
            let Some(grammar) = dialect.parameter_grammar() else {
                return false;
            };
            // This body replay models an ordinary C lambda activation. Jim
            // caller links need their own entered-frame receipt; accepted
            // formal/count syntax cannot supply that activation.
            if grammar != tcl_dialect::ParameterGrammar::Tcl {
                return false;
            }
            let Ok(elements) = dialect.word_values.split_list(lambda) else {
                // Tcl rejects a malformed lambda before entering its body.
                return true;
            };
            if !(2..=3).contains(&elements.len()) {
                return true;
            }
            let Ok(parameters) = tcl_syntax::formal_params::parse_formal_parameters_in(
                elements[0].as_ref(),
                grammar,
            ) else {
                return true;
            };
            let count = tcl_syntax::formal_params::formal_argument_count_shape(
                parameters.iter().map(|parameter| {
                    (
                        tcl_syntax::formal_params::is_variadic_formal_name(&parameter.name),
                        parameter.default.is_some(),
                    )
                }),
                grammar,
            );
            if !count.accepts(argument_count.saturating_sub(1)) {
                // Known arity errors occur before any body command executes.
                return true;
            }
            let body_namespace = match elements.get(2) {
                Some(name) if name.is_empty() => Some("::".to_owned()),
                Some(name) => qualified_namespace("::", name),
                None => Some("::".to_owned()),
            };
            let Some(body_namespace) = body_namespace else {
                return true;
            };
            (
                vec![elements[1].to_string()],
                Some(crate::ir::ExecutionNamespace::exact(body_namespace)),
                true,
            )
        }
        SemanticOperationId::StructuredLowering(LoweringHookId::NamespaceEval) => {
            let Some(argument_count) = invocation.exact_argument_count else {
                return false;
            };
            let Some(&first_body) = body_indices.first() else {
                return false;
            };
            let Some(target_index) = first_body.checked_sub(1) else {
                return false;
            };
            let Some(target) = invocation
                .literal_arguments
                .get(target_index)
                .and_then(Option::as_deref)
            else {
                return false;
            };
            let Some(source) =
                readable_script_argument(invocation, first_body, registry, bindings, namespace)
            else {
                return false;
            };
            if first_body + 1 != argument_count {
                return false;
            }
            let Some(body_namespace) = qualify_execution_name(namespace, target) else {
                // Tcl rejects an empty namespace before evaluating its body.
                return target.is_empty();
            };
            (
                vec![source],
                Some(crate::ir::ExecutionNamespace::exact(body_namespace)),
                false,
            )
        }
        _ => {
            let Some(argument_count) = invocation.exact_argument_count else {
                return false;
            };
            let (first_body, body_namespace) =
                match (invocation.facts.frame_effect, &body_interpreter) {
                    (Some(effect), _) => match effect.layout {
                        FrameArgLayout::ScriptInCurrentFrame => (0, Some(namespace.clone())),
                        FrameArgLayout::ScriptInSelectedFrame => {
                            let words = invocation
                                .literal_arguments
                                .iter()
                                .map(|word| {
                                    word.as_deref().map_or(
                                        tcl_registry::InvocationWord::Dynamic,
                                        tcl_registry::InvocationWord::Literal,
                                    )
                                })
                                .collect::<Vec<_>>();
                            let arguments = tcl_registry::InvocationWords::structured(
                                tcl_registry::InvocationWord::Literal(&invocation.command),
                                &words,
                            )
                            .with_profile(registry.profile())
                            .arguments();
                            let tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                                level_word_len,
                                level,
                            } = effect.resolve_arguments(arguments)
                            else {
                                return false;
                            };
                            let body = &arg_refs[level_word_len..];
                            let body_namespace = match level {
                                FrameLevel::Absolute(0) => {
                                    Some(crate::ir::ExecutionNamespace::exact("::"))
                                }
                                level if level.is_current_frame() => Some(namespace.clone()),
                                // The caller's defining namespace is unavailable in a
                                // per-procedure root. We can still prove a constructed
                                // script harmless to the command table below; any
                                // actual binding transition remains opaque.
                                _ => None,
                            };
                            (arg_refs.len().saturating_sub(body.len()), body_namespace)
                        }
                        FrameArgLayout::AliasPairs | FrameArgLayout::OpaqueCallerVars => {
                            return false;
                        }
                    },
                    // A body executed by a named child interpreter cannot directly
                    // update this module's command table. Still prove the readable
                    // script free of command-binding mutations before preserving
                    // the parent lattice: that remains safe if a child alias later
                    // re-enters a command in the parent interpreter.
                    (None, tcl_registry::InterpreterScope::Named(_)) => {
                        let Some(&first_body) = body_indices.first() else {
                            return false;
                        };
                        (first_body, None)
                    }
                    (
                        None,
                        tcl_registry::InterpreterScope::Current
                        | tcl_registry::InterpreterScope::Any,
                    ) => {
                        return false;
                    }
                };
            if first_body >= argument_count {
                return false;
            }
            let indices: Vec<usize> = if invocation
                .facts
                .traits
                .contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
            {
                // Multi-word concat is exact at runtime but reconstructing its
                // Tcl quoting here would create a second concat interpreter.
                if first_body + 1 != argument_count {
                    return false;
                }
                vec![first_body]
            } else if body_indices.is_empty() {
                (first_body..argument_count).collect()
            } else {
                body_indices
            };
            let Some(sources) = indices
                .into_iter()
                .map(|index| {
                    readable_script_argument(invocation, index, registry, bindings, namespace)
                })
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };
            (sources, body_namespace, false)
        }
    };

    let Some(body_namespace) = body_namespace else {
        return sources
            .iter()
            .all(|source| command_bindings_unchanged_by_script(source, registry, bindings));
    };

    let crate::ir::ExecutionNamespace::Exact(body_namespace) = body_namespace else {
        return sources
            .iter()
            .all(|source| command_bindings_unchanged_by_script(source, registry, bindings));
    };

    for source in sources {
        let execution_namespace = crate::ir::ExecutionNamespace::exact(&body_namespace);
        if let Some(script) = context.retained_roots.evaluated_body(
            &source,
            &execution_namespace,
            invocation.source_span,
        ) {
            let outcome = {
                let mut context = BindingWalkContext {
                    registry,
                    retained_roots: context.retained_roots,
                    timeline: None,
                    source_order_mode: context.source_order_mode,
                };
                collect_binding_states(&script, &mut context, bindings, &execution_namespace)
            };
            observe_binding_state(observed, &outcome.observed);
            *bindings = outcome.post;
            continue;
        }
        let frame = if procedure_target {
            crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: body_namespace.clone(),
                identity: format!("evaluated:{}", invocation.source_span.start()),
            }
        } else {
            crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
                namespace: body_namespace.clone(),
                identity: format!("evaluated:{}", invocation.source_span.start()),
            }
        };
        let outcome = interpret_binding_source(
            &source,
            0,
            &body_namespace,
            &frame,
            registry,
            bindings,
            context.retained_roots,
        );
        observe_binding_state(observed, &outcome.observed);
        *bindings = outcome.post;
    }
    true
}

/// Prove that a source-safe script cannot alter the closed command-binding
/// state, even when a frame shift leaves its execution namespace unknown.
///
/// Every namespace already represented by a procedure or command binding is
/// checked, plus the global namespace. A fresh sentinel namespace covers
/// transitions whose result depends only on qualification (for example, a
/// definition in a previously unseen caller namespace). The ordinary
/// transition walker remains the semantic owner: this helper does not name or
/// reinterpret any command.
fn command_bindings_unchanged_by_script(
    source: &str,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
) -> bool {
    let mut namespaces = BTreeSet::from([
        "::".to_owned(),
        "::__tcl_compiler_unknown_caller".to_owned(),
    ]);
    for qname in bindings
        .bindings
        .keys()
        .filter_map(|key| key.authored_spelling())
        .chain(bindings.procedure_bodies.iter().map(String::as_str))
    {
        let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
        namespaces.insert(if holder.is_empty() {
            "::".to_owned()
        } else {
            holder.to_owned()
        });
    }
    namespaces.into_iter().all(|namespace| {
        let mut retained_roots = RetainedBindingRoots::default();
        let frame = crate::var_resolve::VariableExecutionFrame::Namespace(namespace.clone());
        let outcome = interpret_binding_source(
            source,
            0,
            &namespace,
            &frame,
            registry,
            bindings,
            &mut retained_roots,
        );
        outcome.post.same_state(bindings) && outcome.observed.same_state(bindings)
    })
}

fn readable_script_argument(
    invocation: &ResolvedBindingInvocation,
    index: usize,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
) -> Option<String> {
    readable_script_argument_with_optional_metadata(
        invocation, index, registry, bindings, namespace, None,
    )
}

pub(super) fn readable_script_argument_with_optional_metadata(
    invocation: &ResolvedBindingInvocation,
    index: usize,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<String> {
    invocation
        .literal_arguments
        .get(index)
        .and_then(Option::as_deref)
        .map(str::to_owned)
        .or_else(|| {
            let source = invocation.arguments.get(index)?;
            let crate::ir_helpers::ExecutionNamespace::Exact(namespace) = namespace else {
                return None;
            };
            constructed_script_words_with_optional_metadata(
                source, registry, bindings, namespace, metadata,
            )
            .map(tcl_syntax::list::join_list)
        })
}

pub(in super::super) fn qualify_execution_name(
    namespace: &crate::ir_helpers::ExecutionNamespace,
    name: &str,
) -> Option<String> {
    if tcl_syntax::naming::is_absolute(name) {
        return Some(tcl_syntax::naming::canonical_written_command(name));
    }
    // An authored source context is the same symbolic namespace as an exact
    // one; native and allocated identities keep refusing a display join.
    let (crate::ir_helpers::ExecutionNamespace::Exact(namespace)
    | crate::ir_helpers::ExecutionNamespace::SourceContext(SourceNamespaceKey::Authored(
        namespace,
    ))) = namespace
    else {
        return None;
    };
    Some(tcl_syntax::naming::qualify(namespace, name))
}

pub(super) fn procedure_publication_key(
    namespace: &crate::ir_helpers::ExecutionNamespace,
    name: &str,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Option<String> {
    let constructed = qualify_execution_name(namespace, name)?;
    tcl_registry::native_procedure::published_procedure_key(constructed, dialect)
}

fn qualified_namespace(parent: &str, child: &str) -> Option<String> {
    if child.is_empty() {
        return None;
    }
    // A namespace name does not have the empty-command-tail meaning of a
    // trailing separator. Normalise the written namespace through the shared
    // syntax owner before joining its relative suffix to the constructed key.
    let canonical = nqn(child);
    if tcl_syntax::naming::is_absolute(child) {
        Some(canonical)
    } else {
        Some(tcl_syntax::naming::qualify(
            parent,
            tcl_syntax::naming::unroot_rooted_key(&canonical).unwrap_or(&canonical),
        ))
    }
}

/// Source-safe constructed argv under actual supplied availability.
/// The existing may-binding proof remains required independently; no original
/// evaluated builder, Native lookup, frame or result object is invented.
pub(crate) fn constructed_script_words_with_metadata_context(
    word: &str,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &(impl NamespaceKeyQuery + ?Sized),
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<Vec<String>> {
    let metadata = metadata.filter(|metadata| metadata.matches_registry(registry))?;
    constructed_script_words_with_optional_metadata(
        word,
        registry,
        bindings,
        namespace,
        Some(metadata),
    )
}

pub(in super::super) fn constructed_script_words_with_optional_metadata(
    word: &str,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &(impl NamespaceKeyQuery + ?Sized),
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<Vec<String>> {
    let inner = word
        .strip_prefix('[')
        .and_then(|word| word.strip_suffix(']'))?;
    let config = match metadata {
        Some(metadata) => match metadata.source_analysis_input() {
            Some(input) => input.lexer_config(),
            None => tcl_lexer::LexerConfig::from_grammar(
                tcl_registry::InvocationDialect::of_point(metadata.context().environment.point()?)
                    .lexer_grammar,
            ),
        },
        None => bindings.invocation_dialect().map_or_else(
            || tcl_lexer::LexerConfig::for_profile(registry.profile()),
            |dialect| tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        ),
    };
    let commands =
        crate::segmenter::segment_commands_with_offset_and_config(inner, 0, config.nested());
    let [command] = commands.as_slice() else {
        return None;
    };
    if command.is_partial {
        return None;
    }
    let source_map = tcl_lexer::SourceMap::new(inner);
    let tokens = crate::ir::CommandTokens::from_segmented(&source_map, config, command);
    let words: Vec<String> = tokens
        .words()
        .iter()
        .map(|word| {
            match crate::registry_invocation::effective_invocation_word(
                word,
                config.escapes,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
            ) {
                crate::registry_invocation::EffectiveInvocationWord::Literal(value) => Some(value),
                crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(_)
                | crate::registry_invocation::EffectiveInvocationWord::Dynamic
                | crate::registry_invocation::EffectiveInvocationWord::ArrayElementName {
                    ..
                }
                | crate::registry_invocation::EffectiveInvocationWord::Expanded
                | crate::registry_invocation::EffectiveInvocationWord::KnownExpansion(_)
                | crate::registry_invocation::EffectiveInvocationWord::KnownByteExpansion(_)
                | crate::registry_invocation::EffectiveInvocationWord::Opaque => None,
            }
        })
        .collect::<Option<_>>()?;
    let (builder, operands) = words.split_first()?;
    // A dynamic mutation may have replaced the constructor even when its
    // literal spelling has no individually enumerable transition. Requiring
    // whole-module provenance keeps runtime replay and static effect summaries
    // on the same conservative rule.
    if bindings.target_may_be_unknown(builder, namespace) {
        return None;
    }

    let mut projections = BTreeSet::new();
    let targets = bindings.targets(builder, namespace);
    if targets.is_empty() {
        return None;
    }
    for target in targets {
        if !target.registry_backed {
            return None;
        }
        let mut arguments = target
            .prepended
            .iter()
            .map(|word| word.as_registry_word().literal().map(str::to_owned))
            .collect::<Option<Vec<_>>>()?;
        arguments.extend(operands.iter().cloned());
        let argument_refs: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let invocation = match metadata {
            Some(metadata) => tcl_registry::model::assembly::resolve_invocation_in_context(
                registry,
                Some(metadata.context()),
                &target.command,
                &argument_refs,
            ),
            None => registry.resolve_invocation(
                &target.command,
                &argument_refs,
                registry.own_surface_query(),
            ),
        }?;
        let Some(tcl_registry::ReturnElements::ListOfArgs { from }) =
            invocation.semantics.return_elements
        else {
            return None;
        };
        let projected = arguments.get(usize::from(from)..)?.to_vec();
        if projected.first().is_none_or(String::is_empty) {
            return None;
        }
        projections.insert(projected);
    }
    let mut projections = projections.into_iter();
    let unique = projections.next()?;
    projections.next().is_none().then_some(unique)
}
