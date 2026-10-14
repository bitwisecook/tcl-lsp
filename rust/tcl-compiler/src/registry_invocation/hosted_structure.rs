// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source control-flow grammar keeps hosted metadata separate from handlers.

use super::source_structure::OriginalRegistryWords;
use super::{
    CommandRegistry, CommandTokens, InvocationMetadataContext, LogicalStructuredInvocation,
};

pub(crate) enum SourceStructuredInvocation {
    Logical(Box<LogicalStructuredInvocation>),
    Hosted(Box<HostedStructuredInvocation>),
    NativeSource(Box<HostedStructuredInvocation>),
    Authored(Box<super::authored_controls::AuthoredSourceInvocation>),
}

pub(crate) struct HostedStructuredInvocation {
    words: OriginalRegistryWords,
    input: crate::analyser::ResolvedAnalysisInput,
    descriptor: tcl_registry::CanonicalCommandDescriptorKey<'static>,
    hook: tcl_registry::hooks::LoweringHookId,
    source_roles: Vec<(usize, tcl_registry::ArgRole)>,
    case_bodies: Option<tcl_registry::case_bodies::CaseBodyOperands>,
}

pub(crate) fn original_hosted_structured_invocation(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<SourceStructuredInvocation> {
    let input = context.source_analysis_input()?;
    let metadata =
        super::source_taint::original_hosted_source_registry_metadata(registry, context, tokens)?;
    if metadata
        .shape()
        .original_words()
        .iter()
        .any(|word| word.group().expand)
    {
        return None;
    }
    let words = super::source_structure::original_vendor_registry_words(
        input.borrowed_context_registry(),
        tokens,
        metadata,
    )?;
    let (descriptor, hook, source_roles, case_bodies) =
        words.with_source_schema(input.borrowed_context_registry(), |schema| {
            let hook = schema.facts().lowering_hook?;
            if !matches!(
                hook,
                tcl_registry::hooks::LoweringHookId::If
                    | tcl_registry::hooks::LoweringHookId::Switch
            ) || schema.facts().arity_accepts_frozen_arguments() != Some(true)
            {
                return None;
            }
            let case_bodies = if schema.semantics.options.case_list.is_some() {
                (schema.semantics.frame_effect.is_none()
                    && schema.semantics.body_kind == tcl_registry::BodyKind::Plain
                    && schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current)
                    .then_some(())?;
                let clauses = schema.authored_source_case_clause_projection(
                    tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config()),
                )?;
                for body in &clauses.bodies {
                    if schema.authored_source_script_timing_at(body.argument)?
                        != tcl_registry::ScriptTiming::SameInvocation
                    {
                        return None;
                    }
                }
                Some(clauses)
            } else {
                None
            };
            let source_roles = if let Some(clauses) = &case_bodies {
                let mut roles = Vec::new();
                for body in &clauses.bodies {
                    if !roles.contains(&(body.argument, tcl_registry::ArgRole::Body)) {
                        roles.push((body.argument, tcl_registry::ArgRole::Body));
                    }
                }
                roles
            } else {
                schema.authored_source_argument_roles().1.then_some(())?;
                words.roles()?.to_vec()
            };
            for &(argument, role) in &source_roles {
                if role == tcl_registry::ArgRole::Body
                    && words.arguments().get(argument)?.literal_bytes().is_none()
                {
                    return None;
                }
            }
            Some((schema.descriptor_key(), hook, source_roles, case_bodies))
        })??;
    Some(SourceStructuredInvocation::Hosted(Box::new(
        HostedStructuredInvocation {
            words,
            input: input.clone(),
            descriptor,
            hook,
            source_roles,
            case_bodies,
        },
    )))
}

pub(crate) fn original_native_source_structured_invocation(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<SourceStructuredInvocation> {
    let input = context.source_analysis_input()?;
    if input.has_logical_source_name_context()
        || input.has_hosted_source_name_context()
        || !context.matches_registry(registry)
    {
        return None;
    }
    let words = tokens
        .source_binding
        .as_ref()?
        .original_native_source_words(tokens, input, registry)?;
    let super::source_structure::OriginalRegistrySource::SourceTransitions(advice) = words.source()
    else {
        return None;
    };
    if advice
        .original_words()
        .iter()
        .any(|word| word.group().expand)
    {
        return None;
    }
    if words
        .origins()
        .iter()
        .enumerate()
        .any(|(written, origin)| *origin != super::InvocationWordOrigin::Written(written))
    {
        return None;
    }
    let (descriptor, hook) =
        words.with_source_schema(input.borrowed_context_registry(), |schema| {
            use tcl_registry::hooks::LoweringHookId;
            let hook = schema.facts().lowering_hook?;
            if !matches!(
                hook,
                LoweringHookId::Set
                    | LoweringHookId::Return
                    | LoweringHookId::Expr
                    | LoweringHookId::If
                    | LoweringHookId::While
                    | LoweringHookId::For
                    | LoweringHookId::Foreach
                    | LoweringHookId::Lmap
                    | LoweringHookId::Catch
                    | LoweringHookId::Try
            ) || schema.facts().arity_accepts_frozen_arguments() != Some(true)
                || !schema.authored_source_argument_roles().1
            {
                return None;
            }
            for &(argument, role) in words.roles()? {
                if role == tcl_registry::ArgRole::Body
                    && words.arguments().get(argument)?.literal_bytes().is_none()
                {
                    return None;
                }
            }
            Some((schema.descriptor_key(), hook))
        })??;
    let source_roles = words.roles()?.to_vec();
    Some(SourceStructuredInvocation::NativeSource(Box::new(
        HostedStructuredInvocation {
            words,
            input: input.clone(),
            descriptor,
            hook,
            source_roles,
            case_bodies: None,
        },
    )))
}

impl SourceStructuredInvocation {
    pub(crate) fn canonical_command(&self) -> &str {
        match self {
            Self::Logical(selected) => selected.canonical_command(),
            Self::Authored(selected) => selected.canonical_command(),
            Self::Hosted(selected) | Self::NativeSource(selected) => selected.descriptor.label(),
        }
    }
    pub(crate) fn descriptor_key(&self) -> tcl_registry::CanonicalCommandDescriptorKey<'_> {
        match self {
            Self::Logical(selected) => selected.descriptor_key(),
            Self::Authored(selected) => selected.descriptor_key(),
            Self::Hosted(selected) | Self::NativeSource(selected) => selected.descriptor,
        }
    }
    pub(crate) fn lowering_identity(&self) -> super::SelectedLoweringBindingIdentity<'_> {
        match self {
            Self::Logical(_) => {
                super::SelectedLoweringBindingIdentity::Logical(self.descriptor_key())
            }
            Self::Authored(_) => {
                super::SelectedLoweringBindingIdentity::AuthoredSourceSchema(self.descriptor_key())
            }
            Self::Hosted(_) => {
                super::SelectedLoweringBindingIdentity::HostedSchema(self.descriptor_key())
            }
            Self::NativeSource(_) => {
                super::SelectedLoweringBindingIdentity::NativeSourceSchema(self.descriptor_key())
            }
        }
    }
    pub(crate) fn with_metadata_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        realm: tcl_dialect::model::InvocationRealm,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        match self {
            Self::Authored(_) => None,
            Self::Logical(selected) => {
                selected.with_metadata_schema(registry, context, realm, apply)
            }
            Self::Hosted(selected) | Self::NativeSource(selected) => {
                if (matches!(self, Self::Hosted(_))
                    && realm != tcl_dialect::model::InvocationRealm::RuleLoader)
                    || !context.matches_registry(registry)
                    || context.source_analysis_input() != Some(&selected.input)
                {
                    return None;
                }
                selected
                    .words
                    .with_source_schema(selected.input.borrowed_context_registry(), apply)?
            }
        }
    }
    /// Source grammar retains its catalogue purpose independently of the
    /// original runtime realm. Full input and store correspondence stay mandatory.
    pub(crate) fn with_original_source_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        original_realm: tcl_dialect::model::InvocationRealm,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        if let Self::Authored(selected) = self {
            return selected.with_source_schema(registry, apply);
        }
        let realm = match self {
            Self::Hosted(_) => tcl_dialect::model::InvocationRealm::RuleLoader,
            Self::Logical(_) | Self::NativeSource(_) | Self::Authored(_) => original_realm,
        };
        self.with_metadata_schema(registry, context, realm, apply)
    }

    /// Independently owned Authored source grammar needs no external context.
    /// Other purposes retain their existing complete supplied metadata checks.
    pub(crate) fn with_lowering_source_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        realm: tcl_dialect::model::InvocationRealm,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        if let Self::Authored(selected) = self {
            return selected.with_source_schema(registry, apply);
        }
        self.with_original_source_schema(registry, context?, realm, apply)
    }

    pub(crate) fn lowering_hook(&self) -> Option<tcl_registry::hooks::LoweringHookId> {
        match self {
            Self::Logical(selected) => selected.lowering_hook(),
            Self::Authored(selected) => selected.lowering_hook(),
            Self::Hosted(selected) | Self::NativeSource(selected) => Some(selected.hook),
        }
    }
    pub(crate) fn written_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        match self {
            Self::Logical(selected) => selected.written_roles(),
            Self::Authored(selected) => selected.written_roles(),
            Self::Hosted(selected) | Self::NativeSource(selected) => {
                let roles = selected.case_bodies.as_ref().map_or_else(
                    || selected.source_roles.clone(),
                    |clauses| {
                        clauses
                            .bodies
                            .iter()
                            .map(|body| (body.argument, tcl_registry::ArgRole::Body))
                            .collect()
                    },
                );
                let mut written_roles = Vec::new();
                for (argument, role) in roles {
                    let Some(super::InvocationWordOrigin::Written(written)) = argument
                        .checked_add(1)
                        .and_then(|ordinal| selected.words.origins().get(ordinal))
                    else {
                        continue;
                    };
                    let Some(argument) = written.checked_sub(1) else {
                        continue;
                    };
                    if !written_roles.contains(&(argument, role)) {
                        written_roles.push((argument, role));
                    }
                }
                written_roles
            }
        }
    }
    pub(crate) fn argument_literal(&self, argument: usize) -> Option<String> {
        match self {
            Self::Logical(selected) => selected.argument_literal(argument),
            Self::Authored(selected) => selected.argument_literal(argument),
            Self::Hosted(selected) | Self::NativeSource(selected) => {
                std::str::from_utf8(selected.words.arguments().get(argument)?.literal_bytes()?)
                    .ok()
                    .map(str::to_owned)
            }
        }
    }
    pub(crate) fn traits(&self) -> Option<tcl_registry::Traits> {
        match self {
            Self::Logical(selected) => Some(selected.traits()),
            Self::Authored(selected) => Some(selected.traits()),
            Self::Hosted(selected) | Self::NativeSource(selected) => selected
                .words
                .with_source_schema(selected.input.borrowed_context_registry(), |schema| {
                    schema.semantics.traits
                }),
        }
    }
    pub(crate) fn loop_bindings_conditional(&self) -> bool {
        match self {
            Self::Logical(selected) => selected.loop_bindings_conditional(),
            Self::Authored(selected) => selected.loop_bindings_conditional(),
            Self::Hosted(selected) | Self::NativeSource(selected) => selected
                .words
                .with_source_schema(selected.input.borrowed_context_registry(), |schema| {
                    schema.facts().repeated_args.iter().any(|layout| {
                        layout.conditional_binding
                            && layout.role == tcl_registry::ArgRole::LoopVarList
                    })
                })
                .unwrap_or(true),
        }
    }
    pub(crate) fn error_context(&self) -> Option<tcl_registry::InlineBodyErrorContext> {
        match self {
            Self::Logical(selected) => selected.error_context(),
            Self::Hosted(_) | Self::NativeSource(_) | Self::Authored(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    fn module(source: &str) -> crate::ir::Module {
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let profile = tcl_dialect::DialectProfile::irules();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(context.commands(), config)
                .with_dialect(Some(profile))
                .with_resolved_analysis_input(input),
            source,
        )
    }

    #[test]
    fn hosted_source_controls_keep_authentic_grammar_without_native_or_logical_admission() {
        for (source, switch) in [
            (
                "when HTTP_REQUEST {if {[HTTP::path] eq \"/a\"} {pool route}}",
                false,
            ),
            (
                "when HTTP_REQUEST {switch -exact [HTTP::path] {/a {pool route} default {pool fallback}}}",
                true,
            ),
        ] {
            let module = module(source);
            assert!(module.source_entry.native_entry.is_none());
            assert!(
                module
                    .source_metadata_input
                    .as_ref()
                    .unwrap()
                    .has_hosted_source_name_context()
            );
            assert!(
                !module
                    .source_metadata_input
                    .as_ref()
                    .unwrap()
                    .has_logical_source_name_context()
            );
            let registry = module.resolved_registry();
            let (id, procedure) = module
                .procedures
                .iter()
                .find(|(id, _)| module.irules_event_bodies.contains_key(id))
                .unwrap();
            assert!(
                crate::source_graph::event_body_for_procedure(&module, id, procedure, registry)
                    .is_some()
            );
            let statement = &procedure.body.statements[0];
            assert!(if switch {
                matches!(statement, crate::ir::Statement::Switch { .. })
            } else {
                matches!(statement, crate::ir::Statement::If { .. })
            });
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let metadata = super::InvocationMetadataContext::for_module(registry, &module).unwrap();
            let selected =
                super::original_hosted_structured_invocation(registry, metadata, tokens).unwrap();
            assert!(matches!(
                selected,
                super::SourceStructuredInvocation::Hosted(_)
            ));
            if switch {
                let super::SourceStructuredInvocation::Hosted(owner) = &selected else {
                    unreachable!()
                };
                assert!(owner.case_bodies.as_ref().unwrap().selection_unknown);
                assert!(owner.words.roles().is_none());
                assert_eq!(
                    selected.written_roles(),
                    vec![(2, tcl_registry::ArgRole::Body)]
                );
            }
            assert!(matches!(
                selected.lowering_identity(),
                crate::registry_invocation::SelectedLoweringBindingIdentity::HostedSchema(_)
            ));
            assert!(
                crate::registry_invocation::original_structured_invocation_with_metadata_context(
                    registry, metadata, tokens
                )
                .is_none()
            );
            assert!(
                crate::registry_invocation::admitted_native_compiler_invocation(
                    registry, None, tokens
                )
                .is_none()
            );
            assert!(
                crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                    registry,
                    Some(metadata),
                    tokens
                )
                .is_none()
            );
            assert!(
                selected
                    .with_original_source_schema(
                        registry,
                        metadata,
                        tcl_dialect::model::InvocationRealm::InterpreterRuntime,
                        |schema| Some(schema.facts().lowering_hook),
                    )
                    .flatten()
                    .is_some()
            );
            assert!(
                selected
                    .with_metadata_schema(
                        registry,
                        metadata,
                        tcl_dialect::model::InvocationRealm::InterpreterRuntime,
                        |_| Some(())
                    )
                    .is_none()
            );
            assert!(
                selected
                    .with_metadata_schema(
                        registry,
                        metadata,
                        tcl_dialect::model::InvocationRealm::RuleLoader,
                        |_| Some(())
                    )
                    .is_some()
            );
            let input = module.source_metadata_input.as_ref().unwrap();
            let profile = tcl_dialect::DialectProfile::plain_tcl();
            let policy = tcl_syntax::naming::VendorSourceNamePolicy::authored(
                tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::TmmIRule,
            )
            .unwrap();
            let changed_inputs = [
                crate::analyser::ResolvedAnalysisInput::new(
                    profile,
                    input.unit_profile(),
                    input.context_registry(),
                    input.lexer_config(),
                ),
                crate::analyser::ResolvedAnalysisInput::new(
                    input.analyser_profile(),
                    profile,
                    input.context_registry(),
                    input.lexer_config(),
                ),
                input.clone().with_vendor_source_policy(policy),
            ];
            for changed in changed_inputs {
                assert!(changed.has_hosted_source_name_context());
                let changed_metadata =
                    super::InvocationMetadataContext::for_analysis_input(registry, &changed)
                        .unwrap();
                assert!(
                    super::original_hosted_structured_invocation(
                        registry,
                        changed_metadata,
                        tokens
                    )
                    .is_none()
                );
            }
            for (policy, accepted) in [
                (policy, true),
                (tcl_syntax::naming::VendorSourceNamePolicy::authored(
                    tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::TmshCliScript
                ).unwrap(), false),
            ] {
                let issued_input = input.clone().with_vendor_source_policy(policy);
                let issued_metadata = super::InvocationMetadataContext::for_analysis_input(
                    registry, &issued_input
                ).unwrap();
                let mut issued = tokens.clone();
                issued.hosted_taint_context = Some(std::sync::Arc::new(
                    crate::registry_invocation::HostedSourceTaintContext::from_lowering(
                        module.source.clone(), issued_input.lexer_config(), issued_input.context_registry()
                    ).unwrap().retain_source_structure_input(Some(&issued_input))
                ));
                assert_eq!(super::original_hosted_structured_invocation(
                    registry, issued_metadata, &issued
                ).is_some(), accepted);
            }
            let mut missing_input = tokens.clone();
            missing_input.hosted_taint_context = Some(std::sync::Arc::new(
                crate::registry_invocation::HostedSourceTaintContext::from_lowering(
                    module.source.clone(),
                    input.lexer_config(),
                    input.context_registry(),
                )
                .unwrap(),
            ));
            assert!(
                super::original_hosted_structured_invocation(registry, metadata, &missing_input)
                    .is_none()
            );
            let mut missing = tokens.clone();
            missing.hosted_taint_context = None;
            assert!(
                super::original_hosted_structured_invocation(registry, metadata, &missing)
                    .is_none()
            );
            let mut stale = tokens.clone();
            stale.argv_texts[1] = "changed".to_owned();
            assert!(
                super::original_hosted_structured_invocation(registry, metadata, &stale).is_none()
            );
            assert!(
                super::original_hosted_structured_invocation(
                    registry,
                    crate::registry_invocation::InvocationMetadataContext::from(input_context(
                        registry
                    )),
                    tokens
                )
                .is_none()
            );
        }
    }

    #[test]
    fn hosted_source_grammar_cannot_supply_bytecode_hook_admission() {
        let source = "when HTTP_REQUEST {if {1} {pool route}}";
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let profile = tcl_dialect::DialectProfile::irules();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(context.commands(), config)
                .with_dialect(Some(profile))
                .with_resolved_analysis_input(input)
                .for_bytecode_backend(),
            source,
        );
        let statement = module.top_level.statements.first().unwrap();
        assert!(matches!(
            statement,
            crate::ir::Statement::Call { .. } | crate::ir::Statement::Barrier { .. }
        ));
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        assert!(tokens.original_words_match_argv_text());
        assert_eq!(
            tokens.argv_texts,
            ["when", "HTTP_REQUEST", "if {1} {pool route}"]
        );
        assert!(
            crate::registry_invocation::original_native_compiler_words(
                &module.source,
                tokens.words(),
                0,
                module.lexer_config
            )
            .is_some()
        );
        assert!(module.procedures.values().all(|procedure| {
            procedure
                .body
                .statements
                .iter()
                .all(|statement| !matches!(statement, crate::ir::Statement::If { .. }))
        }));
        assert!(module.body_units.values().all(|unit| {
            unit.body
                .statements
                .iter()
                .all(|statement| !matches!(statement, crate::ir::Statement::If { .. }))
        }));
    }

    fn input_context(
        registry: &tcl_registry::CommandRegistry,
    ) -> &'static tcl_registry::model::ContextRegistry {
        tcl_registry::model::ingress::static_context_for_profile(registry.profile().unwrap())
    }

    #[test]
    fn hosted_source_control_grammar_refuses_shadowed_heads_and_dynamic_bodies() {
        for source in [
            "proc if args {return}; when HTTP_REQUEST {if {1} {pool fake}}",
            "when HTTP_REQUEST {if {1} $body}",
        ] {
            let module = module(source);
            let procedure = module
                .procedures
                .iter()
                .find(|(id, _)| module.irules_event_bodies.contains_key(id))
                .unwrap()
                .1;
            assert!(
                procedure
                    .body
                    .statements
                    .iter()
                    .all(|statement| !matches!(statement, crate::ir::Statement::If { .. }))
            );
        }
    }
}
