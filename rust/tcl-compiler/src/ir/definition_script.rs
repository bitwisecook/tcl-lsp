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

pub(crate) struct DefinitionScriptLayout {
    pub body: usize,
    pub name: usize,
    pub per_object: bool,
    pub offset: u32,
    pub grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    pub descriptor: tcl_registry::CanonicalCommandDescriptorKey<'static>,
}

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
    name: usize,
    per_object: bool,
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
            && self.name == other.name
            && self.per_object == other.per_object
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
        layout: &DefinitionScriptLayout,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let DefinitionScriptLayout {
            body,
            name,
            per_object,
            offset,
            grammar,
            descriptor,
        } = *layout;
        let input = module.source_metadata_input.as_ref()?;
        if !input.has_logical_source_name_context()
            || module.source_entry.native_entry.is_some()
            || module.source_entry.unknown_entry
            || !module.procedure_ids.owns(id)
            || name >= tokens.words().len()
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
            name,
            per_object,
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

    pub(crate) fn matches_source_input(
        &self,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        &self.image == image
            && &self.input == input
            && self.config.nested() == input.lexer_config().nested()
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
        layout: &DefinitionScriptLayout,
        registry: &CommandRegistry,
    ) -> ProcedureId {
        let id = self.procedure_ids.allocate();
        if let Some(source) =
            OriginalDefinitionScriptSource::capture(self, id, tokens, layout, registry)
        {
            self.original_definition_scripts
                .insert(id, std::sync::Arc::new(source));
        }
        id
    }

    pub(crate) fn original_class_definition_body_source(
        &self,
        class: &crate::analyser::ClassDef,
        registry: &CommandRegistry,
    ) -> Option<OriginalDefinitionScriptSource> {
        let mut selected = self.original_definition_scripts.keys().filter_map(|id| {
            let source = self.original_definition_script_source(*id, registry)?;
            if source.body_word().tokens().first()?.span != class.body_span {
                return None;
            }
            if source.per_object
                || source.words.get(source.name)?.tokens().first()?.span != class.name_span
            {
                return None;
            }
            Some(source)
        });
        let source = selected.next()?;
        selected.next().is_none().then(|| source.clone())
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
        let layout =
            crate::registry_invocation::original_source_definition_layout_from_selected_words(
                &selected,
                &source.input,
                &source.tokens,
            )?;
        (layout.name_word == source.name
            && layout.body_word == source.body
            && layout.per_object == source.per_object
            && layout.descriptor == source.descriptor
            && std::ptr::eq(layout.grammar, source.grammar))
        .then_some(source.as_ref())
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
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        )
        .ir_module
    }

    #[test]
    fn class_body_source_joins_original_name_and_body_to_the_issued_definition() {
        let source = "oo::class create C {method m {} {return VALUE}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let module = original_module(source);
        let registry = analysis.resolved_registry().unwrap();
        let class = &analysis.all_classes["::C"];
        assert!(
            module
                .original_class_definition_body_source(class, registry)
                .is_some()
        );
        let mut changed = class.clone();
        changed.name_span = Span::new(0, 1);
        assert!(
            module
                .original_class_definition_body_source(&changed, registry)
                .is_none()
        );
        let mut changed = class.clone();
        changed.body_span = Span::new(0, 1);
        assert!(
            module
                .original_class_definition_body_source(&changed, registry)
                .is_none()
        );
        let mut changed = module.clone();
        changed.source_metadata_input = None;
        assert!(
            changed
                .original_class_definition_body_source(class, registry)
                .is_none()
        );
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
        changed.source = SourceImage::document(&source.replace("VALUE", "OTHER"));
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
    #[test]
    fn issued_definition_reborrow_rejects_changed_selected_name_and_object_mode() {
        let source = "oo::class create C {method m {} {return VALUE}}";
        let module = original_module(source);
        let registry = module.resolved_registry();
        let id = module
            .methods
            .values()
            .next()
            .unwrap()
            .definition_scope
            .unwrap();
        assert!(
            module
                .original_definition_script_source(id, registry)
                .is_some()
        );
        let mut changed = module.clone();
        std::sync::Arc::make_mut(changed.original_definition_scripts.get_mut(&id).unwrap()).name =
            0;
        assert!(
            changed
                .original_definition_script_source(id, registry)
                .is_none()
        );
        let mut changed = module.clone();
        std::sync::Arc::make_mut(changed.original_definition_scripts.get_mut(&id).unwrap())
            .per_object = true;
        assert!(
            changed
                .original_definition_script_source(id, registry)
                .is_none()
        );
    }
}
