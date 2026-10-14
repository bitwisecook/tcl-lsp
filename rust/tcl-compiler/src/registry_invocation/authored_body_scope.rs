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
        }))
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
