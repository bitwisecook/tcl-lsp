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

//! Readonly Logical member declarations from the actual completed class walk.

use super::{AnalysisResult, ClassDef, MemberSide, MethodDef};
use crate::analyser::ResolvedAnalysisInput;
use std::sync::Arc;
use tcl_lexer::SourceImage;

/// A completed Logical class declaration, independent of Native name recipes.
/// The snapshot supplies source membership only, never dispatch or editing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceClassDeclaration {
    image: SourceImage,
    input: ResolvedAnalysisInput,
    class: Arc<ClassDef>,
    body_word: Option<tcl_lexer::NativeWord>,
    candidate_coordinate: Option<tcl_core_types::ByteCommandSlot>,
    name_word: Option<tcl_lexer::NativeWord>,
    relations: Option<Vec<crate::command_binding::LogicalSourceClassRelationProvider>>,
}

/// One exact side and declaration in the retained Logical class snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceMemberDeclaration {
    class: LogicalSourceClassDeclaration,
    side: MemberSide,
    method: MethodDef,
}

/// One actual Registry relation slot operation with independently selected operands.
/// The carrier preserves source order and grants no executed member-table effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceRelationStatement<T> {
    relation: tcl_registry::definer::RelationSlot,
    slot: tcl_registry::definer::SlotSpec,
    operation: tcl_registry::definer::SlotOp,
    operands: Vec<T>,
}
impl<T> LogicalSourceRelationStatement<T> {
    /// Actual selected Registry relation family, independently of spelling.
    #[must_use]
    pub const fn relation(&self) -> tcl_registry::definer::RelationSlot {
        self.relation
    }
    /// Retained whole-operand selections under this original member statement.
    #[must_use]
    pub fn operands(&self) -> &[T] {
        &self.operands
    }
    /// Delegate typed values to the same authored Registry slot operation.
    pub fn apply_values<V: Clone + PartialEq>(&self, current: &mut Vec<V>, values: &[V]) {
        self.slot.apply_values(current, self.operation, values);
    }
}

/// Original static member-name data and its independently quoted source word.
/// This readonly suggestion supplies no method dispatch or editing permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceMemberInsertion {
    name: String,
    word: String,
}
impl LogicalSourceMemberInsertion {
    /// Exact original static name, independent of a display-map key.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// One quoted word under the retained original source grammar.
    #[must_use]
    pub fn source_word(&self) -> &str {
        &self.word
    }
}

impl LogicalSourceClassDeclaration {
    /// Retained original class metadata snapshot; reporting labels do not select it.
    #[must_use]
    pub fn metadata(&self) -> &ClassDef {
        &self.class
    }

    /// Immutable original image and complete retained input correspondence.
    /// This supplies no current class, installed member or execution grant.
    pub(crate) fn matches_source(
        &self,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        input.has_logical_source_name_context() && self.image == *image && self.input == *input
    }

    pub(crate) fn capture(
        image: SourceImage,
        input: ResolvedAnalysisInput,
        class: &ClassDef,
    ) -> Option<Self> {
        let len = u32::try_from(image.bytes().len()).ok()?;
        let bounded = |span: tcl_lexer::Span| span.start() < span.end() && span.end() <= len;
        (input.has_logical_source_name_context()
            && bounded(class.name_span)
            && bounded(class.body_span))
        .then_some(())?;
        Some(Self {
            image,
            input,
            class: Arc::new(class.clone()),
            body_word: None,
            candidate_coordinate: None,
            name_word: None,
            relations: None,
        })
    }

    fn matches(&self, analysis: &AnalysisResult, class: &ClassDef) -> bool {
        if !analysis.allows_retained_logical_declaration_advice()
            || analysis.resolved_input.as_ref() != Some(&self.input)
            || !analysis.matches_original_source_image(&self.image, self.input.lexer_config())
            || !analysis
                .retained_command_realm()
                .is_some_and(|realm| realm.matches_resolved_analysis_input(&self.input))
        {
            return false;
        }
        let mut current = class.clone();
        current
            .qualified_name
            .clone_from(&self.class.qualified_name);
        current == *self.class
    }

    /// Original body selected by the actual definition-script source factory.
    /// It supplies syntax without class allocation, member execution or editing.
    #[must_use]
    pub fn original_body_word(
        &self,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> Option<&tcl_lexer::NativeWord> {
        if image != &self.image || input != &self.input {
            return None;
        }
        let word = self.body_word.as_ref()?;
        (word.image() == image && word.config().nested() == input.lexer_config().nested())
            .then_some(word)
    }

    /// Exact selected original factory Name coordinate for readonly candidates.
    /// The factory's own original name/body correspondence is independent of
    /// the Module-issued body syntax facet and supplies no class identity.
    #[must_use]
    pub fn source_candidate_coordinate(&self) -> Option<&tcl_core_types::ByteCommandSlot> {
        self.candidate_coordinate.as_ref()
    }

    /// Actual naming operand of this retained source factory.
    #[must_use]
    pub fn original_name_word(&self) -> Option<&tcl_lexer::NativeWord> {
        self.name_word.as_ref()
    }

    /// Exact retained member metadata on the selected source side.
    #[must_use]
    pub fn member(
        &self,
        side: MemberSide,
        method: &MethodDef,
    ) -> Option<LogicalSourceMemberDeclaration> {
        let members = match side {
            MemberSide::Instance => &self.class.methods,
            MemberSide::ClassObject => &self.class.class_methods,
        };
        let mut matches = members.values().filter(|candidate| *candidate == method);
        let selected = matches.next()?;
        matches
            .next()
            .is_none()
            .then(|| LogicalSourceMemberDeclaration {
                class: self.clone(),
                side,
                method: selected.clone(),
            })
    }
}

impl LogicalSourceMemberDeclaration {
    /// Exact retained class snapshot; the completed owner authenticates it.
    #[must_use]
    pub const fn class(&self) -> &LogicalSourceClassDeclaration {
        &self.class
    }
    /// Original member side, separate from execution or dispatch.
    #[must_use]
    pub const fn side(&self) -> MemberSide {
        self.side
    }
    /// Current snapshot metadata for report geometry only.
    #[must_use]
    pub const fn metadata(&self) -> &MethodDef {
        &self.method
    }
    /// Actual whole naming operand from the retained Registry member vector.
    #[must_use]
    pub fn original_name_word(&self) -> Option<&tcl_lexer::NativeWord> {
        self.class.class.original_members.logical_name_operand(
            self.side,
            &self.method,
            &self.class.input,
            &self.class.image,
        )
    }
    /// Exact original body operand from the retained Registry member vector.
    /// This supplies source syntax, never an entered receiver or method frame.
    #[must_use]
    pub fn original_body_word(&self) -> Option<&tcl_lexer::NativeWord> {
        self.class.class.original_members.logical_body_operand(
            self.side,
            &self.method,
            &self.class.input,
            &self.class.image,
        )
    }
}

impl AnalysisResult {
    pub(crate) fn retain_logical_class_body_sources(
        &mut self,
        module: &crate::ir::Module,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let sources = self
            .logical_class_declarations
            .iter()
            .map(|declaration| {
                let class = self
                    .all_classes
                    .values()
                    .find(|class| declaration.matches(self, class))?;
                let body = module
                    .original_class_definition_body_source(class, registry)
                    .filter(|source| {
                        source.matches_source_input(&declaration.image, &declaration.input)
                    })
                    .map(|source| source.body_word().clone());
                let factory = self.logical_class_factory(class, &declaration.image);
                let coordinate = factory
                    .as_ref()
                    .map(|factory| factory.source_slot().clone());
                let name_word = factory
                    .as_ref()
                    .and_then(|factory| factory.name_input().logical_input())
                    .map(|name| name.original_word().clone());
                let relations = factory
                    .as_ref()
                    .and_then(|factory| self.logical_relation_providers(class, factory, registry));
                Some((body, coordinate, name_word, relations))
            })
            .collect::<Vec<_>>();
        for (declaration, source) in self.logical_class_declarations.iter_mut().zip(sources) {
            if let Some((body, coordinate, name_word, relations)) = source {
                declaration.body_word = body;
                declaration.candidate_coordinate = coordinate;
                declaration.name_word = name_word;
                declaration.relations = relations;
            } else {
                declaration.body_word = None;
                declaration.candidate_coordinate = None;
                declaration.name_word = None;
                declaration.relations = None;
            }
        }
    }

    /// A current selected member and genuine whole name word supply insertion data.
    /// Computed/generated names without a static original operand remain unavailable.
    #[must_use]
    pub fn logical_source_member_insertion(
        &self,
        class: &ClassDef,
        side: MemberSide,
        method: &MethodDef,
    ) -> Option<LogicalSourceMemberInsertion> {
        let class_owner = self.logical_source_class_declaration(class)?;
        let members = match side {
            MemberSide::Instance => &class.methods,
            MemberSide::ClassObject => &class.class_methods,
        };
        members
            .values()
            .any(|current| std::ptr::eq(current, method))
            .then_some(())?;
        class_owner.member(side, method)?;
        let config = class_owner.input.lexer_config();
        let original = class
            .original_members
            .logical_name_operand(side, method, &class_owner.input, &class_owner.image)?
            .clone();
        let value = tcl_syntax::word_rules::original_static_word_ascii_presentation(&original)?;
        if value.contains(&0) || value.as_slice() != method.name.as_bytes() {
            return None;
        }
        let name = String::from_utf8(value).ok()?;
        let word = tcl_syntax::word_rules::lexical_ascii_source_word(&name, config)?;
        Some(LogicalSourceMemberInsertion { name, word })
    }

    /// Readonly owning-type advice for an original member declaration or a
    /// positioned lexical self selector. Source membership supplies no dispatch.
    #[must_use]
    pub fn logical_source_member_type_at(
        &self,
        image: &SourceImage,
        config: tcl_lexer::LexerConfig,
        cursor: u32,
    ) -> Option<&ClassDef> {
        let selected = self.logical_source_member_at(image, config, cursor)?;
        self.logical_source_member_class(&selected)
    }

    /// A selected original declaration or genuine self selector in the current
    /// Logical source. The receipt supplies no dispatch, lookup closure or edit.
    #[must_use]
    pub fn logical_source_member_at(
        &self,
        image: &SourceImage,
        config: tcl_lexer::LexerConfig,
        cursor: u32,
    ) -> Option<LogicalSourceMemberDeclaration> {
        let input = self.resolved_input.as_ref()?;
        if input.lexer_config() != config
            || !self.matches_original_source_image(image, config)
            || !self.allows_retained_logical_declaration_advice()
        {
            return None;
        }
        match self
            .logical_member_declarations_at(image, input, cursor)
            .as_slice()
        {
            [selected] => return Some(selected.clone()),
            [] => {}
            _ => return None,
        }
        self.logical_self_selector_at(image, config, cursor, input)
    }

    /// Original declaring member behind the actual retained source frame at
    /// this occurrence. Unrepresented bodies cannot infer a caller from spans.
    #[must_use]
    pub fn logical_source_member_frame_at(
        &self,
        image: &SourceImage,
        config: tcl_lexer::LexerConfig,
        cursor: u32,
    ) -> Option<LogicalSourceMemberDeclaration> {
        let input = self.resolved_input.as_ref()?;
        if input.lexer_config() != config
            || !self.matches_original_source_image(image, config)
            || !self.allows_retained_logical_declaration_advice()
        {
            return None;
        }
        let bindings = self.retained_command_realm()?.source_bindings_ref();
        let frame =
            crate::command_binding::SourceCommandBindings::logical_source_member_frame_at_offset(
                bindings, input, image, cursor, config,
            )?;
        if !matches!(
            frame.frame().layout(),
            crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
        ) {
            return None;
        }
        let mut selected = None;
        for class in self.all_classes.values() {
            let Some(owner) = self.logical_source_class_declaration(class) else {
                continue;
            };
            for (side, members) in [
                (MemberSide::Instance, &class.methods),
                (MemberSide::ClassObject, &class.class_methods),
            ] {
                for method in members.values() {
                    let matches = class
                        .original_members
                        .logical_body_operand(side, method, input, image)
                        .and_then(|body| body.content_span().ok())
                        .is_some_and(|content| {
                            content.start() == frame.source().base()
                                && image
                                    .try_text()
                                    .ok()
                                    .and_then(|source| source.get(content.as_range()))
                                    == frame.source().try_text().ok()
                        });
                    if matches {
                        let member = owner.member(side, method)?;
                        if selected.is_some() {
                            return None;
                        }
                        selected = Some(member);
                    }
                }
            }
        }
        selected
    }

    fn logical_member_declarations_at(
        &self,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
        cursor: u32,
    ) -> Vec<LogicalSourceMemberDeclaration> {
        let contains =
            |word: &tcl_lexer::NativeWord| word.span().as_range().contains(&(cursor as usize));
        let mut selected = Vec::new();
        for class in self.all_classes.values() {
            let Some(owner) = self.logical_source_class_declaration(class) else {
                continue;
            };
            for (side, members) in [
                (MemberSide::Instance, &class.methods),
                (MemberSide::ClassObject, &class.class_methods),
            ] {
                for method in members.values() {
                    let Some(name) = class
                        .original_members
                        .logical_name_operand(side, method, input, image)
                    else {
                        continue;
                    };
                    if let Some(member) = owner.member(side, method)
                        && contains(name)
                    {
                        selected.push(member);
                    }
                }
            }
        }
        selected
    }

    fn logical_self_selector_at(
        &self,
        image: &SourceImage,
        config: tcl_lexer::LexerConfig,
        cursor: u32,
        input: &ResolvedAnalysisInput,
    ) -> Option<LogicalSourceMemberDeclaration> {
        let realm = self.retained_command_realm()?;
        let registry = self.resolved_registry()?;
        let bindings = realm.source_bindings_ref();
        let contains =
            |word: &tcl_lexer::NativeWord| word.span().as_range().contains(&(cursor as usize));
        let mut selected = None;
        let caller = self.logical_source_member_frame_at(image, config, cursor)?;
        let caller_class = self.logical_source_member_class(&caller)?;
        for invocation in &self.command_invocations {
            let Some(command) = bindings.original_logical_source_command_at_in_source(
                input,
                image,
                invocation.range.start(),
                config,
            ) else {
                continue;
            };
            let Some(selector) = command.native_words().get(1).filter(|word| contains(word)) else {
                continue;
            };
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &image.source_map(),
                config,
                command.command(),
            );
            realm.stamp_original_tokens(&mut tokens);
            let context =
                crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                    registry, input,
                )?;
            let resolved =
                crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                    registry,
                    context,
                    &tokens,
                    Some(bindings),
                )?;
            if registry.method_dispatch_keyword(resolved.canonical_command())
                != Some(tcl_registry::MethodDispatchKind::SelfDispatch)
                || resolved.written_argument(0) != Some(0)
            {
                return None;
            }
            let selector_value =
                tcl_syntax::word_rules::original_static_word_unicode_value(selector)?;
            for class in self.all_classes.values() {
                let Some(owner) = self.logical_source_class_declaration(class) else {
                    continue;
                };
                for (side, members) in [
                    (MemberSide::Instance, &class.methods),
                    (MemberSide::ClassObject, &class.class_methods),
                ] {
                    if !std::ptr::eq(class, caller_class) || side != caller.side() {
                        continue;
                    }
                    for method in members.values() {
                        let Some(name) = class
                            .original_members
                            .logical_name_operand(side, method, input, image)
                        else {
                            continue;
                        };
                        if tcl_syntax::word_rules::original_static_word_unicode_value(name).as_ref()
                            != Some(&selector_value)
                        {
                            continue;
                        }
                        let member = owner.member(side, method)?;
                        if selected.is_some() {
                            return None;
                        }
                        selected = Some(member);
                    }
                }
            }
        }
        selected
    }

    fn logical_relation_providers(
        &self,
        class: &ClassDef,
        factory: &crate::command_binding::OriginalSourceClassDeclaration,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<crate::command_binding::LogicalSourceClassRelationProvider>> {
        let input = self.resolved_input.as_ref()?;
        let bindings = self.retained_command_realm()?.source_bindings_ref();
        let statements =
            self.logical_relation_statements(class, factory, |body, words, operand| {
                let provider = bindings
                    .logical_class_relation_provider(body, words, operand, input, registry)?;
                provider.matches_receiver(self, class).then_some(provider)
            })?;
        let mut supers = Vec::<tcl_core_types::ByteCommandSlot>::new();
        let mut mixins = Vec::<tcl_core_types::ByteCommandSlot>::new();
        let mut providers = Vec::new();
        for statement in statements {
            let current = match statement.relation {
                tcl_registry::definer::RelationSlot::Superclass => &mut supers,
                tcl_registry::definer::RelationSlot::Mixin => &mut mixins,
                tcl_registry::definer::RelationSlot::Filter => continue,
            };
            let slots = statement
                .operands
                .iter()
                .map(|provider| provider.source_slot().clone())
                .collect::<Vec<_>>();
            statement.apply_values(current, &slots);
            providers.extend(statement.operands);
        }
        supers.extend(mixins);
        let mut selected = Vec::new();
        for slot in supers {
            if let Some(provider) = providers
                .iter()
                .find(|provider| provider.source_slot() == &slot)
                && !selected.iter().any(
                    |selected: &&crate::command_binding::LogicalSourceClassRelationProvider| {
                        selected.source_slot() == &slot
                    },
                )
            {
                selected.push(provider);
            }
        }
        Some(selected.into_iter().cloned().collect())
    }

    fn logical_class_factory(
        &self,
        class: &ClassDef,
        image: &SourceImage,
    ) -> Option<crate::command_binding::OriginalSourceClassDeclaration> {
        let source = image.try_text().ok()?;
        self.command_invocations.iter().find_map(|invocation| {
            if invocation.range.start() >= class.body_span.start() {
                return None;
            }
            let factory =
                crate::registry_invocation::source_structure::source_class_declaration_at(
                    source,
                    self,
                    invocation.range.start(),
                )?;
            std::ptr::eq(factory.logical_source_class(self)?, class).then_some(())?;
            let original = factory.name_input().logical_input()?;
            (tcl_syntax::word_rules::original_static_word_unicode_value(original.original_word())
                .as_deref()
                == Some(original.bytes()))
            .then_some(())?;
            Some(factory)
        })
    }

    fn logical_relation_statements<T>(
        &self,
        class: &ClassDef,
        factory: &crate::command_binding::OriginalSourceClassDeclaration,
        resolve: impl Fn(
            &crate::registry_invocation::OriginalSourceScriptBody,
            &[tcl_lexer::NativeWord],
            &tcl_lexer::NativeWord,
        ) -> Option<T>,
    ) -> Option<Vec<LogicalSourceRelationStatement<T>>> {
        if class.member_set_incomplete
            || !class.original_members.logical_relation_coverage_available()
        {
            return None;
        }
        let input = self.resolved_input.as_ref()?;
        let context = input.context_registry();
        let source = self
            .retained_command_realm()?
            .original_source_image()?
            .try_text()
            .ok()?;
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            source,
            self,
            factory.factory().site().offset,
        )?;
        let body = words
            .source_script_bodies(&context)
            .into_iter()
            .find(|body| {
                body.original_container()
                    .tokens()
                    .first()
                    .is_some_and(|token| token.span == class.body_span)
            })?;
        let region = body.definition_member_region(&context, body.content_span())?;
        region.complete_original_commands()?;
        for words in region.original_commands() {
            region.checked_references(words)?;
        }
        let mut commands = class
            .original_members
            .logical_relation_commands()
            .iter()
            .collect::<Vec<_>>();
        commands.sort_by_key(|command| command.words.first().map(|word| word.span().start()));
        let mut statements = Vec::new();
        for command in commands {
            if command.input != *input {
                return None;
            }
            if command.relation == tcl_registry::definer::RelationSlot::Filter
                || command.side? != MemberSide::Instance
            {
                continue;
            }
            let mut operands = Vec::new();
            for reference in region.checked_references(&command.words)? {
                if reference.kind() != tcl_registry::definer::MemberRefKind::Class {
                    return None;
                }
                operands.push(resolve(&body, &command.words, reference.original_word())?);
            }
            statements.push(LogicalSourceRelationStatement {
                relation: command.relation,
                slot: command.slot?,
                operation: command.operation?,
                operands,
            });
        }
        Some(statements)
    }

    /// Complete original relation statements retain conditional external lookup
    /// candidates without supplying loaded classes or a native relation table.
    #[must_use]
    pub fn logical_source_class_relation_candidates(
        &self,
        class: &ClassDef,
    ) -> Option<
        Vec<
            LogicalSourceRelationStatement<
                crate::command_binding::LogicalSourceClassRelationCandidates,
            >,
        >,
    > {
        let owner = self.logical_source_class_declaration(class)?;
        let factory = self.logical_class_factory(class, &owner.image)?;
        let bindings = self.retained_command_realm()?.source_bindings_ref();
        let input = self.resolved_input.as_ref()?;
        let registry = self.resolved_registry()?;
        self.logical_relation_statements(class, &factory, |body, words, operand| {
            crate::command_binding::SourceCommandBindings::logical_class_relation_candidates(
                bindings, body, words, operand, input, registry,
            )
        })
    }

    /// Reborrow the actual current class behind a selected readonly member.
    #[must_use]
    pub fn logical_source_member_class(
        &self,
        member: &LogicalSourceMemberDeclaration,
    ) -> Option<&ClassDef> {
        let mut classes = self
            .all_classes
            .values()
            .filter(|class| member.class.matches(self, class));
        let class = classes.next()?;
        classes.next().is_none().then_some(class)
    }

    /// Current typed Logical declaration selected by an actual naming or
    /// class-reference operand. Display labels supply no selection authority.
    #[must_use]
    pub fn logical_source_class_at(
        &self,
        image: &SourceImage,
        config: tcl_lexer::LexerConfig,
        cursor: u32,
    ) -> Option<&ClassDef> {
        let input = self.resolved_input.as_ref()?;
        if input.lexer_config() != config || !self.matches_original_source_image(image, config) {
            return None;
        }
        let contains =
            |word: &tcl_lexer::NativeWord| word.span().as_range().contains(&(cursor as usize));
        let mut selected = Vec::new();
        for class in self.all_classes.values() {
            let Some(owner) = self.logical_source_class_declaration(class) else {
                continue;
            };
            if owner.name_word.as_ref().is_some_and(contains) {
                selected.push(class);
            }
            if let Some(relations) = &owner.relations {
                for relation in relations {
                    if contains(relation.operand())
                        && relation.matches_receiver(self, class)
                        && let Some(Some(target)) = relation.source_class(self)
                    {
                        selected.push(target);
                    }
                }
            }
        }
        if selected.is_empty() {
            let source = image.try_text().ok()?;
            for invocation in &self.command_invocations {
                let Some(call) =
                    crate::registry_invocation::source_structure::source_constructor_call_at(
                        source,
                        self,
                        invocation.range.start(),
                    )
                else {
                    continue;
                };
                if !call.original_words().first().is_some_and(contains) {
                    continue;
                }
                let class = call.class_declaration().logical_source_class(self)?;
                self.logical_source_class_declaration(class)?;
                selected.push(class);
            }
        }
        match selected.as_slice() {
            [class] => Some(*class),
            [] => None,
            _ => None,
        }
    }

    /// Source-only direct edges selected by the actual parent-horizon graph.
    /// Unknown member/provider coverage is unavailable, never a known empty list.
    #[must_use]
    pub fn logical_source_direct_classes<'a>(
        &'a self,
        class: &ClassDef,
    ) -> Option<Vec<&'a ClassDef>> {
        let owner = self.logical_source_class_declaration(class)?;
        let mut selected: Vec<&'a ClassDef> = Vec::new();
        for provider in owner.relations.as_ref()? {
            if !provider.matches_receiver(self, class) {
                return None;
            }
            if let Some(target) = provider.source_class(self)? {
                self.logical_source_class_declaration(target)?;
                if !selected
                    .iter()
                    .any(|selected| std::ptr::eq(*selected, target))
                {
                    selected.push(target);
                }
            }
        }
        Some(selected)
    }

    /// Every retained completed Logical class snapshot must reborrow uniquely.
    /// This checks that transport only; dynamic/unrepresented classes remain
    /// outside its positive source catalogue, never a class-absence census.
    #[must_use]
    pub fn checked_logical_source_class_declarations(&self) -> Option<Vec<&ClassDef>> {
        self.allows_retained_logical_declaration_advice()
            .then_some(())?;
        let mut classes = Vec::new();
        for declaration in &self.logical_class_declarations {
            let mut current = self
                .all_classes
                .values()
                .filter(|class| declaration.matches(self, class));
            let class = current.next()?;
            current.next().is_none().then_some(())?;
            self.logical_source_class_declaration(class)?;
            classes.push(class);
        }
        (classes.len() == self.all_classes.len()).then_some(classes)
    }

    /// A current actual class must still agree with the completed source walk.
    /// Reporting qualified labels are excluded; altered member metadata declines.
    #[must_use]
    pub fn logical_source_class_declaration(
        &self,
        class: &ClassDef,
    ) -> Option<&LogicalSourceClassDeclaration> {
        self.all_classes
            .values()
            .any(|current| std::ptr::eq(current, class))
            .then_some(())?;
        let mut declarations = self
            .logical_class_declarations
            .iter()
            .filter(|declaration| declaration.matches(self, class));
        let selected = declarations.next()?;
        declarations.next().is_none().then_some(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn logical_class_body_word_comes_from_the_actual_definition_script_factory() {
        let source = "oo::class create C {method m {arg} {return $arg}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let class = &analysis.all_classes["::C"];
        let declaration = analysis.logical_source_class_declaration(class).unwrap();
        let input = analysis.resolved_input.as_ref().unwrap();
        let image = SourceImage::document(source);
        let word = declaration
            .original_body_word(&image, input)
            .expect("actual issued definition body source");
        assert_eq!(
            source.get(word.content_span().unwrap().as_range()),
            Some("method m {arg} {return $arg}")
        );
        assert!(
            declaration
                .original_body_word(&SourceImage::document("changed"), input)
                .is_none()
        );
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::C")
            .unwrap()
            .methods
            .get_mut("m")
            .unwrap()
            .params
            .clear();
        assert!(
            changed
                .logical_source_class_declaration(&changed.all_classes["::C"])
                .is_none()
        );
        let empty_source = "oo::class create Empty {}";
        let empty = Analyser::new().analyse(empty_source, "tcl");
        let declaration = empty
            .logical_source_class_declaration(&empty.all_classes["::Empty"])
            .unwrap();
        let word = declaration
            .original_body_word(
                &SourceImage::document(empty_source),
                empty.resolved_input.as_ref().unwrap(),
            )
            .expect("empty original class body");
        assert!(word.content_span().unwrap().is_empty());
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            native
                .logical_source_class_declaration(&native.all_classes["::C"])
                .is_none()
        );
    }

    #[test]
    fn logical_member_insertions_keep_literal_name_data_and_current_side_owner() {
        let source = "oo::class create C {method {$literal} {} {}; method {two words} {} {}; method dynamic$name {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let class = &analysis.all_classes["::C"];
        for (name, expected) in [("$literal", "{$literal}"), ("two words", "{two words}")] {
            let method = &class.methods[name];
            let insertion = analysis
                .logical_source_member_insertion(class, MemberSide::Instance, method)
                .expect("actual static Logical method-name operand");
            assert_eq!(insertion.name(), name);
            assert_eq!(insertion.source_word(), expected);
            assert!(
                analysis
                    .logical_source_member_insertion(class, MemberSide::ClassObject, method)
                    .is_none()
            );
            assert!(
                analysis
                    .logical_source_member_insertion(class, MemberSide::Instance, &method.clone())
                    .is_none()
            );
        }
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::C")
            .unwrap()
            .methods
            .get_mut("$literal")
            .unwrap()
            .body_span = tcl_lexer::Span::new(0, 1);
        let class = &changed.all_classes["::C"];
        assert!(
            changed
                .logical_source_member_insertion(
                    class,
                    MemberSide::Instance,
                    &class.methods["$literal"]
                )
                .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        let class = &missing.all_classes["::C"];
        assert!(
            missing
                .logical_source_member_insertion(
                    class,
                    MemberSide::Instance,
                    &class.methods["$literal"]
                )
                .is_none()
        );
    }

    #[test]
    fn logical_family_insertions_require_selected_original_names() {
        for source in [
            "snit::type C {method {two words} {} {}}",
            "itcl::class C {method {two words} {} {}}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let class = &analysis.all_classes["::C"];
            let method = &class.methods["two words"];
            let insertion = analysis
                .logical_source_member_insertion(class, MemberSide::Instance, method)
                .expect("selected original family method-name operand");
            assert_eq!(insertion.name(), "two words");
            assert_eq!(insertion.source_word(), "{two words}");
            assert!(
                analysis
                    .logical_source_member_insertion(class, MemberSide::Instance, &method.clone())
                    .is_none()
            );
        }
    }

    #[test]
    fn logical_member_declarations_keep_actual_side_source_and_current_metadata() {
        let source = "oo::class create Base {method m {arg} {}; self method m {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let class = &analysis.all_classes["::Base"];
        let declaration = analysis
            .logical_source_class_declaration(class)
            .expect("actual Logical class walk");
        let instance = declaration
            .member(MemberSide::Instance, &class.methods["m"])
            .unwrap();
        let class_side = declaration
            .member(MemberSide::ClassObject, &class.class_methods["m"])
            .unwrap();
        assert_ne!(instance, class_side);
        assert!(
            declaration
                .member(MemberSide::ClassObject, &class.methods["m"])
                .is_none()
        );
        assert!(
            analysis
                .logical_source_class_declaration(&class.clone())
                .is_none()
        );
        let mut labels = analysis.clone();
        labels.all_classes.get_mut("::Base").unwrap().qualified_name = "display only".into();
        assert_eq!(
            labels.logical_source_class_declaration(&labels.all_classes["::Base"]),
            Some(declaration)
        );
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::Base")
            .unwrap()
            .methods
            .get_mut("m")
            .unwrap()
            .params
            .clear();
        assert!(
            changed
                .logical_source_class_declaration(&changed.all_classes["::Base"])
                .is_none()
        );
        let mut changed = analysis.clone();
        changed.all_classes.get_mut("::Base").unwrap().body_span = tcl_lexer::Span::new(0, 1);
        assert!(
            changed
                .logical_source_class_declaration(&changed.all_classes["::Base"])
                .is_none()
        );
        let mut changed = analysis.clone();
        changed.resolved_input = None;
        assert!(
            changed
                .logical_source_class_declaration(&changed.all_classes["::Base"])
                .is_none()
        );
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            native
                .logical_source_class_declaration(&native.all_classes["::Base"])
                .is_none()
        );
    }
}

#[cfg(test)]
mod source_navigation_facet_tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn logical_class_navigation_keeps_the_selected_factory_independent_of_module_body_syntax() {
        let source = "oo::class create helper {method run {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let class = &analysis.all_classes["::helper"];
        let declaration = analysis.logical_source_class_declaration(class).unwrap();
        let coordinate = declaration
            .source_candidate_coordinate()
            .expect("actual selected factory Name/body and current class correspondence");
        assert_eq!(
            coordinate.namespace,
            tcl_core_types::ByteNamespacePath::root()
        );
        assert_eq!(coordinate.simple.as_bytes(), b"helper");
        let mut without_module_body = declaration.clone();
        without_module_body.body_word = None;
        assert_eq!(
            without_module_body.source_candidate_coordinate(),
            Some(coordinate)
        );
        assert!(
            without_module_body
                .original_body_word(
                    &SourceImage::document(source),
                    analysis.resolved_input.as_ref().unwrap()
                )
                .is_none()
        );
        let detached = LogicalSourceClassDeclaration::capture(
            SourceImage::document(source),
            analysis.resolved_input.as_ref().unwrap().clone(),
            class,
        )
        .unwrap();
        assert!(detached.source_candidate_coordinate().is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            missing
                .logical_source_class_declaration(&missing.all_classes["::helper"])
                .is_none()
        );
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            native
                .logical_source_class_declaration(&native.all_classes["::helper"])
                .is_none()
        );
    }
}

#[cfg(test)]
mod logical_navigation_receipt_tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn logical_member_and_relation_views_keep_the_actual_source_owners() {
        let source = "oo::class create Base {method run {} {}}\noo::class create Child {superclass Base; method run {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let image = SourceImage::document(source);
        let base = &analysis.all_classes["::Base"];
        let child = &analysis.all_classes["::Child"];
        let edges = analysis.logical_source_direct_classes(child).unwrap();
        assert_eq!(edges.len(), 1);
        assert!(std::ptr::eq(edges[0], base));
        assert!(
            analysis
                .logical_source_direct_classes(base)
                .unwrap()
                .is_empty()
        );
        let at = u32::try_from(source.find("run").unwrap()).unwrap();
        let member = analysis
            .logical_source_member_at(&image, input.lexer_config(), at)
            .unwrap();
        assert_eq!(member.side(), MemberSide::Instance);
        assert!(std::ptr::eq(
            analysis.logical_source_member_class(&member).unwrap(),
            base
        ));
        assert_eq!(member.original_name_word().unwrap().image(), &image);
        assert!(
            analysis
                .logical_source_member_at(
                    &SourceImage::document(&format!("{source}\n# changed")),
                    input.lexer_config(),
                    at
                )
                .is_none()
        );
        assert!(
            analysis
                .logical_source_direct_classes(&child.clone())
                .is_none()
        );
    }

    #[test]
    fn logical_relations_do_not_recover_unknown_parent_state_or_changed_metadata() {
        let source = "oo::class create Base {}\nmystery\noo::class create Child {superclass Base}";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(
            analysis
                .logical_source_direct_classes(&analysis.all_classes["::Child"])
                .is_none()
        );
        let source = "oo::class create Base {}\noo::class create Child {superclass Base}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        assert_eq!(
            analysis
                .logical_source_direct_classes(&analysis.all_classes["::Child"])
                .unwrap()
                .len(),
            1
        );
        analysis
            .all_classes
            .get_mut("::Child")
            .unwrap()
            .member_set_incomplete = true;
        assert!(
            analysis
                .logical_source_direct_classes(&analysis.all_classes["::Child"])
                .is_none()
        );
    }
}

#[cfg(test)]
mod logical_relation_navigation_tests {
    use crate::analyser::Analyser;
    use crate::command_binding::LogicalSourceClassRelationTarget;

    #[test]
    fn relation_navigation_retains_original_external_namespace_coordinates() {
        let source = "namespace eval N {oo::class create Child {superclass Base}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let child = &analysis.all_classes["::N::Child"];
        assert!(analysis.logical_source_direct_classes(child).is_none());
        let statements = analysis
            .logical_source_class_relation_candidates(child)
            .unwrap();
        let operand = &statements[0].operands()[0];
        assert_eq!(
            operand.operand().span().start() as usize,
            source.rfind("Base").unwrap()
        );
        let LogicalSourceClassRelationTarget::External(coordinates) =
            operand.target(&analysis, child).unwrap()
        else {
            panic!("missing local class must retain conditional external candidates");
        };
        assert_eq!(
            coordinates
                .iter()
                .map(tcl_syntax::naming::authored_source_command_spelling)
                .collect::<Option<Vec<_>>>()
                .unwrap(),
            vec!["::N::Base", "::Base"]
        );
        assert!(!operand.has_earlier_unknown_mutations());
    }

    #[test]
    fn relation_navigation_preserves_known_occupants_and_current_input_refusal() {
        let source = "namespace eval N {proc Base {} {}; oo::class create Child {superclass Base}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(
            analysis
                .logical_source_class_relation_candidates(&analysis.all_classes["::N::Child"])
                .is_none()
        );
        let source = "oo::class create Child {superclass Base}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let statements = analysis
            .logical_source_class_relation_candidates(&analysis.all_classes["::Child"])
            .unwrap();
        let config = analysis.body_lexer_config.unwrap();
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(
            statements[0].operands()[0]
                .target(&analysis, &analysis.all_classes["::Child"])
                .is_none()
        );
        assert!(
            analysis
                .logical_source_class_relation_candidates(&analysis.all_classes["::Child"])
                .is_none()
        );
    }
}

#[cfg(test)]
mod logical_relation_navigation_debt_tests {
    use crate::analyser::Analyser;
    use crate::command_binding::LogicalSourceClassRelationTarget;

    #[test]
    fn relation_navigation_retains_unknown_prefix_as_conditional_applicability() {
        let source = "mystery; oo::class create Child {superclass Base}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let child = &analysis.all_classes["::Child"];
        let statements = analysis
            .logical_source_class_relation_candidates(child)
            .unwrap();
        let operand = &statements[0].operands()[0];
        assert!(operand.has_earlier_unknown_mutations());
        assert!(matches!(
            operand.target(&analysis, child),
            Some(LogicalSourceClassRelationTarget::External(_))
        ));
        assert!(analysis.logical_source_direct_classes(child).is_none());
    }
}
