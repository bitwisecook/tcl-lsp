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

//! The static read alternatives of an original procedure and the read flow behind each.

use super::super::declaration_layout::{
    DeclarationLayoutObservation, original_declaration_layouts,
};
use super::super::{CommandAllocationSite, SourceCommandBindings};
use super::procedure_header::{
    declaration_tokens, original_declaration_operand_input, procedure_declaration_header,
};
use super::read_declaration::OriginalProcedureReadDeclaration;
use super::{
    OriginalProcedureReadFlowEntry, OriginalProcedureStaticReadAlternatives, ProcedureOperands,
};
use crate::registry_invocation::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    effective_words_for_target,
};
use std::sync::Arc;

pub(super) fn trace_static_read_refusal<T>(stage: &str, value: Option<T>) -> Option<T> {
    #[cfg(debug_assertions)]
    if value.is_none() && tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some()
    {
        eprintln!("RBS_STATIC_REFUSAL stage={stage}");
    }
    #[cfg(not(debug_assertions))]
    let _ = stage;
    value
}

impl OriginalProcedureStaticReadAlternatives {
    #[cfg(debug_assertions)]
    pub(crate) fn trace_static_read_cell(
        &self,
        key: &crate::var_resolve::VariableCellKey,
        name: &str,
    ) {
        use crate::var_resolve::VariableCellKey;
        fn kind(key: &VariableCellKey) -> &'static str {
            match key {
                VariableCellKey::Authored(_) => "authored",
                VariableCellKey::Namespace { .. } => "namespace",
                VariableCellKey::Activation { .. } => "activation",
                VariableCellKey::SelectedFrame { .. } => "selected-frame",
                VariableCellKey::Instance { .. } => "instance",
                VariableCellKey::AllocatedInstance { .. } => "allocated-instance",
                VariableCellKey::RetainedSlot(_) => "retained-slot",
                VariableCellKey::Lifetime { .. } => "lifetime",
                VariableCellKey::Element { .. } => "element",
            }
        }
        eprintln!(
            "RBS_STATIC_CELL name={name} outer={} root={}",
            kind(key),
            kind(key.root()),
        );
        match key.root() {
            VariableCellKey::RetainedSlot(slot) => match slot.as_ref() {
                crate::raw_binding::RawBindingSlotId::Callable {
                    allocation,
                    name: held,
                } => {
                    let expected = &self.declaration.header.declaration;
                    eprintln!(
                        "RBS_STATIC_CELL callable site={} expected_site={} site_equal={} origin_same={} incarnation={:?} generation={} generation_matches_site={} name_equal={}",
                        allocation.site.offset,
                        expected.offset,
                        allocation.site == *expected,
                        Arc::ptr_eq(&allocation.site.source, &expected.source),
                        allocation.incarnation,
                        allocation.implementation_generation,
                        allocation.implementation_generation == expected.offset,
                        held.as_bytes() == name.as_bytes(),
                    );
                }
                crate::raw_binding::RawBindingSlotId::Variable(cell) => {
                    let owner = match &cell.owner {
                        crate::place::CellOwner::CurrentFrame => "current-frame",
                        crate::place::CellOwner::Activation(_) => "activation",
                        crate::place::CellOwner::Namespace(_) => "namespace",
                        crate::place::CellOwner::NamespaceIdentity(_) => "namespace-identity",
                        crate::place::CellOwner::SelectedFrame(_) => "selected-frame",
                        crate::place::CellOwner::RetainedSlot(_) => "retained-slot",
                        crate::place::CellOwner::Instance(_) => "instance",
                        crate::place::CellOwner::AllocatedInstance(_) => "allocated-instance",
                    };
                    eprintln!(
                        "RBS_STATIC_CELL variable owner={owner} generation={:?} name_equal={}",
                        cell.generation,
                        cell.name.as_bytes() == name.as_bytes(),
                    );
                }
            },
            VariableCellKey::Namespace { identity, simple } => eprintln!(
                "RBS_STATIC_CELL namespace declaration_equal={} name_equal={}",
                identity == &self.declaration.header.namespace,
                simple.as_bytes() == name.as_bytes(),
            ),
            _ => {}
        }
    }

    pub(crate) fn matches_original_procedure(
        &self,
        procedure: &crate::ir::Procedure,
        image: &tcl_lexer::SourceImage,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        &self.input == input && self.declaration.owns_original_procedure(procedure, image)
    }

    pub(crate) fn owns_declared_callable_static(
        &self,
        key: &crate::var_resolve::VariableCellKey,
        name: &str,
    ) -> bool {
        let crate::var_resolve::VariableCellKey::RetainedSlot(slot) = key else {
            return false;
        };
        let crate::raw_binding::RawBindingSlotId::Callable {
            allocation,
            name: held,
        } = slot.as_ref()
        else {
            return false;
        };
        let declaration = &self.declaration.header.declaration;
        self.names.contains(name)
            && allocation.incarnation == super::super::AllocationIncarnation::First
            && allocation.site.offset == declaration.offset
            && Arc::ptr_eq(&allocation.site.source, &declaration.source)
            && allocation.implementation_generation == declaration.offset
            && held.as_bytes() == name.as_bytes()
    }

    pub(crate) fn may_select_static(&self, name: &str, span: tcl_lexer::Span) -> bool {
        self.names.contains(name) && self.declaration.read.owns_original_read(span)
    }

    /// Original direct argument reads have a source-relative local frame even
    /// when readonly CFG entry has no physical activation. This does not turn
    /// an authored compatibility key into a native address or an existing value.
    pub(crate) fn may_select_direct_source_read(
        &self,
        view: crate::ssa::SsaSourceView<'_>,
        symbol: crate::ssa::Symbol,
        name: &str,
        span: tcl_lexer::Span,
    ) -> bool {
        self.direct_source_read(view, symbol, name, span).is_some()
    }

    fn direct_source_read(
        &self,
        view: crate::ssa::SsaSourceView<'_>,
        symbol: crate::ssa::Symbol,
        name: &str,
        span: tcl_lexer::Span,
    ) -> Option<()> {
        static_read_guard!(
            "source_read_scope",
            self.may_select_static(name, span).then_some(())
        );
        let key = static_read_guard!("cell_key", view.function().cell_key(symbol));
        static_read_guard!(
            "cell_owner",
            (matches!(key, crate::var_resolve::VariableCellKey::Authored(_))
                || self.owns_declared_callable_static(key, name))
            .then_some(())
        );
        static_read_guard!(
            "source_symbol",
            (static_read_guard!("source_names", view.source_names()).get(name) == Some(&symbol))
                .then_some(())
        );
        let tokens = static_read_guard!("source_tokens", view.source_tokens());
        let binding = static_read_guard!("source_binding", tokens.source_binding.as_ref());
        static_read_guard!(
            "analysis_owner",
            Arc::ptr_eq(
                static_read_guard!(
                    "analysis_attachment",
                    binding.original_analysis_attachment()
                ),
                &self.owner
            )
            .then_some(())
        );
        let registry = self.input.borrowed_context_registry().commands();
        let metadata = static_read_guard!(
            "invocation_metadata",
            binding.original_invocation_metadata_for_input(tokens, &self.input, registry)
        );
        let advice = static_read_guard!("operand_advice", {
            #[cfg(debug_assertions)]
            let _trace_scope = super::super::RbsOperandTraceScope::enter();
            binding.original_conditional_operand_layout_advice(tokens, registry)
        });
        if advice.has_unclosed_source_coverage() {
            return trace_static_read_refusal("unclosed_source_coverage", None);
        }
        let context = static_read_guard!(
            "declaration_metadata_result",
            advice
                .original_declaration_metadata_context(registry, tokens)
                .ok()
        );
        let context = static_read_guard!("declaration_metadata", context);
        static_read_guard!(
            "metadata_input",
            (context.source_analysis_input() == metadata.source_analysis_input()).then_some(())
        );
        let selected = static_read_guard!(
            "invocation_flow",
            crate::registry_invocation::declaration_invocation_flow(registry, tokens, &advice)
        );
        if selected.effects.frame_reachable() {
            return trace_static_read_refusal("frame_effects", None);
        }
        if selected.effects.unknown_reads {
            return trace_static_read_refusal("unknown_reads", None);
        }
        if selected.effects.unknown_writes {
            return trace_static_read_refusal("unknown_writes", None);
        }
        if !selected.aliases.is_empty() {
            return trace_static_read_refusal("aliases", None);
        }
        if !selected.reads.is_empty() {
            return trace_static_read_refusal("named_reads", None);
        }
        if !matches!(
            selected.flow,
            tcl_registry::script_body_flow::ScriptBodyFlow::None
        ) {
            return trace_static_read_refusal("script_flow", None);
        }
        let image = self.declaration.read.original_image();
        let config = self.input.lexer_config();
        let at = static_read_guard!("first_word", tokens.words().first())
            .source()
            .span
            .start();
        let original = static_read_guard!(
            "original_words",
            crate::registry_invocation::original_native_compiler_words(
                image,
                tokens.words(),
                at,
                config
            )
        );
        self.direct_source_read_geometry(&original, name, span)
            .or_else(|| self.original_child_source_read(tokens, &original, metadata, name, span))
    }

    /// One immutable lexical plan for this exact declaration/input owner.
    /// The cached syntax grants no lookup, frame, effect or successful read.
    pub(super) fn original_body_plan(&self) -> Option<&tcl_lexer::NativeScriptWordsPlan> {
        self.original_body_plan
            .get_or_init(|| {
                let image = self.declaration.read.original_image();
                tcl_lexer::native_script_words_in(
                    image.clone(),
                    static_read_guard!(
                        "original_body_span",
                        self.declaration.read.original_body_span()
                    ),
                    self.input.lexer_config(),
                )
                .ok()
            })
            .as_ref()
    }

    fn direct_source_read_geometry(
        &self,
        original: &[tcl_lexer::NativeWord],
        name: &str,
        span: tcl_lexer::Span,
    ) -> Option<()> {
        let plan = static_read_guard!("body_parse", self.original_body_plan());
        if plan.fatal_tail.is_some() {
            return trace_static_read_refusal("fatal_body_tail", None);
        }
        if !plan
            .commands
            .iter()
            .any(|command| command.words.as_slice() == original)
        {
            return trace_static_read_refusal("whole_command_geometry", None);
        }
        let mut found = false;
        for word in original {
            if word.group().expand {
                return trace_static_read_refusal("expanded_word", None);
            }
            if word.group().kind == tcl_lexer::WordKind::Braced {
                return trace_static_read_refusal("braced_word", None);
            }
            let arena = word.executable_parts();
            for part in arena.all_parts() {
                match &part.part {
                    tcl_lexer::ExecutablePart::Text(_) => {}
                    tcl_lexer::ExecutablePart::Variable { name: root, index }
                        if index.is_none() =>
                    {
                        if arena.source_span(part) == Some(span)
                            && static_read_guard!("variable_bytes", arena.bytes(*root))
                                == name.as_bytes()
                        {
                            found = true;
                        }
                    }
                    _ => return trace_static_read_refusal("part_shape", None),
                }
            }
        }
        trace_static_read_refusal("matching_lexical_read", found.then_some(()))
    }

    pub(crate) fn may_select_static_in_declared_frame(
        &self,
        name: &str,
        span: tcl_lexer::Span,
        context: &crate::var_resolve::ResolveContext,
    ) -> bool {
        SourceCommandBindings::context_owns_frame(
            context,
            &self.declaration.header.frame,
            Some(&self.declaration.header.namespace),
        ) && self.may_select_static(name, span)
    }
}

#[derive(PartialEq, Eq)]
struct OriginalStaticReadAlternativeFacet {
    pub(super) declaration: OriginalProcedureReadFlowEntry,
    pub(super) original: Vec<tcl_lexer::NativeWord>,
    pub(super) effective: crate::registry_invocation::EffectiveCommandWords,
    values: Vec<Vec<u8>>,
    pub(super) statics: crate::signature_scan::scope::SignatureSourceNameInput,
    parsed: Vec<tcl_registry::native_procedure::ByteStaticVariableDeclaration>,
}

impl OriginalProcedureReadFlowEntry {
    pub(in super::super) fn owns_source(
        &self,
        origin: &Arc<super::super::SourceOriginId>,
        offset: u32,
    ) -> bool {
        &self.header.declaration.source == origin
            && self
                .read
                .original_body_span()
                .is_some_and(|body| offset < body.end())
            && self
                .read
                .owns_original_read(tcl_lexer::Span::new(offset, offset))
    }
    pub(in super::super) fn owns_original_procedure(
        &self,
        procedure: &crate::ir::Procedure,
        image: &tcl_lexer::SourceImage,
    ) -> bool {
        self.header.declaration.offset == procedure.span.start()
            && self.read.original_image() == image
            && self
                .read
                .original_body_span()
                .is_some_and(|span| span.start() == procedure.body_offset)
            && self.read.original_body_text() == procedure.body_source.as_deref()
            && self.read.parameters_text() == Some(procedure.params_raw.as_str())
            && self.read.parameter_names() == procedure.params
    }
}

impl SourceCommandBindings {
    /// A possible static lookup is independent of successful construction.
    /// Every original observation and target must retain the same full argv,
    /// static operand, source/formal/body recipe and caller-variable context.
    pub(crate) fn original_procedure_static_read_alternatives(
        self: &Arc<Self>,
        procedure: &crate::ir::Procedure,
        image: &tcl_lexer::SourceImage,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<OriginalProcedureStaticReadAlternatives> {
        if procedure.body_kind != crate::ir::ProcedureBodyKind::OriginalDeclaration
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let binding = self.invocation_at_source("", procedure.span.start());
        let (_, mut tokens) = binding.original_recorded_command()?;
        tokens.source_binding = Some(binding);
        let site = tokens.source_binding.as_ref()?.invocation_site()?;
        (procedure.declaration_site.as_ref() == Some(site)).then_some(())?;
        let owner = self.original_analysis_owner()?;
        owner.matches(self, input).then_some(())?;
        let read = self.original_procedure_read_declaration(&tokens, registry, input)?;
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, input,
        )?;
        let mut agreed = None;
        for row in original_declaration_layouts(self.declaration_layouts.get(site)?)? {
            if row.config != input.lexer_config()
                || declaration_tokens(site, row)?.words() != tokens.words()
            {
                return None;
            }
            let advice = super::super::original_site_operand_layout_advice(
                site,
                &tokens,
                &row.snapshot,
                &row.namespace,
                row.config,
            )?;
            (advice.dialect() == super::super::source_analysis_entry::source_input_dialect(input))
                .then_some(())?;
            for target in advice.targets() {
                let facet = original_static_read_alternative_facet(
                    site,
                    &tokens,
                    row,
                    target,
                    &advice,
                    &OriginalProcedureReadFlowRequest {
                        registry,
                        metadata,
                        read: &read,
                    },
                )?;
                if !facet.declaration.owns_original_procedure(procedure, image)
                    || procedure.body.namespace_context.as_deref()
                        != Some(&facet.declaration.header.namespace)
                    || agreed.as_ref().is_some_and(|old| old != &facet)
                {
                    return None;
                }
                agreed = Some(facet);
            }
        }
        let facet = agreed?;
        let names = facet
            .parsed
            .into_iter()
            .filter_map(|declaration| {
                let name = String::from_utf8(declaration.name).ok()?;
                (!tcl_syntax::naming::is_qualified(name.as_bytes())
                    && tcl_syntax::naming::split_element_ref(&name).is_none())
                .then_some(name)
            })
            .collect();
        Some(OriginalProcedureStaticReadAlternatives {
            source_analysis: Arc::downgrade(self),
            owner: Arc::clone(owner),
            input: input.clone(),
            declaration: facet.declaration,
            names,
            original_body_plan: std::sync::OnceLock::new(),
        })
    }

    pub(in super::super) fn original_procedure_read_flow_entry(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<OriginalProcedureReadFlowEntry> {
        let read = self.original_procedure_read_declaration(tokens, registry, input)?;
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, input,
        )?;
        let site = tokens.source_binding.as_ref()?.invocation_site()?;
        let mut agreed = None;
        for row in original_declaration_layouts(self.declaration_layouts.get(site)?)? {
            if row.config != input.lexer_config()
                || declaration_tokens(site, row)?.words() != tokens.words()
            {
                return None;
            }
            let advice = super::super::original_site_operand_layout_advice(
                site,
                tokens,
                &row.snapshot,
                &row.namespace,
                row.config,
            )?;
            if advice.dialect() != super::super::source_analysis_entry::source_input_dialect(input)
            {
                return None;
            }
            for target in advice.targets() {
                let entry = procedure_read_flow_for_target(
                    site,
                    tokens,
                    row,
                    target,
                    &advice,
                    &OriginalProcedureReadFlowRequest {
                        registry,
                        metadata,
                        read: &read,
                    },
                )?;
                if agreed.as_ref().is_some_and(|previous| previous != &entry) {
                    return None;
                }
                agreed = Some(entry);
            }
        }
        agreed
    }
}

struct OriginalProcedureReadFlowRequest<'a> {
    pub(super) registry: &'a tcl_registry::CommandRegistry,
    pub(super) metadata: crate::registry_invocation::InvocationMetadataContext<'a>,
    pub(super) read: &'a OriginalProcedureReadDeclaration,
}

fn original_static_read_alternative_facet(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    row: &DeclarationLayoutObservation,
    target: &super::super::SourceCommandTarget,
    advice: &super::super::OriginalCompilationLookupAdvice,
    request: &OriginalProcedureReadFlowRequest<'_>,
) -> Option<OriginalStaticReadAlternativeFacet> {
    let declaration = procedure_read_flow_for_target(site, tokens, row, target, advice, request)?;
    let effective = effective_words_for_target(tokens, target)?;
    let dialect = advice.dialect();
    let values = effective
        .words
        .iter()
        .map(|word| {
            effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
                .literal_bytes()
                .map(<[u8]>::to_vec)
        })
        .collect::<Option<Vec<_>>>()?;
    let words = values
        .iter()
        .map(|value| {
            std::str::from_utf8(value)
                .ok()
                .map(tcl_registry::InvocationWord::Literal)
        })
        .collect::<Option<Vec<_>>>()?;
    let (&head, arguments) = words.split_first()?;
    let resolution =
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            request.registry,
            Some(request.metadata.context()),
            tcl_registry::InvocationWords::structured(head, arguments).with_dialect(dialect),
            advice.realm(),
        );
    let at = match resolution
        .resolved()?
        .authored_source_procedure_static_list()?
    {
        tcl_registry::resolved_invocation::AuthoredSourceProcedureStaticList::Argument(at) => at,
        tcl_registry::resolved_invocation::AuthoredSourceProcedureStaticList::Absent => {
            return None;
        }
    };
    matches!(
        effective.origins.get(at.checked_add(1)?),
        Some(crate::registry_invocation::InvocationWordOrigin::Written(_))
    )
    .then_some(())?;
    let statics = original_declaration_operand_input(
        site,
        tokens,
        row,
        target,
        &effective,
        at.checked_add(1)?,
    )?;
    statics
        .is_current(&row.snapshot.state.source_variables)
        .then_some(())?;
    (statics.bytes() == values.get(at.checked_add(1)?)?).then_some(())?;
    let parsed =
        tcl_registry::native_procedure::parse_static_variables_bytes(statics.bytes(), dialect)?
            .ok()?;
    if parsed.iter().any(|declaration| {
        row.snapshot
            .state
            .source_variables
            .original_static_capture_is_definitely_invalid(declaration, request.registry)
    }) {
        return None;
    }
    let original = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        row.config,
    )?;
    Some(OriginalStaticReadAlternativeFacet {
        declaration,
        original,
        effective,
        values,
        statics,
        parsed,
    })
}

fn procedure_read_flow_for_target(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    row: &DeclarationLayoutObservation,
    target: &super::super::SourceCommandTarget,
    advice: &super::super::OriginalCompilationLookupAdvice,
    request: &OriginalProcedureReadFlowRequest<'_>,
) -> Option<OriginalProcedureReadFlowEntry> {
    if !target.registry_backed {
        return None;
    }
    let effective = effective_words_for_target(tokens, target)?;
    let dialect = advice.dialect();
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
            request.registry,
            Some(request.metadata),
            &words,
            Some(dialect),
            advice.realm(),
        )
        .ok()?
    else {
        return None;
    };
    let header = procedure_declaration_header(
        site,
        tokens,
        row,
        &facts,
        target,
        &ProcedureOperands {
            effective: &effective,
            values: &values,
            dialect,
        },
    )?;
    if header
        .parameters
        .iter()
        .map(|formal| formal.name.as_str())
        .collect::<Vec<_>>()
        != request
            .read
            .parameter_names()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    {
        return None;
    }
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
        .seed_unknown(Arc::make_mut(&mut state.source_variables), request.registry);
    state.current_source_origin = Some(Arc::clone(&site.source));
    Some(OriginalProcedureReadFlowEntry {
        header,
        read: request.read.clone(),
        snapshot: Arc::new(super::super::SourceLookupSnapshot::in_realm(
            state,
            row.snapshot.realm,
        )),
    })
}
