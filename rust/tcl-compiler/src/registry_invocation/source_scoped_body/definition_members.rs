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

//! Definition members of a script body: regions, script bodies, references and opaque regions.

use super::OriginalSourceScriptBody;
use super::operand_script_bodies::original_word_script_body_geometry;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};
use tcl_registry::ArgRole;
use tcl_registry::model::{ContextRegistry, ResolvedContext};

/// Exact opaque-script operand selected for lexical command authoring only.
/// This is neither a source script body nor a scope, frame or evaluated region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpaqueScriptAuthoringRegion {
    pub(super) parent: OriginalSourceScriptBody,
    pub(super) original: Vec<NativeWord>,
    pub(super) container: NativeWord,
    pub(super) content: Span,
}

impl OpaqueScriptAuthoringRegion {
    /// Reborrow the actual selected parent and operand under unchanged source
    /// and Registry custody. The resulting geometry grants authoring only.
    #[must_use]
    pub fn content_for_authoring(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> Option<Span> {
        if !self.parent.matches_source(image, config)
            || !self.parent.matches_context(context)
            || self.container.image() != image
            || self.container.config() != config
        {
            return None;
        }
        let members = self
            .parent
            .definition_member_region(context, self.parent.content_span())?;
        members
            .opaque_script_authoring_regions(context, &self.original)?
            .iter()
            .any(|region| region == self)
            .then_some(self.content)
    }
}

/// Original member script selected within an authentic definition body.
/// The genuine parent source schema and complete original member argv stay
/// retained. Member vocabulary is source syntax, never global command lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceDefinitionMemberScriptBody {
    pub(super) parent: OriginalSourceScriptBody,
    pub(super) original: Vec<NativeWord>,
    pub(super) argument: usize,
    pub(super) container: NativeWord,
    pub(super) content: Span,
    retains_definition_grammar: bool,
    presentation: tcl_registry::definer::SourceDefinitionMemberBodyPresentation,
    member_row: Option<tcl_registry::definer::MemberRow>,
}

/// Two genuine source-body issuers joined for readonly member correspondence.
/// Both applicability receipts remain intact; this is no body/frame entry or
/// conversion from conditional source data to a selected Native invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalSourceDefinitionMemberBodyCorrespondence {
    catalogue: OriginalSourceDefinitionMemberScriptBody,
    declaration: OriginalSourceDefinitionMemberScriptBody,
    heads: super::super::source_structure::OriginalSourceHeadCorrespondence,
}

impl OriginalSourceDefinitionMemberBodyCorrespondence {
    pub(crate) const fn catalogue_body(&self) -> &OriginalSourceDefinitionMemberScriptBody {
        &self.catalogue
    }
    pub(crate) const fn declaration_body(&self) -> &OriginalSourceDefinitionMemberScriptBody {
        &self.declaration
    }
    #[cfg(test)]
    pub(crate) const fn original_heads(
        &self,
    ) -> &super::super::source_structure::OriginalSourceHeadCorrespondence {
        &self.heads
    }
}

impl OriginalSourceDefinitionMemberScriptBody {
    /// Actual enclosing declaration body, retained solely as source lineage.
    /// Callable bodies do not acquire the parent's definition vocabulary,
    /// receiver namespace, command lookup or entered context from this borrow.
    pub(crate) const fn original_parent_body(&self) -> &OriginalSourceScriptBody {
        &self.parent
    }

    /// Complete original member argv, including its genuine source keyword.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Exact original whole word owning this script content.
    #[must_use]
    pub const fn original_container(&self) -> &NativeWord {
        &self.container
    }
    /// Actual member effect/receiver and body operand under the retained grammar.
    /// This is source applicability, not member installation or frame entry.
    #[must_use]
    pub fn member_row(&self) -> Option<&tcl_registry::definer::MemberRow> {
        self.member_row.as_ref()
    }
    /// Receiver vocabulary for this actual selected callable body. The
    /// parent source schema owns the grammar; this supplies no entered receiver.
    #[must_use]
    pub fn receiver_grammar(
        &self,
        context: &ContextRegistry,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        use tcl_registry::definer::{CallableRole, MemberEffect, MemberReceiver};
        self.matches_context(context).then_some(())?;
        let row = self.member_row()?;
        let MemberEffect::Callable { role, .. } = row.effect else {
            return None;
        };
        if row.receiver != MemberReceiver::Instance || role == CallableRole::Procedure {
            return None;
        }
        self.parent
            .source
            .with_source_schema(context, |schema| {
                schema.authored_source_definition_body_grammar()
            })
            .flatten()
    }
    /// Whole original member/body source and parser configuration agree.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.parent.matches_source(image, config)
            && self.container.image() == image
            && self.container.config() == config
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == config)
    }
    /// The member issuer retains its actual full context and Registry generation.
    #[must_use]
    pub fn matches_context(&self, context: &ContextRegistry) -> bool {
        self.parent.matches_context(context)
    }
    /// Join the actual original declaration's parent without equating its
    /// applicability receipt with a separate conditional catalogue issuer.
    /// Complete source/configuration, parent argv/origins/values and selected
    /// descriptor/definition grammar must agree; both sealed bodies survive.
    pub(crate) fn original_source_correspondence_in_parent(
        &self,
        context: &ContextRegistry,
        parent: &OriginalSourceScriptBody,
    ) -> Option<OriginalSourceDefinitionMemberBodyCorrespondence> {
        // naming.editor.original-method-candidates
        // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
        let original_parent = self.original_parent_body();
        let image = self.original_container().image();
        let config = self.original_container().config();
        if !self.matches_source(image, config)
            || !self.matches_context(context)
            || !parent.matches_source(image, config)
            || !parent.matches_context(context)
            || original_parent.original_container() != parent.original_container()
            || original_parent.extent_span() != parent.extent_span()
            || original_parent.container_extent_span() != parent.container_extent_span()
            || original_parent.content_span() != parent.content_span()
        {
            return None;
        }
        let left = original_parent.source_words();
        let right = parent.source_words();
        let heads = crate::registry_invocation::source_structure::OriginalRegistryWords::original_source_head_correspondence(left, right, context)?;
        if left.origins().get(1..) != right.origins().get(1..)
            || left.arguments().len() != right.arguments().len()
            || !left
                .arguments()
                .iter()
                .zip(right.arguments())
                .all(
                    |(left, right)| match (left.literal_bytes(), right.literal_bytes()) {
                        (Some(left), Some(right)) => left == right,
                        (None, None) => left == right,
                        _ => false,
                    },
                )
            || !(0..left.arguments().len()).all(|index| {
                left.original_argument_word(index).is_some()
                    && left.original_argument_word(index) == right.original_argument_word(index)
            })
        {
            return None;
        }
        let same_schema = left.with_source_schema(context, |left| {
            right.with_source_schema(context, |right| {
                std::ptr::eq(
                    left.authored_source_descriptors().command,
                    right.authored_source_descriptors().command,
                ) && left.semantics.argument_offset == right.semantics.argument_offset
                    && left
                        .authored_source_definition_body_grammar()
                        .zip(right.authored_source_definition_body_grammar())
                        .is_some_and(|(left, right)| std::ptr::eq(left, right))
            })
        })??;
        same_schema.then_some(())?;
        let region = parent.definition_member_region(context, parent.content_span())?;
        let original =
            region.original_command_words_at(self.original_words().first()?.span().start())?;
        (original == self.original_words()).then_some(())?;
        let mut bodies = region.script_bodies(original)?.into_iter().filter(|body| {
            body.original == self.original
                && body.argument == self.argument
                && body.container == self.container
                && body.content == self.content
                && body.retains_definition_grammar == self.retains_definition_grammar
                && body.presentation == self.presentation
                && body.member_row == self.member_row
        });
        let declaration = bodies.next()?;
        bodies
            .next()
            .is_none()
            .then_some(OriginalSourceDefinitionMemberBodyCorrespondence {
                catalogue: self.clone(),
                declaration,
                heads,
            })
    }

    /// Complete original member-body command geometry. Counted source must
    /// exactly retain the authored content; a partial or cooked body refuses.
    /// This supplies no entered member, effect closure or normal completion.
    pub(crate) fn complete_original_commands(&self) -> Option<Vec<Vec<NativeWord>>> {
        let value = tcl_syntax::word_rules::original_static_word_unicode_value(&self.container)?;
        (self
            .container
            .image()
            .bytes()
            .get(self.content.as_range())?
            == value.as_slice())
        .then_some(())?;
        let plan = tcl_lexer::native_script_words_in(
            self.container.image().clone(),
            self.content,
            self.container.config(),
        )
        .ok()?;
        plan.fatal_tail.is_none().then_some(())?;
        Some(
            plan.commands
                .into_iter()
                .map(|command| command.words)
                .collect(),
        )
    }

    /// Original post-keyword body ordinal, preserving wrapper/option offsets.
    #[must_use]
    pub const fn argument(&self) -> usize {
        self.argument
    }
    /// Presentation selected by the genuine member effect and body slot.
    /// This describes source layout, without entered member or runtime authority.
    #[must_use]
    pub const fn presentation(
        &self,
    ) -> tcl_registry::definer::SourceDefinitionMemberBodyPresentation {
        self.presentation
    }
    /// Exact original script bytes, excluding word delimiters.
    #[must_use]
    pub const fn content_span(&self) -> Span {
        self.content
    }
    /// Original source fold extent, excluding the authentic closing delimiter.
    #[must_use]
    pub fn extent_span(&self) -> Span {
        Span::new(self.container.group().span.start(), self.content.end())
    }
    /// Independently retained source vocabulary applies only to wrapper blocks.
    #[must_use]
    pub const fn definition_parent(&self) -> Option<&OriginalSourceScriptBody> {
        if self.retains_definition_grammar {
            Some(&self.parent)
        } else {
            None
        }
    }
}

/// One readonly member-reference operand from a genuine definition region.
/// Original value production, parent vocabulary and complete source membership
/// remain retained; this grants no member table, entered worker or edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceDefinitionMemberReference {
    pub(super) original: NativeWord,
    pub(super) kind: tcl_registry::definer::MemberRefKind,
    pub(super) value: Vec<u8>,
    protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
}

impl OriginalSourceDefinitionMemberReference {
    /// Whole original operand, independently of its displayed name.
    #[must_use]
    pub const fn original_word(&self) -> &NativeWord {
        &self.original
    }
    /// Registry-selected referenced entity kind.
    #[must_use]
    pub const fn kind(&self) -> tcl_registry::definer::MemberRefKind {
        self.kind
    }
    /// Source-produced counted units, never a runtime member or allocation.
    #[must_use]
    pub fn value(&self) -> &[u8] {
        &self.value
    }
    /// Readonly source-name comparison under the independently selected recipe.
    /// A reporting label cannot supply native table or editable-name evidence.
    #[must_use]
    pub fn matches_reported_name(&self, name: &str) -> bool {
        self.protocol.map_or_else(
            || name.is_ascii() && self.value.as_slice() == name.as_bytes(),
            |protocol| {
                tcl_syntax::backslash::native_source_literal_bytes(
                    name.as_bytes(),
                    self.original.image().channel(),
                    protocol,
                )
                .is_ok_and(|value| value.as_ref() == self.value.as_slice())
            },
        )
    }
}

/// Sealed original member-command inventory under a genuine definition body.
/// One complete lexical plan owns argv membership for the current source region.
/// Selected vocabulary remains conditional source syntax, never native dispatch.
#[derive(Debug, Clone)]
pub struct OriginalSourceDefinitionMemberRegion {
    pub(super) parent: OriginalSourceScriptBody,
    pub(super) context: ResolvedContext,
    pub(super) grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    plan: tcl_lexer::NativeScriptWordsPlan,
    region: Span,
}

impl OriginalSourceScriptBody {
    /// Capture a complete current script region inside this genuine body.
    /// The same original image/configuration and actual context are mandatory.
    /// Callers retain this issuer for every member in the region instead of
    /// re-parsing a class body or manufacturing a nominal grammar owner.
    #[must_use]
    pub fn definition_member_region(
        &self,
        context: &ContextRegistry,
        script_region: Span,
    ) -> Option<OriginalSourceDefinitionMemberRegion> {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        if !self.matches_context(context)
            || script_region.start() < self.content.start()
            || script_region.end() > self.content.end()
            || script_region.start() > script_region.end()
        {
            return None;
        }
        let grammar = self
            .source
            .with_source_schema(context, |schema| {
                schema.authored_source_definition_body_grammar()
            })
            .flatten()?;
        let plan = tcl_lexer::native_script_words_in(
            self.container.image().clone(),
            script_region,
            self.container.config(),
        )
        .ok()?;
        Some(OriginalSourceDefinitionMemberRegion {
            parent: self.clone(),
            context: context.context().clone(),
            grammar,
            plan,
            region: script_region,
        })
    }
}

fn source_member_grammar(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
    schema.authored_source_definition_body_grammar()
}

/// Disposition under one genuine current definition-member vocabulary.
/// Nonmembership is source syntax only, never runtime command absence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalSourceDefinitionMemberDisposition {
    /// The complete retained vocabulary classifies this source command as a nonmember.
    CompleteNonMember,
    /// The retained source or vocabulary cannot supply a complete classification.
    Unavailable,
    /// The source command is a member with the retained script bodies, possibly empty.
    CompleteMember(Vec<OriginalSourceDefinitionMemberScriptBody>),
}

impl OriginalSourceDefinitionMemberRegion {
    /// The definition grammar selected for the parent body.
    #[must_use]
    pub(crate) fn grammar(&self) -> &'static tcl_registry::definer::DefinitionBodyGrammar {
        self.grammar
    }

    /// Complete original vectors only when the retained lexical plan covers
    /// the entire genuine parent definition body without a fatal tail.
    /// An empty complete vector is source coverage, never lifecycle absence.
    #[must_use]
    pub fn complete_original_commands(&self) -> Option<Vec<Vec<NativeWord>>> {
        // The complete-plan caller must retain the whole parent extent; a
        // subregion cannot certify the rest of a class definition.
        let complete = self.region == self.parent.content_span();
        (complete && self.plan.fatal_tail.is_none()).then_some(())?;
        Some(
            self.plan
                .commands
                .iter()
                .map(|command| command.words.clone())
                .collect(),
        )
    }
    /// Complete original argv at its genuine command-head offset in this
    /// retained source region. No fresh segmentation or displayed head is used.
    #[must_use]
    pub fn original_command_words_at(&self, start: u32) -> Option<&[NativeWord]> {
        let index = self
            .plan
            .commands
            .binary_search_by_key(&start, |command| command.span.start())
            .ok()?;
        Some(&self.plan.commands.get(index)?.words)
    }

    /// Actual retained script region; nested ordinary scripts keep their own
    /// independent source region rather than inheriting an enclosing vocabulary.
    #[must_use]
    pub const fn source_region(&self) -> Span {
        self.region
    }

    /// Separate complete nonmember, unavailable owner/layout and complete
    /// selected bodies before a consumer considers global command source data.
    #[must_use]
    pub fn script_body_disposition(
        &self,
        context: &ContextRegistry,
        original: &[NativeWord],
    ) -> OriginalSourceDefinitionMemberDisposition {
        use OriginalSourceDefinitionMemberDisposition::{
            CompleteMember, CompleteNonMember, Unavailable,
        };
        let Some(first) = original.first() else {
            return Unavailable;
        };
        if self.plan.fatal_tail.is_some()
            || self.context != *context.context()
            || !self.parent.matches_context(context)
            || !self.parent.matches_source(first.image(), first.config())
            || self
                .parent
                .source_words()
                .with_source_schema(context, source_member_grammar)
                .flatten()
                .is_none_or(|grammar| !std::ptr::eq(grammar, self.grammar))
        {
            return Unavailable;
        }
        let Some(values) = self.member_values_with_dynamic(original, true) else {
            return Unavailable;
        };
        let Some(keyword) = values
            .first()
            .and_then(Option::as_deref)
            .and_then(|keyword| std::str::from_utf8(keyword).ok())
        else {
            return Unavailable;
        };
        if self.grammar.member(keyword).is_none() {
            return CompleteNonMember;
        }
        if self
            .grammar
            .source_member_in(
                keyword.as_bytes(),
                Some(context.context().authoring_query()),
            )
            .is_none()
        {
            return Unavailable;
        }
        self.checked_script_bodies(original)
            .map_or(Unavailable, CompleteMember)
    }

    /// Complete original command vectors from the retained lexical plan.
    /// Returning a source command grants no reached execution or worker entry.
    pub fn original_commands(&self) -> impl Iterator<Item = &[NativeWord]> {
        self.plan
            .commands
            .iter()
            .map(|command| command.words.as_slice())
    }

    /// Whole original variable-name operands under the selected member role.
    /// These words supply source display geometry, not accepted member names,
    /// variable cells or an entered definition frame.
    #[must_use]
    pub fn variable_argument_words(&self, original: &[NativeWord]) -> Option<Vec<&NativeWord>> {
        self.argument_words_for_role(original, ArgRole::VarWrite)
    }

    /// Whole original operands carrying one selected source member role.
    /// Names and list containers retain their own word geometry; this does
    /// not manufacture fields, member identity or runtime invocation authority.
    #[must_use]
    pub fn argument_words_for_role(
        &self,
        original: &[NativeWord],
        role: ArgRole,
    ) -> Option<Vec<&NativeWord>> {
        let values = self.member_values(original)?;
        let keyword = values.first()?.as_deref()?;
        let arguments = original
            .iter()
            .zip(&values)
            .skip(1)
            .map(|(word, value)| {
                if word.group().expand {
                    tcl_registry::InvocationWord::Expanded
                } else {
                    value.as_deref().map_or(
                        tcl_registry::InvocationWord::Dynamic,
                        tcl_registry::InvocationWord::KnownBytes,
                    )
                }
            })
            .collect::<Vec<_>>();
        let indices = self.grammar.source_member_arg_indices_for_role_in(
            keyword,
            tcl_registry::InvocationArguments::structured(&arguments),
            Some(self.context.authoring_query()),
            role,
        )?;
        let retained = self.original_command_words_at(original.first()?.span().start())?;
        indices
            .into_iter()
            .map(|index| retained.get(index.checked_add(1)?))
            .collect()
    }

    /// Opaque source scripts may supply lexical command authoring without
    /// joining executable Body inventories. Complete argv and unchanged whole
    /// container bytes must retain this actual parent's selected role.
    #[must_use]
    pub fn opaque_script_authoring_regions(
        &self,
        context: &ContextRegistry,
        original: &[NativeWord],
    ) -> Option<Vec<OpaqueScriptAuthoringRegion>> {
        self.complete_original_commands()?;
        let first = original.first()?;
        if self.context != *context.context()
            || !self.parent.matches_source(first.image(), first.config())
            || !self.parent.matches_context(context)
            || self
                .parent
                .source_words()
                .with_source_schema(context, source_member_grammar)
                .flatten()
                .is_none_or(|grammar| !std::ptr::eq(grammar, self.grammar))
        {
            return None;
        }
        let values = self.member_values(original)?;
        let keyword = values.first()?.as_deref()?;
        let arguments = original
            .iter()
            .zip(&values)
            .skip(1)
            .map(|(word, value)| {
                if word.group().expand {
                    tcl_registry::InvocationWord::Expanded
                } else {
                    value.as_deref().map_or(
                        tcl_registry::InvocationWord::Dynamic,
                        tcl_registry::InvocationWord::KnownBytes,
                    )
                }
            })
            .collect::<Vec<_>>();
        let indices = self.grammar.checked_source_member_arg_indices_for_role_in(
            keyword,
            tcl_registry::InvocationArguments::structured(&arguments),
            Some(self.context.authoring_query()),
            ArgRole::OpaqueScript,
        )?;
        indices
            .into_iter()
            .map(|argument| {
                let container = original.get(argument.checked_add(1)?)?;
                let geometry = original_word_script_body_geometry(container)?;
                let value = tcl_syntax::word_rules::original_static_word_unicode_value(container)?;
                (container.image().bytes().get(geometry.content.as_range())? == value.as_slice())
                    .then_some(())?;
                Some(OpaqueScriptAuthoringRegion {
                    parent: self.parent.clone(),
                    original: original.to_vec(),
                    container: container.clone(),
                    content: geometry.content,
                })
            })
            .collect()
    }

    /// Readonly reference operands under this genuine parent vocabulary.
    /// Complete command membership and actual availability remain mandatory;
    /// dynamic operands retain no static reference-name or writable geometry.
    #[must_use]
    pub fn references(
        &self,
        original: &[NativeWord],
    ) -> Option<Vec<OriginalSourceDefinitionMemberReference>> {
        self.project_references(original, false)
    }

    /// Complete selected reference-role projection. Missing dynamic names
    /// remain unavailable; known members without reference roles yield empty.
    #[must_use]
    pub fn checked_references(
        &self,
        original: &[NativeWord],
    ) -> Option<Vec<OriginalSourceDefinitionMemberReference>> {
        self.project_references(original, true)
    }

    fn project_references(
        &self,
        original: &[NativeWord],
        complete: bool,
    ) -> Option<Vec<OriginalSourceDefinitionMemberReference>> {
        // naming.core.original-member-reference-hazard
        // docs/design/analysis/name-resolution-proofs/core-original-member-reference-hazard.md
        let values = self.member_values(original)?;
        let keyword = values.first()?.as_deref()?;
        let arguments = original
            .iter()
            .zip(&values)
            .skip(1)
            .map(|(word, value)| {
                if word.group().expand {
                    tcl_registry::InvocationWord::Expanded
                } else {
                    value.as_deref().map_or(
                        tcl_registry::InvocationWord::Dynamic,
                        tcl_registry::InvocationWord::KnownBytes,
                    )
                }
            })
            .collect::<Vec<_>>();
        let selected = self.grammar.checked_source_member_ref_indices_in(
            keyword,
            tcl_registry::InvocationArguments::structured(&arguments),
            Some(self.context.authoring_query()),
        )?;
        let Some((kind, indices)) = selected else {
            return complete.then(Vec::new);
        };
        let protocol = self
            .parent
            .source
            .dialect()
            .and_then(tcl_native_model::InvocationDialect::authored_name_policy)
            .map(tcl_syntax::naming::NamePolicyProtocol::string_protocol);
        let references = indices
            .into_iter()
            .map(|index| {
                let ordinal = index.checked_add(1)?;
                Some(OriginalSourceDefinitionMemberReference {
                    original: original.get(ordinal)?.clone(),
                    kind,
                    value: values.get(ordinal)?.as_ref()?.clone(),
                    protocol,
                })
            })
            .collect::<Vec<_>>();
        if complete {
            references.into_iter().collect()
        } else {
            Some(references.into_iter().flatten().collect())
        }
    }

    /// Source member scripts through genuine parent vocabulary and whole words.
    /// Every supplied word must match one complete command in the retained
    /// lexical plan. Dynamic selectors, expansions and cooked body geometry
    /// cannot be repaired through a nominal spelling or token prefix.
    #[must_use]
    pub fn script_bodies(
        &self,
        original: &[NativeWord],
    ) -> Option<Vec<OriginalSourceDefinitionMemberScriptBody>> {
        self.project_script_bodies(original, false)
    }

    /// Complete selected member-body geometry through the same source grammar.
    /// Missing or cooked bodies cannot certify full source coverage.
    #[must_use]
    pub fn checked_script_bodies(
        &self,
        original: &[NativeWord],
    ) -> Option<Vec<OriginalSourceDefinitionMemberScriptBody>> {
        self.project_script_bodies(original, true)
    }

    fn project_script_bodies(
        &self,
        original: &[NativeWord],
        complete: bool,
    ) -> Option<Vec<OriginalSourceDefinitionMemberScriptBody>> {
        let values = self.member_values(original)?;
        let keyword = values.first()?.as_deref()?;
        self.grammar.member(std::str::from_utf8(keyword).ok()?)?;
        let arguments: Vec<_> = original
            .iter()
            .zip(&values)
            .skip(1)
            .map(|(word, value)| {
                if word.group().expand {
                    tcl_registry::InvocationWord::Expanded
                } else {
                    value.as_deref().map_or(
                        tcl_registry::InvocationWord::Dynamic,
                        tcl_registry::InvocationWord::KnownBytes,
                    )
                }
            })
            .collect();
        let selected = if complete {
            self.grammar.checked_source_member_script_arguments_in(
                keyword,
                tcl_registry::InvocationArguments::structured(&arguments),
                Some(self.context.authoring_query()),
            )?
        } else {
            self.grammar.source_member_script_arguments_in(
                keyword,
                tcl_registry::InvocationArguments::structured(&arguments),
                Some(self.context.authoring_query()),
            )?
        };
        let member_row = self.source_member_row(original, &values);
        let bodies = selected
            .into_iter()
            .map(|selected| {
                let word = original.get(selected.argument().checked_add(1)?)?;
                let geometry = original_word_script_body_geometry(word)?;
                Some(OriginalSourceDefinitionMemberScriptBody {
                    parent: self.parent.clone(),
                    original: original.to_vec(),
                    argument: selected.argument(),
                    container: geometry.container,
                    content: geometry.content,
                    retains_definition_grammar: selected.retains_definition_grammar(),
                    presentation: selected.presentation(),
                    member_row: member_row.clone().filter(|row| {
                        selected
                            .argument()
                            .checked_add(1)
                            .is_some_and(|ordinal| row.body.is_some_and(|body| body.0 == ordinal))
                    }),
                })
            })
            .collect::<Vec<_>>();
        if complete {
            bodies.into_iter().collect()
        } else {
            Some(bodies.into_iter().flatten().collect())
        }
    }

    /// Member row (effect, receiver, body slot) selected by this genuine
    /// parent vocabulary for complete original argv. Consumers route a member
    /// by this effect, never by its keyword; it grants no installed member,
    /// dispatch or entered body.
    #[must_use]
    pub fn checked_member_row(
        &self,
        original: &[NativeWord],
    ) -> Option<tcl_registry::definer::MemberRow> {
        let values = self.member_values(original)?;
        self.source_member_row(original, &values)
    }

    fn source_member_row(
        &self,
        original: &[NativeWord],
        values: &[Option<Vec<u8>>],
    ) -> Option<tcl_registry::definer::MemberRow> {
        // Row selectors require an independent original Document literal,
        // not a text projection of the retained counted value alone.
        let source_literals = original
            .iter()
            .map(tcl_syntax::word_rules::original_static_word_ascii_presentation)
            .collect::<Vec<_>>();
        let member_words = original
            .iter()
            .zip(values)
            .zip(&source_literals)
            .map(|((word, value), literal)| {
                if word.group().expand {
                    return tcl_registry::InvocationWord::Expanded;
                }
                let counted = value.as_deref().map_or(
                    tcl_registry::InvocationWord::Dynamic,
                    tcl_registry::InvocationWord::KnownBytes,
                );
                match (value.as_deref(), literal.as_deref()) {
                    (Some(value), Some(literal)) if value == literal && !literal.contains(&0) => {
                        super::super::source_structure::source_schema_word(counted)
                    }
                    _ => counted,
                }
            })
            .collect::<Vec<_>>();
        self.grammar.member_row(
            0,
            tcl_registry::InvocationArguments::structured(&member_words),
            Some(self.context.authoring_query()),
        )
    }

    /// Source keyword selected from complete original argv in this genuine
    /// parent region. This supplies vocabulary only, no global command identity.
    #[must_use]
    pub fn member_keyword(
        &self,
        original: &[NativeWord],
    ) -> Option<&'static tcl_registry::definer::MemberSpec> {
        let values = self.member_values(original)?;
        self.grammar.source_member_in(
            values.first()?.as_deref()?,
            Some(self.context.authoring_query()),
        )
    }

    fn member_values(&self, original: &[NativeWord]) -> Option<Vec<Option<Vec<u8>>>> {
        self.member_values_with_dynamic(original, false)
    }

    fn member_values_with_dynamic(
        &self,
        original: &[NativeWord],
        keep_dynamic: bool,
    ) -> Option<Vec<Option<Vec<u8>>>> {
        let first = original.first()?;
        if first.group().expand {
            return None;
        }
        (self.original_command_words_at(first.span().start())? == original).then_some(())?;
        if let super::super::source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            self.parent.source.source()
            && let Some(input) = advice.logical_source_input()
        {
            let context = input.context_registry();
            (input.has_logical_source_name_context()
                && self.context == *context.context()
                && self.parent.matches_context(&context)
                && self
                    .parent
                    .matches_source(first.image(), input.lexer_config())
                && advice.matches_source(first.image(), input.lexer_config())
                && advice.matches_context(&context)
                && advice.site().source.source_image() == first.image()
                && original.iter().all(|word| {
                    word.image() == first.image() && word.config() == input.lexer_config()
                }))
            .then_some(())?;
            if keep_dynamic {
                return Some(
                    original
                        .iter()
                        .map(tcl_syntax::word_rules::original_static_word_unicode_value)
                        .collect(),
                );
            }
            return original
                .iter()
                .map(|word| {
                    tcl_syntax::word_rules::original_static_word_unicode_value(word).map(Some)
                })
                .collect();
        }
        let captured = self
            .parent
            .source
            .dialect()
            .and_then(tcl_native_model::InvocationDialect::authored_name_policy)
            .map(|policy| {
                tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                    original,
                    policy.string_protocol(),
                )
            })
            .transpose()
            .ok()?;
        let values: Vec<_> = original
            .iter()
            .enumerate()
            .map(|(index, word)| {
                captured.as_ref().map_or_else(
                    || tcl_syntax::word_rules::original_static_word_ascii_presentation(word),
                    |captured| captured.literal(index).map(Vec::from),
                )
            })
            .collect();
        Some(values)
    }
}
