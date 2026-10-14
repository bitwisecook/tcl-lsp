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
