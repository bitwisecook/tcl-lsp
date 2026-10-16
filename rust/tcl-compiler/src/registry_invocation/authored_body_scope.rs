// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original standalone body source owns conditional grammar independently of frames.

use crate::command_binding::{SourceAnalysisEntry, SourceNamespaceKey};
use crate::ir::{CommandTokens, Module};
use std::sync::Arc;
use tcl_lexer::{NativeScriptCommandWords, NativeWord};
use tcl_registry::{CommandRegistry, RegistrySnapshot};

/// A genuine source body and its unchanged original lexical command vectors.
/// This provides no reached lookup, physical frame, handler or Normal facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalAuthoredSourceBodyScope {
    header: CommandTokens,
    body: NativeWord,
    namespace: SourceNamespaceKey,
    registry: RegistrySnapshot,
    entry: SourceAnalysisEntry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    realm: tcl_dialect::model::InvocationRealm,
    dialect: Option<tcl_registry::InvocationDialect>,
    commands: Arc<[NativeScriptCommandWords]>,
    parent: Option<Arc<Self>>,
    conditional_source_geometry:
        Option<Arc<crate::command_binding::OriginalAuthoredProcedureSourceGeometry>>,
}

impl OriginalAuthoredSourceBodyScope {
    pub(super) fn capture(
        header: &CommandTokens,
        body: &NativeWord,
        namespace: &SourceNamespaceKey,
        module: &Module,
    ) -> Option<Arc<Self>> {
        if module.source_metadata_input.is_some()
            || module.source_entry.native_entry.is_some()
            || module.source_entry.hosted_execution_context.is_some()
            || module.source_entry.unknown_entry
        {
            return None;
        }
        let binding = header.source_binding.as_ref()?;
        if let Some(geometry) =
            binding.original_authored_procedure_source_geometry(header, module.resolved_registry())
        {
            let registry = geometry.registry_snapshot()?.clone();
            (geometry.body_word() == body
                && geometry.namespace() == namespace
                && geometry.config() == module.native_lexer_config()
                && module.registry_snapshot.as_ref()?.semantic_key() == registry.semantic_key()
                && body.image() == &module.source)
                .then_some(())?;
            return Some(Arc::new(Self {
                header: header.clone(),
                body: body.clone(),
                namespace: namespace.clone(),
                realm: geometry.realm(),
                registry,
                entry: module.source_entry.clone(),
                profile: module.dialect_profile,
                dialect: binding.original_source_word_dialect_for_tokens(header),
                commands: Self::commands(body)?,
                parent: None,
                conditional_source_geometry: Some(Arc::new(geometry)),
            }));
        }
        let registry = binding
            .retained_registry_snapshot_for_tokens(header)?
            .clone();
        let grammar = super::original_authored_source_structure(header, registry.registry())?
            .into_closed_source_grammar()?;
        let positions = grammar.with_source_schema(registry.registry(), |schema| {
            schema.authored_source_procedure_arguments()
        })?;
        (grammar.original_body_word(positions.body)? == body
            && grammar.config() == module.native_lexer_config()
            && module.registry_snapshot.as_ref()?.semantic_key() == registry.semantic_key())
        .then_some(())?;
        let scope = Self {
            header: header.clone(),
            body: body.clone(),
            namespace: namespace.clone(),
            registry,
            entry: module.source_entry.clone(),
            profile: module.dialect_profile,
            realm: binding.invocation_realm()?,
            dialect: binding.original_source_word_dialect_for_tokens(header),
            commands: Self::commands(body)?,
            parent: None,
            conditional_source_geometry: None,
        };
        Some(Arc::new(scope))
    }

    pub(crate) fn capture_plain_root(
        header: &CommandTokens,
        body: &NativeWord,
        module: &Module,
    ) -> Option<Arc<Self>> {
        if module.source_metadata_input.is_some()
            || module.source_entry.native_entry.is_some()
            || module.source_entry.hosted_execution_context.is_some()
            || module.source_entry.unknown_entry
        {
            return None;
        }
        let binding = header.source_binding.as_ref()?;
        let registry = binding
            .retained_registry_snapshot_for_tokens(header)?
            .clone();
        let grammar = super::original_authored_source_structure(header, registry.registry())?
            .into_closed_source_grammar()?;
        let argument = (0..header.words().len().checked_sub(1)?)
            .find(|&argument| grammar.original_argument_word(argument) == Some(body))?;
        grammar.with_source_schema(registry.registry(), |schema| {
            (schema.semantics.frame_effect.is_none()
                && schema.semantics.body_kind == tcl_registry::BodyKind::Plain
                && schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
                && schema
                    .authored_source_plain_script_arguments()?
                    .contains(&argument)
                && schema.authored_source_script_timing_at(argument)?
                    == tcl_registry::ScriptTiming::SameInvocation)
                .then_some(())
        })?;
        (module.registry_snapshot.as_ref()?.semantic_key() == registry.semantic_key()
            && module.native_lexer_config() == body.config()
            && body.image() == &module.source)
            .then_some(())?;
        Some(Arc::new(Self {
            header: header.clone(),
            body: body.clone(),
            namespace: grammar.source_namespace().clone(),
            registry,
            entry: module.source_entry.clone(),
            profile: module.dialect_profile,
            realm: binding.invocation_realm()?,
            dialect: binding.original_source_word_dialect_for_tokens(header),
            commands: Self::commands(body)?,
            parent: None,
            conditional_source_geometry: None,
        }))
    }

    fn commands(body: &NativeWord) -> Option<Arc<[NativeScriptCommandWords]>> {
        let span = super::source_scoped_body::original_written_script_body_span(body)?;
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            body.image().clone(),
            span,
            body.config(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        plan.fatal_tail.is_none().then_some(())?;
        Some(Arc::from(plan.commands))
    }

    pub(crate) fn has_unclosed_declaration_source_coverage(&self) -> bool {
        self.conditional_source_geometry
            .as_ref()
            .is_some_and(|geometry| geometry.has_unclosed_source_coverage())
    }

    /// Original retained declaration/header and body geometry; no point or
    /// physical frame is selected through these readonly source facets.
    pub(crate) fn original_parent_site(
        &self,
    ) -> Option<&crate::command_binding::CommandAllocationSite> {
        self.header.source_binding.as_ref()?.invocation_site()
    }
    pub(crate) fn original_body_word(&self) -> &NativeWord {
        &self.body
    }

    /// Authored naming metadata remains distinct from a Native execution
    /// policy. Only the exact recipe retained by this original Standalone
    /// source entry can accompany its conditional scalar syntax.
    pub(crate) fn retains_authored_source_name_policy(
        &self,
        policy: tcl_syntax::naming::ExecutionNamePolicy,
    ) -> bool {
        self.entry.metadata_context.is_standalone()
            && self.entry.native_entry.is_none()
            && self.entry.logical_source_input.is_none()
            && self.entry.vendor_source_input.is_none()
            && self.entry.hosted_execution_context.is_none()
            && !self.entry.unknown_entry
            && self.entry.options().execution_name_policy() == Some(policy)
            && policy.native_recipe().is_some_and(|recipe| {
                recipe.authority() == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
            })
    }

    pub(crate) fn original_word_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.dialect
    }
    pub(crate) fn source_realm(&self) -> tcl_dialect::model::InvocationRealm {
        self.realm
    }
    pub(crate) fn retained_registry_snapshot(&self) -> &RegistrySnapshot {
        &self.registry
    }

    pub(crate) fn registry(&self) -> &CommandRegistry {
        self.registry.registry()
    }
    pub(crate) fn namespace(&self) -> &SourceNamespaceKey {
        &self.namespace
    }
    pub(crate) fn config(&self) -> tcl_lexer::LexerConfig {
        self.body.config()
    }

    pub(crate) fn original_words(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<Vec<NativeWord>> {
        if tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || registry.snapshot().semantic_key() != self.registry.semantic_key()
        {
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let original = super::original_native_compiler_words(
            self.body.image(),
            tokens.words(),
            offset,
            self.body.config(),
        )?;
        self.commands
            .iter()
            .any(|command| command.words == original)
            .then_some(original)
    }

    pub(crate) fn with_source_schema<T>(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        let original = self.original_words(tokens, registry)?;
        let values = original
            .iter()
            .map(tcl_syntax::word_rules::original_static_word_unicode_value)
            .collect::<Vec<_>>();
        let word = |at: usize| {
            if original[at].group().expand {
                return tcl_registry::InvocationWord::Expanded;
            }
            values[at]
                .as_deref()
                .map_or(tcl_registry::InvocationWord::Dynamic, |bytes| {
                    super::source_structure::source_schema_word(
                        tcl_registry::InvocationWord::KnownBytes(bytes),
                    )
                })
        };
        let arguments = (1..original.len()).map(word).collect::<Vec<_>>();
        let words = tcl_registry::InvocationWords::structured(word(0), &arguments);
        let words = self
            .dialect
            .map_or(words, |dialect| words.with_dialect(dialect));
        let schema =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.registry.registry(),
                None,
                words,
                self.realm,
            )
            .resolved()?;
        apply(&schema)
    }

    /// Derive only a selected same-invocation plain body in this source scope.
    /// Namespace transitions, frame selection and deferred entry retain refusal.
    pub(crate) fn for_plain_body(
        self: &Arc<Self>,
        parent: &CommandTokens,
        body: &NativeWord,
    ) -> Option<Arc<Self>> {
        let original = self.original_words(parent, self.registry.registry())?;
        let argument = original.iter().skip(1).position(|word| word == body)?;
        self.with_source_schema(parent, self.registry.registry(), |schema| {
            (schema.semantics.frame_effect.is_none()
                && schema.semantics.body_kind == tcl_registry::BodyKind::Plain
                && schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
                && schema
                    .authored_source_plain_script_arguments()?
                    .contains(&argument)
                && schema.authored_source_script_timing_at(argument)?
                    == tcl_registry::ScriptTiming::SameInvocation)
                .then_some(())
        })?;
        Some(Arc::new(Self {
            header: self.header.clone(),
            body: body.clone(),
            namespace: self.namespace.clone(),
            registry: self.registry.clone(),
            entry: self.entry.clone(),
            profile: self.profile,
            realm: self.realm,
            dialect: self.dialect,
            commands: Self::commands(body)?,
            parent: Some(Arc::clone(self)),
            conditional_source_geometry: self.conditional_source_geometry.clone(),
        }))
    }

    pub(crate) fn original_analysis_attachment(
        &self,
    ) -> Option<&Arc<crate::command_binding::OriginalSourceAnalysisOwner>> {
        self.header
            .source_binding
            .as_ref()?
            .original_analysis_attachment()
    }

    pub(crate) fn belongs_to_original_scope(&self, original: &Self) -> bool {
        let mut scope = self;
        loop {
            if std::ptr::eq(scope, original) {
                return true;
            }
            let Some(parent) = scope.parent.as_deref() else {
                return false;
            };
            scope = parent;
        }
    }

    pub(crate) fn matches_module(&self, module: &Module) -> bool {
        self.entry == module.source_entry
            && self.profile == module.dialect_profile
            && module.source_metadata_input.is_none()
            && module.source_entry.native_entry.is_none()
            && module.source_entry.hosted_execution_context.is_none()
            && !module.source_entry.unknown_entry
            && self.body.image() == &module.source
            && self.body.config() == module.native_lexer_config()
            && module
                .registry_snapshot
                .as_ref()
                .is_some_and(|owner| owner.semantic_key() == self.registry.semantic_key())
    }
}

/// Conditional own-formal alias incidence, independent of any actual linked
/// variable/cell, caller activation or successful operation.
#[derive(Debug, Clone)]
pub(crate) struct OriginalAuthoredFormalAliasObservation {
    body: crate::ir::ProcedureId,
    card: Arc<super::OriginalAuthoredProcedureBody>,
    original: Arc<[NativeWord]>,
    transition: tcl_registry::VariableCellAliasTransition,
    parameter: String,
    obligations: [super::original_procedure_call::OriginalAuthoredBodyCallObligation; 3],
}
impl OriginalAuthoredFormalAliasObservation {
    pub(crate) fn parameter(&self) -> &str {
        &self.parameter
    }
    pub(crate) fn belongs_to(&self, command: &OriginalAuthoredParameterSourceCommand) -> bool {
        self.body == command.body
            && Arc::ptr_eq(&self.card, &command.card)
            && self.original.first().is_some_and(|word| {
                word.image() == command.original[0].image()
                    && word.config() == command.original[0].config()
            })
            && self
                .transition
                .words
                .local
                .checked_add(1)
                .is_some_and(|ordinal| self.original.get(ordinal).is_some())
            && self
                .transition
                .words
                .target
                .checked_add(1)
                .is_some_and(|ordinal| self.original.get(ordinal).is_some())
            && self.obligations == command.obligations
    }
}

#[derive(Debug, Clone)]
enum AuthoredParameterSourceWord {
    Literal(Vec<u8>),
    Scalar(Vec<u8>),
}

/// Exact original Builtin source syntax and typed Registry alias/role grammar.
/// Every observation is conditional on body entry, lookup and observers; this
/// carries no physical aliases, effects, completion or Native/Normal facts.
pub(crate) struct OriginalAuthoredParameterSourceCommand {
    body: crate::ir::ProcedureId,
    card: Arc<super::OriginalAuthoredProcedureBody>,
    original: Arc<[NativeWord]>,
    words: Vec<AuthoredParameterSourceWord>,
    aliases: Vec<tcl_registry::VariableCellAliasTransition>,
    roles: Vec<(usize, tcl_registry::ArgRole)>,
    dialect: tcl_registry::InvocationDialect,
    continues: bool,
    obligations: [super::original_procedure_call::OriginalAuthoredBodyCallObligation; 3],
}
impl OriginalAuthoredParameterSourceCommand {
    pub(crate) fn body(&self) -> crate::ir::ProcedureId {
        self.body
    }
    pub(crate) fn roles(&self) -> &[(usize, tcl_registry::ArgRole)] {
        &self.roles
    }
    pub(crate) fn aliases(&self) -> &[tcl_registry::VariableCellAliasTransition] {
        &self.aliases
    }
    pub(crate) fn continues(&self) -> bool {
        self.continues
    }
    pub(crate) fn has_unproved_applicability(&self) -> bool {
        !self.obligations.is_empty()
    }
    pub(crate) fn literal(&self, argument: usize) -> Option<&str> {
        let AuthoredParameterSourceWord::Literal(value) =
            self.words.get(argument.checked_add(1)?)?
        else {
            return None;
        };
        std::str::from_utf8(value).ok()
    }
    fn formal(&self, argument: usize) -> Option<&str> {
        let AuthoredParameterSourceWord::Scalar(name) = self.words.get(argument.checked_add(1)?)?
        else {
            return None;
        };
        self.source_name_is_local(name).then_some(())?;
        self.card
            .parameter_names()
            .find(|formal| formal.as_bytes() == name.as_slice())
    }
    fn source_name_is_local(&self, name: &[u8]) -> bool {
        self.dialect.variable_lookup_policy.map_or_else(
            || tcl_syntax::naming::source_variable_name_is_frame_local_for_all_policies(name),
            |policy| tcl_syntax::naming::source_variable_name_is_frame_local(name, policy),
        )
    }
    pub(crate) fn local_scalar(&self, name: &str) -> bool {
        !name.as_bytes().contains(&0)
            && tcl_syntax::naming::split_element_ref_bytes(name.as_bytes()).is_none()
            && self.source_name_is_local(name.as_bytes())
    }
    pub(crate) fn alias_target_formal(
        &self,
        alias: &tcl_registry::VariableCellAliasTransition,
    ) -> Option<&str> {
        self.aliases
            .iter()
            .any(|actual| std::ptr::eq(actual, alias))
            .then_some(())?;
        (alias
            .destination
            .is_active_in_frame(tcl_registry::VariableAliasFrame::Procedure, self.dialect)
            == Some(true))
        .then_some(())?;
        let variable = match &alias.target {
            tcl_registry::VariableAliasTarget::Global { variable }
            | tcl_registry::VariableAliasTarget::CurrentNamespace { variable }
            | tcl_registry::VariableAliasTarget::Namespace { variable, .. }
            | tcl_registry::VariableAliasTarget::CallerSelectedFrame { variable, .. } => variable,
        };
        let tcl_registry::TransitionSubject::Unknown { argument_index, .. } = variable else {
            return None;
        };
        (*argument_index == alias.words.target).then_some(())?;
        self.formal(alias.words.target)
    }

    pub(crate) fn caller_alias(
        &self,
        alias: &tcl_registry::VariableCellAliasTransition,
    ) -> Option<OriginalAuthoredFormalAliasObservation> {
        use tcl_registry::{CallerFrameSelection, FrameLevel, VariableAliasTarget};
        let VariableAliasTarget::CallerSelectedFrame { frame, .. } = &alias.target else {
            return None;
        };
        let level = match frame {
            CallerFrameSelection::DefaultCaller => FrameLevel::DEFAULT,
            CallerFrameSelection::Explicit(subject) => {
                FrameLevel::parse_for_dialect(subject.literal()?, self.dialect)?
            }
        };
        let local = self.literal(alias.words.local)?;
        let parameter = self.alias_target_formal(alias)?;
        (level.is_caller_frame()
            && self.local_scalar(local)
            && alias.local.literal() == Some(local))
        .then_some(())?;
        Some(OriginalAuthoredFormalAliasObservation {
            body: self.body,
            card: Arc::clone(&self.card),
            original: Arc::clone(&self.original),
            transition: alias.clone(),
            parameter: parameter.to_owned(),
            obligations: self.obligations,
        })
    }
}

fn parameter_source_word(word: &NativeWord) -> Option<AuthoredParameterSourceWord> {
    if let Some(literal) = tcl_syntax::word_rules::original_static_word_unicode_value(word) {
        return Some(AuthoredParameterSourceWord::Literal(literal));
    }
    let mut scalar = None;
    let literal = tcl_syntax::word_rules::original_word_unicode_value_with_variables(
        word,
        |arena, component| {
            let tcl_lexer::ExecutablePart::Variable { name, index: None } = component.part else {
                return None;
            };
            if scalar.is_some() {
                return None;
            }
            scalar = Some(arena.bytes(name)?.to_vec());
            Some(Vec::new())
        },
    )?;
    literal
        .is_empty()
        .then_some(AuthoredParameterSourceWord::Scalar(scalar?))
}

/// Reborrow genuine original declaration/source syntax for conditional formal
/// observations. A present stronger source owner retains its own terminal gate.
pub(crate) fn original_authored_parameter_source_command_for_module(
    tokens: &CommandTokens,
    module: &Module,
    registry: &CommandRegistry,
) -> Option<OriginalAuthoredParameterSourceCommand> {
    let retained = module.retained_source_bindings.as_deref()?;
    let positioned = retained.positioned_for_module(module, registry)?;
    retained
        .owns_original_analysis_attachment(tokens)
        .then_some(())?;
    let binding = tokens.source_binding.as_ref()?;
    let scope = if binding.has_original_source_layout_owner() {
        super::original_authored_scalar_invocation(tokens, registry)?;
        binding.original_authored_scalar_body_scope(tokens, registry)?
    } else {
        binding.original_authored_source_body_scope_for_name_roles(tokens, registry)?
    };
    scope.matches_module(module).then_some(())?;
    let original = scope.original_words(tokens, registry)?;
    if original.iter().any(|word| word.group().expand) {
        return None;
    }
    let mut remaining = 4096usize;
    for word in &original {
        remaining = remaining.checked_sub(1)?;
        for _ in word.executable_parts().all_parts() {
            remaining = remaining.checked_sub(1)?;
        }
    }
    let words = original
        .iter()
        .map(parameter_source_word)
        .collect::<Option<Vec<_>>>()?;
    let mut owners = module
        .original_authored_declaration_body_units
        .iter()
        .filter(|(id, card)| {
            let Some(procedure) = module.body_units.get(*id) else {
                return false;
            };
            if !module.procedure_ids.owns(**id)
                || !card.matches_procedure(module, procedure)
                || !card.owns_original_source_scope(scope)
                || !retained.matches_original_procedure_summary(**id, procedure)
            {
                return false;
            }
            let mut held = false;
            crate::ir::for_each_statement(&procedure.body, &mut |statement| {
                held |= statement
                    .tokens()
                    .is_some_and(|actual| std::ptr::eq(actual, tokens));
            });
            held
        });
    let (&body, card) = owners.next()?;
    owners.next().is_none().then_some(())?;
    let declared = if binding.has_original_source_layout_owner() {
        tokens.clone()
    } else {
        positioned.original_authored_declaration_source_tokens(tokens, scope, registry)?
    };
    let advice = declared
        .source_binding
        .as_ref()?
        .original_authored_declaration_scalar_layout(&declared, registry)?;
    (advice.namespace() == scope.namespace()
        && advice.original_variable_context().frame_kind
            == crate::var_resolve::VariableFrameKind::Local)
        .then_some(())?;
    let selected = super::resolve_original_source_layout(registry, None, &declared, &advice)?;
    let dialect = scope.original_word_dialect()?;
    let (roles, aliases, continues) = scope.with_source_schema(tokens, registry, |schema| {
        let facts = schema.facts();
        let (roles, complete) = schema.authored_source_argument_roles();
        if facts.descriptor_key() != selected.facts.descriptor_key()
            || !complete
            || facts.arity_accepts_frozen_arguments() != Some(true)
        {
            return None;
        }
        let roles = roles
            .into_iter()
            .map(|(at, role)| (facts.argument_offset + usize::from(at), role))
            .collect::<Vec<_>>();
        let (_, transitions) = schema.authored_source_effects_and_state_transitions(dialect);
        let continues = schema.semantics.state_transitions.is_declared()
            && transitions.command_resolution_impact()
                == tcl_registry::CommandResolutionImpact::None
            && transitions.facts().iter().all(|fact| {
                matches!(
                    fact.transition,
                    tcl_registry::StateTransition::VariableCellAlias(_)
                )
            })
            && ![
                tcl_registry::StateTransitionDomain::VariableTraces,
                tcl_registry::StateTransitionDomain::CommandTraces,
                tcl_registry::StateTransitionDomain::ExecutionTraces,
                tcl_registry::StateTransitionDomain::InterpreterTopology,
                tcl_registry::StateTransitionDomain::InterpreterPolicy,
                tcl_registry::StateTransitionDomain::Interpreters,
                tcl_registry::StateTransitionDomain::ObjectDispatch,
            ]
            .into_iter()
            .any(|domain| transitions.widens(domain))
            && !roles.iter().any(|(_, role)| {
                matches!(
                    role,
                    tcl_registry::ArgRole::Body
                        | tcl_registry::ArgRole::Expr
                        | tcl_registry::ArgRole::LambdaLiteral
                        | tcl_registry::ArgRole::CommandPrefix
                )
            })
            && !facts.traits.intersects(
                tcl_registry::Traits::EVALUATES_CODE
                    | tcl_registry::Traits::DYNAMIC_EVAL_BODY
                    | tcl_registry::Traits::DEFERS_BODY
                    | tcl_registry::Traits::PERFORMS_SUBSTITUTION,
            );
        let aliases = transitions
            .facts()
            .iter()
            .filter_map(|fact| {
                if let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition {
                    Some(alias.clone())
                } else {
                    None
                }
            })
            .collect();
        Some((roles, aliases, continues))
    })?;
    Some(OriginalAuthoredParameterSourceCommand {
        body,
        card: Arc::clone(card),
        original: original.into(),
        words,
        aliases,
        roles,
        dialect,
        continues,
        obligations: [
            super::original_procedure_call::OriginalAuthoredBodyCallObligation::BodyEntry,
            super::original_procedure_call::OriginalAuthoredBodyCallObligation::ActualLookup,
            super::original_procedure_call::OriginalAuthoredBodyCallObligation::ObserverApplicability,
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_parameter_source_words_preserve_full_scalar_names_and_refuse_indices() {
        let source = tcl_lexer::SourceImage::document(
            r#"take ${p($k)} $arr($k) "$n" prefix$n {$n} [unknown]"#,
        );
        let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            source.clone(),
            tcl_lexer::Span::new(0, u32::try_from(source.bytes().len()).unwrap()),
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .unwrap();
        assert!(plan.fatal_tail.is_none());
        let [command] = plan.commands.as_slice() else {
            panic!("one complete original vector");
        };
        let words = &command.words;
        assert!(matches!(parameter_source_word(&words[1]),
            Some(AuthoredParameterSourceWord::Scalar(name)) if name == b"p($k)"));
        assert!(parameter_source_word(&words[2]).is_none());
        assert!(matches!(parameter_source_word(&words[3]),
            Some(AuthoredParameterSourceWord::Scalar(name)) if name == b"n"));
        assert!(parameter_source_word(&words[4]).is_none());
        assert!(matches!(parameter_source_word(&words[5]),
            Some(AuthoredParameterSourceWord::Literal(value)) if value == b"$n"));
        assert!(parameter_source_word(&words[6]).is_none());
        assert!(
            words
                .iter()
                .all(|word| word.image() == &source && word.config() == config)
        );
    }
}
