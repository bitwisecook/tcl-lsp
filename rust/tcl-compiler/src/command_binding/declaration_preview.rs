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

//! Original procedure-body layouts independent of command installation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::declaration_layout::{
    DeclarationLayoutIssuer, DeclarationLayoutObservation, OriginalDiagnosticFrameEntry,
    original_declaration_layouts,
};
use super::{
    CommandAllocationSite, ExecutedScriptSource, SourceCommandBindings, SourceLookupSnapshot,
};
use crate::registry_invocation::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    effective_words_for_target, resolve_registry_words_in_realm,
};

/// A positioned conditional declaration recipe, never an implementation allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclaredProcedureBody {
    declaration: CommandAllocationSite,
    candidate: super::SourceCommandTarget,
    slot: super::SourceCommandKey,
    pub(super) source: ExecutedScriptSource,
    pub(super) namespace: super::SourceNamespaceKey,
    pub(super) parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    logical_name: Option<String>,
    logical_parameters_text: Option<String>,
    pub(super) original_parameters: Option<super::formal_topology::OriginalFormalTopology>,
    pub(super) frame: crate::var_resolve::VariableExecutionFrame,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclaredProcedureHeader {
    pub(super) declaration: CommandAllocationSite,
    candidate: super::SourceCommandTarget,
    slot: super::SourceCommandKey,
    pub(super) namespace: super::SourceNamespaceKey,
    pub(super) parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    pub(super) original_parameters: Option<super::formal_topology::OriginalFormalTopology>,
    pub(super) frame: crate::var_resolve::VariableExecutionFrame,
    written: usize,
    value: String,
}

/// Checked original body scope for conditional source diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalProcedureReadFlowEntry {
    pub(super) header: DeclaredProcedureHeader,
    pub(super) read: OriginalProcedureReadDeclaration,
    pub(super) snapshot: Arc<super::SourceLookupSnapshot>,
}

impl OriginalProcedureReadFlowEntry {
    pub(super) fn owns_source(&self, origin: &Arc<super::SourceOriginId>, offset: u32) -> bool {
        &self.header.declaration.source == origin
            && self
                .read
                .original_body_span()
                .is_some_and(|body| offset < body.end())
            && self
                .read
                .owns_original_read(tcl_lexer::Span::new(offset, offset))
    }
    pub(super) fn owns_original_procedure(
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

impl DeclaredProcedureBody {
    /// Closed formal-name geometry in the retained Logical declaration. This
    /// does not select Native formal storage, an entered frame or alias cells.
    pub(super) fn logical_alias_local_name_is_byte_unique(
        &self,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        use tcl_syntax::naming::{NativeNameContext, NativeVariableRootGeometry};
        if policy.authority() != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
            || self.original_parameters.is_some()
            || self.logical_name.is_none()
        {
            return false;
        }
        let Some(text) = self.logical_parameters_text.as_deref() else {
            return false;
        };
        if super::logical_definition::static_formals(text, config.list_parse).as_ref()
            != Some(&self.parameters)
        {
            return false;
        }
        let protocol = policy.recipe();
        self.parameters.iter().all(|formal| {
            let input = protocol.combined_variable_input(formal.name.as_bytes());
            input.element().is_none()
                && matches!(
                    protocol.variable_root_geometry(NativeNameContext::root(), input.root().selected()),
                    NativeVariableRootGeometry::Local(name) if name.as_bytes() == formal.name.as_bytes()
                )
        })
    }
}

/// A sealed declaration in the independently retained Logical source model.
/// It describes an original source slot, strict formals and deferred body;
/// it grants no Native publication, formal ABI, entered frame or completion.
#[derive(Debug, Clone)]
pub(crate) struct LogicalProcedureDeclaration {
    body: DeclaredProcedureBody,
}

impl LogicalProcedureDeclaration {
    pub(crate) fn qualified_name(&self) -> Option<&str> {
        self.body.slot.authored_spelling()
    }
    pub(crate) fn name(&self) -> Option<&str> {
        self.body.logical_name.as_deref()
    }
    pub(crate) fn parameters_text(&self) -> Option<&str> {
        self.body.logical_parameters_text.as_deref()
    }
    pub(crate) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        &self.body.parameters
    }
    pub(crate) fn source(&self) -> &ExecutedScriptSource {
        &self.body.source
    }
    pub(crate) fn namespace(&self) -> &super::SourceNamespaceKey {
        &self.body.namespace
    }
}

/// Original declaration applicability for existential local-read advice.
/// Native formals retain their selected byte topology; Logical formals retain
/// their distinct authored declaration recipe. Neither supplies body entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalProcedureReadDeclaration {
    source: ProcedureReadSource,
    formals: ProcedureReadFormals,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum ProcedureReadSource {
    Contiguous(ExecutedScriptSource),
    OriginalBraced {
        producer: crate::signature_scan::scope::SignatureSourceNameKey,
        correspondence: tcl_syntax::backslash::NativeBracedWordSourceMap,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum ProcedureReadFormals {
    Native(super::formal_topology::OriginalFormalTopology),
    Logical {
        parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
        text: String,
    },
}
impl OriginalProcedureReadDeclaration {
    pub(crate) fn original_image(&self) -> &tcl_lexer::SourceImage {
        match &self.source {
            ProcedureReadSource::Contiguous(source) => source.origin.source_image(),
            ProcedureReadSource::OriginalBraced { producer, .. } => {
                producer.original_word().image()
            }
        }
    }
    pub(crate) fn original_body_span(&self) -> Option<tcl_lexer::Span> {
        match &self.source {
            ProcedureReadSource::Contiguous(source) => Some(tcl_lexer::Span::new(
                source.base(),
                source
                    .base()
                    .checked_add(u32::try_from(source.text.len()).ok()?)?,
            )),
            ProcedureReadSource::OriginalBraced { producer, .. } => {
                producer.original_word().content_span().ok()
            }
        }
    }
    pub(crate) fn original_body_text(&self) -> Option<&str> {
        std::str::from_utf8(
            self.original_image()
                .bytes()
                .get(self.original_body_span()?.as_range())?,
        )
        .ok()
    }
    pub(crate) fn owns_original_read(&self, span: tcl_lexer::Span) -> bool {
        let Some(body) = self.original_body_span() else {
            return false;
        };
        if span.start() < body.start() || span.end() > body.end() {
            return false;
        }
        match &self.source {
            ProcedureReadSource::Contiguous(_) => true,
            ProcedureReadSource::OriginalBraced { correspondence, .. } => correspondence
                .native_extent(
                    (span.start() - body.start()) as usize..(span.end() - body.start()) as usize,
                )
                .is_some(),
        }
    }
    pub(crate) fn parameter_names(&self) -> Vec<String> {
        match &self.formals {
            ProcedureReadFormals::Native(topology) => topology.advisory_parameters(),
            ProcedureReadFormals::Logical { parameters, .. } => parameters.clone(),
        }
        .into_iter()
        .map(|formal| formal.name)
        .collect()
    }
    pub(crate) fn parameters_text(&self) -> Option<&str> {
        match &self.formals {
            ProcedureReadFormals::Native(topology) => {
                std::str::from_utf8(topology.original_input().bytes()).ok()
            }
            ProcedureReadFormals::Logical { text, .. } => Some(text),
        }
    }
}

/// One lexical body under its complete immutable parent lookup worlds.
/// The registry is fixed for this collector. Parent command words are replaced
/// by genuine child words before any semantic query, so they are not a body
/// traversal input. Entry ownership, issuer, realm, state, namespace and lexer
/// config remain in structural equality; hashes alone never select a visit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NestedDeclarationLayoutKey {
    source: ExecutedScriptSource,
    parents: Vec<DeclarationLayoutObservation>,
}

impl NestedDeclarationLayoutKey {
    fn new(source: &ExecutedScriptSource, parents: &[DeclarationLayoutObservation]) -> Self {
        let parents = parents
            .iter()
            .map(|parent| DeclarationLayoutObservation {
                words: Arc::from([]),
                ..parent.clone()
            })
            .collect();
        Self {
            source: source.clone(),
            parents,
        }
    }
}

/// Pure original source geometry. Lookup worlds remain separate traversal inputs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NestedLayoutSyntaxKey {
    source: ExecutedScriptSource,
    config: tcl_lexer::LexerConfig,
}

struct NestedLayoutCommand {
    site: CommandAllocationSite,
    tokens: Box<crate::ir::CommandTokens>,
}

/// A collector-local source tape, with no selected invocation or read/store facts.
#[derive(Default)]
struct NestedLayoutSyntax {
    commands: HashMap<NestedLayoutSyntaxKey, Option<Arc<[NestedLayoutCommand]>>>,
    #[cfg(test)]
    parsed_scripts: usize,
}

impl NestedLayoutSyntax {
    fn commands(
        &mut self,
        source: &ExecutedScriptSource,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Arc<[NestedLayoutCommand]>> {
        let key = NestedLayoutSyntaxKey {
            source: source.clone(),
            config,
        };
        if let Some(commands) = self.commands.get(&key) {
            return commands.clone();
        }
        #[cfg(test)]
        {
            self.parsed_scripts += 1;
        }
        let commands = Self::parse(source, config);
        self.commands.insert(key, commands.clone());
        commands
    }

    fn parse(
        source: &ExecutedScriptSource,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Arc<[NestedLayoutCommand]>> {
        let segments = nested_layout_segments(source, config)?;
        Some(
            segments
                .iter()
                .take_while(|segment| !segment.is_partial)
                .map(|segment| NestedLayoutCommand {
                    site: CommandAllocationSite {
                        source: Arc::clone(&source.origin),
                        offset: segment.span.start(),
                    },
                    tokens: super::source_command_tokens_boxed(
                        &source.text,
                        source.base(),
                        config,
                        segment,
                    ),
                })
                .collect::<Vec<_>>()
                .into(),
        )
    }
}

/// Local parser purpose for an authentic body value, including materialised
/// scripts at offset zero. Parent token/metadata/cache configurations remain
/// complete and unchanged; the lexical view supplies no selected invocation.
fn nested_layout_segments(
    source: &ExecutedScriptSource,
    parent_config: tcl_lexer::LexerConfig,
) -> Option<Vec<crate::segmenter::SegmentedCommand>> {
    let local_config = tcl_lexer::SourceSegmentationPurpose::NestedScript
        .config(parent_config)
        .normalized();
    let tokens = tcl_lexer::Lexer::with_source_image(&source.text, local_config)
        .tokenise_all()
        .ok()?;
    let commands = tcl_lexer::group_commands_bytes(&tokens, source.text.bytes(), local_config);
    let cut =
        tcl_lexer::first_parse_cut_image_in_checked(&commands, &tokens, &source.text, local_config)
            .ok()?;
    let prefix = if let Some(cut) = cut {
        // The ordinal belongs to original lexical groups. Select their complete
        // byte prefix before F5 lookahead can merge a malformed following clause.
        let command_start = usize::try_from(commands.get(cut.command)?.span.start()).ok()?;
        tcl_lexer::SourceImage::from_bytes(
            source.text.bytes().get(..command_start)?,
            source.text.channel(),
        )
    } else {
        source.text.clone()
    };
    source
        .base()
        .checked_add(u32::try_from(prefix.bytes().len()).ok()?)?;
    crate::segmenter::segment_commands_image_with_offset_and_config(
        &prefix,
        source.base(),
        local_config,
    )
}

/// One original command layout in its complete immutable lookup world.
/// Children and continuation share this derivation; it grants no body entry.
struct OriginalDeclarationLayout {
    advice: super::OriginalCompilationLookupAdvice,
    flow: crate::registry_invocation::DeclarationInvocationFlow,
}

impl OriginalDeclarationLayout {
    fn select(
        site: &CommandAllocationSite,
        tokens: &crate::ir::CommandTokens,
        snapshot: &Arc<SourceLookupSnapshot>,
        namespace: &super::SourceNamespaceKey,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Self> {
        let advice =
            super::original_site_operand_layout_advice(site, tokens, snapshot, namespace, config)?;
        let flow =
            crate::registry_invocation::declaration_invocation_flow(registry, tokens, &advice)?;
        Some(Self { advice, flow })
    }

    fn retains_successor(&self) -> bool {
        self.flow.effects.lookup_stable
            && !self.flow.effects.unknown_writes
            && matches!(
                self.flow.flow,
                tcl_registry::script_body_flow::ScriptBodyFlow::None
            )
            && self.flow.effective.words.iter().all(|word| {
                effective_invocation_word(
                    word,
                    self.advice.dialect().lexer_grammar.escapes,
                    self.advice.dialect().word_values,
                )
                .literal_bytes()
                .is_some()
            })
    }
}

struct ParentDeclarationLayout<'a> {
    parent: &'a DeclarationLayoutObservation,
    original: Option<OriginalDeclarationLayout>,
}

fn parent_declaration_layouts<'a>(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    parents: &'a [DeclarationLayoutObservation],
    registry: &tcl_registry::CommandRegistry,
) -> Vec<ParentDeclarationLayout<'a>> {
    parents
        .iter()
        .map(|parent| ParentDeclarationLayout {
            parent,
            original: OriginalDeclarationLayout::select(
                site,
                tokens,
                &parent.snapshot,
                &parent.namespace,
                parent.config,
                registry,
            ),
        })
        .collect()
}

#[cfg(any(test, debug_assertions))]
struct NestedLayoutTrace {
    started: Option<std::time::Instant>,
}

#[cfg(any(test, debug_assertions))]
impl NestedLayoutTrace {
    fn new() -> Self {
        Self {
            started: std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
                .is_some()
                .then(std::time::Instant::now),
        }
    }

    fn phase(&self, stage: &str, visits: usize, pending: usize, layouts: usize) {
        if let Some(started) = self.started {
            eprintln!(
                "SOURCE_LAYOUT_PHASE stage={stage} ms={} visits={visits} pending={pending} layouts={layouts}",
                started.elapsed().as_millis()
            );
        }
    }
}

impl SourceCommandBindings {
    /// Conditional declaration from complete supplied Logical input and exact
    /// original source observations. Labels cannot reconstruct this receipt.
    pub(crate) fn original_logical_procedure_declaration(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<LogicalProcedureDeclaration> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        if !input.has_logical_source_name_context()
            || input
                .context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
        {
            return None;
        }
        let site = tokens.source_binding.as_ref()?.invocation_site()?;
        for row in original_declaration_layouts(self.declaration_layouts.get(site)?)? {
            if row.config != input.lexer_config()
                || row.snapshot.state.logical_source_name_advice_input() != Some(input)
                || declaration_tokens(site, row)?.words() != tokens.words()
            {
                return None;
            }
        }
        let body = self.original_declared_procedure_at(site, registry)?;
        body.slot.authored_spelling()?;
        body.original_parameters
            .is_none()
            .then_some(LogicalProcedureDeclaration { body })
    }

    /// Selected original header grammar for conditional diagnostic advice.
    /// Namespace publication, allocated procedure and physical frame are not
    /// prerequisites; the retained declaration and formal producer are.
    pub(crate) fn original_procedure_read_declaration(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<OriginalProcedureReadDeclaration> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, input,
        )?;
        let site = tokens.source_binding.as_ref()?.invocation_site()?;
        if !self.matches_original_source_image(site.source.source_image(), input.lexer_config()) {
            return None;
        }
        if tokens
            .source_binding
            .as_ref()?
            .original_source_advice_purpose(tokens, metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical
        {
            let recipe = self.original_logical_procedure_declaration(tokens, registry, input)?;
            return Some(OriginalProcedureReadDeclaration {
                source: ProcedureReadSource::Contiguous(recipe.source().clone()),
                formals: ProcedureReadFormals::Logical {
                    parameters: recipe.parameters().to_vec(),
                    text: recipe.parameters_text()?.to_owned(),
                },
            });
        }
        let mut agreed = None;
        for row in original_declaration_layouts(self.declaration_layouts.get(site)?)? {
            if row.config != input.lexer_config()
                || declaration_tokens(site, row)?.words() != tokens.words()
            {
                return None;
            }
            let advice = super::original_site_operand_layout_advice(
                site,
                tokens,
                &row.snapshot,
                &row.namespace,
                row.config,
            )?;
            if advice.dialect() != super::source_analysis_entry::source_input_dialect(input) {
                return None;
            }
            for target in advice.targets() {
                if !target.registry_backed {
                    return None;
                }
                let recipe = native_procedure_read_recipe(
                    site, tokens, row, target, &advice, registry, metadata,
                )?;
                if agreed.as_ref().is_some_and(|previous| previous != &recipe) {
                    return None;
                }
                agreed = Some(recipe);
            }
        }
        agreed
    }

    pub(super) fn original_procedure_read_flow_entry(
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
            let advice = super::original_site_operand_layout_advice(
                site,
                tokens,
                &row.snapshot,
                &row.namespace,
                row.config,
            )?;
            if advice.dialect() != super::source_analysis_entry::source_input_dialect(input) {
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

    /// Unanimous conditional procedure body at its genuine declaration.
    /// The lexical recipe is independent of installation and entered calls.
    pub(super) fn original_declared_procedure_at(
        &self,
        site: &CommandAllocationSite,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<DeclaredProcedureBody> {
        let observations = original_declaration_layouts(self.declaration_layouts.get(site)?)?;
        let mut agreed = None;
        for observation in observations {
            let tokens = declaration_tokens(site, observation)?;
            let body = declared_body(site, &tokens, observation, registry)?;
            if agreed.as_ref().is_some_and(|previous| previous != &body) {
                return None;
            }
            agreed = Some(body);
        }
        agreed
    }

    /// Unanimous original procedure `ParamList` syntax at its actual declaration
    /// site. This does not select an installation or an entered body.
    pub(crate) fn original_procedure_formals_at(
        &self,
        site: &CommandAllocationSite,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::signature_scan::formal_parameters::SignatureSourceFormalParameters> {
        let observations = original_declaration_layouts(self.declaration_layouts.get(site)?)?;
        let mut agreed = None;
        for observation in observations {
            let tokens = declaration_tokens(site, observation)?;
            let body = declared_body(site, &tokens, observation, registry)?;
            let topology = body.original_parameters?;
            if agreed.as_ref().is_some_and(|old| old != &topology) {
                return None;
            }
            agreed = Some(topology);
        }
        Some(crate::signature_scan::formal_parameters::SignatureSourceFormalParameters::from_original_topology(&agreed?))
    }

    /// Preserve only original lexical layouts when the declaration's runtime
    /// installation is unknown. No dispatch, body entry or effect inventory is
    /// populated by this separate collector.
    pub(super) fn retain_uninstalled_declaration_body_layouts(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        #[cfg(any(test, debug_assertions))]
        let trace = NestedLayoutTrace::new();
        let roots: Vec<_> = self
            .declaration_layouts
            .iter()
            .filter_map(|(site, observations)| {
                let observation = original_declaration_layouts(observations)?.next()?;
                matches!(
                    observation.entry.as_ref(),
                    OriginalDiagnosticFrameEntry::RootScript { .. }
                        | OriginalDiagnosticFrameEntry::ScriptActivation { .. }
                )
                .then(|| (site.clone(), observation.clone()))
            })
            .collect();
        #[cfg(any(test, debug_assertions))]
        trace.phase(
            "uninstalled-roots",
            0,
            roots.len(),
            self.declaration_layouts.len(),
        );
        for (site, observation) in roots {
            let Some(tokens) = declaration_tokens(&site, &observation) else {
                #[cfg(debug_assertions)]
                if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
                    eprintln!(
                        "ORIGINAL_DECLARATION offset={} stage=tokens-unavailable",
                        site.offset
                    );
                }
                continue;
            };
            let Some(body) = declared_body(&site, &tokens, &observation, registry) else {
                #[cfg(debug_assertions)]
                if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
                    eprintln!(
                        "ORIGINAL_DECLARATION offset={} stage=body-unavailable",
                        site.offset
                    );
                }
                continue;
            };
            // An independently retained installed declaration has its own
            // collector and ownership; never join it with this lexical recipe.
            if self
                .conditional_body_entry_at(&body.source.origin, body.source.base())
                .is_some()
            {
                continue;
            }
            self.retain_declared_body_layouts(body, &observation, registry);
        }
        #[cfg(any(test, debug_assertions))]
        trace.phase("uninstalled-complete", 0, 0, self.declaration_layouts.len());
        self.retain_nested_declared_layouts(registry);
        #[cfg(any(test, debug_assertions))]
        trace.phase("nested-complete", 0, 0, self.declaration_layouts.len());
    }

    fn retain_nested_declared_layouts(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) -> usize {
        let mut visited = HashSet::new();
        self.retain_nested_declared_layouts_with(registry, |source, parents| {
            visited.insert(NestedDeclarationLayoutKey::new(source, parents))
        })
    }

    fn retain_nested_declared_layouts_with(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
        visit: impl FnMut(&ExecutedScriptSource, &[DeclarationLayoutObservation]) -> bool,
    ) -> usize {
        let mut syntax = NestedLayoutSyntax::default();
        let visits =
            self.retain_nested_declared_layouts_using(registry, visit, |source, config| {
                syntax.commands(source, config)
            });
        #[cfg(any(test, debug_assertions))]
        if std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some() {
            let commands: usize = syntax
                .commands
                .values()
                .filter_map(Option::as_ref)
                .map(|commands| commands.len())
                .sum();
            eprintln!(
                "SOURCE_LAYOUT_SYNTAX_SUMMARY visits={visits} inputs={} parsed_commands={commands}",
                syntax.commands.len()
            );
        }
        visits
    }

    fn retain_nested_declared_layouts_using(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
        mut visit: impl FnMut(&ExecutedScriptSource, &[DeclarationLayoutObservation]) -> bool,
        mut parse: impl FnMut(
            &ExecutedScriptSource,
            tcl_lexer::LexerConfig,
        ) -> Option<Arc<[NestedLayoutCommand]>>,
    ) -> usize {
        #[cfg(any(test, debug_assertions))]
        let trace = NestedLayoutTrace::new();
        let roots: Vec<_> = self
            .declaration_layouts
            .iter()
            .filter_map(|(site, observations)| {
                let originals = original_declaration_layouts(observations)?;
                Some((site.clone(), originals.cloned().collect::<Vec<_>>()))
            })
            .collect();
        #[cfg(any(test, debug_assertions))]
        trace.phase(
            "nested-roots",
            0,
            roots.len(),
            self.declaration_layouts.len(),
        );
        let mut pending = Vec::new();
        for (site, observations) in roots {
            let Some(tokens) = declaration_tokens(&site, &observations[0]) else {
                continue;
            };
            self.retain_original_argument_layouts(
                &site,
                &tokens,
                &observations,
                registry,
                &mut pending,
            );
            pending.extend(unanimous_nested_layouts(
                &site,
                &tokens,
                &observations,
                registry,
            ));
        }
        #[cfg(any(test, debug_assertions))]
        trace.phase(
            "nested-seeds",
            0,
            pending.len(),
            self.declaration_layouts.len(),
        );
        let mut visits: usize = 0;
        while let Some((source, parents)) = pending.pop() {
            if !visit(&source, &parents) {
                continue;
            }
            visits += 1;
            #[cfg(any(test, debug_assertions))]
            if visits.is_multiple_of(64) {
                trace.phase(
                    "nested-visits",
                    visits,
                    pending.len(),
                    self.declaration_layouts.len(),
                );
            }
            let Some(commands) = parse(&source, parents[0].config) else {
                continue;
            };
            for command in commands.iter() {
                if !self.retain_nested_layout_command(command, &parents, registry, &mut pending) {
                    break;
                }
            }
        }
        #[cfg(any(test, debug_assertions))]
        trace.phase(
            "nested-drained",
            visits,
            pending.len(),
            self.declaration_layouts.len(),
        );
        visits
    }

    fn retain_nested_layout_command(
        &mut self,
        command: &NestedLayoutCommand,
        parents: &[DeclarationLayoutObservation],
        registry: &tcl_registry::CommandRegistry,
        pending: &mut Vec<(ExecutedScriptSource, Vec<DeclarationLayoutObservation>)>,
    ) -> bool {
        let tokens = command.tokens.as_ref();
        let site = &command.site;
        let observations: Vec<_> = parents
            .iter()
            .map(|parent| DeclarationLayoutObservation {
                words: Arc::from(tokens.words()),
                ..parent.clone()
            })
            .collect();
        let retained = self.declaration_layouts.entry(site.clone()).or_default();
        for observation in &observations {
            if !retained.contains(observation) {
                retained.push(observation.clone());
            }
        }
        self.retain_original_argument_layouts(site, tokens, &observations, registry, pending);
        let originals = parent_declaration_layouts(site, tokens, &observations, registry);
        pending.extend(unanimous_nested_layouts_from_parents(
            site, &originals, registry,
        ));
        originals.iter().all(|row| {
            row.original
                .as_ref()
                .is_some_and(OriginalDeclarationLayout::retains_successor)
        })
    }

    /// Original argv children in the same unentered Logical declaration world.
    /// Literal prefixes do not change lookup; an unknown getter, substitution
    /// result or callback ends this projection before any later child. Native
    /// entries retain their independent before-argv and actual dispatch owners.
    fn retain_original_argument_layouts(
        &mut self,
        site: &CommandAllocationSite,
        tokens: &crate::ir::CommandTokens,
        parents: &[DeclarationLayoutObservation],
        registry: &tcl_registry::CommandRegistry,
        pending: &mut Vec<(ExecutedScriptSource, Vec<DeclarationLayoutObservation>)>,
    ) {
        use crate::ir::{Provenance, WordExpr};
        let Some(input) = parents
            .first()
            .and_then(|parent| parent.snapshot.state.logical_source_name_advice_input())
            .cloned()
        else {
            return;
        };
        let config = input.lexer_config();
        if input
            .borrowed_context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            != registry.snapshot().semantic_key()
            || self.lexer_config != Some(config)
            || self.root_origin.as_ref() != Some(&site.source)
            || parents.is_empty()
            || parents.iter().any(|parent| {
                parent.issuer != DeclarationLayoutIssuer::OriginalDeclaration
                    || parent.config != config
                    || parent.words.as_ref() != tokens.words()
                    || !parent.entry.owns_source(&site.source, site.offset)
                    || !parent
                        .entry
                        .owns_original_context(&parent.snapshot.state.source_variables)
                    || parent.snapshot.state.variable_frame != *parent.entry.frame()
                    || parent.snapshot.state.logical_source_name_advice_input() != Some(&input)
                    || parent
                        .snapshot
                        .state
                        .baseline
                        .metadata_context
                        .source_analysis_input()
                        != Some(&input)
                    || parent.snapshot.state.baseline.registry_snapshot
                        != Some(registry.snapshot().semantic_key())
                    || parent.snapshot.state.baseline.native_entry.is_some()
                    || parent
                        .snapshot
                        .state
                        .baseline
                        .execution_name_policy
                        .is_some()
                    || parent
                        .snapshot
                        .state
                        .baseline
                        .hosted_execution_context
                        .is_some()
            })
            || site
                .source
                .original_lexical_command(tokens.words(), site.offset, config)
                .is_none()
        {
            return;
        }
        let Some(calls) = crate::word_subst::checked_argument_lifted_calls(tokens, config) else {
            return;
        };
        for word in tokens.words() {
            let WordExpr::CommandSubstitution { source, .. } = word else {
                if matches!(
                    word,
                    WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
                ) {
                    continue;
                }
                return;
            };
            if source.provenance != Provenance::Source {
                return;
            }
            // The complete canonical collector supplies the child vector. A
            // nested or ambiguous command prefix needs its own original world.
            let children = calls
                .iter()
                .filter(|call| {
                    source.span.start() <= call.span.start()
                        && call.span.start() <= source.span.end()
                })
                .collect::<Vec<_>>();
            let [child] = children.as_slice() else {
                return;
            };
            let Some(child) = child.tokens.as_ref() else {
                return;
            };
            let Some(head) = child.words().first() else {
                return;
            };
            let offset = head.source().span.start();
            if child.synthetic.is_some()
                || head.source().provenance != Provenance::Source
                || parents.iter().any(|parent| {
                    child.words().iter().any(|word| {
                        !parent
                            .entry
                            .owns_source(&site.source, word.source().span.start())
                            || !parent
                                .entry
                                .owns_source(&site.source, word.source().span.end())
                    })
                })
                || site
                    .source
                    .original_lexical_command(child.words(), offset, config)
                    .is_none()
            {
                return;
            }
            let command = NestedLayoutCommand {
                site: CommandAllocationSite {
                    source: Arc::clone(&site.source),
                    offset,
                },
                tokens: Box::new(child.clone()),
            };
            if !self.retain_nested_layout_command(&command, parents, registry, pending) {
                return;
            }
        }
    }

    fn retain_declared_body_layouts(
        &mut self,
        body: DeclaredProcedureBody,
        declaration: &DeclarationLayoutObservation,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let Some(segments) = nested_layout_segments(&body.source, declaration.config) else {
            return;
        };
        let mut state = declaration.snapshot.state.clone();
        state.variable_frame = body.frame.clone();
        state.source_variables = Arc::new(
            state
                .source_variables
                .in_frame(&body.frame)
                .with_namespace_identity(body.namespace.clone()),
        );
        if let Some(topology) = &body.original_parameters {
            topology.seed_unknown(Arc::make_mut(&mut state.source_variables), registry);
        } else {
            for parameter in &body.parameters {
                Arc::make_mut(&mut state.source_variables)
                    .bind_unknown_incoming(&parameter.name, registry);
            }
        }
        state.current_source_origin = Some(Arc::clone(&body.source.origin));
        let namespace = body.namespace.clone();
        let snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            state,
            declaration.snapshot.realm,
        ));
        let entry = Arc::new(OriginalDiagnosticFrameEntry::DeclaredProcedure(Arc::new(
            body,
        )));
        for segment in segments {
            if segment.is_partial {
                break;
            }
            let tokens = super::source_command_tokens_boxed(
                &entry.source().text,
                entry.source().base(),
                declaration.config,
                &segment,
            );
            let site = CommandAllocationSite {
                source: Arc::clone(&entry.source().origin),
                offset: segment.span.start(),
            };
            let observations = self.declaration_layouts.entry(site.clone()).or_default();
            let observation = DeclarationLayoutObservation {
                issuer: DeclarationLayoutIssuer::OriginalDeclaration,
                entry: Arc::clone(&entry),
                snapshot: Arc::clone(&snapshot),
                namespace: namespace.clone(),
                config: declaration.config,
                words: Arc::from(tokens.words()),
            };
            if !observations.contains(&observation) {
                observations.push(observation);
            }
            if !retains_successor_layout(
                &site,
                &tokens,
                &snapshot,
                &namespace,
                declaration.config,
                registry,
            ) {
                break;
            }
        }
    }
}

fn unanimous_nested_layouts(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    parents: &[DeclarationLayoutObservation],
    registry: &tcl_registry::CommandRegistry,
) -> Vec<(ExecutedScriptSource, Vec<DeclarationLayoutObservation>)> {
    let originals = parent_declaration_layouts(site, tokens, parents, registry);
    unanimous_nested_layouts_from_parents(site, &originals, registry)
}

fn unanimous_nested_layouts_from_parents(
    site: &CommandAllocationSite,
    originals: &[ParentDeclarationLayout<'_>],
    registry: &tcl_registry::CommandRegistry,
) -> Vec<(ExecutedScriptSource, Vec<DeclarationLayoutObservation>)> {
    let mut agreed: Option<Vec<ExecutedScriptSource>> = None;
    for row in originals {
        let sources = row.original.as_ref().map_or_else(Vec::new, |original| {
            nested_conditional_layouts(site, row.parent, original, registry)
        });
        if agreed.as_ref().is_some_and(|previous| previous != &sources) {
            return Vec::new();
        }
        agreed = Some(sources);
    }
    let parents: Vec<_> = originals.iter().map(|row| row.parent.clone()).collect();
    agreed
        .unwrap_or_default()
        .into_iter()
        .map(|source| (source, parents.clone()))
        .collect()
}

fn nested_conditional_layouts(
    site: &CommandAllocationSite,
    parent: &DeclarationLayoutObservation,
    original: &OriginalDeclarationLayout,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<ExecutedScriptSource> {
    let advice = &original.advice;
    let selected = &original.flow;
    let tcl_registry::script_body_flow::ScriptBodyFlow::Conditional(branches) = &selected.flow
    else {
        return Vec::new();
    };
    // Substitutions in argv precede every condition and may change its lookup.
    if selected.effective.words.iter().any(|word| {
        effective_invocation_word(word, parent.config.escapes, advice.dialect().word_values)
            .literal_bytes()
            .is_none()
    }) {
        return Vec::new();
    }
    let mut children = Vec::new();
    for &(condition, body) in branches {
        if condition.is_some_and(|argument| {
            !condition_retains_original_lookup(
                site,
                argument,
                &selected.effective,
                parent,
                advice,
                registry,
            )
        }) {
            break;
        }
        if let Some(source) = original_nested_body(site, body, &selected.effective, parent, advice)
        {
            children.push(source);
        }
    }
    children
}

fn original_nested_body(
    site: &CommandAllocationSite,
    argument: usize,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    parent: &DeclarationLayoutObservation,
    advice: &super::OriginalCompilationLookupAdvice,
) -> Option<ExecutedScriptSource> {
    let written = effective.written_argument(argument)?;
    let word = effective.words.get(argument + 1)?;
    let EffectiveInvocationWord::Literal(value) =
        effective_invocation_word(word, parent.config.escapes, advice.dialect().word_values)
    else {
        return None;
    };
    let source =
        ExecutedScriptSource::from_word(site.clone(), written, word, &value, parent.config);
    let super::ExecutedScriptMapping::Contiguous { base } = source.mapping else {
        return None;
    };
    let end = base.checked_add(u32::try_from(source.text.len()).ok()?)?;
    (source.origin == site.source
        && base > site.offset
        && parent.entry.owns_source(&source.origin, base)
        && end
            .checked_sub(1)
            .is_some_and(|last| parent.entry.owns_source(&source.origin, last)))
    .then_some(source)
}

fn condition_retains_original_lookup(
    site: &CommandAllocationSite,
    argument: usize,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    parent: &DeclarationLayoutObservation,
    advice: &super::OriginalCompilationLookupAdvice,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    use tcl_syntax::expr::ast::ExprNode;
    let Some(source) = original_nested_body(site, argument, effective, parent, advice) else {
        return false;
    };
    let Ok(text) = source.text.try_text() else {
        return false;
    };
    let tcl_syntax::expr::parser::CheckedExprParse::Parsed(expression) =
        tcl_syntax::expr::parser::parse_expr_checked_with_context(
            text,
            &advice.dialect().expression_parse_context(None),
        )
    else {
        return false;
    };
    let context = advice.original_variable_context();
    if context.dynamic_traces {
        return false;
    }
    let mut pending = vec![&expression];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { .. } => {}
            ExprNode::Var { text, name, .. } => {
                let reference =
                    tcl_lexer::word_parts::whole_var_ref(text.as_bytes(), parent.config);
                if !matches!(reference, Ok(Some(reference)) if reference.index.is_none())
                    || tcl_syntax::naming::is_qualified(name.as_bytes())
                    || super::declaration_layout::local_read_scope_is_excluded(context, name)
                    || parent
                        .source_body_name_ownership(
                            registry,
                            crate::script_binds::Ownership::ScopeAliases,
                        )
                        .is_none_or(|effects| {
                            effects.opaque
                                || effects.names.iter().any(|candidate| candidate == name)
                        })
                {
                    return false;
                }
            }
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                pending.extend([
                    condition.as_ref(),
                    true_branch.as_ref(),
                    false_branch.as_ref(),
                ]);
            }
            _ => return false,
        }
    }
    true
}

pub(super) fn declaration_tokens(
    site: &CommandAllocationSite,
    observation: &DeclarationLayoutObservation,
) -> Option<crate::ir::CommandTokens> {
    let text = site.source.try_text().ok()?;
    let first = tcl_lexer::word_span_at(text, observation.words.first()?.source().span);
    let last = tcl_lexer::word_span_at(text, observation.words.last()?.source().span);
    if first.start() != site.offset || first.start() > last.end() {
        return None;
    }
    let image = tcl_lexer::SourceImage::from_bytes(
        text.as_bytes()
            .get(first.start() as usize..last.end() as usize)?,
        site.source.source_image().channel(),
    );
    let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
        &image,
        first.start(),
        observation.config.nested(),
    )?;
    let [segment] = segments.as_slice() else {
        return None;
    };
    let tokens =
        super::source_command_tokens_boxed(&image, first.start(), observation.config, segment);
    (tokens.words() == observation.words.as_ref()).then_some(*tokens)
}

fn declared_body(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    registry: &tcl_registry::CommandRegistry,
) -> Option<DeclaredProcedureBody> {
    let advice = super::original_site_operand_layout_advice(
        site,
        tokens,
        &observation.snapshot,
        &observation.namespace,
        observation.config,
    );
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
        eprintln!(
            "ORIGINAL_DECLARATION offset={} stage=body-advice available={}",
            site.offset,
            advice.is_some()
        );
    }
    let advice = advice?;
    if let Some(input) = observation
        .snapshot
        .state
        .logical_source_name_advice_input()
    {
        return logical_procedure_body_recipe(site, tokens, observation, registry, input, &advice);
    }
    let dialect = advice.dialect();
    let mut agreed = None;
    for target in advice.targets() {
        let effective = effective_words_for_target(tokens, target)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm(registry, None, &words, Some(dialect), advice.realm())
                .ok()?
        else {
            return None;
        };
        let recipe = procedure_body_recipe(
            site,
            tokens,
            observation,
            &facts,
            target,
            &ProcedureOperands {
                effective: &effective,
                values: &values,
                dialect,
            },
        );
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
            eprintln!(
                "ORIGINAL_DECLARATION offset={} stage=procedure-recipe available={} arity={:?} roles={}",
                site.offset,
                recipe.is_some(),
                facts.arity_accepts_frozen_arguments(),
                facts.arg_roles_complete
            );
        }
        let recipe = recipe?;
        if agreed.as_ref().is_some_and(|old| old != &recipe) {
            return None;
        }
        agreed = Some(recipe);
    }
    agreed
}

fn logical_procedure_body_recipe(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    registry: &tcl_registry::CommandRegistry,
    input: &crate::analyser::ResolvedAnalysisInput,
    advice: &super::OriginalCompilationLookupAdvice,
) -> Option<DeclaredProcedureBody> {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    if !advice.closed_logical_source_lookup()
        || input.lexer_config() != observation.config
        || input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            != registry.snapshot().semantic_key()
        || !observation.entry.owns_source(&site.source, site.offset)
        || !observation
            .entry
            .owns_original_context(&observation.snapshot.state.source_variables)
        || observation.snapshot.state.variable_frame != *observation.entry.frame()
        || observation.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
    {
        return None;
    }
    let dialect = advice.dialect();
    let actual_context = input.context_registry();
    let mut agreed = None;
    for target in advice.targets() {
        let effective = effective_words_for_target(tokens, target)?;
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
        let (&head, arguments) = words.split_first()?;
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                registry,
                Some(actual_context.context()),
                tcl_registry::InvocationWords::structured(head, arguments).with_dialect(dialect),
                advice.realm(),
            );
        let shape = resolution
            .resolved()?
            .authored_source_procedure_arguments()?;
        let name = words.get(shape.name + 1)?.literal()?;
        let key = super::logical_definition::source_procedure_key(
            &observation.snapshot.state,
            &observation.namespace,
            name,
        )?;
        let parameters = super::logical_definition::static_formals(
            words.get(shape.parameters + 1)?.literal()?,
            observation.config.list_parse,
        )?;
        let crate::registry_invocation::InvocationWordOrigin::Written(written) =
            effective.origins.get(shape.body + 1)?
        else {
            return None;
        };
        let source = ExecutedScriptSource::from_word(
            site.clone(),
            written.checked_sub(1)?,
            tokens.words().get(*written)?,
            words.get(shape.body + 1)?.literal()?,
            observation.config,
        );
        if source.origin != site.source {
            return None;
        }
        let namespace = key.holder().into_owned();
        let body = DeclaredProcedureBody {
            declaration: site.clone(),
            candidate: target.clone(),
            slot: key,
            frame: declared_procedure_frame(site, &namespace),
            source,
            namespace,
            parameters,
            logical_name: Some(name.to_owned()),
            logical_parameters_text: Some(words.get(shape.parameters + 1)?.literal()?.to_owned()),
            original_parameters: None,
        };
        if agreed.as_ref().is_some_and(|previous| previous != &body) {
            return None;
        }
        agreed = Some(body);
    }
    agreed
}

struct ProcedureOperands<'a> {
    effective: &'a crate::registry_invocation::EffectiveCommandWords,
    values: &'a [EffectiveInvocationWord],
    dialect: tcl_registry::InvocationDialect,
}

/// A readonly source operand follows its actual effective ordinal. It does
/// not acquire installation, entered-frame or Normal authority here.
fn original_declaration_operand_input(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    candidate: &super::SourceCommandTarget,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    index: usize,
) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
    use crate::registry_invocation::InvocationWordOrigin;
    use crate::signature_scan::scope::{
        SignatureSourceNameInput, SignatureSourceNameKey, SignatureSourceNameValue,
    };
    let variables = &observation.snapshot.state.source_variables;
    let policy = variables.execution_name_policy?.native_recipe()?;
    let origin = effective.origins.get(index)?;
    if let InvocationWordOrigin::BindingPrefix(prefix) = origin {
        let input = candidate.original_prepended_name_inputs()?.get(*prefix)?;
        return (input.policy() == policy && input.is_current(variables)).then(|| input.clone());
    }
    let native = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        observation.config,
    )?;
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&observation.config);
    match origin {
        InvocationWordOrigin::Written(written) => {
            SignatureSourceNameKey::from_original_native_word(native.get(*written)?, rules, policy)
                .map(SignatureSourceNameInput::OriginalWord)
        }
        InvocationWordOrigin::ExpandedElement { written, element } => {
            let parent = SignatureSourceNameInput::OriginalValue(
                SignatureSourceNameValue::from_original_static_word(
                    native.get(*written)?,
                    rules,
                    policy,
                )?,
            );
            let children = parent.original_list_elements()?;
            let projected = effective.origins.iter().filter_map(|origin| match origin {
                InvocationWordOrigin::ExpandedElement {
                    written: parent,
                    element,
                } if parent == written => Some(*element),
                _ => None,
            });
            projected.eq(0..children.len()).then_some(())?;
            children.get(*element).cloned()
        }
        InvocationWordOrigin::ResolvedHead | InvocationWordOrigin::BindingPrefix(_) => None,
    }
}

fn procedure_declaration_header(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    facts: &tcl_registry::InvocationFacts,
    candidate: &super::SourceCommandTarget,
    operands: &ProcedureOperands<'_>,
) -> Option<DeclaredProcedureHeader> {
    use tcl_registry::{CommandBindingDefinitionKind, CommandBindingTransition};
    let ProcedureOperands {
        effective,
        values,
        dialect,
    } = *operands;
    if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
        return None;
    }
    let name_argument = facts
        .state_transitions
        .declared()?
        .command_bindings()
        .find_map(|transition| {
            if let CommandBindingTransition::Define {
                name,
                kind: CommandBindingDefinitionKind::Procedure,
            } = transition
            {
                name.argument_index()
            } else {
                None
            }
        })?;
    let name = original_declaration_operand_input(
        site,
        tokens,
        observation,
        candidate,
        effective,
        name_argument.checked_add(1)?,
    )?;
    if !observation.entry.owns_source(&site.source, site.offset)
        || !observation
            .entry
            .owns_original_context(&observation.snapshot.state.source_variables)
        || observation.snapshot.state.variable_frame != *observation.entry.frame()
        || observation.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
    {
        return None;
    }
    let key = observation
        .snapshot
        .state
        .original_conditional_procedure_key(&observation.namespace, &name)?;
    let namespace = key.holder().into_owned();
    let words: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let arguments =
        tcl_registry::InvocationArguments::structured(words.get(1..)?).with_dialect(dialect);
    let original_parameters =
        original_procedure_parameter_topology(site, tokens, observation, facts, effective, dialect);
    let parameters = original_parameters
        .as_ref()
        .map(super::formal_topology::OriginalFormalTopology::advisory_parameters)
        .or_else(|| super::native_procedure_parameters(facts, arguments))?;
    let body_index = facts.arg_roles.iter().find_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::Body)
            .then_some(facts.argument_offset + usize::from(*index) + 1)
    })?;
    let crate::registry_invocation::InvocationWordOrigin::Written(written) =
        effective.origins.get(body_index)?
    else {
        return None;
    };
    let EffectiveInvocationWord::Literal(value) = values.get(body_index)? else {
        return None;
    };
    written.checked_sub(1)?;
    tokens.words().get(*written)?;
    let frame = declared_procedure_frame(site, &namespace);
    Some(DeclaredProcedureHeader {
        declaration: site.clone(),
        candidate: candidate.clone(),
        slot: key,
        namespace,
        parameters,
        original_parameters,
        frame,
        written: *written,
        value: value.to_owned(),
    })
}

fn procedure_body_recipe(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    facts: &tcl_registry::InvocationFacts,
    candidate: &super::SourceCommandTarget,
    operands: &ProcedureOperands<'_>,
) -> Option<DeclaredProcedureBody> {
    let header =
        procedure_declaration_header(site, tokens, observation, facts, candidate, operands)?;
    let source = ExecutedScriptSource::from_word(
        site.clone(),
        header.written.checked_sub(1)?,
        tokens.words().get(header.written)?,
        &header.value,
        observation.config,
    );
    if source.origin != site.source {
        return None;
    }
    Some(DeclaredProcedureBody {
        declaration: header.declaration,
        candidate: header.candidate,
        slot: header.slot,
        source,
        namespace: header.namespace,
        parameters: header.parameters,
        logical_name: None,
        logical_parameters_text: None,
        original_parameters: header.original_parameters,
        frame: header.frame,
    })
}

struct OriginalProcedureReadFlowRequest<'a> {
    registry: &'a tcl_registry::CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'a>,
    read: &'a OriginalProcedureReadDeclaration,
}

fn procedure_read_flow_for_target(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    row: &DeclarationLayoutObservation,
    target: &super::SourceCommandTarget,
    advice: &super::OriginalCompilationLookupAdvice,
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
        snapshot: Arc::new(super::SourceLookupSnapshot::in_realm(
            state,
            row.snapshot.realm,
        )),
    })
}

fn native_procedure_read_recipe(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    row: &DeclarationLayoutObservation,
    target: &super::SourceCommandTarget,
    advice: &super::OriginalCompilationLookupAdvice,
    registry: &tcl_registry::CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
) -> Option<OriginalProcedureReadDeclaration> {
    let dialect = advice.dialect();
    let effective = effective_words_for_target(tokens, target)?;
    let values: Vec<_> = effective
        .words
        .iter()
        .map(|word| {
            effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
        })
        .collect();
    let words: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let RegistryInvocationResolution::Resolved(facts) =
        crate::registry_invocation::resolve_registry_words_in_realm_with_metadata_context(
            registry,
            Some(metadata),
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
    let crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(parameters) =
        original_declaration_operand_input(
            site,
            tokens,
            row,
            target,
            &effective,
            shape.parameters_at.checked_add(1)?,
        )?
    else {
        return None;
    };
    let topology =
        super::formal_topology::OriginalFormalTopology::from_original_key(parameters, dialect)?;
    let body_at = shape.body_at.checked_add(1)?;
    let crate::registry_invocation::InvocationWordOrigin::Written(written) =
        effective.origins.get(body_at)?
    else {
        return None;
    };
    let source = ExecutedScriptSource::from_word(
        site.clone(),
        written.checked_sub(1)?,
        tokens.words().get(*written)?,
        words.get(body_at)?.literal()?,
        row.config,
    );
    let source = if source.origin == site.source {
        ProcedureReadSource::Contiguous(source)
    } else {
        let crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(producer) =
            original_declaration_operand_input(site, tokens, row, target, &effective, body_at)?
        else {
            return None;
        };
        let original = producer.original_word();
        if original.group().kind != tcl_lexer::WordKind::Braced
            || original.image() != site.source.source_image()
        {
            return None;
        }
        let correspondence = tcl_syntax::backslash::native_source_braced_word_map(
            original
                .image()
                .bytes()
                .get(original.content_span().ok()?.as_range())?,
            original.image().channel(),
            original.config().brace_backslash_newline,
            producer.policy().string_protocol(),
        )
        .ok()?;
        let evaluated = tcl_syntax::backslash::native_source_literal_bytes(
            words.get(body_at)?.literal()?.as_bytes(),
            original.image().channel(),
            producer.policy().string_protocol(),
        )
        .ok()?;
        if correspondence.bytes() != producer.bytes()
            || correspondence.bytes() != evaluated.as_ref()
        {
            return None;
        }
        ProcedureReadSource::OriginalBraced {
            producer,
            correspondence,
        }
    };
    Some(OriginalProcedureReadDeclaration {
        source,
        formals: ProcedureReadFormals::Native(topology),
    })
}

fn declared_procedure_frame(
    site: &CommandAllocationSite,
    namespace: &super::SourceNamespaceKey,
) -> crate::var_resolve::VariableExecutionFrame {
    crate::var_resolve::VariableExecutionFrame::Procedure {
        namespace: namespace.display().unwrap_or_default(),
        identity: super::source_activation_name(
            Some(&site.source),
            "declared-procedure-layout",
            site.offset,
        ),
    }
    .with_namespace_identity(namespace.clone())
}

fn original_procedure_parameter_topology(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    facts: &tcl_registry::InvocationFacts,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    dialect: tcl_registry::InvocationDialect,
) -> Option<super::formal_topology::OriginalFormalTopology> {
    facts
        .arg_roles
        .iter()
        .find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::ParamList)
                .then_some(facts.argument_offset + usize::from(*index))
        })
        .and_then(|argument| effective.written_argument(argument))
        .and_then(|written| {
            super::formal_topology::capture(
                site.source.source_image(),
                tokens.words(),
                site.offset,
                written + 1,
                observation.config,
                dialect,
                observation
                    .snapshot
                    .state
                    .source_variables
                    .execution_name_policy?
                    .native_recipe()?,
            )
        })
}

pub(super) fn retains_successor_layout(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &super::SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    OriginalDeclarationLayout::select(site, tokens, snapshot, namespace, config, registry)
        .as_ref()
        .is_some_and(OriginalDeclarationLayout::retains_successor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_logical_declaration_tokens(
        source: &str,
        command: &str,
        context: &Arc<tcl_registry::model::ContextRegistry>,
    ) -> (
        SourceCommandBindings,
        crate::ir::CommandTokens,
        crate::analyser::ResolvedAnalysisInput,
    ) {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(context),
            config,
        );
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            context.commands(),
            super::super::SourceAnalysisOptions::for_logical_source(&input).unwrap(),
        );
        let offset = u32::try_from(source.find(command).unwrap()).unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let part = tcl_lexer::SourceImage::document(command);
        let segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(&part, offset, config)
                .unwrap();
        let [segment] = segments.as_slice() else {
            panic!("one original source vector");
        };
        let mut tokens =
            crate::ir::CommandTokens::from_segmented(&image.source_map(), config, segment);
        bindings.stamp_original_tokens(&mut tokens);
        (bindings, tokens, input)
    }

    #[test]
    fn original_declaration_metadata_keeps_unentered_logical_successor_source() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Original declaration metadata supplies conditional source structure;
        // no reached lookup, Native frame, handler or compiled store is donated.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let source = "proc f {} {set p literal.txt; open $p}";
        for command in ["set p literal.txt", "open $p"] {
            let (_, tokens, input) = complete_logical_declaration_tokens(source, command, &context);
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(binding.lookup_state.is_none(), "unentered procedure source");
            assert!(binding.logical_source_name_advice_input().is_none());
            assert!(binding.proved_handler_target().is_none());
            assert!(binding.native_structured_preparation.is_none());
            assert!(
                binding
                    .original_invocation_metadata_at_point(&tokens, context.commands())
                    .is_err()
            );
            let metadata = binding
                .original_invocation_metadata_for_input(&tokens, &input, context.commands())
                .expect("same original Logical declaration source owner");
            assert_eq!(metadata.source_analysis_input(), Some(&input));
            assert!(crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                context.commands(), metadata, &tokens,
            ).is_some(), "{command}");
            let advice = binding
                .declaration_operand_layout_advice(&tokens)
                .expect("the exact original successor layout");
            assert!(advice.closed_logical_source_lookup());
            let mut raw = tokens.clone();
            raw.source_binding = None;
            let raw_metadata = advice
                .original_declaration_metadata_context(context.commands(), &raw)
                .expect("raw preview vector retains its sealed advice")
                .unwrap();
            assert_eq!(raw_metadata.source_analysis_input(), Some(&input));
            let flow = crate::registry_invocation::declaration_invocation_flow(
                context.commands(),
                &raw,
                &advice,
            )
            .expect("source flow needs no entered binding");
            if command.starts_with("set ") {
                assert_eq!(
                    flow.literal_stores,
                    vec![("p".to_owned(), "literal.txt".to_owned())]
                );
            }
            assert!(raw.source_binding.is_none());
        }
    }

    #[test]
    fn original_declaration_metadata_refuses_missing_foreign_and_changed_source_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Same command store does not identify the availability owner; exact
        // source vectors and every original declaration remain prerequisites.
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let mut selected = registry.get("set").unwrap().clone();
        selected.name = "metadata_store";
        selected.surface = registry.get("dict").unwrap().surface;
        registry.insert(selected);
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.6")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let source = "proc f {} {metadata_store p VALUE; open $p}";
        let (_, tokens, input) =
            complete_logical_declaration_tokens(source, "metadata_store p VALUE", &context);
        let binding = tokens.source_binding.as_ref().unwrap();
        let metadata = binding
            .original_invocation_metadata_for_input(&tokens, &input, &registry)
            .unwrap();
        assert!(crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            &registry, metadata, &tokens,
        ).is_some());
        let (_, older_tokens, older_input) =
            complete_logical_declaration_tokens(source, "metadata_store p VALUE", &older);
        let older_binding = older_tokens.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_invocation_metadata_for_input(&tokens, &older_input, &registry)
                .is_none()
        );
        if let Some(older_metadata) = older_binding.original_invocation_metadata_for_input(
            &older_tokens,
            &older_input,
            &registry,
        ) {
            assert!(crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                &registry, older_metadata, &older_tokens,
            ).is_none());
        }
        let mut foreign_registry = tcl_registry::CommandRegistry::build_default();
        foreign_registry.insert(tcl_registry::CommandSpec {
            name: "foreign_declaration_axis",
            ..registry.get("set").unwrap().clone()
        });
        let foreign = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.6")
                .with_command_store(foreign_registry.snapshot().shared_registry()),
        );
        let changed_input = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::clone(&context),
            tcl_lexer::LexerConfig {
                strict_quoting: !input.lexer_config().strict_quoting,
                ..input.lexer_config()
            },
        );
        let foreign_input = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            foreign,
            input.lexer_config(),
        );
        for owner in [
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
            crate::registry_invocation::OwnedInvocationMetadataContext::Supplied(Arc::clone(
                &context,
            )),
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                &changed_input,
            )),
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                &foreign_input,
            )),
        ] {
            let mut changed = tokens.clone();
            let binding = changed.source_binding.as_mut().unwrap();
            for row in Arc::make_mut(binding.declaration_layout_observations.as_mut().unwrap()) {
                let state = &mut Arc::make_mut(&mut row.snapshot).state;
                Arc::make_mut(&mut state.baseline).metadata_context = owner.clone();
            }
            let binding = changed.source_binding.as_ref().unwrap();
            assert!(
                binding
                    .original_invocation_metadata_for_input(&changed, &input, &registry)
                    .is_none()
            );
        }
        let mut missing = tokens.clone();
        missing
            .source_binding
            .as_mut()
            .unwrap()
            .declaration_layout_observations = None;
        assert!(
            missing
                .source_binding
                .as_ref()
                .unwrap()
                .original_invocation_metadata_for_input(&missing, &input, &registry,)
                .is_none()
        );
        let mut changed_vector = tokens.clone();
        changed_vector.word_exprs.pop();
        assert!(
            binding
                .original_invocation_metadata_for_input(&changed_vector, &input, &registry)
                .is_none()
        );
        let (_, shadowed, shadow_input) = complete_logical_declaration_tokens(
            "proc metadata_store args {return CUSTOM}; proc f {} {metadata_store p VALUE}",
            "metadata_store p VALUE",
            &context,
        );
        let shadow_binding = shadowed.source_binding.as_ref().unwrap();
        if let Some(metadata) = shadow_binding.original_invocation_metadata_for_input(
            &shadowed,
            &shadow_input,
            &registry,
        ) {
            assert!(crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                &registry, metadata, &shadowed,
            ).is_none());
        }
    }

    #[test]
    fn original_declaration_metadata_cannot_repair_invalid_reached_or_native_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Retained source advice cannot replace a present invalid reached
        // owner, nor issue Native lookup or preparation for an unentered body.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let (_, tokens, input) =
            complete_logical_declaration_tokens("set p VALUE", "set p VALUE", &context);
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .lookup_state
                .is_some()
        );
        let mut withdrawn = tokens.clone();
        let binding = withdrawn.source_binding.as_mut().unwrap();
        let state = &mut Arc::make_mut(binding.lookup_state.as_mut().unwrap()).state;
        Arc::make_mut(&mut state.baseline).metadata_context =
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
        assert!(binding.declaration_layout_observations.is_some());
        let binding = withdrawn.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_source_metadata_for_advice(&withdrawn, context.commands())
                .is_err()
        );
        assert!(
            binding
                .original_invocation_metadata_for_input(&withdrawn, &input, context.commands())
                .is_none()
        );
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            context.commands(),
            &input,
        )
        .unwrap();
        assert!(crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            context.commands(), metadata, &withdrawn,
        ).is_none());
        let source = "proc f {} {set p VALUE}";
        let (_, unentered, _) =
            complete_logical_declaration_tokens(source, "set p VALUE", &context);
        let native_profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
        let native_input = crate::analyser::ResolvedAnalysisInput::new(
            native_profile,
            native_profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(native_profile.grammar),
        );
        assert!(!native_input.has_logical_source_name_context());
        let binding = unentered.source_binding.as_ref().unwrap();
        assert!(binding.lookup_state.is_none());
        assert!(
            binding
                .original_invocation_metadata_for_input(
                    &unentered,
                    &native_input,
                    context.commands()
                )
                .is_none()
        );
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.native_structured_preparation.is_none());
    }

    fn inventory(source: &str, command: &str) -> (SourceCommandBindings, crate::ir::CommandTokens) {
        inventory_in_world(source, command, false)
    }

    fn inventory_in_world(
        source: &str,
        command: &str,
        unknown_entry: bool,
    ) -> (SourceCommandBindings, crate::ir::CommandTokens) {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl8.6")).unwrap(),
        );
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                unknown_entry,
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find(command).unwrap()).unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(command, offset, config)
                .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        (bindings, tokens)
    }

    #[test]
    fn original_registry_advice_uses_uncalled_body_layout_without_normal_receipt() {
        // Implementation contract: naming.source.original-registry-header-advice
        // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, command, expected) in [
            (
                "proc p {arg} {set evenWhenWritingCode}",
                "set evenWhenWritingCode",
                "set",
            ),
            ("proc p {} \"puts hi\"", "puts hi", "puts"),
        ] {
            let (_, tokens) = inventory(source, command);
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(
                binding.original_normal_result(&tokens).is_none(),
                "{source}"
            );
            assert!(
                !binding.original_invocation_completes_normally(&tokens),
                "{source}"
            );
            let transfer_available =
                crate::registry_invocation::normal_transfer_invocation(registry, None, &tokens)
                    .is_some();
            let advice = crate::registry_invocation::original_registry_invocation_assistance(
                registry, None, &tokens,
            )
            .expect("the original body layout supplies conditional metadata");
            assert_eq!(
                advice.unanimous_command_words().unwrap().command(),
                expected
            );
            assert_eq!(
                crate::registry_invocation::normal_transfer_invocation(registry, None, &tokens,)
                    .is_some(),
                transfer_available,
                "{source}"
            );
            assert!(
                binding.original_normal_result(&tokens).is_none(),
                "{source}"
            );
            assert!(
                !binding.original_invocation_completes_normally(&tokens),
                "{source}"
            );
            let mut synthetic = tokens.clone();
            synthetic.source_binding = None;
            assert!(
                crate::registry_invocation::original_registry_invocation_assistance(
                    registry, None, &synthetic,
                )
                .is_none()
            );
        }
    }

    #[test]
    fn original_registry_advice_retains_body_shadowing_and_original_word_currency() {
        // Implementation contract: naming.source.original-registry-header-advice
        // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (_, tokens) = inventory(
            "proc set args {return CUSTOM}; proc p {} {set value}",
            "set value",
        );
        assert!(
            crate::registry_invocation::original_registry_invocation_assistance(
                registry, None, &tokens,
            )
            .is_none_or(|advice| advice.unanimous_command_words().is_none())
        );
        let (_, mut changed) = inventory("proc p {} {puts hi}", "puts hi");
        changed.argv_texts[0] = "reporting-counterfactual".to_owned();
        let readonly = crate::registry_invocation::original_registry_invocation_assistance(
            registry, None, &changed,
        )
        .unwrap();
        assert_eq!(
            readonly.unanimous_command_words().unwrap().command(),
            "puts"
        );
        let crate::ir::WordExpr::Literal { text, .. } = &mut changed.word_exprs[0] else {
            panic!("a bare literal original head");
        };
        *text = "set".to_owned();
        assert!(
            crate::registry_invocation::original_registry_invocation_assistance(
                registry, None, &changed,
            )
            .is_none_or(|advice| advice.unanimous_command_words().is_none())
        );
    }

    #[test]
    fn original_declaration_procedure_name_keeps_native_operand_identity() {
        // Implementation contract: naming.source.original-declaration-name-publication (docs/design/analysis/name-resolution-proofs/original-declaration-name-publication.md).
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let owner = tcl_registry::model::ingress::static_context_for(engine);
            let registry = owner.commands();
            let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
            let source = r"proc p\uD800 {value} {set local $value}";
            let bindings = SourceCommandBindings::analyse(source, config, registry);
            let site = CommandAllocationSite {
                source: Arc::clone(bindings.source_origin().unwrap()),
                offset: 0,
            };
            let observations =
                original_declaration_layouts(bindings.declaration_layouts.get(&site).unwrap())
                    .unwrap();
            let observation = observations.clone().next().unwrap();
            let tokens = declaration_tokens(&site, observation).unwrap();
            let recipe = declared_body(&site, &tokens, observation, registry)
                .expect("original native name is not a reporting string");
            let native = crate::registry_invocation::original_native_compiler_words(
                site.source.source_image(),
                tokens.words(),
                site.offset,
                config,
            )
            .unwrap();
            let key =
                crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                    &native[1],
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    observation
                        .snapshot
                        .state
                        .source_variables
                        .execution_name_policy
                        .unwrap()
                        .native_recipe()
                        .unwrap(),
                )
                .unwrap();
            let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key);
            assert_eq!(
                recipe.slot,
                observation
                    .snapshot
                    .state
                    .original_publication_key(
                        &observation.namespace,
                        &input,
                        super::super::namespace_slots::PublicationPurpose::Procedure
                    )
                    .unwrap()
            );
            assert_eq!(recipe.source.origin, site.source);
            assert!(recipe.original_parameters.is_some());
            let dynamic = r"proc $unknown {value} {set local $value}";
            let bindings = SourceCommandBindings::analyse(dynamic, config, registry);
            let site = CommandAllocationSite {
                source: Arc::clone(bindings.source_origin().unwrap()),
                offset: 0,
            };
            if let Some(observation) = bindings
                .declaration_layouts
                .get(&site)
                .and_then(|rows| original_declaration_layouts(rows))
                .and_then(|mut rows| rows.next())
            {
                let tokens = declaration_tokens(&site, observation).unwrap();
                assert!(declared_body(&site, &tokens, observation, registry).is_none());
            }
        }
    }

    #[test]
    fn conditional_declaration_geometry_survives_unknown_table_without_installation() {
        // Implementation contract: naming.source.original-declaration-name-publication
        // docs/design/analysis/name-resolution-proofs/original-declaration-name-publication.md
        let source = "rename $old decl\nproc wrap {arg} {upvar 1 other local; return $local}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (bindings, tokens) = inventory_in_world(
            source,
            "proc wrap {arg} {upvar 1 other local; return $local}",
            true,
        );
        let site = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap();
        let observation =
            original_declaration_layouts(bindings.declaration_layouts.get(site).unwrap())
                .unwrap()
                .next()
                .unwrap();
        assert!(observation.snapshot.state.has_opaque_domain());
        let recipe = declared_body(site, &tokens, observation, registry)
            .expect("genuine declaration geometry is independent of current table existence");
        assert_eq!(recipe.namespace.display().as_deref(), Some("::"));
        assert_eq!(
            recipe
                .original_parameters
                .as_ref()
                .unwrap()
                .advisory_parameters()[0]
                .name,
            "arg"
        );
        assert!(
            observation
                .snapshot
                .state
                .installed_procedure_targets(&recipe.slot, site.offset)
                .is_empty()
        );
        assert!(!bindings.has_actual_procedure_entry(&recipe.source));
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .proved_execution_target()
                .is_none()
        );
        let mut foreign = observation.clone();
        foreign.snapshot = Arc::new(SourceLookupSnapshot::new(
            super::super::ModuleCommandBindings::initial(registry),
        ));
        assert!(declared_body(site, &tokens, &foreign, registry).is_none());
    }

    #[test]
    fn nested_layout_visits_process_each_exact_source_world_once() {
        // Implementation contract: naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // Derive conditional source layouts from one genuine outer observation.
        // All descendants keep that unchanged lookup world; reached condition
        // coercions and their independent snapshots are not substituted here.
        let source = "if {1} {if {1} {if {1} {if {1} {if {1} {if {1} {if {1} {puts done}}}}}}}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (mut cached, outer) = inventory(source, source);
        let outer_site = outer
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap()
            .clone();
        let original =
            original_declaration_layouts(cached.declaration_layouts.get(&outer_site).unwrap())
                .unwrap()
                .next()
                .unwrap()
                .clone();
        cached
            .declaration_layouts
            .retain(|site, _| site == &outer_site);
        assert_eq!(cached.declaration_layouts.len(), 1);
        assert_eq!(cached.retain_nested_declared_layouts(registry), 7);
        assert!(cached.declaration_layouts.values().all(|rows| {
            original_declaration_layouts(rows).unwrap().all(|row| {
                row.snapshot == original.snapshot
                    && row.entry == original.entry
                    && row.namespace == original.namespace
                    && row.config == original.config
            })
        }));
        let mut uncached = cached.clone();
        let cached_visits = cached.retain_nested_declared_layouts(registry);
        let uncached_visits = uncached.retain_nested_declared_layouts_with(registry, |_, _| true);
        assert_eq!(cached_visits, 7);
        assert_eq!(uncached_visits, 28);
        assert_eq!(cached, uncached);
    }

    #[test]
    fn nested_layout_visits_keep_reached_representation_worlds_distinct() {
        // Implementation contract: naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // The same written command table does not make reached variable and
        // literal representation worlds interchangeable. This software source
        // control supplies no Native object, body-entry or elapsed-time claim.
        let source = "if {1} {if {1} {if {1} {if {1} {if {1} {if {1} {if {1} {puts done}}}}}}}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (mut cached, _) = inventory(source, "puts done");
        let worlds = cached
            .declaration_layouts
            .values()
            .flat_map(|rows| original_declaration_layouts(rows).unwrap())
            .collect::<Vec<_>>();
        let first = worlds.first().unwrap();
        assert!(worlds.iter().all(|row| {
            row.snapshot.state.original_command_world == first.snapshot.state.original_command_world
                && row.snapshot.state.source_variables.original_contents_epoch
                    == first
                        .snapshot
                        .state
                        .source_variables
                        .original_contents_epoch
        }));
        let mut distinct = Vec::new();
        for row in &worlds {
            if !distinct.contains(&&row.snapshot) {
                distinct.push(&row.snapshot);
            }
        }
        assert_eq!(distinct.len(), 8);
        for (index, snapshot) in distinct.iter().enumerate() {
            for other in &distinct[index + 1..] {
                assert_ne!(
                    snapshot.state.source_variables.representation_epoch,
                    other.state.source_variables.representation_epoch,
                );
                assert_ne!(
                    snapshot.state.source_variables,
                    other.state.source_variables
                );
                assert_ne!(
                    snapshot.state.ordinary_literal_pool,
                    other.state.ordinary_literal_pool,
                );
            }
        }
        if std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some() {
            let mut first = None;
            for (site, rows) in cached.declaration_layouts.iter() {
                let Some(originals) = original_declaration_layouts(rows) else {
                    continue;
                };
                for original in originals {
                    let state = &original.snapshot.state;
                    let first = first.get_or_insert(state);
                    eprintln!(
                        "NESTED_LAYOUT_ORIGINAL_WORLD offset={} rows={} same_state={} same_variables={} same_literal_pool={} same_command_world={} epoch={:?} epoch_high={:?} contents_epoch={:?} contents_high={:?}",
                        site.offset,
                        rows.len(),
                        *first == state,
                        first.source_variables == state.source_variables,
                        first.ordinary_literal_pool == state.ordinary_literal_pool,
                        first.original_command_world == state.original_command_world,
                        state.source_variables.representation_epoch,
                        state.source_variables.representation_epoch_high_water,
                        state.source_variables.original_contents_epoch,
                        state.source_variables.original_contents_epoch_high_water,
                    );
                }
            }
        }
        let mut uncached = cached.clone();
        let cached_visits = cached.retain_nested_declared_layouts(registry);
        let uncached_visits = uncached.retain_nested_declared_layouts_with(registry, |_, _| true);
        assert_eq!(cached_visits, 28);
        assert_eq!(uncached_visits, 28);
        assert_eq!(cached, uncached);
    }

    #[test]
    fn exact_parent_layout_reuse_keeps_guarded_children_and_successor_decisions() {
        // Implementation contract: naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // Genuine original observations compare one shared projection with
        // independently rederived children and continuation. No elapsed-time,
        // execution, installed implementation or Native frame claim follows.
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, command, child_count, successor) in [
            ("if {1} {puts done}", "if {1} {puts done}", 1, false),
            ("list done; list later", "list done", 0, true),
            (
                "set local $incoming; puts later",
                "set local $incoming",
                0,
                false,
            ),
            (
                "rename puts saved; puts later",
                "rename puts saved",
                0,
                false,
            ),
            (
                "if {[rename puts saved; expr 1]} {puts done}",
                "if {[rename puts saved; expr 1]} {puts done}",
                0,
                false,
            ),
            (
                "proc wrap {condition} {if {$condition} {puts done}}",
                "if {$condition} {puts done}",
                1,
                false,
            ),
        ] {
            let (bindings, tokens) = inventory(source, command);
            let site = tokens
                .source_binding
                .as_ref()
                .unwrap()
                .invocation_site()
                .unwrap();
            let parents: Vec<_> =
                original_declaration_layouts(bindings.declaration_layouts.get(site).unwrap())
                    .unwrap()
                    .cloned()
                    .collect();
            assert!(!parents.is_empty(), "{source}");
            let originals = parent_declaration_layouts(site, &tokens, &parents, registry);
            let children = unanimous_nested_layouts_from_parents(site, &originals, registry);
            assert_eq!(
                children,
                unanimous_nested_layouts(site, &tokens, &parents, registry),
                "{source}"
            );
            assert_eq!(children.len(), child_count, "{source}");
            let shared = originals.iter().all(|row| {
                row.original
                    .as_ref()
                    .is_some_and(OriginalDeclarationLayout::retains_successor)
            });
            let repeated = parents.iter().all(|parent| {
                retains_successor_layout(
                    site,
                    &tokens,
                    &parent.snapshot,
                    &parent.namespace,
                    parent.config,
                    registry,
                )
            });
            assert_eq!(shared, repeated, "{source}");
            assert_eq!(shared, successor, "{source}");
        }
    }

    #[test]
    fn exact_parent_layout_reuse_refuses_lost_original_source_or_frame() {
        // Implementation contract: naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // These changed-premise API controls cannot manufacture a provider
        // execution result from an original layout's reporting fields.
        let source = "if {1} {puts done}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (bindings, tokens) = inventory(source, source);
        let site = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap();
        let parents: Vec<_> =
            original_declaration_layouts(bindings.declaration_layouts.get(site).unwrap())
                .unwrap()
                .cloned()
                .collect();
        let original = &parents[0].snapshot.state;
        let mut lost_source = original.clone();
        lost_source.current_source_origin = None;
        let mut lost_frame = original.clone();
        lost_frame.variable_frame = crate::var_resolve::VariableExecutionFrame::Unknown;
        let mut lost_dialect = original.clone();
        Arc::make_mut(&mut lost_dialect.source_variables).invocation_dialect = None;
        for state in [lost_source, lost_frame, lost_dialect] {
            let mut changed = parents.clone();
            changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
                state,
                changed[0].snapshot.realm,
            ));
            let rows = parent_declaration_layouts(site, &tokens, &changed, registry);
            assert!(rows[0].original.is_none());
            assert!(unanimous_nested_layouts_from_parents(site, &rows, registry).is_empty());
            assert!(!rows.iter().all(|row| {
                row.original
                    .as_ref()
                    .is_some_and(OriginalDeclarationLayout::retains_successor)
            }));
        }
    }

    fn nested_layout_key_fixture() -> (ExecutedScriptSource, Vec<DeclarationLayoutObservation>) {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (bindings, _) = inventory("if {1} {puts done}", "puts done");
        let (site, rows) = bindings.declaration_layouts.iter().next().unwrap();
        let parents: Vec<_> = original_declaration_layouts(rows)
            .unwrap()
            .cloned()
            .collect();
        let tokens = declaration_tokens(site, &parents[0]).unwrap();
        unanimous_nested_layouts(site, &tokens, &parents, registry).remove(0)
    }

    #[test]
    fn nested_layout_syntax_shares_only_exact_original_geometry() {
        // naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // Pure lexical sharing, independently of any Native lookup/frame/header claim.
        let (source, parents) = nested_layout_key_fixture();
        let config = parents[0].config;
        let mut syntax = NestedLayoutSyntax::default();
        let first = syntax.commands(&source, config).unwrap();
        let second = syntax.commands(&source, config).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(syntax.parsed_scripts, 1);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].site.source, source.origin);
        assert_eq!(first[0].site.offset, source.base());
        assert_eq!(first[0].tokens.argv_texts, ["puts", "done"]);
        assert!(first[0].tokens.source_binding.is_none());
        assert!(first[0].tokens.variable_accesses.is_empty());
        assert!(first[0].tokens.nested_bindings.is_empty());

        let mut changed_config = config;
        changed_config.strict_quoting = !config.strict_quoting;
        let changed = syntax.commands(&source, changed_config).unwrap();
        assert!(!Arc::ptr_eq(&first, &changed));
        let materialised = ExecutedScriptSource::materialised(
            CommandAllocationSite {
                source: Arc::clone(&source.origin),
                offset: source.base(),
            },
            vec![0],
            source.try_text().unwrap(),
        );
        let changed = syntax.commands(&materialised, config).unwrap();
        assert!(!Arc::ptr_eq(&first, &changed));
        assert_eq!(changed[0].site.source, materialised.origin);
        assert_eq!(changed[0].site.offset, 0);
        assert_eq!(changed[0].tokens.argv[0].start(), 0);
        let mut changed_channel = source.clone();
        changed_channel.text = tcl_lexer::SourceImage::document(source.try_text().unwrap());
        let changed = syntax.commands(&changed_channel, config).unwrap();
        assert!(!Arc::ptr_eq(&first, &changed));
        assert_eq!(syntax.parsed_scripts, 4);
    }

    #[test]
    fn nested_layout_syntax_reuse_keeps_original_owner_refusal() {
        // naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // A retained pure tape cannot fill a missing source or frame owner.
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (source, parents) = nested_layout_key_fixture();
        let mut syntax = NestedLayoutSyntax::default();
        let tape = syntax.commands(&source, parents[0].config).unwrap();
        let command = &tape[0];
        let rows = parent_declaration_layouts(&command.site, &command.tokens, &parents, registry);
        assert!(rows[0].original.is_some());
        let original = &parents[0].snapshot.state;
        let mut lost_source = original.clone();
        lost_source.current_source_origin = None;
        let mut lost_frame = original.clone();
        lost_frame.variable_frame = crate::var_resolve::VariableExecutionFrame::Unknown;
        for state in [lost_source, lost_frame] {
            let mut changed = parents.clone();
            changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
                state,
                changed[0].snapshot.realm,
            ));
            let rows =
                parent_declaration_layouts(&command.site, &command.tokens, &changed, registry);
            assert!(rows[0].original.is_none());
        }
        assert_eq!(syntax.parsed_scripts, 1);
    }

    #[test]
    fn nested_layout_syntax_memo_matches_original_parser_world_inventory() {
        // naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // Compare complete retained layouts; neither path claims entered execution.
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for source in [
            "if {1} {if {1} {if {1} {puts done}}}",
            "proc p {condition} {if {$condition} {if {1} {puts done}}}",
            "proc p {condition} {if {[rename puts saved; expr 1]} {puts done}}",
        ] {
            let (mut cached, _) = inventory(source, "puts done");
            let mut original = cached.clone();
            let cached_visits = cached.retain_nested_declared_layouts(registry);
            let mut visited = HashSet::new();
            let original_visits = original.retain_nested_declared_layouts_using(
                registry,
                |source, parents| visited.insert(NestedDeclarationLayoutKey::new(source, parents)),
                NestedLayoutSyntax::parse,
            );
            assert_eq!(cached_visits, original_visits, "{source}");
            assert_eq!(cached, original, "{source}");
        }
    }

    #[test]
    fn nested_layout_visit_key_keeps_complete_lookup_and_owner_facets() {
        // Implementation contract: naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // Changing a semantic facet is a distinct source-contract question;
        // these key controls do not manufacture an entered Native frame.
        let (source, parents) = nested_layout_key_fixture();
        let mut visited = HashSet::new();
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &parents)));
        let mut changed = parents.clone();
        changed[0].words = Arc::from([]);
        assert!(!visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        changed[0].issuer = DeclarationLayoutIssuer::EnteredActivation;
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        changed[0].namespace = super::super::SourceNamespaceKey::authored("::other");
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        changed[0].config.expand_syntax = !changed[0].config.expand_syntax;
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            changed[0].snapshot.state.clone(),
            tcl_dialect::model::InvocationRealm::InterpreterRuntime,
        ));
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        let mut state = changed[0].snapshot.state.clone();
        state.mark_opaque_binding_mutation();
        changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            state,
            changed[0].snapshot.realm,
        ));
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        changed[0].entry = Arc::new(OriginalDiagnosticFrameEntry::ScriptActivation {
            source: changed[0].entry.source().clone(),
            frame: changed[0].entry.frame().clone(),
            namespace: changed[0].namespace.clone(),
            config: changed[0].config,
        });
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let materialised = ExecutedScriptSource::materialised(
            CommandAllocationSite {
                source: Arc::clone(&source.origin),
                offset: source.base(),
            },
            vec![0],
            source.try_text().unwrap(),
        );
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&materialised, &parents)));
        assert_eq!(visited.len(), 8);
    }

    #[test]
    fn nested_declared_layout_keeps_original_scope_without_entered_authority() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Alias exclusion requires complete original body ownership, not an
        // installed alias, entered variable frame or successful read.
        // Implementation binding: naming.source.original-registry-header-advice
        // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
        // The missing original variable stops argv before the rename and proc.
        // Their unentered suffix keeps the quiet original lookup world, while
        // its conditional declaration supplies no installed/entered frame.
        let source =
            "rename $old decl\nproc wrap {condition} {if {$condition} {info exists condition}}";
        let (bindings, tokens) = inventory(source, "info exists condition");
        let binding = tokens.source_binding.as_ref().unwrap();
        let layout = binding
            .declaration_operand_layout_advice(&tokens)
            .expect("unchanged scalar condition retains original nested grammar");
        assert!(layout.closed_lookup());
        assert!(binding.unknown);
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.compiler_lookup_state.is_none());
        let site = binding.invocation_site().unwrap();
        assert!(
            bindings
                .conditional_body_entry_at(&site.source, site.offset)
                .is_none()
        );
        assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
        let originals = original_declaration_layouts(
            binding.declaration_layout_observations.as_deref().unwrap(),
        )
        .unwrap();
        assert!(originals.clone().all(|observation| {
            !observation.snapshot.state.source_variables.dynamic_traces
                && observation.snapshot.state.baseline.native_entry.is_none()
                && matches!(
                    observation.entry.as_ref(),
                    OriginalDiagnosticFrameEntry::DeclaredProcedure(_)
                )
                && observation
                    .entry
                    .parameters()
                    .iter()
                    .map(|parameter| parameter.name.as_str())
                    .eq(["condition"])
                && observation.entry.owns_source(&site.source, site.offset)
                && observation
                    .entry
                    .owns_original_context(&observation.snapshot.state.source_variables)
        }));
    }

    #[test]
    fn nested_declared_layout_declines_lookup_after_unknown_variable_observers() {
        // Implementation binding: naming.source.original-registry-header-advice
        // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
        // SourceAnalysisOptions::unknown_entry widens the original variable
        // world through ModuleCommandBindings::mark_opaque_binding_mutation
        // and ResolveContext::widen. The genuine root condition remains a
        // source layout; its unknown read observers cannot donate child lookup.
        let source = "if {$condition} {info exists condition}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (bindings, condition) = inventory_in_world(source, source, true);
        let binding = condition.source_binding.as_ref().unwrap();
        let site = binding.invocation_site().unwrap();
        let parents = original_declaration_layouts(bindings.declaration_layouts.get(site).unwrap())
            .unwrap()
            .cloned()
            .collect::<Vec<_>>();
        assert!(!parents.is_empty());
        assert!(parents.iter().all(|parent| {
            parent.snapshot.state.source_variables.dynamic_traces
                && parent.snapshot.state.baseline.native_entry.is_none()
                && parent.entry.owns_source(&site.source, site.offset)
        }));
        let originals = parent_declaration_layouts(site, &condition, &parents, registry);
        assert!(originals.iter().all(|parent| parent.original.is_some()));
        assert!(unanimous_nested_layouts_from_parents(site, &originals, registry).is_empty());
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.compiler_lookup_state.is_none());
        let (_, nested) = inventory_in_world(source, "info exists condition", true);
        assert!(
            nested
                .source_binding
                .as_ref()
                .unwrap()
                .declaration_operand_layout_advice(&nested)
                .is_none()
        );
    }

    #[test]
    fn nested_declared_layout_stops_before_condition_callbacks_or_aliases() {
        for body in [
            "if {[rename info saved; expr 1]} {info exists condition}",
            "if {$array(index)} {info exists condition}",
            "if {[expr {1}]} {info exists condition}",
            "upvar 1 outer condition; if {$condition} {info exists condition}",
            "trace add variable condition read replaceInfo; if {$condition} {info exists condition}",
        ] {
            let source = format!("proc wrap {{condition}} {{{body}}}");
            let (_, tokens) = inventory_in_world(&source, "info exists condition", true);
            assert!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .declaration_operand_layout_advice(&tokens)
                    .is_none(),
                "condition lookup is unavailable after: {body}"
            );
        }
    }

    #[test]
    fn uninstalled_body_layout_retains_original_candidate_without_runtime_authority() {
        let source = "rename $old decl\nproc wrap {arg} {upvar 1 other local; return $local}";
        let (bindings, tokens) = inventory_in_world(source, "upvar 1 other local", true);
        let binding = tokens.source_binding.as_ref().unwrap();
        let layout = binding
            .declaration_operand_layout_advice(&tokens)
            .expect("conditional original grammar");
        assert!(!layout.closed_lookup());
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let initial = super::super::ModuleCommandBindings::initial(registry);
        let original = super::super::source_binding(
            &initial,
            "upvar",
            &super::super::SourceNamespaceKey::authored("::"),
        );
        assert!(layout.targets().contains(original.proved_target().unwrap()));
        assert!(binding.unknown);
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.compiler_lookup_state.is_none());
        let (_, original_words, original_offset) = binding
            .original_compiler_source(&tokens)
            .expect("unchanged lexical vector is geometry, not compilation admission");
        assert_eq!(original_words, tokens.words());
        assert_eq!(
            original_offset,
            u32::try_from(source.find("upvar").unwrap()).unwrap()
        );
        assert_eq!(
            binding.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
        let site = binding.invocation_site().unwrap();
        let lexical_body = bindings
            .conditional_body_entry_at(&site.source, site.offset)
            .expect("independent unchanged declaration body recipe");
        assert!(lexical_body.owns_source(&site.source, site.offset));
        assert!(lexical_body.matches_parameters(&["arg"]));
        assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
        let observations = binding.declaration_layout_observations.as_deref().unwrap();
        let entry = &original_declaration_layouts(observations)
            .unwrap()
            .next()
            .unwrap()
            .entry;
        let declaration = match entry.as_ref() {
            OriginalDiagnosticFrameEntry::Body(body) => {
                assert_eq!(body.as_ref(), lexical_body.as_ref());
                &body.allocation().site
            }
            OriginalDiagnosticFrameEntry::DeclaredProcedure(body) => &body.declaration,
            _ => panic!("independent procedure declaration owner"),
        };
        assert_eq!(entry.parameters()[0].name, "arg");
        assert_eq!(
            declaration.offset,
            u32::try_from(source.find("proc wrap").unwrap()).unwrap()
        );
        assert_eq!(
            entry.source().try_text().unwrap(),
            "upvar 1 other local; return $local"
        );
        assert_eq!(
            layout
                .snapshot
                .state
                .source_variables
                .namespace_identity
                .as_ref(),
            lexical_body.namespace_context()
        );
        assert!(entry.owns_original_context(&layout.snapshot.state.source_variables));
        let mut changed = tokens.clone();
        changed.word_exprs.pop();
        assert!(
            binding
                .declaration_operand_layout_advice(&changed)
                .is_none()
        );
    }

    #[test]
    fn abrupt_prefix_retains_declaration_body_geometry_without_installation() {
        let source = "rename $old decl\nproc wrap {arg} {upvar 1 other local; return $local}";
        let (bindings, tokens) = inventory(source, "upvar 1 other local");
        let binding = tokens.source_binding.as_ref().unwrap();
        let layout = binding
            .declaration_operand_layout_advice(&tokens)
            .expect("unchanged abrupt table retains original grammar");
        assert!(
            layout.closed_lookup(),
            "the skipped rename did not change the borrowed table"
        );
        assert!(binding.unknown);
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.compiler_lookup_state.is_none());
        let site = binding.invocation_site().unwrap();
        assert!(
            bindings
                .conditional_body_entry_at(&site.source, site.offset)
                .is_none()
        );
        assert!(bindings.entered_scripts.keys().all(
            |parent| parent.offset != u32::try_from(source.find("proc wrap").unwrap()).unwrap()
        ));
    }

    #[test]
    fn public_declaration_consumer_retains_original_authored_body_namespace() {
        let source = "rename $old decl\nproc wrap {} {\nupvar 1 other local\nreturn $local\n}\n";
        let registry = tcl_registry::CommandRegistry::build_default();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let realm = crate::realm::document_realm_bindings(source, profile, &registry);
        let offset = u32::try_from(source.find("upvar").unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            "upvar 1 other local",
            offset,
            config,
        )
        .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        realm.stamp_original_tokens(&mut tokens);
        let assistance = realm
            .original_declaration_assistance(&tokens, &registry)
            .expect("same original authored namespace and lexical declaration owner");
        assert_eq!(
            assistance.declarations,
            vec![crate::registry_invocation::DeclarationArgument {
                argument: 2,
                name: "local".to_owned(),
            },]
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.proved_execution_target().is_none());
        let (_, original_words, original_offset) = binding
            .original_compiler_source(&tokens)
            .expect("conditional declaration retains only its original vector");
        assert_eq!(original_words, tokens.words());
        assert_eq!(original_offset, offset);
        assert_eq!(
            binding.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
    }

    #[test]
    fn uninstalled_body_layout_stops_at_original_lookup_changes() {
        for prefix in [
            "rename upvar replacement",
            "proc upvar args {}",
            "unknown_call",
        ] {
            let source =
                format!("rename $old decl\nproc wrap {{}} {{{prefix}; upvar 1 other local}}");
            let (_, tokens) = inventory(&source, "upvar 1 other local");
            assert!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .declaration_operand_layout_advice(&tokens)
                    .is_none(),
                "{prefix}"
            );
        }
    }
    #[test]
    fn original_declared_body_parse_cut_keeps_only_the_complete_original_prefix() {
        // naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // These are source lexical cuts, with no entered frame or Native admission.
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let config = tcl_lexer::LexerConfig {
            base_offset: 700,
            base_line: 9,
            base_col: 4,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let body = "# retained comment\n\u{feff}puts pre; puts ${a{b}\nputs after";
        let whole = format!("proc p {{}} {{{body}}}");
        let origin = Arc::new(super::super::SourceOriginId::authored(&Arc::from(
            whole.as_str(),
        )));
        let base = u32::try_from(whole.find(body).unwrap()).unwrap();
        let source = ExecutedScriptSource::contiguous(Arc::clone(&origin), body, base).unwrap();
        let mut syntax = NestedLayoutSyntax::default();
        let prefix = syntax.commands(&source, config).unwrap();
        assert_eq!(prefix.len(), 1);
        assert_eq!(prefix[0].tokens.argv_texts, ["\u{feff}puts", "pre"]);
        assert_eq!(prefix[0].site.source, origin);
        assert_eq!(
            prefix[0].site.offset,
            base + u32::try_from(body.find('\u{feff}').unwrap()).unwrap()
        );
        assert_eq!(prefix[0].tokens.argv[0].start(), prefix[0].site.offset);
        assert!(prefix[0].tokens.source_binding.is_none());
        assert!(Arc::ptr_eq(
            &prefix,
            &syntax.commands(&source, config).unwrap()
        ));
        assert!(syntax.commands.keys().any(|key| key.config == config));
        let first_close = tcl_lexer::LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..config
        };
        let complete = syntax.commands(&source, first_close).unwrap();
        assert_eq!(complete.len(), 3);
        assert_eq!(complete[1].tokens.argv_texts, ["puts", "${a{b}"]);
        assert_eq!(complete[2].tokens.argv_texts, ["puts", "after"]);
        assert!(!Arc::ptr_eq(&prefix, &complete));
    }

    #[test]
    fn original_declared_body_parse_cut_uses_geometry_after_f5_segment_merging() {
        // naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // The merged source projection supplies no selected command or execution.
        let profile = tcl_dialect::DialectProfile::find("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::Tcl9Nesting,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        assert!(config.brace_line_continuation.continues());
        let parent = CommandAllocationSite {
            source: Arc::new(super::super::SourceOriginId::authored(&Arc::from(
                "eval $body",
            ))),
            offset: 0,
        };
        let source = ExecutedScriptSource::materialised(
            parent.clone(),
            vec![0],
            "if {1} {puts yes}\nelse {puts no}\nputs ${a{b}\nputs after",
        );
        let mut syntax = NestedLayoutSyntax::default();
        let prefix = syntax.commands(&source, config).unwrap();
        assert_eq!(prefix.len(), 1);
        assert_eq!(
            prefix[0].tokens.argv_texts,
            ["if", "1", "puts yes", "else", "puts no"]
        );
        assert_eq!(prefix[0].site.source, source.origin);
        assert_eq!(prefix[0].site.offset, 0);
        let first_close = tcl_lexer::LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..config
        };
        let complete = syntax.commands(&source, first_close).unwrap();
        assert_eq!(complete.len(), 3);
        assert_eq!(complete[0].tokens.argv_texts, prefix[0].tokens.argv_texts);
        assert_eq!(complete[2].tokens.argv_texts, ["puts", "after"]);
        let malformed_clause =
            ExecutedScriptSource::materialised(parent, vec![0], "if {1} {puts yes}\nelse ${a{b}");
        let clean_if = syntax.commands(&malformed_clause, config).unwrap();
        assert_eq!(clean_if.len(), 1);
        assert_eq!(clean_if[0].tokens.argv_texts, ["if", "1", "puts yes"]);
    }

    #[test]
    fn original_declared_body_parse_cut_keeps_unavailable_distinct_from_empty() {
        // naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // Hard lexical/Unicode refusal is not a complete empty source tape.
        let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
        let parent = CommandAllocationSite {
            source: Arc::new(super::super::SourceOriginId::authored(&Arc::from(
                "eval $body",
            ))),
            offset: 0,
        };
        let mut syntax = NestedLayoutSyntax::default();
        for body in ["puts ${a{b}", "puts [list \"unterminated]"] {
            let source = ExecutedScriptSource::materialised(parent.clone(), vec![0], body);
            assert!(
                syntax.commands(&source, config).unwrap().is_empty(),
                "{body}"
            );
        }
        let clean = ExecutedScriptSource::materialised(parent.clone(), vec![0], "# only comment");
        assert!(syntax.commands(&clean, config).unwrap().is_empty());
        let welded = ExecutedScriptSource::materialised(
            parent.clone(),
            vec![0],
            "puts before; puts \"bad\"tail; puts after",
        );
        let prefix = syntax.commands(&welded, config).unwrap();
        assert_eq!(prefix.len(), 1);
        assert_eq!(prefix[0].tokens.argv_texts, ["puts", "before"]);
        let strict = tcl_lexer::LexerConfig {
            strict_quoting: true,
            ..config
        };
        assert!(syntax.commands(&welded, strict).is_none());
        let parsed = syntax.parsed_scripts;
        assert!(syntax.commands(&welded, strict).is_none());
        assert_eq!(syntax.parsed_scripts, parsed);
        let opaque = ExecutedScriptSource::materialised_image(
            parent,
            vec![0],
            tcl_lexer::SourceImage::native(b"puts \xff".as_slice()),
        );
        assert!(syntax.commands(&opaque, config).is_none());
        let mut overflow = clean;
        overflow.mapping = super::super::ExecutedScriptMapping::Contiguous { base: u32::MAX };
        assert!(syntax.commands(&overflow, config).is_none());
    }

    #[test]
    fn original_declared_body_parser_keeps_bom_value_and_complete_parent_cache_key() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        // A decoded original script value owns its derived origin. Lexical
        // layout sharing provides no frame, entry, Native head or body execution.
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        assert_eq!(config.leading_bom, tcl_lexer::LeadingBom::Skip);
        let source = "eval \"\\uFEFFputs done\"";
        let image = tcl_lexer::SourceImage::document(source);
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let tokens = super::super::source_command_tokens_boxed(&image, 0, config, &segment);
        let parent = CommandAllocationSite {
            source: Arc::new(super::super::SourceOriginId::authored(&Arc::from(source))),
            offset: 0,
        };
        let word = &tokens.words()[1];
        let value = effective_invocation_word(
            word,
            config.escapes,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
        );
        let value = std::str::from_utf8(value.literal_bytes().unwrap()).unwrap();
        assert_eq!(value, "\u{feff}puts done");
        let body = ExecutedScriptSource::from_word(parent.clone(), 0, word, value, config);
        assert_eq!(
            body.mapping,
            super::super::ExecutedScriptMapping::Materialised
        );
        assert_eq!(body.base(), 0);
        assert_ne!(body.origin, parent.source);
        let mut syntax = NestedLayoutSyntax::default();
        let first = syntax.commands(&body, config).unwrap();
        assert_eq!(first[0].tokens.argv_texts, ["\u{feff}puts", "done"]);
        assert_eq!(first[0].site.offset, 0);
        assert_eq!(first[0].site.source, body.origin);
        assert!(first[0].tokens.source_binding.is_none());
        assert!(Arc::ptr_eq(
            &first,
            &syntax.commands(&body, config).unwrap()
        ));
        let mut content = config;
        content.leading_bom = tcl_lexer::LeadingBom::Content;
        let second = syntax.commands(&body, content).unwrap();
        assert_eq!(second[0].tokens.argv_texts, first[0].tokens.argv_texts);
        assert!(
            !Arc::ptr_eq(&first, &second),
            "complete parent configurations remain distinct keys"
        );
        assert!(syntax.commands.keys().any(|key| key.config == config));
        assert!(syntax.commands.keys().any(|key| key.config == content));
        let whole =
            crate::segmenter::segment_commands_image_with_offset_and_config(&body.text, 0, config)
                .unwrap();
        assert_eq!(
            whole[0].texts[0], "puts",
            "the actual whole-file purpose is independent"
        );
        let mut first_close = config;
        first_close.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
        let indexed = ExecutedScriptSource::materialised(parent, vec![0], "puts ${a{b}");
        let selected = syntax.commands(&indexed, first_close).unwrap();
        assert_eq!(selected[0].tokens.argv_texts, ["puts", "${a{b}"]);
        assert!(syntax.commands(&indexed, config).unwrap().is_empty());
    }
}

#[cfg(test)]
mod original_argument_declaration_tests {
    use super::*;

    fn logical(source: &str) -> SourceCommandBindings {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
                .unwrap()
                .default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        SourceCommandBindings::analyse_with_options(
            source,
            input.lexer_config(),
            input.borrowed_context_registry().commands(),
            super::super::SourceAnalysisOptions::for_logical_source(&input).unwrap(),
        )
    }

    fn puts_parent(
        bindings: &SourceCommandBindings,
    ) -> (
        CommandAllocationSite,
        crate::ir::CommandTokens,
        Vec<DeclarationLayoutObservation>,
    ) {
        let (site, rows) = bindings
            .declaration_layouts
            .iter()
            .find(|(_, rows)| {
                rows.first().is_some_and(|row| {
                    row.words
                        .first()
                        .is_some_and(|word| word.legacy_text() == "puts")
                })
            })
            .unwrap();
        let rows = original_declaration_layouts(rows)
            .unwrap()
            .cloned()
            .collect::<Vec<_>>();
        let tokens = declaration_tokens(site, &rows[0]).unwrap();
        (site.clone(), tokens, rows)
    }

    #[test]
    fn original_argv_declaration_retains_real_child_world_without_dispatch_or_frame() {
        // naming.compiler.original-positioned-callee-body-availability
        // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
        let source = "proc subject {} {puts [list FIRST] [package require Tcl]}";
        let bindings = logical(source);
        let (site, mut tokens, _) = puts_parent(&bindings);
        bindings.stamp_original_tokens(&mut tokens);
        let config = bindings.lexer_config.unwrap();
        let calls = crate::word_subst::checked_argument_lifted_calls(&tokens, config).unwrap();
        assert_eq!(calls.len(), 2);
        let input = bindings
            .original_logical_source_name_advice_input()
            .unwrap();
        for call in calls {
            let child = call.tokens.unwrap();
            let binding = child.source_binding.as_ref().unwrap();
            assert!(binding.original_logical_declaration_targets.is_some());
            let targets = binding
                .original_logical_declaration_call_targets(
                    &child,
                    input,
                    input.borrowed_context_registry().commands(),
                )
                .unwrap();
            assert_eq!(targets.len(), 1);
            assert!(targets[0].registry_backed);
            assert!(targets[0].implementation_allocation.is_none());
            assert!(binding.lookup_state.is_none());
            assert!(binding.unknown);
            assert!(binding.targets.is_empty());
            assert!(binding.proved_execution_target().is_none());
            assert!(binding.proved_handler_target().is_none());
            assert_ne!(
                binding.runtime_reachability(),
                super::super::SourceRuntimeReachability::Reached
            );
        }
        assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
    }

    #[test]
    fn original_argv_declaration_stops_after_unknown_callback_or_variable_prefix() {
        // naming.compiler.original-positioned-callee-body-availability
        // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
        for source in [
            "proc subject {value} {puts $value [package require Tcl]}",
            "proc subject {} {puts [unknown] [package require Tcl]}",
        ] {
            let bindings = logical(source);
            let (_, mut tokens, _) = puts_parent(&bindings);
            bindings.stamp_original_tokens(&mut tokens);
            let package = crate::word_subst::checked_argument_lifted_calls(
                &tokens,
                bindings.lexer_config.unwrap(),
            )
            .unwrap()
            .into_iter()
            .find(|call| call.command == "package")
            .unwrap();
            let child = package.tokens.unwrap();
            let binding = child.source_binding.as_ref().unwrap();
            assert!(
                binding.original_logical_declaration_targets.is_none(),
                "{source}"
            );
            assert!(binding.lookup_state.is_none());
            assert!(binding.proved_execution_target().is_none());
            assert!(binding.proved_handler_target().is_none());
        }
    }

    #[test]
    fn original_argv_declaration_requires_original_parent_input_source_config_and_vector() {
        // naming.compiler.original-positioned-callee-body-availability
        // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
        let bindings = logical("proc subject {} {puts [package require Tcl]}");
        let (site, tokens, rows) = puts_parent(&bindings);
        let calls = crate::word_subst::checked_argument_lifted_calls(
            &tokens,
            bindings.lexer_config.unwrap(),
        )
        .unwrap();
        let child_offset = calls[0].tokens.as_ref().unwrap().words()[0]
            .source()
            .span
            .start();
        let child_site = CommandAllocationSite {
            source: Arc::clone(&site.source),
            offset: child_offset,
        };
        let registry = bindings
            .original_logical_source_name_advice_input()
            .unwrap()
            .context_registry();
        for axis in 0..5 {
            let mut changed = bindings.clone();
            changed.declaration_layouts.remove(&child_site);
            let mut parent = tokens.clone();
            let mut originals = rows.clone();
            match axis {
                0 => changed.lexer_config = None,
                1 => {
                    let mut config = changed.lexer_config.unwrap();
                    config.strict_quoting = !config.strict_quoting;
                    changed.lexer_config = Some(config);
                }
                2 => {
                    parent.word_exprs.pop();
                }
                3 => {
                    changed.root_origin =
                        logical("proc foreign {} {puts [package require Tcl]}").root_origin
                }
                4 => {
                    let snapshot = Arc::make_mut(&mut originals[0].snapshot);
                    Arc::make_mut(&mut snapshot.state.baseline).logical_source_input = None;
                }
                _ => unreachable!(),
            }
            changed.retain_original_argument_layouts(
                &site,
                &parent,
                &originals,
                registry.commands(),
                &mut Vec::new(),
            );
            assert!(
                !changed.declaration_layouts.contains_key(&child_site),
                "axis {axis}"
            );
        }
    }
}
