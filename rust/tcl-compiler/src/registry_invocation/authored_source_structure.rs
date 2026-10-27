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

//! Original Authored/Standalone MAY grammar, independent of Native execution.

use super::{CommandRegistry, CommandTokens, InvocationFacts, ResolvedStatementInvocation};

/// Sealed source metadata from the exact retained store and complete original vector.
pub(crate) struct AuthoredSourceStructuredInvocation {
    invocation: ResolvedStatementInvocation,
    config: tcl_lexer::LexerConfig,
    unknown_residual: bool,
    registry: tcl_registry::RegistrySnapshot,
    realm: tcl_dialect::model::InvocationRealm,
    namespace: crate::command_binding::SourceNamespaceKey,
    original_words: Vec<tcl_lexer::NativeWord>,
}
impl AuthoredSourceStructuredInvocation {
    pub(crate) fn into_closed_source_grammar(self) -> Option<AuthoredSourceGrammar> {
        if self.unknown_residual || self.facts().arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        let roles = self.with_source_schema(self.registry.registry(), |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            complete.then(|| {
                roles
                    .into_iter()
                    .map(|(argument, role)| {
                        (schema.facts().argument_offset + usize::from(argument), role)
                    })
                    .collect::<Vec<_>>()
            })
        })?;
        Some(AuthoredSourceGrammar {
            source: self,
            roles,
        })
    }

    pub(crate) fn with_source_schema<T>(
        &self,
        registry: &CommandRegistry,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        if registry.snapshot().semantic_key() != self.registry.semantic_key() {
            return None;
        }
        self.invocation.with_argument_words(|words| {
            let selected =
                tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                    registry, None, words, self.realm,
                )
                .resolved()?;
            (selected.descriptor_key() == self.facts().descriptor_key()).then_some(())?;
            apply(&selected)
        })
    }

    pub(crate) fn has_unknown_residual(&self) -> bool {
        self.unknown_residual
    }
    pub(crate) fn facts(&self) -> &InvocationFacts {
        &self.invocation.facts
    }
    pub(crate) fn direct_written_arguments(&self) -> bool {
        self.invocation.effective.origins.len() == self.invocation.effective.words.len()
            && self.invocation.effective.origins.len()
                == self.invocation.arguments.len().saturating_add(1)
            && self
                .invocation
                .effective
                .origins
                .iter()
                .enumerate()
                .skip(1)
                .all(|(index, origin)| *origin == super::InvocationWordOrigin::Written(index))
    }
    pub(crate) fn argument_literal(&self, argument: usize) -> Option<String> {
        self.invocation.argument_literal(argument)
    }
    pub(crate) fn config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
    pub(crate) fn original_argument_word(&self, argument: usize) -> Option<&tcl_lexer::NativeWord> {
        let written = self.invocation.effective.written_argument(argument)?;
        self.original_words.get(written.checked_add(1)?)
    }
    pub(crate) fn dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.invocation.dialect
    }
    pub(crate) fn realm(&self) -> tcl_dialect::model::InvocationRealm {
        self.realm
    }
}

/// Closed original source grammar; no invoked body, frame or Normal effect follows.
pub(crate) struct AuthoredSourceGrammar {
    source: AuthoredSourceStructuredInvocation,
    roles: Vec<(usize, tcl_registry::ArgRole)>,
}

impl AuthoredSourceGrammar {
    pub(crate) fn has_original_word_expansion(&self) -> bool {
        self.source
            .original_words
            .iter()
            .any(|word| word.group().expand)
    }
    pub(crate) fn canonical_command(&self) -> &str {
        &self.source.facts().canonical_command
    }
    pub(crate) fn descriptor_key(&self) -> tcl_registry::CanonicalCommandDescriptorKey<'_> {
        self.source.facts().descriptor_key()
    }
    pub(crate) fn lowering_hook(&self) -> Option<tcl_registry::hooks::LoweringHookId> {
        self.source.facts().lowering_hook
    }
    pub(crate) fn written_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        self.roles
            .iter()
            .filter_map(|&(argument, role)| {
                self.source
                    .invocation
                    .effective
                    .written_argument(argument)
                    .map(|written| (written, role))
            })
            .collect()
    }
    pub(crate) fn source_namespace(&self) -> &crate::command_binding::SourceNamespaceKey {
        &self.source.namespace
    }
    pub(crate) fn with_source_schema<T>(
        &self,
        registry: &CommandRegistry,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        self.source.with_source_schema(registry, apply)
    }
    pub(crate) fn original_argument_word(&self, argument: usize) -> Option<&tcl_lexer::NativeWord> {
        self.source.original_words.get(argument.checked_add(1)?)
    }
    pub(crate) fn argument_literal(&self, argument: usize) -> Option<String> {
        self.source.argument_literal(argument)
    }
    pub(crate) fn config(&self) -> tcl_lexer::LexerConfig {
        self.source.config()
    }
    pub(crate) fn traits(&self) -> tcl_registry::Traits {
        self.source.facts().traits
    }
    pub(crate) fn loop_bindings_conditional(&self) -> bool {
        self.source.facts().repeated_args.iter().any(|layout| {
            layout.conditional_binding && layout.role == tcl_registry::ArgRole::LoopVarList
        })
    }
    pub(crate) fn original_body_word(&self, argument: usize) -> Option<&tcl_lexer::NativeWord> {
        self.written_roles()
            .contains(&(argument, tcl_registry::ArgRole::Body))
            .then_some(())?;
        self.source.argument_literal(argument)?;
        self.original_argument_word(argument)
    }
}

pub(crate) fn original_authored_source_structure(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<AuthoredSourceStructuredInvocation> {
    if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding
        .original_source_metadata_for_advice(tokens, registry)
        .ok()?;
    let advice = binding.original_authored_source_layout(tokens, registry)?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let site = binding.invocation_site()?;
    let original_words = super::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        config,
    )?;
    let registry_owner = binding
        .retained_registry_snapshot_for_tokens(tokens)?
        .clone();
    let invocation = super::resolve_original_source_layout(registry, metadata, tokens, &advice)?;
    Some(AuthoredSourceStructuredInvocation {
        invocation,
        config,
        unknown_residual: advice.has_unclosed_source_coverage(),
        registry: registry_owner,
        realm: advice.realm(),
        namespace: advice.namespace().clone(),
        original_words,
    })
}

/// Source-data access on a conditional Registry handler path. These variants
/// describe the analysis model, not a successful runtime store or Native step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthoredSourceVariableAccess {
    Read,
    Write,
    MayWrite,
}

impl AuthoredSourceVariableAccess {
    pub(crate) fn is_definition(self) -> bool {
        self != Self::Read
    }
    pub(crate) fn is_may_write(self) -> bool {
        self == Self::MayWrite
    }
}

fn authored_script_scalar_context_is_quiet(tokens: &CommandTokens) -> bool {
    let Some(binding) = tokens.source_binding.as_ref() else {
        return false;
    };
    let variables = &binding.variable_context;
    variables.frame_kind == crate::var_resolve::VariableFrameKind::Global
        && variables.namespace_known
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
        && variables.trace_registration_receivers.is_empty()
        && variables.untracked_traces.is_empty()
        && variables.possible_trace_registrations.is_empty()
}

type AuthoredScriptScalarVariable = (
    crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    String,
    AuthoredSourceVariableAccess,
);

fn authored_script_scalar_definition(
    source: &AuthoredSourceStructuredInvocation,
    tokens: &CommandTokens,
    namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    registry: &CommandRegistry,
) -> Result<Option<AuthoredScriptScalarVariable>, ()> {
    use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
    use crate::signature_scan::variable_symbol::{
        OriginalVariableSymbolPurpose, OriginalVariableSymbolReceiver,
        SignatureSourceVariableSymbol,
    };
    let form = source
        .with_source_schema(registry, |schema| {
            let facts = schema.facts();
            if facts.lowering_hook != Some(tcl_registry::hooks::LoweringHookId::Set) {
                return Some(Ok(None));
            }
            if facts.argument_offset != 0
                || facts.arity_accepts_frozen_arguments() != Some(true)
                || source.original_words.len() != 3
                || schema.semantics.frame_effect.is_some()
                || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
                || schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current
            {
                return Some(Err(()));
            }
            let (roles, complete) = schema.authored_source_argument_roles();
            if !complete || !roles.contains(&(0, tcl_registry::ArgRole::VarWrite)) {
                return Some(Err(()));
            }
            Some(
                schema
                    .authored_source_variable_receiver_operand_form(0)
                    .map(Some)
                    .ok_or(()),
            )
        })
        .ok_or(())??;
    let Some(form) = form else {
        return Ok(None);
    };
    let word = source.original_argument_word(0).ok_or(())?;
    let input = SignatureSourceNameInput::OriginalWord(
        SignatureSourceNameKey::from_original_logical_source_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&word.config()),
            policy,
        )
        .ok_or(())?,
    );
    let name = String::from_utf8(input.bytes().to_vec()).map_err(|_| ())?;
    if name.as_bytes().contains(&0) || tcl_syntax::naming::split_element_ref(&name).is_some() {
        return Err(());
    }
    let symbol = SignatureSourceVariableSymbol::from_original_input(
        &input,
        namespace,
        OriginalVariableSymbolPurpose::NamespaceStorage,
        OriginalVariableSymbolReceiver::Operand(form),
    )
    .ok_or(())?;
    let access = if !source.has_unknown_residual()
        && authored_script_scalar_context_is_quiet(tokens)
        && source
            .original_words
            .iter()
            .all(|word| tcl_syntax::word_rules::original_static_word_unicode_value(word).is_some())
    {
        AuthoredSourceVariableAccess::Write
    } else {
        AuthoredSourceVariableAccess::MayWrite
    };
    Ok(Some((symbol, name, access)))
}

fn authored_script_scalar_reference(
    word: &tcl_lexer::NativeWord,
    namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Result<Option<AuthoredScriptScalarVariable>, ()> {
    use crate::signature_scan::scope::SignatureSourceNameInput;
    use crate::signature_scan::variable_symbol::{
        OriginalVariableSymbolPurpose, OriginalVariableSymbolReceiver,
        SignatureSourceVariableSymbol,
    };
    let Some(reference) =
        tcl_lexer::word_parts::whole_var_ref(word.bytes(), word.config()).map_err(|_| ())?
    else {
        return Ok(None);
    };
    if reference.index.is_some()
        || reference.name.contains(&0)
        || tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
    {
        return Err(());
    }
    let root = crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_logical_source_executable(
        word.executable_parts(), word.image(), word.config(), word.span(),
        tcl_syntax::word_rules::WordValueRules::from_config(&word.config()), policy,
    ).ok_or(())?;
    if root.is_separate_array_root() {
        return Err(());
    }
    let input = SignatureSourceNameInput::OriginalVariableRoot(root);
    let name = String::from_utf8(input.bytes().to_vec()).map_err(|_| ())?;
    let symbol = SignatureSourceVariableSymbol::from_original_input(
        &input,
        namespace,
        OriginalVariableSymbolPurpose::NamespaceStorage,
        OriginalVariableSymbolReceiver::LexicalRoot,
    )
    .ok_or(())?;
    Ok(Some((symbol, name, AuthoredSourceVariableAccess::Read)))
}

/// A bounded scalar source access in the independently retained Standalone
/// root script. Multiple independent names and arrays remain unavailable.
pub(crate) fn original_authored_script_scalar_variable(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<AuthoredScriptScalarVariable> {
    let (namespace, policy) = tokens
        .source_binding
        .as_ref()?
        .original_standalone_script_variable_scope(tokens, registry)?;
    let source = original_authored_source_structure(tokens, registry)?;
    if !source.direct_written_arguments()
        || source.original_words.iter().any(|word| word.group().expand)
    {
        return None;
    }
    let mut selected =
        authored_script_scalar_definition(&source, tokens, &namespace, policy, registry).ok()?;
    for word in source.original_words.iter().skip(1) {
        let Some(current) = authored_script_scalar_reference(word, &namespace, policy).ok()? else {
            continue;
        };
        if selected.as_ref().is_some_and(|prior| prior != &current) {
            return None;
        }
        selected = Some(current);
    }
    if selected
        .as_ref()
        .is_some_and(|(_, _, access)| !access.is_definition())
        && (source.has_unknown_residual()
            || !authored_script_scalar_context_is_quiet(tokens)
            || source.original_words.iter().any(|word| {
                tcl_syntax::word_rules::original_static_word_unicode_value(word).is_none()
                    && tcl_lexer::word_parts::whole_var_ref(word.bytes(), word.config())
                        .ok()
                        .flatten()
                        .is_none()
            }))
    {
        return None;
    }
    selected
}

/// Independently owned possible syntax; it retains every lookup obligation.
/// This route is consumed only by sink diagnostics, never scalar SSA/effects.
fn original_possible_sink_structure(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<(
    AuthoredSourceStructuredInvocation,
    crate::command_binding::OriginalCatalogueSourceCandidate,
)> {
    if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
        return None;
    }
    let candidate = tokens
        .source_binding
        .as_ref()?
        .original_standalone_script_catalogue_candidate(tokens, registry)?;
    let config = candidate.original_head().name_input().lexer_config();
    let mut dialect = candidate.original_source_dialect()?;
    dialect.lexer_grammar = config.grammar_over(dialect.lexer_grammar);
    dialect.word_values =
        tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
    let realm = candidate.original_source_realm()?;
    let namespace = candidate.original_source_namespace_key()?.clone();
    let effective =
        super::compose_original_effective_words(tokens, &candidate.candidate().name, &[])?;
    let values: Vec<_> = effective
        .words
        .iter()
        .map(|word| {
            super::effective_invocation_word(
                word,
                dialect.lexer_grammar.escapes,
                dialect.word_values,
            )
        })
        .collect();
    let words: Vec<_> = values
        .iter()
        .map(super::EffectiveInvocationWord::as_registry_word)
        .collect();
    let (resolution, case_selection) =
        super::resolve_registry_words_in_realm_with_metadata_context_and_case(
            registry,
            None,
            &words,
            Some(dialect),
            realm,
        )
        .ok()?;
    let super::RegistryInvocationResolution::Resolved(facts) = resolution else {
        return None;
    };
    let evaluated_words = values.get(1..)?.to_vec();
    let source = AuthoredSourceStructuredInvocation {
        invocation: ResolvedStatementInvocation {
            dialect: Some(dialect),
            facts,
            arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
            evaluated_arguments: evaluated_words
                .iter()
                .map(|word| match word {
                    super::EffectiveInvocationWord::Literal(value) => Some(value.clone()),
                    _ => None,
                })
                .collect(),
            evaluated_words,
            effective,
            authored_case_body_selection: Some(case_selection),
        },
        config,
        unknown_residual: true,
        registry: registry.snapshot(),
        realm,
        namespace,
        original_words: candidate.original_words().to_vec(),
    };
    Some((source, candidate))
}

/// Conditional taint syntax for one genuine original Standalone script call.
/// The schema retains possible handler applicability, including unknown/error
/// alternatives. It cannot supply effects, a sanitizer, Normal or compilation.
pub(crate) struct AuthoredSourceTaintInvocation {
    source: AuthoredSourceStructuredInvocation,
    tokens: CommandTokens,
    conditional: Option<crate::command_binding::OriginalCatalogueSourceCandidate>,
    symbolic_body_reads: bool,
}

impl AuthoredSourceTaintInvocation {
    pub(crate) fn capture(tokens: &CommandTokens, registry: &CommandRegistry) -> Option<Self> {
        let strong = tokens
            .source_binding
            .as_ref()?
            .original_standalone_script_variable_scope(tokens, registry)
            .and_then(|_| original_authored_source_structure(tokens, registry));
        let (source, conditional) = if let Some(source) = strong {
            (source, None)
        } else {
            let (source, candidate) = original_possible_sink_structure(tokens, registry)?;
            (source, Some(candidate))
        };
        if !source.direct_written_arguments()
            || source.original_words.iter().any(|word| word.group().expand)
            || source.facts().arity_accepts_frozen_arguments() == Some(false)
        {
            return None;
        }
        Some(Self {
            source,
            tokens: tokens.clone(),
            conditional,
            symbolic_body_reads: false,
        })
    }

    pub(crate) fn command(&self) -> &str {
        &self.source.facts().canonical_command
    }

    pub(crate) fn effective(&self) -> &super::EffectiveCommandWords {
        &self.source.invocation.effective
    }

    pub(crate) fn config(&self) -> tcl_lexer::LexerConfig {
        self.source.config
    }

    /// Original operand value/geometry at the held source point. This is not
    /// a cell or a donated display-name lookup.
    pub(crate) fn owns_operand(&self, original: &crate::ir::WordExpr) -> bool {
        self.tokens.words().iter().any(|word| word == original)
    }

    pub(crate) fn has_closed_scalar_read_context(&self) -> bool {
        self.conditional.is_none()
            && !self.source.has_unknown_residual()
            && authored_script_scalar_context_is_quiet(&self.tokens)
    }

    /// Independent readonly literal advice. Raw source SSA taint stays unknown.
    pub(crate) fn literal_read_coverage(
        &self,
        point: crate::ssa::SsaSourceView<'_>,
        key: Option<crate::ssa::ValueKey>,
    ) -> Option<bool> {
        self.matches_point(point).then_some(())?;
        if self.conditional.is_some() {
            return Some(false);
        }
        let complete = point.original_script_literal_read_coverage(key)?;
        Some(complete && self.has_closed_scalar_read_context())
    }

    pub(crate) fn matches_point(&self, point: crate::ssa::SsaSourceView<'_>) -> bool {
        let Some(tokens) = point.source_tokens() else {
            return false;
        };
        let issued = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_analysis_attachment())
            .zip(
                self.tokens
                    .source_binding
                    .as_ref()
                    .and_then(|binding| binding.original_analysis_attachment()),
            )
            .is_some_and(|(actual, held)| std::sync::Arc::ptr_eq(actual, held));
        issued && tokens == &self.tokens
    }

    /// Only conditional registry classification; this does not call the strict
    /// executed-invocation resolver or reopen a refused Native/hosted owner.
    pub(crate) fn sink_schema<'a>(
        &self,
        registry: &'a CommandRegistry,
        tokens: &CommandTokens,
        effective: Option<&super::EffectiveCommandWords>,
        argument_offset: usize,
        arguments: &[String],
    ) -> Option<(&'a tcl_registry::CommandSpec, String, Vec<String>)> {
        if tokens != &self.tokens
            || effective.is_some_and(|effective| effective != self.effective())
        {
            return None;
        }
        self.source.with_source_schema(registry, |_| Some(()))?;
        let args = self
            .effective()
            .argument_spellings(tokens.argv_texts.get(1..)?)?;
        let selected = args.get(argument_offset..argument_offset.checked_add(arguments.len())?)?;
        (selected == arguments).then_some(())?;
        let command = self.command().to_owned();
        let spec = registry.get(&command)?;
        Some((spec, command, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn original_tokens(source: &str, registry: &CommandRegistry) -> CommandTokens {
        let module = crate::lowering::lower_to_ir(source, registry);
        assert!(module.source_entry.native_entry.is_none());
        let script = &module.top_level;
        script
            .statements
            .iter()
            .find_map(|statement| script.retained_source_tokens_for_statement(statement))
            .expect("actual original source vector")
            .clone()
    }

    #[test]
    fn closed_authored_source_grammar_retains_actual_body_geometry_without_entry() {
        let registry = CommandRegistry::build_default();
        let tokens = original_tokens("catch {set {$literal} VALUE} result", &registry);
        let source = original_authored_source_structure(&tokens, &registry).unwrap();
        let grammar = source
            .into_closed_source_grammar()
            .expect("closed authored grammar");
        assert_eq!(
            grammar.lowering_hook(),
            Some(tcl_registry::hooks::LoweringHookId::Catch)
        );
        assert!(grammar.original_body_word(0).is_some());
        assert!(grammar.original_body_word(1).is_none());
        let wrong = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(grammar.with_source_schema(wrong, |_| Some(())).is_none());
        let observed = crate::lowering::lower_to_ir("unknown; catch {} result", &registry);
        let script = &observed.top_level;
        let tokens = script
            .statements
            .last()
            .and_then(|statement| script.retained_source_tokens_for_statement(statement))
            .expect("actual later source");
        assert!(
            original_authored_source_structure(tokens, &registry)
                .is_none_or(|source| source.into_closed_source_grammar().is_none())
        );
    }

    #[test]
    fn authored_source_roles_require_the_actual_store_and_complete_original_vector() {
        let registry = CommandRegistry::build_default();
        let tokens = original_tokens("info exists {$literal}", &registry);
        let selected = original_authored_source_structure(&tokens, &registry)
            .expect("genuine authored role owner");
        assert!(
            selected
                .facts()
                .traits
                .contains(tcl_registry::Traits::INTROSPECTS_BY_NAME)
        );
        assert_eq!(selected.argument_literal(1).as_deref(), Some("$literal"));
        let retained = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .retained_registry_for_tokens(&tokens)
            .expect("retained actual store");
        assert_eq!(
            retained.snapshot().semantic_key(),
            registry.snapshot().semantic_key()
        );
        let wrong = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(original_authored_source_structure(&tokens, wrong).is_none());
        let mut changed = tokens.clone();
        changed.argv_texts[2] = "different".to_owned();
        assert!(original_authored_source_structure(&changed, &registry).is_none());
        let mut absent = tokens;
        absent.source_binding = None;
        assert!(original_authored_source_structure(&absent, &registry).is_none());
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let profiled = context.commands();
        let supplied = crate::compilation_unit::CompilationUnit::build_for_profile(
            "info exists target",
            profiled,
            false,
            profiled.profile().unwrap(),
        );
        let script = &supplied.ir_module.top_level;
        let supplied_tokens = script
            .statements
            .iter()
            .find_map(|statement| script.retained_source_tokens_for_statement(statement))
            .expect("actual supplied source owner");
        assert!(original_authored_source_structure(supplied_tokens, profiled).is_none());
    }
}

#[cfg(test)]
mod standalone_script_source_variable_tests {
    use super::*;

    fn scalar_point<'a>(
        unit: &'a crate::compilation_unit::CompilationUnit,
        head: &str,
    ) -> crate::ssa::SsaSourceView<'a> {
        for (&block, body) in &unit.top_level.ssa.blocks {
            for index in 0..body.statements.len() {
                let point =
                    crate::ssa::SsaSourceView::at_statement(&unit.top_level.ssa, block, index);
                if point
                    .source_tokens()
                    .and_then(|tokens| tokens.argv_texts.first())
                    .map(String::as_str)
                    == Some(head)
                {
                    return point;
                }
            }
        }
        panic!("original scalar point is retained");
    }

    #[test]
    fn script_source_symbols_retain_original_scalar_definitions_without_cells() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "set x [read $fd]\neval $x",
            registry,
            false,
        );
        let write = scalar_point(&unit, "set");
        let definition =
            original_authored_script_scalar_variable(write.source_tokens().unwrap(), registry)
                .expect("genuine root-script scalar definition");
        assert_eq!(definition.1, "x");
        assert!(definition.2.is_may_write());
        let ssa = &unit.top_level.ssa;
        let symbol = ssa
            .source_variable_symbols()
            .find_map(|(symbol, held)| (held == &definition.0).then_some(symbol))
            .expect("own script source symbol independently of display names");
        assert!(ssa.cell_key(symbol).is_none());
        let statement = unit
            .top_level
            .ssa
            .blocks
            .values()
            .flat_map(|body| &body.statements)
            .find(|statement| statement.defs.contains_key(&symbol))
            .expect("actual source definition participates in SSA");
        assert!(statement.may_defs.contains(&symbol));
        assert!(statement.uses.contains_key(&symbol));
        let version = statement.defs[&symbol];
        assert_eq!(
            unit.top_level.sccp.values.get(&(symbol, version)),
            Some(&crate::analyses::LatticeValue::Overdefined)
        );
        assert!(unit.ir_module.source_entry.metadata_context.is_standalone());
        assert!(unit.ir_module.source_entry.native_entry.is_none());
        assert!(unit.ir_module.source_metadata_input.is_none());
    }

    #[test]
    fn conditional_script_sink_keeps_actual_point_and_refuses_foreign_or_changed_vectors() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let text = "set x [read $fd]\neval $x";
        let unit = crate::compilation_unit::CompilationUnit::build_for(text, registry, false);
        let point = scalar_point(&unit, "eval");
        let tokens = point.source_tokens().unwrap();
        let sink = AuthoredSourceTaintInvocation::capture(tokens, registry).unwrap();
        assert!(sink.matches_point(point));
        assert_eq!(sink.command(), "eval");
        let args = sink
            .effective()
            .argument_spellings(&tokens.argv_texts[1..])
            .unwrap();
        assert!(
            sink.sink_schema(registry, tokens, Some(sink.effective()), 0, &args)
                .is_some()
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .proved_execution_target()
                .is_none()
        );
        let foreign = crate::compilation_unit::CompilationUnit::build_for(text, registry, false);
        assert!(!sink.matches_point(scalar_point(&foreign, "eval")));
        let mut changed = tokens.clone();
        changed.argv_texts[1] = "different".to_owned();
        assert!(
            sink.sink_schema(registry, &changed, Some(sink.effective()), 0, &args)
                .is_none()
        );
        assert!(AuthoredSourceTaintInvocation::capture(&changed, registry).is_none());
        let mut missing = tokens.clone();
        missing.source_binding = None;
        assert!(AuthoredSourceTaintInvocation::capture(&missing, registry).is_none());
        let other = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(
            sink.sink_schema(other, tokens, Some(sink.effective()), 0, &args)
                .is_none()
        );
    }

    #[test]
    fn supplied_source_and_multiple_scalar_accesses_cannot_borrow_root_script_projection() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let supplied = crate::compilation_unit::CompilationUnit::build_for_profile(
            "set x value\neval $x",
            registry,
            false,
            registry.profile().unwrap(),
        );
        let point = scalar_point(&supplied, "eval");
        assert!(
            AuthoredSourceTaintInvocation::capture(point.source_tokens().unwrap(), registry)
                .is_none()
        );
        let mixed =
            crate::compilation_unit::CompilationUnit::build_for("set x $y", registry, false);
        assert!(
            original_authored_script_scalar_variable(
                scalar_point(&mixed, "set").source_tokens().unwrap(),
                registry
            )
            .is_none()
        );
        for text in [
            "eval $x $a(k)",
            "set x $a(k)",
            "set a(k) $x",
            "eval $x [unknown_callback]",
        ] {
            let unit = crate::compilation_unit::CompilationUnit::build_for(text, registry, false);
            let head = if text.starts_with("set ") {
                "set"
            } else {
                "eval"
            };
            let tokens = scalar_point(&unit, head).source_tokens().unwrap();
            assert!(tokens.source_binding.is_some());
            assert!(
                original_authored_script_scalar_variable(tokens, registry).is_none(),
                "{text}"
            );
        }
        let array =
            crate::compilation_unit::CompilationUnit::build_for("set a(k) value", registry, false);
        assert!(
            original_authored_script_scalar_variable(
                scalar_point(&array, "set").source_tokens().unwrap(),
                registry
            )
            .is_none()
        );
    }
    #[test]
    fn possible_root_script_sink_uses_original_argv_without_execution_grant() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "set x [read $fd]\neval $x",
            registry,
            false,
        );
        let warnings =
            crate::taint::find_taint_warnings_for_cu(&unit, registry, registry.profile());
        assert!(
            warnings
                .iter()
                .any(|warning| warning.code.to_string() == "T100"
                    && warning.sink_command == "eval"
                    && warning.variable == "x")
        );
        assert!(unit.ir_module.source_entry.native_entry.is_none());
        assert!(unit.ir_module.source_metadata_input.is_none());
    }
    #[test]
    fn literal_script_write_and_unknown_rhs_keep_distinct_advisory_taint() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (text, tainted) in [
            ("set x VALUE\neval $x", false),
            ("set x [read $fd]\neval $x", true),
        ] {
            let unit = crate::compilation_unit::CompilationUnit::build_for(text, registry, false);
            let point = scalar_point(&unit, "set");
            let definition =
                original_authored_script_scalar_variable(point.source_tokens().unwrap(), registry)
                    .expect("actual root-script definition");
            assert_eq!(definition.1, "x");
            assert_eq!(definition.2.is_may_write(), tainted);
            let symbol = unit
                .top_level
                .ssa
                .source_variable_symbols()
                .find_map(|(symbol, held)| (held == &definition.0).then_some(symbol))
                .expect("genuine source identity participates in the solver");
            assert!(unit.top_level.ssa.cell_key(symbol).is_none());
            let warnings =
                crate::taint::find_taint_warnings_for_cu(&unit, registry, registry.profile());
            assert_eq!(
                warnings.iter().any(|warning| {
                    warning.code.to_string() == "T100"
                        && warning.sink_command == "eval"
                        && warning.variable == "x"
                }),
                tainted,
                "{text}"
            );
            assert!(unit.ir_module.source_entry.native_entry.is_none());
        }
    }

    #[test]
    fn conditional_sink_retains_unknown_obligations_without_scalar_or_clean_facts() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let text = "unknown_host; eval $x";
        let unit = crate::compilation_unit::CompilationUnit::build_for(text, registry, false);
        let point = scalar_point(&unit, "eval");
        let tokens = point.source_tokens().unwrap();
        let (source, candidate) = original_possible_sink_structure(tokens, registry)
            .expect("genuine conditional source coordinates retain unknown applicability");
        assert!(source.has_unknown_residual());
        assert!(!candidate.obligations().is_empty());
        assert!(candidate.matches_registry(registry));
        assert!(unit.ir_module.source_entry.metadata_context.is_standalone());
        assert!(unit.ir_module.source_entry.native_entry.is_none());
        assert!(unit.ir_module.source_metadata_input.is_none());
        let selected = AuthoredSourceTaintInvocation {
            source,
            tokens: tokens.clone(),
            conditional: Some(candidate),
        };
        assert_eq!(selected.command(), "eval");
        assert!(!selected.has_closed_scalar_read_context());
        assert_eq!(selected.literal_read_coverage(point, None), Some(false));
        let arguments = selected
            .effective()
            .argument_spellings(&tokens.argv_texts[1..])
            .unwrap();
        assert!(
            selected
                .sink_schema(registry, tokens, Some(selected.effective()), 0, &arguments)
                .is_some()
        );
        let foreign = crate::compilation_unit::CompilationUnit::build_for(text, registry, false);
        let foreign_point = scalar_point(&foreign, "eval");
        assert!(!selected.matches_point(foreign_point));
        assert_eq!(selected.literal_read_coverage(foreign_point, None), None);
        let mut changed = tokens.clone();
        changed.argv_texts[1] = "donated".to_owned();
        assert!(original_possible_sink_structure(&changed, registry).is_none());
        assert!(
            selected
                .sink_schema(
                    registry,
                    &changed,
                    Some(selected.effective()),
                    0,
                    &arguments
                )
                .is_none()
        );
        let other = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(original_possible_sink_structure(tokens, other).is_none());
        assert!(
            selected
                .sink_schema(other, tokens, Some(selected.effective()), 0, &arguments)
                .is_none()
        );
    }

    #[test]
    fn conditional_sink_does_not_borrow_known_document_replacements() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "proc eval args {}; eval $x",
            registry,
            false,
        );
        let point = scalar_point(&unit, "eval");
        assert!(
            original_possible_sink_structure(point.source_tokens().unwrap(), registry).is_none()
        );
    }
}

impl AuthoredSourceTaintInvocation {
    /// Possible syntax inside its original authored body; no actual handler,
    /// successful completion, sanitiser or Native value follows.
    pub(crate) fn capture_authored_body(
        tokens: &CommandTokens,
        scope: &super::OriginalAuthoredSourceBodyScope,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let original_words = scope.original_words(tokens, registry)?;
        if original_words.iter().any(|word| word.group().expand) {
            return None;
        }
        let facts = scope.with_source_schema(tokens, registry, |schema| {
            let facts = schema.facts();
            (facts.arity_accepts_frozen_arguments() != Some(false)).then_some(facts)
        })?;
        let effective =
            super::compose_original_effective_words(tokens, &facts.canonical_command, &[])?;
        let dialect = scope.original_word_dialect();
        let evaluated_words = effective
            .words
            .iter()
            .skip(1)
            .map(|word| {
                super::effective_invocation_word(
                    word,
                    scope.config().escapes,
                    tcl_syntax::word_rules::WordValueRules::from_config(&scope.config()),
                )
            })
            .collect::<Vec<_>>();
        let invocation = ResolvedStatementInvocation {
            dialect,
            facts,
            arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
            evaluated_arguments: evaluated_words
                .iter()
                .map(|word| match word {
                    super::EffectiveInvocationWord::Literal(value) => Some(value.clone()),
                    _ => None,
                })
                .collect(),
            evaluated_words,
            effective,
            authored_case_body_selection: None,
        };
        Some(Self {
            source: AuthoredSourceStructuredInvocation {
                invocation,
                config: scope.config(),
                unknown_residual: true,
                registry: registry.snapshot(),
                realm: scope.source_realm(),
                namespace: scope.namespace().clone(),
                original_words,
            },
            tokens: tokens.clone(),
            conditional: None,
            symbolic_body_reads: true,
        })
    }

    pub(crate) fn uses_symbolic_body_reads(&self) -> bool {
        self.symbolic_body_reads
    }
}
