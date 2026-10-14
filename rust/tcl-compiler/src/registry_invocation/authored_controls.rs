// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional source control grammar retains its original header or body scope.

use super::{CommandRegistry, CommandTokens, OriginalAuthoredSourceBodyScope};
use std::sync::Arc;

pub(crate) struct AuthoredControlInvocation {
    owner: AuthoredControlOwner,
    descriptor: tcl_registry::CanonicalCommandDescriptorKey<'static>,
    hook: tcl_registry::hooks::LoweringHookId,
    roles: Vec<(usize, tcl_registry::ArgRole)>,
    traits: tcl_registry::Traits,
    conditional_loop: bool,
}

enum AuthoredControlOwner {
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
        AuthoredControlOwner::Body(Arc::new(scope.clone()), tokens.clone())
    } else {
        AuthoredControlOwner::Header(
            super::original_authored_source_structure(tokens, registry)?
                .into_closed_source_grammar()?,
        )
    };
    let expansion = match &owner {
        AuthoredControlOwner::Header(grammar) => grammar.has_original_word_expansion(),
        AuthoredControlOwner::Body(scope, tokens) => scope
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
        AuthoredControlInvocation {
            owner,
            descriptor,
            hook,
            roles,
            traits,
            conditional_loop,
        },
    )))
}

impl AuthoredControlOwner {
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

impl AuthoredControlInvocation {
    pub(crate) fn descriptor_key(&self) -> tcl_registry::CanonicalCommandDescriptorKey<'_> {
        match &self.owner {
            AuthoredControlOwner::Header(owner) => owner.descriptor_key(),
            AuthoredControlOwner::Body(..) => self.descriptor,
        }
    }
    pub(crate) fn canonical_command(&self) -> &str {
        match &self.owner {
            AuthoredControlOwner::Header(owner) => owner.canonical_command(),
            AuthoredControlOwner::Body(..) => self.descriptor.label(),
        }
    }
    pub(crate) fn lowering_hook(&self) -> Option<tcl_registry::hooks::LoweringHookId> {
        match &self.owner {
            AuthoredControlOwner::Header(owner) => owner.lowering_hook(),
            AuthoredControlOwner::Body(..) => Some(self.hook),
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
            AuthoredControlOwner::Header(owner) => owner.traits(),
            AuthoredControlOwner::Body(..) => self.traits,
        }
    }
    pub(crate) fn loop_bindings_conditional(&self) -> bool {
        match &self.owner {
            AuthoredControlOwner::Header(owner) => owner.loop_bindings_conditional(),
            AuthoredControlOwner::Body(..) => self.conditional_loop,
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
        fn immediate(_: &[&str]) -> Vec<(u8, tcl_registry::ScriptTiming)> {
            vec![(1, tcl_registry::ScriptTiming::SameInvocation)]
        }
        fn deferred(_: &[&str]) -> Vec<(u8, tcl_registry::ScriptTiming)> {
            vec![(1, tcl_registry::ScriptTiming::Deferred)]
        }
        fn reference(_: &[&str]) -> Vec<(u8, tcl_registry::ScriptTiming)> {
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
