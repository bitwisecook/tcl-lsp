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
    Header(super::authored_source_structure::AuthoredSourceGrammar),
    Body(Arc<OriginalAuthoredSourceBodyScope>, CommandTokens),
}

pub(crate) fn original_authored_control_invocation(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<super::SourceStructuredInvocation> {
    let binding = tokens.source_binding.as_ref()?;
    let owner = if binding.has_original_authored_source_body_name_role_owner() {
        let scope = binding.original_authored_source_body_scope_for_name_roles(tokens, registry)?;
        AuthoredSourceOwner::Body(Arc::new(scope.clone()), tokens.clone())
    } else {
        AuthoredSourceOwner::Header(
            super::original_authored_source_structure(tokens, registry)?
                .into_closed_source_grammar()?,
        )
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
    if advice.namespace() != scope.namespace() {
        return None;
    }
    let variables = advice.original_variable_context();
    if variables.frame_kind != crate::var_resolve::VariableFrameKind::Local
        || variables.execution_name_policy.is_some()
        || variables.interpreter.is_some()
        || variables.execution.is_some()
        || variables.hosted_execution_context.is_some()
        || variables.dynamic_bindings
        || variables.dynamic_traces
        || !variables.globals.is_empty()
        || !variables.ns_vars.is_empty()
        || !variables.upvar_aliases.is_empty()
        || !variables.instance_vars.is_empty()
        || !variables.alias_bindings.is_empty()
        || !variables.namespace_alias_bindings.is_empty()
        || !variables.name_alias_bindings.is_empty()
        || !variables.namespace_name_alias_bindings.is_empty()
        || !variables.traced.is_empty()
        || !variables.trace_registrations.is_empty()
        || !variables.untracked_traces.is_empty()
        || !variables.possible_trace_registrations.is_empty()
    {
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
    let owner = AuthoredSourceOwner::Body(Arc::new(scope.clone()), tokens.clone());
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
                    tcl_lexer::Span::new(offset as u32 + 4, offset as u32 + 9),
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
        let offset = source.find("$value").unwrap() as u32;
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
