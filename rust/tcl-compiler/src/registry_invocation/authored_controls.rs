// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional source grammar retains its original header or body scope.

use super::{CommandRegistry, CommandTokens, OriginalAuthoredSourceBodyScope};
use std::sync::Arc;

pub(crate) struct AuthoredSourceInvocation {
    owner: AuthoredSourceOwner,
    descriptor: tcl_registry::CanonicalCommandDescriptorKey<'static>,
    hook: tcl_registry::hooks::LoweringHookId,
    roles: Vec<(usize, tcl_registry::ArgRole)>,
    traits: tcl_registry::Traits,
    conditional_loop: bool,
}

enum AuthoredSourceOwner {
    Header(Box<super::authored_source_structure::AuthoredSourceGrammar>),
    Body(Arc<OriginalAuthoredSourceBodyScope>, Box<CommandTokens>),
}

pub(crate) fn original_authored_control_invocation(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<super::SourceStructuredInvocation> {
    let binding = tokens.source_binding.as_ref()?;
    let owner = if binding.has_original_authored_source_body_name_role_owner() {
        let scope = binding.original_authored_source_body_scope_for_name_roles(tokens, registry)?;
        AuthoredSourceOwner::Body(Arc::new(scope.clone()), Box::new(tokens.clone()))
    } else {
        AuthoredSourceOwner::Header(Box::new(
            super::original_authored_source_structure(tokens, registry)?
                .into_closed_source_grammar()?,
        ))
    };
    let expansion = match &owner {
        AuthoredSourceOwner::Header(grammar) => grammar.has_original_word_expansion(),
        AuthoredSourceOwner::Body(scope, tokens) => scope
            .original_words(tokens, registry)?
            .iter()
            .any(|word| word.group().expand),
    };
    if expansion {
        return None;
    }
    let (descriptor, hook, roles, traits, conditional_loop) =
        owner.with_schema(registry, |schema| {
            use tcl_registry::hooks::LoweringHookId;
            let hook = schema.facts().lowering_hook?;
            if !matches!(
                hook,
                LoweringHookId::If
                    | LoweringHookId::While
                    | LoweringHookId::For
                    | LoweringHookId::Foreach
                    | LoweringHookId::Lmap
                    | LoweringHookId::Catch
                    | LoweringHookId::Try
                    | LoweringHookId::Switch
            ) || schema.facts().arity_accepts_frozen_arguments() != Some(true)
                || schema.semantics.frame_effect.is_some()
                || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
                || schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current
            {
                return None;
            }
            let (roles, complete) = schema.authored_source_argument_roles();
            if !complete {
                return None;
            }
            let roles = roles
                .into_iter()
                .map(|(arg, role)| (schema.facts().argument_offset + usize::from(arg), role))
                .collect::<Vec<_>>();
            for &(argument, role) in &roles {
                if role == tcl_registry::ArgRole::Body
                    && schema.authored_source_script_timing_at(argument)?
                        != tcl_registry::ScriptTiming::SameInvocation
                {
                    return None;
                }
            }
            let conditional = schema.facts().repeated_args.iter().any(|layout| {
                layout.conditional_binding && layout.role == tcl_registry::ArgRole::LoopVarList
            });
            Some((
                schema.descriptor_key(),
                hook,
                roles,
                schema.semantics.traits,
                conditional,
            ))
        })?;
    for &(argument, role) in &roles {
        if role == tcl_registry::ArgRole::Body {
            let word = owner.original_argument_word(argument)?;
            super::original_written_script_body_span(&word)?;
            owner.argument_literal(argument)?;
        }
    }
    Some(super::SourceStructuredInvocation::Authored(Box::new(
        AuthoredSourceInvocation {
            owner,
            descriptor,
            hook,
            roles,
            traits,
            conditional_loop,
        },
    )))
}

fn authored_scalar_context_is_local(
    variables: &crate::var_resolve::ResolveContext,
    scope: &OriginalAuthoredSourceBodyScope,
) -> bool {
    variables.frame_kind == crate::var_resolve::VariableFrameKind::Local
        && variables
            .execution_name_policy
            .is_none_or(|policy| scope.retains_authored_source_name_policy(policy))
        && variables.interpreter.is_none()
        && variables.execution.is_none()
        && variables.hosted_execution_context.is_none()
        && !variables.dynamic_bindings
        && !variables.dynamic_traces
        && variables.globals.is_empty()
        && variables.ns_vars.is_empty()
        && variables.upvar_aliases.is_empty()
        && variables.instance_vars.is_empty()
        && variables.alias_bindings.is_empty()
        && variables.namespace_alias_bindings.is_empty()
        && variables.name_alias_bindings.is_empty()
        && variables.namespace_name_alias_bindings.is_empty()
        && variables.traced.is_empty()
        && variables.trace_registrations.is_empty()
        && variables.untracked_traces.is_empty()
        && variables.possible_trace_registrations.is_empty()
}

/// Scalar syntax from the exact Standalone declaration lookup and held body.
/// It supplies analytical source operations, never a completed Native handler.
pub(crate) fn original_authored_scalar_invocation(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<AuthoredSourceInvocation> {
    let binding = tokens.source_binding.as_ref()?;
    let scope = binding.original_authored_scalar_body_scope(tokens, registry)?;
    let words = scope.original_words(tokens, registry)?;
    if words.iter().any(|word| word.group().expand) {
        return None;
    }
    let advice = binding.original_authored_declaration_scalar_layout(tokens, registry)?;
    if advice.has_unclosed_source_coverage()
        || scope.has_unclosed_declaration_source_coverage()
        || advice.namespace() != scope.namespace()
    {
        return None;
    }
    let variables = advice.original_variable_context();
    if !authored_scalar_context_is_local(variables, scope) {
        return None;
    }
    let frame_local = |name: &[u8]| {
        scope
            .original_word_dialect()
            .and_then(|dialect| dialect.variable_lookup_policy)
            .map_or_else(
                || tcl_syntax::naming::source_variable_name_is_frame_local_for_all_policies(name),
                |policy| tcl_syntax::naming::source_variable_name_is_frame_local(name, policy),
            )
    };
    let selected = super::resolve_original_source_layout(registry, None, tokens, &advice)?;
    let owner = AuthoredSourceOwner::Body(Arc::new(scope.clone()), Box::new(tokens.clone()));
    let (descriptor, hook, roles, traits) = owner.with_schema(registry, |schema| {
        use tcl_registry::hooks::LoweringHookId;
        let facts = schema.facts();
        let hook = facts.lowering_hook?;
        if facts.descriptor_key() != selected.facts.descriptor_key()
            || !matches!(hook, LoweringHookId::Set | LoweringHookId::Return)
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || schema.semantics.frame_effect.is_some()
            || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
            || schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current
        {
            return None;
        }
        let (roles, complete) = schema.authored_source_argument_roles();
        if !complete {
            return None;
        }
        let roles = roles
            .into_iter()
            .map(|(ordinal, role)| (facts.argument_offset + usize::from(ordinal), role))
            .collect::<Vec<_>>();
        if hook == LoweringHookId::Set {
            if words.len() != 3
                || facts.argument_offset != 0
                || !roles.contains(&(0, tcl_registry::ArgRole::VarWrite))
            {
                return None;
            }
            let name = owner.argument_literal(0)?;
            if name.as_bytes().contains(&0)
                || tcl_syntax::naming::split_element_ref(&name).is_some()
                || !frame_local(name.as_bytes())
            {
                return None;
            }
            owner.argument_literal(1)?;
        } else {
            if words.len() != 2 || facts.argument_offset != 0 {
                return None;
            }
            if owner.argument_literal(0).is_none() {
                let word = words.get(1)?;
                let reference =
                    tcl_lexer::word_parts::whole_var_ref(word.bytes(), word.config()).ok()??;
                if reference.index.is_some()
                    || reference.name.contains(&0)
                    || tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
                    || !frame_local(reference.name)
                {
                    return None;
                }
            }
        }
        Some((
            schema.descriptor_key(),
            hook,
            roles,
            schema.semantics.traits,
        ))
    })?;
    Some(AuthoredSourceInvocation {
        owner,
        descriptor,
        hook,
        roles,
        traits,
        conditional_loop: false,
    })
}

impl AuthoredSourceOwner {
    fn with_schema<T>(
        &self,
        registry: &CommandRegistry,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        match self {
            Self::Header(owner) => owner.with_source_schema(registry, apply),
            Self::Body(scope, tokens) => scope.with_source_schema(tokens, registry, apply),
        }
    }
    fn original_argument_word(&self, argument: usize) -> Option<tcl_lexer::NativeWord> {
        match self {
            Self::Header(owner) => owner.original_argument_word(argument).cloned(),
            Self::Body(scope, tokens) => scope
                .original_words(tokens, scope.registry())?
                .get(argument.checked_add(1)?)
                .cloned(),
        }
    }
    fn argument_literal(&self, argument: usize) -> Option<String> {
        let word = self.original_argument_word(argument)?;
        let value = tcl_syntax::word_rules::original_static_word_unicode_value(&word)?;
        String::from_utf8(value).ok()
    }
}

impl AuthoredSourceInvocation {
    // Exact scalar source identity under the held authored procedure body.
    // The boolean distinguishes a source definition from a source read.
    pub(crate) fn scalar_source_variable(
        &self,
        registry: &CommandRegistry,
    ) -> Option<(
        crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
        String,
        bool,
    )> {
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
        let AuthoredSourceOwner::Body(scope, tokens) = &self.owner else {
            return None;
        };
        let frame =
            crate::command_binding::SourceOriginalVariableFrame::from_authored_scalar_source(
                tokens, registry,
            )?;
        let policy = scope.scalar_name_policy()?;
        let word = self.owner.original_argument_word(0)?;
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&word.config());
        let definition = self.hook == tcl_registry::hooks::LoweringHookId::Set;
        let input = if definition {
            SignatureSourceNameInput::OriginalWord(
                SignatureSourceNameKey::from_original_logical_source_word(&word, rules, policy)?,
            )
        } else {
            // A literal return has no source variable read.
            self.owner.argument_literal(0).is_none().then_some(())?;
            let arena = word.executable_parts();
            let root = crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_logical_source_executable(
                arena, word.image(), word.config(), word.span(), rules, policy,
            )?;
            (!root.is_separate_array_root()).then_some(())?;
            SignatureSourceNameInput::OriginalVariableRoot(root)
        };
        let name = String::from_utf8(input.bytes().to_vec()).ok()?;
        let symbol =
            crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol::from_original_local_input(
                &input,
                frame,
                tcl_core_types::NameBytes::from(input.bytes()),
            );
        Some((symbol, name, definition))
    }

    pub(crate) fn descriptor_key(&self) -> tcl_registry::CanonicalCommandDescriptorKey<'_> {
        match &self.owner {
            AuthoredSourceOwner::Header(owner) => owner.descriptor_key(),
            AuthoredSourceOwner::Body(..) => self.descriptor,
        }
    }
    pub(crate) fn canonical_command(&self) -> &str {
        match &self.owner {
            AuthoredSourceOwner::Header(owner) => owner.canonical_command(),
            AuthoredSourceOwner::Body(..) => self.descriptor.label(),
        }
    }
    pub(crate) fn lowering_hook(&self) -> Option<tcl_registry::hooks::LoweringHookId> {
        match &self.owner {
            AuthoredSourceOwner::Header(owner) => owner.lowering_hook(),
            AuthoredSourceOwner::Body(..) => Some(self.hook),
        }
    }
    pub(crate) fn written_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        self.roles.clone()
    }
    pub(crate) fn argument_literal(&self, argument: usize) -> Option<String> {
        self.owner.argument_literal(argument)
    }
    pub(crate) fn traits(&self) -> tcl_registry::Traits {
        match &self.owner {
            AuthoredSourceOwner::Header(owner) => owner.traits(),
            AuthoredSourceOwner::Body(..) => self.traits,
        }
    }
    pub(crate) fn loop_bindings_conditional(&self) -> bool {
        match &self.owner {
            AuthoredSourceOwner::Header(owner) => owner.loop_bindings_conditional(),
            AuthoredSourceOwner::Body(..) => self.conditional_loop,
        }
    }
    pub(crate) fn with_source_schema<T>(
        &self,
        registry: &CommandRegistry,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        self.owner.with_schema(registry, |schema| {
            (schema.descriptor_key() == self.descriptor).then_some(())?;
            apply(schema)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_scalar_without_runtime_policy_uses_only_unanimous_source_local_geometry() {
        let registry = CommandRegistry::build_default();
        for (name, admitted) in [
            ("value", true),
            ("", true),
            ("$literal", true),
            (":name", true),
            ("a:b", true),
            ("a::b", false),
            ("::name", false),
            ("array(key)", false),
        ] {
            let source = format!("proc p {{}} {{set {{{name}}} DATA}}\n");
            let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
                &source,
                &registry,
                false,
                tcl_lexer::LexerConfig::default(),
            );
            let (&body, _) = unit
                .ir_module
                .original_authored_declaration_body_units
                .iter()
                .next()
                .expect("actual standalone source body");
            let procedure = unit.original_procedure_for_body(body).unwrap();
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(procedure.body.statements.first().unwrap())
                .unwrap();
            assert_eq!(
                original_authored_scalar_invocation(tokens, &registry).is_some(),
                admitted,
                "{name:?}"
            );
            assert!(
                super::super::normal_representation_invocation_with_metadata_context(
                    &registry, None, tokens,
                )
                .is_none()
            );
            assert!(
                super::super::admitted_native_compiler_invocation(&registry, None, tokens)
                    .is_none()
            );
        }
    }

    #[test]
    fn authored_scalar_scopes_keep_profiled_standalone_metadata_without_supplied_fallback() {
        let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
        let context = environment.default_context_registry();
        let registry = context.commands();
        let profile = environment.unit_profile();
        assert!(registry.profile().is_some());
        let source = "proc subject {} {set value 1; return OK}";
        // A profiled default build is a fresh-interpreter document and
        // publishes `::subject` as tclsh8.6
        // does; the dialect-blind build keeps explicit Standalone authoring,
        // still profiled through its Registry.
        let fresh = crate::compilation_unit::CompilationUnit::build_for_profile(
            source, registry, false, profile,
        );
        assert!(fresh.ir_module.source_metadata_input.is_some());
        assert!(fresh.function("::subject").is_some());
        let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
            source,
            registry,
            false,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );
        assert!(unit.ir_module.source_metadata_input.is_none());
        assert!(unit.ir_module.source_entry.metadata_context.is_standalone());
        assert_eq!(
            unit.ir_module
                .original_authored_declaration_body_units
                .len(),
            1
        );
        let (&id, _) = unit
            .ir_module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        assert!(unit.ir_module.procedure_ids.owns(id));
        let procedure = unit.original_procedure_for_body(id).unwrap();
        for statement in &procedure.body.statements {
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            let advice = binding
                .original_authored_declaration_scalar_layout(tokens, registry)
                .expect("closed original standalone scalar declaration layout");
            let metadata = advice
                .original_declaration_metadata_context(registry, tokens)
                .unwrap()
                .expect("the profiled standalone metadata facet");
            assert!(metadata.is_standalone());
            assert!(
                binding
                    .original_authored_scalar_body_scope(tokens, registry)
                    .is_some()
            );
            assert!(original_authored_scalar_invocation(tokens, registry).is_some());
            assert_eq!(
                binding.original_source_advice_purpose(tokens, metadata, registry),
                Some(crate::command_binding::OriginalInvocationSourcePurpose::Standalone)
            );
            assert!(
                binding
                    .original_source_advice_purpose(tokens, context.as_ref().into(), registry)
                    .is_none()
            );
            assert!(
                super::super::normal_representation_invocation_with_metadata_context(
                    registry,
                    Some(metadata),
                    tokens,
                )
                .is_none()
            );
            assert!(
                super::super::admitted_native_compiler_invocation(registry, None, tokens).is_none()
            );
        }
    }

    #[test]
    fn authored_scalar_ssa_keeps_source_frames_separate_from_cells_and_each_other() {
        let registry = CommandRegistry::build_default();
        let source = "proc first {} {set value ONE; return $value}\nproc second {} {set value TWO; return $value}\n";
        let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
            source,
            &registry,
            false,
            tcl_lexer::LexerConfig::default(),
        );
        let definitions = source
            .match_indices("set value")
            .map(|(offset, _)| {
                crate::ssa::original_analysis_definition_at_unit(
                    &unit,
                    tcl_lexer::Span::new(
                        u32::try_from(offset).unwrap() + 4,
                        u32::try_from(offset).unwrap() + 9,
                    ),
                    &registry,
                )
                .expect("retained authored scalar definition")
            })
            .collect::<Vec<_>>();
        assert_eq!(definitions.len(), 2);
        let identities = definitions
            .iter()
            .map(|definition| {
                let ssa = &definition.function().ssa;
                assert!(ssa.cell_key(definition.symbol()).is_none());
                ssa.source_variable_symbol(definition.symbol())
                    .expect("source identity")
            })
            .collect::<Vec<_>>();
        assert_ne!(identities[0], identities[1]);
    }

    #[test]
    fn authored_scalar_bodies_retain_real_source_definitions_without_native_admission() {
        let registry = CommandRegistry::build_default();
        let source = "proc p {} {set value 123; set value TEXT; return $value}\n";
        let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
            source,
            &registry,
            false,
            tcl_lexer::LexerConfig::default(),
        );
        assert!(unit.ir_module.source_metadata_input.is_none());
        assert!(unit.ir_module.source_entry.metadata_context.is_standalone());
        let (&body, _) = unit
            .ir_module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .expect("genuine authored source declaration body");
        let function = unit.function_for_body(Some(body)).unwrap();
        let procedure = unit.original_procedure_for_body(body).unwrap();
        for statement in &procedure.body.statements {
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            assert!(original_authored_scalar_invocation(tokens, &registry).is_some());
            assert!(
                super::super::normal_representation_invocation_with_metadata_context(
                    &registry, None, tokens,
                )
                .is_none()
            );
            assert!(
                super::super::admitted_native_compiler_invocation(&registry, None, tokens)
                    .is_none()
            );
            let mut changed = tokens.clone();
            changed.argv_texts[0].push('_');
            assert!(original_authored_scalar_invocation(&changed, &registry).is_none());
            let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0");
            assert!(original_authored_scalar_invocation(tokens, foreign.commands()).is_none());
        }
        let definitions = source
            .match_indices("set value")
            .map(|(offset, _)| {
                crate::ssa::original_analysis_definition_at_unit(
                    &unit,
                    tcl_lexer::Span::new(
                        u32::try_from(offset).unwrap() + 4,
                        u32::try_from(offset).unwrap() + 9,
                    ),
                    &registry,
                )
                .expect("actual represented authored definition")
            })
            .collect::<Vec<_>>();
        assert_eq!(definitions.len(), 2);
        assert!(
            definitions
                .iter()
                .all(|definition| std::ptr::eq(definition.function(), function))
        );
        assert_eq!(definitions[0].symbol(), definitions[1].symbol());
        assert_ne!(definitions[0].version(), definitions[1].version());
        let offset = u32::try_from(source.find("$value").unwrap()).unwrap();
        let read = crate::ssa::original_source_read_at_unit(
            &unit,
            tcl_lexer::Span::new(offset, offset + 6),
            &registry,
        )
        .expect("actual authored source read");
        assert!(std::ptr::eq(read.function(), function));
        assert_eq!(read.symbol(), definitions[1].symbol());
        assert_eq!(read.version(), Some(definitions[1].version()));
    }

    #[test]
    fn authored_scalar_source_stamp_keeps_actual_parent_rows_and_terminal_points() {
        let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
        let context = environment.default_context_registry();
        let registry = context.commands();
        let config = tcl_lexer::LexerConfig::for_file_grammar(environment.unit_profile().grammar);
        let source = "proc a {used spare} {set local 1; list retained; return $used}\nproc b {} {set other 2}";
        let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
            source, registry, false, config,
        );
        let module = &unit.ir_module;
        let retained = module.retained_source_bindings.as_ref().unwrap();
        let bindings = retained.positioned_for_module(module, registry).unwrap();
        let (&id, _) = module
            .original_authored_declaration_body_units
            .iter()
            .find(|(_, card)| card.name() == "a")
            .unwrap();
        let procedure = &module.body_units[&id];
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(&procedure.body.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let scope = binding
            .original_authored_scalar_body_scope(tokens, registry)
            .unwrap();
        let source_policy = module
            .source_entry
            .options()
            .execution_name_policy()
            .unwrap();
        assert!(scope.retains_authored_source_name_policy(source_policy));
        assert!(original_authored_scalar_invocation(tokens, registry).is_some());
        let source_only = procedure
            .body
            .retained_source_tokens_for_statement(&procedure.body.statements[1])
            .unwrap();
        let source_scope = source_only
            .source_binding
            .as_ref()
            .unwrap()
            .original_authored_source_body_scope_for_name_roles(source_only, registry)
            .unwrap();
        assert!(
            bindings
                .original_authored_declaration_source_tokens(source_only, source_scope, registry)
                .is_some()
        );
        let mut mixed = source_only.clone();
        mixed.source_binding.as_mut().unwrap().may_be_absent = true;
        assert!(
            bindings
                .original_authored_declaration_source_tokens(&mixed, source_scope, registry)
                .is_none()
        );
        let mut mixed = source_only.clone();
        mixed
            .source_binding
            .as_mut()
            .unwrap()
            .evaluated_argument_values
            .push(None);
        assert!(
            bindings
                .original_authored_declaration_source_tokens(&mixed, source_scope, registry)
                .is_none()
        );
        let first = tokens.words().first().unwrap().source().span.start();
        let end = tokens.words().last().unwrap().source().span.end();
        let text = &source[first as usize..end as usize];
        let segments =
            crate::segmenter::segment_commands_with_offset_and_config(text, first, config.nested());
        let [segment] = segments.as_slice() else {
            panic!("one original raw source command");
        };
        let raw =
            crate::ir::CommandTokens::from_segmented(&module.source.source_map(), config, segment);
        assert!(raw.source_binding.is_none());
        let selected = bindings
            .original_authored_declaration_source_tokens(&raw, scope, registry)
            .expect("genuine source-only declaration row");
        let selected_binding = selected.source_binding.as_ref().unwrap();
        assert!(selected_binding.has_original_source_layout_owner());
        assert!(
            selected_binding
                .original_authored_declaration_scalar_layout(&selected, registry)
                .is_some()
        );
        assert!(selected_binding.proved_execution_target().is_none());
        assert!(selected_binding.execution_is_unknown());
        assert!(
            super::super::normal_representation_invocation_with_metadata_context(
                registry, None, &selected
            )
            .is_none()
        );
        assert!(
            super::super::admitted_native_compiler_invocation(registry, None, &selected).is_none()
        );
        assert!(
            bindings
                .original_authored_declaration_source_tokens(&selected, scope, registry)
                .is_none()
        );
        let mut ordinary = raw.clone();
        bindings.stamp_original_tokens(&mut ordinary);
        assert!(
            ordinary
                .source_binding
                .as_ref()
                .unwrap()
                .has_original_source_layout_owner()
        );
        assert!(
            bindings
                .original_authored_declaration_source_tokens(&ordinary, scope, registry)
                .is_none()
        );
        let mut changed = raw.clone();
        changed.argv_texts[0].push('_');
        assert!(
            bindings
                .original_authored_declaration_source_tokens(&changed, scope, registry)
                .is_none()
        );
        let (&other, _) = module
            .original_authored_declaration_body_units
            .iter()
            .find(|(_, card)| card.name() == "b")
            .unwrap();
        let other = &module.body_units[&other];
        let other_tokens = other
            .body
            .retained_source_tokens_for_statement(&other.body.statements[0])
            .unwrap();
        let other_scope = other_tokens
            .source_binding
            .as_ref()
            .unwrap()
            .original_authored_scalar_body_scope(other_tokens, registry)
            .unwrap();
        assert!(
            bindings
                .original_authored_declaration_source_tokens(&raw, other_scope, registry)
                .is_none()
        );
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0");
        assert!(
            bindings
                .original_authored_declaration_source_tokens(&raw, scope, foreign.commands())
                .is_none()
        );
    }

    #[test]
    fn authored_scalar_source_keeps_alias_array_computed_and_expanded_cases_generic() {
        let registry = CommandRegistry::build_default();
        for source in [
            "proc p {} {set array(key) TEXT}",
            "proc p {} {set ::value TEXT}",
            "proc p {} {set $name TEXT}",
            "proc p {} {set value [helper]}",
            "proc p {} {set {*}{value TEXT}}",
            "proc p {} {global value; set value TEXT}",
            "interp alias {} setter {} set; proc p {} {setter value TEXT}",
            "proc set args {return OVERRIDE}; proc p {} {set value TEXT}",
        ] {
            let module = crate::lowering::lower_to_ir(source, &registry);
            let mut tested = 0;
            for (_, procedure) in module.original_source_procedure_bodies() {
                for statement in &procedure.body.statements {
                    let Some(tokens) = procedure
                        .body
                        .retained_source_tokens_for_statement(statement)
                    else {
                        continue;
                    };
                    if tokens
                        .argv_texts
                        .first()
                        .is_some_and(|head| head == "set" || head == "setter")
                    {
                        assert!(
                            original_authored_scalar_invocation(tokens, &registry).is_none(),
                            "{source}"
                        );
                        assert!(
                            matches!(statement, crate::ir::Statement::Call { .. }),
                            "{source}"
                        );
                        tested += 1;
                    }
                }
            }
            assert_eq!(tested, 1, "{source}");
        }
    }

    #[test]
    fn authored_scalar_return_refuses_combined_arrays_and_nul_names() {
        let registry = CommandRegistry::build_default();
        for source in [
            "proc p {} {return ${array(key)}}",
            "proc p {} {return ${nul\0name}}",
        ] {
            let module = crate::lowering::lower_to_ir(source, &registry);
            let bodies = module
                .original_source_procedure_bodies()
                .collect::<Vec<_>>();
            let [(_, procedure)] = bodies.as_slice() else {
                panic!("one genuine original Authored declaration");
            };
            assert_eq!(procedure.body.statements.len(), 1);
            let statement = &procedure.body.statements[0];
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            assert!(original_authored_scalar_invocation(tokens, &registry).is_none());
            assert!(matches!(statement, crate::ir::Statement::Call { .. }));
        }
    }

    #[test]
    fn authored_control_bodies_keep_source_scope_without_execution_owners() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(
            "proc p {} {if {$x} {set {$literal} VALUE}; catch {set {$other} VALUE} result}",
            &registry,
        );
        let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
        let crate::ir::Statement::If { clauses, .. } = &procedure.body.statements[0] else {
            panic!("original conditional If source structure");
        };
        let child = &clauses[0].body;
        assert!(child.executed_source.is_none());
        assert_eq!(child.statements.len(), 1);
        let tokens = child
            .retained_source_tokens_for_statement(&child.statements[0])
            .unwrap();
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_authored_source_body_scope_for_name_roles(tokens, &registry)
                .is_some()
        );
        assert!(
            super::super::normal_representation_invocation_with_metadata_context(
                &registry, None, tokens
            )
            .is_none()
        );
        assert!(
            super::super::admitted_native_compiler_invocation(&registry, None, tokens).is_none()
        );
        assert!(matches!(
            procedure.body.statements[1],
            crate::ir::Statement::Catch { .. }
        ));
        let top = crate::lowering::lower_to_ir("if {$x} {catch {set x 1} result}", &registry);
        let crate::ir::Statement::If { clauses, .. } = &top.top_level.statements[0] else {
            panic!("original top-level conditional If");
        };
        assert!(matches!(
            clauses[0].body.statements[0],
            crate::ir::Statement::Catch { .. }
        ));
    }

    #[test]
    fn authored_control_source_owner_refuses_changed_words_store_and_bytecode() {
        let registry = CommandRegistry::build_default();
        let source = "proc p {} {if {$x} {set x 1}}";
        let module = crate::lowering::lower_to_ir(source, &registry);
        let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(&procedure.body.statements[0])
            .unwrap();
        assert!(original_authored_control_invocation(tokens, &registry).is_some());
        let mut changed = tokens.clone();
        changed.argv_texts[1] = "different".to_owned();
        assert!(original_authored_control_invocation(&changed, &registry).is_none());
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(original_authored_control_invocation(tokens, foreign).is_none());
        let bytecode = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::new(&registry).for_bytecode_backend(),
            source,
        );
        assert!(bytecode.original_authored_declaration_body_units.is_empty());
        assert!(!bytecode.top_level.statements.is_empty());
        assert!(
            bytecode
                .top_level
                .statements
                .iter()
                .all(|statement| !matches!(statement, crate::ir::Statement::If { .. }))
        );
    }
    #[test]
    fn authored_control_grammar_refuses_original_expansions_and_non_immediate_descriptors() {
        let registry = CommandRegistry::build_default();
        for source in ["if {*}{1 {set x 1}}", "if {*}$words"] {
            let module = crate::lowering::lower_to_ir(source, &registry);
            let tokens = module
                .top_level
                .statements
                .iter()
                .find_map(|statement| {
                    module
                        .top_level
                        .retained_source_tokens_for_statement(statement)
                })
                .unwrap();
            assert!(original_authored_control_invocation(tokens, &registry).is_none());
        }
        fn immediate(
            _: tcl_registry::InvocationArguments<'_>,
        ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
            vec![(1, tcl_registry::ScriptTiming::SameInvocation)]
        }
        fn deferred(
            _: tcl_registry::InvocationArguments<'_>,
        ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
            vec![(1, tcl_registry::ScriptTiming::Deferred)]
        }
        fn reference(
            _: tcl_registry::InvocationArguments<'_>,
        ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
            vec![(1, tcl_registry::ScriptTiming::ReferenceOnly)]
        }
        for (timing, expected) in [
            (immediate as tcl_registry::ScriptTimingResolver, true),
            (deferred as tcl_registry::ScriptTimingResolver, false),
            (reference as tcl_registry::ScriptTimingResolver, false),
        ] {
            let mut registry = CommandRegistry::build_default();
            registry.insert(tcl_registry::CommandSpec {
                name: "source-control",
                arity: tcl_registry::Arity::exact(2),
                arg_roles: &[
                    (0, tcl_registry::ArgRole::Expr),
                    (1, tcl_registry::ArgRole::Body),
                ],
                lowering_hook: Some(tcl_registry::hooks::LoweringHookId::If),
                script_timing_resolver: Some(timing),
                ..tcl_registry::CommandSpec::DEFAULT
            });
            let module =
                crate::lowering::lower_to_ir("proc p {} {source-control 1 {set x 1}}", &registry);
            let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(&procedure.body.statements[0])
                .unwrap();
            assert_eq!(
                original_authored_control_invocation(tokens, &registry).is_some(),
                expected
            );
        }
    }
}
