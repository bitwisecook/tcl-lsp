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

//! Original definition-script coordinates supply no callable or entered frame.

use super::{CommandTokens, Module, ProcedureId};
use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::SourceAnalysisEntry;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};

/// Selected source structure from the actual definition-script allocator.
#[derive(Debug, Clone)]
pub struct OriginalDefinitionScriptSource {
    id: ProcedureId,
    image: SourceImage,
    input: ResolvedAnalysisInput,
    entry: SourceAnalysisEntry,
    registry: RegistrySemanticKey,
    config: LexerConfig,
    tokens: CommandTokens,
    words: Vec<NativeWord>,
    body: usize,
    content: Span,
    declaration: Span,
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    descriptor: tcl_registry::CanonicalCommandDescriptorKey<'static>,
}

impl PartialEq for OriginalDefinitionScriptSource {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.image == other.image
            && self.input == other.input
            && self.entry == other.entry
            && self.registry == other.registry
            && self.config == other.config
            && self.tokens == other.tokens
            && self.words == other.words
            && self.body == other.body
            && self.content == other.content
            && self.declaration == other.declaration
            && std::ptr::eq(self.grammar, other.grammar)
            && self.descriptor == other.descriptor
    }
}

impl OriginalDefinitionScriptSource {
    fn capture(
        module: &Module,
        id: ProcedureId,
        tokens: &CommandTokens,
        body: usize,
        offset: u32,
        grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
        descriptor: tcl_registry::CanonicalCommandDescriptorKey<'static>,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let input = module.source_metadata_input.as_ref()?;
        if !input.has_logical_source_name_context()
            || module.source_entry.native_entry.is_some()
            || module.source_entry.unknown_entry
            || !module.procedure_ids.owns(id)
            || !tokens.original_words_match_argv_text()
            || tokens.synthetic.is_some()
            || module.registry_snapshot.as_ref()?.semantic_key()
                != registry.snapshot().semantic_key()
            || input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
        {
            return None;
        }
        let config = tokens.native_lexer_config(module.native_lexer_config());
        if config.nested() != input.lexer_config().nested() {
            return None;
        }
        let first = tokens
            .words()
            .first()?
            .original_written_span(module.source.bytes())?;
        let last = tokens
            .words()
            .last()?
            .original_written_span(module.source.bytes())?;
        let declaration = Span::new(first.start(), last.end());
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            module.source.clone(),
            declaration,
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        if command.words.len() != tokens.words().len()
            || command.words.iter().any(|word| word.group().expand)
            || !command
                .words
                .iter()
                .zip(tokens.words())
                .all(|(original, represented)| {
                    represented.original_written_span(module.source.bytes())
                        == Some(original.word_span())
                })
        {
            return None;
        }
        let content = crate::registry_invocation::original_written_script_body_span(
            command.words.get(body)?,
        )?;
        if content.start() != offset {
            return None;
        }
        Some(Self {
            id,
            image: module.source.clone(),
            input: input.clone(),
            entry: module.source_entry.clone(),
            registry: registry.snapshot().semantic_key(),
            config,
            tokens: tokens.clone(),
            words: command.words.clone(),
            body,
            content,
            declaration,
            grammar,
            descriptor,
        })
    }

    fn matches(&self, module: &Module, id: ProcedureId, registry: &CommandRegistry) -> bool {
        self.id == id
            && module.procedure_ids.owns(id)
            && self.image == module.source
            && module.source_metadata_input.as_ref() == Some(&self.input)
            && self.input.has_logical_source_name_context()
            && self.entry == module.source_entry
            && self.registry == registry.snapshot().semantic_key()
            && module
                .registry_snapshot
                .as_ref()
                .is_some_and(|held| held.semantic_key() == self.registry)
            && self.config.nested() == module.native_lexer_config().nested()
            && self.tokens.original_words_match_argv_text()
            && self
                .words
                .iter()
                .all(|word| word.image() == &module.source && word.config() == self.config)
    }

    /// Whole original definer words; selection and execution are separate.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.words
    }
    /// Exact selected original definition-body word.
    #[must_use]
    pub fn body_word(&self) -> &NativeWord {
        &self.words[self.body]
    }
    /// Exact script-content coordinate issued with this definition.
    #[must_use]
    pub const fn content_span(&self) -> Span {
        self.content
    }
    /// Complete original definer command extent.
    #[must_use]
    pub const fn declaration_span(&self) -> Span {
        self.declaration
    }
    /// Actual selected definition family; no provider or object allocation claim.
    #[must_use]
    pub const fn family(&self) -> tcl_registry::definer::DefinerFamily {
        self.grammar.family
    }
}

impl Module {
    pub(crate) fn retain_definition_script_source(
        &mut self,
        tokens: &CommandTokens,
        body: usize,
        offset: u32,
        grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
        descriptor: tcl_registry::CanonicalCommandDescriptorKey<'static>,
        registry: &CommandRegistry,
    ) -> ProcedureId {
        let id = self.procedure_ids.allocate();
        if let Some(source) = OriginalDefinitionScriptSource::capture(
            self, id, tokens, body, offset, grammar, descriptor, registry,
        ) {
            self.original_definition_scripts
                .insert(id, std::sync::Arc::new(source));
        }
        id
    }

    /// Reborrow an actually issued current original definition-script coordinate.
    /// This supplies no callable body, namespace publication or physical frame.
    #[must_use]
    pub fn original_definition_script_source(
        &self,
        id: ProcedureId,
        registry: &CommandRegistry,
    ) -> Option<&OriginalDefinitionScriptSource> {
        let source = self.original_definition_scripts.get(&id)?;
        source.matches(self, id, registry).then_some(())?;
        let selected = crate::registry_invocation::source_structure::original_logical_registry_words_for_module(
            self, registry, &source.tokens,
        )?;
        selected
            .written_argument_roles()
            .contains(&(source.body.checked_sub(1)?, tcl_registry::ArgRole::Body))
            .then_some(())?;
        selected.with_source_schema(source.input.borrowed_context_registry(), |schema| {
            let grammar = schema.authored_source_definition_body_grammar()?;
            (schema.descriptor_key() == source.descriptor
                && std::ptr::eq(grammar, source.grammar)
                && schema
                    .authored_source_script_arguments()?
                    .contains(&source.body.checked_sub(1)?))
            .then_some(source)
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};

    fn original_module(source: &str) -> Module {
        let analysis = Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let registry = input.borrowed_context_registry().commands();
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        )
        .ir_module
    }

    #[test]
    fn original_definition_script_retains_issued_whole_source_without_body_or_frame_publication() {
        let source = "oo::class create C {method m {} {return VALUE}}";
        let module = original_module(source);
        let registry = module.resolved_registry();
        let method = module
            .methods
            .values()
            .next()
            .expect("actual selected method");
        let id = method.definition_scope.unwrap();
        let record = module
            .original_definition_script_source(id, registry)
            .expect("actual definition allocator source");
        assert!(module.procedure_ids.owns(id));
        assert!(!module.procedures.contains_key(&id));
        assert!(!module.body_units.contains_key(&id));
        assert_eq!(
            &source[record.content_span().as_range()],
            "method m {} {return VALUE}"
        );
        assert_eq!(record.original_words().len(), 4);
        assert_eq!(record.family(), tcl_registry::definer::DefinerFamily::TclOo);

        let mut moved = module.clone();
        let issued = moved.procedure_ids.allocate();
        moved
            .original_definition_scripts
            .insert(issued, module.original_definition_scripts[&id].clone());
        assert!(
            moved
                .original_definition_script_source(issued, registry)
                .is_none()
        );
        let mut foreign = original_module(source);
        foreign
            .original_definition_scripts
            .insert(id, module.original_definition_scripts[&id].clone());
        assert!(
            foreign
                .original_definition_script_source(id, registry)
                .is_none()
        );
        let mut changed = module.clone();
        changed.source = SourceImage::document(source.replace("VALUE", "OTHER"));
        assert!(
            changed
                .original_definition_script_source(id, registry)
                .is_none()
        );
        let mut changed = module.clone();
        changed.source_entry.unknown_entry = true;
        assert!(
            changed
                .original_definition_script_source(id, registry)
                .is_none()
        );
        let mut changed = module.clone();
        changed.source_metadata_input = None;
        assert!(
            changed
                .original_definition_script_source(id, registry)
                .is_none()
        );
    }
}
