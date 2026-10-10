// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional receiver-class candidates from one retained source invocation.

use super::{InstanceClassState, TaintSourceContext};
use crate::ir::{CommandTokens, Statement};
use crate::registry_invocation::{InvocationMetadataContext, ResolvedStatementInvocation};
use std::collections::HashSet;
use tcl_registry::{CommandRegistry, Traits};

/// Original source operands and selected availability supply this candidate
/// map only. No created command, live object, successful handler, variable
/// storage, method implementation or taint sanitisation follows from it.
pub(super) struct InstanceOperation {
    transitions: tcl_registry::StateTransitions,
    publication: Option<(String, Option<String>)>,
    result_class: Option<String>,
    teardown: Option<Vec<String>>,
}

impl InstanceOperation {
    fn apply(&self, state: &mut InstanceClassState) {
        if self
            .transitions
            .widens(tcl_registry::StateTransitionDomain::CommandBindings)
        {
            state.clear();
        }
        for transition in self.transitions.command_bindings() {
            match transition {
                tcl_registry::CommandBindingTransition::Move { from, to } => {
                    let (Some(old), Some(new)) = (from.literal(), to.literal()) else {
                        state.clear();
                        continue;
                    };
                    let class = state.remove(old);
                    // The destination replaces any previous candidate even if
                    // the moved source had no conditional class metadata.
                    state.remove(new);
                    if !new.is_empty()
                        && let Some(class) = class
                    {
                        state.insert(new.to_owned(), class);
                    }
                }
                tcl_registry::CommandBindingTransition::Delete { interpreter, name } => {
                    if interpreter
                        .as_ref()
                        .is_some_and(|realm| realm.literal() != Some(""))
                    {
                        state.clear();
                    } else if let Some(name) = name.literal() {
                        state.remove(name);
                    } else {
                        state.clear();
                    }
                }
                tcl_registry::CommandBindingTransition::Define { name, .. } => {
                    if let Some(name) = name.literal() {
                        state.remove(name);
                    } else {
                        state.clear();
                    }
                }
                tcl_registry::CommandBindingTransition::Alias {
                    source_interpreter,
                    alias,
                    ..
                } => {
                    if source_interpreter.literal() == Some("")
                        && let Some(name) = alias.literal()
                    {
                        state.remove(name);
                    } else {
                        state.clear();
                    }
                }
                tcl_registry::CommandBindingTransition::Unknown { .. } => state.clear(),
            }
        }
        if let Some(targets) = &self.teardown {
            for target in targets {
                state.retain(|name, _| {
                    !tcl_registry::tk_geometry::widget_path_is_within(name, target)
                });
            }
        }
        if let Some((name, class)) = &self.publication {
            state.remove(name);
            if let Some(class) = class {
                state.insert(name.clone(), HashSet::from([class.clone()]));
            }
        }
    }
}

fn original_invocation(
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let context = source.metadata_context()?;
    let input = context.source_analysis_input()?;
    if source
        .module_owner()
        .is_some_and(|owner| !owner.owns_original_tokens(tokens))
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    if binding
        .original_lexer_config_for_tokens(tokens)?
        .nested()
        .normalized()
        != source.lexer_config().nested().normalized()
        || source.lexer_config().nested().normalized() != input.lexer_config().nested().normalized()
    {
        return None;
    }
    crate::registry_invocation::original_callback_invocation_with_metadata_context(
        registry, context, tokens,
    )
}

fn source_schema<T>(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    invocation: &ResolvedStatementInvocation,
    apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
) -> Option<T> {
    let binding = tokens.source_binding.as_ref()?;
    let realm = binding.invocation_realm().or_else(|| {
        binding
            .declaration_operand_layout_advice(tokens)
            .map(|advice| advice.realm())
    })?;
    invocation.with_metadata_schema(registry, context, realm, apply)
}

fn selected_operation(
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
    tokens: &CommandTokens,
    invocation: &ResolvedStatementInvocation,
) -> Option<InstanceOperation> {
    if invocation.facts.arity_accepts_frozen_arguments() != Some(true) {
        return None;
    }
    source_schema(
        registry,
        source.metadata_context()?,
        tokens,
        invocation,
        |schema| {
            let descriptor = schema.authored_source_descriptors().command;
            let publication =
                schema
                    .authored_source_command_publication()
                    .and_then(|publication| {
                        let name = invocation.argument_literal(publication.argument)?;
                        if name.is_empty() {
                            return None;
                        }
                        let class = match publication.kind {
                            tcl_registry::AuthoredSourceCommandPublicationKind::Command => None,
                            tcl_registry::AuthoredSourceCommandPublicationKind::Instance {
                                ..
                            } => descriptor
                                .object_class
                                .map(|class| class.class_name.to_owned()),
                        };
                        Some((name, class))
                    });
            let result_class = schema
                .authored_source_callable_factory_class()
                .map(str::to_owned);
            let teardown = if schema
                .semantics
                .traits
                .contains(Traits::FIRE_AND_FORGET_TEARDOWN)
                && descriptor.required_package == Some("Tk")
            {
                Some(
                    (0..invocation.arguments.len())
                        .map(|index| invocation.argument_literal(index))
                        .collect::<Option<Vec<_>>>()?,
                )
            } else {
                None
            };
            Some(InstanceOperation {
                transitions: schema.state_transitions(),
                publication,
                result_class,
                teardown,
            })
        },
    )
}

fn bound_factory_value(
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
    tokens: &CommandTokens,
    invocation: &ResolvedStatementInvocation,
) -> Option<(String, String)> {
    use tcl_registry::handle_binding::{HandleClassSource, HandleName};
    let binding = source_schema(
        registry,
        source.metadata_context()?,
        tokens,
        invocation,
        |schema| {
            schema
                .authored_source_descriptors()
                .command
                .binds_handle
                .copied()
        },
    )?;
    if let Some(keyword) = binding.keyword
        && invocation
            .argument_literal(usize::from(keyword.at))
            .as_deref()
            != Some(keyword.word)
    {
        return None;
    }
    let name = match binding.name_from {
        HandleName::Word(index) => invocation.argument_literal(usize::from(index))?,
        HandleName::Implicit(name) => name.to_owned(),
    };
    let HandleClassSource::ConstructionValue(value) = binding.class_from else {
        return None;
    };
    let index = invocation.effective.written_argument(usize::from(value))?;
    let word = tokens.words().get(index.checked_add(1)?)?;
    let mut nested = crate::word_subst::whole_word_command_tokens(word, source.lexer_config())?;
    nested.inherit_nested_bindings(tokens);
    let selected = original_invocation(registry, source, &nested)?;
    let class = selected_operation(registry, source, &nested, &selected)?.result_class?;
    let (root, element) = crate::naming::split_array_name_braced(&name, true);
    (element.is_none() && !root.is_empty()).then(|| (root.to_owned(), class))
}

// Each lexical child keeps its own original lookup horizon. Selected expression
// roles come from the same metadata owner; braced script operands remain data.
fn apply_original_substitutions(
    state: &mut InstanceClassState,
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
    tokens: &CommandTokens,
) {
    let Some(metadata) = source.metadata_context() else {
        state.clear();
        return;
    };
    let Some(calls) = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
        tokens,
        source.lexer_config(),
        registry,
        metadata,
    ) else {
        state.clear();
        return;
    };
    for call in &calls {
        let conditional = std::iter::once(tokens)
            .chain(calls.iter().filter_map(|child| child.tokens.as_ref()))
            .flat_map(CommandTokens::words)
            .any(|word| {
                matches!(word, crate::ir::WordExpr::BracedLiteral { source, .. }
                    if source.span.start() < call.span.start()
                        && source.span.end() > call.span.end())
            });
        let Some(nested) = call.tokens.as_ref() else {
            state.clear();
            continue;
        };
        let Some(invocation) = original_invocation(registry, source, nested) else {
            state.clear();
            continue;
        };
        let Some(operation) = selected_operation(registry, source, nested, &invocation) else {
            state.clear();
            continue;
        };
        if conditional {
            let mut candidate = state.clone();
            operation.apply(&mut candidate);
            state.retain(|name, class| candidate.get(name) == Some(class));
        } else {
            operation.apply(state);
        }
    }
}

/// Apply the same retained source command horizon to an interprocedural map.
/// This excludes variable-handle assignments: procedure locals cannot borrow
/// top-level variable bindings merely because their written names coincide.
pub(crate) fn transfer_instance_command(
    state: &mut InstanceClassState,
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
    tokens: Option<&CommandTokens>,
) {
    let Some(tokens) = tokens else {
        state.clear();
        return;
    };
    apply_original_substitutions(state, registry, source, tokens);
    let Some(invocation) = original_invocation(registry, source, tokens) else {
        state.clear();
        return;
    };
    let Some(operation) = selected_operation(registry, source, tokens, &invocation) else {
        state.clear();
        return;
    };
    operation.apply(state);
}

pub(super) fn transfer_instance_statement(
    state: &mut InstanceClassState,
    statement: &Statement,
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
    tokens: Option<&CommandTokens>,
) {
    if statement.has_opaque_native_accesses() {
        state.clear();
        return;
    }
    if source.allows_nominal_metadata() {
        standalone_transfer_instance_statement(state, statement, registry);
        return;
    }
    if source.metadata_context().is_none() {
        state.clear();
        return;
    }
    match statement {
        Statement::Call { .. } | Statement::AssignValue { .. } => {
            // A consumed setter still owns the exact original source vector;
            // no source operation is reconstructed from an IR node label.
            let Some(tokens) = tokens else {
                state.clear();
                return;
            };
            apply_original_substitutions(state, registry, source, tokens);
            let Some(invocation) = original_invocation(registry, source, tokens) else {
                state.clear();
                return;
            };
            let Some(operation) = selected_operation(registry, source, tokens, &invocation) else {
                state.clear();
                return;
            };
            operation.apply(state);
            match statement {
                Statement::Call { defs, .. } => {
                    for name in defs {
                        state.remove(crate::naming::split_array_name_braced(name, true).0);
                    }
                }
                Statement::AssignValue { name, .. } => {
                    state.remove(crate::naming::split_array_name_braced(name, true).0);
                }
                _ => unreachable!(),
            }
            if let Some((name, class)) = bound_factory_value(registry, source, tokens, &invocation)
            {
                state.insert(name, HashSet::from([class]));
            }
        }
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. } => {
            if let Some(tokens) = tokens {
                apply_original_substitutions(state, registry, source, tokens);
            } else if !matches!(statement, Statement::AssignConst { .. }) {
                state.clear();
            }
            state.remove(crate::naming::split_array_name_braced(name, true).0);
        }
        Statement::Barrier { .. } | Statement::NativeCall { .. } => state.clear(),
        _ => {}
    }
}

fn standalone_factory_class<'a>(
    registry: &'a CommandRegistry,
    head: &str,
    args: &[String],
) -> Option<&'a str> {
    if let Some(method) = args.first()
        && registry
            .exported_manufacturer_method(head, method)
            .is_some()
    {
        return registry.object_class(head).map(|class| class.class_name);
    }
    registry
        .get(head)
        .filter(|spec| spec.creates_instance_at.is_some())
        .and_then(|spec| spec.object_class)
        .map(|class| class.class_name)
}

fn literal_receiver(name: &str) -> Option<&str> {
    (!name.is_empty() && !name.starts_with(['$', '[', '{'])).then_some(name)
}

fn standalone_transfer_instance_statement(
    state: &mut InstanceClassState,
    statement: &Statement,
    registry: &CommandRegistry,
) {
    if statement.has_opaque_native_accesses() {
        state.clear();
        return;
    }
    match statement {
        Statement::Call {
            command,
            args,
            defs,
            ..
        } => {
            standalone_transfer_instance_lifecycle(state, command, args, registry);
            // A registry-known write to a handle variable invalidates the old
            // type unless this very statement installs a fresh factory fact.
            for name in defs {
                state.remove(crate::naming::normalise_var_name(name));
            }
            if let Some(spec) = registry.get(command)
                && let Some(index) = spec.creates_instance_at
                && let Some(name) = args.get(usize::from(index))
                && let Some(name) = literal_receiver(name)
            {
                // Every naming factory replaces the command at this literal
                // receiver, even when its registry row does not expose an
                // object class. In that case the sound reaching fact is
                // "untyped", not the previous constructor's stale class.
                state.remove(name);
                if let Some(class) = spec.object_class {
                    state.insert(
                        name.to_owned(),
                        HashSet::from([class.class_name.to_owned()]),
                    );
                }
            }
        }
        Statement::AssignValue { name, value, .. } => {
            let name = crate::naming::normalise_var_name(name);
            state.remove(name);
            if let Some((head, args)) = crate::value_shapes::parse_command_substitution_with_config(
                value.trim(),
                tcl_lexer::LexerConfig::for_profile(registry.profile()),
            ) && let Some(class) = standalone_factory_class(registry, &head, &args)
                && let Some(name) = literal_receiver(name)
            {
                state.insert(name.to_owned(), HashSet::from([class.to_owned()]));
            }
        }
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. } => {
            state.remove(crate::naming::normalise_var_name(name));
        }
        _ => {}
    }
}

/// Apply registry-declared command/object lifecycle changes to the local
/// receiver-class map.  This is deliberately driven by the registry's
/// command-table and teardown descriptors rather than by command-name
/// switches: a rename moves the class fact, an alias/dynamic mutation clears
/// it, and a Tk teardown removes the target window and its descendants.
fn standalone_transfer_instance_lifecycle(
    state: &mut InstanceClassState,
    command: &str,
    args: &[String],
    registry: &CommandRegistry,
) {
    let transitions = crate::alias::command_table_transitions(registry, command, args);
    if transitions.touches_command_bindings() {
        for transition in transitions.command_bindings() {
            match transition {
                tcl_registry::CommandBindingTransition::Move { from, to } => {
                    let (Some(old), Some(new)) = (from.literal(), to.literal()) else {
                        state.clear();
                        return;
                    };
                    let Some(old) = literal_receiver(old) else {
                        state.clear();
                        return;
                    };
                    if new.starts_with(['$', '[', '{']) {
                        state.clear();
                        return;
                    }
                    let class = state.remove(old);
                    if !new.is_empty()
                        && let Some(class) = class
                    {
                        state.insert(new.to_owned(), class);
                    }
                }
                tcl_registry::CommandBindingTransition::Delete { name, .. } => {
                    // The moved-away half of `rename OLD {}`: the receiver
                    // identity at that name is gone.
                    let Some(old) = name.literal().and_then(literal_receiver) else {
                        state.clear();
                        return;
                    };
                    state.remove(old);
                }
                tcl_registry::CommandBindingTransition::Alias { .. }
                | tcl_registry::CommandBindingTransition::Unknown { .. } => {
                    // An alias may target or replace any command, including a
                    // registry-modelled instance command.  Without a precise
                    // target proof all receiver identities become unknown, and
                    // an unknown mutation proves nothing at all.
                    state.clear();
                    return;
                }
                // A definition binds a new name without disturbing an
                // existing receiver identity.
                tcl_registry::CommandBindingTransition::Define { .. } => {}
            }
        }
        return;
    }

    // Tk's `destroy` is a registry-declared fire-and-forget teardown.  Use the
    // package declaration to distinguish it from unrelated teardown commands
    // such as `unset` and `after cancel`, while keeping the consumer generic.
    let Some(spec) = registry.get(command) else {
        return;
    };
    if !spec.traits.contains(Traits::FIRE_AND_FORGET_TEARDOWN)
        || spec.required_package != Some("Tk")
    {
        return;
    }
    for target in args {
        let Some(target) = literal_receiver(target) else {
            state.clear();
            return;
        };
        state.retain(|name, _| !tcl_registry::tk_geometry::widget_path_is_within(name, target));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::ResolvedAnalysisInput;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use std::sync::Arc;
    use tcl_dialect::DialectProfile;
    use tcl_lexer::LexerConfig;
    use tcl_registry::model::ContextRegistry;

    fn receiver_context() -> Arc<ContextRegistry> {
        let base = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let profile = DialectProfile::find("tcl8.6").unwrap();
        let mut catalogue = base.commands().project_for_profile(profile);
        let mut factory = catalogue.get("ttk::entry").unwrap().clone();
        let class = factory.object_class.unwrap();
        factory.name = "source_factory";
        factory.required_package = None;
        factory.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        factory.object_class = Some(Box::leak(Box::new(tcl_registry::ObjectClassSpec {
            class_name: "source_factory",
            instance_methods: class.instance_methods,
            superclasses: class.superclasses,
            allow_unknown_methods: class.allow_unknown_methods,
            method_prefix_matching: class.method_prefix_matching,
        })));
        catalogue.insert(factory);
        Arc::new(base.with_command_store(Arc::new(catalogue)))
    }

    fn logical_unit(source: &str, context: &Arc<ContextRegistry>) -> CompilationUnit {
        let profile = DialectProfile::find("tcl").unwrap();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
    }

    fn reaching(unit: &CompilationUnit, registry: &CommandRegistry) -> InstanceClassState {
        let source =
            TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
        let facts = crate::taint::instance_classes_for_function(
            &unit.top_level.cfg,
            registry,
            None,
            false,
            source,
        );
        let last = unit.ir_module.top_level.statements.last().unwrap();
        facts
            .at
            .get(&last.span().start())
            .cloned()
            .unwrap_or_default()
    }

    #[test]
    fn original_receiver_factories_keep_alias_origins_and_literal_names() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Authored conditional receiver candidates only. The source model
        // does not execute the factory or install an object command.
        let context = receiver_context();
        let registry = context.commands();
        for source in [
            "source_factory {.名前$}; puts MARK",
            "interp alias {} make {} source_factory {.名前$}; make; puts MARK",
            "rename source_factory moved; moved {.名前$}; puts MARK",
        ] {
            let unit = logical_unit(source, &context);
            assert_eq!(
                reaching(&unit, registry).get(".名前$"),
                Some(&HashSet::from(["source_factory".to_owned()])),
                "{source}"
            );
        }
        let unit = logical_unit(
            "set {$handle} [source_factory {.名前$}]; puts MARK",
            &context,
        );
        let facts = reaching(&unit, registry);
        assert_eq!(
            facts.get("$handle"),
            Some(&HashSet::from(["source_factory".to_owned()]))
        );
        assert!(
            facts.contains_key(".名前$"),
            "nested original factory publication"
        );
        assert!(
            !facts.contains_key("handle"),
            "literal binding name is not a variable-reference sigil"
        );
        let original = unit
            .ir_module
            .top_level
            .retained_source_tokens_for_statement(&unit.ir_module.top_level.statements[0])
            .unwrap();
        assert!(
            original
                .source_binding
                .as_ref()
                .unwrap()
                .proved_construction_result(registry)
                .is_none(),
            "conditional class metadata cannot supply an actual result allocation"
        );
    }

    #[test]
    fn original_receiver_lifecycle_obeys_moves_deletes_and_factory_shadows() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // The exact original source schema chooses lifecycle operands. These
        // are source-map invalidations, not successful Native table changes.
        let context = receiver_context();
        let registry = context.commands();
        let unit = logical_unit(
            "source_factory {.名前$}; rename {.名前$} {.moved$}; puts MARK",
            &context,
        );
        let facts = reaching(&unit, registry);
        assert!(!facts.contains_key(".名前$"));
        assert_eq!(
            facts.get(".moved$"),
            Some(&HashSet::from(["source_factory".to_owned()]))
        );
        let moved = logical_unit(
            "source_factory {.名前$}; interp alias {} move {} rename {.名前$}; move {.moved$}; puts MARK",
            &context,
        );
        assert!(reaching(&moved, registry).contains_key(".moved$"));
        for source in [
            "source_factory .w; rename .w {}; puts MARK",
            "source_factory .w; proc source_factory args {return CUSTOM}; source_factory .w; puts MARK",
            "proc source_factory args {return CUSTOM}; source_factory .w; puts MARK",
            "source_factory .w; interp alias {} source_factory {} puts; source_factory .w; puts MARK",
            "source_factory .w; puts [rename .w {}]; puts MARK",
        ] {
            assert!(
                reaching(&logical_unit(source, &context), registry).is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_receiver_maps_require_actual_availability_and_matching_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Availability and input correspondence are separate from the
        // positive Logical naming owner. No physical receiver is observed.
        let current = receiver_context();
        let registry = current.commands();
        let source = "source_factory .w; puts MARK";
        let mut unit = logical_unit(source, &current);
        assert!(reaching(&unit, registry).contains_key(".w"));
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(registry)),
        );
        let older_unit = logical_unit(source, &older);
        assert!(
            TaintSourceContext::for_module_function(
                registry,
                &older_unit.ir_module,
                &older_unit.top_level
            )
            .metadata_context()
            .is_some()
        );
        assert!(
            reaching(&older_unit, registry).is_empty(),
            "same command store, independently unavailable factory"
        );
        let input = unit.ir_module.source_metadata_input.clone().unwrap();
        let changed_availability = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::clone(&older),
            input.lexer_config(),
        );
        unit.ir_module.source_metadata_input = Some(changed_availability.clone());
        unit.top_level.source_metadata_input = Some(changed_availability);
        assert!(
            reaching(&unit, registry).is_empty(),
            "changed metadata is not the original Module interpretation"
        );
        unit.ir_module.source_metadata_input = Some(input.clone());
        unit.top_level.source_metadata_input = Some(input.clone());
        let owner = unit.ir_module.retained_source_bindings.take();
        assert!(
            reaching(&unit, registry).is_empty(),
            "missing Module interpretation"
        );
        unit.ir_module.retained_source_bindings = owner;
        let image = unit.ir_module.source.clone();
        unit.ir_module.source =
            tcl_lexer::SourceImage::document("source_factory .other; puts MARK");
        assert!(
            reaching(&unit, registry).is_empty(),
            "stale original Module source"
        );
        unit.ir_module.source = image;
        unit.ir_module.source_metadata_input = None;
        assert!(reaching(&unit, registry).is_empty(), "missing Module input");
        unit.ir_module.source_metadata_input = Some(input.clone());
        unit.top_level.source_metadata_input = None;
        assert!(
            reaching(&unit, registry).is_empty(),
            "missing FunctionUnit input"
        );
        unit.top_level.source_metadata_input = Some(input.clone());
        unit.ir_module.lexer_config.strict_quoting = !input.lexer_config().strict_quoting;
        assert!(reaching(&unit, registry).is_empty(), "stale Module grammar");
        unit.ir_module.lexer_config = input.lexer_config();
        unit.ir_module.dialect_profile = Some(DialectProfile::find("tcl9.0").unwrap());
        assert!(reaching(&unit, registry).is_empty(), "stale Module profile");
        unit.ir_module.dialect_profile = Some(input.unit_profile());
        let foreign = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
            input.lexer_config(),
        );
        unit.ir_module.source_metadata_input = Some(foreign.clone());
        unit.top_level.source_metadata_input = Some(foreign);
        assert!(
            reaching(&unit, registry).is_empty(),
            "foreign command generation"
        );
        unit.ir_module.source_metadata_input = Some(input.clone());
        unit.top_level.source_metadata_input = Some(input);
        assert!(reaching(&unit, registry).contains_key(".w"));
    }

    #[test]
    fn original_global_receiver_maps_keep_module_input_and_source_order() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Callback-only procedures receive conditional source setup facts,
        // without a callback frame, entered constructor or current object.
        let context = receiver_context();
        let registry = context.commands();
        let mut unit = logical_unit("source_factory .w; proc callback {} {puts MARK}", &context);
        assert!(unit.ir_module.procedures.contains_key("::callback"));
        let classes = crate::interprocedural::global_instance_classes(&unit.ir_module, registry);
        assert_eq!(
            classes["::callback"].get(".w"),
            Some(&HashSet::from(["source_factory".to_owned()]))
        );
        let analysis = crate::interprocedural::InterproceduralAnalysis {
            global_instance_classes: classes,
            global_instance_class_input: unit.ir_module.source_metadata_input.clone(),
            global_instance_class_owner: unit.ir_module.retained_source_bindings.clone(),
            ..crate::interprocedural::InterproceduralAnalysis::default()
        };
        let function = unit.procedures.get("::callback").unwrap();
        let facts = crate::taint::instance_classes_for_function(
            &function.cfg,
            registry,
            Some(&analysis),
            true,
            TaintSourceContext::for_module_function(registry, &unit.ir_module, function),
        );
        assert!(
            facts.at.values().any(|at| at.contains_key(".w")),
            "a function from the same retained Module imports its setup candidates"
        );
        let without_module = crate::taint::instance_classes_for_function(
            &function.cfg,
            registry,
            Some(&analysis),
            true,
            TaintSourceContext::for_function(registry, function),
        );
        assert!(
            without_module.at.values().all(|at| !at.contains_key(".w")),
            "function input equality alone cannot import another producer's map"
        );
        let other = logical_unit(
            "source_factory .other; proc callback {} {puts MARK}",
            &context,
        );
        let other_function = other.procedures.get("::callback").unwrap();
        let mismatched = crate::taint::instance_classes_for_function(
            &other_function.cfg,
            registry,
            Some(&analysis),
            true,
            TaintSourceContext::for_module_function(registry, &other.ir_module, other_function),
        );
        assert!(
            mismatched.at.values().all(|at| !at.contains_key(".w")),
            "equal availability/grammar cannot borrow a different Module source world"
        );
        unit.ir_module.source_metadata_input = None;
        assert!(crate::interprocedural::global_instance_classes(&unit.ir_module, registry)["::callback"].is_empty());
        let unit = logical_unit(
            "source_factory .w; if {$condition} {rename .w {}}; source_factory .w; proc callback {} {puts MARK}",
            &context,
        );
        let classes = crate::interprocedural::global_instance_classes(&unit.ir_module, registry);
        assert!(
            classes["::callback"].contains_key(".w"),
            "later direct source factory restores its conditional candidate"
        );
        for source in [
            "source_factory .w; {*}$unknown; proc callback {} {puts MARK}",
            "source_factory .w; if {[rename .w {}]} {}; proc callback {} {puts MARK}",
            "source_factory .w; puts [rename .w {}]; proc callback {} {puts MARK}",
        ] {
            let unit = logical_unit(source, &context);
            assert!(crate::interprocedural::global_instance_classes(&unit.ir_module, registry)
                ["::callback"].is_empty(), "{source}");
        }
    }
}
