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

//! Source operand layouts retained for an original declaration and their continuations.

use super::super::declaration_layout::{
    DeclarationLayoutObservation, original_declaration_layouts,
};
use super::super::{CommandAllocationSite, SourceCommandBindings, SourceLookupSnapshot};
use super::nested_layouts::retains_successor_layout;
use super::procedure_header::{
    declaration_tokens, original_declaration_operand_input, procedure_declaration_header,
};
use super::read_declaration::{
    OriginalProcedureReadDeclaration, ProcedureReadFormals, ProcedureReadSource,
};
use super::{
    DeclaredProcedureHeader, OriginalDeclarationSourceOperandAttachment,
    OriginalDeclarationSourceOperandCache, ProcedureOperands,
};
use crate::registry_invocation::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    effective_words_for_target,
};
use std::collections::BTreeMap;
use std::sync::Arc;

impl PartialEq for OriginalDeclarationSourceOperandAttachment {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset && std::sync::Weak::ptr_eq(&self.cache, &other.cache)
    }
}

impl Eq for OriginalDeclarationSourceOperandAttachment {}

impl OriginalDeclarationSourceOperandAttachment {
    pub(in super::super) fn owns(
        &self,
        cache: &Arc<OriginalDeclarationSourceOperandCache>,
        offset: u32,
    ) -> bool {
        self.offset == offset && std::sync::Weak::ptr_eq(&self.cache, &Arc::downgrade(cache))
    }
}

/// A conditional source body, its actual declaration lineage and one complete
/// original command vector. The Native-decoded header value is retained
/// separately from the selected source-word transform. No executed body,
/// ordinary occurrence across a fold, installed frame or Normal fact is issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct OriginalDeclarationSourceOperandLayout {
    pub(in super::super) site: CommandAllocationSite,
    pub(in super::super) config: tcl_lexer::LexerConfig,
    pub(in super::super) words: Arc<[crate::ir::WordExpr]>,
    pub(super) dialect: tcl_registry::InvocationDialect,
    pub(super) read: OriginalProcedureReadDeclaration,
    pub(super) header: DeclaredProcedureHeader,
    pub(super) snapshot: Arc<super::super::SourceLookupSnapshot>,
    native_header_value: Arc<[u8]>,
    pub(super) parents: Arc<[DeclarationLayoutObservation]>,
    pub(in super::super) source_operations: Arc<[OriginalDeclarationSourceOperation]>,
}

/// Earlier conditional source operations retain their exact source vectors and
/// every residual effect/applicability premise. This is not an updated world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct OriginalDeclarationSourceOperation {
    pub(super) site: CommandAllocationSite,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) words: Arc<[crate::ir::WordExpr]>,
    pub(in super::super) operation:
        tcl_registry::source_operation::AuthoredSourceOperationContinuation,
}

impl OriginalDeclarationSourceOperandLayout {
    #[cfg(test)]
    pub(in super::super) const fn dialect(&self) -> tcl_registry::InvocationDialect {
        self.dialect
    }

    pub(in super::super) fn metadata_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<super::super::OriginalCompilationLookupAdvice> {
        if !self.source_operations.is_empty() {
            return None;
        }
        self.conditional_source_advice(tokens)
    }

    pub(in super::super) fn conditional_source_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<super::super::OriginalCompilationLookupAdvice> {
        if tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || tokens.words() != self.words.as_ref()
            || !self.current_original_body()
        {
            return None;
        }
        super::super::original_site_operand_layout_advice(
            &self.site,
            tokens,
            &self.snapshot,
            &self.header.namespace,
            self.config,
        )
    }

    /// Reborrow metadata of this sealed original source body. Unlike the
    /// Native declaration facade, this does not require or mint a reached
    /// invocation site. Its caller retains the exact current source attachment.
    pub(in super::super) fn source_metadata_context(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Result<
        Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        use crate::registry_invocation::{
            OwnedInvocationMetadataContext, RegistryInvocationDecline,
        };
        let refusal = RegistryInvocationDecline::IncompleteResolution;
        let state = &self.snapshot.state;
        if tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || tokens.words() != self.words.as_ref()
            || !self.current_original_body()
            || state.baseline.registry_snapshot.as_ref()
                != Some(&registry.snapshot().semantic_key())
            || state.source_variables.invocation_dialect != Some(self.dialect)
        {
            return Err(refusal);
        }
        let owner = &state.baseline.metadata_context;
        match owner {
            OwnedInvocationMetadataContext::SuppliedSource(input) => owner
                .metadata_context_for_source(registry, self.config, Some(input.unit_profile()))
                .map(Some)
                .ok_or(refusal),
            OwnedInvocationMetadataContext::Standalone => owner
                .metadata_context(registry)
                .map(crate::registry_invocation::InvocationMetadataSelection::context)
                .ok_or(refusal),
            OwnedInvocationMetadataContext::Supplied(_)
            | OwnedInvocationMetadataContext::Unavailable => Err(refusal),
        }
    }

    fn matches_original_bindings(&self, bindings: &SourceCommandBindings) -> bool {
        let Some(actual) = bindings
            .declaration_layouts
            .get(&self.header.declaration)
            .and_then(|rows| original_declaration_layouts(rows))
        else {
            return false;
        };
        bindings.lexer_config == Some(self.config)
            && bindings
                .source_origin()
                .is_some_and(|source| Arc::ptr_eq(source, &self.site.source))
            && bindings.source_metadata_owner() == &self.snapshot.state.baseline.metadata_context
            && actual.eq(self.parents.iter())
            && self.current_original_body()
    }

    fn current_original_body(&self) -> bool {
        let ProcedureReadSource::ConfiguredSourceWord {
            producer,
            correspondence,
        } = &self.read.source
        else {
            return false;
        };
        self.source_operations.iter().all(|operation| {
            Arc::ptr_eq(&operation.site.source, &self.site.source)
                && operation.site.offset < self.site.offset
                && operation.config == self.config
                && !operation.words.is_empty()
        }) && original_declaration_layouts(&self.parents).is_some()
            && self.parents.iter().all(|parent| {
                parent.config == self.config
                    && parent.snapshot.state.current_source_origin.as_ref()
                        == Some(&self.header.declaration.source)
            })
            && Arc::ptr_eq(&self.header.declaration.source, &self.site.source)
            && producer.source_image() == self.site.source.source_image()
            && producer.lexer_config() == self.config
            && correspondence.bytes() == producer.bytes()
            && tcl_syntax::backslash::native_source_literal_bytes(
                self.header.value.as_bytes(),
                producer.source_image().channel(),
                producer.policy().string_protocol(),
            )
            .is_ok_and(|value| value.as_ref() == self.native_header_value.as_ref())
            && self.snapshot.state.current_source_origin.as_ref() == Some(&self.site.source)
            && super::super::SourceCommandBindings::context_owns_frame(
                &self.snapshot.state.source_variables,
                &self.header.frame,
                Some(&self.header.namespace),
            )
            && self.snapshot.state.variable_frame == self.header.frame
    }

    pub(in super::super) fn variable_name_is_eligible(
        &self,
        registry: &tcl_registry::CommandRegistry,
        name: &str,
    ) -> bool {
        if !self.current_original_body()
            || self
                .read
                .parameter_names()
                .iter()
                .any(|formal| formal == name)
            || super::super::declaration_layout::local_read_scope_is_excluded(
                &self.snapshot.state.source_variables,
                name,
            )
        {
            return false;
        }
        let Some(metadata) = self
            .snapshot
            .state
            .baseline
            .metadata_context
            .metadata_context_for_source(registry, self.config, registry.profile())
        else {
            return false;
        };
        let Some(body) = self.read.original_body_text() else {
            return false;
        };
        let effects = crate::ir_helpers::script_value_name_ownership_with_metadata_context(
            body,
            registry,
            &self.snapshot.state,
            &crate::ir::ExecutionNamespace::SourceContext(self.header.namespace.clone()),
            metadata,
            self.config,
            crate::script_binds::Ownership::ScopeAliasesAndTraceRegistrations,
        );
        !effects.opaque && !effects.names.contains(&name.to_owned())
    }
}

impl SourceCommandBindings {
    pub(in super::super) fn original_declaration_source_operand_attachment(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalDeclarationSourceOperandAttachment> {
        let owner = self.original_analysis_owner.as_ref()?;
        let cache = owner
            .derived_memo()
            .source_operand_layouts
            .get()?
            .upgrade()?;
        if !Arc::ptr_eq(&cache, &self.original_source_operand_layouts)
            || self.lexer_config != Some(owner.config)
            || self.source_metadata_owner() != &owner.metadata
            || !std::sync::Weak::ptr_eq(
                cache.final_state.get()?,
                &Arc::downgrade(&self.final_state),
            )
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let entries = cache
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let layout = entries.get(&offset)?.as_ref()?;
        if tokens.words() != layout.words.as_ref() || !layout.matches_original_bindings(self) {
            return None;
        }
        Some(OriginalDeclarationSourceOperandAttachment {
            cache: Arc::downgrade(&cache),
            offset,
        })
    }

    pub(in super::super) fn retain_original_declaration_source_operand_layouts(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let Some(owner) = &self.original_analysis_owner else {
            return;
        };
        if self.lexer_config != Some(owner.config)
            || self
                .source_origin()
                .is_none_or(|source| !Arc::ptr_eq(source, &owner.source))
            || self.source_metadata_owner() != &owner.metadata
            || self.final_state.baseline.registry_snapshot.as_ref()
                != Some(&registry.snapshot().semantic_key())
        {
            return;
        }
        let cache = &self.original_source_operand_layouts;
        let mut retained = BTreeMap::new();
        for (site, rows) in &self.declaration_layouts {
            if !Arc::ptr_eq(&site.source, &owner.source) {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let parents: Arc<[DeclarationLayoutObservation]> =
                rows.cloned().collect::<Vec<_>>().into();
            let mut agreed = None;
            let mut complete = true;
            for row in parents.iter() {
                let Some(layouts) =
                    original_source_operand_layout_for_declaration(site, row, &parents, registry)
                else {
                    complete = false;
                    break;
                };
                if agreed.as_ref().is_some_and(|previous| previous != &layouts) {
                    complete = false;
                    break;
                }
                agreed = Some(layouts);
            }
            if !complete {
                continue;
            }
            for layout in agreed.into_iter().flatten() {
                match retained.entry(layout.site.offset) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(Some(Arc::new(layout)));
                    }
                    std::collections::btree_map::Entry::Occupied(mut entry) => {
                        if entry.get().as_deref() != Some(&layout) {
                            entry.insert(None);
                        }
                    }
                }
            }
        }
        if retained.is_empty() {
            return;
        }
        let final_state = cache
            .final_state
            .get_or_init(|| Arc::downgrade(&self.final_state));
        if !std::sync::Weak::ptr_eq(final_state, &Arc::downgrade(&self.final_state)) {
            return;
        }
        let memo = owner.derived_memo();
        let linked = memo
            .source_operand_layouts
            .get_or_init(|| Arc::downgrade(cache));
        if !std::sync::Weak::ptr_eq(linked, &Arc::downgrade(cache)) {
            return;
        }
        let mut entries = cache
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (offset, layout) in retained {
            match entries.entry(offset) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(layout);
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    if entry.get() != &layout {
                        entry.insert(None);
                    }
                }
            }
        }
    }
}

struct PreparedOriginalOperandSource {
    content: tcl_lexer::Span,
    image: tcl_lexer::SourceImage,
    native_header_value: Arc<[u8]>,
    pub(super) read: OriginalProcedureReadDeclaration,
}

fn prepare_original_operand_source(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    row: &DeclarationLayoutObservation,
    target: &super::super::SourceCommandTarget,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    shape: tcl_registry::native_procedure::NativeProcedureDefinition,
    header: &DeclaredProcedureHeader,
) -> Option<PreparedOriginalOperandSource> {
    let crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(producer) =
        original_declaration_operand_input(
            site,
            tokens,
            row,
            target,
            effective,
            shape.body_at.checked_add(1)?,
        )?
    else {
        return None;
    };
    let word = producer.original_word();
    if word.group().kind != tcl_lexer::WordKind::Braced
        || word.image() != site.source.source_image()
        || word.config() != row.config
    {
        return None;
    }
    let content = word.content_span().ok()?;
    let raw = word.image().bytes().get(content.as_range())?;
    // Existing unchanged Native bodies keep their original owner terminally.
    // This facet is solely for an original body transformed at declaration.
    if raw == header.value.as_bytes() {
        return None;
    }
    let image = tcl_lexer::SourceImage::from_bytes(raw, site.source.source_image().channel());
    let correspondence = tcl_syntax::backslash::native_source_braced_word_map(
        raw,
        word.image().channel(),
        row.config.brace_backslash_newline,
        producer.policy().string_protocol(),
    )
    .ok()?;
    if correspondence.bytes() != producer.bytes() {
        return None;
    }
    let native_header_value: Arc<[u8]> = tcl_syntax::backslash::native_source_literal_bytes(
        header.value.as_bytes(),
        word.image().channel(),
        producer.policy().string_protocol(),
    )
    .ok()?
    .into_owned()
    .into();
    let read = OriginalProcedureReadDeclaration {
        source: ProcedureReadSource::ConfiguredSourceWord {
            producer,
            correspondence,
        },
        formals: ProcedureReadFormals::Native(header.original_parameters.clone()?),
    };
    Some(PreparedOriginalOperandSource {
        content,
        image,
        native_header_value,
        read,
    })
}

#[derive(Clone, Copy)]
struct OriginalOperandCommandContext<'a> {
    pub(super) site: &'a CommandAllocationSite,
    row: &'a DeclarationLayoutObservation,
    pub(super) parents: &'a Arc<[DeclarationLayoutObservation]>,
    pub(super) registry: &'a tcl_registry::CommandRegistry,
    pub(super) dialect: tcl_registry::InvocationDialect,
    pub(super) read: &'a OriginalProcedureReadDeclaration,
    pub(super) header: &'a DeclaredProcedureHeader,
    pub(super) snapshot: &'a Arc<SourceLookupSnapshot>,
    native_header_value: &'a Arc<[u8]>,
    image: &'a tcl_lexer::SourceImage,
    content: tcl_lexer::Span,
    plan: &'a tcl_lexer::NativeScriptWordsPlan,
}

struct OriginalOperandCommandLayouts {
    retained: Vec<OriginalDeclarationSourceOperandLayout>,
    source_operations: Vec<OriginalDeclarationSourceOperation>,
}

fn original_operand_command_layouts(
    context: OriginalOperandCommandContext<'_>,
) -> Option<OriginalOperandCommandLayouts> {
    let OriginalOperandCommandContext {
        site,
        row,
        parents,
        registry,
        dialect,
        read,
        header,
        snapshot,
        native_header_value,
        image,
        content,
        plan,
    } = context;
    if plan.fatal_tail.is_some() {
        return None;
    }
    let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
        image,
        content.start(),
        row.config.nested(),
    )?;
    let mut retained = Vec::new();
    let mut source_operations = Vec::new();
    for segment in segments {
        if segment.is_partial {
            return None;
        }
        let command =
            super::super::source_command_tokens_boxed(image, content.start(), row.config, &segment);
        let offset = command.words().first()?.source().span.start();
        let projection =
            site.source
                .original_lexical_command(command.words(), offset, row.config)?;
        if !plan
            .commands
            .iter()
            .any(|original| original.words.as_slice() == projection.native_words())
        {
            return None;
        }
        let child = CommandAllocationSite {
            source: Arc::clone(&site.source),
            offset,
        };
        retained.push(OriginalDeclarationSourceOperandLayout {
            site: child.clone(),
            config: row.config,
            words: Arc::from(command.words()),
            dialect,
            read: (*read).clone(),
            header: (*header).clone(),
            snapshot: Arc::clone(snapshot),
            native_header_value: Arc::clone(native_header_value),
            parents: Arc::clone(parents),
            source_operations: Arc::from(source_operations.as_slice()),
        });
        if source_operations.is_empty()
            && retains_successor_layout(
                &child,
                &command,
                snapshot,
                &header.namespace,
                row.config,
                registry,
            )
        {
            continue;
        }
        let Some(operation) =
            original_source_operation_continuation(retained.last()?, &command, registry)
        else {
            break;
        };
        source_operations.push(operation);
    }
    Some(OriginalOperandCommandLayouts {
        retained,
        source_operations,
    })
}

fn original_operand_body_snapshot(
    site: &CommandAllocationSite,
    row: &DeclarationLayoutObservation,
    header: &DeclaredProcedureHeader,
    registry: &tcl_registry::CommandRegistry,
) -> Option<Arc<SourceLookupSnapshot>> {
    let mut state = row.snapshot.state.clone();
    state.variable_frame = header.frame.clone();
    state.source_variables = Arc::new(
        state
            .source_variables
            .in_frame(&header.frame)
            .with_namespace_identity(header.namespace.clone()),
    );
    header
        .original_parameters
        .as_ref()?
        .seed_unknown(Arc::make_mut(&mut state.source_variables), registry);
    state.current_source_origin = Some(Arc::clone(&site.source));
    let snapshot = Arc::new(SourceLookupSnapshot::in_realm(state, row.snapshot.realm));
    Some(snapshot)
}

fn original_source_operand_layout_for_declaration(
    site: &CommandAllocationSite,
    row: &DeclarationLayoutObservation,
    parents: &Arc<[DeclarationLayoutObservation]>,
    registry: &tcl_registry::CommandRegistry,
) -> Option<Vec<OriginalDeclarationSourceOperandLayout>> {
    let tokens = declaration_tokens(site, row)?;
    let advice = super::super::original_site_operand_layout_advice(
        site,
        &tokens,
        &row.snapshot,
        &row.namespace,
        row.config,
    )?;
    let metadata = advice
        .original_declaration_metadata_context(registry, &tokens)
        .ok()?;
    let dialect = advice.dialect();
    let mut selected = None;
    for target in advice.targets() {
        if !target.registry_backed || !target.prepended.is_empty() {
            return None;
        }
        let effective = effective_words_for_target(&tokens, target)?;
        let values = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect::<Vec<_>>();
        let words = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect::<Vec<_>>();
        let RegistryInvocationResolution::Resolved(facts) =
            crate::registry_invocation::resolve_registry_words_in_realm_with_metadata_context(
                registry,
                metadata,
                &words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?
        else {
            return None;
        };
        let tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(shape) =
            facts.procedure_definition?
        else {
            return None;
        };
        let header = procedure_declaration_header(
            site,
            &tokens,
            row,
            &facts,
            target,
            &ProcedureOperands {
                effective: &effective,
                values: &values,
                dialect,
            },
        )?;
        let PreparedOriginalOperandSource {
            content,
            image,
            native_header_value,
            read,
        } = prepare_original_operand_source(
            site, &tokens, row, target, &effective, shape, &header,
        )?;
        let snapshot = original_operand_body_snapshot(site, row, &header, registry)?;
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            site.source.source_image().clone(),
            content,
            row.config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        let OriginalOperandCommandLayouts {
            retained,
            source_operations: _source_operations,
        } = original_operand_command_layouts(OriginalOperandCommandContext {
            site,
            row,
            parents,
            registry,
            dialect,
            read: &read,
            header: &header,
            snapshot: &snapshot,
            native_header_value: &native_header_value,
            image: &image,
            content,
            plan: &plan,
        })?;
        if selected
            .as_ref()
            .is_some_and(|previous| previous != &retained)
        {
            return None;
        }
        selected = Some(retained);
    }
    selected
}

fn original_source_operation_continuation(
    layout: &OriginalDeclarationSourceOperandLayout,
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> Option<OriginalDeclarationSourceOperation> {
    let advice = layout.conditional_source_advice(tokens)?;
    let metadata = layout.source_metadata_context(registry, tokens).ok()?;
    let dialect = layout.dialect;
    let config = layout.config;
    let mut agreed = None;
    for target in advice.targets() {
        if !target.registry_backed || !target.prepended.is_empty() {
            return None;
        }
        let effective = effective_words_for_target(tokens, target)?;
        let values = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(
                    word,
                    config.escapes,
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                )
            })
            .collect::<Vec<_>>();
        let words = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect::<Vec<_>>();
        let invocation = tcl_registry::InvocationWords::structured(*words.first()?, &words[1..])
            .with_dialect(dialect);
        let selected =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                registry,
                metadata.map(crate::registry_invocation::InvocationMetadataContext::context),
                invocation,
                advice.realm(),
            )
            .resolved()?
            .authored_source_operation_continuation(dialect)?;
        if agreed
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return None;
        }
        agreed = Some(selected);
    }
    Some(OriginalDeclarationSourceOperation {
        site: layout.site.clone(),
        config,
        words: Arc::from(tokens.words()),
        operation: agreed?,
    })
}
