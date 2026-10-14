// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original table absence and independently retained source mutation history.

use super::{MayBinding, ModuleCommandBindings, SourceInvocationBinding, SourceNamespaceKey};
use crate::ir::{CommandTokens, Module};
use tcl_registry::CommandRegistry;

/// Original table absence joined to the same Module's source mutation history.
/// This owner supplies no unknown-handler target or executed dispatch.
pub(crate) struct OriginalMissingCommandContext<'a> {
    module: &'a Module,
    registry: &'a CommandRegistry,
    history: ModuleCommandBindings,
}

impl<'a> OriginalMissingCommandContext<'a> {
    pub(crate) fn for_module(module: &'a Module, registry: &'a CommandRegistry) -> Option<Self> {
        let retained = module.retained_source_bindings.as_deref()?;
        let positioned = retained.positioned_for_module(module, registry)?;
        module.source_metadata_input.as_ref()?;
        let history = positioned.module_projection(module);
        (!history.dynamic_proc_binding && !history.unnameable_rebinding_subject).then_some(Self {
            module,
            registry,
            history,
        })
    }

    pub(crate) fn missing_rebound_command(&self, tokens: &CommandTokens) -> bool {
        tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_missing_rebound_command(tokens, self))
            .unwrap_or(false)
    }
}

impl SourceInvocationBinding {
    fn original_missing_rebound_command(
        &self,
        tokens: &CommandTokens,
        context: &OriginalMissingCommandContext<'_>,
    ) -> Option<bool> {
        let module = context.module;
        let registry = context.registry;
        let history = &context.history;
        if tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || tokens.source_binding.as_ref() != Some(self)
        {
            return None;
        }
        let retained = module.retained_source_bindings.as_deref()?;
        let input = module.source_metadata_input.as_ref()?;
        if !input.has_logical_source_name_context()
            && self.native_compilation_selection()
                != tcl_registry::native_compilation::NativeCompilationSelection::Generic
        {
            return None;
        }
        if input.has_logical_source_name_context() {
            let rows = super::declaration_layout::original_declaration_layouts(
                self.declaration_layout_observations.as_deref()?,
            )?;
            if !rows.clone().all(|row| {
                row.words.as_ref() == tokens.words()
                    && row.config == module.native_lexer_config()
                    && self.original_operands_preserve_lookup(tokens, registry, row, 0)
            }) {
                return None;
            }
        }
        if let Some(snapshot) = self.lookup_state.as_deref() {
            retained.owns_original_tokens(tokens).then_some(())?;
            self.original_invocation_metadata_for_module(tokens, module, registry)?;
            return missing_rebound_at(
                self,
                tokens,
                &snapshot.state,
                &self.lookup_namespace_key,
                history,
                input,
                registry,
            );
        }
        input.has_logical_source_name_context().then_some(())?;
        let site = self.invocation_site()?;
        if site.source.source_image() != &module.source {
            return None;
        }
        let rows = super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let mut agreed = None;
        for row in rows {
            if row.words.as_ref() != tokens.words()
                || row.config != module.native_lexer_config()
                || row.snapshot.state.logical_source_name_advice_input() != Some(input)
                || !row.entry.owns_source(&site.source, site.offset)
            {
                return None;
            }
            let result = missing_rebound_at(
                self,
                tokens,
                &row.snapshot.state,
                &row.namespace,
                history,
                input,
                registry,
            )?;
            if agreed.is_some_and(|previous| previous != result) {
                return None;
            }
            agreed = Some(result);
        }
        agreed
    }
}

fn missing_rebound_at(
    binding: &SourceInvocationBinding,
    tokens: &CommandTokens,
    state: &ModuleCommandBindings,
    namespace: &SourceNamespaceKey,
    history: &ModuleCommandBindings,
    input: &crate::analyser::ResolvedAnalysisInput,
    registry: &CommandRegistry,
) -> Option<bool> {
    if state.has_opaque_domain()
        || state.source_step_observed()
        || !state.command_observers.is_quiet()
    {
        return None;
    }
    let keys = if input.has_logical_source_name_context() {
        if !matches!(namespace, SourceNamespaceKey::Authored(_)) {
            return None;
        }
        let site = binding.invocation_site()?;
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        if config != input.lexer_config()
            || state.logical_source_name_advice_input() != Some(input)
            || state.current_source_origin.as_ref() != Some(&site.source)
            || input
                .context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
        {
            return None;
        }
        let words = crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            config,
        )?;
        let policy =
            super::source_analysis_entry::source_input_dialect(input).authored_name_policy()?;
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        let key = purpose.key(
            words.first()?,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
        )?;
        let name = std::str::from_utf8(key.bytes()).ok()?;
        state.source_keys_checked(name, namespace).ok()?
    } else {
        let head = binding.original_head_name_input(tokens)?;
        state.original_command_keys_for_input(namespace, &head)?
    };
    if keys.is_empty() {
        return None;
    }
    let mut mutated = false;
    for key in keys {
        if state.original_bindings_for_key(&key)? != super::BTreeSet::from([MayBinding::Missing]) {
            return Some(false);
        }
        mutated |= history.rebound_names.contains(&key);
    }
    Some(mutated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(source: &str) -> crate::compilation_unit::CompilationUnit {
        let context =
            crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
    }

    fn last_tokens(unit: &crate::compilation_unit::CompilationUnit) -> &CommandTokens {
        let script = &unit.ir_module.top_level;
        script
            .statements
            .iter()
            .rev()
            .find_map(|statement| script.retained_source_tokens_for_statement(statement))
            .expect("complete original final fixture command")
    }

    #[test]
    fn original_missing_mutated_slot_requires_current_complete_source() {
        // naming.source.retained-positioned-command-realm
        // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
        let unit = unit("proc old {} {}; rename old {}; old");
        let registry = unit.ir_module.resolved_registry();
        let context = OriginalMissingCommandContext::for_module(&unit.ir_module, registry)
            .expect("genuine supplied Logical Module");
        let tokens = last_tokens(&unit);
        assert!(context.missing_rebound_command(tokens));

        let mut altered = tokens.clone();
        altered.argv_texts.push("changed".into());
        assert!(!context.missing_rebound_command(&altered));
        let mut missing = tokens.clone();
        missing.source_binding = None;
        assert!(!context.missing_rebound_command(&missing));
        let mut changed = unit.ir_module.clone();
        changed.source = tcl_lexer::SourceImage::document("proc old {} {}; rename old {}; other");
        assert!(OriginalMissingCommandContext::for_module(&changed, registry).is_none());
    }

    #[test]
    fn source_absence_does_not_turn_external_or_dynamic_heads_into_rebound_slots() {
        // naming.source.retained-positioned-command-realm
        // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
        for source in [
            "proc old {} {}; rename old {}; external",
            "proc old {} {}; rename old {}; $head",
            "proc old {} {}; rename old {}; proc old {} {}; old",
            "proc old {} {}; rename $name {}; old",
            "proc old {} {}; rename old {}; old [proc old {} {}]",
        ] {
            let unit = unit(source);
            let registry = unit.ir_module.resolved_registry();
            if let Some(context) =
                OriginalMissingCommandContext::for_module(&unit.ir_module, registry)
            {
                assert!(
                    !context.missing_rebound_command(last_tokens(&unit)),
                    "{source}"
                );
            }
        }
    }
}
